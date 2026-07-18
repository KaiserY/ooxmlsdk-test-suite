use std::fs;
use std::path::{Path, PathBuf};

use image::ImageFormat;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    CalibrationError, PdfBounds, PdfSummary, RenderedPageImage, Result, parse_pdf_rect,
    rendered_page_image_from_pdf, workspace_root,
};

const RASTER_WIDTH: i32 = 1_333;
// Compare the first glyph's baseline origin and horizontal ink bounds rather
// than loose vertical edges: those edges include font-descriptor differences
// between Office's simple TrueType subsets and our CID subsets. Width stays at
// a tighter relative bound below.
const TEXT_EDGE_TOLERANCE_PT: f32 = 2.0;
const TEXT_WIDTH_TOLERANCE_RATIO: f32 = 0.01;
const TEXT_MASK_PADDING_PT: f32 = 1.0;
const TEXT_FONT_SIZE_TOLERANCE_PT: f32 = 0.12;
// ISO paper sizes convert to fractional PDF points, while Office serializes
// the MediaBox through its fixed-output device grid. Keep this well below one
// rendered pixel while accepting the observed sub-tenth-point quantization.
const MEDIA_BOX_TOLERANCE_PT: f32 = 0.1;

#[derive(Clone, Copy, Debug)]
pub struct OfficeGoldenCase {
    pub id: &'static str,
    pub corpus: &'static str,
    pub source: &'static str,
    pub source_sha256: &'static str,
    pub golden_sha256: &'static str,
    pub environment_id: &'static str,
    pub ui_language: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct VisualTolerance {
    pub significant_channel_delta: u8,
    pub max_significant_pixel_fraction: f64,
    pub max_mean_absolute_channel_delta: f64,
}

impl VisualTolerance {
    /// Shared starting contract for Office golden comparisons.
    ///
    /// The significant-pixel threshold absorbs rasterizer antialiasing noise;
    /// the fraction and mean limits remain independent so a sparse large error
    /// and a broad low-contrast error are both observable.
    pub const OFFICE_FIXED_OUTPUT: Self = Self {
        significant_channel_delta: 16,
        max_significant_pixel_fraction: 0.01,
        max_mean_absolute_channel_delta: 1.5,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualDiffMetrics {
    pub total_pixels: usize,
    pub significant_pixels: usize,
    pub significant_pixel_fraction: f64,
    pub mean_absolute_channel_delta: f64,
    pub max_channel_delta: u8,
}

#[derive(Clone, Debug)]
pub struct OfficeGoldenReport {
    pub case_id: &'static str,
    pub candidate: PdfSummary,
    pub golden: PdfSummary,
    pub page_diffs: Vec<VisualDiffMetrics>,
    pub artifact_dir: Option<PathBuf>,
}

pub fn compare_office_golden(
    case: OfficeGoldenCase,
    tolerance: VisualTolerance,
) -> Result<OfficeGoldenReport> {
    let root = workspace_root();
    let source_path = root.join("corpus").join(case.corpus).join(case.source);
    let golden_path = root
        .join("corpus_pdf_conv")
        .join(case.corpus)
        .join(format!("{}.pdf", case.source));
    verify_manifest_record(&root, case)?;

    let source_bytes = fs::read(&source_path)?;
    verify_sha256("source", &source_path, &source_bytes, case.source_sha256)?;
    let golden_pdf = fs::read(&golden_path)?;
    verify_sha256("golden", &golden_path, &golden_pdf, case.golden_sha256)?;
    let candidate_pdf = crate::render::render_fixture_pdf_with_options(
        &source_path,
        ooxmlsdk_pdf::PdfOptions {
            source_file_name: source_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string),
            ui_language: Some(case.ui_language.to_string()),
            ..Default::default()
        },
    )?;

    let candidate =
        PdfSummary::from_bytes(&candidate_pdf).map_err(CalibrationError::PdfiumExtraction)?;
    let golden = PdfSummary::from_bytes(&golden_pdf).map_err(CalibrationError::PdfiumExtraction)?;

    let text_masks = match assert_document_contract(case.id, &candidate, &golden) {
        Ok(text_masks) => text_masks,
        Err(error) => {
            let artifact_dir = write_candidate_artifact(case.id, &candidate_pdf)?;
            return Err(CalibrationError::OfficeGolden(format!(
                "{error}; artifacts={}",
                artifact_dir.display()
            )));
        }
    };

    let mut page_diffs = Vec::with_capacity(golden.page_count);
    let mut rendered_pages = Vec::with_capacity(golden.page_count);
    let mut visual_passes = true;
    for page_index in 0..golden.page_count {
        let candidate_page = rendered_page_image_from_pdf(&candidate_pdf, page_index, RASTER_WIDTH)
            .map_err(CalibrationError::PdfiumExtraction)?;
        let golden_page = rendered_page_image_from_pdf(&golden_pdf, page_index, RASTER_WIDTH)
            .map_err(CalibrationError::PdfiumExtraction)?;
        let metrics = visual_diff_metrics(
            &candidate_page,
            &golden_page,
            tolerance.significant_channel_delta,
            text_masks.get(page_index).map(Vec::as_slice).unwrap_or(&[]),
            parse_pdf_rect(&golden.media_boxes[page_index])
                .map_err(CalibrationError::PdfiumExtraction)?,
        )?;
        visual_passes &= metrics.significant_pixel_fraction
            <= tolerance.max_significant_pixel_fraction
            && metrics.mean_absolute_channel_delta <= tolerance.max_mean_absolute_channel_delta;
        page_diffs.push(metrics);
        rendered_pages.push((candidate_page, golden_page));
    }

    let artifact_dir = if visual_passes {
        None
    } else {
        Some(write_failure_artifacts(
            case.id,
            &candidate_pdf,
            &rendered_pages,
        )?)
    };

    let report = OfficeGoldenReport {
        case_id: case.id,
        candidate,
        golden,
        page_diffs,
        artifact_dir,
    };
    if visual_passes {
        Ok(report)
    } else {
        Err(CalibrationError::OfficeGolden(format!(
            "case {} exceeds {:?}; page diffs={:?}; artifacts={}",
            case.id,
            tolerance,
            report.page_diffs,
            report
                .artifact_dir
                .as_deref()
                .map_or_else(|| "none".into(), |path| path.display().to_string())
        )))
    }
}

fn verify_manifest_record(root: &Path, case: OfficeGoldenCase) -> Result<()> {
    let manifest_path = root
        .join("corpus_pdf_conv")
        .join(case.corpus)
        .join("manifest.jsonl");
    let manifest = fs::read_to_string(&manifest_path)?;
    let mut matching = Vec::new();
    for (line_index, line) in manifest.lines().enumerate() {
        let record: Value = serde_json::from_str(line).map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid JSON at {}:{}: {error}",
                manifest_path.display(),
                line_index + 1
            ))
        })?;
        if record.get("file").and_then(Value::as_str) == Some(case.source) {
            matching.push(record);
        }
    }
    let [record] = matching.as_slice() else {
        return Err(CalibrationError::OfficeGolden(format!(
            "expected exactly one manifest record for {}/{}, found {}",
            case.corpus,
            case.source,
            matching.len()
        )));
    };
    for (field, expected) in [
        ("status", "converted"),
        ("reference_engine", "Microsoft Office"),
        ("source_sha256", case.source_sha256),
        ("output_sha256", case.golden_sha256),
        ("environment_id", case.environment_id),
    ] {
        let actual = record.get(field).and_then(Value::as_str);
        if actual != Some(expected) {
            return Err(CalibrationError::OfficeGolden(format!(
                "manifest field {field} mismatch for {}: actual={actual:?}, expected={expected:?}",
                case.id
            )));
        }
    }
    let expected_output = format!("{}.pdf", case.source);
    if record.get("output").and_then(Value::as_str) != Some(expected_output.as_str()) {
        return Err(CalibrationError::OfficeGolden(format!(
            "manifest output mismatch for {}",
            case.id
        )));
    }
    Ok(())
}

fn verify_sha256(label: &str, path: &Path, bytes: &[u8], expected: &str) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(CalibrationError::OfficeGolden(format!(
            "{label} SHA-256 mismatch for {}: actual={actual}, expected={expected}",
            path.display()
        )));
    }
    Ok(())
}

fn assert_document_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<Vec<Vec<PdfBounds>>> {
    if candidate.page_count != golden.page_count {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} page count mismatch: candidate={}, golden={}",
            candidate.page_count, golden.page_count
        )));
    }
    if candidate.media_boxes.len() != golden.media_boxes.len() {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} media box count mismatch: candidate={}, golden={}",
            candidate.media_boxes.len(),
            golden.media_boxes.len()
        )));
    }
    for (page_index, (candidate_box, golden_box)) in candidate
        .media_boxes
        .iter()
        .zip(&golden.media_boxes)
        .enumerate()
    {
        let candidate_box =
            parse_pdf_rect(candidate_box).map_err(CalibrationError::PdfiumExtraction)?;
        let golden_box = parse_pdf_rect(golden_box).map_err(CalibrationError::PdfiumExtraction)?;
        let max_delta = [
            (candidate_box.left - golden_box.left).abs(),
            (candidate_box.bottom - golden_box.bottom).abs(),
            (candidate_box.right - golden_box.right).abs(),
            (candidate_box.top - golden_box.top).abs(),
        ]
        .into_iter()
        .fold(0.0_f32, f32::max);
        if max_delta > MEDIA_BOX_TOLERANCE_PT {
            return Err(CalibrationError::OfficeGolden(format!(
                "case {case_id} page {page_index} media box mismatch: candidate={candidate_box:?}, golden={golden_box:?}"
            )));
        }
    }

    let candidate_text = normalized_page_text(candidate);
    let golden_text = normalized_page_text(golden);
    if candidate_text != golden_text {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} normalized page text mismatch: candidate={candidate_text:?}, golden={golden_text:?}"
        )));
    }
    assert_text_style_contract(case_id, candidate, golden)?;
    assert_text_line_geometry(case_id, candidate, golden)
}

#[derive(Clone, Debug)]
struct TextLineContract {
    text: String,
    bounds: PdfBounds,
    origin_y: f32,
}

fn assert_text_style_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<()> {
    use std::collections::BTreeSet;

    let style_set = |summary: &PdfSummary| {
        summary
            .text_objects
            .iter()
            .filter(|object| !object.text.trim().is_empty())
            .map(|object| {
                (
                    object.font_name.clone(),
                    object.render_mode.clone(),
                    object.fill_color.clone(),
                )
            })
            .collect::<BTreeSet<_>>()
    };
    let candidate_styles = style_set(candidate);
    let golden_styles = style_set(golden);
    if candidate_styles != golden_styles {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} text style set mismatch: candidate={candidate_styles:?}, golden={golden_styles:?}"
        )));
    }
    let font_sizes = |summary: &PdfSummary| -> Result<Vec<f32>> {
        summary
            .text_objects
            .iter()
            .filter(|object| !object.text.trim().is_empty())
            .map(|object| {
                object.scaled_font_size.parse::<f32>().map_err(|error| {
                    CalibrationError::OfficeGolden(format!(
                        "invalid extracted font size {:?}: {error}",
                        object.scaled_font_size
                    ))
                })
            })
            .collect()
    };
    let candidate_sizes = font_sizes(candidate)?;
    let golden_sizes = font_sizes(golden)?;
    for (label, actual, expected) in [
        ("candidate", &candidate_sizes, &golden_sizes),
        ("golden", &golden_sizes, &candidate_sizes),
    ] {
        for size in actual {
            if !expected
                .iter()
                .any(|other| (size - other).abs() <= TEXT_FONT_SIZE_TOLERANCE_PT)
            {
                return Err(CalibrationError::OfficeGolden(format!(
                    "case {case_id} {label} text font size {size} has no peer within {TEXT_FONT_SIZE_TOLERANCE_PT}pt: peers={expected:?}"
                )));
            }
        }
    }
    Ok(())
}

fn assert_text_line_geometry(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<Vec<Vec<PdfBounds>>> {
    let candidate_lines = text_line_contracts(candidate)?;
    let golden_lines = text_line_contracts(golden)?;
    let mut masks = vec![Vec::new(); candidate.page_count];
    for page_index in 0..candidate.page_count {
        let candidate_page = &candidate_lines[page_index];
        let golden_page = &golden_lines[page_index];
        if candidate_page.len() != golden_page.len() {
            return Err(CalibrationError::OfficeGolden(format!(
                "case {case_id} page {page_index} text line count mismatch: candidate={}, golden={}",
                candidate_page.len(),
                golden_page.len()
            )));
        }
        for (line_index, (candidate_line, golden_line)) in
            candidate_page.iter().zip(golden_page).enumerate()
        {
            if candidate_line.text != golden_line.text {
                return Err(CalibrationError::OfficeGolden(format!(
                    "case {case_id} page {page_index} line {line_index} text mismatch: candidate={:?}, golden={:?}",
                    candidate_line.text, golden_line.text
                )));
            }
            let golden_width = golden_line.bounds.right - golden_line.bounds.left;
            let width_tolerance =
                (golden_width.abs() * TEXT_WIDTH_TOLERANCE_RATIO).max(TEXT_EDGE_TOLERANCE_PT);
            for (edge, candidate_value, golden_value, tolerance) in [
                (
                    "left",
                    candidate_line.bounds.left,
                    golden_line.bounds.left,
                    TEXT_EDGE_TOLERANCE_PT,
                ),
                (
                    "right",
                    candidate_line.bounds.right,
                    golden_line.bounds.right,
                    width_tolerance,
                ),
                (
                    "baseline origin",
                    candidate_line.origin_y,
                    golden_line.origin_y,
                    TEXT_EDGE_TOLERANCE_PT,
                ),
            ] {
                if (candidate_value - golden_value).abs() > tolerance {
                    return Err(CalibrationError::OfficeGolden(format!(
                        "case {case_id} page {page_index} line {line_index} {edge} mismatch: candidate={candidate_value}, golden={golden_value}, tolerance={tolerance}"
                    )));
                }
            }
            masks[page_index].push(union_pdf_bounds(candidate_line.bounds, golden_line.bounds));
        }
    }
    Ok(masks)
}

fn text_line_contracts(summary: &PdfSummary) -> Result<Vec<Vec<TextLineContract>>> {
    let mut pages = vec![Vec::<TextLineContract>::new(); summary.page_count];
    for character in &summary.text_chars {
        if character.text.chars().all(char::is_whitespace) {
            continue;
        }
        let bounds =
            parse_pdf_rect(&character.bounds).map_err(CalibrationError::PdfiumExtraction)?;
        let origin_y = character.origin_y.parse::<f32>().map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid extracted text origin {:?}: {error}",
                character.origin_y
            ))
        })?;
        let page = pages.get_mut(character.page_index).ok_or_else(|| {
            CalibrationError::OfficeGolden(format!(
                "text character references missing page {}",
                character.page_index
            ))
        })?;
        if let Some(line) = page.last_mut()
            && vertical_bounds_overlap(line.bounds, bounds)
        {
            line.text.push_str(&character.text);
            line.bounds = union_pdf_bounds(line.bounds, bounds);
        } else {
            page.push(TextLineContract {
                text: character.text.clone(),
                bounds,
                origin_y,
            });
        }
    }
    Ok(pages)
}

fn vertical_bounds_overlap(left: PdfBounds, right: PdfBounds) -> bool {
    left.top.min(right.top) > left.bottom.max(right.bottom)
}

fn union_pdf_bounds(left: PdfBounds, right: PdfBounds) -> PdfBounds {
    PdfBounds {
        left: left.left.min(right.left),
        bottom: left.bottom.min(right.bottom),
        right: left.right.max(right.right),
        top: left.top.max(right.top),
    }
}

fn normalized_page_text(summary: &PdfSummary) -> Vec<String> {
    let mut extracted_pages = vec![String::new(); summary.page_count];
    for segment in &summary.text_segments {
        if let Some(page) = extracted_pages.get_mut(segment.page_index) {
            if !page.is_empty() {
                page.push(' ');
            }
            page.push_str(&segment.text);
        }
    }
    extracted_pages
        .into_iter()
        .map(|text| normalize_extracted_text(&text))
        .collect()
}

fn normalize_extracted_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        if character.is_whitespace() {
            pending_space = true;
            continue;
        }
        let previous_is_opening = normalized
            .chars()
            .next_back()
            .is_some_and(|previous| matches!(previous, '(' | '[' | '{'));
        let is_closing = matches!(
            character,
            '.' | ',' | ';' | ':' | '!' | '?' | '%' | ')' | ']' | '}'
        );
        if pending_space && !normalized.is_empty() && !previous_is_opening && !is_closing {
            normalized.push(' ');
        }
        normalized.push(character);
        pending_space = false;
    }
    normalized
}

fn visual_diff_metrics(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    significant_channel_delta: u8,
    text_masks: &[PdfBounds],
    page_bounds: PdfBounds,
) -> Result<VisualDiffMetrics> {
    if candidate.width_px != golden.width_px || candidate.height_px != golden.height_px {
        return Err(CalibrationError::OfficeGolden(format!(
            "rendered page dimensions differ: candidate={}x{}, golden={}x{}",
            candidate.width_px, candidate.height_px, golden.width_px, golden.height_px
        )));
    }
    let total_pixels = (candidate.width_px * candidate.height_px) as usize;
    let mut significant_pixels = 0usize;
    let mut absolute_delta_sum = 0u64;
    let mut max_channel_delta = 0u8;
    for (pixel_index, (candidate_pixel, golden_pixel)) in candidate
        .rgba
        .chunks_exact(4)
        .zip(golden.rgba.chunks_exact(4))
        .enumerate()
    {
        let pixel_x = pixel_index % candidate.width_px as usize;
        let pixel_y = pixel_index / candidate.width_px as usize;
        if pixel_is_in_text_mask(
            pixel_x,
            pixel_y,
            candidate.width_px,
            candidate.height_px,
            page_bounds,
            text_masks,
        ) {
            continue;
        }
        let mut pixel_max = 0u8;
        for channel in 0..3 {
            let delta = candidate_pixel[channel].abs_diff(golden_pixel[channel]);
            absolute_delta_sum += u64::from(delta);
            pixel_max = pixel_max.max(delta);
            max_channel_delta = max_channel_delta.max(delta);
        }
        if pixel_max > significant_channel_delta {
            significant_pixels += 1;
        }
    }
    Ok(VisualDiffMetrics {
        total_pixels,
        significant_pixels,
        significant_pixel_fraction: significant_pixels as f64 / total_pixels as f64,
        mean_absolute_channel_delta: absolute_delta_sum as f64 / (total_pixels * 3) as f64,
        max_channel_delta,
    })
}

fn pixel_is_in_text_mask(
    pixel_x: usize,
    pixel_y: usize,
    width_px: u32,
    height_px: u32,
    page: PdfBounds,
    masks: &[PdfBounds],
) -> bool {
    let page_width = page.right - page.left;
    let page_height = page.top - page.bottom;
    if page_width <= 0.0 || page_height <= 0.0 {
        return false;
    }
    let x = page.left + (pixel_x as f32 + 0.5) / width_px as f32 * page_width;
    let y = page.top - (pixel_y as f32 + 0.5) / height_px as f32 * page_height;
    masks.iter().any(|mask| {
        x >= mask.left - TEXT_MASK_PADDING_PT
            && x <= mask.right + TEXT_MASK_PADDING_PT
            && y >= mask.bottom - TEXT_MASK_PADDING_PT
            && y <= mask.top + TEXT_MASK_PADDING_PT
    })
}

fn write_failure_artifacts(
    case_id: &str,
    candidate_pdf: &[u8],
    pages: &[(RenderedPageImage, RenderedPageImage)],
) -> Result<PathBuf> {
    let artifact_dir = write_candidate_artifact(case_id, candidate_pdf)?;
    for (page_index, (candidate, golden)) in pages.iter().enumerate() {
        write_png(
            &artifact_dir.join(format!("page-{page_index}-candidate.png")),
            candidate,
            &candidate.rgba,
        )?;
        write_png(
            &artifact_dir.join(format!("page-{page_index}-golden.png")),
            golden,
            &golden.rgba,
        )?;
        let diff = candidate
            .rgba
            .chunks_exact(4)
            .zip(golden.rgba.chunks_exact(4))
            .flat_map(|(candidate_pixel, golden_pixel)| {
                let delta = (0..3)
                    .map(|channel| candidate_pixel[channel].abs_diff(golden_pixel[channel]))
                    .max()
                    .unwrap_or(0);
                [delta, 0, 0, 255]
            })
            .collect::<Vec<_>>();
        write_png(
            &artifact_dir.join(format!("page-{page_index}-diff.png")),
            candidate,
            &diff,
        )?;
    }
    Ok(artifact_dir)
}

fn write_candidate_artifact(case_id: &str, candidate_pdf: &[u8]) -> Result<PathBuf> {
    let artifact_dir = workspace_root().join("target/office-golden").join(case_id);
    fs::create_dir_all(&artifact_dir)?;
    fs::write(artifact_dir.join("candidate.pdf"), candidate_pdf)?;
    Ok(artifact_dir)
}

fn write_png(path: &Path, page: &RenderedPageImage, rgba: &[u8]) -> Result<()> {
    image::save_buffer_with_format(
        path,
        rgba,
        page.width_px,
        page.height_px,
        image::ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|error| {
        CalibrationError::OfficeGolden(format!("could not write {}: {error}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_extracted_text;

    #[test]
    fn extracted_text_normalization_ignores_pdf_object_spacing_around_punctuation() {
        assert_eq!(
            normalize_extracted_text("slideMaster .  Two shapes ( one )"),
            "slideMaster. Two shapes (one)"
        );
        assert_eq!(
            normalize_extracted_text("slideMaster. Two shapes (one)"),
            "slideMaster. Two shapes (one)"
        );
    }
}

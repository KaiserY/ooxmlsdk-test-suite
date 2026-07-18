use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use image::ImageFormat;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::pdf_extract::{RenderedPagePairError, visit_rendered_page_pairs};
use crate::{
    CalibrationError, PdfBounds, PdfSummary, RenderedPageImage, Result, parse_pdf_rect,
    workspace_root,
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
pub struct OfficeGoldenCase<'a> {
    pub id: &'a str,
    pub corpus: &'a str,
    pub source: &'a str,
    pub source_sha256: &'a str,
    pub golden_sha256: &'a str,
    pub environment_id: &'a str,
    pub ui_language: &'a str,
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
    pub case_id: String,
    pub candidate: PdfSummary,
    pub golden: PdfSummary,
    pub page_diffs: Vec<VisualDiffMetrics>,
    pub artifact_dir: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OfficeGoldenComparisonLayer {
    Identity,
    Conversion,
    PdfExtraction,
    PageGeometry,
    Text,
    VisibleOutput,
    ComparisonArtifact,
}

impl OfficeGoldenComparisonLayer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Conversion => "conversion",
            Self::PdfExtraction => "pdf-extraction",
            Self::PageGeometry => "page-geometry",
            Self::Text => "text",
            Self::VisibleOutput => "visible-output",
            Self::ComparisonArtifact => "comparison-artifact",
        }
    }
}

impl std::str::FromStr for OfficeGoldenComparisonLayer {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "identity" => Ok(Self::Identity),
            "conversion" => Ok(Self::Conversion),
            "pdf-extraction" => Ok(Self::PdfExtraction),
            "page-geometry" => Ok(Self::PageGeometry),
            "text" => Ok(Self::Text),
            "visible-output" => Ok(Self::VisibleOutput),
            "comparison-artifact" => Ok(Self::ComparisonArtifact),
            _ => Err(format!("unknown Office golden comparison layer {value:?}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfficeGoldenFailure {
    pub layer: OfficeGoldenComparisonLayer,
    pub message: String,
}

impl OfficeGoldenFailure {
    fn new(layer: OfficeGoldenComparisonLayer, error: impl fmt::Display) -> Self {
        Self {
            layer,
            message: error.to_string(),
        }
    }
}

impl fmt::Display for OfficeGoldenFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.layer.as_str(), self.message)
    }
}

impl std::error::Error for OfficeGoldenFailure {}

type DetailedResult<T> = std::result::Result<T, OfficeGoldenFailure>;

pub fn compare_office_golden(
    case: OfficeGoldenCase<'_>,
    tolerance: VisualTolerance,
) -> Result<OfficeGoldenReport> {
    compare_office_golden_detailed(case, tolerance)
        .map_err(|error| CalibrationError::OfficeGolden(error.to_string()))
}

pub fn compare_office_golden_detailed(
    case: OfficeGoldenCase<'_>,
    tolerance: VisualTolerance,
) -> DetailedResult<OfficeGoldenReport> {
    let root = workspace_root();
    let source_path = root.join("corpus").join(case.corpus).join(case.source);
    let golden_path = root
        .join("corpus_pdf_conv")
        .join(case.corpus)
        .join(format!("{}.pdf", case.source));
    verify_manifest_record(&root, case)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;

    let source_bytes = fs::read(&source_path)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    verify_sha256("source", &source_path, &source_bytes, case.source_sha256)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    let golden_pdf = fs::read(&golden_path)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    verify_sha256("golden", &golden_path, &golden_pdf, case.golden_sha256)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
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
    )
    .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Conversion, error))?;

    let candidate = PdfSummary::from_bytes(&candidate_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    let golden = PdfSummary::from_bytes(&golden_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;

    let text_masks = match assert_page_geometry_contract(case.id, &candidate, &golden)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PageGeometry, error))
        .and_then(|()| {
            assert_text_contract(case.id, &candidate, &golden)
                .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Text, error))
        }) {
        Ok(text_masks) => text_masks,
        Err(error) => {
            let artifact_dir =
                write_candidate_artifact(case.id, &candidate_pdf).map_err(|artifact_error| {
                    OfficeGoldenFailure::new(
                        OfficeGoldenComparisonLayer::ComparisonArtifact,
                        artifact_error,
                    )
                })?;
            return Err(OfficeGoldenFailure {
                layer: error.layer,
                message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
            });
        }
    };

    let mut page_diffs = Vec::with_capacity(golden.page_count);
    let mut visual_passes = true;
    let mut failing_pages = Vec::new();
    let mut artifact_dir = None;
    visit_rendered_page_pairs(
        &candidate_pdf,
        &golden_pdf,
        RASTER_WIDTH,
        |page_index, candidate_page, golden_page| -> DetailedResult<()> {
            let metrics = visual_diff_metrics(
                &candidate_page,
                &golden_page,
                tolerance.significant_channel_delta,
                text_masks.get(page_index).map(Vec::as_slice).unwrap_or(&[]),
                parse_pdf_rect(&golden.media_boxes[page_index]).map_err(|error| {
                    OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                })?,
            )
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::VisibleOutput, error)
            })?;
            let page_passes = metrics.significant_pixel_fraction
                <= tolerance.max_significant_pixel_fraction
                && metrics.mean_absolute_channel_delta <= tolerance.max_mean_absolute_channel_delta;
            visual_passes &= page_passes;
            if !page_passes {
                if artifact_dir.is_none() {
                    artifact_dir = Some(
                        write_candidate_artifact(case.id, &candidate_pdf).map_err(|error| {
                            OfficeGoldenFailure::new(
                                OfficeGoldenComparisonLayer::ComparisonArtifact,
                                error,
                            )
                        })?,
                    );
                }
                let artifact_dir = artifact_dir
                    .as_deref()
                    .expect("failure artifact directory should be initialized");
                write_failure_page_artifacts(
                    artifact_dir,
                    page_index,
                    &candidate_page,
                    &golden_page,
                )
                .map_err(|error| {
                    OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::ComparisonArtifact, error)
                })?;
                failing_pages.push(page_index);
            }
            page_diffs.push(metrics);
            Ok(())
        },
    )
    .map_err(|error| match error {
        RenderedPagePairError::Pdf(error) => {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
        }
        RenderedPagePairError::Visit(error) => error,
    })?;

    let report = OfficeGoldenReport {
        case_id: case.id.to_string(),
        candidate,
        golden,
        page_diffs,
        artifact_dir,
    };
    if visual_passes {
        Ok(report)
    } else {
        let max_significant_pixel_fraction = failing_pages
            .iter()
            .map(|&page_index| report.page_diffs[page_index].significant_pixel_fraction)
            .fold(0.0_f64, f64::max);
        let max_mean_absolute_channel_delta = failing_pages
            .iter()
            .map(|&page_index| report.page_diffs[page_index].mean_absolute_channel_delta)
            .fold(0.0_f64, f64::max);
        let max_channel_delta = failing_pages
            .iter()
            .map(|&page_index| report.page_diffs[page_index].max_channel_delta)
            .max()
            .unwrap_or(0);
        Err(OfficeGoldenFailure::new(
            OfficeGoldenComparisonLayer::VisibleOutput,
            format!(
                "case {} exceeds {:?}; failing pages={} ({} of {}); maxima: significant_pixel_fraction={}, mean_absolute_channel_delta={}, max_channel_delta={}; artifacts={}",
                case.id,
                tolerance,
                format_page_ranges(&failing_pages),
                failing_pages.len(),
                report.page_diffs.len(),
                max_significant_pixel_fraction,
                max_mean_absolute_channel_delta,
                max_channel_delta,
                report
                    .artifact_dir
                    .as_deref()
                    .map_or_else(|| "none".into(), |path| path.display().to_string())
            ),
        ))
    }
}

#[derive(Debug)]
struct ConversionManifestIdentity {
    status: String,
    reference_engine: String,
    source_sha256: String,
    output_sha256: String,
    environment_id: String,
    output: String,
}

static CONVERSION_MANIFEST_IDENTITIES: OnceLock<
    std::result::Result<BTreeMap<(String, String), ConversionManifestIdentity>, String>,
> = OnceLock::new();

fn verify_manifest_record(root: &Path, case: OfficeGoldenCase<'_>) -> Result<()> {
    let records = CONVERSION_MANIFEST_IDENTITIES
        .get_or_init(|| load_conversion_manifest_identities(root))
        .as_ref()
        .map_err(|error| CalibrationError::OfficeGolden(error.clone()))?;
    let key = (case.corpus.to_string(), case.source.to_string());
    let Some(record) = records.get(&key) else {
        return Err(CalibrationError::OfficeGolden(format!(
            "expected exactly one manifest record for {}/{}, found 0",
            case.corpus, case.source
        )));
    };
    for (field, actual, expected) in [
        ("status", record.status.as_str(), "converted"),
        (
            "reference_engine",
            record.reference_engine.as_str(),
            "Microsoft Office",
        ),
        (
            "source_sha256",
            record.source_sha256.as_str(),
            case.source_sha256,
        ),
        (
            "output_sha256",
            record.output_sha256.as_str(),
            case.golden_sha256,
        ),
        (
            "environment_id",
            record.environment_id.as_str(),
            case.environment_id,
        ),
    ] {
        if actual != expected {
            return Err(CalibrationError::OfficeGolden(format!(
                "manifest field {field} mismatch for {}: actual={actual:?}, expected={expected:?}",
                case.id
            )));
        }
    }
    let expected_output = format!("{}.pdf", case.source);
    if record.output != expected_output {
        return Err(CalibrationError::OfficeGolden(format!(
            "manifest output mismatch for {}",
            case.id
        )));
    }
    Ok(())
}

fn load_conversion_manifest_identities(
    root: &Path,
) -> std::result::Result<BTreeMap<(String, String), ConversionManifestIdentity>, String> {
    let conversion_root = root.join("corpus_pdf_conv");
    let mut manifest_paths = fs::read_dir(&conversion_root)
        .map_err(|error| format!("could not scan {}: {error}", conversion_root.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("manifest.jsonl"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    manifest_paths.sort();

    let mut records = BTreeMap::new();
    for manifest_path in manifest_paths {
        let corpus = manifest_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid corpus path {}", manifest_path.display()))?;
        let manifest = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
        for (line_index, line) in manifest.lines().enumerate() {
            let record: Value = serde_json::from_str(line).map_err(|error| {
                format!(
                    "invalid JSON at {}:{}: {error}",
                    manifest_path.display(),
                    line_index + 1
                )
            })?;
            let string = |field: &str| {
                record
                    .get(field)
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .ok_or_else(|| {
                        format!(
                            "missing string field {field:?} at {}:{}",
                            manifest_path.display(),
                            line_index + 1
                        )
                    })
            };
            let source = string("file")?;
            let key = (corpus.to_string(), source.clone());
            let identity = ConversionManifestIdentity {
                status: string("status")?,
                reference_engine: string("reference_engine")?,
                source_sha256: string("source_sha256")?,
                output_sha256: string("output_sha256")?,
                environment_id: string("environment_id")?,
                output: string("output")?,
            };
            if records.insert(key, identity).is_some() {
                return Err(format!(
                    "duplicate conversion manifest record for {corpus}/{source}"
                ));
            }
        }
    }
    Ok(records)
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

fn assert_page_geometry_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<()> {
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
    Ok(())
}

fn assert_text_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<Vec<Vec<PdfBounds>>> {
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

fn write_failure_page_artifacts(
    artifact_dir: &Path,
    page_index: usize,
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
) -> Result<()> {
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
    Ok(())
}

fn format_page_ranges(page_indices: &[usize]) -> String {
    let Some((&first, rest)) = page_indices.split_first() else {
        return "none".to_string();
    };
    let mut ranges = Vec::new();
    let mut start = first;
    let mut end = first;
    for &page_index in rest {
        if page_index == end + 1 {
            end = page_index;
            continue;
        }
        ranges.push(format_page_range(start, end));
        start = page_index;
        end = page_index;
    }
    ranges.push(format_page_range(start, end));
    ranges.join(",")
}

fn format_page_range(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
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
    use super::{format_page_ranges, normalize_extracted_text};

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

    #[test]
    fn page_range_summary_coalesces_consecutive_failures() {
        assert_eq!(format_page_ranges(&[]), "none");
        assert_eq!(format_page_ranges(&[0]), "0");
        assert_eq!(format_page_ranges(&[0, 1, 2, 4, 7, 8]), "0-2,4,7-8");
    }
}

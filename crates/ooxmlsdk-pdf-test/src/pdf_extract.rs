use std::collections::BTreeSet;
use std::io::Read;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::ZlibDecoder;
use image::GenericImageView;
use lopdf::{Document as LopdfDocument, Object as LopdfObject};
use ooxmlsdk_fonts::FontFaceInfo;
use pdfium_render::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfSummary {
    pub page_count: usize,
    pub image_count: usize,
    pub link_annotation_count: usize,
    pub annotation_count: usize,
    pub outline_marker_count: usize,
    pub media_box_count: usize,
    pub contains_eof: bool,
    pub media_boxes: Vec<String>,
    pub text: Option<String>,
    pub text_error: Option<String>,
    pub text_segments: Vec<TextSegmentSummary>,
    pub text_chars: Vec<TextCharSummary>,
    pub text_objects: Vec<TextObjectSummary>,
    pub images: Vec<ImageSummary>,
    pub paths: Vec<PathObjectSummary>,
    pub links: Vec<LinkSummary>,
    pub annotations: Vec<AnnotationSummary>,
    pub outlines: Vec<String>,
    pub raw_pages: Vec<RawPageSummary>,
    pub page_objects: Vec<PageObjectSummary>,
    pub content: ContentSummary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PdfFontStructureSummary {
    pub pages: Vec<PdfPageFontStructure>,
    pub actual_text_span_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PdfPageFontStructure {
    pub page_index: usize,
    pub fonts: Vec<PdfFontResourceSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PdfFontResourceSummary {
    pub resource_path: String,
    pub subtype: Option<String>,
    pub base_font: Option<String>,
    pub encoding: Option<String>,
    pub descendant_subtype: Option<String>,
    pub descendant_base_font: Option<String>,
    pub first_char: Option<i64>,
    pub last_char: Option<i64>,
    pub simple_width_count: Option<usize>,
    pub cid_width_entry_count: Option<usize>,
    pub descriptor_flags: Option<i64>,
    pub font_bounds: Option<String>,
    pub ascent: Option<String>,
    pub descent: Option<String>,
    pub cap_height: Option<String>,
    pub italic_angle: Option<String>,
    pub embedded_font_kind: Option<String>,
    pub has_to_unicode: bool,
    pub to_unicode_mapping_count: Option<usize>,
    pub to_unicode_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageSummary {
    pub page_index: usize,
    pub(crate) page_image_index: usize,
    pub width: Option<String>,
    pub height: Option<String>,
    pub bounds: Option<String>,
    pub decoded_pixel_sha256: Option<String>,
    pub(crate) decoded_vertical_flip_sha256: Option<String>,
    pub(crate) axis_aligned_orientation: Option<(i8, i8)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathObjectSummary {
    pub page_index: usize,
    pub segments: u32,
    pub fill_mode: Option<String>,
    pub stroked: Option<bool>,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub bounds: Option<String>,
    pub segment_details: Vec<PathSegmentSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathSegmentSummary {
    pub segment_type: String,
    pub x: String,
    pub y: String,
    pub closed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkSummary {
    pub page_index: usize,
    pub target_kind: LinkTargetKind,
    pub target: Option<String>,
    pub rect: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationSummary {
    pub page_index: usize,
    pub annotation_type: String,
    pub bounds: Option<String>,
    pub action_uri: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RawPageSummary {
    pub page_index: usize,
    pub annotation_count: usize,
    pub annotations: Vec<RawAnnotationSummary>,
    pub xobjects: Vec<RawXObjectSummary>,
    pub(crate) image_draw_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawAnnotationSummary {
    pub page_index: usize,
    pub type_name: Option<String>,
    pub subtype_name: Option<String>,
    pub rect: Option<String>,
    pub action_uri: Option<String>,
    pub field_type_name: Option<String>,
    pub field_value: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawXObjectSummary {
    pub page_index: usize,
    pub name: String,
    pub type_name: Option<String>,
    pub subtype_name: Option<String>,
    pub filter_names: Vec<String>,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub image_format: Option<String>,
    pub decoded_width_px: Option<u32>,
    pub decoded_height_px: Option<u32>,
    pub bits_per_pixel: Option<u16>,
    pub decoded_stream_sha256: Option<String>,
    pub has_soft_mask: bool,
    pub soft_mask_has_opaque_core: Option<bool>,
    pub solid_rgb: Option<[u8; 3]>,
    pub(crate) semantic_soft_mask_image: Option<SemanticSoftMaskImage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SemanticSoftMaskImage {
    pub(crate) alpha: Vec<u8>,
    pub(crate) black_matte_rgb: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTargetKind {
    ExternalUri,
    InternalDestination,
    Action,
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContentSummary {
    pub stream_count: usize,
    pub decoded_stream_count: usize,
    pub text_show_ops: usize,
    pub image_draw_ops: usize,
    pub path_paint_ops: usize,
    pub clipping_ops: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextSegmentSummary {
    pub page_index: usize,
    pub text: String,
    pub bounds: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextCharSummary {
    pub page_index: usize,
    pub text: String,
    pub font_name: String,
    pub bounds: String,
    pub font_metric_bounds: Option<String>,
    pub origin_x: String,
    pub origin_y: String,
    pub angle_degrees: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextObjectSummary {
    pub page_index: usize,
    pub text: String,
    pub font_name: String,
    pub font_family: String,
    pub scaled_font_size: String,
    pub unscaled_font_size: String,
    pub render_mode: String,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub bounds: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PageObjectSummary {
    pub page_index: usize,
    pub text_objects: usize,
    pub path_objects: usize,
    pub image_objects: usize,
    pub shading_objects: usize,
    pub form_objects: usize,
    pub unsupported_objects: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PdfBounds {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelRect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedPageImage {
    pub page_index: usize,
    pub width_px: u32,
    pub height_px: u32,
    pub page_width_pt: f32,
    pub page_height_pt: f32,
    pub rgba_crc32: String,
    pub rgba: Vec<u8>,
}

impl PdfBounds {
    pub fn width(self) -> f32 {
        self.right - self.left
    }

    pub fn height(self) -> f32 {
        self.top - self.bottom
    }

    pub fn center(self) -> (f32, f32) {
        (
            self.left + self.width() / 2.0,
            self.bottom + self.height() / 2.0,
        )
    }
}

impl RenderedPageImage {
    pub fn pixel_rgba(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width_px || y >= self.height_px {
            return None;
        }
        let offset = ((y * self.width_px + x) * 4) as usize;
        Some([
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
            self.rgba[offset + 3],
        ])
    }

    pub fn sample_pdf_point_rgba(&self, x_pt: f32, y_pt: f32) -> Option<[u8; 4]> {
        let (x, y) = self.pdf_point_to_pixel(x_pt, y_pt)?;
        self.pixel_rgba(x, y)
    }

    pub fn sample_pdf_rect_center_rgba(&self, rect: PdfBounds) -> Option<[u8; 4]> {
        let (x, y) = rect.center();
        self.sample_pdf_point_rgba(x, y)
    }

    pub fn pixel_region_crc32(&self, rect: PixelRect) -> Option<String> {
        if rect.width == 0
            || rect.height == 0
            || rect.left >= self.width_px
            || rect.top >= self.height_px
            || rect.left + rect.width > self.width_px
            || rect.top + rect.height > self.height_px
        {
            return None;
        }

        let mut crc = crc32fast::Hasher::new();
        for y in rect.top..rect.top + rect.height {
            let start = ((y * self.width_px + rect.left) * 4) as usize;
            let end = start + (rect.width * 4) as usize;
            crc.update(&self.rgba[start..end]);
        }
        Some(format!("{:08x}", crc.finalize()))
    }

    pub fn pdf_rect_crc32(&self, rect: PdfBounds) -> Option<String> {
        self.pixel_region_crc32(self.pdf_rect_to_pixel_rect(rect)?)
    }

    pub fn pdf_point_to_pixel(&self, x_pt: f32, y_pt: f32) -> Option<(u32, u32)> {
        if !(0.0..=self.page_width_pt).contains(&x_pt)
            || !(0.0..=self.page_height_pt).contains(&y_pt)
        {
            return None;
        }

        let x = (x_pt / self.page_width_pt * self.width_px as f32).floor() as u32;
        let y = ((self.page_height_pt - y_pt) / self.page_height_pt * self.height_px as f32).floor()
            as u32;
        Some((x.min(self.width_px - 1), y.min(self.height_px - 1)))
    }

    pub fn pdf_rect_to_pixel_rect(&self, rect: PdfBounds) -> Option<PixelRect> {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return None;
        }
        let (left, top) = self.pdf_point_to_pixel(rect.left, rect.top)?;
        let (right, bottom) = self.pdf_point_to_pixel(rect.right, rect.bottom)?;
        Some(PixelRect {
            left,
            top,
            width: right.saturating_sub(left).max(1),
            height: bottom.saturating_sub(top).max(1),
        })
    }
}

impl PdfSummary {
    pub fn from_bytes(pdf: &[u8]) -> Result<Self, String> {
        Self::from_bytes_inner(pdf, true)
    }

    pub(crate) fn from_bytes_for_golden(pdf: &[u8]) -> Result<Self, String> {
        Self::from_bytes_inner(pdf, false)
    }

    fn from_bytes_inner(pdf: &[u8], run_pdftotext: bool) -> Result<Self, String> {
        let text = String::from_utf8_lossy(pdf);
        let streams = pdf_streams(pdf);
        let pdfium_summary = pdfium_summary(pdf)?;
        let raw_summary = raw_pdf_summary(pdf)?;
        let pdftotext = run_pdftotext.then(|| pdftotext(pdf));
        Ok(Self {
            page_count: pdfium_summary.page_count,
            image_count: pdfium_summary.images.len(),
            link_annotation_count: pdfium_summary.links.len(),
            annotation_count: pdfium_summary.annotations.len(),
            outline_marker_count: text.matches("/Outlines").count()
                + text.matches("/Title").count(),
            media_box_count: pdfium_summary.media_boxes.len(),
            contains_eof: pdf.strip_suffix_ascii_whitespace().ends_with(b"%%EOF"),
            media_boxes: pdfium_summary.media_boxes,
            text: pdftotext.as_ref().and_then(|result| result.clone().ok()),
            text_error: pdftotext.and_then(Result::err),
            text_segments: pdfium_summary.text_segments,
            text_chars: pdfium_summary.text_chars,
            text_objects: pdfium_summary.text_objects,
            images: pdfium_summary.images,
            paths: pdfium_summary.paths,
            links: pdfium_summary.links,
            annotations: pdfium_summary.annotations,
            outlines: raw_summary.outlines,
            raw_pages: raw_summary.pages,
            page_objects: pdfium_summary.page_objects,
            content: content_summary(&streams),
        })
    }
}

pub fn pdf_page_count(pdf: &[u8]) -> Result<usize, String> {
    let document = LopdfDocument::load_mem(pdf)
        .map_err(|error| format!("lopdf could not load PDF bytes: {error}"))?;
    Ok(document.get_pages().len())
}

pub(crate) fn pdf_page_dimensions(pdf: &[u8]) -> Result<Vec<(f32, f32)>, String> {
    let document = LopdfDocument::load_mem(pdf)
        .map_err(|error| format!("lopdf could not load PDF bytes: {error}"))?;
    document
        .page_iter()
        .enumerate()
        .map(|(page_index, page_id)| {
            let media_box = inherited_page_value(&document, page_id, b"MediaBox")?
                .ok_or_else(|| format!("PDF page {page_index} has no inherited MediaBox"))?;
            let media_box = resolve_pdf_object(&document, media_box)?
                .as_array()
                .map_err(|error| {
                    format!("PDF page {page_index} MediaBox is not an array: {error}")
                })?;
            if media_box.len() != 4 {
                return Err(format!(
                    "PDF page {page_index} MediaBox has {} coordinates",
                    media_box.len()
                ));
            }
            let coordinates =
                media_box
                    .iter()
                    .map(|value| {
                        resolve_pdf_object(&document, value)?.as_float().map_err(|error| {
                    format!("PDF page {page_index} MediaBox coordinate is not numeric: {error}")
                })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
            let mut width = (coordinates[2] - coordinates[0]).abs();
            let mut height = (coordinates[3] - coordinates[1]).abs();
            let rotation = inherited_page_value(&document, page_id, b"Rotate")?
                .map(|value| resolve_pdf_object(&document, value))
                .transpose()?
                .and_then(|value| value.as_i64().ok())
                .unwrap_or_default()
                .rem_euclid(360);
            if rotation == 90 || rotation == 270 {
                std::mem::swap(&mut width, &mut height);
            }
            Ok((width, height))
        })
        .collect()
}

pub fn pdf_font_structure(pdf: &[u8]) -> Result<PdfFontStructureSummary, String> {
    // Krilla's text snapshots assert the PDF font dictionary, descendant
    // font, descriptor, widths, embedding stream, and ToUnicode separately
    // from raster output. Keep the broad Office harness bounded by recording
    // the same structural facts as summaries rather than copying font streams.
    let document = LopdfDocument::load_mem(pdf)
        .map_err(|error| format!("lopdf could not load PDF bytes: {error}"))?;
    let mut pages = Vec::new();
    for (page_number, page_id) in document.get_pages() {
        let page_index = page_number as usize - 1;
        let page = document
            .get_dictionary(page_id)
            .map_err(|error| format!("lopdf could not read page {page_index}: {error}"))?;
        let resources = match inherited_page_dictionary_value(&document, page, b"Resources") {
            Ok(Some(resources)) => {
                lopdf_dictionary_owned(&document, resources, "page font Resources")?
            }
            Ok(None) | Err(lopdf::Error::DictKey(_)) => lopdf::Dictionary::new(),
            Err(error) => {
                return Err(format!(
                    "lopdf could not read page {page_index} font Resources: {error}"
                ));
            }
        };
        let mut fonts = Vec::new();
        let mut visited_forms = BTreeSet::new();
        collect_resource_fonts(
            &document,
            "page",
            &resources,
            &mut visited_forms,
            &mut fonts,
        )?;
        fonts.sort_by(|left, right| left.resource_path.cmp(&right.resource_path));
        pages.push(PdfPageFontStructure { page_index, fonts });
    }
    pages.sort_by_key(|page| page.page_index);
    let actual_text_span_count = actual_text_span_count(&document);
    Ok(PdfFontStructureSummary {
        pages,
        actual_text_span_count,
    })
}

fn actual_text_span_count(document: &LopdfDocument) -> usize {
    document
        .objects
        .values()
        .filter_map(|object| object.as_stream().ok())
        .filter_map(|stream| stream.get_plain_content().ok())
        .map(|content| {
            content
                .windows(b"/ActualText".len())
                .filter(|window| *window == b"/ActualText")
                .count()
        })
        .sum()
}

fn collect_resource_fonts(
    document: &LopdfDocument,
    prefix: &str,
    resources: &lopdf::Dictionary,
    visited_forms: &mut BTreeSet<lopdf::ObjectId>,
    fonts: &mut Vec<PdfFontResourceSummary>,
) -> Result<(), String> {
    if let Ok(fonts_object) = resources.get(b"Font") {
        let font_resources = lopdf_dictionary(document, fonts_object, "font resources")?;
        for (name, font) in font_resources.iter() {
            let resource_path = format!("{prefix}/{}", String::from_utf8_lossy(name));
            fonts.push(font_resource_summary(document, resource_path, font)?);
        }
    }

    let Ok(xobjects_object) = resources.get(b"XObject") else {
        return Ok(());
    };
    let xobjects = lopdf_dictionary(document, xobjects_object, "font resource XObjects")?;
    for (name, object) in xobjects.iter() {
        if let Ok(object_id) = object.as_reference()
            && !visited_forms.insert(object_id)
        {
            continue;
        }
        let Ok(stream) = lopdf_stream(document, object, "font resource XObject") else {
            continue;
        };
        if stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|value| lopdf_name(document, value).ok())
            .as_deref()
            != Some("Form")
        {
            continue;
        }
        let Ok(nested_resources_object) = stream.dict.get(b"Resources") else {
            continue;
        };
        let nested_resources =
            lopdf_dictionary(document, nested_resources_object, "form font Resources")?;
        let nested_prefix = format!("{prefix}/{}", String::from_utf8_lossy(name));
        collect_resource_fonts(
            document,
            &nested_prefix,
            nested_resources,
            visited_forms,
            fonts,
        )?;
    }
    Ok(())
}

fn font_resource_summary(
    document: &LopdfDocument,
    resource_path: String,
    object: &LopdfObject,
) -> Result<PdfFontResourceSummary, String> {
    let font = lopdf_dictionary(document, object, "font resource")?;
    let descendant = font
        .get(b"DescendantFonts")
        .ok()
        .and_then(|value| lopdf_array(document, value, "DescendantFonts").ok())
        .and_then(|fonts| fonts.first())
        .and_then(|font| lopdf_dictionary(document, font, "descendant font").ok());
    let descriptor = descendant
        .and_then(|font| font.get(b"FontDescriptor").ok())
        .or_else(|| font.get(b"FontDescriptor").ok())
        .and_then(|descriptor| lopdf_dictionary(document, descriptor, "font descriptor").ok());
    let encoding = font.get(b"Encoding").ok().and_then(|encoding| {
        lopdf_name(document, encoding).ok().or_else(|| {
            lopdf_dictionary(document, encoding, "font Encoding")
                .ok()
                .and_then(|encoding| encoding.get(b"BaseEncoding").ok())
                .and_then(|base| lopdf_name(document, base).ok())
        })
    });
    let embedded_font_kind = descriptor.and_then(|descriptor| {
        [b"FontFile".as_slice(), b"FontFile2", b"FontFile3"]
            .into_iter()
            .find(|key| descriptor.has(key))
            .map(|key| String::from_utf8_lossy(key).to_string())
    });
    let (has_to_unicode, to_unicode_mapping_count, to_unicode_error) = match font.get(b"ToUnicode")
    {
        Ok(object) => match to_unicode_mapping_count(document, object) {
            Ok(count) => (true, Some(count), None),
            Err(error) => (true, None, Some(error)),
        },
        Err(_) => (false, None, None),
    };
    Ok(PdfFontResourceSummary {
        resource_path,
        subtype: font
            .get(b"Subtype")
            .ok()
            .and_then(|value| lopdf_name(document, value).ok()),
        base_font: font
            .get(b"BaseFont")
            .ok()
            .and_then(|value| lopdf_name(document, value).ok())
            .map(|name| normalize_pdf_font_name(&name)),
        encoding,
        descendant_subtype: descendant
            .and_then(|font| font.get(b"Subtype").ok())
            .and_then(|value| lopdf_name(document, value).ok()),
        descendant_base_font: descendant
            .and_then(|font| font.get(b"BaseFont").ok())
            .and_then(|value| lopdf_name(document, value).ok())
            .map(|name| normalize_pdf_font_name(&name)),
        first_char: font
            .get(b"FirstChar")
            .ok()
            .and_then(|value| lopdf_i64(document, value).ok()),
        last_char: font
            .get(b"LastChar")
            .ok()
            .and_then(|value| lopdf_i64(document, value).ok()),
        simple_width_count: font
            .get(b"Widths")
            .ok()
            .and_then(|value| lopdf_array(document, value, "font Widths").ok())
            .map(Vec::len),
        cid_width_entry_count: descendant
            .and_then(|font| font.get(b"W").ok())
            .and_then(|value| lopdf_array(document, value, "CID font W").ok())
            .map(Vec::len),
        descriptor_flags: descriptor
            .and_then(|descriptor| descriptor.get(b"Flags").ok())
            .and_then(|value| lopdf_i64(document, value).ok()),
        font_bounds: descriptor
            .and_then(|descriptor| descriptor.get(b"FontBBox").ok())
            .and_then(|value| lopdf_numeric_array(document, value).ok()),
        ascent: descriptor
            .and_then(|descriptor| descriptor.get(b"Ascent").ok())
            .and_then(|value| lopdf_number(document, value).ok()),
        descent: descriptor
            .and_then(|descriptor| descriptor.get(b"Descent").ok())
            .and_then(|value| lopdf_number(document, value).ok()),
        cap_height: descriptor
            .and_then(|descriptor| descriptor.get(b"CapHeight").ok())
            .and_then(|value| lopdf_number(document, value).ok()),
        italic_angle: descriptor
            .and_then(|descriptor| descriptor.get(b"ItalicAngle").ok())
            .and_then(|value| lopdf_number(document, value).ok()),
        embedded_font_kind,
        has_to_unicode,
        to_unicode_mapping_count,
        to_unicode_error,
    })
}

fn to_unicode_mapping_count(
    document: &LopdfDocument,
    object: &LopdfObject,
) -> Result<usize, String> {
    let stream = lopdf_stream(document, object, "font ToUnicode CMap")?;
    let bytes = stream
        .get_plain_content()
        .map_err(|error| format!("could not decode font ToUnicode CMap: {error}"))?;
    let cmap = String::from_utf8_lossy(&bytes);
    for marker in [
        "begincmap",
        "endcmap",
        "begincodespacerange",
        "endcodespacerange",
    ] {
        if !cmap.contains(marker) {
            return Err(format!("font ToUnicode CMap is missing {marker}"));
        }
    }
    let tokens = cmap.split_ascii_whitespace().collect::<Vec<_>>();
    let mut mappings = 0usize;
    for pair in tokens.windows(2) {
        if matches!(pair[1], "beginbfchar" | "beginbfrange") {
            let count = pair[0].parse::<usize>().map_err(|error| {
                format!("invalid ToUnicode mapping count {:?}: {error}", pair[0])
            })?;
            mappings = mappings
                .checked_add(count)
                .ok_or_else(|| "font ToUnicode mapping count overflow".to_string())?;
        }
    }
    if mappings == 0 {
        return Err("font ToUnicode CMap has no bfchar or bfrange mappings".to_string());
    }
    Ok(mappings)
}

fn lopdf_i64(document: &LopdfDocument, object: &LopdfObject) -> Result<i64, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference integer: {error}"))?;
    object
        .as_i64()
        .map_err(|error| format!("lopdf expected integer: {error}"))
}

fn lopdf_number(document: &LopdfDocument, object: &LopdfObject) -> Result<String, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference number: {error}"))?;
    object
        .as_float()
        .map(|value| format!("{value:.4}"))
        .map_err(|error| format!("lopdf expected number: {error}"))
}

fn lopdf_numeric_array(document: &LopdfDocument, object: &LopdfObject) -> Result<String, String> {
    let values = lopdf_array(document, object, "numeric array")?;
    let values = values
        .iter()
        .map(|value| lopdf_number(document, value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", values.join(" ")))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PdfPageTextMismatch {
    pub(crate) page_index: usize,
    pub(crate) candidate: String,
    pub(crate) golden: String,
}

pub(crate) fn first_pdf_page_text_mismatch(
    candidate_pdf: &[u8],
    golden_pdf: &[u8],
    mut normalize: impl FnMut(&str) -> String,
) -> Result<Option<PdfPageTextMismatch>, String> {
    // The golden ratchet rejects normalized text before it needs character
    // geometry, page objects, raw XObjects, or raster output. Compare one page
    // at a time while both documents are open so a mismatch on an early page
    // does not build full-document PdfSummary values for large workbooks.
    let _guard = pdfium_lock().lock().unwrap();
    let pdfium = bind_pdfium()?;
    let candidate = pdfium
        .load_pdf_from_byte_slice(candidate_pdf, None)
        .map_err(|error| format!("PDFium could not load candidate PDF bytes: {error}"))?;
    let golden = pdfium
        .load_pdf_from_byte_slice(golden_pdf, None)
        .map_err(|error| format!("PDFium could not load golden PDF bytes: {error}"))?;
    if candidate.pages().len() != golden.pages().len() {
        return Err(format!(
            "PDF text page counts differ: candidate={}, golden={}",
            candidate.pages().len(),
            golden.pages().len()
        ));
    }

    for (page_index, (candidate_page, golden_page)) in candidate
        .pages()
        .iter()
        .zip(golden.pages().iter())
        .enumerate()
    {
        let candidate_text = normalized_pdfium_page_text(page_index, &candidate_page)?;
        let golden_text = normalized_pdfium_page_text(page_index, &golden_page)?;
        let candidate_text = normalize(&candidate_text);
        let golden_text = normalize(&golden_text);
        if candidate_text != golden_text {
            // FPDFText_GetUnicode returns PDFium's post-bidi character stream.
            // In mixed-direction runs it can mirror a neutral bracket even
            // though the PDF's ToUnicode mapping still contains the authored
            // codepoint. Confirm a content mismatch with Poppler before the
            // lightweight preflight is allowed to reject the document. This
            // fallback is cold: matching PDFium pages never start a process.
            let candidate_fallback = normalize(&pdftotext_page(candidate_pdf, page_index)?);
            let golden_fallback = normalize(&pdftotext_page(golden_pdf, page_index)?);
            if candidate_fallback == golden_fallback {
                continue;
            }
            return Ok(Some(PdfPageTextMismatch {
                page_index,
                candidate: candidate_text,
                golden: golden_text,
            }));
        }
    }
    Ok(None)
}

fn normalized_pdfium_page_text(page_index: usize, page: &PdfPage<'_>) -> Result<String, String> {
    let text = page
        .text()
        .map_err(|error| format!("PDFium could not extract page {page_index} text: {error}"))?;
    let pdfium_text_characters = text.chars();
    let mut characters = Vec::with_capacity(pdfium_text_characters.len());
    let mut pdfium_characters = pdfium_text_characters.iter().peekable();
    while let Some(character) = pdfium_characters.next() {
        let (decoded, consume_next) = decode_pdfium_unicode_scalar(
            character.unicode_value(),
            pdfium_characters
                .peek()
                .map(|character| character.unicode_value()),
        );
        if consume_next {
            pdfium_characters.next();
        }
        characters.push(decoded.map(|character| character.to_string()));
    }
    Ok(normalized_pdfium_page_text_from_parts(
        characters,
        text.segments().iter().map(|segment| segment.text()),
    ))
}

fn decode_pdfium_unicode_scalar(value: u32, next: Option<u32>) -> (Option<char>, bool) {
    if (0xd800..=0xdbff).contains(&value) {
        let Some(low) = next.filter(|low| (0xdc00..=0xdfff).contains(low)) else {
            return (None, false);
        };
        let scalar = 0x1_0000 + ((value - 0xd800) << 10) + (low - 0xdc00);
        return (char::from_u32(scalar), true);
    }
    (char::from_u32(value), false)
}

fn normalized_pdfium_page_text_from_parts<C, Character, S, Segment>(
    characters: C,
    segments: S,
) -> String
where
    C: IntoIterator<Item = Option<Character>>,
    Character: AsRef<str>,
    S: IntoIterator<Item = Segment>,
    Segment: AsRef<str>,
{
    // PdfSummary's authoritative text contract uses the character stream when
    // PDFium exposes one and falls back to segments only for pages without
    // character data. Tagged Office PDFs can repeat overlapping marked
    // content through the segment API (testPageref.docx), so the lightweight
    // preflight must make the same choice before it is allowed to reject a
    // corpus case.
    let mut character_text = String::new();
    let mut saw_character = false;
    for character in characters.into_iter().flatten() {
        saw_character = true;
        character_text.push_str(character.as_ref());
    }
    if saw_character {
        return normalize_extracted_text(&character_text);
    }

    let mut segment_text = String::new();
    for segment in segments {
        if !segment_text.is_empty() {
            segment_text.push(' ');
        }
        segment_text.push_str(&normalize_extracted_text(segment.as_ref()));
    }
    normalize_extracted_text(&segment_text)
}

fn inherited_page_value<'a>(
    document: &'a LopdfDocument,
    mut object_id: lopdf::ObjectId,
    key: &[u8],
) -> Result<Option<&'a LopdfObject>, String> {
    for _ in 0..64 {
        let dictionary = document
            .get_object(object_id)
            .map_err(|error| format!("could not load PDF page tree object {object_id:?}: {error}"))?
            .as_dict()
            .map_err(|error| {
                format!("PDF page tree object {object_id:?} is not a dictionary: {error}")
            })?;
        if let Ok(value) = dictionary.get(key) {
            return Ok(Some(value));
        }
        let Ok(parent) = dictionary.get(b"Parent") else {
            return Ok(None);
        };
        object_id = parent
            .as_reference()
            .map_err(|error| format!("PDF page tree Parent is not a reference: {error}"))?;
    }
    Err("PDF page tree exceeds 64 inherited levels".to_string())
}

fn resolve_pdf_object<'a>(
    document: &'a LopdfDocument,
    mut object: &'a LopdfObject,
) -> Result<&'a LopdfObject, String> {
    for _ in 0..64 {
        let LopdfObject::Reference(object_id) = object else {
            return Ok(object);
        };
        object = document
            .get_object(*object_id)
            .map_err(|error| format!("could not resolve PDF object {object_id:?}: {error}"))?;
    }
    Err("PDF object reference chain exceeds 64 levels".to_string())
}

pub fn parse_pdf_rect(rect: &str) -> Result<PdfBounds, String> {
    let trimmed = rect
        .trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("invalid PDF rect format: {rect}"))?;
    let values = trimmed
        .split_ascii_whitespace()
        .map(|value| {
            value
                .parse::<f32>()
                .map_err(|error| format!("invalid PDF rect coordinate {value:?}: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != 4 {
        return Err(format!(
            "expected four PDF rect coordinates, got {} in {rect}",
            values.len()
        ));
    }
    Ok(PdfBounds {
        left: values[0],
        bottom: values[1],
        right: values[2],
        top: values[3],
    })
}

pub fn assert_pdf_rect_close(actual: &str, expected: PdfBounds, tolerance: f32) {
    let actual = parse_pdf_rect(actual).unwrap();
    for (name, actual_value, expected_value) in [
        ("left", actual.left, expected.left),
        ("bottom", actual.bottom, expected.bottom),
        ("right", actual.right, expected.right),
        ("top", actual.top, expected.top),
    ] {
        assert!(
            (actual_value - expected_value).abs() <= tolerance,
            "PDF rect {name} mismatch: actual={actual:?} expected={expected:?} tolerance={tolerance}"
        );
    }
}

pub fn rendered_page_image_from_pdf(
    pdf: &[u8],
    page_index: usize,
    target_width: i32,
) -> Result<RenderedPageImage, String> {
    let _guard = pdfium_lock().lock().unwrap();
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_byte_slice(pdf, None)
        .map_err(|error| format!("PDFium could not load PDF bytes: {error}"))?;

    for (current_page_index, page) in document.pages().iter().enumerate() {
        if current_page_index == page_index {
            return rendered_page_image(page_index, &page, target_width, true);
        }
    }

    Err(format!(
        "PDF page index {page_index} is out of range; page count is {}",
        document.pages().len()
    ))
}

pub(crate) enum RenderedPagePairError<E> {
    Pdf(String),
    Visit(E),
}

pub(crate) fn visit_rendered_page_pairs<E>(
    candidate_pdf: &[u8],
    golden_pdf: &[u8],
    target_width: i32,
    mut visit: impl FnMut(usize, RenderedPageImage, RenderedPageImage) -> Result<(), E>,
) -> Result<(), RenderedPagePairError<E>> {
    // Load each PDF once for the complete visible-output pass. The previous
    // per-page helper reopened and copied both PDFs for every page, turning a
    // multi-page comparison into repeated whole-document work.
    let _guard = pdfium_lock().lock().unwrap();
    let pdfium = bind_pdfium().map_err(RenderedPagePairError::Pdf)?;
    let candidate = pdfium
        .load_pdf_from_byte_slice(candidate_pdf, None)
        .map_err(|error| {
            RenderedPagePairError::Pdf(format!(
                "PDFium could not load candidate PDF bytes: {error}"
            ))
        })?;
    let golden = pdfium
        .load_pdf_from_byte_slice(golden_pdf, None)
        .map_err(|error| {
            RenderedPagePairError::Pdf(format!("PDFium could not load golden PDF bytes: {error}"))
        })?;
    if candidate.pages().len() != golden.pages().len() {
        return Err(RenderedPagePairError::Pdf(format!(
            "rendered PDF page counts differ: candidate={}, golden={}",
            candidate.pages().len(),
            golden.pages().len()
        )));
    }

    for (page_index, (candidate_page, golden_page)) in candidate
        .pages()
        .iter()
        .zip(golden.pages().iter())
        .enumerate()
    {
        let candidate_image = rendered_page_image(page_index, &candidate_page, target_width, false)
            .map_err(RenderedPagePairError::Pdf)?;
        let golden_image = rendered_page_image(page_index, &golden_page, target_width, false)
            .map_err(RenderedPagePairError::Pdf)?;
        visit(page_index, candidate_image, golden_image).map_err(RenderedPagePairError::Visit)?;
    }
    Ok(())
}

pub fn raw_image_pixel_from_pdf(
    pdf: &[u8],
    image_width: u32,
    image_height: u32,
    source_x: u32,
    source_y: u32,
) -> Result<Option<[u8; 4]>, String> {
    let _guard = pdfium_lock().lock().unwrap();
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_byte_slice(pdf, None)
        .map_err(|error| format!("PDFium could not load PDF bytes: {error}"))?;

    for page in document.pages().iter() {
        for object in page.objects().iter() {
            if object.object_type() != PdfPageObjectType::Image {
                continue;
            }
            let Some(image) = object.as_image_object() else {
                continue;
            };
            if image.width().ok() != Some(image_width as i32)
                || image.height().ok() != Some(image_height as i32)
            {
                continue;
            }

            let bitmap = image
                .get_raw_bitmap()
                .map_err(|error| format!("PDFium could not extract raw image bitmap: {error}"))?;
            let width = bitmap.width() as u32;
            let height = bitmap.height() as u32;
            if source_x >= width || source_y >= height {
                return Ok(None);
            }
            let rgba = bitmap.as_rgba_bytes();
            let offset = ((source_y * width + source_x) * 4) as usize;
            return Ok(Some([
                rgba[offset],
                rgba[offset + 1],
                rgba[offset + 2],
                rgba[offset + 3],
            ]));
        }
    }

    Ok(None)
}

struct PdfiumSummary {
    page_count: usize,
    media_boxes: Vec<String>,
    text_segments: Vec<TextSegmentSummary>,
    text_chars: Vec<TextCharSummary>,
    text_objects: Vec<TextObjectSummary>,
    images: Vec<ImageSummary>,
    paths: Vec<PathObjectSummary>,
    links: Vec<LinkSummary>,
    annotations: Vec<AnnotationSummary>,
    page_objects: Vec<PageObjectSummary>,
}

pub(crate) struct RawPdfSummary {
    pub(crate) pages: Vec<RawPageSummary>,
    pub(crate) outlines: Vec<String>,
}

fn pdfium_summary(pdf: &[u8]) -> Result<PdfiumSummary, String> {
    let font_metrics = embedded_font_vertical_metrics(pdf)?;
    // PDFium extraction is not reliably parallel-safe in this harness.
    let _guard = pdfium_lock().lock().unwrap();
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_byte_slice(pdf, None)
        .map_err(|error| format!("PDFium could not load PDF bytes: {error}"))?;

    let mut media_boxes = Vec::new();
    let mut text_segments = Vec::new();
    let mut text_chars = Vec::new();
    let mut text_objects = Vec::new();
    let mut images = Vec::new();
    let mut paths = Vec::new();
    let mut links = Vec::new();
    let mut annotations = Vec::new();
    let mut page_objects = Vec::new();

    for (page_index, page) in document.pages().iter().enumerate() {
        let mut page_image_index = 0;
        media_boxes.push(format!(
            "[0.0 0.0 {:.2} {:.2}]",
            page.width().value,
            page.height().value
        ));

        let text = page
            .text()
            .map_err(|error| format!("PDFium could not extract page {page_index} text: {error}"))?;
        for segment in text.segments().iter() {
            text_segments.push(TextSegmentSummary {
                page_index,
                text: normalize_extracted_text(&segment.text()),
                bounds: format_rect(segment.bounds(), 2),
            });
        }
        let pdfium_text_characters = text.chars();
        let mut characters = pdfium_text_characters.iter().peekable();
        while let Some(character) = characters.next() {
            let (value, consume_next) = decode_pdfium_unicode_scalar(
                character.unicode_value(),
                characters.peek().map(|character| character.unicode_value()),
            );
            if consume_next {
                characters.next();
            }
            if let Some(value) = value {
                let bounds = character
                    .loose_bounds()
                    .map_err(|error| format!("PDFium could not read char bounds: {error}"))?;
                let (origin_x, origin_y) = character
                    .origin()
                    .map_err(|error| format!("PDFium could not read char origin: {error}"))?;
                let matrix = character
                    .matrix()
                    .map_err(|error| format!("PDFium could not read char matrix: {error}"))?;
                let angle_degrees = text_matrix_writing_angle_degrees(matrix.a(), matrix.b());
                let font_name = normalize_pdf_font_name(&character.font_name());
                let font_metric_bounds = font_metrics
                    .get(&font_name)
                    .filter(|_| angle_degrees.abs() <= f32::EPSILON)
                    .and_then(|metrics| {
                        font_metric_bounds(
                            bounds,
                            origin_y.value,
                            character.scaled_font_size().value,
                            *metrics,
                        )
                    });
                text_chars.push(TextCharSummary {
                    page_index,
                    text: value.to_string(),
                    font_name,
                    bounds: format_rect(bounds, 2),
                    font_metric_bounds,
                    origin_x: format_points(origin_x, 2),
                    origin_y: format_points(origin_y, 2),
                    angle_degrees: format!("{angle_degrees:.2}"),
                });
            }
        }

        let mut object_summary = PageObjectSummary {
            page_index,
            ..PageObjectSummary::default()
        };
        for object in page.objects().iter() {
            match object.object_type() {
                PdfPageObjectType::Text => {
                    object_summary.text_objects += 1;
                    if let Some(text) = object.as_text_object() {
                        let font = text.font();
                        text_objects.push(TextObjectSummary {
                            page_index,
                            text: normalize_extracted_text(&text.text()),
                            font_name: normalize_pdf_font_name(&font.name()),
                            font_family: normalize_pdf_font_name(&font.family()),
                            scaled_font_size: format_points(text.scaled_font_size(), 2),
                            unscaled_font_size: format_points(text.unscaled_font_size(), 2),
                            render_mode: format_text_render_mode(text.render_mode()),
                            fill_color: object.fill_color().ok().map(format_color),
                            stroke_color: object.stroke_color().ok().map(format_color),
                            bounds: object
                                .bounds()
                                .ok()
                                .map(|bounds| format_rect(bounds.to_rect(), 2)),
                        });
                    }
                }
                PdfPageObjectType::Path => {
                    object_summary.path_objects += 1;
                    if let Some(path) = object.as_path_object() {
                        let segments = path.segments();
                        paths.push(PathObjectSummary {
                            page_index,
                            segments: segments.len(),
                            fill_mode: path.fill_mode().ok().map(|value| format!("{value:?}")),
                            stroked: path.is_stroked().ok(),
                            fill_color: object.fill_color().ok().map(format_color),
                            stroke_color: object.stroke_color().ok().map(format_color),
                            bounds: object
                                .bounds()
                                .ok()
                                .map(|bounds| format_rect(bounds.to_rect(), 2)),
                            segment_details: segments
                                .iter()
                                .map(|segment| PathSegmentSummary {
                                    segment_type: format!("{:?}", segment.segment_type()),
                                    x: format_points(segment.x(), 2),
                                    y: format_points(segment.y(), 2),
                                    closed: segment.is_close(),
                                })
                                .collect(),
                        });
                    }
                }
                PdfPageObjectType::Image => {
                    object_summary.image_objects += 1;
                    if let Some(image) = object.as_image_object() {
                        let width = image.width().ok();
                        let height = image.height().ok();
                        let decoded_hashes = small_image_decoded_pixel_sha256(image, width, height);
                        images.push(ImageSummary {
                            page_index,
                            page_image_index,
                            width: width.map(|value| value.to_string()),
                            height: height.map(|value| value.to_string()),
                            bounds: object
                                .bounds()
                                .ok()
                                .map(|bounds| format_rect(bounds.to_rect(), 2)),
                            decoded_pixel_sha256: decoded_hashes
                                .as_ref()
                                .map(|hashes| hashes.original.clone()),
                            decoded_vertical_flip_sha256: decoded_hashes
                                .map(|hashes| hashes.vertical_flip),
                            axis_aligned_orientation: axis_aligned_image_orientation(image),
                        });
                        page_image_index += 1;
                    }
                }
                PdfPageObjectType::Shading => object_summary.shading_objects += 1,
                PdfPageObjectType::XObjectForm => object_summary.form_objects += 1,
                PdfPageObjectType::Unsupported => object_summary.unsupported_objects += 1,
            }
        }
        page_objects.push(object_summary);

        for link in page.links().iter() {
            let (target_kind, target) = pdfium_link_target(&link);
            links.push(LinkSummary {
                page_index,
                target_kind,
                target,
                rect: link.rect().ok().map(|rect| format_rect(rect, 2)),
            });
        }

        for annotation in page.annotations().iter() {
            let action_uri = annotation
                .as_link_annotation()
                .and_then(|annotation| annotation.link().ok())
                .and_then(|link| link.action())
                .and_then(|action| action.as_uri_action().and_then(|action| action.uri().ok()))
                .map(|value| normalize_extracted_text(&value));
            annotations.push(AnnotationSummary {
                page_index,
                annotation_type: format!("{:?}", annotation.annotation_type()),
                bounds: annotation
                    .bounds()
                    .ok()
                    .map(|bounds| format_rect_with_precision(bounds, 3)),
                action_uri,
            });
        }
    }

    Ok(PdfiumSummary {
        page_count: document.pages().len() as usize,
        media_boxes,
        text_segments,
        text_chars,
        text_objects,
        images,
        paths,
        links,
        annotations,
        page_objects,
    })
}

fn text_matrix_writing_angle_degrees(a: f32, b: f32) -> f32 {
    // PDF text advances along the first matrix axis (a, b). PDFium's
    // FPDFText_GetCharAngle also reacts to the second-axis shear: Office math
    // uses `1 0 0.3333 1` for synthetic italic text, which PDFium reports as
    // 18.43 degrees even though its baseline is horizontal. Derive the writing
    // direction from the actual advance axis so shear and rotation remain
    // distinct.
    b.atan2(a).to_degrees()
}

struct DecodedImageHashes {
    original: String,
    vertical_flip: String,
}

fn small_image_decoded_pixel_sha256(
    image: &PdfPageImageObject<'_>,
    width: Option<Pixels>,
    height: Option<Pixels>,
) -> Option<DecodedImageHashes> {
    // Localized graphics diagnostics must not turn a corpus scan into an
    // eager decode of every source-resolution photograph. This is the same
    // bounded diagnostic budget used by the raw soft-mask classifier.
    const MAX_DIAGNOSTIC_DECODED_BYTES: usize = 256 * 1024;
    let width = usize::try_from(width?).ok()?;
    let expected_rgba_bytes = width
        .checked_mul(usize::try_from(height?).ok()?)?
        .checked_mul(4)?;
    if expected_rgba_bytes == 0 || expected_rgba_bytes > MAX_DIAGNOSTIC_DECODED_BYTES {
        return None;
    }
    // FPDFImageObj_GetBitmap is a cheap native-sample identity. It explicitly
    // omits the object transform and soft mask; those are compared separately
    // from the raw PDF resources without PDFium resampling below.
    let bitmap = image.get_raw_bitmap().ok()?;
    let pixels = bitmap.as_rgba_bytes();
    if pixels.len() != expected_rgba_bytes {
        return None;
    }
    let row_bytes = width.checked_mul(4)?;
    let mut vertical_flip = Sha256::new();
    for row in pixels.chunks_exact(row_bytes).rev() {
        vertical_flip.update(row);
    }
    Some(DecodedImageHashes {
        original: format!("{:x}", Sha256::digest(pixels)),
        vertical_flip: format!("{:x}", vertical_flip.finalize()),
    })
}

fn axis_aligned_image_orientation(image: &PdfPageImageObject<'_>) -> Option<(i8, i8)> {
    let matrix = image.matrix().ok()?;
    if matrix.b().abs() > f32::EPSILON
        || matrix.c().abs() > f32::EPSILON
        || matrix.a().abs() <= f32::EPSILON
        || matrix.d().abs() <= f32::EPSILON
    {
        return None;
    }
    Some((
        if matrix.a().is_sign_positive() { 1 } else { -1 },
        if matrix.d().is_sign_positive() { 1 } else { -1 },
    ))
}

#[derive(Clone, Copy, Debug)]
struct EmbeddedFontVerticalMetrics {
    ascent_em: f32,
    descent_em: f32,
}

fn embedded_font_vertical_metrics(
    pdf: &[u8],
) -> Result<std::collections::BTreeMap<String, EmbeddedFontVerticalMetrics>, String> {
    let document = LopdfDocument::load_mem(pdf)
        .map_err(|error| format!("lopdf could not load PDF font metrics: {error}"))?;
    let mut metrics = std::collections::BTreeMap::new();
    for object in document.objects.values() {
        let Ok(font) = object.as_dict() else {
            continue;
        };
        if font
            .get(b"Type")
            .ok()
            .and_then(|value| lopdf_name(&document, value).ok())
            .as_deref()
            != Some("Font")
        {
            continue;
        }
        let descendant = font
            .get(b"DescendantFonts")
            .ok()
            .and_then(|value| lopdf_array(&document, value, "font metric DescendantFonts").ok())
            .and_then(|fonts| fonts.first())
            .and_then(|font| lopdf_dictionary(&document, font, "font metric descendant").ok());
        let base_font = font
            .get(b"BaseFont")
            .ok()
            .and_then(|value| lopdf_name(&document, value).ok())
            .or_else(|| {
                descendant
                    .and_then(|font| font.get(b"BaseFont").ok())
                    .and_then(|value| lopdf_name(&document, value).ok())
            })
            .map(|name| normalize_pdf_font_name(&name));
        let Some(base_font) = base_font else {
            continue;
        };
        if metrics.contains_key(&base_font) {
            continue;
        }
        let descriptor = descendant
            .and_then(|font| font.get(b"FontDescriptor").ok())
            .or_else(|| font.get(b"FontDescriptor").ok())
            .and_then(|descriptor| {
                lopdf_dictionary(&document, descriptor, "font metric descriptor").ok()
            });
        let Some(font_stream) = descriptor
            .and_then(|descriptor| descriptor.get(b"FontFile2").ok())
            .and_then(|stream| lopdf_stream(&document, stream, "embedded TrueType font").ok())
        else {
            continue;
        };
        let Ok(data) = font_stream.get_plain_content() else {
            continue;
        };
        let Ok(face) = FontFaceInfo::from_ttf_bytes(&base_font, &data, 0) else {
            continue;
        };
        let vertical = face.metrics.vertical;
        if vertical.ascent_pt <= 0.0 || vertical.descent_pt < 0.0 {
            continue;
        }
        metrics.insert(
            base_font,
            EmbeddedFontVerticalMetrics {
                ascent_em: vertical.ascent_pt,
                descent_em: vertical.descent_pt,
            },
        );
    }
    Ok(metrics)
}

fn font_metric_bounds(
    horizontal_bounds: PdfRect,
    baseline_y: f32,
    font_size: f32,
    metrics: EmbeddedFontVerticalMetrics,
) -> Option<String> {
    if !baseline_y.is_finite() || !font_size.is_finite() || font_size <= 0.0 {
        return None;
    }
    let bottom = baseline_y - metrics.descent_em * font_size;
    let top = baseline_y + metrics.ascent_em * font_size;
    if !bottom.is_finite() || !top.is_finite() || top <= bottom {
        return None;
    }
    Some(format!(
        "[{:.2} {:.2} {:.2} {:.2}]",
        horizontal_bounds.left().value,
        bottom,
        horizontal_bounds.right().value,
        top
    ))
}

pub(crate) fn raw_pdf_summary(pdf: &[u8]) -> Result<RawPdfSummary, String> {
    raw_pdf_summary_inner(pdf, false)
}

pub(crate) fn raw_pdf_summary_with_stream_hashes(pdf: &[u8]) -> Result<RawPdfSummary, String> {
    raw_pdf_summary_inner(pdf, true)
}

fn raw_pdf_summary_inner(
    pdf: &[u8],
    hash_decoded_xobject_streams: bool,
) -> Result<RawPdfSummary, String> {
    let document = LopdfDocument::load_mem(pdf)
        .map_err(|error| format!("lopdf could not load PDF bytes: {error}"))?;

    let mut pages = Vec::new();
    for (page_number, page_id) in document.get_pages() {
        let page_index = page_number as usize - 1;
        pages.push(raw_page_summary(
            &document,
            page_index,
            page_id,
            hash_decoded_xobject_streams,
        )?);
    }
    pages.sort_by_key(|page| page.page_index);

    let outlines = raw_outline_titles(&document)?;

    Ok(RawPdfSummary { pages, outlines })
}

fn raw_page_summary(
    document: &LopdfDocument,
    page_index: usize,
    page_id: lopdf::ObjectId,
    hash_decoded_xobject_streams: bool,
) -> Result<RawPageSummary, String> {
    let page = document
        .get_dictionary(page_id)
        .map_err(|error| format!("lopdf could not read page dictionary {page_id:?}: {error}"))?;

    let mut annotation_refs = Vec::new();
    match page.get(b"Annots") {
        Ok(object) => {
            let annots = lopdf_array(document, object, "page Annots")?;
            annotation_refs.extend_from_slice(annots);
        }
        Err(lopdf::Error::DictKey(_)) => {}
        Err(error) => return Err(format!("lopdf could not read page Annots: {error}")),
    }

    let mut annotations = Vec::new();
    for annotation in &annotation_refs {
        let dictionary = lopdf_dictionary(document, annotation, "annotation dictionary")?;
        let parent = dictionary.get(b"Parent").ok().and_then(|parent| {
            lopdf_dictionary(document, parent, "annotation parent dictionary").ok()
        });
        let action_uri = dictionary
            .get(b"A")
            .ok()
            .and_then(|action| lopdf_dictionary(document, action, "annotation action").ok())
            .and_then(|action| action.get(b"URI").ok())
            .and_then(|uri| lopdf_text(document, uri).ok());
        let field_type_name = dictionary
            .get(b"FT")
            .ok()
            .and_then(|value| lopdf_name(document, value).ok())
            .or_else(|| {
                parent
                    .and_then(|parent| parent.get(b"FT").ok())
                    .and_then(|value| lopdf_name(document, value).ok())
            });
        let field_value = dictionary
            .get(b"V")
            .ok()
            .and_then(|value| lopdf_text(document, value).ok())
            .or_else(|| {
                parent
                    .and_then(|parent| parent.get(b"V").ok())
                    .and_then(|value| lopdf_text(document, value).ok())
            });

        annotations.push(RawAnnotationSummary {
            page_index,
            type_name: dictionary
                .get(b"Type")
                .ok()
                .and_then(|value| lopdf_name(document, value).ok()),
            subtype_name: dictionary
                .get(b"Subtype")
                .ok()
                .and_then(|value| lopdf_name(document, value).ok()),
            rect: dictionary
                .get(b"Rect")
                .ok()
                .and_then(|value| lopdf_rect(document, value).ok()),
            action_uri,
            field_type_name,
            field_value,
        });
    }

    let xobjects = raw_page_xobjects(document, page_index, page, hash_decoded_xobject_streams)?;
    let image_draw_names = raw_page_image_draw_names(document, page_id, &xobjects);

    Ok(RawPageSummary {
        page_index,
        annotation_count: annotation_refs.len(),
        annotations,
        xobjects,
        image_draw_names,
    })
}

fn raw_page_image_draw_names(
    document: &LopdfDocument,
    page_id: lopdf::ObjectId,
    xobjects: &[RawXObjectSummary],
) -> Vec<String> {
    // PDFium enumerates top-level page objects in content order. Preserve the
    // corresponding direct Image `Do` resource names so a repeated 24x24 icon
    // can be associated with its own SMask instead of with every same-sized
    // image on the page. Form XObjects are deliberately not flattened here:
    // PDFium exposes the form itself as the top-level object, so flattening one
    // side only would create a false ordinal association.
    let Ok(content) = document.get_and_decode_page_content(page_id) else {
        return Vec::new();
    };
    content
        .operations
        .iter()
        .filter(|operation| operation.operator == "Do")
        .filter_map(|operation| operation.operands.first())
        .filter_map(|operand| operand.as_name().ok())
        .map(|name| String::from_utf8_lossy(name).to_string())
        .filter(|name| {
            xobjects.iter().any(|xobject| {
                xobject.name == *name && xobject.subtype_name.as_deref() == Some("Image")
            })
        })
        .collect()
}

fn raw_outline_titles(document: &LopdfDocument) -> Result<Vec<String>, String> {
    let trailer_root = document
        .trailer
        .get(b"Root")
        .map_err(|error| format!("lopdf could not read trailer Root: {error}"))?;
    let catalog = lopdf_dictionary(document, trailer_root, "catalog dictionary")?;
    let Ok(outlines_object) = catalog.get(b"Outlines") else {
        return Ok(Vec::new());
    };
    let outlines = lopdf_dictionary(document, outlines_object, "Outlines dictionary")?;
    let Ok(first) = outlines.get(b"First") else {
        return Ok(Vec::new());
    };

    let mut titles = Vec::new();
    collect_outline_siblings(document, first, 0, &mut titles)?;
    Ok(titles)
}

fn collect_outline_siblings(
    document: &LopdfDocument,
    first: &LopdfObject,
    level: usize,
    titles: &mut Vec<String>,
) -> Result<(), String> {
    let mut current = Some(first.clone());
    while let Some(object) = current {
        let item = lopdf_dictionary(document, &object, "outline item dictionary")?;
        if let Ok(title) = item
            .get(b"Title")
            .map_err(|error| error.to_string())
            .and_then(|value| lopdf_text(document, value))
        {
            // Source: LibreOffice vcl/source/pdf/PDFiumLibrary.cxx
            // lcl_getBookmarks() prefixes one space per bookmark level when exposing
            // PDFium bookmark titles to tests.
            titles.push(format!("{}{}", " ".repeat(level), title));
        }
        if let Ok(child) = item.get(b"First") {
            collect_outline_siblings(document, child, level + 1, titles)?;
        }
        current = item.get(b"Next").ok().cloned();
    }
    Ok(())
}

fn raw_page_xobjects(
    document: &LopdfDocument,
    page_index: usize,
    page: &lopdf::Dictionary,
    hash_decoded_streams: bool,
) -> Result<Vec<RawXObjectSummary>, String> {
    let resources = match inherited_page_dictionary_value(document, page, b"Resources") {
        Ok(Some(object)) => lopdf_dictionary_owned(document, object, "page Resources")?,
        Ok(None) => return Ok(Vec::new()),
        Err(lopdf::Error::DictKey(_)) => return Ok(Vec::new()),
        Err(error) => return Err(format!("lopdf could not read page Resources: {error}")),
    };
    let xobjects = match resources.get(b"XObject") {
        Ok(object) => lopdf_dictionary(document, object, "page Resources/XObject")?,
        Err(lopdf::Error::DictKey(_)) => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "lopdf could not read page XObject resources: {error}"
            ));
        }
    };

    let mut summaries = Vec::new();
    collect_terminal_xobjects(
        document,
        page_index,
        None,
        xobjects,
        hash_decoded_streams,
        &mut summaries,
    )?;
    summaries.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(summaries)
}

fn collect_terminal_xobjects(
    document: &LopdfDocument,
    page_index: usize,
    prefix: Option<&str>,
    xobjects: &lopdf::Dictionary,
    hash_decoded_streams: bool,
    summaries: &mut Vec<RawXObjectSummary>,
) -> Result<(), String> {
    for (name, object) in xobjects.iter() {
        let current_name = if let Some(prefix) = prefix {
            format!("{prefix}/{}", String::from_utf8_lossy(name))
        } else {
            String::from_utf8_lossy(name).to_string()
        };
        let stream = lopdf_stream(document, object, "page XObject stream")?;
        let subtype_name = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|value| lopdf_name(document, value).ok());
        if subtype_name.as_deref() == Some("Form")
            && let Ok(resources_object) = stream.dict.get(b"Resources")
            && let Ok(resources) = lopdf_dictionary(document, resources_object, "form Resources")
            && let Ok(nested_object) = resources.get(b"XObject")
            && let Ok(nested_xobjects) = lopdf_dictionary(document, nested_object, "form XObject")
        {
            collect_terminal_xobjects(
                document,
                page_index,
                Some(&current_name),
                nested_xobjects,
                hash_decoded_streams,
                summaries,
            )?;
            continue;
        }

        summaries.push(raw_xobject_summary(
            document,
            page_index,
            &current_name,
            object,
            hash_decoded_streams,
        )?);
    }

    Ok(())
}

fn inherited_page_dictionary_value(
    document: &LopdfDocument,
    page: &lopdf::Dictionary,
    key: &[u8],
) -> Result<Option<LopdfObject>, lopdf::Error> {
    if let Ok(value) = page.get(key) {
        return Ok(Some(value.clone()));
    }

    let mut current_parent = match page.get(b"Parent") {
        Ok(parent) => Some(parent.clone()),
        Err(lopdf::Error::DictKey(_)) => None,
        Err(error) => return Err(error),
    };

    while let Some(parent) = current_parent {
        let parent_dict = lopdf_dictionary(document, &parent, "page parent dictionary")
            .map_err(|_| lopdf::Error::DictKey(String::from("Parent")))?;
        if let Ok(value) = parent_dict.get(key) {
            return Ok(Some(value.clone()));
        }
        current_parent = match parent_dict.get(b"Parent") {
            Ok(next) => Some(next.clone()),
            Err(lopdf::Error::DictKey(_)) => None,
            Err(error) => return Err(error),
        };
    }

    Ok(None)
}

fn raw_xobject_summary(
    document: &LopdfDocument,
    page_index: usize,
    name: &str,
    object: &LopdfObject,
    hash_decoded_stream: bool,
) -> Result<RawXObjectSummary, String> {
    let stream = lopdf_stream(document, object, "page XObject stream")?;
    let type_name = stream
        .dict
        .get(b"Type")
        .ok()
        .and_then(|value| lopdf_name(document, value).ok());
    let subtype_name = stream
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|value| lopdf_name(document, value).ok());
    let filter_names = stream
        .filters()
        .map(|filters| {
            filters
                .into_iter()
                .map(|filter| String::from_utf8_lossy(filter).to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let width_px = stream
        .dict
        .get(b"Width")
        .ok()
        .and_then(|value| lopdf_u32(document, value).ok());
    let height_px = stream
        .dict
        .get(b"Height")
        .ok()
        .and_then(|value| lopdf_u32(document, value).ok());
    let image_format = image::guess_format(&stream.content)
        .ok()
        .map(|format| format!("{format:?}"));
    let decoded = image::load_from_memory(&stream.content).ok();
    let (decoded_width_px, decoded_height_px, bits_per_pixel) = if let Some(image) = decoded {
        let (width, height) = image.dimensions();
        (
            Some(width),
            Some(height),
            Some(image.color().bits_per_pixel()),
        )
    } else {
        (None, None, None)
    };
    let has_soft_mask = stream.dict.has(b"SMask");
    let soft_mask_has_opaque_core = soft_mask_has_opaque_core(document, stream);
    let solid_rgb = solid_rgb_image(stream, subtype_name.as_deref(), width_px, height_px);
    let semantic_soft_mask_image = semantic_soft_mask_image(
        document,
        stream,
        subtype_name.as_deref(),
        width_px,
        height_px,
    );
    let decoded_stream_sha256 = hash_decoded_stream
        .then(|| stream.decompressed_content().ok())
        .flatten()
        .map(|content| format!("{:x}", Sha256::digest(content)));

    Ok(RawXObjectSummary {
        page_index,
        name: name.to_string(),
        type_name,
        subtype_name,
        filter_names,
        width_px,
        height_px,
        image_format,
        decoded_width_px,
        decoded_height_px,
        bits_per_pixel,
        decoded_stream_sha256,
        has_soft_mask,
        soft_mask_has_opaque_core,
        solid_rgb,
        semantic_soft_mask_image,
    })
}

fn semantic_soft_mask_image(
    document: &LopdfDocument,
    image: &lopdf::Stream,
    subtype_name: Option<&str>,
    width_px: Option<u32>,
    height_px: Option<u32>,
) -> Option<SemanticSoftMaskImage> {
    // PDF Reference 1.5 section 7.5.4 defines /Matte as the color with
    // which the parent image samples were preblended. Normalize the two
    // representations emitted by Office and Krilla to black-matte RGB plus
    // exact alpha. Keep this bounded and deliberately narrower than a general
    // PDF image decoder.
    const MAX_DIAGNOSTIC_DECODED_BYTES: usize = 256 * 1024;
    let components = match image
        .dict
        .get(b"ColorSpace")
        .ok()
        .and_then(|value| value.as_name().ok())
    {
        Some(b"DeviceGray") => 1,
        Some(b"DeviceRGB") => 3,
        _ => return None,
    };
    if subtype_name != Some("Image")
        || image
            .dict
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|value| value.as_i64().ok())
            != Some(8)
        || image.dict.has(b"Decode")
    {
        return None;
    }

    let width = usize::try_from(width_px?).ok()?;
    let height = usize::try_from(height_px?).ok()?;
    let pixel_count = width.checked_mul(height)?;
    let rgba_bytes = pixel_count.checked_mul(4)?;
    if pixel_count == 0 || rgba_bytes > MAX_DIAGNOSTIC_DECODED_BYTES {
        return None;
    }

    let mask = image.dict.get(b"SMask").ok()?;
    let mask = lopdf_stream(document, mask, "image soft-mask stream").ok()?;
    if mask
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|value| value.as_name().ok())
        != Some(b"Image")
        || mask
            .dict
            .get(b"ColorSpace")
            .ok()
            .and_then(|value| value.as_name().ok())
            != Some(b"DeviceGray")
        || mask
            .dict
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|value| value.as_i64().ok())
            != Some(8)
        || mask.dict.has(b"Decode")
        || mask.dict.get(b"Width").ok()?.as_i64().ok()? != i64::try_from(width).ok()?
        || mask.dict.get(b"Height").ok()?.as_i64().ok()? != i64::try_from(height).ok()?
    {
        return None;
    }

    let matte_is_black = match mask.dict.get(b"Matte") {
        Err(lopdf::Error::DictKey(_)) => false,
        Ok(value) => {
            let values = value.as_array().ok()?;
            values.len() == 3 && values.iter().all(lopdf_number_is_zero)
        }
        Err(_) => return None,
    };
    let has_matte = mask.dict.has(b"Matte");
    if has_matte && !matte_is_black {
        return None;
    }

    let alpha = mask.decompressed_content().ok()?;
    let samples = image.decompressed_content().ok()?;
    if alpha.len() != pixel_count || samples.len() != pixel_count.checked_mul(components)? {
        return None;
    }
    // A black-preblended component cannot exceed alpha (apart from one 8-bit
    // quantization value). Office's fixed-output writer also emits /Matte
    // [0 0 0] for some straight-alpha icon streams whose transparent samples
    // remain 255. Detect that representation from the PDF 1.5 preblending
    // invariant instead of trusting the marker and multiplying it exactly
    // once, just as for a stream without /Matte.
    let is_valid_black_matte = matte_is_black
        && samples
            .chunks_exact(components)
            .zip(&alpha)
            .all(|(pixel, alpha)| {
                pixel
                    .iter()
                    .all(|component| *component <= alpha.saturating_add(1))
            });
    let mut black_matte_rgb = Vec::with_capacity(pixel_count.checked_mul(3)?);
    for (pixel, alpha) in samples.chunks_exact(components).zip(&alpha) {
        let normalize = |component: u8| {
            if is_valid_black_matte {
                component
            } else {
                ((u16::from(component) * u16::from(*alpha) + 127) / u16::from(u8::MAX)) as u8
            }
        };
        if components == 1 {
            black_matte_rgb.extend([normalize(pixel[0]); 3]);
        } else {
            black_matte_rgb.extend(pixel.iter().copied().map(normalize));
        }
    }
    Some(SemanticSoftMaskImage {
        alpha,
        black_matte_rgb,
    })
}

fn lopdf_number_is_zero(value: &LopdfObject) -> bool {
    match value {
        LopdfObject::Integer(value) => *value == 0,
        LopdfObject::Real(value) => *value == 0.0,
        _ => false,
    }
}

fn soft_mask_has_opaque_core(
    document: &LopdfDocument,
    image_stream: &lopdf::Stream,
) -> Option<bool> {
    let mask = image_stream.dict.get(b"SMask").ok()?;
    let mask = lopdf_stream(document, mask, "image soft-mask stream").ok()?;
    decoded_soft_mask_has_opaque_core(mask)
}

fn decoded_soft_mask_has_opaque_core(mask: &lopdf::Stream) -> Option<bool> {
    // A constant-color image plus a soft mask is not necessarily a stencil:
    // Office also emits low-opacity shadows this way. Only classify the
    // composed object as a stencil when the DeviceGray mask has a genuinely
    // opaque core. Unknown bit depths or non-default Decode arrays remain
    // ordinary pixel regions.
    const MAX_DIAGNOSTIC_DECODED_BYTES: usize = 256 * 1024;
    if mask
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|value| value.as_name().ok())
        != Some(b"Image")
        || mask
            .dict
            .get(b"ColorSpace")
            .ok()
            .and_then(|value| value.as_name().ok())
            != Some(b"DeviceGray")
        || mask
            .dict
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|value| value.as_i64().ok())
            != Some(8)
        || mask.dict.has(b"Decode")
    {
        return None;
    }
    let width = mask.dict.get(b"Width").ok()?.as_i64().ok()?;
    let height = mask.dict.get(b"Height").ok()?.as_i64().ok()?;
    let expected_len = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?;
    if expected_len == 0 || expected_len > MAX_DIAGNOSTIC_DECODED_BYTES {
        return None;
    }
    let decoded = mask.decompressed_content().ok()?;
    (decoded.len() == expected_len).then(|| decoded.contains(&u8::MAX))
}

fn solid_rgb_image(
    stream: &lopdf::Stream,
    subtype_name: Option<&str>,
    width_px: Option<u32>,
    height_px: Option<u32>,
) -> Option<[u8; 3]> {
    // Office represents some rasterized DrawingML text/effect output as a
    // constant RGB paint image plus an SMask carrying the actual silhouette.
    // Decode only small, simple RGB streams: this is diagnostic
    // classification, not a general PDF image decoder.
    const MAX_DIAGNOSTIC_DECODED_BYTES: usize = 256 * 1024;
    if subtype_name != Some("Image") || !stream.dict.has(b"SMask") {
        return None;
    }
    let color_space = stream
        .dict
        .get(b"ColorSpace")
        .ok()
        .and_then(|value| value.as_name().ok())?;
    if color_space != b"DeviceRGB" {
        return None;
    }
    let bits_per_component = stream
        .dict
        .get(b"BitsPerComponent")
        .ok()
        .and_then(|value| value.as_i64().ok())?;
    if bits_per_component != 8 {
        return None;
    }
    let expected_len = usize::try_from(width_px?)
        .ok()?
        .checked_mul(usize::try_from(height_px?).ok()?)?
        .checked_mul(3)?;
    if expected_len == 0 || expected_len > MAX_DIAGNOSTIC_DECODED_BYTES {
        return None;
    }
    let decoded = stream.decompressed_content().ok()?;
    if decoded.len() != expected_len {
        return None;
    }
    let first = decoded.first_chunk::<3>()?;
    decoded
        .chunks_exact(3)
        .all(|pixel| pixel == first)
        .then_some(*first)
}

fn lopdf_array<'a>(
    document: &'a LopdfDocument,
    object: &'a LopdfObject,
    context: &str,
) -> Result<&'a Vec<LopdfObject>, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference {context}: {error}"))?;
    object
        .as_array()
        .map_err(|error| format!("lopdf expected array for {context}: {error}"))
}

fn lopdf_dictionary<'a>(
    document: &'a LopdfDocument,
    object: &'a LopdfObject,
    context: &str,
) -> Result<&'a lopdf::Dictionary, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference {context}: {error}"))?;
    object
        .as_dict()
        .map_err(|error| format!("lopdf expected dictionary for {context}: {error}"))
}

fn lopdf_dictionary_owned(
    document: &LopdfDocument,
    object: LopdfObject,
    context: &str,
) -> Result<lopdf::Dictionary, String> {
    let (_, object) = document
        .dereference(&object)
        .map_err(|error| format!("lopdf could not dereference {context}: {error}"))?;
    object
        .as_dict()
        .cloned()
        .map_err(|error| format!("lopdf expected dictionary for {context}: {error}"))
}

fn lopdf_name(document: &LopdfDocument, object: &LopdfObject) -> Result<String, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference name: {error}"))?;
    let name = object
        .as_name()
        .map_err(|error| format!("lopdf expected name object: {error}"))?;
    Ok(String::from_utf8_lossy(name).to_string())
}

fn lopdf_text(document: &LopdfDocument, object: &LopdfObject) -> Result<String, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference string: {error}"))?;
    match object {
        LopdfObject::String(value, _) => Ok(normalize_extracted_text(&pdf_string_text(value))),
        LopdfObject::Name(value) => Ok(String::from_utf8_lossy(value).to_string()),
        _ => Err(format!(
            "lopdf expected string-like object, found {}",
            object.enum_variant()
        )),
    }
}

fn pdf_string_text(value: &[u8]) -> String {
    if let Some(text) = pdf_utf16_string(value, &[0xfe, 0xff], u16::from_be_bytes) {
        return text;
    }
    if let Some(text) = pdf_utf16_string(value, &[0xff, 0xfe], u16::from_le_bytes) {
        return text;
    }
    String::from_utf8_lossy(value).to_string()
}

fn pdf_utf16_string(value: &[u8], bom: &[u8; 2], convert: fn([u8; 2]) -> u16) -> Option<String> {
    let content = value.strip_prefix(bom)?;
    let units = content
        .chunks_exact(2)
        .map(|chunk| convert([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    Some(String::from_utf16_lossy(&units))
}

fn lopdf_rect(document: &LopdfDocument, object: &LopdfObject) -> Result<String, String> {
    let values = lopdf_array(document, object, "annotation Rect")?;
    if values.len() != 4 {
        return Err(format!(
            "lopdf expected four coordinates in annotation Rect, found {}",
            values.len()
        ));
    }

    let mut numbers = [0.0f32; 4];
    for (index, value) in values.iter().enumerate() {
        let (_, value) = document
            .dereference(value)
            .map_err(|error| format!("lopdf could not dereference Rect coordinate: {error}"))?;
        numbers[index] = value
            .as_float()
            .map_err(|error| format!("lopdf expected numeric Rect coordinate: {error}"))?;
    }

    Ok(format!(
        "[{:.3} {:.3} {:.3} {:.3}]",
        numbers[0], numbers[1], numbers[2], numbers[3]
    ))
}

fn lopdf_stream<'a>(
    document: &'a LopdfDocument,
    object: &'a LopdfObject,
    context: &str,
) -> Result<&'a lopdf::Stream, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference {context}: {error}"))?;
    match object {
        LopdfObject::Stream(stream) => Ok(stream),
        _ => Err(format!(
            "lopdf expected stream for {context}, found {}",
            object.enum_variant()
        )),
    }
}

fn lopdf_u32(document: &LopdfDocument, object: &LopdfObject) -> Result<u32, String> {
    let (_, object) = document
        .dereference(object)
        .map_err(|error| format!("lopdf could not dereference integer: {error}"))?;
    let value = object
        .as_i64()
        .map_err(|error| format!("lopdf expected integer object: {error}"))?;
    u32::try_from(value).map_err(|_| format!("lopdf integer is out of range for u32: {value}"))
}

fn bind_pdfium() -> Result<Pdfium, String> {
    if let Some(path) = std::env::var_os("PDFIUM_DYNAMIC_LIB_PATH") {
        match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&path)) {
            Ok(bindings) => return Ok(Pdfium::new(bindings)),
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => {
                return Ok(Pdfium::default());
            }
            Err(error) => {
                return Err(format!(
                    "could not bind PDFium from PDFIUM_DYNAMIC_LIB_PATH={}: {error}",
                    std::path::PathBuf::from(path).display()
                ));
            }
        }
    }

    match Pdfium::bind_to_system_library() {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(Pdfium::default()),
        Err(error) => Err(format!(
            "could not bind system PDFium library; install libpdfium.so or set PDFIUM_DYNAMIC_LIB_PATH: {error}"
        )),
    }
}

fn pdfium_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn pdfium_link_target(link: &PdfLink<'_>) -> (LinkTargetKind, Option<String>) {
    if let Some(action) = link.action() {
        if let Some(uri) = action.as_uri_action() {
            return (
                LinkTargetKind::ExternalUri,
                uri.uri().ok().map(|value| normalize_extracted_text(&value)),
            );
        }
        if let Some(local) = action.as_local_destination_action() {
            return (
                LinkTargetKind::InternalDestination,
                local
                    .destination()
                    .ok()
                    .and_then(|dest| format_destination(&dest)),
            );
        }
        return (
            LinkTargetKind::Action,
            Some(format!("{:?}", action.action_type())),
        );
    }

    if let Some(destination) = link.destination() {
        return (
            LinkTargetKind::InternalDestination,
            format_destination(&destination),
        );
    }

    (LinkTargetKind::Unknown, None)
}

fn format_destination(destination: &PdfDestination<'_>) -> Option<String> {
    let page = destination.page_index().ok()?;
    let view = destination
        .view_settings()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|_| "Unknown".to_string());
    Some(format!("page={page} view={view}"))
}

fn rendered_page_image(
    page_index: usize,
    page: &PdfPage<'_>,
    target_width: i32,
    calculate_crc32: bool,
) -> Result<RenderedPageImage, String> {
    let bitmap = page
        .render_with_config(
            &PdfRenderConfig::new()
                .set_target_width(target_width)
                .set_reverse_byte_order(true),
        )
        .map_err(|error| format!("PDFium could not render page {page_index}: {error}"))?;
    let width_px = bitmap.width() as u32;
    let height_px = bitmap.height() as u32;
    let rgba = bitmap.as_rgba_bytes();
    let rgba_crc32 = if calculate_crc32 {
        let mut crc = crc32fast::Hasher::new();
        crc.update(&rgba);
        format!("{:08x}", crc.finalize())
    } else {
        String::new()
    };
    Ok(RenderedPageImage {
        page_index,
        width_px,
        height_px,
        page_width_pt: page.width().value,
        page_height_pt: page.height().value,
        rgba_crc32,
        rgba,
    })
}

fn format_color(color: PdfColor) -> String {
    format!(
        "#{:02x}{:02x}{:02x}@{:02x}",
        color.red(),
        color.green(),
        color.blue(),
        color.alpha()
    )
}

fn format_points(points: PdfPoints, precision: usize) -> String {
    format!("{value:.precision$}", value = points.value)
}

fn format_text_render_mode(mode: PdfPageTextRenderMode) -> String {
    format!("{mode:?}")
}

fn format_rect(rect: pdfium_render::prelude::PdfRect, precision: usize) -> String {
    format_rect_with_precision(rect, precision)
}

fn format_rect_with_precision(rect: pdfium_render::prelude::PdfRect, precision: usize) -> String {
    format!(
        "[{left:.precision$} {bottom:.precision$} {right:.precision$} {top:.precision$}]",
        left = rect.left().value,
        bottom = rect.bottom().value,
        right = rect.right().value,
        top = rect.top().value,
    )
}

struct PdfStream {
    dictionary: String,
    decoded: Option<Vec<u8>>,
}

fn pdf_streams(pdf: &[u8]) -> Vec<PdfStream> {
    let mut streams = Vec::new();
    let mut search_start = 0usize;
    while let Some(stream_offset) = find_bytes(&pdf[search_start..], b"stream") {
        let stream_keyword = search_start + stream_offset;
        let dict_start = rfind_bytes(&pdf[..stream_keyword], b"<<")
            .unwrap_or(stream_keyword.saturating_sub(512));
        let dictionary = String::from_utf8_lossy(&pdf[dict_start..stream_keyword]).to_string();
        let mut data_start = stream_keyword + "stream".len();
        if pdf.get(data_start) == Some(&b'\r') {
            data_start += 1;
        }
        if pdf.get(data_start) == Some(&b'\n') {
            data_start += 1;
        }
        let Some(end_offset) = find_bytes(&pdf[data_start..], b"endstream") else {
            break;
        };
        let mut data_end = data_start + end_offset;
        while data_end > data_start && matches!(pdf[data_end - 1], b'\r' | b'\n') {
            data_end -= 1;
        }
        let data = &pdf[data_start..data_end];
        let decoded = if dictionary.contains("/FlateDecode") {
            let mut decoder = ZlibDecoder::new(data);
            let mut output = Vec::new();
            decoder.read_to_end(&mut output).ok().map(|_| output)
        } else if dictionary.contains("/Filter") {
            None
        } else {
            Some(data.to_vec())
        };
        streams.push(PdfStream {
            dictionary,
            decoded,
        });
        search_start = data_end + "endstream".len();
    }
    streams
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn content_summary(streams: &[PdfStream]) -> ContentSummary {
    let mut summary = ContentSummary {
        stream_count: streams.len(),
        ..ContentSummary::default()
    };
    for stream in streams {
        if !stream.dictionary.contains("/Length") {
            continue;
        }
        let Some(decoded) = &stream.decoded else {
            continue;
        };
        summary.decoded_stream_count += 1;
        let content = String::from_utf8_lossy(decoded);
        summary.text_show_ops += operator_count(&content, &["Tj", "TJ", "'", "\""]);
        summary.image_draw_ops += operator_count(&content, &["Do"]);
        summary.path_paint_ops += operator_count(&content, &["S", "s", "f", "F", "f*", "B", "B*"]);
        summary.clipping_ops += operator_count(&content, &["W", "W*"]);
    }
    summary
}

fn operator_count(content: &str, operators: &[&str]) -> usize {
    content
        .split_ascii_whitespace()
        .filter(|token| operators.contains(token))
        .count()
}

pub(crate) fn pdftotext_page(pdf: &[u8], page_index: usize) -> std::result::Result<String, String> {
    let page_number = page_index
        .checked_add(1)
        .ok_or_else(|| format!("PDF page index {page_index} cannot be represented by pdftotext"))?
        .to_string();
    run_pdftotext(
        pdf,
        [
            "-f",
            page_number.as_str(),
            "-l",
            page_number.as_str(),
            "-layout",
            "-nopgbrk",
        ],
    )
}

pub(crate) fn pdftotext_page_raw(
    pdf: &[u8],
    page_index: usize,
) -> std::result::Result<String, String> {
    let page_number = page_index
        .checked_add(1)
        .ok_or_else(|| format!("PDF page index {page_index} cannot be represented by pdftotext"))?
        .to_string();
    run_pdftotext(
        pdf,
        [
            "-f",
            page_number.as_str(),
            "-l",
            page_number.as_str(),
            "-raw",
            "-nopgbrk",
        ],
    )
}

fn pdftotext(pdf: &[u8]) -> std::result::Result<String, String> {
    run_pdftotext(pdf, ["-layout", "-nopgbrk"])
}

fn run_pdftotext<'a>(
    pdf: &[u8],
    args: impl IntoIterator<Item = &'a str>,
) -> std::result::Result<String, String> {
    let path = temp_pdf_path();
    std::fs::write(&path, pdf).map_err(|error| error.to_string())?;
    let output = Command::new("pdftotext")
        .args(args)
        .arg(&path)
        .arg("-")
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.map_err(|error| format!("pdftotext failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "pdftotext failed: status={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(normalize_extracted_text(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn temp_pdf_path() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ooxmlsdk-pdf-extract-{}-{nanos}.pdf",
        std::process::id()
    ))
}

fn normalize_extracted_text(text: &str) -> String {
    text.replace('\t', " ")
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn normalize_pdf_font_name(name: &str) -> String {
    let Some((prefix, rest)) = name.split_once('+') else {
        return name.to_string();
    };
    if prefix.len() == 6 && prefix.bytes().all(|byte| byte.is_ascii_uppercase()) {
        rest.to_string()
    } else {
        name.to_string()
    }
}

trait PdfBytesExt {
    fn strip_suffix_ascii_whitespace(&self) -> &[u8];
}

impl PdfBytesExt for [u8] {
    fn strip_suffix_ascii_whitespace(&self) -> &[u8] {
        let mut end = self.len();
        while end > 0 && self[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        &self[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdfium_utf16_surrogate_pair_decodes_to_one_unicode_scalar() {
        assert_eq!(
            decode_pdfium_unicode_scalar(0xd83e, Some(0xdc78)),
            (Some('\u{1f878}'), true)
        );
        assert_eq!(
            decode_pdfium_unicode_scalar(0xd83e, Some('A' as u32)),
            (None, false)
        );
        assert_eq!(decode_pdfium_unicode_scalar(0xdc78, None), (None, false));
        assert_eq!(
            decode_pdfium_unicode_scalar('\u{1f878}' as u32, None),
            (Some('\u{1f878}'), false)
        );
    }

    #[test]
    fn lightweight_page_text_uses_characters_before_overlapping_segments() {
        let text = normalized_pdfium_page_text_from_parts(
            [Some("\u{f045}"), Some("\u{f04a}"), Some("\u{f049}")],
            ["\u{f045}\u{f04a}", "\u{f045}\u{f04a}", "\u{f049}"],
        );

        assert_eq!(text, "\u{f045}\u{f04a}\u{f049}");
    }

    #[test]
    fn lightweight_page_text_falls_back_only_without_character_data() {
        let text = normalized_pdfium_page_text_from_parts(
            std::iter::empty::<Option<&str>>(),
            ["Hello", "world"],
        );
        assert_eq!(text, "Hello world");

        let empty_character_text =
            normalized_pdfium_page_text_from_parts([Some("")], ["segment fallback"]);
        assert_eq!(empty_character_text, "");
    }

    #[test]
    fn text_writing_angle_uses_the_advance_axis_not_the_shear_axis() {
        // Office math synthetic italic: c=0.3333 is intentionally absent
        // because it shears the glyph without rotating the writing axis.
        assert_eq!(text_matrix_writing_angle_degrees(1.0, 0.0), 0.0);
        assert_eq!(text_matrix_writing_angle_degrees(0.0, 1.0), 90.0);
        assert_eq!(text_matrix_writing_angle_degrees(-1.0, 0.0), 180.0);
    }

    #[test]
    fn parse_pdf_rect_reads_four_point_coordinates() {
        let rect = parse_pdf_rect("[1.5 2.0 30.25 40.75]").unwrap();
        assert_eq!(
            rect,
            PdfBounds {
                left: 1.5,
                bottom: 2.0,
                right: 30.25,
                top: 40.75,
            }
        );
        assert_eq!(rect.width(), 28.75);
        assert_eq!(rect.height(), 38.75);
        assert_eq!(rect.center(), (15.875, 21.375));
    }

    #[test]
    fn font_metric_bounds_use_the_font_baseline_and_vertical_metrics() {
        let bounds = PdfRect::new(
            PdfPoints::new(10.0),
            PdfPoints::new(20.0),
            PdfPoints::new(30.0),
            PdfPoints::new(40.0),
        );
        let reconstructed = font_metric_bounds(
            bounds,
            100.0,
            20.0,
            EmbeddedFontVerticalMetrics {
                ascent_em: 0.9,
                descent_em: 0.2,
            },
        );
        assert_eq!(reconstructed.as_deref(), Some("[20.00 96.00 40.00 118.00]"));
    }

    #[test]
    #[should_panic(expected = "PDF rect right mismatch")]
    fn assert_pdf_rect_close_reports_coordinate_mismatch() {
        assert_pdf_rect_close(
            "[0.0 0.0 12.0 10.0]",
            PdfBounds {
                left: 0.0,
                bottom: 0.0,
                right: 10.0,
                top: 10.0,
            },
            0.1,
        );
    }

    #[test]
    fn rendered_page_image_maps_pdf_coordinates_to_pixels() {
        let image = RenderedPageImage {
            page_index: 0,
            width_px: 200,
            height_px: 100,
            page_width_pt: 400.0,
            page_height_pt: 200.0,
            rgba_crc32: String::new(),
            rgba: vec![0; 200 * 100 * 4],
        };

        assert_eq!(image.pdf_point_to_pixel(0.0, 200.0), Some((0, 0)));
        assert_eq!(image.pdf_point_to_pixel(400.0, 0.0), Some((199, 99)));
        assert_eq!(image.pdf_point_to_pixel(200.0, 100.0), Some((100, 50)));
        assert_eq!(
            image.pdf_rect_to_pixel_rect(PdfBounds {
                left: 100.0,
                bottom: 50.0,
                right: 300.0,
                top: 150.0,
            }),
            Some(PixelRect {
                left: 50,
                top: 25,
                width: 100,
                height: 50,
            })
        );
    }

    #[test]
    fn rendered_page_image_samples_pixels_and_hashes_regions() {
        let mut rgba = vec![0; 4 * 4 * 4];
        rgba[(2 * 4 + 1) * 4..(2 * 4 + 2) * 4].copy_from_slice(&[10, 20, 30, 255]);
        let image = RenderedPageImage {
            page_index: 0,
            width_px: 4,
            height_px: 4,
            page_width_pt: 4.0,
            page_height_pt: 4.0,
            rgba_crc32: String::new(),
            rgba,
        };

        assert_eq!(image.pixel_rgba(1, 2), Some([10, 20, 30, 255]));
        assert_eq!(
            image.sample_pdf_point_rgba(1.0, 2.0),
            Some([10, 20, 30, 255])
        );
        assert!(
            image
                .pixel_region_crc32(PixelRect {
                    left: 1,
                    top: 2,
                    width: 1,
                    height: 1,
                })
                .is_some()
        );
    }

    #[test]
    fn solid_rgb_soft_mask_images_are_classified_from_pdf_structure_and_content() {
        let mut dictionary = lopdf::Dictionary::new();
        dictionary.set("Subtype", "Image");
        dictionary.set("ColorSpace", "DeviceRGB");
        dictionary.set("BitsPerComponent", 8);
        dictionary.set("SMask", LopdfObject::Reference((7, 0)));
        let solid = lopdf::Stream::new(dictionary.clone(), [12, 34, 56].repeat(4));
        assert_eq!(
            solid_rgb_image(&solid, Some("Image"), Some(2), Some(2)),
            Some([12, 34, 56])
        );

        let mut nonuniform_content = [12, 34, 56].repeat(4);
        nonuniform_content[3] = 13;
        let nonuniform = lopdf::Stream::new(dictionary, nonuniform_content);
        assert_eq!(
            solid_rgb_image(&nonuniform, Some("Image"), Some(2), Some(2)),
            None
        );
    }

    #[test]
    fn soft_mask_stencil_requires_a_fully_opaque_core() {
        let mut dictionary = lopdf::Dictionary::new();
        dictionary.set("Subtype", "Image");
        dictionary.set("ColorSpace", "DeviceGray");
        dictionary.set("BitsPerComponent", 8);
        dictionary.set("Width", 2);
        dictionary.set("Height", 2);

        let translucent = lopdf::Stream::new(dictionary.clone(), vec![0, 64, 127, 254]);
        assert_eq!(decoded_soft_mask_has_opaque_core(&translucent), Some(false));

        let stencil = lopdf::Stream::new(dictionary, vec![0, 64, 127, 255]);
        assert_eq!(decoded_soft_mask_has_opaque_core(&stencil), Some(true));
    }

    #[test]
    fn soft_mask_images_normalize_pdf_matte_preblending() {
        let alpha = vec![124];
        let mut document = LopdfDocument::new();

        let mut straight_mask_dictionary = lopdf::Dictionary::new();
        straight_mask_dictionary.set("Subtype", "Image");
        straight_mask_dictionary.set("ColorSpace", "DeviceGray");
        straight_mask_dictionary.set("BitsPerComponent", 8);
        straight_mask_dictionary.set("Width", 1);
        straight_mask_dictionary.set("Height", 1);
        let straight_mask_id =
            document.add_object(lopdf::Stream::new(straight_mask_dictionary, alpha.clone()));
        let mut straight_dictionary = lopdf::Dictionary::new();
        straight_dictionary.set("Subtype", "Image");
        straight_dictionary.set("ColorSpace", "DeviceRGB");
        straight_dictionary.set("BitsPerComponent", 8);
        straight_dictionary.set("SMask", straight_mask_id);
        let straight = lopdf::Stream::new(straight_dictionary, vec![180, 133, 136]);

        let mut matte_mask_dictionary = lopdf::Dictionary::new();
        matte_mask_dictionary.set("Subtype", "Image");
        matte_mask_dictionary.set("ColorSpace", "DeviceGray");
        matte_mask_dictionary.set("BitsPerComponent", 8);
        matte_mask_dictionary.set("Width", 1);
        matte_mask_dictionary.set("Height", 1);
        matte_mask_dictionary.set(
            "Matte",
            vec![
                LopdfObject::Integer(0),
                LopdfObject::Integer(0),
                LopdfObject::Integer(0),
            ],
        );
        let matte_mask_id =
            document.add_object(lopdf::Stream::new(matte_mask_dictionary, alpha.clone()));
        let mut matte_dictionary = lopdf::Dictionary::new();
        matte_dictionary.set("Subtype", "Image");
        matte_dictionary.set("ColorSpace", "DeviceRGB");
        matte_dictionary.set("BitsPerComponent", 8);
        matte_dictionary.set("SMask", matte_mask_id);
        let matte = lopdf::Stream::new(matte_dictionary, vec![87, 64, 66]);

        let straight =
            semantic_soft_mask_image(&document, &straight, Some("Image"), Some(1), Some(1))
                .unwrap();
        let matte =
            semantic_soft_mask_image(&document, &matte, Some("Image"), Some(1), Some(1)).unwrap();

        let mut gray_dictionary = lopdf::Dictionary::new();
        gray_dictionary.set("Subtype", "Image");
        gray_dictionary.set("ColorSpace", "DeviceGray");
        gray_dictionary.set("BitsPerComponent", 8);
        gray_dictionary.set("SMask", straight_mask_id);
        let gray = lopdf::Stream::new(gray_dictionary, vec![180]);
        let gray =
            semantic_soft_mask_image(&document, &gray, Some("Image"), Some(1), Some(1)).unwrap();

        let mut straight_with_matte_dictionary = lopdf::Dictionary::new();
        straight_with_matte_dictionary.set("Subtype", "Image");
        straight_with_matte_dictionary.set("ColorSpace", "DeviceGray");
        straight_with_matte_dictionary.set("BitsPerComponent", 8);
        straight_with_matte_dictionary.set("SMask", matte_mask_id);
        let straight_with_matte = lopdf::Stream::new(straight_with_matte_dictionary, vec![180]);
        let straight_with_matte = semantic_soft_mask_image(
            &document,
            &straight_with_matte,
            Some("Image"),
            Some(1),
            Some(1),
        )
        .unwrap();

        assert_eq!(straight.alpha, alpha);
        assert_eq!(matte.alpha, alpha);
        assert_eq!(gray.alpha, alpha);
        assert_eq!(straight_with_matte.alpha, alpha);
        assert_eq!(straight.black_matte_rgb, vec![88, 65, 66]);
        assert_eq!(matte.black_matte_rgb, vec![87, 64, 66]);
        assert_eq!(gray.black_matte_rgb, vec![88, 88, 88]);
        assert_eq!(straight_with_matte.black_matte_rgb, vec![88, 88, 88]);
    }
}

use std::collections::BTreeMap;

use lopdf::{Document as LopdfDocument, Object as LopdfObject};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::pdf_extract::{
    PdfFontResourceSummary, RawAnnotationSummary, RawXObjectSummary, pdf_font_structure,
    pdf_page_dimensions, raw_pdf_summary_with_stream_hashes,
};

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct PdfFragmentSnapshot {
    schema_version: u32,
    pdf_version: String,
    pages: Vec<PdfPageFragmentSnapshot>,
    outlines: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PdfPageFragmentSnapshot {
    page_index: usize,
    width_pt: String,
    height_pt: String,
    content_operations: Vec<String>,
    fonts: Vec<PdfFontFragmentSnapshot>,
    xobjects: Vec<PdfXObjectFragmentSnapshot>,
    annotations: Vec<PdfAnnotationFragmentSnapshot>,
}

#[derive(Debug, Serialize)]
struct PdfFontFragmentSnapshot {
    resource_path: String,
    subtype: Option<String>,
    base_font: Option<String>,
    encoding: Option<String>,
    descendant_subtype: Option<String>,
    descendant_base_font: Option<String>,
    first_char: Option<i64>,
    last_char: Option<i64>,
    simple_width_count: Option<usize>,
    cid_width_entry_count: Option<usize>,
    descriptor_flags: Option<i64>,
    font_bounds: Option<String>,
    ascent: Option<String>,
    descent: Option<String>,
    cap_height: Option<String>,
    italic_angle: Option<String>,
    embedded_font_kind: Option<String>,
    has_to_unicode: bool,
    to_unicode_mapping_count: Option<usize>,
    to_unicode_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct PdfXObjectFragmentSnapshot {
    resource_path: String,
    type_name: Option<String>,
    subtype_name: Option<String>,
    filters: Vec<String>,
    width_px: Option<u32>,
    height_px: Option<u32>,
    decoded_width_px: Option<u32>,
    decoded_height_px: Option<u32>,
    bits_per_pixel: Option<u16>,
    decoded_stream_sha256: Option<String>,
    has_soft_mask: bool,
    soft_mask_has_opaque_core: Option<bool>,
    solid_rgb: Option<[u8; 3]>,
}

#[derive(Debug, Serialize)]
struct PdfAnnotationFragmentSnapshot {
    type_name: Option<String>,
    subtype_name: Option<String>,
    rect: Option<String>,
    action_uri: Option<String>,
    field_type_name: Option<String>,
    field_value: Option<String>,
}

/// Returns a deterministic, reviewable snapshot of the PDF lowering boundary.
///
/// The snapshot records page content operations and semantic resource facts,
/// while omitting object numbers, xref offsets, stream lengths, trailer IDs,
/// and compressed byte representation. It is intended for focused regression
/// tests, not for declaring two independently produced PDFs byte-identical.
pub fn canonical_pdf_fragment_snapshot(pdf: &[u8]) -> Result<String, String> {
    let document = LopdfDocument::load_mem(pdf)
        .map_err(|error| format!("lopdf could not load PDF fragment snapshot: {error}"))?;
    let dimensions = pdf_page_dimensions(pdf)?;
    let raw = raw_pdf_summary_with_stream_hashes(pdf)?;
    let font_structure = pdf_font_structure(pdf)?;
    let fonts_by_page = font_structure
        .pages
        .into_iter()
        .map(|page| (page.page_index, page.fonts))
        .collect::<BTreeMap<_, _>>();

    let pages = document
        .get_pages()
        .into_iter()
        .enumerate()
        .map(|(page_index, (_, page_id))| {
            let (width_pt, height_pt) = dimensions.get(page_index).copied().ok_or_else(|| {
                format!("PDF fragment snapshot has no dimensions for page {page_index}")
            })?;
            let content = document
                .get_and_decode_page_content(page_id)
                .map_err(|error| {
                    format!(
                        "lopdf could not decode PDF fragment page {page_index} content: {error}"
                    )
                })?;
            let raw_page = raw.pages.get(page_index).ok_or_else(|| {
                format!("PDF fragment snapshot has no raw summary for page {page_index}")
            })?;
            Ok(PdfPageFragmentSnapshot {
                page_index,
                width_pt: canonical_number(width_pt),
                height_pt: canonical_number(height_pt),
                content_operations: content.operations.iter().map(canonical_operation).collect(),
                fonts: fonts_by_page
                    .get(&page_index)
                    .into_iter()
                    .flat_map(|fonts| fonts.iter())
                    .map(PdfFontFragmentSnapshot::from)
                    .collect(),
                xobjects: raw_page
                    .xobjects
                    .iter()
                    .map(PdfXObjectFragmentSnapshot::from)
                    .collect(),
                annotations: raw_page
                    .annotations
                    .iter()
                    .map(PdfAnnotationFragmentSnapshot::from)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    serde_json::to_string_pretty(&PdfFragmentSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        pdf_version: document.version,
        pages,
        outlines: raw.outlines,
    })
    .map(|snapshot| format!("{snapshot}\n"))
    .map_err(|error| format!("could not serialize PDF fragment snapshot: {error}"))
}

impl From<&PdfFontResourceSummary> for PdfFontFragmentSnapshot {
    fn from(font: &PdfFontResourceSummary) -> Self {
        Self {
            resource_path: font.resource_path.clone(),
            subtype: font.subtype.clone(),
            base_font: font.base_font.as_deref().map(canonical_font_name),
            encoding: font.encoding.clone(),
            descendant_subtype: font.descendant_subtype.clone(),
            descendant_base_font: font
                .descendant_base_font
                .as_deref()
                .map(canonical_font_name),
            first_char: font.first_char,
            last_char: font.last_char,
            simple_width_count: font.simple_width_count,
            cid_width_entry_count: font.cid_width_entry_count,
            descriptor_flags: font.descriptor_flags,
            font_bounds: font.font_bounds.clone(),
            ascent: font.ascent.clone(),
            descent: font.descent.clone(),
            cap_height: font.cap_height.clone(),
            italic_angle: font.italic_angle.clone(),
            embedded_font_kind: font.embedded_font_kind.clone(),
            has_to_unicode: font.has_to_unicode,
            to_unicode_mapping_count: font.to_unicode_mapping_count,
            to_unicode_error: font.to_unicode_error.clone(),
        }
    }
}

impl From<&RawXObjectSummary> for PdfXObjectFragmentSnapshot {
    fn from(xobject: &RawXObjectSummary) -> Self {
        Self {
            resource_path: xobject.name.clone(),
            type_name: xobject.type_name.clone(),
            subtype_name: xobject.subtype_name.clone(),
            filters: xobject.filter_names.clone(),
            width_px: xobject.width_px,
            height_px: xobject.height_px,
            decoded_width_px: xobject.decoded_width_px,
            decoded_height_px: xobject.decoded_height_px,
            bits_per_pixel: xobject.bits_per_pixel,
            decoded_stream_sha256: xobject.decoded_stream_sha256.clone(),
            has_soft_mask: xobject.has_soft_mask,
            soft_mask_has_opaque_core: xobject.soft_mask_has_opaque_core,
            solid_rgb: xobject.solid_rgb,
        }
    }
}

impl From<&RawAnnotationSummary> for PdfAnnotationFragmentSnapshot {
    fn from(annotation: &RawAnnotationSummary) -> Self {
        Self {
            type_name: annotation.type_name.clone(),
            subtype_name: annotation.subtype_name.clone(),
            rect: annotation.rect.clone(),
            action_uri: annotation.action_uri.clone(),
            field_type_name: annotation.field_type_name.clone(),
            field_value: annotation.field_value.clone(),
        }
    }
}

fn canonical_font_name(name: &str) -> String {
    let bytes = name.as_bytes();
    if bytes.len() > 7
        && bytes[6] == b'+'
        && bytes[..6].iter().all(|byte| byte.is_ascii_uppercase())
    {
        name[7..].to_string()
    } else {
        name.to_string()
    }
}

fn canonical_operation(operation: &lopdf::content::Operation) -> String {
    let mut output = operation
        .operands
        .iter()
        .map(canonical_object)
        .collect::<Vec<_>>()
        .join(" ");
    if !output.is_empty() {
        output.push(' ');
    }
    output.push_str(&operation.operator);
    output
}

fn canonical_object(object: &LopdfObject) -> String {
    match object {
        LopdfObject::Null => "null".to_string(),
        LopdfObject::Boolean(value) => value.to_string(),
        LopdfObject::Integer(value) => value.to_string(),
        LopdfObject::Real(value) => canonical_number(*value),
        LopdfObject::Name(value) => format!("/{}", String::from_utf8_lossy(value)),
        LopdfObject::String(value, _) => format!("<{}>", hex(value)),
        LopdfObject::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_object)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        LopdfObject::Dictionary(dictionary) => {
            let mut entries = dictionary
                .iter()
                .filter(|(key, _)| key.as_slice() != b"Length")
                .map(|(key, value)| {
                    (
                        String::from_utf8_lossy(key).to_string(),
                        canonical_object(value),
                    )
                })
                .collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            format!(
                "<<{}>>",
                entries
                    .into_iter()
                    .map(|(key, value)| format!("/{key} {value}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        }
        LopdfObject::Stream(stream) => {
            let content = stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone());
            format!(
                "stream({},{:x})",
                canonical_object(&LopdfObject::Dictionary(stream.dict.clone())),
                Sha256::digest(content)
            )
        }
        // Indirect numbering and generations are writer allocation details.
        LopdfObject::Reference(_) => "@ref".to_string(),
    }
}

fn canonical_number(value: f32) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let value = format!("{value:.6}");
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use lopdf::{Document, Object, Stream, content::Operation, dictionary};

    use super::{canonical_font_name, canonical_object, canonical_operation};

    #[test]
    fn canonical_fragment_omits_indirect_ids_and_stream_lengths() {
        let mut document = Document::with_version("1.7");
        let reference = document.add_object(Object::Integer(42));
        let stream = Stream::new(
            dictionary! {
                "Length" => 999,
                "Subtype" => "Form",
                "Ref" => reference,
            },
            b"q Q".to_vec(),
        );

        let canonical = canonical_object(&Object::Stream(stream));

        assert!(!canonical.contains("999"));
        assert!(!canonical.contains("0 R"));
        assert!(canonical.contains("/Ref @ref"));
        assert!(canonical.contains("/Subtype /Form"));
    }

    #[test]
    fn canonical_fragment_uses_hex_strings_and_normalized_numbers() {
        let operation = Operation::new(
            "TJ",
            vec![Object::Array(vec![
                Object::string_literal([0, b'A', b'\n']),
                Object::Real(-0.0),
                Object::Real(12.34),
            ])],
        );

        assert_eq!(canonical_operation(&operation), "[<00410A> 0 12.34] TJ");
    }

    #[test]
    fn canonical_fragment_strips_only_real_subset_prefixes() {
        assert_eq!(canonical_font_name("ABCDEF+Calibri"), "Calibri");
        assert_eq!(canonical_font_name("AbCDEF+Calibri"), "AbCDEF+Calibri");
        assert_eq!(canonical_font_name("Calibri"), "Calibri");
    }
}

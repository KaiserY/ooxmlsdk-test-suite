use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs,
    io::{Cursor, Read},
    path::Path,
    sync::OnceLock,
};

use ooxmlsdk::parts::{
    PartRef, presentation_document::PresentationDocument,
    spreadsheet_document::SpreadsheetDocument, wordprocessing_document::WordprocessingDocument,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::BodyChoice;
use ooxmlsdk::sdk::{SdkPackage, SdkPart};
use quick_xml::{Reader, XmlVersion, escape::unescape, events::Event};
use serde::Deserialize;
use zip::ZipArchive;

#[derive(Clone, Copy, Debug)]
enum DocSampleKind {
    Wordprocessing,
    Spreadsheet,
    Presentation,
}

pub fn assert_package_file_round_trip(path: &Path, file_name: &str) {
    let kind = doc_sample_kind(file_name);
    let original_bytes = fs::read(path).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while reading source package {path:?}: {err}");
    });

    match kind {
        DocSampleKind::Wordprocessing => {
            let mut original = WordprocessingDocument::new_from_file(path).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while opening original wordprocessing package {path:?}: {err:?}");
      });
            let mut buffer = Cursor::new(Vec::new());
            original.save(&mut buffer).unwrap_or_else(|err| {
                panic!(
                    "round-trip failed for {file_name} while saving wordprocessing package: {err:?}"
                );
            });
            let roundtripped_bytes = buffer.into_inner();
            let mut reopened = WordprocessingDocument::new(Cursor::new(roundtripped_bytes.clone())).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while reopening saved wordprocessing package: {err:?}");
      });
            assert_wordprocessing_document_round_trip(&original, &reopened);
            assert_doc_sample_zip_equivalent(&original_bytes, &roundtripped_bytes, file_name);
            clear_deep_recursive_word_tables_for_known_fixture(&mut original, file_name);
            clear_deep_recursive_word_tables_for_known_fixture(&mut reopened, file_name);
        }
        DocSampleKind::Spreadsheet => {
            let original = SpreadsheetDocument::new_from_file(path).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while opening original spreadsheet package {path:?}: {err:?}");
      });
            let mut buffer = Cursor::new(Vec::new());
            original.save(&mut buffer).unwrap_or_else(|err| {
                panic!(
                    "round-trip failed for {file_name} while saving spreadsheet package: {err:?}"
                );
            });
            let roundtripped_bytes = buffer.into_inner();
            let reopened = SpreadsheetDocument::new(Cursor::new(roundtripped_bytes.clone()))
        .unwrap_or_else(|err| {
          panic!(
            "round-trip failed for {file_name} while reopening saved spreadsheet package: {err:?}"
          );
        });
            assert_spreadsheet_document_round_trip(&original, &reopened);
            assert_doc_sample_zip_equivalent(&original_bytes, &roundtripped_bytes, file_name);
        }
        DocSampleKind::Presentation => {
            let original = PresentationDocument::new_from_file(path).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while opening original presentation package {path:?}: {err:?}");
      });
            let mut buffer = Cursor::new(Vec::new());
            original.save(&mut buffer).unwrap_or_else(|err| {
                panic!(
                    "round-trip failed for {file_name} while saving presentation package: {err:?}"
                );
            });
            let roundtripped_bytes = buffer.into_inner();
            let reopened = PresentationDocument::new(Cursor::new(roundtripped_bytes.clone()))
        .unwrap_or_else(|err| {
          panic!(
            "round-trip failed for {file_name} while reopening saved presentation package: {err:?}"
          );
        });
            assert_presentation_document_round_trip(&original, &reopened);
            assert_doc_sample_zip_equivalent(&original_bytes, &roundtripped_bytes, file_name);
        }
    }
}

fn clear_deep_recursive_word_tables_for_known_fixture(
    package: &mut WordprocessingDocument,
    file_name: &str,
) {
    if !file_name.ends_with("Apache-POI/test-data/document/deep-table-cell.docx")
        && !file_name.ends_with("test-data/document/deep-table-cell.docx")
    {
        return;
    }

    let Ok(main_part) = package.main_document_part() else {
        return;
    };
    let Ok(root) = main_part.root_element_mut(package) else {
        return;
    };
    let Some(body) = root.body.as_mut() else {
        return;
    };
    for child in &mut body.body_choice {
        if let BodyChoice::Table(table) = child {
            table.clear_recursive_tables();
        }
    }
}

pub fn assert_package_file_invalid(path: &Path, file_name: &str) {
    let kind = doc_sample_kind(file_name);

    if has_ambiguous_dot_path_zip_entry(path, file_name) {
        return;
    }

    let result = match kind {
        DocSampleKind::Wordprocessing => {
            WordprocessingDocument::new_from_file(path).and_then(|mut package| {
                package
                    .main_document_part()?
                    .root_element(&mut package)
                    .map(|_| ())
            })
        }
        DocSampleKind::Spreadsheet => {
            SpreadsheetDocument::new_from_file(path).and_then(|mut package| {
                package
                    .workbook_part()?
                    .root_element(&mut package)
                    .map(|_| ())
            })
        }
        DocSampleKind::Presentation => {
            PresentationDocument::new_from_file(path).and_then(|mut package| {
                package
                    .presentation_part()?
                    .root_element(&mut package)
                    .map(|_| ())
            })
        }
    };

    assert!(
        result.is_err(),
        "expected {file_name} to be invalid so we can keep it out of round-trip coverage"
    );
}

fn has_ambiguous_dot_path_zip_entry(path: &Path, file_name: &str) -> bool {
    let bytes = fs::read(path).unwrap_or_else(|err| {
        panic!("invalid-package check failed for {file_name} while reading {path:?}: {err}");
    });
    let Ok(mut archive) = ZipArchive::new(Cursor::new(bytes)) else {
        return false;
    };
    let mut names = BTreeMap::new();

    for idx in 0..archive.len() {
        let Ok(file) = archive.by_index(idx) else {
            return false;
        };
        if file.is_dir() {
            continue;
        }

        let normalized = normalize_dot_path_zip_entry_name(file.name());
        if names.insert(normalized, file.name().to_string()).is_some() {
            return true;
        }
    }

    false
}

fn normalize_dot_path_zip_entry_name(name: &str) -> String {
    name.split('/')
        .filter(|segment| *segment != ".")
        .collect::<Vec<_>>()
        .join("/")
}

pub fn assert_package_file_opens(path: &Path, file_name: &str) {
    let kind = doc_sample_kind(file_name);

    match kind {
        DocSampleKind::Wordprocessing => {
            let package = WordprocessingDocument::new_from_file(path).unwrap();
            let main_part = package.main_document_part().unwrap();
            assert_eq!(part_path(&package, &main_part), "word/document.xml");
        }
        DocSampleKind::Spreadsheet => {
            let package = SpreadsheetDocument::new_from_file(path).unwrap();
            let workbook_part = package.workbook_part().unwrap();
            assert_eq!(part_path(&package, &workbook_part), "xl/workbook.xml");
        }
        DocSampleKind::Presentation => {
            let package = PresentationDocument::new_from_file(path).unwrap();
            let presentation_part = package.presentation_part().unwrap();
            assert_eq!(
                part_path(&package, &presentation_part),
                "ppt/presentation.xml"
            );
        }
    }
}

fn assert_wordprocessing_document_round_trip(
    original: &WordprocessingDocument,
    roundtripped: &WordprocessingDocument,
) {
    let original_main = original.main_document_part().unwrap();
    let roundtripped_main = roundtripped.main_document_part().unwrap();
    assert_eq!(
        part_path(original, &original_main),
        part_path(roundtripped, &roundtripped_main)
    );
    assert_eq!(original.parts().count(), roundtripped.parts().count());
    assert_eq!(
        original_main.get_all_parts(original).count(),
        roundtripped_main.get_all_parts(roundtripped).count()
    );
    assert_package_part_graph_equal(original, roundtripped);
    assert_part_subgraph_equal(original, roundtripped, original_main, roundtripped_main);
}

fn assert_spreadsheet_document_round_trip(
    original: &SpreadsheetDocument,
    roundtripped: &SpreadsheetDocument,
) {
    let original_workbook = original.workbook_part().unwrap();
    let roundtripped_workbook = roundtripped.workbook_part().unwrap();
    assert_eq!(
        part_path(original, &original_workbook),
        part_path(roundtripped, &roundtripped_workbook)
    );
    assert_eq!(original.parts().count(), roundtripped.parts().count());
    assert_eq!(
        original_workbook.worksheet_parts(original).count(),
        roundtripped_workbook.worksheet_parts(roundtripped).count()
    );
    assert_package_part_graph_equal(original, roundtripped);
    assert_part_subgraph_equal(
        original,
        roundtripped,
        original_workbook,
        roundtripped_workbook,
    );
}

fn assert_presentation_document_round_trip(
    original: &PresentationDocument,
    roundtripped: &PresentationDocument,
) {
    let original_presentation = original.presentation_part().unwrap();
    let roundtripped_presentation = roundtripped.presentation_part().unwrap();
    assert_eq!(
        part_path(original, &original_presentation),
        part_path(roundtripped, &roundtripped_presentation)
    );
    assert_eq!(original.parts().count(), roundtripped.parts().count());
    assert_eq!(
        original_presentation.slide_parts(original).count(),
        roundtripped_presentation.slide_parts(roundtripped).count()
    );
    assert_eq!(
        original_presentation.slide_master_parts(original).count(),
        roundtripped_presentation
            .slide_master_parts(roundtripped)
            .count()
    );
    assert_package_part_graph_equal(original, roundtripped);
    assert_part_subgraph_equal(
        original,
        roundtripped,
        original_presentation,
        roundtripped_presentation,
    );
}

fn assert_package_part_graph_equal<P>(original: &P, roundtripped: &P)
where
    P: SdkPackage,
{
    assert_eq!(
        package_direct_part_signature(original),
        package_direct_part_signature(roundtripped),
        "package direct part relationship graph differs"
    );
    assert_eq!(
        all_part_signature(original.get_all_parts().collect()),
        all_part_signature(roundtripped.get_all_parts().collect()),
        "package reachable part graph differs"
    );
}

fn assert_part_subgraph_equal<P, T>(
    original_package: &P,
    roundtripped_package: &P,
    original_part: T,
    roundtripped_part: T,
) where
    P: SdkPackage,
    T: SdkPart,
{
    assert_eq!(
        direct_part_signature(
            original_package,
            original_part.parts(original_package).collect()
        ),
        direct_part_signature(
            roundtripped_package,
            roundtripped_part.parts(roundtripped_package).collect()
        ),
        "main part direct relationship graph differs"
    );
    assert_eq!(
        all_part_signature(original_part.get_all_parts(original_package).collect()),
        all_part_signature(
            roundtripped_part
                .get_all_parts(roundtripped_package)
                .collect()
        ),
        "main part reachable graph differs"
    );
}

fn package_direct_part_signature<P: SdkPackage>(package: &P) -> Vec<(String, String)> {
    direct_part_signature(package, package.parts().collect())
}

fn direct_part_signature<P: SdkPackage>(
    _package: &P,
    parts: Vec<ooxmlsdk::parts::IdPartPair<'_>>,
) -> Vec<(String, String)> {
    parts
        .into_iter()
        .map(|pair| {
            (
                pair.relationship_id.to_string(),
                format!("{:?}", pair.part.part_id()),
            )
        })
        .collect()
}

fn all_part_signature(parts: Vec<PartRef>) -> Vec<String> {
    parts
        .into_iter()
        .map(|part| format!("{:?}", part.part_id()))
        .collect()
}

fn part_path<'a, P, T>(package: &'a P, part: &T) -> &'a str
where
    P: ooxmlsdk::sdk::SdkPackage,
    T: SdkPart,
{
    part.path(package).unwrap()
}

fn doc_sample_kind(file_name: &str) -> DocSampleKind {
    match Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("docx") | Some("dotx") | Some("docm") | Some("dotm") => DocSampleKind::Wordprocessing,
        Some("xlsx") | Some("xltx") | Some("xlsm") | Some("xltm") => DocSampleKind::Spreadsheet,
        Some("pptx") | Some("potx") | Some("pptm") | Some("potm") => DocSampleKind::Presentation,
        other => panic!("unsupported doc sample extension for {file_name}: {other:?}"),
    }
}

fn assert_doc_sample_zip_equivalent(original: &[u8], roundtripped: &[u8], file_name: &str) {
    let original = read_zip_entries(original, file_name);
    let roundtripped = read_zip_entries(roundtripped, file_name);
    let original = normalize_zip_entries_for_comparison(original, file_name, "original");
    let roundtripped =
        normalize_zip_entries_for_comparison(roundtripped, file_name, "roundtripped");

    let original_names: Vec<_> = original.keys().collect();
    let mut errors = Vec::new();

    for name in original.keys() {
        if !roundtripped.contains_key(name) && !is_empty_relationships_entry(name, &original[name])
        {
            errors.push(format!("missing zip entry: {name}"));
        }
    }

    for name in roundtripped.keys() {
        if !original.contains_key(name) && !is_empty_relationships_entry(name, &roundtripped[name])
        {
            errors.push(format!("extra zip entry: {name}"));
        }
    }

    for name in original_names {
        let original_bytes = original.get(name).expect("original entry missing");
        let Some(roundtripped_bytes) = roundtripped.get(name) else {
            continue;
        };

        if should_compare_entry_as_xml(name, original_bytes, roundtripped_bytes)
            || is_psmdcp_entry(name)
        {
            errors.extend(xml_equivalence_errors(
                original_bytes,
                roundtripped_bytes,
                file_name,
                name,
            ));
        } else if original_bytes != roundtripped_bytes {
            errors.push(format!(
                "{name}: binary entry mismatch: original {} bytes, roundtripped {} bytes",
                original_bytes.len(),
                roundtripped_bytes.len()
            ));
        }
    }

    assert!(
        errors.is_empty(),
        "doc sample round-trip mismatch for {file_name}\n{}",
        format_doc_sample_errors(&errors)
    );
}

fn read_zip_entries(bytes: &[u8], file_name: &str) -> BTreeMap<String, Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap_or_else(|err| {
        panic!("failed to open zip for {file_name}: {err}");
    });

    let mut entries = BTreeMap::new();
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx).unwrap_or_else(|err| {
            panic!("failed to read zip entry {idx} for {file_name}: {err}");
        });

        if file.is_dir() {
            continue;
        }

        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap_or_else(|err| {
            panic!(
                "failed to read zip entry {} for {file_name}: {err}",
                file.name()
            );
        });

        entries.insert(file.name().to_string(), data);
    }

    entries
}

fn normalize_zip_entries_for_comparison(
    entries: BTreeMap<String, Vec<u8>>,
    file_name: &str,
    side: &str,
) -> BTreeMap<String, Vec<u8>> {
    let mut normalized = BTreeMap::new();
    for (name, bytes) in entries {
        let comparison_name = normalize_relationship_entry_filename_for_comparison(&name)
            .unwrap_or_else(|| name.clone());
        if normalized.insert(comparison_name.clone(), bytes).is_some() {
            panic!(
                "ambiguous relationship zip entry casing in {side} package for {file_name}: {comparison_name}"
            );
        }
    }
    normalized
}

fn normalize_relationship_entry_filename_for_comparison(name: &str) -> Option<String> {
    if name == "_rels/.rels" || !name.ends_with(".rels") {
        return None;
    }

    let (parent_path, file_name) = name.rsplit_once('/')?;
    if !parent_path.ends_with("/_rels") {
        return None;
    }

    let lower_file_name = file_name.to_ascii_lowercase();
    if file_name == lower_file_name {
        return None;
    }

    let mut normalized = String::with_capacity(name.len());
    normalized.push_str(parent_path);
    normalized.push('/');
    normalized.push_str(&lower_file_name);
    Some(normalized)
}

fn should_compare_entry_as_xml(name: &str, original: &[u8], roundtripped: &[u8]) -> bool {
    if name == "[Content_Types].xml" || name.ends_with(".rels") {
        return true;
    }

    name.ends_with(".xml") && looks_like_xml_bytes(original) && looks_like_xml_bytes(roundtripped)
}

fn looks_like_xml_bytes(bytes: &[u8]) -> bool {
    let bytes = bytes
        .strip_prefix(b"\xEF\xBB\xBF")
        .or_else(|| bytes.strip_prefix(b"\xFE\xFF"))
        .or_else(|| bytes.strip_prefix(b"\xFF\xFE"))
        .unwrap_or(bytes);

    bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        == Some(b'<')
}

fn is_psmdcp_entry(name: &str) -> bool {
    name.ends_with(".psmdcp")
}

fn is_empty_relationships_entry(name: &str, bytes: &[u8]) -> bool {
    const RELATIONSHIPS_NAME: &str =
        "{http://schemas.openxmlformats.org/package/2006/relationships}Relationships";
    const RELATIONSHIP_NAME: &str =
        "{http://schemas.openxmlformats.org/package/2006/relationships}Relationship";

    if !name.ends_with(".rels") {
        return false;
    }

    let nodes = canonicalize_xml(bytes, CanonicalOptions::strict(), name, name);
    let element_roots: Vec<_> = nodes
        .iter()
        .filter_map(|node| match node {
            XmlNode::Element(element) => Some(element),
            XmlNode::Declaration(_) => None,
            XmlNode::Text(text) if text.trim().is_empty() => None,
            XmlNode::Text(_) => None,
        })
        .collect();

    let [root] = element_roots.as_slice() else {
        return false;
    };

    root.name == RELATIONSHIPS_NAME
        && !root.children.iter().any(|child| match child {
            XmlNode::Element(element) => element.name == RELATIONSHIP_NAME,
            XmlNode::Declaration(_) | XmlNode::Text(_) => false,
        })
}

fn format_doc_sample_errors(errors: &[String]) -> String {
    const MAX_ERRORS: usize = 120;

    let mut out = String::new();
    for error in errors.iter().take(MAX_ERRORS) {
        out.push_str("- ");
        out.push_str(error);
        out.push('\n');
    }

    if errors.len() > MAX_ERRORS {
        out.push_str(&format!(
            "- ... {} additional mismatches omitted\n",
            errors.len() - MAX_ERRORS
        ));
    }

    out
}

fn xml_equivalence_errors(
    original: &[u8],
    roundtripped: &[u8],
    file_name: &str,
    entry_name: &str,
) -> Vec<String> {
    let strict_options = CanonicalOptions::strict();
    let strict_original = canonicalize_xml(original, strict_options, file_name, entry_name);
    let strict_roundtripped = canonicalize_xml(roundtripped, strict_options, file_name, entry_name);
    let strict_errors = compare_xml_documents(&strict_original, &strict_roundtripped);

    if strict_errors.is_empty() {
        return Vec::new();
    }

    let relaxed_options = CanonicalOptions::relaxed_for_entry(entry_name);
    let relaxed_original = canonicalize_xml(original, relaxed_options, file_name, entry_name);
    let relaxed_roundtripped =
        canonicalize_xml(roundtripped, relaxed_options, file_name, entry_name);
    let relaxed_errors = compare_xml_documents(&relaxed_original, &relaxed_roundtripped);

    if relaxed_errors.is_empty() {
        return Vec::new();
    }

    format_xml_equivalence_errors(entry_name, relaxed_options, &strict_errors, &relaxed_errors)
}

fn format_xml_equivalence_errors(
    entry_name: &str,
    relaxed_options: CanonicalOptions,
    strict_errors: &[String],
    relaxed_errors: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();
    errors.push(format!(
        "{entry_name}: xml mismatch after strict and compatible comparison ({})",
        relaxed_options.describe()
    ));
    if let Some(summary) = summarize_xml_mismatch_cause(relaxed_errors) {
        errors.push(format!("{entry_name}: first structural cause: {summary}"));
    }

    for error in relaxed_errors.iter().take(24) {
        errors.push(format!("{entry_name}: relaxed: {error}"));
    }

    if relaxed_errors.len() > 24 {
        errors.push(format!(
            "{entry_name}: relaxed: ... {} additional XML mismatches omitted",
            relaxed_errors.len() - 24
        ));
    }

    errors.push(format!(
        "{entry_name}: strict comparison found {} mismatch(es); first strict mismatch: {}",
        strict_errors.len(),
        strict_errors.first().map(String::as_str).unwrap_or("none")
    ));

    errors
}

fn summarize_xml_mismatch_cause(errors: &[String]) -> Option<String> {
    errors
        .iter()
        .find_map(|error| summarize_single_xml_mismatch(error))
}

fn summarize_single_xml_mismatch(error: &str) -> Option<String> {
    if let Some((path, detail)) = error.split_once(": missing child in roundtripped XML: ") {
        return Some(format!(
            "round-trip dropped {} under {}",
            detail,
            summarize_xml_parent_path(path)
        ));
    }

    if let Some((path, detail)) = error.split_once(": extra child in roundtripped XML: ") {
        return Some(format!(
            "round-trip added {} under {}",
            detail,
            summarize_xml_parent_path(path)
        ));
    }

    if let Some((path, detail)) = error.split_once(": element name mismatch: original ")
        && let Some((original, roundtripped)) = detail.split_once(", roundtripped ")
    {
        return Some(format!(
            "element at {} changed from {} to {}",
            path, original, roundtripped
        ));
    }

    if let Some((path, detail)) = error.split_once(": node kind mismatch: original ")
        && let Some((original, roundtripped)) = detail.split_once(", roundtripped ")
    {
        return Some(format!(
            "node at {} changed from {} to {}",
            path, original, roundtripped
        ));
    }

    if let Some((path, detail)) = error.split_once(": missing attr in roundtripped XML: ") {
        return Some(format!(
            "round-trip dropped attribute {} at {}",
            detail, path
        ));
    }

    None
}

fn summarize_xml_parent_path(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or(path)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct XmlElement {
    name: String,
    raw_name: String,
    attrs: Vec<(String, String)>,
    raw_attrs: Vec<(String, String)>,
    children: Vec<XmlNode>,
}

impl Drop for XmlElement {
    fn drop(&mut self) {
        let mut stack = Vec::new();
        stack.append(&mut self.children);

        while let Some(node) = stack.pop() {
            if let XmlNode::Element(mut element) = node {
                stack.append(&mut element.children);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum XmlNode {
    Declaration(XmlDeclaration),
    Element(XmlElement),
    Text(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XmlDeclaration {
    Plain,
    Standalone,
}

#[derive(Debug)]
struct XmlFrame {
    name: String,
    raw_name: String,
    attrs: Vec<(String, String)>,
    raw_attrs: Vec<(String, String)>,
    children: Vec<XmlNode>,
    ns: BTreeMap<String, String>,
}

struct ParsedXmlNode {
    name: String,
    raw_name: String,
    attrs: Vec<(String, String)>,
    raw_attrs: Vec<(String, String)>,
    ns: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SchemaFloatKind {
    Single,
    Double,
}

#[derive(Debug, Deserialize)]
struct SchemaFloatRules {
    attrs: Vec<SchemaFloatAttrRule>,
    texts: Vec<SchemaFloatTextRule>,
}

#[derive(Debug, Deserialize)]
struct SchemaFloatAttrRule {
    element: String,
    attr: String,
    kind: SchemaFloatKind,
}

#[derive(Debug, Deserialize)]
struct SchemaFloatTextRule {
    element: String,
    kind: SchemaFloatKind,
}

#[derive(Clone, Copy)]
struct CanonicalOptions {
    normalize_float_lexemes: bool,
    normalize_measure_lexemes: bool,
    normalize_drawingml_percentage_lexemes: bool,
    normalize_signature_line_signing_instructions_attr: bool,
    normalize_word_indentation_left_attr: bool,
    ignore_empty_word_table_position_alignment_attrs: bool,
    ignore_empty_word_document_protection_crypt_algorithm_sid_attr: bool,
    normalize_doc_grid_char_space_overflow: bool,
    normalize_header_footer_odd_type: bool,
    normalize_core_property_dcterms_refinements: bool,
    ignore_core_property_whitespace_text_nodes: bool,
    ignore_spreadsheet_fills_text_nodes: bool,
    ignore_spreadsheet_cell_value_inline_string_type_attr: bool,
    ignore_word_text_leaf_mc_ignorable_attr: bool,
    ignore_word_text_leaf_xsi_nil_true_attr: bool,
    ignore_word_document_invalid_run_container_text_nodes: bool,
    sort_spreadsheet_stylesheet_children: bool,
    sort_package_properties: bool,
    sort_word_settings_children: bool,
    sort_word_document_paragraph_properties_children: bool,
    sort_word_numbering_paragraph_properties_children: bool,
    sort_word_document_numbering_properties_children: bool,
    normalize_word_document_duplicate_empty_paragraph_borders: bool,
    normalize_word_document_duplicate_run_properties: bool,
    sort_word_document_section_properties_children: bool,
    normalize_word_document_duplicate_table_cell_properties: bool,
    sort_word_document_table_cell_properties_children: bool,
    sort_word_document_table_row_properties_children: bool,
    sort_word_document_table_borders_children: bool,
    sort_word_numbering_abstract_children: bool,
    sort_word_numbering_level_children: bool,
    sort_word_numbering_instance_children: bool,
    sort_word_numbering_run_properties_children: bool,
    normalize_word_numbering_multilevel_type_attr: bool,
    normalize_word_numbering_level_suffix_attr: bool,
    normalize_word_style_duplicate_font_size: bool,
    sort_word_font_table_font_children: bool,
    sort_word_style_children: bool,
    sort_word_style_table_cell_margin_children: bool,
    sort_chart_schema_children: bool,
    normalize_chart_show_dlbls_over_max_extlst_order: bool,
    normalize_wordprocessing_drawing_position_offset_text: bool,
    sort_all_particle_children: bool,
}

impl CanonicalOptions {
    fn strict() -> Self {
        Self {
            normalize_float_lexemes: false,
            normalize_measure_lexemes: false,
            normalize_drawingml_percentage_lexemes: false,
            normalize_signature_line_signing_instructions_attr: false,
            normalize_word_indentation_left_attr: false,
            ignore_empty_word_table_position_alignment_attrs: false,
            ignore_empty_word_document_protection_crypt_algorithm_sid_attr: false,
            normalize_doc_grid_char_space_overflow: false,
            normalize_header_footer_odd_type: false,
            normalize_core_property_dcterms_refinements: false,
            ignore_core_property_whitespace_text_nodes: false,
            ignore_spreadsheet_fills_text_nodes: false,
            ignore_spreadsheet_cell_value_inline_string_type_attr: false,
            ignore_word_text_leaf_mc_ignorable_attr: false,
            ignore_word_text_leaf_xsi_nil_true_attr: false,
            ignore_word_document_invalid_run_container_text_nodes: false,
            sort_spreadsheet_stylesheet_children: false,
            sort_package_properties: false,
            sort_word_settings_children: false,
            sort_word_document_paragraph_properties_children: false,
            sort_word_numbering_paragraph_properties_children: false,
            sort_word_document_numbering_properties_children: false,
            normalize_word_document_duplicate_empty_paragraph_borders: false,
            normalize_word_document_duplicate_run_properties: false,
            sort_word_document_section_properties_children: false,
            normalize_word_document_duplicate_table_cell_properties: false,
            sort_word_document_table_cell_properties_children: false,
            sort_word_document_table_row_properties_children: false,
            sort_word_document_table_borders_children: false,
            sort_word_numbering_abstract_children: false,
            sort_word_numbering_level_children: false,
            sort_word_numbering_instance_children: false,
            sort_word_numbering_run_properties_children: false,
            normalize_word_numbering_multilevel_type_attr: false,
            normalize_word_numbering_level_suffix_attr: false,
            normalize_word_style_duplicate_font_size: false,
            sort_word_font_table_font_children: false,
            sort_word_style_children: false,
            sort_word_style_table_cell_margin_children: false,
            sort_chart_schema_children: false,
            normalize_chart_show_dlbls_over_max_extlst_order: false,
            normalize_wordprocessing_drawing_position_offset_text: false,
            sort_all_particle_children: false,
        }
    }

    fn relaxed_for_entry(entry_name: &str) -> Self {
        Self {
            normalize_float_lexemes: true,
            normalize_measure_lexemes: true,
            normalize_drawingml_percentage_lexemes: true,
            normalize_signature_line_signing_instructions_attr: is_word_document_entry(entry_name),
            normalize_word_indentation_left_attr: is_word_styles_entry(entry_name),
            ignore_empty_word_table_position_alignment_attrs: is_word_document_entry(entry_name),
            ignore_empty_word_document_protection_crypt_algorithm_sid_attr: is_word_settings_entry(
                entry_name,
            ),
            normalize_doc_grid_char_space_overflow: true,
            normalize_header_footer_odd_type: true,
            normalize_core_property_dcterms_refinements: is_package_properties_entry(entry_name),
            ignore_core_property_whitespace_text_nodes: is_package_properties_entry(entry_name),
            ignore_spreadsheet_fills_text_nodes: is_spreadsheet_styles_entry(entry_name),
            ignore_spreadsheet_cell_value_inline_string_type_attr: is_spreadsheet_worksheet_entry(
                entry_name,
            ),
            ignore_word_text_leaf_mc_ignorable_attr: is_word_document_entry(entry_name),
            ignore_word_text_leaf_xsi_nil_true_attr: is_word_document_entry(entry_name),
            ignore_word_document_invalid_run_container_text_nodes: is_word_document_entry(
                entry_name,
            ),
            sort_spreadsheet_stylesheet_children: is_spreadsheet_styles_entry(entry_name),
            sort_package_properties: is_package_properties_entry(entry_name),
            sort_word_settings_children: is_word_settings_entry(entry_name),
            sort_word_document_paragraph_properties_children: is_word_document_entry(entry_name)
                || is_word_styles_entry(entry_name)
                || is_word_header_footer_entry(entry_name),
            sort_word_numbering_paragraph_properties_children: is_word_numbering_entry(entry_name),
            sort_word_document_numbering_properties_children: is_word_document_entry(entry_name),
            normalize_word_document_duplicate_empty_paragraph_borders: is_word_document_entry(
                entry_name,
            ),
            normalize_word_document_duplicate_run_properties: is_word_undefined_styles_trial_entry(
                entry_name,
            ),
            sort_word_document_section_properties_children: is_word_document_entry(entry_name),
            normalize_word_document_duplicate_table_cell_properties: is_word_document_entry(
                entry_name,
            ),
            sort_word_document_table_cell_properties_children: is_word_document_entry(entry_name)
                || is_word_header_footer_entry(entry_name),
            sort_word_document_table_row_properties_children: is_word_document_entry(entry_name),
            sort_word_document_table_borders_children: is_word_document_entry(entry_name)
                || is_word_styles_entry(entry_name)
                || is_word_header_footer_entry(entry_name),
            sort_word_numbering_abstract_children: is_word_numbering_entry(entry_name),
            sort_word_numbering_level_children: is_word_numbering_entry(entry_name),
            sort_word_numbering_instance_children: is_word_numbering_entry(entry_name),
            sort_word_numbering_run_properties_children: is_word_numbering_entry(entry_name)
                || is_word_styles_entry(entry_name)
                || is_word_document_entry(entry_name),
            normalize_word_numbering_multilevel_type_attr: is_word_numbering_entry(entry_name),
            normalize_word_numbering_level_suffix_attr: is_word_numbering_entry(entry_name),
            normalize_word_style_duplicate_font_size: is_word_styles_entry(entry_name),
            sort_word_font_table_font_children: is_word_font_table_entry(entry_name),
            sort_word_style_children: is_word_styles_entry(entry_name),
            sort_word_style_table_cell_margin_children: is_word_styles_entry(entry_name),
            sort_chart_schema_children: is_chart_entry(entry_name),
            normalize_chart_show_dlbls_over_max_extlst_order: is_chart_entry(entry_name),
            normalize_wordprocessing_drawing_position_offset_text: is_word_document_entry(
                entry_name,
            ),
            sort_all_particle_children: true,
        }
    }

    fn describe(self) -> String {
        let mut enabled = Vec::new();
        if self.normalize_float_lexemes {
            enabled.push("schema float lexemes");
        }
        if self.normalize_measure_lexemes {
            enabled.push("OOXML measure lexemes");
        }
        if self.normalize_drawingml_percentage_lexemes {
            enabled.push("DrawingML percentage lexemes");
        }
        if self.normalize_signature_line_signing_instructions_attr {
            enabled.push("signature line signinginstructions namespace");
        }
        if self.normalize_word_indentation_left_attr {
            enabled.push("word indentation left namespace");
        }
        if self.ignore_empty_word_table_position_alignment_attrs {
            enabled.push("empty word table position alignment attrs");
        }
        if self.ignore_empty_word_document_protection_crypt_algorithm_sid_attr {
            enabled.push("empty word document protection cryptAlgorithmSid attr");
        }
        if self.normalize_doc_grid_char_space_overflow {
            enabled.push("docGrid charSpace overflow");
        }
        if self.normalize_header_footer_odd_type {
            enabled.push("header/footer odd type");
        }
        if self.normalize_core_property_dcterms_refinements {
            enabled.push("core property dcterms refinements");
        }
        if self.ignore_core_property_whitespace_text_nodes {
            enabled.push("core property whitespace text nodes");
        }
        if self.ignore_spreadsheet_fills_text_nodes {
            enabled.push("spreadsheet fills text nodes");
        }
        if self.ignore_spreadsheet_cell_value_inline_string_type_attr {
            enabled.push("spreadsheet cell value inlineStr type attr");
        }
        if self.ignore_word_text_leaf_mc_ignorable_attr {
            enabled.push("word text leaf mc:Ignorable attr");
        }
        if self.ignore_word_text_leaf_xsi_nil_true_attr {
            enabled.push("word text leaf xsi:nil true attr");
        }
        if self.ignore_word_document_invalid_run_container_text_nodes {
            enabled.push("word document invalid run container text nodes");
        }
        if self.sort_spreadsheet_stylesheet_children {
            enabled.push("spreadsheet stylesheet child order");
        }
        if self.sort_package_properties {
            enabled.push("package property order");
        }
        if self.sort_word_settings_children {
            enabled.push("word settings child order");
        }
        if self.sort_word_document_paragraph_properties_children {
            enabled.push("word document paragraph properties child order");
        }
        if self.sort_word_numbering_paragraph_properties_children {
            enabled.push("word numbering paragraph properties child order");
        }
        if self.sort_word_document_numbering_properties_children {
            enabled.push("word document numbering properties child order");
        }
        if self.normalize_word_document_duplicate_empty_paragraph_borders {
            enabled.push("word document duplicate empty paragraph borders");
        }
        if self.normalize_word_document_duplicate_run_properties {
            enabled.push("word document duplicate run properties");
        }
        if self.sort_word_document_section_properties_children {
            enabled.push("word document section properties child order");
        }
        if self.normalize_word_document_duplicate_table_cell_properties {
            enabled.push("word document duplicate table cell properties");
        }
        if self.sort_word_document_table_cell_properties_children {
            enabled.push("word document table cell properties child order");
        }
        if self.sort_word_document_table_row_properties_children {
            enabled.push("word document table row properties child order");
        }
        if self.sort_word_document_table_borders_children {
            enabled.push("word document table borders child order");
        }
        if self.sort_word_numbering_abstract_children {
            enabled.push("word numbering abstract child order");
        }
        if self.sort_word_numbering_level_children {
            enabled.push("word numbering level child order");
        }
        if self.sort_word_numbering_instance_children {
            enabled.push("word numbering instance child order");
        }
        if self.sort_word_numbering_run_properties_children {
            enabled.push("word numbering run properties child order");
        }
        if self.normalize_word_numbering_multilevel_type_attr {
            enabled.push("word numbering multiLevelType value");
        }
        if self.normalize_word_numbering_level_suffix_attr {
            enabled.push("word numbering level suffix value");
        }
        if self.normalize_word_style_duplicate_font_size {
            enabled.push("word style duplicate font size");
        }
        if self.sort_word_font_table_font_children {
            enabled.push("word font table font child order");
        }
        if self.sort_word_style_children {
            enabled.push("word style child order");
        }
        if self.sort_word_style_table_cell_margin_children {
            enabled.push("word style table cell margin child order");
        }
        if self.sort_chart_schema_children {
            enabled.push("chart schema child order");
        }
        if self.normalize_chart_show_dlbls_over_max_extlst_order {
            enabled.push("chart showDLblsOverMax/extLst order");
        }
        if self.normalize_wordprocessing_drawing_position_offset_text {
            enabled.push("wordprocessing drawing posOffset text whitespace");
        }
        if self.sort_all_particle_children {
            enabled.push("xsd:all child order");
        }

        if enabled.is_empty() {
            "no compatibility relaxations enabled".to_string()
        } else {
            format!("compatibility relaxations: {}", enabled.join(", "))
        }
    }
}

fn canonicalize_xml(
    xml: &[u8],
    options: CanonicalOptions,
    file_name: &str,
    entry_name: &str,
) -> Vec<XmlNode> {
    let decoded;
    let xml = if let Some(bytes) = decode_utf16_xml_bytes(xml) {
        decoded = bytes;
        decoded.as_slice()
    } else {
        xml
    };
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut roots = Vec::new();
    let mut stack: Vec<XmlFrame> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf).unwrap_or_else(|err| {
            panic!("failed to parse xml for {file_name}:{entry_name}: {err}");
        }) {
            Event::Start(event) => {
                let inherited_ns = stack.last().map(|frame| &frame.ns);
                let parsed = parse_xml_node(
                    &reader,
                    &event,
                    inherited_ns,
                    options,
                    file_name,
                    entry_name,
                );
                let frame = XmlFrame {
                    name: parsed.name,
                    raw_name: parsed.raw_name,
                    attrs: parsed.attrs,
                    raw_attrs: parsed.raw_attrs,
                    children: Vec::new(),
                    ns: parsed.ns,
                };
                stack.push(frame);
            }
            Event::Empty(event) => {
                let inherited_ns = stack.last().map(|frame| &frame.ns);
                let parsed = parse_xml_node(
                    &reader,
                    &event,
                    inherited_ns,
                    options,
                    file_name,
                    entry_name,
                );
                let node = XmlNode::Element(XmlElement {
                    name: parsed.name,
                    raw_name: parsed.raw_name,
                    attrs: parsed.attrs,
                    raw_attrs: parsed.raw_attrs,
                    children: Vec::new(),
                });
                push_xml_node(&mut roots, &mut stack, node);
            }
            Event::End(_) => {
                let frame = stack.pop().unwrap_or_else(|| {
                    panic!("unexpected xml end event for {file_name}:{entry_name}")
                });
                let node = XmlNode::Element(XmlElement {
                    name: frame.name,
                    raw_name: frame.raw_name,
                    attrs: frame.attrs,
                    raw_attrs: frame.raw_attrs,
                    children: frame.children,
                });
                push_xml_node(&mut roots, &mut stack, node);
            }
            Event::Text(event) => {
                let raw = unescape(&String::from_utf8_lossy(event.as_ref()))
                    .unwrap_or_else(|err| {
                        panic!("failed to decode xml text for {file_name}:{entry_name}: {err}");
                    })
                    .into_owned();
                if raw.chars().all(|ch| ch.is_whitespace()) && should_skip_whitespace_text(&stack) {
                    // Skip formatting-only whitespace.
                } else {
                    let text = normalize_xml_text(&raw, options, &stack);
                    push_xml_node(&mut roots, &mut stack, XmlNode::Text(text));
                }
            }
            Event::CData(event) => {
                let raw = String::from_utf8_lossy(event.as_ref());
                if !raw.chars().all(|ch| ch.is_whitespace()) || !should_skip_whitespace_text(&stack)
                {
                    let text = normalize_xml_text(raw.as_ref(), options, &stack);
                    push_xml_node(&mut roots, &mut stack, XmlNode::Text(text));
                }
            }
            Event::Decl(event) => {
                push_xml_node(
                    &mut roots,
                    &mut stack,
                    XmlNode::Declaration(xml_declaration_from_event(&event)),
                );
            }
            Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
            Event::GeneralRef(event) => {
                let raw = event.decode().unwrap_or_else(|err| {
                    panic!("failed to decode xml general ref for {file_name}:{entry_name}: {err}");
                });
                let text = unescape(&format!("&{raw};"))
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed to decode xml general ref for {file_name}:{entry_name}: {err}"
                        );
                    })
                    .into_owned();
                push_xml_node(&mut roots, &mut stack, XmlNode::Text(text));
            }
            Event::Eof => break,
        }

        buf.clear();
    }

    assert!(
        stack.is_empty(),
        "unterminated xml for {file_name}:{entry_name}"
    );
    normalize_xml_nodes_for_entry(roots, options, entry_name)
}

fn decode_utf16_xml_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let (little_endian, bytes) = match bytes {
        [0xFF, 0xFE, rest @ ..] => (true, rest),
        [0xFE, 0xFF, rest @ ..] => (false, rest),
        [b'<', 0, b'?', 0, ..] => (true, bytes),
        [0, b'<', 0, b'?', ..] => (false, bytes),
        _ => return None,
    };
    if bytes.len() % 2 != 0 {
        return None;
    }

    let code_units = bytes.chunks_exact(2).map(|chunk| {
        if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        }
    });
    let xml = std::char::decode_utf16(code_units)
        .collect::<Result<String, _>>()
        .ok()?;
    Some(normalize_utf16_xml_decl(xml).into_bytes())
}

fn normalize_utf16_xml_decl(mut xml: String) -> String {
    let Some(decl_end) = xml.find("?>").map(|end| end + 2) else {
        return xml;
    };
    if !xml[..decl_end].starts_with("<?xml") {
        return xml;
    }

    let Some(encoding_pos) = find_ascii_ignore_case(&xml[..decl_end], "encoding") else {
        return xml;
    };
    let bytes = xml.as_bytes();
    let mut pos = encoding_pos + "encoding".len();
    while pos < decl_end && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= decl_end || bytes[pos] != b'=' {
        return xml;
    }
    pos += 1;
    while pos < decl_end && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= decl_end || (bytes[pos] != b'"' && bytes[pos] != b'\'') {
        return xml;
    }

    let quote = bytes[pos];
    let value_start = pos + 1;
    let Some(value_end) = bytes[value_start..decl_end]
        .iter()
        .position(|&b| b == quote)
        .map(|offset| value_start + offset)
    else {
        return xml;
    };
    xml.replace_range(value_start..value_end, "UTF-8");
    xml
}

fn find_ascii_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn normalize_xml_nodes_for_entry(
    nodes: Vec<XmlNode>,
    options: CanonicalOptions,
    entry_name: &str,
) -> Vec<XmlNode> {
    enum NormalizeTask {
        Visit(XmlNode),
        Finish(XmlElement),
    }

    let mut roots = Vec::with_capacity(nodes.len());
    let mut child_lists: Vec<Vec<XmlNode>> = Vec::new();
    let mut tasks = Vec::with_capacity(nodes.len());

    for node in nodes.into_iter().rev() {
        tasks.push(NormalizeTask::Visit(node));
    }

    while let Some(task) = tasks.pop() {
        match task {
            NormalizeTask::Visit(XmlNode::Element(mut element)) => {
                let children = std::mem::take(&mut element.children);
                tasks.push(NormalizeTask::Finish(element));
                child_lists.push(Vec::with_capacity(children.len()));
                for child in children.into_iter().rev() {
                    tasks.push(NormalizeTask::Visit(child));
                }
            }
            NormalizeTask::Visit(node) => {
                push_normalized_xml_node(&mut roots, &mut child_lists, node);
            }
            NormalizeTask::Finish(mut element) => {
                let children = child_lists
                    .pop()
                    .expect("xml normalize child stack should not be empty");
                element.children = collapse_adjacent_xml_text_nodes(children);

                if options.normalize_wordprocessing_drawing_position_offset_text
                    && is_wordprocessing_drawing_position_offset(&element.name)
                {
                    trim_xml_text_children(&mut element.children);
                }
                if options.normalize_core_property_dcterms_refinements
                    && is_core_properties_root(&element.name)
                {
                    normalize_core_property_dcterms_refinement_children(&mut element.children);
                }
                if options.ignore_core_property_whitespace_text_nodes
                    && is_core_property_whitespace_text_relaxed_root(&element.name)
                {
                    element.children.retain(|child| {
                        !matches!(child, XmlNode::Text(text) if text.chars().all(char::is_whitespace))
                    });
                }
                if options.ignore_spreadsheet_fills_text_nodes
                    && is_spreadsheet_fills_root(&element.name)
                {
                    element
                        .children
                        .retain(|child| !matches!(child, XmlNode::Text(_)));
                }
                if options.ignore_word_document_invalid_run_container_text_nodes
                    && is_word_invalid_run_container_text_root(&element.name)
                {
                    element
                        .children
                        .retain(|child| !matches!(child, XmlNode::Text(_)));
                }
                if options.sort_spreadsheet_stylesheet_children
                    && is_spreadsheet_stylesheet_order_relaxed_root(&element.name)
                {
                    normalize_spreadsheet_stylesheet_child_order(&mut element.children);
                }

                if options.sort_package_properties
                    && is_package_properties_sort_root(entry_name, &element.name)
                {
                    element.children.sort_by_key(xml_node_structural_sort_key);
                }
                if options.sort_word_settings_children
                    && is_word_settings_order_relaxed_root(&element.name)
                {
                    element.children.sort_by_key(xml_node_structural_sort_key);
                }
                if options.sort_word_document_paragraph_properties_children
                    && is_word_paragraph_properties_order_relaxed_root(&element.name)
                {
                    if options.normalize_word_document_duplicate_empty_paragraph_borders {
                        normalize_word_duplicate_empty_paragraph_borders(&mut element.children);
                    }
                    normalize_word_paragraph_properties_child_order(&mut element.children);
                }
                if options.sort_word_numbering_paragraph_properties_children
                    && is_word_paragraph_properties_order_relaxed_root(&element.name)
                {
                    normalize_word_paragraph_properties_child_order(&mut element.children);
                }
                if options.sort_word_document_numbering_properties_children
                    && is_word_numbering_properties_order_relaxed_root(&element.name)
                {
                    normalize_word_numbering_properties_child_order(&mut element.children);
                }
                if options.sort_word_document_section_properties_children
                    && is_word_section_properties_order_relaxed_root(&element.name)
                {
                    normalize_word_section_properties_child_order(&mut element.children);
                }
                if options.normalize_word_document_duplicate_table_cell_properties
                    && is_word_table_cell_order_relaxed_root(&element.name)
                {
                    normalize_word_table_cell_duplicate_properties(&mut element.children);
                }
                if options.normalize_word_document_duplicate_run_properties
                    && is_word_run_order_relaxed_root(&element.name)
                {
                    normalize_word_duplicate_run_properties(&mut element.children);
                }
                if options.sort_word_document_table_cell_properties_children
                    && is_word_table_cell_properties_order_relaxed_root(&element.name)
                {
                    normalize_word_table_cell_properties_child_order(&mut element.children);
                }
                if options.sort_word_document_table_cell_properties_children
                    && is_word_table_properties_order_relaxed_root(&element.name)
                {
                    normalize_word_table_properties_child_order(&mut element.children);
                }
                if options.sort_word_document_table_row_properties_children
                    && is_word_table_row_properties_order_relaxed_root(&element.name)
                {
                    normalize_word_table_row_properties_child_order(&mut element.children);
                }
                if options.sort_word_document_table_borders_children
                    && is_word_table_borders_order_relaxed_root(&element.name)
                {
                    normalize_word_table_borders_child_order(&mut element.children);
                }
                if options.sort_word_numbering_abstract_children
                    && is_word_numbering_abstract_order_relaxed_root(&element.name)
                {
                    normalize_word_numbering_abstract_child_order(&mut element.children);
                }
                if options.sort_word_numbering_level_children
                    && is_word_numbering_level_order_relaxed_root(&element.name)
                {
                    element.children.sort_by_key(xml_node_structural_sort_key);
                }
                if options.sort_word_numbering_instance_children
                    && is_word_numbering_instance_order_relaxed_root(&element.name)
                {
                    normalize_word_numbering_instance_child_order(&mut element.children);
                }
                if options.sort_word_numbering_run_properties_children
                    && is_word_run_properties_order_relaxed_root(&element.name)
                {
                    if options.normalize_word_style_duplicate_font_size {
                        normalize_word_style_duplicate_font_size(&mut element.children);
                    }
                    normalize_word_run_properties_child_order(&mut element.children);
                }
                if options.sort_word_font_table_font_children
                    && is_word_font_table_font_order_relaxed_root(&element.name)
                {
                    element.children.sort_by_key(xml_node_structural_sort_key);
                }
                if options.sort_word_style_children
                    && is_word_style_order_relaxed_root(&element.name)
                {
                    element.children.sort_by_key(xml_node_structural_sort_key);
                }
                if options.sort_word_style_table_cell_margin_children
                    && is_word_table_cell_margin_order_relaxed_root(&element.name)
                {
                    normalize_word_table_cell_margin_child_order(&mut element.children);
                }
                if options.normalize_measure_lexemes
                    && is_word_table_cell_margin_order_relaxed_root(&element.name)
                {
                    normalize_word_table_cell_margin_width_lexemes(&mut element.children);
                }
                if options.sort_chart_schema_children {
                    normalize_chart_schema_child_order(&element.name, &mut element.children);
                }
                if options.normalize_chart_show_dlbls_over_max_extlst_order
                    && is_chart_order_relaxed_root(&element.name)
                {
                    normalize_chart_show_dlbls_over_max_extlst_order(&mut element.children);
                }
                if options.sort_all_particle_children && is_all_particle_sort_root(&element.name) {
                    element.children.sort_by_key(xml_node_structural_sort_key);
                }

                push_normalized_xml_node(&mut roots, &mut child_lists, XmlNode::Element(element));
            }
        }
    }

    assert!(
        child_lists.is_empty(),
        "xml normalize child stack should be empty"
    );
    collapse_adjacent_xml_text_nodes(roots)
}

fn push_normalized_xml_node(
    roots: &mut Vec<XmlNode>,
    child_lists: &mut [Vec<XmlNode>],
    node: XmlNode,
) {
    if let Some(children) = child_lists.last_mut() {
        children.push(node);
    } else {
        roots.push(node);
    }
}

fn collapse_adjacent_xml_text_nodes(nodes: Vec<XmlNode>) -> Vec<XmlNode> {
    let mut collapsed = Vec::with_capacity(nodes.len());

    for node in nodes {
        match (collapsed.last_mut(), node) {
            (Some(XmlNode::Text(existing)), XmlNode::Text(text)) => existing.push_str(&text),
            (_, node) => collapsed.push(node),
        }
    }

    collapsed
}

fn trim_xml_text_children(children: &mut [XmlNode]) {
    for child in children {
        if let XmlNode::Text(text) = child {
            let trimmed = text.trim_matches([' ', '\t', '\n', '\r']);
            if trimmed.len() != text.len() {
                *text = trimmed.to_string();
            }
        }
    }
}

fn normalize_core_property_dcterms_refinement_children(children: &mut [XmlNode]) {
    for child in children {
        let XmlNode::Element(element) = child else {
            continue;
        };
        if let Some(name) = normalized_core_property_dcterms_refinement_name(&element.name) {
            element.name = name;
        }
    }
}

fn normalize_chart_show_dlbls_over_max_extlst_order(children: &mut Vec<XmlNode>) {
    let Some(show_idx) = children
        .iter()
        .position(|child| xml_node_name(child) == Some(CHART_SHOW_DLBLS_OVER_MAX_NAME))
    else {
        return;
    };
    let Some(ext_idx) = children
        .iter()
        .position(|child| xml_node_name(child) == Some(CHART_EXT_LST_NAME))
    else {
        return;
    };

    if ext_idx > show_idx {
        return;
    }

    let ext_lst = children.remove(ext_idx);
    let insert_after_show_idx = children
        .iter()
        .position(|child| xml_node_name(child) == Some(CHART_SHOW_DLBLS_OVER_MAX_NAME))
        .expect("showDLblsOverMax should remain after removing chart extLst")
        + 1;
    children.insert(insert_after_show_idx, ext_lst);
}

fn normalize_word_paragraph_properties_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_paragraph_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn normalize_word_numbering_properties_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_numbering_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn normalize_word_duplicate_empty_paragraph_borders(children: &mut Vec<XmlNode>) {
    let mut seen_empty_paragraph_border = false;
    let mut keep = vec![true; children.len()];

    for (idx, child) in children.iter().enumerate().rev() {
        if !is_empty_word_paragraph_border(child) {
            continue;
        }

        if seen_empty_paragraph_border {
            keep[idx] = false;
        } else {
            seen_empty_paragraph_border = true;
        }
    }

    if keep.iter().all(|should_keep| *should_keep) {
        return;
    }

    let mut idx = 0;
    children.retain(|_| {
        let should_keep = keep[idx];
        idx += 1;
        should_keep
    });
}

fn is_empty_word_paragraph_border(node: &XmlNode) -> bool {
    let XmlNode::Element(element) = node else {
        return false;
    };

    element.name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pBdr"
        && element.attrs.is_empty()
        && element.children.is_empty()
}

fn word_paragraph_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pStyle" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}keepNext" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}keepLines" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pageBreakBefore" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}framePr" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}widowControl" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}numPr" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}suppressLineNumbers" => 7,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pBdr" => 8,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}shd" => 9,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tabs" => 10,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}suppressAutoHyphens" => 11,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}kinsoku" => 12,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}wordWrap" => 13,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}overflowPunct" => 14,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}topLinePunct" => 15,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}autoSpaceDE" => 16,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}autoSpaceDN" => 17,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bidi" => 18,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}adjustRightInd" => 19,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}snapToGrid" => 20,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}spacing" => 21,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}ind" => 22,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}contextualSpacing" => 23,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}mirrorIndents" => 24,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}suppressOverlap" => 25,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}jc" => 26,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}textDirection" => 27,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}textAlignment" => 28,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}textboxTightWrap" => 29,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}outlineLvl" => 30,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}divId" => 31,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cnfStyle" => 32,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rPr" => 33,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}sectPr" => 34,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pPrChange" => 35,
        _ => return None,
    })
}

fn normalize_spreadsheet_stylesheet_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        spreadsheet_stylesheet_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn spreadsheet_stylesheet_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}numFmts" => 0,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}fonts" => 1,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}fills" => 2,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}borders" => 3,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}cellStyleXfs" => 4,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}cellXfs" => 5,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}cellStyles" => 6,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}dxfs" => 7,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}tableStyles" => 8,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}colors" => 9,
        "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}extLst" => 10,
        _ => return None,
    })
}

fn normalize_word_section_properties_child_order(children: &mut Vec<XmlNode>) {
    let mut keep = vec![true; children.len()];
    for idx in (0..children.len()).rev() {
        let Some(name) = xml_node_name(&children[idx]) else {
            continue;
        };
        if !is_word_section_properties_singleton_child(name) {
            continue;
        }

        for later_idx in idx + 1..children.len() {
            if !keep[later_idx] || xml_node_name(&children[later_idx]) != Some(name) {
                continue;
            }
            if children[idx] == children[later_idx] {
                keep[idx] = false;
                break;
            }
        }
    }

    if keep.iter().any(|should_keep| !should_keep) {
        let mut idx = 0;
        children.retain(|_| {
            let should_keep = keep[idx];
            idx += 1;
            should_keep
        });
    }

    children.sort_by_key(|child| {
        word_section_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn is_word_section_properties_singleton_child(name: &str) -> bool {
    word_section_properties_child_rank(Some(name)).is_some()
        && !matches!(
            name,
            "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}headerReference"
                | "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}footerReference"
        )
}

fn word_section_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}headerReference" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}footerReference" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}footnotePr" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}endnotePr" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}type" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pgSz" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pgMar" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}paperSrc" => 7,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pgBorders" => 8,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}lnNumType" => 9,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pgNumType" => 10,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cols" => 11,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}formProt" => 12,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}vAlign" => 13,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}noEndnote" => 14,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}titlePg" => 15,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}textDirection" => 16,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bidi" => 17,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rtlGutter" => 18,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}docGrid" => 19,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}printerSettings" => 20,
        "{http://schemas.microsoft.com/office/word/2012/wordml}footnoteColumns" => 21,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}sectPrChange" => 22,
        _ => return None,
    })
}

fn normalize_word_table_borders_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_table_borders_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_table_borders_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}top" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}left" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}start" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bottom" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}right" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}end" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}insideH" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}insideV" => 7,
        _ => return None,
    })
}

fn is_word_table_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblPr"
}

fn normalize_word_table_properties_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_table_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_table_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblStyle" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblpPr" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblOverlap" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bidiVisual" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblStyleRowBandSize" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblStyleColBandSize" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblW" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}jc" => 7,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblCellSpacing" => 8,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblInd" => 9,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblBorders" => 10,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}shd" => 11,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblLayout" => 12,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblCellMar" => 13,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblLook" => 14,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblPrChange" => 15,
        _ => return None,
    })
}

fn is_word_table_row_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}trPr"
}

fn normalize_word_table_row_properties_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_table_row_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_table_row_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cnfStyle" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}divId" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}gridBefore" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}gridAfter" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}wBefore" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}wAfter" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblHeader" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}ins" => 7,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}del" => 8,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}trHeight" => 9,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}hidden" => 10,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cantSplit" => 11,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblCellSpacing" => 12,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}jc" => 13,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}trPrChange" => 14,
        "{http://schemas.microsoft.com/office/word/2010/wordml}conflictIns" => 15,
        "{http://schemas.microsoft.com/office/word/2010/wordml}conflictDel" => 16,
        _ => return None,
    })
}

fn word_numbering_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}ilvl" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}numId" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}numberingChange" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}ins" => 3,
        _ => return None,
    })
}

fn normalize_word_table_cell_properties_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_table_cell_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_table_cell_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cnfStyle" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcW" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}gridSpan" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}hMerge" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}vMerge" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcBorders" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}shd" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}noWrap" => 7,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcMar" => 8,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}textDirection" => 9,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcFitText" => 10,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}vAlign" => 11,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}hideMark" => 12,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}headers" => 13,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cellIns" => 14,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cellDel" => 15,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cellMerge" => 16,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcPrChange" => 17,
        _ => return None,
    })
}

fn normalize_word_table_cell_duplicate_properties(children: &mut Vec<XmlNode>) {
    let mut found_properties = false;
    let mut has_duplicate = false;
    for child in children.iter() {
        if xml_node_name(child)
            != Some("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcPr")
        {
            continue;
        }
        if found_properties {
            has_duplicate = true;
            break;
        }
        found_properties = true;
    }
    if !has_duplicate {
        return;
    }

    let mut keep = vec![true; children.len()];
    for idx in (0..children.len()).rev() {
        if xml_node_name(&children[idx])
            != Some("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcPr")
        {
            continue;
        }

        for later_idx in idx + 1..children.len() {
            if !keep[later_idx]
                || xml_node_name(&children[later_idx])
                    != Some("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcPr")
            {
                continue;
            }
            if children[idx] == children[later_idx] {
                keep[idx] = false;
                break;
            }
        }
    }

    let mut idx = 0;
    children.retain(|_| {
        let should_keep = keep[idx];
        idx += 1;
        should_keep
    });
}

fn normalize_word_table_cell_margin_child_order(children: &mut Vec<XmlNode>) {
    let mut seen = Vec::new();
    let mut keep = vec![true; children.len()];
    for (idx, child) in children.iter().enumerate().rev() {
        let Some(name) = xml_node_name(child) else {
            continue;
        };
        if word_table_cell_margin_child_rank(Some(name)).is_none() {
            continue;
        }
        if seen.iter().any(|seen_name| seen_name == name) {
            keep[idx] = false;
        } else {
            seen.push(name.to_string());
        }
    }
    let mut idx = 0;
    children.retain(|_| {
        let should_keep = keep[idx];
        idx += 1;
        should_keep
    });
    children.sort_by_key(|child| {
        word_table_cell_margin_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_table_cell_margin_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}top" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}start" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}left" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bottom" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}end" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}right" => 5,
        _ => return None,
    })
}

fn normalize_word_numbering_abstract_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_numbering_abstract_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_numbering_abstract_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}nsid" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}multiLevelType" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tmpl" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}name" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}styleLink" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}numStyleLink" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}lvl" => 6,
        _ => return None,
    })
}

fn normalize_word_numbering_instance_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_numbering_instance_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn word_numbering_instance_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}abstractNumId" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}lvlOverride" => 1,
        _ => return None,
    })
}

fn normalize_word_run_properties_child_order(children: &mut [XmlNode]) {
    children.sort_by_key(|child| {
        word_run_properties_child_rank(xml_node_name(child)).unwrap_or(u16::MAX)
    });
}

fn normalize_word_style_duplicate_font_size(children: &mut Vec<XmlNode>) {
    let mut seen_font_size = false;
    let mut index = children.len();
    while index > 0 {
        index -= 1;
        if xml_node_name(&children[index])
            == Some("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}sz")
        {
            if seen_font_size {
                children.remove(index);
            } else {
                seen_font_size = true;
            }
        }
    }
}

fn normalize_word_duplicate_run_properties(children: &mut Vec<XmlNode>) {
    let mut seen_run_properties = false;
    let mut index = children.len();
    while index > 0 {
        index -= 1;
        if xml_node_name(&children[index])
            == Some("{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rPr")
        {
            if seen_run_properties {
                children.remove(index);
            } else {
                seen_run_properties = true;
            }
        }
    }
}

fn word_run_properties_child_rank(name: Option<&str>) -> Option<u16> {
    Some(match name? {
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rStyle" => 0,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rFonts" => 1,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}b" => 2,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bCs" => 3,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}i" => 4,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}iCs" => 5,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}caps" => 6,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}smallCaps" => 7,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}strike" => 8,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}dstrike" => 9,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}outline" => 10,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}shadow" => 11,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}emboss" => 12,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}imprint" => 13,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}noProof" => 14,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}snapToGrid" => 15,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}vanish" => 16,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}webHidden" => 17,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}color" => 18,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}spacing" => 19,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}w" => 20,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}kern" => 21,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}position" => 22,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}sz" => 23,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}szCs" => 24,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}highlight" => 25,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}u" => 26,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}effect" => 27,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}bdr" => 28,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}shd" => 29,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}fitText" => 30,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}vertAlign" => 31,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rtl" => 32,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cs" => 33,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}em" => 34,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}lang" => 35,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}eastAsianLayout" => 36,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}specVanish" => 37,
        _ => return None,
    })
}

fn normalize_chart_schema_child_order(parent_name: &str, children: &mut [XmlNode]) {
    let Some(rank_fn) = chart_child_rank_fn(parent_name) else {
        return;
    };

    children.sort_by_key(|child| rank_fn(xml_node_name(child)).unwrap_or(u16::MAX));
}

type ChartChildRankFn = fn(Option<&str>) -> Option<u16>;

fn chart_child_rank_fn(parent_name: &str) -> Option<ChartChildRankFn> {
    match parent_name {
        CHART_ROOT_NAME => Some(chart_root_child_rank),
        CHART_PLOT_AREA_NAME => Some(chart_plot_area_child_rank),
        CHART_CAT_AXIS_NAME | CHART_VAL_AXIS_NAME | CHART_SER_AXIS_NAME | CHART_DATE_AXIS_NAME => {
            Some(chart_axis_child_rank)
        }
        _ => None,
    }
}

fn chart_root_child_rank(name: Option<&str>) -> Option<u16> {
    match name? {
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}title" => Some(0),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}autoTitleDeleted" => Some(1),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}pivotFmts" => Some(2),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}view3D" => Some(3),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}floor" => Some(4),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}sideWall" => Some(5),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}backWall" => Some(6),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}plotArea" => Some(7),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}legend" => Some(8),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}plotVisOnly" => Some(9),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}dispBlanksAs" => Some(10),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}showDLblsOverMax" => Some(11),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}extLst" => Some(12),
        _ => None,
    }
}

fn chart_plot_area_child_rank(name: Option<&str>) -> Option<u16> {
    match name? {
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}layout" => Some(0),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}areaChart" => Some(10),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}area3DChart" => Some(11),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}lineChart" => Some(12),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}line3DChart" => Some(13),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}stockChart" => Some(14),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}radarChart" => Some(15),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}scatterChart" => Some(16),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}pieChart" => Some(17),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}pie3DChart" => Some(18),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}doughnutChart" => Some(19),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}barChart" => Some(20),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}bar3DChart" => Some(21),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}ofPieChart" => Some(22),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}surfaceChart" => Some(23),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}surface3DChart" => Some(24),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}bubbleChart" => Some(25),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}valAx" => Some(40),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}catAx" => Some(41),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}dateAx" => Some(42),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}serAx" => Some(43),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}dTable" => Some(50),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}spPr" => Some(51),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}extLst" => Some(52),
        _ => None,
    }
}

fn chart_axis_child_rank(name: Option<&str>) -> Option<u16> {
    match name? {
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}axId" => Some(0),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}scaling" => Some(1),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}delete" => Some(2),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}axPos" => Some(3),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}majorGridlines" => Some(4),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}minorGridlines" => Some(5),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}title" => Some(6),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}numFmt" => Some(7),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}majorTickMark" => Some(8),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}minorTickMark" => Some(9),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}tickLblPos" => Some(10),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}spPr" => Some(11),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}txPr" => Some(12),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}crossAx" => Some(13),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}crosses" => Some(14),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}crossesAt" => Some(14),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}auto" => Some(20),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}lblAlgn" => Some(21),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}lblOffset" => Some(22),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}tickLblSkip" => Some(23),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}tickMarkSkip" => Some(24),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}noMultiLvlLbl" => Some(25),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}crossBetween" => Some(30),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}majorUnit" => Some(31),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}minorUnit" => Some(32),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}dispUnits" => Some(33),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}baseTimeUnit" => Some(40),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}majorTimeUnit" => Some(41),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}minorTimeUnit" => Some(42),
        "{http://schemas.openxmlformats.org/drawingml/2006/chart}extLst" => Some(50),
        _ => None,
    }
}

fn normalized_core_property_dcterms_refinement_name(name: &str) -> Option<String> {
    const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
    const DCTERMS_NS: &str = "http://purl.org/dc/terms/";

    let (ns, local) = split_expanded_name(name);
    if ns != DCTERMS_NS || matches!(local, "created" | "modified") {
        return None;
    }

    if matches!(
        local,
        "creator" | "description" | "identifier" | "language" | "subject" | "title"
    ) {
        Some(format!("{{{DC_NS}}}{local}"))
    } else {
        None
    }
}

fn compare_xml_documents(original: &[XmlNode], roundtripped: &[XmlNode]) -> Vec<String> {
    struct CompareListFrame<'a> {
        parent_path: String,
        original: &'a [XmlNode],
        roundtripped: &'a [XmlNode],
        original_idx: usize,
        roundtripped_idx: usize,
        original_ordinals: BTreeMap<String, usize>,
        roundtripped_ordinals: BTreeMap<String, usize>,
    }

    enum CompareTask<'a> {
        List(CompareListFrame<'a>),
        Node {
            path: String,
            original: &'a XmlNode,
            roundtripped: &'a XmlNode,
        },
    }

    let mut errors = Vec::new();
    let mut stack = vec![CompareTask::List(CompareListFrame {
        parent_path: "$".to_string(),
        original,
        roundtripped,
        original_idx: 0,
        roundtripped_idx: 0,
        original_ordinals: BTreeMap::new(),
        roundtripped_ordinals: BTreeMap::new(),
    })];

    while let Some(task) = stack.pop() {
        match task {
            CompareTask::List(mut frame) => {
                let mut next_node = None;
                while frame.original_idx < frame.original.len()
                    && frame.roundtripped_idx < frame.roundtripped.len()
                {
                    if matches!(frame.original[frame.original_idx], XmlNode::Declaration(_)) {
                        frame.original_idx += 1;
                        continue;
                    }
                    if matches!(
                        frame.roundtripped[frame.roundtripped_idx],
                        XmlNode::Declaration(_)
                    ) {
                        frame.roundtripped_idx += 1;
                        continue;
                    }

                    let path = next_xml_child_path(
                        &frame.parent_path,
                        &mut frame.original_ordinals,
                        &frame.original[frame.original_idx],
                    );
                    let _ = next_xml_child_path(
                        &frame.parent_path,
                        &mut frame.roundtripped_ordinals,
                        &frame.roundtripped[frame.roundtripped_idx],
                    );
                    next_node = Some(CompareTask::Node {
                        path,
                        original: &frame.original[frame.original_idx],
                        roundtripped: &frame.roundtripped[frame.roundtripped_idx],
                    });
                    frame.original_idx += 1;
                    frame.roundtripped_idx += 1;
                    break;
                }

                if let Some(next_node) = next_node {
                    stack.push(CompareTask::List(frame));
                    stack.push(next_node);
                } else {
                    for node in &frame.original[frame.original_idx..] {
                        if matches!(node, XmlNode::Declaration(_)) {
                            continue;
                        }
                        errors.push(format!(
                            "{}: missing child in roundtripped XML: {}",
                            frame.parent_path,
                            xml_node_summary(node)
                        ));
                    }

                    for node in &frame.roundtripped[frame.roundtripped_idx..] {
                        if matches!(node, XmlNode::Declaration(_)) {
                            continue;
                        }
                        errors.push(format!(
                            "{}: extra child in roundtripped XML: {}",
                            frame.parent_path,
                            xml_node_summary(node)
                        ));
                    }
                }
            }
            CompareTask::Node {
                path,
                original,
                roundtripped,
            } => match (original, roundtripped) {
                (XmlNode::Declaration(_), _) | (_, XmlNode::Declaration(_)) => {}
                (XmlNode::Text(left), XmlNode::Text(right)) => {
                    if left != right {
                        errors.push(format!(
                            "{path}: text mismatch: original {:?}, roundtripped {:?}",
                            truncate_for_error(left),
                            truncate_for_error(right)
                        ));
                    }
                }
                (XmlNode::Element(left), XmlNode::Element(right)) => {
                    if left.name != right.name {
                        push_xml_name_error(&path, "element", &left.name, &right.name, &mut errors);
                        errors.push(format!(
                            "{path}: original XML snippet: {}",
                            xml_element_snippet(left)
                        ));
                        errors.push(format!(
                            "{path}: roundtripped XML snippet: {}",
                            xml_element_snippet(right)
                        ));
                    }

                    compare_xml_attrs(&path, &left.attrs, &right.attrs, &mut errors);
                    stack.push(CompareTask::List(CompareListFrame {
                        parent_path: path,
                        original: &left.children,
                        roundtripped: &right.children,
                        original_idx: 0,
                        roundtripped_idx: 0,
                        original_ordinals: BTreeMap::new(),
                        roundtripped_ordinals: BTreeMap::new(),
                    }));
                }
                (left, right) => {
                    errors.push(format!(
                        "{path}: node kind mismatch: original {}, roundtripped {}",
                        xml_node_summary(left),
                        xml_node_summary(right)
                    ));
                }
            },
        }
    }

    errors
}

fn compare_xml_attrs(
    path: &str,
    original: &[(String, String)],
    roundtripped: &[(String, String)],
    errors: &mut Vec<String>,
) {
    let original = original
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect::<BTreeMap<_, _>>();
    let roundtripped = roundtripped
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect::<BTreeMap<_, _>>();

    for (name, value) in &original {
        match roundtripped.get(name) {
            Some(roundtripped_value)
                if normalized_xml_attr_value(roundtripped_value).as_ref()
                    == normalized_xml_attr_value(value).as_ref() => {}
            Some(roundtripped_value) => errors.push(format!(
                "{path}: attr value mismatch for {}: original {:?}, roundtripped {:?}",
                readable_xml_name(name),
                truncate_for_error(value),
                truncate_for_error(roundtripped_value)
            )),
            None => errors.push(format!(
                "{path}: missing attr in roundtripped XML: {}={:?}",
                readable_xml_name(name),
                truncate_for_error(value)
            )),
        }
    }

    for (name, value) in &roundtripped {
        if !original.contains_key(name) {
            errors.push(format!(
                "{path}: extra attr in roundtripped XML: {}={:?}",
                readable_xml_name(name),
                truncate_for_error(value)
            ));
        }
    }
}

fn normalized_xml_attr_value(value: &str) -> Cow<'_, str> {
    if !value.bytes().any(|b| matches!(b, b'\t' | b'\n' | b'\r')) {
        return Cow::Borrowed(value);
    }

    Cow::Owned(
        value
            .chars()
            .map(|ch| match ch {
                '\t' | '\n' | '\r' => ' ',
                _ => ch,
            })
            .collect(),
    )
}

fn push_xml_name_error(
    path: &str,
    kind: &str,
    original: &str,
    roundtripped: &str,
    errors: &mut Vec<String>,
) {
    let (original_ns, original_local) = split_expanded_name(original);
    let (roundtripped_ns, roundtripped_local) = split_expanded_name(roundtripped);

    if original_local == roundtripped_local && !original_ns.is_empty() && roundtripped_ns.is_empty()
    {
        errors.push(format!(
      "{path}: missing namespace on {kind}: local name {original_local}, expected namespace {original_ns}"
    ));
    } else if original_local == roundtripped_local
        && original_ns.is_empty()
        && !roundtripped_ns.is_empty()
    {
        errors.push(format!(
      "{path}: extra namespace on {kind}: local name {original_local}, roundtripped namespace {roundtripped_ns}"
    ));
    } else if original_local == roundtripped_local {
        errors.push(format!(
      "{path}: {kind} namespace mismatch: original {original_ns}, roundtripped {roundtripped_ns}, local name {original_local}"
    ));
    } else {
        errors.push(format!(
            "{path}: {kind} name mismatch: original {}, roundtripped {}",
            readable_xml_name(original),
            readable_xml_name(roundtripped)
        ));
    }
}

fn next_xml_child_path(
    parent_path: &str,
    ordinals: &mut BTreeMap<String, usize>,
    node: &XmlNode,
) -> String {
    let key = xml_child_path_key(node);
    let ordinal = ordinals.entry(key).or_default();
    *ordinal += 1;
    xml_child_path(parent_path, node, *ordinal)
}

fn xml_child_path(parent_path: &str, node: &XmlNode, ordinal: usize) -> String {
    match node {
        XmlNode::Declaration(_) => format!("{parent_path}/xml-declaration[{ordinal}]"),
        XmlNode::Element(element) => {
            format!(
                "{parent_path}/{}[{ordinal}]",
                readable_xml_name(&element.name)
            )
        }
        XmlNode::Text(_) => format!("{parent_path}/text()[{ordinal}]"),
    }
}

fn xml_child_path_key(node: &XmlNode) -> String {
    match node {
        XmlNode::Declaration(_) => "xml-declaration".to_string(),
        XmlNode::Element(element) => {
            let mut key = String::with_capacity(element.name.len() + 8);
            key.push_str("element:");
            key.push_str(&element.name);
            key
        }
        XmlNode::Text(_) => "text".to_string(),
    }
}

fn xml_node_summary(node: &XmlNode) -> String {
    match node {
        XmlNode::Declaration(decl) => format!("XML declaration {}", xml_declaration_summary(*decl)),
        XmlNode::Element(element) => format!("element {}", readable_xml_name(&element.name)),
        XmlNode::Text(text) => format!("text {:?}", truncate_for_error(text)),
    }
}

fn xml_element_snippet(element: &XmlElement) -> String {
    truncate_for_error(&render_xml_element_snippet(element, 3))
}

fn render_xml_element_snippet(element: &XmlElement, depth: usize) -> String {
    let mut out = String::new();
    out.push('<');
    out.push_str(&element.raw_name);
    for (name, value) in &element.raw_attrs {
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        out.push_str(&escape_xml_attr(value));
        out.push('"');
    }

    if element.children.is_empty() {
        out.push_str("/>");
        return out;
    }

    out.push('>');
    if depth == 0 {
        out.push_str("...");
    } else {
        for child in &element.children {
            match child {
                XmlNode::Declaration(_) => {}
                XmlNode::Element(child) => {
                    out.push_str(&render_xml_element_snippet(child, depth - 1))
                }
                XmlNode::Text(text) => out.push_str(text),
            }
        }
    }
    out.push_str("</");
    out.push_str(&element.raw_name);
    out.push('>');
    out
}

fn xml_node_structural_sort_key(node: &XmlNode) -> String {
    match node {
        XmlNode::Declaration(decl) => format!("0:{}", xml_declaration_summary(*decl)),
        XmlNode::Element(element) => {
            let (_, local_name) = split_expanded_name(&element.name);
            let mut key = format!("1:{local_name}");
            for (name, value) in &element.attrs {
                let (_, attr_local_name) = split_expanded_name(name);
                key.push('|');
                key.push_str(attr_local_name);
                key.push('=');
                key.push_str(value);
            }
            for child in &element.children {
                key.push_str(">{");
                key.push_str(&xml_node_structural_sort_key(child));
                key.push('}');
            }
            key
        }
        XmlNode::Text(text) => format!("2:{text}"),
    }
}

fn xml_declaration_summary(decl: XmlDeclaration) -> &'static str {
    match decl {
        XmlDeclaration::Plain => "<?xml?>",
        XmlDeclaration::Standalone => "<?xml standalone=\"yes\"?>",
    }
}

fn readable_xml_name(name: &str) -> String {
    let (ns, local) = split_expanded_name(name);
    if ns.is_empty() {
        local.to_string()
    } else {
        format!("{{{ns}}}{local}")
    }
}

fn truncate_for_error(value: &str) -> String {
    const MAX_CHARS: usize = 160;

    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx == MAX_CHARS {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }

    out
}

fn parse_xml_node(
    reader: &Reader<Cursor<&[u8]>>,
    event: &quick_xml::events::BytesStart<'_>,
    inherited_ns: Option<&BTreeMap<String, String>>,
    options: CanonicalOptions,
    file_name: &str,
    entry_name: &str,
) -> ParsedXmlNode {
    let mut raw_attrs = Vec::new();
    let mut ns = inherited_ns.cloned().unwrap_or_default();
    for attr in event.attributes().with_checks(false) {
        let attr = attr.unwrap_or_else(|err| {
            panic!("failed to parse xml attribute for {file_name}:{entry_name}: {err}");
        });
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .unwrap_or_else(|err| {
                panic!("failed to decode xml attribute for {file_name}:{entry_name}: {err}");
            })
            .into_owned();
        if key == "xmlns" {
            ns.insert(String::new(), value.clone());
        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
            ns.insert(prefix.to_string(), value.clone());
        }
        raw_attrs.push((key, value));
    }

    let raw_name = String::from_utf8_lossy(event.name().as_ref()).into_owned();
    let name = expand_xml_name(&raw_name, &ns, false);

    let mut attrs = Vec::new();
    for (key, value) in &raw_attrs {
        if key == "xmlns" || key.starts_with("xmlns:") {
            continue;
        }

        let expanded_key = expand_xml_name(key, &ns, true);
        if options.ignore_empty_word_table_position_alignment_attrs
            && value.is_empty()
            && is_empty_word_table_position_alignment_attr(&name, &expanded_key)
        {
            continue;
        }
        if options.ignore_empty_word_document_protection_crypt_algorithm_sid_attr
            && value.is_empty()
            && is_empty_word_document_protection_crypt_algorithm_sid_attr(&name, &expanded_key)
        {
            continue;
        }
        if options.ignore_word_text_leaf_mc_ignorable_attr
            && is_word_text_leaf_mc_ignorable_attr(&name, &expanded_key)
        {
            continue;
        }
        if options.ignore_word_text_leaf_xsi_nil_true_attr
            && value == "true"
            && is_word_text_leaf_xsi_nil_attr(&name, &expanded_key)
        {
            continue;
        }
        if options.ignore_spreadsheet_cell_value_inline_string_type_attr
            && value == "inlineStr"
            && is_spreadsheet_cell_value_type_attr(&name, &expanded_key)
        {
            continue;
        }

        let value = if is_mc_ignorable_attr(&expanded_key) {
            normalize_ignorable_prefix_list(value, &ns)
        } else if entry_name.ends_with(".rels") && key == "Type" {
            normalize_relationship_type_uri(value)
        } else {
            value.clone()
        };
        let value = if options.normalize_float_lexemes {
            schema_float_kind_for_attr(&name, &expanded_key)
                .map(|kind| normalize_schema_float_lexeme(&value, kind))
                .unwrap_or(value)
        } else {
            value
        };
        let value = if options.normalize_measure_lexemes {
            normalize_ooxml_measure_attr_lexeme(&name, &expanded_key, &value).unwrap_or(value)
        } else {
            value
        };
        let value = if options.normalize_drawingml_percentage_lexemes {
            normalize_drawingml_percentage_attr_lexeme(&name, &expanded_key, &value)
                .unwrap_or(value)
        } else {
            value
        };
        let value = if options.normalize_doc_grid_char_space_overflow {
            normalize_doc_grid_char_space_overflow(&name, &expanded_key, &value).unwrap_or(value)
        } else {
            value
        };
        let value = if options.normalize_header_footer_odd_type {
            normalize_header_footer_odd_type(&name, &expanded_key, &value).unwrap_or(value)
        } else {
            value
        };
        let value = if options.normalize_word_numbering_multilevel_type_attr {
            normalize_word_numbering_multilevel_type_attr(&name, &expanded_key, &value)
                .unwrap_or(value)
        } else {
            value
        };
        let value = if options.normalize_word_numbering_level_suffix_attr {
            normalize_word_numbering_level_suffix_attr(&name, &expanded_key, &value)
                .unwrap_or(value)
        } else {
            value
        };

        let expanded_key = if options.normalize_signature_line_signing_instructions_attr {
            normalize_signature_line_signing_instructions_attr_key(&name, &expanded_key)
                .unwrap_or(expanded_key)
        } else {
            expanded_key
        };
        let expanded_key = if options.normalize_word_indentation_left_attr {
            normalize_word_indentation_left_attr_key(&name, &expanded_key).unwrap_or(expanded_key)
        } else {
            expanded_key
        };

        attrs.push((expanded_key, value));
    }

    attrs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    ParsedXmlNode {
        name,
        raw_name,
        attrs,
        raw_attrs,
        ns,
    }
}

fn expand_xml_name(name: &str, namespaces: &BTreeMap<String, String>, is_attr: bool) -> String {
    if let Some((prefix, local_name)) = name.split_once(':') {
        let uri = if prefix == "xml" {
            "http://www.w3.org/XML/1998/namespace".to_string()
        } else {
            namespaces
                .get(prefix)
                .map(|uri| normalize_namespace_uri(uri))
                .unwrap_or_default()
        };
        format!("{{{uri}}}{local_name}")
    } else if is_attr {
        name.to_string()
    } else if let Some(uri) = namespaces.get("") {
        format!("{{{}}}{name}", normalize_namespace_uri(uri))
    } else {
        name.to_string()
    }
}

fn normalize_signature_line_signing_instructions_attr_key(
    element_name: &str,
    attr_name: &str,
) -> Option<String> {
    (element_name == "{urn:schemas-microsoft-com:office:office}signatureline"
        && attr_name == "{urn:schemas-microsoft-com:office:office}signinginstructions")
        .then(|| "signinginstructions".to_string())
}

fn normalize_word_indentation_left_attr_key(element_name: &str, attr_name: &str) -> Option<String> {
    (element_name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}ind"
        && attr_name == "left")
        .then(|| "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}left".to_string())
}

fn is_empty_word_table_position_alignment_attr(element_name: &str, attr_name: &str) -> bool {
    element_name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblpPr"
        && matches!(
            attr_name,
            "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblpXSpec"
                | "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblpYSpec"
        )
}

fn is_empty_word_document_protection_crypt_algorithm_sid_attr(
    element_name: &str,
    attr_name: &str,
) -> bool {
    element_name
        == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}documentProtection"
        && attr_name
            == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}cryptAlgorithmSid"
}

fn should_skip_whitespace_text(stack: &[XmlFrame]) -> bool {
    let Some(frame) = stack.last() else {
        return true;
    };

    if frame.attrs.iter().any(|(name, value)| {
        name == "{http://www.w3.org/XML/1998/namespace}space" && value == "preserve"
    }) && is_text_content_element(&frame.name)
    {
        return false;
    }

    !matches!(frame.children.last(), Some(XmlNode::Text(_)))
}

fn is_text_content_element(name: &str) -> bool {
    matches!(
        xml_local_name(name),
        "t" | "instrText" | "delText" | "fldSimple" | "templateCode" | "text" | "posText"
    )
}

fn xml_local_name(name: &str) -> &str {
    name.rsplit_once('}')
        .map(|(_, local)| local)
        .unwrap_or(name)
}

fn push_xml_node(roots: &mut Vec<XmlNode>, stack: &mut [XmlFrame], node: XmlNode) {
    if let Some(frame) = stack.last_mut() {
        frame.children.push(node);
    } else {
        roots.push(node);
    }
}

fn xml_declaration_from_event(event: &quick_xml::events::BytesDecl<'_>) -> XmlDeclaration {
    if matches!(
      event.standalone(),
      Some(Ok(value)) if value.as_ref().eq_ignore_ascii_case(b"yes")
    ) {
        XmlDeclaration::Standalone
    } else {
        XmlDeclaration::Plain
    }
}

fn is_extended_properties_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/officeDocument/2006/extended-properties}Properties"
        || name == "{http://purl.oclc.org/ooxml/officeDocument/extendedProperties}Properties"
}

fn is_core_properties_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/package/2006/metadata/core-properties}coreProperties"
}

fn is_core_property_whitespace_text_relaxed_root(name: &str) -> bool {
    matches!(
        name,
        "{http://purl.org/dc/elements/1.1/}creator"
            | "{http://purl.org/dc/elements/1.1/}description"
            | "{http://purl.org/dc/elements/1.1/}identifier"
            | "{http://purl.org/dc/elements/1.1/}subject"
            | "{http://purl.org/dc/elements/1.1/}title"
            | "{http://schemas.openxmlformats.org/package/2006/metadata/core-properties}category"
            | "{http://schemas.openxmlformats.org/package/2006/metadata/core-properties}contentStatus"
            | "{http://schemas.openxmlformats.org/package/2006/metadata/core-properties}contentType"
            | "{http://schemas.openxmlformats.org/package/2006/metadata/core-properties}lastModifiedBy"
            | "{http://schemas.openxmlformats.org/package/2006/metadata/core-properties}version"
    )
}

fn is_package_properties_entry(entry_name: &str) -> bool {
    entry_name.starts_with("docProps/") && entry_name.ends_with(".xml")
}

fn is_spreadsheet_styles_entry(entry_name: &str) -> bool {
    entry_name == "xl/styles.xml"
}

fn is_spreadsheet_worksheet_entry(entry_name: &str) -> bool {
    entry_name.starts_with("xl/worksheets/") && entry_name.ends_with(".xml")
}

fn is_spreadsheet_cell_value_type_attr(element_name: &str, attr_name: &str) -> bool {
    element_name == "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}v"
        && attr_name == "t"
}

fn is_spreadsheet_fills_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}fills"
}

fn is_spreadsheet_stylesheet_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}styleSheet"
}

fn is_word_settings_entry(entry_name: &str) -> bool {
    entry_name == "word/settings.xml"
}

fn is_word_document_entry(entry_name: &str) -> bool {
    entry_name == "word/document.xml"
}

fn is_word_undefined_styles_trial_entry(entry_name: &str) -> bool {
    entry_name == "word/trial.xml"
}

fn is_word_run_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}r"
}

fn is_word_text_leaf_mc_ignorable_attr(element_name: &str, attr_name: &str) -> bool {
    element_name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}t"
        && is_mc_ignorable_attr(attr_name)
}

fn is_word_text_leaf_xsi_nil_attr(element_name: &str, attr_name: &str) -> bool {
    element_name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}t"
        && attr_name == "{http://www.w3.org/2001/XMLSchema-instance}nil"
}

fn is_wordprocessing_drawing_position_offset(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing}posOffset"
}

fn is_word_invalid_run_container_text_root(name: &str) -> bool {
    matches!(
        name,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}p"
            | "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}r"
    )
}

fn is_word_numbering_entry(entry_name: &str) -> bool {
    entry_name == "word/numbering.xml"
}

fn is_word_font_table_entry(entry_name: &str) -> bool {
    entry_name
        .strip_prefix("word/fontTable")
        .is_some_and(|suffix| suffix.ends_with(".xml"))
}

fn is_word_styles_entry(entry_name: &str) -> bool {
    matches!(entry_name, "word/styles.xml" | "word/stylesWithEffects.xml")
}

fn is_word_header_footer_entry(entry_name: &str) -> bool {
    entry_name
        .strip_prefix("word/header")
        .is_some_and(|suffix| suffix.ends_with(".xml"))
        || entry_name
            .strip_prefix("word/footer")
            .is_some_and(|suffix| suffix.ends_with(".xml"))
}

fn is_word_settings_order_relaxed_root(name: &str) -> bool {
    matches!(
        name,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}settings"
            | "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}compat"
    )
}

fn is_word_paragraph_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pPr"
}

fn is_word_numbering_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}numPr"
}

fn is_word_numbering_abstract_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}abstractNum"
}

fn is_word_section_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}sectPr"
}

fn is_word_table_cell_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tc"
}

fn is_word_table_borders_order_relaxed_root(name: &str) -> bool {
    matches!(
        name,
        "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblBorders"
            | "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcBorders"
            | "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}pBdr"
    )
}

fn is_word_table_cell_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tcPr"
}

fn is_word_table_cell_margin_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}tblCellMar"
}

fn is_word_numbering_level_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}lvl"
}

fn is_word_numbering_instance_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}num"
}

fn is_word_run_properties_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}rPr"
}

fn is_word_font_table_font_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}font"
}

fn is_word_style_order_relaxed_root(name: &str) -> bool {
    name == "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}style"
}

const CHART_ROOT_NAME: &str = "{http://schemas.openxmlformats.org/drawingml/2006/chart}chart";
const CHART_PLOT_AREA_NAME: &str =
    "{http://schemas.openxmlformats.org/drawingml/2006/chart}plotArea";
const CHART_CAT_AXIS_NAME: &str = "{http://schemas.openxmlformats.org/drawingml/2006/chart}catAx";
const CHART_VAL_AXIS_NAME: &str = "{http://schemas.openxmlformats.org/drawingml/2006/chart}valAx";
const CHART_SER_AXIS_NAME: &str = "{http://schemas.openxmlformats.org/drawingml/2006/chart}serAx";
const CHART_DATE_AXIS_NAME: &str = "{http://schemas.openxmlformats.org/drawingml/2006/chart}dateAx";
const CHART_SHOW_DLBLS_OVER_MAX_NAME: &str =
    "{http://schemas.openxmlformats.org/drawingml/2006/chart}showDLblsOverMax";
const CHART_EXT_LST_NAME: &str = "{http://schemas.openxmlformats.org/drawingml/2006/chart}extLst";

fn is_chart_entry(entry_name: &str) -> bool {
    matches!(
        entry_name.split_once("/charts/chart"),
        Some((prefix, suffix))
            if matches!(prefix, "word" | "xl" | "ppt") && suffix.ends_with(".xml")
    )
}

fn is_chart_order_relaxed_root(name: &str) -> bool {
    name == CHART_ROOT_NAME
}

fn xml_node_name(node: &XmlNode) -> Option<&str> {
    match node {
        XmlNode::Element(element) => Some(&element.name),
        XmlNode::Declaration(_) | XmlNode::Text(_) => None,
    }
}

fn is_package_properties_sort_root(entry_name: &str, name: &str) -> bool {
    is_core_properties_root(name)
        || is_extended_properties_root(name)
        || (entry_name == "docProps/app.xml" && split_expanded_name(name).1 == "Properties")
}

fn is_all_particle_sort_root(name: &str) -> bool {
    name == "{urn:schemas-microsoft-com:office:office}shapelayout"
}

fn escape_xml_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn split_expanded_name(name: &str) -> (&str, &str) {
    if let Some(rest) = name.strip_prefix('{')
        && let Some((ns, local)) = rest.split_once('}')
    {
        (ns, local)
    } else {
        ("", name)
    }
}

fn normalize_xml_text(value: &str, options: CanonicalOptions, stack: &[XmlFrame]) -> String {
    let value = value.replace("\r\n", "\n").replace('\r', "\n");
    if options.normalize_float_lexemes
        && let Some(frame) = stack.last()
        && let Some(kind) = schema_float_kind_for_text(&frame.name)
    {
        return normalize_schema_float_lexeme(&value, kind);
    }
    value
}

fn normalize_schema_float_lexeme(value: &str, kind: SchemaFloatKind) -> String {
    match kind {
        SchemaFloatKind::Single => value
            .parse::<f32>()
            .map(render_schema_float_f32)
            .unwrap_or_else(|_| value.to_string()),
        SchemaFloatKind::Double => value
            .parse::<f64>()
            .map(render_schema_float_f64)
            .unwrap_or_else(|_| value.to_string()),
    }
}

fn normalize_ooxml_measure_lexeme(value: &str) -> Option<String> {
    for unit in ["mm", "cm", "in", "pt", "pc", "pi"] {
        if let Some(number) = value.strip_suffix(unit) {
            return normalize_decimal_lexeme(number).map(|number| format!("{number}{unit}"));
        }
    }
    None
}

fn normalize_ooxml_measure_attr_lexeme(
    element_name: &str,
    attr_name: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    let (element_ns, element_local) = split_expanded_name(element_name);
    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if attr_ns != WORDPROCESSINGML_NS
        || (!matches!(
            attr_local,
            "w" | "pos"
                | "left"
                | "right"
                | "top"
                | "bottom"
                | "start"
                | "end"
                | "hSpace"
                | "vSpace"
                | "space"
                | "header"
                | "footer"
                | "line"
                | "val"
        ) && !is_word_extra_measure_attr(element_ns, element_local, attr_local))
    {
        return None;
    }

    normalize_ooxml_measure_lexeme(value).or_else(|| {
        normalize_word_pg_mar_bare_twips_decimal_lexeme(
            element_ns,
            element_local,
            attr_local,
            value,
        )
        .or_else(|| {
            normalize_word_table_bare_twips_decimal_lexeme(
                element_ns,
                element_local,
                attr_local,
                value,
            )
        })
    })
}

fn is_word_extra_measure_attr(element_ns: &str, element_local: &str, attr_local: &str) -> bool {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    element_ns == WORDPROCESSINGML_NS
        && matches!(
            (element_local, attr_local),
            ("pgSz", "h") | ("ind", "hanging")
        )
}

fn normalize_word_pg_mar_bare_twips_decimal_lexeme(
    element_ns: &str,
    element_local: &str,
    attr_local: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    if element_ns != WORDPROCESSINGML_NS
        || element_local != "pgMar"
        || !matches!(attr_local, "top" | "right" | "bottom" | "left")
    {
        return None;
    }

    round_bare_decimal_lexeme(value)
}

fn normalize_word_table_bare_twips_decimal_lexeme(
    element_ns: &str,
    element_local: &str,
    attr_local: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    if element_ns != WORDPROCESSINGML_NS
        || attr_local != "w"
        || !matches!(element_local, "gridCol" | "tblInd")
    {
        return None;
    }

    round_bare_decimal_lexeme(value)
}

fn normalize_word_table_cell_margin_width_lexemes(children: &mut [XmlNode]) {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    const WORD_W_ATTR: &str = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}w";

    for child in children {
        let XmlNode::Element(element) = child else {
            continue;
        };
        let (element_ns, element_local) = split_expanded_name(&element.name);
        if element_ns != WORDPROCESSINGML_NS
            || !matches!(element_local, "top" | "left" | "bottom" | "right")
        {
            continue;
        }

        for (attr_name, attr_value) in &mut element.attrs {
            if attr_name == WORD_W_ATTR
                && let Some(normalized) = round_bare_decimal_lexeme(attr_value)
            {
                *attr_value = normalized;
            }
        }
    }
}

fn normalize_drawingml_percentage_attr_lexeme(
    element_name: &str,
    attr_name: &str,
    value: &str,
) -> Option<String> {
    const DRAWINGML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

    let (element_ns, element_local) = split_expanded_name(element_name);
    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if element_ns != DRAWINGML_NS
        || element_local != "srcRect"
        || !attr_ns.is_empty()
        || !matches!(attr_local, "l" | "t" | "r" | "b")
    {
        return None;
    }

    truncate_bare_decimal_lexeme(value)
}

fn truncate_bare_decimal_lexeme(value: &str) -> Option<String> {
    let (negative, digits) = value
        .strip_prefix('-')
        .map(|digits| (true, digits))
        .unwrap_or((false, value));
    let (integer, fraction) = digits.split_once('.')?;
    if integer.is_empty()
        || fraction.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let is_zero = integer == "0";

    let mut normalized = String::new();
    if negative && !is_zero {
        normalized.push('-');
    }
    normalized.push_str(integer);
    Some(normalized)
}

fn round_bare_decimal_lexeme(value: &str) -> Option<String> {
    let (negative, digits) = value
        .strip_prefix('-')
        .map(|digits| (true, digits))
        .unwrap_or((false, value));
    let (integer, fraction) = digits.split_once('.')?;
    if integer.is_empty()
        || fraction.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let mut integer_value: i128 = 0;
    for digit in integer.bytes() {
        integer_value = integer_value
            .checked_mul(10)?
            .checked_add(i128::from(digit - b'0'))?;
    }

    let mut fraction_value: i128 = 0;
    let mut fraction_scale: i128 = 1;
    for digit in fraction.bytes() {
        fraction_value = fraction_value
            .checked_mul(10)?
            .checked_add(i128::from(digit - b'0'))?;
        fraction_scale = fraction_scale.checked_mul(10)?;
    }

    let round_up = fraction_value.checked_mul(2)? >= fraction_scale;
    let rounded = integer_value.checked_add(i128::from(round_up))?;
    let rounded = if negative { -rounded } else { rounded };
    Some(rounded.to_string())
}

fn normalize_decimal_lexeme(value: &str) -> Option<String> {
    let (negative, digits) = value
        .strip_prefix('-')
        .map(|digits| (true, digits))
        .unwrap_or((false, value));
    let (integer, fraction) = digits
        .split_once('.')
        .map_or((digits, None), |(left, right)| (left, Some(right)));

    if integer.is_empty() || !integer.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = fraction
        && (fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }

    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let fraction = fraction
        .map(|value| value.trim_end_matches('0'))
        .unwrap_or("");
    let is_zero = integer == "0" && fraction.is_empty();

    let mut normalized = String::new();
    if negative && !is_zero {
        normalized.push('-');
    }
    normalized.push_str(integer);
    if !fraction.is_empty() {
        normalized.push('.');
        normalized.push_str(fraction);
    }
    Some(normalized)
}

fn normalize_doc_grid_char_space_overflow(
    element_name: &str,
    attr_name: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    let (element_ns, element_local) = split_expanded_name(element_name);
    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if element_ns != WORDPROCESSINGML_NS
        || element_local != "docGrid"
        || attr_ns != WORDPROCESSINGML_NS
        || attr_local != "charSpace"
    {
        return None;
    }

    if value.parse::<i32>().is_ok() {
        return None;
    }

    let mut chars = value.chars();
    let digits = match chars.next() {
        Some('-' | '+') => chars.as_str(),
        Some(_) => value,
        None => return None,
    };
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    Some("0".to_string())
}

fn normalize_word_numbering_level_suffix_attr(
    element_name: &str,
    attr_name: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    let (element_ns, element_local) = split_expanded_name(element_name);
    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if element_ns != WORDPROCESSINGML_NS
        || element_local != "suff"
        || attr_ns != WORDPROCESSINGML_NS
        || attr_local != "val"
        || value != "Tab"
    {
        return None;
    }

    Some("tab".to_string())
}

fn normalize_word_numbering_multilevel_type_attr(
    element_name: &str,
    attr_name: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    let (element_ns, element_local) = split_expanded_name(element_name);
    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if element_ns != WORDPROCESSINGML_NS
        || element_local != "multiLevelType"
        || attr_ns != WORDPROCESSINGML_NS
        || attr_local != "val"
        || value != "SingleLevel"
    {
        return None;
    }

    Some("singleLevel".to_string())
}

fn normalize_header_footer_odd_type(
    element_name: &str,
    attr_name: &str,
    value: &str,
) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    let (element_ns, element_local) = split_expanded_name(element_name);
    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if element_ns != WORDPROCESSINGML_NS
        || !matches!(element_local, "headerReference" | "footerReference")
        || attr_ns != WORDPROCESSINGML_NS
        || attr_local != "type"
        || value != "odd"
    {
        return None;
    }

    Some("default".to_string())
}

fn render_schema_float_f32(value: f32) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f32::INFINITY {
        "INF".to_string()
    } else if value == f32::NEG_INFINITY {
        "-INF".to_string()
    } else {
        value.to_string()
    }
}

fn render_schema_float_f64(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "INF".to_string()
    } else if value == f64::NEG_INFINITY {
        "-INF".to_string()
    } else {
        value.to_string()
    }
}

fn schema_float_kind_for_attr(element_name: &str, attr_name: &str) -> Option<SchemaFloatKind> {
    schema_float_rules()
        .attrs
        .iter()
        .find(|rule| rule.element == element_name && rule.attr == attr_name)
        .map(|rule| rule.kind)
}

fn schema_float_kind_for_text(element_name: &str) -> Option<SchemaFloatKind> {
    schema_float_rules()
        .texts
        .iter()
        .find(|rule| rule.element == element_name)
        .map(|rule| rule.kind)
}

fn schema_float_rules() -> &'static SchemaFloatRules {
    static RULES: OnceLock<SchemaFloatRules> = OnceLock::new();
    RULES.get_or_init(|| {
        serde_json::from_str(include_str!("../data/schema-float-rules.json"))
            .expect("parse schema-float-rules.json")
    })
}

fn is_mc_ignorable_attr(attr_name: &str) -> bool {
    attr_name == "{http://schemas.openxmlformats.org/markup-compatibility/2006}Ignorable"
}

fn normalize_ignorable_prefix_list(value: &str, namespaces: &BTreeMap<String, String>) -> String {
    let mut values = value
        .split_whitespace()
        .map(|prefix| {
            namespaces
                .get(prefix)
                .map(|uri| normalize_namespace_uri(uri))
                .unwrap_or_else(|| format!("prefix:{prefix}"))
        })
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values.join(" ")
}

fn normalize_relationship_type_uri(value: &str) -> String {
    const STRICT_OFFICE_REL_PREFIX: &str =
        "http://purl.oclc.org/ooxml/officeDocument/relationships/";
    const TRANSITIONAL_OFFICE_REL_PREFIX: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";

    match value {
        "http://schemas.microsoft.com/office/2006/relationships/officeDocument" => {
            return "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument".to_string();
        }
        "http://schemas.microsoft.com/office/2006/relationships/docPropsApp" => {
            return "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties".to_string();
        }
        "http://schemas.microsoft.com/package/2005/06/relationships/metadata/core-properties" => {
            return "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties".to_string();
        }
        _ => {}
    }

    value
        .strip_prefix(STRICT_OFFICE_REL_PREFIX)
        .map(|suffix| format!("{TRANSITIONAL_OFFICE_REL_PREFIX}{suffix}"))
        .unwrap_or_else(|| value.to_string())
}

fn normalize_namespace_uri(value: &str) -> String {
    match value {
        "http://schemas.microsoft.com/package/2005/06/relationships" => {
            "http://schemas.openxmlformats.org/package/2006/relationships".to_string()
        }
        "http://purl.oclc.org/ooxml/descriptions/base" => {
            "http://descriptions.openxmlformats.org/description/base".to_string()
        }
        "http://purl.oclc.org/ooxml/descriptions/full" => {
            "http://descriptions.openxmlformats.org/description/full".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/chart" => {
            "http://schemas.openxmlformats.org/drawingml/2006/chart".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/chartDrawing" => {
            "http://schemas.openxmlformats.org/drawingml/2006/chartDrawing".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/compatibility" => {
            "http://schemas.openxmlformats.org/drawingml/2006/compatibility".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/diagram" => {
            "http://schemas.openxmlformats.org/drawingml/2006/diagram".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/lockedCanvas" => {
            "http://schemas.openxmlformats.org/drawingml/2006/lockedCanvas".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/main" => {
            "http://schemas.openxmlformats.org/drawingml/2006/main".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/picture" => {
            "http://schemas.openxmlformats.org/drawingml/2006/picture".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/spreadsheetDrawing" => {
            "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing".to_string()
        }
        "http://purl.oclc.org/ooxml/drawingml/wordprocessingDrawing" => {
            "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/bibliography" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/bibliography".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/customProperties" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/customXml" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/customXml".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/customXmlDataProps" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/customXmlDataProps".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/docPropsVTypes" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/extendedProperties" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/math" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/math".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/relationships" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships".to_string()
        }
        "http://purl.oclc.org/ooxml/officeDocument/sharedTypes" => {
            "http://schemas.openxmlformats.org/officeDocument/2006/sharedTypes".to_string()
        }
        "http://purl.oclc.org/ooxml/presentationml/main" => {
            "http://schemas.openxmlformats.org/presentationml/2006/main".to_string()
        }
        "http://purl.oclc.org/ooxml/schemaLibrary/main" => {
            "http://schemas.openxmlformats.org/schemaLibrary/2006/main".to_string()
        }
        "http://purl.oclc.org/ooxml/spreadsheetml/main" => {
            "http://schemas.openxmlformats.org/spreadsheetml/2006/main".to_string()
        }
        "http://purl.oclc.org/ooxml/wordprocessingml/main" => {
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main".to_string()
        }
        _ => value.to_string(),
    }
}

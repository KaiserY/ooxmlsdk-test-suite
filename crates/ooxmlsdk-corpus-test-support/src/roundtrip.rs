use std::{
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
use ooxmlsdk::sdk::{SdkPackage, SdkPart};
use quick_xml::{Reader, escape::unescape, events::Event};
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
            let original = WordprocessingDocument::new_from_file(path).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while opening original wordprocessing package {path:?}: {err:?}");
      });
            let mut buffer = Cursor::new(Vec::new());
            original.save(&mut buffer).unwrap_or_else(|err| {
                panic!(
                    "round-trip failed for {file_name} while saving wordprocessing package: {err:?}"
                );
            });
            let roundtripped_bytes = buffer.into_inner();
            let reopened = WordprocessingDocument::new(Cursor::new(roundtripped_bytes.clone())).unwrap_or_else(|err| {
        panic!("round-trip failed for {file_name} while reopening saved wordprocessing package: {err:?}");
      });
            assert_wordprocessing_document_round_trip(&original, &reopened);
            assert_doc_sample_zip_equivalent(&original_bytes, &roundtripped_bytes, file_name);
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

pub fn assert_package_file_invalid(path: &Path, file_name: &str) {
    let kind = doc_sample_kind(file_name);

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

        if is_xml_entry(name) || is_psmdcp_entry(name) {
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

fn is_xml_entry(name: &str) -> bool {
    name == "[Content_Types].xml" || name.ends_with(".xml") || name.ends_with(".rels")
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
    normalize_doc_grid_char_space_overflow: bool,
    normalize_header_footer_odd_type: bool,
    sort_package_properties: bool,
    sort_all_particle_children: bool,
}

impl CanonicalOptions {
    fn strict() -> Self {
        Self {
            normalize_float_lexemes: false,
            normalize_measure_lexemes: false,
            normalize_doc_grid_char_space_overflow: false,
            normalize_header_footer_odd_type: false,
            sort_package_properties: false,
            sort_all_particle_children: false,
        }
    }

    fn relaxed_for_entry(entry_name: &str) -> Self {
        Self {
            normalize_float_lexemes: true,
            normalize_measure_lexemes: true,
            normalize_doc_grid_char_space_overflow: true,
            normalize_header_footer_odd_type: true,
            sort_package_properties: is_package_properties_entry(entry_name),
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
        if self.normalize_doc_grid_char_space_overflow {
            enabled.push("docGrid charSpace overflow");
        }
        if self.normalize_header_footer_odd_type {
            enabled.push("header/footer odd type");
        }
        if self.sort_package_properties {
            enabled.push("package property order");
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
                let raw = unescape(&String::from_utf8_lossy(event.as_ref()))
                    .unwrap_or_else(|err| {
                        panic!("failed to decode xml cdata for {file_name}:{entry_name}: {err}");
                    })
                    .into_owned();
                if !raw.chars().all(|ch| ch.is_whitespace()) || !should_skip_whitespace_text(&stack)
                {
                    let text = normalize_xml_text(&raw, options, &stack);
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

                if options.sort_package_properties
                    && is_package_properties_sort_root(entry_name, &element.name)
                {
                    element.children.sort_by_key(xml_node_structural_sort_key);
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

fn compare_xml_documents(original: &[XmlNode], roundtripped: &[XmlNode]) -> Vec<String> {
    struct CompareListFrame<'a> {
        parent_path: String,
        original: &'a [XmlNode],
        roundtripped: &'a [XmlNode],
        original_idx: usize,
        roundtripped_idx: usize,
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
    })];

    while let Some(task) = stack.pop() {
        match task {
            CompareTask::List(mut frame) => {
                let mut next_node = None;
                while frame.original_idx < frame.original.len()
                    && frame.roundtripped_idx < frame.roundtripped.len()
                {
                    match (
                        &frame.original[frame.original_idx],
                        &frame.roundtripped[frame.roundtripped_idx],
                    ) {
                        (XmlNode::Declaration(decl), node)
                            if !matches!(node, XmlNode::Declaration(_)) =>
                        {
                            errors.push(format!(
                                "{}: missing XML declaration {} before {}",
                                xml_child_path(
                                    &frame.parent_path,
                                    frame.original,
                                    frame.original_idx
                                ),
                                xml_declaration_summary(*decl),
                                xml_node_summary(node)
                            ));
                            frame.original_idx += 1;
                        }
                        (node, XmlNode::Declaration(decl))
                            if !matches!(node, XmlNode::Declaration(_)) =>
                        {
                            errors.push(format!(
                                "{}: extra XML declaration {} before {}",
                                xml_child_path(
                                    &frame.parent_path,
                                    frame.original,
                                    frame.original_idx
                                ),
                                xml_declaration_summary(*decl),
                                xml_node_summary(node)
                            ));
                            frame.roundtripped_idx += 1;
                        }
                        _ => {
                            let path = xml_child_path(
                                &frame.parent_path,
                                frame.original,
                                frame.original_idx,
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
                    }
                }

                if let Some(next_node) = next_node {
                    stack.push(CompareTask::List(frame));
                    stack.push(next_node);
                } else {
                    for node in &frame.original[frame.original_idx..] {
                        errors.push(format!(
                            "{}: missing child in roundtripped XML: {}",
                            frame.parent_path,
                            xml_node_summary(node)
                        ));
                    }

                    for node in &frame.roundtripped[frame.roundtripped_idx..] {
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
                (XmlNode::Declaration(left), XmlNode::Declaration(right)) => {
                    if left != right {
                        errors.push(format!(
                            "{path}: XML declaration mismatch: original {}, roundtripped {}",
                            xml_declaration_summary(*left),
                            xml_declaration_summary(*right)
                        ));
                    }
                }
                (XmlNode::Declaration(left), other) => {
                    errors.push(format!(
                        "{path}: missing XML declaration {} before {}",
                        xml_declaration_summary(*left),
                        xml_node_summary(other)
                    ));
                }
                (other, XmlNode::Declaration(right)) => {
                    errors.push(format!(
                        "{path}: extra XML declaration {} before {}",
                        xml_declaration_summary(*right),
                        xml_node_summary(other)
                    ));
                }
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
            Some(roundtripped_value) if roundtripped_value == value => {}
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

fn xml_child_path(parent_path: &str, siblings: &[XmlNode], idx: usize) -> String {
    let node = &siblings[idx];
    let ordinal = xml_child_ordinal(siblings, idx);
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

fn xml_child_ordinal(siblings: &[XmlNode], idx: usize) -> usize {
    let node = &siblings[idx];
    siblings[..=idx]
        .iter()
        .filter(|candidate| xml_nodes_share_path_name(candidate, node))
        .count()
}

fn xml_nodes_share_path_name(left: &XmlNode, right: &XmlNode) -> bool {
    match (left, right) {
        (XmlNode::Declaration(_), XmlNode::Declaration(_)) => true,
        (XmlNode::Text(_), XmlNode::Text(_)) => true,
        (XmlNode::Element(left), XmlNode::Element(right)) => left.name == right.name,
        _ => false,
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
        let decoded = reader
            .decoder()
            .decode(attr.value.as_ref())
            .unwrap_or_else(|err| {
                panic!("failed to decode xml attribute for {file_name}:{entry_name}: {err}");
            });
        let value = unescape(&decoded)
            .unwrap_or_else(|err| {
                panic!("failed to unescape xml attribute for {file_name}:{entry_name}: {err}");
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
            normalize_ooxml_measure_attr_lexeme(&expanded_key, &value).unwrap_or(value)
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

fn should_skip_whitespace_text(stack: &[XmlFrame]) -> bool {
    let Some(frame) = stack.last() else {
        return true;
    };

    if frame.attrs.iter().any(|(name, value)| {
        name == "{http://www.w3.org/XML/1998/namespace}space" && value == "preserve"
    }) {
        return false;
    }

    !matches!(frame.children.last(), Some(XmlNode::Text(_)))
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

fn is_package_properties_entry(entry_name: &str) -> bool {
    entry_name.starts_with("docProps/") && entry_name.ends_with(".xml")
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
    let unit_start = value.len().checked_sub(2)?;
    let unit = &value[unit_start..];
    if !matches!(unit, "mm" | "cm" | "in" | "pt" | "pc" | "pi") {
        return None;
    }

    normalize_decimal_lexeme(&value[..unit_start]).map(|number| format!("{number}{unit}"))
}

fn normalize_ooxml_measure_attr_lexeme(attr_name: &str, value: &str) -> Option<String> {
    const WORDPROCESSINGML_NS: &str =
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    let (attr_ns, attr_local) = split_expanded_name(attr_name);
    if attr_ns != WORDPROCESSINGML_NS
        || !matches!(
            attr_local,
            "w" | "pos" | "left" | "right" | "top" | "bottom" | "hSpace" | "vSpace"
        )
    {
        return None;
    }

    normalize_ooxml_measure_lexeme(value)
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

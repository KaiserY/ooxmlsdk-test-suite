use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::process::Command;

use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::office_pdf_campaign::{CAMPAIGN_ID, read_plan, validate_assignments};

const MAX_XML_PART_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXAMPLES_PER_FONT: usize = 8;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FontReferenceSummary {
    pub name: String,
    pub document_count: usize,
    pub occurrence_count: usize,
    pub examples: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalFontCandidate {
    pub name: String,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalFontCandidateReport {
    pub schema_version: u32,
    pub reference_file: String,
    pub candidate_font_count: usize,
    pub fonts: Vec<LocalFontCandidate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CorpusFontReferenceReport {
    pub schema_version: u32,
    pub campaign_id: &'static str,
    pub assignments_scanned: usize,
    pub xml_parts_scanned: usize,
    pub packages_with_embedded_fonts: usize,
    pub referenced_font_count: usize,
    pub fonts: Vec<FontReferenceSummary>,
    pub parse_errors: Vec<String>,
}

#[derive(Default)]
struct FontAccumulator {
    name: String,
    documents: BTreeSet<String>,
    occurrences: usize,
    examples: BTreeSet<String>,
}

pub fn scan_campaign_font_references(
    root: &Path,
    plan_path: &Path,
    output_path: &Path,
) -> Result<CorpusFontReferenceReport, String> {
    let assignments = read_plan(plan_path)?;
    validate_assignments(root, &assignments)?;
    let mut fonts = BTreeMap::<String, FontAccumulator>::new();
    let mut xml_parts_scanned = 0;
    let mut packages_with_embedded_fonts = 0;
    let mut parse_errors = Vec::new();

    for assignment in &assignments {
        let source_key = assignment.source_key();
        let source_path = assignment.source_path(root);
        let source = File::open(&source_path)
            .map_err(|error| format!("could not open {}: {error}", source_path.display()))?;
        let mut archive = ZipArchive::new(source).map_err(|error| {
            format!("could not read {} as OOXML: {error}", source_path.display())
        })?;
        let has_embedded_fonts = archive.file_names().any(is_embedded_font_part);
        if has_embedded_fonts {
            packages_with_embedded_fonts += 1;
        }
        let mut document_fonts = BTreeMap::<String, (String, usize)>::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| {
                format!("could not read ZIP entry {index} in {source_key}: {error}")
            })?;
            if !entry.name().to_ascii_lowercase().ends_with(".xml") {
                continue;
            }
            let entry_name = entry.name().to_string();
            let mut xml = Vec::new();
            entry
                .by_ref()
                .take(MAX_XML_PART_BYTES + 1)
                .read_to_end(&mut xml)
                .map_err(|error| format!("could not read {source_key}:{entry_name}: {error}"))?;
            if xml.len() as u64 > MAX_XML_PART_BYTES {
                parse_errors.push(format!(
                    "{source_key}:{entry_name}: XML part exceeds {MAX_XML_PART_BYTES} bytes"
                ));
                continue;
            }
            xml_parts_scanned += 1;
            if let Err(error) = scan_xml_font_names(&xml, &mut document_fonts) {
                parse_errors.push(format!("{source_key}:{entry_name}: {error}"));
            }
        }
        for (normalized, (name, occurrences)) in document_fonts {
            let accumulator = fonts.entry(normalized).or_default();
            if accumulator.name.is_empty() || name < accumulator.name {
                accumulator.name = name;
            }
            accumulator.documents.insert(source_key.clone());
            accumulator.occurrences += occurrences;
            if accumulator.examples.len() < MAX_EXAMPLES_PER_FONT {
                accumulator.examples.insert(source_key.clone());
            }
        }
    }

    let fonts = fonts
        .into_values()
        .map(|font| FontReferenceSummary {
            name: font.name,
            document_count: font.documents.len(),
            occurrence_count: font.occurrences,
            examples: font.examples.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    let report = CorpusFontReferenceReport {
        schema_version: 1,
        campaign_id: CAMPAIGN_ID,
        assignments_scanned: assignments.len(),
        xml_parts_scanned,
        packages_with_embedded_fonts,
        referenced_font_count: fonts.len(),
        fonts,
        parse_errors,
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("could not serialize font report: {error}"))?;
    fs::write(output_path, bytes)
        .map_err(|error| format!("could not write {}: {error}", output_path.display()))?;
    Ok(report)
}

pub fn scan_local_font_candidates(
    reference_path: &Path,
    output_path: &Path,
) -> Result<LocalFontCandidateReport, String> {
    #[derive(Deserialize)]
    struct References {
        fonts: Vec<FontReferenceSummary>,
    }

    let references: References = serde_json::from_slice(
        &fs::read(reference_path)
            .map_err(|error| format!("could not read {}: {error}", reference_path.display()))?,
    )
    .map_err(|error| format!("invalid {}: {error}", reference_path.display()))?;
    let output = Command::new("fc-list")
        .args(["--format", "%{family}\t%{file}\n"])
        .output()
        .map_err(|error| format!("could not run fc-list: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "fc-list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut available = BTreeMap::<String, BTreeSet<String>>::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((families, file)) = line.split_once('\t') else {
            continue;
        };
        if !file.starts_with("/usr/share/fonts/") && !file.starts_with("/usr/local/share/fonts/") {
            continue;
        }
        for family in families.split(',') {
            available
                .entry(normalize_font_name(family))
                .or_default()
                .insert(file.to_string());
        }
    }
    let mut fonts = Vec::new();
    for reference in references.fonts {
        let Some(files) = available.get(&normalize_font_name(&reference.name)) else {
            continue;
        };
        let files = files
            .iter()
            .map(|file| windows_path(Path::new(file)))
            .collect::<Result<Vec<_>, _>>()?;
        fonts.push(LocalFontCandidate {
            name: reference.name,
            files,
        });
    }
    let report = LocalFontCandidateReport {
        schema_version: 1,
        reference_file: reference_path.display().to_string(),
        candidate_font_count: fonts.len(),
        fonts,
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    fs::write(
        output_path,
        serde_json::to_vec_pretty(&report)
            .map_err(|error| format!("could not serialize local font candidates: {error}"))?,
    )
    .map_err(|error| format!("could not write {}: {error}", output_path.display()))?;
    Ok(report)
}

fn scan_xml_font_names(
    xml: &[u8],
    document_fonts: &mut BTreeMap<String, (String, usize)>,
) -> Result<(), String> {
    let decoded = decode_utf16_xml(xml)?;
    let xml = decoded.as_deref().unwrap_or(xml);
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut stack = Vec::<Vec<u8>>::new();
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| format!("invalid XML: {error}"))?
        {
            Event::Start(element) => {
                scan_element_font_names(
                    &reader,
                    &element,
                    stack.last().map(Vec::as_slice),
                    document_fonts,
                )?;
                stack.push(element.local_name().as_ref().to_vec());
            }
            Event::Empty(element) => {
                scan_element_font_names(
                    &reader,
                    &element,
                    stack.last().map(Vec::as_slice),
                    document_fonts,
                )?;
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(())
}

fn scan_element_font_names(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    parent_name: Option<&[u8]>,
    document_fonts: &mut BTreeMap<String, (String, usize)>,
) -> Result<(), String> {
    let element_name = element.local_name();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| {
            format!(
                "invalid XML attribute on {:?}: {error}",
                element_name.as_ref()
            )
        })?;
        let attribute_name = attribute.key.local_name();
        let is_font_attribute = attribute_name.as_ref() == b"typeface"
            || (element_name.as_ref() == b"rFonts"
                && matches!(
                    attribute_name.as_ref(),
                    b"ascii" | b"hAnsi" | b"eastAsia" | b"cs"
                ))
            || (element_name.as_ref() == b"font" && attribute_name.as_ref() == b"name")
            || (((element_name.as_ref() == b"name" && parent_name == Some(b"font"))
                || matches!(element_name.as_ref(), b"rFont" | b"mathFont"))
                && attribute_name.as_ref() == b"val");
        if !is_font_attribute {
            continue;
        }
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|error| format!("could not decode font name: {error}"))?;
        record_font_name(document_fonts, &value);
    }
    Ok(())
}

fn decode_utf16_xml(xml: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let little_endian = xml.starts_with(&[0xff, 0xfe])
        || (xml.len() >= 4 && xml[0] == b'<' && xml[1] == 0 && xml[2] == b'?' && xml[3] == 0);
    let big_endian = xml.starts_with(&[0xfe, 0xff])
        || (xml.len() >= 4 && xml[0] == 0 && xml[1] == b'<' && xml[2] == 0 && xml[3] == b'?');
    if !little_endian && !big_endian {
        return Ok(None);
    }
    let start = usize::from(xml.starts_with(&[0xff, 0xfe]) || xml.starts_with(&[0xfe, 0xff])) * 2;
    if !(xml.len() - start).is_multiple_of(2) {
        return Err("UTF-16 XML has an odd byte length".to_string());
    }
    let words = xml[start..]
        .chunks_exact(2)
        .map(|bytes| {
            if little_endian {
                u16::from_le_bytes([bytes[0], bytes[1]])
            } else {
                u16::from_be_bytes([bytes[0], bytes[1]])
            }
        })
        .collect::<Vec<_>>();
    String::from_utf16(&words)
        .map(|text| Some(text.into_bytes()))
        .map_err(|error| format!("invalid UTF-16 XML: {error}"))
}

fn record_font_name(document_fonts: &mut BTreeMap<String, (String, usize)>, value: &str) {
    if value.contains(';') {
        for name in value.split(';') {
            record_font_name(document_fonts, name);
        }
        return;
    }
    let name = value.trim().trim_start_matches('@');
    if name.is_empty()
        || name.starts_with('+')
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "default" | "none" | "minor" | "major" | "sans-serif" | "serif" | "monospace"
        )
    {
        return;
    }
    let normalized = normalize_font_name(name);
    let entry = document_fonts
        .entry(normalized)
        .or_insert_with(|| (name.to_string(), 0));
    entry.1 += 1;
    if name < entry.0.as_str() {
        entry.0 = name.to_string();
    }
}

fn normalize_font_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn is_embedded_font_part(name: &str) -> bool {
    matches!(
        Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("odttf" | "ttf" | "ttc" | "otf" | "fntdata")
    )
}

fn windows_path(path: &Path) -> Result<String, String> {
    let output = Command::new("wslpath")
        .arg("-w")
        .arg(path)
        .output()
        .map_err(|error| format!("could not run wslpath for {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "wslpath failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|path| path.trim().to_string())
        .map_err(|error| format!("wslpath returned non-UTF-8 output: {error}"))
}

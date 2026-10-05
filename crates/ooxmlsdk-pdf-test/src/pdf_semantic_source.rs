//! Independent source and PDF-text checks for references with broken extraction.
//!
//! This is only one part of the glyph-backed golden path. A candidate must
//! also paint the same embedded outlines at matching positions, and the
//! ordinary page/graphics checks still run afterward.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use ooxmlsdk::{schemas::schemas_openxmlformats_org_wordprocessingml_2006_main as w, sdk::SdkType};
use ooxmlsdk_pdf::PdfConversionDiagnostics;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use unicode_normalization::UnicodeNormalization;
use zip::ZipArchive;

use crate::pdf_extract::pdftotext_pages;

const WORD_NAMESPACE: &[u8] = b"http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT_WORD_NAMESPACE: &[u8] = b"http://purl.oclc.org/ooxml/wordprocessingml/main";

#[derive(Clone, Debug)]
pub(crate) struct SourceSemanticReport {
    pub authored_characters: usize,
    pub layout_characters: usize,
    pub candidate_authored_missing: usize,
    pub golden_authored_missing: usize,
    pub candidate_layout_missing: usize,
    pub candidate_layout_extra: usize,
    pub unexpected_layout_extra: usize,
}

impl SourceSemanticReport {
    pub(crate) fn verified(&self) -> bool {
        self.authored_characters > 0
            && self.layout_characters > 0
            && self.candidate_authored_missing == 0
            && self.golden_authored_missing > 0
            && self.candidate_layout_missing == 0
            && self.unexpected_layout_extra == 0
            && self.candidate_layout_extra <= (self.layout_characters / 1000).max(10)
    }
}

pub(crate) fn inspect_docx_semantics(
    source_path: &Path,
    candidate_pdf: &[u8],
    golden_pdf: &[u8],
    diagnostics: &PdfConversionDiagnostics,
) -> Result<SourceSemanticReport, String> {
    let authored = authored_word_text(source_path)?;
    let authored_bag = semantic_bag(&authored);
    let candidate_pages = pdftotext_pages(candidate_pdf)?;
    let golden_pages = pdftotext_pages(golden_pdf)?;
    if candidate_pages.len() != diagnostics.pages.len()
        || golden_pages.len() != diagnostics.pages.len()
    {
        return Err(format!(
            "semantic page counts disagree: layout={} candidate={} golden={}",
            diagnostics.pages.len(),
            candidate_pages.len(),
            golden_pages.len()
        ));
    }

    let mut candidate_bag = BTreeMap::new();
    let mut golden_bag = BTreeMap::new();
    let mut layout_characters = 0;
    let mut candidate_layout_missing = 0;
    let mut candidate_layout_extra = 0;
    let mut unexpected_layout_extra = 0;
    for (page_index, ((layout, candidate), golden)) in diagnostics
        .pages
        .iter()
        .zip(&candidate_pages)
        .zip(&golden_pages)
        .enumerate()
    {
        if layout.page_index != page_index {
            return Err(format!(
                "layout semantic page index {} is out of order at {page_index}",
                layout.page_index
            ));
        }
        let layout_text = layout
            .text_runs
            .iter()
            .map(|run| run.pdf_text.as_str())
            .collect::<String>();
        let layout_bag = semantic_bag(&layout_text);
        let candidate_page_bag = semantic_bag(candidate);
        let golden_page_bag = semantic_bag(golden);
        layout_characters += layout_bag.values().sum::<usize>();
        candidate_layout_missing += missing_count(&layout_bag, &candidate_page_bag);
        let extra = bag_difference(&candidate_page_bag, &layout_bag);
        candidate_layout_extra += extra.values().sum::<usize>();
        unexpected_layout_extra += extra
            .iter()
            .filter(|(ch, _)| !ch.is_ascii_digit() && **ch != '-')
            .map(|(_, count)| count)
            .sum::<usize>();
        merge_bag(&mut candidate_bag, &candidate_page_bag);
        merge_bag(&mut golden_bag, &golden_page_bag);
    }
    Ok(SourceSemanticReport {
        authored_characters: authored_bag.values().sum(),
        layout_characters,
        candidate_authored_missing: missing_count(&authored_bag, &candidate_bag),
        golden_authored_missing: missing_count(&authored_bag, &golden_bag),
        candidate_layout_missing,
        candidate_layout_extra,
        unexpected_layout_extra,
    })
}

fn is_word_text_part(name: &str, note_references: (bool, bool)) -> bool {
    match name {
        "word/document.xml" => true,
        "word/footnotes.xml" => note_references.0,
        "word/endnotes.xml" => note_references.1,
        _ => {
            (name.starts_with("word/header") || name.starts_with("word/footer"))
                && name.ends_with(".xml")
        }
    }
}

fn word_note_references(input: impl std::io::BufRead) -> Result<(bool, bool), String> {
    let mut reader = NsReader::from_reader(input);
    let mut buffer = Vec::new();
    let mut references = (false, false);
    loop {
        let (namespace, event) = reader
            .read_resolved_event_into(&mut buffer)
            .map_err(|error| error.to_string())?;
        match event {
            Event::Start(element) | Event::Empty(element)
                if matches!(namespace, ResolveResult::Bound(ns)
                    if ns.as_ref() == WORD_NAMESPACE || ns.as_ref() == STRICT_WORD_NAMESPACE) =>
            {
                match element.local_name().as_ref() {
                    b"footnoteReference" => references.0 = true,
                    b"endnoteReference" => references.1 = true,
                    _ => {}
                }
            }
            Event::Eof => return Ok(references),
            _ => {}
        }
        buffer.clear();
    }
}

fn authored_word_text(source_path: &Path) -> Result<String, String> {
    let file = File::open(source_path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    // A retained notes part can contain separator text even when the main
    // story has no notes of that kind. Office does not print that dormant
    // story. Count its source text only when a main-story reference uses it;
    // keep the complete text obligation for every enabled notes part.
    let note_references = word_note_references(BufReader::new(
        archive
            .by_name("word/document.xml")
            .map_err(|error| error.to_string())?,
    ))?;
    let styles = match archive.by_name("word/styles.xml") {
        Ok(entry) => VisibilityStyles::from_styles(
            w::Styles::from_reader(BufReader::new(entry)).map_err(|error| error.to_string())?,
        ),
        Err(zip::result::ZipError::FileNotFound) => VisibilityStyles::default(),
        Err(error) => return Err(error.to_string()),
    };
    let mut source = String::new();
    let mut found_document = false;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_owned();
        if !is_word_text_part(&name, note_references) {
            continue;
        }
        found_document |= name == "word/document.xml";
        source.push_str(&word_text_in_part(BufReader::new(entry), &styles)?);
        source.push('\n');
    }
    found_document
        .then_some(source)
        .ok_or("DOCX has no word/document.xml".to_owned())
}

#[derive(Default)]
struct VisibilityStyles {
    default_hidden: bool,
    default_paragraph: Option<String>,
    default_character: Option<String>,
    entries: BTreeMap<String, (Option<String>, bool)>,
}

impl VisibilityStyles {
    fn from_styles(styles: w::Styles) -> Self {
        let default_hidden = styles
            .doc_defaults
            .as_ref()
            .and_then(|value| value.run_properties_default.as_ref())
            .and_then(|value| value.run_properties_base_style.as_ref())
            .and_then(|value| value.vanish.as_ref())
            .is_some_and(|value| value.val.is_none_or(|value| value.as_bool()));
        let mut result = Self {
            default_hidden,
            ..Self::default()
        };
        for style in styles.style {
            let Some(id) = style.style_id else { continue };
            if style.default.is_some_and(|value| value.as_bool()) {
                match style.r#type {
                    Some(w::StyleValues::Paragraph) => result.default_paragraph = Some(id.clone()),
                    Some(w::StyleValues::Character) => result.default_character = Some(id.clone()),
                    _ => {}
                }
            }
            let toggle = style
                .style_run_properties
                .as_ref()
                .and_then(|value| value.vanish.as_ref())
                .is_some_and(|value| value.val.is_none_or(|value| value.as_bool()));
            result
                .entries
                .insert(id, (style.based_on.map(|value| value.val), toggle));
        }
        result
    }

    fn toggle(&self, id: Option<&str>) -> Result<bool, String> {
        let mut current = id;
        let mut hidden = false;
        let mut visited = std::collections::BTreeSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                return Err("cyclic Word visibility style".into());
            }
            let Some((base, toggle)) = self.entries.get(id) else {
                break;
            };
            // In a style hierarchy vanish is a toggle (§17.7.3); direct
            // run formatting is an absolute override, handled below.
            hidden ^= toggle;
            current = base.as_deref();
        }
        Ok(hidden)
    }
}

fn word_attribute(
    reader: &NsReader<impl std::io::BufRead>,
    element: &quick_xml::events::BytesStart<'_>,
    key: &[u8],
) -> Result<Option<String>, String> {
    for attr in element.attributes() {
        let attr = attr.map_err(|error| error.to_string())?;
        let (ns, name) = reader.resolver().resolve_attribute(attr.key);
        if name.as_ref() == key
            && matches!(ns, ResolveResult::Bound(ns)
            if ns.as_ref() == WORD_NAMESPACE || ns.as_ref() == STRICT_WORD_NAMESPACE)
        {
            return attr
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

fn word_text_in_part(
    input: impl std::io::BufRead,
    styles: &VisibilityStyles,
) -> Result<String, String> {
    let mut reader = NsReader::from_reader(input);
    let mut buffer = Vec::new();
    let mut text = String::new();
    let mut in_word_text = false;
    let mut parents = Vec::new();
    let mut runs = Vec::<(usize, bool, Option<bool>)>::new();
    let mut paragraphs = Vec::<bool>::new();
    loop {
        let (namespace, event) = reader
            .read_resolved_event_into(&mut buffer)
            .map_err(|error| error.to_string())?;
        let is_word = matches!(namespace, ResolveResult::Bound(ns)
            if ns.as_ref() == WORD_NAMESPACE || ns.as_ref() == STRICT_WORD_NAMESPACE);
        if let Event::Start(element) | Event::Empty(element) = &event {
            if is_word
                && element.local_name().as_ref() == b"pStyle"
                && parents.ends_with(&[Some("p"), Some("pPr")])
            {
                let id = word_attribute(&reader, element, b"val")?;
                if let Some(hidden) = paragraphs.last_mut() {
                    *hidden = styles.default_hidden ^ styles.toggle(id.as_deref())?;
                }
            }
            if is_word
                && element.local_name().as_ref() == b"rStyle"
                && parents.ends_with(&[Some("r"), Some("rPr")])
            {
                let id = word_attribute(&reader, element, b"val")?;
                if let Some((_, hidden, _)) = runs.last_mut() {
                    *hidden = paragraphs.last().copied().unwrap_or(styles.default_hidden)
                        ^ styles.toggle(id.as_deref())?;
                }
            }
            if is_word
                && element.local_name().as_ref() == b"vanish"
                && parents.ends_with(&[Some("r"), Some("rPr")])
            {
                // Direct w:rPr/w:vanish marks hidden run content, not text
                // owed by the normal fixed output (ISO 29500-1 §17.3.2.41).
                // Paragraph marks, historical rPrChange, webHidden and an
                // explicit false toggle must not hide authored source text.
                let hidden = match word_attribute(&reader, element, b"val")?.as_deref() {
                    None | Some("1" | "true" | "on") => true,
                    Some("0" | "false" | "off") => false,
                    _ => return Err("invalid Word vanish toggle".into()),
                };
                if let Some((_, _, direct_hidden)) = runs.last_mut() {
                    *direct_hidden = Some(hidden);
                }
            }
        }
        match event {
            Event::Start(element) => {
                let name = element.local_name();
                if is_word && name.as_ref() == b"p" {
                    paragraphs.push(
                        styles.default_hidden
                            ^ styles.toggle(styles.default_paragraph.as_deref())?,
                    );
                }
                if is_word && name.as_ref() == b"r" {
                    runs.push((
                        text.len(),
                        paragraphs.last().copied().unwrap_or(styles.default_hidden)
                            ^ styles.toggle(styles.default_character.as_deref())?,
                        None,
                    ));
                }
                parents.push(match (is_word, name.as_ref()) {
                    (true, b"p") => Some("p"),
                    (true, b"pPr") => Some("pPr"),
                    (true, b"r") => Some("r"),
                    (true, b"rPr") => Some("rPr"),
                    _ => None,
                });
                in_word_text = is_word && name.as_ref() == b"t";
            }
            Event::End(element) => {
                in_word_text = false;
                parents.pop();
                if is_word
                    && element.local_name().as_ref() == b"r"
                    && let Some((start, inherited, direct)) = runs.pop()
                    && direct.unwrap_or(inherited)
                {
                    text.truncate(start);
                }
                if is_word && element.local_name().as_ref() == b"p" {
                    paragraphs.pop();
                }
            }
            Event::Text(value) if in_word_text => {
                let decoded = value.xml10_content().map_err(|error| error.to_string())?;
                let unescaped =
                    quick_xml::escape::unescape(&decoded).map_err(|error| error.to_string())?;
                text.push_str(&unescaped);
            }
            Event::CData(value) if in_word_text => {
                text.push_str(&value.decode().map_err(|error| error.to_string())?);
            }
            Event::GeneralRef(value) if in_word_text => {
                if let Some(ch) = value
                    .resolve_char_ref()
                    .map_err(|error| error.to_string())?
                {
                    text.push(ch);
                } else {
                    let name = value.decode().map_err(|error| error.to_string())?;
                    let expanded = quick_xml::escape::resolve_predefined_entity(&name)
                        .ok_or_else(|| format!("unsupported OOXML entity &{name};"))?;
                    text.push_str(expanded);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(text)
}

fn is_transport_direction_control(ch: char) -> bool {
    matches!(
        ch,
        '\u{061c}' | '\u{200b}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}'
    )
}

fn semantic_bag(text: &str) -> BTreeMap<char, usize> {
    let mut collapsed = String::with_capacity(text.len());
    let mut previous = None;
    for ch in text.chars() {
        // Two consecutive ZWNJs have the same joining effect as one. Poppler
        // and MuPDF each extract one for that authored sequence.
        if ch != '\u{200c}' || previous != Some(ch) {
            collapsed.push(ch);
        }
        previous = Some(ch);
    }
    let mut bag = BTreeMap::new();
    for ch in collapsed.nfd() {
        if !ch.is_whitespace() && !is_transport_direction_control(ch) {
            *bag.entry(ch).or_insert(0) += 1;
        }
    }
    bag
}

fn merge_bag(into: &mut BTreeMap<char, usize>, other: &BTreeMap<char, usize>) {
    for (&ch, &count) in other {
        *into.entry(ch).or_insert(0) += count;
    }
}

fn bag_difference(
    left: &BTreeMap<char, usize>,
    right: &BTreeMap<char, usize>,
) -> BTreeMap<char, usize> {
    left.iter()
        .filter_map(|(&ch, &count)| {
            let excess = count.saturating_sub(*right.get(&ch).unwrap_or(&0));
            (excess > 0).then_some((ch, excess))
        })
        .collect()
}

fn missing_count(source: &BTreeMap<char, usize>, observed: &BTreeMap<char, usize>) -> usize {
    bag_difference(source, observed).values().sum()
}

#[cfg(test)]
mod tests {
    use super::{
        SourceSemanticReport, VisibilityStyles, is_word_text_part, semantic_bag,
        word_note_references, word_text_in_part,
    };
    use ooxmlsdk::{
        schemas::schemas_openxmlformats_org_wordprocessingml_2006_main as w, sdk::SdkType,
    };

    #[test]
    fn word_text_parser_keeps_only_wordprocessingml_text() {
        let xml = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:p><w:r><w:t>A&amp;B</w:t></w:r><m:t>math</m:t><w:instrText>field</w:instrText></w:p></w:document>"#;
        assert_eq!(
            word_text_in_part(xml.as_slice(), &Default::default()).unwrap(),
            "A&B"
        );
    }

    #[test]
    fn source_notes_require_main_story_references_of_the_same_kind() {
        // Native Word prints neither a dormant note body nor its custom
        // separator. Adding the corresponding reference enables both.
        for namespace in [super::WORD_NAMESPACE, super::STRICT_WORD_NAMESPACE] {
            for (references, expected) in [
                ("", (false, false)),
                ("<w:footnoteReference w:id=\"1\"/>", (true, false)),
                (
                    "<w:endnoteReference w:id=\"1\"></w:endnoteReference>",
                    (false, true),
                ),
                (
                    "<w:footnoteReference w:id=\"1\"/><w:endnoteReference w:id=\"2\"/>",
                    (true, true),
                ),
                (
                    "<x:footnoteReference/><x:endnoteReference/><w:footnoteRef/><w:endnoteRef/>",
                    (false, false),
                ),
            ] {
                let xml = format!(
                    "<w:document xmlns:w=\"{}\" xmlns:x=\"urn:other\"><w:body><w:p><w:r>{references}</w:r></w:p></w:body></w:document>",
                    std::str::from_utf8(namespace).unwrap()
                );
                let actual = word_note_references(xml.as_bytes()).unwrap();
                assert_eq!(actual, expected);
                assert_eq!(is_word_text_part("word/footnotes.xml", actual), expected.0);
                assert_eq!(is_word_text_part("word/endnotes.xml", actual), expected.1);
                for name in ["word/document.xml", "word/header1.xml", "word/footer1.xml"] {
                    assert!(is_word_text_part(name, actual));
                }
                assert!(!is_word_text_part("word/styles.xml", actual));
            }
        }
    }

    #[test]
    fn source_proof_excludes_only_explicitly_hidden_runs() {
        let xml = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:x="urn:other"><w:p>
<w:pPr><w:rPr><w:vanish/></w:rPr></w:pPr>
<w:r><w:rPr><w:vanish/></w:rPr><w:t>hidden</w:t></w:r>
<w:r><w:rPr><w:vanish w:val="true"/></w:rPr><w:t>also hidden</w:t></w:r>
<w:r><w:rPr><w:vanish w:val="off"/></w:rPr><w:t>A</w:t></w:r>
<w:r><w:rPr><w:webHidden/></w:rPr><w:t>B</w:t></w:r>
<w:r><w:rPr><w:rPrChange><w:rPr><w:vanish/></w:rPr></w:rPrChange></w:rPr><w:t>C</w:t></w:r>
<w:r><w:rPr><x:vanish/></w:rPr><w:t>D</w:t></w:r>
<w:r><w:t>E</w:t></w:r></w:p></w:document>"#;
        assert_eq!(
            word_text_in_part(xml.as_slice(), &Default::default()).unwrap(),
            "ABCDE"
        );
        let strict = String::from_utf8(xml.to_vec()).unwrap().replace(
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
            "http://purl.oclc.org/ooxml/wordprocessingml/main",
        );
        assert_eq!(
            word_text_in_part(strict.as_bytes(), &Default::default()).unwrap(),
            "ABCDE"
        );
    }

    #[test]
    fn canonical_text_bag_retains_joiners_but_collapses_duplicate_zwnj() {
        assert_eq!(
            semantic_bag("آب\u{200c}\u{200c}ها"),
            semantic_bag("ا\u{0653}ب\u{200c}ها")
        );
        assert_ne!(semantic_bag("می\u{200c}شود"), semantic_bag("میشود"));
    }

    #[test]
    fn hidden_style_inheritance_preserves_toggles_and_direct_overrides() {
        let styles = w::Styles::from_bytes(br#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style w:type="paragraph" w:styleId="hidden"><w:rPr><w:vanish/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="child"><w:basedOn w:val="hidden"/></w:style>
<w:style w:type="paragraph" w:styleId="visible"><w:basedOn w:val="child"/><w:rPr><w:vanish/></w:rPr></w:style>
<w:style w:type="character" w:styleId="toggle"><w:rPr><w:vanish/></w:rPr></w:style>
</w:styles>"#).unwrap();
        let styles = VisibilityStyles::from_styles(styles);
        let xml = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:p><w:pPr><w:pStyle w:val="child"/></w:pPr>
<w:r><w:t>hidden</w:t></w:r>
<w:r><w:rPr><w:vanish w:val="0"/></w:rPr><w:t>A</w:t></w:r>
<w:r><w:rPr><w:rStyle w:val="toggle"/></w:rPr><w:t>B</w:t></w:r>
</w:p>
<w:p><w:pPr><w:pStyle w:val="visible"/></w:pPr><w:r><w:t>C</w:t></w:r></w:p>
<w:p><w:r><w:t>D</w:t></w:r><w:r><w:rPr><w:rStyle w:val="toggle"/></w:rPr><w:t>hidden</w:t></w:r></w:p>
</w:document>"#;
        assert_eq!(word_text_in_part(xml.as_slice(), &styles).unwrap(), "ABCD");
    }

    #[test]
    fn source_proof_requires_candidate_and_layout_coverage() {
        let report = SourceSemanticReport {
            authored_characters: 1000,
            layout_characters: 1000,
            candidate_authored_missing: 0,
            golden_authored_missing: 1,
            candidate_layout_missing: 0,
            candidate_layout_extra: 1,
            unexpected_layout_extra: 0,
        };
        assert!(report.verified());
        assert!(
            !SourceSemanticReport {
                candidate_authored_missing: 1,
                ..report.clone()
            }
            .verified()
        );
        assert!(
            !SourceSemanticReport {
                candidate_layout_missing: 1,
                ..report.clone()
            }
            .verified()
        );
        assert!(
            !SourceSemanticReport {
                golden_authored_missing: 0,
                ..report
            }
            .verified()
        );
    }
}

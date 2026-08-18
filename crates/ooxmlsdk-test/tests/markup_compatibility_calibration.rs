use std::io::{Cursor, Read};

#[cfg(feature = "mce")]
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart::{
    ChartSpace, ChartSpaceChoice,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006::{
    AlternateContent, AlternateContentChoice, Choice, Fallback,
};
#[cfg(feature = "mce")]
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Worksheet;
#[cfg(feature = "mce")]
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::ParagraphProperties;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::{
    BodyChoice, Document, Paragraph,
};
#[cfg(feature = "mce")]
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::{
    ParagraphChoice, RunChoice,
};
#[cfg(feature = "mce")]
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings, SdkType,
};
use ooxmlsdk_test::{assert_stable_roundtrip, assert_xml_namespace_prefixes, fixtures};

fn doc_sample_part(file_name: &str, part_name: &str) -> String {
    let bytes = std::fs::read(fixtures::doc_sample_path(file_name)).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut part = archive.by_name(part_name).unwrap();
    let mut xml = String::new();
    part.read_to_string(&mut xml).unwrap();
    xml
}

fn first_paragraph(document: &Document) -> &Paragraph {
    document
        .body
        .as_ref()
        .expect("expected body")
        .body_choice
        .iter()
        .find_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph.as_ref()),
            _ => None,
        })
        .expect("expected paragraph")
}

fn first_mc_choice(alternate_content: &AlternateContent) -> &Choice {
    alternate_content
        .alternate_content_choice
        .iter()
        .find_map(|choice| match choice {
            AlternateContentChoice::Choice(choice) => Some(choice.as_ref()),
            _ => None,
        })
        .expect("expected mc:Choice")
}

#[cfg(feature = "mce")]
fn microsoft_365_mce_settings() -> MarkupCompatibilityProcessSettings {
    MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Microsoft365,
    }
}

#[cfg(feature = "mce")]
#[test]
fn paragraph_alternate_content_is_replaced_with_typed_run_choice() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p><mc:AlternateContent><mc:Choice Requires="w14"><w:r><w:t>choice</w:t></w:r></mc:Choice><mc:Fallback><w:r><w:t>fallback</w:t></w:r></mc:Fallback></mc:AlternateContent></w:p></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&microsoft_365_mce_settings()).unwrap();

    let paragraph = first_paragraph(&document);
    let serialized = document.to_xml().unwrap();
    assert!(
        paragraph.paragraph_choice.iter().any(
            |choice| matches!(choice, ParagraphChoice::WRun(run) if matches!(
                run.run_choice.as_slice(),
                [RunChoice::Text(_)]
            ))
        ),
        "{serialized}"
    );
    assert!(!serialized.contains("AlternateContent"), "{serialized}");
    assert!(serialized.contains(">choice<"), "{serialized}");
    assert!(!serialized.contains(">fallback<"), "{serialized}");
}

#[cfg(feature = "mce")]
#[test]
fn run_alternate_content_is_replaced_with_typed_drawing_choice() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><w:body><w:p><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing/></mc:Choice><mc:Fallback><w:pict/></mc:Fallback></mc:AlternateContent></w:r></w:p></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&microsoft_365_mce_settings()).unwrap();

    let paragraph = first_paragraph(&document);
    let run = paragraph
        .paragraph_choice
        .iter()
        .find_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run.as_ref()),
            _ => None,
        })
        .expect("selected typed run");
    assert!(matches!(run.run_choice.as_slice(), [RunChoice::Drawing(_)]));
}

#[cfg(feature = "mce")]
#[test]
fn chart_space_alternate_content_is_replaced_with_typed_style_choice() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:c14="http://schemas.microsoft.com/office/drawing/2007/8/2/chart" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:AlternateContent><mc:Choice Requires="c14"><c14:style val="10"/></mc:Choice><mc:Fallback><c:style val="2"/></mc:Fallback></mc:AlternateContent><c:chart><c:plotArea><c:layout/></c:plotArea></c:chart></c:chartSpace>"#;
    let mut chart_space = xml.parse::<ChartSpace>().unwrap();

    chart_space
        .process_mce(&microsoft_365_mce_settings())
        .unwrap();

    assert!(matches!(
        chart_space.chart_space_choice,
        Some(ChartSpaceChoice::C14Style(_))
    ));
}

#[cfg(feature = "mce")]
#[test]
fn worksheet_alternate_content_is_dispatched_by_selected_child_qname() {
    let xml = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><sheetData/><drawing r:id="rId1" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/><mc:AlternateContent><mc:Choice Requires="x14"><controls/></mc:Choice></mc:AlternateContent></worksheet>"#;
    let mut worksheet = xml.parse::<Worksheet>().unwrap();

    worksheet
        .process_mce(&microsoft_365_mce_settings())
        .unwrap();

    assert!(worksheet.controls.is_some());
    assert!(worksheet.alternate_content_0.is_empty());
    assert!(worksheet.alternate_content_1.is_empty());
    let serialized = worksheet.to_xml().unwrap();
    assert!(!serialized.contains("AlternateContent"), "{serialized}");
    assert!(serialized.contains("<controls"), "{serialized}");
}

#[test]
fn mcsupport_load_attribute_test() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadAttributeTest
    let xml = doc_sample_part("mcdoc.docx", "word/document.xml");

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(&xml);

    assert_xml_namespace_prefixes(document.mc_ignorable.as_deref(), &["w14", "wp14"]);
    assert_xml_namespace_prefixes(reparsed.mc_ignorable.as_deref(), &["w14", "wp14"]);
    assert!(serialized.contains(r#"mc:Ignorable="w14 wp14""#));
    assert!(!serialized.contains(r#"mc:PreserveAttributes="w14:myattr""#));
    assert!(!serialized.contains(r#"mc:PreserveAttributes="w14:*""#));
}

#[cfg(feature = "mce")]
#[test]
fn mcsupport_load_preserve_attr() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadPreserveAttr
    // Attribute names and values come from mcdoc.docx. The original fixture uses
    // synthetic w14 attributes on types that no longer have an open attr bag.
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" mc:PreserveAttributes="w14:editId"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();
    let paragraph = first_paragraph(&document);

    assert!(paragraph.w14_edit_id.is_some());
    assert!(paragraph.paragraph_id.is_none());
    assert!(paragraph.text_id.is_none());
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_preserve_attributes_namespace_wildcard_on_static_attributes() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" mc:PreserveAttributes="w14:*"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();
    let paragraph = first_paragraph(&document);

    assert!(paragraph.paragraph_id.is_some());
    assert!(paragraph.w14_edit_id.is_some());
    assert!(paragraph.text_id.is_some());
}

#[cfg(feature = "mce")]
#[test]
fn mcsupport_load_ignorable() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadIgnorable
    let xml = doc_sample_part("mcdoc.docx", "word/document.xml");
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessLoadedPartsOnly,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();
    let paragraph = first_paragraph(&document);

    assert!(paragraph.w14_edit_id.is_none());
    assert!(paragraph.paragraph_id.is_none());
    assert!(paragraph.text_id.is_none());
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_keeps_supported_static_versioned_namespace_attributes() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14"><w:body><w:p w14:noSpellErr="1" w14:editId="12345678"/></w:body></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2010,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();
    let paragraph = first_paragraph(&document);

    assert!(paragraph.no_spell_error.is_some());
    assert!(paragraph.w14_edit_id.is_some());
}

#[test]
#[cfg(feature = "mce")]
fn markup_compatibility_ignore_whitespaces_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Ignore_Whitespaces_FullMode
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="  &#x9;&#xA;&#xD; "><w:body><w:p w14:editId="5B733B31"/></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&settings).unwrap();

    assert!(first_paragraph(&document).w14_edit_id.is_some());

    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:wp14="http://schemas.microsoft.com/office/word/2008/9/16/wordprocessingDrawing" mc:Ignorable="w14&#x9;wp14"><w:body><w:p w14:editId="5B733B31"/></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&settings).unwrap();

    assert!(first_paragraph(&document).w14_edit_id.is_none());
}

#[test]
fn markup_compatibility_ignored_known_attribute_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Ignored_KnownAttribute_FullMode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14"><w:body><w:p w14:editId="5B733B31"/></w:body></w:document>"#;

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(xml);

    assert!(first_paragraph(&document).w14_edit_id.is_some());
    assert!(first_paragraph(&reparsed).w14_edit_id.is_some());
    assert!(serialized.contains(r#"mc:Ignorable="w14""#));
    assert!(serialized.contains(r#"w14:editId="5B733B31""#));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_ignored_known_attribute_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Ignored_KnownAttribute_O12Mode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();

    let paragraph = first_paragraph(&document);
    assert!(paragraph.paragraph_id.is_none());
    assert!(paragraph.w14_edit_id.is_none());
    assert!(paragraph.text_id.is_none());
}

#[test]
fn markup_compatibility_process_content_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   ProcessContent_Ignored_UnknownElement_FullMode
    let xml = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1" mc:Ignorable="uns1" mc:ProcessContent="uns1:e1uk1"><mc:Choice Requires="uns1"><uns1:e1uk1><uns1:child/></uns1:e1uk1></mc:Choice><mc:Fallback/></mc:AlternateContent>"#;

    let (alternate_content, serialized, _) = assert_stable_roundtrip::<AlternateContent>(xml);

    assert_eq!(
        alternate_content.alternate_content_choice.len(),
        2,
        "AlternateContent retains choice and fallback branches"
    );
    assert_eq!(
        alternate_content.mc_process_content.as_deref(),
        Some(b"uns1:e1uk1".as_slice())
    );
    assert!(serialized.contains(r#"mc:Ignorable="uns1""#));
    assert!(serialized.contains(r#"mc:ProcessContent="uns1:e1uk1""#));
    assert!(serialized.contains("<uns1:e1uk1>"));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_process_content_ignored_known_element_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   ProcessContent_Ignored_KnownElement_O12Mode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="w" mc:ProcessContent="w:keepNext"><w:keepNext/></w:pPr>"#;

    let properties = xml.parse::<ParagraphProperties>().unwrap();

    assert!(
        properties.keep_next.is_some(),
        "ProcessContent keeps children of an ignored known element upstream"
    );
}

#[test]
fn markup_compatibility_process_content_xml_space_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   ProcessContent_xmlSpace_FullMode
    let xml = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1" xmlns:xml="http://www.w3.org/XML/1998/namespace" mc:Ignorable="uns1" mc:ProcessContent="xml:space"><mc:Choice Requires="uns1"><uns1:e1uk1 xml:space="preserve"> spaced </uns1:e1uk1></mc:Choice><mc:Fallback/></mc:AlternateContent>"#;

    let (_alternate_content, serialized, _) = assert_stable_roundtrip::<AlternateContent>(xml);

    assert!(serialized.contains(r#"mc:ProcessContent="xml:space""#));
    assert!(serialized.contains(r#"xml:space="preserve""#));
}

#[test]
fn markup_compatibility_preserve_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_FullMode
    let xml = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><mc:Choice Requires="w14" mc:Ignorable="w14" mc:PreserveElements="wps:wsp" mc:PreserveAttributes="w14:editId"/><mc:Fallback/></mc:AlternateContent>"#;

    let (alternate_content, serialized, reparsed) =
        assert_stable_roundtrip::<AlternateContent>(xml);

    assert_xml_namespace_prefixes(
        Some(first_mc_choice(&alternate_content).requires.as_slice()),
        &["w14"],
    );
    assert_xml_namespace_prefixes(
        first_mc_choice(&alternate_content).mc_ignorable.as_deref(),
        &["w14"],
    );
    assert_eq!(
        first_mc_choice(&alternate_content)
            .mc_preserve_elements
            .as_deref(),
        Some(b"wps:wsp".as_slice())
    );
    assert_eq!(
        first_mc_choice(&alternate_content)
            .mc_preserve_attributes
            .as_deref(),
        Some(b"w14:editId".as_slice())
    );
    assert_eq!(
        first_mc_choice(&reparsed).mc_preserve_elements.as_deref(),
        Some(b"wps:wsp".as_slice())
    );
    assert_eq!(
        first_mc_choice(&reparsed).mc_preserve_attributes.as_deref(),
        Some(b"w14:editId".as_slice())
    );
    assert_xml_namespace_prefixes(
        Some(first_mc_choice(&reparsed).requires.as_slice()),
        &["w14"],
    );
    assert_xml_namespace_prefixes(first_mc_choice(&reparsed).mc_ignorable.as_deref(), &["w14"]);
    assert!(serialized.contains(r#"Requires="w14""#));
    assert!(serialized.contains(r#"mc:Ignorable="w14""#));
    assert!(serialized.contains(r#"mc:PreserveElements="wps:wsp""#));
    assert!(serialized.contains(r#"mc:PreserveAttributes="w14:editId""#));
}

#[test]
fn markup_compatibility_fallback_static_mce_attributes_roundtrip() {
    let xml = r#"<mc:Fallback xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="w14&#x9;w15" mc:ProcessContent="w:p" mc:MustUnderstand="w14"/>"#;

    let (fallback, serialized, reparsed) = assert_stable_roundtrip::<Fallback>(xml);

    for value in [&fallback, &reparsed] {
        assert_xml_namespace_prefixes(value.mc_ignorable.as_deref(), &["w14", "w15"]);
        assert_eq!(value.mc_process_content.as_deref(), Some(b"w:p".as_slice()));
        assert_xml_namespace_prefixes(value.mc_must_understand.as_deref(), &["w14"]);
    }
    assert!(serialized.contains(r#"mc:Ignorable="w14 w15""#));
    assert!(serialized.contains(r#"mc:ProcessContent="w:p""#));
    assert!(serialized.contains(r#"mc:MustUnderstand="w14""#));
}

#[test]
fn markup_compatibility_choice_requires_is_required() {
    let xml =
        r#"<mc:Choice xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"/>"#;

    assert!(xml.parse::<Choice>().is_err());
}

#[test]
fn markup_compatibility_preserve_wildcard_with_supported_static_attributes_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_Wildcard_FullMode
    // Rust currently has no open unknown-attribute bag, so this calibration
    // covers the wildcard coexisting with supported static attributes.
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" mc:PreserveAttributes="*"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(xml);

    assert_eq!(
        document.mc_preserve_attributes.as_deref(),
        Some(b"*".as_slice())
    );
    assert!(first_paragraph(&document).w14_edit_id.is_some());
    assert!(first_paragraph(&reparsed).w14_edit_id.is_some());
    assert!(serialized.contains(r#"mc:PreserveAttributes="*""#));
    assert!(serialized.contains(r#"w14:editId="5B733B31""#));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_preserve_ignored_unknown_element_wildcard_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_Wildcard_O12Mode
    // This static-field surrogate uses the standardized namespace; the upstream
    // unknown-attribute-bag case remains skipped there and is outside this API.
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" mc:PreserveAttributes="*"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();
    let paragraph = first_paragraph(&document);

    assert!(paragraph.paragraph_id.is_some());
    assert!(paragraph.w14_edit_id.is_some());
    assert!(paragraph.text_id.is_some());
}

#[cfg(feature = "mce")]
fn process_word_document_for_office_2007(xml: &str) -> String {
    process_word_document(xml, FileFormatVersion::Office2007)
}

#[cfg(feature = "mce")]
fn process_word_document(xml: &str, target_file_format_version: FileFormatVersion) -> String {
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version,
    };
    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();
    document.to_xml().unwrap()
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_preserve_elements_exact_and_wildcard_on_static_children() {
    let document = |preserve: &str| {
        format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><mc:AlternateContent><mc:Choice Requires="w" mc:Ignorable="w14"{preserve}><w:p><w:r><w:rPr><w14:glow/><w14:reflection/></w:rPr><w:t>kept text</w:t></w:r></w:p></mc:Choice></mc:AlternateContent></w:body></w:document>"#
        )
    };

    let dropped = process_word_document_for_office_2007(&document(""));
    assert!(!dropped.contains("<w14:glow"));
    assert!(!dropped.contains("<w14:reflection"));
    assert!(dropped.contains("kept text"));

    let exact =
        process_word_document_for_office_2007(&document(r#" mc:PreserveElements="w14:glow""#));
    assert!(exact.contains("<w14:glow"));
    assert!(!exact.contains("<w14:reflection"));

    let wildcard =
        process_word_document_for_office_2007(&document(r#" mc:PreserveElements="w14:*""#));
    assert!(wildcard.contains("<w14:glow"));
    assert!(wildcard.contains("<w14:reflection"));

    let global_wildcard =
        process_word_document_for_office_2007(&document(r#" mc:PreserveElements="*""#));
    assert!(global_wildcard.contains("<w14:glow"));
    assert!(global_wildcard.contains("<w14:reflection"));

    let supported = process_word_document(&document(""), FileFormatVersion::Office2010);
    assert!(supported.contains("<w14:glow"));
    assert!(supported.contains("<w14:reflection"));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_preserve_elements_targets_one_static_choice_variant() {
    let document = |preserve: &str| {
        format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><mc:AlternateContent><mc:Choice Requires="w" mc:Ignorable="w14"{preserve}><w:p><w:r><w:t>before</w:t></w:r><w14:conflictIns w:author="A" w:id="1"><w:r><w:t>inside</w:t></w:r></w14:conflictIns><w:r><w:t>after</w:t></w:r></w:p></mc:Choice></mc:AlternateContent></w:body></w:document>"#
        )
    };

    let dropped = process_word_document_for_office_2007(&document(""));
    assert!(!dropped.contains("<w14:conflictIns"));
    assert!(!dropped.contains(">inside<"));
    assert!(dropped.contains(">before<"));
    assert!(dropped.contains(">after<"));

    let preserved = process_word_document_for_office_2007(&document(
        r#" mc:PreserveElements="w14:conflictIns""#,
    ));
    assert!(preserved.contains("<w14:conflictIns"));
    assert!(preserved.contains(">inside<"));
    assert!(preserved.contains(">before<"));
    assert!(preserved.contains(">after<"));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_process_content_promotes_static_choice_children() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><mc:AlternateContent><mc:Choice Requires="w" mc:Ignorable="w14" mc:ProcessContent="w14:conflictIns"><w:p><w:r><w:t>before</w:t></w:r><w14:conflictIns w:author="A" w:id="1"><w:r><w:t>inside</w:t></w:r></w14:conflictIns><w:r><w:t>after</w:t></w:r></w:p></mc:Choice></mc:AlternateContent></w:body></w:document>"#;

    let processed = process_word_document_for_office_2007(xml);

    assert!(!processed.contains("<w14:conflictIns"));
    assert!(processed.contains(">before<"));
    assert!(processed.contains(">inside<"));
    assert!(processed.contains(">after<"));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_preserve_elements_scope_does_not_leak_from_selected_choice() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14"><w:body><mc:AlternateContent><mc:Choice Requires="w" mc:PreserveElements="w14:glow"><w:p><w:r><w:rPr><w14:glow/></w:rPr><w:t>inside choice</w:t></w:r></w:p></mc:Choice></mc:AlternateContent><w:p><w:r><w:rPr><w14:glow/></w:rPr><w:t>after choice</w:t></w:r></w:p></w:body></w:document>"#;

    let processed = process_word_document_for_office_2007(xml);

    assert_eq!(processed.matches("<w14:glow").count(), 1);
    assert!(processed.contains(">inside choice<"));
    assert!(processed.contains(">after choice<"));
}

#[test]
fn markup_compatibility_must_understand_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Ignored_UnknownElement_FullMode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14" mc:MustUnderstand="w14"><w:body/></w:document>"#;

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(xml);

    assert_xml_namespace_prefixes(document.mc_must_understand.as_deref(), &["w14"]);
    assert_xml_namespace_prefixes(reparsed.mc_must_understand.as_deref(), &["w14"]);
    assert!(serialized.contains(r#"mc:MustUnderstand="w14""#));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_must_understand_ignored_unknown_element_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Ignored_UnknownElement_O12Mode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14" mc:MustUnderstand="w14"><w:body/></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    let processed = document.process_mce(&settings);

    assert!(processed.is_err());
}

#[test]
fn markup_compatibility_must_understand_unselected_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Unselected_FullMode
    let xml = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml"><mc:Choice Requires="w14" mc:MustUnderstand="w14"/></mc:AlternateContent>"#;

    let (alternate_content, serialized, _) = assert_stable_roundtrip::<AlternateContent>(xml);

    assert_eq!(alternate_content.alternate_content_choice.len(), 1);
    assert_xml_namespace_prefixes(
        Some(first_mc_choice(&alternate_content).requires.as_slice()),
        &["w14"],
    );
    assert_xml_namespace_prefixes(
        first_mc_choice(&alternate_content)
            .mc_must_understand
            .as_deref(),
        &["w14"],
    );
    assert!(serialized.contains(r#"mc:MustUnderstand="w14""#));
}

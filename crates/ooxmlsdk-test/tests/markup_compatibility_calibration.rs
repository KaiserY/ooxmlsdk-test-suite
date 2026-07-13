use std::io::{Cursor, Read};

use ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006::{
    AlternateContent, AlternateContentChoice, Choice, Fallback,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::SharedStringTable;
#[cfg(feature = "mce")]
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::ParagraphProperties;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::{
    BodyChoice, Document, Paragraph,
};
#[cfg(feature = "mce")]
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings, SdkMce,
};
use ooxmlsdk_test::{assert_stable_roundtrip, fixtures};

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

#[test]
fn mcsupport_load_attribute_test() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadAttributeTest
    let xml = doc_sample_part("mcdoc.docx", "word/document.xml");

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(&xml);

    assert_eq!(
        document.mc_ignorable.as_deref(),
        Some(b"w14 wp14".as_slice())
    );
    assert_eq!(
        reparsed.mc_ignorable.as_deref(),
        Some(b"w14 wp14".as_slice())
    );
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
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14" mc:PreserveAttributes="w14:editId"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;
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
fn mcsupport_load_process_content() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadProcessContent
    let xml = doc_sample_part("MCExecl.xlsx", "xl/sharedStrings.xml");

    let (table, serialized, _) = assert_stable_roundtrip::<SharedStringTable>(&xml);
    let item = table
        .shared_string_item
        .first()
        .expect("expected shared string item");
    let placeholder_xml = item
        .xml_other_children
        .iter()
        .find_map(|(_, xml)| {
            std::str::from_utf8(xml)
                .ok()
                .filter(|xml| xml.contains("<w14:placeholder"))
        })
        .expect("expected placeholder");

    assert!(!serialized.contains(r#"mc:Ignorable="w14""#));
    assert!(!serialized.contains(r#"w14:attr="value""#));
    assert!(placeholder_xml.contains(r#"mc:ProcessContent="w14:placeholder""#));
    assert!(placeholder_xml.contains(r#"mc:PreserveAttributes="w14:a w14:b""#));
    assert!(placeholder_xml.contains(r#"w14:a="a""#));
    assert!(placeholder_xml.contains(r#"w14:b="b""#));
    assert!(serialized.contains(r#"mc:ProcessContent="w14:placeholder""#));
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
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="  &#x9;&#xA;&#xD; "><w:body><w:p w14:editId="5B733B31"/></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&settings).unwrap();

    assert!(first_paragraph(&document).w14_edit_id.is_some());

    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" xmlns:wp14="http://schemas.microsoft.com/office/word/2008/9/16/wordprocessingDrawing" mc:Ignorable="w14&#x9;wp14"><w:body><w:p w14:editId="5B733B31"/></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&settings).unwrap();

    assert!(first_paragraph(&document).w14_edit_id.is_none());
}

#[test]
fn markup_compatibility_ignored_known_attribute_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Ignored_KnownAttribute_FullMode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14"><w:body><w:p w14:editId="5B733B31"/></w:body></w:document>"#;

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

    assert_eq!(first_mc_choice(&alternate_content).requires, "w14");
    assert_eq!(
        first_mc_choice(&alternate_content).mc_ignorable.as_deref(),
        Some(b"w14".as_slice())
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
    assert_eq!(first_mc_choice(&reparsed).requires, "w14");
    assert_eq!(
        first_mc_choice(&reparsed).mc_ignorable.as_deref(),
        Some(b"w14".as_slice())
    );
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
        assert_eq!(
            value.mc_ignorable.as_deref(),
            Some(b"w14&#x9;w15".as_slice())
        );
        assert_eq!(value.mc_process_content.as_deref(), Some(b"w:p".as_slice()));
        assert_eq!(value.mc_must_understand.as_deref(), Some(b"w14".as_slice()));
    }
    assert!(serialized.contains(r#"mc:Ignorable="w14&#x9;w15""#));
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
fn markup_compatibility_preserve_ignored_unknown_element_wildcard_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_Wildcard_FullMode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14" mc:PreserveAttributes="*"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;

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
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14" mc:PreserveAttributes="*"><w:body><w:p w14:paraId="57290E37" w14:editId="5B733B31" w14:textId="5B733B31"/></w:body></w:document>"#;
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

#[test]
fn markup_compatibility_must_understand_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Ignored_UnknownElement_FullMode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2008/9/12/wordml" mc:Ignorable="w14" mc:MustUnderstand="w14"><w:body/></w:document>"#;

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(xml);

    assert_eq!(
        document.mc_must_understand.as_deref(),
        Some(b"w14".as_slice())
    );
    assert_eq!(
        reparsed.mc_must_understand.as_deref(),
        Some(b"w14".as_slice())
    );
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
    assert_eq!(first_mc_choice(&alternate_content).requires, "w14");
    assert_eq!(
        first_mc_choice(&alternate_content)
            .mc_must_understand
            .as_deref(),
        Some(b"w14".as_slice())
    );
    assert!(serialized.contains(r#"mc:MustUnderstand="w14""#));
}

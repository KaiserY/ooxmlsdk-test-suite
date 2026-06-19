use std::io::{Cursor, Read};

use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::SharedStringTable;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::{
    BodyChoice, Document, Paragraph, ParagraphChoice, ParagraphProperties, Run,
};
#[cfg(feature = "mce")]
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings, SdkMce,
};
use ooxmlsdk_test::{assert_stable_roundtrip, fixtures};

fn xml_other_attr<'a>(attrs: &'a [ooxmlsdk::common::XmlOtherAttr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find_map(|attr| (attr.name() == name).then_some(attr.raw_value()))
}

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

fn first_run(paragraph: &Paragraph) -> &Run {
    paragraph
        .paragraph_choice
        .iter()
        .find_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run.as_ref()),
            _ => None,
        })
        .expect("expected run")
}

fn paragraph_properties(paragraph: &Paragraph) -> &ParagraphProperties {
    paragraph
        .paragraph_properties
        .as_ref()
        .expect("expected paragraph properties")
}

#[test]
fn mcsupport_load_attribute_test() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadAttributeTest
    let xml = doc_sample_part("mcdoc.docx", "word/document.xml");

    let (document, serialized, reparsed) = assert_stable_roundtrip::<Document>(&xml);

    assert_eq!(
        xml_other_attr(&document.xml_other_attrs, "mc:Ignorable"),
        Some("w14 wp14")
    );
    assert_eq!(
        xml_other_attr(&reparsed.xml_other_attrs, "mc:Ignorable"),
        Some("w14 wp14")
    );
    assert!(serialized.contains(r#"mc:Ignorable="w14 wp14""#));
    assert!(serialized.contains(r#"mc:PreserveAttributes="w14:myattr""#));
    assert!(serialized.contains(r#"mc:PreserveAttributes="w14:*""#));
}

#[test]
fn mcsupport_load_preserve_attr() {
    // Source: test/DocumentFormat.OpenXml.Tests/ofapiTest/MCSupport.cs
    //   LoadPreserveAttr
    let xml = doc_sample_part("mcdoc.docx", "word/document.xml");

    let (document, _, _) = assert_stable_roundtrip::<Document>(&xml);
    let paragraph = first_paragraph(&document);
    let properties = paragraph_properties(paragraph);
    let spacing = properties
        .spacing_between_lines
        .as_ref()
        .expect("expected spacing");
    let run = first_run(paragraph);
    let run_properties = run
        .run_properties
        .as_ref()
        .expect("expected run properties");

    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr"),
        Some("myattr")
    );
    assert_eq!(
        xml_other_attr(&spacing.xml_other_attrs, "w14:myattr"),
        Some("myattr")
    );
    assert_eq!(
        xml_other_attr(&run.xml_other_attrs, "w14:myattr"),
        Some("myattr")
    );
    assert_eq!(
        xml_other_attr(&run_properties.xml_other_attrs, "w14:myanotherAttr"),
        Some("anotherattr")
    );
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

    assert!(
        xml_other_attr(&paragraph.xml_other_attrs, "w14:editId").is_none(),
        "ProcessLoadedPartsOnly + Office2007 drops ignored w14:editId in the upstream SDK"
    );
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

    assert_eq!(
        xml_other_attr(&item.xml_other_attrs, "mc:Ignorable"),
        Some("w14")
    );
    assert_eq!(
        xml_other_attr(&item.xml_other_attrs, "w14:attr"),
        Some("value")
    );
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
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="  &#x9;&#xA;&#xD; "><w:body><w:p><w:pPr w14:myattr="kept"><w:keepNext/></w:pPr></w:p></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&settings).unwrap();

    let properties = paragraph_properties(first_paragraph(&document));
    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr"),
        Some("kept")
    );
    assert!(properties.keep_next.is_some());

    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:wp14="http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing" mc:Ignorable="w14&#x9;wp14"><w:body><w:p><w:pPr w14:myattr="drop" wp14:other="drop"><w:keepNext/></w:pPr></w:p></w:body></w:document>"#;
    let mut document = xml.parse::<Document>().unwrap();

    document.process_mce(&settings).unwrap();

    let properties = paragraph_properties(first_paragraph(&document));
    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr"),
        None
    );
    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "wp14:other"),
        None
    );
    assert!(properties.keep_next.is_some());
}

#[test]
fn markup_compatibility_ignored_known_attribute_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Ignored_KnownAttribute_FullMode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" w14:myattr="attribute1 from unknown namespace1."><w:keepNext/></w:pPr>"#;

    let (properties, serialized, reparsed) = assert_stable_roundtrip::<ParagraphProperties>(xml);

    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr"),
        Some("attribute1 from unknown namespace1.")
    );
    assert_eq!(
        xml_other_attr(&reparsed.xml_other_attrs, "w14:myattr"),
        Some("attribute1 from unknown namespace1.")
    );
    assert!(serialized.contains(r#"mc:Ignorable="w14""#));
    assert!(serialized.contains(r#"w14:myattr="attribute1 from unknown namespace1.""#));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_ignored_known_attribute_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Ignored_KnownAttribute_O12Mode
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14"><w:body><w:p><w:pPr w14:myattr="attribute1 from unknown namespace1."><w:keepNext/></w:pPr></w:p></w:body></w:document>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut document = xml.parse::<Document>().unwrap();
    document.process_mce(&settings).unwrap();

    let properties = paragraph_properties(first_paragraph(&document));
    assert!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr").is_none(),
        "ProcessAllParts/Office2007 removes ignored known extension attributes upstream"
    );
}

#[test]
fn markup_compatibility_process_content_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   ProcessContent_Ignored_UnknownElement_FullMode
    let xml = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1" mc:Ignorable="uns1" mc:ProcessContent="uns1:e1uk1"><mc:Choice Requires="uns1"><uns1:e1uk1><uns1:child/></uns1:e1uk1></mc:Choice><mc:Fallback/></mc:AlternateContent>"#;

    let (alternate_content, serialized, _) = assert_stable_roundtrip::<
        ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006::AlternateContent,
    >(xml);

    assert_eq!(
        alternate_content.alternate_content_choice.len(),
        2,
        "AlternateContent retains choice and fallback branches"
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

    let (_alternate_content, serialized, _) = assert_stable_roundtrip::<
        ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006::AlternateContent,
    >(xml);

    assert!(serialized.contains(r#"mc:ProcessContent="xml:space""#));
    assert!(serialized.contains(r#"xml:space="preserve""#));
}

#[test]
fn markup_compatibility_preserve_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_FullMode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1" mc:Ignorable="uns1" mc:PreserveElements="uns1:e1uk1" mc:PreserveAttributes="uns1:a1uk1"><w:keepNext/></w:pPr>"#;

    let (_, serialized, _) = assert_stable_roundtrip::<ParagraphProperties>(xml);

    assert!(serialized.contains(r#"mc:Ignorable="uns1""#));
    assert!(serialized.contains(r#"mc:PreserveElements="uns1:e1uk1""#));
    assert!(serialized.contains(r#"mc:PreserveAttributes="uns1:a1uk1""#));
}

#[test]
fn markup_compatibility_preserve_ignored_unknown_element_wildcard_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_Wildcard_FullMode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" mc:PreserveAttributes="*" w14:myattr="attribute1 from unknown namespace1."><w:keepNext/></w:pPr>"#;

    let (properties, serialized, reparsed) = assert_stable_roundtrip::<ParagraphProperties>(xml);

    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "mc:PreserveAttributes"),
        Some("*")
    );
    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr"),
        Some("attribute1 from unknown namespace1.")
    );
    assert_eq!(
        xml_other_attr(&reparsed.xml_other_attrs, "w14:myattr"),
        Some("attribute1 from unknown namespace1.")
    );
    assert!(serialized.contains(r#"mc:PreserveAttributes="*""#));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_preserve_ignored_unknown_element_wildcard_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   Preserve_Ignored_UnknownElement_Wildcard_O12Mode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" mc:Ignorable="w14" mc:PreserveAttributes="*" w14:myattr="attribute1 from unknown namespace1."><w:keepNext/></w:pPr>"#;

    let properties = xml.parse::<ParagraphProperties>().unwrap();

    assert_eq!(
        xml_other_attr(&properties.xml_other_attrs, "w14:myattr"),
        Some("attribute1 from unknown namespace1."),
        "PreserveAttributes=* keeps ignored extension attributes upstream"
    );
}

#[test]
fn markup_compatibility_must_understand_ignored_unknown_element_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Ignored_UnknownElement_FullMode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1" mc:Ignorable="uns1" mc:MustUnderstand="uns1"><w:keepNext/></w:pPr>"#;

    let (_, serialized, _) = assert_stable_roundtrip::<ParagraphProperties>(xml);

    assert!(serialized.contains(r#"mc:Ignorable="uns1""#));
    assert!(serialized.contains(r#"mc:MustUnderstand="uns1""#));
}

#[cfg(feature = "mce")]
#[test]
fn markup_compatibility_must_understand_ignored_unknown_element_o12_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Ignored_UnknownElement_O12Mode
    let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1" mc:Ignorable="uns1" mc:MustUnderstand="uns1"><w:keepNext/></w:pPr>"#;
    let settings = MarkupCompatibilityProcessSettings {
        process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
        target_file_format_version: FileFormatVersion::Office2007,
    };

    let mut properties = xml.parse::<ParagraphProperties>().unwrap();
    let processed = properties.process_mce(&settings);

    assert!(
        processed.is_err(),
        "MCE processing should reject an unsupported MustUnderstand namespace upstream"
    );
}

#[test]
fn markup_compatibility_must_understand_unselected_full_mode() {
    // Source: test/DocumentFormat.OpenXml.Tests/OpenXmlDomTest/MarkupCompatibilityTest.cs
    //   MustUnderstand_Unselected_FullMode
    let xml = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:uns1="http://test.openxmlsdk.microsoft.com/unknownns1"><mc:Choice Requires="uns1" mc:MustUnderstand="uns1"><uns1:e1uk1/></mc:Choice></mc:AlternateContent>"#;

    let (alternate_content, serialized, _) = assert_stable_roundtrip::<
        ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006::AlternateContent,
    >(xml);

    assert_eq!(alternate_content.alternate_content_choice.len(), 1);
    assert!(serialized.contains(r#"mc:MustUnderstand="uns1""#));
}

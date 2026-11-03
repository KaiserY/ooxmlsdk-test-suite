use std::io::Cursor;

use ooxmlsdk::{
    schemas::{a::Run as DrawingRun, v::TextPath, w::Text as WordText, x::Text as SpreadsheetText},
    sdk::SdkType,
};

#[test]
fn attribute_whitespace_survives_generated_node_round_trip() {
    let xml = r#"<v:textpath xmlns:v="urn:schemas-microsoft-com:vml" string=" &#9; a  b &#10;c&#13;&#10;d&#13;e &amp;#9; "/>"#;
    let expected = " \t a  b \nc\r\nd\re &#9; ";

    for node in [
        TextPath::from_bytes(xml.as_bytes()).unwrap(),
        TextPath::from_reader(Cursor::new(xml.as_bytes())).unwrap(),
    ] {
        assert_eq!(node.string.as_deref(), Some(expected));
        let saved = node.to_xml().unwrap();
        for reopened in [
            TextPath::from_bytes(saved.as_bytes()).unwrap(),
            TextPath::from_reader(Cursor::new(saved.as_bytes())).unwrap(),
        ] {
            assert_eq!(reopened.string.as_deref(), Some(expected));
        }
    }
}

#[test]
fn entity_escapes_and_adjacent_whitespace_survive_without_xml_space() {
    let source = " &amp;&lt;&gt;&quot;&apos; &amp;#13; &amp;amp; ";
    let expected = " &<>\"' &#13; &amp; ";
    let word = format!(
        r#"<w:t xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">{source}</w:t>"#
    );
    assert_text_round_trip::<WordText>(&word, expected, |node| node.xml_content.as_deref());

    let sheet = format!(
        r#"<t xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">{source}</t>"#
    );
    assert_text_round_trip::<SpreadsheetText>(&sheet, expected, |node| {
        node.0.xml_content.as_deref()
    });

    let drawing = format!(
        r#"<a:r xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:t>{source}</a:t></a:r>"#
    );
    assert_text_round_trip::<DrawingRun>(&drawing, expected, |node| Some(node.text.as_str()));

    let attribute =
        format!(r#"<v:textpath xmlns:v="urn:schemas-microsoft-com:vml" string="{source}"/>"#);
    for node in [
        TextPath::from_bytes(attribute.as_bytes()).unwrap(),
        TextPath::from_reader(Cursor::new(attribute.as_bytes())).unwrap(),
    ] {
        assert_eq!(node.string.as_deref(), Some(expected));
        let saved = node.to_xml().unwrap();
        assert_eq!(
            TextPath::from_bytes(saved.as_bytes())
                .unwrap()
                .string
                .as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn raw_line_endings_are_normalized_and_text_character_references_are_preserved() {
    let cases = [
        ("a\r\nb\rc\nd\te", "a\nb\nc\nd\te"),
        ("a\r\nb\rc&amp;d", "a\nb\nc&d"),
        ("a&#13;b", "a\rb"),
        ("a&#13;&#10;b", "a\r\nb"),
        ("<![CDATA[a\r\nb\rc]]>", "a\nb\nc"),
        ("a\r\n<![CDATA[b\rc]]>&#13;d", "a\nb\nc\rd"),
        ("a\r\nb\rc&amp;#13;d", "a\nb\nc&#13;d"),
        (" \t a  b \nc ", " \t a  b \nc "),
    ];

    for (source, expected) in cases {
        let word_xml = format!(
            "<w:t xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xml:space=\"preserve\">{source}</w:t>"
        );
        assert_text_round_trip::<WordText>(&word_xml, expected, |node| node.xml_content.as_deref());

        let spreadsheet_xml = format!(
            "<t xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xml:space=\"preserve\">{source}</t>"
        );
        assert_text_round_trip::<SpreadsheetText>(&spreadsheet_xml, expected, |node| {
            node.0.xml_content.as_deref()
        });
    }
}

fn assert_text_round_trip<T: SdkType>(xml: &str, expected: &str, content: fn(&T) -> Option<&str>) {
    for node in [
        T::from_bytes(xml.as_bytes()).unwrap(),
        T::from_reader(Cursor::new(xml.as_bytes())).unwrap(),
    ] {
        assert_eq!(content(&node), Some(expected), "input: {xml:?}");
        let saved = node.to_xml().unwrap();
        for reopened in [
            T::from_bytes(saved.as_bytes()).unwrap(),
            T::from_reader(Cursor::new(saved.as_bytes())).unwrap(),
        ] {
            assert_eq!(content(&reopened), Some(expected), "saved: {saved:?}");
        }
    }
}

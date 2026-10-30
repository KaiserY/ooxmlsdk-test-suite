use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main::{
    NotesMaster, NotesSlide, Presentation, PresentationProperties, Slide, SlideLayout, SlideMaster,
    ViewProperties,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Worksheet;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::Document;
use ooxmlsdk::sdk::SdkType;
use ooxmlsdk_test::assert_stable_roundtrip;

const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const SLIDE_DATA: &str = r#"<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>"#;
const COLOR_MAP: &str = r#"<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#;

macro_rules! part_root_test {
    ($test:ident, $ty:ty, $qname:literal, $namespace:ident, $body:expr) => {
        #[test]
        fn $test() {
            let qname = $qname;
            let prefix = qname.split_once(':').unwrap().0;
            let namespace = $namespace;
            let body = $body;
            let xml = format!(r#"<{qname} xmlns:{prefix}="{namespace}" xmlns:mc="{MC}" xmlns:u="urn:preserve" mc:PreserveElements="u:note u:*">{body}</{qname}>"#);
            let (parsed, serialized, reparsed) = assert_stable_roundtrip::<$ty>(&xml);

            for root in [&parsed, &reparsed] {
                assert_eq!(root.mc_preserve_elements.as_deref(), Some(b"u:note u:*".as_slice()));
            }
            assert!(serialized.contains(r#"mc:PreserveElements="u:note u:*""#));
        }
    };
}

part_root_test!(document_roundtrip, Document, "w:document", W, "<w:body/>");
part_root_test!(
    worksheet_roundtrip,
    Worksheet,
    "x:worksheet",
    S,
    "<x:sheetData/>"
);
part_root_test!(
    presentation_roundtrip,
    Presentation,
    "p:presentation",
    P,
    r#"<p:notesSz cx="9144000" cy="6858000"/>"#
);
part_root_test!(
    presentation_properties_roundtrip,
    PresentationProperties,
    "p:presentationPr",
    P,
    ""
);
part_root_test!(slide_roundtrip, Slide, "p:sld", P, SLIDE_DATA);
part_root_test!(
    slide_layout_roundtrip,
    SlideLayout,
    "p:sldLayout",
    P,
    SLIDE_DATA
);
part_root_test!(
    slide_master_roundtrip,
    SlideMaster,
    "p:sldMaster",
    P,
    format!("{SLIDE_DATA}{COLOR_MAP}")
);
part_root_test!(notes_slide_roundtrip, NotesSlide, "p:notes", P, SLIDE_DATA);
part_root_test!(
    notes_master_roundtrip,
    NotesMaster,
    "p:notesMaster",
    P,
    format!("{SLIDE_DATA}{COLOR_MAP}")
);
part_root_test!(view_properties_roundtrip, ViewProperties, "p:viewPr", P, "");

#[test]
fn document_preserve_elements_can_be_empty_or_absent() {
    for (attribute, expected) in [
        ("", None),
        (r#"mc:PreserveElements="""#, Some(b"".as_slice())),
    ] {
        let xml = format!(
            r#"<w:document xmlns:w="{W}" xmlns:mc="{MC}" {attribute}><w:body/></w:document>"#
        );
        let (parsed, serialized, reparsed) = assert_stable_roundtrip::<Document>(&xml);

        assert_eq!(parsed.mc_preserve_elements.as_deref(), expected);
        assert_eq!(
            serialized.contains("mc:PreserveElements="),
            expected.is_some()
        );
        assert_eq!(parsed, reparsed);
    }
}

#[test]
fn document_preserve_elements_can_be_changed_and_removed() {
    let xml = format!(
        r#"<w:document xmlns:w="{W}" xmlns:mc="{MC}" xmlns:u="urn:preserve" mc:PreserveElements="u:note"><w:body/></w:document>"#
    );
    let mut document = xml.parse::<Document>().unwrap();
    document.mc_preserve_elements = Some(b"u:*".as_slice().into());
    let (_, serialized, reparsed) =
        assert_stable_roundtrip::<Document>(&document.to_xml().unwrap());
    assert_eq!(
        reparsed.mc_preserve_elements.as_deref(),
        Some(b"u:*".as_slice())
    );
    assert!(serialized.contains(r#"mc:PreserveElements="u:*""#));

    document.mc_preserve_elements = None;
    let (_, serialized, reparsed) =
        assert_stable_roundtrip::<Document>(&document.to_xml().unwrap());
    assert!(reparsed.mc_preserve_elements.is_none());
    assert!(!serialized.contains("mc:PreserveElements="));
}

#[cfg(feature = "parts")]
#[test]
fn document_preserve_elements_survives_package_edit_save_reopen() {
    use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;
    use ooxmlsdk::schemas::xml::SpaceProcessingModeValues;
    use ooxmlsdk_test::fixtures;
    use std::io::Cursor;

    let xml = format!(
        r#"<w:document xmlns:w="{W}" xmlns:mc="{MC}" xmlns:u="urn:preserve" mc:PreserveElements="u:note"><w:body/></w:document>"#
    );
    let mut package =
        WordprocessingDocument::new_from_file(fixtures::doc_sample_path("HelloWorld.docx"))
            .unwrap();
    let main = package.main_document_part().unwrap();
    main.set_data(&mut package, xml.into_bytes()).unwrap();
    main.root_element_mut(&mut package).unwrap().space = Some(SpaceProcessingModeValues::Preserve);

    let bytes = package.to_package_bytes().unwrap();
    let reopened = WordprocessingDocument::new(Cursor::new(bytes)).unwrap();
    let main = reopened.main_document_part().unwrap();
    let document = main.root_element(&reopened).unwrap();

    assert_eq!(
        document.mc_preserve_elements.as_deref(),
        Some(b"u:note".as_slice())
    );
    assert_eq!(document.space, Some(SpaceProcessingModeValues::Preserve));
}

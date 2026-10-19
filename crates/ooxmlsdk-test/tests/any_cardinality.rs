use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006 as mc;
use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;
use ooxmlsdk::sdk::SdkType;

#[test]
fn single_any_fields_keep_the_last_element_without_rejecting_duplicates() {
    let first = r#"<probe:first xmlns:probe="urn:probe"/>"#;
    let last = r#"<probe:last xmlns:probe="urn:probe"/>"#;
    for children in [String::new(), last.to_string(), format!("{first}{last}")] {
        let required = format!(
            r#"<p:ext xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" uri="urn:probe">{children}</p:ext>"#
        );
        let optional = format!(
            r#"<a:ext xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" uri="urn:probe">{children}</a:ext>"#
        );
        let optional = a::Extension::from_bytes(optional.as_bytes()).unwrap();
        let _: &Option<Box<[u8]>> = &optional.xml_children;
        if children.is_empty() {
            assert!(matches!(
                p::Extension::from_bytes(required.as_bytes()),
                Err(ooxmlsdk::common::SdkError::MissingField {
                    field: "xml_children",
                    ..
                })
            ));
            assert!(optional.xml_children.is_none());
            continue;
        }
        let required = p::Extension::from_bytes(required.as_bytes()).unwrap();
        for value in [
            required.xml_children.as_ref(),
            optional.xml_children.as_deref().unwrap(),
        ] {
            let raw = std::str::from_utf8(value).unwrap();
            assert!(raw.contains("probe:last"));
            assert!(!raw.contains("probe:first"));
        }
        assert_eq!(
            p::Extension::from_bytes(required.to_xml().unwrap().as_bytes()).unwrap(),
            required
        );
        assert_eq!(
            a::Extension::from_bytes(optional.to_xml().unwrap().as_bytes()).unwrap(),
            optional
        );
        let _: Box<[u8]> = required.xml_children;
    }
}

#[test]
fn mce_any_fields_keep_all_elements_in_order() {
    for children in [
        "",
        r#"<probe:first xmlns:probe="urn:probe"/><probe:last xmlns:probe="urn:probe"/>"#,
    ] {
        let xml = format!(
            r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice Requires="probe">{children}</mc:Choice><mc:Fallback>{children}</mc:Fallback></mc:AlternateContent>"#
        );
        let parsed = mc::AlternateContent::from_bytes(xml.as_bytes()).unwrap();
        let saved = parsed.to_xml().unwrap();
        for tag in ["probe:first", "probe:last"] {
            assert_eq!(
                saved.matches(tag).count(),
                if children.is_empty() { 0 } else { 2 }
            );
        }
        assert_eq!(
            mc::AlternateContent::from_bytes(saved.as_bytes()).unwrap(),
            parsed
        );
    }
}

use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_markup_compatibility_2006 as mc;
use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;
use ooxmlsdk::sdk::SdkType;

#[test]
fn open_blip_extensions_distinguish_known_aliases_from_foreign_payloads() {
    for prefix in ["asvg", "alias"] {
        let xml = format!(
            r#"<a:ext xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" uri="{{96DAC541-7B7A-43D3-8B79-37D633B846F1}}"><{prefix}:svgBlip xmlns:{prefix}="http://schemas.microsoft.com/office/drawing/2016/SVG/main" r:embed="vector"/></a:ext>"#
        );
        let extension = a::BlipExtension::from_bytes(xml.as_bytes()).unwrap();
        let Some(a::BlipExtensionChoice::SvgBlip(svg)) = &extension.blip_extension_choice else {
            panic!("known SVG namespace was not recognized");
        };
        assert_eq!(svg.embed.as_deref(), Some("vector"));
        let serialized = extension.to_xml().unwrap();
        assert_eq!(
            a::BlipExtension::from_bytes(serialized.as_bytes()).unwrap(),
            extension
        );
    }
    for local_name in ["svgBlip", "useLocalDpi"] {
        for empty in [false, true] {
            let child = if empty {
                format!(r#"<x:{local_name} xmlns:x="urn:foreign" payload="kept"/>"#)
            } else {
                format!(
                    r#"<x:{local_name} xmlns:x="urn:foreign" payload="kept"><x:future/></x:{local_name}>"#
                )
            };
            let xml = format!(
                r#"<a:ext xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" uri="urn:probe">{child}</a:ext>"#
            );
            let extension = a::BlipExtension::from_bytes(xml.as_bytes()).unwrap();
            let Some(a::BlipExtensionChoice::XmlAny(raw)) = &extension.blip_extension_choice else {
                panic!("foreign payload was consumed as {local_name}");
            };
            let raw = std::str::from_utf8(raw).unwrap();
            assert!(raw.contains(r#"payload="kept""#));
            assert_eq!(raw.contains("<x:future"), !empty);
            let serialized = extension.to_xml().unwrap();
            assert_eq!(
                a::BlipExtension::from_bytes(serialized.as_bytes()).unwrap(),
                extension
            );
        }
    }
}

#[test]
fn open_blip_extensions_resolve_root_default_and_encoded_namespace_aliases() {
    for (declarations, child) in [
        (
            r#"xmlns:alias="http://schemas.microsoft.com/office/drawing/2016/SVG/main""#,
            r#"<alias:svgBlip r:embed="vector"/>"#,
        ),
        (
            "",
            r#"<svgBlip xmlns="http://schemas.microsoft.com/office/drawing/2016/SVG/main" r:embed="vector"/>"#,
        ),
        (
            "",
            r#"<alias:svgBlip xmlns:alias="http://schemas.microsoft.com/office/drawing/2016/SVG/ma&#105;n" r:embed="vector"/>"#,
        ),
    ] {
        let xml = format!(
            r#"<a:ext xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" {declarations} uri="urn:probe">{child}</a:ext>"#
        );
        let extension = a::BlipExtension::from_bytes(xml.as_bytes()).unwrap();
        let Some(a::BlipExtensionChoice::SvgBlip(svg)) = &extension.blip_extension_choice else {
            panic!("known SVG binding was not recognized: {child}");
        };
        assert_eq!(svg.embed.as_deref(), Some("vector"));
        let serialized = extension.to_xml().unwrap();
        assert_eq!(
            a::BlipExtension::from_bytes(serialized.as_bytes()).unwrap(),
            extension
        );
    }
    let rebound = r#"<a:ext xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:alias="http://schemas.microsoft.com/office/drawing/2016/SVG/main" uri="urn:probe"><alias:svgBlip xmlns:alias="urn:foreign" payload="kept"/></a:ext>"#;
    let extension = a::BlipExtension::from_bytes(rebound.as_bytes()).unwrap();
    assert!(matches!(
        extension.blip_extension_choice,
        Some(a::BlipExtensionChoice::XmlAny(_))
    ));
}

#[test]
fn open_nonvisual_extensions_preserve_foreign_creation_ids() {
    let xml = r#"<a:ext xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" uri="urn:probe"><x:creationId xmlns:x="urn:foreign" id="foreign"><x:future/></x:creationId></a:ext>"#;
    let extension = a::NonVisualDrawingPropertiesExtension::from_bytes(xml.as_bytes()).unwrap();
    let Some(a::NonVisualDrawingPropertiesExtensionChoice::XmlAny(raw)) =
        &extension.non_visual_drawing_properties_extension_choice
    else {
        panic!("foreign creationId was consumed as a known extension");
    };
    let raw = std::str::from_utf8(raw).unwrap();
    assert!(raw.contains(r#"id="foreign""#));
    assert!(raw.contains("<x:future"));
    let serialized = extension.to_xml().unwrap();
    assert_eq!(
        a::NonVisualDrawingPropertiesExtension::from_bytes(serialized.as_bytes()).unwrap(),
        extension
    );
}

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

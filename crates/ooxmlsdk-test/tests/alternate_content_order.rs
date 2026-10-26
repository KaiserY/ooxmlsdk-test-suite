use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main::{
    Slide, SlideLayout, SlideMaster,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::{
    LegacyDrawing, Worksheet,
};
use ooxmlsdk_test::assert_stable_roundtrip;
use quick_xml::events::Event;

const P_NS: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" mc:Ignorable="p14""#;
const C_SLD: &str = r#"<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>"#;
const CLR_MAP_OVR: &str = r#"<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>"#;
const CLR_MAP: &str = r#"<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#;
const TRANSITION: &str = r#"<p:transition spd="slow"><p:fade thruBlk="1"/></p:transition>"#;
const TIMING: &str = r#"<p:timing><p:tnLst><p:par><p:cTn id="1"/></p:par></p:tnLst></p:timing>"#;
const WRAPPED_TRANSITION: &str = r#"<mc:AlternateContent><mc:Choice Requires="p14"><p:transition spd="slow" p14:dur="2000"><p14:prism/></p:transition></mc:Choice><mc:Fallback><p:transition spd="slow"><p:fade/></p:transition></mc:Fallback></mc:AlternateContent>"#;
const WRAPPED_TIMING: &str = r#"<mc:AlternateContent><mc:Choice Requires="p14"><p:timing><p:tnLst><p:par><p:cTn id="1" p14:presetBounceEnd="1"/></p:par></p:tnLst></p:timing></mc:Choice><mc:Fallback><p:timing><p:tnLst><p:par><p:cTn id="1"/></p:par></p:tnLst></p:timing></mc:Fallback></mc:AlternateContent>"#;

fn child_order(xml: &str) -> Vec<String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut depth = 0usize;
    let mut in_alternate_content = false;
    let mut children = Vec::new();
    loop {
        let (start, empty) = match reader.read_event().unwrap() {
            Event::Start(start) => (start, false),
            Event::Empty(start) => (start, true),
            Event::End(_) => {
                depth -= 1;
                if depth == 1 {
                    in_alternate_content = false;
                }
                continue;
            }
            Event::Eof => return children,
            _ => continue,
        };
        let name = String::from_utf8(start.local_name().as_ref().to_vec()).unwrap();
        if depth == 1 {
            in_alternate_content = name == "AlternateContent";
            children.push(if in_alternate_content {
                "AC()".to_string()
            } else {
                name
            });
        } else if depth == 3 && in_alternate_content {
            let last = children.last_mut().unwrap();
            if last == "AC()" {
                *last = format!("AC({name})");
            }
        }
        if !empty {
            depth += 1;
        }
    }
}

fn slide(children: &str) -> String {
    format!("<p:sld {P_NS}>{C_SLD}{CLR_MAP_OVR}{children}</p:sld>")
}

#[track_caller]
fn assert_slide_order(children: &str, expected: &[&str]) -> Slide {
    let (parsed, serialized, _) = assert_stable_roundtrip::<Slide>(&slide(children));
    assert_eq!(child_order(&serialized), expected, "{serialized}");
    parsed
}

#[test]
fn slide_direct_transition_then_wrapped_timing() {
    assert_slide_order(
        &format!("{TRANSITION}{WRAPPED_TIMING}"),
        &["cSld", "clrMapOvr", "transition", "AC(timing)"],
    );
}

#[test]
fn slide_wrapped_transition_then_direct_timing() {
    assert_slide_order(
        &format!("{WRAPPED_TRANSITION}{TIMING}"),
        &["cSld", "clrMapOvr", "AC(transition)", "timing"],
    );
}

#[test]
fn slide_wrapped_transition_then_wrapped_timing() {
    assert_slide_order(
        &format!("{WRAPPED_TRANSITION}{WRAPPED_TIMING}"),
        &["cSld", "clrMapOvr", "AC(transition)", "AC(timing)"],
    );
}

#[test]
fn slide_direct_transition_then_direct_timing() {
    assert_slide_order(
        &format!("{TRANSITION}{TIMING}"),
        &["cSld", "clrMapOvr", "transition", "timing"],
    );
}

#[test]
fn slide_without_color_map_override_keeps_wrapped_timing_after_transition() {
    let xml = format!("<p:sld {P_NS}>{C_SLD}{TRANSITION}{WRAPPED_TIMING}</p:sld>");
    let (_, serialized, _) = assert_stable_roundtrip::<Slide>(&xml);
    assert_eq!(
        child_order(&serialized),
        ["cSld", "transition", "AC(timing)"],
        "{serialized}"
    );
}

#[test]
fn slide_wrapped_timing_only_then_add_direct_transition() {
    let mut slide = assert_slide_order(WRAPPED_TIMING, &["cSld", "clrMapOvr", "AC(timing)"]);
    slide.transition = Some(Default::default());
    let serialized = slide.to_string();
    assert_eq!(
        child_order(&serialized),
        ["cSld", "clrMapOvr", "transition", "AC(timing)"],
        "{serialized}"
    );
}

#[test]
fn slide_wrapped_transition_only_then_add_direct_timing() {
    let mut slide =
        assert_slide_order(WRAPPED_TRANSITION, &["cSld", "clrMapOvr", "AC(transition)"]);
    slide.timing = Some(Default::default());
    let serialized = slide.to_string();
    assert_eq!(
        child_order(&serialized),
        ["cSld", "clrMapOvr", "AC(transition)", "timing"],
        "{serialized}"
    );
}

#[test]
fn slide_repeated_wrapped_transitions_stay_together_when_transition_is_added() {
    let mut slide = assert_slide_order(
        &format!("{WRAPPED_TRANSITION}{WRAPPED_TRANSITION}{WRAPPED_TIMING}"),
        &[
            "cSld",
            "clrMapOvr",
            "AC(transition)",
            "AC(transition)",
            "AC(timing)",
        ],
    );
    slide.transition = Some(Default::default());
    let serialized = slide.to_string();
    assert_eq!(
        child_order(&serialized),
        [
            "cSld",
            "clrMapOvr",
            "AC(transition)",
            "AC(transition)",
            "transition",
            "AC(timing)"
        ],
        "{serialized}"
    );
}

#[test]
fn slide_unrelated_edit_keeps_transition_and_wrapped_timing() {
    let mut slide = assert_slide_order(
        &format!("{TRANSITION}{WRAPPED_TIMING}"),
        &["cSld", "clrMapOvr", "transition", "AC(timing)"],
    );
    slide.show = Some(false.into());
    let serialized = slide.to_string();
    assert_eq!(
        child_order(&serialized),
        ["cSld", "clrMapOvr", "transition", "AC(timing)"],
        "{serialized}"
    );
    assert!(serialized.contains(
    r#"<mc:Choice Requires="p14"><p:timing><p:tnLst><p:par><p:cTn id="1" p14:presetBounceEnd="1"/>"#
  ));
    assert!(serialized.contains(r#"<mc:Fallback><p:timing><p:tnLst><p:par><p:cTn id="1"/>"#));
}

#[test]
fn slide_foreign_element_named_timing_keeps_its_position() {
    let xml = format!(
        r#"<p:sld {P_NS} xmlns:f="urn:future-extension">{C_SLD}{CLR_MAP_OVR}<mc:AlternateContent><mc:Choice Requires="f"><f:timing marker="opaque"/></mc:Choice><mc:Fallback/></mc:AlternateContent>{TRANSITION}</p:sld>"#
    );
    let (_, serialized, _) = assert_stable_roundtrip::<Slide>(&xml);
    assert_eq!(
        child_order(&serialized),
        ["cSld", "clrMapOvr", "AC(timing)", "transition"],
        "{serialized}"
    );
}

#[test]
fn slide_wrapped_timing_with_aliased_prefixes_stays_after_added_transition() {
    let xml = format!(
        r#"<p:sld {P_NS} xmlns:pp="http://schemas.openxmlformats.org/presentationml/2006/main">{C_SLD}{CLR_MAP_OVR}<mc:AlternateContent><mc:Choice Requires="p14"><pp:timing><pp:tnLst><pp:par><pp:cTn id="1"/></pp:par></pp:tnLst></pp:timing></mc:Choice></mc:AlternateContent><mc:AlternateContent><mc:Choice xmlns:q="http://schemas.openxmlformats.org/presentationml/2006/main" Requires="p14"><q:timing><q:tnLst><q:par><q:cTn id="1"/></q:par></q:tnLst></q:timing></mc:Choice></mc:AlternateContent><mc:AlternateContent><mc:Choice Requires="p14"><timing xmlns="http://schemas.openxmlformats.org/presentationml/2006/main"><tnLst><par><cTn id="1"/></par></tnLst></timing></mc:Choice></mc:AlternateContent></p:sld>"#
    );
    let mut slide = xml.parse::<Slide>().unwrap();
    slide.transition = Some(Default::default());
    let serialized = slide.to_string();
    assert_eq!(
        child_order(&serialized),
        [
            "cSld",
            "clrMapOvr",
            "transition",
            "AC(timing)",
            "AC(timing)",
            "AC(timing)"
        ],
        "{serialized}"
    );
}

#[test]
fn slide_wrapped_timing_with_encoded_namespace_uri_stays_after_added_transition() {
    for uri in [
        "http://schemas.openxmlformats.org/presentationml/2006/main",
        "http://schemas.openxmlformats.org/presentationml/2006/ma&#x69;n",
    ] {
        let mut slide = slide(&format!(
            r#"<mc:AlternateContent><mc:Choice Requires="p14"><q:timing xmlns:q="{uri}"/></mc:Choice></mc:AlternateContent>"#
        ))
        .parse::<Slide>()
        .unwrap();
        slide.transition = Some(Default::default());
        let serialized = slide.to_string();
        assert_eq!(
            child_order(&serialized),
            ["cSld", "clrMapOvr", "transition", "AC(timing)"],
            "{uri}: {serialized}"
        );
    }
}

#[test]
fn slide_preserves_wrapped_payload_with_undecodable_namespace() {
    for payload in [
        r#"<q:timing xmlns:q="urn:&bogus;"/>"#,
        r#"<p:timing xmlns:p="urn:&bogus;"/>"#,
        r#"<timing xmlns="urn:&bogus;"/>"#,
    ] {
        for preceding_transition in [false, true] {
            let wrapped = format!(
                r#"<mc:AlternateContent><mc:Choice Requires="p14">{payload}</mc:Choice></mc:AlternateContent>"#
            );
            let children = if preceding_transition {
                format!("{TRANSITION}{wrapped}")
            } else {
                wrapped
            };
            let (mut parsed, serialized, _) = assert_stable_roundtrip::<Slide>(&slide(&children));
            assert!(serialized.contains(payload), "{serialized}");
            assert_eq!(
                parsed.alternate_content_0.len(),
                usize::from(!preceding_transition)
            );
            assert_eq!(
                parsed.alternate_content_1.len(),
                usize::from(preceding_transition)
            );
            parsed.transition = Some(Default::default());
            let serialized = parsed.to_string();
            let expected = if preceding_transition {
                ["cSld", "clrMapOvr", "transition", "AC(timing)"]
            } else {
                ["cSld", "clrMapOvr", "AC(timing)", "transition"]
            };
            assert_eq!(child_order(&serialized), expected, "{serialized}");
            assert!(serialized.contains(payload), "{serialized}");
            assert_stable_roundtrip::<Slide>(&serialized);
        }
    }
}

#[test]
fn slide_routes_by_valid_fallback_after_undecodable_choice_namespace() {
    let payload = r#"<p:timing xmlns:p="urn:&bogus;"/>"#;
    let xml = slide(&format!(
        r#"<mc:AlternateContent><mc:Choice Requires="p14">{payload}</mc:Choice><mc:Fallback><p:timing/></mc:Fallback></mc:AlternateContent>"#
    ));
    let (mut parsed, _, _) = assert_stable_roundtrip::<Slide>(&xml);
    assert!(parsed.alternate_content_0.is_empty());
    assert_eq!(parsed.alternate_content_1.len(), 1);
    parsed.transition = Some(Default::default());
    let serialized = parsed.to_string();
    assert_eq!(
        child_order(&serialized),
        ["cSld", "clrMapOvr", "transition", "AC(timing)"],
        "{serialized}"
    );
    assert!(serialized.contains(payload), "{serialized}");
}

#[test]
fn slide_still_rejects_structurally_malformed_wrapped_payload() {
    let xml = slide(
        r#"<mc:AlternateContent><mc:Choice Requires="p14"><p:timing></p:transition></mc:Choice></mc:AlternateContent>"#,
    );
    assert!(xml.parse::<Slide>().is_err());
}

#[test]
fn slide_layout_direct_transition_then_wrapped_timing() {
    let xml = format!(
        "<p:sldLayout {P_NS}>{C_SLD}{CLR_MAP_OVR}{TRANSITION}{WRAPPED_TIMING}</p:sldLayout>"
    );
    let (_, serialized, _) = assert_stable_roundtrip::<SlideLayout>(&xml);
    assert_eq!(
        child_order(&serialized),
        ["cSld", "clrMapOvr", "transition", "AC(timing)"],
        "{serialized}"
    );
}

#[test]
fn slide_master_direct_transition_then_wrapped_timing() {
    let xml = format!(
        r#"<p:sldMaster {P_NS}>{C_SLD}{CLR_MAP}<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>{TRANSITION}{WRAPPED_TIMING}</p:sldMaster>"#
    );
    let (_, serialized, _) = assert_stable_roundtrip::<SlideMaster>(&xml);
    assert_eq!(
        child_order(&serialized),
        [
            "cSld",
            "clrMap",
            "sldLayoutIdLst",
            "transition",
            "AC(timing)"
        ],
        "{serialized}"
    );
}

#[test]
fn worksheet_wrapped_controls_stay_after_intervening_siblings() {
    let xml = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" mc:Ignorable="x14"><sheetData/><legacyDrawing r:id="rId1"/><picture r:id="rId2"/><mc:AlternateContent><mc:Choice Requires="x14"><controls><control shapeId="1025" r:id="rId3" name="Check Box 1"/></controls></mc:Choice></mc:AlternateContent><tableParts count="0"/></worksheet>"#;
    let (_, serialized, _) = assert_stable_roundtrip::<Worksheet>(xml);
    assert_eq!(
        child_order(&serialized),
        [
            "sheetData",
            "legacyDrawing",
            "picture",
            "AC(controls)",
            "tableParts"
        ],
        "{serialized}"
    );
}

#[test]
fn worksheet_wrapped_controls_stay_after_added_legacy_drawing() {
    let xml = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" mc:Ignorable="x14"><sheetData/><drawing r:id="rId1"/><mc:AlternateContent><mc:Choice Requires="x14"><controls><control shapeId="1025" r:id="rId3" name="Check Box 1"/></controls></mc:Choice></mc:AlternateContent></worksheet>"#;
    let (mut worksheet, _, _) = assert_stable_roundtrip::<Worksheet>(xml);
    worksheet.legacy_drawing = Some(LegacyDrawing { id: "rId2".into() });
    let serialized = worksheet.to_string();
    assert_eq!(
        child_order(&serialized),
        ["sheetData", "drawing", "legacyDrawing", "AC(controls)"],
        "{serialized}"
    );
}

#[cfg(feature = "mce")]
#[test]
fn slide_process_mce_promotes_wrapped_timing_from_its_slot() {
    use ooxmlsdk::sdk::{
        FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
    };

    for version in [FileFormatVersion::Office2007, FileFormatVersion::Office2010] {
        let mut slide = slide(&format!("{TRANSITION}{WRAPPED_TIMING}"))
            .parse::<Slide>()
            .unwrap();
        slide
            .process_mce(&MarkupCompatibilityProcessSettings {
                process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
                target_file_format_version: version,
            })
            .unwrap();
        assert!(slide.alternate_content_0.is_empty() && slide.alternate_content_1.is_empty());
        assert!(
            slide.transition.is_some() && slide.timing.is_some(),
            "{version:?}"
        );
        let serialized = slide.to_string();
        assert_eq!(
            child_order(&serialized),
            ["cSld", "clrMapOvr", "transition", "timing"],
            "{serialized}"
        );
    }
}

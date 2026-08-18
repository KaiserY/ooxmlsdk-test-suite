use ooxmlsdk::schemas::schemas_microsoft_com_office_office::SignatureLine;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::Indentation;
use ooxmlsdk_test::assert_stable_roundtrip;

const OFFICE_NS: &str = "urn:schemas-microsoft-com:office:office";
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[test]
fn indentation_reads_unqualified_left_alias_and_writes_canonical_qname() {
    let xml = format!(r#"<w:ind xmlns:w="{WORD_NS}" left="2880"/>"#);
    let (indentation, serialized, reparsed) = assert_stable_roundtrip::<Indentation>(&xml);

    assert_eq!(
        indentation
            .left
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("2880")
    );
    assert!(serialized.contains(r#" w:left="2880""#));
    assert!(!serialized.contains(r#" left="2880""#));
    assert_eq!(indentation, reparsed);
}

#[test]
fn signature_line_reads_namespaced_alias_and_writes_canonical_qname() {
    let xml = format!(
        r#"<o:signatureline xmlns:o="{OFFICE_NS}" xmlns:alias="{OFFICE_NS}" alias:signinginstructions="Check"/>"#
    );
    let (signature_line, serialized, reparsed) = assert_stable_roundtrip::<SignatureLine>(&xml);

    assert_eq!(
        signature_line.signing_instructions.as_deref(),
        Some("Check")
    );
    assert!(serialized.contains(r#" signinginstructions="Check""#));
    assert!(!serialized.contains("alias:signinginstructions"));
    assert_eq!(signature_line, reparsed);
}

#[test]
fn primary_attribute_qnames_win_over_read_aliases_in_either_order() {
    for xml in [
        format!(r#"<w:ind xmlns:w="{WORD_NS}" left="1440" w:left="2880"/>"#),
        format!(r#"<w:ind xmlns:w="{WORD_NS}" w:left="2880" left="1440"/>"#),
    ] {
        let (indentation, serialized, _) = assert_stable_roundtrip::<Indentation>(&xml);
        assert_eq!(
            indentation
                .left
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("2880")
        );
        assert!(serialized.contains(r#" w:left="2880""#));
        assert!(!serialized.contains("1440"));
    }

    for xml in [
        format!(
            r#"<o:signatureline xmlns:o="{OFFICE_NS}" o:signinginstructions="alias" signinginstructions="primary"/>"#
        ),
        format!(
            r#"<o:signatureline xmlns:o="{OFFICE_NS}" signinginstructions="primary" o:signinginstructions="alias"/>"#
        ),
    ] {
        let (signature_line, serialized, _) = assert_stable_roundtrip::<SignatureLine>(&xml);
        assert_eq!(
            signature_line.signing_instructions.as_deref(),
            Some("primary")
        );
        assert!(serialized.contains(r#" signinginstructions="primary""#));
        assert!(!serialized.contains("alias"));
    }
}

#[test]
fn read_aliases_do_not_accept_the_same_local_name_from_another_namespace() {
    let indentation_xml =
        format!(r#"<w:ind xmlns:w="{WORD_NS}" xmlns:x="urn:wrong" x:left="2880"/>"#);
    let (indentation, serialized, _) = assert_stable_roundtrip::<Indentation>(&indentation_xml);
    assert!(indentation.left.is_none());
    assert!(!serialized.contains("left="));

    let signature_xml = format!(
        r#"<o:signatureline xmlns:o="{OFFICE_NS}" xmlns:x="urn:wrong" x:signinginstructions="Wrong"/>"#
    );
    let (signature_line, serialized, _) = assert_stable_roundtrip::<SignatureLine>(&signature_xml);
    assert!(signature_line.signing_instructions.is_none());
    assert!(!serialized.contains("signinginstructions="));
}

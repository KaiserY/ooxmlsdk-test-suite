pub mod fixtures;

fn xml_namespace_prefix(namespace: &ooxmlsdk::common::XmlNamespace) -> &[u8] {
    match namespace {
        ooxmlsdk::common::XmlNamespace::Known(namespace) => namespace.prefix_bytes(),
        ooxmlsdk::common::XmlNamespace::Raw(raw) => {
            raw.split(|byte| *byte == 0).next().unwrap_or_default()
        }
    }
}

pub fn xml_namespace_prefixes_match(
    actual: &[ooxmlsdk::common::XmlNamespace],
    expected: &[&str],
) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| xml_namespace_prefix(actual) == expected.as_bytes())
}

#[track_caller]
pub fn assert_xml_namespace_prefixes(
    actual: Option<&[ooxmlsdk::common::XmlNamespace]>,
    expected: &[&str],
) {
    assert!(
        actual.is_some_and(|actual| xml_namespace_prefixes_match(actual, expected)),
        "expected namespace prefixes {expected:?}, got {actual:?}"
    );
}

#[track_caller]
pub fn assert_roundtrip<T>(xml: &str) -> (T, String, T)
where
    T: std::fmt::Display + std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    let parsed = xml.parse::<T>().unwrap();
    let serialized_once = parsed.to_string();
    let reparsed = serialized_once.parse::<T>().unwrap();

    (parsed, serialized_once, reparsed)
}

#[track_caller]
pub fn assert_stable_roundtrip<T>(xml: &str) -> (T, String, T)
where
    T: std::fmt::Display + std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    let (parsed, serialized_once, reparsed) = assert_roundtrip::<T>(xml);
    let serialized_twice = reparsed.to_string();

    assert_eq!(serialized_once, serialized_twice);

    (parsed, serialized_once, reparsed)
}

#[track_caller]
pub fn trim_xml_declaration(xml: &str) -> &str {
    const XML_DECL_END: &str = "?>";

    let xml = xml.trim();

    if let Some(stripped) = xml.strip_prefix("<?xml") {
        let end = stripped
            .find(XML_DECL_END)
            .expect("unterminated xml declaration");
        stripped[end + XML_DECL_END.len()..].trim()
    } else {
        xml
    }
}

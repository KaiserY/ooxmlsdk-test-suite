use ooxmlsdk_pdf_test::{OfficeGoldenFormat, run_office_golden_corpus};

fn assert_ratchet(format: OfficeGoldenFormat, target: usize) {
    let target = std::env::var("OOXMLSDK_GOLDEN_TARGET")
        .ok()
        .map(|value| value.parse::<usize>().expect("valid golden pass target"))
        .unwrap_or(target);
    let report = run_office_golden_corpus(format, target).unwrap();
    if std::env::var_os("OOXMLSDK_GOLDEN_CASE").is_some() {
        assert_eq!(report.attempted, 1);
    } else if std::env::var("OOXMLSDK_GOLDEN_AUDIT_ERRORS").is_ok_and(|value| value == "1")
        && [
            "OOXMLSDK_GOLDEN_ERROR_CLASS",
            "OOXMLSDK_GOLDEN_AUDIT_LIMIT",
            "OOXMLSDK_GOLDEN_AUDIT_OFFSET",
            "OOXMLSDK_GOLDEN_CORPUS",
            "OOXMLSDK_GOLDEN_SOURCE_CONTAINS",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
    {
        assert!(report.attempted > 0);
    } else {
        assert_eq!(report.passed, target);
    }
}

#[test]
#[ignore = "run the streamed Office golden corpus ratchet explicitly"]
fn office_golden_docx_corpus_ratchet() {
    assert_ratchet(OfficeGoldenFormat::Docx, 939);
}

#[test]
#[ignore = "run the streamed Office golden corpus ratchet explicitly"]
fn office_golden_pptx_corpus_ratchet() {
    assert_ratchet(OfficeGoldenFormat::Pptx, 312);
}

#[test]
#[ignore = "run the streamed Office golden corpus ratchet explicitly"]
fn office_golden_xlsx_corpus_ratchet() {
    assert_ratchet(OfficeGoldenFormat::Xlsx, 218);
}

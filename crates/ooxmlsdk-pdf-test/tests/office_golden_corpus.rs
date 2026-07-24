use ooxmlsdk_pdf_test::{OfficeGoldenFormat, run_office_golden_corpus};

fn assert_ratchet(format: OfficeGoldenFormat, target: usize) {
    let target = std::env::var("OOXMLSDK_GOLDEN_TARGET")
        .ok()
        .map(|value| value.parse::<usize>().expect("valid golden pass target"))
        .unwrap_or(target);
    let report = run_office_golden_corpus(format, target).unwrap();
    let exact_case = std::env::var_os("OOXMLSDK_GOLDEN_CASE").is_some();
    let audit_errors =
        std::env::var("OOXMLSDK_GOLDEN_AUDIT_ERRORS").is_ok_and(|value| value == "1");
    if exact_case && !audit_errors {
        assert_eq!(report.passed, 1);
        assert_eq!(report.expected_errors, 0);
    } else if exact_case {
        assert_eq!(report.attempted, 1);
    } else if audit_errors {
        assert!(report.attempted > 0);
    } else {
        assert_eq!(report.passed, target);
    }
}

#[test]
#[ignore = "run the streamed Office golden corpus ratchet explicitly"]
fn office_golden_docx_corpus_ratchet() {
    assert_ratchet(OfficeGoldenFormat::Docx, 993);
}

#[test]
#[ignore = "run the streamed Office golden corpus ratchet explicitly"]
fn office_golden_pptx_corpus_ratchet() {
    assert_ratchet(OfficeGoldenFormat::Pptx, 351);
}

#[test]
#[ignore = "run the streamed Office golden corpus ratchet explicitly"]
fn office_golden_xlsx_corpus_ratchet() {
    assert_ratchet(OfficeGoldenFormat::Xlsx, 265);
}

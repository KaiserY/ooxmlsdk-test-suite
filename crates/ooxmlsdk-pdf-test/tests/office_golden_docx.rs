use ooxmlsdk_pdf_test::{OfficeGoldenCase, VisualTolerance, compare_office_golden};

const ENVIRONMENT_ID: &str = "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157";

fn compare(case: OfficeGoldenCase, expected_pages: usize) {
    let report = compare_office_golden(case, VisualTolerance::OFFICE_FIXED_OUTPUT).unwrap();
    assert_eq!(report.page_diffs.len(), expected_pages);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_desktop_qa_data_blank_text() {
    compare(
        OfficeGoldenCase {
            id: "libreoffice_desktop_qa_data_blank_text",
            corpus: "LibreOffice",
            source: "desktop/qa/data/blank_text.docx",
            source_sha256: "aa1cbfb600ab8cfc6958c6da000824affba622168f24835aae14e3a769c3028c",
            golden_sha256: "85226d496255111778525d65705377887e89866052ea968930244288a76c38b5",
            environment_id: ENVIRONMENT_ID,
            ui_language: "zh-CN",
        },
        1,
    );
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sw_qa_extras_ww8export_data_empty_group() {
    compare(
        OfficeGoldenCase {
            id: "libreoffice_sw_qa_extras_ww8export_data_empty_group",
            corpus: "LibreOffice",
            source: "sw/qa/extras/ww8export/data/empty_group.docx",
            source_sha256: "192035f04b51951a57e7954c9a14005bb1644ee84bccfc44d09810ad70392f7b",
            golden_sha256: "15ee2bddb9185dc9ee02007a6fc64a4199d53336286b7e0a7c8736b834041ab6",
            environment_id: ENVIRONMENT_ID,
            ui_language: "zh-CN",
        },
        1,
    );
}

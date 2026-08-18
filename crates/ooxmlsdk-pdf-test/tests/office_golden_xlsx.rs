use ooxmlsdk_pdf_test::{OfficeGoldenCase, VisualTolerance, compare_office_golden};

const ENVIRONMENT_ID: &str = "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157";

fn compare(case: OfficeGoldenCase, expected_pages: usize) {
    let report = compare_office_golden(case, VisualTolerance::OFFICE_FIXED_OUTPUT).unwrap();
    assert_eq!(report.page_diffs.len(), expected_pages);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sc_qa_unit_data_xlsx_tdf135828_shape_rect() {
    compare(
        OfficeGoldenCase {
            id: "libreoffice_sc_qa_unit_data_xlsx_tdf135828_shape_rect",
            corpus: "LibreOffice",
            source: "sc/qa/unit/data/xlsx/tdf135828_Shape_Rect.xlsx",
            source_sha256: "248b67b0dcfa0613fd94447e4b6671f33c7b481bd2d155348f360a1c7df0d63a",
            golden_sha256: "63109d4589d0a5c1b65f1b9f6f1b9b8104fabb2140e9113cf9dfb5b2047c78e9",
            environment_id: ENVIRONMENT_ID,
            ui_language: "zh-CN",
            format_locale: "zh-CN",
        },
        1,
    );
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sc_qa_unit_data_xlsx_tdf169496_hidden_graphic() {
    compare(
        OfficeGoldenCase {
            id: "libreoffice_sc_qa_unit_data_xlsx_tdf169496_hidden_graphic",
            corpus: "LibreOffice",
            source: "sc/qa/unit/data/xlsx/tdf169496_hidden_graphic.xlsx",
            source_sha256: "0b647da300a085f39914fdfae961463ae9e54ffe772b2e0eb9860a841ab93f72",
            golden_sha256: "9a97bf945e9cadf5fbda011bb4c7a4f0190c49c8ca65743f48cb11ab67708f0a",
            environment_id: ENVIRONMENT_ID,
            ui_language: "zh-CN",
            format_locale: "zh-CN",
        },
        1,
    );
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_open_xml_sdk_xlsx_2d_rotation_print_clip() {
    compare(
        OfficeGoldenCase {
            id: "open_xml_sdk_xlsx_2d_rotation_print_clip",
            corpus: "Open-XML-SDK",
            source: "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/O14ISOStrict/Excel/2D Rotation-O12-XL-OartEffects.xlsx",
            source_sha256: "0e0017c70a5362ef3c49be3fb82c3e80210cfda0e813413c2b28a5ee141c0ad3",
            golden_sha256: "4ab5cb3b0982945d7e4acaa14b95b0dad9d37372288c79ae20bd782cc2740b6d",
            environment_id: ENVIRONMENT_ID,
            ui_language: "zh-CN",
            format_locale: "zh-CN",
        },
        1,
    );
}

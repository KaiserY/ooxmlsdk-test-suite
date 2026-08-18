use ooxmlsdk_pdf_test::{canonical_pdf_fragment_snapshot, libreoffice_fixture, render_fixture_pdf};

fn assert_fragment_snapshot(source: &str, expected: &str) {
    let pdf = render_fixture_pdf(&libreoffice_fixture(source)).unwrap();
    let actual = canonical_pdf_fragment_snapshot(&pdf).unwrap();
    assert_eq!(
        actual, expected,
        "PDF fragment snapshot changed for {source}"
    );
}

#[test]
fn docx_text_pdf_fragment_is_stable() {
    assert_fragment_snapshot(
        "sw/qa/extras/ooxmlexport/data/lastEmptyLineWithDirectFormatting.docx",
        include_str!("../snapshots/docx_text_pdf_fragment.json"),
    );
}

#[test]
fn pptx_drawing_pdf_fragment_is_stable() {
    assert_fragment_snapshot(
        "oox/qa/unit/data/Scene3d_orthographicFront.pptx",
        include_str!("../snapshots/pptx_drawing_pdf_fragment.json"),
    );
}

#[test]
fn xlsx_drawing_pdf_fragment_is_stable() {
    assert_fragment_snapshot(
        "sc/qa/unit/data/xlsx/tdf135828_Shape_Rect.xlsx",
        include_str!("../snapshots/xlsx_drawing_pdf_fragment.json"),
    );
}

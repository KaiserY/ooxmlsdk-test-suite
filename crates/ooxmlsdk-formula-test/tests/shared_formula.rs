use ooxmlsdk_formula::{CellAddress, translate_shared_formula_text};

fn address(reference: &str) -> CellAddress {
    CellAddress::parse_a1(reference).unwrap()
}

#[test]
fn translates_relative_references_in_shared_formula_text() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test3.cxx
    // ::testSharedFormulaExportXLSX and sc/qa/unit/ucalc_sharedformula.cxx.
    assert_eq!(
        translate_shared_formula_text("ROUND(B2,12)=ROUND(C2,12)", address("D2"), address("D3")),
        "ROUND(B3,12)=ROUND(C3,12)"
    );
    assert_eq!(
        translate_shared_formula_text("'Input Sheet'!$A1+B$2", address("A1"), address("C4")),
        "'Input Sheet'!$A4+D$2"
    );
}

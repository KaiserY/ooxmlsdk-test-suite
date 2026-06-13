use std::borrow::Cow;
use std::collections::BTreeMap;

use ooxmlsdk_formula::{CellAddress, FormulaEvaluationBook, FormulaValue, SheetBinding, SheetId};

const SHEET: SheetId = SheetId(1);

fn address(reference: &str) -> CellAddress {
    CellAddress::parse_a1(reference).unwrap()
}

fn book(cells: &[(&str, FormulaValue<'static>)]) -> FormulaEvaluationBook<'static> {
    FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Formula"),
        }],
        cells: cells
            .iter()
            .map(|(reference, value)| ((SHEET, address(reference)), value.clone()))
            .collect::<BTreeMap<_, _>>(),
        ..FormulaEvaluationBook::default()
    }
}

fn number(value: Option<FormulaValue<'_>>) -> f64 {
    match value {
        Some(FormulaValue::Number(value)) => value,
        other => panic!("expected number, got {other:?}"),
    }
}

fn text(value: Option<FormulaValue<'_>>) -> String {
    match value {
        Some(FormulaValue::String(value)) => value.into_owned(),
        other => panic!("expected string, got {other:?}"),
    }
}

fn assert_number(book: &FormulaEvaluationBook<'_>, formula: &str, expected: f64) {
    let actual = number(book.evaluate_formula_text(SHEET, None, formula));
    assert!(
        (actual - expected).abs() <= 1e-10,
        "{formula}: expected {expected}, got {actual}"
    );
}

fn string(value: &'static str) -> FormulaValue<'static> {
    FormulaValue::String(Cow::Borrowed(value))
}

#[test]
fn evaluates_count_and_countblank_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncCOUNT and
    // testFuncCOUNTBLANK.
    let book = book(&[
        ("A1", FormulaValue::Number(2.0)),
        ("A2", FormulaValue::Number(4.0)),
        ("A3", FormulaValue::Number(6.0)),
        ("A4", string("B")),
        ("C1", FormulaValue::Blank),
        ("D1", string("")),
    ]);

    assert_number(&book, "COUNT(A1:A3)", 3.0);
    assert_number(&book, "COUNT(A1:A3,2)", 4.0);
    assert_number(&book, "COUNT(A1:A3,2,4)", 5.0);
    assert_number(&book, "COUNT(A1:A3,2,4,6)", 6.0);
    assert_number(&book, "COUNTBLANK(A1:A4)", 0.0);
    assert_number(&book, "COUNTBLANK(C1)", 1.0);
    assert_number(&book, "COUNTBLANK(D1)", 1.0);
}

#[test]
fn evaluates_sum_and_product_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncSUM and
    // testFuncPRODUCT.
    let sum_book = book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(22.0)),
        ("A3", FormulaValue::Number(4.0)),
        ("A4", FormulaValue::Number(5.0)),
        ("A5", FormulaValue::Number(6.0)),
        ("B1", FormulaValue::Number(3.0)),
        ("B2", FormulaValue::Number(4.0)),
        ("B3", FormulaValue::Number(5.0)),
        ("B4", FormulaValue::Number(6.0)),
        ("B5", FormulaValue::Number(7.0)),
        ("C1", FormulaValue::Number(4.0)),
        ("C2", FormulaValue::Number(8.0)),
        ("C3", FormulaValue::Number(-0.125)),
    ]);

    assert_number(&sum_book, "SUM(A1:A2,B1:B2)", 30.0);
    assert_number(&sum_book, "SUM(A2:A3,B2:B3)", 35.0);
    assert_number(&sum_book, "SUM(A3:A4,B3:B4)", 20.0);
    assert_number(&sum_book, "SUM(0.1,0.2,-0.3)", 0.0);
    assert_number(&sum_book, "0.1+0.2-0.3", 0.0);

    let product_book = book(&[
        ("A1", FormulaValue::Number(-3.0)),
        ("B1", FormulaValue::Number(10.0)),
        ("C1", FormulaValue::Number(4.0)),
        ("A2", FormulaValue::Number(-2.0)),
        ("B2", FormulaValue::Number(-1.0)),
        ("C2", FormulaValue::Number(8.0)),
        ("A3", FormulaValue::Number(0.2)),
        ("B3", FormulaValue::Number(-0.25)),
        ("C3", FormulaValue::Number(-0.125)),
    ]);

    assert_number(&product_book, "PRODUCT(A1)", -3.0);
    assert_number(&product_book, "PRODUCT(A1:C3)", -12.0);
    assert_number(&product_book, "PRODUCT({2;3;4})", 24.0);
    assert_number(&product_book, "PRODUCT({2;-2;2})", -8.0);
    assert_number(&product_book, "PRODUCT({8;0.125;-1})", -1.0);
}

#[test]
fn evaluates_sumproduct_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncSUMPRODUCT.
    let book = book(&[
        ("B1", FormulaValue::Number(1.0)),
        ("B2", FormulaValue::Number(2.0)),
        ("B3", FormulaValue::Number(5.0)),
        ("C1", FormulaValue::Number(1.0)),
        ("C2", FormulaValue::Number(3.0)),
        ("C3", FormulaValue::Number(-2.0)),
        ("E1", FormulaValue::Number(-3.0)),
        ("E2", FormulaValue::Number(4.0)),
    ]);

    assert_number(&book, "SUMPRODUCT(B1:B3,C1:C3)", -3.0);
    assert_number(&book, "SUMPRODUCT(ABS(E1:E2),E1:E2+E1:E2)", 14.0);
}

#[test]
fn evaluates_if_choose_and_iferror_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncIF,
    // testFuncCHOOSE, and testFuncIFERROR.
    let book = book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", string("e")),
        ("A3", FormulaValue::Number(2.0)),
        (
            "A4",
            ooxmlsdk_formula::FormulaValue::Error(ooxmlsdk_formula::FormulaErrorValue::Num),
        ),
        (
            "A5",
            ooxmlsdk_formula::FormulaValue::Error(ooxmlsdk_formula::FormulaErrorValue::Num),
        ),
        (
            "A6",
            ooxmlsdk_formula::FormulaValue::Error(ooxmlsdk_formula::FormulaErrorValue::Div0),
        ),
        (
            "A7",
            ooxmlsdk_formula::FormulaValue::Error(ooxmlsdk_formula::FormulaErrorValue::NA),
        ),
        ("B1", FormulaValue::Number(2.0)),
    ]);

    assert_eq!(
        text(book.evaluate_formula_text(SHEET, None, "IF(B1=2,\"two\",\"not two\")")),
        "two"
    );
    assert_eq!(
        text(book.evaluate_formula_text(SHEET, None, "CHOOSE(2,\"one\",\"two\",\"three\")")),
        "two"
    );
    assert_number(&book, "IFERROR(A1,9)", 1.0);
    assert_eq!(
        text(book.evaluate_formula_text(SHEET, None, "IFERROR(A2,9)")),
        "e"
    );
    assert_number(&book, "IFERROR(A3,9)", 2.0);
    assert_number(&book, "IFERROR(A4,-7)", -7.0);
    assert_number(&book, "IFERROR(A5,-7)", -7.0);
    assert_number(&book, "IFERROR(A6,-7)", -7.0);
    assert_number(&book, "IFERROR(A7,-7)", -7.0);
    assert_number(&book, "IFNA(A7,-7)", -7.0);
}

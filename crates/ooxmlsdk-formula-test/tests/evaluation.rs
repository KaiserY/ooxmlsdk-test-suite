use std::borrow::Cow;
use std::collections::BTreeMap;

use ooxmlsdk_formula::{
    CellAddress, DefinedNameKey, FormulaErrorValue, FormulaEvaluationBook, FormulaGrammar,
    FormulaKind, FormulaParseContext, FormulaRowState, FormulaText, FormulaValue, SheetBinding,
    SheetId, parse_formula_with_context,
};

const SHEET: SheetId = SheetId(1);

fn address(reference: &str) -> CellAddress {
    CellAddress::parse_a1(reference).unwrap()
}

fn evaluation_book(cells: &[(&str, FormulaValue<'static>)]) -> FormulaEvaluationBook<'static> {
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

fn assert_number_with_epsilon(
    book: &FormulaEvaluationBook<'_>,
    formula: &str,
    expected: f64,
    epsilon: f64,
) {
    let actual = number(book.evaluate_formula_text(SHEET, None, formula));
    assert!(
        (actual - expected).abs() <= epsilon,
        "{formula}: expected {expected}, got {actual}"
    );
}

fn assert_number_at(
    book: &FormulaEvaluationBook<'_>,
    current_cell: &str,
    formula: &str,
    expected: f64,
) {
    let current_cell = address(current_cell);
    let actual = number(book.evaluate_formula_text(SHEET, Some(current_cell), formula));
    assert!(
        (actual - expected).abs() <= 1e-10,
        "{formula}: expected {expected}, got {actual}"
    );
}

fn assert_text(book: &FormulaEvaluationBook<'_>, formula: &str, expected: &str) {
    assert_eq!(
        text(book.evaluate_formula_text(SHEET, None, formula)),
        expected,
        "{formula}"
    );
}

fn assert_text_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    formula: &str,
    grammar: FormulaGrammar,
    expected: &str,
) {
    assert_eq!(
        text(book.evaluate_formula_text_with_grammar(SHEET, None, formula, grammar)),
        expected,
        "{formula}"
    );
}

fn assert_text_at_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    sheet: SheetId,
    formula: &str,
    grammar: FormulaGrammar,
    expected: &str,
) {
    assert_eq!(
        text(book.evaluate_formula_text_with_grammar(sheet, None, formula, grammar)),
        expected,
        "{formula}"
    );
}

fn assert_number_at_sheet_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    sheet: SheetId,
    formula: &str,
    grammar: FormulaGrammar,
    expected: f64,
) {
    let actual = number(book.evaluate_formula_text_with_grammar(sheet, None, formula, grammar));
    assert!(
        (actual - expected).abs() <= 1e-10,
        "{formula}: expected {expected}, got {actual}"
    );
}

fn assert_error_at_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    sheet: SheetId,
    formula: &str,
    grammar: FormulaGrammar,
    expected: FormulaErrorValue,
) {
    assert_eq!(
        book.evaluate_formula_text_with_grammar(sheet, None, formula, grammar),
        Some(FormulaValue::Error(expected)),
        "{formula}"
    );
}

fn assert_error(book: &FormulaEvaluationBook<'_>, formula: &str, expected: FormulaErrorValue) {
    assert_eq!(
        book.evaluate_formula_text(SHEET, None, formula),
        Some(FormulaValue::Error(expected)),
        "{formula}"
    );
}

fn assert_boolean(book: &FormulaEvaluationBook<'_>, formula: &str, expected: bool) {
    assert_eq!(
        book.evaluate_formula_text(SHEET, None, formula),
        Some(FormulaValue::Boolean(expected)),
        "{formula}"
    );
}

fn raw_formula_value<'a>(
    book: &'a FormulaEvaluationBook<'a>,
    sheet: SheetId,
    current_cell: Option<&str>,
    formula: &'a str,
    grammar: FormulaGrammar,
    array_context: bool,
) -> Option<FormulaValue<'a>> {
    let current_cell = current_cell.map(address);
    let parsed = parse_formula_with_context(
        FormulaParseContext {
            current_sheet: sheet,
            current_cell,
            grammar,
        },
        Cow::Borrowed(formula),
    );
    book.evaluate_parsed_formula_raw(sheet, current_cell, &parsed, array_context)
}

fn assert_matrix_numbers_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    sheet: SheetId,
    current_cell: Option<&str>,
    formula: &str,
    grammar: FormulaGrammar,
    expected: &[&[f64]],
) {
    let actual = raw_formula_value(book, sheet, current_cell, formula, grammar, true);
    let Some(FormulaValue::Matrix(rows)) = actual else {
        panic!("{formula}: expected matrix, got {actual:?}");
    };
    assert_eq!(rows.len(), expected.len(), "{formula}: row count");
    for (row_index, (row, expected_row)) in rows.iter().zip(expected).enumerate() {
        assert_eq!(
            row.len(),
            expected_row.len(),
            "{formula}: column count in row {row_index}"
        );
        for (column_index, (value, expected)) in row.iter().zip(*expected_row).enumerate() {
            let FormulaValue::Number(actual) = value else {
                panic!("{formula}: expected number at {row_index},{column_index}, got {value:?}");
            };
            assert!(
                (actual - expected).abs() <= 1e-10,
                "{formula}: expected {expected} at {row_index},{column_index}, got {actual}"
            );
        }
    }
}

fn assert_raw_number_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    sheet: SheetId,
    current_cell: Option<&str>,
    formula: &str,
    grammar: FormulaGrammar,
    expected: f64,
) {
    let actual = raw_formula_value(book, sheet, current_cell, formula, grammar, true);
    let actual = number(actual);
    assert!(
        (actual - expected).abs() <= 1e-10,
        "{formula}: expected {expected}, got {actual}"
    );
}

fn assert_matrix_texts_with_grammar(
    book: &FormulaEvaluationBook<'_>,
    sheet: SheetId,
    current_cell: Option<&str>,
    formula: &str,
    grammar: FormulaGrammar,
    expected: &[&[&str]],
) {
    let actual = raw_formula_value(book, sheet, current_cell, formula, grammar, true);
    let Some(FormulaValue::Matrix(rows)) = actual else {
        panic!("{formula}: expected matrix, got {actual:?}");
    };
    assert_eq!(rows.len(), expected.len(), "{formula}: row count");
    for (row_index, (row, expected_row)) in rows.iter().zip(expected).enumerate() {
        assert_eq!(
            row.len(),
            expected_row.len(),
            "{formula}: column count in row {row_index}"
        );
        for (column_index, (value, expected)) in row.iter().zip(*expected_row).enumerate() {
            let FormulaValue::String(actual) = value else {
                panic!("{formula}: expected string at {row_index},{column_index}, got {value:?}");
            };
            assert_eq!(
                actual, expected,
                "{formula}: text at {row_index},{column_index}"
            );
        }
    }
}

fn string(value: &'static str) -> FormulaValue<'static> {
    FormulaValue::String(Cow::Borrowed(value))
}

#[test]
fn evaluates_count_and_countblank_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncCOUNT and
    // testFuncCOUNTBLANK.
    let book = evaluation_book(&[
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
    let sum_book = evaluation_book(&[
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

    let product_book = evaluation_book(&[
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
    assert_number(&product_book, "PRODUCT(B1)", 10.0);
    assert_number(&product_book, "PRODUCT(A1:C3)", -12.0);
    assert_number(&product_book, "PRODUCT({2;3;4})", 24.0);
    assert_number(&product_book, "PRODUCT({2;-2;2})", -8.0);
    assert_number(&product_book, "PRODUCT({8;0.125;-1})", -1.0);
    assert_number(&product_book, "PRODUCT({2;3},{4;5})", 120.0);
    assert_number(
        &product_book,
        "PRODUCT({10;-8},{3;-1},{15;30},{7})",
        756000.0,
    );
    assert_number(
        &product_book,
        "PRODUCT({10;-0.1;8},{0.125;4;0.25;2},{0.5},{1},{-1})",
        1.0,
    );
}

#[test]
fn evaluates_sumproduct_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncSUMPRODUCT.
    let book = evaluation_book(&[
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
    let book = evaluation_book(&[
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
    assert_text(&book, "CHOOSE(1,\"one\",\"two\",\"three\")", "one");
    assert_text(&book, "CHOOSE(2,\"one\",\"two\",\"three\")", "two");
    assert_text(&book, "CHOOSE(3,\"one\",\"two\",\"three\")", "three");
    assert_error(
        &book,
        "CHOOSE(4,\"one\",\"two\",\"three\")",
        FormulaErrorValue::Value,
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
    assert_error(&book, "IFNA(A6,9)", FormulaErrorValue::Div0);
    assert_number(&book, "IFNA(A7,-7)", -7.0);
}

#[test]
fn evaluates_row_and_column_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncCOLUMN and testFuncROW.
    let book = evaluation_book(&[]);

    assert_number_at(&book, "F11", "COLUMN()", 6.0);
    assert_number_at(&book, "F11", "ROW()", 11.0);
    assert_number(&book, "ROW(A5)", 5.0);
    assert_number(&book, "ROW(B5)", 5.0);
    assert_number(&book, "ROW(B6)", 6.0);
}

#[test]
fn evaluates_n_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncN.
    let book = evaluation_book(&[
        ("A1", FormulaValue::Number(0.0)),
        ("A2", FormulaValue::Blank),
        ("A3", string("Text")),
        ("A4", FormulaValue::Number(1.0)),
        ("A5", FormulaValue::Number(-1.0)),
        ("A6", FormulaValue::Number(12.3)),
        ("A7", string("12.3")),
    ]);

    for (formula, expected) in [
        ("N(A1)", 0.0),
        ("N(A2)", 0.0),
        ("N(A3)", 0.0),
        ("N(A4)", 1.0),
        ("N(A5)", -1.0),
        ("N(A6)", 12.3),
        ("N(A7)", 0.0),
        ("N(A9)", 0.0),
        ("N(0)", 0.0),
        ("N(1)", 1.0),
        ("N(-1)", -1.0),
        ("N(123)", 123.0),
        ("N(\"\")", 0.0),
        ("N(\"12\")", 0.0),
        ("N(\"foo\")", 0.0),
    ] {
        assert_number(&book, formula, expected);
    }
}

#[test]
fn evaluates_countif_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncCOUNTIF and
    // ucalc_formula2.cxx::testFuncCOUNTIFEmpty.
    let book = evaluation_book(&[
        ("A1", FormulaValue::Number(1999.0)),
        ("A2", FormulaValue::Number(2000.0)),
        ("A3", FormulaValue::Number(0.0)),
        ("A4", FormulaValue::Number(0.0)),
        ("A5", FormulaValue::Number(0.0)),
        ("A6", FormulaValue::Number(2002.0)),
        ("A7", FormulaValue::Number(2001.0)),
        ("A8", string("X")),
        ("A9", FormulaValue::Number(2002.0)),
        ("A10", FormulaValue::Blank),
        ("A11", FormulaValue::Blank),
        ("A12", FormulaValue::Blank),
    ]);

    for (formula, expected) in [
        ("COUNTIF(A1:A12,1999)", 1.0),
        ("COUNTIF(A1:A12,2002)", 2.0),
        ("COUNTIF(A1:A12,1998)", 0.0),
        ("COUNTIF(A1:A12,\">=1999\")", 5.0),
        ("COUNTIF(A1:A12,\">1999\")", 4.0),
        ("COUNTIF(A1:A12,\"<2001\")", 5.0),
        ("COUNTIF(A1:A12,\">0\")", 5.0),
        ("COUNTIF(A1:A12,\">=0\")", 8.0),
        ("COUNTIF(A1:A12,0)", 3.0),
        ("COUNTIF(A1:A12,\"X\")", 1.0),
        ("COUNTIF(A1:A12,)", 3.0),
    ] {
        assert_number(&book, formula, expected);
    }

    let empty_string_book = evaluation_book(&[
        ("A1", string("")),
        ("A2", string("")),
        ("A3", string("")),
        ("A4", string("")),
    ]);
    assert_number(&empty_string_book, "COUNTIF(A1:A4,\"\")", 4.0);
    assert_number(&empty_string_book, "COUNTIF(A1,1)", 0.0);
}

#[test]
fn evaluates_sumx_and_sumsq_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncSUMXMY2 and
    // ucalc_formula2.cxx::testFuncSUMX2PY2, testFuncSUMX2MY2, testFuncSUMSQ.
    let sumxmy2_book = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("B1", FormulaValue::Number(1.0)),
        ("C1", FormulaValue::Number(-1.0)),
        ("B2", FormulaValue::Number(2.0)),
        ("C2", FormulaValue::Number(3.0)),
        ("B3", FormulaValue::Number(3.0)),
        ("C3", FormulaValue::Number(1.0)),
    ]);

    assert_number(&sumxmy2_book, "SUMXMY2(B1:B3,C1:C3)", 9.0);
    assert_number(&sumxmy2_book, "SUMXMY2({2;3;4},{4;3;2})", 8.0);

    let book = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("B1", FormulaValue::Number(2.0)),
        ("C1", FormulaValue::Number(3.0)),
        ("D1", FormulaValue::Number(2.0)),
        ("E1", FormulaValue::Number(0.0)),
        ("F1", FormulaValue::Number(3.0)),
        ("A2", FormulaValue::Number(10.0)),
        ("B2", FormulaValue::Number(-5.0)),
        ("C2", FormulaValue::Number(0.0)),
        ("D2", FormulaValue::Number(-10.0)),
        ("E2", FormulaValue::Number(-5.0)),
        ("F2", FormulaValue::Number(0.0)),
        ("A3", FormulaValue::Number(-8.0)),
        ("B3", FormulaValue::Number(0.0)),
        ("C3", FormulaValue::Number(1.0)),
        ("D3", FormulaValue::Number(8.0)),
        ("E3", FormulaValue::Number(0.0)),
        ("F3", FormulaValue::Number(1.0)),
    ]);

    assert_number(&book, "SUMX2PY2(A1:C3,D1:F3)", 407.0);
    assert_number(&book, "SUMX2PY2({1;2;3},{2;3;4})", 43.0);
    assert_number(&book, "SUMX2MY2({1;3;5},{0;4;4})", 3.0);
    assert_number(&book, "SUMX2MY2({1;-3;-5},{0;-4;4})", 3.0);
    assert_number(&book, "SUMX2MY2({9;5;1},{3;-3;3})", 80.0);
    let sumsq_book = evaluation_book(&[
        ("A1", FormulaValue::Number(-1.0)),
        ("A2", FormulaValue::Number(-2.0)),
        ("A3", FormulaValue::Number(6.0)),
        ("B1", FormulaValue::Number(3.0)),
        ("B2", FormulaValue::Number(-4.0)),
        ("B3", FormulaValue::Number(0.0)),
        ("C1", FormulaValue::Number(-5.0)),
        ("C2", FormulaValue::Number(3.0)),
        ("C3", FormulaValue::Number(2.0)),
    ]);
    assert_number(&sumsq_book, "SUMSQ(A1:C3)", 104.0);
    assert_number(&book, "SUMSQ({1;2;3})", 14.0);
    assert_number(&book, "SUMSQ({3;6;9})", 126.0);
    assert_number(&book, "SUMSQ({15;0})", 225.0);
    assert_number(&book, "SUMSQ({-3;3;1})", 19.0);
    assert_number(&book, "SUMSQ({2;3},{4;5})", 54.0);
    assert_number(&book, "SUMSQ({-3;3;1},{-1})", 20.0);
    assert_number(&book, "SUMSQ({-4},{1;4;2},{-5;7},{9})", 192.0);
    assert_number(&book, "SUMSQ({-2;2},{1},{-1},{0;0;0;4})", 26.0);
    assert_number(&book, "SUMSQ(4,1,-3)", 26.0);
    assert_number(&book, "SUMSQ(0,5,13,-7,-4)", 259.0);
    assert_number(&book, "SUMSQ(0,12,24,36,48,60)", 7920.0);
    assert_number(&book, "SUMSQ(0,-12,-24,36,-48,60)", 7920.0);
}

#[test]
fn evaluates_gcd_and_lcm_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncGCD and testFuncLCM.
    let book = evaluation_book(&[]);

    for (formula, expected) in [
        ("GCD({3;6;9})", 3.0),
        ("GCD({150;0})", 150.0),
        ("GCD({6;6;6},{3;6;9})", 3.0),
        ("GCD({300;300;300},{150;0})", 150.0),
        ("GCD(12,24,36,48,60)", 12.0),
        ("GCD(0,12,24,36,48,60)", 12.0),
        ("LCM({3;6;9})", 18.0),
        ("LCM({150;0})", 0.0),
        ("LCM({6;6;6},{3;6;9})", 18.0),
        ("LCM({300;300;300},{150;0})", 0.0),
        ("LCM(12,24,36,48,60)", 720.0),
        ("LCM(0,12,24,36,48,60)", 0.0),
    ] {
        assert_number(&book, formula, expected);
    }
}

#[test]
fn evaluates_lookup_match_and_datedif_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncVLOOKUP,
    // testFuncMATCH, and testFuncDATEDIF.
    let book = evaluation_book(&[
        ("A1", string("Key")),
        ("B1", string("Val")),
        ("A2", FormulaValue::Number(10.0)),
        ("B2", FormulaValue::Number(3.0)),
        ("A3", FormulaValue::Number(20.0)),
        ("B3", FormulaValue::Number(4.0)),
        ("A4", FormulaValue::Number(30.0)),
        ("B4", FormulaValue::Number(5.0)),
        ("A5", FormulaValue::Number(40.0)),
        ("B5", FormulaValue::Number(6.0)),
        ("A6", FormulaValue::Number(50.0)),
        ("B6", FormulaValue::Number(7.0)),
        ("A7", FormulaValue::Number(60.0)),
        ("B7", FormulaValue::Number(8.0)),
        ("A8", FormulaValue::Number(70.0)),
        ("B8", FormulaValue::Number(9.0)),
        ("A9", string("B")),
        ("B9", FormulaValue::Number(10.0)),
        ("A10", string("B")),
        ("B10", FormulaValue::Number(11.0)),
        ("A11", string("C")),
        ("B11", FormulaValue::Number(12.0)),
        ("A12", string("D")),
        ("B12", FormulaValue::Number(13.0)),
        ("A13", string("E")),
        ("B13", FormulaValue::Number(14.0)),
        ("A14", string("F")),
        ("B14", FormulaValue::Number(15.0)),
    ]);

    for (lookup, expected) in [
        ("12", 3.0),
        ("29", 4.0),
        ("31", 5.0),
        ("45", 6.0),
        ("56", 7.0),
        ("65", 8.0),
        ("78", 9.0),
        ("100", 9.0),
        ("1000", 9.0),
    ] {
        assert_number(&book, &format!("VLOOKUP({lookup},A2:B14,2,TRUE)"), expected);
    }
    assert_error(
        &book,
        "VLOOKUP(\"Andy\",A2:B14,2,TRUE)",
        FormulaErrorValue::NA,
    );
    assert_number(&book, "VLOOKUP(\"Bruce\",A2:B14,2,TRUE)", 11.0);
    assert_number(&book, "VLOOKUP(\"Charlie\",A2:B14,2,TRUE)", 12.0);
    assert_number(&book, "VLOOKUP(\"David\",A2:B14,2,TRUE)", 13.0);
    assert_number(&book, "VLOOKUP(\"Edward\",A2:B14,2,TRUE)", 14.0);
    assert_number(&book, "VLOOKUP(\"Frank\",A2:B14,2,TRUE)", 15.0);
    assert_number(&book, "VLOOKUP(\"Henry\",A2:B14,2,TRUE)", 15.0);
    assert_number(&book, "VLOOKUP(\"Zena\",A2:B14,2,TRUE)", 15.0);

    let match_book = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(2.0)),
        ("A3", FormulaValue::Number(3.0)),
        ("A4", FormulaValue::Number(4.0)),
        ("A5", FormulaValue::Number(5.0)),
        ("A6", FormulaValue::Number(6.0)),
        ("A7", FormulaValue::Number(7.0)),
        ("A8", FormulaValue::Number(8.0)),
        ("A9", FormulaValue::Number(9.0)),
        ("A10", string("B")),
        ("A11", string("B")),
        ("A12", string("C")),
    ]);
    for (lookup, expected) in [
        ("1.2", 1.0),
        ("2.3", 2.0),
        ("3.9", 3.0),
        ("4.1", 4.0),
        ("5.99", 5.0),
        ("6.1", 6.0),
        ("7.2", 7.0),
        ("8.569", 8.0),
        ("9.59", 9.0),
        ("10", 9.0),
        ("100", 9.0),
    ] {
        assert_number(&match_book, &format!("MATCH({lookup},A1:A12,1)"), expected);
    }
    assert_error(&match_book, "MATCH(0.8,A1:A12,1)", FormulaErrorValue::NA);
    assert_error(
        &match_book,
        "MATCH(\"Andy\",A1:A12,1)",
        FormulaErrorValue::NA,
    );
    assert_number(&match_book, "MATCH(\"Bruce\",A1:A12,1)", 11.0);
    assert_number(&match_book, "MATCH(\"Charlie\",A1:A12,1)", 12.0);

    for (formula, expected) in [
        ("DATEDIF(DATE(2007,1,1),DATE(2007,1,10),\"d\")", 9.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2007,1,31),\"m\")", 0.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2007,2,1),\"m\")", 1.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2007,12,31),\"d\")", 364.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2007,1,31),\"y\")", 0.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2008,7,1),\"d\")", 547.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2008,7,1),\"m\")", 18.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2008,7,1),\"ym\")", 6.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2008,7,1),\"yd\")", 182.0),
        ("DATEDIF(DATE(2008,1,1),DATE(2009,7,1),\"yd\")", 181.0),
        ("DATEDIF(DATE(2007,1,1),DATE(2007,1,31),\"md\")", 30.0),
        ("DATEDIF(DATE(2007,2,1),DATE(2009,3,1),\"md\")", 0.0),
        ("DATEDIF(DATE(2008,2,1),DATE(2009,3,1),\"md\")", 0.0),
    ] {
        assert_number(&book, formula, expected);
    }
}

#[test]
fn evaluates_indirect_reference_syntax_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncINDIRECT.
    let foo = SheetId(1);
    let book = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: foo,
            name: Cow::Borrowed("foo"),
        }],
        cells: BTreeMap::from([((foo, address("A11")), string("Test"))]),
        ..FormulaEvaluationBook::default()
    };

    assert_text_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo.A11\")",
        FormulaGrammar::CalcA1,
        "Test",
    );
    assert_error_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!A11\")",
        FormulaGrammar::CalcA1,
        FormulaErrorValue::Ref,
    );
    assert_error_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!R11C1\")",
        FormulaGrammar::CalcA1,
        FormulaErrorValue::Ref,
    );
    assert_text_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!R11C1\";0)",
        FormulaGrammar::CalcA1,
        "Test",
    );

    assert_error_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo.A11\")",
        FormulaGrammar::ExcelA1,
        FormulaErrorValue::Ref,
    );
    assert_text_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!A11\")",
        FormulaGrammar::ExcelA1,
        "Test",
    );
    assert_error_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!R11C1\")",
        FormulaGrammar::ExcelA1,
        FormulaErrorValue::Ref,
    );
    assert_text_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!R11C1\",0)",
        FormulaGrammar::ExcelA1,
        "Test",
    );

    assert_error_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo.A11\")",
        FormulaGrammar::ExcelR1C1,
        FormulaErrorValue::Ref,
    );
    assert_error_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!A11\")",
        FormulaGrammar::ExcelR1C1,
        FormulaErrorValue::Ref,
    );
    assert_text_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!R11C1\")",
        FormulaGrammar::ExcelR1C1,
        "Test",
    );
    assert_text_at_with_grammar(
        &book,
        foo,
        "INDIRECT(\"foo!R11C1\",0)",
        FormulaGrammar::ExcelR1C1,
        "Test",
    );
}

#[test]
fn evaluates_match_indirect_without_array_context_propagation() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFunc_MATCH_INDIRECT.
    let foo = SheetId(1);
    let book = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: foo,
            name: Cow::Borrowed("foo"),
        }],
        cells: BTreeMap::from([((foo, address("D6")), string("Test1"))]),
        defined_names: BTreeMap::from([(
            DefinedNameKey {
                sheet: None,
                name_upper: "ROLEASSIGNMENT".to_string(),
            },
            Cow::Borrowed("$D$4:$D$13"),
        )]),
        ..FormulaEvaluationBook::default()
    };

    assert_number_at_sheet_with_grammar(
        &book,
        foo,
        "MATCH(\"Test1\";INDIRECT(ADDRESS(ROW(RoleAssignment)+1;COLUMN(RoleAssignment))&\":\"&ADDRESS(ROW(RoleAssignment)+ROWS(RoleAssignment)-1;COLUMN(RoleAssignment)));0)",
        FormulaGrammar::CalcA1,
        2.0,
    );
}

#[test]
fn evaluates_numbervalue_and_len_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncNUMBERVALUE and testFuncLEN.
    let book = evaluation_book(&[
        ("A1", string("1ag9a9b9")),
        ("A2", string("1ag34 5g g6  78b9%%")),
        ("A3", string("1 234d56E-2")),
        ("A4", string("d4")),
        ("A5", string("54.4")),
        ("A6", string("1a2b3e1%")),
        ("B1", FormulaValue::Blank),
        ("B2", FormulaValue::Blank),
        ("B3", FormulaValue::Blank),
    ]);

    assert_number(&book, "NUMBERVALUE(A1,\"b\",\"ag\")", 199.9);
    assert_number(&book, "NUMBERVALUE(A2,\"b\",\"ag\")", 134.56789);
    assert_error(
        &book,
        "NUMBERVALUE(A2,\"b\",\"g\")",
        FormulaErrorValue::Value,
    );
    assert_number(&book, "NUMBERVALUE(A3,\"d\")", 12.3456);
    assert_number(&book, "NUMBERVALUE(A4,\"d\",\"foo\")", 0.4);
    assert_error(
        &book,
        "NUMBERVALUE(A4,)",
        FormulaErrorValue::IllegalArgument,
    );
    assert_error(
        &book,
        "NUMBERVALUE(A5,)",
        FormulaErrorValue::IllegalArgument,
    );
    assert_number(&book, "NUMBERVALUE(A6,\"b\",\"a\")", 1.23);
    assert_number(&book, "LEN(B1)", 0.0);
    assert_number(&book, "LEN(B2)", 0.0);
    assert_number(&book, "LEN(B3)", 0.0);
}

#[test]
fn evaluates_countifs_empty_range_reduce_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncCOUNTIFSRangeReduce.
    let mut cells = BTreeMap::new();
    for (reference, value) in [
        ("A2", string("a")),
        ("B2", FormulaValue::Number(1.0)),
        ("C2", FormulaValue::Number(1.0)),
        ("A3", string("b")),
        ("B3", FormulaValue::Number(2.0)),
        ("C3", FormulaValue::Number(2.0)),
        ("A4", string("c")),
        ("B4", FormulaValue::Number(4.0)),
        ("C4", FormulaValue::Number(3.0)),
        ("A5", string("d")),
        ("B5", FormulaValue::Number(8.0)),
        ("C5", FormulaValue::Number(4.0)),
        ("A6", string("a")),
        ("B6", FormulaValue::Number(16.0)),
        ("C6", FormulaValue::Number(5.0)),
        ("A8", string("b")),
        ("C8", FormulaValue::Number(6.0)),
        ("A9", string("c")),
        ("B9", FormulaValue::Number(64.0)),
        ("C9", FormulaValue::Number(7.0)),
        ("K2", string("")),
    ] {
        cells.insert((SHEET, address(reference)), value);
    }
    let book = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Test"),
        }],
        cells,
        ..FormulaEvaluationBook::default()
    };

    assert_number(
        &book,
        "COUNTIFS($A1:$A21,\"\",$B1:$B21,\"\",$C1:$C21,\"\")",
        14.0,
    );
    assert_number(&book, "COUNTIFS($A1:$A21,A8,$B1:$B21,K2,$C1:$C21,C8)", 1.0);
}

#[test]
fn evaluates_query_empty_cell_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testHoriQueryEmptyCell and
    // testVertQueryEmptyCell.
    let mut cells = BTreeMap::from([
        ((SHEET, address("A1")), string("x")),
        ((SHEET, address("B1")), string("y")),
        ((SHEET, address("C1")), string("z")),
    ]);
    let mut query_empty_cells = std::collections::BTreeSet::new();
    for reference in ["D1", "E1", "F1", "G1", "H1"] {
        query_empty_cells.insert((SHEET, address(reference)));
    }
    let horizontal = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Test"),
        }],
        cells: cells.clone(),
        query_empty_cells,
        ..FormulaEvaluationBook::default()
    };
    assert_number(&horizontal, "COUNTIF(A1:H1,\"=\")", 5.0);
    assert_text(
        &horizontal,
        "CELL(\"ADDRESS\",XLOOKUP(,A1:H1,A1:H1))",
        "$D$1",
    );
    assert_number(&horizontal, "COUNTIF(A1:H1,\"<>y\")", 7.0);

    cells.clear();
    for (reference, value) in [
        ("A1", string("a")),
        ("A2", string("b")),
        ("A4", string("d")),
        ("A8", string("h")),
    ] {
        cells.insert((SHEET, address(reference)), value);
    }
    for row in 1..=10 {
        cells.insert(
            (SHEET, address(&format!("B{row}"))),
            FormulaValue::Number(f64::from(row)),
        );
    }
    let mut query_empty_cells = std::collections::BTreeSet::new();
    for reference in ["A3", "A5", "A6", "A7", "A9", "A10"] {
        query_empty_cells.insert((SHEET, address(reference)));
    }
    let vertical = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Test"),
        }],
        cells,
        query_empty_cells,
        ..FormulaEvaluationBook::default()
    };
    assert_number(&vertical, "COUNTIFS(A1:A10,\"=\",B1:B10,\">0\")", 6.0);
    assert_number(&vertical, "COUNTIFS($B1:$B10,\">0\",A1:A10,\"=\")", 6.0);
}

#[test]
fn evaluates_xlookup_regex_match_case() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testRegexForXLOOKUP.
    let book = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Test1"),
        }],
        cells: BTreeMap::from([
            ((SHEET, address("A2")), string("Hydrogen")),
            ((SHEET, address("B2")), FormulaValue::Number(1.008)),
            ((SHEET, address("A3")), string("Helium")),
            ((SHEET, address("B3")), FormulaValue::Number(4.003)),
            ((SHEET, address("A4")), string("Lithium")),
            ((SHEET, address("B4")), FormulaValue::Number(6.94)),
            ((SHEET, address("A5")), string("Beryllium")),
            ((SHEET, address("B5")), FormulaValue::Number(9.012)),
            ((SHEET, address("A6")), string("Boron")),
            ((SHEET, address("B6")), FormulaValue::Number(10.81)),
            ((SHEET, address("A7")), string("Carbon")),
            ((SHEET, address("B7")), FormulaValue::Number(12.011)),
            ((SHEET, address("A8")), string("Nitrogen")),
            ((SHEET, address("B8")), FormulaValue::Number(14.007)),
            ((SHEET, address("A9")), string("Oxygen")),
            ((SHEET, address("B9")), FormulaValue::Number(15.999)),
            ((SHEET, address("A10")), string("Florine")),
            ((SHEET, address("B10")), FormulaValue::Number(18.998)),
            ((SHEET, address("A11")), string("Neon")),
            ((SHEET, address("B11")), FormulaValue::Number(20.18)),
            ((SHEET, address("E15")), string("^bo.*")),
        ]),
        ..FormulaEvaluationBook::default()
    };

    assert_number(&book, "XLOOKUP(E15,A$2:A$11,B$2:B$11,,3)", 10.81);
}

#[test]
fn evaluates_single_value_operator_case() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testSingleValueOperator.
    let book = evaluation_book(&[]);

    assert_number(&book, "@SEQUENCE(4)", 1.0);
}

#[test]
fn evaluates_sheet_count_and_sheet_index_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncSHEET.
    let book = FormulaEvaluationBook {
        sheet_names: vec![
            SheetBinding {
                id: SheetId(1),
                name: Cow::Borrowed("test1"),
            },
            SheetBinding {
                id: SheetId(2),
                name: Cow::Borrowed("test2"),
            },
            SheetBinding {
                id: SheetId(3),
                name: Cow::Borrowed("test3"),
            },
        ],
        cells: BTreeMap::from([((SheetId(2), address("C2")), FormulaValue::Number(42.0))]),
        ..FormulaEvaluationBook::default()
    };

    assert_number(&book, "SHEETS()", 3.0);
    assert_number(&book, "SHEET(test1!A1)", 1.0);
    assert_number(&book, "SHEET(test2!C2)", 2.0);
    assert_number(&book, "SHEET(test3!A1)", 3.0);
    assert_number(&book, "CELL(\"SHEET\",test2!C2)", 2.0);
}

#[test]
fn evaluates_range_operator_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncRangeOp.
    let sheet1 = SheetId(1);
    let sheet2 = SheetId(2);
    let sheet3 = SheetId(3);
    let mut cells = BTreeMap::new();
    for (sheet, values) in [
        (sheet1, [1.0, 2.0, 4.0]),
        (sheet2, [8.0, 16.0, 32.0]),
        (sheet3, [64.0, 128.0, 256.0]),
    ] {
        for (row, value) in values.into_iter().enumerate() {
            cells.insert(
                (
                    sheet,
                    CellAddress {
                        column: 1,
                        row: row as u32,
                    },
                ),
                FormulaValue::Number(value),
            );
        }
    }
    let book = FormulaEvaluationBook {
        sheet_names: vec![
            SheetBinding {
                id: sheet1,
                name: Cow::Borrowed("Sheet1"),
            },
            SheetBinding {
                id: sheet2,
                name: Cow::Borrowed("Sheet2"),
            },
            SheetBinding {
                id: sheet3,
                name: Cow::Borrowed("Sheet3"),
            },
        ],
        cells,
        ..FormulaEvaluationBook::default()
    };

    for formula in ["SUM(B1:B2:B3)", "SUM(B1:B3:B2)", "SUM(B2:B3:B1)"] {
        assert_number(&book, formula, 7.0);
    }
    assert_number_with_epsilon(&book, "SUM(Sheet2!B1:B2:B3)", 56.0, 1e-10);
    assert_number(&book, "SUM(Sheet1!B1:Sheet2!B2:Sheet3!B3)", 511.0);
    assert_number(&book, "SUM(Sheet1!B1:Sheet3!B2:Sheet2!B3)", 511.0);
    assert_number(&book, "SUM(B$2:B$2:B2)", 2.0);
}

#[test]
fn evaluates_excel_intersection_operator_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testIntersectionOpExcel.
    let book = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Test"),
        }],
        cells: BTreeMap::from([((SHEET, address("C2")), FormulaValue::Number(1.0))]),
        defined_names: BTreeMap::from([
            (
                DefinedNameKey {
                    sheet: None,
                    name_upper: "HORZ".to_string(),
                },
                Cow::Borrowed("$B$2:$D$2"),
            ),
            (
                DefinedNameKey {
                    sheet: None,
                    name_upper: "VERT".to_string(),
                },
                Cow::Borrowed("$C$1:$C$3"),
            ),
        ]),
        ..FormulaEvaluationBook::default()
    };

    assert_number(&book, "B2:D2 C1:C3", 1.0);
    assert_number(&book, "horz vert", 1.0);
    assert_number(&book, "(horz vert)*2", 2.0);
    assert_number(&book, "2*(horz vert)", 2.0);
}

#[test]
fn evaluates_min_lookup_and_conditional_aggregate_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFuncMIN and
    // sc/qa/unit/ucalc_formula2.cxx::testFuncLOOKUP, testFuncLOOKUParrayWithError,
    // testTdf141146, and testFuncSUMIFS.
    let min_book = evaluation_book(&[
        ("B1", FormulaValue::Number(1.0)),
        ("B2", FormulaValue::Number(2.0)),
        ("B3", FormulaValue::Number(20.0)),
        ("B4", FormulaValue::Number(-20.0)),
    ]);
    assert_number(&min_book, "MIN({-2;4;3})", -2.0);
    assert_number(&min_book, "MIN(B1:B4)", -20.0);

    let lookup_book = evaluation_book(&[
        ("A1", string("A")),
        ("B1", FormulaValue::Number(1.0)),
        ("A2", string("B")),
        ("B2", FormulaValue::Number(2.0)),
        ("A3", string("C")),
        ("B3", FormulaValue::Number(3.0)),
        ("A5", string("A")),
        ("A6", string("B")),
        ("A7", string("C")),
        ("C2", string("x")),
        ("D2", string("y")),
        ("E2", string("z")),
        ("C3", string("a")),
        ("D3", string("b")),
        ("E3", string("c")),
        ("G2", string("one")),
        ("G6", string("two")),
        ("K2", string("k1")),
        ("L2", string("value1")),
        ("K3", string("k2")),
        ("L3", string("value2")),
        ("K4", string("k3")),
        ("L4", string("value3")),
        ("N1", string("k2")),
    ]);
    assert_number(&lookup_book, "LOOKUP(A5,A1:A3,B1:B3)", 1.0);
    assert_number(&lookup_book, "LOOKUP(A6,A1:A3,B1:B3)", 2.0);
    assert_number(&lookup_book, "LOOKUP(A7,A1:A3,B1:B3)", 3.0);
    assert_text(&lookup_book, "LOOKUP(2,1/(C2:E2<>\"\"),C3:E3)", "c");
    assert_text(
        &lookup_book,
        "LOOKUP(2,1/(NOT(ISBLANK(G2:G9))),G2:G9)",
        "two",
    );
    assert_error(
        &lookup_book,
        "LOOKUP(2,1/(NOT(ISBLANK(I2:I9))),I2:I9)",
        FormulaErrorValue::NA,
    );
    assert_number(&lookup_book, "LOOKUP(1,1/(K2:K4=N1),1)", 1.0);
    assert_text(&lookup_book, "LOOKUP(N1,K2:K4,L2:L4)", "value2");
    assert_text(&lookup_book, "LOOKUP(1,1/(K2:K4=N1),L2:L4)", "value2");

    let aggregate_book = evaluation_book(&[
        ("A1", string("a")),
        ("B1", FormulaValue::Number(1.0)),
        ("A2", string("b")),
        ("B2", FormulaValue::Number(2.0)),
        ("A3", string("c")),
        ("B3", FormulaValue::Number(4.0)),
        ("A4", string("d")),
        ("B4", FormulaValue::Number(8.0)),
        ("A5", string("a")),
        ("B5", FormulaValue::Number(16.0)),
        ("A6", string("b")),
        ("B6", FormulaValue::Number(32.0)),
        ("A7", string("c")),
        ("B7", FormulaValue::Number(64.0)),
        ("A9", string("a")),
        ("A10", string("b")),
        ("A11", string("c")),
    ]);
    assert_number(&aggregate_book, "SUMIFS(B1:B7,A1:A7,A9)", 17.0);
    assert_number(&aggregate_book, "SUMIFS(B1:B7,A1:A7,A10)", 34.0);
    assert_number(&aggregate_book, "SUMIFS(B1:B7,A1:A7,A11)", 68.0);
    assert_number(&aggregate_book, "COUNTIFS(A1:A7,A9)", 2.0);
    assert_number(&aggregate_book, "COUNTIFS(A1:A7,A10)", 2.0);
    assert_number(&aggregate_book, "COUNTIFS(A1:A7,A11)", 2.0);
    assert_number(&aggregate_book, "AVERAGEIFS(B1:B7,A1:A7,A9)", 8.5);
    assert_number(&aggregate_book, "AVERAGEIFS(B1:B7,A1:A7,A10)", 17.0);
    assert_number(&aggregate_book, "AVERAGEIFS(B1:B7,A1:A7,A11)", 34.0);
}

#[test]
fn evaluates_matrix_operator_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testMatrixOp.
    let book = evaluation_book(&[
        ("A1", FormulaValue::Number(0.0)),
        ("A2", FormulaValue::Number(1.0)),
        ("A3", FormulaValue::Number(2.0)),
        ("A4", FormulaValue::Number(3.0)),
        ("B1", FormulaValue::Number(2.0)),
        ("D1", FormulaValue::Number(1.0)),
        ("D2", FormulaValue::Number(2.0)),
    ]);

    assert_number(&book, "SUMPRODUCT((A1:A4)*B1+D1)", 16.0);
    assert_number(&book, "SUMPRODUCT((A1:A4)*B1-D2)", 4.0);
    for (formula, expected) in [
        ("SUMPRODUCT({1;2;4}+8)", 31.0),
        ("SUMPRODUCT(8+{1;2;4})", 31.0),
        ("SUMPRODUCT({1;2;4}-8)", -17.0),
        ("SUMPRODUCT(8-{1;2;4})", 17.0),
        ("SUMPRODUCT({1;2;4}+{8;16;32})", 63.0),
        ("SUMPRODUCT({8;16;32}+{1;2;4})", 63.0),
        ("SUMPRODUCT({1;2;4}-{8;16;32})", -49.0),
        ("SUMPRODUCT({8;16;32}-{1;2;4})", 49.0),
    ] {
        assert_number(&book, formula, expected);
    }
}

#[test]
fn evaluates_matrix_concatenation_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testMatConcat and
    // testMatConcatReplication.
    let mut cells = BTreeMap::new();
    for column in 0..10 {
        for row in 0..10 {
            cells.insert(
                (SHEET, CellAddress { column, row }),
                FormulaValue::Number(f64::from(column * row)),
            );
        }
    }
    for (reference, value) in [
        ("A12", string("q")),
        ("B12", string("w")),
        ("A13", string("a")),
        ("B13", string("")),
        ("A14", string("")),
        ("B14", string("x")),
        ("A15", string("")),
        ("B15", string("")),
        ("A16", string("e")),
        ("B16", string("r")),
    ] {
        cells.insert((SHEET, address(reference)), value);
    }
    let book = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Test"),
        }],
        cells,
        ..FormulaEvaluationBook::default()
    };

    let expected = [["00", "00", "00"], ["00", "11", "22"], ["00", "22", "44"]];
    assert_matrix_texts_with_grammar(
        &book,
        SHEET,
        Some("A13"),
        "A1:C3&A1:C3",
        FormulaGrammar::ExcelA1,
        &[&expected[0], &expected[1], &expected[2]],
    );
    assert_matrix_texts_with_grammar(
        &book,
        SHEET,
        Some("C17"),
        "A12:A16&B12:B16",
        FormulaGrammar::ExcelA1,
        &[&["qw"], &["a"], &["x"], &[""], &["er"]],
    );
    assert_matrix_texts_with_grammar(
        &book,
        SHEET,
        Some("A13"),
        "A1:C3&A1:C1",
        FormulaGrammar::ExcelA1,
        &[
            &["00", "00", "00"],
            &["00", "10", "20"],
            &["00", "20", "40"],
        ],
    );
}

#[test]
fn evaluates_ref_list_array_subtotal_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncRefListArraySUBTOTAL.
    let mut book = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(2.0)),
        ("A3", FormulaValue::Number(4.0)),
        ("A4", FormulaValue::Number(8.0)),
        ("A5", FormulaValue::Number(16.0)),
        ("A6", FormulaValue::Number(32.0)),
    ]);

    assert_matrix_numbers_with_grammar(
        &book,
        SHEET,
        Some("B7"),
        "SUBTOTAL(9,OFFSET(A1,ROW(1:3),0,2))",
        FormulaGrammar::ExcelA1,
        &[&[6.0], &[12.0], &[24.0]],
    );
    assert_matrix_numbers_with_grammar(
        &book,
        SHEET,
        Some("C7"),
        "SUBTOTAL(1,OFFSET(A1,ROW(1:3),0,2))",
        FormulaGrammar::ExcelA1,
        &[&[3.0], &[6.0], &[12.0]],
    );
    assert_matrix_numbers_with_grammar(
        &book,
        SHEET,
        Some("D7"),
        "SUBTOTAL(5,OFFSET(A1,ROW(1:3),0,2))",
        FormulaGrammar::ExcelA1,
        &[&[2.0], &[4.0], &[8.0]],
    );
    assert_matrix_numbers_with_grammar(
        &book,
        SHEET,
        Some("E7"),
        "SUBTOTAL(4,OFFSET(A1,ROW(1:3),0,2))",
        FormulaGrammar::ExcelA1,
        &[&[4.0], &[8.0], &[16.0]],
    );

    book.row_states.insert(
        (SHEET, address("A2").row),
        FormulaRowState {
            hidden: true,
            filtered: false,
        },
    );
    book.row_states.insert(
        (SHEET, address("A3").row),
        FormulaRowState {
            hidden: true,
            filtered: false,
        },
    );
    book.row_states.insert(
        (SHEET, address("A4").row),
        FormulaRowState {
            hidden: true,
            filtered: false,
        },
    );
    assert_number(
        &book,
        "SUM(SUBTOTAL(109,OFFSET(A1,ROW(A1:A7)-ROW(A1),,1)))",
        49.0,
    );
    assert_number(
        &book,
        "SUMPRODUCT(SUBTOTAL(109,OFFSET(A1,ROW(A1:A7)-ROW(A1),,1)))",
        49.0,
    );
}

#[test]
fn evaluates_jump_matrix_array_if_and_offset_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncJumpMatrixArrayIF
    // and testFuncJumpMatrixArrayOFFSET.
    let book = evaluation_book(&[
        ("A1", string("a")),
        ("A7", string("a")),
        ("B7", FormulaValue::Number(1.0)),
        ("A8", string("b")),
        ("B8", FormulaValue::Number(2.0)),
        ("A9", string("a")),
        ("B9", FormulaValue::Number(4.0)),
    ]);
    assert_raw_number_with_grammar(
        &book,
        SHEET,
        Some("C10"),
        "SUM(IF(EXACT(A7:A9,A$1),B7:B9,0))",
        FormulaGrammar::ExcelA1,
        5.0,
    );
    assert_raw_number_with_grammar(
        &book,
        SHEET,
        Some("C11"),
        "SUM(IF(EXACT(OFFSET(A7,0,0):OFFSET(A7,2,0),A$1),OFFSET(A7,0,1):OFFSET(A7,2,1),0))",
        FormulaGrammar::ExcelA1,
        5.0,
    );

    let offset_book = evaluation_book(&[
        ("A1", string("abc")),
        ("A2", string("bcd")),
        ("A3", string("cde")),
    ]);
    assert_matrix_numbers_with_grammar(
        &offset_book,
        SHEET,
        Some("C5"),
        "FIND(\"c\",OFFSET(A1:A3,0,COLUMN()-3))",
        FormulaGrammar::ExcelA1,
        &[&[3.0], &[2.0], &[1.0]],
    );
}

#[test]
fn evaluates_formula_error_propagation_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFormulaErrorPropagation.
    let book = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("B1", FormulaValue::Number(2.0)),
    ]);

    assert_boolean(&book, "ISERROR(A1:B1+3)", true);
    assert_boolean(&book, "ISERROR(A1:B1+{3})", true);
    assert_boolean(&book, "ISERROR({1,\"x\"}+{3,4})", false);
    assert_boolean(&book, "ISERROR({\"x\",2}+{3,4})", true);
    assert_boolean(&book, "ISERROR(({1,\"x\"}+{3,4})-{5,6})", false);
    assert_boolean(&book, "ISERROR(({\"x\",2}+{3,4})-{5,6})", true);
}

#[test]
fn evaluates_formula_text_and_cell_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncFORMULA and testFuncCELL.
    let mut book = evaluation_book(&[
        ("B1", FormulaValue::Number(0.0)),
        ("B2", FormulaValue::Blank),
        ("B3", FormulaValue::Number(0.0)),
        ("C1", FormulaValue::Number(1.2)),
        ("C9", FormulaValue::Blank),
        ("C10", string("Some random text")),
    ]);
    book.formulas.insert(
        (SHEET, address("B1")),
        FormulaText {
            text: Cow::Borrowed("=A1"),
            kind: FormulaKind::Normal,
            reference: None,
        },
    );
    book.formulas.insert(
        (SHEET, address("B3")),
        FormulaText {
            text: Cow::Borrowed("=A3"),
            kind: FormulaKind::Normal,
            reference: None,
        },
    );

    assert_text(&book, "FORMULATEXT(B1)", "=A1");
    assert_error(&book, "FORMULATEXT(B2)", FormulaErrorValue::NA);
    assert_text(&book, "FORMULATEXT(B3)", "=A3");

    for (formula, expected) in [
        ("CELL(\"COL\",C10)", 3.0),
        ("CELL(\"COL\",C5:C10)", 3.0),
        ("CELL(\"ROW\",C10)", 10.0),
        ("CELL(\"ROW\",C10:E10)", 10.0),
        ("CELL(\"SHEET\",C10)", 1.0),
        ("CELL(\"COLOR\",C10)", 0.0),
        ("CELL(\"PARENTHESES\",C10)", 0.0),
    ] {
        assert_number(&book, formula, expected);
    }
    assert_text(&book, "CELL(\"ADDRESS\",C10)", "$C$10");
    assert_text(&book, "CELL(\"CONTENTS\",C10)", "Some random text");
    assert_text(&book, "CELL(\"TYPE\",C9)", "b");
    assert_text(&book, "CELL(\"TYPE\",C10)", "l");
    assert_text(&book, "CELL(\"TYPE\",C1)", "v");
}

#[test]
fn evaluates_formula_regression_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testTdf93415,
    // testTdf127334, and testTdf132519.
    let book = evaluation_book(&[("C1", string("X")), ("B1", string("R1C3"))]);

    assert_text(&book, "ADDRESS(1,1,,,\"Sheet1\")", "Sheet1!$A$1");
    assert_number(
        &book,
        "(((DATE(2019,9,17)+TIME(0,0,1))-DATE(2019,9,17))-TIME(0,0,1))/TIME(0,0,1)",
        0.0,
    );
    assert_text(&book, "CELL(\"ADDRESS\",C1)", "$C$1");
    assert_text_with_grammar(&book, "INDIRECT(B1)", FormulaGrammar::ExcelR1C1, "X");
}

#[test]
fn evaluates_subtotal_and_aggregate_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncRowsHidden and
    // sc/qa/unit/ucalc_formula.cxx::testFuncSUBTOTALReferenceNotMutated.
    let mut book = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(2.0)),
        ("A3", FormulaValue::Number(4.0)),
        ("A4", FormulaValue::Number(8.0)),
        ("A5", FormulaValue::Number(16.0)),
        ("A6", FormulaValue::Number(32.0)),
    ]);

    assert_number(&book, "SUBTOTAL(109,A1:A6)", 63.0);
    assert_number(&book, "AGGREGATE(9,5,A1:A6)", 63.0);
    assert_number(&book, "SUM(A1:A6)", 63.0);

    book.row_states.insert(
        (SHEET, address("A1").row),
        FormulaRowState {
            hidden: true,
            filtered: false,
        },
    );
    assert_number(&book, "SUBTOTAL(109,A1:A6)", 62.0);
    assert_number(&book, "AGGREGATE(9,5,A1:A6)", 62.0);
    assert_number(&book, "SUM(A1:A6)", 63.0);

    book.row_states.clear();
    for row in address("A2").row..=address("A3").row {
        book.row_states.insert(
            (SHEET, row),
            FormulaRowState {
                hidden: true,
                filtered: false,
            },
        );
    }
    assert_number(&book, "SUBTOTAL(109,A1:A6)", 57.0);

    book.row_states.clear();
    for row in address("A3").row..=address("A5").row {
        book.row_states.insert(
            (SHEET, row),
            FormulaRowState {
                hidden: true,
                filtered: false,
            },
        );
    }
    assert_number(&book, "AGGREGATE(9,5,A1:A6)", 35.0);

    let subtotal_large_range = evaluation_book(&[
        ("B1", FormulaValue::Number(10.0)),
        ("B2", FormulaValue::Number(20.0)),
        ("B3", FormulaValue::Number(30.0)),
        ("B4", FormulaValue::Number(40.0)),
    ]);
    assert_number(&subtotal_large_range, "SUBTOTAL(9,B1:B99999)", 100.0);
}

#[test]
fn evaluates_statistical_test_function_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncFTEST,
    // testFuncFTESTBug, testFuncCHITEST, and testFuncTTEST.
    let book = evaluation_book(&[
        ("A1", FormulaValue::Number(9.0)),
        ("A2", FormulaValue::Number(8.0)),
        ("A3", FormulaValue::Number(2.0)),
        ("B1", FormulaValue::Number(6.0)),
        ("B2", FormulaValue::Number(8.0)),
        ("B3", FormulaValue::Number(4.0)),
        ("C1", FormulaValue::Number(3.0)),
        ("C2", FormulaValue::Number(9.0)),
        ("C3", FormulaValue::Number(13.0)),
        ("D1", FormulaValue::Number(5.0)),
        ("D2", FormulaValue::Number(6.0)),
        ("D3", FormulaValue::Number(8.0)),
        ("E1", FormulaValue::Number(7.0)),
        ("E2", FormulaValue::Number(4.0)),
        ("E3", FormulaValue::Number(7.0)),
        ("F1", FormulaValue::Number(28.0)),
        ("F2", FormulaValue::Number(4.0)),
        ("F3", FormulaValue::Number(5.0)),
        ("H1", FormulaValue::Number(9.0)),
        ("H2", FormulaValue::Number(8.0)),
        ("H3", FormulaValue::Number(6.0)),
        ("I1", FormulaValue::Number(5.0)),
        ("I2", FormulaValue::Number(7.0)),
    ]);

    assert_number_with_epsilon(&book, "FTEST(A1:C3,D1:F3)", 0.0422, 1e-4);
    assert_number_with_epsilon(&book, "FTEST(H1:H3,I1:I3)", 0.9046, 1e-4);

    let chi_2x2 = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(2.0)),
        ("B1", FormulaValue::Number(2.0)),
        ("B2", FormulaValue::Number(0.0)),
        ("D1", FormulaValue::Number(2.0)),
        ("D2", FormulaValue::Number(3.0)),
        ("E1", FormulaValue::Number(3.0)),
        ("E2", FormulaValue::Number(1.0)),
    ]);
    assert_number_with_epsilon(&chi_2x2, "CHITEST(A1:B2,D1:E2)", 0.1410, 1e-4);

    let chi_3x3 = evaluation_book(&[
        ("A1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(2.0)),
        ("A3", FormulaValue::Number(4.0)),
        ("B1", FormulaValue::Number(2.0)),
        ("B2", FormulaValue::Number(0.0)),
        ("B3", FormulaValue::Number(2.0)),
        ("C1", FormulaValue::Number(3.0)),
        ("C2", FormulaValue::Number(2.0)),
        ("C3", FormulaValue::Number(3.0)),
        ("D1", FormulaValue::Number(2.0)),
        ("D2", FormulaValue::Number(3.0)),
        ("D3", FormulaValue::Number(3.0)),
        ("E1", FormulaValue::Number(3.0)),
        ("E2", FormulaValue::Number(1.0)),
        ("E3", FormulaValue::Number(1.0)),
        ("F1", FormulaValue::Number(1.0)),
        ("F2", FormulaValue::Number(2.0)),
        ("F3", FormulaValue::Number(3.0)),
    ]);
    assert_number_with_epsilon(&chi_3x3, "CHITEST(A1:C3,D1:F3)", 0.1117, 1e-4);

    let ttest = evaluation_book(&[
        ("A1", FormulaValue::Number(8.0)),
        ("B1", FormulaValue::Number(2.0)),
        ("C1", FormulaValue::Number(1.0)),
        ("A2", FormulaValue::Number(-4.0)),
        ("B2", FormulaValue::Number(5.0)),
        ("C2", FormulaValue::Number(-1.0)),
        ("A3", FormulaValue::Number(10.0)),
        ("B3", FormulaValue::Number(3.0)),
        ("C3", FormulaValue::Number(-5.0)),
        ("D1", FormulaValue::Number(3.0)),
        ("E1", FormulaValue::Number(1.0)),
        ("F1", FormulaValue::Number(6.0)),
        ("D2", FormulaValue::Number(1.0)),
        ("E2", FormulaValue::Number(-2.0)),
        ("F2", FormulaValue::Number(-3.0)),
        ("D3", FormulaValue::Number(10.0)),
        ("E3", FormulaValue::Number(9.0)),
        ("F3", FormulaValue::Number(6.0)),
    ]);
    assert_number_with_epsilon(&ttest, "TTEST(A1:C3,D1:F3,1,1)", 0.25529, 1e-5);
}

#[test]
fn evaluates_matrix_determinant_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testFuncMDETERM.
    let book = evaluation_book(&[]);

    assert_number_with_epsilon(&book, "MDETERM({1,2,3;4,5,6;7,8,9})", 0.0, 1e-14);
    assert_number_with_epsilon(
        &book,
        "MDETERM({23,31,13,12;34,64,34,31;98,32,33,63;45,54,65,76})",
        -180655.0,
        1e-6,
    );
}

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

fn assert_number_in_range(book: &FormulaEvaluationBook<'_>, formula: &str, min: f64, max: f64) {
    let actual = number(book.evaluate_formula_text(SHEET, None, formula));
    assert!(
        actual >= min && actual <= max,
        "{formula}: expected value in [{min}, {max}], got {actual}"
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

fn number_value(value: f64) -> FormulaValue<'static> {
    FormulaValue::Number(value)
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
fn evaluates_apache_poi_xlookup_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/atp/TestXLookupFunction.java.
    let country = evaluation_book(&[
        ("B2", string("China")),
        ("D2", string("+86")),
        ("F2", string("Brazil")),
        ("B3", string("India")),
        ("D3", string("+91")),
        ("B4", string("United States")),
        ("D4", string("+1")),
        ("B5", string("Indonesia")),
        ("D5", string("+62")),
        ("B6", string("Brazil")),
        ("D6", string("+55")),
        ("B7", string("Pakistan")),
        ("D7", string("+92")),
        ("B8", string("Nigeria")),
        ("D8", string("+234")),
        ("B9", string("Bangladesh")),
        ("D9", string("+880")),
        ("B10", string("Russia")),
        ("D10", string("+7")),
        ("B11", string("Mexico")),
        ("D11", string("+52")),
    ]);
    assert_text(&country, "XLOOKUP(F2,B2:B11,D2:D11)", "+55");
    assert_text(&country, "XLOOKUP(\"Brazil\",B2:B11,D2:D11)", "+55");
    assert_text(&country, "XLOOKUP(\"brazil\",B2:B11,D2:D11)", "+55");
    assert_text(&country, "XLOOKUP(\"brazil\",B2:B11,D2:D11,,2)", "+55");
    assert_text(&country, "XLOOKUP(\"b*l\",B2:B11,D2:D11,,2)", "+55");
    assert_text(&country, "XLOOKUP(\"i???a\",B2:B11,D2:D11,,2)", "+91");

    let employees = evaluation_book(&[
        ("B2", number_value(8389.0)),
        ("B5", number_value(4390.0)),
        ("C5", string("Ned Lanning")),
        ("D5", string("Marketing")),
        ("B6", number_value(8604.0)),
        ("C6", string("Margo Hendrix")),
        ("D6", string("Sales")),
        ("B7", number_value(8389.0)),
        ("C7", string("Dianne Pugh")),
        ("D7", string("Finance")),
        ("B8", number_value(4937.0)),
        ("C8", string("Earlene McCarty")),
        ("D8", string("Accounting")),
        ("B9", number_value(8299.0)),
        ("C9", string("Mia Arnold")),
        ("D9", string("Operation")),
        ("B10", number_value(2643.0)),
        ("C10", string("Jorge Fellows")),
        ("D10", string("Executive")),
        ("B11", number_value(5243.0)),
        ("C11", string("Rose Winters")),
        ("D11", string("Sales")),
        ("B12", number_value(9693.0)),
        ("C12", string("Carmela Hahn")),
        ("D12", string("Finance")),
        ("B13", number_value(1636.0)),
        ("C13", string("Delia Cochran")),
        ("D13", string("Accounting")),
        ("B14", number_value(6703.0)),
        ("C14", string("Marguerite Cervantes")),
        ("D14", string("Marketing")),
    ]);
    assert_matrix_texts_with_grammar(
        &employees,
        SHEET,
        Some("C2"),
        "XLOOKUP(B2,B5:B14,C5:D14)",
        FormulaGrammar::ExcelA1,
        &[&["Dianne Pugh", "Finance"]],
    );

    let missing_employee = evaluation_book(&[
        ("B2", number_value(999999.0)),
        ("B5", number_value(4390.0)),
        ("C5", string("Ned Lanning")),
        ("D5", string("Marketing")),
        ("B6", number_value(8604.0)),
        ("C6", string("Margo Hendrix")),
        ("D6", string("Sales")),
        ("B7", number_value(8389.0)),
        ("C7", string("Dianne Pugh")),
        ("D7", string("Finance")),
        ("B8", number_value(4937.0)),
        ("C8", string("Earlene McCarty")),
        ("D8", string("Accounting")),
        ("B9", number_value(8299.0)),
        ("C9", string("Mia Arnold")),
        ("D9", string("Operation")),
        ("B10", number_value(2643.0)),
        ("C10", string("Jorge Fellows")),
        ("D10", string("Executive")),
        ("B11", number_value(5243.0)),
        ("C11", string("Rose Winters")),
        ("D11", string("Sales")),
        ("B12", number_value(9693.0)),
        ("C12", string("Carmela Hahn")),
        ("D12", string("Finance")),
        ("B13", number_value(1636.0)),
        ("C13", string("Delia Cochran")),
        ("D13", string("Accounting")),
        ("B14", number_value(6703.0)),
        ("C14", string("Marguerite Cervantes")),
        ("D14", string("Marketing")),
    ]);
    assert_error(
        &missing_employee,
        "XLOOKUP(B2,B5:B14,C5:D14)",
        FormulaErrorValue::NA,
    );
    assert_text(
        &missing_employee,
        "XLOOKUP(B2,B5:B14,C5:C14,\"not found\")",
        "not found",
    );
    assert_matrix_texts_with_grammar(
        &missing_employee,
        SHEET,
        Some("C2"),
        "XLOOKUP(B2,B5:B14,C5:D14,\"not found\")",
        FormulaGrammar::ExcelA1,
        &[&["not found", ""]],
    );

    let tax = evaluation_book(&[
        ("B2", number_value(0.10)),
        ("C2", number_value(9700.0)),
        ("E2", number_value(46523.0)),
        ("B3", number_value(0.22)),
        ("C3", number_value(39475.0)),
        ("B4", number_value(0.24)),
        ("C4", number_value(84200.0)),
        ("B5", number_value(0.32)),
        ("C5", number_value(160726.0)),
        ("B6", number_value(0.35)),
        ("C6", number_value(204100.0)),
        ("B7", number_value(0.37)),
        ("C7", number_value(510300.0)),
    ]);
    assert_number(&tax, "XLOOKUP(E2,C2:C7,B2:B7,0,1,1)", 0.24);
    assert_number(&tax, "XLOOKUP(E2,C2:C7,B2:B7,0,1,-1)", 0.24);
    assert_number(&tax, "XLOOKUP(E2,C2:C7,B2:B7,0,1,2)", 0.24);
    assert_number(&tax, "XLOOKUP(39475,C2:C7,B2:B7,0,0,2)", 0.22);
    assert_number(&tax, "XLOOKUP(39474,C2:C7,B2:B7,0,0,2)", 0.0);

    let reverse_tax = evaluation_book(&[
        ("B2", number_value(0.37)),
        ("C2", number_value(510300.0)),
        ("E2", number_value(46523.0)),
        ("B3", number_value(0.35)),
        ("C3", number_value(204100.0)),
        ("B4", number_value(0.32)),
        ("C4", number_value(160726.0)),
        ("B5", number_value(0.24)),
        ("C5", number_value(84200.0)),
        ("B6", number_value(0.22)),
        ("C6", number_value(39475.0)),
        ("B7", number_value(0.10)),
        ("C7", number_value(9700.0)),
    ]);
    assert_number(&reverse_tax, "XLOOKUP(E2,C2:C7,B2:B7,0,1,-2)", 0.24);
    assert_number(&reverse_tax, "XLOOKUP(39475,C2:C7,B2:B7,0,0,-2)", 0.22);
    assert_number(&reverse_tax, "XLOOKUP(39474,C2:C7,B2:B7,0,0,-2)", 0.0);

    let reverse_tax_with_invalid_incomes = evaluation_book(&[
        ("B2", number_value(0.37)),
        ("C2", number_value(510300.0)),
        ("E2", number_value(46523.0)),
        ("B3", number_value(0.35)),
        ("C3", string("invalid")),
        ("B4", number_value(0.32)),
        ("C4", string("invalid")),
        ("B5", number_value(0.24)),
        ("C5", string("invalid")),
        ("B6", number_value(0.22)),
        ("C6", string("invalid")),
        ("B7", number_value(0.10)),
        ("C7", number_value(9700.0)),
    ]);
    assert_number(
        &reverse_tax_with_invalid_incomes,
        "XLOOKUP(E2,C2:C7,B2:B7,0,1,-2)",
        0.37,
    );
    assert_number(
        &reverse_tax_with_invalid_incomes,
        "XLOOKUP(9700,C2:C7,B2:B7,0,0,-2)",
        0.10,
    );
    assert_number(
        &reverse_tax_with_invalid_incomes,
        "XLOOKUP(39474,C2:C7,B2:B7,0,0,-2)",
        0.0,
    );

    let profit = evaluation_book(&[
        ("C3", string("Qtr1")),
        ("D2", string("Gross Profit")),
        ("B5", string("Income Statement")),
        ("C5", string("Qtr1")),
        ("D5", string("Qtr2")),
        ("E5", string("Qtr3")),
        ("F5", string("Qtr4")),
        ("G5", string("Total")),
        ("B6", string("Total Sales")),
        ("C6", number_value(50000.0)),
        ("D6", number_value(78200.0)),
        ("E6", number_value(89500.0)),
        ("F6", number_value(91250.0)),
        ("G6", number_value(308950.0)),
        ("B7", string("Cost of Sales")),
        ("C7", number_value(-25000.0)),
        ("D7", number_value(-42050.0)),
        ("E7", number_value(-59450.0)),
        ("F7", number_value(-60450.0)),
        ("G7", number_value(-186950.0)),
        ("B8", string("Gross Profit")),
        ("C8", number_value(25000.0)),
        ("D8", number_value(37150.0)),
        ("E8", number_value(-30050.0)),
        ("F8", number_value(-30450.0)),
        ("G8", number_value(122000.0)),
        ("B10", string("Depreciation")),
        ("C10", number_value(-899.0)),
        ("D10", number_value(-791.0)),
        ("E10", number_value(-202.0)),
        ("F10", number_value(-412.0)),
        ("G10", number_value(-2304.0)),
        ("B11", string("Interest")),
        ("C11", number_value(-513.0)),
        ("D11", number_value(-853.0)),
        ("E11", number_value(-150.0)),
        ("F11", number_value(-956.0)),
        ("G11", number_value(-2472.0)),
        ("B12", string("Earnings before Tax")),
        ("C12", number_value(23588.0)),
        ("D12", number_value(34506.0)),
        ("E12", number_value(29698.0)),
        ("F12", number_value(29432.0)),
        ("G12", number_value(117224.0)),
        ("B14", string("Tax")),
        ("C14", number_value(-4246.0)),
        ("D14", number_value(-6211.0)),
        ("E14", number_value(-5346.0)),
        ("F14", number_value(-5298.0)),
        ("G14", number_value(-21100.0)),
        ("B16", string("Profit %")),
        ("C16", number_value(0.293)),
        ("D16", number_value(0.278)),
        ("E16", number_value(0.234)),
        ("F16", number_value(0.236)),
        ("G16", number_value(0.269)),
    ]);
    assert_number(&profit, "XLOOKUP(D2,$B6:$B17,$C6:$C17)", 25000.0);
    assert_number(
        &profit,
        "XLOOKUP(D2,$B6:$B17,XLOOKUP($C3,$C5:$G5,$C6:$G17))",
        25000.0,
    );

    let products = evaluation_book(&[
        ("B3", string("Grape")),
        ("C3", string("Banana")),
        ("B6", string("Product")),
        ("C6", string("Qty")),
        ("D6", string("Price")),
        ("E6", string("Total")),
        ("B7", string("Apple")),
        ("C7", number_value(23.0)),
        ("D7", number_value(0.52)),
        ("E7", number_value(11.90)),
        ("B8", string("Grape")),
        ("C8", number_value(98.0)),
        ("D8", number_value(0.77)),
        ("E8", number_value(75.28)),
        ("B9", string("Pear")),
        ("C9", number_value(75.0)),
        ("D9", number_value(0.24)),
        ("E9", number_value(18.16)),
        ("B10", string("Banana")),
        ("C10", number_value(95.0)),
        ("D10", number_value(0.18)),
        ("E10", number_value(17.25)),
        ("B11", string("Cherry")),
        ("C11", number_value(42.0)),
        ("D11", number_value(0.16)),
        ("E11", number_value(6.80)),
    ]);
    assert_number(&products, "XLOOKUP(B3,B6:B10,E6:E10)", 75.28);
    assert_number(&products, "XLOOKUP(C3,B6:B10,E6:E10)", 17.25);
    assert_number(
        &products,
        "SUM(XLOOKUP(B3,B6:B10,E6:E10):XLOOKUP(C3,B6:B10,E6:E10))",
        110.69,
    );
}

#[test]
fn evaluates_apache_poi_xmatch_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/atp/TestXMatchFunction.java.
    let products = evaluation_book(&[
        ("C3", string("Apple")),
        ("E3", string("Grape")),
        ("C4", string("Grape")),
        ("C5", string("Pear")),
        ("C6", string("Banana")),
        ("C7", string("Cherry")),
    ]);
    assert_number(&products, "XMATCH(E3,C3:C7)", 2.0);
    assert_error(&products, "XMATCH(\"Gra\",C3:C7)", FormulaErrorValue::NA);

    let lowercase_product = evaluation_book(&[
        ("C3", string("Apple")),
        ("E3", string("grape")),
        ("C4", string("Grape")),
        ("C5", string("Pear")),
        ("C6", string("Banana")),
        ("C7", string("Cherry")),
    ]);
    assert_number(&lowercase_product, "XMATCH(E3,C3:C7)", 2.0);

    let wildcard_like_product = evaluation_book(&[
        ("C3", string("Apple")),
        ("E3", string("Gra?")),
        ("C4", string("Grape")),
        ("C5", string("Pear")),
        ("C6", string("Banana")),
        ("C7", string("Cherry")),
    ]);
    assert_number(&wildcard_like_product, "XMATCH(E3,C3:C7,1)", 2.0);
    assert_number(&wildcard_like_product, "XMATCH(E3,C3:C7,-1)", 5.0);
    assert_number(&wildcard_like_product, "XMATCH(\"Gra\",C3:C7,1)", 2.0);
    assert_number(&wildcard_like_product, "XMATCH(\"Graz\",C3:C7,1)", 3.0);
    assert_number(&wildcard_like_product, "XMATCH(\"Graz\",C3:C7,-1)", 2.0);

    let lowercase_wildcard_like_product = evaluation_book(&[
        ("C3", string("Apple")),
        ("E3", string("gra?")),
        ("C4", string("Grape")),
        ("C5", string("Pear")),
        ("C6", string("Banana")),
        ("C7", string("Cherry")),
    ]);
    assert_number(&lowercase_wildcard_like_product, "XMATCH(E3,C3:C7,1)", 2.0);
    assert_number(&lowercase_wildcard_like_product, "XMATCH(E3,C3:C7,-1)", 5.0);

    let sales = evaluation_book(&[
        ("C3", number_value(42000.0)),
        ("F2", number_value(15000.0)),
        ("C4", number_value(35000.0)),
        ("C5", number_value(25000.0)),
        ("C6", number_value(15901.0)),
        ("C7", number_value(13801.0)),
        ("C8", number_value(12181.0)),
        ("C9", number_value(9201.0)),
    ]);
    assert_number(&sales, "XMATCH(F2,C3:C9,1)", 4.0);
    assert_number(&sales, "XMATCH(F2,C3:C9,-1)", 5.0);
    assert_error(&sales, "XMATCH(F2,C3:C9,2)", FormulaErrorValue::NA);
    assert_number(&sales, "XMATCH(35000,C3:C9,1)", 2.0);
    assert_number(&sales, "XMATCH(36000,C3:C9,1)", 1.0);

    let table = evaluation_book(&[
        ("B3", string("Andrew Cencini")),
        ("C3", string("Feb")),
        ("B6", string("Michael Neipper")),
        ("C5", string("Jan")),
        ("D5", string("Feb")),
        ("E5", string("Mar")),
        ("C6", number_value(3174.0)),
        ("D6", number_value(6804.0)),
        ("E6", number_value(4713.0)),
        ("B7", string("Jan Kotas")),
        ("C7", number_value(1656.0)),
        ("D7", number_value(8643.0)),
        ("E7", number_value(3445.0)),
        ("B8", string("Nancy Freehafer")),
        ("C8", number_value(2706.0)),
        ("D8", number_value(2310.0)),
        ("E8", number_value(6606.0)),
        ("B9", string("Andrew Cencini")),
        ("C9", number_value(4930.0)),
        ("D9", number_value(8492.0)),
        ("E9", number_value(4474.0)),
        ("B10", string("Anne Hellung-Larsen")),
        ("C10", number_value(6394.0)),
        ("D10", number_value(9846.0)),
        ("E10", number_value(4368.0)),
        ("B11", string("Nancy Freehafer")),
        ("C11", number_value(2539.0)),
        ("D11", number_value(8996.0)),
        ("E11", number_value(4084.0)),
        ("B12", string("Mariya Sergienko")),
        ("C12", number_value(4468.0)),
        ("D12", number_value(5206.0)),
        ("E12", number_value(7343.0)),
    ]);
    assert_number(
        &table,
        "INDEX(C6:E12,XMATCH(B3,B6:B12),XMATCH(C3,C5:E5))",
        8492.0,
    );

    let empty = evaluation_book(&[]);
    assert_number(&empty, "XMATCH(4,{5,4,3,2,1})", 2.0);
    assert_number(&empty, "XMATCH(4.5,{5,4,3,2,1},1)", 1.0);
}

#[test]
fn evaluates_apache_poi_workbook_evaluator_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java
    // and poi/src/test/java/org/apache/poi/ss/usermodel/BaseTestFormulaEvaluator.java.
    let numeric = evaluation_book(&[
        ("A1", number_value(1.0)),
        ("B1", number_value(2.0)),
        ("C1", number_value(3.0)),
    ]);
    assert_number(&numeric, "IF(A1=1, 2, 3)", 2.0);
    assert_number(&numeric, "IF(A1=1, B1, C1)", 2.0);
    assert_number(&numeric, "IF(A1&\"\"=\"1\", B1, C1)", 2.0);
    assert_number(&numeric, "SUM(A1:B1)", 3.0);

    let string_cell = evaluation_book(&[
        ("A1", string("1")),
        ("B1", number_value(2.0)),
        ("C1", number_value(3.0)),
    ]);
    assert_number(&string_cell, "IF(A1=1, B1, C1)", 3.0);
    assert_number(&string_cell, "IF(A1=\"1\", B1, C1)", 2.0);
    assert_number(&string_cell, "IF(A1+0=1, B1, C1)", 2.0);

    let formula_cell_result = evaluation_book(&[
        ("A1", number_value(1.0)),
        ("B1", number_value(2.0)),
        ("C1", number_value(3.0)),
    ]);
    assert_number(&formula_cell_result, "IF(A1=1, B1, C1)", 2.0);

    let blank_cell = evaluation_book(&[
        ("A1", FormulaValue::Blank),
        ("B1", number_value(2.0)),
        ("C1", number_value(3.0)),
    ]);
    assert_number(&blank_cell, "IF(A1=1, B1, C1)", 3.0);
    assert_number(&blank_cell, "IF(A1=0, B1, C1)", 2.0);

    let ref_to_blank = evaluation_book(&[
        ("A1", string("1")),
        ("B1", FormulaValue::Blank),
        ("C1", string("3")),
    ]);
    assert_text(&ref_to_blank, "A1", "1");
    assert_number(&ref_to_blank, "B1+0", 0.0);
    assert_text(&ref_to_blank, "C1", "3");

    assert_number(&numeric, "1+IF(1,,)", 1.0);
    assert_text(&numeric, "\"abc\"&IF(1,,)", "abc");
    assert_text(&numeric, "\"abc\"&CHOOSE(2,5,,9)", "abc");
    assert_boolean(&numeric, "ISNUMBER(B1)", true);
    assert_number(&numeric, "IF(ISNUMBER(B1),B1,B2)", 2.0);

    let main = SheetId(1);
    let other = SheetId(2);
    let vlookup = FormulaEvaluationBook {
        sheet_names: vec![
            SheetBinding {
                id: main,
                name: Cow::Borrowed("main"),
            },
            SheetBinding {
                id: other,
                name: Cow::Borrowed("other"),
            },
        ],
        cells: BTreeMap::from([
            ((main, address("A1")), string("Thing Two")),
            ((other, address("A1")), string("Thing One")),
            ((other, address("B1")), string("1")),
            ((other, address("A2")), string("Thing Two")),
            ((other, address("B2")), string("2")),
        ]),
        ..FormulaEvaluationBook::default()
    };
    assert_text_at_with_grammar(
        &vlookup,
        main,
        "VLOOKUP(A1,other!A:B,2,FALSE)",
        FormulaGrammar::ExcelA1,
        "2",
    );

    let intersection = FormulaEvaluationBook {
        sheet_names: vec![SheetBinding {
            id: SHEET,
            name: Cow::Borrowed("Formula"),
        }],
        cells: BTreeMap::from([
            ((SHEET, address("A4")), number_value(1.0)),
            ((SHEET, address("B4")), number_value(2.0)),
            ((SHEET, address("C4")), number_value(3.0)),
            ((SHEET, address("A5")), number_value(4.0)),
            ((SHEET, address("B5")), number_value(5.0)),
            ((SHEET, address("C5")), number_value(6.0)),
        ]),
        defined_names: BTreeMap::from([
            (
                DefinedNameKey {
                    sheet: None,
                    name_upper: "FOO".to_string(),
                },
                Cow::Borrowed("A4:B5"),
            ),
            (
                DefinedNameKey {
                    sheet: None,
                    name_upper: "BAR".to_string(),
                },
                Cow::Borrowed("B4:C5"),
            ),
        ]),
        ..FormulaEvaluationBook::default()
    };
    assert_number(&intersection, "SUM(A4:B5 B4:C5)", 7.0);
    assert_number(&intersection, "SUM(foo bar)", 7.0);
}

#[test]
fn evaluates_apache_poi_formula_bug_regression_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java
    // and poi/src/test/java/org/apache/poi/ss/formula/functions/TestSumproduct.java.
    // HSSF fixture/API checks are reduced to equivalent public formula
    // behavior.
    let main = SheetId(1);
    let data_table = SheetId(2);
    let vlookup = FormulaEvaluationBook {
        sheet_names: vec![
            SheetBinding {
                id: main,
                name: Cow::Borrowed("Formula"),
            },
            SheetBinding {
                id: data_table,
                name: Cow::Borrowed("DATA TABLE"),
            },
        ],
        cells: BTreeMap::from([
            ((data_table, address("A8")), number_value(1.0)),
            ((data_table, address("B8")), number_value(3.0)),
            ((data_table, address("A9")), number_value(2.0)),
            ((data_table, address("B9")), number_value(4.0)),
            ((data_table, address("A10")), number_value(3.0)),
            ((data_table, address("B10")), number_value(5.0)),
        ]),
        ..FormulaEvaluationBook::default()
    };
    assert_number_at_sheet_with_grammar(
        &vlookup,
        main,
        "VLOOKUP(1,'DATA TABLE'!$A$8:'DATA TABLE'!$B$10,2)",
        FormulaGrammar::ExcelA1,
        3.0,
    );

    let sumproduct_sheet = SheetId(2);
    let sumproduct = FormulaEvaluationBook {
        sheet_names: vec![
            SheetBinding {
                id: main,
                name: Cow::Borrowed("Sheet1"),
            },
            SheetBinding {
                id: sumproduct_sheet,
                name: Cow::Borrowed("A"),
            },
        ],
        cells: BTreeMap::from([
            ((sumproduct_sheet, address("C6")), number_value(3.0)),
            ((sumproduct_sheet, address("C7")), number_value(4.0)),
            ((sumproduct_sheet, address("C67")), number_value(5.0)),
            ((sumproduct_sheet, address("C68")), number_value(6.0)),
            ((main, address("B7")), number_value(7.0)),
            ((main, address("B8")), number_value(8.0)),
            ((main, address("B68")), number_value(9.0)),
            ((main, address("B69")), number_value(10.0)),
        ]),
        ..FormulaEvaluationBook::default()
    };
    assert_number_at_sheet_with_grammar(
        &sumproduct,
        main,
        "SUMPRODUCT(A!C7:A!C67,B8:B68)/B69",
        FormulaGrammar::ExcelA1,
        7.7,
    );

    let lookup = evaluation_book(&[
        ("A1", string("P")),
        ("B1", string("Q")),
        ("C1", string("R")),
        ("A2", string("X")),
        ("B2", string("Y")),
        ("C2", string("Z")),
    ]);
    assert_text(&lookup, "LOOKUP(\"Q\",A1:C1)", "Q");
    assert_text(&lookup, "LOOKUP(\"R\",A1:C1)", "R");
    assert_text(&lookup, "LOOKUP(\"Q\",A1:C1,A1:C1)", "Q");
    assert_text(&lookup, "LOOKUP(\"R\",A1:C1,A1:C1)", "R");
    assert_text(&lookup, "LOOKUP(\"Q\",A1:C2)", "Y");
    assert_text(&lookup, "LOOKUP(\"R\",A1:C2)", "Z");
    assert_text(&lookup, "LOOKUP(\"Q\",A1:C1,A2:C2)", "Y");
    assert_text(&lookup, "LOOKUP(\"R\",A1:C1,A2:C2)", "Z");
    assert_text(&lookup, "LOOKUP(\"P\",A1:B2)", "Q");
    assert_text(&lookup, "LOOKUP(\"X\",A1:A2,C1:C2)", "Z");

    let duration = evaluation_book(&[("B1", number_value(0.0104166666666666))]);
    assert_text(&duration, r#"TEXT(B1,"h""""h"""" m""""m""""")"#, "0h 15m");

    let sumproduct_examples = evaluation_book(&[
        ("A1", number_value(1.0)),
        ("B1", number_value(1.0)),
        ("C1", number_value(10.0)),
        ("A2", number_value(2.0)),
        ("B2", number_value(10.0)),
        ("C2", number_value(20.0)),
        ("D2", number_value(9.0)),
        ("A3", number_value(1.0)),
        ("B3", number_value(20.0)),
        ("C3", number_value(30.0)),
        ("D3", number_value(10.0)),
        ("A4", number_value(2.0)),
        ("B4", number_value(30.0)),
        ("C4", number_value(40.0)),
        ("D4", number_value(7.0)),
        ("A5", number_value(3.0)),
        ("B5", number_value(40.0)),
        ("C5", number_value(3.25)),
        ("D5", number_value(11.0)),
        ("B12", string("Green Tea")),
        ("C12", string("Seattle")),
    ]);
    assert_number_with_epsilon(
        &sumproduct_examples,
        "SUMPRODUCT(C2:C5,D2:D5)",
        78.97,
        1e-10,
    );
    assert_number(&sumproduct_examples, "SUMPRODUCT(--(A1:A3))", 4.0);
    assert_number(&sumproduct_examples, "SUMPRODUCT(--(A1:A4>=2))", 2.0);
    assert_number(&sumproduct_examples, "SUMPRODUCT(--(A1:A4>=2),B1:B4)", 90.0);
    assert_number(&sumproduct_examples, "SUMPRODUCT(B1:B4,--(A1:A4>=2))", 90.0);
    assert_number(&sumproduct_examples, "SUMPRODUCT((A1:A4=B1)*C1:C4)", 40.0);

    let sumproduct_text = evaluation_book(&[
        ("A1", string("yes")),
        ("B1", number_value(10.0)),
        ("A2", string("no")),
        ("B2", number_value(20.0)),
        ("A3", string("yes")),
        ("B3", number_value(30.0)),
        ("A4", string("no")),
        ("B4", number_value(40.0)),
    ]);
    assert_number(&sumproduct_text, "SUMPRODUCT((A1:A4=\"yes\")*B1:B4)", 40.0);
}

#[test]
fn evaluates_apache_poi_range_and_coercion_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/eval/TestRangeEval.java::testRangeUsingOffsetFunc_bug46948
    // and TestOperandResolver string coercion tests.
    for (offset, expected) in [(1.0, 12.0), (2.0, 21.0), (0.0, 5.0)] {
        let book = evaluation_book(&[
            ("B1", number_value(offset)),
            ("C1", number_value(5.0)),
            ("D1", number_value(7.0)),
            ("E1", number_value(9.0)),
        ]);
        assert_number(&book, "SUM(C1:OFFSET(C1,0,B1))", expected);
    }

    let book = evaluation_book(&[]);
    for (formula, expected) in [
        ("\"2019/1/18\"+0", 43483.0),
        ("\"01/18/2019\"+0", 43483.0),
        ("\"18 Jan 2019\"+0", 43483.0),
        ("\"18-Jan-2019\"+0", 43483.0),
        ("\"2019/1/18 12:00\"+0", 43483.5),
        ("\"2019/1/18 6:00 AM\"+0", 43483.25),
        ("\"18-Jan-2019 6:00 PM\"+0", 43483.75),
        ("\"2019/1/18 15:15:15\"+0", 43483.63559027778),
        ("\"18-Jan-2019 6:15:15 PM\"+0", 43483.76059027778),
        ("\"00:00\"+0", 0.0),
        ("\"12:00\"+0", 0.5),
        ("\"15:43:09\"+0", 0.654965278),
        ("\"15:43\"+0", 0.654861111),
        ("\"3:43 PM\"+0", 0.654861111),
    ] {
        assert_number_with_epsilon(&book, formula, expected, 0.00001);
    }
}

#[test]
fn evaluates_apache_poi_atp_logical_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/atp/TestIfError.java,
    // TestIfna.java, TestIfs.java, and TestSwitch.java.
    let iferror = evaluation_book(&[
        ("A1", number_value(210.0)),
        ("A2", number_value(55.0)),
        ("B1", number_value(35.0)),
        ("B2", number_value(0.0)),
        ("C1", FormulaValue::Error(FormulaErrorValue::Div0)),
    ]);
    assert_number(&iferror, "IFERROR(A1/B1,\"Error in calculation\")", 6.0);
    assert_text(
        &iferror,
        "IFERROR(A2/B2,\"Error in calculation\")",
        "Error in calculation",
    );
    assert_text(&iferror, "IFERROR(C1,\"error\")", "error");

    let book = evaluation_book(&[]);
    assert_number(&book, "IFNA(-1,42)", -1.0);
    assert_number(&book, "IFNA(NA(),42)", 42.0);
    assert_text(&book, "IFNA(\"a1\",\"a2\")", "a1");
    assert_text(&book, "IFNA(NA(),\"a2\")", "a2");
    assert_error(&book, "IFNA(1)", FormulaErrorValue::Value);
    assert_error(&book, "IFNA(1,2,3)", FormulaErrorValue::Value);
    assert_error(&book, "IFNA(1/0,42)", FormulaErrorValue::Div0);
    assert_error(&book, "IFNA(NA(),1/0)", FormulaErrorValue::Div0);
    assert_number(&book, "IFNA(42,1/0)", 42.0);

    let a = evaluation_book(&[("A1", string("A"))]);
    assert_text(
        &a,
        "IFS(A1=\"A\", \"Value for A\", A1=\"B\",\"Value for B\")",
        "Value for A",
    );
    assert_text(
        &a,
        "SWITCH(A1, \"A\",\"Value for A\", \"B\",\"Value for B\", \"Something else\")",
        "Value for A",
    );

    let b = evaluation_book(&[("A1", string("B"))]);
    assert_text(
        &b,
        "IFS(A1=\"A\", \"Value for A\", A1=\"B\",\"Value for B\")",
        "Value for B",
    );
    assert_text(
        &b,
        "SWITCH(A1, \"A\",\"Value for A\", \"B\",\"Value for B\", \"Something else\")",
        "Value for B",
    );

    let empty = evaluation_book(&[("A1", string(""))]);
    assert_text(
        &empty,
        "SWITCH(A1, \"A\",\"Value for A\", \"B\",\"Value for B\", \"Something else\")",
        "Something else",
    );
}

#[test]
fn evaluates_apache_poi_atp_date_and_statistical_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/atp/TestNetworkdaysFunction.java,
    // TestWorkdayFunction.java, TestWorkdayIntlFunction.java, TestPercentile.java,
    // TestPercentRankIncFunction.java, TestPercentRankExcFunction.java,
    // TestRandBetween.java, and TestYearFracCalculator.java.
    let book = evaluation_book(&[]);
    assert_error(&book, "NETWORKDAYS()", FormulaErrorValue::Value);
    assert_error(
        &book,
        "NETWORKDAYS(\"2008/10/01\")",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "NETWORKDAYS(\"2008/10/01\",\"2009/03/01\",0,1)",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "NETWORKDAYS(\"Potato\",\"Cucumber\")",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "NETWORKDAYS(\"2009/03/01\",\"2008/10/01\")",
        FormulaErrorValue::Name,
    );
    assert_number(&book, "NETWORKDAYS(\"2008/10/01\",\"2009/03/01\")", 108.0);
    assert_number(
        &book,
        "NETWORKDAYS(\"2008/10/01\",\"2009/03/01\",\"2008/11/26\")",
        107.0,
    );

    let holidays = evaluation_book(&[
        ("A1", string("2008/11/26")),
        ("B1", string("2008/12/04")),
        ("C1", string("2009/01/21")),
    ]);
    assert_number(
        &holidays,
        "NETWORKDAYS(\"2008/10/01\",\"2009/03/01\",A1:C1)",
        105.0,
    );
    assert_number(&holidays, "WORKDAY(\"2008/10/01\",151,A1:C1)", 39938.0);
    assert_number(
        &holidays,
        "WORKDAY.INTL(\"2008/10/01\",151,,A1:C1)",
        39938.0,
    );

    assert_error(&book, "WORKDAY()", FormulaErrorValue::Value);
    assert_error(&book, "WORKDAY(\"2008/10/01\")", FormulaErrorValue::Value);
    assert_error(
        &book,
        "WORKDAY(\"2008/10/01\",151,0,1)",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "WORKDAY(\"Potato\",\"Cucumber\")",
        FormulaErrorValue::Value,
    );
    assert_number(&book, "WORKDAY(\"2008/10/01\",151)", 39932.0);
    assert_number(&book, "WORKDAY(\"2013/09/30\",-1)", 41544.0);
    assert_number(&book, "WORKDAY(\"2013/09/27\",1)", 41547.0);
    assert_number(&book, "WORKDAY(\"2013/10/06\",1)", 41554.0);
    assert_number(&book, "WORKDAY(\"2013/10/06\",-1)", 41551.0);
    assert_number(&book, "WORKDAY(\"2008/10/01\",151.99999)", 39932.0);
    assert_number(&book, "WORKDAY(\"2008/10/01\",-5,\"2008/09/29\")", 39714.0);

    assert_error(&book, "WORKDAY.INTL()", FormulaErrorValue::Value);
    assert_error(
        &book,
        "WORKDAY.INTL(\"2008/10/01\")",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "WORKDAY.INTL(\"2008/10/01\",151,,0,1)",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "WORKDAY.INTL(\"Potato\",\"Cucumber\")",
        FormulaErrorValue::Value,
    );
    assert_number(&book, "WORKDAY.INTL(\"2008/10/01\",151)", 39932.0);
    assert_number(&book, "WORKDAY.INTL(\"2013/09/30\",-1)", 41544.0);
    assert_number(&book, "WORKDAY.INTL(\"2013/09/27\",1)", 41547.0);
    assert_number(&book, "WORKDAY.INTL(\"2013/10/06\",1)", 41554.0);
    assert_number(&book, "WORKDAY.INTL(\"2013/10/06\",-1)", 41551.0);
    assert_number(&book, "WORKDAY.INTL(\"2008/10/01\",151.99999)", 39932.0);
    assert_number(
        &book,
        "WORKDAY.INTL(\"2008/10/01\",-5,,\"2008/09/29\")",
        39714.0,
    );
    assert_error(
        &book,
        "WORKDAY.INTL(\"2012-01-01\",30,0)",
        FormulaErrorValue::Num,
    );
    assert_number(&book, "WORKDAY.INTL(\"2012-01-01\",90,11)", 41013.0);
    assert_number(&book, "WORKDAY.INTL(\"2012-01-01\",30,17)", 40944.0);

    let percentile = evaluation_book(&[
        ("A1", number_value(210.128)),
        ("A2", number_value(65.2182)),
        ("A3", number_value(32.231)),
        ("A4", number_value(12.123)),
        ("A5", number_value(45.32)),
        ("B1", number_value(210.128)),
        ("B2", number_value(65.2182)),
        ("B3", number_value(32.231)),
        ("B4", FormulaValue::Blank),
        ("B5", number_value(45.32)),
        ("C1", number_value(1.0)),
        ("C2", FormulaValue::Error(FormulaErrorValue::Name)),
        ("C3", number_value(3.0)),
        ("C4", FormulaValue::Error(FormulaErrorValue::Div0)),
    ]);
    assert_number_with_epsilon(&percentile, "PERCENTILE(A1:A5,0.95)", 181.14604, 0.00000001);
    assert_number_with_epsilon(&percentile, "PERCENTILE(B1:B5,0.95)", 188.39153, 0.00000001);
    assert_number_with_epsilon(&book, "PERCENTILE({1,2,TRUE,FALSE},0.95)", 1.95, 0.00000001);
    assert_error(&book, "PERCENTILE({1,2},-0.1)", FormulaErrorValue::Num);
    assert_error(&book, "PERCENTILE({1,2},1.1)", FormulaErrorValue::Num);
    assert_error(
        &percentile,
        "PERCENTILE(C1:C4,0.95)",
        FormulaErrorValue::Name,
    );

    let percentrank1 = evaluation_book(&[
        ("A2", number_value(13.0)),
        ("A3", number_value(12.0)),
        ("A4", number_value(11.0)),
        ("A5", number_value(8.0)),
        ("A6", number_value(4.0)),
        ("A7", number_value(3.0)),
        ("A8", number_value(2.0)),
        ("A9", number_value(1.0)),
        ("A10", number_value(1.0)),
        ("A11", number_value(1.0)),
    ]);
    for (formula, expected) in [
        ("PERCENTRANK.INC(A2:A11,2)", 0.333),
        ("PERCENTRANK.INC(A2:A11,4)", 0.555),
        ("PERCENTRANK.INC(A2:A11,8)", 0.666),
        ("PERCENTRANK.INC(A2:A11,8,2)", 0.66),
        ("PERCENTRANK.INC(A2:A11,8,4)", 0.6666),
        ("PERCENTRANK.INC(A2:A11,5)", 0.583),
        ("PERCENTRANK.INC(A2:A11,5,5)", 0.58333),
        ("PERCENTRANK.INC(A2:A11,1)", 0.0),
        ("PERCENTRANK.INC(A2:A11,13)", 1.0),
        ("PERCENTRANK.EXC(A2:A11,1)", 0.09),
        ("PERCENTRANK.EXC(A2:A11,13)", 0.909),
        ("PERCENTRANK.EXC(A2:A11,2)", 0.363),
        ("PERCENTRANK.EXC(A2:A11,4)", 0.545),
        ("PERCENTRANK.EXC(A2:A11,8)", 0.636),
        ("PERCENTRANK.EXC(A2:A11,8,2)", 0.63),
        ("PERCENTRANK.EXC(A2:A11,8,4)", 0.6363),
        ("PERCENTRANK.EXC(A2:A11,5)", 0.568),
    ] {
        assert_number_with_epsilon(&percentrank1, formula, expected, 0.00001);
    }
    assert_error(
        &percentrank1,
        "PERCENTRANK.INC(A2:A11,0)",
        FormulaErrorValue::NA,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.INC(A2:A11,100)",
        FormulaErrorValue::NA,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.INC(B2:B11,100)",
        FormulaErrorValue::Num,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.INC(A2:A11,8,0)",
        FormulaErrorValue::Num,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.EXC(A2:A11,0)",
        FormulaErrorValue::NA,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.EXC(A2:A11,100)",
        FormulaErrorValue::NA,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.EXC(B2:B11,100)",
        FormulaErrorValue::Num,
    );
    assert_error(
        &percentrank1,
        "PERCENTRANK.EXC(A2:A11,8,0)",
        FormulaErrorValue::Num,
    );

    let percentrank2 = evaluation_book(&[
        ("A2", number_value(1.0)),
        ("A3", number_value(2.0)),
        ("A4", number_value(3.0)),
        ("A5", number_value(6.0)),
        ("A6", number_value(6.0)),
        ("A7", number_value(6.0)),
        ("A8", number_value(7.0)),
        ("A9", number_value(8.0)),
        ("A10", number_value(9.0)),
    ]);
    assert_number_with_epsilon(&percentrank2, "PERCENTRANK.EXC(A2:A10,7)", 0.7, 0.00001);
    assert_number_with_epsilon(
        &percentrank2,
        "PERCENTRANK.EXC(A2:A10,5.43)",
        0.381,
        0.00001,
    );
    assert_number_with_epsilon(
        &percentrank2,
        "PERCENTRANK.EXC(A2:A10,5.43,1)",
        0.3,
        0.00001,
    );

    assert_number(&book, "RANDBETWEEN(1,1)", 1.0);
    assert_number(&book, "RANDBETWEEN(-1,-1)", -1.0);
    assert_number_in_range(&book, "RANDBETWEEN(0,9999999999)", 0.0, 9999999999.0);
    assert_error(&book, "RANDBETWEEN(1,0)", FormulaErrorValue::Num);
    assert_error(&book, "RANDBETWEEN(\"STRING\",1)", FormulaErrorValue::Value);
    assert_error(&book, "RANDBETWEEN(1,\"STRING\")", FormulaErrorValue::Value);
    assert_error(
        &book,
        "RANDBETWEEN(\"STRING\",\"STRING\")",
        FormulaErrorValue::Value,
    );

    for (formula, expected) in [
        ("YEARFRAC(DATE(1999,1,1),DATE(1999,4,5),1)", 0.257534247),
        ("YEARFRAC(DATE(1999,4,1),DATE(1999,4,5),1)", 0.010958904),
        ("YEARFRAC(DATE(1999,4,1),DATE(1999,4,4),1)", 0.008219178),
        ("YEARFRAC(DATE(1999,4,2),DATE(1999,4,5),1)", 0.008219178),
        ("YEARFRAC(DATE(1999,3,31),DATE(1999,4,3),1)", 0.008219178),
        ("YEARFRAC(DATE(1999,4,5),DATE(1999,4,8),1)", 0.008219178),
        ("YEARFRAC(DATE(1999,4,4),DATE(1999,4,7),1)", 0.008219178),
        ("YEARFRAC(DATE(2000,2,5),DATE(2000,6,1),0)", 0.322222222),
    ] {
        assert_number_with_epsilon(&book, formula, expected, 0.000000001);
    }
}

#[test]
fn evaluates_apache_poi_value_and_numbervalue_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestValue.java
    // and TestNumberValue.java.
    let book = evaluation_book(&[]);
    for (formula, expected) in [
        ("VALUE(\"100\")", 100.0),
        ("VALUE(\"-2.3\")", -2.3),
        ("VALUE(\".5\")", 0.5),
        ("VALUE(\".5e2\")", 50.0),
        ("VALUE(\".5e-2\")", 0.005),
        ("VALUE(\".5e+2\")", 50.0),
        ("VALUE(\"+5\")", 5.0),
        ("VALUE(\"$1,000\")", 1000.0),
        ("VALUE(\"100.5e1\")", 1005.0),
        ("VALUE(\"1,0000\")", 10000.0),
        ("VALUE(\"1,000,0000\")", 10000000.0),
        ("VALUE(\"1,000,0000,00000\")", 1000000000000.0),
        ("VALUE(\" 100 \")", 100.0),
        ("VALUE(\" + 100\")", 100.0),
        ("VALUE(\"10000\")", 10000.0),
        ("VALUE(\"$-5\")", -5.0),
        ("VALUE(\"$.5\")", 0.5),
        ("VALUE(\"123e+5\")", 12300000.0),
        ("VALUE(\"1,000e2\")", 100000.0),
        ("VALUE(\"$10e2\")", 1000.0),
        ("VALUE(\"$1,000e2\")", 100000.0),
        ("VALUE(\"30%\")", 0.3),
        ("VALUE(\"30 %\")", 0.3),
        ("VALUE(\"4:48:00\")", 0.2),
        ("VALUE(\"1 January 2025\")", 45658.0),
        ("VALUE(\"01 January 2025\")", 45658.0),
        ("VALUE(\"1 Jan 2025\")", 45658.0),
        ("VALUE(\"01 Jan 2025\")", 45658.0),
        ("NUMBERVALUE(\"2.500,27\",\",\",\".\")", 2500.27),
        ("NUMBERVALUE(\" 2.500,27 \",\",\",\".\")", 2500.27),
        ("NUMBERVALUE(\"3.5%\")", 0.035),
        ("NUMBERVALUE(\"9%%\")", 0.0009),
    ] {
        assert_number_with_epsilon(&book, formula, expected, 0.000000000001);
    }

    for formula in [
        "VALUE(\"1+1\")",
        "VALUE(\"1 1\")",
        "VALUE(\"1,00.0\")",
        "VALUE(\"1,00\")",
        "VALUE(\"$1,00.5e1\")",
        "VALUE(\"1,00.5e1\")",
        "VALUE(\"1,0,000\")",
        "VALUE(\"1,00,000\")",
        "VALUE(\"++100\")",
        "VALUE(\"$$5\")",
        "VALUE(\"-\")",
        "VALUE(\"+\")",
        "VALUE(\"$\")",
        "VALUE(\",300\")",
        "VALUE(\"0.233,4\")",
        "VALUE(\"1e2.5\")",
        "VALUE(\"\")",
        "NUMBERVALUE(\"notnum\")",
        "NUMBERVALUE(\"2,00,27\",\",\",\".\")",
    ] {
        assert_error(&book, formula, FormulaErrorValue::Value);
    }

    let blank = evaluation_book(&[("A1", FormulaValue::Blank), ("B1", FormulaValue::Blank)]);
    assert_number(&blank, "VALUE(A1)", 0.0);
    assert_number(&blank, "VALUE(B1)", 0.0);
}

#[test]
fn evaluates_apache_poi_textjoin_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/atp/TestTextJoinFunction.java.
    let book = evaluation_book(&[
        ("A1", string("One")),
        ("B1", string("Two")),
        ("C1", FormulaValue::Blank),
        ("D1", number_value(1.0)),
        ("E1", number_value(2.0)),
    ]);
    assert_text(&book, "TEXTJOIN(\",\", TRUE, \"Text\")", "Text");
    assert_text(
        &book,
        "TEXTJOIN(\",\", TRUE, \"One\", \"Two\", \"Three\")",
        "One,Two,Three",
    );
    assert_text(&book, "TEXTJOIN(\",\", TRUE, \"Text\", 1)", "Text,1");
    assert_text(&book, "TEXTJOIN(\",\", FALSE, \"A\", \"\", \"B\")", "A,,B");
    assert_text(&book, "TEXTJOIN(\",\", TRUE, \"A\", \"\", \"B\")", "A,B");
    assert_text(&book, "TEXTJOIN(\",\", FALSE, \"\", \"\")", ",");
    assert_text(&book, "TEXTJOIN(\",\", TRUE, \"\", \"\")", "");
    assert_text(&book, "TEXTJOIN(\",\", TRUE, A1, B1)", "One,Two");
    assert_text(&book, "TEXTJOIN(\",\", TRUE, D1, E1)", "1,2");
    assert_text(&book, "TEXTJOIN(\",\", FALSE, A1, C1, B1)", "One,,Two");
    assert_text(&book, "TEXTJOIN(\",\", TRUE, A1, C1, B1)", "One,Two");
    assert_error(&book, "TEXTJOIN(\",\", TRUE)", FormulaErrorValue::Value);

    let currencies = evaluation_book(&[
        ("A2", string("US Dollar")),
        ("A3", string("Australian Dollar")),
        ("A4", string("Chinese Yuan")),
        ("A5", string("Hong Kong Dollar")),
        ("A6", string("Israeli Shekel")),
        ("A7", string("South Korean Won")),
        ("A8", string("Russian Ruble")),
    ]);
    assert_text(
        &currencies,
        "TEXTJOIN(\", \", TRUE, A2:A8)",
        "US Dollar, Australian Dollar, Chinese Yuan, Hong Kong Dollar, Israeli Shekel, South Korean Won, Russian Ruble",
    );

    let table = evaluation_book(&[
        ("A2", string("a1")),
        ("B2", string("b1")),
        ("A3", string("a2")),
        ("B3", string("b2")),
        ("A4", FormulaValue::Blank),
        ("B4", FormulaValue::Blank),
        ("A5", string("a4")),
        ("B5", string("b4")),
        ("A6", string("a5")),
        ("B6", string("b5")),
        ("A7", string("a6")),
        ("B7", string("b6")),
        ("A8", string("a7")),
        ("B8", string("b7")),
    ]);
    assert_text(
        &table,
        "TEXTJOIN(\", \", TRUE, A2:B8)",
        "a1, b1, a2, b2, a4, b4, a5, b5, a6, b6, a7, b7",
    );
    assert_text(
        &table,
        "TEXTJOIN(\", \", FALSE, A2:B8)",
        "a1, b1, a2, b2, , , a4, b4, a5, b5, a6, b6, a7, b7",
    );

    let addresses = evaluation_book(&[
        ("A2", string("Tulsa")),
        ("B2", string("OK")),
        ("C2", string("74133")),
        ("D2", string("US")),
        ("A3", string("Seattle")),
        ("B3", string("WA")),
        ("C3", string("98109")),
        ("D3", string("US")),
        ("A4", string("Iselin")),
        ("B4", string("NJ")),
        ("C4", string("08830")),
        ("D4", string("US")),
        ("A5", string("Fort Lauderdale")),
        ("B5", string("FL")),
        ("C5", string("33309")),
        ("D5", string("US")),
        ("A6", string("Tempe")),
        ("B6", string("AZ")),
        ("C6", string("85285")),
        ("D6", string("US")),
        ("A7", string("end")),
        ("A8", string(",")),
        ("B8", string(",")),
        ("C8", string(",")),
        ("D8", string(";")),
    ]);
    assert_text(
        &addresses,
        "TEXTJOIN(A8:D8, TRUE, A2:D7)",
        "Tulsa,OK,74133,US;Seattle,WA,98109,US;Iselin,NJ,08830,US;Fort Lauderdale,FL,33309,US;Tempe,AZ,85285,US;end",
    );
    assert_text(
        &addresses,
        "TEXTJOIN(, TRUE, A2:D7)",
        "TulsaOK74133USSeattleWA98109USIselinNJ08830USFort LauderdaleFL33309USTempeAZ85285USend",
    );
}

#[test]
fn evaluates_apache_poi_mround_and_error_predicate_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/atp/TestMRound.java
    // and poi/src/test/java/org/apache/poi/ss/formula/functions/TestLogicalFunction.java.
    let book = evaluation_book(&[]);
    assert_number(&book, "MROUND(10, 3)", 9.0);
    assert_number(&book, "MROUND(-10, -3)", -9.0);
    assert_number(&book, "MROUND(1.3, 0.2)", 1.4);
    assert_error(&book, "MROUND(5, -2)", FormulaErrorValue::Num);
    assert_number(&book, "MROUND(5, 0)", 0.0);
    assert_number(&book, "MROUND(0.79*7.5, 0.05)", 5.95);

    let bug66189 = evaluation_book(&[
        ("A1", number_value(5.0)),
        ("A2", number_value(1.2205)),
        ("B2", number_value(1.175)),
        ("C1", number_value(0.19775)),
    ]);
    assert_number_with_epsilon(&bug66189, "(A2+(B2-A2)*A1/10)-1", 0.19775, 1e-12);
    assert_number_with_epsilon(&bug66189, "ROUND(C1 * 100, 2)", 19.78, 1e-12);
    assert_number_with_epsilon(&bug66189, "MROUND(C1 * 100, 2)", 19.78, 1e-12);

    let errors = evaluation_book(&[
        ("B1", FormulaValue::Error(FormulaErrorValue::Div0)),
        ("B2", FormulaValue::Error(FormulaErrorValue::NA)),
    ]);
    assert_boolean(&errors, "ISERR(B1)", true);
    assert_boolean(&errors, "ISERR(B2)", false);
    assert_boolean(&errors, "ISERROR(B1)", true);
    assert_boolean(&errors, "ISERROR(B2)", true);
}

#[test]
fn evaluates_apache_poi_conditional_aggregate_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestSumif.java,
    // TestSumifs.java, and TestAverageIf.java. Low-level Eval API assertions are
    // intentionally reduced to equivalent formula-string behavior.
    let commissions = evaluation_book(&[
        ("A2", number_value(100000.0)),
        ("B2", number_value(7000.0)),
        ("C2", number_value(250000.0)),
        ("A3", number_value(200000.0)),
        ("B3", number_value(14000.0)),
        ("A4", number_value(300000.0)),
        ("B4", number_value(21000.0)),
        ("A5", number_value(400000.0)),
        ("B5", number_value(28000.0)),
    ]);
    assert_number(&commissions, "SUMIF(A2:A5,\">160000\",B2:B5)", 63000.0);
    assert_number(&commissions, "SUMIF(A2:A5,\">160000\")", 900000.0);
    assert_number(&commissions, "SUMIF(A2:A5,300000,B2:B5)", 21000.0);
    assert_number(&commissions, "SUMIF(A2:A5,\">\" & C2,B2:B5)", 49000.0);
    assert_number(&commissions, "AVERAGEIF(B2:B5,\"<23000\")", 14000.0);
    assert_number(&commissions, "AVERAGEIF(A2:A5,\"<250000\",A2:A5)", 150000.0);
    assert_error(
        &commissions,
        "AVERAGEIF(A2:A5,\"<95000\",A2:A5)",
        FormulaErrorValue::Div0,
    );
    assert_number(&commissions, "AVERAGEIF(A2:A5,\">250000\",B2:B5)", 24500.0);

    let commissions_with_na = evaluation_book(&[
        ("A2", number_value(100000.0)),
        ("B2", number_value(7000.0)),
        ("A3", number_value(200000.0)),
        ("B3", number_value(14000.0)),
        ("A4", number_value(300000.0)),
        ("B4", number_value(21000.0)),
        ("A5", number_value(400000.0)),
        ("B5", number_value(28000.0)),
        ("A6", number_value(500000.0)),
        ("B6", FormulaValue::Error(FormulaErrorValue::NA)),
    ]);
    assert_error(
        &commissions_with_na,
        "SUMIF(A2:A6,\">160000\",B2:B6)",
        FormulaErrorValue::NA,
    );

    let commissions_with_non_numbers = evaluation_book(&[
        ("A2", number_value(100000.0)),
        ("B2", number_value(7000.0)),
        ("A3", number_value(200000.0)),
        ("B3", number_value(14000.0)),
        ("A4", number_value(300000.0)),
        ("B4", number_value(21000.0)),
        ("A5", number_value(400000.0)),
        ("B5", number_value(28000.0)),
        ("A6", number_value(500000.0)),
        ("B6", FormulaValue::Boolean(true)),
        ("A7", number_value(600000.0)),
        ("B7", string("abc")),
    ]);
    assert_number(
        &commissions_with_non_numbers,
        "SUMIF(A2:A7,\">160000\",B2:B7)",
        63000.0,
    );

    let sales = evaluation_book(&[
        ("A2", string("Vegetables")),
        ("B2", string("Tomatoes")),
        ("C2", number_value(2300.0)),
        ("A3", string("Vegetables")),
        ("B3", string("Celery")),
        ("C3", number_value(5500.0)),
        ("A4", string("Fruits")),
        ("B4", string("Oranges")),
        ("C4", number_value(800.0)),
        ("A5", FormulaValue::Blank),
        ("B5", string("Butter")),
        ("C5", number_value(400.0)),
        ("A6", string("Vegetables")),
        ("B6", string("Carrots")),
        ("C6", number_value(4200.0)),
        ("A7", string("Fruits")),
        ("B7", string("Apples")),
        ("C7", number_value(1200.0)),
    ]);
    assert_number(&sales, "SUMIF(A2:A7,\"Fruits\",C2:C7)", 2000.0);
    assert_number(&sales, "SUMIF(A2:A7,\"Vegetables\",C2:C7)", 12000.0);
    assert_number(&sales, "SUMIF(B2:B7,\"*es\",C2:C7)", 4300.0);
    assert_number(&sales, "SUMIF(A2:A7,\"\",C2:C7)", 400.0);

    let sumifs_example1 = evaluation_book(&[
        ("A2", number_value(5.0)),
        ("B2", string("Apples")),
        ("C2", number_value(1.0)),
        ("A3", number_value(4.0)),
        ("B3", string("Apples")),
        ("C3", number_value(2.0)),
        ("A4", number_value(15.0)),
        ("B4", string("Artichokes")),
        ("C4", number_value(1.0)),
        ("A5", number_value(3.0)),
        ("B5", string("Artichokes")),
        ("C5", number_value(2.0)),
        ("A6", number_value(22.0)),
        ("B6", string("Bananas")),
        ("C6", number_value(1.0)),
        ("A7", number_value(12.0)),
        ("B7", string("Bananas")),
        ("C7", number_value(2.0)),
        ("A8", number_value(10.0)),
        ("B8", string("Carrots")),
        ("C8", number_value(1.0)),
        ("A9", number_value(33.0)),
        ("B9", string("Carrots")),
        ("C9", number_value(2.0)),
    ]);
    assert_number(&sumifs_example1, "SUMIFS(A2:A9,B2:B9,\"A*\",C2:C9,1)", 20.0);
    assert_number(
        &sumifs_example1,
        "SUMIFS(A2:A9,B2:B9,\"<>Bananas\",C2:C9,1)",
        30.0,
    );
    assert_error(
        &sumifs_example1,
        "SUMIFS(A2:A9,B2:B8,\"<>Bananas\",C2:C9,1)",
        FormulaErrorValue::Value,
    );

    let sumifs_example2 = evaluation_book(&[
        ("B2", number_value(100.0)),
        ("C2", number_value(390.0)),
        ("D2", number_value(8321.0)),
        ("E2", number_value(500.0)),
        ("B3", number_value(0.01)),
        ("C3", number_value(0.005)),
        ("D3", number_value(0.03)),
        ("E3", number_value(0.04)),
        ("B4", number_value(0.01)),
        ("C4", number_value(0.013)),
        ("D4", number_value(0.021)),
        ("E4", number_value(0.02)),
        ("B5", number_value(0.005)),
        ("C5", number_value(0.03)),
        ("D5", number_value(0.01)),
        ("E5", number_value(0.04)),
    ]);
    assert_number(
        &sumifs_example2,
        "SUMIFS(B2:E2,B3:E3,\">0.03\",B4:E4,\">=0.02\",B5:E5,\">=0.01\")",
        500.0,
    );

    let sumifs_example3 = evaluation_book(&[
        ("B2", number_value(3.3)),
        ("C2", number_value(0.8)),
        ("D2", number_value(5.5)),
        ("E2", number_value(5.5)),
        ("B3", number_value(55.0)),
        ("C3", number_value(39.0)),
        ("D3", number_value(39.0)),
        ("E3", number_value(57.5)),
        ("B4", number_value(6.5)),
        ("C4", number_value(19.5)),
        ("D4", number_value(6.0)),
        ("E4", number_value(6.5)),
    ]);
    assert_number(
        &sumifs_example3,
        "SUMIFS(B2:E2,B3:E3,\">=40\",B4:E4,\"<10\")",
        8.8,
    );

    let regions = evaluation_book(&[
        ("A2", string("East")),
        ("B2", number_value(45678.0)),
        ("A3", string("West")),
        ("B3", number_value(23789.0)),
        ("A4", string("North")),
        ("B4", number_value(-4789.0)),
        ("A5", string("South (New Office)")),
        ("B5", number_value(0.0)),
        ("A6", string("Midwest")),
        ("B6", number_value(9678.0)),
    ]);
    assert_number(&regions, "AVERAGEIF(A2:A6,\"=*West\",B2:B6)", 16733.5);
    assert_number(
        &regions,
        "AVERAGEIF(A2:A6,\"<>*(New Office)\",B2:B6)",
        18589.0,
    );

    let grades = evaluation_book(&[
        ("B2", string("Quiz")),
        ("B3", string("Grade")),
        ("B4", number_value(75.0)),
        ("B5", number_value(94.0)),
        ("C2", string("Quiz")),
        ("C3", string("Grade")),
        ("C4", number_value(85.0)),
        ("C5", number_value(80.0)),
    ]);
    assert_number(
        &grades,
        "AVERAGEIFS(B2:B5,B2:B5,\">70\",B2:B5,\"<90\")",
        75.0,
    );
    assert_error(
        &grades,
        "AVERAGEIFS(C2:C5,C2:C5,\">95\")",
        FormulaErrorValue::Div0,
    );

    let max_min_1 = evaluation_book(&[
        ("A2", number_value(89.0)),
        ("A3", number_value(93.0)),
        ("A4", number_value(96.0)),
        ("A5", number_value(85.0)),
        ("A6", number_value(91.0)),
        ("A7", number_value(88.0)),
        ("B2", number_value(1.0)),
        ("B3", number_value(2.0)),
        ("B4", number_value(2.0)),
        ("B5", number_value(3.0)),
        ("B6", number_value(1.0)),
        ("B7", number_value(1.0)),
    ]);
    assert_number(&max_min_1, "MAXIFS(A2:A7,B2:B7,1)", 91.0);
    assert_number(&max_min_1, "MINIFS(A2:A7,B2:B7,1)", 88.0);

    let max_min_2 = evaluation_book(&[
        ("A2", number_value(10.0)),
        ("A3", number_value(11.0)),
        ("A4", number_value(100.0)),
        ("A5", number_value(111.0)),
        ("B3", string("a")),
        ("B4", string("a")),
        ("B5", string("b")),
        ("B6", string("a")),
    ]);
    assert_number(&max_min_2, "MAXIFS(A2:A5,B3:B6,\"a\")", 111.0);
    assert_number(&max_min_2, "MINIFS(A2:A5,B3:B6,\"a\")", 10.0);
}

#[test]
fn evaluates_apache_poi_count_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestCountFuncs.java.
    let countblank = evaluation_book(&[
        ("A1", number_value(0.0)),
        ("A2", string("")),
        ("A3", FormulaValue::Boolean(true)),
        ("B1", FormulaValue::Boolean(false)),
        ("B2", FormulaValue::Error(FormulaErrorValue::Div0)),
        ("B3", FormulaValue::Blank),
    ]);
    assert_number(&countblank, "COUNTBLANK(A1:B3)", 2.0);

    let counta = evaluation_book(&[
        ("A1", number_value(0.0)),
        ("A2", number_value(0.0)),
        ("A3", string("")),
        ("D2", FormulaValue::Blank),
        ("E2", FormulaValue::Blank),
        ("F2", FormulaValue::Blank),
        ("D3", FormulaValue::Blank),
        ("E3", FormulaValue::Blank),
        ("F3", FormulaValue::Blank),
        ("D4", FormulaValue::Blank),
        ("E4", FormulaValue::Blank),
        ("F4", FormulaValue::Blank),
        ("D5", FormulaValue::Blank),
        ("E5", FormulaValue::Blank),
        ("F5", FormulaValue::Blank),
    ]);
    assert_number(&counta, "COUNTA(A1)", 1.0);
    assert_number(&counta, "COUNTA(A1:A3)", 3.0);
    assert_number(&counta, "COUNTA(D2:F5)", 12.0);

    let boolean_criteria = evaluation_book(&[
        ("A1", number_value(0.0)),
        ("A2", string("TRUE")),
        ("A3", FormulaValue::Boolean(true)),
        ("B1", FormulaValue::Boolean(false)),
        ("B2", FormulaValue::Boolean(true)),
        ("B3", FormulaValue::Blank),
    ]);
    assert_number(&boolean_criteria, "COUNTIF(A1:B3,TRUE)", 2.0);

    let numeric_criteria = evaluation_book(&[
        ("A1", number_value(0.0)),
        ("A2", string("2")),
        ("A3", string("2.001")),
        ("B1", number_value(2.0)),
        ("B2", number_value(2.0)),
        ("B3", FormulaValue::Boolean(true)),
    ]);
    assert_number(&numeric_criteria, "COUNTIF(A1:B3,2)", 3.0);
    assert_number(&numeric_criteria, "COUNTIF(A1:B3,\"2.00\")", 3.0);
    assert_number(&numeric_criteria, "COUNTIF(A1:B3,\">1\")", 2.0);
    assert_number(&numeric_criteria, "COUNTIF(A1:B3,\">0.5\")", 2.0);

    let not_equal_text = evaluation_book(&[
        ("A1", string("aa")),
        ("A2", string("def")),
        ("A3", string("aa")),
        ("A4", string("ghi")),
        ("A5", string("aa")),
        ("A6", string("aa")),
    ]);
    assert_number(&not_equal_text, "COUNTIF(A1:A6,\"<>aa\")", 2.0);

    let wildcard_text = evaluation_book(&[
        ("A1", string("ab")),
        ("A2", string("aabb")),
        ("A3", string("aa")),
        ("A4", string("abb")),
        ("A5", string("aab")),
        ("A6", string("ba")),
    ]);
    assert_number(&wildcard_text, "COUNTIF(A1:A6,\"<>a*b\")", 2.0);

    let mixed_not_equal = evaluation_book(&[
        ("A1", number_value(222.0)),
        ("A2", number_value(222.0)),
        ("A3", number_value(111.0)),
        ("A4", string("aa")),
        ("A5", string("111")),
    ]);
    assert_number(&mixed_not_equal, "COUNTIF(A1:A5,\"<>111\")", 4.0);

    let case_insensitive = evaluation_book(&[
        ("A1", string("no")),
        ("A2", string("NO")),
        ("A3", string("No")),
        ("A4", string("Yes")),
    ]);
    assert_number(&case_insensitive, "COUNTIF(A1:A4,\"no\")", 3.0);
    assert_number(&case_insensitive, "COUNTIF(A1:A4,\"NO\")", 3.0);
    assert_number(&case_insensitive, "COUNTIF(A1:A4,\"No\")", 3.0);

    let criteria_reference = evaluation_book(&[
        ("A1", number_value(25.0)),
        ("C1", number_value(22.0)),
        ("C2", number_value(25.0)),
        ("C3", number_value(21.0)),
        ("C4", number_value(25.0)),
        ("C5", number_value(25.0)),
        ("C6", number_value(25.0)),
    ]);
    assert_number(&criteria_reference, "COUNTIF(C1:C6,A1)", 4.0);
}

#[test]
fn evaluates_apache_poi_subtotal_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestSubtotal.java.
    let basics = evaluation_book(&[
        ("C1", number_value(1.0)),
        ("D1", number_value(2.0)),
        ("C2", number_value(3.0)),
        ("D2", number_value(4.0)),
        ("C3", number_value(5.0)),
        ("D3", number_value(6.0)),
        ("C4", number_value(7.0)),
        ("D4", number_value(8.0)),
        ("C5", number_value(9.0)),
        ("D5", number_value(10.0)),
    ]);
    assert_number(&basics, "SUBTOTAL(9,C1:D5)", 55.0);
    assert_number(&basics, "SUBTOTAL(1,C1:D5)", 5.5);
    assert_number(&basics, "SUBTOTAL(2,C1:D5)", 10.0);
    assert_number(&basics, "SUBTOTAL(4,C1:D5)", 10.0);
    assert_number(&basics, "SUBTOTAL(5,C1:D5)", 1.0);
    assert_number(&basics, "SUBTOTAL(6,C1:D5)", 3628800.0);
    assert_number_with_epsilon(&basics, "SUBTOTAL(7,C1:D5)", 3.0276503540974917, 1e-12);

    let nested_numeric = evaluation_book(&[
        ("B2", number_value(1.0)),
        ("B3", number_value(3.0)),
        ("B5", number_value(1.0)),
        ("B6", number_value(7.0)),
    ]);
    assert_number(&nested_numeric, "SUBTOTAL(1,B2:B3)", 2.0);
    assert_number(&nested_numeric, "SUBTOTAL(1,B2:B6)*2 + 2", 8.0);
    assert_number(&nested_numeric, "SUBTOTAL(1,B2:B3,B5:B6)", 3.0);
    assert_number(&nested_numeric, "SUBTOTAL(9,B2:B3)", 4.0);
    assert_number(&nested_numeric, "SUBTOTAL(9,B2:B6)*2 + 2", 26.0);
    assert_number(&nested_numeric, "SUBTOTAL(9,B2:B3,B5:B6)", 12.0);
    assert_number(&nested_numeric, "SUBTOTAL(4,B2:B3)", 3.0);
    assert_number(&nested_numeric, "SUBTOTAL(4,B2:B6)*2 + 2", 16.0);
    assert_number(&nested_numeric, "SUBTOTAL(4,B2:B3,B5:B6)", 7.0);
    assert_number(&nested_numeric, "SUBTOTAL(5,B2:B3)", 1.0);
    assert_number(&nested_numeric, "SUBTOTAL(5,B2:B6)*2 + 2", 4.0);
    assert_number(&nested_numeric, "SUBTOTAL(5,B2:B3,B5:B6)", 1.0);
    assert_number_with_epsilon(&nested_numeric, "SUBTOTAL(7,B2:B3)", 1.41421, 0.00001);
    assert_number_with_epsilon(&nested_numeric, "SUBTOTAL(7,B2:B6)*2 + 2", 7.65685, 0.00001);
    assert_number_with_epsilon(&nested_numeric, "SUBTOTAL(7,B2:B3,B5:B6)", 2.82842, 0.00001);
    assert_number_with_epsilon(&nested_numeric, "SUBTOTAL(8,B2:B3)", 1.0, 0.00001);
    assert_number_with_epsilon(
        &nested_numeric,
        "SUBTOTAL(8,B2:B6)*2 + 2",
        6.898979,
        0.00001,
    );
    assert_number_with_epsilon(&nested_numeric, "SUBTOTAL(8,B2:B3,B5:B6)", 2.44949, 0.00001);
    assert_number(&nested_numeric, "SUBTOTAL(10,B2:B3)", 2.0);
    assert_number(&nested_numeric, "SUBTOTAL(10,B2:B6)*2 + 2", 18.0);
    assert_number(&nested_numeric, "SUBTOTAL(10,B2:B3,B5:B6)", 8.0);
    assert_number(&nested_numeric, "SUBTOTAL(11,B2:B3)", 1.0);
    assert_number(&nested_numeric, "SUBTOTAL(11,B2:B6)*2 + 2", 14.0);
    assert_number(&nested_numeric, "SUBTOTAL(11,B2:B3,B5:B6)", 6.0);

    let count_book = evaluation_book(&[
        ("B2", number_value(1.0)),
        ("B3", number_value(3.0)),
        ("B5", string("POI")),
        ("B6", FormulaValue::Blank),
    ]);
    assert_number(&count_book, "SUBTOTAL(2,B2:B3)", 2.0);
    assert_number(&count_book, "SUBTOTAL(2,B2:B6)*2 + 2", 6.0);
    assert_number(&count_book, "SUBTOTAL(2,B2:B6)", 2.0);
    assert_number(&count_book, "SUBTOTAL(3,B2:B3)", 2.0);
    assert_number(&count_book, "SUBTOTAL(3,B2:B6)*2 + 2", 8.0);
    assert_number(&count_book, "SUBTOTAL(3,B2:B6)", 3.0);

    let nested = evaluation_book(&[("B2", number_value(1.0)), ("B3", number_value(1.0))]);
    assert_number(&nested, "SUBTOTAL(9,B2)", 1.0);
    assert_number(&nested, "SUBTOTAL(9,B2:B3)", 2.0);
    assert_error(&nested, "SUBTOTAL(0,B2:B3)", FormulaErrorValue::Value);
    assert_error(&nested, "SUBTOTAL()", FormulaErrorValue::Value);
}

#[test]
fn evaluates_apache_poi_statistical_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestAverage.java,
    // TestAverageA.java, TestStdev.java, TestVar.java, TestForecast.java,
    // TestCorrel.java, TestCovar.java, TestGeomean.java, TestSlope.java,
    // TestIntercept.java, TestNormDist.java, TestNormInv.java,
    // TestNormSDist.java, TestNormSInv.java, TestPoisson.java, and
    // TestPoissonDist.java.
    let book = evaluation_book(&[]);
    assert_number(&book, "AVERAGE(1,2,3,4)", 2.5);
    assert_number(&book, "AVERAGE(1,2,TRUE,FALSE)", 1.0);
    assert_error(
        &book,
        "AVERAGE(1,#NAME?,3,#DIV/0!)",
        FormulaErrorValue::Name,
    );
    assert_number_with_epsilon(&book, "GEOMEAN(2,3)", 2.449489742783178, 1e-15);
    assert_number(&book, "GEOMEAN(TRUE)", 1.0);
    assert_number(&book, "GEOMEAN(\"2\")", 2.0);
    assert_error(&book, "GEOMEAN(\"foo\")", FormulaErrorValue::Value);
    assert_error(&book, "GEOMEAN(1,)", FormulaErrorValue::Num);
    assert_error(&book, "GEOMEAN(1,0)", FormulaErrorValue::Num);
    assert_error(&book, "GEOMEAN(1,-1)", FormulaErrorValue::Num);
    assert_error(&book, "GEOMEAN(#DIV/0!,#NUM!)", FormulaErrorValue::Div0);

    let average = evaluation_book(&[
        ("A1", number_value(1.0)),
        ("A2", number_value(2.0)),
        ("A3", FormulaValue::Blank),
        ("A4", number_value(3.0)),
        ("A5", FormulaValue::Blank),
        ("A6", number_value(4.0)),
        ("A7", FormulaValue::Blank),
    ]);
    assert_number(&average, "AVERAGE(A1:A7)", 2.5);

    let average_a_text = evaluation_book(&[
        ("A1", string("Data")),
        ("A2", number_value(10.0)),
        ("A3", number_value(7.0)),
        ("A4", number_value(9.0)),
        ("A5", number_value(2.0)),
        ("A6", string("Not available")),
        ("A7", string("Formula")),
    ]);
    assert_number_with_epsilon(&average_a_text, "AVERAGEA(A2:A6)", 5.6, 1e-11);
    assert_number_with_epsilon(&average_a_text, "AVERAGEA(A2:A5,A7)", 5.6, 1e-11);
    assert_number(&average_a_text, "AVERAGE(A2:A6)", 7.0);

    let average_a_booleans = evaluation_book(&[
        ("A2", number_value(10.0)),
        ("A3", number_value(7.0)),
        ("A4", number_value(9.0)),
        ("A5", number_value(2.0)),
        ("A6", FormulaValue::Boolean(true)),
        ("A7", FormulaValue::Boolean(false)),
    ]);
    assert_number_with_epsilon(
        &average_a_booleans,
        "AVERAGEA(A2:A7)",
        4.833333333333333,
        1e-11,
    );
    assert_number(&average_a_booleans, "AVERAGE(A2:A7)", 7.0);

    let average_a_numeric_strings = evaluation_book(&[
        ("A2", number_value(10.0)),
        ("A3", number_value(7.0)),
        ("A4", number_value(9.0)),
        ("A5", number_value(2.0)),
        ("A6", string("4.5")),
        ("A7", string("14")),
    ]);
    assert_number_with_epsilon(
        &average_a_numeric_strings,
        "AVERAGEA(A2:A7)",
        4.666666666666667,
        1e-11,
    );
    assert_number(&average_a_numeric_strings, "AVERAGE(A2:A7)", 7.0);

    let strength = evaluation_book(&[
        ("A3", number_value(1345.0)),
        ("A4", number_value(1301.0)),
        ("A5", number_value(1368.0)),
        ("A6", number_value(1322.0)),
        ("A7", number_value(1310.0)),
        ("A8", number_value(1370.0)),
        ("A9", number_value(1318.0)),
        ("A10", number_value(1350.0)),
        ("A11", number_value(1303.0)),
        ("A12", number_value(1299.0)),
    ]);
    assert_number_with_epsilon(&strength, "STDEVP(A3:A12)", 26.0545581424825, 1e-11);
    assert_number_with_epsilon(&strength, "STDEV.P(A3:A12)", 26.0545581424825, 1e-11);
    assert_number_with_epsilon(&strength, "STDEVPA(A3:A12)", 26.0545581424825, 1e-11);
    assert_number_with_epsilon(&strength, "STDEV(A3:A12)", 27.4639157198435, 1e-11);
    assert_number_with_epsilon(&strength, "STDEV.S(A3:A12)", 27.4639157198435, 1e-11);
    assert_number_with_epsilon(&strength, "STDEVA(A3:A12)", 27.4639157198435, 1e-11);
    assert_number_with_epsilon(&strength, "VARP(A3:A12)", 678.84, 1e-11);
    assert_number_with_epsilon(&strength, "VAR.P(A3:A12)", 678.84, 1e-11);
    assert_number_with_epsilon(&strength, "VARPA(A3:A12)", 678.84, 1e-11);
    assert_number_with_epsilon(&strength, "VAR(A3:A12)", 754.26667, 0.00005);
    assert_number_with_epsilon(&strength, "VAR.S(A3:A12)", 754.26667, 0.00005);
    assert_number_with_epsilon(&strength, "VARA(A3:A12)", 754.26667, 0.00005);

    let stats_booleans = evaluation_book(&[
        ("A2", number_value(10.0)),
        ("A3", number_value(7.0)),
        ("A4", number_value(9.0)),
        ("A5", number_value(2.0)),
        ("A6", FormulaValue::Boolean(true)),
        ("A7", FormulaValue::Boolean(false)),
    ]);
    assert_number_with_epsilon(&stats_booleans, "STDEVP(A2:A7)", 3.082207001484488, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "STDEV.P(A2:A7)", 3.082207001484488, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "STDEVPA(A2:A7)", 3.975620147292188, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "STDEV(A2:A7)", 3.559026084010437, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "STDEV.S(A2:A7)", 3.559026084010437, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "STDEVA(A2:A7)", 4.355073669487885, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "VARP(A2:A7)", 9.5, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "VAR.P(A2:A7)", 9.5, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "VARPA(A2:A7)", 15.805555555555557, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "VAR(A2:A7)", 12.666666666666666, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "VAR.S(A2:A7)", 12.666666666666666, 1e-11);
    assert_number_with_epsilon(&stats_booleans, "VARA(A2:A7)", 18.96666666666667, 1e-11);

    let stats_numeric_strings = evaluation_book(&[
        ("A2", number_value(10.0)),
        ("A3", number_value(7.0)),
        ("A4", number_value(9.0)),
        ("A5", number_value(2.0)),
        ("A6", string("4.5")),
        ("A7", string("14")),
    ]);
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "STDEVA(A2:A7)",
        4.546060565661952,
        1e-11,
    );
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "STDEV(A2:A7)",
        3.559026084010437,
        1e-11,
    );
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "STDEVPA(A2:A7)",
        4.149966532662911,
        1e-11,
    );
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "STDEVP(A2:A7)",
        3.082207001484488,
        1e-11,
    );
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "VARA(A2:A7)",
        20.666666666666668,
        1e-11,
    );
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "VAR(A2:A7)",
        12.666666666666666,
        1e-11,
    );
    assert_number_with_epsilon(
        &stats_numeric_strings,
        "VARPA(A2:A7)",
        17.222222222222225,
        1e-11,
    );
    assert_number_with_epsilon(&stats_numeric_strings, "VARP(A2:A7)", 9.5, 1e-11);

    let regression = evaluation_book(&[
        ("A2", number_value(6.0)),
        ("B2", number_value(20.0)),
        ("A3", number_value(7.0)),
        ("B3", number_value(28.0)),
        ("A4", number_value(9.0)),
        ("B4", number_value(31.0)),
        ("A5", number_value(15.0)),
        ("B5", number_value(38.0)),
        ("A6", number_value(21.0)),
        ("B6", number_value(40.0)),
    ]);
    assert_number_with_epsilon(&regression, "FORECAST(30,A2:A6,B2:B6)", 10.607253, 1e-7);
    assert_number_with_epsilon(
        &regression,
        "FORECAST.LINEAR(30,A2:A6,B2:B6)",
        10.607253,
        1e-7,
    );

    let correlation = evaluation_book(&[
        ("A2", number_value(3.0)),
        ("B2", number_value(9.0)),
        ("A3", number_value(2.0)),
        ("B3", number_value(7.0)),
        ("A4", number_value(4.0)),
        ("B4", number_value(12.0)),
        ("A5", number_value(5.0)),
        ("B5", number_value(15.0)),
        ("A6", number_value(6.0)),
        ("B6", number_value(17.0)),
    ]);
    assert_number_with_epsilon(&correlation, "CORREL(A2:A6,B2:B6)", 0.997054486, 5e-10);
    assert_number_with_epsilon(&correlation, "COVAR(A2:A6,B2:B6)", 5.2, 5e-10);
    assert_number_with_epsilon(&correlation, "COVARIANCE.P(A2:A6,B2:B6)", 5.2, 5e-10);
    assert_number_with_epsilon(&correlation, "COVARIANCE.S(A2:A6,B2:B6)", 6.5, 5e-10);
    assert_error(&correlation, "CORREL(A2:A6,B2:B5)", FormulaErrorValue::NA);
    assert_error(&correlation, "COVAR(A2:A6,B2:B5)", FormulaErrorValue::NA);

    let pearson = evaluation_book(&[
        ("A2", number_value(9.0)),
        ("B2", number_value(10.0)),
        ("A3", number_value(7.0)),
        ("B3", number_value(6.0)),
        ("A4", number_value(5.0)),
        ("B4", number_value(1.0)),
        ("A5", number_value(3.0)),
        ("B5", number_value(5.0)),
        ("A6", number_value(1.0)),
        ("B6", number_value(3.0)),
    ]);
    assert_number_with_epsilon(&pearson, "CORREL(A2:A6,B2:B6)", 0.699379, 5e-7);
    assert_number_with_epsilon(&pearson, "PEARSON(A2:A6,B2:B6)", 0.699379, 5e-7);

    let slope_intercept = evaluation_book(&[
        ("A1", number_value(1.0)),
        ("A2", number_value(2.0)),
        ("A3", number_value(3.0)),
        ("A4", number_value(4.0)),
        ("A5", number_value(5.0)),
        ("A6", number_value(6.0)),
        ("B1", number_value(31622779.60168379)),
        ("B2", number_value(31622780.60168379)),
        ("B3", number_value(31622778.60168379)),
        ("B4", number_value(31622781.60168379)),
        ("B5", number_value(31622780.60168379)),
        ("B6", number_value(31622783.60168379)),
    ]);
    assert_number_with_epsilon(
        &slope_intercept,
        "SLOPE(A1:A6,B1:B6)",
        0.7752808988764045,
        1e-12,
    );
    assert_number_with_epsilon(
        &slope_intercept,
        "INTERCEPT(A1:A6,B1:B6)",
        -24516534.39905822,
        0.0000001,
    );
    assert_error(
        &slope_intercept,
        "SLOPE(A1:A2,B1:B3)",
        FormulaErrorValue::NA,
    );
    assert_error(
        &slope_intercept,
        "INTERCEPT(A1:A2,B1:B3)",
        FormulaErrorValue::NA,
    );

    assert_number_with_epsilon(&book, "NORMDIST(42,40,1.5,TRUE)", 0.908788780274132, 1e-14);
    assert_number_with_epsilon(&book, "NORM.DIST(42,40,1.5,TRUE)", 0.908788780274132, 1e-14);
    assert_number_with_epsilon(&book, "NORMDIST(42,40,1.5,FALSE)", 0.109340049783996, 1e-14);
    assert_number_with_epsilon(&book, "NORMINV(0.908789,40,1.5)", 42.000002, 0.000001);
    assert_number_with_epsilon(&book, "NORM.INV(0.908789,40,1.5)", 42.000002, 0.000001);
    assert_number_with_epsilon(&book, "NORMSDIST(1.333333)", 0.908788726, 0.000001);
    assert_number_with_epsilon(&book, "NORM.S.DIST(1.333333)", 0.908788726, 0.000001);
    assert_number_with_epsilon(&book, "NORMSINV(0.9088)", 1.3334, 0.00001);
    assert_number_with_epsilon(&book, "NORM.S.INV(0.9088)", 1.3334, 0.00001);
    assert_error(
        &book,
        "NORMDIST(\"A1\",\"B2\",\"C2\",FALSE)",
        FormulaErrorValue::Value,
    );
    assert_error(&book, "NORMDIST(42,40,0,FALSE)", FormulaErrorValue::Num);
    assert_error(
        &book,
        "NORMINV(\"A1\",\"B2\",\"C2\")",
        FormulaErrorValue::Value,
    );
    assert_error(&book, "NORMINV(0.5,40,0)", FormulaErrorValue::Num);
    assert_error(&book, "NORMSDIST(\"A1\")", FormulaErrorValue::Value);
    assert_error(&book, "NORMSINV(\"A1\")", FormulaErrorValue::Value);
    assert_error(&book, "NORMSINV(0)", FormulaErrorValue::Num);
    assert_error(&book, "NORMSINV(1)", FormulaErrorValue::Num);

    assert_number_with_epsilon(&book, "POISSON(1,0.2,TRUE)", 0.9824769036935787, 1e-15);
    assert_number_with_epsilon(&book, "POISSON(0,0.2,FALSE)", 0.8187307530779818, 1e-15);
    assert_number_with_epsilon(&book, "POISSON(1.1,0.2,TRUE)", 0.9824769036935787, 1e-15);
    assert_number(&book, "POISSON(0,0,TRUE)", 1.0);
    assert_number_with_epsilon(&book, "POISSON.DIST(2,5,TRUE)", 0.12465201948308113, 1e-14);
    assert_number_with_epsilon(&book, "POISSON.DIST(2,5,FALSE)", 0.08422433748856833, 1e-14);
    assert_number_with_epsilon(&book, "POISSON(2,5,TRUE)", 0.12465201948308113, 1e-14);
    assert_number_with_epsilon(&book, "POISSON(2,5,FALSE)", 0.08422433748856833, 1e-14);
    assert_number_with_epsilon(&book, "POISSON(2.9,5,FALSE)", 0.08422433748856833, 1e-14);
    assert_error(&book, "POISSON.DIST(2,5)", FormulaErrorValue::Value);
    assert_error(
        &book,
        "POISSON.DIST(\"abc\",5,TRUE)",
        FormulaErrorValue::Value,
    );
    assert_error(
        &book,
        "POISSON.DIST(2,\"A3\",TRUE)",
        FormulaErrorValue::Value,
    );
    assert_error(&book, "POISSON.DIST(-1,5,TRUE)", FormulaErrorValue::Num);
    assert_error(&book, "POISSON.DIST(2,-5,TRUE)", FormulaErrorValue::Num);
}

#[test]
fn evaluates_apache_poi_financial_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestNpv.java,
    // TestPmt.java, TestRate.java, TestIrr.java, TestMirr.java, and
    // TestNper.java. Spreadsheet-backed .xls checks are used only as source
    // evidence until the test-suite has BIFF fixture execution.
    let npv = evaluation_book(&[
        ("A2", number_value(0.08)),
        ("A3", number_value(-40000.0)),
        ("A4", number_value(8000.0)),
        ("A5", number_value(9200.0)),
        ("A6", number_value(10000.0)),
        ("A7", number_value(12000.0)),
        ("A8", number_value(14500.0)),
    ]);
    assert_number_with_epsilon(&npv, "NPV(A2,A4,A5,A6,A7,A8)+A3", 1922.06, 0.01);
    assert_number_with_epsilon(&npv, "NPV(A2,A4:A8)+A3", 1922.06, 0.01);

    let book = evaluation_book(&[]);
    assert_number_with_epsilon(&book, "PMT(0.08/12,10,10000,0,0)", -1037.0321, 0.00005);
    assert_number_with_epsilon(&book, "PMT(0.08/12,10,10000,0,1)", -1030.1643, 0.00005);
    assert_number_with_epsilon(&book, "PMT(0.005,24,1000)", -44.3206, 0.00005);
    assert_number_with_epsilon(&book, "PV(0.08/12,20*12,500,,0)", -59777.14585, 0.0001);
    assert_number_with_epsilon(&book, "PV(0.08/12,20*12,500,,)", -59777.14585, 0.0001);
    assert_number_with_epsilon(&book, "PV(0.08/12,20*12,500,500,)", -59878.6315455, 0.0001);
    assert_number_with_epsilon(&book, "FV(0.08/12,20*12,500,,)", -294510.207810727, 0.0001);
    assert_number_with_epsilon(&book, "PMT(0.08/12,20*12,500,,)", -4.182200345, 0.0001);
    assert_number_with_epsilon(&book, "NPER(0.08/12,20*12,500,,)", -2.0758873434, 0.0001);

    let rate = evaluation_book(&[
        ("A2", number_value(4.0)),
        ("A3", number_value(-200.0)),
        ("A4", number_value(8000.0)),
    ]);
    assert_number_with_epsilon(&rate, "RATE(A2*12,A3,A4)", 0.007701472, 0.000001);
    assert_number_with_epsilon(&rate, "RATE(A2*12,A3,A4)*12", 0.09241767, 0.000001);
    assert_number_with_epsilon(&book, "RATE(3,-10,900,1,0,0.5)", -0.7634, 0.0001);
    assert_number_with_epsilon(&book, "RATE(3,-10,900)", -0.7563, 0.0001);
    assert_number_with_epsilon(&book, "RATE(2,0,-593.06,214.07,0,0.1)", -0.39920185, 1e-8);
    assert_number_with_epsilon(&book, "RATE(2,0,-4725.38,4509.97,0,0.1)", -0.02305873, 1e-8);
    assert_number_with_epsilon(
        &book,
        "RATE(10,0,-3500,10000,0,0.1)",
        0.11069085371426893,
        1e-6,
    );
    assert_number_with_epsilon(&book, "RATE(360,6.56,-2000)", 0.000948017084406, 0.000001);
    assert_error(
        &book,
        "RATE(12,400,10000,5000,0,0.1)",
        FormulaErrorValue::Num,
    );
    assert_error(
        &book,
        "RATE(2,0,-13.65,-329.67,0,0.1)",
        FormulaErrorValue::Num,
    );

    let irr = evaluation_book(&[
        ("A1", number_value(-4000.0)),
        ("B1", number_value(1200.0)),
        ("C1", number_value(1410.0)),
        ("D1", number_value(1875.0)),
        ("E1", number_value(1050.0)),
        ("A2", number_value(-70000.0)),
        ("A3", number_value(12000.0)),
        ("A4", number_value(15000.0)),
        ("A5", number_value(18000.0)),
        ("A6", number_value(21000.0)),
        ("A7", number_value(26000.0)),
    ]);
    assert_number_with_epsilon(&irr, "IRR(A1:E1)", 0.143, 0.0005);
    assert_number_with_epsilon(&irr, "IRR(A2:A6)", -0.02124484827341093, 1e-4);
    assert_number_with_epsilon(&irr, "IRR(A2:A7)", 0.08663094803653162, 1e-4);
    assert_number_with_epsilon(&irr, "IRR(A2:A4,-0.1)", -0.44350694133474067, 1e-4);

    let mirr = evaluation_book(&[
        ("A2", number_value(-120000.0)),
        ("A3", number_value(39000.0)),
        ("A4", number_value(30000.0)),
        ("A5", number_value(21000.0)),
        ("A6", number_value(37000.0)),
        ("A7", number_value(46000.0)),
        ("A8", number_value(0.1)),
        ("A9", number_value(0.12)),
    ]);
    assert_number_with_epsilon(&mirr, "MIRR(A2:A7,A8,A9)", 0.126094, 0.00000015);
    assert_number_with_epsilon(&mirr, "MIRR(A2:A5,A8,A9)", -0.048044655, 0.00000015);
    assert_number_with_epsilon(&mirr, "MIRR(A2:A7,A8,.14)", 0.134759111, 0.00000015);
    assert_error(&mirr, "MIRR(A3:A7,0.08,0.05)", FormulaErrorValue::Div0);

    assert_number_with_epsilon(&book, "NPER(0.05,250,-1000)", 4.57353557, 0.00000001);
    assert_error(&book, "NPER(12,4500,100000,100000)", FormulaErrorValue::Num);
}

#[test]
fn evaluates_apache_poi_math_and_aggregate_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestRoundFuncs.java,
    // TestQuotient.java, TestProduct.java, and TestSum.java.
    let book = evaluation_book(&[]);
    assert_number_with_epsilon(&book, "ROUNDUP(3987*0.2,2)", 797.40, 1e-10);
    assert_number_with_epsilon(&book, "ROUNDDOWN(3987*0.2,2)", 797.40, 1e-10);
    assert_number_with_epsilon(&book, "ROUND(3987*0.2,2)", 797.40, 1e-10);
    assert_number_with_epsilon(&book, "ROUND(2.05,1)", 2.1, 1e-25);
    assert_error(&book, "ROUNDDOWN(\"abc\",2)", FormulaErrorValue::Value);
    assert_error(&book, "ROUNDUP(\"abc\",2)", FormulaErrorValue::Value);

    assert_number(&book, "QUOTIENT(5,2)", 2.0);
    assert_number(&book, "QUOTIENT(4.5,3.1)", 1.0);
    assert_number(&book, "QUOTIENT(-10,3)", -3.0);
    assert_number(&book, "QUOTIENT(-5.5,2)", -2.0);
    assert_number(&book, "QUOTIENT(3.14159,6.02214179E+23)", 0.0);
    assert_error(&book, "QUOTIENT(\"ABCD\",\"\")", FormulaErrorValue::Value);
    assert_error(&book, "QUOTIENT(\"\",\"ABCD\")", FormulaErrorValue::Value);
    assert_error(&book, "QUOTIENT(3.14159,0)", FormulaErrorValue::Div0);

    assert_number(&book, "PRODUCT()", 0.0);
    assert_number(&book, "PRODUCT(,)", 0.0);
    assert_number(&book, "PRODUCT(2,)", 2.0);
    assert_number(&book, "PRODUCT(2,,\"6\",TRUE)", 12.0);
    assert_number(&book, "PRODUCT(TRUE,TRUE)", 1.0);

    let ranges = evaluation_book(&[
        ("A1", FormulaValue::Boolean(true)),
        ("B1", FormulaValue::Boolean(true)),
        ("B2", number_value(7000.0)),
        ("B3", number_value(14000.0)),
        ("B4", number_value(21000.0)),
        ("B5", number_value(28000.0)),
        ("B6", FormulaValue::Error(FormulaErrorValue::NA)),
        ("B7", string("abc")),
    ]);
    assert_number(&ranges, "PRODUCT(A1:B1)", 0.0);
    assert_number(&ranges, "PRODUCT(A1,B1)", 0.0);
    assert_number(&ranges, "SUM(B2:B5)", 70000.0);
    assert_number(&ranges, "SUM(B2:B5,B7)", 70000.0);
    assert_error(&ranges, "SUM(B2:B6)", FormulaErrorValue::NA);
}

#[test]
fn evaluates_apache_poi_rounding_math_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestAbs.java,
    // TestTrunc.java, TestFloor.java, TestCeiling.java, TestFloorPrecise.java,
    // TestCeilingPrecise.java, TestFloorMath.java, and TestCeilingMath.java.
    let book = evaluation_book(&[]);
    assert_number(&book, "ABS(-4)", 4.0);
    assert_number(&book, "ABS(-4.123)", 4.123);

    let abs_range = evaluation_book(&[("A1", number_value(1.0)), ("B2", number_value(-2.0))]);
    assert_number(&abs_range, "ABS(A1:A2)", 1.0);

    assert_error(&book, "TRUNC(\"abc\",2)", FormulaErrorValue::Value);
    assert_number(&book, "TRUNC(200,2)", 200.0);
    assert_number(&book, "TRUNC(2.612777,3)", 2.612);
    assert_number(&book, "TRUNC(0.29,2)", 0.29);
    assert_number(&book, "TRUNC(21.624/24+.009,2)", 0.91);
    assert_number(&book, "TRUNC(2.612777)", 2.0);
    assert_number(&book, "TRUNC(-8.9,0)", -8.0);

    for (formula, expected) in [
        ("FLOOR(3.7,2)", 2.0),
        ("FLOOR(-2.5,-2)", -2.0),
        ("FLOOR(1.58,0.1)", 1.5),
        ("FLOOR(0.234,0.01)", 0.23),
        ("CEILING(2.5,1)", 3.0),
        ("CEILING(-2.5,-2)", -4.0),
        ("CEILING(-2.5,2)", -2.0),
        ("CEILING(1.5,0.1)", 1.5),
        ("CEILING(0.234,0.01)", 0.24),
        ("FLOOR.PRECISE(-3.2,-1)", -4.0),
        ("FLOOR.PRECISE(3.2,1)", 3.0),
        ("FLOOR.PRECISE(-3.2,1)", -4.0),
        ("FLOOR.PRECISE(3.2,-1)", 3.0),
        ("FLOOR.PRECISE(3.2)", 3.0),
        ("CEILING.PRECISE(4.3)", 5.0),
        ("CEILING.PRECISE(-4.3)", -4.0),
        ("CEILING.PRECISE(4.3,2)", 6.0),
        ("CEILING.PRECISE(4.3,-2)", 6.0),
        ("CEILING.PRECISE(-4.3,2)", -4.0),
        ("CEILING.PRECISE(-4.3,-2)", -4.0),
        ("FLOOR.MATH(24.3,5)", 20.0),
        ("FLOOR.MATH(6.7)", 6.0),
        ("FLOOR.MATH(-8.1,2)", -10.0),
        ("FLOOR.MATH(-5.5,2,-1)", -4.0),
        ("FLOOR.MATH(-2.5,-2)", -4.0),
        ("FLOOR.MATH(-2.5,-2,-1)", -2.0),
        ("FLOOR.MATH(2.5,-2)", 2.0),
        ("FLOOR.MATH(0.234,0.01)", 0.23),
        ("CEILING.MATH(24.3,5)", 25.0),
        ("CEILING.MATH(6.7)", 7.0),
        ("CEILING.MATH(-8.1,2)", -8.0),
        ("CEILING.MATH(-5.5,2,-1)", -6.0),
        ("CEILING.MATH(2.5,-2)", 4.0),
        ("CEILING.MATH(-2.5,-2)", -2.0),
        ("CEILING.MATH(-2.5,-2,-1)", -4.0),
        ("CEILING.MATH(0.234,0.01)", 0.24),
    ] {
        assert_number_with_epsilon(&book, formula, expected, 0.00000000000001);
    }

    assert_error(&book, "FLOOR(2.5,-2)", FormulaErrorValue::Num);
    for formula in [
        "FLOOR()",
        "CEILING()",
        "FLOOR.PRECISE()",
        "CEILING.PRECISE()",
        "FLOOR.MATH()",
        "CEILING.MATH()",
        "FLOOR(\"abc\",\"def\")",
        "CEILING(\"abc\",\"def\")",
        "FLOOR.PRECISE(\"abc\")",
        "CEILING.PRECISE(\"abc\")",
        "FLOOR.MATH(\"abc\")",
        "CEILING.MATH(\"abc\")",
    ] {
        assert_error(&book, formula, FormulaErrorValue::Value);
    }
}

#[test]
fn evaluates_apache_poi_engineering_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestBin2Dec.java,
    // TestDec2Bin.java, TestHex2Dec.java, TestOct2Dec.java, TestComplex.java,
    // TestDelta.java, TestSqrtpi.java, and TestBesselJ.java.
    let book = evaluation_book(&[]);
    for (formula, expected) in [
        ("BIN2DEC(\"00101\")", 5.0),
        ("BIN2DEC(\"1111111111\")", -1.0),
        ("BIN2DEC(\"1111111110\")", -2.0),
        ("BIN2DEC(\"0111111111\")", 511.0),
        ("HEX2DEC(\"A5\")", 165.0),
        ("HEX2DEC(\"FFFFFFFF5B\")", -165.0),
        ("HEX2DEC(\"3DA408B9\")", 1034160313.0),
        ("OCT2DEC(\"\")", 0.0),
        ("OCT2DEC(\"54\")", 44.0),
        ("OCT2DEC(\"7777777533\")", -165.0),
        ("OCT2DEC(\"7000000000\")", -134217728.0),
        ("OCT2DEC(\"7776667533\")", -299173.0),
        ("DELTA(5,4)", 0.0),
        ("DELTA(5,5)", 1.0),
        ("DELTA(0.5,0)", 0.0),
        ("DELTA(0.50,0.5)", 1.0),
        ("DELTA(0.5000000000,0.5)", 1.0),
        ("SQRTPI(1)", 1.77245385090552),
        ("SQRTPI(2)", 2.506628274631),
        ("BESSELJ(1.9,2)", 0.329925829),
        ("BESSELJ(1.9,2.5)", 0.329925829),
        ("BESSELJ(12.4,7)", -0.217156767),
    ] {
        assert_number_with_epsilon(&book, formula, expected, 0.000001);
    }

    for (formula, expected) in [
        ("DEC2BIN(5)", "101"),
        ("DEC2BIN(-1)", "1111111111"),
        ("DEC2BIN(-2)", "1111111110"),
        ("DEC2BIN(511)", "111111111"),
        ("DEC2BIN(-512)", "1000000000"),
        ("DEC2BIN(13.43)", "1101"),
        ("DEC2BIN(13.43,8)", "1101"),
        ("COMPLEX(3,4)", "3+4i"),
        ("COMPLEX(3,4,\"j\")", "3+4j"),
        ("COMPLEX(0,1)", "i"),
        ("COMPLEX(1,0)", "1"),
        ("COMPLEX(2,3)", "2+3i"),
        ("COMPLEX(-2,-3)", "-2-3i"),
        ("COMPLEX(-0.5,-3.2)", "-0.5-3.2i"),
    ] {
        assert_text(&book, formula, expected);
    }

    for formula in [
        "BIN2DEC(\"01010101010\")",
        "BIN2DEC(\"GGGGGGG\")",
        "BIN2DEC(\"3.14159\")",
        "DEC2BIN(512)",
        "DEC2BIN(-513)",
        "DEC2BIN(13.43,1)",
        "DEC2BIN(13.43,-8)",
        "DEC2BIN(13.43,0)",
        "HEX2DEC(\"GGGGGGG\")",
        "HEX2DEC(\"3.14159\")",
        "OCT2DEC(\"ABCDEFGH\")",
        "OCT2DEC(\"99999999\")",
        "OCT2DEC(\"3.14159\")",
        "SQRTPI(-1)",
        "BESSELJ(22.5,-40)",
    ] {
        assert_error(&book, formula, FormulaErrorValue::Num);
    }

    for formula in [
        "BIN2DEC(0,0)",
        "DEC2BIN(\"GGGGGGG\")",
        "DEC2BIN(\"3.14159a\")",
        "DEC2BIN(13.43,8,8)",
        "COMPLEX(\"ABCD\",,)",
        "COMPLEX(1,\"ABCD\",)",
        "COMPLEX(1,1,\"k\")",
        "COMPLEX(1,1,\"I\")",
        "COMPLEX(1,1,\"J\")",
        "DELTA(\"A1\",\"B2\")",
        "DELTA(\"AAAA\",\"BBBB\")",
        "SQRTPI()",
        "SQRTPI(\"num\")",
        "SQRTPI(3,\"num\")",
        "BESSELJ(\"A1\",\"B2\")",
    ] {
        assert_error(&book, formula, FormulaErrorValue::Value);
    }
}

#[test]
fn evaluates_apache_poi_error_and_boolean_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestErrors.java,
    // TestIsBlank.java, and TestOrFunction.java.
    let text_divide = evaluation_book(&[("A1", string("text"))]);
    assert_error(&text_divide, "A1/2", FormulaErrorValue::Value);

    let main = SheetId(1);
    let other = SheetId(2);
    let blank_refs = FormulaEvaluationBook {
        sheet_names: vec![
            SheetBinding {
                id: main,
                name: Cow::Borrowed("Sheet1"),
            },
            SheetBinding {
                id: other,
                name: Cow::Borrowed("Sheet2"),
            },
        ],
        ..FormulaEvaluationBook::default()
    };
    assert_eq!(
        blank_refs.evaluate_formula_text_with_grammar(
            main,
            None,
            "ISBLANK(Sheet2!A1:A1)",
            FormulaGrammar::ExcelA1
        ),
        Some(FormulaValue::Boolean(true))
    );
    assert_boolean(&evaluation_book(&[]), "ISBLANK(D7:D7)", true);

    let book = evaluation_book(&[]);
    assert_boolean(&book, "OR(TRUE,TRUE)", true);
    assert_boolean(&book, "OR(TRUE,FALSE)", true);
    assert_boolean(&book, "OR(1=1,2=2,3=3)", true);
    assert_boolean(&book, "OR(1=2,2=3,3=4)", false);
    assert_number(&book, "INDEX({1},1,IF(OR(FALSE,FALSE),1,1))", 1.0);
    assert_number(&book, "INDEX({1},1,IF(OR(FALSE,FALSE),0,1))", 1.0);
    assert_number(&book, "INDEX({1},1,IF(OR(1=2,2=3,3=4),0,1))", 1.0);

    let example1 = evaluation_book(&[("A2", number_value(50.0)), ("A3", number_value(100.0))]);
    assert_boolean(&example1, "OR(A2>1,A2<100)", true);
    assert_number(
        &example1,
        "IF(OR(A2>1,A2<100),A3,\"The value is out of range\")",
        100.0,
    );
    assert_text(
        &example1,
        "IF(OR(A2<0,A2>50),A2,\"The value is out of range\")",
        "The value is out of range",
    );

    let example2 = evaluation_book(&[
        ("B4", number_value(8500.0)),
        ("B5", number_value(5.0)),
        ("B6", number_value(0.02)),
        ("B14", number_value(15700.0)),
        ("C14", number_value(7.0)),
    ]);
    assert_number(&example2, "IF(OR(B14>=$B$4,C14>=$B$5),B14*$B$6,0)", 314.0);
}

#[test]
fn evaluates_apache_poi_date_and_time_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestDate.java,
    // TestDateValue.java, TestDays.java, TestTime.java, TestTimeValue.java,
    // TestWeekdayFunc.java, TestEDate.java, and TestEOMonth.java.
    let book = evaluation_book(&[]);
    for (formula, expected) in [
        ("DATE(1900, 1, 1)", 1.0),
        ("DATE(1900, 1, 32)", 32.0),
        ("DATE(1900, 222, 1)", 6727.0),
        ("DATE(1900, 2, 0)", 31.0),
        ("DATE(2000, 1, 222)", 36747.0),
        ("DATE(2007, 1, 1)", 39083.0),
        ("DATE(1900, 2, 29)", 60.0),
        ("DATE(1900, 2, 30)", 61.0),
        ("DATE(1900, 1, 222)", 222.0),
        ("DATE(1900, 1, 2222)", 2222.0),
        ("DATE(1900, 1, 22222)", 22222.0),
        ("DATE(4, 1, 1)", 1462.0),
        ("DATE(14, 1, 1)", 5115.0),
        ("DATE(104, 1, 1)", 37987.0),
        ("DATE(1004, 1, 1)", 366705.0),
        ("DATEVALUE(\"2020-02-01\")", 43862.0),
        ("DATEVALUE(\"01-02-2020\")", 43862.0),
        ("DATEVALUE(\"2020-FEB-01\")", 43862.0),
        ("DATEVALUE(\"2020-Feb-01\")", 43862.0),
        ("DATEVALUE(\"2020-FEBRUARY-01\")", 43862.0),
        ("DATEVALUE(\"2/1/2020\")", 43862.0),
        ("DATEVALUE(\"2020/2/1\")", 43862.0),
        ("DATEVALUE(\"2020/FEB/1\")", 43862.0),
        ("DATEVALUE(\"FEB/1/2020\")", 43862.0),
        ("DATEVALUE(\"2020/02/01\")", 43862.0),
        ("DATEVALUE(\"8/22/2011\")", 40777.0),
        ("DATEVALUE(\"22-MAY-2011\")", 40685.0),
        ("DATEVALUE(\"2011/02/23\")", 40597.0),
        ("DATEVALUE(\"8/22/2011 12:00\")", 40777.0),
        ("DATEVALUE(\"8/22/2011 6:02:23 PM\")", 40777.0),
        ("DATEVALUE(\"22-AUG-2011 6:02:23PM\")", 40777.0),
        ("DATEVALUE(\"22-AUG-2011 6:02:23AM\")", 40777.0),
        ("DATEVALUE(\"1954-07-20\")", 19925.0),
        ("DAYS(\"15-MAR-2021\",\"1-FEB-2021\")", 42.0),
        ("DAYS(\"1-FEB-2021\", \"15-MAR-2021\")", -42.0),
        ("TIME(0,0,1)", 1.0 / 86400.0),
        ("TIME(0,1,0)", 60.0 / 86400.0),
        ("TIME(0,0,0)", 0.0),
        ("TIME(1,0,0)", 3600.0 / 86400.0),
        ("TIME(12,0,0)", 0.5),
        ("TIME(23,0,0)", 23.0 / 24.0),
        ("TIME(24,0,0)", 0.0),
        ("TIME(25,0,0)", 1.0 / 24.0),
        ("TIME(48,0,0)", 0.0),
        ("TIME(6,30,0)", 23400.0 / 86400.0),
        ("TIME(6,60,0)", 7.0 / 24.0),
        ("TIME(18,49,60)", 67800.0 / 86400.0),
        ("TIME(18,49,32767)", 14107.0 / 86400.0),
        ("TIME(18,32767,61)", 43681.0 / 86400.0),
        ("TIME(32767,49,61)", 28201.0 / 86400.0),
        ("TIMEVALUE(\"8/22/2011\")", 0.0),
        ("TIMEVALUE(\"8/22/2011 12:00\")", 0.5),
        ("TIMEVALUE(\"1/01/2000 06:00\")", 0.25),
        ("TIMEVALUE(\"1/01/2000 6:00 PM\")", 0.75),
        ("TIMEVALUE(\"12:00\")", 0.5),
        ("TIMEVALUE(\"6:00 PM\")", 0.75),
        ("TIMEVALUE(\"12:03:45\")", 0.5026041666642413),
        ("TIMEVALUE(\"12:03:45.386\")", 0.5026041666642413),
        ("WEEKDAY(1)", 2.0),
        ("WEEKDAY(1,1)", 2.0),
        ("WEEKDAY(1,2)", 1.0),
        ("WEEKDAY(1,3)", 0.0),
        ("WEEKDAY(1,11)", 1.0),
        ("WEEKDAY(1,12)", 7.0),
        ("WEEKDAY(1,13)", 6.0),
        ("WEEKDAY(1,14)", 5.0),
        ("WEEKDAY(1,15)", 4.0),
        ("WEEKDAY(1,16)", 3.0),
        ("WEEKDAY(1,17)", 2.0),
        ("WEEKDAY(39448)", 3.0),
        ("WEEKDAY(39448,2)", 2.0),
        ("WEEKDAY(39448,3)", 1.0),
        ("WEEKDAY(DATE(2008,2,14))", 5.0),
        ("WEEKDAY(DATE(2008,2,14),2)", 4.0),
        ("WEEKDAY(DATE(2008,2,14),3)", 3.0),
        ("EDATE(1000,0)", 1000.0),
        ("EDATE(1,0)", 1.0),
        ("EDATE(0,1)", 31.0),
        ("EDATE(1,1)", 32.0),
        ("EDATE(0,0)", 0.0),
        ("EDATE(0,-2)", -1.0),
        ("EDATE(0,-3)", -1.0),
        ("EDATE(49104,0)", 49104.0),
        ("EDATE(49104,1)", 49134.0),
        ("EOMONTH(1,0)", 31.0),
        ("EOMONTH(1,1)", 59.0),
        ("EOMONTH(1000,0)", 1004.0),
        ("EOMONTH(49104,0)", 49125.0),
        ("EOMONTH(49104,1)", 49156.0),
        ("EOMONTH(0,-2)", -1.0),
        ("EOMONTH(0,-3)", -1.0),
        ("EOMONTH(31,-2)", -1.0),
        ("EOMONTH(0,0)", 31.0),
        ("EOMONTH(0,1)", 59.0),
    ] {
        assert_number_with_epsilon(&book, formula, expected, 0.0001);
    }

    let days_book =
        evaluation_book(&[("A2", number_value(44561.0)), ("A3", number_value(44197.0))]);
    assert_number(&days_book, "DAYS(A2,A3)", 364.0);

    for formula in [
        "DATEVALUE(\"\")",
        "DATEVALUE(\"non-date text\")",
        "DATEVALUE(\"2/32/2020\")",
        "DATEVALUE(\"32/2/2020\")",
        "DATEVALUE(\"32/32/2020\")",
        "DATEVALUE(FALSE)",
        "DATEVALUE(EXP(1))",
        "DAYS(\"15-XYZ\",\"1-FEB-2021\")",
        "DAYS(\"15-MAR-2021\",\"1-XYZ\")",
        "DAYS(\"15-MAR-2021\")",
        "TIMEVALUE(\"non-date text\")",
        "TIMEVALUE(FALSE)",
        "TIMEVALUE(EXP(1))",
        "WEEKDAY()",
        "WEEKDAY(1,1,1)",
        "WEEKDAY(-1)",
        "WEEKDAY(\"\")",
        "WEEKDAY(\"3\",\"\")",
        "EDATE(1000)",
        "EOMONTH(1000)",
        "EOMONTH(\"a\",\"b\")",
    ] {
        assert_error(&book, formula, FormulaErrorValue::Value);
    }

    assert_error(&book, "WEEKDAY(1,18)", FormulaErrorValue::Num);
}

#[test]
fn evaluates_apache_poi_text_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestClean.java,
    // TestCode.java, TestLen.java, TestLeftRight.java, TestMid.java,
    // TestSubstitute.java, TestTrim.java, TestFind.java, TestConcat.java,
    // and TestText.java.
    let book = evaluation_book(&[]);
    for (formula, expected) in [
        ("CLEAN(CHAR(7)&\"text\"&CHAR(7))", "text"),
        ("CLEAN(CHAR(7)&\"text\"&CHAR(17))", "text"),
        (
            "CLEAN(CHAR(181)&\"text\"&CHAR(190))",
            "\u{00B5}text\u{00BE}",
        ),
        ("CLEAN(\"text\"&CHAR(160)&\"'\")", "text\u{00A0}'"),
        (
            "CLEAN(\"\u{0011}aniket\u{0007}\u{0017}\u{007F}\")",
            "aniket\u{007F}",
        ),
        (
            "CLEAN(\"\u{2116}aniket\u{2211}\u{FB5E}\u{2039}\")",
            "\u{2116}aniket\u{2211}\u{FB5E}\u{2039}",
        ),
        ("CODE(\"A\")", "65"),
        ("CODE(\"ABCDEFGHI\")", "65"),
        ("CODE(\"!\")", "33"),
        ("MID(\"galactic\",3,4)", "lact"),
        ("MID(\"galactic\",3.1,4)", "lact"),
        ("MID(\"galactic\",\"3\",4)", "lact"),
        ("MID(123456,\"3.1\",\"2.9\")", "34"),
        ("MID(\"galactic\",3.1,)", ""),
        ("MID(\"galactic\",3,FALSE)", ""),
        ("MID(\"galactic\",3,TRUE)", "l"),
        ("MID(\"galactic\",4,400)", "actic"),
        ("MID(\"galactic\",30,4)", ""),
        ("MID(\"galactic\",3,0)", ""),
        ("SUBSTITUTE(\"ABC\",\"B\",\"DEF\")", "ADEFC"),
        ("SUBSTITUTE(\"ABC\",\"B\",\"CDE\")", "ACDEC"),
        ("SUBSTITUTE(\"ABCBA\",\"B\",\"CDE\")", "ACDECCDEA"),
        ("SUBSTITUTE(\"ABC\",\"B\",\"DEF\",1)", "ADEFC"),
        ("SUBSTITUTE(\"ABC\",\"B\",\"CDE\",1)", "ACDEC"),
        ("SUBSTITUTE(\"ABC\",\"B\",\"DEF\",12)", "ABC"),
        ("SUBSTITUTE(\"ABC\",\"B\",\"CDE\",2)", "ABC"),
        ("SUBSTITUTE(\"ABC\",\"\",\"CDE\")", "ABC"),
        ("SUBSTITUTE(\"ABC\",\"\",\"CDE\",1)", "ABC"),
        ("TRIM(\" hi \")", "hi"),
        ("TRIM(\"hi \")", "hi"),
        ("TRIM(\"  hi\")", "hi"),
        ("TRIM(\" hi there  \")", "hi there"),
        ("TRIM(\"\")", ""),
        ("TRIM(\"   \")", ""),
        ("TRIM(\" hi  there  \")", "hi there"),
        ("TRIM(\"hi   there\")", "hi there"),
        ("TRIM(123456)", "123456"),
        ("TRIM(FALSE)", "FALSE"),
        ("TRIM(TRUE)", "TRUE"),
        ("TEXT(\"abc\",\"abc\")", "abc"),
        ("TEXT(321321.321,\"#,###.00000\")", "321,321.32100"),
        ("TEXT(321.321,\"00000.00000\")", "00321.32100"),
        ("TEXT(321.321,\"$#.#\")", "$321.3"),
        ("TEXT(321.321,\"# #/#\")", "321 1/3"),
        ("TEXT(321.321,\"# #/##\")", "321 26/81"),
        ("TEXT(321.321,\"#/##\")", "26027/81"),
        (
            "TEXT(321.321,\"yyyy-mm-ddThh:MM:ss\")",
            "1900-11-16T07:42:14",
        ),
        (
            "TEXT(321.321,\"yyyy-mm-ddThh:MM:ss.000\")",
            "1900-11-16T07:42:14.400",
        ),
        ("TEXT(-123456.789012345,\"#0.000\")", "-123456.789"),
        ("TEXT(-123456.789012345,\"000000\")", "-123457"),
        ("TEXT(12.78,\"00000.000000\")", "00012.780000"),
        ("TEXT(0.56789012385,\"#0.0000000000\")", "0.5678901239"),
        ("TEXT(\"\",\"yyyymmmdd\")", ""),
        ("TEXT(\"anyText\",\"yyyymmmdd\")", "anyText"),
        ("TEXT(TRUE,\"yyyymmmdd\")", "TRUE"),
        ("TEXT(FALSE,\"#0.000\")", "FALSE"),
        ("TEXT(\"\",\"#0.000\")", ""),
        ("TEXT(\"anyText\",\"#0.000\")", "anyText"),
        ("TEXT(DATE(2022,2,28),\"MMM\")", "Feb"),
        ("TEXT(\"02/28/2022\",\"MMM\")", "Feb"),
        (
            "CONCAT(\"The\",\" \",\"sun\",\" \",\"will\",\" \",\"come\",\" \",\"up\",\" \",\"tomorrow.\")",
            "The sun will come up tomorrow.",
        ),
    ] {
        assert_text(&book, formula, expected);
    }

    let blank = evaluation_book(&[("A1", FormulaValue::Blank)]);
    assert_number(&blank, "LEN(A1)", 0.0);
    assert_text(&blank, "TRIM(A1)", "");
    assert_text(&blank, "MID(A1,3,TRUE)", "");
    assert_text(&blank, "TEXT(A1,\"#0.000\")", "0.000");

    let len = evaluation_book(&[]);
    assert_number(&len, "LEN(\"galactic\")", 8.0);
    assert_number(&len, "LEN(123456)", 6.0);
    assert_number(&len, "LEN(FALSE)", 5.0);
    assert_number(&len, "LEN(TRUE)", 4.0);

    let find = evaluation_book(&[]);
    assert_number(&find, "FIND(\"h\", \"haystack\")", 1.0);
    assert_number(&find, "FIND(\"a\", \"haystack\",2)", 2.0);
    assert_number(&find, "FIND(\"a\", \"haystack\",3)", 6.0);
    assert_number(&find, "FIND(7, 32768)", 3.0);
    assert_number(&find, "FIND(\"34\", 1341235233412, 3)", 10.0);
    assert_number(&find, "FIND(5, 87654)", 4.0);

    let concat = evaluation_book(&[
        ("A2", string("brook trout")),
        ("A3", string("species")),
        ("A4", string("32")),
        ("B1", string("A\u{2019}s")),
        ("C1", string("B\u{2019}s")),
        ("B2", string("a1")),
        ("C2", string("b1")),
        ("B3", string("a2")),
        ("C3", string("b2")),
        ("B4", FormulaValue::Blank),
        ("C4", FormulaValue::Blank),
        ("B5", string("a4")),
        ("C5", string("b4")),
        ("B6", string("a5")),
        ("C6", string("b5")),
        ("B7", string("a6")),
        ("C7", string("b6")),
        ("B8", string("a7")),
        ("C8", string("b7")),
        ("B10", string("Andreas")),
        ("C10", string("Hauser")),
        ("B11", string("Fourth")),
        ("C11", string("Pine")),
    ]);
    assert_text(&concat, "CONCAT(B2:C8)", "a1b1a2b2a4b4a5b5a6b6a7b7");
    assert_text(
        &concat,
        "CONCAT(\"Stream population for \", A2,\" \", A3, \" is \", A4, \"/mile.\")",
        "Stream population for brook trout species is 32/mile.",
    );
    assert_text(&concat, "CONCAT(B10,\" \", C10)", "Andreas Hauser");
    assert_text(&concat, "CONCAT(C10, \", \", B10)", "Hauser, Andreas");
    assert_text(&concat, "CONCAT(B11,\" & \", C11)", "Fourth & Pine");
    assert_text(&concat, "B11 & \" & \" & C11", "Fourth & Pine");

    for formula in [
        "CODE(\"\")",
        "LEFT(\"ANYSTRINGVALUE\",-1)",
        "RIGHT(\"ANYSTRINGVALUE\",-1)",
        "MID(\"galactic\",0,4)",
        "MID(\"galactic\",1,-1)",
        "SUBSTITUTE(\"ABC\",\"B\",\"CDE\",0)",
        "FIND(\"n\", \"haystack\")",
        "FIND(\"k\", \"haystack\",9)",
        "FIND(\"k\", \"haystack\",0)",
        "TEXT(43368,TRUE)",
        "TEXT(43368,FALSE)",
        "TEXT(3.14,TRUE)",
        "TEXT(3.14,FALSE)",
    ] {
        assert_error(&book, formula, FormulaErrorValue::Value);
    }
    assert_error(
        &book,
        "FIND(\"k\", \"haystack\",#REF!)",
        FormulaErrorValue::Ref,
    );
    assert_error(&book, "FIND(#DIV/0!, #N/A, #REF!)", FormulaErrorValue::Div0);
    assert_error(&book, "FIND(2, #N/A, #REF!)", FormulaErrorValue::NA);
}

#[test]
fn evaluates_apache_poi_lookup_reference_function_cases() {
    // Source: Apache POI
    // poi/src/test/java/org/apache/poi/ss/formula/functions/TestAddress.java,
    // TestIndex.java, TestOffset.java, TestRowCol.java, and TestMatch.java.
    let book = evaluation_book(&[]);
    for (formula, expected) in [
        ("ADDRESS(1,2)", "$B$1"),
        ("ADDRESS(1,2,)", "$B$1"),
        ("ADDRESS(22,44)", "$AR$22"),
        ("ADDRESS(1,1)", "$A$1"),
        ("ADDRESS(1,128)", "$DX$1"),
        ("ADDRESS(1,512)", "$SR$1"),
        ("ADDRESS(1,1000)", "$ALL$1"),
        ("ADDRESS(1,10000)", "$NTP$1"),
        ("ADDRESS(2,3)", "$C$2"),
        ("ADDRESS(2,3,2)", "C$2"),
        ("ADDRESS(2,3,2,,\"EXCEL SHEET\")", "'EXCEL SHEET'!C$2"),
        (
            "ADDRESS(2,3,3,TRUE,\"[Book1]Sheet1\")",
            "'[Book1]Sheet1'!$C2",
        ),
    ] {
        assert_text(&book, formula, expected);
    }

    let grid = evaluation_book(&[
        ("A1", number_value(1.0)),
        ("B1", number_value(2.0)),
        ("C1", number_value(3.0)),
        ("A2", number_value(4.0)),
        ("B2", number_value(5.0)),
        ("C2", number_value(6.0)),
        ("A3", number_value(7.0)),
        ("B3", number_value(8.0)),
        ("C3", number_value(9.0)),
    ]);
    assert_number(&grid, "INDEX(A1:C3,2,2)", 5.0);
    assert_number(&grid, "INDEX(A1:C3,3,2)", 8.0);
    assert_number(&grid, "SUM(INDEX(A1:C3,1,0))", 6.0);
    assert_number(&grid, "SUM(INDEX(A1:C3,2,0))", 15.0);
    assert_number(&grid, "SUM(INDEX(A1:C3,0,1))", 12.0);
    assert_number(&grid, "SUM(INDEX(A1:C3,0,3))", 18.0);
    assert_number(&grid, "SUM(B1:INDEX(B1:B3,2))", 7.0);
    assert_number(&grid, "INDEX({1,2;3,4},0,2)", 2.0);
    assert_number(&grid, "OFFSET(INDEX(A1:B2,2,1),1,1,1,1)", 8.0);

    let offset = evaluation_book(&[("B1", string("EXPECTED_VALUE"))]);
    assert_text(&offset, "OFFSET(B1,,)", "EXPECTED_VALUE");

    assert_number(&grid, "COLUMN(C5)", 3.0);
    assert_number(&grid, "COLUMN(E2:H12)", 5.0);
    assert_number(&grid, "ROW(C5)", 5.0);
    assert_number(&grid, "ROW(E2:H12)", 2.0);
    assert_number(&grid, "COLUMNS(A1:F1)", 6.0);
    assert_number(&grid, "COLUMNS(A1:C2)", 3.0);
    assert_number(&grid, "COLUMNS(A1:B3)", 2.0);
    assert_number(&grid, "COLUMNS(A1:A6)", 1.0);
    assert_number(&grid, "COLUMNS(C5)", 1.0);
    assert_number(&grid, "ROWS(A1:F1)", 1.0);
    assert_number(&grid, "ROWS(A1:C2)", 2.0);
    assert_number(&grid, "ROWS(A1:B3)", 3.0);
    assert_number(&grid, "ROWS(A1:A6)", 6.0);
    assert_number(&grid, "ROWS(C5)", 1.0);

    let numbers = evaluation_book(&[
        ("A1", number_value(4.0)),
        ("A2", number_value(5.0)),
        ("A3", number_value(10.0)),
        ("A4", number_value(10.0)),
        ("A5", number_value(25.0)),
    ]);
    assert_number(&numbers, "MATCH(5,A1:A5,1)", 2.0);
    assert_number(&numbers, "MATCH(5,A1:A5)", 2.0);
    assert_number(&numbers, "MATCH(5,A1:A5,0)", 2.0);
    assert_number(&numbers, "MATCH(10,A1:A5,1)", 4.0);
    assert_number(&numbers, "MATCH(10,A1:A5,0)", 3.0);
    assert_number(&numbers, "MATCH(20,A1:A5,1)", 4.0);
    assert_error(&numbers, "MATCH(20,A1:A5,0)", FormulaErrorValue::NA);

    let reversed = evaluation_book(&[
        ("A1", number_value(25.0)),
        ("A2", number_value(10.0)),
        ("A3", number_value(10.0)),
        ("A4", number_value(10.0)),
        ("A5", number_value(4.0)),
    ]);
    assert_number(&reversed, "MATCH(10,A1:A5,-1)", 2.0);
    assert_number(&reversed, "MATCH(10,A1:A5,0)", 2.0);
    assert_number(&reversed, "MATCH(9,A1:A5,-1)", 4.0);
    assert_number(&reversed, "MATCH(20,A1:A5,-1)", 1.0);
    assert_number(&reversed, "MATCH(3,A1:A5,-1)", 5.0);
    assert_error(&reversed, "MATCH(20,A1:A5,0)", FormulaErrorValue::NA);
    assert_error(&reversed, "MATCH(26,A1:A5,-1)", FormulaErrorValue::NA);

    let names = evaluation_book(&[
        ("A1", string("Albert")),
        ("A2", string("Charles")),
        ("A3", string("Ed")),
        ("A4", string("Greg")),
        ("A5", string("Ian")),
    ]);
    assert_number(&names, "MATCH(\"Ed\",A1:A5,1)", 3.0);
    assert_number(&names, "MATCH(\"eD\",A1:A5,1)", 3.0);
    assert_number(&names, "MATCH(\"Ed\",A1:A5,0)", 3.0);
    assert_number(&names, "MATCH(\"ed\",A1:A5,0)", 3.0);
    assert_error(&names, "MATCH(\"Hugh\",A1:A5,0)", FormulaErrorValue::NA);
    assert_number(&names, "MATCH(\"e*\",A1:A5,0)", 3.0);
    assert_number(&names, "MATCH(\"*d\",A1:A5,0)", 3.0);
    assert_number(&names, "MATCH(\"Al*\",A1:A5,0)", 1.0);
    assert_number(&names, "MATCH(\"Char*\",A1:A5,0)", 2.0);
    assert_number(&names, "MATCH(\"*eg\",A1:A5,0)", 4.0);
    assert_number(&names, "MATCH(\"G?eg\",A1:A5,0)", 4.0);
    assert_number(&names, "MATCH(\"??eg\",A1:A5,0)", 4.0);
    assert_number(&names, "MATCH(\"G*?eg\",A1:A5,0)", 4.0);
    assert_number(&names, "MATCH(\"Hugh\",A1:A5,1)", 4.0);
    assert_number(&names, "MATCH(\"*Ian*\",A1:A5,0)", 5.0);
    assert_number(&names, "MATCH(\"*Ian*\",A1:A5,1)", 5.0);

    let wildcards = evaluation_book(&[("A1", string("what?")), ("A2", string("all*"))]);
    assert_number(&wildcards, "MATCH(\"what~?\",A1:A2,0)", 1.0);
    assert_number(&wildcards, "MATCH(\"all~*\",A1:A2,0)", 2.0);

    let booleans = evaluation_book(&[
        ("A1", FormulaValue::Boolean(false)),
        ("A2", FormulaValue::Boolean(false)),
        ("A3", FormulaValue::Boolean(true)),
        ("A4", FormulaValue::Boolean(true)),
    ]);
    assert_number(&booleans, "MATCH(FALSE,A1:A4,1)", 2.0);
    assert_number(&booleans, "MATCH(FALSE,A1:A4,0)", 1.0);
    assert_number(&booleans, "MATCH(TRUE,A1:A4,1)", 4.0);
    assert_number(&booleans, "MATCH(TRUE,A1:A4,0)", 3.0);
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

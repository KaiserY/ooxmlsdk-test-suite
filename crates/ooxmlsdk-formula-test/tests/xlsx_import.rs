use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk_corpus_test_support::corpus_file_path;
use ooxmlsdk_formula::{
    BuiltInName, CellAddress, CellRange, FormulaErrorValue, FormulaKind, FormulaValue, SheetId,
    WorkbookValueModel,
};

fn workbook(relative_path: &str) -> WorkbookValueModel<'static> {
    let path = corpus_file_path(relative_path);
    let mut document = SpreadsheetDocument::new_from_file(&path).unwrap_or_else(|err| {
        panic!("failed to open {}: {err:?}", path.display());
    });
    WorkbookValueModel::from_spreadsheet_document(&mut document).unwrap_or_else(|err| {
        panic!(
            "failed to build formula model for {}: {err:?}",
            path.display()
        );
    })
}

fn recalculated_workbook(relative_path: &str) -> WorkbookValueModel<'static> {
    let mut model = workbook(relative_path);
    model.evaluate_supported_formulas();
    model
}

fn address(reference: &str) -> CellAddress {
    CellAddress::parse_a1(reference).unwrap()
}

fn range(start: &str, end: &str) -> CellRange {
    CellRange::new(address(start), address(end))
}

fn sheet_id(model: &WorkbookValueModel<'_>, name: &str) -> SheetId {
    model
        .identity
        .sheets
        .iter()
        .find(|sheet| sheet.name == name)
        .map(|sheet| sheet.id)
        .unwrap_or_else(|| panic!("sheet {name} not found"))
}

fn cell_value<'a>(
    model: &'a WorkbookValueModel<'a>,
    sheet: SheetId,
    reference: &str,
) -> FormulaValue<'a> {
    model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .and_then(|sheet| sheet.cells.get(&address(reference)))
        .map(|record| {
            record
                .formula
                .as_ref()
                .and_then(|formula| {
                    formula
                        .evaluated_value
                        .clone()
                        .or_else(|| formula.cached_value.clone())
                })
                .unwrap_or_else(|| record.raw_value.clone())
        })
        .unwrap_or_else(|| panic!("cell {reference} not found"))
}

fn formula_text<'a>(model: &'a WorkbookValueModel<'a>, sheet: SheetId, reference: &str) -> &'a str {
    model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .and_then(|sheet| sheet.cells.get(&address(reference)))
        .and_then(|record| record.formula.as_ref())
        .map(|formula| formula.formula_text.as_ref())
        .unwrap_or_else(|| panic!("formula {reference} not found"))
}

fn formula_text_opt<'a>(
    model: &'a WorkbookValueModel<'a>,
    sheet: SheetId,
    reference: &str,
) -> Option<&'a str> {
    model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .and_then(|sheet| sheet.cells.get(&address(reference)))
        .and_then(|record| record.formula.as_ref())
        .map(|formula| formula.formula_text.as_ref())
}

fn display_text<'a>(model: &'a WorkbookValueModel<'a>, sheet: SheetId, reference: &str) -> &'a str {
    model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .and_then(|sheet| sheet.cells.get(&address(reference)))
        .and_then(|record| record.display_value.as_ref())
        .map(|display| display.text.as_ref())
        .unwrap_or_else(|| panic!("display text {reference} not found"))
}

fn assert_cell_value(
    model: &WorkbookValueModel<'_>,
    sheet: SheetId,
    reference: &str,
    expected: FormulaValue<'_>,
) {
    assert_eq!(cell_value(model, sheet, reference), expected, "{reference}");
}

fn assert_cell_numeric_value(
    model: &WorkbookValueModel<'_>,
    sheet: SheetId,
    reference: &str,
    expected: f64,
) {
    let actual = cell_value(model, sheet, reference);
    let actual = match actual {
        FormulaValue::Number(value) => value,
        FormulaValue::Boolean(value) => {
            if value {
                1.0
            } else {
                0.0
            }
        }
        FormulaValue::String(value) => value
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("{reference}: non-numeric string value {value:?}")),
        value => panic!("{reference}: non-numeric value {value:?}"),
    };
    assert_eq!(actual, expected, "{reference}");
}

fn column_name(mut column: u32) -> String {
    let mut name = Vec::new();
    loop {
        let rem = column % 26;
        name.push((b'A' + rem as u8) as char);
        column /= 26;
        if column == 0 {
            break;
        }
        column -= 1;
    }
    name.iter().rev().collect()
}

fn cell_value_opt<'a>(
    model: &'a WorkbookValueModel<'a>,
    sheet: SheetId,
    reference: &str,
) -> Option<FormulaValue<'a>> {
    model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .and_then(|sheet| sheet.cells.get(&address(reference)))
        .map(|record| {
            record
                .formula
                .as_ref()
                .and_then(|formula| {
                    formula
                        .evaluated_value
                        .clone()
                        .or_else(|| formula.cached_value.clone())
                })
                .unwrap_or_else(|| record.raw_value.clone())
        })
}

fn evaluated_formula_value<'a>(
    model: &'a WorkbookValueModel<'a>,
    sheet: SheetId,
    reference: &str,
) -> Option<FormulaValue<'a>> {
    model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .and_then(|sheet| sheet.cells.get(&address(reference)))
        .and_then(|record| record.formula.as_ref())
        .and_then(|formula| formula.evaluated_value.clone())
}

fn assert_poi_formula_value_matches(
    actual: Option<FormulaValue<'_>>,
    expected: FormulaValue<'_>,
    context: &str,
) {
    match (actual, expected) {
        (Some(FormulaValue::Number(actual)), FormulaValue::Number(expected)) => assert!(
            (actual - expected).abs() <= 1e-8,
            "{context}: expected {expected}, got {actual}"
        ),
        (Some(FormulaValue::Boolean(actual)), FormulaValue::Boolean(expected)) => {
            assert_eq!(actual, expected, "{context}")
        }
        (Some(FormulaValue::String(actual)), FormulaValue::String(expected)) => {
            assert_eq!(actual, expected, "{context}")
        }
        (Some(FormulaValue::Blank), FormulaValue::Blank) => {}
        (Some(FormulaValue::Error(_)), FormulaValue::Error(_)) => {}
        (actual, expected) => panic!("{context}: expected {expected:?}, got {actual:?}"),
    }
}

#[test]
fn imports_apache_poi_xlookup_fixture_cached_and_recalculated_values() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/TestXSSFXLookupFunction.java::testXLookupFile.
    let model = workbook("Apache-POI/test-data/spreadsheet/xlookup.xlsx");
    let sheet = sheet_id(&model, "Sheet1");
    assert_eq!(
        cell_value(&model, sheet, "C2"),
        FormulaValue::String("Dianne Pugh".into())
    );
    assert_eq!(
        cell_value(&model, sheet, "D2"),
        FormulaValue::String("Finance".into())
    );

    let recalculated = recalculated_workbook("Apache-POI/test-data/spreadsheet/xlookup.xlsx");
    let sheet = sheet_id(&recalculated, "Sheet1");
    assert_eq!(
        cell_value(&recalculated, sheet, "C2"),
        FormulaValue::String("Dianne Pugh".into())
    );
    assert_eq!(
        cell_value(&recalculated, sheet, "D2"),
        FormulaValue::String("Finance".into())
    );
}

#[test]
fn evaluates_apache_poi_xssf_shared_formula_fixtures() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java
    // testSharedFormulas_evaluateInCell and testEvaluateColumnGreaterThan255.
    let shared = workbook("Apache-POI/test-data/spreadsheet/49872.xlsx");
    let sheet = SheetId(1);
    assert_eq!(formula_text(&shared, sheet, "B3"), "B1+B2");
    assert_eq!(formula_text(&shared, sheet, "C3"), "C1+C2");
    assert_eq!(formula_text(&shared, sheet, "D3"), "D1+D2");
    assert_cell_numeric_value(&shared, sheet, "B3", 3.0);
    assert_cell_numeric_value(&shared, sheet, "C3", 3.0);
    assert_cell_numeric_value(&shared, sheet, "D3", 3.0);

    let recalculated_shared = recalculated_workbook("Apache-POI/test-data/spreadsheet/49872.xlsx");
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_shared, sheet, "B3"),
        FormulaValue::Number(3.0),
        "49872.xlsx B3",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_shared, sheet, "C3"),
        FormulaValue::Number(3.0),
        "49872.xlsx C3",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_shared, sheet, "D3"),
        FormulaValue::Number(3.0),
        "49872.xlsx D3",
    );

    let columns = workbook("Apache-POI/test-data/spreadsheet/50096.xlsx");
    for column in 245_u32..265 {
        let value_ref = format!("{}1", column_name(column));
        let formula_ref = format!("{}2", column_name(column));
        assert_eq!(
            formula_text(&columns, sheet, &formula_ref),
            value_ref,
            "{formula_ref}"
        );
        assert_eq!(
            cell_value(&columns, sheet, &formula_ref),
            cell_value(&columns, sheet, &value_ref),
            "{formula_ref}"
        );
    }

    let recalculated_columns = recalculated_workbook("Apache-POI/test-data/spreadsheet/50096.xlsx");
    for column in 245_u32..265 {
        let value_ref = format!("{}1", column_name(column));
        let formula_ref = format!("{}2", column_name(column));
        let expected = cell_value(&recalculated_columns, sheet, &value_ref);
        assert_poi_formula_value_matches(
            evaluated_formula_value(&recalculated_columns, sheet, &formula_ref),
            expected,
            &format!("50096.xlsx {formula_ref}"),
        );
    }
}

#[test]
fn evaluates_apache_poi_xssf_multisheet_reference_fixture() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java
    // testMultiSheetReferencesHSSFandXSSF and testMultiSheetAreasHSSFandXSSF.
    let model = workbook("Apache-POI/test-data/spreadsheet/55906-MultiSheetRefs.xlsx");
    let sheet = SheetId(1);
    for (reference, formula, expected) in [
        ("A3", "SUM(Sheet1:Sheet3!A1)", 66.0),
        ("B3", "AVERAGE(Sheet1:Sheet3!A1)", 22.0),
        ("B4", "MIN(Sheet1:Sheet3!A$1)", 11.0),
        ("B5", "MAX(Sheet1:Sheet3!A$1)", 33.0),
        ("B6", "COUNT(Sheet1:Sheet3!A$1)", 3.0),
        ("C3", "COUNTA(Sheet1:Sheet3!C1)", 3.0),
        ("D3", "COUNTA(Sheet1:Sheet3!D1)", 0.0),
        ("E3", "COUNTA(Sheet1:Sheet3!E1)", 3.0),
        ("H3", "SUM(Sheet1:Sheet3!A1:B2)", 110.0),
        ("I3", "AVERAGE(Sheet1:Sheet3!A1:B2)", 27.5),
        ("I4", "MIN(Sheet1:Sheet3!A$1:B$2)", 11.0),
        ("I5", "MAX(Sheet1:Sheet3!A$1:B$2)", 44.0),
        ("I6", "COUNT(Sheet1:Sheet3!$A$1:$B$2)", 4.0),
    ] {
        assert_eq!(
            formula_text(&model, sheet, reference),
            formula,
            "{reference}"
        );
        assert_cell_numeric_value(&model, sheet, reference, expected);
    }

    let recalculated =
        recalculated_workbook("Apache-POI/test-data/spreadsheet/55906-MultiSheetRefs.xlsx");
    for (reference, expected) in [
        ("A3", 66.0),
        ("B3", 22.0),
        ("B4", 11.0),
        ("B5", 33.0),
        ("B6", 3.0),
        ("C3", 3.0),
        ("D3", 0.0),
        ("E3", 3.0),
        ("H3", 110.0),
        ("I3", 27.5),
        ("I4", 11.0),
        ("I5", 44.0),
        ("I6", 4.0),
    ] {
        assert_poi_formula_value_matches(
            evaluated_formula_value(&recalculated, sheet, reference),
            FormulaValue::Number(expected),
            &format!("55906-MultiSheetRefs.xlsx {reference}"),
        );
    }
}

#[test]
fn evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java
    // test59736, testBug61468, testBug61495, testBug62834, testBug63934,
    // and testBug60848_sumproductWithUnaryMinusArray.
    let cached_59736 = workbook("Apache-POI/test-data/spreadsheet/59736.xlsx");
    let sheet = SheetId(1);
    assert_cell_numeric_value(&cached_59736, sheet, "A1", 1.0);
    let recalculated_59736 = recalculated_workbook("Apache-POI/test-data/spreadsheet/59736.xlsx");
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_59736, sheet, "A2"),
        FormulaValue::Number(1.0),
        "59736.xlsx A2",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_59736, sheet, "A3"),
        FormulaValue::Number(1.0),
        "59736.xlsx A3",
    );

    let budget = workbook("Apache-POI/test-data/spreadsheet/simple-monthly-budget.xlsx");
    assert_cell_numeric_value(&budget, sheet, "E9", 3750.0);
    let recalculated_budget =
        recalculated_workbook("Apache-POI/test-data/spreadsheet/simple-monthly-budget.xlsx");
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_budget, sheet, "E9"),
        FormulaValue::Number(3750.0),
        "simple-monthly-budget.xlsx E9",
    );

    let recalculated_62834 = recalculated_workbook("Apache-POI/test-data/spreadsheet/62834.xlsx");
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_62834, sheet, "A2"),
        FormulaValue::String("a value".into()),
        "62834.xlsx A2",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_62834, sheet, "A3"),
        FormulaValue::String("a value".into()),
        "62834.xlsx A3",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_62834, sheet, "A5"),
        FormulaValue::String("another value".into()),
        "62834.xlsx A5",
    );

    let recalculated_63934 = recalculated_workbook("Apache-POI/test-data/spreadsheet/63934.xlsx");
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_63934, sheet, "B2"),
        FormulaValue::String("Male".into()),
        "63934.xlsx B2",
    );

    let formatting = workbook("Apache-POI/test-data/spreadsheet/61495-test.xlsm");
    assert_eq!(
        formula_text(&formatting, sheet, "B1"),
        "IF(TEST!A1=\"\",\"\",CONCATENATE(\"D\",\" \",TEXT(TEST!A1,\"00.00\")))"
    );
    assert_eq!(
        formula_text(&formatting, sheet, "B2"),
        "IF(TEST!A2=\"\",\"\",CONCATENATE(\"D\",\" \",TEXT(TEST!A2,\"00,00\")))"
    );
    let recalculated_formatting =
        recalculated_workbook("Apache-POI/test-data/spreadsheet/61495-test.xlsm");
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_formatting, sheet, "B1"),
        FormulaValue::String("D 67.10".into()),
        "61495-test.xlsm B1",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated_formatting, sheet, "B2"),
        FormulaValue::String("D 0,068".into()),
        "61495-test.xlsm B2",
    );

    let sumproduct = recalculated_workbook(
        "Apache-POI/test-data/spreadsheet/bug60848_sumproduct_unary_minus.xlsx",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&sumproduct, sheet, "A3"),
        FormulaValue::Number(0.0),
        "bug60848_sumproduct_unary_minus.xlsx A3",
    );
}

#[test]
fn imports_apache_poi_external_reference_formula_fixture() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java
    // testReferencesToOtherWorkbooks. The POI Java API exception/linking
    // assertions are intentionally omitted; this keeps only formula text and
    // value behavior.
    let model = workbook("Apache-POI/test-data/spreadsheet/ref2-56737.xlsx");
    let sheet = SheetId(1);
    for (reference, formula, expected) in [
        (
            "E3",
            "'56737.xlsx'#$Uses.$A$1",
            FormulaValue::String("Hello!".into()),
        ),
        (
            "G3",
            "'56737.xlsx'#$Defines.NR_To_A1",
            FormulaValue::String("Test A1".into()),
        ),
        (
            "I3",
            "'56737.xlsx'#$.NR_Global_B2",
            FormulaValue::Number(142.0),
        ),
        (
            "E5",
            "'56737.xls'#$Uses.$C$1",
            FormulaValue::String("Hello!".into()),
        ),
        (
            "G5",
            "'56737.xls'#$Defines.NR_To_A1",
            FormulaValue::String("Test A1".into()),
        ),
        (
            "I5",
            "'56737.xls'#$.NR_Global_B2",
            FormulaValue::Number(142.0),
        ),
    ] {
        assert_eq!(
            formula_text(&model, sheet, reference),
            formula,
            "{reference}"
        );
        assert_eq!(
            cell_value(&model, sheet, reference),
            expected,
            "{reference}"
        );
    }

    let recalculated = recalculated_workbook("Apache-POI/test-data/spreadsheet/ref2-56737.xlsx");
    for (reference, expected) in [
        ("E3", FormulaValue::String("Hello!".into())),
        ("G3", FormulaValue::String("Test A1".into())),
        ("I3", FormulaValue::Number(142.0)),
        ("E5", FormulaValue::String("Hello!".into())),
        ("G5", FormulaValue::String("Test A1".into())),
        ("I5", FormulaValue::Number(142.0)),
    ] {
        assert_poi_formula_value_matches(
            evaluated_formula_value(&recalculated, sheet, reference),
            expected,
            &format!("ref2-56737.xlsx {reference}"),
        );
    }
}

#[test]
fn evaluates_apache_poi_structured_reference_formula_fixture() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java
    // verifyAllFormulasInWorkbookCanBeEvaluated for
    // evaluate_formula_with_structured_table_references.xlsx.
    let model = workbook(
        "Apache-POI/test-data/spreadsheet/evaluate_formula_with_structured_table_references.xlsx",
    );
    let sheet = sheet_id(&model, "Tabelle1");
    assert_eq!(formula_text(&model, sheet, "C3"), "SUM(Table1[[A]:[B]])");
    assert_cell_numeric_value(&model, sheet, "C3", 10.0);

    let recalculated = recalculated_workbook(
        "Apache-POI/test-data/spreadsheet/evaluate_formula_with_structured_table_references.xlsx",
    );
    assert_poi_formula_value_matches(
        evaluated_formula_value(&recalculated, sheet, "C3"),
        FormulaValue::Number(10.0),
        "evaluate_formula_with_structured_table_references.xlsx C3",
    );
}

#[test]
fn evaluates_apache_poi_formula_eval_test_data_copy_fixture() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestFormulaEvaluatorOnXSSF.java.
    let mut model = workbook("Apache-POI/test-data/spreadsheet/FormulaEvalTestData_Copy.xlsx");
    model.evaluate_supported_formulas();
    let sheet = sheet_id(&model, "EverythingTests");
    let sheet_model = model
        .sheets
        .iter()
        .find(|item| item.id == sheet)
        .expect("EverythingTests sheet");

    let mut checked = 0usize;
    for start_row in [22_u32, 95_u32] {
        let mut row = start_row;
        loop {
            let function_name_ref = format!("B{}", row + 1);
            let Some(FormulaValue::String(function_name)) =
                cell_value_opt(&model, sheet, &function_name_ref)
            else {
                row += 4;
                continue;
            };
            if function_name == "<END-OF-FUNCTIONS>" {
                break;
            }
            if function_name.eq_ignore_ascii_case("DOLLAR") {
                row += 4;
                continue;
            }

            let mut formula_columns = sheet_model
                .cells
                .iter()
                .filter_map(|(address, record)| {
                    (address.row == row && address.column >= 3 && record.formula.is_some())
                        .then_some(address.column)
                })
                .collect::<Vec<_>>();
            formula_columns.sort_unstable();

            for column in formula_columns {
                let reference = format!("{}{}", column_name(column), row + 1);
                let formula = formula_text(&model, sheet, &reference);
                if formula == "COLUMN(1:2)"
                    || formula == "ROW(2:3)"
                    || formula == "ISREF(currentcell())"
                {
                    continue;
                }
                let expected_ref = format!("{}{}", column_name(column), row + 2);
                let expected = cell_value_opt(&model, sheet, &expected_ref)
                    .unwrap_or_else(|| panic!("{expected_ref}: missing expected value"));
                let context = format!("{function_name} {reference}={formula}");
                assert_poi_formula_value_matches(
                    evaluated_formula_value(&model, sheet, &reference),
                    expected,
                    &context,
                );
                checked += 1;
            }

            row += 4;
        }
    }
    assert!(checked > 1000, "expected broad POI formula coverage");
}

#[test]
fn evaluates_apache_poi_matrix_formula_eval_fixture() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestMatrixFormulasFromXMLSpreadsheet.java.
    let mut model = workbook("Apache-POI/test-data/spreadsheet/MatrixFormulaEvalTestData.xlsx");
    model.evaluate_supported_formulas();
    let sheet = SheetId(1);

    let mut checked = 0usize;
    let mut row = 1_u32;
    loop {
        let function_name_ref = format!("A{}", row + 1);
        let Some(FormulaValue::String(function_name)) =
            cell_value_opt(&model, sheet, &function_name_ref)
        else {
            panic!("{function_name_ref}: missing matrix formula test name");
        };
        if function_name == "<END>" {
            break;
        }

        for formula_row in row..row + 3 {
            for column in 7_u32..10 {
                let reference = format!("{}{}", column_name(column), formula_row + 1);
                let Some(formula) = formula_text_opt(&model, sheet, &reference) else {
                    continue;
                };
                let expected_ref = format!("{}{}", column_name(column + 3), formula_row + 1);
                let expected = cell_value_opt(&model, sheet, &expected_ref)
                    .unwrap_or_else(|| panic!("{expected_ref}: missing expected value"));
                let context = format!("{function_name} {reference}={formula}");
                assert_poi_formula_value_matches(
                    evaluated_formula_value(&model, sheet, &reference),
                    expected,
                    &context,
                );
                checked += 1;
            }
        }

        row += 4;
    }
    assert!(checked > 20, "expected matrix formula coverage");
}

#[test]
fn evaluates_apache_poi_formula_sheet_range_fixture() {
    // Source: Apache POI
    // poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestMultiSheetFormulaEvaluatorOnXSSF.java.
    let mut model = workbook("Apache-POI/test-data/spreadsheet/FormulaSheetRange.xlsx");
    model.evaluate_supported_formulas();
    let sheet = sheet_id(&model, "test");

    let mut checked = 0usize;
    let mut row = 10_u32;
    loop {
        let function_name_ref = format!("A{}", row + 1);
        let Some(FormulaValue::String(function_name)) =
            cell_value_opt(&model, sheet, &function_name_ref)
        else {
            row += 1;
            continue;
        };
        if function_name == "<END>" {
            break;
        }
        let test_name = match cell_value_opt(&model, sheet, &format!("B{}", row + 1)) {
            Some(FormulaValue::String(value)) => value.into_owned(),
            _ => String::new(),
        };
        let reference = format!("D{}", row + 1);
        let expected_ref = format!("C{}", row + 1);
        let expected = cell_value_opt(&model, sheet, &expected_ref)
            .unwrap_or_else(|| panic!("{expected_ref}: missing expected value"));
        let context = format!(
            "{function_name} {test_name} {reference}={}",
            formula_text(&model, sheet, &reference)
        );
        assert_poi_formula_value_matches(
            evaluated_formula_value(&model, sheet, &reference),
            expected,
            &context,
        );
        checked += 1;
        row += 1;
    }
    assert!(checked > 1, "expected sheet-range formula coverage");
}

#[test]
fn imports_named_ranges_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test.cxx::testRangeNameXLSX.
    let model = recalculated_workbook("LibreOffice/sc/qa/unit/data/xlsx/named-ranges-global.xlsx");
    let sheet1 = sheet_id(&model, "Sheet1");
    let sheet2 = sheet_id(&model, "Sheet2");

    assert!(
        model
            .defined_names
            .iter()
            .any(|name| name.name == "Global1" && name.sheet.is_none())
    );
    assert!(
        model
            .defined_names
            .iter()
            .any(|name| name.name.eq_ignore_ascii_case("local1") && name.sheet == Some(sheet1))
    );
    assert!(
        model
            .defined_names
            .iter()
            .any(|name| name.name.eq_ignore_ascii_case("local2") && name.sheet == Some(sheet2))
    );

    assert_eq!(cell_value(&model, sheet1, "B1"), FormulaValue::Number(1.0));
    assert_eq!(cell_value(&model, sheet1, "B3"), FormulaValue::Number(3.0));
    assert_eq!(cell_value(&model, sheet2, "B2"), FormulaValue::Number(7.0));
    assert_eq!(cell_value(&model, sheet1, "B2"), FormulaValue::Number(2.0));
    assert_eq!(cell_value(&model, sheet1, "B4"), FormulaValue::Number(4.0));
    assert_eq!(cell_value(&model, sheet1, "C1"), FormulaValue::Number(10.0));
    assert_eq!(cell_value(&model, sheet2, "B1"), FormulaValue::Number(5.0));
    assert_eq!(cell_value(&model, sheet2, "A6"), FormulaValue::Number(5.0));
}

#[test]
fn imports_hidden_named_ranges_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test.cxx::testHiddenRangeNameXLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/named-ranges-hidden.xlsx");
    let named_range1 = model
        .defined_names
        .iter()
        .find(|name| name.name.eq_ignore_ascii_case("NamedRange1"))
        .expect("NamedRange1");
    let named_range2 = model
        .defined_names
        .iter()
        .find(|name| name.name.eq_ignore_ascii_case("NamedRange2"))
        .expect("NamedRange2");

    assert!(named_range1.hidden);
    assert!(!named_range2.hidden);
}

#[test]
fn imports_data_table_formulas_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test.cxx::testDataTableOneVarXLSX
    // and testDataTableMultiTableXLSX.
    let one_var = workbook("LibreOffice/sc/qa/unit/data/xlsx/data-table/one-variable.xlsx");
    assert!(one_var.data_tables.iter().any(|table| {
        table.sheet == SheetId(1)
            && table.range == range("B5", "B11")
            && !table.row_table
            && !table.two_dimensional
    }));
    assert_eq!(formula_text(&one_var, SheetId(1), "B5"), "TABLE(A5,A2)");
    assert_eq!(
        cell_value(&one_var, SheetId(1), "B5"),
        FormulaValue::Number(2.0)
    );
    assert_eq!(formula_text(&one_var, SheetId(1), "B11"), "TABLE(A11,A2)");
    assert_eq!(
        cell_value(&one_var, SheetId(1), "B11"),
        FormulaValue::Number(14.0)
    );
    assert_eq!(formula_text(&one_var, SheetId(1), "E5"), "TABLE(E4,B2)");
    assert_eq!(
        cell_value(&one_var, SheetId(1), "E5"),
        FormulaValue::Number(10.0)
    );
    assert_eq!(formula_text(&one_var, SheetId(1), "I5"), "TABLE(I4,B2)");
    assert_eq!(
        cell_value(&one_var, SheetId(1), "I5"),
        FormulaValue::Number(50.0)
    );

    let multi = workbook("LibreOffice/sc/qa/unit/data/xlsx/data-table/multi-table.xlsx");
    assert!(
        multi
            .data_tables
            .iter()
            .any(|table| table.sheet == SheetId(1) && table.range == range("B4", "M15"))
    );
    assert_eq!(formula_text(&multi, SheetId(1), "B4"), "TABLE(A4,E1,D1,B3)");
    assert_eq!(
        cell_value(&multi, SheetId(1), "B4"),
        FormulaValue::Number(1.0)
    );
    assert_eq!(
        formula_text(&multi, SheetId(1), "M15"),
        "TABLE(A15,E1,D1,M3)"
    );
    assert_eq!(
        cell_value(&multi, SheetId(1), "M15"),
        FormulaValue::Number(144.0)
    );
}

#[test]
fn imports_spill_formula_metadata_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test.cxx::testArrayFormulaSpillXLSX
    // and testConventionalArrayFormulaSpillXLSX. The edit-time blocker clearing behavior is
    // outside the current formula model; this test ports the import state.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/Spill.xlsx");
    let d2 = model.sheets[0]
        .cells
        .get(&address("D2"))
        .and_then(|record| record.formula.as_ref())
        .expect("D2 formula");
    assert_eq!(d2.formula_text, "_xlfn.UNIQUE($A$2:$A$5)");
    assert_eq!(
        d2.cached_value,
        Some(FormulaValue::Error(
            ooxmlsdk_formula::FormulaErrorValue::Spill
        ))
    );
    assert_eq!(
        cell_value(&model, SheetId(1), "D5"),
        FormulaValue::String("block".into())
    );

    let g2 = model.sheets[0]
        .cells
        .get(&address("G2"))
        .and_then(|record| record.formula.as_ref())
        .expect("G2 formula");
    assert_eq!(g2.formula_kind, FormulaKind::Array);
    assert_eq!(g2.formula_text, "$A$2:$A$5");
    assert_eq!(
        g2.cached_value,
        Some(FormulaValue::Error(
            ooxmlsdk_formula::FormulaErrorValue::Spill
        ))
    );
    assert_eq!(
        cell_value(&model, SheetId(1), "G5"),
        FormulaValue::String("block".into())
    );
}

#[test]
fn imports_external_reference_cache_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test2.cxx::testExternalRefCacheXLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/external-refs.xlsx");
    let cached = model
        .external_cached_cells
        .iter()
        .map(|cell| (cell.reference.as_ref(), cell.value.clone()))
        .collect::<Vec<_>>();
    assert!(cached.contains(&("A1", FormulaValue::String("Name".into()))));
    assert!(cached.contains(&("A2", FormulaValue::String("Andy".into()))));
    assert!(cached.contains(&("A3", FormulaValue::String("Bruce".into()))));
    assert!(cached.contains(&("A4", FormulaValue::String("Charlie".into()))));
}

#[test]
fn imports_shared_formula_cached_values_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test3.cxx::testSharedFormulaExportXLSX
    // and testSharedFormulaStringResultExportXLSX.
    let numeric = workbook("LibreOffice/sc/qa/unit/data/xlsx/shared-formula/3d-reference.xlsx");
    for row in 2..=7 {
        assert_eq!(
            cell_value(&numeric, SheetId(1), &format!("B{row}")),
            FormulaValue::Number((row - 1) as f64)
        );
        assert_eq!(
            cell_value(&numeric, SheetId(1), &format!("C{row}")),
            FormulaValue::Number(((row - 1) * 10) as f64)
        );
        assert_eq!(
            cell_value(&numeric, SheetId(1), &format!("D{row}")),
            FormulaValue::Number((row - 1) as f64)
        );
    }

    let text = workbook("LibreOffice/sc/qa/unit/data/xlsx/shared-formula/text-results.xlsx");
    for (row, expected) in (2..=7).zip(["A", "B", "C", "D", "E", "F"]) {
        assert_eq!(
            cell_value(&text, SheetId(1), &format!("B{row}")),
            FormulaValue::String(expected.into())
        );
    }
    for (row, expected) in (2..=7).zip(["AA", "BB", "CC", "DD", "EE", "FF"]) {
        assert_eq!(
            cell_value(&text, SheetId(1), &format!("C{row}")),
            FormulaValue::String(expected.into())
        );
    }
}

#[test]
fn imports_shared_formula_group_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test4.cxx::testSharedFormulaXLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/shared-formula/basic.xlsx");
    let sheet = SheetId(1);

    for row in 2..=19 {
        assert_cell_value(
            &model,
            sheet,
            &format!("B{row}"),
            FormulaValue::Number(((row - 1) * 10) as f64),
        );
    }

    assert!(model.shared_formula_groups.iter().any(|group| {
        group.sheet == sheet
            && group.origin == address("B2")
            && group.range == Some(range("B2", "B19"))
            && group.dependents.len() == 17
    }));
}

#[test]
fn imports_shared_formula_refupdate_fixture_initial_state() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test4.cxx::testSharedFormulaRefUpdateXLSX.
    // LO deletes row 5 before asserting rewritten formulas; this imports the pre-edit shared
    // formula state only. The edit rewrite itself is a structural reference-update API gap.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/shared-formula/refupdate.xlsx");
    let sheet = SheetId(1);

    for reference in ["B1", "C1", "D1", "E1"] {
        assert!(formula_text(&model, sheet, reference).ends_with("+1"));
    }
}

#[test]
fn imports_builtin_defined_names_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test.cxx::testBuiltinRangesXLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/built-in_ranges.xlsx");
    assert!(model.defined_names.iter().any(|name| {
        name.built_in == Some(BuiltInName::FilterDatabase)
            && name.sheet == Some(SheetId(1))
            && name.formula_text == "'Sheet1 Test'!$A$1:$A$5"
    }));
    assert!(model.defined_names.iter().any(|name| {
        name.built_in == Some(BuiltInName::FilterDatabase)
            && name.sheet == Some(SheetId(2))
            && name.formula_text == "'Sheet2 Test'!$K$10:$K$14"
    }));
    assert!(model.defined_names.iter().any(
        |name| name.built_in == Some(BuiltInName::PrintArea) && name.sheet == Some(SheetId(1))
    ));
}

#[test]
fn imports_formula_intersection_and_cached_values_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test2.cxx::testTdf136364,
    // testTdf131424, and testRefStringXLSX.
    let intersection = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf136364.xlsx");
    let sheet = SheetId(1);
    assert_eq!(
        formula_text(&intersection, sheet, "E1"),
        "SUM((B2:B3,C4:C5,D6:D7))"
    );
    assert_cell_value(&intersection, sheet, "E1", FormulaValue::Number(27.0));
    assert_eq!(formula_text(&intersection, sheet, "E2"), "SUM((B2,C4,D6))");
    assert_cell_value(&intersection, sheet, "E2", FormulaValue::Number(12.0));

    let table_refs = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf131424.xlsx");
    for (reference, expected) in [("C2", 35.0), ("C3", 58.0), ("C4", 81.0), ("C5", 104.0)] {
        assert_cell_value(
            &table_refs,
            sheet,
            reference,
            FormulaValue::Number(expected),
        );
    }

    let ref_string = workbook("LibreOffice/sc/qa/unit/data/xlsx/ref_string.xlsx");
    assert_cell_value(&ref_string, sheet, "C3", FormulaValue::Number(3.0));
}

#[test]
fn imports_indirect_intersection_formula_text_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test2.cxx::testTdf160371.
    let model = recalculated_workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf160371.xlsx");
    let sheet = SheetId(1);

    assert_eq!(
        formula_text(&model, sheet, "B4"),
        "INDIRECT(B2)!INDIRECT(B3)"
    );
    assert_cell_value(&model, sheet, "B4", FormulaValue::Number(1.0));
}

#[test]
fn imports_named_table_reference_cache_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test2.cxx::testNamedTableRef.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/tablerefsnamed.xlsx");
    let sheet = SheetId(1);

    for row in 2..=7 {
        assert!(
            !matches!(
                cell_value(&model, sheet, &format!("F{row}")),
                FormulaValue::Error(FormulaErrorValue::Ref)
            ),
            "F{row}"
        );
        assert_cell_value(
            &model,
            sheet,
            &format!("G{row}"),
            FormulaValue::Boolean(true),
        );
    }
}

#[test]
fn imports_complex_formula_text_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test2.cxx::testTdf131536.
    let model = recalculated_workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf131536.xlsx");
    let sheet = SheetId(1);

    assert_cell_numeric_value(&model, sheet, "D10", 1.0);
    assert_eq!(
        formula_text(&model, sheet, "D10"),
        "IF(D$4=\"-\",\"-\",MID(TEXT(INDEX(Comparison!$I:$J,Comparison!$A5,Comparison!D$2),\"\")\
,2,4)=RIGHT(TEXT(INDEX(Comparison!$L:$Z,Comparison!$A5,Comparison!D$4),\"\"),4))"
    );

    assert_cell_numeric_value(&model, sheet, "E10", 1.0);
    assert_eq!(
        formula_text(&model, sheet, "E10"),
        "IF(D$4=\"-\",\"-\",MID(TEXT(INDEX(Comparison!$I:$J,Comparison!$A5,Comparison!D$2),\"0\")\
,2,4)=RIGHT(TEXT(INDEX(Comparison!$L:$Z,Comparison!$A5,Comparison!D$4),\"0\"),4))"
    );
}

#[test]
fn imports_structured_reference_formula_text_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test2.cxx::testTdf105272
    // and testTdf118990.
    let structured = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf105272.xlsx");
    assert_eq!(
        formula_text(&structured, SheetId(1), "H4"),
        "Table1[[#This Row],[Total]]/Table1[[#This Row],['# Athletes]]"
    );

    let external = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf118990.xlsx");
    assert_eq!(
        formula_text(&external, SheetId(1), "A2"),
        "VLOOKUP(B1,'file://192.168.1.1/share/lookupsource.xlsx'#$Sheet1.A1:B5,2)"
    );
    assert_eq!(
        formula_text(&external, SheetId(1), "A3"),
        "VLOOKUP(B1,'file://NETWORKHOST/share/lookupsource.xlsx'#$Sheet1.A1:B5,2)"
    );
}

#[test]
fn imports_cross_sheet_formula_persistence_regression_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test5.cxx::testTdf163554.
    let model = recalculated_workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf163554.xlsx");
    let sheet = sheet_id(&model, "time (misc) - last");

    assert_eq!(
        formula_text(&model, sheet, "A1"),
        "SUM($'time (misc) - last'.B1:$'time (pnrst)'.B1)"
    );
    assert_cell_value(&model, sheet, "A1", FormulaValue::Number(7.0));
}

#[test]
fn imports_external_defined_name_cache_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test5.cxx::testExternalDefinedNameXLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf144397.xlsx");
    let sheet = SheetId(1);

    assert_cell_value(&model, sheet, "B2", FormulaValue::String("January".into()));
    assert_cell_value(&model, sheet, "B4", FormulaValue::String("March".into()));
    assert_cell_value(
        &model,
        sheet,
        "B6",
        FormulaValue::Error(FormulaErrorValue::NA),
    );
    assert_cell_value(&model, sheet, "B7", FormulaValue::String("June".into()));

    assert!(model.external_references.iter().any(|external| {
        external.sheet_names.iter().any(|sheet| sheet == "Munka1")
            && external.defined_names.iter().any(|name| {
                name.name == "MonthNames" && name.formula_text == "[1]Munka1!$A$2:$A$13"
            })
    }));
    assert!(model.external_cached_cells.iter().any(|cell| {
        cell.sheet_name == "Munka1"
            && cell.reference == "A3"
            && cell.value == FormulaValue::String("February".into())
    }));
}

#[test]
fn imports_external_link_targets_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test5.cxx::testMissingPathExternal and
    // subsequent_export_test6.cxx::testXlStartupExternalXLSX.
    let missing = workbook("LibreOffice/sc/qa/unit/data/xlsx/MissingPathExternal.xlsx");
    assert!(
        missing
            .external_references
            .iter()
            .any(|external| external.target.as_deref() == Some("Tabelle1"))
    );

    let startup = workbook("LibreOffice/sc/qa/unit/data/xlsx/XlStartupExternal.xlsx");
    assert!(
        startup
            .external_references
            .iter()
            .any(|external| external.target.as_deref() == Some("personal.xls"))
    );
}

#[test]
fn imports_hyperlink_formula_and_matrix_cached_values_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test.cxx checks the hyperlink target;
    // the formula model covers the imported formula/cached result at the hyperlink cell.
    let hyperlink = workbook("LibreOffice/sc/qa/unit/data/xlsx/hyperlink_formula.xlsx");
    assert_eq!(formula_text(&hyperlink, SheetId(1), "A2"), "A1");
    assert_cell_value(
        &hyperlink,
        SheetId(1),
        "A2",
        FormulaValue::String("formula".into()),
    );

    let matrix = workbook("LibreOffice/sc/qa/unit/data/xlsx/matrix-multiplication.xlsx");
    let formula = matrix.sheets[0]
        .cells
        .get(&address("G5"))
        .and_then(|record| record.formula.as_ref())
        .expect("G5 array formula");
    assert_eq!(formula.formula_kind, FormulaKind::Array);
    assert_eq!(formula.reference, Some(range("G5", "G6")));
    assert_eq!(formula.formula_text, "MMULT(A1:C2,E1:E3)");
    assert_cell_value(&matrix, SheetId(1), "G5", FormulaValue::Number(49.2));
    assert_cell_value(&matrix, SheetId(1), "G6", FormulaValue::Number(103.6));
}

#[test]
fn imports_excel_2010_function_comparison_cache_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test3.cxx::testFunctionsExcel2010XLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/functions-excel-2010.xlsx");
    let sheet = SheetId(1);

    for row in 3..=80 {
        if row == 45 || row == 79 {
            continue;
        }
        let comparison_cell = format!("D{row}");
        assert_cell_value(&model, sheet, &comparison_cell, FormulaValue::Boolean(true));
    }
}

#[test]
fn imports_ceiling_floor_aggregate_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorXLSX.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/ceiling-floor.xlsx");
    let sheet = sheet_id(&model, "Sheet1");

    assert_eq!(formula_text(&model, sheet, "K1"), "AND(K3:K81)");
    assert_cell_value(&model, sheet, "K1", FormulaValue::Boolean(true));
}

#[test]
fn imports_table_total_formula_cache_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test.cxx::testTdf162963.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf162963_TableWithTotalsEnabled.xlsx");
    let sheet = SheetId(1);

    assert_cell_value(&model, sheet, "A1", FormulaValue::String("Name".into()));
    assert_cell_value(&model, sheet, "B6", FormulaValue::Number(115.0));
    assert_eq!(formula_text(&model, sheet, "B6"), "SUM(myData[Sales])");
}

#[test]
fn imports_defined_name_formula_text_from_xlsx_fixture() {
    // Source: LibreOffice sc/qa/unit/subsequent_export_test3.cxx::testForumMsoEn4145327.
    let model = workbook("LibreOffice/sc/qa/unit/data/xlsx/forum-mso-en4-145327.xlsx");

    assert!(model.defined_names.iter().any(|name| {
        name.formula_text
            == "REPLACE(CELL(\"filename\",!A1),1,FIND(\"]\",CELL(\"filename\",!A1)),\"\")-1"
    }));
}

#[test]
fn imports_lookup_and_sumif_regression_caches_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test3.cxx::testTdf98481 and
    // testTdf115022.
    let lookup = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf98481.xlsx");
    let sheet = SheetId(1);
    for (reference, expected) in [
        ("E2", 4.0),
        ("E3", 0.0),
        ("E4", 3.0),
        ("B5", 4.0),
        ("C5", 0.0),
        ("D5", 3.0),
    ] {
        assert_cell_numeric_value(&lookup, sheet, reference, expected);
    }

    let sumif = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf115022.xlsx");
    assert_cell_numeric_value(&sumif, sheet, "B9", 6.0);
}

#[test]
fn imports_formula_display_regressions_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test3.cxx::testTdf137091,
    // testTdf141495, and testTdf70455.
    let date_fraction = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf137091.xlsx");
    assert_eq!(display_text(&date_fraction, SheetId(1), "C2"), "28/4");

    let addin_date = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf141495.xlsx");
    assert_eq!(display_text(&addin_date, SheetId(1), "L7"), "44926");

    let currency = workbook("LibreOffice/sc/qa/unit/data/xlsx/tdf70455.xlsx");
    assert_eq!(display_text(&currency, SheetId(1), "H8"), "€780.00");
}

#[test]
fn imports_table_total_row_formula_caches_from_xlsx_fixtures() {
    // Source: LibreOffice sc/qa/unit/subsequent_filters_test5.cxx::testTableStyleTest
    // and testFullColumnRefs.
    let table = workbook("LibreOffice/sc/qa/unit/data/xlsx/TableStyleTest.xlsx");
    let sheet = SheetId(1);
    assert_cell_value(&table, sheet, "A10", FormulaValue::String("Total".into()));
    assert_cell_value(&table, sheet, "B10", FormulaValue::Number(3.0));

    let full_column = workbook("LibreOffice/sc/qa/unit/data/xlsx/forum-mso-en4-134670.xlsx");
    let first_sheet = full_column.identity.sheets[0].id;
    assert_cell_value(
        &full_column,
        first_sheet,
        "A1",
        FormulaValue::String("Total # Of Companies".into()),
    );
    assert_cell_value(&full_column, first_sheet, "K2", FormulaValue::Number(1.0));
}

use std::collections::BTreeMap;

use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk_corpus_test_support::corpus_file_path;
use ooxmlsdk_formula::program::{FormulaPrintOptions, FormulaProgram};
use ooxmlsdk_formula::source::{
    FormulaCellAddress, FormulaCompileContext, FormulaSource, FormulaSourceKind,
    FormulaSourcePosition,
};
use ooxmlsdk_formula::{CellAddress, FormulaGrammar, SheetId, WorkbookValueModel};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RoundtripStatus {
    Stable,
    Unsupported,
    Unstable,
}

#[derive(Debug)]
struct RoundtripOutcome {
    status: RoundtripStatus,
    first_print: Option<String>,
    second_print: Option<String>,
}

#[derive(Debug)]
struct FormulaCase<'a> {
    source: &'a str,
    position: FormulaSourcePosition,
    kind: FormulaSourceKind,
}

#[derive(Debug)]
struct RoundtripSummary {
    counts: BTreeMap<RoundtripStatus, usize>,
    samples: Vec<String>,
    total: usize,
}

impl RoundtripSummary {
    fn new() -> Self {
        Self {
            counts: BTreeMap::new(),
            samples: Vec::new(),
            total: 0,
        }
    }

    fn record(&mut self, context: impl Into<String>, outcome: RoundtripOutcome) {
        self.total += 1;
        *self.counts.entry(outcome.status).or_insert(0) += 1;
        if outcome.status != RoundtripStatus::Stable && self.samples.len() < 24 {
            self.samples.push(format!(
                "{}: {:?}, first={:?}, second={:?}",
                context.into(),
                outcome.status,
                outcome.first_print,
                outcome.second_print
            ));
        }
    }

    fn count(&self, status: RoundtripStatus) -> usize {
        self.counts.get(&status).copied().unwrap_or(0)
    }
}

fn formula_context(
    position: FormulaSourcePosition,
    kind: FormulaSourceKind,
) -> FormulaCompileContext {
    FormulaCompileContext {
        grammar: FormulaGrammar::ExcelA1,
        position,
        kind,
    }
}

fn roundtrip_formula(case: FormulaCase<'_>) -> RoundtripOutcome {
    let first = FormulaProgram::from_source(FormulaSource {
        text: case.source,
        context: formula_context(case.position, case.kind),
    })
    .print_formula(&FormulaPrintOptions::default());
    let Some(first_print) = first else {
        return RoundtripOutcome {
            status: RoundtripStatus::Unsupported,
            first_print: None,
            second_print: None,
        };
    };

    let second = FormulaProgram::from_source(FormulaSource {
        text: &first_print,
        context: formula_context(case.position, case.kind),
    })
    .print_formula(&FormulaPrintOptions::default());
    let Some(second_print) = second else {
        return RoundtripOutcome {
            status: RoundtripStatus::Unsupported,
            first_print: Some(first_print),
            second_print: None,
        };
    };

    let status = if first_print == second_print {
        RoundtripStatus::Stable
    } else {
        RoundtripStatus::Unstable
    };
    RoundtripOutcome {
        status,
        first_print: Some(first_print),
        second_print: Some(second_print),
    }
}

fn cell_position(sheet: SheetId, address: CellAddress) -> FormulaSourcePosition {
    FormulaSourcePosition::Cell(FormulaCellAddress {
        sheet,
        cell: address,
    })
}

fn default_cell_position() -> FormulaSourcePosition {
    cell_position(SheetId(1), CellAddress { column: 0, row: 0 })
}

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

fn cell_reference(address: CellAddress) -> String {
    format!("{}{}", column_name(address.column), address.row + 1)
}

fn sweep_workbook_formulas(relative_path: &str) -> RoundtripSummary {
    let model = workbook(relative_path);
    let mut summary = RoundtripSummary::new();

    for name in &model.defined_names {
        let position = name
            .sheet
            .map(FormulaSourcePosition::Sheet)
            .unwrap_or_else(|| FormulaSourcePosition::Sheet(SheetId(1)));
        let outcome = roundtrip_formula(FormulaCase {
            source: name.formula_text.as_ref(),
            position,
            kind: FormulaSourceKind::DefinedName,
        });
        summary.record(
            format!("{relative_path} defined-name {}", name.name),
            outcome,
        );
    }

    for sheet in &model.sheets {
        for (address, record) in &sheet.cells {
            let Some(formula) = &record.formula else {
                continue;
            };
            let outcome = roundtrip_formula(FormulaCase {
                source: formula.formula_text.as_ref(),
                position: cell_position(sheet.id, *address),
                kind: FormulaSourceKind::Cell,
            });
            summary.record(
                format!(
                    "{relative_path} sheet {} {}",
                    sheet.id.0,
                    cell_reference(*address)
                ),
                outcome,
            );
        }
    }

    summary
}

fn sweep_workbooks(paths: &[&str]) -> RoundtripSummary {
    let mut summary = RoundtripSummary::new();
    for path in paths {
        let workbook_summary = sweep_workbook_formulas(path);
        summary.total += workbook_summary.total;
        for (status, count) in workbook_summary.counts {
            *summary.counts.entry(status).or_insert(0) += count;
        }
        for sample in workbook_summary.samples {
            if summary.samples.len() < 24 {
                summary.samples.push(sample);
            }
        }
    }
    summary
}

fn assert_summary(
    summary: RoundtripSummary,
    expected_total: usize,
    expected_stable: usize,
    expected_unsupported: usize,
    expected_unstable: usize,
) {
    let actual_stable = summary.count(RoundtripStatus::Stable);
    let actual_unsupported = summary.count(RoundtripStatus::Unsupported);
    let actual_unstable = summary.count(RoundtripStatus::Unstable);
    let actual = (
        summary.total,
        actual_stable,
        actual_unsupported,
        actual_unstable,
    );
    let expected = (
        expected_total,
        expected_stable,
        expected_unsupported,
        expected_unstable,
    );
    assert_eq!(
        actual, expected,
        "(total, stable, unsupported, unstable) changed; samples: {:#?}",
        summary.samples
    );
}

#[test]
fn apache_poi_parser_renderer_formula_strings_print_stably() {
    // Source: Apache POI FormulaParser / FormulaRenderer tests. This is a formula
    // writeback oracle, not an original-string preservation oracle.
    let formulas = [
        "ABC10",
        "A500000",
        "ABC500000",
        "XFD1048576",
        "ISEVEN(A1)",
        "SUM(A1:B3)",
        "SUM(Sheet1!A1)",
        "SUM(Sheet1!A1:B3)",
        "LOG10(100)",
        "Uses!A1",
        "'Testing 47100'!A1",
        "Defines!NR_To_A1",
        "NR_Global_B2",
        "[1]Uses!$A$1",
        "[1]Defines!NR_To_A1",
        "[1]!NR_Global_B2",
        "SUM(Sheet1:Sheet3!A1)",
        "MAX(Sheet1:Sheet3!A$1)",
        "SUM(Sheet1:Sheet3!A1:B3)",
        "(ABC10 )",
        " INTERCEPT ( \t \r A2 : \nA5 , B2 : B5 ) \t",
        "A1:B1 B1:B2",
        "SUM((B2:B3,C4:C5,D6:D7))",
        "SUM(A:A)",
        "SUM(1:2)",
        "SUM($A:$C)",
        "SUM($1:$4)",
        "SUM(Sheet1!A:A)",
        "SUM('My Sheet'!$A$1:$B$2)",
        "mode({1,2,2,#REF!;FALSE,3,3,2})",
        "{1,2;3,4}",
        "\"  hi  \"",
        "\"a\"\"b\"&C1",
        "-3",
        "--4",
        "+++5",
        "++-6",
        "+ 12",
        "- 13",
        "lookup(A1, A3:A52, B3:B52)",
        "match(A1, A3:A52)",
        "40000/2",
        "Cash_Flow!A1",
        "DA6_LEO_WBS_Number*2",
        "(A1_*_A1+A_1)/A_1_",
        "INDEX(DA6_LEO_WBS_Name,MATCH($A3,DA6_LEO_WBS_Number,0))",
        "SUM(OFFSET(A1,0,0):B2:C3:D4:E5:OFFSET(F6,1,1):G7)",
        "SUM(A1:B2:C3:D4)",
        "IF(A1,,B1)",
        "Table1[]",
        "Table1[Name]",
        "Table1[[#Totals],[col]]",
        "Table1[#All]",
        "Table1[#Data]",
        "Table1[#Headers]",
        "Table1[#This Row]",
        "Table1[@]",
        "Table1[[#Data],[Number]]",
        "Table1[[#All],[Name]:[Number]]",
        "Table1[[#This Row],[col1]]",
        "Table1[ [col1]:[col2] ]",
    ];

    let mut summary = RoundtripSummary::new();
    for formula in formulas {
        let outcome = roundtrip_formula(FormulaCase {
            source: formula,
            position: default_cell_position(),
            kind: FormulaSourceKind::Cell,
        });
        summary.record(formula, outcome);
    }

    assert_summary(summary, formulas.len(), formulas.len(), 0, 0);
}

#[test]
fn apache_poi_existing_xlsx_formula_cells_print_stably() {
    // Source: existing Apache POI fixture coverage in xlsx_import.rs. This widens
    // the printer checks over every imported formula cell and defined name.
    let summary = sweep_workbooks(&[
        "Apache-POI/test-data/spreadsheet/xlookup.xlsx",
        "Apache-POI/test-data/spreadsheet/49872.xlsx",
        "Apache-POI/test-data/spreadsheet/50096.xlsx",
        "Apache-POI/test-data/spreadsheet/55906-MultiSheetRefs.xlsx",
        "Apache-POI/test-data/spreadsheet/59736.xlsx",
        "Apache-POI/test-data/spreadsheet/simple-monthly-budget.xlsx",
        "Apache-POI/test-data/spreadsheet/62834.xlsx",
        "Apache-POI/test-data/spreadsheet/63934.xlsx",
        "Apache-POI/test-data/spreadsheet/61495-test.xlsm",
        "Apache-POI/test-data/spreadsheet/bug60848_sumproduct_unary_minus.xlsx",
        "Apache-POI/test-data/spreadsheet/ref2-56737.xlsx",
        "Apache-POI/test-data/spreadsheet/evaluate_formula_with_structured_table_references.xlsx",
        "Apache-POI/test-data/spreadsheet/FormulaEvalTestData_Copy.xlsx",
        "Apache-POI/test-data/spreadsheet/MatrixFormulaEvalTestData.xlsx",
        "Apache-POI/test-data/spreadsheet/FormulaSheetRange.xlsx",
    ]);

    assert_summary(summary, 1692, 1692, 0, 0);
}

#[test]
fn apache_poi_curated_xlsx_formula_cells_print_stably() {
    // Source: curated Apache POI spreadsheet corpus. This is the Tier 3 sweep:
    // broad enough to expose printer gaps, but still an explicit OOXML allowlist.
    let summary = sweep_workbooks(&[
        "Apache-POI/test-data/spreadsheet/conditional_formatting_with_formula_on_second_sheet.xlsx",
        "Apache-POI/test-data/spreadsheet/formula-eval.xlsx",
        "Apache-POI/test-data/spreadsheet/NewlineInFormulas.xlsx",
        "Apache-POI/test-data/spreadsheet/TestShiftRowSharedFormula.xlsx",
        "Apache-POI/test-data/spreadsheet/VLookupFullColumn.xlsx",
        "Apache-POI/test-data/spreadsheet/shared_formulas.xlsx",
        "Apache-POI/test-data/spreadsheet/testSharedFormulasSetBlank.xlsx",
        "Apache-POI/test-data/spreadsheet/testSharedFormulasRangeSetBlankBug.xlsx",
        "Apache-POI/test-data/spreadsheet/Tables.xlsx",
        "Apache-POI/test-data/spreadsheet/ExcelTables.xlsx",
        "Apache-POI/test-data/spreadsheet/WithTable.xlsx",
        "Apache-POI/test-data/spreadsheet/50755_workday_formula_example.xlsx",
        "Apache-POI/test-data/spreadsheet/DataTableCities.xlsx",
        "Apache-POI/test-data/spreadsheet/chartTitle_withTitleFormula.xlsx",
    ]);

    assert_summary(summary, 576, 576, 0, 0);
}

use std::path::{Path, PathBuf};

use ooxmlsdk_corpus_test_support::{workspace_relative_path, workspace_root};
use ooxmlsdk_formula::{CellAddress, FormulaValue};
use ooxmlsdk_formula_test::fods::{FodsFormulaCase, read_fods_workbook};

#[test]
fn libreoffice_function_fods_corpus_matches_cached_results() {
    // Source: LibreOffice sc/qa/unit/functions_*.cxx recursiveScan(test::pass)
    // over sc/qa/unit/data/functions/**/fods/*.fods. The expected values are
    // the cached formula results stored in the upstream FODS fixtures.
    let root = workspace_root().join("fixtures/LibreOffice/sc/qa/unit/data/functions");
    let files = fods_files(&root);
    assert_eq!(
        files.len(),
        507,
        "LibreOffice function FODS fixture count changed"
    );

    let mut summary = CorpusSummary::default();
    for file in files {
        summary.files += 1;
        let workbook = read_fods_workbook(&file).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", workspace_relative_path(&file))
        });
        let book = workbook.evaluation_book();
        let cases = workbook.formula_cases();
        summary.formulas += cases.len();
        for case in cases {
            match case.evaluate(&book) {
                Some(actual) if formula_values_match(&actual, &case.expected) => {
                    summary.passed += 1;
                }
                Some(actual) => {
                    summary.failed += 1;
                    summary.push_failure(format_failure(&file, &case, Some(&actual)));
                }
                None => {
                    summary.unsupported += 1;
                    summary.push_failure(format_failure(&file, &case, None));
                }
            }
        }
    }

    if summary.failed != 0 || summary.unsupported != 0 {
        panic!("{summary}");
    }
}

#[derive(Default)]
struct CorpusSummary {
    files: usize,
    formulas: usize,
    passed: usize,
    failed: usize,
    unsupported: usize,
    samples: Vec<String>,
}

impl CorpusSummary {
    fn push_failure(&mut self, failure: String) {
        if self.samples.len() < 50 {
            self.samples.push(failure);
        }
    }
}

impl std::fmt::Display for CorpusSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "LibreOffice FODS formula corpus: files={}, formulas={}, passed={}, failed={}, unsupported={}",
            self.files, self.formulas, self.passed, self.failed, self.unsupported
        )?;
        for sample in &self.samples {
            writeln!(f, "{sample}")?;
        }
        Ok(())
    }
}

fn fods_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_fods_files(root, &mut files);
    files.sort();
    files
}

fn collect_fods_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap_or_else(|err| {
        panic!(
            "failed to read fixture directory {}: {err}",
            workspace_relative_path(path)
        )
    }) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_fods_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("fods")
            && path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                == Some("fods")
        {
            files.push(path);
        }
    }
}

fn formula_values_match(actual: &FormulaValue<'_>, expected: &FormulaValue<'_>) -> bool {
    match (actual, expected) {
        (FormulaValue::Number(actual), FormulaValue::Number(expected)) => {
            let tolerance = 1e-9_f64.max(expected.abs() * 1e-10);
            (actual - expected).abs() <= tolerance
        }
        (FormulaValue::String(actual), FormulaValue::String(expected)) => actual == expected,
        (FormulaValue::Boolean(actual), FormulaValue::Boolean(expected)) => actual == expected,
        (FormulaValue::Blank, FormulaValue::Blank) => true,
        (FormulaValue::Error(_), FormulaValue::Error(_)) => true,
        _ => false,
    }
}

fn format_failure(
    file: &Path,
    case: &FodsFormulaCase,
    actual: Option<&FormulaValue<'_>>,
) -> String {
    format!(
        "{} {}!{} formula={} expected={:?} actual={:?}",
        workspace_relative_path(file),
        case.sheet_name,
        a1(case.address),
        case.formula,
        case.expected,
        actual
    )
}

fn a1(address: CellAddress) -> String {
    format!("{}{}", column_name(address.column), address.row + 1)
}

fn column_name(mut column: u32) -> String {
    column += 1;
    let mut chars = Vec::new();
    while column > 0 {
        column -= 1;
        chars.push(char::from_u32('A' as u32 + column % 26).unwrap_or('A'));
        column /= 26;
    }
    chars.into_iter().rev().collect()
}

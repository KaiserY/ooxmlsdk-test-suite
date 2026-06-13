use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

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
                    summary.push_mismatch(file_group(&root, &file), formula_key(&case.formula));
                    summary.push_failure(format_failure(&file, &case, Some(&actual)));
                }
                None => {
                    summary.unsupported += 1;
                    let key = formula_key(&case.formula);
                    let failure = format_failure(&file, &case, None);
                    summary.push_unsupported(file_group(&root, &file), key, failure.clone());
                    summary.push_failure(failure);
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
    unsupported_by_formula: BTreeMap<String, usize>,
    unsupported_sample_by_formula: BTreeMap<String, String>,
    mismatch_by_formula: BTreeMap<String, usize>,
    unsupported_by_group: BTreeMap<String, usize>,
    mismatch_by_group: BTreeMap<String, usize>,
}

impl CorpusSummary {
    fn push_failure(&mut self, failure: String) {
        if self.samples.len() < 50 {
            self.samples.push(failure);
        }
    }

    fn push_unsupported(&mut self, group: String, formula: String, sample: String) {
        *self.unsupported_by_group.entry(group).or_default() += 1;
        *self
            .unsupported_by_formula
            .entry(formula.clone())
            .or_default() += 1;
        self.unsupported_sample_by_formula
            .entry(formula)
            .or_insert(sample);
    }

    fn push_mismatch(&mut self, group: String, formula: String) {
        *self.mismatch_by_group.entry(group).or_default() += 1;
        *self.mismatch_by_formula.entry(formula).or_default() += 1;
    }
}

impl std::fmt::Display for CorpusSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "LibreOffice FODS formula corpus: files={}, formulas={}, passed={}, failed={}, unsupported={}",
            self.files, self.formulas, self.passed, self.failed, self.unsupported
        )?;
        write_top_counts(f, "unsupported by formula", &self.unsupported_by_formula)?;
        write_top_samples(
            f,
            "unsupported sample by formula",
            &self.unsupported_by_formula,
            &self.unsupported_sample_by_formula,
        )?;
        write_top_counts(f, "mismatch by formula", &self.mismatch_by_formula)?;
        write_top_counts(f, "unsupported by group", &self.unsupported_by_group)?;
        write_top_counts(f, "mismatch by group", &self.mismatch_by_group)?;
        for sample in &self.samples {
            writeln!(f, "{sample}")?;
        }
        Ok(())
    }
}

fn write_top_samples(
    f: &mut std::fmt::Formatter<'_>,
    label: &str,
    counts: &BTreeMap<String, usize>,
    samples: &BTreeMap<String, String>,
) -> std::fmt::Result {
    let mut items = counts.iter().collect::<Vec<_>>();
    items.sort_by(|(left_key, left_count), (right_key, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_key.cmp(right_key))
    });
    writeln!(f, "{label}:")?;
    for (key, _) in items.into_iter().take(10) {
        if let Some(sample) = samples.get(key) {
            writeln!(f, "  {key}: {sample}")?;
        }
    }
    Ok(())
}

fn write_top_counts(
    f: &mut std::fmt::Formatter<'_>,
    label: &str,
    counts: &BTreeMap<String, usize>,
) -> std::fmt::Result {
    let mut items = counts.iter().collect::<Vec<_>>();
    items.sort_by(|(left_key, left_count), (right_key, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_key.cmp(right_key))
    });
    writeln!(f, "{label}:")?;
    for (key, count) in items.into_iter().take(30) {
        writeln!(f, "  {count:>6} {key}")?;
    }
    Ok(())
}

fn file_group(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .ok()
        .and_then(|path| path.iter().next())
        .and_then(|part| part.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn formula_key(formula: &str) -> String {
    let text = formula
        .trim()
        .strip_prefix("of:=")
        .or_else(|| formula.trim().strip_prefix('='))
        .unwrap_or(formula.trim());
    let text = text.strip_prefix("ORG.LIBREOFFICE.").unwrap_or(text);
    let mut name = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
            name.push(ch.to_ascii_uppercase());
        } else if ch == '(' && !name.is_empty() {
            return name;
        } else if !name.is_empty() {
            break;
        }
    }
    if name.is_empty() {
        "expression".to_string()
    } else {
        name
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

use std::{
    borrow::Cow,
    cmp::Reverse,
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

use ooxmlsdk_corpus_test_support::{workspace_relative_path, workspace_root};
use ooxmlsdk_formula::{
    CellAddress, FormulaEvaluationBook, FormulaGrammar, FormulaSearchType, FormulaValue,
    normalize_formula_text,
};
use ooxmlsdk_formula_test::fods::{FodsFormulaCase, read_fods_workbook};

#[test]
fn libreoffice_function_fods_corpus_matches_functions_test_load() {
    // Source: LibreOffice sc/qa/unit/functions_*.cxx recursiveScan(test::pass)
    // over sc/qa/unit/data/functions/**/fods/*.fods. Function-test workbooks
    // follow FunctionsTest::load: hard-recalculate, assert Sheet1.B3, then
    // scan the Correct column only to diagnose failures. Other sheets compare
    // against upstream cached results.
    let root = workspace_root().join("fixtures/LibreOffice/sc/qa/unit/data/functions");
    let files = fods_files(&root);
    assert_eq!(
        files.len(),
        507,
        "LibreOffice function FODS fixture count changed"
    );

    let mut summary = CorpusSummary::default();
    for file in files {
        let start = Instant::now();
        summary.files += 1;
        let workbook = read_fods_workbook(&file).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", workspace_relative_path(&file))
        });
        assert_eq!(
            workbook.formula_search_type,
            expected_lo_formula_search_type(&file),
            "FODS calculation-settings search mode drifted from LibreOffice XML import semantics for {}",
            workspace_relative_path(&file)
        );
        summary.push_reader_search_type(workbook.formula_search_type);
        summary.raw_formula_cells += workbook.cached_formula_cases().len();
        let raw_book = workbook.evaluation_book();
        let hard_recalc_book = workbook.hard_recalc_book();
        let (book, cases) =
            if let Some(cases) = workbook.libreoffice_function_test_cases(&hard_recalc_book) {
                (hard_recalc_book, cases)
            } else {
                (raw_book, workbook.formula_cases())
            };
        summary.assertions += cases.len();
        for case in cases {
            match case.evaluate(&book) {
                Some(actual)
                    if formula_values_match(&actual, &case.expected, &case.formula)
                        || function_test_row_values_match(&case, &actual, &book) =>
                {
                    summary.passed += 1;
                }
                Some(actual) => {
                    summary.failed += 1;
                    let key = formula_key(&case.formula);
                    let target_key = target_formula_key(&case, &book);
                    let failure = format_failure(&file, &case, Some(&actual), &book);
                    summary.push_mismatch(
                        file_group(&root, &file),
                        key,
                        target_key,
                        failure.clone(),
                    );
                    summary.push_failure(failure);
                }
                None => {
                    summary.unsupported += 1;
                    let key = formula_key(&case.formula);
                    let failure = format_failure(&file, &case, None, &book);
                    summary.push_unsupported(file_group(&root, &file), key, failure.clone());
                    summary.push_failure(failure);
                }
            }
        }
        summary.push_file_time(workspace_relative_path(&file), start.elapsed());
    }

    if summary.failed != 0 || summary.unsupported != 0 {
        panic!("{summary}");
    }
}

#[test]
fn libreoffice_fods_reader_search_settings_match_raw_xml() {
    // Source: LibreOffice ScXMLCalculationSettingsContext initializes the XML
    // calculation settings context as Regex, then applies use-regular-expressions
    // and use-wildcards attributes. Without a calculation-settings element, the
    // document options default to Wildcard.
    let root = workspace_root().join("fixtures/LibreOffice/sc/qa/unit/data/functions");
    for file in fods_files(&root) {
        let workbook = read_fods_workbook(&file).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", workspace_relative_path(&file))
        });
        assert_eq!(
            workbook.formula_search_type,
            expected_lo_formula_search_type(&file),
            "{}",
            workspace_relative_path(&file)
        );
    }
}

#[derive(Default)]
struct CorpusSummary {
    files: usize,
    raw_formula_cells: usize,
    assertions: usize,
    passed: usize,
    failed: usize,
    unsupported: usize,
    samples: Vec<String>,
    unsupported_by_formula: BTreeMap<String, usize>,
    unsupported_sample_by_formula: BTreeMap<String, String>,
    mismatch_sample_by_formula: BTreeMap<String, String>,
    mismatch_by_formula: BTreeMap<String, usize>,
    mismatch_by_target_formula: BTreeMap<String, usize>,
    mismatch_sample_by_target_formula: BTreeMap<String, String>,
    unsupported_by_group: BTreeMap<String, usize>,
    mismatch_by_group: BTreeMap<String, usize>,
    reader_search_types: BTreeMap<String, usize>,
    slow_files: Vec<(String, u128)>,
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

    fn push_mismatch(
        &mut self,
        group: String,
        formula: String,
        target_formula: String,
        sample: String,
    ) {
        *self.mismatch_by_group.entry(group).or_default() += 1;
        *self.mismatch_by_formula.entry(formula.clone()).or_default() += 1;
        self.mismatch_sample_by_formula
            .entry(formula)
            .or_insert_with(|| sample.clone());
        *self
            .mismatch_by_target_formula
            .entry(target_formula.clone())
            .or_default() += 1;
        self.mismatch_sample_by_target_formula
            .entry(target_formula)
            .or_insert(sample);
    }

    fn push_file_time(&mut self, file: String, elapsed: std::time::Duration) {
        self.slow_files.push((file, elapsed.as_millis()));
    }

    fn push_reader_search_type(&mut self, search_type: FormulaSearchType) {
        *self
            .reader_search_types
            .entry(format!("{search_type:?}"))
            .or_default() += 1;
    }
}

impl std::fmt::Display for CorpusSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "LibreOffice FODS formula corpus: files={}, raw_formula_cells={}, assertions={}, passed={}, failed={}, unsupported={}",
            self.files,
            self.raw_formula_cells,
            self.assertions,
            self.passed,
            self.failed,
            self.unsupported
        )?;
        write_top_counts(f, "unsupported by formula", &self.unsupported_by_formula)?;
        write_top_samples(
            f,
            "unsupported sample by formula",
            &self.unsupported_by_formula,
            &self.unsupported_sample_by_formula,
        )?;
        write_top_counts(f, "mismatch by formula", &self.mismatch_by_formula)?;
        write_top_samples(
            f,
            "mismatch sample by formula",
            &self.mismatch_by_formula,
            &self.mismatch_sample_by_formula,
        )?;
        write_top_counts(
            f,
            "mismatch by target formula",
            &self.mismatch_by_target_formula,
        )?;
        write_top_samples(
            f,
            "mismatch sample by target formula",
            &self.mismatch_by_target_formula,
            &self.mismatch_sample_by_target_formula,
        )?;
        let mut slow_files = self.slow_files.clone();
        slow_files.sort_by_key(|(_, elapsed_ms)| Reverse(*elapsed_ms));
        writeln!(f, "slow files:")?;
        for (file, elapsed_ms) in slow_files.into_iter().take(10) {
            writeln!(f, "{elapsed_ms:>8} ms {file}")?;
        }
        write_top_counts(f, "unsupported by group", &self.unsupported_by_group)?;
        write_top_counts(f, "mismatch by group", &self.mismatch_by_group)?;
        write_top_counts(f, "reader formula search type", &self.reader_search_types)?;
        for sample in &self.samples {
            writeln!(f, "{sample}")?;
        }
        Ok(())
    }
}

fn expected_lo_formula_search_type(file: &Path) -> FormulaSearchType {
    let text = std::fs::read_to_string(file)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", workspace_relative_path(file)));
    let Some(settings) = first_start_tag(&text, "table:calculation-settings") else {
        return FormulaSearchType::Wildcard;
    };
    let mut search_type = FormulaSearchType::Regex;
    if attr_bool(settings, "table:use-regular-expressions") == Some(false)
        && search_type == FormulaSearchType::Regex
    {
        search_type = FormulaSearchType::Normal;
    }
    if attr_bool(settings, "table:use-wildcards") == Some(true) {
        search_type = FormulaSearchType::Wildcard;
    }
    search_type
}

fn first_start_tag<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("<{name}"))?;
    let rest = &text[start..];
    let end = rest.find('>')?;
    Some(&rest[..=end])
}

fn attr_bool(tag: &str, name: &str) -> Option<bool> {
    let value = attr_value(tag, name)?;
    match value {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn attr_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
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

fn target_formula_key(case: &FodsFormulaCase, book: &FormulaEvaluationBook<'static>) -> String {
    let key = formula_key(&case.formula);
    if !matches!(key.as_str(), "expression" | "ISERROR" | "ISNA" | "ISERR") {
        return key;
    }
    let function_string_address = CellAddress {
        column: 3,
        row: case.address.row,
    };
    match book.cell_value(case.sheet, function_string_address) {
        FormulaValue::String(text) if !text.is_empty() => formula_key(&text),
        _ => key,
    }
}

fn function_test_row_values_match(
    case: &FodsFormulaCase,
    actual: &FormulaValue<'_>,
    book: &FormulaEvaluationBook<'static>,
) -> bool {
    if !value_gets_as_true(&case.expected) {
        return false;
    }
    if !matches!(actual, FormulaValue::Boolean(false)) {
        return false;
    }
    let expected = book.cell_value(
        case.sheet,
        CellAddress {
            column: 1,
            row: case.address.row,
        },
    );
    let result = book.cell_value(
        case.sheet,
        CellAddress {
            column: 0,
            row: case.address.row,
        },
    );
    formula_values_match(
        &visible_cell_value(&result),
        &visible_cell_value(&expected),
        &case.formula,
    )
}

fn visible_cell_value<'a>(value: &'a FormulaValue<'a>) -> FormulaValue<'a> {
    match value {
        FormulaValue::Matrix(rows) => rows
            .first()
            .and_then(|row| row.first())
            .cloned()
            .unwrap_or(FormulaValue::Blank),
        value => value.clone(),
    }
}

fn value_gets_as_true(value: &FormulaValue<'_>) -> bool {
    match value {
        FormulaValue::Boolean(value) => *value,
        FormulaValue::Number(value) => (*value - 1.0).abs() <= 1e-14,
        _ => false,
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

fn formula_values_match(
    actual: &FormulaValue<'_>,
    expected: &FormulaValue<'_>,
    formula: &str,
) -> bool {
    match (actual, expected) {
        (FormulaValue::Number(actual), FormulaValue::Number(expected)) => {
            let tolerance = 1e-9_f64.max(expected.abs() * 1e-10);
            (actual - expected).abs() <= tolerance
        }
        (FormulaValue::Number(actual), FormulaValue::Boolean(expected))
        | (FormulaValue::Boolean(expected), FormulaValue::Number(actual)) => {
            (*actual != 0.0) == *expected
        }
        (FormulaValue::String(actual), FormulaValue::String(expected)) => {
            let actual = comparable_text(actual);
            let expected = comparable_text(expected);
            actual == expected
                || (formula_key(formula).starts_with("IM")
                    && normalize_fods_numeric_text(&actual)
                        == normalize_fods_numeric_text(&expected))
        }
        (FormulaValue::Boolean(actual), FormulaValue::Boolean(expected)) => actual == expected,
        (FormulaValue::Blank, FormulaValue::Blank) => true,
        (FormulaValue::String(actual), FormulaValue::Blank)
        | (FormulaValue::Blank, FormulaValue::String(actual)) => comparable_text(actual).is_empty(),
        (FormulaValue::Error(_), FormulaValue::Error(_)) => true,
        _ => false,
    }
}

fn comparable_text(value: &str) -> String {
    normalize_fods_cached_text(value)
        .chars()
        .filter(|ch| !ch.is_control() && !is_unicode_noncharacter(*ch))
        .collect()
}

fn normalize_fods_numeric_text(value: &str) -> Cow<'_, str> {
    if !value.chars().all(|ch| {
        ch.is_ascii_digit()
            || matches!(
                ch,
                '+' | '-' | ',' | '.' | 'e' | 'E' | 'i' | 'j' | 'I' | 'J' | ' '
            )
    }) {
        return Cow::Borrowed(value);
    }
    let mut output = value.replace(',', ".");
    output = normalize_exponent_zeros(&output);
    if output == value {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(output)
    }
}

fn normalize_exponent_zeros(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        output.push(ch);
        if ch != 'e' && ch != 'E' {
            continue;
        }
        if let Some(sign @ ('+' | '-')) = chars.peek().copied() {
            output.push(sign);
            chars.next();
        }
        let mut stripped_zero = false;
        while matches!(chars.peek(), Some('0')) {
            stripped_zero = true;
            chars.next();
        }
        if stripped_zero && !matches!(chars.peek(), Some('0'..='9')) {
            output.push('0');
        }
    }
    output
}

fn normalize_fods_cached_text(value: &str) -> Cow<'_, str> {
    let Some(bytes) = value.chars().map(latin1_byte).collect::<Option<Vec<_>>>() else {
        return Cow::Borrowed(value);
    };
    match String::from_utf8(bytes) {
        Ok(decoded) if decoded != value => Cow::Owned(decoded),
        _ => Cow::Borrowed(value),
    }
}

fn latin1_byte(ch: char) -> Option<u8> {
    match ch {
        '\u{20ac}' => Some(0x80),
        '\u{201a}' => Some(0x82),
        '\u{0192}' => Some(0x83),
        '\u{201e}' => Some(0x84),
        '\u{2026}' => Some(0x85),
        '\u{2020}' => Some(0x86),
        '\u{2021}' => Some(0x87),
        '\u{02c6}' => Some(0x88),
        '\u{2030}' => Some(0x89),
        '\u{0160}' => Some(0x8a),
        '\u{2039}' => Some(0x8b),
        '\u{0152}' => Some(0x8c),
        '\u{017d}' => Some(0x8e),
        '\u{2018}' => Some(0x91),
        '\u{2019}' => Some(0x92),
        '\u{201c}' => Some(0x93),
        '\u{201d}' => Some(0x94),
        '\u{2022}' => Some(0x95),
        '\u{2013}' => Some(0x96),
        '\u{2014}' => Some(0x97),
        '\u{02dc}' => Some(0x98),
        '\u{2122}' => Some(0x99),
        '\u{0161}' => Some(0x9a),
        '\u{203a}' => Some(0x9b),
        '\u{0153}' => Some(0x9c),
        '\u{017e}' => Some(0x9e),
        '\u{0178}' => Some(0x9f),
        _ => (ch as u32 <= u8::MAX as u32).then_some(ch as u8),
    }
}

fn is_unicode_noncharacter(ch: char) -> bool {
    let code = ch as u32;
    (0xfdd0..=0xfdef).contains(&code) || code & 0xfffe == 0xfffe
}

fn format_failure(
    file: &Path,
    case: &FodsFormulaCase,
    actual: Option<&FormulaValue<'_>>,
    book: &FormulaEvaluationBook<'static>,
) -> String {
    format!(
        "{} {}!{} formula={} expected={:?} actual={:?}{}",
        workspace_relative_path(file),
        case.sheet_name,
        a1(case.address),
        case.formula,
        case.expected,
        actual,
        failure_diagnostics(case, book)
    )
}

fn failure_diagnostics(case: &FodsFormulaCase, book: &FormulaEvaluationBook<'static>) -> String {
    let mut parts = Vec::new();
    let row = case.address.row;
    let row_values = (0..12)
        .map(|column| {
            let address = CellAddress { column, row };
            format!("{}={:?}", a1(address), book.cell_value(case.sheet, address))
        })
        .collect::<Vec<_>>()
        .join(", ");
    parts.push(format!(" row[{row_values}]"));

    if let Some(range) = case.array_reference {
        let target_values = case
            .array_expected
            .iter()
            .take(16)
            .map(|(address, expected)| {
                format!(
                    "{} expected={:?} actual={:?}",
                    a1(*address),
                    expected,
                    book.cell_value(case.sheet, *address)
                )
            })
            .collect::<Vec<_>>();
        parts.push(format!(
            " array_target[{}:{} => {}]",
            a1(range.start),
            a1(range.end),
            target_values.join(", ")
        ));
    }

    let refs = referenced_cells(&case.formula)
        .into_iter()
        .filter(|address| address.row < 1_048_576 && address.column < 16_384)
        .take(16)
        .map(|address| format!("{}={:?}", a1(address), book.cell_value(case.sheet, address)))
        .collect::<Vec<_>>();
    if !refs.is_empty() {
        parts.push(format!(" refs[{}]", refs.join(", ")));
    }

    if formula_key(&case.formula) == "AND" {
        let bad_and_cells = (7..=94)
            .filter_map(|row| {
                let address = CellAddress { column: 1, row };
                let value = book.cell_value(case.sheet, address);
                (!matches!(value, FormulaValue::Boolean(true))).then(|| {
                    let formula_text = book
                        .cell_value(case.sheet, CellAddress { column: 3, row })
                        .clone();
                    let cell_formula = book.formula_text(case.sheet, address).unwrap_or_default();
                    let nested_bad =
                        referenced_formula_bad_cells(book, case.sheet, &cell_formula, 4);
                    format!(
                        "{}={:?} formula={} A={:?} C={:?} D={:?} nested_bad=[{}]",
                        a1(address),
                        value,
                        cell_formula,
                        book.cell_value(case.sheet, CellAddress { column: 0, row }),
                        book.cell_value(case.sheet, CellAddress { column: 2, row }),
                        formula_text,
                        nested_bad.join(", ")
                    )
                })
            })
            .take(8)
            .collect::<Vec<_>>();
        if !bad_and_cells.is_empty() {
            parts.push(format!(" and_range_bad[{}]", bad_and_cells.join(", ")));
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!(" diagnostics:{}", parts.join(""))
    }
}

fn referenced_formula_bad_cells(
    book: &FormulaEvaluationBook<'static>,
    current_sheet: ooxmlsdk_formula::SheetId,
    formula: &str,
    limit: usize,
) -> Vec<String> {
    let Some(start) = formula.find('(') else {
        return Vec::new();
    };
    let Some(end) = formula.rfind(')') else {
        return Vec::new();
    };
    let reference = formula[start + 1..end].trim();
    let (sheet, range) = if let Some((sheet_name, range)) = reference.split_once('!') {
        (
            book.sheet_id_by_name(sheet_name).unwrap_or(current_sheet),
            range,
        )
    } else {
        (current_sheet, reference)
    };
    let Some((start, end)) = range.split_once(':') else {
        return Vec::new();
    };
    let Ok(start) = CellAddress::parse_a1(start) else {
        return Vec::new();
    };
    let Ok(end) = CellAddress::parse_a1(end) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for row in start.row.min(end.row)..=start.row.max(end.row) {
        for column in start.column.min(end.column)..=start.column.max(end.column) {
            let address = CellAddress { column, row };
            let value = book.cell_value(sheet, address);
            if !matches!(value, FormulaValue::Boolean(true)) {
                values.push(format!("{}={:?}", a1(address), value));
                if values.len() >= limit {
                    return values;
                }
            }
        }
    }
    values
}

fn referenced_cells(formula: &str) -> Vec<CellAddress> {
    let normalized = normalize_formula_text(formula, FormulaGrammar::OpenFormula);
    let mut refs = Vec::new();
    let mut token = String::new();
    for ch in normalized.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '$' | '!' | ':' | '.') {
            token.push(ch);
            continue;
        }
        push_referenced_cells_from_token(&token, &mut refs);
        token.clear();
    }
    refs.sort_by_key(|address| (address.row, address.column));
    refs.dedup();
    refs
}

fn push_referenced_cells_from_token(token: &str, refs: &mut Vec<CellAddress>) {
    let token = token
        .rsplit_once('!')
        .map(|(_, reference)| reference)
        .unwrap_or(token)
        .trim_matches('$');
    if token.is_empty() {
        return;
    }
    let (start, end) = token.split_once(':').unwrap_or((token, token));
    if let Some(address) = parse_debug_cell_address(start) {
        refs.push(address);
    }
    if end != start
        && let Some(address) = parse_debug_cell_address(end)
    {
        refs.push(address);
    }
}

fn parse_debug_cell_address(text: &str) -> Option<CellAddress> {
    let clean = text.replace('$', "");
    if clean.is_empty() || !clean.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    CellAddress::parse_a1(&clean).ok()
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

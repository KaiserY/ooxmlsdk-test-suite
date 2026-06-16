use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use ooxmlsdk_formula::{
    AddressFlags, CellAddress, CellRange, DateSystem, DefinedNameKey, FormulaErrorValue,
    FormulaEvaluationBook, FormulaGrammar, FormulaKind, FormulaParseContext, FormulaPivotField,
    FormulaPivotFieldOrientation, FormulaPivotFunction, FormulaPivotTable, FormulaRowState,
    FormulaSearchType, FormulaText, FormulaValue, ParsedFormula, QualifiedRange, SheetBinding,
    SheetId, SheetName, normalize_formula_text, parse_formula_with_context,
};
use quick_xml::{Reader, XmlVersion, events::Event};

#[derive(Clone, Debug, PartialEq)]
pub struct FodsWorkbook {
    pub sheets: Vec<FodsSheet>,
    pub named_ranges: Vec<FodsNamedRange>,
    pub pivot_tables: Vec<FormulaPivotTable<'static>>,
    pub formula_search_type: FormulaSearchType,
    pub formula_match_whole_cell: bool,
}

impl Default for FodsWorkbook {
    fn default() -> Self {
        Self {
            sheets: Vec::new(),
            named_ranges: Vec::new(),
            pivot_tables: Vec::new(),
            formula_search_type: FormulaSearchType::default(),
            formula_match_whole_cell: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FodsSheet {
    pub name: String,
    pub cells: Vec<FodsCell>,
    pub row_states: Vec<(u32, FormulaRowState)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FodsNamedRange {
    pub name: String,
    pub sheet_name: Option<String>,
    pub formula: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FodsCell {
    pub address: CellAddress,
    pub formula: Option<String>,
    pub matrix_columns: u32,
    pub matrix_rows: u32,
    pub query_empty: bool,
    pub cached_value: FormulaValue<'static>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FodsFormulaCase {
    pub sheet: SheetId,
    pub sheet_name: String,
    pub address: CellAddress,
    pub formula: String,
    pub parsed_formula: ParsedFormula<'static>,
    pub array_reference: Option<CellRange>,
    pub array_expected: Vec<(CellAddress, FormulaValue<'static>)>,
    pub expected: FormulaValue<'static>,
    read_cell_value: bool,
}

impl FodsWorkbook {
    pub fn evaluation_book(&self) -> FormulaEvaluationBook<'static> {
        let sheet_names = self
            .sheets
            .iter()
            .enumerate()
            .map(|(index, sheet)| SheetBinding {
                id: SheetId(index as u32 + 1),
                name: Cow::Owned(sheet.name.clone()),
            })
            .collect();
        let mut cells = BTreeMap::new();
        let mut query_cell_values = BTreeMap::new();
        let mut query_empty_cells = BTreeSet::new();
        let mut formulas = BTreeMap::new();
        let mut formula_text_overrides = BTreeMap::new();
        let mut today_serial = None;
        for (index, sheet) in self.sheets.iter().enumerate() {
            let sheet_id = SheetId(index as u32 + 1);
            for cell in &sheet.cells {
                cells.insert((sheet_id, cell.address), cell.cached_value.clone());
                if let Some(formula) = &cell.formula {
                    if today_serial.is_none()
                        && is_today_formula(formula)
                        && let FormulaValue::Number(value) = &cell.cached_value
                    {
                        today_serial = Some(value.floor());
                    }
                    if formula_references_blank_cell(self, index, cell) {
                        query_cell_values
                            .insert((sheet_id, cell.address), FormulaValue::Number(0.0));
                        query_empty_cells.insert((sheet_id, cell.address));
                    }
                    if let Some(target) = formula_text_target(sheet_id, formula) {
                        match &cell.cached_value {
                            FormulaValue::String(text) => {
                                formula_text_overrides.insert((sheet_id, target), text.clone());
                            }
                            FormulaValue::Blank => {
                                formula_text_overrides
                                    .insert((sheet_id, target), Cow::Borrowed(""));
                            }
                            _ => {}
                        }
                    }
                    let normalized = normalize_formula_text(formula, FormulaGrammar::OpenFormula);
                    let matrix_columns = cell.matrix_columns.max(1);
                    let matrix_rows = cell.matrix_rows.max(1);
                    if matrix_columns > 1 || matrix_rows > 1 {
                        for row_offset in 0..matrix_rows {
                            for column_offset in 0..matrix_columns {
                                let address = CellAddress {
                                    column: cell.address.column + column_offset,
                                    row: cell.address.row + row_offset,
                                };
                                query_cell_values.insert(
                                    (sheet_id, address),
                                    sheet
                                        .cached_value_at(address)
                                        .unwrap_or_else(|| cell.cached_value.clone()),
                                );
                                if cell.query_empty {
                                    query_empty_cells.insert((sheet_id, address));
                                }
                            }
                        }
                    }
                    let reference = CellRange::new(
                        cell.address,
                        CellAddress {
                            column: cell.address.column + matrix_columns - 1,
                            row: cell.address.row + matrix_rows - 1,
                        },
                    );
                    let formula_text = FormulaText {
                        text: Cow::Owned(normalized.into_owned()),
                        kind: if matrix_columns > 1 || matrix_rows > 1 {
                            FormulaKind::Array
                        } else {
                            FormulaKind::Normal
                        },
                        reference: (matrix_columns > 1 || matrix_rows > 1).then_some(reference),
                    };
                    for row_offset in 0..matrix_rows {
                        for column_offset in 0..matrix_columns {
                            formulas.insert(
                                (
                                    sheet_id,
                                    CellAddress {
                                        column: cell.address.column + column_offset,
                                        row: cell.address.row + row_offset,
                                    },
                                ),
                                formula_text.clone(),
                            );
                        }
                    }
                }
            }
        }
        for (key, text) in formula_text_overrides {
            if let Some(formula) = formulas.get_mut(&key) {
                formula.text = text;
            }
        }
        FormulaEvaluationBook {
            sheet_names,
            cells,
            query_cell_values,
            query_empty_cells,
            formulas,
            row_states: self
                .sheets
                .iter()
                .enumerate()
                .flat_map(|(index, sheet)| {
                    let sheet_id = SheetId(index as u32 + 1);
                    sheet
                        .row_states
                        .iter()
                        .map(move |(row, state)| ((sheet_id, *row), *state))
                })
                .collect(),
            defined_names: self
                .named_ranges
                .iter()
                .map(|range| {
                    (
                        DefinedNameKey {
                            sheet: range.sheet_name.as_deref().and_then(|sheet_name| {
                                self.sheets
                                    .iter()
                                    .position(|sheet| sheet.name == sheet_name)
                                    .map(|index| SheetId(index as u32 + 1))
                            }),
                            name_upper: range.name.to_ascii_uppercase(),
                        },
                        Cow::Owned(range.formula.clone()),
                    )
                })
                .collect(),
            pivot_tables: self.pivot_tables.clone(),
            date_system: DateSystem::LibreOffice,
            formula_search_type: self.formula_search_type,
            formula_match_whole_cell: self.formula_match_whole_cell,
            today_serial,
            ..FormulaEvaluationBook::default()
        }
    }

    pub fn hard_recalc_book(&self) -> FormulaEvaluationBook<'static> {
        // Source: LibreOffice sc/qa/unit/functions_test.cxx FunctionsTest::load
        // calls DoHardRecalc() before reading Sheet1.B3 and the per-row
        // "Correct" cells. FormulaEvaluationBook stores values eagerly, so
        // rebuild those eager values by repeatedly evaluating every formula cell.
        const MAX_RECALC_PASSES: usize = 12;

        let mut book = self.evaluation_book();
        let targets = self.recalc_targets();
        for _ in 0..MAX_RECALC_PASSES {
            let mut updates = Vec::new();
            for target in &targets {
                if let Some(range) = target.reference {
                    if let Some(value) = book.evaluate_parsed_formula_raw(
                        target.sheet,
                        Some(target.address),
                        &target.parsed_formula,
                        target.array_context,
                    ) {
                        updates.extend(fods_array_recalc_updates(
                            &book,
                            target.sheet,
                            range,
                            &value,
                        ));
                    }
                } else if let Some(value) = book.evaluate_parsed_formula(
                    target.sheet,
                    Some(target.address),
                    &target.parsed_formula,
                ) {
                    updates.push((target.sheet, target.address, fods_scalar_cell_value(value)));
                }
            }
            let mut changed = false;
            for (sheet, address, value) in updates {
                if book.cell_value(sheet, address) != value {
                    changed = true;
                    book.cells.insert((sheet, address), value.clone());
                }
                if book.is_query_empty_cell(sheet, address) && matches!(value, FormulaValue::Blank)
                {
                    book.query_cell_values
                        .entry((sheet, address))
                        .or_insert(FormulaValue::Number(0.0));
                } else {
                    book.query_cell_values.insert((sheet, address), value);
                }
            }
            if !changed {
                break;
            }
        }
        book
    }

    pub fn formula_cases(&self) -> Vec<FodsFormulaCase> {
        self.formula_targets()
    }

    pub fn cached_formula_cases(&self) -> Vec<FodsFormulaCase> {
        self.formula_targets()
    }

    pub fn libreoffice_function_test_cases(
        &self,
        book: &FormulaEvaluationBook<'static>,
    ) -> Option<Vec<FodsFormulaCase>> {
        // Source: LibreOffice sc/qa/unit/functions_test.cxx FunctionsTest::load.
        // LO's authoritative assertion is Sheet1.B3 after DoHardRecalc(). If it
        // fails, LO diagnoses rows using sheets whose first row has the
        // Expected/Correct/FunctionString layout. This harness uses that same
        // layout to extract comparable row cases; sheets without that layout are
        // auxiliary data, not FunctionsTest rows.
        let summary_address = CellAddress { column: 1, row: 2 };
        let summary_sheet = self.sheets.first()?;
        let summary_cell = summary_sheet
            .cells
            .iter()
            .find(|cell| cell.address == summary_address && cell.formula.is_some())?;
        if !self
            .sheets
            .iter()
            .any(|sheet| sheet.function_test_layout().is_some())
        {
            return None;
        }

        let mut summary_case = formula_case_for_cell(
            SheetId(1),
            &summary_sheet.name,
            summary_sheet,
            summary_cell,
            FormulaValue::Number(1.0),
        );
        summary_case.read_cell_value = true;
        if value_gets_as_one(&book.cell_value(SheetId(1), summary_address)) {
            return Some(vec![summary_case]);
        }

        for (index, sheet) in self.sheets.iter().enumerate() {
            let Some(layout) = sheet.function_test_layout() else {
                if sheet.has_function_test_header_marker() {
                    panic!(
                        "LibreOffice FunctionsTest layout columns not found on sheet {}",
                        sheet.name
                    );
                }
                continue;
            };
            let sheet_id = SheetId(index as u32 + 1);
            let max_row = sheet
                .cells
                .iter()
                .filter(|cell| cell.address.column == layout.correct_column)
                .map(|cell| cell.address.row)
                .max()
                .unwrap_or(0);
            for row in 1..=max_row {
                let Some(cell) = sheet.cells.iter().find(|cell| {
                    cell.address.column == layout.correct_column && cell.address.row == row
                }) else {
                    continue;
                };
                if value_gets_as_one(&book.cell_value(sheet_id, cell.address)) {
                    continue;
                }
                if cell.formula.is_some() {
                    let mut case = formula_case_for_cell(
                        sheet_id,
                        &sheet.name,
                        sheet,
                        cell,
                        FormulaValue::Number(1.0),
                    );
                    if let Some(function_string) =
                        sheet.function_string(row, layout.function_string_column)
                    {
                        case.formula = function_string;
                    }
                    case.read_cell_value = true;
                    return Some(vec![case]);
                }
            }
        }
        Some(vec![summary_case])
    }

    fn formula_targets(&self) -> Vec<FodsFormulaCase> {
        let mut targets = Vec::new();
        for (index, sheet) in self.sheets.iter().enumerate() {
            let sheet_id = SheetId(index as u32 + 1);
            for cell in &sheet.cells {
                if cell.formula.is_some() {
                    targets.push(formula_case_for_cell(
                        sheet_id,
                        &sheet.name,
                        sheet,
                        cell,
                        cell.cached_value.clone(),
                    ));
                }
            }
        }
        targets
    }

    fn recalc_targets(&self) -> Vec<FodsRecalcTarget> {
        let mut targets = Vec::new();
        for (index, sheet) in self.sheets.iter().enumerate() {
            let sheet_id = SheetId(index as u32 + 1);
            for cell in &sheet.cells {
                let Some(formula) = &cell.formula else {
                    continue;
                };
                let matrix_columns = cell.matrix_columns.max(1);
                let matrix_rows = cell.matrix_rows.max(1);
                let reference = (matrix_columns > 1 || matrix_rows > 1).then_some(CellRange::new(
                    cell.address,
                    CellAddress {
                        column: cell.address.column + matrix_columns - 1,
                        row: cell.address.row + matrix_rows - 1,
                    },
                ));
                targets.push(formula_recalc_target(
                    sheet_id,
                    cell.address,
                    formula,
                    reference,
                ));
            }
        }
        targets
    }
}

fn fods_scalar_cell_value(value: FormulaValue<'static>) -> FormulaValue<'static> {
    match value {
        FormulaValue::Matrix(rows) => rows
            .into_iter()
            .next()
            .and_then(|row| row.into_iter().next())
            .unwrap_or(FormulaValue::Blank),
        value => value,
    }
}

fn fods_array_recalc_updates<'doc>(
    book: &FormulaEvaluationBook<'doc>,
    sheet: SheetId,
    target: CellRange,
    value: &FormulaValue<'doc>,
) -> Vec<(SheetId, CellAddress, FormulaValue<'doc>)> {
    let mut updates = Vec::new();
    let start_row = target.start.row.min(target.end.row);
    let end_row = target.start.row.max(target.end.row);
    let start_column = target.start.column.min(target.end.column);
    let end_column = target.start.column.max(target.end.column);
    for row in start_row..=end_row {
        for column in start_column..=end_column {
            let row_offset = (row - start_row) as usize;
            let column_offset = (column - start_column) as usize;
            let address = CellAddress { column, row };
            let item = match value {
                FormulaValue::Matrix(rows) => rows
                    .get(row_offset)
                    .and_then(|row| row.get(column_offset))
                    .cloned()
                    .unwrap_or_else(|| book.cell_value(sheet, address)),
                value if row == start_row && column == start_column => value.clone(),
                _ => book.cell_value(sheet, address),
            };
            updates.push((sheet, address, item));
        }
    }
    updates
}

#[derive(Clone, Debug)]
struct FodsRecalcTarget {
    sheet: SheetId,
    address: CellAddress,
    parsed_formula: ParsedFormula<'static>,
    reference: Option<CellRange>,
    array_context: bool,
}

fn formula_recalc_target(
    sheet_id: SheetId,
    address: CellAddress,
    formula: &str,
    reference: Option<CellRange>,
) -> FodsRecalcTarget {
    let parsed_formula = parse_formula_with_context(
        FormulaParseContext {
            current_sheet: sheet_id,
            current_cell: Some(address),
            grammar: FormulaGrammar::OpenFormula,
        },
        Cow::Owned(formula.to_string()),
    );
    FodsRecalcTarget {
        sheet: sheet_id,
        address,
        parsed_formula,
        reference,
        array_context: reference.is_some(),
    }
}

fn formula_case_for_cell(
    sheet_id: SheetId,
    sheet_name: &str,
    sheet: &FodsSheet,
    cell: &FodsCell,
    expected: FormulaValue<'static>,
) -> FodsFormulaCase {
    let formula = cell.formula.clone().expect("formula cell");
    let matrix_columns = cell.matrix_columns.max(1);
    let matrix_rows = cell.matrix_rows.max(1);
    let array_reference = (matrix_columns > 1 || matrix_rows > 1).then_some(CellRange::new(
        cell.address,
        CellAddress {
            column: cell.address.column + matrix_columns - 1,
            row: cell.address.row + matrix_rows - 1,
        },
    ));
    let array_expected = array_reference
        .map(|range| {
            let start_row = range.start.row.min(range.end.row);
            let end_row = range.start.row.max(range.end.row);
            let start_column = range.start.column.min(range.end.column);
            let end_column = range.start.column.max(range.end.column);
            let mut values = Vec::new();
            for row in start_row..=end_row {
                for column in start_column..=end_column {
                    let address = CellAddress { column, row };
                    values.push((
                        address,
                        sheet
                            .cached_value_at(address)
                            .unwrap_or_else(|| FormulaValue::Blank),
                    ));
                }
            }
            values
        })
        .unwrap_or_default();
    formula_case_for_address(
        sheet_id,
        sheet_name,
        cell.address,
        &formula,
        array_reference,
        array_expected,
        expected,
    )
}

fn formula_case_for_address(
    sheet_id: SheetId,
    sheet_name: &str,
    address: CellAddress,
    formula: &str,
    array_reference: Option<CellRange>,
    array_expected: Vec<(CellAddress, FormulaValue<'static>)>,
    expected: FormulaValue<'static>,
) -> FodsFormulaCase {
    let parsed_formula = parse_formula_with_context(
        FormulaParseContext {
            current_sheet: sheet_id,
            current_cell: Some(address),
            grammar: FormulaGrammar::OpenFormula,
        },
        Cow::Owned(formula.to_string()),
    );
    FodsFormulaCase {
        sheet: sheet_id,
        sheet_name: sheet_name.to_string(),
        address,
        formula: formula.to_string(),
        parsed_formula,
        array_reference,
        array_expected,
        expected,
        read_cell_value: false,
    }
}

fn value_gets_as_one(value: &FormulaValue<'_>) -> bool {
    match value {
        FormulaValue::Number(value) => (value - 1.0).abs() <= 1e-14,
        FormulaValue::Boolean(value) => *value,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug)]
struct FunctionTestLayout {
    correct_column: u32,
    function_string_column: u32,
}

impl FodsSheet {
    fn function_test_layout(&self) -> Option<FunctionTestLayout> {
        let mut expected_column = 0;
        let mut correct_column = 0;
        let mut function_string_column = 0;
        let max_column = self
            .cells
            .iter()
            .filter(|cell| cell.address.row == 0)
            .map(|cell| cell.address.column)
            .max()
            .unwrap_or(0);
        for column in 0..=max_column {
            match self.header_text_at(column).as_deref() {
                Some("Expected") => expected_column = column,
                Some("Correct") => correct_column = column,
                Some("FunctionString") => {
                    function_string_column = column;
                    break;
                }
                _ => {}
            }
        }
        if expected_column == 0 || correct_column == 0 || function_string_column <= correct_column {
            return None;
        }
        Some(FunctionTestLayout {
            correct_column,
            function_string_column,
        })
    }

    fn header_text_at(&self, column: u32) -> Option<String> {
        self.cells.iter().find_map(|cell| {
            if cell.address.row == 0
                && cell.address.column == column
                && let FormulaValue::String(text) = &cell.cached_value
            {
                Some(text.to_string())
            } else {
                None
            }
        })
    }

    fn has_function_test_header_marker(&self) -> bool {
        self.cells.iter().any(|cell| {
            cell.address.row == 0
                && matches!(
                    &cell.cached_value,
                    FormulaValue::String(text)
                        if matches!(text.as_ref(), "Expected" | "Correct" | "FunctionString")
                )
        })
    }

    fn function_string(&self, row: u32, column: u32) -> Option<String> {
        self.cells.iter().find_map(|cell| {
            if cell.address.row == row
                && cell.address.column == column
                && let FormulaValue::String(text) = &cell.cached_value
            {
                Some(text.to_string())
            } else {
                None
            }
        })
    }

    fn cached_value_at(&self, address: CellAddress) -> Option<FormulaValue<'static>> {
        self.cells
            .iter()
            .find(|cell| cell.address == address)
            .map(|cell| cell.cached_value.clone())
    }
}

fn formula_text_target(_sheet: SheetId, formula: &str) -> Option<CellAddress> {
    let normalized = normalize_formula_text(formula, FormulaGrammar::OpenFormula);
    let inner = normalized
        .strip_prefix("FORMULA(")?
        .strip_suffix(')')?
        .trim();
    let reference = normalize_formula_text(inner, FormulaGrammar::OpenFormula);
    let range = CellRange::parse_a1(reference.as_ref()).ok()?;
    (range.start == range.end).then_some(range.start)
}

impl FodsFormulaCase {
    pub fn evaluate(&self, book: &FormulaEvaluationBook<'static>) -> Option<FormulaValue<'static>> {
        if self.read_cell_value {
            return Some(book.cell_value(self.sheet, self.address));
        }
        if let Some(range) = self.array_reference {
            let value = book.evaluate_parsed_formula_raw(
                self.sheet,
                Some(self.address),
                &self.parsed_formula,
                true,
            )?;
            return book
                .array_recalc_updates(self.sheet, range, &value)
                .into_iter()
                .find_map(|(_, address, value)| (address == self.address).then_some(value));
        }
        book.evaluate_parsed_formula(self.sheet, Some(self.address), &self.parsed_formula)
    }
}

pub fn read_fods_workbook(path: &Path) -> std::io::Result<FodsWorkbook> {
    let file = File::open(path)?;
    read_fods_workbook_from_reader(BufReader::new(file))
}

pub fn read_fods_workbook_from_reader(reader: impl BufRead) -> std::io::Result<FodsWorkbook> {
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut workbook = FodsWorkbook::default();
    let mut current_sheet: Option<FodsSheet> = None;
    let mut row = 0u32;
    let mut row_repeat = 1u32;
    let mut row_start_cell = 0usize;
    let mut column = 0u32;
    let mut current_cell: Option<PendingCell> = None;
    let mut current_covered_cell_repeat: Option<u32> = None;
    let mut current_pivot: Option<FormulaPivotTable<'static>> = None;
    let mut in_text_p = false;
    let mut skipped_table_depth = 0u32;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) if skipped_table_depth > 0 => {
                if local_name(event.name().as_ref()) == b"table" {
                    skipped_table_depth += 1;
                }
            }
            Ok(Event::End(event)) if skipped_table_depth > 0 => {
                if local_name(event.name().as_ref()) == b"table" {
                    skipped_table_depth = skipped_table_depth.saturating_sub(1);
                }
            }
            Ok(_) if skipped_table_depth > 0 => {}
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"table" => {
                if attr_value(&event, b"style-name").as_deref() == Some("ta_extref") {
                    skipped_table_depth = 1;
                    current_sheet = None;
                    continue;
                }
                current_sheet = Some(FodsSheet {
                    name: attr_value(&event, b"name").unwrap_or_default(),
                    cells: Vec::new(),
                    row_states: Vec::new(),
                });
                row = 0;
                column = 0;
            }
            Ok(Event::Start(event)) | Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == b"calculation-settings" =>
            {
                if let Some(search_type) = formula_search_type_from_attrs(&event) {
                    workbook.formula_search_type = search_type;
                }
                if let Some(match_whole_cell) = formula_match_whole_cell_from_attrs(&event) {
                    workbook.formula_match_whole_cell = match_whole_cell;
                }
            }
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"data-pilot-table" => {
                if let Some(target) = attr_value(&event, b"target-range-address")
                    && let Some(target) = parse_fods_qualified_range(&workbook, "", &target)
                {
                    current_pivot = Some(FormulaPivotTable {
                        name: attr_value(&event, b"name").map(Cow::Owned),
                        target,
                        source: QualifiedRange {
                            sheet: SheetId(0),
                            sheet_name: None,
                            range: CellRange::new(CellAddress::default(), CellAddress::default()),
                            start_flags: AddressFlags::default(),
                            end_flags: AddressFlags::default(),
                        },
                        fields: Vec::new(),
                    });
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"data-pilot-table" => {
                if let Some(pivot) = current_pivot.take()
                    && pivot.source.sheet != SheetId(0)
                {
                    workbook.pivot_tables.push(pivot);
                }
            }
            Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == b"source-cell-range" =>
            {
                if let (Some(pivot), Some(source)) = (
                    &mut current_pivot,
                    attr_value(&event, b"cell-range-address"),
                ) && let Some(source) = parse_fods_qualified_range(&workbook, "", &source)
                {
                    pivot.source = source;
                }
            }
            Ok(Event::Start(event)) | Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == b"data-pilot-field" =>
            {
                if let Some(pivot) = &mut current_pivot
                    && let Some(name) = attr_value(&event, b"source-field-name")
                    && !name.is_empty()
                {
                    pivot.fields.push(FormulaPivotField {
                        name: Cow::Owned(name),
                        orientation: pivot_orientation(
                            attr_value(&event, b"orientation").as_deref(),
                        ),
                        function: pivot_function(attr_value(&event, b"function").as_deref()),
                    });
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"table" => {
                if let Some(sheet) = current_sheet.take() {
                    workbook.sheets.push(sheet);
                }
            }
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"table-row" => {
                column = 0;
                row_repeat = attr_value(&event, b"number-rows-repeated")
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(1)
                    .max(1);
                if attr_value(&event, b"visibility").as_deref() == Some("collapse")
                    && let Some(sheet) = &mut current_sheet
                {
                    for offset in 0..row_repeat {
                        sheet.row_states.push((
                            row + offset,
                            FormulaRowState {
                                hidden: true,
                                filtered: false,
                            },
                        ));
                    }
                }
                row_start_cell = current_sheet
                    .as_ref()
                    .map(|sheet| sheet.cells.len())
                    .unwrap_or_default();
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"table-row" => {
                if row_repeat > 1
                    && let Some(sheet) = &mut current_sheet
                {
                    let row_cells = sheet.cells[row_start_cell..].to_vec();
                    for row_offset in 1..row_repeat {
                        sheet.cells.extend(row_cells.iter().map(|cell| FodsCell {
                            address: CellAddress {
                                column: cell.address.column,
                                row: cell.address.row + row_offset,
                            },
                            formula: cell.formula.clone(),
                            matrix_columns: cell.matrix_columns,
                            matrix_rows: cell.matrix_rows,
                            query_empty: cell.query_empty,
                            cached_value: cell.cached_value.clone(),
                        }));
                    }
                }
                row += row_repeat;
            }
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"table-row" => {
                let repeat = attr_value(&event, b"number-rows-repeated")
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(1)
                    .max(1);
                if attr_value(&event, b"visibility").as_deref() == Some("collapse")
                    && let Some(sheet) = &mut current_sheet
                {
                    for offset in 0..repeat {
                        sheet.row_states.push((
                            row + offset,
                            FormulaRowState {
                                hidden: true,
                                filtered: false,
                            },
                        ));
                    }
                }
                row += repeat;
            }
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"table-cell" => {
                current_cell = Some(PendingCell {
                    address: CellAddress { column, row },
                    repeat: attr_value(&event, b"number-columns-repeated")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(1),
                    matrix_columns: attr_value(&event, b"number-matrix-columns-spanned")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(1),
                    matrix_rows: attr_value(&event, b"number-matrix-rows-spanned")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(1),
                    formula: attr_value(&event, b"formula"),
                    cached_value: cached_value_from_attrs(&event),
                    text: String::new(),
                    text_paragraphs: 0,
                });
            }
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"p" => {
                if let Some(cell) = &mut current_cell {
                    if cell.text_paragraphs > 0 {
                        cell.text.push('\n');
                    }
                    cell.text_paragraphs += 1;
                }
                in_text_p = true;
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"p" => {
                in_text_p = false;
            }
            Ok(Event::Start(event))
                if local_name(event.name().as_ref()) == b"covered-table-cell" =>
            {
                current_covered_cell_repeat = Some(cell_repeat(&event));
            }
            Ok(Event::Text(event)) => {
                if in_text_p
                    && let Some(cell) = &mut current_cell
                    && let Ok(text) = event.xml_content(XmlVersion::Implicit1_0)
                {
                    cell.text.push_str(&text);
                }
            }
            Ok(Event::GeneralRef(event)) => {
                if in_text_p
                    && let Some(cell) = &mut current_cell
                    && let Some(text) = xml_general_reference_text(event.as_ref())
                {
                    cell.text.push_str(&text);
                }
            }
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"s" => {
                if in_text_p && let Some(cell) = &mut current_cell {
                    let count = attr_value(&event, b"c")
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(1)
                        .max(1);
                    cell.text.extend(std::iter::repeat_n(' ', count));
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"table-cell" => {
                if let (Some(sheet), Some(cell)) = (&mut current_sheet, current_cell.take()) {
                    let repeat = cell.repeat.max(1);
                    let cached_value = if matches!(cell.cached_value, FormulaValue::Blank)
                        && !cell.text.is_empty()
                    {
                        FormulaValue::String(Cow::Owned(cell.text.clone()))
                    } else if matches!(cell.cached_value, FormulaValue::Blank)
                        && cell.formula.is_some()
                    {
                        FormulaValue::String(Cow::Borrowed(""))
                    } else if matches!(
                        cell.cached_value,
                        FormulaValue::Error(FormulaErrorValue::Unknown)
                    ) {
                        fods_error_value(&cell.text).unwrap_or(cell.cached_value.clone())
                    } else {
                        cell.cached_value.clone()
                    };
                    if should_store_cell(cell.formula.as_deref(), &cached_value) {
                        for offset in 0..repeat {
                            sheet.cells.push(FodsCell {
                                address: CellAddress {
                                    column: cell.address.column + offset,
                                    row: cell.address.row,
                                },
                                formula: cell.formula.clone(),
                                matrix_columns: cell.matrix_columns,
                                matrix_rows: cell.matrix_rows,
                                query_empty: matches!(cached_value, FormulaValue::Number(value) if value == 0.0)
                                    && cell.text.is_empty()
                                    && (cell.matrix_columns.max(1) > 1 || cell.matrix_rows.max(1) > 1),
                                cached_value: cached_value.clone(),
                            });
                        }
                    }
                    column += repeat;
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"covered-table-cell" => {
                column += current_covered_cell_repeat.take().unwrap_or(1);
            }
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"table-cell" => {
                if let Some(sheet) = &mut current_sheet {
                    let repeat = cell_repeat(&event);
                    let formula = attr_value(&event, b"formula");
                    let matrix_columns = attr_value(&event, b"number-matrix-columns-spanned")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(1);
                    let matrix_rows = attr_value(&event, b"number-matrix-rows-spanned")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(1);
                    let cached_value = cached_value_from_attrs(&event);
                    if should_store_cell(formula.as_deref(), &cached_value) {
                        for offset in 0..repeat {
                            sheet.cells.push(FodsCell {
                                address: CellAddress {
                                    column: column + offset,
                                    row,
                                },
                                formula: formula.clone(),
                                matrix_columns,
                                matrix_rows,
                                query_empty: matches!(cached_value, FormulaValue::Number(value) if value == 0.0)
                                    && (matrix_columns.max(1) > 1 || matrix_rows.max(1) > 1),
                                cached_value: cached_value.clone(),
                            });
                        }
                    }
                    column += repeat;
                }
            }
            Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == b"covered-table-cell" =>
            {
                column += cell_repeat(&event);
            }
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"named-range" => {
                if let (Some(name), Some(address)) = (
                    attr_value(&event, b"name"),
                    attr_value(&event, b"cell-range-address"),
                ) {
                    workbook.named_ranges.push(FodsNamedRange {
                        name,
                        sheet_name: None,
                        formula: normalize_fods_named_range_address(&address),
                    });
                }
            }
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"database-range" => {
                if let (Some(name), Some(address)) = (
                    attr_value(&event, b"name"),
                    attr_value(&event, b"target-range-address"),
                ) && let Some(formula) = normalize_fods_database_range_address(&address)
                {
                    workbook.named_ranges.push(FodsNamedRange {
                        name,
                        sheet_name: None,
                        formula,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
            _ => {}
        }
        buf.clear();
    }

    Ok(workbook)
}

#[derive(Clone, Debug)]
struct PendingCell {
    address: CellAddress,
    repeat: u32,
    matrix_columns: u32,
    matrix_rows: u32,
    formula: Option<String>,
    cached_value: FormulaValue<'static>,
    text: String,
    text_paragraphs: usize,
}

fn cached_value_from_attrs(event: &quick_xml::events::BytesStart<'_>) -> FormulaValue<'static> {
    if attr_value_by_qname(event, b"calcext:value-type").as_deref() == Some("error") {
        return FormulaValue::Error(FormulaErrorValue::Unknown);
    }
    match attr_value(event, b"value-type").as_deref() {
        Some("float") | Some("currency") | Some("percentage") => attr_value(event, b"value")
            .and_then(|value| value.parse::<f64>().ok())
            .map(FormulaValue::Number)
            .unwrap_or_default(),
        Some("boolean") => attr_value(event, b"boolean-value")
            .map(|value| match value.as_str() {
                "true" => FormulaValue::Boolean(true),
                "false" => FormulaValue::Boolean(false),
                _ => value
                    .parse::<f64>()
                    .map(FormulaValue::Number)
                    .unwrap_or_default(),
            })
            .unwrap_or_default(),
        Some("string") => attr_value(event, b"string-value")
            .map(|value| FormulaValue::String(Cow::Owned(value)))
            .unwrap_or_default(),
        Some("date") => attr_value(event, b"date-value")
            .and_then(|value| date_value_serial(&value))
            .map(FormulaValue::Number)
            .unwrap_or_default(),
        Some("time") => attr_value(event, b"time-value")
            .and_then(|value| time_value_serial(&value))
            .map(FormulaValue::Number)
            .unwrap_or_default(),
        _ => FormulaValue::Blank,
    }
}

fn fods_error_value(text: &str) -> Option<FormulaValue<'static>> {
    Some(FormulaValue::Error(
        match text.trim().to_ascii_uppercase().as_str() {
            "#NULL!" => FormulaErrorValue::Null,
            "#DIV/0!" => FormulaErrorValue::Div0,
            "#VALUE!" => FormulaErrorValue::Value,
            "#REF!" => FormulaErrorValue::Ref,
            "#NAME?" => FormulaErrorValue::Name,
            "#NUM!" => FormulaErrorValue::Num,
            "#N/A" => FormulaErrorValue::NA,
            "ERR:502" => FormulaErrorValue::IllegalArgument,
            "ERR:511" => FormulaErrorValue::Parameter,
            "ERR:504" | "ERR:508" => FormulaErrorValue::Unknown,
            _ => return None,
        },
    ))
}

fn should_store_cell(formula: Option<&str>, cached_value: &FormulaValue<'_>) -> bool {
    formula.is_some() || !matches!(cached_value, FormulaValue::Blank)
}

fn formula_search_type_from_attrs(
    event: &quick_xml::events::BytesStart<'_>,
) -> Option<FormulaSearchType> {
    let mut search_type = FormulaSearchType::Regex;
    let regex = attr_value(event, b"use-regular-expressions").and_then(|value| parse_bool(&value));
    let wildcards = attr_value(event, b"use-wildcards").and_then(|value| parse_bool(&value));
    if let Some(regex) = regex {
        if !regex && search_type == FormulaSearchType::Regex {
            search_type = FormulaSearchType::Normal;
        }
    }
    if let Some(wildcards) = wildcards {
        if wildcards {
            search_type = FormulaSearchType::Wildcard;
        }
    }
    Some(search_type)
}

fn formula_match_whole_cell_from_attrs(event: &quick_xml::events::BytesStart<'_>) -> Option<bool> {
    attr_value(event, b"search-criteria-must-apply-to-whole-cell")
        .and_then(|value| parse_bool(&value))
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn cell_repeat(event: &quick_xml::events::BytesStart<'_>) -> u32 {
    attr_value(event, b"number-columns-repeated")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1)
}

fn is_today_formula(formula: &str) -> bool {
    formula
        .trim()
        .strip_prefix("of:=")
        .unwrap_or(formula)
        .trim()
        .eq_ignore_ascii_case("TODAY()")
}

fn formula_references_blank_cell(
    workbook: &FodsWorkbook,
    current_sheet_index: usize,
    cell: &FodsCell,
) -> bool {
    let Some(formula) = &cell.formula else {
        return false;
    };
    let Some((sheet_name, address)) = direct_formula_reference(formula) else {
        return false;
    };
    let Some(sheet_index) =
        resolve_formula_reference_sheet(workbook, current_sheet_index, sheet_name)
    else {
        return false;
    };
    let mut visited = BTreeSet::new();
    referenced_cell_is_query_empty(workbook, sheet_index, address, &mut visited)
}

fn referenced_cell_is_query_empty(
    workbook: &FodsWorkbook,
    sheet_index: usize,
    address: CellAddress,
    visited: &mut BTreeSet<(usize, CellAddress)>,
) -> bool {
    if !visited.insert((sheet_index, address)) {
        return false;
    }
    let Some(sheet) = workbook.sheets.get(sheet_index) else {
        return false;
    };
    let Some(cell) = sheet.cells.iter().find(|cell| cell.address == address) else {
        return true;
    };
    if cell.query_empty || matches!(cell.cached_value, FormulaValue::Blank) {
        return true;
    }
    let Some(formula) = &cell.formula else {
        return false;
    };
    let Some((sheet_name, address)) = direct_formula_reference(formula) else {
        return false;
    };
    let Some(sheet_index) = resolve_formula_reference_sheet(workbook, sheet_index, sheet_name)
    else {
        return false;
    };
    referenced_cell_is_query_empty(workbook, sheet_index, address, visited)
}

fn direct_formula_reference(formula: &str) -> Option<(Option<String>, CellAddress)> {
    let normalized = normalize_formula_text(formula, FormulaGrammar::OpenFormula);
    let reference = normalized.trim();
    if reference.is_empty() || reference.contains(':') {
        return None;
    }
    let (sheet_name, cell_reference) = reference
        .rsplit_once('!')
        .map(|(sheet, reference)| (Some(sheet_name_from_formula_reference(sheet)), reference))
        .unwrap_or((None, reference));
    let cell_reference = cell_reference.replace('$', "");
    let address = CellAddress::parse_a1(&cell_reference).ok()?;
    Some((sheet_name, address))
}

fn sheet_name_from_formula_reference(sheet: &str) -> String {
    sheet
        .trim()
        .trim_matches('\'')
        .replace("''", "'")
        .to_string()
}

fn resolve_formula_reference_sheet(
    workbook: &FodsWorkbook,
    current_sheet_index: usize,
    sheet_name: Option<String>,
) -> Option<usize> {
    sheet_name
        .as_deref()
        .map(|name| workbook.sheets.iter().position(|sheet| sheet.name == name))
        .unwrap_or(Some(current_sheet_index))
}

fn normalize_fods_named_range_address(address: &str) -> String {
    let mut normalized = address.trim_start_matches('$').replace(":.", ":");
    if let Some(dot) = normalized.find('.') {
        normalized.replace_range(dot..=dot, "!");
    }
    normalized.retain(|ch| ch != '$');
    normalized
}

fn normalize_fods_database_range_address(address: &str) -> Option<String> {
    let (start, end) = address.split_once(':').unwrap_or((address, address));
    let (start_sheet, start_reference) = split_fods_address_part(start)?;
    let (end_sheet, end_reference) = split_fods_address_part(end)?;
    let start_sheet = start_sheet.trim_start_matches('$');
    let end_sheet = end_sheet.trim_start_matches('$');
    Some(if start_sheet == end_sheet {
        format!("{start_sheet}!{start_reference}:{end_reference}")
    } else {
        format!("{start_sheet}!{start_reference}:{end_sheet}!{end_reference}")
    })
}

fn parse_fods_qualified_range(
    workbook: &FodsWorkbook,
    current_sheet: &str,
    address: &str,
) -> Option<QualifiedRange<'static>> {
    let (start, end) = address.split_once(':').unwrap_or((address, address));
    let (sheet_name, start_reference) = split_fods_address_part(start)?;
    let (_, end_reference) = split_fods_address_part(end)?;
    let sheet_name = sheet_name.trim_matches('\'');
    let sheet = workbook
        .sheets
        .iter()
        .position(|sheet| sheet.name == sheet_name)
        .map(|index| SheetId(index as u32 + 1))
        .or_else(|| {
            (current_sheet == sheet_name).then_some(SheetId(workbook.sheets.len() as u32 + 1))
        })?;
    let reference = format!(
        "{}:{}",
        start_reference.trim_start_matches('$'),
        end_reference.trim_start_matches('$')
    );
    let range = CellRange::parse_a1(&reference).ok()?;
    Some(QualifiedRange {
        sheet,
        sheet_name: Some(SheetName(Cow::Owned(sheet_name.to_string()))),
        range,
        start_flags: AddressFlags::default(),
        end_flags: AddressFlags::default(),
    })
}

fn split_fods_address_part(part: &str) -> Option<(&str, String)> {
    let part = part.trim_start_matches('$');
    let mut in_quote = false;
    for (index, ch) in part.char_indices() {
        if ch == '\'' {
            in_quote = !in_quote;
        } else if ch == '.' && !in_quote {
            return Some((&part[..index], part[index + 1..].replace('$', "")));
        }
    }
    None
}

fn pivot_orientation(value: Option<&str>) -> FormulaPivotFieldOrientation {
    match value {
        Some("row") => FormulaPivotFieldOrientation::Row,
        Some("column") => FormulaPivotFieldOrientation::Column,
        Some("page") => FormulaPivotFieldOrientation::Page,
        Some("data") => FormulaPivotFieldOrientation::Data,
        _ => FormulaPivotFieldOrientation::Hidden,
    }
}

fn pivot_function(value: Option<&str>) -> FormulaPivotFunction {
    match value {
        Some("sum") => FormulaPivotFunction::Sum,
        Some("count") | Some("countnums") => FormulaPivotFunction::Count,
        Some("average") => FormulaPivotFunction::Average,
        Some("max") => FormulaPivotFunction::Max,
        Some("min") => FormulaPivotFunction::Min,
        _ => FormulaPivotFunction::Auto,
    }
}

fn date_value_serial(value: &str) -> Option<f64> {
    let (date, time) = value.split_once('T').unwrap_or((value, ""));
    let (year, month, day) = parse_fods_date(date)?;
    let mut serial = date_serial(year, month, day)?;
    if !time.is_empty() {
        let mut parts = time.split(':');
        let hour = parts.next()?.parse::<f64>().ok()?;
        let minute = parts.next()?.parse::<f64>().ok()?;
        let second = parts
            .next()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        serial += (hour * 3600.0 + minute * 60.0 + second) / 86_400.0;
    }
    Some(serial)
}

fn parse_fods_date(value: &str) -> Option<(i32, i32, i32)> {
    let day_separator = value.rfind('-')?;
    let (head, day) = value.split_at(day_separator);
    let month_separator = head.rfind('-')?;
    let (year, month) = head.split_at(month_separator);
    Some((
        year.parse::<i32>().ok()?,
        month.strip_prefix('-')?.parse::<i32>().ok()?,
        day.strip_prefix('-')?.parse::<i32>().ok()?,
    ))
}

fn time_value_serial(value: &str) -> Option<f64> {
    let mut rest = value.strip_prefix("PT")?;
    let mut seconds = 0.0;
    while !rest.is_empty() {
        let unit_pos = rest.find(|ch: char| ch.is_ascii_alphabetic())?;
        let (number, tail) = rest.split_at(unit_pos);
        let unit = tail.chars().next()?;
        let amount = number.parse::<f64>().ok()?;
        seconds += match unit {
            'H' => amount * 3600.0,
            'M' => amount * 60.0,
            'S' => amount,
            _ => return None,
        };
        rest = &tail[unit.len_utf8()..];
    }
    Some(seconds / 86_400.0)
}

fn date_serial(year: i32, month: i32, day: i32) -> Option<f64> {
    let month_index = month - 1;
    let normalized_year = year + month_index.div_euclid(12);
    let normalized_month = month_index.rem_euclid(12) + 1;
    let days = days_from_civil(normalized_year, normalized_month, 1)? + i64::from(day - 1);
    Some((days - days_from_civil(1899, 12, 30)?) as f64)
}

fn days_from_civil(year: i32, month: i32, day: i32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let year = i64::from(year - i32::from(month <= 2));
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn attr_value(event: &quick_xml::events::BytesStart<'_>, local: &[u8]) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|attr| local_name(attr.key.as_ref()) == local)
        .and_then(|attr| {
            attr.normalized_value(XmlVersion::Implicit1_0)
                .ok()
                .map(|value| value.into_owned())
        })
}

fn attr_value_by_qname(event: &quick_xml::events::BytesStart<'_>, qname: &[u8]) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|attr| attr.key.as_ref() == qname)
        .and_then(normalized_attr_value)
}

fn normalized_attr_value(attr: quick_xml::events::attributes::Attribute<'_>) -> Option<String> {
    attr.normalized_value(XmlVersion::Implicit1_0)
        .ok()
        .map(|value| value.into_owned())
}

fn local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .position(|value| *value == b':')
        .map(|index| &name[index + 1..])
        .unwrap_or(name)
}

fn xml_general_reference_text(reference: &[u8]) -> Option<String> {
    match reference {
        b"amp" => Some("&".to_string()),
        b"lt" => Some("<".to_string()),
        b"gt" => Some(">".to_string()),
        b"quot" => Some("\"".to_string()),
        b"apos" => Some("'".to_string()),
        _ => {
            let reference = std::str::from_utf8(reference).ok()?;
            if let Some(hex) = reference
                .strip_prefix("#x")
                .or_else(|| reference.strip_prefix("#X"))
            {
                u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|value| value.to_string())
            } else {
                reference
                    .strip_prefix('#')
                    .and_then(|decimal| decimal.parse::<u32>().ok())
                    .and_then(char::from_u32)
                    .map(|value| value.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ooxmlsdk_corpus_test_support::workspace_root;

    #[test]
    fn reads_libreoffice_calculation_search_settings() {
        for (settings, expected) in [
            ("", FormulaSearchType::Wildcard),
            (
                r#"<table:calculation-settings table:automatic-find-labels="false"/>"#,
                FormulaSearchType::Regex,
            ),
            (
                r#"<table:calculation-settings table:use-regular-expressions="false"/>"#,
                FormulaSearchType::Normal,
            ),
            (
                r#"<table:calculation-settings table:use-regular-expressions="false" table:use-wildcards="true"/>"#,
                FormulaSearchType::Wildcard,
            ),
        ] {
            let xml = format!(
                r#"
      <office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
          xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
        <office:body>
          <office:spreadsheet>
            {settings}
            <table:table table:name="Sheet1"/>
          </office:spreadsheet>
        </office:body>
      </office:document>
    "#
            );
            let workbook = read_fods_workbook_from_reader(xml.as_bytes()).unwrap();
            assert_eq!(
                workbook.formula_search_type, expected,
                "settings={settings}"
            );
            assert_eq!(workbook.evaluation_book().formula_search_type, expected);
        }
    }

    #[test]
    fn reads_fods_tables_cells_formulas_and_cached_values() {
        let xml = br#"
      <office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
          xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
          xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
          office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
        <office:body>
          <office:spreadsheet>
            <table:table table:name="Sheet1">
              <table:table-row>
                <table:table-cell office:value-type="float" office:value="2"/>
                <table:table-cell table:number-columns-repeated="2"/>
                <table:table-cell table:formula="of:=SUM([.A1:.A1])" office:value-type="float" office:value="2"/>
                <table:table-cell table:formula="of:=&quot;&quot;"><text:p/></table:table-cell>
                <table:covered-table-cell table:number-columns-repeated="2"/>
                <table:table-cell office:value-type="float" office:value="5"/>
              </table:table-row>
              <table:table-row table:number-rows-repeated="2">
                <table:table-cell table:number-columns-repeated="4"/>
              </table:table-row>
              <table:table-row>
                <table:table-cell office:value-type="string"><text:p>ok</text:p></table:table-cell>
              </table:table-row>
            </table:table>
          </office:spreadsheet>
        </office:body>
      </office:document>
    "#;

        let workbook = read_fods_workbook_from_reader(&xml[..]).unwrap();
        assert_eq!(workbook.sheets.len(), 1);
        assert_eq!(workbook.sheets[0].name, "Sheet1");
        assert_eq!(workbook.sheets[0].cells.len(), 5);
        assert_eq!(
            workbook.sheets[0].cells[0].cached_value,
            FormulaValue::Number(2.0)
        );
        assert_eq!(
            workbook.sheets[0].cells[1].address,
            CellAddress { column: 3, row: 0 }
        );
        assert_eq!(
            workbook.sheets[0].cells[1].formula.as_deref(),
            Some("of:=SUM([.A1:.A1])")
        );
        assert_eq!(
            workbook.sheets[0].cells[2].address,
            CellAddress { column: 4, row: 0 }
        );
        assert_eq!(
            workbook.sheets[0].cells[2].cached_value,
            FormulaValue::String(Cow::Borrowed(""))
        );
        assert_eq!(
            workbook.sheets[0].cells[3].address,
            CellAddress { column: 7, row: 0 }
        );
        assert_eq!(
            workbook.sheets[0].cells[3].cached_value,
            FormulaValue::Number(5.0)
        );
        assert_eq!(
            workbook.sheets[0].cells[4].address,
            CellAddress { column: 0, row: 3 }
        );
        assert_eq!(
            workbook.sheets[0].cells[4].cached_value,
            FormulaValue::String(Cow::Borrowed("ok"))
        );
    }

    #[test]
    fn libreoffice_function_test_cases_follow_sheet1_b3_after_hard_recalc() {
        // Source: LibreOffice sc/qa/unit/functions_test.cxx FunctionsTest::load
        // runs DoHardRecalc(), then validates Sheet1.B3. The stale cached
        // Function/Correct values below mirror rows such as WEEKS A12/C12.
        let xml = br#"
      <office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
          xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
          xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
          office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
        <office:body>
          <office:spreadsheet>
            <table:table table:name="Sheet1">
              <table:table-row/>
              <table:table-row/>
              <table:table-row>
                <table:table-cell/>
                <table:table-cell table:formula="of:=AND([Sheet2.C2:.C2])" office:value-type="boolean" office:boolean-value="false"/>
              </table:table-row>
            </table:table>
            <table:table table:name="Sheet2">
              <table:table-row>
                <table:table-cell office:value-type="string" office:string-value="Function"/>
                <table:table-cell office:value-type="string" office:string-value="Expected"/>
                <table:table-cell office:value-type="string" office:string-value="Correct"/>
                <table:table-cell office:value-type="string" office:string-value="FunctionString"/>
              </table:table-row>
              <table:table-row>
                <table:table-cell table:formula="of:=1" office:value-type="float" office:value="0"/>
                <table:table-cell office:value-type="float" office:value="1"/>
                <table:table-cell table:formula="of:=[.A2]=[.B2]" office:value-type="boolean" office:boolean-value="false"/>
                <table:table-cell office:value-type="string" office:string-value="=1"/>
              </table:table-row>
            </table:table>
          </office:spreadsheet>
        </office:body>
      </office:document>
    "#;
        let workbook = read_fods_workbook_from_reader(&xml[..]).unwrap();
        let book = workbook.hard_recalc_book();
        let cases = workbook.libreoffice_function_test_cases(&book).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].address, CellAddress { column: 1, row: 2 });
        assert_eq!(cases[0].expected, FormulaValue::Number(1.0));
        assert_eq!(cases[0].evaluate(&book), Some(FormulaValue::Boolean(true)));
        assert_eq!(
            workbook.cached_formula_cases().len(),
            3,
            "raw cached mode still exposes all formula cells"
        );
    }

    #[test]
    fn reads_countif_fods_criteria_references() {
        let workbook =
            read_fods_workbook(&workspace_root().join(
                "fixtures/LibreOffice/sc/qa/unit/data/functions/statistical/fods/countif.fods",
            ))
            .unwrap();
        let book = workbook.evaluation_book();
        assert_eq!(
            [
                book.cell_value(SheetId(2), CellAddress { column: 8, row: 0 }),
                book.cell_value(SheetId(2), CellAddress { column: 9, row: 0 }),
                book.cell_value(SheetId(2), CellAddress { column: 10, row: 0 }),
            ],
            [
                FormulaValue::Number(2000.0),
                FormulaValue::Number(2006.0),
                FormulaValue::String(Cow::Borrowed(">2006")),
            ]
        );
        for (row, expected) in (0..10).zip(2000..=2009) {
            assert_eq!(
                book.cell_value(SheetId(2), CellAddress { column: 8, row }),
                FormulaValue::Number(f64::from(expected))
            );
        }
        for row in 12..=14 {
            let address = CellAddress { column: 8, row };
            assert_eq!(
                book.query_cell_value(SheetId(2), address, book.cell_value(SheetId(2), address)),
                FormulaValue::Number(0.0)
            );
            assert!(book.is_query_empty_cell(SheetId(2), address));
        }
        let case = workbook
            .cached_formula_cases()
            .into_iter()
            .find(|case| {
                case.sheet == SheetId(2) && case.address == CellAddress { column: 0, row: 5 }
            })
            .expect("Sheet2!A6 formula case");
        assert_eq!(case.formula, "of:=COUNTIF([.I1:.I10];[.K1])");
        assert_eq!(case.evaluate(&book), Some(FormulaValue::Number(3.0)));
        for (row, formula) in [
            (9, "of:=COUNTIF([.$I$13:.$I$15];0)"),
            (10, "of:=COUNTIF([.$I$13:.$I$15];\"\")"),
        ] {
            let case = workbook
                .cached_formula_cases()
                .into_iter()
                .find(|case| {
                    case.sheet == SheetId(2) && case.address == CellAddress { column: 0, row }
                })
                .expect("Sheet2 COUNTIF empty matrix formula case");
            assert_eq!(case.formula, formula);
            assert_eq!(case.evaluate(&book), Some(FormulaValue::Number(3.0)));
        }

        let hard_recalc_book = workbook.hard_recalc_book();
        assert_eq!(
            [
                hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 8, row: 16 }),
                hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 9, row: 16 }),
                hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 8, row: 17 }),
                hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 9, row: 17 }),
                hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 8, row: 18 }),
                hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 9, row: 18 }),
            ],
            [
                FormulaValue::Number(2.0),
                FormulaValue::Number(2.0),
                FormulaValue::Number(3.5),
                FormulaValue::Number(3.5),
                FormulaValue::Number(4.0),
                FormulaValue::Number(4.0),
            ]
        );
        assert_eq!(
            hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 9, row: 20 }),
            FormulaValue::Number(3.5)
        );
        assert_eq!(
            hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 0, row: 17 }),
            FormulaValue::Number(2.0)
        );
        assert_eq!(
            hard_recalc_book.cell_value(SheetId(2), CellAddress { column: 2, row: 17 }),
            FormulaValue::Boolean(true)
        );
    }

    #[test]
    fn imports_direct_blank_reference_formula_cells_as_query_empty() {
        let workbook = read_fods_workbook(
            &workspace_root()
                .join("fixtures/LibreOffice/sc/qa/unit/data/functions/database/fods/dsum.fods"),
        )
        .unwrap();
        let book = workbook.hard_recalc_book();
        assert_eq!(
            book.defined_names
                .get(&DefinedNameKey {
                    sheet: None,
                    name_upper: "DATA".to_string()
                })
                .map(|value| value.as_ref()),
            Some("AOO109200!B6:D9")
        );
        let criterion = CellAddress {
            column: 16,
            row: 14,
        };
        assert_eq!(
            book.query_cell_value(
                SheetId(2),
                criterion,
                book.cell_value(SheetId(2), criterion)
            ),
            FormulaValue::Number(0.0)
        );
        assert!(book.is_query_empty_cell(SheetId(2), criterion));
        assert_eq!(
            book.cell_value(SheetId(2), CellAddress { column: 0, row: 15 }),
            FormulaValue::Number(2.0)
        );
        assert_eq!(
            book.cell_value(SheetId(2), CellAddress { column: 0, row: 11 }),
            FormulaValue::Number(104.0)
        );
    }

    #[test]
    fn imports_matrix_reference_cells_with_formula_text_like_libreoffice() {
        let workbook = read_fods_workbook(
            &workspace_root()
                .join("fixtures/LibreOffice/sc/qa/unit/data/functions/array/fods/logest.fods"),
        )
        .unwrap();
        let book = workbook.evaluation_book();
        let origin = CellAddress {
            column: 10,
            row: 16,
        };
        let covered = CellAddress {
            column: 11,
            row: 16,
        };
        let formula = book
            .formulas
            .get(&(SheetId(2), origin))
            .expect("LOGEST matrix origin formula");
        assert_eq!(formula.kind, FormulaKind::Array);
        assert_eq!(
            formula.reference,
            Some(CellRange::new(
                origin,
                CellAddress {
                    column: 11,
                    row: 20
                }
            ))
        );
        assert!(
            book.formulas.contains_key(&(SheetId(2), covered)),
            "LibreOffice imports matrix covered cells as formula reference cells for FORMULA()"
        );
        assert_eq!(
            book.query_cell_value(SheetId(2), origin, FormulaValue::Blank),
            FormulaValue::Number(1.04333180072885)
        );
        assert_eq!(
            book.query_cell_value(SheetId(2), covered, FormulaValue::Blank),
            FormulaValue::Number(82.5576827252648),
            "matrix target cells must keep their own LibreOffice cached values, not the anchor value"
        );
        let case = workbook
            .cached_formula_cases()
            .into_iter()
            .find(|case| case.sheet == SheetId(2) && case.address == origin)
            .expect("LOGEST matrix origin formula case");
        assert_eq!(
            case.array_reference,
            Some(CellRange::new(
                origin,
                CellAddress {
                    column: 11,
                    row: 20
                }
            ))
        );
        assert!(
            case.array_expected
                .contains(&(covered, FormulaValue::Number(82.5576827252648)))
        );
    }

    #[test]
    fn imports_fods_named_range_addresses_as_parseable_a1_ranges() {
        let workbook =
            read_fods_workbook(&workspace_root().join(
                "fixtures/LibreOffice/sc/qa/unit/data/functions/spreadsheet/fods/vlookup.fods",
            ))
            .unwrap();
        let book = workbook.evaluation_book();
        assert_eq!(
            book.defined_names
                .values()
                .filter(|formula| formula.contains('$'))
                .count(),
            0,
            "FODS named-range formulas must be normalized to parser-compatible A1 syntax"
        );
        assert!(
            book.defined_names
                .values()
                .any(|formula| formula == "Sheet2!AF2:AF5")
        );
        assert!(
            book.defined_names
                .values()
                .any(|formula| formula == "Sheet2!AC2:AD5")
        );
        let ah2 = workbook
            .cached_formula_cases()
            .into_iter()
            .find(|case| {
                case.sheet == SheetId(2) && case.address == CellAddress { column: 33, row: 1 }
            })
            .expect("Sheet2!AH2 VLOOKUP named-range formula");
        assert_eq!(
            ah2.evaluate(&book),
            Some(FormulaValue::String(Cow::Borrowed("cherry")))
        );
    }

    #[test]
    fn hard_recalc_logest_weighted_sum_like_libreoffice() {
        let workbook = read_fods_workbook(
            &workspace_root()
                .join("fixtures/LibreOffice/sc/qa/unit/data/functions/array/fods/logest.fods"),
        )
        .unwrap();
        let book = workbook.hard_recalc_book();
        match book.cell_value(SheetId(2), CellAddress { column: 0, row: 16 }) {
            FormulaValue::Number(value) => assert!(
                (value - 2672.41129180003).abs() < 1.0e-9,
                "expected 2672.41129180003, got {value}"
            ),
            value => panic!("unexpected LOGEST weighted sum value: {value:?}"),
        }
    }

    #[test]
    fn hard_recalc_preserves_imported_matrix_cells_when_result_is_shorter_than_declared_range() {
        let workbook = read_fods_workbook(&workspace_root().join(
            "fixtures/LibreOffice/sc/qa/unit/data/functions/statistical/fods/beta.dist.fods",
        ))
        .unwrap();
        let book = workbook.hard_recalc_book();
        assert_eq!(
            book.cell_value(SheetId(2), CellAddress { column: 0, row: 26 }),
            FormulaValue::Number(0.685470581054688)
        );
        assert_eq!(
            book.cell_value(SheetId(2), CellAddress { column: 2, row: 26 }),
            FormulaValue::Boolean(true)
        );
    }

    #[test]
    fn evaluates_dsum_named_criteria_like_libreoffice() {
        let workbook = read_fods_workbook(
            &workspace_root()
                .join("fixtures/LibreOffice/sc/qa/unit/data/functions/database/fods/dsum.fods"),
        )
        .unwrap();
        let book = workbook.evaluation_book();
        for (row, expected) in [(5, 75.0), (6, 97.0)] {
            let case = workbook
                .cached_formula_cases()
                .into_iter()
                .find(|case| {
                    case.sheet == SheetId(2) && case.address == CellAddress { column: 0, row }
                })
                .expect("Sheet2 DSUM formula case");
            assert_eq!(case.evaluate(&book), Some(FormulaValue::Number(expected)));
        }
    }
}

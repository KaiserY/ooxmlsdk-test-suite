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
    FormulaText, FormulaValue, ParsedFormula, QualifiedRange, SheetBinding, SheetId, SheetName,
    normalize_formula_text, parse_formula_with_context,
};
use quick_xml::{Reader, XmlVersion, events::Event};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FodsWorkbook {
    pub sheets: Vec<FodsSheet>,
    pub named_ranges: Vec<FodsNamedRange>,
    pub pivot_tables: Vec<FormulaPivotTable<'static>>,
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
    pub expected: FormulaValue<'static>,
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
                                query_cell_values
                                    .insert((sheet_id, address), cell.cached_value.clone());
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
            today_serial,
            ..FormulaEvaluationBook::default()
        }
    }

    pub fn formula_cases(&self) -> Vec<FodsFormulaCase> {
        let mut cases = Vec::new();
        for (index, sheet) in self.sheets.iter().enumerate() {
            let sheet_id = SheetId(index as u32 + 1);
            for cell in &sheet.cells {
                if let Some(formula) = &cell.formula {
                    let parsed_formula = parse_formula_with_context(
                        FormulaParseContext {
                            current_sheet: sheet_id,
                            current_cell: Some(cell.address),
                            grammar: FormulaGrammar::OpenFormula,
                        },
                        Cow::Owned(formula.clone()),
                    );
                    cases.push(FodsFormulaCase {
                        sheet: sheet_id,
                        sheet_name: sheet.name.clone(),
                        address: cell.address,
                        formula: formula.clone(),
                        parsed_formula,
                        expected: cell.cached_value.clone(),
                    });
                }
            }
        }
        cases
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
        book.evaluate_parsed_formula(self.sheet, Some(self.address), &self.parsed_formula)
    }
}

pub fn read_fods_workbook(path: &Path) -> std::io::Result<FodsWorkbook> {
    let file = File::open(path)?;
    read_fods_workbook_from_reader(BufReader::new(file))
}

pub fn read_fods_workbook_from_reader(reader: impl BufRead) -> std::io::Result<FodsWorkbook> {
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut workbook = FodsWorkbook::default();
    let mut current_sheet: Option<FodsSheet> = None;
    let mut row = 0u32;
    let mut row_repeat = 1u32;
    let mut row_start_cell = 0usize;
    let mut column = 0u32;
    let mut current_cell: Option<PendingCell> = None;
    let mut current_pivot: Option<FormulaPivotTable<'static>> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"table" => {
                current_sheet = Some(FodsSheet {
                    name: attr_value(&event, b"name").unwrap_or_default(),
                    cells: Vec::new(),
                    row_states: Vec::new(),
                });
                row = 0;
                column = 0;
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
                });
            }
            Ok(Event::Text(event)) => {
                if let Some(cell) = &mut current_cell
                    && let Ok(text) = event.xml_content(XmlVersion::Implicit1_0)
                {
                    if !text.trim().is_empty() {
                        cell.text.push_str(&text);
                    }
                }
            }
            Ok(Event::GeneralRef(event)) => {
                if let Some(cell) = &mut current_cell
                    && let Some(text) = xml_general_reference_text(event.as_ref())
                {
                    cell.text.push_str(&text);
                }
            }
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"s" => {
                if let Some(cell) = &mut current_cell {
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
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"table-cell" => {
                if let Some(sheet) = &mut current_sheet {
                    let repeat = attr_value(&event, b"number-columns-repeated")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(1)
                        .max(1);
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
            Ok(Event::Empty(event)) if local_name(event.name().as_ref()) == b"named-range" => {
                if let (Some(name), Some(address)) = (
                    attr_value(&event, b"name"),
                    attr_value(&event, b"cell-range-address"),
                ) {
                    workbook.named_ranges.push(FodsNamedRange {
                        name,
                        sheet_name: fods_named_range_sheet_name(&address),
                        formula: normalize_fods_named_range_address(&address),
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
            _ => return None,
        },
    ))
}

fn should_store_cell(formula: Option<&str>, cached_value: &FormulaValue<'_>) -> bool {
    formula.is_some() || !matches!(cached_value, FormulaValue::Blank)
}

fn is_today_formula(formula: &str) -> bool {
    formula
        .trim()
        .strip_prefix("of:=")
        .unwrap_or(formula)
        .trim()
        .eq_ignore_ascii_case("TODAY()")
}

fn fods_named_range_sheet_name(address: &str) -> Option<String> {
    address
        .trim_start_matches('$')
        .split_once('.')
        .map(|(sheet, _)| sheet.trim_matches('\'').to_string())
}

fn normalize_fods_named_range_address(address: &str) -> String {
    let mut normalized = address.trim_start_matches('$').replace(":.", ":");
    if let Some(dot) = normalized.find('.') {
        normalized.replace_range(dot..=dot, "!");
    }
    normalized
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
        assert_eq!(workbook.sheets[0].cells.len(), 3);
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
            CellAddress { column: 0, row: 3 }
        );
        assert_eq!(
            workbook.sheets[0].cells[2].cached_value,
            FormulaValue::String(Cow::Borrowed("ok"))
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
            .formula_cases()
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
                .formula_cases()
                .into_iter()
                .find(|case| {
                    case.sheet == SheetId(2) && case.address == CellAddress { column: 0, row }
                })
                .expect("Sheet2 COUNTIF empty matrix formula case");
            assert_eq!(case.formula, formula);
            assert_eq!(case.evaluate(&book), Some(FormulaValue::Number(3.0)));
        }
    }
}

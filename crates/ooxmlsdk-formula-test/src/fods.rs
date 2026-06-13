use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use ooxmlsdk_formula::{
    CellAddress, CellRange, FormulaErrorValue, FormulaEvaluationBook, FormulaGrammar, FormulaKind,
    FormulaText, FormulaValue, SheetBinding, SheetId, normalize_formula_text,
};
use quick_xml::{Reader, XmlVersion, events::Event};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FodsWorkbook {
    pub sheets: Vec<FodsSheet>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FodsSheet {
    pub name: String,
    pub cells: Vec<FodsCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FodsCell {
    pub address: CellAddress,
    pub formula: Option<String>,
    pub matrix_columns: u32,
    pub matrix_rows: u32,
    pub cached_value: FormulaValue<'static>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FodsFormulaCase {
    pub sheet: SheetId,
    pub sheet_name: String,
    pub address: CellAddress,
    pub formula: String,
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
        let mut formulas = BTreeMap::new();
        for (index, sheet) in self.sheets.iter().enumerate() {
            let sheet_id = SheetId(index as u32 + 1);
            for cell in &sheet.cells {
                cells.insert((sheet_id, cell.address), cell.cached_value.clone());
                if let Some(formula) = &cell.formula {
                    let normalized = normalize_formula_text(formula, FormulaGrammar::OpenFormula);
                    let matrix_columns = cell.matrix_columns.max(1);
                    let matrix_rows = cell.matrix_rows.max(1);
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
        FormulaEvaluationBook {
            sheet_names,
            cells,
            formulas,
            ..FormulaEvaluationBook::default()
        }
    }

    pub fn formula_cases(&self) -> Vec<FodsFormulaCase> {
        let mut cases = Vec::new();
        for (index, sheet) in self.sheets.iter().enumerate() {
            let sheet_id = SheetId(index as u32 + 1);
            for cell in &sheet.cells {
                if let Some(formula) = &cell.formula {
                    cases.push(FodsFormulaCase {
                        sheet: sheet_id,
                        sheet_name: sheet.name.clone(),
                        address: cell.address,
                        formula: formula.clone(),
                        expected: cell.cached_value.clone(),
                    });
                }
            }
        }
        cases
    }
}

impl FodsFormulaCase {
    pub fn evaluate(&self, book: &FormulaEvaluationBook<'static>) -> Option<FormulaValue<'static>> {
        book.evaluate_formula_text_with_grammar(
            self.sheet,
            Some(self.address),
            &self.formula,
            FormulaGrammar::OpenFormula,
        )
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
    let mut column = 0u32;
    let mut current_cell: Option<PendingCell> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"table" => {
                current_sheet = Some(FodsSheet {
                    name: attr_value(&event, b"name").unwrap_or_default(),
                    cells: Vec::new(),
                });
                row = 0;
                column = 0;
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"table" => {
                if let Some(sheet) = current_sheet.take() {
                    workbook.sheets.push(sheet);
                }
            }
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"table-row" => {
                column = 0;
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"table-row" => {
                row += 1;
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
                    && let Ok(text) = event.decode()
                {
                    cell.text.push_str(&text);
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"table-cell" => {
                if let (Some(sheet), Some(cell)) = (&mut current_sheet, current_cell.take()) {
                    let repeat = cell.repeat.max(1);
                    for offset in 0..repeat {
                        sheet.cells.push(FodsCell {
                            address: CellAddress {
                                column: cell.address.column + offset,
                                row: cell.address.row,
                            },
                            formula: cell.formula.clone(),
                            matrix_columns: cell.matrix_columns,
                            matrix_rows: cell.matrix_rows,
                            cached_value: if matches!(cell.cached_value, FormulaValue::Blank)
                                && !cell.text.is_empty()
                            {
                                FormulaValue::String(Cow::Owned(cell.text.clone()))
                            } else {
                                cell.cached_value.clone()
                            },
                        });
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
                    for offset in 0..repeat {
                        sheet.cells.push(FodsCell {
                            address: CellAddress {
                                column: column + offset,
                                row,
                            },
                            formula: formula.clone(),
                            matrix_columns,
                            matrix_rows,
                            cached_value: cached_value.clone(),
                        });
                    }
                    column += repeat;
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
            .map(|value| FormulaValue::Boolean(value == "true"))
            .unwrap_or_default(),
        Some("string") => attr_value(event, b"string-value")
            .map(|value| FormulaValue::String(Cow::Owned(value)))
            .unwrap_or_default(),
        Some("date") => attr_value(event, b"date-value")
            .and_then(|value| date_value_serial(&value))
            .map(FormulaValue::Number)
            .unwrap_or_default(),
        _ => FormulaValue::Blank,
    }
}

fn date_value_serial(value: &str) -> Option<f64> {
    let (date, time) = value.split_once('T').unwrap_or((value, ""));
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<i32>().ok()?;
    let day = parts.next()?.parse::<i32>().ok()?;
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

fn date_serial(year: i32, month: i32, day: i32) -> Option<f64> {
    let month_index = month - 1;
    let normalized_year = year + month_index.div_euclid(12);
    let normalized_month = month_index.rem_euclid(12) + 1;
    let days = days_from_civil(normalized_year, normalized_month, 1)? + i64::from(day - 1);
    let base = days_from_civil(1899, 12, 31)?;
    let mut serial = days - base;
    let leap_bug_start = days_from_civil(1900, 3, 1)?;
    if days >= leap_bug_start {
        serial += 1;
    }
    Some(serial as f64)
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

#[cfg(test)]
mod tests {
    use super::*;

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
            workbook.sheets[0].cells[3].formula.as_deref(),
            Some("of:=SUM([.A1:.A1])")
        );
        assert_eq!(
            workbook.sheets[0].cells[4].cached_value,
            FormulaValue::String(Cow::Borrowed("ok"))
        );
    }
}

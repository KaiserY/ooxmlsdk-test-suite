use ooxmlsdk_layout::common::{Color, DisplayItem, Fill};
use ooxmlsdk_layout_test::{
    all_page_text, assert_link_target, assert_page_contains, assert_page_contains_any,
    assert_page_image_count, assert_page_not_contains, assert_page_path_count_at_least,
    assert_page_size, assert_page_text_occurrences, assert_text_color, assert_text_font_size,
    debug_shapes, page_display_items, run_named_cases_parallel, xlsx_layout,
};

#[test]
fn serial_zero_formula_dates_fit_the_original_excel_cells() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;

    // The unchanged Office export of 61652.xlsx shows January 0, 1900.
    // Formatting it as December 31, 1899 overflows A1/A2 and prints hashes.
    let package = SpreadsheetDocument::new_from_file(ooxmlsdk_layout_test::corpus_file(
        "../Apache-POI/test-data/spreadsheet/61652.xlsx",
    ))
    .unwrap();
    let document = ooxmlsdk_layout::xlsx::layout_document(
        &package,
        &LayoutOptions {
            format_locale: Some("zh-CN".into()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    assert_eq!(document.pages.len(), 11);
    for page in [0, 1] {
        assert_page_text_occurrences(&document, page, "1900/1/0", 2);
        assert_page_not_contains(&document, page, "########");
        assert_page_not_contains(&document, page, "1899/");
    }
}

#[test]
fn automatic_rows_convert_through_the_normal_font_print_grid() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    // Configured Excel exports and native screen/printer EMFs of POI 64473:
    // all nine Calibri 24 controls have the same 30.75pt screen row height.
    // The Normal font changes the printed row spacing. The last controls
    // distinguish an all-column override from partly/wholly unstyled columns.
    let cases = [
        ("Arial", 10, 24., "all", 29.76),
        ("Arial", 11, 24., "all", 31.08),
        ("Arial", 12, 24., "all", 30.24),
        ("Calibri", 11, 24., "all", 31.32),
        ("Times New Roman", 10, 24., "all", 29.76),
        ("SimSun", 10, 24., "all", 29.76),
        ("SimSun", 11, 24., "all", 30.84),
        ("SimSun", 12, 24., "all", 28.56),
        ("DengXian", 11, 24., "all", 27.60),
        ("Arial", 10, 11., "all", 13.80),
        ("SimSun", 11, 11., "all", 14.28),
        ("SimSun", 11, 11., "single", 14.64),
        ("SimSun", 11, 11., "none", 14.64),
    ];
    for (family, normal_size, cell_size, columns, expected) in cases {
        let mut package = SpreadsheetDocument::new_from_file(ooxmlsdk_layout_test::corpus_file(
            "../Apache-POI/test-data/openxml4j/64473.xlsx",
        ))
        .unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet_id = workbook.root_element(&package).unwrap().sheets.sheet[0]
            .id
            .clone();
        let Some(ooxmlsdk::parts::PartRef::WorksheetPart(sheet)) =
            workbook.get_part_by_id(&package, &sheet_id)
        else {
            panic!("missing worksheet");
        };
        let styles = workbook.workbook_styles_part(&package).unwrap();
        let fonts = &mut styles
            .root_element_mut(&mut package)
            .unwrap()
            .fonts
            .as_mut()
            .unwrap()
            .xml_children;
        let charset = if matches!(family, "SimSun" | "DengXian") {
            134
        } else {
            1
        };
        fonts[0] = x::FontsChoice::Font(x::Font::from_bytes(format!(r#"<font xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sz val="{normal_size}"/><name val="{family}"/><charset val="{charset}"/></font>"#).as_bytes()).unwrap());
        let x::FontsChoice::Font(font) = &mut fonts[1] else {
            panic!("missing cell font");
        };
        for choice in &mut font.font_choice {
            if let x::FontChoice::FontSize(size) = choice {
                size.val = cell_size;
            }
        }
        let worksheet = sheet.root_element_mut(&mut package).unwrap();
        for column in worksheet
            .columns
            .iter_mut()
            .flat_map(|columns| &mut columns.column)
        {
            if columns == "none" || columns == "single" && column.min != 1 {
                column.style = None;
            }
        }
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("ko-KR".into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..LayoutOptions::default()
            },
        )
        .unwrap();
        assert_eq!(document.pages.len(), 1);
        let mut baselines = page_display_items(&document, 0)
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Text(run) if run.text == "#REF!" => Some(run.origin.y.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        baselines.sort_by(f32::total_cmp);
        assert_eq!(baselines.len(), 4);
        for pair in baselines.windows(2) {
            assert!(
                (pair[1] - pair[0] - expected).abs() < 0.02,
                "{family} {normal_size}, Calibri {cell_size}, columns={columns}: {baselines:?}, expected step {expected}"
            );
        }
    }
}

#[test]
fn automatic_mixed_font_rows_share_only_bottom_aligned_baselines() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk_layout::LayoutOptions;

    // Three Office controls of the TNR 10 / SimSun 10 row grid. A1 and A3
    // have the same font and alignment, so their baseline distance measures
    // the combined height of rows 1 and 2, including the blank styled row.
    for (alignment, expected) in [
        (x::VerticalAlignmentValues::Center, 25.44),
        (x::VerticalAlignmentValues::Top, 25.44),
        (x::VerticalAlignmentValues::Bottom, 26.16),
    ] {
        let mut package = SpreadsheetDocument::new_from_file(ooxmlsdk_layout_test::corpus_file(
            "sc/qa/unit/data/xlsx/tdf130104_indent.xlsx",
        ))
        .unwrap();
        let workbook = package.workbook_part().unwrap();
        let styles = workbook.workbook_styles_part(&package).unwrap();
        let styles = styles.root_element_mut(&mut package).unwrap();
        for format in styles
            .cell_formats
            .as_mut()
            .unwrap()
            .xml_children
            .iter_mut()
            .filter_map(|child| match child {
                x::CellFormatsChoice::CellFormat(format) => Some(format),
                _ => None,
            })
        {
            format.alignment.as_mut().unwrap().vertical = Some(alignment);
        }
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("ko-KR".into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..LayoutOptions::default()
            },
        )
        .unwrap();
        let items = page_display_items(&document, 0);
        let baselines = ["Indent by 0", "Indent by 1"].map(|text| {
            items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(run) if run.text == text => Some(run.origin.y.0),
                    _ => None,
                })
                .expect("missing indent text")
        });
        assert!(
            (baselines[1] - baselines[0] - expected).abs() < 0.02,
            "{alignment:?}: {baselines:?}, expected distance {expected}"
        );
    }
}

#[test]
fn automatic_thick_rows_include_blank_cell_fonts_and_ignore_height_caches() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    // Configured Excel controls of POI 61652.xlsx. B8..B12 are top-aligned
    // markers: bottom-aligned cells share a row baseline and cannot measure
    // grid heights independently. N9:S9 remain blank but carry font 2.
    // Compare automatic rows after realizing each on Office's 600-DPI grid.
    // The layout can already carry the Normal font's printer height, which
    // differs slightly from COM.Top's logical screen height.
    for (font_size, expected_height) in [(11.0, 13.92), (14.0, 18.0), (18.0, 23.64)] {
        for cache in [None, Some(60.0)] {
            let mut package = SpreadsheetDocument::new_from_file(
                ooxmlsdk_layout_test::corpus_file("../Apache-POI/test-data/spreadsheet/61652.xlsx"),
            )
            .unwrap();
            let workbook = package.workbook_part().unwrap();
            let sheet_id = workbook
                .root_element(&package)
                .unwrap()
                .sheets
                .sheet
                .iter()
                .find(|sheet| sheet.name == "IRPPCalc")
                .unwrap()
                .id
                .clone();
            let Some(ooxmlsdk::parts::PartRef::WorksheetPart(sheet)) =
                workbook.get_part_by_id(&package, &sheet_id)
            else {
                panic!("missing IRPPCalc");
            };
            let styles = workbook.workbook_styles_part(&package).unwrap();
            let styles = styles.root_element_mut(&mut package).unwrap();
            let x::FontsChoice::Font(font) = &mut styles.fonts.as_mut().unwrap().xml_children[2]
            else {
                panic!("missing row font");
            };
            for choice in &mut font.font_choice {
                if let x::FontChoice::FontSize(size) = choice {
                    size.val = font_size;
                }
            }
            let formats = styles.cell_formats.as_mut().unwrap();
            assert!(
                formats
                    .xml_children
                    .iter()
                    .all(|child| matches!(child, x::CellFormatsChoice::CellFormat(_)))
            );
            let top_style = formats.xml_children.len();
            formats.xml_children.push(x::CellFormatsChoice::CellFormat(Box::new(x::CellFormat::from_bytes(br#"<xf xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment vertical="top"/></xf>"#).unwrap())));
            formats.count = Some(formats.xml_children.len() as u32);
            let worksheet = sheet.root_element_mut(&mut package).unwrap();
            for index in 8..=12 {
                if !worksheet
                    .sheet_data
                    .row
                    .iter()
                    .any(|row| row.row_index == Some(index))
                {
                    worksheet.sheet_data.row.push(x::Row {
                        row_index: Some(index),
                        ..Default::default()
                    });
                }
                let row = worksheet
                    .sheet_data
                    .row
                    .iter_mut()
                    .find(|row| row.row_index == Some(index))
                    .unwrap();
                if matches!(index, 8 | 9) {
                    row.height = cache;
                }
                assert!(
                    !row.cell
                        .iter()
                        .any(|cell| cell.cell_reference.as_deref() == Some(&format!("B{index}")))
                );
                row.cell.push(x::Cell::from_bytes(format!(r#"<c xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" r="B{index}" s="{top_style}" t="n"><v>{}</v></c>"#, 9000+index).as_bytes()).unwrap());
                row.cell.sort_by_key(|cell| cell.cell_reference.clone());
            }
            worksheet.sheet_data.row.sort_by_key(|row| row.row_index);
            let document = ooxmlsdk_layout::xlsx::layout_document(
                &package,
                &LayoutOptions {
                    ui_language: Some("es-MX".into()),
                    format_locale: Some("zh-CN".into()),
                    default_document_language: Some("zh-CN".into()),
                    ..LayoutOptions::default()
                },
            )
            .unwrap();
            let items = page_display_items(&document, 0);
            let markers = (8..=12)
                .map(|row| {
                    let text = (9000 + row).to_string();
                    items
                        .iter()
                        .find_map(|item| match item {
                            DisplayItem::Text(run) if run.text == text => Some(run.origin.y.0),
                            _ => None,
                        })
                        .expect("missing top-aligned marker")
                })
                .collect::<Vec<_>>();
            for (pair, expected) in markers.windows(2).zip([13.92, expected_height]) {
                let printed_height =
                    ooxmlsdk_layout::units::quantize_points_to_office_print_grid(pair[1] - pair[0]);
                assert!(
                    (printed_height - expected).abs() < 0.02,
                    "font={font_size}, cache={cache:?}, expected={expected}, markers={markers:?}"
                );
            }
        }
    }
}

#[test]
fn xlsx_workbooks_without_printable_pages_do_not_gain_a_blank_page() {
    // Excel exported zero pages for these unchanged sources in both the
    // configured campaign and the 2026-09-12 print/all-pages recovery probe.
    for file in [
        "sc/qa/unit/data/xlsx/empty-noconf.xlsx",
        "sc/qa/unit/data/xlsx/hiddenShape.xlsx",
        "../Apache-POI/test-data/spreadsheet/SheetTabColors.xlsx",
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/WorkbookProperties.xlsx",
    ] {
        let document = xlsx_layout(file).unwrap();
        assert!(
            document.pages.is_empty(),
            "{file}: {} pages",
            document.pages.len()
        );
    }
}

#[test]
fn empty_sheet_headers_print_when_a_visible_cell_supplies_content() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    // Excel's unchanged sources export zero pages. Adding only A1 produces
    // one page and preserves these headers, including their colors/styles.
    // See the Office controls linked from xlsx-legacy-assertion-resolution.md.
    for (file, sheet_index) in [
        ("hidden_sheets.xlsx", 1),
        ("tdf134459_HeaderFooterColor.xlsx", 0),
        ("tdf134817_HeaderFooterTextWith2Section.xlsx", 0),
        ("tdf134826.xlsx", 0),
    ] {
        let path = ooxmlsdk_layout_test::corpus_file(&format!("sc/qa/unit/data/xlsx/{file}"));
        let mut package = SpreadsheetDocument::new_from_file(path).unwrap();
        let sheet = package
            .workbook_part()
            .unwrap()
            .worksheet_parts(&package)
            .nth(sheet_index)
            .unwrap();
        let rows = &mut sheet.root_element_mut(&mut package).unwrap().sheet_data.row;
        assert!(rows.is_empty());
        rows.push(x::Row::from_bytes(br#"<row xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" r="1"><c r="A1" t="inlineStr"><is><t>Office print probe</t></is></c></row>"#).unwrap());
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                source_file_name: Some(file.into()),
                ..LayoutOptions::default()
            },
        )
        .unwrap();
        assert_eq!(document.pages.len(), 1, "{file}");
        assert_page_contains(&document, 0, "Office print probe");
        match file {
            "hidden_sheets.xlsx" => {
                assert_page_contains(&document, 0, "Sheet2");
                assert_page_not_contains(&document, 0, "Sheet1");
                assert_page_not_contains(&document, 0, "Sheet3");
            }
            "tdf134459_HeaderFooterColor.xlsx" => {
                for text in ["l", "c", "r"] {
                    assert_eq!(
                        page_display_items(&document, 0)
                            .iter()
                            .filter(|item| matches!(
                                item, DisplayItem::Text(run) if run.text == text
                            ))
                            .count(),
                        2,
                        "header/footer text {text}"
                    );
                }
                let colors = page_display_items(&document, 0)
                    .iter()
                    .filter_map(|item| match item {
                        DisplayItem::Text(run) if matches!(run.text.as_ref(), "l" | "c" | "r") => {
                            Some(run.color)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                for (r, g, b) in [(192, 96, 64), (76, 55, 137), (100, 207, 95), (205, 21, 170)] {
                    let color = Color { r, g, b, a: 255 };
                    assert!(
                        colors.contains(&color),
                        "missing header/footer color {color:?}"
                    );
                }
            }
            "tdf134817_HeaderFooterTextWith2Section.xlsx" => {
                for text in ["aaa", "bbb", "camb", "dant"] {
                    assert_page_contains(&document, 0, text);
                }
            }
            "tdf134826.xlsx" => {
                for (text, bold, italic) in [("Bold", true, false), ("Italic", false, true)] {
                    assert!(page_display_items(&document, 0).iter().any(|item| matches!(
                        item, DisplayItem::Text(run)
                        if run.text == text && run.style.bold == bold && run.style.italic == italic
                    )), "missing styled header {text}");
                }
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn explicit_blank_print_areas_preserve_office_pages_and_manual_breaks() {
    // The unchanged Office PDFs contain six and one completely blank pages.
    // Sheets.xlsx has two disjoint areas plus a second sheet split 2 x 2 by
    // manual breaks. LoadSheetsWithCommas.xlsx also has an implicit blank
    // second sheet, which must remain absent from the output.
    for (file, pages) in [
        (
            "../ClosedXML/ClosedXML.Tests/Resource/Examples/PageSetup/Sheets.xlsx",
            6,
        ),
        (
            "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/LoadSheetsWithCommas.xlsx",
            1,
        ),
    ] {
        let document = xlsx_layout(file).unwrap();
        assert_eq!(document.pages.len(), pages, "{file}");
        for page in 0..pages {
            assert_page_size(&document, page, 595.32, 841.92);
            assert!(
                page_display_items(&document, page).is_empty(),
                "{file}: page {page}"
            );
        }
    }
}

#[test]
fn removing_explicit_print_areas_leaves_the_same_blank_sheets_unprinted() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;

    for file in [
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/PageSetup/Sheets.xlsx",
        "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/LoadSheetsWithCommas.xlsx",
    ] {
        let path = ooxmlsdk_layout_test::corpus_file(file);
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let names = workbook
            .root_element_mut(&mut package)
            .unwrap()
            .defined_names
            .as_mut()
            .unwrap();
        assert!(!names.defined_name.is_empty());
        // These sources only define print areas; leave worksheet data,
        // dimensions, manual breaks, and every other setting unchanged.
        assert!(
            names
                .defined_name
                .iter()
                .all(|name| name.name == "_xlnm.Print_Area")
        );
        names.defined_name.clear();
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        assert!(document.pages.is_empty(), "{file}");
    }
}

#[test]
fn conditional_stop_if_true_preserves_office_value_and_only_lower_row_icons() {
    // The original Office PDF prints only A1's 6 and three traffic lights.
    // A1 satisfies a higher-priority stop rule with no differential style;
    // showValue=0 on the later icon rule must not hide this stopped value.
    let document = xlsx_layout(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/CFStopIfTrue.xlsx",
    )
    .unwrap();
    assert_eq!(document.pages.len(), 1);
    assert_page_text_occurrences(&document, 0, "6", 1);
    for value in ["1", "2", "3"] {
        assert_page_text_occurrences(&document, 0, value, 0);
    }
    assert_page_image_count(&document, 0, 3);
}

#[test]
fn conditional_icon_rules_obey_matching_stop_flags_and_priority() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk_layout::LayoutOptions;

    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/CFStopIfTrue.xlsx",
    );
    // Exercise the same source values and icon rule with disabled stopping,
    // a lower-priority stop, no matching value, the opposite match, and a
    // higher-priority rule stored in another conditionalFormatting block.
    for (label, stop, priority, operator, threshold, separate, visible, icons) in [
        (
            "no stop",
            false,
            1,
            x::ConditionalFormattingOperatorValues::GreaterThan,
            "5",
            false,
            &[][..],
            4,
        ),
        (
            "icon first",
            true,
            3,
            x::ConditionalFormattingOperatorValues::GreaterThan,
            "5",
            false,
            &[][..],
            4,
        ),
        (
            "no match",
            true,
            1,
            x::ConditionalFormattingOperatorValues::GreaterThan,
            "9",
            false,
            &[][..],
            4,
        ),
        (
            "opposite match",
            true,
            1,
            x::ConditionalFormattingOperatorValues::LessThan,
            "5",
            false,
            &["1", "2", "3"][..],
            1,
        ),
        (
            "separate block",
            true,
            1,
            x::ConditionalFormattingOperatorValues::GreaterThan,
            "5",
            true,
            &["6"][..],
            3,
        ),
    ] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        let worksheet = sheet.root_element_mut(&mut package).unwrap();
        let formats = &mut worksheet.conditional_formatting;
        let rule = &mut formats[0].conditional_formatting_rule[0];
        assert!(rule.format_id.is_none());
        assert!(rule.icon_set.is_none());
        rule.stop_if_true = Some(stop.into());
        rule.priority = priority;
        rule.operator = Some(operator);
        rule.formula[0].xml_content = Some(threshold.to_owned());
        if separate {
            let mut earlier_rule = formats[0].clone();
            earlier_rule.conditional_formatting_rule.truncate(1);
            formats[0].conditional_formatting_rule.remove(0);
            formats.push(earlier_rule);
        }
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        assert_eq!(document.pages.len(), 1, "{label}");
        for value in ["6", "1", "2", "3"] {
            assert_page_text_occurrences(
                &document,
                0,
                value,
                usize::from(visible.contains(&value)),
            );
        }
        assert_page_image_count(&document, 0, icons);
    }
}

#[test]
fn narrow_general_numbers_use_scientific_notation_before_hashes() {
    // Current configured Office PDFs use scientific notation when the
    // authored General integer cannot fit its original column width.
    for (file, expected) in [
        ("../Apache-POI/test-data/spreadsheet/57914.xlsx", "1E+09"),
        ("sc/qa/unit/data/xlsx/escape-unicode.xlsx", "1.16E+09"),
    ] {
        let document = xlsx_layout(file).unwrap();
        let items = page_display_items(&document, 0);
        assert!(
            items.iter().any(|item| matches!(item,
                DisplayItem::Text(text) if text.text == expected
            )),
            "{file}: missing exact General number {expected}"
        );
        assert!(
            !items.iter().any(|item| matches!(item,
                DisplayItem::Text(text) if text.text.starts_with("##")
            )),
            "{file}: numeric overflow hashes"
        );
    }
}

#[test]
fn text_formula_date_escape_markers_are_not_painted() {
    // The unchanged source's TEXT formula, cached value, and current Office
    // PDF agree on 09 Mar 2016; the format's escape markers are not output.
    let document = xlsx_layout("../Apache-POI/test-data/spreadsheet/59106.xlsx").unwrap();
    assert_page_contains(&document, 0, "09 Mar 2016");
    assert_page_not_contains(&document, 0, r"09\ Mar\ 2016");
}

#[test]
fn rtl_worksheet_columns_follow_office_without_reversing_cell_text() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;

    // Office prints C1, B1, A1 from left to right. Its implicit column width
    // differs from ours in both LTR and RTL sheets; compare placement against
    // the same source's LTR geometry here. The configured PDF comparison
    // separately retains its complete Office geometry checks.
    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/RightToLeft.xlsx",
    );
    let mut snapshots = Vec::new();
    for right_to_left in [false, true] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        let view = &mut sheet
            .root_element_mut(&mut package)
            .unwrap()
            .sheet_views
            .as_mut()
            .unwrap()
            .sheet_view[0];
        assert!(view.right_to_left.unwrap().as_bool());
        if !right_to_left {
            view.right_to_left = Some(false.into());
        }
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        assert_eq!(document.pages.len(), 1);
        assert_page_size(&document, 0, 595.32, 841.92);
        let mut runs = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| left.text.cmp(&right.text));
        assert_eq!(
            runs.iter()
                .map(|text| text.text.as_ref())
                .collect::<Vec<_>>(),
            ["A1", "B1", "C1"]
        );
        snapshots.push(runs);
    }
    let ltr = &snapshots[0];
    let rtl = &snapshots[1];
    let column_width = ltr[1].origin.x.0 - ltr[0].origin.x.0;
    assert!(column_width > 0.0);
    assert!((ltr[2].origin.x.0 - ltr[1].origin.x.0 - column_width).abs() < 0.001);
    // The unchanged source authors equal 0.75-inch page margins. Reflect
    // cell rectangles at the page center while preserving the LTR text inset.
    let text_inset = ltr[0].origin.x.0 - 0.75 * 72.0;
    assert!(text_inset > 0.0 && text_inset < column_width);
    for (ltr_run, rtl_run) in ltr.iter().zip(rtl) {
        let expected_x = 595.32 - ltr_run.origin.x.0 - column_width + 2.0 * text_inset;
        assert!(
            (rtl_run.origin.x.0 - expected_x).abs() < 0.001,
            "{}: x={} expected {expected_x}",
            rtl_run.text,
            rtl_run.origin.x.0
        );
        assert_eq!(ltr_run.text, rtl_run.text);
        assert_eq!(ltr_run.style, rtl_run.style);
        assert_eq!(ltr_run.color, rtl_run.color);
        assert_eq!(ltr_run.origin.y, rtl_run.origin.y);
        assert_eq!(rtl_run.style.rotation_degrees, 0.0);
    }
    assert!(rtl[2].origin.x.0 < rtl[1].origin.x.0);
    assert!(rtl[1].origin.x.0 < rtl[0].origin.x.0);
}

#[test]
fn rtl_worksheet_keeps_general_and_authored_alignment_inside_each_cell() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/RightToLeft.xlsx",
    );
    let mut snapshots = Vec::new();
    for right_to_left in [false, true] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        *sheet.root_element_mut(&mut package).unwrap() = x::Worksheet::from_bytes(
            format!(
                r#"
          <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <sheetViews><sheetView workbookViewId="0" rightToLeft="{right_to_left}"/></sheetViews>
            <cols><col min="1" max="1" width="20" customWidth="1"/></cols>
            <sheetData>
              <row r="1"><c r="A1" s="1" t="inlineStr"><is><t>AB</t></is></c></row>
              <row r="2"><c r="A2" s="2" t="inlineStr"><is><t>AB</t></is></c></row>
              <row r="3"><c r="A3" s="3" t="inlineStr"><is><t>AB</t></is></c></row>
              <row r="4"><c r="A4" s="0" t="inlineStr"><is><t>AB</t></is></c></row>
              <row r="5"><c r="A5" s="0"><v>12</v></c></row>
            </sheetData>
          </worksheet>"#
            )
            .as_bytes(),
        )
        .unwrap();
        let styles = workbook.workbook_styles_part(&package).unwrap();
        *styles.root_element_mut(&mut package).unwrap() = x::Stylesheet::from_bytes(br#"
          <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
            <fills count="1"><fill><patternFill patternType="none"/></fill></fills>
            <borders count="1"><border/></borders>
            <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
            <cellXfs count="4">
              <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
              <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment horizontal="left"/></xf>
              <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment horizontal="center"/></xf>
              <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment horizontal="right"/></xf>
            </cellXfs>
            <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
          </styleSheet>"#).unwrap();
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let mut runs = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) if text.source.is_some() => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        runs.sort_by_key(|text| text.source.as_ref().unwrap().path.clone());
        assert_eq!(runs.len(), 5);
        assert!(runs[0].origin.x.0 < runs[1].origin.x.0);
        assert!(runs[1].origin.x.0 < runs[2].origin.x.0);
        assert!((runs[0].origin.x.0 - runs[3].origin.x.0).abs() < 0.01);
        snapshots.push(runs);
    }
    let shift = snapshots[1][0].origin.x.0 - snapshots[0][0].origin.x.0;
    assert!(shift > 200.0);
    for (ltr, rtl) in snapshots[0].iter().zip(&snapshots[1]) {
        assert_eq!(ltr.text, rtl.text);
        assert_eq!(ltr.style, rtl.style);
        assert_eq!(ltr.color, rtl.color);
        assert!((rtl.origin.x.0 - ltr.origin.x.0 - shift).abs() < 0.01);
        assert_eq!(ltr.origin.y, rtl.origin.y);
    }
}

#[test]
fn rtl_worksheet_repeated_columns_and_merged_cells_keep_their_owners() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/RightToLeft.xlsx",
    );
    let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let workbook = package.workbook_part().unwrap();
    workbook.root_element_mut(&mut package).unwrap().defined_names = Some(x::DefinedNames::from_bytes(br#"
      <definedNames xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <definedName name="_xlnm.Print_Titles" localSheetId="0">RightToLeftSheet!$A:$A</definedName>
        <definedName name="_xlnm.Print_Area" localSheetId="0">RightToLeftSheet!$A$1:$F$1</definedName>
      </definedNames>"#).unwrap());
    let sheet = workbook.worksheet_parts(&package).next().unwrap();
    *sheet.root_element_mut(&mut package).unwrap() = x::Worksheet::from_bytes(br#"
      <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <sheetViews><sheetView workbookViewId="0" rightToLeft="1"/></sheetViews>
        <cols><col min="1" max="6" width="10" customWidth="1"/></cols>
        <sheetData><row r="1">
          <c r="A1" t="inlineStr"><is><t>Title</t></is></c>
          <c r="B1" t="inlineStr"><is><t>Merged</t></is></c>
          <c r="D1" t="inlineStr"><is><t>D-value</t></is></c>
          <c r="E1" t="inlineStr"><is><t>E-value</t></is></c>
          <c r="F1" t="inlineStr"><is><t>F-value</t></is></c>
        </row></sheetData>
        <mergeCells count="1"><mergeCell ref="B1:C1"/></mergeCells>
        <printOptions headings="1" gridLines="1"/>
        <pageMargins left="0.75" right="0.75" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
        <pageSetup paperSize="9" scale="100" usePrinterDefaults="0"/>
        <colBreaks count="1" manualBreakCount="1"><brk id="3" min="0" max="1048575" man="1"/></colBreaks>
      </worksheet>"#).unwrap();
    let document =
        ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
    assert_eq!(document.pages.len(), 2);
    assert_page_text_occurrences(&document, 0, "Merged", 1);
    assert_page_text_occurrences(&document, 1, "Title", 1);
    let items = page_display_items(&document, 1);
    let x = |label: &str| {
        items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text == label => Some(text.origin.x.0),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {label}"))
    };
    assert!(x("F-value") < x("E-value"));
    assert!(x("E-value") < x("D-value"));
    assert!(x("D-value") < x("Title"));
    assert!(
        x("Title") < x("1"),
        "row heading stays on the sheet's leading side"
    );
    assert!(x("F") < x("E") && x("E") < x("D"));
}

#[test]
fn boolean_cells_follow_office_ui_language_without_translating_strings() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;
    use ooxmlsdk_layout_test::corpus_file;

    for (file, language, true_text, true_count, false_text, false_count) in [
        (
            "../ClosedXML/ClosedXML.Tests/Resource/Examples/Ranges/ShiftingRanges.xlsx",
            "de-DE",
            "WAHR",
            1,
            "FALSCH",
            2,
        ),
        (
            "../ClosedXML/ClosedXML.Tests/Resource/Examples/Ranges/TransposeRangesPlus.xlsx",
            "fr-CA",
            "VRAI",
            1,
            "FAUX",
            1,
        ),
        (
            "../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/ShowCase.xlsx",
            "es-MX",
            "VERDADERO",
            0,
            "FALSO",
            2,
        ),
    ] {
        let path = corpus_file(file);
        let package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some(language.into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("en-US".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_text_occurrences(&document, 0, true_text, true_count);
        assert_page_text_occurrences(&document, 0, false_text, false_count);
    }

    let path = corpus_file("../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/CellValues.xlsx");
    let package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let document = ooxmlsdk_layout::xlsx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("es-MX".into()),
            format_locale: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_page_text_occurrences(&document, 0, "VERDADERO", 3);
    // C4 is a logical value too wide for its column; F4/G4 are text TRUE.
    let hashes = page_display_items(&document, 0)
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text)
                if text
                    .source
                    .as_ref()
                    .is_some_and(|source| source.path == [3, 2]) =>
            {
                Some(text.text.as_ref())
            }
            _ => None,
        })
        .collect::<String>();
    assert!(
        !hashes.is_empty() && hashes.chars().all(|ch| ch == '#'),
        "C4: {hashes}"
    );
    let strings = (0..document.pages.len())
        .map(|page| all_page_text(&document, page).matches("TRUE").count())
        .sum::<usize>();
    assert_eq!(strings, 2);
}

#[test]
fn spreadsheet_time_periods_keep_explicit_and_system_formats_distinct() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;
    use ooxmlsdk_layout_test::corpus_file;

    let path = corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/CellsWithTimeSpanDataTypeOrFormatting.xlsx",
    );
    let package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let document = ooxmlsdk_layout::xlsx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("fr-CA".into()),
            format_locale: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_page_text_occurrences(&document, 0, "1:30 PM", 1);
    assert_page_text_occurrences(&document, 0, "1:30:55 PM", 2);
    // B9 uses F400 and follows the host's 24-hour format; the other four
    // occurrences are explicit 24-hour numeric/text controls in this sheet.
    assert_page_text_occurrences(&document, 0, "13:30:55", 5);
    assert_page_not_contains(&document, 0, "下午");

    let path = corpus_file("chart2/qa/extras/data/xlsx/tdf98690.xlsx");
    let package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let document = ooxmlsdk_layout::xlsx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("de-DE".into()),
            format_locale: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let text = (0..document.pages.len())
        .map(|page| all_page_text(&document, page))
        .collect::<Vec<_>>()
        .join("\n");
    // These are authored AM/PM formats, not built-in time IDs.
    assert!(text.contains("06:41:00 PM"));
    assert!(text.contains("06:44:00 PM"));
}

#[test]
fn fractional_time_cells_match_office_at_tenths_and_milliseconds() {
    let times = xlsx_layout(
        "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/CellsWithTimeSpanDataTypeOrFormatting.xlsx",
    ).unwrap();
    // Built-in format 47 and an authored mm:ss.0 both print 30:55.2;
    // the third occurrence is the workbook's explanatory text control.
    assert_page_text_occurrences(&times, 0, "30:55.2", 3);
    assert_page_not_contains(&times, 0, "3055.0");
    assert_page_contains(&times, 0, "13:30:55");

    let dates = xlsx_layout("../Apache-POI/test-data/spreadsheet/DateFormatTests.xlsx").unwrap();
    // This workbook uses the 1904 date system. C21:C40 retain nine
    // milliseconds in Office rather than emitting the .000 mask literally.
    let count = (0..dates.pages.len())
        .map(|page| {
            all_page_text(&dates, page)
                .matches("01-Feb-2003 04:05:06.009")
                .count()
        })
        .sum::<usize>();
    assert_eq!(count, 20);
}

#[test]
fn expense_report_empty_date_sections_keep_zero_dates_blank() {
    let document = xlsx_layout(
        "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/spreadsheet/O12 templates/ExpenseReport_TP10073879.xltx",
    ).unwrap();
    // L4/L5 contain numeric zero with m/d/yyyy;;. Office retains the period
    // labels while leaving both dates blank; worksheet showZeros is enabled.
    assert_page_contains(&document, 0, "PAY PERIOD:");
    assert_page_contains(&document, 0, "From");
    assert_page_contains(&document, 0, "To");
    let text = all_page_text(&document, 0);
    assert_eq!(
        text.split_whitespace().filter(|word| *word == "0").count(),
        0
    );
    assert_page_not_contains(&document, 0, "1900");
}

#[test]
fn mixed_and_improper_fractions_match_office() {
    let mixed = xlsx_layout("sc/qa/unit/data/xlsx/tdf81939.xlsx").unwrap();
    assert_page_text_occurrences(&mixed, 0, "-3 14093/99532", 1);
    assert_page_not_contains(&mixed, 0, "--3");

    let odds = xlsx_layout("sc/qa/unit/data/xlsx/tdf70455.xlsx").unwrap();
    let text = all_page_text(&odds, 0);
    for expected in ["7/1", "16/1", "12/1", "10/1", "6/1", "1/4"] {
        assert_eq!(
            text.split_whitespace()
                .filter(|word| *word == expected)
                .count(),
            1,
            "{expected}"
        );
    }
}

#[test]
fn chinese_system_long_dates_match_office_in_both_date_systems() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;
    use ooxmlsdk_layout_test::corpus_file;

    for (path, ui_language, expected) in [
        (
            "sc/qa/unit/data/xlsx/tdf165180_date1904.xlsx",
            "fr-FR",
            &["1904年3月1日", "2005年9月12日", "1976年8月26日"][..],
        ),
        (
            "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/CellsWithDateTimeWithLocalePrefix.xlsx",
            "ja-JP",
            &["2019年1月21日"][..],
        ),
        (
            "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/CellsWithDateTimeDataTypeOrFormatting.xlsx",
            "de-DE",
            &["2012年3月14日"][..],
        ),
    ] {
        let path = corpus_file(path);
        let package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some(ui_language.into()),
                format_locale: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        for expected in expected {
            assert_page_text_occurrences(&document, 0, expected, 1);
        }
        assert_page_not_contains(&document, 0, "星期");
    }
}

#[test]
fn optional_zero_placeholders_match_office_transaction_values() {
    let document = xlsx_layout("../Apache-POI/test-data/spreadsheet/52348.xlsx").unwrap();
    let text = all_page_text(&document, 0);
    // Debit L8/L9 use # and are blank; credit O4:O9 uses #0.#,
    // which keeps the decimal separator but omits insignificant fractions.
    for (expected, count) in [("0", 0), ("0.", 4), ("1000.", 2), ("600.3", 1)] {
        assert_eq!(
            text.split_whitespace()
                .filter(|word| *word == expected)
                .count(),
            count,
            "{expected}"
        );
    }
}

#[test]
fn conditional_number_formats_choose_office_kilo_and_mega_sections() {
    let document = xlsx_layout("../Apache-POI/test-data/spreadsheet/FormatKM.xlsx").unwrap();
    // Each expected value appears twice: a numeric cell and the workbook's
    // authored text control. Both pages were confirmed in the current Office PDF.
    for (page, expected) in [
        (0, ["1K", "102K", "1M", "1021M"]),
        (1, ["1.020", "1.021K", "1.021M", "1021.021M"]),
    ] {
        let text = all_page_text(&document, page);
        for expected in expected {
            // Compare whole values: 1M is also a suffix of 1021M.
            assert_eq!(
                text.split_whitespace()
                    .filter(|word| *word == expected)
                    .count(),
                2,
                "{page}: {expected}"
            );
        }
    }
}

#[test]
fn explicit_date_locale_prefixes_preserve_office_month_and_weekday_names() {
    let document = xlsx_layout(
        "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/CellsWithDateTimeWithLocalePrefix.xlsx",
    )
    .unwrap();
    // Office uses the cell's en-AU marker for A3 even though the campaign
    // format locale is zh-CN. A2/A4 retain their en-US/en-GB date pictures.
    for text in ["21-Jan-19", "Monday, 21 January 2019", "21 Jan 2019"] {
        assert_page_text_occurrences(&document, 0, text, 1);
    }
}

#[test]
fn elapsed_hours_above_one_day_match_office() {
    let document =
        xlsx_layout("../ClosedXML/ClosedXML.Tests/Resource/Examples/Misc/DataTypes.xlsx").unwrap();
    // The current Office PDF prints numeric C7=1.40650462962963 with
    // built-in format 46 as cumulative hours, without brackets or day wrap.
    assert_page_text_occurrences(&document, 0, "33:45:22", 1);
}

#[test]
fn negative_prefixed_numbers_keep_automatic_and_authored_signs_separate() {
    for name in ["Input", "Output"] {
        let document = xlsx_layout(&format!(
            "../ClosedXML/ClosedXML.Tests/Resource/Other/NumberFormats/NonSequentialNumberFormatsIds-{name}.xlsx"
        )).unwrap();
        assert_page_text_occurrences(&document, 0, "-AMT 12.50", 1);
        assert_page_text_occurrences(&document, 0, "(NEG 5.20)", 1);
        assert_page_not_contains(&document, 0, "-(NEG 5.20)");
        if name == "Output" {
            assert_page_text_occurrences(&document, 0, "P 1.20", 1);
            assert_page_text_occurrences(&document, 0, "N 1.20", 1);
            assert_page_not_contains(&document, 0, "-N 1.20");
        }
    }
}

#[test]
fn decimal_half_ties_match_office_without_rounding_up_nearby_values() {
    let document = xlsx_layout("../Apache-POI/test-data/spreadsheet/decimal-format.xlsx").unwrap();
    // Current Office PDF: A1=1.005 prints 1.01; A2=1.004 prints 1.00.
    assert_eq!(document.pages.len(), 1);
    assert_page_text_occurrences(&document, 0, "1.01", 1);
    assert_page_text_occurrences(&document, 0, "1.00", 1);
}

#[test]
fn negative_number_format_colors_reach_the_printed_text() {
    // Current Office PDF has three red -€130.00 values; positive and zero
    // values in the same format remain black.
    let document = xlsx_layout("sc/qa/unit/data/xlsx/tdf70455.xlsx").unwrap();
    assert_page_text_occurrences(&document, 0, "-€130.00", 3);
    let red = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    let red_values = page_display_items(&document, 0)
        .iter()
        .filter(|item| {
            matches!(
                item, DisplayItem::Text(text) if text.text == "-€130.00" && text.color == red
            )
        })
        .count();
    assert_eq!(red_values, 3);
    for text in ["€910.00", "€260.00", "€0.00"] {
        assert_text_color(
            &document,
            text,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
    }
}

#[test]
fn scientific_cells_use_exponent_notation_instead_of_overflow_hashes() {
    let document = xlsx_layout("../Apache-POI/test-data/spreadsheet/49273.xlsx").unwrap();
    // Current Office PDF prints both numeric cells using built-in format 11.
    // The campaign selects source page 2; this helper lays out the whole book.
    // Isolate these cells from the fixture's separate DBNum and date gaps.
    let occurrences = (0..document.pages.len())
        .map(|page| all_page_text(&document, page).matches("1.03E+08").count())
        .sum::<usize>();
    assert_eq!(occurrences, 2);
}

#[test]
fn missing_workbook_theme_preserves_office_table_colors() {
    // The current Office PDF uses a white header on accent1 (#156082).
    // The package has no theme part; TableStyleMedium2 still resolves colors.
    let document =
        xlsx_layout("sc/qa/unit/data/xlsx/tdf167689_xmlMaps_and_xmlColumnPr.xlsx").unwrap();
    for header in ["Code", "Description"] {
        assert_text_color(
            &document,
            header,
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        );
    }
    for body in ["001", "Test 001"] {
        assert_text_color(
            &document,
            body,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
    }
    let accent = Fill::Solid(Color {
        r: 0x15,
        g: 0x60,
        b: 0x82,
        a: 255,
    });
    assert!(
        page_display_items(&document, 0)
            .iter()
            .any(|item| match item {
                DisplayItem::Rect(rect) => rect.fill == accent,
                DisplayItem::Path(path) => path.fill == accent,
                _ => false,
            }),
        "missing Office default accent1 table header"
    );
}

#[test]
fn manual_excel_legend_insets_preserve_visible_entries_and_positions() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::simple_type::Coordinate32Value;
    use ooxmlsdk_layout::LayoutOptions;

    let path =
        ooxmlsdk_layout_test::corpus_file("chart2/qa/extras/data/xlsx/legend_manual_layout.xlsx");
    let mut original = None;
    // Excel 20326 prints identical labels for all five body-inset controls.
    for insets in [
        None,
        Some([0, 0, 0, 0]),
        Some([91440, 91440, 45720, 45720]),
        Some([0, 0, 45720, 45720]),
        Some([91440, 91440, 0, 0]),
    ] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        if let Some([left, right, top, bottom]) = insets {
            let workbook = package.workbook_part().unwrap();
            let drawings = workbook
                .worksheet_parts(&package)
                .filter_map(|sheet| sheet.drawings_part(&package))
                .collect::<Vec<_>>();
            let charts = drawings
                .iter()
                .flat_map(|drawing| drawing.chart_parts(&package))
                .collect::<Vec<_>>();
            assert_eq!(charts.len(), 1);
            let space = charts[0].root_element_mut(&mut package).unwrap();
            let body = &mut space
                .chart
                .legend
                .as_mut()
                .unwrap()
                .text_properties
                .as_mut()
                .unwrap()
                .body_properties;
            body.left_inset = Some(Coordinate32Value::Emu(left));
            body.right_inset = Some(Coordinate32Value::Emu(right));
            body.top_inset = Some(Coordinate32Value::Emu(top));
            body.bottom_inset = Some(Coordinate32Value::Emu(bottom));
        }
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("ko-KR".into()),
                format_locale: Some("zh-CN".into()),
                ..LayoutOptions::default()
            },
        )
        .unwrap();
        let entries = page_display_items(&document, 1)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(run) if matches!(run.text.as_ref(), "col1" | "col2") => {
                    Some((run.text.to_string(), run.origin))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|(text, _)| text == "col1"));
        assert!(entries.iter().any(|(text, _)| text == "col2"));
        if let Some(expected) = &original {
            assert_eq!(&entries, expected);
        } else {
            original = Some(entries);
        }
    }
}

#[test]
fn automatic_chart_title_uses_ui_instead_of_format_locale() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;

    let path =
        ooxmlsdk_layout_test::corpus_file("chart2/qa/extras/data/xlsx/legend_manual_layout.xlsx");
    let package = SpreadsheetDocument::new_from_file(&path).unwrap();
    for (ui_language, format_locale, expected) in [
        ("ko-KR", "en-US", "차트 제목"),
        ("en-US", "ko-KR", "Chart Title"),
        ("fr-CA", "en-US", "Titre du graphique"),
        ("es-MX", "zh-CN", "Título del gráfico"),
        ("ja-JP", "fr-CA", "グラフ タイトル"),
    ] {
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some(ui_language.into()),
                format_locale: Some(format_locale.into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_contains(&document, 1, expected);
        assert_page_contains(&document, 1, "col1");
        assert_page_contains(&document, 1, "col2");
    }
}

#[test]
fn text_conditional_formats_share_spreadsheet_function_semantics() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    // Reuse the existing red-fill dxf. The current CFEndsWith Office PDF
    // supplies the unmodified control; vary only worksheet content in memory
    // to exercise ECMA-376 §18.18.12's explicit function equivalences.
    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/CFEndsWith.xlsx",
    );
    let cases: &[(&str, &str, &[usize])] = &[
        ("containsText", "ll", &[1, 2, 3, 4, 5]),
        ("notContainsText", "ll", &[6, 7, 8, 9]),
        ("beginsWith", "he", &[1, 5]),
        ("endsWith", "LO", &[1, 4, 5]),
        ("containsText", "h~*llo", &[4]),
        ("containsText", "he\"llo", &[5]),
        ("containsText", "123", &[8]),
        ("containsText", "#DIV/0!", &[]),
        ("notContainsText", "#DIV/0!", &[1, 2, 3, 4, 5, 6, 7, 8, 9]),
        ("containsBlanks", "", &[6]),
        ("notContainsBlanks", "", &[1, 2, 3, 4, 5, 7, 8]),
    ];
    for &(kind, needle, expected_rows) in cases {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        let needle = needle.replace('&', "&amp;").replace('"', "&quot;");
        let mut rows = String::new();
        for (index, value) in [
            "HELLO", "yellow", " heLLo ", "h*llo", "he\"llo", "   ", "\u{a0}",
        ]
        .iter()
        .enumerate()
        {
            let row = index + 1;
            rows.push_str(&format!(r#"<row r="{row}"><c r="A{row}" t="inlineStr"><is><t xml:space="preserve">{value}</t></is></c><c r="B{row}" t="inlineStr"><is><t>row{row}</t></is></c></row>"#));
        }
        rows.push_str(r#"<row r="8"><c r="A8"><v>123</v></c><c r="B8" t="inlineStr"><is><t>row8</t></is></c></row><row r="9"><c r="A9" t="e"><v>#DIV/0!</v></c><c r="B9" t="inlineStr"><is><t>row9</t></is></c></row>"#);
        let xml = format!(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="2" width="20" customWidth="1"/></cols><sheetData>{rows}</sheetData><conditionalFormatting sqref="A1:A9"><cfRule type="{kind}" dxfId="0" priority="1" text="{needle}"/></conditionalFormatting></worksheet>"#
        );
        *sheet.root_element_mut(&mut package).unwrap() =
            x::Worksheet::from_bytes(xml.as_bytes()).unwrap();
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let fills = items
            .iter()
            .filter_map(|item| {
                let (fill, bounds) = match item {
                    DisplayItem::Rect(rect) => (&rect.fill, rect.bounds),
                    DisplayItem::Path(path) => (&path.fill, path.bounds),
                    _ => return None,
                };
                (*fill
                    == Fill::Solid(Color {
                        r: 255,
                        g: 0,
                        b: 0,
                        a: 255,
                    }))
                .then_some(bounds)
            })
            .collect::<Vec<_>>();
        assert_eq!(fills.len(), expected_rows.len(), "{kind}: {needle}");
        for row in 1..=9 {
            let label = format!("row{row}");
            let y = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text.as_ref() == label => {
                        // Use the line interior: the top origin can lie exactly
                        // on the shared boundary of two adjacent row fills.
                        Some(text.origin.y.0 + text.line_height.0 * 0.5)
                    }
                    _ => None,
                })
                .expect("row label remains visible");
            let highlighted = fills.iter().any(|bounds| {
                y >= bounds.origin.y.0 && y < bounds.origin.y.0 + bounds.size.height.0
            });
            assert_eq!(
                highlighted,
                expected_rows.contains(&row),
                "{kind}: {needle}, row {row}"
            );
        }
    }
}

#[test]
fn header_footer_clock_uses_the_supplied_time_and_format_locale() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::options::{FieldUpdateDateTime, LayoutOptions};

    let path = ooxmlsdk_layout_test::corpus_file(
        "../Apache-POI/test-data/spreadsheet/customIndexedColors.xlsx",
    );
    let package = SpreadsheetDocument::new_from_file(&path).unwrap();
    // This fixture's current Office PDF has 2026/9/11 - - 16:04 in its
    // header. Both fields follow the supplied civil clock and format locale,
    // regardless of the UI language or the date on the test host.
    for (ui_language, clock, expected) in [
        (
            "de-DE",
            FieldUpdateDateTime {
                year: 2026,
                month: 9,
                day: 11,
                hour: 16,
                minute: 4,
                second: 59,
            },
            "2026/9/11 - - 16:04",
        ),
        (
            "ja-JP",
            FieldUpdateDateTime {
                year: 2001,
                month: 2,
                day: 3,
                hour: 13,
                minute: 7,
                second: 42,
            },
            "2001/2/3 - - 13:07",
        ),
    ] {
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                source_file_name: Some("customIndexedColors.xlsx".into()),
                ui_language: Some(ui_language.into()),
                format_locale: Some("zh-CN".into()),
                field_update_datetime: Some(clock),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            page_display_items(&document, 0).iter().any(|item| matches!(
                item, DisplayItem::Text(text) if text.text == expected
            )),
            "expected exact short-time header {expected}"
        );
        assert_page_not_contains(&document, 0, "D - - T");
    }
}

#[test]
fn chart_without_series_keeps_its_frame_without_falling_back_to_title_text() {
    let document = xlsx_layout("sc/qa/unit/data/xlsx/tdf107586.xlsx").unwrap();
    let items = page_display_items(&document, 0);
    assert!(
        items
            .iter()
            .all(|item| !matches!(item, DisplayItem::Text(_)))
    );
    assert!(
        items.iter().any(|item| match item {
            DisplayItem::Path(path) =>
                path.closed
                    && path.bounds.size.width.0 > 600.0
                    && path.bounds.size.height.0 > 400.0
                    && path.fill
                        == Fill::Solid(Color {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 255
                        })
                    && path.stroke.is_some(),
            _ => false,
        }),
        "the current Office PDF retains the large empty chart-area frame"
    );
}

#[test]
fn error_conditional_formats_fill_only_the_matching_rows() {
    let red = Fill::Solid(Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    });
    for (fixture, highlighted_rows) in [("CFNotError.xlsx", [0, 3]), ("CFIsError.xlsx", [1, 2])] {
        let document = xlsx_layout(&format!(
            "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/{fixture}"
        ))
        .unwrap();
        let items = page_display_items(&document, 0);
        let fills = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Rect(rect) if rect.fill == red => Some(rect.bounds),
                DisplayItem::Path(path) if path.fill == red => Some(path.bounds),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(fills.len(), 2, "{fixture}: expected exactly two red cells");
        // The current CFNotError Office PDF fills Hello/Holl (A1/A4),
        // leaving the two evaluated #DIV/0! cells unfilled. ISERROR is its
        // complementary rule, as defined by ECMA-376 §18.18.12.
        for row in 0..4 {
            let text = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text)
                        if text
                            .source
                            .as_ref()
                            .is_some_and(|source| source.path.as_slice() == [row, 0]) =>
                    {
                        Some(text)
                    }
                    _ => None,
                })
                .expect("all four source rows must be rendered");
            if row == 1 || row == 2 {
                assert_eq!(text.text.as_ref(), "#DIV/0!");
            }
            // Office's fill clip and logical text origin have separate
            // device insets. Check the interior of the line, not its top edge.
            let midline_y = text.origin.y.0 + text.line_height.0 * 0.5;
            let filled = fills.iter().any(|bounds| {
                midline_y >= bounds.origin.y.0
                    && midline_y < bounds.origin.y.0 + bounds.size.height.0
            });
            assert_eq!(
                filled,
                highlighted_rows.contains(&row),
                "{fixture}: row {}",
                row + 1
            );
        }
    }
}

#[test]
fn cell_reading_order_moves_neutral_punctuation_without_reversing_english() {
    let document = xlsx_layout("sc/qa/unit/data/xlsx/writingMode.xlsx").unwrap();
    let items = page_display_items(&document, 0);
    let row_text = |row| {
        items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text)
                    if text
                        .source
                        .as_ref()
                        .is_some_and(|source| source.path.as_slice() == [row, 0]) =>
                {
                    Some(text.text.as_ref())
                }
                _ => None,
            })
            .collect::<String>()
    };
    // A3 uses contextual reading order, A4 explicitly uses LTR, and A5
    // explicitly uses RTL. The current Office PDF places only A5's period
    // before the English phrase; the phrase itself stays left-to-right.
    assert_eq!(row_text(2), "English (Yes).");
    assert_eq!(row_text(3), "English(Yes).");
    assert_eq!(row_text(4), ".English (Yes)");
}

#[test]
fn explicit_print_area_does_not_expand_to_complete_a_merged_cell() {
    let document = xlsx_layout("sc/qa/unit/data/xlsx/tdf100034.xlsx").unwrap();
    // A1:G1 crosses the explicit A1:F5 print area. Office keeps the title,
    // B4 and C5, but excludes the unrelated notes in G3 and G5 from paint.
    assert_page_contains(&document, 0, "Budget");
    assert_page_contains(&document, 0, "Ordinary Income/Expense");
    assert_page_contains(&document, 0, "Frobnostication");
    let print_pages = debug_shapes(&document, "xlsx_print_page");
    let page = print_pages
        .iter()
        .find(|page| page.page_index == 0)
        .unwrap();
    assert_eq!(
        ooxmlsdk_layout_test::debug_integer_property(page, "end_column"),
        Some(6)
    );
    // A neighbouring cell may survive as clipped scan context. It must not
    // gain an ordinary paint rectangle by widening the explicit print area.
    for item in page_display_items(&document, 0) {
        let DisplayItem::Text(text) = item else {
            continue;
        };
        if text.text.contains("Notes") || text.text.contains("Why wont this print") {
            let clip = text.paint_clip.expect("out-of-area note must be clipped");
            assert!(clip.origin.x.0 + clip.size.width.0 <= text.origin.x.0 + 0.1);
        }
    }
}

#[derive(Clone, Copy)]
struct XlsxCase {
    name: &'static str,
    source: &'static str,
    file: &'static str,
    page_count: usize,
    contains: &'static [PageText],
    contains_any: &'static [PageAnyText],
    not_contains: &'static [PageText],
    occurrences: &'static [PageTextCount],
    page_sizes: &'static [PageSize],
    image_counts: &'static [PageCount],
    link_targets: &'static [&'static str],
    path_minimums: &'static [PageCount],
    font_sizes: &'static [TextFontSize],
    colors: &'static [TextColor],
}

#[derive(Clone, Copy)]
struct PageText {
    page: usize,
    text: &'static str,
}

#[derive(Clone, Copy)]
struct PageAnyText {
    page: usize,
    alternatives: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct PageTextCount {
    page: usize,
    text: &'static str,
    count: usize,
}

#[derive(Clone, Copy)]
struct PageSize {
    page: usize,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct PageCount {
    page: usize,
    count: usize,
}

#[derive(Clone, Copy)]
struct TextFontSize {
    text: &'static str,
    size: f32,
}

#[derive(Clone, Copy)]
struct TextColor {
    text: &'static str,
    color: Color,
}

macro_rules! pt {
    ($page:expr, $text:expr) => {
        PageText {
            page: $page,
            text: $text,
        }
    };
}

macro_rules! any {
    ($page:expr, [$($text:expr),+ $(,)?]) => {
        PageAnyText {
            page: $page,
            alternatives: &[$($text),+],
        }
    };
}

macro_rules! count {
    ($page:expr, $text:expr, $count:expr) => {
        PageTextCount {
            page: $page,
            text: $text,
            count: $count,
        }
    };
}

macro_rules! page_count {
    ($page:expr, $count:expr) => {
        PageCount {
            page: $page,
            count: $count,
        }
    };
}

macro_rules! size {
    ($page:expr, $width:expr, $height:expr) => {
        PageSize {
            page: $page,
            width: $width,
            height: $height,
        }
    };
}

macro_rules! font {
    ($text:expr, $size:expr) => {
        TextFontSize {
            text: $text,
            size: $size,
        }
    };
}

macro_rules! color {
    ($text:expr, $r:expr, $g:expr, $b:expr, $a:expr) => {
        TextColor {
            text: $text,
            color: Color {
                r: $r,
                g: $g,
                b: $b,
                a: $a,
            },
        }
    };
}

macro_rules! case {
    (
        $name:ident,
        source: $source:expr,
        file: $file:expr,
        pages: $pages:expr
        $(, contains: [$($contains:expr),* $(,)?])?
        $(, contains_any: [$($contains_any:expr),* $(,)?])?
        $(, not_contains: [$($not_contains:expr),* $(,)?])?
        $(, occurrences: [$($occurrences:expr),* $(,)?])?
        $(, page_sizes: [$($page_sizes:expr),* $(,)?])?
        $(, image_counts: [$($image_counts:expr),* $(,)?])?
        $(, link_targets: [$($link_targets:expr),* $(,)?])?
        $(, path_minimums: [$($path_minimums:expr),* $(,)?])?
        $(, font_sizes: [$($font_sizes:expr),* $(,)?])?
        $(, colors: [$($colors:expr),* $(,)?])?
        $(,)?
    ) => {
        XlsxCase {
            name: stringify!($name),
            source: $source,
            file: $file,
            page_count: $pages,
            contains: &[$($($contains),*)?],
            contains_any: &[$($($contains_any),*)?],
            not_contains: &[$($($not_contains),*)?],
            occurrences: &[$($($occurrences),*)?],
            page_sizes: &[$($($page_sizes),*)?],
            image_counts: &[$($($image_counts),*)?],
            link_targets: &[$($($link_targets),*)?],
            path_minimums: &[$($($path_minimums),*)?],
            font_sizes: &[$($($font_sizes),*)?],
            colors: &[$($($colors),*)?],
        }
    };
}

const CASES: &[XlsxCase] = &[
    case!(
        tdf123026_optimal_row_height,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf123026_optimalRowHeight",
        file: "sc/qa/unit/data/xlsx/tdf123026_optimalRowHeight.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Sales Summary Report"),
            pt!(0, "Single level semi attached"),
            pt!(0, "Reflects $3,526/sqm."),
        ],
    ),
    case!(
        tdf159581_optimal_row_height,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf159581_optimalRowHeight",
        file: "sc/qa/unit/data/xlsx/tdf159581_optimalRowHeight.xlsx",
        pages: 2,
        contains: [
            pt!(0, "One honking big, optimal cell size"),
            pt!(0, "Should not affect other sheets"),
            pt!(1, "still optimally sized row heights on sheet2"),
        ],
    ),
    case!(
        tdf144642_calc_saved_row_height,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf144642_RowHeightRounding_saveByCalc",
        file: "sc/qa/unit/data/xlsx/tdf144642_RowHeight_10mm_SavedByCalc.xlsx",
        pages: 2,
        contains: [pt!(0, "25 ___"), pt!(1, "26 ___")],
    ),
    case!(
        tdf144642_excel_saved_row_height,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf144642_RowHeightRounding_saveByExcel",
        file: "sc/qa/unit/data/xlsx/tdf144642_RowHeight_28.35pt_SavedByExcel.xlsx",
        // Default layout options retain the stored Latin Calibri font. The
        // Office explicit-Calibri control also prints two pages; the configured
        // zh-CN theme selects DengXian and is a separate golden obligation.
        pages: 2,
        contains: [pt!(1, "26 ___")],
    ),
    case!(
        tdf145129_default_row_height,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf145129_DefaultRowHeightRounding",
        file: "sc/qa/unit/data/xlsx/tdf145129_DefaultRowHeight_28.35pt_SavedByExcel.xlsx",
        // Office with the same explicit Latin Calibri font prints three pages.
        pages: 3,
        contains: [pt!(0, "1"), pt!(1, "2"), pt!(2, "28")],
    ),
    case!(
        misc_row_heights,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testMiscRowHeightExport",
        file: "sc/qa/unit/data/xlsx/miscrowheights.xlsx",
        pages: 1,
        occurrences: [count!(0, "30", 6), count!(0, "50", 4)],
    ),
    case!(
        seconds_without_truncate,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testSecondsWithoutTruncateAndDecimals",
        file: "sc/qa/unit/data/xlsx/seconds-without-truncate-and-decimals.xlsx",
        pages: 1,
        contains: [pt!(0, "271433.61")],
        page_sizes: [size!(0, 595.28, 841.89)],
    ),
    case!(
        embedded_text_in_decimal,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testEmbeddedTextInDecimal",
        file: "sc/qa/unit/data/xlsx/embedded-text-in-decimal.xlsx",
        pages: 1,
        contains: [pt!(0, "6,543,210.123 456 78")],
    ),
    case!(
        cell_borders,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testCellBordersXLSX",
        file: "sc/qa/unit/data/xlsx/cell-borders.xlsx",
        pages: 3,
        contains: [
            pt!(0, "hair"),
            pt!(0, "mediumDashDotDot"),
            pt!(0, "double"),
            pt!(1, "Screenshot of how the borders look in Excel XP"),
        ],
    ),
    case!(
        tdf136721_paper_size,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf136721_paper_size",
        file: "sc/qa/unit/data/xlsx/tdf136721_letter_sized_paper.xlsx",
        pages: 1,
        contains: [pt!(0, "Start"), pt!(0, "End")],
        page_sizes: [size!(0, 419.56, 297.64)],
    ),
    case!(
        tdf166724_cell_anchor,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf166724_cellAnchor",
        file: "sc/qa/unit/data/xlsx/tdf166724_cellAnchor.xlsx",
        pages: 1,
        contains: [pt!(0, "B3 checkBox")],
    ),
    case!(
        image_hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf91634XLSX",
        file: "sc/qa/unit/data/xlsx/image_hyperlink.xlsx",
        pages: 2,
        not_contains: [pt!(0, "https://www.google.com/")],
        image_counts: [page_count!(0, 0), page_count!(1, 1)],
        link_targets: ["https://www.google.com/"],
    ),
    case!(
        hidden_shape,
        source: "Excel configured export and 2026-09-12 recovery: no printable pages",
        file: "sc/qa/unit/data/xlsx/hiddenShape.xlsx",
        pages: 0,
    ),
    case!(
        tdf169496_hidden_graphic,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testtdf169496_hidden_graphic",
        file: "sc/qa/unit/data/xlsx/tdf169496_hidden_graphic.xlsx",
        pages: 1,
        image_counts: [page_count!(0, 1)],
    ),
    case!(
        hyperlinks,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testHyperlinksXLSX",
        file: "sc/qa/unit/data/xlsx/hyperlinks.xlsx",
        pages: 6,
        contains: [
            pt!(0, "10:ABC10"),
            pt!(0, "10:ABC11"),
            pt!(0, "10:ABC12"),
            pt!(1, "Invalid Value"),
        ],
    ),
    case!(
        preserve_whitespace,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testPreserveTextWhitespaceXLSX",
        file: "sc/qa/unit/data/xlsx/preserve-whitespace.xlsx",
        pages: 1,
        contains: [pt!(0, "abc")],
        page_sizes: [size!(0, 595.28, 841.89)],
    ),
    case!(
        preserve_space,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testPreserveTextWhitespace2XLSX",
        file: "sc/qa/unit/data/xlsx/preserve_space.xlsx",
        pages: 1,
        contains: [pt!(0, "abc 123456 456")],
        page_sizes: [size!(0, 595.28, 841.89)],
    ),
    case!(
        escape_unicode,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testEscapedUnicodeXLSX",
        file: "sc/qa/unit/data/xlsx/escape-unicode.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Line 1"),
            pt!(0, "Line 2"),
            pt!(0, "Line 3"),
            pt!(0, "Line 4"),
        ],
        not_contains: [pt!(0, "_x000D_")],
    ),
    case!(
        cell_multi_line,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testSingleLine_xlsx",
        file: "sc/qa/unit/data/xlsx/cell-multi-line.xlsx",
        pages: 1,
        contains: [pt!(0, "Line1Line2Line3"), pt!(0, "Line1 Line2 Line3")],
    ),
    case!(
        check_boolean,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testBooleanFormatXLSX",
        file: "sc/qa/unit/data/xlsx/check-boolean.xlsx",
        pages: 1,
        occurrences: [count!(0, "TRUE", 2)],
        page_sizes: [size!(0, 595.28, 841.89)],
    ),
    case!(
        cell_value,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testCellValueXLSX",
        file: "sc/qa/unit/data/xlsx/cell-value.xlsx",
        pages: 8,
        contains: [
            pt!(0, "-2012"),
            pt!(0, "-3.14"),
            pt!(0, "Hello, Calc!"),
            pt!(0, "Calc is the spreadsheet program you've always needed."),
        ],
    ),
    case!(
        font_size,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testFontSizeXLSX",
        file: "sc/qa/unit/data/xlsx/fontSize.xlsx",
        pages: 1,
        contains: [pt!(0, "sardfasef")],
        font_sizes: [font!("sardfasef", 18.0)],
    ),
    case!(
        text_color,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetRunParagraphPropertyXLSX",
        file: "sc/qa/unit/data/xlsx/TextColor.xlsx",
        pages: 1,
        contains: [pt!(0, "Red Green")],
        colors: [color!("Red", 255, 0, 0, 255)],
    ),
    case!(
        underline_color,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testTextUnderlineColorXLSX",
        file: "sc/qa/unit/data/xlsx/underlineColor.xlsx",
        pages: 2,
        occurrences: [count!(0, "Text Box", 2)],
    ),
    case!(
        strike_through,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testEditEngStrikeThroughXLSX",
        file: "sc/qa/unit/data/xlsx/strike-through.xlsx",
        pages: 1,
        contains: [pt!(0, "this is strike through this not")],
        page_sizes: [size!(0, 595.28, 841.89)],
    ),
    case!(
        hidden_sheets,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testHiddenSheetsXLSX",
        file: "sc/qa/unit/data/xlsx/hidden_sheets.xlsx",
        pages: 0,
    ),
    case!(
        tdf121715_header_footer,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf121715_FirstPageHeaderFooterXLSX",
        file: "sc/qa/unit/data/xlsx/tdf121715.xlsx",
        pages: 2,
        contains: [
            pt!(0, "First Page Header"),
            pt!(0, "First Page Footer"),
            pt!(1, "Even Header"),
            pt!(1, "Even Footer"),
        ],
    ),
    case!(
        tdf134459_header_footer_color,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf134459_HeaderFooterColorXLSX",
        file: "sc/qa/unit/data/xlsx/tdf134459_HeaderFooterColor.xlsx",
        pages: 0,
    ),
    case!(
        tdf134817_header_footer_sections,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf134817_HeaderFooterTextWith2SectionXLSX",
        file: "sc/qa/unit/data/xlsx/tdf134817_HeaderFooterTextWith2Section.xlsx",
        pages: 0,
    ),
    case!(
        writing_mode,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTextDirectionXLSX",
        file: "sc/qa/unit/data/xlsx/writingMode.xlsx",
        pages: 1,
        contains: [pt!(0, "English (Yes)."), pt!(0, "English(Yes).")],
    ),
    case!(
        hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testHyperlinkXLSX",
        file: "sc/qa/unit/data/xlsx/hyperlink.xlsx",
        pages: 1,
        contains: [pt!(0, ">")],
        link_targets: ["#Sheet2!A1"],
    ),
    case!(
        textbox_hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetTextBoxHyperlinkXLSX",
        file: "sc/qa/unit/data/xlsx/textbox-hyperlink.xlsx",
        pages: 2,
        contains: [pt!(0, "text")],
    ),
    case!(
        chart_hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf123645XLSX",
        file: "sc/qa/unit/data/xlsx/chart_hyperlink.xlsx",
        pages: 3,
        contains: [pt!(0, "Chart Title"), pt!(1, "Series1")],
        link_targets: [
            "file:///C:/TEMP/test.xlsx",
            "#Sheet2!A1",
            "https://bugs.documentfoundation.org/show_bug.cgi?id=123645",
        ],
    ),
    case!(
        table_style,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTotalRowToggle",
        file: "sc/qa/unit/data/xlsx/TableStyleTest.xlsx",
        pages: 1,
        contains: [pt!(0, "A B C"), pt!(0, "Total 3 2.75 Ft")],
    ),
    case!(
        totals_row_function,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTotalsRowFunction",
        file: "sc/qa/unit/data/xlsx/totalsRowFunction.xlsx",
        // Without a document-language default the stored Century Gothic
        // remains active, matching the two-page explicit-font Office control.
        pages: 2,
        contains: [pt!(0, "PRESENT PLANNER"), pt!(0, "Total £350.00")],
        page_sizes: [size!(0, 841.89, 595.28)],
    ),
    case!(
        totals_row_shown,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTotalsRowShown",
        file: "sc/qa/unit/data/xlsx/totalsRowShown.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Desc Corn Hay Soy"),
            pt!(0, "Price $ 1.00 $ 2.00 $ 3.00"),
            pt!(0, "Unit ton bushel pound"),
        ],
    ),
    case!(
        hidden_button,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testAutofilterHiddenButton",
        file: "sc/qa/unit/data/xlsx/hiddenButton.xlsx",
        pages: 1,
        contains: [pt!(0, "Col 1 Col 2 Col 3 Col 4 Col 5")],
    ),
    case!(
        autofilter_show_button,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testAutofilterShowButton",
        file: "sc/qa/unit/data/xlsx/autofilterShowButton.xlsx",
        pages: 2,
        contains: [pt!(0, "a b c"), pt!(0, "DDD"), pt!(0, "g h"), pt!(0, "III"), pt!(0, "1 def example"), pt!(1, "III")],
    ),
    case!(
        date_autofilter,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testDateAutofilterXLSX",
        file: "sc/qa/unit/data/xlsx/dateAutofilter.xlsx",
        pages: 1,
        contains: [pt!(0, "ID Date"), pt!(0, "one 3/2/2017"), pt!(0, "three 10/1/2014")],
    ),
    case!(
        autofilter_colors,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testAutofilterColorsOOXML",
        file: "sc/qa/unit/data/xlsx/autofilter-colors.xlsx",
        pages: 1,
        contains: [pt!(0, "2 2 2"), pt!(0, "3 3 3")],
        contains_any: [any!(0, ["BackgroundForeground Both", "Background Foreground Both"])],
        not_contains: [pt!(0, "1 1 1")],
    ),
    case!(
        autofilter_colors_fg,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testAutofilterColorsOOXML2",
        file: "sc/qa/unit/data/xlsx/autofilter-colors-fg.xlsx",
        pages: 1,
        contains: [pt!(0, "1 1 1"), pt!(0, "2 2 2"), pt!(0, "3 3 3")],
        contains_any: [any!(0, ["BackgroundForeground Both", "Background Foreground Both"])],
    ),
    case!(
        tdf130104_indent,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf130104_XLSXIndent",
        file: "sc/qa/unit/data/xlsx/tdf130104_indent.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Indent by 0"),
            pt!(0, "Indent by 5"),
            pt!(0, "Indent by 10"),
            pt!(0, "缩进"),
            pt!(0, "0"),
            pt!(0, "5"),
            pt!(0, "10"),
        ],
    ),
    case!(
        tdf134826_header_font_style,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testHeaderFontStyleXLSX",
        file: "sc/qa/unit/data/xlsx/tdf134826.xlsx",
        pages: 0,
    ),
    case!(
        tdf151755_styles,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf151755_stylesLostOnXLSXExport",
        file: "sc/qa/unit/data/xlsx/tdf151755_stylesLostOnXLSXExport.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Daily Calendar"), pt!(0, "Date"),
            pt!(0, "Time Appointment To Do Errands Calls"),
            pt!(0, "07:00:00"),
            pt!(0, "18:30:00"),
        ],
    ),
    case!(
        tdf152581_bordercolor,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf152581_bordercolorNotExportedToXLSX",
        file: "sc/qa/unit/data/xlsx/tdf152581_bordercolorNotExportedToXLSX.xlsx",
        pages: 1,
        contains: [pt!(0, "test"), pt!(0, "x")],
    ),
    case!(
        checkbox_form_control,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testCheckboxFormControlXlsxExport",
        file: "sc/qa/unit/data/xlsx/checkbox-form-control.xlsx",
        pages: 1,
        contains: [pt!(0, "1"), pt!(0, "Check Box 1")],
    ),
    case!(
        radio_buttons,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testActiveXOptionButtonGroup",
        file: "sc/qa/unit/data/xlsx/tdf111980_radioButtons.xlsx",
        pages: 2,
        contains: [
            pt!(1, "Group Box 7"),
            pt!(0, "Form button1"),
            pt!(0, "Form button2"),
            pt!(1, "Form groupbox1"), pt!(1, "Form groupbox2"), pt!(1, "Form outside groupbox3"),
        ],
        // ActiveX captions belong to the EMF previews and are tested in PDF output.
        image_counts: [page_count!(0, 6), page_count!(1, 5)],
    ),
    case!(
        tdf153767,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf153767",
        file: "sc/qa/unit/data/xlsx/tdf153767.xlsx",
        pages: 2,
        contains: [pt!(0, "Contact Name Address City Postal Code Country"), pt!(1, "TRUE"), pt!(1, "FALSE")],
        page_sizes: [size!(0, 595.28, 841.89)],
    ),
    case!(
        tdf161301,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf161301",
        file: "sc/qa/unit/data/xlsx/tdf161301.xlsx",
        pages: 2,
        // Current Office PDF resolves formatCode16's Japanese calendar.
        occurrences: [count!(0, "令和6年5月28日", 2), count!(1, "令和6年5月28日", 2)],
    ),
    case!(
        new_cond_format,
        source: "../core/sc/qa/unit/cond_format.cxx:testNewCondFormatXLSX",
        file: "sc/qa/unit/data/xlsx/new_cond_format_test.xlsx",
        pages: 3,
        contains: [
            pt!(0, "top n elements bottom n elements top n percent bottom n percent"),
            pt!(1, "above average below average above equal average below equal average"),
            pt!(2, "2.00 2 1 1.000 4.00 3"),
        ],
        // Calibri Office controls keep this heading only on the continuation.
        not_contains: [pt!(0, "above average")],
    ),
    case!(
        tdf83671_smartart,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf83671_SmartArt_import",
        file: "sc/qa/unit/data/xlsx/tdf83671_SmartArt_import.xlsx",
        pages: 1,
        contains: [pt!(0, "start"), pt!(0, "back"), pt!(0, "middle"), pt!(0, "front"), pt!(0, "end")],
    ),
    case!(
        tdf151818_smartart,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf151818_SmartArtFontColor",
        file: "sc/qa/unit/data/xlsx/tdf151818_SmartartThemeFontColor.xlsx",
        pages: 1,
        contains: [pt!(0, "One Two Three")],
    ),
    case!(
        pivot_cache_mixed_types,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotCacheExportXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/with-strings-integers-and-dates.xlsx",
        pages: 3,
        contains: [
            pt!(0, "Row Labels Sum of all fields are integers"),
            pt!(0, "Grand Total 16665"),
            pt!(2, "tekst 6/7/09 10:53 AM"),
        ],
    ),
    case!(
        two_data_fields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableTwoDataFields",
        file: "sc/qa/unit/data/xlsx/pivot-table/two-data-fields.xlsx",
        pages: 2,
        contains: [pt!(0, "Name Sum of Value Count of Value2"), pt!(0, "Grand Total 3.651248215 7"), pt!(1, "Name Value")],
    ),
    case!(
        pivotcompact,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableCompactLayoutXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/pivotcompact.xlsx",
        pages: 2,
        contains: [pt!(1, "Sum of Val Column Labels"), pt!(1, "Row Labels ddd ddx Grand Total"), pt!(1, "Grand Total 40 41 81")],
    ),
    case!(
        first_header_row_zero,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testFirstHeaderRowZero",
        file: "sc/qa/unit/data/xlsx/pivot-table/first_header_row_zero.xlsx",
        pages: 2,
        contains: [pt!(1, "A Suma de N Suma de V"), pt!(1, "Total general 12 12"), pt!(1, "under")],
    ),
    case!(
        calcfields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFields1XLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/calcfields.xlsx",
        pages: 4,
        contains: [pt!(2, "Pivot Table_Sheet1_1"), pt!(2, "TATA 24 6.00 Ft"), pt!(2, "Total Result 114 18.00 Ft")],
    ),
    case!(
        onlycalcfields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFields2XLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/onlycalcfields.xlsx",
        pages: 4,
        contains: [pt!(2, "Name Sum - Field1 Sum - Field2"), pt!(2, "TATA"), pt!(2, "TITI"), pt!(2, "TOTO"), pt!(2, "Total Result"), pt!(3, "5")],
    ),
    case!(
        groupwithcalcfields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testGroupAndCalcFieldXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/groupwithcalcfields.xlsx",
        pages: 4,
        contains: [pt!(0, "Data"), pt!(0, "Group1 45 54 63"), pt!(0, "Total Result 171 189 207")],
    ),
    case!(
        tdf126858,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFieldSingleDataDimXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/tdf126858-1.xlsx",
        pages: 2,
        contains: [
            pt!(0, "Названия строк"), pt!(0, "Сумма по полю Стоимость"),
            pt!(0, "апельсин 8976"), pt!(0, "банан 9936"), pt!(0, "вишня 37408"), pt!(0, "Общий итог 168168"),
            pt!(1, "товар"),
            pt!(1, "кол-во"),
            pt!(1, "цена"),
            pt!(1, "за"),
            pt!(1, "ед"),
            pt!(1, "стоимость"),
        ],
    ),
    case!(
        test_diff_aggregation,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFieldDiffAggregationXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/test_diff_aggregation.xlsx",
        pages: 4,
        contains: [pt!(2, "Pivot Table_Sheet1_1"), pt!(2, "2010 78"), pt!(3, "Pivot Table_Sheet1_2"), pt!(3, "Grand Total 6")],
    ),
    case!(
        pivottable_double_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableDoubleFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_double_field_filter.xlsx",
        pages: 2,
        contains: [pt!(0, "Double field1 Double field2 Double field3 Datas"), pt!(0, "2 2.00 20,000.00 12"), pt!(1, "10,000.00 22")],
    ),
    case!(
        pivottable_string_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableStringFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_string_field_filter.xlsx",
        pages: 2,
        contains: [pt!(0, "Sum - Amount Order ID Country Total"), pt!(0, "United States")],
        not_contains: [pt!(0, "United Kingdom")],
    ),
    case!(
        pivottable_date_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableDateFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_date_field_filter.xlsx",
        pages: 3,
        contains: [pt!(0, "Sum - Amount Date Date2 Date3 Total"), pt!(1, "2016/ January 7/"), pt!(2, "2016/ 1/ 8. 0:00")],
    ),
    case!(
        shared_group_field,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-group-field.xlsx",
        pages: 4,
        contains: [pt!(0, "Sorcímkék Összeg / a Összeg / b"), pt!(0, "Csoport1 15 20 25"), pt!(0, "Végösszeg 171 189 207"), pt!(1, "Összeg / h Összeg / i")],
    ),
    case!(
        shared_dategroup,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedDateGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-dategroup.xlsx",
        pages: 4,
        contains: [pt!(2, "Sorcímkék Összeg / c Összeg / d Összeg / e"), pt!(2, "1965 163877 212212 262738"), pt!(2, "Végösszeg 1113132 1301928 2042856")],
    ),
    case!(
        shared_nested_dategroup,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedNestedDateGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-nested-dategroup.xlsx",
        pages: 4,
        contains: [pt!(2, "Sorcímkék Összeg / c Összeg / e"), pt!(2, "1965"), pt!(2, "Jan 53274 87176"), pt!(2, "Végösszeg 1113132 2042856")],
    ),
    case!(
        shared_numgroup,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedNumGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-numgroup.xlsx",
        pages: 4,
        contains: [pt!(2, "Sorcímkék Összeg / c Összeg / d Összeg / e"), pt!(2, "32674-47673 193380 194190 414100"), pt!(2, "Végösszeg 1113132 1301928 2042856")],
    ),
    case!(
        pivottable_bool_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableBoolFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_bool_field_filter.xlsx",
        pages: 2,
        contains: [pt!(0, "Sum of Amount Bool field Total"), pt!(0, "TRUE"), pt!(1, "FALSE")],
        not_contains: [pt!(0, "FALSE")],
    ),
    case!(
        pivottable_row_col_page_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableRowColPageFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_rowcolpage_field_filter.xlsx",
        pages: 3,
        contains: [pt!(0, "Double3 field (All)"), pt!(0, "Order ID 2"), pt!(0, "1 4270"), pt!(2, "Double3 field Double4 field")],
    ),
    case!(
        pivottable_error_item_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableErrorItemFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_error_item_filter.xlsx",
        pages: 1,
        contains: [pt!(0, "a b Row Labels Sum of a"), pt!(0, "2 #DIV/0! Grand Total 4")],
    ),
    case!(
        pivottable_long_text,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf125046",
        file: "sc/qa/unit/data/xlsx/pivottable_long_text.xlsx",
        pages: 4,
        contains: [pt!(0, "Row Labels a b (blank) Grand Total"), pt!(1, "A very-very long"), pt!(1, "greater than wto hundred fifty five characte")],
    ),
    case!(
        pivottable_one_second_difference,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf125055",
        file: "sc/qa/unit/data/xlsx/pivottable_1s_difference.xlsx",
        pages: 1,
        contains: [pt!(0, "n d Row Labels"), pt!(0, "a 7/10/2017 9:11"), pt!(0, "b 7/10/2017 9:11")],
    ),
    case!(
        pivot_cached_definition_in_sync,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableXLSX_OutOfSyncPivotTableCachedDefinitionImport",
        file: "sc/qa/unit/data/xlsx/PivotTable_CachedDefinitionAndDataInSync.xlsx",
        pages: 3,
        contains: [pt!(0, "Row Labels Sum of A"), pt!(0, "1 5"), pt!(0, "Grand Total 10")],
    ),
    case!(
        pivot_cached_definition_with_cache_data,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableXLSX_OutOfSyncPivotTableCachedDefinitionImport2",
        file: "sc/qa/unit/data/xlsx/PivotTable_CachedDefinitionAndDataNotInSync_SheetColumnsRemoved_WithCacheData.xlsx",
        pages: 2,
        contains: [pt!(0, "Row Labels Sum of A"), pt!(0, "2 5"), pt!(1, "A K")],
    ),
    case!(
        pivot_cached_definition_without_cache_data,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableXLSX_OutOfSyncPivotTableCachedDefinitionImport3",
        file: "sc/qa/unit/data/xlsx/PivotTable_CachedDefinitionAndDataNotInSync_SheetColumnsRemoved_WithoutCacheData.xlsx",
        pages: 2,
        contains: [pt!(0, "Row Labels Sum of A"), pt!(0, "Grand Total 10"), pt!(1, "1 1")],
    ),
    case!(
        book1_custom,
        source: "../core/sc/qa/unit/subsequent_export_test6.cxx:testTableStyleCustomRoundtripXLSX",
        file: "sc/qa/unit/data/xlsx/Book1_custom.xlsx",
        pages: 2,
        contains: [pt!(0, "Names Numbers Dates Age"), pt!(0, "Summary 382")],
    ),
    case!(
        tdf165180_date1904,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf165180_date1904_XLSX",
        file: "sc/qa/unit/data/xlsx/tdf165180_date1904.xlsx",
        pages: 1,
        contains: [pt!(0, "Tuesday, March 1, 1904 Mar 1 1904"), pt!(0, "Monday, September 12, 2005 Sept 12 2005")],
    ),
    case!(
        tdf122191,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf122191",
        file: "sc/qa/unit/data/xlsx/tdf122191.xlsx",
        pages: 1,
        contains: [pt!(0, "TRUE")],
        not_contains: [pt!(0, "BOOL00AN")],
    ),
    case!(
        tdf123139_apply_alignment,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf123139XLSX",
        file: "sc/qa/unit/data/xlsx/tdf123139_applyAlignment.xlsx",
        pages: 2,
        contains: [pt!(0, "foofoofoofoofoofoofoofoofoofoo"), pt!(0, "bar"), pt!(1, "hidden formula"), pt!(1, "unlocked distributed")],
    ),
    case!(
        test_shape_autofit,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testShapeAutofitXLSX",
        file: "sc/qa/unit/data/xlsx/testShapeAutofit.xlsx",
        pages: 2,
        contains: [pt!(0, "This one is autofit."), pt!(0, "This one is not autofit.")],
    ),
    case!(
        pivot_calcfield_nameerror,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFieldNameErrorXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/pivot_calcfield_nameerror.xlsx",
        pages: 5,
        contains: [pt!(0, "Bez Werte Hilfe + Hilfe -"), pt!(2, "Zeilenbeschriftungen bezeichnung von deinen Pluswerten"), pt!(2, "Gesamtergebnis 1300 -1678")],
    ),
    case!(
        tdf169326,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf169326_ignoreLineBreaksInReferencedCells",
        file: "sc/qa/unit/data/xlsx/tdf169326_ignore_line_breaks_in_referenced_cells.xlsx",
        pages: 1,
        contains: [pt!(0, "Hello World Hello World")],
    ),
    case!(
        tdf120301,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testtdf120301_xmlSpaceParsingXLSX",
        file: "sc/qa/unit/data/xlsx/tdf120301_xmlSpaceParsing.xlsx",
        pages: 1,
        contains: [pt!(0, "Check Box 1"), pt!(0, "Option Button 2")],
    ),
    case!(
        functions_excel_2010,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testFunctionsExcel2010XLSX",
        file: "sc/qa/unit/data/xlsx/functions-excel-2010.xlsx",
        pages: 6,
        contains: [pt!(0, "Function Formula Result should be Equal? All equal?"), pt!(0, "BETA.DIST"), pt!(0, "TRUE")],
    ),
    case!(
        ceiling_floor,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testCeilingFloorXLSX",
        file: "sc/qa/unit/data/xlsx/ceiling-floor.xlsx",
        pages: 4,
        contains: [pt!(0, "23.5 -23.5"), pt!(0, "#NUM!"), pt!(0, "#DIV/0!")],
    ),
    case!(
        hyperlink_formula,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf126024XLSX",
        file: "sc/qa/unit/data/xlsx/hyperlink_formula.xlsx",
        pages: 1,
        occurrences: [count!(0, "formula", 2)],
    ),
    case!(
        hyperlink_export,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf126177XLSX",
        file: "sc/qa/unit/data/xlsx/hyperlink_export.xlsx",
        pages: 1,
        contains: [pt!(0, "C:\\TEMP\\test.xlsx#Munka1!A5")],
    ),
    case!(
        tdf135828_shape_rect,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf135828_Shape_Rect",
        file: "sc/qa/unit/data/xlsx/tdf135828_Shape_Rect.xlsx",
        pages: 1,
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf137000_export_upright,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf137000_handle_upright",
        file: "sc/qa/unit/data/xlsx/tdf137000_export_upright.xlsx",
        pages: 2,
        contains: [pt!(0, "Simple Text"), pt!(1, "Simple Text")],
    ),
    case!(
        tdf106197_import_upright,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTextBoxBodyUpright",
        file: "sc/qa/unit/data/xlsx/tdf106197_import_upright.xlsx",
        pages: 2,
        contains: [pt!(0, "Simple Text"), pt!(1, "Simple Text")],
    ),
    case!(
        activex_checkbox,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testActiveXCheckboxXLSX",
        file: "sc/qa/unit/data/xlsx/activex_checkbox.xlsx",
        pages: 1,
        // The caption is inside this EMF preview; metafile_pdf tests its PDF text.
        image_counts: [page_count!(0, 1)],
    ),
    case!(
        cell_anchored_hidden_shapes,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testCellAnchoredHiddenShapesXLSX",
        file: "sc/qa/unit/data/xlsx/cell-anchored-hidden-shapes.xlsx",
        pages: 1,
        contains: [pt!(0, "TAIWAN PROMOTION AIRFARES"), pt!(0, "BUSINESS CLASS")],
    ),
    case!(
        shape_rotation_import,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testShapeRotationImport",
        file: "sc/qa/unit/data/xlsx/testShapeRotationImport.xlsx",
        pages: 2,
        contains: [pt!(0, "jo"), pt!(0, "bb"), pt!(0, "ra")],
    ),
    case!(
        tdf134455,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf134455",
        file: "sc/qa/unit/data/xlsx/tdf134455.xlsx",
        pages: 2,
        contains: [pt!(0, "00:05"), pt!(0, "01:05"), pt!(0, "04:00")],
    ),
    case!(
        tdf131424,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf131424",
        file: "sc/qa/unit/data/xlsx/tdf131424.xlsx",
        pages: 1,
        contains: [pt!(0, "This is the first column"), pt!(0, "12 23 35"), pt!(0, "36 45 81")],
    ),
    case!(
        tdf122336,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTdf122336",
        file: "sc/qa/unit/data/xlsx/tdf122336.xlsx",
        pages: 9,
        contains: [pt!(0, "UitvoeringsdatumStarttijd"), pt!(0, "12/25/2018 11:30"), pt!(2, "Van Rompaey Marcus")],
    ),
    case!(
        shared_formula_3d_reference,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:shared-formula 3D reference assertions",
        file: "sc/qa/unit/data/xlsx/shared-formula/3d-reference.xlsx",
        pages: 2,
        contains: [pt!(0, "Value Same sheet Another sheet Same sheet but sheet name shown"), pt!(0, "1 1 10 1"), pt!(1, "10 20 30 40 50 60")],
    ),
    case!(
        shared_formula_basic,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testSharedFormulaXLSX",
        file: "sc/qa/unit/data/xlsx/shared-formula/basic.xlsx",
        pages: 1,
        contains: [pt!(0, "Value Formula"), pt!(0, "1 10"), pt!(0, "18 180")],
    ),
    case!(
        shared_formula_text_results,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSharedFormulaStringResultExportXLSX",
        file: "sc/qa/unit/data/xlsx/shared-formula/text-results.xlsx",
        pages: 2,
        contains: [pt!(0, "Text Same Sheet Another Sheet"), pt!(0, "A A AA"), pt!(0, "F F FF"), pt!(1, "AA BB CC DD EE FF")],
    ),
    case!(
        external_refs,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testExternalRefCacheXLSX",
        file: "sc/qa/unit/data/xlsx/external-refs.xlsx",
        pages: 1,
        contains: [pt!(0, "Name Andy Bruce Charlie")],
    ),
    case!(
        ref_string,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testRefStringXLSX",
        file: "sc/qa/unit/data/xlsx/ref_string.xlsx",
        pages: 1,
        contains: [pt!(0, "1 2 3")],
    ),
    case!(
        tdf139934,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testTdf139934",
        file: "sc/qa/unit/data/xlsx/tdf139934.xlsx",
        pages: 9,
        contains: [pt!(0, "Absence Requests"), pt!(0, "1/20/2021 Wednesday Annual Leave")],
    ),
    case!(
        tdf100154,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testNonAsciiWithDotXLSX",
        file: "sc/qa/unit/data/xlsx/tdf100154.xlsx",
        pages: 2,
        contains: [pt!(0, "5"), pt!(1, "5")],
    ),
    case!(
        tdf160371,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf160371",
        file: "sc/qa/unit/data/xlsx/tdf160371.xlsx",
        pages: 1,
        contains: [pt!(0, "Intersection Example Data Table"), pt!(0, "Value 1 Row_2 4 5 6")],
    ),
    case!(
        tdf100709,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf100709XLSX",
        file: "sc/qa/unit/data/xlsx/tdf100709.xlsx",
        pages: 2,
        // Upstream checks unformatted B52/A75 = 218, not the malformed date styles.
        // Excel cannot open this source; it remains a REFERENCE_FAIL.
        contains: [pt!(0, "65 218 46"), pt!(1, "218 11")],
    ),
    case!(
        tdf105272,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf105272",
        file: "sc/qa/unit/data/xlsx/tdf105272.xlsx",
        pages: 1,
        contains: [pt!(0, "Gold Silver Bronze Total"), pt!(0, "13 11 9 33"), pt!(0, "232 0.14")],
    ),
    case!(
        tdf119292,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testTdf119292",
        file: "sc/qa/unit/data/xlsx/tdf119292.xlsx",
        pages: 1,
        contains: [pt!(0, "text rotated by 270 degrees"), pt!(0, "text rotated by 90 degrees")],
    ),
    case!(
        tdf131536,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf131536",
        file: "sc/qa/unit/data/xlsx/tdf131536.xlsx",
        pages: 2,
        contains: [pt!(0, "Excel - true Calc -true"), pt!(0, "L0001 L0001"), pt!(1, "MMR/HEV/LIC/000001")],
    ),
    case!(
        tdf137543,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf137543XLSX",
        file: "sc/qa/unit/data/xlsx/tdf137543.xlsx",
        pages: 3,
        contains: [pt!(0, "25"), pt!(0, "Source array"), pt!(1, "A B E F I J M N Q R U V")],
    ),
    case!(
        tdf170201,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf170201_empty_values_in_array_formulas",
        file: "sc/qa/unit/data/xlsx/tdf170201.xlsx",
        pages: 1,
        // Office repair-mode export rejects the named constant {1,,3};
        // the golden and controlled exports print three #NAME? errors.
        contains: [pt!(0, "Sheet1"), pt!(0, "#NAME? #NAME? #NAME?")],
    ),
    case!(
        tdf134553,
        source: "../core/sc/qa/unit/jumbosheets-test.cxx:testTdf134553",
        file: "sc/qa/unit/data/xlsx/tdf134553.xlsx",
        pages: 2,
        // The mapped LibreOffice test verifies that the chart object survives
        // cut/paste with stable geometry; it does not prescribe Calc's
        // semicolon data-label separator. Keep the visible-content probe
        // aligned with the Office fixed-output golden.
        contains: [pt!(0, "Chart Title"), pt!(0, "First data point, 2"), pt!(0, "Second data point, 3")],
    ),
    case!(
        tdf142929,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf142929_filterLessThanXLSX",
        file: "sc/qa/unit/data/xlsx/tdf142929.xlsx",
        pages: 1,
        contains: [pt!(0, "Numbers"), pt!(0, "1")],
    ),
    case!(
        tdf143068_top10filter,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testAutofilterTop10XLSX",
        file: "sc/qa/unit/data/xlsx/tdf143068_top10filter.xlsx",
        pages: 1,
        contains: [pt!(0, "Numbers"), pt!(0, "7 8 9 10")],
    ),
    case!(
        tdf144397,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testExternalDefinedNameXLSX",
        file: "sc/qa/unit/data/xlsx/tdf144397.xlsx",
        pages: 1,
        contains: [pt!(0, "Strings With Range name:"), pt!(0, "January January"), pt!(0, "May #N/A")],
    ),
    case!(
        tdf162963,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf162963",
        file: "sc/qa/unit/data/xlsx/tdf162963_TableWithTotalsEnabled.xlsx",
        pages: 1,
        contains: [pt!(0, "Name Sales"), pt!(0, "Miller 23"), pt!(0, "All 115")],
    ),
    case!(
        autofilter,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testAutofilterXLSX",
        file: "sc/qa/unit/data/xlsx/autofilter.xlsx",
        pages: 1,
        contains: [pt!(0, "column1 column2 column3"), pt!(0, "2 3 4"), pt!(0, "4 5 4")],
    ),
    case!(
        tablerefsnamed,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testNamedTableRef",
        file: "sc/qa/unit/data/xlsx/tablerefsnamed.xlsx",
        pages: 1,
        contains: [pt!(0, "Name Score ScoreNames"), pt!(0, "aaa 3.5 ScoreNames aaa TRUE"), pt!(0, "fff 4.1 ScoreNames fff TRUE")],
    ),
    case!(
        database_ranges,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testDatabaseRangesXLSX",
        file: "sc/qa/unit/data/xlsx/database.xlsx",
        pages: 2,
        contains: [pt!(0, "Col1 Col2 Col3 Col4"), pt!(1, "Using named db range Using unnamed db range"), pt!(1, "Name Age Children")],
    ),
    case!(
        matrix_multiplication,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testMatrixMultiplicationXLSX",
        file: "sc/qa/unit/data/xlsx/matrix-multiplication.xlsx",
        pages: 1,
        contains: [pt!(0, "1 2 4 5.2"), pt!(0, "49.2"), pt!(0, "103.6")],
    ),
    case!(
        tdf118990,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf118990",
        file: "sc/qa/unit/data/xlsx/tdf118990.xlsx",
        pages: 1,
        contains: [pt!(0, "333 C")],
        occurrences: [count!(0, "333", 3)],
    ),
    case!(
        tdf163554,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf163554",
        file: "sc/qa/unit/data/xlsx/tdf163554.xlsx",
        pages: 2,
        contains: [pt!(0, "time (misc) - last"), pt!(0, "#NAME? 7"), pt!(1, "time (pnrst)"), pt!(1, "Seite 1")],
        occurrences: [count!(0, "#NAME?", 5)],
    ),
    case!(
        tdf85617,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf85617",
        file: "sc/qa/unit/data/xlsx/tdf85617.xlsx",
        pages: 1,
        contains: [pt!(0, "Code name Quantity Price"), pt!(0, "Товар 1 1 4.5")],
    ),
    case!(
        tdf118668,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTdf118668",
        file: "sc/qa/unit/data/xlsx/tdf118668.xlsx",
        pages: 3,
        contains: [pt!(0, "ПУТЕВОЙ ЛИСТ ТРАКТОРА"), pt!(1, "Результат работы автомобиля за смену")],
    ),
    case!(
        pivottable_outline_mode,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableOutlineModeXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_outline_mode.xlsx",
        pages: 1,
        contains: [pt!(0, "field1 field2 field3"), pt!(0, "Sum of field3"), pt!(0, "Grand Total 6")],
    ),
    case!(
        pivottable_tabular_mode,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableTabularModeXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_tabular_mode.xlsx",
        pages: 4,
        contains: [pt!(0, "pwdLastSet (Multiple Items)"), pt!(0, "company employeeID mail"), pt!(0, "Grand Total 9")],
    ),
    case!(
        pivot_dark1,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf124810_pivotDark",
        file: "sc/qa/unit/data/xlsx/pivot_dark1.xlsx",
        pages: 2,
        contains: [pt!(0, "name date value"), pt!(0, "Column Labels"), pt!(1, "Grand Total")],
    ),
    case!(
        pivottable_invalid_formats,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testInvalidFormats",
        file: "sc/qa/unit/data/xlsx/pivottable_invalid_formats.xlsx",
        pages: 5,
        contains: [pt!(0, "Spalte 1 Spalte 2 Spalte 3"), pt!(0, "A Ur 600 0.65"), pt!(0, "H Th 1144 149.89")],
    ),
    case!(
        tdf89139_pivot_table,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableExportXLSX",
        file: "sc/qa/unit/data/xlsx/tdf89139_pivot_table.xlsx",
        pages: 2,
        contains: [pt!(0, "Name Id Dept Date"), pt!(1, "Row Labels Count of Id"), pt!(1, "Grand Total 2")],
    ),
    case!(
        tdf165503,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf165503",
        file: "sc/qa/unit/data/xlsx/tdf165503.xlsx",
        pages: 2,
        contains: [pt!(0, "Date Val"), pt!(0, "1/3/2021 3"), pt!(0, "######### 9")],
    ),
    case!(
        tdf137091,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf137091",
        file: "sc/qa/unit/data/xlsx/tdf137091.xlsx",
        pages: 20,
        contains: [pt!(0, "SG STOKKODU Sütun2"), pt!(0, "IP 1 B J7 0280 4 4 08 YY MR0084 28/4")],
    ),
    case!(
        tdf70455,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf70455",
        file: "sc/qa/unit/data/xlsx/tdf70455.xlsx",
        pages: 1,
        contains: [pt!(0, "Bet History"), pt!(0, "Gross: €780.00"), pt!(0, "€130.00 No 7/1 8.0000 Win €1,040.00 €910.00")],
    ),
    case!(
        tdf98481,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf98481",
        file: "sc/qa/unit/data/xlsx/tdf98481.xlsx",
        pages: 3,
        contains: [pt!(0, "Horizontal and vertical Sums with source as separate sheet."), pt!(0, "Sum 4 0 3")],
    ),
    case!(
        tdf115022,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf115022",
        file: "sc/qa/unit/data/xlsx/tdf115022.xlsx",
        pages: 1,
        contains: [pt!(0, "Index amount"), pt!(0, "a 1 a 2 a 3"), pt!(0, "6")],
    ),
    case!(
        tdf164895,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf164895",
        file: "sc/qa/unit/data/xlsx/tdf164895.xlsx",
        pages: 1,
        // Office rejects the unqualified references in X/Cnt; Calc accepts them.
        contains: [pt!(0, "a 1"), pt!(0, "8 #NAME? #NAME? 9 10")],
        occurrences: [count!(0, "#NAME?", 2)],
    ),
    case!(
        tdf162093,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf162093",
        file: "sc/qa/unit/data/xlsx/tdf162093.xlsx",
        pages: 2,
        contains: [pt!(0, "Surname Count Region Surname Count Region"), pt!(0, "Murray 15 North Murray 15 North"), pt!(0, "Total 296 Total 296"), pt!(1, "=myData[#Headers] =myData[#Headers]")],
    ),
    case!(
        tdf147955,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf147955",
        file: "sc/qa/unit/data/xlsx/tdf147955.xlsx",
        pages: 1,
        contains: [pt!(0, "Leasingham Community Benefit Society Ltd"), pt!(0, "Sales 892.75"), pt!(0, "Food - CoS 130.25"), pt!(0, "Cleaning 10.98")],
    ),
    case!(
        tdf155046,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf155046",
        file: "sc/qa/unit/data/xlsx/tdf155046.xlsx",
        pages: 5,
        contains: [pt!(0, "Respondent ID Publication ID Submitted Submitted Time"), pt!(0, "89nre8cuc7i3 69lue27dr864 TRUE"), pt!(1, "Publication Consent")],
    ),
    case!(
        tdf136364,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf136364",
        file: "sc/qa/unit/data/xlsx/tdf136364.xlsx",
        pages: 1,
        contains: [pt!(0, "1 1 1 1 27"), pt!(0, "2 2 2 2 12")],
    ),
    case!(
        tdf157689,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTdf157689",
        file: "sc/qa/unit/data/xlsx/tdf157689.xlsx",
        pages: 2,
        contains: [pt!(0, "col1 col2"), pt!(0, "1 2 2 3"), pt!(1, "col3 col4"), pt!(1, "2 2 4 2")],
    ),
    case!(
        tdf142905,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf142905",
        file: "sc/qa/unit/data/xlsx/tdf142905.xlsx",
        pages: 1,
        contains: [pt!(0, "3M 3M")],
    ),
    case!(
        tdf119190,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf119190",
        file: "sc/qa/unit/data/xlsx/tdf119190.xlsx",
        pages: 1,
        contains: [pt!(0, "c?")],
        not_contains: [pt!(0, "Kelemen Gábor 2:"), pt!(0, "Comment!")],
    ),
    case!(
        tdf141495,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf141495",
        file: "sc/qa/unit/data/xlsx/tdf141495.xlsx",
        pages: 2,
        // Office has no LibreOffice Analysis add-in; retain the literal A5
        // serial and recalculate its service-name calls as #NAME?.
        contains: [pt!(0, "44227 #NAME? #NAME? #NAME?"), pt!(1, "#NAME? #NAME? #NAME? #NAME? #NAME?")],
        not_contains: [pt!(0, "44255"), pt!(1, "44926")],
    ),
    case!(
        tdf79972,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf79972XLSX",
        file: "sc/qa/unit/data/xlsx/tdf79972.xlsx",
        pages: 1,
        contains: [pt!(0, "123")],
        link_targets: ["https://bugs.documentfoundation.org/show_bug.cgi?id=79972"],
    ),
    case!(
        tdf111876,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf111876",
        file: "sc/qa/unit/data/xlsx/tdf111876.xlsx",
        pages: 1,
        contains: [pt!(0, "..\\xls\\bug-fixes.xls")],
    ),
    case!(
        tdf119565,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf119565",
        file: "sc/qa/unit/data/xlsx/tdf119565.xlsx",
        pages: 1,
        contains: [pt!(0, "Lorem ipsum dolor"), pt!(0, "Maecenas porttitor")],
    ),
    case!(
        textbox_char_kerning_space,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetCharacterKerningSpaceXLSX",
        file: "sc/qa/unit/data/xlsx/textbox-CharKerningSpace.xlsx",
        pages: 2,
        contains: [pt!(0, "AVAIL")],
    ),
    case!(
        textbox_condensed_character_space,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetCondensedCharacterSpaceXLSX",
        file: "sc/qa/unit/data/xlsx/textbox-CondensedCharacterSpace.xlsx",
        pages: 2,
        contains: [pt!(0, "AvaiL")],
    ),
    case!(
        tdf141644,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTextBoxBodyRotateAngle",
        file: "sc/qa/unit/data/xlsx/tdf141644.xlsx",
        pages: 1,
        contains: [pt!(0, "Textdir: 270 deg")],
    ),
    case!(
        tdf142881,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf142881",
        file: "sc/qa/unit/data/xlsx/tdf142881.xlsx",
        pages: 2,
        contains: [pt!(0, "Rotated:"), pt!(0, "35"), pt!(1, "25")],
    ),
    case!(
        tdf145057,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf145057",
        file: "sc/qa/unit/data/xlsx/tdf145057.xlsx",
        pages: 1,
        contains: [pt!(0, "Numbers Names abc def fgh"), pt!(0, "4.00 s 3 4 1")],
    ),
    case!(
        tdf161365,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf161365",
        file: "sc/qa/unit/data/xlsx/tdf161365.xlsx",
        pages: 1,
        contains: [pt!(0, "This spreadsheet contains checkbox which dissapeared after re-export")],
    ),
    case!(
        test_115192,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf115192XLSX",
        file: "sc/qa/unit/data/xlsx/test_115192.xlsx",
        pages: 1,
        contains: [pt!(0, "Hyperlink: test.xlxs"), pt!(0, "Hyperlink: Sheet2!A1"), pt!(0, "Hyperlink: Bug 115192")],
    ),
    case!(
        pivot_many_fields_in_cache,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:many-fields-in-cache import check",
        file: "sc/qa/unit/data/xlsx/pivot-table/many-fields-in-cache.xlsx",
        pages: 2,
        contains: [pt!(0, "F1 F2 F3 F4 F5 F6 F7 F8"), pt!(0, "Sum of F10 F4"), pt!(0, "Grand Total 5 6 11")],
    ),
    case!(
        pivottable_duplicated_member_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableDuplicatedMemberFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_duplicated_member_filter.xlsx",
        pages: 3,
        contains: [pt!(0, "pwdLastSet (blank)"), pt!(0, "First type 4"), pt!(0, "Second type 5"), pt!(0, "Grand Total 9")],
    ),
    case!(
        tdf104310,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf104310_x14",
        file: "sc/qa/unit/data/xlsx/tdf104310.xlsx",
        pages: 1,
        contains: [pt!(0, "1 2 3 4 5")],
    ),
    case!(
        text_length_data_validity,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTextLengthDataValidityXLSX",
        file: "sc/qa/unit/data/xlsx/textLengthDataValidity.xlsx",
        pages: 1,
        contains: [pt!(0, "1234"), pt!(0, "1234.00"), pt!(0, "12.3")],
    ),
    case!(
        tdf129985,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf129985",
        file: "sc/qa/unit/data/xlsx/tdf129985.xlsx",
        pages: 1,
        contains: [pt!(0, "Érvényesség kezdete")],
        occurrences: [count!(0, "1/13/2020", 2)],
    ),
    case!(
        tdf73063,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf73063",
        file: "sc/qa/unit/data/xlsx/tdf73063.xlsx",
        pages: 1,
        // The explicit Serbian LCID overrides the UI language.
        contains: [pt!(0, "субота, 17. јуни 1972")],
    ),
    case!(
        tdf139021,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testExtCondFormatXLSX",
        file: "sc/qa/unit/data/xlsx/tdf139021.xlsx",
        pages: 1,
        contains: [pt!(0, "hello hello"), pt!(0, "hello bye")],
    ),
    case!(
        tdf139394,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf139394",
        file: "sc/qa/unit/data/xlsx/tdf139394.xlsx",
        pages: 1,
        occurrences: [count!(0, "+", 3), count!(0, "-", 1)],
    ),
    case!(
        tdf55417,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf55417",
        file: "sc/qa/unit/data/xlsx/tdf55417.xlsx",
        pages: 1,
        contains: [pt!(0, "29")],
    ),
    case!(
        tdf139167,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf139167",
        file: "sc/qa/unit/data/xlsx/tdf139167.xlsx",
        pages: 1,
        contains: [pt!(0, "Hello")],
    ),
    case!(
        tdf109061,
        source: "../core/sc/qa/unit/jumbosheets-test.cxx:testTdf109061",
        file: "sc/qa/unit/data/xlsx/tdf109061.xlsx",
        pages: 1,
        contains: [pt!(0, "Test"), pt!(0, "1 2 3"), pt!(0, "Sum: 6")],
    ),
    case!(
        tdf112106,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf112106",
        file: "sc/qa/unit/data/xlsx/tdf112106.xlsx",
        pages: 2,
        contains: [pt!(0, "Country (All)"), pt!(0, "Banana 617"), pt!(0, "Grand Total 13126"), pt!(1, "Order ID Product Category Amount Date Country")],
    ),
    case!(
        tdf155402,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf155402",
        file: "sc/qa/unit/data/xlsx/tdf155402.xlsx",
        // This helper supplies only the basename to CELL(FILENAME). Office
        // includes its long temporary directory, overflowing onto page 2.
        pages: 1,
        contains: [pt!(0, "[tdf155402.xlsx]Sheet1")],
    ),
    case!(
        tdf91251,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf91251_missingOverflowRoundtrip",
        file: "sc/qa/unit/data/xlsx/tdf91251_missingOverflowRoundtrip.xlsx",
        pages: 2,
        contains: [pt!(1, "Text Box")],
    ),
    case!(
        tdf164417,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf164417",
        file: "sc/qa/unit/data/xlsx/tdf164417.xlsx",
        pages: 2,
        contains: [pt!(0, "Num Text Date"), pt!(0, "1 a 31/12/24"), pt!(0, "2 a 31/12/2024 (text)")],
    ),
    case!(
        tdf165886,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf165886",
        file: "sc/qa/unit/data/xlsx/tdf165886.xlsx",
        pages: 1,
        contains: [pt!(0, "0 #NAME? TRUE")],
        occurrences: [count!(0, "#NAME?", 8)],
    ),
    case!(
        tdf166413,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf166413",
        file: "sc/qa/unit/data/xlsx/tdf166413.xlsx",
        pages: 1,
        occurrences: [count!(0, r#"test ABC "ABC""#, 4)],
    ),
    case!(
        tdf95640,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf95640_xlsx_to_xlsx",
        file: "sc/qa/unit/data/xlsx/tdf95640.xlsx",
        pages: 1,
        contains: [pt!(0, "AAA BBB"), pt!(0, "jan 2"), pt!(0, "feb 1"), pt!(0, "mar 3")],
    ),
    case!(
        tdf97598,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf97598XLSX",
        file: "sc/qa/unit/data/xlsx/tdf97598_scenarios.xlsx",
        pages: 1,
        contains: [pt!(0, "Cell A1")],
    ),
    case!(
        tdf81939,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testEscapeCharInNumberFormatXLSX",
        file: "sc/qa/unit/data/xlsx/tdf81939.xlsx",
        pages: 1,
        contains: [pt!(0, "01 23 45 67 89"), pt!(0, "01.23.45.678.9"), pt!(0, "123,456,789 €")],
    ),
    case!(
        universal_content_strict,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testContentXLSXStrict",
        file: "sc/qa/unit/data/xlsx/universal-content-strict.xlsx",
        pages: 1,
        contains: [pt!(0, "1 String1 6"), pt!(0, "2 String2 5"), pt!(0, "-1 11")],
    ),
    case!(
        row_index_1_based,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testRowIndex1BasedXLSX",
        file: "sc/qa/unit/data/xlsx/row-index-1-based.xlsx",
        pages: 1,
        contains: [pt!(0, "Action Plan.Name Action Plan.Description"), pt!(0, "Jerry This is a longer Text."), pt!(0, "Second line."), pt!(0, "Third line.")],
    ),
    case!(
        pivot1_row,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot1_Row",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot1_Row.xlsx",
        pages: 2,
        contains: [pt!(1, "Name Type Sum of Price"), pt!(1, "X1 A 100"), pt!(1, "X5 A 50")],
    ),
    case!(
        pivot1_row_grand,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot1_Row_Grand",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot1_Row_Grand.xlsx",
        pages: 2,
        contains: [pt!(1, "Name Type Sum of Price"), pt!(1, "Grand Total 600")],
    ),
    case!(
        pivot1_row_grand_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot1_Row_Grand_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot1_Row_Grand_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "X1 Total 100"), pt!(1, "X5 Total 50"), pt!(1, "Grand Total 600")],
    ),
    case!(
        pivot2_row,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name Sum of Price"), pt!(1, "A X1 100"), pt!(1, "B X2 200")],
    ),
    case!(
        pivot2_row_compact,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Compact",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Compact.xlsx",
        pages: 2,
        contains: [pt!(1, "Row Labels Sum of Price"), pt!(1, "X1 100"), pt!(1, "X4 100")],
    ),
    case!(
        pivot2_row_grand,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Grand",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Grand.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name Sum of Price"), pt!(1, "Grand Total 600")],
    ),
    case!(
        pivot2_row_grand_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Grand_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Grand_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "A Total 150"), pt!(1, "B Total 350"), pt!(1, "Grand Total 600")],
    ),
    case!(
        pivot2_row_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "A Total 150"), pt!(1, "B Total 350"), pt!(1, "C Total 100")],
    ),
    case!(
        pivot2_row_subtotals_sort_desc,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Subtotals_SortDescendingAll",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Subtotals_SortDescendingAll.xlsx",
        pages: 2,
        contains: [pt!(1, "C X4 100"), pt!(1, "B X3 150"), pt!(1, "A X5 50")],
    ),
    case!(
        pivot3_column_grand_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot3_Column_Grand_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot3_Column_Grand_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name"), pt!(1, "Grand Total"), pt!(1, "100 50 150 200 150 350 100 100 600")],
    ),
    case!(
        pivot4_column_grand_subtotals_sort_desc,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot4_Column_Grand_Subtotals_SortDescending",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot4_Column_Grand_Subtotals_SortDescending.xlsx",
        pages: 2,
        contains: [
            pt!(1, "Type Name"),
            pt!(1, "Grand Total"),
            pt!(1, "100 100 200 150 350 100 50"),
            pt!(1, "600"),
        ],
    ),
    case!(
        new_cond_format_export,
        source: "../core/sc/qa/unit/cond_format.cxx:testConditionalFormatExportXLSX",
        file: "sc/qa/unit/data/xlsx/new_cond_format_test_export.xlsx",
        pages: 3,
        contains: [
            pt!(0, "top n elements bottom n elements top n percent bottom n percent"),
            pt!(1, "above average below average above equal average below equal average"),
            pt!(1, "1.00 2 2.00"),
        ],
        // Calibri Office controls keep this heading only on the continuation.
        not_contains: [pt!(0, "above average")],
    ),
];

#[test]
fn xlsx_unnamed_chart_series_use_office_legend_names() {
    let document = xlsx_layout("sc/qa/unit/data/xlsx/chart_hyperlink.xlsx").unwrap();
    assert_page_contains(&document, 0, "Series1");
    assert_page_contains(&document, 0, "Series2");
    assert_page_not_contains(&document, 0, "Row 1");
    assert_page_not_contains(&document, 0, "Column 1");
}

#[test]
fn xlsx_literal_number_format_does_not_center_a_numeric_value() {
    let document = xlsx_layout("sc/qa/unit/data/xlsx/check-boolean.xlsx").unwrap();
    let items = page_display_items(&document, 0);
    let mut values = items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) if text.text == "TRUE" => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| a.origin.y.0.total_cmp(&b.origin.y.0));
    assert_eq!(values.len(), 2);
    // A1 is boolean and centered; A2 is numeric with a literal-only format
    // and remains right-aligned. Office's corresponding origins differ by
    // 15.84 pt; centering both values would put them at the same x coordinate.
    let offset = values[1].origin.x.0 - values[0].origin.x.0;
    assert!(
        (offset - 15.84).abs() < 1.5,
        "numeric alignment offset: {offset}"
    );
}

#[test]
fn xlsx_custom_table_total_font_overrides_column_format_snapshot() {
    // The Office PDF paints both Total and £350.00 bold in the custom
    // table's total-row color. Column totalsRowDxfId records store regular
    // black fonts, so applying those last loses the visible table style.
    let document = xlsx_layout("sc/qa/unit/data/xlsx/totalsRowFunction.xlsx").unwrap();
    let items = page_display_items(&document, 0);
    for column in [1, 5] {
        let run = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(run)
                    if run
                        .source
                        .as_ref()
                        .is_some_and(|source| source.path == [6, column]) =>
                {
                    Some(run)
                }
                _ => None,
            })
            .unwrap();
        assert!(run.style.bold);
        assert_eq!(
            run.color,
            Color {
                r: 14,
                g: 99,
                b: 105,
                a: 255
            }
        );
    }
}

#[test]
fn xlsx_differential_font_distinguishes_omitted_and_disabled_effects() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    // ECMA-376 §18.8.14–15 defines DXF as an incremental format;
    // §18.8.2 distinguishes b@val=false from an absent b element.
    let path = ooxmlsdk_layout_test::corpus_file(
        "../Apache-POI/test-data/spreadsheet/sheetProtection_not_protected.xlsx",
    );
    let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let workbook = package.workbook_part().unwrap();
    let sheet = workbook.worksheet_parts(&package).next().unwrap();
    *sheet.root_element_mut(&mut package).unwrap() = x::Worksheet::from_bytes(br#"
      <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <sheetData>
          <row r="1"><c r="A1" s="1"><v>1</v></c></row>
          <row r="2"><c r="A2" s="1"><v>2</v></c></row>
          <row r="3"><c r="A3" s="0"><v>3</v></c></row>
          <row r="4"><c r="A4" s="1"><v>4</v></c></row>
        </sheetData>
        <conditionalFormatting sqref="A1"><cfRule type="expression" dxfId="0" priority="1"><formula>TRUE()</formula></cfRule></conditionalFormatting>
        <conditionalFormatting sqref="A2"><cfRule type="expression" dxfId="1" priority="2"><formula>TRUE()</formula></cfRule></conditionalFormatting>
        <conditionalFormatting sqref="A3"><cfRule type="expression" dxfId="2" priority="3"><formula>TRUE()</formula></cfRule></conditionalFormatting>
        <conditionalFormatting sqref="A4"><cfRule type="expression" dxfId="3" priority="4"><formula>TRUE()</formula></cfRule></conditionalFormatting>
      </worksheet>"#).unwrap();
    let styles = workbook.workbook_styles_part(&package).unwrap();
    *styles.root_element_mut(&mut package).unwrap() = x::Stylesheet::from_bytes(br#"
      <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <fonts count="2"><font><sz val="11"/><name val="Arial"/></font><font><sz val="11"/><name val="Arial"/><b/><i/><u/><strike/></font></fonts>
        <cellXfs count="2"><xf fontId="0"/><xf fontId="1" applyFont="1"/></cellXfs>
        <dxfs count="4">
          <dxf><font><b val="0"/><i val="false"/><u val="none"/><strike val="0"/></font></dxf>
          <dxf><font><color rgb="FFFF0000"/></font></dxf>
          <dxf><font><b/><i/><u/><strike/></font></dxf>
          <dxf><font><b val="false"/></font></dxf>
        </dxfs>
      </styleSheet>"#).unwrap();
    let document =
        ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
    let expected = [[false; 4], [true; 4], [true; 4], [false, true, true, true]];
    let items = page_display_items(&document, 0);
    for (row, effects) in expected.iter().enumerate() {
        let run = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(run)
                    if run
                        .source
                        .as_ref()
                        .is_some_and(|source| source.path == [row, 0]) =>
                {
                    Some(run)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(
            [
                run.style.bold,
                run.style.italic,
                run.style.underline,
                run.style.strikethrough
            ],
            *effects,
            "row {}",
            row + 1
        );
        assert_eq!(
            run.color,
            Color {
                r: if row == 1 { 255 } else { 0 },
                g: 0,
                b: 0,
                a: 255
            }
        );
    }
}

#[test]
fn xlsx_light_up_fill_preserves_office_bitmap_brush() {
    // Office's PDF image is an 8x8 RGB tile repeated every 0.96pt. Check
    // both automatic black/white colors and an independently authored gray.
    let files = [
        (
            "../Apache-POI/test-data/spreadsheet/FillWithoutColor.xlsx",
            0,
        ),
        (
            "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/spreadsheet/O12 templates/2006-2007StudentAttendanceRecord_TP10196270.xltx",
            166,
        ),
    ];
    for (file, gray) in files {
        let document = xlsx_layout(file).unwrap();
        let patterns = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Path(path) => match path.fill {
                    Fill::Pattern(pattern) => Some(pattern),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!patterns.is_empty(), "{file}");
        for pattern in patterns {
            assert_eq!(
                pattern.mask,
                ooxmlsdk_layout::common::PatternMask::Bitmap8([
                    0x18, 0x30, 0x60, 0xc0, 0x81, 0x03, 0x06, 0x0c
                ])
            );
            assert_eq!(pattern.tile_size_milli_points, 960);
            assert!(pattern.page_origin);
            // The existing theme-tint conversion differs from Office by
            // one channel level for the attendance template (165 vs 166).
            // Keep that separate from exact brush topology and scale.
            assert!(pattern.foreground.r.abs_diff(gray) <= 1);
            assert_eq!(pattern.foreground.r, pattern.foreground.g);
            assert_eq!(pattern.foreground.r, pattern.foreground.b);
            assert_eq!(pattern.foreground.a, 255);
            assert_eq!(
                pattern.background,
                Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255
                }
            );
        }
    }
}

#[test]
fn xlsx_conditional_statistics_paint_office_top_and_bottom_ties() {
    let red = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    // The configured Office PDFs paint A3:A4 for top two, and both tied
    // A1:A2 values for bottom ten percent of the four numeric cells.
    for (file, expected_rows) in [("CFTop", [2, 3]), ("CFBottom", [0, 1])] {
        let document = xlsx_layout(&format!(
            "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/{file}.xlsx"
        ))
        .unwrap();
        let items = page_display_items(&document, 0);
        let fills = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Rect(rect) if rect.fill == Fill::Solid(red) => Some(rect.bounds),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(fills.len(), 2, "{file}: {fills:?}");
        for row in 0..4 {
            let run = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(run)
                        if run
                            .source
                            .as_ref()
                            .is_some_and(|source| source.path == [row, 0]) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .unwrap();
            let midline_y = run.origin.y.0 + run.line_height.0 * 0.5;
            let filled = fills.iter().any(|bounds| {
                midline_y >= bounds.origin.y.0
                    && midline_y < bounds.origin.y.0 + bounds.size.height.0
            });
            assert_eq!(
                filled,
                expected_rows.contains(&row),
                "{file} row {}: origin {:?}, fills {fills:?}",
                row + 1,
                run.origin
            );
        }
    }
}

#[test]
fn xlsx_conditional_statistics_exclude_text_and_boolean_cells() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    let rules = [
        ("top10\" rank=\"2", [false, false, true, true]),
        (
            "top10\" rank=\"10\" percent=\"1\" bottom=\"1",
            [true, true, false, false],
        ),
        ("aboveAverage", [false, false, false, true]),
        (
            "aboveAverage\" equalAverage=\"1",
            [false, false, true, true],
        ),
        (
            "aboveAverage\" aboveAverage=\"0",
            [true, true, false, false],
        ),
        (
            "aboveAverage\" aboveAverage=\"0\" equalAverage=\"1",
            [true, true, true, false],
        ),
    ];
    // Mean is exactly 2; numeric-looking strings and the boolean must
    // neither alter the population nor receive the statistical format.
    let values = [
        "<v>1</v>",
        "<v>1</v>",
        "<v>2</v>",
        "<v>4</v>",
        "<is><t>100</t></is>",
        "<is><t>-100</t></is>",
        "<v>1</v>",
    ];
    let mut rows = String::new();
    for (index, value) in values.iter().enumerate() {
        let row = index + 1;
        let data_type = match index {
            4 | 5 => " t=\"inlineStr\"",
            6 => " t=\"b\"",
            _ => "",
        };
        rows.push_str(&format!("<row r=\"{row}\">"));
        for col in 'A'..='F' {
            rows.push_str(&format!("<c r=\"{col}{row}\"{data_type}>{value}</c>"));
        }
        rows.push_str("</row>");
    }
    let mut conditions = String::new();
    for (col, (attributes, _)) in ('A'..='F').zip(rules.iter()) {
        conditions.push_str(&format!("<conditionalFormatting sqref=\"{col}1:{col}7\"><cfRule type=\"{attributes}\" dxfId=\"0\" priority=\"1\"/></conditionalFormatting>"));
    }
    let path = ooxmlsdk_layout_test::corpus_file(
        "../Apache-POI/test-data/spreadsheet/sheetProtection_not_protected.xlsx",
    );
    let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let workbook = package.workbook_part().unwrap();
    let sheet = workbook.worksheet_parts(&package).next().unwrap();
    *sheet.root_element_mut(&mut package).unwrap() = x::Worksheet::from_bytes(format!("<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>{rows}</sheetData>{conditions}</worksheet>").as_bytes()).unwrap();
    let styles = workbook.workbook_styles_part(&package).unwrap();
    styles.root_element_mut(&mut package).unwrap().differential_formats = Some(x::DifferentialFormats::from_bytes(br#"<dxfs xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1"><dxf><font><color rgb="FF0000FF"/></font><fill><patternFill patternType="solid"><bgColor rgb="FFFF0000"/></patternFill></fill></dxf></dxfs>"#).unwrap());
    let document =
        ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
    let items = page_display_items(&document, 0);
    for (col, (_, expected)) in rules.iter().enumerate() {
        for row in 0..values.len() {
            let run = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(run)
                        if run
                            .source
                            .as_ref()
                            .is_some_and(|source| source.path == [row, col]) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                run.color,
                Color {
                    r: 0,
                    g: 0,
                    b: if expected.get(row).copied().unwrap_or(false) {
                        255
                    } else {
                        0
                    },
                    a: 255
                },
                "row {}, column {}",
                row + 1,
                col + 1
            );
        }
    }
    assert_eq!(items.iter().filter(|item| matches!(item, DisplayItem::Rect(rect) if rect.fill == Fill::Solid(Color { r: 255, g: 0, b: 0, a: 255 }))).count(), 12);
}

#[test]
fn xlsx_conditional_text_comparisons_preserve_types_and_formula_references() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    let cases = [
        ("equal", "alpha", "ALPHA", "", true),
        ("notEqual", "beta", "ALPHA", "", true),
        ("lessThan", "delta", "OMEGA", "", true),
        ("lessThanOrEqual", "epsilon", "EPSILON", "", true),
        ("greaterThan", "zeta", "BETA", "", true),
        ("greaterThanOrEqual", "eta", "ETA", "", true),
        ("between", "middle", "Z", "A", true),
        ("notBetween", "outside", "M", "N", true),
        ("lessThan", "2", "10", "", false),
        ("equal", "a\"b", "a\"b", "", true),
        ("equal", "#N/A", "#N/A", "", true),
        ("notEqual", "match", "MATCH", "", false),
        ("between", "upper", "M", "N", false),
        ("notBetween", "inside", "H", "J", false),
    ];
    let mut rows = String::new();
    let mut rules = String::new();
    for (index, (operator, value, first, second, _)) in cases.iter().enumerate() {
        let row = index + 1;
        rows.push_str(&format!("<row r=\"{row}\"><c r=\"A{row}\" t=\"inlineStr\"><is><t>{value}</t></is></c><c r=\"B{row}\" t=\"inlineStr\"><is><t>{first}</t></is></c><c r=\"C{row}\" t=\"inlineStr\"><is><t>{second}</t></is></c></row>"));
        rules.push_str(&format!("<conditionalFormatting sqref=\"A{row}\"><cfRule type=\"cellIs\" operator=\"{operator}\" dxfId=\"0\" priority=\"{row}\"><formula>B{row}</formula><formula>C{row}</formula></cfRule></conditionalFormatting>"));
    }
    rows.push_str("<row r=\"15\"><c r=\"A15\"><v>2</v></c><c r=\"B15\"><v>10</v></c></row>");
    rules.push_str("<conditionalFormatting sqref=\"A15\"><cfRule type=\"cellIs\" operator=\"lessThan\" dxfId=\"0\" priority=\"15\"><formula>B15</formula></cfRule></conditionalFormatting>");
    let path = ooxmlsdk_layout_test::corpus_file(
        "../Apache-POI/test-data/spreadsheet/sheetProtection_not_protected.xlsx",
    );
    let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let workbook = package.workbook_part().unwrap();
    let sheet = workbook.worksheet_parts(&package).next().unwrap();
    *sheet.root_element_mut(&mut package).unwrap() = x::Worksheet::from_bytes(format!("<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><cols><col min=\"1\" max=\"3\" width=\"20\" customWidth=\"1\"/></cols><sheetData>{rows}</sheetData>{rules}</worksheet>").as_bytes()).unwrap();
    let styles = workbook.workbook_styles_part(&package).unwrap();
    styles.root_element_mut(&mut package).unwrap().differential_formats = Some(x::DifferentialFormats::from_bytes(br#"<dxfs xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1"><dxf><font><color rgb="FFFF0000"/></font></dxf></dxfs>"#).unwrap());
    // An @ number format does not change a numeric cell's declared type.
    styles.root_element_mut(&mut package).unwrap().cell_formats = Some(x::CellFormats::from_bytes(br#"<cellXfs xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1"><xf numFmtId="49" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/></cellXfs>"#).unwrap());
    let document =
        ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
    let items = page_display_items(&document, 0);
    for (index, (_, value, _, _, matches)) in cases.iter().enumerate() {
        let run = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(run)
                    if run
                        .source
                        .as_ref()
                        .is_some_and(|source| source.path == [index, 0]) =>
                {
                    Some(run)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(run.text, *value);
        assert_eq!(
            run.color,
            Color {
                r: if *matches { 255 } else { 0 },
                g: 0,
                b: 0,
                a: 255
            },
            "row {}",
            index + 1
        );
    }

    let numeric = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Text(run)
                if run
                    .source
                    .as_ref()
                    .is_some_and(|source| source.path == [14, 0]) =>
            {
                Some(run)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(numeric.text, "2");
    assert_eq!(
        numeric.color,
        Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );

    // The immutable Office reference paints these four letter cells with
    // their conditional backgrounds and white text (D20:D23).
    let document = xlsx_layout("sc/qa/unit/data/xlsx/tdf148820.xlsx").unwrap();
    for (index, value) in ["A", "B", "C", "D"].iter().enumerate() {
        let run = page_display_items(&document, 0)
            .into_iter()
            .find_map(|item| match item {
                DisplayItem::Text(run)
                    if run
                        .source
                        .as_ref()
                        .is_some_and(|source| source.path == [19 + index, 3]) =>
                {
                    Some(run)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(run.text, *value);
        assert_eq!(
            run.color,
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
    }
}

#[test]
fn xlsx_conditional_formulas_paint_office_cells_across_ranges() {
    let red = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    let expression =
        xlsx_layout("../Apache-POI/test-data/spreadsheet/WithConditionalFormatting.xlsx").unwrap();
    assert_text_color(&expression, "A8 > 5", red);

    // Office paints D1, B3, D3, A4 and B4. All three sqref regions use A1
    // as the origin of B2's relative reference, including the D1:G5 region.
    let document = xlsx_layout(
        "../Apache-POI/test-data/spreadsheet/conditional_formatting_multiple_ranges.xlsx",
    )
    .unwrap();
    let bounds = page_display_items(&document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Rect(rect) if rect.fill == Fill::Solid(red) => Some(rect.bounds),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(bounds.len(), 5, "Office's five red cells: {bounds:?}");
    let left = bounds
        .iter()
        .map(|r| r.origin.x.0)
        .fold(f32::INFINITY, f32::min);
    let top = bounds
        .iter()
        .map(|r| r.origin.y.0)
        .fold(f32::INFINITY, f32::min);
    let width = bounds[0].size.width.0;
    let height = bounds[0].size.height.0;
    let mut cells = bounds
        .iter()
        .map(|r| {
            (
                ((r.origin.x.0 - left) / width).round() as i32,
                ((r.origin.y.0 - top) / height).round() as i32,
            )
        })
        .collect::<Vec<_>>();
    cells.sort();
    assert_eq!(cells, [(0, 3), (1, 2), (1, 3), (3, 0), (3, 2)]);
}

#[test]
fn xlsx_formula_dates_share_the_configured_clock_and_workbook_epoch() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::options::{FieldUpdateDateTime, LayoutOptions};

    // ECMA-376 §18.17.7.326 gives these two TODAY serials for 2006-02-25.
    // Reuse a corpus package, replacing its worksheet only in memory.
    let path = ooxmlsdk_layout_test::corpus_file(
        "../Apache-POI/test-data/spreadsheet/sheetProtection_not_protected.xlsx",
    );
    for (date_1904, serial) in [(false, "38773"), (true, "37311")] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        workbook
            .root_element_mut(&mut package)
            .unwrap()
            .workbook_properties
            .get_or_insert_with(Default::default)
            .date1904 = Some(date_1904.into());
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        *sheet.root_element_mut(&mut package).unwrap() = x::Worksheet::from_bytes(
            br#"
          <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <cols><col min="1" max="1" width="30" customWidth="1"/></cols>
            <sheetData>
              <row r="1"><c r="A1"><f>TODAY()</f><v>1</v></c></row>
              <row r="2"><c r="A2"><f>YEAR(TODAY())</f><v>1</v></c></row>
              <row r="3"><c r="A3"><f>DATE(2006,2,25)</f><v>1</v></c></row>
              <row r="4"><c r="A4"><v>999</v></c></row>
            </sheetData>
            <conditionalFormatting sqref="A4">
              <cfRule type="expression" dxfId="0" priority="1" stopIfTrue="1">
                <formula>FALSE()</formula>
              </cfRule>
              <cfRule type="expression" dxfId="0" priority="2">
                <formula>AND(TODAY()=A1,DATE(2006,2,25)=A3)</formula>
              </cfRule>
            </conditionalFormatting>
          </worksheet>"#,
        )
        .unwrap();
        let styles = workbook.workbook_styles_part(&package).unwrap();
        styles.root_element_mut(&mut package).unwrap().differential_formats = Some(
            x::DifferentialFormats::from_bytes(br#"<dxfs xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1"><dxf><font><color rgb="FFFF0000"/></font><numFmt numFmtId="164" formatCode="&quot;clock match &quot;0"/></dxf></dxfs>"#).unwrap(),
        );
        let options = LayoutOptions {
            field_update_datetime: Some(FieldUpdateDateTime {
                year: 2006,
                month: 2,
                day: 25,
                hour: 23,
                minute: 59,
                second: 59,
            }),
            ..LayoutOptions::default()
        };
        let document = ooxmlsdk_layout::xlsx::layout_document(&package, &options).unwrap();
        assert_page_text_occurrences(&document, 0, serial, 2);
        assert_page_contains(&document, 0, "2006");
        assert_page_contains(&document, 0, "clock match 999");
        assert_text_color(
            &document,
            "clock match 999",
            Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
        );
    }
}

#[test]
fn xlsx_letter_printer_canvas_owns_unscaled_column_breaks() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk_layout::LayoutOptions;
    use ooxmlsdk_layout::common::{DebugRecord, DebugValue};

    // The related DEVMODE requests a Letter worksheet canvas; the PDF is A4.
    // Office's configured SimSun theme starts the continuation at K, whereas
    // retaining Calibri starts at J and repeats the I-column overflow string.
    let path = ooxmlsdk_layout_test::corpus_file("../Apache-POI/test-data/spreadsheet/59264.xlsx");
    let package = SpreadsheetDocument::new_from_file(&path).unwrap();
    for (language, start_column, repeats_medium_dashed) in
        [("zh-CN", 11, false), ("en-US", 10, true)]
    {
        let mut options = LayoutOptions {
            default_document_language: Some(language.into()),
            ..Default::default()
        };
        options.diagnostics.collect_debug_records = true;
        let document = ooxmlsdk_layout::xlsx::layout_document(&package, &options).unwrap();
        let page = document
            .debug_records
            .iter()
            .find_map(|record| match record {
                DebugRecord::Shape(shape)
                    if shape.kind == "xlsx_print_page" && shape.page_index == 1 =>
                {
                    Some(shape)
                }
                _ => None,
            })
            .expect("second worksheet page");
        let value = |name| {
            page.metadata
                .iter()
                .find_map(|property| match property.value {
                    DebugValue::Integer(value) if property.name == name => Some(value),
                    _ => None,
                })
        };
        assert_eq!(value("start_column"), Some(start_column), "{language}");
        assert_eq!(value("paper_scale_percent"), Some(95));
        assert_eq!(value("pagination_paper_scale_percent"), Some(100));
        assert_page_not_contains(&document, 0, "MEDIUM_DASH_DOT");
        assert_page_contains(&document, 1, "MEDIUM_DASH_DOT");
        assert_page_text_occurrences(
            &document,
            1,
            "MEDIUM_DASHED",
            usize::from(repeats_medium_dashed),
        );
        assert_page_size(&document, 1, 595.32, 841.92);
    }

    let graph_paper = xlsx_layout("../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/spreadsheet/O12 templates/GraphPaper_TP10193274.xltx").unwrap();
    assert_eq!(
        graph_paper.pages.len(),
        3,
        "explicit-height grids must retain A4 row pagination"
    );

    let wide_cell = xlsx_layout("../Apache-POI/test-data/spreadsheet/58325_db.xlsx").unwrap();
    assert_page_contains(&wide_cell, 0, "Area de Anexos");
    assert_page_contains(&wide_cell, 1, "Area de Anexos");
}

#[test]
fn xlsx_cell_text_continues_across_horizontal_pages_unless_blocked() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    // The configured Office PDF repeats I13's text on page 2, with its ink
    // clipped to the continuation page. An occupied J13 must stop that spill.
    let path = ooxmlsdk_layout_test::corpus_file(
        "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/O14ISOStrict/Excel/Photo Formats - CGM-O12-XL-Pictures.xlsx",
    );
    for blocked in [false, true] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        if blocked {
            let sheets = package
                .workbook_part()
                .unwrap()
                .worksheet_parts(&package)
                .collect::<Vec<_>>();
            let mut changed = 0;
            for sheet in sheets {
                for row in &mut sheet.root_element_mut(&mut package).unwrap().sheet_data.row {
                    if row
                        .cell
                        .iter()
                        .any(|cell| cell.cell_reference.as_deref() == Some("I13"))
                    {
                        row.cell.push(x::Cell::from_bytes(br#"<c xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" r="J13" t="inlineStr"><is><t>block</t></is></c>"#).unwrap());
                        changed += 1;
                    }
                }
            }
            assert_eq!(changed, 1);
        }
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        assert_page_contains(&document, 0, "image with a style applied");
        if blocked {
            assert_page_not_contains(&document, 1, "image with a style applied");
        } else {
            assert_page_contains(&document, 1, "image with a style applied");
        }
    }
}

#[test]
fn xlsx_chart_background_images_resolve_strict_emf_resources() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::sdk::SdkType;
    use ooxmlsdk_layout::LayoutOptions;

    let file = ooxmlsdk_layout_test::corpus_file(
        "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/O14ISOStrict/Excel/Photo Formats - CGM-O12-XL-Pictures.xlsx",
    );
    let mut image_counts = Vec::new();
    for tiled in [false, true] {
        let mut package = SpreadsheetDocument::new_from_file(&file).unwrap();
        if tiled {
            let workbook = package.workbook_part().unwrap();
            let drawings = workbook
                .worksheet_parts(&package)
                .filter_map(|sheet| sheet.drawings_part(&package))
                .chain(
                    workbook
                        .chartsheet_parts(&package)
                        .filter_map(|sheet| sheet.drawings_part(&package)),
                )
                .collect::<Vec<_>>();
            let charts = drawings
                .iter()
                .flat_map(|drawing| drawing.chart_parts(&package))
                .collect::<Vec<_>>();
            let mut changed = 0;
            for chart in charts {
                let space = chart.root_element_mut(&mut package).unwrap();
                let Some(c::ShapePropertiesChoice2::BlipFill(fill)) = space
                    .shape_properties
                    .as_mut()
                    .and_then(|properties| properties.shape_properties_choice2.as_mut())
                else {
                    continue;
                };
                fill.blip_fill_choice = Some(a::BlipFillChoice::Tile(Box::new(a::Tile::from_bytes(
                    br#"<a:tile xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" sx="50000" sy="50000"/>"#,
                ).unwrap())));
                changed += 1;
            }
            assert_eq!(changed, 1);
        }
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = (0..document.pages.len())
            .flat_map(|page| page_display_items(&document, page))
            .collect::<Vec<_>>();
        assert!(!items.iter().any(|item| matches!(item, DisplayItem::Path(path) if matches!(path.fill, Fill::Image { .. }))));
        image_counts.push(
            items
                .iter()
                .filter(|item| matches!(item, DisplayItem::Image(_)))
                .count(),
        );
    }
    assert!(image_counts[0] > 0);
    assert!(
        image_counts[1] > image_counts[0],
        "tile background must retain multiple placements"
    );
}

#[test]
fn xlsx_multiline_headers_and_footers_keep_separate_text_lines() {
    // ECMA-376 Part 1 §18.3.1.46's U+000A example and the configured Office
    // PDFs for these fixtures both put the authored lines on separate baselines.
    for (file, expected) in [
        (
            "sc/qa/unit/data/xlsx/tdf113271.xlsx",
            ["Workforce TeleStaff", "Accruals Report", "Limits"],
        ),
        (
            "sc/qa/unit/data/xlsx/tdf124525.xlsx",
            ["GrSA 6.0", "Generado el 01/04/2019 16:39", ""],
        ),
    ] {
        let document = xlsx_layout(file).expect("multiline header/footer layout");
        let items = page_display_items(&document, 0);
        let mut previous_y = None;
        for expected in expected.into_iter().filter(|text| !text.is_empty()) {
            let run = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(run) if run.text.starts_with(expected) => Some(run),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{file}: missing header/footer line {expected:?}"));
            assert!(
                !run.text.contains('\n'),
                "{file}: newline reached display text"
            );
            if let Some(previous_y) = previous_y {
                assert!(
                    run.origin.y.0 > previous_y,
                    "{file}: overlapping authored lines"
                );
            }
            previous_y = Some(run.origin.y.0);
        }
    }
}

#[test]
fn xlsx_clustered_column_chart_uses_semantic_axis_and_legend_text() {
    // ECMA-376 Part 1 §21.2.2.16 and LibreOffice
    // VCartesianAxis::getTextLabelString: an absent category sequence exposes
    // numeric categories beginning at 1.
    let document = xlsx_layout("chart2/qa/extras/data/xlsx/chart.xlsx")
        .expect("clustered-column chart layout");
    assert_eq!(document.pages.len(), 2);
    assert_page_contains(&document, 0, "25");
    assert_page_contains(&document, 0, "1");
    assert_page_contains(&document, 1, "Col 1");
    assert_page_contains(&document, 1, "Col2");
    assert_page_contains(&document, 1, "Col 33");
}

#[test]
// Sources:
// - ECMA-376 Part 1 §21.2.2.59 (dPt) and §21.2.2.105 (marker)
// - ../core/chart2/qa/extras/chart2import.cxx::testTdf126115IndividualMarker
// - immutable Microsoft Excel fixed-format output for tdf126115.xlsx
fn xlsx_tdf126115_point_markers_override_series_geometry_and_paint() {
    let document = xlsx_layout("chart2/qa/extras/data/xlsx/tdf126115.xlsx")
        .expect("point-marker chart layout");
    let items = page_display_items(&document, 0);
    let red_square = items.iter().find_map(|item| match item {
        DisplayItem::Path(path)
            if path.fill
                == Fill::Solid(Color {
                    r: 0xff,
                    g: 0,
                    b: 0,
                    a: 0xff,
                })
                && path.stroke.as_ref().is_some_and(|stroke| {
                    stroke.color
                        == Color {
                            r: 0x5b,
                            g: 0x9b,
                            b: 0xd5,
                            a: 0xff,
                        }
                }) =>
        {
            Some(path)
        }
        _ => None,
    });
    let red_square = red_square.expect("missing red 8pt square with the inherited accent1 outline");
    assert!((red_square.bounds.size.width.0 - 8.0).abs() < 0.01);
    assert!((red_square.bounds.size.height.0 - 8.0).abs() < 0.01);

    let purple_x = items.iter().find_map(|item| match item {
        DisplayItem::Path(path)
            if path.fill == Fill::None
                && path.stroke.as_ref().is_some_and(|stroke| {
                    stroke.color
                        == Color {
                            r: 0x70,
                            g: 0x30,
                            b: 0xa0,
                            a: 0xff,
                        }
                }) =>
        {
            Some(path)
        }
        _ => None,
    });
    let purple_x = purple_x.expect("missing unfilled purple 15pt X point marker");
    assert!((purple_x.bounds.size.width.0 - 15.0).abs() < 0.01);
    assert!((purple_x.bounds.size.height.0 - 15.0).abs() < 0.01);
}

#[test]
// Sources:
// - ECMA-376 Part 1 §21.2.2.105 (`c:symbol val="none"`)
// - ../core/chart2/qa/extras/chart2import.cxx::testTdf126033IndividualMarker
// - immutable Microsoft Excel fixed-format output for tdf126033.xlsx
fn xlsx_tdf126033_point_none_suppresses_only_the_selected_points() {
    let document = xlsx_layout("chart2/qa/extras/data/xlsx/tdf126033.xlsx")
        .expect("point-marker suppression chart layout");
    let items = (0..document.pages.len())
        .flat_map(|page_index| page_display_items(&document, page_index))
        .collect::<Vec<_>>();
    let marker_paths = items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Path(path)
                if path.bounds.size.width.0 >= 4.0
                    && path.bounds.size.width.0 <= 6.0
                    && path.bounds.size.height.0 >= 4.0
                    && path.bounds.size.height.0 <= 6.0 =>
            {
                Some(path)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let marker_count = |color: Color| {
        items
            .iter()
            .filter(|item| match item {
                DisplayItem::Path(path) => {
                    path.fill == Fill::Solid(color)
                        && (path.bounds.size.width.0 - 5.0).abs() < 0.01
                        && (path.bounds.size.height.0 - 5.0).abs() < 0.01
                }
                _ => false,
            })
            .count()
    };
    assert_eq!(
        marker_count(Color {
            r: 0x5b,
            g: 0x9b,
            b: 0xd5,
            a: 0xff,
        }),
        2,
        "one blue plot point plus its legend key must remain; marker paths={marker_paths:?}"
    );
    assert_eq!(
        marker_count(Color {
            r: 0xed,
            g: 0x7d,
            b: 0x31,
            a: 0xff,
        }),
        2,
        "one orange plot point plus its legend key must remain; marker paths={marker_paths:?}"
    );
}

#[test]
// Excel retains the preceding header/footer pages and places the control on
// Sheet1's tenth page. The LibreOffice empty-range policy expected three pages.
fn xlsx_singlecontrol_retains_header_footer_pages_before_the_row_517_control() {
    let document = xlsx_layout("sc/qa/unit/data/xlsx/singlecontrol.xlsx").unwrap();
    let print_pages = debug_shapes(&document, "xlsx_print_page");
    assert_eq!(
        document.pages.len(),
        12,
        "unexpected retained print ranges: {print_pages:?}"
    );
    for page in 0..9 {
        assert_page_contains(&document, page, "Sheet1");
        assert_page_not_contains(&document, page, "form control inside cell b517");
    }
    assert_page_contains(&document, 9, "form control inside cell b517");
    assert_page_contains(&document, 10, "adfa");
    assert_page_contains(&document, 11, "adfad");
}

#[test]
fn manual_rows_do_not_freeze_the_cached_automatic_row_height() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk::sdk::{
        FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
        OpenSettings, SdkType,
    };
    use ooxmlsdk_layout::LayoutOptions;

    // Office row markers 100..103 are 13.56pt apart in both controls. Existing
    // manual rows and the row-517 control remain in place when only the cached
    // automatic defaultRowHeight changes from 14.25 to 30pt.
    for cached_height in [14.25, 30.0] {
        let mut package = SpreadsheetDocument::new_with_settings(
            std::fs::File::open(ooxmlsdk_layout_test::corpus_file(
                "sc/qa/unit/data/xlsx/singlecontrol.xlsx",
            ))
            .unwrap(),
            OpenSettings {
                markup_compatibility_process_settings: MarkupCompatibilityProcessSettings {
                    process_mode: MarkupCompatibilityProcessMode::ProcessLoadedPartsOnly,
                    target_file_format_version: FileFormatVersion::Microsoft365,
                },
                ..Default::default()
            },
        )
        .unwrap();
        let workbook_part = package.workbook_part().unwrap();
        let workbook = workbook_part.root_element(&package).unwrap();
        let sheet_id = &workbook
            .sheets
            .sheet
            .iter()
            .find(|sheet| sheet.name == "Sheet1")
            .unwrap()
            .id;
        let Some(ooxmlsdk::parts::PartRef::WorksheetPart(sheet)) =
            workbook_part.get_part_by_id(&package, sheet_id)
        else {
            panic!("missing Sheet1 worksheet part");
        };
        let worksheet = sheet.root_element_mut(&mut package).unwrap();
        worksheet
            .sheet_format_properties
            .as_mut()
            .unwrap()
            .default_row_height = cached_height;
        for row in 100..104 {
            assert!(
                !worksheet
                    .sheet_data
                    .row
                    .iter()
                    .any(|r| r.row_index == Some(row))
            );
            worksheet.sheet_data.row.push(x::Row::from_bytes(format!(
                r#"<row xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" r="{row}"><c r="A{row}" t="n"><v>{row}</v></c></row>"#
            ).as_bytes()).unwrap());
        }
        worksheet.sheet_data.row.sort_by_key(|row| row.row_index);
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        assert_eq!(
            document.pages.len(),
            12,
            "cached_height={cached_height}, pages={:?}",
            (0..document.pages.len())
                .map(|page| all_page_text(&document, page))
                .collect::<Vec<_>>()
        );
        assert_page_contains(&document, 9, "form control inside cell b517");
        let mut baselines = Vec::new();
        for row in 100..104 {
            let text = row.to_string();
            let baseline = page_display_items(&document, 1)
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(run) if run.text == text => Some(run.origin.y.0),
                    _ => None,
                })
                .expect("missing row marker on the second page");
            baselines.push(baseline);
        }
        for pair in baselines.windows(2) {
            assert!((pair[1] - pair[0] - 13.56).abs() < 0.02, "{baselines:?}");
        }
    }
}

#[test]
fn xlsx_comment_print_settings_control_note_text_and_empty_pages() {
    use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
    use ooxmlsdk_layout::LayoutOptions;

    // The configured Excel PDF prints the Comments sheet's four cells without
    // note captions. The later sheets containing only notes must not add pages.
    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/Comments/AddingComments.xlsx",
    );
    for mode in [
        Some(x::CellCommentsValues::None),
        None,
        Some(x::CellCommentsValues::AsDisplayed),
    ] {
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let sheets = package
            .workbook_part()
            .unwrap()
            .worksheet_parts(&package)
            .collect::<Vec<_>>();
        for sheet in sheets {
            sheet
                .root_element_mut(&mut package)
                .unwrap()
                .page_setup
                .as_mut()
                .unwrap()
                .cell_comments = mode;
        }
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        if mode == Some(x::CellCommentsValues::AsDisplayed) {
            assert_page_contains(&document, 0, "Start on Col E");
            assert_page_contains(&document, 0, "Automatic Size");
        } else {
            assert_eq!(document.pages.len(), 2, "note-only sheets must not print");
            assert_page_contains(&document, 1, "I have a hidden comment");
            let mut words = all_page_text(&document, 0)
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            words.sort();
            assert_eq!(words, ["Hidden", "On", "Top", "Underneath", "Visible"]);
        }
    }
}

#[test]
fn xlsx_layout_matches_libreoffice_layout_coverage() {
    let failures = run_named_cases_parallel(
        CASES,
        |case| case.name,
        run_case,
        |case, message| {
            format!(
                "{}\n  source: {}\n  file: {}\n  failure: {}",
                case.name, case.source, case.file, message
            )
        },
    );
    assert!(
        failures.is_empty(),
        "{} XLSX layout cases failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn run_case(case: &XlsxCase) {
    let document = xlsx_layout(case.file).unwrap_or_else(|error| {
        panic!(
            "{}: failed to build layout for {}; source={}: {error}",
            case.name, case.file, case.source
        )
    });
    let page_text = document
        .pages
        .iter()
        .enumerate()
        .map(|(page_index, _)| format!("{page_index}: {:?}", all_page_text(&document, page_index)))
        .collect::<Vec<_>>()
        .join("; ");
    assert_eq!(
        document.pages.len(),
        case.page_count,
        "{} page count mismatch; source={}; file={}; pages=[{}]",
        case.name,
        case.source,
        case.file,
        page_text
    );
    for expected in case.contains {
        assert_page_contains(&document, expected.page, expected.text);
    }
    for expected in case.contains_any {
        assert_page_contains_any(&document, expected.page, expected.alternatives);
    }
    for unexpected in case.not_contains {
        assert_page_not_contains(&document, unexpected.page, unexpected.text);
    }
    for expected in case.occurrences {
        assert_page_text_occurrences(&document, expected.page, expected.text, expected.count);
    }
    for expected in case.page_sizes {
        assert_page_size(&document, expected.page, expected.width, expected.height);
    }
    for expected in case.image_counts {
        assert_page_image_count(&document, expected.page, expected.count);
    }
    for expected in case.link_targets {
        assert_link_target(&document, expected);
    }
    for expected in case.path_minimums {
        assert_page_path_count_at_least(&document, expected.page, expected.count);
    }
    for expected in case.font_sizes {
        assert_text_font_size(&document, expected.text, expected.size);
    }
    for expected in case.colors {
        assert_text_color(&document, expected.text, expected.color);
    }
}

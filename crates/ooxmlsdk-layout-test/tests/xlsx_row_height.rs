use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk_layout::{LayoutOptions, common::DisplayItem};
use ooxmlsdk_layout_test::{corpus_file, page_display_items};

#[test]
fn mixed_authored_row_grid_preserves_office_pagination() {
    // The configured Office export has one page. Replacing only the default
    // grid with Arial 10 printer metrics while retaining its authored row
    // sizes splits it over two pages. Those sizes need their own conversion.
    let package = SpreadsheetDocument::new_from_file(corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/TryToLoad/WorksheetWithExclamationMark.xlsx",
    ))
    .unwrap();
    let document = ooxmlsdk_layout::xlsx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("ko-KR".into()),
            format_locale: Some("zh-CN".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(document.pages.len(), 1);
}

#[test]
fn automatic_times_new_roman_rows_follow_the_office_printer_font() {
    // Office controls on the original tdf130104_indent and copies with cached
    // defaultRowHeight=15/30 preserve 12.72pt rows. Changing Normal to 11pt
    // makes the rows 13.92pt. Screen COM RowHeight gives different values.
    for (font_size, cached_height, expected_height) in [
        (10.0, 12.75, 12.72),
        (10.0, 15.0, 12.72),
        (10.0, 30.0, 12.72),
        (11.0, 12.75, 13.92),
    ] {
        let mut package = SpreadsheetDocument::new_from_file(corpus_file(
            "../LibreOffice/sc/qa/unit/data/xlsx/tdf130104_indent.xlsx",
        ))
        .unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        sheet
            .root_element_mut(&mut package)
            .unwrap()
            .sheet_format_properties
            .as_mut()
            .unwrap()
            .default_row_height = cached_height;
        let styles = workbook.workbook_styles_part(&package).unwrap();
        let fonts = &mut styles
            .root_element_mut(&mut package)
            .unwrap()
            .fonts
            .as_mut()
            .unwrap()
            .xml_children;
        let x::FontsChoice::Font(normal) = &mut fonts[0] else {
            panic!("fixture Normal font is missing");
        };
        for choice in &mut normal.font_choice {
            if let x::FontChoice::FontSize(size) = choice {
                size.val = font_size;
            }
        }
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("en-US".into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let items = page_display_items(&document, 0);
        let text_y = |label: &str| {
            items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text == label => Some(text.origin.y.0),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing fixture text: {label}"))
        };
        let first_y = text_y("Indent by 0");
        // Text occupies rows 1, 3, 6 and 9. Multiple distances catch cumulative
        // row drift without depending on the PDF font's baseline convention.
        for (label, preceding_rows) in [
            ("Indent by 1", 2.0),
            ("Indent by 2", 5.0),
            ("Indent by 3", 8.0),
        ] {
            let actual = text_y(label) - first_y;
            let expected = preceding_rows * expected_height;
            assert!(
                (actual - expected).abs() < 0.02,
                "{font_size}pt, cached {cached_height}, {label}: {actual} != {expected}"
            );
        }
    }
}

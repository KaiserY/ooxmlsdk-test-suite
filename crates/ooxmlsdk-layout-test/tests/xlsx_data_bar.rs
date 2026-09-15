use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk_layout::{
    LayoutOptions,
    common::{DisplayItem, Fill, PathItem},
};
use ooxmlsdk_layout_test::{assert_page_text_occurrences, page_display_items, xlsx_layout};

const DATA_BAR: &str =
    "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/CFDataBar.xlsx";

fn gradient_bars<'a>(document: &'a ooxmlsdk_layout::LayoutDocument<'a>) -> Vec<&'a PathItem<'a>> {
    page_display_items(document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Path(path) if path.closed && matches!(path.fill, Fill::Gradient(_)) => {
                Some(path)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn data_bars_hide_values_including_cells_with_zero_length_bars() {
    // Office's unchanged PDF contains two red gradients (A3/A4) and no text.
    // A1/A2 equal the minimum: their zero-length bars still hide the values.
    let document = xlsx_layout(DATA_BAR).unwrap();
    assert_eq!(document.pages.len(), 1);
    assert!(
        !page_display_items(&document, 0)
            .iter()
            .any(|item| matches!(item, DisplayItem::Text(_)))
    );
    let bars = gradient_bars(&document);
    assert_eq!(bars.len(), 2);
    assert!((bars[0].bounds.size.width.0 / bars[1].bounds.size.width.0 - 0.5).abs() < 0.01);
    for bar in bars {
        let Fill::Gradient(fill) = &bar.fill else {
            unreachable!()
        };
        assert_eq!(
            (
                fill.stops[0].color.r,
                fill.stops[0].color.g,
                fill.stops[0].color.b
            ),
            (255, 0, 0)
        );
        assert_eq!(fill.stops[1].color.a, 255);
    }
}

#[test]
fn negative_data_bars_keep_independent_value_visibility_and_reverse_gradients() {
    // Office prints A's four values, hides C's four values, and draws eight
    // bars: five green positive bars and three red negative bars.
    let document = xlsx_layout("../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/CFDataBarNegative.xlsx").unwrap();
    let text = page_display_items(&document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(text, ["-1", "1", "2", "3"]);
    let bars = gradient_bars(&document);
    assert_eq!(bars.len(), 8);
    let mut negative = 0;
    for bar in bars {
        let Fill::Gradient(fill) = &bar.fill else {
            unreachable!()
        };
        let (start, end) = fill.line.unwrap();
        if fill.stops[0].color.r == 255 {
            negative += 1;
            assert!(start.x.0 > end.x.0);
        } else {
            assert!(start.x.0 < end.x.0);
        }
    }
    assert_eq!(negative, 3);
}

#[test]
fn data_bars_respect_stop_if_true_without_a_differential_style() {
    let path = ooxmlsdk_layout_test::corpus_file(
        "../ClosedXML/ClosedXML.Tests/Resource/Examples/ConditionalFormatting/CFStopIfTrue.xlsx",
    );
    let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let workbook = package.workbook_part().unwrap();
    let sheet = workbook.worksheet_parts(&package).next().unwrap();
    let worksheet = sheet.root_element_mut(&mut package).unwrap();
    let rules = &mut worksheet.conditional_formatting[0].conditional_formatting_rule;
    assert!(rules[0].format_id.is_none());
    assert!(rules[0].stop_if_true.unwrap().as_bool());
    let lower = &mut rules[1];
    lower.r#type = x::ConditionalFormatValues::DataBar;
    lower.icon_set = None;
    lower.data_bar = Some(Box::new(x::DataBar {
        show_value: Some(false.into()),
        conditional_format_value_object: vec![
            x::ConditionalFormatValueObject {
                r#type: x::ConditionalFormatValueObjectValues::Number,
                val: Some("0".into()),
                ..Default::default()
            },
            x::ConditionalFormatValueObject {
                r#type: x::ConditionalFormatValueObjectValues::Number,
                val: Some("10".into()),
                ..Default::default()
            },
        ],
        color: Box::new(x::Color {
            rgb: Some("FFFF0000".into()),
            ..Default::default()
        }),
        ..Default::default()
    }));
    let document =
        ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
    assert_page_text_occurrences(&document, 0, "6", 1);
    for value in ["1", "2", "3"] {
        assert_page_text_occurrences(&document, 0, value, 0);
    }
    assert_eq!(gradient_bars(&document).len(), 3);
}

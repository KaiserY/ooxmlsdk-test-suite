use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk::sdk::SdkType;
use ooxmlsdk_layout::{LayoutOptions, common::DisplayItem};
use ooxmlsdk_layout_test::{assert_page_text_occurrences, corpus_file, page_display_items};

#[test]
fn error_cells_follow_office_ui_resources_without_translating_text() {
    let codes = [
        "#NULL!",
        "#DIV/0!",
        "#VALUE!",
        "#REF!",
        "#NAME?",
        "#NUM!",
        "#N/A",
        "#GETTING_DATA",
    ];
    let formulas = [
        "A1:A2 C1:C2",
        "1/0",
        "1+\"a\"",
        "#REF!",
        "missingUdf()",
        "SQRT(-1)",
        "NA()",
    ];
    let mut rows = String::new();
    for (index, code) in codes.iter().enumerate() {
        let row = index + 1;
        rows.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}" t="inlineStr"><is><t>{code}</t></is></c><c r="B{row}" t="e"><v>{code}</v></c>"#
        ));
        if let Some(formula) = formulas.get(index) {
            rows.push_str(&format!(
                r#"<c r="C{row}" t="e"><f>{formula}</f><v>{code}</v></c>"#
            ));
        }
        rows.push_str(&format!(
            r#"<c r="D{row}" t="inlineStr"><is><t>{code}</t></is></c></row>"#
        ));
    }
    let xml = format!(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetFormatPr defaultRowHeight="18"/><cols><col min="1" max="4" width="29" customWidth="1"/></cols><sheetData>{rows}</sheetData><pageMargins left="0.25" right="0.25" top="0.25" bottom="0.25" header="0" footer="0"/><pageSetup paperSize="9" orientation="landscape" scale="100"/></worksheet>"#
    );
    let path =
        corpus_file("../ClosedXML/ClosedXML.Tests/Resource/Other/Formulas/ArrayFormula.xlsx");
    let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
    let workbook = package.workbook_part().unwrap();
    let sheet = workbook.worksheet_parts(&package).next().unwrap();
    *sheet.root_element_mut(&mut package).unwrap() =
        x::Worksheet::from_bytes(xml.as_bytes()).unwrap();

    // Eight Office exports keep format/authoring locale at zh-CN while
    // changing only UI language. Cached and calculated errors agree.
    for (language, expected) in [
        ("en-US", codes),
        (
            "de-DE",
            [
                "#NULL!",
                "#DIV/0!",
                "#WERT!",
                "#BEZUG!",
                "#NAME?",
                "#ZAHL!",
                "#NV",
                "#DATEN_ABRUFEN",
            ],
        ),
        (
            "es-MX",
            [
                "#NULO!",
                "#DIV/0!",
                "#VALOR!",
                "#REF!",
                "#NOMBRE?",
                "#N¡NUM!",
                "#N/D",
                "#OBTENIENDO_DATOS",
            ],
        ),
        (
            "fr-CA",
            [
                "#NUL!",
                "#DIV/0!",
                "#VALEUR!",
                "#REF!",
                "#NOM?",
                "#NOMBRE!",
                "#N/A",
                "#GETTING_DATA",
            ],
        ),
        ("ja-JP", codes),
        ("ko-KR", codes),
        ("zh-CN", codes),
        ("zh-TW", codes),
    ] {
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some(language.into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(document.pages.len(), 1, "{language}");
        let items = page_display_items(&document, 0);
        for (row, code) in codes.iter().enumerate() {
            for (column, expected_text) in [
                (0, *code),
                (1, expected[row]),
                (2, expected[row]),
                (3, *code),
            ] {
                if row == 7 && column == 2 {
                    continue;
                }
                let text = items
                    .iter()
                    .filter_map(|item| match item {
                        DisplayItem::Text(text)
                            if text
                                .source
                                .as_ref()
                                .is_some_and(|source| source.path == [row, column]) =>
                        {
                            Some(text.text.as_ref())
                        }
                        _ => None,
                    })
                    .collect::<String>();
                assert_eq!(text, expected_text, "{language} row {row} column {column}");
            }
        }
    }
}

#[test]
fn external_udf_error_uses_the_configured_mexican_spanish_resource() {
    let package = SpreadsheetDocument::new_from_file(&corpus_file(
        "../Apache-POI/test-data/spreadsheet/56502.xlsx",
    ))
    .unwrap();
    let document = ooxmlsdk_layout::xlsx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("es-MX".into()),
            format_locale: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_page_text_occurrences(&document, 0, "#NOMBRE?", 1);
    assert_page_text_occurrences(&document, 0, "#NAME?", 0);
}

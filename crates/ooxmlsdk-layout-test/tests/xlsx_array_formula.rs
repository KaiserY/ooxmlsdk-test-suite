use ooxmlsdk::parts::{
    cell_metadata_part::CellMetadataPart, spreadsheet_document::SpreadsheetDocument,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk::sdk::SdkType;
use ooxmlsdk_layout::{LayoutOptions, common::DisplayItem};
use ooxmlsdk_layout_test::{page_display_items, xlsx_layout};

const ARRAY: &str = "../ClosedXML/ClosedXML.Tests/Resource/Other/Formulas/ArrayFormula.xlsx";

#[test]
fn fixed_array_scalar_result_fills_the_office_output_region() {
    // The unchanged configured Office PDF prints 3 in all four cells A1:B2.
    let document = xlsx_layout(ARRAY).unwrap();
    let text = page_display_items(&document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(text, ["3", "3", "3", "3"]);
}

#[test]
fn fixed_array_broadcast_preserves_scalar_types_and_replaces_stale_members() {
    for (formula, expected) in [
        ("1-1", "0"),
        ("\"array\"", "array"),
        ("TRUE()", "TRUE"),
        ("1/0", "#DIV/0!"),
    ] {
        let path = ooxmlsdk_layout_test::corpus_file(ARRAY);
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        let worksheet = sheet.root_element_mut(&mut package).unwrap();
        for row in &mut worksheet.sheet_data.row {
            for cell in &mut row.cell {
                cell.cell_value = Some(x::CellValue(x::XstringType {
                    xml_content: Some("99".into()),
                    ..Default::default()
                }));
            }
        }
        worksheet.sheet_data.row[0].cell[0]
            .cell_formula
            .as_mut()
            .unwrap()
            .xml_content = Some(formula.into());
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let text = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) => Some(text.text.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(text, [expected; 4], "{formula}");
    }
}

#[test]
fn scalar_broadcast_distinguishes_dynamic_metadata_and_normal_formulas() {
    for (dynamic, array, count) in [
        (None, false, 1),
        (Some(true), true, 1),
        (Some(false), true, 4),
    ] {
        let path = ooxmlsdk_layout_test::corpus_file(ARRAY);
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        let worksheet = sheet.root_element_mut(&mut package).unwrap();
        let anchor = &mut worksheet.sheet_data.row[0].cell[0];
        anchor.cell_formula.as_mut().unwrap().formula_type = Some(if array {
            x::CellFormulaValues::Array
        } else {
            x::CellFormulaValues::Normal
        });
        if let Some(dynamic) = dynamic {
            anchor.cell_meta_index = Some(1);
            let xml = format!(
                r#"<metadata xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:xda="http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray"><metadataTypes><metadataType name="XLDAPR" minSupportedVersion="120000"/></metadataTypes><futureMetadata name="XLDAPR"><bk><extLst><ext uri="{{bdbb8cdc-fa1e-496e-a857-3c3f30c029c3}}"><xda:dynamicArrayProperties fDynamic="{dynamic}"/></ext></extLst></bk></futureMetadata><cellMetadata><bk><rc t="1" v="0"/></bk></cellMetadata></metadata>"#
            );
            let metadata = workbook
                .add_new_part_auto_id::<_, CellMetadataPart>(&mut package)
                .unwrap();
            metadata
                .set_root_element(
                    &mut package,
                    x::Metadata::from_bytes(xml.as_bytes()).unwrap(),
                )
                .unwrap();
        }
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let text = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) => Some(text.text.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(text, vec!["3"; count], "dynamic={dynamic:?}, array={array}");
    }
}

#[test]
fn named_array_results_keep_their_matrix_shape() {
    for name in ["numbers", "empty_array"] {
        let path = ooxmlsdk_layout_test::corpus_file(ARRAY);
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        workbook
            .root_element_mut(&mut package)
            .unwrap()
            .defined_names = Some(x::DefinedNames {
            defined_name: vec![x::DefinedName {
                name: name.into(),
                xml_content: Some("{1,2;3,4}".into()),
                ..Default::default()
            }],
        });
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        sheet.root_element_mut(&mut package).unwrap().sheet_data.row[0].cell[0]
            .cell_formula
            .as_mut()
            .unwrap()
            .xml_content = Some(name.into());
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let text = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) => Some(text.text.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(text, ["1", "2", "3", "4"], "{name}");
    }
}

#[test]
fn named_array_constants_distinguish_missing_items_and_quoted_text() {
    for (formula, expected) in [
        ("{1,,3}", vec!["#NAME?"; 4]),
        ("{1,2;3}", vec!["#NAME?"; 4]),
        ("{1,0;3,4}", vec!["1", "0", "3", "4"]),
        (r#"{1,"";3,4}"#, vec!["1", "3", "4"]),
        (r#"{1,"2";3,4}"#, vec!["1", "2", "3", "4"]),
        (r#"{1,"a,b;c";3,4}"#, vec!["1", "a,b;c", "3", "4"]),
    ] {
        let path = ooxmlsdk_layout_test::corpus_file(ARRAY);
        let mut package = SpreadsheetDocument::new_from_file(&path).unwrap();
        let workbook = package.workbook_part().unwrap();
        workbook
            .root_element_mut(&mut package)
            .unwrap()
            .defined_names = Some(x::DefinedNames {
            defined_name: vec![x::DefinedName {
                name: "values".into(),
                xml_content: Some(formula.into()),
                ..Default::default()
            }],
        });
        let sheet = workbook.worksheet_parts(&package).next().unwrap();
        sheet.root_element_mut(&mut package).unwrap().sheet_data.row[0].cell[0]
            .cell_formula
            .as_mut()
            .unwrap()
            .xml_content = Some("values".into());
        let document =
            ooxmlsdk_layout::xlsx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let text = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) => Some(text.text.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(text, expected, "{formula}");
    }
}

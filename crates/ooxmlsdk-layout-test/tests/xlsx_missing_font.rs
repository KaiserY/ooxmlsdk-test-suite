use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use ooxmlsdk_layout::{LayoutOptions, common::DisplayItem, fonts::FontResolver};
use ooxmlsdk_layout_test::{corpus_file, page_display_items};

#[test]
fn missing_swiss_face_uses_simsun_and_synthetic_bold_but_installed_arial_stays_arial() {
    // Office controls on tdf134392, including an unchanged original, resolve
    // FreeSans + family=2 + charset=1 to SimSun. The otherwise identical Arial
    // control keeps Arial and its real bold face. Both use the zh-CN host locale.
    for (authored, expected) in [("FreeSans", "SimSun"), ("Arial", "Arial")] {
        let mut package = SpreadsheetDocument::new_from_file(&corpus_file(
            "../LibreOffice/sc/qa/unit/data/xlsx/tdf134392.xlsx",
        ))
        .unwrap();
        let workbook = package.workbook_part().unwrap();
        let styles = workbook.workbook_styles_part(&package).unwrap();
        for child in &mut styles
            .root_element_mut(&mut package)
            .unwrap()
            .fonts
            .as_mut()
            .unwrap()
            .xml_children
        {
            let x::FontsChoice::Font(font) = child else {
                continue;
            };
            for choice in &mut font.font_choice {
                if let x::FontChoice::FontName(name) = choice
                    && name.val == "FreeSans"
                {
                    name.val = authored.into();
                }
            }
        }
        let document = ooxmlsdk_layout::xlsx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("de-DE".into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let mut resolver = FontResolver::default();
        let mut regular = 0;
        let mut bold = 0;
        for item in page_display_items(&document, 0) {
            let DisplayItem::Text(text) = item else {
                continue;
            };
            if text.style.font_family.as_deref() != Some(authored) {
                continue;
            }
            let actual = resolver.load_text_face(&text.style).unwrap();
            let mut expected_style = text.style.clone();
            expected_style.font_family = Some(expected.into());
            expected_style.fallback_font_family = None;
            let expected_face = resolver.load_text_face(&expected_style).unwrap();
            assert_eq!(actual, expected_face, "{authored}: {}", text.text);
            assert_eq!(
                actual.synthetic_bold,
                authored == "FreeSans" && text.style.bold
            );
            if text.style.bold {
                bold += 1;
            } else {
                regular += 1;
            }
        }
        assert!(
            regular > 0 && bold > 0,
            "{authored}: both styles must be exercised"
        );
    }
}

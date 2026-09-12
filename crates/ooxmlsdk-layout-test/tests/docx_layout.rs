use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main as w;
use ooxmlsdk_layout::common::{Color, DisplayItem, Fill};
use ooxmlsdk_layout::{LayoutOptions, docx};
use ooxmlsdk_layout_test::{
    assert_close, assert_page_contains, docx_layout, docx_layout_with_options, line_heights,
    page_display_items, page_text_contains, row_heights, table_row_count_for_block,
    text_origins_for,
};
use std::path::Path;

fn assert_page_run_text_occurrences(
    document: &ooxmlsdk_layout::common::LayoutDocument<'_>,
    page_index: usize,
    expected: &str,
    expected_count: usize,
) {
    // Font fallback can split one Japanese or Korean word into several runs.
    // Preserve authored spaces while joining those emitted text fragments.
    let text: String = page_display_items(document, page_index)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) => Some(run.text.as_ref()),
            _ => None,
        })
        .collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let expected = expected.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        text.matches(&expected).count(),
        expected_count,
        "text {expected:?} occurrence mismatch on page {page_index}; page_text={text:?}"
    );
}

#[test]
fn calibri_symbol_uses_its_unicode_cmap_despite_duplicate_symbol_charset() {
    let file = "sw/qa/extras/uiwriter/data/tdf164620.docx";
    for reverse_calibri_entries in [false, true] {
        let mut package =
            WordprocessingDocument::new_from_file(ooxmlsdk_layout_test::corpus_file(file)).unwrap();
        if reverse_calibri_entries {
            let main = package.main_document_part().unwrap();
            let font_part = main.font_table_part(&package).unwrap();
            let font_table = font_part.root_element_mut(&mut package).unwrap();
            let indices: Vec<_> = font_table
                .xml_children
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| match entry {
                    w::FontsChoice::Font(font) if font.name == "Calibri" => Some(index),
                    _ => None,
                })
                .collect();
            assert_eq!(indices.len(), 2, "retained duplicate Calibri declarations");
            font_table.xml_children.swap(indices[0], indices[1]);
        }
        let document = docx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("es-MX".into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..LayoutOptions::default()
            },
        )
        .unwrap();
        let symbols: Vec<_> = (0..document.pages.len())
            .flat_map(|page| page_display_items(&document, page))
            .filter_map(|item| match item {
                DisplayItem::Text(run) if run.style.explicit_symbol_character => Some(run),
                _ => None,
            })
            .collect();
        assert_eq!(symbols.len(), 1, "the source has one explicit w:sym");
        let symbol = symbols[0];
        assert_eq!(symbol.text, "&", "Office emits WinAnsi Calibri ampersand");
        assert_eq!(symbol.style.font_family.as_deref(), Some("Calibri"));
        assert_eq!(
            symbol.style.high_ansi_font_family.as_deref(),
            Some("Calibri")
        );
        assert_eq!(
            symbol.style.east_asia_font_family.as_deref(),
            Some("Calibri")
        );
        assert_eq!(symbol.style.complex_font_family.as_deref(), Some("Calibri"));
        assert_eq!(symbol.style.symbol_font_family.as_deref(), Some("Calibri"));
        assert!(!symbol.style.wordprocessingml_font_slots);
        let shaped = ooxmlsdk_layout::fonts::shape_text_runs(&symbol.text, &symbol.style)
            .expect("configured Calibri font must be available for the imported regression");
        assert_eq!(shaped.len(), 1);
        assert!(!shaped[0].approximate);
        assert!(shaped[0].diagnostics.missing_glyphs.is_empty());
        assert!(shaped[0].diagnostics.fallback_runs.is_empty());
        assert_eq!(shaped[0].glyphs.len(), 1);
        assert_ne!(shaped[0].glyphs[0].glyph_id, 0);
        assert_eq!(shaped[0].glyphs[0].source_char, Some('&'));
    }
}

#[test]
fn address_block_placeholders_use_chinese_ui_resources_and_keep_authored_text() {
    for (file, language, placeholder, trailing_text) in [
        (
            "sw/qa/extras/ww8export/data/tdf134264.docx",
            "zh-TW",
            "«地址區塊»",
            None,
        ),
        (
            "sw/qa/extras/odfexport/data/tdf129520.docx",
            "zh-CN",
            "«地址块»",
            Some("Cette facture fait office de garantie"),
        ),
    ] {
        for (ui_language, format_locale, expected, absent) in [
            (language, "fr-FR", placeholder, "«AddressBlock»"),
            ("en-US", language, "«AddressBlock»", placeholder),
        ] {
            let document = docx_layout_with_options(
                file,
                &LayoutOptions {
                    ui_language: Some(ui_language.into()),
                    format_locale: Some(format_locale.into()),
                    default_document_language: Some("en-US".into()),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_page_run_text_occurrences(&document, 0, expected, 1);
            assert_page_run_text_occurrences(&document, 0, absent, 0);
            assert_page_run_text_occurrences(&document, 0, "1815", 0);
            if let Some(text) = trailing_text {
                assert_page_run_text_occurrences(&document, 0, text, 1);
            }
            let mut delimiters = 0;
            for item in page_display_items(&document, 0) {
                let DisplayItem::Text(run) = item else {
                    continue;
                };
                if run.text.contains('«') || run.text.contains('»') {
                    delimiters += run.text.matches('«').count() + run.text.matches('»').count();
                    assert_eq!(run.style.font_family.as_deref(), Some("Garamond"));
                    assert!(!run.style.bold);
                    assert_close(run.style.font_size.0, 12.0, 0.01, "Office address-block em");
                }
                if run.text.contains('»') {
                    // Both retained Office streams paint a final Latin blank.
                    assert!(
                        run.text.ends_with(' '),
                        "preserved address-block trailing blank"
                    );
                }
            }
            assert_eq!(delimiters, 2);
        }
    }
}

#[test]
fn japanese_generated_diagnostics_use_resource_fonts_without_changing_authored_text() {
    use ooxmlsdk::sdk::SdkType;

    let options = LayoutOptions {
        ui_language: Some("ja-JP".into()),
        format_locale: Some("zh-CN".into()),
        default_document_language: Some("zh-CN".into()),
        ..Default::default()
    };
    for (file, message, latin_face) in [
        (
            "sw/qa/extras/ooxmlexport/data/fdo78910.docx",
            "エラー! ブックマークが定義されていません。",
            Some("Calibri"),
        ),
        (
            "sw/qa/extras/ooxmlexport/data/tdf171299_tableInField.docx",
            "エラー! 参照元が見つかりません。",
            Some("Arial"),
        ),
        (
            "sw/qa/core/docnode/data/redline-ends-before-toc.docx",
            "目次項目が見つかりません。",
            None,
        ),
        (
            "sw/qa/extras/ooxmlexport/data/sdt-citation-run.docx",
            "無効な資料文献が指定されました。",
            None,
        ),
    ] {
        let document = docx_layout_with_options(file, &options).unwrap();
        assert_page_run_text_occurrences(&document, 0, message, 1);
        let mut japanese_characters = 0;
        let mut latin_punctuation = 0;
        for item in page_display_items(&document, 0) {
            let DisplayItem::Text(run) = item else {
                continue;
            };
            let count = run
                .text
                .chars()
                .filter(|character| !character.is_ascii())
                .count();
            if count > 0 {
                japanese_characters += count;
                assert_eq!(
                    run.style.east_asia_font_family.as_deref(),
                    Some("MS Mincho"),
                    "{file}: {}",
                    run.text
                );
                assert!(run.style.wordprocessingml_font_slots);
                assert!(!run.style.wordprocessingml_east_asia_language_is_chinese);
                assert!(run.style.bold);
            }
            if run.text.contains('!') {
                latin_punctuation += 1;
                assert_eq!(
                    run.style.font_family.as_deref(),
                    latin_face,
                    "{file}: ASCII diagnostic face"
                );
            }
        }
        assert_eq!(
            japanese_characters,
            message
                .chars()
                .filter(|character| !character.is_ascii())
                .count()
        );
        assert_eq!(latin_punctuation, usize::from(latin_face.is_some()));
    }

    // The generated citation resource does not change an authored Japanese
    // run, even when document-language defaults select a different CJK face.
    let path =
        ooxmlsdk_layout_test::corpus_file("sw/qa/extras/ooxmlexport/data/sdt-citation-run.docx");
    let mut package = WordprocessingDocument::new_from_file(path).unwrap();
    let authored = w::Paragraph::from_bytes(
        r#"<w:p xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:r><w:rPr><w:rFonts w:ascii="Yu Gothic" w:hAnsi="Yu Gothic" w:eastAsia="Yu Gothic" w:cs="Yu Gothic"/><w:b w:val="0"/><w:lang w:val="ja-JP" w:eastAsia="ja-JP"/></w:rPr><w:t>著者の文章</w:t></w:r></w:p>"#.as_bytes(),
    ).unwrap();
    let main = package.main_document_part().unwrap();
    main.root_element_mut(&mut package)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .body_choice
        .push(w::BodyChoice::Paragraph(Box::new(authored)));
    let document = docx::layout_document(&package, &options).unwrap();
    assert_page_run_text_occurrences(&document, 0, "無効な資料文献が指定されました。", 1);
    assert_page_run_text_occurrences(&document, 0, "著者の文章", 1);
    let authored_runs = page_display_items(&document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) if run.text.contains("著者") => Some(run),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!authored_runs.is_empty());
    for run in authored_runs {
        assert_eq!(
            run.style.east_asia_font_family.as_deref(),
            Some("Yu Gothic")
        );
        assert!(!run.style.bold);
    }
}

#[test]
fn greeting_line_resources_follow_the_office_ui_language_for_simple_and_complex_fields() {
    for (file, ui_language, expected) in [
        (
            "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/O14ISOStrict/Word/fldSimple-Field Definitions-GreetingLine.docx",
            "es-MX",
            "«Línea de saludo»",
        ),
        (
            "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/mailmerge.docx",
            "ja-JP",
            "«あいさつ文»",
        ),
    ] {
        let localized = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some(ui_language.into()),
                format_locale: Some("en-US".into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&localized, 0, expected, 1);
        assert_page_run_text_occurrences(&localized, 0, "«GreetingLine»", 0);
        let english = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some("en-US".into()),
                format_locale: Some(ui_language.into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&english, 0, "«GreetingLine»", 1);
        assert_page_run_text_occurrences(&english, 0, expected, 0);
        if ui_language == "ja-JP" {
            for text in ["Junzha", "Hello"] {
                assert_page_contains(&localized, 0, text);
                assert_page_contains(&english, 0, text);
            }
            let japanese_runs = page_display_items(&localized, 0)
                .into_iter()
                .filter_map(|item| match item {
                    DisplayItem::Text(run)
                        if run
                            .text
                            .chars()
                            .any(|character| "あいさつ文".contains(character)) =>
                    {
                        Some(run)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!japanese_runs.is_empty());
            for run in japanese_runs {
                // The source Hans theme uses 宋体, whose font-table altName
                // is SimSun (the Office PDF face). Preserve that authored
                // slot instead of applying the MS Mincho diagnostic face.
                assert_eq!(run.style.east_asia_font_family.as_deref(), Some("宋体"));
                assert!(!run.style.bold);
            }
        }
    }
}

#[test]
fn traditional_chinese_bookmark_resource_keeps_latin_face_and_explicit_plain_style() {
    use ooxmlsdk::sdk::SdkType;

    let file = "sw/qa/extras/ooxmlexport/data/tdf64531.docx";
    for (ui_language, format_locale, expected, absent) in [
        (
            "zh-TW",
            "en-US",
            "錯誤! 尚未定義書籤。",
            "Error! Bookmark not defined.",
        ),
        (
            "en-US",
            "zh-TW",
            "Error! Bookmark not defined.",
            "錯誤! 尚未定義書籤。",
        ),
    ] {
        let document = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some(ui_language.into()),
                format_locale: Some(format_locale.into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&document, 0, expected, 1);
        assert_page_run_text_occurrences(&document, 0, absent, 0);
        assert_page_run_text_occurrences(&document, 0, "錯誤！ 書籤未定義。", 0);
        assert_page_run_text_occurrences(&document, 0, "Heading 1", 1);
        let mut chinese_characters = 0;
        let mut ascii_exclamations = 0;
        for item in page_display_items(&document, 0) {
            let DisplayItem::Text(run) = item else {
                continue;
            };
            let count = run
                .text
                .chars()
                .filter(|character| !character.is_ascii())
                .count();
            if count > 0 {
                chinese_characters += count;
                assert_eq!(run.style.east_asia_font_family.as_deref(), Some("PMingLiU"));
                assert!(run.style.wordprocessingml_font_slots);
                assert!(run.style.wordprocessingml_east_asia_language_is_chinese);
                // Office's diagnostic is plain despite the inherited bold
                // heading style: the paragraph mark explicitly has w:b=false.
                assert!(!run.style.bold);
                assert_close(run.style.font_size.0, 12.0, 0.01, "Office resource em");
            }
            if run.text.contains('!') {
                ascii_exclamations += run.text.matches('!').count();
                assert_eq!(
                    run.style.font_family.as_deref(),
                    Some("Lucida Sans Unicode")
                );
                assert!(!run.style.bold);
            }
        }
        assert_eq!(
            chinese_characters,
            expected.chars().filter(|c| !c.is_ascii()).count()
        );
        assert_eq!(ascii_exclamations, 1);
    }

    let mut package =
        WordprocessingDocument::new_from_file(ooxmlsdk_layout_test::corpus_file(file)).unwrap();
    let authored = w::Paragraph::from_bytes(
        r#"<w:p xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:r><w:rPr><w:rFonts w:ascii="Microsoft JhengHei" w:hAnsi="Microsoft JhengHei" w:eastAsia="Microsoft JhengHei" w:cs="Microsoft JhengHei"/><w:b w:val="0"/><w:lang w:val="zh-TW" w:eastAsia="zh-TW"/></w:rPr><w:t>作者的文字</w:t></w:r></w:p>"#.as_bytes(),
    ).unwrap();
    let main = package.main_document_part().unwrap();
    main.root_element_mut(&mut package)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .body_choice
        .push(w::BodyChoice::Paragraph(Box::new(authored)));
    let document = docx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("zh-TW".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_page_run_text_occurrences(&document, 0, "錯誤! 尚未定義書籤。", 1);
    assert_page_run_text_occurrences(&document, 0, "作者的文字", 1);
    let authored_runs: Vec<_> = page_display_items(&document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) if run.text.contains("作者") => Some(run),
            _ => None,
        })
        .collect();
    assert!(!authored_runs.is_empty());
    for run in authored_runs {
        assert_eq!(
            run.style.east_asia_font_family.as_deref(),
            Some("Microsoft JhengHei")
        );
        assert!(!run.style.bold);
    }
}

#[test]
fn french_reference_diagnostic_uses_the_office_resource_and_keeps_cell_text() {
    let file = "sw/qa/extras/ooxmlexport/data/tdf171383_refFieldInCell.docx";
    for (ui_language, format_locale, expected, absent) in [
        (
            "fr-CA",
            "en-US",
            "Erreur! Source du renvoi introuvable.",
            "Error! Reference source not found.",
        ),
        (
            "en-US",
            "fr-CA",
            "Error! Reference source not found.",
            "Erreur! Source du renvoi introuvable.",
        ),
    ] {
        let document = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some(ui_language.into()),
                format_locale: Some(format_locale.into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&document, 0, expected, 1);
        assert_page_run_text_occurrences(&document, 0, absent, 0);
        for text in ["1", "Xyz", "YELLOW"] {
            assert_page_contains(&document, 0, text);
        }
    }
}

#[test]
fn empty_bibliography_and_missing_citation_source_use_distinct_japanese_resources() {
    let file = "sw/qa/extras/ooxmlexport/data/99_Fields.docx";
    let japanese = docx_layout_with_options(
        file,
        &LayoutOptions {
            ui_language: Some("ja-JP".into()),
            format_locale: Some("en-US".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    for message in [
        "作業中の文書には元データがありません。",
        "資料文献が指定されていません。",
    ] {
        assert_page_run_text_occurrences(&japanese, 0, message, 1);
        // Office emits each full diagnostic as continuous MS Mincho text.
        // In particular, the shared kana letter ー must not split データ into
        // portions separated by Latin/East Asian automatic spacing.
        let runs: Vec<_> = page_display_items(&japanese, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(run) if run.text == message => Some(run),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 1, "continuous diagnostic: {message}");
        assert_close(runs[0].origin.x.0, 72.02, 0.03, "Office diagnostic start");
        assert_close(
            runs[0].style.font_size.0,
            11.04,
            0.01,
            "Office fullwidth em",
        );
    }
    assert_page_run_text_occurrences(&japanese, 0, "###bidi outline", 1);
    assert_page_run_text_occurrences(&japanese, 0, "無効な資料文献が指定されました。", 0);
    for item in page_display_items(&japanese, 0) {
        let DisplayItem::Text(run) = item else {
            continue;
        };
        if !run.text.is_ascii() {
            assert_eq!(
                run.style.east_asia_font_family.as_deref(),
                Some("MS Mincho")
            );
            assert!(run.style.bold);
        }
    }
    let english = docx_layout_with_options(
        file,
        &LayoutOptions {
            ui_language: Some("en-US".into()),
            format_locale: Some("ja-JP".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    for message in [
        "There are no sources in the current document.",
        "No source specified.",
        "###bidi outline",
    ] {
        assert_page_run_text_occurrences(&english, 0, message, 1);
    }
    assert_page_run_text_occurrences(&english, 0, "資料文献が指定されていません。", 0);
}

#[test]
fn missing_bookmark_fields_use_the_german_and_japanese_office_resources() {
    for (file, language, page, message, count) in [
        (
            "../Pandoc/test/docx/golden/image.docx",
            "de-DE",
            1,
            "Fehler! Verweisquelle konnte nicht gefunden werden.",
            2,
        ),
        (
            "sw/qa/extras/ooxmlexport/data/fdo78910.docx",
            "ja-JP",
            0,
            "エラー! ブックマークが定義されていません。",
            1,
        ),
        (
            "sw/qa/extras/ooxmlexport/data/tdf171299_tableInField.docx",
            "ja-JP",
            0,
            "エラー! 参照元が見つかりません。",
            1,
        ),
    ] {
        let document = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some(language.into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("en-US".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&document, page, message, count);
        assert!(!page_text_contains(
            &document,
            page,
            "Error! Reference source not found."
        ));
        assert!(!page_text_contains(
            &document,
            page,
            "Error! Bookmark not defined."
        ));
    }
}

#[test]
fn spanish_waterfall_uses_office_axis_and_legend_resources() {
    let file =
        "../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/Of16-06.docx";
    let document = docx_layout_with_options(
        file,
        &LayoutOptions {
            ui_language: Some("es-MX".into()),
            format_locale: Some("en-US".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    for text in [
        "Título del gráfico",
        "Título del eje",
        "Aumento",
        "Disminución",
        "Total",
    ] {
        assert_page_contains(&document, 0, text);
    }
    for text in ["Axis Title", "Increase", "Decrease"] {
        assert!(
            !page_text_contains(&document, 0, text),
            "stale English resource: {text}"
        );
    }
}

#[test]
fn empty_toc_messages_use_the_japanese_and_french_office_ui_resources() {
    // Existing configured Office PDFs: b9c7b075... (Japanese complex TOC),
    // 13fe0e8d... (French simple TOC). The stored results use different text.
    for (file, language, expected, authored) in [
        (
            "sw/qa/core/docnode/data/redline-ends-before-toc.docx",
            "ja-JP",
            "目次項目が見つかりません。",
            "start",
        ),
        (
            "sw/qa/core/exportdata/ooxml/pass/sdt-before-table.docx",
            "fr-CA",
            "Aucune entrée de table des matières n'a été trouvée.",
            "Contents",
        ),
    ] {
        let localized = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some(language.into()),
                format_locale: Some("zh-CN".into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&localized, 0, expected, 1);
        assert_page_contains(&localized, 0, authored);

        let english = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some("en-US".into()),
                format_locale: Some(language.into()),
                default_document_language: Some(language.into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_page_run_text_occurrences(&english, 0, "NO TABLE OF CONTENTS ENTRIES FOUND.", 1);
        assert_page_run_text_occurrences(&english, 0, expected, 0);
    }
}

#[test]
fn german_automatic_chart_title_preserves_authored_captions() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
    use ooxmlsdk::sdk::SdkType;

    let file = "chart2/qa/extras/data/docx/tdf121744.docx";
    let options = LayoutOptions {
        ui_language: Some("de-DE".into()),
        format_locale: Some("en-US".into()),
        default_document_language: Some("zh-CN".into()),
        ..Default::default()
    };
    let document = docx_layout_with_options(file, &options).unwrap();
    assert_page_contains(&document, 0, "Diagrammtitel");
    assert_page_contains(&document, 0, "Kategória 1");
    assert!(!page_text_contains(&document, 0, "Chart Title"));

    // Only automatic titles are UI resources. An authored English title
    // stays English even while the application UI is German.
    let path = ooxmlsdk_layout_test::corpus_file(file);
    let mut package = WordprocessingDocument::new_from_file(&path).unwrap();
    let main = package.main_document_part().unwrap();
    let chart = main.chart_parts(&package).next().unwrap();
    chart.root_element_mut(&mut package).unwrap().chart.title = Some(Box::new(
        c::Title::from_bytes(br#"<c:title xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Authored Chart Title</a:t></a:r></a:p></c:rich></c:tx></c:title>"#).unwrap(),
    ));
    let document = docx::layout_document(&package, &options).unwrap();
    assert_page_contains(&document, 0, "Authored Chart Title");
    assert!(!page_text_contains(&document, 0, "Diagrammtitel"));
}

#[test]
fn korean_style_ref_diagnostic_replaces_stale_english_heading_results() {
    let file = "sw/qa/core/fields/data/suppress-non-numerical.docx";
    let document = docx_layout_with_options(
        file,
        &LayoutOptions {
            ui_language: Some("ko-KR".into()),
            format_locale: Some("en-US".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    // One complex and three simple STYLEREF fields. Current Korean Office
    // rejects the named English built-ins while retaining the style names in
    // the diagnostic; the original numbered paragraphs remain visible.
    for heading in ["Heading 1", "Heading 2"] {
        let message =
            format!("오류! 여기에 표시할 텍스트에 {heading} 을(를) 적용하려면 홈 탭을 사용하세요.");
        assert_page_run_text_occurrences(&document, 0, &message, 2);
    }
    assert_page_contains(&document, 0, "List items");
    assert_page_contains(&document, 0, "Baz");

    let english = docx_layout_with_options(
        file,
        &LayoutOptions {
            ui_language: Some("en-US".into()),
            format_locale: Some("ko-KR".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!page_text_contains(&english, 0, "오류!"));
    assert!(!page_text_contains(&english, 0, "Error! Use the Home tab"));
}

#[test]
fn invalid_citation_diagnostic_uses_ui_language_instead_of_field_language() {
    let file = "sw/qa/extras/ooxmlexport/data/sdt-citation-run.docx";
    for (ui_language, format_locale, expected, absent) in [
        (
            "ja-JP",
            "en-US",
            "無効な資料文献が指定されました。",
            "Invalid source specified.",
        ),
        (
            "en-US",
            "ja-JP",
            "Invalid source specified.",
            "無効な資料文献が指定されました。",
        ),
    ] {
        let document = docx_layout_with_options(
            file,
            &LayoutOptions {
                ui_language: Some(ui_language.into()),
                format_locale: Some(format_locale.into()),
                default_document_language: Some("zh-CN".into()),
                ..Default::default()
            },
        )
        .unwrap();
        // CITATION BBC11 \\l 1033 selects the bibliography language, while
        // the unresolved-field diagnostic is an Office UI resource.
        assert_page_contains(&document, 0, expected);
        assert_page_contains(&document, 0, "Before sdt.");
        assert_page_contains(&document, 0, "After sdt.");
        assert!(!page_text_contains(&document, 0, absent));
        assert!(!page_text_contains(&document, 0, "BBC News 2011"));
    }
}

#[test]
fn paragraph_frame_default_anchor_and_explicit_y_follow_office() {
    let frame_y = |fixture: &str| {
        let document =
            docx_layout(&format!("sw/qa/extras/ooxmlexport/data/{fixture}.docx")).unwrap();
        page_display_items(&document, 0)
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text.starts_with("Text") => Some(text.origin.y.0),
                _ => None,
            })
            .expect("frame text")
    };
    let text_relative_y = frame_y("tdf112287B");
    // Current Office PDFs differ only by the frame's vertical placement:
    // omitted vAnchor + bottom aligns to the bottom text margin; explicit
    // y=0 stays at the paragraph; nonzero y overrides yAlign. Explicit text
    // anchors keep their paragraph-relative position despite yAlign.
    for (fixture, expected_delta) in [
        ("tdf112287", 142.01),
        ("tdf157572_defaultVAnchor", -22.08),
        ("tdf157572_insidiousCombination", -0.60),
        ("tdf157572_noVAlignAsText", 0.0),
    ] {
        assert_close(
            frame_y(fixture) - text_relative_y,
            expected_delta,
            2.0,
            fixture,
        );
    }
}

#[test]
fn author_refresh_uses_the_core_text_value_before_applying_format_switches() {
    use ooxmlsdk::schemas::opc_core_properties::Creator;

    // Office configuration da652716470cac6adf670e9202f267c048ba6f64b2b87a0adf8f4632da13ae1f.
    // The current Word PDF contains ANTONI and 16 June 2010, with no XML tags.
    let file =
        ooxmlsdk_layout_test::corpus_file("../Apache-POI/test-data/document/FieldCodes.docx");
    let options = LayoutOptions {
        field_update_datetime: Some(ooxmlsdk_layout::options::FieldUpdateDateTime {
            year: 2026,
            month: 9,
            day: 11,
            hour: 15,
            minute: 57,
            second: 0,
        }),
        ui_language: Some("ja-JP".into()),
        format_locale: Some("zh-CN".into()),
        default_document_language: Some("zh-CN".into()),
        ..Default::default()
    };
    let mut package = WordprocessingDocument::new_from_file(file).unwrap();
    for (author, expected) in [
        ("Antoni", "ANTONI16 June 2010"),
        ("<Antoni & Sons>", "<ANTONI & SONS>16 June 2010"),
    ] {
        let part = package.core_file_properties_part().unwrap();
        part.root_element_mut(&mut package).unwrap().creator = Some(Creator {
            xml_content: Some(author.into()),
            ..Default::default()
        });
        let document = docx::layout_document(&package, &options).unwrap();
        assert_eq!(document.pages.len(), 1);
        let text: String = page_display_items(&document, 0)
            .into_iter()
            .filter_map(|item| match item {
                DisplayItem::Text(run) => Some(run.text.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(text, expected, "AUTHOR value {author:?}");
    }

    // Updating a different core value does not refresh the ordinary cache.
    let cached = docx::layout_document(&package, &LayoutOptions::default()).unwrap();
    assert_page_contains(&cached, 0, "ANTONI");
    assert!(!page_text_contains(&cached, 0, "SONS"));
}

#[test]
fn author_refresh_reads_creator_instead_of_the_hand_edited_field_cache() {
    let file = "sw/qa/extras/ooxmlexport/data/tdf148380_createField.docx";
    let cached = docx_layout(file).unwrap();
    assert_page_contains(&cached, 0, "Myself - that's who");
    let refreshed = docx_layout_with_options(
        file,
        &LayoutOptions {
            field_update_datetime: Some(ooxmlsdk_layout::options::FieldUpdateDateTime {
                year: 2026,
                month: 9,
                day: 11,
                hour: 15,
                minute: 57,
                second: 0,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    // The current Office PDF refreshes the AUTHOR field to dc:creator.
    assert_page_contains(&refreshed, 0, "Lorenzo Chavez");
    assert!(!page_text_contains(&refreshed, 0, "Myself - that's who"));
}

#[test]
fn last_saved_by_refresh_reads_core_properties_and_preserves_the_default_cache() {
    let file = "sw/qa/extras/ooxmlexport/data/tdf148380_modifiedField.docx";
    let cached = docx_layout(file).unwrap();
    assert_page_contains(&cached, 0, "Charlie Brown");
    let refreshed = docx_layout_with_options(
        file,
        &LayoutOptions {
            field_update_datetime: Some(ooxmlsdk_layout::options::FieldUpdateDateTime {
                year: 2026,
                month: 9,
                day: 11,
                hour: 15,
                minute: 57,
                second: 0,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    // cp:lastModifiedBy contains Charles; the authored LASTSAVEDBY cache
    // contains Charlie. Office's refreshed PDF uses the core property.
    assert_page_contains(&refreshed, 0, "Charles Brown");
    assert!(!page_text_contains(&refreshed, 0, "Charlie Brown"));
}

#[test]
fn docx_chart_plot_images_resolve_stretch_and_tile_resources() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    let file = ooxmlsdk_layout_test::corpus_file(
        "chart2/qa/extras/data/docx/fdo74115_WallBitmapFill.docx",
    );
    for tiled in [false, true] {
        let mut package = WordprocessingDocument::new_from_file(&file).unwrap();
        if tiled {
            let chart = package
                .main_document_part()
                .unwrap()
                .chart_parts(&package)
                .next()
                .unwrap();
            let space = chart.root_element_mut(&mut package).unwrap();
            let properties = space.chart.plot_area.shape_properties.as_mut().unwrap();
            let c::ShapePropertiesChoice2::BlipFill(fill) =
                properties.shape_properties_choice2.as_mut().unwrap()
            else {
                panic!("fixture must retain its plot image fill");
            };
            fill.blip_fill_choice = Some(a::BlipFillChoice::Tile(Box::default()));
        }
        let document = docx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        assert!(!items.iter().any(|item| matches!(item, DisplayItem::Path(path) if matches!(path.fill, Fill::Image { .. }))));
        let images = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Image(image) => Some(image),
                _ => None,
            })
            .collect::<Vec<_>>();
        if tiled {
            assert!(images.len() > 1);
        } else {
            assert_eq!(images.len(), 1);
        }
        for image in images {
            assert!(!image.bytes.is_empty());
            assert!(
                !image.clip_path.is_empty(),
                "plot image must retain its clipping boundary"
            );
            assert!(image.bounds.size.width.0 > 0.0 && image.bounds.size.height.0 > 0.0);
        }
    }
}

#[test]
fn docx_inline_activex_caption_moves_with_its_centered_picture() {
    let document =
        docx_layout("sw/qa/extras/ooxmlexport/data/tdf151548_activeContentDemo.docm").unwrap();
    let items = page_display_items(&document, 0);
    let caption = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Text(run) if run.text == "Synchronize form" => Some(run),
            _ => None,
        })
        .unwrap();
    let picture = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Image(image) if image.content_type.as_ref() == "image/x-wmf" => {
                Some(image)
            }
            _ => None,
        })
        .unwrap();
    // The WMF TextOut starts at x=57 in a 201-unit frame; the inline
    // picture is 150.75pt wide. Paragraph centering must preserve that
    // 42.75pt relative position (also present in the Office PDF).
    assert!((caption.origin.x.0 - picture.bounds.origin.x.0 - 42.75).abs() < 0.1);
}

#[test]
fn docx_hidden_table_paragraph_mark_keeps_caption_on_one_baseline() {
    // Office joins the caption and the following framed paragraph across
    // the caption's hidden paragraph mark in tdf164901's second table row.
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf164901.docx").unwrap();
    let items = page_display_items(&document, 0);
    let text = items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) => Some(run),
            _ => None,
        })
        .collect::<Vec<_>>();
    let caption = text.iter().find(|run| run.text.contains("Grund")).unwrap();
    let following = text
        .iter()
        .find(|run| run.text.contains("Test 1-2-3"))
        .unwrap();
    assert!((caption.origin.y.0 - following.origin.y.0).abs() < 0.01);
    // The picture paragraph inherits 1.5-line spacing. Its top remains at
    // the cell print edge, independently of the text-only baseline offset.
    let picture = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Image(image) => Some(image),
            _ => None,
        })
        .unwrap();
    assert!((picture.bounds.origin.y.0 - 73.25).abs() < 0.1);
}

#[test]
fn docx_pie_label_paragraphs_have_separate_baselines() {
    // The configured Office PDF for tdf134866 paints the custom label on
    // one line and its resolved PERCENTAGE field on the following line.
    let document =
        docx_layout("sw/qa/extras/layout/data/tdf134866.docx").expect("multiline pie-label layout");
    let items = page_display_items(&document, 0);
    let texts = items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) => Some(run),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(texts.iter().all(|run| !run.text.contains('\n')));
    let label = texts.iter().find(|run| run.text.contains("Cusom")).unwrap();
    let percentage = texts.iter().find(|run| run.text == "100%").unwrap();
    assert!(percentage.origin.y.0 > label.origin.y.0);

    // The Office circle is 198.5pt across inside a 432x252pt chart frame.
    // Adding paragraph layout must retain the Word host's plot geometry.
    let pie = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Path(path) if !path.points.is_empty() => Some(path),
            _ => None,
        })
        .expect("pie outline");
    let left = pie
        .points
        .iter()
        .map(|point| point.x.0)
        .fold(f32::INFINITY, f32::min);
    let right = pie
        .points
        .iter()
        .map(|point| point.x.0)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (right - left - 198.5).abs() < 0.5,
        "unexpected pie diameter: {}",
        right - left
    );
}

#[test]
// Sources:
// - immutable Microsoft Office fixed-format output for floattable-hidemark.docx;
// - LibreOffice commit 25ab6816990b (tdf#162730), which treats a splittable
//   floating-table anchor as real row content even when w:hideMark is present;
// - LibreOffice
//   sw/qa/writerfilter/dmapper/DomainMapperTableHandler.cxx::
//   testDOCXFloatingTableHidemark (one fly on each of exactly two pages).
fn docx_floattable_hidemark_moves_only_the_split_follow_to_page_two() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/LibreOffice/sw/qa/writerfilter/dmapper/data/floattable-hidemark.docx");
    let options = LayoutOptions {
        ui_language: Some("zh-CN".to_string()),
        ..LayoutOptions::default()
    };
    let package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let summary = docx::inspect_layout(&package, &options).unwrap();
    let document = docx::layout_document(&package, &options).unwrap();

    assert_eq!(
        document.pages.len(),
        2,
        "the inner fly master must stay on page 1 and its follow must move to page 2; rows={:?}; frames={:?}; follows={:?}",
        summary
            .rows
            .iter()
            .map(|row| (
                row.block_index,
                row.page_index,
                row.row_index,
                row.x_pt,
                row.y_pt,
                row.width_pt,
                row.height_pt,
            ))
            .collect::<Vec<_>>(),
        document
            .frames
            .iter()
            .map(|frame| (
                frame.block_index,
                frame.page_index,
                frame.kind.as_ref(),
                frame.bounds,
                frame.fragments.clone(),
            ))
            .collect::<Vec<_>>(),
        document.follows,
    );
    assert_page_contains(&document, 0, "body start");
    assert_page_contains(&document, 1, "If for any reason");
    assert!(
        page_text_contains(&document, 1, "They receive packages"),
        "the moved split follow must retain the complete final inner-cell paragraph; rows={:?}; frames={:?}; follows={:?}",
        summary.rows,
        document.frames,
        document.follows,
    );
    assert_page_contains(&document, 1, "End");
}

#[test]
// Sources:
// - immutable Microsoft Office fixed-format output for
//   sdt_after_section_break.docx (one blank page, then `text` on page 2);
// - ../core/sw/qa/extras/ooxmlexport/ooxmlexport20.cxx::testTdf158971;
// - ../core/sw/source/writerfilter/dmapper/PropertyMap.hxx, which records that
//   an empty sectPr carrier's spacing is metadata rather than direct layout.
fn docx_empty_continuous_section_carrier_does_not_add_a_third_page() {
    let document =
        docx_layout("sw/qa/extras/ooxmlexport/data/sdt_after_section_break.docx").unwrap();

    assert_eq!(
        document.pages.len(),
        2,
        "the empty section carrier and the page-number restart must not synthesize a second blank page: sections={:?}",
        document
            .pages
            .iter()
            .map(|page| (
                page.section_index,
                page.section_page_index,
                page.items.len()
            ))
            .collect::<Vec<_>>(),
    );
    assert!(!page_text_contains(&document, 0, "text"));
    assert_page_contains(&document, 1, "text");
}

#[test]
// Sources:
// - immutable Microsoft Office fixed-format output for floattable-nomargins.docx;
// - ../core/sw/qa/core/layout/frmtool.cxx::testFloattableNoMargins;
// - LibreOffice commit 50a1df360c90 (n#775899), which enables the DOCX
//   FLOATTABLE_NOMARGINS compatibility rule.
fn docx_floating_table_nomargins_keeps_following_paragraphs_on_one_page() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/LibreOffice/sw/qa/core/layout/data/floattable-nomargins.docx");
    let options = LayoutOptions {
        ui_language: Some("zh-CN".to_string()),
        ..LayoutOptions::default()
    };
    let package = WordprocessingDocument::new_from_file(&fixture).unwrap();
    let summary = docx::inspect_layout(&package, &options).unwrap();
    let document = docx::layout_document(&package, &options).unwrap();

    assert_eq!(
        document.pages.len(),
        1,
        "Word and Writer keep this floating table flow on one page; lines={:?}; rows={:?}; frames={:?}; follows={:?}",
        summary
            .lines
            .iter()
            .map(|line| (
                line.block_index,
                line.page_index,
                line.x_pt,
                line.y_pt,
                line.width_pt,
                line.height_pt,
            ))
            .collect::<Vec<_>>(),
        summary
            .rows
            .iter()
            .map(|row| (
                row.block_index,
                row.page_index,
                row.x_pt,
                row.y_pt,
                row.width_pt,
                row.height_pt,
            ))
            .collect::<Vec<_>>(),
        document
            .frames
            .iter()
            .map(|frame| (
                frame.block_index,
                frame.page_index,
                frame.kind.as_ref(),
                frame.bounds,
                frame.fragments.len(),
            ))
            .collect::<Vec<_>>(),
        document.follows,
    );
}

#[test]
// Sources:
// - ISO/IEC 29500-1 §20.4.2.17 (`wp:wrapTopAndBottom`);
// - immutable Microsoft Office fixed-format output for stress017.docx;
// - ../core/sw/source/writerfilter/dmapper/GraphicImport.cxx, which maps
//   wrapTopAndBottom to WrapTextMode_NONE;
// - ../core/sw/source/core/text/txtfrm.cxx::CalcBaseOfstForFly(), which
//   computes the anchor base while ignoring objects owned by the text frame.
fn docx_stress017_top_bottom_header_anchor_keeps_its_inline_content_on_page_one() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Apache-POI/test-data/integration/stress017.docx");
    let options = LayoutOptions {
        ui_language: Some("zh-CN".to_string()),
        ..LayoutOptions::default()
    };
    let package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(&package, &options).unwrap();

    assert_eq!(
        document.pages.len(),
        1,
        "the header anchor must not push the final contact line onto a second page"
    );
    assert_page_contains(&document, 0, "katarzyna.gurowska@bialystok.lasy.gov.pl");
}

#[test]
// Sources: immutable Microsoft Office fixed-format output for tdf108714.docx.
// The source is intentionally non-conformant: Word recovers w:br children
// placed directly in w:body and w:tc instead of discarding them.
fn docx_tdf108714_recovers_out_of_place_breaks_and_minimal_table() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/LibreOffice/sw/qa/extras/ooxmlimport/data/tdf108714.docx");
    let package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let options = LayoutOptions {
        ui_language: Some("zh-CN".to_string()),
        ..LayoutOptions::default()
    };
    let main = package.main_document_part().unwrap();
    let root = main.root_element(&package).unwrap();
    let table = root
        .body
        .as_ref()
        .unwrap()
        .body_choice
        .iter()
        .find_map(|choice| match choice {
            w::BodyChoice::Table(table) => Some(table.as_ref()),
            _ => None,
        })
        .expect("minimal table must survive typed import");
    let row = table
        .table_choice2
        .iter()
        .find_map(|choice| match choice {
            w::TableChoice2::TableRow(row) => Some(row.as_ref()),
            _ => None,
        })
        .expect("minimal table row must survive typed import");
    assert_eq!(
        row.table_row_choice
            .iter()
            .filter(|choice| matches!(choice, w::TableRowChoice::TableCell(_)))
            .count(),
        1,
        "minimal table cell must survive typed import"
    );
    let summary = docx::inspect_layout(&package, &options).unwrap();
    let document = docx::layout_document(&package, &options).unwrap();

    assert_eq!(document.pages.len(), 4);
    assert_eq!(
        summary.rows.len(),
        1,
        "minimal table row must be laid out; frames={:?}; follows={:?}; backward_moves={:?}; reruns={:?}",
        document
            .frames
            .iter()
            .map(|frame| (
                frame.page_index,
                frame.block_index,
                frame.kind.as_ref(),
                frame.fragments.len()
            ))
            .collect::<Vec<_>>(),
        document.follows,
        document.reflow.backward_moves,
        document.reflow.layout_reruns,
    );
    assert_page_contains(&document, 2, "Paragraph 5 in table");
}

#[test]
// Sources: ECMA-376 Part 1 §21.2.2 (DrawingML charts);
// ../core/chart2/source/view/axes/VCartesianAxis.cxx;
// immutable Microsoft Office fixed-format output for testBarChart.docx.
fn docx_clustered_column_chart_uses_semantic_axes_categories_and_legend() {
    let document = docx_layout("chart2/qa/extras/data/docx/testBarChart.docx").unwrap();
    let texts = document.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();

    for expected in ["0", "6", "Category 1", "Category 4", "Series 1", "Series 3"] {
        assert!(
            texts.contains(&expected),
            "missing semantic chart text {expected:?}; texts={texts:?}"
        );
    }
    for cached_value in ["4.3", "4.4000000000000004"] {
        assert!(
            !texts.contains(&cached_value),
            "cached series value must not become document body text: {texts:?}"
        );
    }
}

#[test]
// Sources:
// - ECMA-376 Part 1 §21.2.3.46 chart-style table (`LinearSeries` / `Subtle`)
// - ../core/oox/source/drawingml/chart/objectformatter.cxx::spLinearSeriesLines
// - immutable Microsoft Word fixed-format output for fdo74115_WallGradientFill.docx
fn docx_automatic_line_markers_share_the_themed_series_line_width() {
    let document = docx_layout("chart2/qa/extras/data/docx/fdo74115_WallGradientFill.docx")
        .expect("automatic-marker line chart layout");
    let items = page_display_items(&document, 0);
    let series_colors = [
        Color {
            r: 0x4f,
            g: 0x81,
            b: 0xbd,
            a: 0xff,
        },
        Color {
            r: 0xc0,
            g: 0x50,
            b: 0x4d,
            a: 0xff,
        },
        Color {
            r: 0x9b,
            g: 0xbb,
            b: 0x59,
            a: 0xff,
        },
    ];
    let automatic_series_lines = items
        .iter()
        .filter(|item| match item {
            DisplayItem::Path(path) => path.stroke.as_ref().is_some_and(|stroke| {
                series_colors.contains(&stroke.color) && (stroke.width.0 - 2.25).abs() < 0.001
            }),
            _ => false,
        })
        .count();
    assert!(
        automatic_series_lines >= 3,
        "all three style-2 series must use the theme's 0.75pt Subtle line at 3x"
    );

    let has_marker = |color: Color, width_pt: f32, height_pt: f32| {
        items.iter().any(|item| match item {
            DisplayItem::Path(path) => {
                path.fill == Fill::Solid(color)
                    && (path.bounds.size.width.0 - width_pt).abs() < 0.01
                    && (path.bounds.size.height.0 - height_pt).abs() < 0.01
            }
            _ => false,
        })
    };
    assert!(
        has_marker(series_colors[0], 7.7, 7.7),
        "missing 7pt automatic diamond"
    );
    assert!(
        has_marker(series_colors[1], 7.0, 7.0),
        "missing 7pt automatic square"
    );
    assert!(
        has_marker(series_colors[2], 7.7, 7.35),
        "missing 7pt automatic triangle"
    );
}

#[test]
// Sources:
// - ECMA-376 Part 1 §§17.3.2.36 and 17.9.29
// - [MS-OI29500] §2.1.97
// - ../core/sw/source/writerfilter/dmapper/DomainMapper.cxx (specVanish)
// - immutable Microsoft Office golden for tdf131728.docx
fn docx_tdf131728_advances_long_numbering_past_the_style_separator_heading() {
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf131728.docx").unwrap();
    let label = text_origins_for(&document, 0, "Article 1.")
        .into_iter()
        .next()
        .expect("Article 1 numbering label");
    let heading = text_origins_for(&document, 0, "Definitions")
        .into_iter()
        .next()
        .expect("Definitions heading");

    assert_close(label.x.0, 108.0, 0.1, "numbering label origin");
    assert_close(heading.x.0, 162.0, 0.1, "heading origin after tab");
}

#[test]
fn docx_rotated_vml_text_frame_keeps_one_upward_line_and_the_authored_inset() {
    // Office configuration 2134117fd72bece5860c8a14e8a313b2208334aecb3aa652f800ef60acc11c6d.
    // The authored -90-degree host rotates the 72pt left inset to the bottom;
    // Office places the leading text edge at y=569.230pt on the page.
    let document = docx_layout_with_options(
        "sw/qa/extras/ooxmlexport/data/TextFrameRotation.docx",
        &LayoutOptions {
            ui_language: Some("de-DE".into()),
            format_locale: Some("zh-CN".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(document.pages.len(), 1);
    let text_runs: Vec<_> = page_display_items(&document, 0)
        .into_iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text),
            _ => None,
        })
        .collect();
    let text: String = text_runs.iter().map(|run| run.text.as_ref()).collect();
    assert_eq!(text, "33333    33333    33333");
    let leading = text_runs.first().copied().expect("rotated textbox text");
    assert_close(
        leading.origin.y.0,
        569.23,
        0.05,
        "rotated left inset becomes the bottom edge",
    );
    // Display origins locate the line box; PDF adds the rotated font ascent
    // when resolving its baseline. Center this line in the rotated inset box.
    assert_close(
        leading.origin.x.0 + leading.line_height.0 / 2.0,
        172.2,
        0.01,
        "upward line box centered in the rotated inset rectangle",
    );
    for run in text_runs {
        assert_close(
            run.origin.x.0,
            leading.origin.x.0,
            0.001,
            "all runs share one upward line",
        );
        assert_close(
            run.style.rotation_degrees,
            -90.0,
            0.001,
            "independent bottom-to-top writing",
        );
    }
}

#[test]
// Sources: ECMA-376 Part 1 §§20.4.2.12, 20.4.3.5;
// ../core/sw/qa/writerfilter/ooxml/ooxml.cxx::testNestedRuns
fn docx_nested_runs_keeps_paragraph_relative_textbox_offset() {
    let document = docx_layout("sw/qa/writerfilter/ooxml/data/nested-runs.docx").unwrap();
    let origin = ooxmlsdk_layout_test::text_origins_for(&document, 0, "Test text box")
        .into_iter()
        .next()
        .expect("missing nested textbox text");

    assert_close(
        document.pages[0].setup.margins.top.0,
        56.7,
        0.05,
        "page top margin",
    );
    assert_close(origin.x.0, 88.85, 0.05, "textbox horizontal origin");
    assert_close(origin.y.0, 72.8, 0.05, "textbox vertical origin");
}

#[test]
// Sources:
// - ECMA-376 Part 1 §§20.4.2.8 and 20.4.2.10 (`wp:inline`)
// - ../core/sw/qa/extras/layout/layout3.cxx::testTdf117188
// - immutable Microsoft Office fixed output plus Word object-model and re-save geometry
//   for tdf117188.docx
fn docx_tdf117188_inline_wps_textbox_uses_the_recovered_line_grid() {
    let document = docx_layout_with_options(
        "sw/qa/extras/layout/data/tdf117188.docx",
        &LayoutOptions {
            default_document_language: Some("zh-CN".to_string()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    let origin = text_origins_for(&document, 0, "Der …")
        .into_iter()
        .next()
        .expect("inline WPS textbox text");

    assert_eq!(document.pages.len(), 1);
    assert_close(origin.x.0, 90.25, 0.05, "zero-inset textbox x origin");
    assert_close(
        origin.y.0,
        79.8,
        0.05,
        "inline textbox origin centered in the recovered line grid",
    );
}

#[test]
// Sources:
// - ECMA-376 Part 1 §17.6.5 (`w:docGrid`)
// - ../core/sw/qa/extras/ooxmlimport/ooxmlimport2.cxx::testTdf141969
// - immutable Microsoft Office fixed output and Office re-save of tdf141969
fn docx_tdf141969_recovers_the_omitted_section_grid_with_an_existing_styles_part() {
    let document = docx_layout_with_options(
        "sw/qa/extras/ooxmlimport/data/tdf141969-font_in_table_with_style.docx",
        &LayoutOptions {
            default_document_language: Some("zh-CN".to_string()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    let pitch = document.pages[0]
        .setup
        .doc_grid_line_pitch
        .expect("Office-recovered section grid");
    let origins = text_origins_for(&document, 0, "<<link:website>>");

    assert_close(pitch.0, 15.6, 0.01, "recovered 312-twip line pitch");
    assert_eq!(origins.len(), 2);
}

#[test]
// Sources: ECMA-376 Part 1 §§17.3.2.26 and 17.15.1.88.
fn docx_mailmerge_uses_word_font_slots_for_latin1_delimiters() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/mailmerge.docx",
    );
    let package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("zh-CN".to_string()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    // Adjacent Latin text may share a display run with either delimiter.
    // Keep checking both delimiters' effective Word font state and count.
    for delimiter in ['«', '»'] {
        let mut count = 0;
        for item in document.pages.iter().flat_map(|page| &page.items) {
            let DisplayItem::Text(text) = item else {
                continue;
            };
            let occurrences = text.text.matches(delimiter).count();
            if occurrences == 0 {
                continue;
            }
            count += occurrences;
            assert!(text.style.wordprocessingml_font_slots);
            assert_eq!(text.style.font_family.as_deref(), Some("Calibri"));
        }
        assert_eq!(count, 1, "mail-merge delimiter {delimiter}");
    }
}

#[test]
// Sources: ECMA-376 Part 1 §§17.15.1.18 and 17.18.7;
// [MS-OE376] Part 4 §2.15.3.31 (lineWrapLikeWord6).
fn docx_a5_uses_word_full_width_punctuation_line_fitting() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/page layout/page layout(big file)/A5.docx",
    );
    let package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("zh-CN".to_string()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    let lines = document.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.trim_start()),
            _ => None,
        })
        .collect::<Vec<_>>();

    for expected in [
        "哈巴谷爱神，但却很少有人像他这样，预备好与",
        "神进行对话，质询神的作为是否公义。当然，在属灵",
        "生活中，大部分信徒都会遭遇怀疑神、质问神的时刻。",
    ] {
        assert!(
            lines.contains(&expected),
            "missing Office line {expected:?}; lines={lines:?}"
        );
    }
}

#[test]
// Sources:
// - ECMA-376 Part 1 §§17.5.2.32–33: a cell-level structured document tag
//   surrounds one table cell, and its sdtContent contains that cell.
// - immutable Microsoft Office fixed-format output for the Open XML SDK
//   wordprocessing/SDT/Sdt/sdtContent.docx fixture.
fn docx_cell_level_sdt_keeps_the_wrapped_cell_on_its_row_baseline() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/SDT/Sdt/sdtContent.docx",
    );
    let package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(&package, &LayoutOptions::default()).unwrap();
    let page = &document.pages[0];
    let item_index_for = |expected: &str| {
        page.items
            .iter()
            .rposition(
                |item| matches!(item, DisplayItem::Text(text) if text.text.contains(expected)),
            )
            .unwrap_or_else(|| panic!("missing text item {expected:?}"))
    };
    for item_index in [
        item_index_for("This is an SdtCell."),
        item_index_for("SdtRun"),
    ] {
        let owners = document
            .frames
            .iter()
            .filter(|frame| frame.page_index == 0)
            .filter(|frame| {
                frame.lines.iter().any(|line| {
                    line.item_range.start <= item_index && item_index < line.item_range.end
                })
            })
            .map(|frame| frame.kind.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            owners,
            ["table"],
            "table-cell text owner for item {item_index}"
        );
        let cell_fragments = document
            .frames
            .iter()
            .filter(|frame| frame.page_index == 0 && frame.kind == "table")
            .flat_map(|frame| &frame.fragments)
            .filter(|fragment| {
                fragment.kind == ooxmlsdk_layout::common::FrameFragmentKind::TableCell
                    && fragment.item_range.start <= item_index
                    && item_index < fragment.item_range.end
            })
            .collect::<Vec<_>>();
        assert_eq!(
            cell_fragments.len(),
            1,
            "table-cell fragment for item {item_index}: {cell_fragments:?}"
        );
        let bounds = cell_fragments[0]
            .bounds
            .expect("table-cell fragment bounds");
        let text_y = match &page.items[item_index] {
            DisplayItem::Text(text) => text.origin.y.0,
            _ => unreachable!(),
        };
        assert!(
            bounds.origin.y.0 < text_y,
            "table-cell clip must start above its text baseline: bounds={bounds:?}, text_y={text_y}"
        );
    }
    let wrapped = text_origins_for(&document, 0, "This is an SdtCell.")
        .into_iter()
        .next()
        .expect("cell wrapped by w:sdt");
    let sibling = text_origins_for(&document, 0, "SdtRun")
        .into_iter()
        .last()
        .expect("ordinary sibling table cell");

    assert_close(
        wrapped.y.0,
        sibling.y.0,
        0.05,
        "cell-level sdt must not shift its table-cell baseline",
    );
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx::testTdf151704_thinColumnHeight
fn docx_tdf151704_keeps_nested_and_follow_table_row_heights_equal() {
    let document =
        docx_layout("sw/qa/extras/ooxmlexport/data/tdf151704_thinColumnHeight.docx").unwrap();
    let first_page_rows = row_heights(&document, 0);
    let second_page_rows = row_heights(&document, 1);
    assert!(
        first_page_rows.iter().any(|first| {
            *first > 1.0
                && second_page_rows
                    .iter()
                    .any(|second| (first - second).abs() <= 0.5)
        }),
        "LibreOffice asserts the nested table row height on page 1 equals the follow table row height on page 2; page1={first_page_rows:?}; page2={second_page_rows:?}"
    );
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx::testTdf153128
fn docx_tdf153128_keeps_first_line_height_near_one_point() {
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf153128.docx").unwrap();
    let heights = line_heights(&document, 0);
    let first = heights
        .first()
        .copied()
        .unwrap_or_else(|| panic!("missing first layout line; heights={heights:?}"));
    assert!(
        first > 0.0 && first < 30.0 / 20.0,
        "LibreOffice asserts the first line height is positive and near the 20 twip text height; first={first}; heights={heights:?}"
    );
}

#[test]
// Sources:
// - ../core/sw/qa/extras/layout/layout5.cxx::testTdf153136
// - ../core/sw/source/core/text/porlay.cxx::SwLineLayout::CalcLine
// - immutable Microsoft Word fixed output for tdf153136.docx
fn docx_tdf153136_preserves_space_character_line_height_rules() {
    let document = docx_layout("sw/qa/extras/layout/data/tdf153136.docx").unwrap();
    let page_one_lines = line_heights(&document, 0);
    let page_two_rows = row_heights(&document, 1);
    let page_three_rows = row_heights(&document, 2);
    let assert_height_classes = |actual: &[f32], large: &[bool], context: &str| {
        assert_eq!(actual.len(), large.len(), "{context}: {actual:?}");
        for (index, (&height, &is_large)) in actual.iter().zip(large).enumerate() {
            if is_large {
                assert!(
                    height > 1000.0 / 20.0,
                    "{context} item {} must exceed 1000 twips: {height}; all={actual:?}",
                    index + 1
                );
            } else {
                assert!(
                    height < 300.0 / 20.0,
                    "{context} item {} must stay below 300 twips: {height}; all={actual:?}",
                    index + 1
                );
            }
        }
    };

    assert_eq!(document.pages.len(), 3);
    assert_height_classes(
        &page_one_lines,
        &[
            false, false, true, true, true, false, false, true, false, true, true, true, true,
            true, true,
        ],
        "standalone paragraphs",
    );
    assert_height_classes(
        &page_two_rows,
        &[
            false, true, true, true, false, false, true, false, true, true, true, true, true, true,
        ],
        "table without paragraph-mark size",
    );
    assert_height_classes(
        &page_three_rows,
        &[true; 14],
        "table with explicit 48pt paragraph-mark size",
    );
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx::testTdf81100
fn docx_tdf81100_keeps_explicit_no_repeat_header_flow_across_three_pages() {
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf81100.docx").unwrap();
    assert_eq!(document.pages.len(), 3);
    assert_eq!(table_row_count_for_block(&document, 1, 1), 2);
    assert_eq!(table_row_count_for_block(&document, 2, 4), 1);
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx::testTdf58944RepeatingTableHeader
fn docx_tdf58944_repeating_header_keeps_second_page_table_content() {
    let document =
        docx_layout("sw/qa/extras/ooxmlexport/data/tdf58944-repeating-table-header.docx").unwrap();
    assert_eq!(document.pages.len(), 2);
    assert_page_contains(&document, 1, "Test1");
    assert_page_contains(&document, 1, "Test2");
}

#[test]
// Source: ../core/vcl/qa/cppunit/pdfexport/pdfexport2.cxx::testTdf152246
fn docx_content_control_rtl_retains_paragraph_direction_on_widget_text() {
    let document = docx_layout("vcl/qa/cppunit/pdfexport/data/content-control-rtl.docx").unwrap();
    let widgets = document.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) if text.form_widget_id.is_some() => Some((
                text.form_widget_id,
                text.paragraph_bidi,
                text.origin.x.0,
                text.origin.y.0,
                text.text.to_string(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(widgets.len(), 5, "widget text runs={widgets:?}");
    assert_eq!(
        widgets
            .iter()
            .map(|(_, paragraph_bidi, _, _, _)| *paragraph_bidi)
            .collect::<Vec<_>>(),
        [false, false, true, true, false],
        "widget text runs={widgets:?}"
    );
    assert!(
        widgets[0].2 < widgets[1].2 && widgets[2].2 < widgets[3].2 && widgets[2].2 > widgets[1].2,
        "LTR controls advance from the left while RTL controls occupy the right side in physical page order; widget text runs={widgets:?}"
    );
}

#[test]
// Sources:
// - ../core/sw/qa/extras/ooxmlexport/ooxmlexport2.cxx::testN592908_Frame;
// - ../core/sw/qa/extras/ooxmlexport/ooxmlexport2.cxx::testN592908_Picture.
fn docx_n592908_vml_floating_objects_keep_frame_records_page_local() {
    for fixture in [
        "sw/qa/extras/ooxmlexport/data/n592908-frame.docx",
        "sw/qa/extras/ooxmlexport/data/n592908-picture.docx",
    ] {
        let document = docx_layout(fixture).unwrap();
        for (frame_index, frame) in document.frames.iter().enumerate() {
            assert!(
                frame.page_index < document.pages.len()
                    && frame.section_index == document.pages[frame.page_index].section_index
                    && frame.section_page_index
                        == document.pages[frame.page_index].section_page_index
                    && frame.item_range.start <= frame.item_range.end
                    && frame.column_index < 64,
                "{fixture}: invalid frame {frame_index}: {frame:?}"
            );
            for (line_index, line) in frame.lines.iter().enumerate() {
                assert!(
                    line.item_range.start >= frame.item_range.start
                        && line.item_range.end <= frame.item_range.end
                        && line.item_range.start < line.item_range.end,
                    "{fixture}: line {line_index} escapes frame {frame_index}: line={line:?}; frame={frame:?}"
                );
            }
            for (fragment_index, fragment) in frame.fragments.iter().enumerate() {
                assert!(
                    fragment.item_range.start >= frame.item_range.start
                        && fragment.item_range.end <= frame.item_range.end
                        && fragment.item_range.start < fragment.item_range.end,
                    "{fixture}: fragment {fragment_index} escapes frame {frame_index}: fragment={fragment:?}; frame={frame:?}"
                );
            }
            for (influence_index, influence) in frame.influences.iter().enumerate() {
                assert!(
                    influence.count > 0 && influence.block_index == frame.block_index,
                    "{fixture}: invalid influence {influence_index} in frame {frame_index}: influence={influence:?}; frame={frame:?}"
                );
            }
        }
    }
}

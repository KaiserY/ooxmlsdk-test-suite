use std::borrow::Cow;
use std::sync::Arc;

use ooxmlsdk_fonts::{FontFaceInfo, FontId, FontRegistry, FontSource, TextScript, ThemeFontKind};
use ooxmlsdk_layout::common::{DisplayItem, LayoutFontRequest, LayoutOptions, ScriptFontFamilies};
use ooxmlsdk_layout::docx::{
    DocxBlock, DocxDocument, DocxParagraph, DocxSection, DocxTextRun, InlineItem, TextStyle,
};
use ooxmlsdk_layout::pptx::{
    PptxPresentation, PptxShape, PptxSlide, TextBody, TextParagraph, TextRun, TextRunStyle,
};
use ooxmlsdk_layout::xlsx::{
    CellAddress, CellFormat, XlsxCell, XlsxRow, XlsxSheet, XlsxStyleCatalog, XlsxTextStyle,
    XlsxWorkbook,
};

#[test]
fn layout_font_request_selects_family_by_script() {
    // Source: LibreOffice writerfilter OOXML rFonts import paths in DomainMapper.
    let request = LayoutFontRequest {
        base: Default::default(),
        families: ScriptFontFamilies {
            latin: Some(Cow::Borrowed("Latin Face")),
            high_ansi: Some(Cow::Borrowed("High ANSI Face")),
            east_asian: Some(Cow::Borrowed("East Asian Face")),
            complex_script: Some(Cow::Borrowed("Complex Script Face")),
            symbol: Some(Cow::Borrowed("Symbol Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        request.for_script(TextScript::Latin).family,
        Some(Cow::Borrowed("High ANSI Face"))
    );
    assert_eq!(
        request.for_script(TextScript::Han).family,
        Some(Cow::Borrowed("East Asian Face"))
    );
    assert_eq!(
        request.for_script(TextScript::Arabic).family,
        Some(Cow::Borrowed("Complex Script Face"))
    );
    assert_eq!(
        request.for_script(TextScript::Common).family,
        Some(Cow::Borrowed("Symbol Face"))
    );
}

#[test]
fn layout_font_request_falls_back_to_theme_slot_by_script() {
    // Source: LibreOffice OOXML theme font handling for major/minor script slots.
    let request = LayoutFontRequest {
        base: Default::default(),
        families: ScriptFontFamilies {
            latin_theme: Some(ThemeFontKind::MinorLatin),
            east_asian_theme: Some(ThemeFontKind::MajorEastAsian),
            complex_script_theme: Some(ThemeFontKind::MajorComplexScript),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        request.for_script(TextScript::Latin).theme_family,
        Some(ThemeFontKind::MinorLatin)
    );
    assert_eq!(
        request.for_script(TextScript::Han).theme_family,
        Some(ThemeFontKind::MajorEastAsian)
    );
    assert_eq!(
        request.for_script(TextScript::Arabic).theme_family,
        Some(ThemeFontKind::MajorComplexScript)
    );
}

#[test]
fn layout_font_request_uses_latin_family_when_high_ansi_is_absent() {
    // Source: LibreOffice writerfilter rFonts ascii/hAnsi fallback behavior.
    let request = LayoutFontRequest {
        base: Default::default(),
        families: ScriptFontFamilies {
            latin: Some(Cow::Borrowed("Latin Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        request.for_script(TextScript::Latin).family,
        Some(Cow::Borrowed("Latin Face"))
    );
}

#[test]
fn east_asian_and_complex_script_families_fall_back_to_latin_family() {
    // Source: LibreOffice writerfilter OOXML rFonts fallback for absent ea/cs slots.
    let request = LayoutFontRequest {
        base: Default::default(),
        families: ScriptFontFamilies {
            latin: Some(Cow::Borrowed("Latin Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        request.for_script(TextScript::Han).family,
        Some(Cow::Borrowed("Latin Face"))
    );
    assert_eq!(
        request.for_script(TextScript::Arabic).family,
        Some(Cow::Borrowed("Latin Face"))
    );
}

#[test]
fn east_asian_theme_falls_back_to_latin_theme_when_ea_slot_is_absent() {
    // Source: LibreOffice OOXML theme font fallback between script slots.
    let request = LayoutFontRequest {
        base: Default::default(),
        families: ScriptFontFamilies {
            latin_theme: Some(ThemeFontKind::MinorLatin),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        request.for_script(TextScript::Han).theme_family,
        Some(ThemeFontKind::MinorLatin)
    );
}

#[test]
fn docx_layout_splits_latin_and_east_asian_text_to_script_fonts() {
    // Source: LibreOffice sw/qa/extras/ooxmlexport CJK/list-font and rFonts coverage.
    let mut registry = FontRegistry::new();
    let mut latin = FontFaceInfo::synthetic("latin", "Latin Face");
    latin.coverage.unicode_ranges = std::iter::once(u32::from('A')..u32::from('A') + 1).collect();
    registry.register_face(FontSource::System, latin);

    let mut east_asian = FontFaceInfo::synthetic("east-asian", "East Asian Face");
    east_asian.coverage.unicode_ranges =
        std::iter::once(u32::from('中')..u32::from('中') + 1).collect();
    registry.register_face(FontSource::System, east_asian);

    let document = DocxDocument {
        sections: vec![DocxSection {
            body_blocks: vec![DocxBlock::Paragraph(DocxParagraph {
                inlines: vec![InlineItem::Text(DocxTextRun {
                    text: Cow::Borrowed("A中"),
                    style: TextStyle {
                        font_families: Box::new(ScriptFontFamilies {
                            latin: Some(Cow::Borrowed("Latin Face")),
                            east_asian: Some(Cow::Borrowed("East Asian Face")),
                            ..ScriptFontFamilies::default()
                        }),
                        ..TextStyle::default()
                    },
                })],
                ..DocxParagraph::default()
            })],
            ..DocxSection::default()
        }],
        ..DocxDocument::default()
    };

    let layout = ooxmlsdk_layout::layout_docx_model_with_fonts(
        &document,
        LayoutOptions::default(),
        &registry,
    );
    let fonts = layout.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Glyphs(run) => Some(run.shaped.font_id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        fonts,
        vec![FontId(Arc::from("latin")), FontId(Arc::from("east-asian"))]
    );
}

#[test]
fn docx_layout_splits_latin_east_asian_and_complex_script_text() {
    // Source: LibreOffice sw/qa/extras/ooxmlimport/data/tdf87533_bidi.docx coverage.
    let mut registry = FontRegistry::new();
    let mut latin = FontFaceInfo::synthetic("latin", "Latin Face");
    latin.coverage.unicode_ranges = std::iter::once(u32::from('A')..u32::from('A') + 1).collect();
    registry.register_face(FontSource::System, latin);
    let mut east_asian = FontFaceInfo::synthetic("east-asian", "East Asian Face");
    east_asian.coverage.unicode_ranges =
        std::iter::once(u32::from('中')..u32::from('中') + 1).collect();
    registry.register_face(FontSource::System, east_asian);
    let mut complex = FontFaceInfo::synthetic("complex", "Complex Face");
    complex.coverage.unicode_ranges = std::iter::once(u32::from('ش')..u32::from('ش') + 1).collect();
    registry.register_face(FontSource::System, complex);

    let document = DocxDocument {
        sections: vec![DocxSection {
            body_blocks: vec![DocxBlock::Paragraph(DocxParagraph {
                inlines: vec![InlineItem::Text(DocxTextRun {
                    text: Cow::Borrowed("A中ش"),
                    style: TextStyle {
                        font_families: Box::new(ScriptFontFamilies {
                            latin: Some(Cow::Borrowed("Latin Face")),
                            east_asian: Some(Cow::Borrowed("East Asian Face")),
                            complex_script: Some(Cow::Borrowed("Complex Face")),
                            ..ScriptFontFamilies::default()
                        }),
                        ..TextStyle::default()
                    },
                })],
                ..DocxParagraph::default()
            })],
            ..DocxSection::default()
        }],
        ..DocxDocument::default()
    };

    let layout = ooxmlsdk_layout::layout_docx_model_with_fonts(
        &document,
        LayoutOptions::default(),
        &registry,
    );
    let fonts = glyph_run_font_ids(&layout.pages[0].items);

    assert_eq!(
        fonts,
        vec![
            FontId(Arc::from("latin")),
            FontId(Arc::from("east-asian")),
            FontId(Arc::from("complex")),
        ]
    );
}

#[test]
fn docx_layout_keeps_common_punctuation_with_current_script_run() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx layout-run normalization coverage.
    let mut registry = FontRegistry::new();
    let mut latin = FontFaceInfo::synthetic("latin", "Latin Face");
    latin.coverage.unicode_ranges = vec![
        u32::from('A')..u32::from('A') + 1,
        u32::from(',')..u32::from(',') + 1,
    ];
    registry.register_face(FontSource::System, latin);

    let document = DocxDocument {
        sections: vec![DocxSection {
            body_blocks: vec![DocxBlock::Paragraph(DocxParagraph {
                inlines: vec![InlineItem::Text(DocxTextRun {
                    text: Cow::Borrowed("A,"),
                    style: TextStyle {
                        font_families: Box::new(ScriptFontFamilies {
                            latin: Some(Cow::Borrowed("Latin Face")),
                            symbol: Some(Cow::Borrowed("Symbol Face")),
                            ..ScriptFontFamilies::default()
                        }),
                        ..TextStyle::default()
                    },
                })],
                ..DocxParagraph::default()
            })],
            ..DocxSection::default()
        }],
        ..DocxDocument::default()
    };

    let layout = ooxmlsdk_layout::layout_docx_model_with_fonts(
        &document,
        LayoutOptions::default(),
        &registry,
    );
    let fonts = glyph_run_font_ids(&layout.pages[0].items);

    assert_eq!(fonts, vec![FontId(Arc::from("latin"))]);
}

#[test]
fn docx_layout_uses_symbol_family_for_leading_common_text() {
    // Source: LibreOffice OOXML symbol font import for bullets and symbol runs.
    let mut registry = FontRegistry::new();
    let mut symbol = FontFaceInfo::synthetic("symbol", "Symbol Face");
    symbol.coverage.unicode_ranges = std::iter::once(u32::from('•')..u32::from('•') + 1).collect();
    registry.register_face(FontSource::System, symbol);

    let document = DocxDocument {
        sections: vec![DocxSection {
            body_blocks: vec![DocxBlock::Paragraph(DocxParagraph {
                inlines: vec![InlineItem::Text(DocxTextRun {
                    text: Cow::Borrowed("•"),
                    style: TextStyle {
                        font_families: Box::new(ScriptFontFamilies {
                            symbol: Some(Cow::Borrowed("Symbol Face")),
                            latin: Some(Cow::Borrowed("Latin Face")),
                            ..ScriptFontFamilies::default()
                        }),
                        ..TextStyle::default()
                    },
                })],
                ..DocxParagraph::default()
            })],
            ..DocxSection::default()
        }],
        ..DocxDocument::default()
    };

    let layout = ooxmlsdk_layout::layout_docx_model_with_fonts(
        &document,
        LayoutOptions::default(),
        &registry,
    );

    assert_eq!(
        glyph_run_font_ids(&layout.pages[0].items),
        vec![FontId(Arc::from("symbol"))]
    );
}

#[test]
fn docx_layout_keeps_han_punctuation_with_han_script_run() {
    // Source: LibreOffice CJK text layout keeps common punctuation with surrounding CJK run.
    let mut registry = FontRegistry::new();
    let mut east_asian = FontFaceInfo::synthetic("east-asian", "East Asian Face");
    east_asian.coverage.unicode_ranges = vec![
        u32::from('中')..u32::from('中') + 1,
        u32::from('。')..u32::from('。') + 1,
    ];
    registry.register_face(FontSource::System, east_asian);

    let document = DocxDocument {
        sections: vec![DocxSection {
            body_blocks: vec![DocxBlock::Paragraph(DocxParagraph {
                inlines: vec![InlineItem::Text(DocxTextRun {
                    text: Cow::Borrowed("中。"),
                    style: TextStyle {
                        font_families: Box::new(ScriptFontFamilies {
                            east_asian: Some(Cow::Borrowed("East Asian Face")),
                            symbol: Some(Cow::Borrowed("Symbol Face")),
                            ..ScriptFontFamilies::default()
                        }),
                        ..TextStyle::default()
                    },
                })],
                ..DocxParagraph::default()
            })],
            ..DocxSection::default()
        }],
        ..DocxDocument::default()
    };

    let layout = ooxmlsdk_layout::layout_docx_model_with_fonts(
        &document,
        LayoutOptions::default(),
        &registry,
    );

    assert_eq!(
        glyph_run_font_ids(&layout.pages[0].items),
        vec![FontId(Arc::from("east-asian"))]
    );
}

#[test]
fn xlsx_layout_uses_cell_font_style_for_glyph_runs() {
    // Source: LibreOffice sc/qa/unit font style import/export coverage.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("cell-font", "Cell Font");
    face.coverage.unicode_ranges = std::iter::once(u32::from('A')..u32::from('A') + 1).collect();
    registry.register_face(FontSource::System, face);

    let workbook = XlsxWorkbook {
        styles: XlsxStyleCatalog {
            cell_formats: vec![CellFormat {
                text_style: XlsxTextStyle {
                    font_family: Some(Cow::Borrowed("Cell Font")),
                    ..XlsxTextStyle::default()
                },
                ..CellFormat::default()
            }],
            ..XlsxStyleCatalog::default()
        },
        sheets: vec![XlsxSheet {
            name: Cow::Borrowed("Sheet1"),
            rows: vec![XlsxRow {
                cells: vec![XlsxCell {
                    address: Some(CellAddress { column: 0, row: 0 }),
                    display_text: Cow::Borrowed("A"),
                    style_index: Some(0),
                    ..XlsxCell::default()
                }],
                ..XlsxRow::default()
            }],
            ..XlsxSheet::default()
        }],
        ..XlsxWorkbook::default()
    };

    let layout = ooxmlsdk_layout::layout_xlsx_model_with_fonts(
        &workbook,
        LayoutOptions::default(),
        &registry,
    );
    let fonts = glyph_run_font_ids(&layout.pages[0].items);

    assert_eq!(fonts, vec![FontId(Arc::from("cell-font"))]);
}

#[test]
fn xlsx_text_style_maps_size_bold_and_italic_to_font_request() {
    // Source: LibreOffice sc/qa/unit font size/style import coverage.
    let style = XlsxTextStyle {
        font_family: Some(Cow::Borrowed("Cell Font")),
        size: Some(ooxmlsdk_layout::common::Pt(14.0)),
        bold: true,
        italic: true,
        ..XlsxTextStyle::default()
    };
    let request = style.layout_font_request().base;

    assert_eq!(request.family, Some(Cow::Borrowed("Cell Font")));
    assert_eq!(request.size_pt.0, 14.0);
    assert!(request.bold);
    assert!(request.italic);
}

#[test]
fn pptx_layout_uses_run_font_for_glyph_runs() {
    // Source: LibreOffice sd/oox DrawingML text font import coverage.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("pptx-font", "Pptx Font");
    face.coverage.unicode_ranges = std::iter::once(u32::from('A')..u32::from('A') + 1).collect();
    registry.register_face(FontSource::System, face);

    let presentation = PptxPresentation {
        slides: vec![PptxSlide {
            shapes: vec![PptxShape {
                text_body: Some(TextBody {
                    paragraphs: vec![TextParagraph {
                        runs: vec![TextRun {
                            text: Cow::Borrowed("A"),
                            style: TextRunStyle {
                                font: ooxmlsdk_fonts::FontRequest {
                                    family: Some(Cow::Borrowed("Pptx Font")),
                                    ..ooxmlsdk_fonts::FontRequest::default()
                                },
                                ..TextRunStyle::default()
                            },
                        }],
                        ..TextParagraph::default()
                    }],
                    ..TextBody::default()
                }),
                ..PptxShape::default()
            }],
            ..PptxSlide::default()
        }],
        ..PptxPresentation::default()
    };

    let layout = ooxmlsdk_layout::layout_pptx_model_with_fonts(
        &presentation,
        LayoutOptions::default(),
        &registry,
    );

    assert_eq!(
        glyph_run_font_ids(&layout.pages[0].items),
        vec![FontId(Arc::from("pptx-font"))]
    );
}

fn glyph_run_font_ids(items: &[DisplayItem<'_>]) -> Vec<FontId> {
    items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Glyphs(run) => Some(run.shaped.font_id.clone()),
            _ => None,
        })
        .collect()
}

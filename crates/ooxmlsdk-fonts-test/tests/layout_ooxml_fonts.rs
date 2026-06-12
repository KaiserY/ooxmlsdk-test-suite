use std::borrow::Cow;
use std::sync::Arc;

use ooxmlsdk_fonts::{FontFaceInfo, FontId, FontRegistry, FontSource, TextScript, ThemeFontKind};
use ooxmlsdk_layout::common::{DisplayItem, LayoutFontRequest, LayoutOptions, ScriptFontFamilies};
use ooxmlsdk_layout::docx::{
    DocxBlock, DocxDocument, DocxParagraph, DocxSection, DocxTextRun, InlineItem, TextStyle,
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

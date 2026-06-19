use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use ooxmlsdk_fonts::{
    FontFaceInfo, FontId, FontRegistry, FontRequest, FontSource, ShapeOptions, TextDirection,
    TextScript, ThemeFontKind, script_direction_runs,
};
use ooxmlsdk_layout::common::{LayoutFontRequest, Pt, ScriptFontFamilies};
use ooxmlsdk_layout::xlsx::XlsxTextStyle;

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
fn layout_font_request_shapes_latin_and_east_asian_text_with_script_fonts() {
    // Source: LibreOffice sw/qa/extras/ooxmlexport CJK/list-font and rFonts coverage.
    let mut registry = FontRegistry::new();
    let mut latin = FontFaceInfo::synthetic("latin", "Latin Face");
    latin.coverage.unicode_ranges = coverage_ranges(['A']);
    registry.register_face(FontSource::System, latin);

    let mut east_asian = FontFaceInfo::synthetic("east-asian", "East Asian Face");
    east_asian.coverage.unicode_ranges = coverage_ranges(['中']);
    registry.register_face(FontSource::System, east_asian);

    let request = LayoutFontRequest {
        base: FontRequest::default(),
        families: ScriptFontFamilies {
            latin: Some(Cow::Borrowed("Latin Face")),
            east_asian: Some(Cow::Borrowed("East Asian Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        shape_layout_font_request(&registry, &request, "A中"),
        vec![FontId(Arc::from("latin")), FontId(Arc::from("east-asian"))]
    );
}

#[test]
fn layout_font_request_shapes_latin_east_asian_and_complex_script_text() {
    // Source: LibreOffice sw/qa/extras/ooxmlimport/data/tdf87533_bidi.docx coverage.
    let mut registry = FontRegistry::new();
    let mut latin = FontFaceInfo::synthetic("latin", "Latin Face");
    latin.coverage.unicode_ranges = coverage_ranges(['A']);
    registry.register_face(FontSource::System, latin);
    let mut east_asian = FontFaceInfo::synthetic("east-asian", "East Asian Face");
    east_asian.coverage.unicode_ranges = coverage_ranges(['中']);
    registry.register_face(FontSource::System, east_asian);
    let mut complex = FontFaceInfo::synthetic("complex", "Complex Face");
    complex.coverage.unicode_ranges = coverage_ranges(['ش']);
    registry.register_face(FontSource::System, complex);

    let request = LayoutFontRequest {
        base: FontRequest::default(),
        families: ScriptFontFamilies {
            latin: Some(Cow::Borrowed("Latin Face")),
            east_asian: Some(Cow::Borrowed("East Asian Face")),
            complex_script: Some(Cow::Borrowed("Complex Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        shape_layout_font_request(&registry, &request, "A中ش"),
        vec![
            FontId(Arc::from("latin")),
            FontId(Arc::from("east-asian")),
            FontId(Arc::from("complex")),
        ]
    );
}

#[test]
fn layout_font_request_keeps_common_punctuation_with_current_script_run() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx layout-run normalization coverage.
    let mut registry = FontRegistry::new();
    let mut latin = FontFaceInfo::synthetic("latin", "Latin Face");
    latin.coverage.unicode_ranges = coverage_ranges(['A', ',']);
    registry.register_face(FontSource::System, latin);

    let request = LayoutFontRequest {
        base: FontRequest::default(),
        families: ScriptFontFamilies {
            latin: Some(Cow::Borrowed("Latin Face")),
            symbol: Some(Cow::Borrowed("Symbol Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        shape_layout_font_request(&registry, &request, "A,"),
        vec![FontId(Arc::from("latin"))]
    );
}

#[test]
fn layout_font_request_uses_symbol_family_for_leading_common_text() {
    // Source: LibreOffice OOXML symbol font import for bullets and symbol runs.
    let mut registry = FontRegistry::new();
    let mut symbol = FontFaceInfo::synthetic("symbol", "Symbol Face");
    symbol.coverage.unicode_ranges = coverage_ranges(['•']);
    registry.register_face(FontSource::System, symbol);

    let request = LayoutFontRequest {
        base: FontRequest::default(),
        families: ScriptFontFamilies {
            symbol: Some(Cow::Borrowed("Symbol Face")),
            latin: Some(Cow::Borrowed("Latin Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        shape_layout_font_request_for_script(&registry, &request, TextScript::Common, "•"),
        vec![FontId(Arc::from("symbol"))]
    );
}

#[test]
fn layout_font_request_keeps_han_punctuation_with_han_script_run() {
    // Source: LibreOffice CJK text layout keeps common punctuation with surrounding CJK run.
    let mut registry = FontRegistry::new();
    let mut east_asian = FontFaceInfo::synthetic("east-asian", "East Asian Face");
    east_asian.coverage.unicode_ranges = coverage_ranges(['中', '。']);
    registry.register_face(FontSource::System, east_asian);

    let request = LayoutFontRequest {
        base: FontRequest::default(),
        families: ScriptFontFamilies {
            east_asian: Some(Cow::Borrowed("East Asian Face")),
            symbol: Some(Cow::Borrowed("Symbol Face")),
            ..ScriptFontFamilies::default()
        },
        ..LayoutFontRequest::default()
    };

    assert_eq!(
        shape_layout_font_request(&registry, &request, "中。"),
        vec![FontId(Arc::from("east-asian"))]
    );
}

#[test]
fn xlsx_layout_font_request_uses_cell_font_style_for_glyph_runs() {
    // Source: LibreOffice sc/qa/unit font style import/export coverage.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("cell-font", "Cell Font");
    face.coverage.unicode_ranges = coverage_ranges(['A']);
    registry.register_face(FontSource::System, face);

    let style = XlsxTextStyle {
        font_family: Some(Cow::Borrowed("Cell Font")),
        ..XlsxTextStyle::default()
    };

    assert_eq!(
        shape_layout_font_request(&registry, &style.layout_font_request(), "A"),
        vec![FontId(Arc::from("cell-font"))]
    );
}

#[test]
fn xlsx_text_style_maps_size_bold_and_italic_to_font_request() {
    // Source: LibreOffice sc/qa/unit font size/style import coverage.
    let style = XlsxTextStyle {
        font_family: Some(Cow::Borrowed("Cell Font")),
        size: Some(Pt(14.0)),
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

fn shape_layout_font_request(
    registry: &FontRegistry<'_>,
    request: &LayoutFontRequest<'_>,
    text: &str,
) -> Vec<FontId> {
    let mut font_ids = Vec::new();
    for script_run in
        script_direction_runs(text.to_string(), request.base.size_pt, request.small_caps)
    {
        let mut font_request = request.for_script(script_run.script);
        font_request.size_pt = script_run.size_pt;
        let mut options = ShapeOptions::from_request(&font_request, script_run.direction);
        options.character_spacing_pt = request.character_spacing.0;
        options.small_caps = false;
        options.scan_registered_fallbacks = false;
        let runs = registry
            .shape_text_runs_with_options(&font_request, script_run.text.as_ref(), &options)
            .expect("layout font request should shape text");
        font_ids.extend(runs.into_iter().map(|run| run.font_id));
    }
    font_ids
}

fn shape_layout_font_request_for_script(
    registry: &FontRegistry<'_>,
    request: &LayoutFontRequest<'_>,
    script: TextScript,
    text: &str,
) -> Vec<FontId> {
    let font_request = request.for_script(script);
    registry
        .shape_text_runs(&font_request, text, TextDirection::LeftToRight)
        .expect("layout font request should shape script text")
        .into_iter()
        .map(|run| run.font_id)
        .collect()
}

fn coverage_ranges<const N: usize>(chars: [char; N]) -> Vec<Range<u32>> {
    chars
        .into_iter()
        .map(|ch| u32::from(ch)..u32::from(ch) + 1)
        .collect()
}

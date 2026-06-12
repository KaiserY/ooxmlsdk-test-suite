use std::borrow::Cow;
use std::sync::Arc;

use ooxmlsdk_fonts::{
    FallbackRun, FontCoverage, FontFaceInfo, FontFallbackChain, FontFamilyAlias, FontId,
    FontRegistry, FontRequest, FontSize, FontSource, FontStretch, FontSubstitutionReason,
    FontSubstitutionRule, FontUsageCollector, FontWeight, ShapedGlyph, ShapedRun,
    ShapingDiagnostics, TextDirection, TextScript, ThemeFontKind, ThemeFontMap,
};

#[test]
fn normalized_family_aliases_resolve_before_matching() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testFontFamilyAliases.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("liberation-serif", "Liberation Serif"),
    );
    registry.book.family_aliases.push(FontFamilyAlias {
        from: Cow::Borrowed("Times-New_Roman"),
        to: Cow::Borrowed("Liberation Serif"),
    });

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Times New Roman")),
            ..FontRequest::default()
        })
        .expect("alias should resolve to registered substitute family");

    assert_eq!(resolved.font_id, FontId(Arc::from("liberation-serif")));
    assert_eq!(
        resolved.substitution.expect("alias substitution").reason,
        FontSubstitutionReason::Alias
    );
}

#[test]
fn face_ranking_prefers_closest_weight_and_stretch() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontface.cxx ordering and match-status tests.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("regular", "Example"),
    );

    let mut bold = FontFaceInfo::synthetic("bold", "Example");
    bold.weight = FontWeight::Bold;
    registry.register_face(FontSource::System, bold);

    let mut condensed = FontFaceInfo::synthetic("condensed", "Example");
    condensed.stretch = FontStretch::Condensed;
    registry.register_face(FontSource::System, condensed);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Example")),
            bold: true,
            stretch: Some(FontStretch::Normal),
            ..FontRequest::default()
        })
        .expect("family should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("bold")));
    assert!(!resolved.synthetic_bold);
    assert_eq!(resolved.match_diagnostics.candidates.len(), 3);
}

#[test]
fn explicit_substitution_records_substituted_family() {
    // Source: LibreOffice vcl/source/font/DirectFontSubstitution.cxx behavior.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("replacement", "Replacement"),
    );
    registry.book.substitutions.push(FontSubstitutionRule {
        requested_family: Cow::Borrowed("Missing Family"),
        substitute_family: Cow::Borrowed("Replacement"),
        reason: FontSubstitutionReason::MissingFamily,
    });

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Missing Family")),
            ..FontRequest::default()
        })
        .expect("substitution should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("replacement")));
    assert_eq!(resolved.resolved_family, Cow::Borrowed("Replacement"));
    assert_eq!(
        resolved
            .substitution
            .expect("substitution diagnostics")
            .reason,
        FontSubstitutionReason::MissingFamily
    );
}

#[test]
fn fallback_chain_splits_missing_glyph_runs_by_coverage() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx::testImplLayoutRuns_PrepareFallbackRuns_LTR.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);

    let mut fallback = FontFaceInfo::synthetic("fallback", "Fallback");
    fallback.coverage = coverage_for_chars(['中']);
    registry.register_face(FontSource::System, fallback);
    registry.book.fallback_chains.push(FontFallbackChain {
        script: Some(TextScript::Latin),
        language: None,
        families: vec![Cow::Borrowed("Fallback")],
    });

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                script: Some(TextScript::Latin),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "A中A",
            TextDirection::LeftToRight,
        )
        .expect("fallback shaping should succeed");

    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].font_id, FontId(Arc::from("primary")));
    assert_eq!(runs[1].font_id, FontId(Arc::from("fallback")));
    assert_eq!(runs[2].font_id, FontId(Arc::from("primary")));
    assert_eq!(
        runs[1].diagnostics.fallback_runs,
        vec![FallbackRun {
            text_range: 1..4,
            font_id: Some(FontId(Arc::from("fallback"))),
            fallback_level: 1,
            reason: FontSubstitutionReason::MissingGlyph,
            family: Some(Cow::Borrowed("Fallback")),
        }]
    );
}

#[test]
fn registered_memory_face_data_is_available_for_renderers() {
    // Source: LibreOffice vcl/source/pdf/pdfwriter*.cxx uses the selected face for output.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::Memory {
            id: Cow::Borrowed("memory-face"),
            data: Cow::Borrowed(&[0, 1, 2, 3]),
        },
        FontFaceInfo::synthetic("memory-face", "Memory Face"),
    );

    let data = registry
        .font_face_data(&FontId(Arc::from("memory-face")))
        .expect("face data should be exposed by stable font id");

    assert_eq!(data.face_index, 0);
    assert_eq!(data.data.as_deref(), Some([0, 1, 2, 3].as_slice()));
    assert_eq!(data.family_names, vec![Cow::Borrowed("Memory Face")]);
}

#[test]
fn font_usage_collector_records_glyphs_and_unicode_ranges() {
    // Source: LibreOffice PDF font subsetting paths in vcl/source/pdf/.
    let run = ShapedRun {
        font_id: FontId(Arc::from("subset-face")),
        text: Cow::Borrowed("AB"),
        text_range: 0..2,
        glyphs: Cow::Owned(vec![
            ShapedGlyph {
                glyph_id: 41,
                text_range: 0..1,
                source_char: Some('A'),
                ..ShapedGlyph::default()
            },
            ShapedGlyph {
                glyph_id: 42,
                text_range: 1..2,
                source_char: Some('B'),
                ..ShapedGlyph::default()
            },
        ]),
        advance_pt: 24.0,
        direction: TextDirection::LeftToRight,
        script: Some(TextScript::Latin),
        language: None,
        safe_breaks: Vec::new(),
        approximate: false,
        decorations: Vec::new(),
        diagnostics: ShapingDiagnostics::default(),
    };
    let mut collector = FontUsageCollector::default();

    collector.record_run(&run);

    assert_eq!(collector.usages.len(), 1);
    assert!(collector.usages[0].needs_embedding);
    assert!(collector.usages[0].glyph_ids.contains(&41));
    assert!(collector.usages[0].glyph_ids.contains(&42));
    assert_eq!(collector.usages[0].unicode_ranges, vec![65..67]);
}

#[test]
fn theme_font_map_resolves_each_ooxml_theme_slot() {
    // Source: LibreOffice oox theme-font import tests, e.g. oox/qa/unit/drawingml.cxx.
    let map = ThemeFontMap {
        major_latin: Some(Cow::Borrowed("Major Latin")),
        minor_latin: Some(Cow::Borrowed("Minor Latin")),
        major_east_asian: Some(Cow::Borrowed("Major EA")),
        minor_complex_script: Some(Cow::Borrowed("Minor CS")),
        ..ThemeFontMap::default()
    };

    assert_eq!(
        map.resolve(ThemeFontKind::MajorLatin),
        Some(Cow::Borrowed("Major Latin"))
    );
    assert_eq!(
        map.resolve(ThemeFontKind::MinorLatin),
        Some(Cow::Borrowed("Minor Latin"))
    );
    assert_eq!(
        map.resolve(ThemeFontKind::MajorEastAsian),
        Some(Cow::Borrowed("Major EA"))
    );
    assert_eq!(
        map.resolve(ThemeFontKind::MinorComplexScript),
        Some(Cow::Borrowed("Minor CS"))
    );
}

fn coverage_for_chars<const N: usize>(chars: [char; N]) -> FontCoverage {
    FontCoverage {
        unicode_ranges: chars
            .into_iter()
            .map(|ch| u32::from(ch)..u32::from(ch) + 1)
            .collect(),
        scripts: Default::default(),
    }
}

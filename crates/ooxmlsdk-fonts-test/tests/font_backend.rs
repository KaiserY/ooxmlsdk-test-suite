use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use ooxmlsdk_fonts::{
    DecorationMetrics, FallbackRun, FontCharset, FontCoverage, FontEmbeddingPolicy, FontFaceInfo,
    FontFallbackChain, FontFamilyAlias, FontFamilyClass, FontFlags, FontId, FontMetrics, FontPitch,
    FontRegistry, FontRequest, FontScriptRun, FontSize, FontSource, FontStretch, FontSubsetPolicy,
    FontSubstitutionReason, FontSubstitutionRule, FontUsageCollector, FontWeight, ScriptMetrics,
    ScriptScanOptions, ShapeOptions, ShapedGlyph, ShapedRun, ShapingDiagnostics, TextDirection,
    TextScript, ThemeFontKind, ThemeFontMap, VariationValue, VerticalMetrics,
    format_font_variations, parse_font_feature_settings, parse_font_variations,
    script_direction_runs, script_direction_runs_with_options, trim_font_name_features,
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
    assert_eq!(resolved.resolved_family, Cow::Borrowed("Liberation Serif"));
}

#[test]
fn semicolon_family_tokens_match_registered_family_names() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindFontFamilyByTokenNames.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("token-face", "Second Token"),
    );

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Missing; Second Token; Other")),
            ..FontRequest::default()
        })
        .expect("semicolon token should match registered family");

    assert_eq!(resolved.font_id, FontId(Arc::from("token-face")));
}

#[test]
fn direct_family_lookup_ignores_spacing_hyphen_and_underscore() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx normalized search-name tests.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("normalized", "Liberation Serif"),
    );

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Liberation-Serif")),
            ..FontRequest::default()
        })
        .expect("normalized family name should match");

    assert_eq!(resolved.font_id, FontId(Arc::from("normalized")));
}

#[test]
fn missing_family_request_returns_no_match() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldNotFindFontFamily.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("test-family", "Test Font Family Name"),
    );

    let error = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("blah")),
            ..FontRequest::default()
        })
        .expect_err("unregistered family should not match an arbitrary font");

    assert!(matches!(error, ooxmlsdk_fonts::FontError::NoMatch));
}

#[test]
fn alternate_family_names_participate_in_lookup() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindFamilyName.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("family-name", "Primary Name");
    face.family_names.push(Cow::Borrowed("Test font name"));
    registry.register_face(FontSource::System, face);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Test font name")),
            ..FontRequest::default()
        })
        .expect("alternate family name should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("family-name")));
}

#[test]
fn fixed_pitch_request_prefers_fixed_pitch_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontface.cxx pitch match ranking.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("variable", "Mono Candidate"),
    );
    let mut fixed = FontFaceInfo::synthetic("fixed", "Mono Candidate");
    fixed.pitch = FontPitch::Fixed;
    registry.register_face(FontSource::System, fixed);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Mono Candidate")),
            pitch: Some(FontPitch::Fixed),
            ..FontRequest::default()
        })
        .expect("fixed pitch request should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("fixed")));
}

#[test]
fn generic_family_class_request_matches_explicit_face_class() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx generic family tests.
    let mut registry = FontRegistry::new();
    let mut serif = FontFaceInfo::synthetic("serif", "Serif Candidate");
    serif.family_class = Some(FontFamilyClass::Serif);
    registry.register_face(FontSource::System, serif);
    let mut sans = FontFaceInfo::synthetic("sans", "Sans Candidate");
    sans.family_class = Some(FontFamilyClass::SansSerif);
    registry.register_face(FontSource::System, sans);

    let resolved = registry
        .resolve(&FontRequest {
            family_class: Some(FontFamilyClass::SansSerif),
            ..FontRequest::default()
        })
        .expect("sans-serif class should resolve to matching face");

    assert_eq!(resolved.font_id, FontId(Arc::from("sans")));
}

#[test]
fn generic_family_class_request_rejects_mismatched_class() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx negative generic family tests.
    let mut registry = FontRegistry::new();
    let mut serif = FontFaceInfo::synthetic("serif", "Serif Candidate");
    serif.family_class = Some(FontFamilyClass::Serif);
    registry.register_face(FontSource::System, serif);

    let error = registry
        .resolve(&FontRequest {
            family_class: Some(FontFamilyClass::SansSerif),
            ..FontRequest::default()
        })
        .expect_err("sans-serif request should reject a serif-only registry");

    assert!(matches!(error, ooxmlsdk_fonts::FontError::NoMatch));
}

#[test]
fn fixed_family_class_matches_fixed_pitch_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldMatchFixedFamily.
    let mut registry = FontRegistry::new();
    let mut fixed = FontFaceInfo::synthetic("fixed", "Matching family name");
    fixed.pitch = FontPitch::Fixed;
    registry.register_face(FontSource::System, fixed);

    let resolved = registry
        .resolve(&FontRequest {
            family_class: Some(FontFamilyClass::Fixed),
            ..FontRequest::default()
        })
        .expect("fixed class should match a fixed-pitch face");

    assert_eq!(resolved.font_id, FontId(Arc::from("fixed")));
}

#[test]
fn decorative_family_class_ignores_weight_and_slant_mismatch() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldMatchDecorativeFamily.
    let mut registry = FontRegistry::new();
    let mut decorative = FontFaceInfo::synthetic("decorative", "Decorative");
    decorative.family_class = Some(FontFamilyClass::Decorative);
    decorative.weight = FontWeight::Medium;
    registry.register_face(FontSource::System, decorative);

    let resolved = registry
        .resolve(&FontRequest {
            family_class: Some(FontFamilyClass::Decorative),
            italic: true,
            weight: Some(FontWeight::Normal),
            ..FontRequest::default()
        })
        .expect("decorative class should resolve despite style distance");

    assert_eq!(resolved.font_id, FontId(Arc::from("decorative")));
}

#[test]
fn generic_family_class_can_be_inferred_from_family_name() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx name-derived generic family tests.
    let cases = [
        ("script", FontFamilyClass::BrushScript),
        ("testtitling", FontFamilyClass::Titling),
        ("testcaps", FontFamilyClass::Capitals),
        ("testoldstyle", FontFamilyClass::OldStyle),
        ("testschoolbook", FontFamilyClass::Schoolbook),
    ];

    for (family, class) in cases {
        let mut registry = FontRegistry::new();
        registry.register_face(FontSource::System, FontFaceInfo::synthetic(family, family));

        let resolved = registry
            .resolve(&FontRequest {
                family_class: Some(class),
                ..FontRequest::default()
            })
            .expect("name-derived family class should resolve");

        assert_eq!(resolved.font_id, FontId(Arc::from(family)));
    }
}

#[test]
fn other_style_family_class_does_not_match_unrelated_monotype_name() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldNotFindOtherStyleFamily.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("monotype", "monotype"),
    );

    let error = registry
        .resolve(&FontRequest {
            family_class: Some(FontFamilyClass::OldStyle),
            ..FontRequest::default()
        })
        .expect_err("oldstyle request should not match unrelated monotype name");

    assert!(matches!(error, ooxmlsdk_fonts::FontError::NoMatch));
}

#[test]
fn charset_request_prefers_registered_charset_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx CJK family matching.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("ansi", "CJK Candidate"),
    );
    let mut cjk = FontFaceInfo::synthetic("shift-jis", "CJK Candidate");
    cjk.face_index = 1;
    registry.register_face(FontSource::System, cjk);
    registry.faces[1].charset = Some(FontCharset::ShiftJis);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("CJK Candidate")),
            charset: Some(FontCharset::ShiftJis),
            ..FontRequest::default()
        })
        .expect("charset-specific request should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("shift-jis")));
}

#[test]
fn symbol_charset_request_prefers_symbolic_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx symbol-family tests.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("text-symbol", "Symbolic Family"),
    );
    let mut symbolic = FontFaceInfo::synthetic("real-symbol", "Symbolic Family");
    symbolic.flags = FontFlags {
        symbolic: true,
        ..FontFlags::default()
    };
    registry.register_face(FontSource::System, symbolic);

    let resolved = registry
        .resolve_with_diagnostics(&FontRequest {
            family: Some(Cow::Borrowed("Symbolic Family")),
            charset: Some(FontCharset::Symbol),
            ..FontRequest::default()
        })
        .expect("symbol charset should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("real-symbol")));
    assert_eq!(
        resolved
            .match_diagnostics
            .candidates
            .iter()
            .find(|candidate| candidate.font_id == FontId(Arc::from("text-symbol")))
            .expect("text face candidate")
            .reason,
        Some(ooxmlsdk_fonts::FontMatchReason::Charset)
    );
}

#[test]
fn symbol_charset_without_family_rejects_normal_text_faces() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldNotFindSymbolFamily.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("normal", "Normal Text"),
    );

    let error = registry
        .resolve(&FontRequest {
            charset: Some(FontCharset::Symbol),
            ..FontRequest::default()
        })
        .expect_err("symbol request should not match a non-symbol face");

    assert!(matches!(error, ooxmlsdk_fonts::FontError::NoMatch));
}

#[test]
fn symbol_charset_without_family_selects_symbolic_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindSymbolFamilyByMatchType.
    let mut registry = FontRegistry::new();
    let mut symbolic = FontFaceInfo::synthetic("symbols", "symbols");
    symbolic.flags.symbolic = true;
    registry.register_face(FontSource::System, symbolic);

    let resolved = registry
        .resolve(&FontRequest {
            charset: Some(FontCharset::Symbol),
            ..FontRequest::default()
        })
        .expect("symbol charset should find a symbolic face");

    assert_eq!(resolved.font_id, FontId(Arc::from("symbols")));
}

#[test]
fn symbol_charset_without_family_prefers_opensymbol_by_stable_family_order() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindOpensymbolFamilyWithMultipleSymbolFamilies.
    let mut registry = FontRegistry::new();
    let mut wingdings = FontFaceInfo::synthetic("wingdings", "wingdings");
    wingdings.flags.symbolic = true;
    registry.register_face(FontSource::System, wingdings);
    let mut open_symbol = FontFaceInfo::synthetic("opensymbol", "opensymbol");
    open_symbol.flags.symbolic = true;
    registry.register_face(FontSource::System, open_symbol);

    let resolved = registry
        .resolve(&FontRequest {
            charset: Some(FontCharset::Symbol),
            ..FontRequest::default()
        })
        .expect("symbol charset should resolve to a symbolic family");

    assert_eq!(resolved.font_id, FontId(Arc::from("opensymbol")));
}

#[test]
fn normalized_alias_can_be_requested_without_spaces() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontcollection.cxx::testFontFamilyAliases.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("testfont", "Test Font"),
    );
    registry.book.family_aliases.push(FontFamilyAlias {
        from: Cow::Borrowed("Some Alias"),
        to: Cow::Borrowed("Test Font"),
    });

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("somealias")),
            ..FontRequest::default()
        })
        .expect("normalized alias should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("testfont")));
    assert_eq!(resolved.resolved_family, Cow::Borrowed("Test Font"));
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
        .resolve_with_diagnostics(&FontRequest {
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
fn stretch_request_prefers_closest_width_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontface.cxx width CompareIgnoreSize tests.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("normal", "Width Candidate"),
    );
    let mut condensed = FontFaceInfo::synthetic("condensed", "Width Candidate");
    condensed.stretch = FontStretch::Condensed;
    registry.register_face(FontSource::System, condensed);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Width Candidate")),
            stretch: Some(FontStretch::Condensed),
            ..FontRequest::default()
        })
        .expect("condensed request should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("condensed")));
}

#[test]
fn weight_request_prefers_closest_weight_face() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontface.cxx weight CompareIgnoreSize tests.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("regular", "Weight Candidate"),
    );
    let mut black = FontFaceInfo::synthetic("black", "Weight Candidate");
    black.weight = FontWeight::Black;
    registry.register_face(FontSource::System, black);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Weight Candidate")),
            weight: Some(FontWeight::Black),
            ..FontRequest::default()
        })
        .expect("black weight request should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("black")));
}

#[test]
fn equal_rank_candidates_are_ordered_by_family_name() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontface.cxx alphabetical family-name ordering.
    let mut registry = FontRegistry::new();
    registry.register_face(FontSource::System, FontFaceInfo::synthetic("b", "B Family"));
    registry.register_face(FontSource::System, FontFaceInfo::synthetic("a", "A Family"));

    let resolved = registry
        .resolve_with_diagnostics(&FontRequest::default())
        .expect("unqualified request should resolve the first equal-rank candidate");

    assert_eq!(resolved.font_id, FontId(Arc::from("a")));
    assert_eq!(
        resolved
            .match_diagnostics
            .candidates
            .iter()
            .map(|candidate| candidate.family.as_ref())
            .collect::<Vec<_>>(),
        vec!["A Family", "B Family"]
    );
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
}

#[test]
fn substitution_request_uses_normalized_family_name() {
    // Source: LibreOffice vcl/source/font/DirectFontSubstitution.cxx normalized lookup.
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
            family: Some(Cow::Borrowed("Missing-Family")),
            ..FontRequest::default()
        })
        .expect("normalized substitution should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("replacement")));
    assert_eq!(resolved.resolved_family, Cow::Borrowed("Replacement"));
}

#[test]
fn italic_request_prefers_italic_face_without_synthetic_style() {
    // Source: LibreOffice vcl/qa/cppunit/physicalfontface.cxx italic match ranking.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("upright", "Italic Candidate"),
    );
    let mut italic = FontFaceInfo::synthetic("italic", "Italic Candidate");
    italic.slant = ooxmlsdk_fonts::FontSlant::Italic;
    registry.register_face(FontSource::System, italic);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Italic Candidate")),
            italic: true,
            ..FontRequest::default()
        })
        .expect("italic request should resolve");

    assert_eq!(resolved.font_id, FontId(Arc::from("italic")));
    assert!(!resolved.synthetic_italic);
}

#[test]
fn bold_request_uses_synthetic_bold_when_only_regular_face_exists() {
    // Source: LibreOffice font selection synthesizes style when exact face is absent.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("regular", "Synthetic Candidate"),
    );

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Synthetic Candidate")),
            bold: true,
            ..FontRequest::default()
        })
        .expect("bold request should resolve regular face");

    assert_eq!(resolved.font_id, FontId(Arc::from("regular")));
    assert!(resolved.synthetic_bold);
}

#[test]
fn resolved_font_preserves_variation_values() {
    // Source: LibreOffice vcl/qa/cppunit/complextext.cxx::testFontVariationEquality.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("variable", "Variable Face"),
    );

    let request = FontRequest {
        family: Some(Cow::Borrowed("Variable Face")),
        variations: vec![
            VariationValue {
                tag: Cow::Borrowed("wght"),
                value: 700.0,
            },
            VariationValue {
                tag: Cow::Borrowed("wdth"),
                value: 75.0,
            },
        ],
        ..FontRequest::default()
    };
    let resolved = registry
        .resolve(&request)
        .expect("variable font request should resolve");
    let options = ShapeOptions::from_request(&request, TextDirection::LeftToRight);

    assert_eq!(resolved.font_id, FontId(Arc::from("variable")));
    assert_eq!(options.variations.len(), 2);
    assert_eq!(options.variations[0].tag, Cow::Borrowed("wght"));
    assert_eq!(options.variations[0].value, 700.0);
    assert_eq!(options.variations[1].tag, Cow::Borrowed("wdth"));
    assert_eq!(options.variations[1].value, 75.0);
}

#[test]
fn font_variation_settings_parse_and_roundtrip_like_libreoffice() {
    // Source: LibreOffice vcl/qa/cppunit/complextext.cxx::testFontVariationParsing.
    let variations = parse_font_variations("\"wght\" 700, \"wdth\" 75");

    assert_eq!(
        variations,
        vec![
            VariationValue {
                tag: Cow::Borrowed("wght"),
                value: 700.0,
            },
            VariationValue {
                tag: Cow::Borrowed("wdth"),
                value: 75.0,
            },
        ]
    );
    assert_eq!(
        format_font_variations(&variations),
        "\"wght\" 700, \"wdth\" 75"
    );
    assert_eq!(parse_font_variations(""), Vec::<VariationValue<'_>>::new());
    assert_eq!(
        parse_font_variations("'wght' 400"),
        vec![VariationValue {
            tag: Cow::Borrowed("wght"),
            value: 400.0,
        }]
    );
    assert_eq!(
        parse_font_variations("\"slnt\" -12"),
        vec![VariationValue {
            tag: Cow::Borrowed("slnt"),
            value: -12.0,
        }]
    );
}

#[test]
fn font_feature_parser_matches_libreoffice_font_name_feature_syntax() {
    // Source: LibreOffice vcl/qa/cppunit/FontFeatureTest.cxx::testParseFeature.
    assert_eq!(trim_font_name_features("Font Name:abcd=5"), "Font Name");

    let (features, language) = parse_font_feature_settings("Font Name:abcd&bcde=2&-efgh&lang=slo");

    assert_eq!(language, Some(Cow::Borrowed("slo")));
    assert_eq!(features.len(), 3);
    assert_eq!(features[0].tag, Cow::Borrowed("abcd"));
    assert_eq!(features[0].value, 1);
    assert_eq!(features[1].tag, Cow::Borrowed("bcde"));
    assert_eq!(features[1].value, 2);
    assert_eq!(features[2].tag, Cow::Borrowed("efgh"));
    assert_eq!(features[2].value, 0);
}

#[test]
fn font_feature_parser_preserves_harfbuzz_ranges_and_css_forms() {
    // Source: LibreOffice vcl/qa/cppunit/FontFeatureTest.cxx::testParseFeature.
    let (features, _) = parse_font_feature_settings("Font:abcd[3:6]=2&'bcde' off&\"efgh\" on");

    assert_eq!(features[0].tag, Cow::Borrowed("abcd"));
    assert_eq!(features[0].value, 2);
    assert_eq!(features[0].start, 3);
    assert_eq!(features[0].end, 6);
    assert_eq!(features[1].tag, Cow::Borrowed("bcde"));
    assert_eq!(features[1].value, 0);
    assert_eq!(features[2].tag, Cow::Borrowed("efgh"));
    assert_eq!(features[2].value, 1);
}

#[test]
fn shape_options_preserve_features_variations_and_text_context() {
    // Source: LibreOffice vcl/qa/cppunit/FontFeatureTest.cxx and complextext.cxx feature/variation handling.
    let request = FontRequest {
        size_pt: FontSize(14.0),
        script: Some(TextScript::Arabic),
        language: Some(Cow::Borrowed("ar-SA")),
        features: vec![ooxmlsdk_fonts::FeatureValue {
            tag: Cow::Borrowed("liga"),
            value: 0,
        }],
        variations: vec![VariationValue {
            tag: Cow::Borrowed("slnt"),
            value: -12.0,
        }],
        ..FontRequest::default()
    };

    let options = ShapeOptions::from_request(&request, TextDirection::RightToLeft);

    assert_eq!(options.size_pt, FontSize(14.0));
    assert_eq!(options.direction, TextDirection::RightToLeft);
    assert_eq!(options.script, Some(TextScript::Arabic));
    assert_eq!(options.language, Some(Cow::Borrowed("ar-SA")));
    assert_eq!(options.features[0].tag, Cow::Borrowed("liga"));
    assert_eq!(options.features[0].value, 0);
    assert_eq!(options.variations[0].tag, Cow::Borrowed("slnt"));
    assert_eq!(options.variations[0].value, -12.0);
}

#[test]
fn approximate_shaping_preserves_rtl_script_and_language_context() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx bidi layout argument tests.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("arabic", "Arabic Face"),
    );

    let run = registry
        .shape_text(
            &FontRequest {
                family: Some(Cow::Borrowed("Arabic Face")),
                script: Some(TextScript::Arabic),
                language: Some(Cow::Borrowed("ar")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "ش",
            TextDirection::RightToLeft,
        )
        .expect("rtl text should shape");

    assert_eq!(run.direction, TextDirection::RightToLeft);
    assert_eq!(run.script, Some(TextScript::Arabic));
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
        requested_family: None,
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
fn fallback_diagnostics_keep_original_multibyte_text_range() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx::testImplLayoutRuns_PrepareFallbackRuns_LTR.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A', 'B']);
    registry.register_face(FontSource::System, primary);
    let mut fallback = FontFaceInfo::synthetic("fallback", "Fallback");
    fallback.coverage = coverage_for_chars(['中']);
    registry.register_face(FontSource::System, fallback);

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "A中B",
            TextDirection::LeftToRight,
        )
        .expect("fallback shaping should succeed");

    assert_eq!(runs[1].text, "中");
    assert_eq!(runs[1].text_range, 1..4);
    assert_eq!(runs[1].glyphs[0].text_range, 1..4);
    assert_eq!(runs[1].diagnostics.fallback_runs[0].text_range, 1..4);
    assert_eq!(
        runs[1].diagnostics.fallback_runs[0].family,
        Some(Cow::Borrowed("Fallback"))
    );
}

#[test]
fn fallback_keeps_private_use_symbols_on_primary_font() {
    // Source: LibreOffice sw/qa/extras/ooxmlexport/ooxmlexport27.cxx::testCool15788_symbolContentControl.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);
    let mut fallback = FontFaceInfo::synthetic("fallback", "Fallback");
    fallback.coverage = coverage_for_chars(['\u{f06c}']);
    registry.register_face(FontSource::System, fallback);

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "\u{f06c}",
            TextDirection::LeftToRight,
        )
        .expect("PUA text should shape on the primary font");

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font_id, FontId(Arc::from("primary")));
    assert!(runs[0].diagnostics.fallback_runs.is_empty());
}

#[test]
fn fallback_keeps_mongolian_nnbsp_with_following_mongolian_cluster() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx::testMongolianNNBSP.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);
    let mut fallback = FontFaceInfo::synthetic("mongolian", "Mongolian");
    fallback.coverage = coverage_for_chars(['\u{202f}', '\u{1822}']);
    registry.register_face(FontSource::System, fallback);

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "A\u{202f}\u{1822}A",
            TextDirection::LeftToRight,
        )
        .expect("Mongolian fallback should shape");

    assert_eq!(runs.len(), 3);
    assert_eq!(runs[1].font_id, FontId(Arc::from("mongolian")));
    assert_eq!(runs[1].text, "\u{202f}\u{1822}");
    assert_eq!(runs[1].text_range, 1..7);
}

#[test]
fn fallback_chain_is_filtered_by_script() {
    // Source: LibreOffice glyph fallback keeps script-specific fallback chains separate.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);
    let mut arabic = FontFaceInfo::synthetic("arabic", "Arabic Fallback");
    arabic.coverage = coverage_for_chars(['ش']);
    registry.register_face(FontSource::System, arabic);
    registry.book.fallback_chains.push(FontFallbackChain {
        requested_family: None,
        script: Some(TextScript::Arabic),
        language: None,
        families: vec![Cow::Borrowed("Arabic Fallback")],
    });

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                script: Some(TextScript::Latin),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "Aش",
            TextDirection::LeftToRight,
        )
        .expect("fallback shaping should still use global coverage search");

    assert_eq!(runs[1].font_id, FontId(Arc::from("arabic")));
    assert_eq!(runs[1].diagnostics.fallback_runs[0].fallback_level, 1);
}

#[test]
fn fallback_chain_language_matching_is_case_insensitive() {
    // Source: LibreOffice glyph fallback language matching.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);
    let mut fallback = FontFaceInfo::synthetic("ja", "Japanese Fallback");
    fallback.coverage = coverage_for_chars(['中']);
    registry.register_face(FontSource::System, fallback);
    registry.book.fallback_chains.push(FontFallbackChain {
        requested_family: None,
        script: Some(TextScript::Han),
        language: Some(Cow::Borrowed("ja-JP")),
        families: vec![Cow::Borrowed("Japanese Fallback")],
    });

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                script: Some(TextScript::Han),
                language: Some(Cow::Borrowed("JA-jp")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "A中",
            TextDirection::LeftToRight,
        )
        .expect("language fallback should match case-insensitively");

    assert_eq!(runs[1].font_id, FontId(Arc::from("ja")));
}

#[test]
fn fallback_chain_does_not_duplicate_primary_family() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx fallback run preparation coverage.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);
    registry.book.fallback_chains.push(FontFallbackChain {
        requested_family: None,
        script: Some(TextScript::Latin),
        language: None,
        families: vec![Cow::Borrowed("Primary")],
    });

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                script: Some(TextScript::Latin),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "AA",
            TextDirection::LeftToRight,
        )
        .expect("primary shaping should succeed");

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font_id, FontId(Arc::from("primary")));
    assert!(runs[0].diagnostics.fallback_runs.is_empty());
}

#[test]
fn fallback_can_use_any_registered_face_with_required_coverage() {
    // Source: LibreOffice PhysicalFontCollection glyph fallback search.
    let mut registry = FontRegistry::new();
    let mut primary = FontFaceInfo::synthetic("primary", "Primary");
    primary.coverage = coverage_for_chars(['A']);
    registry.register_face(FontSource::System, primary);
    let mut discovered = FontFaceInfo::synthetic("discovered", "Discovered Fallback");
    discovered.coverage = coverage_for_chars(['中']);
    registry.register_face(FontSource::System, discovered);

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Primary")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "A中",
            TextDirection::LeftToRight,
        )
        .expect("global coverage fallback should resolve");

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[1].font_id, FontId(Arc::from("discovered")));
}

#[test]
fn shaping_empty_text_still_returns_a_stable_run() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx empty layout-run handling.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("empty", "Empty"),
    );

    let runs = registry
        .shape_text_runs(
            &FontRequest {
                family: Some(Cow::Borrowed("Empty")),
                ..FontRequest::default()
            },
            "",
            TextDirection::LeftToRight,
        )
        .expect("empty text should shape");

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text_range, 0..0);
    assert!(runs[0].glyphs.is_empty());
}

#[test]
fn approximate_shaping_records_whitespace_safe_breaks() {
    // Source: LibreOffice vcl/qa/cppunit/text.cxx layout safe-break handling.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("approx", "Approx"),
    );

    let run = registry
        .shape_text(
            &FontRequest {
                family: Some(Cow::Borrowed("Approx")),
                size_pt: FontSize(12.0),
                ..FontRequest::default()
            },
            "A B",
            TextDirection::LeftToRight,
        )
        .expect("approximate shaping should succeed");

    assert!(run.approximate);
    assert_eq!(run.safe_breaks, vec![2]);
    assert!(run.glyphs[1].safe_to_break);
}

#[test]
fn coverage_reports_missing_glyph_text_ranges() {
    // Source: LibreOffice glyph fallback missing-codepoint diagnostics.
    let coverage = coverage_for_chars(['A']);

    assert_eq!(
        coverage.missing_glyphs("A中"),
        vec![ooxmlsdk_fonts::MissingGlyph {
            codepoint: u32::from('中'),
            text_range: 1..4,
        }]
    );
}

#[test]
fn coverage_tracks_non_bmp_codepoints() {
    // Source: LibreOffice vcl font coverage and fallback tests for full Unicode ranges.
    let coverage = coverage_for_chars(['😀']);

    assert!(coverage.contains_char('😀'));
    assert!(!coverage.contains_char('A'));
}

#[test]
fn registered_memory_face_data_is_available_for_renderers() {
    // Source: LibreOffice vcl/source/pdf/pdfwriter*.cxx uses the selected face for output.
    let mut registry = FontRegistry::new();
    registry.register_face(
        FontSource::Memory {
            id: Cow::Borrowed("memory-face"),
            data: [0, 1, 2, 3].as_slice().into(),
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
fn ahem_ttf_fixture_extracts_name_style_and_postscript_metadata() {
    // Source: LibreOffice vcl/qa/cppunit/font/TTFStructureTest.cxx::testReadTTFStructure.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/font/data/Ahem.ttf");

    let font_id = registry
        .register_test_fixture_font("ahem", bytes)
        .expect("Ahem.ttf should parse");
    let face = registry
        .face(&font_id)
        .expect("registered face should exist");

    assert_eq!(face.family_names, vec![Cow::Borrowed("Ahem")]);
    assert_eq!(face.style_name, Some(Cow::Borrowed("Regular")));
    assert_eq!(face.postscript_name, Some(Cow::Borrowed("Ahem")));
}

#[test]
fn ahem_ttf_fixture_extracts_weight_stretch_pitch_and_metrics() {
    // Source: LibreOffice vcl/qa/cppunit/font/TTFStructureTest.cxx OS/2/head table assertions.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/font/data/Ahem.ttf");

    let font_id = registry
        .register_test_fixture_font("ahem", bytes)
        .expect("Ahem.ttf should parse");
    let face = registry
        .face(&font_id)
        .expect("registered face should exist");

    assert_eq!(face.weight, FontWeight::Normal);
    assert_eq!(face.stretch, FontStretch::Normal);
    assert_eq!(face.pitch, FontPitch::Fixed);
    assert_eq!(face.metrics.em_size, 1.0);
    assert!(face.metrics.vertical.ascent_pt > 0.0);
    assert!(face.metrics.vertical.descent_pt > 0.0);
}

#[test]
fn ahem_ttf_fixture_extracts_coverage_without_variation_axes() {
    // Source: LibreOffice vcl/qa/cppunit/fontcharmap.cxx and FontVariationTest non-variable coverage.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/font/data/Ahem.ttf");

    let font_id = registry
        .register_test_fixture_font("ahem", bytes)
        .expect("Ahem.ttf should parse");
    let face = registry
        .face(&font_id)
        .expect("registered face should exist");

    assert!(face.coverage.contains_char('A'));
    assert!(face.coverage.contains_char(' '));
    assert!(face.axes.is_empty());
}

#[test]
fn fraunces_variable_fixture_extracts_opsz_and_wght_axes() {
    // Source: LibreOffice vcl/qa/cppunit/FontVariationTest.cxx variable axis behavior.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/data/Fraunces-VariableFont_opsz,wght.ttf");

    let font_id = registry
        .register_test_fixture_font("fraunces", bytes)
        .expect("Fraunces variable fixture should parse");
    let face = registry
        .face(&font_id)
        .expect("registered face should exist");
    let tags = face
        .axes
        .iter()
        .map(|axis| axis.tag.as_ref())
        .collect::<Vec<_>>();

    assert_eq!(tags, vec!["opsz", "wght"]);
    for axis in &face.axes {
        assert!(axis.min <= axis.default);
        assert!(axis.default <= axis.max);
        assert!(axis.min < axis.max);
    }
}

#[test]
fn fraunces_variable_fixture_exposes_opentype_feature_tags() {
    // Source: LibreOffice vcl/qa/cppunit/FontFeatureTest.cxx OpenType feature discovery.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/data/Fraunces-VariableFont_opsz,wght.ttf");

    let font_id = registry
        .register_test_fixture_font("fraunces", bytes)
        .expect("Fraunces variable fixture should parse");
    let face = registry
        .face(&font_id)
        .expect("registered face should exist");

    assert!(!face.features.is_empty());
    assert!(face.features.iter().any(|feature| feature.tag == "kern"));
    assert!(face.features.iter().all(|feature| feature.tag.len() == 4));
}

#[test]
fn emoji_subset_fixture_extracts_non_bmp_coverage() {
    // Source: LibreOffice vcl/qa/cppunit/data/tdf153440.ttf fixture coverage.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/data/tdf153440.ttf");

    let font_id = registry
        .register_test_fixture_font("emoji-subset", bytes)
        .expect("emoji subset fixture should parse");
    let face = registry
        .face(&font_id)
        .expect("registered face should exist");

    assert!(face.coverage.contains_char('🌿'));
    assert!(face.coverage.contains_char(' '));
    assert!(!face.coverage.contains_char('A'));
}

#[test]
fn default_office_policy_provides_pdf_font_aliases() {
    // Source: ooxmlsdk-pdf fonts.rs Office/PDF fallback aliases.
    let mut registry = FontRegistry::with_default_policy();
    registry.register_face(
        FontSource::System,
        FontFaceInfo::synthetic("carlito", "Carlito"),
    );

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Calibri")),
            ..FontRequest::default()
        })
        .expect("Calibri should resolve through default policy");

    assert_eq!(resolved.font_id, FontId(Arc::from("carlito")));
    assert_eq!(resolved.resolved_family, Cow::Borrowed("Carlito"));
}

#[test]
fn real_ttf_shaping_applies_character_spacing_to_inter_glyph_advances() {
    // Source: ooxmlsdk-pdf text_metrics.rs character_spacing_pt tracking behavior.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/font/data/Ahem.ttf");
    registry
        .register_test_fixture_font("ahem", bytes)
        .expect("Ahem.ttf should parse");
    let request = FontRequest {
        family: Some(Cow::Borrowed("Ahem")),
        size_pt: FontSize(10.0),
        ..FontRequest::default()
    };
    let resolved = registry.resolve(&request).expect("Ahem should resolve");
    let base = registry
        .shape_font_face(
            &resolved,
            "AA",
            &ShapeOptions::from_request(&request, TextDirection::LeftToRight),
        )
        .expect("base shaping should succeed");
    let mut options = ShapeOptions::from_request(&request, TextDirection::LeftToRight);
    options.character_spacing_pt = 2.0;

    let tracked = registry
        .shape_font_face(&resolved, "AA", &options)
        .expect("tracked shaping should succeed");

    assert_eq!(tracked.advance_pt, base.advance_pt + 2.0);
}

#[test]
fn real_ttf_small_caps_maps_lowercase_to_uppercase_and_scales_size() {
    // Source: ooxmlsdk-pdf text_metrics.rs small-caps case mapping behavior.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/font/data/Ahem.ttf");
    registry
        .register_test_fixture_font("ahem", bytes)
        .expect("Ahem.ttf should parse");
    let request = FontRequest {
        family: Some(Cow::Borrowed("Ahem")),
        size_pt: FontSize(10.0),
        ..FontRequest::default()
    };
    let resolved = registry.resolve(&request).expect("Ahem should resolve");
    let mut options = ShapeOptions::from_request(&request, TextDirection::LeftToRight);
    options.small_caps = true;

    let shaped = registry
        .shape_font_face(&resolved, "a", &options)
        .expect("small-caps shaping should succeed");

    assert_eq!(shaped.glyphs[0].source_char, Some('A'));
    assert!(shaped.advance_pt > 0.0);
    assert!(shaped.advance_pt < 10.0);
}

#[test]
fn script_direction_runs_split_script_bidi_and_small_caps_segments() {
    // Source: LibreOffice tdf#160401/#i78474 disables small-caps for CTL script runs.
    let text = "aش";
    let runs = script_direction_runs(text, FontSize(10.0), true);

    assert_eq!(runs.len(), 2);
    assert_eq!(&text[runs[0].text_range.clone()], "a");
    assert_eq!(runs[0].script, TextScript::Latin);
    assert_eq!(runs[0].direction, TextDirection::LeftToRight);
    assert_eq!(runs[0].size_pt, FontSize(10.0));
    assert!(runs[0].small_caps);
    assert_eq!(runs[1].script, TextScript::Arabic);
    assert_eq!(runs[1].direction, TextDirection::RightToLeft);
    assert!(!runs[1].small_caps);
}

fn script_direction_runs_with_app_script(text: &str, app_script: TextScript) -> Vec<FontScriptRun> {
    script_direction_runs_with_options(
        text,
        FontSize(10.0),
        ScriptScanOptions {
            app_script,
            small_caps: false,
        },
    )
}

fn run_texts_and_scripts<'a>(
    text: &'a str,
    runs: &'a [FontScriptRun],
) -> Vec<(&'a str, TextScript)> {
    runs.iter()
        .map(|run| (&text[run.text_range.clone()], run.script))
        .collect()
}

#[test]
fn script_direction_runs_assign_initial_weak_text_to_first_strong_script() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx::testWeakAtStart.
    let runs = script_direction_runs_with_app_script("“x”", TextScript::Arabic);

    assert_eq!(
        run_texts_and_scripts("“x”", &runs),
        vec![("“x”", TextScript::Latin)]
    );
}

#[test]
fn script_direction_runs_assign_only_weak_text_to_application_script() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx::testOnlyWeak.
    let runs = script_direction_runs_with_app_script("“”", TextScript::Arabic);

    assert_eq!(
        run_texts_and_scripts("“”", &runs),
        vec![("“”", TextScript::Arabic)]
    );
}

#[test]
fn script_direction_runs_keep_weak_text_with_adjacent_strong_runs() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx::testStrongChange.
    let runs = script_direction_runs_with_app_script("wide 廣 vast", TextScript::Latin);

    assert_eq!(
        run_texts_and_scripts("wide 廣 vast", &runs),
        vec![
            ("wide ", TextScript::Latin),
            ("廣 ", TextScript::Han),
            ("vast", TextScript::Latin),
        ]
    );
}

#[test]
fn script_direction_runs_assign_smart_quotes_like_libreoffice() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx smart quote tests.
    let runs = script_direction_runs_with_app_script("Before “水” After", TextScript::Latin);

    assert_eq!(
        run_texts_and_scripts("Before “水” After", &runs),
        vec![
            ("Before “", TextScript::Latin),
            ("水” ", TextScript::Han),
            ("After", TextScript::Latin),
        ]
    );

    let leading = script_direction_runs_with_app_script("“廣”", TextScript::Latin);
    assert_eq!(
        run_texts_and_scripts("“廣”", &leading),
        vec![("“廣”", TextScript::Han)]
    );
}

#[test]
fn script_direction_runs_attach_inherited_mark_sequence_to_next_strong_script() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx::testNonspacingMark.
    let runs = script_direction_runs_with_app_script(
        "Before \u{0944}\u{0911}\u{0911} After",
        TextScript::Latin,
    );

    assert_eq!(
        run_texts_and_scripts("Before \u{0944}\u{0911}\u{0911} After", &runs),
        vec![
            ("Before", TextScript::Latin),
            (" \u{0944}\u{0911}\u{0911} ", TextScript::Devanagari),
            ("After", TextScript::Latin),
        ]
    );
}

#[test]
fn script_direction_runs_split_simple_rtl_span_like_libreoffice() {
    // Source: LibreOffice i18nutil/qa/cppunit/test_scriptchangescanner.cxx::testRtlRunTrivial.
    let runs = script_direction_runs_with_app_script("Before אאאאאא after", TextScript::Latin);

    assert_eq!(
        run_texts_and_scripts("Before אאאאאא after", &runs),
        vec![
            ("Before ", TextScript::Latin),
            ("אאאאאא ", TextScript::Hebrew),
            ("after", TextScript::Latin),
        ]
    );
    assert_eq!(runs[1].direction, TextDirection::RightToLeft);
}

#[test]
fn font_usage_collector_applies_embedding_policy() {
    // Source: LibreOffice embeddedfontsmanager.cxx restricted embedding policy.
    let run = shaped_usage_run("restricted", 41, "A", 'A');
    let mut collector = FontUsageCollector::default();

    collector.record_run_with_policy(
        &run,
        FontEmbeddingPolicy {
            subset_policy: FontSubsetPolicy::DoNotEmbed,
            installable: false,
            restricted: true,
        },
    );

    assert_eq!(
        collector.usages[0].subset_policy,
        FontSubsetPolicy::DoNotEmbed
    );
    assert!(!collector.usages[0].needs_embedding);
}

#[test]
fn real_ttf_face_and_glyph_bounds_are_exposed() {
    // Source: LibreOffice vcl/qa/cppunit/logicalfontinstance.cxx glyph bounds coverage.
    let mut registry = FontRegistry::new();
    let bytes = read_core_fixture("vcl/qa/cppunit/font/data/Ahem.ttf");
    registry
        .register_test_fixture_font("ahem", bytes.clone())
        .expect("Ahem.ttf should parse");
    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Ahem")),
            size_pt: FontSize(10.0),
            ..FontRequest::default()
        })
        .expect("Ahem should resolve");
    let face = registry
        .face(&resolved.font_id)
        .expect("registered face should exist");
    let shaped = registry
        .shape_text(
            &FontRequest {
                family: Some(Cow::Borrowed("Ahem")),
                size_pt: FontSize(10.0),
                ..FontRequest::default()
            },
            "A",
            TextDirection::LeftToRight,
        )
        .expect("Ahem shaping should succeed");

    let bounds = resolved
        .glyph_bounds(&bytes, shaped.glyphs[0].glyph_id, FontSize(10.0))
        .expect("glyph bounds lookup should succeed");

    assert!(face.bounds.global.is_some());
    assert!(shaped.glyphs[0].bounds.is_some());
    assert!(bounds.is_some());
}

#[test]
fn shaped_glyphs_mark_cjk_and_arabic_as_justifiable() {
    // Source: Typst shaping.rs and LibreOffice VCL justification metadata.
    let resolved = ooxmlsdk_fonts::ResolvedFont {
        font_id: FontId(Arc::from("approx")),
        resolved_family: Cow::Borrowed("Approx"),
        source: FontSource::System,
        face_index: 0,
        synthetic_bold: false,
        synthetic_italic: false,
        metrics: FontMetrics::default(),
        match_diagnostics: Default::default(),
    };

    let shaped =
        resolved.shape_approximate("中شA", FontSize(10.0), TextDirection::LeftToRight, None);

    assert!(shaped.glyphs[0].justifiable);
    assert!(shaped.glyphs[1].justifiable);
    assert!(!shaped.glyphs[2].justifiable);
}

#[test]
fn registered_face_data_preserves_style_source_and_face_index() {
    // Source: LibreOffice VCL face metadata is carried to renderer/output paths.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("embedded-face", "Embedded Face");
    face.style_name = Some(Cow::Borrowed("Bold"));
    face.face_index = 3;
    registry.register_face(
        FontSource::EmbeddedOoxml {
            id: Cow::Borrowed("embedded-face"),
            data: [9, 8, 7].as_slice().into(),
        },
        face,
    );

    let data = registry
        .font_face_data(&FontId(Arc::from("embedded-face")))
        .expect("registered face data should be available");

    assert_eq!(data.face_index, 3);
    assert_eq!(data.style_name, Some(Cow::Borrowed("Bold")));
    assert_eq!(data.data.as_deref(), Some([9, 8, 7].as_slice()));
    assert!(matches!(data.source, FontSource::EmbeddedOoxml { .. }));
}

#[test]
fn resolved_font_preserves_registered_source_and_face_index() {
    // Source: LibreOffice PDF/font output uses the selected physical face identity.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("path-face", "Path Face");
    face.face_index = 2;
    registry.register_face(
        FontSource::Path {
            id: Cow::Borrowed("path-face"),
            path: std::path::PathBuf::from("/fonts/path-face.ttc"),
            data: None,
        },
        face,
    );

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Path Face")),
            ..FontRequest::default()
        })
        .expect("path face should resolve");

    assert_eq!(resolved.face_index, 2);
    assert!(matches!(resolved.source, FontSource::Path { .. }));
}

#[test]
fn unknown_font_face_data_returns_none() {
    // Source: LibreOffice renderer paths only expose data for known selected faces.
    let registry = FontRegistry::new();

    assert_eq!(registry.font_face_data(&FontId(Arc::from("missing"))), None);
}

#[test]
fn font_usage_collector_records_glyphs_and_unicode_ranges() {
    // Source: LibreOffice PDF font subsetting paths in vcl/source/pdf/.
    let run = ShapedRun {
        font_id: FontId(Arc::from("subset-face")),
        text: "AB",
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
fn font_usage_collector_merges_multiple_runs_for_same_font() {
    // Source: LibreOffice PDF font subsetting paths merge glyph use per selected face.
    let first = shaped_usage_run("subset-face", 41, "A", 'A');
    let second = shaped_usage_run("subset-face", 42, "B", 'B');
    let mut collector = FontUsageCollector::default();

    collector.record_run(&first);
    collector.record_run(&second);

    assert_eq!(collector.usages.len(), 1);
    assert!(collector.usages[0].glyph_ids.contains(&41));
    assert!(collector.usages[0].glyph_ids.contains(&42));
    assert_eq!(collector.usages[0].unicode_ranges, vec![65..67]);
}

#[test]
fn approximate_runs_do_not_require_font_embedding() {
    // Source: LibreOffice PDF output distinguishes real embedded face data from fallback metrics.
    let run = ShapedRun {
        font_id: FontId(Arc::from("system-face")),
        text: "A",
        text_range: 0..1,
        glyphs: Cow::Owned(vec![ShapedGlyph {
            glyph_id: 0,
            text_range: 0..1,
            source_char: Some('A'),
            ..ShapedGlyph::default()
        }]),
        advance_pt: 0.0,
        direction: TextDirection::LeftToRight,
        script: Some(TextScript::Latin),
        safe_breaks: Vec::new(),
        approximate: true,
        decorations: Vec::new(),
        diagnostics: ShapingDiagnostics::default(),
    };
    let mut collector = FontUsageCollector::default();

    collector.record_run(&run);

    assert!(!collector.usages[0].needs_embedding);
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

#[test]
fn missing_theme_font_slot_returns_none() {
    // Source: LibreOffice OOXML theme font import falls back only when a slot exists.
    let map = ThemeFontMap {
        minor_latin: Some(Cow::Borrowed("Minor Latin")),
        ..ThemeFontMap::default()
    };

    assert_eq!(map.resolve(ThemeFontKind::MajorLatin), None);
    assert_eq!(
        map.resolve(ThemeFontKind::MinorLatin),
        Some(Cow::Borrowed("Minor Latin"))
    );
}

#[test]
fn default_font_metrics_start_at_zero_values() {
    // Source: LibreOffice vcl/qa/cppunit/fontmetric.cxx::testSpacings default metric values.
    let metrics = FontMetrics::default();

    assert_eq!(metrics.em_size, 1.0);
    assert_eq!(metrics.vertical.ascent_pt, 0.0);
    assert_eq!(metrics.vertical.descent_pt, 0.0);
    assert_eq!(metrics.vertical.internal_leading_pt, 0.0);
    assert_eq!(metrics.vertical.external_leading_pt, 0.0);
    assert_eq!(metrics.decoration.underline_offset_pt, 0.0);
    assert_eq!(metrics.script.superscript_offset_pt, 0.0);
}

#[test]
fn resolved_metrics_scale_vertical_values_to_requested_size() {
    // Source: LibreOffice vcl/source/font/fontmetric.cxx metric scaling behavior.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("metric-face", "Metric Face");
    face.metrics.vertical = VerticalMetrics {
        ascent_pt: 0.8,
        descent_pt: 0.2,
        line_gap_pt: 0.1,
        ..VerticalMetrics::default()
    };
    registry.register_face(FontSource::System, face);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Metric Face")),
            ..FontRequest::default()
        })
        .expect("metric face should resolve");
    let metrics = resolved.metrics_at_size(FontSize(10.0));

    assert_eq!(metrics.vertical.ascent_pt, 8.0);
    assert_eq!(metrics.vertical.descent_pt, 2.0);
    assert_eq!(metrics.vertical.line_gap_pt, 1.0);
}

#[test]
fn resolved_metrics_scale_leading_and_cjk_advances() {
    // Source: LibreOffice vcl/qa/cppunit/fontmetric.cxx spacing metric behavior.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("metric-face", "Metric Face");
    face.metrics.vertical = VerticalMetrics {
        internal_leading_pt: 0.125,
        external_leading_pt: 0.25,
        cjk_horizontal_advance_pt: 1.0,
        cjk_vertical_advance_pt: 1.25,
        ..VerticalMetrics::default()
    };
    registry.register_face(FontSource::System, face);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Metric Face")),
            ..FontRequest::default()
        })
        .expect("metric face should resolve");
    let metrics = resolved.metrics_at_size(FontSize(20.0));

    assert_eq!(metrics.vertical.internal_leading_pt, 2.5);
    assert_eq!(metrics.vertical.external_leading_pt, 5.0);
    assert_eq!(metrics.vertical.cjk_horizontal_advance_pt, 20.0);
    assert_eq!(metrics.vertical.cjk_vertical_advance_pt, 25.0);
}

#[test]
fn resolved_metrics_scale_decoration_values() {
    // Source: LibreOffice font metric underline/strikeout values feed text decoration output.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("decor-face", "Decor Face");
    face.metrics.decoration = DecorationMetrics {
        underline_offset_pt: 0.125,
        underline_thickness_pt: 0.25,
        strikeout_offset_pt: 0.5,
        strikeout_thickness_pt: 0.75,
    };
    registry.register_face(FontSource::System, face);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Decor Face")),
            ..FontRequest::default()
        })
        .expect("decoration face should resolve");
    let metrics = resolved.metrics_at_size(FontSize(10.0));

    assert_eq!(metrics.decoration.underline_offset_pt, 1.25);
    assert_eq!(metrics.decoration.underline_thickness_pt, 2.5);
    assert_eq!(metrics.decoration.strikeout_offset_pt, 5.0);
    assert_eq!(metrics.decoration.strikeout_thickness_pt, 7.5);
}

#[test]
fn resolved_metrics_scale_script_offsets_but_not_scale_factors() {
    // Source: LibreOffice font metric equality/scalar behavior for script-related font data.
    let mut registry = FontRegistry::new();
    let mut face = FontFaceInfo::synthetic("script-face", "Script Face");
    face.metrics.script = ScriptMetrics {
        superscript_scale: 0.58,
        subscript_scale: 0.62,
        superscript_offset_pt: 0.375,
        subscript_offset_pt: -0.25,
        small_caps_scale: 0.8,
    };
    registry.register_face(FontSource::System, face);

    let resolved = registry
        .resolve(&FontRequest {
            family: Some(Cow::Borrowed("Script Face")),
            ..FontRequest::default()
        })
        .expect("script metric face should resolve");
    let metrics = resolved.metrics_at_size(FontSize(20.0));

    assert_eq!(metrics.script.superscript_scale, 0.58);
    assert_eq!(metrics.script.subscript_scale, 0.62);
    assert_eq!(metrics.script.superscript_offset_pt, 7.5);
    assert_eq!(metrics.script.subscript_offset_pt, -5.0);
    assert_eq!(metrics.script.small_caps_scale, 0.8);
}

#[test]
fn font_metrics_scaled_sets_output_em_size() {
    // Source: LibreOffice vcl/qa/cppunit/fontmetric.cxx metric setter/equality behavior.
    let metrics = FontMetrics {
        em_size: 2.0,
        vertical: VerticalMetrics {
            ascent_pt: 1.0,
            ..VerticalMetrics::default()
        },
        ..FontMetrics::default()
    }
    .scaled(10.0);

    assert_eq!(metrics.em_size, 10.0);
    assert_eq!(metrics.vertical.ascent_pt, 5.0);
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

fn shaped_usage_run(
    font_id: &'static str,
    glyph_id: u32,
    text: &'static str,
    ch: char,
) -> ShapedRun<'static, 'static> {
    ShapedRun {
        font_id: FontId(Arc::from(font_id)),
        text,
        text_range: 0..text.len(),
        glyphs: Cow::Owned(vec![ShapedGlyph {
            glyph_id,
            text_range: 0..text.len(),
            source_char: Some(ch),
            ..ShapedGlyph::default()
        }]),
        advance_pt: 12.0,
        direction: TextDirection::LeftToRight,
        script: Some(TextScript::Latin),
        safe_breaks: Vec::new(),
        approximate: false,
        decorations: Vec::new(),
        diagnostics: ShapingDiagnostics::default(),
    }
}

fn read_core_fixture(relative_path: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../core")
        .join(relative_path);
    fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

# LibreOffice Fonts Test Migration Index

This document is a migration index for LibreOffice font-related coverage.
Use it as the checklist for translating tests into `ooxmlsdk-fonts`,
`ooxmlsdk-pdf`, and this test-suite.
All listed rows are intended to migrate eventually; target only records where
the migrated coverage should land first.

## Migration Rule

A row counts as migrated only when the test exercises behavior owned by
`ooxmlsdk-fonts`, `ooxmlsdk-layout`, or `ooxmlsdk-pdf`.

Valid migrated coverage includes:

- font matching, aliasing, substitution, charset/pitch/style ranking, fallback,
  shaping, metrics, or usage collection through `ooxmlsdk-fonts`
- OOXML font request/theme/script mapping that feeds layout
- package import or fixture-backed layout/PDF output that feeds those APIs and
  asserts the resolved behavior

Raw ZIP/XML containment checks are source evidence only. Keep those fixture
paths in this document, but do not add them to `ooxmlsdk-fonts-test` unless the
test also proves imported or rendered font behavior.

## Current Scope

The current `ooxmlsdk-fonts-test` migration is intentionally behavior-first:

- `font_backend.rs` covers the LibreOffice VCL font model equivalents that
  already fit `ooxmlsdk-fonts`: normalized lookup, semicolon family tokens,
  aliasing, charset/pitch/symbol matching, style ranking, substitution,
  fallback slicing and filtering, metrics, shaping, safe breaks, TTF face
  metadata, coverage, variation axes, OpenType feature tags, face data, and usage
  collection. It also covers the shared font capabilities migrated from
  `ooxmlsdk-pdf`: Office alias/fallback policy, character spacing,
  small-caps shaping, script/bidi run splitting, embedding policy, glyph
  bounds, CJK/Arabic justification metadata, private-use symbol fallback
  ownership, Mongolian NNBSP fallback clustering, font feature parsing, and font
  variation parsing/serialization.
- `layout_ooxml_fonts.rs` covers the OOXML-to-layout boundary: script-specific
  DOCX font families, theme fallback, common-character inheritance, shaped
  script runs, and XLSX text style mapping into layout font requests.
- Fixture-backed LibreOffice DOCX/XLSX/PPTX/PDF rows remain in the table as
  source evidence. They should migrate only when the test can assert imported
  layout/PDF font behavior instead of raw XML presence.

Current known gaps versus LibreOffice are still tracked in the tables below:
full `ScriptChangeScanner` hint/bidi-control behavior, generic family-class
matching, package-backed embedded font behavior, and PDF-visible variable
font/kashida output.

## Next Alignment Pass

Prioritize behavior that can be asserted through stable font/layout APIs:

| Priority | Scope | Upstream source | Notes |
| --- | --- | --- | --- |
| P0 | generic family-class matching | `../core/vcl/qa/cppunit/physicalfontcollection.cxx` | Initial synthetic coverage migrated: explicit serif/sans/decorative class matching, fixed pitch class matching, name-derived brush/script, titling, capitals, oldstyle, schoolbook, and negative oldstyle matching. |
| P1 | script/direction scan parity | `../core/i18nutil/qa/cppunit/test_scriptchangescanner.cxx` | Initial coverage migrated: weak-at-start, only-weak, strong-change, smart quotes, nonspacing mark ownership, and simple RTL. Full hint/bidi-control behavior still needs an API that carries paragraph/run hints and control-character ownership. |
| P2 | package-backed font semantics | Writer/Calc/Impress font fixtures listed below | PPTX embedded typeface import is covered through `PptxLayoutSummary`. Remaining fixtures already exist in corpus for DOCX embedded fonts, DOCX font family, XLSX charset/font-size, and WordArt font theme/text; migrate only rows that can assert imported font requests, layout runs, or PDF-visible output. |
| P3 | PDF-visible variable/kashida output | `../core/vcl/qa/cppunit/pdfexport/*.cxx` | Keep in `ooxmlsdk-pdf-test`; these are rendering/output assertions, not pure font registry tests. |

## Legend

| Field | Meaning |
| --- | --- |
| Target | Suggested destination for the translated test |
| Fixture | LibreOffice fixture path when the test is fixture-backed |

## VCL Font Model

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| font request defaults and setters: name, weight, width, pitch, italic, alignment, quality | `../core/vcl/qa/cppunit/font.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| emphasis mark language positioning and mark geometry | `../core/vcl/qa/cppunit/font.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| face ordering by width, weight, italic, family, style | `../core/vcl/qa/cppunit/physicalfontface.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| LO face match score components | `../core/vcl/qa/cppunit/physicalfontface.cxx::testMatchStatusValue` | synthetic | `ooxmlsdk-fonts` unit |
| family create/find and normalized search names | `../core/vcl/qa/cppunit/physicalfontcollection.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| semicolon token family lookup | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindFontFamilyByTokenNames` | synthetic | `ooxmlsdk-fonts` unit |
| CJK family attribute matching | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindCJKFamily` | synthetic | `ooxmlsdk-fonts` unit |
| CJK family negative match | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldNotFindCJKFamily` | synthetic | `ooxmlsdk-fonts` unit |
| symbol family preference | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindStarsymbolFamily` | synthetic | `ooxmlsdk-fonts` unit |
| OpenSymbol preferred among symbol families | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindOpensymbolFamilyWithMultipleSymbolFamilies` | synthetic | `ooxmlsdk-fonts` unit |
| symbol-encoded face matching | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldFindSymbolFamilyByMatchType` | synthetic | `ooxmlsdk-fonts` unit |
| normal face does not satisfy symbol request | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testShouldNotFindSymbolFamily` | synthetic | `ooxmlsdk-fonts` unit |
| generic family attribute matching | `../core/vcl/qa/cppunit/physicalfontcollection.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| family alias behavior | `../core/vcl/qa/cppunit/physicalfontcollection.cxx::testFontFamilyAliases` | synthetic | `ooxmlsdk-fonts` unit |
| physical family face insertion | `../core/vcl/qa/cppunit/physicalfontfamily.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| stable physical face ids | `../core/vcl/qa/cppunit/physicalfontfacecollection.cxx` | synthetic | `ooxmlsdk-fonts` unit |

## Fallback Runs, Bidi, and Script Runs

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| layout run normalization | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_Normalize` | synthetic | `ooxmlsdk-fonts` unit |
| LTR fallback run slicing | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_PrepareFallbackRuns_LTR` | synthetic | `ooxmlsdk-fonts` unit |
| LTR fallback preserves original order | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_PrepareFallbackRuns_LTR_PreservesOrder` | synthetic | `ooxmlsdk-fonts` unit |
| RTL fallback run ordering | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_PrepareFallbackRuns_RTL` | synthetic | `ooxmlsdk-fonts` unit |
| fallback run tdf161397 regression | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_tdf161397` | synthetic | `ooxmlsdk-fonts` unit |
| bidirectional run growth | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_GrowBidirectional` | synthetic | `ooxmlsdk-fonts` unit |
| reverse RTL tail ordering | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutRuns_ReverseTail` | synthetic | `ooxmlsdk-fonts` unit |
| bidi strong run splitting | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutArgsBiDiStrong` | synthetic | `ooxmlsdk-fonts` unit |
| bidi RTL run splitting | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutArgsBiDiRtl` | synthetic | `ooxmlsdk-fonts` unit |
| right-align bidi run splitting | `../core/vcl/qa/cppunit/text.cxx::testImplLayoutArgsRightAlign` | synthetic | `ooxmlsdk-fonts` unit |
| script scanner embedded RTL/LTR cases, including Mongolian NNBSP ownership | `../core/i18nutil/qa/cppunit/test_scriptchangescanner.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| bidi mark run ownership | `../core/i18nutil/qa/cppunit/test_scriptchangescanner.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| manual kashida positions | `../core/i18nutil/qa/cppunit/test_kashida.cxx` | synthetic | `ooxmlsdk-fonts` unit |
| kashida justification data | `../core/vcl/qa/cppunit/justificationdata.cxx` | synthetic | `ooxmlsdk-fonts` unit |

## Font Files, Metrics, Coverage, and Features

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| TTF OS/2, head, and name-table parsing | `../core/vcl/qa/cppunit/font/TTFStructureTest.cxx::testReadTTFStructure` | `../core/vcl/qa/cppunit/font/data/Ahem.ttf` | test-suite fixture + `ooxmlsdk-fonts` unit |
| font metrics initialization | `../core/vcl/qa/cppunit/fontmetric.cxx` | fixture/system-dependent split | `ooxmlsdk-fonts` unit |
| charmap/coverage defaults | `../core/vcl/qa/cppunit/fontcharmap.cxx` | fixture/system-dependent split | `ooxmlsdk-fonts` unit |
| glyph bound rectangles | `../core/vcl/qa/cppunit/logicalfontinstance.cxx` | fixture/system-dependent split | `ooxmlsdk-fonts` unit |
| Ahem fixture metrics/coverage | `../core/vcl/qa/cppunit/font/data/Ahem.ttf` | `../core/vcl/qa/cppunit/font/data/Ahem.ttf` | test-suite fixture + `ooxmlsdk-fonts` unit |
| variable font axes | `../core/vcl/qa/cppunit/FontVariationTest.cxx` | `../core/vcl/qa/cppunit/data/Fraunces-VariableFont_opsz,wght.ttf` | test-suite fixture + `ooxmlsdk-fonts` unit |
| toolkit variable font service, variable/non-variable/unknown font | `../core/toolkit/qa/cppunit/FontVariations.cxx` | bundled-font dependent | `ooxmlsdk-fonts` unit |
| font variation parsing/equality/settings | `../core/vcl/qa/cppunit/complextext.cxx` | synthetic | `ooxmlsdk-fonts` unit + `ooxmlsdk-fonts-test` |
| Graphite font features | `../core/vcl/qa/cppunit/FontFeatureTest.cxx::testGetFontFeaturesGraphite` | fixture/system-dependent split | `ooxmlsdk-fonts` unit |
| OpenType font features and font-name feature parser | `../core/vcl/qa/cppunit/FontFeatureTest.cxx` | fixture/system-dependent split + synthetic parser cases | `ooxmlsdk-fonts` unit + `ooxmlsdk-fonts-test` |
| OpenType enum features | `../core/vcl/qa/cppunit/FontFeatureTest.cxx::testGetFontFeaturesOpenTypeEnum` | fixture/system-dependent split | `ooxmlsdk-fonts` unit |
| cached glyph behavior | `../core/vcl/qa/cppunit/complextext.cxx` | system-dependent | `ooxmlsdk-fonts` unit |

## CJK and Vertical Text

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| vertical CJK glyph sizing/orientation | `../core/vcl/qa/cppunit/cjktext.cxx::testVerticalText` | system CJK font dependent | `ooxmlsdk-fonts` + `ooxmlsdk-pdf-test` |
| Writer CJK import/layout regressions | `../core/sw/qa/extras/cjk/cjk.cxx` | `../core/sw/qa/extras/cjk/data/` | test-suite |
| underline trailing space in CJK document | `../core/sw/qa/extras/cjk/cjk.cxx::testMsWordUlTrailSpace` | `corpus/LibreOffice/sw/qa/extras/cjk/data/UnderlineTrailingSpace.docx` | test-suite + `ooxmlsdk-pdf-test` |
| WW8 CJK list font fixtures | `../core/sw/qa/extras/cjk/cjk.cxx` | `../core/sw/qa/extras/cjk/data/cjklist30.doc`, `cjklist31.doc`, `cjklist34.doc`, `cjklist35.doc` | test-suite |

## Embedded Fonts

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| subsetted embedded font unavailable for editing | `../core/sw/qa/writerfilter/dmapper/FontTable.cxx::testSubsettedEmbeddedFont` | `corpus/LibreOffice/sw/qa/writerfilter/dmapper/data/subsetted-embedded-font.docx` | test-suite + `ooxmlsdk-fonts` |
| subsetted full embedded font usable by coverage | `../core/sw/qa/writerfilter/dmapper/FontTable.cxx::testSubsettedFullEmbeddedFont` | `corpus/LibreOffice/sw/qa/writerfilter/dmapper/data/subsetted-full-embedded-font.docx` | test-suite + `ooxmlsdk-fonts` |
| generic font family import from DOCX font table | `../core/sw/qa/writerfilter/dmapper/FontTable.cxx::testFontFamily` | `corpus/LibreOffice/sw/qa/writerfilter/dmapper/data/font-family.docx` | test-suite |
| restricted embedded font open behavior | `../core/sw/qa/extras/embedded_fonts/embedded_fonts.cxx` | `corpus/LibreOffice/sw/qa/extras/embedded_fonts/data/embed-restricted+unrestricted.docx` | test-suite + `ooxmlsdk-fonts` |
| ODT embedded font properties | `../core/sw/qa/extras/embedded_fonts/embedded_fonts.cxx` | `../core/sw/qa/extras/embedded_fonts/data/embedded-font-props.odt` | test-suite |
| DOCX font embedding export behavior | `../core/sw/qa/extras/embedded_fonts/embedded_fonts.cxx::testFontEmbeddingDOCX` | generated/export | `ooxmlsdk-pdf` |
| PPTX embedded font roundtrip | `../core/sd/qa/unit/FontEmbeddingTest.cxx::testRoundtripEmbeddedFontsPPTX` | `corpus/LibreOffice/sd/qa/unit/data/BoldonseFontEmbedded.pptx` | test-suite |
| PPTX embedded font export behavior | `../core/sd/qa/unit/FontEmbeddingTest.cxx::testExportEmbeddedFontsPPTX` | generated/export | `ooxmlsdk-pdf` |

## DOCX Font Semantics

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| numbering font | `../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx::testNumberingFont` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/numbering-font.docx` | test-suite |
| symbol Chicago list | `../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx::testOoxmlSymbolChicagoList` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/symbol_chicago_list.docx` | test-suite |
| symbol content control / PUA symbol text ownership | `../core/sw/qa/extras/ooxmlexport/ooxmlexport27.cxx::testCool15788_symbolContentControl` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/Cool15788_symbolContentControl.docx` | `ooxmlsdk-fonts-test` for PUA fallback behavior; fixture import later in test-suite |
| group shape theme font | `../core/sw/qa/extras/ooxmlexport/ooxmlexport7.cxx::testGroupshapeThemeFont` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/groupshape-theme-font.docx` | test-suite |
| DML group shape run fonts | `../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx::testDMLGroupShapeRunFonts` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/dml-groupshape-runfonts.docx` | test-suite |
| empty font name | `../core/sw/qa/extras/ooxmlexport/ooxmlexport3.cxx::testFontNameIsEmpty` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/font-name-is-empty.docx` | test-suite |
| font type metadata | `../core/sw/qa/extras/ooxmlexport/ooxmlexport3.cxx::testFontTypes` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/tdf120344_FontTypes.docx` | test-suite |
| watermark font | `../core/sw/qa/extras/ooxmlexport/ooxmlexport2.cxx::testWatermarkFont` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/watermark-font.docx` | test-suite |
| no font defaults | `../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx::testTdf108350_noFontdefaults` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/tdf108350_noFontdefaults.docx` | test-suite |
| group shape font name import | `../core/sw/qa/extras/ooxmlimport/ooxmlimport2.cxx::testGroupShapeFontName` | `corpus/LibreOffice/sw/qa/extras/ooxmlimport/data/groupshape-fontname.docx` | test-suite |
| bidi import | `../core/sw/qa/extras/ooxmlimport/data/tdf87533_bidi.docx` | `corpus/LibreOffice/sw/qa/extras/ooxmlimport/data/tdf87533_bidi.docx` | test-suite |
| table style font inheritance | `../core/sw/qa/extras/ooxmlimport/data/tdf141969-font_in_table_with_style.docx` | `corpus/LibreOffice/sw/qa/extras/ooxmlimport/data/tdf141969-font_in_table_with_style.docx` | test-suite |
| fontRef shading override interaction | `../core/sw/qa/extras/ooxmlimport/data/tdf153791-shd_overrides_fontRef.docx` | `corpus/LibreOffice/sw/qa/extras/ooxmlimport/data/tdf153791-shd_overrides_fontRef.docx` | test-suite |
| CJK list font fixtures | `../core/sw/qa/extras/ooxmlexport/data/cjklist*.docx` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/cjklist30.docx`, `cjklist31.docx`, `cjklist34.docx`, `cjklist35.docx`, `cjklist44.docx` | test-suite |
| Arabic/Hebrew numbering | `../core/sw/qa/extras/ooxmlexport/data/tdf141231_arabicHebrewNumbering.docx` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/tdf141231_arabicHebrewNumbering.docx` | test-suite |
| font size around field separator | `../core/sw/qa/extras/ooxmlexport/data/fontsize-field-separator.docx` | `corpus/LibreOffice/sw/qa/extras/ooxmlexport/data/fontsize-field-separator.docx` | test-suite |
| RTF dispatch symbols | `../core/sw/qa/writerfilter/rtftok/rtfdispatchsymbol.cxx` | synthetic/fixture from test | test-suite |
| RTF font family import | `../core/sw/qa/writerfilter/dmapper/DomainMapper.cxx::testRTFFontFamily` | `../core/sw/qa/writerfilter/dmapper/data/font-family.rtf` | test-suite |
| RTF numbering font | `../core/sw/qa/extras/rtfexport/rtfexport7.cxx::testNumberingFont` | `../core/sw/qa/extras/rtfexport/data/numbering-font.rtf` | test-suite |
| RTF font override | `../core/sw/qa/extras/rtfexport/rtfexport5.cxx::testFontOverride` | `../core/sw/qa/extras/rtfexport/data/font-override.rtf` | test-suite |

## XLSX Font Semantics

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| textbox font size | `../core/sc/qa/unit/subsequent_export_test3.cxx::testFontSizeXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/fontSize.xlsx` | test-suite + `ooxmlsdk-pdf-test` |
| header font style | `../core/sc/qa/unit/subsequent_export_test4.cxx::testHeaderFontStyleXLSX` | fixture source in LO test | test-suite |
| font with charset | `../core/sc/qa/unit/data/xlsx/tdf122716_font_with_charset.xlsx` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tdf122716_font_with_charset.xlsx` | test-suite |
| SmartArt theme font/color | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf151818_SmartArtFontColor` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tdf151818_SmartartThemeFontColor.xlsx` | test-suite + `ooxmlsdk-pdf-test` |
| table style fonts | `../core/sc/qa/unit/ucalc_tablestyles.cxx::testTableStyleFonts` | synthetic/generated | test-suite |
| empty font name export | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf170189_empty_font_name` | fixture source in LO test | test-suite |
| default font height | `../core/sc/qa/unit/subsequent_export_test.cxx::testDefaultFontHeight` | fixture/generated | test-suite |
| font color with multiple attrs | `../core/sc/qa/unit/subsequent_export_test.cxx::testFontColorWithMultipleAttributesDefined` | fixture/generated | test-suite |

## PPTX / DrawingML Font Semantics

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| WordArt font theme | `../core/oox/qa/unit/drawingml.cxx::testTdf125085WordArtFontTheme` | `corpus/LibreOffice/oox/qa/unit/data/tdf125085_WordArtFontTheme.pptx` | test-suite |
| WordArt font text | `../core/oox/qa/unit/drawingml.cxx::testTdf125085WordArtFontText` | `corpus/LibreOffice/oox/qa/unit/data/tdf125085_WordArtFontText.pptx` | test-suite |
| theme font typeface | `../core/oox/qa/unit/export.cxx::testThemeFontTypeface` | generated/export | test-suite |
| font scale | `../core/sd/qa/unit/export-tests-ooxml3.cxx::testFontScale` | `corpus/LibreOffice/sd/qa/unit/data/pptx/font-scale.pptx` | test-suite + `ooxmlsdk-pdf-test` |
| SmartArt font size | `../core/sd/qa/unit/import-tests-smartart.cxx::testFontSize` | `corpus/LibreOffice/sd/qa/unit/data/pptx/smartart-font-size.pptx` | test-suite + `ooxmlsdk-pdf-test` |
| ActiveX font properties | `../core/sd/qa/unit/activex-controls-tests.cxx::testFontProperties` | `corpus/LibreOffice/sd/qa/unit/data/pptx/activex_fontproperties.pptx` | test-suite |
| bullet char and font | `../core/sd/qa/unit/export-tests-ooxml1.cxx::testBulletCharAndFont` | `../core/sd/qa/unit/data/odp/bulletCharAndFont.odp` | test-suite |
| Fontwork font properties | `../core/oox/qa/unit/export.cxx::testFontworkFontProperties` | `../core/oox/qa/unit/data/tdf128568_FontworkFontProperties.odt` | test-suite |
| Fontwork scale X | `../core/sd/qa/unit/export-tests-ooxml3.cxx::testTdf125573_FontWorkScaleX` | `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf125573_FontWorkScaleX.pptx` | test-suite |

## PDF Font Output

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| variable font PostScript name 1 | `../core/vcl/qa/cppunit/pdfexport/pdfexport.cxx::testVariableFontPSName1` | `../core/vcl/qa/cppunit/pdfexport/data/variable-font-psname-1.odt` | `ooxmlsdk-pdf-test` |
| variable font PostScript name 2 | `../core/vcl/qa/cppunit/pdfexport/pdfexport.cxx::testVariableFontPSName2` | `../core/vcl/qa/cppunit/pdfexport/data/variable-font-psname-2.odt` | `ooxmlsdk-pdf-test` |
| font variation settings ODT | `../core/vcl/qa/cppunit/pdfexport/pdfexport.cxx::testFontVariationSettingsODT` | `../core/vcl/qa/cppunit/pdfexport/data/testFontVariationSettings.odt` | `ooxmlsdk-pdf-test` |
| font variation settings ODP | `../core/vcl/qa/cppunit/pdfexport/pdfexport.cxx::testFontVariationSettingsODP` | `../core/vcl/qa/cppunit/pdfexport/data/testFontVariationSettings.odp` | `ooxmlsdk-pdf-test` |
| form font name | `../core/vcl/qa/cppunit/pdfexport/pdfexport2.cxx::testFormFontName` | `../core/vcl/qa/cppunit/pdfexport/data/form-font-name.odt` | `ooxmlsdk-pdf-test` |
| justified Arabic kashida | `../core/vcl/qa/cppunit/pdfexport/data/justified-arabic-kashida.odt` | `../core/vcl/qa/cppunit/pdfexport/data/justified-arabic-kashida.odt` | `ooxmlsdk-pdf-test` |
| kashida space expansion | `../core/vcl/qa/cppunit/pdfexport/pdfexport2.cxx::testTdf151748KashidaSpace` | fixture source in LO test | `ooxmlsdk-pdf-test` |
| underline kashida portion | `../core/vcl/qa/cppunit/pdfexport/pdfexport2.cxx::testTdf155557UnderlineKashidaPortion` | fixture source in LO test | `ooxmlsdk-pdf-test` |

## Non-OOXML Font Import References

| Coverage | Source Test | Fixture | Target |
| --- | --- | --- | --- |
| SVG font variation settings | `../core/svgio/qa/cppunit/SvgImportTest.cxx` | `../core/svgio/qa/cppunit/data/font-variation-settings.svg` | `ooxmlsdk-fonts` parser |
| SVG font-size parsing: percentage, keywords, relative | `../core/svgio/qa/cppunit/SvgImportTest.cxx` | `../core/svgio/qa/cppunit/data/FontsizePercentage.svg`, `FontsizeKeywords.svg`, `FontsizeRelative.svg` | parser |
| SVG font-family quoting/apostrophes | `../core/svgio/qa/cppunit/SvgImportTest.cxx` | `../core/svgio/qa/cppunit/data/FontFamilyIncludingApostrophes.svg` | parser |
| SVG bidi/RTL text | `../core/svgio/qa/cppunit/SvgImportTest.cxx` | `../core/svgio/qa/cppunit/data/BiDitext.svg`, `RTLtext.svg` | bidi |
| StarMath font style import | `../core/starmath/qa/cppunit/test_import.cxx::testFontStyles` | `../core/starmath/qa/cppunit/data/font-styles.odf` | formula/math |
| EPUB font embedding | `../core/writerperfect/qa/unit/data/writer/epubexport/font-embedding.fodt` | `../core/writerperfect/qa/unit/data/writer/epubexport/font-embedding.fodt` | test-suite |

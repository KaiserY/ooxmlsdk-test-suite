# LibreOffice Layout Test Migration Index

This document is the migration index for LibreOffice layout coverage.
Use it as the checklist for translating layout assertions into
`ooxmlsdk-layout` and this test-suite.

The source of truth is the local LibreOffice checkout at `../core`. Existing
`ooxmlsdk-pdf-test` coverage is migration evidence, not the authority: when the
PDF matrix and LibreOffice source disagree, keep the LibreOffice source
semantics and adjust the layout test plan.

## Goal

Move source OOXML -> layout assertions out of the PDF-focused test lane and into
an `ooxmlsdk-layout-test` lane backed by `common::LayoutDocument`.

The layout-test lane should assert layout facts before PDF serialization:

- page count, page size, section/page indexes, margins, headers, footers, and
  page background
- text presence, ordering, origin, line height, text style, bidi state,
  field-generated text, and text overflow/visibility
- table row/cell/frame bounds, frame fragments, split/follow behavior, repeated
  headers, columns, and footnote/endnote placement
- image, shape, path, rectangle, line, fill, stroke, crop, rotation, flip,
  floating, and behind-text behavior
- link areas, form widgets, outline entries, and other layout-time output that
  later becomes PDF annotations/bookmarks/widgets
- reflow diagnostics when LibreOffice asserts move-back, replay, invalidation,
  or follow-frame behavior

PDF object serialization stays in `ooxmlsdk-pdf-test`: raw PDF dictionaries,
XObjects, font embedding resources, PDF tags/UA structure, annotations as PDF
objects, PDF bookmarks as catalog structures, final content streams, and
PDFium/lopdf extraction behavior.

## Migration Rule

A row counts as migrated only when a Rust test loads a source OOXML fixture,
builds `common::LayoutDocument`, and asserts a LibreOffice-backed layout fact.

Valid source evidence:

- LibreOffice layout dump assertions from `parseLayoutDump()`
- Writer layout frame/portion assertions such as `SwParaPortion`,
  `SwLineLayout`, frame bounds, row/cell bounds, follow frames, page count, and
  cursor-independent layout geometry
- Impress/Draw layout summaries, shape geometry, text box layout, SmartArt
  geometry, bullet metrics, slide/notes/master layout facts
- Calc print-layout facts: page size, pagination, row/column sizing, text
  overflow/wrap, print range, sheet shape anchors, visible grid/border/image/text
  placement
- existing `ooxmlsdk-pdf-test` assertions that are really layout facts projected
  through PDF output

Do not count these as layout migration:

- pure PDF object assertions without an observable layout precursor
- raw OOXML export XML assertions, unless the asserted XML behavior has a
  concrete layout consequence
- crash-only import/export tests with no layout fact
- editing, cursor, undo/redo, selection, command dispatch, UI, tiled-rendering
  invalidation, or UNO workflow tests unless they also provide a static source
  OOXML -> layout assertion
- ODS/XLS/XLSB-only tests for this first OOXML layout lane

## Current Baseline

Existing layout-relevant coverage is split:

| Existing lane | Current role | Migration action |
| --- | --- | --- |
| `crates/ooxmlsdk-pdf-test/tests/mapped_docx_pdf_fixtures.rs` | Large DOCX visible-output lane with many Writer layout projections and a few direct `DocxLayoutSummary` assertions. | Convert layout facts to `common::LayoutDocument`; keep final-PDF object tests in PDF lane. |
| `crates/ooxmlsdk-pdf-test/tests/mapped_pptx_pdf_fixtures.rs` | PPTX visible-output lane with many Impress shape/text/SmartArt/layout projections and some `PptxLayoutSummary` assertions. | Convert slide geometry/text/layout facts to `common::LayoutDocument`; keep PDF extraction tests where they assert PDF serialization. |
| `crates/ooxmlsdk-pdf-test/tests/mapped_xlsx_pdf_fixtures.rs` | XLSX visible-output lane for Calc print output, formulas-as-rendered-values, pagination, row heights, images, links, pivots, and conditional formatting candidates. | Convert print-layout facts to `common::LayoutDocument`; formula/value-only cases remain formula or PDF-visible output evidence. |
| `crates/ooxmlsdk-pdf-test/tests/pdfexport_fixtures.rs` | Direct LibreOffice PDF export/object tests. | Keep PDF-object rows in PDF lane; mirror only layout precursors such as link area/form widget/outline creation when useful. |
| `docs/ooxmlsdk-pdf-test/LibreOffice.md` | Current PDF migration index. | Keep reconciled with this document so PDF-only and layout-owned rows stay separate. |

Recent verification:

| Package | Result |
| --- | --- |
| `cargo test -p ooxmlsdk-layout` | 72 tests passed |
| `cargo test -p ooxmlsdk-layout-test` | layout split passed |
| `cargo test -p ooxmlsdk-pdf-test` | 698 PDF tests passed |

The test-suite workspace already depends on `ooxmlsdk-layout`, but it does not
yet have an `ooxmlsdk-layout-test` crate. The first implementation pass should
add that crate instead of placing large layout coverage in the main repository.

## `common::LayoutDocument` Coverage Contract

Use `common::LayoutDocument` as the only target model for new layout tests.
Do not assert through legacy `compat::LayoutDocument` except temporarily inside
helpers while the public test API is being stabilized.

The current common model is sufficient for the first large migration batch:

| LibreOffice assertion family | `common::LayoutDocument` target |
| --- | --- |
| page count/size/margins/sections | `pages`, `DisplayPage::setup`, `DisplayPage::bounds`, page indexes |
| text content/order/position | `DisplayItem::Text`, `TextRun::text`, `origin`, `line_height`, `source` |
| character/text styling | `TextRun::style`, `font_id`, `color`, `paragraph_bidi`, `preserve_text_portion` |
| tables/paragraph frames | `frames`, `FrameRecord::kind`, `bounds`, `print_bounds`, `lines`, `fragments` |
| row/cell split and repeated headers | `FrameFragment`, `FragmentSplitKind`, `FrameFollow`, `ItemRange` |
| footnotes/endnotes/columns | frame kind, page/section/column indexes, fragments, follows, line boxes |
| shapes and borders | `DisplayItem::Path`, `Rect`, `Line`, fill/stroke/bounds/points/line kind |
| images | `DisplayItem::Image`, bounds, crop, rotation, flip, floating, behind-text |
| links/widgets/outlines | `LinkArea`, `form_widgets`, `outline_entries` |
| reflow/replay diagnostics | `ReflowDiagnostics`, `ReflowRequest`, `BackwardMove`, `LayoutRerun`, `RestartPlan` |

Known limits are not blockers for starting migration:

- LibreOffice layout dump XML node paths are not copied literally. Preserve the
  asserted page/frame/text/path relation in common layout terms.
- The current public `layout_document()` entry points use a compat bridge. That
  is acceptable for migrating tests; engine-native zero-copy common output can
  replace the bridge later without changing test assertions.
- If a future LO row needs exact internal dump metadata that common layout does
  not expose, add a narrow debug/frame field. Do not reintroduce a separate
  legacy assertion model.

## Source Matrix

### Writer Layout Core

These are the primary Writer sources. They should be migrated before broad
OOXML import/export rows because they directly exercise Writer layout behavior.

| Source | Fixture root | Migration status | Notes |
| --- | --- | --- | --- |
| `../core/sw/qa/core/layout/*.cxx` | `corpus/LibreOffice/sw/qa/core/layout/data/` | first priority | Page, frame, table, border, floating table, footnote/endnote, follow/reflow, and paint-frame cases. |
| `../core/sw/qa/core/text/*.cxx` | `corpus/LibreOffice/sw/qa/core/text/data/` | first priority where static DOCX -> layout | Text portions, content controls, redline rendering, field portions, line boxes. |
| `../core/sw/qa/core/objectpositioning/*.cxx` | `corpus/LibreOffice/sw/qa/core/objectpositioning/data/` | first priority for static fixtures | Anchor/relative positioning rows; exclude rows that create the asserted object only through UNO after load. |
| `../core/sw/qa/core/txtnode/*.cxx` | mixed Writer fixture roots | second priority | Include split/fly anchor layout rows; exclude model-only text-node assertions. |

High-value first-batch groups:

| Group | Fixture examples | Assertion target |
| --- | --- | --- |
| top margin/page style | `ignore-top-margin*.docx` | first text/table frame y-position and page setup |
| table/fly overlap | `table-fly-overlap*.docx` | frame overlap/spacing and item bounds |
| table borders | `border-collapse-compat.docx`, `double-border-*.docx`, `inner-border.docx`, `vmerge-cell-border.docx` | path/line count, stroke geometry, table fragments |
| endnotes/footnotes | `inline-endnote-position.docx`, `endnote-cont-separator.docx` | note frame position and separator path |
| floating tables | `floattable*.docx` | frame kind, follow/split, row/cell fragments, page/column indexes |
| redline layout output | `redline*.docx` | visible/deleted/inserted text and decoration path output in default render mode |

### Writer Extras Layout

| Source | Fixture root | Migration status | Notes |
| --- | --- | --- | --- |
| `../core/sw/qa/extras/layout/layout*.cxx` | `corpus/LibreOffice/sw/qa/extras/layout/data/` | first priority after core layout | Dense layout-regression source; many rows are already visible-output covered in `ooxmlsdk-pdf-test`. |
| `../core/sw/qa/extras/tiledrendering/*.cxx` | `corpus/LibreOffice/sw/qa/extras/tiledrendering/data/` | selective | Include static bitmap/render facts only; exclude view invalidation/editor/UI rows. |
| `../core/sw/qa/extras/uiwriter/*.cxx` | `corpus/LibreOffice/sw/qa/extras/uiwriter/data/` | selective | Include source-backed static layout facts such as bookmarks/split floating table; exclude editor workflow. |

High-value first-batch groups:

| Group | Fixture examples | Assertion target |
| --- | --- | --- |
| row height at least | `tdf155229_row_height_at_least.docx`, `tdf164907_rowHeightAtLeast.docx` | row/cell fragment bounds and table bottom |
| chart/OLE layout | `legend-itemorder-min.docx`, `long_legendentry.docx`, `tdf138465min.docx` | text order, image/object bounds |
| split floating tables | `tdf170381-split-float-table-*.docx`, `tdf81100.docx` | repeated/split frame fragments and page rows |
| line height | `tdf153136.docx`, `tdf153128.docx` | line box heights and row heights |
| hidden/redline paragraphs | `hidden-para-separator.docx`, `CT-formatted-deletion.docx`, `tdf104797.docx` | visible text, decoration paths, frame lines |

### Writer OOXML Import/Export Layout Rows

`sw/qa/extras/ooxmlimport` and `sw/qa/extras/ooxmlexport` are broader than
layout. Migrate only rows with static layout consequences.

| Source | Migration status | Include when |
| --- | --- | --- |
| `../core/sw/qa/extras/ooxmlimport/*.cxx` | selective | The test asserts layout dump, visible position, page count, table/fly shape, or rendered text. |
| `../core/sw/qa/extras/ooxmlexport/*.cxx` | selective | The OOXML export assertion corresponds to a source fixture layout fact already visible before export. |
| `../core/sw/qa/writerfilter/dmapper/*.cxx` | selective | The import property feeds rendered layout or text effect output. |
| `../core/oox/qa/unit/*.cxx` | selective | DrawingML/VML/shape rows that become Writer layout shapes, paths, text, colors, or images. |

Examples already represented in PDF tests and should be promoted to layout
assertions where possible:

| Area | Fixture examples | Layout assertion target |
| --- | --- | --- |
| `layoutInCell` | `tdf160077_layoutInCell*.docx`, `tdf37153.docx`, `tdf128646.docx` | image/shape/text relative position inside or outside table cell flow |
| shape anchors and relative sizes | `tdf105143.docx`, `tdf114212.docx`, `tdf167770_marginInsideOutside.docx` | frame/path bounds relative to margin/page/cell |
| VML/DML shapes | `vml-vertical-alignment.docx`, `tdf112450_vml_polyline.docx`, `dml-*.docx` | path points, line bounds, text box bounds |
| page backgrounds/images | `tdf126533_pageBitmap.docx`, `i120928.docx` | page background/image items |
| numbering/table layout | `testGridBefore`, `tdf81100.docx`, `tdf58944RepeatingTableHeader` | table row/cell fragments and numbering text |

### Impress / Draw / PowerPoint Layout

The PPTX PDF lane already contains many layout-like assertions. Move geometry
and text-layout assertions to `common::LayoutDocument`; keep PDF object/raster
checks in PDF.

| Source | Fixture root | Migration status | Notes |
| --- | --- | --- | --- |
| `../core/sd/qa/unit/layout-tests.cxx` | `corpus/LibreOffice/sd/qa/unit/data/` | first priority for Impress-native layout | Dedicated Impress layout source. |
| `../core/sd/qa/unit/import-tests*.cxx` | `corpus/LibreOffice/sd/qa/unit/data/pptx/` | first priority where OOXML and layout-visible | Shape geometry, text boxes, tables, crop, slide masters/layouts, bullet metrics. |
| `../core/sd/qa/unit/import-tests-smartart.cxx` | `corpus/LibreOffice/sd/qa/unit/data/pptx/` | first priority | SmartArt geometry/text layout rows. |
| `../core/sd/qa/unit/export-tests-ooxml*.cxx` | `corpus/LibreOffice/sd/qa/unit/data/pptx/` | selective | Include imported/rendered layout facts; exclude export XML-only rows. |
| `../core/oox/qa/unit/*.cxx` | `corpus/LibreOffice/oox/qa/unit/data/` | selective | DrawingML/shape/SmartArt rows that render through PPTX layout. |

High-value PPTX groups:

| Group | Fixture examples | Assertion target |
| --- | --- | --- |
| table row heights | `n80340.pptx`, `tablescale.pptx`, `tdf93830.pptx` | table row bounds/fragments |
| SmartArt layout | `smartart-*.pptx`, `tdf145528_SmartArt_Matrix.pptx`, `tdf134221.pptx` | shape tree geometry and text positions |
| bullet graphics | `tdf90626.pptx`, `tdf138148.pptx`, `tdf114913.pptx` | graphic bullet size and text association |
| masters/layouts/notes | `onemaster-twolayouts.pptx`, `multiplelayoutfooter.pptx`, `tdf142913.pptx` | page names, master/layout inheritance, notes page shape counts |
| crop/rotation/flip | `PptCrop.pptx`, mirrored/crop fixtures | image crop, rotation, flip flags and bounds |

### Calc / SpreadsheetML Print Layout

XLSX layout tests are about the printed sheet, not cell-model import alone.
Formula and value correctness should stay in `ooxmlsdk-formula-test` unless the
assertion is explicitly about printed output.

| Source | Fixture root | Migration status | Notes |
| --- | --- | --- | --- |
| `../core/sc/qa/extras/scpdfexport.cxx` | mixed Calc data roots | selective | Direct Calc PDF/export rows; include only XLSX/static-print cases with visible layout facts. |
| `../core/sc/qa/unit/subsequent_filters_test*.cxx` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/` | selective | Include row/column sizing, wrap, page size, print range, images/shapes, links, grid/border/color output. |
| `../core/sc/qa/unit/subsequent_export_test*.cxx` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/` | selective | Include imported layout-visible output, not export XML-only rows. |
| `../core/sc/qa/unit/pivottable*.cxx` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/` | selective | Include printed pivot layout rows; model-only pivot cache assertions stay out. |
| `../core/sc/qa/unit/cond_format.cxx` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/` | review | Promote only when the assertion is rendered fill/text/color in printed output. |

High-value XLSX groups:

| Group | Fixture examples | Assertion target |
| --- | --- | --- |
| row height / pagination | `tdf123026_optimalRowHeight.xlsx`, `tdf159581_optimalRowHeight.xlsx`, `tdf144642_RowHeight*.xlsx`, `tdf145129_DefaultRowHeight*.xlsx` | page count, row/text bounds, wrapped text visibility |
| visible cell formatting | `cell-borders.xlsx`, `fontSize.xlsx`, `TextColor.xlsx`, `underlineColor.xlsx` | text style, border paths, fill/stroke colors |
| images/shapes/anchors | `image_hyperlink.xlsx`, `hiddenShape.xlsx`, `tdf169496_hidden_graphic.xlsx`, `tdf166724_cellAnchor.xlsx` | image/shape/link bounds and visibility |
| print/page setup | `tdf136721_letter_sized_paper.xlsx`, print-range fixtures | page setup, page count, sheet content bounds |
| pivots/tables | `pivot*.xlsx`, pivot-table format fixtures | printed pivot text order, row/column layout, colors when visible |

### Direct PDF Export Sources

These rows remain in `ooxmlsdk-pdf-test` unless a layout precursor is useful.

| Source | Keep in PDF | Optional layout precursor |
| --- | --- | --- |
| `../core/vcl/qa/cppunit/pdfexport/*.cxx` | raw PDF object counts, XObjects, annotations, widgets, outlines, tagged PDF, PDF/UA | link areas, form widgets, outline entries, image layout |
| `../core/svx/qa/unit/svdraw.cxx` | PDF page object counts and draw-layer clipping as final PDF output | page item counts and clipped shape bounds |
| PDFium/lopdf extraction helper assertions | PDF parser/export correctness | none |

## First Migration Batch

Start with rows already present in `ooxmlsdk-pdf-test`, because their fixtures
and upstream source comments are known-good. The first batch should avoid
fixing layout bugs; failing assertions should stay active unless the failure is
a test-harness mistake.

1. Add `crates/ooxmlsdk-layout-test` in the test-suite workspace.
2. Add shared helpers for loading DOCX/PPTX/XLSX from
   `corpus/LibreOffice/**`, producing `common::LayoutDocument`, and querying
   pages/text/items/frames with tolerance helpers.
3. Port direct summary tests first. Prefer `common::LayoutDocument` for row
   heights, line heights, frame/page counts, visible page content, and
   debug-only LibreOffice-backed metadata such as master/notes shape records,
   SmartArt text anchors, per-shape text insets, geometry preset names, or
   graphic bullet dimensions.
4. Port PDF tests whose assertion is actually a layout fact:
   page count, text order/position, path/image counts when they reflect layout
   primitives, table row/cell geometry, and image/shape visibility.
5. Leave PDF-object tests in `ooxmlsdk-pdf-test` and add optional paired layout
   tests only for link area, form widget, outline, and image precursor behavior.

Initial priority list:

| Priority | Group | Source evidence | Expected outcome |
| --- | --- | --- | --- |
| P0 | test harness and fixtures | current PDF tests + `corpus/LibreOffice` | Layout crate compiles and loads DOCX/PPTX/XLSX fixtures. |
| P1 | existing summary tests | former `layout_summary()` / `pptx_layout_summary()` call sites | Migrate assertions to `common::LayoutDocument`, using `debug_records` for metadata that is diagnostic rather than paint output. |
| P2 | Writer core layout rows | `sw/qa/core/layout`, `sw/qa/core/text`, `sw/qa/core/objectpositioning` | Cover page/frame/text/table/floating/footnote/border basics. |
| P3 | Writer extras layout rows | `sw/qa/extras/layout/layout*.cxx` | Cover dense layout regressions and split/reflow cases. |
| P4 | PPTX layout rows | `sd/qa/unit/layout-tests.cxx`, `import-tests*.cxx`, SmartArt tests | Cover slide/table/shape/text/SmartArt layout. |
| P5 | XLSX print-layout rows | `scpdfexport.cxx`, selected `sc/qa/unit` XLSX rows | Cover printed sheet layout, row heights, anchors, links, formatting. |

## Gap Audit Against Existing PDF Matrix

The PDF matrix already marks many rows as `covered`, but layout migration needs
finer ownership:

| PDF matrix class | Layout-test handling |
| --- | --- |
| direct PDF/object rows | keep in PDF; add layout precursor only if common layout exposes it |
| layout dump projection rows | migrate to layout-test; assert common layout directly |
| raster/bitmap rows | migrate only if the underlying primitive is in common layout; otherwise keep PDF/raster |
| metafile/render XML rows | migrate path/text/shape primitive assertions to common layout |
| graphics/color/effects rows | migrate simple fill/stroke/text/image facts; keep complex final-render effects in PDF/raster |
| other visible output rows | decide per row; printed text/value assertions may stay PDF if layout does not own the value |

Known PDF-test gaps to fix during migration:

| Gap | Action |
| --- | --- |
| PDF matrix uses PDF-visible projection wording for many LO layout dump rows. | Replace with direct common-layout assertion wording in this document and future tests. |
| Some rows are value/import correctness rather than layout, especially XLSX formula/pivot/cache cases. | Keep them out of layout-test unless the printed position/page/formatting is the asserted behavior. |
| Direct PDF annotations/widgets/bookmarks are final serialization checks. | Keep PDF assertions; add layout precursor checks for `LinkArea`, `FormWidget`, and `OutlineEntry` when source-backed. |
| LO layout dump has internal XPath node shape not modeled by common layout. | Assert equivalent page/frame/text/path fact; add debug records only for unavoidable internal diagnostics. |
| Focused PPTX rows need master/notes shape records, SmartArt text anchor geometry, per-shape text inset diagnostics, preset geometry names, and graphic bullet dimensions. | Assert these through `common::LayoutDocument.debug_records`; promote only stable paint-facing data into normal display items. |
| Existing PDF tests do not fully cover `sw/qa/core/layout/data/floattable*.docx`. | Add explicit layout-test rows for floating table split/follow/reflow behavior. |
| Existing PDF tests only selectively cover Impress `sd_layout_tests`. | Add P4 scan and migrate static PPTX/ODP-equivalent OOXML rows first. |
| Existing PDF tests review some Calc conditional-format rows. | Promote only after rendered cell fill/text color can be asserted through layout or PDF raster. |

## Fixture Boundary

Layout fixtures belong in `../ooxmlsdk-test-suite/corpus/LibreOffice` when they
come from LibreOffice. Do not copy those fixtures into the main repository.

Rules:

- use existing corpus paths whenever present
- add missing LibreOffice fixtures under the same relative path as `../core`
- keep license attribution under `licenses/LibreOffice/`
- add `// Source:` comments in Rust tests with the exact LibreOffice source file
  and test name
- do not add derived expected values from current Rust output; expected values
  must come from LibreOffice assertions, reference output, or fixture evidence

## Status Legend

| Status | Meaning |
| --- | --- |
| `migrate-now` | Static OOXML -> layout assertion, common layout can represent it now. |
| `pdf-only` | Final PDF serialization/object assertion; keep in `ooxmlsdk-pdf-test`. |
| `review` | Likely layout-visible, but source assertion needs item-level review before adding an active test. |
| `blocked-api` | Good layout target, but common layout lacks a required narrow field. Add the field before migrating. |
| `deferred` | Editing/UI/ODS/XLS/XLSB/export-XML/crash-only/model-only behavior for the current layout lane. |

## Next Checklist

- [x] Create `crates/ooxmlsdk-layout-test`.
- [x] Add fixture loading helpers for DOCX/PPTX/XLSX common layout.
- [x] Port current DOCX summary assertions to `common::LayoutDocument`.
- [x] Move focused PPTX metadata assertions to `common::LayoutDocument.debug_records`.
- [x] Port Writer core layout P2 rows.
- [x] Port Writer extras layout P3 rows.
- [x] Port PPTX P4 rows.
- [x] Port XLSX print-layout P5 rows.
- [ ] Reconcile this document with the PDF matrix after each batch so PDF-only
      and layout-test ownership stay separate.

Current migrated mapped coverage:

- DOCX: 265 active mapped layout cases plus 5 focused DOCX layout tests.
- PPTX: 135 active mapped layout cases plus 17 focused PPTX debug metadata
  tests.
- XLSX: 187 active mapped layout cases.

Current verification status:

- `cargo test -p ooxmlsdk-layout`: passed in the main repository.
- `cargo test -p ooxmlsdk-layout-test`: passed.
- `cargo test -p ooxmlsdk-pdf-test`: passed after PDF/layout ownership split.
- `cargo test -p ooxmlsdk-layout-test --test pptx_layout`: passed with
  focused PPTX metadata assertions backed by `common::LayoutDocument.debug_records`.
- `cargo test -p ooxmlsdk-layout-test --test mapped_docx_layout`: passed.
- `cargo test -p ooxmlsdk-layout-test --test mapped_pptx_layout`: passed.
- `cargo test -p ooxmlsdk-layout-test --test xlsx_layout`: passed.
- `// Source: ../core/...` references in layout/PDF tests resolve to existing
  LibreOffice source files.

Rows intentionally left in `ooxmlsdk-pdf-test` are final PDF serialization,
raw XObject, PDF pixel/raster, or color-effect assertions where common layout
does not expose the final rendered evidence. Mixed PDF tests that also contain
layout assertions should be split gradually: move the layout assertion to this
crate and leave only the final PDF-visible/object assertion in `ooxmlsdk-pdf-test`.

Rows deferred after source re-check:

- `sd/qa/unit/import-tests3.cxx:testTdf131553`: verifies that the SmartArt child
  object imports as OLE2; not a rendered slide layout assertion.
- `sc/qa/unit/subsequent_filters_test4.cxx:testControlImport`: UNO control
  import smoke; not a printed layout assertion.
- `sc/qa/unit/subsequent_export_test2.cxx:testTdf120168`: verifies exported
  styles.xml alignment and sheetFormatPr customHeight; not a printed layout
  assertion.
- `sc/qa/unit/PivotTableFormatsImportExport.cxx`: compares pivot output cell
  patterns against reference ranges. These rows need cell-level style
  diagnostics before they can be represented as layout tests.

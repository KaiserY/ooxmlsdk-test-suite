# LibreOffice PDF Test Migration Index

This document is the migration index for LibreOffice-backed PDF export
coverage. Use it as the checklist for moving `ooxmlsdk-pdf-test` out of the
main repository and into this test-suite.

The source of truth is the local LibreOffice checkout at `../core`. The old
main-repository PDF test crate and matrix were migration evidence, not
authority. When they disagree with LibreOffice source semantics, keep the
LibreOffice semantics and reclassify the Rust test.

## Goal

Move LibreOffice-derived source document -> exported PDF assertions into an
`ooxmlsdk-pdf-test` lane in this test-suite.

This lane owns final PDF behavior:

- PDF page/object serialization: raw page dictionaries, catalog structures,
  outlines, bookmarks, forms, widgets, annotations, actions, links, XObjects,
  images, fonts, tagged PDF, PDF/UA structures, and content streams
- PDF-visible extracted primitives: text objects, font size, render mode,
  fill/stroke color, alpha, image dimensions, path fill/stroke, and PDFium or
  lopdf extraction summaries
- final rendered PDF rasters: page snapshots, region pixels, color ratios,
  image pixels, deterministic raster hashes, and visual output checks whose
  expected values come from LibreOffice evidence
- PDF export options that change final PDF output, such as image compression,
  page range/range selection, form export, tagged PDF, PDF/UA, or
  single-page-sheet export

This lane must not become a second layout suite. Layout assertions belong in
`docs/ooxmlsdk-layout-test/LibreOffice.md` and the `ooxmlsdk-layout-test`
crate, even if an old main-repository test currently reaches them through PDF.

## Migration Rule

A row counts as migrated only when a Rust test loads a LibreOffice-derived
fixture, exports it through `ooxmlsdk-pdf`, and asserts a final PDF fact backed
by LibreOffice source, LibreOffice expected output, or checked fixture
evidence.

Valid PDF evidence:

- LibreOffice tests that call `parsePDFExport()`, PDFium, PDF object parsers,
  or raw PDF stream/dictionary checks
- LibreOffice tests that export to PDF and assert page count, text objects,
  annotations, links, widgets, bookmarks, XObjects, images, font data, tags, or
  content stream structure
- LibreOffice bitmap/tiled-rendering assertions when the local Rust assertion
  is final exported PDF raster output, not an intermediate layout dump
- existing main-repository `ooxmlsdk-pdf-test` assertions that already inspect
  final PDF text/path/image/raw object/raster output

Do not count these as PDF migration:

- `common::LayoutDocument`, legacy `DocxLayoutSummary`, `PptxLayoutSummary`,
  page/frame/row/cell/shape geometry, line boxes, text ordering, page setup, or
  other pre-PDF layout facts
- source OOXML import/export XML, package part containment, relationship XML,
  model property, UNO property, or round-trip assertions unless the same Rust
  test also asserts final PDF behavior
- formula/parser/evaluator correctness, except the final printed value as
  visible PDF text or raster output
- editor workflow, cursor, undo/redo, command dispatch, tiled invalidation,
  UI, or UNO mutation mechanics
- ODS/XLS/XLSB coverage for this first OOXML PDF lane unless a dedicated
  fixture reader and source-backed final PDF assertion already exist

Mixed old tests must be split. Keep the PDF object/raster assertion here and
move the layout assertion to `ooxmlsdk-layout-test`.

## Current Baseline

The first migration batch came from the former main-repository staging area. It
contained valuable PDF-specific tests mixed with layout projections that should
not remain in the PDF lane.

| Former staging file | Migrated role | Migration action |
| --- | --- | --- |
| `crates/ooxmlsdk-pdf-test/tests/pdfexport_fixtures.rs` | Direct LibreOffice PDF export/object assertions. | Migrate first; this is the cleanest PDF lane. |
| `crates/ooxmlsdk-pdf-test/tests/core_docx_pdf_fixtures.rs` | Supplemental DOCX PDF-visible color/alpha assertions from Writer/Oox sources. | Migrate with PDF-visible text/path color extraction. |
| `crates/ooxmlsdk-pdf-test/tests/mapped_docx_pdf_fixtures.rs` | Large mixed DOCX visible-output lane: many layout projections plus real PDF color/image/path/raster checks. | Split by assertion. Layout facts go to layout-test; final PDF text/path/image/raster checks stay here. |
| `crates/ooxmlsdk-pdf-test/tests/mapped_pptx_pdf_fixtures.rs` | Mixed PPTX lane: Impress layout/shape summaries plus PDF links, text colors, paths, images, and raster checks. | Split by assertion. Keep PDF links/annotations/raster/text/path checks only. |
| `crates/ooxmlsdk-pdf-test/tests/mapped_xlsx_pdf_fixtures.rs` | Mixed XLSX printed-output lane: Calc print layout plus links, PDF text style, image/raster checks. | Split by assertion. Page/row/cell print layout goes to layout-test; PDF annotations/text objects/raster checks stay here. |

The former main-repository helper crate exported both PDF extraction helpers
and layout summaries. The test-suite PDF crate intentionally exposes only PDF
observation APIs. If a migrated PDF test needs a layout precursor, add or reuse
the corresponding layout-test assertion separately.

## Assertion Ownership

| Assertion class | Owner | Notes |
| --- | --- | --- |
| raw PDF page/catalog/object dictionaries | PDF test | Use lopdf-style extraction and source-backed expected keys/counts. |
| annotations, link actions, widget forms, popups | PDF test | Link/form layout precursors may also have layout-test coverage, but final PDF object correctness lives here. |
| bookmarks/outlines/destinations | PDF test | Assert catalog/outline order or destination data from exported PDF. |
| XObjects, image codecs, image dimensions, bpp | PDF test | Includes compression/export-option rows. |
| text object font size, fill color, alpha, render mode | PDF test | This is final PDF extraction, not a layout text-run assertion. |
| path fill/stroke color, alpha, clipping, painted primitive count | PDF test when asserted from exported PDF; layout-test when asserted from layout document | Prefer PDF here only when the final content stream/raster is the behavior under test. |
| rendered page pixels, color ratios, image pixels, snapshots | PDF test | Expected values must come from LibreOffice output or upstream assertions, not current Rust output. |
| page count, page size, row/cell/frame bounds, text order, shape bounds | layout-test by default | Keep in PDF only when LibreOffice is explicitly testing exported PDF page/object output. |
| package XML/source part containment | package/round-trip lanes | Not PDF unless paired with final PDF evidence. |
| formula value correctness | formula-test | PDF may only assert the visible printed/exported result. |

## Source Map

Primary LibreOffice sources for this PDF migration:

| Source | Fixture root | Migration status | Notes |
| --- | --- | --- | --- |
| `../core/vcl/qa/cppunit/pdfexport/*.cxx` | `corpus/LibreOffice/vcl/qa/cppunit/pdfexport/data/` | first priority | Direct PDF export rows: PDF dictionaries, annotations, links, forms, images, outlines, tags, compression, and content streams. |
| `../core/sc/qa/extras/scpdfexport.cxx` | mixed Calc roots | first priority for PDF-specific rows | Include PDF page/object/text/tag/link/range/export-option checks. Move pure print layout to layout-test. |
| `../core/svx/qa/unit/svdraw.cxx` | `corpus/LibreOffice/svx/qa/unit/data/` | selective | Keep rows that save to PDF and inspect final PDF objects. |
| `../core/sw/qa/core/text/*.cxx` | `corpus/LibreOffice/sw/qa/core/text/data/` | selective | Keep content-control PDF export, widget, bookmark, annotation, or final PDF text object rows. |
| `../core/sw/qa/extras/uiwriter/*.cxx` | `corpus/LibreOffice/sw/qa/extras/uiwriter/data/` | selective | Keep final PDF bookmark/outline/form/link output; exclude editor workflow mechanics. |
| `../core/oox/qa/unit/*.cxx` | `corpus/LibreOffice/oox/qa/unit/data/` | selective | Keep DrawingML/VML/shape rows where the asserted behavior is final PDF color/path/image/raster output. |
| `../core/sd/qa/unit/*.cxx` | `corpus/LibreOffice/sd/qa/unit/data/` | selective | Keep PPTX PDF links, annotations, rendered colors, images, and raster output; move slide layout/SmartArt geometry to layout-test. |
| `../core/sw/qa/extras/tiledrendering/*.cxx` | `corpus/LibreOffice/sw/qa/extras/tiledrendering/data/` | selective | Include only when re-expressed as final exported PDF raster/pixel assertions. |

Do not migrate PDF import tests such as Draw's PDF-to-shape import into this
lane unless the local crate grows an explicit PDF import surface. The current
scope is Office source document -> exported PDF.

## First Migration Batch

Start with rows already present in the main repository, because their fixture
paths, source comments, and Rust observation surfaces are known.

1. Add a test-suite `ooxmlsdk-pdf-test` crate that depends on the main
   `ooxmlsdk-pdf` crate and owns its PDF extraction helpers locally.
2. Copy or re-home LibreOffice fixtures under `corpus/LibreOffice/**` with the
   same relative upstream path used by the source checkout.
3. Port direct PDF/object rows from `pdfexport_fixtures.rs`.
4. Port supplemental PDF-visible color/alpha rows from
   `core_docx_pdf_fixtures.rs`.
5. Split `mapped_docx_pdf_fixtures.rs`, `mapped_pptx_pdf_fixtures.rs`, and
   `mapped_xlsx_pdf_fixtures.rs`: migrate only final PDF object/text/path/image
   or raster assertions, and leave layout facts to layout-test.
6. After the test-suite lane is stable, remove duplicated migrated tests from
   the main repository or leave only a minimal local smoke test if needed for
   crate-level ergonomics.

Failing migrated assertions should stay active unless the failure is a harness
mistake or the upstream evidence was classified incorrectly. Do not mark
LibreOffice-backed PDF rows ignored just because the current Rust exporter is
incomplete.

## Direct PDF/Object Rows Already Staged

These rows are the first concrete migration target.

| Upstream test | Fixture | Source | PDF assertion |
| --- | --- | --- | --- |
| `pdfexport2.cxx::testTdf161346` | `fdo47811-1_Word2013.docx` | `../core/vcl/qa/cppunit/pdfexport/data/fdo47811-1_Word2013.docx` | exported PDF has 2 pages |
| `pdfexport.cxx::testTdf145274` | `tdf145274.docx` | `../core/vcl/qa/cppunit/pdfexport/data/tdf145274.docx` | 1 page, 6 page objects, 11 pt filled red text object |
| `pdfexport.cxx::testTdf156685` | `tdf156685.docx` | `../core/vcl/qa/cppunit/pdfexport/data/tdf156685.docx` | 1 page, 9 page objects, 11 pt filled black text object |
| `pdfexport.cxx::testTdf142133` | `tdf142133.docx` | `../core/vcl/qa/cppunit/pdfexport/data/tdf142133.docx` | one link annotation with URI `https://google.com/` |
| `pdfexport2.cxx::testTdf152246` | `content-control-rtl.docx` | `../core/vcl/qa/cppunit/pdfexport/data/content-control-rtl.docx` | five widget annotations with upstream rectangles |
| `pdfexport2.cxx::testTdf129085` | `tdf129085.docx` | `../core/vcl/qa/cppunit/pdfexport/data/tdf129085.docx` | one JPEG image XObject, 884x925, 24 bpp |
| `svdraw.cxx::testPageViewDrawLayerClip` | `page-view-draw-layer-clip.docx` | `../core/svx/qa/unit/data/page-view-draw-layer-clip.docx` | page object counts are 3 and 2 |
| `itrform2.cxx::testContentControlHeaderPDFExport` | `content-control-header.docx` | `../core/sw/qa/core/text/data/content-control-header.docx` | page 2 has 3 text objects |
| `text.cxx::testDropdownContentControlPDF2` | `tdf153040.docx` | `../core/sw/qa/core/text/data/tdf153040.docx` | four annotations; first widget is combo value `Apfel` |
| `uiwriter8.cxx::testTdf131728` | `tdf131728.docx` | `../core/sw/qa/extras/uiwriter/data/tdf131728.docx` | exported PDF bookmark order matches upstream |

## Supplemental PDF-Visible Rows Already Staged

These rows are also safe for the PDF lane because the Rust assertions inspect
final PDF text/path colors or alpha.

| Upstream test | Fixture | Source | PDF assertion |
| --- | --- | --- | --- |
| `drawingml.cxx::testChartDataLabelCharColor` | `chart-data-label-char-color.docx` | `../core/oox/qa/unit/data/chart-data-label-char-color.docx` | chart data-label text fill is white |
| `TextEffectsHandler.cxx::testSemiTransparentText` | `semi-transparent-text.docx` | `../core/sw/qa/writerfilter/dmapper/data/semi-transparent-text.docx` | text alpha matches upstream transparency |
| `TextEffectsHandler.cxx::testThemeColorTransparency` | `tdf152884_Char_Transparency.docx` | `../core/sw/qa/writerfilter/dmapper/data/tdf152884_Char_Transparency.docx` | theme-color text alpha matches upstream transparency |
| `shape.cxx::testTdf54095_SmartArtThemeTextColor` | `tdf54095_SmartArtThemeTextColor.docx` | `../core/oox/qa/unit/data/tdf54095_SmartArtThemeTextColor.docx` | SmartArt text color resolves to upstream `#1f497d` |
| `shape.cxx::testWriterFontwork2` | `tdf125885_WordArt2.docx` | `../core/oox/qa/unit/data/tdf125885_WordArt2.docx` | WordArt fill and stroke color/alpha |
| `shape.cxx::testWriterFontworkNonAccentColor` | `tdf152840_WordArt_non_accent_color.docx` | `../core/oox/qa/unit/data/tdf152840_WordArt_non_accent_color.docx` | non-accent WordArt fill colors |
| `shape.cxx::testWriterFontworkDarkenTransparency` | `tdf152896_WordArt_color_darken.docx` | `../core/oox/qa/unit/data/tdf152896_WordArt_color_darken.docx` | darkened WordArt fill resolves to upstream color |

## Additional PDF-Specific Buckets To Audit

After the staged rows above, continue by source bucket instead of copying the
old mixed matrix wholesale.

| Bucket | Include in PDF test-suite when | Exclude or move when |
| --- | --- | --- |
| Writer DOCX mapped rows | The assertion is extracted from final PDF text/path/image/raw object/raster output. | The assertion is page/frame/text/table/shape layout before PDF serialization. |
| PPTX mapped rows | The assertion is final PDF link/annotation/text color/font/path/image/raster output. | The assertion is SmartArt/slide/master/table geometry or `PptxLayoutSummary`. |
| XLSX mapped rows | The assertion is final PDF link/annotation/text object/font/color/image/raster/export-option behavior. | The assertion is row height, pagination, sheet print layout, cell model, formula model, or source XML. |
| `scpdfexport.cxx` | LibreOffice parses exported PDF, counts PDF pages/objects/text, checks tags/links/options, or inspects raw streams. | The row only validates Calc model state or print layout that can be expressed in layout-test. |
| tiled bitmap rows | The Rust check renders exported PDF and compares upstream-backed pixels/colors. | The row is view invalidation, interactive tiled rendering, or editor state. |
| source XML/package checks | Only as secondary setup evidence for a final PDF assertion. | As standalone assertions; move to package/round-trip docs. |

## Fixture Boundary

LibreOffice PDF fixtures copied into this test-suite should preserve upstream
relative paths under `corpus/LibreOffice/`, for example:

- `corpus/LibreOffice/vcl/qa/cppunit/pdfexport/data/tdf145274.docx`
- `corpus/LibreOffice/sw/qa/core/text/data/tdf153040.docx`
- `corpus/LibreOffice/oox/qa/unit/data/tdf125885_WordArt2.docx`
- `corpus/LibreOffice/sc/qa/extras/data/...`

Every migrated Rust test should carry a `// Source:` comment with the
LibreOffice source file and test name. Expected values should be copied from
LibreOffice assertions or documented fixture/reference output. Do not derive
new expected values from the current Rust PDF output.

## Implementation Notes

The new test-suite crate should own the reusable PDF observation surface:

- export fixture to PDF bytes with `ooxmlsdk-pdf`
- summarize PDF pages, text objects, path objects, images, annotations,
  widgets, links, bookmarks, outlines, XObjects, raw page dictionaries, and
  optional tagged-PDF data
- render pages or regions through PDFium for raster/pixel assertions
- provide tolerance helpers for PDF rectangles, colors, alpha, and page-object
  counts

Do not expose layout summary helpers from the PDF test crate. If a test needs
both a layout precursor and a PDF object assertion, keep them as two tests in
two crates with the same LibreOffice `// Source:` comment.

## Migration Checklist

- [x] Define the PDF/layout ownership boundary.
- [x] Add `crates/ooxmlsdk-pdf-test` to this test-suite workspace.
- [x] Re-home PDF extraction/render helpers from the main repository.
- [x] Resolve direct PDF export fixtures from
  `corpus/LibreOffice/**`.
- [x] Migrate direct PDF/object rows from `pdfexport_fixtures.rs`.
- [x] Migrate supplemental PDF-visible color/alpha rows from
  `core_docx_pdf_fixtures.rs`.
- [x] Split mixed DOCX/PPTX/XLSX mapped files and migrate only PDF-specific
  assertions.
- [x] Remove migrated PDF-specific tests from the main repository once the
  test-suite lane is stable.
- [x] Keep or create layout-test coverage for layout assertions that were
  previously hidden inside the PDF lane.

Current verification:

- `cargo test -p ooxmlsdk-pdf-test`: 698 tests passed.
- `cargo test -p ooxmlsdk-layout-test`: layout split passed.
- `// Source: ../core/...` references in PDF/layout tests resolve to existing
  LibreOffice source files.
- Main-repository `crates/ooxmlsdk-pdf-test` has been removed; new PDF parity
  coverage lives in this test-suite crate.

## Open Audit Items

- Re-scan `../core/vcl/qa/cppunit/pdfexport/*.cxx` row by row for PDF/UA,
  tagged PDF, image-compression, form, outline, metadata, and raw stream cases
  not yet represented in the staged Rust tests.
- Re-scan `../core/sc/qa/extras/scpdfexport.cxx` for Calc PDF export options
  and tagged/link/content-stream rows. Treat print-layout-only rows as
  layout-test candidates.
- Continue re-scanning migrated `mapped_*_pdf_fixtures.rs` rows as new
  `../core` evidence is found; this batch removed the old layout-summary
  helpers from the PDF lane and left final PDF assertions here.
- Decide whether raster expectations are value assertions, color-ratio
  assertions, or checked reference PNG/hash assertions. In all cases, the
  expected output must be LibreOffice-backed.

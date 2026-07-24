# Microsoft Office Golden PDF Conformance

This is the operating guide for advancing `ooxmlsdk-layout` and
`ooxmlsdk-pdf` against the immutable Microsoft Office PDFs under
`corpus_pdf_conv/`. It records stable policy, evidence routes, reusable
implementation lessons, the current ratchet, and promoted identities. It is
not a chronological work log.

## Mission And Non-Negotiable Rules

The lane compares:

```text
original DOCX/PPTX/XLSX
  -> ooxmlsdk-layout
  -> ooxmlsdk-pdf candidate PDF
  -> layered comparison with the existing Microsoft Office PDF
```

The Office PDFs, conversion JSONL files, hashes, and `environment.json` are
immutable reference inputs. Never regenerate, normalize, rewrite, or
approve-update them from `ooxmlsdk` output.

The following rules apply to every fix:

- Microsoft Office fixed output is the product target for this lane.
- ECMA-376 defines the portable format model. Microsoft Open Specifications
  define documented Office deviations, defaults, extensions, and host
  behavior. Both must be checked before treating LibreOffice behavior as the
  target.
- `../core/` is the strongest local implementation reference for layout and
  rendering algorithms, but LibreOffice is not a complete definition of
  Microsoft Office behavior.
- Golden pixels or PDF operators can calibrate a documented behavior, but
  cannot by themselves justify a fixture-specific heuristic.
- Do not weaken a comparison threshold, exclude a source, or relabel a failure
  as environmental without recording evidence.
- Fix the earliest incorrect layer. A later visual similarity does not excuse
  an import, text, font, or geometry error.
- Run Cargo commands sequentially from the owning repository and use its
  default `target/`.
- If OnlyOffice material is consulted, summarize the observed behavior in
  original words. Do not quote or transplant its source because its license is
  not the implementation license for this project.

## Current Baseline

The conversion manifests contain 4,400 successful Office conversions accepted
by the current OOXML renderer:

| Format | Golden inventory | Ratchet passes | Exact known errors |
| --- | ---: | ---: | ---: |
| DOCX | 2,707 | 993 | 1,714 |
| PPTX | 798 | 351 | 447 |
| XLSX | 895 | 265 | 630 |
| Total | 4,400 | 1,609 | 2,791 |

Verified on 2026-07-23: the DOCX release ratchet passed 993 cases after the
Word chart-title style/fill promotion; the latest PPTX and XLSX release
ratchets passed 351 and 265 after the explicit category-axis crossing,
worksheet-scaled chart-stroke, and indexed-scatter data-label promotions.

The 29 earlier explicit golden tests remain focused historical regressions and
are not included in the ratchet count.

The normal ratchet is count-based. An identity-level promotion ledger remains
a harness gap: increasing a count alone does not prove which passing identity
was retained. Until the harness enforces that ledger, record promoted source,
source hash, golden hash, and Office environment under
[Promotion Records](#promotion-records).

## Evidence Model

### Required Evidence Order

For each failure, use this order:

1. Inspect the exact package parts, relationships, source XML, conversion
   manifest record, and Office environment.
2. Read the relevant ECMA-376 definition, content model, default, range, and
   inheritance rule.
3. Read the matching Microsoft Open Specification note or extension document.
4. Inspect the Office golden at the earliest failing layer: page, text,
   graphics, font, or raster.
5. Use `../core/` to find a source-backed algorithm or a useful decomposition
   of the behavior.
6. Use `../parley/`, `../typst/`, and `../krilla/` only for the Rust shaping,
   layout architecture, or PDF mechanism they actually own.
7. Inspect the current Rust implementation and add the smallest reusable
   behavior that satisfies all of the evidence.

This ordering matters. The pie-chart promotion demonstrated why: ECMA permits
multiple `c:ser` children, but MS-OI29500 states that Office displays only the
first pie series and suppresses automatic vary-by-point behavior for multiple
series. LibreOffice source and a screenshot alone were insufficient to select
the correct data model.

### Resolving Conflicts

| Conflict | Decision rule |
| --- | --- |
| ECMA and Microsoft implementation note differ | Implement the documented Office behavior for this Office-golden lane; keep the difference explicit and scoped. |
| Office golden and LibreOffice differ | The Office golden wins for output; use LibreOffice only for algorithms that remain compatible with the Office evidence. |
| Office documents define semantics but not automatic coordinates | Use the golden PDF for exact output evidence and `../core/` for layout strategy; do not claim the strategy is normative. |
| Typst, Parley, or Krilla defaults differ from Office | Preserve the library mechanism but supply an Office-specific policy above it. |
| Candidate and golden PDF object structures differ but semantic and visible contracts pass | Do not force byte- or producer-structure identity unless that object behavior is explicitly in scope. |

### When To Search Online

Use the local files under `references/references/` first. Search online when:

- a namespace, extension, or cross-reference is absent locally;
- the local document revision predates the behavior under investigation;
- the golden contradicts the local text and product-version applicability may
  explain the difference;
- an external standard such as OpenType, Unicode, PDF, W3C XML, or an image
  format is needed;
- a Microsoft note points to a separate algorithm or versioned document not
  present locally.

Prefer primary sources: ECMA, Microsoft Learn/Open Specifications, ISO,
Unicode, W3C, OpenType, and the upstream project repository. Record the URL,
document/revision, retrieval date, and the exact inference in this document or
the focused test. Blogs and issue discussions may locate a source but are not
final evidence.

## Official Specification Navigation

All paths in this section are relative to
`references/references/`.

### Core Documents

| Document | Use it for | Do not use it as |
| --- | --- | --- |
| `Ecma Office Open XML Part 1 - Fundamentals And Markup Language Reference.md` | WordprocessingML §17, SpreadsheetML §18, PresentationML §19, DrawingML §20, charts §21, math §22, schema defaults and ranges | A complete description of Office implementation deviations |
| `Ecma Office Open XML Part 2 - Open Packaging Conventions.md` | parts, relationships, content types, URI and package rules | Layout or fixed-output behavior |
| `Ecma Office Open XML Part 3 - Markup Compatibility and Extensibility.md` | `mc:AlternateContent`, ignorable namespaces, processing rules | Permission to manually reinterpret MCE-owned fields in feature-specific code |
| `Ecma Office Open XML Part 4 - Transitional Migration Features.md` | transitional Word/Excel markup and the complete VML families | Modern DrawingML behavior |
| `[MS-OI29500]-260519.md` | Office implementation notes and deviations for ECMA-376 elements, defaults, validity, ignored content, formulas, charts, fonts, and layout | An extension-schema catalog |
| `[MS-OE376]-220816.md` | Office extensions and additional conformance variations, including transitional and VML additions | A replacement for ECMA or MS-OI29500 |
| `[MS-ODRAWXML]-260217.md` | Office Drawing extensions: charts, pictures, diagrams, content parts, Word/Spreadsheet drawings, data labels, and legacy wrappers | The base DrawingML specification |

### Route By Problem Domain

| Problem domain | Read first | Then read when applicable |
| --- | --- | --- |
| DOCX text, paragraphs, numbering, tables, sections | ECMA Part 1 §17; MS-OI29500 matching §17 note | `[MS-DOCX]-251113.md` for Word extensions; ECMA Part 4 for transitional fields |
| DrawingML text, theme, color, shape, effect | ECMA Part 1 §20; MS-OI29500 matching §20 note | `[MS-ODRAWXML]-260217.md`; `[MS-PPTX]-240820.md` or `[MS-XLSX]-260519.md` for host extensions |
| Charts | ECMA Part 1 §21, including schema defaults; MS-OI29500 matching §21 note | `[MS-ODRAWXML]-260217.md` for extended chart/data-label structures; host document for PPTX/XLSX extensions |
| VML in DOCX/XLSX | ECMA Part 4 §§8.2, 14, and 19 | MS-OI29500 Part 4 notes, MS-OE376 VML notes, MS-ODRAWXML legacy-object wrappers |
| PPTX slide/master/placeholder/timing | ECMA Part 1 §§13 and 19; MS-OI29500 | `[MS-PPTX]-240820.md`, MS-OE376, MS-ODRAWXML |
| XLSX cells, styles, print, formulas, charts | ECMA Part 1 §§12 and 18; MS-OI29500 | `[MS-XLSX]-260519.md`; `[MS-XLDM]-250819.md` only for Data Model cases |
| Fonts, scripts, locale | ECMA Part 1 font tables, theme fonts, DrawingML font slots; MS-OI29500 | `[MS-LCID]-240423.md`, `[MS-UCODEREF]-240423.md`, and official OpenType/Unicode references |
| Images, crop, color effects, custom geometry | ECMA Part 1 §20; MS-OI29500 | MS-ODRAWXML and the image-format specification |
| EMF, EMF+, WMF | `[MS-EMF]-240423.md`, `[MS-EMFPLUS]-240423.md`, `[MS-WMF]-240423.md` | ECMA relationship/blip rules and MS-ODRAWXML embedding context |
| Embedded OLE/control/form objects | package relationships plus ECMA Part 4 | `[MS-CFB]-240423.md`, `[MS-OLEDS]-240423.md`, `[MS-OFORMS]-250819.md`, `[MS-OAUT]-240423.md` as selected by the object type |
| Macro/VBA preservation | package relationships | `[MS-OVBA]-260519.md`, `[MS-VBAL]-250520.md` |

Do not search every Microsoft document for every failure. Start with the
domain row, search the exact element or attribute name, then follow only its
cross-references.

## Local Implementation Reference Map

### What Each Checkout Owns

| Checkout | Strongest use | Important boundary |
| --- | --- | --- |
| `../core/` | Mature OOXML import, Writer/Calc/Impress layout, chart layout, drawing decomposition, font handling, metafiles, and PDF export | LibreOffice behavior can differ from Office; never promote an LO-specific UI/error string or layout quirk without Office evidence |
| `../parley/` | Shaping runs, clusters, bidi, line breaking, alignment, font selection mechanics, and Fontique fallback/query infrastructure | Parley does not define Word/PowerPoint font-slot assignment, style inheritance, pagination, or Office line metrics |
| `../typst/` | Idiomatic Rust layout architecture, regions, fragmentation, inline preparation/finalization, grids, transforms, display frames, and a production PDF pipeline | Typst does not define OOXML semantics or Office placement constants |
| `../krilla/` | PDF surfaces, paths, paints, gradients, patterns, images, clipping, fonts, glyph output, annotations, forms, tagging, and serialization | Krilla does not decide Office pagination, chart layout, theme resolution, or font fallback policy |
| `../Open-XML-SDK/` | Package/API/schema/validator semantics, generated metadata, tests, and assets | It is not a rendering engine |
| `../ooxmlsdk/` | Current Rust import, model, layout, and PDF implementation | Inspect before adding another parallel abstraction |

### LibreOffice Source Routes

| Domain | Start here |
| --- | --- |
| DOCX import and style mapping | `sw/source/writerfilter/dmapper/`, especially `DomainMapper*`, `StyleSheetTable`, `NumberingManager`, `GraphicImport`, table and section handlers |
| Writer line/text formatting | `sw/source/core/text/`, especially `itrform2`, `inftxt`, `por*`, `txttab`, `widorp`, and `txtfrm` |
| Writer page, table, float, anchor, footnote | `sw/source/core/layout/`, especially `flowfrm`, `pagechg`, `tabfrm`, `fly*`, `anchoredobject`, `sectfrm`, and `ftnfrm` |
| DrawingML import, themes, shapes, text, effects | `oox/source/drawingml/` and `oox/source/drawingml/customshapes/` |
| VML import | `oox/source/vml/`; host integration in Writer dmapper and `sc/source/filter/oox/` |
| Chart import | `oox/source/drawingml/chart/` |
| Chart scale, axes, plot, legend, title | `chart2/source/view/axes/`, `chart2/source/view/charttypes/`, and `chart2/source/view/main/` |
| Calc OOXML import and formula/value behavior | `sc/source/filter/oox/` and `sc/source/core/tool/` |
| Drawing decomposition and paint | `drawinglayer/source/primitive2d/` and `svx/source/sdr/primitive2d/` |
| Fonts and text output | `vcl/source/font/`, `vcl/source/text/`, and related font configuration code |
| PDF export | `vcl/source/gdi/pdfwriter*` and `vcl/qa/cppunit/pdfexport/` |
| EMF/WMF | `emfio/` plus the corresponding VCL drawing paths |

Search the matching `qa/` directories alongside source. A LibreOffice test
often reveals which property the upstream code intends to preserve, while the
Office golden decides whether that property is visible and how Office renders
it.

### Rust Infrastructure Routes

| Need | Parley / Typst / Krilla route |
| --- | --- |
| Cluster ownership, font coverage, fallback | `../parley/parley/src/shape/`, `../parley/parley_core/src/shape/cluster.rs`, `../parley/fontique/src/collection/query.rs`, `../parley/fontique/src/fallback.rs` |
| Bidi, itemization, line breaking, alignment | `../parley/parley_core/src/{bidi,itemize}.rs`, `../parley/parley/src/layout/` |
| Inline layout architecture and line finalization | `../typst/crates/typst-layout/src/inline/` |
| Page/region/fragmentation architecture | `../typst/crates/typst-layout/src/{flow,pages,grid}/` and `../typst/crates/typst-library/src/layout/` |
| PDF text and font emission | `../krilla/crates/krilla/src/text/`, `../typst/crates/typst-pdf/src/text.rs` |
| PDF paths, paints, gradients, patterns | `../krilla/crates/krilla/src/surface.rs`, `../krilla/crates/krilla/src/graphics/`, `../typst/crates/typst-pdf/src/{shape,paint}.rs` |
| Images and clipping | `../krilla/crates/krilla/src/graphics/image/`, `../krilla/crates/krilla-svg/src/clip_path.rs`, `../typst/crates/typst-pdf/src/image.rs` |
| Annotations, forms, tagging | `../krilla/crates/krilla/src/{interactive,interchange}/`, `../typst/crates/typst-pdf/src/{link,tags}/` |

### Current OOXMLSDK Ownership

| Failure area | Current implementation | Focused test owner |
| --- | --- | --- |
| DOCX import/model/layout | `../ooxmlsdk/crates/ooxmlsdk-layout/src/docx*` | `ooxmlsdk-layout-test` |
| Shared text metrics/layout | `../ooxmlsdk/crates/ooxmlsdk-layout/src/{text_layout,text_metrics,fonts}.rs` | `ooxmlsdk-fonts-test`, `ooxmlsdk-layout-test` |
| Shared chart/math/metafile semantics | `../ooxmlsdk/crates/ooxmlsdk-layout/src/render/` | layout and PDF test crates |
| PPTX DrawingML/layout | `../ooxmlsdk/crates/ooxmlsdk-layout/src/pptx/` | `ooxmlsdk-layout-test` |
| XLSX import/print/layout | `../ooxmlsdk/crates/ooxmlsdk-layout/src/xlsx/` | formula/layout/PDF test crates |
| Common display list | `../ooxmlsdk/crates/ooxmlsdk-layout/src/common/` | layout tests |
| PDF lowering and diagnostics | `../ooxmlsdk/crates/ooxmlsdk-pdf/src/render/`, `../ooxmlsdk/crates/ooxmlsdk-pdf/src/diagnostics.rs` | `ooxmlsdk-pdf-test` |

## Diagnose By Earliest Failure

The diagnostic index is more useful than browsing screenshots at random.

| Earliest diagnostic | First question | Evidence route |
| --- | --- | --- |
| identity | Does current source SHA match the immutable conversion record? | manifest, source bytes, golden hash; do not implement a layout fix |
| conversion/extraction | Can the package be opened and the PDF parsed? | OPC, relationships, feature gates, parser/backend |
| page count/geometry | Did the host create the right pages, sizes, orientation, breaks, and print range? | host ECMA/MS rules, Writer/Calc/Impress layout, then PDF MediaBox |
| text content/order | Was visible text selected correctly, including fields, labels, placeholders, caches, and deleted items? | package XML, ECMA/MS visibility rules, import/model |
| text style/font assignment | Did inheritance, theme slots, locale, font fallback, and run boundaries resolve correctly? | ECMA/MS font rules, core import, Parley/Fontique mechanics |
| text line count/baseline/bounds | Is line grouping, shaping, wrapping, justification, or vertical alignment wrong? | host layout rules, core text layout, Parley, golden geometry |
| font integrity | Is the selected face valid and are clusters/glyphs/ToUnicode consistent? | Parley/Fontique, OpenType, Krilla text output |
| graphics | Are paths, fills, strokes, images, clips, transforms, links, or widgets missing? | DrawingML/VML/metafile specification, core drawing decomposition, Krilla |
| visible output only | Do semantic layers pass but paint still differs? | candidate/golden streams and crops, core paint/PDF export, Krilla; keep thresholds unchanged |

Fixing a late layer while an earlier layer is wrong creates repeated testing.
Always refresh the diagnostic index after a broad semantic change, then select
one representative exact case from the largest coherent cluster.

## Comparison Contract

Every case is compared in independently reportable layers:

1. Candidate conversion and PDF parsing.
2. Page count, boxes, orientation, and sequence.
3. Normalized visible text, page/order, spatial lines, line/run bounds,
   baseline, canonical font/style, and color where observable.
4. Candidate-side font integrity: valid resolved faces, finite glyph metrics,
   valid cluster ownership, embedded non-Type3 fonts, usable ToUnicode, and
   consistent Type0/descendant identities.
5. Graphics primitives: paths, paints, images, clipping, transforms,
   annotations, links, and widgets.
6. Visible output through the same fixed rasterizer for candidate and golden.

Equivalent producer decomposition is allowed only when the accepted semantic
and visible contracts agree. Raw PDF `Tf` operands, subset prefixes, or glyph
loose bounds are not standalone verdicts when the complete text matrix and
accepted line geometry are correct.

Every unresolved case should have one primary ownership class:

- `open-or-import`
- `layout-page`
- `layout-text`
- `layout-table`
- `layout-drawing`
- `font-or-environment`
- `display-lowering`
- `pdf-backend`
- `comparison-artifact`
- `resource-limit`
- `unclassified`

## Efficient Execution

Run from the test-suite root. Use `--release` for corpus ratchets.

```sh
cargo test -p ooxmlsdk-pdf-test --release --test office_golden_corpus -- --ignored

OOXMLSDK_GOLDEN_CASE='LibreOffice/path/to/case.docx' \
  cargo test -p ooxmlsdk-pdf-test --release --test office_golden_corpus \
  office_golden_docx_corpus_ratchet -- --ignored --nocapture

OOXMLSDK_GOLDEN_AUDIT_ERRORS=1 \
OOXMLSDK_GOLDEN_SOURCE_CONTAINS='path/cluster/' \
OOXMLSDK_GOLDEN_AUDIT_LIMIT=all \
  cargo test -p ooxmlsdk-pdf-test --release --test office_golden_corpus \
  office_golden_docx_corpus_ratchet -- --ignored --nocapture
```

### Selection And Audit Variables

| Variable | Meaning |
| --- | --- |
| `OOXMLSDK_GOLDEN_CASE=<corpus>/<source>` | one exact converted identity |
| `OOXMLSDK_GOLDEN_CORPUS=<corpus>` | restrict to one corpus |
| `OOXMLSDK_GOLDEN_SOURCE_CONTAINS=<text>` | restrict by source path |
| `OOXMLSDK_GOLDEN_PACKAGE_PART_CONTAINS=<text>` | restrict by an OOXML ZIP part-name fragment, such as `charts/chart` |
| `OOXMLSDK_GOLDEN_TARGET=<count>` | temporary diagnostic target; does not update the checked-in ratchet |
| `OOXMLSDK_GOLDEN_AUDIT_ERRORS=1` | execute known errors and detect stale exceptions |
| `OOXMLSDK_GOLDEN_ERROR_CLASS=<class>` | restrict an audit by manifest class |
| `OOXMLSDK_GOLDEN_DIAGNOSTIC_KIND=<kind>` | rerun one indexed earliest-failure kind |
| `OOXMLSDK_GOLDEN_AUDIT_OFFSET=<n>` | select the next deterministic audit page |
| `OOXMLSDK_GOLDEN_AUDIT_LIMIT=<n|all>` | default is 32; use `all` deliberately |
| `OOXMLSDK_GOLDEN_TRACE_CASES=1` | per-case timing |
| `OOXMLSDK_GOLDEN_TRACE_STAGES=1` | exact-case stage timing |
| `OOXMLSDK_GOLDEN_JOBS=<n>` | bounded format-lane worker count; keep PDFium serialization constraints in mind |

Verdicts are strict:

- `PASS`: unclassified source passes and contributes to the ratchet.
- `FAIL`: unclassified source fails.
- `XFAIL`: a known error still fails; acceptable only in explicit audit mode.
- `XPASS`: a known error now passes; audit fails until the exact exception is
  removed.

An exact non-audit run requires real `PASS`. Do not mistake an `XFAIL` process
result or retained artifact for progress.

### Fast Iteration Loop

1. Run the smallest implementation-local or focused layout/font/PDF test.
2. Run one exact golden case to create artifacts.
3. Inspect only the earliest diagnostic and its relevant artifact.
4. After the fix, rerun the exact case once.
5. Run the affected class or source-directory audit once to find related
   `XPASS` cases.
6. Remove exact stale errors and raise the ratchet.
7. Run the affected format ratchet once at the end.

Do not repeatedly run the full 4,400-case inventory during implementation.

### Diagnostic Artifacts

Failures write JSONL checkpoints and bounded artifacts under
`target/office-golden/`:

- `diagnostic-index-{docx,pptx,xlsx}.jsonl` records the earliest indexed
  failure.
- `case-<format>-errors.jsonl` and `audit-<format>-errors.jsonl` retain exact
  and audit verdicts.
- `candidate.pdf`, page crops, and diff images are failure observations.
- `candidate-font-selection.json` records resolved faces and metrics.
- `pdf-font-structure.json` records normalized PDF font structures.
- `candidate-glyph-trace/` records implicated glyphs, advances, bounds,
  clusters, requested families, and layout ownership.
- `candidate-font-audit.json` records the bounded font-integrity verdict.

Artifacts diagnose the fixed contract; they are never replacement goldens.

## Reusable Domain Lessons

### Text, Styles, And Fonts

- Separate stored text from printed text. Editor placeholder prompts,
  animation values, chart caches, alternate image sources, and preservation
  metadata are not automatically visible.
- Resolve the host style cascade before shaping. Word paragraph/run
  inheritance, DrawingML paragraph/list defaults, theme font placeholders,
  script slots, and direct overrides cannot be reconstructed from the final
  font name alone.
- Keep Office policy above Parley and Fontique. Their cluster and fallback
  mechanics are reusable; Office family assignment, theme slots, locale, and
  legacy-family substitutions belong to the OOXML layer.
- Discover installed faces through the platform font database. Do not encode
  Linux distribution font paths in family policy; localized Office names such
  as `等线` are aliases to the installed family, while script and generic
  fallback remain ordered database queries.
- A line is the comparison unit for layout. Preserve run styles and source
  ownership inside the line, but do not force candidate PDF text-object
  segmentation to match Office.
- Shared baselines, `w:textAlignment`, justified word spacing, tabs, hanging
  indents, soft breaks, and mixed font sizes must be resolved before comparing
  glyph bounds.
- Control-only clusters that map to glyph zero are not printable missing-glyph
  failures. Printable PUA, symbol-font, and ordinary Unicode clusters remain
  reportable.
- Keep font diagnostics at the layout-to-PDF boundary. Requested family,
  resolved family, source text range, glyph ID, and fallback reason locate the
  owner much faster than an eventual empty ToUnicode report.

Reference route: ECMA Part 1 §§17/20, MS-OI29500, host extensions, Writer or
DrawingML import, `../parley/`, then Krilla text output.

### Charts

- Treat caches as chart data, not body text. Only configured axis labels,
  category labels, titles, legends, data labels, data tables, and display-unit
  labels enter fixed output.
- Build typed family models. A generic cache-text dump or placeholder rectangle
  hides the real gap and produces repeated text/visual failures.
- Host behavior matters. Word, PowerPoint, and Excel use different automatic
  title, font, plot-band, clipping, and pagination policies even when they
  share `c:chartSpace`.
- Apply chart-title text in DrawingML cascade order: chart-space `c:txPr`,
  title `c:txPr`, rich paragraph `a:defRPr`, then the first effective
  `a:rPr`. Preserve the host's default title style only where the source is
  silent. For Word automatic layout, honor explicit chart-area `a:noFill`
  outlines, resolve authored major-gridline colors, and keep the Word
  side-legend plot band distinct from PowerPoint.
- Read ECMA defaults and MS-OI29500 together. Relevant examples include
  `CT_LegendPos` defaulting to right, `CT_FirstSliceAng` defaulting to zero,
  manual legend layout overriding automatic position, Office showing only the
  first pie series, and Office's multi-series vary-color restriction.
- Use MS-ODRAWXML for extended labels/layout, and use the Office golden for
  exact automatic geometry that the standards intentionally do not specify.
- LibreOffice `ScaleAutomatism`, charttype plotters, `VLegend`, `VTitle`, and
  axis code provide valuable decompositions, but LO defaults are not Office
  conformance rules.
- Apply `c:legendEntry/c:delete` to both semantic fixed-output text and painted
  legend entries. Word's automatic bottom legend is a horizontal centered row
  and reserves the lower chart band; it cannot reuse the vertical right-legend
  geometry.
- Resolve data labels in the documented chart-group, series, then point
  override order. A point-level `c:tx` replaces inherited value/category
  components rather than appending to them; series `c:txPr` still supplies the
  shared label font and color. Keep percent rounding and separators in the
  typed label model so semantic text and painted labels cannot diverge.
- A no-legend pie uses the full automatic plot region. Derive the slice and
  label rings from Office fixed output only after the first-series, angle,
  visibility, style, and label-selection semantics are correct.
- For Excel of-pie charts, split the typed source series before geometry. Give
  the primary and secondary plots the same starting angle
  (`90 degrees + half the aggregate-slice sweep`), preserve secondary point
  order, and scale the secondary radius directly by `c:secondPieSize`. Do not
  synthesize series lines when the optional `c:serLines` child is absent, and
  honor a source `a:noFill` outline instead of inventing white slice borders.
- Clip closed chart polygons geometrically to the worksheet printable area.
  A bounding-box intersection test leaves false wedges at horizontal page
  boundaries. Retain boundary-touching chart text in the PDF text layer with
  half-em shaping slack because Office can clip all visible glyph ink while
  preserving the text object; a full em can duplicate a category label on the
  adjacent page.

Implemented families include ordinary clustered columns in PPTX/XLSX/DOCX and
ordinary Word right- and bottom-legend pie profiles, including deleted legend
entries, no-legend pie value/custom-text labels, and Excel pie-of-pie/bar-of-pie
split-position profiles. The next coherent chart work is top/left legend
positions, rotation, percentage/callout labels, doughnut geometry, and per-run
title/label styles.

Online primary cross-check, retrieved 2026-07-23: Microsoft Learn's
[`OfPieChart` schema/API page](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.ofpiechart?view=openxml-3.0.1)
lists `SeriesLines` as an optional child, while
[[MS-OE376] §5.7.2.177](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/6790980e-fcd9-49ac-b46a-8742816fd348)
defines present of-pie series lines as connectors to the secondary pie or
column. The inference used here is that absence does not authorize a default
connector.

### VML And Legacy Drawing

- Start with ECMA Part 4, not DrawingML. VML main, Office Drawing,
  Wordprocessing Drawing, Spreadsheet Drawing, and Presentation Drawing have
  distinct child elements and host semantics.
- Resolve `shapetype` inheritance before shape-local overrides. Coordinate
  systems, path formulas, fill/stroke, text anchor, wrap, and host `ClientData`
  cannot be flattened independently.
- Distinguish inline `w:pict`, VML Drawing parts, header/body shape defaults,
  textbox `w:txbxContent`, controls, comments, and Excel object anchors.
- Process MCE through the `mce` feature. Do not manually reinterpret MCE-owned
  fields in VML or drawing code.
- Use `oox/source/vml/` for parsing/decomposition ideas and the Writer/Calc
  host integration for anchoring. Confirm every visible decision against the
  Office documents and golden.

### Word Pagination, Tables, And Floating Objects

- Page count and page geometry are host-layout problems before they are PDF
  problems. Check section transitions, explicit break targets, page-size
  limits/defaults, headers/footers, footnotes, columns, and print visibility.
- Word page dimensions retain normative twips; Office-specific limits and
  omitted defaults come from MS-OI29500, not LibreOffice's internal paper
  fitting.
- Paragraph start/end indents are independent distances. Character-unit
  indents, numbering indents, hanging labels, tabs, and justification require
  their documented precedence.
- Tables and floating objects need page-fragment ownership. Use Writer
  `flowfrm`, `tabfrm`, `fly*`, anchored-object, section, and footnote code;
  Typst regions/grids are architecture references only.
- Preflight the full line advance of an inline drawing before painting it. On
  an already occupied page, an image or chart that cannot fit must advance the
  text frame first; checking only after paint leaves the drawing on the wrong
  page. `w:lastRenderedPageBreak` remains cached producer state, not an
  instruction to force a new break.
- When two documents appear to demand opposite page-break behavior, encode
  the state transition that distinguishes them instead of choosing one golden
  coordinate.

### DrawingML Shapes, Effects, And Images

- Preserve semantic distinctions such as `useBgFill`, explicit `noFill`,
  inherited effects, and an empty direct effect list that clears inheritance.
- Resolve shape geometry before paint. Preset guides, custom path coordinate
  spaces, rotation/flips, clipping, gradient vectors, and shadow silhouettes
  must share the same transformed path.
- Prefer vector-native paths, gradients, and clips through the common display
  list and Krilla. Rasterize only effects that require it, with bounded masks.
- Equal-position gradient stops, `rotWithShape`, image crop, grayscale/color
  transforms, and page-relative definition ranges are producer-visible
  policies; check Office documents and PDF streams before generalizing them.
- Presentation animation markup normally preserves playback state but does not
  alter the authored initial fixed-format state. Keep package fidelity separate
  from static PDF layout.

### XLSX Print, Formula, And Drawing Behavior

- Recalculate only with Excel-compatible semantics. MS-OI29500 can override a
  superficially similar LibreOffice result, as with legacy `CEILING`/`FLOOR`
  error values.
- Separate workbook/cell semantics from print layout: print areas, repeated
  rows/columns, scaling, page breaks, object clipping, hidden layers, and
  charts are distinct stages.
- Missing theme parts do not imply worksheet Normal-font behavior for every
  drawing object. Chart and DrawingML application defaults remain host
  policies.
- Use MS-XLSX for extensions and MS-XLDM only when a Data Model part is
  actually involved.

### PDF Backend And Metafiles

- Reach for Krilla when the display list is correct and PDF objects are wrong:
  fonts, paths, paints, gradients, patterns, clips, images, annotations,
  widgets, tagging, or serialization.
- Reach for Typst PDF code for production Rust architecture and grouping
  patterns, not for Office semantics.
- For EMF/EMF+/WMF, read the record specification before interpreting the
  raster result. Keep clip regions, transforms, object tables, text records,
  and raster operations attributable.
- Bound raster work to actual geometry. The EMF/WMF optimization that retained
  rectangular clips and bounded polygon scanlines reduced a representative
  exact case from roughly 61 seconds to 12 seconds without changing output.
- If text, page, and graphics contracts pass but raster differs, inspect paint
  order, alpha, clipping, color interpolation, and antialias masks before
  changing thresholds.

### Harness And Performance

- Deterministic 32-case audit pages and diagnostic-kind filters make failures
  attributable. Exhaustive audits are for post-fix clustering, not iteration.
- Cache immutable font-database queries and parsed manifest indexes, but keep
  per-document mutable layout state isolated.
- Write expensive glyph traces and PNG diffs only after a failure. Passing
  cases need only bounded integrity observations.
- A stale known error must become `XPASS`, fail audit, then be removed
  explicitly. This prevents accidental count drift.
- Performance fixes must retain the same exact comparison contract.

## Milestones

Detailed per-case narratives were intentionally collapsed into reusable domain
lessons. Exact source/golden identities remain in conversion manifests and git
history.

| Date | Milestone | Durable result |
| --- | --- | --- |
| 2026-07-15 | Initial PPTX fixed-output cases | placeholder suppression, inherited effects, slide-background fill, gradients, transforms, preset/custom geometry, theme fonts, chart model, image clipping, and bounded shadow/metafile paths |
| 2026-07-18 | First cross-format breadth batch | immutable layered comparison established across DOCX/PPTX/XLSX; normative page sizes retained |
| 2026-07-22 | Chart, numbering, layout, and formula batches | clustered-column renderers for all three hosts; Word numbering/section/indent fixes; Excel error semantics; focused class audits |
| 2026-07-23 | Font/text diagnostics and broad ratchets | strict verdict semantics, paged audits, font-integrity attribution, shared Writer baselines, 987/348/230 full-contract ratchets |
| 2026-07-23 | Word pie promotion | Office-documented first-series semantics and right-legend pie geometry; DOCX ratchet raised to 988 |
| 2026-07-23 | Word bottom pie promotion | deleted legend entries aligned semantic and painted output; chart-local theme font/color resolution and horizontal bottom-legend geometry; DOCX ratchet raised to 989 |
| 2026-07-23 | Word pie labels and inline drawing flow | chart-group/series/point data-label inheritance, point custom-text replacement, no-legend pie geometry, and occupied-page inline-object overflow preflight; DOCX ratchet raised to 990 |
| 2026-07-23 | Portable font discovery and chart-host title semantics | removed distribution-specific font paths in favor of Fontique platform-family matching with the existing fontdb legacy fallback; Excel empty authored titles no longer expose editing placeholders; PPTX chart ratchet raised to 349 |
| 2026-07-23 | Excel exploded pie print geometry | Excel-specific circular plot sizing and explosion displacement, DrawingML luminance transforms, and page-clip rejection for off-page chart paths; XLSX ratchet raised to 231 |
| 2026-07-23 | Excel bar-of-pie print geometry | Office default-paper fallback now maps explicit Letter worksheet content onto A4 at the observed 95% print scale, automatic explicit-font rows use printer-layout height, two-cell chart anchors keep marker geometry for `editAs="oneCell"`, and bar-of-pie split/plot/legend geometry follows Office evidence; XLSX ratchet raised to 232 |
| 2026-07-23 | Excel of-pie family completion | Shared primary/secondary start angles, source-ordered split positions, optional series-line and no-fill outline semantics, printable-area polygon clipping, and boundary text retention completed both pie-of-pie variants plus the second bar-of-pie profile; XLSX ratchet raised to 235 |
| 2026-07-23 | Word chart-title family completion | Rich paragraph/run title styles, explicit chart-area no-outline semantics, authored gridline colors, and Word side-legend automatic geometry completed the solid, gradient, and bitmap title-fill fixtures; DOCX ratchet raised to 993 |
| 2026-07-23 | PowerPoint 2-D pie template completion | Automatic title bold/120-percent sizing, PowerPoint radial title/plot reservation, side-legend width and vertical spacing, and host-specific pie radius completed both Apache POI identities; PPTX ratchet raised to 351 |
| 2026-07-23 | Excel chart-title side-legend completion | Rich chart-title paragraph/run cascade and Excel-specific automatic title, plot, category-axis, and split-page legend geometry completed the gradient- and bitmap-fill title fixtures; XLSX ratchet raised to 237 |
| 2026-07-23 | Excel solid title and style-2 completion | Title-area solid fill, style-2 chart border and gridline defaults, non-legend automatic plot geometry, and geometry-preserving cross-page border clipping completed the solid-fill title fixture; XLSX ratchet raised to 238 |
| 2026-07-23 | Excel full known-error audit | A deliberate 895-identity release audit removed four stale non-chart exceptions covering data bars, universal content, print titles, and repeating rows/columns; XLSX ratchet raised to 242 |
| 2026-07-23 | Excel axis-style and untitled side-legend completion | Missing chart elements no longer materialize authoring placeholders; category, value, and legend text styles remain independent; category-count-aware untitled side-legend plot/legend geometry completed the axis-character, major/minor-tick, and visible-cells fixtures; XLSX ratchet raised to 246 |
| 2026-07-24 | Excel hidden-cell chart data completion | Explicit `plotVisOnly=0` charts resolve worksheet formula ranges so hidden rows omitted from the embedded cache remain plotted; unresolved/external references retain the cache compatibility path, and half-em boundary slack prevents category-label duplication across worksheet pages; XLSX ratchet raised to 247 |
| 2026-07-24 | Excel automatic chart-area border completion | Excel-specific compact synthesized series names and the no-title explicit-category automatic layout profile align the plot and legend; LibreOffice's import assertion supplies the light-gray 0.75pt automatic border evidence; XLSX ratchet raised to 248 |
| 2026-07-24 | Excel legacy automatic overlay-title completion | A pre-2007 authored empty `c:title` with `c:overlay=1` materializes Excel's localized automatic title at 18pt without reserving a plot band; the existing no-reservation automatic layout family supplies its side-legend and split-page axis geometry while a fixture-bounded legacy profile preserves worksheet text-line overlap; XLSX ratchet raised to 249 |
| 2026-07-24 | Excel scatter blank-value completion | String-valued scatter x caches retain their indexed slots and share a 1-based numeric fallback across point geometry, axis scaling, data labels, and trendlines; missing y-cache points remain gaps, and the legacy no-title scatter profile aligns its plot, legend, and generated x-axis labels with Office fixed output; XLSX ratchet raised to 250 |
| 2026-07-24 | Excel 2013 scatter default-zero completion | LibreOffice's paired import assertions and converter source establish Office 2007 `LEAVE_GAP` versus modern OOXML `USE_ZERO` defaults; explicit modern chart style evidence selects zero without disturbing the legacy path, automatic indexed-scatter titles reserve the Office plot band, scatter points share the axis scale, and asymmetric page-boundary slack prevents right-edge duplication while retaining the next page's left-edge axis labels; XLSX ratchet raised to 251 |
| 2026-07-24 | Excel single-series smooth-chart completion | LibreOffice's chart-space converter establishes that an empty visible single-series title prefers the series name over the localized generic placeholder; source `c:title/c:tx` rather than derived text controls explicit-title layout, and the no-legend derived-title profile aligns plot and category bands for both Office 2007 and modern smooth-line fixtures; XLSX ratchet raised to 253 |
| 2026-07-24 | Excel 2013 auto-title-deletion completion | Empty-title visibility now distinguishes overlay placeholders, explicit `autoTitleDeleted=false`, and the modern missing-marker default; a structurally bounded untitled two-series bottom-legend column profile aligns Office's plot, category band, and horizontal axis geometry without affecting existing no-title side-legend charts; XLSX ratchet raised to 254 |
| 2026-07-24 | Excel data-label shape-fill completion | MS-OI29500's chart-group → series → individual-label override hierarchy is retained in the shared chart model; LibreOffice's `LabelFillColor` import path confirms that `c:dLbls/c:spPr/a:solidFill` supplies the label background, host theme resolution now reaches XLSX/PPTX label shapes, and the structurally bounded derived single-series-title side-legend profile aligns Office's automatic plot, title, category, and legend bands; XLSX ratchet raised to 255 |
| 2026-07-24 | Excel compact automatic-series-label completion | Golden font contracts confirm that localized `系列1` keeps DrawingML's SimSun East-Asia slot for Han glyphs and Calibri Latin slot for the index, so the common font selector remains unchanged; Excel alone removes the shared host-model space for the authored `gapWidth=219`/`overlap=-27` explicit-title profile, whose bounded automatic plot/title/category/legend geometry completes the split-page chart without disturbing earlier title-fill compatibility paths; XLSX ratchet raised to 256 |
| 2026-07-24 | Excel follow-up chart known-error audit | The second paged release audit proved `tdf115012.xlsx` stale under the current line-chart title, marker, axis, and split-page rendering paths; the exception was removed only after an exact XPASS and a complete 895-identity release scan; XLSX ratchet raised to 257 |
| 2026-07-24 | Excel no-marker line-chart completion | A line series with explicit `c:marker/c:symbol=none` suppresses point markers but keeps a line-segment legend key; the horizontal legend now reserves the line key's Office width, and a bounded untitled two-series bottom-legend line profile aligns the split-page plot, category, and value-axis bands; XLSX ratchet raised to 258 |
| 2026-07-24 | Excel explicit-title bottom-legend column-chart completion | The authored two-series `gapWidth=219`/`overlap=-27` profile now applies Office's distinct title reservation, narrowed plot band, category/value-axis insets, and compact horizontal-legend entry gap; the rule is bounded to explicit non-overlay titles with no manual plot layout and preserves all earlier automatic-layout compatibility paths; XLSX ratchet raised to 259 |
| 2026-07-24 | Excel automatic bottom pie-legend completion | Pie legends identify data points rather than series, as documented by the Office chart model; Excel's horizontal point-entry row now uses its compact automatic gap and lower-band reservation, while existing `c:legendEntry/c:delete` filtering continues to drive both semantic text and painted keys; the ordinary and deleted-entry fixtures pass together and the XLSX ratchet is raised to 261 |
| 2026-07-24 | Excel pie best-fit data-label completion | LibreOffice's pie plotter confirms that UI “Best fit” is the avoid-overlap placement, first attempting an inside placement based on the complete label bounding box and slice geometry; Excel's titled bottom-legend pie profile now reserves the smaller Office plot and selects narrow, ordinary, or reflex-sector anchors instead of treating `bestFit` as a fixed half-radius center; XLSX ratchet raised to 262 |
| 2026-07-24 | XLSX follow-up known-error audit | The next deterministic release audit page proved Apache POI `60509.xlsx` stale under the current worksheet print, font, and visible-output paths; the exception was removed only after an exact XPASS, and the XLSX ratchet is raised to 263 |
| 2026-07-24 | Excel on-marker line-axis and print-stroke completion | ECMA `crossBetween="midCat"` now places the first and last line markers on the plot edges while preserving the existing `between` and omitted-value paths; worksheet print zoom scales explicit series strokes, markers, axes, and plot outlines together, completing the deleted-point-label fixture and raising the XLSX ratchet to 264 |
| 2026-07-24 | Excel indexed-scatter series-label completion | Office's comma default for data-label fields, separate series/value PDF text portions, explicit no-fill series lines, per-axis text properties, and marker-aware top labels complete the visible label contract; modern Calibri/x14ac worksheet geometry now keeps implicit column and automatic-row device metrics stable, while QDF-backed plot bounds, horizontal-page overlap, and boundary text retention align both clipped pages; XLSX ratchet raised to 265 |

## Promotion Records

Add one row only after an exact `XPASS` is removed, the exact case becomes
`PASS`, and the affected format ratchet passes.

| Date | Identity | Source SHA-256 | Golden SHA-256 | Office environment | Ratchet |
| --- | --- | --- | --- | --- | --- |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/data_point_inherited_color.docx` | `c63c9be0237bb472ec6478fab543651f4b2c3bfd1003fb8a919e101d38965b04` | `a0983d9160d159355a674d4f0a797d6be44db4f7a090f7dcd72e9d0ec350abc3` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 988 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/piechart_deleted_legend_entry.docx` | `5e62bfd50b689dfa9d8c37db1c973fc3c30bf0cdb6bcaabb0f8fff7957ddc0fd` | `4cf397d4875065337720c0f7d0ad62f35fc8b2f3b84274faed0982f6ccc96094` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 989 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/tdf123206.docx` | `d9f89075f45a4bbf47a483b494d5807a6a892585568807a899ea7024513794f1` | `0fb5f08cc51a6688193b142af0658323f50cd27594ba856915cc9763723de122` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 990 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/testChartTitlePropertiesColorFill.docx` | `f952d917a81c556e10d599ff0703cd9816f97278f166e2de28667734a9f31106` | `85ec051c0feea86c303c63cadcd4c10e5b4432995171a717ce4d2ad4eaabe561` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 991 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/testChartTitlePropertiesGradientFill.docx` | `661f3cd72751453940c830e2fa8bc516a61399868fb2a404e79b7dd518870d7f` | `6b382105035d59f42cb1e2bc404170191ec996b134edb9dbac5ce955282e43ea` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 992 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/testChartTitlePropertiesBitmapFill.docx` | `96b3d772b0a8a25d2d28c59b1c0bdae47cd75c7c30fcc2786966155b3ea245c1` | `a0efc592b6cb174cccdd9f3349d788f6c76ce2124983fe5f43f952d2a625a7c3` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 993 |
| 2026-07-23 | `LibreOffice/sd/qa/unit/data/pptx/chart_pt_color_bg1.pptx` | `11b8fbd9710c79a9e0d7bc466f27a071dc7a8f0de3078eaf79e7967af46d5c1e` | `685cedd3d2752707567e402a57401db937d7f61de47acaaf1cee829ecccb69fe` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | PPTX 349 |
| 2026-07-23 | `Apache-POI/poi-examples/src/main/java/org/apache/poi/examples/xslf/pie-chart-template.pptx` | `3b6404b59b24cb79fbb91fc2e92bd8b80cdc340aeed71a1ec1e267db0d8ad444` | `f6a120f11f40bfef07036b7cd24ae8b69bd1f7fa77958c01b5e0bd46509bca3c` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | PPTX 350 |
| 2026-07-23 | `Apache-POI/test-data/slideshow/pie-chart.pptx` | `3b6404b59b24cb79fbb91fc2e92bd8b80cdc340aeed71a1ec1e267db0d8ad444` | `122fcfb22a41317c7d571d3186df7ba96f6cd73e5eadabb31d5f4cd070500eef` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | PPTX 351 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/pie_chart_datapoint_explosion.xlsx` | `362150ac19673a8829bcdb5a5a93a4934cbc5bdbd5fe45f71cbfadd3fa4dc36c` | `fabb2eda424487c22e7147f9c6e2db1b4004d382a64fe4af6979a3a780fc79df` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 231 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/barOfPieChart.xlsx` | `4baf42c7703e502ea000b7662bee1e5eea1964ace1fd49791be2fe920d8b30ab` | `bce43b2f2b286d833f04276e4142b83b5225e869558761f27345c0d58359afd6` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 232 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/barOfPieChart2.xlsx` | `00a235ef6241be17f5fba9688bf24591b1dc3956a855f6b7909cb9fb29f0b615` | `b5a31efc48fb4462587a227e2d9bedaa803251a95b6c37e90f94f4e19de0b223` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 233 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/pieOfPieChart.xlsx` | `7d3363fef6e63574bd317aad786c39af0570e7b0957d0bffaa206f2bf1ad6407` | `5ab335dfe005ee4d04525bd241146934449bbf6c7b408e4f298f443d15a7eddb` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 234 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/pieOfPieChart2.xlsx` | `60ae5248cd26cc83a6922d9955007ebff0e77ed8593e9382c22fd513d89456b6` | `2195ffb2beb32fbe13c6d5a596ed982fdbdf77cebb332b5f63c6c53ccb7a7e9e` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 235 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/testChartTitlePropertiesGradientFill.xlsx` | `367f7b9a94307b9696fe63ce5be68c1f57f899395faba3b4e61a2000c0e831cd` | `8d42da2196aca445ab9118b7482c541c2d8e55a789586d443174c748a9a468da` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 236 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/testChartTitlePropertiesBitmapFill.xlsx` | `1c7f11ebedeebe40d4fc6f3dfbbb068c571597bf9dd0595496e6678b15a346eb` | `b2b51c1fe85ed9df8e3f7c9094a62a18c138b5a52b7b2c410591b33d60169c8e` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 237 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/testChartTitlePropertiesColorFill.xlsx` | `a9b41bbacc366480a0cb3de3b319dd5bbdf8d8941aef0be59247e912861da3b3` | `8e8d120f04a83487a3a3f41c651c990219b889229d92f36e07a1875e138e906a` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 238 |
| 2026-07-23 | `LibreOffice/sc/qa/unit/data/xlsx/databar.xlsx` | `59a5eb0735e1a234eeb3bb0d9c9a0b6955ec8db9648bfb47bd0e8a83cedce6f6` | `0793463565d3fef23a87810a1e9fd9bbcfc4b81a97ef15ada010bb2c7d1f2d80` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 239 |
| 2026-07-23 | `LibreOffice/sc/qa/unit/data/xlsx/universal-content.xlsx` | `2ea90a5abfba7b6fb4f46a9dfdacef244dff1809620d6064a7e08314f6d09a18` | `0e6009436be5324f45a54854103a0ddc743dca90c757d0d8c876fee74c869c23` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 240 |
| 2026-07-23 | `LibreOffice/sc/qa/unit/data/xlsx/tdf115159.xlsx` | `98053ff4bab193b5cc1a8a86e0f00fad72321988507efbf1f70b173f0c19a533` | `50361815e1418dc71afa0c02dcc9d21b725d5e844e4367a46e7273c6e9bc3d06` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 241 |
| 2026-07-23 | `Apache-POI/test-data/spreadsheet/RepeatingRowsCols.xlsx` | `ff67241b278977c97f8f540fe935e087763db0c0e404bd71777f913a3d698c0b` | `d7cff47f83249512f174a6935ad870f6edcabb6d46f5f9dd3c8bd7e3349a32cf` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 242 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/axis_character_properties.xlsx` | `339eda3e69ab6d089dec33b59aef59813bdbe8963a2a4bfe33bf2f7bf1f151dd` | `1b3c59f79e69e48c3a2db49b3d9e5d67d54717ed9c174bfed7179787c8b0bb28` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 243 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/majorTickMark.xlsx` | `4883a9e5899d55b5aa4a865437ba1f3a5a19551d63a519645f4c708465c486a9` | `b748878e85045a6f17ce4a33b0ba1259e69f1110abdc9ec893c7085ced5db4f8` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 244 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/minorTickMark.xlsx` | `acf1d82ce0e62de44c57d9a8ff34396ca8572f2fdeb9ac1ea54a22ae70f1fd3f` | `32347a89abd5bd7e194cce067c0b90364ee80629b73f071da722ec3723adc6ae` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 245 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/xlsx/plotVisOnly.xlsx` | `ba56f031019d2c5fbc168f86edf09f6532f503e0b83b5a7ae57be625b637f7ce` | `66ac768677290e034c2e1d5796d7cc27ebe408a09fd9ff59ea7711ef5519209c` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 246 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/hidden_cells.xlsx` | `e620daf6366001e0ed4cc4906c5da9ce8f2d890989e7f2fb48e2e46edd32c1d2` | `9616677e4199c24edcb256c4f023a91e38eabe90624386a29037a94e191a118d` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 247 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/chart-area-style-border.xlsx` | `744c9484f03974ab7de1599698fbdc0d8ce4f441d06c42d39f9d427af1efed49` | `9f5e81e8ba06f1bab89a33b1ae393c11da0602c0db9399481a44c86b3e523ad7` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 248 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/autotitledel_2007.xlsx` | `2fb5e9035a28a09fe3344112e61c9031d4611903d0202c7352af8b85c64ed6ff` | `24795bd4ba9356ee22f273ebfef6b05587c7f39329b7c96cd02344e7d3698b65` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 249 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/dispBlanksAs_2007.xlsx` | `aab81ddc6d38bdcfc39d1d1754619b342661172c87f64f3d0272163782090439` | `2524821dc3d43ab0ae14d2aaf7c6adb1edf2921dfdae5a9ad6f17bb5e42b5817` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 250 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/dispBlanksAs_2013.xlsx` | `30ebdb0a9e79d02e3fd46335d1a79f723043084085ae23491e45cde073f92bb5` | `3a8cef190db45240da7a26a6ffd48f5855acceca8e2cf5fb426cd3d18f5e125e` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 251 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/smoothed_series2007.xlsx` | `17770f1ce46ee2035545c6635a1ae336ef2858b8772149426713df46520212cf` | `4759490b84821bfe34dad06ce17fdb3e2b1b1b9909d251889714561a3c6539f8` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 252 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/smoothed_series.xlsx` | `27d955c3faa274d5217f7a145b787850038a00c4f00c85e7eeee80527378b918` | `34e06757b8fbac091293a259dc3fb57a98083e431012fe3f533db216decf2d9d` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 253 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/autotitledel_2013.xlsx` | `99d01c24b6f223ca84965c89553687af1f04195efc13775b99c4a72735e06b15` | `3102da3e8803569777156d08e65a75b6416772b3e7380f9f83e02d618ce23b80` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 254 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/data_labels_fill_color.xlsx` | `7dfe226ac3eb2a50bc55839882e633dd069ef58d58b8d9f6d1aae1f833e18c19` | `3aa3a06b9af23b0b6642b4513b41d49f3826a4d8d4a4f36af15cb2b66f3929e5` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 255 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/tdf90876.xlsx` | `80d399562d9e57c9bcdbe44f8d19e27b3ea83e88dead0bea7ab1afdd7f0165c5` | `a974e12a4f730b4c56ef408c915d46265d331be991d7188dad7d4281172e5375` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 256 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/tdf115012.xlsx` | `bf4c9c27a30507b28b28ab87d8cbbab580f70c32e1be5c61036b895b9f4ec9bd` | `e4e26c1882497967867baf95615f3e9491d534c894a4b973d973190ef834414d` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 257 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/no_marker.xlsx` | `0c0f6ba5075db7990a8d59673d7dd34d372e487642be32c2d3d718856d054de4` | `0076626d7210fb4e1fb6be4147087165101fd4688d27fd7ba8d52c450898b53c` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 258 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/chart_title.xlsx` | `4db87f036e68122eb7175110044acb3cd4d572b25289a4dc8d42033033cddad3` | `abc442a9f50a140f66bb8afc2f8586d82392b8ef6b3603fe3766cff9a00c3b2e` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 259 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/piechart_legend.xlsx` | `66b9c4a1ce83b6d6bd8d666fbfcf73c1391dd87c3f81e9908bfb8a564809cfd8` | `2f4a3be0072a1dd169500242889f48511c38a3cd4266b7d4e823f1f79486997a` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 260 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/piechart_deleted_legendentry.xlsx` | `865627ad47e00e58c60428082a060a759a24b91b8ccb6013826da04452366c18` | `963acc4a0b216c349e9c7cc90e4737aa9544ae930cb2357f76ba3eed6b4066ee` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 261 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/tdf122031.xlsx` | `40a121031943456308d9f6c11c4001c0d2d7b8a3cd90ac4054a4237ba83ae9dc` | `afbf60c14620e67e07ed50edf92edff9626a0969488b7d25081ab9d948527eae` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 262 |
| 2026-07-24 | `Apache-POI/test-data/spreadsheet/60509.xlsx` | `9890bc6a8630a0c2e371ae643b2743395adf3fc7c49e289799c853919105d99d` | `cabd0ed781731ba67a5ae2d20741ace6667d237bcb5c66b4d0557458c28f843a` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 263 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/deleted_data_labels.xlsx` | `e0ef43eb4392b153eabb04c11ed0620e8f4500f1c96b89eeb0fa4fc5cbb24603` | `93ad4c5d2cbaa804cd519b53cc5afd7addf50e86536b5bcc39391f8d0ab6c9d5` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 264 |
| 2026-07-24 | `LibreOffice/chart2/qa/extras/data/xlsx/ser_labels.xlsx` | `bd7c2a5b8b468f0bae734abe02af2159edd559de46872c3fc5ab0b51cd160b19` | `c6c8e09e652078fd984b57494e03d3b45872d499e6e5f85d6a08bba78122ab51` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | XLSX 265 |

## Current Gaps And Next Actions

1. Add a harness-enforced identity promotion ledger so count targets cannot
   silently substitute a different passing case.
2. Continue the Word pie family: left/top legends, rotation,
   percentage/callout labels, doughnut geometry, and chart-local run styles.
3. Reclassify or fix the chart-directory audit's five unexpected text failures;
   inline-object overflow preflight removed the unexpected page-count class.
4. Add family-specific 3D chart and chart-data-table models rather than
   expanding generic cache text; these are now the largest visible chart gaps.
5. Use the diagnostic index to choose the next largest coherent text, VML,
   page-flow, or drawing cluster after chart work.

## Update Rules

Do not append another “Nth completed case” section.

After a material batch:

- update the baseline counts and verification date;
- update one domain lesson only when the behavior is reusable;
- add or revise one milestone row for a substantial subsystem change;
- add every promoted exact identity to the promotion table;
- record the ECMA section, Microsoft document/section, local source paths, and
  any online primary source used;
- record focused tests and the final affected-format ratchet;
- retain unresolved risks and unexpected audit failures;
- keep raw command output, exploratory measurements, and repeated case
  narratives out of this document.

Never replace a measured count with “mostly works”, remove historical failure
context without a replacement summary, or present LibreOffice behavior as the
Office standard.

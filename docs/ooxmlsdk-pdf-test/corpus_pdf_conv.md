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
| DOCX | 2,707 | 990 | 1,717 |
| PPTX | 798 | 349 | 449 |
| XLSX | 895 | 230 | 665 |
| Total | 4,400 | 1,569 | 2,831 |

Verified on 2026-07-23: the DOCX release ratchet passed 990 cases after the
no-legend pie data-label promotion; the latest PPTX and XLSX release ratchets
passed 349 and 230.

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

Implemented families include ordinary clustered columns in PPTX/XLSX/DOCX and
ordinary Word right- and bottom-legend pie profiles, including deleted legend
entries, plus no-legend pie value/custom-text labels. The next coherent chart
work is top/left legend positions, rotation, percentage/callout labels,
doughnut geometry, and per-run title/label styles.

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

## Promotion Records

Add one row only after an exact `XPASS` is removed, the exact case becomes
`PASS`, and the affected format ratchet passes.

| Date | Identity | Source SHA-256 | Golden SHA-256 | Office environment | Ratchet |
| --- | --- | --- | --- | --- | --- |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/data_point_inherited_color.docx` | `c63c9be0237bb472ec6478fab543651f4b2c3bfd1003fb8a919e101d38965b04` | `a0983d9160d159355a674d4f0a797d6be44db4f7a090f7dcd72e9d0ec350abc3` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 988 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/piechart_deleted_legend_entry.docx` | `5e62bfd50b689dfa9d8c37db1c973fc3c30bf0cdb6bcaabb0f8fff7957ddc0fd` | `4cf397d4875065337720c0f7d0ad62f35fc8b2f3b84274faed0982f6ccc96094` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 989 |
| 2026-07-23 | `LibreOffice/chart2/qa/extras/data/docx/tdf123206.docx` | `d9f89075f45a4bbf47a483b494d5807a6a892585568807a899ea7024513794f1` | `0fb5f08cc51a6688193b142af0658323f50cd27594ba856915cc9763723de122` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | DOCX 990 |
| 2026-07-23 | `LibreOffice/sd/qa/unit/data/pptx/chart_pt_color_bg1.pptx` | `11b8fbd9710c79a9e0d7bc466f27a071dc7a8f0de3078eaf79e7967af46d5c1e` | `685cedd3d2752707567e402a57401db937d7f61de47acaaf1cee829ecccb69fe` | `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157` | PPTX 349 |

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

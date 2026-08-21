# Configured Office Golden PDF Fidelity

This is the operating guide for advancing `ooxmlsdk-layout` and
`ooxmlsdk-pdf` against explicitly configured Microsoft Office PDFs, with a
bounded LibreOffice capability lane where Office exposes no equivalent export
control.

Keep only current verified status, reusable development/debugging practice,
and durable source routes here. Campaign reference failures belong in the
per-corpus manifests; candidate audit findings belong in its report and review
summary. Implementation details belong in code and tests; run history belongs
in Git.

## Current Campaign Status

The clean `office-ooxml-pdf-options-v1` rebaseline completed on 2026-08-18.
The size, profile, and schema of the previous 4,400-reference lane were not
carried forward.

- In scope: DOCX/DOCM/DOTX/DOTM, XLSX/XLSM/XLTX/XLTM, and
  PPTX/PPTM/PPSX/PPSM/POTX/POTM converted to PDF.
- Deferred: DOC, XLS, PPT, their PDF lane, and legacy-to-OOXML conversion.
- The plan contains exactly one deterministic assignment per one of the 5,370
  round-trip source identities. It uses campaign seed
  `ooxmlsdk-office-pdf-options-2026-08-18-v1` and is checked in as
  `corpus_pdf_conv/plan.jsonl`.
- Every Office conversion has a terminal manifest record. Only a non-empty,
  parseable PDF with matching source/configuration/environment/output hashes is
  promoted as a golden.
- Unsupported options remain visible as `U` in the capability matrix and are
  not assignable; engine-specific raw controls are recorded separately from
  candidate requests and observed PDF facts.

| Family | Assigned | Office golden | Office failed | Office timeout | Candidate PASS | Candidate FAIL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Word | 3,045 | 2,972 | 71 | 2 | 1,895 | 1,077 |
| Excel | 1,345 | 1,215 | 128 | 2 | 190 | 1,025 |
| PowerPoint | 980 | 957 | 23 | 0 | 498 | 459 |
| **Total** | **5,370** | **5,144** | **222** | **4** | **2,583** | **2,561** |

The final replay skipped all 5,370 records with zero conversion attempts,
proving plan, source, environment, manifest, and output-hash stability. The
full candidate audit accounts for all 5,370 assignments and has zero
infrastructure errors; its ordinary FAIL layers are conversion 2, font 2,
page geometry 120, text 2,100, and visible output 337. The 226 Office failures
remain explicit `REFERENCE_FAIL` records and have no golden file.

The current worktree raises the checked-in checkpoint by 24
identity-preserving PASS changes, 23 in Word and one in PowerPoint, with no
prior PASS regression.
The latest promotions include one Poppler layout-order false negative recovered
by an independent raw-order confirmation and LibreOffice `fdo73215.docx`, whose
VML group path overflow and legacy GDI textbox line fit were independently
controlled. The latter retained all 200 prior PASS documents in the 408-case
VML group/textbox set; exact configured audits and the completed full-campaign
identity diff lock the gains. The Wordprocessing Canvas batch then promoted
eight more documents by implementing the canvas-owned background and keeping
zero relative-size fallback on the host instead of suppressing child shapes.
Those ownership rules are backed by LibreOffice's WPC import path and 0%
relative-size QA, and all seven prior PASS documents in the 22-document WPC
cluster remain PASS. Finally, geometry-aware `wrapTopAndBottom` avoidance
promoted `tdf142305StrokeGlowMargin.docx`; the no-following-text
`tdf136841.docx` and prior-PASS `tdf137850_compat15ZOrder.docx` controls remain
PASS. A subsequent shared DrawingML arc fix converts the authored ray angles
to non-circular ellipse parameters, promoting both
`WPC_tdf104671_Cloud.docx` and the independent preset/custom-geometry control
`tdf144742_funnel.pptx`. LibreOffice's `ARCANGLETO` conversion and focused
funnel QA provide the source and opposite representation evidence; full-page
inspection shows only subpixel antialiasing differences. The next table-wrap
batch implements the side segment chosen by LibreOffice
`SwTabFrame::CalcFlyOffsets()` when an inline table fits beside a floating
table, while retaining the existing vertical dodge when neither side fits.
Word COM measurements for `tdf134227.docx`, a focused fit/no-fit unit control,
and the independent LibreOffice `floattable-wrapped-by-table.docx` fixture all
agree; both documents are now PASS. The following WPC table-cell batch separates
a table cell's stored baseline from the paragraph line top used for floating
object collision, preserving that offset when `wrapTopAndBottom` moves a line.
It also converts the preset text-rectangle top to a baseline only for a direct,
centered, overflowing WPS story whose host is owned by a real table cell.
LibreOffice's `wpc_drawing_canvas.cxx`, `WpsContext.cxx`, and `svdotext.cxx`
establish the ownership and vertical-anchor path. Word COM measured the cell
and paragraph top at 96.75 pt and the canvas at a paragraph-relative 16.5 pt;
GDB independently exposed the candidate's 107.266 pt stored baseline, 96.792 pt
line top, and 113.292 pt wrap boundary. The line box therefore does not collide,
while the old baseline-as-top comparison did. The direct and opposite controls
include `WPC_tdf48610_Textbox_with_table_inside.docx`, `fdo74401.docx`, and nine
previously passing body-owned WPS documents. All remain at their prior verdict,
while `WPC_tdf158348_shape_text_in_table_cell.docx` changes from text FAIL to
PASS. Candidate, Office, and diff pages were inspected. The focused layout and
PDF libraries pass 1,109 and 61 tests, respectively. A reconstructed old-rule
control audit and the final 5,370-assignment audit differ at exactly that one
configuration identity, with no PASS-to-FAIL change and zero infrastructure
errors.

The balanced 300-case go gate completed before full generation: 282 Office
goldens, 18 reference failures, 85 candidate PASS, 197 ordinary FAIL, and zero
infrastructure errors. The earlier three-source hard-PASS calibration remains
useful as a narrow option-adapter regression, but is not the corpus baseline.
Detailed first-round decisions and representative visual cases are in
`corpus_pdf_conv/review-cases.md`.

## Capability Matrix

`ooxmlsdk-pdf` exposes `PdfOptionFeature::ALL`,
`pdf_option_support(document_kind, feature)`, and
`resolve_pdf_options(document_kind, requested)`. Resolution happens before
layout for every DOCX/XLSX/PPTX conversion entry point. It returns effective
options plus deterministic adjustments, and rejects invalid or unavailable
requests with a typed error.

Legend: **S** is implemented and observable in output; **R** is implemented
with the stated restrictions and may be assigned only when those restrictions
are satisfied; **U** is not assignable.

| Option family | DOCX | XLSX | PPTX | Current boundary |
| --- | :---: | :---: | :---: | --- |
| PDF version | S | S | S | PDF 1.4/1.5/1.6/1.7/2.0 backend selection |
| PDF/A | R | R | R | A-1/A-2/A-3/A-4 variants supported; deterministic creation date required; conformance compatibility still applies |
| PDF/UA | R | R | R | UA-1 request forces tags, outline availability, and display-title preference |
| content-stream compression | S | S | S | changes serialized page streams |
| fixed-format optimization | S | S | S | print is the default; screen caps bitmap-backed fixed-output surfaces at 96 DPI independently of explicit image downsampling |
| UI language | R | R | R | valid BCP 47; generated UI strings have English and Chinese packs, otherwise recorded English fallback |
| format locale | S | S | S | valid BCP 47; number/date/currency formatting only |
| document language | S | S | S | valid BCP 47; authoring defaults and PDF document language |
| field update date/time and time zone | R | R | R | valid civil time and IANA zone; zone is inactive without an update time; only fields with implemented refresh semantics change |
| paper-size override | U | U | U | authored page/slide/sheet sizes are honored; there is no conversion-time override yet |
| tagged PDF | S | S | S | structure output is wired; standards may force it on |
| bookmarks/outlines | S | S | S | export and open-level resolution are wired |
| page range | S | S | S | LibreOffice-compatible sequence grammar, including lists, open ranges, reverse ranges, and duplicates; dependent page references are remapped |
| skip empty pages | U | U | U | layout cannot yet distinguish application-inserted blank pages |
| transparency flattening | U | U | U | no document-wide flattening pass yet |
| image compression | S | S | S | lossless/JPEG policy and JPEG quality are effective |
| image downsampling | S | S | S | enabled DPI is validated in the 51–2400 range |
| links | R | R | R | URI and remove-external modes work; relative-file rewriting, remote destinations, and launch actions are rejected |
| form fields | R | U | U | DOCX AcroForm widgets only; PDF submit and unique names only |
| viewer preferences | S | S | S | catalog page mode/layout, open action, direction, and window preferences are serialized |
| metadata | S | S | S | document-info/XMP fields and deterministic creation/modification date are serialized |
| attachments | R | R | R | embedded files and associations work subject to selected PDF standard |
| watermark | U | U | U | requires shaped Unicode text and archival-safe embedding |
| single-page sheets | U | U | U | spreadsheet pagination and drawing-scale integration are missing |

`PaperSize` remains a capability identifier so planners cannot silently assume
an override exists. It has no request field until layout can implement it.
Where an unavailable request field already exists, the resolver returns
`PdfError::UnsupportedOption`; it never accepts and ignores the value.

## One Source, One Configuration

The cardinality is a data-model invariant, not a generator convention:

```text
(corpus, normalized source path, source SHA-256)
  -> exactly one assignment record
  -> exactly one terminal conversion record
  -> zero or one promoted reference-engine PDF
```

- A plan or manifest is keyed by the source identity above. Duplicate source
  identities are invalid; there is no `variants[]` collection beneath a
  source. Changing an assignment creates a new versioned campaign record that
  replaces the old assignment rather than appending another variant.
- The Rust generator enforces this invariant for the complete plan, validates
  exact source bytes/SHA-256, rejects duplicate source/configuration identity,
  and marks exactly 100 sources per family for the 300-case go gate. The Office
  supervisor gives the checked-in PowerShell adapter a one-record plan under
  WSL `/tmp`; the adapter independently rejects duplicate/unknown/missing
  fields and refuses a non-empty output directory.
- Every promoted record must preserve the requested candidate options, resolved
  effective options and adjustments, exact Office/LibreOffice raw parameters,
  reference engine/version/build, environment identity, source/output hashes,
  selected pages, and observed PDF facts such as header version, tags, and
  PDF/A XMP. Raw request names are not treated as observed conformance.
- Assignment is deterministic and quota-balanced rather than an unrepeatable
  RNG walk: sources are partitioned by document family, marginal values are
  balanced, sources are stable-shuffled with the versioned seed, and exactly
  one row is consumed per source. A repeated run reproduces the plan byte for
  byte.
- A source that Office cannot open or cannot export with its assignment gets
  one terminal failure record, not a fallback configuration or second
  reference. Reassigning it requires a new versioned campaign. Coverage is
  measured across the 5,370 sources, not by multiplying variants per source.

The locale pool is intentionally compact:

```text
zh-CN, zh-TW, ja-JP, ko-KR, en-US, de-DE, fr-FR, es-ES
```

These values apply to format/document locale without injecting translated
content. UI-language assignments remain English/Chinese until the generated UI
resource packs exist for the other languages. Time zone and field-update
assignments require a matching, recorded reference-engine environment; Office
culture/time zone are environment strata rather than fictional per-call COM
arguments.

## Reference Engines And Phase Gate

Office remains the primary visible-output engine because it renders Office
documents most faithfully. Its per-call option intersection is narrower than
the candidate API. LibreOffice is a later, bounded option-capability lane for
controls Office does not expose per call, using isolated user profiles and
explicit filter data. The source partitions are disjoint so the one-source
cardinality rule still holds.

Mass generation may begin only after all of these are true:

1. every matrix feature has an `S`, `R`, or `U` answer for each document kind;
2. requested/effective/raw/observed configuration can be recorded without
   hidden defaults;
3. engine adapters reject fields they cannot apply;
4. object-level tests prove non-visual controls and page tests prove visible
   controls;
5. at least one configured DOCX, XLSX, and PPTX case is a hard PASS; and
6. the deterministic assignment generator rejects duplicate source identity.

This gate is now satisfied for the Office lane. The runtime matrix, explicit
rejection, object/page tests, hard-PASS calibration, balanced assignment model,
unified promotion record, 300-case go gate, complete Office conversion, hash
replay, and full candidate audit are all in place. LibreOffice remains a later,
disjoint capability lane for controls that Office cannot expose per call.

## Contract

```text
OOXML package
  -> import/effective model
  -> layout display list
  -> candidate PDF
  -> layered comparison with the configured reference PDF
```

- An accepted reference PDF and its conversion record are immutable within a
  campaign. A rebaseline replacement must come from a newly recorded Office or
  LibreOffice export; never regenerate or approve-update one from candidate
  output.
- Microsoft Office fixed output is the visible target. Microsoft documents and
  ECMA-376 explain behavior; LibreOffice and other projects provide source and
  counterexample evidence.
- Fix the earliest incorrect layer. Later visual resemblance cannot compensate
  for wrong import, text, font, pagination, geometry, or ownership.
- A PASS satisfies independent identity, page, text, font, line, geometry,
  image-placement, and visible-output checks. The raster check masks text only
  after the text contracts pass, and combines a per-channel delta cutoff of 16
  with 1% global/localized pixel-fraction and 1.5 mean-channel limits.
  `office_golden.rs` is the source of truth; 1% is not the whole contract.
- Every promoted PASS still requires complete candidate/Office page inspection.
- Never weaken thresholds, exclude a source, or relabel a failure without
  evidence.
- Run Cargo commands sequentially in the owning repository with the default
  `target/` directory.

## Evidence-First Development

Evidence priority:

1. recorded Office output tied to the exact source, options, and environment;
2. Microsoft Open Specifications and Office documentation;
3. ECMA-376 and other normative format specifications;
4. LibreOffice production code and nearby QA;
5. independent implementations and fixtures.

Office output decides visible behavior. LibreOffice is the primary open-source
DOCX/PDF/layout reference, not the Office specification. Open-XML-SDK is the
package/schema/API reference, not a renderer. Generic libraries may provide
mechanisms; Office policy stays in the OOXML adapter.

For each gap:

1. verify the fixture identity, package parts, relationships, and authored
   property presence;
2. inspect complete Office/candidate pages and identify the earliest failing
   comparison layer;
3. find the semantic owner in local specifications, production source, and QA;
4. find a same-state failure plus an opposite-state PASS or upstream test;
5. trace package -> import -> cascade -> model -> layout -> display list -> PDF;
6. use GDB and PDF/image artifacts to prove raw value, resolved value, branch,
   owner, resource, and final coordinate;
7. implement the complete sourced state chain at its owner and add focused
   positive/negative tests where golden output cannot preserve the boundary;
8. predict the first diagnostic to disappear, then run one exact golden, its
   coherent cluster, and a stopping counterexample.

Missing functionality comes first. Use elimination on later layers only after
the feature chain is complete. Golden tests are acceptance instruments, not
search instruments: repeated rendering and parameter tuning overfit hidden
state. If output contradicts a debugger-backed prediction, return to sources
and GDB.

When no source answers the question, record the searched routes, one unresolved
question, two opposite predictions, and one bounded experiment. Do not infer a
general rule from one fixture.

Experiments remain valid elimination tools even before the semantic owner is
fully identified. Give each experiment an explicit hypothesis, opposite
predictions, a reversible change, and a positive/negative observation that can
rule something out. A large pixel delta, many affected pages, or a moved first
mismatch does not by itself make a case unsuitable: one broadly repeated
feature gap can produce all three.

Stop and rotate only when successive experiments no longer narrow the
hypothesis space, add no new source or debugger evidence, or turn into
case-specific parameter or branch tuning. When a same-state counterexample
contradicts a proposed general rule, revert that rule and record the narrower
unresolved boundary; it is evidence against that hypothesis, not a ban on a
new discriminating experiment. This stopping rule prevents fitting one
complex fixture without suppressing evidence-producing experiments.

## Source Map

Search local checkouts and converted documents before browsing. The Markdown
files in `references/references/` are the searchable copies; do not reconvert
their source documents unless the copy is demonstrably defective.

| Need | Durable route |
| --- | --- |
| ECMA Word, DrawingML, charts, math, MCE, OPC | `references/references/Ecma Office Open XML Part *.md` |
| Office deviations/defaults/extensions | local `[MS-OI29500]`, `[MS-DOCX]`, `[MS-OE376]` Markdown |
| Microsoft Open XML/API guidance | `../open-xml-docs/`, Microsoft Learn |
| Office fixed-format export policy | `scripts/convert_office_corpus.ps1`, `scripts/probe_office_pdf_options.ps1`; Microsoft Learn [`Word.Document.ExportAsFixedFormat`](https://learn.microsoft.com/en-us/office/vba/api/word.document.exportasfixedformat), [`Excel.Workbook.ExportAsFixedFormat`](https://learn.microsoft.com/en-us/office/vba/api/excel.workbook.exportasfixedformat), [`PowerPoint.Presentation.ExportAsFixedFormat`](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.exportasfixedformat), and [Extending the Office fixed-format export feature](https://learn.microsoft.com/en-us/office/pdf/extendingofficepdfexport) |
| Archived Microsoft Open XML examples | `../msdn-code-gallery-microsoft/` |
| VML, GDI, and Win32 contracts | `../win32/`, especially `desktop-src/VML/` |
| Windows rendering, printing, and XPS probes | `../Windows-classic-samples/`, `../Win2D/` |
| Package, schema, validators, fixtures | `../Open-XML-SDK/` |
| Independent WordprocessingML transforms and content-control handling | `../Open-Xml-PowerTools/` (archived Microsoft guidance/example code; supplemental implementation evidence, not Office rendering policy) |
| DOCX import and visible layout | `../core/sw/`, `../core/oox/`, matching `qa/` |
| XLSX import and print layout | `../core/sc/source/filter/oox/`, `../core/sc/source/ui/view/printfun.cxx`, `../core/sc/qa/` |
| PPTX import and fixed pages | `../core/oox/source/ppt/`, `../core/oox/source/drawingml/`, `../core/sd/qa/` |
| LibreOffice PDF export | `../core/vcl/source/pdf/`, `../core/filter/source/pdf/`, `../core/officecfg/registry/schema/org/openoffice/Office/Common.xcs`, `../core/vcl/qa/cppunit/pdfexport/`; official [PDF export initial-view help](https://help.libreoffice.org/latest/en-GB/text/shared/01/ref_pdf_export_initial_view.html) |
| PDF catalog, viewer, action, metadata, and conformance objects | [Adobe PDF Reference 1.5](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.5_v6.pdf), [Adobe pdfmark reference](https://opensource.adobe.com/dc-acrobat-sdk-docs/library/pdfmark/toc.html), `../krilla/`, and object-level `lopdf` tests |
| OpenType contract | current Microsoft OpenType pages; `../OpenType-Specification/` is a historical local mirror |
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../parley/` |
| PDF comparison and output mechanisms | test code plus `../pdfium-render/`, `../krilla/`, `../typst/crates/typst-pdf/`, `../cairo/`, `../tiny-skia/` |
| Geometry and neutral graphics types | `../kurbo/`, `../color/`, `../peniko/` |
| EMF/WMF/GDI/OLE/CFB | local Microsoft specs, `../emfsdk/`, `../olecfsdk/`, `../libgdiplus/`, `../reactos/`, `../wine/` |
| Locale and fallback mechanisms | `../icu4x/` |
| CJK/ruby counterexamples | `../clreq/`, `../klreq/`, `../simple-ruby/`, `../i18n-tests/`, W3C JLREQ |
| OfficeMath/UnicodeMath semantics | ECMA-376, Microsoft deviations, `../UnicodeMathML/` |
| Independent spreadsheet behavior | `../poi/`, `../EPPlus/`, `../ClosedXML/` |

Use DirectWrite APIs and official samples as Windows behavior probes, not
portable fallback tables. OpenType defines face data, coverage, metrics,
outlines, and shaping tables, but not system or Office fallback order. PDFium,
Krilla, Cairo, tiny-skia, and graphics crates provide mechanisms, not Office
policy. ReactOS and Wine are counterexamples, not Windows authorities.
EPPlus is source-available under its own license: use it as independent
evidence, not code to port.

`../msdn-code-gallery-microsoft/` is a useful archived official-sample search
route. The five community gallery mirrors are last-resort leads whose origin
and license must be verified per sample. `../vello/` is renderer research, not
evidence for current golden behavior.

Search exact XML elements, properties, records, APIs, and diagnostics together
with nearby production code and QA. State narrow boundaries when sources
disagree; do not present an implementation choice as normative.

## Fast Test Loop

Run from `../ooxmlsdk-test-suite`; use debug builds for investigation and
`--release` for golden acceptance. Use one calibrated host for comparable
counts: PDFium must bind through the system library or
`PDFIUM_DYNAMIC_LIB_PATH`, and candidate font availability must not change.
`environment.json` fingerprints the reference Office conversion environment,
not the candidate host.

### Configured Campaign

The campaign binary owns plan validation, font scanning, Office supervision,
manifest promotion, hash replay, and candidate audit:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
./target/release/office_pdf_campaign validate-plan
./target/release/office_pdf_campaign audit \
  --selection full --timeout-seconds 180
```

On a host where WSL cannot launch Windows COM children directly, start the
`convert` command through Windows PowerShell and `wsl.exe`; substitute the
calibrated distribution and suite root rather than checking machine-local
paths into this document. Re-running `convert --selection full` is the required
integrity replay: a stable complete campaign reports 5,370 skipped records and
zero attempts.

Use `audit-one --write-artifacts true` only for bounded visual review. Full
audits do not emit thousands of page images by default.

### Windows Office Probes

Use Office after the Source Map reduces a gap to a bounded behavior question;
use GDB first when the candidate's resolved state or branch is unclear. A probe
supplies same/opposite-state evidence and does not by itself establish a
general implementation rule.

The checked-in probe scripts require PowerShell 7 with `-STA`. They accept only
pre-existing WSL `/tmp` output directories, refuse existing outputs, open
corpus inputs read-only with macros disabled, and never delete, move, or
overwrite corpus or promoted reference data. The option probe moves its private
staging directory beneath the `/tmp` worker output for postmortem inspection;
it does not delete it. Do not use PowerShell for networking or unrelated
system access. One corpus case:

```sh
probe_dir=$(mktemp -d /tmp/ooxmlsdk-office-probe.XXXXXX)
printf '%s\n' 'Open-XML-SDK/path/to/case.docx' > "$probe_dir/cases.txt"
pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass \
  -File "$(wslpath -w "$PWD/scripts/probe_office_word_corpus.ps1")" \
  -CorpusRoot "$(wslpath -w "$PWD/corpus")" \
  -ListFile "$(wslpath -w "$probe_dir/cases.txt")" \
  -OutputRoot "$(wslpath -w "$probe_dir")"
pdftoppm -f 1 -singlefile -png -r 150 \
  "$probe_dir/case-000.pdf" "$probe_dir/page-1"
```

For text/font/bidi counterexamples, use
`scripts/probe_office_word_fragment.ps1`; vary paragraph and run directions
independently, then inspect both `fragment.docx` XML and `fragment.pdf` text and
pages. The scripts follow Microsoft's contracts for
[`Documents.Open`](https://learn.microsoft.com/en-us/office/vba/api/word.documents.open),
[`ExportAsFixedFormat`](https://learn.microsoft.com/en-us/office/vba/api/word.document.exportasfixedformat),
[`RtlPara`](https://learn.microsoft.com/en-us/office/vba/api/word.selection.rtlpara),
and [`RtlRun`](https://learn.microsoft.com/en-us/office/vba/api/word.selection.rtlrun).
Keep every probe under `/tmp`; never copy it into `corpus_pdf_conv/`.

For the configured three-format pilot, use
`scripts/probe_office_pdf_options.ps1`. Its JSONL plan has exactly
`schema_version`, `file`, and `options`; the options object has an exact schema
for quality, page range, tags, bookmarks, PDF/A request, document properties,
bitmap fallback, and hidden slides. The script rejects duplicate sources and
application options that the selected Office COM API cannot apply. It emits
one PDF plus one configuration/hash record per source. The ignored acceptance
test consumes that directory without modifying it:

```sh
cp scripts/office_pdf_options_pilot.jsonl /tmp/<probe>/plan.jsonl
pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass \
  -File <probe_office_pdf_options.ps1-windows-path> \
  -CorpusRoot <corpus-windows-path> \
  -PlanFile <plan-windows-path> \
  -OutputRoot <empty-output-windows-path>
OOXMLSDK_OFFICE_PDF_OPTIONS_PROBE_DIR=/tmp/<probe>/out \
  cargo test -p ooxmlsdk-pdf-test --test pdf_options \
  office_pdf_options_probe_is_a_hard_pass -- \
  --ignored --exact --nocapture
```

Always inspect the emitted PDF header/catalog/XMP and full rendered pages. In
the current Office build, Word's raw `UseISO19005_1=true` request produces XMP
declaring PDF/A-3A; the record therefore keeps the raw COM request separate
from the observed standard and maps the candidate to `PdfA3a`.

Build the campaign binary after implementation changes, then prepare and audit
one exact configured identity by configuration ID. `prepare-audit-one`
validates the complete plan plus the matching conversion environment and
golden hash before writing the single task; it does not require a prior full
audit or a populated `target/` directory.

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
case_id='<configuration-id>'
task="/tmp/ooxmlsdk-$case_id-task.json"
result="/tmp/ooxmlsdk-$case_id-result.json"
./target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id "$case_id" \
  --task "$task"
./target/release/office_pdf_campaign audit-one \
  --task "$task" \
  --result "$result" \
  --write-artifacts true
jq -e --arg id "$case_id" \
  '.configuration_id == $id and .verdict == "PASS"' \
  "$result"
```

`audit-one` records ordinary comparison failures in JSON and therefore can
exit successfully while the recorded verdict is `FAIL`; the final `jq -e`
check is part of the acceptance command, not optional display. The configured
campaign reports a fixed identity directly as `PASS` or `FAIL`. Do not
substitute the default-options `office_golden_corpus` ratchet: it uses a
different option contract and a separate `golden-errors.toml` ledger, and is
not an acceptance path for this campaign. Reports are valid only after their
reporting phase completes; check timestamps and audited revisions after an
interruption.

## Diagnostics And Inspection

| Diagnostic | First owner to inspect |
| --- | --- |
| identity/open/extraction | manifest, package, parser, feature gate |
| page count/geometry | sections, breaks, page layout, printable region |
| text content/order | import, visibility, fields, model |
| text style/font | cascade, theme/font slot, locale, fallback |
| line count/content | shaping, wrapping, tabs, bidi, paragraph construction |
| bounds/baseline | metrics, alignment, frame ownership, layout |
| font integrity | face, glyph, cluster, embedding, `ToUnicode` |
| graphics | host geometry, transform, clip, paint order, image/metafile |
| visible output | display lowering, PDF paint, raster/backend |

Inspect package state before effective state. Keep absent, explicit false,
inherited, and defaulted distinct even when they currently resolve alike.

Inspect output in this order:

1. normalized text, order, and page assignment;
2. style, selected font, and line reconstruction;
3. bounds, baseline, frame/clip ownership, and transforms;
4. glyph/CID mapping, widths, embedding, `ToUnicode`, and `ActualText`;
5. non-text paint, images, masks, and composition.

Failures write bounded evidence under `target/office-golden/`: exact/audit
JSONL, diagnostic index, candidate PDF, page images/diffs, font selection,
glyph traces, and PDF font audit.

A comparator `PASS` is not visual proof because the threshold permits finite
error. For every promoted case, enable page artifacts, verify their timestamps,
and inspect every full candidate/Office page side by side. Check page count,
content, clipping, paint order, table/drawing edges, and text baselines; then
inspect a crop around the repaired feature. Exit status and diff images are
supporting evidence only.

Keep coordinate contracts explicit across layers. Trace physical frame edge,
print edge, line box, ascent, display-list coordinate, owner, PDF matrix, and
clip separately. Likewise keep natural content height, resolved line height,
flow advance, and page-fit boundary separate. A change in ownership can expose
an obsolete offset even when the outer geometry is unchanged.

Topic-specific evidence routes:

| Area | State to trace | Primary evidence |
| --- | --- | --- |
| repeating stories/tables | story frame, cell print edge, ascent, item owner, PDF baseline | ECMA §§17.10, 17.4, 17.6; `[MS-DOCX]`; LibreOffice page/table/text frames |
| line spacing/pagination | natural height, eligible text base, gap, full advance, page bottom | ECMA §17.3.1.33; LibreOffice `CalcLine()`, `CalcRealHeight()`, widow/orphan code |
| footnotes/endnotes | special IDs, story/section owner, separator, area top, body bottom | ECMA §17.11; `[MS-OI29500]`; LibreOffice `ftnfrm.cxx` |
| fonts/fallback | four Word slots, theme/cascade, requested and realized face, coverage, metrics | ECMA §17.8; DirectWrite probe; OpenType; fontations/parley |
| CJK/ruby | locale/region, break class, punctuation compression, justification, hanging, vertical state | Microsoft deviations, Office output, CLREQ/JLREQ/KLREQ |
| OLE/metafiles | relationship, host rectangle, native/cache payload, records, GDI state, surfaces, PDF object | Microsoft binary specs, emfsdk/olecfsdk, LibreOffice |

These rows route investigation; they are not permission to copy another
implementation's policy. Preserve same-state and opposite-state examples for
every generalized rule.

### GDB Workflow

Use GDB for unexplained runtime state, ownership, or control flow. A direct
typed resolver/backend validation error already identifies its branch and does
not need a performative debugger session; fix that semantic owner, then verify
the resulting PDF objects and pages.

1. Build the campaign binary without `--release`. The suite test profile
   enables full debug info for the golden-path layout, font, PDF, metafile,
   and compound-file packages; other packages retain line tables.
2. Prepare the exact task with the same debug binary and verify both embedded
   configuration IDs before starting GDB:

   ```sh
   cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign
   case_id='<configuration-id>'
   task="/tmp/ooxmlsdk-$case_id-gdb-task.json"
   result="/tmp/ooxmlsdk-$case_id-gdb-result.json"
   ./target/debug/office_pdf_campaign prepare-audit-one \
     --configuration-id "$case_id" --task "$task"
   jq -e --arg id "$case_id" \
     '.assignment.configuration_id == $id and
      .conversion.configuration_id == $id' "$task"
   gdb --args ./target/debug/office_pdf_campaign audit-one \
     --task "$task" --result "$result" --write-artifacts true
   ```

3. `audit-one` consumes exactly one task in the current process; it does not
   spawn campaign workers and needs no `OOXMLSDK_GOLDEN_CASE` or
   `OOXMLSDK_GOLDEN_JOBS` variables. Resolve qualified breakpoints to concrete
   addresses and confirm the task ID at the breakpoint.
4. Condition on source, part, page, object, or content hash. Trace one value
   through import, effective model, layout/shaping, display list, and PDF.
5. The pipeline may render more than once; use `tbreak` or disable a breakpoint
   after the intended hit.
6. Record authored/raw value, resolved value, selected branch, owner, final
   coordinate/resource, and the first point where candidate diverges.
7. Inspect the exact written PDF and images. Run release acceptance only when
   the trace and predicted diagnostic change agree.

For crashes capture all threads, full backtraces, arguments, locals, and
`$_siginfo`. For apparent hangs, interrupt more than once: changing stacks mean
slow progress; identical blocked stacks suggest non-progress. Debug timing is
not performance evidence.

## Regression And Promotion

The configured campaign has three terminal audit states: `PASS`, `FAIL`, and
`REFERENCE_FAIL`. It does not read `golden-errors.toml` and therefore has no
`XFAIL` or `XPASS` state. Those names belong only to the separate legacy
default-options ratchet; do not use its ledger or counts when reporting this
campaign.

For a promotion batch:

1. record exact removed and introduced identities, not only counts;
2. run focused implementation regressions;
3. run each exact configured golden, its coherent cluster, and a stopping
   counterexample;
4. visually inspect every PASS and require the earliest failure scope to shrink;
5. at the phase gate, complete the configured full audit and verify an exact
   identity diff with no prior `PASS` regression or infrastructure error.

Implementation-local tests preserve private algorithms and boundaries that
golden output cannot distinguish. Fixture-backed public behavior belongs in
the test suite. Do not update a failing expectation unless independent
specification or production-source evidence proves the old assertion stale.

Never present a focused result as an exhaustive baseline. Delete obsolete
plans and case narratives instead of accumulating them here.

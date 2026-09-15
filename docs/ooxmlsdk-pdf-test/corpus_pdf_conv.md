# Configured Office Golden PDF Fidelity

Session entry point: current obligations, next repairs, evidence locations and
reusable development/debugging lessons. Keep per-attempt logs under `/tmp`.
Update this entry at meaningful stage boundaries; replace stale status instead
of appending a chronology. Keep attempt commands, measurements and validation
details in the indexed archive, and promote only reusable lessons here.

## Current progress

Accepted checkpoint: **batch 28**, **5,370 configured identities**:

| Result | Count |
| --- | ---: |
| PASS | **3,169** |
| FAIL | 2,070 |
| REFERENCE_FAIL | 131 |
| Infrastructure errors | 0 |

There are **38 additional pending PASSes**, so protect **3,207 known PASSes**,
including the user's original 3,099 baseline. New passes never cancel regressions.
Accepted counts remain unchanged until a checkpoint is promoted.

Latest complete Excel check: `xlsx-general-ledger/formula-02-excel-release/`
verified all 1,236 valid references: **651 PASS / 585 FAIL**, retaining all 651
protected Excel PASSes with zero regressions or infrastructure errors. The entry
check discovered an earlier PASS, ClosedXML `kanji-with-new-line-input.xlsx`.

Last closed repair: `xlsx-double-borders/` implements previously unused
`BorderStyle.compound`: double rails, transparent corner/intersection gaps,
shared-edge color priority and printed-grid exclusion. **244 related cases:
105 PASS / 139 FAIL**, retaining all 105 entry protected cases (including all
38 pending PASSes), with zero regressions or infrastructure errors and no new
whole-file PASS. All 266 private XLSX tests pass. Ten Office exports verify the
behavior; 2,880 corner samples and 97 color pairs match. Environments are restored.
`PivotTableWithMissingReference.xlsx` still has the same 12 failing pages; its
ordinary horizontal weights already match, leaving row/line positions to inspect.
The earlier visible-clip centering repair and its new `tdf151755` PASS remain
protected. This is the current runtime; the complete Excel check above belongs
to the earlier formula runtime.
The latest actual full campaign audit remains batch 17. Do not describe an Excel,
scoped or merged checkpoint as a fresh full campaign audit.

The earlier 12 layout, five public-font and three public-XLSX failures are closed;
do not repeatedly reassess them. Office-based assertion decisions are recorded in
[legacy assertions](legacy-assertion-resolution.md) and
[XLSX assertions](xlsx-legacy-assertion-resolution.md).

Archive root: `/tmp/ooxmlsdk-repair-resume-20260912/`.
Paths below are relative to that root unless stated otherwise.

| Need | Authoritative entry |
| --- | --- |
| Resume current work | `CURRENT.md`, `ledger/pending-golden-repairs.json` |
| Accepted identities/results | `checkpoint-batch-28-local/pass-baseline.jsonl`, `cumulative-results.jsonl`, `manifest.json` |
| Additional protected PASSes | `ledger/pending-pass-obligations.jsonl` |
| Latest scoped verification and current runtime hashes | `xlsx-double-borders/closure.json`, `xlsx-double-borders/double-02-release/`, stage `compiled-source-hashes.json` |
| Latest full campaign evidence | `batch-17-full-audit/`, `checkpoint-batch-17-full/` |
| Source/configuration preservation checks | `snapshot-batch-28-final/`, latest stage `validate_scope.py` / `scope-validation.json` |
| Individual repairs and Office experiments | Stage `evidence.md`, `office-*-validation.json`; consult the ledger before repeating work |
| Detailed history removed from this entry | `xlsx-chartsheet-geometry/guide-before-condense.md`; earlier `corpus_pdf_conv-before-cleanup.md` |

The retired 2,774/2,783 source cohorts also remain obligations through
`baseline/historical-2783-source-bridge.jsonl`: **121 mapped sources still FAIL**
in the new configurations. Changed reference/configuration hashes make these
source obligations, not measured regressions of old runtime identities. Do not
feed retired IDs from `/tmp/ooxmlsdk-golden-baseline-20260911/` into the v2 plan.

## Next work

1. Prioritize unused attributes and enum/type branches against actual failures;
   trace them through import, layout and paint before declaring a feature gap.
   `BorderStyle.compound` is now implemented and verified in
   `xlsx-double-borders/`; do not repeat the earlier ignored-compound diagnosis.
   Continue `PivotTableWithMissingReference.xlsx` with row/line positions and
   printer phase. Its 12 failing pages are unchanged by the double repair;
   native/SDK ordinary horizontal widths are both 0.96pt / 1.92pt.
   `xlsx-double-borders/evidence.md` indexes the frozen target, native vectors,
   double controls and the separate minimal-template manual-row-grid discrepancy.
   `xlsx-table-centering/double-border-failure-leads.json` retains the nine
   sources using double-border XFs; their other failures remain unresolved.
   GeneralLedger, EscapedApostrophe and tdf151755 remain protected PASS.
2. `BloodPressureTracker_TP10073878.xltx` still fails localized marker comparison.
   Source/shadow work: `xlsx-chartsheet-shadow/evidence.md`; foreground diamond/
   square pixels match, triangle has one corner residual. Printer origin and
   final raster phase remain unresolved. New read-only mapping evidence is in
   `xlsx-chartsheet-origin-grid/evidence.md` (12 Office PDFs, two XPS, five printer
   DC controls). Native printer hard margins are zero; do not interpret the
   existing empirical trim as a measured hard margin or apply fixed offsets.
   `xlsx-chartsheet-shadow/layer-order-review.json` confirms Office also paints
   marker foreground above lines. Native round line caps/joins are another lead,
   not a proven cause of the localized failure. Prior source controls are in
   `xlsx-chartsheet-marker-grid/` and `xlsx-chartsheet-marker/`; font-band/rotation
   controls are in `xlsx-chartsheet-inner/native-summary.json`.
3. During iteration, use release individual golden checks and necessary private
   tests. Accumulate repairs before a scoped/Excel audit; run full audits only when
   the combined impact warrants them. Skip public layout/XLSX suites and Clippy by
   default per user. The user commits; preserve unrelated changes. No subagents
   unless newly authorized.

Retained boundaries: Wingdings F0A8 must remain private-use text in protected Word
cases; a blanket remap caused a regression. Empty Office exports require explicit
observations, not automatic acceptance. The absent-array-member lead is closed by
`xlsx-array-members/coverage-check.json`. Further Word Han fallback experiments
are indexed in `word/address-block-resource/deep-font-fallback/round-1-conclusions.md`;
prepared probes are not completed evidence. read-fonts 0.44/skrifa 0.47 showed no
benefit for the static-font repair; retain current dependencies and the user's lock updates.

## Golden contract and provenance

Reference generation is **closed: 5,239 valid Office PDFs / 131 unresolved /
0 pending**. Recovery added 94 PDFs to the original 5,145 successes; unresolved
retries comprise 92 zero-page outputs, 32 errors and seven timeouts. Keep their
terminal records. Round-trip eligibility does not guarantee Office export success;
do not silently drop these sources or resume generation as part of renderer repair.

All 5,370 original source hashes are retained. Never rewrite corpus files or
manifests: round-trip tests depend on them. Goldens, frozen configuration IDs and
acceptance thresholds are immutable during implementation repairs. Experiments
use byte-identical copies under `/tmp`; a reference revision needs concrete Office
conflict evidence and preserved provenance.

The v2 plan uses seeded configuration coverage with a few directed exceptions:
**5,212 of 5,370 assignments export all pages (97.1%)**. Page-range and hidden-slide
controls verify actual content membership against complementary full exports.
Generic JPEG/DPI options do not imply effective Office coverage, which follows
Office quality policy. Visual PASS also does not establish metadata, tagging,
bookmarks or conformance correctness; those need functional assertions.

Successful receipts verify actual UI language from `de-DE`, `en-US`, `es-MX`,
`fr-CA`, `ja-JP`, `ko-KR`, `zh-CN`, `zh-TW`; do not relabel installed variants as
`es-ES`/`fr-FR`. Format locale, preferred authoring language and time zone are
observed host values. Schema-4 receipts retain open/refresh/export UTC boundaries;
the candidate replays refresh start in that recorded time zone. Windows clock and
settings are not changed. Authored run languages are distinct from Office defaults.

| Need | Retained source |
| --- | --- |
| Frozen assignments | `corpus_pdf_conv/plan.jsonl`; exact hash in the checkpoint's audit/build manifest |
| Adapter policy and provenance | [reference-adapters](../../corpus_pdf_conv/reference-adapters/README.md) |
| Authorized failed-reference recovery | `corpus_pdf_conv/reference-recovery-history/`, `recovery-plan-overrides.jsonl` |
| Directed configuration/content coverage | `crates/ooxmlsdk-pdf-test/data/office-functional-coverage.json`, `corpus_pdf_conv/functional-controls/` |
| Earlier experiment/repair history | `/tmp/office-golden-rebuild-20260911/pending-repairs/`; current archive `ledger/` and `corpus_pdf_conv-before-cleanup.md` |

Retained functional coverage can be verified without Office:

```sh
python3 scripts/verify_office_pdf_functional_coverage.py --verify-only
```

## Build and audit commands

Run each command from its owning workspace root with the default `target/`.
Cargo, GDB, Office and campaign audits are serial; independent reading may continue.
Do not set `CARGO_TARGET_DIR`. Use `cargo fmt --all`, not standalone rustfmt.

For iteration, choose tests for the changed subsystem. The user has waived
clippy for subsequent golden repairs: prioritize fixes, necessary functional
tests and affected configured audits. Inspect local `../fontations/` and
`../harfrust/` before dependency upgrades; upgrade when the changes help fidelity.
If a `cargo update` fails, stop and give the user its command and error; do not
retry or substitute path dependencies. A successful dependency update still needs
functional checks and an audit of the affected scope before acceptance.

The clippy commands below are retained as a reference for separately requested lint work:

| Workspace | Command |
| --- | --- |
| Implementation | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test suite | `cargo clippy --workspace --tests -- -D warnings` |
| emfsdk, when changed | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |

Table-driven layout case labels are not Cargo test names. Select a case through
`OOXMLSDK_LAYOUT_CASE` and run its actual test function; verify that tests ran:

```sh
env OOXMLSDK_LAYOUT_CASE=tdf170201 cargo test -p ooxmlsdk-layout-test --release \
  --test xlsx_layout xlsx_layout_matches_libreoffice_layout_coverage -- --exact
```

Private XLSX display tests use several named submodules: filter with
`xlsx::display::`, not `xlsx::display::tests`; zero selected tests are not verification.

From the suite root, build once before configured verification:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
./target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id '<id>' --task /tmp/case-task.json
./target/release/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-result.json --write-artifacts true
jq -e '.verdict == "PASS"' /tmp/case-result.json
```

Use `audit --selection full --configuration-ids FILE --jobs 4` for a batch;
looping `prepare-audit-one` repeatedly revalidates the plan. Native exit 0 alone
is not a PASS. To enforce the protected selection, omit `--full` from the wrapper
below and add `--require-all-pass`; omit `--previous-results` or supply results
with exactly that selected ID set, not the full campaign.
Scoped selections accept only PASS/FAIL rows with valid references. Keep affected
REFERENCE_FAIL rows separately and inherit their unresolved obligations in the
merged ledger; including them in a scoped selection causes validation to fail.

Full verification when warranted (fresh output directory each time). Include the
accepted PASS selection **and every pending PASS** in the protected baseline;
refresh this union immediately before auditing:

```sh
repair_archive=/tmp/ooxmlsdk-repair-resume-20260912
python3 - <<'PYTHON'
import json
from pathlib import Path
root = Path('/tmp/ooxmlsdk-repair-resume-20260912')
protected = {}
for name in ('checkpoint-batch-28-local/pass-baseline.jsonl',
             'ledger/pending-pass-obligations.jsonl'):
    for line in (root / name).read_text().splitlines():
        row = json.loads(line)
        protected[row['configuration_id']] = row
(root / 'ledger/current-pass-union.jsonl').write_text(
    ''.join(json.dumps(row) + '\n' for row in protected.values()))
PYTHON
python3 scripts/audit_office_pdf_pass_union.py --full --build \
  --baseline "$repair_archive/ledger/current-pass-union.jsonl" \
  --previous-results "$repair_archive/checkpoint-batch-28-local/cumulative-results.jsonl" \
  --output-root /tmp/ooxmlsdk-golden-audit-next --jobs 4
```

Omit `--build` only when the release binary matches current source. Full acceptance
requires exact identities, no infrastructure errors and every protected PASS;
existing FAIL/REFERENCE_FAIL remain reported. `summary.json` is authoritative;
`diagnostic-deltas.json` contains changes. Archive sources, binary/plan hashes,
results and the promoted PASS selection. Audits share native report paths: never
overlap them. Read selected summary fields rather than dumping the whole JSON:
embedded unresolved rows can make it several megabytes. Whitelist the small
summary fields to print; excluding guessed large keys can still dump
`unresolved_union` and its full diagnostic payload. Wrapper tests:
`python3 -m unittest discover -s scripts -p 'test_audit_office_pdf_pass_union.py'`.

## Source Map

Office configured output is the target. Start with the relevant local spec and
implementation, then use primary online sources or controlled Office observations
to resolve gaps. Read complete functions and their consumers. Use this index to
choose a route; do not scan every project for each bug.

| Need | Durable route |
| --- | --- |
| ECMA Word, DrawingML, charts, math, MCE, OPC | `references/references/Ecma Office Open XML Part *.md` |
| Office deviations/defaults/extensions | local `[MS-OI29500]`, `[MS-DOCX]`, `[MS-OE376]`, and `[MS-ODRAWXML]` Markdown |
| Microsoft Open XML/API guidance | `../open-xml-docs/`, Microsoft Learn |
| Office fixed-format export policy | `scripts/convert_office_corpus.ps1`, `scripts/probe_office_pdf_options.ps1`; Microsoft Learn [Word](https://learn.microsoft.com/en-us/office/vba/api/word.document.exportasfixedformat), [Excel](https://learn.microsoft.com/en-us/office/vba/api/excel.workbook.exportasfixedformat), [PowerPoint](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.exportasfixedformat), and [fixed-format extension](https://learn.microsoft.com/en-us/office/pdf/extendingofficepdfexport) documentation |
| Archived Microsoft/Windows examples | `../msdn-code-gallery-microsoft/`, `../msdn-code-gallery-community-{0-9-non-alphabetic,a-c,d-l,s-z}/`; verify each sample's API, provenance, and license |
| VML, GDI, and Win32 contracts | `../win32/`, especially `desktop-src/VML/` |
| Windows API declarations and interop | `../win32metadata/`, `../CsWin32/`, `../WindowsAppSDK/`; `../windows-api-function-cheatsheets/` for discovery, not normative behavior |
| Windows Forms GDI text measurement and padding | `../winforms/src/System.Windows.Forms/System/Windows/Forms/Rendering/TextExtensions.cs` and `../winforms/src/test/unit/System.Windows.Forms/System/Windows/Forms/TextRendererTests.cs` |
| Windows rendering, printing, and XPS | `../Windows-classic-samples/`, `../Win2D/`; `../wpf/src/Microsoft.DotNet.Wpf/src/ReachFramework/Serialization/XpsImageSerializationService.cs`, `../wpf/src/Microsoft.DotNet.Wpf/src/System.Printing/CPP/src/GDIExporter/gdibitmap.cpp`, `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/common/Gamma.{cpp,h}`, `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/sw/swlib/swglyphpainter.cpp`, and `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/resources/{BlurEffect,DropShadowEffect}.cpp` |
| Direct2D path coverage and device grids | `../win32/desktop-src/Direct2D/`, local `d2d1.h` in `../win32metadata/`; Microsoft [D3D11.3 functional specification](https://microsoft.github.io/DirectX-Specs/d3d/archive/D3D11_3_FunctionalSpec.htm), especially fixed-point rasterization and per-path resolve. The standard sample patterns do not by themselves establish which profile an Office render target uses. |
| Package, schema, validators, fixtures | `../Open-XML-SDK/` |
| WordprocessingML transforms and content controls | `../Open-Xml-PowerTools/` as supplemental implementation evidence |
| DOCX import and visible layout | `../core/sw/`, `../core/oox/`, matching `qa/` |
| XLSX import and print layout | `../core/sc/source/filter/oox/`, `../core/sc/source/ui/view/printfun.cxx`, `../core/sc/qa/` |
| PPTX import and fixed pages | `../core/oox/source/ppt/`, `../core/oox/source/drawingml/`, `../core/sd/qa/` |
| LibreOffice PDF export | `../core/vcl/source/pdf/`, `../core/filter/source/pdf/`, `../core/officecfg/registry/schema/org/openoffice/Office/Common.xcs`, `../core/vcl/qa/cppunit/pdfexport/` |
| PDF catalog, actions, metadata, conformance | Adobe PDF Reference/pdfmark; `../pdf-issues/` for errata with their approval status; `../pdf-writer/`, `../krilla/`, and object-level `../lopdf/` tests for implementation evidence |
| OpenType contract | current Microsoft OpenType pages; `../OpenType-Specification/` is a historical mirror |
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../harfrust/`, `../parley/` |
| PDF comparison and output mechanisms | test code, `../pdfium-render/`, `../krilla/`, `../typst/crates/typst-pdf/`, `../cairo/`, `../tiny-skia/` |
| Geometry and graphics types | `../kurbo/`, `../color/`, `../peniko/` |
| EMF/WMF/GDI/OLE/CFB | local Microsoft specs, `../emfsdk/`, `../olecfsdk/`, `../libgdiplus/`, `../reactos/`, `../wine/` |
| Locale, generated resources and font fallback | `../icu4x/`; local `[MS-DOC]` FFM and Win32 font association/linking documentation. Word policy and shared font fallback need separate evidence. |
| CJK/ruby counterexamples | `../clreq/`, `../klreq/`, `../simple-ruby/`, `../i18n-tests/`, W3C JLREQ |
| OfficeMath/UnicodeMath | ECMA-376, Microsoft deviations, `../UnicodeMathML/` |
| Independent spreadsheet behavior | `../poi/`, `../EPPlus/`, `../ClosedXML/` |

Local implementations supply algorithms and counterexamples, not proof of Office
policy. Check provenance/license; read complete relevant sections and consumers.
Reuse the existing Markdown specs instead of reconverting them.
The `references/references/` paths above are relative to the **test-suite root**.
Before using optional PDF inspection libraries, check the available tools;
`qpdf --qdf --object-streams=disable`, `pdftotext`, `pdftoppm` and `mutool` are
available in the current environment; Python `fitz` is not.
For initial content inspection, limit `pdftotext` with `-f N -l N` and inspect
formula-bearing cells with per-part output limits; a short first-page mismatch
can come from a very large workbook.
The normalized text-content diagnostic sorts characters; it is not literal PDF
text. Read the actual page text or glyph trace before interpreting error codes
or formula output. Truncate long result messages when inspecting many-page cases.
Literal page-text array differences alone do not establish a configured failure:
the content gate compares page character bags, followed by line topology,
geometry and font checks. Follow the actual failed layer before changing the
order of independent text blocks. Diagnostic `page_index` and `page-N-*.png`
artifact names are zero-based; index 15 means physical page 16. Old failure
images can survive a later PASS; render the final candidate PDF for fresh images.
Batch audit results with `artifacts=none` also leave earlier `candidate.pdf`
files untouched. Use `audit-one --write-artifacts true` or `render-one` on the
current release binary before taking screenshots; existing paths prove no freshness.
The visible-output gate masks text already checked by the text contract. A better
header/footer screenshot can therefore leave that gate's metrics unchanged.
Inspect non-text differences first, including the last painted row on each page:
repeated titles can shorten the first page if pagination reserves them twice.
For physical-point vector comparisons, omit `mutool draw -r` or normalize by
its scaled media box. Office can coalesce adjacent same-color fills; identify
actual cell boundaries before comparing junctions. Inspect every page when row
geometry differs. `render-one` rejects existing output files: use a fresh path
or archive the disposable output before rerendering.

`mutool draw -F trace` and `pdftotext -bbox` can omit text lying wholly outside
the physical page even when PDFium extracts it. Inspect the decoded stream with
`mutool show FILE.pdf pages/N/Contents` before concluding that such payload is
absent. DynamicArrayFixture's page 5 keeps its merged title at x = -596.76 with
a page clip; see `xlsx-dynamic-array-merged/` in the repair archive.

## Development and debugging

Per user, prioritize remaining golden PDF failures. During iteration, use the
individual configured gate and only necessary private tests. Accumulate repairs
before a scoped or Excel audit; do not repeat public or campaign checks per fix.
Subsequent `ooxmlsdk-layout-test` runs are skipped by default; collect public-test
regressions for separate batch cleanup. Clippy is also skipped per user.

Use focused tests and configured goldens for deterministic fixes. For complex
visual differences, inspect enlarged candidate/Office images and PDF metrics,
trace the final consumer with GDB, then vary one input in a controlled Office
experiment. Reuse existing evidence and counterexamples before adding probes.
A passing diagnostic is not a replacement for configured acceptance.
Interpret PDF paths with their transforms and active clips: retained geometry
outside a print body does not prove that the overflow was painted. Distinguish
COM font metadata and text bounds from the effective PDF font and glyph advances.
For apparent layer seams, inspect drawing order and RGB/soft-mask samples separately.
GDI+ primitive coverage and premultiply/unpremultiply rounding can change how an
image edge blends with the line beneath it; reuse native pixel controls before
changing layer order or authored colors.

Use the **release** campaign binary for configured checks, including individual
cases and scoped audits. Try the existing release debug symbols for GDB first;
build dev only if required locals are unavailable. Do not attribute dev timeouts to renderer regressions without a release check; the resolved example
is archived under `xlsx-grid-strokes/excel-05-dev/`.

### Visual and font measurements

- Inspect every glyph advance: rotated PDF text may alternate long/short steps,
  so the first step or whole-string average can conceal device rounding. XPS
  `Glyphs.Indices` provides a useful cross-check of Office's pre-PDF distances;
  PDF can round tracking separately. Keep the golden PDF gate authoritative.
  Native f32 angle projection can straddle an integer boundary differently from
  f64 trigonometry (`xlsx-rotated-font-advances/xps-validation.json`).
- Excel `Range.Text` can report hashes while the exported PDF fits the full value
  at a boundary width. Use PDF glyphs/geometry for fixed-output decisions; retain
  COM values and alignment as supporting observations. See `xlsx-boolean-width/`.
- Excel `Range.Width` is a screen observation, not the printed column width.
  Use colored column markers at multiple print scales. In `xlsx-print-columns/`,
  31.5pt screen width prints as 30.96pt; the wrong conversion admits an extra
  column. Include sub-character widths: the inverse-storage half-MDW correction
  truncates before restoring pixels, so rounding `width * MDW` alone is wrong.
  Invalid `#N/A` PrintArea names were independently ruled out by identical
  original, cleared-area and explicit-area Office replays.
- Excel COM `RowHeight`/`StandardHeight` describe screen metrics; verify printed
  row distances in the exported PDF. Automatic rows may ignore cached
  `defaultRowHeight` and `dyDescent`. Use exact VDMX/GDI measurements where
  available; custom rows and mixed cell fonts need separate controls. Verified
  examples: `xlsx-text-position/evidence.md` and `probe-validation.json` in the archive.
- Inspect every cell in a row before attributing its height to a cache. In
  `51626.xlsx`, wrapped G/J/Q headers determine the A/C-page row height; changing
  `ht` to 40 leaves the Office PDF unchanged. Post-export `RowHeight` can also
  return a device-rounded value differing from `Range.Height`. Compare both
  with printed stripe distances (`xlsx-merged-borders/wrap-validation.json`).
- Check rich-string `rPr/scheme` as well as cell font schemes: the cached `rFont`
  can differ from Office's effective theme face. Shared and inline runs follow
  the same rule. Separate missing-theme substitution from installed-face and
  explicit non-theme behavior (`xlsx-template-fonts/` controls).
- Inspect `col@style` alongside row, cell and Normal fonts before attributing
  an automatic-row difference. Columns outside the printed range can affect
  the default height; the COM Normal style alone does not identify its owner.
  Fully styled columns can replace Normal in the screen font box while Normal
  still defines its printer conversion. Only bottom-aligned fonts combine their
  separate ascent/descent bounds; top and center alignment use independent font
  heights. See the native EMF controls in `xlsx-64473-rows/evidence.md`.
- When a public helper leaves the document language unset, verify its actual
  PDF font before applying a configured CJK assertion. Explicit-font and theme-
  selected-font controls distinguish font resolution from the stored Latin name.
  `new_cond_format` and `59264` have different Calibri/SimSun column-page owners;
  changing an implicit metric must not also change authored column widths.
- Compare the same physical grid, crop, zoom, alpha and background. Verify units
  and transforms before blaming layout. Lossless/pre-JPEG output isolates painting.
  Inspect enlarged crops as well as whole pages: downscaling hid the reversed
  table bands after a filtered row in `FilterColumn.xlsx` (`xlsx-print-grid/`).
- Compare native fill boundaries as well as border rectangles. At intersections,
  Office may trim a horizontal rule against the vertical rule; its left endpoint
  alone does not identify the cell origin. For margin controls, fix
  `PageSetup.Zoom`: fit-to-page can change the scale when a margin
  changes (`xlsx-border-origins/`).
- Distinguish requested fonts from actual glyph coverage/fallback and PDF fonts;
  missing PDF text may mean rasterized content or hidden output. An absent Windows
  SystemLink entry does not prove that no other fallback mechanism applies.
- EMF records use bytes; WMF record sizes use WORDs. An embedded EMF record may
  cross WMFC chunks. Validate the complete alternative before playback and keep
  raster/native extraction consistent; do not replay fallback graphics twice.
- Inspect native candidate realization density, alpha and placement:
  `office_pdf_campaign render-native-one --task TASK --input DOCX --output-root NEW_DIR --dpi 600`.

### Office experiments from WSL

Use `scripts/probe_office_pdf_options.ps1` and
`scripts/prepare_office_pdf_options_probe_plan.ps1`; reuse the complete golden
`assignment.office` and receipt `office_options`. The worker
`scripts/invoke_office_pdf_worker.ps1` accepts exactly one schema-2 record per
plan. Optional Word companions are `-DiagnosticWordXps` and `-DiagnosticWordEmf`
on the probe. Adapter checks: `scripts/test_office_pdf_options_adapter.ps1`.

Before reproducing a recovered reference, resolve its `reference_environment_id`
to the saved `corpus_pdf_conv/reference-adapters/` bundle and use that bundle's
worker. The ordinary probe's open behavior differs from repair-mode loading;
matching export options alone does not reproduce the recovery import contract.
The original environment is recorded in `corpus_pdf_conv/environment.json`;
its three adapter hashes match the scripts under `scripts/`. It has no separate
hash-named recovery bundle.

Resolve paths with `wslpath -w PATH`, then pass literal Windows paths to
`pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass -File SCRIPT`.
Use the authorized `pwsh.exe` executable directly: command authorization matches
the executable prefix. Invoke it in a separate command beginning with `pwsh.exe`;
do not prepend Python preparation or another shell command in that same invocation.
Keep logs/results under `/tmp` and reuse approved commands.
Avoid fragile nested `-Command` quoting.
Let the tool capture `pwsh.exe` output directly; the Office worker already writes
per-case logs. In this environment, adding shell `>` redirection failed at WSL
startup while the identical direct invocation ran successfully. A startup failure
does not establish that the prepared Office experiment ran.

For native table controls, `ListObject.ShowTotals = true` plus `SaveCopyAs`
provides an Office-authored experimental workbook. Enabling totals can insert
worksheet rows and move later tables: read their resulting ranges instead of
keeping original cell addresses. Preserve the saved copy used for SDK comparison.

When constructing SpreadsheetML border probes, preserve schema child order:
`vertical` precedes `horizontal`. Reversing them caused Excel `Workbooks.Open`
to fail; correcting that order made the same probe load successfully.

Keep original inputs read-only, macros disabled and Office process ownership
explicit. Do not force-close user applications. Run one Office worker at a time;
verify actual options/environment and preference restoration on success, error
and timeout. Do not change the OS clock/time zone. Prepared scripts or requested
settings are not evidence until their observed receipts and output are inspected.
When editing XML probe attributes, match the full attribute boundary and parse
the result to assert the intended element/value: a bare `ht="..."` replacement
also matches the suffix of `defaultRowHeight="..."`.
Check formula argument grammar when designing error controls. `DCOUNT` requires
a reference expression for its database argument (MS-XLSX 2.2.2); use
`INDEX(A1:C7,NA(),0)` to produce a runtime error there. A literal `#N/A` instead
tests rejected syntax and can fail both workbook Open and `Range.Formula` assignment.
Windows `GetLocaleInfoEx` supplies useful primary locale data, but compare it with
observed Excel text and exported PDF: some month abbreviations still differ.
BCP47 number-format markers may be normalized to hexadecimal LCIDs on Open;
compare the resolved language and rendered text rather than literal marker spelling.
The plan's `document_language` checks the observed Office authoring language;
changing that field alone does not switch the installed preference.
Receipt build strings are application-specific: this Office installation reports
Word `16.0.20326` and Excel `20326`; do not require an identical string for both.
Optional `HPageBreaks.Item` diagnostics can raise `DISP_E_BADINDEX` after a
successful PDF export despite `Count`. Catch and record that enumeration error
separately, then inspect the PDF and receipt; it does not indicate an Open failure.
For Excel AutoFit probes, select the target worksheet alone with `Select(true)`
first: grouped-sheet selection caused a multi-selection error in the local API
experiment. Read `Range.Height` for precise geometry; `RowHeight` reported 74.7pt
where `Height` was 74.625pt. See `xlsx-rotated-row-height/office-api-validation.json`.
Excel [XlBordersIndex](https://learn.microsoft.com/en-us/office/vba/api/excel.xlbordersindex)
uses positive 7/8/9/10 for left/top/bottom/right and 11/12 for internal rules.
Cast mixed numeric arrays to `[double]` when assigning COM `Range.RowHeight`;
an integer entry such as 48 can fail after earlier fractional entries succeeded.
For manual-height matrices, retain `SaveCopyAs` output and check a reopened copy.
The API request, serialized `ht`, displayed `RowHeight` and screen `Range.Height`
can differ: a 36.79pt request serialized as 36.85pt and displayed as 36.9pt.
Use the exported PDF to verify the actual printer grid; see `xlsx-border-origins/`.
For table header/totals controls, mutate a valid copied workbook through
`ListObject.ShowHeaders`/`ShowTotals`; the XML-only totals shortcuts failed Open
in `xlsx-print-grid/`. Set manual row hiding explicitly with
`Range('F16').EntireRow.Hidden`, then verify the printed bands.

For font matrices, give independent missing-font cases distinct names: Excel can
coalesce same-name records despite differing `family`/`charset`. When replacing
a worksheet's cells, remove stale `calcChain` entries and their package edges in
the experimental copy. Do not infer another authoring locale from a requested
value: this adapter validates the host's actual document language and format
locale (currently zh-CN). Requesting en-US format locale alone fails environment
validation before export; it does not switch Excel's host format settings.
Evidence: `xlsx-freesans/validation.json`, `xlsx-zero-date-fit/validation.json`.

For XML probe mutations, preserve the source namespace declarations and prefixes.
ElementTree reserialization can rename namespace prefixes while leaving QName
values such as `mc:Ignorable="x14ac xr xr2 xr3"` unchanged; Excel then fails Open
before the intended experiment. Use bounded lexical attribute changes or a
serializer that preserves those bindings. The failed first height matrix and its
replacement are retained under `xlsx-public-legacy/`.
When changing row or border fragments, check preserved cell-to-row membership,
ascending cell-column order and border indices as well as XML syntax. Excel can
reject parseable A2/C2/B2/D2 order at Open (`xlsx-row-column-fill/probe-02/`).
A greedy expression can consume a
self-closing element and its following sibling while leaving parseable XML.
Preserve worksheet child order when inserting controls: `mergeCells` follows
`customSheetViews` and precedes `phoneticPr`; otherwise Excel can reject Open
before testing the intended merge behavior (`xlsx-boolean-width/`).

For experimental PivotTable field changes, a `saveData="0"` cache may need
`PivotCache.Refresh()` before setting field orientation. Keep that refresh out
of the original-source replay. Verify custom-style color controls on a populated
report too: Excel's solid custom-table DXF uses `bgColor`; a foreground-only
control resolves white. See `xlsx-empty-pivot-style/`.

Worker diagnostic sidecars may remain under `output/_worker-stage/case-000/`
with the original filename; the controller copies only the PDF/receipt to
`case-000.*`. Enumerate the sidecars before reading them. Read indexed COM
properties explicitly: PowerShell's bare `PageSetup.PrintQuality` is a property
wrapper, not a DPI observation. `ListObject.TableStyle` returns a COM object;
read its `.Name` and release the reference instead of casting it to a string.
For standalone chart diagnostics, enumerate `Workbook.Charts`: `Chart.Type`
reports the chart kind, so comparing it with the `xlChart` sheet-type constant
silently skips the observation. Check diagnostic row counts as well as export receipts.
`CELL("filename")` also exposes the worker's
random temporary directory, so even original-source replays can differ in text,
width and page count. Preserve this distinction from a basename-only SDK call.
For SDK worksheet mutations, resolve the worksheet through the workbook sheet's
name and relationship ID. `worksheet_parts()` enumerates relationships, which
need not follow the workbook's displayed sheet order. Inspect the complete generated
type before mutating a collection: `Fonts` and `CellFormats` use `xml_children`
choice lists, while some neighboring collections expose named vectors.
`FontsChoice::Font` contains a direct `Font`; `CellFormatsChoice::CellFormat`
contains `Box<CellFormat>`. Do not infer boxing from a neighboring choice list.
Field access may also use `Deref`: `CellValue` wraps `XstringType`, so its struct
literal differs from the fields visible through `cell_value.xml_content`.
Match UI, format and document languages separately in public layout probes;
format locale alone does not select a theme's Hans font. Compare logical
COM row distances before printer rounding, and printed PDF distances afterward.

Record DPI awareness for native font probes. An unaware PowerShell process can
report 96 DPI through `GetDpiForSystem` on this 192-DPI host. The font experiment
sets and restores its thread's awareness explicitly; see
`xlsx-61652-rows/measure-font-aware.ps1`. Use `mutool draw -F trace` to inspect
PDF font sizes and glyph origins when Python PDF libraries are unavailable.

### GDB

Try the existing **release** binary with debug symbols first: GeneralLedger
page/setup locals were readable without a dev build
(`xlsx-rotated-print-extent/page-gdb.log`). Build dev only when the needed locals
are optimized out, then
verify final behavior in release. Use a script, gate hot-loop breakpoints,
and inspect the inferior's normal exit: GDB exit 0 can
hide script errors. Debugger overhead is not renderer performance.
Invoke `gdb` directly in the tool command. In this environment, a Python
subprocess wrapper failed with ptrace EPERM before startup while the direct
invocation succeeded. Explicitly source Rust's pretty printers; without them,
collection locals may show only raw pointers.

```gdb
set pagination off
set breakpoint pending on
source <rustc-sysroot>/lib/rustlib/etc/gdb_load_rust_pretty_printers.py
set logging file /tmp/case-gdb.log
set logging overwrite on
set logging enabled on
break path/to/source.rs:LINE
run
```

```sh
gdb -q -x /tmp/case.gdb --args ./target/release/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-gdb-result.json --write-artifacts true
```

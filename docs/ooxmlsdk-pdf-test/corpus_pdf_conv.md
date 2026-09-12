# Configured Office Golden PDF Fidelity

Repair entry point: accepted progress, audit commands and source references.
Keep experiments under `/tmp`, not in this guide.

## Golden Status

Reference generation closed 2026-09-12 at the user's request: **5,239 valid
Office PDFs / 131 unresolved references / 0 pending**, retaining all **5,370
source assignments**. The initial completed campaign produced 5,145 PDFs,
221 failures and four timeouts. Authorized recovery of failed references added
**94 PDFs: 54 Word, 20 Excel, 20 PowerPoint**, including all four original
timeouts. All 94 pass `qpdf --check`, source/output/configuration checks and
observed environment/clock validation. All 225 retry workers restored Office
preferences. The remaining retry outcomes are **92 zero-page outputs, 32 errors
and seven timeouts**; their original failed records are retained. No further
Office retries are planned for this closure.

All 5,370 source SHA-256 values still match the original frozen selection.
The original 5,145 successful configurations, conversion records and PDF bytes
are unchanged. Recovery changes only failed-reference configurations and uses
byte-identical temporary input copies, optionally opened with Office repair.
Original files and manifests under `corpus/` are never rewritten. Exact adapter
scripts, policy and provenance are in
[`reference-adapters/`](../../corpus_pdf_conv/reference-adapters/README.md);
`reference-recovery-history/` retains old assignments and worker evidence.
`recovery-plan-overrides.jsonl` reproduces the updated plan. Its SHA-256 is
`2670c58cf0c6f9f86b9f65207d747b773c33424042ac145ae5aa719bc587cd80`.
Recovery evidence: `/tmp/office-reference-failure-review-20260912/`.
Initial completion evidence: `/tmp/office-golden-rebuild-20260911/full-readiness/`.

Zero-page exports do not automatically establish normal empty output. Confirmed
empty workbooks have separate functional observations in
`corpus_pdf_conv/empty-output-observations.json`. The SDK now keeps their layout
empty, including implicit header/footer-only sheets, and its PDF API reports an
empty-layout error instead of inventing a white page. These checks do not count
as valid PDF goldens or candidate PASS results. Four source layout checks and two
PDF checks pass. A related comment-printing control passes. The pre-existing
`singlecontrol.xlsx` test remains a separate pagination failure: its LibreOffice
assertion expects three pages, the current default SDK layout has eleven, and
the current configured Office PDF has twelve. Its assertion was not changed.

Recovery replaces five pilot reference-failure identities; the archived
311-identity checkpoint remains historical. Its 156 PASS and 140 FAIL identities
are unchanged; recovered identities have no candidate verdict yet. All eleven
campaign configuration/environment tests pass; the regenerated plan is byte
identical. Both workspaces are formatted and the debug campaign binary builds.
Rebuild the release campaign binary before the next configured candidate audit.
No Office processes remain. No clippy or full candidate audit was run.

Rebuild was authorized 2026-09-11. The old golden directory was cleared
(5,146 PDFs, including reference revisions). Old plan/receipt metadata is in
`/tmp/office-golden-rebuild-20260911/previous-metadata/`. The first replacement
selection is 300 cases (100 per Office family). The v2 plan contains 5,370
assignments. Production conversion uses one Office worker at a time; each worker
temporarily selects the requested Office UI language, verifies actual readback,
and restores the original Office preferences, including after timeout.
The current inventory supplies `de-DE`, `en-US`, `es-MX`, `fr-CA`, `ja-JP`,
`ko-KR`, `zh-CN`, and `zh-TW`; it does not substitute `es-ES`/`fr-FR` labels.
Format locale, preferred authoring language and time zone are captured from the
host and verified per export. Windows settings and clock are never changed.
Each successful schema-4 receipt records the actual rendering environment and
open/refresh/export UTC clock boundaries; the candidate replays the refresh
start in the recorded time zone. Time-sensitive SDK discrepancies remain FAIL;
the visual acceptance thresholds have not been relaxed.

The first 300-case pilot is complete: **285 valid Office PDFs**, 14 conversion
failures and one timeout. All 15 reference failures already failed or timed out
in the retired campaign. Candidate comparison: **149 PASS / 136 FAIL /
15 REFERENCE_FAIL / 0 infrastructure errors**. All 285 PDFs pass `qpdf --check`;
source/output hashes and requested versus observed environments match. All 315
attempts restored Office preferences; the final readback matches the initial
snapshot and no Office processes remain. All eight UI languages are represented
in successful receipts. Evidence and the pilot checkpoint are under
`/tmp/office-golden-rebuild-20260911/`; the live report is
`target/office-pdf-campaign/bootstrap-audit.jsonl`.

The 5,370-source selection already excludes manifest `invalid` and `open_only`
cases. Round-trip eligibility does not guarantee that Office accepts a file or
exports a nonempty PDF; retain those failures as reference failures rather than
silently shrinking the source set.

Verification: workspace formatting, release build, seven campaign unit tests,
the hidden-slide layout regression, Word adapter tests, and controlled success,
environment-mismatch and timeout probes pass. No clippy or full campaign audit
was run. The complete-receipt and registered-environment checks were subsequently
run after all 5,370 conversions acquired terminal records.

Rendering takes priority in the current plan: **5,212 / 5,370 assignments export
all pages (97.1%)** after failed-reference recovery. The initial pilot had 150
first-page selections; recovery overrides are recorded separately. Only nine
additional directed sources exercise middle pages, final pages and non-first
contiguous intervals (three per Office family). UI language, print/screen and
other applicable settings retain the seeded random assignment; small directed
exceptions are recorded in
`crates/ooxmlsdk-pdf-test/data/office-functional-coverage.json`.

Two content-sensitive hidden-slide references now export all pages:
`2411-Performance_Up.pptx` includes all 48 slides, and `60810.pptx` excludes its
three hidden slides to export 25. Complementary print/screen and include/exclude
controls, plus full-document counterparts for the nine range cases, are retained
under `corpus_pdf_conv/functional-controls/`. The repeatable verifier checks
actual page membership/content against full exports, not just PDF page counts:

```sh
python3 scripts/verify_office_pdf_functional_coverage.py --verify-only
```

Directed verification is complete: all 11 main references and 15 complementary
control PDFs were generated successfully; 13 content-sensitive checks pass,
including retained-evidence verification without Office. Candidate comparison
for these 11 cases is 7 PASS / 4 text-layer FAIL / 0 infrastructure errors, with
no page-count or page-dimension failures. Office preferences were restored and
no Office processes remain. Eight campaign unit tests and the release build pass.
The original 300-case pilot is unchanged; cumulative checked identities now total
311: **156 PASS / 140 FAIL / 15 REFERENCE_FAIL**. The combined checkpoint and
final plan coverage are under `/tmp/office-golden-rebuild-20260911/full-readiness/`.
Full generation of the remaining 5,059 sources started with user authorization
on 2026-09-11. It uses the frozen plan, one Office worker, a 120-second per-source
timeout and at most two attempts, preserving the 311 completed records.
Live conversion log:
`/tmp/office-golden-rebuild-20260911/full-readiness/full-convert.log`.
Generation was stopped at the user's request on 2026-09-12 (local time), after
the active Office worker completed and confirmed environment restoration.
The persisted manifest has **4,393 converted / 14 failed / 1 timeout**;
**962 assignments remain without records**. Resume with the same command,
frozen plan and output directory: existing records are skipped, with successful
PDFs checked by size and SHA-256. The final drained worker was not promoted by
the stopped scheduler and may run once again. Failed/timeout records require
separate retry handling; normal resume skips them too.
The user authorized resuming conversion and simple repairs on 2026-09-12.
Resumed conversion completed with the same frozen plan; its log is
`/tmp/office-golden-rebuild-20260911/full-readiness/full-convert-resume-20260912.log`.
Stop/resume details are in `full-readiness/full-run.json`; final receipts and
counts are in `generation-completion.json` and `generation-native-summary.json`.
Four sources that previously failed now have references. Three previously
converted sources failed this time and are listed separately in
`reference-status-deltas-final.json`: Open-XML-SDK `TestFiles/complex0.docx`
(Word export error), Apache-POI `absolute-anchor-over-empty-sheet.xlsx`
(Excel open error), and LibreOffice `testModernVBADelete.docm` (locked staging
directory during cleanup). Their source hashes are unchanged. The Excel source
was an old candidate PASS, but old/new configured identities differ; this is a
reference-generation issue to investigate, not a measured candidate regression.
Do not classify these three sources as damaged without further evidence.

Forty-five SDK repairs and one Traditional Chinese waterfall resource
correction are staged, with **no new configured PASS claimed**.
The repair inventory, existing-fixture regressions, evidence, known
limitations, scoped configuration selection and deferred verification commands
are under `/tmp/office-golden-rebuild-20260911/pending-repairs/`.
Evidence comes from current Office PDFs and the Source Map; no new Office
experiments or acceptance-gate changes were made. After stopping Office, both
workspaces were formatted; five focused private checks and the tdf161301
fixture check passed at that snapshot. Remaining earlier runtime checks and
configured candidate comparisons are pending. Repairs made or revised after
conversion resumed still await formatting, compilation and focused tests,
including the non-Japanese e/ee follow-up to the previously tested era path. The
311-identity accepted checkpoint above is unchanged; staged repairs add no
claimed PASS. In particular, the print-area repair still needs semantic-text
verification, and the conditional-format repair strengthens a functional check
for an existing PASS rather than claiming a newly passing identity.
Reference recovery is now closed as described above. The earlier 45 repairs
still need their remaining scoped runtime and configured-candidate checks;
compilation alone does not establish their fidelity. The user requested returning
to ordinary FAIL repairs after preparing this closure for their commit.

The directed manifest is test-suite configuration, not SDK special-casing.
Ordinary sources use full-page rendering outside the unchanged pilot. No old
pilot identities or receipts were replaced when changing the pending plan.

Generic SDK JPEG/DPI requests do not constitute effective JPEG/DPI coverage under
the Office raster policy, which follows Office quality selection. Likewise,
visual PASS does not establish complete PDF metadata, bookmark, tagging or
conformance correctness; those need their functional assertions. Formatting and
authoring defaults remain the observed host profile, with observed-clock replay.

Rebuild commands, from this workspace root after closing Office applications:

```sh
target/release/office_pdf_campaign capture-environment --pwsh pwsh.exe
target/release/office_pdf_campaign generate-plan
target/release/office_pdf_campaign convert --selection pilot-300 --jobs 1 --pwsh pwsh.exe
target/release/office_pdf_campaign audit --selection pilot-300 --jobs 4
```

Capture and generation precede the first conversion only. Resume conversion with
the existing plan/environment; do not replace their identities once receipts
exist. Conversion skips terminal receipts and verifies existing PDF hashes.
The pilot plus directed cases provide 156 PASS identities; a full replacement
baseline is not available yet. The counts and protection rules below describe
the retired campaign; they are historical provenance and
must not be applied across the new environment/configuration identities.

Historical results before the authorized replacement:

| Scope | PASS | FAIL | Reference fail | Infrastructure error |
| --- | ---: | ---: | ---: | ---: |
| Latest full configured audit | 2,774 | 2,370 | 226 | 0 |
| Cumulative, including targeted repairs | 2,783 | 2,361 | 226 | 0 |
| New frozen PASS baseline | 2,774 | 0 | 0 | 0 |

Frozen archive: `/tmp/ooxmlsdk-golden-baseline-20260911/accepted-audit/`.
Results SHA-256: `b4b54d5b6b78d7892c6c90a587a423ac07a720282071d1f1b988c6d4426b20a7`.
Promoted to 2,774 PASS identities at the user's request after the full audit.
The previous 2,694-case baseline remains unchanged under
`/tmp/ooxmlsdk-golden-baseline-20260910/` as provenance.
Non-golden check results/exceptions: `/tmp/phase-closure-20260910/verification.json`
and `/tmp/phase-closure-20260910/README.md`; non-golden tests are not all green.

Latest full audit: `/tmp/full-pdf-printer-defaults-20260911/`, 293 seconds.
All 2,770 previously accepted PASS identities remain PASS; four additional cases
pass beyond the targeted repair. Results SHA-256:
`b4b54d5b6b78d7892c6c90a587a423ac07a720282071d1f1b988c6d4426b20a7`.
Latest targeted repair: `/tmp/pdf-xlsx-dxf-font-20260911/`: sparse DXF font
effects now distinguish omitted, enabled and explicitly disabled values.
Custom table regions apply after the table/column format snapshots;
`totalsRowFunction.xlsx` now retains Office's bold teal total-row font, though
its localized font-family mismatch remains FAIL. Scoped audit: explicit-disable
and custom-table cases plus all 275 Excel PASS identities, 340 total,
275 PASS / 65 FAIL; no new/lost PASS or new page-count mismatches. Two public
regressions and 22 private style/table tests, formatting and release build pass;
no clippy or full audit. The preceding pattern repair retained Office's `lightUp`
bitmap brush; other pattern types/custom table patterns remain incomplete.
The last added PASS remains `PivotSubtotalsSource/output.xlsx`.
Use the latest repair's `protected-results.jsonl` as `--previous-results`; it
protects all 2,783 PASS identities. This is a mixed checkpoint; the 2,774-case
frozen baseline remains unchanged. Other style/geometry failures, including the
sponsor report's earlier 12/13-page mismatch, remain open.

### Required scope

The new required selection is
`/tmp/ooxmlsdk-golden-baseline-20260911/pass-baseline.jsonl`: **2,774 PASS identities**.
SHA-256: `a9322d04d10612bab4fe0efec4df2898e2e822f517add0a15b8fd7430f280e4b`.
The sibling `manifest.json` pins the full archive and selection; neither depends
on `target/`. The historical union below is fully recovered and contained in this
new baseline; it remains provenance, not the current 2,774-case selection.

| Historical checkpoint | Subject | PASS |
| --- | --- | ---: |
| `1415b2688fba8902e0993ba8d7ae97db42d1b961` | fix(pdf): raise configured DOCX golden fidelity to 2,567 PASS | 2,567 |
| `45189fab4397aa5dd41f92ce51b90f028ab28b4b` | refactor(pdf): migrate rendering to direct pdf-writer output | 2,591 |

Their union is **2,660 identities** (2,498 shared + 69 + 93).
Records: `/home/kazeno/test/ooxmlsdk-progress/ooxmlsdk-dual-baseline-results/`;
selection: `target-pass-union.jsonl`, SHA-256
`92de1314a71d728bc6405dfae880641becaa2b1fe1e264c13b2d6fa00e56186a`.

Frozen PASS identities define scope; **current configured golden gates** define
acceptance. Preserve the new baseline and every subsequently accepted PASS identity;
new passes cannot cancel regressions. Do not change thresholds or exclusions.
Only investigate reference refreshes on concrete old/current Office conflicts;
preserve source/options and archive provenance as described in
`corpus_pdf_conv/reference-revisions/README.md`.

## Audit commands

Run from the suite root, with a fresh archive directory. Prefer affected cases
and relevant PASS controls during repairs. After accumulating a substantial repair
batch, run a full audit and promote the verified PASS baseline. Full audits also
apply to broad changes or evidence of cross-case risk. Skip clippy per the current
user instruction.
Full closure when warranted (build → audit → identity/PASS checks → archive):

```sh
python3 scripts/audit_office_pdf_pass_union.py --full --build \
  --baseline /tmp/ooxmlsdk-golden-baseline-20260911/pass-baseline.jsonl \
  --previous-results /tmp/pdf-xlsx-dxf-font-20260911/protected-results.jsonl \
  --output-root /tmp/ooxmlsdk-golden-audit-next --jobs 4
```

Use the latest **accepted** full results or the documented cumulative protection
checkpoint as `--previous-results`.
Exit 0 requires complete matching identities, no infrastructure errors, a
passing baseline and no lost PASS. Existing out-of-baseline FAIL/REFERENCE_FAIL are
reported, not reclassified. `summary.json` is the verdict; `diagnostic-deltas.json`
holds details. Build/campaign logs are quiet files. Full rendering can take
minutes; wrapper-only work should be negligible. Never overlap audits: native
report paths are shared (wrapper runs lock automatically).

For baseline-only iteration, omit `--full`, compare previous **selected-set** results if
available, and add `--require-all-pass` to enforce closure. Omit `--build` only
when the release binary is already current. Wrapper tests:
`python3 -m unittest discover -s scripts -p 'test_audit_office_pdf_pass_union.py'`.

Single case after building the release campaign:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
./target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id '<id>' --task /tmp/case-task.json
./target/release/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-result.json --write-artifacts true
jq -e '.verdict == "PASS"' /tmp/case-result.json
```

Batch selection uses `audit --selection full --configuration-ids FILE --jobs 4`;
do not loop `prepare-audit-one` across a corpus (it revalidates the plan).
A native process exit alone does not establish a PASS.

## Source Map

Start with local documents and implementations, supplement with primary online
sources, and use controlled Office observations when behavior remains unclear.
Read the relevant complete section or function and its callers, not just a
matching line. The table is an index, not an exhaustive search boundary or a
requirement to scan every project for every bug.

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
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../parley/` |
| PDF comparison and output mechanisms | test code, `../pdfium-render/`, `../krilla/`, `../typst/crates/typst-pdf/`, `../cairo/`, `../tiny-skia/` |
| Geometry and graphics types | `../kurbo/`, `../color/`, `../peniko/` |
| EMF/WMF/GDI/OLE/CFB | local Microsoft specs, `../emfsdk/`, `../olecfsdk/`, `../libgdiplus/`, `../reactos/`, `../wine/` |
| Locale and fallback mechanisms | `../icu4x/` |
| CJK/ruby counterexamples | `../clreq/`, `../klreq/`, `../simple-ruby/`, `../i18n-tests/`, W3C JLREQ |
| OfficeMath/UnicodeMath | ECMA-376, Microsoft deviations, `../UnicodeMathML/` |
| Independent spreadsheet behavior | `../poi/`, `../EPPlus/`, `../ClosedXML/` |


Local implementations supply algorithms and counterexamples, not proof of Office
policy. Check provenance/license; read complete relevant sections and consumers.
Reuse the existing Markdown specs instead of reconverting them.


## Diagnostic commands and pitfalls

- Simple deterministic fixes: focused tests + configured goldens. Complex visual
  fixes: enlarged AB + metrics, GDB at the consumer, controlled Office cases.
  Reuse existing controls; record only conclusions, counterexamples and next steps
  per case. A factorial round around 1,100 cases is available when interactions
  require it, not a routine prerequisite.
- Compare identical physical grids, alpha/background, crops and zoom. Verify
  measurement coordinates before diagnosing geometry/color. Lossless/preJPEG
  comparisons isolate rendering; only configured PDF checks establish golden PASS.
- Office adapter: `scripts/probe_office_pdf_options.ps1`; plan helper:
  `scripts/prepare_office_pdf_options_probe_plan.ps1`. Reuse the complete golden
  `assignment.office` and `conversion.office_options`, including cross-XML controls.
  Companion exports: `-DiagnosticWordXps`, `-DiagnosticWordEmf`.
- Native candidate:
  `office_pdf_campaign render-native-one --task TASK --input DOCX --output-root NEW_DIR --dpi 600`.
  Inspect the realization manifest for actual density, alpha and placement.
- WSL: first resolve `wslpath -w PATH`, then use literal Windows paths:
  `pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass -File '<script>' ...`.
  Keep inputs read-only, macros disabled, and Office process ownership explicit.
  Adapter tests: `scripts/test_office_pdf_options_adapter.ps1`.
- Run Cargo, GDB, Office and audits serially, from the owning workspace with its
  default `target/`; no `CARGO_TARGET_DIR`. Use `cargo fmt --all`, not standalone
  rustfmt. See `AGENTS.md` for closure tests/clippy and generation commands.

GDB: build **dev** to retain Rust locals; verify final behavior in release.
Use literal `gdb` and a script, and check the inferior's normal exit (GDB exit 0
can hide script errors). Gate hot-loop breakpoints; their cost is not render time.

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
gdb -q -x /tmp/case.gdb --args ./target/debug/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-dev-result.json --write-artifacts true
```

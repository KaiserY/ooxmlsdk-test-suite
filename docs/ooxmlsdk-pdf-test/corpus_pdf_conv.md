# Configured Office Golden PDF Fidelity

Session entry point: accepted progress, next repairs, and verification/debug routes.
Keep per-attempt logs and experimental plans in the archive, not in this guide.

## Current progress

The full audit on **2026-09-12** covered all **5,370 configured identities**:

| Result | Count |
| --- | ---: |
| PASS | **3,099** |
| FAIL | 2,140 |
| REFERENCE_FAIL | 131 |
| Infrastructure errors | 0 |

All 3,099 accepted PASSes were retained: **zero regressions**. This is a complete
rerun, confirming **41 additional PASSes** since the first full audit of the
rebuilt references (3,058). A partial rendering improvement remains FAIL until
all configured gates pass.

Archive root: `/tmp/ooxmlsdk-repair-resume-20260912/`.
Paths in the following table are relative to that root.

| Purpose | Authoritative artifact |
| --- | --- |
| Required PASS selection | `checkpoint-batch-09-full/pass-baseline.jsonl` |
| Latest complete results | `checkpoint-batch-09-full/cumulative-results.jsonl` |
| Checkpoint identity/accounting | `checkpoint-batch-09-full/manifest.json` |
| Full audit and diagnostic changes | `batch-09-full-audit/summary.json`, `diagnostic-deltas.json` |
| Audited sources and release binary hash | `snapshot-batch-09/source-snapshot.json`, `build.json` |
| Subsequent lint cleanup and validation | `closure-lint-snapshot/`, `closure-validation.json` |
| Handoff and individual repair evidence | `STAGE-CLOSURE.md`, `CURRENT.md`, `ledger/` |
| Retired baseline/source mapping | `baseline/` |

Protect this PASS selection and every subsequently accepted PASS. New passes
cannot cancel regressions. The original 2,774 baseline and cumulative 2,783
source scopes remain required through `baseline/historical-2783-source-bridge.jsonl`;
**126 mapped sources remain FAIL** in the new configurations. Their environment,
configuration and reference hashes changed during replacement, so these are
source obligations, not measured regressions of the old runtime identities.
The retired baseline is preserved at
`/tmp/ooxmlsdk-golden-baseline-20260911/pass-baseline.jsonl`; do not feed its old
IDs to the v2 plan.

Implemented repair areas include Word localized fields and text spacing; Excel
number/time formats, RTL, print areas and conditional formats; PPTX table styles,
SmartArt, script scaling and hyperlink bullets; and PDF symbol/vector handling.
WMF now assembles complete embedded EMF chunks, validates either the strict or
native Windows checksum convention, and selects one representation across raster
and native extraction. Invalid optional EMF streams retain WMF fallback drawing.
`GeomLec1.pptx` now converts but still has a text-content FAIL.

Focused checks and the release build passed before the full audit. Closure clippy
passed for the implementation and suite workspaces, and for emfsdk with all targets
and features, using `-D warnings`. After equivalent lint cleanup, 22 focused tests
and 198 emfsdk tests passed, with one existing ignored test. The current source is
archived separately from the full-audit build; rebuild before the next configured
comparison. No full PDF audit was repeated for this cleanup.

An unrelated OLE adapter compile error was fixed by propagating image-relationship
lookup errors. Its scoped `--no-deps` clippy and affected integration tests pass.
The extra standalone OLE check also exposed **55 existing core-library lints**,
which remain outside this repair; see `closure-validation.json`. Legacy integration
test exceptions below likewise remain open.

## Next session

1. Read the current checkpoint and `STAGE-CLOSURE.md`; inspect worktree changes
   before editing. All subagents are stopped; use the main thread unless the user
   authorizes delegation again. The user creates commits.
2. Select a concrete FAIL from the latest full results. Prefer a simple, evidenced
   implementation gap; consult `ledger/` before repeating prior experiments.
   Compile/functional success alone is not a configured PASS.
3. For the prepared complex leads below, use the smallest discriminating control
   before choosing a policy. Prepared probes are not completed evidence.
4. Run focused checks and affected configured cases. Protect accepted PASSes before
   promoting a checkpoint. Accumulate repairs before another full audit unless a
   shared change or observed regression warrants it; documentation/lint-only edits
   do not require another full rendering audit.

| Lead | Established evidence and next discriminating step |
| --- | --- |
| Word Han fallback / ADDRESSBLOCK | First 11 controls use SimSun for generated and ordinary authored Han; explicit YaHei remains YaHei. Verify UI language, base-font family and compatibility mode 12/15 before choosing a Word-specific fallback policy. Do not change shared XLSX/PPTX fallback or let cached font metrics cross policy boundaries. Evidence: `word/address-block-resource/deep-font-fallback/round-1-conclusions.md`; further plans are prepared but unrun. |
| Excel icon size / automatic rows | Seven Office controls show padding scales with print zoom and short rows crop height without proportional width reduction. Custom-row geometry does not follow the naive 14.5/15-point model; the tall grid lacks PDF text objects. Read `xlsx-next-format/icon-stop/appearance/office-experiment-01/measured-results.json`; its `interpretation.md` predates the run. Next: the prepared `office-experiment-02/` object-model observations and GDI font metrics, both unrun. No global size multiplier or font-family constant is established. |
| Remaining conversion/content gaps | `GeomLec1.pptx`, `tdf164620.docx` and `tdf104492.docx` now convert but retain text-content failures. `SheetTab.xlsx` retains heading text differences; `stress004.docx` retains a 145/95-page mismatch. Use the actual configured report to choose the next defect, not the disappearance of an earlier error. |

Keep these boundaries when resuming:

- Office preserves Wingdings F0A8 private-use text in protected `tdf138899.docx`
  and another PDF/A control. A blanket Unicode remap caused a real regression and
  was withdrawn. Do not reintroduce it or add a profile-specific acceptance bypass.
- Serbian LCID month naming, empty query-table header recovery and manual chart
  legends need stronger Office controls. Their proposed policies are not accepted.
- Existing `singlecontrol.xlsx` pagination and LibreOffice coverage/legend test
  failures are documented in the ledgers. Establish Office behavior before changing
  a legacy assertion; do not weaken golden gates to accommodate it.
- Empty Office exports are not automatically valid empty goldens. Confirmed empty
  workbooks have observations in `corpus_pdf_conv/empty-output-observations.json`;
  the SDK preserves empty layout and the PDF API reports an empty-layout error.

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

For iteration, choose tests for the changed subsystem. Closure clippy commands:

| Workspace | Command |
| --- | --- |
| Implementation | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test suite | `cargo clippy --workspace --tests -- -D warnings` |
| emfsdk, when changed | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |

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

Full verification when warranted (fresh output directory each time):

```sh
repair_archive=/tmp/ooxmlsdk-repair-resume-20260912
python3 scripts/audit_office_pdf_pass_union.py --full --build \
  --baseline "$repair_archive/checkpoint-batch-09-full/pass-baseline.jsonl" \
  --previous-results "$repair_archive/checkpoint-batch-09-full/cumulative-results.jsonl" \
  --output-root /tmp/ooxmlsdk-golden-audit-next --jobs 4
```

Omit `--build` only when the release binary matches current source. Full acceptance
requires exact identities, no infrastructure errors and every protected PASS;
existing FAIL/REFERENCE_FAIL remain reported. `summary.json` is authoritative;
`diagnostic-deltas.json` contains changes. Archive sources, binary/plan hashes,
results and the promoted PASS selection. Audits share native report paths: never
overlap them. Wrapper tests:
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
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../parley/` |
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

## Development and debugging

Use focused tests and configured goldens for deterministic fixes. For complex
visual differences, inspect enlarged candidate/Office images and PDF metrics,
trace the final consumer with GDB, then vary one input in a controlled Office
experiment. Reuse existing evidence and counterexamples before adding probes.
A passing diagnostic is not a replacement for configured acceptance.

### Visual and font measurements

- Compare the same physical grid, crop, zoom, alpha and background. Verify units
  and transforms before blaming layout. Lossless/pre-JPEG output isolates painting.
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

Resolve paths with `wslpath -w PATH`, then pass literal Windows paths to
`pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass -File SCRIPT`.
When the approved entry is Windows PowerShell, a literal `-File` bootstrap can
invoke PowerShell 7 in STA mode. Keep logs/results under `/tmp` and reuse approved
commands. Avoid fragile nested `-Command` quoting.

Keep original inputs read-only, macros disabled and Office process ownership
explicit. Do not force-close user applications. Run one Office worker at a time;
verify actual options/environment and preference restoration on success, error
and timeout. Do not change the OS clock/time zone. Prepared scripts or requested
settings are not evidence until their observed receipts and output are inspected.

### GDB

Build **dev** for Rust locals; verify final behavior in release. Use a script,
gate hot-loop breakpoints, and inspect the inferior's normal exit: GDB exit 0 can
hide script errors. Debugger overhead is not renderer performance.

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

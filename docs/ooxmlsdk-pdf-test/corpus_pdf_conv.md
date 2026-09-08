# Configured Office Golden PDF Fidelity

Current progress and evidence entry points for matching configured Microsoft
Office output. Paths below are relative to the test-suite root unless noted.
This is a working reference, not a fixed debugging procedure. Earlier experiments
are evidence with a scope, not rules that prevent a better implementation.

## Golden Status

The `office-ooxml-pdf-options-v1` campaign has 5,370 assignments:
5,144 usable Office PDFs and 226 explicit `REFERENCE_FAIL` records.

### Current accepted result — 2026-09-08

| Scope | PASS | FAIL | Reference fail | Infrastructure error |
| --- | ---: | ---: | ---: | ---: |
| Full configured campaign | 2,606 | 2,538 | 226 | 0 |
| Required dual-baseline PASS union | 2,601 | 59 | 0 | 0 |
| Related 3D text cases, including `31166a26` and `5dff0c41` | 10 | 0 | 0 | 0 |

Latest full evidence:
`target/full-native-locked-canvas-20260908/{results.jsonl,campaign-summary.json,verification.json}`.
Exact identity comparison against `target/full-direct2d-coverage-20260908/`
retains all 2,605 accepted PASS identities, including five outside the required
union, and adds `b0b95eb1`. The verification file lists the 59 remaining union
failures. Results SHA-256:
`73e36e08d9f60b5ad29afa6f6a0e9e7bbbcf7b63ce26b2d775271e9dd8216ea3`.
Release binary SHA-256:
`999c94f0b2f59d6e36d2774bf2f909a5d4401d462a233e10e9e27ebe8934a900`.

`b0b95eb1` (lockedCanvas image/line) now PASS: per-path device coverage,
physical-EMU pen realization, native source transport, and one associated-alpha
resolve to the configured PDF grid. Default output matches the passing candidate
byte-for-byte. The final full audit also verifies arbitrary group-coordinate
units do not scale physical pen width (`dc11a2b1`) and oversized source ranges
cannot abort allocation (`c9827663`, unchanged pre-existing FAIL). The rejected
first audit is retained separately in
`target/full-native-locked-canvas-initial-20260908/`.
Temporary detail: `/tmp/locked-canvas-image-line-ledger.md` and
`/tmp/ooxmlsdk-phase-closure-20260908.md`.

`31166a26` now passes the unchanged configured PDF gate:
17 / 3,325 significant foreground blocks (0.511278%), localized MAE
0.85004668534, comparison offset [0, 0]. Its paired actual-preJPEG diagnostic has
0 / 3,378 significant blocks and MAE 0.58300653595; this is supporting evidence,
not a replacement for PDF acceptance. Dev and release produce identical PDF bytes.

The 3D closure evidence is in `target/full-profile-realization-20260908/`;
temporary detail is in `/tmp/31166-ledger.md` and
`/tmp/static3d-ten-profile-realization/`. Passing the campaign does not mean
every diagnostic pixel is identical or improves.

`feb5dadf` (grouped-shape text highlighting) now PASS: corrected Word highlight
colors, consistent Windows vertical metrics, and logical cell widths retained
through output-font realization and paint segmentation. Dev/release PDF bytes
match; GDB verifies final consumed widths. Retained Office controls cover 45
color/host combinations and 72 font/size/content combinations. Temporary details
and counterexamples are indexed in `/tmp/feb5-ledger.md`. `/tmp` is disposable,
not baseline storage.

Next priority after the phase commit: restore the remaining 59 required identities while retaining
current PASS gains. The ten-case 3D cluster is closed at the existing acceptance
standard; tolerance-level differences alone do not require more work.

### Phase handoff checks — 2026-09-08

`target/phase-closure-20260908/verification.json` indexes complete check logs and
failed assertion names. All three changed workspaces pass `cargo fmt --all -- --check`.
Implementation workspace strict clippy and the suite's PDF/layout test-package
strict clippy pass. Native-picture, device-stroke, allocation-guard, PDF resolve,
audit-wrapper, and mocked Office adapter argument/cleanup checks pass.

Non-golden checks are **not all green**: implementation tests have 1,982 PASS /
12 FAIL / 3 ignored; PDF/layout suite tests have 720 PASS / 137 FAIL / 34 ignored.
These include older rendering expectations and other subsystem gaps; do not
assume every failure is a LibreOffice mismatch without checking Office evidence.
No assertions were rewritten just to obtain a green handoff. emfsdk all-feature
tests have 193 PASS / 0 FAIL / 1 ignored; its six strict-clippy findings are in
unchanged functions. Whole-suite clippy is blocked by the unchanged
`../olecfsdk/crates/olecfsdk-ooxml/src/ppt.rs:1304` Option/Result mismatch.
These exceptions are recorded separately from the accepted golden audit above.

### Baselines and completion

| Historical checkpoint | PASS | FAIL | Reference fail |
| --- | ---: | ---: | ---: |
| `1415b2688fba8902e0993ba8d7ae97db42d1b961` | 2,567 | 2,577 | 226 |
| `45189fab4397aa5dd41f92ce51b90f028ab28b4b` | 2,591 | 2,553 | 226 |

Subjects:
`fix(pdf): raise configured DOCX golden fidelity to 2,567 PASS` and
`refactor(pdf): migrate rendering to direct pdf-writer output`.
The second is the completed refactor, not its `50a2e302` starting checkpoint.

Their PASS union contains **2,660 identities**:
2,498 shared, 69 calibration-only, 93 refactor-only.
Durable records are in
`/home/kazeno/test/ooxmlsdk-progress/ooxmlsdk-dual-baseline-results/`:
`README.md`, `1415b268-full-audit.jsonl`, `45189fab-full-audit.jsonl`,
and `target-pass-union.jsonl`.
Union SHA-256:
`92de1314a71d728bc6405dfae880641becaa2b1fe1e264c13b2d6fa00e56186a`.

These historical sets define scope, not today's comparison behavior. Completion
requires all 2,660 identities to PASS under current gates and no loss of accepted
current PASS identities. New passes do not cancel regressions elsewhere.
Reference PDFs and their complete configured options remain the target;
threshold changes, exclusions and relabeling failures are not repairs.

### Audit entry points

Run from the test-suite root. Build once after code changes; use release for
campaign timing and acceptance:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release

./target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id '<configuration-id>' --task /tmp/case-task.json
./target/release/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-result.json --write-artifacts true
jq -e '.verdict == "PASS"' /tmp/case-result.json

./target/release/office_pdf_campaign audit \
  --selection full --timeout-seconds 180
```

A successful process exit does not imply a PASS verdict. Compare reports by exact
configuration ID, retaining full-campaign and selected-set counts separately.

For the required union, use the batch wrapper and a fresh output directory:

```sh
python3 scripts/audit_office_pdf_pass_union.py \
  --baseline /home/kazeno/test/ooxmlsdk-progress/ooxmlsdk-dual-baseline-results/target-pass-union.jsonl \
  --output-root target/pass-union-audit-new --jobs 4
```

Add `--previous-results target/<previous-union-run>/results.jsonl` for exact
verdict transitions, and `--require-all-pass` for final closure. Previous results
must cover the same ID/file set. A union-only audit does not replace a full audit.
The wrapper archives identities, hashes and timing; audit once per batch rather
than looping `prepare-audit-one`, which revalidates the whole plan each time.
Run only one campaign audit at a time because its native report paths are shared.

## Source Map

Start with local documents and implementations, supplement with primary online
sources, and use controlled Office observations when behavior remains unclear.
Read the relevant complete section or function and its callers, not just a
matching line. The table is an index, not an exhaustive search boundary or a
requirement to scan every project for every bug.

| Need | Durable route |
| --- | --- |
| ECMA Word, DrawingML, charts, math, MCE, OPC | `references/references/Ecma Office Open XML Part *.md` |
| Office deviations/defaults/extensions | local `[MS-OI29500]`, `[MS-DOCX]`, and `[MS-OE376]` Markdown |
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


Portable implementations provide algorithms and counterexamples, not automatic
proof of Office policy. Distinguish normative specifications, documented Office
deviations, implementation observations and hypotheses. Check provenance and
license before adapting code. Existing Markdown under `references/references/`
usually makes document reconversion unnecessary.

### Office and image diagnostics

- Configured conversion: `scripts/probe_office_pdf_options.ps1`.
  Minimum-case plans: `scripts/prepare_office_pdf_options_probe_plan.ps1`.
  Reuse the complete accepted task options for both producers, including
  `assignment.office` and `conversion.office_options`; avoid implicit COM defaults.
- Companion exports: `-DiagnosticWordXps` and `-DiagnosticWordEmf` on the same
  adapter. These preserve the normal PDF export and record companion hashes.
  Inspect actual image payloads, alpha and physical placement when relating formats.
- Candidate lossless route:
  `office_pdf_campaign render-native-one --task TASK --input DOCX --output-root NEW_DIR --dpi 600`.
  It writes PNG assets and a realization manifest without PDF/JPEG encoding.
  Native and actual-preJPEG diagnostics can isolate shared rendering defects;
  their results remain distinct from configured PDF verdicts.
- Adapter checks: `scripts/test_office_pdf_options_adapter.ps1`.
  Batch wrapper checks:
  `python3 -m unittest discover -s scripts -p 'test_audit_office_pdf_pass_union.py'`.

For WSL Office commands, resolve paths with a separate `wslpath -w PATH` call,
then use literal Windows paths with this prefix:

```text
pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass -File '<script-path>' ...
```

Keep probes under `/tmp`, inputs read-only, macros disabled, and Office process
ownership/cleanup explicit. An interop or authorization failure is a failed
measurement, not evidence about rendering.

### Debugging reference

Use tools to resolve the actual uncertainty, not to satisfy a ritual. Inspect
missing XML attributes/children and their effective-state consumers as well as
rendering algorithms. A clear implementation bug need not wait for a large
experiment; a proposed visual rule needs enough independent evidence to generalize.
For straightforward, undisputed defects, prefer a focused regression test and
the affected configured golden checks. Reserve enlarged AB comparisons and larger
Office experiments for complex visual uncertainty, such as the closed `31166`
case; do not make every repair repeat that workflow. Acceptance gates stay unchanged.

Compare Office/current images on the same physical grid with identical alpha,
background, crop and zoom operations. Full pages reveal missing content; enlarged
letters/layers reveal boundaries. Pair visual inspection with quantitative checks.
Verify coordinates and the measurement pipeline before blaming geometry or color.
An AB should identify its producers and current artifacts.

Reuse previous controls. When Office experiments are needed, isolate variables,
including duplicated XML representations; vary content and numeric boundaries as
appropriate. Complex interactions can justify a factorial round of about 1,100
cases. Stop when the evidence supports an independent repair; experiments serve
PASS, not indefinite exploration. Record conclusions, counterexamples and the next
action in one ledger per case. Revisit conclusions when contrary evidence warrants
it, retaining the reason and any regression debt.

For Rust state inspection, dev avoids optimized-out locals; validate the actual
release result afterwards. Start with literal `gdb`, using a script under `/tmp`:

```gdb
set pagination off
set breakpoint pending on
source <rustc-sysroot>/lib/rustlib/etc/gdb_load_rust_pretty_printers.py
set logging file /tmp/case-gdb.log
set logging overwrite on
set logging enabled on
# set environment NAME VALUE
# break path/to/source.rs:LINE
run
```

```sh
gdb -q -x /tmp/case.gdb --args ./target/debug/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-dev-result.json --write-artifacts true
```

Confirm state at its consumer, especially after a debugger intervention.
Gate hot-loop breakpoints by phase/sample; debugger overhead is not renderer
performance. Keep Cargo, GDB and other heavy commands serial; independent
read-only investigation can continue while they run.

Focused tests, configured goldens, related cases and full audits provide
complementary evidence. Select checks proportionate to the changed owner; preserve
the accepted PASS set and do not confuse unverified assumptions with frozen facts.

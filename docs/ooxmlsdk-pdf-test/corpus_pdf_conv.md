# Configured Office Golden PDF Fidelity

Repair entry point: accepted progress, audit commands and source references.
Keep experiments under `/tmp`, not in this guide.

## Golden Status

Verified 2026-09-10; regression recovery is complete, full Office fidelity is not.

| Scope | PASS | FAIL | Reference fail | Infrastructure error |
| --- | ---: | ---: | ---: | ---: |
| Full configured campaign | 2,694 | 2,450 | 226 | 0 |
| Required dual-baseline PASS union | 2,660 | 0 | 0 | 0 |

Accepted checkpoint: `target/full-phase-closure-20260910/summary.json`.
Results SHA-256: `d38f5d161b54c9f10f6b0f84dfa018d710f20a99af2b72b7781e2cf7e59f264f`.
Closure retains all 2,694 PASS identities with zero diagnostic changes.
Latest repair gains: `3dtestdot` and `3dtestdash`. Remaining FAILs are outside
the required union. Cached 3D preview support does not imply native GLB regeneration.
Full audit: 285 seconds; wrapper validation/archive: 1.03 seconds (build separate).
Non-golden check results/exceptions: `target/phase-closure-20260910/verification.json`
and `/tmp/phase-closure-20260910/README.md`; non-golden tests are not all green.

### Required scope

| Historical checkpoint | Subject | PASS |
| --- | --- | ---: |
| `1415b2688fba8902e0993ba8d7ae97db42d1b961` | fix(pdf): raise configured DOCX golden fidelity to 2,567 PASS | 2,567 |
| `45189fab4397aa5dd41f92ce51b90f028ab28b4b` | refactor(pdf): migrate rendering to direct pdf-writer output | 2,591 |

Their union is **2,660 identities** (2,498 shared + 69 + 93).
Records: `/home/kazeno/test/ooxmlsdk-progress/ooxmlsdk-dual-baseline-results/`;
selection: `target-pass-union.jsonl`, SHA-256
`92de1314a71d728bc6405dfae880641becaa2b1fe1e264c13b2d6fa00e56186a`.

Historical PASS sets define scope; **current configured golden gates** define
acceptance. Preserve the union and every subsequently accepted PASS identity;
new passes cannot cancel regressions. Do not change thresholds or exclusions.
Only investigate reference refreshes on concrete old/current Office conflicts;
preserve source/options and archive provenance as described in
`corpus_pdf_conv/reference-revisions/README.md`.

## Audit commands

Run from the suite root, with a fresh archive directory. Full closure in one
command (build → audit → identity/PASS checks → hashes, timing and archive):

```sh
python3 scripts/audit_office_pdf_pass_union.py --full --build \
  --baseline /home/kazeno/test/ooxmlsdk-progress/ooxmlsdk-dual-baseline-results/target-pass-union.jsonl \
  --previous-results target/full-phase-closure-20260910/results.jsonl \
  --output-root target/full-audit-next --jobs 4
```

Use the latest **accepted** full results as `--previous-results`.
Exit 0 requires complete matching identities, no infrastructure errors, a
passing union and no lost PASS. Existing out-of-union FAIL/REFERENCE_FAIL are
reported, not reclassified. `summary.json` is the verdict; `diagnostic-deltas.json`
holds details. Build/campaign logs are quiet files. Full rendering can take
minutes; wrapper-only work should be negligible. Never overlap audits: native
report paths are shared (wrapper runs lock automatically).

For union-only iteration, omit `--full`, compare previous **union** results if
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

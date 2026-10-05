# Configured Office Golden PDF Fidelity

**`/tmp/` is the authoritative working/evidence root across all stages.** Search
from `/tmp/` for prior scripts, C# probes, GDB traces and images. A directory such
as `/tmp/ooxmlsdk-repair-resume-20260912/` is one stage, not the archive root.
This entry keeps current obligations, source routes and reusable commands.
Keep experiments and case-specific history under `/tmp/`; replace stale status.

## Current progress

Accepted **batch54 full**: **5,370 identities = 3,641 PASS / 1,598 FAIL /
131 REFERENCE_FAIL**. All 3,641 protected passes preserved; **14 new PASS** versus
batch53, zero regressions or infrastructure errors. Audit: 370.679 seconds.
Receipt: `/tmp/ooxmlsdk-repair-resume-20260912/checkpoint-batch-54-full-final/`
(`summary.json`, `promotion.json`, frozen source/binary hashes).

**Original Astra84 completed and accepted**: 78 indexed cases verified against
exact IDs, plus 6 previously resolved obligations preserved. Receipt:
`/tmp/ooxmlsdk-repair-resume-20260912/ledger/astra-batch-54.json`.
Pause at this milestone as requested. On resumption, select one of the remaining
1,598 configured failures, prioritize concrete missing features, and accumulate
repairs before another full audit. Keep the 131 reference failures separate.

Closure verification: all three release Clippy lanes below PASS; private layout
2,233 PASS / 3 recorded old skips, PDF 293 PASS, emfsdk 225 PASS / 1 existing
ignored and 2 derive PASS. Native controls, actual dev GDB and release PDF checks
are in `/tmp/ooxmlsdk-batch54-regressions/README-current.md`; regression repairs
preserve golden files and acceptance gates. Full validation receipt:
`/tmp/ooxmlsdk-repair-resume-20260912/ledger/validation-batch-54.json`.

**`/tmp/ab.png/json` is floating-table-caption physical page 1, final-v2-PASS**.
Latest regression comparisons: `/tmp/ooxmlsdk-batch54-regressions/after-final/`.
Case histories and prior entry text are archived under
`/tmp/ooxmlsdk-floating-table-caption/entry-{progress-before-batch54,reference-before-compaction}.md`.

## Repair workflow and obligations

1. Start with one configured FAIL and its source XML. Separate missing features,
   geometry, text shaping and PDF paint before changing code. Prioritize unused
   attributes/types and clear functional gaps; avoid fitting small differences.
2. Search `/tmp/` for the same subsystem before creating probes. Combine local
   sources/specs, primary web sources, actual dev GDB, images and Office output.
   A metric alone does not justify a fix or reverting one. Office configured
   output is the target; preserve golden acceptance gates.
3. Check the affected case and meaningful controls in release. Add every new
   PASS to the protected union immediately. Accumulate repairs before full audits;
   public layout/XLSX suites and Clippy are not routine iteration gates. Run Clippy
   when requested for closure. Never claim an unrun command as a failed case.
4. Keep Cargo, GDB, Office and campaigns serial. Use each owning workspace's
   default `target/`, never `CARGO_TARGET_DIR`; use workspace `cargo fmt --all`.
   Release for checks, dev for GDB. The user commits; preserve unrelated changes.
5. Prefer pure Rust and read-only upstream references (especially `../fontations/`).
   On a failed crate update, stop and give the user the exact error; no retries or
   path-dependency substitution. Pause for a major dependency issue. No subagents
   without authorization.

Current ledger stage: `/tmp/ooxmlsdk-repair-resume-20260912/`:
`ledger/current-pass-union.jsonl` protects accepted **and pending** PASSes;
`ledger/pending-pass-obligations.jsonl` contains pending promotions;
`ledger/pending-golden-repairs.json` points to the accepted checkpoint.
`astra-deferred-index.tsv` indexes the original difficult cases.
Retired IDs are not valid v2 campaign IDs: their continuing obligations are mapped
in `baseline/historical-2783-source-bridge.jsonl`.

## Build and verification

From the **test-suite root**, build once and reuse the binary:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id '<id>' --task /tmp/case-task.json
target/release/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/case-result.json --write-artifacts true
jq -e '.verdict == "PASS"' /tmp/case-result.json
target/release/office_pdf_campaign render-one \
  --task /tmp/case-task.json --input /absolute/source.docx --output /tmp/fresh-candidate.pdf
```

Exit0 alone is not PASS. Use a fresh result/output path; `render-one` does not
replace existing files. A PASS need not refresh the failure artifact at
`target/office-golden/<id>/candidate.pdf`; render explicitly before comparing.
Record input/PDF hashes and page number in `/tmp/ab.json` alongside `/tmp/ab.png`.

Select private tests for the changed subsystem. Zero selected tests is not
verification. Private XLSX filters use `xlsx::display::`, not only `::tests`;
chart filters should cover importer/model/render modules. Table-driven public
layout labels require `OOXMLSDK_LAYOUT_CASE` and the actual test function.
Recorded unrelated private layout skips are listed in the current case receipt.

Requested closure commands, run serially from their respective roots:

| Workspace | Command |
| --- | --- |
| Implementation | `cargo clippy --workspace --all-targets --release -- -D warnings` |
| Test suite | `cargo clippy --workspace --tests --release -- -D warnings` |
| emfsdk, when changed | `cargo clippy --workspace --all-targets --all-features --release -- -D warnings` |

If `clippy --fix` fails to bind its lock-management TCP listener in the sandbox,
ordinary Clippy still works. Use `--message-format=json` and review/apply its
suggestions directly; do not treat this as a Cargo or Office permission failure.

For cumulative full verification, refresh the protected union from the accepted
checkpoint and pending PASS rows **before** starting. Keep that baseline, source
and binary frozen throughout the run. Use a fresh output directory:

```sh
repair_stage=/tmp/ooxmlsdk-repair-resume-20260912
python3 scripts/audit_office_pdf_pass_union.py --full --build \
  --baseline "$repair_stage/ledger/current-pass-union.jsonl" \
  --previous-results "$repair_stage/checkpoint-batch-54-full-final/results.jsonl" \
  --output-root /tmp/ooxmlsdk-golden-audit-next --jobs 12
```

Omit `--build` only for a current binary. For a protected scoped audit, omit
`--full`, add `--require-all-pass`, and omit `--previous-results` unless it has
exactly the same selected IDs. Native `--configuration-ids` expects compact
JSONL identity objects, not bare hashes or multiline JSON. Scoped selections
exclude REFERENCE_FAIL; retain those obligations separately. Campaign subcommands
have no `--help`; inspect the CLI source or use this wrapper's help.
`summary.json` is authoritative: require exact identities, no infrastructure
errors, and all protected PASSes. Print selected summary fields; unresolved
payloads can be huge. Archive source/binary hashes and promote only after checks.
Audits share report paths and must not overlap.

## Office and GDB commands

First check the command against `~/.codex/rules/default.rules` and existing `/tmp/`
scripts; do not immediately diagnose an environment failure. Resolve Windows paths
with `wslpath -w`. Invoke authorized executables directly, without a shell/Python
prefix or output redirection. A working PowerShell form on this host is:

```sh
pwsh.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command '& "\\wsl.localhost\Ubuntu-26.04\tmp\case\probe.ps1"'
```

`default.rules` explicitly allows the standalone `pwsh.exe` and `gdb` prefixes.
Run each as its own tool command, with the owning `workdir`; do not prepend
`cd ... &&`, `timeout`, `env`, Python or another shell. Let the tool capture
PowerShell output; use GDB's internal logging below. Keep shell single quotes
around the PowerShell expression so `$variables` are not expanded by Bash.
These rules describe known working invocations, not a way to override an actual
sandbox denial. If the same command is genuinely denied, report the exact action
and error instead of trying increasingly different wrappers.

`-File` with a UNC path can fail before PowerShell starts with
`UtilBindVsockAnyPort`; the standalone `-Command` form above has worked. Reuse
`scripts/invoke_office_pdf_worker.ps1`: its plan is exactly one schema2 record
with `schema_version,file,options,environment`, where `options=assignment.office`.
Use the suite `corpus/` root and a corpus-prefixed relative filename. Create a
fresh OutputRoot. The environment's UI/format/document languages are distinct;
a requested locale does not switch the host's installed preferences.

Recovery references must use their saved worker under
`corpus_pdf_conv/reference-adapters/`, selected by `reference_environment_id`.
Match import/recovery behavior as well as export options and package content type
(e.g. stress014.docx is internally DOTX). Original adapters are in `scripts/`,
identified by `corpus_pdf_conv/environment.json`. Optional probe companions:
`-DiagnosticWordXps`, `-DiagnosticWordEmf`; sidecars can remain in
`output/_worker-stage/case-000/` with the original filename.

Office matrix probes up to1100controls are authorized. Export before intrusive
COM diagnostics: header access and Excel page-break queries can repaginate.
COM positions do not supersede the actual exported PDF/XPS. If golden disagrees,
re-export the unchanged source to exclude Office version drift. Keep input
read-only, macros disabled, owned process cleanup explicit; never close unrelated
Office processes. Font realization can be cached across documents, so isolate
font/charset variants in separate Word processes. Native RVA probes must pin the
actual binary hash; relocate stale addresses instead of assuming them valid.

Reusable C# native observers:
`/tmp/ooxmlsdk-stress019/native-space-v18/producer/CharacterWidthObserver.cs`,
`/tmp/ooxmlsdk-stress019/native-punct-v10/LineAdvanceObserver.cs` and `observe.ps1`;
earlier hint probes under
`/tmp/ooxmlsdk-repair-resume-20260912/docx-WordWithAttachments/astra-hint-readback/`.
Check HRESULT, retry/buffer capacity, observed counts and actual process exit.
An API request or prepared script is not an observed result.

For GDB, build the dev campaign serially, then invoke `gdb` directly:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign
gdb -q -batch -x /tmp/case.gdb --args target/debug/office_pdf_campaign
```

```gdb
set pagination off
set confirm off
set debuginfod enabled off
source <rustc-sysroot>/lib/rustlib/etc/gdb_load_rust_pretty_printers.py
set logging file /tmp/case-gdb.log
set logging overwrite on
set logging enabled on
break path/to/source.rs:LINE
set args render-one --task /tmp/case-task.json --input /absolute/source.docx --output /tmp/fresh-gdb.pdf
run
p $_exitcode
```

Resolve `<rustc-sysroot>` once with `rustc --print sysroot` and substitute its
literal path in the `.gdb` file. Keep `debuginfod` disabled for local builds;
do not download symbols merely because a breakpoint or expression failed.

Assert nonempty captures and inferior normal exit; GDB exit0 can hide script
errors. Break after local assignment, not on iterator-closure lines. Gate hot
breakpoints by source identity/text and emit only needed fields. Rust slices use
`data_ptr`/`length` in GDB Python. Section-local page indices reset; identify the
physical page via source owner or page collection. Debugger time is not renderer
performance. Confirm the final release gate and compare dev/release PDFs.

## Diagnostic guardrails and evidence routes

- Separate source layout, device-grid conversion and final PDF operators.
  Check real transforms/units, clipping and paint order before tuning positions.
  Identical geometry split into different paths can rasterize differently.
- Red text contours alone do not prove wrong outlines: compare actual glyphs,
  origins, font size, fallback, paint and semantic mapping separately. Structured
  extraction can merge overlaps or omit clipped text; inspect raw BT/Tj, recursive
  `mutool draw -F trace`, PDFium boxes and native XPS before declaring glyph loss.
  MuPDF can emit glyph names and truncate font names; resolve embedded font maps.
- Use `hb-shape FONT.ttf TEXT --no-glyph-names --output-format=json` to isolate
  shaping, and compare to Office. Check probe DPI awareness and actual font face.
  Terminal-line controls need a following visible paragraph or end coordinate;
  matching glyphs alone does not prove height/pagination.
- Preserve namespaces/QName bindings (`mc:Ignorable`), XML child/cell order and
  package relationships in probes. Parse and verify the exact mutation. Copy
  `ZipInfo` before `writestr`; reusing its mutated metadata can cause false CRC or
  overlapping-entry errors. Match source defaults; empty content can select a
  different formatter path from the behavior under test.
- For COM matrices, use separate numeric/string `Value2` setters and explicit
  `[double]`/`[string]` casts. Inspect indexed properties and exported output,
  release child objects, quit the owned application and verify its recorded PID.
- Optimize measured whole-document bottlenecks: count calls and establish the
  algorithm/source contract first. Conservative off-page effect bounds must
  include shadows/glows; overlapping profiler intervals are not additive.

Search `/tmp/` first. High-value starting points (paths are under that root):

| Topic | Evidence owner |
| --- | --- |
| Font outlines vs origins, raw visual diff | `ooxmlsdk-text-outline-diagnostic/README.md` |
| U2004/U2006 font dispatch and native width producer | `ooxmlsdk-stress019/space-width-sources-v18.md` |
| Drawing, glyph hinting, C# Office probes | `ooxmlsdk-repair-resume-20260912/docx-drawing/` and `docx-textbox-zorder/` in that stage |
| Table rules, print grids and text baselines | `ooxmlsdk-bib-horizontal-borders/`, `ooxmlsdk-bib-body-grid/`, `ooxmlsdk-bug65649/` |
| Floating table grid vs outside frame | `ooxmlsdk-floating-table-caption/README-current.md` |
| Unicode/PDF semantic text and EMF | `ooxmlsdk-stress010-pagination/README-current.md` |
| Footnotes, bidi and line ownership | `ooxmlsdk-stress008/README.md`, `ooxmlsdk-stress011-cover/README-current.md` |
| Office codec choice / pure Rust cost model | `ooxmlsdk-bug59058-pagination/image-codec/README.md` |
| Performance algorithms / conservative effect culling | `ooxmlsdk-audit-performance/README.md`, `visibility-sources.md` in that owner |

Full earlier business/debug notes, searchable without bloating this entry:
`/tmp/ooxmlsdk-floating-table-caption/entry-reference-before-compaction.md`.
Older case-specific relative paths in that archive refer to
`/tmp/ooxmlsdk-repair-resume-20260912/` unless explicitly rooted otherwise.
Functional-coverage receipt check: `python3 scripts/verify_office_pdf_functional_coverage.py --verify-only`.

## Source Map

Office configured output is the target. Start with the relevant local spec and
implementation, then use primary online sources or controlled Office observations
to resolve gaps. Read complete functions and their consumers. Use this index to
choose a route; do not scan every project for each bug.

| Need | Durable route |
| --- | --- |
| ECMA Word, DrawingML, charts, math, MCE, OPC | `references/references/Ecma Office Open XML Part *.md` |
| Office deviations/defaults/extensions | `references/[MS-OI29500]-260519.docx`, `[MS-DOCX]-251113.docx`, `[MS-OE376]-220816.docx`, `[MS-ODRAWXML]-260217.docx` (all in `references/`). Read `word/document.xml` from the ZIP; do not assume Markdown conversions exist. |
| Microsoft Open XML/API guidance | `../open-xml-docs/`, Microsoft Learn |
| Office fixed-format export policy | `scripts/convert_office_corpus.ps1`, `scripts/probe_office_pdf_options.ps1`; Microsoft Learn [Word](https://learn.microsoft.com/en-us/office/vba/api/word.document.exportasfixedformat), [Excel](https://learn.microsoft.com/en-us/office/vba/api/excel.workbook.exportasfixedformat), [PowerPoint](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.exportasfixedformat), and [fixed-format extension](https://learn.microsoft.com/en-us/office/pdf/extendingofficepdfexport) documentation |
| Archived Microsoft/Windows examples | `../msdn-code-gallery-microsoft/`, `../msdn-code-gallery-community-{0-9-non-alphabetic,a-c,d-l,s-z}/`; verify each sample's API, provenance, and license |
| VML, GDI, and Win32 contracts | `../win32/`, especially `desktop-src/VML/` |
| Windows API declarations and interop | `../win32metadata/`, `../CsWin32/`, `../WindowsAppSDK/`; `../windows-api-function-cheatsheets/` for discovery, not normative behavior |
| Windows Forms GDI text measurement and padding | `../winforms/src/System.Windows.Forms/System/Windows/Forms/Rendering/TextExtensions.cs` and `../winforms/src/test/unit/System.Windows.Forms/System/Windows/Forms/TextRendererTests.cs` |
| Geometry libraries | Existing `kurbo 0.13.1`, local `../kurbo/kurbo/src/`, owns curves/paths/strokes. [geo](https://docs.rs/geo/latest/geo/) is a candidate for polygon operations/indexed queries; [spade](https://docs.rs/spade/latest/spade/) for constrained triangulation. Neither is currently imported. Preserve Office fill rules, event/source-face ownership and material interpolation; ordinary buffering/retriangulation is not a direct replacement. Assessment: `/tmp/ooxmlsdk-audit-performance/geometry-libraries.md`. |
| Conservative visible-effect bounds | `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/uce/drawingcontext.cpp` (`GetClippedWorldSpaceBounds`); `../core/drawinglayer/source/primitive2d/{shadow,glow}primitive2d.cxx`; `../skia/src/core/SkImageFilter.cpp`; [Direct2D custom-effect bounds](https://learn.microsoft.com/en-us/windows/win32/direct2d/custom-effects). Reject only complete conservative effect support at its final coordinate owner; an off-page caster can still cast a visible shadow. |
| Geometry performance and active-edge algorithms | `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/geometry/scanner.{cpp,h}` (`CActiveList`, candidate heap); [CGAL straight-skeleton event propagation](https://doc.cgal.org/latest/Straight_skeleton_2/classCGAL_1_1Straight__skeleton__builder__2.html). Preserve event ties and winding predicates when adding indexes. Measure Office export separately from process startup/open and SDK rendering separately from comparison/diagnostic rerenders. |
| Windows rendering, printing, and XPS | `../Windows-classic-samples/`, `../Win2D/`; `../wpf/src/Microsoft.DotNet.Wpf/src/ReachFramework/Serialization/XpsImageSerializationService.cs`, `../wpf/src/Microsoft.DotNet.Wpf/src/System.Printing/CPP/src/GDIExporter/gdibitmap.cpp`, `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/common/Gamma.{cpp,h}`, `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/sw/swlib/swglyphpainter.cpp`, and `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/resources/{BlurEffect,DropShadowEffect}.cpp` |
| Direct2D path coverage and device grids | `../win32/desktop-src/Direct2D/`, local `d2d1.h` in `../win32metadata/`; Microsoft [D3D11.3 functional specification](https://microsoft.github.io/DirectX-Specs/d3d/archive/D3D11_3_FunctionalSpec.htm), especially fixed-point rasterization and per-path resolve. The standard sample patterns do not by themselves establish which profile an Office render target uses. |
| Package, schema, validators, fixtures | `../Open-XML-SDK/`; local ZIP implementation evidence in `../zip2/` |
| WordprocessingML transforms and content controls | `../Open-Xml-PowerTools/` as supplemental implementation evidence |
| DOCX import and visible layout | `../core/sw/`, `../core/oox/`, matching `qa/` |
| HTML altChunk parsing | `../html5ever/html5ever/src/{tokenizer,tree_builder}/` for HTML tokenization and tree construction; `../rust-cssparser/src/` for CSS syntax, stylesheet/declaration parsing and error recovery |
| CSS selector matching and computed styles | `../stylo/selectors/{parser,matching}.rs` (standalone selector engine); `../stylo/style/dom.rs` and `../stylo/style/properties/cascade.rs` (full Stylo engine adapters and cascade). Keep Word-specific import and pagination policy in our layout model; browser CSS behavior alone does not establish Office behavior. |
| XLSX import and print layout | `../core/sc/source/filter/oox/`, `../core/sc/source/ui/view/printfun.cxx`, `../core/sc/qa/` |
| PPTX import and fixed pages | `../core/oox/source/ppt/`, `../core/oox/source/drawingml/`, `../core/sd/qa/` |
| LibreOffice PDF export | `../core/vcl/source/pdf/`, `../core/filter/source/pdf/`, `../core/officecfg/registry/schema/org/openoffice/Office/Common.xcs`, `../core/vcl/qa/cppunit/pdfexport/` |
| DEFLATE algorithm versus Office codec selection | `../wine/libs/zlib/{deflate,trees}.c`; [stock zlib](https://github.com/madler/zlib), [zlib-rs algorithm differences](https://trifectatech.org/blog/zlib-rs-in-firefox/). `/tmp/ooxmlsdk-bug59058-pagination/image-codec/` has actual native trial-size probes. Compression format compatibility does not establish identical size or image-policy decisions. |
| PDF catalog, actions, metadata, conformance | Adobe PDF Reference/pdfmark; `../pdf-issues/` for errata with their approval status; `../pdf-writer/`, `../krilla/`, and object-level `../lopdf/` tests for implementation evidence |
| OpenType contract | current Microsoft OpenType pages; `../OpenType-Specification/` is a historical mirror |
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../harfrust/`, `../parley/`; `../cosmic-text/` for source-cluster/bidi/fallback implementation comparisons (not PDF extraction or Office layout policy) |
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

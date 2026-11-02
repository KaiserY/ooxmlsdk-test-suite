# Configured Office Golden PDF Fidelity

**`/tmp/` is the authoritative working/evidence root across all stages.** Search
there for prior scripts, C# probes, GDB traces and images. A stage directory such
as `/tmp/ooxmlsdk-repair-resume-20260912/` is one owner, not the archive root.
This entry owns current progress, next steps and reusable routes; case history
belongs under `/tmp/`.

## Current progress and next step

Accepted full **batch74/v1105: 5,370 = 4,150 PASS / 1,089 FAIL /
131 REFERENCE_FAIL**, zero protected regressions/infra.
Protected union **4,150**, pending **0**; 19 direct repairs plus
3 additional gains accepted since batch73.
Full 236.979s; serial retained/current ratios
0.632–1.237, no sustained material performance regression.
Full duration includes isolated slow-case scheduling; serial pairs measure
renderer performance. The 180s per-case deadline is unchanged.
Full/performance/Clippy: `/tmp/ooxmlsdk-batch74-audit-preparation-20261102/closure-current.json`.
Acceptance: `/tmp/ooxmlsdk-repair-resume-20260912/checkpoint-batch-74-full-v1105/acceptance.json`.

Latest: **`tdf61000.docx` original golden PASS**. Fixed tracked Latin shaping,
absolute pitch/device realization and dotted table grids. Reused complete,
explicitly qualified reconstructed template input from the same environment;
original source/golden/options/gates unchanged, no original Office re-export.
Final dev GDB: 47 observations; dev/release PDF bytes match; all 8 pages reviewed.
Source/native/private receipts: `/tmp/ooxmlsdk-tdf61000-20261102/verification-current.json`.
Reusable: `/tmp/ooxmlsdk-knowledge/word-tracked-latin-and-dotted-cell-grid.md`.

**Stage closed; next: `tdf66039.docx` (not started)**, then ordered DOCX failures.
Keep about 20 repairs between routine full/performance audits. `/tmp/ab.json` pins
the live image/runtime. Detailed history and qualified auxiliary residuals stay
under `/tmp/`, indexed by `/tmp/ooxmlsdk-knowledge/README.md` and the ledger.

Single-agent; Cargo/GDB/Office/campaign lanes serial. Implementation 0.15.0.
Clippy: implementation/suite clean at v1105; emfsdk last requested closure v831.
Font assessment: `/tmp/ooxmlsdk-font-dependencies-20261015/assessment.md`.
Original Astra84 is complete. Pending/scoped gains never replace an accepted full.

## Repair workflow

1. Inspect one configured failure and its XML. Separate missing content,
   geometry, shaping and PDF paint. Fix clear feature gaps before tuning details.
2. Search **from `/tmp/`** first. Combine the Source Map, primary web sources,
   dev GDB, images and Office observations. A metric alone does not establish
   correctness or justify reverting code. Trust saved golden by default;
   re-export the original only with specific evidence of export/version drift.
3. Office matrix probes up to1100controls are authorized. Pin task options,
   locales, native module hashes and actual observed output. An API request,
   prepared script or exit0 alone is not a verified observation.
4. Use release checks and meaningful affected private tests. Protect every new
   PASS immediately, then confirm with actual GDB and image review before closure.
   Extractable text can survive while glyph paint is clipped or culled; inspect
   visible ink and its clip/culling owners. Public layout/XLSX reruns are optional stage audits; run
   Clippy when requested for closure. Never relax gates or drop protected cases.
   Batch routine performance comparisons with about20 repairs; investigate
   earlier when actual timings indicate a material slowdown. Keep one owned
   comparison baseline when needed and remove obsolete compiled copies.
5. Keep Cargo, GDB, Office and campaigns serial. Run from the owning workspace
   with its default `target/`; never set `CARGO_TARGET_DIR`. Use workspace
   formatting. Parallelize independent reading, probe preparation and artifact
   work; combine checks by affected subsystem and reuse established evidence.
   Preserve unrelated edits; the user commits.
6. Prefer pure Rust; upstream references, especially `../fontations/`, are
   read-only. Stop on a failed crate update or major dependency issue. Assess
   `Arc`/locks against actual sharing; reuse local caches. Do not add `unsafe`
   without reporting a major performance need to the user first.

Knowledge index: `/tmp/ooxmlsdk-knowledge/README.md`. Record source/spec/web
provenance, counterexamples and probe receipts in each owner. Detailed commands
and uncommon traps: `/tmp/ooxmlsdk-knowledge/office-golden-command-details.md`.
Ownership examples: `/tmp/ooxmlsdk-batch59-closure-20261019/concurrency-review.md`.
Remove only positively identified obsolete compilation artifacts; retain scripts,
source snapshots, native symbols and diagnostic evidence. `/tmp/` aliases can
resolve to live workspace targets: check real paths before deleting.

Ledger stage: `/tmp/ooxmlsdk-repair-resume-20260912/ledger/`:
`current-pass-union.jsonl` protects accepted **and pending** PASSes;
`pending-pass-obligations.jsonl` and `pending-golden-repairs.json` track promotions
and the accepted checkpoint. The stage's `astra-deferred-index.tsv` and
`baseline/historical-2783-source-bridge.jsonl` retain historical obligations.

## Build and verification

From the **test-suite root**, build once and reuse the binary:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id '<id>' --task /tmp/case-task.json
target/release/office_pdf_campaign audit-one \
  --task /tmp/case-task.json --result /tmp/fresh-case-result.json --write-artifacts true
jq -e '.verdict == "PASS"' /tmp/fresh-case-result.json
target/release/office_pdf_campaign render-one \
  --task /tmp/case-task.json --input /absolute/source.docx --output /tmp/fresh-candidate.pdf
```

Use fresh paths. `render-one` does not overwrite; a PASS may leave an old failure
artifact in `target/office-golden/`. Campaign page indices are **zero-based**.
For render-only derivatives, retain the configured task and pass the changed
source with `--input`; rewriting task identities is unnecessary.
Keep `/tmp/ab.json` with source/PDF/binary hashes, page and renderer settings.
Reuse PDFium width1333/flags17, white-background raw x6 diff; inspect x1 too.
Red glyph contours alone do not prove wrong outlines or configured failure.
For exact byte comparisons, reuse the identical task/input path: the configured
campaign attachment contains that path and affects PDF identifiers.

Private libtest filters after `--` are ORed; assert a positive selected count.
Use `xlsx::display::` for private XLSX filters. Run closure checks serially:

| Workspace | Command |
| --- | --- |
| Implementation | `cargo fmt --all`; `cargo clippy --workspace --all-targets --release -- -D warnings` |
| Test suite | `cargo fmt --all`; `cargo clippy --workspace --tests --release -- -D warnings` |
| emfsdk, when changed | `cargo clippy --workspace --all-targets --all-features --release -- -D warnings` |

If `clippy --fix` cannot bind its lock TCP listener, ordinary Clippy still works:
use `--message-format=json` and apply reviewed suggestions. Fix warnings at their
source/cfg boundary; do not suppress them.

For a full audit, snapshot the complete protected union outside a **fresh,
nonexistent** output root. Freeze source/binary/baseline during the run:

```sh
repair_stage=/tmp/ooxmlsdk-repair-resume-20260912
cp "$repair_stage/ledger/current-pass-union.jsonl" /tmp/frozen-protected-union.jsonl
python3 scripts/audit_office_pdf_pass_union.py --full --build \
  --baseline /tmp/frozen-protected-union.jsonl \
  --previous-results "$repair_stage/checkpoint-batch-74-full-v1105/results.jsonl" \
  --output-root /tmp/fresh-golden-audit --jobs 12
```

Omit `--build` only for a current binary. Scoped audits omit `--full` and add
`--require-all-pass`; reuse the archived baseline, checking row count/hash.
If supplied, `--previous-results` must contain exactly the scoped identities.
Native `--configuration-ids` requires compact JSONL identity objects. Campaign
subcommands have no `--help`; inspect CLI source or the wrapper's help.
`summary.json` must show exact identities, no infrastructure errors and every
protected PASS. Compare timings in the same persistent-worker/cache mode; cold
`audit-one` is not comparable to campaign timing. Performance method:
`/tmp/ooxmlsdk-knowledge/performance-comparison.md`.

## Office and GDB commands

Check `~/.codex/rules/default.rules` and existing `/tmp/` scripts before blaming
the environment. Resolve Windows paths with `wslpath -w`. Run authorized tools
**standalone**, without `cd ... &&`, shell/Python prefixes or output redirection:

```sh
pwsh.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command '& "\\wsl.localhost\Ubuntu-26.04\tmp\case\probe.ps1"'
```

Keep Bash single quotes around the PowerShell expression; `$variables` belong
to PowerShell. The known working form is `-Command`; `-File` with UNC has failed
before startup. Capture output through the tool, logging inside GDB. A genuine
sandbox denial must be reported, not bypassed by wrappers.

Reuse `scripts/invoke_office_pdf_worker.ps1`; its schema2 plan contains
`schema_version,file,options,environment`, with `options=assignment.office`.
Use the suite corpus root and corpus-prefixed filename. Office OutputRoot must
already exist and be empty. Match all locale/time-zone fields and the recorded
recovery adapter. Export before intrusive COM/page-break diagnostics. Keep input
read-only, macros disabled, snapshot/restore application options and terminate
only the owned Office process. Actual PDF/XPS supersedes COM page coordinates.
For Word fields/TOCs, pin `UpdateFieldsAtPrint` and `UpdateLinksAtPrint`:
application defaults can preserve caches that the configured print route rebuilds.
Use explicit on/off controls and restore the observed original values.

Syntax-check PowerShell via `Parser.ParseFile` and C# observers via `Add-Type`
before a queue; verify observed counts and normal process exit. If Office never
started, preserve that attempt as NOT_RUN. Native RVA probes must match the
actual module hash. Detailed C#/COM observer routes are in the knowledge index.

From the test-suite root, build dev for GDB and invoke directly:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign
gdb -q -batch -x /tmp/case.gdb --args target/debug/office_pdf_campaign
```

If ptrace is denied, use the authorized `require_escalated` tool mode with the
`gdb` prefix. Do not retry through wrappers or count startup failure as a trace.

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

Resolve the literal sysroot with `rustc --print sysroot`; do not download symbols
for a local breakpoint failure. Assert nonempty owner captures, no script errors,
inferior normal exit and final dev/release PDF agreement. Break after assignments,
check resolved locations and committed caller values. Bound hot callbacks and
large-array reads; use actual Rust enum/slice/String layouts. Debugger elapsed
time is not renderer performance. Use separate GDB processes for interventions
and final observations: CLI `delete breakpoints` can retain Python internal
breakpoints. More examples remain in the detailed guide.

## Diagnostic essentials

- Pin each geometry fact to its actual PDF owner/hash: a current Office probe
  is distinct from the saved golden. Separate source layout, device grids and PDF paint. Inspect transforms,
  clipping, operation grouping and paint order before adjusting coordinates.
  Capture native rounding mode for device math.
- For glyph differences, compare actual font versions/glyph sets as well as
  family names. A stale local mirror can change GSUB with identical OS/2 metrics.
  Isolate font selection first; proven one-font syncs retain backups/receipts.
  Route: `/tmp/ooxmlsdk-knowledge/word-mixed-vertical-shape-columns.md`.
- PDF extraction can omit clipped text or merge overlaps. Check raw PDF
  operators, font maps, PDFium boxes and transformed MuPDF traces before
  declaring glyph loss. Trace `metatext@txt` can contain raw `<`/`>`; escape
  only that attribute before XML parsing. `hb-shape` isolates shaping.
- Diagnostic derivatives must preserve XML namespaces/QNames, mandatory child
  order and relationships. Verify exact labels, uniqueness and drawing counts;
  a successful parse/export does not prove the intended mutation. Replace
  existing wrap choices in place. Copy `ZipInfo` before rewriting ZIP entries.
- Pin configured defaults/options as well as content. Source attributes can be
  overridden by styles; a default trait getter can hide a lost property. Trace
  new font fields through every adapter to the realized face.
- Fix measured document bottlenecks using established algorithms. Effect bounds
  must include off-page shadows/glows; profiler intervals may overlap.

Search `/tmp/` excluding private OS directories:
`rg ... /tmp/ -g '!systemd-private*' -g '!snap-private-tmp' -g '!codex-daemon-*'`.
Functional coverage receipt check:
`python3 scripts/verify_office_pdf_functional_coverage.py --verify-only`.

## Source Map

Office configured output is the target. Start with the relevant local spec and
implementation, then use primary online sources or controlled Office observations
to resolve gaps. Read complete functions and their consumers. Use this index to
choose a route; do not scan every project for each bug.

| Need | Durable route |
| --- | --- |
| ECMA Word, DrawingML, charts, math, MCE, OPC | `references/references/Ecma Office Open XML Part *.md` |
| Office deviations/defaults/extensions | `references/[MS-OI29500]-260519.docx`, `[MS-DOCX]-251113.docx`, `[MS-OE376]-220816.docx`, `[MS-ODRAWXML]-260217.docx` (all in `references/`). Discover existing Markdown with `rg --files --no-ignore references/` first; otherwise read `word/document.xml` from the corresponding ZIP. |
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
| JPEG sampling and MCU edge padding | `../wine/libs/jpeg/{jcprepct,jcsample,jccoefct,jfdctint}.c`; distinguish input right-edge expansion from padding completed downsampled row groups. Native vectors: `/tmp/ooxmlsdk-stress002-xlsx-20261011/native-jpeg-padding-v258/`. |
| PDF catalog, actions, metadata, conformance | Adobe PDF Reference/pdfmark; `../pdf-issues/` for errata with their approval status; `../pdf-writer/`, `../krilla/`, and object-level `../lopdf/` tests for implementation evidence |
| OpenType contract | current Microsoft OpenType pages; `../OpenType-Specification/` is a historical mirror |
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../harfrust/`, `../parley/{parley_engine,parley,fontique}/`; `../cosmic-text/` for source-cluster/bidi/fallback implementation comparisons (not PDF extraction or Office layout policy) |
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
Discover ignored converted specs with `rg --files --no-ignore references/`;
use the actual full filename instead of guessing a shortened Part1 filename.
Before using optional PDF inspection libraries, check the available tools;
`qpdf --qdf --object-streams=disable`, `pdftotext`, `pdftoppm` and `mutool` are
available in the current environment; Python `fitz` is not.

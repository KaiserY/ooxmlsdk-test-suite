# Configured Office Golden PDF Fidelity

This is the operating guide for advancing `ooxmlsdk-layout` and
`ooxmlsdk-pdf` against configured Microsoft Office PDFs. Keep only the latest
golden status, durable evidence routes, and reusable development/debugging
practice here. Per-case conclusions belong in tests and code; chronological run
history belongs in Git or disposable `/tmp` audit artifacts.

## Golden Status

The `office-ooxml-pdf-options-v1` plan has 5,370 deterministic assignments:
5,144 usable Office PDFs and 226 explicit `REFERENCE_FAIL` records. Every
reference is tied to one source identity, one complete option object, the
Office environment, and input/output hashes.

| Checkpoint | PASS | FAIL | Reference fail | Infrastructure error |
| --- | ---: | ---: | ---: | ---: |
| calibration commit `1415b2688fba8902e0993ba8d7ae97db42d1b961` | 2,567 | 2,577 | 226 | 0 |
| latest completed worktree full audit | 2,563 | 2,581 | 226 | 0 |

The calibration commit subject is
`fix(pdf): raise configured DOCX golden fidelity to 2,567 PASS`. The latest
full audit has 78 identities that were PASS at calibration and are now FAIL.
Acceptance requires all of those regressions to return to PASS and total PASS
to be at least 2,567; unrelated new passes never offset a regression.

The retained summaries are
`/tmp/ooxmlsdk-baseline-1415-full-audit-summary.json` and
`/tmp/ooxmlsdk-full-audit-centered-gdi-20260823-summary.json`. Counts are valid only for a
completed release audit with zero infrastructure errors. Update this section
only after another completed full identity audit; do not append case history.

## Contract

```text
OOXML package
  -> import/effective model
  -> layout display list
  -> candidate PDF
  -> layered comparison with the configured Office PDF
```

- A reference PDF and its conversion record are immutable within the campaign.
  Never regenerate it from candidate output or silently change its options.
- One normalized source identity has exactly one assignment and one terminal
  conversion record. Unsupported Office options remain explicit.
- Office fixed output is the visible target. Specifications and production
  source establish semantics; Office probes settle bounded visible behavior.
- Fix the earliest incorrect owner: import, cascade, font/shaping, layout,
  display list, or PDF lowering.
- A PASS must satisfy the independent identity, page, text, font, line,
  geometry, image-placement, and raster checks in `office_golden.rs`.
  Never weaken thresholds, exclude a source, or relabel a failure.
- Cargo build, test, format, clippy, and GDB preparation run sequentially in
  the owning repository with its default `target/`. Use release artifacts for
  development, debugging, focused acceptance, and full audits.

## Evidence And Fix Workflow

Use evidence in this order:

1. local specifications and documents under `references/references/`;
2. local production projects and their nearby tests in the Source Map;
3. primary online specifications or official documentation;
4. an exact-config Office minimum-case matrix when sources do not settle the
   visible rule.

A code change for a regression requires all three runtime checks: GDB identifies
the first wrong state or branch, complete Office/candidate images identify the
visible boundary, and an exact-config Office comparison establishes the target.
Source and test evidence remains necessary whenever it exists.

Treat each missing function or state transition as an independent repair point.
Do not tune the golden fixture. Build a minimum DOCX/XLSX/PPTX that contains the
gap, copy the complete option object from its golden task, and vary one
independent variable at a time. For numeric boundaries, cover both sides and
interpolate between them. Include same-state positives and opposite-state
counterexamples. The rule is pinned only when the matrix, original golden, and
counterexamples all agree.

For each gap:

1. verify source bytes, package part, relationship, authored presence, and exact
   assignment;
2. inspect the full Office and candidate pages before crops;
3. search the Source Map for the semantic owner and nearby QA;
4. trace package -> resolved state -> layout -> display list -> PDF in GDB;
5. construct the smallest exact-config Office matrix that distinguishes the
   remaining hypotheses;
6. implement the complete state chain at its owner and add focused
   positive/negative tests;
7. run the minimum matrix, exact golden, coherent cluster, and a stopping
   counterexample;
8. run a full release identity audit after any broad or complex change.

When sources disagree with Office, state the narrow disagreement and use the
controlled Office matrix for visible behavior. When no source exists, Office
may pin the rule only after single-variable combinations and boundary values
cover the relevant state space. Keep moving: an unresolved source search is not
a reason to leave a known regression unfixed.

## Source Map

Search local checkouts and converted documents before browsing. The Markdown
files in `references/references/` are searchable copies; do not reconvert
their source documents unless a copy is demonstrably defective.

| Need | Durable route |
| --- | --- |
| ECMA Word, DrawingML, charts, math, MCE, OPC | `references/references/Ecma Office Open XML Part *.md` |
| Office deviations/defaults/extensions | local `[MS-OI29500]`, `[MS-DOCX]`, and `[MS-OE376]` Markdown |
| Microsoft Open XML/API guidance | `../open-xml-docs/`, Microsoft Learn |
| Office fixed-format export policy | `scripts/convert_office_corpus.ps1`, `scripts/probe_office_pdf_options.ps1`; Microsoft Learn [Word](https://learn.microsoft.com/en-us/office/vba/api/word.document.exportasfixedformat), [Excel](https://learn.microsoft.com/en-us/office/vba/api/excel.workbook.exportasfixedformat), [PowerPoint](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.exportasfixedformat), and [fixed-format extension](https://learn.microsoft.com/en-us/office/pdf/extendingofficepdfexport) documentation |
| Archived Microsoft Open XML examples | `../msdn-code-gallery-microsoft/` |
| VML, GDI, and Win32 contracts | `../win32/`, especially `desktop-src/VML/` |
| Windows Forms GDI text measurement and padding | `../winforms/src/System.Windows.Forms/System/Windows/Forms/Rendering/TextExtensions.cs` and `../winforms/src/test/unit/System.Windows.Forms/System/Windows/Forms/TextRendererTests.cs` |
| Windows rendering, printing, and XPS | `../Windows-classic-samples/`, `../Win2D/`; `../wpf/src/Microsoft.DotNet.Wpf/src/ReachFramework/Serialization/XpsImageSerializationService.cs`, `../wpf/src/Microsoft.DotNet.Wpf/src/System.Printing/CPP/src/GDIExporter/gdibitmap.cpp`, `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/common/Gamma.{cpp,h}`, `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/sw/swlib/swglyphpainter.cpp`, and `../wpf/src/Microsoft.DotNet.Wpf/src/WpfGfx/core/resources/{BlurEffect,DropShadowEffect}.cpp` |
| Package, schema, validators, fixtures | `../Open-XML-SDK/` |
| WordprocessingML transforms and content controls | `../Open-Xml-PowerTools/` as supplemental implementation evidence |
| DOCX import and visible layout | `../core/sw/`, `../core/oox/`, matching `qa/` |
| XLSX import and print layout | `../core/sc/source/filter/oox/`, `../core/sc/source/ui/view/printfun.cxx`, `../core/sc/qa/` |
| PPTX import and fixed pages | `../core/oox/source/ppt/`, `../core/oox/source/drawingml/`, `../core/sd/qa/` |
| LibreOffice PDF export | `../core/vcl/source/pdf/`, `../core/filter/source/pdf/`, `../core/officecfg/registry/schema/org/openoffice/Office/Common.xcs`, `../core/vcl/qa/cppunit/pdfexport/` |
| PDF catalog, actions, metadata, conformance | Adobe PDF Reference/pdfmark, `../krilla/`, object-level `lopdf` tests |
| OpenType contract | current Microsoft OpenType pages; `../OpenType-Specification/` is a historical mirror |
| Font parsing, shaping, bidi, breaking | `../fontations/`, `../parley/` |
| PDF comparison and output mechanisms | test code, `../pdfium-render/`, `../krilla/`, `../typst/crates/typst-pdf/`, `../cairo/`, `../tiny-skia/` |
| Geometry and graphics types | `../kurbo/`, `../color/`, `../peniko/` |
| EMF/WMF/GDI/OLE/CFB | local Microsoft specs, `../emfsdk/`, `../olecfsdk/`, `../libgdiplus/`, `../reactos/`, `../wine/` |
| Locale and fallback mechanisms | `../icu4x/` |
| CJK/ruby counterexamples | `../clreq/`, `../klreq/`, `../simple-ruby/`, `../i18n-tests/`, W3C JLREQ |
| OfficeMath/UnicodeMath | ECMA-376, Microsoft deviations, `../UnicodeMathML/` |
| Independent spreadsheet behavior | `../poi/`, `../EPPlus/`, `../ClosedXML/` |

For Windows Office boundaries, consult official Win32 APIs and samples, WPF,
and WinForms first. OpenType defines font data, not system or Office fallback
policy. LibreOffice, Wine, ReactOS, PDFium, Krilla, Cairo, and tiny-skia supply
portable algorithms and counterexamples; do not present them as Office policy
without an Office control. Verify origin and license before translating code.

## Release Golden Loop

Run from `../ooxmlsdk-test-suite`. Keep candidate fonts and PDFium binding
stable. Build once after implementation changes:

```sh
cargo build -p ooxmlsdk-pdf-test --bin office_pdf_campaign --release
```

Prepare one exact configured task, verify both embedded identities, then use
the same release binary for GDB and acceptance:

```sh
case_id='<configuration-id>'
task="/tmp/ooxmlsdk-$case_id-task.json"
result="/tmp/ooxmlsdk-$case_id-result.json"

./target/release/office_pdf_campaign prepare-audit-one \
  --configuration-id "$case_id" --task "$task"
jq -e --arg id "$case_id" \
  '.assignment.configuration_id == $id and
   .conversion.configuration_id == $id' "$task"

gdb --args ./target/release/office_pdf_campaign audit-one \
  --task "$task" --result "$result" --write-artifacts true

./target/release/office_pdf_campaign audit-one \
  --task "$task" --result "$result" --write-artifacts true
jq -e --arg id "$case_id" \
  '.configuration_id == $id and .verdict == "PASS"' "$result"
```

`audit-one` can exit successfully while recording `FAIL`; the final
`jq -e` is mandatory. Inspect the newly written PDF and every page image,
then audit the coherent feature cluster.

After a broad change, run the complete configured audit:

```sh
./target/release/office_pdf_campaign audit \
  --selection full --timeout-seconds 180
```

Compare verdicts by configuration ID with the calibration report. Record exact
PASS->FAIL and FAIL->PASS identities, not only counts. Acceptance requires zero
calibration PASS regressions, at least 2,567 PASS, 5,144 audited references, 226
`REFERENCE_FAIL`, and zero infrastructure errors.

## Exact Office Minimum Cases

Configured Office conversion has one owner:
`scripts/probe_office_pdf_options.ps1`. The accepted task stores complete,
equal `assignment.office` and `conversion.office_options` objects. That exact
object must drive both Office and candidate conversion. Never hand-write a
shorter `ExportAsFixedFormat` call or rely on COM defaults.

Generate every minimum-case plan with
`scripts/prepare_office_pdf_options_probe_plan.ps1`, passing the accepted
golden task JSON and the list of minimum inputs. The preparer copies the complete
options object without inference and records the golden configuration ID, task
hash, options, and input hashes. Pass its plan unchanged to the canonical
adapter. A missing or mismatched field is a hard error.

The mappings are fixed: Word print/screen -> 0/1, Excel quality -> 0/1, and
PowerPoint intent -> 2/1. Candidate fixed-output quality must come from the same
recorded `quality` field. Any adapter/default/order change requires adapter
tests and a configured identity replay.

PowerShell probes require PowerShell 7, STA, and the full prefix:

```text
pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass
```

Resolve every WSL path first, in a separate command:

```sh
wslpath -w /home/kazeno/git/ooxmlsdk-test-suite/scripts/probe_office_pdf_options.ps1
wslpath -w /tmp/<probe-root>
wslpath -w /tmp/<probe-root>/plan.jsonl
wslpath -w /tmp/<empty-output-root>
```

Copy those outputs literally into a new command whose first token is
`pwsh.exe`:

```sh
pwsh.exe -NoProfile -NonInteractive -STA -ExecutionPolicy Bypass \
  -File '<literal-probe-script-UNC-path>' \
  -CorpusRoot '<literal-probe-root-UNC-path>' \
  -PlanFile '<literal-plan-UNC-path>' \
  -OutputRoot '<literal-empty-output-UNC-path>'
```

Do not put `$(wslpath ...)`, backticks, variable assignments, a loop,
`bash -lc`, or another command in front of `pwsh.exe`. WSL interop opens an
AF_VSOCK socket before PowerShell starts, so a sandbox allow-rule miss appears
as `UtilBindVsockAnyPort: ... socket failed 1` (`EPERM`); it is not evidence
that PowerShell or Office COM is unavailable. A literal-prefix `-Command
'$PSVersionTable.PSVersion.ToString()'` success paired with a wrapped-command
failure pins the cause to authorization matching: rewrite the command instead
of retrying the wrapper. Only when the fully literal command itself fails
should the identical literal command be retried as a transient WSL interop
case. An unsigned-script error means interop worked and
`-ExecutionPolicy Bypass` was missing.

Keep plans, inputs, outputs, and staging under `/tmp`. The canonical adapter
requires an existing empty output directory, opens inputs read-only with macros
disabled, and records Office version/build, exact options, and hashes. A probe
with different options is only a lead and cannot justify a code change.

## Diagnostics, Images, And GDB

Inspect package state before effective state; keep absent, explicit false,
inherited, and defaulted values distinct. Diagnose the first failing layer:

| Diagnostic | First owner |
| --- | --- |
| identity/open/extraction | manifest, package, parser, feature gate |
| page count/geometry | sections, breaks, page layout, printable region |
| text content/order/style | import, cascade, visibility, fields, fallback |
| line content/bounds/baseline | shaping, metrics, wrapping, bidi, frame owner |
| font integrity | face, glyph, cluster, embedding, `ToUnicode` |
| graphics | host geometry, transform, clip, paint order, image/metafile |
| visible output | display lowering, PDF paint, raster/backend |

Inspect output in this order:

1. full-page count, content, clipping, paint order, and geometry;
2. normalized text/order, selected fonts, and line reconstruction;
3. bounds, baselines, ownership, transforms, and clips;
4. glyph/CID widths, embedding, `ToUnicode`, and `ActualText`;
5. images, masks, color space, interpolation, and composition;
6. a crop around the repaired feature and its pixel/alpha differences.

A comparator PASS is not visual proof. Verify artifact timestamps and inspect
all candidate/Office pages. Keep physical frame edge, print edge, line box,
baseline, display coordinate, PDF matrix, clip, natural height, flow advance,
and page-fit boundary as separate quantities.

For unexplained state or ownership, run GDB against the exact release task.
Confirm the configuration ID at the breakpoint. Trace one value through import,
resolution, layout/shaping, display list, and PDF lowering; record the authored
value, resolved value, selected branch, owner, final coordinate/resource, and
first divergence. Use conditional or temporary breakpoints because the pipeline
may render more than once.

For crashes capture all threads, full backtraces, arguments, locals, and
`$_siginfo`. For an apparent hang, interrupt more than once: changing stacks
mean slow progress; identical blocked stacks suggest non-progress. Debug timing
is not performance evidence.

## Regression Gate

For every repair:

1. run focused implementation tests;
2. run the exact configured golden and inspect its full pages;
3. run the minimum Office matrix, same-state cluster, opposite-state controls,
   and all known baseline PASS cases touched by the owner;
4. after a complex or wide change, run the full release audit immediately;
5. compare exact identities against calibration and continue until every
   PASS->FAIL regression is gone.

Retain audit summaries and evidence under `/tmp`; delete temporary scripts,
instrumentation, images, and build probes after the repair. Never report a
focused run as the full baseline, and never let a new PASS hide a regression.

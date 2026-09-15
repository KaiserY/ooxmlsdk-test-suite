# Office alignment of the three public XLSX failures

The three original top-level failures are resolved and golden-audited: **56 public
XLSX tests** and **1,628 private layout tests** pass. The configured golden audit
and accepted checkpoint are tracked in [the campaign guide](corpus_pdf_conv.md).
Batch 20 keeps all 565 previous Excel PASSes and adds five, with zero regressions. Corpus packages, reference
PDFs, configured options and acceptance thresholds remain unchanged.

## Decisions and retained coverage

| Failure or assertion group | Resolution |
| --- | --- |
| Automatic chart title / missing manual legend entries | **Implementation fix.** Excel's manual Cartesian legend ignores the omitted, zero and explicitly supplied body insets. Preserve both entries, their positions, and the five UI-language title assertions. Word, PowerPoint and radial legend paths retain their own behavior. |
| `singlecontrol.xlsx` | **Implementation fix and corrected expectation.** Excel keeps the preceding header/footer pages: 12 pages total, with the row-517 control on page 10. Resolve the automatic row grid from printer font metrics, including independent Windows ascent/descent rounding when VDMX is absent. Two marker controls retain 13.56pt row steps with cached defaults of 14.25 and 30pt; Arial yields 13.68pt through VDMX. Keep cached automatic per-row heights outside this new scope; they can contain other fonts' expansion. |
| `two-data-fields.xlsx` total | **Implementation fix and corrected caption.** Excel prints `Grand Total 3.651248215 7`. General display limits decimal precision even in wide columns; preserve integer and scientific boundaries and exact small decimals. Separate plain-cell and pivot controls, widths 12–40, and screen/print exports establish the behavior. |
| Pivot captions, filtered totals and preserved languages | Replace LibreOffice-generated captions with Office output, including `Row Labels`, `Grand Total`, `(blank)`, `(All)` and `(Multiple Items)`. Preserve authored German, Hungarian and Russian captions. Check filtered totals rather than old unfiltered totals: examples include 9 instead of 13, and 2 instead of 17. |
| Picture/text page membership | Check the actual Office pages, including blank pages retained before pictures, the second image-hyperlink page, and the four-page aggregation export. Text presence checks do not impose a single display-list reading order across independent columns and drawings. |
| Four empty worksheet/header sources | Original Excel exports contain zero pages. Keep zero-page layout assertions, then add a single A1 cell in separate controls to verify the visible sheet, header/footer text, four colors, and bold/italic styles. Empty reference failures are not converted into accepted PDF goldens. |
| ActiveX checkbox and option-button captions | Captions live inside EMF previews. Keep layout image/control assertions and verify caption text, page membership and occurrence counts in `metafile_pdf`. The checkbox's configured golden was already PASS; adding ordinary layout text would duplicate its caption. |
| `tdf144642` and `tdf145129` page counts | Default layout options retain the stored Latin Calibri font. Explicit-Calibri Office controls reproduce the public output exactly at the page-membership level: 2 and 3 pages. The configured zh-CN theme selects DengXian and prints 1 and 2 pages. Those separate configured row-conversion failures remain open; no universal row-height ratio is established. |
| `totalsRowFunction.xlsx` page count | The default helper retains stored Century Gothic, matching the two-page explicit-font Office control. The zh-CN application-font configuration prints one page, which the corresponding SDK options also reproduce. Preserve the £350 total and landscape page assertions. |
| `tdf155402.xlsx` | The helper supplies a basename to `CELL("filename")`, producing one page. Office includes its randomized worker directory and prints two. Upstream `testTdf155402` checks the `[tdf155402.xlsx]Sheet1` suffix, not a fixed page count. Keep that filename-format assertion; the configured path-dependent failure remains separate. |
| `tdf100709.xlsx` | Upstream `testTdf100709XLSX` specifically checks that B52 and A75 remain unformatted `218`. Restore those assertions instead of demanding a date from unrelated malformed style references. Excel still fails to open this original package, including with the saved recovery adapter; it remains REFERENCE_FAIL, not an empty-output observation. |
| Comment-only and overflow expectations | `tdf119190` prints `c?` without note captions. Date cells in `tdf165503` print hashes when too narrow. Preserve those observed print settings and overflow outcomes. |
| `tdf141495.xlsx` foreign `getEomonth` calls | **Subsequent golden repair.** The configured Office PDF retains literal A5 = `44227` and returns `#NAME?` for all 23 LibreOffice Analysis service-name formulas. Remove the layout-only rewrite to Excel `EOMONTH`; update this case's layout/PDF assertions while retaining two pages and the literal serial. The formula-import test still checks the serialized `44926` cache and remains valid. Evidence: archive `xlsx-foreign-eomonth/`; batch acceptance remains separate. |
| `tdf163554.xlsx` 3D reference and page numbering | **Subsequent golden repair.** Excel's sheet-range grammar quotes the complete span; this source quotes endpoints separately and Office prints five localized name errors. Preserve the original formula text instead of converting it to Calc syntax, and check the five errors in layout/PDF. Sheet 2 sets `useFirstPageNumber` without a value, so its footer restarts at 1. Keep both pages, sheet captions and adjacent values. Evidence: archive `xlsx-quoted-3d/`; batch acceptance remains separate. |

## Evidence

Archive root: `/tmp/ooxmlsdk-repair-resume-20260912/xlsx-public-legacy/`.
`assertion-decisions-01.json` through `-04.json` retain the old/new blocks for all
57 unique coverage cases. `cases.json` maps them to unchanged corpus sources and
configured identities. The controls use Excel build **20326**, en-US UI,
zh-CN format/authoring locale and Asia/Shanghai, except the two original
configured-language replays. Validation checks source/PDF hashes, options,
observed environment, restored language preferences and absence of leftover
Office processes.

- `office-en-us/validation.json`: 56 exports, including configured-language
  singlecontrol and manual-legend replays identical in bbox and 144-DPI pixels.
  Shared nested dategroup and tdf112106 originally selected only page 1;
  `row-diagnostics/validation.json` adds verified all-page controls.
- `legend-matrix/validation.json`: 12 controls distinguish insets, frame size,
  font size and overflow. The cache-only long-label mutation was ineffective
  because Excel refreshed the labels from worksheet cells; it is not evidence
  for long-label wrapping.
- `empty-sheet-probe/validation.json`: four zero-page originals and four
  one-page A1 controls. The separate tdf100709 Open failure is explicitly retained.
- `height-matrix-lexical/validation.json` and
  `grid-precision-probe/validation.json`: font-specific custom/default row
  behavior, automatic row markers, explicit-font controls and pivot column widths.
  `public-font-metrics.json` contains GDI measurements at 96, 192 and 600 DPI.
- `58325-regression/probe/validation.json`: five controls show that a column
  theme font raises the automatic grid from Normal's 14.52pt to 14.88pt,
  independent of the cached 15/30pt default. The original four-page replay
  matches golden bbox and pixels. Keep mixed column fonts outside the new
  Normal-only manual-grid path; the first audit's regression is rejected.
- `general-number-probe/validation.json`,
  `general-number-boundary/validation.json`,
  `general-number-small/validation.json` and
  `general-print-quality/validation.json`: plain-cell decimal/scientific
  controls; the three print/screen comparisons have identical bbox output.

The first height matrix failed Open after XML reserialization discarded prefixes
named by `mc:Ignorable`; it supplies no height evidence. The corrected lexical
mutations preserve namespace bindings. Worksheet mutations in public tests resolve
sheet names through workbook relationship IDs, not relationship enumeration order.

## Subsequent formula and fill assertions

After the dynamic-array and solid-fill repairs, three public XLSX assertions
needed their observations updated. The configured Office PDFs remain unchanged.

- `tdf162093.xlsx` stores dynamic arrays with XLDAPR metadata. Office's
  FORMULATEXT omits CSE braces; the public default-language assertion now expects
  `=myData[#Headers]` without braces. The configured German spelling and the
  default English spelling remain separate language checks.
- `CFNotError`/`CFIsError` and `CFTop`/`CFBottom` retain the same expected
  highlighted rows. Office positions fill rectangles separately from the logical
  text origin, so the tests sample the line interior. Top/bottom checks now also
  verify unhighlighted rows. Native paths confirm the original row selection;
  no golden geometry or pixel tolerance changed.

Evidence: archive `batch-26-formula-checks/conditional-fill-office-evidence.json`,
its original golden traces, `xlsx-structured-spill-values/` and
`xlsx-dynamic-array-pagination/`. The separate FODS FORMULA regression is an
implementation dialect-boundary fix; its assertions remain unchanged.

## Batch 27 formula, locale and column checks

Office controls separate assertion changes from implementation failures:

- `tdf164895.xlsx`: upstream Calc expects C8 = 30 and D8 = 5, but the
  recovered Office import removes the two names with unqualified references.
  Layout and PDF assert both `#NAME?` results, the adjacent literals and one page.
- `tdf73063.xlsx`: upstream checks the serialized `[$-1C1A]` marker, not an
  English date. Office prints `субота, 17. јуни 1972` even with an English UI.
  Layout and PDF retain the one-page check and the complete localized date.
- `new_cond_format_test.xlsx` and its export counterpart: the old Calibri
  continuation-heading expectation is valid. Office controls with both explicit
  and theme-selected Calibri keep `above average` only on page 2; the configured
  SimSun output keeps it on page 1. Correct the implicit Calibri column metric,
  retain the layout expectation, correct the mapped PDF expectation and forbid
  the duplicate heading on page 1. Configured acceptance still targets SimSun.
- `59264.xlsx`: the Letter-canvas test had a default Calibri layout but an
  assertion from the configured SimSun PDF. Test both authoring-font contexts:
  the continuation starts at K with SimSun and J with Calibri; only Calibri
  repeats the preceding `MEDIUM_DASHED` overflow string. Preserve the A4 output,
  95% paper scale and unscaled Letter pagination checks in both contexts.
- The mapped `tdf162093.xlsx` PDF check now mirrors the already-established
  dynamic FORMULATEXT result without CSE braces; its two-page and value checks remain.

Archive `batch-27-assertion-closure/` retains 15 validated Excel build 20326
exports, including three exact original replays in text boxes and 144-dpi pixels.
Source/PDF hashes, options, actual environments and restoration were checked.
Three top/bottom-percent matrices additionally establish 864 comparisons: truncate
percentage ranks, select at least one numeric value, and include ties at the cutoff.
This corrects the two conditional-format sources' numeric formatting independently
of their font/page distinction. No corpus, reference PDF or golden gate changed.

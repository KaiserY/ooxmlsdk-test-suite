# LibreOffice Formula Test Migration Index

This document is a migration index for LibreOffice Calc formula coverage.
Use it as the checklist for translating tests into `ooxmlsdk-formula` and this
test-suite.

## Migration Status

The LibreOffice `sc/qa/unit/data/functions/**/fods/*.fods` function corpus has
been copied into `fixtures/LibreOffice/sc/qa/unit/data/functions/` and is covered
by `crates/ooxmlsdk-formula-test/tests/fods_corpus.rs`.

Current baseline:

| Corpus | Fixtures | Cached formula cells | Current formula assertions | Mismatched | Unsupported |
| --- | ---: | ---: | ---: | ---: | ---: |
| LibreOffice functions FODS | 507 | 52,191 | 3,139 | 0 | 0 |

This baseline follows LibreOffice `FunctionsTest::load`: the runner
hard-recalculates each workbook, asserts the summary result shape, and keeps
row extraction as failure diagnostics. It intentionally does not compare every
cached formula cell as an independent assertion.

Additional non-FODS migration baseline:

| Group | Source | Migrated Rust tests | Verification | Notes |
| --- | --- | ---: | --- | --- |
| Synthetic address/evaluator/shared formula | `ucalc_formula.cxx`, `ucalc_formula2.cxx`, `ucalc_sharedformula.cxx`, `subsequent_export_test3.cxx` | 58 | `cargo test -p ooxmlsdk-formula-test` passes | Public evaluator/address/shared-formula APIs cover all currently expressible non-edit synthetic formula cases. |
| XLSX formula import metadata/cache | `subsequent_filters_test*.cxx`, `subsequent_export_test*.cxx` | 34 | `cargo test -p ooxmlsdk-formula-test --test xlsx_import` passes | Includes `functions-excel-2010.xlsx` row whitelist, named/table/shared/array/external formula metadata, and fixture-backed display/cache assertions. |

The FODS corpus runner should follow LibreOffice formula-test semantics, not raw
FODS import semantics. For `sc/qa/unit/data/functions/**/fods/*.fods`, LO loads
the file, calls `DoHardRecalc()`, and asserts `Sheet1.B3 == 1.0`. Only when that
summary cell fails does LO scan sheets with the `Expected` / `Correct` /
`FunctionString` header layout to report the first failing row. Rust may keep
per-row extraction for diagnostics, but the authoritative formula assertion is
the hard-recalculated summary result.

The current migration pass ran `cargo fmt --all`,
`cargo test -p ooxmlsdk-formula`, and
`cargo test -p ooxmlsdk-formula-test`. Both packages pass with active
assertions.

Current API-blocked non-FODS groups:

| Group | Blocker |
| --- | --- |
| Token compiler/stringify/equality | No public token compiler/stringifier API equivalent to LO `ScCompiler`/`ScTokenArray` yet. |
| Structural reference updates | No public model for insert/delete/move/undo/sheet-copy/name-update reference rewriting yet. |
| Formula listeners/dependency edit state | Current dependency graph is import/evaluation metadata, not LO listener lifecycle state. |
| Spill/dynamic-array edit behavior | Import metadata and the scalar `@` operator expectation are covered where expressible; blocker clearing/auto-resolve/edit-time matrix state is not modeled yet. |
| ODS/XLS/XLSB fixture-backed formula import/export | Test-suite currently has OOXML package API and FODS fixture reader only; no ODS/XLS/BIFF fixture reader. |
| Export XML preservation checks | Raw XML assertions are excluded from this formula migration unless the behavior is exposed through formula text/value/metadata. |
| Volatile/time-dependent calculation | No deterministic public test hook for volatile functions such as `NOW` yet. |
| Structured table/external reference evaluation | XLSX import metadata/cache is covered where exposed, but full LO-style table and external-workbook live evaluation is not modeled yet. |

Current FODS alignment notes:

| Area | LO behavior | Current Rust state | Evaluation |
| --- | --- | --- | --- |
| Function corpus assertion | `FunctionsTest::load` hard-recalculates, then asserts `Sheet1.B3 == 1.0`; row scan is diagnostic only. | `libreoffice_function_test_cases` models this shape and falls back to row cases when the summary fails. | Keep, but document/measure the summary-first result separately from broad cached-cell comparison. |
| Hard recalc | LO recalculates the loaded document before checking formula results. | `hard_recalc_book()` repeatedly evaluates formula targets and updates eager cell values. | Formula-relevant; keep aligning when evaluator behavior changes. |
| FODS formula grammar | LO reads OpenFormula FODS formulas and evaluates them in Calc. | Reader normalizes OpenFormula text, arrays, named ranges, query-empty cells, row hidden/filter state, pivot tables, and volatile `TODAY` serials into `FormulaEvaluationBook`. | Formula-relevant support code; keep only where it affects formula evaluation. |
| Stale copied FODS cached values | Copied fixtures can lag later LibreOffice fixes. `AMORDEGRC(...;2)` was marked `Err:502` in the old FODS file, but LO commit `75d5acfc7820` intentionally accepts basis 2 for ODFF/OpenFormula while noting Excel does not. | The copied `amordegrc.fods` cache is corrected for the OpenFormula corpus; direct evaluator assertions separately pin Excel grammar to an error. | When FODS data conflicts with current `../core` implementation/history, prefer current LO for OpenFormula and add a grammar-specific Excel assertion instead of blindly preserving stale caches. |
| Function workbook failure diagnostics | LO scans tabs from index 1, finds `Expected`, `Correct`, `FunctionString`, and reports first incorrect row. | Rust extracts all failing row cases for better aggregate diagnostics. | Acceptable diagnostic extension, but not a separate migration target. |
| Dubious FODS fixtures | LO keeps `functions/array/dubious/fods` as observed edge-case data. | Rust uses cached rows for dubious fixtures instead of hard recalculating. | Keep documented as a deliberate oracle choice. |
| Calculation-settings raw XML assertion | LO XML import sets regex/wildcard search options, which can affect formula criteria. | Raw XML-only search-settings tests were removed from formula migration coverage. | Reintroduce only through formula results such as `COUNTIF`/`SUMIF`/database criteria behavior. |
| Generic FODS table/cell parser tests | Not part of formula assertions in `FunctionsTest`. | Raw table/cell/repeat/text parser shape tests were removed from formula migration coverage. | Treat parser details as harness support, not LibreOffice formula migration coverage. Avoid adding new import-XML-only assertions here. |

## 2026-06-18 Gap Review

Current `crates/ooxmlsdk-formula-test/tests/` inventory:

| Area | Rust tests | Coverage state |
| --- | ---: | --- |
| FODS function corpus | 1 formula corpus test | Formula coverage is the 507-fixture `FunctionsTest::load`-style corpus test; XML-support assertions are not counted as migration coverage. |
| Synthetic address parsing | 4 | Covers selected LO A1/R1C1 address cases; token compiler/stringifier cases are still blocked. |
| Synthetic evaluator assertions | 53 | Covers LO/POI scalar, lookup, aggregate, hidden-row, sheet, matrix, statistical, text, error, reference-grammar, query-empty, XLOOKUP regex, range/intersection, dynamic-array scalar, and regression formulas that current public APIs can express. |
| Shared formula translation | 1 | Covers direct shared-formula text translation only; structural edit/update behavior is still blocked. |
| XLSX import metadata/cache | 34 | Covers selected LO/POI `.xlsx` fixtures for defined names, data tables, shared formulas, spill metadata, structured references, external references, cached values, formula text, and display text. |

The highest-value missing tests fall into these buckets:

| Priority | Missing coverage | Source evidence | Current path |
| --- | --- | --- | --- |
| 1 | Remaining evaluator-only synthetic functions that are not yet expressible through public APIs: token compiler/stringifier cases, structural reference updates, dependency listener lifecycle, iteration state, live external-workbook evaluation, and edit-time dynamic-array lifecycle. | `ucalc_formula.cxx`, `ucalc_formula2.cxx`, `ucalc_sharedformula.cxx` | Blocked until the formula crate exposes equivalent public models; do not simulate LO edit internals with unrelated assertions. |
| 2 | FODS harness formula alignment. | `functions_test.cxx`, `functions_*.cxx` | Done for formula coverage: summary-first `FunctionsTest::load` behavior is modeled; row extraction remains diagnostic only. Do not add raw FODS import/XML shape checks as migration work. |
| 3 | Remaining XLSX fixture-backed cases whose assertions are pure export XML, drawing macro XML, conditional-format listener lifecycle, or data-validation/external formula XML. | `subsequent_filters_test*.cxx`, `subsequent_export_test*.cxx` | Excluded from this formula pass unless exposed through `WorkbookValueModel` as formula text/value/cache/metadata. |
| 4 | Shared formula XLS/XLSB/ODS and most `ucalc_sharedformula.cxx` edit cases. | `ucalc_sharedformula.cxx`, `subsequent_filters_test2.cxx`, `subsequent_filters_test4.cxx`, `subsequent_export_test3.cxx` | Keep blocked until structural edit/reference-update and non-XLSX fixture readers exist; keep lightweight XLSX formula-state tests where possible. |
| 5 | Token compiler/stringifier/equality and low-level reference token data. | `ucalc_formula.cxx` | Blocked on public token compiler/stringifier APIs. |
| 6 | Dynamic-array/spill edit lifecycle: blocker clearing, matrix master resize/growth, copy/undo/redo, and single-value operator edge cases. | `ucalc_formula2.cxx`, `subsequent_filters_test.cxx`, `subsequent_export_test2.cxx` | Import metadata is partly covered; edit-time behavior is blocked until the formula model exposes dynamic-array recalculation/edit state. |
| 7 | ODS/XLS/XLSB fixture-backed formula import/export. | `subsequent_filters_test*.cxx`, `subsequent_export_test*.cxx` | Blocked on ODS/XLS/XLSB readers in this test-suite; keep FODS under the corpus runner. |

## 2026-06-18 Dispatch Coverage Audit

This audit compares the current `ooxmlsdk-formula` function dispatch surface
against formula semantics exercised by `crates/ooxmlsdk-formula-test`. It is
scoped to formula evaluation tests only. Import-only XML assertions and raw
package checks are not counted as formula coverage.

Coverage counting uses `FormulaFunctionId`, not raw function-name strings:
LibreOffice FODS formulas often use prefixes such as `ORG.LIBREOFFICE.*`,
while the evaluator resolves them to the same function id as the Excel-facing
name. A function id is counted as covered when at least one registered alias is
used by either a direct evaluator assertion or the LibreOffice FODS function
corpus.

| Metric | Count | Notes |
| --- | ---: | --- |
| Registered formula function ids | 478 | Extracted from `FormulaFunctionId` and the public alias table in `crates/ooxmlsdk-formula/src/function.rs`. |
| Registered formula aliases | 531 | Public names after alias registration, before namespace-prefix normalization. |
| Function ids with ordinary dispatch entry | 437 | Unique `FormulaFunctionId::...` references in `crates/ooxmlsdk-formula/src/function/dispatch.rs`. |
| Function ids with any evaluator implementation reference | 444 | Unique `FormulaFunctionId::...` references across `crates/ooxmlsdk-formula/src/`, excluding the registry table itself. This includes ordinary dispatch and special evaluator paths. |
| Function ids hit by direct evaluator assertions | 213 | Extracted from formula strings in `tests/evaluation.rs`. |
| Function ids hit by all non-FODS formula-test assertions | 220 | Extracted from formula strings in `tests/evaluation.rs`, `tests/xlsx_import.rs`, and `tests/shared_formula.rs`. |
| Raw function names found in LibreOffice FODS formulas | 511 | Static scan of `table:formula` attributes in 507 FODS files. |
| Function ids hit by LibreOffice FODS formulas | 471 | After mapping aliases and normalizing `COM.MICROSOFT.*`, `ORG.OPENOFFICE.*`, and `ORG.LIBREOFFICE.*` prefixes. Legacy `GAMMADIST` is counted separately from modern `GAMMA.DIST` because their LO/Excel edge semantics differ. |
| Function ids hit by any formula-test path | 476 | Non-FODS assertions plus FODS corpus after namespace-prefix normalization. |
| Dispatched function ids hit by any formula-test path | 435 | Current ordinary dispatch surface covered by active formula-test paths. |
| Dispatched function ids with no current formula-test hit | 2 | Listed below. |
| Static FODS function ids without ordinary dispatch | 41 | FODS formula ids that appear in static formulas but do not have a `dispatch.rs` branch. Some are special evaluator paths or non-asserted workbook formulas. |
| Static FODS function ids without any evaluator implementation reference | 34 | Conservative remaining implementation-audit gap after excluding special evaluator refs. |

Current any-path ordinary-dispatch gaps:

| Priority | Function id | Public names | Evidence | Action |
| --- | --- | --- | --- | --- |
| P0 | `ForecastDotEtsDotSeasonality` | `FORECAST.ETS.SEASONALITY` | Present in LibreOffice OOXML formula mapping and interpreter dispatch, but not present in copied FODS function formulas or POI formula tests. | Add an Excel/LO-backed evaluator assertion or a focused FODS/XLSX fixture before treating ETS seasonal detection as covered. |
| P0 | `ForecastDotEtsDotStat` | `FORECAST.ETS.STAT`, `FORECAST.ETS.STAT.ADD` | Present in LibreOffice OOXML formula mapping and interpreter dispatch. Current FODS coverage exercises `FORECAST.ETS.STAT.MULT`, not additive `STAT`. | Add an Excel/LO-backed evaluator assertion or a focused FODS/XLSX fixture. |

Important follow-up: `evaluation.rs` is the primary evaluator coverage table
and currently hits 213 function ids directly. The broader formula-test suite now
covers 435 of the 437 ordinary dispatched function ids through active
assertions and the LibreOffice FODS corpus. The active FODS regression lane is
green (`507` files, `3,139` assertions, `0` unsupported, `0` mismatched), but
the static FODS surface still contains implementation-audit gaps: 41
FODS-mapped function ids are not ordinary dispatch entries, and 34 do not have
any evaluator implementation reference in the current tree. FODS remains a
valuable corpus oracle, but it should not be treated as a substitute for
explicit, readable evaluator assertions or for static dispatch coverage. Add
small direct assertions for FODS-only functions, prioritizing high-risk
behavior: date serials, criteria coercion, lookup modes, matrix returns,
financial signs, text parsing, error propagation, and volatile/dynamic-array
semantics.

Recent passes closed FODS-backed gaps for `MODE.SNGL`, `MODE.MULT`, `PROB`,
`PDURATION`, `AMORDEGRC`, `AMORLINC`, `ODDLPRICE`, and `ODDLYIELD`, including
direct evaluator assertions for scalar cases, array returns, array-expression
arguments, LO column-major `MODE.MULT` ordering, `PDURATION` matrix
broadcasting, financial date/basis validation, and the Excel-vs-OpenFormula
`AMOR*` basis-2 split.

Historical check: implementation commit `854ae9e` still referenced about 475
formula function ids across the evaluator/dispatch path. The direct evaluator
refactor in `09cf9ac` reduced that count to about 261. The current working tree
has restored ordinary dispatch coverage from 324 to 437 ids and active
formula-test coverage from 320 to 435 dispatched ids, but the static FODS
implementation gap above should still be closed against the old commit plus
LibreOffice/POI/Excel before treating formula function coverage as complete.

Only SpreadsheetML/Calc formula behavior belongs here: formula text, addresses,
shared formulas, array/dynamic-array formulas, data tables, names, external
references, cached results, recalculation state, dependency state, and function
evaluation. Pure XML import/export containment checks are not listed unless the
assertion is formula semantics. Office Math, MathML, StarMath, UI-only, and
UNO-editing cases are listed as non-migratable boundary cases.

| Migrate | Case | Source Test | Fixture | What to port / reason not to port |
| --- | --- | --- | --- | --- |
| yes | token string round-trip | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCreateStringFromTokens` | synthetic | Formula token array to string serialization. |
| yes | A1/R1C1 reference parse | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaParseReference` | synthetic | Absolute/relative cell/range and sheet reference parsing. |
| yes | vector reference array fetch | `../core/sc/qa/unit/ucalc_formula.cxx::testFetchVectorRefArray` | synthetic | Vectorized reference/value access used by evaluator. |
| yes | 3D group conversion | `../core/sc/qa/unit/ucalc_formula.cxx::testGroupConverter3D` | synthetic | Multi-sheet/3D reference conversion. |
| yes | token equality | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaTokenEquality` | synthetic | Formula token comparison semantics. |
| yes | reference data | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefData` | synthetic | Reference token flags and address identity. |
| yes | compiler basics | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompiler` | synthetic | Formula text to compiled representation. |
| yes | jump token ordering | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompilerJumpReordering` | synthetic | Compiler jump-token order for conditional flow. |
| yes | implicit intersection, two params | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompilerImplicitIntersection2Param` | synthetic | Excel-style implicit intersection insertion. |
| yes | implicit intersection, unchanged one param | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompilerImplicitIntersection1ParamNoChange` | synthetic | No-op implicit intersection cases. |
| yes | implicit intersection, changed one param | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompilerImplicitIntersection1ParamWithChange` | synthetic | Required implicit intersection changes. |
| yes | implicit intersection, no group | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompilerImplicitIntersection1NoGroup` | synthetic | Ungrouped implicit intersection. |
| yes | implicit intersection operators | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaCompilerImplicitIntersectionOperators` | synthetic | Operator interactions with implicit intersection. |
| yes | trim double refs | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaAnnotateTrimOnDoubleRefs` | synthetic | Compiler annotation on double references. |
| yes | basic reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdate` | synthetic | Formula reference rewrite model. |
| yes | range reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateRange` | synthetic | Range rewrite after structural edits. |
| yes | sheet reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateSheets` | synthetic | Sheet insert/move/update references. |
| yes | insert-row reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateInsertRows` | synthetic | Row insertion reference adjustment. |
| yes | delete-sheet reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateSheetsDelete` | synthetic | Sheet deletion reference adjustment. |
| yes | insert-column reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateInsertColumns` | synthetic | Column insertion reference adjustment. |
| yes | move reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMove` | synthetic | Move operation reference rewrite. |
| yes | move undo reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveUndo` | synthetic | Undo state for moved references. |
| yes | move undo reference update 2 | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveUndo2` | synthetic | Additional move/undo rewrite case. |
| yes | move undo non-shared | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveUndo3NonShared` | synthetic | Non-shared formula move/undo. |
| yes | move undo shared | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveUndo3Shared` | synthetic | Shared formula move/undo. |
| yes | move undo dependents | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveUndoDependents` | synthetic | Dependent formula update after move/undo. |
| yes | move undo reference update 4 | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveUndo4` | synthetic | Additional move/undo rewrite case. |
| yes | move to sheet | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateMoveToSheet` | synthetic | Cross-sheet move rewrite. |
| yes | delete content | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateDeleteContent` | synthetic | Reference behavior after content deletion. |
| yes | delete and shift left | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateDeleteAndShiftLeft` | synthetic | Delete/shift-left rewrite. |
| yes | delete and shift left 2 | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateDeleteAndShiftLeft2` | synthetic | Additional delete/shift-left case. |
| yes | delete and shift up | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateDeleteAndShiftUp` | synthetic | Delete/shift-up rewrite. |
| yes | named expression reference update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateName` | synthetic | Named expression reference rewrite. |
| yes | named expression move | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateNameMove` | synthetic | Named expression rewrite after move. |
| yes | named expression expand ref | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateNameExpandRef` | synthetic | Named range expansion. |
| yes | named expression expand ref 2 | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateNameExpandRef2` | synthetic | Additional named range expansion case. |
| yes | named expression delete row | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateNameDeleteRow` | synthetic | Named range row deletion. |
| yes | named expression copy sheet | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateNameCopySheet` | synthetic | Named range behavior on copied sheets. |
| yes | sheet-local named expression move | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateSheetLocalMove` | synthetic | Sheet-local named expression rewrite. |
| yes | named expression delete | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateNameDelete` | synthetic | Name deletion propagation. |
| yes | validation formula update | `../core/sc/qa/unit/ucalc_formula.cxx::testFormulaRefUpdateValidity` | synthetic | Data-validation formula reference rewrite. |
| yes | token array move update | `../core/sc/qa/unit/ucalc_formula.cxx::testTokenArrayRefUpdateMove` | synthetic | Low-level token array move rewrite. |
| yes | multiple operations | `../core/sc/qa/unit/ucalc_formula.cxx::testMultipleOperations` | synthetic | Data-table/multiple-operation formula model. |
| yes | COLUMN | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncCOLUMN` | synthetic | `COLUMN` evaluation. |
| yes | COUNT | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncCOUNT` | synthetic | `COUNT` evaluation. |
| yes | COUNTBLANK | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncCOUNTBLANK` | synthetic | `COUNTBLANK` evaluation. |
| yes | ROW | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncROW` | synthetic | `ROW` evaluation. |
| yes | SUM | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUM` | synthetic | `SUM` evaluation. |
| yes | PRODUCT | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncPRODUCT` | synthetic | `PRODUCT` evaluation. |
| yes | SUMPRODUCT | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUMPRODUCT` | synthetic | `SUMPRODUCT` evaluation. |
| yes | SUBTOTAL | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUBTOTAL` | synthetic | Covered in `evaluation.rs`; relative named-expression rewrite portions remain structural/name-reference API gaps. |
| yes | SUBTOTAL reference immutability | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUBTOTALReferenceNotMutated` | synthetic | Covered for LO result over oversized range; token reference immutability inspection is blocked on token APIs. |
| yes | SUMXMY2 | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUMXMY2` | synthetic | `SUMXMY2` evaluation. |
| yes | MIN | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncMIN` | synthetic | `MIN` evaluation. |
| yes | N | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncN` | synthetic | `N` evaluation. |
| yes | COUNTIF | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncCOUNTIF` | synthetic | `COUNTIF` evaluation. |
| yes | row/column labels | `../core/sc/qa/unit/ucalc_formula.cxx::testInsertRowColLabel` | synthetic | Label behavior used by formula references. |
| yes | IF | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncIF` | synthetic | `IF` evaluation. |
| yes | CHOOSE | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCHOOSE` | synthetic | `CHOOSE` evaluation. |
| yes | IFERROR | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncIFERROR` | synthetic | `IFERROR` evaluation. |
| yes | SHEET | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSHEET` | synthetic | Covered in `evaluation.rs` for sheet count and sheet index. |
| yes | NOW | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncNOW` | synthetic | Volatile `NOW` handling. |
| yes | NUMBERVALUE | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncNUMBERVALUE` | synthetic | Locale-aware number text parsing. |
| yes | LEN | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncLEN` | synthetic | Text length evaluation. |
| yes | LOOKUP | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncLOOKUP` | synthetic | `LOOKUP` evaluation. |
| yes | LOOKUP array with error | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncLOOKUParrayWithError` | synthetic | Lookup with error values. |
| yes | tdf141146 function regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf141146` | synthetic | Formula regression; port expected LO behavior. |
| yes | VLOOKUP | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncVLOOKUP` | synthetic | `VLOOKUP` evaluation. |
| yes | MATCH | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncMATCH` | synthetic | `MATCH` evaluation. |
| yes | CELL | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCELL` | synthetic | `CELL` evaluation. |
| yes | DATEDIF | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncDATEDIF` | synthetic | `DATEDIF` evaluation. |
| yes | INDIRECT | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncINDIRECT` | synthetic | `INDIRECT` parsing/evaluation. |
| yes | INDIRECT 2 | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncINDIRECT2` | synthetic | Additional `INDIRECT` behavior. |
| yes | MATCH with INDIRECT | `../core/sc/qa/unit/ucalc_formula2.cxx::testFunc_MATCH_INDIRECT` | synthetic | Reference function interaction. |
| yes | dependency tracking | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaDepTracking` | synthetic | Dependency graph state. |
| yes | dependency tracking 2 | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaDepTracking2` | synthetic | Additional dependency graph state. |
| yes | dependency tracking 3 | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaDepTracking3` | synthetic | Additional dependency graph state. |
| yes | dependency tracking delete row | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaDepTrackingDeleteRow` | synthetic | Dependency graph after row deletion. |
| yes | dependency tracking delete column | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaDepTrackingDeleteCol` | synthetic | Dependency graph after column deletion. |
| yes | matrix result update | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaMatrixResultUpdate` | synthetic | Matrix formula result update. |
| yes | external reference | `../core/sc/qa/unit/ucalc_formula2.cxx::testExternalRef` | synthetic | External workbook reference model. |
| yes | external range name | `../core/sc/qa/unit/ucalc_formula2.cxx::testExternalRangeName` | synthetic | External named range model. |
| yes | external reference functions | `../core/sc/qa/unit/ucalc_formula2.cxx::testExternalRefFunctions` | synthetic | Functions over external refs. |
| yes | unresolved external ref | `../core/sc/qa/unit/ucalc_formula2.cxx::testExternalRefUnresolved` | synthetic | Unresolved external reference behavior. |
| yes | matrix operator | `../core/sc/qa/unit/ucalc_formula2.cxx::testMatrixOp` | synthetic | Matrix operations. |
| yes | range operator | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncRangeOp` | synthetic | Covered in `evaluation.rs`; active failures expose range-operator parser/evaluator gaps. |
| yes | FORMULA | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncFORMULA` | synthetic | `FORMULA` introspection. |
| yes | table references | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncTableRef` | synthetic | Structured table reference parsing/evaluation. |
| yes | FTEST | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncFTEST` | synthetic | Statistical function evaluation. |
| yes | FTEST bug regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncFTESTBug` | synthetic | Statistical function regression. |
| yes | CHITEST | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCHITEST` | synthetic | Statistical function evaluation. |
| yes | TTEST | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncTTEST` | synthetic | Statistical function evaluation. |
| yes | SUMX2PY2 | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSUMX2PY2` | synthetic | Math function evaluation. |
| yes | SUMX2MY2 | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSUMX2MY2` | synthetic | Math function evaluation. |
| yes | GCD | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncGCD` | synthetic | Math function evaluation. |
| yes | LCM | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncLCM` | synthetic | Math function evaluation. |
| yes | SUMSQ | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSUMSQ` | synthetic | Math function evaluation. |
| yes | MDETERM | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncMDETERM` | synthetic | Matrix determinant evaluation. |
| yes | error propagation | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaErrorPropagation` | synthetic | Error value propagation. |
| yes | tdf97369 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf97369` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf97587 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf97587` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf93415 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf93415` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf132519 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf132519` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf127334 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf127334` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf100818 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf100818` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf147398 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf147398` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | tdf156985 regression | `../core/sc/qa/unit/ucalc_formula2.cxx::testTdf156985` | synthetic | Calc formula regression; port expected LO behavior. |
| yes | matrix concatenation | `../core/sc/qa/unit/ucalc_formula2.cxx::testMatConcat` | synthetic | Covered in `evaluation.rs`; active failure exposes matrix text-concatenation gap. |
| yes | matrix concatenation replication | `../core/sc/qa/unit/ucalc_formula2.cxx::testMatConcatReplication` | synthetic | Covered in `evaluation.rs`; active failure exposes matrix text-concatenation/broadcast gap. |
| yes | R1C1 whole-column ref | `../core/sc/qa/unit/ucalc_formula2.cxx::testRefR1C1WholeCol` | synthetic | Whole-column R1C1 parsing. |
| yes | R1C1 whole-row ref | `../core/sc/qa/unit/ucalc_formula2.cxx::testRefR1C1WholeRow` | synthetic | Whole-row R1C1 parsing. |
| yes | copied column label | `../core/sc/qa/unit/ucalc_formula2.cxx::testSingleCellCopyColumnLabel` | synthetic | Column-label formula behavior. |
| yes | Excel intersection | `../core/sc/qa/unit/ucalc_formula2.cxx::testIntersectionOpExcel` | synthetic | Covered in `evaluation.rs`. |
| yes | hidden rows | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncRowsHidden` | synthetic | Covered in `evaluation.rs` for `SUBTOTAL`, `AGGREGATE`, and `SUM`. |
| yes | SUMIFS | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSUMIFS` | synthetic | Conditional aggregate evaluation. |
| yes | COUNTIF empty | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCOUNTIFEmpty` | synthetic | Empty-cell conditional count behavior. |
| yes | COUNTIFS range reduce | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCOUNTIFSRangeReduce` | synthetic | Conditional count range reduction. |
| yes | reference list array SUBTOTAL | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncRefListArraySUBTOTAL` | synthetic | Covered in `evaluation.rs`; active failure exposes ref-list array `SUBTOTAL`/`OFFSET` gap. |
| yes | jump matrix array IF | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncJumpMatrixArrayIF` | synthetic | Covered in `evaluation.rs`; active failure exposes matrix array-context `IF` gap. |
| yes | jump matrix array OFFSET | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncJumpMatrixArrayOFFSET` | synthetic | Covered in `evaluation.rs`; active failure exposes matrix array-context `OFFSET` gap. |
| yes | iterative calculation | `../core/sc/qa/unit/ucalc_formula2.cxx::testIterations` | synthetic | Iteration/recalculation behavior. |
| yes | insert-column cell-store event swap | `../core/sc/qa/unit/ucalc_formula2.cxx::testInsertColCellStoreEventSwap` | synthetic | Formula state after column insertion/storage swap. |
| yes | delete-row aftermath | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaAfterDeleteRows` | synthetic | Formula state after row deletion. |
| yes | XLOOKUP regex | `../core/sc/qa/unit/ucalc_formula2.cxx::testRegexForXLOOKUP` | synthetic | Covered in `evaluation.rs`; active failure exposes missing `XLOOKUP` regex evaluation. |
| yes | horizontal query empty cell | `../core/sc/qa/unit/ucalc_formula2.cxx::testHoriQueryEmptyCell` | synthetic | Covered in `evaluation.rs`; active failure exposes empty-cell `XLOOKUP` reference behavior gap. |
| yes | vertical query empty cell | `../core/sc/qa/unit/ucalc_formula2.cxx::testVertQueryEmptyCell` | synthetic | Covered in `evaluation.rs`. |
| yes | spill error basic | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorBasic` | synthetic | Dynamic array `#SPILL!` behavior. |
| yes | spill error no blocker | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorNoBlockingData` | synthetic | Dynamic array without blocking data. |
| yes | spill error multi column | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorMultiColumn` | synthetic | Multi-column spill behavior. |
| yes | spill disabled by default | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorDisabledByDefault` | synthetic | Default spill behavior. |
| yes | spill error single cell | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorSingleCell` | synthetic | Single-cell spill behavior. |
| yes | spill auto expand | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorAutoExpand` | synthetic | Auto-expand spill behavior. |
| yes | spill auto expand empty | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorAutoExpandEmpty` | synthetic | Auto-expand into empty cells. |
| yes | spill overwrites | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillErrorOverwrites` | synthetic | Overwrite-blocked spill behavior. |
| yes | spill resolves after blocker delete | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixResolveAfterBlockerDelete` | synthetic | Spill matrix after blocker deletion. |
| yes | sequence resolves after blocker delete | `../core/sc/qa/unit/ucalc_formula2.cxx::testSequenceFormulaResolveAfterBlockerIsDeleted` | synthetic | Sequence formula after blocker deletion. |
| yes | spill collapses on ref edit | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixCollapsesOnRefCellEdit` | synthetic | Spill matrix collapse after input edit. |
| yes | write to master replaces matrix | `../core/sc/qa/unit/ucalc_formula2.cxx::testWriteToMasterReplacesWholeMatrix` | synthetic | Master write replaces whole matrix. |
| yes | delete master deletes matrix | `../core/sc/qa/unit/ucalc_formula2.cxx::testDeleteMasterDeletesWholeMatrix` | synthetic | Master deletion removes matrix. |
| yes | spill undo redo ref blocker | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixUndoRedoRefCellBlocker` | synthetic | Spill undo/redo with blocker. |
| yes | spill undo restores tracking | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixUndoOfDeleteRestoresTracking` | synthetic | Spill dependency tracking after undo. |
| yes | spill contraction on value change | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixContractionOnValueChange` | synthetic | Spill range contraction. |
| yes | spill auto resolve on value change | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixAutoResolveOnValueChange` | synthetic | Spill auto-resolution. |
| yes | spill complex scenario | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixComplexScenario` | synthetic | Complex dynamic array scenario. |
| yes | spill undo redo blocker delete | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixUndoRedoBlockerDelete` | synthetic | Undo/redo after blocker deletion. |
| yes | spill undo redo input change | `../core/sc/qa/unit/ucalc_formula2.cxx::testSpillMatrixUndoRedoInputChange` | synthetic | Undo/redo after input change. |
| yes | dynamic array flag copy | `../core/sc/qa/unit/ucalc_formula2.cxx::testDynamicArrayFlagCopy` | synthetic | Dynamic array flag copy. |
| yes | single value operator | `../core/sc/qa/unit/ucalc_formula2.cxx::testSingleValueOperator` | synthetic | Covered in `evaluation.rs`; active failure exposes missing `@` operator parsing/evaluation. |
| yes | dynamic array master survives cell copy | `../core/sc/qa/unit/ucalc_formula2.cxx::testDynamicArrayMasterSurvivesCellCopy` | synthetic | Dynamic array master state after cell copy. |
| yes | dynamic array master grows on recalc | `../core/sc/qa/unit/ucalc_formula2.cxx::testDynamicArrayMasterGrowsOnRecalc` | synthetic | Dynamic array master resize during recalculation. |
| yes | dynamic array resize copy to clipboard | `../core/sc/qa/unit/ucalc_formula2.cxx::testDynamicArrayResizeDuringCopyToClip` | synthetic | Dynamic array resize during clipboard copy. |
| yes | dynamic array resize static copy | `../core/sc/qa/unit/ucalc_formula2.cxx::testDynamicArrayResizeDuringCopyStaticToDocument` | synthetic | Dynamic array resize during static document copy. |
| yes | dynamic array resize updated copy | `../core/sc/qa/unit/ucalc_formula2.cxx::testDynamicArrayResizeDuringCopyUpdated` | synthetic | Dynamic array resize after updated copy. |
| yes | shared formula basics | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulas` | synthetic | Shared formula grouping and expansion. |
| yes | shared formula ref update | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdate` | synthetic | Shared formula ref rewrite. |
| yes | shared formula ref update move | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateMove` | synthetic | Shared formula move rewrite. |
| yes | shared formula ref update move 2 | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateMove2` | synthetic | Additional shared formula move rewrite. |
| yes | shared formula ref update range | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateRange` | synthetic | Shared formula range rewrite. |
| yes | shared formula ref update delete row | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateRangeDeleteRow` | synthetic | Shared formula row-delete rewrite. |
| yes | shared formula ref update external | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateExternal` | synthetic | Shared formula external reference rewrite. |
| yes | shared formula insert row | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasInsertRow` | synthetic | Shared formula state after row insertion. |
| yes | shared formula delete rows | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasDeleteRows` | synthetic | Shared formula state after row deletion. |
| yes | shared formula delete columns | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasDeleteColumns` | synthetic | Shared formula state after column deletion. |
| yes | shared formula insert column | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaInsertColumn` | synthetic | Shared formula state after column insertion. |
| yes | shared formula insert shift | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaInsertShift` | synthetic | Shared formula state after insert shift. |
| yes | shared formula move sheets | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateMoveSheets` | synthetic | Shared formula sheet move rewrite. |
| yes | shared formula copy sheets | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateCopySheets` | synthetic | Shared formula sheet copy rewrite. |
| yes | shared formula delete sheets | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasRefUpdateDeleteSheets` | synthetic | Shared formula sheet delete rewrite. |
| yes | shared formula copy paste | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulasCopyPaste` | synthetic | Shared formula copy/paste behavior. |
| yes | shared formula move block | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaMoveBlock` | synthetic | Shared formula block move. |
| yes | shared formula cut/copy move into ref | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaCutCopyMoveIntoRef` | synthetic | Shared formula cut/copy into referenced area. |
| yes | shared formula cut/copy move with ref | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaCutCopyMoveWithRef` | synthetic | Shared formula cut/copy with referenced area. |
| yes | shared formula cut/copy move within run | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaCutCopyMoveWithinRun` | synthetic | Shared formula cut/copy within formula run. |
| yes | shared formula named range change | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaUpdateOnNamedRangeChange` | synthetic | Shared formula dependency on named range. |
| yes | shared formula database range change | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaUpdateOnDBChange` | synthetic | Shared formula dependency on database range. |
| yes | shared formula absolute cell listener | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaAbsCellListener` | synthetic | Shared formula listener behavior. |
| yes | shared formula unshare area listeners | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaUnshareAreaListeners` | synthetic | Shared formula listener behavior after unshare. |
| yes | shared formula listener delete area | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaListenerDeleteArea` | synthetic | Shared formula listener after area deletion. |
| yes | shared formula replacement update | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaUpdateOnReplacement` | synthetic | Shared formula replacement behavior. |
| yes | shared formula delete top cell | `../core/sc/qa/unit/ucalc_sharedformula.cxx::testSharedFormulaDeleteTopCell` | synthetic | Shared formula top-cell deletion. |
| yes | add-in function corpus | `../core/sc/qa/unit/functions_addin.cxx::testAddinFormulasFODS` | `../core/sc/qa/unit/data/functions/addin/fods/` | 49 engineering/add-in formula fixtures. |
| yes | array function corpus | `../core/sc/qa/unit/functions_array.cxx::testArrayFormulasFODS` | `../core/sc/qa/unit/data/functions/array/fods/` | 13 array formula fixtures. |
| yes | dubious array function corpus | `../core/sc/qa/unit/functions_array.cxx::testDubiousArrayFormulasFODS` | `../core/sc/qa/unit/data/functions/array/dubious/fods/` | 2 edge-case array fixtures. |
| yes | database function corpus | `../core/sc/qa/unit/functions_database.cxx::testDatabaseFormulasFODS` | `../core/sc/qa/unit/data/functions/database/fods/` | 12 database formula fixtures. |
| yes | date/time function corpus | `../core/sc/qa/unit/functions_datetime.cxx::testDateTimeFormulasFODS` | `../core/sc/qa/unit/data/functions/date_time/fods/` | 32 date/time formula fixtures. |
| yes | financial function corpus | `../core/sc/qa/unit/functions_financial.cxx::testFinancialFormulasFODS` | `../core/sc/qa/unit/data/functions/financial/fods/` | 51 financial formula fixtures. |
| yes | information function corpus | `../core/sc/qa/unit/functions_information.cxx::testInformationFormulasFODS` | `../core/sc/qa/unit/data/functions/information/fods/` | 20 information formula fixtures. |
| yes | logical function corpus | `../core/sc/qa/unit/functions_logical.cxx::testLogicalFormulasFODS` | `../core/sc/qa/unit/data/functions/logical/fods/` | 9 logical formula fixtures. |
| yes | mathematical function corpus | `../core/sc/qa/unit/functions_mathematical.cxx::testMathematicalFormulasFODS` | `../core/sc/qa/unit/data/functions/mathematical/fods/` | 79 mathematical formula fixtures. |
| yes | old/mixed function corpus | `../core/sc/qa/unit/functions_old.cxx::testFormulasFODS` | `../core/sc/qa/unit/data/functions/fods/` | 5 mixed formula fixtures. |
| yes | spreadsheet function corpus | `../core/sc/qa/unit/functions_spreadsheet.cxx::testSpreadsheetFormulasFODS` | `../core/sc/qa/unit/data/functions/spreadsheet/fods/` | 44 lookup/reference/dynamic-array formula fixtures. |
| yes | statistical function corpus | `../core/sc/qa/unit/functions_statistical.cxx::testStatisticalFormulasFODS` | `../core/sc/qa/unit/data/functions/statistical/fods/` | 147 statistical formula fixtures. |
| yes | text function corpus | `../core/sc/qa/unit/functions_text.cxx::testTextFormulasFODS` | `../core/sc/qa/unit/data/functions/text/fods/` | 44 text formula fixtures. |
| yes | tdf143809 formula import | `../core/sc/qa/unit/subsequent_filters_test.cxx::testTdf143809` | LO fixture in `sc/qa/unit/data/` | Formula text import behavior, not raw XML. |
| yes | tdf76310 formula import | `../core/sc/qa/unit/subsequent_filters_test.cxx::testTdf76310` | LO fixture in `sc/qa/unit/data/` | Formula text import behavior, not raw XML. |
| yes | global range name XLS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testRangeNameXLS` | LO XLS fixture | Named range formula semantics. |
| yes | local range name XLS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testRangeNameLocalXLS` | LO XLS fixture | Sheet-local named range formulas. |
| yes | global range name XLSX | `../core/sc/qa/unit/subsequent_filters_test.cxx::testRangeNameXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/named-ranges-global.xlsx` | Named range formula semantics. |
| yes | global range name ODS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testRangeNameODS` | LO ODS fixture | Named range formula semantics. |
| yes | hidden range name ODS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testHiddenRangeNameODS` | LO ODS fixture | Hidden named expression semantics. |
| yes | hidden range name XLSX | `../core/sc/qa/unit/subsequent_filters_test.cxx::testHiddenRangeNameXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/named-ranges-hidden.xlsx` | Hidden named expression semantics. |
| yes | hidden named expression | `../core/sc/qa/unit/subsequent_filters_test.cxx::testHiddenNamedExpression` | generated | Hidden named expression export/import behavior. |
| yes | hidden named expression ODS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testHiddenNamedExpressionODS` | LO ODS fixture | Hidden named expression import. |
| yes | hard recalculation ODS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testHardRecalcODS` | LO ODS fixture | Recalc state preservation. |
| yes | cached formula results functions | `../core/sc/qa/unit/subsequent_filters_test.cxx::testCachedFormulaResultsODS_functions` | LO ODS fixture | Cached formula result preservation. |
| yes | cached formula results value | `../core/sc/qa/unit/subsequent_filters_test.cxx::testCachedFormulaResultsODS_cachedValue` | LO ODS fixture | Cached formula value preservation. |
| yes | cached matrix formula results | `../core/sc/qa/unit/subsequent_filters_test.cxx::testCachedMatrixFormulaResultsODS` | LO ODS fixture | Cached matrix formula results. |
| yes | functions ODS import | `../core/sc/qa/unit/subsequent_filters_test.cxx::testFunctionsODS` | LO ODS fixture | Function formula import. |
| yes | database functions ODS import | `../core/sc/qa/unit/subsequent_filters_test.cxx::testFunctionsODS_databaseFunctions` | LO ODS fixture | Database function formula import. |
| yes | date-time functions ODS import | `../core/sc/qa/unit/subsequent_filters_test.cxx::testFunctionsODS_dateTimeFunctions` | LO ODS fixture | Date/time function formula import. |
| yes | user-defined functions ODS import | `../core/sc/qa/unit/subsequent_filters_test.cxx::testFunctionsODS_usedDefinedFunctions` | LO ODS fixture | UDF formula preservation. |
| yes | formula dependency across sheets | `../core/sc/qa/unit/subsequent_filters_test.cxx::testFormulaDepAcrossSheetsODS` | LO ODS fixture | Dependency state after import. |
| yes | formula dependency delete contents | `../core/sc/qa/unit/subsequent_filters_test.cxx::testFormulaDepDeleteContentsODS` | LO ODS fixture | Dependency state after delete. |
| yes | matrix formulas ODS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testMatrixODS` | LO ODS fixture | Matrix formula import. |
| yes | matrix formulas XLS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testMatrixXLS` | LO XLS fixture | Matrix formula import. |
| yes | data table mortgage XLS | `../core/sc/qa/unit/subsequent_filters_test.cxx::testDataTableMortgageXLS` | LO XLS fixture | Data-table formula import. |
| yes | data table one variable XLSX | `../core/sc/qa/unit/subsequent_filters_test.cxx::testDataTableOneVarXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/data-table/one-variable.xlsx` | Data-table formula import. |
| yes | data table multi table XLSX | `../core/sc/qa/unit/subsequent_filters_test.cxx::testDataTableMultiTableXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/data-table/multi-table.xlsx` | Data-table formula import. |
| yes | array formula spill XLSX | `../core/sc/qa/unit/subsequent_filters_test.cxx::testArrayFormulaSpillXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/Spill.xlsx` | Dynamic array/spill import. |
| yes | conventional array formula spill XLSX | `../core/sc/qa/unit/subsequent_filters_test.cxx::testConventionalArrayFormulaSpillXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/Spill.xlsx` | Conventional array/spill import. |
| yes | expanded array collapses on blocker | `../core/sc/qa/unit/subsequent_filters_test.cxx::testExpandedArrayCollapsesOnNewBlockerXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/Spill.xlsx` | Dynamic array blocker behavior. |
| yes | shared formula horizontal XLS | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaHorizontalXLS` | LO XLS fixture | Shared formula import. |
| yes | shared formula wrapped refs XLS | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaWrappedRefsXLS` | LO XLS fixture | Shared formula wrapped references. |
| yes | shared formula BIFF5 | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaBIFF5` | LO XLS fixture | BIFF5 shared formula import. |
| yes | shared formula XLSB | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaXLSB` | LO XLSB fixture | XLSB shared formula import. |
| yes | shared formula fdo80091 | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaXLS_fdo80091` | LO XLS fixture | Shared formula relative refs. |
| yes | shared formula fdo84556 | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaXLS_fdo84556` | LO XLS fixture | Shared formula relative refs. |
| yes | shared formula column labels ODS | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaColumnLabelsODS` | LO ODS fixture | Shared formula with labels. |
| yes | shared formula column-row labels ODS | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testSharedFormulaColumnRowLabelsODS` | LO ODS fixture | Shared formula with column/row labels. |
| yes | shared formula XLS import | `../core/sc/qa/unit/subsequent_filters_test4.cxx::testSharedFormulaXLS` | LO XLS fixture | Shared formula import. |
| yes | shared formula XLS import 2 | `../core/sc/qa/unit/subsequent_filters_test4.cxx::testSharedFormulaXLS2` | LO XLS fixture | Additional shared formula import. |
| yes | shared formula XLSX import | `../core/sc/qa/unit/subsequent_filters_test4.cxx::testSharedFormulaXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/shared-formula/basic.xlsx` | Shared formula group import state; initial state is covered in Rust. |
| yes | shared formula XLSX ref-update import state | `../core/sc/qa/unit/subsequent_filters_test4.cxx::testSharedFormulaRefUpdateXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/shared-formula/refupdate.xlsx` | Pre-edit shared formula import state; LO row-delete rewrite is a structural reference-update gap. |
| yes | external reference cache XLSX | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testExternalRefCacheXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/external-refs.xlsx` | External reference cache import. |
| yes | external reference cache ODS | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testExternalRefCacheODS` | LO ODS fixture | External reference cache import. |
| yes | VBA/UDF formulas | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testVBAUserFunctionXLSM` | LO XLSM fixture | VBA user-function formula text. |
| yes | unresolved external references | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testErrorOnExternalReferences` | LO fixture | Error handling for unresolved externals. |
| yes | tdf160371 formula string | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf160371` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tdf160371.xlsx` | Covered in `xlsx_import.rs`; active failure exposes LO import normalization gap from space intersection to `!`. |
| yes | tdf136364 formula string | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf136364` | LO fixture | Imported formula string/reference semantics. |
| yes | tdf131424 table reference cache | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf131424` | LO XLSX fixture | Structured/table-reference formula cached results; covered in Rust. |
| yes | tdf85617 implicit intersection cache | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf85617` | LO XLSX fixture | Implicit-intersection formula cached result; covered in Rust. |
| yes | reference string XLSX | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testRefStringXLSX` | LO XLSX fixture | Reference-string formula cached result; covered in Rust. |
| yes | tdf131536 formula string | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf131536` | LO fixture | Imported formula string/reference semantics. |
| no | Excel XML named expressions global | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testNamedExpressionsXLSXML_Global` | LO XML fixture | Not OOXML package coverage for this test-suite migration. |
| no | Excel XML named expressions local | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testNamedExpressionsXLSXML_Local` | LO XML fixture | Not OOXML package coverage for this test-suite migration. |
| no | Excel XML empty rows | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testEmptyRowsXLSXML` | LO XML fixture | Not OOXML package coverage for this test-suite migration. |
| yes | named table references | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testNamedTableRef` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tablerefsnamed.xlsx` | Covered in `xlsx_import.rs`. |
| yes | conditional-format formula listener | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testCondFormatFormulaListenerXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/cond_format_formula_listener.xlsx` | Formula dependency/listener behavior if exposed by formula model. |
| yes | formula text regression | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf112780` | LO fixture | Formula import regression. |
| yes | VBA macro function import | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testVBAMacroFunctionODS` | LO ODS fixture | Macro function formula preservation. |
| yes | tdf137091 fraction display | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf137091` | LO XLSX fixture | Formula display text regression; covered in Rust. |
| yes | tdf141495 add-in date display | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf141495` | LO XLSX fixture | Formula display text regression; covered in Rust. |
| yes | tdf70455 currency display | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf70455` | LO XLSX fixture | Formula display text regression; covered in Rust. |
| yes | tdf98481 lookup caches | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf98481` | LO XLSX fixture | `LOOKUP` formula cached result regression; covered in Rust. |
| yes | tdf115022 SUMIF cache | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf115022` | LO XLSX fixture | `SUMIF` formula cached result regression; covered in Rust. |
| yes | LOOKUP external ref | `../core/sc/qa/unit/subsequent_filters_test5.cxx::testTdf167134_LOOKUP_extRef` | LO FODS fixtures | External reference lookup semantics. |
| yes | named range formula | `../core/sc/qa/unit/subsequent_filters_test5.cxx::testTdf94627` | LO XLSB fixture | Named range formula preservation. |
| yes | full-column refs | `../core/sc/qa/unit/subsequent_filters_test5.cxx::testFullColumnRefs` | LO fixture | Full-column formula references. |
| no | formula XML export node | `../core/sc/qa/unit/subsequent_export_test.cxx::testTdf90104` | LO XLSX fixture | Pure XML export shape; `ooxmlsdk` package/schema tests already cover raw XML. |
| yes | EASTERSUNDAY export semantics | `../core/sc/qa/unit/subsequent_export_test.cxx::testTdf162177_EastersundayODF14` | LO FODS fixture | Function name/namespace semantics if evaluator/export supports it. |
| yes | named range export regression | `../core/sc/qa/unit/subsequent_export_test.cxx::testNamedRangeBugfdo62729` | LO ODS fixture | Named range formula export semantics. |
| yes | built-in ranges | `../core/sc/qa/unit/subsequent_export_test.cxx::testBuiltinRangesXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/built-in_ranges.xlsx` | Built-in named ranges. |
| yes | table total formula cache | `../core/sc/qa/unit/subsequent_export_test.cxx::testTdf162963` | LO XLSX fixture | Table total-row formula/cached result; covered in Rust. |
| yes | table total formula ODF cache | `../core/sc/qa/unit/subsequent_export_test.cxx::testTdf162963_ODF` | LO ODS fixture | Table total-row formula/cached result. |
| yes | quoted sheet name | `../core/sc/qa/unit/subsequent_export_test.cxx::testFormulaRefSheetNameODS` | LO ODS fixture | Formula sheet-name quoting. |
| yes | generated formula values | `../core/sc/qa/unit/subsequent_export_test.cxx::testCellValuesExportODS` | generated | Formula string/value round-trip behavior. |
| yes | inline array XLS | `../core/sc/qa/unit/subsequent_export_test.cxx::testInlineArrayXLS` | LO XLS fixture | Inline array formula import/export semantics. |
| yes | formula references XLS | `../core/sc/qa/unit/subsequent_export_test.cxx::testFormulaReferenceXLS` | LO XLS fixture | Absolute/relative/3D reference formulas. |
| yes | matrix multiplication XLSX | `../core/sc/qa/unit/subsequent_export_test2.cxx::testMatrixMultiplicationXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/matrix-multiplication.xlsx` | Matrix multiplication formula result/import. |
| yes | structured reference export tdf105272 | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf105272` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tdf105272.xlsx` | Covered in `xlsx_import.rs`. |
| yes | structured reference export tdf118990 | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf118990` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tdf118990.xlsx` | Covered in `xlsx_import.rs`; active failure exposes external-link formula path normalization gap. |
| yes | validation formula copy/paste | `../core/sc/qa/unit/subsequent_export_test2.cxx::testValidationCopyPaste` | generated | Data-validation formula reference behavior. |
| yes | empty values in array formulas | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf170201_empty_values_in_array_formulas` | LO XLSX fixture | Array formula empty-value preservation. |
| yes | hyperlink formula | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf126024XLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/hyperlink_formula.xlsx` | `HYPERLINK` formula preservation. |
| yes | illegal OFFSET parameter | `../core/sc/qa/unit/subsequent_export_test2.cxx::testOffsetIllegalParam` | LO ODS fixture | Formula error behavior. |
| yes | TRUE/FALSE as defined names | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTrueFalseAsDefinedName` | LO XLS fixture | Defined-name parsing conflict. |
| yes | spill error round-trip XLSX | `../core/sc/qa/unit/subsequent_export_test2.cxx::testSpillErrorRoundtripXLSX` | generated | `#SPILL!` formula error preservation. |
| yes | spill error round-trip ODS | `../core/sc/qa/unit/subsequent_export_test2.cxx::testSpillErrorRoundtripODS` | generated | `#SPILL!` formula error preservation. |
| yes | array formula spill round-trip XLSX | `../core/sc/qa/unit/subsequent_export_test2.cxx::testArrayFormulaSpillRoundtripXLSX` | generated | Dynamic array formula and spill preservation. |
| yes | array formula spill round-trip ODS | `../core/sc/qa/unit/subsequent_export_test2.cxx::testArrayFormulaSpillRoundtripODS` | generated | Dynamic array formula and spill preservation. |
| yes | shared formula export XLS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSharedFormulaExportXLS` | LO XLS fixture | Shared formula export. |
| yes | shared formula export XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSharedFormulaExportXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/shared-formula/3d-reference.xlsx` | Shared formula export. |
| yes | shared formula string result export XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSharedFormulaStringResultExportXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/shared-formula/text-results.xlsx` | Shared formula cached string results. |
| yes | Excel 2010 functions XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testFunctionsExcel2010XLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/functions-excel-2010.xlsx` | Excel 2010 formula functions and no-error checks. |
| yes | Excel 2010 functions XLS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testFunctionsExcel2010XLS` | LO XLS fixture | Excel 2010 formula functions and no-error checks. |
| yes | Excel 2010 functions ODS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testFunctionsExcel2010ODS` | LO ODS fixture | Excel 2010 formula functions and no-error checks. |
| yes | defined-name formula text | `../core/sc/qa/unit/subsequent_export_test3.cxx::testForumMsoEn4145327` | LO XLSX fixture | Defined-name formula text import; covered in Rust. |
| yes | CEILING/FLOOR XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/ceiling-floor.xlsx` | `CEILING`/`FLOOR` compatibility semantics. |
| yes | CEILING/FLOOR XLS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorXLS` | LO XLS fixture | `CEILING`/`FLOOR` compatibility semantics. |
| yes | CEILING/FLOOR ODS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorODS` | LO ODS fixture | `CEILING`/`FLOOR` compatibility semantics. |
| yes | CEILING/FLOOR ODS to XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorODSToXLSX` | LO ODS fixture | `CEILING`/`FLOOR` compatibility semantics. |
| yes | external virtual path | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSupBookVirtualPathXLS` | LO XLS fixture | External workbook path/formula preservation. |
| yes | sheet-local range name | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSheetLocalRangeNameXLS` | LO XLS fixture | Sheet-local named formulas. |
| yes | relative named expressions | `../core/sc/qa/unit/subsequent_export_test3.cxx::testRelativeNamedExpressionsXLS` | LO ODS fixture | Relative named expressions. |
| yes | external defined name XLSX | `../core/sc/qa/unit/subsequent_export_test5.cxx::testExternalDefinedNameXLSX` | LO XLSX fixture | External defined-name cache/import state; covered in Rust. |
| yes | formula persistence regression | `../core/sc/qa/unit/subsequent_export_test5.cxx::testTdf163554` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tdf163554.xlsx` | Covered in `xlsx_import.rs`; active failure exposes missing/error formula model exposure for LO-normalized 3D sheet range. |
| yes | empty functions | `../core/sc/qa/unit/subsequent_export_test5.cxx::testTdf170565_empty_functions` | LO ODS fixture | Empty function call preservation. |
| yes | external refs in data validation | `../core/sc/qa/unit/subsequent_export_test5.cxx::testErrorExternalsInDataValidation` | LO fixture | External formulas in validation. |
| yes | missing-path external | `../core/sc/qa/unit/subsequent_export_test5.cxx::testMissingPathExternal` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/MissingPathExternal.xlsx` | Missing external path behavior. |
| yes | startup external refs XLS | `../core/sc/qa/unit/subsequent_export_test6.cxx::testXlStartupExternalXLS` | LO XLS fixture | Startup external reference behavior. |
| yes | startup external refs XLSX | `../core/sc/qa/unit/subsequent_export_test6.cxx::testXlStartupExternalXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/XlStartupExternal.xlsx` | Startup external reference behavior. |
| no | shape macro external ref | `../core/sc/qa/unit/subsequent_export_test6.cxx::testShapeMacroExtRef` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/shape-macro-ext-ref.xlsx` | Drawing `macro` XML and exported external-link XML are raw package/export assertions, not formula-model assertions. |
| no | cell cursor get/set array formula | `../core/sc/qa/extras/sccellcursorobj.cxx::testGetSetArrayFormula` | LO UNO fixture | UNO editing/query API, not package/formula model test-suite coverage yet. |
| no | cell cursor get/set formula array | `../core/sc/qa/extras/sccellcursorobj.cxx::testGetSetFormulaArray` | LO UNO fixture | UNO editing/query API, not package/formula model test-suite coverage yet. |
| no | cell cursor query dependents | `../core/sc/qa/extras/sccellcursorobj.cxx::testQueryDependents` | LO UNO fixture | UNO query API, not package/formula model test-suite coverage yet. |
| no | cell cursor query precedents | `../core/sc/qa/extras/sccellcursorobj.cxx::testQueryPrecedents` | LO UNO fixture | UNO query API, not package/formula model test-suite coverage yet. |
| no | pasted formula UI | `../core/sc/qa/uitest/calc_tests8/tdf119343_calculate_pasted_formula.py` | `../core/sc/qa/uitest/data/tdf119343.ods` | UI paste/recalculate workflow. |
| no | formula dialog UI | `../core/sc/qa/uitest/calc_tests8/tdf163275_checking_formula_dialog.py` | UI dialog | UI-only. |
| no | inline array UI entry | `../core/sc/qa/uitest/calc_tests9/tdf117879_inline_array_in_formula.py` | UI dialog/input | UI-only. |
| no | hyperlink into formula cell UI | `../core/sc/qa/uitest/calc_tests9/tdf148437_insert_hyperlink_to_cell_with_formula.py` | UI dialog/input | UI-only. |
| no | CSV evaluate formulas UI | `../core/sc/qa/uitest/csv_dialog/tdf114878_evaluate_formulas_option_csv.py` | CSV dialog | UI import option, not OOXML formula coverage. |
| no | OOXML MathML import characters | `../core/oox/qa/unit/mathml.cxx::testImportCharacters` | `../core/oox/qa/unit/data/tdf144742_mathEqual_mathNotEqual.pptx` | Office Math/MathML object import belongs to layout/math rendering, not `ooxmlsdk-formula`. |
| no | OOXML MathML MCE import | `../core/oox/qa/unit/mathml.cxx::testImportMce` | `../core/oox/qa/unit/data/tdf144742_funnel.pptx` | Office Math/MathML object import belongs to layout/math rendering, not `ooxmlsdk-formula`. |
| no | MathML import to StarMath | `../core/starmath/qa/extras/mmlimport-test.cxx` | `../core/starmath/qa/extras/data/*.mml`, `tdf151842.odf` | StarMath equation model, not SpreadsheetML formulas. |
| no | MathML export from StarMath | `../core/starmath/qa/extras/mmlexport-test.cxx` | generated / MML fixtures | StarMath equation model, not SpreadsheetML formulas. |
| no | StarMath parse tests | `../core/starmath/qa/cppunit/test_parse.cxx` | synthetic | Equation parser, not Calc formula parser. |
| no | StarMath node tests | `../core/starmath/qa/cppunit/test_node.cxx` | synthetic | Equation node behavior, not Calc formula parser. |
| no | StarMath node-to-text tests | `../core/starmath/qa/cppunit/test_nodetotextvisitors.cxx` | synthetic | Equation serializer, not Calc formula parser. |
| no | StarMath cursor/editor tests | `../core/starmath/qa/cppunit/test_cursor.cxx` | synthetic | Equation editor workflow. |
| no | StarMath font style import | `../core/starmath/qa/cppunit/test_import.cxx::testFontStyles` | `../core/starmath/qa/cppunit/data/font-styles.odf` | Equation font behavior, not formula engine. |
| no | StarMath temp device font restore | `../core/starmath/qa/cppunit/test_starmath.cxx::testSmTmpDeviceRestoreFont` | synthetic | Equation device/font behavior, not formula engine. |

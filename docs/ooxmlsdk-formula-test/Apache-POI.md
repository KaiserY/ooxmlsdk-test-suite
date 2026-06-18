# Apache POI Formula Test Migration Index

This document is the migration checklist for Apache POI spreadsheet formula
coverage. It is intentionally scoped to formula semantics: formula text,
formula parsing, function evaluation, cached formula results, shared/array
formula metadata, names, external references, structured references, and formula
reference ranges.

Do not migrate POI assertions that only check POI Java APIs, raw XML shape,
charts, drawings, styles, comments, or package mechanics unless the assertion is
observable through `ooxmlsdk-formula` formula text/value/metadata.

## Fixture Layout

| Location | Purpose |
| --- | --- |
| `corpus/Apache-POI/` | Copied Apache POI source-side fixture corpus. Spreadsheet files live under `corpus/Apache-POI/test-data/spreadsheet/`. Prefer these for `.xlsx`/`.xlsm` formula import and cached-result tests. |
| `fixtures/Apache-POI/` | Future extracted minimal XML/formula fixtures created for this test-suite when POI only has synthetic Java construction. Keep these formula-focused; do not add raw XML import assertions here. |
| `../poi/` | Local upstream source checkout. Use it as the source of truth for test intent and expected values. |

Current corpus inventory relevant to formula migration:

| Corpus | Count | Notes |
| --- | ---: | --- |
| `corpus/Apache-POI/test-data/spreadsheet/*.{xlsx,xlsm,xls}` | 363 | Use `.xlsx`/`.xlsm` directly. Treat `.xls` as source evidence unless this test-suite gains a BIFF reader. |
| POI `ss/formula/functions/Test*.java` files | 143 | Mostly function evaluator unit tests; migrate direct formula semantics into `tests/evaluation.rs` or a POI-specific evaluator test. |
| POI `ss/formula/atp/Test*.java` files | 19 | Analysis ToolPak and modern function tests, including `XLOOKUP` and `XMATCH`. |
| POI `ss/formula/eval/Test*.java` files | 15 | Low-level evaluator behavior; migrate only behavior visible through public formula evaluation. |

Current migration status:

| Area | Rust test | Source | Status |
| --- | --- | --- | --- |
| XLOOKUP/XMATCH synthetic evaluator cases | `tests/evaluation.rs::evaluates_apache_poi_xlookup_cases`, `evaluates_apache_poi_xmatch_cases` | POI `TestXLookupFunction`, `TestXMatchFunction` | Migrated active. XMATCH currently passes; XLOOKUP multi-cell return currently fails because the evaluator returns a reference instead of a matrix value. |
| Workbook evaluator synthetic cases | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases`, `evaluates_apache_poi_range_and_coercion_cases` | POI `TestWorkbookEvaluator`, `BaseTestFormulaEvaluator`, `TestRangeEval`, `TestOperandResolver` | Migrated active. Current failures expose POI/Excel string-number comparison and date/time string coercion gaps. |
| ATP/logical/text/math direct function cases | `tests/evaluation.rs::evaluates_apache_poi_atp_logical_function_cases`, `evaluates_apache_poi_atp_date_and_statistical_cases`, `evaluates_apache_poi_value_and_numbervalue_cases`, `evaluates_apache_poi_textjoin_cases`, `evaluates_apache_poi_mround_and_error_predicate_cases` | POI `TestIfError`, `TestIfna`, `TestIfs`, `TestSwitch`, `TestNetworkdaysFunction`, `TestWorkdayFunction`, `TestWorkdayIntlFunction`, `TestPercentile`, `TestPercentRankExcFunction`, `TestPercentRankIncFunction`, `TestRandBetween`, `TestYearFracCalculator`, `TestValue`, `TestNumberValue`, `TestTextJoinFunction`, `TestMRound`, `TestLogicalFunction` | Migrated active. Current failures expose unimplemented/partial `TEXTJOIN`, date/number text coercion, `IFNA` arity error mapping, and `MROUND` edge behavior. |
| Error/boolean direct function cases | `tests/evaluation.rs::evaluates_apache_poi_error_and_boolean_cases` | POI `TestErrors`, `TestIsBlank`, `TestOrFunction` | Migrated active. Current failures expose missing text-arithmetic error propagation and OR/INDEX array coercion gaps. |
| Conditional aggregate and count direct function cases | `tests/evaluation.rs::evaluates_apache_poi_conditional_aggregate_cases`, `evaluates_apache_poi_count_function_cases` | POI `TestSumif`, `TestSumifs`, `TestAverageIf`, `TestAverageifs`, `TestMaxifs`, `TestMinifs`, `TestCountFuncs` | Migrated active. Current failures expose `SUMIF` error propagation and `COUNTBLANK` empty-string semantics differences. |
| Subtotal direct function cases | `tests/evaluation.rs::evaluates_apache_poi_subtotal_cases` | POI `TestSubtotal` | Migrated active. Nested formula-cell suppression is represented only where public formula text can express the same result. |
| Statistical direct function cases | `tests/evaluation.rs::evaluates_apache_poi_statistical_function_cases` | POI `TestAverage`, `TestAverageA`, `TestStdev`, `TestVar`, `TestForecast`, `TestCorrel`, `TestCovar`, `TestGeomean`, `TestSlope`, `TestIntercept`, `TestNormDist`, `TestNormInv`, `TestNormSDist`, `TestNormSInv`, `TestPoisson`, `TestPoissonDist` | Migrated active. Current failures identify unsupported statistical functions and coercion differences. |
| Financial direct function cases | `tests/evaluation.rs::evaluates_apache_poi_financial_function_cases` | POI `TestNpv`, `TestPmt`, `TestRate`, `TestIrr`, `TestMirr`, `TestNper`, `TestFormulaBugs::test55032` | Migrated active. Spreadsheet-backed `.xls` finance fixtures remain source evidence until BIFF support exists. |
| Math/aggregate direct function cases | `tests/evaluation.rs::evaluates_apache_poi_math_and_aggregate_cases`, `evaluates_apache_poi_rounding_math_cases`, `evaluates_apache_poi_engineering_function_cases`, `evaluates_apache_poi_formula_bug_regression_cases` | POI `TestRoundFuncs`, `TestQuotient`, `TestProduct`, `TestSum`, `TestAbs`, `TestTrunc`, `TestFloor`, `TestCeiling`, `TestFloorPrecise`, `TestCeilingPrecise`, `TestFloorMath`, `TestCeilingMath`, `TestBin2Dec`, `TestDec2Bin`, `TestHex2Dec`, `TestOct2Dec`, `TestComplex`, `TestDelta`, `TestSqrtpi`, `TestBesselJ`, `TestSumproduct`, `TestFormulaBugs` | Migrated active. Current failures expose missing string-argument error propagation, aggregate handling, engineering function, and array-evaluation gaps. |
| Date/time direct function cases | `tests/evaluation.rs::evaluates_apache_poi_date_and_time_function_cases` | POI `TestDate`, `TestDateValue`, `TestDays`, `TestTime`, `TestTimeValue`, `TestWeekdayFunc`, `TestEDate`, `TestEOMonth` | Migrated active. Current failures expose Excel 1900 leap-year compatibility and date/time parsing gaps. Dynamic current-year locale cases were intentionally not migrated. |
| Text direct function cases | `tests/evaluation.rs::evaluates_apache_poi_text_function_cases` | POI `TestClean`, `TestCode`, `TestLen`, `TestLeftRight`, `TestMid`, `TestSubstitute`, `TestTrim`, `TestFind`, `TestConcat`, `TestText`, `TestProperXSSF` | Migrated active. Current failures expose POI/Excel text-cleaning, `PROPER`, and formatting differences; locale-sensitive POI assertions were reduced to stable formula outputs. |
| Lookup/reference direct function cases | `tests/evaluation.rs::evaluates_apache_poi_lookup_reference_function_cases` | POI `TestAddress`, `TestAreas`, `TestIndex`, `TestOffset`, `TestRowCol`, `TestMatch`, `TestIndirect` | Migrated active. Current failures expose array/scalar coercion differences for `INDEX`, `AREAS`, `INDIRECT`, and related reference-return behavior. |
| XLOOKUP OOXML fixture | `tests/xlsx_import.rs::imports_apache_poi_xlookup_fixture_cached_and_recalculated_values` | POI `TestXSSFXLookupFunction::testXLookupFile` | Migrated active and currently passing. |
| XSSF shared/high-column formula fixtures | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_shared_formula_fixtures` | POI `TestXSSFFormulaEvaluation::testSharedFormulas_evaluateInCell`, `testEvaluateColumnGreaterThan255` | Migrated active. Cached formula text/value assertions are covered; recalculated assertions currently fail because `evaluated_value` is not populated. |
| XSSF multi-sheet reference fixture | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_multisheet_reference_fixture` | POI `TestXSSFFormulaEvaluation::testMultiSheetReferencesHSSFandXSSF`, `testMultiSheetAreasHSSFandXSSF` | Migrated active. Current recalculation failure exposes unsupported/incorrect 3D sheet-range evaluation. |
| XSSF formula regression fixtures | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures`, `imports_apache_poi_external_reference_formula_fixture`, `evaluates_apache_poi_structured_reference_formula_fixture` | POI `TestXSSFFormulaEvaluation::test59736`, `testBug61468`, `testBug61495`, `testBug62834`, `testBug63934`, `testBug60848_sumproductWithUnaryMinusArray`, `testReferencesToOtherWorkbooks`, `verifyAllFormulasInWorkbookCanBeEvaluated` | Migrated active. Current failures expose the same evaluated-value population gap, external workbook linking gaps, and structured-reference evaluator gaps. |
| Formula workbook corpus | `tests/xlsx_import.rs::evaluates_apache_poi_formula_eval_test_data_copy_fixture` | POI `TestFormulaEvaluatorOnXSSF` | Migrated active. Currently fails at first formula because `evaluate_supported_formulas()` does not populate `evaluated_value` for the fixture formula. |
| Matrix formula workbook | `tests/xlsx_import.rs::evaluates_apache_poi_matrix_formula_eval_fixture` | POI `TestMatrixFormulasFromXMLSpreadsheet` | Migrated active. Currently fails at first matrix formula for the same evaluated-value gap. |
| Multi-sheet range workbook | `tests/xlsx_import.rs::evaluates_apache_poi_formula_sheet_range_fixture` | POI `TestMultiSheetFormulaEvaluatorOnXSSF` | Migrated active. Currently fails at first 3D range formula for the same evaluated-value gap. |

## Test Matrix

| Migrate | Priority | Source | Fixture | Target | Formula coverage / decision |
| --- | --- | --- | --- | --- | --- |
| done | P0 | `poi-ooxml/src/test/java/org/apache/poi/xssf/TestXSSFXLookupFunction.java` | `corpus/Apache-POI/test-data/spreadsheet/xlookup.xlsx` plus synthetic Microsoft example | `tests/xlsx_import.rs`, `tests/evaluation.rs` | XLOOKUP cached values before/after evaluation; multi-column return result `Dianne Pugh` / `Finance`. |
| done | P0 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestXLookupFunction.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_xlookup_cases` | XLOOKUP exact match, not-found value, wildcard/match modes, binary and reverse binary search, multi-cell return arrays. |
| done | P0 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestXMatchFunction.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_xmatch_cases` | XMATCH exact/wildcard/search-mode behavior; case-insensitive examples. |
| done | P0 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestFormulaEvaluatorOnXSSF.java` | `corpus/Apache-POI/test-data/spreadsheet/FormulaEvalTestData_Copy.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_formula_eval_test_data_copy_fixture` | OOXML copy of POI's formula workbook; evaluate formula cells and compare expected cells by POI's row/column harness. |
| partial | P0 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulasFromSpreadsheet.java` | `FormulaEvalTestData.xls` | source evidence only until BIFF reader exists | Same workbook shape as the XSSF copy but `.xls`; use for expected-value evidence, not direct fixture execution yet. |
| done | P0 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestMatrixFormulasFromXMLSpreadsheet.java` | `corpus/Apache-POI/test-data/spreadsheet/MatrixFormulaEvalTestData.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_matrix_formula_eval_fixture` | Matrix/array formula evaluation against expected cells. |
| done | P0 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestMultiSheetFormulaEvaluatorOnXSSF.java` | `corpus/Apache-POI/test-data/spreadsheet/FormulaSheetRange.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_formula_sheet_range_fixture` | 3D sheet-range references and multi-sheet formula evaluation. |
| partial | P0 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestMultiSheetEval.java` | `FormulaSheetRange.xls` | source evidence only until BIFF reader exists | HSSF version of multi-sheet formula workbook. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testMultiSheetReferencesHSSFandXSSF` | `55906-MultiSheetRefs.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_multisheet_reference_fixture` | 3D references for `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `COUNTA`. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testMultiSheetAreasHSSFandXSSF` | `55906-MultiSheetRefs.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_multisheet_reference_fixture` | 3D area references for aggregate functions. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testBug60848_sumproductWithUnaryMinusArray` | `bug60848_sumproduct_unary_minus.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures` | `SUMPRODUCT(--(B5:B20))` over blanks should evaluate to `0`. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testBug61468` | `simple-monthly-budget.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures` | Cached/evaluated numeric formula result `3750`. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testBug61495` | `61495-test.xlsm` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures` | Formula evaluation with locale-like formatted text results `D 67.10` and `D 0,068`. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testBug62834` | `62834.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures` | `evaluateInCell` string results for formulas resolving to `"a value"` / `"another value"`. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testBug63934` | `63934.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures` | Formula result string `"Male"`. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::test59736` | `59736.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_xssf_formula_evaluation_regression_fixtures` | Re-evaluating a fixture formula preserves numeric result `1`. |
| partial | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testExternalReferences` | `ref2-56737.xlsx`, `56737.xlsx`, `56737.xls`, synthetic `alt.xlsx` | `tests/xlsx_import.rs::imports_apache_poi_external_reference_formula_fixture` | Formula text and cached/live expected values for `ref2-56737.xlsx` migrated. `.xls` peer and POI workbook-linking exception/API assertions remain source evidence until external workbook linking and BIFF support exist. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaEvaluation.java::testEvaluateAllSpreadsheetWithoutException` | `evaluate_formula_with_structured_table_references.xlsx` | `tests/xlsx_import.rs::evaluates_apache_poi_structured_reference_formula_fixture` | Structured-reference formula import/evaluation smoke. POI's performance-only fixture variant is not migrated. |
| done | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFEvaluationWorkbook.java::testRefToBlankCellInArrayFormula` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | Blank cell references coerce to zero where formula value semantics expose the behavior. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java::testRefToBlankCellInArrayFormula` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | Same behavior as XSSF variant; source evidence for workbook-neutral evaluator behavior. |
| blocked | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestFormulaEval.java` | synthetic | blocked | Circular reference dependency graph behavior needs formula-cell dependency evaluation rather than direct formula text evaluation. |
| blocked | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestFormulaEval.java` | synthetic | blocked | Circular reference behavior and deep dependency stack regression need formula-cell dependency evaluation rather than direct formula text evaluation. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java::testIFEqualsFormulaEvaluation_*` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | `IF(A1=B1,2,3)` coercion across numeric/string/boolean/formula/blank inputs. |
| blocked | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java::testAttrSum` | synthetic Ptg evidence | blocked | `AttrPtg.SUM` is POI token-internal behavior; normal formula `SUM` coverage is migrated elsewhere. |
| blocked | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java::testMemFunc` | synthetic Ptg evidence | blocked | Memory-function token behavior is POI token-internal unless a public token model is added. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java::testEvaluateFormulaWithRowBeyond32768_Bug44539` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | Large row index reference behavior is covered by direct public formula evaluation where expressible. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/TestWorkbookEvaluator.java::testMissingArg` and `testMissingArgWithAreaRef` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | Missing argument semantics where parser/evaluator expose blank/missing args. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/usermodel/BaseTestFormulaEvaluator.java::testFormulaEvaluatorEvaluateSimpleFormulaCell` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | Simple arithmetic formula preserves formula text and evaluates numeric result. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/usermodel/BaseTestFormulaEvaluator.java::testFormulaEvaluatorEvaluateVlookupFormulaCell` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | VLOOKUP over a synthetic table returns string `"2"`. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/usermodel/BaseTestFormulaEvaluator.java::testIntersectionInFunctionArgs_60980` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_workbook_evaluator_cases` | Significant-space intersection inside `SUM`, including named ranges. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestRangeEval.java::testRangeUsingOffsetFunc_bug46948` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_range_and_coercion_cases` | Dynamic `SUM(C1:OFFSET(...))` range endpoint behavior. |
| partial | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestRangeEval.java::testPermutations` | synthetic low-level areas | `tests/address.rs` or blocked | Port only if public range operator exposes equivalent first/last cell behavior. |
| blocked | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestMinusZeroResult.java` | synthetic low-level Eval operands | blocked | Raw `-0.0` operand identity and text rendering are low-level Eval behavior; public formula text cannot directly construct the same operand state. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestOperandResolver.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_range_and_coercion_cases` | Numeric/date/time string coercion into formula values. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java::test27349` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_formula_bug_regression_cases` | Quoted sheet references in `VLOOKUP` evaluate to `3`. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java::test27405` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_formula_bug_regression_cases` | `ISNUMBER` inside `IF` and boolean result. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java::test42448` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_formula_bug_regression_cases` | `SUMPRODUCT` across sheet-qualified ranges. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java::test55032` | fixture/synthetic | `tests/evaluation.rs::evaluates_apache_poi_financial_function_cases` | Formula behavior migrated; workbook API exception wording intentionally omitted. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java::testLookupFormula` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_formula_bug_regression_cases` | LOOKUP/VLOOKUP regression formulas and expected values. |
| done | P1 | `poi/src/test/java/org/apache/poi/ss/formula/eval/TestFormulaBugs.java::testFormula_58571` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_formula_bug_regression_cases` | `TEXT` formula quote escaping and duration formatting result `0h 15m`. |
| partial | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFFormulaParser.java` | synthetic plus `56737.xlsx` | parser/source evidence | Formula text behaviors covered by evaluator/import tests where visible. POI Ptg-count and token-class assertions are parser internals outside this formula-value crate. |
| partial | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFName.java` | synthetic | source evidence | Named range formula text/evaluation already has local coverage; POI Java API create/delete/duplicate-name assertions are outside formula-test scope. |
| partial | P1 | `poi/src/test/java/org/apache/poi/ss/usermodel/BaseTestNamedRange.java` | synthetic plus `.xls` fixtures | `tests/evaluation.rs`, `tests/xlsx_import.rs` | Named formulas/ranges, print-area formulas, unicode names. Exclude Java API duplicate-name and sheet-index exception assertions. |
| partial | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFTable.java` | `ExcelTables.xlsx`, `StructuredReferences.xlsx`, table fixtures | `tests/xlsx_import.rs::evaluates_apache_poi_structured_reference_formula_fixture` | Structured table formula text/evaluation smoke migrated. Table style/layout XML and Java table API checks are outside formula-test scope. |
| blocked | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFDataValidation.java::testTableBasedValidationList` | `dataValidationTableRange.xlsx` | blocked | Formula-derived validation list values need a public data-validation formula consumer model; not part of formula evaluator/import tests yet. |
| partial | P1 | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFConditionalFormatting.java` | conditional-formatting fixtures | future formula-consumer tests | Only formula strings/criteria are formula-related; skip formatting/style assertions. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/functions/TestAbs.java` through `TestXYNumericFunction.java`, plus `poi-ooxml/src/test/java/org/apache/poi/ss/tests/formula/functions/TestProperXSSF.java` | mostly synthetic | `tests/evaluation.rs` | Direct function evaluator assertions migrated as formula-value groups where POI expected values are stable and public formula text can express them, including `AREAS`, `INDIRECT`, and `PROPER`. Remaining `.xls` spreadsheet-backed function suites are source evidence until BIFF support exists. |
| partial | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestAnalysisToolPak.java` | synthetic | source evidence | Function availability dispatch internals are not exposed directly; specific ATP functions are migrated below. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestIfError.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_logical_function_cases` | `IFERROR` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestIfna.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_logical_function_cases` | `IFNA` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestIfs.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_logical_function_cases` | `IFS` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestMRound.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_mround_and_error_predicate_cases` | `MROUND` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestNetworkdaysFunction.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_date_and_statistical_cases` | `NETWORKDAYS` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestWorkdayFunction.java` and `TestWorkdayIntlFunction.java` | synthetic and `50755_workday_formula_example.xlsx` | `tests/evaluation.rs::evaluates_apache_poi_atp_date_and_statistical_cases` | `WORKDAY` / `WORKDAY.INTL` date arithmetic. Spreadsheet `.xlsx` example is source evidence if it contains only duplicate assertions. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestTextJoinFunction.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_textjoin_cases` | `TEXTJOIN` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestSwitch.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_logical_function_cases` | `SWITCH` behavior. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestPercentile.java`, `TestPercentRankExcFunction.java`, `TestPercentRankIncFunction.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_date_and_statistical_cases` | Percentile/percentrank variants. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestRandBetween.java` | synthetic | `tests/evaluation.rs::evaluates_apache_poi_atp_date_and_statistical_cases` | Volatile range/shape semantics only; nondeterministic exact values intentionally avoided. |
| done | P2 | `poi/src/test/java/org/apache/poi/ss/formula/atp/TestYearFracCalculator*.java` | synthetic/spreadsheet | `tests/evaluation.rs::evaluates_apache_poi_atp_date_and_statistical_cases` | `YEARFRAC` basis behavior. |
| partial | P2 | `poi/src/test/java/org/apache/poi/ss/formula/functions/*FromSpreadsheet.java` | `.xls` spreadsheets such as `FormulaEvalTestData.xls` and function-specific `.xls` files | source evidence until BIFF reader exists | Spreadsheet-backed function corpora are formula-relevant but not directly executable in this OOXML/FODS test-suite yet. Port individual formulas synthetically where high value. |
| no | blocked | `poi/src/test/java/org/apache/poi/ss/formula/TestFormulaShifter.java` | synthetic Ptg arrays | blocked | Structural row/column/sheet formula rewrite over `Ptg[]`. Needs public reference-update model before migration. |
| no | blocked | `poi/src/test/java/org/apache/poi/ss/formula/ptg/Test*.java` | synthetic Ptg binary tokens | blocked | POI token internals. Port only if `ooxmlsdk-formula` exposes equivalent token/stringify APIs. |
| no | blocked | `poi/src/test/java/org/apache/poi/ss/formula/constant/TestConstantValueParser.java` | synthetic binary records | blocked | BIFF token/value parser internals, not OOXML formula behavior. |
| partial | blocked | `poi/src/test/java/org/apache/poi/ss/formula/TestEvaluationCache.java`, `TestCellCacheEntry.java`, `TestPlainCellCache.java` | synthetic POI cache internals | blocked | POI evaluator cache lifecycle. Port only user-visible stale-cache formula behavior if a public cache model exists. |
| partial | blocked | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFSheetShiftRows*.java`, `TestXSSFSheetShiftColumns.java`, helper/base shift tests | `.xlsx` and synthetic | blocked | Formula reference rewrite after row/column edits. Formula-relevant, but current test-suite is import/evaluation focused. |
| partial | blocked | `poi/src/test/java/org/apache/poi/ss/usermodel/BaseTestSheetUpdateArrayFormulas.java`, `poi-ooxml/.../TestXSSFSheetUpdateArrayFormulas.java` | synthetic and XML checks | blocked/import subset | Array formula metadata is formula-relevant; edit-time protected-cell exceptions and raw `CTCellFormula` assertions are not. |
| partial | blocked | `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFForkedEvaluator.java` and `poi/src/test/java/org/apache/poi/ss/formula/eval/forked/BaseTestForkedEvaluator.java` | synthetic | blocked | Forked evaluator overlay semantics; needs public overlay evaluation model. |
| partial | blocked | `poi/src/test/java/org/apache/poi/ss/formula/BaseTestExternalFunctions.java`, `TestFunctionRegistry.java`, `udf/TestUDFFinder.java`, `eval/TestExternalFunction.java`, `ptg/TestExternalFunctionFormulas.java` | synthetic | blocked | UDF/external-function registry behavior. Port only registered-function dispatch if the Rust evaluator exposes UDF hooks. |
| no | out | chart title formula tests such as `poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/charts/TestXSSFChartTitle.java` | chart fixtures | out of scope | Chart object formula storage, not spreadsheet formula evaluation. |
| no | out | extractor/eventusermodel tests with `setFormulasNotResults` | spreadsheet fixtures | out of scope | Text extraction mode, not formula evaluation semantics. |
| no | out | OpenXML4J/package/security/POIFS tests | package fixtures | out of scope | Package behavior is not formula migration. |

## Function Test Inventory

The `ss/formula/functions` and `ss/formula/atp` directories are formula-first
and should be mined after P0/P1 fixture-backed tests. Directly migrate tests
that can be expressed as formula strings against `FormulaEvaluationBook`.

High-value first groups:

| Group | POI sources | Notes |
| --- | --- | --- |
| Lookup/reference | `TestAddress`, `TestAreas`, `TestIndex`, `TestIndexFunctionFromSpreadsheet`, `TestIndirect`, `TestIndirectFunctionFromSpreadsheet`, `TestLookupFunctionsFromSpreadsheet`, `TestMatch`, `TestMatchFunctionsFromSpreadsheet`, `TestOffset`, `TestRowCol`, `TestSheet`, ATP `TestXLookupFunction`, ATP `TestXMatchFunction` | Largest overlap with current XLOOKUP/XMATCH work. |
| Conditional aggregates | `TestAverageIf`, `TestAverageifs`, `TestCountFuncs`, `TestMaxifs`, `TestMinifs`, `TestSumif`, `TestSumifs`, `TestSubtotal`, `TestD*` database tests | Useful for criterion parsing and blank/error semantics. |
| Array/matrix/math | `TestFrequency`, `TestSumproduct`, `TestXYNumericFunction`, `TestTrendFunctionsFromSpreadsheet`, `TestMathX`, `TestProduct`, `TestSum`, `TestRoundFuncs`, `TestFloor*`, `TestCeiling*` | Prefer synthetic formulas; spreadsheet-backed `.xls` rows are evidence until BIFF support. |
| Text/coercion | `TestClean`, `TestCode`, `TestConcat`, `TestFind`, `TestFixed`, `TestLen`, `TestLeftRight`, `TestMid`, `TestNumberValue`, `TestSubstitute`, `TestText`, `TestTrim`, `TestValue`, ATP `TestTextJoinFunction` | Good follow-up after LO text alignment. |
| Date/time/financial | `TestDate`, `TestDateValue`, `TestDays`, `TestDays360`, `TestEDate`, `TestEOMonth`, `TestTime`, `TestTimeValue`, `TestWeekNum*`, `TestWeekdayFunc`, `TestWorkdayFunc`, `TestFinanceLib`, `TestIrr`, `TestMirr`, `TestNpv`, `TestPmt`, `TestRate`, ATP `TestNetworkdaysFunction`, ATP `TestWorkday*`, ATP `TestYearFracCalculator*` | Keep date-system assumptions explicit. |
| Statistical | `TestAverage`, `TestAverageA`, `TestCorrel`, `TestCovar`, `TestForecast`, `TestGeomean`, `TestIntercept`, `TestNorm*`, `TestPercentRank`, `TestPoisson*`, `TestRank`, `TestSlope`, `TestStandardize`, `TestStatsLib`, `TestStdev`, `TestTDist*`, `TestVar`, ATP percentile/percentrank tests | Compare with LO where POI and Calc differ. |
| Add-in/engineering | `TestBesselJ`, `TestBin2Dec`, `TestComplex*`, `TestDec2Bin`, `TestDec2Hex`, `TestDelta*`, `TestFactDoubleFunctionsFromSpreadsheet`, `TestHex2Dec`, `TestImRealFunctionsFromSpreadsheet`, `TestImaginaryFunctionsFromSpreadsheet`, `TestOct2Dec`, `TestQuotient*`, `TestRomanFunctionsFromSpreadsheet`, `TestSqrtpi` | Often already covered by LO FODS; use POI to fill missing Excel-compatible edges. |
| Logical/error | `TestBooleanFunctionsFromSpreadsheet`, `TestErrors`, `TestIFFunctionFromSpreadsheet`, `TestIfnaFromSpreadsheet`, `TestIsBlank`, `TestLogicalFunction`, `TestLogicalFunctionsFromSpreadsheet`, `TestOrFunction`, `TestRelationalOperations`, ATP `TestIfError`, ATP `TestIfna`, ATP `TestIfs`, ATP `TestSwitch` | Port as synthetic evaluator assertions. |

## Migration Notes

1. Prefer `.xlsx` fixture tests from `corpus/Apache-POI/test-data/spreadsheet`
   when POI already has an OOXML fixture. Do not copy the same file into
   `fixtures/`.
2. Use `fixtures/Apache-POI/` only for small extracted XML/formula fixtures that
   are needed because the POI source test constructs the workbook entirely in
   Java.
3. Keep `.xls` POI tests in this document as source evidence, but mark them
   blocked unless the test can be translated into synthetic formula-evaluator
   assertions.
4. Avoid raw XML assertions. For array/shared formulas, assert formula text,
   shared group metadata, cached values, array ranges, or evaluated results.
5. When POI and LibreOffice disagree, document the disagreement in the Rust test
   source comment and prefer Excel/POI behavior only for POI-specific tests.

## 2026-06-18 Formula Dispatch Gap Check

Cross-checking current `ooxmlsdk-formula` ordinary dispatch ids against direct
`evaluation.rs` assertions, the LibreOffice FODS corpus, and POI formula tests
shows two any-path formula-test gaps. POI does not currently fill either ETS
gap:

| Priority | Function id | Public names | POI status | Follow-up |
| --- | --- | --- | --- | --- |
| P0 | `ForecastDotEtsDotSeasonality` | `FORECAST.ETS.SEASONALITY` | No matching POI formula test or fixture found in the local `../poi` checkout. | Add Excel/LibreOffice-backed focused formula assertions or fixture coverage. |
| P0 | `ForecastDotEtsDotStat` | `FORECAST.ETS.STAT`, `FORECAST.ETS.STAT.ADD` | No matching POI formula test or fixture found in the local `../poi` checkout. | Add Excel/LibreOffice-backed focused formula assertions or fixture coverage. |

This does not mean `evaluation.rs` is complete. Direct evaluator coverage is
still incomplete and should be expanded from POI/LO/Excel evidence even when a
function is already exercised indirectly by the FODS corpus.

# LibreOffice Formula Test Migration Index

This document is a migration index for LibreOffice Calc formula coverage.
Use it as the checklist for translating tests into `ooxmlsdk-formula` and this
test-suite.

## Migration Status

The LibreOffice `sc/qa/unit/data/functions/**/fods/*.fods` function corpus has
been copied into `fixtures/LibreOffice/sc/qa/unit/data/functions/` and is covered
by `crates/ooxmlsdk-formula-test/tests/fods_corpus.rs`.

Current baseline:

| Corpus | Fixtures | Formula cells | Passed | Mismatched | Unsupported |
| --- | ---: | ---: | ---: | ---: | ---: |
| LibreOffice functions FODS | 507 | 52,191 | 17,095 | 4,133 | 30,963 |

The corpus runner compares `ooxmlsdk-formula` evaluation results with the cached
formula values stored in the upstream FODS files. Failures are intentionally not
fixed in this migration pass; they are the follow-up bug backlog for
`ooxmlsdk-formula`.

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
| yes | SUBTOTAL | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUBTOTAL` | synthetic | `SUBTOTAL` evaluation and hidden-row behavior. |
| yes | SUBTOTAL reference immutability | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUBTOTALReferenceNotMutated` | synthetic | `SUBTOTAL` must not mutate reference inputs. |
| yes | SUMXMY2 | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncSUMXMY2` | synthetic | `SUMXMY2` evaluation. |
| yes | MIN | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncMIN` | synthetic | `MIN` evaluation. |
| yes | N | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncN` | synthetic | `N` evaluation. |
| yes | COUNTIF | `../core/sc/qa/unit/ucalc_formula.cxx::testFuncCOUNTIF` | synthetic | `COUNTIF` evaluation. |
| yes | row/column labels | `../core/sc/qa/unit/ucalc_formula.cxx::testInsertRowColLabel` | synthetic | Label behavior used by formula references. |
| yes | IF | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncIF` | synthetic | `IF` evaluation. |
| yes | CHOOSE | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCHOOSE` | synthetic | `CHOOSE` evaluation. |
| yes | IFERROR | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncIFERROR` | synthetic | `IFERROR` evaluation. |
| yes | SHEET | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSHEET` | synthetic | `SHEET` evaluation. |
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
| yes | range operator | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncRangeOp` | synthetic | Range operator behavior. |
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
| yes | matrix concatenation | `../core/sc/qa/unit/ucalc_formula2.cxx::testMatConcat` | synthetic | Matrix concatenation. |
| yes | matrix concatenation replication | `../core/sc/qa/unit/ucalc_formula2.cxx::testMatConcatReplication` | synthetic | Matrix concatenation replication. |
| yes | R1C1 whole-column ref | `../core/sc/qa/unit/ucalc_formula2.cxx::testRefR1C1WholeCol` | synthetic | Whole-column R1C1 parsing. |
| yes | R1C1 whole-row ref | `../core/sc/qa/unit/ucalc_formula2.cxx::testRefR1C1WholeRow` | synthetic | Whole-row R1C1 parsing. |
| yes | copied column label | `../core/sc/qa/unit/ucalc_formula2.cxx::testSingleCellCopyColumnLabel` | synthetic | Column-label formula behavior. |
| yes | Excel intersection | `../core/sc/qa/unit/ucalc_formula2.cxx::testIntersectionOpExcel` | synthetic | Excel intersection operator. |
| yes | hidden rows | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncRowsHidden` | synthetic | Function behavior with hidden rows. |
| yes | SUMIFS | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncSUMIFS` | synthetic | Conditional aggregate evaluation. |
| yes | COUNTIF empty | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCOUNTIFEmpty` | synthetic | Empty-cell conditional count behavior. |
| yes | COUNTIFS range reduce | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncCOUNTIFSRangeReduce` | synthetic | Conditional count range reduction. |
| yes | reference list array SUBTOTAL | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncRefListArraySUBTOTAL` | synthetic | Ref-list array behavior. |
| yes | jump matrix array IF | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncJumpMatrixArrayIF` | synthetic | Matrix array with `IF`. |
| yes | jump matrix array OFFSET | `../core/sc/qa/unit/ucalc_formula2.cxx::testFuncJumpMatrixArrayOFFSET` | synthetic | Matrix array with `OFFSET`. |
| yes | iterative calculation | `../core/sc/qa/unit/ucalc_formula2.cxx::testIterations` | synthetic | Iteration/recalculation behavior. |
| yes | delete-row aftermath | `../core/sc/qa/unit/ucalc_formula2.cxx::testFormulaAfterDeleteRows` | synthetic | Formula state after row deletion. |
| yes | XLOOKUP regex | `../core/sc/qa/unit/ucalc_formula2.cxx::testRegexForXLOOKUP` | synthetic | `XLOOKUP` regex semantics. |
| yes | horizontal query empty cell | `../core/sc/qa/unit/ucalc_formula2.cxx::testHoriQueryEmptyCell` | synthetic | Horizontal query behavior on empty cells. |
| yes | vertical query empty cell | `../core/sc/qa/unit/ucalc_formula2.cxx::testVertQueryEmptyCell` | synthetic | Vertical query behavior on empty cells. |
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
| yes | external reference cache XLSX | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testExternalRefCacheXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/external-refs.xlsx` | External reference cache import. |
| yes | external reference cache ODS | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testExternalRefCacheODS` | LO ODS fixture | External reference cache import. |
| yes | VBA/UDF formulas | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testVBAUserFunctionXLSM` | LO XLSM fixture | VBA user-function formula text. |
| yes | unresolved external references | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testErrorOnExternalReferences` | LO fixture | Error handling for unresolved externals. |
| yes | tdf160371 formula string | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf160371` | LO fixture | Imported formula string/reference semantics. |
| yes | tdf136364 formula string | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf136364` | LO fixture | Imported formula string/reference semantics. |
| yes | tdf131536 formula string | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testTdf131536` | LO fixture | Imported formula string/reference semantics. |
| no | Excel XML named expressions global | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testNamedExpressionsXLSXML_Global` | LO XML fixture | Not OOXML package coverage for this test-suite migration. |
| no | Excel XML named expressions local | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testNamedExpressionsXLSXML_Local` | LO XML fixture | Not OOXML package coverage for this test-suite migration. |
| no | Excel XML empty rows | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testEmptyRowsXLSXML` | LO XML fixture | Not OOXML package coverage for this test-suite migration. |
| yes | named table references | `../core/sc/qa/unit/subsequent_filters_test2.cxx::testNamedTableRef` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/tablerefsnamed.xlsx` | Structured reference import. |
| yes | conditional-format formula listener | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testCondFormatFormulaListenerXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/cond_format_formula_listener.xlsx` | Formula dependency/listener behavior if exposed by formula model. |
| yes | formula text regression | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testTdf112780` | LO fixture | Formula import regression. |
| yes | VBA macro function import | `../core/sc/qa/unit/subsequent_filters_test3.cxx::testVBAMacroFunctionODS` | LO ODS fixture | Macro function formula preservation. |
| yes | LOOKUP external ref | `../core/sc/qa/unit/subsequent_filters_test5.cxx::testTdf167134_LOOKUP_extRef` | LO FODS fixtures | External reference lookup semantics. |
| yes | named range formula | `../core/sc/qa/unit/subsequent_filters_test5.cxx::testTdf94627` | LO XLSB fixture | Named range formula preservation. |
| yes | full-column refs | `../core/sc/qa/unit/subsequent_filters_test5.cxx::testFullColumnRefs` | LO fixture | Full-column formula references. |
| no | formula XML export node | `../core/sc/qa/unit/subsequent_export_test.cxx::testTdf90104` | LO XLSX fixture | Pure XML export shape; `ooxmlsdk` package/schema tests already cover raw XML. |
| yes | EASTERSUNDAY export semantics | `../core/sc/qa/unit/subsequent_export_test.cxx::testTdf162177_EastersundayODF14` | LO FODS fixture | Function name/namespace semantics if evaluator/export supports it. |
| yes | named range export regression | `../core/sc/qa/unit/subsequent_export_test.cxx::testNamedRangeBugfdo62729` | LO ODS fixture | Named range formula export semantics. |
| yes | built-in ranges | `../core/sc/qa/unit/subsequent_export_test.cxx::testBuiltinRangesXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/built-in_ranges.xlsx` | Built-in named ranges. |
| yes | quoted sheet name | `../core/sc/qa/unit/subsequent_export_test.cxx::testFormulaRefSheetNameODS` | LO ODS fixture | Formula sheet-name quoting. |
| yes | generated formula values | `../core/sc/qa/unit/subsequent_export_test.cxx::testCellValuesExportODS` | generated | Formula string/value round-trip behavior. |
| yes | inline array XLS | `../core/sc/qa/unit/subsequent_export_test.cxx::testInlineArrayXLS` | LO XLS fixture | Inline array formula import/export semantics. |
| yes | formula references XLS | `../core/sc/qa/unit/subsequent_export_test.cxx::testFormulaReferenceXLS` | LO XLS fixture | Absolute/relative/3D reference formulas. |
| yes | matrix multiplication XLSX | `../core/sc/qa/unit/subsequent_export_test2.cxx::testMatrixMultiplicationXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/matrix-multiplication.xlsx` | Matrix multiplication formula result/import. |
| yes | structured reference export tdf105272 | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf105272` | LO XLSX fixture | Structured reference formula preservation. |
| yes | structured reference export tdf118990 | `../core/sc/qa/unit/subsequent_export_test2.cxx::testTdf118990` | LO XLSX fixture | Structured reference formula preservation. |
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
| yes | CEILING/FLOOR XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/ceiling-floor.xlsx` | `CEILING`/`FLOOR` compatibility semantics. |
| yes | CEILING/FLOOR XLS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorXLS` | LO XLS fixture | `CEILING`/`FLOOR` compatibility semantics. |
| yes | CEILING/FLOOR ODS | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorODS` | LO ODS fixture | `CEILING`/`FLOOR` compatibility semantics. |
| yes | CEILING/FLOOR ODS to XLSX | `../core/sc/qa/unit/subsequent_export_test3.cxx::testCeilingFloorODSToXLSX` | LO ODS fixture | `CEILING`/`FLOOR` compatibility semantics. |
| yes | external virtual path | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSupBookVirtualPathXLS` | LO XLS fixture | External workbook path/formula preservation. |
| yes | sheet-local range name | `../core/sc/qa/unit/subsequent_export_test3.cxx::testSheetLocalRangeNameXLS` | LO XLS fixture | Sheet-local named formulas. |
| yes | relative named expressions | `../core/sc/qa/unit/subsequent_export_test3.cxx::testRelativeNamedExpressionsXLS` | LO ODS fixture | Relative named expressions. |
| yes | formula persistence regression | `../core/sc/qa/unit/subsequent_export_test5.cxx::testTdf163554` | LO fixture | Formula persistence; port expected LO formula string/value. |
| yes | empty functions | `../core/sc/qa/unit/subsequent_export_test5.cxx::testTdf170565_empty_functions` | LO ODS fixture | Empty function call preservation. |
| yes | external refs in data validation | `../core/sc/qa/unit/subsequent_export_test5.cxx::testErrorExternalsInDataValidation` | LO fixture | External formulas in validation. |
| yes | missing-path external | `../core/sc/qa/unit/subsequent_export_test5.cxx::testMissingPathExternal` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/MissingPathExternal.xlsx` | Missing external path behavior. |
| yes | startup external refs XLS | `../core/sc/qa/unit/subsequent_export_test6.cxx::testXlStartupExternalXLS` | LO XLS fixture | Startup external reference behavior. |
| yes | startup external refs XLSX | `../core/sc/qa/unit/subsequent_export_test6.cxx::testXlStartupExternalXLSX` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/XlStartupExternal.xlsx` | Startup external reference behavior. |
| yes | shape macro external ref | `../core/sc/qa/unit/subsequent_export_test6.cxx::testShapeMacroExtRef` | `corpus/LibreOffice/sc/qa/unit/data/xlsx/shape-macro-ext-ref.xlsx` | Formula-like macro external reference preservation if exposed through formula metadata. |
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

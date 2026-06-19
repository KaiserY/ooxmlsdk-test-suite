# Gnumeric Formula Test Migration Index

This document tracks formula-semantics coverage migrated from Gnumeric into the
external `ooxmlsdk-test-suite`.

License boundary: Gnumeric is available under GPL version 2 or GPL version 3.
Keep Gnumeric-derived coverage out of the `ooxmlsdk` main repository. Do not
copy Gnumeric source files, Perl helpers, or `.gnumeric` fixtures into the main
repo. In this test-suite, migrate only formula behavior as Rust assertions and
annotate each group with `Source:` comments pointing back to the upstream test
and sample files.

## Fixture Layout

| Location | Purpose |
| --- | --- |
| `../gnumeric/test/` | Local upstream test scripts. Use these to identify expected pass/fail cells and test intent. |
| `../gnumeric/samples/` | Local upstream spreadsheet samples referenced by the scripts. Use these as source evidence for formulas and expected values. |
| `crates/ooxmlsdk-formula-test/tests/evaluation.rs` | Formula-value assertions migrated from Gnumeric. |
| `docs/ooxmlsdk-formula-test/Gnumeric.md` | Migration checklist and license notes. |

Do not add Gnumeric fixture files to `corpus/` unless a later pass explicitly
adds a GPL-compatible, test-suite-only fixture lane.

## P1 Bootstrap

| Status | Source | Sample | Rust target | Coverage |
| --- | --- | --- | --- | --- |
| migrated active | `test/t1110-xlookup.pl` | `samples/xlookup.gnumeric` | `tests/evaluation.rs::evaluates_gnumeric_xlookup_and_xmatch_cases` | `XLOOKUP`/`XMATCH` exact match, not-found value, wildcard match, forward/reverse search, and approximate numeric match. |
| migrated active | `test/t1109-unique.pl` | `samples/unique.gnumeric` | `tests/evaluation.rs::evaluates_gnumeric_unique_cases` | `UNIQUE` duplicate removal, scalar single-result behavior, text identity, empty-string distinction, and row-wise uniqueness. |
| migrated active | `test/t1903-intersection-tests.pl` | `samples/intersection-tests.gnumeric` | `tests/evaluation.rs::evaluates_gnumeric_intersection_cases` | Implicit intersection from the formula cell against vertical ranges, horizontal ranges, whole-row-style ranges, and whole-column-style ranges. |
| migrated active | `test/t1112-regextest.pl`, `test/t1113-regexextract.pl`, `test/t1114-regexreplace.pl` | `samples/regextest.gnumeric`, `samples/regexextract.gnumeric`, `samples/regexreplace.gnumeric` | `tests/evaluation.rs::evaluates_gnumeric_regex_function_cases` | Gnumeric `REGEXTEST`, `REGEXEXTRACT`, and `REGEXREPLACE` semantics, including case-insensitive matching, anchors, classes, captures, repeated extraction, and replacement backreferences. |

These tests are intentionally active and not ignored. Runtime failures should be
treated as formula coverage gaps for a later implementation pass.

## Deferred Candidates

| Source / sample | Reason to defer |
| --- | --- |
| `samples/formula-tests.gnumeric` | Broad parser/evaluator sweep; migrate after P1 failure surface is understood. |
| `samples/direct-string-args.gnumeric`, `samples/direct-bool-args.gnumeric`, `samples/indirect-string-args.gnumeric` | Coercion-focused function matrix; useful after current POI/LibreOffice coercion gaps are triaged. |
| `samples/excel12/countif.xlsx`, `samples/excel12/ifs-funcs.xlsx`, `samples/switch.gnumeric` | Overlaps existing POI/LibreOffice function assertions; migrate later if they expose distinct semantics. |
| Mixed `UNIQUE` cases with errors, blanks, booleans, and strings | Useful for full dynamic-array parity, but better added with a mixed-value matrix assertion helper in a focused pass. |

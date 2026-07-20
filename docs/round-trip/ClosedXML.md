# ClosedXML Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/ClosedXML` |
| Manifest | `corpus/ClosedXML/manifest.toml` |
| Source | `https://github.com/ClosedXML/ClosedXML` |
| License | MIT |
| License files | `licenses/ClosedXML/LICENSE` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 286 | 284 | 0 | 2 | 0 | 2026-07-20 | 286 | 0 |

The imported corpus contains 287 OOXML package paths. The generated lane
excludes the transient `~$LoadPivotTables.xlsx` lock file, leaving these 286
tests:

| Extension | Files |
| --- | ---: |
| `xlsx` | 285 |
| `xlsm` | 1 |

The two invalid expectations are intentional malformed-input fixtures: one has
a negative `activeTab`, and the other contains invalid boolean, numeric, and
enum attribute lexemes. ClosedXML's `TryToLoad/LO` subtree is not imported
because its distinct payloads are already present byte-for-byte in the
LibreOffice corpus, as detailed in `corpus/README.md`.

## Run Command

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test closedxml_roundtrip -- --ignored
```

## Last Run

```text
test result: ok. 286 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.86s
```

Current failures: none.

The lane uses the shared package comparison model: open, save, reopen, package
part graph comparison, ZIP entry comparison, canonical XML equivalence, and
schema-derived float lexical normalization.

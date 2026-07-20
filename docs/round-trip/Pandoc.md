# Pandoc Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/Pandoc` |
| Manifest | `corpus/Pandoc/manifest.toml` |
| Source | `https://github.com/jgm/pandoc` |
| License | GPL-2.0-or-later |
| License files | `licenses/Pandoc/COPYING.md`, `licenses/Pandoc/COPYRIGHT` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 235 | 234 | 1 | 0 | 0 | 2026-07-20 | 235 | 0 |

The imported corpus covers Pandoc's DOCX, PPTX, and XLSX reader/writer fixtures:

| Extension | Files |
| --- | ---: |
| `docx` | 131 |
| `pptx` | 103 |
| `xlsx` | 1 |

Six byte-identical PPTX paths are omitted from the import; the retained paths
are recorded in `corpus/README.md`. The `ns0-reference.docx` fixture is
open-only because it uses arbitrary namespace prefixes for WordprocessingML
attributes and a non-schema `sectPr` type attribute.

## Run Command

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test pandoc_roundtrip -- --ignored
```

## Last Run

```text
test result: ok. 235 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.28s
```

Current failures: none.

The lane uses the shared package comparison model: open, save, reopen, package
part graph comparison, ZIP entry comparison, canonical XML equivalence, and
schema-derived float lexical normalization.

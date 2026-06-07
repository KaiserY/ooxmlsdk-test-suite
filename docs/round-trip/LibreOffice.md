# LibreOffice Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/LibreOffice` |
| Manifest | `corpus/LibreOffice/manifest.toml` |
| Source | `https://github.com/LibreOffice/core` |
| License | MPL-2.0 |
| License files | `licenses/LibreOffice/COPYING.MPL`, `licenses/LibreOffice/COPYING.LGPL`, `licenses/LibreOffice/COPYING` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 3368 | 3345 | 0 | 23 | 0 | 2026-06-07 | 3368 | 0 |

The imported corpus preserves LibreOffice core-relative paths for supported
Office package fixtures only.

Extension distribution:

| Extension | Files |
| --- | ---: |
| `docx` | 2239 |
| `docm` | 16 |
| `dotx` | 4 |
| `dotm` | 1 |
| `xlsx` | 540 |
| `xlsm` | 14 |
| `pptx` | 549 |
| `pptm` | 1 |
| `potx` | 4 |

## Run Command

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test libreoffice_roundtrip -- --ignored
```

## Last Run

```text
test result: ok. 3368 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.91s
```

Current failures: none.

Invalid expectations currently cover empty/non-OOXML files, encrypted CDFV2
packages, corrupt ZIP packages, and LibreOffice fixtures that require repair
mode rather than normal OOXML package loading.

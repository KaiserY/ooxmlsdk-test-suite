# LibreOffice Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/LibreOffice` |
| Manifest | `corpus/LibreOffice/manifest.toml` |
| Source | `https://github.com/LibreOffice/core` |
| License | MPL-2.0 |
| License files | `licenses/LibreOffice/COPYING.MPL`, `licenses/LibreOffice/COPYING.LGPL`, `licenses/LibreOffice/COPYING` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Status |
| ---: | ---: | ---: | ---: | ---: | --- | --- |
| 3368 | 3368 | 0 | 0 | 0 | not run | imported |

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

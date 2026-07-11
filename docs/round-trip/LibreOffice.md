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
| 3368 | 3335 | 7 | 26 | 0 | 2026-06-07 | 3368 | 0 |

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

## CFB Round-Trip

| Total files | Round-trip candidates | Unsupported | Invalid | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 790 | 690 | 28 | 72 | 2026-07-11 | 790 | 0 |

```sh
cargo test -p olecfsdk-roundtrip-tests --test libreoffice_cfb_roundtrip -- --ignored
```

```text
test result: ok. 790 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
```

Each valid CFB fixture is opened, rebuilt, reopened, compared by its logical
storage/stream tree and stream bytes, then rebuilt and checked a second time.
LibreOffice's RC4-wrapped regression fixtures are unwrapped in memory with the
upstream corpus key before applying the same checks. Unsupported entries cover
raw pre-CFB Office formats and mislabeled non-CFB files; invalid entries cover
empty, malformed, and deliberately corrupt fixtures.

The build-time manifest audit rejects missing fixtures, duplicate
`cfb_roundtrip` entries, unsupported extensions, escaping paths, and empty
reasons. Files without an exception entry default to round-trip, so every
scanned fixture generates exactly one test.

# Open-XML-SDK Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/Open-XML-SDK` |
| Manifest | `corpus/Open-XML-SDK/manifest.toml` |
| Source | `https://github.com/dotnet/Open-XML-SDK` |
| License | MIT |
| License files | `licenses/Open-XML-SDK/LICENSE`, `licenses/Open-XML-SDK/NOTICE` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 884 | 875 | 6 | 3 | 0 | 2026-07-16 | 884 | 0 |

The current scaffold generates one ignored test per supported Office package
fixture. Encrypted and intentionally invalid fixtures are classified as
`invalid` in the corpus manifest and passed their invalid-package expectations.

## Last Run

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test open_xml_sdk_roundtrip -- --ignored
```

Result:

```text
test result: ok. 884 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.88s
```

The round-trip check is aligned with the high-standard `doc_samples` comparison
model: open, save, reopen, package part graph comparison, zip entry comparison,
canonical XML equivalence, and schema-derived float lexical normalization.

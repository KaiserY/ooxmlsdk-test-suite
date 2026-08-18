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
| 886 | 880 | 3 | 3 | 0 | 2026-08-15 | 886 | 0 |

The current scaffold generates one ignored test per supported Office package
fixture. Encrypted and intentionally invalid fixtures are classified as
`invalid` in the corpus manifest and passed their invalid-package expectations.

## Last Run

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test open_xml_sdk_roundtrip -- --ignored
```

Result:

```text
test result: ok. 886 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.34s
```

The round-trip check eagerly opens, saves, and reopens each package before
comparing its public part graph, complete relationship edge graph, ZIP entries,
and producer-compatible canonical XML. It then saves and eagerly reopens the
SDK output a second time, requiring the public graph and exact entry set to stay
stable, XML to remain strictly canonically equivalent, and binary payloads to
remain byte-identical.

# Apache POI Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/Apache-POI` |
| Manifest | `corpus/Apache-POI/manifest.toml` |
| Source | `https://github.com/apache/poi` |
| License | Apache-2.0 |
| License files | `licenses/Apache-POI/LICENSE`, `licenses/Apache-POI/NOTICE` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 677 | 614 | 0 | 63 | 0 | 2026-06-02 | 628 | 49 |

The current scaffold generates one ignored test per supported Office package
fixture.

## Last Run

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip -- --ignored
```

Result:

```text
test result: FAILED. 628 passed; 49 failed; 0 ignored; 0 measured; 0 filtered out; finished in 122.70s
```

Observed failure categories:

- unsupported or strict schema cases: `AlternateContent`, negative chart axis
  ids, `ep:Properties`, `xm:macrosheet`
- round-trip mismatches: chart extension ordering, customXml attribute loss,
  relationship namespace attributes, diagram extension namespaces

Invalid-package expectations include encrypted CDFV2 packages, corrupt fuzz
and crash fixtures, empty or missing-part packages, XML entity expansion
fixtures, and intentionally invalid OPC compliance cases.

## Run Command

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip -- --ignored
```

Useful first-pass filters:

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip test_data_spreadsheet -- --ignored
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip test_data_slideshow -- --ignored
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip test_data_integration -- --ignored
```

The round-trip check uses the same high-standard comparison model as the
Open-XML-SDK corpus lane.

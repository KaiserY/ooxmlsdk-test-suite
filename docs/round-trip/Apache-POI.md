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
| 677 | 601 | 12 | 64 | 0 | 2026-07-16 | 677 | 0 |

The current scaffold generates one ignored test per supported Office package
fixture.

## Last Run

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip -- --ignored
```

Result:

```text
test result: ok. 677 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.94s
```

Current failures: none.

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

## CFB Round-Trip

| Total files | Round-trip candidates | Unsupported | Invalid | Last run | Passed | Failed |
| ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 743 | 704 | 4 | 35 | 2026-07-11 | 743 | 0 |

```sh
cargo test -p olecfsdk-roundtrip-tests --test apache_poi_cfb_roundtrip -- --ignored
```

```text
test result: ok. 743 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
```

Each valid CFB fixture is opened, rebuilt, reopened, compared by its logical
storage/stream tree and stream bytes, then rebuilt and checked a second time.
The four unsupported fixtures are legacy non-CFB or mislabeled files. The 35
invalid fixtures are corrupt or fuzzed inputs and must be rejected.

The build-time manifest audit rejects missing fixtures, duplicate
`cfb_roundtrip` entries, unsupported extensions, escaping paths, and empty
reasons. Files without an exception entry default to round-trip, so every
scanned fixture generates exactly one test.

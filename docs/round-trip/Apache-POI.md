# Apache POI Round-Trip

| Field | Value |
| --- | --- |
| Corpus path | `corpus/Apache-POI` |
| Manifest | `corpus/Apache-POI/manifest.toml` |
| Source | `https://github.com/apache/poi` |
| License | Apache-2.0 |
| License files | `licenses/Apache-POI/LICENSE`, `licenses/Apache-POI/NOTICE` |

## Current Status

| Total files | Round-trip candidates | Open-only | Invalid | Known failures | Last run | Status |
| ---: | ---: | ---: | ---: | ---: | --- | --- |
| 677 | 677 | 0 | 0 | 0 | 2026-06-02 | full run aborted |

The current scaffold generates one ignored test per supported Office package
fixture.

The first full run aborted on
`Apache-POI/test-data/document/deep_table_cell.docx` with a stack overflow, so
there is not yet a complete pass/fail total for all 677 files.

## Initial Results

| Filter | Files run | Passed | Failed | Notes |
| --- | ---: | ---: | ---: | --- |
| `test_data_spreadsheet` | 364 | 316 | 48 | includes malformed, fuzz, chart, drawing, pivot, and xmlbomb failures |
| `test_data_slideshow` | 96 | 76 | 20 | includes corrupt zip, fuzz, chart axis, and chart extension failures |
| `test_data_integration` | 27 | 22 | 5 | includes customXml, altChunk, AlternateContent, and diagram namespace failures |
| Combined completed filters | 487 | 414 | 73 | excludes document corpus after the aborting case and other smaller directories |

Observed failure categories:

- corrupt or adversarial packages: `clusterfuzz-*`, `crash-*`,
  `poc-xmlbomb*`, `xlsx-corrupted.xlsx`
- unsupported or strict schema cases: `AlternateContent`, negative chart axis
  ids, `ep:Properties`, `xm:macrosheet`
- round-trip mismatches: chart extension ordering, customXml attribute loss,
  relationship namespace attributes, diagram extension namespaces
- aborting case: `test-data/document/deep_table_cell.docx`

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

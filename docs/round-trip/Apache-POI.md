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
| 682 | 612 | 6 | 64 | 0 | 2026-10-11 | 682 | 0 |

The current scaffold generates one ignored test per supported Office package
fixture.

## Last Run

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip -- --ignored
```

Result:

```text
test result: ok. 682 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.36s
```

`test-data/document/unicode-path.docx` now uses `open_only`: the SDK must eagerly
parse its main document, without requiring a lossless package save. Its existing
first-save failure, `Duplicate filename: word/document.xml`, is caused by
conflicting ZIP entry names and occurs before canonical XML comparison.

Upstream classifies this as a corrupt ZIP rejection test, not a normal
round-trip fixture:
[`testUnicodePathDocWithCorruptZipEntry`](https://github.com/apache/poi/blob/trunk/poi-ooxml/src/test/java/org/apache/poi/xwpf/usermodel/TestXWPFDocument.java)
asserts an opening exception. It is byte-identical to LibreOffice's
`package/qa/cppunit/data/unicode-path.docx`, whose test also requires rejection.
ZIP header filenames conflict with the Unicode path extra fields. The SDK
currently accepts the archive but later copies an extra entry under the main
document's name. The import recovery policy is unchanged. The generated test
continues to require successful parsing, rather than skipping the fixture or
expecting an arbitrary error.

Its configured Office PDF is an approved `OpenAndRepair` recovery reference and
remains a protected PASS. The PDF campaign includes this recovery despite its
`open_only` classification; ordinary open-only fixtures remain excluded. See
[the golden PDF entry point](../ooxmlsdk-pdf-test/corpus_pdf_conv.md).

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

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
| 3388 | 3355 | 7 | 26 | 0 | 2026-10-11 | 3388 | 0 |

The imported corpus preserves LibreOffice core-relative paths for supported
Office package fixtures only.

Extension distribution:

| Extension | Files |
| --- | ---: |
| `docx` | 2249 |
| `docm` | 16 |
| `dotx` | 4 |
| `dotm` | 1 |
| `xlsx` | 547 |
| `xlsm` | 14 |
| `pptx` | 552 |
| `pptm` | 1 |
| `potx` | 4 |

## Run Command

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test libreoffice_roundtrip -- --ignored
```

## Last Run

```text
test result: ok. 3388 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 32.25s
```

Three malformed packages now use `open_only`, which requires successful eager
parsing and main-part lookup. Their existing lossless-save failures were:

- `package/qa/cppunit/data/unicode-path.docx`: first save reports
  `Duplicate filename: word/document.xml`.
- `package/qa/cppunit/data/casing.docx`: complete relationship graph differs.
- `sw/qa/core/data/ooxml/pass/tdf93284.docx`: complete relationship graph differs.

All three failures reproduce with the SDK XML reader restored to the merged
baseline, before canonical XML comparison.

Upstream expectations support import/recovery checks for these fixtures:

- `unicode-path.docx` is a corrupt ZIP rejection test. Its ZIP header filenames
  conflict with the Unicode path extra fields; upstream `testUnicodeDOCX`
  requires an opening exception. Apache POI imports the same bytes and also
  requires rejection.
- `casing.docx` contains both `WORD/DOCUMENT.XML` and `word/document.xml` with
  different payloads. Upstream `testCaseInsensitiveDOCX` requires rejection
  for duplicate OPC part names. The current graph failure shows a relationship
  source changing from `word/document.xml` to `WORD/document.xml`; treating
  this solely as harmless case normalization would miss the conflicting parts.
- `tdf93284.docx` is in the import `pass` directory, which checks successful
  loading, not lossless package round-trip. Its `word/document.xml.rels` has
  duplicate `rId5` and `rId6`: font/theme relationships collide with image
  relationships. The SDK's existing first-wins recovery omits the two later
  image edges on save, reducing the complete graph from 84 to 82 edges.
  An import or recovery test fits this fixture better than exact graph
  preservation.

Duplicate-ID recovery varies by implementation. Apache POI's
`PackageRelationshipCollection` overwrites an earlier entry with the same ID;
the SDK retains the first. Open XML SDK's default packaging backend requires
unique relationship IDs (see
[`System.IO.Packaging`](https://github.com/dotnet/runtime/blob/v10.0.2/src/libraries/System.IO.Packaging/src/System/IO/Packaging/InternalRelationshipCollection.cs)).
Source inspection does not establish a shared repair behavior for this fixture.

The rejection assertions are in upstream
[`package/qa/cppunit/test_zippackage.cxx`](https://github.com/LibreOffice/core/blob/master/package/qa/cppunit/test_zippackage.cxx).
The import directory runner is in
[`unotest/source/cpp/filters-test.cxx`](https://github.com/LibreOffice/core/blob/master/unotest/source/cpp/filters-test.cxx).
Package loading policy is unchanged. The open-only main-part check compares
Word part names without ASCII case sensitivity, allowing the imported
`WORD/DOCUMENT.XML` spelling while still requiring a parsed Word main part.

All three configured Office PDFs are approved `OpenAndRepair` recovery
references and remain protected PASSes. Their golden coverage is retained
through existing, source-hash-bound recovery overrides. Ordinary open-only
fixtures remain excluded. See
[the golden PDF entry point](../ooxmlsdk-pdf-test/corpus_pdf_conv.md).

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

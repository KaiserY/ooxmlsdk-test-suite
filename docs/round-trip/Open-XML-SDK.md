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
| 886 | 880 | 3 | 3 | 0 | 2026-10-26 | 886 | 0 |

The current scaffold generates one ignored test per supported Office package
fixture. Encrypted and intentionally invalid fixtures are classified as
`invalid` in the corpus manifest and passed their invalid-package expectations.

## Last Run

```sh
cargo test -p ooxmlsdk-roundtrip-tests --test open_xml_sdk_roundtrip -- --ignored
```

Result:

```text
test result: ok. 886 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.68s
```

Rechecked after the Visio integration in PR #31 on 2026-10-26, using
ooxmlsdk 0.14.0 and zip 9.0.5: original-package comparison
and second-save stability both pass, with no new comparison relaxations or
manifest exceptions. The ZIP dependency enables `unreserved` to preserve
registered Unix extra fields when raw-copying package entries.

The round-trip check eagerly opens, saves, and reopens each package before
comparing its public part graph, complete relationship edge graph, ZIP entries,
and producer-compatible canonical XML. It then saves and eagerly reopens the
SDK output a second time, requiring the public graph and exact entry set to stay
stable, XML to remain strictly canonically equivalent, and binary payloads to
remain byte-identical.

XML whitespace is normalized once during parsing. Literal attribute TAB, LF,
and CR become spaces, while their character references retain distinct decoded
values in both producer-compatible and strict comparisons. Literal text CR and
CRLF become LF; a text `&#13;` remains CR. The comparator must not normalize
decoded attribute values a second time, which would hide serialization loss.

The 2026-10-10 XML whitespace audit also passed the core test lanes:

```sh
cargo test -p ooxmlsdk-test --no-default-features
cargo test -p ooxmlsdk-test
cargo test -p ooxmlsdk-test --features flat-opc
cargo test -p ooxmlsdk-test --features mce
cargo test -p ooxmlsdk-test --features validators
cargo test -p ooxmlsdk-corpus-test-support --lib
```

The new public `xml_whitespace` tests exercise byte and buffered readers,
save/reopen, literal CR/CRLF, character references, CDATA, mixed text, and
literal entity-like strings. A generated-node regression also compares the
original XML with its saved XML using the strict corpus comparator.

The 2026-10-11 full control run without the SDK's fast-text CR normalization
failed 104 existing corpus tests: Apache POI 39, LibreOffice 33, Open-XML-SDK 21,
and ClosedXML 11. Pandoc passed. With normalization restored, all 5,477 tests
passed. The pre-PR text writer emitted raw CR, masking the reader error because
the comparator normalized both input and output to LF. Reinstating that old
text-escaping behavior made the failing POI `ExcelWithHyperlinks.xlsx` control
pass again; it does not provide correct character-reference round-trip.

Four malformed POI/LibreOffice packages are now import-only expectations. Their
approved Office recovery goldens remain in the 5,370-source PDF campaign; the
Open-XML-SDK fixture classifications are unchanged.

Release performance controls compared the current fast path, unconditional
`xml10_content()`, in-place byte normalization, and normalized `Text` events
without `FastBytesText`. Twelve inputs across slice, cursor, and buffered reads
produced identical serialized models. Nine interleaved rounds on one CPU showed
about 5% higher parse time for the 1 MB worksheet and 10–15% for 512 long text
runs when fast events were removed. The complex Word fixture differed by only
0–2%, within run variation. In-place normalization helped some CR-heavy stream
inputs but did not materially improve ordinary text, so the smaller CR fallback
is retained. The extra CR check measured about 4–7 ns for 16–128 byte ASCII text
and 30–34 ns for 4 KB ASCII text in isolated conversion controls; these costs
are not whole-package throughput changes.

# ooxmlsdk-test-suite

## Round-Trip

Index of round-trip corpus coverage.

| Corpus | Files | Round-trip candidates | Open-only | Invalid | Result | Details |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Apache POI | 682 | 606 | 12 | 64 | 682 passed / 0 failed | [Apache POI](docs/round-trip/Apache-POI.md) |
| LibreOffice | 3388 | 3355 | 7 | 26 | 3388 passed / 0 failed | [LibreOffice](docs/round-trip/LibreOffice.md) |
| Open-XML-SDK | 886 | 877 | 6 | 3 | 886 passed / 0 failed | [Open-XML-SDK](docs/round-trip/Open-XML-SDK.md) |
| Pandoc | 235 | 234 | 1 | 0 | 235 passed / 0 failed | [Pandoc](docs/round-trip/Pandoc.md) |
| ClosedXML | 286 | 284 | 0 | 2 | 286 passed / 0 failed | [ClosedXML](docs/round-trip/ClosedXML.md) |
| **Total** | **5477** | **5356** | **26** | **95** | **5477 passed / 0 failed** | |

## CFB Round-Trip

The binary Office lane generates one ignored test for every imported legacy
Office fixture. Manifest exceptions are still tests: malformed CFB files must
be rejected, while non-CFB legacy formats must be reported as unsupported.

| Corpus | Files | Round-trip candidates | Unsupported | Invalid | Result | Details |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Apache POI | 743 | 704 | 4 | 35 | 743 passed / 0 failed | [Apache POI](docs/round-trip/Apache-POI.md#cfb-round-trip) |
| LibreOffice | 790 | 690 | 28 | 72 | 790 passed / 0 failed | [LibreOffice](docs/round-trip/LibreOffice.md#cfb-round-trip) |
| **Total** | **1533** | **1394** | **32** | **107** | **1533 passed / 0 failed** | |

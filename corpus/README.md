# Corpus

This directory stores third-party and project-derived documents used by the
test workspace.

The repository license covers the test code. Corpus files keep the license and
notice terms of their original projects. See `../licenses/` and
`../corpus-manifest.toml` for source metadata.

## Imported Corpora

| Corpus | Path | Files | License | Source | Manifest | License files |
| --- | --- | ---: | --- | --- | --- | --- |
| Apache POI | `corpus/Apache-POI` | 677 | Apache-2.0 | `https://github.com/apache/poi` | `corpus/Apache-POI/manifest.toml` | `licenses/Apache-POI/LICENSE`, `licenses/Apache-POI/NOTICE` |
| Open-XML-SDK | `corpus/Open-XML-SDK` | 884 | MIT | `https://github.com/dotnet/Open-XML-SDK` | `corpus/Open-XML-SDK/manifest.toml` | `licenses/Open-XML-SDK/LICENSE`, `licenses/Open-XML-SDK/NOTICE` |

The file count includes supported Office package fixtures only:
`docx`, `dotx`, `docm`, `dotm`, `xlsx`, `xltx`, `xlsm`, `xltm`, `pptx`,
`potx`, `pptm`, and `potm`.

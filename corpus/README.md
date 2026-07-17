# Corpus

This directory stores third-party and project-derived documents used by the
test workspace.

The repository license covers the test code. Corpus files keep the license and
notice terms of their original projects. See `../licenses/` and
`../corpus-manifest.toml` for source metadata.

## Imported Corpora

| Corpus | Path | OOXML files | Binary Office files | License | Source | Manifest | License files |
| --- | --- | ---: | ---: | --- | --- | --- | --- |
| Apache POI | `corpus/Apache-POI` | 682 | 743 | Apache-2.0 | `https://github.com/apache/poi` | `corpus/Apache-POI/manifest.toml` | `licenses/Apache-POI/LICENSE`, `licenses/Apache-POI/NOTICE` |
| LibreOffice | `corpus/LibreOffice` | 3388 | 790 | MPL-2.0 | `https://github.com/LibreOffice/core` | `corpus/LibreOffice/manifest.toml` | `licenses/LibreOffice/COPYING.MPL`, `licenses/LibreOffice/COPYING.LGPL`, `licenses/LibreOffice/COPYING` |
| libemf2svg | `corpus/libemf2svg` | 0 | 0 | GPL-2.0-only | `https://github.com/kakwa/libemf2svg` | `corpus/libemf2svg/manifest.toml` | `licenses/libemf2svg/LICENSE`, `licenses/libemf2svg/libuemf-COPYING` |
| Open-XML-SDK | `corpus/Open-XML-SDK` | 886 | 0 | MIT | `https://github.com/dotnet/Open-XML-SDK` | `corpus/Open-XML-SDK/manifest.toml` | `licenses/Open-XML-SDK/LICENSE`, `licenses/Open-XML-SDK/NOTICE` |

The OOXML count includes supported Office package fixtures only:
`docx`, `dotx`, `docm`, `dotm`, `xlsx`, `xltx`, `xlsm`, `xltm`, `pptx`,
`potx`, `pptm`, and `potm`.

The binary Office count includes `doc`, `dot`, `xls`, `xlt`, `ppt`, `pps`, and
`pot`. These 1533 files are covered by the manifest-driven CFB test lane.

`emfsdk-test` also uses EMF/WMF fixtures from existing Apache POI and
LibreOffice corpus directories, plus the libemf2svg corpus. Current EMF/WMF
fixture counts are:

| Corpus | EMF/WMF files | Used by |
| --- | ---: | --- |
| Apache POI | 22 | POI parser behavior and corpus roundtrip |
| LibreOffice | 156 | emfio and graphicfilter pass/fail corpus roundtrip |
| libemf2svg | 240 | converter resource and corrupted-input corpus roundtrip |

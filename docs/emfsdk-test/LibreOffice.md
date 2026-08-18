# LibreOffice EMF/WMF Coverage

`emfsdk-test` imports LibreOffice EMF/WMF fixtures from:

- `core/emfio/qa/cppunit/emf/data`
- `core/emfio/qa/cppunit/wmf/data`
- `core/vcl/qa/cppunit/graphicfilter/data/emf/pass`
- `core/vcl/qa/cppunit/graphicfilter/data/emf/fail`
- `core/vcl/qa/cppunit/graphicfilter/data/wmf/pass`
- `core/vcl/qa/cppunit/graphicfilter/data/wmf/fail`

The emfio files come from rendering and primitive import tests, so the initial
Rust coverage treats them as parse/write roundtrip fixtures. The graphicfilter
`pass` directories are also roundtrip fixtures. The graphicfilter `fail`
directories are reject fixtures: `emfsdk` should not accept them as valid
metafiles.

The copied fixture set currently contains 130 EMF/WMF files. LibreOffice
license files remain under `licenses/LibreOffice/`.

## Render Output Coverage

`emfsdk-test` owns focused EMF/WMF renderer coverage for bare metafile files.
This mirrors LibreOffice `emfio/qa/cppunit/{emf,wmf}` tests, which parse
metafiles into drawing primitives before any OOXML layout or PDF serialization
layer is involved.

The render lane intentionally checks output-chain completeness, not pixel-level
LibreOffice rendering equivalence. Active tests should assert that selected
LibreOffice metafiles decode to visible PNG output at a bounded target size,
covering representative vector, bitmap, text, brush, clip, and EMF+ records.
Keep final document-to-PDF assertions in `ooxmlsdk-pdf-test`; keep source OOXML
placement/bounds assertions in `ooxmlsdk-layout-test`.

Current focused render cases are in `crates/emfsdk-test/tests/render.rs` and
cover representative EMF/WMF fixtures from `emfio/qa/cppunit`.

## Commit Readiness

For EMF/WMF output-chain changes, the minimum test-suite verification before
reporting that changes are ready to commit is:

- `cargo test -p emfsdk-test`
- `cargo test -p ooxmlsdk-layout-test`
- `cargo test -p ooxmlsdk-pdf-test`

When the implementation in `../emfsdk` or the adapter in `../ooxmlsdk` changed
in the same branch, also verify those workspaces with their local default
checks before committing.

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

# libemf2svg EMF/WMF Coverage

`emfsdk-test` imports libemf2svg EMF/WMF fixtures from:

- `libemf2svg/tests/resources/emf`
- `libemf2svg/tests/resources/emf-ea`
- `libemf2svg/tests/resources/emf-corrupted`
- `libemf2svg/vendor/libuemf`

The normal resource and libuemf reference files are parse/write roundtrip
fixtures. The `emf-corrupted` directory is a reject corpus: `emfsdk` should
return an error instead of accepting those files.

The copied fixture set currently contains 240 EMF/WMF files. License and notice
material is copied into `licenses/libemf2svg/`, including the root GPL-2.0
license, libuemf COPYING, and vendor license files referenced by the upstream
tree.

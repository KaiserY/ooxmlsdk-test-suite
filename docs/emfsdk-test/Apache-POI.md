# Apache POI EMF/WMF Coverage

`emfsdk-test` imports Apache POI's EMF/WMF fixtures from
`poi/test-data/**.{emf,wmf}` into `corpus/Apache-POI/test-data`.

The POI-derived coverage has two parts:

- `tests/upstream_units.rs` ports small parser invariants from POI scratchpad
  tests, including placeable WMF units-per-inch validation, invalid
  `META_CREATEREGION` scan counts, invalid `EMR_POLYDRAW` Bezier command
  sequences, and EMF header description bounds/UTF-16 shape checks.
- `tests/corpus_roundtrip.rs` roundtrips all copied POI `.emf` and `.wmf`
  files through `emfsdk::Metafile`.

The copied fixture set currently contains 22 EMF/WMF files. Existing Apache POI
license and notice files remain under `licenses/Apache-POI/`.

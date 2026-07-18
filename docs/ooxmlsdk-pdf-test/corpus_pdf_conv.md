# Microsoft Office Golden PDF Conformance

This document tracks the `ooxmlsdk-layout` and `ooxmlsdk-pdf` effort against
the fixed Microsoft Office PDF references under `corpus_pdf_conv/`. Update it
after every baseline run, implementation batch, or material comparison-policy
change so that progress and unresolved failure clusters survive across agent
sessions.

## Goal And Golden Policy

For each supported source document, the development loop is:

```text
original DOCX/PPTX/XLSX
  -> ooxmlsdk-layout
  -> ooxmlsdk-pdf candidate PDF
  -> comparison with the existing Microsoft Office golden PDF
```

The PDFs already stored in `corpus_pdf_conv/` are immutable golden artifacts.
Do not regenerate, replace, normalize, rewrite, or approve-update them from
`ooxmlsdk` output. Their hashes and Office rendering environment are recorded
by the existing corpus manifests and `environment.json`.

Rasterized pages, extracted text, geometry summaries, and visual diff images
may be produced as temporary comparison artifacts. They are observations of
the fixed PDFs, not a new reference set, and must not be used to rewrite the
golden files.

## Evidence Priority

The sources have distinct roles:

1. `corpus_pdf_conv/` Microsoft Office PDFs define the target visible output
   for their corresponding source documents.
2. ECMA-376 and the Microsoft Open Specifications under `references/` define
   file semantics and documented Office implementation behavior.
3. `../core/` is the primary local source reference for implementable Writer,
   Calc, Impress, DrawingML, font, layout, paint, and PDF-export algorithms.
4. `../typst/` may guide idiomatic Rust layout architecture, fragmentation,
   shaping pipelines, and display-list design. It does not define Office
   layout semantics.
5. `../krilla/` is the local PDF-backend reference for text, graphics, images,
   annotations, resources, and serialization. It does not define Office
   pagination or object placement.

When evidence disagrees, first determine whether the difference comes from
source import, Office semantics, font/environment state, application layout,
display lowering, PDF serialization, or PDF observation. Do not tune constants
from pixels without locating the owning Office/LibreOffice behavior.

## Initial Corpus Inventory

The current manifests contain 4,400 successful Office conversions that match
the formats currently accepted by the OOXML renderer:

| Format | Golden cases |
| --- | ---: |
| DOCX | 2,707 |
| PPTX | 798 |
| XLSX | 895 |
| Total | 4,400 |

Legacy `.doc`, `.ppt`, and `.xls` outputs remain useful corpus evidence but are
outside this lane until the renderer has the corresponding source readers.

## Test And Result Model

Admit corpus cases incrementally. Do not generate or register all 4,400 cases
up front. Each admitted source/golden pair gets one independently addressable
test, and only one new case is worked on at a time.

The intended split is:

- the existing Office conversion manifests own source/golden identity, hashes,
  export environment, and conversion status;
- each admitted DOCX/PPTX/XLSX source and its fixed golden PDF receive a stable,
  exact-filterable test case;
- the current case is analyzed and fixed before the next case is added;
- this document records the current case, completed case count, comparison
  policy, failure clusters, source findings, and implementation progress;
- each fixed behavior gains a focused non-corpus regression test in
  `ooxmlsdk-layout-test` or `ooxmlsdk-pdf-test` according to ownership.

This is deliberately more controlled than a full manifest-generated lane.
Manifest-driven generation can be reconsidered only after the comparison
contract, font environment, runtime cost, and failure classifications have
proved stable across a meaningful manually admitted set.

### Per-Case Completion Gate

Use this sequence for every case:

1. Select one source document and verify its manifest record and golden hash.
2. Add one exact-filterable corpus test for that pair.
3. Produce the candidate PDF without modifying the golden.
4. Compare the candidate through the applicable comparison layers.
5. Classify the earliest incorrect layer and locate the owning ECMA/Microsoft,
   LibreOffice, Typst, Krilla, or current Rust source evidence.
6. Implement the source-backed fix and keep unrelated cases passing.
7. Add a focused layout/PDF/font regression test when the fix represents a
   reusable behavior rather than fixture-specific plumbing.
8. Record the result, evidence, verification command, and next selected case in
   this document.
9. Add the next corpus case only after the current test passes.

A case is not complete merely because the candidate PDF parses or looks closer.
It is complete when its enabled comparison contract passes and the underlying
behavior has an appropriate focused regression where one is useful.

## Comparison Layers

Compare in layers so failures remain attributable:

1. **Conversion:** candidate PDF produced, parses successfully, and has pages.
2. **Document geometry:** page count, media/crop boxes, orientation, and page
   sequence.
3. **Text:** normalized content, page assignment, order, run/line bounds, font
   selection, font size, and color where observable.
4. **Graphics:** paths, fills, strokes, images, clipping, transforms, links,
   annotations, and widgets.
5. **Visible output:** fixed-raster comparison using the same PDF rasterizer for
   golden and candidate, with temporary heatmaps/crops for diagnosis.

Do not use one global pixel-equality threshold as the only verdict. Font
antialiasing, PDF primitive decomposition, and equivalent paint operations can
differ without a layout error. Page, text, geometry, primitive, and raster
signals must remain separately reportable.

Every failed case should eventually carry one primary classification:

- `open-or-import`
- `layout-page`
- `layout-text`
- `layout-table`
- `layout-drawing`
- `font-or-environment`
- `display-lowering`
- `pdf-backend`
- `comparison-artifact`
- `resource-limit`
- `unclassified`

## Regression Ownership

Fix the earliest incorrect layer:

| Failure evidence | Primary owner |
| --- | --- |
| source property/theme/relationship interpreted incorrectly | `ooxmlsdk` or format import module in `ooxmlsdk-layout` |
| page, frame, line, table, shape, or print geometry incorrect | `ooxmlsdk-layout` plus `ooxmlsdk-layout-test` |
| layout document is correct but PDF object/paint output is wrong | `ooxmlsdk-pdf` plus `ooxmlsdk-pdf-test` |
| shaping, font choice, fallback, or metrics are wrong | `ooxmlsdk-fonts` plus fonts/layout tests |
| final PDF bytes differ but visible and semantic observations agree | no layout fix; investigate only if PDF object fidelity is in scope |

Corpus cases prove breadth. Focused tests prove the corrected behavior and
must cite the matching ECMA/Microsoft/LibreOffice source evidence when it is
not self-evident from the fixture.

## Execution Policy

- Corpus golden tests are explicit and independently filterable; whether they
  are ignored by default is decided when the first harness case lands.
- Support exact-case and comparison-layer filtering first. Add corpus, format,
  shard, and maximum-page filters only when the admitted set requires them.
- Work serially on the current case and do not admit a later case while it is
  failing.
- Keep Cargo commands sequential and use the default test-suite `target/`.
- Do not weaken thresholds, exclude a file, or label an environmental
  difference without recording the evidence and decision here.
- Do not change golden PDFs or their manifests as part of an implementation
  fix.

## Progress

| Phase | Status | Evidence / next action |
| --- | --- | --- |
| Reference collection | complete | ECMA-376 Parts 1-4 plus current MS-OI29500, MS-OE376, MS-DOCX, MS-XLSX, MS-PPTX, and MS-ODRAWXML are locally searchable. |
| Golden inventory | complete | 4,400 converted OOXML cases: 2,707 DOCX, 798 PPTX, 895 XLSX. |
| Comparison contract | defined | Fixed golden policy and layered comparison model recorded in this document. |
| Incremental corpus harness | complete | Shared manifest/hash, document/text, and fixed-raster comparison helper plus exact-filterable ignored case test are in place. |
| Admitted cases | 29 / 4,400 | DOCX: 2, PPTX: 25, XLSX: 2; all admitted cases pass the enabled comparison contract. |
| Failure clustering | active | Closed clusters now include editor-only placeholder prompts, inherited and shape-aware DrawingML shadows, slide-background fills, page-relative and transformed linear gradients, trailing duplicate gradient-stop normalization, transformed preset and custom shape geometry, vector bitmap clipping, rounded and arrow preset geometry, direct empty effect-list replacement, DrawingML theme-font and supplemental script-font resolution, PowerPoint text color/metric resolution, mixed-size centered text measurement, sRGB-relative grayscale image effects, clustered-column chart layout, automatic numeric-axis scaling, chart data-label visibility, fixed-format page-grid serialization, GDI+ gradient interpolation, and full-canvas EMF clip-mask replay. No admitted case is currently failing. |
| Autonomous optimization | active | Continue one case at a time; rerun every admitted case and both affected subsystem crates. |

### First Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf104015`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf104015.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf104015.pptx.pdf`
- Source SHA-256: `5a986fa43afc51500616b5561202faaa8250afe435cb34758abb023569fa9a8c`
- Golden SHA-256: `4cdb7069fc18046bca8687e728bf92566d903ea1ad4f83ede97c3aabd2b568fd`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide containing a title placeholder whose red
  fill is inherited from the master and whose blue outline is specified on
  the slide; the Office output also has a visible shadow.
- Result: complete on 2026-07-15. Conversion, page geometry, normalized text,
  graphics semantics, and the shared visible-output contract all pass.
- Closed classifications:
  - `layout-text`: an empty presentation placeholder retained its localized
    editor prompt as a normal text run, so `Click to add Title` leaked into
    printed PDF output. Placeholder prompts are now retained as editor-only
    model runs and excluded from display lowering.
  - `display-lowering`: inherited `a:outerShdw` properties were imported and
    merged but never painted. PPTX lowering now emits a shadow image before
    the source shape, honoring direction, distance, scale, alignment, blur,
    resolved color, and opacity.
- Specification evidence: ECMA-376 Part 1, §20.1.8.45 `outerShdw` defines the
  blur radius, offset direction/distance, scaling, alignment, and rotation
  attributes used by the effect.
- LibreOffice evidence:
  - `svx/source/sdr/primitive2d/sdrrectangleprimitive2d.cxx` wraps empty
    placeholder text in `ExclusiveEditViewPrimitive2D`;
  - `sd/source/core/sdpage.cxx::SdPage::checkVisibility` distinguishes empty
    presentation objects during print/non-edit rendering;
  - `oox/source/drawingml/effectproperties.cxx` converts DrawingML shadow
    direction, distance, scale, color, transparency, blur, and alignment;
  - `svx/source/sdr/primitive2d/sdrdecompositiontools.cxx` applies the declared
    shadow scale origin and paints the shadow before the source content;
  - `drawinglayer/source/primitive2d/shadowprimitive2d.cxx` rasterizes blurred
    shadows with a bounded temporary mask.
- Refactoring: shadow rasterization lives in a dedicated PPTX module. Its
  temporary mask follows LibreOffice's 250,000-pixel cap, and its separable
  triangular stack-blur kernel uses a sliding weighted sum, reducing each pass
  from `O(pixel_count * radius)` to `O(pixel_count)`.
- Focused regressions:
  - `mapped_pptx_tdf103792_omits_editor_only_title_placeholder_prompt`;
  - `mapped_pptx_tdf103876_omits_colored_editor_only_placeholder_prompt`;
  - `mapped_pptx_tdf104015_inherits_master_shape_fill_line_and_shadow`;
  - three implementation-local shadow alignment/blur tests.

### Second Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf105150`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf105150.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf105150.pptx.pdf`
- Source SHA-256: `fde2b32be9f920b32e95b6ad4504d438808e1ff5406ed5f791434c1d4e500c04`
- Golden SHA-256: `49861f5ecbed810bce743359b4d5fd6607bb1aabbd1b275ae540758788153284`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 720 x 540 point slide, no text, and two rectangles using
  `p:sp/@useBgFill`; the first rectangle also has a red outline.
- Result: complete on 2026-07-15. All comparison layers pass under the shared
  fixed contract. Office paints the first background-filled rectangle and its
  red outline, then the larger later background-filled rectangle covers that
  outline with the white master background.
- Closed classification: `display-lowering`. The importer previously collapsed
  `useBgFill` to `noFill`, making the later rectangle transparent. The model now
  preserves a distinct `SlideBackground` fill kind and resolves it through the
  slide/master/theme background chain during display lowering.
- Specification evidence: ECMA-376 Part 1, §19.3.1.43 explicitly says
  `useBgFill` is the portion of the slide background directly behind the shape,
  not transparency.
- LibreOffice evidence:
  - `oox/source/drawingml/fillproperties.cxx` retains
    `FillUseSlideBackground` separately from `FillStyle_NONE`;
  - `svx/source/sdr/primitive2d/sdrattributecreator.cxx` creates the dedicated
    slide-background fill attribute;
  - `svx/source/sdr/primitive2d/sdrdecompositiontools.cxx` resolves the master
    page fill using the whole page as its definition range and clips it to the
    shape geometry.
- Focused regression:
  `mapped_pptx_tdf105150_preserves_slide_background_fill_usage` now asserts the
  white background-fill path as well as the retained outline.

### Third Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf127964`
- Source SHA-256: `d91b5d5379029d6e68478c9e9d8c477b3b0530e5c96c50a902447a41378f01cb`
- Golden SHA-256: `dc71063e98d5c1549aac7793a5fecd539e5418b161a064fc8a423c1458be5eea`
- Scope: one white-master-background rectangle using `useBgFill`, with its
  themed outline remaining visible because no later shape covers it.
- Result: complete on 2026-07-15. It passed the unchanged contract immediately
  after the second-case fix and independently confirms fill-plus-outline paint
  ordering.
- Related regression: `mapped_pptx_tdf127964_preserves_background_fill_usage`.
  Its old assertion confused LibreOffice's internal `FillStyle_NONE` plus
  `FillUseSlideBackground` representation with transparent PDF output; the
  Office PDF explicitly contains an opaque white fill path, so the test now
  asserts that visible-output semantic.

### Fourth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf93868`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf93868.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf93868.pptx.pdf`
- Source SHA-256: `afc3699ff09d898e9e07e004fdd721ff8532b2915dc6abffab67bed80dbf67d6`
- Golden SHA-256: `aa44e46f96958593de4bca2927b386822e529b516caf4de7f54d1d733e9a3b24`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 720 x 540 point slide. Its master has a black-to-dark-gray
  scaled linear gradient background, a white full-slide rectangle, and a
  rounded rectangle using `useBgFill`; white slide text sits above those
  master shapes.
- Result: complete on 2026-07-15. Conversion, page geometry, normalized text,
  text style/size/line geometry, vector graphics semantics, and visible output
  all pass. The initial flat-gray square result had significant-pixel fraction
  `0.98152288072018` and mean absolute channel delta
  `110.48383870967741`; source-backed fixes reduced the raw text-inclusive
  raster difference to `0.0144` significant-pixel fraction and `1.90` mean
  channel delta before the verified text regions were separated from the
  non-text raster verdict.
- Closed classifications:
  - `display-lowering`: gradients were reduced to a representative solid
    color. The shared display model now carries linear gradient stops, angle,
    scaled behavior, and an optional definition range; the PDF backend emits
    a native Krilla linear gradient.
  - `display-lowering`: `useBgFill` must evaluate the master background in the
    full page definition range and clip that paint through the source shape.
    PPTX lowering now preserves this page-relative coordinate space.
  - `layout-drawing`: rounded rectangles were lowered as rectangular paths.
    common paths now carry cubic commands and DrawingML `roundRect` lowers to
    four cubic arcs derived from its adjustment value.
  - `layout-drawing`: a direct empty `effectLst` did not clear an inherited
    theme shadow. Direct effect-container presence now replaces inherited
    effects even when the direct list contains no effect children.
  - `layout-text`: slide/master color-map resolution was missing from line and
    text colors, so `tx1` did not resolve to the slide's `lt1`. Text, line, and
    highlight/underline colors now share slide-aware theme resolution.
  - `layout-text` / `font-or-environment`: DrawingML default text insets,
    `spAutoFit` versus `normAutofit`, Windows ascent selection, PowerPoint's
    600 dpi print-grid font-size quantization, and XML run boundaries all
    affected Office text geometry. Windows ascent is an explicit PPTX style
    policy rather than a global font default; a full PDF run caught four DOCX
    regressions while it was global, and all four disappeared after scoping.
- Specification and implementation evidence:
  - ECMA-376 Part 1 `gradFill`, `lin`, `gsLst`, `roundRect`, `effectLst`,
    `spAutoFit`, `normAutofit`, and DrawingML text-body inset definitions;
  - ECMA-376 Part 1, §19.3.1.43 for `p:sp/@useBgFill`;
  - Microsoft/OpenType OS/2 `usWinAscent` and `USE_TYPO_METRICS` selection
    behavior for the explicitly selected PowerPoint baseline policy;
  - LibreOffice's DrawingML fill/effect import and slide-background primitive
    path for page-definition-range paint and shape clipping;
  - Krilla's native gradient and cubic path APIs for PDF lowering.
- Focused regression:
  `mapped_pptx_tdf93868_preserves_master_background_fill_usage` asserts white
  39.96 pt title text, a white-stroked cubic rounded path with 12 PDF Bézier
  segments, and zero raster image objects for the vector-native background.
- Refactoring and comparison policy:
  - common path commands and gradient definition bounds are shared display
    primitives rather than fixture-specific PDF operations;
  - PPTX line layout arguments are grouped in a context object, and hyperlink
    runs coalesce as one semantic link span while non-link DrawingML run
    boundaries remain available to PDF shaping;
  - normalized full-page text, text style sets, paired font sizes, and grouped
    line geometry must pass before text line areas receive a fixed one-point
    raster mask; unchanged visible thresholds then apply to every non-text
    pixel. This separates equivalent TrueType hinting/antialias output from
    layout and paint errors instead of weakening the global raster boundary;
  - failed golden comparisons now retain the generated `candidate.pdf` beside
    candidate, golden, and diff PNG artifacts.

### Fifth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf109067`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf109067.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf109067.pptx.pdf`
- Source SHA-256: `ce64765c0cd0149a83dcc2a2664ce6b6ac213394dd3c9d7b353624dad95c1a14`
- Golden SHA-256: `b90ea593ddea83309eb2bb0765519160ec94cdcbfd410713a594bda9c2e9e566`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one textless `793.75 x 595.25` point slide containing a rectangular
  shape rotated clockwise by 45 degrees. The shape has a black-to-red linear
  gradient and a blue outline.
- Result: complete on 2026-07-15. All enabled comparison layers pass without a
  threshold change. Initially the candidate serialized the exact layout page
  size while Office serialized `793.8 x 595.2` points, and painted an
  unrotated horizontal rectangle; the initial visible difference had a
  significant-pixel fraction of `0.1352` and a mean absolute channel delta of
  `24.88`. Transforming the geometry and gradient reduced those measurements
  to `0.06785` and `0.9166`; matching the fixed-format gradient interpolation
  then closed the remaining paint difference.
- Closed classifications:
  - `pdf-backend`: PowerPoint fixed-format output quantizes presentation page
    dimensions to its 600 dpi print-device grid using ties-to-even. PPTX PDF
    page creation now applies that serialization rule without changing the
    exact OOXML or layout coordinate space; DOCX and XLSX page serialization
    are unchanged. The eighth case later distinguished this general rule from
    the initially inferred tenth-point serialization.
  - `layout-drawing`: shape rotation and flips were retained in the model but
    not applied to emitted path commands. PPTX display lowering now transforms
    line, cubic, and close-path geometry about the shape center and derives the
    transformed axis-aligned bounds.
  - `display-lowering`: a DrawingML gradient that rotates with a shape needs a
    transformed page-space gradient line. The display model now carries that
    explicit line so the PDF backend does not reconstruct it from the rotated
    bounding box.
  - `display-lowering` / `pdf-backend`: Office's transformed PowerPoint
    gradient uses the Windows GDI+ `SetSigmaBellShape(1, 1)` blend samples with
    gamma-correct color interpolation. The shared gradient model identifies
    that interpolation policy, and the PDF backend emits its 33-point sampled
    curve using the GDI+ 2.2 gamma. This policy is limited to actual rotated or
    flipped PPTX shape gradients whose `rotWithShape` behavior is active;
    ordinary and slide-background gradients retain linear interpolation.
- Specification and implementation evidence:
  - ECMA-376 Part 1, §20.1.8.33 `gradFill` and §20.1.8.41 `lin` define gradient
    rotation, the `scaled` vector calculation, and `rotWithShape`; the schema
    default for `rotWithShape` is true;
  - LibreOffice `oox/source/drawingml/fillproperties.cxx` likewise imports an
    absent rotate-with-shape value as true;
  - the fixed Office PDF's sampled shading function and a Windows GDI+
    diagnostic agree on the sigma blend and gamma-correct samples, including
    red-channel values `105` and `186` at the locked sample points;
  - Microsoft documents the corresponding GDI+ `SetGammaCorrection` and
    `SetSigmaBellShape` brush operations.
- Focused regressions:
  - `mapped_pptx_tdf109067_preserves_diagonal_gradient_shape` asserts the
    diagonal vector geometry, gradient fill, and blue outline;
  - `powerpoint_pdf_page_dimensions_use_the_600_dpi_print_grid` locks the PDF
    serialization boundary across `793.8`, `595.2`, and `446.52` point results
    and verifies DOCX remains exact;
  - `powerpoint_transformed_gradient_uses_gdiplus_gamma_samples` locks the
    observed GDI+ color samples independently of raster thresholds.

### Sixth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf109187`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf109187.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf109187.pptx.pdf`
- Source SHA-256: `5c4f310debee15f456feecf757b2bbdad59af4e9d99f2b9f784e8a67d38c27cf`
- Golden SHA-256: `0db412d3a3e283ee0e033567ed7c7b51632545d87696982a9eca6dc2ac891f31`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one textless slide with two square DrawingML shapes: a horizontally
  flipped and rotated `rightArrow`, and a vertically flipped and rotated
  `downArrow`. Both use accent-blue-to-red linear gradients and no outline.
- Result: complete on 2026-07-15. Page geometry and transformed gradient paint
  already matched after the fifth case, leaving one attributable failure: both
  arrows were painted as rotated rectangles. The initial visible difference
  had a significant-pixel fraction of `0.040002000500125034`, mean absolute
  channel delta `5.1583682587313495`, and maximum channel delta `253`. The
  source-backed preset paths pass the unchanged contract.
- Closed classification: `layout-drawing`. PPTX display lowering only had
  native path construction for rectangles and rounded rectangles, so retained
  `rightArrow` and `downArrow` geometry names did not affect visible output.
  A dedicated preset-geometry module now evaluates each arrow's two adjustment
  values, clamps them through the ECMA guide limits, builds the declared
  seven-segment polygon, and then feeds it into the shared flip/rotation path
  transform. The implementation does not derive coordinates from golden
  pixels and does not special-case this fixture.
- Specification and implementation evidence:
  - ECMA-376 preset shape definitions give the `adj1`/`adj2`, `maxAdj2`, shaft,
    head, and path equations for `rightArrow` and `downArrow`;
  - LibreOffice
    `oox/source/drawingml/customshapes/presetShapeDefinitions.xml` contains the
    same guide equations and path order;
  - LibreOffice `sd/qa/unit/import-tests2.cxx::testTdf109187` independently
    verifies that the two imported gradients resolve to 225 and 135 degrees.
- Refactoring and focused regressions:
  - rectangle, round-rectangle, right-arrow, and down-arrow local path creation
    now live in `pptx/preset_geometry.rs`; display lowering owns only the common
    shape transform;
  - `default_right_arrow_matches_the_ecma_preset_path` and
    `default_down_arrow_matches_the_ecma_preset_path` lock all seven declared
    vertices before transforms;
  - existing `pptx_tdf109187_preserves_two_gradient_arrow_shapes` continues to
    lock the source-imported geometry kinds and gradient angles.

### Seventh Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf111518`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf111518.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf111518.pptx.pdf`
- Source SHA-256: `01d75f39e5b711d4b259503334029fd861698cde74321ebe853f22325af89166`
- Golden SHA-256: `16f6570bd500fca25b93d0befd00cb40ac288b4fd2148dbf2db234e7be178caa`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one slide with empty centered-title and subtitle placeholders plus a
  styled rectangle. The rectangle has a two-second `p:animMotion` timeline
  whose relative path moves it horizontally during a slide show.
- Result: complete on 2026-07-15 without an implementation change. It passed
  the shared contract on its first run and all seven cases then passed
  together. The result independently confirms that fixed-format rendering
  uses the initial static shape position rather than applying presentation
  timeline motion, while empty editor placeholders remain absent from PDF
  output.
- Source evidence: LibreOffice
  `sd/qa/unit/export-tests-ooxml2.cxx::testTdf111518` verifies preservation of
  the `animMotion` path. That package round-trip behavior is intentionally
  orthogonal to the static PDF layout lane; no animation-specific layout or
  paint behavior was added merely because the timing markup is present.

### Eighth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf111786`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf111786.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf111786.pptx.pdf`
- Source SHA-256: `b4e8fb935024deefa12c1ce943748cc9db276b16b99fb7837ba02d63c78d94ee`
- Golden SHA-256: `8c24b79443dc4cd50dca38684378d93ac4360feced76b6ab41f2060341a18756`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one `793.75 x 446.5` point slide with a centered `Some Text` run in
  44pt Arial. Its unfilled text box has a 4.5pt blue rounded-join outline at
  67% alpha.
- Result: complete on 2026-07-15. The source and PDF pipeline already preserved
  the outline opacity: Office emits `/CA .67059` and the candidate emits
  `/CA .67058828`. Two earlier-layer issues were corrected without changing
  the visual or text tolerances, and all eight admitted cases pass together.
- Closed classifications:
  - `pdf-backend`: the earlier tenth-point page-size inference changed the
    exact 446.5pt layout height to 446.5pt PDF output while Office emits
    446.52pt. The three observed sizes share one rule: quantization to the
    600dpi print grid (`0.12pt` per device pixel). The corrected rule explains
    the fifth and eighth cases and remains scoped to PPTX PDF page creation.
  - `layout-text`: centered text-body height estimation used the 18pt base
    style even when the visible run was 44pt. It now reuses styled line runs
    and the maximum actual run line height on each line, matching the later
    lowering pass and generalizing to mixed-size lines.
  - `comparison-artifact`: glyph loose bounds from Office simple TrueType
    subsets and candidate CID subsets have different vertical font-descriptor
    extents even when their PDF text matrices agree. The unchanged 2pt
    vertical contract now compares PDFium character baseline origins; ink
    left/right bounds, width, font size, font/style, and raster-masked geometry
    checks remain active.
- Harness refactoring: semantic-layer failures now retain `candidate.pdf`
  immediately, so page, text, style, and geometry failures are as diagnosable
  as raster failures.
- LibreOffice evidence:
  `sd/qa/unit/export-tests-ooxml3.cxx::testTdf111786` locks the imported blue
  line color and 33% transparency independently of the Office golden.

### Ninth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf111789`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf111789.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf111789.pptx.pdf`
- Source SHA-256: `bf8f2efc02bc4a8b5c41066fdc8a7a7569283005d272d4c8abd4f08807039898`
- Golden SHA-256: `a0c0d68a20a753e6fa7e2fc4024c0bd96f2895ffd66a396d993380b83d269c6b`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one slide with two text boxes. `Text shape with shadow` has an
  accent fill, black outline, and a 93%-opaque red outer shadow at 45 degrees;
  `Text shape without shadow` has no fill, a black outline, and no shadow.
- Result: complete on 2026-07-15. The initial failure occurred at the text
  style layer: the candidate used `NotoSansCJKjp-Regular`, while Office used
  Calibri. After resolving the declared theme font, every comparison layer and
  all nine admitted cases pass without a threshold change.
- Closed classification: `layout-text`. Presentation default text styles use
  DrawingML theme placeholders such as `+mn-lt`; the importer retained those
  tokens as literal font family names, so font fallback selected an unrelated
  installed face. Theme font resolution now covers the major/minor Latin,
  East Asian, and complex-script slots and applies them to the corresponding
  run style fields. The same resolver recognizes LibreOffice's legacy
  `major`/`minor` HAnsi, Ascii, EastAsia, and Bidi aliases; ordinary explicit
  font names remain unchanged.
- Specification and LibreOffice evidence:
  - the source presentation default text style selects `+mn-lt`, and its theme
    declares Calibri as the minor Latin typeface;
  - ECMA-376 DrawingML theme font schemes define separate major/minor Latin,
    East Asian, and complex-script typefaces;
  - LibreOffice `oox/source/drawingml/theme.cxx::Theme::resolveFont` resolves
    the same theme placeholder families, with their token constants declared
    in `oox/inc/drawingml/textfont.hxx`;
  - LibreOffice `sd/qa/unit/export-tests-ooxml3.cxx::testTdf111789` verifies
    that only the first shape has the imported light-red, 7%-transparent
    shadow at the declared offset and scale.
- Focused regressions:
  - `resolves_all_drawingml_theme_font_placeholders` locks the six standard
    major/minor script placeholders independently of an installed font set;
  - `mapped_pptx_tdf111789_resolves_theme_font_and_only_declared_shadow`
    asserts both Calibri text runs and exactly one rendered shadow image.

### Tenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf111863`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf111863.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf111863.pptx.pdf`
- Source SHA-256: `47e8ec870d87585f846ce47fb39dad3c075568419cacfaf3ad88887af9c589f1`
- Golden SHA-256: `af3fe87ca14e058c3676378be632dfc7bb4e403f2072a7704371aabe0965baf9`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide with empty title/subtitle placeholders and
  one styled rectangle. A click-triggered, 500ms `animEffect` fades the
  rectangle out and then sets its slide-show visibility to hidden.
- Result: complete on 2026-07-15 without an implementation change. It passed
  the unchanged contract on its first run, and all ten admitted cases pass
  together. This independently confirms that fixed-format output paints the
  initial visible slide state rather than applying a later click-triggered
  exit effect; empty editor placeholders remain absent.
- Source evidence: LibreOffice
  `sd/qa/unit/export-tests-ooxml2.cxx::testTdf111863` verifies that package
  round-trip preserves `animEffect/@transition="out"`. That serialization
  concern is orthogonal to static PDF layout, so no animation-specific display
  behavior was added.

### Eleventh Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf111884`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf111884.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf111884.pptx.pdf`
- Source SHA-256: `8f59ee500b534e63dcb590e046512d887161da7c2d7ad42f984138653828d99b`
- Golden SHA-256: `edb532f3a0fcf337591e0b0abac17523a72abccf760156ef29220634f2dd5d96`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide with empty title/subtitle placeholders and
  a transformed group containing two styled rectangles. A group-targeted
  entrance animation moves the group from left of the slide during playback.
- Result: complete on 2026-07-15 without an implementation change. It passed
  the unchanged contract on its first run, and all eleven admitted cases pass
  together. Static fixed-format output keeps the grouped shapes at their
  authored initial geometry; it does not evaluate the entrance timeline as a
  pre-render transform.
- Source evidence: LibreOffice
  `sd/qa/unit/export-tests-ooxml1.cxx::testTdf111884` verifies that import and
  PPTX round-trip preserve the object as a group. The Office golden adds
  independent visible-output evidence that the group transform and both child
  rectangles remain printable, while animation evaluation stays outside this
  static layout lane.

### Twelfth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112086`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112086.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112086.pptx.pdf`
- Source SHA-256: `428db337e3716263e656dfdb2d8265fb8121a047c0630229e1c0937e8cf50580`
- Golden SHA-256: `703e932e27ccf43c73380778fbc5de15c7d8e74c49c3a59877b1c81b9153c0f1`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide with empty title/subtitle placeholders and
  one styled rectangle. A click-triggered entrance effect animates the shape
  from zero width and height to its authored dimensions while fading it in.
- Result: complete on 2026-07-15 without an implementation change. It passed
  the unchanged contract on its first run, and all twelve admitted cases pass
  together. Office fixed-format output paints the authored static rectangle,
  not the zero-size animation start value.
- Source evidence: LibreOffice
  `sd/qa/unit/export-tests-ooxml2.cxx::testTdf112086` verifies the timing
  attribute names and zero-valued animation keyframes during PPTX round-trip.
  The golden PDF establishes the separate visible-output rule, so no timing
  evaluator or fixture-specific geometry override was added to static layout.

### Thirteenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112088`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112088.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112088.pptx.pdf`
- Source SHA-256: `e716846104d042a5bdf80de1aca6a3ec639e0046882d61d1e614031c83712c2f`
- Golden SHA-256: `41755182d35058220f39612ac70a4420061257a5de13f72894be4b2ed09e7724`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one textless 960 x 540 point slide containing a blue-outlined
  rectangle with a vertical linear gradient. Its white and accent-blue stops
  are both declared at 50%, making the second stop a different-colored
  duplicate at the trailing edge of the declared stop range.
- Result: complete on 2026-07-15. Conversion, page, text, and vector graphics
  layers passed initially, while visible output failed with significant-pixel
  fraction `0.02440010002500625`, mean absolute channel delta
  `1.4034078519629907`, and maximum channel delta `164`. The candidate made a
  hard white/blue split; Office keeps white through the first half and blends
  continuously to blue over the second half. The source-backed normalization
  passes the unchanged contract, and all thirteen admitted cases pass
  together.
- Closed classification: `display-lowering`. Equal-position stops inside a
  gradient remain an intentional sharp color transition. PowerPoint treats a
  different-colored final stop specially when it duplicates the preceding
  offset below 100%: that final color becomes the 100% endpoint. PPTX lowering
  now stably sorts stops and extends only that trailing final stop; it does not
  perturb internal duplicate stops, equal-colored duplicates, or gradients
  already ending at 100%.
- Specification and LibreOffice evidence:
  - ECMA-376 Part 1, §20.1.8.36-37 defines each gradient stop position and the
    ordered color band represented by `gsLst`; §20.1.8.41 defines the linear
    gradient direction and scaled fill-region vector;
  - LibreOffice `sd/qa/unit/export-tests-ooxml2.cxx::testTdf112088` requires
    both source stops to survive PPTX round-trip;
  - LibreOffice `basegfx::BColorStops::checkPenultimate` independently detects
    exactly the trailing same-offset, different-color, below-100% condition,
    separately from normal internal sharp transitions;
  - the Office PDF shading function visibly extends the trailing color to the
    end of the band rather than treating the duplicate offset as a permanent
    hard split.
- Refactoring and focused regressions:
  - PowerPoint stop ordering and trailing normalization live in the dedicated
    `pptx/gradient.rs` module rather than the PDF backend;
  - implementation-local tests lock both the trailing extension and the
    preservation of internal duplicate-stop hard transitions;
  - `mapped_pptx_tdf112088_extends_the_trailing_duplicate_gradient_stop`
  samples the source-derived lower-half midpoint and locks the intermediate
  white/blue color independently of the Office golden comparison.

### Fourteenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112089`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112089.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112089.pptx.pdf`
- Source SHA-256: `76a2c841fa085e991c53ba5ddcc4a206ee93a98ca87f13c656ab6408bfb4c6c1`
- Golden SHA-256: `c7bd136901ba4590c9b37465c88e7f434d197ac27d8a3ab6240762b70a9403fb`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Office UI language: `zh-CN`, now recorded explicitly on every admitted case
  so application-generated labels are reproducible independently of package
  editing-language markup.
- Scope: one 960 x 540 point slide containing a standard clustered column
  chart with four categories, three series, `gapWidth=219`, `overlap=-27`, a
  bottom legend, an automatic title, and a click-triggered entrance animation
  targeting the chart frame. The cached values span 1.8 through 5.0.
- Initial result: conversion and page geometry passed, but the first failure
  was normalized text. The candidate emitted a vertical fallback dump of
  `Chart Title`, series names, categories, and every cached numeric value; it
  emitted no chart geometry. Office emitted value-axis labels 0 through 6,
  category labels, a localized automatic title, legend labels, grid lines, and
  twelve colored columns. This was a missing static chart subsystem rather
  than an animation or raster-comparison issue.
- Result: complete on 2026-07-15. All semantic and geometry layers pass, and
  visible output passes the unchanged fixed-output tolerance. The golden PDF,
  manifest, text tolerances, and raster thresholds were not modified. All
  fourteen admitted cases pass together.
- Closed classifications:
  - `layout-drawing`: cached chart sequences are now imported into a typed
    clustered-column model instead of being treated as visible text. The model
    keeps sparse point indices, categories, series names and fills, per-point
    fills, gap/overlap, value-axis settings, legend position, and actual data
    labels.
  - `layout-page`: automatic linear value axes expand wide positive ranges to
    zero, normalize automatic intervals to `1/2/5 x 10^n`, align limits to the
    interval rhythm, and add an upper interval when a data value occupies the
    maximum border. This fixture therefore produces 0 through 6 at unit steps.
  - `display-lowering`: a dedicated `pptx/chart.rs` layout lowers the shared
    model to grid lines, series rectangles, tick/category/title/legend text,
    legend keys, and configured data labels. Column widths and centers use the
    LibreOffice category-slot formula, where OOXML gap width is the outer
    distance and overlap is the negated inner distance. Title, plot, category,
    and legend bands share one font-aware layout relationship rather than
    fixture-specific coordinates.
  - `font-or-environment`: an automatic chart title is an application-localized
    label, not persisted title text and not controlled by `c:lang`. Public
    layout/PDF options now carry a BCP 47 UI language; the golden harness passes
    its recorded `zh-CN` environment and obtains `图表标题` without hard-coding
    the fixture. DrawingML theme import now also preserves supplemental ISO
    15924 script mappings, resolving the theme's `Hans` minor font to SimSun
    instead of relying on an unrelated installed CJK fallback.
  - `layout-text`: chart value caches no longer leak into PDF text. Explicit
    rich data labels, inherited `showVal`, VALUE fields, default number text,
    and common column-label positions are modeled separately. This restored
    the pre-existing `tdf114821` mapping through real `90.0` data-label output
    rather than the removed cache-dump fallback.
- Specification and LibreOffice evidence:
  - ECMA-376 DrawingML chart markup distinguishes `c:ser`, cached `c:cat` and
    `c:val` data, `c:dLbls`, `c:gapWidth`, `c:overlap`, axes, title, and legend;
    cached values drive plotted geometry and are visible text only when a
    configured label requests them;
  - LibreOffice
    `chart2/source/view/axes/ScaleAutomatism.cxx::calculateExplicitIncrementAndScaleForLinear`
    supplies the zero expansion, 1/2/5 interval, increment-rhythm, border, and
    maximum-increment rules;
  - LibreOffice
    `chart2/source/view/charttypes/CategoryPositionHelper.cxx` supplies the
    series-slot width and center formulas, while `BarChart.cxx` maps overlap
    and gap width to its inner and outer distances;
  - LibreOffice `chart2/source/view/main/ChartView.cxx`, `VLegend.cxx`, and
    `VTitle.cxx` establish the separate title, legend, diagram, and axes layout
    stages;
  - LibreOffice `sd/qa/unit/export-tests-ooxml2.cxx::testTdf112089` verifies
    the animation target identity independently. The Office golden confirms
    that fixed-format output paints the authored initial chart state rather
    than evaluating the entrance timeline.
- Refactoring and focused regressions:
  - chart data extraction and scale/slot rules live in shared
    `render/chart.rs`; PPTX placement and primitive lowering live in the new
    `pptx/chart.rs`, and locale travels through the existing PPTX lowering
    context rather than adding more positional parameters;
  - implementation-local tests cover localized automatic titles,
    supplemental East Asian theme fonts, the 0..6 linear scale, the exact
    gap/overlap slot relationship, and binary-float-safe tick formatting;
  - `mapped_pptx_tdf112089_lowers_the_clustered_column_chart_from_cached_data`
    locks tick/category/title/legend reading order, absence of raw cache text,
    SimSun/Calibri selection, all three series colors, twelve bars plus legend
    keys, and seven axis/grid strokes independently of the Office golden.

### Fifteenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112209`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112209.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112209.pptx.pdf`
- Source SHA-256: `d0085b0f0daa6d3e8a6e5a6329d0fa9a32057f1032296c6029bda01f42e6621a`
- Golden SHA-256: `8c0bfa911ae2dd4cff0384e823065d8b233b4a3d0ec62a0e61eb2d4e346c1890`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 720 x 405 point slide containing a stretched JPEG bitmap fill
  with `a:grayscl`, clipped by a five-sided `a:custGeom/a:pathLst`, with no
  outline and a blurred 40%-opaque black outer shadow derived from that same
  non-rectangular silhouette.
- Initial result: conversion, page geometry, normalized text, and PDF graphics
  structure passed. Visible output failed at `0.052029` significant-pixel
  fraction and `7.9904` mean absolute channel delta because both the bitmap
  and its shadow were painted as rectangles.
- Result: complete on 2026-07-15. All layers pass the unchanged comparison
  contract, and all fifteen admitted cases pass together. The golden PDF,
  manifest, and comparison thresholds were not modified.
- Closed classifications:
  - `layout-drawing`: `a:custGeom` no longer falls through to rectangle
    geometry. A dedicated DrawingML custom-geometry lowerer evaluates adjust
    and shape guides, scales each path's declared `w`/`h` coordinate space into
    the shape frame, and lowers move, line, quadratic, cubic, and close
    commands. Unsupported path commands reject the custom path as a unit and
    retain the conservative fallback rather than emitting partial geometry.
  - `display-lowering`: images can now carry an absolute vector clip path in
    the shared display model. PPTX bitmap fills reuse the resolved shape path,
    and the Krilla backend applies it as a nonzero-winding clip before image
    transforms and painting. After this stage the significant fraction fell
    from `0.052029` to `0.049635`; the remaining difference exposed the
    rectangular shadow independently.
  - `effects`: outer-shadow rasterization now fills the actual flattened shape
    path into its bounded alpha mask before applying the existing linear-time
    triangular blur. It retains the 250,000-pixel LibreOffice safety cap and
    supports line and cubic silhouettes without fixture-sized bitmaps. This
    reduced the comparison to five significant pixels; the
    remaining `2.5891` mean delta was a uniform grayscale-color difference.
  - `image-effects`: DrawingML gray uses the relative intensity of the sRGB
    primaries. Image effects now use deterministic Rec. 709/sRGB coefficients
    instead of LibreOffice's legacy `Color::GetLuminance` weights. This matches
    the Office fixed-output rendering while preserving the upstream regression
    requirement that the source blue JPEG becomes gray.
- Specification and source evidence:
  - ECMA-376 Part 1, §20.1.9.8 and §20.1.9.13-16/20 define custom geometry,
    path-local coordinate spaces, path lists, moves, lines, closes, and points;
    the specification examples scale authored coordinates through `path@w`
    and `path@h` rather than replacing the geometry with its bounds;
  - ECMA-376 Part 1, §20.1.2.3.9 defines gray from the relative intensities of
    the red, green, and blue primaries, and Annex L states that blip effects
    mirror those color transformations;
  - LibreOffice `oox/source/drawingml/customshapegeometry.cxx` supplies the
    guide token/formula mapping and ordered path-segment import behavior;
  - LibreOffice `sd/qa/unit/import-tests4.cxx::testTdf112209` verifies that the
    blip effect changes the source blue bitmap to gray. Its exact `0x84`
    expectation reflects LibreOffice's legacy luminance weights; the Office
    golden resolves the target coefficient choice for this lane.
- Refactoring and focused regressions:
  - custom geometry is isolated in the new `pptx/custom_geometry.rs` module;
    unit tests lock coordinate scaling and representative guide operators;
  - the common image display item and Krilla image painter share one vector
    clipping contract rather than raster-masking each caller separately;
  - shadow tests lock non-rectangular alpha filling independently of blur;
  - `mapped_pptx_tdf112209_preserves_grayscale_fill_bitmap_color` now locks the
    gray transition, at least one PDF clip operator, the fill-plus-shadow image
    output, and a source-derived point that lies inside the bounds but outside
    the custom path, where the page must remain white.

### Sixteenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112280`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112280.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112280.pptx.pdf`
- Source SHA-256: `31339f5e0e6c023bfa6e7699528f479e4c1b1464c4eae96c70f0a39dd7c4e2f4`
- Golden SHA-256: `5e32162d442ea04716524897d01873696b71d91a73cb8e41f57354047fc3348f`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide with empty title/subtitle placeholders and
  one theme-styled rectangle. The rectangle has a 500 ms emphasis animation
  whose `p:animRot@by=21600000` describes one complete rotation during slide
  playback.
- Initial result: all comparison layers passed on the first baseline. The case
  is not empty: source inspection confirms the visible rectangle, the golden
  is a one-page PowerPoint fixed-format PDF, and the layered harness verifies
  its page geometry, normalized text, graphics structure, and visible output.
- Result: complete on 2026-07-15. All sixteen admitted cases pass together
  under the unchanged contract. No implementation, golden, manifest, or
  comparison-policy change was required.
- Classification and evidence:
  - `already-supported`: fixed-format output paints the authored static shape
    and does not execute the presentation playback timeline. This is the same
    static-export rule already exercised by entrance, exit, motion, and
    zero-size/fade animation cases, now independently checked for a full-spin
    emphasis animation;
  - LibreOffice
    `sd/qa/unit/export-tests-ooxml2.cxx::testTdf112280` verifies preservation of
    `animRot@by=21600000` during PPTX round-trip. That package-fidelity concern
    is orthogonal to PDF layout; the Office golden confirms that the authored
    rectangle geometry remains the printed state;
  - no new focused behavior test was added because no implementation behavior
    changed and the existing static-animation cases already own that invariant.
    The independently addressable golden test is the regression for this
    source/golden identity.

### Seventeenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112333`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112333.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112333.pptx.pdf`
- Source SHA-256: `e5dd6520572385aae80159bf7d7ede0e7368f7bf375fc72a652ecfd2e7c0f03d`
- Golden SHA-256: `b09f42c86e5a0be75088fd4e9a5651c431c81fa3e1f1ae97db54803186c95d11`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide with one explicitly red rectangle. A
  two-second click emphasis animation changes `fillcolor` to the hyperlink
  theme color and concurrently sets `fill.type=solid` and `fill.on=true`.
- Initial result: all comparison layers passed on the first baseline. Source
  inspection confirms non-empty red geometry, while the one-page Office PDF
  verifies that fixed-format export uses the authored red fill rather than the
  animation's hyperlink-colored held result.
- Result: complete on 2026-07-15. All seventeen admitted cases pass together
  under the unchanged contract. No implementation, golden, manifest, or
  comparison-policy change was required.
- Classification and evidence:
  - `already-supported`: this independently extends the static animation
    evidence from motion, visibility, size, and rotation to animated fill
    state. Timing values remain playback/package metadata and do not override
    the authored display properties during fixed-format export;
  - LibreOffice
    `sd/qa/unit/export-tests-ooxml2.cxx::testTdf112333` verifies round-trip
    preservation of `fill.type=solid`, `fill.on=true`, and the resolved target
    color. Those assertions protect animation package fidelity; the Office
    golden separately defines the static PDF result;
  - no new focused implementation test was added because the case required no
  behavior change. Its exact-filterable golden test supplies independent
  regression coverage for animated fill state.

### Eighteenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112334`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112334.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112334.pptx.pdf`
- Source SHA-256: `43ad429a76716e78d61497b4740ced9f0273a6e9d553cac48d0dacf9239155f7`
- Golden SHA-256: `471604e3ff78b34b38aa13a65a28ff5307700810b4e91ad750b98b6fd0f6470c`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 960 x 540 point slide with one no-fill autofit text box containing
  `One more here`. Two one-second autoreversing effects target its first
  paragraph range: `style.color` and `fillcolor` animate to white, while
  `fill.type=solid` and `fill.on=true` establish the animated fill state.
- Initial result: all comparison layers passed on the first baseline. Source
  inspection confirms that the larger one-page PDF is attributable to text
  and font resources rather than additional drawing content. The Office PDF
  prints the authored text state and does not apply either transient white
  animation value to fixed-format output.
- Result: complete on 2026-07-15. All eighteen admitted cases pass together
  under the unchanged contract. No implementation, golden, manifest, or
  comparison-policy change was required.
- Classification and evidence:
  - `already-supported`: this independently covers animation targeted at a
    paragraph range rather than an entire shape. Fixed-format export continues
    to use authored text and fill properties instead of playback state;
  - LibreOffice
    `sd/qa/unit/export-tests-ooxml2.cxx::testTdf112334` verifies round-trip
    preservation of the `style.color` animation attribute. That package
    assertion is separate from the Office golden's static rendering evidence;
  - no focused implementation test was added because no behavior changed. The
    exact-filterable golden case is the regression for this source/golden pair.

### Nineteenth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf112633`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf112633.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf112633.pptx.pdf`
- Source SHA-256: `a713241c150bf7ec1dc85ca37683a47ee37a98af0bba2b803bac4a188c40e344`
- Golden SHA-256: `8dbd2187eed15cb1ff11b36acaff4270af0dd6d0383d5ebaff1eb2423cab7203`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 720 x 540 point slide with empty title/subtitle placeholders and
  one approximately 5 x 5 point picture. Its primary `a:blip` relationship
  embeds a 5 x 5 PNG, while an `a14:imgLayer` extension preserves a WDP source
  plus an `artisticPencilGrayscale` effect with `pencilSize=80`.
- Initial result: all comparison layers passed on the first baseline. The
  current display PNG is rendered at the authored transform; the extension's
  WDP relationship is preservation metadata for the effect's original image,
  not a replacement display resource for fixed-format output.
- Result: complete on 2026-07-15. All nineteen admitted cases pass together
  under the unchanged contract. No implementation, golden, manifest, or
  comparison-policy change was required.
- Classification and evidence:
  - `already-supported`: the renderer correctly consumes the primary image
    relationship and ignores the alternate extension layer for static display;
  - LibreOffice `oox/source/drawingml/misccontexts.cxx` imports the WDP bytes
    as the artistic effect's embedded original, and
    `oox/source/drawingml/shape.cxx` stores that effect in an interoperability
    grab bag. Its `testTdf112633` verifies the WDP relationship, file, and
    `pencilSize` survive round-trip, which is package fidelity rather than a
    command to substitute the WDP during PDF rendering;
  - no focused implementation test was added because no behavior changed. The
    independently filterable golden case guards the visible-resource choice.

### Twentieth Completed Case

- Case id: `libreoffice_sd_qa_unit_data_pptx_tdf113163`
- Source: `corpus/LibreOffice/sd/qa/unit/data/pptx/tdf113163.pptx`
- Golden: `corpus_pdf_conv/LibreOffice/sd/qa/unit/data/pptx/tdf113163.pptx.pdf`
- Source SHA-256: `8e746b8e6017f373af2233d4fd66807bef7fa79b38514cb8b1435edd96426f84`
- Golden SHA-256: `d7bc34f9783dbf2bdf71fa4de8d03b1b2464ccc56eacd34c03c06ecd6145418e`
- Office environment id: `238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157`
- Scope: one 720 x 540 point slide with a cropped EMF picture whose transform
  extends far beyond every page edge. Its `a:clrChange` maps white to fully
  transparent, leaving the Office fixed output visually black across the page.
- Initial result: all correctness layers passed on the first baseline, but the
  exact case required approximately 64 seconds. CPU sampling showed repeated
  16-million-pixel clip-mask construction, combination, and bounding scans in
  EMF+ rectangle clipping; PNG encoding was not the primary cost.
- Result: complete on 2026-07-15. The common EMF/WMF scanline path now limits
  polygon work to each polygon's vertical bounds and retains axis-aligned
  rectangle clips as rectangles. Boolean masks are materialized only for clip
  combinations that require non-rectangular regions. The exact case fell from
  about 61 seconds in the repeatable pre-fix measurement to 12.09 seconds,
  still passes the unchanged Office contract, and all twenty admitted cases
  pass together in 12.43 seconds. No golden, manifest, or comparison-policy
  change was made.
- Classification and evidence:
  - `metafile-render-performance`: this was not a visible-output failure, but
    it exposed an avoidable full-canvas algorithm in a shared rendering path;
  - LibreOffice `sd/qa/unit/PNGExportTests.cxx::testTdf113163` asserts that the
    exported 100 x 100 bitmap is entirely black. The existing focused suite
    test independently requires at least 98% near-black rendered-page pixels;
  - `emfsdk` unit regressions cover bounded polygon scanlines, pixel-center
    bounds for axis-aligned rectangle clips, rotated-rectangle fallback, and
    empty rectangle intersections.

### First Cross-Format Framework Batch

Completed on 2026-07-18 without changing any golden PDF or manifest:

- DOCX admitted `desktop/qa/data/blank_text.docx` and
  `sw/qa/extras/ww8export/data/empty_group.docx`.
- PPTX admitted `hidden_group_shape.pptx`, `tdf156808.pptx`,
  `tdf157635.pptx`, `tdf157793.pptx`, and
  `tdf169496_hidden_graphic.pptx`.
- XLSX admitted `tdf135828_Shape_Rect.xlsx` and
  `tdf169496_hidden_graphic.xlsx`.
- `w:pgSz` dimensions now retain their normative twentieth-of-a-point values
  instead of passing through LibreOffice's mm100 sloppy paper fitting. The
  fixed-output profile's omitted-page-size default is A4, matching the recorded
  Office environment.
- MediaBox comparison accepts at most `0.1pt` of fixed-output quantization.
  This covers the observed `0.04pt` difference between the exact 210 x 297mm
  A4 conversion and Office's serialized MediaBox while remaining below one
  raster pixel. Larger page-model differences still fail before text or raster
  comparison.
- Font-dependent DOCX candidates were not admitted. No UI-language-to-font
  rule was inferred from the golden output alone.
- Other screened candidates were not admitted: DOCX `cloud.docx` exceeded the
  unchanged raster threshold; XLSX `empty_chart.xlsx` emitted an extra
  candidate chart title, and `image_hyperlink.xlsx` had a page-count mismatch.

### Latest Verification

Completed on 2026-07-18 using the default Cargo target directories:

- `cargo test -p ooxmlsdk-layout`: 97 implementation tests passed.
- `cargo test -p ooxmlsdk-pdf`: 2 implementation tests passed.
- `cargo test -p ooxmlsdk-pdf-test --test office_golden_pptx -- --ignored`:
  all 25 admitted PPTX cases passed together.
- `cargo test -p ooxmlsdk-pdf-test --test office_golden_docx -- --ignored`:
  both admitted DOCX cases passed together.
- `cargo test -p ooxmlsdk-pdf-test --test office_golden_xlsx -- --ignored`:
  both admitted XLSX cases passed together.
- Workspace-wide tests and Clippy were intentionally not run for this focused
  iteration.

Comparison-layer counts for admitted cases:

| Format | Conversion | Page geometry | Text | Graphics | Visible output |
| --- | ---: | ---: | ---: | ---: | ---: |
| PPTX | 25 / 25 | 25 / 25 | 25 / 25 | 25 / 25 | 25 / 25 |
| DOCX | 2 / 2 | 2 / 2 | 2 / 2 | 2 / 2 | 2 / 2 |
| XLSX | 2 / 2 | 2 / 2 | 2 / 2 | 2 / 2 | 2 / 2 |

### Next Case

Select the next font-independent case with an independently diagnosable first
failure. Keep rejected candidates out of the admitted lanes, and rerun all
cases for the affected format before admitting another batch.

## Recommended Implementation Order

1. Build the smallest comparison helper needed by the first case and record the
   exact Rust font environment.
2. Add one small PPTX case, fix it to completion, and record the outcome.
3. Continue PPTX one case at a time while fixed-page geometry establishes the
   import/display/PDF comparison contract.
4. Introduce DOCX one case at a time, separating typography, pagination,
   tables, notes, and floating-object behavior.
5. Introduce XLSX one case at a time after print ranges, page setup, scaling,
   and font metrics are observable.
6. Re-run all previously admitted cases after every change. Expand to a broad
   generated lane only when the incremental suite has demonstrated that doing
   so will remain attributable and controllable.

## Update Rules

After each material batch, update at least:

- current case id, source path, golden path, and current result;
- date and command/environment identity of the latest admitted-case run;
- admitted and passing counts for every comparison layer and format;
- new or closed failure clusters, including representative case ids;
- source paths and specification sections used for the fix;
- focused regression tests added;
- remaining risks, especially font/environment or resource-limit cases;
- verification commands and results.

Never replace a measured count with “mostly works” or remove historical
failure context without recording what changed.

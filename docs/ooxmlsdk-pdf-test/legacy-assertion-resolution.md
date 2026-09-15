# Office alignment of the 12 layout and 5 public font failures

The prior and updated dependency/source snapshots reproduce the same 17 failures.
They were resolved on 2026-09-12. Golden thresholds, reference PDFs and corpus
sources were preserved. This cohort must not be reported as an open legacy
exception again.

## Decisions and retained coverage

| Original failure(s) | Resolution and evidence |
| --- | --- |
| Hidden character-style whitespace / STYLEREF | **Implementation fix.** `Iterator::all` on an empty run made the variation-selector pass delete a hidden range separator. Require nonempty selector text; retain the original three-run and candidate assertions. Existing emoji-selector controls continue to apply. |
| Local effect source independent of page translation | **Implementation fix.** Allocate the one-pixel guard directly in device coordinates. A page-space `1/scale` subtraction followed by f32 scaling changed sample coverage under translation. Preserve strict image equality, extend it to four translations and the requested fill plane. |
| Named-family fixed pitch, charset and Symbol requests (3) | Explicit installed family names win over substitution hints. Retain both named and unnamed requests so pitch/charset still constrain replacement faces. Sources: ECMA-376 Part 1 §§17.8.2–17.8.3; [MS-DOC FFN](../../references/references/[MS-DOC]-260217.md#ffn); Office neutral/conflicting Arial controls below. |
| Symbol request without family rejects normal text | Make the negative synthetic fixture explicitly ANSI. Missing charset metadata is unknown; keep that eligibility control as well as known-ANSI rejection. This fixes the fixture without changing fallback policy. |
| Character tracking on AA | Include tracking after both characters in measured run width. Check -2/0/+2pt and each glyph advance. Source: ECMA-376 §17.3.2.35 and centered Office controls below. |
| VML image host paint order | Inspect the single inline group, then assert fill → image → stroke, colors and clipping. Grouping keeps the three paint layers together during baseline/alignment movement; layer ownership and painting expectations remain strict. |
| Continuous area coverage and half-pixel contour phase (2) | Source glyph fills retain continuous byte coverage; widened strokes use WPF/MIL 8×8 coverage. Assert the legal stroke lattice, many partial coverage values, 0/255 coverage, and half coverage 128. Sources: WPF `core/geometry/bezier.cpp`, `core/sw/aacoverage.h`, `core/sw/swlib/aarasterizer.cpp::ScalePPAACoverage_32bppPBGRA_Out_Slow`. |
| Projected inset contour collapse | Replace the obsolete parabolic inset expectation with the captured six-section circular tube and its round cap. Assert depth values, bounds, and front/back vertices even where opposite inner edges overlap. |
| Taller angle bevel exposes less contour RGB | Physical tubes retain full contribution below the occluding bevel depth; assert full = shallow > tall, with a nonempty contour. The old assertion incorrectly required attenuation for every nonzero bevel. |
| Exact raster mapping and sample policy | Keep axis scales, origin and invalid-mapping assertions. GDI+ 8×4 samples around integer centers, giving half-covered integer edges and quarter-covered corners, including the far edge. Other modes retain their original expectations. |
| Extreme auto spacing and mixed/translucent baselines (2) | Keep proportional excess below the text baseline while preserving the complete flow advance. Assert inherited 115% for opaque/mixed/translucent text and 132× extreme spacing. Office controls below replace the obsolete default-115%-box and opacity-counterexample assumptions. |
| WordArt outline color and run-property override (2) | Change only blue 0x35 → 0x34 for theme C0504D + lumMod 75000. Keep opacity, alpha, width, cap, dash, join and override assertions. Office opaque and transparent controls confirm the RGB below. |

## Controlled Office observations

Archive: `/tmp/ooxmlsdk-repair-resume-20260912/legacy-test-cleanup/`.
Eighteen Word exports used the unchanged reference adapter, Word build
16.0.20326, en-US UI, zh-CN format/authoring locale, Asia/Shanghai, print quality,
all pages, tagged PDF, no bookmarks/PDF-A/document properties. Receipts validate
source/PDF hashes and options; Office language was restored, host settings were
unchanged, and no Office processes remained.

The ten `office-controls/` cases were derived in temporary copies of
LibreOffice `sw/qa/writerfilter/dmapper/data/tdf152884_Char_Transparency.docx`:

- Theme accent2 C0504D, lumMod 75000, with alpha 0 or 20000: PDF stroke operator
  `0.584 0.216 0.204 RG`, corresponding to RGB **149,55,52**. Direct base RGB
  C0504D plus the same transform agrees. Direct transformed RGB controls are
  retained separately; they are not assumed equivalent to theme resolution.
- Installed Arial with ordinary versus conflicting Symbol/fixed/modern font-table
  hints: identical full bbox text and identical page pixels at 144 DPI.
- Centered Arial AA with 0/+2/-2pt tracking: first text X is
  **291.05 / 289.01 / 292.97pt**. Printer quantization accounts for the small
  residual from ±2pt; the movement distinguishes two tracked advances from an
  inter-glyph-only width change of ±1pt.

The eight `baseline-controls/` copies isolate direct/inherited spacing and
opaque/mixed/translucent paint. At 115%, direct and inherited ordinary text share
baseline **795.6pt**; mixed and translucent cases retain it. Extreme 132× cases
use **795.48pt**, within one 600-DPI printer dot. The ordinary 1× opaque control
uses a slightly different PDF font quantization (10.875 vs 11.04pt); do not treat
all control coordinates as bit-identical or derive a universal font metric
constant from them. Validation: `office-controls-validation.json` and
`baseline-controls-validation.json`.

Retained contour evidence predates this cleanup: `/tmp/31166-office-contour-sections/report.json`
measures the actual Office mesh, with a horizontal E section at radial/depth
(±0.5,0), (±0.25,±0.4330126)pt; its previous parabolic model predicted 0.55pt.
`/tmp/31166-office-contour-join/verified.json` verifies source/options, observer,
image identity and join branches. Hashes and selected measurements are copied
into `retained-mesh-sources.json`. The round cap and full section are represented
in `drawingml_3d/contour_mesh.rs`; these are geometry observations, not new claims
of a complete causal image replay.

## Verification

SDK libraries: **1,624 layout + 50 font tests passed**. Public `font_backend`:
**80 passed**. The final configured audit and accepted checkpoint are tracked in
[the campaign guide](corpus_pdf_conv.md); these test passes alone do not promote
a golden baseline. Clippy remains waived by the user.

The separate three-failure public XLSX cohort is covered in
[its own assertion decisions](xlsx-legacy-assertion-resolution.md).

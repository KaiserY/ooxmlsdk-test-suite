# Office OOXML PDF Campaign: First-Round Review

Campaign: `office-ooxml-pdf-options-v1`  
Plan: 5,370 sources, exactly one deterministic configuration per source  
Office environment: `26a412e53b5135051ef70c29ccb7c7993a64b96fefdd230a0a19e782f52b521a`

## Audit Result

| Family | Assigned | Golden | Reference fail | Candidate PASS | Candidate FAIL |
| --- | ---: | ---: | ---: | ---: | ---: |
| Word | 3,045 | 2,972 | 73 | 1,369 | 1,603 |
| Excel | 1,345 | 1,215 | 130 | 146 | 1,069 |
| PowerPoint | 980 | 957 | 23 | 375 | 582 |
| **Total** | **5,370** | **5,144** | **226** | **1,890** | **3,254** |

All 5,370 records passed the no-op replay (`skipped=5370`, `attempts=0`).
The full candidate audit has zero infrastructure errors. Ordinary failures are
kept as development evidence: conversion 1,306, font 5, page geometry 89,
text 1,598, and visible output 256.

## Small Confirmation List

1. **Keep the 226 reference failures terminal in v1 (recommended).** They are
   91 Excel zero-page exports, 131 Office conversion/open failures, and four
   two-attempt timeouts. None produced a golden. Producing a PDF for these
   sources should use a v2 campaign assignment rather than mutating v1.
2. **Treat PDF/UA eligibility as a later bounded lane (recommended).** Of the
   1,306 candidate-conversion failures, 1,185 mention a missing document
   outline, 594 a missing title, 90 missing annotation alternative text, 37
   archival image interpolation, and 22 the tagged-form API; categories
   overlap. These are ordinary capability gaps, not damaged Office goldens.
3. **Leave unavailable fonts recorded, without guessed substitution
   (recommended).** The scan found 672 referenced names: 191 exact installed,
   36 available through Office cloud fonts, and 445 unavailable. No exact local
   install candidates remain; most unavailable names are proprietary,
   historical, malformed, or test-only.

The four terminal timeout cases are:

- `LibreOffice/chart2/qa/extras/data/xlsx/tdf81396.xlsx`
- `Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/spreadsheet/FilterByCF.xlsx`
- `LibreOffice/sw/qa/extras/ooxmlexport/data/sdt-before-field.docx`
- `LibreOffice/sw/qa/extras/ooxmlimport/data/n751017.docx`

## Representative Visual Checks

- Excel `ClosedXML.Tests/Resource/Examples/Ranges/DeletingRanges.xlsx`: the
  candidate omits the Office green range block; the visible FAIL is real.
- PowerPoint `sd/qa/unit/data/pptx/tdf144092-emptyShapeTextProps.pptx`: the
  lower rectangle and table band dimensions/edges differ visibly.
- Word `oox/qa/unit/data/WPC_ThemeColor.docx`: the heart geometry matches
  closely, while the Office page background is gray and the candidate is
  white.

Artifacts are under `target/office-golden/<configuration-id>/`; the complete
report and summary are under `target/office-pdf-campaign/full-audit*`.

## Major Issue Fixed During Audit

Seven cases initially panicked while serializing tagged image/shape links:
Krilla registered the annotation but the page tag tree omitted it when the
graphic had no alternative text. The PDF backend now attaches every such link
annotation to a `Link` structure node. The seven exact cases became two PASS
and five normal layered FAIL, and the complete rerun finished with zero
infrastructure errors.

One failed Excel COM call also left its exact worker-owned `/automation`
process alive. The PID was proven against the campaign sidecar before it was
stopped; the supervisor now stops a sidecar-owned Office process after any
unsuccessful worker exit as well as after a timeout.

The integrity replay originally refreshed only `environment.json`'s
observation timestamp even when the environment ID was unchanged. The first
promoted environment snapshot is now immutable: later probes validate its ID
without replacing it. Its SHA-256 remained
`ea513706a7e09360852686d40e796616b4294ce8a79f1c0c9ecae5406ee9f9ae`
across the final 5,370-record no-op replay.

No corpus or golden file was deleted. Office worker staging and focused audit
artifacts remain in `/tmp` or `target/` for inspection.

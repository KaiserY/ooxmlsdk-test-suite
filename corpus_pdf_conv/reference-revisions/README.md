# Reviewed reference revisions

The original `../environment.json` describes the original campaign batch. It is
not rewritten when a reviewed individual reference is refreshed with another
Office installation.

Additional captures live in `../environments/<environment_id>.json`. Each must
include `approved_references` entries binding the exact configuration ID, source
SHA-256 and new PDF SHA-256. Audit rejects unknown environments or an unapproved
identity. The conversion manifest retains the complete original requested,
effective and Office options, while recording the actual new build, environment,
PDF identity and conversion receipt. The bulk converter still refuses accidental
environment mixing. No visual acceptance thresholds are changed.

## 2026-09-08 — b77a9232 / tdf134784.docx

The user approved individual refreshes after evaluation of genuine conflicts.
Same-source/full-options controls with the archived and current adapters, under
both independent and shared Office processes, consistently exported a Gray JPEG
with interpolation. The old reference used raw RGB without interpolation.
Two independent tiny Gray JPEG documents confirmed the new direct-image behavior;
two 64×64 Gray JPEG documents remained pixel-identical to their old references.

Only b77a9232 is refreshed. Original source and options are unchanged. Other
references remain intact. This allows removal of a temporary tiny-Gray raw-RGB
compatibility branch instead of pretending it is current Office behavior.

Build changed from 16.0.20228.20190 to 16.0.20326.20132. The font fingerprint also
changed; the exact internal cause is not attributed to the build number alone.

The case directory contains `review.json` (old and revised full conversion
records), `receipt.json`, the recoverable `previous.pdf`, and `old-vs-new.png`
(old Office / new Office / RGB absolute difference ×4; same 144-DPI page grid,
same crop and nearest-neighbor zoom). The new reference is at its normal corpus
PDF path. The original environment and historical PASS-union records remain
unchanged; a reference revision is not counted as a new implementation PASS.

## 2026-09-09 — db20249b / tdf158512.pptx

A concrete conflict appeared while isolating the house picture's edge noise.
Twelve same-options PNG controls selected JPEG, including the original pixels.
Re-exporting the unchanged package with both the archived and current adapters
in independent Office processes produces identical native JPEG bytes and page
pixels. The old reference used Flate RGB for this picture; current Office uses
JPEG with color interpolation. Dimensions, background RGB and both alpha planes
are unchanged byte-for-byte; masks remain non-interpolated with black Matte.

The user's authorization permits an individual refresh after this verification.
Only this PDF, its conversion receipt and its exact approved environment binding
are revised. The complete environment was recaptured; its fingerprint matches
the existing `8d0112e7` capture. The build change alone is not proven causal.
The case directory preserves `previous.pdf`, `review.json`, both adapter
receipts, `environment-capture.json`, `verification.json`, and `old-vs-new.png`.
The comparison uses the same calibrated PDFium 1333×750 RGB grid and crop.

The unchanged candidate already passes against this reviewed reference, so its
PASS is recorded as a reference revision, not an implementation gain. A separate
PowerPoint Screen fix preserves selected JPEG color and independent SMask directly,
removing the old decode-to-RGB workaround; that repair has its own verification.

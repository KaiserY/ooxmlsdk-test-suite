# Reviewed Office reference revisions

Keep the original campaign environment and requested configuration unchanged.
An individually approved revision records its actual Office build and complete
conversion receipt. Its additional environment must authorize the exact
configuration ID, source SHA-256 and PDF SHA-256. The conversion also carries a
matching `reference-revision` annotation. An unknown, unscoped or mismatched
environment remains rejected; bulk conversion and recovery restrictions remain.
Numerical and semantic golden acceptance gates are unchanged.

Frozen PowerShell/C# export adapters are retained once per environment under
`corpus_pdf_conv/reference-adapters/<environment_id>/`. Revision folders use
`adapter-reference.json` when sharing that snapshot. Only the scripts and their
actual C# dependencies are retained; unused experiment attachments live under
`/tmp/`. Registered exporter/controller/environment script hashes stay unchanged.

| Date | Configured source | Previous Office | Revised Office | Result |
| --- | --- | --- | --- | --- |
| 2026-10-15 | `LibreOffice/sw/qa/extras/docbookexport/data/tdf91095.docx` | `16.0.20326.20144`,9pages | `16.0.20430.20146`,8pages | Approved then; superseded below |
| 2026-10-29 | Same source/configuration | Approved8-page revision | Restored original `16.0.20326.20144`,9pages | PASS; print-update conflict resolved |

On October15 the user approved this single revision after15 controlled exports, including the
exact original bytes and unchanged configured options, consistently reproduced
the8-page output. SDK and current Office have matching page-wise painted text;
maximum significant pixel fraction is0.47% on the1333px/flags17 diagnostic grid.
The build difference is recorded without attributing the behavior to it alone.
This is a reference revision, not an implementation gain.

Configuration ID:
`6db82d9aa0729584fac83fc85b9def831c9c894dc71e0939de9b210798ef3f43`.
Under `corpus_pdf_conv/reference-revisions/<configuration_id>/`, `previous.pdf`
and `review.json` preserve the old PDF and both complete conversion records;
`receipt.json` and `environment-capture.json` retain the observed new export.
The exact approval lives in
`corpus_pdf_conv/environments/2d18b7c4f973d2fe32675fdf7be260c898854c1b24af7e39cdafe5862eb6c7c6.json`.
Detailed diagnostics: `/tmp/ooxmlsdk-tdf91095-20261015/verification-current.json`.

Earlier single-reference methodology is archived under
`/tmp/office-golden-rebuild-20260911/previous-metadata/reference-revisions/`.

## Print-option resolution, 2026-10-29

The October15 exports did not record `Options.UpdateFieldsAtPrint`, so they did
not establish an identical application print-update profile. Controlled current
Word16.0.20527.20088 output gives8pages with the option OFF,9with ON, and8with a
locked TOC. ON reproduces the saved original9-page golden pixel-for-pixel on
all9pages. An Office-version cause was not established.

The user requested support for this option and resolution of the conflict.
The original9-page PDF and original conversion record were restored verbatim in
value; source bytes, configuration identity and acceptance gates are unchanged.
No document re-export was needed for this restoration. The superseded8-page PDF,
its receipt, approval and evidence remain in
`corpus_pdf_conv/reference-revisions/<configuration_id>/restoration-20261029/`.
The October15 environment registration remains as historical provenance.

`PdfOptions` and `LayoutOptions` now expose `update_fields_at_print`:
`Some(false)` preserves optional body-field caches; `Some(true)` enables their
print update; `None` retains legacy behavior. Automatic pagination/display fields
and headers/footers have separate update rules. The option does not replace
`w:updateFields` on-open or supply a date/time clock. Office workers apply,
observe and restore it; observed flags accompany new conversion receipts.

v880: canonical9-page PASS; explicit OFF8/ON9 controls both pass the unchanged
full comparison.11selected tests and2,681protected DOCX pass; no new full corpus
baseline or bulk random-option regeneration is claimed.
Evidence: `/tmp/ooxmlsdk-word-print-field-update-20261029/verification-current.json`.

## Terminal floating anchors, 2026-10-30

`LibreOffice/sw/qa/extras/ooxmlexport/data/tdf153613_textboxAfterPgBreak3.docx`
was individually approved for revision from two pages (Word16.0.20326.20144) to
one (16.0.20527.20088). Exact original bytes, configured options and languages
were retained. Both print-field/link-update states produce one page, and SDK
passes the unchanged complete comparison against each; eight changed-source
controls independently confirm the anchor ownership. The observed build change
is recorded without assigning it sole causality. This is not an implementation gain.

Configuration `082731798f644a73fc3c429b69fbaf53a28ced24efab55b196654f42f987e273`:
its `corpus_pdf_conv/reference-revisions/<configuration_id>/` preserves the old
PDF, both full conversion records, native module hashes, print-option receipts
and both original-source exports. The exact three hashes are approved only in
`corpus_pdf_conv/environments/09707cbf51b06e89c88ce0e25c3d99a568d9fff50bc8d66ff924c6ca3e4241c8.json`.
Original campaign plan/environment and acceptance gates remain unchanged.
Evidence: `/tmp/ooxmlsdk-tdf146984-anchorInShape-20261030/textboxAfterPgBreak3/reference-refresh/`.

## Run-break terminal mark, 2026-10-31

`LibreOffice/sw/qa/extras/ooxmlexport/data/tdf153964_topMarginAfterBreak14.docx`
now records six pages from Word16.0.20527.20088, preserving the seven-page
Word16.0.20326.20144 reference. After implementation repairs,74changed-source
controls retained every prior PASS and70passed completely; all4full-context
split=false controls passed. Two exact-original exports, with configured PDF
options/languages unchanged and observed print-field update ON/OFF, both have
six pages and both pass the unchanged complete SDK comparison. The saved
reference has an additional terminal mark. The build difference is recorded
without assigning it sole causality.

This follows the user's standing instruction to apply the established single-case
revision procedure to confirmed conflicts and preserve history. It is a reference
revision, not an extra implementation gain. Source bytes, configuration identity,
original campaign plan/environment and acceptance gates are unchanged.

Configuration `536b9740b24694698dbac73d377154e3a16c5bbc78e96f32bcbd38a07d06f087`:
`corpus_pdf_conv/reference-revisions/<configuration_id>/` retains the original
PDF, complete old/new conversion records, current native modules, both original
exports and observed print-option receipts. Environment
`a72ce9c43f7e078c1acb4dc72b3d834c5fc4ea76b924df71c29fa249a3c0a3de`
authorizes only this exact configuration/source/PDF hash triple. Print-link
observations stay in the native receipts; the strict conversion schema retains
its supported fields. Evidence:
`/tmp/ooxmlsdk-tdf153964-topMarginAfterBreak14-20261031/reference-refresh/`.

## Modern run-break before-space, 2026-10-31

`LibreOffice/sw/qa/extras/ooxmlexport/data/tdf153964_topMarginAfterBreak15.docx`
now records five pages from Word16.0.20527.20088; the seven-page
Word16.0.20326.20144 PDF and complete conversion record are preserved.
The SDK first repaired physical-body-top before-space ownership while retaining
lower continuous-column spacing.48changed-source controls improve34→48complete
PASS without losing a prior PASS. Both exact-original exports, unchanged bytes,
assigned options/languages and observed print-field update ON/OFF, produce five
pages and pass the unchanged complete comparison. This confirms a specific
reference conflict; it does not establish that the Office build alone caused it.

The user's standing single-case instruction applies. The revision itself is not
an extra implementation gain; source, configuration, campaign plan/environment
and acceptance gates remain unchanged. Configuration
`785631d83d8530cdc87fb32a9236ba95e0165ba667849d5944cbfa71005d845b`
has its old PDF, complete records, both current outputs, print-option receipts,
module hashes and an adapter reference under `corpus_pdf_conv/reference-revisions/<id>/`.
Environment `a72ce9c43f7e078c1acb4dc72b3d834c5fc4ea76b924df71c29fa249a3c0a3de`
appends only this exact configuration/source/PDF triple, preserving its earlier
approval and an archived copy of the previous registration.76private tests,
19previous native guards and638protected cases pass; actual GDB83captures and
dev/release byte agreement are verified. Evidence:
`/tmp/ooxmlsdk-tdf153964-topMarginAfterBreak15-20261031/verification-current.json`.

## Modern Kashida, 2026-10-31

`LibreOffice/sw/qa/extras/ooxmlexport/data/tdf155707.docx` now retains the
Word16.0.20527.20088 reference, with the two-page Word16.0.20326.20144 PDF and
complete original record preserved. Page count stays two; the confirmed conflict
concerns Latin expansion, Arabic connection selection and glyph mappings. Two
exact-original exports, unchanged configured options/languages and observed
print-field update ON/OFF, both pass the unchanged complete SDK comparison.
Implementation repairs were verified before replacing the reference; SDK PDF
bytes remain identical before/after revision. A build-only cause is not claimed.

The user's standing single-case instruction applies. The revision is not an
additional implementation gain. Configuration
`d7b1ffa7a5fc5bd2a36ebadd1b5d7e6f7e92ea0b2e9df4f43b3d403cf67d785b`
retains its old PDF, old/new records, both current exports, native modules,
print-option receipts and an adapter reference under `corpus_pdf_conv/reference-revisions/<id>/`.
Environment `8f5d880a2724c96254ab0872bd8fe54fa05b3c539314c9d156fc4f88b23c273d`
authorizes only the exact configuration/source/PDF triple. Source bytes, original
campaign plan/environment and acceptance gates are unchanged.31private tests,
2,725prior protected Word cases and final dev GDB46line/96extender observations
pass. Canonical v1007PASS is immediately protected; no full audit is claimed.
Evidence: `/tmp/ooxmlsdk-tdf155707-20261031/verification-current.json`.

## Anchored pictures and modern run-break spacing, 2026-10-31

`LibreOffice/sw/qa/extras/ooxmlexport/data/tdf160049_anchorMargin15.docx`
retains six pages with Word16.0.20527.20088; the six-page Word16.0.20326.20144
PDF and full record are preserved. Implementation first repaired independent
picture effect axes and empty-break-master background device coordinates.
58native comparisons improve19→58completePASS with no priorPASS loss;
23private tests,2733prior protectedWord cases and final read-only devGDB30 pass.

The separate conflict concerns before-space after manual page breaks.
32factor controls and two full-context bridges establish suppression in current
Word. Two exact-original exports, unchanged configured options/languages and
observed print-field update ON/OFF with linksOFF, both pass the unchanged complete
SDK comparison. The build difference is recorded without assigning sole causality.
The user's standing single-case revision procedure applies; this is not an extra
implementation gain. Source/configuration/campaign plan/environment and gates
remain unchanged; SDK candidate bytes are identical before/after revision.

Configuration `88b35128ceab158cfef89ad4d3da13f678d258b95db658a9162777757f30685b`
keeps old/new full records, oldPDF, both current outputs, flags, native modules
and an adapter reference under `corpus_pdf_conv/reference-revisions/<id>/`. Environment
`2745802959e232d8be20849cd3bfcf29ef3e2b50f2256ca6c04af917e15cbf35`
authorizes only its exact configuration/source/PDF triple. Evidence:
`/tmp/ooxmlsdk-tdf160049-anchorMargin15-20261031/verification-current.json`.

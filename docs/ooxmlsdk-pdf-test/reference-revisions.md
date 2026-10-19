# Reviewed Office reference revisions

Keep the original campaign environment and requested configuration unchanged.
An individually approved revision records its actual Office build and complete
conversion receipt. Its additional environment must authorize the exact
configuration ID, source SHA-256 and PDF SHA-256. The conversion also carries a
matching `reference-revision` annotation. An unknown, unscoped or mismatched
environment remains rejected; bulk conversion and recovery restrictions remain.
Numerical and semantic golden acceptance gates are unchanged.

| Date | Configured source | Previous Office | Revised Office | Result |
| --- | --- | --- | --- | --- |
| 2026-10-15 | `LibreOffice/sw/qa/extras/docbookexport/data/tdf91095.docx` | `16.0.20326.20144`,9pages | `16.0.20430.20146`,8pages | PASS; SDK PDF unchanged |

The user approved this single revision after15 controlled exports, including the
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

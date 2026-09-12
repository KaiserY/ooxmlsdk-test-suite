# Office recovery adapters

These immutable bundles retain the actual scripts used to retry failed Office
references. Their directory names are captured environment identities. The
original adapter and 5,145 successful references remain unchanged.

The user authorized simpler export configurations for previously failed or
timed-out sources on 2026-09-12, prioritizing printable content. Recovery does not
modify files or manifests under `corpus/`. Workers copy source bytes to Windows
TEMP, open only that copy, disable macros, and close without saving source files.
Content-type/filename mismatches may change the temporary filename extension;
the package bytes are not rewritten. Office's repair mode can repair the opened
copy in memory. That behavior belongs to the reference adapter, not the corpus.

Bundle `896d3a3e77cd3e86922192044a2784d17a253c71a13d3364f48e79cf71053d6b`
uses the same Office build, fonts, printer and host locale as the original
capture. Only adapter fingerprints differ. Its recovery behavior is:

- Word: invoke `Documents.Open` using named COM arguments with `OpenAndRepair`;
  preserve cached ASK/FILLIN results by locking these interactive fields in the
  temporary document. Clock fields still follow the recorded refresh interval.
- Excel: use explicit `Workbooks.Open` arguments with `xlRepairFile`, no link
  updates, read-only mode, and ignored read-only recommendations.
- PowerPoint: use `Open2007` with repair and supply a null COM dispatch pointer
  for `PrintRange` when exporting all slides. Explicit ranges remain explicit.
- Preserve UTF-8 errors and stage journals, verify actual export language and
  clock ordering, and restore temporary Office language preferences after every
  attempt. Windows clock, time zone and other OS settings are not changed.

The retry configuration uses print quality, no PDF/A, no bookmarks or document
properties, and no optional tagging. Excel retains its supported tagging policy.
PowerPoint includes hidden slides. All pages are selected except a recorded
content-preserving range for `FilterByCF.xlsx`: its ten numbers occupy the final
two pages of Excel's 894,447-page print layout. Selecting those pages avoids
exporting hundreds of thousands of preceding empty pages.

`office_pdf_campaign import-recoveries --requests FILE` accepts only completed,
restored workers with nonzero-page, readable PDFs and matching source, output,
configuration and adapter evidence. It refuses to replace an existing converted
reference. Each revision is approved for exact configuration/source/PDF hashes;
old failed assignments and complete successful worker evidence are retained in
`reference-recovery-history/`. `recovery-plan-overrides.jsonl` makes the updated
plan reproducible without editing source corpora. Recovered references create
new configured identities; they do not establish candidate PASS results.

Zero-page outputs remain separate diagnostics. Confirmed absence of printable
content should be covered by a functional empty-output assertion, while failed
exports or malformed print configuration require further diagnosis. Neither
class is counted as a valid PDF reference or fixed by inserting a blank page.

API references: Microsoft Learn
[Word Open](https://learn.microsoft.com/en-us/office/vba/api/word.documents.open),
[Excel Open](https://learn.microsoft.com/en-us/office/vba/api/excel.workbooks.open),
[PowerPoint Open2007](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentations.open2007),
and [PowerPoint fixed-format export](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.exportasfixedformat).
The live Word COM signature was recorded because its optional argument order
differs from the published VBA signature; named invocation avoids that mismatch.

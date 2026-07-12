# Microsoft Office PDF References

This directory contains the primary visible-output references for Office source
documents in the imported corpora. The PDFs are exported by the installed
Windows Microsoft Office applications, not by `ooxmlsdk-pdf`.

Microsoft Office is the reference engine for this directory. LibreOffice may be
used temporarily to investigate a disagreement, but LibreOffice output is not a
second committed reference set.

## Layout

Source-relative directories are preserved under one directory per imported
corpus. The original extension remains before `.pdf`, so `sample.doc.pdf` and
`sample.docx.pdf` cannot collide.

Each corpus owns one sorted JSONL manifest:

- `Apache-POI/manifest.jsonl`
- `LibreOffice/manifest.jsonl`
- `Open-XML-SDK/manifest.jsonl`

`environment.json` describes the minimal rendering environment shared by those
manifests. Every converted record references its `environment_id`.

## Manifest Records

Schema version 2 records file-derived facts first:

- source-relative `file`, extension, byte size, and SHA-256
- conversion status and Office application
- output-relative path, byte size, and SHA-256
- conversion timestamp and elapsed time
- export profile and rendering `environment_id`
- machine-readable `failure_class`, locale-independent COM `error_code`, and the
  original Office error for failed attempts
- structured `annotations` for known source-specific caveats

The converter preserves `annotations` when a file is exported again. Suggested
annotation kinds include `environment-dependent`, `source-repair`,
`office-file-block`, `encrypted`, and `known-rendering-difference`. Keep notes
specific to the source and observable output; do not put chat history or local
machine details in them.

Failed records are useful corpus evidence and remain in the manifest even when
a checked batch later substitutes another source file. Current classes include
`office-file-block`, `office-security-rejected`, `encrypted-or-password-protected`,
`invalid-or-corrupt-source`, `pdf-export-failed`, and a generic
`invalid-pdf-output`, and `office-conversion-error` fallback. The
`invalid-pdf-output` class includes structurally unverifiable and zero-page
Office exports.

## Minimal Environment Fingerprint

Office rendering can change when Office, Windows, fonts, locale, time zone, or
the default paper changes. `environment.json` therefore records only:

- Office bitness and reported build
- Windows build number
- culture, UI culture, and time-zone identifier
- default paper name, dimensions, and orientation
- installed-font count and a one-way aggregate name/size/mtime hash
- generator version and export profile

It does **not** record the machine name, user name, printer name, printer driver,
port, hardware identifiers, installation paths, font filenames, document paths,
or account/licensing information. Office error messages are sanitized before
writing so Windows user-profile paths do not expose the local account name.

The environment ID excludes the observation timestamp. If any fingerprinted
input changes, the ID changes and existing PDFs are regenerated instead of
being silently reused. An unchanged PDF is skipped only when all of these match:

1. manifest schema and environment ID
2. source SHA-256
3. output PDF SHA-256
4. `%PDF-` header and a non-zero Office PDF page-tree count

## Export Policy

The `office-fixed-format-print-no-macros-no-links-v2` profile means:

- sources are copied to a Windows-local temporary directory before Office opens
  them
- inputs are opened read-only and never saved back
- Office macros are force-disabled
- Excel external-link updates and alerts are disabled
- Word uses `Document.ExportAsFixedFormat` with PDF output
- Excel uses `Workbook.ExportAsFixedFormat` with PDF output
- PowerPoint uses its native `ppSaveAsPDF` path
- applications are reused but recycled after a configurable number of files
- a failed export restarts the responsible Office application and retries once
- generated files are checked for a PDF header and at least one page before
  being copied back

The script intentionally requires an audited list file. It never scans all of
`corpus/` implicitly, because the corpora contain encrypted files, malformed
packages, CVE/fuzzer inputs, Office File Block cases, and deliberately invalid
fixtures.

## Known Boundaries

- PDF bytes are reference artifacts, not byte-stable expectations. Tests should
  compare pages, geometry, extracted primitives, or fixed-raster output.
- Missing source paper settings may inherit the recorded default paper.
- Explicit paper settings can still be affected by Excel's active printing
  environment; annotate proven cases in the source manifest.
- Excel used ranges or print areas can legitimately produce thousands of PDF
  pages. Keep those outputs as stress references, but avoid rasterizing every
  page in the default test lane.
- Missing fonts, font updates, fallback order, locale, fields, volatile formulas,
  and cached formula values can change visible output.
- File Block, Protected View, encryption, corruption-repair prompts, and hidden
  modal dialogs are expected failure classes. Do not weaken global Office
  security settings to convert such fixtures.
- Open XML SDK `TestFiles` document-property fixtures and some v2Fx
  relationship/pivot workbooks have triggered hidden Office interactions in
  unattended runs. Keep them out of checked batches until a process-level
  per-file supervisor can terminate and record timed-out conversions.
- Office COM is not a supported server-side automation environment. Run checked
  batches from an interactive, activated Windows user session. The current
  converter records failures and recycles applications, but a future full-corpus
  supervisor should also enforce a process-level per-file timeout.

## Running A Checked Batch

From the test-suite root in WSL:

```bash
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
  -File "$(wslpath -w scripts/convert_office_corpus.ps1)" \
  -CorpusRoot "$(wslpath -w corpus)" \
  -OutputRoot "$(wslpath -w corpus_pdf_conv)" \
  -ListFile "$(wslpath -w scripts/office_corpus_batch_100.txt)"
```

Use `-Force` to re-export every selected file. Without it, the converter applies
the source/output/environment checks above. `-RecycleEvery 25` is the default.

After a run, validate manifest JSON, resolve every converted output path, parse
every PDF, and ensure there are no PDFs without manifest records. The thirty-six
checked 100-file batches and their repeated all-skipped runs are the current
smoke test for this workflow.

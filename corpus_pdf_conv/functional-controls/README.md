# Office functional controls

These complementary Office exports verify that page-range and hidden-slide
options change the intended content. They supplement the main one-configuration-
per-source campaign; they are not additional sources in its 5,370-case count.

Each SHA-256 directory contains the configuration/source/host identity, the raw
Office receipt, its reference PDF, and confirmation that Office preferences were
restored. Source paths point into `corpus/`; source provenance and licenses remain
with the corresponding imported corpus.

The directed cases are defined in
`crates/ooxmlsdk-pdf-test/data/office-functional-coverage.json`. From the suite root:

```sh
python3 scripts/verify_office_pdf_functional_coverage.py --verify-only
```

The verifier checks hashes, actual environment, ordered clock observations,
PDF structure, selected page content and visible/hidden slide membership.
It rejects fixtures whose selected text cannot distinguish returning the first
page instead. Without `--verify-only`, it generates missing controls using the
existing serial Office controller; close Office applications first.

The main references must already exist. Their deterministic configuration IDs
can be selected for conversion and comparison with `--selection full
--configuration-ids PATH`; this selection does not execute the full campaign.

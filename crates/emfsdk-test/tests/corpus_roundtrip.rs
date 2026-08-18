use std::path::{Path, PathBuf};

use emfsdk_test::{
    assert_all_ok, collect_metafiles, corpus_dir, expect_parse_rejected, expects_parse_rejected,
};

#[test]
fn upstream_metafile_corpus_matches_source_expectations() {
    let mut failures = Vec::new();
    let root = corpus_dir("");
    let files = collect_metafiles(&root);
    assert!(!files.is_empty(), "no EMF/WMF corpus files found");
    let mut report = emfsdk_test::RoundtripReport::default();

    for path in files {
        if expects_parse_rejected(&path) {
            if let Err(err) = expect_parse_rejected(&path) {
                failures.push(format!("{}: {err}", relative_path(&path).display()));
            }
        } else {
            match emfsdk_test::roundtrip_metafile(&path) {
                Ok(file_report) => report.add(file_report),
                Err(err) => failures.push(format!("{}: {err}", relative_path(&path).display())),
            }
        }
    }

    assert_all_ok(failures);
    assert_eq!(
        report,
        emfsdk_test::RoundtripReport {
            emf_records: 950_508,
            wmf_records: 22_824,
            emf_plus_records: 14_274,
            compatible_emf_records: 9,
            compatible_wmf_records: 4,
            compatible_emf_plus_records: 2,
            unknown_emf_records: 0,
            unknown_wmf_records: 0,
            unknown_emf_plus_records: 0,
            compatibility_diagnostics: 32_003,
        },
        "metafile typed/compatibility coverage changed"
    );
    eprintln!("metafile corpus round-trip report: {report:?}");
}

fn relative_path(path: &Path) -> PathBuf {
    path.strip_prefix(corpus_dir(""))
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

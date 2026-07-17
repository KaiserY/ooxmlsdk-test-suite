use std::path::{Path, PathBuf};

use emfsdk_test::{assert_all_ok, collect_metafiles, corpus_dir, expect_parse_rejected};

#[test]
fn upstream_metafile_corpus_matches_source_expectations() {
    let mut failures = Vec::new();
    let root = corpus_dir("");
    let files = collect_metafiles(&root);
    assert!(!files.is_empty(), "no EMF/WMF corpus files found");

    for path in files {
        let result = if expects_parse_rejected(&path) {
            expect_parse_rejected(&path)
        } else {
            emfsdk_test::roundtrip_metafile(&path)
        };
        if let Err(err) = result {
            failures.push(format!("{}: {err}", relative_path(&path).display()));
        }
    }

    assert_all_ok(failures);
}

fn expects_parse_rejected(path: &Path) -> bool {
    apache_poi_expects_reject(path)
        || libreoffice_expects_reject(path)
        || relative_path(path)
            .to_str()
            .is_some_and(|path| path.starts_with("libemf2svg/tests/resources/emf-corrupted/"))
}

fn relative_path(path: &Path) -> PathBuf {
    path.strip_prefix(corpus_dir(""))
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn apache_poi_expects_reject(path: &Path) -> bool {
    let relative = relative_path(path);
    matches!(
        relative.to_str(),
        Some(
            "Apache-POI/test-data/slideshow/61338.wmf"
                | "Apache-POI/test-data/slideshow/clusterfuzz-testcase-minimized-6701721724125184.wmf"
                | "Apache-POI/test-data/slideshow/clusterfuzz-testcase-minimized-POIFileHandlerFuzzer-6060921738035200.wmf"
                | "Apache-POI/test-data/slideshow/clusterfuzz-testcase-minimized-POIFileHandlerFuzzer-6466833057382400.emf"
                | "Apache-POI/test-data/slideshow/crash-7b60e9fe792eaaf1bba8be90c2b62f057cfff142.emf"
                | "Apache-POI/test-data/slideshow/VHZ2NYFUYUUJNGLABL26ORTQZA76FJEW.emf"
                | "Apache-POI/test-data/spreadsheet/61294.emf"
        )
    )
}

fn libreoffice_expects_reject(path: &Path) -> bool {
    let relative = relative_path(path);
    let Some(relative) = relative.to_str() else {
        return false;
    };

    relative.contains("/graphicfilter/data/emf/fail/")
        || relative.contains("/graphicfilter/data/wmf/fail/")
        || relative == "LibreOffice/framework/qa/complex/broken_document/test_documents/dbf.dbf.emf"
}

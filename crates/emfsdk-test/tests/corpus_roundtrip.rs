use std::path::{Path, PathBuf};

use emfsdk_test::{assert_all_ok, collect_metafiles, corpus_dir, expect_parse_rejected};

#[derive(Clone, Copy)]
enum Expectation {
    Roundtrip,
    Reject,
    ApachePoi,
    LibreOffice,
}

struct CorpusSet {
    root: &'static str,
    expectation: Expectation,
}

const CORPUS_SETS: &[CorpusSet] = &[
    CorpusSet {
        root: "Apache-POI/test-data",
        expectation: Expectation::ApachePoi,
    },
    CorpusSet {
        root: "LibreOffice",
        expectation: Expectation::LibreOffice,
    },
    CorpusSet {
        root: "libemf2svg/tests/resources/emf",
        expectation: Expectation::Roundtrip,
    },
    CorpusSet {
        root: "libemf2svg/tests/resources/emf-ea",
        expectation: Expectation::Roundtrip,
    },
    CorpusSet {
        root: "libemf2svg/vendor/libuemf",
        expectation: Expectation::Roundtrip,
    },
    CorpusSet {
        root: "libemf2svg/tests/resources/emf-corrupted",
        expectation: Expectation::Reject,
    },
];

#[test]
fn upstream_metafile_corpus_matches_source_expectations() {
    let mut failures = Vec::new();

    for set in CORPUS_SETS {
        let root = corpus_dir(set.root);
        let files = collect_metafiles(&root);
        if files.is_empty() {
            failures.push(format!("{}: no EMF/WMF files found", set.root));
            continue;
        }

        for path in files {
            let result = match set.expectation {
                Expectation::Roundtrip => emfsdk_test::roundtrip_metafile(&path),
                Expectation::Reject => expect_parse_rejected(&path),
                Expectation::ApachePoi if apache_poi_expects_reject(&path) => {
                    expect_parse_rejected(&path)
                }
                Expectation::ApachePoi => emfsdk_test::roundtrip_metafile(&path),
                Expectation::LibreOffice if libreoffice_expects_reject(&path) => {
                    expect_parse_rejected(&path)
                }
                Expectation::LibreOffice => emfsdk_test::roundtrip_metafile(&path),
            };
            if let Err(err) = result {
                failures.push(format!("{}: {err}", relative_path(&path).display()));
            }
        }
    }

    assert_all_ok(failures);
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

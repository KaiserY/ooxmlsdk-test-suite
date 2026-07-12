use std::{collections::BTreeSet, fs, path::Path};

use olecfsdk::{
    cfb::CompoundFile,
    property_set::{PropertySetStream, TypedPropertyValue},
};
use olecfsdk_corpus_test_support::manifest::{ExpectationMode, read_manifest};

#[test]
#[ignore = "OLEPS corpus round-trip runs explicitly"]
fn legacy_office_property_set_streams_round_trip() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);
    let expected_invalid = expected_invalid_files(&corpus);
    let mut observed_invalid = BTreeSet::new();
    let mut checked = 0usize;
    let mut failures = Vec::new();
    let filter = std::env::var("OLEPS_FILTER").ok();
    for path in files {
        if filter
            .as_ref()
            .is_some_and(|filter| !path.to_string_lossy().contains(filter))
        {
            continue;
        }
        let Ok(bytes) = olecfsdk_corpus_test_support::corpus_bytes(&path) else {
            continue;
        };
        let Ok(compound) = CompoundFile::from_bytes(&bytes) else {
            continue;
        };
        for entry in compound.entries().iter().filter(|entry| {
            entry.is_stream()
                && matches!(
                    entry.name.as_str(),
                    "\u{5}SummaryInformation" | "\u{5}DocumentSummaryInformation"
                )
        }) {
            checked += 1;
            let result = (|| {
                let parsed = PropertySetStream::from_bytes(&entry.data)?;
                for property_set in &parsed.property_sets {
                    let code_page = property_set.code_page()?;
                    for property in &property_set.properties {
                        if property.identifier == 0 {
                            let dictionary =
                                property.dictionary(code_page.ok_or_else(|| {
                                    olecfsdk::Error::invalid(
                                        property.offset as u64,
                                        "OLEPS dictionary has no CodePage property",
                                    )
                                })?)?;
                            if dictionary.to_bytes()? != property.raw {
                                return Err(olecfsdk::Error::invalid(
                                    property.offset as u64,
                                    "OLEPS dictionary bytes changed after round-trip",
                                ));
                            }
                            continue;
                        }
                        let typed = property.typed_value()?;
                        if let TypedPropertyValue::Unknown { property_type, .. } = &typed {
                            return Err(olecfsdk::Error::invalid(
                                property.offset as u64,
                                format!(
                                    "unimplemented OLEPS property type 0x{:04x}",
                                    property_type.0
                                ),
                            ));
                        }
                        if typed.to_bytes()? != property.raw {
                            return Err(olecfsdk::Error::invalid(
                                property.offset as u64,
                                "typed OLEPS property bytes changed after round-trip",
                            ));
                        }
                    }
                }
                let saved = parsed.to_bytes()?;
                let reopened = PropertySetStream::from_bytes(&saved)?;
                if parsed != reopened {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "OLEPS structure changed after round-trip",
                    ));
                }
                Ok::<_, olecfsdk::Error>(())
            })();
            if let Err(error) = result {
                if expected_invalid.contains(&path) {
                    observed_invalid.insert(path.clone());
                } else {
                    failures.push(format!("{}:{}: {error}", path.display(), entry.name));
                }
            }
        }
    }
    assert!(checked > 0, "no OLEPS streams found in legacy corpus");
    let missing: Vec<_> = expected_invalid.difference(&observed_invalid).collect();
    assert!(
        missing.is_empty(),
        "OLEPS invalid expectations no longer fail: {missing:?}"
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} OLEPS streams failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn expected_invalid_files(corpus: &Path) -> BTreeSet<std::path::PathBuf> {
    let mut files = BTreeSet::new();
    for name in ["Apache-POI", "LibreOffice"] {
        let root = corpus.join(name);
        let manifest = read_manifest(&root.join("manifest.toml")).expect("read corpus manifest");
        for expectation in manifest.expectation {
            if expectation.test == "oleps_roundtrip" && expectation.mode == ExpectationMode::Invalid
            {
                files.insert(root.join(expectation.file));
            }
        }
    }
    files
}

fn collect(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("read corpus directory") {
        let path = entry.expect("read corpus entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "doc" | "dot" | "xls" | "xlt" | "ppt" | "pps" | "pot"
                )
            })
        {
            files.push(path);
        }
    }
}

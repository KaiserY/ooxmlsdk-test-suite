use std::{collections::BTreeSet, fs, path::Path};

use olecfsdk::{
    cfb::CompoundFile,
    vba::{
        cache::VbaProjectStream,
        compression::CompressedContainer,
        directory::DirStream,
        module::ModuleStream,
        project::{ProjectLkStream, ProjectStream, ProjectWmStream},
    },
};
use olecfsdk_corpus_test_support::manifest::{ExpectationMode, read_manifest};

#[test]
#[ignore = "VBA compression corpus round-trip runs explicitly"]
fn legacy_office_vba_dir_streams_round_trip() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);
    let expected_invalid = expected_invalid_files(&corpus);
    let mut observed_invalid = BTreeSet::new();
    let mut checked = 0usize;
    let mut module_checked = 0usize;
    let mut project_cache_checked = 0usize;
    let mut project_checked = 0usize;
    let mut project_wm_checked = 0usize;
    let mut project_lk_checked = 0usize;
    let mut failures = Vec::new();
    let filter = std::env::var("VBA_FILTER").ok();
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
                && entry.name.eq_ignore_ascii_case("dir")
                && entry.path.components().any(|component| {
                    component
                        .as_os_str()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("VBA")
                })
        }) {
            checked += 1;
            let result = (|| {
                let parsed = CompressedContainer::from_bytes(&entry.data)?;
                let saved = parsed.to_bytes()?;
                if saved != entry.data {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "VBA compressed bytes changed after round-trip",
                    ));
                }
                let reopened = CompressedContainer::from_bytes(&saved)?;
                if parsed != reopened {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "VBA compression structure changed after round-trip",
                    ));
                }
                let decompressed = reopened.decompress()?;
                let directory = DirStream::from_bytes(&decompressed)?;
                let incomplete_ids: BTreeSet<_> = directory
                    .records
                    .iter()
                    .filter_map(|record| match record {
                        olecfsdk::vba::directory::DirRecord::Unknown { id, .. } => Some(*id),
                        _ => None,
                    })
                    .collect();
                if !incomplete_ids.is_empty() {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        format!("incomplete VBA dir record ids: {incomplete_ids:#06x?}"),
                    ));
                }
                let saved_directory = directory.to_bytes()?;
                if saved_directory != decompressed {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "VBA dir record bytes changed after round-trip",
                    ));
                }
                if DirStream::from_bytes(&saved_directory)? != directory {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "VBA dir record structure changed after round-trip",
                    ));
                }
                let parent = entry.path.parent().ok_or_else(|| {
                    olecfsdk::Error::invalid(0, "VBA dir stream has no parent storage")
                })?;
                let project_root = parent.parent().ok_or_else(|| {
                    olecfsdk::Error::invalid(0, "VBA storage has no project root")
                })?;
                let project_entry = compound
                    .entries()
                    .iter()
                    .find(|candidate| {
                        candidate.is_stream()
                            && candidate.path.parent() == Some(project_root)
                            && candidate.name.eq_ignore_ascii_case("PROJECT")
                    })
                    .ok_or_else(|| {
                        olecfsdk::Error::invalid(0, "VBA project has no PROJECT stream")
                    })?;
                let project = ProjectStream::from_bytes(&project_entry.data)?;
                if project.has_unknown_records() {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "PROJECT stream contains an unknown grammar record",
                    ));
                }
                if project.to_bytes() != project_entry.data {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "PROJECT bytes changed after round-trip",
                    ));
                }
                let code_page = directory.code_page().ok_or_else(|| {
                    olecfsdk::Error::invalid(0, "VBA dir has no PROJECTCODEPAGE record")
                })?;
                let _ = project.text(olecfsdk::common::CodePage(code_page))?;
                project_checked += 1;
                if let Some(project_wm_entry) = compound.entries().iter().find(|candidate| {
                    candidate.is_stream()
                        && candidate.path.parent() == Some(project_root)
                        && candidate.name.eq_ignore_ascii_case("PROJECTwm")
                }) {
                    let project_wm = ProjectWmStream::from_bytes(&project_wm_entry.data)?;
                    project_wm.validate_names(olecfsdk::common::CodePage(code_page))?;
                    if project_wm.to_bytes()? != project_wm_entry.data {
                        return Err(olecfsdk::Error::invalid(
                            0,
                            "PROJECTwm bytes changed after round-trip",
                        ));
                    }
                    project_wm_checked += 1;
                }
                if let Some(project_lk_entry) = compound.entries().iter().find(|candidate| {
                    candidate.is_stream()
                        && candidate.path.parent() == Some(project_root)
                        && candidate.name.eq_ignore_ascii_case("PROJECTlk")
                }) {
                    let project_lk = ProjectLkStream::from_bytes(&project_lk_entry.data)?;
                    if project_lk.to_bytes()? != project_lk_entry.data {
                        return Err(olecfsdk::Error::invalid(
                            0,
                            "PROJECTlk bytes changed after round-trip",
                        ));
                    }
                    project_lk_checked += 1;
                }
                let project_cache_entry = compound
                    .entries()
                    .iter()
                    .find(|candidate| {
                        candidate.is_stream()
                            && candidate.path.parent() == Some(parent)
                            && candidate.name.eq_ignore_ascii_case("_VBA_PROJECT")
                    })
                    .ok_or_else(|| {
                        olecfsdk::Error::invalid(0, "VBA storage has no _VBA_PROJECT stream")
                    })?;
                let project_cache = VbaProjectStream::from_bytes(&project_cache_entry.data)?;
                if project_cache.to_bytes()? != project_cache_entry.data {
                    return Err(olecfsdk::Error::invalid(
                        0,
                        "_VBA_PROJECT bytes changed after round-trip",
                    ));
                }
                project_cache_checked += 1;
                for descriptor in directory.modules() {
                    let stream_name = descriptor.stream_name().ok_or_else(|| {
                        olecfsdk::Error::invalid(0, "VBA module has no usable stream name")
                    })?;
                    let text_offset = descriptor.text_offset.ok_or_else(|| {
                        olecfsdk::Error::invalid(0, "VBA module has no text offset")
                    })?;
                    let module_entry = compound
                        .entries()
                        .iter()
                        .find(|candidate| {
                            candidate.is_stream()
                                && candidate.path.parent() == Some(parent)
                                && candidate.name.eq_ignore_ascii_case(&stream_name)
                        })
                        .ok_or_else(|| {
                            olecfsdk::Error::invalid(
                                0,
                                format!("VBA module stream not found: {stream_name}"),
                            )
                        })?;
                    let module = ModuleStream::from_bytes(&module_entry.data, text_offset)?;
                    if module.to_bytes()? != module_entry.data {
                        return Err(olecfsdk::Error::invalid(
                            0,
                            format!("VBA module bytes changed: {stream_name}"),
                        ));
                    }
                    let _ = module.source_bytes()?;
                    module_checked += 1;
                }
                Ok::<_, olecfsdk::Error>(())
            })();
            if let Err(error) = result {
                if expected_invalid.contains(&path) {
                    observed_invalid.insert(path.clone());
                } else {
                    failures.push(format!(
                        "{}:{}: {error}",
                        path.display(),
                        entry.path.display()
                    ));
                }
            }
        }
    }
    assert!(checked > 0, "no VBA dir streams found in legacy corpus");
    assert!(
        module_checked > 0,
        "no VBA module streams found in legacy corpus"
    );
    assert!(
        project_cache_checked > 0,
        "no _VBA_PROJECT streams found in legacy corpus"
    );
    assert!(
        project_checked > 0,
        "no PROJECT streams found in legacy corpus"
    );
    eprintln!(
        "checked {checked} dir, {project_cache_checked} _VBA_PROJECT, {project_checked} PROJECT, {project_wm_checked} PROJECTwm, {project_lk_checked} PROJECTlk, and {module_checked} module streams"
    );
    let missing: Vec<_> = expected_invalid.difference(&observed_invalid).collect();
    assert!(
        missing.is_empty(),
        "VBA compression invalid expectations no longer fail: {missing:?}"
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} VBA dir streams failed:\n{}",
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
            if expectation.test == "vba_compression_roundtrip"
                && expectation.mode == ExpectationMode::Invalid
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

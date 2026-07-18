use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    cfb::CompoundFile,
    doc::{DocFile, DocOfficeArtRecordTree},
    forms::ParentControlStorage,
    limits::Limits,
    office_art::{OfficeArtPartialStream, OfficeArtRecordData, OfficeArtStream},
    ppt::{PicturesStream, PptFile, PptRecordData},
    property_set::{PropertySetStream, TypedPropertyValue},
    vba::VbaProject,
    xls::{BiffRecordData, MsoDrawingData, XlsFile},
};
use olecfsdk_corpus_test_support::{
    corpus_bytes, corpus_root,
    manifest::{ExpectationMode, read_manifest},
};

use crate::{CoverageCategoryReport, CoverageDomain, CoverageReport};

const SOURCES: [&str; 2] = ["Apache-POI", "LibreOffice"];
const ALL_DOMAINS: [CoverageDomain; 8] = [
    CoverageDomain::Cfb,
    CoverageDomain::Doc,
    CoverageDomain::Xls,
    CoverageDomain::Ppt,
    CoverageDomain::Vba,
    CoverageDomain::OlePropertySet,
    CoverageDomain::OfficeArt,
    CoverageDomain::Forms,
];

enum Attempt {
    RoundTripped,
    RoundTripFailed(String),
    Rejected(String),
}

struct AuditSpec<'a> {
    domain: CoverageDomain,
    extensions: &'a [&'a str],
    exclusion_tests: &'a [&'a str],
    strict: fn(&[u8]) -> Attempt,
    compatible: fn(&[u8]) -> Attempt,
}

pub fn audit_classic_office_file_roots() -> Result<CoverageReport, String> {
    let corpus = corpus_root();
    let mut report = CoverageReport::default();
    for domain in ALL_DOMAINS {
        report
            .categories
            .insert(domain, CoverageCategoryReport::default());
    }

    audit_domain(
        &corpus,
        AuditSpec {
            domain: CoverageDomain::Cfb,
            extensions: &["doc", "dot", "xls", "xlt", "ppt", "pps", "pot"],
            exclusion_tests: &["cfb_roundtrip"],
            strict: attempt_cfb_strict,
            compatible: attempt_cfb_compatible,
        },
        &mut report,
    )?;
    audit_domain(
        &corpus,
        AuditSpec {
            domain: CoverageDomain::Doc,
            extensions: &["doc"],
            exclusion_tests: &["doc_fib_roundtrip"],
            strict: attempt_doc_strict,
            compatible: attempt_doc_compatible,
        },
        &mut report,
    )?;
    audit_domain(
        &corpus,
        AuditSpec {
            domain: CoverageDomain::Xls,
            extensions: &["xls", "xlt"],
            exclusion_tests: &["xls_biff_roundtrip"],
            strict: attempt_xls_strict,
            compatible: attempt_xls_compatible,
        },
        &mut report,
    )?;
    audit_domain(
        &corpus,
        AuditSpec {
            domain: CoverageDomain::Ppt,
            extensions: &["ppt"],
            exclusion_tests: &["cfb_roundtrip", "ppt_record_roundtrip"],
            strict: attempt_ppt_strict,
            compatible: attempt_ppt_compatible,
        },
        &mut report,
    )?;
    audit_vba(&corpus, &mut report)?;
    audit_ole_property_sets(&corpus, &mut report)?;
    audit_forms(&corpus, &mut report)?;
    audit_office_art(&corpus, &mut report);
    report.validate()?;
    Ok(report)
}

fn audit_vba(corpus: &Path, report: &mut CoverageReport) -> Result<(), String> {
    let extensions = &["doc", "dot", "xls", "xlt", "ppt", "pps", "pot"];
    let exclusions = exclusions_for(corpus, &["vba_compression_roundtrip"], extensions)?;
    let category = report
        .categories
        .get_mut(&CoverageDomain::Vba)
        .expect("all coverage domains were initialized");
    for path in corpus_files(corpus, extensions) {
        let Ok(bytes) = corpus_bytes(&path) else {
            continue;
        };
        let Ok(compound) = CompoundFile::from_bytes(&bytes) else {
            continue;
        };
        let paths = compound
            .entries()
            .iter()
            .filter(|entry| {
                entry.is_storage()
                    && entry.name.eq_ignore_ascii_case("VBA")
                    && compound.stream(entry.path.join("dir")).is_some()
            })
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        for storage_path in paths {
            category.counts.discovered += 1;
            if exclusions.contains(&path) {
                category.counts.excluded += 1;
            } else {
                record_single_mode(category, attempt_vba(&compound, &storage_path));
            }
        }
    }
    Ok(())
}

fn attempt_vba(compound: &CompoundFile, storage_path: &Path) -> Attempt {
    let project = match VbaProject::from_compound_file_at(compound, storage_path) {
        Ok(project) => project,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let source_before = match project
        .modules
        .iter()
        .map(|module| module.stream.source_bytes())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(source) => source,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    let mut rewritten = compound.clone();
    if let Err(error) = project.write_interoperable_to_compound_file(&mut rewritten) {
        return Attempt::RoundTripFailed(error.to_string());
    }
    let reopened = match VbaProject::from_compound_file_at(&rewritten, storage_path) {
        Ok(project) => project,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    let source_after = match reopened
        .modules
        .iter()
        .map(|module| module.stream.source_bytes())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(source) => source,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    if source_before == source_after {
        Attempt::RoundTripped
    } else {
        Attempt::RoundTripFailed("VBA module source changed after interoperable write".to_owned())
    }
}

fn audit_ole_property_sets(corpus: &Path, report: &mut CoverageReport) -> Result<(), String> {
    let extensions = &["doc", "dot", "xls", "xlt", "ppt", "pps", "pot"];
    let exclusions = exclusions_for(corpus, &["oleps_roundtrip"], extensions)?;
    let category = report
        .categories
        .get_mut(&CoverageDomain::OlePropertySet)
        .expect("all coverage domains were initialized");
    for path in corpus_files(corpus, extensions) {
        let Ok(bytes) = corpus_bytes(&path) else {
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
            category.counts.discovered += 1;
            if exclusions.contains(&path) {
                category.counts.excluded += 1;
            } else {
                record_single_mode(category, attempt_ole_property_set(&entry.data));
            }
        }
    }
    Ok(())
}

fn attempt_ole_property_set(bytes: &[u8]) -> Attempt {
    let parsed = match PropertySetStream::from_bytes(bytes) {
        Ok(parsed) => parsed,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let result = (|| {
        for property_set in &parsed.property_sets {
            let code_page = property_set.code_page()?;
            for property in &property_set.properties {
                if property.identifier == 0 {
                    let dictionary = property.dictionary(code_page.ok_or_else(|| {
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
                if let TypedPropertyValue::Unknown { property_type, .. } = typed {
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
        if PropertySetStream::from_bytes(&saved)? != parsed {
            return Err(olecfsdk::Error::invalid(
                0,
                "OLEPS structure changed after round-trip",
            ));
        }
        Ok::<(), olecfsdk::Error>(())
    })();
    match result {
        Ok(()) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn audit_forms(corpus: &Path, report: &mut CoverageReport) -> Result<(), String> {
    const CLASSES: [&str; 3] = [
        "46e31370-3f7a-11ce-bed6-00aa00611080",
        "6e182020-f460-11ce-9bcd-00aa00608e01",
        "c62a69f0-16dc-11ce-9e98-00aa00574a4f",
    ];
    let category = report
        .categories
        .get_mut(&CoverageDomain::Forms)
        .expect("all coverage domains were initialized");
    for path in corpus_files(corpus, &["doc", "dot", "xls", "xlt", "ppt", "pps", "pot"]) {
        let Ok(bytes) = corpus_bytes(&path) else {
            continue;
        };
        let Ok(compound) = CompoundFile::from_bytes(&bytes) else {
            continue;
        };
        let paths = compound
            .entries()
            .iter()
            .filter(|entry| {
                entry.is_storage() && CLASSES.contains(&entry.clsid.to_string().as_str())
            })
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        for storage_path in paths {
            category.counts.discovered += 1;
            let aggregate = match ParentControlStorage::from_compound(&compound, &storage_path) {
                Ok(aggregate) => aggregate,
                Err(error) => {
                    record_single_mode(category, Attempt::Rejected(error.to_string()));
                    continue;
                }
            };
            let mut rewritten = compound.clone();
            let attempt = match aggregate.write_to_compound(&mut rewritten) {
                Ok(()) if compound.logical_eq(&rewritten) => Attempt::RoundTripped,
                Ok(()) => Attempt::RoundTripFailed("Forms CFB tree changed".to_owned()),
                Err(error) => Attempt::RoundTripFailed(error.to_string()),
            };
            record_single_mode(category, attempt);
        }
    }
    Ok(())
}

fn audit_office_art(corpus: &Path, report: &mut CoverageReport) {
    let category = report
        .categories
        .get_mut(&CoverageDomain::OfficeArt)
        .expect("all coverage domains were initialized");

    for path in corpus_files(corpus, &["doc"]) {
        let Ok(bytes) = corpus_bytes(&path) else {
            continue;
        };
        let Ok(outcome) = DocFile::from_bytes_compatible(&bytes) else {
            continue;
        };
        let Some(content) = outcome.value.table.office_art.as_ref() else {
            continue;
        };
        audit_doc_office_art_tree(category, &content.value.drawing_group);
        for drawing in &content.value.drawings {
            audit_doc_office_art_tree(category, &drawing.container);
        }
    }

    for path in corpus_files(corpus, &["xls", "xlt"]) {
        let Ok(bytes) = corpus_bytes(&path) else {
            continue;
        };
        let Ok(outcome) = XlsFile::from_bytes_compatible(&bytes) else {
            continue;
        };
        for workbook in &outcome.value.workbooks {
            for record in &workbook.tree.stream.records {
                match &record.data {
                    BiffRecordData::MsoDrawingGroup(value) | BiffRecordData::MsoDrawing(value) => {
                        match &value.data {
                            MsoDrawingData::Complete(stream) => {
                                audit_complete_office_art(category, stream, false)
                            }
                            MsoDrawingData::Partial(stream) => {
                                audit_partial_office_art(category, stream)
                            }
                            MsoDrawingData::Incomplete { bytes, reason } => {
                                category.counts.discovered += 1;
                                category.counts.rejected += 1;
                                increment_metric(category, "incomplete_units", 1);
                                increment_metric(category, "bytes", bytes.len() as u64);
                                *category
                                    .rejection_reasons
                                    .entry(reason.clone())
                                    .or_default() += 1;
                            }
                        }
                    }
                    BiffRecordData::GelFrame(stream) => {
                        audit_complete_office_art(category, stream, false)
                    }
                    _ => {}
                }
            }
        }
    }

    for path in corpus_files(corpus, &["ppt"]) {
        let Ok(bytes) = corpus_bytes(&path) else {
            continue;
        };
        let Ok(outcome) = PptFile::from_bytes_compatible(&bytes) else {
            continue;
        };
        outcome.value.document.records.visit(&mut |record| {
            if let PptRecordData::OfficeArt(value) = &record.data {
                audit_complete_office_art(
                    category,
                    &OfficeArtStream {
                        records: vec![(**value).clone()],
                    },
                    false,
                );
            }
        });
        if let Some(pictures) = &outcome.value.pictures {
            match pictures {
                PicturesStream::Complete(value) => audit_complete_office_art(
                    category,
                    &OfficeArtStream {
                        records: value.records.clone(),
                    },
                    false,
                ),
                PicturesStream::Compatibility { stream, .. } => {
                    audit_complete_office_art(category, stream, true)
                }
                PicturesStream::Partial(stream) => audit_partial_office_art(category, stream),
            }
        }
    }
}

fn audit_doc_office_art_tree(category: &mut CoverageCategoryReport, tree: &DocOfficeArtRecordTree) {
    match tree {
        DocOfficeArtRecordTree::Complete(stream) => {
            audit_complete_office_art(category, stream, false)
        }
        DocOfficeArtRecordTree::Partial(stream) => audit_partial_office_art(category, stream),
    }
}

fn audit_complete_office_art(
    category: &mut CoverageCategoryReport,
    stream: &OfficeArtStream,
    compatible: bool,
) {
    category.counts.discovered += 1;
    increment_metric(category, "complete_units", 1);
    let bytes = match stream.to_bytes() {
        Ok(bytes) => bytes,
        Err(error) => {
            record_office_art_attempt(
                category,
                Attempt::RoundTripFailed(error.to_string()),
                compatible,
            );
            return;
        }
    };
    increment_metric(category, "bytes", bytes.len() as u64);
    stream.visit(|record| {
        increment_metric(category, "records", 1);
        match &record.data {
            OfficeArtRecordData::CompatibilityContainer(_) => {
                increment_metric(category, "compatibility_containers", 1)
            }
            OfficeArtRecordData::Atom(payload) => {
                increment_metric(category, "opaque_atom_records", 1);
                increment_metric(category, "opaque_atom_bytes", payload.len() as u64);
            }
            OfficeArtRecordData::IncompletePropertyTable(_) => {
                increment_metric(category, "incomplete_property_tables", 1)
            }
            _ => {}
        }
    });
    let attempt = match OfficeArtStream::from_bytes(&bytes) {
        Ok(reopened) => match reopened.to_bytes() {
            Ok(reopened_bytes) if reopened_bytes == bytes => {
                if reopened != *stream {
                    increment_metric(category, "contextual_tree_differences", 1);
                }
                Attempt::RoundTripped
            }
            Ok(_) => Attempt::RoundTripFailed(
                "OfficeArt bytes changed after parse-write-parse-write".to_owned(),
            ),
            Err(error) => Attempt::RoundTripFailed(error.to_string()),
        },
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    };
    record_office_art_attempt(category, attempt, compatible);
}

fn audit_partial_office_art(
    category: &mut CoverageCategoryReport,
    stream: &OfficeArtPartialStream,
) {
    category.counts.discovered += 1;
    increment_metric(category, "partial_units", 1);
    increment_metric(
        category,
        "complete_records_in_partial_units",
        stream.complete_record_count() as u64,
    );
    increment_metric(
        category,
        "incomplete_records",
        stream.incomplete_record_count() as u64,
    );
    increment_metric(
        category,
        "unparsed_bytes",
        stream.unparsed_byte_count() as u64,
    );
    let bytes = match stream.to_bytes() {
        Ok(bytes) => bytes,
        Err(error) => {
            record_office_art_attempt(category, Attempt::RoundTripFailed(error.to_string()), true);
            return;
        }
    };
    increment_metric(category, "bytes", bytes.len() as u64);
    let attempt = match OfficeArtPartialStream::from_bytes_with_limits(
        &bytes,
        Limits::default(),
        stream.reason.clone(),
    ) {
        Ok(reopened) => match reopened.to_bytes() {
            Ok(reopened_bytes) if reopened_bytes == bytes => {
                if reopened != *stream {
                    increment_metric(category, "contextual_tree_differences", 1);
                }
                Attempt::RoundTripped
            }
            Ok(_) => Attempt::RoundTripFailed(
                "partial OfficeArt bytes changed after parse-write-parse-write".to_owned(),
            ),
            Err(error) => Attempt::RoundTripFailed(error.to_string()),
        },
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    };
    record_office_art_attempt(category, attempt, true);
}

fn record_office_art_attempt(
    category: &mut CoverageCategoryReport,
    attempt: Attempt,
    compatible: bool,
) {
    match attempt {
        Attempt::RoundTripped => {
            if compatible {
                category.counts.compatible += 1;
            } else {
                category.counts.strict += 1;
            }
            category.counts.round_tripped += 1;
        }
        Attempt::RoundTripFailed(reason) => {
            if compatible {
                category.counts.compatible += 1;
            } else {
                category.counts.strict += 1;
            }
            *category
                .round_trip_failure_reasons
                .entry(reason)
                .or_default() += 1;
        }
        Attempt::Rejected(reason) => {
            category.counts.rejected += 1;
            *category.rejection_reasons.entry(reason).or_default() += 1;
        }
    }
}

fn increment_metric(category: &mut CoverageCategoryReport, metric: &str, value: u64) {
    *category.metrics.entry(metric.to_owned()).or_default() += value;
}

fn record_single_mode(category: &mut CoverageCategoryReport, attempt: Attempt) {
    match attempt {
        Attempt::RoundTripped => {
            category.counts.strict += 1;
            category.counts.round_tripped += 1;
        }
        Attempt::RoundTripFailed(reason) => {
            category.counts.strict += 1;
            *category
                .round_trip_failure_reasons
                .entry(reason)
                .or_default() += 1;
        }
        Attempt::Rejected(reason) => {
            category.counts.rejected += 1;
            *category.rejection_reasons.entry(reason).or_default() += 1;
        }
    }
}

fn audit_domain(
    corpus: &Path,
    spec: AuditSpec<'_>,
    report: &mut CoverageReport,
) -> Result<(), String> {
    let files = corpus_files(corpus, spec.extensions);
    let exclusions = exclusions_for(corpus, spec.exclusion_tests, spec.extensions)?;
    let category = report
        .categories
        .get_mut(&spec.domain)
        .expect("all coverage domains were initialized");
    category.counts.discovered = files.len() as u64;
    for path in files {
        if exclusions.contains(&path) {
            category.counts.excluded += 1;
            continue;
        }
        let bytes = match corpus_bytes(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                category.counts.rejected += 1;
                *category.rejection_reasons.entry(error).or_default() += 1;
                continue;
            }
        };
        match (spec.strict)(&bytes) {
            Attempt::RoundTripped => {
                category.counts.strict += 1;
                category.counts.round_tripped += 1;
            }
            Attempt::RoundTripFailed(reason) => {
                category.counts.strict += 1;
                *category
                    .round_trip_failure_reasons
                    .entry(reason)
                    .or_default() += 1;
            }
            Attempt::Rejected(_) => match (spec.compatible)(&bytes) {
                Attempt::RoundTripped => {
                    category.counts.compatible += 1;
                    category.counts.round_tripped += 1;
                }
                Attempt::RoundTripFailed(reason) => {
                    category.counts.compatible += 1;
                    *category
                        .round_trip_failure_reasons
                        .entry(reason)
                        .or_default() += 1;
                }
                Attempt::Rejected(reason) => {
                    category.counts.rejected += 1;
                    *category.rejection_reasons.entry(reason).or_default() += 1;
                }
            },
        }
    }
    Ok(())
}

fn attempt_cfb_strict(bytes: &[u8]) -> Attempt {
    let file = match CompoundFile::from_bytes_strict(bytes) {
        Ok(file) => file,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    round_trip_cfb(file)
}

fn attempt_cfb_compatible(bytes: &[u8]) -> Attempt {
    let file = match CompoundFile::from_bytes(bytes) {
        Ok(file) => file,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    round_trip_cfb(file)
}

fn round_trip_cfb(file: CompoundFile) -> Attempt {
    let saved = match file.to_bytes() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match CompoundFile::from_bytes_strict(&saved) {
        Ok(reopened) if file.logical_eq(&reopened) => Attempt::RoundTripped,
        Ok(_) => Attempt::RoundTripFailed("logical CFB tree changed".to_owned()),
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn attempt_doc_strict(bytes: &[u8]) -> Attempt {
    let file = match DocFile::from_bytes(bytes) {
        Ok(file) => file,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let saved = match file.to_bytes() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match DocFile::from_bytes(&saved) {
        Ok(_) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn attempt_doc_compatible(bytes: &[u8]) -> Attempt {
    let file = match DocFile::from_bytes_compatible(bytes) {
        Ok(outcome) => outcome.value,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let saved = match file.to_bytes_preserving_compatibility() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match DocFile::from_bytes_compatible(&saved) {
        Ok(_) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn attempt_xls_strict(bytes: &[u8]) -> Attempt {
    let file = match XlsFile::from_bytes(bytes) {
        Ok(file) => file,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let saved = match file.to_bytes() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match XlsFile::from_bytes(&saved) {
        Ok(_) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn attempt_xls_compatible(bytes: &[u8]) -> Attempt {
    let file = match XlsFile::from_bytes_compatible(bytes) {
        Ok(outcome) => outcome.value,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let saved = match file.to_bytes_preserving_compatibility() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match XlsFile::from_bytes_compatible(&saved) {
        Ok(_) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn attempt_ppt_strict(bytes: &[u8]) -> Attempt {
    let file = match PptFile::from_bytes(bytes) {
        Ok(file) => file,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let saved = match file.to_bytes() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match PptFile::from_bytes(&saved) {
        Ok(_) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn attempt_ppt_compatible(bytes: &[u8]) -> Attempt {
    let file = match PptFile::from_bytes_compatible(bytes) {
        Ok(outcome) => outcome.value,
        Err(error) => return Attempt::Rejected(error.to_string()),
    };
    let saved = match file.to_bytes_preserving_compatibility() {
        Ok(saved) => saved,
        Err(error) => return Attempt::RoundTripFailed(error.to_string()),
    };
    match PptFile::from_bytes_compatible(&saved) {
        Ok(_) => Attempt::RoundTripped,
        Err(error) => Attempt::RoundTripFailed(error.to_string()),
    }
}

fn corpus_files(corpus: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for source in SOURCES {
        collect(&corpus.join(source), extensions, &mut files);
    }
    files.sort();
    files
}

fn collect(directory: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extensions, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extensions
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            })
        {
            files.push(path);
        }
    }
}

fn exclusions_for(
    corpus: &Path,
    test_names: &[&str],
    extensions: &[&str],
) -> Result<BTreeSet<PathBuf>, String> {
    let mut exclusions = BTreeSet::new();
    for source in SOURCES {
        let root = corpus.join(source);
        let manifest = read_manifest(&root.join("manifest.toml"))?;
        for expectation in manifest.expectation {
            if test_names.contains(&expectation.test.as_str())
                && matches!(
                    expectation.mode,
                    ExpectationMode::Invalid
                        | ExpectationMode::Unsupported
                        | ExpectationMode::RequiresPassword
                        | ExpectationMode::KnownFailure
                )
                && expectation
                    .file
                    .rsplit_once('.')
                    .is_some_and(|(_, extension)| {
                        extensions
                            .iter()
                            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
                    })
            {
                exclusions.insert(root.join(expectation.file));
            }
        }
    }
    Ok(exclusions)
}

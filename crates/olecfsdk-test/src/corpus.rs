use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    cfb::CompoundFile,
    common::Guid,
    doc::{DocDataNodeValue, DocFile, DocOfficeArtRecordTree},
    forms::{
        CachedControlClass, FormControlPersistence, LocatedParentControlStorage,
        ParentControlStorage, SiteClassIndex,
    },
    limits::Limits,
    office_art::{OfficeArtPartialStream, OfficeArtRecordData, OfficeArtStream},
    ppt::{PicturesStream, PptFile, PptRecordData},
    property_set::{ArrayValue, PropertySetStream, TypedPropertyValue, VectorValue},
    vba::{VbaProject, directory::DirRecord, project::ProjectRecordKind},
    xls::{
        BiffRecordData, MsoDrawingData, ObjPictureFlags, XlsFile, XlsFileEntryRole,
        XlsObjectPersistenceRef,
    },
};
use olecfsdk_corpus_test_support::{
    corpus_bytes, corpus_root,
    manifest::{ExpectationMode, read_manifest},
};

use crate::{CoverageCategoryReport, CoverageDomain, CoverageReport, bundled_coverage_evidence};

const SOURCES: [&str; 2] = ["Apache-POI", "LibreOffice"];
enum Attempt {
    RoundTripped,
    RoundTripFailed(String),
    Rejected(String),
}

#[derive(Clone, Copy)]
enum Disposition {
    Typed,
    SpecificationOpaque,
    ExternalLeaf,
    UnknownExtension,
    Compatibility,
    Malformed,
    TemporaryUntyped,
}

impl Disposition {
    const fn name(self) -> &'static str {
        match self {
            Self::Typed => "typed",
            Self::SpecificationOpaque => "specification_opaque",
            Self::ExternalLeaf => "external_leaf",
            Self::UnknownExtension => "unknown_extension",
            Self::Compatibility => "compatibility",
            Self::Malformed => "malformed",
            Self::TemporaryUntyped => "temporary_untyped",
        }
    }
}

struct AuditSpec<'a> {
    domain: CoverageDomain,
    extensions: &'a [&'a str],
    exclusion_tests: &'a [&'a str],
    strict: fn(&[u8]) -> Attempt,
    compatible: fn(&[u8]) -> Attempt,
    record_metrics: fn(&[u8], &mut CoverageCategoryReport),
}

pub fn audit_classic_office_file_roots() -> Result<CoverageReport, String> {
    let corpus = corpus_root();
    let mut report = CoverageReport::default();

    audit_domain(
        &corpus,
        AuditSpec {
            domain: CoverageDomain::Cfb,
            extensions: &["doc", "dot", "xls", "xlt", "ppt", "pps", "pot"],
            exclusion_tests: &["cfb_roundtrip"],
            strict: attempt_cfb_strict,
            compatible: attempt_cfb_compatible,
            record_metrics: record_cfb_metrics,
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
            record_metrics: record_doc_metrics,
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
            record_metrics: record_xls_metrics,
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
            record_metrics: record_ppt_metrics,
        },
        &mut report,
    )?;
    audit_vba(&corpus, &mut report)?;
    audit_ole_property_sets(&corpus, &mut report)?;
    audit_forms(&corpus, &mut report)?;
    audit_office_art(&corpus, &mut report);
    report.validate()?;
    report.assert_debt_has_evidence(&bundled_coverage_evidence()?)?;
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
            .collect::<BTreeSet<_>>();
        let root_paths = paths
            .iter()
            .filter(|path| {
                !path
                    .ancestors()
                    .skip(1)
                    .any(|ancestor| paths.contains(ancestor))
            })
            .cloned()
            .collect::<Vec<_>>();
        for storage_path in root_paths {
            category.counts.discovered += 1;
            if let Ok(entries) = compound.walk_storage(&storage_path) {
                increment_metric(category, "storage_entries", entries.len() as u64);
                increment_metric(
                    category,
                    "storage_stream_bytes",
                    entries.iter().map(|entry| entry.data.len() as u64).sum(),
                );
            }
            if let Some(reason) = exclusions.get(&path) {
                category.counts.excluded += 1;
                *category
                    .exclusion_reasons
                    .entry(reason.clone())
                    .or_default() += 1;
            } else {
                if let Ok(project) = VbaProject::from_compound_file_at(&compound, &storage_path) {
                    record_vba_metrics(category, &project)?;
                }
                record_single_mode(category, attempt_vba(&compound, &storage_path));
            }
        }
    }
    Ok(())
}

fn record_vba_metrics(
    category: &mut CoverageCategoryReport,
    project: &VbaProject,
) -> Result<(), String> {
    increment_metric(category, "projects", 1);
    increment_metric(category, "modules", project.modules.len() as u64);
    increment_metric(
        category,
        "directory_records",
        project.directory.records.len() as u64,
    );
    increment_metric(
        category,
        "structural_records",
        project.directory.records.len() as u64,
    );
    increment_metric(category, "directory_reserved_fields", 1);
    increment_metric(category, "directory_reserved_bytes", 4);
    for record in &project.directory.records {
        let disposition = match record {
            DirRecord::Unknown { id, payload } => {
                increment_metric(
                    category,
                    "unknown_directory_record_bytes",
                    payload.len() as u64,
                );
                increment_metric(
                    category,
                    &format!("unknown_directory_record.id_0x{id:04x}.records"),
                    1,
                );
                Disposition::UnknownExtension
            }
            _ => Disposition::Typed,
        };
        increment_disposition(category, disposition, "records", 1);
    }

    if let Some(project_stream) = &project.project {
        increment_metric(category, "project_streams", 1);
        increment_metric(
            category,
            "project_records",
            project_stream.records.len() as u64,
        );
        increment_metric(
            category,
            "structural_records",
            project_stream.records.len() as u64,
        );
        for record in &project_stream.records {
            let disposition = match &record.kind {
                ProjectRecordKind::Unknown { bytes } => {
                    increment_metric(category, "unknown_project_record_bytes", bytes.len() as u64);
                    Disposition::UnknownExtension
                }
                _ => Disposition::Typed,
            };
            increment_disposition(category, disposition, "records", 1);
        }
    }
    if let Some(project_wm) = &project.project_wm {
        increment_metric(category, "project_wm_streams", 1);
        increment_metric(category, "project_wm_names", project_wm.names.len() as u64);
    }
    if let Some(project_lk) = &project.project_lk {
        increment_metric(category, "project_lk_streams", 1);
        increment_metric(
            category,
            "project_lk_licenses",
            project_lk.licenses.len() as u64,
        );
        for license in &project_lk.licenses {
            increment_metric(
                category,
                "project_lk_license_key_bytes",
                license.license_key.len() as u64,
            );
            record_vba_specification_opaque_leaf(category, &license.license_key);
        }
    }

    for module in &project.modules {
        let compressed_source_bytes = module
            .stream
            .compressed_source_code
            .to_bytes()
            .map_err(|error| format!("parsed VBA module source did not serialize: {error}"))?
            .len() as u64;
        increment_metric(category, "leaf_units", 1);
        increment_metric(category, "leaf_bytes", compressed_source_bytes);
        increment_metric(category, "compressed_source_bytes", compressed_source_bytes);
        increment_disposition(category, Disposition::Typed, "units", 1);
        increment_disposition(
            category,
            Disposition::Typed,
            "bytes",
            compressed_source_bytes,
        );
        record_vba_cache_leaf(category, &module.stream.performance_cache);
    }
    record_vba_cache_leaf(category, &project.cache.performance_cache);
    increment_metric(category, "srp_streams", project.srp_streams.len() as u64);
    for srp in &project.srp_streams {
        record_vba_cache_leaf(category, &srp.implementation_specific_cache);
    }
    Ok(())
}

fn record_vba_cache_leaf(category: &mut CoverageCategoryReport, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    increment_metric(category, "implementation_cache_units", 1);
    increment_metric(category, "implementation_cache_bytes", bytes.len() as u64);
    record_vba_specification_opaque_leaf(category, bytes);
}

fn record_vba_specification_opaque_leaf(category: &mut CoverageCategoryReport, bytes: &[u8]) {
    increment_metric(category, "leaf_units", 1);
    increment_metric(category, "leaf_bytes", bytes.len() as u64);
    increment_disposition(category, Disposition::SpecificationOpaque, "units", 1);
    increment_disposition(
        category,
        Disposition::SpecificationOpaque,
        "bytes",
        bytes.len() as u64,
    );
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
            increment_metric(category, "property_stream_bytes", entry.data.len() as u64);
            if let Some(reason) = exclusions.get(&path) {
                category.counts.excluded += 1;
                *category
                    .exclusion_reasons
                    .entry(reason.clone())
                    .or_default() += 1;
            } else {
                record_ole_property_set_metrics(&entry.data, category);
                record_single_mode(category, attempt_ole_property_set(&entry.data));
            }
        }
    }
    Ok(())
}

fn record_ole_property_set_metrics(bytes: &[u8], category: &mut CoverageCategoryReport) {
    let Ok(stream) = PropertySetStream::from_bytes(bytes) else {
        return;
    };
    increment_metric(category, "parsed_property_streams", 1);
    increment_metric(category, "property_sets", stream.property_sets.len() as u64);
    for property_set in &stream.property_sets {
        let code_page = property_set.code_page().ok().flatten();
        for property in &property_set.properties {
            let property_bytes = property.raw.len() as u64;
            increment_metric(category, "properties", 1);
            increment_metric(category, "property_bytes", property_bytes);
            let disposition = if property.identifier == 0 {
                increment_metric(category, "property_kind.dictionary.units", 1);
                increment_metric(category, "property_kind.dictionary.bytes", property_bytes);
                match code_page
                    .ok_or(())
                    .and_then(|code_page| property.dictionary(code_page).map_err(|_| ()))
                {
                    Ok(dictionary)
                        if dictionary.to_bytes().is_ok_and(|raw| raw == property.raw) =>
                    {
                        increment_metric(
                            category,
                            "dictionary_entries",
                            dictionary.entries.len() as u64,
                        );
                        Disposition::Typed
                    }
                    _ => Disposition::Malformed,
                }
            } else {
                match property.typed_value() {
                    Ok(value) => {
                        let property_type = oleps_property_type(&value);
                        increment_metric(
                            category,
                            &format!("property_type.type_0x{property_type:04x}.units"),
                            1,
                        );
                        increment_metric(
                            category,
                            &format!("property_type.type_0x{property_type:04x}.bytes"),
                            property_bytes,
                        );
                        let mut unknown = Vec::new();
                        collect_oleps_unknown_values(&value, &mut unknown);
                        let byte_exact = value.to_bytes().is_ok_and(|raw| raw == property.raw);
                        if !byte_exact {
                            Disposition::Malformed
                        } else if unknown.is_empty() {
                            Disposition::Typed
                        } else {
                            for (unknown_type, unknown_bytes) in unknown {
                                increment_metric(
                                    category,
                                    &format!(
                                        "debt.unknown_extension.type_0x{unknown_type:04x}.units"
                                    ),
                                    1,
                                );
                                increment_metric(
                                    category,
                                    &format!(
                                        "debt.unknown_extension.type_0x{unknown_type:04x}.bytes"
                                    ),
                                    unknown_bytes,
                                );
                            }
                            Disposition::UnknownExtension
                        }
                    }
                    Err(_) => Disposition::Malformed,
                }
            };
            increment_disposition(category, disposition, "units", 1);
            increment_disposition(category, disposition, "bytes", property_bytes);
        }
    }
}

fn oleps_property_type(value: &TypedPropertyValue) -> u16 {
    match value {
        TypedPropertyValue::Empty { .. } => 0x0000,
        TypedPropertyValue::Null { .. } => 0x0001,
        TypedPropertyValue::I8Bit { property_type, .. }
        | TypedPropertyValue::U8Bit { property_type, .. }
        | TypedPropertyValue::I32 { property_type, .. }
        | TypedPropertyValue::U32 { property_type, .. }
        | TypedPropertyValue::I64 { property_type, .. }
        | TypedPropertyValue::F64Bits { property_type, .. }
        | TypedPropertyValue::CodePageString { property_type, .. }
        | TypedPropertyValue::Blob { property_type, .. }
        | TypedPropertyValue::IndirectPropertyName { property_type, .. }
        | TypedPropertyValue::Vector { property_type, .. }
        | TypedPropertyValue::Array { property_type, .. }
        | TypedPropertyValue::Unknown { property_type, .. } => property_type.0,
        TypedPropertyValue::I16 { .. } => 0x0002,
        TypedPropertyValue::U16 { .. } => 0x0012,
        TypedPropertyValue::U64 { .. } => 0x0015,
        TypedPropertyValue::F32Bits { .. } => 0x0004,
        TypedPropertyValue::Bool { .. } => 0x000b,
        TypedPropertyValue::Decimal { .. } => 0x000e,
        TypedPropertyValue::Filetime { .. } => 0x0040,
        TypedPropertyValue::UnicodeString { .. } => 0x001f,
        TypedPropertyValue::VersionedStream { .. } => 0x0049,
        TypedPropertyValue::ClipboardData { .. } => 0x0047,
        TypedPropertyValue::Clsid { .. } => 0x0048,
    }
}

fn collect_oleps_unknown_values(value: &TypedPropertyValue, unknown: &mut Vec<(u16, u64)>) {
    match value {
        TypedPropertyValue::Unknown {
            property_type, raw, ..
        } => unknown.push((property_type.0, raw.len() as u64 + 4)),
        TypedPropertyValue::Vector {
            values: VectorValue::Variants(values),
            ..
        }
        | TypedPropertyValue::Array {
            values: ArrayValue::Variants(values),
            ..
        } => {
            increment_nested_oleps_values(values, unknown);
        }
        _ => {}
    }
}

fn increment_nested_oleps_values(values: &[TypedPropertyValue], unknown: &mut Vec<(u16, u64)>) {
    for value in values {
        collect_oleps_unknown_values(value, unknown);
    }
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
        if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("xls" | "xlt")
        ) {
            record_xls_activex_metrics(&bytes, category);
        }
        for storage_path in
            LocatedParentControlStorage::discover_root_paths_below(&compound, Path::new("/"))
        {
            category.counts.discovered += 1;
            if let Ok(entries) = compound.walk_storage(&storage_path) {
                increment_metric(category, "storage_entries", entries.len() as u64);
                increment_metric(
                    category,
                    "storage_stream_bytes",
                    entries.iter().map(|entry| entry.data.len() as u64).sum(),
                );
            }
            let aggregate = match ParentControlStorage::from_compound(&compound, &storage_path) {
                Ok(aggregate) => aggregate,
                Err(error) => {
                    record_single_mode(category, Attempt::Rejected(error.to_string()));
                    continue;
                }
            };
            record_forms_metrics(category, &aggregate);
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

fn record_forms_metrics(category: &mut CoverageCategoryReport, storage: &ParentControlStorage) {
    increment_metric(category, "parent_storages", 1);
    increment_metric(
        category,
        &format!("parent_class.{}.units", forms_parent_class_name(storage)),
        1,
    );
    increment_metric(
        category,
        "class_table_entries",
        storage.form.site_data.class_table.len() as u64,
    );
    increment_metric(category, "sites", storage.form.site_data.sites.len() as u64);
    increment_metric(
        category,
        "streamed_controls",
        storage.object_stream.controls.len() as u64,
    );
    increment_metric(
        category,
        "embedded_parent_controls",
        storage.children.len() as u64,
    );
    if let Some(multi_page) = &storage.multi_page_x {
        increment_metric(category, "multi_page_x_streams", 1);
        increment_metric(category, "multi_page_pages", multi_page.pages.len() as u64);
    }

    for control in &storage.object_stream.controls {
        let class = forms_site_class_name(control.class_index);
        increment_metric(category, &format!("site_class.{class}.units"), 1);
        let persistence = forms_persistence_name(&control.persistence);
        increment_metric(category, &format!("persistence.{persistence}.units"), 1);
        match &control.persistence {
            FormControlPersistence::ExternalClass(value) => {
                increment_disposition(category, Disposition::ExternalLeaf, "units", 1);
                increment_disposition(
                    category,
                    Disposition::ExternalLeaf,
                    "bytes",
                    value.bytes.len() as u64,
                );
                increment_metric(
                    category,
                    "external_com_persistence_bytes",
                    value.bytes.len() as u64,
                );
            }
            _ => increment_disposition(category, Disposition::Typed, "units", 1),
        }
    }
    for child in &storage.children {
        increment_disposition(category, Disposition::Typed, "units", 1);
        record_forms_metrics(category, &child.storage);
    }
}

fn forms_parent_class_name(storage: &ParentControlStorage) -> &'static str {
    if storage.class_id == Guid::ZERO {
        "user_form"
    } else if storage.class_id == ParentControlStorage::MULTI_PAGE_CLASS_ID {
        "multi_page"
    } else if storage.class_id == ParentControlStorage::FRAME_CLASS_ID {
        "frame"
    } else if storage.class_id == ParentControlStorage::PAGE_CLASS_ID {
        "page"
    } else {
        "unexpected"
    }
}

fn record_xls_activex_metrics(bytes: &[u8], category: &mut CoverageCategoryReport) {
    let Ok(outcome) = XlsFile::from_bytes_compatible(bytes) else {
        return;
    };
    let file = outcome.value;
    let inventory = file.storages_and_streams_compatible();
    for entry in inventory.by_role(XlsFileEntryRole::ControlStream) {
        increment_metric(category, "activex.control_streams", 1);
        increment_metric(
            category,
            "activex.control_stream_bytes",
            entry.entry().data.len() as u64,
        );
    }

    for workbook in &file.workbooks {
        let Ok(view) = workbook.relationships_compatible() else {
            continue;
        };
        for sheet in view.sheets() {
            for object in sheet.objects() {
                if !object
                    .picture_flags()
                    .is_some_and(|flags| flags.contains(ObjPictureFlags::CONTROL_STREAM))
                {
                    continue;
                }
                increment_metric(category, "activex.host_objects", 1);
                increment_metric(category, "activex.relationship.typed.units", 1);
                match inventory.resolve_object_persistence_compatible(&view, object) {
                    Ok(Some(XlsObjectPersistenceRef::ControlStream { data, .. })) => {
                        increment_metric(category, "activex.resolved_persistence.units", 1);
                        increment_metric(
                            category,
                            "activex.resolved_persistence.bytes",
                            data.len() as u64,
                        );
                        increment_metric(category, "activex.payload.external_leaf.units", 1);
                        increment_metric(
                            category,
                            "activex.payload.external_leaf.bytes",
                            data.len() as u64,
                        );
                    }
                    Ok(Some(_)) | Ok(None) | Err(_) => {
                        increment_metric(category, "activex.unresolved_persistence.units", 1);
                        increment_metric(category, "activex.relationship.malformed.units", 1);
                    }
                }
            }
        }
    }
}

fn forms_site_class_name(class: SiteClassIndex) -> String {
    match class {
        SiteClassIndex::Cached(class) => match class {
            CachedControlClass::Form => "cached_form".to_owned(),
            CachedControlClass::Image => "cached_image".to_owned(),
            CachedControlClass::Frame => "cached_frame".to_owned(),
            CachedControlClass::MorphDataLegacy => "cached_morph_data_legacy".to_owned(),
            CachedControlClass::SpinButton => "cached_spin_button".to_owned(),
            CachedControlClass::CommandButton => "cached_command_button".to_owned(),
            CachedControlClass::TabStrip => "cached_tab_strip".to_owned(),
            CachedControlClass::Label => "cached_label".to_owned(),
            CachedControlClass::TextBox => "cached_text_box".to_owned(),
            CachedControlClass::ListBox => "cached_list_box".to_owned(),
            CachedControlClass::ComboBox => "cached_combo_box".to_owned(),
            CachedControlClass::CheckBox => "cached_check_box".to_owned(),
            CachedControlClass::OptionButton => "cached_option_button".to_owned(),
            CachedControlClass::ToggleButton => "cached_toggle_button".to_owned(),
            CachedControlClass::ScrollBar => "cached_scroll_bar".to_owned(),
            CachedControlClass::MultiPage => "cached_multi_page".to_owned(),
            CachedControlClass::Compatibility(value) => {
                format!("compatibility_0x{value:04x}")
            }
        },
        SiteClassIndex::ClassTable(index) => format!("class_table_{index}"),
        SiteClassIndex::Invalid => "invalid".to_owned(),
    }
}

fn forms_persistence_name(persistence: &FormControlPersistence) -> &'static str {
    match persistence {
        FormControlPersistence::Image(_) => "image",
        FormControlPersistence::Label(_) => "label",
        FormControlPersistence::SpinButton(_) => "spin_button",
        FormControlPersistence::ScrollBar(_) => "scroll_bar",
        FormControlPersistence::CommandButton(_) => "command_button",
        FormControlPersistence::TabStrip(_) => "tab_strip",
        FormControlPersistence::MorphData(_) => "morph_data",
        FormControlPersistence::ExternalClass(_) => "external_class",
    }
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
        record_office_art_disposition(category, record);
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
    stream.visit_complete(|record| record_office_art_disposition(category, record));
    increment_disposition(
        category,
        Disposition::Malformed,
        "records",
        stream.incomplete_record_count() as u64,
    );
    increment_disposition(
        category,
        Disposition::Malformed,
        "bytes",
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

fn record_office_art_disposition(
    category: &mut CoverageCategoryReport,
    record: &olecfsdk::office_art::OfficeArtRecord,
) {
    let disposition = match &record.data {
        OfficeArtRecordData::Atom(payload) if record.header.record_type == 0xf004 => {
            increment_disposition(
                category,
                Disposition::Malformed,
                "bytes",
                payload.len() as u64,
            );
            increment_metric(category, "debt.malformed.type_0xf004.records", 1);
            increment_metric(
                category,
                "debt.malformed.type_0xf004.bytes",
                payload.len() as u64,
            );
            Disposition::Malformed
        }
        OfficeArtRecordData::Atom(payload) => {
            increment_disposition(
                category,
                Disposition::UnknownExtension,
                "bytes",
                payload.len() as u64,
            );
            increment_metric(
                category,
                &format!(
                    "debt.unknown_extension.type_0x{:04x}.records",
                    record.header.record_type
                ),
                1,
            );
            increment_metric(
                category,
                &format!(
                    "debt.unknown_extension.type_0x{:04x}.bytes",
                    record.header.record_type
                ),
                payload.len() as u64,
            );
            Disposition::UnknownExtension
        }
        OfficeArtRecordData::CompatibilityContainer(_)
        | OfficeArtRecordData::EmptyCompatibilityAtom => Disposition::Compatibility,
        OfficeArtRecordData::IncompletePropertyTable(_) => Disposition::Malformed,
        OfficeArtRecordData::BitmapBlip(_) | OfficeArtRecordData::MetafileBlip(_) => {
            Disposition::ExternalLeaf
        }
        _ => Disposition::Typed,
    };
    increment_disposition(category, disposition, "records", 1);
}

fn increment_metric(category: &mut CoverageCategoryReport, metric: &str, value: u64) {
    *category.metrics.entry(metric.to_owned()).or_default() += value;
}

fn increment_disposition(
    category: &mut CoverageCategoryReport,
    disposition: Disposition,
    measure: &str,
    value: u64,
) {
    increment_metric(
        category,
        &format!("disposition.{}.{measure}", disposition.name()),
        value,
    );
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
        if let Some(reason) = exclusions.get(&path) {
            category.counts.excluded += 1;
            *category
                .exclusion_reasons
                .entry(reason.clone())
                .or_default() += 1;
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
        (spec.record_metrics)(&bytes, category);
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

fn record_cfb_metrics(bytes: &[u8], category: &mut CoverageCategoryReport) {
    increment_metric(category, "audited_input_bytes", bytes.len() as u64);
    let Ok(compound) = CompoundFile::from_bytes(bytes) else {
        return;
    };
    increment_metric(category, "entries", compound.entries().len() as u64);
    increment_disposition(
        category,
        Disposition::Typed,
        "units",
        compound.entries().len() as u64,
    );
    increment_metric(
        category,
        "streams",
        compound
            .entries()
            .iter()
            .filter(|entry| entry.is_stream())
            .count() as u64,
    );
    increment_metric(
        category,
        "storages_including_roots",
        compound
            .entries()
            .iter()
            .filter(|entry| entry.is_storage())
            .count() as u64,
    );
    increment_metric(
        category,
        "stream_bytes",
        compound
            .entries()
            .iter()
            .map(|entry| entry.data.len() as u64)
            .sum(),
    );
    increment_disposition(
        category,
        Disposition::ExternalLeaf,
        "units",
        compound
            .entries()
            .iter()
            .filter(|entry| entry.is_stream())
            .count() as u64,
    );
    increment_disposition(
        category,
        Disposition::ExternalLeaf,
        "bytes",
        compound
            .entries()
            .iter()
            .map(|entry| entry.data.len() as u64)
            .sum(),
    );
}

fn record_doc_metrics(bytes: &[u8], category: &mut CoverageCategoryReport) {
    increment_metric(category, "audited_input_bytes", bytes.len() as u64);
    let Ok(outcome) = DocFile::from_bytes_compatible(bytes) else {
        return;
    };
    let file = &outcome.value;
    record_doc_typed_nodes(
        category,
        "text_piece",
        file.word_document.text_pieces.len() as u64,
    );
    record_doc_typed_nodes(
        category,
        "character_format_run",
        file.word_document
            .chpx_runs
            .as_ref()
            .map_or(0, |runs| runs.len() as u64),
    );
    record_doc_typed_nodes(
        category,
        "paragraph_format_run",
        file.word_document
            .papx_runs
            .as_ref()
            .map_or(0, |runs| runs.len() as u64),
    );
    record_doc_typed_nodes(
        category,
        "character_fkp_page",
        file.word_document.chpx_fkp_pages().len() as u64,
    );
    record_doc_typed_nodes(
        category,
        "paragraph_fkp_page",
        file.word_document.papx_fkp_pages().len() as u64,
    );
    record_doc_typed_nodes(
        category,
        "section_properties",
        file.word_document.section_properties.len() as u64,
    );
    record_doc_table_nodes(category, file);
    increment_metric(
        category,
        "text_characters",
        file.word_document
            .text_pieces
            .iter()
            .map(|piece| piece.value.characters.character_count() as u64)
            .sum(),
    );
    increment_metric(
        category,
        "compatibility_tables",
        file.table.compatibility_tables.len() as u64,
    );
    for table in &file.table.compatibility_tables {
        let physical_bytes = table
            .physical_bytes
            .as_ref()
            .map_or(0, |bytes| bytes.len() as u64);
        let (disposition, debt_family) = doc_table_debt(&table.label, &table.reason);
        record_doc_debt_node(category, disposition, &debt_family, physical_bytes);
    }
    record_doc_data_nodes(category, file);
    record_doc_object_pool_nodes(category, file);
}

fn record_doc_table_nodes(category: &mut CoverageCategoryReport, file: &DocFile) {
    let table = &file.table;
    for name in [
        "clx",
        "character_bin_table",
        "paragraph_bin_table",
        "sections",
    ] {
        record_doc_typed_nodes(category, name, 1);
    }
    macro_rules! optional_node {
        ($field:ident) => {
            record_doc_typed_nodes(
                category,
                stringify!($field),
                u64::from(table.$field.is_some()),
            );
        };
    }
    optional_node!(styles);
    optional_node!(fonts);
    record_doc_typed_nodes(category, "field_table", table.fields.len() as u64);
    optional_node!(bookmarks);
    optional_node!(header_text);
    optional_node!(footnotes);
    optional_node!(endnotes);
    optional_node!(annotations);
    optional_node!(annotation_owners);
    optional_node!(annotation_bookmarks);
    optional_node!(annotation_extended_data);
    record_doc_typed_nodes(
        category,
        "textbox_story_table",
        table.textbox_stories.len() as u64,
    );
    record_doc_typed_nodes(
        category,
        "textbox_break_table",
        table.textbox_breaks.len() as u64,
    );
    record_doc_typed_nodes(
        category,
        "shape_anchor_table",
        table.shape_anchors.len() as u64,
    );
    optional_node!(office_art);
    optional_node!(revision_authors);
    optional_node!(captions);
    optional_node!(subdocuments);
    optional_node!(user_variables);
    optional_node!(embedded_fonts);
    optional_node!(spelling_state);
    optional_node!(grammar_state);
    optional_node!(language_detection_state);
    optional_node!(list_definitions);
    optional_node!(list_names);
    optional_node!(list_overrides);
    optional_node!(document_properties);
    optional_node!(associated_strings);
    optional_node!(external_file_names);
    optional_node!(mail_merge_state);
    optional_node!(new_mail_merge_state);
    optional_node!(office_data_source);
    optional_node!(printer_driver_info);
    optional_node!(ole_control_infos);
    optional_node!(table_character_cache);
    optional_node!(revision_message_threading);
    optional_node!(list_style_templates);
    optional_node!(frame_and_list_records);
    optional_node!(grammar_option_sets);
    optional_node!(legacy_grammar_option_sets);
    optional_node!(auto_summary_ranges);
    optional_node!(smart_tag_recognizer_state);
    optional_node!(xml_schema_references);
    optional_node!(xml_transform_path);
    optional_node!(paragraph_group_properties);
    optional_node!(save_history);
    optional_node!(grammar_checker_cookies);
    optional_node!(legacy_grammar_checker_cookies);
    optional_node!(grammar_cookie_data);
    optional_node!(smart_tag_data);
    optional_node!(revision_save_ids);
    optional_node!(selection_state);
    optional_node!(command_customizations);
    optional_node!(structured_tag_bookmarks);
    optional_node!(range_protection);
    optional_node!(smart_tag_bookmarks);
    optional_node!(format_consistency_bookmarks);
    optional_node!(repair_bookmarks);
    optional_node!(user_input_methods);
    optional_node!(mso_envelope);
    if let Some(cache) = &table.deprecated_numbering_field_cache {
        record_doc_debt_node(
            category,
            Disposition::SpecificationOpaque,
            "debt.specification_opaque.deprecated_numbering_field_cache",
            cache.physical_bytes.len() as u64,
        );
    }
}

fn record_doc_data_nodes(category: &mut CoverageCategoryReport, file: &DocFile) {
    let Some(data) = &file.data else {
        return;
    };
    increment_metric(category, "data_streams", 1);
    increment_metric(
        category,
        "data_stream_bytes",
        data.physical_bytes.len() as u64,
    );
    for node in &data.nodes {
        let name = match node.value {
            DocDataNodeValue::Picture(_) => "data_picture",
            DocDataNodeValue::Binary(_) => "data_binary",
            DocDataNodeValue::ParagraphProperties(_) => "data_paragraph_properties",
        };
        record_doc_typed_nodes(category, name, 1);
    }
}

fn record_doc_object_pool_nodes(category: &mut CoverageCategoryReport, file: &DocFile) {
    let Some(pool) = &file.object_pool else {
        return;
    };
    increment_metric(category, "object_pools", 1);
    for object in &pool.objects {
        record_doc_typed_nodes(category, "embedded_object_descriptor", 1);
        for path in &object.entry_paths {
            if path == &object.path || path == &object.descriptor_stream_path {
                continue;
            }
            let Some(entry) = file.source_compound_file().entry(path) else {
                continue;
            };
            if entry.is_stream() {
                record_doc_external_node(
                    category,
                    "embedded_object_external_stream",
                    entry.data.len() as u64,
                );
            }
        }
    }
    for object in &pool.compatibility_objects {
        let bytes = object
            .entry_paths
            .iter()
            .filter_map(|path| file.source_compound_file().entry(path))
            .filter(|entry| entry.is_stream())
            .map(|entry| entry.data.len() as u64)
            .sum();
        let (disposition, debt_family) = doc_object_debt(&object.reason);
        record_doc_debt_node(category, disposition, debt_family, bytes);
    }
}

fn doc_table_debt(label: &str, reason: &str) -> (Disposition, String) {
    if let Some(version) = reason.split("unknown MsoEnvelope version ").nth(1) {
        return (
            Disposition::UnknownExtension,
            format!(
                "debt.unknown_extension.table.msoenvelope.version_{}",
                metric_component(version)
            ),
        );
    }
    if let Some(record_type) = reason.split("unsupported Tcg255 record ").nth(1) {
        return (
            Disposition::UnknownExtension,
            format!(
                "debt.unknown_extension.table.cmds.record_{}",
                metric_component(record_type)
            ),
        );
    }
    (
        Disposition::Malformed,
        format!("debt.malformed.table.{}", metric_component(label)),
    )
}

fn doc_object_debt(reason: &str) -> (Disposition, &'static str) {
    if reason.contains("has no ObjInfo stream") {
        (
            Disposition::Malformed,
            "debt.malformed.object.missing_obj_info",
        )
    } else if reason.contains("unknown") {
        (
            Disposition::UnknownExtension,
            "debt.unknown_extension.object.descriptor",
        )
    } else {
        (Disposition::Malformed, "debt.malformed.object.descriptor")
    }
}

fn record_doc_typed_nodes(category: &mut CoverageCategoryReport, name: &str, count: u64) {
    if count == 0 {
        return;
    }
    increment_metric(category, "content_nodes", count);
    increment_metric(category, &format!("node_type.{name}.units"), count);
    increment_disposition(category, Disposition::Typed, "units", count);
}

fn record_doc_external_node(category: &mut CoverageCategoryReport, name: &str, bytes: u64) {
    increment_metric(category, "content_nodes", 1);
    increment_metric(category, &format!("node_type.{name}.units"), 1);
    increment_metric(category, &format!("node_type.{name}.bytes"), bytes);
    increment_disposition(category, Disposition::ExternalLeaf, "units", 1);
    increment_disposition(category, Disposition::ExternalLeaf, "bytes", bytes);
}

fn record_doc_debt_node(
    category: &mut CoverageCategoryReport,
    disposition: Disposition,
    debt_family: &str,
    bytes: u64,
) {
    increment_metric(category, "content_nodes", 1);
    increment_metric(category, &format!("{debt_family}.units"), 1);
    increment_metric(category, &format!("{debt_family}.bytes"), bytes);
    increment_disposition(category, disposition, "units", 1);
    increment_disposition(category, disposition, "bytes", bytes);
}

fn metric_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn record_xls_metrics(bytes: &[u8], category: &mut CoverageCategoryReport) {
    increment_metric(category, "audited_input_bytes", bytes.len() as u64);
    let Ok(outcome) = XlsFile::from_bytes_compatible(bytes) else {
        return;
    };
    increment_metric(category, "workbooks", outcome.value.workbooks.len() as u64);
    increment_metric(
        category,
        "biff_records",
        outcome
            .value
            .workbooks
            .iter()
            .map(|workbook| workbook.tree.stream.records.len() as u64)
            .sum(),
    );
    for workbook in &outcome.value.workbooks {
        let legacy = !workbook.tree.stream.is_biff8();
        for record in &workbook.tree.stream.records {
            record_xls_disposition(category, &record.data, legacy);
        }
    }
    increment_metric(
        category,
        "biff_substreams",
        outcome
            .value
            .workbooks
            .iter()
            .map(|workbook| workbook.tree.substreams.len() as u64)
            .sum(),
    );
}

fn record_ppt_metrics(bytes: &[u8], category: &mut CoverageCategoryReport) {
    increment_metric(category, "audited_input_bytes", bytes.len() as u64);
    let Ok(outcome) = PptFile::from_bytes_compatible(bytes) else {
        return;
    };
    let mut record_count = 0u64;
    outcome.value.document.records.visit(&mut |record| {
        record_count += 1;
        record_ppt_disposition(category, &record.data);
    });
    increment_metric(category, "records", record_count);
    if let Some(pictures) = &outcome.value.pictures {
        let picture_records = match pictures {
            PicturesStream::Complete(stream) => stream.records.len(),
            PicturesStream::Compatibility { stream, .. } => stream.records.len(),
            PicturesStream::Partial(stream) => stream.complete_record_count(),
        };
        increment_metric(category, "picture_records", picture_records as u64);
    }
}

fn record_xls_disposition(
    category: &mut CoverageCategoryReport,
    data: &BiffRecordData,
    legacy: bool,
) {
    let disposition = match data {
        BiffRecordData::Unknown {
            record_type,
            payload,
        } if !legacy => {
            increment_disposition(
                category,
                Disposition::UnknownExtension,
                "bytes",
                payload.len() as u64,
            );
            increment_metric(
                category,
                &format!("debt.unknown_extension.type_0x{record_type:04x}.records"),
                1,
            );
            increment_metric(
                category,
                &format!("debt.unknown_extension.type_0x{record_type:04x}.bytes"),
                payload.len() as u64,
            );
            Disposition::UnknownExtension
        }
        BiffRecordData::Unknown { payload, .. } => {
            increment_disposition(
                category,
                Disposition::Compatibility,
                "bytes",
                payload.len() as u64,
            );
            Disposition::Compatibility
        }
        BiffRecordData::LegacyBof { payload } => {
            increment_disposition(
                category,
                Disposition::Compatibility,
                "bytes",
                payload.len() as u64,
            );
            Disposition::Compatibility
        }
        BiffRecordData::Encrypted { payload, .. } => {
            increment_disposition(
                category,
                Disposition::SpecificationOpaque,
                "bytes",
                payload.len() as u64,
            );
            Disposition::SpecificationOpaque
        }
        BiffRecordData::Formula4Compatibility(_)
        | BiffRecordData::ObjCompatibility { .. }
        | BiffRecordData::BoundSheet8Compatibility { .. }
        | BiffRecordData::XfCompatibility { .. }
        | BiffRecordData::ChartSeriesCompatibility { .. }
        | BiffRecordData::FontCompatibility { .. } => Disposition::Compatibility,
        BiffRecordData::ImData(_) => Disposition::ExternalLeaf,
        _ => Disposition::Typed,
    };
    increment_disposition(category, disposition, "records", 1);
}

fn record_ppt_disposition(category: &mut CoverageCategoryReport, data: &PptRecordData) {
    let disposition = match data {
        PptRecordData::TextChars(value) => {
            increment_metric(
                category,
                "text_code_units",
                value.encode_utf16().count() as u64,
            );
            Disposition::Typed
        }
        PptRecordData::TextBytes(value) => {
            increment_metric(category, "text_code_units", value.chars().count() as u64);
            Disposition::Typed
        }
        PptRecordData::CompatibilityTextChars(code_units)
        | PptRecordData::CompatibilityCString(code_units) => {
            increment_disposition(
                category,
                Disposition::Compatibility,
                "bytes",
                (code_units.len() as u64) * 2,
            );
            increment_metric(
                category,
                "compatibility_utf16_code_units",
                code_units.len() as u64,
            );
            Disposition::Compatibility
        }
        PptRecordData::CString(value) => {
            increment_metric(
                category,
                "cstring_code_units",
                value.encode_utf16().count() as u64,
            );
            Disposition::Typed
        }
        PptRecordData::Unknown(value) if value.record_type == 0 => {
            increment_disposition(
                category,
                Disposition::Compatibility,
                "bytes",
                value.body.len() as u64,
            );
            increment_metric(category, "debt.compatibility.type_0x0000.records", 1);
            increment_metric(
                category,
                "debt.compatibility.type_0x0000.bytes",
                value.body.len() as u64,
            );
            Disposition::Compatibility
        }
        PptRecordData::Unknown(value) => {
            increment_disposition(
                category,
                Disposition::UnknownExtension,
                "bytes",
                value.body.len() as u64,
            );
            increment_metric(
                category,
                &format!(
                    "debt.unknown_extension.type_0x{:04x}.records",
                    value.record_type
                ),
                1,
            );
            increment_metric(
                category,
                &format!(
                    "debt.unknown_extension.type_0x{:04x}.bytes",
                    value.record_type
                ),
                value.body.len() as u64,
            );
            Disposition::UnknownExtension
        }
        PptRecordData::MalformedSpecRecord(value) => {
            increment_disposition(
                category,
                Disposition::Malformed,
                "bytes",
                value.body.len() as u64,
            );
            Disposition::Malformed
        }
        PptRecordData::Truncated(bytes)
        | PptRecordData::MalformedTextMasterStyle(bytes)
        | PptRecordData::MalformedTextRuler(bytes)
        | PptRecordData::MalformedTextSpecialInfo(bytes)
        | PptRecordData::MalformedStyleTextProp9(bytes)
        | PptRecordData::MalformedTimeVariant(bytes) => {
            increment_disposition(
                category,
                Disposition::Malformed,
                "bytes",
                bytes.len() as u64,
            );
            Disposition::Malformed
        }
        PptRecordData::MalformedStyleTextProp(_) | PptRecordData::MalformedBlipEntity9 { .. } => {
            Disposition::Malformed
        }
        PptRecordData::UnresolvedStyleTextProp(bytes) => {
            increment_disposition(
                category,
                Disposition::TemporaryUntyped,
                "bytes",
                bytes.len() as u64,
            );
            Disposition::TemporaryUntyped
        }
        PptRecordData::HandoutCompatibility(value) => {
            increment_disposition(
                category,
                Disposition::Compatibility,
                "bytes",
                value.bytes.len() as u64,
            );
            increment_metric(category, "debt.compatibility.type_0x200a.records", 1);
            increment_metric(
                category,
                "debt.compatibility.type_0x200a.bytes",
                value.bytes.len() as u64,
            );
            Disposition::Compatibility
        }
        PptRecordData::FontEmbedDataBlob(bytes) | PptRecordData::EnvelopeData9(bytes) => {
            increment_disposition(
                category,
                Disposition::ExternalLeaf,
                "bytes",
                bytes.len() as u64,
            );
            Disposition::ExternalLeaf
        }
        PptRecordData::Metafile(_)
        | PptRecordData::MacPrintSettings(_)
        | PptRecordData::MacPageFormat(_) => Disposition::ExternalLeaf,
        _ => Disposition::Typed,
    };
    increment_disposition(category, disposition, "records", 1);
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
) -> Result<BTreeMap<PathBuf, String>, String> {
    let mut exclusions = BTreeMap::<PathBuf, BTreeSet<String>>::new();
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
                exclusions
                    .entry(root.join(expectation.file))
                    .or_default()
                    .insert(format!("{:?}: {}", expectation.mode, expectation.reason));
            }
        }
    }
    Ok(exclusions
        .into_iter()
        .map(|(path, reasons)| (path, reasons.into_iter().collect::<Vec<_>>().join(" | ")))
        .collect())
}

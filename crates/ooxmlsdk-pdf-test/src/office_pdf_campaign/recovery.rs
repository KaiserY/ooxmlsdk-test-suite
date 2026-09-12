//! Import successful, explicitly configured retries without replacing existing
//! successful references. Each adapter revision binds exact source/config/PDF hashes.

use super::*;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryRequest {
    configuration_id: String,
    worker_root: PathBuf,
    environment_path: PathBuf,
    reason: String,
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn same_host(original: &Value, revision: &Value) -> Result<(), String> {
    let mut original = original["environment"].clone();
    let mut revision = revision["environment"].clone();
    for value in [&mut original, &mut revision] {
        let object = value.as_object_mut().ok_or("missing environment object")?;
        for key in [
            "probe_script_sha256",
            "controller_script_sha256",
            "environment_helper_sha256",
        ] {
            object
                .remove(key)
                .ok_or_else(|| format!("missing adapter fingerprint {key}"))?;
        }
    }
    if original != revision {
        return Err("recovery changed the captured Office host, fonts, locale or printer".into());
    }
    Ok(())
}

pub(super) fn validate_assignment_environment(
    root: &Path,
    assignment: &CampaignAssignment,
    original_id: &str,
) -> Result<(), String> {
    if assignment.reference_environment_id == original_id {
        return Ok(());
    }
    let id = &assignment.reference_environment_id;
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid recovery environment identity".into());
    }
    let directory = root.join("corpus_pdf_conv");
    let revision = read_json(&directory.join("environments").join(format!("{id}.json")))?;
    same_host(&read_json(&directory.join("environment.json"))?, &revision)?;
    if revision["environment_id"] != *id
        || !revision["approved_references"]
            .as_array()
            .is_some_and(|references| {
                references.iter().any(|reference| {
                    reference["configuration_id"] == assignment.configuration_id
                        && reference["source_sha256"] == assignment.source_sha256
                })
            })
    {
        return Err(format!(
            "unapproved recovery configuration {}",
            assignment.source_key()
        ));
    }
    Ok(())
}

fn overrides(root: &Path) -> Result<Vec<CampaignAssignment>, String> {
    let path = root.join("corpus_pdf_conv/recovery-plan-overrides.jsonl");
    if path.exists() {
        read_plan(&path)
    } else {
        Ok(Vec::new())
    }
}

pub(super) fn apply_overrides(
    root: &Path,
    assignments: &mut [CampaignAssignment],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for recovered in overrides(root)? {
        let key = recovered.source_key();
        if !seen.insert(key.clone()) {
            return Err(format!("duplicate recovery override: {key}"));
        }
        let original = assignments
            .iter_mut()
            .find(|a| a.source_key() == key)
            .ok_or_else(|| format!("unknown recovered source: {key}"))?;
        if original.source_sha256 != recovered.source_sha256
            || original.source_bytes != recovered.source_bytes
            || original.source_extension != recovered.source_extension
            || original.family != recovered.family
            || original.pilot_300 != recovered.pilot_300
        {
            return Err(format!(
                "recovery changed source identity or pilot membership: {key}"
            ));
        }
        *original = recovered;
    }
    Ok(())
}

fn recovered_assignment(
    original: &CampaignAssignment,
    result: &ProbeResult,
    environment_id: String,
) -> Result<CampaignAssignment, String> {
    let mut assignment = original.clone();
    assignment.reference_environment_id = environment_id;
    assignment.office = result.options.clone();
    let requested = &mut assignment.requested;
    requested.ui_language = result.environment.ui_language.clone();
    requested.page_range =
        result
            .options
            .page_from
            .zip(result.options.page_to)
            .map(|(from, to)| {
                if from == to {
                    from.to_string()
                } else {
                    format!("{from}-{to}")
                }
            });
    requested.export_bookmarks = result.options.bookmarks != "none";
    if !requested.export_bookmarks {
        requested.open_bookmark_levels = None;
    }
    requested.tagged_pdf = result.options.tagged_pdf;
    if !requested.tagged_pdf {
        requested.pdf_ua_compliance = false;
        requested
            .standards
            .retain(|s| *s != CampaignPdfStandard::PdfUa1);
    }
    if !result.options.pdf_a_1 {
        requested
            .standards
            .retain(|s| *s != CampaignPdfStandard::PdfA3a);
    } else if !requested.standards.contains(&CampaignPdfStandard::PdfA3a) {
        requested.standards.push(CampaignPdfStandard::PdfA3a);
    }
    if requested.standards.is_empty() {
        requested.standards.push(CampaignPdfStandard::Pdf17);
    }
    let source_key = original.source_key();
    let file_name = Path::new(&original.file)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid source name")?;
    let options = requested.to_pdf_options(&source_key, file_name);
    let resolved = resolve_pdf_options(original.family.document_kind(), &options)
        .map_err(|e| e.to_string())?;
    assignment.effective = CampaignPdfOptions::from_effective(&resolved.effective)?;
    assignment.adjustments = resolved
        .adjustments
        .iter()
        .map(|a| CampaignAdjustment {
            feature: a.feature.to_string(),
            reason: a.reason.to_string(),
        })
        .collect();
    if assignment.effective.tagged_pdf != result.options.tagged_pdf
        && original.family != OfficeFamily::Excel
    {
        return Err("recovered tagging disagrees with candidate options".into());
    }
    validate_office_options(&assignment)?;
    let source = SourceIdentity {
        corpus: original.corpus.clone(),
        file: original.file.clone(),
        family: original.family,
        source_extension: original.source_extension.clone(),
        source_bytes: original.source_bytes,
        source_sha256: original.source_sha256.clone(),
    };
    assignment.configuration_id = configuration_id(
        &source,
        &assignment.requested,
        &assignment.effective,
        &assignment.adjustments,
        &assignment.office,
        &assignment.reference_environment_id,
        &assignment.reference_time_zone,
    )?;
    Ok(assignment)
}

/// The request contains only successful worker directories. Failed and zero-page
/// attempts remain diagnostics; this operation cannot replace a converted record.
pub fn import_reference_recoveries(root: &Path, request_path: &Path) -> Result<usize, String> {
    let requests: Vec<RecoveryRequest> =
        serde_json::from_value(read_json(request_path)?).map_err(|e| e.to_string())?;
    if requests.is_empty() {
        return Err("recovery selection is empty".into());
    }
    let conversion_root = root.join("corpus_pdf_conv");
    let plan_path = conversion_root.join("plan.jsonl");
    let mut assignments = read_plan(&plan_path)?;
    validate_assignments(root, &assignments)?;
    let original_environment = read_json(&conversion_root.join("environment.json"))?;
    let mut records = read_conversion_records(root)?;
    let mut updated_overrides: BTreeMap<_, _> = overrides(root)?
        .into_iter()
        .map(|a| (a.source_key(), a))
        .collect();
    let mut revisions = BTreeMap::<String, Value>::new();
    let mut histories = Vec::new();
    let mut seen = BTreeSet::new();
    let stage = root.join("target/office-pdf-campaign").join(format!(
        "recovery-import-{}",
        jiff::Timestamp::now().as_nanosecond()
    ));
    fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    for request in &requests {
        if request.reason.trim().is_empty() || !seen.insert(&request.configuration_id) {
            return Err("recovery needs a reason and unique original configuration".into());
        }
        let assignment = assignments
            .iter_mut()
            .find(|a| a.configuration_id == request.configuration_id)
            .ok_or_else(|| {
                format!(
                    "unknown original configuration {}",
                    request.configuration_id
                )
            })?;
        let key = (assignment.corpus.clone(), assignment.file.clone());
        let old_record = records.get(&key).ok_or("missing failed reference record")?;
        validate_conversion_record(assignment, old_record)?;
        if !matches!(old_record.status.as_str(), "failed" | "timeout") {
            return Err(format!(
                "refusing to replace successful reference {}",
                assignment.source_key()
            ));
        }
        let worker = &request.worker_root;
        let completion = read_json(&worker.join("result.json"))?;
        let restoration = read_json(&worker.join("environment-restored.json"))?;
        if completion["exit_code"] != 0
            || completion["restored"] != true
            || restoration["restored"] != true
        {
            return Err(format!(
                "worker did not complete and restore: {}",
                worker.display()
            ));
        }
        let environment = read_json(&request.environment_path)?;
        same_host(&original_environment, &environment)?;
        let adapter = read_json(&worker.join("adapter-evidence.json"))?;
        for name in [
            "probe_script_sha256",
            "controller_script_sha256",
            "environment_helper_sha256",
        ] {
            if adapter[name].as_str().is_none() || adapter[name] != environment["environment"][name]
            {
                return Err(format!("worker adapter fingerprint drift: {name}"));
            }
        }
        let environment_id = environment["environment_id"]
            .as_str()
            .ok_or("missing recovery environment identity")?
            .to_string();
        if environment_id.len() != 64
            || !environment_id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("invalid recovery environment hash".into());
        }
        let probe_receipt = read_json(&worker.join("output/case-000.json"))?;
        let result: ProbeResult =
            serde_json::from_value(probe_receipt.clone()).map_err(|e| e.to_string())?;
        let new_assignment = recovered_assignment(assignment, &result, environment_id.clone())?;
        let pdf = fs::read(worker.join("output/case-000.pdf")).map_err(|e| e.to_string())?;
        let elapsed = completion["elapsed_ms"]
            .as_u64()
            .ok_or("missing worker duration")?;
        let mut record = converted_record(
            &stage,
            &new_assignment,
            &environment_id,
            1,
            elapsed,
            result,
            &pdf,
        )?;
        record.annotations.push(
            json!({"kind": "reference-recovery", "reason": request.reason,
            "previous_configuration_id": request.configuration_id}),
        );
        validate_conversion_record(&new_assignment, &record)?;
        if environment_id != original_environment["environment_id"] {
            let revision_path = conversion_root
                .join("environments")
                .join(format!("{environment_id}.json"));
            if !revisions.contains_key(&environment_id) {
                let mut revision = if revision_path.exists() {
                    read_json(&revision_path)?
                } else {
                    environment.clone()
                };
                if revision["environment"] != environment["environment"] {
                    return Err("recovery capture drift".into());
                }
                if revision.get("approved_references").is_none() {
                    revision["approved_references"] = json!([]);
                }
                revisions.insert(environment_id.clone(), revision);
            }
            revisions.get_mut(&environment_id).unwrap()["approved_references"].as_array_mut().ok_or("invalid recovery approvals")?.push(json!({
                "configuration_id": record.configuration_id, "source_sha256": record.source_sha256,
                "output_sha256": record.output_sha256,
            }));
        }
        histories.push(
            json!({"original_assignment": assignment, "original_conversion": old_record,
            "recovered_assignment": new_assignment, "reason": request.reason,
            "adapter": adapter, "worker_completion": completion,
            "environment_restoration": restoration, "probe_receipt": probe_receipt,
            "stages": fs::read_to_string(worker.join("stages.jsonl")).map_err(|e| e.to_string())?,
            "worker_root": worker, "environment_path": request.environment_path}),
        );
        updated_overrides.insert(new_assignment.source_key(), new_assignment.clone());
        *assignment = new_assignment;
        records.insert(key, record);
    }
    // Finish all PDF/configuration checks before publishing the transition. Keep
    // the prepared plan and receipts for completing an interrupted publication.
    write_plan(&stage.join("plan.jsonl"), &assignments)?;
    for (corpus, _) in records.keys() {
        fs::create_dir_all(stage.join("corpus_pdf_conv").join(corpus))
            .map_err(|e| e.to_string())?;
    }
    write_conversion_manifests(&stage, &records)?;
    write_json(&stage.join("transition.json"), &histories)?;
    // Validate the complete proposed registry before touching published references.
    write_json(
        &stage.join("corpus_pdf_conv/environment.json"),
        &original_environment,
    )?;
    let existing_revisions = conversion_root.join("environments");
    if existing_revisions.exists() {
        for entry in fs::read_dir(&existing_revisions).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                write_json(
                    &stage
                        .join("corpus_pdf_conv/environments")
                        .join(path.file_name().unwrap()),
                    &read_json(&path)?,
                )?;
            }
        }
    }
    for (id, revision) in &revisions {
        write_json(
            &stage
                .join("corpus_pdf_conv/environments")
                .join(format!("{id}.json")),
            revision,
        )?;
    }
    let prepared_environments =
        crate::office_reference_environment::ReferenceEnvironments::load(&stage)?;
    for record in records.values() {
        validate_audit_reference_environment(&prepared_environments, record)?;
    }
    fs::create_dir_all(conversion_root.join("environments")).map_err(|e| e.to_string())?;
    fs::create_dir_all(conversion_root.join("reference-recovery-history"))
        .map_err(|e| e.to_string())?;
    for (id, revision) in revisions {
        write_json(
            &conversion_root
                .join("environments")
                .join(format!("{id}.json")),
            &revision,
        )?;
    }
    for history in &histories {
        let id = history["original_assignment"]["configuration_id"]
            .as_str()
            .unwrap();
        write_json(
            &conversion_root
                .join("reference-recovery-history")
                .join(format!("{id}.json")),
            history,
        )?;
        let assignment: CampaignAssignment =
            serde_json::from_value(history["recovered_assignment"].clone())
                .map_err(|e| e.to_string())?;
        let output = PathBuf::from("corpus_pdf_conv")
            .join(&assignment.corpus)
            .join(format!("{}.pdf", assignment.file));
        promote_bytes(
            &root.join(&output),
            &fs::read(stage.join(output)).map_err(|e| e.to_string())?,
        )?;
    }
    validate_assignments(root, &assignments)?;
    write_conversion_manifests(root, &records)?;
    write_json_lines(
        &conversion_root.join("recovery-plan-overrides.jsonl"),
        &updated_overrides.into_values().collect::<Vec<_>>(),
    )?;
    write_plan(&plan_path, &assignments)?;
    let environments = crate::office_reference_environment::ReferenceEnvironments::load(root)?;
    for assignment in &assignments {
        let record = &records[&(assignment.corpus.clone(), assignment.file.clone())];
        validate_conversion_record(assignment, record)?;
        validate_audit_reference_environment(&environments, record)?;
    }
    Ok(requests.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_revision_cannot_change_the_office_host() {
        let original = json!({"environment": {
            "probe_script_sha256": "old", "controller_script_sha256": "controller",
            "environment_helper_sha256": "helper", "office": {"version": "16.0"},
            "fonts": {"sha256": "fonts"}, "locale": {"culture": "zh-CN"}
        }});
        let mut revision = original.clone();
        revision["environment"]["probe_script_sha256"] = json!("fixed");
        assert!(same_host(&original, &revision).is_ok());
        for property in ["office", "fonts", "locale"] {
            let mut drift = revision.clone();
            drift["environment"][property] = json!({"changed": true});
            assert!(same_host(&original, &drift).is_err());
        }
        revision["environment"]
            .as_object_mut()
            .unwrap()
            .remove("controller_script_sha256");
        assert!(same_host(&original, &revision).is_err());
    }

    #[test]
    fn recovery_environment_must_bind_the_exact_configuration_and_source() {
        let root = std::env::temp_dir().join(format!(
            "ooxmlsdk-recovery-test-{}",
            jiff::Timestamp::now().as_nanosecond()
        ));
        let original = json!({"environment": {"probe_script_sha256": "old",
            "controller_script_sha256": "controller", "environment_helper_sha256": "helper"}});
        let mut revision = original.clone();
        revision["environment_id"] = json!("b".repeat(64));
        let mut assignment = read_plan(&crate::workspace_root().join("corpus_pdf_conv/plan.jsonl"))
            .unwrap()
            .remove(0);
        assignment.reference_environment_id = "b".repeat(64);
        revision["approved_references"] = json!([{"configuration_id": assignment.configuration_id,
            "source_sha256": assignment.source_sha256, "output_sha256": "c".repeat(64)}]);
        write_json(&root.join("corpus_pdf_conv/environment.json"), &original).unwrap();
        write_json(
            &root
                .join("corpus_pdf_conv/environments")
                .join(format!("{}.json", "b".repeat(64))),
            &revision,
        )
        .unwrap();
        assert!(validate_assignment_environment(&root, &assignment, &"a".repeat(64)).is_ok());
        let saved_id = assignment.configuration_id.clone();
        assignment.configuration_id = "d".repeat(64);
        assert!(validate_assignment_environment(&root, &assignment, &"a".repeat(64)).is_err());
        assignment.configuration_id = saved_id;
        assignment.source_sha256 = "d".repeat(64);
        assert!(validate_assignment_environment(&root, &assignment, &"a".repeat(64)).is_err());
        // Temporary test outputs are retained as diagnostics; imported corpus is never written.
    }
}

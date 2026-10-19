//! Explicitly scoped Office reference revisions. The original batch remains
//! authoritative unless an additional environment binds an exact reference.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovedReference {
    configuration_id: String,
    source_sha256: String,
    output_sha256: String,
}

#[derive(Debug)]
pub(crate) struct ReferenceEnvironment {
    pub culture: String,
    pub ui_culture: String,
    pub time_zone: String,
    pub office_version: String,
    approved_references: Option<Vec<ApprovedReference>>,
}

impl ReferenceEnvironment {
    pub(crate) fn is_scoped_revision(&self) -> bool {
        self.approved_references.is_some()
    }
}

#[derive(Debug, Default)]
pub(crate) struct ReferenceEnvironments(BTreeMap<String, ReferenceEnvironment>);

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl ReferenceEnvironments {
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        let conversion_root = root.join("corpus_pdf_conv");
        let mut result = Self::default();
        result.insert_file(&conversion_root.join("environment.json"), true)?;
        let revisions = conversion_root.join("environments");
        match fs::read_dir(&revisions) {
            Ok(entries) => {
                for entry in entries {
                    let path = entry.map_err(|error| error.to_string())?.path();
                    if path
                        .extension()
                        .is_some_and(|extension| extension == "json")
                    {
                        result.insert_file(&path, false)?;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("could not read {}: {error}", revisions.display())),
        }
        Ok(result)
    }

    fn insert_file(&mut self, path: &Path, original: bool) -> Result<(), String> {
        let bytes = fs::read(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid {}: {error}", path.display()))?;
        if !original
            && path.file_stem().and_then(|name| name.to_str()) != value["environment_id"].as_str()
        {
            return Err(format!(
                "reference environment filename mismatch: {}",
                path.display()
            ));
        }
        self.insert_document(&value, original)
            .map_err(|error| format!("{}: {error}", path.display()))
    }

    fn insert_document(&mut self, value: &Value, original: bool) -> Result<(), String> {
        let string = |pointer: &str| {
            value
                .pointer(pointer)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("missing {pointer}"))
        };
        let id = string("/environment_id")?;
        if !is_sha256(&id)
            || value["schema_version"] != 1
            || value
                .pointer("/environment/reference_engine")
                .and_then(Value::as_str)
                != Some("Microsoft Office")
        {
            return Err("invalid Office reference environment identity".into());
        }
        let approved_references = if original {
            None
        } else {
            let approved: Vec<ApprovedReference> =
                serde_json::from_value(value["approved_references"].clone())
                    .map_err(|error| format!("invalid approved_references: {error}"))?;
            let mut identities = std::collections::BTreeSet::new();
            if approved.is_empty()
                || approved.iter().any(|r| {
                    !is_sha256(&r.configuration_id)
                        || !is_sha256(&r.source_sha256)
                        || !is_sha256(&r.output_sha256)
                        || !identities.insert(&r.configuration_id)
                })
            {
                return Err(
                    "reference revision needs unique exact configuration/source/output identities"
                        .into(),
                );
            }
            Some(approved)
        };
        let environment = ReferenceEnvironment {
            culture: string("/environment/locale/culture")?,
            ui_culture: string("/environment/locale/ui_culture")?,
            time_zone: string("/environment/locale/time_zone")?,
            office_version: string("/environment/office/version_to_report")?,
            approved_references,
        };
        if self.0.contains_key(&id) {
            return Err(format!("duplicate reference environment {id}"));
        }
        self.0.insert(id, environment);
        Ok(())
    }

    pub(crate) fn get(
        &self,
        environment_id: &str,
        configuration_id: &str,
        source_sha256: &str,
        output_sha256: &str,
    ) -> Result<&ReferenceEnvironment, String> {
        let environment = self
            .0
            .get(environment_id)
            .ok_or_else(|| format!("unregistered reference environment {environment_id}"))?;
        if let Some(approved) = &environment.approved_references
            && !approved.iter().any(|r| {
                r.configuration_id == configuration_id
                    && r.source_sha256 == source_sha256
                    && r.output_sha256 == output_sha256
            })
        {
            return Err(format!(
                "reference revision is not approved for the exact identity {configuration_id}"
            ));
        }
        Ok(environment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document() -> Value {
        json!({"schema_version": 1, "environment_id": "a".repeat(64),
            "environment": {"reference_engine": "Microsoft Office",
                "office": {"version_to_report": "16.0.20326.12345"},
                "locale": {"culture": "zh-CN", "ui_culture": "zh-CN", "time_zone": "China Standard Time"}},
            "approved_references": [{"configuration_id": "b".repeat(64),
                "source_sha256": "c".repeat(64), "output_sha256": "d".repeat(64)}]})
    }

    #[test]
    fn revised_environment_requires_all_three_exact_reference_identities() {
        let mut environments = ReferenceEnvironments::default();
        environments.insert_document(&document(), false).unwrap();
        let values = [
            "a".repeat(64),
            "b".repeat(64),
            "c".repeat(64),
            "d".repeat(64),
        ];
        assert!(
            environments
                .get(&values[0], &values[1], &values[2], &values[3])
                .is_ok()
        );
        for index in 0..4 {
            let mut changed = values.clone();
            changed[index] = "e".repeat(64);
            assert!(
                environments
                    .get(&changed[0], &changed[1], &changed[2], &changed[3])
                    .is_err()
            );
        }
        assert!(environments.insert_document(&document(), false).is_err());
    }

    #[test]
    fn revised_environment_rejects_empty_duplicate_or_invalid_approvals() {
        for approvals in [
            Value::Null,
            json!([]),
            json!([{}]),
            json!([
                document()["approved_references"][0],
                document()["approved_references"][0]
            ]),
        ] {
            let mut value = document();
            value["approved_references"] = approvals;
            assert!(
                ReferenceEnvironments::default()
                    .insert_document(&value, false)
                    .is_err()
            );
        }
    }

    #[test]
    fn original_environment_does_not_require_revision_approvals() {
        let mut value = document();
        value.as_object_mut().unwrap().remove("approved_references");
        let mut environments = ReferenceEnvironments::default();
        environments.insert_document(&value, true).unwrap();
        let environment = environments
            .get(
                &"a".repeat(64),
                &"e".repeat(64),
                &"f".repeat(64),
                &"0".repeat(64),
            )
            .unwrap();
        assert_eq!(environment.culture, "zh-CN");
        assert_eq!(environment.time_zone, "China Standard Time");
    }
}

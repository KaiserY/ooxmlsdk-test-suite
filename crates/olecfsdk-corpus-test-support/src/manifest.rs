use std::{fs, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CorpusManifest {
    #[serde(default)]
    pub expectation: Vec<Expectation>,
}

#[derive(Debug, Deserialize)]
pub struct Expectation {
    pub file: String,
    pub test: String,
    pub mode: ExpectationMode,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectationMode {
    RoundTrip,
    OpenOnly,
    Invalid,
    Unsupported,
    RequiresPassword,
    KnownFailure,
}

pub fn read_manifest(path: &Path) -> Result<CorpusManifest, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("parse {}: {err}", path.display()))
}

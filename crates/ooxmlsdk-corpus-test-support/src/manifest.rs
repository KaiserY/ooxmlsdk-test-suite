use std::{error::Error, fmt, fs, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CorpusManifest {
    #[serde(default)]
    pub corpus: Option<CorpusMetadata>,
    #[serde(default)]
    pub expectation: Vec<Expectation>,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceCorpusManifest {
    #[serde(default)]
    pub corpus: Vec<CorpusEntry>,
}

#[derive(Debug, Deserialize)]
pub struct CorpusMetadata {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub path: String,
    pub license: String,
    pub source: String,
    #[serde(default)]
    pub source_commit: Option<String>,
    #[serde(default)]
    pub default_roundtrip: bool,
}

#[derive(Debug, Deserialize)]
pub struct CorpusManifestLegacy {
    #[serde(default)]
    pub expectation: Vec<Expectation>,
}

#[derive(Debug, Deserialize)]
pub struct CorpusEntry {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub manifest: Option<String>,
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

pub fn read_manifest(path: &Path) -> Result<CorpusManifest, ManifestError> {
    let raw = fs::read_to_string(path).map_err(ManifestError::Read)?;
    toml::from_str(&raw).map_err(ManifestError::Parse)
}

#[derive(Debug)]
pub enum ManifestError {
    Read(std::io::Error),
    Parse(toml::de::Error),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => write!(f, "failed to read manifest: {err}"),
            Self::Parse(err) => write!(f, "failed to parse manifest: {err}"),
        }
    }
}

impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(err) => Some(err),
            Self::Parse(err) => Some(err),
        }
    }
}

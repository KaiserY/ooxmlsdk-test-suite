//! Shared helpers for corpus-based `ooxmlsdk` tests.

pub mod manifest;
pub mod roundtrip;

use std::path::{Path, PathBuf};

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn corpus_root() -> PathBuf {
    workspace_root().join("corpus")
}

pub fn corpus_file_path(relative_path: &str) -> PathBuf {
    corpus_root().join(relative_path)
}

pub fn workspace_relative_path(path: &Path) -> String {
    let root = workspace_root();
    path.strip_prefix(&root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfficeDocumentKind {
    Wordprocessing,
    Spreadsheet,
    Presentation,
}

pub fn office_document_kind(path_or_name: impl AsRef<Path>) -> Option<OfficeDocumentKind> {
    match path_or_name
        .as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("docx" | "dotx" | "docm" | "dotm") => Some(OfficeDocumentKind::Wordprocessing),
        Some("xlsx" | "xltx" | "xlsm" | "xltm") => Some(OfficeDocumentKind::Spreadsheet),
        Some("pptx" | "potx" | "pptm" | "potm") => Some(OfficeDocumentKind::Presentation),
        _ => None,
    }
}

//! PDF export test helpers for `ooxmlsdk-pdf`.
//!
//! This crate is intentionally separate from the runtime PDF converter. It
//! renders LibreOffice corpus DOCX/PPTX/XLSX fixtures through `ooxmlsdk-pdf`
//! and exposes PDFium/lopdf-based summaries for tests that mirror upstream
//! LibreOffice final PDF assertions.

pub mod pdf_extract;
pub mod render;

use std::path::{Path, PathBuf};

pub use pdf_extract::{
    AnnotationSummary, LinkTargetKind, PathObjectSummary, PdfBounds, PdfSummary, PixelRect,
    RawAnnotationSummary, RawPageSummary, RawXObjectSummary, RenderedPageImage,
    assert_pdf_rect_close, parse_pdf_rect, pdf_page_count, raw_image_pixel_from_pdf,
    rendered_page_image_from_pdf,
};
pub use render::render_fixture_pdf;

pub type Result<T> = std::result::Result<T, CalibrationError>;

#[derive(Debug, thiserror::Error)]
pub enum CalibrationError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ooxmlsdk-pdf conversion failed: {0}")]
    Pdf(#[from] ooxmlsdk_pdf::PdfError),
    #[error("PDFium extraction failed: {0}")]
    PdfiumExtraction(String),
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn pdfexport_fixture_dir() -> PathBuf {
    libreoffice_corpus_dir()
}

pub fn libreoffice_corpus_dir() -> PathBuf {
    workspace_root().join("corpus/LibreOffice")
}

pub fn libreoffice_fixture(path: &str) -> PathBuf {
    let root = libreoffice_corpus_dir();
    let direct = root.join(path);
    if direct.exists() {
        return direct;
    }

    if let Some(stripped) = path.strip_prefix("xlsx/") {
        let direct = root.join(stripped);
        if direct.exists() {
            return direct;
        }
    }

    let file_name = Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or(path);
    unique_corpus_file_named(file_name)
}

pub fn pdfexport_fixtures() -> Vec<PathBuf> {
    let mut fixtures = Vec::new();
    collect_office_documents(&pdfexport_fixture_dir(), &mut fixtures);
    fixtures.sort();
    fixtures
}

pub fn pdf_summary_for_fixture(fixture: &Path) -> Result<PdfSummary> {
    let pdf = render::render_fixture_pdf(fixture)?;
    PdfSummary::from_bytes(&pdf).map_err(CalibrationError::PdfiumExtraction)
}

pub fn pdf_summary_for_fixture_with_options(
    fixture: &Path,
    options: ooxmlsdk_pdf::PdfOptions,
) -> Result<PdfSummary> {
    let pdf = render::render_fixture_pdf_with_options(fixture, options)?;
    PdfSummary::from_bytes(&pdf).map_err(CalibrationError::PdfiumExtraction)
}

pub fn pdf_page_count_for_fixture(fixture: &Path) -> Result<usize> {
    let pdf = render_fixture_pdf(fixture)?;
    pdf_page_count(&pdf).map_err(CalibrationError::PdfiumExtraction)
}

pub fn rendered_page_image_for_fixture(
    fixture: &Path,
    page_index: usize,
    target_width: i32,
) -> Result<RenderedPageImage> {
    let pdf = render_fixture_pdf(fixture)?;
    rendered_page_image_from_pdf(&pdf, page_index, target_width)
        .map_err(CalibrationError::PdfiumExtraction)
}

pub fn raw_image_pixel_for_fixture(
    fixture: &Path,
    image_width: u32,
    image_height: u32,
    source_x: u32,
    source_y: u32,
) -> Result<Option<[u8; 4]>> {
    let pdf = render_fixture_pdf(fixture)?;
    raw_image_pixel_from_pdf(&pdf, image_width, image_height, source_x, source_y)
        .map_err(CalibrationError::PdfiumExtraction)
}

pub fn workspace_relative_path(path: &Path) -> String {
    let root = workspace_root();
    path.strip_prefix(&root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_office_documents(root: &Path, fixtures: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }

    for entry in walkdir::WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.into_path();
        if is_pdf_fixture_document(&path) {
            fixtures.push(path);
        }
    }
}

fn is_pdf_fixture_document(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "docx" | "docm" | "dotx" | "dotm" | "pptx" | "pptm" | "ppsx" | "ppsm" | "xlsx" | "xlsm",
        )
    )
}

fn unique_corpus_file_named(name: &str) -> PathBuf {
    let root = libreoffice_corpus_dir();
    let mut matches = Vec::new();
    if root.exists() {
        for entry in walkdir::WalkDir::new(&root)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
        {
            if entry.file_name().to_str() == Some(name) {
                matches.push(entry.into_path());
            }
        }
    }

    match matches.as_slice() {
        [path] => path.clone(),
        [] => root.join(name),
        paths => panic!("fixture name {name:?} is ambiguous in LibreOffice corpus: {paths:?}"),
    }
}

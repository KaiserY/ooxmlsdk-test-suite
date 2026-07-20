use std::fs::File;
use std::path::Path;

use ooxmlsdk_pdf::{
    PdfConversionOutput, PdfFontAuditOutput, PdfOptions, convert_docx,
    convert_docx_with_diagnostics, convert_docx_with_font_audit, convert_pptx,
    convert_pptx_with_diagnostics, convert_pptx_with_font_audit, convert_xlsx,
    convert_xlsx_with_diagnostics, convert_xlsx_with_font_audit,
};

use crate::Result;

pub fn render_fixture_pdf(fixture: &Path) -> Result<Vec<u8>> {
    let options = PdfOptions {
        source_file_name: fixture
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string),
        ..PdfOptions::default()
    };
    render_fixture_pdf_with_options(fixture, options)
}

pub fn render_fixture_pdf_with_options(fixture: &Path, options: PdfOptions) -> Result<Vec<u8>> {
    let file = File::open(fixture)?;
    match fixture.extension().and_then(|extension| extension.to_str()) {
        Some("pptx" | "pptm" | "ppsx" | "ppsm") => Ok(convert_pptx(file, options)?),
        Some("xlsx" | "xlsm") => Ok(convert_xlsx(file, options)?),
        _ => Ok(convert_docx(file, options)?),
    }
}

pub fn render_fixture_pdf_with_diagnostics(
    fixture: &Path,
    options: PdfOptions,
) -> Result<PdfConversionOutput> {
    let file = File::open(fixture)?;
    match fixture.extension().and_then(|extension| extension.to_str()) {
        Some("pptx" | "pptm" | "ppsx" | "ppsm") => {
            Ok(convert_pptx_with_diagnostics(file, options)?)
        }
        Some("xlsx" | "xlsm") => Ok(convert_xlsx_with_diagnostics(file, options)?),
        _ => Ok(convert_docx_with_diagnostics(file, options)?),
    }
}

pub fn render_fixture_pdf_with_font_audit(
    fixture: &Path,
    options: PdfOptions,
) -> Result<PdfFontAuditOutput> {
    let file = File::open(fixture)?;
    match fixture.extension().and_then(|extension| extension.to_str()) {
        Some("pptx" | "pptm" | "ppsx" | "ppsm") => Ok(convert_pptx_with_font_audit(file, options)?),
        Some("xlsx" | "xlsm") => Ok(convert_xlsx_with_font_audit(file, options)?),
        _ => Ok(convert_docx_with_font_audit(file, options)?),
    }
}

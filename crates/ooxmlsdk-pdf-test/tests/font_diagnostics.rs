use ooxmlsdk_pdf::{PdfConversionOutput, PdfFontAuditOutput, PdfOptions};
use ooxmlsdk_pdf_test::{libreoffice_fixture, pdf_font_structure};

fn render_with_diagnostics(path: &str) -> PdfConversionOutput {
    let fixture = libreoffice_fixture(path);
    ooxmlsdk_pdf_test::render::render_fixture_pdf_with_diagnostics(
        &fixture,
        PdfOptions {
            source_file_name: fixture
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string),
            ..Default::default()
        },
    )
    .unwrap()
}

fn render_with_font_audit(path: &str) -> PdfFontAuditOutput {
    let fixture = libreoffice_fixture(path);
    ooxmlsdk_pdf_test::render::render_fixture_pdf_with_font_audit(
        &fixture,
        PdfOptions {
            source_file_name: fixture
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string),
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
// Source: Krilla tests/src/text.rs snapshots require deterministic text serialization.
fn collecting_font_diagnostics_does_not_change_pdf_bytes() {
    let fixture = libreoffice_fixture("lastEmptyLineWithDirectFormatting.docx");
    let options = PdfOptions {
        source_file_name: fixture
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string),
        ..Default::default()
    };
    let ordinary =
        ooxmlsdk_pdf_test::render::render_fixture_pdf_with_options(&fixture, options.clone())
            .unwrap();
    let diagnostic =
        ooxmlsdk_pdf_test::render::render_fixture_pdf_with_diagnostics(&fixture, options).unwrap();
    assert_eq!(ordinary, diagnostic.pdf);
    let audited = render_with_font_audit("lastEmptyLineWithDirectFormatting.docx");
    assert_eq!(ordinary, audited.pdf);
    assert!(
        audited.audit.issues.is_empty(),
        "{:#?}",
        audited.audit.issues
    );
    assert!(audited.audit.glyph_count > 0);
}

#[test]
// Sources: MS-OI29500 17.8; core PDFWriter ToUnicode tests; Typst shaping invariants.
fn lightweight_font_audit_accepts_docx_pptx_and_xlsx_font_runs() {
    for fixture in [
        "lastEmptyLineWithDirectFormatting.docx",
        "pptx/trailing-paragraphs.pptx",
        "seconds-without-truncate-and-decimals.xlsx",
    ] {
        let output = render_with_font_audit(fixture);
        assert!(
            output.audit.issues.is_empty(),
            "{fixture}: {:#?}",
            output.audit.issues
        );
        assert!(!output.audit.fonts.is_empty(), "{fixture}");
        assert!(output.audit.glyph_count > 0, "{fixture}");
    }
}

fn assert_font_trace(output: &PdfConversionOutput, expected_text: &str) {
    assert!(
        String::from_utf8_lossy(&output.pdf)
            .trim_end()
            .ends_with("%%EOF")
    );
    assert!(!output.diagnostics.pages.is_empty());
    assert!(!output.diagnostics.fonts.is_empty());
    assert!(output.diagnostics.fonts.iter().all(|font| {
        font.data_len > 0
            && font.parse_error.is_none()
            && font.units_per_em > 0
            && font.glyph_count > 0
            && font.checksum_adjustment.is_some()
    }));
    let text_runs = output
        .diagnostics
        .pages
        .iter()
        .flat_map(|page| &page.text_runs)
        .collect::<Vec<_>>();
    assert!(text_runs.iter().any(|run| run.text.contains(expected_text)));
    assert!(text_runs.iter().any(|run| {
        run.portions
            .iter()
            .flat_map(|portion| &portion.glyph_runs)
            .flat_map(|run| &run.glyphs)
            .any(|glyph| glyph.bounds_em.is_some())
    }));

    let structure = pdf_font_structure(&output.pdf).unwrap();
    let fonts = structure
        .pages
        .iter()
        .flat_map(|page| &page.fonts)
        .collect::<Vec<_>>();
    assert!(!fonts.is_empty());
    assert!(fonts.iter().any(|font| font.embedded_font_kind.is_some()));
    assert!(fonts.iter().any(|font| font.has_to_unicode));
    assert!(fonts.iter().filter(|font| font.has_to_unicode).all(|font| {
        font.to_unicode_error.is_none()
            && font.to_unicode_mapping_count.is_some_and(|count| count > 0)
    }));
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport19.cxx
fn docx_font_diagnostics_capture_resolved_faces_and_glyph_geometry() {
    let output = render_with_diagnostics("lastEmptyLineWithDirectFormatting.docx");
    assert_font_trace(&output, "line");
}

#[test]
// Source: ../core/sd/qa/unit/layout-tests.cxx:testTdf168010_PPTX
fn pptx_font_diagnostics_capture_resolved_faces_and_glyph_geometry() {
    let output = render_with_diagnostics("pptx/trailing-paragraphs.pptx");
    assert_font_trace(&output, "textbox");
}

#[test]
// Source: ../core/sc/qa/unit/subsequent_export_test5.cxx:testSecondsWithoutTruncateAndDecimals
fn xlsx_font_diagnostics_capture_resolved_faces_and_glyph_geometry() {
    let output = render_with_diagnostics("seconds-without-truncate-and-decimals.xlsx");
    assert_font_trace(&output, "271433.61");
}

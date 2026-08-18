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
// Sources:
// - ../core/sw/qa/extras/layout/layout5.cxx::testTdf153136
// - ../core/sw/source/core/text/porlay.cxx::SwLineLayout::CalcLine
fn docx_writer_blank_line_metrics_survive_the_layout_to_pdf_boundary() {
    let output = render_with_diagnostics("sw/qa/extras/layout/data/tdf153136.docx");
    let line_metrics = |page_index: usize, label: &str| {
        let page = output
            .diagnostics
            .pages
            .get(page_index)
            .unwrap_or_else(|| panic!("missing page {page_index}"));
        let label_run = page
            .text_runs
            .iter()
            .find(|run| run.text == label)
            .unwrap_or_else(|| panic!("missing {label} on page {page_index}"));
        let blank_run = page
            .text_runs
            .iter()
            .find(|run| {
                run.text.is_empty()
                    && run.font_size_pt > 40.0
                    && run.source_frame_index == label_run.source_frame_index
                    && run.x_pt > label_run.x_pt
                    && if page_index == 0 {
                        run.source_line_index == label_run.source_line_index
                    } else {
                        label_run.source_line_index.is_some_and(|line_index| {
                            run.source_line_index == line_index.checked_add(1)
                        })
                    }
            })
            .unwrap_or_else(|| panic!("missing 48pt blank beside {label} on page {page_index}"));
        if page_index == 0 {
            assert!(
                (label_run.baseline_y_pt - blank_run.baseline_y_pt).abs() <= 0.01,
                "{label} and its blank must share the PDF baseline: label={label_run:#?}; blank={blank_run:#?}"
            );
        }
        (
            label_run.baseline_y_pt - label_run.y_pt,
            blank_run.line_height_pt,
        )
    };
    let assert_compact = |page_index: usize, label: &str| {
        let (baseline_offset, line_height) = line_metrics(page_index, label);
        assert!(
            line_height < 20.0,
            "{label} on page {page_index} must keep the paragraph-mark line height; height={line_height}"
        );
        if page_index == 0 {
            assert!(
                baseline_offset < 20.0,
                "{label} on page {page_index} must keep the paragraph-mark PDF baseline; offset={baseline_offset}"
            );
        }
    };
    let assert_tall = |page_index: usize, label: &str| {
        let (baseline_offset, line_height) = line_metrics(page_index, label);
        assert!(
            line_height > 40.0,
            "{label} on page {page_index} must keep the 48pt blank line height; height={line_height}"
        );
        if page_index == 0 {
            assert!(
                baseline_offset > 40.0,
                "{label} on page {page_index} must keep the 48pt blank PDF baseline; offset={baseline_offset}"
            );
        }
    };

    for page_index in [0, 1] {
        for label in ["U+0020", "U+2002", "U+2003", "U+2005"] {
            assert_compact(page_index, label);
        }
        for label in ["U+00A0", "U+2000", "U+2001", "U+2004", "U+2006"] {
            assert_tall(page_index, label);
        }
    }

    // An explicit 48pt paragraph mark makes even the otherwise ignored blank
    // classes tall. This is the counterexample that prevents a character-only
    // PDF workaround from passing the test.
    for label in ["U+0020", "U+2002", "U+2003", "U+2005"] {
        assert_tall(2, label);
    }
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

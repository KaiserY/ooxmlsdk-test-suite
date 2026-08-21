use std::collections::HashSet;
use std::fs;

use lopdf::{Document, Object};
use ooxmlsdk_pdf::{
    PdfDocumentKind, PdfError, PdfLinkDefaultAction, PdfOptimizeFor, PdfOptionFeature,
    PdfOptionSupport, PdfOptions, PdfPageLayout, PdfStandard, PdfViewerMagnification,
    PdfViewerPageMode, pdf_option_support, resolve_pdf_options,
};
use ooxmlsdk_pdf_test::{PdfSummary, libreoffice_fixture, rendered_page_image_from_pdf};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[test]
fn public_capability_matrix_is_document_kind_aware() {
    for kind in [
        PdfDocumentKind::Docx,
        PdfDocumentKind::Xlsx,
        PdfDocumentKind::Pptx,
    ] {
        assert_eq!(
            pdf_option_support(kind, PdfOptionFeature::PageRange),
            PdfOptionSupport::Supported
        );
        assert_eq!(
            pdf_option_support(kind, PdfOptionFeature::ViewerPreferences),
            PdfOptionSupport::Supported
        );
        assert_eq!(
            pdf_option_support(kind, PdfOptionFeature::OptimizeFor),
            PdfOptionSupport::Supported
        );
    }

    assert_eq!(PdfOptions::default().optimize_for, PdfOptimizeFor::Print);

    assert_eq!(
        pdf_option_support(PdfDocumentKind::Docx, PdfOptionFeature::FormFields),
        PdfOptionSupport::SupportedWithRestrictions
    );
    assert_eq!(
        pdf_option_support(PdfDocumentKind::Xlsx, PdfOptionFeature::FormFields),
        PdfOptionSupport::Unsupported
    );
    assert_eq!(
        pdf_option_support(PdfDocumentKind::Pptx, PdfOptionFeature::FormFields),
        PdfOptionSupport::Unsupported
    );
}

#[test]
fn public_resolver_records_effective_pdf_ua_options_and_typed_rejections() {
    let mut options = PdfOptions::default();
    options.standards.push(PdfStandard::PdfUa1);
    options.general.export_bookmarks = false;
    options.viewer.display_document_title = false;

    let resolved = resolve_pdf_options(PdfDocumentKind::Docx, &options).unwrap();
    assert!(resolved.effective.general.pdf_ua_compliance);
    assert!(resolved.effective.general.tagged_pdf);
    assert!(resolved.effective.general.export_bookmarks);
    assert!(resolved.effective.viewer.display_document_title);
    assert!(
        resolved
            .adjustments
            .iter()
            .any(|adjustment| adjustment.feature == PdfOptionFeature::TaggedPdf)
    );

    let mut unsupported = PdfOptions::default();
    unsupported.forms.export_form_fields = true;
    assert!(matches!(
        resolve_pdf_options(PdfDocumentKind::Xlsx, &unsupported),
        Err(PdfError::UnsupportedOption {
            feature: PdfOptionFeature::FormFields,
            document_kind: PdfDocumentKind::Xlsx,
            ..
        })
    ));

    unsupported.forms.export_form_fields = false;
    unsupported.links.default_action = PdfLinkDefaultAction::Launch;
    assert!(matches!(
        resolve_pdf_options(PdfDocumentKind::Pptx, &unsupported),
        Err(PdfError::UnsupportedOption {
            feature: PdfOptionFeature::Links,
            document_kind: PdfDocumentKind::Pptx,
            ..
        })
    ));
}

#[test]
// Sources:
// - ../core/vcl/source/pdf/pdfwriter_impl.cxx (catalog viewer preferences and open action)
// - ../core/tools/source/memtools/multisel.cxx (page-range sequence grammar)
// - Adobe PDF Reference 1.5, sections 8.1 and 8.2
fn public_docx_conversion_applies_page_range_and_viewer_options_to_pdf_objects() {
    let fixture = libreoffice_fixture("sw/qa/extras/layout/data/tdf153136.docx");
    let mut options = PdfOptions {
        source_file_name: fixture
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string),
        ..Default::default()
    };
    options.general.page_range = Some("2,1".to_string());
    options.metadata.title = Some("PDF option integration test".to_string());
    options.viewer.page_mode = PdfViewerPageMode::UseOutlines;
    options.viewer.page_layout = PdfPageLayout::ContinuousFacing;
    options.viewer.magnification = PdfViewerMagnification::FitVisible;
    options.viewer.initial_page = 2;
    options.viewer.hide_toolbar = true;
    options.viewer.center_window = true;
    options.viewer.first_page_left = true;

    let bytes =
        ooxmlsdk_pdf_test::render::render_fixture_pdf_with_options(&fixture, options).unwrap();
    let pdf = Document::load_mem(&bytes).unwrap();
    let pages = pdf.get_pages();
    assert_eq!(pages.len(), 2);

    let catalog_id = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let catalog = pdf.get_dictionary(catalog_id).unwrap();
    assert_eq!(
        catalog.get(b"PageMode").unwrap().as_name().unwrap(),
        b"UseOutlines"
    );
    assert_eq!(
        catalog.get(b"PageLayout").unwrap().as_name().unwrap(),
        b"TwoColumnRight"
    );

    let preferences = catalog
        .get(b"ViewerPreferences")
        .unwrap()
        .as_dict()
        .unwrap();
    assert!(preferences.get(b"HideToolbar").unwrap().as_bool().unwrap());
    assert!(preferences.get(b"CenterWindow").unwrap().as_bool().unwrap());
    assert!(
        preferences
            .get(b"DisplayDocTitle")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        preferences.get(b"Direction").unwrap().as_name().unwrap(),
        b"R2L"
    );

    let open_action = catalog.get(b"OpenAction").unwrap().as_array().unwrap();
    assert!(matches!(open_action[0], Object::Reference(id) if id == pages[&2]));
    assert_eq!(open_action[1].as_name().unwrap(), b"FitBH");
}

fn pilot_options(index: usize) -> (&'static str, PdfOptions) {
    match index {
        0 => {
            let mut options = PdfOptions::default();
            // Office 16.0.20228 records UseISO19005_1=true as PDF/A-3A in
            // XMP. Keep the observed conformance separate from the misleading
            // COM parameter name.
            options.standards.push(PdfStandard::PdfA3a);
            options.general.tagged_pdf = true;
            options.general.export_bookmarks = false;
            options.metadata.title = Some("blank_text".to_string());
            options.metadata.creation_date = Some(ooxmlsdk_pdf::PdfDateTime {
                year: 2026,
                month: Some(8),
                day: Some(17),
                hour: Some(12),
                minute: Some(0),
                second: Some(0),
                utc_offset_hour: Some(8),
                utc_offset_minute: Some(0),
            });
            options.default_document_language = Some("zh-CN".to_string());
            ("desktop/qa/data/blank_text.docx", options)
        }
        1 => {
            let mut options = PdfOptions::default();
            options.general.tagged_pdf = true;
            options.general.export_bookmarks = false;
            options.general.page_range = Some("2".to_string());
            ("sc/qa/unit/data/xlsx/autofilterShowButton.xlsx", options)
        }
        2 => {
            let mut options = PdfOptions::default();
            options.general.tagged_pdf = false;
            options.general.export_bookmarks = false;
            options.general.page_range = Some("2".to_string());
            ("sd/qa/unit/data/pptx/master-bg-color.pptx", options)
        }
        _ => unreachable!(),
    }
}

fn office_pilot_expectation(index: usize) -> (&'static str, Value) {
    let (application, page, pdf_a_1, tagged_pdf) = match index {
        0 => ("Word", None, true, true),
        1 => ("Excel", Some(2), false, true),
        2 => ("PowerPoint", Some(2), false, false),
        _ => unreachable!(),
    };
    (
        application,
        json!({
            "bitmap_missing_fonts": true,
            "bookmarks": "none",
            "include_document_properties": false,
            "page_from": page,
            "page_to": page,
            "pdf_a_1": pdf_a_1,
            "print_hidden_slides": false,
            "quality": "print",
            "tagged_pdf": tagged_pdf,
        }),
    )
}

fn office_pilot_plan() -> Vec<Value> {
    include_str!("../../../scripts/office_pdf_options_pilot.jsonl")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn office_pilot_plan_assigns_exactly_one_configuration_per_source() {
    let plan = office_pilot_plan();
    assert_eq!(plan.len(), 3);
    let mut sources = HashSet::new();
    for (index, record) in plan.iter().enumerate() {
        let (source, _) = pilot_options(index);
        let expected_file = format!("LibreOffice/{source}");
        let (_, expected_options) = office_pilot_expectation(index);
        assert_eq!(record.get("schema_version"), Some(&json!(1)));
        assert_eq!(
            record.get("file").and_then(Value::as_str),
            Some(expected_file.as_str())
        );
        assert!(sources.insert(expected_file));
        assert_eq!(record.get("options"), Some(&expected_options));
        assert_eq!(record.as_object().unwrap().len(), 3);
    }
}

fn assert_visible_output_close(candidate: &[u8], office: &[u8], label: &str) {
    let candidate_summary = PdfSummary::from_bytes(candidate).unwrap();
    let office_summary = PdfSummary::from_bytes(office).unwrap();
    assert_eq!(candidate_summary.page_count, 1, "{label}");
    assert_eq!(
        candidate_summary.page_count, office_summary.page_count,
        "{label}"
    );
    assert_eq!(candidate_summary.text, office_summary.text, "{label}");

    let candidate_page = rendered_page_image_from_pdf(candidate, 0, 1_333).unwrap();
    let office_page = rendered_page_image_from_pdf(office, 0, 1_333).unwrap();
    assert_eq!(candidate_page.width_px, office_page.width_px, "{label}");
    assert_eq!(candidate_page.height_px, office_page.height_px, "{label}");
    assert!(
        (candidate_page.page_width_pt - office_page.page_width_pt).abs() <= 0.1,
        "{label}: candidate width={} Office width={}",
        candidate_page.page_width_pt,
        office_page.page_width_pt
    );
    assert!(
        (candidate_page.page_height_pt - office_page.page_height_pt).abs() <= 0.1,
        "{label}: candidate height={} Office height={}",
        candidate_page.page_height_pt,
        office_page.page_height_pt
    );

    let mut significant_pixels = 0usize;
    let mut absolute_channel_delta = 0u64;
    for (candidate_pixel, office_pixel) in candidate_page
        .rgba
        .chunks_exact(4)
        .zip(office_page.rgba.chunks_exact(4))
    {
        let maximum_delta = candidate_pixel
            .iter()
            .zip(office_pixel)
            .map(|(candidate, office)| candidate.abs_diff(*office))
            .max()
            .unwrap();
        significant_pixels += usize::from(maximum_delta > 16);
        absolute_channel_delta += candidate_pixel
            .iter()
            .zip(office_pixel)
            .map(|(candidate, office)| u64::from(candidate.abs_diff(*office)))
            .sum::<u64>();
    }
    let pixel_count = candidate_page.rgba.len() / 4;
    let significant_fraction = significant_pixels as f64 / pixel_count as f64;
    let mean_delta = absolute_channel_delta as f64 / candidate_page.rgba.len() as f64;
    assert!(
        significant_fraction <= 0.01,
        "{label}: significant pixel fraction {significant_fraction} exceeds 0.01"
    );
    assert!(
        mean_delta <= 1.5,
        "{label}: mean channel delta {mean_delta} exceeds 1.5"
    );
}

#[test]
fn page_selection_preserves_selected_candidate_page_pixels_for_all_three_formats() {
    for index in 1..=2 {
        let (source, selected_options) = pilot_options(index);
        let fixture = libreoffice_fixture(source);
        let selected =
            ooxmlsdk_pdf_test::render::render_fixture_pdf_with_options(&fixture, selected_options)
                .unwrap();
        let full = ooxmlsdk_pdf_test::render::render_fixture_pdf_with_options(
            &fixture,
            PdfOptions {
                general: ooxmlsdk_pdf::PdfGeneralOptions {
                    tagged_pdf: index == 1,
                    export_bookmarks: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

        let selected_page = rendered_page_image_from_pdf(&selected, 0, 1_333).unwrap();
        let full_second_page = rendered_page_image_from_pdf(&full, 1, 1_333).unwrap();
        assert_eq!(
            selected_page.rgba_crc32, full_second_page.rgba_crc32,
            "{source}"
        );
        assert_eq!(selected_page.rgba, full_second_page.rgba, "{source}");
    }
}

#[test]
#[ignore = "requires scripts/probe_office_pdf_options.ps1 output under WSL /tmp"]
// This is the pre-promotion gate for a new configured golden. The probe
// records one configuration per source and never reads candidate output.
fn office_pdf_options_probe_is_a_hard_pass() {
    let root = std::env::var_os("OOXMLSDK_OFFICE_PDF_OPTIONS_PROBE_DIR")
        .map(std::path::PathBuf::from)
        .expect("set OOXMLSDK_OFFICE_PDF_OPTIONS_PROBE_DIR to the probe output directory");
    assert!(
        root.starts_with("/tmp"),
        "probe input must remain under /tmp"
    );

    let mut seen_sources = HashSet::new();
    let plan = office_pilot_plan();
    assert_eq!(plan.len(), 3);
    for (index, expected_plan_record) in plan.iter().enumerate() {
        let (source, mut options) = pilot_options(index);
        let fixture = libreoffice_fixture(source);
        options.source_file_name = fixture
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string);
        let candidate =
            ooxmlsdk_pdf_test::render::render_fixture_pdf_with_options(&fixture, options).unwrap();
        let office_path = root.join(format!("case-{index:03}.pdf"));
        let office = fs::read(&office_path).unwrap();
        assert_visible_output_close(&candidate, &office, source);
        if index == 0 {
            let candidate_text = String::from_utf8_lossy(&candidate);
            assert!(candidate_text.contains("<pdfaid:part>3</pdfaid:part>"));
            assert!(candidate_text.contains("<pdfaid:conformance>A</pdfaid:conformance>"));
        }

        let record_path = root.join(format!("case-{index:03}.json"));
        let record: Value = serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
        let expected_file = format!("LibreOffice/{source}");
        assert!(seen_sources.insert(expected_file.clone()));
        let (expected_application, expected_options) = office_pilot_expectation(index);
        assert_eq!(
            record.get("schema_version"),
            expected_plan_record.get("schema_version")
        );
        assert_eq!(
            record.get("file").and_then(Value::as_str),
            Some(expected_file.as_str())
        );
        assert_eq!(
            record.get("application").and_then(Value::as_str),
            Some(expected_application)
        );
        assert_eq!(record.get("options"), Some(&expected_options));
        assert_eq!(record.get("options"), expected_plan_record.get("options"));
        assert!(
            record
                .get("application_version")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            record
                .get("application_build")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
        );

        let source_bytes = fs::read(&fixture).unwrap();
        let expected_source_hash = format!("{:x}", Sha256::digest(&source_bytes));
        assert_eq!(
            record.get("source_sha256").and_then(Value::as_str),
            Some(expected_source_hash.as_str())
        );
        let expected_output = format!("case-{index:03}.pdf");
        assert_eq!(
            record.get("output").and_then(Value::as_str),
            Some(expected_output.as_str())
        );
        assert_eq!(
            record.get("output_bytes").and_then(Value::as_u64),
            Some(office.len() as u64)
        );
        let expected_output_hash = format!("{:x}", Sha256::digest(&office));
        assert_eq!(
            record.get("output_sha256").and_then(Value::as_str),
            Some(expected_output_hash.as_str())
        );
    }

    let office_pdf_a = fs::read(root.join("case-000.pdf")).unwrap();
    let office_pdf_a_text = String::from_utf8_lossy(&office_pdf_a);
    assert!(office_pdf_a_text.contains("<pdfaid:part>3</pdfaid:part>"));
    assert!(office_pdf_a_text.contains("<pdfaid:conformance>A</pdfaid:conformance>"));
}

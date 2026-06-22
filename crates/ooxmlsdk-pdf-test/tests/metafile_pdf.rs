use ooxmlsdk_pdf_test::{
    libreoffice_fixture, pdf_summary_for_fixture, rendered_page_image_for_fixture,
};

#[test]
// Source: ../core/sw/qa/extras/layout/layout.cxx:testTdf109137
fn docx_embedded_emf_exports_as_visible_pdf_image() {
    let fixture = libreoffice_fixture("sw/qa/extras/layout/data/tdf109137.docx");
    let summary = pdf_summary_for_fixture(&fixture).unwrap();

    assert_eq!(
        summary.image_count, 1,
        "expected only the embedded EMF image in the exported PDF; images={:?}; page_objects={:?}",
        summary.images, summary.page_objects
    );
    assert_eq!(
        page_image_count(&summary, 0),
        1,
        "expected the embedded EMF to stay visible on page 1; images={:?}; page_objects={:?}",
        summary.images,
        summary.page_objects
    );
    assert_eq!(
        summary
            .images
            .iter()
            .filter(|image| image.page_index != 0)
            .count(),
        0,
        "expected the embedded EMF not to move away from page 1; images={:?}; page_objects={:?}",
        summary.images,
        summary.page_objects
    );

    let rendered = rendered_page_image_for_fixture(&fixture, 0, 1024).unwrap();
    let [r, g, b, _] = rendered
        .pixel_rgba(512, 655)
        .expect("missing rendered pixel at the calibrated tdf109137 blue rectangle sample point");
    let diff = i16::from(r).abs() + i16::from(g).abs() + (i16::from(b) - 255).abs();
    assert!(
        diff <= 12,
        "expected visible blue output from embedded EMF on page 1; sampled #{r:02x}{g:02x}{b:02x}; rendered_crc={}",
        rendered.rgba_crc32
    );
}

fn page_image_count(summary: &ooxmlsdk_pdf_test::PdfSummary, page_index: usize) -> usize {
    summary
        .images
        .iter()
        .filter(|image| image.page_index == page_index)
        .count()
}

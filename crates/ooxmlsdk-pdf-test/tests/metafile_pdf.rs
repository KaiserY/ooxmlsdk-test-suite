use ooxmlsdk_pdf_test::{
    libreoffice_fixture, parse_pdf_rect, pdf_summary_for_fixture, rendered_page_image_for_fixture,
    workspace_root,
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

#[test]
// Sources: [MS-XLSX] 2.5.3, [MS-EMF] 2.3.1.1, and
// ../msdn-code-gallery-microsoft/**/GenPrint Print Processor Sample/C++/emf.cpp.
fn xlsx_ole_emf_keeps_excel_grid_placement_and_alpha_blend_content() {
    let fixture = workspace_root().join("corpus/Apache-POI/test-data/spreadsheet/58325_lt.xlsx");
    let summary = pdf_summary_for_fixture(&fixture).unwrap();
    let heading = summary
        .text_segments
        .iter()
        .find(|segment| segment.text.split_whitespace().collect::<String>() == "AreadeAnexos")
        .expect("missing worksheet heading");
    let bounds = parse_pdf_rect(&heading.bounds).expect("invalid worksheet heading bounds");
    assert!(
        (89.0..=91.5).contains(&bounds.left),
        "the missing-metadata Excel grid moved the B2 heading: {bounds:?}"
    );

    let rendered = rendered_page_image_for_fixture(&fixture, 0, 1024).unwrap();
    let adobe_red_pixels = rendered
        .rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] >= 150 && pixel[1] <= 120 && pixel[2] <= 120 && pixel[3] >= 200)
        .count();
    assert!(
        adobe_red_pixels >= 100,
        "EMR_ALPHABLEND lost the embedded PDF icon; red_pixels={adobe_red_pixels}; crc={}",
        rendered.rgba_crc32
    );
}

#[test]
// Sources: ../poi/poi-ooxml/src/test/java/org/apache/poi/xssf/usermodel/TestXSSFShape.java,
// ../msdn-code-gallery-microsoft/OneCodeTeam/Create new events for shape in Excel
// (CSExcelNewEventForShapes)/**/MyShapes.cs, and the Microsoft Office Developer
// Documentation Team's Excel.ExportAsFixedFormat sample.
fn xlsx_multiple_ole_previews_keep_clipped_following_cell_pdf_semantics() {
    let fixture = workspace_root().join("corpus/Apache-POI/test-data/spreadsheet/58325_db.xlsx");
    let summary = pdf_summary_for_fixture(&fixture).unwrap();
    assert_eq!(summary.page_count, 4);

    let mut heading_pages = summary
        .text_segments
        .iter()
        .filter(|segment| segment.text.split_whitespace().collect::<String>() == "AreadeAnexos")
        .map(|segment| segment.page_index)
        .collect::<Vec<_>>();
    heading_pages.sort_unstable();
    heading_pages.dedup();
    assert_eq!(
        heading_pages,
        [0, 1],
        "the tagged heading at the first horizontal page clip was dropped: {:?}",
        summary.text_segments
    );
}

fn page_image_count(summary: &ooxmlsdk_pdf_test::PdfSummary, page_index: usize) -> usize {
    summary
        .images
        .iter()
        .filter(|image| image.page_index == page_index)
        .count()
}

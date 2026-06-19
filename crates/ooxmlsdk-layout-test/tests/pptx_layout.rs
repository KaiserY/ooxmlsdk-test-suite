use ooxmlsdk_layout::pptx::{PptxDrawShapeSummary, PptxLayoutSummary};
use ooxmlsdk_layout_test::{pptx_import_summary, pptx_layout};

fn close(actual: i32, expected: i32) -> bool {
    (actual - expected).abs() <= 3
}

fn assert_master_text_contains(summary: &PptxLayoutSummary, expected: &str) {
    assert!(
        summary
            .master_text_shapes
            .iter()
            .any(|shape| shape.text.contains(expected)),
        "missing master text {expected:?}; master_text_shapes={:?}",
        summary.master_text_shapes
    );
}

fn draw_shapes_with_geometry<'a>(
    summary: &'a PptxLayoutSummary,
    page_index: usize,
    geometry: &str,
) -> Vec<&'a PptxDrawShapeSummary> {
    summary
        .draw_shapes
        .iter()
        .filter(|shape| {
            shape.page_index == page_index
                && shape
                    .geometry
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(geometry))
        })
        .collect()
}

fn assert_draw_shape_rect_100mm(
    summary: &PptxLayoutSummary,
    page_index: usize,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    assert!(
        summary.draw_shapes.iter().any(|shape| {
            shape.page_index == page_index
                && close(shape.left_100mm, left)
                && close(shape.top_100mm, top)
                && close(shape.right_100mm, right)
                && close(shape.bottom_100mm, bottom)
        }),
        "missing draw shape rect ({left},{top},{right},{bottom}) on page {page_index}; draw_shapes={:?}",
        summary.draw_shapes
    );
}

fn assert_draw_shape_size_100mm(
    summary: &PptxLayoutSummary,
    page_index: usize,
    width: i32,
    height: i32,
) {
    assert!(
        summary.draw_shapes.iter().any(|shape| {
            shape.page_index == page_index
                && close(shape.width_100mm, width)
                && close(shape.height_100mm, height)
        }),
        "missing draw shape size ({width}x{height}) on page {page_index}; draw_shapes={:?}",
        summary.draw_shapes
    );
}

fn assert_graphic_bullet_size_100mm(
    summary: &PptxLayoutSummary,
    page_index: usize,
    expected_text: &str,
    expected_width: i32,
    expected_height: i32,
) {
    assert!(
        summary.bullet_paragraphs.iter().any(|paragraph| {
            paragraph.page_index == page_index
                && paragraph
                    .text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    == expected_text
                && paragraph
                    .graphic_width_100mm
                    .is_some_and(|width| (width - expected_width).abs() <= 1)
                && paragraph
                    .graphic_height_100mm
                    .is_some_and(|height| (height - expected_height).abs() <= 1)
        }),
        "missing graphic bullet {expected_width}x{expected_height} for {expected_text:?} on page {page_index}; bullets={:?}",
        summary.bullet_paragraphs
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testBnc584721_1 and testBnc584721_2
fn pptx_bnc584721_preserves_single_master_title_text_shape() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/bnc584721_1_2.pptx").unwrap();
    assert_master_text_contains(&summary, "Click to edit Master title style");
    assert_eq!(summary.master_text_shapes.len(), 1);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testTdf93830
fn pptx_tdf93830_preserves_text_left_distance_offset() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf93830.pptx").unwrap();
    assert!(
        summary
            .smartart_text_shapes
            .iter()
            .any(|shape| shape.text_left_distance_100mm == 4024),
        "missing SmartArt text left distance 4024; smartart_text_shapes={:?}",
        summary.smartart_text_shapes
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests-smartart.cxx::testTdf134221
fn pptx_smartart_tdf134221_preserves_negative_upper_text_inset() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/smartart-tdf134221.pptx").unwrap();
    assert!(
        summary.smartart_text_shapes.iter().any(|shape| {
            shape.page_index == 0 && shape.text == "B" && shape.text_upper_distance_100mm == -248
        }),
        "missing LibreOffice SmartArt text upper distance -248 for B; smartart_text_shapes={:?}",
        summary.smartart_text_shapes
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests-smartart.cxx::testTdf145528Matrix
fn pptx_smartart_tdf145528_matrix_preserves_text_positions() {
    let summary =
        pptx_import_summary("sd/qa/unit/data/pptx/tdf145528_SmartArt_Matrix.pptx").unwrap();
    let expected = [
        ("Writer", 4001, 9999),
        ("Calc", 12001, 1999),
        ("Impress", 12001, 12499),
        ("Draw", 18501, 5999),
    ];
    for (text, left, top) in expected {
        assert!(
            summary.smartart_text_shapes.iter().any(|shape| {
                shape.page_index == 0
                    && shape.text.contains(text)
                    && close(shape.text_anchor_left_100mm, left)
                    && close(shape.text_anchor_top_100mm, top)
                    && close(
                        shape.text_anchor_right_100mm - shape.text_anchor_left_100mm,
                        10001,
                    )
                    && close(
                        shape.text_anchor_bottom_100mm - shape.text_anchor_top_100mm,
                        4500,
                    )
            }),
            "missing LibreOffice matrix text anchor for {text:?}; smartart_text_shapes={:?}",
            summary.smartart_text_shapes
        );
    }
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf165321
fn pptx_tdf165321_preserves_smartart_child_dimensions() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf165321.pptx").unwrap();
    assert_draw_shape_size_100mm(&summary, 0, 6592, 3597);
    assert_draw_shape_size_100mm(&summary, 0, 6402, 3597);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf103473
fn pptx_tdf103473_preserves_picture_geometry() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf103473.pptx").unwrap();
    assert_draw_shape_rect_100mm(&summary, 0, 3629, 4431, 8353, 9155);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf109187
fn pptx_tdf109187_preserves_two_gradient_arrow_shapes() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf109187.pptx").unwrap();
    let right_arrows = draw_shapes_with_geometry(&summary, 0, "ooxml-rightarrow");
    let down_arrows = draw_shapes_with_geometry(&summary, 0, "ooxml-downarrow");
    assert_eq!(
        right_arrows.first().and_then(|shape| shape.gradient_angle),
        Some(2250)
    );
    assert_eq!(
        down_arrows.first().and_then(|shape| shape.gradient_angle),
        Some(1350)
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf90626
fn pptx_tdf90626_preserves_graphic_bullet_size() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf90626.pptx").unwrap();
    assert_graphic_bullet_size_100mm(&summary, 0, "Test", 372, 372);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf138148
fn pptx_tdf138148_preserves_narrow_graphic_bullet_size() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf138148.pptx").unwrap();
    assert_graphic_bullet_size_100mm(&summary, 0, "Aaa", 148, 444);
    assert_graphic_bullet_size_100mm(&summary, 0, "Bbb", 148, 444);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf114913
fn pptx_tdf114913_preserves_graphic_bullet_height() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf114913.pptx").unwrap();
    assert_graphic_bullet_size_100mm(&summary, 0, "Test", 692, 692);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests4.cxx::testTdf149785
fn pptx_tdf149785_imports_single_visible_object() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf149785.pptx").unwrap();
    assert_eq!(summary.draw_page_shape_counts, vec![1]);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests4.cxx::testTdf149985
fn pptx_tdf149985_imports_single_visible_object() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf149985.pptx").unwrap();
    assert_eq!(summary.draw_page_shape_counts, vec![1]);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests4.cxx::tdf158512
fn pptx_tdf158512_preserves_unfilled_foreground_shape() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf158512.pptx").unwrap();
    assert_eq!(summary.draw_page_shape_counts.first().copied(), Some(2));
    assert!(
        summary
            .draw_shapes
            .iter()
            .find(|shape| shape.page_index == 0)
            .is_some_and(|shape| shape.fill_style == "None" && !shape.fill_uses_slide_background),
        "first foreground shape is not no-fill; draw_shapes={:?}",
        summary.draw_shapes
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testTdf150789
fn pptx_tdf150789_preserves_up_arrow_callout_text_distances() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf150789.pptx").unwrap();
    let up_arrow_callouts = draw_shapes_with_geometry(&summary, 0, "ooxml-uparrowcallout");
    assert!(
        up_arrow_callouts
            .iter()
            .filter(|shape| {
                shape.text_upper_distance_100mm == Some(395)
                    && shape.text_lower_distance_100mm == Some(1424)
                    && shape.text_right_distance_100mm == Some(395)
                    && shape.text_left_distance_100mm == Some(395)
            })
            .count()
            >= 2,
        "missing two LibreOffice upArrowCallout text distance shapes; draw_shapes={:?}",
        summary.draw_shapes
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests.cxx::testTdf142913
fn pptx_tdf142913_preserves_first_page_selection() {
    let document = pptx_layout("sd/qa/unit/data/pptx/tdf142913.pptx").unwrap();
    assert_eq!(document.pages.len(), 3);
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf142913.pptx").unwrap();
    assert_eq!(summary.first_page_name.as_deref(), Some("Second"));
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf89064
fn pptx_tdf89064_preserves_single_notes_shape() {
    let summary = pptx_import_summary("sd/qa/unit/data/pptx/tdf89064.pptx").unwrap();
    assert_eq!(summary.notes_page_shape_counts, vec![1]);
}

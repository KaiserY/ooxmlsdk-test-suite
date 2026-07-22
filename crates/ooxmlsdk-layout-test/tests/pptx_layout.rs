use std::path::Path;

use ooxmlsdk::parts::presentation_document::PresentationDocument;
use ooxmlsdk_layout::LayoutOptions;
use ooxmlsdk_layout::common::{Color, DebugShape, DisplayItem, Fill, LayoutDocument};
use ooxmlsdk_layout::options::LayoutDiagnosticsOptions;
use ooxmlsdk_layout_test::{
    all_page_text, assert_text_color, assert_text_font_size, debug_bool_property,
    debug_integer_property, debug_shape_has_text_property, debug_shape_integer_close, debug_shapes,
    debug_text_property, normalize_space, pptx_layout,
};

fn pptx_debug(path: &str) -> LayoutDocument<'static> {
    pptx_layout(path).unwrap()
}

fn open_xml_sdk_pptx_debug(path: &str) -> LayoutDocument<'static> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Open-XML-SDK")
        .join(path);
    let mut package = PresentationDocument::new_from_file(fixture).unwrap();
    ooxmlsdk_layout::pptx::layout_document(
        &mut package,
        &LayoutOptions {
            diagnostics: LayoutDiagnosticsOptions {
                collect_debug_records: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

fn shapes<'a, 'doc>(document: &'a LayoutDocument<'doc>, kind: &str) -> Vec<&'a DebugShape<'doc>> {
    debug_shapes(document, kind)
}

fn draw_shapes<'a, 'doc>(document: &'a LayoutDocument<'doc>) -> Vec<&'a DebugShape<'doc>> {
    shapes(document, "pptx_draw_shape")
}

fn draw_shapes_with_geometry<'a, 'doc>(
    document: &'a LayoutDocument<'doc>,
    page_index: usize,
    geometry: &str,
) -> Vec<&'a DebugShape<'doc>> {
    draw_shapes(document)
        .into_iter()
        .filter(|shape| {
            shape.page_index == page_index
                && debug_text_property(shape, "geometry")
                    .is_some_and(|value| value.eq_ignore_ascii_case(geometry))
        })
        .collect()
}

fn assert_master_text_contains(document: &LayoutDocument<'_>, expected: &str) {
    let shapes = shapes(document, "pptx_master_text_shape");
    assert!(
        shapes
            .iter()
            .any(|shape| debug_shape_has_text_property(shape, "text", expected)),
        "missing master text {expected:?}; shapes={shapes:?}"
    );
}

fn assert_draw_shape_rect_100mm(
    document: &LayoutDocument<'_>,
    page_index: usize,
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
) {
    let shapes = draw_shapes(document);
    assert!(
        shapes.iter().any(|shape| {
            shape.page_index == page_index
                && debug_shape_integer_close(shape, "left_100mm", left)
                && debug_shape_integer_close(shape, "top_100mm", top)
                && debug_shape_integer_close(shape, "right_100mm", right)
                && debug_shape_integer_close(shape, "bottom_100mm", bottom)
        }),
        "missing draw shape rect ({left},{top},{right},{bottom}) on page {page_index}; shapes={shapes:?}"
    );
}

fn assert_draw_shape_size_100mm(
    document: &LayoutDocument<'_>,
    page_index: usize,
    width: i64,
    height: i64,
) {
    let shapes = draw_shapes(document);
    assert!(
        shapes.iter().any(|shape| {
            shape.page_index == page_index
                && debug_shape_integer_close(shape, "width_100mm", width)
                && debug_shape_integer_close(shape, "height_100mm", height)
        }),
        "missing draw shape size ({width}x{height}) on page {page_index}; shapes={shapes:?}"
    );
}

fn assert_graphic_bullet_size_100mm(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_text: &str,
    expected_width: i64,
    expected_height: i64,
) {
    let bullets = shapes(document, "pptx_bullet_paragraph");
    assert!(
        bullets.iter().any(|shape| {
            shape.page_index == page_index
                && debug_text_property(shape, "text")
                    .map(normalize_space)
                    .as_deref()
                    == Some(expected_text)
                && debug_shape_integer_close(shape, "graphic_width_100mm", expected_width)
                && debug_shape_integer_close(shape, "graphic_height_100mm", expected_height)
        }),
        "missing graphic bullet {expected_width}x{expected_height} for {expected_text:?} on page {page_index}; bullets={bullets:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testBnc584721_1 and testBnc584721_2
fn pptx_bnc584721_preserves_single_master_title_text_shape() {
    let document = pptx_debug("sd/qa/unit/data/pptx/bnc584721_1_2.pptx");
    assert_master_text_contains(&document, "Click to edit Master title style");
    assert_eq!(shapes(&document, "pptx_master_text_shape").len(), 1);
}

#[test]
// Sources:
// - ECMA-376 Part 1 §19.3.1.25 (omitted p:hf@sldNum defaults to true)
// - ../core/sd/qa/unit/data/pptx/tdf119187.pptx
fn pptx_tdf119187_preserves_master_slide_number_field() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf119187.pptx");
    assert_master_text_contains(&document, "‹#›");
    let page_title = draw_shapes(&document)
        .into_iter()
        .find(|shape| {
            shape.page_index == 0
                && debug_text_property(shape, "text")
                    .map(normalize_space)
                    .as_deref()
                    == Some("Page 1")
        })
        .expect("missing Page 1 title shape");
    assert_eq!(
        debug_text_property(page_title, "service_name"),
        Some("TitleText")
    );
    assert_text_font_size(&document, "Page 1", 27.96);
    assert_text_color(
        &document,
        "Page 1",
        Color {
            r: 0x02,
            g: 0x1b,
            b: 0x45,
            a: 0xff,
        },
    );
    let paragraph_run = document
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter_map(|item| match item {
            DisplayItem::Text(text) if text.text.contains("Paragraph") => Some(text),
            _ => None,
        })
        .next()
        .expect("missing paragraph text run");
    assert_eq!(
        paragraph_run.style.fallback_font_family.as_deref(),
        Some("Calibri")
    );
    let slide_number_run = document.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) if text.text == "1" && text.origin.x.0 > 900.0 => Some(text),
            _ => None,
        })
        .next()
        .expect("missing right-aligned master slide-number run");
    assert!(
        (slide_number_run.origin.y.0 - 518.59).abs() < 0.1,
        "10pt slide-number line should be bottom-anchored using its own line height: {slide_number_run:?}"
    );
    for (page_index, expected) in ["1", "2", "3"].into_iter().enumerate() {
        let text = all_page_text(&document, page_index);
        assert_eq!(
            text.split_whitespace()
                .filter(|token| *token == expected)
                .count(),
            3,
            "slide number {expected} was not lowered separately on page {page_index}; text={text:?}"
        );
    }
}

#[test]
// Sources:
// - ECMA-376 Part 1 §19.3.1.25 (header/footer visibility defaults to true)
// - Microsoft Office golden for ../core/sd/qa/unit/data/pptx/tdf130058.pptx
fn pptx_tdf130058_keeps_slide_number_without_header_footer_element() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf130058.pptx");

    assert_eq!(normalize_space(&all_page_text(&document, 0)), "1");
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testTdf93830
fn pptx_tdf93830_preserves_text_left_distance_offset() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf93830.pptx");
    let shapes = shapes(&document, "pptx_smartart_text_shape");
    assert!(
        shapes.iter().any(|shape| {
            debug_integer_property(shape, "text_left_distance_100mm") == Some(4024)
        }),
        "missing SmartArt text left distance 4024; shapes={shapes:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests-smartart.cxx::testTdf134221
fn pptx_smartart_tdf134221_preserves_negative_upper_text_inset() {
    let document = pptx_debug("sd/qa/unit/data/pptx/smartart-tdf134221.pptx");
    let shapes = shapes(&document, "pptx_smartart_text_shape");
    assert!(
        shapes.iter().any(|shape| {
            shape.page_index == 0
                && debug_text_property(shape, "text") == Some("B")
                && debug_integer_property(shape, "text_upper_distance_100mm") == Some(-248)
        }),
        "missing LibreOffice SmartArt text upper distance -248 for B; shapes={shapes:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests-smartart.cxx::testTdf145528Matrix
fn pptx_smartart_tdf145528_matrix_preserves_text_positions() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf145528_SmartArt_Matrix.pptx");
    let shapes = shapes(&document, "pptx_smartart_text_shape");
    let expected = [
        ("Writer", 4001, 9999),
        ("Calc", 12001, 1999),
        ("Impress", 12001, 12499),
        ("Draw", 18501, 5999),
    ];
    for (text, left, top) in expected {
        assert!(
            shapes.iter().any(|shape| {
                shape.page_index == 0
                    && debug_shape_has_text_property(shape, "text", text)
                    && debug_shape_integer_close(shape, "text_anchor_left_100mm", left)
                    && debug_shape_integer_close(shape, "text_anchor_top_100mm", top)
                    && debug_integer_property(shape, "text_anchor_right_100mm")
                        .zip(debug_integer_property(shape, "text_anchor_left_100mm"))
                        .is_some_and(|(right, left)| (right - left - 10001).abs() <= 3)
                    && debug_integer_property(shape, "text_anchor_bottom_100mm")
                        .zip(debug_integer_property(shape, "text_anchor_top_100mm"))
                        .is_some_and(|(bottom, top)| (bottom - top - 4500).abs() <= 3)
            }),
            "missing LibreOffice matrix text anchor for {text:?}; shapes={shapes:?}"
        );
    }
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf165321
fn pptx_tdf165321_preserves_smartart_child_dimensions() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf165321.pptx");
    assert_draw_shape_size_100mm(&document, 0, 6592, 3597);
    assert_draw_shape_size_100mm(&document, 0, 6402, 3597);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf103473
fn pptx_tdf103473_preserves_picture_geometry() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf103473.pptx");
    assert_draw_shape_rect_100mm(&document, 0, 3629, 4431, 8353, 9155);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf109187
fn pptx_tdf109187_preserves_two_gradient_arrow_shapes() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf109187.pptx");
    let right_arrows = draw_shapes_with_geometry(&document, 0, "ooxml-rightarrow");
    let down_arrows = draw_shapes_with_geometry(&document, 0, "ooxml-downarrow");
    assert_eq!(
        right_arrows
            .first()
            .and_then(|shape| debug_integer_property(shape, "gradient_angle")),
        Some(2250)
    );
    assert_eq!(
        down_arrows
            .first()
            .and_then(|shape| debug_integer_property(shape, "gradient_angle")),
        Some(1350)
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf90626
fn pptx_tdf90626_preserves_graphic_bullet_size() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf90626.pptx");
    assert_graphic_bullet_size_100mm(&document, 0, "Test", 372, 372);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf138148
fn pptx_tdf138148_preserves_narrow_graphic_bullet_size() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf138148.pptx");
    assert_graphic_bullet_size_100mm(&document, 0, "Aaa", 148, 444);
    assert_graphic_bullet_size_100mm(&document, 0, "Bbb", 148, 444);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf114913
fn pptx_tdf114913_preserves_graphic_bullet_height() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf114913.pptx");
    assert_graphic_bullet_size_100mm(&document, 0, "Test", 692, 692);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests4.cxx::testTdf149785
fn pptx_tdf149785_imports_single_visible_object() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf149785.pptx");
    assert_eq!(draw_shapes(&document).len(), 1);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests4.cxx::testTdf149985
fn pptx_tdf149985_imports_single_visible_object() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf149985.pptx");
    assert_eq!(draw_shapes(&document).len(), 1);
}

#[test]
// Source: ../core/sd/qa/unit/import-tests4.cxx::tdf158512
fn pptx_tdf158512_preserves_unfilled_foreground_shape() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf158512.pptx");
    let draw_shapes = draw_shapes(&document);
    assert_eq!(
        draw_shapes
            .iter()
            .filter(|shape| shape.page_index == 0)
            .count(),
        2
    );
    assert!(
        draw_shapes
            .iter()
            .find(|shape| shape.page_index == 0)
            .is_some_and(|shape| {
                debug_text_property(shape, "fill_style") == Some("None")
                    && debug_bool_property(shape, "fill_uses_slide_background") == Some(false)
            }),
        "first foreground shape is not no-fill; shapes={draw_shapes:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testTdf150789
fn pptx_tdf150789_preserves_up_arrow_callout_text_distances() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf150789.pptx");
    let up_arrow_callouts = draw_shapes_with_geometry(&document, 0, "ooxml-uparrowcallout");
    assert!(
        up_arrow_callouts
            .iter()
            .filter(|shape| {
                debug_integer_property(shape, "text_upper_distance_100mm") == Some(395)
                    && debug_integer_property(shape, "text_lower_distance_100mm") == Some(1424)
                    && debug_integer_property(shape, "text_right_distance_100mm") == Some(395)
                    && debug_integer_property(shape, "text_left_distance_100mm") == Some(395)
            })
            .count()
            >= 2,
        "missing two LibreOffice upArrowCallout text distance shapes; shapes={up_arrow_callouts:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests3.cxx::testTdf165732
fn pptx_tdf165732_clamps_text_insets_symmetrically() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf165732.pptx");
    let draw_shapes = draw_shapes(&document);
    assert!(
        draw_shapes.iter().any(|shape| {
            shape.page_index == 0
                && debug_text_property(shape, "text") == Some("2")
                && debug_integer_property(shape, "text_left_distance_100mm") == Some(199)
                && debug_integer_property(shape, "text_right_distance_100mm") == Some(199)
        }),
        "missing clamped horizontal text insets for shape 2; shapes={draw_shapes:?}"
    );
    assert!(
        draw_shapes.iter().any(|shape| {
            shape.page_index == 0
                && debug_text_property(shape, "text") == Some("1")
                && debug_integer_property(shape, "text_left_distance_100mm") == Some(100)
        }),
        "missing unclamped left text inset for shape 1; shapes={draw_shapes:?}"
    );
    assert!(
        draw_shapes.iter().any(|shape| {
            shape.page_index == 0
                && debug_text_property(shape, "text") == Some("5")
                && debug_integer_property(shape, "text_upper_distance_100mm") == Some(183)
                && debug_integer_property(shape, "text_lower_distance_100mm") == Some(183)
        }),
        "missing clamped vertical text insets for shape 5; shapes={draw_shapes:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests.cxx::testTdf142913
fn pptx_tdf142913_preserves_first_page_selection() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf142913.pptx");
    assert_eq!(document.pages.len(), 3);
    let first_pages = shapes(&document, "pptx_first_page");
    assert!(
        first_pages
            .iter()
            .any(|shape| debug_text_property(shape, "name") == Some("Second")),
        "missing first page debug record; shapes={first_pages:?}"
    );
}

#[test]
// Source: ../core/sd/qa/unit/import-tests2.cxx::testTdf89064
fn pptx_tdf89064_preserves_single_notes_shape() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf89064.pptx");
    assert_eq!(shapes(&document, "pptx_notes_shape").len(), 1);
}

#[test]
// Sources:
// - ISO/IEC 29500-1:2016 §21.2.2.16 (barChart)
// - ISO/IEC 29500-1:2016 §21.2.2.95 (legendPos)
// - Open XML SDK Chart_2D.pptx source and immutable Microsoft Office golden PDF
fn pptx_chart_2d_lowers_slide_chart_placeholder_content() {
    let document = open_xml_sdk_pptx_debug(
        "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/presentation/Chart_2D.pptx",
    );
    let draw_shapes = draw_shapes(&document);
    assert!(
        draw_shapes.iter().any(|shape| {
            debug_text_property(shape, "service_name") == Some("Chart") && shape.page_index == 0
        }),
        "missing slide chart frame; shapes={draw_shapes:?}"
    );
    let text = all_page_text(&document, 0);
    for expected in ["0", "6", "Category 1", "Category 4", "Series 1", "Series 3"] {
        assert!(
            text.contains(expected),
            "missing {expected:?}; text={text:?}"
        );
    }
}

#[test]
// Sources:
// - ISO/IEC 29500-1:2016 §21.2.2.210 (title) and §21.2.2.197 (spPr)
// - immutable Microsoft Office fixed-format output for the LibreOffice fixture
fn pptx_chart_title_preserves_explicit_solid_fill() {
    let document = pptx_debug("chart2/qa/extras/data/pptx/testChartTitlePropertiesColorFill.pptx");
    assert!(
        document.pages[1].items.iter().any(|item| matches!(
            item,
            DisplayItem::Rect(rect)
                if rect.fill == Fill::Solid(Color { r: 255, g: 0, b: 0, a: 255 })
                    && rect.bounds.size.width.0 > 240.0
                    && rect.bounds.size.height.0 > 20.0
        )),
        "missing explicit red chart-title background: {:?}",
        document.pages[1].items
    );
}

#[test]
// Sources:
// - ISO/IEC 29500-1:2016 §20.1.8.58 (tile) and §21.2.2.197 (spPr)
// - JFIF 1.02 pixel-density fields in the embedded 75 DPI JPEG
fn pptx_chart_title_tiles_bitmap_at_its_physical_resolution() {
    let document = pptx_debug("chart2/qa/extras/data/pptx/testChartTitlePropertiesBitmapFill.pptx");
    let title_tiles = document.pages[1]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Image(image)
                if image.bounds.origin.y.0 >= 149.0
                    && image.bounds.origin.y.0 <= 151.0
                    && image.bounds.size.height.0 >= 25.0
                    && image.bounds.size.height.0 <= 27.0 =>
            {
                Some(image)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(title_tiles.len(), 3, "title tiles={title_tiles:?}");
    assert!(
        (title_tiles[0].bounds.size.width.0 - 122.88).abs() < 0.1,
        "first tile must use 128 px at 75 DPI: {:?}",
        title_tiles[0].bounds
    );
}

#[test]
// Sources:
// - ISO/IEC 29500-1:2016 §20.1.2.2.8 (cNvPr/@hidden)
// - LibreOffice tdf147586.pptx source and immutable Microsoft Office golden PDF
fn pptx_tdf147586_keeps_hidden_chart_placeholder_content_out_of_layout() {
    let document = pptx_debug("sd/qa/unit/data/pptx/tdf147586.pptx");
    let text = all_page_text(&document, 0);
    assert!(!text.contains("xxx"), "hidden chart text leaked: {text:?}");
    assert!(text.contains("item"), "missing visible item text: {text:?}");
    assert!(
        text.contains("two"),
        "missing visible second item: {text:?}"
    );
}

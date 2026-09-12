use std::path::Path;

use ooxmlsdk::parts::presentation_document::PresentationDocument;
use ooxmlsdk_layout::LayoutOptions;
use ooxmlsdk_layout::common::{Color, DebugShape, DisplayItem, Fill, LayoutDocument};
use ooxmlsdk_layout::options::LayoutDiagnosticsOptions;
use ooxmlsdk_layout_test::{
    all_page_text, assert_text_color, assert_text_font_size, debug_bool_property,
    debug_integer_property, debug_shape_has_text_property, debug_shape_integer_close, debug_shapes,
    debug_text_property, normalize_space, page_display_items, pptx_layout,
};

fn pptx_debug(path: &str) -> LayoutDocument<'static> {
    pptx_layout(path).unwrap()
}

#[test]
fn wrapped_overlong_latin_text_breaks_even_when_latin_line_break_is_disabled() {
    let document = pptx_layout("sd/qa/unit/data/pptx/font-scale.pptx").unwrap();
    let items = page_display_items(&document, 0);
    // Current Office PDF uses 3 A-lines, 4 B-lines and 3 C-lines. The master
    // explicitly has latinLnBrk=0; bodyPr has wrapping and normal autofit.
    for (letter, expected_lines) in [('A', 3), ('B', 4), ('C', 3)] {
        let mut origins = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) if text.text.contains(letter) => {
                    Some(text.origin.y.0.round() as i32)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        origins.sort_unstable();
        origins.dedup();
        assert_eq!(
            origins.len(),
            expected_lines,
            "wrong line count for {letter}"
        );
    }
}

#[test]
fn unbulleted_hanging_indent_stays_within_the_text_body_inset() {
    let document = pptx_layout("sd/qa/unit/data/pptx/paraMarginAndIndentation.pptx").unwrap();
    let items = page_display_items(&document, 0);
    let run = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Text(run) if run.text == "priyanka" => Some(run),
            _ => None,
        })
        .expect("missing paragraph text");
    // Office starts at shape x + body inset, despite marL + indent < 0.
    assert!(
        (run.origin.x.0 - 151.78).abs() < 0.5,
        "unexpected origin: {:?}",
        run.origin
    );
}

#[test]
fn smartart_counter_rotated_text_remains_inside_its_frame() {
    let document = pptx_layout("sd/qa/unit/data/pptx/n862510_2.pptx").unwrap();
    let text = all_page_text(&document, 0);
    assert!(
        text.contains("List 1"),
        "missing SmartArt child text: {text}"
    );
    let runs = page_display_items(&document, 0);
    let run = runs
        .iter()
        .find_map(|item| match item {
            DisplayItem::Text(run) if run.text == "List 1" => Some(run),
            _ => None,
        })
        .expect("missing List 1 text run");
    assert!(
        run.style.rotation_degrees.abs() < 0.01,
        "counter-rotation did not cancel"
    );
    // Current Office origin is about x=319 pt. This broad frame check catches
    // the extra rotation that moved the cached child text outside the slide.
    assert!(
        run.origin.x.0 > 250.0 && run.origin.x.0 < 400.0,
        "unexpected origin: {:?}",
        run.origin
    );
}

#[test]
fn japanese_automatic_chart_title_uses_the_office_ui_language() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Apache-POI/test-data/slideshow/chart-texture-bg.pptx");
    let package = PresentationDocument::new_from_file(fixture).unwrap();
    let document = ooxmlsdk_layout::pptx::layout_document(
        &package,
        &LayoutOptions {
            ui_language: Some("ja-JP".into()),
            format_locale: Some("en-US".into()),
            default_document_language: Some("zh-CN".into()),
            ..Default::default()
        },
    )
    .unwrap();
    // The chart's c:title has no authored c:tx. Its current Office PDF uses
    // the Japanese UI title while keeping the authored English categories.
    let text = all_page_text(&document, 0);
    assert!(text.contains("グラフ タイトル"), "{text:?}");
    assert!(!text.contains("Chart Title"), "{text:?}");
}

#[test]
fn automatic_numbering_uses_text_font_and_preserves_character_bullet_fonts() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;

    let fixture = ooxmlsdk_layout_test::corpus_file("sd/qa/unit/data/pptx/n83889.pptx");
    for bullet_font in ["Arial", "Courier New", "Wingdings"] {
        let mut package = PresentationDocument::new_from_file(&fixture).unwrap();
        let presentation = package.presentation_part().unwrap();
        let slide = presentation.slide_parts(&package).next().unwrap();
        let root = slide.root_element_mut(&mut package).unwrap();
        let body = root
            .common_slide_data
            .shape_tree
            .shape_tree_choice
            .iter_mut()
            .find_map(|shape| match shape {
                p::ShapeTreeChoice::Shape(shape) => shape.text_body.as_mut(),
                _ => None,
            })
            .expect("numbered text body");
        let properties = body.paragraph[0].paragraph_properties.as_mut().unwrap();
        let Some(a::ParagraphPropertiesChoice3::BulletFont(font)) =
            properties.paragraph_properties_choice3.as_mut()
        else {
            panic!("bullet font")
        };
        font.typeface = bullet_font.into();

        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let number = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text == "B." => Some(text),
                _ => None,
            })
            .expect("startAt=2 alphabetic number, not a symbol-font selector");
        // Current Office PDF uses Calibri for B., despite buFont="Arial".
        assert_eq!(
            number.style.font_family.as_deref(),
            Some("Calibri"),
            "{bullet_font}"
        );
        let character_bullets = items
            .iter()
            .filter(|item| {
                matches!(
                    item, DisplayItem::Text(text)
                        if text.style.font_family.as_deref() == Some("Wingdings")
                            && text.text == "\u{f0a7}"
                )
            })
            .count();
        assert_eq!(
            character_bullets, 2,
            "explicit character bullets retain Wingdings"
        );
    }
}

#[test]
fn existing_table_requires_an_explicit_style_reference() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;

    // Office renders the original table's abc in regular black, even though
    // tableStyles.xml defines a bold white first row for newly inserted tables.
    // ECMA-376 §20.1.4.2.27 and [MS-OI29500] §21.1.3.12.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Apache-POI/test-data/slideshow/table-with-no-theme.pptx");
    let black = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    let white = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    for (style_id, styled) in [
        (None, false),
        (Some("{00000000-0000-0000-0000-000000000000}"), false),
        (Some("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"), true),
    ] {
        let mut package = PresentationDocument::new_from_file(&fixture).unwrap();
        let presentation = package.presentation_part().unwrap();
        let slide = presentation.slide_parts(&package).next().unwrap();
        let root = slide.root_element_mut(&mut package).unwrap();
        let table = root
            .common_slide_data
            .shape_tree
            .shape_tree_choice
            .iter_mut()
            .filter_map(|shape| match shape {
                p::ShapeTreeChoice::GraphicFrame(frame) => Some(frame),
                _ => None,
            })
            .flat_map(|frame| frame.graphic.graphic_data.graphic_data_choice.iter_mut())
            .find_map(|data| match data {
                a::GraphicDataChoice::Table(table) => Some(table),
                _ => None,
            })
            .expect("fixture table");
        table
            .table_properties
            .as_mut()
            .unwrap()
            .table_properties_choice3 =
            style_id.map(|id| a::TablePropertiesChoice3::TableStyleId(id.into()));
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        for (value, bold, italic, color) in [
            ("abc", styled, false, if styled { white } else { black }),
            ("def", true, false, black),
            ("hij", false, true, black),
        ] {
            let text = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text == value => Some(text),
                    _ => None,
                })
                .expect(value);
            assert_eq!(text.color, color, "{style_id:?}: {value}");
            assert_eq!(text.style.bold, bold, "{style_id:?}: {value}");
            assert_eq!(text.style.italic, italic, "{style_id:?}: {value}");
        }
    }
}

#[test]
fn hidden_slides_follow_explicit_export_option() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Apache-POI/test-data/slideshow/60810.pptx");
    let package = PresentationDocument::new_from_file(fixture).unwrap();
    for (include_hidden_slides, expected_pages) in [(false, 25), (true, 28)] {
        let document = ooxmlsdk_layout::pptx::layout_document(
            &package,
            &LayoutOptions {
                include_hidden_slides,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(document.pages.len(), expected_pages);
    }
}

#[test]
fn unwrapped_right_aligned_title_preserves_its_right_edge_when_it_overflows() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Pandoc/test/pptx/document-properties/templated.pptx");
    let package = PresentationDocument::new_from_file(fixture).unwrap();
    let document =
        ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
    let items = page_display_items(&document, 0);
    let title = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Text(text) if text.text == "Testing custom properties" => Some(text),
            _ => None,
        })
        .expect("title text");
    // The layout supplies wrap=none and algn=r. Office starts the 96pt
    // title at -70.20pt, outside the slide, retaining its right alignment.
    // Clamping the alignment offset used to place it at +181.20pt.
    assert!(
        (title.origin.x.0 + 70.20).abs() < 5.0,
        "overflowing title origin: {:?}",
        title.origin
    );
}

#[test]
fn text_paragraph_continues_in_the_next_column_before_a_line_overflows() {
    let document = open_xml_sdk_pptx_debug(
        "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/O14ISOStrict/PowerPoint/main_bodyPr_rtlCol.pptx",
    );
    // The configured Office PDF has eight lines in the first column, then
    // three continuation lines in the second. The later paragraph starts in
    // the third column after the authored empty paragraphs consume the gap.
    let mut columns: [String; 3] = Default::default();
    let mut line_positions: [Vec<f32>; 3] = Default::default();
    for item in page_display_items(&document, 0) {
        let DisplayItem::Text(text) = item else {
            continue;
        };
        if text.text.trim().is_empty() {
            continue;
        }
        let column = if text.origin.x.0 < 260.4 {
            0
        } else if text.origin.x.0 < 393.6 {
            1
        } else {
            2
        };
        columns[column].extend(text.text.chars().filter(|ch| !ch.is_whitespace()));
        if !line_positions[column]
            .iter()
            .any(|y| (y - text.origin.y.0).abs() < 0.1)
        {
            line_positions[column].push(text.origin.y.0);
        }
    }
    assert_eq!(
        columns,
        [
            "rtlCol–specficieswhethercolumnsareusedinarighttoleftorlefttorightorder.Setsthecolumnorderusedtodeterminewhich",
            "columnoverflowtextshouldgotonext.",
            "InsertTextbox|DrawingTools|Home|Paragraph|Columns",
        ]
    );
    assert_eq!(line_positions.map(|lines| lines.len()), [8, 3, 5]);
}

#[test]
fn chart_background_images_resolve_stretch_and_tile_resources() {
    for (name, tiled) in [
        ("chart-picture-bg.pptx", false),
        ("chart-texture-bg.pptx", true),
    ] {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/Apache-POI/test-data/slideshow")
            .join(name);
        let package = PresentationDocument::new_from_file(fixture).unwrap();
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let images = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Image(image) => Some(image),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!images.is_empty(), "{name}: chart background resource");
        assert_eq!(images.len() > 1, tiled, "{name}: authored bitmap mode");
        assert!(
            items.iter().all(|item| !matches!(item,
                DisplayItem::Path(path) if matches!(path.fill, Fill::Image { .. })
            )),
            "{name}: unresolved chart fill"
        );
        assert!(images.iter().all(|image| !image.bytes.is_empty()));
        let left = images
            .iter()
            .map(|image| image.bounds.origin.x.0)
            .fold(f32::INFINITY, f32::min);
        let right = images
            .iter()
            .map(|image| image.bounds.origin.x.0 + image.bounds.size.width.0)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((left - 160.0).abs() < 0.01, "{name}: chart left edge");
        assert!((right - 800.0).abs() < 0.01, "{name}: chart right edge");
    }
}

fn open_xml_sdk_pptx_debug(path: &str) -> LayoutDocument<'static> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Open-XML-SDK")
        .join(path);
    let package = PresentationDocument::new_from_file(fixture).unwrap();
    ooxmlsdk_layout::pptx::layout_document(
        &package,
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
// Source: ../core/sd/qa/unit/import-tests-smartart.cxx::testMissingBulletAndIndent
fn pptx_smartart_missing_bullet_preserves_child_bullet_margin() {
    let document = pptx_debug("sd/qa/unit/data/pptx/smartart-missing-bullet.pptx");
    let paragraphs = shapes(&document, "pptx_bullet_paragraph");
    assert!(
        paragraphs.iter().any(|paragraph| {
            debug_text_property(paragraph, "text") == Some("Bullet yes")
                && debug_integer_property(paragraph, "left_margin_100mm") == Some(309)
        }),
        "missing SmartArt child bullet left margin 309; paragraphs={paragraphs:?}"
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
// - ECMA-376 Part 1 §21.2.2.105 (marker) and §21.2.2.142 (series spPr)
// - ../core/chart2/qa/extras/chart2import.cxx::testTdf105517
// - immutable Microsoft PowerPoint fixed-format output for tdf105517.pptx
fn pptx_tdf105517_automatic_marker_size_tracks_the_explicit_series_line() {
    let document = pptx_debug("chart2/qa/extras/data/pptx/tdf105517.pptx");
    let items = page_display_items(&document, 0);
    let blue = Color {
        r: 0x22,
        g: 0x6c,
        b: 0xa9,
        a: 0xff,
    };
    assert!(
        items.iter().any(|item| match item {
            DisplayItem::Path(path) => path
                .stroke
                .as_ref()
                .is_some_and(|stroke| stroke.color == blue && (stroke.width.0 - 6.0).abs() < 0.001),
            _ => false,
        }),
        "missing the authored 6pt blue series line"
    );
    let markers = items
        .iter()
        .filter(|item| match item {
            DisplayItem::Path(path) => {
                path.fill == Fill::Solid(blue)
                    && (path.bounds.size.width.0 - 16.5).abs() < 0.01
                    && (path.bounds.size.height.0 - 15.75).abs() < 0.01
            }
            _ => false,
        })
        .count();
    assert_eq!(
        markers, 5,
        "a 6pt automatic series line must produce Office's 15pt triangle markers"
    );
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

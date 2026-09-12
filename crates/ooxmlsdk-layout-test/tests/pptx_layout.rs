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
fn smartart_snake_rows_keep_six_distinct_child_bullets() {
    let document = pptx_layout("sd/qa/unit/data/pptx/smartart-snake-rows.pptx").unwrap();
    let items = page_display_items(&document, 0);
    let text = items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) => Some(run),
            _ => None,
        })
        .collect::<Vec<_>>();
    let bullets = text
        .iter()
        .copied()
        .filter(|run| run.text == "•")
        .collect::<Vec<_>>();
    assert_eq!(bullets.len(), 6);
    // The current Office PDF has three columns and two rows, with every
    // child bullet above its own gray parent label. Overlapping bullet text
    // is merged by PDFium even when all six display items were emitted.
    for index in 1..=6 {
        let child = text
            .iter()
            .find(|run| run.text == format!("Child {index}"))
            .unwrap();
        let parent = text
            .iter()
            .find(|run| run.text == format!("Parent {index}"))
            .unwrap();
        assert!(
            parent.origin.y.0 - child.origin.y.0 > 40.0,
            "child/parent overlap: {index}"
        );
        assert_eq!(
            bullets
                .iter()
                .filter(|bullet| {
                    (bullet.origin.y.0 - child.origin.y.0).abs() < 0.1
                        && bullet.origin.x.0 < child.origin.x.0
                        && child.origin.x.0 - bullet.origin.x.0 < 30.0
                })
                .count(),
            1,
            "distinct bullet for Child {index}"
        );
        if index <= 3 {
            let below = text
                .iter()
                .find(|run| run.text == format!("Child {}", index + 3))
                .unwrap();
            assert!((below.origin.x.0 - child.origin.x.0).abs() < 0.1);
            assert!((below.origin.y.0 - child.origin.y.0 - 196.2).abs() < 5.0);
        }
    }
}

#[test]
fn smartart_color_lists_preserve_fill_and_line_alpha() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_diagram as dgm;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;

    let fixture =
        ooxmlsdk_layout_test::corpus_file("sd/qa/unit/data/pptx/smartart-snake-rows.pptx");
    // The authored 90% circles overlap opaque parent panels in the current
    // Office PDF. Change fill and line independently to protect both paths.
    for (fill_alpha, line_alpha) in [(90_000, 90_000), (45_000, 30_000), (100_000, 100_000)] {
        let mut package = PresentationDocument::new_from_file(&fixture).unwrap();
        let presentation = package.presentation_part().unwrap();
        let slide = presentation.slide_parts(&package).next().unwrap();
        let colors_part = slide.diagram_colors_parts(&package).next().unwrap();
        let colors = colors_part.root_element_mut(&mut package).unwrap();
        let label = colors
            .color_transform_style_label
            .iter_mut()
            .find(|label| label.name == "fgAccFollowNode1")
            .unwrap();
        let dgm::FillColorListChoice::SchemeColor(fill) = &mut label
            .fill_color_list
            .as_mut()
            .unwrap()
            .fill_color_list_choice[0]
        else {
            panic!("scheme fill");
        };
        let alpha = fill
            .scheme_color_choice
            .iter_mut()
            .find_map(|choice| match choice {
                a::SchemeColorChoice::Alpha(alpha) => Some(alpha),
                _ => None,
            })
            .unwrap();
        alpha.val = ooxmlsdk::units::DrawingmlPercentageValue::Decimal(fill_alpha);
        let dgm::LineColorListChoice::SchemeColor(line) = &mut label
            .line_color_list
            .as_mut()
            .unwrap()
            .line_color_list_choice[0]
        else {
            panic!("scheme line");
        };
        let alpha = line
            .scheme_color_choice
            .iter_mut()
            .find_map(|choice| match choice {
                a::SchemeColorChoice::Alpha(alpha) => Some(alpha),
                _ => None,
            })
            .unwrap();
        alpha.val = ooxmlsdk::units::DrawingmlPercentageValue::Decimal(line_alpha);
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let circles = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Path(path)
                    if (path.bounds.size.width.0 - path.bounds.size.height.0).abs() < 0.1
                        && path.bounds.size.width.0 > 45.0
                        && path.bounds.size.width.0 < 60.0 =>
                {
                    Some(path)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(circles.len(), 6, "six visible circles");
        let expected_fill_alpha = (fill_alpha as f32 / 100_000.0 * 255.0).round() as u8;
        let expected_line_alpha = (line_alpha as f32 / 100_000.0 * 255.0).round() as u8;
        for circle in circles {
            let Fill::Solid(fill) = &circle.fill else {
                panic!("solid circle fill");
            };
            assert_eq!(fill.a, expected_fill_alpha);
            assert_eq!(circle.stroke.as_ref().unwrap().color.a, expected_line_alpha);
        }
        let parents = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Path(path)
                    if path.bounds.size.width.0 > 140.0
                        && path.bounds.size.width.0 < 155.0
                        && path.bounds.size.height.0 > 40.0
                        && path.bounds.size.height.0 < 60.0 =>
                {
                    Some(path)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(parents.len(), 6, "six opaque parent panels");
        for parent in parents {
            let Fill::Solid(fill) = &parent.fill else {
                panic!("solid parent fill");
            };
            assert_eq!(fill.a, 255);
            assert_eq!(parent.stroke.as_ref().unwrap().color.a, 255);
        }
    }
}

#[test]
fn smartart_snake_rows_scale_child_basis_and_keep_transition_spacing() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_diagram as dgm;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk_layout::render::diagram::{DiagramBounds, layout_shapes};

    let fixture =
        ooxmlsdk_layout_test::corpus_file("sd/qa/unit/data/pptx/smartart-snake-rows.pptx");
    let package = PresentationDocument::new_from_file(fixture).unwrap();
    let presentation = package.presentation_part().unwrap();
    let slide = presentation.slide_parts(&package).next().unwrap();
    let data_part = slide.diagram_data_parts(&package).next().unwrap();
    let layout_part = slide
        .diagram_layout_definition_parts(&package)
        .next()
        .unwrap();
    let data = data_part.root_element(&package).unwrap();
    let layout = layout_part.root_element(&package).unwrap();
    for (scale, transition_factor, child_anchor, spacing_reference) in [
        (1.0, 0.08, None, dgm::ConstraintValues::Width),
        (2.0, 0.08, None, dgm::ConstraintValues::Width),
        (1.0, 0.20, None, dgm::ConstraintValues::Width),
        (1.0, 0.08, Some("b"), dgm::ConstraintValues::Width),
        (1.0, 0.08, None, dgm::ConstraintValues::Height),
    ] {
        let mut layout = layout.clone();
        if let Some(anchor) = child_anchor {
            let each = layout
                .layout_node
                .layout_node_choice
                .iter_mut()
                .find_map(|choice| {
                    if let dgm::LayoutNodeChoice::ForEach(each) = choice {
                        Some(each)
                    } else {
                        None
                    }
                })
                .unwrap();
            let composite = each
                .for_each_choice
                .iter_mut()
                .find_map(|choice| {
                    if let dgm::ForEachChoice::LayoutNode(node) = choice {
                        Some(node)
                    } else {
                        None
                    }
                })
                .unwrap();
            let child = composite
                .layout_node_choice
                .iter_mut()
                .find_map(|choice| {
                    if let dgm::LayoutNodeChoice::LayoutNode(node) = choice {
                        (node.name.as_deref() == Some("childRect")).then_some(node)
                    } else {
                        None
                    }
                })
                .unwrap();
            let algorithm = child
                .layout_node_choice
                .iter_mut()
                .find_map(|choice| {
                    if let dgm::LayoutNodeChoice::Algorithm(algorithm) = choice {
                        Some(algorithm)
                    } else {
                        None
                    }
                })
                .unwrap();
            algorithm.parameter.push(dgm::Parameter {
                r#type: dgm::ParameterIdValues::TextAnchorVerticalWithChildren,
                val: Some(anchor.into()),
            });
        }
        let transition = layout
            .layout_node
            .layout_node_choice
            .iter_mut()
            .find_map(|choice| {
                let dgm::LayoutNodeChoice::Constraints(constraints) = choice else {
                    return None;
                };
                constraints.constraint.iter_mut().find(|constraint| {
                    constraint.r#type == dgm::ConstraintValues::Width
                        && constraint.point_type == Some(dgm::ElementValues::SiblingTransition)
                })
            })
            .unwrap();
        transition.fact = Some(transition_factor);
        let spacing = layout
            .layout_node
            .layout_node_choice
            .iter_mut()
            .find_map(|choice| {
                let dgm::LayoutNodeChoice::Constraints(constraints) = choice else {
                    return None;
                };
                constraints
                    .constraint
                    .iter_mut()
                    .find(|constraint| constraint.r#type == dgm::ConstraintValues::Spacing)
            })
            .unwrap();
        spacing.reference_type = Some(spacing_reference);
        let shapes = layout_shapes(
            data,
            Some(&layout),
            None,
            None,
            DiagramBounds {
                x: 0.0,
                y: 0.0,
                width: 647.7453 * scale,
                height: 363.3333 * scale,
            },
            Default::default(),
        );
        let children = (1..=6)
            .map(|index| {
                shapes
                    .iter()
                    .find(|shape| {
                        shape.text_body.paragraphs.iter().any(|paragraph| {
                            paragraph
                                .runs
                                .iter()
                                .any(|run| run.text == format!("Child {index}"))
                        })
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for (index, child) in children.iter().enumerate() {
            assert_eq!(
                child.text_body.body_properties.as_ref().unwrap().anchor,
                Some(if child_anchor.is_some() {
                    a::TextAnchoringTypeValues::Bottom
                } else {
                    a::TextAnchoringTypeValues::Top
                }),
                "descendant text uses the child anchor, including a single list level"
            );
            // Office child panels are approximately 147 x 110 points. The
            // composite's w=1 is a basis for these slots, not a final extent.
            assert!((child.width / scale - 147.0).abs() < 3.0, "{child:?}");
            assert!((child.height / scale - 110.0).abs() < 3.0, "{child:?}");
            if index >= 3 {
                assert!((child.x - children[index - 3].x).abs() < 0.1);
                let above = children[index - 3];
                let composite_width = child.width / 0.923;
                let composite_height = child.height / 0.65;
                let reference_extent = if spacing_reference == dgm::ConstraintValues::Width {
                    composite_width
                } else {
                    composite_height
                };
                let row_gap = child.y - above.y - composite_height;
                assert!(
                    (row_gap / reference_extent - 0.16).abs() < 0.001,
                    "row spacing retains its authored reference: {row_gap} / {reference_extent}"
                );
            }
            if index % 3 != 2 {
                let composite_width = child.width / 0.923;
                let gap_factor = (children[index + 1].x - child.x) / composite_width - 1.0;
                assert!(
                    (gap_factor - transition_factor as f32).abs() < 0.001,
                    "authored sibling transition spacing: {gap_factor}"
                );
            }
        }
        let parent = shapes
            .iter()
            .find(|shape| {
                shape
                    .text_body
                    .paragraphs
                    .iter()
                    .any(|paragraph| paragraph.runs.iter().any(|run| run.text == "Parent 1"))
            })
            .unwrap();
        assert_eq!(
            parent.text_body.body_properties.as_ref().unwrap().anchor,
            Some(a::TextAnchoringTypeValues::Center),
            "parent text retains its independent default"
        );
    }
}

#[test]
fn waterfall_resource_legend_uses_ui_font_and_preserves_authored_fonts() {
    use ooxmlsdk::schemas::schemas_microsoft_com_office_drawing_2014_chartex as cx;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    let fixture =
        ooxmlsdk_layout_test::corpus_file("sd/qa/unit/data/pptx/forum-mso-de-138303.pptx");
    // theme1.xml declares Hant=新細明體 (PMingLiU), Hans=等线 (DengXian).
    // Display styles preserve those source names; PDF font names use aliases.
    for explicit_face in [None, Some("SimSun")] {
        // The source stores ChartEx in a Microsoft 365 AlternateContent branch.
        // Use the same MCE target as the suite helper and configured converter.
        let mut package = PresentationDocument::new_with_settings(
            std::fs::File::open(&fixture).unwrap(),
            ooxmlsdk::sdk::OpenSettings {
                markup_compatibility_process_settings:
                    ooxmlsdk::sdk::MarkupCompatibilityProcessSettings {
                        process_mode:
                            ooxmlsdk::sdk::MarkupCompatibilityProcessMode::ProcessLoadedPartsOnly,
                        target_file_format_version: ooxmlsdk::sdk::FileFormatVersion::Microsoft365,
                    },
                ..Default::default()
            },
        )
        .unwrap();
        if let Some(face) = explicit_face {
            let presentation = package.presentation_part().unwrap();
            let slide = presentation.slide_parts(&package).next().unwrap();
            let chart = slide.extended_chart_parts(&package).next().unwrap();
            let chart = chart.root_element_mut(&mut package).unwrap();
            let properties = a::DefaultRunProperties {
                east_asian_font: Some(a::EastAsianFont {
                    typeface: Some(face.into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            chart.chart.legend.as_mut().unwrap().tx_pr_text_body =
                Some(Box::new(cx::TxPrTextBody {
                    paragraph: vec![a::Paragraph {
                        paragraph_properties: Some(Box::new(a::ParagraphProperties {
                            default_run_properties: Some(Box::new(properties)),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }],
                    ..Default::default()
                }));
        }
        let document = ooxmlsdk_layout::pptx::layout_document(
            &package,
            &LayoutOptions {
                ui_language: Some("zh-TW".into()),
                default_document_language: Some("zh-CN".into()),
                format_locale: Some("de-DE".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let items = page_display_items(&document, 0);
        for name in ["增加", "減少", "合計"] {
            let run = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text == name => Some(text),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    panic!("missing {name}; text={:?}", all_page_text(&document, 0))
                });
            assert_eq!(
                run.style.east_asia_font_family.as_deref(),
                Some(explicit_face.unwrap_or("新細明體"))
            );
            assert!((run.style.font_size.0 - 9.0).abs() < 0.01);
        }
        let category = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text == "Kategorie 1" => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(category.style.font_family.as_deref(), Some("Calibri"));
        assert_eq!(
            category.style.east_asia_font_family.as_deref(),
            Some("等线")
        );
        let title = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text == "圖表標題" => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            title.style.east_asia_font_family.as_deref(),
            Some("新細明體")
        );
    }
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
        font.typeface = Some(bullet_font.into());

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
fn predefined_medium_table_style_bolds_enabled_edges_and_preserves_direct_formatting() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;

    // The configured Office PDF renders this fixture's header in Calibri Bold.
    // MS-OE376 §2.1.1343(b) defines the same bold edge regions for Medium Style 2
    // and each accent variant. The package omits the built-in style definition.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Pandoc/test/pptx/tables/output.pptx");
    for style_id in [
        "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
        "{073A0DAA-6AF3-43AB-8588-CEC1D06C72B9}",
        "{93296810-A885-4BE3-A3E7-6D5BEEA58F35}",
    ] {
        for (first_row, last_row, first_column, last_column) in [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, true),
            (false, false, false, false),
            (true, true, true, true),
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
                .unwrap();
            let properties = table.table_properties.as_mut().unwrap();
            properties.first_row = Some(first_row.into());
            properties.last_row = Some(last_row.into());
            properties.first_column = Some(first_column.into());
            properties.last_column = Some(last_column.into());
            properties.table_properties_choice3 =
                Some(a::TablePropertiesChoice3::TableStyleId(style_id.into()));
            let mut expected = Vec::new();
            for (row_index, row) in table.table_row.iter_mut().enumerate() {
                for (column_index, cell) in row.table_cell.iter_mut().enumerate() {
                    let a::ParagraphChoice::Run(run) =
                        &mut cell.text_body.as_mut().unwrap().paragraph[0].paragraph_choice[0]
                    else {
                        panic!("fixture cell run");
                    };
                    let label = format!("R{row_index}C{column_index}");
                    run.text = label.clone();
                    let direct_bold = match (row_index, column_index) {
                        (0, 0) => Some(false),
                        (1, 1) => Some(true),
                        _ => None,
                    };
                    run.run_properties.get_or_insert_with(Default::default).bold =
                        direct_bold.map(Into::into);
                    let edge = (first_row && row_index == 0)
                        || (last_row && row_index == 3)
                        || (first_column && column_index == 0)
                        || (last_column && column_index == 3);
                    expected.push((label, direct_bold.unwrap_or(edge), edge));
                }
            }
            let document =
                ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default())
                    .unwrap();
            let items = page_display_items(&document, 0);
            let border_widths = items
                .iter()
                .filter_map(|item| match item {
                    DisplayItem::Line(line) => Some(line.stroke.width.0),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!border_widths.is_empty());
            // Office's built-in XML and this fixture's native PDF use 1pt
            // grid lines and 3pt separators on the selected edge rows.
            assert!(
                border_widths
                    .iter()
                    .all(|width| (*width - 1.0).abs() < 0.001 || (*width - 3.0).abs() < 0.001),
                "{border_widths:?}"
            );
            assert_eq!(
                border_widths
                    .iter()
                    .filter(|width| (**width - 3.0).abs() < 0.001)
                    .count(),
                4 * (usize::from(first_row) + usize::from(last_row)),
                "only first/last row separators use the thick pen"
            );
            for (label, bold, edge) in expected {
                let text = items
                    .iter()
                    .find_map(|item| match item {
                        DisplayItem::Text(text) if text.text == label => Some(text),
                        _ => None,
                    })
                    .expect(&label);
                assert_eq!(text.style.bold, bold, "{style_id}: {label}");
                assert!(!text.style.italic, "{style_id}: {label}");
                assert_eq!(
                    text.color,
                    Color {
                        r: if edge { 255 } else { 0 },
                        g: if edge { 255 } else { 0 },
                        b: if edge { 255 } else { 0 },
                        a: 255,
                    },
                    "{style_id}: {label}"
                );
                assert!((text.style.font_size.0 - 13.5).abs() < 0.1, "{text:?}");
            }
        }
    }
}

#[test]
fn zero_height_table_rows_render_text_using_the_master_other_style() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Pandoc/test/pptx/slide-level-0/h1-with-table/output.pptx");
    let package = PresentationDocument::new_from_file(fixture).unwrap();
    let document =
        ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
    let items = page_display_items(&document, 0);
    let cell = items
        .iter()
        .find_map(|item| match item {
            DisplayItem::Text(text) if text.text == "1" => Some(text),
            _ => None,
        })
        .expect("Office renders the h=0 table row");
    // The current configured Office PDF has a short row at y=94 pt, not
    // the 267 pt graphic-frame height. Its cell inherits otherStyle sz=1350.
    assert!((cell.style.font_size.0 - 13.5).abs() < 0.1, "{cell:?}");
    assert!((cell.origin.x.0 - 43.2).abs() < 0.2, "{cell:?}");
    assert!(
        cell.origin.y.0 > 94.0 && cell.origin.y.0 < 105.0,
        "{cell:?}"
    );
    assert!(
        items.iter().any(|item| matches!(item,
            DisplayItem::Rect(rect) if (rect.bounds.origin.y.0 - 94.0).abs() < 0.1
                && rect.bounds.size.width.0 > 640.0
                && rect.bounds.size.height.0 > 18.0 && rect.bounds.size.height.0 < 28.0
        )),
        "automatic row is measured from its content and insets"
    );
}

#[test]
fn zero_height_table_rows_measure_content_and_preserve_fixed_and_merged_rows() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;
    use ooxmlsdk::simple_type::CoordinateValue;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Pandoc/test/pptx/slide-level-0/h1-with-table/output.pptx");
    for (large_font, fixed_height, merged) in [
        (false, 0, false),
        (true, 0, false),
        (false, 70, false),
        (false, 0, true),
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
            .unwrap();
        let mut second_row = table.table_row[0].clone();
        let a::ParagraphChoice::Run(run) = &mut second_row.table_cell[0]
            .text_body
            .as_mut()
            .unwrap()
            .paragraph[0]
            .paragraph_choice[0]
        else {
            panic!("cell run");
        };
        run.text = "2".into();
        table.table_row[0].height = CoordinateValue::Emu(fixed_height * 12_700);
        if large_font {
            let a::ParagraphChoice::Run(run) = &mut table.table_row[0].table_cell[0]
                .text_body
                .as_mut()
                .unwrap()
                .paragraph[0]
                .paragraph_choice[0]
            else {
                panic!("cell run");
            };
            run.run_properties
                .get_or_insert_with(Default::default)
                .font_size = Some(2700);
        }
        if merged {
            table.table_row[0].table_cell[0].row_span = Some(2);
            second_row.table_cell[0].vertical_merge = Some(true.into());
        }
        table.table_row.push(second_row);
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let runs = items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Text(text) => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>();
        let first = runs.iter().find(|text| text.text == "1").unwrap();
        assert_eq!(runs.iter().filter(|text| text.text == "1").count(), 1);
        let second = runs.iter().find(|text| text.text == "2");
        if merged {
            assert!(
                second.is_none(),
                "merged continuation must not emit another cell"
            );
        } else {
            let separation = second.unwrap().origin.y.0 - first.origin.y.0;
            if fixed_height > 0 {
                assert!(
                    (separation - fixed_height as f32).abs() < 0.1,
                    "{separation}"
                );
            } else if large_font {
                assert!(
                    separation > 30.0 && separation < 45.0,
                    "large font: {separation}"
                );
            } else {
                assert!(
                    separation > 18.0 && separation < 28.0,
                    "automatic height: {separation}"
                );
            }
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
fn drawingml_baseline_scales_script_fonts_before_measuring_text() {
    // These current Office PDFs use identity text matrices and script sizes
    // equal to two-thirds of the nominal font, quantized on the print grid.
    for (file, script, script_size, baseline) in [
        (
            "Pandoc/test/pptx/inline-formatting/output.pptx",
            "script",
            15.96,
            7.2,
        ),
        (
            "Pandoc/test/pptx/inline-formatting/templated.pptx",
            "script",
            18.72,
            8.4,
        ),
        (
            "LibreOffice/sd/qa/unit/data/pptx/n828390.pptx",
            "R",
            13.32,
            -5.0,
        ),
    ] {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus")
            .join(file);
        let package = PresentationDocument::new_from_file(fixture).unwrap();
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let text = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text == script => Some(text),
                _ => None,
            })
            .expect(script);
        assert!(
            (text.style.font_size.0 - script_size).abs() < 0.06,
            "{file}: {text:?}"
        );
        assert!(
            (text.style.baseline_shift.0 - baseline).abs() < 0.06,
            "{file}: {text:?}"
        );
        if file.ends_with("inline-formatting/output.pptx") {
            let subscript = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text == "2" => Some(text),
                    _ => None,
                })
                .unwrap();
            assert!((subscript.style.font_size.0 - 15.96).abs() < 0.06);
            assert!((subscript.style.baseline_shift.0 + 6.0).abs() < 0.06);
            let following = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text.starts_with("0)") => Some(text),
                    _ => None,
                })
                .unwrap();
            assert!((following.style.font_size.0 - 24.0).abs() < 0.06);
            assert_eq!(following.style.baseline_shift.0, 0.0);
        }
    }
}

#[test]
fn drawingml_baseline_inheritance_uses_run_size_and_zero_restores_normal_text() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;
    use ooxmlsdk::units::DrawingmlPercentageValue;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Pandoc/test/pptx/inline-formatting/output.pptx");
    // Repeating the same baseline in rPr after defRPr must not apply the
    // script-size factor a second time.
    for repeat_baseline in [false, true] {
        let mut package = PresentationDocument::new_from_file(&fixture).unwrap();
        let presentation = package.presentation_part().unwrap();
        let slide = presentation.slide_parts(&package).next().unwrap();
        let root = slide.root_element_mut(&mut package).unwrap();
        let paragraph = root
            .common_slide_data
            .shape_tree
            .shape_tree_choice
            .iter_mut()
            .filter_map(|shape| match shape {
                p::ShapeTreeChoice::Shape(shape) => shape.text_body.as_mut(),
                _ => None,
            })
            .flat_map(|body| body.paragraph.iter_mut())
            .find(|paragraph| {
                paragraph.paragraph_choice.iter().any(
                    |choice| matches!(choice, a::ParagraphChoice::Run(run) if run.text == "script"),
                )
            })
            .unwrap();
        let defaults = paragraph
            .paragraph_properties
            .get_or_insert_with(Default::default)
            .default_run_properties
            .get_or_insert_with(Default::default);
        defaults.font_size = Some(3000);
        defaults.baseline = Some(DrawingmlPercentageValue::Decimal(30000));
        for choice in &mut paragraph.paragraph_choice {
            let a::ParagraphChoice::Run(run) = choice else {
                continue;
            };
            let properties = run.run_properties.get_or_insert_with(Default::default);
            properties.baseline = if run.text == "script" && repeat_baseline {
                Some(DrawingmlPercentageValue::Decimal(30000))
            } else if run.text == "script" || run.text == "2" {
                None
            } else {
                Some(DrawingmlPercentageValue::Decimal(0))
            };
            properties.font_size = if run.text == "script" {
                Some(4800)
            } else {
                None
            };
        }
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        for (value, size, baseline) in [("script", 32.04, 14.4), ("2", 20.04, 9.0)] {
            let text = items
                .iter()
                .find_map(|item| match item {
                    DisplayItem::Text(text) if text.text == value => Some(text),
                    _ => None,
                })
                .expect(value);
            assert!((text.style.font_size.0 - size).abs() < 0.06, "{text:?}");
            assert!(
                (text.style.baseline_shift.0 - baseline).abs() < 0.06,
                "{text:?}"
            );
        }
        let normal = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text.starts_with("0)") => Some(text),
                _ => None,
            })
            .unwrap();
        assert!((normal.style.font_size.0 - 30.0).abs() < 0.06, "{normal:?}");
        assert_eq!(normal.style.baseline_shift.0, 0.0);
    }
}

#[test]
fn hyperlink_bullets_keep_character_color_and_honor_explicit_overrides() {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
    use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main as p;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/Pandoc/test/pptx/slide-breaks-toc/output.pptx");
    let black = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    let blue = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    let red = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    let yellow = Color {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    };
    for (run_red, bullet_yellow, mouse_over) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let mut package = PresentationDocument::new_from_file(&fixture).unwrap();
        let presentation = package.presentation_part().unwrap();
        let slide = presentation.slide_parts(&package).next().unwrap();
        let root = slide.root_element_mut(&mut package).unwrap();
        let paragraph = root
            .common_slide_data
            .shape_tree
            .shape_tree_choice
            .iter_mut()
            .filter_map(|shape| match shape {
                p::ShapeTreeChoice::Shape(shape) => shape.text_body.as_mut(),
                _ => None,
            })
            .flat_map(|body| body.paragraph.iter_mut())
            .find(|paragraph| {
                paragraph.paragraph_choice.iter().any(|choice|
                matches!(choice, a::ParagraphChoice::Run(run) if run.text == "Below section-level")
            )
            })
            .unwrap();
        if bullet_yellow {
            paragraph
                .paragraph_properties
                .get_or_insert_with(Default::default)
                .paragraph_properties_choice1 = Some(a::ParagraphPropertiesChoice::BulletColor(
                Box::new(a::BulletColor {
                    bullet_color_choice: Some(a::BulletColorChoice::PresetColor(a::PresetColor {
                        val: a::PresetColorValues::Yellow,
                        ..Default::default()
                    })),
                }),
            ));
        }
        let a::ParagraphChoice::Run(run) = &mut paragraph.paragraph_choice[0] else {
            panic!("fixture link run");
        };
        let properties = run.run_properties.as_mut().unwrap();
        if run_red {
            properties.run_properties_choice1 =
                Some(a::RunPropertiesChoice::SolidFill(Box::new(a::SolidFill {
                    solid_fill_choice: Some(a::SolidFillChoice::PresetColor(a::PresetColor {
                        val: a::PresetColorValues::Red,
                        ..Default::default()
                    })),
                    ..Default::default()
                })));
        }
        if mouse_over {
            let click = properties.hyperlink_on_click.take().unwrap();
            properties.hyperlink_on_mouse_over = Some(Box::new(a::HyperlinkOnMouseOver {
                id: click.id.clone(),
                action: click.action.clone(),
                ..Default::default()
            }));
        }
        let document =
            ooxmlsdk_layout::pptx::layout_document(&package, &LayoutOptions::default()).unwrap();
        let items = page_display_items(&document, 0);
        let bullet = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text == "•" => Some(text),
                _ => None,
            })
            .unwrap();
        let text = items
            .iter()
            .find_map(|item| match item {
                DisplayItem::Text(text) if text.text.starts_with("Below") => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            bullet.color,
            if bullet_yellow {
                yellow
            } else if run_red {
                red
            } else {
                black
            }
        );
        assert_eq!(text.color, if run_red { red } else { blue });
        assert!(!bullet.style.underline);
        assert!(text.style.underline);
        assert!((bullet.style.font_size.0 - 24.0).abs() < 0.06);
        assert!((text.style.font_size.0 - 24.0).abs() < 0.06);
    }
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
        // Column origins accumulate f32 inset/width arithmetic. A third-column
        // origin can be 393.59998pt, just below its mathematical boundary;
        // keep that rounding error from assigning its first run to column two.
        let column = if text.origin.x.0 < 260.4 - 0.01 {
            0
        } else if text.origin.x.0 < 393.6 - 0.01 {
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

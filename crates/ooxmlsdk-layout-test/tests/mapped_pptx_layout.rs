use ooxmlsdk::parts::presentation_document::PresentationDocument;
use ooxmlsdk_layout_test::{
    assert_page_contains, assert_page_contains_in_order, assert_page_filled_path_count_at_least,
    assert_page_image_count_at_least, assert_page_not_contains,
    assert_page_stroked_path_count_at_least, assert_page_text_occurrences_at_least, corpus_file,
    pptx_layout,
};

#[derive(Clone, Copy)]
struct PptxCase {
    name: &'static str,
    source: &'static str,
    file: &'static str,
    page_count: Option<usize>,
    ordered: &'static [PageTexts],
    contains: &'static [PageText],
    absent: &'static [PageText],
    occurrences_at_least: &'static [PageTextCount],
    image_minimums: &'static [PageCount],
    stroked_path_minimums: &'static [PageCount],
    filled_path_minimums: &'static [PageCount],
}

#[derive(Clone, Copy)]
struct PageTexts {
    page: usize,
    texts: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct PageText {
    page: usize,
    text: &'static str,
}

#[derive(Clone, Copy)]
struct PageTextCount {
    page: usize,
    text: &'static str,
    count: usize,
}

#[derive(Clone, Copy)]
struct PageCount {
    page: usize,
    count: usize,
}

macro_rules! ordered {
    ($page:expr, [$($text:expr),+ $(,)?]) => {
        PageTexts {
            page: $page,
            texts: &[$($text),+],
        }
    };
}

macro_rules! pt {
    ($page:expr, $text:expr) => {
        PageText {
            page: $page,
            text: $text,
        }
    };
}

macro_rules! count {
    ($page:expr, $text:expr, $count:expr) => {
        PageTextCount {
            page: $page,
            text: $text,
            count: $count,
        }
    };
}

macro_rules! page_count {
    ($page:expr, $count:expr) => {
        PageCount {
            page: $page,
            count: $count,
        }
    };
}

macro_rules! case_inner {
    (
        $name:ident,
        source: $source:expr,
        file: $file:expr,
        page_count: $page_count:expr
        $(, ordered: [$($ordered:expr),* $(,)?])?
        $(, contains: [$($contains:expr),* $(,)?])?
        $(, absent: [$($absent:expr),* $(,)?])?
        $(, occurrences_at_least: [$($occurrences_at_least:expr),* $(,)?])?
        $(, image_minimums: [$($image_minimums:expr),* $(,)?])?
        $(, stroked_path_minimums: [$($stroked_path_minimums:expr),* $(,)?])?
        $(, filled_path_minimums: [$($filled_path_minimums:expr),* $(,)?])?
        $(,)?
    ) => {
        PptxCase {
            name: stringify!($name),
            source: $source,
            file: $file,
            page_count: $page_count,
            ordered: &[$($($ordered),*)?],
            contains: &[$($($contains),*)?],
            absent: &[$($($absent),*)?],
            occurrences_at_least: &[$($($occurrences_at_least),*)?],
            image_minimums: &[$($($image_minimums),*)?],
            stroked_path_minimums: &[$($($stroked_path_minimums),*)?],
            filled_path_minimums: &[$($($filled_path_minimums),*)?],
        }
    };
}

macro_rules! case {
    (
        $name:ident,
        source: $source:expr,
        file: $file:expr,
        pages: $pages:expr
        $(, $field:ident: [$($values:expr),* $(,)?])*
        $(,)?
    ) => {
        case_inner!(
            $name,
            source: $source,
            file: $file,
            page_count: Some($pages)
            $(, $field: [$($values),*])*
        )
    };
    (
        $name:ident,
        source: $source:expr,
        file: $file:expr
        $(, $field:ident: [$($values:expr),* $(,)?])*
        $(,)?
    ) => {
        case_inner!(
            $name,
            source: $source,
            file: $file,
            page_count: None
            $(, $field: [$($values),*])*
        )
    };
}

const CASES: &[PptxCase] = &[
    case!(
        trailing_paragraphs,
        source: "../core/sd/qa/unit/layout-tests.cxx:testTdf168010_PPTX",
        file: "sd/qa/unit/data/pptx/trailing-paragraphs.pptx",
        ordered: [ordered!(0, ["textbox"])],
    ),
    case!(
        tdf104722,
        source: "../core/sd/qa/unit/layout-tests.cxx:testTdf104722",
        file: "sd/qa/unit/data/pptx/tdf104722.pptx",
        ordered: [ordered!(0, ["Subtitle for this part"])],
    ),
    case!(
        table_vertical_text,
        source: "../core/sd/qa/unit/layout-tests.cxx:testTableVerticalText",
        file: "sd/qa/unit/data/pptx/tcPr-vert-roundtrip.pptx",
        contains: [pt!(0, "Abcdefg-90-degrees"), pt!(0, "12345-270-degrees")],
    ),
    case!(
        tdf128212,
        source: "../core/sd/qa/unit/layout-tests.cxx:testTdf128212",
        file: "sd/qa/unit/data/pptx/tdf128212.pptx",
        ordered: [ordered!(0, ["Vertical it should be!"])],
    ),
    case!(
        tdf148966,
        source: "../core/sd/qa/unit/layout-tests.cxx:testTdf148966",
        file: "sd/qa/unit/data/pptx/tdf148966.pptx",
        ordered: [ordered!(
            0,
            [
                "Some multi line hyperlink/field",
                "text that follows after a",
                "linebreak"
            ]
        )],
    ),
    case!(
        tdf128206,
        source: "../core/sd/qa/unit/layout-tests.cxx:testTdf128206",
        file: "sd/qa/unit/data/pptx/tdf128206.pptx",
        ordered: [ordered!(0, ["a b c d e f g h I j k l m n o p q"])],
    ),
    case!(
        smoketest,
        source: "../core/sd/qa/unit/import-tests.cxx:testSmoketest",
        file: "sd/qa/unit/data/smoketest.pptx",
        pages: 1,
        ordered: [ordered!(0, ["Hello", "Radekski :-)"])],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf150770,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf150770",
        file: "sd/qa/unit/data/pptx/tdf150770.pptx",
        pages: 4,
    ),
    case!(
        bnc591147,
        source: "../core/sd/qa/unit/import-tests3.cxx:testBnc591147",
        file: "sd/qa/unit/data/pptx/bnc591147.pptx",
        pages: 2,
    ),
    case!(
        tdf103792,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf103792",
        file: "sd/qa/unit/data/pptx/tdf103792.pptx",
        ordered: [ordered!(0, ["Click to add Title"])],
    ),
    case!(
        tdf119649,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf119649",
        file: "sd/qa/unit/data/pptx/tdf119649.pptx",
        ordered: [ordered!(0, ["default_color(", "colored_text", ")"])],
    ),
    case!(
        tdf103800,
        source: "../core/sd/qa/unit/import-tests3.cxx:testTdf103800",
        file: "sd/qa/unit/data/pptx/tdf103800.pptx",
        ordered: [ordered!(0, ["test"])],
    ),
    case!(
        tdf89927,
        source: "../core/sd/qa/unit/import-tests3.cxx:testTdf89927",
        file: "sd/qa/unit/data/pptx/tdf89927.pptx",
        ordered: [ordered!(0, ["TEST"])],
    ),
    case!(
        tdf137367,
        source: "../core/sd/qa/unit/import-tests.cxx:testHyperlinkColor",
        file: "sd/qa/unit/data/pptx/tdf137367.pptx",
        ordered: [ordered!(
            0,
            [
                "hyperlink color 1",
                "hyperlink color 2",
                "hyperlink color 3"
            ]
        )],
    ),
    case!(
        n828390_2,
        source: "../core/sd/qa/unit/import-tests.cxx:testN828390_2",
        file: "sd/qa/unit/data/pptx/n828390_2.pptx",
        ordered: [ordered!(0, ["Linux", "Standard Platform"])],
    ),
    case!(
        tdf150719,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf150719",
        file: "sd/qa/unit/data/pptx/tdf150719.pptx",
        ordered: [ordered!(0, ["Jump", "to", "Slide 2"])],
    ),
    case!(
        tdf103477,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf103477",
        file: "sd/qa/unit/data/pptx/tdf103477.pptx",
        ordered: [ordered!(0, ["nnnn"])],
    ),
    case!(
        tdf156718,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf156718",
        file: "sd/qa/unit/data/pptx/tdf156718.pptx",
        stroked_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf151767,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf151767",
        file: "sd/qa/unit/data/pptx/tdf151767.pptx",
        stroked_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        table_border_line_style,
        source: "../core/sd/qa/unit/import-tests.cxx:testTableBorderLineStyle",
        file: "sd/qa/unit/data/pptx/tableBorderLineStyle.pptx",
        contains: [
            pt!(0, "System Dash"),
            pt!(0, "System Dot"),
            pt!(0, "System Dash Dot"),
            pt!(0, "Solid"),
            pt!(0, "No Border")
        ],
        stroked_path_minimums: [page_count!(0, 10)],
    ),
    case!(
        bnc862510_7,
        source: "../core/sd/qa/unit/import-tests3.cxx:testBnc862510_7",
        file: "sd/qa/unit/data/pptx/bnc862510_7.pptx",
        ordered: [ordered!(0, ["Text aligned to center"])],
    ),
    case!(
        n83889,
        source: "../core/sd/qa/unit/import-tests.cxx:testN83889",
        file: "sd/qa/unit/data/pptx/n83889.pptx",
        ordered: [ordered!(0, ["test:", "In test 1", "Second line"])],
    ),
    case!(
        tdf152070,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf152070",
        file: "sd/qa/unit/data/pptx/tdf152070.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf51340,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf51340",
        file: "sd/qa/unit/data/pptx/tdf51340.pptx",
        ordered: [ordered!(
            0,
            [
                "Spacing is set on master slide",
                "Spacing is set on slide layout",
                "Direct formatting overrides master slide spacing",
                "Direct formatting overrides slide layout spacing"
            ]
        )],
    ),
    case!(
        tdf120028,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf120028",
        file: "sd/qa/unit/data/pptx/tdf120028.pptx",
        ordered: [ordered!(
            0,
            [
                "Aaaaaaa aaaaa",
                "Bbbbbb bbbbbbbb bbbbbbbb",
                "Ccccccccc ccc cccccc",
                "Dddddd dddddd",
                "Lll l llllll lllll"
            ]
        )],
    ),
    case!(
        tdf100926,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf100926",
        file: "sd/qa/unit/data/pptx/tdf100926.pptx",
        ordered: [ordered!(
            0,
            [
                "Top to Bottom vertical text",
                "Bottom to Top vertical text",
                "Horizontal text"
            ]
        )],
    ),
    case!(
        tdf134174,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf134174",
        file: "sd/qa/unit/data/pptx/tdf134174.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf134210,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf134210",
        file: "sd/qa/unit/data/pptx/tdf134210.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf114821,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf114821",
        file: "sd/qa/unit/data/pptx/tdf114821.pptx",
        ordered: [ordered!(0, ["90.0", "B"])],
    ),
    case!(
        tdf148685,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf148685",
        file: "sd/qa/unit/data/pptx/tdf148685.pptx",
        ordered: [ordered!(0, ["TEXT", "TE", "XT"])],
    ),
    case!(
        tdf128684,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf128684",
        file: "sd/qa/unit/data/pptx/tdf128684.pptx",
        ordered: [ordered!(0, ["Foo bar foo bar foo bar"])],
    ),
    case!(
        tdf113198,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf113198",
        file: "sd/qa/unit/data/pptx/tdf113198.pptx",
        ordered: [ordered!(0, ["Awesome text in center"])],
    ),
    case!(
        tdf149206,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf149206",
        file: "sd/qa/unit/data/pptx/tdf149206.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf163852,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf163852",
        file: "sd/qa/unit/data/pptx/tdf163852.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        crop_to_shape,
        source: "../core/sd/qa/unit/import-tests.cxx:testCropToShape",
        file: "sd/qa/unit/data/pptx/crop-to-shape.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        mirrored_graphic,
        source: "../core/sd/qa/unit/import-tests.cxx:testMirroredGraphic",
        file: "sd/qa/unit/data/pptx/mirrored-graphic.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf128596,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf128596",
        file: "sd/qa/unit/data/pptx/tdf128596.pptx",
        image_minimums: [page_count!(0, 2)],
    ),
    case!(
        transparent_white_text,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf151547",
        file: "sd/qa/unit/data/pptx/tdf151547-transparent-white-text.pptx",
        ordered: [ordered!(0, ["Fully transparent white text"])],
    ),
    case!(
        transparent_solid_fill,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf149588",
        file: "sd/qa/unit/data/pptx/tdf149588_transparentSolidFill.pptx",
        ordered: [ordered!(0, ["EDGE"])],
    ),
    case!(
        tdf79007,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf79007",
        file: "sd/qa/unit/data/pptx/tdf79007.pptx",
        pages: 1,
    ),
    case!(
        tdf104445,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf104445",
        file: "sd/qa/unit/data/pptx/tdf104445.pptx",
        pages: 1,
        absent: [pt!(0, "• Tartalom helye 2"), pt!(0, "Click to add Text")],
    ),
    case!(
        n80340,
        source: "../core/sd/qa/unit/import-tests.cxx:testN80340",
        file: "sd/qa/unit/data/pptx/n80340.pptx",
        ordered: [ordered!(0, ["Yogesh"])],
    ),
    case!(
        tablescale,
        source: "../core/sd/qa/unit/import-tests.cxx:testTableScale",
        file: "sd/qa/unit/data/pptx/tablescale.pptx",
        ordered: [ordered!(0, ["xxx", "yyy"])],
    ),
    case!(
        tdf62255,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf62255",
        file: "sd/qa/unit/data/pptx/tdf62255.pptx",
        ordered: [ordered!(0, ["Test"])],
        stroked_path_minimums: [page_count!(0, 4)],
    ),
    case!(
        tdf127964,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf127964",
        file: "sd/qa/unit/data/pptx/tdf127964.pptx",
        stroked_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf106638,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf106638",
        file: "sd/qa/unit/data/pptx/tdf106638.pptx",
        ordered: [ordered!(0, ["stratégique si la France veut se positionner"])],
    ),
    case!(
        formatting_bullet_indent,
        source: "../core/sd/qa/unit/import-tests.cxx:testFormattingBulletIndent",
        file: "sd/qa/unit/data/pptx/formatting-bullet-indent.pptx",
        ordered: [ordered!(
            0,
            ["Paragraph with indent", "Paragraph without indent."]
        )],
    ),
    case!(
        tdf153008_src_rect,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf153008",
        file: "sd/qa/unit/data/pptx/tdf153008-srcRect-smallNegBound.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        smartart1,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArt1",
        file: "sd/qa/unit/data/pptx/smartart1.pptx",
        ordered: [ordered!(0, ["a", "b", "c", "d", "e"])],
    ),
    case!(
        smartart_children,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtChildren",
        file: "sd/qa/unit/data/pptx/smartart-children.pptx",
        ordered: [ordered!(0, ["a", "b", "c", "x", "y", "z"])],
    ),
    case!(
        smartart_text,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtText",
        file: "sd/qa/unit/data/pptx/smartart-text.pptx",
        ordered: [ordered!(0, ["test"])],
    ),
    case!(
        smartart_cnt,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtCnt",
        file: "sd/qa/unit/data/pptx/smartart-cnt.pptx",
        ordered: [ordered!(0, ["a", "b", "c"])],
    ),
    case!(
        smartart_dir,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtDir",
        file: "sd/qa/unit/data/pptx/smartart-dir.pptx",
        ordered: [ordered!(0, ["first", "second"])],
    ),
    case!(
        tdf148665,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf148665",
        file: "sd/qa/unit/data/pptx/tdf148665.pptx",
        contains: [pt!(0, "Fufufu"), pt!(0, "Susu"), pt!(0, "Sasa Haha")],
    ),
    case!(
        tdf148921,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf148921",
        file: "sd/qa/unit/data/pptx/tdf148921.pptx",
        ordered: [ordered!(0, ["Test"])],
    ),
    case!(
        smartart_maxdepth,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtMaxDepth",
        file: "sd/qa/unit/data/pptx/smartart-maxdepth.pptx",
        ordered: [ordered!(0, ["first", "second"])],
    ),
    case!(
        smartart_rotation,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtRotation",
        file: "sd/qa/unit/data/pptx/smartart-rotation.pptx",
        ordered: [ordered!(0, ["a", "b", "c"])],
        stroked_path_minimums: [page_count!(0, 3)],
    ),
    case!(
        smartart_pyramid_1child,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtPyramid1Child",
        file: "sd/qa/unit/data/pptx/smartart-pyramid-1child.pptx",
        ordered: [ordered!(0, ["A"])],
    ),
    case!(
        smartart_chevron,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtChevron",
        file: "sd/qa/unit/data/pptx/smartart-chevron.pptx",
        ordered: [ordered!(0, ["a", "b", "c"])],
    ),
    case!(
        smartart_cycle,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtCycle",
        file: "sd/qa/unit/data/pptx/smartart-cycle.pptx",
        contains: [pt!(0, "a"), pt!(0, "b"), pt!(0, "c"), pt!(0, "d"), pt!(0, "e")],
        stroked_path_minimums: [page_count!(0, 5)],
    ),
    case!(
        smartart_right_to_left_block,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtRightToLeftBlockDiagram",
        file: "sd/qa/unit/data/pptx/smartart-rightoleftblockdiagram.pptx",
        contains: [pt!(0, "a"), pt!(0, "b"), pt!(0, "c"), pt!(0, "d"), pt!(0, "e")],
    ),
    case!(
        vertical_bracket_list,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testVerticalBracketList",
        file: "sd/qa/unit/data/pptx/vertical-bracket-list.pptx",
        ordered: [ordered!(0, ["1", "A"])],
    ),
    case!(
        table_list,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTableList",
        file: "sd/qa/unit/data/pptx/table-list.pptx",
        ordered: [ordered!(0, ["Parent", "Child 1", "Child 2"])],
    ),
    case!(
        smartart_accent_process,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtAccentProcess",
        file: "sd/qa/unit/data/pptx/smartart-accent-process.pptx",
        ordered: [ordered!(0, ["a", "b", "c", "c", "d"])],
        stroked_path_minimums: [page_count!(0, 3)],
    ),
    case!(
        smartart_continuous_block_process,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtContinuousBlockProcess",
        file: "sd/qa/unit/data/pptx/smartart-continuous-block-process.pptx",
        ordered: [ordered!(0, ["A", "B", "C"])],
        stroked_path_minimums: [page_count!(0, 3)],
    ),
    case!(
        smartart_org_chart,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtOrgChart",
        file: "sd/qa/unit/data/pptx/smartart-org-chart.pptx",
        ordered: [ordered!(0, ["Manager", "Assistant"])],
    ),
    case!(
        smartart_picture_strip,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtPictureStrip",
        file: "sd/qa/unit/data/pptx/smartart-picture-strip.pptx",
        contains: [pt!(0, "Foo Bar"), pt!(0, "Baz Blah"), pt!(0, "A"), pt!(0, "B"), pt!(0, "C")],
        image_minimums: [page_count!(0, 3)],
    ),
    case!(
        smartart_background,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtBackground",
        file: "sd/qa/unit/data/pptx/smartart-background.pptx",
        ordered: [ordered!(0, ["Background", "should", "be", "green"])],
    ),
    case!(
        smartart_center_cycle,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtCenterCycle",
        file: "sd/qa/unit/data/pptx/smartart-center-cycle.pptx",
        contains: [pt!(0, "center"), pt!(0, "a"), pt!(0, "b"), pt!(0, "c")],
    ),
    case!(
        smartart_vertical_block_list,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtVerticalBlockList",
        file: "sd/qa/unit/data/pptx/smartart-vertical-block-list.pptx",
        ordered: [ordered!(0, ["a", "b", "c", "x", "y", "z", "empty"])],
    ),
    case!(
        smartart_missing_bullet,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtMissingBullet",
        file: "sd/qa/unit/data/pptx/smartart-missing-bullet.pptx",
        ordered: [ordered!(0, ["Bullet no", "Bullet yes"])],
    ),
    case!(
        smartart_bullet_list,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtBulletList",
        file: "sd/qa/unit/data/pptx/smartart-bullet-list.pptx",
        ordered: [ordered!(0, ["A", "B", "C"])],
    ),
    case!(
        smartart_recursion,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtRecursion",
        file: "sd/qa/unit/data/pptx/smartart-recursion.pptx",
        ordered: [ordered!(0, ["A", "B1", "C1", "C2", "B2", "C3"])],
    ),
    case!(
        smartart_data_follow,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtDataFollow",
        file: "sd/qa/unit/data/pptx/smartart-data-follow.pptx",
        ordered: [ordered!(0, ["A1", "B1", "B2", "A2", "C1", "C2"])],
    ),
    case!(
        fill_color_list,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testFillColorList",
        file: "sd/qa/unit/data/pptx/fill-color-list.pptx",
        ordered: [ordered!(0, ["A", "B", "C"])],
    ),
    case!(
        smartart_linear_rule,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtLinearRule",
        file: "sd/qa/unit/data/pptx/smartart-linear-rule.pptx",
        ordered: [ordered!(0, ["A", "B", "C"])],
    ),
    case!(
        smartart_linear_rule_vert,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtLinearRuleVert",
        file: "sd/qa/unit/data/pptx/smartart-linear-rule-vert.pptx",
        ordered: [ordered!(0, ["P1", "P2", "P3"])],
    ),
    case!(
        smartart_autofit_sync,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtAutofitSync",
        file: "sd/qa/unit/data/pptx/smartart-autofit-sync.pptx",
        contains: [pt!(0, "A"), pt!(0, "B"), pt!(0, "C"), pt!(0, "A1"), pt!(0, "A2"), pt!(0, "B1"), pt!(0, "B2"), pt!(0, "C1")],
    ),
    case!(
        smartart_snake_rows,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtSnakeRows",
        file: "sd/qa/unit/data/pptx/smartart-snake-rows.pptx",
        ordered: [ordered!(
            0,
            [
                "Parent 3", "Child 3", "Child 2", "Child 5", "Child 6", "Child 1"
            ]
        )],
    ),
    case!(
        smartart_composite_infer_right,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtCompositeInferRight",
        file: "sd/qa/unit/data/pptx/smartart-composite-infer-right.pptx",
        ordered: [ordered!(0, ["Parent", "Child 1", "Child 2"])],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf149551_smartart_pie,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf149551SmartArtPie",
        file: "sd/qa/unit/data/pptx/tdf149551_SmartArt_Pie.pptx",
        ordered: [ordered!(0, ["1 a b c", "2 a b c", "3 a b c"])],
    ),
    case!(
        tdf149551_smartart_pyramid,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf149551SmartArtPyramid",
        file: "sd/qa/unit/data/pptx/tdf149551_SmartArt_Pyramid.pptx",
        ordered: [ordered!(0, ["1 a b c", "2 a b c", "3 a b c"])],
    ),
    case!(
        tdf149551_smartart_venn,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf149551SmartArtVenn",
        file: "sd/qa/unit/data/pptx/tdf149551_SmartArt_Venn.pptx",
        ordered: [ordered!(0, ["1 a b c", "2 a b c", "3 a b c"])],
    ),
    case!(
        tdf149551_smartart_gear,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf149551SmartArtGear",
        file: "sd/qa/unit/data/pptx/tdf149551_SmartArt_Gear.pptx",
        ordered: [ordered!(0, ["One", "Two", "Three"])],
    ),
    case!(
        tdf145528_smartart_matrix,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testTdf145528Matrix",
        file: "sd/qa/unit/data/pptx/tdf145528_SmartArt_Matrix.pptx",
        ordered: [ordered!(0, ["Writer", "Calc", "Impress", "Draw"])],
    ),
    case!(
        tdf157529,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf157529",
        file: "sd/qa/unit/data/pptx/tdf157529.pptx",
        ordered: [ordered!(0, ["LIBREOFFICE", "Text with 100% transparency"])],
    ),
    case!(
        tdf160490,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf160490",
        file: "sd/qa/unit/data/pptx/tdf160490.pptx",
        ordered: [
            ordered!(0, ["HELLO", "Set Top, Bottom margin"]),
            ordered!(1, ["HELLO", "Not set Top, Bottom margin"]),
        ],
    ),
    case!(
        tdf165321,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf165321",
        file: "sd/qa/unit/data/pptx/tdf165321.pptx",
        contains: [pt!(0, "Gestion"), pt!(0, "changement"), pt!(0, "succès")],
    ),
    case!(
        tdf165341,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf165341",
        file: "sd/qa/unit/data/pptx/tdf165341.pptx",
        ordered: [ordered!(0, ["The shape is top", "center"])],
    ),
    case!(
        tdf152186,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf152186",
        file: "sd/qa/unit/data/pptx/tdf152186.pptx",
        stroked_path_minimums: [page_count!(0, 3)],
    ),
    case!(
        tdf93868,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf93868",
        file: "sd/qa/unit/data/pptx/tdf93868.pptx",
        ordered: [ordered!(0, ["Test", "Slide inherits objects from slideMaster"])],
        stroked_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf103473,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf103473",
        file: "sd/qa/unit/data/pptx/tdf103473.pptx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf100065,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf100065",
        file: "sd/qa/unit/data/pptx/tdf100065.pptx",
        ordered: [ordered!(0, ["This is a test"])],
        stroked_path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf90626,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf90626",
        file: "sd/qa/unit/data/pptx/tdf90626.pptx",
        ordered: [ordered!(0, ["Test"])],
        image_minimums: [page_count!(0, 4)],
    ),
    case!(
        tdf138148,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf138148",
        file: "sd/qa/unit/data/pptx/tdf138148.pptx",
        ordered: [ordered!(0, ["Aaa", "Bbb"])],
        image_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf114913,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf114913",
        file: "sd/qa/unit/data/pptx/tdf114913.pptx",
        ordered: [ordered!(0, ["Test"])],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf157216,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf157216",
        file: "sd/qa/unit/data/pptx/tdf157216.pptx",
        ordered: [ordered!(0, ["Flowchart"])],
        stroked_path_minimums: [page_count!(0, 3)],
    ),
    case!(
        tdf154363,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf154363",
        file: "sd/qa/unit/data/pptx/tdf154363.pptx",
        ordered: [ordered!(0, ["Flip horizontal", "Flip vertical"])],
        stroked_path_minimums: [page_count!(0, 2)],
    ),
    case!(
        connectors,
        source: "../core/sd/qa/unit/import-tests.cxx:testConnectors",
        file: "sd/qa/unit/data/pptx/connectors.pptx",
        stroked_path_minimums: [page_count!(0, 16)],
    ),
    case!(
        tdf153036,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf153036",
        file: "sd/qa/unit/data/pptx/tdf153036_resizedConnectorL.pptx",
        ordered: [ordered!(0, ["TextBox"])],
        stroked_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf146223,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf146223",
        file: "sd/qa/unit/data/pptx/tdf146223.pptx",
        ordered: [ordered!(0, ["Title", "Bullet Point 1", "Bullet Point 2"])],
    ),
    case!(
        tdf148965,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf148965",
        file: "sd/qa/unit/data/pptx/tdf148965.pptx",
        pages: 3,
        ordered: [ordered!(1, ["First", "Third"])],
    ),
    case!(
        hyperlink_on_image,
        source: "../core/sd/qa/unit/import-tests.cxx:testHyperlinkOnImage",
        file: "sd/qa/unit/data/pptx/hyperlinkOnImage.pptx",
        pages: 2,
        ordered: [ordered!(0, ["First slide"]), ordered!(1, ["Last slide"])],
        image_minimums: [page_count!(0, 1), page_count!(1, 1)],
    ),
    case!(
        tdf141704,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf141704",
        file: "sd/qa/unit/data/pptx/tdf141704.pptx",
        pages: 7,
        ordered: [
            ordered!(0, ["Go to the last slide"]),
            ordered!(3, ["http://www.example.com"]),
            ordered!(5, ["End Show"]),
        ],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf65724,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf65724",
        file: "sd/qa/unit/data/pptx/tdf65724.pptx",
        pages: 2,
        ordered: [ordered!(0, ["Slide1", "goToSlide2"]), ordered!(1, ["Slide2"])],
    ),
    case!(
        multicol,
        source: "../core/sd/qa/unit/import-tests.cxx:testMultiCol",
        file: "sd/qa/unit/data/pptx/multicol.pptx",
        ordered: [ordered!(0, ["slideshape1", "Slideshape2"])],
    ),
    case!(
        smartart_auto_tx_rot,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtAutoTxRot",
        file: "sd/qa/unit/data/pptx/smartart-autoTxRot.pptx",
        pages: 3,
        ordered: [
            ordered!(0, ["a", "b", "c", "d", "e", "f"]),
            ordered!(1, ["a", "b", "c", "d", "e", "f"]),
            ordered!(2, ["a", "b", "c", "d", "e", "f"]),
        ],
        stroked_path_minimums: [page_count!(0, 16)],
    ),
    case!(
        n759180,
        source: "../core/sd/qa/unit/import-tests.cxx:testN759180",
        file: "sd/qa/unit/data/n759180.pptx",
        ordered: [ordered!(0, ["textrun1", "Textrun2", "Textrun3"])],
    ),
    case!(
        n862510_2,
        source: "../core/sd/qa/unit/import-tests.cxx:testN862510_2",
        file: "sd/qa/unit/data/pptx/n862510_2.pptx",
        pages: 1,
        stroked_path_minimums: [page_count!(0, 1)],
        filled_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf157285,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf157285",
        file: "sd/qa/unit/data/pptx/tdf157285.pptx",
        ordered: [ordered!(0, ["Hello"])],
    ),
    case!(
        shape_glow_effect,
        source: "../core/sd/qa/unit/import-tests.cxx:testShapeGlowEffect",
        file: "sd/qa/unit/data/pptx/shape-glow-effect.pptx",
        pages: 1,
        filled_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        shape_text_glow_effect,
        source: "../core/sd/qa/unit/import-tests.cxx:testShapeTextGlowEffect",
        file: "sd/qa/unit/data/pptx/shape-text-glow-effect.pptx",
        ordered: [ordered!(0, ["Text Glow in Shape"])],
        filled_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf144616,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf144616",
        file: "sd/qa/unit/data/pptx/tdf144616.pptx",
        pages: 2,
        stroked_path_minimums: [page_count!(0, 7)],
    ),
    case!(
        tdf149961_autofit_indentation,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf149961",
        file: "sd/qa/unit/data/pptx/tdf149961-autofitIndentation.pptx",
        ordered: [ordered!(0, ["Autofit", "Autofit"])],
    ),
    case!(
        tdf169524,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf169524",
        file: "sd/qa/unit/data/pptx/tdf169524.pptx",
        ordered: [ordered!(0, ["A", "B"])],
    ),
    case!(
        smartart_multidirectional,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtMultidirectional",
        file: "sd/qa/unit/data/pptx/smartart-multidirectional.pptx",
        contains: [pt!(0, "a"), pt!(0, "b"), pt!(0, "c")],
        stroked_path_minimums: [page_count!(0, 3)],
    ),
    case!(
        smartart_vertical_box_list,
        source: "../core/sd/qa/unit/import-tests-smartart.cxx:testSmartArtVerticalBoxList",
        file: "sd/qa/unit/data/pptx/smartart-vertical-box-list.pptx",
        ordered: [ordered!(0, ["x"])],
        filled_path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf156856,
        source: "../core/sd/qa/unit/import-tests4.cxx:testTdf156856",
        file: "sd/qa/unit/data/pptx/tdf156856.pptx",
        pages: 1,
    ),
    case!(
        n819614,
        source: "../core/sd/qa/unit/import-tests.cxx:testN819614",
        file: "sd/qa/unit/data/n819614.pptx",
        ordered: [ordered!(0, ["Test"])],
    ),
    case!(
        n902652,
        source: "../core/sd/qa/unit/import-tests.cxx:testN902652",
        file: "sd/qa/unit/data/n902652.pptx",
        ordered: [ordered!(0, ["LibreOffice"])],
    ),
    case!(
        bnc870237,
        source: "../core/sd/qa/unit/import-tests.cxx:testBnc870237",
        file: "sd/qa/unit/data/pptx/bnc870237.pptx",
        ordered: [ordered!(0, ["Text"])],
        filled_path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf150789,
        source: "../core/sd/qa/unit/import-tests3.cxx:testTdf150789",
        file: "sd/qa/unit/data/pptx/tdf150789.pptx",
        ordered: [ordered!(0, ["Right", "Left", "Sunshine"])],
    ),
    case!(
        bnc584721_1_2,
        source: "../core/sd/qa/unit/import-tests3.cxx:testBnc584721_1 and testBnc584721_2",
        file: "sd/qa/unit/data/pptx/bnc584721_1_2.pptx",
        ordered: [ordered!(0, ["Title"])],
    ),
    case!(
        master_slides,
        source: "../core/sd/qa/unit/import-tests.cxx:testMasterSlides",
        file: "sd/qa/unit/data/pptx/master-slides.pptx",
        pages: 1,
        ordered: [ordered!(
            0,
            [
                "This is a dark theme cover page",
                "Best for presentations given in a large, dark room"
            ]
        )],
    ),
    case!(
        tdf142645,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf142645",
        file: "sd/qa/unit/data/pptx/tdf142645.pptx",
        pages: 1,
        ordered: [ordered!(0, ["Hello"])],
    ),
    case!(
        tdf142913,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf142913",
        file: "sd/qa/unit/data/pptx/tdf142913.pptx",
        pages: 3,
        ordered: [
            ordered!(0, ["First"]),
            ordered!(1, ["Second"]),
            ordered!(2, ["Third"]),
        ],
    ),
    case!(
        tdf142590,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf142590",
        file: "sd/qa/unit/data/pptx/tdf142590.pptx",
        pages: 3,
        ordered: [ordered!(0, ["1"])],
    ),
    case!(
        tdf131390,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf131390",
        file: "sd/qa/unit/data/pptx/tdf131390.pptx",
        pages: 3,
        ordered: [ordered!(0, ["First"])],
    ),
    case!(
        altdescription,
        source: "../core/sd/qa/unit/import-tests.cxx:testAltDescription",
        file: "sd/qa/unit/data/pptx/altdescription.pptx",
        pages: 1,
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        ooxtheme,
        source: "../core/sd/qa/unit/import-tests4.cxx:testOOXTheme",
        file: "sd/qa/unit/data/pptx/ooxtheme.pptx",
        ordered: [ordered!(0, ["Theme"])],
    ),
    case!(
        tdf103347,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf103347",
        file: "sd/qa/unit/data/pptx/tdf103347.pptx",
        pages: 3,
        ordered: [ordered!(0, ["Hello"])],
    ),
    case!(
        text_distances_insets1,
        source: "../core/sd/qa/unit/ShapeImportExportTest.cxx:testTextDistancesOOXML",
        file: "sd/qa/unit/data/TextDistancesInsets1.pptx",
        pages: 1,
        occurrences_at_least: [
            count!(0, "TOP", 6),
            count!(0, "MIDDLE", 6),
            count!(0, "BOTTOM", 6),
        ],
    ),
    case!(
        boldonse_font_embedded,
        source: "../core/sd/qa/unit/import-tests.cxx:testEmbeddedFont",
        file: "sd/qa/unit/data/BoldonseFontEmbedded.pptx",
        ordered: [ordered!(0, ["Test"])],
    ),
    case!(
        tdf89064,
        source: "../core/sd/qa/unit/import-tests2.cxx:testTdf89064",
        file: "sd/qa/unit/data/pptx/tdf89064.pptx",
        pages: 1,
    ),
    case!(
        tdf115394,
        source: "../core/sd/qa/unit/import-tests.cxx:testTdf115394",
        file: "sd/qa/unit/data/pptx/tdf115394.pptx",
        pages: 5,
        ordered: [
            ordered!(0, ["Standard transition", "slow"]),
            ordered!(1, ["Standard transition", "medium"]),
            ordered!(2, ["Standard transition", "fast"]),
            ordered!(3, ["Custom transition", "0.25 s"]),
            ordered!(4, ["Custom transition", "4.25"]),
        ],
    ),
];

#[test]
fn mapped_pptx_layout_matches_libreoffice_layout_coverage() {
    let mut failures = Vec::new();
    for case in CASES {
        if let Err(error) = std::panic::catch_unwind(|| run_case(case)) {
            let message = if let Some(message) = error.downcast_ref::<String>() {
                message.clone()
            } else if let Some(message) = error.downcast_ref::<&str>() {
                (*message).to_string()
            } else {
                "unknown panic".to_string()
            };
            failures.push(format!(
                "{}\n  source: {}\n  file: {}\n  failure: {}",
                case.name, case.source, case.file, message
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mapped PPTX layout cases failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn run_case(case: &PptxCase) {
    let document = pptx_layout(case.file).unwrap_or_else(|error| {
        panic!(
            "{}: failed to build layout for {}; source={}: {error}",
            case.name, case.file, case.source
        )
    });
    if let Some(page_count) = case.page_count {
        assert_eq!(
            document.pages.len(),
            page_count,
            "{} page count mismatch; source={}; file={}",
            case.name,
            case.source,
            case.file
        );
    }
    for expected in case.ordered {
        assert_page_contains_in_order(&document, expected.page, expected.texts);
    }
    for expected in case.contains {
        assert_page_contains(&document, expected.page, expected.text);
    }
    for unexpected in case.absent {
        assert_page_not_contains(&document, unexpected.page, unexpected.text);
    }
    for expected in case.occurrences_at_least {
        assert_page_text_occurrences_at_least(
            &document,
            expected.page,
            expected.text,
            expected.count,
        );
    }
    for expected in case.image_minimums {
        assert_page_image_count_at_least(&document, expected.page, expected.count);
    }
    for expected in case.stroked_path_minimums {
        assert_page_stroked_path_count_at_least(&document, expected.page, expected.count);
    }
    for expected in case.filled_path_minimums {
        assert_page_filled_path_count_at_least(&document, expected.page, expected.count);
    }
}

#[test]
fn pptx_embedded_font_typeface_is_imported_into_layout_summary() {
    // Source: ../core/sd/qa/unit/FontEmbeddingTest.cxx::testRoundtripEmbeddedFontsPPTX.
    let mut package = PresentationDocument::new_from_file(corpus_file(
        "sd/qa/unit/data/BoldonseFontEmbedded.pptx",
    ))
    .expect("Boldonse embedded-font fixture should open");
    let summary =
        ooxmlsdk_layout::pptx::inspect_layout(&mut package).expect("PPTX layout should inspect");

    assert!(summary.embed_true_type_fonts);
    assert_eq!(summary.embedded_font_typefaces, vec!["Boldonse"]);
}

use ooxmlsdk_layout_test::{
    assert_image_below_table_top_and_flush_right, assert_page_contains,
    assert_page_contains_in_order, assert_page_has_no_text, assert_page_image_count,
    assert_page_image_count_at_least, assert_page_not_contains, assert_page_path_count,
    assert_page_path_count_at_least, assert_page_starts_with, assert_page_text_occurrences,
    assert_path_width_count, assert_text_strikethrough, assert_text_underline, docx_layout,
    docx_layout_named, run_cases_parallel,
};

#[derive(Clone, Copy)]
struct DocxCase {
    name: &'static str,
    source: &'static str,
    file: &'static str,
    page_count: Option<usize>,
    contains: &'static [PageText],
    starts_with: &'static [PageText],
    ordered: &'static [PageTexts],
    absent: &'static [PageText],
    empty_pages: &'static [usize],
    occurrences: &'static [PageTextCount],
    image_counts: &'static [PageCount],
    image_minimums: &'static [PageCount],
    path_counts: &'static [PageCount],
    path_minimums: &'static [PageCount],
    path_width_counts: &'static [PathWidthCount],
    underlined_text: &'static [&'static str],
    strikethrough_text: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct PageText {
    page: usize,
    text: &'static str,
}

#[derive(Clone, Copy)]
struct PageTexts {
    page: usize,
    texts: &'static [&'static str],
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

#[derive(Clone, Copy)]
struct PathWidthCount {
    width: f32,
    count: usize,
}

macro_rules! pt {
    ($page:expr, $text:expr) => {
        PageText {
            page: $page,
            text: $text,
        }
    };
}

macro_rules! ordered {
    ($page:expr, [$($text:expr),+ $(,)?]) => {
        PageTexts {
            page: $page,
            texts: &[$($text),+],
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

macro_rules! width_count {
    ($width:expr, $count:expr) => {
        PathWidthCount {
            width: $width,
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
        $(, contains: [$($contains:expr),* $(,)?])?
        $(, starts_with: [$($starts_with:expr),* $(,)?])?
        $(, ordered: [$($ordered:expr),* $(,)?])?
        $(, absent: [$($absent:expr),* $(,)?])?
        $(, empty_pages: [$($empty_pages:expr),* $(,)?])?
        $(, occurrences: [$($occurrences:expr),* $(,)?])?
        $(, image_counts: [$($image_counts:expr),* $(,)?])?
        $(, image_minimums: [$($image_minimums:expr),* $(,)?])?
        $(, path_counts: [$($path_counts:expr),* $(,)?])?
        $(, path_minimums: [$($path_minimums:expr),* $(,)?])?
        $(, path_width_counts: [$($path_width_counts:expr),* $(,)?])?
        $(, underlined_text: [$($underlined_text:expr),* $(,)?])?
        $(, strikethrough_text: [$($strikethrough_text:expr),* $(,)?])?
        $(,)?
    ) => {
        DocxCase {
            name: stringify!($name),
            source: $source,
            file: $file,
            page_count: $page_count,
            contains: &[$($($contains),*)?],
            starts_with: &[$($($starts_with),*)?],
            ordered: &[$($($ordered),*)?],
            absent: &[$($($absent),*)?],
            empty_pages: &[$($($empty_pages),*)?],
            occurrences: &[$($($occurrences),*)?],
            image_counts: &[$($($image_counts),*)?],
            image_minimums: &[$($($image_minimums),*)?],
            path_counts: &[$($($path_counts),*)?],
            path_minimums: &[$($($path_minimums),*)?],
            path_width_counts: &[$($($path_width_counts),*)?],
            underlined_text: &[$($($underlined_text),*)?],
            strikethrough_text: &[$($($strikethrough_text),*)?],
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

const CASES: &[DocxCase] = &[
    case!(
        fdo66145_headers,
        source: "../core/sw/qa/core/header_footer/HeaderFooterTest.cxx:testFirstPageHeadersAndEmptyFooters",
        file: "fdo66145.docx",
        contains: [
            pt!(0, "This is the FIRST page header."),
            pt!(1, "This is the header for the REST OF THE FILE."),
            pt!(2, "This is the header for the REST OF THE FILE.")
        ],
    ),
    case!(
        first_header_footer,
        source: "../core/sw/qa/core/header_footer/HeaderFooterTest.cxx:testFirstHeaderFooterImport",
        file: "first-header-footer.docx",
        pages: 6,
    ),
    case!(
        tdf166205_first_page_header_footer,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:tdf166205_first_page_header_footer_visible",
        file: "tdf166205_first_page_header_footer_visible.docx",
        pages: 2,
        contains: [
            pt!(0, "HEADER TOP #1"),
            pt!(0, "HEADER BOTTOM #1"),
            pt!(0, "THIS IS FOOTER #1")
        ],
    ),
    case!(
        title_page,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTitlePage",
        file: "testTitlePage.docx",
        contains: [pt!(1, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")],
    ),
    case!(
        n750255,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:n750255",
        file: "n750255.docx",
        contains: [pt!(1, "one"), pt!(2, "two")],
    ),
    case!(
        n780843,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:n780843",
        file: "n780843.docx",
        pages: 2,
        contains: [pt!(1, "shown footer")],
    ),
    case!(
        tdf155736_page_numbers_footer,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf155736_PageNumbers_footer",
        file: "tdf155736_PageNumbers_footer.docx",
        pages: 2,
        contains: [pt!(0, "Page 1 of 2"), pt!(1, "Page 2 of 2")],
    ),
    case!(
        num_override_lvltext,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:num-override-lvltext",
        file: "num-override-lvltext.docx",
        contains: [pt!(0, "1.1")],
    ),
    case!(
        tdf147646_merged_cell_numbering,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf147646_mergedCellNumbering",
        file: "tdf147646_mergedCellNumbering.docx",
        contains: [pt!(0, "2.")],
    ),
    case!(
        tdf153613_anchored_after_page_break,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf153613_anchoredAfterPgBreak",
        file: "tdf153613_anchoredAfterPgBreak.docx",
        image_counts: [page_count!(0, 1)],
    ),
    case!(
        tdf153613_anchored_after_page_break2,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf153613_anchoredAfterPgBreak2",
        file: "tdf153613_anchoredAfterPgBreak2.docx",
        image_counts: [page_count!(1, 1)],
    ),
    case!(
        tdf153613_anchored_after_page_break6,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf153613_anchoredAfterPgBreak6",
        file: "tdf153613_anchoredAfterPgBreak6.docx",
        pages: 2,
        contains: [pt!(1, "y")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf153613_inline_after_page_break2,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf153613_inlineAfterPgBreak2",
        file: "tdf153613_inlineAfterPgBreak2.docx",
        pages: 2,
        contains: [pt!(0, "x")],
        image_counts: [page_count!(1, 1)],
    ),
    case!(
        tdf153613_textbox_after_page_break3,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf153613_textboxAfterPgBreak3",
        file: "tdf153613_textboxAfterPgBreak3.docx",
        pages: 2,
        contains: [pt!(1, "Page 2 right"), pt!(1, "Page 2 middle")],
    ),
    case!(
        tdf147724,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf147724",
        file: "tdf147724.docx",
        contains: [pt!(0, "Placeholder -> *ABC*")],
    ),
    case!(
        n751077,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:n751077",
        file: "n751077.docx",
        contains: [pt!(0, "TEXT1")],
    ),
    case!(
        tdf123636_newline_page_break3,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf123636_newlinePageBreak3",
        file: "tdf123636_newlinePageBreak3.docx",
        pages: 2,
        contains: [pt!(0, "Last line on page 1")],
    ),
    case!(
        tdf123636_newline_page_break4,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf123636_newlinePageBreak4",
        file: "tdf123636_newlinePageBreak4.docx",
        pages: 2,
    ),
    case!(
        tdf169802_hidden_shape,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf169802_hidden_shape",
        file: "tdf169802_hidden_shape.docx",
        image_counts: [page_count!(0, 0)],
    ),
    case!(
        tdf75573_page1frame,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf75573_page1frame",
        file: "tdf75573_page1frame.docx",
        contains: [pt!(0, "lorem ipsum")],
    ),
    case!(
        tdf95495,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf95495",
        file: "sw/qa/extras/ooxmlexport/data/tdf95495.docx",
        contains: [pt!(0, "A.2.1"), pt!(0, ".DESCRIPTION")],
    ),
    case!(
        tdf117923,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf127606",
        file: "sw/qa/extras/layout/data/tdf117923.docx",
        contains: [pt!(0, "GHI GHI GHI GHI"), pt!(0, "2.")],
    ),
    case!(
        tdf104492,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf104492",
        file: "tdf104492.docx",
        pages: 3,
    ),
    case!(
        floattable_multi_nested,
        source: "../core/sw/qa/extras/uiwriter/uiwriter.cxx:testFloatingTableMultiNested",
        file: "floattable-multi-nested.docx",
        pages: 2,
    ),
    case!(
        tdf102466,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf102466",
        file: "tdf102466.docx",
        pages: 11,
    ),
    case!(
        tdf95367_inherit_follow_style,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf95367_inheritFollowStyle",
        file: "tdf95367_inheritFollowStyle.docx",
        contains: [pt!(1, "header")],
    ),
    case!(
        tdf95377,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf95377",
        file: "tdf95377.docx",
        contains: [pt!(0, "a.")],
    ),
    case!(
        tdf134063,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf134063",
        file: "tdf134063.docx",
        pages: 2,
    ),
    case!(
        tdf163894_hidden,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf163894_hidden",
        file: "tdf163894_hidden.docx",
        contains: [
            pt!(0, "handbooks"),
            pt!(0, "infuriating"),
            pt!(1, "infuriating"),
            pt!(3, "mitosis"),
            pt!(3, "modicum")
        ],
    ),
    case!(
        tdf135595_hf_table_wrap,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf135595_HFtableWrap_c12",
        file: "tdf135595_HFtableWrap_c12.docx",
        contains: [
            pt!(0, "Table anchored flies"),
            pt!(0, "don’t loose their wrapping powers.")
        ],
    ),
    case!(
        tdf133000_num_style_formatting,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf133000_numStyleFormatting",
        file: "tdf133000_numStyleFormatting.docx",
        contains: [pt!(0, "First line"), pt!(0, "One sublevel")],
    ),
    case!(
        tdf78352,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf78352",
        file: "tdf78352.docx",
        pages: 1,
    ),
    case!(
        tdf83309,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf83309",
        file: "tdf83309.docx",
        pages: 2,
    ),
    case!(
        tdf131801,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf131801",
        file: "tdf131801.docx",
        pages: 1,
    ),
    case!(
        tdf135949_anchored_before_break,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf135949_anchoredBeforeBreak",
        file: "tdf135949_anchoredBeforeBreak.docx",
        image_counts: [page_count!(0, 1)],
    ),
    case!(
        list_with_lgl,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testListWithLgl",
        file: "listWithLgl.docx",
        contains: [
            pt!(0, "CH I"),
            pt!(0, "Sect 1.01"),
            pt!(0, "CH II"),
            pt!(0, "Sect 2.01")
        ],
    ),
    case!(
        tdf160077_layout_in_cell,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf160077_layoutInCell",
        file: "tdf160077_layoutInCell.docx",
        contains: [pt!(0, "Some text")],
    ),
    case!(
        tdf165047_consolidated_top_margin,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf165047_consolidatedTopMargin",
        file: "tdf165047_consolidatedTopMargin.docx",
        pages: 2,
        contains: [pt!(1, "Consolidate the space between paragraphs")],
    ),
    case!(
        tdf165047_contextual_spacing_top_margin,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf165047_contextualSpacingTopMargin",
        file: "tdf165047_contextualSpacingTopMargin.docx",
        pages: 2,
        contains: [pt!(1, "Don’t add space between paragraphs")],
    ),
    case!(
        tdf138020_all_rows_tbl_header,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf138020_all_rows_tblHeader",
        file: "tdf138020_all_rows_tblHeader.docx",
        pages: 3,
        contains: [pt!(0, "Some text"), pt!(1, "No Header"), pt!(2, "Page.")],
    ),
    case!(
        ignore_top_margin,
        source: "../core/sw/qa/extras/layout/layout.cxx:testIgnoreTopMargin",
        file: "ignore-top-margin.docx",
        pages: 2,
        contains: [pt!(1, "Page 2")],
    ),
    case!(
        ignore_top_margin_table,
        source: "../core/sw/qa/extras/layout/layout.cxx:testIgnoreTopMarginTable",
        file: "ignore-top-margin-table.docx",
        pages: 2,
        contains: [pt!(1, "A1"), pt!(1, "B1")],
    ),
    case!(
        ignore_top_margin_page_style_change,
        source: "../core/sw/qa/extras/layout/layout.cxx:testIgnoreTopMarginPageStyleChange",
        file: "ignore-top-margin-page-style-change.docx",
        pages: 3,
        contains: [pt!(1, "after page break"), pt!(2, "after section break")],
    ),
    case!(
        tdf88496,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf88496",
        file: "sw/qa/extras/layout/data/tdf88496.docx",
        pages: 3,
    ),
    case!(
        tdf157596_paragraph_numbering,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf157596_paragraph_numbering",
        file: "tdf157596_paragraph_numbering.docx",
        ordered: [ordered!(0, ["1.", "2.", "3."])],
    ),
    case!(
        tdf122225,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf122225",
        file: "tdf122225.docx",
        occurrences: [count!(0, "Advanced Diploma", 1), count!(0, "Hispanic", 1)],
    ),
    case!(
        tdf75659,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf75659",
        file: "tdf75659.docx",
        ordered: [ordered!(0, ["Series1", "Series2", "Series3"])],
    ),
    case!(
        tdf139336_columns_with_footnote,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf139336_ColumnsWithFootnoteDoNotOccupyEntirePage",
        file: "tdf139336_ColumnsWithFootnoteDoNotOccupyEntirePage.docx",
        pages: 2,
    ),
    case!(
        fld_in_tbl,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testFldInTbl",
        file: "fld-in-tbl.docx",
        pages: 1,
        occurrences: [count!(0, "LOCATION", 1)],
    ),
    case!(
        tdf170381_normal_table,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf170381SplitFloatTableInNormalTable",
        file: "tdf170381-split-float-table-in-normal-table.docx",
        pages: 2,
        contains: [
            pt!(0, "elit ipsum lorem dolor"),
            pt!(1, "adipiscing ipsum elit lorem"),
            pt!(1, "consectetur dolor lorem ipsum")
        ],
    ),
    case!(
        tdf170846_1,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf170846_1",
        file: "tdf170846_1.docx",
        pages: 2,
        contains: [pt!(1, "Some floating table")],
        absent: [pt!(0, "Some floating table")],
    ),
    case!(
        tdf170846_2,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf170846_2",
        file: "tdf170846_2.docx",
        pages: 2,
        contains: [pt!(1, "adipiscing")],
        absent: [pt!(0, "adipiscing")],
    ),
    case!(
        tdf64264,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf64264",
        file: "tdf64264.docx",
        pages: 2,
        ordered: [ordered!(1, ["Repeating Table Header", "Text"])],
        occurrences: [count!(1, "Repeating Table Header", 1)],
    ),
    case!(
        tdf58944_repeating_table_header,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx:testTdf58944RepeatingTableHeader",
        file: "tdf58944-repeating-table-header.docx",
        pages: 2,
        ordered: [ordered!(1, ["Test1", "Test2"])],
    ),
    case!(
        tdf81100,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx:testTdf81100",
        file: "tdf81100.docx",
        pages: 3,
    ),
    case!(
        tdf126533_no_page_bitmap,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf126533_noPageBitmap",
        file: "tdf126533_noPageBitmap.docx",
        image_counts: [page_count!(0, 0)],
    ),
    case!(
        tdf126533_page_bitmap,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTdf126533_pageBitmap",
        file: "tdf126533_pageBitmap.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf107889,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf107889",
        file: "tdf107889.docx",
        contains: [pt!(0, "Before"), pt!(0, "A1"), pt!(1, "A6")],
    ),
    case!(
        tdf119952_negative_margins,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx:testTdf119952_negativeMargins",
        file: "tdf119952_negativeMargins.docx",
        contains: [pt!(0, "f1"), pt!(0, "f8"), pt!(1, "p1"), pt!(2, "aaaa"), pt!(2, "eeee")],
    ),
    case!(
        tdf106606_graphic_bullets,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf106606",
        file: "tdf106606.docx",
        image_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf156902_glow_group,
        source: "../core/oox/qa/unit/shape.cxx:testGlowOnGroup",
        file: "tdf156902_GlowOnGroup.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        floattable_split,
        source: "../core/sw/qa/extras/uiwriter/uiwriter9.cxx:testSplitFloatingTable",
        file: "sw/qa/extras/uiwriter/data/floattable-split.docx",
        pages: 3,
        path_minimums: [page_count!(0, 1), page_count!(1, 1)],
    ),
    case!(
        floattable_anchor_split,
        source: "../core/sw/qa/core/txtnode/txtnode.cxx:testSplitFlyAnchorSplit",
        file: "floattable-anchor-split.docx",
        pages: 2,
        contains: [pt!(0, "First paragraph")],
        path_minimums: [page_count!(0, 1), page_count!(1, 1)],
    ),
    case!(
        continuous_section_break_header_footer,
        source: "../core/sw/qa/core/header_footer/HeaderFooterTest.cxx:testContSectBreakHeaderFooter",
        file: "sw/qa/core/header_footer/data/cont-sect-break-header-footer.docx",
        contains: [
            pt!(0, "First page header, section 1"),
            pt!(0, "First page footer, section 1"),
            pt!(1, "First page header, section 2"),
            pt!(1, "First page footer, section 2"),
            pt!(2, "Header, section 2"),
            pt!(2, "Footer, section 3")
        ],
    ),
    case!(
        inherit_first_header,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport4.cxx:testInheritFirstHeader",
        file: "inheritFirstHeader.docx",
        contains: [
            pt!(0, "First Header"),
            pt!(1, "Follow Header"),
            pt!(2, "Follow Header"),
            pt!(3, "First Header"),
            pt!(4, "Last Header")
        ],
    ),
    case!(
        tdf153613_anchored_after_page_break3,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx:testTdf153613_anchoredAfterPgBreak3",
        file: "tdf153613_anchoredAfterPgBreak3.docx",
        image_counts: [page_count!(1, 1)],
    ),
    case!(
        tdf153613_inline_after_page_break,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx:testTdf153613_inlineAfterPgBreak",
        file: "tdf153613_inlineAfterPgBreak.docx",
        image_counts: [page_count!(1, 1)],
    ),
    case!(
        tdf136952_pg_break3,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf136952_pgBreak3",
        file: "tdf136952_pgBreak3.docx",
        starts_with: [pt!(5, "Lorem ipsum")],
    ),
    case!(
        tdf123636_newline_page_break4_empty_page,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport13.cxx:testTdf123636_newlinePageBreak4",
        file: "tdf123636_newlinePageBreak4.docx",
        pages: 2,
        empty_pages: [1],
    ),
    case!(
        tdf169802_hidden_shape_paths,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport13.cxx:testTdf169802_hidden_shape",
        file: "tdf169802_hidden_shape.docx",
        image_counts: [page_count!(0, 0)],
        path_counts: [page_count!(0, 0)],
    ),
    case!(
        tdf124594_shape_margin,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport13.cxx:testTdf124594",
        file: "tdf124594.docx",
        contains: [pt!(
            0,
            "Er horte leise Schritte hinter sich. Das bedeutete nichts Gutes. Wer wu"
        )],
    ),
    case!(
        tdf149313_section_page_sizes,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport17.cxx:testTdf149313",
        file: "tdf149313.docx",
        pages: 2,
    ),
    case!(
        tdf124600_header_shape_text,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport2.cxx:testTdf124600",
        file: "sw/qa/extras/ooxmlimport/data/tdf124600.docx",
        contains: [pt!(0, "Shape 1 text"), pt!(0, "X")],
    ),
    case!(
        tdf103544_image_in_frame,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport9.cxx:testTdf103544",
        file: "tdf103544.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        styleref_de,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport_de_locale.cxx:testTdf160402",
        file: "StyleRef-DE.docx",
        contains: [
            pt!(0, "Heading 1"),
            pt!(1, "Nunc viverra imperdiet enim. Fusce est. Vivamus a tellus."),
            pt!(2, "Cras faucibus condimentum odio. Sed ac ligula. Aliquam at eros."),
            pt!(3, "Nunc viverra imperdiet enim. Fusce est. Vivamus a tellus."),
            pt!(4, "Aenean nec lorem. In porttitor. Donec laoreet nonummy augue.")
        ],
    ),
    case!(
        tdf163894,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx:testTdf163894",
        file: "tdf163894.docx",
        contains: [
            pt!(0, "handbooks"),
            pt!(0, "infuriating"),
            pt!(1, "infuriating"),
            pt!(2, "initializes"),
            pt!(3, "misrepresenting"),
            pt!(3, "modicum")
        ],
    ),
    case!(
        tdf32363,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx:testTdf32363",
        file: "tdf32363.docx",
        contains: [
            pt!(2, "Do not shorten this short heading"),
            pt!(3, "Beginning of the paragraph"),
            pt!(4, "Beginning of the paragraph + ellipsis…"),
            pt!(5, "Beginning of the paragraph + ellipsis…"),
            pt!(6, "Hidden text with the referred character style"),
            pt!(7, "Hidden text with the referred character style")
        ],
    ),
    case!(
        tdf163894_from_top,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx:testTdf163894_from_top_to_beginning_of_the_documentMarguerite",
        file: "tdf163894_from_top.docx",
        contains: [
            pt!(0, "handbooks"),
            pt!(0, "infuriating"),
            pt!(1, "infuriating"),
            pt!(2, "initializes"),
            pt!(3, "maroon"),
            pt!(3, "modicum")
        ],
    ),
    case!(
        tdf78749_shape_background_image,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx:testTdf78749",
        file: "tdf78749.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        bnc891663,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport19.cxx:testBnc891663",
        file: "bnc891663.docx",
        contains: [pt!(0, "Some text")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf160077_layout_in_cell_b,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf160077_layoutInCellB",
        file: "tdf160077_layoutInCellB.docx",
        contains: [pt!(0, "OBJECTIVE"), pt!(0, "EXPERIENCE")],
    ),
    case!(
        tdf160077_layout_in_cell_c,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf160077_layoutInCellC",
        file: "tdf160077_layoutInCellC.docx",
        contains: [pt!(0, "Top margin"), pt!(0, "-anchor paragrap"), pt!(0, "h-")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf160077_layout_in_cell_d,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf160077_layoutInCellD",
        file: "tdf160077_layoutInCellD.docx",
        contains: [pt!(0, "Below logo"), pt!(0, "Below image")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf153909_follow_text_flow,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf153909_followTextFlow",
        file: "tdf153909_followTextFlow.docx",
        contains: [pt!(0, "Enterprise")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf162541_not_layout_in_cell,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf162541",
        file: "tdf162541_notLayoutInCell_paraLeft.docx",
        contains: [pt!(0, "Cell text")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf162551_layout_in_cell,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf162551",
        file: "tdf162551_notLayoutInCell_charLeft_fromTop.docx",
        contains: [pt!(0, "-anchor point-")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf146346_footnote_tables,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport23.cxx:testTdf146346",
        file: "tdf146346.docx",
        pages: 1,
    ),
    case!(
        tdf165354_hyphenation_flow,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport23.cxx:testTdf165354",
        file: "tdf165354.docx",
        contains: [pt!(0, "except that it has"), pt!(1, "atmosphere. The Earth")],
    ),
    case!(
        tdf166544_no_top_margin_fields,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport25.cxx:testTdf166544_noTopMargin_fields",
        file: "tdf166544_noTopMargin_fields.docx",
        contains: [pt!(1, "Page 2")],
    ),
    case!(
        tdf166510_section_bottom_spacing,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport25.cxx:testTdf166510_sectPr_bottomSpacing",
        file: "tdf166510_sectPr_bottomSpacing.docx",
        contains: [pt!(1, "Page 2")],
    ),
    case!(
        tdf169986_bottom_spacing,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport25.cxx:testTdf169986_bottomSpacing",
        file: "tdf169986_bottomSpacing.docx",
        pages: 1,
    ),
    case!(
        tdf167657_section_bottom_spacing,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport25.cxx:testTdf167657_sectPr_bottomSpacing",
        file: "tdf167657_sectPr_bottomSpacing.docx",
        pages: 1,
    ),
    case!(
        inline_endnote_position,
        source: "../core/sw/qa/core/layout/ftnfrm.cxx:testInlineEndnotePosition",
        file: "inline-endnote-position.docx",
        contains: [pt!(0, "Endnote")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        table_fly_overlap,
        source: "../core/sw/qa/core/layout/layout.cxx:testTableFlyOverlap",
        file: "table-fly-overlap.docx",
        contains: [pt!(0, "Table1:B1")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf128195,
        source: "../core/sw/qa/core/layout/layout.cxx:testTdf128195",
        file: "tdf128195.docx",
        contains: [pt!(0, "Body")],
    ),
    case!(
        border_collapse_compat,
        source: "../core/sw/qa/core/layout/layout.cxx:testBorderCollapseCompat",
        file: "border-collapse-compat.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        table_fly_overlap_spacing,
        source: "../core/sw/qa/core/layout/layout.cxx:testTableFlyOverlapSpacing",
        file: "table-fly-overlap-spacing.docx",
        contains: [pt!(0, "Before table"), pt!(0, "After table."), pt!(0, "These")],
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        textbox_autogrow_vertical,
        source: "../core/sw/qa/core/layout/layout.cxx:testTextBoxAutoGrowVertical",
        file: "textbox-autogrow-vertical.docx",
        contains: [pt!(0, "Shape")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        header_textbox,
        source: "../core/sw/qa/core/layout/layout.cxx:testTextBoxInHeaderIsPositioned",
        file: "header-textbox.docx",
        contains: [pt!(0, "XXXXXXX")],
    ),
    case!(
        vmerge_cell_border,
        source: "../core/sw/qa/core/layout/layout.cxx:testVerticallyMergedCellBorder",
        file: "vmerge-cell-border.docx",
        path_minimums: [page_count!(0, 7)],
    ),
    case!(
        inner_border,
        source: "../core/sw/qa/core/layout/layout.cxx:testInnerCellBorderIntersect",
        file: "inner-border.docx",
        path_minimums: [page_count!(0, 3)],
    ),
    case!(
        double_border_vertical,
        source: "../core/sw/qa/core/layout/layout.cxx:testDoubleBorderVertical",
        file: "double-border-vertical.docx",
        path_minimums: [page_count!(0, 4)],
    ),
    case!(
        double_border_horizontal,
        source: "../core/sw/qa/core/layout/layout.cxx:testDoubleBorderHorizontal",
        file: "double-border-horizontal.docx",
        path_minimums: [page_count!(0, 4)],
    ),
    case!(
        para_border_in_cell_clip,
        source: "../core/sw/qa/core/layout/layout.cxx:testParaBorderInCellClip",
        file: "para-border-in-cell-clip.docx",
        contains: [pt!(0, "A"), pt!(0, "1")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        double_page_border,
        source: "../core/sw/qa/core/layout/layout.cxx:testDoublePageBorder",
        file: "double-page-border.docx",
        path_minimums: [page_count!(0, 4)],
    ),
    case!(
        rtl_table,
        source: "../core/sw/qa/core/layout/paintfrm.cxx:testRTLBorderMerge",
        file: "rtl-table.docx",
        path_minimums: [page_count!(0, 6)],
    ),
    case!(
        endnote_cont_separator,
        source: "../core/sw/qa/core/layout/paintfrm.cxx:testEndnoteContSeparator",
        file: "endnote-cont-separator.docx",
        pages: 2,
        path_minimums: [page_count!(1, 1)],
    ),
    case!(
        table_print_area_left,
        source: "../core/sw/qa/core/layout/tabfrm.cxx:testTablePrintAreaLeft",
        file: "table-print-area-left.docx",
        contains: [pt!(0, "Date & venue")],
    ),
    case!(
        tdf136588,
        source: "../core/sw/qa/extras/layout/layout.cxx:TestTdf136588",
        file: "tdf136588.docx",
        contains: [pt!(
            0,
            "effectively by modern-day small to medium enterprises?"
        )],
    ),
    case!(
        tdf137025,
        source: "../core/sw/qa/extras/layout/layout.cxx:TestTdf137025",
        file: "tdf137025.docx",
        contains: [pt!(0, "xxxx")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf116486,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf116486",
        file: "tdf116486.docx",
        contains: [pt!(0, "Flying Box")],
    ),
    case!(
        fdo43573_2_min,
        source: "../core/sw/qa/extras/layout/layout.cxx:TestTdf142080",
        file: "fdo43573-2-min.docx",
        contains: [pt!(8, "De kleur u (rood) in het rechtervlak")],
        image_minimums: [page_count!(8, 1)],
    ),
    case!(
        tdf128198,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf128198",
        file: "tdf128198-1.docx",
        contains: [
            pt!(0, "From this perspective"),
            pt!(0, "satellite boasts some significant advantages.")
        ],
    ),
    case!(
        tdf106153,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf106153",
        file: "tdf106153.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf157628,
        source: "../core/sw/qa/extras/layout/layout.cxx:testTdf157628",
        file: "tdf157628.docx",
        contains: [pt!(0, "This is in first row"), pt!(0, "This is second row")],
    ),
    case!(
        hidden_para_separator,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf152872",
        file: "hidden-para-separator.docx",
        ordered: [ordered!(0, ["C", "D", "E"])],
    ),
    case!(
        tdf125300,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf125300",
        file: "tdf125300.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        legend_itemorder_min,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf134247",
        file: "legend-itemorder-min.docx",
        contains: [pt!(0, "1. adatsor")],
    ),
    case!(
        long_legendentry,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf126425",
        file: "long_legendentry.docx",
        contains: [pt!(0, "Data series with a long long title")],
    ),
    case!(
        tdf115630,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf115630",
        file: "tdf115630.docx",
        contains: [pt!(0, "1. Column with long name")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf128996,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf128996",
        file: "tdf128996.docx",
        contains: [pt!(0, "A very long category name 1")],
    ),
    case!(
        tdf126244,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf126244",
        file: "tdf126244.docx",
        contains: [pt!(0, "FIRST LEVEL")],
    ),
    case!(
        tdf69648,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf69648",
        file: "tdf69648.docx",
        contains: [pt!(0, "Text in right box"), pt!(0, "Text in left box")],
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf117982,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf117982",
        file: "tdf117982.docx",
        starts_with: [pt!(0, "FOO AAA")],
    ),
    case!(
        tdf128959,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf128959",
        file: "tdf128959.docx",
        contains: [pt!(
            0,
            "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Maecenas porttitor congue"
        )],
    ),
    case!(
        tdf124423,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf124423_DOCX",
        file: "tdf124423.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf138782,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf138782",
        file: "tdf138782.docx",
        contains: [pt!(0, "10")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf135035,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf135035_DOCX",
        file: "tdf135035.docx",
        contains: [pt!(0, "A")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        fdo48718,
        source: "../core/sw/qa/extras/layout/layout4.cxx:TestTdf161348",
        file: "fdo48718-1.docx",
        contains: [pt!(0, "INFORME DE ASISTENCIA"), pt!(0, "ARGUMENTACIÓN JURÍDICA")],
    ),
    case!(
        sdt_framepr,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf159259",
        file: "sdt+framePr.docx",
        pages: 1,
        occurrences: [count!(
            0,
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            1
        )],
    ),
    case!(
        tdf115094,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf115094",
        file: "sw/qa/extras/layout/data/tdf115094.docx",
        contains: [pt!(0, "Zufahrt"), pt!(0, "Rollstuhlfahrer")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf112290,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf112290",
        file: "tdf112290.docx",
        contains: [pt!(0, "Xxxx Xxxx")],
    ),
    case!(
        tdf123651,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf123651",
        file: "tdf123651.docx",
        occurrences: [count!(
            0,
            "Lorem ipsum dolor sit amet, consectetuer adipiscing elit.",
            2
        )],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf64222,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf64222",
        file: "tdf64222.docx",
        contains: [pt!(0, "Another one title of document")],
    ),
    case!(
        tdf170381_float_table,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf170381_split_float_table_in_float_table",
        file: "tdf170381-split-float-table-in-float-table.docx",
        pages: 2,
        contains: [
            pt!(0, "Table1 A1 dolor elit"),
            pt!(
                0,
                "adipiscing dolor adipiscing amet ipsum elit sit elit lorem elit adipiscing dolor ipsum"
            ),
            pt!(0, "Table2 A22 elit"),
            pt!(1, "Table2 A23"),
            pt!(1, "Table2 A31")
        ],
    ),
    case!(
        tdf170620,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf170620_float_table_after_keep_with_next_para",
        file: "tdf170620.docx",
        pages: 2,
        contains: [pt!(0, "Keep-with-next paragraph"), pt!(0, "Something")],
    ),
    case!(
        tdf170630,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf170630",
        file: "tdf170630.docx",
        pages: 2,
        contains: [pt!(0, "Keep-with-next paragraph")],
    ),
    case!(
        xaxis_labelbreak,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf138194",
        file: "xaxis-labelbreak.docx",
        contains: [pt!(0, "really really long data label 1 made even longer")],
    ),
    case!(
        tdf138773,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf138773",
        file: "tdf138773.docx",
        occurrences: [count!(0, "2000-01", 1)],
    ),
    case!(
        tdf130969,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf130969",
        file: "tdf130969.docx",
        contains: [pt!(0, "0.35781")],
    ),
    case!(
        tdf129054,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf129054",
        file: "tdf129054.docx",
        contains: [pt!(0, "Értékesítés")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        test_area_chart_number_format,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf129173",
        file: "testAreaChartNumberFormat.docx",
        contains: [pt!(0, "56")],
    ),
    case!(
        tdf134866,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf134866",
        file: "tdf134866.docx",
        contains: [pt!(0, "100%")],
    ),
    case!(
        tdf137116,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf137116",
        file: "tdf137116.docx",
        contains: [pt!(0, "datalabel2"), pt!(0, "datalabel4")],
    ),
    case!(
        tdf137154,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf137154",
        file: "tdf137154.docx",
        contains: [pt!(0, "long data label 1"), pt!(0, "long data label 4")],
    ),
    case!(
        outside_long_data_label,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf138777",
        file: "outside_long_data_label.docx",
        occurrences: [count!(0, "really", 2)],
    ),
    case!(
        tdf130031,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf130031",
        file: "tdf130031.docx",
        contains: [pt!(0, "23")],
    ),
    case!(
        tdf138018,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf138018",
        file: "tdf138018.docx",
        contains: [pt!(0, "Értékesítés")],
        path_counts: [page_count!(0, 2)],
    ),
    case!(
        tdf130380,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf130380",
        file: "tdf130380.docx",
        contains: [pt!(0, "1. adatsor")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf129095,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf129095",
        file: "tdf129095.docx",
        contains: [pt!(0, "Category 1")],
    ),
    case!(
        tdf132956,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf132956",
        file: "tdf132956.docx",
        contains: [pt!(0, "Category 1")],
    ),
    case!(
        tdf122014,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf122014",
        file: "tdf122014.docx",
        contains: [pt!(0, "Chart title alignment")],
    ),
    case!(
        tdf167202_footnote,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf167202_footnote",
        file: "tdf167202_footnote.docx",
        contains: [pt!(0, "FOOTNOTE #1")],
    ),
    case!(
        tdf134659,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf134659",
        file: "tdf134659.docx",
        contains: [pt!(0, "Test the axis label aligment!")],
    ),
    case!(
        tdf134235,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf134235",
        file: "tdf134235.docx",
        contains: [pt!(0, "When opened in Writer the long chart title")],
    ),
    case!(
        tdf134676,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf134676",
        file: "tdf134676.docx",
        contains: [pt!(0, "default length of the axis title box")],
    ),
    case!(
        tdf134146,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf134146",
        file: "tdf134146.docx",
        occurrences: [count!(0, "Horizontal", 2)],
    ),
    case!(
        tdf136061,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf136061",
        file: "tdf136061.docx",
        contains: [pt!(0, "Customlabel")],
    ),
    case!(
        tdf116925,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf116925",
        file: "tdf116925.docx",
        contains: [pt!(0, "hello")],
    ),
    case!(
        tdf117028,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf117028",
        file: "tdf117028.docx",
        contains: [pt!(0, "Hello")],
        path_counts: [page_count!(0, 0)],
    ),
    case!(
        tdf150200,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf150200_DOCX",
        file: "tdf150200.docx",
        contains: [
            pt!(0, "-(dash)"),
            pt!(0, "–(en-dash)"),
            pt!(0, "—(em-dash)"),
            pt!(0, "‒(figure dash)")
        ],
    ),
    case!(
        tdf150438,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf150438_DOCX",
        file: "tdf150438.docx",
        contains: [
            pt!(0, "“Lorem ipsum"),
            pt!(0, "”Nunc viverra imperdiet enim."),
            pt!(0, "‘Aenean nec lorem."),
            pt!(0, "’Aenean nec lorem.")
        ],
    ),
    case!(
        tdf127118,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf127118",
        file: "tdf127118.docx",
        pages: 2,
        contains: [pt!(1, "2.")],
    ),
    case!(
        tdf141220,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf141220",
        file: "tdf141220.docx",
        contains: [pt!(0, "Lorem ipsum")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf134685,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf134685",
        file: "tdf134685.docx",
        contains: [pt!(0, "fffffffff")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf109077,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf109077",
        file: "tdf109077.docx",
        contains: [pt!(0, "x1")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf164903,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf164903",
        file: "tdf164903.docx",
        contains: [pt!(0, "Definitions")],
    ),
    case!(
        tdf134463,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf134463",
        file: "tdf134463.docx",
        ordered: [ordered!(0, ["A1", "A2", "B1", "B2"])],
    ),
    case!(
        tdf117188,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf117188",
        file: "tdf117188.docx",
        contains: [pt!(0, "Der")],
        path_counts: [page_count!(0, 0)],
    ),
    case!(
        tdf161718,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf161718",
        file: "tdf161718.docx",
        pages: 1,
        contains: [
            pt!(0, "Header Text"),
            pt!(0, "Body text"),
            pt!(0, "Footer Text."),
            pt!(0, "Footnote area")
        ],
    ),
    case!(
        tdf119908,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf119908",
        file: "sw/qa/extras/layout/data/tdf130088.docx",
        contains: [pt!(
            0,
            "viverra odio. Donec auctor molestie sem, sit amet tristique lectus hendrerit sed."
        )],
    ),
    case!(
        tdf158333,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf158333",
        file: "sw/qa/extras/layout/data/tdf130088.docx",
        contains: [
            pt!(
                0,
                "viverra odio. Donec auctor molestie sem, sit amet tristique lectus hendrerit sed."
            ),
            pt!(
                0,
                "laoreet vel leo nec, volutpat facilisis eros. Donec consequat arcu ut diam tempor"
            ),
            pt!(
                0,
                "Donec auctor molestie sem, sit amet tristique lectus hendrerit sed. Cras sodales"
            ),
            pt!(
                0,
                "consequat arcu ut diam tempor luctus. Cum sociis natoque penatibus et magnis"
            ),
            pt!(0, "venenatis, quis commodo dolor posuere. Curabitur dignissim sapien quis")
        ],
    ),
    case!(
        tdf164905,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf164905",
        file: "tdf164905.docx",
        ordered: [ordered!(0, ["INHALT", "VERANTWORTLICHKEIT", "ZIELSETZUNG DER"])],
        occurrences: [count!(0, "VERANTWORTLICHKEIT", 1)],
    ),
    case!(
        tdf163149,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf163149",
        file: "tdf163149.docx",
        contains: [pt!(
            0,
            "vulputate nisl commodo. Proin aliquet turpis ac posuere commodo. Curabitur facilisis mauris ac nulla dapibus"
        )],
    ),
    case!(
        tdf164499,
        source: "../core/sw/qa/extras/layout/layout3.cxx:testTdf164499",
        file: "tdf164499.docx",
        contains: [pt!(0, "2.5.5"), pt!(0, "pH-Messung"), pt!(0, "hat und ich keine Werte habe?)")],
    ),
    case!(
        writer_image_no_capture,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testWriterImageNoCapture",
        file: "writer-image-no-capture.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf152298,
        source: "../core/sw/qa/extras/layout/layout4.cxx:testTdf152298",
        file: "tdf152298.docx",
        pages: 2,
        ordered: [ordered!(1, ["1", "2", "3", "10"])],
    ),
    case!(
        redline_table,
        source: "../core/sw/qa/core/layout/paintfrm.cxx:testTableRedlineRenderMode",
        file: "redline-table.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        redline_default,
        source: "../core/sw/qa/core/text/itrpaint.cxx:testRedlineRenderModeOmitInsertDelete",
        file: "redline.docx",
        ordered: [ordered!(0, ["baseline", "oldcontent", "newcontent"])],
    ),
    case!(
        redline_number_portion,
        source: "../core/sw/qa/core/text/porfld.cxx:testNumberPortionRedlineRenderMode",
        file: "redline-number-portion.docx",
        contains: [pt!(0, "2.")],
        underlined_text: ["2."],
    ),
    case!(
        redline_bullet,
        source: "../core/sw/qa/core/text/porfld.cxx:testTabPortionRedlineRenderMode",
        file: "redline-bullet.docx",
        strikethrough_text: ["o"],
    ),
    case!(
        ct_formatted_deletion,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf165322",
        file: "CT-formatted-deletion.docx",
        contains: [pt!(
            0,
            "Nunc viverra imperdiet enim. Fusce est. Vivamus a tellus."
        )],
        strikethrough_text: ["Nunc viverra imperdiet enim. Fusce est. Vivamus a tellus."],
    ),
    case!(
        tdf104797_move_redline,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testRedlineMovingDOCX",
        file: "sw/qa/extras/layout/data/tdf104797.docx",
        contains: [
            pt!(0, "Will this sentence be duplicated?"),
            pt!(0, "This is a filler sentence."),
            pt!(0, "ADDED STUFF")
        ],
    ),
    case!(
        tdf155229_row_height_at_least,
        source: "../core/sw/qa/extras/layout/layout4.cxx:TestTdf155229RowAtLeast",
        file: "tdf155229_row_height_at_least.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf164907_row_height_at_least,
        source: "../core/sw/qa/extras/layout/layout4.cxx:TestTdf164907_rowHeightAtLeast",
        file: "tdf164907_rowHeightAtLeast.docx",
        pages: 1,
        contains: [pt!(0, "2106/0001")],
    ),
    case!(
        tdf105035_framepr_b,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx:testTdf105035_framePrB",
        file: "tdf105035_framePrB.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf105035_framepr_c,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx:testTdf105035_framePrC",
        file: "tdf105035_framePrC.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf37153,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport24.cxx:testTdf37153",
        file: "tdf37153_considerWrapOnObjPos.docx",
        contains: [pt!(0, "Bottom aligned")],
    ),
    case!(
        tdf150822,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport25.cxx:testTdf150822",
        file: "tdf150822.docx",
        contains: [pt!(0, "AAAA BBBB CCCC")],
    ),
    case!(
        tdf167526,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf167526",
        file: "tdf167526.docx",
        occurrences: [count!(0, "2", 1)],
    ),
    case!(
        tdf167540,
        source: "../core/sw/qa/extras/layout/layout5.cxx:testTdf167540",
        file: "tdf167540.docx",
        ordered: [ordered!(
            0,
            [
                "1",
                "Text",
                "2",
                "First floating table",
                "3",
                "Second floating table",
                "A normal table",
                "4",
                "More text"
            ]
        )],
    ),
    case!(
        tdf130804,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf130804",
        file: "tdf130804.docx",
        contains: [pt!(0, "Lorem ipsum")],
    ),
    case!(
        tdf105143,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf105143",
        file: "tdf105143.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        floating_table_section_columns,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testFloatingTableSectionColumns",
        file: "floating-table-section-columns.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf60351,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf60351",
        file: "tdf60351.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf98882,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf98882",
        file: "tdf98882.docx",
        image_minimums: [page_count!(0, 1)],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf100072,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testTdf100072",
        file: "tdf100072.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf114212,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport2.cxx:testTdf114212",
        file: "tdf114212.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf133070_no_footer,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport15.cxx:testRelativeAnchorHeightFromBottomMarginNoFooter",
        file: "tdf133070_testRelativeAnchorHeightFromBottomMarginNoFooter.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf133670,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport4.cxx:testRelativeAnchorWidthFromRightMargin",
        file: "tdf133670_testRelativeAnchorWidthFromRightMargin.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf165478_bottom_aligned,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport25.cxx:testTdf165478_bottomAligned",
        file: "tdf165478_bottomAligned.docx",
        contains: [pt!(0, "Bottom aligned")],
        image_minimums: [page_count!(0, 1)],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        i120928,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport2.cxx:testI120928",
        file: "i120928.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        dml_shape_fill_bitmap_crop,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport6.cxx:testDMLShapeFillBitmapCrop",
        file: "dml-shape-fillbitmapcrop.docx",
        image_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf112450_vml_polyline,
        source: "../core/oox/qa/unit/vml.cxx:tdf112450_vml_polyline",
        file: "tdf112450_vml_polyline.docx",
        path_minimums: [page_count!(0, 3)],
    ),
    case!(
        tdf153000_wordart_types,
        source: "../core/svx/qa/unit/customshapes.cxx:testTdf153000_MS0_SPT_25_31",
        file: "tdf153000_WordArt_type_25_to_31.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf138465min,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testUnusedOLEprops",
        file: "tdf138465min.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf97618_vml_shape_text_wrap,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport23.cxx:testVmlShapeTextWordWrap",
        file: "tdf97618_testVmlShapeTextWordWrap.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        i124106,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx:testI124106",
        file: "i124106.docx",
        pages: 1,
    ),
    case!(
        large_twips,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx:testLargeTwips",
        file: "large-twips.docx",
        contains: [pt!(0, "text")],
    ),
    case!(
        gridbefore,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx:testGridBefore",
        file: "gridbefore.docx",
        contains: [pt!(0, "A3"), pt!(0, "B2")],
    ),
    case!(
        tdf125324,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport13.cxx:testTdf125324",
        file: "tdf125324.docx",
        contains: [pt!(0, "Position")],
    ),
    case!(
        tdf162746,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf162746",
        file: "tdf162746.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf166850,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport_de_locale.cxx:testTdf166850",
        file: "tdf166850.docx",
        contains: [pt!(1, "Heading 1")],
    ),
    case!(
        toplevel_line_hori_offset,
        source: "../core/oox/qa/unit/drawingml.cxx:testToplevelLineHorOffsetDOCX",
        file: "toplevel-line-hori-offset.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        line_vertical_rotation,
        source: "../core/oox/qa/unit/drawingml.cxx:testDOCXVerticalLineRotation",
        file: "line-vertical-rotation.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        customshape_position,
        source: "../core/oox/qa/unit/shape.cxx:testCustomshapePosition",
        file: "customshape-position.docx",
        image_minimums: [page_count!(0, 1)],
    ),
    case!(
        multiple_group_shapes,
        source: "../core/oox/qa/unit/shape.cxx:testMultipleGroupShapes",
        file: "multiple-group-shapes.docx",
        contains: [pt!(0, "Fly2")],
    ),
    case!(
        inside_outside_vert_align,
        source: "../core/sw/qa/core/objectpositioning/objectpositioning.cxx:testInsideOutsideVertAlignBottomMargin",
        file: "inside-outside-vert-align.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        vml_vertical_alignment,
        source: "../core/sw/qa/core/objectpositioning/objectpositioning.cxx:testVMLVertAlignBottomMargin",
        file: "vml-vertical-alignment.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        fdo38414,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx:testFdo38414",
        file: "fdo38414.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf115180,
        source: "../core/sw/qa/extras/rtfexport/rtfexport3.cxx:testTdf115180",
        file: "tdf115180.docx",
        path_minimums: [page_count!(0, 3)],
    ),
    case!(
        tdf98987,
        source: "../core/sw/qa/extras/uiwriter/uiwriter4.cxx:testTdf98987",
        file: "tdf98987.docx",
        path_minimums: [page_count!(0, 3)],
    ),
    case!(
        tdf99004,
        source: "../core/sw/qa/extras/uiwriter/uiwriter4.cxx:testTdf99004",
        file: "tdf99004.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf135943_shape_text,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx:testTdf135943_shapeWithText_L0c15",
        file: "tdf135943_shapeWithText_LayoutInCell0_compat15.docx",
        contains: [pt!(0, "lk")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf167770_margin_inside_outside,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport22.cxx:testTdf167770_marginInsideOutside",
        file: "tdf167770_marginInsideOutside.docx",
        path_minimums: [page_count!(0, 1), page_count!(1, 1)],
    ),
    case!(
        tdf87348_linked_textboxes,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport7.cxx:testTDF87348",
        file: "tdf87348_linkedTextboxes.docx",
        path_minimums: [page_count!(0, 13)],
    ),
    case!(
        tdf125885_wordart,
        source: "../core/oox/qa/unit/shape.cxx:testWriterFontwork",
        file: "tdf125885_WordArt.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf125885_wordart3,
        source: "../core/oox/qa/unit/shape.cxx:testWriterFontwork3",
        file: "tdf125885_WordArt3.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf152840_theme_color_non_accent,
        source: "../core/oox/qa/unit/shape.cxx:testWriterShapeFillNonAccentColor",
        file: "tdf152840_theme_color_non_accent.docx",
        path_minimums: [page_count!(0, 4)],
    ),
    case!(
        n793998,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport8.cxx:testN793998",
        file: "n793998.docx",
        contains: [pt!(0, "He heard quiet steps behind him. Over.")],
    ),
    case!(
        footnote_spacing_hanging_para,
        source: "../core/sw/qa/extras/odfexport/odfexport4.cxx:testTdf159382_DOCX",
        file: "footnote_spacing_hanging_para.docx",
        contains: [pt!(0, "1")],
    ),
    case!(
        tdf116256,
        source: "../core/sw/qa/extras/layout/layout2.cxx:testTdf116256",
        file: "tdf116256.docx",
        occurrences: [count!(0, "xxx", 1)],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf124600_layout,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf124600",
        file: "sw/qa/extras/layout/data/tdf124600.docx",
        occurrences: [count!(
            0,
            "nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam",
            1
        )],
    ),
    case!(
        camera_rotation_revolution,
        source: "../core/oox/qa/unit/drawingml.cxx:testCameraRotationRevolution",
        file: "camera-rotation-revolution.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf151518_smartart_text_location,
        source: "../core/oox/qa/unit/shape.cxx:testTdf151518VertAnchor",
        file: "tdf151518_SmartArtTextLocation.docx",
        contains: [pt!(0, "Pet"), pt!(0, "Farm"), pt!(0, "Cat"), pt!(0, "Dog")],
        path_minimums: [page_count!(0, 4)],
    ),
    case!(
        tdf167527_title_letters,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport22.cxx:tdf167527_title_letters_cut_from_below",
        file: "tdf167527_title_letters_cut_from_below.docx",
        contains: [pt!(0, "random text here")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf147126,
        source: "../core/sw/qa/extras/uiwriter/uiwriter3.cxx:testTdf147126",
        file: "tdf147126.docx",
        contains: [pt!(0, "Processo Metodológico da Pesquisa")],
        path_minimums: [page_count!(0, 7)],
    ),
    case!(
        tdf139418,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport22.cxx:testTdf139418",
        file: "tdf139418.docx",
        contains: [pt!(0, "enko Yoshi"), pt!(0, "G")],
    ),
    case!(
        tdf122878,
        source: "../core/sw/qa/extras/layout/layout6.cxx:testTdf122878",
        file: "tdf122878.docx",
        contains: [pt!(0, "1"), pt!(0, "28"), pt!(0, "A1")],
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        studentische_arbeit_header_layout,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport12.cxx:testTd112202",
        file: "090716_Studentische_Arbeit_VWS.docx",
        contains: [pt!(2, "AUFGABENSTELLUNG")],
    ),
    case!(
        n758883_numbering_font_height,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport.cxx:testN758883",
        file: "n758883.docx",
        contains: [pt!(0, "1.")],
    ),
    case!(
        number_portion_noformat,
        source: "../core/sw/qa/core/text/text.cxx:testNumberPortionNoformat",
        file: "number-portion-noformat.docx",
        contains: [pt!(0, "1.")],
    ),
    case!(
        tdf113946,
        source: "../core/sw/qa/extras/ooxmlimport/ooxmlimport2.cxx:testTdf113946",
        file: "tdf113946.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf132976,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport3.cxx:testRelativeAnchorWidthFromLeftMargin",
        file: "tdf132976_testRelativeAnchorWidthFromLeftMargin.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf133861,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport3.cxx:testRelativeAnchorWidthFromInsideOutsideMargin",
        file: "tdf133861_RelativeAnchorWidthFromInsideOutsideMargin.docx",
        path_width_counts: [width_count!(72.0, 2), width_count!(127.6, 2)],
    ),
    case!(
        tdf133045,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport6.cxx:testRelativeAlignmentFromTopMargin",
        file: "tdf133045_TestShapeAlignmentRelativeFromTopMargin.docx",
        path_minimums: [page_count!(0, 3)],
    ),
    case!(
        tdf113183,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport11.cxx:testTdf113183",
        file: "tdf113183.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf120511_eaten_section,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport11.cxx:testTdf120511_eatenSection",
        file: "tdf120511_eatenSection.docx",
        pages: 2,
    ),
    case!(
        tdf119760_position_cell_border,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport11.cxx:testTdf119760_positionCellBorder",
        file: "tdf119760_positionCellBorder.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf116985,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport11.cxx:testTdf116985",
        file: "tdf116985.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf84678,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport9.cxx:testTdf84678",
        file: "tdf84678.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        negative_cell_margin_twips,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx:testNegativeCellMarginTwips",
        file: "negative-cell-margin-twips.docx",
        contains: [pt!(0, "A")],
    ),
    case!(
        tdf153042_large_tab,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx:testTdf153042_largeTab",
        file: "tdf153042_largeTab.docx",
        contains: [pt!(0, "Some regular text.")],
    ),
    case!(
        text_box_word_wrap,
        source: "../core/sw/qa/core/doc/doc.cxx:testTextBoxWordWrap",
        file: "text-box-word-wrap.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf153042_no_tab,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx:testTdf153042_noTab",
        file: "tdf153042_noTab.docx",
        contains: [pt!(0, "Some regular text.")],
    ),
    case!(
        tdf148360,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport17.cxx:testTdf148360",
        file: "tdf148360.docx",
        contains: [pt!(0, "Here should be tab before")],
    ),
    case!(
        first_page_footer_enabled,
        source: "../core/sw/qa/core/header_footer/HeaderFooterTest.cxx:testFirstPageFooterEnabled",
        file: "TestFirstFooterDisabled.docx",
        contains: [pt!(0, "URGENT 1")],
    ),
    case!(
        textbox_right_edge,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport.cxx:testTextboxRightEdge",
        file: "textbox-right-edge.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        wpg_nested,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport10.cxx:testWpgNested",
        file: "wpg-nested.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf133070_has_footer,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport15.cxx:testRelativeAnchorHeightFromBottomMarginHasFooter",
        file: "tdf133070_testRelativeAnchorHeightFromBottomMarginHasFooter.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf123324_has_header,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport23.cxx:testRelativeAnchorHeightFromTopMarginHasHeader",
        file: "tdf123324_testRelativeAnchorHeightFromTopMarginHasHeader.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf123324_no_header,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport23.cxx:testRelativeAnchorHeightFromTopMarginNoHeader",
        file: "tdf123324_testRelativeAnchorHeightFromTopMarginNoHeader.docx",
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf165492_exact,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport22.cxx:testTdf165492_exactWithBottomSpacing",
        file: "tdf165492_exactWithBottomSpacing.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf165492_at_least,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport22.cxx:testTdf165492_atLeastWithBottomSpacing",
        file: "tdf165492_atLeastWithBottomSpacing.docx",
        path_minimums: [page_count!(0, 2)],
    ),
    case!(
        tdf154369_green_heading_numbering,
        source: "../core/sw/qa/extras/ooxmlexport/ooxmlexport21.cxx:testTdf154369",
        file: "tdf154369.docx",
        contains: [pt!(0, "A."), pt!(0, "B.")],
    ),
];

#[test]
fn mapped_docx_layout_matches_libreoffice_layout_coverage() {
    let failures = run_cases_parallel(CASES, run_case, |case, message| {
        format!(
            "{}\n  source: {}\n  file: {}\n  failure: {}",
            case.name, case.source, case.file, message
        )
    });
    assert!(
        failures.is_empty(),
        "{} mapped DOCX layout cases failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn tdf128646_matches_libreoffice_layout_in_cell_shape_position() {
    let document = docx_layout_named("tdf128646.docx").unwrap();
    assert_image_below_table_top_and_flush_right(&document, 0, 1.0);
}

fn run_case(case: &DocxCase) {
    let document = if case.file.contains('/') {
        docx_layout(case.file)
    } else {
        docx_layout_named(case.file)
    }
    .unwrap_or_else(|error| {
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
    for expected in case.contains {
        assert_page_contains(&document, expected.page, expected.text);
    }
    for expected in case.starts_with {
        assert_page_starts_with(&document, expected.page, expected.text);
    }
    for expected in case.ordered {
        assert_page_contains_in_order(&document, expected.page, expected.texts);
    }
    for unexpected in case.absent {
        assert_page_not_contains(&document, unexpected.page, unexpected.text);
    }
    for page_index in case.empty_pages {
        assert_page_has_no_text(&document, *page_index);
    }
    for expected in case.occurrences {
        assert_page_text_occurrences(&document, expected.page, expected.text, expected.count);
    }
    for expected in case.image_counts {
        assert_page_image_count(&document, expected.page, expected.count);
    }
    for expected in case.image_minimums {
        assert_page_image_count_at_least(&document, expected.page, expected.count);
    }
    for expected in case.path_counts {
        assert_page_path_count(&document, expected.page, expected.count);
    }
    for expected in case.path_minimums {
        assert_page_path_count_at_least(&document, expected.page, expected.count);
    }
    for expected in case.path_width_counts {
        assert_path_width_count(&document, expected.width, expected.count, 0.75);
    }
    for expected in case.underlined_text {
        assert_text_underline(&document, expected);
    }
    for expected in case.strikethrough_text {
        assert_text_strikethrough(&document, expected);
    }
}

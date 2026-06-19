use ooxmlsdk_layout::common::Color;
use ooxmlsdk_layout_test::{
    assert_link_target, assert_page_contains, assert_page_contains_any, assert_page_image_count,
    assert_page_not_contains, assert_page_path_count_at_least, assert_page_size,
    assert_page_text_occurrences, assert_text_color, assert_text_font_size, xlsx_layout,
};

#[derive(Clone, Copy)]
struct XlsxCase {
    name: &'static str,
    source: &'static str,
    file: &'static str,
    page_count: usize,
    contains: &'static [PageText],
    contains_any: &'static [PageAnyText],
    not_contains: &'static [PageText],
    occurrences: &'static [PageTextCount],
    page_sizes: &'static [PageSize],
    image_counts: &'static [PageCount],
    link_targets: &'static [&'static str],
    path_minimums: &'static [PageCount],
    font_sizes: &'static [TextFontSize],
    colors: &'static [TextColor],
}

#[derive(Clone, Copy)]
struct PageText {
    page: usize,
    text: &'static str,
}

#[derive(Clone, Copy)]
struct PageAnyText {
    page: usize,
    alternatives: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct PageTextCount {
    page: usize,
    text: &'static str,
    count: usize,
}

#[derive(Clone, Copy)]
struct PageSize {
    page: usize,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct PageCount {
    page: usize,
    count: usize,
}

#[derive(Clone, Copy)]
struct TextFontSize {
    text: &'static str,
    size: f32,
}

#[derive(Clone, Copy)]
struct TextColor {
    text: &'static str,
    color: Color,
}

macro_rules! pt {
    ($page:expr, $text:expr) => {
        PageText {
            page: $page,
            text: $text,
        }
    };
}

macro_rules! any {
    ($page:expr, [$($text:expr),+ $(,)?]) => {
        PageAnyText {
            page: $page,
            alternatives: &[$($text),+],
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

macro_rules! size {
    ($page:expr, $width:expr, $height:expr) => {
        PageSize {
            page: $page,
            width: $width,
            height: $height,
        }
    };
}

macro_rules! font {
    ($text:expr, $size:expr) => {
        TextFontSize {
            text: $text,
            size: $size,
        }
    };
}

macro_rules! color {
    ($text:expr, $r:expr, $g:expr, $b:expr, $a:expr) => {
        TextColor {
            text: $text,
            color: Color {
                r: $r,
                g: $g,
                b: $b,
                a: $a,
            },
        }
    };
}

macro_rules! case {
    (
        $name:ident,
        source: $source:expr,
        file: $file:expr,
        pages: $pages:expr
        $(, contains: [$($contains:expr),* $(,)?])?
        $(, contains_any: [$($contains_any:expr),* $(,)?])?
        $(, not_contains: [$($not_contains:expr),* $(,)?])?
        $(, occurrences: [$($occurrences:expr),* $(,)?])?
        $(, page_sizes: [$($page_sizes:expr),* $(,)?])?
        $(, image_counts: [$($image_counts:expr),* $(,)?])?
        $(, link_targets: [$($link_targets:expr),* $(,)?])?
        $(, path_minimums: [$($path_minimums:expr),* $(,)?])?
        $(, font_sizes: [$($font_sizes:expr),* $(,)?])?
        $(, colors: [$($colors:expr),* $(,)?])?
        $(,)?
    ) => {
        XlsxCase {
            name: stringify!($name),
            source: $source,
            file: $file,
            page_count: $pages,
            contains: &[$($($contains),*)?],
            contains_any: &[$($($contains_any),*)?],
            not_contains: &[$($($not_contains),*)?],
            occurrences: &[$($($occurrences),*)?],
            page_sizes: &[$($($page_sizes),*)?],
            image_counts: &[$($($image_counts),*)?],
            link_targets: &[$($($link_targets),*)?],
            path_minimums: &[$($($path_minimums),*)?],
            font_sizes: &[$($($font_sizes),*)?],
            colors: &[$($($colors),*)?],
        }
    };
}

const CASES: &[XlsxCase] = &[
    case!(
        tdf123026_optimal_row_height,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf123026_optimalRowHeight",
        file: "sc/qa/unit/data/xlsx/tdf123026_optimalRowHeight.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Sales Summary Report"),
            pt!(0, "Single level semi attached"),
            pt!(0, "Reflects $3,526/sqm."),
        ],
    ),
    case!(
        tdf159581_optimal_row_height,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf159581_optimalRowHeight",
        file: "sc/qa/unit/data/xlsx/tdf159581_optimalRowHeight.xlsx",
        pages: 2,
        contains: [
            pt!(0, "One honking big, optimal cell size"),
            pt!(0, "Should not affect other sheets"),
            pt!(1, "still optimally sized row heights on sheet2"),
        ],
    ),
    case!(
        tdf144642_calc_saved_row_height,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf144642_RowHeightRounding_saveByCalc",
        file: "sc/qa/unit/data/xlsx/tdf144642_RowHeight_10mm_SavedByCalc.xlsx",
        pages: 2,
        contains: [pt!(0, "25 ___"), pt!(1, "26 ___")],
    ),
    case!(
        tdf144642_excel_saved_row_height,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf144642_RowHeightRounding_saveByExcel",
        file: "sc/qa/unit/data/xlsx/tdf144642_RowHeight_28.35pt_SavedByExcel.xlsx",
        pages: 1,
        contains: [pt!(0, "26 ___")],
    ),
    case!(
        tdf145129_default_row_height,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf145129_DefaultRowHeightRounding",
        file: "sc/qa/unit/data/xlsx/tdf145129_DefaultRowHeight_28.35pt_SavedByExcel.xlsx",
        pages: 2,
        contains: [pt!(0, "1"), pt!(0, "2"), pt!(1, "28")],
    ),
    case!(
        misc_row_heights,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testMiscRowHeightExport",
        file: "sc/qa/unit/data/xlsx/miscrowheights.xlsx",
        pages: 1,
        occurrences: [count!(0, "30", 6), count!(0, "50", 4)],
    ),
    case!(
        seconds_without_truncate,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testSecondsWithoutTruncateAndDecimals",
        file: "sc/qa/unit/data/xlsx/seconds-without-truncate-and-decimals.xlsx",
        pages: 1,
        contains: [pt!(0, "271433.61")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        embedded_text_in_decimal,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testEmbeddedTextInDecimal",
        file: "sc/qa/unit/data/xlsx/embedded-text-in-decimal.xlsx",
        pages: 1,
        contains: [pt!(0, "6,543,210.123 456 78")],
    ),
    case!(
        cell_borders,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testCellBordersXLSX",
        file: "sc/qa/unit/data/xlsx/cell-borders.xlsx",
        pages: 3,
        contains: [
            pt!(0, "hair"),
            pt!(0, "mediumDashDotDot"),
            pt!(0, "double"),
            pt!(1, "Screenshot of how the borders look in Excel XP"),
        ],
    ),
    case!(
        tdf136721_paper_size,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf136721_paper_size",
        file: "sc/qa/unit/data/xlsx/tdf136721_letter_sized_paper.xlsx",
        pages: 1,
        contains: [pt!(0, "Start"), pt!(0, "End")],
        page_sizes: [size!(0, 419.56, 297.64)],
    ),
    case!(
        tdf166724_cell_anchor,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf166724_cellAnchor",
        file: "sc/qa/unit/data/xlsx/tdf166724_cellAnchor.xlsx",
        pages: 1,
        contains: [pt!(0, "B3 checkBox")],
    ),
    case!(
        image_hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf91634XLSX",
        file: "sc/qa/unit/data/xlsx/image_hyperlink.xlsx",
        pages: 1,
        not_contains: [pt!(0, "https://www.google.com/")],
        image_counts: [page_count!(0, 1)],
        link_targets: ["https://www.google.com/"],
    ),
    case!(
        hidden_shape,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testHiddenShapeXLSX",
        file: "sc/qa/unit/data/xlsx/hiddenShape.xlsx",
        pages: 1,
        image_counts: [page_count!(0, 0)],
    ),
    case!(
        tdf169496_hidden_graphic,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testtdf169496_hidden_graphic",
        file: "sc/qa/unit/data/xlsx/tdf169496_hidden_graphic.xlsx",
        pages: 1,
        image_counts: [page_count!(0, 1)],
    ),
    case!(
        hyperlinks,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testHyperlinksXLSX",
        file: "sc/qa/unit/data/xlsx/hyperlinks.xlsx",
        pages: 6,
        contains: [
            pt!(0, "10:ABC10"),
            pt!(0, "10:ABC11"),
            pt!(0, "10:ABC12"),
            pt!(1, "Invalid Value"),
        ],
    ),
    case!(
        preserve_whitespace,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testPreserveTextWhitespaceXLSX",
        file: "sc/qa/unit/data/xlsx/preserve-whitespace.xlsx",
        pages: 1,
        contains: [pt!(0, "abc")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        preserve_space,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testPreserveTextWhitespace2XLSX",
        file: "sc/qa/unit/data/xlsx/preserve_space.xlsx",
        pages: 1,
        contains: [pt!(0, "abc 123456 456")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        escape_unicode,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testEscapedUnicodeXLSX",
        file: "sc/qa/unit/data/xlsx/escape-unicode.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Line 1"),
            pt!(0, "Line 2"),
            pt!(0, "Line 3"),
            pt!(0, "Line 4"),
        ],
        not_contains: [pt!(0, "_x000D_")],
    ),
    case!(
        cell_multi_line,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testSingleLine_xlsx",
        file: "sc/qa/unit/data/xlsx/cell-multi-line.xlsx",
        pages: 1,
        contains: [pt!(0, "Line1Line2Line3"), pt!(0, "Line1 Line2 Line3")],
    ),
    case!(
        check_boolean,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testBooleanFormatXLSX",
        file: "sc/qa/unit/data/xlsx/check-boolean.xlsx",
        pages: 1,
        occurrences: [count!(0, "TRUE", 2)],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        cell_value,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testCellValueXLSX",
        file: "sc/qa/unit/data/xlsx/cell-value.xlsx",
        pages: 8,
        contains: [
            pt!(0, "-2012"),
            pt!(0, "-3.14"),
            pt!(0, "Hello, Calc!"),
            pt!(0, "Calc is the spreadsheet program you've always needed."),
        ],
    ),
    case!(
        font_size,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testFontSizeXLSX",
        file: "sc/qa/unit/data/xlsx/fontSize.xlsx",
        pages: 1,
        contains: [pt!(0, "sardfasef")],
        font_sizes: [font!("sardfasef", 18.0)],
    ),
    case!(
        text_color,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetRunParagraphPropertyXLSX",
        file: "sc/qa/unit/data/xlsx/TextColor.xlsx",
        pages: 1,
        contains: [pt!(0, "Red Green")],
        colors: [color!("Red", 255, 0, 0, 255)],
    ),
    case!(
        underline_color,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testTextUnderlineColorXLSX",
        file: "sc/qa/unit/data/xlsx/underlineColor.xlsx",
        pages: 1,
        occurrences: [count!(0, "Text Box", 2)],
    ),
    case!(
        strike_through,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testEditEngStrikeThroughXLSX",
        file: "sc/qa/unit/data/xlsx/strike-through.xlsx",
        pages: 1,
        contains: [pt!(0, "this is strike through this not")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        hidden_sheets,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testHiddenSheetsXLSX",
        file: "sc/qa/unit/data/xlsx/hidden_sheets.xlsx",
        pages: 1,
        contains: [pt!(0, "Sheet2")],
        not_contains: [pt!(0, "Sheet1")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        tdf121715_header_footer,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf121715_FirstPageHeaderFooterXLSX",
        file: "sc/qa/unit/data/xlsx/tdf121715.xlsx",
        pages: 2,
        contains: [
            pt!(0, "First Page Header"),
            pt!(0, "First Page Footer"),
            pt!(1, "Even Header"),
            pt!(1, "Even Footer"),
        ],
    ),
    case!(
        tdf134459_header_footer_color,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf134459_HeaderFooterColorXLSX",
        file: "sc/qa/unit/data/xlsx/tdf134459_HeaderFooterColor.xlsx",
        pages: 1,
        occurrences: [count!(0, "l c r", 2)],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        tdf134817_header_footer_sections,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf134817_HeaderFooterTextWith2SectionXLSX",
        file: "sc/qa/unit/data/xlsx/tdf134817_HeaderFooterTextWith2Section.xlsx",
        pages: 1,
        contains: [pt!(0, "aaa bbb"), pt!(0, "cambdant")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        writing_mode,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTextDirectionXLSX",
        file: "sc/qa/unit/data/xlsx/writingMode.xlsx",
        pages: 1,
        contains: [pt!(0, "English (Yes)."), pt!(0, "English(Yes).")],
    ),
    case!(
        hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testHyperlinkXLSX",
        file: "sc/qa/unit/data/xlsx/hyperlink.xlsx",
        pages: 1,
        contains: [pt!(0, ">")],
        link_targets: ["#Sheet2!A1"],
    ),
    case!(
        textbox_hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetTextBoxHyperlinkXLSX",
        file: "sc/qa/unit/data/xlsx/textbox-hyperlink.xlsx",
        pages: 2,
        contains: [pt!(0, "text")],
    ),
    case!(
        chart_hyperlink,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf123645XLSX",
        file: "sc/qa/unit/data/xlsx/chart_hyperlink.xlsx",
        pages: 3,
        contains: [pt!(0, "Chart Title"), pt!(1, "Row 1")],
        link_targets: [
            "file:///C:/TEMP/test.xlsx",
            "#Sheet2!A1",
            "https://bugs.documentfoundation.org/show_bug.cgi?id=123645",
        ],
    ),
    case!(
        table_style,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTotalRowToggle",
        file: "sc/qa/unit/data/xlsx/TableStyleTest.xlsx",
        pages: 1,
        contains: [pt!(0, "A B C"), pt!(0, "Total 3 2.75 Ft")],
    ),
    case!(
        totals_row_function,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTotalsRowFunction",
        file: "sc/qa/unit/data/xlsx/totalsRowFunction.xlsx",
        pages: 1,
        contains: [pt!(0, "PRESENT PLANNER"), pt!(0, "Total £350.00")],
        page_sizes: [size!(0, 792.0, 612.0)],
    ),
    case!(
        totals_row_shown,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTotalsRowShown",
        file: "sc/qa/unit/data/xlsx/totalsRowShown.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Desc Corn Hay Soy"),
            pt!(0, "Price $ 1.00 $ 2.00 $ 3.00"),
            pt!(0, "Unit ton bushel pound"),
        ],
    ),
    case!(
        hidden_button,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testAutofilterHiddenButton",
        file: "sc/qa/unit/data/xlsx/hiddenButton.xlsx",
        pages: 1,
        contains: [pt!(0, "Col 1 Col 2 Col 3 Col 4 Col 5")],
    ),
    case!(
        autofilter_show_button,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testAutofilterShowButton",
        file: "sc/qa/unit/data/xlsx/autofilterShowButton.xlsx",
        pages: 2,
        contains: [pt!(0, "a b c DDD g h III"), pt!(0, "1 def example"), pt!(1, "III")],
    ),
    case!(
        date_autofilter,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testDateAutofilterXLSX",
        file: "sc/qa/unit/data/xlsx/dateAutofilter.xlsx",
        pages: 1,
        contains: [pt!(0, "ID Date"), pt!(0, "one 3/2/2017"), pt!(0, "three 10/1/2014")],
    ),
    case!(
        autofilter_colors,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testAutofilterColorsOOXML",
        file: "sc/qa/unit/data/xlsx/autofilter-colors.xlsx",
        pages: 1,
        contains: [pt!(0, "2 2 2"), pt!(0, "3 3 3")],
        contains_any: [any!(0, ["BackgroundForeground Both", "Background Foreground Both"])],
        not_contains: [pt!(0, "1 1 1")],
    ),
    case!(
        autofilter_colors_fg,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testAutofilterColorsOOXML2",
        file: "sc/qa/unit/data/xlsx/autofilter-colors-fg.xlsx",
        pages: 1,
        contains: [pt!(0, "1 1 1"), pt!(0, "2 2 2"), pt!(0, "3 3 3")],
        contains_any: [any!(0, ["BackgroundForeground Both", "Background Foreground Both"])],
    ),
    case!(
        tdf130104_indent,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf130104_XLSXIndent",
        file: "sc/qa/unit/data/xlsx/tdf130104_indent.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Indent by 0"),
            pt!(0, "Indent by 5"),
            pt!(0, "Indent by 10"),
            pt!(0, "缩进"),
            pt!(0, "0"),
            pt!(0, "5"),
            pt!(0, "10"),
        ],
    ),
    case!(
        tdf134826_header_font_style,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testHeaderFontStyleXLSX",
        file: "sc/qa/unit/data/xlsx/tdf134826.xlsx",
        pages: 1,
        contains: [pt!(0, "Bold"), pt!(0, "Italic")],
    ),
    case!(
        tdf151755_styles,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf151755_stylesLostOnXLSXExport",
        file: "sc/qa/unit/data/xlsx/tdf151755_stylesLostOnXLSXExport.xlsx",
        pages: 1,
        contains: [
            pt!(0, "Daily Calendar Date"),
            pt!(0, "Time Appointment To Do Errands Calls"),
            pt!(0, "07:00:00"),
            pt!(0, "18:30:00"),
        ],
    ),
    case!(
        tdf152581_bordercolor,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf152581_bordercolorNotExportedToXLSX",
        file: "sc/qa/unit/data/xlsx/tdf152581_bordercolorNotExportedToXLSX.xlsx",
        pages: 1,
        contains: [pt!(0, "test"), pt!(0, "x")],
    ),
    case!(
        checkbox_form_control,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testCheckboxFormControlXlsxExport",
        file: "sc/qa/unit/data/xlsx/checkbox-form-control.xlsx",
        pages: 1,
        contains: [pt!(0, "1"), pt!(0, "Check Box 1")],
    ),
    case!(
        radio_buttons,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testActiveXOptionButtonGroup",
        file: "sc/qa/unit/data/xlsx/tdf111980_radioButtons.xlsx",
        pages: 2,
        contains: [
            pt!(0, "Group Box 7"),
            pt!(0, "ActiveX button1"),
            pt!(0, "Form button2"),
            pt!(1, "ActiveX 3"),
        ],
    ),
    case!(
        tdf153767,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf153767",
        file: "sc/qa/unit/data/xlsx/tdf153767.xlsx",
        pages: 2,
        contains: [pt!(0, "Contact Name Address City Postal Code Country"), pt!(1, "TRUE"), pt!(1, "FALSE")],
        page_sizes: [size!(0, 612.0, 792.0)],
    ),
    case!(
        tdf161301,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf161301",
        file: "sc/qa/unit/data/xlsx/tdf161301.xlsx",
        pages: 2,
        occurrences: [count!(1, "CE784年2月20日", 2)],
    ),
    case!(
        new_cond_format,
        source: "../core/sc/qa/unit/cond_format.cxx:testNewCondFormatXLSX",
        file: "sc/qa/unit/data/xlsx/new_cond_format_test.xlsx",
        pages: 3,
        contains: [
            pt!(0, "top n elements bottom n elements top n percent bottom n percent above average"),
            pt!(1, "below average above equal average below equal average"),
            pt!(2, "2.00 2 1 1.000 4.00 3"),
        ],
    ),
    case!(
        tdf83671_smartart,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf83671_SmartArt_import",
        file: "sc/qa/unit/data/xlsx/tdf83671_SmartArt_import.xlsx",
        pages: 1,
        contains: [pt!(0, "start"), pt!(0, "back"), pt!(0, "middle"), pt!(0, "front"), pt!(0, "end")],
    ),
    case!(
        tdf151818_smartart,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf151818_SmartArtFontColor",
        file: "sc/qa/unit/data/xlsx/tdf151818_SmartartThemeFontColor.xlsx",
        pages: 1,
        contains: [pt!(0, "One Two Three")],
    ),
    case!(
        pivot_cache_mixed_types,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotCacheExportXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/with-strings-integers-and-dates.xlsx",
        pages: 3,
        contains: [
            pt!(0, "mixed strings a Sum of all fields are integers"),
            pt!(0, "Total Result 16665"),
            pt!(2, "tekst 6/7/09 10:53 AM"),
        ],
    ),
    case!(
        two_data_fields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableTwoDataFields",
        file: "sc/qa/unit/data/xlsx/pivot-table/two-data-fields.xlsx",
        pages: 2,
        contains: [pt!(0, "Name Sum of Value Count of Value2"), pt!(0, "Total Result 3.6512482152 7"), pt!(1, "Name Value")],
    ),
    case!(
        pivotcompact,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableCompactLayoutXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/pivotcompact.xlsx",
        pages: 2,
        contains: [pt!(1, "Sum of Val D"), pt!(1, "Row Labels ddd ddx Total Result"), pt!(1, "Total Result 40 41 81")],
    ),
    case!(
        first_header_row_zero,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testFirstHeaderRowZero",
        file: "sc/qa/unit/data/xlsx/pivot-table/first_header_row_zero.xlsx",
        pages: 2,
        contains: [pt!(1, "A Suma de N Suma de V"), pt!(1, "Total Result 12 12"), pt!(1, "under")],
    ),
    case!(
        calcfields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFields1XLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/calcfields.xlsx",
        pages: 3,
        contains: [pt!(2, "Pivot Table_Sheet1_1"), pt!(2, "TATA 24 6.00 Ft"), pt!(2, "Total Result 114 18.00 Ft")],
    ),
    case!(
        onlycalcfields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFields2XLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/onlycalcfields.xlsx",
        pages: 4,
        contains: [pt!(2, "Name (empty)"), pt!(2, "TATA"), pt!(2, "TITI"), pt!(2, "TOTO"), pt!(2, "Total Result"), pt!(3, "5")],
    ),
    case!(
        groupwithcalcfields,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testGroupAndCalcFieldXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/groupwithcalcfields.xlsx",
        pages: 4,
        contains: [pt!(0, "Data"), pt!(0, "Group1 45 54 63"), pt!(0, "Total Result 171 189 207")],
    ),
    case!(
        tdf126858,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFieldSingleDataDimXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/tdf126858-1.xlsx",
        pages: 2,
        contains: [
            pt!(0, "товар (empty)"),
            pt!(0, "апельсин банан вишня Total Result"),
            pt!(1, "товар"),
            pt!(1, "кол-во"),
            pt!(1, "цена"),
            pt!(1, "за"),
            pt!(1, "ед"),
            pt!(1, "стоимость"),
        ],
    ),
    case!(
        test_diff_aggregation,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFieldDiffAggregationXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/test_diff_aggregation.xlsx",
        pages: 3,
        contains: [pt!(1, "Pivot Table_Sheet1_1"), pt!(1, "2010 78"), pt!(2, "Pivot Table_Sheet1_2"), pt!(2, "Total Result 6")],
    ),
    case!(
        pivottable_double_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableDoubleFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_double_field_filter.xlsx",
        pages: 2,
        contains: [pt!(0, "Double field1 Double field2 Double field3 Datas"), pt!(0, "2 2.00 20,000.00 12"), pt!(1, "10,000.00 22")],
    ),
    case!(
        pivottable_string_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableStringFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_string_field_filter.xlsx",
        pages: 2,
        contains: [pt!(0, "Order ID Country Sum - Amount"), pt!(0, "United States")],
        not_contains: [pt!(0, "United Kingdom")],
    ),
    case!(
        pivottable_date_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableDateFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_date_field_filter.xlsx",
        pages: 3,
        contains: [pt!(0, "Date Date2 Date3 Sum - Amount"), pt!(1, "2016/ January 7/"), pt!(2, "2016/ 1/ 8. 0:00")],
    ),
    case!(
        shared_group_field,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-group-field.xlsx",
        pages: 4,
        contains: [pt!(0, "a2 Összeg / a Összeg / b"), pt!(0, "Csoport1 15 20 25"), pt!(0, "Total Result 171 189 207"), pt!(1, "Összeg / h Összeg / i")],
    ),
    case!(
        shared_dategroup,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedDateGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-dategroup.xlsx",
        pages: 4,
        contains: [pt!(2, "a Összeg / c Összeg / d Összeg / e"), pt!(2, "1965 163877 212212 262738"), pt!(2, "Total Result 1113132 1301928 2042856")],
    ),
    case!(
        shared_nested_dategroup,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedNestedDateGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-nested-dategroup.xlsx",
        pages: 4,
        contains: [pt!(2, "Row Labels Összeg / c Összeg / e"), pt!(2, "1965"), pt!(2, "Jan 53274 87176"), pt!(2, "Total Result 1113132 2042856")],
    ),
    case!(
        shared_numgroup,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableSharedNumGroupXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/shared-numgroup.xlsx",
        pages: 4,
        contains: [pt!(2, "f Összeg / c Összeg / d Összeg / e"), pt!(2, "32674-47673 193380 194190 414100"), pt!(2, "Total Result 1113132 1301928 2042856")],
    ),
    case!(
        pivottable_bool_field_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableBoolFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_bool_field_filter.xlsx",
        pages: 2,
        contains: [pt!(0, "Bool field Sum of Amount"), pt!(0, "TRUE"), pt!(1, "FALSE")],
        not_contains: [pt!(0, "FALSE")],
    ),
    case!(
        pivottable_row_col_page_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableRowColPageFieldFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_rowcolpage_field_filter.xlsx",
        pages: 3,
        contains: [pt!(0, "Double3 field - multiple -"), pt!(0, "Order ID 2"), pt!(0, "1 $4,270"), pt!(2, "Double3 field Double4 field")],
    ),
    case!(
        pivottable_error_item_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableErrorItemFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_error_item_filter.xlsx",
        pages: 1,
        contains: [pt!(0, "a b b Sum of a"), pt!(0, "2 #DIV/0! Total Result 4")],
    ),
    case!(
        pivottable_long_text,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf125046",
        file: "sc/qa/unit/data/xlsx/pivottable_long_text.xlsx",
        pages: 4,
        contains: [pt!(0, "n (empty)"), pt!(1, "A very-very long"), pt!(1, "greater than wto hundred fifty five character")],
    ),
    case!(
        pivottable_one_second_difference,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf125055",
        file: "sc/qa/unit/data/xlsx/pivottable_1s_difference.xlsx",
        pages: 1,
        contains: [pt!(0, "n d n (empty)"), pt!(0, "a 7/10/2017 9:11"), pt!(0, "b 7/10/2017 9:11")],
    ),
    case!(
        pivot_cached_definition_in_sync,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableXLSX_OutOfSyncPivotTableCachedDefinitionImport",
        file: "sc/qa/unit/data/xlsx/PivotTable_CachedDefinitionAndDataInSync.xlsx",
        pages: 3,
        contains: [pt!(0, "K Sum of A"), pt!(0, "1 5"), pt!(0, "Total Result 10")],
    ),
    case!(
        pivot_cached_definition_with_cache_data,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableXLSX_OutOfSyncPivotTableCachedDefinitionImport2",
        file: "sc/qa/unit/data/xlsx/PivotTable_CachedDefinitionAndDataNotInSync_SheetColumnsRemoved_WithCacheData.xlsx",
        pages: 2,
        contains: [pt!(0, "K Sum of A"), pt!(0, "2 5"), pt!(1, "A K")],
    ),
    case!(
        pivot_cached_definition_without_cache_data,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableXLSX_OutOfSyncPivotTableCachedDefinitionImport3",
        file: "sc/qa/unit/data/xlsx/PivotTable_CachedDefinitionAndDataNotInSync_SheetColumnsRemoved_WithoutCacheData.xlsx",
        pages: 2,
        contains: [pt!(0, "K Sum of A"), pt!(0, "Total Result 10"), pt!(1, "1 1")],
    ),
    case!(
        book1_custom,
        source: "../core/sc/qa/unit/subsequent_export_test6.cxx:testTableStyleCustomRoundtripXLSX",
        file: "sc/qa/unit/data/xlsx/Book1_custom.xlsx",
        pages: 1,
        contains: [pt!(0, "Names Numbers Dates Age"), pt!(0, "Summary 382")],
    ),
    case!(
        tdf165180_date1904,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf165180_date1904_XLSX",
        file: "sc/qa/unit/data/xlsx/tdf165180_date1904.xlsx",
        pages: 1,
        contains: [pt!(0, "Tuesday, March 1, 1904 Mar 1 1904"), pt!(0, "Monday, September 12, 2005 Sept 12 2005")],
    ),
    case!(
        tdf122191,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf122191",
        file: "sc/qa/unit/data/xlsx/tdf122191.xlsx",
        pages: 1,
        contains: [pt!(0, "IGAZ")],
        not_contains: [pt!(0, "BOOL00AN")],
    ),
    case!(
        tdf123139_apply_alignment,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf123139XLSX",
        file: "sc/qa/unit/data/xlsx/tdf123139_applyAlignment.xlsx",
        pages: 2,
        contains: [pt!(0, "foofoofoofoofoofoofoofoofoofoo"), pt!(0, "bar"), pt!(1, "hidden formula"), pt!(1, "unlocked distributed")],
    ),
    case!(
        test_shape_autofit,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testShapeAutofitXLSX",
        file: "sc/qa/unit/data/xlsx/testShapeAutofit.xlsx",
        pages: 2,
        contains: [pt!(0, "This one is autofit."), pt!(0, "This one is not autofit.")],
    ),
    case!(
        pivot_calcfield_nameerror,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testCalcFieldNameErrorXLSX",
        file: "sc/qa/unit/data/xlsx/pivot-table/pivot_calcfield_nameerror.xlsx",
        pages: 3,
        contains: [pt!(0, "Bez Werte Hilfe + Hilfe -"), pt!(2, "Bez bezeichnung von deinen Pluswerten"), pt!(2, "Total Result 1300 -1678")],
    ),
    case!(
        tdf169326,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf169326_ignoreLineBreaksInReferencedCells",
        file: "sc/qa/unit/data/xlsx/tdf169326_ignore_line_breaks_in_referenced_cells.xlsx",
        pages: 1,
        contains: [pt!(0, "Hello World Hello World")],
    ),
    case!(
        tdf120301,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testtdf120301_xmlSpaceParsingXLSX",
        file: "sc/qa/unit/data/xlsx/tdf120301_xmlSpaceParsing.xlsx",
        pages: 1,
        contains: [pt!(0, "Check Box 1"), pt!(0, "Option Button 2")],
    ),
    case!(
        functions_excel_2010,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testFunctionsExcel2010XLSX",
        file: "sc/qa/unit/data/xlsx/functions-excel-2010.xlsx",
        pages: 6,
        contains: [pt!(0, "Function Formula Result should be Equal? All equal?"), pt!(0, "BETA.DIST"), pt!(0, "TRUE")],
    ),
    case!(
        ceiling_floor,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testCeilingFloorXLSX",
        file: "sc/qa/unit/data/xlsx/ceiling-floor.xlsx",
        pages: 4,
        contains: [pt!(0, "23.5 -23.5"), pt!(0, "Err:502"), pt!(0, "#DIV/0!")],
    ),
    case!(
        hyperlink_formula,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf126024XLSX",
        file: "sc/qa/unit/data/xlsx/hyperlink_formula.xlsx",
        pages: 1,
        occurrences: [count!(0, "formula", 2)],
    ),
    case!(
        hyperlink_export,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf126177XLSX",
        file: "sc/qa/unit/data/xlsx/hyperlink_export.xlsx",
        pages: 1,
        contains: [pt!(0, "C:\\TEMP\\test.xlsx#Munka1!A5")],
    ),
    case!(
        tdf135828_shape_rect,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf135828_Shape_Rect",
        file: "sc/qa/unit/data/xlsx/tdf135828_Shape_Rect.xlsx",
        pages: 1,
        path_minimums: [page_count!(0, 1)],
    ),
    case!(
        tdf137000_export_upright,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf137000_handle_upright",
        file: "sc/qa/unit/data/xlsx/tdf137000_export_upright.xlsx",
        pages: 1,
        contains: [pt!(0, "Simple Text")],
    ),
    case!(
        tdf106197_import_upright,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTextBoxBodyUpright",
        file: "sc/qa/unit/data/xlsx/tdf106197_import_upright.xlsx",
        pages: 1,
        contains: [pt!(0, "Simple Text")],
    ),
    case!(
        activex_checkbox,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testActiveXCheckboxXLSX",
        file: "sc/qa/unit/data/xlsx/activex_checkbox.xlsx",
        pages: 1,
        contains: [pt!(0, "Custom Caption")],
    ),
    case!(
        cell_anchored_hidden_shapes,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testCellAnchoredHiddenShapesXLSX",
        file: "sc/qa/unit/data/xlsx/cell-anchored-hidden-shapes.xlsx",
        pages: 1,
        contains: [pt!(0, "TAIWAN PROMOTION AIRFARES"), pt!(0, "BUSINESS CLASS")],
    ),
    case!(
        shape_rotation_import,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testShapeRotationImport",
        file: "sc/qa/unit/data/xlsx/testShapeRotationImport.xlsx",
        pages: 2,
        contains: [pt!(0, "jo"), pt!(0, "bb"), pt!(0, "ra")],
    ),
    case!(
        tdf134455,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf134455",
        file: "sc/qa/unit/data/xlsx/tdf134455.xlsx",
        pages: 2,
        contains: [pt!(0, "00:05"), pt!(0, "01:05"), pt!(0, "04:00")],
    ),
    case!(
        tdf131424,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf131424",
        file: "sc/qa/unit/data/xlsx/tdf131424.xlsx",
        pages: 1,
        contains: [pt!(0, "This is the first column"), pt!(0, "12 23 35"), pt!(0, "36 45 81")],
    ),
    case!(
        tdf122336,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTdf122336",
        file: "sc/qa/unit/data/xlsx/tdf122336.xlsx",
        pages: 9,
        contains: [pt!(0, "UitvoeringsdatumStarttijd"), pt!(0, "12/25/2018 11:30"), pt!(2, "Van Rompaey Marcus")],
    ),
    case!(
        shared_formula_3d_reference,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:shared-formula 3D reference assertions",
        file: "sc/qa/unit/data/xlsx/shared-formula/3d-reference.xlsx",
        pages: 2,
        contains: [pt!(0, "Value Same sheet Another sheet Same sheet but sheet name shown"), pt!(0, "1 1 10 1"), pt!(1, "10 20 30 40 50 60")],
    ),
    case!(
        shared_formula_basic,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testSharedFormulaXLSX",
        file: "sc/qa/unit/data/xlsx/shared-formula/basic.xlsx",
        pages: 1,
        contains: [pt!(0, "Value Formula"), pt!(0, "1 10"), pt!(0, "18 180")],
    ),
    case!(
        shared_formula_text_results,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSharedFormulaStringResultExportXLSX",
        file: "sc/qa/unit/data/xlsx/shared-formula/text-results.xlsx",
        pages: 2,
        contains: [pt!(0, "Text Same Sheet Another Sheet"), pt!(0, "A A AA"), pt!(0, "F F FF"), pt!(1, "AA BB CC DD EE FF")],
    ),
    case!(
        external_refs,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testExternalRefCacheXLSX",
        file: "sc/qa/unit/data/xlsx/external-refs.xlsx",
        pages: 1,
        contains: [pt!(0, "Name Andy Bruce Charlie")],
    ),
    case!(
        ref_string,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testRefStringXLSX",
        file: "sc/qa/unit/data/xlsx/ref_string.xlsx",
        pages: 1,
        contains: [pt!(0, "1 2 3")],
    ),
    case!(
        tdf139934,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testTdf139934",
        file: "sc/qa/unit/data/xlsx/tdf139934.xlsx",
        pages: 9,
        contains: [pt!(0, "Absence Requests"), pt!(0, "1/20/2021 Wednesday Annual Leave")],
    ),
    case!(
        tdf100154,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testNonAsciiWithDotXLSX",
        file: "sc/qa/unit/data/xlsx/tdf100154.xlsx",
        pages: 2,
        contains: [pt!(0, "5"), pt!(1, "5")],
    ),
    case!(
        tdf160371,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf160371",
        file: "sc/qa/unit/data/xlsx/tdf160371.xlsx",
        pages: 1,
        contains: [pt!(0, "Intersection Example Data Table"), pt!(0, "Value 1 Row_2 4 5 6")],
    ),
    case!(
        tdf100709,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf100709XLSX",
        file: "sc/qa/unit/data/xlsx/tdf100709.xlsx",
        pages: 2,
        contains: [pt!(0, "65 218"), pt!(0, "05-Mar-00 218")],
    ),
    case!(
        tdf105272,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf105272",
        file: "sc/qa/unit/data/xlsx/tdf105272.xlsx",
        pages: 1,
        contains: [pt!(0, "Gold Silver Bronze Total"), pt!(0, "13 11 9 33"), pt!(0, "232 0.14")],
    ),
    case!(
        tdf119292,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testTdf119292",
        file: "sc/qa/unit/data/xlsx/tdf119292.xlsx",
        pages: 1,
        contains: [pt!(0, "text rotated by 270 degrees"), pt!(0, "text rotated by 90 degrees")],
    ),
    case!(
        tdf131536,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf131536",
        file: "sc/qa/unit/data/xlsx/tdf131536.xlsx",
        pages: 2,
        contains: [pt!(0, "Excel - true Calc -true"), pt!(0, "L0001 L0001"), pt!(1, "MMR/HEV/LIC/000001")],
    ),
    case!(
        tdf137543,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf137543XLSX",
        file: "sc/qa/unit/data/xlsx/tdf137543.xlsx",
        pages: 3,
        contains: [pt!(0, "25"), pt!(0, "Source array"), pt!(1, "A B E F I J M N Q R U V")],
    ),
    case!(
        tdf170201,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf170201_empty_values_in_array_formulas",
        file: "sc/qa/unit/data/xlsx/tdf170201.xlsx",
        pages: 1,
        contains: [pt!(0, "Sheet1"), pt!(0, "1 3")],
    ),
    case!(
        tdf134553,
        source: "../core/sc/qa/unit/jumbosheets-test.cxx:testTdf134553",
        file: "sc/qa/unit/data/xlsx/tdf134553.xlsx",
        pages: 2,
        contains: [pt!(0, "Chart Title"), pt!(0, "First data point; 2"), pt!(0, "Third data point; 8")],
    ),
    case!(
        tdf142929,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf142929_filterLessThanXLSX",
        file: "sc/qa/unit/data/xlsx/tdf142929.xlsx",
        pages: 1,
        contains: [pt!(0, "Numbers"), pt!(0, "1")],
    ),
    case!(
        tdf143068_top10filter,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testAutofilterTop10XLSX",
        file: "sc/qa/unit/data/xlsx/tdf143068_top10filter.xlsx",
        pages: 1,
        contains: [pt!(0, "Numbers"), pt!(0, "7 8 9 10")],
    ),
    case!(
        tdf144397,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testExternalDefinedNameXLSX",
        file: "sc/qa/unit/data/xlsx/tdf144397.xlsx",
        pages: 1,
        contains: [pt!(0, "Strings With Range name:"), pt!(0, "January January"), pt!(0, "May #N/A")],
    ),
    case!(
        tdf162963,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf162963",
        file: "sc/qa/unit/data/xlsx/tdf162963_TableWithTotalsEnabled.xlsx",
        pages: 1,
        contains: [pt!(0, "Name Sales"), pt!(0, "Miller 23"), pt!(0, "All 115")],
    ),
    case!(
        autofilter,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testAutofilterXLSX",
        file: "sc/qa/unit/data/xlsx/autofilter.xlsx",
        pages: 1,
        contains: [pt!(0, "column1 column2 column3"), pt!(0, "2 3 4"), pt!(0, "4 5 4")],
    ),
    case!(
        tablerefsnamed,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testNamedTableRef",
        file: "sc/qa/unit/data/xlsx/tablerefsnamed.xlsx",
        pages: 1,
        contains: [pt!(0, "Name Score ScoreNames"), pt!(0, "aaa 3.5 ScoreNames aaa TRUE"), pt!(0, "fff 4.1 ScoreNames fff TRUE")],
    ),
    case!(
        database_ranges,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testDatabaseRangesXLSX",
        file: "sc/qa/unit/data/xlsx/database.xlsx",
        pages: 2,
        contains: [pt!(0, "Col1 Col2 Col3 Col4"), pt!(1, "Using named db range Using unnamed db range"), pt!(1, "Name Age Children")],
    ),
    case!(
        matrix_multiplication,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testMatrixMultiplicationXLSX",
        file: "sc/qa/unit/data/xlsx/matrix-multiplication.xlsx",
        pages: 1,
        contains: [pt!(0, "1 2 4 5.2"), pt!(0, "49.2"), pt!(0, "103.6")],
    ),
    case!(
        tdf118990,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf118990",
        file: "sc/qa/unit/data/xlsx/tdf118990.xlsx",
        pages: 1,
        contains: [pt!(0, "333 C")],
        occurrences: [count!(0, "333", 3)],
    ),
    case!(
        tdf163554,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf163554",
        file: "sc/qa/unit/data/xlsx/tdf163554.xlsx",
        pages: 2,
        contains: [pt!(0, "time (misc) - last"), pt!(0, "7 7"), pt!(1, "time (pnrst)")],
    ),
    case!(
        tdf85617,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf85617",
        file: "sc/qa/unit/data/xlsx/tdf85617.xlsx",
        pages: 1,
        contains: [pt!(0, "Code name Quantity Price"), pt!(0, "Товар 1 1 4.5")],
    ),
    case!(
        tdf118668,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTdf118668",
        file: "sc/qa/unit/data/xlsx/tdf118668.xlsx",
        pages: 3,
        contains: [pt!(0, "ПУТЕВОЙ ЛИСТ ТРАКТОРА"), pt!(1, "Результат работы автомобиля за смену")],
    ),
    case!(
        pivottable_outline_mode,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableOutlineModeXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_outline_mode.xlsx",
        pages: 1,
        contains: [pt!(0, "field1 field2 field3"), pt!(0, "Sum of field3"), pt!(0, "Total Result 6")],
    ),
    case!(
        pivottable_tabular_mode,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableTabularModeXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_tabular_mode.xlsx",
        pages: 4,
        contains: [pt!(0, "pwdLastSet (empty)"), pt!(0, "company employeeID Count - mail"), pt!(0, "Total Result 13")],
    ),
    case!(
        pivot_dark1,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf124810_pivotDark",
        file: "sc/qa/unit/data/xlsx/pivot_dark1.xlsx",
        pages: 2,
        contains: [pt!(0, "name date value"), pt!(0, "Count of v date"), pt!(1, "Total Result")],
    ),
    case!(
        pivottable_invalid_formats,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testInvalidFormats",
        file: "sc/qa/unit/data/xlsx/pivottable_invalid_formats.xlsx",
        pages: 5,
        contains: [pt!(0, "Spalte 1 Spalte 2 Spalte 3"), pt!(0, "A Ur 600 0.65"), pt!(1, "H Th 1144 149.89")],
    ),
    case!(
        tdf89139_pivot_table,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableExportXLSX",
        file: "sc/qa/unit/data/xlsx/tdf89139_pivot_table.xlsx",
        pages: 2,
        contains: [pt!(0, "Name Id Dept Date"), pt!(1, "Dept Count of Id"), pt!(1, "Total Result 17")],
    ),
    case!(
        tdf165503,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf165503",
        file: "sc/qa/unit/data/xlsx/tdf165503.xlsx",
        pages: 2,
        contains: [pt!(0, "Date Val"), pt!(0, "1/3/2021 3"), pt!(0, "10/10/2022 9")],
    ),
    case!(
        tdf137091,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf137091",
        file: "sc/qa/unit/data/xlsx/tdf137091.xlsx",
        pages: 2,
        contains: [pt!(0, "SG STOKKODU Sütun2"), pt!(0, "IP 1 B J7 0280 4 4 08 YY MR0084 28/4")],
    ),
    case!(
        tdf70455,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf70455",
        file: "sc/qa/unit/data/xlsx/tdf70455.xlsx",
        pages: 1,
        contains: [pt!(0, "Bet History"), pt!(0, "Gross: €780.00"), pt!(0, "€130.00 No 7/1 8.0000 Win €1,040.00 €910.00")],
    ),
    case!(
        tdf98481,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf98481",
        file: "sc/qa/unit/data/xlsx/tdf98481.xlsx",
        pages: 3,
        contains: [pt!(0, "Horizontal and vertical Sums with source as separate sheet."), pt!(0, "Sum 4 0 3")],
    ),
    case!(
        tdf115022,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf115022",
        file: "sc/qa/unit/data/xlsx/tdf115022.xlsx",
        pages: 1,
        contains: [pt!(0, "Index amount"), pt!(0, "a 1 a 2 a 3"), pt!(0, "6")],
    ),
    case!(
        tdf164895,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf164895",
        file: "sc/qa/unit/data/xlsx/tdf164895.xlsx",
        pages: 1,
        contains: [pt!(0, "a 1"), pt!(0, "8 30 5")],
    ),
    case!(
        tdf162093,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf162093",
        file: "sc/qa/unit/data/xlsx/tdf162093.xlsx",
        pages: 2,
        contains: [pt!(0, "Surname Count Region Surname Count Region"), pt!(0, "Murray 15 North Murray 15 North"), pt!(0, "Total 296 Total 296"), pt!(1, "{=myData[#Headers]} =myData[#Headers]")],
    ),
    case!(
        tdf147955,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf147955",
        file: "sc/qa/unit/data/xlsx/tdf147955.xlsx",
        pages: 1,
        contains: [pt!(0, "Leasingham Community Benefit Society Ltd"), pt!(0, "Sales 892.75"), pt!(0, "Food - CoS 130.25"), pt!(0, "Cleaning 10.98")],
    ),
    case!(
        tdf155046,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf155046",
        file: "sc/qa/unit/data/xlsx/tdf155046.xlsx",
        pages: 5,
        contains: [pt!(0, "Respondent ID Publication ID Submitted Submitted Time"), pt!(0, "89nre8cuc7i3 69lue27dr864 TRUE"), pt!(1, "Publication Consent")],
    ),
    case!(
        tdf136364,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf136364",
        file: "sc/qa/unit/data/xlsx/tdf136364.xlsx",
        pages: 1,
        contains: [pt!(0, "1 1 1 1 27"), pt!(0, "2 2 2 2 12")],
    ),
    case!(
        tdf157689,
        source: "../core/sc/qa/unit/subsequent_filters_test5.cxx:testTdf157689",
        file: "sc/qa/unit/data/xlsx/tdf157689.xlsx",
        pages: 2,
        contains: [pt!(0, "col1 col2"), pt!(0, "1 2 2 3"), pt!(1, "col3 col4"), pt!(1, "2 2 4 2")],
    ),
    case!(
        tdf142905,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf142905",
        file: "sc/qa/unit/data/xlsx/tdf142905.xlsx",
        pages: 1,
        contains: [pt!(0, "3M 3M")],
    ),
    case!(
        tdf119190,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf119190",
        file: "sc/qa/unit/data/xlsx/tdf119190.xlsx",
        pages: 1,
        contains: [pt!(0, "Kelemen Gábor 2:"), pt!(0, "Comment!")],
    ),
    case!(
        tdf141495,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf141495",
        file: "sc/qa/unit/data/xlsx/tdf141495.xlsx",
        pages: 2,
        contains: [pt!(0, "44227 44255 44286 44316"), pt!(1, "44804 44834 44865 44895 44926")],
    ),
    case!(
        tdf79972,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf79972XLSX",
        file: "sc/qa/unit/data/xlsx/tdf79972.xlsx",
        pages: 1,
        contains: [pt!(0, "123")],
        link_targets: ["https://bugs.documentfoundation.org/show_bug.cgi?id=79972"],
    ),
    case!(
        tdf111876,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf111876",
        file: "sc/qa/unit/data/xlsx/tdf111876.xlsx",
        pages: 1,
        contains: [pt!(0, "..\\xls\\bug-fixes.xls")],
    ),
    case!(
        tdf119565,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf119565",
        file: "sc/qa/unit/data/xlsx/tdf119565.xlsx",
        pages: 1,
        contains: [pt!(0, "Lorem ipsum dolor"), pt!(0, "Maecenas porttitor")],
    ),
    case!(
        textbox_char_kerning_space,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetCharacterKerningSpaceXLSX",
        file: "sc/qa/unit/data/xlsx/textbox-CharKerningSpace.xlsx",
        pages: 2,
        contains: [pt!(0, "AVAIL")],
    ),
    case!(
        textbox_condensed_character_space,
        source: "../core/sc/qa/unit/subsequent_export_test3.cxx:testSheetCondensedCharacterSpaceXLSX",
        file: "sc/qa/unit/data/xlsx/textbox-CondensedCharacterSpace.xlsx",
        pages: 2,
        contains: [pt!(0, "AvaiL")],
    ),
    case!(
        tdf141644,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTextBoxBodyRotateAngle",
        file: "sc/qa/unit/data/xlsx/tdf141644.xlsx",
        pages: 1,
        contains: [pt!(0, "Textdir: 270 deg")],
    ),
    case!(
        tdf142881,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf142881",
        file: "sc/qa/unit/data/xlsx/tdf142881.xlsx",
        pages: 2,
        contains: [pt!(0, "Rotated:"), pt!(0, "35"), pt!(1, "25")],
    ),
    case!(
        tdf145057,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf145057",
        file: "sc/qa/unit/data/xlsx/tdf145057.xlsx",
        pages: 1,
        contains: [pt!(0, "Numbers Names abc def fgh"), pt!(0, "4.00 s 3 4 1")],
    ),
    case!(
        tdf161365,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf161365",
        file: "sc/qa/unit/data/xlsx/tdf161365.xlsx",
        pages: 1,
        contains: [pt!(0, "This spreadsheet contains checkbox which dissapeared after re-export")],
    ),
    case!(
        test_115192,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf115192XLSX",
        file: "sc/qa/unit/data/xlsx/test_115192.xlsx",
        pages: 1,
        contains: [pt!(0, "Hyperlink: test.xlxs"), pt!(0, "Hyperlink: Sheet2!A1"), pt!(0, "Hyperlink: Bug 115192")],
    ),
    case!(
        pivot_many_fields_in_cache,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:many-fields-in-cache import check",
        file: "sc/qa/unit/data/xlsx/pivot-table/many-fields-in-cache.xlsx",
        pages: 2,
        contains: [pt!(0, "F1 F2 F3 F4 F5 F6 F7 F8"), pt!(0, "Sum of F10 F4"), pt!(0, "Total Result 5 6 11")],
    ),
    case!(
        pivottable_duplicated_member_filter,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testPivotTableDuplicatedMemberFilterXLSX",
        file: "sc/qa/unit/data/xlsx/pivottable_duplicated_member_filter.xlsx",
        pages: 3,
        contains: [pt!(0, "pwdLastSet (empty)"), pt!(0, "First type 4"), pt!(0, "Second type 9"), pt!(0, "Total Result 13")],
    ),
    case!(
        tdf104310,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTdf104310_x14",
        file: "sc/qa/unit/data/xlsx/tdf104310.xlsx",
        pages: 1,
        contains: [pt!(0, "1 2 3 4 5")],
    ),
    case!(
        text_length_data_validity,
        source: "../core/sc/qa/unit/subsequent_filters_test3.cxx:testTextLengthDataValidityXLSX",
        file: "sc/qa/unit/data/xlsx/textLengthDataValidity.xlsx",
        pages: 1,
        contains: [pt!(0, "1234"), pt!(0, "1234.00"), pt!(0, "12.3")],
    ),
    case!(
        tdf129985,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf129985",
        file: "sc/qa/unit/data/xlsx/tdf129985.xlsx",
        pages: 1,
        contains: [pt!(0, "Érvényesség kezdete")],
        occurrences: [count!(0, "1/13/2020", 2)],
    ),
    case!(
        tdf73063,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf73063",
        file: "sc/qa/unit/data/xlsx/tdf73063.xlsx",
        pages: 1,
        contains: [pt!(0, "Saturday, 17. June 1972")],
    ),
    case!(
        tdf139021,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testExtCondFormatXLSX",
        file: "sc/qa/unit/data/xlsx/tdf139021.xlsx",
        pages: 1,
        contains: [pt!(0, "hello hello"), pt!(0, "hello bye")],
    ),
    case!(
        tdf139394,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf139394",
        file: "sc/qa/unit/data/xlsx/tdf139394.xlsx",
        pages: 1,
        occurrences: [count!(0, "+", 3), count!(0, "-", 1)],
    ),
    case!(
        tdf55417,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf55417",
        file: "sc/qa/unit/data/xlsx/tdf55417.xlsx",
        pages: 1,
        contains: [pt!(0, "29")],
    ),
    case!(
        tdf139167,
        source: "../core/sc/qa/unit/subsequent_export_test.cxx:testTdf139167",
        file: "sc/qa/unit/data/xlsx/tdf139167.xlsx",
        pages: 1,
        contains: [pt!(0, "Hello")],
    ),
    case!(
        tdf109061,
        source: "../core/sc/qa/unit/jumbosheets-test.cxx:testTdf109061",
        file: "sc/qa/unit/data/xlsx/tdf109061.xlsx",
        pages: 1,
        contains: [pt!(0, "Test"), pt!(0, "1 2 3"), pt!(0, "Sum: 6")],
    ),
    case!(
        tdf112106,
        source: "../core/sc/qa/unit/pivottable_filters_test.cxx:testTdf112106",
        file: "sc/qa/unit/data/xlsx/tdf112106.xlsx",
        pages: 2,
        contains: [pt!(0, "Country - all -"), pt!(0, "Banana $617"), pt!(0, "Total Result $13,126"), pt!(1, "Order ID Product Category Amount Date Country")],
    ),
    case!(
        tdf155402,
        source: "../core/sc/qa/unit/subsequent_filters_test4.cxx:testTdf155402",
        file: "sc/qa/unit/data/xlsx/tdf155402.xlsx",
        pages: 2,
        contains: [pt!(0, "[tdf155402.xlsx]Sheet1")],
    ),
    case!(
        tdf91251,
        source: "../core/sc/qa/unit/subsequent_export_test4.cxx:testTdf91251_missingOverflowRoundtrip",
        file: "sc/qa/unit/data/xlsx/tdf91251_missingOverflowRoundtrip.xlsx",
        pages: 1,
        contains: [pt!(0, "Text Box")],
    ),
    case!(
        tdf164417,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf164417",
        file: "sc/qa/unit/data/xlsx/tdf164417.xlsx",
        pages: 2,
        contains: [pt!(0, "Num Text Date"), pt!(0, "1 a 31/12/24"), pt!(0, "2 a 31/12/2024 (text)")],
    ),
    case!(
        tdf165886,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf165886",
        file: "sc/qa/unit/data/xlsx/tdf165886.xlsx",
        pages: 1,
        contains: [pt!(0, "0 #NAME? TRUE")],
        occurrences: [count!(0, "#NAME?", 8)],
    ),
    case!(
        tdf166413,
        source: "../core/sc/qa/unit/subsequent_export_test5.cxx:testTdf166413",
        file: "sc/qa/unit/data/xlsx/tdf166413.xlsx",
        pages: 1,
        occurrences: [count!(0, r#"test ABC "ABC""#, 4)],
    ),
    case!(
        tdf95640,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testTdf95640_xlsx_to_xlsx",
        file: "sc/qa/unit/data/xlsx/tdf95640.xlsx",
        pages: 1,
        contains: [pt!(0, "AAA BBB"), pt!(0, "jan 2"), pt!(0, "feb 1"), pt!(0, "mar 3")],
    ),
    case!(
        tdf97598,
        source: "../core/sc/qa/unit/subsequent_filters_test2.cxx:testTdf97598XLSX",
        file: "sc/qa/unit/data/xlsx/tdf97598_scenarios.xlsx",
        pages: 1,
        contains: [pt!(0, "Cell A1")],
    ),
    case!(
        tdf81939,
        source: "../core/sc/qa/unit/subsequent_export_test2.cxx:testEscapeCharInNumberFormatXLSX",
        file: "sc/qa/unit/data/xlsx/tdf81939.xlsx",
        pages: 1,
        contains: [pt!(0, "01 23 45 67 89"), pt!(0, "01.23.45.678.9"), pt!(0, "123,456,789 €")],
    ),
    case!(
        universal_content_strict,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testContentXLSXStrict",
        file: "sc/qa/unit/data/xlsx/universal-content-strict.xlsx",
        pages: 4,
        contains: [pt!(0, "1 String1 6"), pt!(0, "2 String2 5"), pt!(0, "-1 11")],
    ),
    case!(
        row_index_1_based,
        source: "../core/sc/qa/unit/subsequent_filters_test.cxx:testRowIndex1BasedXLSX",
        file: "sc/qa/unit/data/xlsx/row-index-1-based.xlsx",
        pages: 1,
        contains: [pt!(0, "Action Plan.Name Action Plan.Description"), pt!(0, "Jerry This is a longer Text."), pt!(0, "Second line."), pt!(0, "Third line.")],
    ),
    case!(
        pivot1_row,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot1_Row",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot1_Row.xlsx",
        pages: 2,
        contains: [pt!(1, "Name Type Sum of Price"), pt!(1, "X1 A 100"), pt!(1, "X5 A 50")],
    ),
    case!(
        pivot1_row_grand,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot1_Row_Grand",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot1_Row_Grand.xlsx",
        pages: 2,
        contains: [pt!(1, "Name Type Sum of Price"), pt!(1, "Total Result 600")],
    ),
    case!(
        pivot1_row_grand_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot1_Row_Grand_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot1_Row_Grand_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "X1 Result 100"), pt!(1, "X5 Result 50"), pt!(1, "Total Result 600")],
    ),
    case!(
        pivot2_row,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name Sum of Price"), pt!(1, "A X1 100"), pt!(1, "B X2 200")],
    ),
    case!(
        pivot2_row_compact,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Compact",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Compact.xlsx",
        pages: 2,
        contains: [pt!(1, "Row Labels Sum of Price"), pt!(1, "X1 100"), pt!(1, "X4 100")],
    ),
    case!(
        pivot2_row_grand,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Grand",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Grand.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name Sum of Price"), pt!(1, "Total Result 600")],
    ),
    case!(
        pivot2_row_grand_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Grand_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Grand_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "A Result 150"), pt!(1, "B Result 350"), pt!(1, "Total Result 600")],
    ),
    case!(
        pivot2_row_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "A Result 150"), pt!(1, "B Result 350"), pt!(1, "C Result 100")],
    ),
    case!(
        pivot2_row_subtotals_sort_desc,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot2_Row_Subtotals_SortDescendingAll",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot2_Row_Subtotals_SortDescendingAll.xlsx",
        pages: 2,
        contains: [pt!(1, "C X4 100"), pt!(1, "B X3 150"), pt!(1, "A X5 50")],
    ),
    case!(
        pivot3_column_grand_subtotals,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot3_Column_Grand_Subtotals",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot3_Column_Grand_Subtotals.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name"), pt!(1, "Total Result"), pt!(1, "100 50 150 200 150 350 100 100 600")],
    ),
    case!(
        pivot4_column_grand_subtotals_sort_desc,
        source: "../core/sc/qa/unit/PivotTable_FieldsAndItemsExport.cxx:Pivot4_Column_Grand_Subtotals_SortDescending",
        file: "sc/qa/unit/data/xlsx/pivot/Pivot4_Column_Grand_Subtotals_SortDescending.xlsx",
        pages: 2,
        contains: [pt!(1, "Type Name"), pt!(1, "Total Result"), pt!(1, "100 100 200 150 350 100 50### 600")],
    ),
    case!(
        new_cond_format_export,
        source: "../core/sc/qa/unit/cond_format.cxx:testConditionalFormatExportXLSX",
        file: "sc/qa/unit/data/xlsx/new_cond_format_test_export.xlsx",
        pages: 3,
        contains: [
            pt!(0, "top n elements bottom n elements top n percent bottom n percent above average"),
            pt!(1, "below average above equal average below equal average"),
            pt!(1, "1.00 2 2.00"),
        ],
    ),
];

#[test]
fn xlsx_layout_matches_libreoffice_layout_coverage() {
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
        "{} XLSX layout cases failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn run_case(case: &XlsxCase) {
    let document = xlsx_layout(case.file).unwrap_or_else(|error| {
        panic!(
            "{}: failed to build layout for {}; source={}: {error}",
            case.name, case.file, case.source
        )
    });
    assert_eq!(
        document.pages.len(),
        case.page_count,
        "{} page count mismatch; source={}; file={}",
        case.name,
        case.source,
        case.file
    );
    for expected in case.contains {
        assert_page_contains(&document, expected.page, expected.text);
    }
    for expected in case.contains_any {
        assert_page_contains_any(&document, expected.page, expected.alternatives);
    }
    for unexpected in case.not_contains {
        assert_page_not_contains(&document, unexpected.page, unexpected.text);
    }
    for expected in case.occurrences {
        assert_page_text_occurrences(&document, expected.page, expected.text, expected.count);
    }
    for expected in case.page_sizes {
        assert_page_size(&document, expected.page, expected.width, expected.height);
    }
    for expected in case.image_counts {
        assert_page_image_count(&document, expected.page, expected.count);
    }
    for expected in case.link_targets {
        assert_link_target(&document, expected);
    }
    for expected in case.path_minimums {
        assert_page_path_count_at_least(&document, expected.page, expected.count);
    }
    for expected in case.font_sizes {
        assert_text_font_size(&document, expected.text, expected.size);
    }
    for expected in case.colors {
        assert_text_color(&document, expected.text, expected.color);
    }
}

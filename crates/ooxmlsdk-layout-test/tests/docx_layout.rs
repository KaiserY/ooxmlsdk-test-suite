use ooxmlsdk_layout_test::{
    assert_page_contains, docx_layout, line_heights, row_heights, table_row_count_for_block,
};

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport14.cxx::testTdf151704_thinColumnHeight
fn docx_tdf151704_keeps_nested_and_follow_table_row_heights_equal() {
    let document =
        docx_layout("sw/qa/extras/ooxmlexport/data/tdf151704_thinColumnHeight.docx").unwrap();
    let first_page_rows = row_heights(&document, 0);
    let second_page_rows = row_heights(&document, 1);
    assert!(
        first_page_rows.iter().any(|first| {
            *first > 1.0
                && second_page_rows
                    .iter()
                    .any(|second| (first - second).abs() <= 0.5)
        }),
        "LibreOffice asserts the nested table row height on page 1 equals the follow table row height on page 2; page1={first_page_rows:?}; page2={second_page_rows:?}"
    );
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport18.cxx::testTdf153128
fn docx_tdf153128_keeps_first_line_height_near_one_point() {
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf153128.docx").unwrap();
    let heights = line_heights(&document, 0);
    let first = heights
        .first()
        .copied()
        .unwrap_or_else(|| panic!("missing first layout line; heights={heights:?}"));
    assert!(
        first > 0.0 && first < 30.0 / 20.0,
        "LibreOffice asserts the first line height is positive and near the 20 twip text height; first={first}; heights={heights:?}"
    );
}

#[test]
// Source: ../core/sw/qa/extras/layout/layout5.cxx::testTdf153136
fn docx_tdf153136_preserves_space_character_line_height_rules() {
    let document = docx_layout("sw/qa/extras/layout/data/tdf153136.docx").unwrap();
    let page_one_lines = line_heights(&document, 0);
    let page_two_rows = row_heights(&document, 1);
    assert!(
        page_one_lines.iter().any(|height| *height < 300.0 / 20.0),
        "LibreOffice small line threshold is 300 twips; page_one_lines={page_one_lines:?}"
    );
    assert!(
        page_one_lines.iter().any(|height| *height > 1000.0 / 20.0),
        "LibreOffice large line threshold is 1000 twips; page_one_lines={page_one_lines:?}"
    );
    assert!(
        page_two_rows.iter().any(|height| *height < 300.0 / 20.0),
        "LibreOffice small row threshold is 300 twips; page_two_rows={page_two_rows:?}"
    );
    assert!(
        page_two_rows.iter().any(|height| *height > 1000.0 / 20.0),
        "LibreOffice large row threshold is 1000 twips; page_two_rows={page_two_rows:?}"
    );
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx::testTdf81100
fn docx_tdf81100_keeps_explicit_no_repeat_header_flow_across_three_pages() {
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf81100.docx").unwrap();
    assert_eq!(document.pages.len(), 3);
    assert_eq!(table_row_count_for_block(&document, 1, 1), 2);
    assert_eq!(table_row_count_for_block(&document, 2, 4), 1);
}

#[test]
// Source: ../core/sw/qa/extras/ooxmlexport/ooxmlexport26.cxx::testTdf58944RepeatingTableHeader
fn docx_tdf58944_repeating_header_keeps_second_page_table_content() {
    let document =
        docx_layout("sw/qa/extras/ooxmlexport/data/tdf58944-repeating-table-header.docx").unwrap();
    assert_eq!(document.pages.len(), 2);
    assert_page_contains(&document, 1, "Test1");
    assert_page_contains(&document, 1, "Test2");
}

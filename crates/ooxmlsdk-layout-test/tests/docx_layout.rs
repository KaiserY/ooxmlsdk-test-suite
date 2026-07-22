use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main as w;
use ooxmlsdk_layout::common::DisplayItem;
use ooxmlsdk_layout::{LayoutOptions, docx};
use ooxmlsdk_layout_test::{
    assert_close, assert_page_contains, docx_layout, line_heights, row_heights,
    table_row_count_for_block, text_origins_for,
};
use std::path::Path;

#[test]
// Sources: immutable Microsoft Office fixed-format output for tdf108714.docx.
// The source is intentionally non-conformant: Word recovers w:br children
// placed directly in w:body and w:tc instead of discarding them.
fn docx_tdf108714_recovers_out_of_place_breaks_and_minimal_table() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/LibreOffice/sw/qa/extras/ooxmlimport/data/tdf108714.docx");
    let mut package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let options = LayoutOptions {
        ui_language: Some("zh-CN".to_string()),
        ..LayoutOptions::default()
    };
    let main = package.main_document_part().unwrap();
    let root = main.root_element(&mut package).unwrap();
    let table = root
        .body
        .as_ref()
        .unwrap()
        .body_choice
        .iter()
        .find_map(|choice| match choice {
            w::BodyChoice::Table(table) => Some(table.as_ref()),
            _ => None,
        })
        .expect("minimal table must survive typed import");
    let row = table
        .table_choice2
        .iter()
        .find_map(|choice| match choice {
            w::TableChoice2::TableRow(row) => Some(row.as_ref()),
            _ => None,
        })
        .expect("minimal table row must survive typed import");
    assert_eq!(
        row.table_row_choice
            .iter()
            .filter(|choice| matches!(choice, w::TableRowChoice::TableCell(_)))
            .count(),
        1,
        "minimal table cell must survive typed import"
    );
    let summary = docx::inspect_layout(&mut package, &options).unwrap();
    let document = docx::layout_document(&mut package, &options).unwrap();

    assert_eq!(document.pages.len(), 4);
    assert_eq!(
        summary.rows.len(),
        1,
        "minimal table row must be laid out; frames={:?}; follows={:?}; backward_moves={:?}; reruns={:?}",
        document
            .frames
            .iter()
            .map(|frame| (
                frame.page_index,
                frame.block_index,
                frame.kind.as_ref(),
                frame.fragments.len()
            ))
            .collect::<Vec<_>>(),
        document.follows,
        document.reflow.backward_moves,
        document.reflow.layout_reruns,
    );
    assert_page_contains(&document, 2, "Paragraph 5 in table");
}

#[test]
// Sources: ECMA-376 Part 1 §21.2.2 (DrawingML charts);
// ../core/chart2/source/view/axes/VCartesianAxis.cxx;
// immutable Microsoft Office fixed-format output for testBarChart.docx.
fn docx_clustered_column_chart_uses_semantic_axes_categories_and_legend() {
    let document = docx_layout("chart2/qa/extras/data/docx/testBarChart.docx").unwrap();
    let texts = document.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();

    for expected in ["0", "6", "Category 1", "Category 4", "Series 1", "Series 3"] {
        assert!(
            texts.contains(&expected),
            "missing semantic chart text {expected:?}; texts={texts:?}"
        );
    }
    for cached_value in ["4.3", "4.4000000000000004"] {
        assert!(
            !texts.contains(&cached_value),
            "cached series value must not become document body text: {texts:?}"
        );
    }
}

#[test]
// Sources:
// - ECMA-376 Part 1 §§17.3.2.36 and 17.9.29
// - [MS-OI29500] §2.1.97
// - ../core/sw/source/writerfilter/dmapper/DomainMapper.cxx (specVanish)
// - immutable Microsoft Office golden for tdf131728.docx
fn docx_tdf131728_advances_long_numbering_past_the_style_separator_heading() {
    let document = docx_layout("sw/qa/extras/ooxmlexport/data/tdf131728.docx").unwrap();
    let label = text_origins_for(&document, 0, "Article 1.")
        .into_iter()
        .next()
        .expect("Article 1 numbering label");
    let heading = text_origins_for(&document, 0, "Definitions")
        .into_iter()
        .next()
        .expect("Definitions heading");

    assert_close(label.x.0, 108.0, 0.1, "numbering label origin");
    assert_close(heading.x.0, 162.0, 0.1, "heading origin after tab");
}

#[test]
// Sources: ECMA-376 Part 1 §§20.4.2.12, 20.4.3.5;
// ../core/sw/qa/writerfilter/ooxml/ooxml.cxx::testNestedRuns
fn docx_nested_runs_keeps_paragraph_relative_textbox_offset() {
    let document = docx_layout("sw/qa/writerfilter/ooxml/data/nested-runs.docx").unwrap();
    let origins = text_origins_for(&document, 0, "Test text box");
    let origin = origins
        .first()
        .unwrap_or_else(|| panic!("missing nested textbox text; origins={origins:?}"));

    assert_close(
        document.pages[0].setup.margins.top.0,
        56.7,
        0.05,
        "page top margin",
    );
    assert_close(origin.x.0, 88.85, 0.05, "textbox horizontal origin");
    assert_close(origin.y.0, 72.8, 0.05, "textbox vertical origin");
}

#[test]
// Sources: ECMA-376 Part 1 §§17.3.2.26 and 17.15.1.88.
fn docx_mailmerge_uses_word_font_slots_for_latin1_delimiters() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/mailmerge.docx",
    );
    let mut package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(
        &mut package,
        &LayoutOptions {
            ui_language: Some("zh-CN".to_string()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    let style = document
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find_map(|item| match item {
            DisplayItem::Text(text) if text.text == "»" => Some(&text.style),
            _ => None,
        })
        .expect("mail-merge closing delimiter");

    assert!(style.wordprocessingml_font_slots);
    assert_eq!(style.font_family.as_deref(), Some("Calibri"));
}

#[test]
// Sources: ECMA-376 Part 1 §§17.15.1.18 and 17.18.7;
// [MS-OE376] Part 4 §2.15.3.31 (lineWrapLikeWord6).
fn docx_a5_uses_word_full_width_punctuation_line_fitting() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/page layout/page layout(big file)/A5.docx",
    );
    let mut package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(
        &mut package,
        &LayoutOptions {
            ui_language: Some("zh-CN".to_string()),
            ..LayoutOptions::default()
        },
    )
    .unwrap();
    let lines = document.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.trim_start()),
            _ => None,
        })
        .collect::<Vec<_>>();

    for expected in [
        "哈巴谷爱神，但却很少有人像他这样，预备好与",
        "神进行对话，质询神的作为是否公义。当然，在属灵",
        "生活中，大部分信徒都会遭遇怀疑神、质问神的时刻。",
    ] {
        assert!(
            lines.contains(&expected),
            "missing Office line {expected:?}; lines={lines:?}"
        );
    }
}

#[test]
// Sources:
// - ECMA-376 Part 1 §§17.5.2.32–33: a cell-level structured document tag
//   surrounds one table cell, and its sdtContent contains that cell.
// - immutable Microsoft Office fixed-format output for the Open XML SDK
//   wordprocessing/SDT/Sdt/sdtContent.docx fixture.
fn docx_cell_level_sdt_keeps_the_wrapped_cell_on_its_row_baseline() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/wordprocessing/SDT/Sdt/sdtContent.docx",
    );
    let mut package = WordprocessingDocument::new_from_file(fixture).unwrap();
    let document = docx::layout_document(&mut package, &LayoutOptions::default()).unwrap();
    let page = &document.pages[0];
    let item_index_for = |expected: &str| {
        page.items
            .iter()
            .rposition(
                |item| matches!(item, DisplayItem::Text(text) if text.text.contains(expected)),
            )
            .unwrap_or_else(|| panic!("missing text item {expected:?}"))
    };
    for item_index in [
        item_index_for("This is an SdtCell."),
        item_index_for("SdtRun"),
    ] {
        let owners = document
            .frames
            .iter()
            .filter(|frame| frame.page_index == 0)
            .filter(|frame| {
                frame.lines.iter().any(|line| {
                    line.item_range.start <= item_index && item_index < line.item_range.end
                })
            })
            .map(|frame| frame.kind.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            owners,
            ["table"],
            "table-cell text owner for item {item_index}"
        );
        let cell_fragments = document
            .frames
            .iter()
            .filter(|frame| frame.page_index == 0 && frame.kind == "table")
            .flat_map(|frame| &frame.fragments)
            .filter(|fragment| {
                fragment.kind == ooxmlsdk_layout::common::FrameFragmentKind::TableCell
                    && fragment.item_range.start <= item_index
                    && item_index < fragment.item_range.end
            })
            .collect::<Vec<_>>();
        assert_eq!(
            cell_fragments.len(),
            1,
            "table-cell fragment for item {item_index}: {cell_fragments:?}"
        );
        let bounds = cell_fragments[0]
            .bounds
            .expect("table-cell fragment bounds");
        let text_y = match &page.items[item_index] {
            DisplayItem::Text(text) => text.origin.y.0,
            _ => unreachable!(),
        };
        assert!(
            bounds.origin.y.0 < text_y,
            "table-cell clip must start above its text baseline: bounds={bounds:?}, text_y={text_y}"
        );
    }
    let wrapped = text_origins_for(&document, 0, "This is an SdtCell.")
        .into_iter()
        .next()
        .expect("cell wrapped by w:sdt");
    let sibling = text_origins_for(&document, 0, "SdtRun")
        .into_iter()
        .last()
        .expect("ordinary sibling table cell");

    assert_close(
        wrapped.y.0,
        sibling.y.0,
        0.05,
        "cell-level sdt must not shift its table-cell baseline",
    );
}

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

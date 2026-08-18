use std::{
    io::{Cursor, Read},
    sync::Arc,
};

use olecfsdk::{
    ograph::{OgraphChartGroupKind, OgraphFile, OgraphRecordData},
    xls::{BiffRecordData, XlStringCharacters},
};

const OLE_DOCX: &str = "Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/\
TestDataStorage/v2FxTestFiles/wordprocessing/ole/ole.docx";

fn graph_object_bytes() -> Vec<u8> {
    let package = std::fs::read(olecfsdk_corpus_test_support::corpus_file_path(OLE_DOCX))
        .expect("read Open XML SDK OLE fixture");
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open DOCX package");
    let mut graph = Vec::new();
    archive
        .by_name("word/embeddings/oleObject1.bin")
        .expect("find embedded Microsoft Graph object")
        .read_to_end(&mut graph)
        .expect("read embedded Microsoft Graph object");
    graph
}

fn chart_bof_offset(file: &OgraphFile) -> u32 {
    file.workbook
        .records
        .iter()
        .filter_map(|record| match &record.data {
            OgraphRecordData::Common(BiffRecordData::Bof(_)) => Some(record.offset),
            _ => None,
        })
        .nth(1)
        .expect("chart-sheet BOF")
}

fn bound_sheet_offset(file: &OgraphFile) -> u32 {
    file.workbook
        .records
        .iter()
        .find_map(|record| match &record.data {
            OgraphRecordData::Common(BiffRecordData::BoundSheet8(sheet)) => {
                Some(sheet.sheet_bof_offset)
            }
            _ => None,
        })
        .expect("Graph BoundSheet8")
}

#[test]
fn microsoft_graph_8_root_round_trips_and_relayouts_the_embedded_chart() {
    let bytes = graph_object_bytes();
    let strict_error =
        OgraphFile::from_bytes(&bytes).expect_err("historical Label id is nonstrict");
    assert!(
        strict_error.to_string().contains("Label") && strict_error.to_string().contains("0x0004"),
        "unexpected strict diagnostic: {strict_error}"
    );

    let outcome = OgraphFile::from_bytes_compatible(&bytes).expect("open typed Graph 8 root");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.structure == "Label"
                && diagnostic.message.contains("0x0004")),
        "Microsoft's historical Label encoding must be explicit compatibility evidence"
    );
    assert!(
        !outcome
            .value
            .workbook
            .records
            .iter()
            .any(|record| matches!(&record.data, OgraphRecordData::Compatibility { .. })),
        "every record in the Microsoft-produced Graph fixture must have a typed implementation"
    );
    let labels = outcome
        .value
        .workbook
        .records
        .iter()
        .filter_map(|record| match &record.data {
            OgraphRecordData::Label(label) => Some(label.text.text()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "1st Qtr"), "{labels:?}");
    assert_eq!(
        outcome
            .value
            .workbook
            .records
            .iter()
            .filter(|record| matches!(
                &record.data,
                OgraphRecordData::Common(BiffRecordData::ChartSeries(_))
            ))
            .count(),
        4
    );
    assert!(!outcome.value.workbook.groups().unwrap().groups.is_empty());
    let chart = outcome
        .value
        .workbook
        .chart()
        .expect("resolve Graph datasheet, series, and chart-group relationships");
    assert_eq!(
        (
            chart.source.x,
            chart.source.y,
            chart.source.width,
            chart.source.height,
        ),
        (0, 0, 0x00d8_0000, 0x0090_0000),
        "MS-OGRAPH Chart dimensions are 16.16 fixed-point points"
    );
    assert_eq!(chart.width_points(), 216.0);
    assert_eq!(chart.height_points(), 144.0);
    assert_eq!(chart.groups.len(), 1);
    assert_eq!(chart.series.len(), 4);
    assert!(chart.series.iter().all(|series| series.included));
    let group = &chart.groups[0];
    assert!(matches!(group.kind, OgraphChartGroupKind::Bar(_)));
    let view_3d = group.view_3d.expect("Graph column chart 3-D view");
    assert_eq!(
        (
            view_3d.rotation,
            view_3d.elevation,
            view_3d.field_of_view,
            view_3d.depth_percent,
        ),
        (20, 15, 30, 100)
    );
    let legend = group.legend.as_ref().expect("Graph legend");
    assert_eq!(legend.source.spacing, 1);
    assert!(legend.source.flags.contains(
        olecfsdk::xls::ChartLegendFlags::AUTO_POSITION
            | olecfsdk::xls::ChartLegendFlags::AUTO_X
            | olecfsdk::xls::ChartLegendFlags::AUTO_Y
            | olecfsdk::xls::ChartLegendFlags::VERTICAL
    ));
    assert_eq!(
        (
            legend.position.source.top_left_mode,
            legend.position.source.bottom_right_mode,
            legend.position.x1,
            legend.position.y1,
            legend.position.x2,
            legend.position.y2,
        ),
        (5, 2, 3209, 1121, 0, 0),
        "Graph Pos semantics must use the signed low words, not the shared BIFF i32 values"
    );
    let legend_frame = legend.frame.as_ref().expect("Graph legend frame");
    assert_eq!(legend_frame.source.border_type, 0);
    assert_eq!(legend_frame.line.expect("legend line").color_index, 0x004d);
    assert_eq!(legend_frame.area.expect("legend area").fill_pattern, 0);
    assert_eq!(chart.axes.len(), 2);
    let category_axis = chart
        .axes
        .iter()
        .find(|axis| axis.source.axis_type == 0)
        .expect("category axis");
    assert_eq!(
        category_axis.line_formats[3]
            .expect("3-D wall line")
            .color_index,
        23
    );
    assert_eq!(
        category_axis.area_formats[3]
            .expect("3-D wall fill")
            .foreground_color_index,
        22
    );
    let value_axis = chart
        .axes
        .iter()
        .find(|axis| axis.source.axis_type == 1)
        .expect("value axis");
    assert!(
        value_axis.area_formats[3]
            .expect("3-D floor fill")
            .flags
            .contains(olecfsdk::xls::ChartAreaFlags::AUTO)
    );
    assert!(
        chart
            .series
            .iter()
            .flat_map(|series| &series.categories)
            .any(|category| matches!(
                category,
                Some(olecfsdk::ograph::OgraphCellValue::Text(value)) if value == "1st Qtr"
            )),
        "Graph BRAI/category resolution omitted the quarter labels: {:?}",
        chart.series
    );
    assert_eq!(
        bound_sheet_offset(&outcome.value),
        chart_bof_offset(&outcome.value)
    );

    let rebuilt = outcome
        .value
        .to_compound_file_preserving_compatibility()
        .expect("rebuild Graph CFB");
    assert!(
        outcome.value.source_compound_file().logical_eq(&rebuilt),
        "unmodified typed Graph root must preserve every CFB stream"
    );
    let saved = rebuilt.to_bytes().expect("serialize rebuilt Graph CFB");
    let reopened = OgraphFile::from_bytes_compatible(&saved).expect("reopen rebuilt Graph CFB");
    assert_eq!(outcome.value.workbook, reopened.value.workbook);
    assert_eq!(
        chart,
        reopened
            .value
            .workbook
            .chart()
            .expect("rebuild semantic Graph chart after root round trip")
    );

    let mut edited = reopened.value;
    let original_chart_bof = chart_bof_offset(&edited);
    let workbook = Arc::make_mut(&mut edited.workbook);
    let chart_bof_index = workbook
        .records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| match &record.data {
            OgraphRecordData::Common(BiffRecordData::Bof(_)) => Some(index),
            _ => None,
        })
        .nth(1)
        .unwrap();
    let font = workbook.records[..chart_bof_index]
        .iter_mut()
        .find_map(|record| match &mut record.data {
            OgraphRecordData::Common(BiffRecordData::Font(font)) => Some(font),
            _ => None,
        })
        .expect("Graph globals Font");
    match &mut font.name.characters {
        XlStringCharacters::Compressed(characters) => characters.extend_from_slice(b" SDK"),
        XlStringCharacters::Unicode(characters) => {
            characters.extend(" SDK".encode_utf16());
        }
    }
    edited
        .relayout_preserving_compatibility()
        .expect("transactionally relayout edited Graph root");
    assert!(chart_bof_offset(&edited) > original_chart_bof);
    assert_eq!(bound_sheet_offset(&edited), chart_bof_offset(&edited));

    let edited_bytes = edited
        .to_bytes_preserving_compatibility()
        .expect("serialize edited Graph root");
    let edited_reopened =
        OgraphFile::from_bytes_compatible(&edited_bytes).expect("reopen edited Graph root");
    assert_eq!(edited.workbook, edited_reopened.value.workbook);
    assert_eq!(
        edited_reopened
            .value
            .to_bytes_preserving_compatibility()
            .unwrap(),
        edited_bytes,
        "second Graph file-root save must be byte-stable"
    );
}

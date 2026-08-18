#[path = "common/xml.rs"]
mod xml;

use criterion::{Criterion, criterion_group, criterion_main};
use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main::Presentation;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::Worksheet;
use ooxmlsdk::schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::Document;
use xml::{
    PRESENTATION_PRESENTATION_XML, SPREADSHEET_WORKSHEET_NO_EXT_DATA_B1_SHEET1_XML,
    WORDPROCESSING_DOCUMENT_COMPLEX0_XML, WORDPROCESSING_DOCUMENT_HELLO_WORLD_XML,
    bench_xml_round_trip,
};

fn bench_xml(c: &mut Criterion) {
    bench_xml_round_trip::<Document>(
        c,
        "xml/word/document_hello_world",
        WORDPROCESSING_DOCUMENT_HELLO_WORLD_XML,
    );
    bench_xml_round_trip::<Document>(
        c,
        "xml/word/document_complex0",
        WORDPROCESSING_DOCUMENT_COMPLEX0_XML,
    );
    bench_xml_round_trip::<Worksheet>(
        c,
        "xml/sheet/worksheet_no_ext_data_b1_sheet1",
        SPREADSHEET_WORKSHEET_NO_EXT_DATA_B1_SHEET1_XML,
    );
    bench_xml_round_trip::<Presentation>(
        c,
        "xml/slides/presentation",
        PRESENTATION_PRESENTATION_XML,
    );
}

criterion_group!(benches, bench_xml);
criterion_main!(benches);

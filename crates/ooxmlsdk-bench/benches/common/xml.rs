use criterion::{BenchmarkId, Criterion, Throughput};
use ooxmlsdk::sdk::SdkType;
use std::hint::black_box;
use std::io::{BufReader, Cursor};
use std::time::Duration;

pub const WORDPROCESSING_DOCUMENT_HELLO_WORLD_XML: &str =
    include_str!("../../../../fixtures/perf/xml/wordprocessing_document_hello_world.xml");
pub const WORDPROCESSING_DOCUMENT_COMPLEX0_XML: &str =
    include_str!("../../../../fixtures/perf/xml/wordprocessing_document_complex0.xml");
pub const SPREADSHEET_WORKBOOK_XML: &str =
    include_str!("../../../../fixtures/perf/xml/spreadsheet_workbook.xml");
pub const PRESENTATION_PRESENTATION_XML: &str =
    include_str!("../../../../fixtures/perf/xml/presentation_presentation.xml");

pub fn bench_xml_round_trip<T>(c: &mut Criterion, group_name: &str, xml: &'static str)
where
    T: std::fmt::Display + std::str::FromStr + SdkType,
    T::Err: std::fmt::Debug,
{
    let parsed = xml.parse::<T>().unwrap();
    let mut group = c.benchmark_group(group_name);

    group.throughput(Throughput::Bytes(xml.len() as u64));
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    group.bench_with_input(BenchmarkId::new("read", "slice"), &xml, |b, xml| {
        b.iter(|| black_box(xml).parse::<T>().unwrap())
    });
    group.bench_with_input(BenchmarkId::new("read", "stream_cursor"), &xml, |b, xml| {
        b.iter(|| T::from_reader(Cursor::new(black_box(xml.as_bytes()))).unwrap())
    });
    group.bench_with_input(
        BenchmarkId::new("read", "stream_bufreader"),
        &xml,
        |b, xml| {
            b.iter(|| {
                T::from_reader(BufReader::new(Cursor::new(black_box(xml.as_bytes())))).unwrap()
            })
        },
    );
    group.bench_with_input(BenchmarkId::new("write", "parsed"), &parsed, |b, value| {
        b.iter(|| black_box(value).to_xml().unwrap())
    });
    group.bench_with_input(BenchmarkId::new("round_trip", "slice"), &xml, |b, xml| {
        b.iter(|| {
            let value = black_box(xml).parse::<T>().unwrap();
            let serialized = black_box(&value).to_xml().unwrap();
            black_box(serialized)
        })
    });

    group.finish();
}

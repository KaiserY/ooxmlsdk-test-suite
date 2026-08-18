use std::hint::black_box;
use std::io::Cursor;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ooxmlsdk_corpus_test_support::corpus_file_path;
use ooxmlsdk_pdf::{PdfOptions, convert_docx, convert_pptx, convert_xlsx};

const DOCUMENTS: &[(&str, &str)] = &[
    ("docx", "Apache-POI/test-data/document/55966.docx"),
    ("pptx", "LibreOffice/sd/qa/unit/data/pptx/tdf104015.pptx"),
    (
        "xlsx",
        "LibreOffice/chart2/qa/extras/chart2dump/data/tdf118150.xlsx",
    ),
];

fn bench_pdf_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/render");
    group.sample_size(20);
    for &(format, relative) in DOCUMENTS {
        let path = corpus_file_path(relative);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("read benchmark fixture {}: {error}", path.display()));
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(format), &bytes, |b, bytes| {
            b.iter(|| {
                let reader = Cursor::new(black_box(bytes.as_slice()));
                let options = PdfOptions {
                    ui_language: Some("zh-CN".to_string()),
                    ..PdfOptions::default()
                };
                match format {
                    "docx" => convert_docx(reader, options),
                    "pptx" => convert_pptx(reader, options),
                    "xlsx" => convert_xlsx(reader, options),
                    _ => unreachable!("benchmark format is fixed"),
                }
                .expect("render benchmark fixture")
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pdf_render);
criterion_main!(benches);

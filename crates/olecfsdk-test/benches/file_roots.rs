use std::{hint::black_box, io::Cursor, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use olecfsdk::{
    cfb::{CompoundFile, CompoundFileReader},
    doc::DocFile,
    ppt::PptFile,
    xls::XlsFile,
};
use olecfsdk_corpus_test_support::{corpus_bytes, corpus_file_path};

const LARGE_DOC: &str = "Apache-POI/test-data/document/Bug61268.doc";
const SMALL_XLS: &str = "Apache-POI/test-data/spreadsheet/Simple.xls";
const LARGE_XLS: &str = "Apache-POI/test-data/spreadsheet/Basic_Expense_Template_2011.xls";
const SMALL_PPT: &str = "Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt";
const LARGE_PPT: &str = "Apache-POI/test-data/slideshow/customGeo.ppt";

fn fixture(relative: &str) -> Vec<u8> {
    let path = corpus_file_path(relative);
    corpus_bytes(&path)
        .unwrap_or_else(|error| panic!("read benchmark fixture {}: {error}", path.display()))
}

fn configure(group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>) {
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));
}

fn bench_cfb(c: &mut Criterion) {
    let bytes = fixture(LARGE_DOC);
    let parsed = CompoundFile::from_bytes(&bytes).expect("open large CFB benchmark fixture");
    let saved = parsed.to_bytes().expect("save large CFB benchmark fixture");
    CompoundFile::from_bytes_strict(&saved).expect("strictly reopen large CFB benchmark fixture");

    let mut group = c.benchmark_group("cfb/large_doc");
    configure(&mut group);
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_with_input(BenchmarkId::new("open", "owned"), &bytes, |b, bytes| {
        b.iter(|| CompoundFile::from_bytes(black_box(bytes.as_slice())).unwrap())
    });
    group.bench_with_input(
        BenchmarkId::new("open", "streaming_metadata"),
        &bytes,
        |b, bytes| {
            b.iter(|| {
                CompoundFileReader::from_reader(Cursor::new(black_box(bytes.as_slice()))).unwrap()
            })
        },
    );
    group.bench_function(BenchmarkId::new("save", "owned"), |b| {
        b.iter(|| black_box(&parsed).to_bytes().unwrap())
    });
    group.bench_function(BenchmarkId::new("clone", "owned"), |b| {
        b.iter(|| black_box(black_box(&parsed).clone()))
    });
    group.finish();
}

fn bench_doc(c: &mut Criterion) {
    let bytes = fixture(LARGE_DOC);
    let parsed = DocFile::from_bytes(&bytes).expect("open large DOC benchmark fixture");
    let saved = parsed.to_bytes().expect("save large DOC benchmark fixture");
    DocFile::from_bytes(&saved).expect("strictly reopen large DOC benchmark fixture");

    let mut group = c.benchmark_group("doc/large_strict");
    configure(&mut group);
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("open", "from_bytes"),
        &bytes,
        |b, bytes| b.iter(|| DocFile::from_bytes(black_box(bytes.as_slice())).unwrap()),
    );
    group.bench_function(BenchmarkId::new("relationships", "content_tree"), |b| {
        b.iter(|| black_box(&parsed).content_tree().unwrap())
    });
    group.bench_function(BenchmarkId::new("save", "to_bytes"), |b| {
        b.iter(|| black_box(&parsed).to_bytes().unwrap())
    });
    group.bench_function(BenchmarkId::new("clone", "file_root"), |b| {
        b.iter(|| black_box(black_box(&parsed).clone()))
    });
    group.finish();
}

fn bench_xls(c: &mut Criterion) {
    let strict_bytes = fixture(SMALL_XLS);
    let strict = XlsFile::from_bytes(&strict_bytes).expect("open strict XLS benchmark fixture");
    let strict_saved = strict
        .to_bytes()
        .expect("save strict XLS benchmark fixture");
    XlsFile::from_bytes(&strict_saved).expect("strictly reopen XLS benchmark fixture");

    let compatible_bytes = fixture(LARGE_XLS);
    let compatible = XlsFile::from_bytes_compatible(&compatible_bytes)
        .expect("open compatible XLS benchmark fixture")
        .value;
    let compatible_saved = compatible
        .to_bytes_preserving_compatibility()
        .expect("save compatible XLS benchmark fixture");
    XlsFile::from_bytes_compatible(&compatible_saved)
        .expect("compatibly reopen XLS benchmark fixture");

    let mut strict_group = c.benchmark_group("xls/small_strict");
    configure(&mut strict_group);
    strict_group.throughput(Throughput::Bytes(strict_bytes.len() as u64));
    strict_group.bench_with_input(
        BenchmarkId::new("open", "from_bytes"),
        &strict_bytes,
        |b, bytes| b.iter(|| XlsFile::from_bytes(black_box(bytes.as_slice())).unwrap()),
    );
    strict_group.bench_function(BenchmarkId::new("relationships", "workbook"), |b| {
        b.iter(|| black_box(&strict).workbooks[0].relationships().unwrap())
    });
    strict_group.bench_function(BenchmarkId::new("save", "to_bytes"), |b| {
        b.iter(|| black_box(&strict).to_bytes().unwrap())
    });
    strict_group.finish();

    let mut compatible_group = c.benchmark_group("xls/large_compatible");
    configure(&mut compatible_group);
    compatible_group.throughput(Throughput::Bytes(compatible_bytes.len() as u64));
    compatible_group.bench_with_input(
        BenchmarkId::new("open", "from_bytes"),
        &compatible_bytes,
        |b, bytes| {
            b.iter(|| {
                XlsFile::from_bytes_compatible(black_box(bytes.as_slice()))
                    .unwrap()
                    .value
            })
        },
    );
    compatible_group.bench_function(BenchmarkId::new("save", "to_bytes"), |b| {
        b.iter(|| {
            black_box(&compatible)
                .to_bytes_preserving_compatibility()
                .unwrap()
        })
    });
    compatible_group.bench_function(BenchmarkId::new("clone", "file_root"), |b| {
        b.iter(|| black_box(black_box(&compatible).clone()))
    });
    compatible_group.finish();
}

fn bench_ppt(c: &mut Criterion) {
    let small_bytes = fixture(SMALL_PPT);
    let small = PptFile::from_bytes(&small_bytes).expect("open small PPT benchmark fixture");
    let small_saved = small.to_bytes().expect("save small PPT benchmark fixture");
    PptFile::from_bytes(&small_saved).expect("strictly reopen small PPT benchmark fixture");

    let large_bytes = fixture(LARGE_PPT);
    let large = PptFile::from_bytes(&large_bytes).expect("open large PPT benchmark fixture");
    let large_saved = large.to_bytes().expect("save large PPT benchmark fixture");
    PptFile::from_bytes(&large_saved).expect("strictly reopen large PPT benchmark fixture");

    let mut small_group = c.benchmark_group("ppt/small_strict");
    configure(&mut small_group);
    small_group.throughput(Throughput::Bytes(small_bytes.len() as u64));
    small_group.bench_with_input(
        BenchmarkId::new("open", "from_bytes"),
        &small_bytes,
        |b, bytes| b.iter(|| PptFile::from_bytes(black_box(bytes.as_slice())).unwrap()),
    );
    small_group.bench_function(BenchmarkId::new("relationships", "live"), |b| {
        b.iter(|| black_box(&small).live_presentation().unwrap())
    });
    small_group.bench_function(BenchmarkId::new("save", "to_bytes"), |b| {
        b.iter(|| black_box(&small).to_bytes().unwrap())
    });
    small_group.finish();

    let mut large_group = c.benchmark_group("ppt/large_strict");
    configure(&mut large_group);
    large_group.throughput(Throughput::Bytes(large_bytes.len() as u64));
    large_group.bench_with_input(
        BenchmarkId::new("open", "from_bytes"),
        &large_bytes,
        |b, bytes| b.iter(|| PptFile::from_bytes(black_box(bytes.as_slice())).unwrap()),
    );
    large_group.bench_function(BenchmarkId::new("save", "to_bytes"), |b| {
        b.iter(|| black_box(&large).to_bytes().unwrap())
    });
    large_group.bench_function(BenchmarkId::new("clone", "file_root"), |b| {
        b.iter(|| black_box(black_box(&large).clone()))
    });
    large_group.finish();
}

criterion_group!(benches, bench_cfb, bench_doc, bench_xls, bench_ppt);
criterion_main!(benches);

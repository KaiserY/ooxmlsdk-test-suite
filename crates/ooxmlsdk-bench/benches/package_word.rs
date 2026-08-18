#[path = "common/package.rs"]
mod package;

use criterion::{Criterion, criterion_group, criterion_main};
use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;
use package::{bench_package_round_trip, open_xml_sdk_corpus_file};

fn bench_packages(c: &mut Criterion) {
    bench_package_round_trip::<WordprocessingDocument>(
        c,
        "package/word/hello_world",
        open_xml_sdk_corpus_file(
            "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/HelloWorld.docx",
        ),
    );
    bench_package_round_trip::<WordprocessingDocument>(
        c,
        "package/word/comments",
        open_xml_sdk_corpus_file(
            "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/Comments.docx",
        ),
    );
    bench_package_round_trip::<WordprocessingDocument>(
        c,
        "package/word/complex0_upstream_benchmark",
        open_xml_sdk_corpus_file(
            "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/complex0.docx",
        ),
    );
}

criterion_group!(benches, bench_packages);
criterion_main!(benches);

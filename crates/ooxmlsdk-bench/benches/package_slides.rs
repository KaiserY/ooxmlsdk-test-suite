#[path = "common/package.rs"]
mod package;

use criterion::{Criterion, criterion_group, criterion_main};
use ooxmlsdk::parts::presentation_document::PresentationDocument;
use package::{bench_package_round_trip, open_xml_sdk_corpus_file};

fn bench_packages(c: &mut Criterion) {
    bench_package_round_trip::<PresentationDocument>(
        c,
        "package/slides/basic",
        open_xml_sdk_corpus_file(
            "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/Presentation.pptx",
        ),
    );
    bench_package_round_trip::<PresentationDocument>(
        c,
        "package/slides/performance_typical",
        open_xml_sdk_corpus_file(
            "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/o09_Performance_typical.pptx",
        ),
    );
}

criterion_group!(benches, bench_packages);
criterion_main!(benches);

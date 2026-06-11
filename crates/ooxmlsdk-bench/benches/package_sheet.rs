#[path = "common/package.rs"]
mod package;

use package::{bench_package_round_trip, open_xml_sdk_corpus_file};
use criterion::{criterion_group, criterion_main, Criterion};
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;

fn bench_packages(c: &mut Criterion) {
  bench_package_round_trip::<SpreadsheetDocument>(
    c,
    "package/sheet/basic",
    open_xml_sdk_corpus_file(
      "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/Spreadsheet.xlsx",
    ),
  );
  bench_package_round_trip::<SpreadsheetDocument>(
    c,
    "package/sheet/complex01",
    open_xml_sdk_corpus_file(
      "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/Complex01.xlsx",
    ),
  );
  bench_package_round_trip::<SpreadsheetDocument>(
    c,
    "package/sheet/performance_eng",
    open_xml_sdk_corpus_file(
      "test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/spreadsheet/PerformanceEng.xlsx",
    ),
  );
}

criterion_group!(benches, bench_packages);
criterion_main!(benches);

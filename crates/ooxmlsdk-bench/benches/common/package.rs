use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn open_xml_sdk_corpus_file(path: &str) -> PathBuf {
  ooxmlsdk_corpus_test_support::corpus_file_path(
    Path::new("Open-XML-SDK").join(path).to_str().unwrap(),
  )
}

pub fn bench_package_round_trip<T>(c: &mut Criterion, group_name: &str, path: PathBuf)
where
  T: PackageRoundTrip,
{
  let bytes = std::fs::read(&path)
    .unwrap_or_else(|err| panic!("read benchmark package fixture {}: {err}", path.display()));
  let parsed = T::open(bytes.as_slice()).unwrap_or_else(|err| {
    panic!("parse benchmark package fixture {}: {err:?}", path.display())
  });
  let mut group = c.benchmark_group(group_name);

  group.throughput(Throughput::Bytes(bytes.len() as u64));
  group.sample_size(30);
  group.measurement_time(Duration::from_secs(10));

  group.bench_with_input(BenchmarkId::new("read", "cursor"), &bytes, |b, bytes| {
    b.iter(|| T::open(black_box(bytes.as_slice())).unwrap())
  });

  group.bench_with_input(BenchmarkId::new("write", "parsed"), &parsed, |b, value| {
    b.iter_batched_ref(
      || Cursor::new(Vec::with_capacity(bytes.len())),
      |output| {
        output.get_mut().clear();
        output.set_position(0);
        value.save_to(output).unwrap();
        black_box(output.get_ref().len())
      },
      BatchSize::SmallInput,
    )
  });

  group.bench_with_input(
    BenchmarkId::new("round_trip", "cursor"),
    &bytes,
    |b, bytes| {
      b.iter_batched_ref(
        || Cursor::new(Vec::with_capacity(bytes.len())),
        |output| {
          output.get_mut().clear();
          output.set_position(0);
          let parsed = T::open(black_box(bytes.as_slice())).unwrap();
          parsed.save_to(output).unwrap();
          let round_tripped = T::open(output.get_ref().as_slice()).unwrap();
          black_box(round_tripped)
        },
        BatchSize::SmallInput,
      )
    },
  );

  group.finish();
}

pub trait PackageRoundTrip: Sized {
  fn open(bytes: &[u8]) -> Result<Self, Box<dyn std::fmt::Debug>>;
  fn save_to(&self, output: &mut Cursor<Vec<u8>>) -> Result<(), Box<dyn std::fmt::Debug>>;
}

macro_rules! impl_package_round_trip {
  ($ty:ty) => {
    impl PackageRoundTrip for $ty {
      fn open(bytes: &[u8]) -> Result<Self, Box<dyn std::fmt::Debug>> {
        <$ty>::new(Cursor::new(bytes)).map_err(|err| Box::new(err) as Box<dyn std::fmt::Debug>)
      }

      fn save_to(&self, output: &mut Cursor<Vec<u8>>) -> Result<(), Box<dyn std::fmt::Debug>> {
        self.save(output)
          .map_err(|err| Box::new(err) as Box<dyn std::fmt::Debug>)
      }
    }
  };
}

impl_package_round_trip!(ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument);
impl_package_round_trip!(ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument);
impl_package_round_trip!(ooxmlsdk::parts::presentation_document::PresentationDocument);

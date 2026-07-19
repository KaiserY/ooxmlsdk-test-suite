use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ooxmlsdk_formula::{SheetId, parse_formula_text};

const FORMULAS: &[(&str, &str)] = &[
    ("arithmetic", "=SUM(A1:A100)+IF(B2>0,B2*1.2,0)"),
    (
        "lookup",
        "=INDEX('Sales Data'!$B$2:$M$500,MATCH($A2,'Sales Data'!$A$2:$A$500,0),MATCH(B$1,'Sales Data'!$B$1:$M$1,0))",
    ),
    (
        "dynamic_array",
        "=LET(filtered,FILTER(Table1[Amount],(Table1[Region]=$A$2)*(Table1[Date]>=B$1)),SORT(UNIQUE(filtered)))",
    ),
];

fn bench_formula_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula/parse");
    for &(name, formula) in FORMULAS {
        group.throughput(Throughput::Bytes(formula.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), formula, |b, formula| {
            b.iter(|| parse_formula_text(SheetId(0), black_box(formula)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_formula_parse);
criterion_main!(benches);

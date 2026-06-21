# Benchmarks

The benchmark harness lives in `crates/ooxmlsdk-bench` and uses Criterion. It is
separate from corpus round-trip tests and split by document family so focused
runs do not require the full package/XML suite.

## Running

Run only the area being investigated:

```bash
cargo bench -p ooxmlsdk-bench --bench package_word
cargo bench -p ooxmlsdk-bench --bench package_sheet
cargo bench -p ooxmlsdk-bench --bench package_slides
cargo bench -p ooxmlsdk-bench --bench xml
```

Criterion output is written under `target/criterion/`.

The bench profile keeps line-table debug information for `cargo-flamegraph` and
`perf` without enabling LTO. Full LTO was tested and rejected because the final
`xml` bench binary link/codegen for generated `ooxmlsdk` schemas used about 14
GiB RSS and did not complete promptly.

```toml
[profile.bench]
debug = "line-tables-only"
```

## Flamegraph

For profiling a focused Criterion benchmark, build the bench binary first and
record the bench executable directly. Prefer the `perf` + Inferno flow below
over `cargo flamegraph`: recording the bench binary directly avoids sampling
Cargo and reduces Criterion helper noise. Use frame pointers and disable perf
child process inheritance so report helpers such as `gnuplot` are not sampled.

```bash
RUSTFLAGS="-C force-frame-pointers=yes" \
  cargo bench -p ooxmlsdk-bench --bench xml --no-run

BIN=$(find target/release/deps -maxdepth 1 -type f -executable -name 'xml-*' | sort | tail -n 1)

perf record \
  --no-inherit \
  --delay 3000 \
  -F 997 \
  -g \
  --call-graph fp \
  -o perf-xml-word-complex0-read-fp.data \
  -- "$BIN" \
  --bench \
  --profile-time 60 \
  --noplot \
  xml/word/document_complex0/read/slice

perf script -i perf-xml-word-complex0-read-fp.data \
  | inferno-collapse-perf \
  > folded-xml-word-complex0-read-fp.txt

inferno-flamegraph \
  --title "xml word complex0 read slice fp" \
  folded-xml-word-complex0-read-fp.txt \
  > flamegraph-xml-word-complex0-read-fp.svg
```

Generate folded stacks when comparing hotspots. The SVG is useful for browsing,
but inclusive frames near the root can hide the actionable self-cost. Start with
leaf cost and then inspect inclusive stacks for the selected function.

```bash
python3 - <<'PY'
from collections import Counter
from pathlib import Path

folded = Path('folded-xml-word-complex0-read-fp.txt')
inclusive = Counter()
leaf = Counter()
total = 0

for line in folded.read_text(errors='replace').splitlines():
    if not line:
        continue
    stack, count = line.rsplit(' ', 1)
    count = int(count)
    frames = stack.split(';')
    total += count
    leaf[frames[-1]] += count
    for frame in set(frames):
        inclusive[frame] += count

print('leaf')
for frame, count in leaf.most_common(40):
    print(f'{count / total * 100:6.2f}% {frame}')

print('inclusive')
for frame, count in inclusive.most_common(40):
    print(f'{count / total * 100:6.2f}% {frame}')
PY
```

Notes:

- Inferno's general Linux workflow uses `perf record --call-graph dwarf`, then
  `perf script | inferno-collapse-perf | inferno-flamegraph`. In this benchmark,
  prefer `--call-graph fp`; `--call-graph dwarf` produced cleaner process
  selection but lost useful Rust caller stacks.
- Pass the folded stack file path to `inferno-flamegraph` when generating the
  final SVG. This avoids depending on stdin behavior across installed Inferno
  versions.
- Keep `--no-inherit`; without it, `gnuplot` can dominate the profile even when
  the bench executable is invoked directly.
- Keep `--delay`; it skips process startup and dynamic linker samples.
- Treat the SVG root and Criterion frames as navigation context, not as the
  optimization target. Use folded-stack leaf cost to decide what to optimize.
- If the resulting SVG does not contain `read_inner`, `quick_xml`, and
  `ooxmlsdk::common::xml` frames, rebuild the bench with the `RUSTFLAGS` above
  before recording again.

## Coverage

Package benchmarks run these operations for each fixture:

- `read`: open package from an in-memory cursor.
- `write`: save an already parsed package to an in-memory cursor.
- `round_trip`: open, save, and reopen the saved package.

Typed XML benchmarks run these operations for typed root elements:

- `read/slice`: parse from `&str`.
- `read/stream_cursor`: parse from `Cursor<&[u8]>`.
- `read/stream_bufreader`: parse from `BufReader<Cursor<&[u8]>>`.
- `write/parsed`: serialize an already parsed value.
- `round_trip/slice`: parse and serialize.


Package fixtures come from the checked-in Open XML SDK corpus under
`corpus/Open-XML-SDK/`. `complex0.docx` is included because upstream Open XML SDK
uses `TestAssets.TestFiles.Complex0docx` in its BenchmarkDotNet package and
validation benchmarks.

XML fixtures live under `fixtures/perf/xml/`. Small XML fixtures are copied from
local `ooxmlsdk` regression samples. `wordprocessing_document_complex0.xml` is
extracted from `word/document.xml` in `complex0.docx`.

## Run history

### 2026-06-21 XML run after WML table stack changes

Command:

```bash
cargo bench -p ooxmlsdk-bench --bench xml
```

XML results:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.5914 us | 4.1367 us | 4.0616 us | 777.22 ns | 4.4063 us |
| `xml/word/document_complex0` | 1.4696 ms | 1.7786 ms | 1.7799 ms | 193.89 us | 1.7311 ms |
| `xml/sheet/workbook` | 1.4782 us | 1.7703 us | 1.7888 us | 424.11 ns | 2.0399 us |
| `xml/slides/presentation` | 9.7968 us | 11.622 us | 11.593 us | 1.9154 us | 11.525 us |

Focused profile:

```bash
RUSTFLAGS="-C force-frame-pointers=yes" \
  cargo bench -p ooxmlsdk-bench --bench xml --no-run

BIN=$(find target/release/deps -maxdepth 1 -type f -executable -name 'xml-*' | sort | tail -n 1)

perf record \
  --no-inherit \
  --delay 3000 \
  -F 997 \
  -g \
  --call-graph fp \
  -o perf-xml-word-complex0-read-fp.data \
  -- "$BIN" \
  --bench \
  --profile-time 45 \
  --noplot \
  xml/word/document_complex0/read/slice
```

The focused run captured 42,072 samples. `inferno-collapse-perf` produced 5,850
folded stacks, and the SVG output was 1.2 MiB.

Hotspot summary for `xml/word/document_complex0/read/slice`:

| Area | Evidence |
| --- | --- |
| Generated parse dispatch | `read_inner<ooxmlsdk::common::xml::SliceReader>` was the largest leaf at 13.06% and 87.07% inclusive. |
| WML table parse path | Inclusive table frames were high because the fixture is table-heavy: `Table` 67.15%, `TableRow` 66.44%, `TableCell` 63.48%. |
| XML scanner | `next_tag_event` was 26.93% inclusive, `quick_xml::Reader::read_event_impl` was 23.83% inclusive and 8.16% self in `perf report`. |
| Byte search | `find_avx2`, `memchr3_raw`, `memchr2`, and related search frames accounted for a visible scanner share. |
| Allocation/drop | Allocator/free leaf frames and generated DOM drops remain visible; each read iteration parses and then drops the resulting typed DOM. |
| Recursive table stack path | `__ooxmlsdk_read_inner_stack*` did not show as a hotspot in this fixture; the measured path is dominated by ordinary generated `read_inner` and XML scanning. |

### 2026-06-11 XML run with profiler symbols

Command:

```bash
cargo bench -p ooxmlsdk-bench --bench xml
```

XML results:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.6074 us | 4.0765 us | 4.2495 us | 789.57 ns | 4.5367 us |
| `xml/word/document_complex0` | 1.5727 ms | 1.9167 ms | 1.9543 ms | 289.73 us | 1.9448 ms |
| `xml/sheet/workbook` | 1.4797 us | 1.8234 us | 1.8988 us | 481.59 ns | 2.0975 us |
| `xml/slides/presentation` | 9.6096 us | 12.419 us | 12.893 us | 3.1002 us | 13.363 us |

### 2026-06-11 XML split run

Command:

```bash
cargo bench -p ooxmlsdk-bench --bench xml
```

XML results:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.6753 us | 4.2857 us | 4.3978 us | 755.92 ns | 4.7228 us |
| `xml/word/document_complex0` | 1.6037 ms | 1.9253 ms | 1.9827 ms | 289.57 us | 2.0195 ms |
| `xml/sheet/workbook` | 1.5585 us | 1.8854 us | 1.9145 us | 467.43 ns | 2.1756 us |
| `xml/slides/presentation` | 9.8961 us | 12.363 us | 12.507 us | 2.9631 us | 13.352 us |

### 2026-06-11 full run before target split

Command used before the original full `perf` target was split:

```bash
cargo bench -p ooxmlsdk-bench --bench perf
```

Package results:

| Benchmark | Read | Write | Round-trip |
| --- | ---: | ---: | ---: |
| `package/word/hello_world` | 325.48 us | 541.45 us | 1.3688 ms |
| `package/word/comments` | 305.06 us | 510.33 us | 1.2407 ms |
| `package/word/complex0_upstream_benchmark` | 3.0494 ms | 6.7030 ms | 12.811 ms |
| `package/sheet/basic` | 969.33 us | 1.4388 ms | 3.5541 ms |
| `package/sheet/complex01` | 937.60 us | 2.2658 ms | 4.5012 ms |
| `package/sheet/performance_eng` | 16.861 ms | 17.537 ms | 50.283 ms |
| `package/slides/basic` | 3.9674 ms | 17.513 ms | 25.298 ms |
| `package/slides/performance_typical` | 2.8674 ms | 22.711 ms | 28.597 ms |

XML results:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.4799 us | 4.2921 us | 4.3082 us | 823.37 ns | 4.7346 us |
| `xml/word/document_complex0` | 1.5756 ms | 1.9286 ms | 1.9396 ms | 314.39 us | 1.9490 ms |
| `xml/sheet/workbook` | 1.5363 us | 1.7929 us | 1.8738 us | 498.11 ns | 2.0870 us |
| `xml/slides/presentation` | 9.3485 us | 11.600 us | 11.960 us | 3.0586 us | 12.940 us |

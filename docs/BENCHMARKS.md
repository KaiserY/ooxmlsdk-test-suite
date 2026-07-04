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
`spreadsheet_worksheet_no_ext_data_b1_sheet1.xml` is extracted from
`xl/worksheets/sheet1.xml` in Open XML SDK's `NoExtDataB1.xlsx`; it replaced the
old tiny `spreadsheet_workbook.xml` benchmark case because the workbook part did
not represent numeric/float-heavy spreadsheet XML.

## Run history

### 2026-07-04 XML text event fast path

Command:

```bash
cargo bench -p ooxmlsdk-bench --bench xml
```

Change under test:

- Add a quick-xml serde-style `DeEvent` layer for element body reads.
- Return `DeEvent::FastBytesText` for isolated `Text` events so generated
  `#[sdk(text)]` and `text_child` paths can parse or copy raw text bytes before
  falling back to decoded text.
- Keep consecutive `Text` / `CData` / `GeneralRef` handling on the decoded
  `DeEvent::Text` path.
- Do not change MCE XML replacement logic.

Absolute medians from the run:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.5369 us | 4.0694 us | 4.0557 us | 741.31 ns | 4.2564 us |
| `xml/word/document_complex0` | 1.4557 ms | 1.7327 ms | 1.7691 ms | 156.16 us | 1.6556 ms |
| `xml/sheet/worksheet_no_ext_data_b1_sheet1` | 6.1210 ms | 7.5950 ms | 7.7558 ms | 549.86 us | 6.6875 ms |
| `xml/slides/presentation` | 9.1188 us | 10.971 us | 12.103 us | 1.4732 us | 11.073 us |

Compared with the previous documented valid run after numeric write fast paths:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | improved (-4.1%) | improved (-0.5%) | improved (-4.0%) | improved (-2.1%) | improved (-7.9%) |
| `xml/word/document_complex0` | improved (-4.8%) | improved (-3.7%) | improved (-3.2%) | no change (-0.4%) | improved (-5.9%) |
| `xml/sheet/worksheet_no_ext_data_b1_sheet1` | improved (-6.4%) | improved (-5.7%) | improved (-5.0%) | improved (-3.2%) | improved (-4.9%) |
| `xml/slides/presentation` | improved (-2.8%) | improved (-4.9%) | noise (+3.6%) | no change (-0.5%) | improved (-2.4%) |

Conclusion: keep the text event fast path. It targets generated text fields
directly, especially simple spreadsheet cell value bodies such as `<v>...</v>`,
while leaving MCE handling alone. The saved Criterion change report from this
run compared against a discarded run that was affected by unrelated system
load, so the table above compares absolute medians with the previous documented
valid run instead.

### 2026-07-04 XML numeric write fast paths

Command:

```bash
cargo bench -p ooxmlsdk-bench --bench xml
```

Fixture update:

- Retired `xml/sheet/workbook` from the active XML bench set. That fixture was
  only 810 bytes and had too few numeric attributes to represent spreadsheet XML
  performance.
- Added `xml/sheet/worksheet_no_ext_data_b1_sheet1`, parsed as
  `schemas_openxmlformats_org_spreadsheetml_2006_main::Worksheet`.
- Source:
  `corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/v2FxTestFiles/spreadsheet/NoExtDataB1.xlsx!xl/worksheets/sheet1.xml`.
- Extracted XML size: 1,009,993 bytes. Rough scan before adding the fixture:
  23,888 numeric attributes, 21 float-like attributes, 18,748 numeric text
  nodes, and 17,491 float-like text nodes.

Fresh baseline after replacing the spreadsheet fixture and clearing
`target/criterion/xml`:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.6018 us | 4.1754 us | 4.1672 us | 798.17 ns | 4.5607 us |
| `xml/word/document_complex0` | 1.5709 ms | 1.8600 ms | 1.8546 ms | 186.07 us | 1.8336 ms |
| `xml/sheet/worksheet_no_ext_data_b1_sheet1` | 6.6547 ms | 8.3342 ms | 8.2693 ms | 856.14 us | 7.5472 ms |
| `xml/slides/presentation` | 9.5975 us | 11.739 us | 12.225 us | 2.0131 us | 11.940 us |

Change under test:

- Add `itoa` for generated integer attribute serialization.
- Add `zmij` for generated finite `f32`/`f64` attribute serialization.
- Keep existing OOXML float special-case output for `NaN`, `INF`, and `-INF`.
- Keep read-side scalar parsing changes from the previous XML attribute parse
  work; this run measures the incremental write-side change against the fresh
  worksheet baseline.

After integer/float write fast paths:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.6871 us | 4.0880 us | 4.2263 us | 757.17 ns | 4.6190 us |
| `xml/word/document_complex0` | 1.5289 ms | 1.7996 ms | 1.8285 ms | 156.79 us | 1.7588 ms |
| `xml/sheet/worksheet_no_ext_data_b1_sheet1` | 6.5408 ms | 8.0564 ms | 8.1612 ms | 568.02 us | 7.0343 ms |
| `xml/slides/presentation` | 9.3837 us | 11.537 us | 11.686 us | 1.4811 us | 11.349 us |

Criterion change summary for the post-change run:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | regressed (+2.94%) | improved (-1.69%) | noise (+0.75%) | improved (-5.42%) | noise (+1.04%) |
| `xml/word/document_complex0` | no change (-1.29%) | improved (-2.99%) | no change (-0.47%) | improved (-15.68%) | improved (-3.55%) |
| `xml/sheet/worksheet_no_ext_data_b1_sheet1` | improved (-1.72%) | improved (-4.18%) | no change (-0.81%) | improved (-33.54%) | improved (-6.57%) |
| `xml/slides/presentation` | improved (-2.10%) | improved (-2.82%) | improved (-4.14%) | improved (-26.22%) | improved (-4.51%) |

Conclusion: keep the integer/float write fast paths. The new worksheet fixture
hits the numeric spreadsheet path directly, and the write-side gain is large
enough to be above ordinary Criterion noise. The read-side deltas in this run
are secondary and mostly noise or indirect effects; the intended win is
serialization.

### 2026-07-01 XML lexical integer/float parser experiment

Note: this run used the retired `xml/sheet/workbook` fixture. Its spreadsheet
numbers are not comparable with the current worksheet benchmark set.

Command:

```bash
cargo bench -p ooxmlsdk-bench --bench xml
```

Change under test:

- `lexical-parse-integer` for generated integer attribute and text-child parse
  paths.
- `lexical-parse-float` for generated XML Schema `float`/`double` attribute and
  text-child parse paths, preserving `NaN`, `INF`, and `-INF`.
- The XML scanner stayed on `quick-xml`; this only changed typed scalar parsing
  after XML tokenization.

Fresh baseline before the lexical change:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.6143 us | 4.1529 us | 4.1058 us | 807.22 ns | 4.5441 us |
| `xml/word/document_complex0` | 1.5051 ms | 1.8026 ms | 1.8194 ms | 182.45 us | 1.7472 ms |
| `xml/sheet/workbook` | 1.3947 us | 1.7448 us | 1.8261 us | 436.05 ns | 1.9728 us |
| `xml/slides/presentation` | 9.1260 us | 11.241 us | 11.318 us | 2.0307 us | 11.670 us |

After lexical integer/float parse integration:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | 3.6015 us | 4.1143 us | 4.3083 us | 833.79 ns | 4.6060 us |
| `xml/word/document_complex0` | 1.5241 ms | 1.8127 ms | 1.8288 ms | 188.53 us | 1.7296 ms |
| `xml/sheet/workbook` | 1.4538 us | 1.7455 us | 1.8089 us | 419.40 ns | 2.0030 us |
| `xml/slides/presentation` | 9.4291 us | 11.291 us | 11.634 us | 1.9946 us | 12.206 us |

Criterion change summary for the post-change run:

| Benchmark | Read slice | Read cursor | Read bufreader | Write | Round-trip |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xml/word/document_hello_world` | no change (-0.35%) | no change (-0.54%) | regressed (+3.42%) | regressed (+3.55%) | noise (+0.64%) |
| `xml/word/document_complex0` | no change (+0.41%) | no change (+0.17%) | no change (-0.20%) | regressed (+2.13%) | no change (-0.45%) |
| `xml/sheet/workbook` | regressed (+3.46%) | noise (+0.57%) | noise (-1.50%) | improved (-4.16%) | noise (+0.64%) |
| `xml/slides/presentation` | regressed (+4.02%) | noise (+0.43%) | regressed (+2.96%) | improved (-2.78%) | regressed (+5.55%) |

Conclusion: the broad lexical parser integration did not produce a reliable XML
read-path speedup. The changes are mostly neutral on the large Word read case,
but several small/read-heavy cases regressed, and the write changes are mixed
even though the experiment targeted read-side scalar parsing. Do not keep this
change on performance grounds without a narrower follow-up that demonstrates a
clear win.

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

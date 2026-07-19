use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use olecfsdk::{
    cfb::CompoundFile,
    doc::DocFile,
    ppt::{BinaryTagData, PowerPointDocument, PptFile, PptRecordData, PptRecordSequence},
    xls::{BiffRecordData, XlsFile},
};
use olecfsdk_corpus_test_support::{corpus_bytes, corpus_file_path};
use serde::Serialize;

const LARGE_DOC: &str = "Apache-POI/test-data/document/Bug61268.doc";
const LARGE_XLS: &str = "Apache-POI/test-data/spreadsheet/Basic_Expense_Template_2011.xls";
const LARGE_PPT: &str = "Apache-POI/test-data/slideshow/customGeo.ppt";

static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static REALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);

struct ProfileAllocator;

#[global_allocator]
static ALLOCATOR: ProfileAllocator = ProfileAllocator;

unsafe impl GlobalAlloc for ProfileAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the layout is forwarded unchanged to the system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the layout is forwarded unchanged to the system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: the pointer and layout originate from the system allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the pointer and layout originate from the system allocator and the new size is
        // passed through unchanged.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() && ENABLED.load(Ordering::Relaxed) {
            REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            if new_size >= layout.size() {
                record_growth(new_size - layout.size());
            } else {
                record_shrink(layout.size() - new_size);
            }
        }
        resized
    }
}

fn record_allocation(size: usize) {
    if ENABLED.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        record_growth(size);
    }
}

fn record_deallocation(size: usize) {
    if ENABLED.load(Ordering::Relaxed) {
        record_shrink(size);
    }
}

fn record_growth(size: usize) {
    let size = size as u64;
    ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
    let live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
}

fn record_shrink(size: usize) {
    let size = size as u64;
    DEALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
    let _ = LIVE_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |live| {
        Some(live.saturating_sub(size))
    });
}

#[derive(Debug, Serialize)]
struct AllocationSample {
    case: &'static str,
    input_bytes: usize,
    allocations: u64,
    reallocations: u64,
    allocated_bytes: u64,
    deallocated_bytes: u64,
    peak_live_bytes: u64,
}

fn measure<T>(
    case: &'static str,
    input_bytes: usize,
    operation: impl FnOnce() -> T,
) -> (T, AllocationSample) {
    ENABLED.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    REALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);

    ENABLED.store(true, Ordering::SeqCst);
    let result = black_box(operation());
    ENABLED.store(false, Ordering::SeqCst);

    let sample = AllocationSample {
        case,
        input_bytes,
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        reallocations: REALLOCATIONS.load(Ordering::Relaxed),
        allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
        deallocated_bytes: DEALLOCATED_BYTES.load(Ordering::Relaxed),
        peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
    };
    (result, sample)
}

fn fixture(relative: &str) -> Vec<u8> {
    let path = corpus_file_path(relative);
    corpus_bytes(&path)
        .unwrap_or_else(|error| panic!("read allocation fixture {}: {error}", path.display()))
}

fn grow_first_ppt_text(sequence: &mut PptRecordSequence) -> bool {
    for record in &mut sequence.records {
        match &mut record.data {
            PptRecordData::TextChars(value) | PptRecordData::TextBytes(value) => {
                value.push('x');
                return true;
            }
            PptRecordData::Container(children) | PptRecordData::ProgTags(children) => {
                if grow_first_ppt_text(children) {
                    return true;
                }
            }
            PptRecordData::ProgBinaryTag(value) => {
                if grow_first_ppt_text(&mut value.records) {
                    return true;
                }
            }
            PptRecordData::BinaryTagData(BinaryTagData::Records(children)) => {
                if grow_first_ppt_text(children) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn main() {
    let doc_bytes = fixture(LARGE_DOC);
    let xls_bytes = fixture(LARGE_XLS);
    let ppt_bytes = fixture(LARGE_PPT);

    let cfb = CompoundFile::from_bytes(&doc_bytes).expect("open CFB allocation fixture");
    let doc = DocFile::from_bytes(&doc_bytes).expect("open DOC allocation fixture");
    let xls = XlsFile::from_bytes_compatible(&xls_bytes)
        .expect("open XLS allocation fixture")
        .value;
    let mut edited_xls = xls.clone();
    let workbook = Arc::make_mut(&mut edited_xls.workbooks)
        .first_mut()
        .expect("XLS allocation fixture has a workbook");
    let tree = Arc::make_mut(&mut workbook.tree);
    let sheet = tree
        .stream
        .records
        .iter_mut()
        .find_map(|record| match &mut record.data {
            BiffRecordData::BoundSheet8(value)
            | BiffRecordData::BoundSheet8Compatibility { value, .. } => Some(value),
            _ => None,
        })
        .expect("XLS allocation fixture has a BoundSheet8 record");
    sheet.name.value.push('x');
    let ppt = PptFile::from_bytes(&ppt_bytes).expect("open PPT allocation fixture");
    let mut edited_ppt = ppt.clone();
    let PowerPointDocument { records } = Arc::make_mut(&mut edited_ppt.document);
    assert!(
        grow_first_ppt_text(records),
        "PPT allocation fixture has a text atom"
    );
    edited_ppt
        .relayout()
        .expect("relayout edited PPT allocation fixture");

    let mut samples = Vec::with_capacity(24);

    let cfb_input = doc_bytes.clone();
    let (_, sample) = measure("cfb.open_owned", doc_bytes.len(), || {
        CompoundFile::from_vec(cfb_input).expect("profile CFB open")
    });
    samples.push(sample);
    let (_, sample) = measure("cfb.clone", doc_bytes.len(), || cfb.clone());
    samples.push(sample);
    let (_, sample) = measure("cfb.save_to_bytes", doc_bytes.len(), || {
        cfb.to_bytes().expect("profile CFB save")
    });
    samples.push(sample);
    let (_, sample) = measure("cfb.write_to_sink", doc_bytes.len(), || {
        cfb.write_to(std::io::sink())
            .expect("profile CFB sink write")
    });
    samples.push(sample);

    let (_, sample) = measure("doc.open_from_bytes", doc_bytes.len(), || {
        DocFile::from_bytes(&doc_bytes).expect("profile DOC open")
    });
    samples.push(sample);
    let doc_input = doc_bytes.clone();
    let (_, sample) = measure("doc.open_owned", doc_bytes.len(), || {
        DocFile::from_vec(doc_input).expect("profile owned DOC open")
    });
    samples.push(sample);
    let (_, sample) = measure("doc.clone", doc_bytes.len(), || doc.clone());
    samples.push(sample);
    let (_, sample) = measure("doc.build_compound_compatible", doc_bytes.len(), || {
        doc.to_compound_file_preserving_compatibility()
            .expect("profile DOC compatible compound build")
    });
    samples.push(sample);
    let (_, sample) = measure("doc.build_compound_strict", doc_bytes.len(), || {
        doc.to_compound_file()
            .expect("profile DOC strict compound build")
    });
    samples.push(sample);
    let (_, sample) = measure("doc.save_compatible", doc_bytes.len(), || {
        doc.to_bytes_preserving_compatibility()
            .expect("profile DOC compatible save")
    });
    samples.push(sample);
    let (_, sample) = measure("doc.write_compatible_to_sink", doc_bytes.len(), || {
        doc.write_to_preserving_compatibility(std::io::sink())
            .expect("profile DOC compatible sink write")
    });
    samples.push(sample);
    let (_, sample) = measure("doc.save_to_bytes", doc_bytes.len(), || {
        doc.to_bytes().expect("profile DOC save")
    });
    samples.push(sample);

    let (_, sample) = measure("xls.open_compatible", xls_bytes.len(), || {
        XlsFile::from_bytes_compatible(&xls_bytes)
            .expect("profile XLS open")
            .value
    });
    samples.push(sample);
    let xls_input = xls_bytes.clone();
    let (_, sample) = measure("xls.open_owned_compatible", xls_bytes.len(), || {
        XlsFile::from_vec_compatible(xls_input)
            .expect("profile owned XLS open")
            .value
    });
    samples.push(sample);
    let (_, sample) = measure("xls.clone", xls_bytes.len(), || xls.clone());
    samples.push(sample);
    let (_, sample) = measure("xls.save_compatible", xls_bytes.len(), || {
        xls.to_bytes_preserving_compatibility()
            .expect("profile XLS save")
    });
    samples.push(sample);
    let (_, sample) = measure("xls.save_after_layout_edit", xls_bytes.len(), || {
        edited_xls
            .to_bytes_preserving_compatibility()
            .expect("profile XLS save after layout edit")
    });
    samples.push(sample);
    let (_, sample) = measure("xls.write_compatible_to_sink", xls_bytes.len(), || {
        xls.write_to_preserving_compatibility(std::io::sink())
            .expect("profile XLS compatible sink write")
    });
    samples.push(sample);

    let (_, sample) = measure("ppt.open_from_bytes", ppt_bytes.len(), || {
        PptFile::from_bytes(&ppt_bytes).expect("profile PPT open")
    });
    samples.push(sample);
    let ppt_input = ppt_bytes.clone();
    let (_, sample) = measure("ppt.open_owned", ppt_bytes.len(), || {
        PptFile::from_vec(ppt_input).expect("profile owned PPT open")
    });
    samples.push(sample);
    let (_, sample) = measure("ppt.clone", ppt_bytes.len(), || ppt.clone());
    samples.push(sample);
    let (_, sample) = measure("ppt.save_to_bytes", ppt_bytes.len(), || {
        ppt.to_bytes().expect("profile PPT save")
    });
    samples.push(sample);
    let (_, sample) = measure("ppt.save_after_layout_edit", ppt_bytes.len(), || {
        edited_ppt
            .to_bytes()
            .expect("profile PPT save after layout edit")
    });
    samples.push(sample);
    let (_, sample) = measure("ppt.write_to_sink", ppt_bytes.len(), || {
        ppt.write_to(std::io::sink())
            .expect("profile PPT sink write")
    });
    samples.push(sample);

    println!(
        "{}",
        serde_json::to_string_pretty(&samples).expect("serialize allocation profile")
    );
}

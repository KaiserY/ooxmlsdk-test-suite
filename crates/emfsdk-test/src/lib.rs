use std::fs;
use std::path::{Path, PathBuf};

use emfsdk::emfplus::{EmfPlusRecord, EmfPlusRecordData, EmfPlusStreamRef};
use emfsdk::{
    EmfRecord, EmfRecordData, EmrComment, Metafile, MetafileRef, WmfRecord, WmfRecordData,
};
use walkdir::WalkDir;

pub fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives under workspace/crates/emfsdk-test")
        .to_path_buf()
}

pub fn corpus_dir(relative: &str) -> PathBuf {
    workspace_dir().join("corpus").join(relative)
}

pub fn collect_metafiles(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_metafile(path))
        .collect();
    files.sort();
    files
}

pub fn is_metafile(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if ext.eq_ignore_ascii_case("emf") || ext.eq_ignore_ascii_case("wmf")
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoundtripReport {
    pub emf_records: usize,
    pub wmf_records: usize,
    pub emf_plus_records: usize,
    pub compatible_emf_records: usize,
    pub compatible_wmf_records: usize,
    pub compatible_emf_plus_records: usize,
    pub unknown_emf_records: usize,
    pub unknown_wmf_records: usize,
    pub unknown_emf_plus_records: usize,
    pub compatibility_diagnostics: usize,
}

impl RoundtripReport {
    pub fn add(&mut self, other: Self) {
        self.emf_records += other.emf_records;
        self.wmf_records += other.wmf_records;
        self.emf_plus_records += other.emf_plus_records;
        self.compatible_emf_records += other.compatible_emf_records;
        self.compatible_wmf_records += other.compatible_wmf_records;
        self.compatible_emf_plus_records += other.compatible_emf_plus_records;
        self.unknown_emf_records += other.unknown_emf_records;
        self.unknown_wmf_records += other.unknown_wmf_records;
        self.unknown_emf_plus_records += other.unknown_emf_plus_records;
        self.compatibility_diagnostics += other.compatibility_diagnostics;
    }
}

pub fn roundtrip_metafile(path: &Path) -> Result<RoundtripReport, String> {
    let bytes = corpus_bytes(path)?;
    roundtrip_metafile_bytes(&bytes)
}

pub fn roundtrip_metafile_bytes(bytes: &[u8]) -> Result<RoundtripReport, String> {
    let borrowed =
        MetafileRef::from_bytes(bytes).map_err(|err| format!("borrowed parse: {err}"))?;
    validate_borrowed_records(&borrowed, bytes)?;
    let borrowed_owned = borrowed.into_owned();
    let metafile = Metafile::from_bytes(bytes).map_err(|err| format!("parse: {err}"))?;
    if borrowed_owned != metafile {
        return Err("borrowed and owned parsing differ".to_string());
    }
    let mut report = RoundtripReport {
        compatibility_diagnostics: metafile.compatibility_diagnostics().len(),
        ..RoundtripReport::default()
    };
    match &metafile {
        Metafile::Emf(value) => {
            report.emf_records = value.records.len();
            for record in &value.records {
                match record.parse_data() {
                    Ok(EmfRecordData::Unknown(_)) => report.unknown_emf_records += 1,
                    Ok(data) => {
                        if let EmfRecordData::Comment(EmrComment::EmfPlus { records, .. }) = &data {
                            report.emf_plus_records += records.len();
                            for record in records {
                                match record.parse_data() {
                                    Ok(EmfPlusRecordData::Unknown(_)) => {
                                        report.unknown_emf_plus_records += 1;
                                    }
                                    Ok(_) => {
                                        if !emf_plus_record_roundtrips(record) {
                                            report.compatible_emf_plus_records += 1;
                                        }
                                    }
                                    Err(_) => report.compatible_emf_plus_records += 1,
                                }
                            }
                        }
                        if !emf_record_roundtrips(record) {
                            report.compatible_emf_records += 1;
                        }
                    }
                    Err(_) => report.compatible_emf_records += 1,
                }
            }
        }
        Metafile::Wmf(value) => {
            report.wmf_records = value.records.len();
            for record in &value.records {
                match record.parse_data() {
                    Ok(WmfRecordData::Unknown(_)) => report.unknown_wmf_records += 1,
                    Ok(_) => {
                        if !wmf_record_roundtrips(record) {
                            report.compatible_wmf_records += 1;
                        }
                    }
                    Err(_) => report.compatible_wmf_records += 1,
                }
            }
        }
    }
    let roundtripped = metafile.to_bytes().map_err(|err| format!("write: {err}"))?;
    if roundtripped.as_slice() == bytes {
        Ok(report)
    } else {
        Err(format!(
            "roundtrip bytes differ: original={} roundtripped={}",
            bytes.len(),
            roundtripped.len()
        ))
    }
}

fn validate_borrowed_records(view: &MetafileRef<'_>, input: &[u8]) -> Result<(), String> {
    match view {
        MetafileRef::Emf(value) => {
            if value.records().len() != value.record_count() {
                return Err("borrowed EMF record count differs from iterator length".to_string());
            }
            for record in value.records() {
                ensure_slice_belongs_to_input(record.data, input, "EMF record")?;
                let Some(payload) = record.emf_plus_payload() else {
                    continue;
                };
                let Ok(stream) = EmfPlusStreamRef::from_bytes(payload) else {
                    continue;
                };
                if stream.records().len() != stream.record_count() {
                    return Err(
                        "borrowed EMF+ record count differs from iterator length".to_string()
                    );
                }
                ensure_slice_belongs_to_input(stream.trailing_data(), input, "EMF+ trailing data")?;
                for nested_record in stream.records() {
                    ensure_slice_belongs_to_input(nested_record.data, input, "EMF+ record")?;
                    ensure_slice_belongs_to_input(nested_record.padding, input, "EMF+ padding")?;
                }
            }
        }
        MetafileRef::Wmf(value) => {
            if value.records().len() != value.record_count() {
                return Err("borrowed WMF record count differs from iterator length".to_string());
            }
            for record in value.records() {
                ensure_slice_belongs_to_input(record.data, input, "WMF record")?;
            }
        }
    }
    Ok(())
}

fn ensure_slice_belongs_to_input(data: &[u8], input: &[u8], name: &str) -> Result<(), String> {
    let input_start = input.as_ptr() as usize;
    let input_end = input_start
        .checked_add(input.len())
        .ok_or_else(|| format!("{name} input address range overflows"))?;
    let data_start = data.as_ptr() as usize;
    let data_end = data_start
        .checked_add(data.len())
        .ok_or_else(|| format!("{name} address range overflows"))?;
    if data_start < input_start || data_end > input_end {
        return Err(format!("{name} data is not borrowed from the input"));
    }
    Ok(())
}

fn emf_record_roundtrips(record: &EmfRecord) -> bool {
    record
        .rebuild_typed()
        .is_ok_and(|rebuilt| rebuilt == *record)
}

fn emf_plus_record_roundtrips(record: &EmfPlusRecord) -> bool {
    record
        .rebuild_typed()
        .is_ok_and(|rebuilt| rebuilt == *record)
}

fn wmf_record_roundtrips(record: &WmfRecord) -> bool {
    record
        .rebuild_typed()
        .is_ok_and(|rebuilt| rebuilt == *record)
}

pub fn expect_parse_rejected(path: &Path) -> Result<(), String> {
    let bytes = corpus_bytes(path)?;
    match Metafile::from_bytes(&bytes) {
        Ok(metafile) => match validate_metafile(&metafile) {
            Ok(()) => Err("parse and deep validation unexpectedly succeeded".to_string()),
            Err(_) => Ok(()),
        },
        Err(_) => Ok(()),
    }
}

pub fn validate_metafile(metafile: &Metafile) -> Result<(), String> {
    metafile
        .validate_strict()
        .map_err(|err| format!("strict validation: {err}"))
}

pub fn corpus_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|err| format!("read: {err}"))?;
    if is_libreoffice_encrypted_regression(path) {
        Ok(rc4(&bytes, b"CVE"))
    } else {
        Ok(bytes)
    }
}

pub fn expects_parse_rejected(path: &Path) -> bool {
    let relative = path
        .strip_prefix(corpus_dir(""))
        .unwrap_or(path)
        .to_string_lossy();
    matches!(
        relative.as_ref(),
        "Apache-POI/test-data/slideshow/61338.wmf"
            | "Apache-POI/test-data/slideshow/clusterfuzz-testcase-minimized-6701721724125184.wmf"
            | "Apache-POI/test-data/slideshow/clusterfuzz-testcase-minimized-POIFileHandlerFuzzer-6060921738035200.wmf"
            | "Apache-POI/test-data/slideshow/clusterfuzz-testcase-minimized-POIFileHandlerFuzzer-6466833057382400.emf"
            | "Apache-POI/test-data/slideshow/crash-7b60e9fe792eaaf1bba8be90c2b62f057cfff142.emf"
            | "Apache-POI/test-data/slideshow/VHZ2NYFUYUUJNGLABL26ORTQZA76FJEW.emf"
            | "Apache-POI/test-data/spreadsheet/61294.emf"
            | "LibreOffice/framework/qa/complex/broken_document/test_documents/dbf.dbf.emf"
    ) || relative.contains("/graphicfilter/data/emf/fail/")
        || relative.contains("/graphicfilter/data/wmf/fail/")
        || relative.starts_with("libemf2svg/tests/resources/emf-corrupted/")
}

fn is_libreoffice_encrypted_regression(path: &Path) -> bool {
    if !path
        .components()
        .any(|component| component.as_os_str() == "graphicfilter")
    {
        return false;
    }
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    ["BID", "CVE", "EDB", "RC4"]
        .iter()
        .any(|marker| file_name.contains(marker))
}

fn rc4(input: &[u8], key: &[u8]) -> Vec<u8> {
    let mut state = [0u8; 256];
    for (index, value) in state.iter_mut().enumerate() {
        *value = index as u8;
    }

    let mut j = 0usize;
    for i in 0..256 {
        j = (j + state[i] as usize + key[i % key.len()] as usize) & 0xFF;
        state.swap(i, j);
    }

    let mut i = 0usize;
    j = 0;
    input
        .iter()
        .map(|byte| {
            i = (i + 1) & 0xFF;
            j = (j + state[i] as usize) & 0xFF;
            state.swap(i, j);
            let key_index = (state[i] as usize + state[j] as usize) & 0xFF;
            byte ^ state[key_index]
        })
        .collect()
}

pub fn assert_all_ok(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

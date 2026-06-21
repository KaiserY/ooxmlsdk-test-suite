use std::fs;
use std::path::{Path, PathBuf};

use emfsdk::Metafile;
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

pub fn roundtrip_metafile(path: &Path) -> Result<(), String> {
    let bytes = corpus_bytes(path)?;
    let metafile = Metafile::from_bytes(&bytes).map_err(|err| format!("parse: {err}"))?;
    let roundtripped = metafile.to_bytes().map_err(|err| format!("write: {err}"))?;
    if roundtripped == bytes {
        Ok(())
    } else {
        Err(format!(
            "roundtrip bytes differ: original={} roundtripped={}",
            bytes.len(),
            roundtripped.len()
        ))
    }
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
    match metafile {
        Metafile::Emf(value) => {
            value
                .validate_header_metrics()
                .map_err(|err| format!("validate EMF header: {err}"))?;
            for record in &value.records {
                record
                    .parse_data()
                    .map_err(|err| format!("parse EMF record {}: {err}", record.record_type))?;
            }
        }
        Metafile::Wmf(value) => {
            if let Some(placeable_header) = &value.placeable_header {
                placeable_header
                    .validate()
                    .map_err(|err| format!("validate WMF placeable header: {err}"))?;
            }
            value
                .validate_header_metrics()
                .map_err(|err| format!("validate WMF header: {err}"))?;
            for record in &value.records {
                record
                    .parse_data()
                    .map_err(|err| format!("parse WMF record {}: {err}", record.function))?;
            }
        }
    }
    Ok(())
}

pub fn corpus_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|err| format!("read: {err}"))?;
    if is_libreoffice_encrypted_regression(path) {
        Ok(rc4(&bytes, b"CVE"))
    } else {
        Ok(bytes)
    }
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

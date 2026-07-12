use std::{
    fs,
    path::{Path, PathBuf},
};

pub mod manifest;

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn corpus_root() -> PathBuf {
    workspace_root().join("corpus")
}
pub fn corpus_file_path(relative: &str) -> PathBuf {
    corpus_root().join(relative)
}

pub fn corpus_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    if is_libreoffice_rc4_fixture(path) {
        Ok(rc4(&bytes, b"CVE"))
    } else {
        Ok(bytes)
    }
}

pub fn assert_cfb_round_trip(path: &Path, file_name: &str) {
    let bytes = corpus_bytes(path).unwrap_or_else(|err| panic!("{file_name}: {err}"));
    let original = olecfsdk::cfb::CompoundFile::from_bytes(&bytes)
        .unwrap_or_else(|err| panic!("{file_name}: open original CFB: {err}"));
    let saved = original
        .to_bytes()
        .unwrap_or_else(|err| panic!("{file_name}: save CFB: {err}"));
    cfb::CompoundFile::open_strict(std::io::Cursor::new(saved.as_slice()))
        .unwrap_or_else(|err| panic!("{file_name}: strict reference reopen: {err}"));
    let reopened = olecfsdk::cfb::CompoundFile::from_bytes(&saved)
        .unwrap_or_else(|err| panic!("{file_name}: reopen saved CFB: {err}"));
    assert!(
        original.logical_eq(&reopened),
        "{file_name}: logical CFB graph or stream bytes differ"
    );
    let saved_again = reopened
        .to_bytes()
        .unwrap_or_else(|err| panic!("{file_name}: second save CFB: {err}"));
    let reopened_again = olecfsdk::cfb::CompoundFile::from_bytes(&saved_again)
        .unwrap_or_else(|err| panic!("{file_name}: reopen second saved CFB: {err}"));
    assert!(
        reopened.logical_eq(&reopened_again),
        "{file_name}: second CFB round-trip is not stable"
    );
}

pub fn assert_cfb_opens(path: &Path, file_name: &str) {
    let bytes = corpus_bytes(path).unwrap_or_else(|err| panic!("{file_name}: {err}"));
    olecfsdk::cfb::CompoundFile::from_bytes(&bytes)
        .unwrap_or_else(|err| panic!("{file_name}: expected CFB to open: {err}"));
}

pub fn assert_cfb_rejected(path: &Path, file_name: &str) {
    let bytes = corpus_bytes(path).unwrap_or_else(|err| panic!("{file_name}: {err}"));
    assert!(
        olecfsdk::cfb::CompoundFile::from_bytes(&bytes).is_err(),
        "{file_name}: expected invalid CFB to be rejected"
    );
}

pub fn assert_cfb_unsupported(path: &Path, file_name: &str) {
    let bytes = corpus_bytes(path).unwrap_or_else(|err| panic!("{file_name}: {err}"));
    assert!(
        olecfsdk::cfb::CompoundFile::from_bytes(&bytes).is_err(),
        "{file_name}: unsupported non-CFB input unexpectedly parsed as CFB"
    );
}

pub fn is_libreoffice_rc4_fixture(path: &Path) -> bool {
    if !path
        .components()
        .any(|component| component.as_os_str() == "LibreOffice")
    {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    ["BID", "CVE", "EDB", "RC4"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn rc4(input: &[u8], key: &[u8]) -> Vec<u8> {
    let mut state = [0u8; 256];
    for (index, value) in state.iter_mut().enumerate() {
        *value = index as u8;
    }
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + state[i] as usize + key[i % key.len()] as usize) & 0xff;
        state.swap(i, j);
    }
    let mut i = 0usize;
    j = 0;
    input
        .iter()
        .map(|byte| {
            i = (i + 1) & 0xff;
            j = (j + state[i] as usize) & 0xff;
            state.swap(i, j);
            byte ^ state[(state[i] as usize + state[j] as usize) & 0xff]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_is_symmetric() {
        let encrypted = rc4(b"compound file", b"CVE");
        assert_eq!(rc4(&encrypted, b"CVE"), b"compound file");
    }
}

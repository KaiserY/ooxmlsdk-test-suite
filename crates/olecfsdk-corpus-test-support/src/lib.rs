use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use olecfsdk::cfb::{CompoundFile, CompoundFileReader};

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
    let original = CompoundFile::from_bytes(&bytes)
        .unwrap_or_else(|err| panic!("{file_name}: open original CFB: {err}"));
    assert_streaming_matches(&bytes, &original, false, file_name);
    let saved = original
        .to_bytes()
        .unwrap_or_else(|err| panic!("{file_name}: save CFB: {err}"));
    let reopened = CompoundFile::from_bytes_strict(&saved)
        .unwrap_or_else(|err| panic!("{file_name}: strict reopen saved CFB: {err}"));
    assert_streaming_matches(&saved, &reopened, true, file_name);
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

fn assert_streaming_matches(bytes: &[u8], expected: &CompoundFile, strict: bool, file_name: &str) {
    let cursor = Cursor::new(bytes);
    let mut streamed = if strict {
        CompoundFileReader::from_reader_strict(cursor)
    } else {
        CompoundFileReader::from_reader(cursor)
    }
    .unwrap_or_else(|err| panic!("{file_name}: open streaming CFB: {err}"));
    assert_eq!(
        streamed.entries().len(),
        expected.entries().len(),
        "{file_name}: streaming entry count differs"
    );
    for entry in expected.entries() {
        let info = streamed.entry(&entry.path).unwrap_or_else(|| {
            panic!(
                "{file_name}: streaming CFB is missing {}",
                entry.path.display()
            )
        });
        assert_eq!(info.path, entry.path, "{file_name}: entry path differs");
        assert_eq!(info.name, entry.name, "{file_name}: entry name differs");
        assert_eq!(info.kind, entry.kind, "{file_name}: entry kind differs");
        assert_eq!(info.clsid, entry.clsid, "{file_name}: entry CLSID differs");
        assert_eq!(
            info.state_bits, entry.state_bits,
            "{file_name}: entry state bits differ"
        );
        assert_eq!(
            info.created, entry.created,
            "{file_name}: entry creation time differs"
        );
        assert_eq!(
            info.modified, entry.modified,
            "{file_name}: entry modification time differs"
        );
        if entry.is_stream() {
            assert_eq!(
                info.stream_len,
                entry.data.len() as u64,
                "{file_name}: stream length differs for {}",
                entry.path.display()
            );
            let mut actual = Vec::new();
            streamed
                .open_stream(&entry.path)
                .and_then(|mut stream| {
                    stream.read_to_end(&mut actual)?;
                    Ok(())
                })
                .unwrap_or_else(|err| {
                    panic!(
                        "{file_name}: streaming read failed for {}: {err}",
                        entry.path.display()
                    )
                });
            assert_eq!(
                actual,
                entry.data,
                "{file_name}: streaming bytes differ for {}",
                entry.path.display()
            );
        }
    }
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

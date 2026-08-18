use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    cfb::CompoundFile,
    forms::{FormControl, FormObjectStream, MultiPageXStream, ParentControlStorage, SiteFlags},
};

#[test]
#[ignore = "cross-format embedded OLE control inventory runs explicitly"]
fn embedded_ole_control_inventory() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);
    files.sort();

    let mut compound_files = 0usize;
    let mut control_classes = BTreeMap::<String, usize>::new();
    let mut form_stream_shapes =
        BTreeMap::<(String, Option<usize>, Option<usize>, Option<usize>), usize>::new();
    for path in files {
        let Ok(bytes) = olecfsdk_corpus_test_support::corpus_bytes(&path) else {
            continue;
        };
        let Ok(compound) = CompoundFile::from_bytes(&bytes) else {
            continue;
        };
        compound_files += 1;
        for storage in compound
            .entries()
            .iter()
            .filter(|entry| entry.is_storage() && !entry.clsid.is_zero())
        {
            let mut children = compound
                .entries()
                .iter()
                .filter(|entry| entry.path.parent() == Some(storage.path.as_path()))
                .map(|entry| {
                    (
                        format!(
                            "{}:{}",
                            if entry.is_stream() {
                                "stream"
                            } else {
                                "storage"
                            },
                            entry.name
                        ),
                        entry.data.len(),
                    )
                })
                .collect::<Vec<_>>();
            children.sort();
            let looks_like_control = children.iter().any(|(name, _)| {
                matches!(
                    name.as_str(),
                    "stream:contents" | "stream:f" | "stream:\u{3}OCXDATA"
                )
            });
            if !looks_like_control {
                continue;
            }
            let class_id = storage.clsid.to_string();
            *control_classes.entry(class_id.clone()).or_default() += 1;
            if matches!(
                class_id.as_str(),
                "46e31370-3f7a-11ce-bed6-00aa00611080"
                    | "6e182020-f460-11ce-9bcd-00aa00608e01"
                    | "c62a69f0-16dc-11ce-9e98-00aa00574a4f"
            ) {
                let stream_size = |name: &str| {
                    children
                        .iter()
                        .find(|(entry_name, _)| entry_name == &format!("stream:{name}"))
                        .map(|(_, size)| *size)
                };
                *form_stream_shapes
                    .entry((
                        class_id,
                        stream_size("f"),
                        stream_size("o"),
                        stream_size("x"),
                    ))
                    .or_default() += 1;
            }
        }
    }

    assert_eq!(compound_files, 1_407);
    assert_eq!(
        control_classes,
        BTreeMap::from([
            ("46e31370-3f7a-11ce-bed6-00aa00611080".into(), 1),
            ("6e182020-f460-11ce-9bcd-00aa00608e01".into(), 3),
            ("8bd21d10-ec42-11ce-9e0d-00aa006002f3".into(), 116),
            ("8bd21d40-ec42-11ce-9e0d-00aa006002f3".into(), 4),
            ("8bd21d50-ec42-11ce-9e0d-00aa006002f3".into(), 2),
            ("ae24fdae-03c6-11d1-8b76-0080c744f389".into(), 1),
            ("c62a69f0-16dc-11ce-9e98-00aa00574a4f".into(), 2),
            ("d7053240-ce69-11cd-a777-00dd01143c57".into(), 1),
        ])
    );
    assert_eq!(
        form_stream_shapes,
        BTreeMap::from([
            (
                (
                    "46e31370-3f7a-11ce-bed6-00aa00611080".into(),
                    Some(176),
                    Some(144),
                    Some(48),
                ),
                1,
            ),
            (
                (
                    "6e182020-f460-11ce-9bcd-00aa00608e01".into(),
                    Some(165),
                    Some(64),
                    None,
                ),
                1,
            ),
            (
                (
                    "6e182020-f460-11ce-9bcd-00aa00608e01".into(),
                    Some(169),
                    Some(92),
                    None,
                ),
                1,
            ),
            (
                (
                    "6e182020-f460-11ce-9bcd-00aa00608e01".into(),
                    Some(265),
                    Some(458),
                    None,
                ),
                1,
            ),
            (
                (
                    "c62a69f0-16dc-11ce-9e98-00aa00574a4f".into(),
                    Some(40),
                    Some(0),
                    None,
                ),
                2,
            ),
        ])
    );
}

#[test]
#[ignore = "VBA UserForm corpus round-trip runs explicitly"]
fn vba_form_streams_round_trip_static() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut multi_page_x_streams = 0usize;
    let mut object_shapes = BTreeMap::<(u16, u32), usize>::new();
    let mut typed_objects = BTreeMap::<u16, usize>::new();
    for relative in [
        "Apache-POI/test-data/spreadsheet/15556.xls",
        "Apache-POI/test-data/spreadsheet/31749.xls",
    ] {
        let path = corpus.join(relative);
        let bytes = olecfsdk_corpus_test_support::corpus_bytes(&path).unwrap();
        let compound = CompoundFile::from_bytes(&bytes).unwrap();
        for storage in compound.entries().iter().filter(|entry| {
            entry.is_storage()
                && matches!(
                    entry.clsid.to_string().as_str(),
                    "46e31370-3f7a-11ce-bed6-00aa00611080"
                        | "6e182020-f460-11ce-9bcd-00aa00608e01"
                        | "c62a69f0-16dc-11ce-9e98-00aa00574a4f"
                )
        }) {
            let aggregate = ParentControlStorage::from_compound(&compound, &storage.path)
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to aggregate {relative}:{}: {error}",
                        storage.path.display()
                    )
                });
            let mut rewritten = compound.clone();
            aggregate.write_to_compound(&mut rewritten).unwrap();
            assert!(compound.logical_eq(&rewritten));

            let stream = compound.stream(storage.path.join("f")).unwrap();
            let form = FormControl::from_bytes(stream).unwrap_or_else(|error| {
                panic!(
                    "failed to parse {relative}:{}: {error}",
                    storage.path.display()
                )
            });
            assert_eq!(form.to_bytes().unwrap(), stream);
            let object_stream = compound.stream(storage.path.join("o")).unwrap_or(&[]);
            let objects = FormObjectStream::from_form(&form, object_stream).unwrap();
            assert_eq!(objects.to_bytes(&form).unwrap(), object_stream);
            for control in &objects.controls {
                *typed_objects
                    .entry(control.class_index.to_raw().unwrap())
                    .or_default() += 1;
            }
            let mut object_stream_bytes = 0u32;
            for site in &form.site_data.sites {
                let bit_flags = site
                    .data_block
                    .bit_flags
                    .as_ref()
                    .map_or_else(|| SiteFlags::from_bits_retain(0x33), |value| value.value);
                if bit_flags.contains(SiteFlags::STREAMED) {
                    let class_index = site
                        .data_block
                        .clsid_cache_index
                        .as_ref()
                        .map_or(0x7fff, |value| value.value.to_raw().unwrap());
                    let size = site
                        .data_block
                        .object_stream_size
                        .as_ref()
                        .map_or(0, |value| value.value);
                    object_stream_bytes += size;
                    *object_shapes.entry((class_index, size)).or_default() += 1;
                }
            }
            assert_eq!(
                usize::try_from(object_stream_bytes).unwrap(),
                compound
                    .stream(storage.path.join("o"))
                    .map_or(0, <[u8]>::len)
            );
            if storage.clsid.to_string() == "46e31370-3f7a-11ce-bed6-00aa00611080" {
                let x_stream = compound.stream(storage.path.join("x")).unwrap();
                let x = MultiPageXStream::from_bytes(x_stream).unwrap();
                assert_eq!(x.pages.len(), 3);
                assert_eq!(x.multi_page.page_ids, [4, 5]);
                assert_eq!(x.to_bytes().unwrap(), x_stream);
                multi_page_x_streams += 1;
            }
            *counts.entry(storage.clsid.to_string()).or_default() += 1;
        }
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("46e31370-3f7a-11ce-bed6-00aa00611080".into(), 1),
            ("6e182020-f460-11ce-9bcd-00aa00608e01".into(), 3),
            ("c62a69f0-16dc-11ce-9e98-00aa00574a4f".into(), 2),
        ])
    );
    assert_eq!(multi_page_x_streams, 1);
    assert_eq!(
        object_shapes,
        BTreeMap::from([
            ((17, 182), 1),
            ((17, 184), 1),
            ((18, 144), 1),
            ((24, 64), 1),
            ((24, 92), 2),
        ])
    );
    assert_eq!(typed_objects, BTreeMap::from([(17, 2), (18, 1), (24, 3)]));
}

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}

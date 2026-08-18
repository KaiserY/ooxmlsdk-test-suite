#![cfg(feature = "mce")]

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use ooxmlsdk::{
    parts::{
        presentation_document::PresentationDocument, spreadsheet_document::SpreadsheetDocument,
        wordprocessing_document::WordprocessingDocument,
    },
    sdk::{
        FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
        OpenSettings, PackageOpenMode,
    },
};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde::Deserialize;

const MCE_MARKERS: [&[u8]; 6] = [
    b"AlternateContent",
    b":Ignorable=",
    b":ProcessContent=",
    b":PreserveElements=",
    b":PreserveAttributes=",
    b":MustUnderstand=",
];

#[derive(Default)]
struct CorpusMceStats {
    packages_scanned: usize,
    invalid_packages_skipped: usize,
    packages_with_mce: usize,
    packages_with_alternate_content: usize,
    alternate_content_parts: usize,
    typed_alternate_content_parts: usize,
    opaque_alternate_content_parts: usize,
}

struct PackageMceInventory {
    has_mce: bool,
    typed_alternate_content_parts: Vec<String>,
    opaque_alternate_content_parts: Vec<(String, Vec<u8>)>,
}

#[derive(Deserialize)]
struct CorpusManifest {
    #[serde(default)]
    expectation: Vec<CorpusExpectation>,
}

#[derive(Deserialize)]
struct CorpusExpectation {
    file: String,
    test: String,
    mode: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_office_packages(dir: &Path, packages: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read corpus directory {}: {err}", dir.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| panic!("read corpus entry in {}: {err}", dir.display()));
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_office_packages(&path, packages);
        } else if is_office_package(&path) {
            packages.push(path);
        }
    }
}

fn is_office_package(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("~$"))
    {
        return false;
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "docx"
                    | "dotx"
                    | "docm"
                    | "dotm"
                    | "xlsx"
                    | "xltx"
                    | "xlsm"
                    | "xltm"
                    | "pptx"
                    | "potx"
                    | "pptm"
                    | "potm"
                    | "ppsx"
                    | "ppsm"
            )
        })
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn invalid_roundtrip_files(corpus_dir: &Path) -> HashSet<String> {
    let manifest_path = corpus_dir.join("manifest.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("read corpus manifest {}: {err}", manifest_path.display()));
    let manifest: CorpusManifest = toml::from_str(&manifest)
        .unwrap_or_else(|err| panic!("parse corpus manifest {}: {err}", manifest_path.display()));

    manifest
        .expectation
        .into_iter()
        .filter(|expectation| expectation.test == "roundtrip" && expectation.mode == "invalid")
        .map(|expectation| expectation.file)
        .collect()
}

fn typed_xml_parts(
    archive: &mut zip::ZipArchive<File>,
    path: &Path,
) -> Result<HashSet<String>, String> {
    let mut content_types = archive
        .by_name("[Content_Types].xml")
        .map_err(|err| format!("open [Content_Types].xml in {}: {err}", path.display()))?;
    let mut bytes = Vec::new();
    content_types
        .read_to_end(&mut bytes)
        .map_err(|err| format!("read [Content_Types].xml in {}: {err}", path.display()))?;
    drop(content_types);

    let mut reader = Reader::from_reader(bytes.as_slice());
    let mut buffer = Vec::new();
    let mut typed_parts = HashSet::new();
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|err| format!("parse [Content_Types].xml in {}: {err}", path.display()))?
        {
            Event::Start(element) | Event::Empty(element)
                if element.local_name().as_ref() == b"Override" =>
            {
                let mut part_name = None;
                let mut content_type = None;
                for attribute in element.attributes() {
                    let attribute = attribute.map_err(|err| {
                        format!(
                            "parse [Content_Types].xml attribute in {}: {err}",
                            path.display()
                        )
                    })?;
                    let value = attribute
                        .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                        .map_err(|err| {
                            format!(
                                "decode [Content_Types].xml attribute in {}: {err}",
                                path.display()
                            )
                        })?
                        .into_owned();
                    match attribute.key.local_name().as_ref() {
                        b"PartName" => part_name = Some(value),
                        b"ContentType" => content_type = Some(value),
                        _ => {}
                    }
                }
                if let (Some(part_name), Some(content_type)) = (part_name, content_type)
                    && content_type != "application/xml"
                {
                    typed_parts.insert(part_name.trim_start_matches('/').to_owned());
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(typed_parts)
}

fn package_mce_parts(path: &Path) -> Result<PackageMceInventory, String> {
    let file = File::open(path).map_err(|err| format!("open package: {err}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| format!("open ZIP package: {err}"))?;
    let typed_parts = typed_xml_parts(&mut archive, path)?;
    let mut has_mce = false;
    let mut typed_alternate_content_parts = Vec::new();
    let mut opaque_alternate_content_parts = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| format!("read ZIP entry {index}: {err}"))?;
        if !entry.name().ends_with(".xml") {
            continue;
        }
        let entry_name = entry.name().to_owned();
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|err| format!("read XML part {entry_name}: {err}"))?;
        has_mce |= MCE_MARKERS
            .iter()
            .any(|marker| contains_bytes(&bytes, marker));
        if contains_bytes(&bytes, b"AlternateContent") {
            if typed_parts.contains(&entry_name) {
                typed_alternate_content_parts.push(entry_name);
            } else {
                opaque_alternate_content_parts.push((entry_name, bytes));
            }
        }
    }

    Ok(PackageMceInventory {
        has_mce,
        typed_alternate_content_parts,
        opaque_alternate_content_parts,
    })
}

fn process_package(path: &Path, version: FileFormatVersion) -> Result<Vec<u8>, String> {
    let settings = OpenSettings {
        open_mode: PackageOpenMode::Lazy,
        markup_compatibility_process_settings: MarkupCompatibilityProcessSettings {
            process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
            target_file_format_version: version,
        },
        ..Default::default()
    };
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut output = Cursor::new(Vec::new());

    match extension.as_str() {
        "docx" | "dotx" | "docm" | "dotm" => {
            let package = WordprocessingDocument::new_from_file_with_settings(path, settings)
                .map_err(|err| format!("open wordprocessing package: {err:?}"))?;
            package
                .save(&mut output)
                .map_err(|err| format!("save wordprocessing package: {err:?}"))?;
        }
        "xlsx" | "xltx" | "xlsm" | "xltm" => {
            let package = SpreadsheetDocument::new_from_file_with_settings(path, settings)
                .map_err(|err| format!("open spreadsheet package: {err:?}"))?;
            package
                .save(&mut output)
                .map_err(|err| format!("save spreadsheet package: {err:?}"))?;
        }
        "pptx" | "potx" | "pptm" | "potm" | "ppsx" | "ppsm" => {
            let package = PresentationDocument::new_from_file_with_settings(path, settings)
                .map_err(|err| format!("open presentation package: {err:?}"))?;
            package
                .save(&mut output)
                .map_err(|err| format!("save presentation package: {err:?}"))?;
        }
        _ => return Err(format!("unsupported package extension {extension}")),
    }

    Ok(output.into_inner())
}

fn mce_output_failures(
    bytes: &[u8],
    inventory: &PackageMceInventory,
) -> Result<Vec<String>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|err| format!("open processed ZIP package: {err}"))?;
    let mut failures = Vec::new();

    for part_name in &inventory.typed_alternate_content_parts {
        let mut entry = archive
            .by_name(part_name)
            .map_err(|err| format!("open processed XML part {part_name}: {err}"))?;
        let mut part = Vec::new();
        entry
            .read_to_end(&mut part)
            .map_err(|err| format!("read processed XML part {part_name}: {err}"))?;
        if contains_bytes(&part, b"AlternateContent") {
            failures.push(format!(
                "AlternateContent remains in typed part {part_name}"
            ));
        }
    }

    for (part_name, original) in &inventory.opaque_alternate_content_parts {
        let mut entry = archive
            .by_name(part_name)
            .map_err(|err| format!("open processed opaque XML part {part_name}: {err}"))?;
        let mut part = Vec::new();
        entry
            .read_to_end(&mut part)
            .map_err(|err| format!("read processed opaque XML part {part_name}: {err}"))?;
        if &part != original {
            failures.push(format!("opaque XML part {part_name} was modified"));
        }
    }

    Ok(failures)
}

fn assert_corpus_mce_is_fully_processed(corpus_name: &str) {
    let corpus_dir = workspace_root().join("corpus").join(corpus_name);
    let mut packages = Vec::new();
    collect_office_packages(&corpus_dir, &mut packages);
    packages.sort();
    let invalid_files = invalid_roundtrip_files(&corpus_dir);

    let mut stats = CorpusMceStats::default();
    let mut failures = Vec::new();

    for path in packages {
        stats.packages_scanned += 1;
        let corpus_relative = path
            .strip_prefix(&corpus_dir)
            .unwrap_or(&path)
            .display()
            .to_string();
        if invalid_files.contains(&corpus_relative) {
            stats.invalid_packages_skipped += 1;
            continue;
        }
        let relative = path
            .strip_prefix(workspace_root())
            .unwrap_or(&path)
            .display()
            .to_string();
        let inventory = match package_mce_parts(&path) {
            Ok(inventory) => inventory,
            Err(err) => {
                failures.push(format!("{relative}: inventory failed: {err}"));
                continue;
            }
        };
        if !inventory.has_mce {
            continue;
        }
        stats.packages_with_mce += 1;
        let alternate_content_part_count = inventory.typed_alternate_content_parts.len()
            + inventory.opaque_alternate_content_parts.len();
        if alternate_content_part_count > 0 {
            stats.packages_with_alternate_content += 1;
            stats.alternate_content_parts += alternate_content_part_count;
            stats.typed_alternate_content_parts += inventory.typed_alternate_content_parts.len();
            stats.opaque_alternate_content_parts += inventory.opaque_alternate_content_parts.len();
        }

        for version in [
            FileFormatVersion::Office2007,
            FileFormatVersion::Microsoft365,
        ] {
            let processed = match process_package(&path, version) {
                Ok(processed) => processed,
                Err(err) => {
                    failures.push(format!("{relative} [{version:?}]: {err}"));
                    continue;
                }
            };
            match mce_output_failures(&processed, &inventory) {
                Ok(output_failures) if output_failures.is_empty() => {}
                Ok(output_failures) => failures.push(format!(
                    "{relative} [{version:?}]: {}",
                    output_failures.join(", ")
                )),
                Err(err) => failures.push(format!("{relative} [{version:?}]: {err}")),
            }
        }
    }

    assert!(
        stats.packages_scanned > 0,
        "no Office packages found under {}",
        corpus_dir.display()
    );
    assert!(
        stats.packages_with_mce > 0,
        "no MCE-bearing Office packages found under {}",
        corpus_dir.display()
    );
    assert!(
        failures.is_empty(),
        "{corpus_name} MCE corpus gate failed with {} failure(s); \
         scanned {} package(s), skipped {} manifest-invalid package(s), found {} MCE package(s), \
         {} package(s)/{} XML part(s) with AlternateContent ({} typed, {} opaque):\n{}",
        failures.len(),
        stats.packages_scanned,
        stats.invalid_packages_skipped,
        stats.packages_with_mce,
        stats.packages_with_alternate_content,
        stats.alternate_content_parts,
        stats.typed_alternate_content_parts,
        stats.opaque_alternate_content_parts,
        failures.join("\n")
    );
}

macro_rules! corpus_mce_test {
    ($name:ident, $corpus:literal) => {
        #[test]
        #[ignore = "exhaustive MCE corpus tests are run explicitly"]
        fn $name() {
            assert_corpus_mce_is_fully_processed($corpus);
        }
    };
}

corpus_mce_test!(open_xml_sdk_mce_corpus, "Open-XML-SDK");
corpus_mce_test!(apache_poi_mce_corpus, "Apache-POI");
corpus_mce_test!(libreoffice_mce_corpus, "LibreOffice");
corpus_mce_test!(pandoc_mce_corpus, "Pandoc");
corpus_mce_test!(closedxml_mce_corpus, "ClosedXML");

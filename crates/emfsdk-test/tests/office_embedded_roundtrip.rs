use std::{fs, path::Path};

use emfsdk::{DeviceIndependentBitmap, DibColorUsage};
use olecfsdk::{
    cfb::CompoundFile,
    office_art::{
        OfficeArtBStoreDelay, OfficeArtBitmapData, OfficeArtMetafileData, OfficeArtPartialStream,
        OfficeArtRecord, OfficeArtRecordData, OfficeArtStream,
    },
    ppt::{BinaryTagData, PicturesStream, PowerPointDocument, PptRecordData, PptRecordSequence},
    xls::{BiffRecordData, BiffStream, BkHimImage, MsoDrawingData, MsoDrawingRecord},
};
use walkdir::WalkDir;

#[derive(Default)]
struct Audit {
    file_read_failures: usize,
    compound_parse_failures: usize,
    workbook_parse_failures: usize,
    powerpoint_parse_failures: usize,
    pictures_parse_failures: usize,
    compound_files: usize,
    workbook_streams: usize,
    powerpoint_streams: usize,
    pictures_streams: usize,
    complete_pictures_streams: usize,
    compatibility_pictures_streams: usize,
    partial_pictures_streams: usize,
    emf: usize,
    wmf: usize,
    dib: usize,
    emf_bytes: usize,
    wmf_bytes: usize,
    dib_bytes: usize,
    emf_records: usize,
    wmf_records: usize,
    emf_plus_records: usize,
    compatible_emf_records: usize,
    compatible_wmf_records: usize,
    compatible_emf_plus_records: usize,
    unknown_emf_records: usize,
    unknown_wmf_records: usize,
    unknown_emf_plus_records: usize,
    compatibility_diagnostics: usize,
    failures: Vec<String>,
}

#[test]
fn embedded_office_metafiles_round_trip_through_emfsdk() {
    let corpus = emfsdk_test::corpus_dir("");
    let mut files: Vec<_> = WalkDir::new(&corpus)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some(extension)
                    if extension.eq_ignore_ascii_case("xls")
                        || extension.eq_ignore_ascii_case("ppt")
            )
        })
        .collect();
    files.sort();

    let mut audit = Audit::default();
    for path in files {
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                audit.file_read_failures += 1;
                continue;
            }
        };
        let compound = match CompoundFile::from_bytes(&bytes) {
            Ok(compound) => compound,
            Err(_) => {
                audit.compound_parse_failures += 1;
                continue;
            }
        };
        audit.compound_files += 1;
        for entry in compound.entries().iter().filter(|entry| entry.is_stream()) {
            if entry.name.eq_ignore_ascii_case("Workbook")
                || entry.name.eq_ignore_ascii_case("Book")
            {
                match BiffStream::from_bytes(&entry.data) {
                    Ok(workbook) => {
                        audit.workbook_streams += 1;
                        visit_workbook(&workbook, &path, &mut audit);
                    }
                    Err(_) => audit.workbook_parse_failures += 1,
                }
            } else if entry.name.eq_ignore_ascii_case("PowerPoint Document") {
                match PowerPointDocument::from_bytes(&entry.data) {
                    Ok(powerpoint) => {
                        audit.powerpoint_streams += 1;
                        visit_ppt_sequence(&powerpoint.records, &path, &mut audit);
                    }
                    Err(_) => audit.powerpoint_parse_failures += 1,
                }
            } else if entry.name.eq_ignore_ascii_case("Pictures") {
                match PicturesStream::from_bytes(&entry.data) {
                    Ok(pictures) => {
                        audit.pictures_streams += 1;
                        match &pictures {
                            PicturesStream::Complete(stream) => {
                                audit.complete_pictures_streams += 1;
                                visit_bstore_delay(stream, &path, &mut audit);
                            }
                            PicturesStream::Compatibility { stream, .. } => {
                                audit.compatibility_pictures_streams += 1;
                                visit_stream(stream, &path, &mut audit);
                            }
                            PicturesStream::Partial(stream) => {
                                audit.partial_pictures_streams += 1;
                                visit_partial_stream(stream, &path, &mut audit);
                            }
                        }
                    }
                    Err(_) => audit.pictures_parse_failures += 1,
                }
            }
        }
    }

    eprintln!(
        "embedded ratchet: EMF {}/{} WMF {}/{} EMF+ {}/{} diagnostics {}",
        audit.emf_records,
        audit.compatible_emf_records,
        audit.wmf_records,
        audit.compatible_wmf_records,
        audit.emf_plus_records,
        audit.compatible_emf_plus_records,
        audit.compatibility_diagnostics,
    );

    assert_eq!(audit.file_read_failures, 0, "Office file reads changed");
    assert_eq!(
        audit.compound_parse_failures, 100,
        "Office non-CFB inputs changed"
    );
    assert_eq!(
        audit.workbook_parse_failures, 4,
        "unsupported Workbook streams changed"
    );
    assert_eq!(
        audit.powerpoint_parse_failures, 0,
        "unsupported PowerPoint streams changed"
    );
    assert_eq!(
        audit.pictures_parse_failures, 0,
        "unsupported Pictures streams changed"
    );
    assert_eq!(audit.compound_files, 898, "Office CFB coverage changed");
    assert_eq!(audit.workbook_streams, 723, "BIFF coverage changed");
    assert_eq!(audit.powerpoint_streams, 179, "PPT coverage changed");
    assert_eq!(audit.pictures_streams, 87, "Pictures coverage changed");
    assert_eq!(
        audit.complete_pictures_streams, 79,
        "complete Pictures stream coverage changed"
    );
    assert_eq!(
        audit.compatibility_pictures_streams, 1,
        "compatibility Pictures stream coverage changed"
    );
    assert_eq!(
        audit.partial_pictures_streams, 7,
        "partial Pictures stream coverage changed"
    );
    assert_eq!(audit.emf, 134, "embedded EMF coverage changed");
    assert_eq!(audit.emf_bytes, 6_486_764, "embedded EMF bytes changed");
    assert_eq!(audit.wmf, 156, "embedded WMF coverage changed");
    assert_eq!(audit.wmf_bytes, 8_023_392, "embedded WMF bytes changed");
    assert_eq!(audit.dib, 50, "embedded DIB coverage changed");
    assert_eq!(audit.dib_bytes, 80_820, "embedded DIB bytes changed");
    assert_eq!(audit.emf_records, 40_339, "embedded EMF records changed");
    assert_eq!(audit.wmf_records, 66_356, "embedded WMF records changed");
    assert_eq!(audit.emf_plus_records, 889, "embedded EMF+ records changed");
    assert_eq!(
        audit.compatible_emf_records, 3,
        "embedded compatible EMF records changed"
    );
    assert_eq!(
        audit.compatible_wmf_records, 134,
        "embedded compatible WMF records changed"
    );
    assert_eq!(
        audit.compatible_emf_plus_records, 1,
        "embedded compatible EMF+ records changed"
    );
    assert_eq!(
        audit.unknown_emf_records, 0,
        "embedded unknown EMF records changed"
    );
    assert_eq!(
        audit.unknown_wmf_records, 0,
        "embedded unknown WMF records changed"
    );
    assert_eq!(
        audit.unknown_emf_plus_records, 0,
        "embedded unknown EMF+ records changed"
    );
    assert_eq!(
        audit.compatibility_diagnostics, 4_099,
        "embedded compatibility diagnostics changed"
    );
    assert!(
        audit.failures.is_empty(),
        "{} embedded Office image round-trip failures:\n{}",
        audit.failures.len(),
        audit.failures.join("\n")
    );
    eprintln!(
        "Office embedded image corpus: {} CFB files, {} Workbook/{} PowerPoint/{} Pictures streams ({} complete/{} compatibility/{} partial); {} EMF/{} bytes, {} WMF/{} bytes, {} DIB/{} bytes",
        audit.compound_files,
        audit.workbook_streams,
        audit.powerpoint_streams,
        audit.pictures_streams,
        audit.complete_pictures_streams,
        audit.compatibility_pictures_streams,
        audit.partial_pictures_streams,
        audit.emf,
        audit.emf_bytes,
        audit.wmf,
        audit.wmf_bytes,
        audit.dib,
        audit.dib_bytes,
    );
    eprintln!(
        "Office embedded metafile records: {} EMF ({} compatible/{} unknown), {} WMF ({} compatible/{} unknown), {} EMF+ ({} compatible/{} unknown); {} compatibility diagnostics",
        audit.emf_records,
        audit.compatible_emf_records,
        audit.unknown_emf_records,
        audit.wmf_records,
        audit.compatible_wmf_records,
        audit.unknown_wmf_records,
        audit.emf_plus_records,
        audit.compatible_emf_plus_records,
        audit.unknown_emf_plus_records,
        audit.compatibility_diagnostics,
    );
    eprintln!(
        "Office scan failures: {} read, {} CFB, {} Workbook, {} PowerPoint, {} Pictures",
        audit.file_read_failures,
        audit.compound_parse_failures,
        audit.workbook_parse_failures,
        audit.powerpoint_parse_failures,
        audit.pictures_parse_failures,
    );
}

fn visit_workbook(workbook: &BiffStream, path: &Path, audit: &mut Audit) {
    for record in &workbook.records {
        match &record.data {
            BiffRecordData::MsoDrawingGroup(drawing) | BiffRecordData::MsoDrawing(drawing) => {
                visit_mso_drawing(drawing, path, audit)
            }
            BiffRecordData::GelFrame(stream) => visit_stream(stream, path, audit),
            BiffRecordData::BkHim(value) | BiffRecordData::ImData(value) => {
                if let BkHimImage::Bitmap(bytes) = &value.image {
                    check_dib(bytes, path, audit);
                }
            }
            _ => {}
        }
    }
}

fn visit_mso_drawing(drawing: &MsoDrawingRecord, path: &Path, audit: &mut Audit) {
    match &drawing.data {
        MsoDrawingData::Complete(stream) => visit_stream(stream, path, audit),
        MsoDrawingData::Partial(stream) => visit_partial_stream(stream, path, audit),
        MsoDrawingData::Incomplete { .. } => {}
    }
}

fn visit_ppt_sequence(sequence: &PptRecordSequence, path: &Path, audit: &mut Audit) {
    for record in &sequence.records {
        match &record.data {
            PptRecordData::Container(children) | PptRecordData::ProgTags(children) => {
                visit_ppt_sequence(children, path, audit);
            }
            PptRecordData::ProgBinaryTag(tag) => visit_ppt_sequence(&tag.records, path, audit),
            PptRecordData::BinaryTagData(BinaryTagData::Records(children)) => {
                visit_ppt_sequence(children, path, audit);
            }
            PptRecordData::OfficeArt(record) => visit_record(record, path, audit),
            _ => {}
        }
    }
}

fn visit_stream(stream: &OfficeArtStream, path: &Path, audit: &mut Audit) {
    stream.visit(|record| inspect_record(record, path, audit));
}

fn visit_bstore_delay(stream: &OfficeArtBStoreDelay, path: &Path, audit: &mut Audit) {
    for record in &stream.records {
        visit_record(record, path, audit);
    }
}

fn visit_partial_stream(stream: &OfficeArtPartialStream, path: &Path, audit: &mut Audit) {
    stream.visit_complete(|record| inspect_record(record, path, audit));
}

fn visit_record(record: &OfficeArtRecord, path: &Path, audit: &mut Audit) {
    inspect_record(record, path, audit);
    match &record.data {
        OfficeArtRecordData::Container(children)
        | OfficeArtRecordData::CompatibilityContainer(children) => {
            for child in children {
                visit_record(child, path, audit);
            }
        }
        OfficeArtRecordData::Fbse(value) => {
            if let Some(blip) = value.embedded_blip.as_deref() {
                visit_record(blip, path, audit);
            }
        }
        _ => {}
    }
}

fn inspect_record(record: &OfficeArtRecord, path: &Path, audit: &mut Audit) {
    match &record.data {
        OfficeArtRecordData::MetafileBlip(value) => match &value.file_data {
            OfficeArtMetafileData::Emf(data) => check_emf(data.decoded(), path, audit),
            OfficeArtMetafileData::Wmf(data) => check_wmf(data.decoded(), path, audit),
            OfficeArtMetafileData::Pict(_) | OfficeArtMetafileData::Opaque { .. } => {}
        },
        OfficeArtRecordData::BitmapBlip(value) => {
            if let OfficeArtBitmapData::Dib(bytes) = &value.file_data {
                check_dib(bytes, path, audit);
            }
        }
        _ => {}
    }
}

fn check_emf(bytes: &[u8], path: &Path, audit: &mut Audit) {
    audit.emf += 1;
    audit.emf_bytes += bytes.len();
    match emfsdk_test::roundtrip_metafile_bytes(bytes) {
        Ok(report) => add_roundtrip_report(audit, report),
        Err(error) => audit
            .failures
            .push(format!("{}: EMF round-trip: {error}", path.display())),
    }
}

fn check_wmf(bytes: &[u8], path: &Path, audit: &mut Audit) {
    audit.wmf += 1;
    audit.wmf_bytes += bytes.len();
    match emfsdk_test::roundtrip_metafile_bytes(bytes) {
        Ok(report) => add_roundtrip_report(audit, report),
        Err(error) => audit
            .failures
            .push(format!("{}: WMF round-trip: {error}", path.display())),
    }
}

fn add_roundtrip_report(audit: &mut Audit, report: emfsdk_test::RoundtripReport) {
    audit.emf_records += report.emf_records;
    audit.wmf_records += report.wmf_records;
    audit.emf_plus_records += report.emf_plus_records;
    audit.compatible_emf_records += report.compatible_emf_records;
    audit.compatible_wmf_records += report.compatible_wmf_records;
    audit.compatible_emf_plus_records += report.compatible_emf_plus_records;
    audit.unknown_emf_records += report.unknown_emf_records;
    audit.unknown_wmf_records += report.unknown_wmf_records;
    audit.unknown_emf_plus_records += report.unknown_emf_plus_records;
    audit.compatibility_diagnostics += report.compatibility_diagnostics;
}

fn check_dib(bytes: &[u8], path: &Path, audit: &mut Audit) {
    audit.dib += 1;
    audit.dib_bytes += bytes.len();
    match DeviceIndependentBitmap::from_packed_slice(bytes, DibColorUsage::RgbColors)
        .and_then(|value| value.to_packed_bytes())
    {
        Ok(rebuilt) if rebuilt == bytes => {}
        Ok(rebuilt) => audit.failures.push(format!(
            "{}: DIB bytes changed ({} -> {})",
            path.display(),
            bytes.len(),
            rebuilt.len()
        )),
        Err(error) => audit
            .failures
            .push(format!("{}: DIB round-trip: {error}", path.display())),
    }
}

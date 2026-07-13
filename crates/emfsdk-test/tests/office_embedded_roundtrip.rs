use std::{fs, path::Path};

use emfsdk::{DeviceIndependentBitmap, DibColorUsage, EmfMetafile, WmfMetafile};
use olecfsdk::{
    cfb::CompoundFile,
    office_art::{
        OfficeArtBitmapData, OfficeArtMetafileData, OfficeArtPartialStream, OfficeArtRecord,
        OfficeArtRecordData, OfficeArtStream,
    },
    ppt::{BinaryTagData, PicturesStream, PowerPointDocument, PptRecordData, PptRecordSequence},
    xls::{BiffRecordData, BiffStream, BkHimImage, MsoDrawingData, MsoDrawingRecord},
};
use walkdir::WalkDir;

#[derive(Default)]
struct Audit {
    compound_files: usize,
    workbook_streams: usize,
    powerpoint_streams: usize,
    pictures_streams: usize,
    emf: usize,
    wmf: usize,
    dib: usize,
    emf_bytes: usize,
    wmf_bytes: usize,
    dib_bytes: usize,
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
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(compound) = CompoundFile::from_bytes(&bytes) else {
            continue;
        };
        audit.compound_files += 1;
        for entry in compound.entries().iter().filter(|entry| entry.is_stream()) {
            if entry.name.eq_ignore_ascii_case("Workbook")
                || entry.name.eq_ignore_ascii_case("Book")
            {
                if let Ok(workbook) = BiffStream::from_bytes(&entry.data) {
                    audit.workbook_streams += 1;
                    visit_workbook(&workbook, &path, &mut audit);
                }
            } else if entry.name.eq_ignore_ascii_case("PowerPoint Document") {
                if let Ok(powerpoint) = PowerPointDocument::from_bytes(&entry.data) {
                    audit.powerpoint_streams += 1;
                    visit_ppt_sequence(&powerpoint.records, &path, &mut audit);
                }
            } else if entry.name.eq_ignore_ascii_case("Pictures")
                && let Ok(pictures) = PicturesStream::from_bytes(&entry.data)
            {
                audit.pictures_streams += 1;
                match &pictures {
                    PicturesStream::Complete(stream) => visit_stream(stream, &path, &mut audit),
                    PicturesStream::Partial(stream) => {
                        visit_partial_stream(stream, &path, &mut audit);
                    }
                }
            }
        }
    }

    assert_eq!(audit.compound_files, 898, "Office CFB coverage changed");
    assert_eq!(audit.workbook_streams, 723, "BIFF coverage changed");
    assert_eq!(audit.powerpoint_streams, 179, "PPT coverage changed");
    assert_eq!(audit.pictures_streams, 87, "Pictures coverage changed");
    assert_eq!(audit.emf, 134, "embedded EMF coverage changed");
    assert_eq!(audit.emf_bytes, 6_486_764, "embedded EMF bytes changed");
    assert_eq!(audit.wmf, 156, "embedded WMF coverage changed");
    assert_eq!(audit.wmf_bytes, 8_023_392, "embedded WMF bytes changed");
    assert_eq!(audit.dib, 50, "embedded DIB coverage changed");
    assert_eq!(audit.dib_bytes, 80_820, "embedded DIB bytes changed");
    assert!(
        audit.failures.is_empty(),
        "{} embedded Office image round-trip failures:\n{}",
        audit.failures.len(),
        audit.failures.join("\n")
    );
    eprintln!(
        "Office embedded image corpus: {} CFB files, {} Workbook/{} PowerPoint/{} Pictures streams; {} EMF/{} bytes, {} WMF/{} bytes, {} DIB/{} bytes",
        audit.compound_files,
        audit.workbook_streams,
        audit.powerpoint_streams,
        audit.pictures_streams,
        audit.emf,
        audit.emf_bytes,
        audit.wmf,
        audit.wmf_bytes,
        audit.dib,
        audit.dib_bytes,
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
            OfficeArtMetafileData::Emf { decoded, .. } => check_emf(decoded, path, audit),
            OfficeArtMetafileData::Wmf { decoded, .. } => check_wmf(decoded, path, audit),
            OfficeArtMetafileData::Pict { .. } | OfficeArtMetafileData::Opaque { .. } => {}
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
    match EmfMetafile::from_bytes(bytes).and_then(|value| value.to_bytes()) {
        Ok(rebuilt) if rebuilt == bytes => {}
        Ok(rebuilt) => audit.failures.push(format!(
            "{}: EMF bytes changed ({} -> {})",
            path.display(),
            bytes.len(),
            rebuilt.len()
        )),
        Err(error) => audit
            .failures
            .push(format!("{}: EMF round-trip: {error}", path.display())),
    }
}

fn check_wmf(bytes: &[u8], path: &Path, audit: &mut Audit) {
    audit.wmf += 1;
    audit.wmf_bytes += bytes.len();
    match WmfMetafile::from_bytes(bytes).and_then(|value| value.to_bytes()) {
        Ok(rebuilt) if rebuilt == bytes => {}
        Ok(rebuilt) => audit.failures.push(format!(
            "{}: WMF bytes changed ({} -> {})",
            path.display(),
            bytes.len(),
            rebuilt.len()
        )),
        Err(error) => audit
            .failures
            .push(format!("{}: WMF round-trip: {error}", path.display())),
    }
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

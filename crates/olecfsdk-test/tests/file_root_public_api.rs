use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use olecfsdk::{
    SaveOptions,
    cfb::CompoundFile,
    doc::{DocFile, TextPieceCharacters},
    ppt::{PptFile, PptRecordData, PptSlideId},
    xls::{XlStringCharacters, XlsFile, XlsSheetId, XlsStreamName},
};
use olecfsdk_corpus_test_support::corpus_file_path;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempOutput(PathBuf);

impl TempOutput {
    fn new(extension: &str) -> Self {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Self(std::env::temp_dir().join(format!(
            "olecfsdk-public-api-{}-{sequence}.{extension}",
            std::process::id()
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempOutput {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.0)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            panic!("remove temporary output {}: {error}", self.0.display());
        }
    }
}

#[test]
fn doc_public_file_root_opens_edits_saves_and_reopens() {
    let path = fixture("Apache-POI/test-data/document/simple.doc");
    let bytes = fs::read(&path).expect("read DOC fixture");
    let opened = DocFile::open(&path).expect("open DOC fixture from a path");
    let from_bytes = DocFile::from_bytes(&bytes).expect("open DOC fixture from bytes");
    let compound = CompoundFile::from_bytes(&bytes).expect("open DOC fixture as CFB");
    let mut file =
        DocFile::from_compound_file(compound).expect("open DOC fixture from an owned CFB");

    assert!(
        opened
            .source_compound_file()
            .logical_eq(from_bytes.source_compound_file())
    );
    assert!(
        from_bytes
            .source_compound_file()
            .logical_eq(file.source_compound_file())
    );
    let content = file
        .content_tree()
        .expect("traverse strict DOC content tree");
    assert_eq!(content.parts().len(), 7);
    assert!(
        content
            .parts()
            .iter()
            .any(|part| part.text_pieces().count() > 0)
    );

    let source_snapshot = file.source_compound_file().clone();
    let (cp, replacement, expected) = first_editable_doc_character(&file);
    let incompatible_replacement = match &replacement {
        TextPieceCharacters::Compressed(_) => TextPieceCharacters::Utf16(vec![expected]),
        TextPieceCharacters::Utf16(_) => {
            TextPieceCharacters::Compressed(vec![u8::try_from(expected).unwrap_or(b'X')])
        }
    };
    let before_failed_edit = file.clone();
    assert!(
        file.replace_main_text_range(cp..cp + 1, incompatible_replacement)
            .is_err()
    );
    assert_eq!(file, before_failed_edit);
    file.replace_main_text_range(cp..cp + 1, replacement)
        .expect("edit DOC text through the file-root API");
    assert!(source_snapshot.logical_eq(file.source_compound_file()));

    let current = file
        .to_compound_file_with_options(SaveOptions::default())
        .expect("serialize current DOC tree to CFB");
    assert!(!source_snapshot.logical_eq(&current));
    let serialized = file
        .to_bytes_with_options(SaveOptions::default())
        .expect("serialize current DOC tree to bytes");
    let reopened = DocFile::from_bytes(&serialized).expect("reopen edited DOC bytes strictly");
    assert_eq!(doc_character_at(&reopened, cp), expected);
    assert!(current.logical_eq(reopened.source_compound_file()));

    let output = TempOutput::new("doc");
    file.save_with_options(output.path(), SaveOptions::default())
        .expect("save edited DOC path strictly");
    let saved = DocFile::open(output.path()).expect("reopen edited DOC path strictly");
    assert_eq!(doc_character_at(&saved, cp), expected);
    assert_second_cycle_logically_stable(
        saved.source_compound_file(),
        &saved.to_bytes().expect("save DOC a second time"),
    );
}

#[test]
fn xls_public_file_root_opens_edits_saves_and_reopens() {
    let path = fixture("Apache-POI/test-data/spreadsheet/Simple.xls");
    let bytes = fs::read(&path).expect("read XLS fixture");
    let opened = XlsFile::open(&path).expect("open XLS fixture from a path");
    let from_bytes = XlsFile::from_bytes(&bytes).expect("open XLS fixture from bytes");
    let compound = CompoundFile::from_bytes(&bytes).expect("open XLS fixture as CFB");
    let mut file =
        XlsFile::from_compound_file(compound).expect("open XLS fixture from an owned CFB");

    assert!(
        opened
            .source_compound_file()
            .logical_eq(from_bytes.source_compound_file())
    );
    assert!(
        from_bytes
            .source_compound_file()
            .logical_eq(file.source_compound_file())
    );
    let (workbook_name, sheet_id, edited_name) = xls_sheet_name_edit(&file);
    let source_snapshot = file.source_compound_file().clone();
    let mut invalid_name = edited_name.clone();
    match &mut invalid_name.characters {
        XlStringCharacters::Compressed(characters) => characters.clear(),
        XlStringCharacters::Unicode(characters) => characters.clear(),
    }
    let before_failed_edit = file.clone();
    assert!(
        file.set_sheet_name(workbook_name, sheet_id, invalid_name)
            .is_err()
    );
    assert_eq!(file, before_failed_edit);
    file.set_sheet_name(workbook_name, sheet_id, edited_name.clone())
        .expect("edit XLS sheet name through the file-root API");
    assert!(source_snapshot.logical_eq(file.source_compound_file()));

    let current = file
        .to_compound_file_with_options(SaveOptions::default())
        .expect("serialize current XLS tree to CFB");
    assert!(!source_snapshot.logical_eq(&current));
    let serialized = file
        .to_bytes_with_options(SaveOptions::default())
        .expect("serialize current XLS tree to bytes");
    let reopened = XlsFile::from_bytes(&serialized).expect("reopen edited XLS bytes strictly");
    assert_eq!(
        xls_sheet_name(&reopened, workbook_name, sheet_id),
        edited_name
    );
    assert!(current.logical_eq(reopened.source_compound_file()));

    let output = TempOutput::new("xls");
    file.save_with_options(output.path(), SaveOptions::default())
        .expect("save edited XLS path strictly");
    let saved = XlsFile::open(output.path()).expect("reopen edited XLS path strictly");
    assert_eq!(xls_sheet_name(&saved, workbook_name, sheet_id), edited_name);
    assert_second_cycle_logically_stable(
        saved.source_compound_file(),
        &saved.to_bytes().expect("save XLS a second time"),
    );
}

#[test]
fn ppt_public_file_root_opens_edits_saves_and_reopens() {
    let path = fixture("Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt");
    let bytes = fs::read(&path).expect("read PPT fixture");
    let opened = PptFile::open(&path).expect("open PPT fixture from a path");
    let from_bytes = PptFile::from_bytes(&bytes).expect("open PPT fixture from bytes");
    let compound = CompoundFile::from_bytes(&bytes).expect("open PPT fixture as CFB");
    let mut file =
        PptFile::from_compound_file(compound).expect("open PPT fixture from an owned CFB");

    assert!(
        opened
            .source_compound_file()
            .logical_eq(from_bytes.source_compound_file())
    );
    assert!(
        from_bytes
            .source_compound_file()
            .logical_eq(file.source_compound_file())
    );
    let (slide_id, body_index, unicode, replacement) = ppt_text_edit(&file);
    let source_snapshot = file.source_compound_file().clone();
    let before_failed_edit = file.clone();
    let failed: olecfsdk::Result<()> = file.edit_slide_text_body(slide_id, body_index, |_| {
        Err(olecfsdk::Error::invalid(
            0,
            "intentional public API rollback",
        ))
    });
    assert!(failed.is_err());
    assert_eq!(file, before_failed_edit);
    file.edit_slide_text_body(slide_id, body_index, |mut body| {
        replace_ppt_text_body_first_unit(&mut body, unicode, replacement)
    })
    .expect("edit PPT text through the file-root API");
    assert!(source_snapshot.logical_eq(file.source_compound_file()));

    let current = file
        .to_compound_file_with_options(SaveOptions::default())
        .expect("serialize current PPT tree to CFB");
    assert!(!source_snapshot.logical_eq(&current));
    let serialized = file
        .to_bytes_with_options(SaveOptions::default())
        .expect("serialize current PPT tree to bytes");
    let reopened = PptFile::from_bytes(&serialized).expect("reopen edited PPT bytes strictly");
    assert_eq!(
        ppt_text_body_first_unit(&reopened, slide_id, body_index, unicode),
        replacement
    );
    assert!(current.logical_eq(reopened.source_compound_file()));

    let output = TempOutput::new("ppt");
    file.save_with_options(output.path(), SaveOptions::default())
        .expect("save edited PPT path strictly");
    let saved = PptFile::open(output.path()).expect("reopen edited PPT path strictly");
    assert_eq!(
        ppt_text_body_first_unit(&saved, slide_id, body_index, unicode),
        replacement
    );
    assert_second_cycle_logically_stable(
        saved.source_compound_file(),
        &saved.to_bytes().expect("save PPT a second time"),
    );
}

#[test]
fn compatible_doc_input_is_diagnosed_then_canonicalized_by_strict_save() {
    let path = fixture("Apache-POI/test-data/document/Bug44431.doc");
    let bytes = fs::read(&path).expect("read compatible DOC fixture");
    assert!(DocFile::from_bytes(&bytes).is_err());

    let opened = DocFile::open_compatible(&path).expect("open compatible DOC path");
    let outcome = DocFile::from_bytes_compatible(&bytes).expect("open compatible DOC bytes");
    assert!(!outcome.diagnostics.is_empty());
    assert_eq!(opened.diagnostics, outcome.diagnostics);

    // The diagnostic belongs to the physical source CFB. Rebuilding the file
    // canonicalizes that CFB, so no compatibility payload needs preserving.
    let canonical = outcome
        .value
        .to_bytes_with_options(SaveOptions::default())
        .expect("strict DOC save canonicalizes the source CFB");
    DocFile::from_bytes(&canonical).expect("canonicalized DOC reopens strictly");
}

#[test]
fn compatible_doc_nodes_require_an_explicit_preserving_save_policy() {
    let path = fixture("Apache-POI/test-data/document/47304.doc");
    let bytes = fs::read(&path).expect("read compatible DOC fixture");
    assert!(DocFile::from_bytes(&bytes).is_err());

    let opened = DocFile::open_compatible(&path).expect("open compatible DOC path");
    let outcome = DocFile::from_bytes_compatible(&bytes).expect("open compatible DOC bytes");
    assert!(!outcome.diagnostics.is_empty());
    assert_eq!(opened.diagnostics, outcome.diagnostics);
    assert!(
        outcome
            .value
            .to_bytes_with_options(SaveOptions::default())
            .is_err()
    );

    let strict_output = TempOutput::new("doc");
    fs::write(strict_output.path(), b"do not overwrite").expect("seed DOC output sentinel");
    assert!(
        outcome
            .value
            .save_with_options(strict_output.path(), SaveOptions::default())
            .is_err()
    );
    assert_eq!(
        fs::read(strict_output.path()).expect("read DOC output sentinel"),
        b"do not overwrite"
    );

    let preserving = SaveOptions::preserving_compatibility();
    let serialized = outcome
        .value
        .to_bytes_with_options(preserving)
        .expect("preserve explicitly compatible DOC nodes");
    DocFile::from_bytes_compatible(&serialized).expect("reopen preserved DOC bytes");
    let output = TempOutput::new("doc");
    outcome
        .value
        .save_with_options(output.path(), preserving)
        .expect("save compatible DOC path with explicit policy");
    DocFile::open_compatible(output.path()).expect("reopen preserved DOC path");
}

#[test]
fn compatible_xls_nodes_require_an_explicit_preserving_save_policy() {
    let path = fixture("Apache-POI/test-data/spreadsheet/blankworkbook.xls");
    let bytes = fs::read(&path).expect("read compatible XLS fixture");
    assert!(XlsFile::from_bytes(&bytes).is_err());

    let opened = XlsFile::open_compatible(&path).expect("open compatible XLS path");
    let outcome = XlsFile::from_bytes_compatible(&bytes).expect("open compatible XLS bytes");
    assert!(!outcome.diagnostics.is_empty());
    assert_eq!(opened.diagnostics, outcome.diagnostics);
    assert!(
        outcome
            .value
            .to_bytes_with_options(SaveOptions::default())
            .is_err()
    );

    let strict_output = TempOutput::new("xls");
    fs::write(strict_output.path(), b"do not overwrite").expect("seed XLS output sentinel");
    assert!(
        outcome
            .value
            .save_with_options(strict_output.path(), SaveOptions::default())
            .is_err()
    );
    assert_eq!(
        fs::read(strict_output.path()).expect("read XLS output sentinel"),
        b"do not overwrite"
    );

    let preserving = SaveOptions::preserving_compatibility();
    let serialized = outcome
        .value
        .to_bytes_with_options(preserving)
        .expect("preserve explicitly compatible XLS nodes");
    XlsFile::from_bytes_compatible(&serialized).expect("reopen preserved XLS bytes");
    let output = TempOutput::new("xls");
    outcome
        .value
        .save_with_options(output.path(), preserving)
        .expect("save compatible XLS path with explicit policy");
    XlsFile::open_compatible(output.path()).expect("reopen preserved XLS path");
}

#[test]
fn compatible_ppt_nodes_require_an_explicit_preserving_save_policy() {
    let path = fixture("Apache-POI/test-data/slideshow/missing_core_records.ppt");
    let bytes = fs::read(&path).expect("read compatible PPT fixture");
    assert!(PptFile::from_bytes(&bytes).is_err());

    let opened = PptFile::open_compatible(&path).expect("open compatible PPT path");
    let outcome = PptFile::from_bytes_compatible(&bytes).expect("open compatible PPT bytes");
    assert!(!outcome.diagnostics.is_empty());
    assert_eq!(opened.diagnostics, outcome.diagnostics);
    assert!(
        outcome
            .value
            .to_bytes_with_options(SaveOptions::default())
            .is_err()
    );

    let strict_output = TempOutput::new("ppt");
    fs::write(strict_output.path(), b"do not overwrite").expect("seed PPT output sentinel");
    assert!(
        outcome
            .value
            .save_with_options(strict_output.path(), SaveOptions::default())
            .is_err()
    );
    assert_eq!(
        fs::read(strict_output.path()).expect("read PPT output sentinel"),
        b"do not overwrite"
    );

    let preserving = SaveOptions::preserving_compatibility();
    let serialized = outcome
        .value
        .to_bytes_with_options(preserving)
        .expect("preserve explicitly compatible PPT nodes");
    PptFile::from_bytes_compatible(&serialized).expect("reopen preserved PPT bytes");
    let output = TempOutput::new("ppt");
    outcome
        .value
        .save_with_options(output.path(), preserving)
        .expect("save compatible PPT path with explicit policy");
    PptFile::open_compatible(output.path()).expect("reopen preserved PPT path");
}

fn fixture(relative_path: &str) -> PathBuf {
    let path = corpus_file_path(relative_path);
    assert!(path.is_file(), "fixture is missing: {}", path.display());
    path
}

fn assert_second_cycle_logically_stable(first: &CompoundFile, bytes: &[u8]) {
    let second = CompoundFile::from_bytes_strict(bytes).expect("reopen second-cycle CFB strictly");
    assert!(first.logical_eq(&second));
}

fn first_editable_doc_character(file: &DocFile) -> (u32, TextPieceCharacters, u16) {
    let main_length = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .expect("DOC main-text character count is nonnegative");
    for piece in &file.word_document.text_pieces {
        let start = u32::try_from(piece.value.cp_start).expect("DOC text-piece CP is nonnegative");
        if start >= main_length {
            break;
        }
        match &piece.value.characters {
            TextPieceCharacters::Compressed(characters) => {
                if let Some((index, character)) = characters
                    .iter()
                    .copied()
                    .enumerate()
                    .find(|(_, character)| character.is_ascii_alphabetic())
                {
                    let edited = if character == b'X' { b'Y' } else { b'X' };
                    return (
                        start + u32::try_from(index).expect("DOC text index fits u32"),
                        TextPieceCharacters::Compressed(vec![edited]),
                        u16::from(edited),
                    );
                }
            }
            TextPieceCharacters::Utf16(characters) => {
                if let Some((index, character)) =
                    characters
                        .iter()
                        .copied()
                        .enumerate()
                        .find(|(_, character)| {
                            char::from_u32(u32::from(*character)).is_some_and(char::is_alphabetic)
                        })
                {
                    let edited = if character == u16::from(b'X') {
                        u16::from(b'Y')
                    } else {
                        u16::from(b'X')
                    };
                    return (
                        start + u32::try_from(index).expect("DOC text index fits u32"),
                        TextPieceCharacters::Utf16(vec![edited]),
                        edited,
                    );
                }
            }
        }
    }
    panic!("DOC fixture has no editable main-text character");
}

fn doc_character_at(file: &DocFile, cp: u32) -> u16 {
    for piece in &file.word_document.text_pieces {
        let start = u32::try_from(piece.value.cp_start).expect("DOC text-piece CP is nonnegative");
        let end = u32::try_from(piece.value.cp_end).expect("DOC text-piece CP is nonnegative");
        if !(start..end).contains(&cp) {
            continue;
        }
        let index = usize::try_from(cp - start).expect("DOC text index fits usize");
        return match &piece.value.characters {
            TextPieceCharacters::Compressed(characters) => u16::from(characters[index]),
            TextPieceCharacters::Utf16(characters) => characters[index],
        };
    }
    panic!("DOC character CP {cp} is missing after reopen");
}

fn xls_sheet_name_edit(
    file: &XlsFile,
) -> (
    XlsStreamName,
    XlsSheetId,
    olecfsdk::xls::ShortXlUnicodeString,
) {
    for workbook in &file.workbooks {
        let relationships = workbook
            .relationships()
            .expect("traverse strict XLS workbook relationships");
        if let Some(sheet) = relationships.sheets().first().copied() {
            let mut name = sheet.metadata().name.clone();
            match &mut name.characters {
                XlStringCharacters::Compressed(characters) if characters.len() < 31 => {
                    characters.push(b'X');
                }
                XlStringCharacters::Unicode(characters) if characters.len() < 31 => {
                    characters.push(u16::from(b'X'));
                }
                XlStringCharacters::Compressed(characters) => characters[0] = b'X',
                XlStringCharacters::Unicode(characters) => characters[0] = u16::from(b'X'),
            }
            return (workbook.name, sheet.id(), name);
        }
    }
    panic!("XLS fixture has no editable sheet");
}

fn xls_sheet_name(
    file: &XlsFile,
    workbook_name: XlsStreamName,
    sheet_id: XlsSheetId,
) -> olecfsdk::xls::ShortXlUnicodeString {
    let workbook = file
        .workbook_stream(workbook_name)
        .expect("XLS Workbook stream remains present");
    workbook
        .relationships()
        .expect("traverse reopened XLS relationships")
        .sheet(sheet_id)
        .expect("edited XLS sheet identity remains present")
        .metadata()
        .name
        .clone()
}

fn ppt_text_edit(file: &PptFile) -> (PptSlideId, usize, bool, u16) {
    let presentation = file
        .live_presentation()
        .expect("traverse strict PPT live presentation");
    for slide in presentation.slides().expect("traverse strict PPT slides") {
        for (body_index, body) in slide.object.text_bodies().iter().enumerate() {
            for record in body.records {
                match &record.data {
                    PptRecordData::TextChars(characters) if !characters.is_empty() => {
                        let replacement = if characters[0] == u16::from(b'X') {
                            u16::from(b'Y')
                        } else {
                            u16::from(b'X')
                        };
                        return (slide.id(), body_index, true, replacement);
                    }
                    PptRecordData::TextBytes(characters) if !characters.is_empty() => {
                        let replacement = if characters[0] == b'X' { b'Y' } else { b'X' };
                        return (slide.id(), body_index, false, u16::from(replacement));
                    }
                    _ => {}
                }
            }
        }
    }
    panic!("PPT fixture has no editable slide text");
}

fn replace_ppt_text_body_first_unit(
    body: &mut olecfsdk::ppt::PptLiveTextBodyMut<'_>,
    unicode: bool,
    replacement: u16,
) -> olecfsdk::Result<()> {
    for record in body.records_mut() {
        match (&mut record.data, unicode) {
            (PptRecordData::TextChars(characters), true) if !characters.is_empty() => {
                characters[0] = replacement;
                return Ok(());
            }
            (PptRecordData::TextBytes(characters), false) if !characters.is_empty() => {
                characters[0] = u8::try_from(replacement)
                    .map_err(|_| olecfsdk::Error::invalid(0, "PPT byte text exceeds u8"))?;
                return Ok(());
            }
            _ => {}
        }
    }
    Err(olecfsdk::Error::invalid(
        0,
        "selected PPT text body changed static text variant",
    ))
}

fn ppt_text_body_first_unit(
    file: &PptFile,
    slide_id: PptSlideId,
    body_index: usize,
    unicode: bool,
) -> u16 {
    let presentation = file
        .live_presentation()
        .expect("traverse reopened PPT presentation");
    let slides = presentation.slides().expect("traverse reopened PPT slides");
    let slide = slides
        .iter()
        .find(|slide| slide.id() == slide_id)
        .expect("edited PPT slide identity remains present");
    let bodies = slide.object.text_bodies();
    bodies[body_index]
        .records
        .iter()
        .find_map(|record| match (&record.data, unicode) {
            (PptRecordData::TextChars(characters), true) => characters.first().copied(),
            (PptRecordData::TextBytes(characters), false) => {
                characters.first().copied().map(u16::from)
            }
            _ => None,
        })
        .expect("edited PPT text atom remains present")
}

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use olecfsdk::{
    Error, Result, SaveOptions,
    cfb::CompoundFile,
    common::CodePage,
    doc::{DocFile, TextPieceCharacters},
    forms::FormPropertyMask,
    ppt::{
        CurrentUserData, ExternalStorageAtom, ExternalStorageVba, PptFile, PptRecordData,
        PptSlideId,
    },
    property_set::{Property, PropertySetStream, PropertyType, TypedPropertyValue},
    shared_content::{OfficePropertySetKind, OfficeSharedContent},
    vba::cache::VbaProjectStream,
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
fn shared_oleps_metadata_is_host_owned_mutable_and_cycle_stable() {
    let doc_path = fixture("Apache-POI/test-data/document/simple.doc");
    let mut doc = DocFile::open(&doc_path).expect("open DOC OLEPS fixture");
    let doc_source = doc.source_compound_file().clone();
    let doc_edit = edit_string_metadata(&mut doc.shared).expect("edit DOC root metadata");
    assert!(doc_source.logical_eq(doc.source_compound_file()));
    let doc_bytes = doc.to_bytes().expect("serialize DOC metadata edit");
    let doc_reopened = DocFile::from_bytes(&doc_bytes).expect("reopen DOC metadata edit strictly");
    assert_metadata_edit(&doc_reopened.shared, &doc_edit);
    assert_second_cycle_logically_stable(
        doc_reopened.source_compound_file(),
        &doc_reopened
            .to_bytes()
            .expect("save DOC metadata second cycle"),
    );

    let xls_path = fixture("Apache-POI/test-data/spreadsheet/Simple.xls");
    let mut xls = XlsFile::open(&xls_path).expect("open XLS OLEPS fixture");
    let xls_source = xls.source_compound_file().clone();
    let xls_edit = edit_string_metadata(&mut xls.shared).expect("edit XLS root metadata");
    assert!(xls_source.logical_eq(xls.source_compound_file()));
    let xls_bytes = xls.to_bytes().expect("serialize XLS metadata edit");
    let xls_reopened = XlsFile::from_bytes(&xls_bytes).expect("reopen XLS metadata edit strictly");
    assert_metadata_edit(&xls_reopened.shared, &xls_edit);
    assert_second_cycle_logically_stable(
        xls_reopened.source_compound_file(),
        &xls_reopened
            .to_bytes()
            .expect("save XLS metadata second cycle"),
    );

    let ppt_path = fixture("Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt");
    let mut ppt = PptFile::open(&ppt_path).expect("open PPT OLEPS fixture");
    let ppt_source = ppt.source_compound_file().clone();
    let ppt_edit = edit_string_metadata(&mut ppt.shared).expect("edit PPT root metadata");
    assert!(ppt_source.logical_eq(ppt.source_compound_file()));
    let ppt_bytes = ppt.to_bytes().expect("serialize PPT metadata edit");
    let ppt_reopened = PptFile::from_bytes(&ppt_bytes).expect("reopen PPT metadata edit strictly");
    assert_metadata_edit(&ppt_reopened.shared, &ppt_edit);
    assert_second_cycle_logically_stable(
        ppt_reopened.source_compound_file(),
        &ppt_reopened
            .to_bytes()
            .expect("save PPT metadata second cycle"),
    );
}

#[test]
fn host_vba_source_mutation_is_transactional_and_cycle_stable() {
    let doc_path = fixture("Apache-POI/test-data/document/SimpleMacro.doc");
    let mut doc = DocFile::open(&doc_path).expect("open macro-enabled DOC strictly");
    let doc_source = doc.source_compound_file().clone();
    let (doc_module, doc_previous) = first_vba_module(&doc.shared);
    let doc_replacement = edited_vba_source(&doc_previous);
    let doc_before_failed_edit = doc.clone();
    assert!(
        doc.replace_vba_module_source("missing-module", b"invalid")
            .is_err()
    );
    assert_eq!(doc, doc_before_failed_edit);
    let doc_report = doc
        .replace_vba_module_source(&doc_module, &doc_replacement)
        .expect("transactionally edit DOC VBA source");
    assert_eq!(doc_report.vba.previous_source, doc_previous);
    assert_eq!(doc_report.invalidated_host_signatures, 0);
    assert!(doc_source.logical_eq(doc.source_compound_file()));
    let doc_bytes = doc.to_bytes().expect("serialize edited DOC VBA project");
    let doc_reopened = DocFile::from_bytes(&doc_bytes).expect("strictly reopen edited DOC VBA");
    assert_vba_interoperable_edit(&doc_reopened.shared, &doc_module, &doc_replacement);
    assert_second_cycle_logically_stable(
        doc_reopened.source_compound_file(),
        &doc_reopened.to_bytes().expect("save DOC VBA second cycle"),
    );

    let xls_path = fixture("Apache-POI/test-data/spreadsheet/SimpleMacro.xls");
    let mut xls_compound =
        CompoundFile::from_bytes(&fs::read(&xls_path).expect("read macro-enabled XLS fixture"))
            .expect("parse macro-enabled XLS CFB");
    insert_test_vba_signature(&mut xls_compound);
    let mut xls =
        XlsFile::from_compound_file(xls_compound).expect("open macro-enabled XLS strictly");
    let xls_source = xls.source_compound_file().clone();
    let (xls_module, xls_previous) = first_vba_module(&xls.shared);
    let xls_replacement = edited_vba_source(&xls_previous);
    let xls_before_failed_edit = xls.clone();
    assert!(
        xls.replace_vba_module_source("missing-module", b"invalid")
            .is_err()
    );
    assert_eq!(xls, xls_before_failed_edit);
    let xls_report = xls
        .replace_vba_module_source(&xls_module, &xls_replacement)
        .expect("transactionally edit XLS VBA source");
    assert_eq!(xls_report.vba.previous_source, xls_previous);
    assert_eq!(xls_report.invalidated_oleps_signatures, 1);
    assert!(xls_source.logical_eq(xls.source_compound_file()));
    let xls_bytes = xls.to_bytes().expect("serialize edited XLS VBA project");
    let xls_reopened = XlsFile::from_bytes(&xls_bytes).expect("strictly reopen edited XLS VBA");
    assert!(!has_test_vba_signature(&xls_reopened.shared));
    assert_vba_interoperable_edit(&xls_reopened.shared, &xls_module, &xls_replacement);
    assert_second_cycle_logically_stable(
        xls_reopened.source_compound_file(),
        &xls_reopened.to_bytes().expect("save XLS VBA second cycle"),
    );

    let ppt_path = fixture("Apache-POI/test-data/slideshow/SimpleMacro.ppt");
    let mut ppt = PptFile::open(&ppt_path).expect("open macro-enabled PPT strictly");
    let ppt_source = ppt.source_compound_file().clone();
    let (ppt_module, ppt_previous) = first_ppt_vba_module(&ppt);
    let ppt_replacement = edited_vba_source(&ppt_previous);
    let ppt_before_failed_edit = ppt.clone();
    assert!(
        ppt.replace_vba_module_source("missing-module", b"invalid")
            .is_err()
    );
    assert_eq!(ppt, ppt_before_failed_edit);
    let ppt_report = ppt
        .replace_vba_module_source(&ppt_module, &ppt_replacement)
        .expect("transactionally edit PPT VBA source");
    assert_eq!(ppt_report.vba.previous_source, ppt_previous);
    assert!(ppt_source.logical_eq(ppt.source_compound_file()));
    let ppt_bytes = ppt.to_bytes().expect("serialize edited PPT VBA project");
    let ppt_reopened = PptFile::from_bytes(&ppt_bytes).expect("strictly reopen edited PPT VBA");
    assert_ppt_vba_interoperable_edit(&ppt_reopened, &ppt_module, &ppt_replacement);
    assert_second_cycle_logically_stable(
        ppt_reopened.source_compound_file(),
        &ppt_reopened.to_bytes().expect("save PPT VBA second cycle"),
    );
}

#[test]
fn host_forms_designer_is_owned_transactional_and_cycle_stable() {
    let path = fixture("Apache-POI/test-data/spreadsheet/15556.xls");
    let source_compound =
        CompoundFile::from_bytes(&fs::read(&path).expect("read Forms XLS fixture"))
            .expect("parse Forms XLS CFB");
    // The real fixture has a nonzero root creation timestamp. Re-emitting the
    // same logical CFB removes that unrelated strict-CFB violation so this test
    // can exercise strict host parsing and saving of the Forms tree itself.
    let forms_source = CompoundFile::from_bytes(
        &source_compound
            .to_bytes()
            .expect("canonicalize Forms fixture CFB metadata"),
    )
    .expect("reopen canonical Forms fixture CFB");
    let mut xls_compound = forms_source.clone();
    insert_test_vba_signature(&mut xls_compound);
    let mut xls = XlsFile::from_compound_file(xls_compound).expect("open Forms XLS strictly");
    let xls_source = xls.source_compound_file().clone();
    let project = xls
        .shared
        .vba_project()
        .and_then(|project| project.project())
        .expect("Forms fixture has a parsed VBA project");
    assert_eq!(project.designer_storages().len(), 1);
    assert_eq!(
        project.designer_storages()[0].identity().path,
        Path::new("/_VBA_PROJECT_CUR/UserForm1")
    );
    assert_eq!(project.designer_storages()[0].model().children.len(), 1);

    let before_failed_edit = xls.clone();
    assert!(xls.edit_vba_designer_storage(1, |_| Ok(())).is_err());
    assert_eq!(xls, before_failed_edit);
    let failed: Result<()> = xls
        .edit_vba_designer_storage(0, |_| {
            Err(Error::invalid(0, "intentional Forms transaction rollback"))
        })
        .map(|_| ());
    assert!(failed.is_err());
    assert_eq!(xls, before_failed_edit);

    let xls_previous_tiling = shared_form_picture_tiling(&xls.shared);
    let report = xls
        .edit_vba_designer_storage(0, toggle_form_picture_tiling)
        .expect("transactionally edit the typed UserForm root");
    assert_eq!(report.invalidated_oleps_signatures, 1);
    assert!(xls_source.logical_eq(xls.source_compound_file()));

    let bytes = xls.to_bytes().expect("serialize edited XLS Forms tree");
    let xls_reopened = XlsFile::from_bytes(&bytes).expect("strictly reopen edited XLS Forms tree");
    assert_eq!(
        shared_form_picture_tiling(&xls_reopened.shared),
        !xls_previous_tiling
    );
    assert!(!has_test_vba_signature(&xls_reopened.shared));
    let reopened_project = xls_reopened
        .shared
        .vba_project()
        .and_then(|project| project.project())
        .expect("reopened Forms fixture has a parsed VBA project");
    assert_eq!(
        reopened_project.designer_storages()[0].identity().path,
        Path::new("/_VBA_PROJECT_CUR/UserForm1")
    );
    assert_second_cycle_logically_stable(
        xls_reopened.source_compound_file(),
        &xls_reopened
            .to_bytes()
            .expect("save Forms XLS second cycle"),
    );

    let doc_path = fixture("Apache-POI/test-data/document/SimpleMacro.doc");
    let mut doc_compound =
        CompoundFile::from_bytes(&fs::read(&doc_path).expect("read macro DOC fixture"))
            .expect("parse macro DOC CFB");
    copy_cfb_storage_tree(
        &forms_source,
        Path::new("/_VBA_PROJECT_CUR/UserForm1"),
        &mut doc_compound,
        Path::new("/Macros/UserForm1"),
    );
    insert_test_vba_signature(&mut doc_compound);
    let mut doc =
        DocFile::from_compound_file(doc_compound).expect("open DOC with UserForm strictly");
    let doc_source = doc.source_compound_file().clone();
    let doc_previous_tiling = shared_form_picture_tiling(&doc.shared);
    assert_eq!(
        doc.shared
            .vba_project()
            .and_then(|project| project.project())
            .expect("DOC has parsed VBA")
            .designer_storages()[0]
            .identity()
            .path,
        Path::new("/Macros/UserForm1")
    );
    let report = doc
        .edit_vba_designer_storage(0, toggle_form_picture_tiling)
        .expect("transactionally edit DOC UserForm");
    assert_eq!(report.invalidated_oleps_signatures, 1);
    assert!(doc_source.logical_eq(doc.source_compound_file()));
    let doc_bytes = doc.to_bytes().expect("serialize DOC Forms tree");
    let doc_reopened = DocFile::from_bytes(&doc_bytes).expect("strictly reopen DOC Forms tree");
    assert_eq!(
        shared_form_picture_tiling(&doc_reopened.shared),
        !doc_previous_tiling
    );
    assert!(!has_test_vba_signature(&doc_reopened.shared));
    assert_second_cycle_logically_stable(
        doc_reopened.source_compound_file(),
        &doc_reopened
            .to_bytes()
            .expect("save Forms DOC second cycle"),
    );

    let ppt_path = fixture("Apache-POI/test-data/slideshow/SimpleMacro.ppt");
    let mut ppt_compound =
        CompoundFile::from_bytes(&fs::read(&ppt_path).expect("read macro PPT fixture"))
            .expect("parse macro PPT CFB");
    insert_test_vba_signature(&mut ppt_compound);
    let mut ppt = PptFile::from_compound_file(ppt_compound).expect("open macro PPT strictly");
    let ppt_source = ppt.source_compound_file().clone();
    let record_index = ppt_vba_record_index(&ppt);
    let mut embedded = match &ppt.document.records.records[record_index].data {
        PptRecordData::ExternalStorage(ExternalStorageAtom::Parsed(storage)) => {
            storage.compound_file().clone()
        }
        _ => panic!("PPT VBA persist object is a parsed external storage"),
    };
    copy_cfb_storage_tree(
        &forms_source,
        Path::new("/_VBA_PROJECT_CUR/UserForm1"),
        &mut embedded,
        Path::new("/UserForm1"),
    );
    ppt.document.records.records[record_index].data = PptRecordData::ExternalStorage(
        ExternalStorageAtom::recompress(embedded).expect("recompress PPT VBA with UserForm"),
    );
    let ppt_previous_tiling = ppt_form_picture_tiling(&ppt);
    assert_eq!(
        ppt_vba_project(&ppt).designer_storages()[0].identity().path,
        Path::new("/UserForm1")
    );
    let report = ppt
        .edit_vba_designer_storage(0, toggle_form_picture_tiling)
        .expect("transactionally edit PPT UserForm");
    assert_eq!(report.invalidated_oleps_signatures, 1);
    assert!(ppt_source.logical_eq(ppt.source_compound_file()));
    let ppt_bytes = ppt.to_bytes().expect("serialize PPT Forms tree");
    let ppt_reopened = PptFile::from_bytes(&ppt_bytes).expect("strictly reopen PPT Forms tree");
    assert_eq!(ppt_form_picture_tiling(&ppt_reopened), !ppt_previous_tiling);
    assert!(!has_test_vba_signature(&ppt_reopened.shared));
    assert_second_cycle_logically_stable(
        ppt_reopened.source_compound_file(),
        &ppt_reopened
            .to_bytes()
            .expect("save Forms PPT second cycle"),
    );
}

fn shared_form_picture_tiling(shared: &OfficeSharedContent) -> bool {
    shared
        .vba_project()
        .and_then(|project| project.project())
        .expect("XLS has a parsed VBA project")
        .designer_storages()
        .first()
        .expect("VBA project has a Designer storage")
        .model()
        .form
        .picture_tiling
}

fn ppt_form_picture_tiling(file: &PptFile) -> bool {
    ppt_vba_project(file).designer_storages()[0]
        .model()
        .form
        .picture_tiling
}

fn toggle_form_picture_tiling(
    model: &mut olecfsdk::forms::ParentControlStorageModel,
) -> Result<()> {
    model.form.picture_tiling = !model.form.picture_tiling;
    model
        .form
        .property_mask
        .toggle(FormPropertyMask::PICTURE_TILING);
    Ok(())
}

fn copy_cfb_storage_tree(
    source: &CompoundFile,
    source_root: &Path,
    destination: &mut CompoundFile,
    destination_root: &Path,
) {
    for entry in source
        .walk_storage(source_root)
        .expect("walk Forms storage tree")
    {
        let relative = entry
            .path
            .strip_prefix(source_root)
            .expect("Forms entry is below source root");
        let path = destination_root.join(relative);
        if entry.is_storage() {
            destination
                .create_storage(&path)
                .expect("create copied Forms storage");
            destination
                .replace_storage_class_id(&path, entry.clsid)
                .expect("copy Forms storage CLSID");
            destination
                .replace_creation_time(&path, entry.created)
                .expect("copy Forms storage creation time");
            destination
                .replace_modified_time(&path, entry.modified)
                .expect("copy Forms storage modified time");
        } else {
            destination
                .create_stream(&path, entry.data.clone())
                .expect("create copied Forms stream");
        }
        destination
            .replace_state_bits(&path, entry.state_bits)
            .expect("copy Forms entry state bits");
    }
}

const DOCUMENT_SUMMARY_FMTID: [u8; 16] = [
    0x02, 0xd5, 0xcd, 0xd5, 0x9c, 0x2e, 0x1b, 0x10, 0x93, 0x97, 0x08, 0x00, 0x2b, 0x2c, 0xf9, 0xae,
];

fn insert_test_vba_signature(compound: &mut CompoundFile) {
    let path = olecfsdk::shared_content::DOCUMENT_SUMMARY_INFORMATION_STREAM;
    let mut stream = PropertySetStream::from_bytes(
        compound
            .stream(path)
            .expect("macro XLS has DocumentSummaryInformation"),
    )
    .expect("parse macro XLS DocumentSummaryInformation");
    let property_set = stream
        .property_sets
        .iter_mut()
        .find(|property_set| property_set.format_identifier == DOCUMENT_SUMMARY_FMTID)
        .expect("DocumentSummaryInformation contains PIDDSI");
    assert!(
        property_set
            .properties
            .iter()
            .all(|property| property.identifier != 0x18)
    );
    property_set.properties.push(Property {
        identifier: 0x18,
        offset: 0,
        raw: TypedPropertyValue::Blob {
            property_type: PropertyType::BLOB,
            reserved: 0,
            bytes: vec![1, 2, 3, 4],
            padding: Vec::new(),
        }
        .to_bytes()
        .expect("encode test VtDigSig property"),
    });
    compound
        .replace_stream(path, stream.to_bytes().expect("encode signed PIDDSI"))
        .expect("replace signed DocumentSummaryInformation");
}

fn has_test_vba_signature(shared: &OfficeSharedContent) -> bool {
    shared
        .property_set(OfficePropertySetKind::DocumentSummaryInformation)
        .is_some_and(|stream| {
            stream.property_sets.iter().any(|property_set| {
                property_set.format_identifier == DOCUMENT_SUMMARY_FMTID
                    && property_set
                        .properties
                        .iter()
                        .any(|property| property.identifier == 0x18)
            })
        })
}

fn first_vba_module(shared: &OfficeSharedContent) -> (String, Vec<u8>) {
    let project = shared
        .vba_project()
        .and_then(|project| project.project())
        .expect("fixture has one parsed host VBA project");
    let model = project.model();
    let module = model.modules.first().expect("VBA project has a module");
    let name = module
        .descriptor
        .stream_name_with_code_page(CodePage(model.directory.code_page().unwrap_or(1252)))
        .expect("decode VBA module stream name");
    let source = module.stream.source_bytes().expect("decompress VBA source");
    (name, source)
}

fn edited_vba_source(previous: &[u8]) -> Vec<u8> {
    let mut edited = previous.to_vec();
    edited.extend_from_slice(b"\r\n' olecfsdk host mutation\r\n");
    edited
}

fn assert_vba_interoperable_edit(
    shared: &OfficeSharedContent,
    module_name: &str,
    expected_source: &[u8],
) {
    let project = shared
        .vba_project()
        .and_then(|project| project.project())
        .expect("reopened host VBA project is parsed");
    let model = project.model();
    assert_eq!(model.cache.version, VbaProjectStream::INTEROPERABLE_VERSION);
    assert!(model.cache.performance_cache.is_empty());
    assert!(model.srp_caches.is_empty());
    assert!(
        model
            .modules
            .iter()
            .all(|module| module.stream.performance_cache.is_empty())
    );
    assert!(model.directory.module_offsets().all(|offset| offset == 0));
    let code_page = CodePage(model.directory.code_page().unwrap_or(1252));
    let module = model
        .modules
        .iter()
        .find(|module| {
            module
                .descriptor
                .stream_name_with_code_page(code_page)
                .is_ok_and(|name| name.eq_ignore_ascii_case(module_name))
        })
        .expect("reopened edited VBA module exists");
    assert_eq!(
        module
            .stream
            .source_bytes()
            .expect("decompress edited VBA source"),
        expected_source
    );
}

fn first_ppt_vba_module(file: &PptFile) -> (String, Vec<u8>) {
    let project = ppt_vba_project(file);
    let model = project.model();
    let module = model.modules.first().expect("PPT VBA project has a module");
    let code_page = CodePage(model.directory.code_page().unwrap_or(1252));
    (
        module
            .descriptor
            .stream_name_with_code_page(code_page)
            .expect("decode PPT VBA module name"),
        module
            .stream
            .source_bytes()
            .expect("decompress PPT VBA module source"),
    )
}

fn ppt_vba_project(file: &PptFile) -> &olecfsdk::vba::LocatedVbaProject {
    let CurrentUserData::Parsed(current_user) = &file.current_user.data else {
        panic!("PPT CurrentUserAtom is parsed");
    };
    let live = file
        .document
        .live_presentation(current_user)
        .expect("resolve PPT live presentation");
    let record = live.vba_project.expect("PPT has a live VBA project").record;
    let PptRecordData::ExternalStorage(ExternalStorageAtom::Parsed(storage)) = &record.data else {
        panic!("PPT VBA persist object is a parsed external storage");
    };
    let ExternalStorageVba::Parsed(project) = storage.vba_project() else {
        panic!("PPT external storage contains a parsed VBA project");
    };
    project
}

fn ppt_vba_record_index(file: &PptFile) -> usize {
    let CurrentUserData::Parsed(current_user) = &file.current_user.data else {
        panic!("PPT CurrentUserAtom is parsed");
    };
    file.document
        .live_presentation(current_user)
        .expect("resolve PPT live presentation")
        .vba_project
        .expect("PPT has a live VBA project")
        .reference
        .record_index
}

fn assert_ppt_vba_interoperable_edit(file: &PptFile, module_name: &str, expected_source: &[u8]) {
    let project = ppt_vba_project(file);
    let model = project.model();
    assert_eq!(model.cache.version, VbaProjectStream::INTEROPERABLE_VERSION);
    assert!(model.cache.performance_cache.is_empty());
    assert!(model.srp_caches.is_empty());
    assert!(
        model
            .modules
            .iter()
            .all(|module| module.stream.performance_cache.is_empty())
    );
    assert!(model.directory.module_offsets().all(|offset| offset == 0));
    let code_page = CodePage(model.directory.code_page().unwrap_or(1252));
    let module = model
        .modules
        .iter()
        .find(|module| {
            module
                .descriptor
                .stream_name_with_code_page(code_page)
                .is_ok_and(|name| name.eq_ignore_ascii_case(module_name))
        })
        .expect("reopened PPT VBA module exists");
    assert_eq!(
        module
            .stream
            .source_bytes()
            .expect("decompress edited PPT VBA source"),
        expected_source
    );
}

#[derive(Clone, Debug)]
struct MetadataEdit {
    kind: OfficePropertySetKind,
    property_set_index: usize,
    property_index: usize,
    expected_raw: Vec<u8>,
}

fn edit_string_metadata(shared: &mut OfficeSharedContent) -> Result<MetadataEdit> {
    let candidate = [
        OfficePropertySetKind::SummaryInformation,
        OfficePropertySetKind::DocumentSummaryInformation,
    ]
    .into_iter()
    .find_map(|kind| {
        shared.property_set(kind).and_then(|stream| {
            stream.property_sets.iter().enumerate().find_map(
                |(property_set_index, property_set)| {
                    property_set.properties.iter().enumerate().find_map(
                        |(property_index, property)| {
                            matches!(
                                property.typed_value(),
                                Ok(TypedPropertyValue::CodePageString { .. }
                                    | TypedPropertyValue::UnicodeString { .. })
                            )
                            .then_some((
                                kind,
                                property_set_index,
                                property_index,
                            ))
                        },
                    )
                },
            )
        })
    })
    .ok_or_else(|| Error::invalid(0, "fixture has no editable OLEPS string property"))?;
    let (kind, property_set_index, property_index) = candidate;
    let expected_raw = shared.edit_property_set(kind, |stream| {
        let property = &mut stream.property_sets[property_set_index].properties[property_index];
        let mut value = property.typed_value()?;
        match &mut value {
            TypedPropertyValue::CodePageString { bytes, padding, .. } => {
                let terminated = bytes.last() == Some(&0);
                *bytes = b"olecfsdk metadata".to_vec();
                if terminated {
                    bytes.push(0);
                }
                *padding = vec![0; (4 - ((8 + bytes.len()) % 4)) % 4];
            }
            TypedPropertyValue::UnicodeString {
                code_units,
                padding,
                ..
            } => {
                let terminated = code_units.last() == Some(&0);
                *code_units = "olecfsdk metadata".encode_utf16().collect();
                if terminated {
                    code_units.push(0);
                }
                *padding = vec![0; (4 - ((8 + code_units.len() * 2) % 4)) % 4];
            }
            _ => return Err(Error::invalid(0, "selected OLEPS property is not a string")),
        }
        property.raw = value.to_bytes()?;
        Ok(property.raw.clone())
    })?;
    Ok(MetadataEdit {
        kind,
        property_set_index,
        property_index,
        expected_raw,
    })
}

fn assert_metadata_edit(shared: &OfficeSharedContent, edit: &MetadataEdit) {
    let stream = shared
        .property_set(edit.kind)
        .expect("reopened property-set stream exists");
    assert_eq!(
        stream.property_sets[edit.property_set_index].properties[edit.property_index].raw,
        edit.expected_raw
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

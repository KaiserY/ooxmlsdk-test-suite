use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use olecfsdk::{
    Error, Result, SaveOptions,
    cfb::CompoundFile,
    common::CodePage,
    doc::{
        DocBlockRef, DocCp, DocDataNodeValue, DocFile, DocOutlineLevel, FieldDocumentPart,
        TextPieceCharacters, TextPieceEncoding,
    },
    forms::{FormPropertyMask, LocatedParentControlStorage},
    ppt::{
        CurrentUserData, ExternalStorageAtom, ExternalStorageVba, PptFile, PptLiveNotesLink,
        PptLiveTextAtomRef, PptPlaceholderType, PptRecordData, PptSlideId, PptTextEncoding,
        PptTextType,
    },
    property_set::{Property, PropertySetStream, PropertyType, TypedPropertyValue},
    shared_content::{OfficePropertySetKind, OfficeSharedContent},
    vba::cache::VbaProjectStream,
    xls::{
        FormulaElfExtraLocation, FormulaElfLocation, FormulaNaturalLanguageToken, FormulaTokenData,
        FormulaTokenStream, ObjPictureFlags, ShortXlUnicodeString, XlsCellValue, XlsFile,
        XlsFormulaCachedValue, XlsHyperlinkTarget, XlsNumberFormatRef, XlsObjectPersistenceRef,
        XlsSheetId, XlsStreamName,
    },
};
use olecfsdk_corpus_test_support::corpus_file_path;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn well_known_office_cfb_names_are_public_static_sdk_values() {
    use olecfsdk::{
        doc::{
            DATA_STREAM_PATH, DocTableStreamName, OBJECT_INFO_STREAM_NAME,
            OBJECT_POOL_STORAGE_PATH, TABLE0_STREAM_PATH, TABLE1_STREAM_PATH,
            WORD_DOCUMENT_STREAM_PATH,
        },
        forms::{FORM_STREAM_NAME, MULTIPAGE_STREAM_NAME, OBJECT_STREAM_NAME},
        ppt::{CURRENT_USER_STREAM_PATH, PICTURES_STREAM_PATH, POWERPOINT_DOCUMENT_STREAM_PATH},
        vba::{
            DOC_VBA_PROJECT_STORAGE_NAME, VBA_DIRECTORY_STREAM_NAME, VBA_PROJECT_CACHE_STREAM_NAME,
            VBA_PROJECT_LK_STREAM_NAME, VBA_PROJECT_STREAM_NAME, VBA_PROJECT_WM_STREAM_NAME,
            VBA_STORAGE_NAME, XLS_VBA_PROJECT_STORAGE_NAME,
        },
        xls::{
            BOOK_STREAM_PATH, PIVOT_CACHE_STORAGE_NAME, PIVOT_CACHE_STORAGE_PATH,
            REVISION_LOG_STREAM_PATH, USER_NAMES_STREAM_PATH, WORKBOOK_STREAM_PATH, XlsStreamName,
        },
    };

    assert_eq!(WORD_DOCUMENT_STREAM_PATH, "/WordDocument");
    assert_eq!(DocTableStreamName::Table0.path(), TABLE0_STREAM_PATH);
    assert_eq!(DocTableStreamName::Table1.path(), TABLE1_STREAM_PATH);
    assert_eq!(DATA_STREAM_PATH, "/Data");
    assert_eq!(OBJECT_POOL_STORAGE_PATH, "/ObjectPool");
    assert_eq!(OBJECT_INFO_STREAM_NAME, "\u{3}ObjInfo");

    assert_eq!(POWERPOINT_DOCUMENT_STREAM_PATH, "/PowerPoint Document");
    assert_eq!(CURRENT_USER_STREAM_PATH, "/Current User");
    assert_eq!(PICTURES_STREAM_PATH, "/Pictures");

    assert_eq!(XlsStreamName::Workbook.path(), WORKBOOK_STREAM_PATH);
    assert_eq!(XlsStreamName::Book.path(), BOOK_STREAM_PATH);
    assert_eq!(PIVOT_CACHE_STORAGE_NAME, "_SX_DB_CUR");
    assert_eq!(PIVOT_CACHE_STORAGE_PATH, "/_SX_DB_CUR");
    assert_eq!(REVISION_LOG_STREAM_PATH, "/Revision Log");
    assert_eq!(USER_NAMES_STREAM_PATH, "/User Names");

    assert_eq!(FORM_STREAM_NAME, "f");
    assert_eq!(OBJECT_STREAM_NAME, "o");
    assert_eq!(MULTIPAGE_STREAM_NAME, "x");
    assert_eq!(VBA_STORAGE_NAME, "VBA");
    assert_eq!(VBA_DIRECTORY_STREAM_NAME, "dir");
    assert_eq!(VBA_PROJECT_CACHE_STREAM_NAME, "_VBA_PROJECT");
    assert_eq!(VBA_PROJECT_STREAM_NAME, "PROJECT");
    assert_eq!(VBA_PROJECT_WM_STREAM_NAME, "PROJECTwm");
    assert_eq!(VBA_PROJECT_LK_STREAM_NAME, "PROJECTlk");
    assert_eq!(DOC_VBA_PROJECT_STORAGE_NAME, "Macros");
    assert_eq!(XLS_VBA_PROJECT_STORAGE_NAME, "_VBA_PROJECT_CUR");
}

#[test]
fn xls_natural_language_and_mem_no_mem_formula_tokens_are_public_typed_values() {
    let rgce = [
        0x18, 0x03, 5, 0, 7, 0, // PtgElfCol
        0x18, 0x0d, 0xaa, 0xbb, 0xcc, 0xdd, // PtgElfColS
        0x68, 1, 2, 3, 4, 9, 0, // PtgMemNoMem
    ];
    let rgcb = [2, 0, 0, 0x80, 1, 0, 2, 0, 3, 0, 4, 0];
    let mut formula = FormulaTokenStream::from_bytes(&rgce).expect("parse typed formula tokens");
    assert!(formula.unparsed_tail.is_empty());
    assert_eq!(formula.missing_extra_count(), 1);
    assert!(
        formula
            .parse_extra_data(&rgcb)
            .expect("parse PtgExtraElf")
            .is_empty()
    );
    assert_eq!(formula.missing_extra_count(), 0);
    assert_eq!(formula.nonconforming_token_count(), 0);
    assert_eq!(formula.to_bytes().unwrap(), rgce);
    assert_eq!(formula.extra_data_to_bytes().unwrap(), rgcb);
    assert!(matches!(
        formula.tokens[0].data,
        FormulaTokenData::NaturalLanguage {
            extended_opcode: 0x03,
            value: FormulaNaturalLanguageToken::Location(FormulaElfLocation { row: 5, column: 7 }),
        }
    ));
    let FormulaTokenData::NaturalLanguage {
        value:
            FormulaNaturalLanguageToken::MultipleCell {
                extra: Some(extra), ..
            },
        ..
    } = &formula.tokens[1].data
    else {
        panic!("PtgElfColS remains a typed multiple-cell token");
    };
    assert!(extra.relative);
    assert!(!extra.reserved);
    assert_eq!(
        extra.locations,
        [
            FormulaElfExtraLocation { row: 1, column: 2 },
            FormulaElfExtraLocation { row: 3, column: 4 },
        ]
    );
}

#[test]
fn owned_archive_inputs_open_all_file_roots_without_changing_the_typed_result() {
    let doc_bytes = fs::read(fixture("Apache-POI/test-data/document/simple.doc"))
        .expect("read owned DOC fixture");
    let doc_borrowed = DocFile::from_bytes(&doc_bytes).expect("open borrowed DOC fixture");
    let doc_owned = DocFile::from_vec(doc_bytes).expect("open owned DOC fixture");
    assert!(
        doc_borrowed
            .source_compound_file()
            .logical_eq(doc_owned.source_compound_file())
    );
    let mut doc_output = Vec::new();
    doc_owned
        .write_to(&mut doc_output)
        .expect("write DOC to a caller-owned sink");
    assert_eq!(doc_output, doc_owned.to_bytes().unwrap());

    let xls_bytes = fs::read(fixture(
        "Apache-POI/test-data/spreadsheet/SimpleWithFormula.xls",
    ))
    .expect("read owned XLS fixture");
    let xls_borrowed = XlsFile::from_bytes(&xls_bytes).expect("open borrowed XLS fixture");
    let xls_owned = XlsFile::from_vec(xls_bytes).expect("open owned XLS fixture");
    assert!(
        xls_borrowed
            .source_compound_file()
            .logical_eq(xls_owned.source_compound_file())
    );
    let mut xls_output = Vec::new();
    xls_owned
        .write_to(&mut xls_output)
        .expect("write XLS to a caller-owned sink");
    assert_eq!(xls_output, xls_owned.to_bytes().unwrap());

    let ppt_bytes = fs::read(fixture(
        "Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt",
    ))
    .expect("read owned PPT fixture");
    let ppt_borrowed = PptFile::from_bytes(&ppt_bytes).expect("open borrowed PPT fixture");
    let ppt_owned = PptFile::from_vec(ppt_bytes).expect("open owned PPT fixture");
    assert!(
        ppt_borrowed
            .source_compound_file()
            .logical_eq(ppt_owned.source_compound_file())
    );
    let mut ppt_output = Vec::new();
    ppt_owned
        .write_to(&mut ppt_output)
        .expect("write PPT to a caller-owned sink");
    assert_eq!(ppt_output, ppt_owned.to_bytes().unwrap());
}

#[test]
fn ppt_slide_shape_text_placeholder_and_notes_relationships_are_native() {
    let basic = PptFile::open(fixture(
        "Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt",
    ))
    .expect("strictly open PPT relationship fixture");
    assert_basic_ppt_relationships(&basic);
    let basic_bytes = basic.to_bytes().expect("save PPT relationship fixture");
    let basic_reopened =
        PptFile::from_bytes(&basic_bytes).expect("strictly reopen PPT relationship fixture");
    assert_basic_ppt_relationships(&basic_reopened);

    let mixed = PptFile::open(fixture("Apache-POI/test-data/slideshow/SampleShow.ppt"))
        .expect("strictly open mixed-encoding PPT fixture");
    assert_mixed_ppt_text_and_notes(&mixed);
    let mixed_bytes = mixed.to_bytes().expect("save mixed-encoding PPT fixture");
    let mixed_reopened =
        PptFile::from_bytes(&mixed_bytes).expect("strictly reopen mixed-encoding PPT fixture");
    assert_mixed_ppt_text_and_notes(&mixed_reopened);
    assert_second_cycle_logically_stable(
        mixed_reopened.source_compound_file(),
        &mixed_reopened
            .to_bytes()
            .expect("save mixed-encoding PPT a second time"),
    );
}

#[test]
fn ppt_table_marker_and_child_shape_identity_are_native() {
    let file = PptFile::open(fixture("Apache-POI/test-data/slideshow/table_test.ppt"))
        .expect("strictly open PPT table fixture");
    assert_ppt_table_relationships(&file);
    let bytes = file.to_bytes().expect("save PPT table fixture");
    let reopened = PptFile::from_bytes(&bytes).expect("strictly reopen PPT table fixture");
    assert_ppt_table_relationships(&reopened);
    assert_second_cycle_logically_stable(
        reopened.source_compound_file(),
        &reopened.to_bytes().expect("save PPT table a second time"),
    );
}

#[test]
fn ppt_utf16_is_string_and_unpaired_surrogate_is_compatible_only() {
    let path = fixture("Apache-POI/test-data/slideshow/54880_chinese.ppt");
    let source = fs::read(&path).expect("read PPT UTF-16 fixture");
    let file = PptFile::from_bytes(&source).expect("strictly open PPT UTF-16 fixture");
    let live = file
        .live_presentation()
        .expect("resolve strict PPT UTF-16 presentation");
    let slides = live.slides().expect("resolve strict PPT UTF-16 slides");
    assert_eq!(slides.len(), 1);
    let atoms = slides[0]
        .object
        .record_text_bodies()
        .into_iter()
        .flat_map(|body| body.character_atoms())
        .collect::<Vec<_>>();
    assert_eq!(atoms.len(), 1);
    let PptLiveTextAtomRef::String {
        source_record,
        value,
        encoding,
    } = atoms[0]
    else {
        panic!("strict PPT UTF-16 text is a Rust string");
    };
    assert_eq!(encoding, PptTextEncoding::Utf16);
    assert_eq!(
        source_record.header.record_type,
        olecfsdk::ppt::TEXT_CHARS_ATOM
    );
    assert_eq!(
        value,
        "Single byte\r複数の文字\rカタカナ\rﾊﾝｶｸ\r表十ソ\r𠮟\r表Mixパﾋﾟ𠮟"
    );
    let units = value.encode_utf16().collect::<Vec<_>>();
    let unit_index = units
        .windows(2)
        .position(|pair| pair == [0xd842, 0xdf9f])
        .expect("PPT UTF-16 text contains the expected surrogate pair");
    let record_offset = source_record.offset;

    let strict_bytes = file.to_bytes().expect("save conforming PPT UTF-16");
    let strict_reopened =
        PptFile::from_bytes(&strict_bytes).expect("strictly reopen conforming PPT UTF-16");
    assert_second_cycle_logically_stable(
        strict_reopened.source_compound_file(),
        &strict_reopened
            .to_bytes()
            .expect("save conforming PPT UTF-16 a second time"),
    );

    let mut malformed = CompoundFile::from_bytes(&source).expect("open PPT UTF-16 CFB");
    let document = malformed
        .stream_mut("/PowerPoint Document")
        .expect("PPT has PowerPoint Document stream");
    let byte_offset = usize::try_from(record_offset)
        .expect("PPT record offset fits usize")
        .checked_add(8 + (unit_index + 1) * 2)
        .expect("PPT UTF-16 byte offset does not overflow");
    document[byte_offset..byte_offset + 2].copy_from_slice(&u16::from(b'A').to_le_bytes());
    let malformed_bytes = malformed
        .to_bytes()
        .expect("serialize malformed PPT UTF-16");

    let strict_error = PptFile::from_bytes(&malformed_bytes)
        .expect_err("strict PPT rejects an unpaired UTF-16 surrogate");
    assert!(strict_error.to_string().contains("unpaired surrogate"));

    let compatible = PptFile::from_bytes_compatible(&malformed_bytes)
        .expect("compatible PPT preserves an unpaired UTF-16 surrogate");
    assert!(compatible.diagnostics.iter().any(|diagnostic| {
        diagnostic.structure == "TextCharsAtom" && diagnostic.message.contains("unpaired surrogate")
    }));
    let compatible_live = compatible
        .value
        .live_presentation_compatible()
        .expect("resolve compatible PPT presentation")
        .into_value();
    let compatible_slides = compatible_live
        .slides_compatible()
        .expect("resolve compatible PPT slides");
    let compatible_atoms = compatible_slides[0]
        .object
        .record_text_bodies()
        .into_iter()
        .flat_map(|body| body.character_atoms())
        .collect::<Vec<_>>();
    assert_eq!(compatible_atoms.len(), 1);
    let PptLiveTextAtomRef::CompatibilityUtf16 { code_units, .. } = compatible_atoms[0] else {
        panic!("malformed PPT UTF-16 remains an explicit compatibility value");
    };
    assert_eq!(&code_units[unit_index..unit_index + 2], &[0xd842, 0x0041]);
    assert!(compatible.value.to_bytes().is_err());

    let saved = compatible
        .value
        .to_bytes_preserving_compatibility()
        .expect("preserve malformed PPT UTF-16 explicitly");
    let reopened = PptFile::from_bytes_compatible(&saved)
        .expect("compatibly reopen malformed PPT UTF-16")
        .value;
    let reopened_live = reopened
        .live_presentation_compatible()
        .expect("resolve reopened compatible PPT presentation")
        .into_value();
    let reopened_slides = reopened_live
        .slides_compatible()
        .expect("resolve reopened compatible PPT slides");
    let reopened_atoms = reopened_slides[0]
        .object
        .record_text_bodies()
        .into_iter()
        .flat_map(|body| body.character_atoms())
        .collect::<Vec<_>>();
    let PptLiveTextAtomRef::CompatibilityUtf16 { code_units, .. } = reopened_atoms[0] else {
        panic!("reopened malformed PPT UTF-16 remains explicit");
    };
    assert_eq!(&code_units[unit_index..unit_index + 2], &[0xd842, 0x0041]);
    assert_second_cycle_logically_stable(
        reopened.source_compound_file(),
        &reopened
            .to_bytes_preserving_compatibility()
            .expect("save malformed PPT UTF-16 a second time"),
    );
}

#[test]
fn xls_cells_formulas_formats_comments_hyperlinks_and_merges_are_native() {
    assert_strict_xls_cycle(
        "Apache-POI/test-data/spreadsheet/SimpleWithFormula.xls",
        assert_xls_formula_values,
    );
    assert_strict_xls_cycle(
        "Apache-POI/test-data/spreadsheet/Formatting.xls",
        assert_xls_number_formats,
    );
    assert_strict_xls_cycle(
        "Apache-POI/test-data/spreadsheet/SimpleWithComments.xls",
        assert_xls_comments,
    );
    assert_strict_xls_cycle(
        "Apache-POI/test-data/spreadsheet/WithTwoHyperLinks.xls",
        assert_xls_hyperlinks,
    );
    assert_strict_xls_cycle(
        "Apache-POI/test-data/spreadsheet/13796.xls",
        assert_xls_merged_range,
    );
}

#[test]
fn doc_sections_join_main_ranges_sed_sepx_and_blocks() {
    let file = DocFile::open(fixture("Apache-POI/test-data/document/Bug53453Section.doc"))
        .expect("strictly open DOC section fixture");
    assert_doc_sections(&file);
    let bytes = file.to_bytes().expect("save DOC section fixture");
    let reopened = DocFile::from_bytes(&bytes).expect("strictly reopen DOC section fixture");
    assert_doc_sections(&reopened);
    assert_second_cycle_logically_stable(
        reopened.source_compound_file(),
        &reopened
            .to_bytes()
            .expect("save DOC sections a second time"),
    );
}

#[test]
fn doc_paragraph_style_outline_and_block_order_are_native() {
    let file = DocFile::open(fixture("Apache-POI/test-data/document/Lists.doc"))
        .expect("strictly open DOC outline fixture");
    let tree = file.content_tree().expect("traverse DOC outline fixture");
    let main = tree.part(FieldDocumentPart::Main).expect("DOC Main part");
    let blocks = main.blocks().expect("derive DOC block order");
    assert!(blocks.diagnostics().is_empty());
    assert_eq!(blocks.blocks().len(), 40);
    assert!(
        blocks
            .blocks()
            .iter()
            .all(|block| matches!(block, DocBlockRef::Paragraph(_)))
    );
    let expected_boundaries = [
        0, 16, 68, 85, 90, 95, 123, 138, 143, 148, 206, 213, 220, 249, 256, 263, 270, 277, 284,
        291, 298, 305, 312, 319, 326, 352, 357, 362, 369, 376, 385, 394, 405, 414, 419, 472, 486,
        501, 522, 531, 532,
    ];
    assert_eq!(
        blocks
            .blocks()
            .iter()
            .map(|block| {
                let range = block.local_cp_range();
                (range.start.value(), range.end.value())
            })
            .collect::<Vec<_>>(),
        expected_boundaries
            .windows(2)
            .map(|range| (range[0], range[1]))
            .collect::<Vec<_>>()
    );

    let paragraphs = main.paragraphs().collect::<Vec<_>>();
    let styles = paragraphs
        .iter()
        .map(|paragraph| paragraph.style().expect("resolve paragraph style"))
        .collect::<Vec<_>>();
    assert_eq!(styles[0].style_index(), 1);
    assert_eq!(styles[0].source().base.invariant_style_id, 1);
    let inherited = styles[0].properties().expect("resolve inherited style");
    assert_eq!(inherited.style_index, 1);
    assert_eq!(inherited.lineage.last(), Some(&1));
    assert_eq!(
        paragraphs
            .iter()
            .enumerate()
            .filter_map(|(index, paragraph)| {
                let level = paragraph.outline_level().expect("resolve outline level");
                (level != DocOutlineLevel::BodyText).then_some((
                    index,
                    paragraph.local_cp_range().start.value(),
                    level,
                ))
            })
            .collect::<Vec<_>>(),
        vec![(0, 0, DocOutlineLevel::Level1)]
    );
}

#[test]
fn doc_blocks_emit_tables_once_and_preserve_nested_table_identity() {
    let strict = DocFile::open(fixture("Apache-POI/test-data/document/simple-table.doc"))
        .expect("strictly open DOC table fixture");
    assert_simple_doc_table_blocks(&strict);
    let strict_bytes = strict.to_bytes().expect("save strict DOC table fixture");
    assert_simple_doc_table_blocks(
        &DocFile::from_bytes(&strict_bytes).expect("strictly reopen DOC table fixture"),
    );

    let nested_path = fixture("Apache-POI/test-data/document/innertable.doc");
    assert!(DocFile::open(&nested_path).is_err());
    let nested =
        DocFile::open_compatible(&nested_path).expect("compatibly open nested DOC table fixture");
    assert!(!nested.diagnostics.is_empty());
    assert_nested_doc_table_blocks(&nested.value);
    let nested_bytes = nested
        .value
        .to_bytes_preserving_compatibility()
        .expect("preserve compatible nested DOC table fixture");
    let reopened = DocFile::from_bytes_compatible(&nested_bytes)
        .expect("compatibly reopen nested DOC table fixture")
        .value;
    assert_nested_doc_table_blocks(&reopened);
    assert_second_cycle_logically_stable(
        reopened.source_compound_file(),
        &reopened
            .to_bytes_preserving_compatibility()
            .expect("save nested DOC table fixture a second time"),
    );
}

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
    let before_failed_edit = file.clone();
    assert!(Arc::ptr_eq(
        &file.word_document,
        &before_failed_edit.word_document
    ));
    assert!(Arc::ptr_eq(&file.table, &before_failed_edit.table));
    assert!(file.replace_main_text_range(cp..cp + 1, "\r").is_err());
    assert_eq!(file, before_failed_edit);
    assert!(Arc::ptr_eq(
        &file.word_document,
        &before_failed_edit.word_document
    ));
    assert!(Arc::ptr_eq(&file.table, &before_failed_edit.table));
    file.replace_main_text_range(cp..cp + 1, replacement)
        .expect("edit DOC text through the file-root API");
    assert!(!Arc::ptr_eq(
        &file.word_document,
        &before_failed_edit.word_document
    ));
    assert!(!Arc::ptr_eq(&file.table, &before_failed_edit.table));
    if let (Some(current), Some(before)) = (&file.data, &before_failed_edit.data) {
        assert!(Arc::ptr_eq(current, before));
    }
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
fn doc_save_rejects_inconsistent_typed_data_links_without_reparsing_source_bytes() {
    let original = DocFile::open(fixture("Apache-POI/test-data/document/Bug61268.doc"))
        .expect("open DOC Data-link fixture strictly");
    let original_data = original.data.as_deref().expect("fixture has a Data stream");
    assert!(original_data.nodes.len() > 1);

    let mut out_of_bounds = original.clone();
    let data = Arc::make_mut(out_of_bounds.data.as_mut().unwrap());
    data.nodes[0].physical_len = data.physical_bytes.len() + 1;
    let error = out_of_bounds
        .to_compound_file_preserving_compatibility()
        .expect_err("an out-of-bounds typed Data node must fail before save");
    assert!(error.to_string().contains("physical range exceeds"));

    let mut overlap = original.clone();
    let data = Arc::make_mut(overlap.data.as_mut().unwrap());
    data.nodes[1].offset = data.nodes[0].offset;
    let error = overlap
        .to_compound_file_preserving_compatibility()
        .expect_err("overlapping typed Data nodes must fail before save");
    assert!(error.to_string().contains("overlap"));

    let mut missing = original.clone();
    Arc::make_mut(missing.data.as_mut().unwrap()).nodes.pop();
    let error = missing
        .to_compound_file_preserving_compatibility()
        .expect_err("removing a referenced typed Data node must fail before save");
    assert!(error.to_string().contains("missing typed Data node"));

    let mut wrong_kind = original;
    let data = Arc::make_mut(wrong_kind.data.as_mut().unwrap());
    let picture = data
        .nodes
        .iter()
        .position(|node| matches!(node.value, DocDataNodeValue::Picture(_)))
        .expect("fixture has picture Data");
    let paragraph = data
        .nodes
        .iter()
        .position(|node| matches!(node.value, DocDataNodeValue::ParagraphProperties(_)))
        .expect("fixture has paragraph-property Data");
    let (left, right) = data.nodes.split_at_mut(paragraph.max(picture));
    if picture < paragraph {
        std::mem::swap(&mut left[picture].value, &mut right[0].value);
    } else {
        std::mem::swap(&mut right[0].value, &mut left[paragraph].value);
    }
    let error = wrong_kind
        .to_compound_file_preserving_compatibility()
        .expect_err("changing the referenced typed Data-node kind must fail before save");
    assert!(error.to_string().contains("type link changed"));
}

#[test]
fn doc_utf16_is_string_and_unpaired_surrogate_is_compatible_only() {
    let path = fixture("Apache-POI/test-data/document/simple.doc");
    let source = fs::read(&path).expect("read DOC UTF-16 upgrade fixture");
    let file = DocFile::from_bytes(&source).expect("strictly open DOC UTF-16 upgrade fixture");
    let (emoji_cp, _, _) = first_editable_doc_character(&file);
    let mut emoji_file = file.clone();
    emoji_file
        .replace_main_text_range(emoji_cp..emoji_cp + 1, "😀")
        .expect("replace one DOC code unit with a surrogate pair");
    let emoji_bytes = emoji_file.to_bytes().expect("save DOC surrogate pair");
    let emoji_reopened = DocFile::from_bytes(&emoji_bytes).expect("strictly reopen DOC emoji");
    let emoji_main = emoji_reopened
        .content_tree()
        .expect("traverse DOC emoji content")
        .part(FieldDocumentPart::Main)
        .expect("DOC has Main part");
    assert_eq!(emoji_main.character_at(DocCp::new(emoji_cp)), Some(0xd83d));
    assert_eq!(
        emoji_main.character_at(DocCp::new(emoji_cp + 1)),
        Some(0xde00)
    );
    assert!(
        emoji_reopened
            .word_document
            .text_pieces
            .iter()
            .any(|piece| {
                matches!(
                    &piece.value.characters,
                    TextPieceCharacters::String(value) if value.value.contains('😀')
                )
            })
    );
    assert_second_cycle_logically_stable(
        emoji_reopened.source_compound_file(),
        &emoji_reopened
            .to_bytes()
            .expect("save DOC emoji second cycle"),
    );

    let (piece_index, file_offset, unit_index) = emoji_reopened
        .word_document
        .text_pieces
        .iter()
        .enumerate()
        .find_map(|(piece_index, piece)| {
            let TextPieceCharacters::String(value) = &piece.value.characters else {
                return None;
            };
            if value.encoding != TextPieceEncoding::Utf16 {
                return None;
            }
            let units = value.value.encode_utf16().collect::<Vec<_>>();
            let unit_index = units.windows(2).position(|pair| pair == [0xd83d, 0xde00])?;
            Some((piece_index, piece.value.file_offset, unit_index))
        })
        .expect("reopened DOC has the inserted surrogate pair");

    let mut malformed =
        CompoundFile::from_bytes(&emoji_bytes).expect("open DOC surrogate-pair CFB");
    let word = malformed
        .stream_mut("/WordDocument")
        .expect("Unicode DOC has WordDocument stream");
    let byte_offset = usize::try_from(file_offset)
        .expect("DOC file offset fits usize")
        .checked_add((unit_index + 1) * 2)
        .expect("DOC UTF-16 byte offset does not overflow");
    word[byte_offset..byte_offset + 2].copy_from_slice(&u16::from(b'A').to_le_bytes());
    let malformed_bytes = malformed
        .to_bytes()
        .expect("serialize malformed Unicode DOC");

    let strict_error = DocFile::from_bytes(&malformed_bytes)
        .expect_err("strict DOC rejects an unpaired UTF-16 surrogate");
    assert!(
        strict_error
            .to_string()
            .contains("unpaired UTF-16 surrogate")
    );

    let compatible = DocFile::from_bytes_compatible(&malformed_bytes)
        .expect("compatible DOC preserves an unpaired UTF-16 surrogate");
    assert!(compatible.diagnostics.iter().any(|diagnostic| {
        diagnostic.structure == "PlcPcd" && diagnostic.message.contains("unpaired UTF-16 surrogate")
    }));
    let compatible_units = compatible.value.word_document.text_pieces[piece_index]
        .value
        .characters
        .compatibility_code_units()
        .expect("malformed text piece is an explicit compatibility value");
    assert_eq!(
        &compatible_units[unit_index..unit_index + 2],
        &[0xd83d, 0x0041]
    );
    assert!(compatible.value.to_bytes().is_err());

    let saved = compatible
        .value
        .to_bytes_preserving_compatibility()
        .expect("preserve malformed DOC UTF-16 explicitly");
    let reopened = DocFile::from_bytes_compatible(&saved)
        .expect("compatibly reopen malformed DOC UTF-16")
        .value;
    assert_eq!(
        reopened.word_document.text_pieces[piece_index]
            .value
            .characters
            .compatibility_code_units()
            .expect("reopened malformed text remains explicit")[unit_index..unit_index + 2],
        [0xd83d, 0x0041]
    );
    assert!(
        CompoundFile::from_bytes(&malformed_bytes)
            .expect("reopen malformed source CFB")
            .logical_eq(reopened.source_compound_file())
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
    invalid_name.value.clear();
    let before_failed_edit = file.clone();
    assert!(Arc::ptr_eq(&file.workbooks, &before_failed_edit.workbooks));
    assert!(Arc::ptr_eq(
        &file.pivot_caches,
        &before_failed_edit.pivot_caches
    ));
    assert!(
        file.set_sheet_name(workbook_name, sheet_id, invalid_name)
            .is_err()
    );
    assert_eq!(file, before_failed_edit);
    assert!(Arc::ptr_eq(&file.workbooks, &before_failed_edit.workbooks));
    file.set_sheet_name(workbook_name, sheet_id, edited_name.clone())
        .expect("edit XLS sheet name through the file-root API");
    assert!(!Arc::ptr_eq(&file.workbooks, &before_failed_edit.workbooks));
    assert!(Arc::ptr_eq(
        &file.pivot_caches,
        &before_failed_edit.pivot_caches
    ));
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
fn xls_sheet_name_uses_lossless_rust_string_with_physical_encoding_metadata() {
    let path = fixture("Apache-POI/test-data/spreadsheet/Simple.xls");
    let source = fs::read(&path).expect("read XLS Unicode fixture");
    for (value, expected_utf16_flag, expected_code_units) in [("Café", 0, 4), ("A😀中", 1, 4)] {
        let mut file = XlsFile::from_bytes(&source).expect("strictly open XLS Unicode fixture");
        let (workbook_name, sheet_id, _) = xls_sheet_name_edit(&file);
        let name = ShortXlUnicodeString::new(value);
        assert_eq!(name.value, value);
        assert_eq!(name.flags & 1, expected_utf16_flag);
        assert_eq!(name.value.encode_utf16().count(), expected_code_units);

        file.set_sheet_name(workbook_name, sheet_id, name.clone())
            .expect("set Rust String sheet name");
        let saved = file.to_bytes().expect("save Rust String sheet name");
        let reopened = XlsFile::from_bytes(&saved).expect("strictly reopen Rust String sheet name");
        assert_eq!(xls_sheet_name(&reopened, workbook_name, sheet_id), name);
        assert_second_cycle_logically_stable(
            reopened.source_compound_file(),
            &reopened.to_bytes().expect("save Unicode XLS second cycle"),
        );
    }
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
    assert_eq!(
        LocatedParentControlStorage::discover_root_paths_below(
            &forms_source,
            Path::new("/_VBA_PROJECT_CUR"),
        ),
        vec![PathBuf::from("/_VBA_PROJECT_CUR/UserForm1")]
    );
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
    std::sync::Arc::make_mut(&mut ppt.document).records.records[record_index].data =
        PptRecordData::ExternalStorage(
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

#[derive(Debug, PartialEq, Eq)]
struct XlsActiveXPersistenceSnapshot {
    workbook: XlsStreamName,
    sheet: XlsSheetId,
    object_id: u16,
    stream_path: PathBuf,
    offset: u32,
    data: Vec<u8>,
}

#[test]
fn xls_activex_host_relationship_resolves_exact_ctls_slices_across_save() {
    let path = fixture("Apache-POI/test-data/spreadsheet/WithCheckBoxes.xls");
    let bytes = fs::read(&path).expect("read XLS ActiveX fixture");
    let file = XlsFile::from_bytes_compatible(&bytes)
        .expect("compatibly open XLS ActiveX fixture")
        .value;
    let before = xls_activex_persistence_snapshot(&file);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].stream_path, Path::new("/Ctls"));
    assert_eq!(before[0].offset, 0);
    assert_eq!(before[0].data.len(), 104);

    let saved = file
        .to_bytes_preserving_compatibility()
        .expect("save XLS ActiveX fixture");
    let reopened = XlsFile::from_bytes_compatible(&saved)
        .expect("compatibly reopen XLS ActiveX fixture")
        .value;
    assert_eq!(xls_activex_persistence_snapshot(&reopened), before);
    assert_second_cycle_logically_stable(
        reopened.source_compound_file(),
        &reopened
            .to_bytes_preserving_compatibility()
            .expect("save XLS ActiveX fixture second cycle"),
    );
}

fn xls_activex_persistence_snapshot(file: &XlsFile) -> Vec<XlsActiveXPersistenceSnapshot> {
    let inventory = file.storages_and_streams_compatible();
    let mut snapshot = Vec::new();
    for workbook in file.workbooks.iter() {
        let view = workbook
            .relationships_compatible()
            .expect("traverse XLS ActiveX workbook relationships");
        for sheet in view.sheets() {
            for object in sheet.objects() {
                if !object
                    .picture_flags()
                    .is_some_and(|flags| flags.contains(ObjPictureFlags::CONTROL_STREAM))
                {
                    continue;
                }
                let persistence = inventory
                    .resolve_object_persistence_compatible(&view, object)
                    .expect("resolve ActiveX Obj persistence");
                let Some(XlsObjectPersistenceRef::ControlStream {
                    stream,
                    offset,
                    data,
                }) = persistence
                else {
                    panic!("ActiveX Obj must resolve to an exact Ctls stream slice");
                };
                snapshot.push(XlsActiveXPersistenceSnapshot {
                    workbook: workbook.name,
                    sheet: sheet.id(),
                    object_id: object
                        .id()
                        .expect("ActiveX Obj has an FtCmo identity")
                        .value(),
                    stream_path: stream.path.clone(),
                    offset,
                    data: data.to_vec(),
                });
            }
        }
    }
    snapshot
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
                .create_stream(&path, entry.data.to_vec())
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

fn assert_strict_xls_cycle(relative: &str, verify: fn(&XlsFile)) {
    let file = XlsFile::open(fixture(relative)).expect("strictly open XLS native-object fixture");
    verify(&file);
    let bytes = file.to_bytes().expect("save XLS native-object fixture");
    let reopened = XlsFile::from_bytes(&bytes).expect("strictly reopen XLS native-object fixture");
    verify(&reopened);
    assert_second_cycle_logically_stable(
        reopened.source_compound_file(),
        &reopened
            .to_bytes()
            .expect("save XLS native-object fixture a second time"),
    );
}

fn assert_xls_formula_values(file: &XlsFile) {
    assert_eq!(file.workbooks.len(), 1);
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS formula relationships");
    assert_eq!(view.sheets().len(), 3);
    let sheet = view.sheets()[0];
    let index = sheet
        .sparse_cell_index()
        .expect("build strict XLS formula cell index");
    assert_eq!(index.len(), 3);
    let first = index.cell(0, 0).expect("lookup A1").expect("A1 exists");
    let second = index.cell(1, 0).expect("lookup A2").expect("A2 exists");
    let formula_cell = index.cell(2, 0).expect("lookup A3").expect("A3 exists");
    assert_eq!(
        view.resolve_cell_value(&index, first)
            .expect("resolve A1 stored value"),
        XlsCellValue::String("replaceme".to_owned())
    );
    assert_eq!(
        view.resolve_cell_value(&index, second)
            .expect("resolve A2 stored value"),
        XlsCellValue::String("replaceme".to_owned())
    );
    assert_eq!(
        view.resolve_cell_value(&index, formula_cell)
            .expect("resolve A3 formula cache"),
        XlsCellValue::Formula(XlsFormulaCachedValue::String(
            "replacemereplaceme".to_owned()
        ))
    );
    let formula = index
        .resolve_cell_formula(formula_cell)
        .expect("resolve A3 formula relationship")
        .expect("A3 is a formula cell");
    assert!(std::ptr::eq(
        formula.source_record(),
        formula_cell.source_record()
    ));
    assert_eq!(formula.formula().tokens.rgce.tokens.len(), 3);
    assert!(matches!(
        formula.definition(),
        olecfsdk::xls::XlsFormulaDefinitionRef::Inline(_)
    ));
    assert_eq!(
        formula
            .cached_value()
            .expect("resolve native formula cache"),
        XlsFormulaCachedValue::String("replacemereplaceme".to_owned())
    );
}

fn assert_xls_number_formats(file: &XlsFile) {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS formatting relationships");
    let sheet = view.sheets()[0];
    let index = sheet
        .sparse_cell_index()
        .expect("build strict XLS formatting cell index");
    let built_in_cell = index.cell(1, 1).expect("lookup B2").expect("B2 exists");
    assert_eq!(
        view.resolve_cell_value(&index, built_in_cell)
            .expect("resolve B2 stored value"),
        XlsCellValue::Number(39045.0)
    );
    let built_in = view
        .resolve_cell_format_ref(built_in_cell)
        .expect("resolve B2 XF and format");
    assert_eq!(built_in.number_format, XlsNumberFormatRef::BuiltIn(14));
    assert_eq!(built_in.custom_number_format_code, None);

    let custom_cell = index.cell(2, 1).expect("lookup B3").expect("B3 exists");
    let custom = view
        .resolve_cell_format_ref(custom_cell)
        .expect("resolve B3 XF and custom format");
    let XlsNumberFormatRef::Custom(format) = custom.number_format else {
        panic!("B3 references a custom Format record");
    };
    assert_eq!(format.format_index, 165);
    assert_eq!(
        custom.custom_number_format_code.as_deref(),
        Some("yyyy/mm/dd")
    );
    assert_eq!(custom.xf.number_format_index, format.format_index);
}

fn assert_xls_comments(file: &XlsFile) {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS comment relationships");
    let sheet = view.sheets()[0];
    let comments = sheet.comments().expect("join NoteSh, Obj and TxO comments");
    assert_eq!(comments.len(), 3);
    assert_eq!(
        comments
            .iter()
            .map(|comment| (
                comment.note().row,
                comment.note().column,
                comment.note().object_id,
                comment
                    .note()
                    .flags
                    .contains(olecfsdk::xls::NoteFlags::SHOW),
                comment.author.as_str(),
                comment.content.as_str(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (0, 1, 1, false, "Yegor Kozlov", "Yegor Kozlov:\nfirst cell"),
            (1, 1, 2, false, "Yegor Kozlov", "Yegor Kozlov:\nsecond cell"),
            (2, 1, 3, true, "Yegor Kozlov", "Yegor Kozlov:\nthird cell"),
        ]
    );
    for comment in &comments {
        assert_eq!(
            comment
                .object()
                .id()
                .expect("comment Obj has cmo.id")
                .value(),
            comment.note().object_id
        );
        assert!(std::ptr::eq(
            comment.source_record(),
            comment.object().source_record()
        ));
        assert!(matches!(
            comment.text_host().data,
            olecfsdk::xls::MsoDrawingHostData::Txo(_)
        ));
        assert_eq!(
            comment.text_object().declared_text_length as usize,
            comment.content.encode_utf16().count()
        );
    }
    let index = sheet
        .sparse_cell_index()
        .expect("build strict XLS comment cell index");
    let b1 = index.cell(0, 1).expect("lookup B1").expect("B1 exists");
    let comment = index
        .comment(b1)
        .expect("resolve B1 comment")
        .expect("B1 has a comment");
    assert_eq!(comment.note().object_id, 1);
    assert_eq!(comment.content, "Yegor Kozlov:\nfirst cell");
}

fn assert_xls_hyperlinks(file: &XlsFile) {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS hyperlink relationships");
    let sheet = view.sheets()[0];
    let links = sheet.hyperlinks().expect("resolve strict HLink records");
    assert_eq!(links.len(), 2);
    assert_eq!(
        links
            .iter()
            .map(|link| (
                link.value().first_row,
                link.value().first_column,
                link.display_name.as_deref(),
                match &link.target {
                    Some(XlsHyperlinkTarget::Url(value)) => Some(value.as_str()),
                    _ => None,
                },
            ))
            .collect::<Vec<_>>(),
        vec![
            (4, 0, Some("Foo"), Some("http://poi.apache.org/")),
            (8, 1, Some("Bar"), Some("http://poi.apache.org/hssf/")),
        ]
    );
    assert!(links.iter().all(|link| {
        link.target_frame_name.is_none()
            && link.location.is_none()
            && matches!(
                link.source_record().data,
                olecfsdk::xls::BiffRecordData::Hyperlink(_)
            )
    }));
    let index = sheet
        .sparse_cell_index()
        .expect("build strict XLS hyperlink cell index");
    let a5 = index.cell(4, 0).expect("lookup A5").expect("A5 exists");
    let a5_links = index.hyperlinks(a5).expect("resolve A5 hyperlinks");
    assert_eq!(a5_links.len(), 1);
    assert_eq!(a5_links[0].display_name.as_deref(), Some("Foo"));
}

fn assert_xls_merged_range(file: &XlsFile) {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS merged-range relationships");
    let sheet = view.sheets()[0];
    let ranges = sheet.merged_cells().collect::<Vec<_>>();
    assert_eq!(ranges.len(), 1);
    assert_eq!(
        (
            ranges[0].first_row,
            ranges[0].last_row,
            ranges[0].first_column,
            ranges[0].last_column
        ),
        (0, 0, 1, 2)
    );
    let index = sheet
        .sparse_cell_index()
        .expect("build strict XLS merged-range cell index");
    let b1 = index.cell(0, 1).expect("lookup B1").expect("B1 exists");
    let cell_ranges = index.merged_ranges(b1).expect("resolve B1 merged ranges");
    assert_eq!(cell_ranges, ranges);
}

fn assert_second_cycle_logically_stable(first: &CompoundFile, bytes: &[u8]) {
    let second = CompoundFile::from_bytes_strict(bytes).expect("reopen second-cycle CFB strictly");
    assert!(first.logical_eq(&second));
}

fn assert_doc_sections(file: &DocFile) {
    let tree = file
        .content_tree()
        .expect("traverse strict DOC section tree");
    let main = tree.part(FieldDocumentPart::Main).expect("DOC Main part");
    let sections = main.sections().expect("join DOC section relationships");
    assert_eq!(
        sections
            .sections()
            .iter()
            .map(|section| (
                section.section_index(),
                section.local_cp_range().start.value(),
                section.local_cp_range().end.value(),
                section.global_cp_range().start.value(),
                section.global_cp_range().end.value(),
                section
                    .blocks()
                    .expect("derive section blocks")
                    .blocks()
                    .len(),
            ))
            .collect::<Vec<_>>(),
        vec![(0, 0, 10, 0, 10, 2), (1, 10, 19, 10, 19, 1)]
    );
    for (index, section) in sections.sections().iter().copied().enumerate() {
        assert_eq!(section.document_part().part(), FieldDocumentPart::Main);
        assert_eq!(section.properties().section_index, index);
        assert_eq!(section.source().sepx_offset, section.properties().offset);
        assert!(section.properties().value.is_some());
    }
    assert!(
        tree.part(FieldDocumentPart::Header)
            .expect("DOC Header part")
            .sections()
            .is_err()
    );
}

fn assert_basic_ppt_relationships(file: &PptFile) {
    let live = file
        .live_presentation()
        .expect("resolve strict PPT live presentation");
    let slides = live.slides().expect("resolve strict PPT slides");
    assert_eq!(slides.len(), 2);
    let expected_shape_ids = [vec![2048, 2050, 2051, 2049], vec![5120, 5122, 5123, 5121]];
    let expected_roles = [
        vec![PptTextType::CenterTitle, PptTextType::CenterBody],
        vec![PptTextType::Title, PptTextType::Body],
    ];
    let expected_placeholders = [
        vec![
            PptPlaceholderType::CenterTitle,
            PptPlaceholderType::SubTitle,
        ],
        vec![PptPlaceholderType::Title, PptPlaceholderType::Body],
    ];
    let expected_strings = [
        vec![
            "This is a test title",
            "This is a test subtitle\rThis is on page 1",
        ],
        vec![
            "This is the title on page 2",
            "This is page two\rIt has several blocks of text\rNone of them have formatting",
        ],
    ];
    let expected_notes = [
        "These are the notes for page 1",
        "These are the notes on page two, again lacking formatting",
    ];

    for (slide_index, slide) in slides.iter().copied().enumerate() {
        let shapes = slide.shapes().expect("resolve PPT slide shapes");
        assert_eq!(
            shapes
                .iter()
                .map(|shape| shape.shape_id())
                .collect::<Vec<_>>(),
            expected_shape_ids[slide_index]
        );
        assert_eq!(
            shapes
                .iter()
                .filter_map(|shape| shape.placeholder.map(|value| value.placement_id))
                .collect::<Vec<_>>(),
            expected_placeholders[slide_index]
        );
        let bodies = slide.object.text_bodies();
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.header.text_type)
                .collect::<Vec<_>>(),
            expected_roles[slide_index]
        );
        assert_eq!(
            bodies
                .iter()
                .flat_map(|body| body.character_atoms())
                .map(|atom| match atom {
                    PptLiveTextAtomRef::String {
                        source_record,
                        value,
                        encoding,
                    } => {
                        assert_eq!(encoding, PptTextEncoding::Bytes);
                        assert_eq!(
                            source_record.header.record_type,
                            olecfsdk::ppt::TEXT_BYTES_ATOM
                        );
                        value
                    }
                    PptLiveTextAtomRef::CompatibilityUtf16 { .. } => {
                        panic!("strict PPT has compatibility UTF-16")
                    }
                })
                .collect::<Vec<_>>(),
            expected_strings[slide_index]
        );
        assert_eq!(
            bodies
                .iter()
                .flat_map(|body| body.style_text_properties())
                .count(),
            0
        );
        let text_shapes = shapes
            .iter()
            .filter_map(|shape| shape.outline_text.map(|text| (shape, text)))
            .collect::<Vec<_>>();
        assert_eq!(text_shapes.len(), 2);
        for (expected_index, (shape, text)) in text_shapes.into_iter().enumerate() {
            assert_eq!(text.value.index as usize, expected_index);
            assert!(std::ptr::eq(
                text.text_body.header_record,
                bodies[expected_index].header_record
            ));
            assert!(
                text.shape_record
                    .is_some_and(|record| std::ptr::eq(record, shape.source_record))
            );
        }
        let PptLiveNotesLink::Resolved {
            object, notes_atom, ..
        } = slide.notes
        else {
            panic!("PPT slide notes relationship is resolved");
        };
        assert_eq!(notes_atom.slide_id_ref, slide.id().value());
        let note_bodies = object.record_text_bodies();
        assert_eq!(note_bodies.len(), 1);
        assert_eq!(
            note_bodies[0]
                .character_atoms()
                .map(|atom| match atom {
                    PptLiveTextAtomRef::String { value, .. } => value,
                    PptLiveTextAtomRef::CompatibilityUtf16 { .. } => {
                        panic!("strict notes have compatibility UTF-16")
                    }
                })
                .collect::<Vec<_>>(),
            vec![expected_notes[slide_index]]
        );
    }
}

fn assert_mixed_ppt_text_and_notes(file: &PptFile) {
    let live = file
        .live_presentation()
        .expect("resolve mixed-encoding PPT presentation");
    let slides = live.slides().expect("resolve mixed-encoding PPT slides");
    assert_eq!(slides.len(), 2);
    let expected = [
        vec![
            (PptTextEncoding::Bytes, "Title of the first slide"),
            (
                PptTextEncoding::Bytes,
                "Subtitle of the first slide\r\rThis bit is in italic green",
            ),
        ],
        vec![
            (PptTextEncoding::Bytes, "This is the second slide"),
            (
                PptTextEncoding::Utf16,
                "It has bullet points on it\rThey’re fun, aren’t they?\rEspecially in a different font like Arial Black at 16 point!",
            ),
        ],
    ];
    let expected_notes = [
        vec!["I am the notes of the first slide", "*"],
        vec![
            "These are the notes of the 2nd slide\rTHIS LINE IS BOLD",
            "*",
        ],
    ];
    let expected_styles = [
        vec![Vec::new(), vec![(56, 1, 2)]],
        vec![Vec::new(), vec![(113, 1, 2)]],
    ];
    for (index, slide) in slides.iter().copied().enumerate() {
        let bodies = slide.object.record_text_bodies();
        assert_eq!(
            bodies
                .iter()
                .map(|body| {
                    body.style_text_properties()
                        .map(|style| {
                            (
                                style.corresponding_text_character_count,
                                style.paragraph_runs.len(),
                                style.character_runs.len(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
            expected_styles[index]
        );
        assert_eq!(
            bodies
                .into_iter()
                .flat_map(|body| body.character_atoms())
                .map(|atom| match atom {
                    PptLiveTextAtomRef::String {
                        value, encoding, ..
                    } => (encoding, value),
                    PptLiveTextAtomRef::CompatibilityUtf16 { .. } => {
                        panic!("strict PPT has compatibility UTF-16")
                    }
                })
                .collect::<Vec<_>>(),
            expected[index]
        );
        let PptLiveNotesLink::Resolved { object, .. } = slide.notes else {
            panic!("mixed PPT slide notes relationship is resolved");
        };
        assert_eq!(
            object
                .record_text_bodies()
                .into_iter()
                .flat_map(|body| body.character_atoms())
                .map(|atom| match atom {
                    PptLiveTextAtomRef::String { value, .. } => value,
                    PptLiveTextAtomRef::CompatibilityUtf16 { .. } => {
                        panic!("strict PPT notes have compatibility UTF-16")
                    }
                })
                .collect::<Vec<_>>(),
            expected_notes[index]
        );
    }
}

fn assert_ppt_table_relationships(file: &PptFile) {
    let live = file
        .live_presentation()
        .expect("resolve PPT table presentation");
    let slides = live.slides().expect("resolve PPT table slide");
    assert_eq!(slides.len(), 1);
    let shapes = slides[0].shapes().expect("resolve PPT table shapes");
    assert_eq!(shapes.len(), 31);
    let table = shapes
        .iter()
        .find(|shape| shape.is_table())
        .copied()
        .expect("PPT table marker exists");
    assert_eq!(table.shape_id(), 2050);
    assert_eq!(table.shape_type(), 0);
    let property = table
        .table_property
        .expect("PPT table property source exists");
    assert_eq!(property.property_id, 0x039f);
    assert!(matches!(
        property.value,
        olecfsdk::office_art::OfficeArtPropertyValue::Simple(value) if value & 1 != 0
    ));
    let children = table.child_shapes();
    assert_eq!(
        children
            .iter()
            .map(|shape| shape.shape_id())
            .collect::<Vec<_>>(),
        (2051..=2079).collect::<Vec<_>>()
    );
    assert_eq!(
        children
            .iter()
            .filter(|shape| shape.shape_type() == 1)
            .count(),
        18
    );
    assert_eq!(
        children
            .iter()
            .filter(|shape| shape.shape_type() == 20)
            .count(),
        11
    );
    assert!(children.iter().all(|shape| {
        shape.group_record.is_some_and(|group| {
            table
                .group_record
                .is_some_and(|owner| std::ptr::eq(group, owner))
        })
    }));
}

fn assert_simple_doc_table_blocks(file: &DocFile) {
    let tree = file.content_tree().expect("traverse strict DOC table tree");
    let main = tree.part(FieldDocumentPart::Main).expect("DOC Main part");
    let blocks = main.blocks().expect("derive strict DOC block order");
    assert!(blocks.diagnostics().is_empty());
    assert_eq!(blocks.blocks().len(), 3);
    assert_eq!(
        blocks
            .blocks()
            .iter()
            .map(|block| {
                let range = block.local_cp_range();
                (
                    matches!(block, DocBlockRef::Table(_)),
                    range.start.value(),
                    range.end.value(),
                )
            })
            .collect::<Vec<_>>(),
        vec![(false, 0, 154), (true, 154, 210), (false, 210, 240)]
    );
    let DocBlockRef::Table(table) = &blocks.blocks()[1] else {
        panic!("middle DOC block is the physical table");
    };
    assert_eq!(table.table_depth(), 1);
    assert_eq!(table.rows().len(), 2);
    assert_eq!(table.local_cp_range(), blocks.blocks()[1].local_cp_range());
    assert_eq!(
        table
            .rows()
            .iter()
            .map(|row| row.cells().expect("derive strict DOC cells").cells().len())
            .collect::<Vec<_>>(),
        vec![3, 3]
    );
    for row in table.rows() {
        let cells = row.cells().expect("derive strict DOC cells");
        assert!(cells.diagnostics().is_empty());
        assert_eq!(
            cells
                .cells()
                .first()
                .expect("row has cells")
                .global_cp_range()
                .start,
            row.global_cp_range().start
        );
        assert_eq!(
            cells
                .cells()
                .last()
                .expect("row has cells")
                .global_cp_range()
                .end,
            row.terminating_paragraph().global_cp_range().start
        );
        assert_eq!(
            row.terminating_paragraph().global_cp_range().end,
            row.global_cp_range().end
        );
        for cell in cells.cells() {
            assert_eq!(
                cell.cell_mark().global_cp_range().end,
                cell.global_cp_range().end
            );
            assert_eq!(cell.row().global_cp_range(), row.global_cp_range());
        }
    }
}

fn assert_nested_doc_table_blocks(file: &DocFile) {
    let tree = file
        .content_tree_compatible()
        .expect("traverse compatible nested DOC table tree");
    let main = tree.part(FieldDocumentPart::Main).expect("DOC Main part");
    let blocks = main.blocks().expect("derive compatible DOC block order");
    assert!(blocks.diagnostics().is_empty());
    assert_eq!(
        blocks
            .blocks()
            .iter()
            .map(|block| {
                let range = block.local_cp_range();
                (
                    matches!(block, DocBlockRef::Table(_)),
                    range.start.value(),
                    range.end.value(),
                )
            })
            .collect::<Vec<_>>(),
        vec![(true, 0, 33), (false, 33, 34)]
    );
    let tables = main.tables().expect("derive all nested DOC tables");
    assert_eq!(tables.tables().len(), 2);
    assert_eq!(
        tables
            .tables()
            .iter()
            .map(|table| {
                (
                    table.table_depth(),
                    table.rows().len(),
                    table
                        .rows()
                        .iter()
                        .map(|row| row.cells().expect("derive nested DOC cells").cells().len())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
        vec![(1, 3, vec![3, 3, 3]), (2, 2, vec![2, 2])]
    );
    let outer = &tables.tables()[0];
    let nested_owners = outer
        .rows()
        .iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            row.cells()
                .expect("derive outer DOC cells")
                .cells()
                .iter()
                .copied()
                .enumerate()
                .map(move |(cell_index, cell)| (row_index, cell_index, cell))
                .collect::<Vec<_>>()
        })
        .filter_map(|(row, cell, source)| {
            let nested = tables.tables_in_cell(source).collect::<Vec<_>>();
            (!nested.is_empty()).then(|| (row, cell, nested[0].global_cp_range()))
        })
        .collect::<Vec<_>>();
    assert_eq!(nested_owners.len(), 1);
    assert_eq!((nested_owners[0].0, nested_owners[0].1), (1, 1));
    assert_eq!(nested_owners[0].2, tables.tables()[1].global_cp_range());
}

fn first_editable_doc_character(file: &DocFile) -> (u32, String, u16) {
    let main_length = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .expect("DOC main-text character count is nonnegative");
    for piece in &file.word_document.text_pieces {
        let start = u32::try_from(piece.value.cp_start).expect("DOC text-piece CP is nonnegative");
        if start >= main_length {
            break;
        }
        let TextPieceCharacters::String(value) = &piece.value.characters else {
            continue;
        };
        let mut unit_index = 0usize;
        for character in value.value.chars() {
            let character_units = character.len_utf16();
            if character_units == 1 && character.is_alphabetic() {
                let edited = if character == 'X' { "Y" } else { "X" };
                return (
                    start + u32::try_from(unit_index).expect("DOC text index fits u32"),
                    edited.to_owned(),
                    u16::from(edited.as_bytes()[0]),
                );
            }
            unit_index += character_units;
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
            TextPieceCharacters::String(value) => value
                .value
                .encode_utf16()
                .nth(index)
                .expect("DOC CP selects a String code unit"),
            TextPieceCharacters::CompatibilityUtf16 { code_units } => code_units[index],
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
    for workbook in file.workbooks.iter() {
        let relationships = workbook
            .relationships()
            .expect("traverse strict XLS workbook relationships");
        if let Some(sheet) = relationships.sheets().first().copied() {
            let mut name = sheet.metadata().name.clone();
            if name.value.encode_utf16().count() < 31 {
                name.value.push('X');
            } else {
                let first_character_bytes = name
                    .value
                    .chars()
                    .next()
                    .expect("sheet names are nonempty")
                    .len_utf8();
                name.value.replace_range(..first_character_bytes, "X");
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

fn ppt_text_edit(file: &PptFile) -> (PptSlideId, usize, bool, char) {
    let presentation = file
        .live_presentation()
        .expect("traverse strict PPT live presentation");
    for slide in presentation.slides().expect("traverse strict PPT slides") {
        for (body_index, body) in slide.object.text_bodies().iter().enumerate() {
            for record in body.records {
                match &record.data {
                    PptRecordData::TextChars(characters) if !characters.is_empty() => {
                        let replacement = if characters.starts_with('X') {
                            'Y'
                        } else {
                            'X'
                        };
                        return (slide.id(), body_index, true, replacement);
                    }
                    PptRecordData::TextBytes(characters) if !characters.is_empty() => {
                        let replacement = if characters.starts_with('X') {
                            'Y'
                        } else {
                            'X'
                        };
                        return (slide.id(), body_index, false, replacement);
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
    replacement: char,
) -> olecfsdk::Result<()> {
    for record in body.records_mut() {
        match (&mut record.data, unicode) {
            (PptRecordData::TextChars(characters), true) if !characters.is_empty() => {
                let end = characters
                    .chars()
                    .next()
                    .expect("nonempty PPT String")
                    .len_utf8();
                characters.replace_range(..end, &replacement.to_string());
                return Ok(());
            }
            (PptRecordData::TextBytes(characters), false) if !characters.is_empty() => {
                let end = characters
                    .chars()
                    .next()
                    .expect("nonempty PPT String")
                    .len_utf8();
                characters.replace_range(..end, &replacement.to_string());
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
) -> char {
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
            (PptRecordData::TextChars(characters), true)
            | (PptRecordData::TextBytes(characters), false) => characters.chars().next(),
            _ => None,
        })
        .expect("edited PPT text atom remains present")
}

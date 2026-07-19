use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

use olecfsdk::doc::{
    DocBlockRef, DocDataNodeValue, DocFile, DocOfficeArtColor, DocOfficeArtFill,
    DocOfficeArtImageLink, DocOfficeArtLine, DocParagraphRef, DocSpecialContentRef,
    DocTextPieceValueRef, FieldDocumentPart, KnownSprm, SprmKind, SprmOperand, StyleFormatting,
};
use olecfsdk::office_art::{OfficeArtClientAnchor, OfficeArtImageFormat, OfficeArtShapeFlags};
use olecfsdk::ppt::{
    PptFile, PptLiveImageLink, PptLiveNotesLink, PptLiveShapeRef, PptLiveTextAtomRef,
};
use olecfsdk::shared_content::{OfficePropertySetKind, OfficeSharedContent};
use olecfsdk::xls::{
    CellErrorCode, ExtFontScheme, ExtPropertyData, XlsCellValue, XlsFile, XlsFormulaCachedValue,
    XlsHyperlinkTarget, XlsNumberFormatRef, XlsPictureImageLink,
};
use olecfsdk_corpus_test_support::corpus_file_path;
use olecfsdk_ooxml::{
    ConversionCode, ConversionOptions, Error, LossPolicy, SourceLocation, convert_doc,
    convert_doc_with_options, convert_ppt, convert_ppt_with_options, convert_xls,
    convert_xls_with_options,
};
use ooxmlsdk::schemas::schemas_microsoft_com_office_word_2010_wordprocessing_shape as wps;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::{
    self as a, ParagraphChoice as DrawingParagraphChoice,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_wordprocessing_drawing as wp;
use ooxmlsdk::schemas::schemas_openxmlformats_org_presentationml_2006_main::{
    self as p, ShapeTreeChoice,
};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::{
    self as x, CellValues,
};
use ooxmlsdk::{
    parts::image_part::ImagePart,
    parts::presentation_document::PresentationDocument,
    parts::spreadsheet_document::SpreadsheetDocument,
    parts::wordprocessing_document::WordprocessingDocument,
    schemas::opc_core_properties::CoreProperties,
    schemas::schemas_openxmlformats_org_wordprocessingml_2006_main::{
        BodyChoice, CommentChoice, DrawingChoice, EndnoteChoice, FieldCharValues, FootnoteChoice,
        LineSpacingRuleValues, Paragraph, ParagraphChoice, RunChoice, RunPropertiesChoice,
        SectionMarkValues, SectionProperties, TableCellChoice, TableChoice2, TableRowChoice,
    },
    sdk::SdkPart,
    simple_type::{CoordinateValue, HpsMeasureValue, SignedTwipsMeasureValue},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ExpectedCoreProperties {
    title: Option<String>,
    subject: Option<String>,
    creator: Option<String>,
    keywords: Option<String>,
    description: Option<String>,
    last_modified_by: Option<String>,
    revision: Option<String>,
    category: Option<String>,
    content_type: Option<String>,
    content_status: Option<String>,
    language: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExpectedFieldToken {
    Text(String),
    Instruction(String),
    Marker(FieldCharValues),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BookmarkSnapshot {
    name: String,
    column_first: Option<i32>,
    column_last: Option<i32>,
    content: String,
}

#[test]
fn doc_conversion_preserves_shared_oleps_core_properties() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/simple.doc"))
        .expect("strictly open DOC core-properties fixture");
    let expected = source_core_properties(&source.shared);
    assert!(expected.mapped_count() >= 4, "fixture metadata is too weak");

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC core properties with explicit unrelated losses");
    let mut document = converted.document;
    assert_eq!(target_doc_core_properties(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save DOCX core properties");
    assert_core_property_namespaces(bytes.get_ref());
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen DOCX core properties");
    assert_eq!(target_doc_core_properties(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save DOCX core properties a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen DOCX core properties a second time");
    assert_eq!(target_doc_core_properties(&mut second), expected);
}

#[test]
fn doc_conversion_preserves_complex_hyperlink_field_structure() {
    let path = fixture("Apache-POI/test-data/document/hyperlink.doc");
    assert!(DocFile::open(&path).is_err());
    let opened =
        DocFile::open_compatible(&path).expect("compatibly open diagnosed DOC hyperlink fixture");
    assert!(!opened.diagnostics.is_empty());
    let source = opened.value;
    let main = source
        .content_tree()
        .expect("resolve DOC hyperlink tree")
        .part(FieldDocumentPart::Main)
        .expect("DOC hyperlink fixture has a main story");
    let fields = main.fields().collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].source().begin.field_type, 0x58);

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC hyperlink with explicit unrelated losses");
    assert!(converted.report.issues().iter().all(|issue| {
        issue.code != ConversionCode::ControlCharacterNotMapped
            || !matches!(
                issue.source,
                SourceLocation::DocRange {
                    part: FieldDocumentPart::Main,
                    ..
                }
            )
    }));
    let expected = vec![
        ExpectedFieldToken::Text("Before text; ".into()),
        ExpectedFieldToken::Marker(FieldCharValues::Begin),
        ExpectedFieldToken::Instruction(" HYPERLINK \"http://testuri.org/\"".into()),
        ExpectedFieldToken::Marker(FieldCharValues::Separate),
        ExpectedFieldToken::Text("Hyperlink text".into()),
        ExpectedFieldToken::Marker(FieldCharValues::End),
        ExpectedFieldToken::Text("; after text".into()),
    ];
    let mut document = converted.document;
    assert_eq!(target_doc_field_tokens(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save DOCX hyperlink field");
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen DOCX hyperlink field");
    assert_eq!(target_doc_field_tokens(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save DOCX hyperlink field a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen DOCX hyperlink field a second time");
    assert_eq!(target_doc_field_tokens(&mut second), expected);
}

#[test]
fn doc_conversion_preserves_typed_bookmark_pairs_across_two_cycles() {
    for (relative, minimum_bookmarks, minimum_cross_paragraph) in [
        ("Apache-POI/test-data/document/Bug47287.doc", 12, 0),
        ("Apache-POI/test-data/document/Bug47731.doc", 4, 0),
        ("Apache-POI/test-data/document/59322.doc", 1, 1),
    ] {
        let source = DocFile::open(fixture(relative)).expect("strictly open DOC bookmark fixture");
        let expected = source_bookmarks(&source);
        assert!(
            expected.len() >= minimum_bookmarks,
            "bookmark fixture {relative} is too weak"
        );
        assert!(
            source_cross_paragraph_bookmark_count(&source) >= minimum_cross_paragraph,
            "bookmark fixture {relative} lacks its required cross-paragraph range"
        );

        let converted = convert_doc_with_options(
            &source,
            ConversionOptions {
                unsupported: LossPolicy::Report,
            },
        )
        .expect("convert typed DOC bookmarks with explicit unrelated losses");
        assert!(
            converted.report.issues().iter().all(|issue| !matches!(
                issue.code,
                ConversionCode::BookmarkNameCompatibilityUtf16
                    | ConversionCode::BookmarkNameNotMapped
                    | ConversionCode::BookmarkColumnRangeNotMapped
                    | ConversionCode::BookmarkStoryNotMapped
                    | ConversionCode::BookmarkBoundaryNotMapped
            )),
            "{relative}: {:?}",
            converted.report.issues()
        );
        let mut document = converted.document;
        assert_eq!(target_bookmarks(&mut document), expected, "{relative}");

        let mut bytes = Cursor::new(Vec::new());
        document
            .save(&mut bytes)
            .expect("save converted DOCX bookmarks");
        let saved = bytes.into_inner();
        assert_package_entry_contains(&saved, "word/document.xml", "w:bookmarkStart");
        assert_package_entry_contains(&saved, "word/document.xml", "w:bookmarkEnd");
        let mut reopened = WordprocessingDocument::new(Cursor::new(saved))
            .expect("reopen converted DOCX bookmarks");
        assert_eq!(target_bookmarks(&mut reopened), expected, "{relative}");

        let mut second_bytes = Cursor::new(Vec::new());
        reopened
            .save(&mut second_bytes)
            .expect("save converted DOCX bookmarks a second time");
        let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
            .expect("reopen converted DOCX bookmarks a second time");
        assert_eq!(target_bookmarks(&mut second), expected, "{relative}");
    }
}

#[test]
fn doc_conversion_preserves_footnote_and_endnote_parts_and_references() {
    let path = fixture("Apache-POI/test-data/document/footnote.doc");
    let opened = DocFile::open_compatible(&path).expect("compatibly open DOC note fixture");
    let source = opened.value;
    let expected = source_notes(&source);
    assert_eq!(expected.0.len(), 1, "fixture footnote count");
    assert_eq!(expected.1.len(), 1, "fixture endnote count");

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert typed DOC notes with explicit unrelated losses");
    assert!(
        converted.report.issues().iter().all(|issue| !matches!(
            issue.code,
            ConversionCode::NoteCustomMarkNotMapped | ConversionCode::NoteBoundaryNotMapped
        )),
        "unexpected note loss: {:?}",
        converted.report.issues()
    );
    let mut document = converted.document;
    assert_eq!(target_notes(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save converted DOCX notes");
    let saved = bytes.into_inner();
    assert_package_entry_contains(&saved, "word/document.xml", "w:footnoteReference");
    assert_package_entry_contains(&saved, "word/document.xml", "w:endnoteReference");
    assert_package_entry_contains(&saved, "word/footnotes.xml", "w:footnoteRef");
    assert_package_entry_contains(&saved, "word/endnotes.xml", "w:endnoteRef");
    let mut reopened =
        WordprocessingDocument::new(Cursor::new(saved)).expect("reopen converted DOCX notes");
    assert_eq!(target_notes(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save converted DOCX notes a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen converted DOCX notes a second time");
    assert_eq!(target_notes(&mut second), expected);
}

#[test]
fn doc_conversion_preserves_typed_comments_and_ranges_across_two_cycles() {
    for relative in [
        "Apache-POI/test-data/document/MarkAuthorsTable.doc",
        "LibreOffice/sw/qa/extras/ww8export/data/commented-table.doc",
    ] {
        let opened = DocFile::open_compatible(fixture(relative))
            .expect("compatibly open DOC comment fixture");
        let source = opened.value;
        let expected = source_comments(&source);
        assert!(!expected.is_empty(), "fixture comment count: {relative}");

        let converted = convert_doc_with_options(
            &source,
            ConversionOptions {
                unsupported: LossPolicy::Report,
            },
        )
        .expect("convert typed DOC comments with explicit unrelated losses");
        assert!(
            converted.report.issues().iter().all(|issue| !matches!(
                issue.code,
                ConversionCode::CommentRelationshipNotMapped
                    | ConversionCode::CommentMetadataNotMapped
                    | ConversionCode::CommentThreadNotMapped
                    | ConversionCode::CommentInkNotMapped
                    | ConversionCode::CommentBoundaryNotMapped
            )),
            "unexpected comment loss in {relative}: {:?}",
            converted.report.issues()
        );
        let mut document = converted.document;
        assert_eq!(target_comments(&mut document), expected, "{relative}");

        let mut bytes = Cursor::new(Vec::new());
        document
            .save(&mut bytes)
            .expect("save converted DOCX comments");
        let saved = bytes.into_inner();
        assert_package_entry_contains(&saved, "word/document.xml", "w:commentRangeStart");
        assert_package_entry_contains(&saved, "word/document.xml", "w:commentRangeEnd");
        assert_package_entry_contains(&saved, "word/document.xml", "w:commentReference");
        assert_package_entry_contains(&saved, "word/comments.xml", "w:annotationRef");
        let mut reopened = WordprocessingDocument::new(Cursor::new(saved))
            .expect("reopen converted DOCX comments");
        assert_eq!(target_comments(&mut reopened), expected, "{relative}");

        let mut second_bytes = Cursor::new(Vec::new());
        reopened
            .save(&mut second_bytes)
            .expect("save converted DOCX comments a second time");
        let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
            .expect("reopen converted DOCX comments a second time");
        assert_eq!(target_comments(&mut second), expected, "{relative}");
    }
}

#[test]
fn doc_conversion_preserves_floating_textbox_identity_geometry_and_content() {
    for relative in [
        "LibreOffice/sw/qa/extras/ww8export/data/tdf76349_textboxMargins.doc",
        "LibreOffice/sw/qa/extras/ww8export/data/tdf101826_xattrTextBoxFill.doc",
    ] {
        assert_floating_textbox_conversion(relative);
    }
}

#[test]
fn doc_conversion_preserves_typed_floating_line_ellipse_and_shape_geometry() {
    for relative in [
        "LibreOffice/sw/qa/extras/ww8export/data/shapes-line-ellipse.doc",
        "LibreOffice/sw/qa/extras/ww8export/data/tdf112618_textbox_no_bg.doc",
    ] {
        assert_floating_shape_conversion(relative);
    }
}

fn assert_floating_shape_conversion(relative: &str) {
    let source = DocFile::open_compatible(fixture(relative))
        .expect("compatibly open floating-shape DOC")
        .value;
    let expected = source_floating_shapes(&source);
    assert!(
        !expected.is_empty(),
        "fixture contains non-text, non-picture floating shapes: {relative}"
    );
    let shape_ids = expected.keys().copied().collect::<Vec<_>>();

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert typed floating DOC shapes");
    let shape_issues = converted
        .report
        .issues()
        .iter()
        .filter(|issue| {
            matches!(
                issue.code,
                ConversionCode::FloatingShapeNotMapped
                    | ConversionCode::FloatingShapeGeometryNotMapped
                    | ConversionCode::FloatingShapeFormattingNotMapped
            ) && matches!(
                issue.source,
                SourceLocation::DocTextbox { shape_id, .. } if shape_ids.contains(&shape_id)
            )
        })
        .collect::<Vec<_>>();
    let shape_sources = source
        .content_tree_compatible()
        .expect("rebuild compatible floating-shape tree")
        .main_textboxes()
        .expect("rejoin compatible floating shapes")
        .anchors()
        .iter()
        .filter_map(|anchor| {
            let shape = anchor.shape()?;
            shape_ids.contains(&shape.shape().shape_id).then_some((
                shape.shape().shape_id,
                shape.shape_type(),
                *anchor.source(),
            ))
        })
        .collect::<Vec<_>>();
    assert!(
        shape_issues.is_empty(),
        "supported floating shapes have no silent or diagnosed loss in {relative}: {shape_issues:?}; sources={shape_sources:?}"
    );
    let mut document = converted.document;
    assert_eq!(
        target_floating_shapes(&mut document),
        expected,
        "{relative}"
    );

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save converted floating shapes");
    let saved = bytes.into_inner();
    assert_package_entry_contains(&saved, "word/document.xml", "wps:wsp");
    let mut reopened =
        WordprocessingDocument::new(Cursor::new(saved)).expect("reopen converted floating shapes");
    assert_eq!(
        target_floating_shapes(&mut reopened),
        expected,
        "{relative}"
    );

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save converted floating shapes a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen converted floating shapes a second time");
    assert_eq!(target_floating_shapes(&mut second), expected, "{relative}");
}

fn assert_floating_textbox_conversion(relative: &str) {
    let source = DocFile::open_compatible(fixture(relative))
        .expect("compatibly open floating textbox DOC")
        .value;
    let (expected, expected_formatting_losses) = source_floating_textboxes(&source);
    assert!(
        !expected.is_empty(),
        "fixture contains floating textboxes: {relative}"
    );

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert floating DOC textboxes");
    assert!(
        converted.report.issues().iter().all(|issue| !matches!(
            issue.code,
            ConversionCode::TextboxRelationshipNotMapped
                | ConversionCode::TextboxBoundaryNotMapped
                | ConversionCode::TextboxFlowNotMapped
                | ConversionCode::FloatingShapeNotMapped
                | ConversionCode::FloatingShapeGeometryNotMapped
        )),
        "unexpected textbox loss in {relative}: {:?}",
        converted.report.issues()
    );
    let mut actual_formatting_losses = converted
        .report
        .issues()
        .iter()
        .filter(|issue| issue.code == ConversionCode::FloatingShapeFormattingNotMapped)
        .map(|issue| match issue.source {
            SourceLocation::DocTextbox { shape_id, .. } => shape_id,
            source => panic!("floating formatting issue has wrong source {source:?}"),
        })
        .collect::<Vec<_>>();
    actual_formatting_losses.sort_unstable();
    assert_eq!(
        actual_formatting_losses, expected_formatting_losses,
        "every unsupported floating formatting unit is diagnosed exactly once: {relative}"
    );
    let mut document = converted.document;
    assert_eq!(
        target_floating_textboxes(&mut document),
        expected,
        "{relative}"
    );

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save converted floating textboxes");
    let saved = bytes.into_inner();
    assert_package_entry_contains(&saved, "word/document.xml", "wp:anchor");
    assert_package_entry_contains(&saved, "word/document.xml", "wps:txbx");
    assert_package_entry_contains(&saved, "word/document.xml", "w:txbxContent");
    assert_package_entry_contains(&saved, "word/document.xml", "wps:bodyPr");
    let mut reopened = WordprocessingDocument::new(Cursor::new(saved))
        .expect("reopen converted floating textboxes");
    assert_eq!(
        target_floating_textboxes(&mut reopened),
        expected,
        "{relative}"
    );

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save converted floating textboxes a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen converted floating textboxes a second time");
    assert_eq!(
        target_floating_textboxes(&mut second),
        expected,
        "{relative}"
    );
}

#[test]
fn xls_conversion_preserves_shared_oleps_core_properties() {
    let source = XlsFile::open(fixture("Apache-POI/test-data/spreadsheet/Simple.xls"))
        .expect("strictly open XLS core-properties fixture");
    let expected = source_core_properties(&source.shared);
    assert!(expected.mapped_count() >= 2, "fixture metadata is too weak");

    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS core properties with explicit unrelated losses");
    let mut document = converted.document;
    assert_eq!(target_xls_core_properties(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save XLSX core properties");
    assert_core_property_namespaces(bytes.get_ref());
    let mut reopened = SpreadsheetDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen XLSX core properties");
    assert_eq!(target_xls_core_properties(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save XLSX core properties a second time");
    let mut second = SpreadsheetDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen XLSX core properties a second time");
    assert_eq!(target_xls_core_properties(&mut second), expected);
}

#[test]
fn ppt_conversion_preserves_shared_oleps_core_properties() {
    let source = PptFile::open(fixture(
        "Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt",
    ))
    .expect("strictly open PPT core-properties fixture");
    let expected = source_core_properties(&source.shared);
    assert!(expected.mapped_count() >= 2, "fixture metadata is too weak");

    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert PPT core properties with explicit unrelated losses");
    let mut document = converted.document;
    assert_eq!(target_ppt_core_properties(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document
        .save(&mut bytes)
        .expect("save PPTX core properties");
    assert_core_property_namespaces(bytes.get_ref());
    let mut reopened = PresentationDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen PPTX core properties");
    assert_eq!(target_ppt_core_properties(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save PPTX core properties a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen PPTX core properties a second time");
    assert_eq!(target_ppt_core_properties(&mut second), expected);
}

#[test]
fn doc_conversion_preserves_inline_jpeg_png_payloads_and_scaled_extents() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/two_images.doc"))
        .expect("strictly open DOC inline-image fixture");
    let expected = source_inline_images(&source);
    assert_eq!(
        expected
            .iter()
            .map(|image| image.content_type.as_str())
            .collect::<Vec<_>>(),
        ["image/jpeg", "image/png"]
    );
    assert!(
        expected
            .iter()
            .all(|image| (image.cx, image.cy) == (254_000, 254_000))
    );

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC inline images with explicit unrelated losses");
    let media_issues = converted
        .report
        .issues()
        .iter()
        .filter(|issue| {
            matches!(
                issue.code,
                ConversionCode::InlinePictureNotMapped
                    | ConversionCode::ControlCharacterNotMapped
                    | ConversionCode::CharacterFormattingNotMapped
            )
        })
        .collect::<Vec<_>>();
    assert!(
        media_issues.is_empty(),
        "unexpected media issues: {media_issues:?}"
    );
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save DOCX inline-image conversion");
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen DOCX inline-image conversion");
    assert_eq!(target_inline_images(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save DOCX inline-image conversion a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen DOCX inline-image conversion a second time");
    assert_eq!(target_inline_images(&mut second), expected);
}

#[test]
fn doc_conversion_preserves_decoded_inline_emf_payload_and_scaled_extent() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/vector_image.doc"))
        .expect("strictly open DOC inline-EMF fixture");
    let expected = source_inline_images(&source);
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].content_type, "image/x-emf");
    assert_eq!(expected[0].data.len(), 7_348);

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC inline EMF with explicit malformed-section degradation");
    assert!(
        converted
            .report
            .issues()
            .iter()
            .any(|issue| { issue.code == ConversionCode::SectionBoundaryNotMapped })
    );
    assert!(converted.report.issues().iter().all(|issue| {
        !matches!(
            issue.code,
            ConversionCode::InlinePictureNotMapped | ConversionCode::ControlCharacterNotMapped
        )
    }));
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save DOCX inline-EMF conversion");
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen DOCX inline-EMF conversion");
    assert_eq!(target_inline_images(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save DOCX inline-EMF conversion a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen DOCX inline-EMF conversion a second time");
    assert_eq!(target_inline_images(&mut second), expected);
}

#[test]
fn doc_conversion_preserves_floating_picture_payload_identity_and_anchor() {
    let source = DocFile::open_compatible(fixture(
        "Apache-POI/test-data/document/FloatingPictures.doc",
    ))
    .expect("compatibly open DOC floating-picture fixture")
    .value;
    let expected = source_floating_images(&source);
    assert!(!expected.is_empty(), "fixture contains floating pictures");

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC floating pictures");
    let picture_shape_ids = expected
        .iter()
        .map(|image| image.shape_id)
        .collect::<Vec<_>>();
    let unexpected_picture_issues = converted
        .report
        .issues()
        .iter()
        .filter(|issue| {
            if issue.code == ConversionCode::FloatingPictureNotMapped {
                return true;
            }
            matches!(
                issue.code,
                ConversionCode::FloatingShapeNotMapped
                    | ConversionCode::FloatingShapeGeometryNotMapped
            ) && matches!(
                issue.source,
                SourceLocation::DocTextbox { shape_id, .. }
                    if picture_shape_ids.contains(&shape_id)
            )
        })
        .collect::<Vec<_>>();
    let source_picture_anchors = source
        .content_tree()
        .expect("rebuild floating-picture source tree")
        .main_textboxes()
        .expect("rejoin floating-picture anchors")
        .anchors()
        .iter()
        .filter(|anchor| picture_shape_ids.contains(&anchor.source().shape_id))
        .map(|anchor| *anchor.source())
        .collect::<Vec<_>>();
    assert!(
        unexpected_picture_issues.is_empty(),
        "unexpected floating-picture loss: {unexpected_picture_issues:?}; anchors={source_picture_anchors:?}"
    );
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save floating-picture DOCX");
    let saved = bytes.into_inner();
    assert_package_entry_contains(&saved, "word/document.xml", "wp:anchor");
    assert_package_entry_contains(&saved, "word/document.xml", "pic:pic");
    let mut reopened =
        WordprocessingDocument::new(Cursor::new(saved)).expect("reopen floating-picture DOCX");
    assert_eq!(target_floating_images(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save floating-picture DOCX a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen floating-picture DOCX a second time");
    assert_eq!(target_floating_images(&mut second), expected);
}

#[derive(Debug, PartialEq, Eq)]
struct FloatingImage {
    shape_id: u32,
    content_type: String,
    data: Vec<u8>,
    left: i32,
    top: i32,
    width: i64,
    height: i64,
    z_order: u32,
    crop: [i32; 4],
}

fn source_floating_images(source: &DocFile) -> Vec<FloatingImage> {
    let tree = source
        .content_tree()
        .expect("build strict DOC floating-image tree");
    let textboxes = tree
        .main_textboxes()
        .expect("resolve strict DOC floating-image relationships");
    let mut images = textboxes
        .anchors()
        .iter()
        .filter_map(|anchor| {
            let shape = anchor.shape()?;
            let identifier = shape
                .primary_blip_identifier()
                .expect("fixture has valid primary BLIP properties")?;
            let DocOfficeArtImageLink::Resolved(image) =
                source
                    .office_art_image_link(identifier)
                    .expect("resolve fixture OfficeArt image store")?
            else {
                panic!("fixture floating image resolves to a typed BLIP")
            };
            let rectangle = anchor.source().rectangle;
            let crop = shape.picture_crop();
            let to_emu = |value: i32| {
                i32::try_from(i64::from(value) * ooxmlsdk::units::EMUS_PER_TWIP)
                    .expect("fixture floating-image coordinate fits i32")
            };
            Some(FloatingImage {
                shape_id: shape.shape().shape_id,
                content_type: image_content_type_for_test(image.format).into(),
                data: image.data.to_vec(),
                left: to_emu(rectangle.left),
                top: to_emu(rectangle.top),
                width: i64::from(to_emu(rectangle.right - rectangle.left)),
                height: i64::from(to_emu(rectangle.bottom - rectangle.top)),
                z_order: u32::try_from(shape.z_order()).expect("fixture z-order fits u32"),
                crop: [crop.left(), crop.top(), crop.right(), crop.bottom()]
                    .map(fixed_16_16_to_percentage_for_test),
            })
        })
        .collect::<Vec<_>>();
    images.sort_unstable_by_key(|image| image.shape_id);
    images
}

fn image_content_type_for_test(format: OfficeArtImageFormat) -> &'static str {
    match format {
        OfficeArtImageFormat::Emf => "image/x-emf",
        OfficeArtImageFormat::Wmf => "image/x-wmf",
        OfficeArtImageFormat::Pict => "image/x-pict",
        OfficeArtImageFormat::Jpeg => "image/jpeg",
        OfficeArtImageFormat::Png => "image/png",
        OfficeArtImageFormat::Dib => "image/bmp",
        OfficeArtImageFormat::Tiff => "image/tiff",
    }
}

#[derive(Debug, PartialEq, Eq)]
struct InlineImage {
    content_type: String,
    data: Vec<u8>,
    cx: i64,
    cy: i64,
}

fn source_inline_images(source: &DocFile) -> Vec<InlineImage> {
    let tree = source
        .content_tree()
        .expect("build strict DOC inline-image tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    main.special_contents()
        .expect("resolve strict DOC inline-image relationships")
        .into_iter()
        .filter_map(|content| {
            let DocSpecialContentRef::Picture { data_node, .. } = content else {
                return None;
            };
            let DocDataNodeValue::Picture(picture) = &data_node.value else {
                unreachable!("typed picture relationships point at picture Data nodes")
            };
            let mut image = None;
            let mut count = 0;
            picture.picture.visit(|record| {
                if let Some(candidate) = record.image_ref() {
                    count += 1;
                    image.get_or_insert(candidate);
                }
            });
            if count != 1 {
                return None;
            }
            let image = image.expect("counted fixture BLIP remains borrowed");
            let content_type = match image.format {
                OfficeArtImageFormat::Jpeg => "image/jpeg",
                OfficeArtImageFormat::Png => "image/png",
                OfficeArtImageFormat::Emf => "image/x-emf",
                OfficeArtImageFormat::Wmf => "image/x-wmf",
                other => panic!("unexpected fixture image format {other:?}"),
            };
            let dimensions = picture.picf.picture;
            Some(InlineImage {
                content_type: content_type.into(),
                data: image.data.to_vec(),
                cx: scaled_image_extent(
                    dimensions.goal_width_twips,
                    dimensions.horizontal_scale_tenths_percent,
                ),
                cy: scaled_image_extent(
                    dimensions.goal_height_twips,
                    dimensions.vertical_scale_tenths_percent,
                ),
            })
        })
        .collect()
}

fn scaled_image_extent(goal_twips: i16, scale_tenths_percent: u16) -> i64 {
    (i64::from(goal_twips) * i64::from(scale_tenths_percent) * ooxmlsdk::units::EMUS_PER_TWIP + 500)
        / 1_000
}

fn target_inline_images(document: &mut WordprocessingDocument) -> Vec<InlineImage> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let drawings = {
        let root = main
            .root_element(document)
            .expect("parse converted DOCX inline-image root");
        root.body
            .as_deref()
            .expect("converted DOCX has a body")
            .body_choice
            .iter()
            .filter_map(|choice| match choice {
                BodyChoice::Paragraph(paragraph) => Some(paragraph),
                _ => None,
            })
            .flat_map(|paragraph| paragraph.paragraph_choice.iter())
            .filter_map(|choice| match choice {
                ParagraphChoice::WRun(run) => Some(run),
                _ => None,
            })
            .flat_map(|run| run.run_choice.iter())
            .filter_map(|choice| match choice {
                RunChoice::Drawing(drawing) => drawing.drawing_choice.as_ref(),
                _ => None,
            })
            .filter_map(|choice| match choice {
                ooxmlsdk::schemas::w::DrawingChoice::Inline(inline) => Some(inline),
                _ => None,
            })
            .map(|inline| {
                let relationship_id = inline
                    .graphic
                    .graphic_data
                    .graphic_data_choice
                    .iter()
                    .find_map(|choice| match choice {
                        ooxmlsdk::schemas::a::GraphicDataChoice::Picture(picture) => picture
                            .blip_fill
                            .as_deref()
                            .and_then(|fill| fill.blip.as_deref())
                            .and_then(|blip| blip.embed.as_deref()),
                        _ => None,
                    })
                    .expect("typed inline picture embeds an image relationship")
                    .to_owned();
                (relationship_id, inline.extent.cx, inline.extent.cy)
            })
            .collect::<Vec<_>>()
    };
    let mut media = main
        .related_parts_of_type::<_, ImagePart>(document)
        .map(|related| {
            (
                related.relationship_id().to_owned(),
                (
                    related
                        .part()
                        .content_type(document)
                        .expect("converted image part has a content type")
                        .to_owned(),
                    related
                        .part()
                        .data_to_vec(document)
                        .expect("converted image part has payload data"),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    drawings
        .into_iter()
        .map(|(relationship_id, cx, cy)| {
            let (content_type, data) = media
                .remove(&relationship_id)
                .expect("inline picture relationship resolves to an ImagePart");
            InlineImage {
                content_type,
                data,
                cx,
                cy,
            }
        })
        .collect()
}

fn target_floating_images(document: &mut WordprocessingDocument) -> Vec<FloatingImage> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let drawings = {
        let root = main
            .root_element(document)
            .expect("parse converted DOCX floating-image root");
        let mut drawings = Vec::new();
        for choice in &root
            .body
            .as_deref()
            .expect("converted DOCX has a body")
            .body_choice
        {
            collect_target_floating_image_body(choice, &mut drawings);
        }
        drawings
    };
    let media = main
        .related_parts_of_type::<_, ImagePart>(document)
        .map(|related| {
            (
                related.relationship_id().to_owned(),
                (
                    related
                        .part()
                        .content_type(document)
                        .expect("converted floating image has a content type")
                        .to_owned(),
                    related
                        .part()
                        .data_to_vec(document)
                        .expect("converted floating image has payload data"),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut images = drawings
        .into_iter()
        .map(
            |(relationship_id, shape_id, left, top, width, height, z_order, crop)| {
                let (content_type, data) = media
                    .get(&relationship_id)
                    .expect("floating picture relationship resolves to an ImagePart");
                FloatingImage {
                    shape_id,
                    content_type: content_type.clone(),
                    data: data.clone(),
                    left,
                    top,
                    width,
                    height,
                    z_order,
                    crop,
                }
            },
        )
        .collect::<Vec<_>>();
    images.sort_unstable_by_key(|image| image.shape_id);
    images
}

type TargetFloatingImage = (String, u32, i32, i32, i64, i64, u32, [i32; 4]);

fn collect_target_floating_image_body(choice: &BodyChoice, images: &mut Vec<TargetFloatingImage>) {
    match choice {
        BodyChoice::Paragraph(paragraph) => {
            collect_target_floating_image_paragraph(paragraph, images)
        }
        BodyChoice::Table(table) => collect_target_floating_image_table(table, images),
        _ => {}
    }
}

fn collect_target_floating_image_table(
    table: &ooxmlsdk::schemas::w::Table,
    images: &mut Vec<TargetFloatingImage>,
) {
    for row in table
        .table_choice2
        .iter()
        .filter_map(|choice| match choice {
            TableChoice2::TableRow(row) => Some(row),
            _ => None,
        })
    {
        for cell in row
            .table_row_choice
            .iter()
            .filter_map(|choice| match choice {
                TableRowChoice::TableCell(cell) => Some(cell),
                _ => None,
            })
        {
            for choice in &cell.table_cell_choice {
                match choice {
                    TableCellChoice::Paragraph(paragraph) => {
                        collect_target_floating_image_paragraph(paragraph, images)
                    }
                    TableCellChoice::Table(table) => {
                        collect_target_floating_image_table(table, images)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn collect_target_floating_image_paragraph(
    paragraph: &Paragraph,
    images: &mut Vec<TargetFloatingImage>,
) {
    for anchor in paragraph
        .paragraph_choice
        .iter()
        .filter_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run),
            _ => None,
        })
        .flat_map(|run| &run.run_choice)
        .filter_map(|choice| match choice {
            RunChoice::Drawing(drawing) => drawing.drawing_choice.as_ref(),
            _ => None,
        })
        .filter_map(|choice| match choice {
            DrawingChoice::Anchor(anchor) => Some(anchor.as_ref()),
            _ => None,
        })
    {
        let Some(picture) = anchor
            .graphic
            .graphic_data
            .graphic_data_choice
            .iter()
            .find_map(|choice| match choice {
                a::GraphicDataChoice::Picture(picture) => Some(picture.as_ref()),
                _ => None,
            })
        else {
            continue;
        };
        let relationship_id = picture
            .blip_fill
            .as_deref()
            .and_then(|fill| fill.blip.as_deref())
            .and_then(|blip| blip.embed.as_deref())
            .expect("floating picture embeds an image relationship")
            .to_owned();
        let shape_id = picture
            .non_visual_picture_properties
            .as_deref()
            .expect("floating picture retains nonvisual identity")
            .non_visual_drawing_properties
            .id;
        let crop = picture
            .blip_fill
            .as_deref()
            .and_then(|fill| fill.source_rectangle.as_ref())
            .map_or([0; 4], |rectangle| {
                [
                    target_percentage_for_test(rectangle.left),
                    target_percentage_for_test(rectangle.top),
                    target_percentage_for_test(rectangle.right),
                    target_percentage_for_test(rectangle.bottom),
                ]
            });
        let left = match anchor
            .horizontal_position
            .as_deref()
            .and_then(|position| position.horizontal_position_choice.as_ref())
        {
            Some(wp::HorizontalPositionChoice::PositionOffset(value)) => *value,
            _ => panic!("floating picture has exact horizontal positioning"),
        };
        let top = match anchor
            .vertical_position
            .as_deref()
            .and_then(|position| position.vertical_position_choice.as_ref())
        {
            Some(wp::VerticalPositionChoice::PositionOffset(value)) => *value,
            _ => panic!("floating picture has exact vertical positioning"),
        };
        images.push((
            relationship_id,
            shape_id,
            left,
            top,
            anchor.extent.cx,
            anchor.extent.cy,
            anchor
                .relative_height
                .expect("floating picture retains z-order"),
            crop,
        ));
    }
}

fn fixed_16_16_to_percentage_for_test(value: i32) -> i32 {
    let scaled = i64::from(value) * 100_000;
    i32::try_from(if scaled < 0 {
        (scaled - 32_768) / 65_536
    } else {
        (scaled + 32_768) / 65_536
    })
    .expect("fixture crop fraction fits DrawingML percentage")
}

fn target_percentage_for_test(
    value: Option<ooxmlsdk::simple_type::DrawingmlPercentageValue>,
) -> i32 {
    match value.unwrap_or_default() {
        ooxmlsdk::simple_type::DrawingmlPercentageValue::Decimal(value)
        | ooxmlsdk::simple_type::DrawingmlPercentageValue::PercentString(value) => value,
    }
}

#[test]
fn doc_conversion_uses_clipped_typed_segments_and_reports_unmapped_formatting() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/simple.doc"))
        .expect("strictly open DOC conversion fixture");
    let error = convert_doc(&source).expect_err("strict conversion rejects formatting loss");
    assert!(matches!(
        error,
        Error::Unsupported {
            code: ConversionCode::StyleFormattingNotMapped,
            ..
        }
    ));

    let expected_paragraphs = source_paragraph_text(&source);
    let expected_styles = source_paragraph_styles(&source);
    let expected_outlines = source_paragraph_outlines(&source);
    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("explicitly convert DOC with reported degradation");
    assert!(!converted.report.issues().is_empty());
    assert!(converted.report.issues().iter().all(|issue| {
        issue.code == ConversionCode::ParagraphFormattingNotMapped
            || issue.code == ConversionCode::StyleFormattingNotMapped
            || issue.code == ConversionCode::SectionFormattingNotMapped
            || issue.code == ConversionCode::NonMainStoryNotMapped
            || issue.code == ConversionCode::CorePropertyNotMapped
    }));
    assert_eq!(
        converted.report.counts().unsupported(),
        converted.report.issues().len()
    );

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save typed DOCX conversion");
    let first_cycle = bytes.into_inner();
    let mut reopened = WordprocessingDocument::new(Cursor::new(first_cycle))
        .expect("reopen typed DOCX conversion");
    assert_eq!(target_paragraph_text(&mut reopened), expected_paragraphs);
    assert_eq!(target_paragraph_styles(&mut reopened), expected_styles);
    assert_eq!(target_paragraph_outlines(&mut reopened), expected_outlines);
    assert_target_style_references_resolve(&mut reopened);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save DOCX conversion a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen DOCX conversion a second time");
    assert_eq!(target_paragraph_text(&mut second), expected_paragraphs);
    assert_eq!(target_paragraph_styles(&mut second), expected_styles);
    assert_eq!(target_paragraph_outlines(&mut second), expected_outlines);
    assert_target_style_references_resolve(&mut second);
}

#[test]
fn doc_conversion_preserves_exact_style_spacing_and_font_sizes() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/SampleDoc.doc"))
        .expect("strictly open DOC style-formatting fixture");
    let expected_direct_font_sizes = source_direct_run_font_sizes(&source);
    assert!(expected_direct_font_sizes.contains(&(Some(32), Some(32))));
    let style = source
        .table
        .styles
        .as_ref()
        .expect("fixture has STSH")
        .value
        .styles[0]
        .definition
        .as_ref()
        .expect("fixture has normal style");
    let StyleFormatting::Paragraph {
        paragraph,
        character,
    } = &style.formatting
    else {
        panic!("fixture normal style is a paragraph style")
    };
    assert!(paragraph.properties.properties.iter().any(|property| {
        property.sprm.kind() == SprmKind::Known(KnownSprm::PDyaLine)
            && property.operand == SprmOperand::Dword([20, 1, 1, 0])
    }));
    assert!(paragraph.properties.properties.iter().any(|property| {
        property.sprm.kind() == SprmKind::Known(KnownSprm::PDyaAfter)
            && property.operand == SprmOperand::Word5([200, 0])
    }));
    for known in [KnownSprm::CHps, KnownSprm::CHpsBi] {
        assert!(character.properties.properties.iter().any(|property| {
            property.sprm.kind() == SprmKind::Known(known)
                && property.operand == SprmOperand::Word([22, 0])
        }));
    }

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC style formatting with explicit remaining losses");
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save style-formatting DOCX conversion");
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen style-formatting DOCX conversion");
    assert_target_sample_style_formatting(&mut reopened);
    assert_eq!(
        target_direct_run_font_sizes(&mut reopened),
        expected_direct_font_sizes
    );

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save style-formatting DOCX conversion a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen style-formatting DOCX conversion a second time");
    assert_target_sample_style_formatting(&mut second);
    assert_eq!(
        target_direct_run_font_sizes(&mut second),
        expected_direct_font_sizes
    );
}

#[test]
fn doc_conversion_preserves_direct_logical_paragraph_indentation() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/Lists.doc"))
        .expect("strictly open DOC direct-paragraph-formatting fixture");
    let tree = source
        .content_tree()
        .expect("build strict DOC paragraph tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    let paragraphs = main.paragraphs().collect::<Vec<_>>();
    let (paragraph_index, paragraph) = paragraphs
        .iter()
        .enumerate()
        .find(|(_, paragraph)| paragraph.local_cp_range().start.value() == 472)
        .expect("fixture has the 720-twip direct-indentation paragraph");
    let properties = &paragraph
        .source()
        .properties
        .as_deref()
        .expect("fixture paragraph has direct PAPX")
        .properties
        .properties;
    for known in [KnownSprm::PDxaLeft80, KnownSprm::PDxaLeft] {
        assert!(properties.iter().any(|property| {
            property.sprm.kind() == SprmKind::Known(known)
                && property.operand == SprmOperand::Word4([208, 2])
        }));
    }

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert direct paragraph indentation with explicit remaining losses");
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save direct-paragraph-formatting DOCX conversion");
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen direct-paragraph-formatting DOCX conversion");
    assert_target_paragraph_start_indent(&mut reopened, paragraph_index, 720);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save direct-paragraph-formatting DOCX conversion a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen direct-paragraph-formatting DOCX conversion a second time");
    assert_target_paragraph_start_indent(&mut second, paragraph_index, 720);
}

#[test]
fn doc_table_conversion_preserves_row_cell_and_text_structure() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/simple-table.doc"))
        .expect("strictly open DOC table conversion fixture");
    let expected = source_table_cells(&source);

    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC table with explicit formatting degradation");
    assert!(
        converted
            .report
            .issues()
            .iter()
            .any(|issue| { issue.code == ConversionCode::TableFormattingNotMapped })
    );

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save converted DOC table");
    let saved = bytes.into_inner();
    assert_xml_namespaces(
        &saved,
        "word/document.xml",
        &[
            "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"",
            "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"",
            "xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"",
            "xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\"",
        ],
    );
    let mut reopened =
        WordprocessingDocument::new(Cursor::new(saved)).expect("reopen converted DOC table");
    assert_eq!(target_table_cells(&mut reopened), expected);
}

#[test]
fn doc_conversion_preserves_section_boundaries() {
    let source = DocFile::open(fixture("Apache-POI/test-data/document/Bug53453Section.doc"))
        .expect("strictly open DOC section fixture");
    let tree = source
        .content_tree()
        .expect("build strict DOC section content tree");
    let expected = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part")
        .sections()
        .expect("resolve strict DOC sections")
        .sections()
        .len();
    assert!(expected > 1);
    let converted = convert_doc_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert DOC sections with explicit formatting degradation");
    assert!(
        converted
            .report
            .issues()
            .iter()
            .any(|issue| issue.code == ConversionCode::SectionFormattingNotMapped)
    );
    assert!(
        converted
            .report
            .issues()
            .iter()
            .all(|issue| issue.code != ConversionCode::SectionBoundaryNotMapped)
    );

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save DOCX section conversion");
    let mut reopened = WordprocessingDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen DOCX section conversion");
    assert_eq!(target_section_count(&mut reopened), expected);
    assert_target_section_layouts(&mut reopened);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save DOCX section conversion a second time");
    let mut second = WordprocessingDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen DOCX section conversion a second time");
    assert_eq!(target_section_count(&mut second), expected);
    assert_target_section_layouts(&mut second);
}

fn target_section_count(document: &mut WordprocessingDocument) -> usize {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let body = main
        .root_element(document)
        .expect("parse converted DOCX section root")
        .body
        .as_deref()
        .expect("converted DOCX has a body");
    usize::from(body.section_properties.is_some())
        + body
            .body_choice
            .iter()
            .filter(|choice| match choice {
                BodyChoice::Paragraph(paragraph) => paragraph
                    .paragraph_properties
                    .as_deref()
                    .is_some_and(|properties| properties.section_properties.is_some()),
                _ => false,
            })
            .count()
}

fn assert_target_section_layouts(document: &mut WordprocessingDocument) {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX section layouts");
    let body = root.body.as_deref().expect("converted DOCX has a body");
    let mut sections = body
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => paragraph
                .paragraph_properties
                .as_deref()
                .and_then(|properties| properties.section_properties.as_deref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    sections.push(
        body.section_properties
            .as_deref()
            .expect("converted DOCX has final section properties"),
    );
    assert_eq!(sections.len(), 2);
    assert_section_layout(sections[0], None, None);
    assert_section_layout(sections[1], Some(3), Some(SectionMarkValues::Continuous));
}

fn assert_section_layout(
    section: &SectionProperties,
    column_count: Option<i16>,
    section_type: Option<SectionMarkValues>,
) {
    let size = section
        .page_size
        .as_ref()
        .expect("converted section has typed page size");
    assert_eq!(
        size.width,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(11_906))
    );
    assert_eq!(
        size.height,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(16_838))
    );
    let margin = section
        .page_margin
        .as_ref()
        .expect("converted section has typed page margins");
    assert_eq!(
        margin.left,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(1_440))
    );
    assert_eq!(
        margin.right,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(1_440))
    );
    assert_eq!(margin.top, Some(SignedTwipsMeasureValue::Twips(1_440)));
    assert_eq!(margin.bottom, Some(SignedTwipsMeasureValue::Twips(1_440)));
    assert_eq!(
        margin.header,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(708))
    );
    assert_eq!(
        margin.footer,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(708))
    );
    assert_eq!(
        margin.gutter,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(0))
    );
    let columns = section
        .columns
        .as_ref()
        .expect("converted section has typed column layout");
    assert_eq!(columns.column_count, column_count);
    assert_eq!(
        columns.space,
        Some(ooxmlsdk::simple_type::TwipsMeasureValue::Twips(708))
    );
    assert_eq!(
        section.section_type.as_ref().map(|value| value.val),
        section_type
    );
}

#[test]
fn xls_conversion_preserves_sheet_order_sparse_coordinates_and_stored_scalars() {
    let source = XlsFile::open(fixture("Apache-POI/test-data/spreadsheet/Simple.xls"))
        .expect("strictly open XLS scalar conversion fixture");
    let expected = source_xls_cells(&source);
    assert!(expected.iter().any(|sheet| !sheet.cells.is_empty()));
    assert!(matches!(
        convert_xls(&source).expect_err("strict XLS conversion rejects known feature loss"),
        Error::Unsupported { .. }
    ));

    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS with explicit unsupported-feature reporting");
    assert!(
        converted
            .report
            .issues()
            .iter()
            .any(|issue| issue.code == ConversionCode::WorksheetFeatureNotMapped)
    );

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save typed XLSX conversion");
    let mut reopened = SpreadsheetDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen typed XLSX conversion");
    assert_eq!(target_xls_cells(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save XLSX conversion a second time");
    let mut second = SpreadsheetDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen XLSX conversion a second time");
    assert_eq!(target_xls_cells(&mut second), expected);
}

#[test]
fn xls_conversion_preserves_xf_indices_and_number_formats() {
    let source = XlsFile::open(fixture("Apache-POI/test-data/spreadsheet/Formatting.xls"))
        .expect("strictly open XLS formatting fixture");
    let expected = source_xls_number_styles(&source);
    assert_eq!(
        expected,
        [
            ExpectedXlsNumberStyle {
                reference: "B2".into(),
                style_index: expected[0].style_index,
                number_format_id: 14,
                custom_format_code: None,
                font_theme: Some(1),
                font_scheme: Some(x::FontSchemeValues::Minor),
            },
            ExpectedXlsNumberStyle {
                reference: "B3".into(),
                style_index: expected[1].style_index,
                number_format_id: 165,
                custom_format_code: Some("yyyy/mm/dd".into()),
                font_theme: Some(1),
                font_scheme: Some(x::FontSchemeValues::Minor),
            },
        ]
    );

    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS XF records with explicit unrelated degradation reporting");
    let formatting_issues = converted
        .report
        .issues()
        .iter()
        .filter(|issue| issue.code == ConversionCode::CellFormattingNotMapped)
        .collect::<Vec<_>>();
    let source_view = source.workbooks[0]
        .relationships()
        .expect("re-resolve XLS formatting relationships");
    let source_xfs = expected
        .iter()
        .map(|style| {
            (
                source_view
                    .xfs()
                    .nth(usize::try_from(style.style_index).expect("style index fits usize"))
                    .expect("cell XF exists"),
                source_view
                    .xf_extension(u16::try_from(style.style_index).expect("style index fits u16")),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        formatting_issues.iter().all(|issue| {
            issue.code != ConversionCode::CellFormattingNotMapped
                || !matches!(
                    issue.source,
                    SourceLocation::XlsCell {
                        sheet_index: 0,
                        row: 1 | 2,
                        column: 1,
                        ..
                    }
                )
        }),
        "unexpected formatting losses {formatting_issues:#?}; source XFs {source_xfs:#?}; source font 0 {:#?}",
        source_view.font(0)
    );

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save typed XLSX formatting conversion");
    let saved = bytes.into_inner();
    assert_xml_namespaces(
        &saved,
        "xl/workbook.xml",
        &["xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\""],
    );
    let mut reopened = SpreadsheetDocument::new(Cursor::new(saved))
        .expect("reopen typed XLSX formatting conversion");
    assert_eq!(target_xls_number_styles(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save XLSX formatting conversion a second time");
    let mut second = SpreadsheetDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen XLSX formatting conversion a second time");
    assert_eq!(target_xls_number_styles(&mut second), expected);
}

#[test]
fn xls_conversion_preserves_inline_formula_and_cached_string() {
    let source = XlsFile::open(fixture(
        "Apache-POI/test-data/spreadsheet/SimpleWithFormula.xls",
    ))
    .expect("strictly open XLS formula fixture");
    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS formula with explicit unrelated degradation reporting");
    assert!(converted.report.issues().iter().all(|issue| {
        issue.code != ConversionCode::FormulaNotMapped
            || issue.source
                != SourceLocation::XlsCell {
                    workbook_index: 0,
                    sheet_index: 0,
                    row: 2,
                    column: 0,
                }
    }));

    let expected = (
        "CONCATENATE(A1,A2)".to_owned(),
        Some(CellValues::String),
        Some("replacemereplaceme".to_owned()),
    );
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save typed XLSX formula conversion");
    let mut reopened = SpreadsheetDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen typed XLSX formula conversion");
    assert_eq!(target_xls_formula(&mut reopened, "A3"), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save XLSX formula conversion a second time");
    let mut second = SpreadsheetDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen XLSX formula conversion a second time");
    assert_eq!(target_xls_formula(&mut second, "A3"), expected);
}

#[test]
fn xls_conversion_preserves_merged_ranges() {
    let source = XlsFile::open(fixture("Apache-POI/test-data/spreadsheet/13796.xls"))
        .expect("strictly open XLS merged-range fixture");
    let expected = source_xls_merged_ranges(&source);
    assert!(expected.iter().any(|ranges| !ranges.is_empty()));
    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS merged ranges with explicit degradation reporting");
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save XLSX merged-range conversion");
    let mut reopened = SpreadsheetDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen XLSX merged-range conversion");
    assert_eq!(target_xls_merged_ranges(&mut reopened), expected);
}

#[test]
fn xls_conversion_preserves_comment_cells_authors_and_text() {
    let source = XlsFile::open(fixture(
        "Apache-POI/test-data/spreadsheet/SimpleWithComments.xls",
    ))
    .expect("strictly open XLS comment fixture");
    let expected = vec![
        (
            "B1".to_string(),
            "Yegor Kozlov".to_string(),
            "Yegor Kozlov:\nfirst cell".to_string(),
        ),
        (
            "B2".to_string(),
            "Yegor Kozlov".to_string(),
            "Yegor Kozlov:\nsecond cell".to_string(),
        ),
        (
            "B3".to_string(),
            "Yegor Kozlov".to_string(),
            "Yegor Kozlov:\nthird cell".to_string(),
        ),
    ];

    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS comments with explicit shape-formatting loss");
    assert_eq!(
        converted
            .report
            .issues()
            .iter()
            .filter_map(|issue| {
                (issue.code == ConversionCode::CommentFormattingNotMapped).then_some(issue.source)
            })
            .collect::<Vec<_>>(),
        vec![
            SourceLocation::XlsCell {
                workbook_index: 0,
                sheet_index: 0,
                row: 0,
                column: 1,
            },
            SourceLocation::XlsCell {
                workbook_index: 0,
                sheet_index: 0,
                row: 1,
                column: 1,
            },
            SourceLocation::XlsCell {
                workbook_index: 0,
                sheet_index: 0,
                row: 2,
                column: 1,
            },
        ]
    );
    let mut document = converted.document;
    assert_eq!(target_xls_comments(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document.save(&mut bytes).expect("save XLSX comments");
    assert_xml_namespaces(
        bytes.get_ref(),
        "xl/comments1.xml",
        &["xmlns:x=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\""],
    );
    let mut reopened =
        SpreadsheetDocument::new(Cursor::new(bytes.into_inner())).expect("reopen XLSX comments");
    assert_eq!(target_xls_comments(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save XLSX comments a second time");
    let mut second = SpreadsheetDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen XLSX comments a second time");
    assert_eq!(target_xls_comments(&mut second), expected);
}

#[test]
fn xls_conversion_preserves_url_hyperlink_relationships() {
    let source = XlsFile::open(fixture(
        "Apache-POI/test-data/spreadsheet/WithTwoHyperLinks.xls",
    ))
    .expect("strictly open XLS hyperlink fixture");
    let expected = source_xls_hyperlinks(&source);
    assert_eq!(expected.iter().map(Vec::len).sum::<usize>(), 2);
    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS hyperlinks with explicit degradation reporting");
    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save XLSX hyperlink conversion");
    let mut reopened = SpreadsheetDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen XLSX hyperlink conversion");
    assert_eq!(target_xls_hyperlinks(&mut reopened), expected);
}

#[derive(Debug, PartialEq, Eq)]
struct XlsPictureSnapshot {
    sheet_index: usize,
    shape_id: u32,
    content_type: String,
    data: Vec<u8>,
    edit_as: xdr::EditAsValues,
    marker: [i64; 8],
    crop: [i32; 4],
    horizontal_flip: bool,
    vertical_flip: bool,
}

#[test]
fn xls_conversion_preserves_picture_payload_identity_and_two_cell_anchor() {
    let source = XlsFile::open(fixture(
        "Apache-POI/test-data/spreadsheet/SimpleWithImages.xls",
    ))
    .expect("strictly open XLS picture fixture");
    let expected = source_xls_pictures(&source);
    assert!(
        expected.len() >= 2,
        "fixture has multiple worksheet pictures"
    );
    assert!(
        expected
            .iter()
            .any(|picture| picture.content_type == "image/jpeg")
    );
    assert!(
        expected
            .iter()
            .any(|picture| picture.content_type == "image/png")
    );

    let converted = convert_xls_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert XLS pictures with explicit unrelated degradation reporting");
    let picture_issues = converted
        .report
        .issues()
        .iter()
        .filter(|issue| {
            matches!(
                issue.code,
                ConversionCode::SpreadsheetPictureNotMapped
                    | ConversionCode::SpreadsheetPictureAnchorNotMapped
                    | ConversionCode::SpreadsheetPictureFormattingNotMapped
            )
        })
        .collect::<Vec<_>>();
    assert!(
        picture_issues.is_empty(),
        "supported XLS pictures have no loss: {picture_issues:?}"
    );
    let mut document = converted.document;
    assert_eq!(target_xls_pictures(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document.save(&mut bytes).expect("save XLSX pictures");
    let saved = bytes.into_inner();
    assert_package_entry_contains(&saved, "xl/drawings/drawing1.xml", "xdr:pic");
    let mut reopened = SpreadsheetDocument::new(Cursor::new(saved)).expect("reopen XLSX pictures");
    assert_eq!(target_xls_pictures(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save XLSX pictures a second time");
    let mut second = SpreadsheetDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen XLSX pictures a second time");
    assert_eq!(target_xls_pictures(&mut second), expected);
}

fn source_xls_pictures(file: &XlsFile) -> Vec<XlsPictureSnapshot> {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS picture relationships");
    view.pictures()
        .expect("join XLS picture relationships")
        .into_iter()
        .map(|picture| {
            let sheet_index = view
                .sheets()
                .iter()
                .position(|sheet| sheet.id() == picture.sheet().id())
                .expect("picture sheet belongs to workbook view");
            let OfficeArtClientAnchor::Words18 { flags, coordinates } = picture.anchor() else {
                panic!("fixture picture uses the BIFF client anchor")
            };
            let XlsPictureImageLink::Resolved(image) = picture.image() else {
                panic!("fixture picture resolves to a typed embedded BLIP")
            };
            let content_type = match image.format {
                OfficeArtImageFormat::Emf => "image/x-emf",
                OfficeArtImageFormat::Wmf => "image/x-wmf",
                OfficeArtImageFormat::Jpeg => "image/jpeg",
                OfficeArtImageFormat::Png => "image/png",
                OfficeArtImageFormat::Tiff => "image/tiff",
                value => panic!("unsupported fixture picture format {value:?}"),
            };
            let crop = picture.crop();
            XlsPictureSnapshot {
                sheet_index,
                shape_id: picture.shape().shape_id,
                content_type: content_type.into(),
                data: image.data.to_vec(),
                edit_as: match flags {
                    0 => xdr::EditAsValues::TwoCell,
                    2 => xdr::EditAsValues::OneCell,
                    3 => xdr::EditAsValues::Absolute,
                    value => panic!("unsupported fixture anchor behavior {value}"),
                },
                marker: source_xls_picture_marker(coordinates),
                crop: [crop.left(), crop.top(), crop.right(), crop.bottom()]
                    .map(fixed_16_16_to_percentage_for_test),
                horizontal_flip: picture
                    .shape()
                    .flags
                    .contains(OfficeArtShapeFlags::FLIP_HORIZONTAL),
                vertical_flip: picture
                    .shape()
                    .flags
                    .contains(OfficeArtShapeFlags::FLIP_VERTICAL),
            }
        })
        .collect()
}

fn source_xls_picture_marker(coordinates: [u16; 8]) -> [i64; 8] {
    [
        i64::from(coordinates[0]),
        i64::from(coordinates[1]) * 64 * 9_525 / 1_024,
        i64::from(coordinates[2]),
        i64::from(coordinates[3]) * 15 * 12_700 / 256,
        i64::from(coordinates[4]),
        i64::from(coordinates[5]) * 64 * 9_525 / 1_024,
        i64::from(coordinates[6]),
        i64::from(coordinates[7]) * 15 * 12_700 / 256,
    ]
}

fn target_xls_pictures(document: &mut SpreadsheetDocument) -> Vec<XlsPictureSnapshot> {
    let workbook = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let worksheets = workbook.worksheet_parts(document).collect::<Vec<_>>();
    let mut result = Vec::new();
    for (sheet_index, worksheet) in worksheets.into_iter().enumerate() {
        let Some(drawings) = worksheet.drawings_part(document) else {
            continue;
        };
        let anchors = {
            let root = drawings
                .root_element(document)
                .expect("parse converted worksheet drawing");
            root.worksheet_drawing_choice.clone()
        };
        let media = drawings
            .related_parts_of_type::<_, ImagePart>(document)
            .map(|related| {
                (
                    related.relationship_id().to_owned(),
                    (
                        related
                            .part()
                            .content_type(document)
                            .expect("converted XLS picture has a content type")
                            .to_owned(),
                        related
                            .part()
                            .data_to_vec(document)
                            .expect("converted XLS picture has payload data"),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for choice in anchors {
            let xdr::WorksheetDrawingChoice::TwoCellAnchor(anchor) = choice else {
                continue;
            };
            let Some(xdr::TwoCellAnchorChoice::Picture(picture)) =
                anchor.two_cell_anchor_choice.as_ref()
            else {
                continue;
            };
            let relationship_id = picture
                .blip_fill
                .as_deref()
                .and_then(|fill| fill.blip.as_deref())
                .and_then(|blip| blip.embed.as_deref())
                .expect("converted XLS picture embeds an image relationship");
            let (content_type, data) = media
                .get(relationship_id)
                .expect("XLS picture relationship resolves to an ImagePart");
            let properties = picture.shape_properties.as_ref();
            let transform = properties.transform2_d.as_deref();
            let crop = picture
                .blip_fill
                .as_deref()
                .and_then(|fill| fill.source_rectangle.as_ref())
                .map_or([0; 4], |rectangle| {
                    [
                        target_percentage_for_test(rectangle.left),
                        target_percentage_for_test(rectangle.top),
                        target_percentage_for_test(rectangle.right),
                        target_percentage_for_test(rectangle.bottom),
                    ]
                });
            result.push(XlsPictureSnapshot {
                sheet_index,
                shape_id: picture
                    .non_visual_picture_properties
                    .non_visual_drawing_properties
                    .id,
                content_type: content_type.clone(),
                data: data.clone(),
                edit_as: anchor.edit_as.unwrap_or_default(),
                marker: [
                    i64::from(anchor.from_marker.column_id),
                    coordinate_emu(anchor.from_marker.column_offset),
                    i64::from(anchor.from_marker.row_id),
                    coordinate_emu(anchor.from_marker.row_offset),
                    i64::from(anchor.to_marker.column_id),
                    coordinate_emu(anchor.to_marker.column_offset),
                    i64::from(anchor.to_marker.row_id),
                    coordinate_emu(anchor.to_marker.row_offset),
                ],
                crop,
                horizontal_flip: transform
                    .and_then(|value| value.horizontal_flip)
                    .is_some_and(bool::from),
                vertical_flip: transform
                    .and_then(|value| value.vertical_flip)
                    .is_some_and(bool::from),
            });
        }
    }
    result
}

#[derive(Debug, PartialEq)]
struct ExpectedHyperlink {
    reference: String,
    display: Option<String>,
    location: Option<String>,
    target: Option<String>,
}

#[derive(Debug, PartialEq)]
struct ExpectedXlsNumberStyle {
    reference: String,
    style_index: u32,
    number_format_id: u32,
    custom_format_code: Option<String>,
    font_theme: Option<u32>,
    font_scheme: Option<x::FontSchemeValues>,
}

fn source_xls_number_styles(file: &XlsFile) -> Vec<ExpectedXlsNumberStyle> {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS formatting relationships");
    let index = view.sheets()[0]
        .sparse_cell_index()
        .expect("build strict XLS formatting cell index");
    [(1, 1, "B2"), (2, 1, "B3")]
        .into_iter()
        .map(|(row, column, reference)| {
            let cell = index
                .cell(row, column)
                .expect("lookup XLS formatting cell")
                .expect("XLS formatting cell exists");
            let style = view
                .resolve_cell_format_ref(cell)
                .expect("resolve XLS formatting cell XF");
            let (number_format_id, custom_format_code) = match style.number_format {
                XlsNumberFormatRef::BuiltIn(id) => (u32::from(id), None),
                XlsNumberFormatRef::Custom(format) => (
                    u32::from(format.format_index),
                    style.custom_number_format_code,
                ),
                XlsNumberFormatRef::Compatibility(id) => {
                    panic!("strict formatting fixture has unresolved format id {id}")
                }
            };
            let extension = view.xf_extension(cell.cell().format_index);
            let font_theme = extension.and_then(|extension| {
                extension
                    .properties
                    .iter()
                    .rev()
                    .find_map(|property| match property.data {
                        ExtPropertyData::FullColor {
                            property_type: 0x000d,
                            color,
                        } if color.color_type == 3 => Some(color.color_value),
                        _ => None,
                    })
            });
            let font_scheme = extension.and_then(|extension| {
                extension
                    .properties
                    .iter()
                    .rev()
                    .find_map(|property| match property.data {
                        ExtPropertyData::FontScheme(ExtFontScheme::Byte(0))
                        | ExtPropertyData::FontScheme(ExtFontScheme::Word(0)) => {
                            Some(x::FontSchemeValues::None)
                        }
                        ExtPropertyData::FontScheme(ExtFontScheme::Byte(1))
                        | ExtPropertyData::FontScheme(ExtFontScheme::Word(1)) => {
                            Some(x::FontSchemeValues::Major)
                        }
                        ExtPropertyData::FontScheme(ExtFontScheme::Byte(2))
                        | ExtPropertyData::FontScheme(ExtFontScheme::Word(2)) => {
                            Some(x::FontSchemeValues::Minor)
                        }
                        _ => None,
                    })
            });
            ExpectedXlsNumberStyle {
                reference: reference.into(),
                style_index: u32::from(cell.cell().format_index),
                number_format_id,
                custom_format_code,
                font_theme,
                font_scheme,
            }
        })
        .collect()
}

fn target_xls_number_styles(document: &mut SpreadsheetDocument) -> Vec<ExpectedXlsNumberStyle> {
    let workbook_part = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let styles_part = workbook_part
        .workbook_styles_part(document)
        .expect("converted XLSX has a styles part");
    let (formats, fonts, custom_formats) = {
        let stylesheet = styles_part
            .root_element(document)
            .expect("parse converted XLSX styles");
        let formats = stylesheet
            .cell_formats
            .as_ref()
            .expect("converted XLSX has cell XFs")
            .xml_children
            .iter()
            .map(|choice| match choice {
                x::CellFormatsChoice::CellFormat(format) => {
                    (format.number_format_id, format.font_id)
                }
                x::CellFormatsChoice::AlternateContent(_) => {
                    panic!("converter does not emit alternate-content cell XFs")
                }
            })
            .collect::<Vec<_>>();
        let fonts = stylesheet
            .fonts
            .as_ref()
            .expect("converted XLSX has fonts")
            .xml_children
            .iter()
            .map(|choice| match choice {
                x::FontsChoice::Font(font) => {
                    let theme = font
                        .font_choice
                        .iter()
                        .rev()
                        .find_map(|choice| match choice {
                            x::FontChoice::Color(color) => color.theme,
                            _ => None,
                        });
                    let scheme = font
                        .font_choice
                        .iter()
                        .rev()
                        .find_map(|choice| match choice {
                            x::FontChoice::FontScheme(scheme) => Some(scheme.val),
                            _ => None,
                        });
                    (theme, scheme)
                }
                x::FontsChoice::AlternateContent(_) => {
                    panic!("converter does not emit alternate-content fonts")
                }
            })
            .collect::<Vec<_>>();
        let custom_formats = stylesheet
            .numbering_formats
            .as_ref()
            .map(|formats| {
                formats
                    .numbering_format
                    .iter()
                    .map(|format| (format.number_format_id, format.format_code.clone()))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        (formats, fonts, custom_formats)
    };
    let worksheet_part = workbook_part
        .worksheet_parts(document)
        .next()
        .expect("converted XLSX has its first worksheet");
    let worksheet = worksheet_part
        .root_element(document)
        .expect("parse converted XLSX worksheet");
    worksheet
        .sheet_data
        .row
        .iter()
        .flat_map(|row| &row.cell)
        .filter_map(|cell| {
            let reference = cell.cell_reference.as_deref()?;
            matches!(reference, "B2" | "B3").then(|| {
                let style_index = cell.style_index.expect("formatted target cell has a style");
                let (number_format_id, font_id) = formats
                    .get(usize::try_from(style_index).expect("style index fits usize"))
                    .copied()
                    .expect("target cell style has a number format");
                let number_format_id =
                    number_format_id.expect("target cell style has a number format");
                let (font_theme, font_scheme) = fonts
                    .get(
                        usize::try_from(font_id.expect("target cell style has a font id"))
                            .expect("font id fits usize"),
                    )
                    .copied()
                    .expect("target cell style font exists");
                ExpectedXlsNumberStyle {
                    reference: reference.into(),
                    style_index,
                    number_format_id,
                    custom_format_code: custom_formats.get(&number_format_id).cloned(),
                    font_theme,
                    font_scheme,
                }
            })
        })
        .collect()
}

fn target_xls_comments(document: &mut SpreadsheetDocument) -> Vec<(String, String, String)> {
    let workbook_part = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let worksheet_part = workbook_part
        .worksheet_parts(document)
        .next()
        .expect("converted XLSX has its first worksheet");
    let comments_part = worksheet_part
        .worksheet_comments_part(document)
        .expect("converted XLSX worksheet has comments");
    let comments = comments_part
        .root_element(document)
        .expect("parse converted XLSX comments");
    comments
        .comment_list
        .comment
        .iter()
        .map(|comment| {
            let author = comments
                .authors
                .author
                .get(usize::try_from(comment.author_id).expect("author id fits usize"))
                .and_then(|author| author.0.xml_content.clone())
                .expect("comment author id resolves");
            let text = comment
                .comment_text
                .text
                .as_ref()
                .and_then(|text| text.0.xml_content.clone())
                .expect("converted comment has plain text");
            (comment.reference.clone(), author, text)
        })
        .collect()
}

fn target_xls_formula(
    document: &mut SpreadsheetDocument,
    reference: &str,
) -> (String, Option<CellValues>, Option<String>) {
    let workbook_part = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let worksheet_part = workbook_part
        .worksheet_parts(document)
        .next()
        .expect("converted XLSX has its first worksheet");
    let worksheet = worksheet_part
        .root_element(document)
        .expect("parse converted XLSX worksheet");
    let cell = worksheet
        .sheet_data
        .row
        .iter()
        .flat_map(|row| &row.cell)
        .find(|cell| cell.cell_reference.as_deref() == Some(reference))
        .expect("converted formula cell exists");
    (
        cell.cell_formula
            .as_ref()
            .and_then(|formula| formula.xml_content.clone())
            .expect("converted formula cell has typed formula content"),
        cell.data_type,
        cell.cell_value
            .as_ref()
            .and_then(|value| value.0.xml_content.clone()),
    )
}

fn source_xls_hyperlinks(file: &XlsFile) -> Vec<Vec<ExpectedHyperlink>> {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS hyperlinks");
    view.sheets()
        .iter()
        .copied()
        .map(|sheet| {
            sheet
                .hyperlinks()
                .expect("resolve strict XLS sheet hyperlinks")
                .into_iter()
                .map(|link| {
                    let range = link.value();
                    let target = match link.target {
                        Some(
                            XlsHyperlinkTarget::String(value) | XlsHyperlinkTarget::Url(value),
                        ) => Some(value),
                        Some(XlsHyperlinkTarget::File {
                            long_path: Some(value),
                            ..
                        }) => Some(value),
                        Some(
                            XlsHyperlinkTarget::File {
                                long_path: None, ..
                            }
                            | XlsHyperlinkTarget::Standard { .. },
                        ) => None,
                        None => None,
                    };
                    ExpectedHyperlink {
                        reference: format!(
                            "{}:{}",
                            xls_cell_reference(range.first_row, range.first_column),
                            xls_cell_reference(range.last_row, range.last_column)
                        ),
                        display: link.display_name,
                        location: link.location,
                        target,
                    }
                })
                .collect()
        })
        .collect()
}

fn target_xls_hyperlinks(document: &mut SpreadsheetDocument) -> Vec<Vec<ExpectedHyperlink>> {
    let workbook = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let parts = workbook.worksheet_parts(document).collect::<Vec<_>>();
    parts
        .into_iter()
        .map(|part| {
            let links = part
                .root_element(document)
                .expect("parse converted XLSX hyperlinks")
                .hyperlinks
                .as_ref()
                .map(|links| links.hyperlink.clone())
                .unwrap_or_default();
            links
                .into_iter()
                .map(|link| ExpectedHyperlink {
                    reference: link.reference,
                    target: link.id.as_deref().map(|id| {
                        part.get_hyperlink_relationship(document, id)
                            .expect("resolve converted XLSX hyperlink relationship")
                            .target()
                            .to_owned()
                    }),
                    display: link.display,
                    location: link.location,
                })
                .collect()
        })
        .collect()
}

fn source_xls_merged_ranges(file: &XlsFile) -> Vec<Vec<String>> {
    let view = file.workbooks[0]
        .relationships()
        .expect("resolve strict XLS merged ranges");
    view.sheets()
        .iter()
        .copied()
        .map(|sheet| {
            sheet
                .merged_cells()
                .map(|range| {
                    format!(
                        "{}:{}",
                        xls_cell_reference(range.first_row, range.first_column),
                        xls_cell_reference(range.last_row, range.last_column)
                    )
                })
                .collect()
        })
        .collect()
}

fn target_xls_merged_ranges(document: &mut SpreadsheetDocument) -> Vec<Vec<String>> {
    let workbook = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let parts = workbook.worksheet_parts(document).collect::<Vec<_>>();
    parts
        .into_iter()
        .map(|part| {
            part.root_element(document)
                .expect("parse converted XLSX merged ranges")
                .merge_cells
                .as_ref()
                .map(|ranges| {
                    ranges
                        .merge_cell
                        .iter()
                        .map(|range| range.reference.clone())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PptTransitionSnapshot {
    effect: &'static str,
    speed: p::TransitionSpeedValues,
    advance_on_click: bool,
    advance_after_time: Option<String>,
    show: bool,
}

#[test]
fn ppt_conversion_preserves_typed_alpha_fade_and_directional_wipe_transitions() {
    for relative in [
        "Apache-POI/test-data/slideshow/49541_symbol_map.ppt",
        "Apache-POI/test-data/slideshow/bug45124.ppt",
    ] {
        let source = PptFile::open_compatible(fixture(relative))
            .expect("compatibly open PPT transition fixture")
            .value;
        let expected = source_ppt_transitions(&source);
        assert!(
            expected.iter().any(Option::is_some),
            "fixture has slide transitions: {relative}"
        );

        let converted = convert_ppt_with_options(
            &source,
            ConversionOptions {
                unsupported: LossPolicy::Report,
            },
        )
        .expect("convert PPT transitions with explicit unrelated degradation reporting");
        let transition_issues = converted
            .report
            .issues()
            .iter()
            .filter(|issue| {
                matches!(
                    issue.code,
                    ConversionCode::SlideTransitionNotMapped
                        | ConversionCode::SlideTransitionFeatureNotMapped
                )
            })
            .collect::<Vec<_>>();
        assert!(
            transition_issues.is_empty(),
            "supported PPT transitions have no loss in {relative}: {transition_issues:?}"
        );
        let mut document = converted.document;
        assert_eq!(
            target_ppt_transitions(&mut document),
            expected,
            "{relative}"
        );

        let mut bytes = Cursor::new(Vec::new());
        document.save(&mut bytes).expect("save PPTX transitions");
        let saved = bytes.into_inner();
        assert_package_entry_contains(&saved, "ppt/slides/slide1.xml", "p:transition");
        let mut reopened =
            PresentationDocument::new(Cursor::new(saved)).expect("reopen PPTX transitions");
        assert_eq!(
            target_ppt_transitions(&mut reopened),
            expected,
            "{relative}"
        );

        let mut second_bytes = Cursor::new(Vec::new());
        reopened
            .save(&mut second_bytes)
            .expect("save PPTX transitions a second time");
        let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
            .expect("reopen PPTX transitions a second time");
        assert_eq!(target_ppt_transitions(&mut second), expected, "{relative}");
    }
}

fn source_ppt_transitions(source: &PptFile) -> Vec<Option<PptTransitionSnapshot>> {
    source
        .live_presentation()
        .expect("resolve PPT transition presentation")
        .slides()
        .expect("resolve PPT transition slides")
        .into_iter()
        .map(|slide| {
            slide
                .transition()
                .expect("resolve unique PPT slide transition")
                .map(|transition| {
                    let value = transition.value;
                    let effect = match (value.effect_type, value.effect_direction) {
                        (23, 0) => "fade",
                        (10, 0) => "wipe-left",
                        (10, 1) => "wipe-up",
                        (10, 2) => "wipe-right",
                        (10, 3) => "wipe-down",
                        pair => panic!("unsupported transition fixture effect {pair:?}"),
                    };
                    PptTransitionSnapshot {
                        effect,
                        speed: match value.speed {
                            0 => p::TransitionSpeedValues::Slow,
                            1 => p::TransitionSpeedValues::Medium,
                            2 => p::TransitionSpeedValues::Fast,
                            speed => panic!("unsupported transition fixture speed {speed}"),
                        },
                        advance_on_click: value.flags & 1 != 0,
                        advance_after_time: (value.flags & (1 << 10) != 0)
                            .then(|| value.slide_time.to_string()),
                        show: value.flags & (1 << 2) == 0,
                    }
                })
        })
        .collect()
}

fn target_ppt_transitions(
    document: &mut PresentationDocument,
) -> Vec<Option<PptTransitionSnapshot>> {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    let slides = presentation.slide_parts(document).collect::<Vec<_>>();
    slides
        .into_iter()
        .map(|part| {
            let slide = part
                .root_element(document)
                .expect("parse converted PPTX transition slide");
            slide.transition.as_deref().map(|transition| {
                let effect = match transition.transition_choice.as_ref() {
                    Some(p::TransitionChoice::FadeTransition(_)) => "fade",
                    Some(p::TransitionChoice::WipeTransition(wipe)) => match wipe.direction {
                        Some(p::TransitionSlideDirectionValues::Left) => "wipe-left",
                        Some(p::TransitionSlideDirectionValues::Up) => "wipe-up",
                        Some(p::TransitionSlideDirectionValues::Right) => "wipe-right",
                        Some(p::TransitionSlideDirectionValues::Down) => "wipe-down",
                        value => panic!("converted wipe has exact direction: {value:?}"),
                    },
                    value => panic!("unexpected converted transition effect {value:?}"),
                };
                PptTransitionSnapshot {
                    effect,
                    speed: transition.speed.expect("transition retains speed"),
                    advance_on_click: transition.advance_on_click.is_some_and(bool::from),
                    advance_after_time: transition.advance_after_time.clone(),
                    show: slide.show.is_none_or(bool::from),
                }
            })
        })
        .collect()
}

#[test]
fn ppt_conversion_preserves_master_legacy_palette_as_typed_theme() {
    let source = PptFile::open(fixture(
        "Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt",
    ))
    .expect("strictly open PPT master-theme fixture");
    let expected = source_ppt_master_theme_palettes(&source);
    assert!(!expected.is_empty(), "fixture has a main-master palette");

    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert PPT master theme with explicit unrelated losses");
    let mut document = converted.document;
    assert_eq!(target_ppt_master_theme_palettes(&mut document), expected);

    let mut bytes = Cursor::new(Vec::new());
    document.save(&mut bytes).expect("save PPTX master theme");
    let saved = bytes.into_inner();
    assert_package_entry_family_contains(&saved, "ppt/theme/", ".xml", "a:clrScheme");
    let mut reopened =
        PresentationDocument::new(Cursor::new(saved)).expect("reopen PPTX master theme");
    assert_eq!(target_ppt_master_theme_palettes(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save PPTX master theme a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen PPTX master theme a second time");
    assert_eq!(target_ppt_master_theme_palettes(&mut second), expected);
}

fn source_ppt_master_theme_palettes(source: &PptFile) -> Vec<[String; 12]> {
    source
        .live_presentation()
        .expect("resolve PPT theme presentation")
        .master_slides
        .into_iter()
        .filter(|master| master.role == olecfsdk::ppt::PptLivePersistObjectRole::MainMasterSlide)
        .map(|master| {
            let colors = master
                .color_scheme()
                .expect("resolve unique active master palette")
                .expect("main master has an active palette")
                .value
                .colors;
            let [
                background,
                text_and_lines,
                shadow,
                title_text,
                fills,
                accent,
                hyperlink,
                followed,
            ] = colors;
            [
                ppt_legacy_rgb(text_and_lines),
                ppt_legacy_rgb(background),
                ppt_legacy_rgb(title_text),
                ppt_legacy_rgb(shadow),
                ppt_legacy_rgb(fills),
                ppt_legacy_rgb(accent),
                ppt_legacy_rgb(hyperlink),
                ppt_legacy_rgb(followed),
                ppt_legacy_rgb(fills),
                ppt_legacy_rgb(accent),
                ppt_legacy_rgb(hyperlink),
                ppt_legacy_rgb(followed),
            ]
        })
        .collect()
}

fn ppt_legacy_rgb(value: u32) -> String {
    let bytes = value.to_le_bytes();
    format!("{:02X}{:02X}{:02X}", bytes[0], bytes[1], bytes[2])
}

fn target_ppt_master_theme_palettes(document: &mut PresentationDocument) -> Vec<[String; 12]> {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    presentation
        .slide_master_parts(document)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|master| {
            let theme_part = master
                .theme_part(document)
                .expect("converted slide master owns a typed theme part");
            let theme = theme_part
                .root_element(document)
                .expect("parse converted typed master theme");
            let colors = &theme.theme_elements.color_scheme;
            [
                dark1_theme_rgb(&colors.dark1_color),
                light1_theme_rgb(&colors.light1_color),
                dark2_theme_rgb(&colors.dark2_color),
                light2_theme_rgb(&colors.light2_color),
                accent1_theme_rgb(&colors.accent1_color),
                accent2_theme_rgb(&colors.accent2_color),
                accent3_theme_rgb(&colors.accent3_color),
                accent4_theme_rgb(&colors.accent4_color),
                accent5_theme_rgb(&colors.accent5_color),
                accent6_theme_rgb(&colors.accent6_color),
                hyperlink_theme_rgb(&colors.hyperlink),
                followed_hyperlink_theme_rgb(&colors.followed_hyperlink_color),
            ]
        })
        .collect()
}

macro_rules! typed_theme_rgb {
    ($name:ident, $container:ty, $field:ident, $choice:path) => {
        fn $name(value: &$container) -> String {
            let Some($choice(color)) = value.$field.as_ref() else {
                panic!("converted theme color is typed sRGB")
            };
            color.val.clone()
        }
    };
}

typed_theme_rgb!(
    dark1_theme_rgb,
    a::Dark1Color,
    dark1_color_choice,
    a::Dark1ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    light1_theme_rgb,
    a::Light1Color,
    light1_color_choice,
    a::Light1ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    dark2_theme_rgb,
    a::Dark2Color,
    dark2_color_choice,
    a::Dark2ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    light2_theme_rgb,
    a::Light2Color,
    light2_color_choice,
    a::Light2ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    accent1_theme_rgb,
    a::Accent1Color,
    accent1_color_choice,
    a::Accent1ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    accent2_theme_rgb,
    a::Accent2Color,
    accent2_color_choice,
    a::Accent2ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    accent3_theme_rgb,
    a::Accent3Color,
    accent3_color_choice,
    a::Accent3ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    accent4_theme_rgb,
    a::Accent4Color,
    accent4_color_choice,
    a::Accent4ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    accent5_theme_rgb,
    a::Accent5Color,
    accent5_color_choice,
    a::Accent5ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    accent6_theme_rgb,
    a::Accent6Color,
    accent6_color_choice,
    a::Accent6ColorChoice::RgbColorModelHex
);
typed_theme_rgb!(
    hyperlink_theme_rgb,
    a::Hyperlink,
    hyperlink_choice,
    a::HyperlinkChoice::RgbColorModelHex
);
typed_theme_rgb!(
    followed_hyperlink_theme_rgb,
    a::FollowedHyperlinkColor,
    followed_hyperlink_color_choice,
    a::FollowedHyperlinkColorChoice::RgbColorModelHex
);

#[test]
fn ppt_conversion_preserves_slide_shape_order_ids_and_text() {
    let source = PptFile::open(fixture(
        "Apache-POI/test-data/slideshow/basic_test_ppt_file.ppt",
    ))
    .expect("strictly open PPT text conversion fixture");
    let expected = source_ppt_shapes(&source);
    let expected_notes = source_ppt_notes_graph(&source);
    let expected_geometry = source_ppt_geometry(&source);
    assert!(expected.iter().flatten().any(|(_, text)| !text.is_empty()));
    let strict_error =
        convert_ppt(&source).expect_err("strict PPT conversion rejects known feature loss");
    assert!(
        matches!(
            strict_error,
            Error::Unsupported {
                code: ConversionCode::TextFormattingNotMapped,
                location: SourceLocation::PptNotesMasterShape { shape_id: 3074 },
            }
        ),
        "unexpected strict PPT conversion error: {strict_error:?}"
    );

    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert PPT with explicit unsupported-feature reporting");
    assert!(
        converted
            .report
            .issues()
            .iter()
            .all(|issue| issue.code != ConversionCode::ShapeGeometryNotMapped)
    );
    assert!(
        converted
            .report
            .issues()
            .iter()
            .all(|issue| { issue.code != ConversionCode::MasterRelationshipNotMapped })
    );

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save typed PPTX conversion");
    let saved = bytes.into_inner();
    assert_xml_namespaces(
        &saved,
        "ppt/presentation.xml",
        &[
            "xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"",
            "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"",
        ],
    );
    assert_xml_namespaces(
        &saved,
        "ppt/notesMasters/notesMaster1.xml",
        &[
            "xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"",
            "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"",
        ],
    );
    assert_xml_namespaces(
        &saved,
        "ppt/notesSlides/notesSlide1.xml",
        &[
            "xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"",
            "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"",
            "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"",
        ],
    );
    let mut reopened =
        PresentationDocument::new(Cursor::new(saved)).expect("reopen typed PPTX conversion");
    assert_eq!(target_ppt_shapes(&mut reopened), expected);
    assert_eq!(target_ppt_geometry(&mut reopened), expected_geometry);
    assert_eq!(target_ppt_notes_graph(&mut reopened), expected_notes);
    let expected_master_graph = PptMasterGraph {
        master_layout_types: vec![vec![
            p::SlideLayoutValues::Title,
            p::SlideLayoutValues::Text,
        ]],
        slide_layout_types: vec![p::SlideLayoutValues::Title, p::SlideLayoutValues::Text],
    };
    assert_eq!(
        target_ppt_master_graph(&mut reopened),
        expected_master_graph
    );

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save PPTX conversion a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen PPTX conversion a second time");
    assert_eq!(target_ppt_shapes(&mut second), expected);
    assert_eq!(target_ppt_geometry(&mut second), expected_geometry);
    assert_eq!(target_ppt_notes_graph(&mut second), expected_notes);
    assert_eq!(target_ppt_master_graph(&mut second), expected_master_graph);
}

#[test]
fn ppt_conversion_maps_title_master_to_its_main_master_layout() {
    let source = PptFile::open(fixture("Apache-POI/test-data/slideshow/slide_master.ppt"))
        .expect("strictly open PPT title-master fixture");
    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert PPT title master with explicit unrelated losses");
    assert!(
        converted
            .report
            .issues()
            .iter()
            .all(|issue| { issue.code != ConversionCode::MasterRelationshipNotMapped })
    );
    let expected = PptMasterGraph {
        master_layout_types: vec![
            vec![p::SlideLayoutValues::Text],
            vec![p::SlideLayoutValues::Title, p::SlideLayoutValues::Text],
        ],
        slide_layout_types: vec![
            p::SlideLayoutValues::Text,
            p::SlideLayoutValues::Text,
            p::SlideLayoutValues::Title,
        ],
    };

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save title-master PPTX conversion");
    let mut reopened = PresentationDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen title-master PPTX conversion");
    assert_eq!(target_ppt_master_graph(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save title-master PPTX conversion a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen title-master PPTX conversion a second time");
    assert_eq!(target_ppt_master_graph(&mut second), expected);
}

#[test]
fn ppt_conversion_preserves_png_picture_payload_and_relationship() {
    let source = PptFile::open(fixture("Apache-POI/test-data/slideshow/ppt_with_png.ppt"))
        .expect("strictly open PPT PNG fixture");
    let source_images = source
        .live_image_store()
        .expect("resolve borrowed PPT image store");
    let expected = source_images
        .entries
        .iter()
        .find_map(|link| match link {
            PptLiveImageLink::Resolved(image) if image.blip_identifier == 2 => {
                Some(image.image.data.to_vec())
            }
            _ => None,
        })
        .expect("fixture BLIP 2 resolves to its Pictures-stream PNG");
    let source_shape = source
        .live_presentation()
        .expect("resolve strict PPT PNG presentation")
        .slides()
        .expect("resolve strict PPT PNG slides")[0]
        .shapes()
        .expect("resolve strict PPT PNG shapes")
        .into_iter()
        .find(|shape| shape.shape_id() == 2053)
        .expect("fixture contains picture shape 2053");
    let anchor = source_shape
        .anchor()
        .expect("resolve PPT PNG anchor")
        .expect("PPT PNG shape has an anchor");
    let expected_geometry = (
        ppt_master_to_emu(anchor.left),
        ppt_master_to_emu(anchor.top),
        ppt_master_to_emu(anchor.right - anchor.left),
        ppt_master_to_emu(anchor.bottom - anchor.top),
    );

    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert PPT PNG with explicit unrelated losses");
    assert!(converted.report.issues().iter().all(|issue| {
        issue.code != ConversionCode::PictureNotMapped
            || !matches!(
                issue.source,
                SourceLocation::PptShape {
                    slide_index: 0,
                    shape_id: 2053,
                }
            )
    }));

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save PPTX PNG conversion");
    let mut reopened = PresentationDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen PPTX PNG conversion");
    assert_eq!(
        target_ppt_picture(&mut reopened),
        (2053, expected.clone(), expected_geometry)
    );

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save PPTX PNG conversion a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen PPTX PNG conversion a second time");
    assert_eq!(
        target_ppt_picture(&mut second),
        (2053, expected, expected_geometry)
    );
}

#[test]
fn ppt_conversion_maps_supported_picture_formats_and_reports_pict() {
    let source = PptFile::open_compatible(fixture("Apache-POI/test-data/slideshow/pictures.ppt"))
        .expect("compatibly open multi-format PPT picture fixture")
        .value;
    let live = source
        .live_presentation()
        .expect("resolve multi-format PPT presentation");
    let images = source
        .live_image_store()
        .expect("resolve multi-format PPT image store");
    let slides = live.slides().expect("resolve multi-format PPT slides");
    let mut expected = Vec::new();
    let mut unsupported_pict = Vec::new();
    for (slide_index, slide) in slides.into_iter().enumerate() {
        for shape in slide
            .shapes()
            .expect("resolve multi-format PPT picture shapes")
        {
            let Some(blip_identifier) = shape
                .primary_blip_identifier()
                .expect("resolve multi-format PPT shape BLIP")
            else {
                continue;
            };
            let image = images
                .entries
                .iter()
                .find_map(|link| match link {
                    PptLiveImageLink::Resolved(image)
                        if image.blip_identifier == blip_identifier =>
                    {
                        Some(image)
                    }
                    _ => None,
                })
                .expect("multi-format PPT shape BLIP resolves");
            if image.image.format == OfficeArtImageFormat::Pict {
                unsupported_pict.push((slide_index, shape.shape_id()));
            } else {
                expected.push((slide_index, shape.shape_id(), image.image.data.to_vec()));
            }
        }
    }
    assert_eq!(expected.len(), 4);
    assert_eq!(unsupported_pict.len(), 1);

    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert multi-format PPT pictures with explicit PICT loss");
    assert!(unsupported_pict.iter().all(|&(slide_index, shape_id)| {
        converted.report.issues().iter().any(|issue| {
            issue.code == ConversionCode::PictureNotMapped
                && issue.source
                    == SourceLocation::PptShape {
                        slide_index,
                        shape_id,
                    }
        })
    }));
    assert!(converted.report.issues().iter().any(|issue| {
        issue.code == ConversionCode::ShapeIdentityNotMapped
            && matches!(issue.source, SourceLocation::PptShape { shape_id: 0, .. })
    }));

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save multi-format PPTX conversion");
    let mut reopened = PresentationDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen multi-format PPTX conversion");
    assert_ppt_picture_payloads(target_ppt_pictures(&mut reopened), &expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save multi-format PPTX conversion a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen multi-format PPTX conversion a second time");
    assert_ppt_picture_payloads(target_ppt_pictures(&mut second), &expected);
}

#[test]
fn ppt_conversion_maps_native_table_grid_geometry_and_text() {
    let source = PptFile::open(fixture("Apache-POI/test-data/slideshow/table_test.ppt"))
        .expect("strictly open PPT table fixture");
    let expected = source_ppt_table(&source);
    assert_eq!(
        (expected.row_heights.len(), expected.column_widths.len()),
        (6, 3)
    );
    assert_eq!(expected.cells.iter().flatten().count(), 18);

    let converted = convert_ppt_with_options(
        &source,
        ConversionOptions {
            unsupported: LossPolicy::Report,
        },
    )
    .expect("convert PPT table with explicit formatting loss");
    assert!(converted.report.issues().iter().any(|issue| {
        issue.code == ConversionCode::TableFormattingNotMapped
            && issue.source
                == SourceLocation::PptShape {
                    slide_index: 0,
                    shape_id: 2050,
                }
    }));

    let mut bytes = Cursor::new(Vec::new());
    converted
        .document
        .save(&mut bytes)
        .expect("save PPTX table conversion");
    let mut reopened = PresentationDocument::new(Cursor::new(bytes.into_inner()))
        .expect("reopen PPTX table conversion");
    assert_eq!(target_ppt_table(&mut reopened), expected);

    let mut second_bytes = Cursor::new(Vec::new());
    reopened
        .save(&mut second_bytes)
        .expect("save PPTX table conversion a second time");
    let mut second = PresentationDocument::new(Cursor::new(second_bytes.into_inner()))
        .expect("reopen PPTX table conversion a second time");
    assert_eq!(target_ppt_table(&mut second), expected);
}

#[derive(Debug, PartialEq)]
struct PptTableSnapshot {
    shape_id: u32,
    geometry: (i64, i64, i64, i64),
    column_widths: Vec<i64>,
    row_heights: Vec<i64>,
    cells: Vec<Vec<String>>,
}

fn source_ppt_table(file: &PptFile) -> PptTableSnapshot {
    let live = file
        .live_presentation()
        .expect("resolve strict PPT table presentation");
    let slide = live.slides().expect("resolve strict PPT table slide")[0];
    let table_shape = slide
        .shapes()
        .expect("resolve strict PPT table shapes")
        .into_iter()
        .find(|shape| shape.is_table())
        .expect("strict PPT table marker exists");
    let table = table_shape
        .table()
        .expect("project strict PPT table")
        .expect("table marker projects to a native table");
    let mut cells = vec![vec![String::new(); table.columns.len()]; table.rows.len()];
    for cell in &table.cells {
        assert_eq!((cell.row_span, cell.column_span), (1, 1));
        cells[cell.row][cell.column] = source_ppt_shape_values(vec![cell.shape])
            .pop()
            .expect("PPT table cell shape has an identity")
            .1;
    }
    PptTableSnapshot {
        shape_id: table.shape.shape_id(),
        geometry: (
            ppt_master_to_emu(table.anchor.left),
            ppt_master_to_emu(table.anchor.top),
            ppt_master_to_emu(table.anchor.right - table.anchor.left),
            ppt_master_to_emu(table.anchor.bottom - table.anchor.top),
        ),
        column_widths: table
            .columns
            .iter()
            .map(|column| ppt_master_to_emu(column.end - column.start))
            .collect(),
        row_heights: table
            .rows
            .iter()
            .map(|row| ppt_master_to_emu(row.end - row.start))
            .collect(),
        cells,
    }
}

fn target_ppt_table(document: &mut PresentationDocument) -> PptTableSnapshot {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    let slide = presentation
        .slide_parts(document)
        .next()
        .expect("converted PPTX table has a slide");
    let root = slide
        .root_element(document)
        .expect("parse converted PPTX table slide");
    let frame = root
        .common_slide_data
        .shape_tree
        .shape_tree_choice
        .iter()
        .find_map(|choice| match choice {
            ShapeTreeChoice::GraphicFrame(frame) => Some(frame.as_ref()),
            _ => None,
        })
        .expect("converted PPTX contains a graphic frame");
    let table = frame
        .graphic
        .graphic_data
        .graphic_data_choice
        .iter()
        .find_map(|choice| match choice {
            a::GraphicDataChoice::Table(table) => Some(table.as_ref()),
            _ => None,
        })
        .expect("converted PPTX graphic frame contains a typed table");
    let offset = frame
        .transform
        .offset
        .as_ref()
        .expect("converted PPTX table has an offset");
    let extents = frame
        .transform
        .extents
        .as_ref()
        .expect("converted PPTX table has extents");
    PptTableSnapshot {
        shape_id: frame
            .non_visual_graphic_frame_properties
            .non_visual_drawing_properties
            .id,
        geometry: (
            coordinate_emu(offset.x),
            coordinate_emu(offset.y),
            coordinate_emu(extents.cx),
            coordinate_emu(extents.cy),
        ),
        column_widths: table
            .table_grid
            .grid_column
            .iter()
            .map(|column| coordinate_emu(column.width))
            .collect(),
        row_heights: table
            .table_row
            .iter()
            .map(|row| coordinate_emu(row.height))
            .collect(),
        cells: table
            .table_row
            .iter()
            .map(|row| {
                row.table_cell
                    .iter()
                    .map(|cell| {
                        cell.text_body
                            .as_deref()
                            .map(drawing_text_body_value)
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect(),
    }
}

fn assert_ppt_picture_payloads(
    actual: Vec<(usize, u32, Vec<u8>)>,
    expected: &[(usize, u32, Vec<u8>)],
) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            (actual.0, actual.2.len()),
            (expected.0, expected.2.len()),
            "PPT picture {index} slide/length"
        );
        if expected.1 == 0 {
            assert_ne!(actual.1, 0, "normalized PPT picture {index} ID");
        } else {
            assert_eq!(actual.1, expected.1, "PPT picture {index} source ID");
        }
        assert!(
            actual.2 == expected.2,
            "PPT picture {index} payload differs"
        );
    }
}

fn target_ppt_picture(document: &mut PresentationDocument) -> (u32, Vec<u8>, (i64, i64, i64, i64)) {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    let slide = presentation
        .slide_parts(document)
        .next()
        .expect("converted PPTX has a first slide");
    let (shape_id, relationship_id, geometry) = {
        let root = slide
            .root_element(document)
            .expect("parse converted PPTX picture slide");
        root.common_slide_data
            .shape_tree
            .shape_tree_choice
            .iter()
            .find_map(|choice| match choice {
                ShapeTreeChoice::Picture(picture) => {
                    let transform = picture
                        .shape_properties
                        .transform2_d
                        .as_ref()
                        .expect("converted PPTX picture has a transform");
                    let offset = transform
                        .offset
                        .as_ref()
                        .expect("converted PPTX picture has an offset");
                    let extents = transform
                        .extents
                        .as_ref()
                        .expect("converted PPTX picture has extents");
                    Some((
                        picture
                            .non_visual_picture_properties
                            .non_visual_drawing_properties
                            .id,
                        picture
                            .blip_fill
                            .as_deref()
                            .and_then(|fill| fill.blip.as_deref())
                            .and_then(|blip| blip.embed.as_deref())
                            .expect("converted PPTX picture embeds an image relationship")
                            .to_owned(),
                        (
                            coordinate_emu(offset.x),
                            coordinate_emu(offset.y),
                            coordinate_emu(extents.cx),
                            coordinate_emu(extents.cy),
                        ),
                    ))
                }
                _ => None,
            })
            .expect("converted PPTX slide contains a typed picture")
    };
    let payload = slide
        .related_parts_of_type::<_, ImagePart>(document)
        .find(|related| related.relationship_id() == relationship_id)
        .expect("typed PPTX picture relationship resolves to an ImagePart")
        .part()
        .data_to_vec(document)
        .expect("converted PPTX picture has payload data");
    (shape_id, payload, geometry)
}

fn target_ppt_pictures(document: &mut PresentationDocument) -> Vec<(usize, u32, Vec<u8>)> {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    presentation
        .slide_parts(document)
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
        .flat_map(|(slide_index, slide)| {
            let pictures = {
                let root = slide
                    .root_element(document)
                    .expect("parse converted multi-format PPTX slide");
                root.common_slide_data
                    .shape_tree
                    .shape_tree_choice
                    .iter()
                    .filter_map(|choice| match choice {
                        ShapeTreeChoice::Picture(picture) => Some((
                            picture
                                .non_visual_picture_properties
                                .non_visual_drawing_properties
                                .id,
                            picture
                                .blip_fill
                                .as_deref()
                                .and_then(|fill| fill.blip.as_deref())
                                .and_then(|blip| blip.embed.as_deref())
                                .expect("converted picture embeds an image")
                                .to_owned(),
                        )),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            pictures
                .into_iter()
                .map(|(shape_id, relationship_id)| {
                    let payload = slide
                        .related_parts_of_type::<_, ImagePart>(document)
                        .find(|related| related.relationship_id() == relationship_id)
                        .expect("converted picture relationship resolves")
                        .part()
                        .data_to_vec(document)
                        .expect("converted picture payload is readable");
                    (slide_index, shape_id, payload)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[derive(Debug, PartialEq)]
struct PptNotesGraph {
    notes_size: (i64, i64),
    master_shapes: Option<Vec<(u32, String)>>,
    slide_shapes: Vec<Option<Vec<(u32, String)>>>,
}

fn source_ppt_notes_graph(file: &PptFile) -> PptNotesGraph {
    let live = file
        .live_presentation()
        .expect("resolve strict live PPT notes graph");
    let master_shapes = live.notes_master_slide.map(|master| {
        source_ppt_shape_values(master.shapes().expect("resolve notes master shapes"))
    });
    let slide_shapes = live
        .slides()
        .expect("resolve strict PPT notes relationships")
        .into_iter()
        .map(|slide| match slide.notes {
            PptLiveNotesLink::Resolved { object, .. } => Some(source_ppt_shape_values(
                object.shapes().expect("resolve notes slide shapes"),
            )),
            PptLiveNotesLink::NotSpecified => None,
            value => panic!("strict notes fixture has unresolved notes relationship: {value:?}"),
        })
        .collect();
    PptNotesGraph {
        notes_size: (
            ppt_master_to_emu(live.document_atom.notes_size.x),
            ppt_master_to_emu(live.document_atom.notes_size.y),
        ),
        master_shapes,
        slide_shapes,
    }
}

fn target_ppt_notes_graph(document: &mut PresentationDocument) -> PptNotesGraph {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    let notes_master = presentation.notes_master_part(document);
    let slide_parts = presentation.slide_parts(document).collect::<Vec<_>>();
    let (notes_size, listed_notes_master) = {
        let root = presentation
            .root_element(document)
            .expect("parse converted PPTX notes graph");
        let listed = root
            .notes_master_id_list
            .as_deref()
            .and_then(|list| list.notes_master_id.as_deref())
            .map(|id| id.id.clone());
        ((root.notes_size.cx, root.notes_size.cy), listed)
    };
    assert_eq!(
        listed_notes_master,
        notes_master.as_ref().map(|master| {
            presentation
                .get_id_of_part(document, master)
                .expect("presentation owns converted notes master")
                .to_owned()
        })
    );
    let master_shapes = notes_master.as_ref().map(|master| {
        let root = master
            .root_element(document)
            .expect("parse converted notes master");
        target_ppt_shape_values(&root.common_slide_data.shape_tree)
    });
    let slide_shapes = slide_parts
        .into_iter()
        .map(|slide| {
            let notes = slide.notes_slide_part(document)?;
            assert_eq!(notes.slide_part(document).as_ref(), Some(&slide));
            assert_eq!(notes.notes_master_part(document), notes_master);
            let root = notes
                .root_element(document)
                .expect("parse converted notes slide");
            Some(target_ppt_shape_values(&root.common_slide_data.shape_tree))
        })
        .collect();
    PptNotesGraph {
        notes_size,
        master_shapes,
        slide_shapes,
    }
}

#[derive(Debug, PartialEq)]
struct PptMasterGraph {
    master_layout_types: Vec<Vec<p::SlideLayoutValues>>,
    slide_layout_types: Vec<p::SlideLayoutValues>,
}

fn target_ppt_master_graph(document: &mut PresentationDocument) -> PptMasterGraph {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    let master_parts = presentation
        .slide_master_parts(document)
        .collect::<Vec<_>>();
    let slide_parts = presentation.slide_parts(document).collect::<Vec<_>>();
    let listed_master_relationships = presentation
        .root_element(document)
        .expect("parse converted PPTX presentation")
        .slide_master_id_list
        .as_ref()
        .expect("converted PPTX lists its slide masters")
        .slide_master_id
        .iter()
        .map(|master| master.relationship_id.clone())
        .collect::<Vec<_>>();
    let actual_master_relationships = master_parts
        .iter()
        .map(|master| {
            presentation
                .get_id_of_part(document, master)
                .expect("presentation owns converted slide master")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(listed_master_relationships, actual_master_relationships);

    let master_layout_types = master_parts
        .iter()
        .map(|master| {
            let layout_parts = master.slide_layout_parts(document).collect::<Vec<_>>();
            let listed_layout_relationships = master
                .root_element(document)
                .expect("parse converted PPTX slide master")
                .slide_layout_id_list
                .as_ref()
                .expect("converted slide master lists its layouts")
                .slide_layout_id
                .iter()
                .map(|layout| layout.relationship_id.clone())
                .collect::<Vec<_>>();
            let actual_layout_relationships = layout_parts
                .iter()
                .map(|layout| {
                    master
                        .get_id_of_part(document, layout)
                        .expect("slide master owns converted layout")
                        .to_owned()
                })
                .collect::<Vec<_>>();
            assert_eq!(listed_layout_relationships, actual_layout_relationships);
            layout_parts
                .into_iter()
                .map(|layout| {
                    assert_eq!(layout.slide_master_part(document).as_ref(), Some(master));
                    layout
                        .root_element(document)
                        .expect("parse converted PPTX slide layout")
                        .r#type
                        .expect("converted slide layout has a typed layout kind")
                })
                .collect()
        })
        .collect();
    let slide_layout_types = slide_parts
        .into_iter()
        .map(|slide| {
            let layout = slide
                .slide_layout_part(document)
                .expect("converted slide has a layout relationship");
            assert!(layout.slide_master_part(document).is_some());
            layout
                .root_element(document)
                .expect("parse converted slide layout")
                .r#type
                .expect("converted slide layout has a typed layout kind")
        })
        .collect();
    PptMasterGraph {
        master_layout_types,
        slide_layout_types,
    }
}

#[derive(Debug, PartialEq)]
struct ExpectedPptGeometry {
    root_shape_id: u32,
    shapes: Vec<ExpectedPptShapeGeometry>,
}

#[derive(Debug, PartialEq)]
struct ExpectedPptShapeGeometry {
    shape_id: u32,
    preset: a::ShapeTypeValues,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    placeholder: Option<(p::PlaceholderValues, u32)>,
}

fn source_ppt_geometry(file: &PptFile) -> Vec<ExpectedPptGeometry> {
    let live = file
        .live_presentation()
        .expect("resolve strict live PPT geometry");
    let slide_size = (
        ppt_master_to_emu(live.document_atom.slide_size.x),
        ppt_master_to_emu(live.document_atom.slide_size.y),
    );
    live.slides()
        .expect("resolve strict PPT geometry slides")
        .into_iter()
        .map(|slide| {
            let source_shapes = slide.shapes().expect("resolve strict PPT geometry shapes");
            let root_shape_id = source_shapes
                .iter()
                .find(|shape| {
                    shape.shape.flags.contains(
                        olecfsdk::office_art::OfficeArtShapeFlags::GROUP
                            | olecfsdk::office_art::OfficeArtShapeFlags::PATRIARCH,
                    )
                })
                .map_or(1, |shape| shape.shape_id().max(1));
            let shapes = source_shapes
                .into_iter()
                .filter(|shape| {
                    !shape.shape.flags.contains(
                        olecfsdk::office_art::OfficeArtShapeFlags::GROUP
                            | olecfsdk::office_art::OfficeArtShapeFlags::PATRIARCH,
                    )
                })
                .map(|shape| {
                    assert_eq!(shape.shape_type(), 1);
                    let (x, y, cx, cy) = if shape
                        .shape
                        .flags
                        .contains(olecfsdk::office_art::OfficeArtShapeFlags::BACKGROUND)
                    {
                        (0, 0, slide_size.0, slide_size.1)
                    } else {
                        let anchor = shape.anchor().expect("resolve PPT anchor").expect(
                            "non-background shape in strict geometry fixture has an anchor",
                        );
                        (
                            ppt_master_to_emu(anchor.left),
                            ppt_master_to_emu(anchor.top),
                            ppt_master_to_emu(anchor.right - anchor.left),
                            ppt_master_to_emu(anchor.bottom - anchor.top),
                        )
                    };
                    let placeholder = shape.placeholder.map(|placeholder| {
                        let value = match placeholder.placement_id {
                            olecfsdk::ppt::PptPlaceholderType::Title => p::PlaceholderValues::Title,
                            olecfsdk::ppt::PptPlaceholderType::Body => p::PlaceholderValues::Body,
                            olecfsdk::ppt::PptPlaceholderType::CenterTitle => {
                                p::PlaceholderValues::CenteredTitle
                            }
                            olecfsdk::ppt::PptPlaceholderType::SubTitle => {
                                p::PlaceholderValues::SubTitle
                            }
                            value => panic!("unexpected strict geometry placeholder {value:?}"),
                        };
                        (
                            value,
                            u32::try_from(placeholder.position)
                                .expect("strict placeholder position is nonnegative"),
                        )
                    });
                    ExpectedPptShapeGeometry {
                        shape_id: shape.shape_id(),
                        preset: a::ShapeTypeValues::Rectangle,
                        x,
                        y,
                        cx,
                        cy,
                        placeholder,
                    }
                })
                .collect();
            ExpectedPptGeometry {
                root_shape_id,
                shapes,
            }
        })
        .collect()
}

fn target_ppt_geometry(document: &mut PresentationDocument) -> Vec<ExpectedPptGeometry> {
    let presentation_part = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    presentation_part
        .slide_parts(document)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|slide_part| {
            let slide = slide_part
                .root_element(document)
                .expect("parse converted PPTX slide geometry");
            let tree = &slide.common_slide_data.shape_tree;
            let root_shape_id = tree
                .non_visual_group_shape_properties
                .non_visual_drawing_properties
                .id;
            let shapes = tree
                .shape_tree_choice
                .iter()
                .filter_map(|choice| match choice {
                    ShapeTreeChoice::Shape(shape) => Some(shape),
                    _ => None,
                })
                .map(|shape| {
                    let transform = shape
                        .shape_properties
                        .transform2_d
                        .as_ref()
                        .expect("converted PPTX shape has a transform");
                    let offset = transform.offset.as_ref().expect("shape has an offset");
                    let extents = transform.extents.as_ref().expect("shape has extents");
                    let p::ShapePropertiesChoice::PresetGeometry(preset) = shape
                        .shape_properties
                        .shape_properties_choice1
                        .as_ref()
                        .expect("converted PPTX shape has preset geometry")
                    else {
                        panic!("converter emits preset geometry for native preset shapes")
                    };
                    let placeholder = shape
                        .non_visual_shape_properties
                        .application_non_visual_drawing_properties
                        .placeholder_shape
                        .as_ref()
                        .map(|placeholder| {
                            (
                                placeholder.r#type.expect("placeholder has a type"),
                                placeholder.index.expect("placeholder has an index"),
                            )
                        });
                    ExpectedPptShapeGeometry {
                        shape_id: shape
                            .non_visual_shape_properties
                            .non_visual_drawing_properties
                            .id,
                        preset: preset.preset,
                        x: coordinate_emu(offset.x),
                        y: coordinate_emu(offset.y),
                        cx: coordinate_emu(extents.cx),
                        cy: coordinate_emu(extents.cy),
                        placeholder,
                    }
                })
                .collect();
            ExpectedPptGeometry {
                root_shape_id,
                shapes,
            }
        })
        .collect()
}

const fn coordinate_emu(value: CoordinateValue) -> i64 {
    match value {
        CoordinateValue::Emu(value) => value,
        CoordinateValue::UniversalMeasure(_) => panic!("converter emits integral EMU coordinates"),
    }
}

fn ppt_master_to_emu(value: i32) -> i64 {
    let product = i64::from(value) * 3_175;
    if product >= 0 {
        (product + 1) / 2
    } else {
        (product - 1) / 2
    }
}

fn source_ppt_shapes(file: &PptFile) -> Vec<Vec<(u32, String)>> {
    let live = file
        .live_presentation()
        .expect("resolve strict live PPT presentation");
    live.slides()
        .expect("resolve strict PPT slides")
        .into_iter()
        .map(|slide| {
            source_ppt_shape_values(slide.shapes().expect("resolve strict PPT slide shapes"))
        })
        .collect()
}

fn source_ppt_shape_values(shapes: Vec<PptLiveShapeRef<'_>>) -> Vec<(u32, String)> {
    shapes
        .into_iter()
        .filter(|shape| {
            !shape.shape.flags.contains(
                olecfsdk::office_art::OfficeArtShapeFlags::GROUP
                    | olecfsdk::office_art::OfficeArtShapeFlags::PATRIARCH,
            )
        })
        .map(|shape| {
            let mut bodies = shape.text_bodies();
            if let Some(outline) = shape.outline_text {
                bodies.insert(0, outline.text_body);
            }
            let mut text = String::new();
            for (body_index, body) in bodies.into_iter().enumerate() {
                if body_index != 0 {
                    text.push('\n');
                }
                for atom in body.character_atoms() {
                    match atom {
                        PptLiveTextAtomRef::String { value, .. } => {
                            text.extend(value.chars().filter_map(|character| match character {
                                '\r' => Some('\n'),
                                value
                                    if value.is_control()
                                        && !matches!(value, '\t' | '\n' | '\u{000b}') =>
                                {
                                    None
                                }
                                value => Some(value),
                            }))
                        }
                        PptLiveTextAtomRef::CompatibilityUtf16 { .. } => {
                            panic!("strict PPT fixture contains compatible UTF-16")
                        }
                    }
                }
            }
            (shape.shape_id().max(1), text)
        })
        .collect()
}

fn target_ppt_shapes(document: &mut PresentationDocument) -> Vec<Vec<(u32, String)>> {
    let presentation = document
        .presentation_part()
        .expect("converted PPTX has a presentation part");
    let slide_parts = presentation.slide_parts(document).collect::<Vec<_>>();
    slide_parts
        .into_iter()
        .map(|slide_part| {
            let slide = slide_part
                .root_element(document)
                .expect("parse converted PPTX slide");
            target_ppt_shape_values(&slide.common_slide_data.shape_tree)
        })
        .collect()
}

fn target_ppt_shape_values(tree: &p::ShapeTree) -> Vec<(u32, String)> {
    tree.shape_tree_choice
        .iter()
        .filter_map(|choice| match choice {
            ShapeTreeChoice::Shape(shape) => Some(shape),
            _ => None,
        })
        .map(|shape| {
            let id = shape
                .non_visual_shape_properties
                .non_visual_drawing_properties
                .id;
            let text = shape
                .text_body
                .as_deref()
                .map(presentation_text_body_value)
                .unwrap_or_default();
            (id, text)
        })
        .collect()
}

fn drawing_text_body_value(body: &a::TextBody) -> String {
    drawing_paragraphs_value(&body.paragraph)
}

fn presentation_text_body_value(body: &p::TextBody) -> String {
    drawing_paragraphs_value(&body.paragraph)
}

fn drawing_paragraphs_value(paragraphs: &[a::Paragraph]) -> String {
    paragraphs
        .iter()
        .map(|paragraph| {
            let mut value = String::new();
            for choice in &paragraph.paragraph_choice {
                match choice {
                    DrawingParagraphChoice::Run(run) => value.push_str(&run.text),
                    DrawingParagraphChoice::Break(_) => value.push('\u{000b}'),
                    _ => {}
                }
            }
            value
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, PartialEq)]
struct ExpectedSheet {
    name: String,
    cells: Vec<ExpectedCell>,
}

#[derive(Debug, PartialEq)]
struct ExpectedCell {
    reference: String,
    kind: &'static str,
    value: Option<String>,
}

fn source_xls_cells(file: &XlsFile) -> Vec<ExpectedSheet> {
    let workbook = file.workbooks.first().expect("XLS has a Workbook stream");
    let view = workbook
        .relationships()
        .expect("resolve strict XLS workbook relationships");
    view.sheets()
        .iter()
        .copied()
        .map(|sheet| {
            let index = sheet
                .sparse_cell_index()
                .expect("build strict XLS sparse cell index");
            let mut cells = Vec::new();
            for row in index.rows() {
                for cell in row.cells() {
                    let header = cell.cell();
                    let expected = if let Some(label) = cell.label_sst() {
                        ExpectedCell {
                            reference: xls_cell_reference(header.row, header.column),
                            kind: "string",
                            value: view
                                .shared_string_value(label.shared_string_index)
                                .expect("decode XLS SST value"),
                        }
                    } else {
                        let value = view
                            .resolve_cell_value(&index, cell)
                            .expect("resolve XLS stored cell value");
                        expected_xls_value(xls_cell_reference(header.row, header.column), value)
                    };
                    cells.push(expected);
                }
            }
            ExpectedSheet {
                name: sheet.metadata().name.value.clone(),
                cells,
            }
        })
        .collect()
}

fn expected_xls_value(reference: String, value: XlsCellValue) -> ExpectedCell {
    let (kind, value) = match value {
        XlsCellValue::Blank => ("blank", None),
        XlsCellValue::Number(value) => ("number", Some(value.to_string())),
        XlsCellValue::Boolean(value) => ("boolean", Some(if value { "1" } else { "0" }.to_owned())),
        XlsCellValue::Error(value) => ("error", Some(xls_error(value).to_owned())),
        XlsCellValue::String(value) => ("string", Some(value)),
        XlsCellValue::Formula(value) => match value {
            XlsFormulaCachedValue::Number(value) => ("number", Some(value.to_string())),
            XlsFormulaCachedValue::String(value) => ("string", Some(value)),
            XlsFormulaCachedValue::Boolean(value) => {
                ("boolean", Some(if value { "1" } else { "0" }.to_owned()))
            }
            XlsFormulaCachedValue::Error(value) => ("error", Some(xls_error(value).to_owned())),
            XlsFormulaCachedValue::Empty => ("blank", None),
        },
        XlsCellValue::CompatibilityBoolErr { .. } => {
            panic!("strict XLS fixture has a compatibility BoolErr")
        }
    };
    ExpectedCell {
        reference,
        kind,
        value,
    }
}

fn target_xls_cells(document: &mut SpreadsheetDocument) -> Vec<ExpectedSheet> {
    let workbook_part = document
        .workbook_part()
        .expect("converted XLSX has a workbook part");
    let shared_strings = workbook_part
        .shared_string_table_part(document)
        .map(|part| {
            part.root_element(document)
                .expect("parse converted XLSX shared strings")
                .shared_string_item
                .iter()
                .map(|item| {
                    item.text
                        .as_ref()
                        .and_then(|text| text.0.xml_content.clone())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let names = workbook_part
        .root_element(document)
        .expect("parse converted XLSX workbook")
        .sheets
        .sheet
        .iter()
        .map(|sheet| sheet.name.clone())
        .collect::<Vec<_>>();
    let worksheet_parts = workbook_part.worksheet_parts(document).collect::<Vec<_>>();
    names
        .into_iter()
        .zip(worksheet_parts)
        .map(|(name, part)| {
            let root = part
                .root_element(document)
                .expect("parse converted XLSX worksheet");
            let cells = root
                .sheet_data
                .row
                .iter()
                .flat_map(|row| &row.cell)
                .map(|cell| {
                    let raw = cell
                        .cell_value
                        .as_ref()
                        .and_then(|value| value.0.xml_content.clone());
                    let (kind, value) = match cell.data_type {
                        Some(CellValues::SharedString) => {
                            let index = raw
                                .as_deref()
                                .expect("shared-string cell has an index")
                                .parse::<usize>()
                                .expect("shared-string index is numeric");
                            ("string", Some(shared_strings[index].clone()))
                        }
                        Some(CellValues::String) => ("string", raw),
                        Some(CellValues::Boolean) => ("boolean", raw),
                        Some(CellValues::Error) => ("error", raw),
                        Some(CellValues::Number) | None if raw.is_some() => ("number", raw),
                        None => ("blank", None),
                        Some(value) => panic!("unexpected converted cell type {value:?}"),
                    };
                    ExpectedCell {
                        reference: cell
                            .cell_reference
                            .clone()
                            .expect("converted cell has an A1 reference"),
                        kind,
                        value,
                    }
                })
                .collect();
            ExpectedSheet { name, cells }
        })
        .collect()
}

fn xls_cell_reference(row: u16, column: u16) -> String {
    let mut column = u32::from(column) + 1;
    let mut letters = Vec::new();
    while column != 0 {
        column -= 1;
        letters.push(char::from(b'A' + (column % 26) as u8));
        column /= 26;
    }
    letters.reverse();
    let mut value = letters.into_iter().collect::<String>();
    value.push_str(&(u32::from(row) + 1).to_string());
    value
}

const fn xls_error(value: CellErrorCode) -> &'static str {
    match value {
        CellErrorCode::Null => "#NULL!",
        CellErrorCode::DivisionByZero => "#DIV/0!",
        CellErrorCode::Value => "#VALUE!",
        CellErrorCode::Reference => "#REF!",
        CellErrorCode::Name => "#NAME?",
        CellErrorCode::Number => "#NUM!",
        CellErrorCode::NotAvailable => "#N/A",
        CellErrorCode::GettingData => "#GETTING_DATA",
    }
}

fn source_paragraph_text(file: &DocFile) -> Vec<String> {
    let tree = file.content_tree().expect("build strict DOC content tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    main.blocks()
        .expect("build DOC block order")
        .blocks()
        .iter()
        .filter_map(|block| match block {
            DocBlockRef::Paragraph(paragraph) => Some(*paragraph),
            DocBlockRef::Table(_) => None,
        })
        .map(|paragraph| source_paragraph_value(paragraph))
        .collect()
}

fn source_paragraph_styles(file: &DocFile) -> Vec<String> {
    let tree = file.content_tree().expect("build strict DOC content tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    main.blocks()
        .expect("build DOC block order")
        .blocks()
        .iter()
        .filter_map(|block| match block {
            DocBlockRef::Paragraph(paragraph) => Some(*paragraph),
            DocBlockRef::Table(_) => None,
        })
        .map(|paragraph| {
            let style = paragraph.style().expect("resolve DOC paragraph style");
            format!("Style{}", style.style_index())
        })
        .collect()
}

fn source_paragraph_outlines(file: &DocFile) -> Vec<Option<i32>> {
    let tree = file.content_tree().expect("build strict DOC content tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    main.blocks()
        .expect("build DOC block order")
        .blocks()
        .iter()
        .filter_map(|block| match block {
            DocBlockRef::Paragraph(paragraph) => Some(*paragraph),
            DocBlockRef::Table(_) => None,
        })
        .map(|paragraph| {
            let level = paragraph
                .style_state()
                .expect("resolve DOC paragraph style state")
                .outline_level()
                .raw();
            (level < 9).then_some(i32::from(level))
        })
        .collect()
}

fn source_direct_run_font_sizes(file: &DocFile) -> Vec<(Option<u64>, Option<u64>)> {
    let tree = file.content_tree().expect("build strict DOC content tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    main.blocks()
        .expect("build DOC block order")
        .blocks()
        .iter()
        .filter_map(|block| match block {
            DocBlockRef::Paragraph(paragraph) => Some(*paragraph),
            DocBlockRef::Table(_) => None,
        })
        .flat_map(|paragraph| paragraph.formatted_text_segments())
        .map(|segment| segment.expect("strict DOC has complete CHPX coverage"))
        .filter(
            |segment| match segment.text().value().expect("read strict DOC text") {
                DocTextPieceValueRef::String { value, .. } => value.chars().any(|character| {
                    !character.is_control() || matches!(character, '\t' | '\u{000b}')
                }),
                DocTextPieceValueRef::CompatibilityUtf16(_) => false,
            },
        )
        .map(|segment| {
            let properties = segment.character_run().source().properties.as_deref();
            let size = |kind| {
                properties
                    .into_iter()
                    .flat_map(|properties| properties.properties.iter())
                    .rev()
                    .find(|property| property.sprm.kind() == SprmKind::Known(kind))
                    .and_then(|property| match property.operand {
                        SprmOperand::Word(value) => Some(u64::from(u16::from_le_bytes(value))),
                        _ => None,
                    })
            };
            (size(KnownSprm::CHps), size(KnownSprm::CHpsBi))
        })
        .collect()
}

fn source_table_cells(file: &DocFile) -> Vec<Vec<String>> {
    let tree = file.content_tree().expect("build strict DOC table tree");
    let main = tree
        .part(FieldDocumentPart::Main)
        .expect("DOC has a main document part");
    let blocks = main.blocks().expect("build DOC table block order");
    let table = blocks
        .blocks()
        .iter()
        .find_map(|block| match block {
            DocBlockRef::Table(table) => Some(table),
            DocBlockRef::Paragraph(_) => None,
        })
        .expect("fixture has a table");
    table
        .rows()
        .iter()
        .map(|row| {
            row.cells()
                .expect("resolve DOC table cells")
                .cells()
                .iter()
                .map(|cell| {
                    cell.blocks()
                        .expect("resolve DOC cell block order")
                        .blocks()
                        .iter()
                        .filter_map(|block| match block {
                            DocBlockRef::Paragraph(paragraph) => {
                                Some(source_paragraph_value(*paragraph))
                            }
                            DocBlockRef::Table(_) => None,
                        })
                        .collect::<String>()
                })
                .collect()
        })
        .collect()
}

fn source_paragraph_value(paragraph: DocParagraphRef<'_>) -> String {
    let mut text = String::new();
    for segment in paragraph.text_segments() {
        let segment_range = segment.local_cp_range();
        let paragraph_range = paragraph.local_cp_range();
        assert!(paragraph_range.start <= segment_range.start);
        assert!(segment_range.end <= paragraph_range.end);
        match segment.value().expect("read clipped DOC text segment") {
            DocTextPieceValueRef::String { value, .. } => {
                text.extend(
                    value
                        .chars()
                        .filter(|character| !matches!(character, '\r' | '\u{0007}')),
                );
            }
            DocTextPieceValueRef::CompatibilityUtf16(_) => {
                panic!("strict DOC fixture contains compatible UTF-16")
            }
        }
    }
    text
}

fn target_paragraph_text(document: &mut WordprocessingDocument) -> Vec<String> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX main root");
    root.body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .map(|paragraph| target_paragraph_value(paragraph))
        .collect()
}

fn source_bookmarks(file: &DocFile) -> BTreeMap<String, BookmarkSnapshot> {
    let tree = file
        .content_tree()
        .expect("resolve strict DOC bookmark tree");
    tree.bookmarks()
        .expect("join strict DOC bookmark tables")
        .bookmarks()
        .iter()
        .map(|bookmark| {
            let properties = bookmark.properties();
            let (column_first, column_last) = if properties.column {
                assert!(properties.column_limit > properties.column_start);
                (
                    Some(i32::from(properties.column_start)),
                    Some(i32::from(properties.column_limit - 1)),
                )
            } else {
                (None, None)
            };
            let text = bookmark.text();
            let content = String::from_utf16(
                &(0..text.local_cp_range().len())
                    .filter_map(|offset| text.character_at(olecfsdk::doc::DocCp::new(offset)))
                    .filter(|character| {
                        !matches!(
                            character,
                            0x0001
                                | 0x0007
                                | 0x000d
                                | 0x0013
                                | 0x0014
                                | 0x0015
                                | 0x0019
                                | 0x001a
                                | 0x001b
                                | 0x001c
                                | 0x001d
                                | 0x001e
                                | 0x001f
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .expect("strict DOC bookmark content is valid UTF-16");
            (
                bookmark.index().to_string(),
                BookmarkSnapshot {
                    name: String::from_utf16(bookmark.name())
                        .expect("strict DOC bookmark name is valid UTF-16"),
                    column_first,
                    column_last,
                    content,
                },
            )
        })
        .collect()
}

type NotesSnapshot = (Vec<(i64, String)>, Vec<(i64, String)>, Vec<i64>, Vec<i64>);

fn source_notes(file: &DocFile) -> NotesSnapshot {
    let tree = file.content_tree().expect("resolve strict DOC note tree");
    let footnotes = tree.footnotes().expect("join strict DOC footnotes");
    let endnotes = tree.endnotes().expect("join strict DOC endnotes");
    let project = |notes: &olecfsdk::doc::DocNotes<'_>| {
        notes
            .notes()
            .iter()
            .map(|note| {
                (
                    i64::try_from(note.index() + 1).expect("fixture note ID fits i64"),
                    source_note_text(note.text()),
                )
            })
            .collect::<Vec<_>>()
    };
    (
        project(&footnotes),
        project(&endnotes),
        footnotes
            .notes()
            .iter()
            .map(|note| i64::try_from(note.index() + 1).expect("fixture note ID fits i64"))
            .collect(),
        endnotes
            .notes()
            .iter()
            .map(|note| i64::try_from(note.index() + 1).expect("fixture note ID fits i64"))
            .collect(),
    )
}

fn source_note_text(text: olecfsdk::doc::DocTextRangeRef<'_>) -> String {
    String::from_utf16(
        &(0..text.local_cp_range().len())
            .filter_map(|offset| text.character_at(olecfsdk::doc::DocCp::new(offset)))
            .filter(|character| !matches!(character, 0x0002 | 0x0007 | 0x000d))
            .collect::<Vec<_>>(),
    )
    .expect("strict DOC note text is valid UTF-16")
}

fn target_notes(document: &mut WordprocessingDocument) -> NotesSnapshot {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let footnotes = main
        .footnotes_part(document)
        .expect("converted DOCX has a footnotes part")
        .root_element(document)
        .expect("parse converted DOCX footnotes")
        .footnote
        .iter()
        .map(|note| {
            assert!(note.footnote_choice.iter().any(|choice| {
                matches!(choice, FootnoteChoice::Paragraph(paragraph) if paragraph.paragraph_choice.iter().any(|choice| matches!(choice, ParagraphChoice::WRun(run) if run.run_choice.contains(&RunChoice::FootnoteReferenceMark))))
            }));
            (
                note.id,
                note.footnote_choice
                    .iter()
                    .filter_map(|choice| match choice {
                        FootnoteChoice::Paragraph(paragraph) => {
                            Some(target_paragraph_value(paragraph))
                        }
                        _ => None,
                    })
                    .collect::<String>(),
            )
        })
        .collect();
    let endnotes = main
        .endnotes_part(document)
        .expect("converted DOCX has an endnotes part")
        .root_element(document)
        .expect("parse converted DOCX endnotes")
        .endnote
        .iter()
        .map(|note| {
            assert!(note.endnote_choice.iter().any(|choice| {
                matches!(choice, EndnoteChoice::Paragraph(paragraph) if paragraph.paragraph_choice.iter().any(|choice| matches!(choice, ParagraphChoice::WRun(run) if run.run_choice.contains(&RunChoice::EndnoteReferenceMark))))
            }));
            (
                note.id,
                note.endnote_choice
                    .iter()
                    .filter_map(|choice| match choice {
                        EndnoteChoice::Paragraph(paragraph) => {
                            Some(target_paragraph_value(paragraph))
                        }
                        _ => None,
                    })
                    .collect::<String>(),
            )
        })
        .collect();
    let root = main
        .root_element(document)
        .expect("parse converted DOCX main note references");
    let mut footnote_references = Vec::new();
    let mut endnote_references = Vec::new();
    for paragraph in root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph.as_ref()),
            _ => None,
        })
    {
        for run in paragraph
            .paragraph_choice
            .iter()
            .filter_map(|choice| match choice {
                ParagraphChoice::WRun(run) => Some(run.as_ref()),
                _ => None,
            })
        {
            for choice in &run.run_choice {
                match choice {
                    RunChoice::FootnoteReference(reference) => {
                        footnote_references.push(reference.id)
                    }
                    RunChoice::EndnoteReference(reference) => endnote_references.push(reference.id),
                    _ => {}
                }
            }
        }
    }
    (footnotes, endnotes, footnote_references, endnote_references)
}

#[derive(Debug, PartialEq, Eq)]
struct CommentSnapshot {
    author: String,
    initials: Option<String>,
    date: Option<String>,
    content: String,
    selected: String,
}

fn source_comments(file: &DocFile) -> BTreeMap<String, CommentSnapshot> {
    file.content_tree()
        .expect("resolve strict DOC comment tree")
        .comments()
        .expect("join strict DOC comments")
        .comments()
        .iter()
        .map(|comment| {
            (
                comment.index().to_string(),
                CommentSnapshot {
                    author: String::from_utf16(comment.author().unwrap_or_default())
                        .expect("strict DOC comment author is valid UTF-16"),
                    initials: (!comment.initials().is_empty()).then(|| {
                        String::from_utf16(comment.initials())
                            .expect("strict DOC comment initials are valid UTF-16")
                    }),
                    date: comment
                        .extended()
                        .and_then(|extended| source_comment_date(extended.modified)),
                    content: source_comment_text(comment.text()),
                    selected: source_comment_text(comment.commented_text()),
                },
            )
        })
        .collect()
}

fn source_comment_text(text: olecfsdk::doc::DocTextRangeRef<'_>) -> String {
    String::from_utf16(
        &(0..text.local_cp_range().len())
            .filter_map(|offset| text.character_at(olecfsdk::doc::DocCp::new(offset)))
            .filter(|character| {
                !matches!(
                    character,
                    0x0001
                        | 0x0005
                        | 0x0007
                        | 0x000d
                        | 0x0013
                        | 0x0014
                        | 0x0015
                        | 0x0019
                        | 0x001a
                        | 0x001b
                        | 0x001c
                        | 0x001d
                        | 0x001e
                        | 0x001f
                )
            })
            .collect::<Vec<_>>(),
    )
    .expect("strict DOC comment text is valid UTF-16")
}

fn source_comment_date(value: olecfsdk::doc::Dttm) -> Option<String> {
    if value.is_ignored() {
        return None;
    }
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00",
        1900u16 + value.year_offset,
        value.month,
        value.day,
        value.hour,
        value.minute
    ))
}

fn target_comments(document: &mut WordprocessingDocument) -> BTreeMap<String, CommentSnapshot> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let comments = main
        .wordprocessing_comments_part(document)
        .expect("converted DOCX has a comments part")
        .root_element(document)
        .expect("parse converted DOCX comments")
        .comment
        .iter()
        .map(|comment| {
            assert!(comment.comment_choice.iter().any(|choice| {
                matches!(choice, CommentChoice::Paragraph(paragraph) if paragraph.paragraph_choice.iter().any(|choice| matches!(choice, ParagraphChoice::WRun(run) if run.run_choice.contains(&RunChoice::AnnotationReferenceMark))))
            }));
            (
                comment.id.clone(),
                CommentSnapshot {
                    author: comment.author.clone(),
                    initials: comment.initials.clone(),
                    date: comment.date.clone(),
                    content: comment
                        .comment_choice
                        .iter()
                        .filter_map(|choice| match choice {
                            CommentChoice::Paragraph(paragraph) => {
                                Some(target_paragraph_value(paragraph))
                            }
                            _ => None,
                        })
                        .collect(),
                    selected: String::new(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let root = main
        .root_element(document)
        .expect("parse converted DOCX comment anchors");
    let mut projection = Vec::new();
    let mut starts = BTreeMap::new();
    let mut ends = BTreeMap::new();
    let mut references = BTreeMap::new();
    for choice in &root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
    {
        collect_target_comment_body(
            choice,
            &mut projection,
            &mut starts,
            &mut ends,
            &mut references,
        );
    }
    let mut result = comments;
    for (id, snapshot) in &mut result {
        let start = starts.remove(id).expect("comment has a range start");
        let end = ends.remove(id).expect("comment has a range end");
        assert!(references.remove(id).is_some(), "comment has a reference");
        snapshot.selected = String::from_utf16(
            &projection[start..end]
                .iter()
                .copied()
                .filter(|character| *character != 0x000d)
                .collect::<Vec<_>>(),
        )
        .expect("converted DOCX comment selection is valid UTF-16");
    }
    assert!(starts.is_empty(), "no orphan comment range starts");
    assert!(ends.is_empty(), "no orphan comment range ends");
    assert!(references.is_empty(), "no orphan comment references");
    result
}

type TargetCommentPositions = BTreeMap<String, usize>;

fn collect_target_comment_body(
    choice: &BodyChoice,
    projection: &mut Vec<u16>,
    starts: &mut TargetCommentPositions,
    ends: &mut TargetCommentPositions,
    references: &mut TargetCommentPositions,
) {
    match choice {
        BodyChoice::Paragraph(paragraph) => {
            collect_target_comment_paragraph(paragraph, projection, starts, ends, references)
        }
        BodyChoice::Table(table) => {
            collect_target_comment_table(table, projection, starts, ends, references)
        }
        _ => {}
    }
}

fn collect_target_comment_table(
    table: &ooxmlsdk::schemas::w::Table,
    projection: &mut Vec<u16>,
    starts: &mut TargetCommentPositions,
    ends: &mut TargetCommentPositions,
    references: &mut TargetCommentPositions,
) {
    for choice in &table.table_choice2 {
        let TableChoice2::TableRow(row) = choice else {
            continue;
        };
        for choice in &row.table_row_choice {
            let TableRowChoice::TableCell(cell) = choice else {
                continue;
            };
            for choice in &cell.table_cell_choice {
                match choice {
                    TableCellChoice::Paragraph(paragraph) => collect_target_comment_paragraph(
                        paragraph, projection, starts, ends, references,
                    ),
                    TableCellChoice::Table(table) => {
                        collect_target_comment_table(table, projection, starts, ends, references)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn collect_target_comment_paragraph(
    paragraph: &Paragraph,
    projection: &mut Vec<u16>,
    starts: &mut TargetCommentPositions,
    ends: &mut TargetCommentPositions,
    references: &mut TargetCommentPositions,
) {
    for choice in &paragraph.paragraph_choice {
        match choice {
            ParagraphChoice::CommentRangeStart(value) => {
                assert!(starts.insert(value.id.clone(), projection.len()).is_none());
            }
            ParagraphChoice::CommentRangeEnd(value) => {
                assert!(ends.insert(value.id.clone(), projection.len()).is_none());
            }
            ParagraphChoice::WRun(run) => {
                for choice in &run.run_choice {
                    match choice {
                        RunChoice::Text(value) => {
                            if let Some(value) = &value.0.xml_content {
                                projection.extend(value.encode_utf16());
                            }
                        }
                        RunChoice::FieldCode(value) => {
                            if let Some(value) = &value.0.xml_content {
                                projection.extend(value.encode_utf16());
                            }
                        }
                        RunChoice::TabChar => projection.push(u16::from(b'\t')),
                        RunChoice::Break(_) => projection.push(0x000b),
                        RunChoice::CommentReference(value) => {
                            assert!(
                                references
                                    .insert(value.id.clone(), projection.len())
                                    .is_none()
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    projection.push(u16::from(b'\r'));
}

#[derive(Debug, PartialEq, Eq)]
struct FloatingTextboxSnapshot {
    story_id: u16,
    chain_index: u16,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    z_order: u32,
    behind_text: bool,
    locked: bool,
    horizontal_origin: &'static str,
    vertical_origin: &'static str,
    wrap: &'static str,
    layout_in_cell: bool,
    allow_overlap: bool,
    hidden: bool,
    text_insets: [i32; 4],
    wrap_distances: [u32; 4],
    fill: Option<FloatingFillSnapshot>,
    line: Option<FloatingLineSnapshot>,
    content: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum FloatingFillSnapshot {
    None,
    Solid([u8; 3]),
}

#[derive(Debug, PartialEq, Eq)]
enum FloatingLineSnapshot {
    None,
    Solid { color: [u8; 3], width_emu: i32 },
}

#[derive(Debug, PartialEq, Eq)]
struct FloatingShapeSnapshot {
    preset: a::ShapeTypeValues,
    connector: bool,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    z_order: u32,
    behind_text: bool,
    locked: bool,
    horizontal_origin: &'static str,
    vertical_origin: &'static str,
    wrap: &'static str,
    layout_in_cell: bool,
    allow_overlap: bool,
    hidden: bool,
    horizontal_flip: bool,
    vertical_flip: bool,
    fill: FloatingFillSnapshot,
    line: FloatingLineSnapshot,
}

fn source_floating_shapes(file: &DocFile) -> BTreeMap<u32, FloatingShapeSnapshot> {
    let tree = file
        .content_tree_compatible()
        .expect("resolve compatible DOC floating-shape tree");
    let textboxes = tree.main_textboxes().expect("join DOC floating shapes");
    textboxes
        .anchors()
        .iter()
        .filter_map(|anchor| {
            let shape = anchor.shape()?;
            if shape.textbox_link().is_some()
                || shape.shape_type() == 75
                || shape
                    .primary_blip_identifier()
                    .expect("fixture has valid BLIP properties")
                    .is_some()
            {
                return None;
            }
            let source = anchor.source();
            let rectangle = source.rectangle;
            let to_emu = |value: i32| {
                i32::try_from(i64::from(value) * ooxmlsdk::units::EMUS_PER_TWIP)
                    .expect("fixture floating-shape geometry fits i32")
            };
            let size_to_emu = |value: u32| {
                i32::try_from(i64::from(value) * ooxmlsdk::units::EMUS_PER_TWIP)
                    .expect("fixture floating-shape extent fits i32")
            };
            let reverse_horizontal = rectangle.right < rectangle.left;
            let reverse_vertical = rectangle.bottom < rectangle.top;
            let fill = match shape.fill() {
                DocOfficeArtFill::None => FloatingFillSnapshot::None,
                DocOfficeArtFill::Solid(color) => FloatingFillSnapshot::Solid(
                    source_floating_color(color).expect("fixture uses direct RGB shape fill"),
                ),
                DocOfficeArtFill::Other { fill_type } => {
                    panic!("fixture uses unsupported shape fill type {fill_type}")
                }
            };
            let line = match shape.line() {
                DocOfficeArtLine::None => FloatingLineSnapshot::None,
                DocOfficeArtLine::Solid { color, width_emu } => FloatingLineSnapshot::Solid {
                    color: source_floating_color(color)
                        .expect("fixture uses direct RGB shape outline"),
                    width_emu,
                },
                DocOfficeArtLine::Other => panic!("fixture uses unsupported shape outline"),
            };
            Some((
                shape.shape().shape_id,
                FloatingShapeSnapshot {
                    preset: source_floating_shape_preset(shape.shape_type())
                        .expect("fixture shape has a supported preset geometry"),
                    connector: matches!(shape.shape_type(), 20 | 32..=40),
                    left: to_emu(rectangle.left.min(rectangle.right)),
                    top: to_emu(rectangle.top.min(rectangle.bottom)),
                    width: size_to_emu(rectangle.right.abs_diff(rectangle.left)),
                    height: size_to_emu(rectangle.bottom.abs_diff(rectangle.top)),
                    z_order: u32::try_from(shape.z_order())
                        .expect("fixture shape z-order fits u32"),
                    behind_text: source.below_text,
                    locked: source.anchor_locked,
                    horizontal_origin: source_horizontal_origin(source),
                    vertical_origin: source_vertical_origin(source),
                    wrap: source_wrap_style(source.wrap_style),
                    layout_in_cell: shape.layout_in_cell(),
                    allow_overlap: shape.allow_overlap(),
                    hidden: shape.hidden(),
                    horizontal_flip: shape
                        .shape()
                        .flags
                        .contains(OfficeArtShapeFlags::FLIP_HORIZONTAL)
                        ^ reverse_horizontal,
                    vertical_flip: shape
                        .shape()
                        .flags
                        .contains(OfficeArtShapeFlags::FLIP_VERTICAL)
                        ^ reverse_vertical,
                    fill,
                    line,
                },
            ))
        })
        .collect()
}

const fn source_floating_shape_preset(shape_type: u16) -> Option<a::ShapeTypeValues> {
    use a::ShapeTypeValues as Shape;
    Some(match shape_type {
        1 | 202 => Shape::Rectangle,
        2 => Shape::RoundRectangle,
        3 => Shape::Ellipse,
        4 => Shape::Diamond,
        5 => Shape::Triangle,
        6 => Shape::RightTriangle,
        7 => Shape::Parallelogram,
        8 => Shape::Trapezoid,
        9 => Shape::Hexagon,
        10 => Shape::Octagon,
        11 => Shape::Plus,
        12 => Shape::Star5,
        13 | 14 => Shape::RightArrow,
        15 => Shape::HomePlate,
        19 => Shape::Arc,
        20 | 32 => Shape::Line,
        21 => Shape::Plaque,
        22 => Shape::Can,
        23 => Shape::Donut,
        55 => Shape::Chevron,
        56 => Shape::Pentagon,
        57 => Shape::NoSmoking,
        58 => Shape::Star8,
        59 => Shape::Star16,
        60 => Shape::Star32,
        61 => Shape::WedgeRectangleCallout,
        62 => Shape::WedgeRoundRectangleCallout,
        63 => Shape::WedgeEllipseCallout,
        64 => Shape::Wave,
        66 => Shape::LeftArrow,
        67 => Shape::DownArrow,
        68 => Shape::UpArrow,
        69 => Shape::LeftRightArrow,
        70 => Shape::UpDownArrow,
        73 => Shape::LightningBolt,
        74 => Shape::Heart,
        76 => Shape::QuadArrow,
        95 => Shape::BlockArc,
        96 => Shape::SmileyFace,
        99 => Shape::CircularArrow,
        _ => return None,
    })
}

fn source_horizontal_origin(source: &olecfsdk::doc::ShapeAnchor) -> &'static str {
    if source.simple_rectangle {
        return "page";
    }
    match source.horizontal_origin {
        0 => "margin",
        1 => "page",
        2 => "column",
        value => panic!("unsupported fixture horizontal origin {value}"),
    }
}

fn source_vertical_origin(source: &olecfsdk::doc::ShapeAnchor) -> &'static str {
    if source.simple_rectangle {
        return "page";
    }
    match source.vertical_origin {
        0 => "margin",
        1 => "page",
        2 => "paragraph",
        value => panic!("unsupported fixture vertical origin {value}"),
    }
}

fn source_wrap_style(value: u8) -> &'static str {
    match value {
        0 | 2 => "square",
        1 => "top-bottom",
        3 => "none",
        4 => "tight",
        5 => "through",
        value => panic!("unsupported fixture wrap style {value}"),
    }
}

fn source_floating_textboxes(file: &DocFile) -> (BTreeMap<u32, FloatingTextboxSnapshot>, Vec<u32>) {
    let tree = file
        .content_tree()
        .expect("resolve strict DOC textbox tree");
    let textboxes = tree.main_textboxes().expect("join DOC textboxes");
    let mut formatting_losses = Vec::new();
    let snapshots = textboxes
        .anchors()
        .iter()
        .filter_map(|anchor| {
            let shape = anchor.shape()?;
            let (story, link) = textboxes.stories().iter().find_map(|story| {
                story.shapes().iter().find_map(|candidate| {
                    (candidate.shape().shape_id == shape.shape().shape_id)
                        .then(|| candidate.textbox_link().map(|link| (story, link)))
                        .flatten()
                })
            })?;
            let rectangle = anchor.source().rectangle;
            let to_emu = |value: i32| {
                i32::try_from(i64::from(value) * ooxmlsdk::units::EMUS_PER_TWIP)
                    .expect("fixture textbox geometry fits OOXML position")
            };
            let horizontal_origin = if anchor.source().simple_rectangle {
                "page"
            } else {
                match anchor.source().horizontal_origin {
                    0 => "margin",
                    1 => "page",
                    2 => "column",
                    value => panic!("unsupported fixture horizontal origin {value}"),
                }
            };
            let vertical_origin = if anchor.source().simple_rectangle {
                "page"
            } else {
                match anchor.source().vertical_origin {
                    0 => "margin",
                    1 => "page",
                    2 => "paragraph",
                    value => panic!("unsupported fixture vertical origin {value}"),
                }
            };
            let wrap = match anchor.source().wrap_style {
                0 | 2 => "square",
                1 => "top-bottom",
                3 => "none",
                value => panic!("unsupported fixture wrap style {value}"),
            };
            let insets = shape.text_insets();
            let distances = shape.wrap_distances();
            let fill = match shape.fill() {
                DocOfficeArtFill::None => Some(FloatingFillSnapshot::None),
                DocOfficeArtFill::Solid(color) => {
                    source_floating_color(color).map(FloatingFillSnapshot::Solid)
                }
                DocOfficeArtFill::Other { .. } => None,
            };
            let line = match shape.line() {
                DocOfficeArtLine::None => Some(FloatingLineSnapshot::None),
                DocOfficeArtLine::Solid { color, width_emu } => source_floating_color(color)
                    .map(|color| FloatingLineSnapshot::Solid { color, width_emu }),
                DocOfficeArtLine::Other => None,
            };
            let wrap_distances = [
                distances.left(),
                distances.top(),
                distances.right(),
                distances.bottom(),
            ];
            if fill.is_none() || line.is_none() || wrap_distances.iter().any(|value| *value < 0) {
                formatting_losses.push(shape.shape().shape_id);
            }
            Some((
                shape.shape().shape_id,
                FloatingTextboxSnapshot {
                    story_id: u16::try_from(story.index() + 1)
                        .expect("fixture textbox story index fits u16"),
                    chain_index: link.chain_index(),
                    left: to_emu(rectangle.left),
                    top: to_emu(rectangle.top),
                    width: to_emu(rectangle.right - rectangle.left),
                    height: to_emu(rectangle.bottom - rectangle.top),
                    z_order: u32::try_from(shape.z_order())
                        .expect("fixture textbox z-order fits u32"),
                    behind_text: anchor.source().below_text,
                    locked: anchor.source().anchor_locked,
                    horizontal_origin,
                    vertical_origin,
                    wrap,
                    layout_in_cell: shape.layout_in_cell(),
                    allow_overlap: shape.allow_overlap(),
                    hidden: shape.hidden(),
                    text_insets: [insets.left(), insets.top(), insets.right(), insets.bottom()],
                    wrap_distances: wrap_distances.map(|value| u32::try_from(value).unwrap_or(0)),
                    fill,
                    line,
                    content: (link.chain_index() == 0).then(|| source_comment_text(story.text())),
                },
            ))
        })
        .collect();
    formatting_losses.sort_unstable();
    (snapshots, formatting_losses)
}

fn source_floating_color(color: DocOfficeArtColor) -> Option<[u8; 3]> {
    match color {
        DocOfficeArtColor::Rgb { red, green, blue } => Some([red, green, blue]),
        DocOfficeArtColor::Other(_) => None,
    }
}

fn target_floating_shapes(
    document: &mut WordprocessingDocument,
) -> BTreeMap<u32, FloatingShapeSnapshot> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX floating-shape root");
    let mut result = BTreeMap::new();
    for choice in &root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
    {
        collect_target_floating_shape_body(choice, &mut result);
    }
    result
}

fn collect_target_floating_shape_body(
    choice: &BodyChoice,
    result: &mut BTreeMap<u32, FloatingShapeSnapshot>,
) {
    match choice {
        BodyChoice::Paragraph(paragraph) => {
            collect_target_floating_shape_paragraph(paragraph, result)
        }
        BodyChoice::Table(table) => collect_target_floating_shape_table(table, result),
        _ => {}
    }
}

fn collect_target_floating_shape_table(
    table: &ooxmlsdk::schemas::w::Table,
    result: &mut BTreeMap<u32, FloatingShapeSnapshot>,
) {
    for row in table
        .table_choice2
        .iter()
        .filter_map(|choice| match choice {
            TableChoice2::TableRow(row) => Some(row),
            _ => None,
        })
    {
        for cell in row
            .table_row_choice
            .iter()
            .filter_map(|choice| match choice {
                TableRowChoice::TableCell(cell) => Some(cell),
                _ => None,
            })
        {
            for choice in &cell.table_cell_choice {
                match choice {
                    TableCellChoice::Paragraph(paragraph) => {
                        collect_target_floating_shape_paragraph(paragraph, result)
                    }
                    TableCellChoice::Table(table) => {
                        collect_target_floating_shape_table(table, result)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn collect_target_floating_shape_paragraph(
    paragraph: &Paragraph,
    result: &mut BTreeMap<u32, FloatingShapeSnapshot>,
) {
    for anchor in paragraph
        .paragraph_choice
        .iter()
        .filter_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run),
            _ => None,
        })
        .flat_map(|run| &run.run_choice)
        .filter_map(|choice| match choice {
            RunChoice::Drawing(drawing) => drawing.drawing_choice.as_ref(),
            _ => None,
        })
        .filter_map(|choice| match choice {
            DrawingChoice::Anchor(anchor) => Some(anchor.as_ref()),
            _ => None,
        })
    {
        let Some(shape) = anchor
            .graphic
            .graphic_data
            .graphic_data_choice
            .iter()
            .find_map(|choice| match choice {
                a::GraphicDataChoice::WordprocessingShape(shape) => Some(shape.as_ref()),
                _ => None,
            })
        else {
            continue;
        };
        if shape.wordprocessing_shape_choice2.is_some() {
            continue;
        }
        let shape_id = shape
            .non_visual_drawing_properties
            .as_deref()
            .expect("floating shape retains source shape identity")
            .id;
        let properties = shape
            .shape_properties
            .as_deref()
            .expect("floating shape has typed properties");
        let preset = match properties.shape_properties_choice1.as_ref() {
            Some(wps::ShapePropertiesChoice::PresetGeometry(preset)) => preset.preset,
            value => panic!("floating shape uses typed preset geometry: {value:?}"),
        };
        let transform = properties
            .transform2_d
            .as_deref()
            .expect("floating shape has a typed transform");
        let fill = match properties.shape_properties_choice2.as_ref() {
            Some(wps::ShapePropertiesChoice2::NoFill(_)) => FloatingFillSnapshot::None,
            Some(wps::ShapePropertiesChoice2::SolidFill(fill)) => FloatingFillSnapshot::Solid(
                target_floating_color(fill).expect("floating shape has direct RGB fill"),
            ),
            value => panic!("unexpected converted floating shape fill {value:?}"),
        };
        let line = properties
            .outline
            .as_deref()
            .map(|outline| match outline.outline_choice1.as_ref() {
                Some(a::OutlineChoice::NoFill(_)) => FloatingLineSnapshot::None,
                Some(a::OutlineChoice::SolidFill(fill)) => FloatingLineSnapshot::Solid {
                    color: target_floating_color(fill)
                        .expect("floating shape has direct RGB outline"),
                    width_emu: outline.width.expect("floating shape outline has a width"),
                },
                value => panic!("unexpected converted floating shape outline {value:?}"),
            })
            .expect("floating shape retains an explicit outline");
        let left = match anchor
            .horizontal_position
            .as_deref()
            .and_then(|position| position.horizontal_position_choice.as_ref())
        {
            Some(wp::HorizontalPositionChoice::PositionOffset(value)) => *value,
            _ => panic!("floating shape uses an exact horizontal offset"),
        };
        let top = match anchor
            .vertical_position
            .as_deref()
            .and_then(|position| position.vertical_position_choice.as_ref())
        {
            Some(wp::VerticalPositionChoice::PositionOffset(value)) => *value,
            _ => panic!("floating shape uses an exact vertical offset"),
        };
        let horizontal_origin = match anchor
            .horizontal_position
            .as_deref()
            .expect("floating shape has horizontal positioning")
            .relative_from
        {
            wp::HorizontalRelativePositionValues::Margin => "margin",
            wp::HorizontalRelativePositionValues::Page => "page",
            wp::HorizontalRelativePositionValues::Column => "column",
            value => panic!("unexpected target horizontal origin {value:?}"),
        };
        let vertical_origin = match anchor
            .vertical_position
            .as_deref()
            .expect("floating shape has vertical positioning")
            .relative_from
        {
            wp::VerticalRelativePositionValues::Margin => "margin",
            wp::VerticalRelativePositionValues::Page => "page",
            wp::VerticalRelativePositionValues::Paragraph => "paragraph",
            value => panic!("unexpected target vertical origin {value:?}"),
        };
        let wrap = match anchor.anchor_choice.as_ref() {
            Some(wp::AnchorChoice::WrapSquare(_)) => "square",
            Some(wp::AnchorChoice::WrapTopBottom(_)) => "top-bottom",
            Some(wp::AnchorChoice::WrapNone) => "none",
            Some(wp::AnchorChoice::WrapTight(_)) => "tight",
            Some(wp::AnchorChoice::WrapThrough(_)) => "through",
            value => panic!("unexpected target floating shape wrap {value:?}"),
        };
        assert!(
            result
                .insert(
                    shape_id,
                    FloatingShapeSnapshot {
                        preset,
                        connector: matches!(
                            shape.wordprocessing_shape_choice1,
                            Some(wps::WordprocessingShapeChoice::NonVisualConnectorProperties(_))
                        ),
                        left,
                        top,
                        width: i32::try_from(anchor.extent.cx)
                            .expect("target floating-shape width fits i32"),
                        height: i32::try_from(anchor.extent.cy)
                            .expect("target floating-shape height fits i32"),
                        z_order: anchor
                            .relative_height
                            .expect("floating shape retains z-order"),
                        behind_text: bool::from(anchor.behind_doc),
                        locked: bool::from(anchor.locked),
                        horizontal_origin,
                        vertical_origin,
                        wrap,
                        layout_in_cell: bool::from(anchor.layout_in_cell),
                        allow_overlap: bool::from(anchor.allow_overlap),
                        hidden: anchor.hidden.is_some_and(bool::from),
                        horizontal_flip: transform.horizontal_flip.is_some_and(bool::from),
                        vertical_flip: transform.vertical_flip.is_some_and(bool::from),
                        fill,
                        line,
                    },
                )
                .is_none(),
            "floating shape identity is unique"
        );
    }
}

fn target_floating_textboxes(
    document: &mut WordprocessingDocument,
) -> BTreeMap<u32, FloatingTextboxSnapshot> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX textbox root");
    let mut result = BTreeMap::new();
    for choice in &root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
    {
        collect_target_floating_textbox_body(choice, &mut result);
    }
    result
}

fn collect_target_floating_textbox_body(
    choice: &BodyChoice,
    result: &mut BTreeMap<u32, FloatingTextboxSnapshot>,
) {
    match choice {
        BodyChoice::Paragraph(paragraph) => {
            collect_target_floating_textbox_paragraph(paragraph, result)
        }
        BodyChoice::Table(table) => collect_target_floating_textbox_table(table, result),
        _ => {}
    }
}

fn collect_target_floating_textbox_table(
    table: &ooxmlsdk::schemas::w::Table,
    result: &mut BTreeMap<u32, FloatingTextboxSnapshot>,
) {
    for row in table
        .table_choice2
        .iter()
        .filter_map(|choice| match choice {
            TableChoice2::TableRow(row) => Some(row),
            _ => None,
        })
    {
        for cell in row
            .table_row_choice
            .iter()
            .filter_map(|choice| match choice {
                TableRowChoice::TableCell(cell) => Some(cell),
                _ => None,
            })
        {
            for choice in &cell.table_cell_choice {
                match choice {
                    TableCellChoice::Paragraph(paragraph) => {
                        collect_target_floating_textbox_paragraph(paragraph, result)
                    }
                    TableCellChoice::Table(table) => {
                        collect_target_floating_textbox_table(table, result)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn collect_target_floating_textbox_paragraph(
    paragraph: &Paragraph,
    result: &mut BTreeMap<u32, FloatingTextboxSnapshot>,
) {
    for drawing in paragraph
        .paragraph_choice
        .iter()
        .filter_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run),
            _ => None,
        })
        .flat_map(|run| &run.run_choice)
        .filter_map(|choice| match choice {
            RunChoice::Drawing(drawing) => Some(drawing),
            _ => None,
        })
    {
        let Some(DrawingChoice::Anchor(anchor)) = &drawing.drawing_choice else {
            continue;
        };
        let shape = anchor
            .graphic
            .graphic_data
            .graphic_data_choice
            .iter()
            .find_map(|choice| match choice {
                a::GraphicDataChoice::WordprocessingShape(shape) => Some(shape.as_ref()),
                _ => None,
            })
            .expect("floating textbox uses a typed WPS shape");
        let shape_id = shape
            .non_visual_drawing_properties
            .as_deref()
            .expect("floating textbox retains source shape identity")
            .id;
        let Some(textbox_choice) = shape.wordprocessing_shape_choice2.as_ref() else {
            continue;
        };
        let (story_id, chain_index, content) = match textbox_choice {
            wps::WordprocessingShapeChoice2::TextBoxInfo2(textbox) => (
                textbox.id.expect("floating textbox has a story ID"),
                0,
                Some(target_textbox_content(
                    textbox
                        .text_box_content
                        .as_ref()
                        .expect("first floating textbox has typed content"),
                )),
            ),
            wps::WordprocessingShapeChoice2::LinkedTextBox(textbox) => {
                (textbox.id, textbox.sequence, None)
            }
        };
        let left = match anchor
            .horizontal_position
            .as_deref()
            .and_then(|position| position.horizontal_position_choice.as_ref())
        {
            Some(wp::HorizontalPositionChoice::PositionOffset(value)) => *value,
            _ => panic!("floating textbox uses an exact horizontal offset"),
        };
        let top = match anchor
            .vertical_position
            .as_deref()
            .and_then(|position| position.vertical_position_choice.as_ref())
        {
            Some(wp::VerticalPositionChoice::PositionOffset(value)) => *value,
            _ => panic!("floating textbox uses an exact vertical offset"),
        };
        let horizontal_origin = match anchor
            .horizontal_position
            .as_deref()
            .expect("floating textbox has horizontal positioning")
            .relative_from
        {
            wp::HorizontalRelativePositionValues::Margin => "margin",
            wp::HorizontalRelativePositionValues::Page => "page",
            wp::HorizontalRelativePositionValues::Column => "column",
            value => panic!("unexpected target horizontal origin {value:?}"),
        };
        let vertical_origin = match anchor
            .vertical_position
            .as_deref()
            .expect("floating textbox has vertical positioning")
            .relative_from
        {
            wp::VerticalRelativePositionValues::Margin => "margin",
            wp::VerticalRelativePositionValues::Page => "page",
            wp::VerticalRelativePositionValues::Paragraph => "paragraph",
            value => panic!("unexpected target vertical origin {value:?}"),
        };
        let wrap = match anchor.anchor_choice.as_ref() {
            Some(wp::AnchorChoice::WrapSquare(_)) => "square",
            Some(wp::AnchorChoice::WrapTopBottom(_)) => "top-bottom",
            Some(wp::AnchorChoice::WrapNone) => "none",
            value => panic!("unexpected target textbox wrap {value:?}"),
        };
        let properties = shape
            .shape_properties
            .as_deref()
            .expect("floating textbox has typed shape properties");
        let fill = match properties.shape_properties_choice2.as_ref() {
            Some(wps::ShapePropertiesChoice2::NoFill(_)) => Some(FloatingFillSnapshot::None),
            Some(wps::ShapePropertiesChoice2::SolidFill(fill)) => {
                target_floating_color(fill).map(FloatingFillSnapshot::Solid)
            }
            None => None,
            value => panic!("unexpected converted textbox fill {value:?}"),
        };
        let line = properties.outline.as_deref().and_then(|outline| {
            match outline.outline_choice1.as_ref() {
                Some(a::OutlineChoice::NoFill(_)) => Some(FloatingLineSnapshot::None),
                Some(a::OutlineChoice::SolidFill(fill)) => {
                    target_floating_color(fill).map(|color| FloatingLineSnapshot::Solid {
                        color,
                        width_emu: outline.width.expect("solid textbox outline has a width"),
                    })
                }
                None => None,
                value => panic!("unexpected converted textbox outline {value:?}"),
            }
        });
        let body_properties = shape
            .text_body_properties
            .as_deref()
            .expect("floating textbox has typed body properties");
        assert!(
            result
                .insert(
                    shape_id,
                    FloatingTextboxSnapshot {
                        story_id,
                        chain_index,
                        left,
                        top,
                        width: i32::try_from(anchor.extent.cx)
                            .expect("target textbox width fits i32"),
                        height: i32::try_from(anchor.extent.cy)
                            .expect("target textbox height fits i32"),
                        z_order: anchor.relative_height.expect("textbox retains z-order"),
                        behind_text: bool::from(anchor.behind_doc),
                        locked: bool::from(anchor.locked),
                        horizontal_origin,
                        vertical_origin,
                        wrap,
                        layout_in_cell: bool::from(anchor.layout_in_cell),
                        allow_overlap: bool::from(anchor.allow_overlap),
                        hidden: anchor.hidden.is_some_and(bool::from),
                        text_insets: [
                            body_properties
                                .left_inset
                                .expect("textbox retains its left inset"),
                            body_properties
                                .top_inset
                                .expect("textbox retains its top inset"),
                            body_properties
                                .right_inset
                                .expect("textbox retains its right inset"),
                            body_properties
                                .bottom_inset
                                .expect("textbox retains its bottom inset"),
                        ],
                        wrap_distances: [
                            anchor.distance_from_left.unwrap_or(0),
                            anchor.distance_from_top.unwrap_or(0),
                            anchor.distance_from_right.unwrap_or(0),
                            anchor.distance_from_bottom.unwrap_or(0),
                        ],
                        fill,
                        line,
                        content,
                    },
                )
                .is_none(),
            "floating textbox shape identity is unique"
        );
    }
}

fn target_floating_color(fill: &a::SolidFill) -> Option<[u8; 3]> {
    let a::SolidFillChoice::RgbColorModelHex(color) = fill.solid_fill_choice.as_ref()? else {
        return None;
    };
    let value = color.val.as_bytes();
    if value.len() != 6 {
        return None;
    }
    Some([
        target_hex_byte(&value[0..2])?,
        target_hex_byte(&value[2..4])?,
        target_hex_byte(&value[4..6])?,
    ])
}

fn target_hex_byte(value: &[u8]) -> Option<u8> {
    let digit = |value| match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    };
    Some(digit(*value.first()?)? << 4 | digit(*value.get(1)?)?)
}

fn target_textbox_content(content: &ooxmlsdk::schemas::w::TextBoxContent) -> String {
    content
        .text_box_content_choice
        .iter()
        .map(|choice| match choice {
            ooxmlsdk::schemas::w::TextBoxContentChoice::Paragraph(paragraph) => {
                target_paragraph_value(paragraph)
            }
            ooxmlsdk::schemas::w::TextBoxContentChoice::Table(table) => {
                target_textbox_table_content(table)
            }
            _ => String::new(),
        })
        .collect()
}

fn target_textbox_table_content(table: &ooxmlsdk::schemas::w::Table) -> String {
    table
        .table_choice2
        .iter()
        .filter_map(|choice| match choice {
            TableChoice2::TableRow(row) => Some(row),
            _ => None,
        })
        .flat_map(|row| &row.table_row_choice)
        .filter_map(|choice| match choice {
            TableRowChoice::TableCell(cell) => Some(cell),
            _ => None,
        })
        .flat_map(|cell| &cell.table_cell_choice)
        .map(|choice| match choice {
            TableCellChoice::Paragraph(paragraph) => target_paragraph_value(paragraph),
            TableCellChoice::Table(table) => target_textbox_table_content(table),
            _ => String::new(),
        })
        .collect()
}

fn source_cross_paragraph_bookmark_count(file: &DocFile) -> usize {
    file.content_tree()
        .expect("resolve strict DOC bookmark tree")
        .bookmarks()
        .expect("join strict DOC bookmark tables")
        .bookmarks()
        .iter()
        .filter(|bookmark| bookmark.text().paragraphs().count() > 1)
        .count()
}

fn target_bookmarks(document: &mut WordprocessingDocument) -> BTreeMap<String, BookmarkSnapshot> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX bookmark root");
    let mut projection = Vec::new();
    let mut starts = BTreeMap::new();
    let mut ends = BTreeMap::new();
    for choice in &root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
    {
        collect_target_bookmark_body(choice, &mut projection, &mut starts, &mut ends);
    }
    assert_eq!(starts.len(), ends.len(), "bookmark start/end cardinality");
    let result = starts
        .into_iter()
        .map(|(id, (name, column_first, column_last, start))| {
            let end = ends
                .remove(&id)
                .expect("each bookmark start has a matching end");
            assert!(start <= end, "bookmark {id} has a reversed target range");
            (
                id,
                BookmarkSnapshot {
                    name,
                    column_first,
                    column_last,
                    content: String::from_utf16(
                        &projection[start..end]
                            .iter()
                            .copied()
                            .filter(|character| *character != 0x000d)
                            .collect::<Vec<_>>(),
                    )
                    .expect("converted DOCX bookmark content is valid UTF-16"),
                },
            )
        })
        .collect();
    assert!(ends.is_empty(), "each bookmark end has a matching start");
    result
}

type TargetBookmarkStarts = BTreeMap<String, (String, Option<i32>, Option<i32>, usize)>;

fn collect_target_bookmark_body(
    choice: &BodyChoice,
    projection: &mut Vec<u16>,
    starts: &mut TargetBookmarkStarts,
    ends: &mut BTreeMap<String, usize>,
) {
    match choice {
        BodyChoice::Paragraph(paragraph) => {
            collect_target_bookmark_paragraph(paragraph, projection, starts, ends)
        }
        BodyChoice::Table(table) => collect_target_bookmark_table(table, projection, starts, ends),
        _ => {}
    }
}

fn collect_target_bookmark_table(
    table: &ooxmlsdk::schemas::w::Table,
    projection: &mut Vec<u16>,
    starts: &mut TargetBookmarkStarts,
    ends: &mut BTreeMap<String, usize>,
) {
    for choice in &table.table_choice2 {
        let TableChoice2::TableRow(row) = choice else {
            continue;
        };
        for choice in &row.table_row_choice {
            let TableRowChoice::TableCell(cell) = choice else {
                continue;
            };
            for choice in &cell.table_cell_choice {
                match choice {
                    TableCellChoice::Paragraph(paragraph) => {
                        collect_target_bookmark_paragraph(paragraph, projection, starts, ends)
                    }
                    TableCellChoice::Table(table) => {
                        collect_target_bookmark_table(table, projection, starts, ends)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn collect_target_bookmark_paragraph(
    paragraph: &Paragraph,
    projection: &mut Vec<u16>,
    starts: &mut TargetBookmarkStarts,
    ends: &mut BTreeMap<String, usize>,
) {
    for choice in &paragraph.paragraph_choice {
        match choice {
            ParagraphChoice::BookmarkStart(bookmark) => {
                assert!(
                    starts
                        .insert(
                            bookmark.id.clone(),
                            (
                                bookmark.name.clone(),
                                bookmark.column_first,
                                bookmark.column_last,
                                projection.len(),
                            ),
                        )
                        .is_none(),
                    "bookmark ID {} starts more than once",
                    bookmark.id
                );
            }
            ParagraphChoice::BookmarkEnd(bookmark) => {
                assert!(
                    ends.insert(bookmark.id.clone(), projection.len()).is_none(),
                    "bookmark ID {} ends more than once",
                    bookmark.id
                );
            }
            ParagraphChoice::WRun(run) => {
                for choice in &run.run_choice {
                    match choice {
                        RunChoice::Text(value) => {
                            if let Some(value) = &value.0.xml_content {
                                projection.extend(value.encode_utf16());
                            }
                        }
                        RunChoice::FieldCode(value) => {
                            if let Some(value) = &value.0.xml_content {
                                projection.extend(value.encode_utf16());
                            }
                        }
                        RunChoice::TabChar => projection.push(u16::from(b'\t')),
                        RunChoice::Break(_) => projection.push(0x000b),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    projection.push(u16::from(b'\r'));
}

fn target_paragraph_styles(document: &mut WordprocessingDocument) -> Vec<String> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX main root");
    root.body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .map(|paragraph| {
            paragraph
                .paragraph_properties
                .as_deref()
                .and_then(|properties| properties.paragraph_style_id.as_ref())
                .expect("converted paragraph has a typed style reference")
                .val
                .clone()
        })
        .collect()
}

fn target_paragraph_outlines(document: &mut WordprocessingDocument) -> Vec<Option<i32>> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX main root");
    root.body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .map(|paragraph| {
            paragraph
                .paragraph_properties
                .as_deref()
                .and_then(|properties| properties.outline_level.as_ref())
                .map(|level| level.val)
        })
        .collect()
}

fn assert_target_style_references_resolve(document: &mut WordprocessingDocument) {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let styles_part = main
        .style_definitions_part(document)
        .expect("converted DOCX has a style definitions part");
    let root = styles_part
        .root_element(document)
        .expect("parse converted DOCX style definitions");
    let ids = root
        .style
        .iter()
        .filter_map(|style| style.style_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!ids.is_empty());
    for style in &root.style {
        for reference in [
            style.based_on.as_ref().map(|value| value.val.as_str()),
            style
                .next_paragraph_style
                .as_ref()
                .map(|value| value.val.as_str()),
            style.linked_style.as_ref().map(|value| value.val.as_str()),
        ]
        .into_iter()
        .flatten()
        {
            assert!(
                ids.contains(reference),
                "unresolved style reference {reference}"
            );
        }
    }
}

fn assert_target_sample_style_formatting(document: &mut WordprocessingDocument) {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let styles_part = main
        .style_definitions_part(document)
        .expect("converted DOCX has a style definitions part");
    let root = styles_part
        .root_element(document)
        .expect("parse converted DOCX style definitions");
    let style = root
        .style
        .iter()
        .find(|style| style.style_id.as_deref() == Some("Style0"))
        .expect("converted DOCX preserves the normal style identity");
    let spacing = style
        .style_paragraph_properties
        .as_deref()
        .and_then(|properties| properties.spacing_between_lines.as_ref())
        .expect("converted normal style has typed paragraph spacing");
    assert_eq!(spacing.line, Some(SignedTwipsMeasureValue::Twips(276)));
    assert_eq!(spacing.line_rule, Some(LineSpacingRuleValues::Auto));
    assert_eq!(spacing.after, Some(SignedTwipsMeasureValue::Twips(200)));
    let run = style
        .style_run_properties
        .as_deref()
        .expect("converted normal style has typed run formatting");
    assert_eq!(
        run.font_size.as_ref().map(|size| size.val),
        Some(HpsMeasureValue::HalfPoints(22))
    );
    assert_eq!(
        run.font_size_complex_script.as_ref().map(|size| size.val),
        Some(HpsMeasureValue::HalfPoints(22))
    );
}

fn target_direct_run_font_sizes(
    document: &mut WordprocessingDocument,
) -> Vec<(Option<u64>, Option<u64>)> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX direct run formatting");
    root.body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .flat_map(|paragraph| paragraph.paragraph_choice.iter())
        .filter_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run),
            _ => None,
        })
        .map(|run| {
            let choices = run
                .run_properties
                .as_deref()
                .map(|properties| properties.run_properties_choice.as_slice())
                .unwrap_or_default();
            let size = choices.iter().rev().find_map(|choice| match choice {
                RunPropertiesChoice::FontSize(value) => match value.val {
                    HpsMeasureValue::HalfPoints(value) => Some(value),
                    HpsMeasureValue::UniversalMeasure(_) => None,
                },
                _ => None,
            });
            let complex_size = choices.iter().rev().find_map(|choice| match choice {
                RunPropertiesChoice::FontSizeComplexScript(value) => match value.val {
                    HpsMeasureValue::HalfPoints(value) => Some(value),
                    HpsMeasureValue::UniversalMeasure(_) => None,
                },
                _ => None,
            });
            (size, complex_size)
        })
        .collect()
}

fn target_doc_field_tokens(document: &mut WordprocessingDocument) -> Vec<ExpectedFieldToken> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX field root");
    let mut tokens = Vec::new();
    for run in root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .flat_map(|paragraph| paragraph.paragraph_choice.iter())
        .filter_map(|choice| match choice {
            ParagraphChoice::WRun(run) => Some(run),
            _ => None,
        })
    {
        for choice in &run.run_choice {
            match choice {
                RunChoice::Text(value) => {
                    if let Some(value) = &value.0.xml_content {
                        push_field_token(&mut tokens, ExpectedFieldToken::Text(value.clone()));
                    }
                }
                RunChoice::FieldCode(value) => {
                    if let Some(value) = &value.0.xml_content {
                        push_field_token(
                            &mut tokens,
                            ExpectedFieldToken::Instruction(value.clone()),
                        );
                    }
                }
                RunChoice::FieldChar(value) => {
                    tokens.push(ExpectedFieldToken::Marker(value.field_char_type));
                }
                _ => {}
            }
        }
    }
    tokens
}

fn push_field_token(tokens: &mut Vec<ExpectedFieldToken>, token: ExpectedFieldToken) {
    match (tokens.last_mut(), token) {
        (Some(ExpectedFieldToken::Text(current)), ExpectedFieldToken::Text(next))
        | (Some(ExpectedFieldToken::Instruction(current)), ExpectedFieldToken::Instruction(next)) => {
            current.push_str(&next)
        }
        (_, token) => tokens.push(token),
    }
}

fn assert_target_paragraph_start_indent(
    document: &mut WordprocessingDocument,
    paragraph_index: usize,
    expected: i64,
) {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX direct paragraph formatting");
    let paragraph = root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .filter_map(|choice| match choice {
            BodyChoice::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .nth(paragraph_index)
        .expect("converted DOCX preserves paragraph order");
    let indentation = paragraph
        .paragraph_properties
        .as_deref()
        .and_then(|properties| properties.indentation.as_ref())
        .expect("converted paragraph has typed direct indentation");
    assert_eq!(indentation.left, None);
    assert_eq!(
        indentation.start,
        Some(SignedTwipsMeasureValue::Twips(expected))
    );
}

fn target_table_cells(document: &mut WordprocessingDocument) -> Vec<Vec<String>> {
    let main = document
        .main_document_part()
        .expect("converted DOCX has a main document part");
    let root = main
        .root_element(document)
        .expect("parse converted DOCX table root");
    let table = root
        .body
        .as_deref()
        .expect("converted DOCX has a body")
        .body_choice
        .iter()
        .find_map(|choice| match choice {
            BodyChoice::Table(table) => Some(table),
            _ => None,
        })
        .expect("converted DOCX has a table");
    table
        .table_choice2
        .iter()
        .filter_map(|choice| match choice {
            TableChoice2::TableRow(row) => Some(row),
            _ => None,
        })
        .map(|row| {
            row.table_row_choice
                .iter()
                .filter_map(|choice| match choice {
                    TableRowChoice::TableCell(cell) => Some(cell),
                    _ => None,
                })
                .map(|cell| {
                    cell.table_cell_choice
                        .iter()
                        .filter_map(|choice| match choice {
                            TableCellChoice::Paragraph(paragraph) => Some(paragraph),
                            _ => None,
                        })
                        .map(|paragraph| target_paragraph_value(paragraph))
                        .collect::<String>()
                })
                .collect()
        })
        .collect()
}

fn target_paragraph_value(paragraph: &Paragraph) -> String {
    let mut text = String::new();
    for choice in &paragraph.paragraph_choice {
        let ParagraphChoice::WRun(run) = choice else {
            continue;
        };
        for choice in &run.run_choice {
            match choice {
                RunChoice::Text(value) => {
                    if let Some(value) = &value.0.xml_content {
                        text.push_str(value);
                    }
                }
                RunChoice::TabChar => text.push('\t'),
                RunChoice::Break(_) => text.push('\u{000b}'),
                _ => {}
            }
        }
    }
    text
}

impl ExpectedCoreProperties {
    fn mapped_count(&self) -> usize {
        [
            &self.title,
            &self.subject,
            &self.creator,
            &self.keywords,
            &self.description,
            &self.last_modified_by,
            &self.revision,
            &self.category,
            &self.content_type,
            &self.content_status,
            &self.language,
        ]
        .into_iter()
        .filter(|value| value.is_some())
        .count()
    }
}

fn source_core_properties(shared: &OfficeSharedContent) -> ExpectedCoreProperties {
    let mut expected = ExpectedCoreProperties::default();
    if let Some(stream) = shared.property_set(OfficePropertySetKind::SummaryInformation) {
        let property_set = stream
            .property_sets
            .first()
            .expect("SummaryInformation has its standard property set");
        let code_page = property_set
            .code_page()
            .expect("SummaryInformation CodePage is typed");
        for property in &property_set.properties {
            let destination = match property.identifier {
                2 => &mut expected.title,
                3 => &mut expected.subject,
                4 => &mut expected.creator,
                5 => &mut expected.keywords,
                6 => &mut expected.description,
                8 => &mut expected.last_modified_by,
                9 => &mut expected.revision,
                _ => continue,
            };
            if let Some(value) = property
                .string_value(code_page)
                .expect("SummaryInformation string is lossless")
            {
                assert!(destination.replace(value).is_none());
            }
        }
    }
    if let Some(stream) = shared.property_set(OfficePropertySetKind::DocumentSummaryInformation) {
        let property_set = stream
            .property_sets
            .first()
            .expect("DocumentSummaryInformation has its standard property set");
        let code_page = property_set
            .code_page()
            .expect("DocumentSummaryInformation CodePage is typed");
        for property in &property_set.properties {
            let destination = match property.identifier {
                2 => &mut expected.category,
                0x1a => &mut expected.content_type,
                0x1b => &mut expected.content_status,
                0x1c => &mut expected.language,
                _ => continue,
            };
            if let Some(value) = property
                .string_value(code_page)
                .expect("DocumentSummaryInformation string is lossless")
            {
                assert!(destination.replace(value).is_none());
            }
        }
    }
    expected
}

fn target_doc_core_properties(document: &mut WordprocessingDocument) -> ExpectedCoreProperties {
    let part = document
        .core_file_properties_part()
        .expect("converted DOCX has core properties");
    let root = part
        .root_element(document)
        .expect("parse converted DOCX core properties");
    target_core_properties(root)
}

fn target_xls_core_properties(document: &mut SpreadsheetDocument) -> ExpectedCoreProperties {
    let part = document
        .core_file_properties_part()
        .expect("converted XLSX has core properties");
    let root = part
        .root_element(document)
        .expect("parse converted XLSX core properties");
    target_core_properties(root)
}

fn target_ppt_core_properties(document: &mut PresentationDocument) -> ExpectedCoreProperties {
    let part = document
        .core_file_properties_part()
        .expect("converted PPTX has core properties");
    let root = part
        .root_element(document)
        .expect("parse converted PPTX core properties");
    target_core_properties(root)
}

fn target_core_properties(root: &CoreProperties) -> ExpectedCoreProperties {
    ExpectedCoreProperties {
        title: root.title.clone(),
        subject: root.subject.clone(),
        creator: root
            .creator
            .as_ref()
            .map(|value| value.xml_content.clone().unwrap_or_default()),
        keywords: root
            .keywords
            .as_ref()
            .map(|value| value.xml_content.clone().unwrap_or_default()),
        description: root.description.clone(),
        last_modified_by: root.last_modified_by.clone(),
        revision: root.revision.clone(),
        category: root.category.clone(),
        content_type: root.content_type.clone(),
        content_status: root.content_status.clone(),
        language: root
            .language
            .as_ref()
            .map(|value| value.xml_content.clone().unwrap_or_default()),
    }
}

fn assert_core_property_namespaces(package: &[u8]) {
    assert_xml_namespaces(
        package,
        "docProps/core.xml",
        &[
            "xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\"",
            "xmlns:dc=\"http://purl.org/dc/elements/1.1/\"",
            "xmlns:dcterms=\"http://purl.org/dc/terms/\"",
        ],
    );
}

fn fixture(relative: &str) -> std::path::PathBuf {
    let path = corpus_file_path(relative);
    assert!(path.is_file(), "fixture is missing: {}", path.display());
    path
}

fn assert_xml_namespaces(package: &[u8], part_name: &str, expected: &[&str]) {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open converted OOXML ZIP");
    let mut part = archive
        .by_name(part_name)
        .expect("converted OOXML package contains asserted XML part");
    let mut xml = String::new();
    part.read_to_string(&mut xml)
        .expect("converted OOXML part is UTF-8 XML");
    for declaration in expected {
        assert!(
            xml.contains(declaration),
            "{part_name} is missing namespace declaration {declaration}: {xml}"
        );
    }
}

fn assert_package_entry_contains(package: &[u8], part_name: &str, expected: &str) {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open converted OOXML ZIP");
    let mut part = archive
        .by_name(part_name)
        .expect("converted OOXML package contains asserted XML part");
    let mut xml = String::new();
    part.read_to_string(&mut xml)
        .expect("converted OOXML part is UTF-8 XML");
    assert!(
        xml.contains(expected),
        "{part_name} is missing {expected}: {xml}"
    );
}

fn assert_package_entry_family_contains(
    package: &[u8],
    prefix: &str,
    suffix: &str,
    expected: &str,
) {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open converted OOXML ZIP");
    let all_names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("read converted OOXML ZIP entry")
                .name()
                .to_owned()
        })
        .collect::<Vec<_>>();
    let names = all_names
        .iter()
        .filter(|name| name.starts_with(prefix) && name.ends_with(suffix))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        !names.is_empty(),
        "OOXML package has a {prefix}*{suffix} part; entries: {all_names:?}"
    );
    assert!(names.into_iter().any(|name| {
        let mut part = archive
            .by_name(&name)
            .expect("reopen converted OOXML family entry");
        let mut xml = String::new();
        part.read_to_string(&mut xml)
            .expect("converted OOXML family entry is UTF-8 XML");
        xml.contains(expected)
    }));
}

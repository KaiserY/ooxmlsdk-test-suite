use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use olecfsdk::{
    doc::{
        DocDataNodeValue, DocFile, DocPapxRun, DocSpecialContentLink, DocSpecialContentRef,
        DocTextPieceValueRef, FieldDocumentPart, NilPicfBinaryData, OleObjectPersist1Flags,
        TextPieceCharacters, TextPieceEncoding,
    },
    office_art::{
        OfficeArtBitmapData, OfficeArtDrawingGraphIssue, OfficeArtRecord, OfficeArtRecordData,
    },
    ppt::{
        BinaryTagData, CurrentUserData, PersistObjectReferenceStatus, PicturesStream, PptFile,
        PptLiveMasterLink, PptLiveNotesLink, PptLiveOutlineTextLink, PptLivePersistObjectRole,
        PptLivePresentation, PptRecordData, PptRecordSequence, PptSlideId,
        PptTopLevelLiveRecordStatus, PptTopLevelRecordRole,
    },
    xls::{
        BiffRecordData, XlsCellMut, XlsCellValueRef, XlsCustomViewActiveSheetLink,
        XlsCustomViewLink, XlsFile, XlsFileEntryRole, XlsFormulaDefinitionRef,
        XlsObjectPersistenceRef, XlsPivotCache, XlsPivotTableLink, XlsRevisionCellOrFormatRef,
        XlsRevisionLog, XlsRevisionRecordRef, XlsRevisionSheetLink, XlsStreamName, XlsUserNames,
        XlsUserRevisionLogLink,
    },
};
use olecfsdk_corpus_test_support::{
    corpus_bytes,
    manifest::{ExpectationMode, read_manifest},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocSavePolicy {
    Strict,
    PreserveCompatibility,
}

fn save_doc_with_policy(file: &DocFile, policy: DocSavePolicy) -> Result<Vec<u8>, String> {
    match policy {
        DocSavePolicy::Strict => file.to_bytes(),
        DocSavePolicy::PreserveCompatibility => file.to_bytes_preserving_compatibility(),
    }
    .map_err(|error| error.to_string())
}

#[test]
#[ignore = "classic Office file-root corpus round-trip runs explicitly"]
fn doc_files_round_trip_through_typed_root() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let exclusions = exclusions_for(&corpus, &["doc_fib_roundtrip"], &["doc"]);
    let files = corpus_files(&corpus, &["doc"]);
    let mut opened = 0usize;
    let mut reopened = 0usize;
    let mut strict_saved = 0usize;
    let mut compatibility_saved = 0usize;
    let mut strict_save_rejections = BTreeMap::<String, usize>::new();
    let mut rejected = BTreeMap::<String, usize>::new();
    let mut observed_exclusions = BTreeSet::new();
    let mut failures = Vec::new();
    let mut edited_document_properties = false;
    let mut relocated_associated_strings = false;
    let mut relocated_section_properties = false;
    let mut relocated_text_piece = false;
    let mut relocated_text_character_count = false;
    let mut relocated_multiple_text_edits = false;
    let mut relocated_character_format_boundary = false;
    let mut relocated_text_piece_boundary = false;
    let mut removed_text_piece = false;
    let mut relocated_paragraph_format_boundary = false;
    let mut edited_chpx_tree = false;
    let mut edited_papx_tree = false;
    let mut edited_paragraph_structure = false;
    let mut edited_non_main_paragraph_parts = BTreeSet::new();
    let mut non_main_paragraph_validation_errors =
        BTreeMap::<(FieldDocumentPart, String), usize>::new();
    let mut relocated_non_main_text_parts = BTreeSet::new();
    let mut edited_object_descriptor = false;
    let mut edited_data_picture = false;
    let mut edited_nil_picf_form = false;
    let mut edited_nil_picf_hyperlink = false;
    let mut relocated_data_node = false;
    let mut nil_picf_kinds = BTreeMap::<&'static str, usize>::new();
    let mut missing_chpx_cp_trees = BTreeSet::new();
    let mut missing_papx_cp_trees = BTreeSet::new();
    let mut direct_formatting_queries = 0usize;
    let mut direct_formatting_errors = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    let mut direct_table_states = BTreeMap::<(bool, u32, bool), usize>::new();
    let mut comment_table_states = BTreeMap::<(bool, u32, bool), usize>::new();
    let mut comment_cf_spec_markers = 0usize;
    let mut comment_cf_spec_false_markers = 0usize;
    let mut comment_cf_spec_errors = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    let mut comment_cf_spec_false = BTreeSet::<PathBuf>::new();
    let mut related_document_parts = 0usize;
    let mut related_text_pieces = 0usize;
    let mut related_paragraphs = 0usize;
    let mut related_character_runs = 0usize;
    let mut related_fields = 0usize;
    let mut related_bookmarks = 0usize;
    let mut related_footnotes = 0usize;
    let mut related_endnotes = 0usize;
    let mut related_comments = 0usize;
    let mut related_comment_replies = 0usize;
    let mut related_annotation_bookmarks = 0usize;
    let mut related_textbox_stories = 0usize;
    let mut related_textbox_breaks = 0usize;
    let mut related_shape_anchors = 0usize;
    let mut related_office_art_shapes = 0usize;
    let mut content_relationship_diagnostics = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    let mut related_tables = 0usize;
    let mut related_table_rows = 0usize;
    let mut related_table_cells = 0usize;
    let mut related_nested_tables = 0usize;
    let mut table_relationship_diagnostics = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    let mut related_pictures = 0usize;
    let mut related_binary_payloads = 0usize;
    let mut related_ole_objects = 0usize;
    let mut compatible_ole_objects = 0usize;
    let mut unresolved_special_contents = BTreeMap::<String, BTreeSet<PathBuf>>::new();

    for path in &files {
        if exclusions.contains_key(path) {
            observed_exclusions.insert(path.clone());
            continue;
        }
        let mut parsed_root = false;
        let result = (|| {
            let bytes = corpus_bytes(path).map_err(|error| error.to_string())?;
            let outcome =
                DocFile::from_bytes_compatible(&bytes).map_err(|error| error.to_string())?;
            let has_compatibility_diagnostics = !outcome.diagnostics.is_empty();
            let file = outcome.value;
            let content = file
                .content_tree_compatible()
                .map_err(|error| error.to_string())?;
            let bookmarks = content.bookmarks().map_err(|error| error.to_string())?;
            for diagnostic in bookmarks.diagnostics() {
                content_relationship_diagnostics
                    .entry(diagnostic.reason.clone())
                    .or_default()
                    .insert(path.clone());
            }
            for bookmark in bookmarks.bookmarks() {
                if bookmark.name().is_empty()
                    || bookmark.text().local_cp_range().len()
                        != bookmark.text().global_cp_range().len()
                {
                    return Err("DOC standard bookmark relationship changed".to_owned());
                }
                let _ = bookmark.text().paragraphs().count();
                related_bookmarks += 1;
            }
            for notes in [
                content.footnotes().map_err(|error| error.to_string())?,
                content.endnotes().map_err(|error| error.to_string())?,
            ] {
                for diagnostic in notes.diagnostics() {
                    content_relationship_diagnostics
                        .entry(diagnostic.reason.clone())
                        .or_default()
                        .insert(path.clone());
                }
                for note in notes.notes() {
                    if notes
                        .note_at_reference_cp(note.reference_cp())
                        .map(|value| value.index())
                        != Some(note.index())
                        || note.reference_document().part() != FieldDocumentPart::Main
                        || note.text().local_cp_range().len() != note.text().global_cp_range().len()
                    {
                        return Err("DOC note relationship changed".to_owned());
                    }
                    let _ = note.reference_character();
                    let _ = note.text().text_pieces().count();
                    match note.kind() {
                        olecfsdk::doc::DocNoteKind::Footnote => related_footnotes += 1,
                        olecfsdk::doc::DocNoteKind::Endnote => related_endnotes += 1,
                    }
                }
            }
            let comments = content.comments().map_err(|error| error.to_string())?;
            for diagnostic in comments.diagnostics() {
                content_relationship_diagnostics
                    .entry(diagnostic.reason.clone())
                    .or_default()
                    .insert(path.clone());
            }
            for comment in comments.comments() {
                if comments
                    .comment_at_reference_cp(comment.reference_cp())
                    .map(|value| value.index())
                    != Some(comment.index())
                    || comment.reference_document().part() != FieldDocumentPart::Main
                    || comment.text().document_part().part() != FieldDocumentPart::Comment
                {
                    return Err("DOC comment relationship changed".to_owned());
                }
                if let Some(parent) = comment.parent() {
                    if !parent
                        .children()
                        .any(|child| child.index() == comment.index())
                    {
                        return Err("DOC comment reply relationship changed".to_owned());
                    }
                    related_comment_replies += 1;
                }
                if comment.annotation_bookmark().is_some() {
                    related_annotation_bookmarks += 1;
                }
                let _ = comment.initials();
                let _ = comment.author();
                let _ = comment.commented_text().paragraphs().count();
                let _ = comment.text().paragraphs().count();
                related_comments += 1;
            }
            for textboxes in [
                content
                    .main_textboxes()
                    .map_err(|error| error.to_string())?,
                content
                    .header_textboxes()
                    .map_err(|error| error.to_string())?,
            ] {
                for diagnostic in textboxes.diagnostics() {
                    content_relationship_diagnostics
                        .entry(diagnostic.reason.clone())
                        .or_default()
                        .insert(path.clone());
                }
                for story in textboxes.stories() {
                    if textboxes.story(story.index()).map(|value| value.index())
                        != Some(story.index())
                        || story.document_part() != textboxes.document_part()
                    {
                        return Err("DOC textbox story relationship changed".to_owned());
                    }
                    for shape in story.shapes() {
                        if textboxes.shape(shape.shape().shape_id).is_none() {
                            return Err("DOC textbox shape relationship changed".to_owned());
                        }
                    }
                    for value in story.breaks() {
                        if value.story_index() != Some(story.index()) {
                            return Err("DOC textbox break relationship changed".to_owned());
                        }
                    }
                    let _ = story.text().paragraphs().count();
                    related_textbox_stories += 1;
                }
                related_textbox_breaks += textboxes.breaks().len();
                for anchor in textboxes.anchors() {
                    if anchor.anchor_document().part()
                        != match textboxes.document_part() {
                            olecfsdk::doc::TextboxDocumentPart::Main => FieldDocumentPart::Main,
                            olecfsdk::doc::TextboxDocumentPart::Header => FieldDocumentPart::Header,
                        }
                    {
                        return Err("DOC shape anchor relationship changed".to_owned());
                    }
                    let _ = anchor.anchor_character();
                    related_shape_anchors += 1;
                }
                related_office_art_shapes += textboxes.shapes().len();
            }
            let mut expected_part_start = 0u32;
            for part in content.parts() {
                let global = part.global_cp_range();
                if global.start.value() != expected_part_start
                    || global.len() != part.local_cp_range().len()
                {
                    return Err("DOC document-part CP aggregation is discontinuous".to_owned());
                }
                expected_part_start = global.end.value();
                related_document_parts += 1;
                for piece in part.text_pieces() {
                    if piece.descriptor().is_none()
                        || piece.global_cp_range().len() != piece.local_cp_range().len()
                    {
                        return Err(
                            "DOC document-part text-piece relationship is unresolved".to_owned()
                        );
                    }
                    let character_count = match piece.value().map_err(|error| error.to_string())? {
                        DocTextPieceValueRef::String {
                            value,
                            encoding: TextPieceEncoding::Compressed,
                        } => value.chars().count(),
                        DocTextPieceValueRef::String {
                            value,
                            encoding: TextPieceEncoding::Utf16,
                        } => value.encode_utf16().count(),
                        DocTextPieceValueRef::CompatibilityUtf16(value) => value.len(),
                    };
                    if u32::try_from(character_count).ok() != Some(piece.global_cp_range().len()) {
                        return Err(
                            "DOC text-piece CP range does not match borrowed character units"
                                .to_owned(),
                        );
                    }
                    related_text_pieces += 1;
                }
                for paragraph in part.paragraphs() {
                    if paragraph.global_cp_range().len() != paragraph.local_cp_range().len() {
                        return Err("DOC paragraph CP projection changed its length".to_owned());
                    }
                    let _ = paragraph.text_pieces().count();
                    let _ = paragraph.character_runs().count();
                    related_paragraphs += 1;
                }
                let tables = part.tables().map_err(|error| error.to_string())?;
                for diagnostic in tables.diagnostics() {
                    table_relationship_diagnostics
                        .entry(diagnostic.reason.clone())
                        .or_default()
                        .insert(path.clone());
                }
                for table in tables.tables() {
                    if table.global_cp_range().len() != table.local_cp_range().len()
                        || table.rows().first().map(|row| row.global_cp_range().start)
                            != Some(table.global_cp_range().start)
                        || table.rows().last().map(|row| row.global_cp_range().end)
                            != Some(table.global_cp_range().end)
                    {
                        return Err("DOC table CP relationship changed".to_owned());
                    }
                    for row in table.rows() {
                        if row.table_depth() != table.table_depth()
                            || row.global_cp_range().len() != row.local_cp_range().len()
                            || row.terminating_paragraph().global_cp_range().end
                                != row.global_cp_range().end
                        {
                            return Err("DOC table-row CP relationship changed".to_owned());
                        }
                        let cells = row.cells().map_err(|error| error.to_string())?;
                        for diagnostic in cells.diagnostics() {
                            table_relationship_diagnostics
                                .entry(diagnostic.reason.clone())
                                .or_default()
                                .insert(path.clone());
                        }
                        if cells.cells().len() != row.cell_count() {
                            return Err("DOC table-row cell relationship changed".to_owned());
                        }
                        for cell in cells.cells() {
                            if cell.global_cp_range().len() != cell.local_cp_range().len()
                                || cell.cell_mark().global_cp_range().end
                                    != cell.global_cp_range().end
                                || cell.paragraphs().next().is_none()
                            {
                                return Err("DOC table-cell CP relationship changed".to_owned());
                            }
                            related_nested_tables += tables.tables_in_cell(*cell).count();
                            related_table_cells += 1;
                        }
                        related_table_rows += 1;
                    }
                    related_tables += 1;
                }
                for run in part.character_runs() {
                    if run.global_cp_range().len() != run.local_cp_range().len() {
                        return Err("DOC character-run CP projection changed its length".to_owned());
                    }
                    let _ = run.text_pieces().count();
                    related_character_runs += 1;
                }
                let mut fields = part.fields().collect::<Vec<_>>();
                while let Some(field) = fields.pop() {
                    let local = field.local_cp_range().map_err(|error| error.to_string())?;
                    let aggregate = field.global_cp_range().map_err(|error| error.to_string())?;
                    if local.len() != aggregate.len()
                        || aggregate.start.value()
                            != part.global_cp_range().start.value() + local.start.value()
                    {
                        return Err("DOC field local/global CP relationship changed".to_owned());
                    }
                    fields.extend(field.instruction_fields());
                    fields.extend(field.result_fields());
                    related_fields += 1;
                }
                for content in part
                    .special_contents_compatible()
                    .map_err(|error| error.to_string())?
                {
                    match content {
                        DocSpecialContentLink::Resolved(DocSpecialContentRef::Picture {
                            ..
                        }) => related_pictures += 1,
                        DocSpecialContentLink::Resolved(DocSpecialContentRef::Binary {
                            ..
                        }) => related_binary_payloads += 1,
                        DocSpecialContentLink::Resolved(DocSpecialContentRef::OleObject {
                            ..
                        }) => related_ole_objects += 1,
                        DocSpecialContentLink::CompatibilityOleObject { .. } => {
                            compatible_ole_objects += 1
                        }
                        DocSpecialContentLink::Unresolved { reason, .. } => {
                            unresolved_special_contents
                                .entry(reason)
                                .or_default()
                                .insert(path.clone());
                        }
                    }
                }
            }
            if file.word_document.chpx_runs.is_none() {
                missing_chpx_cp_trees.insert(path.clone());
            }
            if file.word_document.papx_runs.is_none() {
                missing_papx_cp_trees.insert(path.clone());
            }
            if file.word_document.chpx_runs.is_some() && file.word_document.papx_runs.is_some() {
                for part in [
                    FieldDocumentPart::Main,
                    FieldDocumentPart::Footnote,
                    FieldDocumentPart::Header,
                    FieldDocumentPart::Comment,
                    FieldDocumentPart::Endnote,
                    FieldDocumentPart::Textbox,
                    FieldDocumentPart::HeaderTextbox,
                ] {
                    let (_, part_len) = doc_part_range(&file, part)?;
                    if part_len == 0 {
                        continue;
                    }
                    let positions = if part_len == 1 {
                        vec![0]
                    } else {
                        vec![0, part_len - 1]
                    };
                    for local_cp in positions {
                        match file.direct_formatting_at_cp(part, local_cp) {
                            Ok(formatting) => {
                                if formatting.part != part || formatting.local_cp != local_cp {
                                    return Err(
                                        "DOC direct-formatting aggregate changed its CP identity"
                                            .to_owned(),
                                    );
                                }
                                let state = formatting
                                    .paragraph
                                    .table_state()
                                    .map_err(|error| error.to_string())?;
                                *direct_table_states
                                    .entry((state.in_table, state.depth, state.depth_is_explicit))
                                    .or_default() += 1;
                                direct_formatting_queries += 1;
                            }
                            Err(error) => {
                                direct_formatting_errors
                                    .entry(error.to_string())
                                    .or_default()
                                    .insert(path.clone());
                            }
                        }
                    }
                }
                if let Some(annotations) = &file.table.annotations {
                    for range in annotations.text.value.positions
                        [..annotations.text.value.positions.len().saturating_sub(1)]
                        .windows(2)
                    {
                        match file.effective_cf_spec_at_cp(FieldDocumentPart::Comment, range[0]) {
                            Ok(true) => comment_cf_spec_markers += 1,
                            Ok(false) => {
                                comment_cf_spec_false_markers += 1;
                                comment_cf_spec_false.insert(path.clone());
                            }
                            Err(error) => {
                                comment_cf_spec_errors
                                    .entry(error.to_string())
                                    .or_default()
                                    .insert(path.clone());
                            }
                        }
                        let Some(local_cp) = range[1].checked_sub(1) else {
                            continue;
                        };
                        let Ok(formatting) =
                            file.direct_formatting_at_cp(FieldDocumentPart::Comment, local_cp)
                        else {
                            continue;
                        };
                        let state = formatting
                            .paragraph
                            .table_state()
                            .map_err(|error| error.to_string())?;
                        *comment_table_states
                            .entry((state.in_table, state.depth, state.depth_is_explicit))
                            .or_default() += 1;
                    }
                }
            }
            if let Some(data) = &file.data {
                for node in &data.nodes {
                    let DocDataNodeValue::Binary(value) = &node.value else {
                        continue;
                    };
                    let kind = match &value.binary_data {
                        NilPicfBinaryData::Unresolved(_) => "unresolved",
                        NilPicfBinaryData::Hyperlink { .. } => "hyperlink",
                        NilPicfBinaryData::Form { .. } => "form",
                        NilPicfBinaryData::Private { .. } => "private",
                        NilPicfBinaryData::Invalid { .. } => "invalid",
                        NilPicfBinaryData::InvalidContext(_) => "invalid-context",
                    };
                    *nil_picf_kinds.entry(kind).or_default() += 1;
                }
            }
            parsed_root = true;
            opened += 1;
            let (saved, save_policy) = match file.to_bytes() {
                Ok(saved) => {
                    strict_saved += 1;
                    (saved, DocSavePolicy::Strict)
                }
                Err(error) => {
                    if !has_compatibility_diagnostics {
                        return Err(format!(
                            "strict DOC save rejected a compatible parse without diagnostics: {error}"
                        ));
                    }
                    *strict_save_rejections.entry(error.to_string()).or_default() += 1;
                    compatibility_saved += 1;
                    (
                        file.to_bytes_preserving_compatibility()
                            .map_err(|error| error.to_string())?,
                        DocSavePolicy::PreserveCompatibility,
                    )
                }
            };
            let round_tripped = DocFile::from_bytes_compatible(&saved)
                .map_err(|error| error.to_string())?
                .value;
            if round_tripped.word_document != file.word_document
                || round_tripped.table != file.table
                || round_tripped.data != file.data
                || round_tripped.object_pool != file.object_pool
            {
                return Err("managed DOC Rust tree changed after write and reopen".to_owned());
            }
            if !round_tripped
                .source_compound_file()
                .logical_eq(file.source_compound_file())
            {
                return Err(
                    "DOC compound-file object tree changed after write and reopen".to_owned(),
                );
            }
            if !edited_document_properties
                && file.table.document_properties.is_some()
                && file.table.compatibility_tables.is_empty()
            {
                let mut edited = file.clone();
                let properties = &mut Arc::make_mut(&mut edited.table)
                    .document_properties
                    .as_mut()
                    .expect("checked above")
                    .value;
                properties.word97.base.format_flags.facing_pages =
                    !properties.word97.base.format_flags.facing_pages;
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                if edited_reopened.table.document_properties != edited.table.document_properties {
                    return Err(
                        "edited DOC document-properties node was not written through DocFile"
                            .to_owned(),
                    );
                }
                edited_document_properties = true;
            }
            if !relocated_associated_strings
                && file.table.compatibility_tables.is_empty()
                && file
                    .table
                    .associated_strings
                    .as_ref()
                    .is_some_and(|value| value.value.title.len() < 255)
            {
                let mut edited = file.clone();
                let strings = Arc::make_mut(&mut edited.table)
                    .associated_strings
                    .as_mut()
                    .expect("checked above");
                let old_length = strings.location.lcb;
                strings.value.title.push(u16::from(b'X'));
                let expected_title = strings.value.title.clone();
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened_strings = edited_reopened
                    .table
                    .associated_strings
                    .as_ref()
                    .ok_or_else(|| "edited DOC lost SttbfAssoc".to_owned())?;
                if reopened_strings.value.title != expected_title
                    || reopened_strings.location.lcb != old_length + 2
                {
                    return Err(
                        "variable-length DOC SttbfAssoc edit was not relocated through DocFile"
                            .to_owned(),
                    );
                }
                relocated_associated_strings = true;
            }
            if !relocated_section_properties
                && file.table.compatibility_tables.is_empty()
                && let Some(section_index) =
                    file.word_document
                        .section_properties
                        .iter()
                        .position(|section| {
                            section
                                .value
                                .as_ref()
                                .is_some_and(|value| !value.properties.properties.is_empty())
                        })
            {
                let mut edited = file.clone();
                let word_document = Arc::make_mut(&mut edited.word_document);
                let section = &mut word_document.section_properties[section_index];
                let old_offset = section.offset;
                let old_cb_mac = word_document.fib.rg_lw.cb_mac;
                let value = section.value.as_mut().expect("checked above");
                value
                    .properties
                    .properties
                    .push(value.properties.properties[0].clone());
                let expected = value.clone();
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened_section =
                    &edited_reopened.word_document.section_properties[section_index];
                if reopened_section.value.as_ref() != Some(&expected)
                    || reopened_section.offset == old_offset
                    || edited_reopened.word_document.fib.rg_lw.cb_mac <= old_cb_mac
                {
                    return Err(
                        "variable-length DOC Sepx edit was not relocated through PlcfSed"
                            .to_owned(),
                    );
                }
                relocated_section_properties = true;
            }
            if !relocated_text_piece
                && file.table.compatibility_tables.is_empty()
                && let Some(piece_index) = file.word_document.text_pieces.iter().position(|piece| {
                    piece.value.characters.encoding() == TextPieceEncoding::Compressed
                        && piece
                            .value
                            .characters
                            .value()
                            .is_some_and(|value| !value.is_empty())
                })
            {
                let mut edited = file.clone();
                let word_document = Arc::make_mut(&mut edited.word_document);
                let old_cb_mac = word_document.fib.rg_lw.cb_mac;
                let piece = &mut word_document.text_pieces[piece_index].value;
                let old_offset = piece.file_offset;
                let characters = piece
                    .characters
                    .value()
                    .expect("selected a conforming compressed text piece")
                    .to_owned();
                piece.characters = TextPieceCharacters::utf16(characters);
                let expected = piece.characters.clone();
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened_piece = &edited_reopened.word_document.text_pieces[piece_index].value;
                if reopened_piece.characters != expected
                    || reopened_piece.file_offset == old_offset
                    || edited_reopened.word_document.fib.rg_lw.cb_mac <= old_cb_mac
                    || edited_reopened.table.clx.value.piece_table.pieces[piece_index]
                        .file_position
                        .compressed
                {
                    return Err(
                        "DOC text encoding edit was not relocated through CLX and WordDocument"
                            .to_owned(),
                    );
                }
                relocated_text_piece = true;
            }
            if !relocated_text_character_count && file.table.compatibility_tables.is_empty() {
                'pieces: for piece in &file.word_document.text_pieces {
                    let piece_start = u32::try_from(piece.value.cp_start)
                        .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
                    if piece_start
                        >= u32::try_from(file.word_document.fib.rg_lw.ccp_text)
                            .map_err(|_| "DOC ccpText is negative".to_owned())?
                    {
                        break;
                    }
                    if piece.value.characters.encoding() != TextPieceEncoding::Compressed {
                        continue;
                    }
                    let Some(characters) = piece.value.characters.value() else {
                        continue;
                    };
                    for (index, character) in characters.chars().enumerate() {
                        if !character.is_ascii_alphabetic() {
                            continue;
                        }
                        let cp = piece_start
                            .checked_add(
                                u32::try_from(index)
                                    .map_err(|_| "DOC text piece index exceeds u32".to_owned())?,
                            )
                            .ok_or_else(|| "DOC text edit CP overflow".to_owned())?;
                        let mut edited = file.clone();
                        if edited
                            .replace_main_text_range(cp..cp + 1, format!("{character}X"))
                            .is_err()
                        {
                            continue;
                        }
                        let Ok(edited_bytes) = save_doc_with_policy(&edited, save_policy) else {
                            continue;
                        };
                        let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                            .map_err(|error| error.to_string())?
                            .value;
                        if edited_reopened.word_document.fib.rg_lw.ccp_text
                            != file.word_document.fib.rg_lw.ccp_text + 1
                            || edited_reopened
                                .table
                                .clx
                                .value
                                .piece_table
                                .character_positions
                                .last()
                                != edited
                                    .table
                                    .clx
                                    .value
                                    .piece_table
                                    .character_positions
                                    .last()
                            || edited_reopened.word_document.text_pieces[piece.piece_index]
                                .value
                                .characters
                                != edited.word_document.text_pieces[piece.piece_index]
                                    .value
                                    .characters
                        {
                            return Err(
                                "variable-length DOC text edit did not relocate CP/FC references"
                                    .to_owned(),
                            );
                        }
                        relocated_text_character_count = true;
                        break 'pieces;
                    }
                }
            }
            if file.table.compatibility_tables.is_empty() {
                if !relocated_multiple_text_edits
                    && try_doc_multiple_text_edits(&file, save_policy)?
                {
                    relocated_multiple_text_edits = true;
                }
                if !relocated_character_format_boundary
                    && try_doc_character_format_boundary_edit(&file, save_policy)?
                {
                    relocated_character_format_boundary = true;
                }
                if !relocated_text_piece_boundary
                    && try_doc_text_piece_boundary_edit(&file, save_policy)?
                {
                    relocated_text_piece_boundary = true;
                }
                if !removed_text_piece && try_doc_text_piece_removal(&file, save_policy)? {
                    removed_text_piece = true;
                }
                if !relocated_paragraph_format_boundary
                    && try_doc_paragraph_format_boundary_edit(&file, save_policy)?
                {
                    relocated_paragraph_format_boundary = true;
                }
                if !edited_chpx_tree && try_doc_chpx_tree_edit(&file, save_policy)? {
                    edited_chpx_tree = true;
                }
                if !edited_papx_tree && try_doc_papx_tree_edit(&file, save_policy)? {
                    edited_papx_tree = true;
                }
                if !edited_paragraph_structure && try_doc_paragraph_mark_edit(&file, save_policy)? {
                    edited_paragraph_structure = true;
                }
                for part in [
                    FieldDocumentPart::Footnote,
                    FieldDocumentPart::Header,
                    FieldDocumentPart::Comment,
                    FieldDocumentPart::Endnote,
                    FieldDocumentPart::Textbox,
                    FieldDocumentPart::HeaderTextbox,
                ] {
                    let structure_valid = match file.validate_document_part_structure(part) {
                        Ok(()) => true,
                        Err(error) => {
                            let (_, part_len) = doc_part_range(&file, part)?;
                            if part_len != 0 {
                                *non_main_paragraph_validation_errors
                                    .entry((part, error.to_string()))
                                    .or_default() += 1;
                            }
                            false
                        }
                    };
                    if !edited_non_main_paragraph_parts.contains(&part)
                        && structure_valid
                        && try_doc_part_paragraph_mark_edit(&file, part, save_policy)?
                    {
                        edited_non_main_paragraph_parts.insert(part);
                    }
                    if !relocated_non_main_text_parts.contains(&part)
                        && try_doc_part_text_edit(&file, part, save_policy)?
                    {
                        relocated_non_main_text_parts.insert(part);
                    }
                }
            }
            if !edited_object_descriptor
                && let Some(object) = file
                    .object_pool
                    .as_ref()
                    .and_then(|pool| pool.objects.first())
            {
                let mut edited = file.clone();
                let edited_object =
                    Arc::make_mut(edited.object_pool.as_mut().expect("checked above"))
                        .objects
                        .iter_mut()
                        .find(|candidate| candidate.path == object.path)
                        .expect("cloned object path is present");
                if edited_object
                    .descriptor
                    .persist1
                    .contains(OleObjectPersist1Flags::VIEW_OBJECT)
                {
                    edited_object
                        .descriptor
                        .persist1
                        .remove(OleObjectPersist1Flags::VIEW_OBJECT);
                } else {
                    edited_object
                        .descriptor
                        .persist1
                        .insert(OleObjectPersist1Flags::VIEW_OBJECT);
                }
                let expected = edited.object_pool.clone();
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                if edited_reopened.object_pool != expected {
                    return Err(
                        "edited DOC ObjectPool ODT node was not written through DocFile".to_owned(),
                    );
                }
                edited_object_descriptor = true;
            }
            if !edited_data_picture
                && let Some(data_node) = file.data.as_ref().and_then(|data| {
                    data.nodes
                        .iter()
                        .find(|node| matches!(node.value, DocDataNodeValue::Picture(_)))
                })
            {
                let mut edited = file.clone();
                let edited_node = Arc::make_mut(edited.data.as_mut().expect("checked above"))
                    .nodes
                    .iter_mut()
                    .find(|node| node.offset == data_node.offset)
                    .expect("cloned Data node is present");
                let DocDataNodeValue::Picture(picture) = &mut edited_node.value else {
                    unreachable!("selected a picture Data node")
                };
                let scale = &mut picture.picf.picture.horizontal_scale_tenths_percent;
                *scale = if *scale == u16::MAX {
                    scale.saturating_sub(1)
                } else {
                    scale.saturating_add(1)
                };
                let expected = edited_node.value.clone();
                let offset = edited_node.offset;
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened = edited_reopened
                    .data
                    .as_ref()
                    .and_then(|data| data.nodes.iter().find(|node| node.offset == offset));
                if reopened.map(|node| &node.value) != Some(&expected) {
                    return Err("edited DOC PICF node was not written through DocFile".to_owned());
                }
                edited_data_picture = true;
            }
            if !edited_nil_picf_form
                && let Some(data_node) = file.data.as_ref().and_then(|data| {
                    data.nodes.iter().find(|node| {
                        matches!(
                            &node.value,
                            DocDataNodeValue::Binary(value)
                                if matches!(
                                    &value.binary_data,
                                    NilPicfBinaryData::Form { value, .. }
                                        if value.name.characters.len() < 20
                                )
                        )
                    })
                })
            {
                let mut edited = file.clone();
                let node = Arc::make_mut(edited.data.as_mut().expect("checked above"))
                    .nodes
                    .iter_mut()
                    .find(|node| node.offset == data_node.offset)
                    .expect("cloned NilPICF node is present");
                let DocDataNodeValue::Binary(container) = &mut node.value else {
                    unreachable!("selected a binary Data node")
                };
                let NilPicfBinaryData::Form { value, .. } = &mut container.binary_data else {
                    unreachable!("selected an FFData node")
                };
                value.name.characters.push(b'X' as u16);
                let expected = container.binary_data.clone();
                let offset = node.offset;
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened = edited_reopened.data.as_ref().and_then(|data| {
                    data.nodes
                        .iter()
                        .find_map(|node| (node.offset == offset).then_some(&node.value))
                });
                if !matches!(
                    reopened,
                    Some(DocDataNodeValue::Binary(value)) if value.binary_data == expected
                ) {
                    return Err("edited DOC FFData node was not written through DocFile".to_owned());
                }
                edited_nil_picf_form = true;
            }
            if !edited_nil_picf_hyperlink
                && let Some(data_node) = file.data.as_ref().and_then(|data| {
                    data.nodes.iter().find(|node| {
                        matches!(
                            &node.value,
                            DocDataNodeValue::Binary(value)
                                if matches!(value.binary_data, NilPicfBinaryData::Hyperlink { .. })
                        )
                    })
                })
            {
                let mut edited = file.clone();
                let node = Arc::make_mut(edited.data.as_mut().expect("checked above"))
                    .nodes
                    .iter_mut()
                    .find(|node| node.offset == data_node.offset)
                    .expect("cloned NilPICF node is present");
                let DocDataNodeValue::Binary(container) = &mut node.value else {
                    unreachable!("selected a binary Data node")
                };
                let NilPicfBinaryData::Hyperlink { value, .. } = &mut container.binary_data else {
                    unreachable!("selected an HFD node")
                };
                value.bits.open_in_new_window = !value.bits.open_in_new_window;
                let expected = container.binary_data.clone();
                let offset = node.offset;
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened = edited_reopened.data.as_ref().and_then(|data| {
                    data.nodes
                        .iter()
                        .find_map(|node| (node.offset == offset).then_some(&node.value))
                });
                if !matches!(
                    reopened,
                    Some(DocDataNodeValue::Binary(value)) if value.binary_data == expected
                ) {
                    return Err("edited DOC HFD node was not written through DocFile".to_owned());
                }
                edited_nil_picf_hyperlink = true;
            }
            if !relocated_data_node
                && let Some((node_index, _)) = file.data.as_ref().and_then(|data| {
                    data.nodes.iter().enumerate().find(|(index, node)| {
                        *index + 1 < data.nodes.len()
                            && matches!(
                                &node.value,
                                DocDataNodeValue::ParagraphProperties(value)
                                    if !value.properties.properties.is_empty()
                            )
                    })
                })
            {
                let mut edited = file.clone();
                let data = Arc::make_mut(edited.data.as_mut().expect("checked above"));
                let old_length = data.nodes[node_index].physical_len;
                let old_following_offset = data.nodes[node_index + 1].offset;
                let DocDataNodeValue::ParagraphProperties(properties) =
                    &mut data.nodes[node_index].value
                else {
                    unreachable!("selected a PrcData node")
                };
                properties
                    .properties
                    .properties
                    .push(properties.properties.properties[0].clone());
                let expected = data.nodes[node_index].value.clone();
                let edited_bytes = save_doc_with_policy(&edited, save_policy)?;
                let edited_reopened = DocFile::from_bytes_compatible(&edited_bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened_data = edited_reopened
                    .data
                    .as_ref()
                    .ok_or_else(|| "variable-length edit lost the DOC Data stream".to_owned())?;
                if reopened_data.nodes[node_index].value != expected
                    || reopened_data.nodes[node_index].physical_len <= old_length
                    || reopened_data.nodes[node_index + 1].offset <= old_following_offset
                {
                    return Err(
                        "variable-length DOC PrcData edit did not relocate following SPRM offsets"
                            .to_owned(),
                    );
                }
                relocated_data_node = true;
            }
            reopened += 1;
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            if parsed_root {
                failures.push(format!("{}: {error}", path.display()));
            } else {
                *rejected.entry(error).or_default() += 1;
            }
        }
    }

    eprintln!(
        "DOC file roots: {} corpus files/{opened} opened/{reopened} reopened/{strict_saved} strict saves/{compatibility_saved} compatibility-preserving saves/{} manifest exclusions/{} other rejected; strict-save rejection shapes {strict_save_rejections:#?}; content relationships {related_document_parts} document parts/{related_text_pieces} text pieces/{related_paragraphs} paragraphs/{related_character_runs} character runs/{related_fields} fields/{related_bookmarks} bookmarks/{related_footnotes} footnotes/{related_endnotes} endnotes/{related_comments} comments/{related_comment_replies} comment replies/{related_annotation_bookmarks} annotation bookmarks/{related_textbox_stories} textbox stories/{related_textbox_breaks} textbox breaks/{related_shape_anchors} shape anchors/{related_office_art_shapes} OfficeArt shapes/{related_tables} tables/{related_table_rows} table rows/{related_table_cells} table cells/{related_nested_tables} nested-table links/{related_pictures} pictures/{related_binary_payloads} binary payloads/{related_ole_objects} OLE objects/{compatible_ole_objects} compatible OLE storages; content relationship diagnostics {content_relationship_diagnostics:#?}; table relationship diagnostics {table_relationship_diagnostics:#?}; unresolved special content {unresolved_special_contents:#?}; {direct_formatting_queries} direct-formatting queries; direct-formatting errors {direct_formatting_errors:#?}; direct table states {direct_table_states:?}; {comment_cf_spec_markers} effective comment CFSpec markers/{comment_cf_spec_false_markers} false; false-marker files {comment_cf_spec_false:#?}; CFSpec errors {comment_cf_spec_errors:#?}; comment paragraph-mark table states {comment_table_states:?}; edited non-main text parts {relocated_non_main_text_parts:?}; non-main paragraph validation errors {non_main_paragraph_validation_errors:#?}; NilPICF kinds {nil_picf_kinds:?}; rejection shapes {rejected:#?}",
        files.len(),
        exclusions.len(),
        rejected.values().sum::<usize>()
    );
    assert_eq!(observed_exclusions, exclusions.keys().cloned().collect());
    assert_eq!(
        strict_saved + compatibility_saved,
        opened,
        "every opened DOC must select an explicit save policy"
    );
    assert_eq!(
        strict_saved, 263,
        "strict DOC file-root save coverage changed; investigate before updating the ratchet"
    );
    assert_eq!(
        compatibility_saved, 140,
        "compatibility-preserving DOC save inventory changed; decreases should move coverage into the strict category"
    );
    assert_eq!(
        strict_save_rejections.values().sum::<usize>(),
        compatibility_saved,
        "strict-save rejection report does not account for every compatibility save"
    );
    assert!(
        missing_chpx_cp_trees.is_empty(),
        "conforming corpus files without a CHPX CP tree: {missing_chpx_cp_trees:#?}"
    );
    assert_eq!(
        missing_papx_cp_trees.len(),
        3,
        "PAPX CP-tree compatibility inventory changed: {missing_papx_cp_trees:#?}"
    );
    assert!(direct_formatting_queries > 0);
    assert_eq!(
        direct_formatting_errors
            .values()
            .map(BTreeSet::len)
            .sum::<usize>(),
        10,
        "direct-formatting compatibility inventory changed: {direct_formatting_errors:#?}"
    );
    assert!(direct_formatting_errors.keys().all(|error| {
        error.contains("no containing CHPX run")
            || error.contains("no containing PlcPcd text piece")
    }));
    assert_eq!(
        comment_table_states,
        BTreeMap::from([((false, 0, false), 86)]),
        "comment paragraph marks no longer all have direct table depth zero"
    );
    assert_eq!(
        comment_cf_spec_markers, 81,
        "effective sprmCFSpec comment-marker coverage changed"
    );
    assert_eq!(comment_cf_spec_false_markers, 5);
    assert_eq!(comment_cf_spec_false.len(), 3);
    assert!(comment_cf_spec_errors.is_empty());
    assert!(
        failures.is_empty(),
        "DOC file-root failures:\n{}",
        failures.join("\n")
    );
    assert!(opened > 0, "no DOC files opened through DocFile");
    assert!(
        edited_document_properties,
        "no DOC DOP node was edited through DocFile"
    );
    assert!(
        relocated_associated_strings,
        "no variable-length DOC table node was relocated through DocFile"
    );
    assert!(
        relocated_section_properties,
        "no variable-length DOC Sepx node was relocated through DocFile"
    );
    assert!(
        relocated_text_piece,
        "no DOC text piece encoding was relocated through DocFile"
    );
    assert!(
        relocated_text_character_count,
        "no variable-length DOC text edit relocated CP/FC references through DocFile"
    );
    assert!(
        relocated_multiple_text_edits,
        "no DOC text piece accepted multiple composable variable-length edits"
    );
    assert!(
        relocated_character_format_boundary,
        "no DOC text edit rebuilt a crossed CHPX formatting boundary"
    );
    assert!(
        relocated_text_piece_boundary,
        "no DOC text edit crossed a PlcPcd text-piece boundary"
    );
    assert!(
        removed_text_piece,
        "no DOC text edit removed a complete PlcPcd text-piece descriptor"
    );
    assert!(
        relocated_paragraph_format_boundary,
        "no DOC text edit rebuilt a crossed PAPX paragraph boundary"
    );
    assert!(
        edited_chpx_tree,
        "no public DOC CHPX CP node was edited and rebuilt into FKP pages"
    );
    assert!(
        edited_papx_tree,
        "no public DOC PAPX CP node was edited and rebuilt into FKP pages"
    );
    assert!(
        edited_paragraph_structure,
        "no DOC paragraph mark was inserted and deleted with an explicit PAPX CP tree"
    );
    assert_eq!(
        edited_non_main_paragraph_parts,
        BTreeSet::from([
            FieldDocumentPart::Footnote,
            FieldDocumentPart::Header,
            FieldDocumentPart::Comment,
            FieldDocumentPart::Endnote,
            FieldDocumentPart::Textbox,
            FieldDocumentPart::HeaderTextbox,
        ]),
        "not every non-main DOC document part inserted and deleted a paragraph mark with an explicit PAPX tree"
    );
    assert_eq!(
        relocated_non_main_text_parts,
        BTreeSet::from([
            FieldDocumentPart::Footnote,
            FieldDocumentPart::Header,
            FieldDocumentPart::Comment,
            FieldDocumentPart::Endnote,
            FieldDocumentPart::Textbox,
            FieldDocumentPart::HeaderTextbox,
        ]),
        "not every non-main DOC document part passed a variable-length CP/FC edit"
    );
    assert!(
        edited_object_descriptor,
        "no DOC ObjectPool ODT node was edited through DocFile"
    );
    assert!(
        edited_data_picture,
        "no DOC Data Stream PICF node was edited through DocFile"
    );
    assert!(
        edited_nil_picf_form,
        "no DOC Data Stream FFData node was edited through DocFile"
    );
    assert!(
        edited_nil_picf_hyperlink,
        "no DOC Data Stream HFD node was edited through DocFile"
    );
    assert!(
        relocated_data_node,
        "no variable-length DOC PrcData node was relocated through DocFile"
    );
    assert_eq!(reopened, opened, "not every opened DOC file was reopened");
    assert_eq!(
        files.len(),
        opened + exclusions.len() + rejected.values().sum::<usize>(),
        "DOC corpus inventory was not fully accounted for"
    );
    assert_eq!(nil_picf_kinds.get("unresolved"), None);
    assert!(nil_picf_kinds.get("form").copied().unwrap_or_default() > 0);
    assert!(nil_picf_kinds.get("hyperlink").copied().unwrap_or_default() > 0);
}

#[test]
#[ignore = "classic Office file-root corpus round-trip runs explicitly"]
fn ppt_files_round_trip_through_typed_root() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let exclusions = exclusions_for(
        &corpus,
        &["cfb_roundtrip", "ppt_record_roundtrip"],
        &["ppt"],
    );
    let files = corpus_files(&corpus, &["ppt"]);
    let mut opened = 0usize;
    let mut reopened = 0usize;
    let mut rejected = BTreeMap::<String, usize>::new();
    let mut observed_exclusions = BTreeSet::new();
    let mut failures = Vec::new();
    let mut relocated_ppt_layout = false;
    let mut relocated_ppt_picture = false;
    let mut rebuilt_ppt_live_state = false;
    let mut appended_ppt_user_edit = false;
    let mut edited_ppt_text_body = false;
    let mut persist_directory_files = 0usize;
    let mut current_persist_references = 0usize;
    let mut superseded_persist_references = 0usize;
    let mut incremental_save_metadata_records = 0usize;
    let mut unreferenced_top_level_records = 0usize;
    let mut persist_directory_errors = BTreeMap::<String, usize>::new();
    let mut live_presentation_files = 0usize;
    let mut live_master_slides = 0usize;
    let mut live_presentation_slides = 0usize;
    let mut live_notes_slides = 0usize;
    let mut live_active_x_controls = 0usize;
    let mut live_embedded_ole_objects = 0usize;
    let mut live_linked_ole_objects = 0usize;
    let mut live_vba_projects = 0usize;
    let mut live_persist_object_records = 0usize;
    let mut live_direct_persist_handles = 0usize;
    let mut live_list_text_bodies = 0usize;
    let mut live_outline_text_references = 0usize;
    let mut live_outline_text_shape_references = 0usize;
    let mut live_unresolved_outline_text_references = 0usize;
    let mut live_outline_text_errors = BTreeMap::<String, usize>::new();
    let mut live_slide_relationships = 0usize;
    let mut live_slide_master_links = BTreeMap::<&'static str, usize>::new();
    let mut live_slide_notes_links = BTreeMap::<&'static str, usize>::new();
    let mut live_slide_relationship_errors = BTreeMap::<String, usize>::new();
    let mut strict_live_slide_relationship_errors = BTreeMap::<String, usize>::new();
    let mut dead_top_level_records = 0usize;
    let mut live_presentation_errors = BTreeMap::<String, usize>::new();
    let mut live_drawing_graph_files = 0usize;
    let mut strict_live_drawing_graph_files = 0usize;
    let mut compatibility_live_drawing_graph_files = 0usize;
    let mut live_drawing_graph_drawings = 0usize;
    let mut live_drawing_graph_shapes = 0usize;
    let mut live_drawing_graph_blip_stores = 0usize;
    let mut live_drawing_graph_blip_entries = 0usize;
    let mut live_drawing_graph_blip_references = 0usize;
    let mut live_drawing_graph_blip_reference_count_relations = BTreeMap::<String, usize>::new();
    let mut live_drawing_graph_issues = BTreeMap::<&'static str, usize>::new();
    let mut live_drawing_graph_errors = BTreeMap::<String, usize>::new();
    let mut compatible_live_presentation_files = 0usize;
    let mut compatible_live_presentation_diagnostics = 0usize;
    let mut compatible_live_presentation_errors = BTreeMap::<String, usize>::new();

    for path in &files {
        if exclusions.contains_key(path) {
            observed_exclusions.insert(path.clone());
            continue;
        }
        let mut parsed_root = false;
        let result = (|| {
            let bytes = corpus_bytes(path).map_err(|error| error.to_string())?;
            let mut file = PptFile::from_bytes_compatible(&bytes)
                .map_err(|error| error.to_string())?
                .value;
            parsed_root = true;
            opened += 1;
            let mut file_text_body_edited = false;
            if matches!(file.current_user.data, CurrentUserData::Parsed(_)) {
                match file.persist_object_directory() {
                    Ok(directory) => {
                        persist_directory_files += 1;
                        for reference in &directory.references {
                            match reference.status {
                                PersistObjectReferenceStatus::Current => {
                                    current_persist_references += 1;
                                }
                                PersistObjectReferenceStatus::Superseded => {
                                    superseded_persist_references += 1;
                                }
                            }
                        }
                        for record in &directory.top_level_records {
                            match &record.role {
                                PptTopLevelRecordRole::IncrementalSaveMetadata(_) => {
                                    incremental_save_metadata_records += 1;
                                }
                                PptTopLevelRecordRole::Unreferenced => {
                                    unreferenced_top_level_records += 1;
                                }
                                PptTopLevelRecordRole::PersistObject { .. } => {}
                            }
                        }
                    }
                    Err(error) => {
                        let error = error.to_string();
                        let shape = if error.contains("UserEditAtom offset is not a record") {
                            "UserEditAtom offset is not a record"
                        } else if error.contains(
                            "previous UserEditAtom offset is not before the current UserEditAtom",
                        ) {
                            "previous UserEditAtom offset is not before the current UserEditAtom"
                        } else {
                            "other persist object directory error"
                        };
                        *persist_directory_errors
                            .entry(shape.to_owned())
                            .or_default() += 1;
                    }
                }
                match file.live_presentation() {
                    Ok(presentation) => {
                        live_presentation_files += 1;
                        live_master_slides += presentation.master_slides.len();
                        live_presentation_slides += presentation.presentation_slides.len();
                        live_notes_slides += presentation.notes_slides.len();
                        live_active_x_controls += presentation.active_x_controls.len();
                        live_embedded_ole_objects += presentation.embedded_ole_objects.len();
                        live_linked_ole_objects += presentation.linked_ole_objects.len();
                        live_vba_projects += usize::from(presentation.vba_project.is_some());
                        for object in std::iter::once(&presentation.document).chain(
                            presentation
                                .notes_master_slide
                                .iter()
                                .chain(presentation.handout_master_slide.iter())
                                .chain(&presentation.master_slides)
                                .chain(&presentation.presentation_slides)
                                .chain(&presentation.notes_slides)
                                .chain(&presentation.active_x_controls)
                                .chain(&presentation.embedded_ole_objects)
                                .chain(&presentation.linked_ole_objects)
                                .chain(presentation.vba_project.iter()),
                        ) {
                            live_direct_persist_handles += 1;
                            assert_eq!(
                                object.record.offset,
                                u64::from(object.reference.stream_offset)
                            );
                            assert!(std::ptr::eq(
                                object.record,
                                &file.document.records.records[object.reference.record_index]
                            ));
                            if object.slide_persist().is_some() {
                                live_list_text_bodies += object.text_bodies().len();
                                for link in object.outline_text_references_compatible() {
                                    match link {
                                        PptLiveOutlineTextLink::Resolved(reference) => {
                                            live_outline_text_references += 1;
                                            live_outline_text_shape_references +=
                                                usize::from(reference.shape_record.is_some());
                                        }
                                        PptLiveOutlineTextLink::Unresolved { .. } => {
                                            live_unresolved_outline_text_references += 1;
                                        }
                                    }
                                }
                                if let Err(error) = object.outline_text_references() {
                                    *live_outline_text_errors
                                        .entry(format!(
                                            "{} {:?}: {error}",
                                            path.display(),
                                            object.role
                                        ))
                                        .or_default() += 1;
                                }
                            }
                        }
                        match presentation.slides_compatible() {
                            Ok(slides) => {
                                live_slide_relationships += slides.len();
                                for slide in slides {
                                    let master_shape = match slide.master {
                                        PptLiveMasterLink::Resolved(master) => {
                                            assert_eq!(
                                                master.slide_persist().unwrap().slide_id,
                                                slide.slide_atom.master_id_ref
                                            );
                                            "resolved"
                                        }
                                        PptLiveMasterLink::NotSpecified => "not-specified",
                                        PptLiveMasterLink::Missing { .. } => "missing",
                                        PptLiveMasterLink::Ambiguous { .. } => "ambiguous",
                                    };
                                    *live_slide_master_links.entry(master_shape).or_default() += 1;
                                    let notes_shape = match slide.notes {
                                        PptLiveNotesLink::Resolved { notes_atom, .. } => {
                                            assert_eq!(
                                                notes_atom.slide_id_ref,
                                                slide.persist.slide_id
                                            );
                                            "resolved"
                                        }
                                        PptLiveNotesLink::NotSpecified => "not-specified",
                                        PptLiveNotesLink::Missing { .. } => "missing",
                                        PptLiveNotesLink::Ambiguous { .. } => "ambiguous",
                                        PptLiveNotesLink::SlideMismatch { .. } => "slide-mismatch",
                                    };
                                    *live_slide_notes_links.entry(notes_shape).or_default() += 1;
                                }
                            }
                            Err(error) => {
                                *live_slide_relationship_errors
                                    .entry(format!("{}: {error}", path.display()))
                                    .or_default() += 1;
                            }
                        }
                        if let Err(error) = presentation.slides() {
                            *strict_live_slide_relationship_errors
                                .entry(format!("{}: {error}", path.display()))
                                .or_default() += 1;
                        }
                        for record in &presentation.top_level_records {
                            match &record.status {
                                PptTopLevelLiveRecordStatus::LivePersistObject { .. } => {
                                    live_persist_object_records += 1;
                                }
                                PptTopLevelLiveRecordStatus::Dead => {
                                    dead_top_level_records += 1;
                                }
                                PptTopLevelLiveRecordStatus::LiveIncrementalSaveMetadata(_) => {}
                            }
                        }
                        match file.live_drawing_graph() {
                            Ok(graph) => {
                                live_drawing_graph_files += 1;
                                live_drawing_graph_drawings += graph.drawings.len();
                                live_drawing_graph_shapes += graph
                                    .drawings
                                    .iter()
                                    .map(|drawing| drawing.shapes.len())
                                    .sum::<usize>();
                                live_drawing_graph_blip_references += graph.blip_references.len();
                                if let Some(store) = &graph.blip_store {
                                    live_drawing_graph_blip_stores += 1;
                                    live_drawing_graph_blip_entries += store.entries.len();
                                    for entry in &store.entries {
                                        if let Some(relation) = entry.reference_count_relation {
                                            *live_drawing_graph_blip_reference_count_relations
                                                .entry(format!("{relation:?}"))
                                                .or_default() += 1;
                                        }
                                    }
                                }
                                for issue in &graph.issues {
                                    let shape = match issue {
                                        OfficeArtDrawingGraphIssue::MaximumShapeIdOutOfRange {
                                            ..
                                        } => "maximum-shape-id-out-of-range",
                                        OfficeArtDrawingGraphIssue::DrawingIdOutOfRange { .. } => {
                                            "drawing-id-out-of-range"
                                        }
                                        OfficeArtDrawingGraphIssue::DuplicateDrawingId { .. } => {
                                            "duplicate-drawing-id"
                                        }
                                        OfficeArtDrawingGraphIssue::DuplicateShapeId { .. } => {
                                            "duplicate-shape-id"
                                        }
                                        OfficeArtDrawingGraphIssue::ShapeInClusterZero { .. } => {
                                            "shape-in-cluster-zero"
                                        }
                                        OfficeArtDrawingGraphIssue::ShapeClusterMissing { .. } => {
                                            "shape-cluster-missing"
                                        }
                                        OfficeArtDrawingGraphIssue::ShapeClusterDrawingMismatch {
                                            ..
                                        } => "shape-cluster-drawing-mismatch",
                                        OfficeArtDrawingGraphIssue::BlipStoreEntryCountMismatch {
                                            ..
                                        } => "blip-store-entry-count-mismatch",
                                        OfficeArtDrawingGraphIssue::BlipReferenceOutOfRange {
                                            ..
                                        } => "blip-reference-out-of-range",
                                        OfficeArtDrawingGraphIssue::EmptyBlipStoreSlotReferenced {
                                            ..
                                        } => "empty-blip-store-slot-referenced",
                                    };
                                    *live_drawing_graph_issues.entry(shape).or_default() += 1;
                                }
                                if graph.validate_strict().is_ok() {
                                    strict_live_drawing_graph_files += 1;
                                } else {
                                    compatibility_live_drawing_graph_files += 1;
                                }
                            }
                            Err(error) => {
                                let error = error.to_string();
                                let shape = error
                                    .split_once(": ")
                                    .map_or(error.as_str(), |(_, shape)| shape);
                                *live_drawing_graph_errors
                                    .entry(shape.to_owned())
                                    .or_default() += 1;
                            }
                        }
                    }
                    Err(error) => {
                        let error = error.to_string();
                        let shape = error
                            .split_once(": ")
                            .map_or(error.as_str(), |(_, shape)| shape);
                        *live_presentation_errors
                            .entry(shape.to_owned())
                            .or_default() += 1;
                    }
                }
                match file.live_presentation_compatible() {
                    Ok(outcome) => {
                        compatible_live_presentation_files += 1;
                        compatible_live_presentation_diagnostics += outcome.diagnostics.len();
                    }
                    Err(error) => {
                        let error = error.to_string();
                        let shape = error
                            .split_once(": ")
                            .map_or(error.as_str(), |(_, shape)| shape);
                        *compatible_live_presentation_errors
                            .entry(shape.to_owned())
                            .or_default() += 1;
                    }
                }
                if !rebuilt_ppt_live_state && let Ok(before) = file.live_presentation() {
                    let mut rebuilt = file.clone();
                    rebuilt
                        .rebuild_current_live_state()
                        .map_err(|error| error.to_string())?;
                    let rebuilt_bytes = rebuilt
                        .to_bytes_preserving_compatibility()
                        .map_err(|error| error.to_string())?;
                    let rebuilt_reopened = PptFile::from_bytes_compatible(&rebuilt_bytes)
                        .map_err(|error| error.to_string())?
                        .value;
                    let after = rebuilt_reopened
                        .live_presentation()
                        .map_err(|error| error.to_string())?;
                    if ppt_live_signature(&after) != ppt_live_signature(&before)
                        || after.top_level_records.iter().any(|record| {
                            matches!(record.status, PptTopLevelLiveRecordStatus::Dead)
                        })
                        || after
                            .persist_object_directory
                            .incremental_save_chain
                            .edits
                            .len()
                            != 1
                    {
                        return Err(
                            "PPT current-live-state rebuild changed the live presentation"
                                .to_owned(),
                        );
                    }
                    rebuilt_ppt_live_state = true;
                }
                if !relocated_ppt_picture && try_ppt_picture_growth(&file)? {
                    relocated_ppt_picture = true;
                }
                if !appended_ppt_user_edit && try_ppt_append_user_edit(&file)? {
                    appended_ppt_user_edit = true;
                }
                let edited_text_body = !edited_ppt_text_body && try_ppt_text_body_edit(&mut file)?;
                if edited_text_body {
                    edited_ppt_text_body = true;
                }
                file_text_body_edited = edited_text_body;
            }
            let edited_ppt_layout = !relocated_ppt_layout && try_ppt_cstring_growth(&mut file)?;
            if edited_ppt_layout {
                relocated_ppt_layout = true;
            }
            let saved = file
                .to_bytes_preserving_compatibility()
                .map_err(|error| error.to_string())?;
            let round_tripped = PptFile::from_bytes_compatible(&saved)
                .map_err(|error| error.to_string())?
                .value;
            if round_tripped.document != file.document
                || round_tripped.current_user != file.current_user
                || round_tripped.pictures != file.pictures
            {
                return Err("managed PPT Rust tree changed after write and reopen".to_owned());
            }
            if !edited_ppt_layout
                && !file_text_body_edited
                && !round_tripped
                    .source_compound_file()
                    .logical_eq(file.source_compound_file())
            {
                return Err(
                    "PPT compound-file object tree changed after write and reopen".to_owned(),
                );
            }
            reopened += 1;
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            if parsed_root {
                failures.push(format!("{}: {error}", path.display()));
            } else {
                *rejected.entry(error).or_default() += 1;
            }
        }
    }

    assert_eq!(observed_exclusions, exclusions.keys().cloned().collect());
    assert!(
        failures.is_empty(),
        "PPT file-root failures:\n{}",
        failures.join("\n")
    );
    assert!(opened > 0, "no PPT files opened through PptFile");
    assert!(
        persist_directory_files > 0,
        "no PPT persist object directory was constructed"
    );
    assert!(
        current_persist_references > 0,
        "no current PPT persist object reference was classified"
    );
    assert!(
        live_presentation_files > 0,
        "no PPT live presentation was constructed"
    );
    assert_eq!(persist_directory_files, 167);
    assert_eq!(current_persist_references, 2_338);
    assert_eq!(superseded_persist_references, 189);
    assert_eq!(incremental_save_metadata_records, 498);
    assert_eq!(unreferenced_top_level_records, 0);
    assert_eq!(
        persist_directory_errors,
        BTreeMap::from([
            ("UserEditAtom offset is not a record".to_owned(), 7),
            (
                "previous UserEditAtom offset is not before the current UserEditAtom".to_owned(),
                1,
            ),
        ])
    );
    assert_eq!(live_presentation_files, 138);
    assert_eq!(live_master_slides, 199);
    assert_eq!(live_presentation_slides, 821);
    assert_eq!(live_notes_slides, 237);
    assert_eq!(live_active_x_controls, 0);
    assert_eq!(live_embedded_ole_objects, 74);
    assert_eq!(live_linked_ole_objects, 0);
    assert_eq!(live_vba_projects, 3);
    assert_eq!(live_persist_object_records, 1_588);
    assert_eq!(live_direct_persist_handles, 1_589);
    assert_eq!(live_list_text_bodies, 847);
    assert_eq!(live_outline_text_references, 682);
    assert_eq!(live_outline_text_shape_references, 682);
    assert_eq!(
        live_unresolved_outline_text_references, 2,
        "PPT compatible outline-text inventory changed"
    );
    assert_eq!(
        live_outline_text_errors.values().sum::<usize>(),
        2,
        "PPT strict outline-text relationship errors changed: {live_outline_text_errors:#?}"
    );
    assert_eq!(live_slide_relationships, 821);
    assert_eq!(
        live_slide_master_links,
        BTreeMap::from([("missing", 1), ("resolved", 820)])
    );
    assert_eq!(
        live_slide_notes_links,
        BTreeMap::from([("not-specified", 584), ("resolved", 237)])
    );
    assert!(live_slide_relationship_errors.is_empty());
    assert_eq!(
        strict_live_slide_relationship_errors
            .values()
            .sum::<usize>(),
        1,
        "PPT strict slide relationship errors changed: {strict_live_slide_relationship_errors:#?}"
    );
    assert_eq!(dead_top_level_records, 177);
    assert_eq!(live_drawing_graph_files, 138);
    assert_eq!(strict_live_drawing_graph_files, 0);
    assert_eq!(compatibility_live_drawing_graph_files, 138);
    assert_eq!(live_drawing_graph_drawings, 1_369);
    assert_eq!(live_drawing_graph_shapes, 13_786);
    assert_eq!(live_drawing_graph_blip_stores, 55);
    assert_eq!(live_drawing_graph_blip_entries, 446);
    assert_eq!(live_drawing_graph_blip_references, 735);
    assert_eq!(
        live_drawing_graph_blip_reference_count_relations,
        BTreeMap::from([
            ("AboveActual".to_owned(), 6),
            ("BelowActual".to_owned(), 1),
            ("EqualToActual".to_owned(), 439),
        ])
    );
    assert!(live_drawing_graph_errors.is_empty());
    assert_eq!(
        live_drawing_graph_issues,
        BTreeMap::from([
            ("duplicate-drawing-id", 13),
            ("duplicate-shape-id", 26),
            ("shape-cluster-drawing-mismatch", 1_857),
            ("shape-cluster-missing", 1),
            ("shape-in-cluster-zero", 27),
        ])
    );
    assert_eq!(
        live_presentation_errors,
        BTreeMap::from([
            (
                "MasterPersistAtom.persistIdRef does not resolve to MasterOrSlideContainer"
                    .to_owned(),
                1,
            ),
            (
                "NotesPersistAtom.persistIdRef does not resolve to NotesContainer".to_owned(),
                2,
            ),
            ("PPT UserEditAtom offset is not a record".to_owned(), 7),
            (
                "PPT previous UserEditAtom offset is not before the current UserEditAtom"
                    .to_owned(),
                1,
            ),
            (
                "VBAInfoAtom fHasMacros 0 or version 0 violates MS-PPT 2.4.11".to_owned(),
                1,
            ),
            (
                "VBAInfoAtom fHasMacros 0 or version 1 violates MS-PPT 2.4.11".to_owned(),
                23,
            ),
            (
                "required DocumentContainer.documentAtom is missing".to_owned(),
                1,
            ),
            (
                "required DocumentContainer.masterList is missing".to_owned(),
                1,
            ),
        ])
    );
    assert_eq!(compatible_live_presentation_files, 166);
    assert_eq!(compatible_live_presentation_diagnostics, 30);
    assert_eq!(
        compatible_live_presentation_errors,
        BTreeMap::from([
            ("PPT UserEditAtom offset is not a record".to_owned(), 7),
            (
                "PPT previous UserEditAtom offset is not before the current UserEditAtom"
                    .to_owned(),
                1,
            ),
            (
                "required DocumentContainer.documentAtom is missing".to_owned(),
                1,
            ),
        ])
    );
    assert!(
        relocated_ppt_layout,
        "no variable-length PPT CString was relaid out through PptFile"
    );
    assert!(
        rebuilt_ppt_live_state,
        "no PPT physical history was rebuilt from the live presentation"
    );
    assert!(
        relocated_ppt_picture,
        "no PPT Pictures Stream edit relocated a live OfficeArtFBSE reference"
    );
    assert!(
        appended_ppt_user_edit,
        "no PPT live persist-object edit appended a user-edit checkpoint"
    );
    assert!(
        edited_ppt_text_body,
        "no PPT list text body was edited through its typed transaction"
    );
    assert_eq!(reopened, opened, "not every opened PPT file was reopened");
    assert_eq!(
        files.len(),
        opened + exclusions.len() + rejected.values().sum::<usize>(),
        "PPT corpus inventory was not fully accounted for"
    );
    eprintln!(
        "PPT file roots: {} corpus files/{opened} opened/{reopened} reopened/{} manifest exclusions/{} other rejected; rejection shapes {rejected:#?}",
        files.len(),
        exclusions.len(),
        rejected.values().sum::<usize>()
    );
    eprintln!(
        "PPT persist directories: {persist_directory_files} files/{current_persist_references} current references/{superseded_persist_references} superseded references/{incremental_save_metadata_records} edit metadata records/{unreferenced_top_level_records} unreferenced top-level records; errors {persist_directory_errors:#?}"
    );
    eprintln!(
        "PPT live presentations: {live_presentation_files} files/{live_master_slides} masters/{live_presentation_slides} slides/{live_notes_slides} notes/{live_active_x_controls} ActiveX/{live_embedded_ole_objects} embedded OLE/{live_linked_ole_objects} linked OLE/{live_vba_projects} VBA/{live_persist_object_records} live persist top-level records/{dead_top_level_records} dead top-level records/{live_direct_persist_handles} direct handles/{live_list_text_bodies} list text bodies/{live_outline_text_references} outline-text links ({live_outline_text_shape_references} with shape)/{live_unresolved_outline_text_references} unresolved; slide relationships {live_slide_relationships}, master {live_slide_master_links:?}, notes {live_slide_notes_links:?}, compatible errors {live_slide_relationship_errors:#?}, strict errors {strict_live_slide_relationship_errors:#?}; strict outline link errors {live_outline_text_errors:#?}; errors {live_presentation_errors:#?}"
    );
    eprintln!(
        "PPT live OfficeArt graphs: {live_drawing_graph_files} files ({strict_live_drawing_graph_files} strict/{compatibility_live_drawing_graph_files} compatibility), {live_drawing_graph_drawings} drawings/{live_drawing_graph_shapes} shapes; BLIP {live_drawing_graph_blip_stores} stores/{live_drawing_graph_blip_entries} entries/{live_drawing_graph_blip_references} references, cRef {live_drawing_graph_blip_reference_count_relations:?}; issues {live_drawing_graph_issues:?}; errors {live_drawing_graph_errors:#?}"
    );
    eprintln!(
        "PPT compatible live presentations: {compatible_live_presentation_files} files/{compatible_live_presentation_diagnostics} diagnostics; errors {compatible_live_presentation_errors:#?}"
    );
}

fn ppt_live_signature(
    presentation: &PptLivePresentation<'_>,
) -> Vec<(u32, PptLivePersistObjectRole)> {
    let mut signature = vec![(
        presentation.document.reference.persist_id,
        presentation.document.role,
    )];
    signature.extend(
        presentation
            .notes_master_slide
            .iter()
            .chain(presentation.handout_master_slide.iter())
            .chain(&presentation.master_slides)
            .chain(&presentation.presentation_slides)
            .chain(&presentation.notes_slides)
            .chain(&presentation.active_x_controls)
            .chain(&presentation.embedded_ole_objects)
            .chain(&presentation.linked_ole_objects)
            .chain(presentation.vba_project.iter())
            .map(|object| (object.reference.persist_id, object.role)),
    );
    signature.sort_unstable();
    signature
}

fn try_ppt_text_body_edit(file: &mut PptFile) -> Result<bool, String> {
    if matches!(
        file.pictures.as_deref(),
        Some(PicturesStream::Compatibility { .. } | PicturesStream::Partial(_))
    ) {
        return Ok(false);
    }
    let candidate = {
        let presentation = match file.live_presentation() {
            Ok(value) => value,
            Err(_) => return Ok(false),
        };
        let slides = match presentation.slides() {
            Ok(value) => value,
            Err(_) => return Ok(false),
        };
        if slides
            .iter()
            .any(|slide| slide.object.outline_text_references().is_err())
        {
            return Ok(false);
        }
        slides.iter().find_map(|slide| {
            slide
                .object
                .text_bodies()
                .iter()
                .enumerate()
                .find_map(|(body_index, body)| {
                    body.records.iter().find_map(|record| match &record.data {
                        PptRecordData::TextChars(values) if !values.is_empty() => {
                            let replacement = if values.starts_with('X') {
                                u16::from(b'Y')
                            } else {
                                u16::from(b'X')
                            };
                            Some((slide.id(), body_index, true, replacement))
                        }
                        PptRecordData::TextBytes(values) if !values.is_empty() => {
                            let replacement = if values.starts_with('X') { b'Y' } else { b'X' };
                            Some((slide.id(), body_index, false, u16::from(replacement)))
                        }
                        _ => None,
                    })
                })
        })
    };
    let Some((slide_id, body_index, unicode, replacement)) = candidate else {
        return Ok(false);
    };

    let before_failed_edit = file.clone();
    let failed: olecfsdk::Result<()> =
        file.edit_slide_text_body(slide_id, body_index, |mut body| {
            replace_ppt_text_body_first_unit(&mut body, unicode, replacement)?;
            Err(olecfsdk::Error::invalid(
                0,
                "intentional PPT text transaction rollback",
            ))
        });
    if failed.is_ok() || file != &before_failed_edit {
        return Err("failed PPT text-body transaction changed the file root".to_owned());
    }

    file.edit_slide_text_body(slide_id, body_index, |mut body| {
        replace_ppt_text_body_first_unit(&mut body, unicode, replacement)
    })
    .map_err(|error| error.to_string())?;
    if ppt_text_body_first_unit(file, slide_id, body_index, unicode)? != replacement {
        return Err("PPT typed text-body edit did not update its static text atom".to_owned());
    }

    let bytes = file
        .to_bytes_preserving_compatibility()
        .map_err(|error| error.to_string())?;
    let reopened = PptFile::from_bytes_compatible(&bytes)
        .map_err(|error| error.to_string())?
        .value;
    if ppt_text_body_first_unit(&reopened, slide_id, body_index, unicode)? != replacement {
        return Err("PPT typed text-body edit did not survive save and reopen".to_owned());
    }
    Ok(true)
}

fn replace_ppt_text_body_first_unit(
    body: &mut olecfsdk::ppt::PptLiveTextBodyMut<'_>,
    unicode: bool,
    replacement: u16,
) -> olecfsdk::Result<()> {
    for record in body.records_mut() {
        match (&mut record.data, unicode) {
            (PptRecordData::TextChars(values), true) if !values.is_empty() => {
                replace_first_ppt_text_character(values, replacement)?;
                return Ok(());
            }
            (PptRecordData::TextBytes(values), false) if !values.is_empty() => {
                replace_first_ppt_text_character(values, replacement)?;
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

fn replace_first_ppt_text_character(values: &mut String, replacement: u16) -> olecfsdk::Result<()> {
    let replacement = char::from_u32(u32::from(replacement))
        .ok_or_else(|| olecfsdk::Error::invalid(0, "PPT replacement is not a Unicode scalar"))?;
    let end = values
        .char_indices()
        .nth(1)
        .map_or(values.len(), |(index, _)| index);
    values.replace_range(..end, &replacement.to_string());
    Ok(())
}

fn ppt_text_body_first_unit(
    file: &PptFile,
    slide_id: PptSlideId,
    body_index: usize,
    unicode: bool,
) -> Result<u16, String> {
    let presentation = file
        .live_presentation()
        .map_err(|error| error.to_string())?;
    let slides = presentation.slides().map_err(|error| error.to_string())?;
    let slide = slides
        .iter()
        .find(|slide| slide.id() == slide_id)
        .ok_or_else(|| "edited PPT slide ID is missing".to_owned())?;
    let bodies = slide.object.text_bodies();
    let body = bodies
        .get(body_index)
        .ok_or_else(|| "edited PPT text-body index is missing".to_owned())?;
    body.records
        .iter()
        .find_map(|record| match (&record.data, unicode) {
            (PptRecordData::TextChars(values), true)
            | (PptRecordData::TextBytes(values), false) => values
                .chars()
                .next()
                .and_then(|value| u16::try_from(value).ok()),
            _ => None,
        })
        .ok_or_else(|| "edited PPT text atom is missing".to_owned())
}

fn try_ppt_picture_growth(file: &PptFile) -> Result<bool, String> {
    let presentation = match file.live_presentation() {
        Ok(presentation) => presentation,
        Err(_) => return Ok(false),
    };
    let mut referenced_offsets = BTreeSet::new();
    for state in &presentation.top_level_records {
        if !matches!(
            state.status,
            PptTopLevelLiveRecordStatus::LivePersistObject { .. }
        ) {
            continue;
        }
        let Some(record) = file.document.records.records.get(state.record_index) else {
            return Err("PPT live top-level record index is out of bounds".to_owned());
        };
        collect_ppt_delay_offsets(record, &mut referenced_offsets);
    }
    let Some(PicturesStream::Complete(pictures)) = file.pictures.as_deref() else {
        return Ok(false);
    };

    let mut offsets = Vec::with_capacity(pictures.records.len());
    let mut offset = 0u32;
    for record in &pictures.records {
        offsets.push(offset);
        offset = offset
            .checked_add(record.header.declared_length)
            .and_then(|value| value.checked_add(8))
            .ok_or_else(|| "PPT Pictures Stream offset overflow".to_owned())?;
    }
    let Some((target_index, old_target_offset, grow_index)) = offsets
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, offset)| referenced_offsets.contains(offset))
        .find_map(|(target_index, old_target_offset)| {
            pictures.records[..target_index]
                .iter()
                .rposition(|record| matches!(record.data, OfficeArtRecordData::BitmapBlip(_)))
                .map(|grow_index| (target_index, old_target_offset, grow_index))
        })
    else {
        return Ok(false);
    };

    let mut edited = file.clone();
    let Some(pictures_stream) = &mut edited.pictures else {
        unreachable!("the cloned PPT has the same Pictures Stream variant")
    };
    let PicturesStream::Complete(pictures) = Arc::make_mut(pictures_stream) else {
        unreachable!("the cloned PPT has the same Pictures Stream variant")
    };
    let OfficeArtRecordData::BitmapBlip(blip) = &mut pictures.records[grow_index].data else {
        unreachable!("the selected Pictures Stream file block is a bitmap BLIP")
    };
    match &mut blip.file_data {
        OfficeArtBitmapData::Dib(bytes) | OfficeArtBitmapData::Encoded(bytes) => bytes.push(0),
    }
    if edited.relayout().is_err() {
        return Ok(false);
    }

    let Some(PicturesStream::Complete(pictures)) = edited.pictures.as_deref() else {
        unreachable!("PPT relayout preserves the Pictures Stream variant")
    };
    let new_target_offset = pictures.records[..target_index]
        .iter()
        .try_fold(0u32, |offset, record| {
            offset
                .checked_add(record.header.declared_length)
                .and_then(|value| value.checked_add(8))
        })
        .ok_or_else(|| "relayout PPT Pictures Stream offset overflow".to_owned())?;
    if new_target_offset != old_target_offset + 1 {
        return Err("PPT Pictures Stream edit did not move the referenced BLIP".to_owned());
    }

    let after = edited
        .live_presentation()
        .map_err(|error| error.to_string())?;
    let mut relocated_offsets = BTreeSet::new();
    for state in &after.top_level_records {
        if matches!(
            state.status,
            PptTopLevelLiveRecordStatus::LivePersistObject { .. }
        ) {
            let record = edited
                .document
                .records
                .records
                .get(state.record_index)
                .ok_or_else(|| "relayout PPT live record index is out of bounds".to_owned())?;
            collect_ppt_delay_offsets(record, &mut relocated_offsets);
        }
    }
    if !relocated_offsets.contains(&new_target_offset) {
        return Err("PPT live OfficeArtFBSE foDelay was not relocated".to_owned());
    }

    let saved = edited
        .to_bytes_preserving_compatibility()
        .map_err(|error| error.to_string())?;
    let reopened = PptFile::from_bytes_compatible(&saved)
        .map_err(|error| error.to_string())?
        .value;
    if reopened.document != edited.document
        || reopened.current_user != edited.current_user
        || reopened.pictures != edited.pictures
    {
        return Err("edited PPT picture tree changed after write and reopen".to_owned());
    }
    Ok(true)
}

fn try_ppt_append_user_edit(file: &PptFile) -> Result<bool, String> {
    let before = match file.live_presentation() {
        Ok(presentation) => presentation,
        Err(_) => return Ok(false),
    };
    let previous_edit_count = before
        .persist_object_directory
        .incremental_save_chain
        .edits
        .len();
    let previous_record_count = file.document.records.records.len();
    let mut edited = file.clone();
    let mut grew = false;
    for state in &before.top_level_records {
        if !matches!(
            state.status,
            PptTopLevelLiveRecordStatus::LivePersistObject { .. }
        ) {
            continue;
        }
        let record = Arc::make_mut(&mut edited.document)
            .records
            .records
            .get_mut(state.record_index)
            .ok_or_else(|| "PPT live record index is out of bounds".to_owned())?;
        if grow_ppt_record_cstring(record) {
            grew = true;
            break;
        }
    }
    if !grew {
        return Ok(false);
    }
    let report = match edited.append_user_edit() {
        Ok(report) => report,
        Err(_) => return Ok(false),
    };
    let after = edited
        .live_presentation()
        .map_err(|error| error.to_string())?;
    if ppt_live_signature(&after) != ppt_live_signature(&before) {
        return Err("appended PPT user edit changed the live persist-object roles".to_owned());
    }
    if after
        .persist_object_directory
        .incremental_save_chain
        .edits
        .len()
        != previous_edit_count + 1
    {
        return Err("PPT append-user-edit did not extend the edit chain once".to_owned());
    }
    if edited.document.records.records.len()
        != previous_record_count + report.appended_persist_records + 2
    {
        return Err("PPT append-user-edit record count is inconsistent".to_owned());
    }
    if report.persist_ids.is_empty() {
        return Err("PPT append-user-edit wrote an empty persist directory".to_owned());
    }
    let saved = edited.to_bytes().map_err(|error| error.to_string())?;
    let reopened = PptFile::from_bytes(&saved).map_err(|error| error.to_string())?;
    if reopened.document != edited.document
        || reopened.current_user != edited.current_user
        || reopened.pictures != edited.pictures
    {
        return Err("appended PPT user-edit tree changed after write and reopen".to_owned());
    }
    Ok(true)
}

fn collect_ppt_delay_offsets(record: &olecfsdk::ppt::PptRecord, offsets: &mut BTreeSet<u32>) {
    match &record.data {
        PptRecordData::Container(children) | PptRecordData::ProgTags(children) => {
            for child in &children.records {
                collect_ppt_delay_offsets(child, offsets);
            }
        }
        PptRecordData::ProgBinaryTag(value) => {
            for child in &value.records.records {
                collect_ppt_delay_offsets(child, offsets);
            }
        }
        PptRecordData::BinaryTagData(BinaryTagData::Records(children)) => {
            for child in &children.records {
                collect_ppt_delay_offsets(child, offsets);
            }
        }
        PptRecordData::OfficeArt(value) => collect_office_art_delay_offsets(value, offsets),
        PptRecordData::BlipEntity9(value) => {
            collect_office_art_delay_offsets(&value.blip, offsets);
        }
        _ => {}
    }
}

fn collect_office_art_delay_offsets(record: &OfficeArtRecord, offsets: &mut BTreeSet<u32>) {
    match &record.data {
        OfficeArtRecordData::Container(children)
        | OfficeArtRecordData::CompatibilityContainer(children) => {
            for child in children {
                collect_office_art_delay_offsets(child, offsets);
            }
        }
        OfficeArtRecordData::Fbse(fbse)
            if fbse.reference_count != 0
                && fbse.embedded_blip.is_none()
                && fbse.delay_offset != u32::MAX =>
        {
            offsets.insert(fbse.delay_offset);
        }
        OfficeArtRecordData::Fbse(fbse) => {
            if let Some(embedded) = &fbse.embedded_blip {
                collect_office_art_delay_offsets(embedded, offsets);
            }
        }
        _ => {}
    }
}

fn try_ppt_cstring_growth(file: &mut PptFile) -> Result<bool, String> {
    let CurrentUserData::Parsed(current_user) = &file.current_user.data else {
        return Ok(false);
    };
    let old_current_edit = current_user.offset_to_current_edit;
    let mut candidate = file.clone();
    if !grow_first_ppt_cstring(&mut Arc::make_mut(&mut candidate.document).records) {
        return Ok(false);
    }
    if candidate.relayout().is_err() {
        return Ok(false);
    }
    let CurrentUserData::Parsed(current_user) = &candidate.current_user.data else {
        unreachable!("PPT relayout requires a parsed CurrentUserAtom")
    };
    if current_user.offset_to_current_edit == old_current_edit {
        return Ok(false);
    }
    *file = candidate;
    Ok(true)
}

#[allow(clippy::collapsible_match)] // Mutable recursion is not legal in match guards.
fn grow_first_ppt_cstring(records: &mut PptRecordSequence) -> bool {
    for record in &mut records.records {
        if grow_ppt_record_cstring(record) {
            return true;
        }
    }
    false
}

#[allow(clippy::collapsible_match)] // Mutable recursion is not legal in match guards.
fn grow_ppt_record_cstring(record: &mut olecfsdk::ppt::PptRecord) -> bool {
    match &mut record.data {
        PptRecordData::CString(values) if !values.starts_with("___PPT") => {
            values.push('X');
            true
        }
        PptRecordData::Container(children) | PptRecordData::ProgTags(children) => {
            grow_first_ppt_cstring(children)
        }
        PptRecordData::ProgBinaryTag(value) => grow_first_ppt_cstring(&mut value.records),
        PptRecordData::BinaryTagData(BinaryTagData::Records(children)) => {
            grow_first_ppt_cstring(children)
        }
        _ => false,
    }
}

#[test]
#[ignore = "classic Office file-root corpus round-trip runs explicitly"]
fn xls_files_round_trip_through_typed_root() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let exclusions = exclusions_for(&corpus, &["xls_biff_roundtrip"], &["xls", "xlt"]);
    let files = corpus_files(&corpus, &["xls", "xlt"]);
    let mut opened = 0usize;
    let mut reopened = 0usize;
    let mut rejected = BTreeMap::<String, usize>::new();
    let mut observed_exclusions = BTreeSet::new();
    let mut failures = Vec::new();
    let mut relocated_sheet_layout = false;
    let mut edited_cell_value = false;
    let mut reordered_sheets = false;
    let mut unresolved_sheet_links = 0usize;
    let mut unlinked_sheet_substreams = 0usize;
    let mut unlinked_biff8_substreams = 0usize;
    let mut unlinked_biff8_details = Vec::new();
    let mut typed_cells = 0usize;
    let mut typed_rows = 0usize;
    let mut duplicate_cell_coordinates = 0usize;
    let mut duplicate_row_coordinates = 0usize;
    let mut typed_formulas = 0usize;
    let mut shared_formulas = 0usize;
    let mut unresolved_formula_expressions = 0usize;
    let mut unresolved_exp_formulas = 0usize;
    let mut unresolved_table_formulas = 0usize;
    let mut merged_cells = 0usize;
    let mut drawing_groups = 0usize;
    let mut sheet_drawings = 0usize;
    let mut drawing_host_objects = 0usize;
    let mut sheet_objects = 0usize;
    let mut control_stream_objects = 0usize;
    let mut embedding_storage_objects = 0usize;
    let mut link_storage_objects = 0usize;
    let mut dde_data_items = 0usize;
    let mut object_relationship_errors = BTreeMap::<String, usize>::new();
    let mut cell_relationship_errors = BTreeMap::<String, usize>::new();
    let mut typed_file_entries = 0usize;
    let mut file_entry_issues = BTreeMap::<String, usize>::new();
    let mut pivot_cache_streams = 0usize;
    let mut parsed_pivot_cache_streams = 0usize;
    let mut compatible_pivot_cache_streams = 0usize;
    let mut pivot_cache_records = 0usize;
    let mut pivot_cache_compatibility = BTreeMap::<String, usize>::new();
    let mut pivot_cache_definitions = 0usize;
    let mut pivot_table_views = 0usize;
    let mut resolved_pivot_table_cache_streams = 0usize;
    let mut unresolved_pivot_table_cache_links = 0usize;
    let mut pivot_relationship_errors = BTreeMap::<String, usize>::new();
    let mut revision_log_streams = 0usize;
    let mut revision_log_records = 0usize;
    let mut compatible_revision_logs = 0usize;
    let mut nonconforming_revision_logs = 0usize;
    let mut revision_log_compatibility = BTreeMap::<String, usize>::new();
    let mut revision_logs = 0usize;
    let mut revision_graph_logs = 0usize;
    let mut revision_graph_nodes = 0usize;
    let mut revision_graph_resolved_sheets = 0usize;
    let mut revision_graph_without_sheet = 0usize;
    let mut revision_graph_unresolved_sheets = 0usize;
    let mut unlinked_revision_records = 0usize;
    let mut revision_productions = BTreeMap::<&'static str, usize>::new();
    let mut unlinked_revision_production_records = 0usize;
    let mut revision_delete_productions = 0usize;
    let mut incomplete_revision_productions = 0usize;
    let mut nested_revision_change_cells = 0usize;
    let mut nested_revision_formats = 0usize;
    let mut revision_font_reset_records = 0usize;
    let mut revision_sheet_links = 0usize;
    let mut revision_without_sheet = 0usize;
    let mut revision_local_sheet_links = 0usize;
    let mut revision_custom_view_links = 0usize;
    let mut unresolved_revision_custom_view_links = 0usize;
    let mut revision_relationship_errors = BTreeMap::<String, usize>::new();
    let mut custom_views = 0usize;
    let mut custom_sheet_views = 0usize;
    let mut chart_custom_sheet_views = 0usize;
    let mut unlinked_custom_sheet_views = 0usize;
    let mut unlinked_custom_view_records = 0usize;
    let mut custom_view_active_sheet_links = 0usize;
    let mut custom_views_without_active_sheet = 0usize;
    let mut unresolved_custom_view_active_sheet_links = 0usize;
    let mut custom_view_compatibility = BTreeMap::<String, usize>::new();
    let mut custom_view_defined_names = 0usize;
    let mut custom_view_defined_name_kinds = BTreeMap::<String, usize>::new();
    let mut user_names_streams = 0usize;
    let mut user_names_records = 0usize;
    let mut compatible_user_names = 0usize;
    let mut nonconforming_user_names = 0usize;
    let mut user_names_compatibility = BTreeMap::<String, usize>::new();
    let mut shared_workbook_users = 0usize;
    let mut unresolved_user_revision_links = 0usize;

    for path in &files {
        if exclusions.contains_key(path) {
            observed_exclusions.insert(path.clone());
            continue;
        }
        let mut parsed_root = false;
        let result = (|| {
            let bytes = corpus_bytes(path).map_err(|error| error.to_string())?;
            let mut file = XlsFile::from_bytes_compatible(&bytes)
                .map_err(|error| error.to_string())?
                .value;
            parsed_root = true;
            opened += 1;
            let storage_inventory = file.storages_and_streams_compatible();
            typed_file_entries += storage_inventory
                .entries()
                .iter()
                .filter(|entry| entry.role() != XlsFileEntryRole::Other)
                .count();
            for issue in storage_inventory.issues() {
                *file_entry_issues.entry(format!("{issue:?}")).or_default() += 1;
            }
            pivot_cache_streams += file.pivot_caches.len();
            for cache in file.pivot_caches.iter() {
                match cache {
                    XlsPivotCache::Parsed { stream, .. } => {
                        parsed_pivot_cache_streams += 1;
                        pivot_cache_records += stream.records.len();
                    }
                    XlsPivotCache::Compatibility {
                        stream_id, reason, ..
                    } => {
                        compatible_pivot_cache_streams += 1;
                        *pivot_cache_compatibility
                            .entry(format!("{} #{stream_id:04X}: {reason}", path.display()))
                            .or_default() += 1;
                    }
                }
            }
            if let Some(revision_log) = file.revision_log.as_deref() {
                revision_log_streams += 1;
                match revision_log {
                    XlsRevisionLog::Parsed(stream) => {
                        revision_log_records += stream.records.len();
                        nonconforming_revision_logs += usize::from(stream.validate().is_err());
                    }
                    XlsRevisionLog::Compatibility { reason, .. } => {
                        compatible_revision_logs += 1;
                        *revision_log_compatibility
                            .entry(format!("{}: {reason}", path.display()))
                            .or_default() += 1;
                    }
                }
            }
            if let Some(user_names) = file.user_names.as_deref() {
                user_names_streams += 1;
                match user_names {
                    XlsUserNames::Parsed(stream) => {
                        user_names_records += stream.records.len();
                        nonconforming_user_names += usize::from(stream.validate().is_err());
                    }
                    XlsUserNames::Compatibility { reason, .. } => {
                        compatible_user_names += 1;
                        *user_names_compatibility
                            .entry(format!("{}: {reason}", path.display()))
                            .or_default() += 1;
                    }
                }
            }
            let revision_workbook = file
                .workbook_view_compatible(XlsStreamName::Workbook)
                .map_err(|error| format!("revision workbook relationship: {error}"))?;
            let revision_relationships = match file.revision_stream_view_compatible() {
                Ok(Some(view)) => {
                    revision_logs += view.revision_logs().len();
                    unlinked_revision_records += view.unlinked_records().len();
                    for log in view.revision_logs() {
                        let productions = log.revision_records_compatible();
                        if let Err(error) = log.revision_records() {
                            *revision_relationship_errors
                                .entry(format!("{} revision production: {error}", path.display()))
                                .or_default() += 1;
                        }
                        unlinked_revision_production_records +=
                            productions.unlinked_records().len();
                        for production in productions.revisions() {
                            if let Some(workbook) = &revision_workbook {
                                match production.resolve_sheet(*log, workbook) {
                                    Ok(Some(_)) => revision_sheet_links += 1,
                                    Ok(None) => revision_without_sheet += 1,
                                    Err(error) => {
                                        *revision_relationship_errors
                                            .entry(format!(
                                                "{} revision sheet: {error}",
                                                path.display()
                                            ))
                                            .or_default() += 1;
                                    }
                                }
                            }
                            let kind = match production {
                                XlsRevisionRecordRef::RenameSheet { .. } => "rename-sheet",
                                XlsRevisionRecordRef::InsertDelete(value) => {
                                    revision_delete_productions += usize::from(value.is_delete());
                                    incomplete_revision_productions += usize::from(
                                        value.is_delete() && value.end_record().is_none(),
                                    );
                                    for change in value.changes() {
                                        if let Some(workbook) = &revision_workbook {
                                            match change.resolve_sheet(*log, workbook) {
                                                Ok(Some(_)) => revision_sheet_links += 1,
                                                Ok(None) => revision_without_sheet += 1,
                                                Err(error) => {
                                                    *revision_relationship_errors
                                                        .entry(format!(
                                                            "{} nested revision sheet: {error}",
                                                            path.display()
                                                        ))
                                                        .or_default() += 1;
                                                }
                                            }
                                        }
                                        match change {
                                            XlsRevisionCellOrFormatRef::ChangeCell(value) => {
                                                nested_revision_change_cells += 1;
                                                revision_font_reset_records +=
                                                    value.font_reset_records().len();
                                            }
                                            XlsRevisionCellOrFormatRef::Format { .. } => {
                                                nested_revision_formats += 1;
                                            }
                                        }
                                    }
                                    "insert-delete"
                                }
                                XlsRevisionRecordRef::Conflict { .. } => "conflict",
                                XlsRevisionRecordRef::InsertSheet { .. } => "insert-sheet",
                                XlsRevisionRecordRef::ChangeCell(value) => {
                                    revision_font_reset_records += value.font_reset_records().len();
                                    "change-cell"
                                }
                                XlsRevisionRecordRef::Move(value) => {
                                    incomplete_revision_productions +=
                                        usize::from(value.end_record().is_none());
                                    for change in value.changes() {
                                        if let Some(workbook) = &revision_workbook {
                                            match change.resolve_sheet(*log, workbook) {
                                                Ok(Some(_)) => revision_sheet_links += 1,
                                                Ok(None) => revision_without_sheet += 1,
                                                Err(error) => {
                                                    *revision_relationship_errors
                                                        .entry(format!(
                                                            "{} nested revision sheet: {error}",
                                                            path.display()
                                                        ))
                                                        .or_default() += 1;
                                                }
                                            }
                                        }
                                        match change {
                                            XlsRevisionCellOrFormatRef::ChangeCell(value) => {
                                                nested_revision_change_cells += 1;
                                                revision_font_reset_records +=
                                                    value.font_reset_records().len();
                                            }
                                            XlsRevisionCellOrFormatRef::Format { .. } => {
                                                nested_revision_formats += 1;
                                            }
                                        }
                                    }
                                    "move"
                                }
                                XlsRevisionRecordRef::Format { .. } => "format",
                                XlsRevisionRecordRef::AutoFormat { .. } => "auto-format",
                                XlsRevisionRecordRef::DefinedName { .. } => {
                                    if let Some(workbook) = &revision_workbook {
                                        match production
                                            .resolve_defined_name_local_sheet(*log, workbook)
                                        {
                                            Ok(Some(_)) => revision_local_sheet_links += 1,
                                            Ok(None) => {}
                                            Err(error) => {
                                                *revision_relationship_errors
                                                    .entry(format!(
                                                        "{} revision defined-name sheet: {error}",
                                                        path.display()
                                                    ))
                                                    .or_default() += 1;
                                            }
                                        }
                                    }
                                    "defined-name"
                                }
                                XlsRevisionRecordRef::UserView { .. } => {
                                    if let Some(workbook) = &revision_workbook {
                                        match production.resolve_custom_view_compatible(workbook) {
                                            Some(XlsCustomViewLink::Resolved(_)) => {
                                                revision_custom_view_links += 1;
                                            }
                                            Some(
                                                XlsCustomViewLink::Missing { .. }
                                                | XlsCustomViewLink::Ambiguous { .. },
                                            ) => unresolved_revision_custom_view_links += 1,
                                            None => unreachable!(
                                                "UserView production has a custom-view link"
                                            ),
                                        }
                                    }
                                    "user-view"
                                }
                                XlsRevisionRecordRef::Note { .. } => "note",
                                XlsRevisionRecordRef::TrashQueryTableField { .. } => {
                                    "trash-query-table-field"
                                }
                            };
                            *revision_productions.entry(kind).or_default() += 1;
                        }
                    }
                    Some(view)
                }
                Ok(None) => None,
                Err(error) => {
                    *revision_relationship_errors
                        .entry(format!("{}: {error}", path.display()))
                        .or_default() += 1;
                    None
                }
            };
            if let Some(workbook) = &revision_workbook
                && let Some(graph) = file
                    .revision_graph_compatible(workbook)
                    .map_err(|error| format!("revision root graph: {error}"))?
            {
                revision_graph_logs += graph.logs().len();
                for log in graph.logs() {
                    assert!(log.unlinked_records().is_empty());
                    revision_graph_nodes += log.revisions().len();
                    for revision in log.revisions() {
                        match revision.sheet() {
                            XlsRevisionSheetLink::Resolved(_) => {
                                revision_graph_resolved_sheets += 1
                            }
                            XlsRevisionSheetLink::NotSpecified => revision_graph_without_sheet += 1,
                            XlsRevisionSheetLink::Unresolved { .. } => {
                                revision_graph_unresolved_sheets += 1
                            }
                        }
                    }
                }
            }
            match file.user_log_view_compatible() {
                Ok(Some(users)) => {
                    shared_workbook_users += users.users().len();
                    if let Some(revisions) = &revision_relationships {
                        for user in users.users() {
                            if !matches!(
                                users.resolve_revision_log_compatible(*user, revisions),
                                XlsUserRevisionLogLink::Resolved(_)
                            ) {
                                unresolved_user_revision_links += 1;
                            }
                        }
                    } else {
                        unresolved_user_revision_links += users.users().len();
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    *revision_relationship_errors
                        .entry(format!("{} User Names: {error}", path.display()))
                        .or_default() += 1;
                }
            }
            for workbook in file.workbooks.iter() {
                let relationships = workbook.relationships_compatible().map_err(|error| {
                    format!(
                        "{} relationship tree is not closed: {error}",
                        workbook.name.path()
                    )
                })?;
                unresolved_sheet_links += relationships.unresolved_sheets().len();
                unlinked_sheet_substreams += relationships.unlinked_substreams().len();
                custom_views += relationships.custom_views().len();
                custom_sheet_views += relationships
                    .custom_views()
                    .iter()
                    .map(|view| view.sheet_views().len())
                    .sum::<usize>();
                chart_custom_sheet_views += relationships
                    .custom_views()
                    .iter()
                    .flat_map(|view| view.sheet_views())
                    .filter(|view| view.begin().is_chart())
                    .count();
                for custom_view in relationships.custom_views() {
                    custom_view_defined_names += custom_view.defined_names().len();
                    for name in custom_view.defined_names() {
                        *custom_view_defined_name_kinds
                            .entry(format!("{:?}", name.kind()))
                            .or_default() += 1;
                    }
                    match relationships.resolve_custom_view_active_sheet_compatible(custom_view) {
                        XlsCustomViewActiveSheetLink::Resolved(_) => {
                            custom_view_active_sheet_links += 1;
                        }
                        XlsCustomViewActiveSheetLink::NotSpecified => {
                            custom_views_without_active_sheet += 1;
                        }
                        XlsCustomViewActiveSheetLink::Missing { sheet_identifier } => {
                            unresolved_custom_view_active_sheet_links += 1;
                            *custom_view_compatibility
                                .entry(format!(
                                    "{}: missing active sheet {sheet_identifier}",
                                    path.display()
                                ))
                                .or_default() += 1;
                        }
                        XlsCustomViewActiveSheetLink::Ambiguous { sheet_identifier } => {
                            unresolved_custom_view_active_sheet_links += 1;
                            *custom_view_compatibility
                                .entry(format!(
                                    "{}: ambiguous active sheet {sheet_identifier}",
                                    path.display()
                                ))
                                .or_default() += 1;
                        }
                    }
                }
                unlinked_custom_sheet_views += relationships.unlinked_custom_sheet_views().len();
                unlinked_custom_view_records += relationships.unlinked_custom_view_records().len();
                pivot_cache_definitions += relationships.pivot_cache_definitions().len();
                for drawing_group in relationships.drawing_groups() {
                    drawing_groups += 1;
                    assert!(matches!(
                        drawing_group.source_record().data,
                        BiffRecordData::MsoDrawingGroup(_)
                    ));
                }
                for definition in relationships.pivot_cache_definitions() {
                    if let Err(error) = file.resolve_pivot_cache(*definition) {
                        *pivot_relationship_errors
                            .entry(format!("{}: {error}", path.display()))
                            .or_default() += 1;
                    }
                }
                if workbook.tree.stream.is_biff8() {
                    unlinked_biff8_substreams += relationships.unlinked_substreams().len();
                    if !relationships.unlinked_substreams().is_empty() {
                        unlinked_biff8_details.push(format!(
                            "{} {}: {:?}",
                            path.display(),
                            workbook.name.path(),
                            relationships
                                .unlinked_substreams()
                                .iter()
                                .map(|node| (node.kind, node.record_range.clone()))
                                .collect::<Vec<_>>()
                        ));
                    }
                }
                for sheet in relationships.sheets().iter().copied() {
                    merged_cells += sheet.merged_cells().count();
                    for drawing in sheet.drawings() {
                        sheet_drawings += 1;
                        assert_eq!(drawing.sheet().id(), sheet.id());
                        assert!(matches!(
                            drawing.source_record().data,
                            BiffRecordData::MsoDrawing(_)
                        ));
                        drawing_host_objects += drawing.objects().count();
                    }
                    for object in sheet.objects() {
                        sheet_objects += 1;
                        match storage_inventory
                            .resolve_object_persistence_compatible(&relationships, object)
                        {
                            Ok(Some(XlsObjectPersistenceRef::ControlStream { .. })) => {
                                control_stream_objects += 1;
                            }
                            Ok(Some(XlsObjectPersistenceRef::EmbeddingStorage { .. })) => {
                                embedding_storage_objects += 1;
                            }
                            Ok(Some(XlsObjectPersistenceRef::LinkStorage { .. })) => {
                                link_storage_objects += 1;
                            }
                            Ok(Some(XlsObjectPersistenceRef::DdeDataItem { .. })) => {
                                dde_data_items += 1;
                            }
                            Ok(None) => {}
                            Err(error) => {
                                *object_relationship_errors
                                    .entry(format!("{}: {error}", path.display()))
                                    .or_default() += 1;
                            }
                        }
                    }
                    for pivot_view in sheet.pivot_table_views() {
                        pivot_table_views += 1;
                        match file.resolve_pivot_table_compatible(&relationships, pivot_view) {
                            XlsPivotTableLink::Resolved(pivot_table) => {
                                if pivot_table.cache_stream().is_some() {
                                    resolved_pivot_table_cache_streams += 1;
                                } else {
                                    *pivot_relationship_errors
                                        .entry(format!(
                                            "{}: PivotTable resolved to an opaque cache stream",
                                            path.display()
                                        ))
                                        .or_default() += 1;
                                }
                                assert_eq!(pivot_table.sheet().id(), sheet.id());
                                assert_eq!(
                                    pivot_table.definition().stream_id(),
                                    pivot_table.cache().stream_id()
                                );
                            }
                            XlsPivotTableLink::Unresolved { .. } => {
                                unresolved_pivot_table_cache_links += 1;
                            }
                        }
                    }
                    let sparse_cells = match sheet.sparse_cell_index_compatible() {
                        Ok(value) => value,
                        Err(error) => {
                            *cell_relationship_errors
                                .entry(error.to_string())
                                .or_default() += 1;
                            continue;
                        }
                    };
                    typed_rows += sparse_cells.rows().count();
                    duplicate_cell_coordinates += sparse_cells.duplicate_cells().count();
                    duplicate_row_coordinates += sparse_cells
                        .rows()
                        .filter(|row| row.definitions().len() > 1)
                        .count();
                    for cell in sparse_cells.rows().flat_map(|row| row.cells()) {
                        typed_cells += 1;
                        if let Err(error) = relationships.resolve_cell_shared_string(cell) {
                            *cell_relationship_errors
                                .entry(error.to_string())
                                .or_default() += 1;
                        }
                        if let Err(error) = relationships.resolve_cell_format_ref_compatible(cell) {
                            *cell_relationship_errors
                                .entry(error.to_string())
                                .or_default() += 1;
                        }
                        match sparse_cells.resolve_cell_formula_compatible(cell) {
                            Ok(Some(formula)) => {
                                typed_formulas += 1;
                                match formula.definition() {
                                    XlsFormulaDefinitionRef::Shared(_) => shared_formulas += 1,
                                    XlsFormulaDefinitionRef::UnresolvedExp { .. } => {
                                        unresolved_formula_expressions += 1;
                                        unresolved_exp_formulas += 1;
                                    }
                                    XlsFormulaDefinitionRef::UnresolvedTable { .. } => {
                                        unresolved_formula_expressions += 1;
                                        unresolved_table_formulas += 1;
                                    }
                                    _ => {}
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                *cell_relationship_errors
                                    .entry(error.to_string())
                                    .or_default() += 1;
                            }
                        }
                    }
                }
            }
            let edited_sheet_layout =
                !relocated_sheet_layout && try_xls_sheet_name_growth(&mut file)?;
            if edited_sheet_layout {
                relocated_sheet_layout = true;
            }
            let edited_cell = !edited_cell_value && try_xls_number_cell_edit(&mut file)?;
            if edited_cell {
                edited_cell_value = true;
            }
            let reordered_this_file = !reordered_sheets && try_xls_sheet_reorder(&mut file)?;
            if reordered_this_file {
                reordered_sheets = true;
            }
            let saved = file
                .to_bytes_preserving_compatibility()
                .map_err(|error| error.to_string())?;
            let round_tripped = XlsFile::from_bytes_compatible(&saved)
                .map_err(|error| error.to_string())?
                .value;
            if round_tripped.workbooks != file.workbooks
                || round_tripped.pivot_caches != file.pivot_caches
                || round_tripped.revision_log != file.revision_log
                || round_tripped.user_names != file.user_names
            {
                return Err("managed XLS Rust tree changed after write and reopen".to_owned());
            }
            if !edited_sheet_layout
                && !edited_cell
                && !reordered_this_file
                && !round_tripped
                    .source_compound_file()
                    .logical_eq(file.source_compound_file())
            {
                let original = file.source_compound_file();
                let reopened = round_tripped.source_compound_file();
                let differing_entries = original
                    .entries()
                    .iter()
                    .zip(reopened.entries())
                    .filter(|(before, after)| before != after)
                    .map(|(before, after)| {
                        let byte_differences = before
                            .data
                            .iter()
                            .zip(after.data.iter())
                            .enumerate()
                            .filter_map(|(offset, (before, after))| {
                                (before != after)
                                    .then_some(format!("0x{offset:x}:{before:02x}->{after:02x}"))
                            })
                            .take(16)
                            .collect::<Vec<_>>();
                        format!(
                            "{} ({} -> {} bytes, {byte_differences:?})",
                            before.path.display(),
                            before.data.len(),
                            after.data.len()
                        )
                    })
                    .collect::<Vec<_>>();
                return Err(format!(
                    "XLS compound-file object tree changed after write and reopen; entries {} -> {}, differing {differing_entries:?}",
                    original.entries().len(),
                    reopened.entries().len()
                ));
            }
            reopened += 1;
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            if parsed_root {
                failures.push(format!("{}: {error}", path.display()));
            } else {
                *rejected.entry(error).or_default() += 1;
            }
        }
    }

    assert_eq!(observed_exclusions, exclusions.keys().cloned().collect());
    assert!(
        failures.is_empty(),
        "XLS file-root failures:\n{}",
        failures.join("\n")
    );
    assert!(opened > 0, "no XLS files opened through XlsFile");
    assert!(
        relocated_sheet_layout,
        "no variable-length BoundSheet8 name was relaid out through XlsFile"
    );
    assert!(
        edited_cell_value,
        "no XLS Number cell was edited through the root transaction API"
    );
    assert!(
        reordered_sheets,
        "no XLS workbook was reordered through stable sheet identities"
    );
    assert_eq!(reopened, opened, "not every opened XLS file was reopened");
    assert_eq!(
        unresolved_sheet_links, 1,
        "XLS unresolved BoundSheet8 relationship inventory changed"
    );
    assert_eq!(
        unlinked_sheet_substreams, 69,
        "XLS compatible unlinked substream inventory changed"
    );
    assert_eq!(
        unlinked_biff8_substreams,
        34,
        "BIFF8 compatible unlinked substream inventory changed:\n{}",
        unlinked_biff8_details.join("\n")
    );
    assert_eq!(typed_rows, 506_466, "XLS sparse row inventory changed");
    assert_eq!(
        duplicate_row_coordinates, 0,
        "XLS duplicate Row-coordinate inventory changed"
    );
    assert_eq!(typed_cells, 2_379_566, "XLS logical cell inventory changed");
    assert_eq!(
        duplicate_cell_coordinates, 5_579,
        "XLS duplicate cell-coordinate inventory changed"
    );
    assert_eq!(typed_formulas, 98_651, "XLS formula inventory changed");
    assert_eq!(
        shared_formulas, 49_079,
        "XLS resolved shared-formula inventory changed"
    );
    assert_eq!(
        unresolved_formula_expressions, 12,
        "XLS compatible unresolved formula inventory changed"
    );
    assert_eq!(
        unresolved_exp_formulas, 12,
        "XLS compatible unresolved PtgExp inventory changed"
    );
    assert_eq!(
        unresolved_table_formulas, 0,
        "XLS compatible unresolved PtgTbl inventory changed"
    );
    assert_eq!(merged_cells, 12_469, "XLS merged-cell inventory changed");
    assert_eq!(drawing_groups, 345, "XLS drawing-group inventory changed");
    assert_eq!(sheet_drawings, 603, "XLS sheet drawing inventory changed");
    assert!(
        drawing_host_objects > 0,
        "XLS drawing handles did not expose any hosted Obj records"
    );
    assert_eq!(sheet_objects, 3_511, "XLS sheet Obj inventory changed");
    assert_eq!(
        control_stream_objects, 153,
        "XLS Ctls-backed Obj inventory changed"
    );
    assert_eq!(
        embedding_storage_objects, 16,
        "XLS MBD-backed Obj inventory changed"
    );
    assert_eq!(
        link_storage_objects, 0,
        "XLS LNK-backed Obj inventory changed"
    );
    assert_eq!(dde_data_items, 0, "XLS DDE Obj inventory changed");
    assert!(
        object_relationship_errors.is_empty(),
        "XLS typed object relationships failed: {object_relationship_errors:#?}"
    );
    assert!(
        cell_relationship_errors.is_empty(),
        "XLS typed cell relationships failed: {cell_relationship_errors:#?}"
    );
    assert_eq!(
        typed_file_entries, 2_986,
        "XLS typed storage/stream inventory changed"
    );
    assert_eq!(
        pivot_cache_streams, 41,
        "XLS PivotCache stream inventory changed"
    );
    assert_eq!(
        parsed_pivot_cache_streams, 38,
        "XLS parsed PivotCache inventory changed"
    );
    assert_eq!(
        compatible_pivot_cache_streams, 3,
        "XLS compatible PivotCache inventory changed"
    );
    assert_eq!(
        pivot_cache_records, 38_667,
        "XLS PivotCache record inventory changed"
    );
    assert_eq!(
        pivot_cache_definitions, 22,
        "XLS PivotCache definition inventory changed"
    );
    assert_eq!(
        pivot_table_views, 24,
        "XLS PivotTable view inventory changed"
    );
    assert_eq!(
        resolved_pivot_table_cache_streams, 23,
        "XLS fully resolved PivotTable cache-stream inventory changed"
    );
    assert_eq!(
        unresolved_pivot_table_cache_links, 1,
        "XLS compatible unresolved PivotTable cache-link inventory changed"
    );
    assert!(
        pivot_relationship_errors.is_empty(),
        "XLS PivotCache relationships failed: {pivot_relationship_errors:#?}"
    );
    assert_eq!(
        revision_log_streams, 8,
        "XLS Revision Stream inventory changed"
    );
    assert_eq!(
        revision_log_records, 70,
        "XLS Revision Stream record inventory changed"
    );
    assert_eq!(
        compatible_revision_logs, 0,
        "XLS opaque Revision Stream inventory changed"
    );
    assert_eq!(
        nonconforming_revision_logs, 3,
        "XLS nonconforming typed Revision Stream inventory changed"
    );
    assert_eq!(revision_logs, 8, "XLS revision-log inventory changed");
    assert_eq!(
        revision_graph_logs, 8,
        "XLS root revision-graph log inventory changed"
    );
    assert_eq!(
        revision_graph_nodes, 14,
        "XLS root revision-graph production inventory changed"
    );
    assert_eq!(
        revision_graph_unresolved_sheets, 0,
        "XLS root revision graph has unresolved sheet relationships"
    );
    assert_eq!(
        revision_graph_resolved_sheets + revision_graph_without_sheet,
        revision_graph_nodes,
        "XLS root revision graph did not classify every sheet relationship"
    );
    assert_eq!(
        unlinked_revision_records, 0,
        "XLS unlinked revision-record inventory changed"
    );
    assert_eq!(
        revision_productions,
        BTreeMap::from([("change-cell", 2), ("insert-delete", 12)]),
        "XLS revision-production inventory changed"
    );
    assert_eq!(
        unlinked_revision_production_records, 0,
        "XLS unlinked revision-production record inventory changed"
    );
    assert_eq!(
        revision_delete_productions, 0,
        "XLS revision-delete inventory changed"
    );
    assert_eq!(
        incomplete_revision_productions, 0,
        "XLS incomplete revision-production inventory changed"
    );
    assert_eq!(
        nested_revision_change_cells, 12,
        "XLS nested revision-cell inventory changed"
    );
    assert_eq!(
        nested_revision_formats, 0,
        "XLS nested revision-format inventory changed"
    );
    assert_eq!(
        revision_font_reset_records, 0,
        "XLS revision font-reset inventory changed"
    );
    assert_eq!(
        revision_sheet_links + revision_without_sheet,
        revision_productions.values().sum::<usize>() + nested_revision_change_cells,
        "XLS revision sheet relationships were not traversed completely"
    );
    assert_eq!(
        unresolved_revision_custom_view_links, 0,
        "XLS revision custom-view relationships are unresolved"
    );
    assert_eq!(custom_views, 26, "XLS custom-view inventory changed");
    assert_eq!(
        custom_sheet_views, 43,
        "XLS custom sheet-view inventory changed"
    );
    assert_eq!(
        chart_custom_sheet_views, 0,
        "XLS chart custom sheet-view inventory changed"
    );
    assert_eq!(
        unlinked_custom_sheet_views, 0,
        "XLS UserSViewBegin records are not linked to UserBView"
    );
    assert_eq!(
        unlinked_custom_view_records, 0,
        "XLS CUSTOMVIEW delimiters are not closed"
    );
    assert_eq!(
        unresolved_custom_view_active_sheet_links, 12,
        "XLS compatible custom-view active-sheet inventory changed"
    );
    assert_eq!(
        custom_view_active_sheet_links
            + custom_views_without_active_sheet
            + unresolved_custom_view_active_sheet_links,
        custom_views,
        "XLS custom-view active-sheet relationships were not traversed completely"
    );
    assert_eq!(
        custom_view_defined_name_kinds,
        BTreeMap::from([
            ("FilterData".to_string(), 12),
            ("HiddenColumns".to_string(), 2),
            ("PrintArea".to_string(), 3),
            ("PrintTitles".to_string(), 9),
        ]),
        "XLS custom-view defined-name inventory changed"
    );
    assert_eq!(
        custom_view_defined_names,
        custom_view_defined_name_kinds.values().sum::<usize>(),
        "XLS custom-view defined-name relationships were not traversed completely"
    );
    assert_eq!(
        user_names_streams, 3,
        "XLS User Names Stream inventory changed"
    );
    assert_eq!(
        user_names_records, 13,
        "XLS User Names record inventory changed"
    );
    assert_eq!(
        compatible_user_names, 0,
        "XLS opaque User Names Stream inventory changed"
    );
    assert_eq!(
        nonconforming_user_names, 3,
        "XLS nonconforming typed User Names Stream inventory changed"
    );
    assert_eq!(
        shared_workbook_users, 1,
        "XLS shared-workbook user inventory changed"
    );
    assert_eq!(
        unresolved_user_revision_links, 0,
        "XLS user/revision relationship inventory changed"
    );
    assert!(
        revision_relationship_errors.is_empty(),
        "XLS revision relationships failed: {revision_relationship_errors:#?}"
    );
    assert!(
        file_entry_issues.is_empty(),
        "XLS storage/stream relationships failed: {file_entry_issues:#?}"
    );
    assert_eq!(
        files.len(),
        opened + exclusions.len() + rejected.values().sum::<usize>(),
        "XLS corpus inventory was not fully accounted for"
    );
    eprintln!(
        "XLS file roots: {} corpus files/{opened} opened/{reopened} reopened/{typed_file_entries} typed storage/stream entries/{pivot_cache_streams} PivotCache streams ({parsed_pivot_cache_streams} parsed/{compatible_pivot_cache_streams} compatible, {pivot_cache_records} records)/{pivot_cache_definitions} PivotCache definitions/{pivot_table_views} PivotTable views/{unresolved_pivot_table_cache_links} unresolved PivotTable cache links/{custom_views} custom views/{custom_sheet_views} custom sheet views ({chart_custom_sheet_views} chart/{custom_view_active_sheet_links} active-sheet links/{custom_views_without_active_sheet} without active sheet/{unresolved_custom_view_active_sheet_links} unresolved active sheets/{custom_view_defined_names} defined names {custom_view_defined_name_kinds:?}/{unlinked_custom_sheet_views} unlinked/{unlinked_custom_view_records} unmatched delimiters)/{revision_log_streams} Revision Log streams ({revision_log_records} records/{compatible_revision_logs} compatible, {revision_logs} logs/{unlinked_revision_records} unlinked records, productions {revision_productions:?}/{revision_delete_productions} delete/{incomplete_revision_productions} incomplete/{nested_revision_change_cells} nested cell/{nested_revision_formats} nested format/{revision_font_reset_records} font resets/{unlinked_revision_production_records} unlinked production records/{revision_sheet_links} resolved sheet links/{revision_without_sheet} without sheet/{revision_local_sheet_links} local-name sheet links/{revision_custom_view_links} custom-view links/{unresolved_revision_custom_view_links} unresolved custom-view links)/{user_names_streams} User Names streams ({user_names_records} records/{compatible_user_names} compatible, {shared_workbook_users} users/{unresolved_user_revision_links} unresolved revision links)/{unresolved_sheet_links} unresolved sheet links/{unlinked_sheet_substreams} unlinked sheet substreams/{typed_rows} sparse rows/{duplicate_row_coordinates} duplicate row coordinates/{typed_cells} typed cells/{duplicate_cell_coordinates} duplicate cell coordinates/{typed_formulas} formulas/{shared_formulas} shared formulas/{unresolved_formula_expressions} unresolved formula expressions ({unresolved_exp_formulas} PtgExp/{unresolved_table_formulas} PtgTbl)/{merged_cells} merged ranges/{sheet_drawings} drawings/{sheet_objects} objects ({control_stream_objects} Ctls/{embedding_storage_objects} MBD/{link_storage_objects} LNK/{dde_data_items} DDE)/{} manifest exclusions/{} other rejected; PivotCache compatibility {pivot_cache_compatibility:#?}; pivot relationship errors {pivot_relationship_errors:#?}; Revision Log compatibility {revision_log_compatibility:#?}; User Names compatibility {user_names_compatibility:#?}; custom-view compatibility {custom_view_compatibility:#?}; revision relationship errors {revision_relationship_errors:#?}; storage/stream issues {file_entry_issues:#?}; object relationship errors {object_relationship_errors:#?}; cell relationship errors {cell_relationship_errors:#?}; rejection shapes {rejected:#?}",
        files.len(),
        exclusions.len(),
        rejected.values().sum::<usize>()
    );
}

fn try_xls_sheet_reorder(file: &mut XlsFile) -> Result<bool, String> {
    if file.revision_log.is_some() {
        return Ok(false);
    }
    let candidate = file.workbooks.iter().find_map(|workbook| {
        let relationships = workbook.relationships().ok()?;
        if relationships.sheets().len() < 2 || !relationships.unresolved_sheets().is_empty() {
            return None;
        }
        let old_order = relationships
            .sheets()
            .iter()
            .map(|sheet| sheet.id())
            .collect::<Vec<_>>();
        let mut new_order = old_order.clone();
        new_order.swap(0, 1);
        let metadata = relationships
            .sheets()
            .iter()
            .map(|sheet| {
                (
                    sheet.id(),
                    (
                        sheet.metadata().state,
                        sheet.metadata().sheet_type,
                        sheet.metadata().name.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let name_scopes = (1..=relationships.defined_names().len())
            .map(|index| {
                let index = u32::try_from(index).ok()?;
                relationships
                    .defined_name_scope(index)
                    .ok()
                    .map(|scope| scope.map(|sheet| sheet.id()))
            })
            .collect::<Option<Vec<_>>>()?;
        Some((workbook.name, old_order, new_order, metadata, name_scopes))
    });
    let Some((workbook_name, old_order, new_order, metadata, name_scopes)) = candidate else {
        return Ok(false);
    };

    let before_invalid = file.clone();
    let mut invalid_order = old_order.clone();
    invalid_order[1] = invalid_order[0];
    if file.reorder_sheets(workbook_name, &invalid_order).is_ok() || file != &before_invalid {
        return Err("failed XLS sheet reorder changed the file root".to_owned());
    }

    let before_reorder = file.clone();
    if let Err(error) = file.reorder_sheets(workbook_name, &new_order) {
        if file != &before_reorder {
            return Err(format!(
                "failed XLS sheet reorder was not transactional: {error}"
            ));
        }
        return Ok(false);
    }
    let relationships = file
        .workbook_stream(workbook_name)
        .ok_or_else(|| "reordered XLS Workbook stream is missing".to_owned())?
        .relationships()
        .map_err(|error| error.to_string())?;
    let actual_order = relationships
        .sheets()
        .iter()
        .map(|sheet| sheet.id())
        .collect::<Vec<_>>();
    if actual_order != new_order {
        return Err("XLS sheet identities did not survive reorder".to_owned());
    }
    for sheet in relationships.sheets() {
        let expected = &metadata[&sheet.id()];
        if (sheet.metadata().state, sheet.metadata().sheet_type) != (expected.0, expected.1)
            || sheet.metadata().name != expected.2
        {
            return Err("XLS sheet identity changed its BoundSheet8 metadata".to_owned());
        }
    }
    let actual_name_scopes = (1..=relationships.defined_names().len())
        .map(|index| {
            let index = u32::try_from(index)
                .map_err(|_| "XLS defined-name index exceeds u32".to_owned())?;
            relationships
                .defined_name_scope(index)
                .map(|scope| scope.map(|sheet| sheet.id()))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if actual_name_scopes != name_scopes {
        return Err("XLS local defined-name scope changed after sheet reorder".to_owned());
    }

    let expected_ordered_names = new_order
        .iter()
        .map(|id| metadata[id].2.clone())
        .collect::<Vec<_>>();
    let bytes = file
        .to_bytes_preserving_compatibility()
        .map_err(|error| error.to_string())?;
    let reopened = XlsFile::from_bytes_compatible(&bytes)
        .map_err(|error| error.to_string())?
        .value;
    let reopened_relationships = reopened
        .workbook_stream(workbook_name)
        .ok_or_else(|| "reopened reordered Workbook stream is missing".to_owned())?
        .relationships()
        .map_err(|error| error.to_string())?;
    let reopened_names = reopened_relationships
        .sheets()
        .iter()
        .map(|sheet| sheet.metadata().name.clone())
        .collect::<Vec<_>>();
    if reopened_names != expected_ordered_names {
        return Err("XLS sheet order changed after save and reopen".to_owned());
    }
    Ok(true)
}

fn try_xls_number_cell_edit(file: &mut XlsFile) -> Result<bool, String> {
    let mut candidate = None;
    'workbooks: for workbook in file.workbooks.iter() {
        let Ok(relationships) = workbook.relationships() else {
            continue;
        };
        for sheet in relationships.sheets().iter().copied() {
            let Ok(index) = sheet.sparse_cell_index() else {
                continue;
            };
            for cell in index.rows().flat_map(|row| row.cells()) {
                let XlsCellValueRef::Number(value) = cell.value() else {
                    continue;
                };
                candidate = Some((
                    workbook.name,
                    sheet.id(),
                    cell.cell().row,
                    cell.cell().column,
                    value.value_bits,
                ));
                break 'workbooks;
            }
        }
    }
    let Some((workbook_name, sheet_id, row, column, old_bits)) = candidate else {
        return Ok(false);
    };

    let before_failed_edit = file.clone();
    let failed: olecfsdk::Result<()> =
        file.edit_cell(workbook_name, sheet_id, row, column, |cell| {
            let XlsCellMut::Number(value) = cell else {
                return Err(olecfsdk::Error::invalid(
                    0,
                    "selected Number cell changed static variant",
                ));
            };
            value.value_bits ^= 1;
            Err(olecfsdk::Error::invalid(
                0,
                "intentional transaction rollback",
            ))
        });
    if failed.is_ok() || file != &before_failed_edit {
        return Err("failed cell edit transaction changed the XLS root".to_owned());
    }

    file.edit_cell(workbook_name, sheet_id, row, column, |cell| {
        let XlsCellMut::Number(value) = cell else {
            return Err(olecfsdk::Error::invalid(
                0,
                "selected Number cell changed static variant",
            ));
        };
        value.value_bits ^= 1;
        Ok(())
    })
    .map_err(|error| error.to_string())?;

    let workbook = file
        .workbook_stream(workbook_name)
        .expect("edited Workbook stream remains present");
    let relationships = workbook
        .relationships()
        .map_err(|error| error.to_string())?;
    let sheet = relationships
        .sheet(sheet_id)
        .expect("edited sheet identity remains present");
    let index = sheet
        .sparse_cell_index()
        .map_err(|error| error.to_string())?;
    let cell = index
        .cell(row, column)
        .map_err(|error| error.to_string())?
        .expect("edited Number cell remains present");
    let XlsCellValueRef::Number(value) = cell.value() else {
        return Err("edited Number cell changed static variant".to_owned());
    };
    if value.value_bits != old_bits ^ 1 {
        return Err("Number cell edit did not update its typed IEEE-754 bits".to_owned());
    }
    Ok(true)
}

fn try_xls_sheet_name_growth(file: &mut XlsFile) -> Result<bool, String> {
    let mut candidate = None;
    for workbook in file.workbooks.iter() {
        let Ok(relationships) = workbook.relationships() else {
            continue;
        };
        for sheet in relationships.sheets().iter().copied() {
            let character_count = sheet.metadata().name.value.chars().count();
            if character_count < 31 {
                let mut name = sheet.metadata().name.clone();
                name.value.push('X');
                candidate = Some((
                    workbook.name,
                    sheet.id(),
                    sheet.metadata().sheet_bof_offset,
                    name,
                ));
                break;
            }
        }
        if candidate.is_some() {
            break;
        }
    }
    let Some((workbook_name, sheet_id, old_sheet_bof, name)) = candidate else {
        return Ok(false);
    };

    let mut invalid_name = name.clone();
    invalid_name.value.clear();
    let before_invalid_edit = file.clone();
    if file
        .set_sheet_name(workbook_name, sheet_id, invalid_name)
        .is_ok()
    {
        return Err("empty BoundSheet8 name was accepted".to_owned());
    }
    if file != &before_invalid_edit {
        return Err("failed sheet-name transaction changed the XLS root".to_owned());
    }

    file.set_sheet_name(workbook_name, sheet_id, name)
        .map_err(|error| error.to_string())?;

    let workbook = file
        .workbook_stream(workbook_name)
        .expect("edited Workbook stream remains present");
    let relationships = workbook
        .relationships()
        .map_err(|error| error.to_string())?;
    let sheet = relationships
        .sheet(sheet_id)
        .expect("edited sheet identity remains present");
    if sheet.metadata().sheet_bof_offset == old_sheet_bof {
        return Err("BoundSheet8 growth did not relocate its sheet BOF pointer".to_owned());
    }
    if !sheet.records().first().is_some_and(|record| {
        record.offset == sheet.metadata().sheet_bof_offset
            && matches!(record.data, BiffRecordData::Bof(_))
    }) {
        return Err("relocated BoundSheet8 pointer does not reference a BOF record".to_owned());
    }
    Ok(true)
}

fn conforming_doc_text_units(
    characters: &TextPieceCharacters,
) -> Option<(TextPieceEncoding, Vec<u16>)> {
    let value = characters.value()?;
    let encoding = characters.encoding();
    let units = match encoding {
        TextPieceEncoding::Compressed => value
            .chars()
            .map(|character| u16::try_from(u32::from(character)).ok())
            .collect::<Option<Vec<_>>>()?,
        TextPieceEncoding::Utf16 => value.encode_utf16().collect(),
    };
    Some((encoding, units))
}

fn conforming_doc_text_from_units(encoding: TextPieceEncoding, units: &[u16]) -> Option<String> {
    match encoding {
        TextPieceEncoding::Compressed => units
            .iter()
            .copied()
            .map(|unit| u8::try_from(unit).ok().map(char::from))
            .collect::<Option<String>>(),
        TextPieceEncoding::Utf16 => String::from_utf16(units).ok(),
    }
}

fn try_doc_paragraph_mark_edit(file: &DocFile, save_policy: DocSavePolicy) -> Result<bool, String> {
    let Some(source_runs) = &file.word_document.papx_runs else {
        return Ok(false);
    };
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let candidate_end = piece_end.min(main_len);
        if piece_start >= candidate_end {
            continue;
        }
        let limit = usize::try_from(candidate_end - piece_start)
            .map_err(|_| "DOC text piece length exceeds usize".to_owned())?;
        let Some((encoding, values)) = conforming_doc_text_units(&piece.value.characters) else {
            continue;
        };
        let candidate = values
            .iter()
            .take(limit)
            .enumerate()
            .find(|(index, value)| **value >= 0x20 && index + 1 < limit)
            .and_then(|(index, value)| {
                conforming_doc_text_from_units(encoding, &[*value, 0x000d])
                    .map(|replacement| (index, replacement))
            });
        let Some((index, replacement)) = candidate else {
            continue;
        };
        let cp = piece_start
            .checked_add(u32::try_from(index).map_err(|_| "DOC text index exceeds u32".to_owned())?)
            .ok_or_else(|| "DOC text CP overflow".to_owned())?;
        let Some(run_index) = source_runs
            .iter()
            .position(|run| run.cp_start <= cp && cp + 1 < run.cp_end)
        else {
            continue;
        };
        let mut inserted_runs = Vec::with_capacity(source_runs.len() + 1);
        for (index, run) in source_runs.iter().enumerate() {
            if index < run_index {
                inserted_runs.push(run.clone());
            } else if index == run_index {
                let mut first = run.clone();
                first.cp_end = cp + 2;
                let mut second = run.clone();
                second.cp_start = cp + 2;
                second.cp_end = second
                    .cp_end
                    .checked_add(1)
                    .ok_or_else(|| "DOC PAPX CP overflow".to_owned())?;
                inserted_runs.push(first);
                inserted_runs.push(second);
            } else {
                let mut shifted = run.clone();
                shifted.cp_start = shifted
                    .cp_start
                    .checked_add(1)
                    .ok_or_else(|| "DOC PAPX CP overflow".to_owned())?;
                shifted.cp_end = shifted
                    .cp_end
                    .checked_add(1)
                    .ok_or_else(|| "DOC PAPX CP overflow".to_owned())?;
                inserted_runs.push(shifted);
            }
        }
        let mut inserted = file.clone();
        if inserted
            .replace_main_text_range_with_papx_runs(cp..cp + 1, replacement, inserted_runs.clone())
            .is_err()
        {
            continue;
        }
        let inserted_bytes = match save_doc_with_policy(&inserted, save_policy) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let inserted_reopened = match DocFile::from_bytes(&inserted_bytes) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if inserted_reopened.word_document.fib.rg_lw.ccp_text
            != file.word_document.fib.rg_lw.ccp_text + 1
            || inserted_reopened.word_document.papx_runs != Some(inserted_runs)
        {
            return Err(
                "DOC paragraph insertion did not rebuild the explicit PAPX tree".to_owned(),
            );
        }
        let Some(marker_piece) = inserted_reopened
            .word_document
            .text_pieces
            .iter()
            .find(|piece| {
                u32::try_from(piece.value.cp_start).is_ok_and(|start| start <= cp + 1)
                    && u32::try_from(piece.value.cp_end).is_ok_and(|end| cp + 1 < end)
            })
        else {
            return Err("inserted DOC paragraph mark has no text piece".to_owned());
        };
        let empty = conforming_doc_text_from_units(marker_piece.value.characters.encoding(), &[])
            .expect("empty DOC text is valid in either physical encoding");
        let mut deleted = inserted_reopened.clone();
        deleted
            .replace_main_text_range_with_papx_runs(cp + 1..cp + 2, empty, source_runs.clone())
            .map_err(|error| error.to_string())?;
        let deleted_bytes = save_doc_with_policy(&deleted, save_policy)?;
        let deleted_reopened =
            DocFile::from_bytes(&deleted_bytes).map_err(|error| error.to_string())?;
        if deleted_reopened.word_document.fib.rg_lw.ccp_text
            != file.word_document.fib.rg_lw.ccp_text
            || deleted_reopened.word_document.papx_runs != Some(source_runs.clone())
        {
            return Err("DOC paragraph deletion did not restore the explicit PAPX tree".to_owned());
        }
        let second_save = save_doc_with_policy(&deleted_reopened, save_policy)?;
        let second_reopen = DocFile::from_bytes(&second_save).map_err(|error| error.to_string())?;
        if second_reopen.word_document != deleted_reopened.word_document
            || second_reopen.table != deleted_reopened.table
        {
            return Err(
                "DOC explicit paragraph edit was not stable after a second save".to_owned(),
            );
        }
        return Ok(true);
    }
    Ok(false)
}

fn try_doc_part_paragraph_mark_edit(
    file: &DocFile,
    part: FieldDocumentPart,
    save_policy: DocSavePolicy,
) -> Result<bool, String> {
    if file.validate_document_part_structure(part).is_err() {
        return Ok(false);
    }
    let Some(source_runs) = &file.word_document.papx_runs else {
        return Ok(false);
    };
    let (part_start, part_len) = doc_part_range(file, part)?;
    let part_end = part_start
        .checked_add(part_len)
        .ok_or_else(|| "DOC document-part range overflow".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let candidate_start = piece_start.max(part_start);
        let candidate_end = piece_end.min(part_end);
        if candidate_start >= candidate_end {
            continue;
        }
        let first = usize::try_from(candidate_start - piece_start)
            .map_err(|_| "DOC text index exceeds usize".to_owned())?;
        let last = usize::try_from(candidate_end - piece_start)
            .map_err(|_| "DOC text index exceeds usize".to_owned())?;
        let Some((encoding, values)) = conforming_doc_text_units(&piece.value.characters) else {
            continue;
        };
        let candidate = values
            .get(first..last)
            .and_then(|range| {
                range
                    .iter()
                    .enumerate()
                    .find(|(index, value)| **value >= 0x20 && first + index + 1 < last)
            })
            .and_then(|(offset, value)| {
                conforming_doc_text_from_units(encoding, &[*value, 0x000d])
                    .map(|replacement| (first + offset, replacement))
            });
        let Some((index, replacement)) = candidate else {
            continue;
        };
        let global_cp = piece_start
            .checked_add(u32::try_from(index).map_err(|_| "DOC text index exceeds u32".to_owned())?)
            .ok_or_else(|| "DOC text CP overflow".to_owned())?;
        let local_cp = global_cp - part_start;
        let Some(inserted_runs) = papx_runs_after_paragraph_insert(source_runs, global_cp)? else {
            continue;
        };
        let mut inserted = file.clone();
        if inserted
            .replace_text_range_with_papx_runs(
                part,
                local_cp..local_cp + 1,
                replacement,
                inserted_runs.clone(),
            )
            .is_err()
        {
            continue;
        }
        let inserted_bytes = match save_doc_with_policy(&inserted, save_policy) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        // The root corpus is opened in compatible mode; some otherwise usable files
        // have PAPX data references without a Data stream. Preserve that mode while
        // exercising the typed edit instead of upgrading the source's conformance.
        let inserted_reopened = match DocFile::from_bytes_compatible(&inserted_bytes) {
            Ok(value) => value.value,
            Err(_) => continue,
        };
        inserted_reopened
            .validate_document_part_structure(part)
            .map_err(|error| error.to_string())?;
        let (_, inserted_part_len) = doc_part_range(&inserted_reopened, part)?;
        if inserted_part_len != part_len + 1
            || inserted_reopened.word_document.papx_runs != Some(inserted_runs)
        {
            return Err(format!(
                "{part:?} paragraph insertion did not rebuild the explicit PAPX tree"
            ));
        }
        let Some(marker_piece) = inserted_reopened
            .word_document
            .text_pieces
            .iter()
            .find(|piece| {
                u32::try_from(piece.value.cp_start).is_ok_and(|start| start <= global_cp + 1)
                    && u32::try_from(piece.value.cp_end).is_ok_and(|end| global_cp + 1 < end)
            })
        else {
            return Err(format!(
                "inserted {part:?} paragraph mark has no text piece"
            ));
        };
        let empty = conforming_doc_text_from_units(marker_piece.value.characters.encoding(), &[])
            .expect("empty DOC text is valid in either physical encoding");
        let mut deleted = inserted_reopened.clone();
        deleted
            .replace_text_range_with_papx_runs(
                part,
                local_cp + 1..local_cp + 2,
                empty,
                source_runs.clone(),
            )
            .map_err(|error| error.to_string())?;
        let deleted_bytes = save_doc_with_policy(&deleted, save_policy)?;
        let deleted_reopened = DocFile::from_bytes_compatible(&deleted_bytes)
            .map_err(|error| error.to_string())?
            .value;
        deleted_reopened
            .validate_document_part_structure(part)
            .map_err(|error| error.to_string())?;
        let (_, deleted_part_len) = doc_part_range(&deleted_reopened, part)?;
        if deleted_part_len != part_len
            || deleted_reopened.word_document.papx_runs != Some(source_runs.clone())
        {
            return Err(format!(
                "{part:?} paragraph deletion did not restore the explicit PAPX tree"
            ));
        }
        let second_save = save_doc_with_policy(&deleted_reopened, save_policy)?;
        let second_reopen = DocFile::from_bytes_compatible(&second_save)
            .map_err(|error| error.to_string())?
            .value;
        if second_reopen.word_document != deleted_reopened.word_document
            || second_reopen.table != deleted_reopened.table
        {
            return Err(format!(
                "{part:?} explicit paragraph edit was not stable after a second save"
            ));
        }
        return Ok(true);
    }
    Ok(false)
}

fn papx_runs_after_paragraph_insert(
    source_runs: &[DocPapxRun],
    cp: u32,
) -> Result<Option<Vec<DocPapxRun>>, String> {
    let Some(run_index) = source_runs
        .iter()
        .position(|run| run.cp_start <= cp && cp + 1 < run.cp_end)
    else {
        return Ok(None);
    };
    let mut inserted_runs = Vec::with_capacity(source_runs.len() + 1);
    for (index, run) in source_runs.iter().enumerate() {
        if index < run_index {
            inserted_runs.push(run.clone());
        } else if index == run_index {
            let mut first = run.clone();
            first.cp_end = cp + 2;
            let mut second = run.clone();
            second.cp_start = cp + 2;
            second.cp_end = second
                .cp_end
                .checked_add(1)
                .ok_or_else(|| "DOC PAPX CP overflow".to_owned())?;
            inserted_runs.push(first);
            inserted_runs.push(second);
        } else {
            let mut shifted = run.clone();
            shifted.cp_start = shifted
                .cp_start
                .checked_add(1)
                .ok_or_else(|| "DOC PAPX CP overflow".to_owned())?;
            shifted.cp_end = shifted
                .cp_end
                .checked_add(1)
                .ok_or_else(|| "DOC PAPX CP overflow".to_owned())?;
            inserted_runs.push(shifted);
        }
    }
    Ok(Some(inserted_runs))
}

fn doc_part_range(file: &DocFile, target: FieldDocumentPart) -> Result<(u32, u32), String> {
    let fib = &file.word_document.fib.rg_lw;
    let parts = [
        (FieldDocumentPart::Main, fib.ccp_text),
        (FieldDocumentPart::Footnote, fib.ccp_footnote),
        (FieldDocumentPart::Header, fib.ccp_header),
        (FieldDocumentPart::Comment, fib.ccp_comment),
        (FieldDocumentPart::Endnote, fib.ccp_endnote),
        (FieldDocumentPart::Textbox, fib.ccp_textbox),
        (FieldDocumentPart::HeaderTextbox, fib.ccp_header_textbox),
    ];
    let mut start = 0u32;
    for (part, signed_len) in parts {
        let len = u32::try_from(signed_len)
            .map_err(|_| "DOC document-part length is negative".to_owned())?;
        if part == target {
            return Ok((start, len));
        }
        start = start
            .checked_add(len)
            .ok_or_else(|| "DOC document-part range overflow".to_owned())?;
    }
    Err("unknown DOC document part".to_owned())
}

fn try_doc_chpx_tree_edit(file: &DocFile, save_policy: DocSavePolicy) -> Result<bool, String> {
    let Some(runs) = &file.word_document.chpx_runs else {
        return Ok(false);
    };
    let Some(index) = runs.iter().enumerate().find_map(|(index, run)| {
        let left_is_formatted = index == 0 || runs[index - 1].properties.is_some();
        let right_is_formatted = index + 1 == runs.len() || runs[index + 1].properties.is_some();
        (run.properties.is_some() && left_is_formatted && right_is_formatted).then_some(index)
    }) else {
        return Ok(false);
    };
    let mut edited = file.clone();
    Arc::make_mut(&mut edited.word_document)
        .chpx_runs
        .as_mut()
        .expect("CHPX CP tree was checked")[index]
        .properties = None;
    let bytes = match save_doc_with_policy(&edited, save_policy) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    let reopened = match DocFile::from_bytes(&bytes) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };
    if reopened.word_document.chpx_runs != edited.word_document.chpx_runs {
        return Err("DOC CHPX CP-tree edit did not rebuild physical FKP pages".to_owned());
    }
    let second_save = save_doc_with_policy(&reopened, save_policy)?;
    let second_reopen = DocFile::from_bytes(&second_save).map_err(|error| error.to_string())?;
    if second_reopen.word_document != reopened.word_document
        || second_reopen.table != reopened.table
    {
        return Err("DOC CHPX CP-tree edit was not stable after a second save".to_owned());
    }
    Ok(true)
}

fn try_doc_papx_tree_edit(file: &DocFile, save_policy: DocSavePolicy) -> Result<bool, String> {
    let Some(runs) = &file.word_document.papx_runs else {
        return Ok(false);
    };
    let Some(index) = runs.iter().position(|run| {
        run.properties
            .as_ref()
            .is_some_and(|properties| properties.style_index != 0)
    }) else {
        return Ok(false);
    };
    let mut edited = file.clone();
    let properties = Arc::make_mut(&mut edited.word_document)
        .papx_runs
        .as_mut()
        .expect("PAPX CP tree was checked")[index]
        .properties
        .as_mut()
        .expect("non-default PAPX was checked");
    Arc::make_mut(properties).style_index = 0;
    let bytes = match save_doc_with_policy(&edited, save_policy) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    let reopened = match DocFile::from_bytes(&bytes) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };
    if reopened.word_document.papx_runs != edited.word_document.papx_runs {
        return Err("DOC PAPX CP-tree edit did not rebuild physical FKP pages".to_owned());
    }
    let second_save = save_doc_with_policy(&reopened, save_policy)?;
    let second_reopen = DocFile::from_bytes(&second_save).map_err(|error| error.to_string())?;
    if second_reopen.word_document != reopened.word_document
        || second_reopen.table != reopened.table
    {
        return Err("DOC PAPX CP-tree edit was not stable after a second save".to_owned());
    }
    Ok(true)
}

fn try_doc_paragraph_format_boundary_edit(
    file: &DocFile,
    save_policy: DocSavePolicy,
) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start_cp = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end_cp = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_start_fc = piece.value.file_offset;
        let width = match piece.value.characters.encoding() {
            TextPieceEncoding::Compressed => 1u32,
            TextPieceEncoding::Utf16 => 2u32,
        };
        let piece_end_fc = piece_start_fc
            .checked_add(
                (piece_end_cp - piece_start_cp)
                    .checked_mul(width)
                    .ok_or_else(|| "DOC text piece FC limit overflow".to_owned())?,
            )
            .ok_or_else(|| "DOC text piece FC limit overflow".to_owned())?;
        for boundary_fc in file
            .word_document
            .papx_fkp_pages()
            .iter()
            .flat_map(|page| page.value.file_positions.iter().copied())
        {
            if boundary_fc <= piece_start_fc || boundary_fc >= piece_end_fc {
                continue;
            }
            let delta = boundary_fc - piece_start_fc;
            if delta % width != 0 {
                continue;
            }
            let boundary_cp = piece_start_cp + delta / width;
            if boundary_cp == 0 || boundary_cp >= piece_end_cp || boundary_cp >= main_len {
                continue;
            }
            let first_index = usize::try_from(boundary_cp - 1 - piece_start_cp)
                .map_err(|_| "DOC text index exceeds usize".to_owned())?;
            let Some((encoding, units)) = conforming_doc_text_units(&piece.value.characters) else {
                continue;
            };
            let Some(values) = units.get(first_index..first_index + 2) else {
                continue;
            };
            if !matches!(values[0], 0x0007 | 0x000c | 0x000d) {
                continue;
            };
            let Some(replacement) =
                conforming_doc_text_from_units(encoding, &[u16::from(b'X'), values[0], values[1]])
            else {
                continue;
            };
            let mut edited = file.clone();
            if edited
                .replace_main_text_range(boundary_cp - 1..boundary_cp + 1, replacement)
                .is_err()
            {
                continue;
            }
            let bytes = match save_doc_with_policy(&edited, save_policy) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            let reopened = DocFile::from_bytes_compatible(&bytes)
                .map_err(|error| error.to_string())?
                .value;
            if reopened.word_document.fib.rg_lw.ccp_text
                != file.word_document.fib.rg_lw.ccp_text + 1
                || reopened.table.clx.value.piece_table.character_positions
                    != edited.table.clx.value.piece_table.character_positions
                || reopened.word_document.text_pieces[piece.piece_index]
                    .value
                    .characters
                    != edited.word_document.text_pieces[piece.piece_index]
                        .value
                        .characters
            {
                return Err(
                    "DOC PAPX-boundary edit did not rebuild paragraph formatting layout".to_owned(),
                );
            }
            let second_save = save_doc_with_policy(&reopened, save_policy)?;
            let second_reopen = DocFile::from_bytes_compatible(&second_save)
                .map_err(|error| error.to_string())?
                .value;
            if second_reopen.word_document != reopened.word_document
                || second_reopen.table != reopened.table
            {
                return Err("DOC PAPX-boundary edit was not stable after a second save".to_owned());
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn try_doc_text_piece_boundary_edit(
    file: &DocFile,
    save_policy: DocSavePolicy,
) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for pieces in file.word_document.text_pieces.windows(2) {
        let left_start = u32::try_from(pieces[0].value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let boundary = u32::try_from(pieces[0].value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let right_end = u32::try_from(pieces[1].value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        if pieces[1].value.cp_start != pieces[0].value.cp_end
            || boundary == 0
            || boundary >= main_len
            || boundary - left_start < 2
            || right_end - boundary < 2
        {
            continue;
        }
        let Some((encoding, units)) = conforming_doc_text_units(&pieces[0].value.characters) else {
            continue;
        };
        let Some(value) = units.last() else {
            continue;
        };
        let Some(replacement) =
            conforming_doc_text_from_units(encoding, &[*value, u16::from(b'X'), u16::from(b'Y')])
        else {
            continue;
        };
        let mut edited = file.clone();
        if edited
            .replace_main_text_range(boundary - 1..boundary + 1, replacement)
            .is_err()
        {
            continue;
        }
        let bytes = match save_doc_with_policy(&edited, save_policy) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let reopened = DocFile::from_bytes_compatible(&bytes)
            .map_err(|error| error.to_string())?
            .value;
        if reopened.word_document.fib.rg_lw.ccp_text != file.word_document.fib.rg_lw.ccp_text + 1
            || reopened.table.clx.value.piece_table.character_positions
                != edited.table.clx.value.piece_table.character_positions
            || !reopened
                .word_document
                .text_pieces
                .iter()
                .zip(&edited.word_document.text_pieces)
                .all(|(actual, expected)| {
                    actual.value.cp_start == expected.value.cp_start
                        && actual.value.cp_end == expected.value.cp_end
                        && actual.value.characters == expected.value.characters
                })
        {
            return Err("DOC text-piece boundary edit did not rebuild CLX/FC layout".to_owned());
        }
        let second_save = save_doc_with_policy(&reopened, save_policy)?;
        let second_reopen = DocFile::from_bytes_compatible(&second_save)
            .map_err(|error| error.to_string())?
            .value;
        if second_reopen.word_document != reopened.word_document
            || second_reopen.table != reopened.table
        {
            return Err(
                "DOC text-piece boundary edit was not stable after a second save".to_owned(),
            );
        }
        return Ok(true);
    }
    Ok(false)
}

fn try_doc_text_piece_removal(file: &DocFile, save_policy: DocSavePolicy) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for pieces in file.word_document.text_pieces.windows(2) {
        let left_start = u32::try_from(pieces[0].value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let boundary = u32::try_from(pieces[0].value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let right_end = u32::try_from(pieces[1].value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        if pieces[1].value.cp_start != pieces[0].value.cp_end
            || boundary > main_len
            || right_end > main_len
            || boundary - left_start < 2
            || boundary >= right_end
        {
            continue;
        }
        let Some((left_encoding, left_units)) =
            conforming_doc_text_units(&pieces[0].value.characters)
        else {
            continue;
        };
        let Some((_, right_units)) = conforming_doc_text_units(&pieces[1].value.characters) else {
            continue;
        };
        let mut terminators = Vec::new();
        let left_last = left_units.last().copied();
        if left_last.is_some_and(|value| matches!(value, 0x0007 | 0x000c | 0x000d)) {
            terminators.push(left_last.expect("left text piece is nonempty"));
        }
        terminators.extend(
            right_units
                .iter()
                .copied()
                .filter(|value| matches!(value, 0x0007 | 0x000c | 0x000d)),
        );
        let Some(replacement) = conforming_doc_text_from_units(left_encoding, &terminators) else {
            continue;
        };
        let replacement_len = u32::try_from(replacement.encode_utf16().count())
            .map_err(|_| "DOC replacement length exceeds u32".to_owned())?;
        let removed_len = right_end - (boundary - 1);
        let expected_main_len = main_len - removed_len + replacement_len;
        let mut edited = file.clone();
        if edited
            .replace_main_text_range(boundary - 1..right_end, replacement)
            .is_err()
        {
            continue;
        }
        if edited.word_document.text_pieces.len() + 1 != file.word_document.text_pieces.len() {
            continue;
        }
        let bytes = match save_doc_with_policy(&edited, save_policy) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let reopened = DocFile::from_bytes_compatible(&bytes)
            .map_err(|error| error.to_string())?
            .value;
        let expected_main_len = i32::try_from(expected_main_len)
            .map_err(|_| "DOC expected ccpText exceeds i32".to_owned())?;
        if reopened.word_document.fib.rg_lw.ccp_text != expected_main_len
            || reopened.table.clx.value.piece_table.character_positions
                != edited.table.clx.value.piece_table.character_positions
            || reopened.word_document.text_pieces.len() != edited.word_document.text_pieces.len()
            || !reopened
                .word_document
                .text_pieces
                .iter()
                .zip(&edited.word_document.text_pieces)
                .all(|(actual, expected)| {
                    actual.value.cp_start == expected.value.cp_start
                        && actual.value.cp_end == expected.value.cp_end
                        && actual.value.characters == expected.value.characters
                })
        {
            return Err(
                "DOC complete text-piece removal did not rebuild CLX/FKP layout".to_owned(),
            );
        }
        let second_save = save_doc_with_policy(&reopened, save_policy)?;
        let second_reopen = DocFile::from_bytes_compatible(&second_save)
            .map_err(|error| error.to_string())?
            .value;
        if second_reopen.word_document != reopened.word_document
            || second_reopen.table != reopened.table
        {
            return Err(
                "DOC complete text-piece removal was not stable after a second save".to_owned(),
            );
        }
        return Ok(true);
    }
    Ok(false)
}

fn try_doc_character_format_boundary_edit(
    file: &DocFile,
    save_policy: DocSavePolicy,
) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start_cp = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end_cp = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_start_fc = piece.value.file_offset;
        let width = match piece.value.characters.encoding() {
            TextPieceEncoding::Compressed => 1u32,
            TextPieceEncoding::Utf16 => 2u32,
        };
        let piece_end_fc = piece_start_fc
            .checked_add(
                (piece_end_cp - piece_start_cp)
                    .checked_mul(width)
                    .ok_or_else(|| "DOC text piece FC limit overflow".to_owned())?,
            )
            .ok_or_else(|| "DOC text piece FC limit overflow".to_owned())?;
        for boundary_fc in file
            .word_document
            .chpx_fkp_pages()
            .iter()
            .flat_map(|page| page.value.file_positions.iter().copied())
        {
            if boundary_fc <= piece_start_fc || boundary_fc >= piece_end_fc {
                continue;
            }
            let delta = boundary_fc - piece_start_fc;
            if delta % width != 0 {
                continue;
            }
            let boundary_cp = piece_start_cp + delta / width;
            if boundary_cp == 0 || boundary_cp >= piece_end_cp || boundary_cp >= main_len {
                continue;
            }
            let first_index = usize::try_from(boundary_cp - 1 - piece_start_cp)
                .map_err(|_| "DOC text index exceeds usize".to_owned())?;
            let Some((encoding, units)) = conforming_doc_text_units(&piece.value.characters) else {
                continue;
            };
            let Some(values) = units.get(first_index..first_index + 2) else {
                continue;
            };
            let Some(replacement) =
                conforming_doc_text_from_units(encoding, &[values[0], u16::from(b'X'), values[1]])
            else {
                continue;
            };
            let mut edited = file.clone();
            if edited
                .replace_main_text_range(boundary_cp - 1..boundary_cp + 1, replacement)
                .is_err()
            {
                continue;
            }
            let bytes = match save_doc_with_policy(&edited, save_policy) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            let reopened = DocFile::from_bytes_compatible(&bytes)
                .map_err(|error| error.to_string())?
                .value;
            if reopened.word_document.fib.rg_lw.ccp_text
                != file.word_document.fib.rg_lw.ccp_text + 1
                || reopened.table.clx.value.piece_table.character_positions
                    != edited.table.clx.value.piece_table.character_positions
                || reopened.word_document.text_pieces[piece.piece_index]
                    .value
                    .characters
                    != edited.word_document.text_pieces[piece.piece_index]
                        .value
                        .characters
            {
                return Err(
                    "DOC CHPX-boundary edit did not rebuild text and formatting layout".to_owned(),
                );
            }
            let second_save = save_doc_with_policy(&reopened, save_policy)?;
            let second_reopen = DocFile::from_bytes_compatible(&second_save)
                .map_err(|error| error.to_string())?
                .value;
            if second_reopen.word_document != reopened.word_document
                || second_reopen.table != reopened.table
            {
                return Err("DOC CHPX-boundary edit was not stable after a second save".to_owned());
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn try_doc_multiple_text_edits(file: &DocFile, save_policy: DocSavePolicy) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let candidate_end = piece_end.min(main_len);
        if piece_start >= candidate_end {
            continue;
        }
        let limit = usize::try_from(candidate_end - piece_start)
            .map_err(|_| "DOC text piece length exceeds usize".to_owned())?;
        let Some((encoding, units)) = conforming_doc_text_units(&piece.value.characters) else {
            continue;
        };
        let candidates = units
            .iter()
            .take(limit)
            .enumerate()
            .filter(|(_, value)| **value >= 0x20)
            .take(2)
            .map(|(index, value)| (index, *value))
            .collect::<Vec<_>>();
        let [(first_index, first), (second_index, second)] = candidates.as_slice() else {
            continue;
        };
        let first_cp = piece_start
            .checked_add(
                u32::try_from(*first_index).map_err(|_| "DOC text index exceeds u32".to_owned())?,
            )
            .ok_or_else(|| "DOC first text edit CP overflow".to_owned())?;
        let second_cp = piece_start
            .checked_add(
                u32::try_from(*second_index)
                    .map_err(|_| "DOC text index exceeds u32".to_owned())?,
            )
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| "DOC second text edit CP overflow".to_owned())?;
        let Some(first_replacement) =
            conforming_doc_text_from_units(encoding, &[*first, u16::from(b'X')])
        else {
            continue;
        };
        let Some(second_replacement) =
            conforming_doc_text_from_units(encoding, &[*second, u16::from(b'Y'), u16::from(b'Z')])
        else {
            continue;
        };
        let mut edited = file.clone();
        if edited
            .replace_main_text_range(first_cp..first_cp + 1, first_replacement)
            .is_err()
            || edited
                .replace_main_text_range(second_cp..second_cp + 1, second_replacement)
                .is_err()
        {
            continue;
        }
        let bytes = match save_doc_with_policy(&edited, save_policy) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let reopened = DocFile::from_bytes_compatible(&bytes)
            .map_err(|error| error.to_string())?
            .value;
        if reopened.word_document.fib.rg_lw.ccp_text != file.word_document.fib.rg_lw.ccp_text + 3
            || reopened.table.clx.value.piece_table.character_positions
                != edited.table.clx.value.piece_table.character_positions
            || reopened.word_document.text_pieces[piece.piece_index]
                .value
                .characters
                != edited.word_document.text_pieces[piece.piece_index]
                    .value
                    .characters
        {
            return Err(
                "multiple DOC text edits were not composed through CP/FC layout".to_owned(),
            );
        }
        let second_save = save_doc_with_policy(&reopened, save_policy)?;
        let second_reopen = DocFile::from_bytes_compatible(&second_save)
            .map_err(|error| error.to_string())?
            .value;
        if second_reopen.word_document != reopened.word_document
            || second_reopen.table != reopened.table
        {
            return Err("multiple DOC text edits were not stable after a second save".to_owned());
        }

        let Some(first_replacement) =
            conforming_doc_text_from_units(encoding, &[*first, u16::from(b'Q')])
        else {
            continue;
        };
        let empty_replacement = conforming_doc_text_from_units(encoding, &[])
            .expect("empty DOC text is valid in either physical encoding");
        let mut net_zero = file.clone();
        net_zero
            .replace_main_text_range(first_cp..first_cp + 1, first_replacement)
            .map_err(|error| error.to_string())?;
        net_zero
            .replace_main_text_range(second_cp..second_cp + 1, empty_replacement)
            .map_err(|error| error.to_string())?;
        let net_zero_bytes = save_doc_with_policy(&net_zero, save_policy)?;
        let net_zero_reopened = DocFile::from_bytes_compatible(&net_zero_bytes)
            .map_err(|error| error.to_string())?
            .value;
        if net_zero_reopened.word_document.fib.rg_lw.ccp_text
            != file.word_document.fib.rg_lw.ccp_text
            || net_zero_reopened
                .table
                .clx
                .value
                .piece_table
                .character_positions
                != net_zero.table.clx.value.piece_table.character_positions
            || net_zero_reopened.word_document.text_pieces[piece.piece_index]
                .value
                .characters
                != net_zero.word_document.text_pieces[piece.piece_index]
                    .value
                    .characters
        {
            return Err(
                "net-zero multiple DOC text edits were not composed through CP/FC layout"
                    .to_owned(),
            );
        }
        let net_zero_second_save = save_doc_with_policy(&net_zero_reopened, save_policy)?;
        let net_zero_second_reopen = DocFile::from_bytes_compatible(&net_zero_second_save)
            .map_err(|error| error.to_string())?
            .value;
        if net_zero_second_reopen.word_document != net_zero_reopened.word_document
            || net_zero_second_reopen.table != net_zero_reopened.table
        {
            return Err("net-zero DOC text edits were not stable after a second save".to_owned());
        }
        return Ok(true);
    }
    Ok(false)
}

fn try_doc_part_text_edit(
    file: &DocFile,
    target: FieldDocumentPart,
    save_policy: DocSavePolicy,
) -> Result<bool, String> {
    let fib = &file.word_document.fib.rg_lw;
    let parts = [
        (FieldDocumentPart::Main, fib.ccp_text),
        (FieldDocumentPart::Footnote, fib.ccp_footnote),
        (FieldDocumentPart::Header, fib.ccp_header),
        (FieldDocumentPart::Comment, fib.ccp_comment),
        (FieldDocumentPart::Endnote, fib.ccp_endnote),
        (FieldDocumentPart::Textbox, fib.ccp_textbox),
        (FieldDocumentPart::HeaderTextbox, fib.ccp_header_textbox),
    ];
    let mut part_start = 0u32;
    for (part, signed_len) in parts {
        let part_len = u32::try_from(signed_len)
            .map_err(|_| "DOC document-part length is negative".to_owned())?;
        let part_end = part_start
            .checked_add(part_len)
            .ok_or_else(|| "DOC document-part range overflow".to_owned())?;
        if part == target && part_len > 0 {
            for piece in &file.word_document.text_pieces {
                let piece_start = u32::try_from(piece.value.cp_start)
                    .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
                let piece_end = u32::try_from(piece.value.cp_end)
                    .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
                let candidate_start = piece_start.max(part_start);
                let candidate_end = piece_end.min(part_end);
                if candidate_start >= candidate_end {
                    continue;
                }
                let first = usize::try_from(candidate_start - piece_start)
                    .map_err(|_| "DOC text index exceeds usize".to_owned())?;
                let last = usize::try_from(candidate_end - piece_start)
                    .map_err(|_| "DOC text index exceeds usize".to_owned())?;
                let Some((encoding, values)) = conforming_doc_text_units(&piece.value.characters)
                else {
                    continue;
                };
                let candidate = values
                    .get(first..last)
                    .and_then(|range| range.iter().position(|value| *value >= 0x20))
                    .and_then(|offset| {
                        let index = first + offset;
                        conforming_doc_text_from_units(encoding, &[values[index], u16::from(b'X')])
                            .map(|replacement| (index, replacement))
                    });
                let Some((index, replacement)) = candidate else {
                    continue;
                };
                let global_cp = piece_start
                    .checked_add(
                        u32::try_from(index)
                            .map_err(|_| "DOC text index exceeds u32".to_owned())?,
                    )
                    .ok_or_else(|| "DOC text CP overflow".to_owned())?;
                let local_cp = global_cp - part_start;
                let mut edited = file.clone();
                if edited
                    .replace_text_range(part, local_cp..local_cp + 1, replacement)
                    .is_err()
                {
                    continue;
                }
                let Ok(bytes) = save_doc_with_policy(&edited, save_policy) else {
                    continue;
                };
                let reopened = DocFile::from_bytes_compatible(&bytes)
                    .map_err(|error| error.to_string())?
                    .value;
                let reopened_len = match part {
                    FieldDocumentPart::Footnote => reopened.word_document.fib.rg_lw.ccp_footnote,
                    FieldDocumentPart::Header => reopened.word_document.fib.rg_lw.ccp_header,
                    FieldDocumentPart::Comment => reopened.word_document.fib.rg_lw.ccp_comment,
                    FieldDocumentPart::Endnote => reopened.word_document.fib.rg_lw.ccp_endnote,
                    FieldDocumentPart::Textbox => reopened.word_document.fib.rg_lw.ccp_textbox,
                    FieldDocumentPart::HeaderTextbox => {
                        reopened.word_document.fib.rg_lw.ccp_header_textbox
                    }
                    FieldDocumentPart::Main | FieldDocumentPart::Macro => unreachable!(),
                };
                let text_matches = reopened
                    .word_document
                    .text_pieces
                    .iter()
                    .zip(&edited.word_document.text_pieces)
                    .all(|(actual, expected)| {
                        actual.value.cp_start == expected.value.cp_start
                            && actual.value.cp_end == expected.value.cp_end
                            && actual.value.characters == expected.value.characters
                    });
                let field_values_match = reopened
                    .table
                    .fields
                    .iter()
                    .map(|(part, value)| (part, &value.value))
                    .eq(edited
                        .table
                        .fields
                        .iter()
                        .map(|(part, value)| (part, &value.value)));
                let shape_anchor_values_match = reopened
                    .table
                    .shape_anchors
                    .iter()
                    .map(|(part, value)| (part, &value.value))
                    .eq(edited
                        .table
                        .shape_anchors
                        .iter()
                        .map(|(part, value)| (part, &value.value)));
                if reopened_len != signed_len + 1
                    || reopened.table.clx.value.piece_table.character_positions
                        != edited.table.clx.value.piece_table.character_positions
                    || !text_matches
                    || !field_values_match
                    || reopened.table.header_text != edited.table.header_text
                    || reopened.table.footnotes != edited.table.footnotes
                    || reopened.table.endnotes != edited.table.endnotes
                    || reopened.table.annotations != edited.table.annotations
                    || reopened.table.textbox_stories != edited.table.textbox_stories
                    || reopened.table.textbox_breaks != edited.table.textbox_breaks
                    || !shape_anchor_values_match
                {
                    return Err(format!(
                        "non-main DOC text edit did not relocate its typed CP/FC tree: len={}/{}, clx={}, text={}, fields={}, header={}, footnotes={}, endnotes={}, annotations={}, stories={}, breaks={}, anchors={}",
                        reopened_len,
                        signed_len + 1,
                        reopened.table.clx.value.piece_table.character_positions
                            == edited.table.clx.value.piece_table.character_positions,
                        text_matches,
                        field_values_match,
                        reopened.table.header_text == edited.table.header_text,
                        reopened.table.footnotes == edited.table.footnotes,
                        reopened.table.endnotes == edited.table.endnotes,
                        reopened.table.annotations == edited.table.annotations,
                        reopened.table.textbox_stories == edited.table.textbox_stories,
                        reopened.table.textbox_breaks == edited.table.textbox_breaks,
                        shape_anchor_values_match,
                    ));
                }
                return Ok(true);
            }
        }
        part_start = part_end;
    }
    Ok(false)
}

fn corpus_files(corpus: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for source in ["Apache-POI", "LibreOffice"] {
        collect(&corpus.join(source), extensions, &mut files);
    }
    files.sort();
    files
}

fn collect(directory: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extensions, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extensions
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            })
        {
            files.push(path);
        }
    }
}

fn exclusions_for(
    corpus: &Path,
    test_names: &[&str],
    extensions: &[&str],
) -> BTreeMap<PathBuf, ExpectationMode> {
    let mut exclusions = BTreeMap::new();
    for source in ["Apache-POI", "LibreOffice"] {
        let root = corpus.join(source);
        let manifest = read_manifest(&root.join("manifest.toml"))
            .unwrap_or_else(|error| panic!("read {source} manifest: {error}"));
        for expectation in manifest.expectation {
            if test_names.contains(&expectation.test.as_str())
                && matches!(
                    expectation.mode,
                    ExpectationMode::Invalid
                        | ExpectationMode::Unsupported
                        | ExpectationMode::RequiresPassword
                        | ExpectationMode::KnownFailure
                )
                && expectation
                    .file
                    .rsplit_once('.')
                    .is_some_and(|(_, extension)| {
                        extensions
                            .iter()
                            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
                    })
            {
                exclusions.insert(root.join(expectation.file), expectation.mode);
            }
        }
    }
    exclusions
}

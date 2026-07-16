use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    doc::{
        DocDataNodeValue, DocFile, DocPapxRun, FieldDocumentPart, NilPicfBinaryData,
        OleObjectPersist1Flags, TextPieceCharacters,
    },
    office_art::{
        OfficeArtBitmapData, OfficeArtDrawingGraphIssue, OfficeArtRecord, OfficeArtRecordData,
    },
    ppt::{
        BinaryTagData, CurrentUserData, PersistObjectReferenceStatus, PicturesStream, PptFile,
        PptLivePersistObjectRole, PptLivePresentation, PptRecordData, PptRecordSequence,
        PptTopLevelLiveRecordStatus, PptTopLevelRecordRole,
    },
    xls::{BiffRecordData, XlStringCharacters, XlsFile},
};
use olecfsdk_corpus_test_support::{
    corpus_bytes,
    manifest::{ExpectationMode, read_manifest},
};

#[test]
#[ignore = "classic Office file-root corpus round-trip runs explicitly"]
fn doc_files_round_trip_through_typed_root() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let exclusions = exclusions_for(&corpus, &["doc_fib_roundtrip"], &["doc"]);
    let files = corpus_files(&corpus, &["doc"]);
    let mut opened = 0usize;
    let mut reopened = 0usize;
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

    for path in &files {
        if exclusions.contains_key(path) {
            observed_exclusions.insert(path.clone());
            continue;
        }
        let mut parsed_root = false;
        let result = (|| {
            let bytes = corpus_bytes(path).map_err(|error| error.to_string())?;
            let file = DocFile::from_bytes_compatible(&bytes)
                .map_err(|error| error.to_string())?
                .value;
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
            let saved = file.to_bytes().map_err(|error| error.to_string())?;
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
            if !round_tripped.compound_file.logical_eq(&file.compound_file) {
                return Err(
                    "DOC compound-file object tree changed after write and reopen".to_owned(),
                );
            }
            if !edited_document_properties
                && file.table.document_properties.is_some()
                && file.table.compatibility_tables.is_empty()
            {
                let mut edited = file.clone();
                let properties = &mut edited
                    .table
                    .document_properties
                    .as_mut()
                    .expect("checked above")
                    .value;
                properties.word97.base.format_flags.facing_pages =
                    !properties.word97.base.format_flags.facing_pages;
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                let strings = edited
                    .table
                    .associated_strings
                    .as_mut()
                    .expect("checked above");
                let old_length = strings.location.lcb;
                strings.value.title.push(u16::from(b'X'));
                let expected_title = strings.value.title.clone();
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                let section = &mut edited.word_document.section_properties[section_index];
                let old_offset = section.offset;
                let old_cb_mac = edited.word_document.fib.rg_lw.cb_mac;
                let value = section.value.as_mut().expect("checked above");
                value
                    .properties
                    .properties
                    .push(value.properties.properties[0].clone());
                let expected = value.clone();
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                    matches!(
                        &piece.value.characters,
                        TextPieceCharacters::Compressed(value) if !value.is_empty()
                    )
                })
            {
                let mut edited = file.clone();
                let piece = &mut edited.word_document.text_pieces[piece_index].value;
                let old_offset = piece.file_offset;
                let old_cb_mac = edited.word_document.fib.rg_lw.cb_mac;
                let TextPieceCharacters::Compressed(characters) = &piece.characters else {
                    unreachable!("selected a compressed text piece")
                };
                piece.characters =
                    TextPieceCharacters::Utf16(characters.iter().copied().map(u16::from).collect());
                let expected = piece.characters.clone();
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                    let TextPieceCharacters::Compressed(characters) = &piece.value.characters
                    else {
                        continue;
                    };
                    for (index, character) in characters.iter().copied().enumerate() {
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
                            .replace_main_text_range(
                                cp..cp + 1,
                                TextPieceCharacters::Compressed(vec![character, b'X']),
                            )
                            .is_err()
                        {
                            continue;
                        }
                        let Ok(edited_bytes) = edited.to_bytes() else {
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
                if !relocated_multiple_text_edits && try_doc_multiple_text_edits(&file)? {
                    relocated_multiple_text_edits = true;
                }
                if !relocated_character_format_boundary
                    && try_doc_character_format_boundary_edit(&file)?
                {
                    relocated_character_format_boundary = true;
                }
                if !relocated_text_piece_boundary && try_doc_text_piece_boundary_edit(&file)? {
                    relocated_text_piece_boundary = true;
                }
                if !removed_text_piece && try_doc_text_piece_removal(&file)? {
                    removed_text_piece = true;
                }
                if !relocated_paragraph_format_boundary
                    && try_doc_paragraph_format_boundary_edit(&file)?
                {
                    relocated_paragraph_format_boundary = true;
                }
                if !edited_chpx_tree && try_doc_chpx_tree_edit(&file)? {
                    edited_chpx_tree = true;
                }
                if !edited_papx_tree && try_doc_papx_tree_edit(&file)? {
                    edited_papx_tree = true;
                }
                if !edited_paragraph_structure && try_doc_paragraph_mark_edit(&file)? {
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
                        && try_doc_part_paragraph_mark_edit(&file, part)?
                    {
                        edited_non_main_paragraph_parts.insert(part);
                    }
                    if !relocated_non_main_text_parts.contains(&part)
                        && try_doc_part_text_edit(&file, part)?
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
                let edited_object = edited
                    .object_pool
                    .as_mut()
                    .expect("checked above")
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
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                let edited_node = edited
                    .data
                    .as_mut()
                    .expect("checked above")
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
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                let node = edited
                    .data
                    .as_mut()
                    .expect("checked above")
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
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                let node = edited
                    .data
                    .as_mut()
                    .expect("checked above")
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
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
                let data = edited.data.as_mut().expect("checked above");
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
                let edited_bytes = edited.to_bytes().map_err(|error| error.to_string())?;
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
        "DOC file roots: {} corpus files/{opened} opened/{reopened} reopened/{} manifest exclusions/{} other rejected; {direct_formatting_queries} direct-formatting queries; direct-formatting errors {direct_formatting_errors:#?}; direct table states {direct_table_states:?}; {comment_cf_spec_markers} effective comment CFSpec markers/{comment_cf_spec_false_markers} false; false-marker files {comment_cf_spec_false:#?}; CFSpec errors {comment_cf_spec_errors:#?}; comment paragraph-mark table states {comment_table_states:?}; edited non-main text parts {relocated_non_main_text_parts:?}; non-main paragraph validation errors {non_main_paragraph_validation_errors:#?}; NilPICF kinds {nil_picf_kinds:?}; rejection shapes {rejected:#?}",
        files.len(),
        exclusions.len(),
        files.len() - opened - exclusions.len()
    );
    assert_eq!(observed_exclusions, exclusions.keys().cloned().collect());
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
    assert_eq!(files.len(), 533, "DOC extension inventory changed");
    assert_eq!(opened, 403, "typed DOC file-root coverage changed");
    assert_eq!(nil_picf_kinds.get("unresolved"), None);
    assert!(nil_picf_kinds.get("form").copied().unwrap_or_default() > 0);
    assert!(nil_picf_kinds.get("hyperlink").copied().unwrap_or_default() > 0);
    assert_eq!(exclusions.len(), 39, "DOC exclusion inventory changed");
    assert_eq!(
        rejected.values().sum::<usize>(),
        91,
        "DOC rejected inventory changed"
    );
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
            if !edited_ppt_layout && !round_tripped.compound_file.logical_eq(&file.compound_file) {
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
    assert_eq!(reopened, opened, "not every opened PPT file was reopened");
    assert_eq!(files.len(), 228, "PPT extension inventory changed");
    assert_eq!(opened, 176, "typed PPT file-root coverage changed");
    assert_eq!(exclusions.len(), 45, "PPT exclusion inventory changed");
    assert_eq!(
        rejected.values().sum::<usize>(),
        7,
        "PPT rejected inventory changed"
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
        "PPT live presentations: {live_presentation_files} files/{live_master_slides} masters/{live_presentation_slides} slides/{live_notes_slides} notes/{live_active_x_controls} ActiveX/{live_embedded_ole_objects} embedded OLE/{live_linked_ole_objects} linked OLE/{live_vba_projects} VBA/{live_persist_object_records} live persist top-level records/{dead_top_level_records} dead top-level records; errors {live_presentation_errors:#?}"
    );
    eprintln!(
        "PPT live OfficeArt graphs: {live_drawing_graph_files} files ({strict_live_drawing_graph_files} strict/{compatibility_live_drawing_graph_files} compatibility), {live_drawing_graph_drawings} drawings/{live_drawing_graph_shapes} shapes; BLIP {live_drawing_graph_blip_stores} stores/{live_drawing_graph_blip_entries} entries/{live_drawing_graph_blip_references} references, cRef {live_drawing_graph_blip_reference_count_relations:?}; issues {live_drawing_graph_issues:?}; errors {live_drawing_graph_errors:#?}"
    );
    eprintln!(
        "PPT compatible live presentations: {compatible_live_presentation_files} files/{compatible_live_presentation_diagnostics} diagnostics; errors {compatible_live_presentation_errors:#?}"
    );
}

fn ppt_live_signature(presentation: &PptLivePresentation) -> Vec<(u32, PptLivePersistObjectRole)> {
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
    let Some(PicturesStream::Complete(pictures)) = &file.pictures else {
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
    let Some(PicturesStream::Complete(pictures)) = &mut edited.pictures else {
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

    let Some(PicturesStream::Complete(pictures)) = &edited.pictures else {
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
        let record = edited
            .document
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
    if !grow_first_ppt_cstring(&mut candidate.document.records) {
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
        PptRecordData::CString(values)
            if !String::from_utf16_lossy(values).starts_with("___PPT") =>
        {
            values.push(u16::from(b'X'));
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
            let edited_sheet_layout =
                !relocated_sheet_layout && try_xls_sheet_name_growth(&mut file)?;
            if edited_sheet_layout {
                relocated_sheet_layout = true;
            }
            let saved = file
                .to_bytes_preserving_compatibility()
                .map_err(|error| error.to_string())?;
            let round_tripped = XlsFile::from_bytes_compatible(&saved)
                .map_err(|error| error.to_string())?
                .value;
            if round_tripped.workbooks != file.workbooks
                || round_tripped.revision_log != file.revision_log
            {
                return Err("managed XLS Rust tree changed after write and reopen".to_owned());
            }
            if !edited_sheet_layout && !round_tripped.compound_file.logical_eq(&file.compound_file)
            {
                return Err(
                    "XLS compound-file object tree changed after write and reopen".to_owned(),
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
        "XLS file-root failures:\n{}",
        failures.join("\n")
    );
    assert!(opened > 0, "no XLS files opened through XlsFile");
    assert!(
        relocated_sheet_layout,
        "no variable-length BoundSheet8 name was relaid out through XlsFile"
    );
    assert_eq!(reopened, opened, "not every opened XLS file was reopened");
    assert_eq!(files.len(), 770, "XLS extension inventory changed");
    assert_eq!(opened, 725, "typed XLS file-root coverage changed");
    assert_eq!(exclusions.len(), 9, "XLS exclusion inventory changed");
    assert_eq!(
        rejected.values().sum::<usize>(),
        36,
        "XLS rejected inventory changed"
    );
    eprintln!(
        "XLS file roots: {} corpus files/{opened} opened/{reopened} reopened/{} manifest exclusions/{} other rejected; rejection shapes {rejected:#?}",
        files.len(),
        exclusions.len(),
        rejected.values().sum::<usize>()
    );
}

fn try_xls_sheet_name_growth(file: &mut XlsFile) -> Result<bool, String> {
    let mut candidate_location = None;
    for (workbook_index, workbook) in file.workbooks.iter().enumerate() {
        for (record_index, record) in workbook.tree.stream.records.iter().enumerate() {
            let sheet = match &record.data {
                BiffRecordData::BoundSheet8(value)
                | BiffRecordData::BoundSheet8Compatibility { value, .. } => value,
                _ => continue,
            };
            let character_count = match &sheet.name.characters {
                XlStringCharacters::Compressed(values) => values.len(),
                XlStringCharacters::Unicode(values) => values.len(),
            };
            if character_count < 31 {
                candidate_location = Some((workbook_index, record_index, sheet.sheet_bof_offset));
                break;
            }
        }
        if candidate_location.is_some() {
            break;
        }
    }
    let Some((workbook_index, record_index, old_sheet_bof)) = candidate_location else {
        return Ok(false);
    };

    let sheet = match &mut file.workbooks[workbook_index].tree.stream.records[record_index].data {
        BiffRecordData::BoundSheet8(value)
        | BiffRecordData::BoundSheet8Compatibility { value, .. } => value,
        _ => unreachable!("candidate was a BoundSheet8 record"),
    };
    match &mut sheet.name.characters {
        XlStringCharacters::Compressed(values) => values.push(b'X'),
        XlStringCharacters::Unicode(values) => values.push(u16::from(b'X')),
    }
    file.relayout().map_err(|error| error.to_string())?;

    let workbook = &file.workbooks[workbook_index];
    let sheet = match &workbook.tree.stream.records[record_index].data {
        BiffRecordData::BoundSheet8(value)
        | BiffRecordData::BoundSheet8Compatibility { value, .. } => value,
        _ => unreachable!("candidate remained a BoundSheet8 record"),
    };
    if sheet.sheet_bof_offset == old_sheet_bof {
        return Err("BoundSheet8 growth did not relocate its sheet BOF pointer".to_owned());
    }
    if !workbook.tree.stream.records.iter().any(|record| {
        record.offset == sheet.sheet_bof_offset && matches!(record.data, BiffRecordData::Bof(_))
    }) {
        return Err("relocated BoundSheet8 pointer does not reference a BOF record".to_owned());
    }
    Ok(true)
}

fn try_doc_paragraph_mark_edit(file: &DocFile) -> Result<bool, String> {
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
        let candidate = match &piece.value.characters {
            TextPieceCharacters::Compressed(values) => values
                .iter()
                .take(limit)
                .enumerate()
                .find(|(index, value)| **value >= 0x20 && index + 1 < limit)
                .map(|(index, value)| (index, TextPieceCharacters::Compressed(vec![*value, 0x0d]))),
            TextPieceCharacters::Utf16(values) => values
                .iter()
                .take(limit)
                .enumerate()
                .find(|(index, value)| **value >= 0x20 && index + 1 < limit)
                .map(|(index, value)| (index, TextPieceCharacters::Utf16(vec![*value, 0x000d]))),
        };
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
        let inserted_bytes = match inserted.to_bytes() {
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
        let empty = match marker_piece.value.characters {
            TextPieceCharacters::Compressed(_) => TextPieceCharacters::Compressed(Vec::new()),
            TextPieceCharacters::Utf16(_) => TextPieceCharacters::Utf16(Vec::new()),
        };
        let mut deleted = inserted_reopened.clone();
        deleted
            .replace_main_text_range_with_papx_runs(cp + 1..cp + 2, empty, source_runs.clone())
            .map_err(|error| error.to_string())?;
        let deleted_bytes = deleted.to_bytes().map_err(|error| error.to_string())?;
        let deleted_reopened =
            DocFile::from_bytes(&deleted_bytes).map_err(|error| error.to_string())?;
        if deleted_reopened.word_document.fib.rg_lw.ccp_text
            != file.word_document.fib.rg_lw.ccp_text
            || deleted_reopened.word_document.papx_runs != Some(source_runs.clone())
        {
            return Err("DOC paragraph deletion did not restore the explicit PAPX tree".to_owned());
        }
        let second_save = deleted_reopened
            .to_bytes()
            .map_err(|error| error.to_string())?;
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
        let candidate = match &piece.value.characters {
            TextPieceCharacters::Compressed(values) => values
                .get(first..last)
                .and_then(|range| {
                    range
                        .iter()
                        .enumerate()
                        .find(|(index, value)| **value >= 0x20 && first + index + 1 < last)
                })
                .map(|(offset, value)| {
                    (
                        first + offset,
                        TextPieceCharacters::Compressed(vec![*value, 0x0d]),
                    )
                }),
            TextPieceCharacters::Utf16(values) => values
                .get(first..last)
                .and_then(|range| {
                    range
                        .iter()
                        .enumerate()
                        .find(|(index, value)| **value >= 0x20 && first + index + 1 < last)
                })
                .map(|(offset, value)| {
                    (
                        first + offset,
                        TextPieceCharacters::Utf16(vec![*value, 0x000d]),
                    )
                }),
        };
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
        let inserted_bytes = match inserted.to_bytes() {
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
        let empty = match marker_piece.value.characters {
            TextPieceCharacters::Compressed(_) => TextPieceCharacters::Compressed(Vec::new()),
            TextPieceCharacters::Utf16(_) => TextPieceCharacters::Utf16(Vec::new()),
        };
        let mut deleted = inserted_reopened.clone();
        deleted
            .replace_text_range_with_papx_runs(
                part,
                local_cp + 1..local_cp + 2,
                empty,
                source_runs.clone(),
            )
            .map_err(|error| error.to_string())?;
        let deleted_bytes = deleted.to_bytes().map_err(|error| error.to_string())?;
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
        let second_save = deleted_reopened
            .to_bytes()
            .map_err(|error| error.to_string())?;
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

fn try_doc_chpx_tree_edit(file: &DocFile) -> Result<bool, String> {
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
    edited
        .word_document
        .chpx_runs
        .as_mut()
        .expect("CHPX CP tree was checked")[index]
        .properties = None;
    let bytes = match edited.to_bytes() {
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
    let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
    let second_reopen = DocFile::from_bytes(&second_save).map_err(|error| error.to_string())?;
    if second_reopen.word_document != reopened.word_document
        || second_reopen.table != reopened.table
    {
        return Err("DOC CHPX CP-tree edit was not stable after a second save".to_owned());
    }
    Ok(true)
}

fn try_doc_papx_tree_edit(file: &DocFile) -> Result<bool, String> {
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
    edited
        .word_document
        .papx_runs
        .as_mut()
        .expect("PAPX CP tree was checked")[index]
        .properties
        .as_mut()
        .expect("non-default PAPX was checked")
        .style_index = 0;
    let bytes = match edited.to_bytes() {
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
    let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
    let second_reopen = DocFile::from_bytes(&second_save).map_err(|error| error.to_string())?;
    if second_reopen.word_document != reopened.word_document
        || second_reopen.table != reopened.table
    {
        return Err("DOC PAPX CP-tree edit was not stable after a second save".to_owned());
    }
    Ok(true)
}

fn try_doc_paragraph_format_boundary_edit(file: &DocFile) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start_cp = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end_cp = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_start_fc = piece.value.file_offset;
        let width = match &piece.value.characters {
            TextPieceCharacters::Compressed(_) => 1u32,
            TextPieceCharacters::Utf16(_) => 2u32,
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
            let replacement = match &piece.value.characters {
                TextPieceCharacters::Compressed(values) => {
                    let Some(values) = values.get(first_index..first_index + 2) else {
                        continue;
                    };
                    if !matches!(values[0], 0x07 | 0x0c | 0x0d) {
                        continue;
                    }
                    TextPieceCharacters::Compressed(vec![b'X', values[0], values[1]])
                }
                TextPieceCharacters::Utf16(values) => {
                    let Some(values) = values.get(first_index..first_index + 2) else {
                        continue;
                    };
                    if !matches!(values[0], 0x0007 | 0x000c | 0x000d) {
                        continue;
                    }
                    TextPieceCharacters::Utf16(vec![u16::from(b'X'), values[0], values[1]])
                }
            };
            let mut edited = file.clone();
            if edited
                .replace_main_text_range(boundary_cp - 1..boundary_cp + 1, replacement)
                .is_err()
            {
                continue;
            }
            let bytes = match edited.to_bytes() {
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
            let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
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

fn try_doc_text_piece_boundary_edit(file: &DocFile) -> Result<bool, String> {
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
        let replacement = match &pieces[0].value.characters {
            TextPieceCharacters::Compressed(values) => {
                let Some(value) = values.last() else {
                    continue;
                };
                TextPieceCharacters::Compressed(vec![*value, b'X', b'Y'])
            }
            TextPieceCharacters::Utf16(values) => {
                let Some(value) = values.last() else {
                    continue;
                };
                TextPieceCharacters::Utf16(vec![*value, u16::from(b'X'), u16::from(b'Y')])
            }
        };
        let mut edited = file.clone();
        if edited
            .replace_main_text_range(boundary - 1..boundary + 1, replacement)
            .is_err()
        {
            continue;
        }
        let bytes = match edited.to_bytes() {
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
        let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
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

fn try_doc_text_piece_removal(file: &DocFile) -> Result<bool, String> {
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
        let mut terminators = Vec::new();
        let left_last = match &pieces[0].value.characters {
            TextPieceCharacters::Compressed(values) => values.last().copied().map(u16::from),
            TextPieceCharacters::Utf16(values) => values.last().copied(),
        };
        if left_last.is_some_and(|value| matches!(value, 0x0007 | 0x000c | 0x000d)) {
            terminators.push(left_last.expect("left text piece is nonempty"));
        }
        match &pieces[1].value.characters {
            TextPieceCharacters::Compressed(values) => terminators.extend(
                values
                    .iter()
                    .copied()
                    .map(u16::from)
                    .filter(|value| matches!(value, 0x0007 | 0x000c | 0x000d)),
            ),
            TextPieceCharacters::Utf16(values) => terminators.extend(
                values
                    .iter()
                    .copied()
                    .filter(|value| matches!(value, 0x0007 | 0x000c | 0x000d)),
            ),
        }
        let replacement = match &pieces[0].value.characters {
            TextPieceCharacters::Compressed(_) => TextPieceCharacters::Compressed(
                terminators
                    .iter()
                    .map(|value| u8::try_from(*value).expect("DOC terminator fits u8"))
                    .collect(),
            ),
            TextPieceCharacters::Utf16(_) => TextPieceCharacters::Utf16(terminators.clone()),
        };
        let replacement_len = u32::try_from(replacement.character_count())
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
        let bytes = match edited.to_bytes() {
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
        let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
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

fn try_doc_character_format_boundary_edit(file: &DocFile) -> Result<bool, String> {
    let main_len = u32::try_from(file.word_document.fib.rg_lw.ccp_text)
        .map_err(|_| "DOC ccpText is negative".to_owned())?;
    for piece in &file.word_document.text_pieces {
        let piece_start_cp = u32::try_from(piece.value.cp_start)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_end_cp = u32::try_from(piece.value.cp_end)
            .map_err(|_| "DOC text piece has a negative CP".to_owned())?;
        let piece_start_fc = piece.value.file_offset;
        let width = match &piece.value.characters {
            TextPieceCharacters::Compressed(_) => 1u32,
            TextPieceCharacters::Utf16(_) => 2u32,
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
            let replacement = match &piece.value.characters {
                TextPieceCharacters::Compressed(values) => {
                    let Some(values) = values.get(first_index..first_index + 2) else {
                        continue;
                    };
                    TextPieceCharacters::Compressed(vec![values[0], b'X', values[1]])
                }
                TextPieceCharacters::Utf16(values) => {
                    let Some(values) = values.get(first_index..first_index + 2) else {
                        continue;
                    };
                    TextPieceCharacters::Utf16(vec![values[0], u16::from(b'X'), values[1]])
                }
            };
            let mut edited = file.clone();
            if edited
                .replace_main_text_range(boundary_cp - 1..boundary_cp + 1, replacement)
                .is_err()
            {
                continue;
            }
            let bytes = match edited.to_bytes() {
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
            let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
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

fn try_doc_multiple_text_edits(file: &DocFile) -> Result<bool, String> {
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
        let candidates = match &piece.value.characters {
            TextPieceCharacters::Compressed(values) => values
                .iter()
                .take(limit)
                .enumerate()
                .filter(|(_, value)| **value >= 0x20)
                .take(2)
                .map(|(index, value)| (index, u16::from(*value)))
                .collect::<Vec<_>>(),
            TextPieceCharacters::Utf16(values) => values
                .iter()
                .take(limit)
                .enumerate()
                .filter(|(_, value)| **value >= 0x20)
                .take(2)
                .map(|(index, value)| (index, *value))
                .collect::<Vec<_>>(),
        };
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
        let (first_replacement, second_replacement) = match &piece.value.characters {
            TextPieceCharacters::Compressed(_) => (
                TextPieceCharacters::Compressed(vec![*first as u8, b'X']),
                TextPieceCharacters::Compressed(vec![*second as u8, b'Y', b'Z']),
            ),
            TextPieceCharacters::Utf16(_) => (
                TextPieceCharacters::Utf16(vec![*first, u16::from(b'X')]),
                TextPieceCharacters::Utf16(vec![*second, u16::from(b'Y'), u16::from(b'Z')]),
            ),
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
        let bytes = match edited.to_bytes() {
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
        let second_save = reopened.to_bytes().map_err(|error| error.to_string())?;
        let second_reopen = DocFile::from_bytes_compatible(&second_save)
            .map_err(|error| error.to_string())?
            .value;
        if second_reopen.word_document != reopened.word_document
            || second_reopen.table != reopened.table
        {
            return Err("multiple DOC text edits were not stable after a second save".to_owned());
        }

        let (first_replacement, empty_replacement) = match &piece.value.characters {
            TextPieceCharacters::Compressed(_) => (
                TextPieceCharacters::Compressed(vec![*first as u8, b'Q']),
                TextPieceCharacters::Compressed(Vec::new()),
            ),
            TextPieceCharacters::Utf16(_) => (
                TextPieceCharacters::Utf16(vec![*first, u16::from(b'Q')]),
                TextPieceCharacters::Utf16(Vec::new()),
            ),
        };
        let mut net_zero = file.clone();
        net_zero
            .replace_main_text_range(first_cp..first_cp + 1, first_replacement)
            .map_err(|error| error.to_string())?;
        net_zero
            .replace_main_text_range(second_cp..second_cp + 1, empty_replacement)
            .map_err(|error| error.to_string())?;
        let net_zero_bytes = net_zero.to_bytes().map_err(|error| error.to_string())?;
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
        let net_zero_second_save = net_zero_reopened
            .to_bytes()
            .map_err(|error| error.to_string())?;
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

fn try_doc_part_text_edit(file: &DocFile, target: FieldDocumentPart) -> Result<bool, String> {
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
                let candidate = match &piece.value.characters {
                    TextPieceCharacters::Compressed(values) => values
                        .get(first..last)
                        .and_then(|range| range.iter().position(|value| *value >= 0x20))
                        .map(|offset| {
                            let index = first + offset;
                            (
                                index,
                                TextPieceCharacters::Compressed(vec![values[index], b'X']),
                            )
                        }),
                    TextPieceCharacters::Utf16(values) => values
                        .get(first..last)
                        .and_then(|range| range.iter().position(|value| *value >= 0x20))
                        .map(|offset| {
                            let index = first + offset;
                            (
                                index,
                                TextPieceCharacters::Utf16(vec![values[index], u16::from(b'X')]),
                            )
                        }),
                };
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
                let Ok(bytes) = edited.to_bytes() else {
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

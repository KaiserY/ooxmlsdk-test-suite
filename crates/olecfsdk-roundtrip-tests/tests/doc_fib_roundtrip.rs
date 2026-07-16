use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    cfb::CompoundFile,
    doc::{
        AnnotationBookmarks, AnnotationExtendedData, AnnotationOwners, AnnotationReferenceTable,
        AssociatedStrings, AutoCaptionDefinitions, AutoSummaryDesiredSize, AutoSummaryRangeTable,
        AutoSummaryView, Bookmarks, CaptionDefinitions, ChpxFkp, Clx, CommandCustomizationRecord,
        CommandCustomizations, CpOnlyTable, CustomKinsokuLanguage, DocFile, DocOfficeArtContent,
        DocumentClassification, DocumentProperties, DocumentProtectionMode, EmbeddedFontSubset,
        EmbeddedFontTable, EmbeddedFontTableOffset, ExternalFileNameTable,
        FIB_LAST_SAVED_FILETIME_INDEX, Fib, FibBase, FibBaseFlags, FieldDocumentPart, FontTable,
        FormatConsistencyBookmarks, FrameAndListRecord, FrameAndListRecords,
        GrammarCheckerCookieTable, GrammarCookieErrorType, GrammarCookieStore, GrammarOptionSets,
        GrammarStateKind, GrammarStateTable, GridDisplayFrequency, HeaderStoryBoundary,
        HeaderTextTable, HtmlBlockType, KinsokuLevel, LanguageDetectionStateKind,
        LanguageDetectionStateTable, LegacyGrammarCheckerCookieTable, LegacyGrammarOptionSets,
        ListDefinitions, ListLevelTemplateCode, ListNamesTable, ListOverrides, ListStyleTemplates,
        MailMergeDestination, MailMergeDocumentType, MailMergeErrorHandling,
        MailMergeFileReference, MailMergeSourceKind, MailMergeState, MathBinaryOperatorBreak,
        MathBinarySubtractionBreak, MathFixedConstants, MathJustification, NoteReferenceTable,
        OfficeDataSource, OleControlDocumentPart, OleControlInfos, OleObjectDescriptor, PapxFkp,
        PapxLengthEncoding, ParagraphGroupProperties, ParagraphIdentifierContext, PlcBte, PlcfSed,
        PrinterDriverInfo, Prm, PropertyBagString, ProtectedUsers, RangeProtection,
        RepairBookmarks, RevisionAuthors, RevisionMessageThreading, RevisionSaveIdTable,
        SaveHistory, SavedOutlineLevel, SavedViewKind, SelectionRange, SelectionState,
        SelectionStateExtension, SelectionStyle, Sepx, ShapeAnchorTable, SmartTagBookmarks,
        SmartTagData, SmartTagFactoidTypeId, SmartTagRecognizerStateKind,
        SmartTagRecognizerStateTable, SmartTagSource, SpellingStateKind, SpellingStateTable,
        SprmGroup, SprmKind, SprmOperand, StructuredTagBookmarks, StyleFormatting, StyleKind,
        StyleSheet, StyleSortMethod, SubdocumentTable, TableCharacterCacheTable, TextLineEnding,
        TextPieceCharacters, TextboxBreakTable, TextboxDocumentPart, TextboxStoryChain,
        TextboxStoryTable, TypographyJustification, UserInputMethods, UserVariableKind,
        UserVariables, WORD97_FILE_IDENTIFIER, WebTargetScreenSize, XmlSchemaReferences,
        XmlSchemaStringTable, XmlTransformPath,
    },
    forms::{CommandButtonControl, FmStringLengthMode, MorphDataControl, SingleStreamOleControl},
    office_art::{OfficeArtDrawingGraphIssue, OfficeArtRecordData, OfficeArtShapeFlags},
    shared::{
        EnvelopeFlagStatus, EnvelopeImportance, EnvelopeRecipientPropertyValue,
        EnvelopeSensitivity, MsoEnvelopeClsid, MsoEnvelopeData, MsoEnvelopeVersion,
    },
};
use olecfsdk_corpus_test_support::{
    corpus_bytes,
    manifest::{ExpectationMode, read_manifest},
};

#[derive(Debug, Default)]
struct OfficeArtDrawingGraphAudit {
    graphs: usize,
    partial_graphs: usize,
    typed_graphs: usize,
    strict_graphs: usize,
    compatibility_graphs: usize,
    dgg_records: usize,
    missing_or_multiple_dgg: usize,
    drawings: usize,
    fdg_records: usize,
    missing_or_multiple_fdg: usize,
    shapes: usize,
    patriarch_shapes: usize,
    deleted_shapes: usize,
    duplicate_drawing_ids: usize,
    duplicate_shape_ids: usize,
    fdg_shape_count_deltas: BTreeMap<i64, usize>,
    fdg_shape_count_without_patriarch_deltas: BTreeMap<i64, usize>,
    fdg_current_shape_relations: BTreeMap<&'static str, usize>,
    dgg_saved_shape_vs_fdg_deltas: BTreeMap<i64, usize>,
    dgg_saved_shape_vs_fsp_deltas: BTreeMap<i64, usize>,
    dgg_saved_drawing_deltas: BTreeMap<i64, usize>,
    dgg_max_shape_relations: BTreeMap<&'static str, usize>,
    dgg_max_shape_deltas: BTreeMap<i64, usize>,
    shape_cluster_zero: usize,
    shape_cluster_missing: usize,
    shape_cluster_drawing_mismatches: usize,
    cluster_without_current_shape: usize,
    cluster_cursor_relations: BTreeMap<&'static str, usize>,
    cluster_cursor_deltas: BTreeMap<i64, usize>,
    blip_stores: usize,
    blip_entries: usize,
    blip_references: usize,
    blip_reference_count_relations: BTreeMap<String, usize>,
    blip_store_entry_count_mismatches: usize,
    blip_store_entry_count_mismatch_shapes: BTreeMap<(u16, usize), usize>,
    blip_references_out_of_range: usize,
    empty_blip_store_slots_referenced: usize,
}

impl OfficeArtDrawingGraphAudit {
    fn audit(&mut self, content: &DocOfficeArtContent) {
        self.graphs += 1;
        if content.drawing_group.is_partial()
            || content
                .drawings
                .iter()
                .any(|drawing| drawing.container.is_partial())
        {
            self.partial_graphs += 1;
            return;
        }
        let graph = content
            .drawing_graph()
            .expect("complete DOC OfficeArt trees build a drawing graph");
        self.typed_graphs += 1;
        if graph.validate_strict().is_ok() {
            self.strict_graphs += 1;
        } else {
            self.compatibility_graphs += 1;
        }
        self.blip_references += graph.blip_references.len();
        if let Some(store) = &graph.blip_store {
            self.blip_stores += 1;
            self.blip_entries += store.entries.len();
            for entry in &store.entries {
                if let Some(relation) = entry.reference_count_relation {
                    *self
                        .blip_reference_count_relations
                        .entry(format!("{relation:?}"))
                        .or_default() += 1;
                }
            }
        }
        for issue in &graph.issues {
            match issue {
                OfficeArtDrawingGraphIssue::BlipStoreEntryCountMismatch { declared, actual } => {
                    self.blip_store_entry_count_mismatches += 1;
                    *self
                        .blip_store_entry_count_mismatch_shapes
                        .entry((*declared, *actual))
                        .or_default() += 1;
                }
                OfficeArtDrawingGraphIssue::BlipReferenceOutOfRange { .. } => {
                    self.blip_references_out_of_range += 1;
                }
                OfficeArtDrawingGraphIssue::EmptyBlipStoreSlotReferenced { .. } => {
                    self.empty_blip_store_slots_referenced += 1;
                }
                _ => {}
            }
        }

        let mut dgg_records = Vec::new();
        content.drawing_group.visit_complete(|record| {
            if let OfficeArtRecordData::DggBlock(value) = &record.data {
                dgg_records.push(value.clone());
            }
        });
        self.dgg_records += dgg_records.len();
        if dgg_records.len() != 1 {
            self.missing_or_multiple_dgg += 1;
            return;
        }
        let dgg = &dgg_records[0];

        let mut drawing_ids = BTreeSet::new();
        let mut shape_ids = BTreeSet::new();
        let mut shapes_by_drawing = Vec::new();
        let mut fdg_shape_total = 0usize;
        for drawing in &content.drawings {
            self.drawings += 1;
            let mut fdg_records = Vec::new();
            let mut shapes = Vec::new();
            drawing
                .container
                .visit_complete(|record| match &record.data {
                    OfficeArtRecordData::Drawing(value) => {
                        fdg_records.push((record.header.instance, *value));
                    }
                    OfficeArtRecordData::Shape(value) => shapes.push(*value),
                    _ => {}
                });
            self.fdg_records += fdg_records.len();
            if fdg_records.len() != 1 {
                self.missing_or_multiple_fdg += 1;
                continue;
            }
            let (drawing_id, fdg) = fdg_records[0];
            self.duplicate_drawing_ids += usize::from(!drawing_ids.insert(drawing_id));
            self.shapes += shapes.len();
            let patriarch_count = shapes
                .iter()
                .filter(|shape| shape.flags.contains(OfficeArtShapeFlags::PATRIARCH))
                .count();
            self.patriarch_shapes += patriarch_count;
            self.deleted_shapes += shapes
                .iter()
                .filter(|shape| shape.flags.contains(OfficeArtShapeFlags::DELETED))
                .count();
            self.duplicate_shape_ids += shapes
                .iter()
                .filter(|shape| !shape_ids.insert(shape.shape_id))
                .count();
            *self
                .fdg_shape_count_deltas
                .entry(signed_delta(fdg.shape_count, shapes.len()))
                .or_default() += 1;
            *self
                .fdg_shape_count_without_patriarch_deltas
                .entry(signed_delta(
                    fdg.shape_count,
                    shapes.len().saturating_sub(patriarch_count),
                ))
                .or_default() += 1;
            let max_shape_id = shapes.iter().map(|shape| shape.shape_id).max();
            *self
                .fdg_current_shape_relations
                .entry(relation_to_optional(fdg.current_shape_id, max_shape_id))
                .or_default() += 1;
            fdg_shape_total = fdg_shape_total.saturating_add(fdg.shape_count as usize);
            shapes_by_drawing.push((drawing_id, shapes));
        }

        *self
            .dgg_saved_shape_vs_fdg_deltas
            .entry(signed_delta(dgg.saved_shape_count, fdg_shape_total))
            .or_default() += 1;
        *self
            .dgg_saved_shape_vs_fsp_deltas
            .entry(signed_delta(dgg.saved_shape_count, shape_ids.len()))
            .or_default() += 1;
        *self
            .dgg_saved_drawing_deltas
            .entry(signed_delta(
                dgg.saved_drawing_count,
                content.drawings.len(),
            ))
            .or_default() += 1;
        let max_shape_id = shape_ids.iter().copied().max();
        *self
            .dgg_max_shape_relations
            .entry(relation_to_optional(dgg.maximum_shape_id, max_shape_id))
            .or_default() += 1;
        if let Some(max_shape_id) = max_shape_id {
            *self
                .dgg_max_shape_deltas
                .entry(i64::from(dgg.maximum_shape_id) - i64::from(max_shape_id))
                .or_default() += 1;
        }

        let mut cluster_max_offsets = vec![None::<u32>; dgg.clusters.len()];
        for (drawing_id, shapes) in &shapes_by_drawing {
            for shape in shapes {
                let cluster_number = shape.shape_id / 0x400;
                if cluster_number == 0 {
                    self.shape_cluster_zero += 1;
                    continue;
                }
                let cluster_index = usize::try_from(cluster_number - 1).unwrap_or(usize::MAX);
                let Some(cluster) = dgg.clusters.get(cluster_index) else {
                    self.shape_cluster_missing += 1;
                    continue;
                };
                self.shape_cluster_drawing_mismatches +=
                    usize::from(cluster.drawing_id != u32::from(*drawing_id));
                let local_offset = shape.shape_id % 0x400;
                let max_offset = &mut cluster_max_offsets[cluster_index];
                *max_offset =
                    Some(max_offset.map_or(local_offset, |value| value.max(local_offset)));
            }
        }
        for (cluster, max_offset) in dgg.clusters.iter().zip(cluster_max_offsets) {
            let Some(max_offset) = max_offset else {
                self.cluster_without_current_shape += 1;
                continue;
            };
            let expected_cursor = max_offset + 1;
            *self
                .cluster_cursor_relations
                .entry(relation(cluster.current_shape_id_count, expected_cursor))
                .or_default() += 1;
            *self
                .cluster_cursor_deltas
                .entry(i64::from(cluster.current_shape_id_count) - i64::from(expected_cursor))
                .or_default() += 1;
        }
    }
}

fn signed_delta(declared: u32, actual: usize) -> i64 {
    i64::from(declared) - i64::try_from(actual).unwrap_or(i64::MAX)
}

fn relation_to_optional(value: u32, expected: Option<u32>) -> &'static str {
    match expected {
        Some(expected) => relation(value, expected),
        None if value == 0 => "empty-zero",
        None => "empty-nonzero",
    }
}

fn relation(value: u32, expected: u32) -> &'static str {
    match value.cmp(&expected) {
        std::cmp::Ordering::Less => "below",
        std::cmp::Ordering::Equal => "equal",
        std::cmp::Ordering::Greater => "above",
    }
}

#[test]
#[ignore = "DOC FIB corpus round-trip runs explicitly"]
fn legacy_word_fibs_round_trip() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);
    let exclusions = excluded_files(&corpus);
    let atrd_extra_exclusions = excluded_files_for_test(&corpus, "doc_atrd_extra_roundtrip");
    let plcf_wkb_exclusions = excluded_files_for_test(&corpus, "doc_plcf_wkb_roundtrip");
    let mut observed_exclusions = BTreeSet::new();
    let mut observed_atrd_extra_exclusions = BTreeSet::new();
    let mut observed_plcf_wkb_exclusions = BTreeSet::new();

    let mut checked = 0usize;
    let mut legacy = BTreeMap::<u16, usize>::new();
    let mut versions = BTreeMap::<u16, usize>::new();
    let mut fc_lcb_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut csw_new_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut nonzero_fib_pairs = BTreeMap::<usize, usize>::new();
    let mut zero_last_saved_file_times = 0usize;
    let mut nonzero_last_saved_file_times = 0usize;
    let mut distinct_last_saved_file_times = BTreeSet::<u64>::new();
    let mut minimum_last_saved_file_time = u64::MAX;
    let mut maximum_last_saved_file_time = 0u64;
    let mut last_saved_file_time_part_mismatches = 0usize;
    let mut last_saved_high_zero_nonzero = 0usize;
    let mut table0 = 0usize;
    let mut table1 = 0usize;
    let mut encrypted_exclusions = 0usize;
    let mut invalid_exclusions = 0usize;
    let mut clx_count = 0usize;
    let mut property_runs = 0usize;
    let mut pieces = 0usize;
    let mut compressed_pieces = 0usize;
    let mut simple_property_modifiers = 0usize;
    let mut complex_property_modifiers = 0usize;
    let mut prl_count = 0usize;
    let mut sprm_opcodes = BTreeMap::<u16, usize>::new();
    let mut sprm_groups = BTreeMap::<u8, usize>::new();
    let mut sprm_operand_shapes = BTreeMap::<&'static str, usize>::new();
    let mut variable_operand_bytes = 0usize;
    let mut unknown_sprm_kinds = BTreeSet::<u16>::new();
    let mut text_characters = 0usize;
    let mut compressed_text_bytes = 0usize;
    let mut utf16_text_units = 0usize;
    let mut chpx_bte_count = 0usize;
    let mut chpx_pages = 0usize;
    let mut chpx_runs = 0usize;
    let mut chpx_default_runs = 0usize;
    let mut chpx_unused_bytes = 0usize;
    let mut chpx_prls = 0usize;
    let mut chpx_unknown_sprms = BTreeSet::<u16>::new();
    let mut chpx_sprm_frequencies = BTreeMap::<u16, usize>::new();
    let mut chpx_raw_variable_operands = 0usize;
    let mut chpx_raw_variable_frequencies = BTreeMap::<u16, usize>::new();
    let mut chpx_static_variable_operands = BTreeMap::<&'static str, usize>::new();
    let mut papx_bte_count = 0usize;
    let mut papx_pages = 0usize;
    let mut papx_runs = 0usize;
    let mut papx_default_runs = 0usize;
    let mut papx_prls = 0usize;
    let mut papx_unknown_sprms = BTreeSet::<u16>::new();
    let mut papx_sprm_frequencies = BTreeMap::<u16, usize>::new();
    let mut papx_raw_variable_operands = 0usize;
    let mut papx_raw_variable_frequencies = BTreeMap::<u16, usize>::new();
    let mut papx_static_variable_operands = BTreeMap::<&'static str, usize>::new();
    let mut papx_short_lengths = 0usize;
    let mut papx_extended_lengths = 0usize;
    let mut papx_unused_bytes = 0usize;
    let mut papx_trailing_bytes = BTreeMap::<u8, usize>::new();
    let mut section_tables = 0usize;
    let mut sections = 0usize;
    let mut default_sections = 0usize;
    let mut sepx_count = 0usize;
    let mut sepx_prls = 0usize;
    let mut sepx_unknown_sprms = BTreeSet::<u16>::new();
    let mut sepx_unknown_fixed_shapes = BTreeMap::<(u16, u32), usize>::new();
    let mut sepx_raw_variable_operands = 0usize;
    let mut sepx_raw_variable_frequencies = BTreeMap::<u16, usize>::new();
    let mut sepx_raw_variable_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut sepx_static_variable_operands = BTreeMap::<&'static str, usize>::new();
    let mut outline_list_restart_values = BTreeMap::<u8, usize>::new();
    let mut outline_list_reserved_shapes = BTreeSet::<[u8; 3]>::new();
    let mut outline_list_nonzero_text_units = 0usize;
    let mut section_header_footer_flag_shapes = BTreeMap::<u8, usize>::new();
    let mut sepx_trailing_bytes = BTreeMap::<u8, usize>::new();
    let mut style_sheets = 0usize;
    let mut style_sheet_info_shapes = BTreeMap::<(usize, u16), usize>::new();
    let mut styles = 0usize;
    let mut empty_styles = 0usize;
    let mut style_definition_bytes = 0usize;
    let mut style_name_units = 0usize;
    let mut revision_marked_styles = 0usize;
    let mut style_upx_prls = 0usize;
    let mut style_upx_padding = BTreeMap::<u8, usize>::new();
    let mut style_upx_index_mismatches = BTreeMap::<(u16, u16), usize>::new();
    let mut style_upx_unknown_sprms = BTreeSet::<u16>::new();
    let mut style_upx_raw_variable_operands = 0usize;
    let mut style_upx_raw_variable_frequencies = BTreeMap::<u16, usize>::new();
    let mut style_upx_raw_variable_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut style_upx_static_variable_operands = BTreeMap::<&'static str, usize>::new();
    let mut style_kind_counts = BTreeMap::<StyleKind, usize>::new();
    let mut style_cupx_shapes = BTreeMap::<(StyleKind, u8, bool), usize>::new();
    let mut latent_style_entries = 0usize;
    let mut standard_style_prls = 0usize;
    let mut style_alignment_padding = BTreeMap::<u8, usize>::new();
    let mut field_tables = BTreeMap::<FieldDocumentPart, usize>::new();
    let mut field_records = 0usize;
    let mut field_character_counts = BTreeMap::<(FieldDocumentPart, u8), usize>::new();
    let mut field_reserved_counts = BTreeMap::<u8, usize>::new();
    let mut field_type_counts = BTreeMap::<u8, usize>::new();
    let mut bookmark_sets = 0usize;
    let mut bookmarks_count = 0usize;
    let mut bookmark_name_units = 0usize;
    let mut hidden_bookmarks = 0usize;
    let mut column_bookmarks = 0usize;
    let mut header_tables = 0usize;
    let mut header_boundaries = 0usize;
    let mut missing_header_boundaries = 0usize;
    let mut footnote_sets = 0usize;
    let mut footnote_references = 0usize;
    let mut footnote_custom_references = 0usize;
    let mut endnote_sets = 0usize;
    let mut endnote_references = 0usize;
    let mut endnote_custom_references = 0usize;
    let mut annotation_sets = 0usize;
    let mut annotation_references = 0usize;
    let mut annotation_initial_units = 0usize;
    let mut annotation_empty_range_tags = 0usize;
    let mut annotation_unused_words = BTreeMap::<(u16, u16), usize>::new();
    let mut annotation_extended_tables = 0usize;
    let mut annotation_extended_records = 0usize;
    let mut annotation_extended_count_mismatches = 0usize;
    let mut annotation_extended_depths = BTreeMap::<u32, usize>::new();
    let mut annotation_extended_parent_offsets = BTreeMap::<i32, usize>::new();
    let mut annotation_extended_ink = 0usize;
    let mut annotation_extended_ows = 0usize;
    let mut annotation_extended_nonzero_padding1 = 0usize;
    let mut annotation_extended_nonzero_padding2 = 0usize;
    let mut annotation_extended_zero_dates = 0usize;
    let mut user_input_method_tables = 0usize;
    let mut user_input_methods = 0usize;
    let mut user_input_method_guids = 0usize;
    let mut user_input_method_service_bytes = 0usize;
    let mut user_input_method_empty_service_data = 0usize;
    let mut user_input_method_duplicate_positions = 0usize;
    let mut user_input_method_descending_positions = 0usize;
    let mut user_input_method_negative_character_counts = 0usize;
    let mut user_input_method_nonzero_private_data = 0usize;
    let mut user_input_method_reference_pairs = BTreeMap::<(i16, i16), usize>::new();
    let mut user_input_method_character_counts = BTreeMap::<i32, usize>::new();
    let mut user_input_method_service_sizes = BTreeMap::<u32, usize>::new();
    let mut user_input_method_private_values = BTreeMap::<u32, usize>::new();
    let mut user_input_method_guid_values = BTreeSet::<[u8; 16]>::new();
    let mut mso_envelope_tables = 0usize;
    let mut mso_envelope_typed = 0usize;
    let mut mso_envelope_out_of_scope = 0usize;
    let mut mso_envelope_versions = BTreeMap::<MsoEnvelopeVersion, usize>::new();
    let mut mso_envelope_subject_units = 0usize;
    let mut mso_envelope_recipients = 0usize;
    let mut mso_envelope_recipient_properties = 0usize;
    let mut mso_envelope_property_types = BTreeMap::<&'static str, usize>::new();
    let mut mso_envelope_attachments = 0usize;
    let mut mso_envelope_attachment_bytes = 0usize;
    let mut mso_envelope_attachment_name_units = 0usize;
    let mut mso_envelope_attachment_methods = BTreeMap::<u32, usize>::new();
    let mut mso_envelope_intro_units = 0usize;
    let mut mso_envelope_shapes = BTreeMap::<
        (
            EnvelopeFlagStatus,
            EnvelopeSensitivity,
            EnvelopeImportance,
            u32,
            bool,
            bool,
            bool,
        ),
        usize,
    >::new();
    let mut printer_driver_info_tables = 0usize;
    let mut printer_driver_info_total_bytes = 0usize;
    let mut printer_driver_info_empty_fields = 0usize;
    let mut printer_driver_info_length_shapes =
        BTreeMap::<(usize, usize, usize, usize), usize>::new();
    let mut printer_driver_names = BTreeSet::<Vec<u8>>::new();
    let mut printer_port_names = BTreeSet::<Vec<u8>>::new();
    let mut printer_driver_file_names = BTreeSet::<Vec<u8>>::new();
    let mut printer_product_names = BTreeSet::<Vec<u8>>::new();
    let mut ole_control_info_tables = 0usize;
    let mut ole_control_infos = 0usize;
    let mut ole_control_cookie_index_mismatches = 0usize;
    let mut ole_control_duplicate_cookies = 0usize;
    let mut ole_control_field_reference_mismatches = 0usize;
    let mut ole_control_document_parts = BTreeMap::<OleControlDocumentPart, usize>::new();
    let mut ole_control_nonzero_accelerator_handles = 0usize;
    let mut ole_control_nonzero_accelerator_counts = 0usize;
    let mut ole_control_unlinked_fields = 0usize;
    let mut ole_control_compatibility_document_parts = 0usize;
    let mut ole_control_failed_load = 0usize;
    let mut ole_control_corrupt = 0usize;
    let mut ole_control_behavior_flags = BTreeMap::<(bool, bool, bool, bool, bool), usize>::new();
    let mut ole_control_nonzero_reserved1 = 0usize;
    let mut ole_control_nonzero_reserved2 = 0usize;
    let mut ole_object_descriptors = 0usize;
    let mut ole_object_descriptor_shapes = BTreeMap::<(u16, u16, Option<u16>, usize), usize>::new();
    let mut ole_object_control_descriptors = 0usize;
    let mut ole_object_control_streams = 0usize;
    let mut ole_object_control_missing_payloads = 0usize;
    let mut ole_object_control_storage_shapes = BTreeMap::<(bool, Vec<String>), usize>::new();
    let mut ole_object_control_classes =
        BTreeMap::<(String, bool), (usize, BTreeSet<usize>)>::new();
    let mut morph_data_controls = 0usize;
    let mut morph_data_shapes = BTreeMap::<(String, u64, usize, usize), usize>::new();
    let mut morph_data_text_props_masks = BTreeMap::<u32, usize>::new();
    let mut morph_data_low_word_compatibility_strings = 0usize;
    let mut command_button_controls = 0usize;
    let mut command_button_shapes = BTreeMap::<(u32, u32, usize), usize>::new();
    let mut single_stream_ole_controls = 0usize;
    let mut single_stream_ole_control_shapes = BTreeMap::<(String, usize, bool), usize>::new();
    let mut annotation_owner_sets = 0usize;
    let mut annotation_owners = 0usize;
    let mut annotation_owner_name_units = 0usize;
    let mut annotation_bookmark_sets = 0usize;
    let mut annotation_bookmarks = 0usize;
    let mut textbox_story_sets = BTreeMap::<TextboxDocumentPart, usize>::new();
    let mut textbox_stories = 0usize;
    let mut reusable_textbox_stories = 0usize;
    let mut textbox_break_sets = BTreeMap::<TextboxDocumentPart, usize>::new();
    let mut textbox_breaks = 0usize;
    let mut textbox_overflows = 0usize;
    let mut shape_anchor_sets = BTreeMap::<TextboxDocumentPart, usize>::new();
    let mut shape_anchors = 0usize;
    let mut below_text_shapes = 0usize;
    let mut locked_shape_anchors = 0usize;
    let mut textbox_stories_without_anchor = 0usize;
    let mut office_art_contents = 0usize;
    let mut office_art_drawings = BTreeMap::<TextboxDocumentPart, usize>::new();
    let mut office_art_records = 0usize;
    let mut office_art_atom_bytes = 0usize;
    let mut office_art_atom_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut office_art_partial_trees = 0usize;
    let mut office_art_graph_audit = OfficeArtDrawingGraphAudit::default();
    let mut word_client_anchors = 0usize;
    let mut word_client_data = 0usize;
    let mut word_client_textboxes = 0usize;
    let mut word_client_anchor_invalid_indexes = 0usize;
    let mut word_client_textbox_invalid_indexes = 0usize;
    let mut list_definition_sets = 0usize;
    let mut list_definitions = 0usize;
    let mut simple_list_definitions = 0usize;
    let mut list_levels = 0usize;
    let mut list_level_paragraph_prls = 0usize;
    let mut list_level_character_prls = 0usize;
    let mut list_level_text_units = 0usize;
    let mut list_level_bytes = 0usize;
    let mut list_level_to_override_gaps = BTreeMap::<i64, usize>::new();
    let mut list_levels_in_declared_length = 0usize;
    let mut list_level_incomplete_tails = BTreeMap::<(&'static str, usize), usize>::new();
    let mut list_name_tables = 0usize;
    let mut list_name_entries = 0usize;
    let mut nonempty_list_names = 0usize;
    let mut list_name_units = 0usize;
    let mut maximum_list_name_length = 0usize;
    let mut list_name_count_shapes = BTreeMap::<usize, usize>::new();
    let mut list_name_definition_count_differences = BTreeMap::<i64, usize>::new();
    let mut list_override_sets = 0usize;
    let mut list_overrides = 0usize;
    let mut list_override_levels = 0usize;
    let mut formatted_list_override_levels = 0usize;
    let mut list_override_level_prls = 0usize;
    let mut list_override_text_units = 0usize;
    let mut list_override_missing_definitions = 0usize;
    let mut document_property_shapes = BTreeMap::<(u16, u32), usize>::new();
    let mut document_typography_shapes = BTreeMap::<
        (
            TypographyJustification,
            KinsokuLevel,
            CustomKinsokuLanguage,
            bool,
            bool,
            bool,
            bool,
            u16,
            u16,
        ),
        usize,
    >::new();
    let mut document_typography_following_units = 0usize;
    let mut document_typography_leading_units = 0usize;
    let mut document_typography_nonzero_unused_following_slots = 0usize;
    let mut document_typography_nonzero_unused_leading_slots = 0usize;
    let mut document_compatibility_option_shapes = BTreeMap::<(u16, u32), usize>::new();
    let mut document_compatibility_option_mismatches = BTreeMap::<u16, usize>::new();
    let mut document_format_flag_shapes = BTreeMap::<u8, usize>::new();
    let mut document_footnote_numbering_shapes = BTreeMap::<u16, usize>::new();
    let mut document_state_flag_shapes = BTreeMap::<u32, usize>::new();
    let mut document_endnote_numbering_shapes = BTreeMap::<u16, usize>::new();
    let mut document_endnote_option_shapes = BTreeMap::<u16, usize>::new();
    let mut document_saved_view_shapes = BTreeMap::<u16, usize>::new();
    let mut document_saved_view_kinds = BTreeMap::<SavedViewKind, usize>::new();
    let mut document_saved_zoom_kinds = BTreeMap::<u8, usize>::new();
    let mut document_saved_zoom_percentages = BTreeSet::<u16>::new();
    let mut document_display_flag_shapes = BTreeMap::<u16, usize>::new();
    let mut document_outline_levels = BTreeMap::<SavedOutlineLevel, usize>::new();
    let mut document_version_flag_shapes = BTreeMap::<u16, usize>::new();
    let mut document_event_shapes = BTreeMap::<u32, usize>::new();
    let mut document_virus_flag_shapes = BTreeMap::<(bool, bool), usize>::new();
    let mut document_virus_session_keys = BTreeSet::<u32>::new();
    let mut document_drawing_grid_shapes = BTreeMap::<
        (
            u16,
            u16,
            u16,
            u16,
            GridDisplayFrequency,
            bool,
            GridDisplayFrequency,
            bool,
        ),
        usize,
    >::new();
    let mut document_2000_extensions = 0usize;
    let mut document_2000_level_shapes = BTreeMap::<(u8, u8), usize>::new();
    let mut document_2000_flag_shapes = BTreeMap::<u32, usize>::new();
    let mut document_2000_screen_sizes = BTreeMap::<WebTargetScreenSize, usize>::new();
    let mut document_2000_initialized_web_options = 0usize;
    let mut document_2000_initialized_ppi = BTreeMap::<u16, usize>::new();
    let mut document_copts_named_shapes = BTreeMap::<u32, usize>::new();
    let mut document_copts_cached_column_balance = 0usize;
    let mut document_copts_nonzero_empty1 = 0usize;
    let mut document_copts_nonzero_empty_dwords = 0usize;
    let mut document_copts_word8_mismatches = 0usize;
    let mut document_2000_pre_word10_shapes = BTreeMap::<u16, usize>::new();
    let mut document_2000_flag2_shapes = BTreeMap::<u16, usize>::new();
    let mut document_2002_extensions = 0usize;
    let mut document_2002_flag_shapes = BTreeMap::<u16, usize>::new();
    let mut document_2002_line_endings = BTreeMap::<TextLineEnding, usize>::new();
    let mut document_2002_feature_shapes = BTreeMap::<u16, usize>::new();
    let mut document_2002_default_table_styles = BTreeSet::<u16>::new();
    let mut document_2002_style_filters = BTreeMap::<u16, usize>::new();
    let mut document_2002_booklet_pages = BTreeMap::<u16, usize>::new();
    let mut document_2002_code_pages = BTreeMap::<u32, usize>::new();
    let mut document_2002_nonzero_unused = 0usize;
    let mut document_2002_nonzero_revision_positions = [0usize; 7];
    let mut document_2002_maximum_revision_positions = [0u32; 7];
    let mut document_2002_nonzero_root_revision_ids = 0usize;
    let mut document_2002_root_revision_ids = BTreeSet::<u32>::new();
    let mut document_2003_extensions = 0usize;
    let mut document_2003_flag_shapes = BTreeMap::<u32, usize>::new();
    let mut document_2003_protection_shapes = BTreeMap::<u16, usize>::new();
    let mut document_2003_protection_modes = BTreeMap::<DocumentProtectionMode, usize>::new();
    let mut document_2003_page_widths = BTreeMap::<u32, usize>::new();
    let mut document_2003_page_heights = BTreeMap::<u32, usize>::new();
    let mut document_2003_font_percentages = BTreeMap::<u32, usize>::new();
    let mut document_2003_toolbar_shapes = BTreeMap::<u8, usize>::new();
    let mut document_2003_cleanup_limits = BTreeMap::<u16, usize>::new();
    let mut document_2007_extensions = 0usize;
    let mut document_2007_reserved_values = BTreeMap::<u32, usize>::new();
    let mut document_2007_flag_shapes = BTreeMap::<u32, usize>::new();
    let mut document_2007_style_sort_methods = BTreeMap::<StyleSortMethod, usize>::new();
    let mut document_math_flag_shapes = BTreeMap::<u32, usize>::new();
    let mut document_math_enum_shapes = BTreeMap::<
        (
            MathBinaryOperatorBreak,
            MathBinarySubtractionBreak,
            MathJustification,
        ),
        usize,
    >::new();
    let mut document_math_fixed_constants = BTreeMap::<MathFixedConstants, usize>::new();
    let mut document_math_font_indexes = BTreeMap::<u16, usize>::new();
    let mut document_math_left_margins = BTreeMap::<i32, usize>::new();
    let mut document_math_right_margins = BTreeMap::<i32, usize>::new();
    let mut document_math_wrapped_indents = BTreeMap::<i32, usize>::new();
    let mut document_2010_extensions = 0usize;
    let mut document_2010_compatibility_zero_contexts = 0usize;
    let mut document_2010_standard_contexts = BTreeSet::<u32>::new();
    let mut document_2010_reserved_values = BTreeMap::<u32, usize>::new();
    let mut document_2010_discard_image_data = BTreeMap::<bool, usize>::new();
    let mut document_2010_image_resolutions = BTreeMap::<u32, usize>::new();
    let mut document_2013_chart_tracking = BTreeMap::<bool, usize>::new();
    let mut document_classifications = BTreeMap::<DocumentClassification, usize>::new();
    let mut document_undefined_space_shapes = BTreeSet::<[u8; 30]>::new();
    let mut document_nonzero_undefined_spaces = 0usize;
    let mut document_nonzero_undefined_space_bytes = 0usize;
    let mut document_last_list_indexes = BTreeSet::<(u16, u16)>::new();
    let mut document_nonzero_last_list_indexes = 0usize;
    let mut document_last_list_index_matches = 0usize;
    let mut document_last_list_index_mismatches = BTreeMap::<(u16, u16, usize), usize>::new();
    let mut document_cleanup_limit_matches = 0usize;
    let mut document_cleanup_limit_mismatches = BTreeMap::<(u16, usize), usize>::new();
    let mut document_numbering_cache_states = BTreeMap::<(bool, bool), usize>::new();
    let mut document_numbering_cache_lengths = BTreeMap::<u32, usize>::new();
    let mut document_numbering_cache_present_max_positions = BTreeSet::<i32>::new();
    let mut document_numbering_cache_absent_max_positions = BTreeSet::<i32>::new();
    let mut document_note_number_formats = BTreeMap::<(u8, u8), usize>::new();
    let mut document_pagination_display_shapes = BTreeMap::<(u16, u16), usize>::new();
    let mut document_characters_with_spaces_shapes = BTreeSet::<(i32, i32)>::new();
    let mut document_double_byte_character_shapes = BTreeSet::<(i32, i32)>::new();
    let mut document_negative_character_count_pairs = 0usize;
    let mut document_character_count_relation_mismatches = 0usize;
    let mut document_character_statistic_states =
        BTreeMap::<(bool, bool, bool, bool), usize>::new();
    let mut document_main_statistic_shapes = BTreeSet::<(i32, i32, i16, i32, i32)>::new();
    let mut document_subdocument_statistic_shapes = BTreeSet::<(i32, i32, i16, i32, i32)>::new();
    let mut document_negative_statistics = 0usize;
    let mut document_statistic_relation_mismatches = 0usize;
    let mut document_statistic_states = BTreeMap::<(bool, bool, bool), usize>::new();
    let mut document_exact_statistic_character_bound_mismatches = 0usize;
    let mut document_created_timestamps = BTreeSet::new();
    let mut document_revised_timestamps = BTreeSet::new();
    let mut document_last_printed_timestamps = BTreeSet::new();
    let mut document_ignored_timestamp_counts = [0usize; 3];
    let mut document_revision_counts = BTreeSet::<i16>::new();
    let mut document_editing_times = BTreeSet::<i32>::new();
    let mut document_negative_editing_times = 0usize;
    let mut document_protection_hashes = BTreeSet::<i32>::new();
    let mut document_protection_hash_states = BTreeMap::<(bool, bool), usize>::new();
    let mut document_default_tab_widths = BTreeSet::<i16>::new();
    let mut document_web_code_pages = BTreeMap::<u16, usize>::new();
    let mut document_hyphenation_zones = BTreeSet::<u16>::new();
    let mut document_consecutive_hyphen_limits = BTreeSet::<u16>::new();
    let mut document_reserved2_values = BTreeMap::<u16, usize>::new();
    let mut document_lock_revision_marking_mismatches = 0usize;
    let mut document_lock_revision_annotation_conflicts = 0usize;
    let mut document_reserved3a_values = BTreeMap::<u32, usize>::new();
    let mut auto_summary_info_shapes = BTreeMap::<
        (
            bool,
            bool,
            AutoSummaryView,
            bool,
            AutoSummaryDesiredSize,
            i32,
            i32,
        ),
        usize,
    >::new();
    let mut auto_summary_range_tables = 0usize;
    let mut auto_summary_ranges = 0usize;
    let mut auto_summary_range_count_shapes = BTreeMap::<usize, usize>::new();
    let mut auto_summary_priority_shapes = BTreeMap::<i32, usize>::new();
    let mut font_tables = 0usize;
    let mut fonts = 0usize;
    let mut alternate_font_names = 0usize;
    let mut font_name_units = 0usize;
    let mut padded_font_names = 0usize;
    let mut font_name_padding_units = 0usize;
    let mut font_family_shapes = BTreeMap::<(u8, bool, u8), usize>::new();
    let mut font_character_sets = BTreeMap::<u8, usize>::new();
    let mut embedded_font_tables = 0usize;
    let mut embedded_font_references = 0usize;
    let mut embedded_font_table_offsets = BTreeMap::<EmbeddedFontTableOffset, usize>::new();
    let mut embedded_font_table_shapes = BTreeMap::<(usize, u32), usize>::new();
    let mut embedded_font_subsets = BTreeMap::<EmbeddedFontSubset, usize>::new();
    let mut embedded_font_nonzero_ignored_flags = 0usize;
    let mut associated_string_tables = 0usize;
    let mut associated_string_units = 0usize;
    let mut nonempty_associated_strings = BTreeMap::<usize, usize>::new();
    let mut maximum_associated_string_lengths = BTreeMap::<usize, usize>::new();
    let mut associated_string_padding = BTreeMap::<u8, usize>::new();
    let mut user_variable_tables = 0usize;
    let mut user_variables = 0usize;
    let mut user_variable_name_units = 0usize;
    let mut user_variable_value_units = 0usize;
    let mut maximum_user_variable_value_units = 0usize;
    let mut user_variable_nonzero_metadata = 0usize;
    let mut user_variable_kinds = BTreeMap::<UserVariableKind, usize>::new();
    let mut user_variable_table_shapes = BTreeMap::<(usize, u32), usize>::new();
    let mut mail_merge_tables = 0usize;
    let mut new_mail_merge_tables = 0usize;
    let mut office_data_source_tables = 0usize;
    let mut office_data_source_properties = 0usize;
    let mut mail_merge_sql_units = 0usize;
    let mut mail_merge_string_tables = 0usize;
    let mut mail_merge_document_type_records = 0usize;
    let mut mail_merge_compatibility_sources = 0usize;
    let mut mail_merge_document_types = BTreeMap::<MailMergeDocumentType, usize>::new();
    let mut mail_merge_destinations = BTreeMap::<MailMergeDestination, usize>::new();
    let mut mail_merge_source_kinds = BTreeMap::<MailMergeSourceKind, usize>::new();
    let mut mail_merge_error_handling = BTreeMap::<MailMergeErrorHandling, usize>::new();
    let mut mail_merge_shapes = BTreeMap::<(u32, usize, bool, bool), usize>::new();
    let mut subdocument_tables = 0usize;
    let mut subdocument_references = 0usize;
    let mut subdocument_nonzero_ignored_flags = 0usize;
    let mut external_file_name_tables = 0usize;
    let mut external_file_names = 0usize;
    let mut format_consistency_bookmark_tables = 0usize;
    let mut format_consistency_bookmarks = 0usize;
    let mut repair_bookmark_tables = 0usize;
    let mut repair_bookmarks = 0usize;
    let mut xml_schema_tables = 0usize;
    let mut xml_schema_references = 0usize;
    let mut xml_schema_element_names = 0usize;
    let mut xml_schema_attribute_names = 0usize;
    let mut xml_schema_ansi_tables = 0usize;
    let mut structured_tag_bookmark_tables = 0usize;
    let mut structured_tag_bookmarks = 0usize;
    let mut structured_tag_attributes = 0usize;
    let mut structured_tag_placeholder_units = 0usize;
    let mut xml_transform_paths = 0usize;
    let mut xml_transform_path_units = 0usize;
    let mut range_protection_tables = 0usize;
    let mut range_permissions = 0usize;
    let mut protected_user_tables = 0usize;
    let mut protected_users = 0usize;
    let mut caption_tables = 0usize;
    let mut caption_definitions = 0usize;
    let mut auto_caption_tables = 0usize;
    let mut auto_caption_definitions = 0usize;
    let mut ignored_non_template_caption_pairs = 0usize;
    let mut revision_author_tables = 0usize;
    let mut revision_authors = 0usize;
    let mut revision_author_units = 0usize;
    let mut revision_author_count_shapes = BTreeMap::<usize, usize>::new();
    let mut maximum_revision_author_length = 0usize;
    let mut revision_author_zero_placeholders = 0usize;
    let mut spelling_state_tables = 0usize;
    let mut spelling_ranges = 0usize;
    let mut spelling_duplicate_positions = 0usize;
    let mut spelling_state_shapes = BTreeMap::<(SpellingStateKind, bool), usize>::new();
    let mut grammar_state_tables = 0usize;
    let mut grammar_ranges = 0usize;
    let mut grammar_duplicate_positions = 0usize;
    let mut grammar_state_shapes = BTreeMap::<(GrammarStateKind, bool, bool, bool), usize>::new();
    let mut language_detection_state_tables = 0usize;
    let mut language_detection_ranges = 0usize;
    let mut language_detection_duplicate_positions = 0usize;
    let mut language_detection_state_shapes =
        BTreeMap::<(LanguageDetectionStateKind, bool), usize>::new();
    let mut list_style_template_tables = 0usize;
    let mut list_style_template_lists = 0usize;
    let mut empty_list_style_templates = 0usize;
    let mut built_in_list_level_templates = 0usize;
    let mut user_list_level_templates = 0usize;
    let mut list_style_template_count_mismatches = 0usize;
    let mut extra_list_style_template_counts = BTreeMap::<usize, usize>::new();
    let mut frame_and_list_tables = 0usize;
    let mut frame_and_list_records = 0usize;
    let mut list_style_references = 0usize;
    let mut custom_list_style_references = 0usize;
    let mut standard_list_style_references = 0usize;
    let mut out_of_range_custom_list_style_references = 0usize;
    let mut grammar_option_tables = 0usize;
    let mut grammar_options = 0usize;
    let mut grammar_option_shapes = BTreeMap::<(u16, u16, u32, u16), usize>::new();
    let mut legacy_grammar_option_tables = 0usize;
    let mut legacy_grammar_options = 0usize;
    let mut legacy_grammar_option_shapes = BTreeMap::<(u16, u16, u16, u16), usize>::new();
    let mut smart_tag_state_tables = 0usize;
    let mut smart_tag_state_ranges = 0usize;
    let mut smart_tag_duplicate_positions = 0usize;
    let mut smart_tag_state_shapes = BTreeMap::<SmartTagRecognizerStateKind, usize>::new();
    let mut paragraph_group_tables = 0usize;
    let mut paragraph_group_entries = 0usize;
    let mut paragraph_group_root_entries = 0usize;
    let mut paragraph_group_maximum_depth = 0u32;
    let mut paragraph_group_missing_parents = 0usize;
    let mut paragraph_group_option_shapes = BTreeMap::<u16, usize>::new();
    let mut paragraph_group_html_types = BTreeMap::<HtmlBlockType, usize>::new();
    let mut save_history_tables = 0usize;
    let mut save_history_entries = 0usize;
    let mut save_history_author_units = 0usize;
    let mut save_history_path_units = 0usize;
    let mut save_history_maximum_author_length = 0usize;
    let mut save_history_maximum_path_length = 0usize;
    let mut save_history_entry_counts = BTreeMap::<usize, usize>::new();
    let mut smart_tag_bookmark_tables = 0usize;
    let mut smart_tag_bookmarks = 0usize;
    let mut smart_tag_sub_entities = 0usize;
    let mut smart_tag_nonzero_unused = 0usize;
    let mut smart_tag_nonzero_property_bag_pointers = 0usize;
    let mut smart_tag_sources = BTreeMap::<SmartTagSource, usize>::new();
    let mut smart_tag_start_depths = BTreeMap::<u16, usize>::new();
    let mut smart_tag_end_depths = BTreeMap::<u16, usize>::new();
    let mut grammar_cookie_tables = 0usize;
    let mut grammar_cookies = 0usize;
    let mut grammar_cookie_headers = 0usize;
    let mut grammar_cookie_errors = 0usize;
    let mut grammar_cookie_duplicate_positions = 0usize;
    let mut grammar_cookie_error_types = BTreeMap::<GrammarCookieErrorType, usize>::new();
    let mut grammar_cookie_languages = BTreeMap::<(u8, u8), usize>::new();
    let mut grammar_cookie_shapes =
        BTreeMap::<(i16, i16, u32, GrammarCookieErrorType, bool, u8, u8, bool), usize>::new();
    let mut grammar_cookie_data_tables = 0usize;
    let mut grammar_cookie_data_entries = 0usize;
    let mut grammar_cookie_provider_bytes = 0usize;
    let mut grammar_cookie_data_shapes = BTreeMap::<(usize, u32), usize>::new();
    let mut grammar_cookie_unreferenced_data = 0usize;
    let mut legacy_grammar_cookie_tables = 0usize;
    let mut legacy_grammar_cookies = 0usize;
    let mut legacy_grammar_cookie_errors = 0usize;
    let mut legacy_grammar_cookie_duplicate_positions = 0usize;
    let mut legacy_grammar_cookie_shapes = BTreeMap::<
        (
            u16,
            i16,
            i16,
            u16,
            GrammarCookieErrorType,
            u16,
            bool,
            u16,
            u32,
        ),
        usize,
    >::new();
    let mut smart_tag_data_tables = 0usize;
    let mut smart_tag_factoid_types = 0usize;
    let mut smart_tag_malformed_cve_factoid_types = 0usize;
    let mut smart_tag_property_bags = 0usize;
    let mut smart_tag_properties = 0usize;
    let mut smart_tag_ansi_strings = 0usize;
    let mut smart_tag_unicode_strings = 0usize;
    let mut smart_tag_reserved_factoid_counts = BTreeMap::<u32, usize>::new();
    let mut smart_tag_property_bag_count_mismatches = 0usize;
    let mut table_character_cache_tables = 0usize;
    let mut table_character_cache_ranges = 0usize;
    let mut table_character_unknown_ranges = 0usize;
    let mut table_character_nonzero_unused = 0usize;
    let mut table_character_cache_shapes = BTreeMap::<usize, usize>::new();
    let mut table_character_canonical_undefined = 0usize;
    let mut revision_threading_tables = 0usize;
    let mut revision_thread_messages = 0usize;
    let mut revision_thread_message_units = 0usize;
    let mut revision_thread_style_units = 0usize;
    let mut revision_thread_nonempty_messages = 0usize;
    let mut revision_thread_nonempty_styles = 0usize;
    let mut revision_thread_nonzero_dates = 0usize;
    let mut revision_thread_nonzero_reserved = 0usize;
    let mut revision_thread_author_indexes = BTreeMap::<i16, usize>::new();
    let mut revision_thread_author_attributes = 0usize;
    let mut revision_thread_message_attributes = 0usize;
    let mut revision_thread_attribute_units = 0usize;
    let mut revision_thread_value_units = 0usize;
    let mut revision_thread_message_count_shapes = BTreeMap::<usize, usize>::new();
    let mut revision_thread_author_count_mismatches = 0usize;
    let mut revision_save_id_tables = 0usize;
    let mut revision_save_ids = 0usize;
    let mut zero_revision_save_ids = 0usize;
    let mut duplicate_revision_save_ids = 0usize;
    let mut distinct_revision_save_ids = BTreeSet::<u32>::new();
    let mut revision_save_id_count_shapes = BTreeMap::<usize, usize>::new();
    let mut revision_save_id_reserved2 = BTreeMap::<u32, usize>::new();
    let mut nonzero_revision_save_id_reserved3 = 0usize;
    let mut selection_state_shapes = BTreeMap::<u32, usize>::new();
    let mut selection_states = 0usize;
    let mut selection_ranges = BTreeMap::<&'static str, usize>::new();
    let mut selection_styles = BTreeMap::<SelectionStyle, usize>::new();
    let mut selection_extensions = BTreeMap::<[u32; 2], usize>::new();
    let mut command_customization_shapes = BTreeMap::<(u32, u8, u8), usize>::new();
    let mut typed_command_customizations = 0usize;
    let mut command_customization_records = BTreeMap::<&'static str, usize>::new();
    let mut pending_command_customization_shapes = BTreeMap::<(u32, u8, u8), usize>::new();
    let mut toolbar_control_records = 0usize;
    let mut toolbar_control_bytes = 0usize;
    let mut toolbar_control_shapes = BTreeMap::<(u8, u16, u8, u8), usize>::new();
    let mut custom_toolbar_records = 0usize;
    let mut custom_toolbar_controls = 0usize;
    let mut shape_anchors_without_fsp = 0usize;
    let mut textbox_stories_without_fsp = 0usize;
    let mut failures = Vec::new();

    for path in files {
        // These duplicate the normal 47950 document and exist specifically to
        // test MS-CFB's case-insensitive stream naming. Keep them out of the
        // semantic DOC inventory and cover them in the focused test below.
        if is_word_document_case_fixture(&path) {
            continue;
        }
        if let Some(mode) = exclusions.get(&path) {
            observed_exclusions.insert(path);
            match mode {
                ExpectationMode::RequiresPassword => encrypted_exclusions += 1,
                ExpectationMode::Invalid => invalid_exclusions += 1,
                _ => unreachable!("DOC exclusion modes are filtered when loaded"),
            }
            continue;
        }
        let result = (|| {
            let bytes = corpus_bytes(&path).map_err(|error| error.to_string())?;
            let Ok(cfb) = CompoundFile::from_bytes(&bytes) else {
                return Ok(false);
            };
            let Some(word_document) = cfb.entry("/WordDocument") else {
                return Ok(false);
            };
            if word_document.data.len() < 2 {
                return Err("WordDocument stream is shorter than wIdent".to_owned());
            }
            let identifier = u16::from_le_bytes(
                word_document.data[..2]
                    .try_into()
                    .expect("two bytes were checked"),
            );
            if identifier != WORD97_FILE_IDENTIFIER {
                *legacy.entry(identifier).or_default() += 1;
                return Ok(false);
            }

            let base = FibBase::from_word_document(&word_document.data)
                .map_err(|error| error.to_string())?;
            if base.flags.contains(FibBaseFlags::ENCRYPTED) {
                return Err(
                    "encrypted DOC is missing a requires_password manifest entry".to_owned(),
                );
            }

            for obj_info in cfb
                .entries()
                .iter()
                .filter(|entry| entry.is_stream() && entry.name == "\u{3}ObjInfo")
            {
                let descriptor = OleObjectDescriptor::from_bytes(&obj_info.data)
                    .map_err(|error| format!("{}: {error}", obj_info.path.display()))?;
                if descriptor.to_bytes() != obj_info.data {
                    return Err(format!(
                        "{}: ODT writer changed physical bytes",
                        obj_info.path.display()
                    ));
                }
                ole_object_descriptors += 1;
                *ole_object_descriptor_shapes
                    .entry((
                        descriptor.persist1.bits(),
                        descriptor.clipboard_format.raw(),
                        descriptor.persist2.map(|value| value.bits()),
                        obj_info.data.len(),
                    ))
                    .or_default() += 1;
                if !descriptor.is_ole_control() {
                    continue;
                }
                ole_object_control_descriptors += 1;
                let uses_stream = descriptor.control_uses_stream();
                ole_object_control_streams += usize::from(uses_stream);
                let parent = obj_info
                    .path
                    .parent()
                    .ok_or_else(|| "ObjInfo stream has no parent storage".to_owned())?;
                let parent_entry = cfb
                    .entry(parent)
                    .ok_or_else(|| "ObjInfo parent storage is missing".to_owned())?;
                let mut children = cfb
                    .entries()
                    .iter()
                    .filter(|entry| {
                        entry.path.parent() == Some(parent) && entry.path != obj_info.path
                    })
                    .map(|entry| {
                        format!(
                            "{}:{}",
                            if entry.is_stream() {
                                "stream"
                            } else {
                                "storage"
                            },
                            entry.name
                        )
                    })
                    .collect::<Vec<_>>();
                children.sort();
                let has_payload = if uses_stream {
                    cfb.entries().iter().any(|entry| {
                        entry.is_stream()
                            && entry.path.parent() == Some(parent)
                            && entry.name == "\u{3}OCXDATA"
                    })
                } else {
                    cfb.entries().iter().any(|entry| {
                        entry.is_stream()
                            && entry.path.parent() == Some(parent)
                            && matches!(entry.name.as_str(), "contents" | "f")
                    })
                };
                let payload_length = cfb
                    .entries()
                    .iter()
                    .find(|entry| {
                        entry.is_stream()
                            && entry.path.parent() == Some(parent)
                            && if uses_stream {
                                entry.name == "\u{3}OCXDATA"
                            } else {
                                matches!(entry.name.as_str(), "contents" | "f")
                            }
                    })
                    .map_or(0, |entry| entry.data.len());
                let class_id = parent_entry.clsid.to_string();
                if matches!(
                    class_id.as_str(),
                    "8bd21d10-ec42-11ce-9e0d-00aa006002f3"
                        | "8bd21d40-ec42-11ce-9e0d-00aa006002f3"
                        | "8bd21d50-ec42-11ce-9e0d-00aa006002f3"
                ) {
                    let payload = cfb
                        .entries()
                        .iter()
                        .find(|entry| {
                            entry.is_stream()
                                && entry.path.parent() == Some(parent)
                                && entry.name == "contents"
                        })
                        .ok_or_else(|| "MorphData control has no contents stream".to_owned())?;
                    let morph = MorphDataControl::from_bytes(&payload.data)
                        .map_err(|error| format!("{}: {error}", payload.path.display()))?;
                    if morph.to_bytes().map_err(|error| error.to_string())? != payload.data {
                        return Err(format!(
                            "{}: MorphData writer changed physical bytes",
                            payload.path.display()
                        ));
                    }
                    morph_data_controls += 1;
                    *morph_data_text_props_masks
                        .entry(morph.text_props.property_mask.bits())
                        .or_default() += 1;
                    morph_data_low_word_compatibility_strings += [
                        morph.extra_data_block.value.as_ref(),
                        morph.extra_data_block.caption.as_ref(),
                        morph.extra_data_block.group_name.as_ref(),
                        morph.text_props.extra_data_block.font_name.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .filter(|value| value.length_mode == FmStringLengthMode::LowWordCompatibility)
                    .count();
                    *morph_data_shapes
                        .entry((
                            class_id.clone(),
                            morph.property_mask.bits(),
                            morph
                                .data_and_extra_size()
                                .map_err(|error| error.to_string())?,
                            morph
                                .following_data_size()
                                .map_err(|error| error.to_string())?,
                        ))
                        .or_default() += 1;
                }
                if class_id == "d7053240-ce69-11cd-a777-00dd01143c57" {
                    let payload = cfb
                        .entries()
                        .iter()
                        .find(|entry| {
                            entry.is_stream()
                                && entry.path.parent() == Some(parent)
                                && entry.name == "contents"
                        })
                        .ok_or_else(|| "CommandButton has no contents stream".to_owned())?;
                    let command_button = CommandButtonControl::from_bytes(&payload.data)
                        .map_err(|error| format!("{}: {error}", payload.path.display()))?;
                    if command_button
                        .to_bytes()
                        .map_err(|error| error.to_string())?
                        != payload.data
                    {
                        return Err(format!(
                            "{}: CommandButton writer changed physical bytes",
                            payload.path.display()
                        ));
                    }
                    command_button_controls += 1;
                    *command_button_shapes
                        .entry((
                            command_button.property_mask.bits(),
                            command_button.text_props.property_mask.bits(),
                            payload.data.len(),
                        ))
                        .or_default() += 1;
                }
                if class_id == "ae24fdae-03c6-11d1-8b76-0080c744f389" {
                    let payload = cfb
                        .entries()
                        .iter()
                        .find(|entry| {
                            entry.is_stream()
                                && entry.path.parent() == Some(parent)
                                && entry.name == "\u{3}OCXDATA"
                        })
                        .ok_or_else(|| {
                            "single-stream OLE control has no OCXDATA stream".to_owned()
                        })?;
                    let control = SingleStreamOleControl::from_bytes(&payload.data)
                        .map_err(|error| format!("{}: {error}", payload.path.display()))?;
                    if control.class_id.to_string() != class_id {
                        return Err(format!(
                            "{}: OCXDATA CLSID does not match its parent storage",
                            payload.path.display()
                        ));
                    }
                    if control.to_bytes() != payload.data {
                        return Err(format!(
                            "{}: single-stream OLE control writer changed physical bytes",
                            payload.path.display()
                        ));
                    }
                    single_stream_ole_controls += 1;
                    *single_stream_ole_control_shapes
                        .entry((
                            class_id.clone(),
                            control.persistence.bytes.len(),
                            control.is_scriptlet_component(),
                        ))
                        .or_default() += 1;
                }
                let class = ole_object_control_classes
                    .entry((class_id, uses_stream))
                    .or_default();
                class.0 += 1;
                class.1.insert(payload_length);
                ole_object_control_missing_payloads += usize::from(!has_payload);
                *ole_object_control_storage_shapes
                    .entry((uses_stream, children))
                    .or_default() += 1;
            }

            let fib =
                Fib::from_word_document(&word_document.data).map_err(|error| error.to_string())?;
            let mut current_list_style_template_count = None;
            let mut current_custom_list_style_indices = Vec::new();
            let mut current_smart_tag_bookmark_count = None;
            let encoded = fib.to_bytes().map_err(|error| error.to_string())?;
            if word_document.data.get(..encoded.len()) != Some(encoded.as_slice()) {
                return Err("FIB write did not reproduce its physical prefix".to_owned());
            }
            *versions.entry(fib.version().n_fib()).or_default() += 1;
            *fc_lcb_shapes
                .entry((fib.version().n_fib(), fib.fc_lcb.len()))
                .or_default() += 1;
            *csw_new_shapes
                .entry((fib.version().n_fib(), fib.csw_new.word_count()))
                .or_default() += 1;
            for (index, location) in fib.fc_lcb.iter().enumerate() {
                if location.lcb != 0 {
                    *nonzero_fib_pairs.entry(index).or_default() += 1;
                }
            }
            let last_saved = fib
                .last_saved_file_time()
                .ok_or_else(|| "FIB is missing its last-saved FILETIME pair".to_owned())?;
            let raw_last_saved = fib
                .fc_lcb(FIB_LAST_SAVED_FILETIME_INDEX)
                .ok_or_else(|| "FIB is missing pair 87".to_owned())?;
            last_saved_file_time_part_mismatches += usize::from(
                last_saved.low() != raw_last_saved.fc || last_saved.high() != raw_last_saved.lcb,
            );
            if last_saved.ticks() == 0 {
                zero_last_saved_file_times += 1;
            } else {
                nonzero_last_saved_file_times += 1;
                last_saved_high_zero_nonzero += usize::from(last_saved.high() == 0);
                distinct_last_saved_file_times.insert(last_saved.ticks());
                minimum_last_saved_file_time = minimum_last_saved_file_time.min(last_saved.ticks());
                maximum_last_saved_file_time = maximum_last_saved_file_time.max(last_saved.ticks());
            }
            if let Some(location) = fib.fc_lcb(30)
                && location.lcb != 0
            {
                *selection_state_shapes.entry(location.lcb).or_default() += 1;
            }
            if fib.base.flags.contains(FibBaseFlags::USE_1_TABLE) {
                table1 += 1;
                if cfb.entry("/1Table").is_none() {
                    return Err("FIB selects 1Table but the stream is absent".to_owned());
                }
            } else {
                table0 += 1;
                if cfb.entry("/0Table").is_none() {
                    return Err("FIB selects 0Table but the stream is absent".to_owned());
                }
            }
            let table = if fib.base.flags.contains(FibBaseFlags::USE_1_TABLE) {
                &cfb.entry("/1Table").expect("presence checked above").data
            } else {
                &cfb.entry("/0Table").expect("presence checked above").data
            };
            let file = DocFile::from_compound_file_compatible(cfb.clone())
                .map_err(|error| format!("typed DOC file root: {error}"))?
                .value;
            if file.word_document.fib != fib {
                return Err("typed DOC file root changed the FIB".to_owned());
            }
            let rebuilt = file
                .to_compound_file()
                .map_err(|error| format!("write typed DOC file root: {error}"))?;
            if rebuilt.stream("/WordDocument") != Some(word_document.data.as_slice())
                || rebuilt.stream(file.table.name.path()) != Some(table.as_slice())
            {
                return Err("typed DOC file root changed a managed stream".to_owned());
            }
            if let Some([metadata_location, starts_location, ends_location]) =
                fib.format_consistency_bookmark_locations()
                && (metadata_location.lcb != 0
                    || starts_location.lcb != 0
                    || ends_location.lcb != 0)
            {
                if metadata_location.lcb == 0 || starts_location.lcb == 0 || ends_location.lcb == 0
                {
                    return Err("format-consistency bookmark tables are incomplete".to_owned());
                }
                let metadata = bounded_slice(
                    table,
                    metadata_location.fc,
                    metadata_location.lcb,
                    "SttbfBkmkFcc",
                )?;
                let starts =
                    bounded_slice(table, starts_location.fc, starts_location.lcb, "PlcfBkfFcc")?;
                let ends = bounded_slice(table, ends_location.fc, ends_location.lcb, "PlcfBklFcc")?;
                let bookmarks = FormatConsistencyBookmarks::from_bytes(metadata, starts, ends)
                    .map_err(|error| format!("FCC bookmarks: {error}"))?;
                let character_count = u32::try_from(fib.rg_lw.ccp_text)
                    .map_err(|_| "FCC bookmarks have negative ccpText".to_owned())?;
                bookmarks
                    .validate_main_document(character_count)
                    .map_err(|error| format!("FCC bookmarks/FibRgLw97: {error}"))?;
                let written = bookmarks.to_bytes().map_err(|error| error.to_string())?;
                if written.metadata != metadata || written.starts != starts || written.ends != ends
                {
                    return Err("FCC bookmark writer changed physical bytes".to_owned());
                }
                format_consistency_bookmark_tables += 1;
                format_consistency_bookmarks += bookmarks.records.len();
            }
            if let Some([metadata_location, starts_location, ends_location]) =
                fib.repair_bookmark_locations()
                && (metadata_location.lcb != 0
                    || starts_location.lcb != 0
                    || ends_location.lcb != 0)
            {
                if metadata_location.lcb == 0 || starts_location.lcb == 0 || ends_location.lcb == 0
                {
                    return Err("repair bookmark tables are incomplete".to_owned());
                }
                let metadata = bounded_slice(
                    table,
                    metadata_location.fc,
                    metadata_location.lcb,
                    "SttbfBkmkBPRepairs",
                )?;
                let starts = bounded_slice(
                    table,
                    starts_location.fc,
                    starts_location.lcb,
                    "PlcfBkfBPRepairs",
                )?;
                let ends = bounded_slice(
                    table,
                    ends_location.fc,
                    ends_location.lcb,
                    "PlcfBklBPRepairs",
                )?;
                let bookmarks = RepairBookmarks::from_bytes(metadata, starts, ends)
                    .map_err(|error| format!("repair bookmarks: {error}"))?;
                let character_count = u32::try_from(fib.rg_lw.ccp_text)
                    .map_err(|_| "repair bookmarks have negative ccpText".to_owned())?;
                bookmarks
                    .validate_main_document(character_count)
                    .map_err(|error| format!("repair bookmarks/FibRgLw97: {error}"))?;
                let written = bookmarks.to_bytes().map_err(|error| error.to_string())?;
                if written.metadata != metadata || written.starts != starts || written.ends != ends
                {
                    return Err("repair bookmark writer changed physical bytes".to_owned());
                }
                repair_bookmark_tables += 1;
                repair_bookmarks += bookmarks.descriptions.len();
            }
            let external_files = if let Some(location) = fib.external_file_names_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbFnm")?;
                let files = ExternalFileNameTable::from_bytes(physical)
                    .map_err(|error| format!("SttbFnm: {error}"))?;
                if files.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbFnm writer changed physical bytes".to_owned());
                }
                external_file_name_tables += 1;
                external_file_names += files.files.len();
                Some(files)
            } else {
                None
            };
            let xml_schemas = if let Some(location) = fib.xml_schema_references_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Hplxsdr")?;
                let schemas = XmlSchemaReferences::from_bytes(physical)
                    .map_err(|error| format!("Hplxsdr: {error}"))?;
                if schemas.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Hplxsdr writer changed physical bytes".to_owned());
                }
                xml_schema_tables += 1;
                xml_schema_references += schemas.schemas.len();
                for schema in &schemas.schemas {
                    xml_schema_element_names += match &schema.elements {
                        XmlSchemaStringTable::Ansi(values) => {
                            xml_schema_ansi_tables += 1;
                            values.len()
                        }
                        XmlSchemaStringTable::Utf16(values) => values.len(),
                    };
                    xml_schema_attribute_names += match &schema.attributes {
                        XmlSchemaStringTable::Ansi(values) => {
                            xml_schema_ansi_tables += 1;
                            values.len()
                        }
                        XmlSchemaStringTable::Utf16(values) => values.len(),
                    };
                }
                Some(schemas)
            } else {
                None
            };
            if let Some([tags_location, starts_location, ends_location]) =
                fib.structured_tag_bookmark_locations()
                && (tags_location.lcb != 0 || starts_location.lcb != 0 || ends_location.lcb != 0)
            {
                if tags_location.lcb == 0 || starts_location.lcb == 0 || ends_location.lcb == 0 {
                    return Err("structured-tag bookmark tables are incomplete".to_owned());
                }
                let tags_physical =
                    bounded_slice(table, tags_location.fc, tags_location.lcb, "SttbfBkmkSdt")?;
                let starts_physical =
                    bounded_slice(table, starts_location.fc, starts_location.lcb, "PlcfBkfSdt")?;
                let ends_physical =
                    bounded_slice(table, ends_location.fc, ends_location.lcb, "PlcfBklSdt")?;
                let bookmarks = StructuredTagBookmarks::from_bytes(
                    tags_physical,
                    starts_physical,
                    ends_physical,
                )
                .map_err(|error| format!("structured-tag bookmarks: {error}"))?;
                bookmarks
                    .validate_schema_references(xml_schemas.as_ref().ok_or_else(|| {
                        "structured-tag bookmarks exist but Hplxsdr is absent".to_owned()
                    })?)
                    .map_err(|error| format!("structured-tag schemas: {error}"))?;
                let written = bookmarks.to_bytes().map_err(|error| error.to_string())?;
                if written.tags != tags_physical
                    || written.starts != starts_physical
                    || written.ends != ends_physical
                {
                    return Err("structured-tag bookmark writer changed physical bytes".to_owned());
                }
                structured_tag_bookmark_tables += 1;
                structured_tag_bookmarks += bookmarks.tags.len();
                structured_tag_attributes += bookmarks
                    .tags
                    .iter()
                    .map(|tag| tag.attributes.len())
                    .sum::<usize>();
                structured_tag_placeholder_units += bookmarks
                    .tags
                    .iter()
                    .map(|tag| tag.placeholder.len())
                    .sum::<usize>();
            }
            if let Some(location) = fib.xml_transform_path_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "CustomXForm")?;
                let transform = XmlTransformPath::from_bytes(physical)
                    .map_err(|error| format!("CustomXForm: {error}"))?;
                if transform.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("CustomXForm writer changed physical bytes".to_owned());
                }
                xml_transform_paths += 1;
                xml_transform_path_units += transform.path.len();
            }
            if let Some(
                [
                    permissions_location,
                    starts_location,
                    ends_location,
                    users_location,
                ],
            ) = fib.range_protection_locations()
                && (permissions_location.lcb != 0
                    || starts_location.lcb != 0
                    || ends_location.lcb != 0
                    || users_location.lcb != 0)
            {
                let user_physical = if users_location.lcb == 0 {
                    &[][..]
                } else {
                    bounded_slice(table, users_location.fc, users_location.lcb, "SttbProtUser")?
                };
                let users = ProtectedUsers::from_bytes(user_physical)
                    .map_err(|error| format!("SttbProtUser: {error}"))?;
                if users.to_bytes().map_err(|error| error.to_string())? != user_physical {
                    return Err("SttbProtUser writer changed physical bytes".to_owned());
                }
                if users_location.lcb != 0 {
                    protected_user_tables += 1;
                    protected_users += users.users.len();
                }

                if permissions_location.lcb != 0
                    || starts_location.lcb != 0
                    || ends_location.lcb != 0
                {
                    if permissions_location.lcb == 0
                        || starts_location.lcb == 0
                        || ends_location.lcb == 0
                    {
                        return Err("range-protection bookmark tables are incomplete".to_owned());
                    }
                    let permissions_physical = bounded_slice(
                        table,
                        permissions_location.fc,
                        permissions_location.lcb,
                        "SttbfBkmkProt",
                    )?;
                    let starts_physical = bounded_slice(
                        table,
                        starts_location.fc,
                        starts_location.lcb,
                        "PlcfBkfProt",
                    )?;
                    let ends_physical =
                        bounded_slice(table, ends_location.fc, ends_location.lcb, "PlcfBklProt")?;
                    let protection = RangeProtection::from_bytes(
                        permissions_physical,
                        starts_physical,
                        ends_physical,
                        user_physical,
                    )
                    .map_err(|error| format!("range protection: {error}"))?;
                    let written = protection.to_bytes().map_err(|error| error.to_string())?;
                    if written.permissions != permissions_physical
                        || written.starts != starts_physical
                        || written.ends != ends_physical
                        || written.users != user_physical
                    {
                        return Err("range-protection writer changed physical bytes".to_owned());
                    }
                    range_protection_tables += 1;
                    range_permissions += protection.permissions.len();
                }
            }
            if let Some(location) = fib.mail_merge_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Pms")?;
                let state = MailMergeState::from_bytes(physical)
                    .map_err(|error| format!("Pms: {error}"))?;
                if state.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Pms writer changed physical bytes".to_owned());
                }
                mail_merge_tables += 1;
                let sql_units = state.sql_query.as_ref().map_or(0, Vec::len);
                mail_merge_sql_units += sql_units;
                mail_merge_string_tables += usize::from(state.strings.is_some());
                mail_merge_document_type_records += usize::from(state.document_type.is_some());
                *mail_merge_document_types
                    .entry(state.status.document_type)
                    .or_default() += 1;
                *mail_merge_destinations
                    .entry(state.status.destination)
                    .or_default() += 1;
                *mail_merge_error_handling
                    .entry(state.filter.error_handling)
                    .or_default() += 1;
                *mail_merge_shapes
                    .entry((
                        location.lcb,
                        sql_units,
                        state.strings.is_some(),
                        state.document_type.is_some(),
                    ))
                    .or_default() += 1;
                for source in state.sources {
                    *mail_merge_source_kinds.entry(source.kind).or_default() += 1;
                    mail_merge_compatibility_sources +=
                        usize::from(source.file == MailMergeFileReference::NilCompatibility);
                }
                if let Some(files) = &external_files {
                    state
                        .validate_file_references(files)
                        .map_err(|error| format!("Pms/SttbFnm: {error}"))?;
                } else if state
                    .sources
                    .iter()
                    .any(|source| matches!(source.file, MailMergeFileReference::Identifier(_)))
                {
                    return Err("Pms has an FNPI reference but SttbFnm is absent".to_owned());
                }
            }
            let new_mail_merge_state = if let Some(location) = fib.new_mail_merge_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PmsNew")?;
                let state = MailMergeState::from_bytes(physical)
                    .map_err(|error| format!("PmsNew: {error}"))?;
                if state.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PmsNew writer changed physical bytes".to_owned());
                }
                if let Some(files) = &external_files {
                    state
                        .validate_file_references(files)
                        .map_err(|error| format!("PmsNew/SttbFnm: {error}"))?;
                } else if state
                    .sources
                    .iter()
                    .any(|source| matches!(source.file, MailMergeFileReference::Identifier(_)))
                {
                    return Err("PmsNew has an FNPI reference but SttbFnm is absent".to_owned());
                }
                new_mail_merge_tables += 1;
                Some(state)
            } else {
                None
            };
            if let Some(location) = fib.office_data_source_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "ODSO")?;
                let source = OfficeDataSource::from_bytes(physical)
                    .map_err(|error| format!("ODSO: {error}"))?;
                if source.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("ODSO writer changed physical bytes".to_owned());
                }
                if let Some(state) = &new_mail_merge_state {
                    source
                        .validate_mail_merge_state(state)
                        .map_err(|error| format!("ODSO/PmsNew: {error}"))?;
                }
                office_data_source_tables += 1;
                office_data_source_properties += source.properties.len();
            }
            if let Some(location) = fib.subdocuments_location()
                && location.lcb != 0
            {
                if plcf_wkb_exclusions.contains_key(&path) {
                    observed_plcf_wkb_exclusions.insert(path.clone());
                } else {
                    let physical = bounded_slice(table, location.fc, location.lcb, "PlcfWKB")?;
                    let subdocuments = SubdocumentTable::from_bytes(physical)
                        .map_err(|error| format!("PlcfWKB: {error}"))?;
                    if subdocuments.to_bytes().map_err(|error| error.to_string())? != physical {
                        return Err("PlcfWKB writer changed physical bytes".to_owned());
                    }
                    let character_count = u32::try_from(fib.rg_lw.ccp_text)
                        .map_err(|_| "PlcfWKB has a negative ccpText".to_owned())?;
                    subdocuments
                        .validate_main_document_length(character_count)
                        .map_err(|error| format!("PlcfWKB/FibRgLw97: {error}"))?;
                    subdocuments
                        .validate_file_references(external_files.as_ref().ok_or_else(|| {
                            "PlcfWKB has FNPI references but SttbFnm is absent".to_owned()
                        })?)
                        .map_err(|error| format!("PlcfWKB/SttbFnm: {error}"))?;
                    subdocument_tables += 1;
                    subdocument_references += subdocuments.subdocuments.len();
                    for subdocument in subdocuments.subdocuments {
                        subdocument_nonzero_ignored_flags +=
                            usize::from(subdocument.ignored_flag3 || subdocument.ignored_flag8);
                    }
                }
            }
            if let Some((caption_location, automatic_location)) = fib.caption_locations()
                && (caption_location.lcb != 0 || automatic_location.lcb != 0)
            {
                if !fib.base.flags.contains(FibBaseFlags::DOCUMENT_TEMPLATE) {
                    ignored_non_template_caption_pairs += 1;
                } else {
                    let captions = if caption_location.lcb == 0 {
                        None
                    } else {
                        let physical = bounded_slice(
                            table,
                            caption_location.fc,
                            caption_location.lcb,
                            "SttbfCaption",
                        )?;
                        let captions = CaptionDefinitions::from_bytes(physical)
                            .map_err(|error| format!("SttbfCaption: {error}"))?;
                        if captions.to_bytes().map_err(|error| error.to_string())? != physical {
                            return Err("SttbfCaption writer changed physical bytes".to_owned());
                        }
                        caption_tables += 1;
                        caption_definitions += captions.captions.len();
                        Some(captions)
                    };
                    if automatic_location.lcb != 0 {
                        let physical = bounded_slice(
                            table,
                            automatic_location.fc,
                            automatic_location.lcb,
                            "SttbfAutoCaption",
                        )?;
                        let automatic = AutoCaptionDefinitions::from_bytes(physical)
                            .map_err(|error| format!("SttbfAutoCaption: {error}"))?;
                        if automatic.to_bytes().map_err(|error| error.to_string())? != physical {
                            return Err("SttbfAutoCaption writer changed physical bytes".to_owned());
                        }
                        automatic
                            .validate_against(
                                captions.as_ref().ok_or_else(|| {
                                    "SttbfAutoCaption has no SttbfCaption".to_owned()
                                })?,
                            )
                            .map_err(|error| format!("caption tables: {error}"))?;
                        auto_caption_tables += 1;
                        auto_caption_definitions += automatic.entries.len();
                    }
                }
            }
            if let Some(location) = fib.mso_envelope_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "MsoEnvelope")?;
                let envelope = MsoEnvelopeClsid::from_bytes(physical)
                    .map_err(|error| format!("MsoEnvelope: {error}"))?;
                if envelope.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("MsoEnvelope writer changed physical bytes".to_owned());
                }
                mso_envelope_tables += 1;
                match envelope.data {
                    MsoEnvelopeData::OutOfScope(_) => mso_envelope_out_of_scope += 1,
                    MsoEnvelopeData::Envelope(envelope) => {
                        mso_envelope_typed += 1;
                        *mso_envelope_versions.entry(envelope.version).or_default() += 1;
                        *mso_envelope_shapes
                            .entry((
                                envelope.flag_status,
                                envelope.sensitivity,
                                envelope.importance,
                                envelope.security.bits(),
                                envelope.delete_after_submit,
                                envelope.originator_delivery_report_requested,
                                envelope.read_receipt_requested,
                            ))
                            .or_default() += 1;
                        mso_envelope_subject_units += envelope.subject.len();
                        for collection in [
                            Some(&envelope.reply_recipients),
                            envelope.contact_link_recipients.as_ref(),
                            Some(&envelope.recipients),
                        ]
                        .into_iter()
                        .flatten()
                        {
                            mso_envelope_recipients += collection.recipients.len();
                            for recipient in &collection.recipients {
                                mso_envelope_recipient_properties += recipient.properties.len();
                                for property in &recipient.properties {
                                    let kind = match property.value {
                                        EnvelopeRecipientPropertyValue::Long(_) => "long",
                                        EnvelopeRecipientPropertyValue::Null(_) => "null",
                                        EnvelopeRecipientPropertyValue::Boolean(_) => "boolean",
                                        EnvelopeRecipientPropertyValue::SystemTime(_) => "systime",
                                        EnvelopeRecipientPropertyValue::Error(_) => "error",
                                        EnvelopeRecipientPropertyValue::String8(_) => "string8",
                                        EnvelopeRecipientPropertyValue::Unicode(_) => "unicode",
                                        EnvelopeRecipientPropertyValue::Binary(_) => "binary",
                                        EnvelopeRecipientPropertyValue::MultiString8(_) => {
                                            "multi-string8"
                                        }
                                        EnvelopeRecipientPropertyValue::MultiBinary(_) => {
                                            "multi-binary"
                                        }
                                    };
                                    *mso_envelope_property_types.entry(kind).or_default() += 1;
                                }
                            }
                        }
                        mso_envelope_attachments += envelope.attachments.len();
                        mso_envelope_attachment_bytes += envelope
                            .attachments
                            .iter()
                            .map(|attachment| attachment.data.len())
                            .sum::<usize>();
                        mso_envelope_attachment_name_units += envelope
                            .attachments
                            .iter()
                            .map(|attachment| attachment.name.len())
                            .sum::<usize>();
                        for attachment in &envelope.attachments {
                            *mso_envelope_attachment_methods
                                .entry(attachment.method)
                                .or_default() += 1;
                        }
                        mso_envelope_intro_units +=
                            envelope.intro_text.as_ref().map_or(0, Vec::len);
                    }
                }
            }
            let mut current_revision_author_count = None;
            if let Some(location) = fib.printer_driver_info_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PrDrvr")?;
                let info = PrinterDriverInfo::from_bytes(physical)
                    .map_err(|error| format!("PrDrvr: {error}"))?;
                if info.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PrDrvr writer changed physical bytes".to_owned());
                }
                printer_driver_info_tables += 1;
                printer_driver_info_total_bytes += physical.len();
                printer_driver_info_empty_fields += [
                    &info.printer_name,
                    &info.port_name,
                    &info.driver_name,
                    &info.product_name,
                ]
                .into_iter()
                .filter(|value| value.is_empty())
                .count();
                *printer_driver_info_length_shapes
                    .entry((
                        info.printer_name.len(),
                        info.port_name.len(),
                        info.driver_name.len(),
                        info.product_name.len(),
                    ))
                    .or_default() += 1;
                printer_driver_names.insert(info.printer_name);
                printer_port_names.insert(info.port_name);
                printer_driver_file_names.insert(info.driver_name);
                printer_product_names.insert(info.product_name);
            }
            if let Some(location) = fib.fc_lcb(24)
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Tcg")?;
                *command_customization_shapes
                    .entry((
                        location.lcb,
                        physical.first().copied().unwrap_or(0),
                        physical.get(1).copied().unwrap_or(0),
                    ))
                    .or_default() += 1;
                match CommandCustomizations::from_bytes(physical) {
                    Ok(customizations) => {
                        if customizations
                            .to_bytes()
                            .map_err(|error| error.to_string())?
                            != physical
                        {
                            return Err("Tcg write did not reproduce its physical bytes".to_owned());
                        }
                        typed_command_customizations += 1;
                        for record in customizations.records {
                            *command_customization_records
                                .entry(match record {
                                    CommandCustomizationRecord::MacroCommands(_) => {
                                        "macro-commands"
                                    }
                                    CommandCustomizationRecord::CommandStrings(_) => {
                                        "command-strings"
                                    }
                                    CommandCustomizationRecord::MacroNames(_) => "macro-names",
                                    CommandCustomizationRecord::Toolbar(value) => {
                                        toolbar_control_records += value.controls.len();
                                        for toolbar in value
                                            .customizations
                                            .iter()
                                            .filter_map(|value| value.custom_toolbar.as_ref())
                                        {
                                            custom_toolbar_records += 1;
                                            custom_toolbar_controls += toolbar.controls.len();
                                        }
                                        for control in &value.controls {
                                            *toolbar_control_shapes
                                                .entry((
                                                    control.header.control_type,
                                                    control.header.control_id,
                                                    control.header.flags,
                                                    control
                                                        .data
                                                        .as_ref()
                                                        .map_or(0, |data| data.general.flags),
                                                ))
                                                .or_default() += 1;
                                        }
                                        toolbar_control_bytes += value
                                            .customizations
                                            .iter()
                                            .flat_map(|customization| &customization.deltas)
                                            .map(|delta| usize::from(delta.control_byte_count))
                                            .sum::<usize>();
                                        "toolbar"
                                    }
                                })
                                .or_default() += 1;
                        }
                    }
                    Err(_) => {
                        *pending_command_customization_shapes
                            .entry((location.lcb, physical[0], physical[1]))
                            .or_default() += 1;
                    }
                }
            }
            let mut current_font_count = None;
            if let Some(location) = fib.font_table_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbfFfn")?;
                let font_table = FontTable::from_bytes(physical).map_err(|error| {
                    format!(
                        "SttbfFfn fc={:#x} lcb={:#x} bytes={physical:02x?}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if font_table.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbfFfn write did not reproduce its physical bytes".to_owned());
                }
                font_tables += 1;
                fonts += font_table.fonts.len();
                current_font_count = Some(font_table.fonts.len());
                for font in font_table.fonts {
                    alternate_font_names += usize::from(font.alternate_name().is_some());
                    font_name_units +=
                        font.name_units.len() + usize::from(font.trailing_name_nulls);
                    padded_font_names += usize::from(font.trailing_name_nulls != 0);
                    font_name_padding_units += usize::from(font.trailing_name_nulls);
                    *font_family_shapes
                        .entry((font.family.pitch, font.family.true_type, font.family.family))
                        .or_default() += 1;
                    *font_character_sets.entry(font.character_set).or_default() += 1;
                }
            }
            if let Some(location) = fib.embedded_fonts_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbTtmbd")?;
                let embedded = EmbeddedFontTable::from_bytes(physical)
                    .map_err(|error| format!("SttbTtmbd: {error}"))?;
                if embedded.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbTtmbd write did not reproduce its physical bytes".to_owned());
                }
                let font_count = current_font_count
                    .ok_or_else(|| "SttbTtmbd has no corresponding SttbfFfn".to_owned())?;
                embedded
                    .validate_against_font_table(font_count)
                    .map_err(|error| format!("SttbTtmbd/SttbfFfn: {error}"))?;
                embedded_font_tables += 1;
                embedded_font_references += embedded.fonts.len();
                *embedded_font_table_offsets
                    .entry(embedded.producer_offset)
                    .or_default() += 1;
                *embedded_font_table_shapes
                    .entry((embedded.fonts.len(), location.lcb))
                    .or_default() += 1;
                for font in embedded.fonts {
                    *embedded_font_subsets.entry(font.subset).or_default() += 1;
                    embedded_font_nonzero_ignored_flags += usize::from(font.ignored_flags != 0);
                }
            }
            let mut current_grammar_cookie_store = None;
            if let Some(location) = fib.grammar_cookie_data_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "RgCdb")?;
                let store = GrammarCookieStore::from_bytes(physical)
                    .map_err(|error| format!("RgCdb: {error}"))?;
                if store.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("RgCdb writer changed physical bytes".to_owned());
                }
                grammar_cookie_data_tables += 1;
                grammar_cookie_data_entries += store.cookies.len();
                grammar_cookie_provider_bytes += store
                    .cookies
                    .iter()
                    .map(|cookie| cookie.provider_data.len())
                    .sum::<usize>();
                *grammar_cookie_data_shapes
                    .entry((store.cookies.len(), location.lcb))
                    .or_default() += 1;
                current_grammar_cookie_store = Some(store);
            }
            let mut auto_summary_info = None;
            let mut current_last_list_indexes = None;
            let mut current_word2003 = None;
            if let Some(location) = fib.document_properties_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Dop")?;
                let properties = DocumentProperties::from_bytes(physical)
                    .map_err(|error| format!("Dop: {error}"))?;
                if properties.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Dop write did not reproduce its physical bytes".to_owned());
                }
                *document_property_shapes
                    .entry((fib.version().n_fib(), location.lcb))
                    .or_default() += 1;
                *document_classifications
                    .entry(properties.word97.document_classification)
                    .or_default() += 1;
                let undefined_space = *properties.word97.undefined_space.bytes();
                document_undefined_space_shapes.insert(undefined_space);
                let nonzero_space_bytes = undefined_space
                    .into_iter()
                    .filter(|byte| *byte != 0)
                    .count();
                document_nonzero_undefined_spaces += usize::from(nonzero_space_bytes != 0);
                document_nonzero_undefined_space_bytes += nonzero_space_bytes;
                let list_indexes = properties.word97.last_list_indexes;
                current_last_list_indexes = Some(list_indexes);
                document_last_list_indexes.insert((list_indexes.bullet, list_indexes.numbering));
                document_nonzero_last_list_indexes +=
                    usize::from(list_indexes.bullet != 0 || list_indexes.numbering != 0);
                *document_note_number_formats
                    .entry((
                        properties.word97.footnote_number_format.code(),
                        properties.word97.endnote_number_format.code(),
                    ))
                    .or_default() += 1;
                *document_pagination_display_shapes
                    .entry((
                        properties.word97.pagination_zoom_font_size,
                        properties.word97.pagination_screen_height,
                    ))
                    .or_default() += 1;
                let characters_with_spaces = properties.word97.characters_with_spaces;
                let double_byte_characters = properties.word97.double_byte_characters;
                document_characters_with_spaces_shapes.insert((
                    characters_with_spaces.main,
                    characters_with_spaces.with_subdocuments,
                ));
                document_double_byte_character_shapes.insert((
                    double_byte_characters.main,
                    double_byte_characters.with_subdocuments,
                ));
                document_negative_character_count_pairs += usize::from(
                    !characters_with_spaces.is_nonnegative()
                        || !double_byte_characters.is_nonnegative(),
                );
                document_character_count_relation_mismatches += usize::from(
                    !characters_with_spaces.includes_main()
                        || !double_byte_characters.includes_main(),
                );
                *document_character_statistic_states
                    .entry((
                        properties.word97.base.document_flags.exact_statistics,
                        properties
                            .word97
                            .base
                            .endnote_options
                            .include_subdocuments_in_statistics,
                        characters_with_spaces.includes_main(),
                        double_byte_characters.includes_main(),
                    ))
                    .or_default() += 1;
                let base = &properties.word97.base;
                let statistics = base.statistics;
                document_main_statistic_shapes.insert((
                    statistics.main.words,
                    statistics.main.characters,
                    statistics.main.pages,
                    statistics.main.paragraphs,
                    statistics.main.lines,
                ));
                document_subdocument_statistic_shapes.insert((
                    statistics.with_subdocuments.words,
                    statistics.with_subdocuments.characters,
                    statistics.with_subdocuments.pages,
                    statistics.with_subdocuments.paragraphs,
                    statistics.with_subdocuments.lines,
                ));
                document_negative_statistics += usize::from(!statistics.is_nonnegative());
                document_statistic_relation_mismatches += usize::from(!statistics.includes_main());
                let statistics_are_exact = base.document_flags.exact_statistics;
                let include_subdocuments = base.endnote_options.include_subdocuments_in_statistics;
                *document_statistic_states
                    .entry((
                        statistics_are_exact,
                        include_subdocuments,
                        statistics.includes_main(),
                    ))
                    .or_default() += 1;
                if let Some(exact) = base.exact_statistics() {
                    let character_bound = if include_subdocuments {
                        i64::from(fib.rg_lw.ccp_text)
                            + i64::from(fib.rg_lw.ccp_footnote)
                            + i64::from(fib.rg_lw.ccp_endnote)
                            + i64::from(fib.rg_lw.ccp_textbox)
                    } else {
                        i64::from(fib.rg_lw.ccp_text)
                    };
                    document_exact_statistic_character_bound_mismatches += usize::from(
                        exact.characters < 0 || i64::from(exact.characters) > character_bound,
                    );
                }
                document_created_timestamps.insert(base.created);
                document_revised_timestamps.insert(base.revised);
                document_last_printed_timestamps.insert(base.last_printed);
                document_ignored_timestamp_counts[0] += usize::from(base.created.is_ignored());
                document_ignored_timestamp_counts[1] += usize::from(base.revised.is_ignored());
                document_ignored_timestamp_counts[2] += usize::from(base.last_printed.is_ignored());
                document_revision_counts.insert(base.revision_count);
                document_editing_times.insert(base.editing_time);
                document_negative_editing_times += usize::from(base.editing_time < 0);
                let protection_hash = base.protection_password_hash.0;
                document_protection_hashes.insert(protection_hash);
                let protection_enabled = base.document_flags.lock_revisions
                    || base.document_flags.form_protection
                    || base.document_flags.lock_annotations
                    || base.document_flags.revision_marking;
                *document_protection_hash_states
                    .entry((protection_enabled, protection_hash != 0))
                    .or_default() += 1;
                document_default_tab_widths.insert(base.default_tab_width);
                *document_web_code_pages
                    .entry(base.web_code_page.0)
                    .or_default() += 1;
                document_hyphenation_zones.insert(base.hyphenation_zone);
                document_consecutive_hyphen_limits.insert(base.consecutive_hyphen_limit);
                *document_reserved2_values.entry(base.reserved2).or_default() += 1;
                document_lock_revision_marking_mismatches += usize::from(
                    base.document_flags.lock_revisions && !base.document_flags.revision_marking,
                );
                document_lock_revision_annotation_conflicts += usize::from(
                    base.document_flags.lock_revisions && base.document_flags.lock_annotations,
                );
                *document_reserved3a_values
                    .entry(properties.word97.reserved3a)
                    .or_default() += 1;
                let numbering_cache = properties.word97.deprecated_numbering_field_cache_metadata(
                    fib.deprecated_numbering_field_cache_location(),
                );
                let numbering_cache_present = numbering_cache.is_present();
                *document_numbering_cache_states
                    .entry((numbering_cache_present, numbering_cache.invalid))
                    .or_default() += 1;
                if let Some(location) = numbering_cache.location
                    && location.lcb != 0
                {
                    bounded_slice(table, location.fc, location.lcb, "PlcfBteLvc")?;
                    *document_numbering_cache_lengths
                        .entry(location.lcb)
                        .or_default() += 1;
                    document_numbering_cache_present_max_positions
                        .insert(numbering_cache.maximum_valid_position);
                } else {
                    document_numbering_cache_absent_max_positions
                        .insert(numbering_cache.maximum_valid_position);
                }
                let typography = &properties.word97.typography;
                *document_typography_shapes
                    .entry((
                        typography.justification,
                        typography.kinsoku_level,
                        typography.custom_kinsoku_language,
                        typography.kern_punctuation,
                        typography.print_two_on_one,
                        typography.unused,
                        typography.japanese_use_level2,
                        typography.following_punctuation_count,
                        typography.leading_punctuation_count,
                    ))
                    .or_default() += 1;
                document_typography_following_units += typography
                    .following_punctuation()
                    .map_err(|error| error.to_string())?
                    .len();
                document_typography_leading_units += typography
                    .leading_punctuation()
                    .map_err(|error| error.to_string())?
                    .len();
                document_typography_nonzero_unused_following_slots += typography
                    .following_punctuation_slots
                    [usize::from(typography.following_punctuation_count)..]
                    .iter()
                    .filter(|unit| **unit != 0)
                    .count();
                document_typography_nonzero_unused_leading_slots += typography
                    .leading_punctuation_slots[usize::from(typography.leading_punctuation_count)..]
                    .iter()
                    .filter(|unit| **unit != 0)
                    .count();
                let options60 = properties.word97.base.compatibility_options_60;
                let options80 = properties.word97.compatibility_options_80;
                *document_compatibility_option_shapes
                    .entry((options60.bits(), options80.bits()))
                    .or_default() += 1;
                if !properties.word97.compatibility_options_match() {
                    *document_compatibility_option_mismatches
                        .entry(options60.bits() ^ options80.word6.bits())
                        .or_default() += 1;
                }
                *document_format_flag_shapes
                    .entry(
                        properties
                            .word97
                            .base
                            .format_flags
                            .bits()
                            .map_err(|error| error.to_string())?,
                    )
                    .or_default() += 1;
                *document_footnote_numbering_shapes
                    .entry(
                        properties
                            .word97
                            .base
                            .footnote_numbering
                            .bits()
                            .map_err(|error| error.to_string())?,
                    )
                    .or_default() += 1;
                *document_state_flag_shapes
                    .entry(
                        properties
                            .word97
                            .base
                            .document_flags
                            .bits()
                            .map_err(|error| error.to_string())?,
                    )
                    .or_default() += 1;
                *document_endnote_numbering_shapes
                    .entry(
                        properties
                            .word97
                            .base
                            .endnote_numbering
                            .bits()
                            .map_err(|error| error.to_string())?,
                    )
                    .or_default() += 1;
                *document_endnote_option_shapes
                    .entry(
                        properties
                            .word97
                            .base
                            .endnote_options
                            .bits()
                            .map_err(|error| error.to_string())?,
                    )
                    .or_default() += 1;
                let saved_view_bits = properties
                    .word97
                    .base
                    .saved_view
                    .bits()
                    .map_err(|error| error.to_string())?;
                *document_saved_view_shapes
                    .entry(saved_view_bits)
                    .or_default() += 1;
                *document_saved_view_kinds
                    .entry(properties.word97.base.saved_view.kind)
                    .or_default() += 1;
                *document_saved_zoom_kinds
                    .entry(((saved_view_bits >> 12) & 3) as u8)
                    .or_default() += 1;
                document_saved_zoom_percentages.insert((saved_view_bits >> 3) & 0x01ff);
                *document_display_flag_shapes
                    .entry(properties.word97.display_flags.bits())
                    .or_default() += 1;
                *document_outline_levels
                    .entry(properties.word97.display_flags.outline_level)
                    .or_default() += 1;
                *document_version_flag_shapes
                    .entry(properties.word97.version_flags.bits())
                    .or_default() += 1;
                *document_event_shapes
                    .entry(properties.word97.document_events.bits())
                    .or_default() += 1;
                *document_virus_flag_shapes
                    .entry((
                        properties.word97.virus_info.prompted,
                        properties.word97.virus_info.load_safe,
                    ))
                    .or_default() += 1;
                document_virus_session_keys.insert(properties.word97.virus_info.session_key);
                let grid = properties.word97.drawing_grid;
                *document_drawing_grid_shapes
                    .entry((
                        grid.horizontal_origin,
                        grid.vertical_origin,
                        grid.horizontal_spacing,
                        grid.vertical_spacing,
                        grid.vertical_display_frequency,
                        grid.unused,
                        grid.horizontal_display_frequency,
                        grid.follow_margins,
                    ))
                    .or_default() += 1;
                if let Some(word2000) = properties.extension.word2000() {
                    document_2000_extensions += 1;
                    *document_2000_level_shapes
                        .entry((word2000.last_bullet_level, word2000.last_numbering_level))
                        .or_default() += 1;
                    *document_2000_flag_shapes
                        .entry(word2000.flags.bits().map_err(|error| error.to_string())?)
                        .or_default() += 1;
                    *document_2000_screen_sizes
                        .entry(word2000.flags.target_screen_size)
                        .or_default() += 1;
                    if word2000.flags.web_options_initialized {
                        document_2000_initialized_web_options += 1;
                        *document_2000_initialized_ppi
                            .entry(word2000.flags.pixels_per_inch)
                            .or_default() += 1;
                    }
                    *document_copts_named_shapes
                        .entry(word2000.compatibility_options.named_bits())
                        .or_default() += 1;
                    document_copts_cached_column_balance +=
                        usize::from(word2000.compatibility_options.cached_column_balance);
                    document_copts_nonzero_empty1 +=
                        usize::from(word2000.compatibility_options.empty1 != 0);
                    document_copts_nonzero_empty_dwords += word2000
                        .compatibility_options
                        .empty
                        .iter()
                        .filter(|value| **value != 0)
                        .count();
                    document_copts_word8_mismatches +=
                        usize::from(!word2000.compatibility_options_match(&properties.word97));
                    *document_2000_pre_word10_shapes
                        .entry(
                            word2000
                                .pre_word10_features
                                .bits()
                                .map_err(|error| error.to_string())?,
                        )
                        .or_default() += 1;
                    *document_2000_flag2_shapes
                        .entry(word2000.flags2.bits().map_err(|error| error.to_string())?)
                        .or_default() += 1;
                }
                if let Some(word2002) = properties.extension.word2002() {
                    document_2002_extensions += 1;
                    *document_2002_flag_shapes
                        .entry(word2002.flags.bits().map_err(|error| error.to_string())?)
                        .or_default() += 1;
                    *document_2002_line_endings
                        .entry(word2002.flags.text_line_ending)
                        .or_default() += 1;
                    *document_2002_feature_shapes
                        .entry(
                            word2002
                                .feature_compatibility
                                .bits()
                                .map_err(|error| error.to_string())?,
                        )
                        .or_default() += 1;
                    document_2002_default_table_styles.insert(word2002.default_table_style);
                    *document_2002_style_filters
                        .entry(word2002.style_filter)
                        .or_default() += 1;
                    *document_2002_booklet_pages
                        .entry(word2002.booklet_pages)
                        .or_default() += 1;
                    *document_2002_code_pages
                        .entry(word2002.text_code_page)
                        .or_default() += 1;
                    document_2002_nonzero_unused += usize::from(word2002.unused != 0);
                    let positions = [
                        word2002.minimum_revision_positions.main,
                        word2002.minimum_revision_positions.footnote,
                        word2002.minimum_revision_positions.header,
                        word2002.minimum_revision_positions.comment,
                        word2002.minimum_revision_positions.endnote,
                        word2002.minimum_revision_positions.textbox,
                        word2002.minimum_revision_positions.header_textbox,
                    ];
                    for (index, position) in positions.into_iter().enumerate() {
                        document_2002_nonzero_revision_positions[index] +=
                            usize::from(position != 0);
                        document_2002_maximum_revision_positions[index] =
                            document_2002_maximum_revision_positions[index].max(position);
                    }
                    document_2002_nonzero_root_revision_ids +=
                        usize::from(word2002.root_revision_save_id != 0);
                    document_2002_root_revision_ids.insert(word2002.root_revision_save_id);
                }
                if let Some(word2003) = properties.extension.word2003() {
                    current_word2003 = Some(*word2003);
                    document_2003_extensions += 1;
                    *document_2003_flag_shapes
                        .entry(word2003.flags.bits().map_err(|error| error.to_string())?)
                        .or_default() += 1;
                    *document_2003_protection_shapes
                        .entry(word2003.protection.bits())
                        .or_default() += 1;
                    *document_2003_protection_modes
                        .entry(word2003.protection.mode)
                        .or_default() += 1;
                    *document_2003_page_widths
                        .entry(word2003.page_lock_width)
                        .or_default() += 1;
                    *document_2003_page_heights
                        .entry(word2003.page_lock_height)
                        .or_default() += 1;
                    *document_2003_font_percentages
                        .entry(word2003.locked_font_percentage)
                        .or_default() += 1;
                    *document_2003_toolbar_shapes
                        .entry(word2003.state_toolbars.bits())
                        .or_default() += 1;
                    *document_2003_cleanup_limits
                        .entry(word2003.list_override_cleanup_limit)
                        .or_default() += 1;
                }
                if let Some(word2007) = properties.extension.word2007() {
                    document_2007_extensions += 1;
                    *document_2007_reserved_values
                        .entry(word2007.reserved)
                        .or_default() += 1;
                    *document_2007_flag_shapes
                        .entry(word2007.flags.bits())
                        .or_default() += 1;
                    *document_2007_style_sort_methods
                        .entry(word2007.flags.style_sort_method)
                        .or_default() += 1;
                    *document_math_flag_shapes
                        .entry(word2007.math.flag_bits())
                        .or_default() += 1;
                    *document_math_enum_shapes
                        .entry((
                            word2007.math.binary_operator_break,
                            word2007.math.binary_subtraction_break,
                            word2007.math.justification,
                        ))
                        .or_default() += 1;
                    *document_math_fixed_constants
                        .entry(word2007.math.fixed_constants)
                        .or_default() += 1;
                    *document_math_font_indexes
                        .entry(word2007.math.font_index)
                        .or_default() += 1;
                    *document_math_left_margins
                        .entry(word2007.math.left_margin)
                        .or_default() += 1;
                    *document_math_right_margins
                        .entry(word2007.math.right_margin)
                        .or_default() += 1;
                    *document_math_wrapped_indents
                        .entry(word2007.math.wrapped_line_indent)
                        .or_default() += 1;
                }
                if let Some(word2010) = properties.extension.word2010() {
                    document_2010_extensions += 1;
                    match word2010.paragraph_identifier_context {
                        ParagraphIdentifierContext::Standard(value) => {
                            document_2010_standard_contexts.insert(value);
                        }
                        ParagraphIdentifierContext::ProducerCompatibilityZero => {
                            document_2010_compatibility_zero_contexts += 1;
                        }
                    }
                    *document_2010_reserved_values
                        .entry(word2010.reserved)
                        .or_default() += 1;
                    *document_2010_discard_image_data
                        .entry(word2010.discard_image_editing_data)
                        .or_default() += 1;
                    *document_2010_image_resolutions
                        .entry(word2010.image_resolution_dpi)
                        .or_default() += 1;
                }
                if let Some(word2013) = properties.extension.word2013() {
                    *document_2013_chart_tracking
                        .entry(word2013.chart_tracking_reference_based)
                        .or_default() += 1;
                }
                let info = properties.word97.auto_summary;
                *auto_summary_info_shapes
                    .entry((
                        info.valid,
                        info.view_active,
                        info.view_by,
                        info.update_properties,
                        info.desired_size,
                        info.highest_level,
                        info.current_level,
                    ))
                    .or_default() += 1;
                auto_summary_info = Some(info);
            }
            if let Some(location) = fib.auto_summary_ranges_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PlcfAsumy")?;
                let ranges = AutoSummaryRangeTable::from_bytes(physical)
                    .map_err(|error| format!("PlcfAsumy: {error}"))?;
                if ranges.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PlcfAsumy write did not reproduce its physical bytes".to_owned());
                }
                let info = auto_summary_info
                    .as_ref()
                    .ok_or_else(|| "PlcfAsumy has no corresponding Dop Asumyi".to_owned())?;
                ranges
                    .validate_against(info)
                    .map_err(|error| format!("PlcfAsumy/Asumyi: {error}"))?;
                auto_summary_range_tables += 1;
                auto_summary_ranges += ranges.priorities.len();
                *auto_summary_range_count_shapes
                    .entry(ranges.priorities.len())
                    .or_default() += 1;
                for priority in ranges.priorities {
                    *auto_summary_priority_shapes
                        .entry(priority.level)
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.associated_strings_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbfAssoc")?;
                let strings = AssociatedStrings::from_bytes(physical).map_err(|error| {
                    format!(
                        "SttbfAssoc fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if strings.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbfAssoc write did not reproduce its physical bytes".to_owned());
                }
                associated_string_tables += 1;
                *associated_string_padding
                    .entry(strings.trailing_zero_words)
                    .or_default() += 1;
                for (index, string) in strings.iter().enumerate() {
                    associated_string_units += string.len();
                    if !string.is_empty() {
                        *nonempty_associated_strings.entry(index).or_default() += 1;
                    }
                    maximum_associated_string_lengths
                        .entry(index)
                        .and_modify(|length| *length = (*length).max(string.len()))
                        .or_insert(string.len());
                }
            }
            if let Some(location) = fib.user_variables_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "StwUser")?;
                let variables = UserVariables::from_bytes(physical)
                    .map_err(|error| format!("StwUser: {error}"))?;
                if variables.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("StwUser write did not reproduce its physical bytes".to_owned());
                }
                user_variable_tables += 1;
                user_variables += variables.variables.len();
                *user_variable_table_shapes
                    .entry((variables.variables.len(), location.lcb))
                    .or_default() += 1;
                for variable in variables.variables {
                    user_variable_name_units += variable.name.len();
                    user_variable_value_units += variable.value.len();
                    maximum_user_variable_value_units =
                        maximum_user_variable_value_units.max(variable.value.len());
                    user_variable_nonzero_metadata +=
                        usize::from(variable.ignored_name_metadata != 0);
                    *user_variable_kinds.entry(variable.kind()).or_default() += 1;
                }
            }
            if let Some(location) = fib.revision_authors_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbfRMark")?;
                let authors = RevisionAuthors::from_bytes(physical).map_err(|error| {
                    format!(
                        "SttbfRMark fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if authors.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbfRMark write did not reproduce its physical bytes".to_owned());
                }
                if let RevisionAuthors::Standard { names } = &authors {
                    current_revision_author_count = Some(names.len());
                }
                revision_author_tables += 1;
                revision_author_zero_placeholders += usize::from(matches!(
                    authors,
                    RevisionAuthors::CompatibilityZeroPlaceholder
                ));
                revision_authors += authors.names().len();
                *revision_author_count_shapes
                    .entry(authors.names().len())
                    .or_default() += 1;
                for author in authors.names() {
                    revision_author_units += author.len();
                    maximum_revision_author_length =
                        maximum_revision_author_length.max(author.len());
                }
            }
            if let Some(location) = fib.spelling_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Plcfspl")?;
                let states = SpellingStateTable::from_bytes(physical).map_err(|error| {
                    format!(
                        "Plcfspl fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if states.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Plcfspl write did not reproduce its physical bytes".to_owned());
                }
                spelling_state_tables += 1;
                spelling_ranges += states.states.len();
                spelling_duplicate_positions += states
                    .positions
                    .windows(2)
                    .filter(|positions| positions[0] == positions[1])
                    .count();
                for state in states.states {
                    *spelling_state_shapes
                        .entry((state.kind, state.error))
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.grammar_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Plcfgram")?;
                let states = GrammarStateTable::from_bytes(physical).map_err(|error| {
                    format!(
                        "Plcfgram fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if states.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Plcfgram write did not reproduce its physical bytes".to_owned());
                }
                grammar_state_tables += 1;
                grammar_ranges += states.states.len();
                grammar_duplicate_positions += states
                    .positions
                    .windows(2)
                    .filter(|positions| positions[0] == positions[1])
                    .count();
                for state in states.states {
                    *grammar_state_shapes
                        .entry((state.kind, state.error, state.extend, state.typo))
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.language_detection_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Plcflad")?;
                let states =
                    LanguageDetectionStateTable::from_bytes(physical).map_err(|error| {
                        format!(
                            "Plcflad fc={:#x} lcb={:#x}: {error}",
                            location.fc, location.lcb
                        )
                    })?;
                if states.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Plcflad write did not reproduce its physical bytes".to_owned());
                }
                language_detection_state_tables += 1;
                language_detection_ranges += states.states.len();
                language_detection_duplicate_positions += states
                    .positions
                    .windows(2)
                    .filter(|positions| positions[0] == positions[1])
                    .count();
                for state in states.states {
                    *language_detection_state_shapes
                        .entry((state.kind, state.error))
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.list_style_templates_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbRgtplc")?;
                let templates = ListStyleTemplates::from_bytes(physical).map_err(|error| {
                    format!(
                        "SttbRgtplc fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if templates.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbRgtplc write did not reproduce its physical bytes".to_owned());
                }
                list_style_template_tables += 1;
                list_style_template_lists += templates.lists.len();
                current_list_style_template_count = Some(templates.lists.len());
                for list in templates.lists {
                    if let Some(levels) = list {
                        for level in levels {
                            match level {
                                ListLevelTemplateCode::BuiltIn { .. } => {
                                    built_in_list_level_templates += 1;
                                }
                                ListLevelTemplateCode::UserDefined { .. } => {
                                    user_list_level_templates += 1;
                                }
                            }
                        }
                    } else {
                        empty_list_style_templates += 1;
                    }
                }
            }
            if let Some(location) = fib.frame_and_list_records_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "RgDofr")?;
                let records = FrameAndListRecords::from_bytes(physical)
                    .map_err(|error| format!("RgDofr: {error}"))?;
                if records.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("RgDofr write did not reproduce its physical bytes".to_owned());
                }
                frame_and_list_tables += 1;
                frame_and_list_records += records.records.len();
                for record in records.records {
                    let FrameAndListRecord::ListStyles(styles) = record else {
                        return Err("RgDofr corpus contains a non-list-style record".to_owned());
                    };
                    list_style_references += styles.len();
                    for style in styles {
                        if style.style_definition {
                            custom_list_style_references += 1;
                            current_custom_list_style_indices.push(style.list_index);
                        } else {
                            standard_list_style_references += 1;
                        }
                    }
                }
            }
            if let Some(location) = fib.grammar_option_sets_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PlfCosl")?;
                let sets = GrammarOptionSets::from_bytes(physical)
                    .map_err(|error| format!("PlfCosl: {error}"))?;
                if sets.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PlfCosl write did not reproduce its physical bytes".to_owned());
                }
                grammar_option_tables += 1;
                grammar_options += sets.options.len();
                for option in sets.options {
                    *grammar_option_shapes
                        .entry((
                            option.option_set,
                            option.language_id,
                            option.checker_version,
                            option.company_id,
                        ))
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.legacy_grammar_option_sets_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PlfGosl")?;
                let sets = LegacyGrammarOptionSets::from_bytes(physical)
                    .map_err(|error| format!("PlfGosl: {error}"))?;
                if sets.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PlfGosl writer changed physical bytes".to_owned());
                }
                legacy_grammar_option_tables += 1;
                legacy_grammar_options += sets.options.len();
                for option in sets.options {
                    *legacy_grammar_option_shapes
                        .entry((
                            option.option_set,
                            option.language_id,
                            option.checker_version,
                            option.company_id,
                        ))
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.smart_tag_recognizer_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Plcffactoid")?;
                let states = SmartTagRecognizerStateTable::from_bytes(physical)
                    .map_err(|error| format!("Plcffactoid: {error}"))?;
                if states.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Plcffactoid write did not reproduce its physical bytes".to_owned());
                }
                smart_tag_state_tables += 1;
                smart_tag_state_ranges += states.states.len();
                smart_tag_duplicate_positions += states
                    .positions
                    .windows(2)
                    .filter(|positions| positions[0] == positions[1])
                    .count();
                for state in states.states {
                    *smart_tag_state_shapes.entry(state.kind).or_default() += 1;
                }
            }
            if let Some(location) = fib.paragraph_group_properties_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PGPArray")?;
                let properties = ParagraphGroupProperties::from_bytes(physical)
                    .map_err(|error| format!("PGPArray: {error}"))?;
                if properties.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PGPArray write did not reproduce its physical bytes".to_owned());
                }
                paragraph_group_tables += 1;
                paragraph_group_entries += properties.entries.len();
                let ids = properties
                    .entries
                    .iter()
                    .map(|entry| entry.id)
                    .collect::<BTreeSet<_>>();
                for entry in properties.entries {
                    paragraph_group_root_entries += usize::from(entry.parent_id == 0);
                    paragraph_group_maximum_depth =
                        paragraph_group_maximum_depth.max(entry.table_depth);
                    paragraph_group_missing_parents +=
                        usize::from(entry.parent_id != 0 && !ids.contains(&entry.parent_id));
                    let options = entry.options;
                    let shape = u16::from(options.left_margin.is_some())
                        | (u16::from(options.right_margin.is_some()) << 1)
                        | (u16::from(options.top_margin.is_some()) << 2)
                        | (u16::from(options.bottom_margin.is_some()) << 3)
                        | (u16::from(options.left_border.is_some()) << 4)
                        | (u16::from(options.right_border.is_some()) << 5)
                        | (u16::from(options.top_border.is_some()) << 6)
                        | (u16::from(options.bottom_border.is_some()) << 7)
                        | (u16::from(options.html_block_type.is_some()) << 8);
                    *paragraph_group_option_shapes.entry(shape).or_default() += 1;
                    if let Some(value) = options.html_block_type {
                        *paragraph_group_html_types.entry(value).or_default() += 1;
                    }
                }
            }
            if let Some(location) = fib.save_history_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbSavedBy")?;
                let history = SaveHistory::from_bytes(physical)
                    .map_err(|error| format!("SttbSavedBy: {error}"))?;
                if history.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SttbSavedBy write did not reproduce its physical bytes".to_owned());
                }
                save_history_tables += 1;
                save_history_entries += history.entries.len();
                *save_history_entry_counts
                    .entry(history.entries.len())
                    .or_default() += 1;
                for entry in history.entries {
                    save_history_author_units += entry.author.len();
                    save_history_path_units += entry.path.len();
                    save_history_maximum_author_length =
                        save_history_maximum_author_length.max(entry.author.len());
                    save_history_maximum_path_length =
                        save_history_maximum_path_length.max(entry.path.len());
                }
            }
            if let Some([info_location, start_location, end_location]) =
                fib.smart_tag_bookmark_locations()
                && [info_location, start_location, end_location]
                    .iter()
                    .any(|location| location.lcb != 0)
            {
                if [info_location, start_location, end_location]
                    .iter()
                    .any(|location| location.lcb == 0)
                {
                    return Err("smart-tag bookmark tables are only partially present".to_owned());
                }
                let info_bytes = bounded_slice(
                    table,
                    info_location.fc,
                    info_location.lcb,
                    "SttbfBkmkFactoid",
                )?;
                let start_bytes = bounded_slice(
                    table,
                    start_location.fc,
                    start_location.lcb,
                    "PlcfBkfFactoid",
                )?;
                let end_bytes =
                    bounded_slice(table, end_location.fc, end_location.lcb, "PlcfBklFactoid")?;
                let bookmarks = SmartTagBookmarks::from_bytes(info_bytes, start_bytes, end_bytes)
                    .map_err(|error| format!("smart-tag bookmarks: {error}"))?;
                let written = bookmarks.to_bytes().map_err(|error| error.to_string())?;
                if written.0 != info_bytes || written.1 != start_bytes || written.2 != end_bytes {
                    return Err("smart-tag bookmark writers changed physical bytes".to_owned());
                }
                smart_tag_bookmark_tables += 1;
                smart_tag_bookmarks += bookmarks.infos.len();
                current_smart_tag_bookmark_count = Some(bookmarks.infos.len());
                for info in bookmarks.infos {
                    smart_tag_sub_entities += usize::from(info.sub_entity);
                    smart_tag_nonzero_unused += usize::from(info.unused != 0);
                    smart_tag_nonzero_property_bag_pointers +=
                        usize::from(info.ignored_property_bag_pointer != 0);
                    *smart_tag_sources.entry(info.source).or_default() += 1;
                }
                for start in bookmarks.starts.bookmarks {
                    *smart_tag_start_depths.entry(start.depth).or_default() += 1;
                }
                for end in bookmarks.ends.bookmarks {
                    *smart_tag_end_depths.entry(end.depth).or_default() += 1;
                }
            }
            let mut grammar_cookie_references = BTreeSet::new();
            if let Some(location) = fib.legacy_grammar_checker_cookies_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PlcfcookieOld")?;
                let cookies = LegacyGrammarCheckerCookieTable::from_bytes(physical)
                    .map_err(|error| format!("PlcfcookieOld: {error}"))?;
                if cookies.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PlcfcookieOld writer changed physical bytes".to_owned());
                }
                let store = current_grammar_cookie_store
                    .as_ref()
                    .ok_or_else(|| "PlcfcookieOld has no corresponding RgCdb".to_owned())?;
                store
                    .validate_legacy_references(&cookies)
                    .map_err(|error| format!("PlcfcookieOld/RgCdb: {error}"))?;
                legacy_grammar_cookie_tables += 1;
                legacy_grammar_cookies += cookies.cookies.len();
                legacy_grammar_cookie_duplicate_positions += cookies
                    .positions
                    .windows(2)
                    .filter(|positions| positions[0] == positions[1])
                    .count();
                for cookie in cookies.cookies {
                    grammar_cookie_references.insert(cookie.data_offset);
                    legacy_grammar_cookie_errors += usize::from(cookie.error);
                    *legacy_grammar_cookie_shapes
                        .entry((
                            cookie.language_id,
                            cookie.character_count,
                            cookie.sentence_offset,
                            cookie.padding1,
                            cookie.error_type,
                            cookie.spare,
                            cookie.error,
                            cookie.padding2,
                            cookie.data_offset,
                        ))
                        .or_default() += 1;
                }
            }
            if let Some(location) = fib.grammar_checker_cookies_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Plcfcookie")?;
                let cookies = GrammarCheckerCookieTable::from_bytes(physical)
                    .map_err(|error| format!("Plcfcookie: {error}"))?;
                if cookies.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Plcfcookie writer changed physical bytes".to_owned());
                }
                let store = current_grammar_cookie_store
                    .as_ref()
                    .ok_or_else(|| "Plcfcookie has no corresponding RgCdb".to_owned())?;
                store
                    .validate_references(&cookies)
                    .map_err(|error| format!("Plcfcookie/RgCdb: {error}"))?;
                grammar_cookie_tables += 1;
                grammar_cookies += cookies.cookies.len();
                grammar_cookie_duplicate_positions += cookies
                    .positions
                    .windows(2)
                    .filter(|positions| positions[0] == positions[1])
                    .count();
                for cookie in cookies.cookies {
                    grammar_cookie_references.insert(cookie.data_offset);
                    grammar_cookie_headers += usize::from(cookie.header);
                    grammar_cookie_errors += usize::from(cookie.error);
                    *grammar_cookie_error_types
                        .entry(cookie.error_type)
                        .or_default() += 1;
                    *grammar_cookie_languages
                        .entry((cookie.language_sub, cookie.language_primary))
                        .or_default() += 1;
                    *grammar_cookie_shapes
                        .entry((
                            cookie.character_count,
                            cookie.sentence_offset,
                            cookie.data_offset,
                            cookie.error_type,
                            cookie.error,
                            cookie.language_sub,
                            cookie.language_primary,
                            cookie.header,
                        ))
                        .or_default() += 1;
                }
            }
            if let Some(store) = current_grammar_cookie_store.as_ref() {
                grammar_cookie_unreferenced_data += store
                    .entry_offsets()
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .filter(|offset| !grammar_cookie_references.contains(offset))
                    .count();
            }
            if let Some(location) = fib.smart_tag_data_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SmartTagData")?;
                let data = SmartTagData::from_bytes(physical)
                    .map_err(|error| format!("SmartTagData: {error}"))?;
                if data.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("SmartTagData writer changed physical bytes".to_owned());
                }
                smart_tag_data_tables += 1;
                smart_tag_factoid_types += data.factoid_types.len();
                smart_tag_malformed_cve_factoid_types += data
                    .factoid_types
                    .iter()
                    .filter(|value| value.id == SmartTagFactoidTypeId::MalformedCve20163133)
                    .count();
                smart_tag_property_bags += data.property_bags.len();
                *smart_tag_reserved_factoid_counts
                    .entry(data.reserved_factoid_count)
                    .or_default() += 1;
                if let Some(bookmark_count) = current_smart_tag_bookmark_count {
                    smart_tag_property_bag_count_mismatches +=
                        usize::from(bookmark_count != data.property_bags.len());
                }
                for value in data
                    .factoid_types
                    .iter()
                    .flat_map(|value| [&value.uri, &value.tag, &value.download_url])
                    .chain(data.strings.iter())
                {
                    match value {
                        PropertyBagString::Ansi(_) => smart_tag_ansi_strings += 1,
                        PropertyBagString::Unicode(_) => smart_tag_unicode_strings += 1,
                    }
                }
                smart_tag_properties += data
                    .property_bags
                    .iter()
                    .map(|bag| bag.properties.len())
                    .sum::<usize>();
            }
            if let Some(location) = fib.table_character_cache_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PlcfTch")?;
                let caches = TableCharacterCacheTable::from_bytes(physical).map_err(|error| {
                    format!(
                        "PlcfTch fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if caches.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PlcfTch write did not reproduce its physical bytes".to_owned());
                }
                table_character_cache_tables += 1;
                table_character_cache_ranges += caches.caches.len();
                *table_character_cache_shapes
                    .entry(caches.caches.len())
                    .or_default() += 1;
                table_character_unknown_ranges +=
                    caches.caches.iter().filter(|cache| cache.unknown).count();
                table_character_nonzero_unused += caches
                    .caches
                    .iter()
                    .filter(|cache| cache.unused != 0)
                    .count();
                if let Ok(text_length) = u32::try_from(fib.rg_lw.ccp_text)
                    && caches.positions == [0, text_length, text_length.saturating_add(2)]
                    && caches.caches.len() == 2
                    && !caches.caches[0].unknown
                    && caches.caches[0].unused == 0
                    && caches.caches[1].unknown
                    && caches.caches[1].unused == 0
                {
                    table_character_canonical_undefined += 1;
                }
            }
            if let Some(location) = fib.revision_message_threading_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "RmdThreading")?;
                let threading =
                    RevisionMessageThreading::from_bytes(physical).map_err(|error| {
                        format!(
                            "RmdThreading fc={:#x} lcb={:#x}: {error}",
                            location.fc, location.lcb
                        )
                    })?;
                if threading.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err(
                        "RmdThreading write did not reproduce its physical bytes".to_owned()
                    );
                }
                revision_threading_tables += 1;
                revision_thread_messages += threading.messages.len();
                *revision_thread_message_count_shapes
                    .entry(threading.messages.len())
                    .or_default() += 1;
                if let Some(author_count) = current_revision_author_count {
                    revision_thread_author_count_mismatches +=
                        usize::from(author_count != threading.messages.len());
                }
                for message in &threading.messages {
                    revision_thread_message_units += message.identifier.len();
                    revision_thread_nonempty_messages +=
                        usize::from(!message.identifier.is_empty());
                    let created = message.display.created;
                    revision_thread_nonzero_dates += usize::from(
                        created.minute != 0
                            || created.hour != 0
                            || created.day != 0
                            || created.month != 0
                            || created.year_offset != 0
                            || created.weekday != 0,
                    );
                    revision_thread_nonzero_reserved += usize::from(message.display.reserved != 0);
                    *revision_thread_author_indexes
                        .entry(message.display.author_index)
                        .or_default() += 1;
                }
                for style in &threading.styles {
                    revision_thread_style_units += style.len();
                    revision_thread_nonempty_styles += usize::from(!style.is_empty());
                }
                revision_thread_author_attributes += threading.author_attributes.len();
                revision_thread_message_attributes += threading.message_attributes.len();
                revision_thread_attribute_units += threading
                    .author_attributes
                    .iter()
                    .chain(&threading.message_attributes)
                    .map(|attribute| attribute.name.len())
                    .sum::<usize>();
                revision_thread_value_units += threading
                    .author_values
                    .iter()
                    .chain(&threading.message_values)
                    .map(Vec::len)
                    .sum::<usize>();
            }
            if let Some(location) = fib.revision_save_ids_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "PLRSID")?;
                let ids = RevisionSaveIdTable::from_bytes(physical).map_err(|error| {
                    format!(
                        "PLRSID fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if ids.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PLRSID write did not reproduce its physical bytes".to_owned());
                }
                revision_save_id_tables += 1;
                revision_save_ids += ids.ids.len();
                *revision_save_id_count_shapes
                    .entry(ids.ids.len())
                    .or_default() += 1;
                *revision_save_id_reserved2.entry(ids.reserved2).or_default() += 1;
                nonzero_revision_save_id_reserved3 += usize::from(ids.reserved3 != 0);
                let mut seen = BTreeSet::new();
                for id in ids.ids {
                    zero_revision_save_ids += usize::from(id.0 == 0);
                    duplicate_revision_save_ids += usize::from(!seen.insert(id.0));
                    distinct_revision_save_ids.insert(id.0);
                }
            }
            if let Some(location) = fib.selection_state_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "Selsf")?;
                let selection = SelectionState::from_bytes(physical)
                    .map_err(|error| format!("Selsf: {error}"))?;
                if selection.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("Selsf write did not reproduce its physical bytes".to_owned());
                }
                selection_states += 1;
                *selection_ranges
                    .entry(match selection.range {
                        SelectionRange::Unused(_) => "unused",
                        SelectionRange::Block { .. } => "block",
                        SelectionRange::Table { .. } => "table",
                    })
                    .or_default() += 1;
                *selection_styles.entry(selection.style).or_default() += 1;
                if let SelectionStateExtension::Compatibility(words) = selection.extension {
                    *selection_extensions.entry(words).or_default() += 1;
                }
            }
            let mut parsed_list_ids = BTreeSet::new();
            let mut current_list_definition_count = 0usize;
            if let Some(location) = fib.list_definition_location()
                && location.lcb != 0
            {
                let definitions =
                    ListDefinitions::from_table_stream(table, location).map_err(|error| {
                        let start = usize::try_from(location.fc).unwrap_or(usize::MAX);
                        let prefix = table
                            .get(start..table.len().min(start.saturating_add(32)))
                            .unwrap_or_default();
                        let count = prefix
                            .get(..2)
                            .and_then(|value| <[u8; 2]>::try_from(value).ok())
                            .map(u16::from_le_bytes)
                            .unwrap_or(0);
                        let level_start = start.saturating_add(2 + usize::from(count) * 28);
                        let level_prefix = table
                            .get(level_start..table.len().min(level_start.saturating_add(96)))
                            .unwrap_or_default();
                        format!(
                            "PlfLst fc={:#x} lcb={:#x} PlfLfo={:?} prefix={prefix:02x?} level={level_prefix:02x?}: {error}",
                            location.fc,
                            location.lcb,
                            fib.list_override_location()
                        )
                    })?;
                let written = definitions.to_bytes().map_err(|error| error.to_string())?;
                list_levels_in_declared_length +=
                    usize::from(definitions.levels_in_declared_length);
                let base = bounded_slice(table, location.fc, location.lcb, "PlfLst")?;
                if written.0 != base {
                    return Err("PlfLst writer changed physical bytes".to_owned());
                }
                let level_start = usize::try_from(location.fc)
                    .map_err(|_| "PlfLst fc exceeds usize".to_owned())?
                    .checked_add(
                        usize::try_from(location.lcb)
                            .map_err(|_| "PlfLst lcb exceeds usize".to_owned())?,
                    )
                    .ok_or_else(|| "PlfLst level offset overflow".to_owned())?;
                let level_end = level_start
                    .checked_add(written.1.len())
                    .ok_or_else(|| "PlfLst level end overflow".to_owned())?;
                if table.get(level_start..level_end) != Some(written.1.as_slice()) {
                    return Err("LVL writer changed physical bytes".to_owned());
                }
                if let Some(override_location) = fib.list_override_location()
                    && override_location.lcb != 0
                {
                    *list_level_to_override_gaps
                        .entry(i64::from(override_location.fc) - level_end as i64)
                        .or_default() += 1;
                }
                list_definition_sets += 1;
                list_definitions += definitions.definitions.len();
                current_list_definition_count = definitions.definitions.len();
                simple_list_definitions += definitions
                    .definitions
                    .iter()
                    .filter(|definition| definition.info.simple)
                    .count();
                parsed_list_ids.extend(
                    definitions
                        .definitions
                        .iter()
                        .map(|definition| definition.info.list_id),
                );
                list_level_bytes +=
                    written.0.len() + written.1.len() - (2 + definitions.definitions.len() * 28);
                for definition in definitions.definitions {
                    list_levels += definition.levels.len();
                    for level in definition.levels {
                        if !level.paragraph_incomplete_prl_tail.is_empty() {
                            *list_level_incomplete_tails
                                .entry(("paragraph", level.paragraph_incomplete_prl_tail.len()))
                                .or_default() += 1;
                        }
                        if !level.number_incomplete_prl_tail.is_empty() {
                            *list_level_incomplete_tails
                                .entry(("character", level.number_incomplete_prl_tail.len()))
                                .or_default() += 1;
                        }
                        list_level_paragraph_prls += level.paragraph_properties.properties.len();
                        list_level_character_prls += level.number_properties.properties.len();
                        list_level_text_units += level.number_text.len();
                    }
                }
            }
            if let Some(location) = fib.list_names_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "SttbListNames")?;
                let names = ListNamesTable::from_bytes(physical).map_err(|error| {
                    format!(
                        "SttbListNames fc={:#x} lcb={:#x}: {error}",
                        location.fc, location.lcb
                    )
                })?;
                if names.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err(
                        "SttbListNames write did not reproduce its physical bytes".to_owned()
                    );
                }
                list_name_tables += 1;
                list_name_entries += names.names.len();
                *list_name_count_shapes.entry(names.names.len()).or_default() += 1;
                *list_name_definition_count_differences
                    .entry(names.names.len() as i64 - current_list_definition_count as i64)
                    .or_default() += 1;
                for name in names.names {
                    nonempty_list_names += usize::from(!name.is_empty());
                    list_name_units += name.len();
                    maximum_list_name_length = maximum_list_name_length.max(name.len());
                }
            }
            if let Some(template_count) = current_list_style_template_count {
                if template_count != current_list_definition_count {
                    *extra_list_style_template_counts
                        .entry(template_count.saturating_sub(current_list_definition_count))
                        .or_default() += 1;
                }
                list_style_template_count_mismatches +=
                    usize::from(template_count != current_list_definition_count);
            }
            out_of_range_custom_list_style_references += current_custom_list_style_indices
                .iter()
                .filter(|index| usize::from(**index) >= current_list_definition_count)
                .count();
            let mut current_list_override_count = 0usize;
            if let Some(location) = fib.list_override_location()
                && location.lcb != 0
            {
                let bytes = bounded_slice(table, location.fc, location.lcb, "PlfLfo")?;
                let overrides = ListOverrides::from_bytes(bytes).map_err(|error| {
                    format!(
                        "PlfLfo fc={:#x} lcb={:#x} prefix={:02x?} suffix={:02x?}: {error}",
                        location.fc,
                        location.lcb,
                        &bytes[..bytes.len().min(64)],
                        &bytes[bytes.len().saturating_sub(64)..]
                    )
                })?;
                if overrides.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err("PlfLfo writer changed physical bytes".to_owned());
                }
                current_list_override_count = overrides.overrides.len();
                list_override_sets += 1;
                list_overrides += overrides.overrides.len();
                for value in overrides.overrides {
                    list_override_missing_definitions +=
                        usize::from(!parsed_list_ids.contains(&value.info.list_id));
                    list_override_levels += value.data.levels.len();
                    for level in value.data.levels {
                        if let Some(level) = level.level {
                            formatted_list_override_levels += 1;
                            list_override_level_prls += level.paragraph_properties.properties.len();
                            list_override_level_prls += level.number_properties.properties.len();
                            list_override_text_units += level.number_text.len();
                        }
                    }
                }
            }
            if let Some(indexes) = current_last_list_indexes {
                if indexes.matches_override_count(current_list_override_count) {
                    document_last_list_index_matches += 1;
                } else {
                    *document_last_list_index_mismatches
                        .entry((
                            indexes.bullet,
                            indexes.numbering,
                            current_list_override_count,
                        ))
                        .or_default() += 1;
                }
            }
            if let Some(word2003) = current_word2003 {
                if word2003.cleanup_limit_matches_override_count(current_list_override_count) {
                    document_cleanup_limit_matches += 1;
                } else {
                    *document_cleanup_limit_mismatches
                        .entry((
                            word2003.list_override_cleanup_limit,
                            current_list_override_count,
                        ))
                        .or_default() += 1;
                }
            }
            let mut current_field_counts = BTreeMap::<FieldDocumentPart, usize>::new();
            for (part, location) in fib.field_table_locations() {
                if location.lcb == 0 {
                    continue;
                }
                let field_bytes = bounded_slice(table, location.fc, location.lcb, "Plcfld")?;
                let fields = &file
                    .table
                    .fields
                    .get(&part)
                    .ok_or_else(|| format!("typed DOC tree omitted {part:?} Plcfld"))?
                    .value;
                if fields.to_bytes().map_err(|error| error.to_string())? != field_bytes {
                    return Err(format!("{part:?} Plcfld write changed physical bytes"));
                }
                *field_tables.entry(part).or_default() += 1;
                let mut current_count = 0usize;
                let mut pending = fields.fields.iter().collect::<Vec<_>>();
                while let Some(field) = pending.pop() {
                    current_count += 2 + usize::from(field.separator.is_some());
                    *field_character_counts.entry((part, 0x13)).or_default() += 1;
                    *field_reserved_counts
                        .entry(field.begin.reserved)
                        .or_default() += 1;
                    *field_type_counts.entry(field.begin.field_type).or_default() += 1;
                    if let Some(separator) = field.separator {
                        *field_character_counts.entry((part, 0x14)).or_default() += 1;
                        *field_reserved_counts.entry(separator.reserved).or_default() += 1;
                    }
                    *field_character_counts.entry((part, 0x15)).or_default() += 1;
                    *field_reserved_counts.entry(field.end.reserved).or_default() += 1;
                    pending.extend(&field.instruction_fields);
                    pending.extend(&field.result_fields);
                }
                field_records += current_count;
                current_field_counts.insert(part, current_count);
            }
            if let Some(location) = fib.ole_control_info_location()
                && location.lcb != 0
            {
                let physical = bounded_slice(table, location.fc, location.lcb, "RgxOcxInfo")?;
                let infos = OleControlInfos::from_bytes(physical)
                    .map_err(|error| format!("RgxOcxInfo: {error}"))?;
                if infos.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("RgxOcxInfo writer changed physical bytes".to_owned());
                }
                ole_control_info_tables += 1;
                ole_control_infos += infos.controls.len();
                let mut seen_cookies = BTreeSet::new();
                for (index, control) in infos.controls.into_iter().enumerate() {
                    ole_control_duplicate_cookies +=
                        usize::from(!seen_cookies.insert(control.cookie));
                    ole_control_cookie_index_mismatches +=
                        usize::from(usize::try_from(control.cookie).ok() != Some(index));
                    let field_part = match control.document_part {
                        OleControlDocumentPart::Main => Some(FieldDocumentPart::Main),
                        OleControlDocumentPart::Header => Some(FieldDocumentPart::Header),
                        OleControlDocumentPart::Footnote => Some(FieldDocumentPart::Footnote),
                        OleControlDocumentPart::Textbox => Some(FieldDocumentPart::Textbox),
                        OleControlDocumentPart::Endnote => Some(FieldDocumentPart::Endnote),
                        OleControlDocumentPart::Comment => Some(FieldDocumentPart::Comment),
                        OleControlDocumentPart::HeaderTextbox => {
                            Some(FieldDocumentPart::HeaderTextbox)
                        }
                        OleControlDocumentPart::Compatibility(_) => {
                            ole_control_compatibility_document_parts += 1;
                            None
                        }
                    };
                    if control.field_linked {
                        ole_control_field_reference_mismatches +=
                            usize::from(field_part.is_none_or(|field_part| {
                                current_field_counts
                                    .get(&field_part)
                                    .is_none_or(|field_count| {
                                        usize::try_from(control.field_index)
                                            .map_or(true, |field_index| field_index >= *field_count)
                                    })
                            }));
                    } else {
                        ole_control_unlinked_fields += 1;
                    }
                    *ole_control_document_parts
                        .entry(control.document_part)
                        .or_default() += 1;
                    ole_control_nonzero_accelerator_handles +=
                        usize::from(control.ignored_accelerator_handle != 0);
                    ole_control_nonzero_accelerator_counts +=
                        usize::from(control.accelerator_count != 0);
                    ole_control_failed_load += usize::from(control.failed_load);
                    ole_control_corrupt += usize::from(control.corrupt);
                    *ole_control_behavior_flags
                        .entry((
                            control.eats_return,
                            control.eats_escape,
                            control.default_button,
                            control.cancel_button,
                            control.right_to_left,
                        ))
                        .or_default() += 1;
                    ole_control_nonzero_reserved1 += usize::from(control.reserved1 != 0);
                    ole_control_nonzero_reserved2 += usize::from(control.reserved2 != 0);
                }
            }
            if let Some((name_location, start_location, end_location)) = fib.bookmark_locations() {
                let lengths = [name_location.lcb, start_location.lcb, end_location.lcb];
                if lengths.iter().any(|length| *length != 0) {
                    if lengths.contains(&0) {
                        return Err("parallel standard bookmark table is missing".to_owned());
                    }
                    let name_bytes =
                        bounded_slice(table, name_location.fc, name_location.lcb, "SttbfBkmk")?;
                    let start_bytes =
                        bounded_slice(table, start_location.fc, start_location.lcb, "Plcfbkf")?;
                    let end_bytes =
                        bounded_slice(table, end_location.fc, end_location.lcb, "Plcfbkl")?;
                    let bookmarks = Bookmarks::from_bytes(name_bytes, start_bytes, end_bytes)
                        .map_err(|error| format!("bookmarks: {error}"))?;
                    let written = bookmarks.to_bytes().map_err(|error| error.to_string())?;
                    if written.0 != name_bytes || written.1 != start_bytes || written.2 != end_bytes
                    {
                        return Err("bookmark writer changed physical bytes".to_owned());
                    }
                    bookmark_sets += 1;
                    bookmarks_count += bookmarks.names.names.len();
                    bookmark_name_units +=
                        bookmarks.names.names.iter().map(Vec::len).sum::<usize>();
                    hidden_bookmarks += bookmarks
                        .names
                        .names
                        .iter()
                        .filter(|name| name.first() == Some(&(b'_' as u16)))
                        .count();
                    column_bookmarks += bookmarks
                        .starts
                        .bookmarks
                        .iter()
                        .filter(|bookmark| bookmark.column)
                        .count();
                }
            }
            if let Some(location) = fib.header_text_location()
                && location.lcb != 0
            {
                let bytes = bounded_slice(table, location.fc, location.lcb, "Plcfhdd")?;
                let headers = HeaderTextTable::from_bytes(bytes)
                    .map_err(|error| format!("Plcfhdd: {error}"))?;
                if headers.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err("Plcfhdd writer changed physical bytes".to_owned());
                }
                header_tables += 1;
                header_boundaries += headers.boundaries.len();
                missing_header_boundaries += headers
                    .boundaries
                    .iter()
                    .filter(|boundary| matches!(boundary, HeaderStoryBoundary::Missing))
                    .count();
            }
            if let Some((reference_location, text_location)) = fib.footnote_locations() {
                let lengths = [reference_location.lcb, text_location.lcb];
                if lengths.iter().any(|length| *length != 0) {
                    if lengths.contains(&0) {
                        return Err("parallel footnote PLC is missing".to_owned());
                    }
                    let reference_bytes = bounded_slice(
                        table,
                        reference_location.fc,
                        reference_location.lcb,
                        "PlcffndRef",
                    )?;
                    let text_bytes =
                        bounded_slice(table, text_location.fc, text_location.lcb, "PlcffndTxt")?;
                    let references = NoteReferenceTable::from_bytes(reference_bytes)
                        .map_err(|error| format!("PlcffndRef: {error}"))?;
                    let text = CpOnlyTable::from_bytes(text_bytes)
                        .map_err(|error| format!("PlcffndTxt: {error}"))?;
                    if text.positions.len() != references.indices.len() + 2 {
                        return Err("footnote reference/text cardinality differs".to_owned());
                    }
                    if references.to_bytes().map_err(|error| error.to_string())? != reference_bytes
                        || text.to_bytes().map_err(|error| error.to_string())? != text_bytes
                    {
                        return Err("footnote PLC writer changed physical bytes".to_owned());
                    }
                    footnote_sets += 1;
                    footnote_references += references.indices.len();
                    footnote_custom_references += references
                        .indices
                        .iter()
                        .filter(|value| **value == 0)
                        .count();
                }
            }
            if let Some((reference_location, text_location)) = fib.endnote_locations() {
                let lengths = [reference_location.lcb, text_location.lcb];
                if lengths.iter().any(|length| *length != 0) {
                    if lengths.contains(&0) {
                        return Err("parallel endnote PLC is missing".to_owned());
                    }
                    let reference_bytes = bounded_slice(
                        table,
                        reference_location.fc,
                        reference_location.lcb,
                        "PlcfendRef",
                    )?;
                    let text_bytes =
                        bounded_slice(table, text_location.fc, text_location.lcb, "PlcfendTxt")?;
                    let references = NoteReferenceTable::from_bytes(reference_bytes)
                        .map_err(|error| format!("PlcfendRef: {error}"))?;
                    let text = CpOnlyTable::from_bytes(text_bytes)
                        .map_err(|error| format!("PlcfendTxt: {error}"))?;
                    if text.positions.len() != references.indices.len() + 2 {
                        return Err("endnote reference/text cardinality differs".to_owned());
                    }
                    if references.to_bytes().map_err(|error| error.to_string())? != reference_bytes
                        || text.to_bytes().map_err(|error| error.to_string())? != text_bytes
                    {
                        return Err("endnote PLC writer changed physical bytes".to_owned());
                    }
                    endnote_sets += 1;
                    endnote_references += references.indices.len();
                    endnote_custom_references += references
                        .indices
                        .iter()
                        .filter(|value| **value == 0)
                        .count();
                }
            }
            let mut annotation_metadata_references = None;
            if let Some((reference_location, text_location)) = fib.annotation_locations() {
                let lengths = [reference_location.lcb, text_location.lcb];
                if lengths.iter().any(|length| *length != 0) {
                    if lengths.contains(&0) {
                        return Err("parallel annotation PLC is missing".to_owned());
                    }
                    let reference_bytes = bounded_slice(
                        table,
                        reference_location.fc,
                        reference_location.lcb,
                        "PlcfandRef",
                    )?;
                    let text_bytes =
                        bounded_slice(table, text_location.fc, text_location.lcb, "PlcfandTxt")?;
                    let references = AnnotationReferenceTable::from_bytes(reference_bytes)
                        .map_err(|error| format!("PlcfandRef: {error}"))?;
                    let text = CpOnlyTable::from_bytes(text_bytes)
                        .map_err(|error| format!("PlcfandTxt: {error}"))?;
                    if text.positions.len() != references.annotations.len() + 2 {
                        return Err("annotation reference/text cardinality differs".to_owned());
                    }
                    if references.to_bytes().map_err(|error| error.to_string())? != reference_bytes
                        || text.to_bytes().map_err(|error| error.to_string())? != text_bytes
                    {
                        return Err("annotation PLC writer changed physical bytes".to_owned());
                    }
                    annotation_sets += 1;
                    annotation_references += references.annotations.len();
                    for annotation in &references.annotations {
                        annotation_initial_units += usize::from(annotation.initials_length);
                        annotation_empty_range_tags += usize::from(annotation.bookmark_tag == -1);
                        *annotation_unused_words
                            .entry((annotation.bits_not_used, annotation.flags_not_used))
                            .or_default() += 1;
                    }
                    annotation_metadata_references = Some(references);
                }
            }
            if let Some(location) = fib.annotation_extended_data_location()
                && location.lcb != 0
            {
                if atrd_extra_exclusions.contains_key(&path) {
                    observed_atrd_extra_exclusions.insert(path.clone());
                } else {
                    let physical = bounded_slice(table, location.fc, location.lcb, "AtrdExtra")?;
                    let extended = AnnotationExtendedData::from_bytes(physical)
                        .map_err(|error| format!("AtrdExtra: {error}"))?;
                    if extended.to_bytes().map_err(|error| error.to_string())? != physical {
                        return Err("AtrdExtra writer changed physical bytes".to_owned());
                    }
                    annotation_extended_tables += 1;
                    annotation_extended_records += extended.comments.len();
                    annotation_extended_count_mismatches +=
                        usize::from(annotation_metadata_references.as_ref().is_none_or(
                            |references| references.annotations.len() != extended.comments.len(),
                        ));
                    for comment in extended.comments {
                        *annotation_extended_depths.entry(comment.depth).or_default() += 1;
                        *annotation_extended_parent_offsets
                            .entry(comment.parent_offset)
                            .or_default() += 1;
                        annotation_extended_ink += usize::from(comment.ink);
                        annotation_extended_ows += usize::from(comment.ows_discussion_item);
                        annotation_extended_nonzero_padding1 += usize::from(comment.padding1 != 0);
                        annotation_extended_nonzero_padding2 += usize::from(comment.padding2 != 0);
                        annotation_extended_zero_dates += usize::from(
                            comment.modified.minute == 0
                                && comment.modified.hour == 0
                                && comment.modified.day == 0
                                && comment.modified.month == 0
                                && comment.modified.year_offset == 0
                                && comment.modified.weekday == 0,
                        );
                    }
                }
            }
            if let Some([method_location, guid_location]) = fib.user_input_method_locations() {
                let lengths = [method_location.lcb, guid_location.lcb];
                if lengths.iter().any(|length| *length != 0) {
                    if lengths.contains(&0) {
                        return Err("parallel Plcfuim/PlfguidUim table is missing".to_owned());
                    }
                    let method_physical =
                        bounded_slice(table, method_location.fc, method_location.lcb, "Plcfuim")?;
                    let guid_physical =
                        bounded_slice(table, guid_location.fc, guid_location.lcb, "PlfguidUim")?;
                    let methods = UserInputMethods::from_bytes(method_physical, guid_physical)
                        .map_err(|error| format!("Plcfuim/PlfguidUim: {error}"))?;
                    let (written_methods, written_guids) =
                        methods.to_bytes().map_err(|error| error.to_string())?;
                    if written_methods != method_physical || written_guids != guid_physical {
                        return Err("Plcfuim/PlfguidUim writer changed physical bytes".to_owned());
                    }
                    user_input_method_tables += 1;
                    user_input_methods += methods.methods.len();
                    user_input_method_guids += methods.service_guids.len();
                    user_input_method_guid_values.extend(methods.service_guids.iter().copied());
                    user_input_method_duplicate_positions += methods
                        .positions
                        .windows(2)
                        .filter(|positions| positions[0] == positions[1])
                        .count();
                    user_input_method_descending_positions += methods
                        .positions
                        .windows(2)
                        .filter(|positions| positions[0] > positions[1])
                        .count();
                    for method in methods.methods {
                        let service_data = method
                            .service_data(table)
                            .map_err(|error| format!("UIM service data: {error}"))?;
                        user_input_method_service_bytes += service_data.len();
                        user_input_method_empty_service_data +=
                            usize::from(service_data.is_empty());
                        user_input_method_negative_character_counts +=
                            usize::from(method.character_count < 0);
                        user_input_method_nonzero_private_data +=
                            usize::from(method.private_data != 0);
                        *user_input_method_reference_pairs
                            .entry((method.service_category_index, method.service_clsid_index))
                            .or_default() += 1;
                        *user_input_method_character_counts
                            .entry(method.character_count)
                            .or_default() += 1;
                        *user_input_method_service_sizes
                            .entry(method.service_data_size)
                            .or_default() += 1;
                        *user_input_method_private_values
                            .entry(method.private_data)
                            .or_default() += 1;
                    }
                }
            }
            let mut parsed_annotation_owners = None;
            if let Some(location) = fib.annotation_owner_location()
                && location.lcb != 0
            {
                let bytes = bounded_slice(table, location.fc, location.lcb, "GrpXstAtnOwners")?;
                let owners = AnnotationOwners::from_bytes(bytes)
                    .map_err(|error| format!("GrpXstAtnOwners: {error}"))?;
                if owners.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err("GrpXstAtnOwners writer changed physical bytes".to_owned());
                }
                annotation_owner_sets += 1;
                annotation_owners += owners.names.len();
                annotation_owner_name_units += owners.names.iter().map(Vec::len).sum::<usize>();
                parsed_annotation_owners = Some(owners);
            }
            if let Some(references) = &annotation_metadata_references {
                let owners = parsed_annotation_owners
                    .as_ref()
                    .ok_or_else(|| "annotations are missing GrpXstAtnOwners".to_owned())?;
                for annotation in &references.annotations {
                    let author_index = usize::try_from(annotation.author_index).map_err(|_| {
                        format!(
                            "negative ATRDPre10 author index {}",
                            annotation.author_index
                        )
                    })?;
                    if author_index >= owners.names.len() {
                        return Err(format!(
                            "ATRDPre10 author index {author_index} is outside {} owners",
                            owners.names.len()
                        ));
                    }
                }
            }
            if let Some((info_location, start_location, end_location)) =
                fib.annotation_bookmark_locations()
            {
                let lengths = [info_location.lcb, start_location.lcb, end_location.lcb];
                if annotation_metadata_references.is_some()
                    && lengths.iter().any(|length| *length != 0)
                {
                    if start_location.lcb == 0 || end_location.lcb == 0 {
                        return Err(format!(
                            "parallel annotation bookmark table is missing: {lengths:?}"
                        ));
                    }
                    let info_bytes =
                        bounded_slice(table, info_location.fc, info_location.lcb, "SttbfAtnBkmk")?;
                    let start_bytes =
                        bounded_slice(table, start_location.fc, start_location.lcb, "PlcfAtnBkf")?;
                    let end_bytes =
                        bounded_slice(table, end_location.fc, end_location.lcb, "PlcfAtnBkl")?;
                    let bookmarks =
                        AnnotationBookmarks::from_bytes(info_bytes, start_bytes, end_bytes)
                            .map_err(|error| format!("annotation bookmarks: {error}"))?;
                    let written = bookmarks.to_bytes().map_err(|error| error.to_string())?;
                    if written.0 != info_bytes || written.1 != start_bytes || written.2 != end_bytes
                    {
                        return Err("annotation bookmark writer changed physical bytes".to_owned());
                    }
                    let references = annotation_metadata_references.as_ref().ok_or_else(|| {
                        "annotation bookmarks are missing annotation references".to_owned()
                    })?;
                    let mut tags = bookmarks
                        .infos
                        .entries
                        .iter()
                        .map(|entry| entry.tag)
                        .collect::<BTreeSet<_>>();
                    for annotation in &references.annotations {
                        if annotation.bookmark_tag != -1 && !tags.remove(&annotation.bookmark_tag) {
                            return Err(format!(
                                "ATRDPre10 bookmark tag {} has no unique ATNBE",
                                annotation.bookmark_tag
                            ));
                        }
                    }
                    if !tags.is_empty() {
                        return Err(format!("ATNBE tags have no ATRDPre10: {tags:?}"));
                    }
                    annotation_bookmark_sets += 1;
                    annotation_bookmarks += bookmarks.infos.entries.len();
                }
            }
            let mut parsed_textbox_stories = BTreeMap::new();
            for (part, location) in fib.textbox_story_locations() {
                let character_count = match part {
                    TextboxDocumentPart::Main => fib.rg_lw.ccp_textbox,
                    TextboxDocumentPart::Header => fib.rg_lw.ccp_header_textbox,
                };
                if character_count <= 0 {
                    continue;
                }
                if location.lcb == 0 {
                    return Err(format!("{part:?} textbox story table is missing"));
                }
                let bytes = bounded_slice(table, location.fc, location.lcb, "PlcftxbxTxt")?;
                let stories = TextboxStoryTable::from_bytes(bytes)
                    .map_err(|error| format!("{part:?} PlcftxbxTxt: {error}"))?;
                if stories.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err(format!(
                        "{part:?} textbox story writer changed physical bytes"
                    ));
                }
                *textbox_story_sets.entry(part).or_default() += 1;
                textbox_stories += stories.stories.len();
                reusable_textbox_stories += stories
                    .stories
                    .iter()
                    .filter(|story| matches!(story.chain, TextboxStoryChain::Reusable { .. }))
                    .count();
                parsed_textbox_stories.insert(part, stories);
            }
            for (part, location) in fib.textbox_break_locations() {
                let Some(stories) = parsed_textbox_stories.get(&part) else {
                    continue;
                };
                if location.lcb == 0 {
                    return Err(format!("{part:?} textbox break table is missing"));
                }
                let bytes = bounded_slice(table, location.fc, location.lcb, "PlcfTxbxBkd")?;
                let breaks = TextboxBreakTable::from_bytes(bytes)
                    .map_err(|error| format!("{part:?} PlcfTxbxBkd: {error}"))?;
                if breaks.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err(format!(
                        "{part:?} textbox break writer changed physical bytes"
                    ));
                }
                for record in breaks
                    .breaks
                    .iter()
                    .take(breaks.breaks.len().saturating_sub(1))
                {
                    let story_index = usize::try_from(record.story_index).map_err(|_| {
                        format!("{part:?} Tbkd has negative nonterminal story index")
                    })?;
                    if story_index >= stories.stories.len() {
                        return Err(format!(
                            "{part:?} Tbkd story index {story_index} is outside {} stories",
                            stories.stories.len()
                        ));
                    }
                }
                *textbox_break_sets.entry(part).or_default() += 1;
                textbox_breaks += breaks.breaks.len();
                textbox_overflows += breaks
                    .breaks
                    .iter()
                    .filter(|record| record.text_overflow)
                    .count();
            }
            let mut parsed_shape_anchor_ids = BTreeMap::new();
            let mut parsed_shape_anchor_counts = BTreeMap::new();
            for (part, location) in fib.shape_anchor_locations() {
                if location.lcb == 0 {
                    continue;
                }
                let bytes = bounded_slice(table, location.fc, location.lcb, "PlcfSpa")?;
                let anchors = ShapeAnchorTable::from_bytes(bytes)
                    .map_err(|error| format!("{part:?} PlcfSpa: {error}"))?;
                if anchors.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err(format!("{part:?} PlcfSpa writer changed physical bytes"));
                }
                if let Some(stories) = parsed_textbox_stories.get(&part) {
                    let anchor_ids = anchors
                        .anchors
                        .iter()
                        .map(|anchor| anchor.shape_id)
                        .collect::<BTreeSet<_>>();
                    for story in &stories.stories {
                        if matches!(story.chain, TextboxStoryChain::NonReusable { .. })
                            && !anchor_ids.contains(&story.shape_id)
                        {
                            textbox_stories_without_anchor += 1;
                        }
                    }
                }
                *shape_anchor_sets.entry(part).or_default() += 1;
                shape_anchors += anchors.anchors.len();
                below_text_shapes += anchors
                    .anchors
                    .iter()
                    .filter(|anchor| anchor.below_text)
                    .count();
                locked_shape_anchors += anchors
                    .anchors
                    .iter()
                    .filter(|anchor| anchor.anchor_locked)
                    .count();
                parsed_shape_anchor_ids.insert(
                    part,
                    anchors
                        .anchors
                        .iter()
                        .map(|anchor| anchor.shape_id)
                        .collect::<BTreeSet<_>>(),
                );
                parsed_shape_anchor_counts.insert(part, anchors.anchors.len());
            }
            if let Some(location) = fib.office_art_content_location()
                && location.lcb != 0
            {
                let bytes = bounded_slice(table, location.fc, location.lcb, "OfficeArtContent")?;
                let content = DocOfficeArtContent::from_bytes(bytes).map_err(|error| {
                    format!(
                        "OfficeArtContent fc={:#x} lcb={:#x} prefix={:02x?}: {error}",
                        location.fc,
                        location.lcb,
                        &bytes[..bytes.len().min(16)]
                    )
                })?;
                if content.to_bytes().map_err(|error| error.to_string())? != bytes {
                    return Err("OfficeArtContent writer changed physical bytes".to_owned());
                }
                office_art_graph_audit.audit(&content);
                let mut fsp_ids = BTreeMap::<TextboxDocumentPart, BTreeSet<u32>>::new();
                office_art_partial_trees += usize::from(content.drawing_group.is_partial());
                content.drawing_group.visit_complete(|record| {
                    office_art_records += 1;
                    match &record.data {
                        OfficeArtRecordData::Atom(bytes) => {
                            office_art_atom_bytes += bytes.len();
                            *office_art_atom_shapes
                                .entry((record.header.record_type, bytes.len()))
                                .or_default() += 1;
                        }
                        OfficeArtRecordData::WordClientAnchor(_) => word_client_anchors += 1,
                        OfficeArtRecordData::WordClientData(_) => word_client_data += 1,
                        OfficeArtRecordData::WordClientTextbox(_) => word_client_textboxes += 1,
                        _ => {}
                    }
                });
                for drawing in &content.drawings {
                    *office_art_drawings
                        .entry(drawing.document_part)
                        .or_default() += 1;
                    office_art_partial_trees += usize::from(drawing.container.is_partial());
                    drawing.container.visit_complete(|record| {
                        office_art_records += 1;
                        match &record.data {
                            OfficeArtRecordData::Shape(shape) => {
                                fsp_ids
                                    .entry(drawing.document_part)
                                    .or_default()
                                    .insert(shape.shape_id);
                            }
                            OfficeArtRecordData::Atom(bytes) => {
                                office_art_atom_bytes += bytes.len();
                                *office_art_atom_shapes
                                    .entry((record.header.record_type, bytes.len()))
                                    .or_default() += 1;
                            }
                            OfficeArtRecordData::WordClientAnchor(index) => {
                                word_client_anchors += 1;
                                let valid = *index == -1
                                    || usize::try_from(*index).is_ok_and(|index| {
                                        parsed_shape_anchor_counts
                                            .get(&drawing.document_part)
                                            .is_some_and(|count| index < *count)
                                    });
                                word_client_anchor_invalid_indexes += usize::from(!valid);
                            }
                            OfficeArtRecordData::WordClientData(_) => word_client_data += 1,
                            OfficeArtRecordData::WordClientTextbox(value) => {
                                word_client_textboxes += 1;
                                let valid = value.story_index != 0
                                    && parsed_textbox_stories
                                        .get(&drawing.document_part)
                                        .is_some_and(|stories| {
                                            usize::from(value.story_index) <= stories.stories.len()
                                        });
                                word_client_textbox_invalid_indexes += usize::from(!valid);
                            }
                            _ => {}
                        }
                    });
                }
                for (part, ids) in &parsed_shape_anchor_ids {
                    let drawing_ids = fsp_ids.get(part);
                    shape_anchors_without_fsp += ids
                        .iter()
                        .filter(|shape_id| drawing_ids.is_none_or(|ids| !ids.contains(shape_id)))
                        .count();
                }
                for (part, stories) in &parsed_textbox_stories {
                    let drawing_ids = fsp_ids.get(part);
                    textbox_stories_without_fsp += stories
                        .stories
                        .iter()
                        .filter(|story| {
                            matches!(story.chain, TextboxStoryChain::NonReusable { .. })
                                && drawing_ids.is_none_or(|ids| !ids.contains(&story.shape_id))
                        })
                        .count();
                }
                office_art_contents += 1;
            }
            let style_location = fib
                .style_sheet_location()
                .ok_or_else(|| "FIB does not contain STSH location".to_owned())?;
            let style_bytes = bounded_slice(table, style_location.fc, style_location.lcb, "STSH")?;
            let style_sheet =
                StyleSheet::from_bytes(style_bytes).map_err(|error| format!("STSH: {error}"))?;
            if style_sheet.to_bytes().map_err(|error| error.to_string())? != style_bytes {
                return Err("STSH write did not reproduce its physical bytes".to_owned());
            }
            style_sheets += 1;
            let info_length = usize::from(u16::from_le_bytes(
                style_bytes[..2]
                    .try_into()
                    .expect("STSH parser checked LPStshi"),
            ));
            *style_sheet_info_shapes
                .entry((info_length, style_sheet.info.header.std_base_size))
                .or_default() += 1;
            if let Some(latent) = &style_sheet.info.latent_styles {
                latent_style_entries += latent.entries.len();
            }
            standard_style_prls += style_sheet
                .info
                .standard_character_properties
                .as_ref()
                .map_or(0, |properties| properties.properties.len());
            standard_style_prls += style_sheet
                .info
                .standard_paragraph_properties
                .as_ref()
                .map_or(0, |properties| properties.properties.len());
            styles += style_sheet.styles.len();
            for (style_index, style) in style_sheet.styles.iter().enumerate() {
                let Some(definition) = &style.definition else {
                    empty_styles += 1;
                    continue;
                };
                style_definition_bytes += usize::from(definition.base.byte_count);
                style_name_units += definition.name.characters.len();
                let mut groups = Vec::new();
                let mut papx = Vec::new();
                match &definition.formatting {
                    StyleFormatting::Paragraph {
                        paragraph,
                        character,
                    } => {
                        papx.push(paragraph);
                        groups.extend([&paragraph.properties, &character.properties]);
                        if let Some(value) = paragraph.padding {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                        if let Some(value) = character.padding {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                    }
                    StyleFormatting::Character { character } => {
                        groups.push(&character.properties);
                        if let Some(value) = character.padding {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                    }
                    StyleFormatting::RevisionParagraph {
                        paragraph,
                        character,
                        original_paragraph,
                        original_character,
                        ..
                    } => {
                        revision_marked_styles += 1;
                        papx.extend([paragraph, original_paragraph]);
                        groups.extend([
                            &paragraph.properties,
                            &character.properties,
                            &original_paragraph.properties,
                            &original_character.properties,
                        ]);
                        for value in [
                            paragraph.padding,
                            character.padding,
                            original_paragraph.padding,
                            original_character.padding,
                        ]
                        .into_iter()
                        .flatten()
                        {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                    }
                    StyleFormatting::RevisionCharacter {
                        character,
                        original_character,
                        ..
                    } => {
                        revision_marked_styles += 1;
                        groups.extend([&character.properties, &original_character.properties]);
                        for value in [character.padding, original_character.padding]
                            .into_iter()
                            .flatten()
                        {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                    }
                    StyleFormatting::Table {
                        table,
                        paragraph,
                        character,
                    } => {
                        papx.push(paragraph);
                        groups.extend([
                            &table.properties,
                            &paragraph.properties,
                            &character.properties,
                        ]);
                        for value in [table.padding, paragraph.padding, character.padding]
                            .into_iter()
                            .flatten()
                        {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                    }
                    StyleFormatting::Numbering { paragraph } => {
                        papx.push(paragraph);
                        groups.push(&paragraph.properties);
                        if let Some(value) = paragraph.padding {
                            *style_upx_padding.entry(value).or_default() += 1;
                        }
                    }
                }
                for paragraph in papx {
                    let expected = u16::try_from(style_index).expect("cstd fits u16");
                    if paragraph.style_index != expected {
                        *style_upx_index_mismatches
                            .entry((expected, paragraph.style_index))
                            .or_default() += 1;
                    }
                }
                while let Some(group) = groups.pop() {
                    for property in &group.properties {
                        style_upx_prls += 1;
                        if let Some(shape) = static_variable_shape(&property.operand) {
                            *style_upx_static_variable_operands.entry(shape).or_default() += 1;
                        }
                        match &property.operand {
                            SprmOperand::ConditionalFormatting(value) => {
                                groups.push(&value.properties);
                            }
                            SprmOperand::CharacterMajority(value) => groups.push(value),
                            _ => {}
                        }
                        if matches!(property.sprm.kind(), SprmKind::Other(_)) {
                            style_upx_unknown_sprms.insert(
                                property
                                    .sprm
                                    .opcode()
                                    .expect("parsed SPRM opcode remains encodable"),
                            );
                        }
                        if matches!(
                            property.operand,
                            SprmOperand::Variable8(_) | SprmOperand::Variable16PlusOne(_)
                        ) {
                            style_upx_raw_variable_operands += 1;
                            let length = match &property.operand {
                                SprmOperand::Variable8(value)
                                | SprmOperand::Variable16PlusOne(value) => value.len(),
                                _ => unreachable!(),
                            };
                            let opcode = property
                                .sprm
                                .opcode()
                                .expect("parsed SPRM opcode remains encodable");
                            *style_upx_raw_variable_frequencies
                                .entry(opcode)
                                .or_default() += 1;
                            *style_upx_raw_variable_shapes
                                .entry((opcode, length))
                                .or_default() += 1;
                        }
                    }
                }
                *style_kind_counts
                    .entry(definition.base.style_kind)
                    .or_default() += 1;
                *style_cupx_shapes
                    .entry((
                        definition.base.style_kind,
                        definition.base.formatting_count,
                        definition
                            .post_2000
                            .is_some_and(|post| post.has_original_style),
                    ))
                    .or_default() += 1;
                if let Some(value) = style.alignment_padding {
                    *style_alignment_padding.entry(value).or_default() += 1;
                }
            }
            let section_location = fib
                .section_table_location()
                .ok_or_else(|| "FIB does not contain PlcfSed location".to_owned())?;
            let section_bytes =
                bounded_slice(table, section_location.fc, section_location.lcb, "PlcfSed")?;
            let section_table =
                PlcfSed::from_bytes(section_bytes).map_err(|error| error.to_string())?;
            if section_table
                .to_bytes()
                .map_err(|error| error.to_string())?
                != section_bytes
            {
                return Err("PlcfSed write did not reproduce its physical bytes".to_owned());
            }
            section_tables += 1;
            sections += section_table.sections.len();
            for section in &section_table.sections {
                let Some(sepx) = Sepx::from_word_document(&word_document.data, section.sepx_offset)
                    .map_err(|error| format!("Sepx at {:#x}: {error}", section.sepx_offset))?
                else {
                    default_sections += 1;
                    continue;
                };
                let physical = sepx.to_bytes().map_err(|error| error.to_string())?;
                let start = usize::try_from(section.sepx_offset)
                    .map_err(|_| "negative Sepx offset".to_owned())?;
                if word_document.data.get(start..start + physical.len())
                    != Some(physical.as_slice())
                {
                    return Err("Sepx write did not reproduce its physical bytes".to_owned());
                }
                sepx_count += 1;
                if let Some(value) = sepx.trailing_byte {
                    *sepx_trailing_bytes.entry(value).or_default() += 1;
                }
                for property in &sepx.properties.properties {
                    sepx_prls += 1;
                    if let Some(shape) = static_variable_shape(&property.operand) {
                        *sepx_static_variable_operands.entry(shape).or_default() += 1;
                    }
                    if let SprmOperand::OutlineListData(value) = &property.operand {
                        *outline_list_restart_values
                            .entry(value.restart_heading)
                            .or_default() += 1;
                        outline_list_reserved_shapes.insert(value.reserved);
                        outline_list_nonzero_text_units +=
                            value.display_text.iter().filter(|unit| **unit != 0).count();
                    }
                    if let SprmOperand::SectionHeaderFooterFlags(value) = &property.operand {
                        *section_header_footer_flag_shapes
                            .entry(value.bits().map_err(|error| error.to_string())?)
                            .or_default() += 1;
                    }
                    if let SprmKind::Other(opcode) = property.sprm.kind() {
                        sepx_unknown_sprms.insert(opcode);
                        let value = match &property.operand {
                            SprmOperand::Byte(value) => u32::from(*value),
                            SprmOperand::Word(value) => u32::from(u16::from_le_bytes(*value)),
                            _ => u32::MAX,
                        };
                        *sepx_unknown_fixed_shapes
                            .entry((opcode, value))
                            .or_default() += 1;
                    }
                    if matches!(
                        property.operand,
                        SprmOperand::Variable8(_) | SprmOperand::Variable16PlusOne(_)
                    ) {
                        sepx_raw_variable_operands += 1;
                        *sepx_raw_variable_frequencies
                            .entry(property.sprm.opcode().unwrap())
                            .or_default() += 1;
                        let length = match &property.operand {
                            SprmOperand::Variable8(value)
                            | SprmOperand::Variable16PlusOne(value) => value.len(),
                            _ => unreachable!(),
                        };
                        *sepx_raw_variable_shapes
                            .entry((property.sprm.opcode().unwrap(), length))
                            .or_default() += 1;
                    }
                }
            }
            let chpx_location = fib
                .chpx_bte_location()
                .ok_or_else(|| "FIB does not contain PlcBteChpx location".to_owned())?;
            let chpx_bte_bytes =
                bounded_slice(table, chpx_location.fc, chpx_location.lcb, "PlcBteChpx")?;
            let chpx_bte = PlcBte::from_bytes(chpx_bte_bytes).map_err(|error| error.to_string())?;
            if chpx_bte.to_bytes().map_err(|error| error.to_string())? != chpx_bte_bytes {
                return Err("PlcBteChpx write did not reproduce its physical bytes".to_owned());
            }
            chpx_bte_count += 1;
            for page in &chpx_bte.pages {
                let start = page.byte_offset().map_err(|error| error.to_string())?;
                let physical = word_document
                    .data
                    .get(start..start + 512)
                    .ok_or_else(|| "ChpxFkp page exceeds WordDocument".to_owned())?;
                let fkp = ChpxFkp::from_bytes(physical).map_err(|error| error.to_string())?;
                if fkp.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("ChpxFkp write did not reproduce its physical page".to_owned());
                }
                chpx_pages += 1;
                chpx_runs += fkp.runs.len();
                chpx_default_runs += fkp
                    .runs
                    .iter()
                    .filter(|run| run.properties.is_none())
                    .count();
                chpx_unused_bytes += fkp
                    .unused_regions
                    .iter()
                    .map(|region| region.bytes.len())
                    .sum::<usize>();
                for run in &fkp.runs {
                    let Some(properties) = &run.properties else {
                        continue;
                    };
                    for property in &properties.properties {
                        chpx_prls += 1;
                        *chpx_sprm_frequencies
                            .entry(property.sprm.opcode().map_err(|error| error.to_string())?)
                            .or_default() += 1;
                        if let SprmKind::Other(opcode) = property.sprm.kind() {
                            chpx_unknown_sprms.insert(opcode);
                        }
                        if matches!(
                            property.operand,
                            SprmOperand::Variable8(_) | SprmOperand::Variable16PlusOne(_)
                        ) {
                            chpx_raw_variable_operands += 1;
                            *chpx_raw_variable_frequencies
                                .entry(property.sprm.opcode().unwrap())
                                .or_default() += 1;
                        }
                        if let Some(shape) = static_variable_shape(&property.operand) {
                            *chpx_static_variable_operands.entry(shape).or_default() += 1;
                        }
                    }
                }
            }
            let papx_location = fib
                .papx_bte_location()
                .ok_or_else(|| "FIB does not contain PlcBtePapx location".to_owned())?;
            let papx_bte_bytes =
                bounded_slice(table, papx_location.fc, papx_location.lcb, "PlcBtePapx")?;
            let papx_bte = PlcBte::from_bytes(papx_bte_bytes).map_err(|error| error.to_string())?;
            if papx_bte.to_bytes().map_err(|error| error.to_string())? != papx_bte_bytes {
                return Err("PlcBtePapx write did not reproduce its physical bytes".to_owned());
            }
            papx_bte_count += 1;
            for page in &papx_bte.pages {
                let start = page.byte_offset().map_err(|error| error.to_string())?;
                let physical = word_document
                    .data
                    .get(start..start + 512)
                    .ok_or_else(|| "PapxFkp page exceeds WordDocument".to_owned())?;
                let fkp = PapxFkp::from_bytes(physical).map_err(|error| error.to_string())?;
                if fkp.to_bytes().map_err(|error| error.to_string())? != physical {
                    return Err("PapxFkp write did not reproduce its physical page".to_owned());
                }
                papx_pages += 1;
                papx_runs += fkp.runs.len();
                papx_default_runs += fkp
                    .runs
                    .iter()
                    .filter(|run| run.properties.is_none())
                    .count();
                papx_unused_bytes += fkp
                    .unused_regions
                    .iter()
                    .map(|region| region.bytes.len())
                    .sum::<usize>();
                for run in &fkp.runs {
                    let Some(properties) = &run.properties else {
                        continue;
                    };
                    match properties.length_encoding {
                        PapxLengthEncoding::HalfWordsMinusOne => papx_short_lengths += 1,
                        PapxLengthEncoding::ExtendedHalfWords => papx_extended_lengths += 1,
                    }
                    if let Some(value) = properties.trailing_byte {
                        *papx_trailing_bytes.entry(value).or_default() += 1;
                    }
                    for property in &properties.properties.properties {
                        papx_prls += 1;
                        *papx_sprm_frequencies
                            .entry(property.sprm.opcode().map_err(|error| error.to_string())?)
                            .or_default() += 1;
                        if let SprmKind::Other(opcode) = property.sprm.kind() {
                            papx_unknown_sprms.insert(opcode);
                        }
                        if matches!(
                            property.operand,
                            SprmOperand::Variable8(_) | SprmOperand::Variable16PlusOne(_)
                        ) {
                            papx_raw_variable_operands += 1;
                            *papx_raw_variable_frequencies
                                .entry(property.sprm.opcode().unwrap())
                                .or_default() += 1;
                        }
                        if let Some(shape) = static_variable_shape(&property.operand) {
                            *papx_static_variable_operands.entry(shape).or_default() += 1;
                        }
                    }
                }
            }
            let location = fib
                .clx_location()
                .ok_or_else(|| "FIB does not contain fcClx/lcbClx".to_owned())?;
            let start =
                usize::try_from(location.fc).map_err(|_| "fcClx exceeds usize".to_owned())?;
            let length =
                usize::try_from(location.lcb).map_err(|_| "lcbClx exceeds usize".to_owned())?;
            let end = start
                .checked_add(length)
                .ok_or_else(|| "CLX bounds overflow".to_owned())?;
            let physical = table
                .get(start..end)
                .ok_or_else(|| "CLX extends beyond the selected Table stream".to_owned())?;
            let clx = Clx::from_bytes(physical).map_err(|error| error.to_string())?;
            if clx.to_bytes().map_err(|error| error.to_string())? != physical {
                return Err("CLX write did not reproduce its physical bytes".to_owned());
            }
            clx_count += 1;
            property_runs += clx.property_runs.len();
            for run in &clx.property_runs {
                for property in &run.properties.properties {
                    prl_count += 1;
                    *sprm_opcodes
                        .entry(property.sprm.opcode().unwrap())
                        .or_default() += 1;
                    if let SprmKind::Other(opcode) = property.sprm.kind() {
                        unknown_sprm_kinds.insert(opcode);
                    }
                    let group = match property.sprm.group {
                        SprmGroup::Paragraph => 1,
                        SprmGroup::Character => 2,
                        SprmGroup::Picture => 3,
                        SprmGroup::Section => 4,
                        SprmGroup::Table => 5,
                        SprmGroup::Compatibility(value) => value,
                    };
                    *sprm_groups.entry(group).or_default() += 1;
                    let (shape, bytes) = match &property.operand {
                        SprmOperand::Toggle(_) => ("toggle", 0),
                        SprmOperand::Byte(_) => ("byte", 0),
                        SprmOperand::Word(_) => ("word", 0),
                        SprmOperand::Dword(_) => ("dword", 0),
                        SprmOperand::Word4(_) => ("word4", 0),
                        SprmOperand::Word5(_) => ("word5", 0),
                        SprmOperand::Variable8(value) => ("variable8", value.len()),
                        SprmOperand::ParagraphChangeTabs(value) => (
                            "paragraph-change-tabs",
                            2 + value.deleted.len() * 4 + value.added.len() * 3,
                        ),
                        SprmOperand::ParagraphChangeTabsPapx(value) => (
                            "paragraph-change-tabs-papx",
                            2 + value.deleted_positions.len() * 2 + value.added.len() * 3,
                        ),
                        SprmOperand::Shading(_) => ("shading", 10),
                        SprmOperand::Border(_) => ("border", 8),
                        SprmOperand::PropertyRevisionMark(_) => ("property-revision-mark", 7),
                        SprmOperand::CharacterFitText(_) => ("character-fit-text", 8),
                        SprmOperand::TableCellSpacing(_) => ("table-cell-spacing", 6),
                        SprmOperand::TableBorderColors(value) => {
                            ("table-border-colors", value.len() * 4)
                        }
                        SprmOperand::TableShading80(value) => ("table-shading-80", value.len() * 2),
                        SprmOperand::TableShading(value) => ("table-shading", value.len() * 10),
                        SprmOperand::TableCellHideMark(_) => ("table-cell-hide-mark", 3),
                        SprmOperand::TableCellWidth(_) => ("table-cell-width", 5),
                        SprmOperand::ParagraphTableStyleInfo(_) => {
                            ("paragraph-table-style-info", 16)
                        }
                        SprmOperand::TableBorders(_) => ("table-borders", 48),
                        SprmOperand::TableBorders80(_) => ("table-borders-80", 24),
                        SprmOperand::TableBorder(_) => ("table-border", 11),
                        SprmOperand::TableBorder80(_) => ("table-border-80", 7),
                        SprmOperand::TableDefinition(value) => (
                            "table-definition",
                            1 + value.column_boundaries.len() * 2 + value.cells.len() * 20,
                        ),
                        SprmOperand::ParagraphNumberRevisionMark(_) => {
                            ("paragraph-number-revision", 128)
                        }
                        SprmOperand::CharacterMajority(value) => {
                            ("character-majority", value.to_bytes().unwrap().len())
                        }
                        SprmOperand::CharacterDisplayFieldRevisionMark(_) => {
                            ("character-display-field-revision", 39)
                        }
                        SprmOperand::StylePermutation(value) => (
                            "style-permutation",
                            5 + value.remapped_style_indices.len() * 2,
                        ),
                        SprmOperand::ConditionalFormatting(value) => (
                            "conditional-formatting",
                            2 + value.properties.to_bytes().unwrap().len(),
                        ),
                        SprmOperand::AutoNumberedListData(_) => ("auto-numbered-list-data", 84),
                        SprmOperand::OutlineListData(_) => ("outline-list-data", 212),
                        SprmOperand::SectionHeaderFooterFlags(_) => {
                            ("section-header-footer-flags", 0)
                        }
                        SprmOperand::Variable16PlusOne(value) => ("variable16+1", value.len()),
                        SprmOperand::ThreeBytes(_) => ("three-bytes", 0),
                    };
                    *sprm_operand_shapes.entry(shape).or_default() += 1;
                    variable_operand_bytes += bytes;
                }
            }
            pieces += clx.piece_table.pieces.len();
            for (index, piece) in clx.piece_table.pieces.iter().enumerate() {
                compressed_pieces += usize::from(piece.file_position.compressed);
                match piece.property_modifier {
                    Prm::Simple { .. } => simple_property_modifiers += 1,
                    Prm::Complex { property_run_index } => {
                        if usize::from(property_run_index) >= clx.property_runs.len() {
                            return Err(format!(
                                "Prm1 references PRC {property_run_index}, but CLX has {} entries",
                                clx.property_runs.len()
                            ));
                        }
                        complex_property_modifiers += 1;
                    }
                }
                let text_piece = piece
                    .text_piece(
                        &word_document.data,
                        clx.piece_table.character_positions[index],
                        clx.piece_table.character_positions[index + 1],
                    )
                    .map_err(|error| error.to_string())?;
                let physical = text_piece.to_bytes();
                let start = usize::try_from(text_piece.file_offset)
                    .map_err(|_| "text-piece offset exceeds usize".to_owned())?;
                if word_document.data.get(start..start + physical.len()) != Some(&physical) {
                    return Err("text piece write did not reproduce its physical bytes".to_owned());
                }
                text_characters += text_piece.character_count();
                match text_piece.characters {
                    TextPieceCharacters::Compressed(value) => {
                        compressed_text_bytes += value.len();
                    }
                    TextPieceCharacters::Utf16(value) => utf16_text_units += value.len(),
                }
            }
            checked += 1;
            Ok(true)
        })();

        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }

    assert!(
        failures.is_empty(),
        "DOC FIB round-trip failures:\n{}",
        failures.join("\n")
    );
    assert_eq!(selection_states, 306);
    assert_eq!(selection_state_shapes, BTreeMap::from([(36, 303), (44, 3)]));
    assert_eq!(selection_ranges, BTreeMap::from([("unused", 306)]));
    assert_eq!(
        selection_styles,
        BTreeMap::from([
            (SelectionStyle::Undefined, 284),
            (SelectionStyle::Character, 14),
            (SelectionStyle::Word, 4),
            (SelectionStyle::Line, 3),
            (SelectionStyle::Row, 1),
        ])
    );
    assert_eq!(
        selection_extensions,
        BTreeMap::from([
            ([0, 0], 1),
            ([0x0003_0003, 0x032c_043b], 1),
            ([0x0003_0003, 0x0358_0367], 1),
        ])
    );
    assert_eq!(typed_command_customizations, 353);
    assert_eq!(toolbar_control_records, 2);
    assert_eq!(toolbar_control_bytes, 262);
    assert_eq!(custom_toolbar_records, 0);
    assert_eq!(custom_toolbar_controls, 0);
    assert_eq!(
        toolbar_control_shapes,
        BTreeMap::from([((0x0a, 0x0001, 0x00, 0x05), 2)])
    );
    assert_eq!(
        command_customization_records,
        BTreeMap::from([
            ("command-strings", 3),
            ("macro-commands", 3),
            ("macro-names", 3),
            ("toolbar", 1),
        ])
    );
    assert_eq!(pending_command_customization_shapes, BTreeMap::new());
    assert_eq!(
        command_customization_shapes,
        BTreeMap::from([
            ((2, 0xff, 0x40), 349),
            ((151, 0xff, 0x01), 1),
            ((187, 0xff, 0x01), 1),
            ((324, 0xff, 0x12), 1),
            ((471, 0xff, 0x01), 1),
        ])
    );
    assert_eq!(observed_exclusions.len(), exclusions.len());
    assert_eq!(
        observed_atrd_extra_exclusions.len(),
        atrd_extra_exclusions.len()
    );
    assert_eq!(
        observed_plcf_wkb_exclusions.len(),
        plcf_wkb_exclusions.len()
    );
    assert_eq!(observed_plcf_wkb_exclusions.len(), 1);
    assert_eq!(encrypted_exclusions, 3);
    assert_eq!(invalid_exclusions, 36);
    assert_eq!(checked, 403);
    assert_eq!(zero_last_saved_file_times, 70);
    assert_eq!(nonzero_last_saved_file_times, 333);
    assert_eq!(distinct_last_saved_file_times.len(), 329);
    assert_eq!(minimum_last_saved_file_time, 0x0000_0000_c4ee_20b1);
    assert_eq!(maximum_last_saved_file_time, 0x01dc_f8fb_bd7b_33c0);
    assert_eq!(last_saved_high_zero_nonzero, 2);
    assert_eq!(last_saved_file_time_part_mismatches, 0);
    assert_eq!(
        nonzero_fib_pairs,
        BTreeMap::from([
            (0, 399),
            (1, 403),
            (2, 14),
            (3, 14),
            (4, 13),
            (5, 13),
            (6, 403),
            (8, 3),
            (11, 209),
            (12, 403),
            (13, 403),
            (15, 403),
            (16, 94),
            (17, 73),
            (18, 2),
            (19, 1),
            (21, 77),
            (22, 77),
            (23, 77),
            (24, 353),
            (27, 8),
            (28, 8),
            (29, 8),
            (30, 306),
            (31, 403),
            (32, 351),
            (33, 403),
            (36, 13),
            (37, 10),
            (40, 90),
            (41, 28),
            (42, 10),
            (43, 10),
            (44, 1),
            (46, 7),
            (47, 7),
            (48, 1),
            (50, 301),
            (51, 319),
            (53, 1),
            (54, 1),
            (55, 313),
            (56, 49),
            (57, 11),
            (58, 14),
            (59, 6),
            (60, 5),
            (61, 1),
            (62, 1),
            (63, 8),
            (64, 8),
            (71, 56),
            (73, 164),
            (74, 165),
            (75, 49),
            (76, 14),
            (84, 5),
            (85, 8),
            (86, 114),
            (87, 331),
            (88, 27),
            (90, 296),
            (91, 164),
            (93, 345),
            (94, 313),
            (96, 132),
            (97, 1),
            (98, 218),
            (99, 113),
            (100, 6),
            (102, 3),
            (103, 3),
            (109, 31),
            (110, 2),
            (111, 2),
            (112, 14),
            (113, 284),
            (114, 16),
            (115, 16),
            (116, 1),
            (117, 16),
            (118, 16),
            (129, 16),
            (130, 9),
            (132, 100),
            (146, 3),
            (147, 4),
            (148, 50),
            (149, 12),
            (154, 3),
            (155, 3),
            (156, 3),
            (181, 219),
            (182, 219),
        ])
    );
    assert_eq!(font_tables, 403);
    assert_eq!(fonts, 2_817);
    assert_eq!(alternate_font_names, 290);
    assert_eq!(font_name_units, 33_001);
    assert_eq!(padded_font_names, 2);
    assert_eq!(font_name_padding_units, 5);
    assert_eq!(embedded_font_tables, 1);
    assert_eq!(embedded_font_references, 0);
    assert_eq!(
        embedded_font_table_offsets,
        BTreeMap::from([(EmbeddedFontTableOffset::Word97Compatibility, 1)])
    );
    assert_eq!(embedded_font_table_shapes, BTreeMap::from([((0, 10), 1)]));
    assert!(embedded_font_subsets.is_empty());
    assert_eq!(embedded_font_nonzero_ignored_flags, 0);
    assert_eq!(associated_string_tables, 351);
    assert_eq!(associated_string_units, 13_593);
    assert_eq!(
        nonempty_associated_strings,
        BTreeMap::from([
            (1, 26),
            (2, 156),
            (3, 22),
            (4, 15),
            (5, 1),
            (6, 306),
            (7, 328)
        ])
    );
    assert_eq!(
        maximum_associated_string_lengths,
        BTreeMap::from([
            (0, 0),
            (1, 129),
            (2, 144),
            (3, 255),
            (4, 250),
            (5, 91),
            (6, 57),
            (7, 28),
            (8, 0),
            (9, 0),
            (10, 0),
            (11, 0),
            (12, 0),
            (13, 0),
            (14, 0),
            (15, 0),
            (16, 0),
            (17, 0),
        ])
    );
    assert_eq!(
        associated_string_padding,
        BTreeMap::from([(0, 350), (15, 1)])
    );
    assert_eq!(user_variable_tables, 5);
    assert_eq!(user_variables, 42);
    assert_eq!(user_variable_name_units, 425);
    assert_eq!(user_variable_value_units, 76_585);
    assert_eq!(maximum_user_variable_value_units, 65_280);
    assert_eq!(user_variable_nonzero_metadata, 22);
    assert_eq!(
        user_variable_kinds,
        BTreeMap::from([(UserVariableKind::Ordinary, 42)])
    );
    assert_eq!(
        user_variable_table_shapes,
        BTreeMap::from([
            ((2, 92), 1),
            ((3, 112), 1),
            ((8, 153_236), 1),
            ((10, 458), 1),
            ((19, 488), 1),
        ])
    );
    assert_eq!(mail_merge_tables, 1);
    assert_eq!(new_mail_merge_tables, 0);
    assert_eq!(office_data_source_tables, 0);
    assert_eq!(office_data_source_properties, 0);
    assert_eq!(mail_merge_sql_units, 50);
    assert_eq!(mail_merge_string_tables, 0);
    assert_eq!(mail_merge_document_type_records, 1);
    assert_eq!(mail_merge_compatibility_sources, 2);
    assert_eq!(
        mail_merge_document_types,
        BTreeMap::from([(MailMergeDocumentType::Letters, 1)])
    );
    assert_eq!(
        mail_merge_destinations,
        BTreeMap::from([(MailMergeDestination::None, 1)])
    );
    assert_eq!(
        mail_merge_source_kinds,
        BTreeMap::from([(MailMergeSourceKind::DataFile, 2)])
    );
    assert_eq!(
        mail_merge_error_handling,
        BTreeMap::from([(MailMergeErrorHandling::CompleteAndPause, 1)])
    );
    assert_eq!(
        mail_merge_shapes,
        BTreeMap::from([((136, 50, false, true), 1)])
    );
    assert_eq!(subdocument_tables, 0);
    assert_eq!(subdocument_references, 0);
    assert_eq!(subdocument_nonzero_ignored_flags, 0);
    assert_eq!(external_file_name_tables, 0);
    assert_eq!(external_file_names, 0);
    assert_eq!(format_consistency_bookmark_tables, 0);
    assert_eq!(format_consistency_bookmarks, 0);
    assert_eq!(repair_bookmark_tables, 0);
    assert_eq!(repair_bookmarks, 0);
    assert_eq!(xml_schema_tables, 0);
    assert_eq!(xml_schema_references, 0);
    assert_eq!(xml_schema_element_names, 0);
    assert_eq!(xml_schema_attribute_names, 0);
    assert_eq!(xml_schema_ansi_tables, 0);
    assert_eq!(structured_tag_bookmark_tables, 0);
    assert_eq!(structured_tag_bookmarks, 0);
    assert_eq!(structured_tag_attributes, 0);
    assert_eq!(structured_tag_placeholder_units, 0);
    assert_eq!(xml_transform_paths, 0);
    assert_eq!(xml_transform_path_units, 0);
    assert_eq!(range_protection_tables, 0);
    assert_eq!(range_permissions, 0);
    assert_eq!(protected_user_tables, 0);
    assert_eq!(protected_users, 0);
    assert_eq!(caption_tables, 0);
    assert_eq!(caption_definitions, 0);
    assert_eq!(auto_caption_tables, 0);
    assert_eq!(auto_caption_definitions, 0);
    assert_eq!(ignored_non_template_caption_pairs, 1);
    assert_eq!(revision_author_tables, 319);
    assert_eq!(revision_authors, 343);
    assert_eq!(revision_author_units, 2_441);
    assert_eq!(maximum_revision_author_length, 30);
    assert_eq!(revision_author_zero_placeholders, 1);
    assert_eq!(
        revision_author_count_shapes,
        BTreeMap::from([(0, 1), (1, 304), (2, 11), (3, 1), (4, 1), (10, 1)])
    );
    assert_eq!(spelling_state_tables, 313);
    assert_eq!(spelling_ranges, 14_745);
    assert_eq!(spelling_duplicate_positions, 259);
    assert_eq!(
        spelling_state_shapes,
        BTreeMap::from([
            ((SpellingStateKind::MaybeDirty, false), 470),
            ((SpellingStateKind::Dirty, false), 148),
            ((SpellingStateKind::Edit, false), 701),
            ((SpellingStateKind::Edit, true), 1),
            ((SpellingStateKind::Foreign, false), 1_131),
            ((SpellingStateKind::Clean, false), 7_341),
            ((SpellingStateKind::RepeatWord, true), 10),
            ((SpellingStateKind::UnknownWord, true), 4_936),
            ((SpellingStateKind::Compatibility13, true), 7),
        ])
    );
    assert_eq!(grammar_state_tables, 296);
    assert_eq!(grammar_ranges, 6_777);
    assert_eq!(grammar_duplicate_positions, 219);
    assert_eq!(
        grammar_state_shapes,
        BTreeMap::from([
            ((GrammarStateKind::MaybeDirty, false, false, false), 447),
            ((GrammarStateKind::Dirty, false, false, false), 387),
            ((GrammarStateKind::Dirty, true, true, false), 618),
            ((GrammarStateKind::Edit, false, false, false), 693),
            ((GrammarStateKind::Foreign, false, false, false), 809),
            ((GrammarStateKind::Clean, false, false, false), 3_172),
            ((GrammarStateKind::ErrorMin, true, false, false), 441),
            ((GrammarStateKind::ErrorMin, true, true, false), 66),
            ((GrammarStateKind::ErrorMin, true, true, true), 144),
        ])
    );
    assert_eq!(language_detection_state_tables, 218);
    assert_eq!(language_detection_ranges, 4_833);
    assert_eq!(language_detection_duplicate_positions, 385);
    assert_eq!(
        language_detection_state_shapes,
        BTreeMap::from([
            ((LanguageDetectionStateKind::MaybeDirty, false), 444),
            ((LanguageDetectionStateKind::Dirty, false), 448),
            ((LanguageDetectionStateKind::Edit, false), 1_088),
            ((LanguageDetectionStateKind::Foreign, false), 1_016),
            ((LanguageDetectionStateKind::Clean, false), 1_809),
            ((LanguageDetectionStateKind::NoLanguageDetection, false), 28,),
        ])
    );
    assert_eq!(list_style_template_tables, 132);
    assert_eq!(list_style_template_lists, 2_834);
    assert_eq!(empty_list_style_templates, 2_145);
    assert_eq!(built_in_list_level_templates, 5_651);
    assert_eq!(user_list_level_templates, 550);
    assert_eq!(list_style_template_count_mismatches, 5);
    assert_eq!(
        extra_list_style_template_counts,
        BTreeMap::from([(3, 1), (6, 1), (12, 1), (14, 1), (755, 1)])
    );
    assert_eq!(frame_and_list_tables, 113);
    assert_eq!(frame_and_list_records, 113);
    assert_eq!(list_style_references, 1_436);
    assert_eq!(custom_list_style_references, 5);
    assert_eq!(standard_list_style_references, 1_431);
    assert_eq!(out_of_range_custom_list_style_references, 0);
    assert_eq!(grammar_option_tables, 6);
    assert_eq!(grammar_options, 27);
    assert_eq!(legacy_grammar_option_tables, 5);
    assert_eq!(legacy_grammar_options, 6);
    assert_eq!(
        legacy_grammar_option_shapes,
        BTreeMap::from([
            ((0, 1036, 512, 9), 1),
            ((0, 1049, 512, 1), 1),
            ((1, 1033, 513, 8), 1),
            ((1, 1036, 512, 9), 1),
            ((1, 1049, 512, 1), 1),
            ((1, 2057, 513, 8), 1),
        ])
    );
    assert_eq!(
        grammar_option_shapes,
        BTreeMap::from([
            ((0, 1033, 0, 64), 1),
            ((0, 1033, 4096, 64), 1),
            ((0, 1033, 131078, 64), 2),
            ((0, 1036, 0, 64), 1),
            ((0, 1036, 4096, 64), 1),
            ((0, 2057, 0, 64), 1),
            ((0, 2057, 4096, 64), 1),
            ((0, 2057, 131078, 64), 2),
            ((0, 3081, 131078, 64), 1),
            ((1, 1031, 131078, 64), 1),
            ((1, 1033, 5, 64), 1),
            ((1, 1033, 6, 64), 2),
            ((1, 1033, 131078, 64), 2),
            ((1, 1036, 6, 64), 1),
            ((1, 1036, 131078, 64), 2),
            ((1, 2052, 5, 64), 1),
            ((1, 2057, 6, 64), 1),
            ((1, 2057, 131077, 64), 1),
            ((1, 2057, 131078, 64), 2),
            ((1, 3081, 131078, 64), 1),
            ((1, 5129, 131078, 64), 1),
        ])
    );
    assert_eq!(smart_tag_state_tables, 100);
    assert_eq!(smart_tag_state_ranges, 3_911);
    assert_eq!(smart_tag_duplicate_positions, 1_245);
    assert_eq!(
        smart_tag_state_shapes,
        BTreeMap::from([
            (SmartTagRecognizerStateKind::Pending, 1),
            (SmartTagRecognizerStateKind::MaybeDirty, 318),
            (SmartTagRecognizerStateKind::Dirty, 919),
            (SmartTagRecognizerStateKind::Edit, 1_644),
            (SmartTagRecognizerStateKind::Clean, 1_029),
        ])
    );
    assert_eq!(paragraph_group_tables, 31);
    assert_eq!(paragraph_group_entries, 3_023);
    assert_eq!(paragraph_group_root_entries, 670);
    assert_eq!(paragraph_group_maximum_depth, 3);
    assert_eq!(paragraph_group_missing_parents, 0);
    assert_eq!(
        paragraph_group_option_shapes,
        BTreeMap::from([
            (0, 2_284),
            (1, 3),
            (4, 9),
            (8, 37),
            (192, 2),
            (213, 1),
            (256, 655),
            (257, 2),
            (260, 2),
            (261, 11),
            (269, 5),
            (271, 3),
            (285, 6),
            (287, 3),
        ])
    );
    assert_eq!(
        paragraph_group_html_types,
        BTreeMap::from([(HtmlBlockType::BlockQuote, 17), (HtmlBlockType::Body, 670)])
    );
    assert_eq!(save_history_tables, 56);
    assert_eq!(save_history_entries, 281);
    assert_eq!(save_history_author_units, 2_653);
    assert_eq!(save_history_path_units, 19_794);
    assert_eq!(save_history_maximum_author_length, 19);
    assert_eq!(save_history_maximum_path_length, 242);
    assert_eq!(
        save_history_entry_counts,
        BTreeMap::from([
            (1, 23),
            (2, 5),
            (4, 3),
            (5, 1),
            (6, 1),
            (7, 1),
            (8, 1),
            (10, 21)
        ])
    );
    assert_eq!(smart_tag_bookmark_tables, 16);
    assert_eq!(smart_tag_bookmarks, 339);
    assert_eq!(smart_tag_sub_entities, 71);
    assert_eq!(smart_tag_nonzero_unused, 322);
    assert_eq!(smart_tag_nonzero_property_bag_pointers, 160);
    assert_eq!(
        smart_tag_sources,
        BTreeMap::from([
            (SmartTagSource::Unknown, 1),
            (SmartTagSource::Grammar, 140),
            (SmartTagSource::ScanDll, 198),
        ])
    );
    assert_eq!(smart_tag_start_depths, BTreeMap::from([(1, 214), (2, 125)]));
    assert_eq!(smart_tag_end_depths, BTreeMap::from([(0, 326), (1, 13)]));
    assert_eq!(grammar_cookie_data_tables, 1);
    assert_eq!(grammar_cookie_data_entries, 1);
    assert_eq!(grammar_cookie_provider_bytes, 4);
    assert_eq!(grammar_cookie_data_shapes, BTreeMap::from([((1, 16), 1)]));
    assert_eq!(grammar_cookie_unreferenced_data, 0);
    assert_eq!(legacy_grammar_cookie_tables, 0);
    assert_eq!(legacy_grammar_cookies, 0);
    assert_eq!(legacy_grammar_cookie_errors, 0);
    assert_eq!(legacy_grammar_cookie_duplicate_positions, 0);
    assert_eq!(legacy_grammar_cookie_shapes, BTreeMap::new());
    assert_eq!(grammar_cookie_tables, 1);
    assert_eq!(grammar_cookies, 1);
    assert_eq!(grammar_cookie_headers, 1);
    assert_eq!(grammar_cookie_errors, 1);
    assert_eq!(grammar_cookie_duplicate_positions, 0);
    assert_eq!(
        grammar_cookie_error_types,
        BTreeMap::from([(GrammarCookieErrorType::Typo, 1)])
    );
    assert_eq!(grammar_cookie_languages, BTreeMap::from([((1, 17), 1)]));
    assert_eq!(
        grammar_cookie_shapes,
        BTreeMap::from([(
            (
                16,
                3_479,
                8,
                GrammarCookieErrorType::Typo,
                true,
                1,
                17,
                true
            ),
            1
        )])
    );
    assert_eq!(table_character_cache_tables, 345);
    assert_eq!(table_character_cache_ranges, 5_845);
    assert_eq!(table_character_unknown_ranges, 355);
    assert_eq!(table_character_nonzero_unused, 5_124);
    assert_eq!(table_character_canonical_undefined, 218);
    assert_eq!(
        table_character_cache_shapes,
        BTreeMap::from([
            (1, 70),
            (2, 226),
            (3, 2),
            (4, 3),
            (5, 3),
            (7, 5),
            (8, 4),
            (9, 1),
            (10, 1),
            (11, 1),
            (12, 1),
            (13, 1),
            (14, 2),
            (15, 1),
            (18, 1),
            (20, 1),
            (22, 1),
            (24, 1),
            (26, 1),
            (28, 1),
            (31, 1),
            (35, 1),
            (40, 2),
            (43, 3),
            (45, 1),
            (65, 1),
            (93, 1),
            (99, 1),
            (119, 1),
            (126, 1),
            (156, 1),
            (245, 1),
            (745, 2),
            (2_274, 1),
        ])
    );
    assert_eq!(smart_tag_data_tables, 16);
    assert_eq!(smart_tag_factoid_types, 54);
    assert_eq!(smart_tag_malformed_cve_factoid_types, 1);
    assert_eq!(smart_tag_property_bags, 339);
    assert_eq!(smart_tag_properties, 16);
    assert_eq!(smart_tag_ansi_strings, 182);
    assert_eq!(smart_tag_unicode_strings, 7);
    assert_eq!(
        smart_tag_reserved_factoid_counts,
        BTreeMap::from([
            (0, 1),
            (1, 1),
            (18, 1),
            (654, 1),
            (1_856_816, 1),
            (71_280_872, 1),
            (73_524_768, 1),
            (93_529_752, 1),
            (146_562_804, 2),
            (168_487_032, 1),
            (203_835_684, 1),
            (209_832_440, 1),
            (252_695_824, 1),
            (259_139_680, 1),
            (280_216_412, 1),
        ])
    );
    assert_eq!(smart_tag_property_bag_count_mismatches, 0);
    assert_eq!(revision_threading_tables, 313);
    assert_eq!(revision_thread_messages, 329);
    assert_eq!(revision_thread_message_units, 0);
    assert_eq!(revision_thread_style_units, 0);
    assert_eq!(revision_thread_nonempty_messages, 0);
    assert_eq!(revision_thread_nonempty_styles, 0);
    assert_eq!(revision_thread_nonzero_dates, 0);
    assert_eq!(revision_thread_nonzero_reserved, 0);
    assert_eq!(
        revision_thread_author_indexes,
        BTreeMap::from([
            (0, 313),
            (1, 8),
            (2, 1),
            (3, 1),
            (4, 1),
            (5, 1),
            (6, 1),
            (7, 1),
            (8, 1),
            (9, 1),
        ])
    );
    assert_eq!(revision_thread_author_attributes, 0);
    assert_eq!(revision_thread_message_attributes, 0);
    assert_eq!(revision_thread_attribute_units, 0);
    assert_eq!(revision_thread_value_units, 0);
    assert_eq!(revision_thread_author_count_mismatches, 0);
    assert_eq!(
        revision_thread_message_count_shapes,
        BTreeMap::from([(1, 305), (2, 7), (10, 1)])
    );
    assert_eq!(revision_save_id_tables, 284);
    assert_eq!(revision_save_ids, 37_728);
    assert_eq!(zero_revision_save_ids, 0);
    assert_eq!(duplicate_revision_save_ids, 0);
    assert_eq!(distinct_revision_save_ids.len(), 37_043);
    assert_eq!(revision_save_id_reserved2, BTreeMap::from([(0, 284)]));
    assert_eq!(nonzero_revision_save_id_reserved3, 276);
    assert_eq!(
        revision_save_id_count_shapes,
        BTreeMap::from([
            (1, 7),
            (2, 30),
            (3, 24),
            (4, 21),
            (5, 19),
            (6, 17),
            (7, 15),
            (8, 13),
            (9, 9),
            (10, 10),
            (11, 8),
            (12, 6),
            (13, 2),
            (14, 3),
            (15, 4),
            (16, 6),
            (17, 2),
            (19, 5),
            (20, 3),
            (21, 1),
            (22, 3),
            (23, 4),
            (24, 1),
            (25, 1),
            (27, 3),
            (28, 2),
            (29, 2),
            (30, 1),
            (32, 1),
            (33, 2),
            (34, 2),
            (35, 1),
            (36, 1),
            (37, 1),
            (38, 1),
            (39, 2),
            (45, 1),
            (46, 1),
            (52, 1),
            (53, 1),
            (54, 1),
            (58, 1),
            (60, 1),
            (62, 2),
            (64, 1),
            (67, 2),
            (71, 1),
            (74, 1),
            (75, 1),
            (76, 1),
            (83, 1),
            (89, 1),
            (90, 2),
            (95, 1),
            (110, 1),
            (112, 1),
            (120, 1),
            (139, 1),
            (146, 1),
            (147, 1),
            (164, 1),
            (174, 1),
            (193, 1),
            (196, 1),
            (199, 1),
            (202, 1),
            (213, 1),
            (221, 1),
            (224, 1),
            (225, 1),
            (239, 1),
            (256, 1),
            (286, 1),
            (310, 1),
            (358, 1),
            (360, 1),
            (398, 1),
            (401, 1),
            (465, 1),
            (514, 1),
            (776, 1),
            (861, 1),
            (3_023, 1),
            (23_037, 1),
        ])
    );
    assert_eq!(
        font_family_shapes,
        BTreeMap::from([
            ((0, false, 0), 2),
            ((0, false, 1), 14),
            ((0, false, 2), 3),
            ((0, true, 0), 58),
            ((0, true, 1), 19),
            ((0, true, 2), 34),
            ((0, true, 3), 13),
            ((0, true, 4), 1),
            ((0, true, 5), 1),
            ((1, false, 0), 1),
            ((1, false, 1), 2),
            ((1, false, 3), 8),
            ((1, true, 0), 1),
            ((1, true, 1), 2),
            ((1, true, 3), 127),
            ((1, true, 4), 1),
            ((2, false, 0), 11),
            ((2, false, 1), 50),
            ((2, false, 2), 17),
            ((2, false, 5), 1),
            ((2, true, 0), 331),
            ((2, true, 1), 1_168),
            ((2, true, 2), 943),
            ((2, true, 3), 1),
            ((2, true, 4), 5),
            ((2, true, 5), 3),
        ])
    );
    assert_eq!(
        font_character_sets,
        BTreeMap::from([
            (0, 1_540),
            (1, 223),
            (2, 493),
            (77, 1),
            (128, 160),
            (129, 4),
            (134, 46),
            (136, 17),
            (161, 4),
            (177, 1),
            (178, 3),
            (204, 112),
            (238, 213),
        ])
    );
    assert_eq!(
        document_property_shapes,
        BTreeMap::from([
            ((0x00c1, 500), 19),
            ((0x00c2, 500), 1),
            ((0x00c3, 544), 1),
            ((0x00d9, 544), 26),
            ((0x00d9, 600), 11),
            ((0x0101, 594), 16),
            ((0x0101, 610), 57),
            ((0x010c, 616), 52),
            ((0x010c, 674), 2),
            ((0x010c, 690), 1),
            ((0x0112, 674), 60),
            ((0x0112, 690), 60),
            ((0x0112, 694), 97),
        ])
    );
    assert_eq!(
        document_classifications,
        BTreeMap::from([(DocumentClassification::NotSpecified, 403)])
    );
    assert_eq!(document_undefined_space_shapes, BTreeSet::from([[0; 30]]));
    assert_eq!(document_nonzero_undefined_spaces, 0);
    assert_eq!(document_nonzero_undefined_space_bytes, 0);
    assert_eq!(
        document_last_list_indexes,
        BTreeSet::from([(0, 0), (0, 1), (0, 5), (0, 19)])
    );
    assert_eq!(document_nonzero_last_list_indexes, 3);
    assert_eq!(document_last_list_index_matches, 401);
    assert_eq!(
        document_last_list_index_mismatches,
        BTreeMap::from([((0, 1, 1), 1), ((0, 19, 19), 1)])
    );
    assert_eq!(document_cleanup_limit_matches, 264);
    assert_eq!(
        document_cleanup_limit_mismatches,
        BTreeMap::from([
            ((1, 1), 1),
            ((4, 4), 1),
            ((5, 5), 1),
            ((10, 10), 1),
            ((11, 11), 1),
            ((14, 14), 1),
            ((21, 21), 1),
            ((27, 27), 1),
        ])
    );
    assert_eq!(
        document_numbering_cache_states,
        BTreeMap::from([
            ((false, false), 265),
            ((false, true), 24),
            ((true, false), 45),
            ((true, true), 69),
        ])
    );
    assert_eq!(
        document_numbering_cache_lengths,
        BTreeMap::from([
            (12, 107),
            (28, 2),
            (36, 1),
            (44, 1),
            (60, 1),
            (108, 1),
            (364, 1),
        ])
    );
    assert_eq!(
        document_numbering_cache_present_max_positions,
        BTreeSet::from([
            0, 7, 91, 168, 934, 3932, 7200, 7919, 9728, 14943, 16495, 30574
        ])
    );
    assert_eq!(
        document_numbering_cache_absent_max_positions,
        BTreeSet::from([0])
    );
    assert_eq!(
        document_note_number_formats,
        BTreeMap::from([
            ((0x00, 0x00), 9),
            ((0x00, 0x02), 392),
            ((0x00, 0x04), 1),
            ((0x04, 0x04), 1)
        ])
    );
    assert_eq!(
        document_pagination_display_shapes,
        BTreeMap::from([
            ((0, 0), 389),
            ((0, 368), 4),
            ((0, 448), 1),
            ((0, 506), 1),
            ((0, 578), 1),
            ((0, 605), 1),
            ((0, 648), 1),
            ((0, 653), 1),
            ((0, 805), 1),
            ((0, 820), 1),
            ((0, 824), 1),
            ((0, 829), 1),
        ])
    );
    assert_eq!(document_characters_with_spaces_shapes.len(), 233);
    assert_eq!(
        document_characters_with_spaces_shapes.first(),
        Some(&(0, 0))
    );
    assert_eq!(
        document_characters_with_spaces_shapes.last(),
        Some(&(289_840, 289_840))
    );
    assert_eq!(
        document_double_byte_character_shapes,
        BTreeSet::from([(0, 0), (1, 1)])
    );
    assert_eq!(document_negative_character_count_pairs, 0);
    assert_eq!(document_character_count_relation_mismatches, 32);
    assert_eq!(
        document_character_statistic_states,
        BTreeMap::from([
            ((false, false, false, true), 32),
            ((false, false, true, true), 118),
            ((false, true, true, true), 247),
            ((true, false, true, true), 6),
        ])
    );
    assert_eq!(document_reserved3a_values, BTreeMap::from([(0, 403)]));
    assert_eq!(document_main_statistic_shapes.len(), 296);
    assert_eq!(document_subdocument_statistic_shapes.len(), 276);
    assert_eq!(document_negative_statistics, 0);
    assert_eq!(document_statistic_relation_mismatches, 29);
    assert_eq!(
        document_statistic_states,
        BTreeMap::from([
            ((false, false, false), 29),
            ((false, false, true), 121),
            ((false, true, true), 247),
            ((true, false, true), 6),
        ])
    );
    assert_eq!(document_exact_statistic_character_bound_mismatches, 0);
    assert_eq!(document_created_timestamps.len(), 381);
    assert_eq!(document_revised_timestamps.len(), 367);
    assert_eq!(document_last_printed_timestamps.len(), 89);
    assert_eq!(document_ignored_timestamp_counts, [6, 25, 294]);
    assert_eq!(document_revision_counts.len(), 33);
    assert_eq!(document_revision_counts.first(), Some(&1));
    assert_eq!(document_revision_counts.last(), Some(&135));
    assert_eq!(document_editing_times.len(), 63);
    assert_eq!(document_editing_times.first(), Some(&0));
    assert_eq!(document_editing_times.last(), Some(&4_287));
    assert_eq!(document_negative_editing_times, 0);
    assert_eq!(document_protection_hashes.len(), 6);
    assert_eq!(document_protection_hashes.first(), Some(&-2_041_130_755));
    assert_eq!(document_protection_hashes.last(), Some(&609_995_782));
    assert_eq!(
        document_protection_hash_states,
        BTreeMap::from([
            ((false, false), 383),
            ((true, false), 13),
            ((true, true), 7),
        ])
    );
    assert_eq!(
        document_default_tab_widths,
        BTreeSet::from([
            284, 360, 420, 432, 480, 576, 624, 706, 708, 709, 710, 720, 794, 851, 1_298, 1_304,
        ])
    );
    assert_eq!(
        document_web_code_pages,
        BTreeMap::from([
            (0, 360),
            (932, 1),
            (950, 1),
            (1_250, 2),
            (1_251, 3),
            (1_252, 29),
            (1_253, 2),
            (10_000, 1),
            (20_127, 1),
            (65_001, 3),
        ])
    );
    assert_eq!(
        document_hyphenation_zones,
        BTreeSet::from([0, 357, 360, 420, 425, 950, 1_026])
    );
    assert_eq!(
        document_consecutive_hyphen_limits,
        BTreeSet::from([0, 28_257])
    );
    assert_eq!(document_reserved2_values, BTreeMap::from([(0, 403)]));
    assert_eq!(document_lock_revision_marking_mismatches, 0);
    assert_eq!(document_lock_revision_annotation_conflicts, 0);
    assert_eq!(
        document_typography_shapes,
        BTreeMap::from([
            (
                (
                    TypographyJustification::DoNotCompress,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    false,
                    false,
                    false,
                    false,
                    0,
                    0,
                ),
                118,
            ),
            (
                (
                    TypographyJustification::DoNotCompress,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    false,
                    false,
                    false,
                    true,
                    0,
                    0,
                ),
                72,
            ),
            (
                (
                    TypographyJustification::DoNotCompress,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    true,
                    false,
                    false,
                    false,
                    0,
                    0,
                ),
                191,
            ),
            (
                (
                    TypographyJustification::DoNotCompress,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    true,
                    false,
                    false,
                    false,
                    63,
                    28,
                ),
                1,
            ),
            (
                (
                    TypographyJustification::CompressPunctuation,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    true,
                    false,
                    false,
                    false,
                    0,
                    0,
                ),
                16,
            ),
            (
                (
                    TypographyJustification::CompressPunctuation,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    true,
                    false,
                    false,
                    false,
                    51,
                    22,
                ),
                1,
            ),
            (
                (
                    TypographyJustification::CompressPunctuation,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    true,
                    false,
                    false,
                    false,
                    63,
                    27,
                ),
                2,
            ),
            (
                (
                    TypographyJustification::CompressPunctuation,
                    KinsokuLevel::LanguageDefault,
                    CustomKinsokuLanguage::None,
                    true,
                    false,
                    false,
                    false,
                    90,
                    38,
                ),
                1,
            ),
            (
                (
                    TypographyJustification::CompressPunctuation,
                    KinsokuLevel::Custom,
                    CustomKinsokuLanguage::ChineseTraditional,
                    true,
                    false,
                    false,
                    false,
                    61,
                    28,
                ),
                1,
            ),
        ])
    );
    assert_eq!(document_typography_following_units, 391);
    assert_eq!(document_typography_leading_units, 170);
    assert_eq!(document_typography_nonzero_unused_following_slots, 0);
    assert_eq!(document_typography_nonzero_unused_leading_slots, 0);
    assert_eq!(
        document_compatibility_option_shapes,
        BTreeMap::from([
            ((0x0000, 0x0000_0000), 42),
            ((0x0000, 0x0000_2000), 41),
            ((0x0000, 0x0000_3000), 1),
            ((0x0000, 0x0008_0000), 3),
            ((0x0000, 0x0010_f000), 11),
            ((0x0000, 0x0400_0000), 1),
            ((0x0000, 0x0408_0000), 1),
            ((0x0000, 0x0410_0000), 1),
            ((0x0000, 0x8000_0000), 2),
            ((0x0000, 0x8410_0000), 1),
            ((0x0029, 0x8040_0029), 1),
            ((0x0c56, 0x8400_0c56), 1),
            ((0x0cd6, 0x8400_0cd6), 1),
            ((0x2000, 0x0000_2000), 22),
            ((0xf000, 0x0010_f000), 263),
            ((0xf000, 0x0410_f000), 1),
            ((0xf000, 0x8410_f000), 3),
            ((0xf029, 0x0410_f029), 1),
            ((0xf229, 0x8410_f229), 1),
            ((0xf580, 0x843b_f580), 1),
            ((0xfc56, 0x0410_fc56), 2),
            ((0xfc56, 0x8010_fc56), 1),
            ((0xfc56, 0x8410_fc56), 1),
        ])
    );
    assert_eq!(
        document_compatibility_option_mismatches,
        BTreeMap::from([(0x2000, 41), (0x3000, 1), (0xf000, 11)])
    );
    assert_eq!(
        document_format_flag_shapes,
        BTreeMap::from([
            (0x20, 9),
            (0x21, 1),
            (0x22, 304),
            (0x23, 16),
            (0x26, 1),
            (0x42, 72),
        ])
    );
    assert_eq!(
        document_footnote_numbering_shapes,
        BTreeMap::from([(0x0004, 397), (0x0005, 2), (0x0006, 4)])
    );
    assert_eq!(document_state_flag_shapes.len(), 75);
    let document_state_bit_frequencies: [usize; 32] = std::array::from_fn(|bit| {
        document_state_flag_shapes
            .iter()
            .filter(|(bits, _)| (*bits >> bit) & 1 != 0)
            .map(|(_, count)| *count)
            .sum()
    });
    assert_eq!(
        document_state_bit_frequencies,
        [
            394, 6, 0, 0, 262, 262, 163, 108, 7, 5, 0, 384, 6, 0, 3, 7, 77, 6, 185, 352, 0, 3, 0,
            398, 1, 13, 5, 398, 386, 0, 1, 0,
        ]
    );
    assert_eq!(
        document_endnote_numbering_shapes,
        BTreeMap::from([(0x0004, 403)])
    );
    assert_eq!(
        document_endnote_option_shapes,
        BTreeMap::from([
            (0x0003, 14),
            (0x0083, 7),
            (0x0113, 1),
            (0x1000, 1),
            (0x1003, 124),
            (0x1083, 9),
            (0x8003, 1),
            (0x9000, 1),
            (0x9003, 146),
            (0x9080, 3),
            (0x9083, 96),
        ])
    );
    assert_eq!(document_saved_view_shapes.len(), 59);
    assert_eq!(
        document_saved_view_kinds,
        BTreeMap::from([
            (SavedViewKind::Print, 322),
            (SavedViewKind::Normal, 11),
            (SavedViewKind::Web, 2),
            (SavedViewKind::Compatibility7, 68),
        ])
    );
    assert_eq!(
        document_saved_zoom_kinds,
        BTreeMap::from([(0, 391), (1, 2), (2, 9), (3, 1)])
    );
    assert_eq!(document_saved_zoom_percentages.len(), 47);
    assert_eq!(document_saved_zoom_percentages.first(), Some(&0));
    assert_eq!(document_saved_zoom_percentages.last(), Some(&348));
    assert_eq!(
        document_display_flag_shapes,
        BTreeMap::from([
            (0x0012, 6),
            (0x0032, 5),
            (0x0072, 8),
            (0x0412, 3),
            (0x0432, 1),
            (0x043e, 1),
            (0x0472, 2),
            (0x3012, 179),
            (0x3032, 26),
            (0x303e, 1),
            (0x3072, 85),
            (0x3412, 41),
            (0x3432, 16),
            (0x3472, 29),
        ])
    );
    assert_eq!(
        document_outline_levels,
        BTreeMap::from([
            (SavedOutlineLevel::All9, 401),
            (SavedOutlineLevel::All15, 2)
        ])
    );
    assert_eq!(document_version_flag_shapes, BTreeMap::from([(0, 403)]));
    assert_eq!(document_event_shapes, BTreeMap::from([(0, 401), (2, 2)]));
    assert_eq!(
        document_virus_flag_shapes,
        BTreeMap::from([
            ((false, false), 383),
            ((true, false), 17),
            ((true, true), 3)
        ])
    );
    assert_eq!(document_virus_session_keys.len(), 17);
    assert_eq!(document_virus_session_keys.first(), Some(&0));
    assert_eq!(document_virus_session_keys.last(), Some(&0x360a_ec15));
    let mut drawing_grid_vertical_frequencies = BTreeMap::<GridDisplayFrequency, usize>::new();
    let mut drawing_grid_horizontal_frequencies = BTreeMap::<GridDisplayFrequency, usize>::new();
    let mut drawing_grid_flag_shapes = BTreeMap::<(bool, bool), usize>::new();
    let mut drawing_grid_horizontal_origins = BTreeSet::new();
    let mut drawing_grid_vertical_origins = BTreeSet::new();
    let mut drawing_grid_horizontal_spacings = BTreeSet::new();
    let mut drawing_grid_vertical_spacings = BTreeSet::new();
    for (
        (
            horizontal_origin,
            vertical_origin,
            horizontal_spacing,
            vertical_spacing,
            vertical_frequency,
            unused,
            horizontal_frequency,
            follow_margins,
        ),
        count,
    ) in &document_drawing_grid_shapes
    {
        *drawing_grid_vertical_frequencies
            .entry(*vertical_frequency)
            .or_default() += *count;
        *drawing_grid_horizontal_frequencies
            .entry(*horizontal_frequency)
            .or_default() += *count;
        *drawing_grid_flag_shapes
            .entry((*unused, *follow_margins))
            .or_default() += *count;
        drawing_grid_horizontal_origins.insert(*horizontal_origin);
        drawing_grid_vertical_origins.insert(*vertical_origin);
        drawing_grid_horizontal_spacings.insert(*horizontal_spacing);
        drawing_grid_vertical_spacings.insert(*vertical_spacing);
    }
    assert_eq!(document_drawing_grid_shapes.len(), 80);
    assert_eq!(
        drawing_grid_vertical_frequencies,
        BTreeMap::from([
            (GridDisplayFrequency::DisabledCompatibility, 161),
            (GridDisplayFrequency::Every(1), 211),
            (GridDisplayFrequency::Every(2), 26),
            (GridDisplayFrequency::Every(3), 5),
        ])
    );
    assert_eq!(
        drawing_grid_horizontal_frequencies,
        BTreeMap::from([
            (GridDisplayFrequency::DisabledCompatibility, 177),
            (GridDisplayFrequency::Every(1), 195),
            (GridDisplayFrequency::Every(2), 31),
        ])
    );
    assert_eq!(
        drawing_grid_flag_shapes,
        BTreeMap::from([
            ((false, false), 110),
            ((false, true), 5),
            ((true, false), 52),
            ((true, true), 236),
        ])
    );
    assert_eq!(drawing_grid_horizontal_origins.len(), 27);
    assert_eq!(drawing_grid_horizontal_origins.first(), Some(&0));
    assert_eq!(drawing_grid_horizontal_origins.last(), Some(&2_160));
    assert_eq!(drawing_grid_vertical_origins.len(), 35);
    assert_eq!(drawing_grid_vertical_origins.first(), Some(&0));
    assert_eq!(drawing_grid_vertical_origins.last(), Some(&2_999));
    assert_eq!(drawing_grid_horizontal_spacings.len(), 13);
    assert_eq!(drawing_grid_horizontal_spacings.first(), Some(&0));
    assert_eq!(drawing_grid_horizontal_spacings.last(), Some(&360));
    assert_eq!(drawing_grid_vertical_spacings.len(), 10);
    assert_eq!(drawing_grid_vertical_spacings.first(), Some(&0));
    assert_eq!(drawing_grid_vertical_spacings.last(), Some(&360));
    assert_eq!(document_2000_extensions, 383);
    assert_eq!(document_2000_level_shapes, BTreeMap::from([((0, 0), 383)]));
    let dop2000_flag_bit_frequencies: [usize; 32] = std::array::from_fn(|bit| {
        document_2000_flag_shapes
            .iter()
            .filter(|(bits, _)| (*bits >> bit) & 1 != 0)
            .map(|(_, count)| *count)
            .sum()
    });
    assert_eq!(document_2000_flag_shapes.len(), 72);
    assert_eq!(
        dop2000_flag_bit_frequencies,
        [
            76, 0, 56, 64, 0, 0, 0, 0, 132, 308, 7, 117, 165, 165, 143, 0, 308, 308, 0, 0, 0, 11,
            6, 303, 308, 0, 0, 0, 307, 38, 88, 0,
        ]
    );
    assert_eq!(
        document_2000_screen_sizes,
        BTreeMap::from([
            (WebTargetScreenSize::Size544x376, 75),
            (WebTargetScreenSize::Size800x600, 165),
            (WebTargetScreenSize::Size1024x768, 143),
        ])
    );
    assert_eq!(document_2000_initialized_web_options, 307);
    assert_eq!(
        document_2000_initialized_ppi,
        BTreeMap::from([(72, 5), (96, 296), (120, 6)])
    );
    let copts_named_bit_frequencies: [usize; 32] = std::array::from_fn(|bit| {
        document_copts_named_shapes
            .iter()
            .filter(|(bits, _)| (*bits >> bit) & 1 != 0)
            .map(|(_, count)| *count)
            .sum()
    });
    assert_eq!(document_copts_named_shapes.len(), 41);
    assert_eq!(
        copts_named_bit_frequencies,
        [
            43, 43, 110, 282, 43, 2, 43, 43, 42, 43, 58, 59, 55, 7, 60, 59, 97, 98, 247, 247, 247,
            247, 247, 247, 247, 17, 247, 247, 248, 247, 247, 247,
        ]
    );
    assert_eq!(document_copts_cached_column_balance, 236);
    assert_eq!(document_copts_nonzero_empty1, 1);
    assert_eq!(document_copts_nonzero_empty_dwords, 1);
    assert_eq!(document_copts_word8_mismatches, 0);
    assert_eq!(
        document_2000_pre_word10_shapes,
        BTreeMap::from([(0x0000, 173), (0x0200, 1), (0x0800, 209)])
    );
    assert_eq!(
        document_2000_flag2_shapes,
        BTreeMap::from([
            (0x0000, 91),
            (0x0008, 2),
            (0x0040, 1),
            (0x0048, 9),
            (0x004c, 1),
            (0x00c0, 1),
            (0x0258, 1),
            (0x5000, 1),
            (0x5008, 10),
            (0x500c, 1),
            (0x5040, 42),
            (0x5048, 78),
            (0x504b, 3),
            (0x50c0, 3),
            (0x50c8, 7),
            (0x5800, 1),
            (0x5808, 19),
            (0x5848, 97),
            (0x584b, 1),
            (0x58c8, 10),
            (0x5a18, 1),
            (0x5a58, 3),
        ])
    );
    assert_eq!(document_2002_extensions, 356);
    assert_eq!(
        document_2002_flag_shapes,
        BTreeMap::from([
            (0x0000, 11),
            (0x3000, 57),
            (0x3008, 1),
            (0xe000, 1),
            (0xe008, 1),
            (0xe028, 1),
            (0xf000, 27),
            (0xf001, 2),
            (0xf008, 15),
            (0xf009, 203),
            (0xf00c, 1),
            (0xf00d, 1),
            (0xf020, 6),
            (0xf028, 8),
            (0xf029, 17),
            (0xf02d, 1),
            (0xf108, 1),
            (0xf109, 1),
            (0xf10b, 1),
        ])
    );
    assert_eq!(
        document_2002_line_endings,
        BTreeMap::from([(TextLineEnding::CrLf, 353), (TextLineEnding::Cr, 3)])
    );
    assert_eq!(
        document_2002_feature_shapes,
        BTreeMap::from([
            (0x0000, 80),
            (0x0001, 57),
            (0x0100, 10),
            (0x0800, 17),
            (0x0801, 54),
            (0x0900, 138),
        ])
    );
    assert_eq!(
        document_2002_default_table_styles,
        BTreeSet::from([0, 0x0fff])
    );
    assert_eq!(
        document_2002_style_filters,
        BTreeMap::from([
            (0x0000, 99),
            (0x0004, 3),
            (0x0808, 1),
            (0x1f08, 2),
            (0x2801, 2),
            (0x3001, 1),
            (0x3f01, 79),
            (0x5024, 169),
        ])
    );
    assert_eq!(document_2002_booklet_pages, BTreeMap::from([(0, 356)]));
    assert_eq!(
        document_2002_code_pages,
        BTreeMap::from([
            (0, 112),
            (936, 3),
            (950, 1),
            (1250, 37),
            (1251, 13),
            (1252, 180),
            (1253, 2),
            (1255, 1),
            (1257, 1),
            (10_000, 3),
            (u32::MAX, 3),
        ])
    );
    assert_eq!(document_2002_nonzero_unused, 0);
    assert_eq!(
        document_2002_nonzero_revision_positions,
        [282, 286, 285, 284, 286, 286, 286]
    );
    assert_eq!(
        document_2002_maximum_revision_positions,
        [i32::MAX as u32; 7]
    );
    assert_eq!(document_2002_nonzero_root_revision_ids, 286);
    assert_eq!(document_2002_root_revision_ids.len(), 263);
    assert_eq!(document_2002_root_revision_ids.first(), Some(&0));
    assert_eq!(document_2002_root_revision_ids.last(), Some(&3_745_368_673));
    assert_eq!(document_2003_extensions, 272);
    assert_eq!(
        document_2003_flag_shapes,
        BTreeMap::from([(0x0000, 40), (0x0400, 196), (0x0600, 36)])
    );
    assert_eq!(
        document_2003_protection_shapes,
        BTreeMap::from([
            (0x000a, 1),
            (0x002a, 5),
            (0x002e, 1),
            (0x0030, 3),
            (0x0032, 194),
            (0x0036, 19),
            (0x003e, 1),
            (0x00aa, 3),
            (0x00b0, 1),
            (0x00b2, 44),
        ])
    );
    assert_eq!(
        document_2003_protection_modes,
        BTreeMap::from([
            (DocumentProtectionMode::TrackedChanges, 1),
            (DocumentProtectionMode::Forms, 9),
            (DocumentProtectionMode::RangePermissions, 262),
        ])
    );
    assert_eq!(document_2003_page_widths, BTreeMap::from([(0, 272)]));
    assert_eq!(document_2003_page_heights, BTreeMap::from([(0, 272)]));
    assert_eq!(document_2003_font_percentages, BTreeMap::from([(0, 272)]));
    assert_eq!(
        document_2003_toolbar_shapes,
        BTreeMap::from([(0x00, 266), (0x01, 3), (0x02, 2), (0x03, 1)])
    );
    assert_eq!(
        document_2003_cleanup_limits,
        BTreeMap::from([
            (0, 259),
            (1, 1),
            (3, 1),
            (4, 1),
            (5, 1),
            (10, 1),
            (11, 2),
            (13, 1),
            (14, 1),
            (21, 1),
            (27, 1),
            (32, 1),
            (52, 1),
        ])
    );
    assert_eq!(document_2007_extensions, 220);
    assert_eq!(document_2007_reserved_values, BTreeMap::from([(0, 220)]));
    assert_eq!(
        document_2007_flag_shapes,
        BTreeMap::from([
            (0x0000, 1),
            (0x0001, 2),
            (0x0021, 17),
            (0x0401, 1),
            (0x0421, 199),
        ])
    );
    assert_eq!(
        document_2007_style_sort_methods,
        BTreeMap::from([
            (StyleSortMethod::Name, 4),
            (StyleSortMethod::ApplicationDefault, 216),
        ])
    );
    assert_eq!(
        document_math_flag_shapes,
        BTreeMap::from([(0x0000, 2), (0x1410, 2), (0x1c10, 214), (0x1d10, 2),])
    );
    assert_eq!(
        document_math_enum_shapes,
        BTreeMap::from([
            (
                (
                    MathBinaryOperatorBreak::Before,
                    MathBinarySubtractionBreak::MinusMinus,
                    MathJustification::ProducerCompatibilityZero,
                ),
                2,
            ),
            (
                (
                    MathBinaryOperatorBreak::Before,
                    MathBinarySubtractionBreak::MinusMinus,
                    MathJustification::CenteredAsGroup,
                ),
                218,
            ),
        ])
    );
    assert_eq!(
        document_math_fixed_constants,
        BTreeMap::from([
            (MathFixedConstants::Standard120, 217),
            (MathFixedConstants::ProducerCompatibilityZero, 3),
        ])
    );
    assert_eq!(
        document_math_font_indexes,
        BTreeMap::from([
            (0, 1),
            (3, 6),
            (4, 55),
            (5, 52),
            (6, 33),
            (7, 26),
            (8, 17),
            (9, 9),
            (10, 5),
            (11, 7),
            (12, 4),
            (13, 1),
            (14, 2),
            (18, 1),
            (34, 1),
        ])
    );
    assert_eq!(document_math_left_margins, BTreeMap::from([(0, 220)]));
    assert_eq!(document_math_right_margins, BTreeMap::from([(0, 220)]));
    assert_eq!(
        document_math_wrapped_indents,
        BTreeMap::from([(0, 2), (1440, 218)])
    );
    assert_eq!(document_2010_extensions, 158);
    assert_eq!(document_2010_compatibility_zero_contexts, 101);
    assert_eq!(document_2010_standard_contexts.len(), 57);
    assert_eq!(document_2010_standard_contexts.first(), Some(&30_159_384));
    assert_eq!(document_2010_standard_contexts.last(), Some(&2_142_460_963));
    assert_eq!(
        document_2010_reserved_values,
        BTreeMap::from([(0x0000_000b, 158)])
    );
    assert_eq!(
        document_2010_discard_image_data,
        BTreeMap::from([(false, 157), (true, 1)])
    );
    assert_eq!(
        document_2010_image_resolutions,
        BTreeMap::from([(0, 11), (220, 145), (300, 1), (32_767, 1)])
    );
    assert_eq!(
        document_2013_chart_tracking,
        BTreeMap::from([(false, 18), (true, 79)])
    );
    assert_eq!(
        auto_summary_info_shapes,
        BTreeMap::from([
            (
                (
                    false,
                    false,
                    AutoSummaryView::Highlight,
                    false,
                    AutoSummaryDesiredSize::Percentage(0),
                    0,
                    0,
                ),
                344,
            ),
            (
                (
                    false,
                    false,
                    AutoSummaryView::Highlight,
                    true,
                    AutoSummaryDesiredSize::Percentage(25),
                    100,
                    25,
                ),
                57,
            ),
            (
                (
                    false,
                    false,
                    AutoSummaryView::CreateDocument,
                    true,
                    AutoSummaryDesiredSize::Percentage(25),
                    3_655,
                    914,
                ),
                1,
            ),
            (
                (
                    true,
                    false,
                    AutoSummaryView::Highlight,
                    true,
                    AutoSummaryDesiredSize::Percentage(25),
                    100,
                    25,
                ),
                1,
            ),
        ])
    );
    assert_eq!(auto_summary_range_tables, 0);
    assert_eq!(auto_summary_ranges, 0);
    assert!(auto_summary_range_count_shapes.is_empty());
    assert!(auto_summary_priority_shapes.is_empty());
    assert_eq!(style_sheets, 403);
    assert_eq!(
        style_sheet_info_shapes,
        BTreeMap::from([
            ((18, 10), 88),
            ((20, 10), 27),
            ((20, 18), 16),
            ((646, 18), 51),
            ((1062, 18), 1),
            ((1114, 18), 1),
            ((1118, 18), 46),
            ((1122, 18), 3),
            ((1126, 18), 4),
            ((1130, 18), 62),
            ((1134, 18), 5),
            ((1154, 18), 1),
            ((1166, 18), 1),
            ((1534, 18), 10),
            ((1538, 18), 2),
            ((1546, 18), 19),
            ((1550, 18), 9),
            ((1554, 18), 9),
            ((1558, 18), 4),
            ((1562, 18), 8),
            ((1566, 18), 29),
            ((1570, 18), 7),
        ])
    );
    assert_eq!(styles, 12_514);
    assert_eq!(empty_styles, 3_986);
    assert_eq!(style_definition_bytes, 612_108);
    assert_eq!(style_name_units, 101_716);
    assert_eq!(revision_marked_styles, 0);
    assert_eq!(style_upx_prls, 43_260);
    assert_eq!(
        field_tables,
        BTreeMap::from([
            (FieldDocumentPart::Main, 94),
            (FieldDocumentPart::Header, 73),
            (FieldDocumentPart::Footnote, 2),
            (FieldDocumentPart::Comment, 1),
            (FieldDocumentPart::Endnote, 1),
            (FieldDocumentPart::Textbox, 11),
            (FieldDocumentPart::HeaderTextbox, 6),
        ])
    );
    assert_eq!(field_records, 4_880);
    assert_eq!(
        field_character_counts,
        BTreeMap::from([
            ((FieldDocumentPart::Main, 0x13), 1_309),
            ((FieldDocumentPart::Main, 0x14), 1_233),
            ((FieldDocumentPart::Main, 0x15), 1_309),
            ((FieldDocumentPart::Header, 0x13), 183),
            ((FieldDocumentPart::Header, 0x14), 162),
            ((FieldDocumentPart::Header, 0x15), 183),
            ((FieldDocumentPart::Footnote, 0x13), 5),
            ((FieldDocumentPart::Footnote, 0x14), 5),
            ((FieldDocumentPart::Footnote, 0x15), 5),
            ((FieldDocumentPart::Comment, 0x13), 1),
            ((FieldDocumentPart::Comment, 0x14), 1),
            ((FieldDocumentPart::Comment, 0x15), 1),
            ((FieldDocumentPart::Endnote, 0x13), 1),
            ((FieldDocumentPart::Endnote, 0x14), 1),
            ((FieldDocumentPart::Endnote, 0x15), 1),
            ((FieldDocumentPart::Textbox, 0x13), 153),
            ((FieldDocumentPart::Textbox, 0x14), 153),
            ((FieldDocumentPart::Textbox, 0x15), 153),
            ((FieldDocumentPart::HeaderTextbox, 0x13), 7),
            ((FieldDocumentPart::HeaderTextbox, 0x14), 7),
            ((FieldDocumentPart::HeaderTextbox, 0x15), 7),
        ])
    );
    assert_eq!(
        field_reserved_counts,
        BTreeMap::from([
            (0, 3_194),
            (1, 22),
            (2, 22),
            (3, 129),
            (4, 1_437),
            (5, 9),
            (6, 10),
            (7, 57)
        ])
    );
    assert_eq!(
        field_type_counts,
        BTreeMap::from([
            (2, 1),
            (3, 11),
            (7, 3),
            (10, 10),
            (12, 4),
            (13, 11),
            (15, 1),
            (16, 2),
            (17, 5),
            (20, 1),
            (21, 4),
            (22, 4),
            (23, 2),
            (25, 1),
            (26, 11),
            (29, 10),
            (31, 6),
            (32, 5),
            (33, 139),
            (35, 4),
            (37, 396),
            (39, 7),
            (51, 12),
            (56, 1),
            (58, 137),
            (59, 1),
            (60, 1),
            (64, 1),
            (66, 1),
            (67, 21),
            (69, 1),
            (70, 129),
            (71, 68),
            (83, 8),
            (85, 11),
            (87, 124),
            (88, 349),
            (95, 156),
        ])
    );
    assert_eq!(bookmark_sets, 77);
    assert_eq!(bookmarks_count, 1_932);
    assert_eq!(bookmark_name_units, 23_617);
    assert_eq!(hidden_bookmarks, 1_717);
    assert_eq!(column_bookmarks, 0);
    assert_eq!(header_tables, 209);
    assert_eq!(header_boundaries, 3_295);
    assert_eq!(missing_header_boundaries, 4);
    assert_eq!(footnote_sets, 14);
    assert_eq!(footnote_references, 73);
    assert_eq!(footnote_custom_references, 0);
    assert_eq!(endnote_sets, 7);
    assert_eq!(endnote_references, 10);
    assert_eq!(endnote_custom_references, 0);
    assert_eq!(annotation_sets, 13);
    assert_eq!(annotation_references, 86);
    assert_eq!(annotation_initial_units, 164);
    assert_eq!(annotation_empty_range_tags, 13);
    assert_eq!(annotation_unused_words, BTreeMap::from([((0, 0), 86)]));
    assert_eq!(annotation_extended_tables, 13);
    assert_eq!(annotation_extended_records, 86);
    assert_eq!(annotation_extended_count_mismatches, 0);
    assert_eq!(annotation_extended_depths, BTreeMap::from([(0, 86)]));
    assert_eq!(
        annotation_extended_parent_offsets,
        BTreeMap::from([(0, 86)])
    );
    assert_eq!(annotation_extended_ink, 0);
    assert_eq!(annotation_extended_ows, 0);
    assert_eq!(annotation_extended_nonzero_padding1, 0);
    assert_eq!(annotation_extended_nonzero_padding2, 0);
    assert_eq!(annotation_extended_zero_dates, 9);
    assert_eq!(user_input_method_tables, 2);
    assert_eq!(user_input_methods, 3);
    assert_eq!(user_input_method_guids, 4);
    assert_eq!(user_input_method_service_bytes, 48);
    assert_eq!(user_input_method_empty_service_data, 0);
    assert_eq!(user_input_method_duplicate_positions, 0);
    assert_eq!(user_input_method_descending_positions, 0);
    assert_eq!(user_input_method_negative_character_counts, 0);
    assert_eq!(user_input_method_nonzero_private_data, 3);
    assert_eq!(
        user_input_method_reference_pairs,
        BTreeMap::from([((0, 1), 3)])
    );
    assert_eq!(
        user_input_method_character_counts,
        BTreeMap::from([(1, 1), (5, 1), (11, 1)])
    );
    assert_eq!(user_input_method_service_sizes, BTreeMap::from([(16, 3)]));
    assert_eq!(user_input_method_private_values, BTreeMap::from([(3, 3)]));
    assert_eq!(
        user_input_method_guid_values,
        BTreeSet::from([
            [
                0x20, 0xd5, 0xe2, 0xf1, 0x69, 0x09, 0xd3, 0x11, 0x8d, 0xf0, 0x00, 0x10, 0x5a, 0x27,
                0x99, 0xb5,
            ],
            [
                0x60, 0xbc, 0xa4, 0xb6, 0x49, 0x07, 0xd3, 0x11, 0x8d, 0xef, 0x00, 0x10, 0x5a, 0x27,
                0x99, 0xb5,
            ],
        ])
    );
    assert_eq!(mso_envelope_tables, 1);
    assert_eq!(mso_envelope_typed, 1);
    assert_eq!(mso_envelope_out_of_scope, 0);
    assert_eq!(
        mso_envelope_versions,
        BTreeMap::from([(MsoEnvelopeVersion::Unicode8, 1)])
    );
    assert_eq!(
        mso_envelope_shapes,
        BTreeMap::from([(
            (
                EnvelopeFlagStatus::NotFlagged,
                EnvelopeSensitivity::Normal,
                EnvelopeImportance::Normal,
                0,
                false,
                false,
                false,
            ),
            1
        )])
    );
    assert_eq!(mso_envelope_subject_units, 19);
    assert_eq!(mso_envelope_recipients, 0);
    assert_eq!(mso_envelope_recipient_properties, 0);
    assert_eq!(mso_envelope_property_types, BTreeMap::new());
    assert_eq!(mso_envelope_attachments, 1);
    assert_eq!(mso_envelope_attachment_bytes, 345);
    assert_eq!(mso_envelope_attachment_name_units, 16);
    assert_eq!(mso_envelope_attachment_methods, BTreeMap::from([(1, 1)]));
    assert_eq!(mso_envelope_intro_units, 0);
    assert_eq!(printer_driver_info_tables, 8);
    assert_eq!(printer_driver_info_total_bytes, 523);
    assert_eq!(printer_driver_info_empty_fields, 0);
    assert_eq!(
        printer_driver_info_length_shapes,
        BTreeMap::from([
            ((11, 16, 8, 11), 1),
            ((14, 5, 8, 14), 1),
            ((17, 5, 8, 26), 1),
            ((17, 33, 8, 25), 1),
            ((20, 5, 8, 26), 1),
            ((21, 5, 5, 21), 1),
            ((27, 5, 8, 34), 1),
            ((39, 5, 8, 28), 1),
        ])
    );
    assert_eq!(printer_driver_names.len(), 8);
    assert_eq!(printer_port_names.len(), 7);
    assert_eq!(printer_driver_file_names.len(), 4);
    assert_eq!(printer_product_names.len(), 8);
    assert_eq!(ole_control_info_tables, 8);
    assert_eq!(ole_control_infos, 141);
    assert_eq!(ole_control_cookie_index_mismatches, 17);
    assert_eq!(ole_control_duplicate_cookies, 14);
    assert_eq!(ole_control_field_reference_mismatches, 0);
    assert_eq!(ole_control_unlinked_fields, 17);
    assert_eq!(ole_control_compatibility_document_parts, 2);
    assert_eq!(
        ole_control_document_parts,
        BTreeMap::from([
            (OleControlDocumentPart::Main, 23),
            (OleControlDocumentPart::Textbox, 116),
            (OleControlDocumentPart::Compatibility(0), 1),
            (OleControlDocumentPart::Compatibility(13_182), 1),
        ])
    );
    assert_eq!(ole_control_nonzero_accelerator_handles, 1);
    assert_eq!(ole_control_nonzero_accelerator_counts, 1);
    assert_eq!(ole_control_failed_load, 0);
    assert_eq!(ole_control_corrupt, 0);
    assert_eq!(
        ole_control_behavior_flags,
        BTreeMap::from([((false, false, false, false, false), 141)])
    );
    assert_eq!(ole_control_nonzero_reserved1, 0);
    assert_eq!(ole_control_nonzero_reserved2, 8);
    assert_eq!(ole_object_descriptors, 255);
    assert_eq!(ole_object_control_descriptors, 124);
    assert_eq!(ole_object_control_streams, 1);
    assert_eq!(ole_object_control_missing_payloads, 0);
    assert_eq!(
        ole_object_descriptor_shapes,
        BTreeMap::from([
            ((0, 3, None, 4), 38),
            ((0, 3, Some(0), 6), 1),
            ((0, 3, Some(4), 6), 41),
            ((0, 3, Some(13), 6), 24),
            ((16, 3, None, 4), 1),
            ((64, 3, Some(1), 6), 1),
            ((128, 3, None, 4), 1),
            ((512, 3, None, 4), 2),
            ((512, 3, Some(1), 6), 11),
            ((512, 3, Some(13), 6), 11),
            ((4608, 3, Some(4), 6), 123),
            ((12800, 3, Some(4), 6), 1),
        ])
    );
    assert_eq!(
        ole_object_control_storage_shapes,
        BTreeMap::from([
            (
                (
                    false,
                    vec![
                        "stream:\u{1}CompObj".into(),
                        "stream:\u{1}Ole".into(),
                        "stream:\u{3}OCXNAME".into(),
                        "stream:\u{3}PRINT".into(),
                        "stream:contents".into()
                    ]
                ),
                116
            ),
            (
                (
                    false,
                    vec![
                        "stream:\u{1}CompObj".into(),
                        "stream:\u{3}OCXNAME".into(),
                        "stream:\u{3}PRINT".into(),
                        "stream:contents".into()
                    ]
                ),
                6
            ),
            (
                (
                    false,
                    vec![
                        "stream:\u{1}CompObj".into(),
                        "stream:\u{3}OCXNAME".into(),
                        "stream:contents".into()
                    ]
                ),
                1
            ),
            (
                (
                    true,
                    vec![
                        "stream:\u{1}CompObj".into(),
                        "stream:\u{3}OCXDATA".into(),
                        "stream:\u{3}OCXNAME".into()
                    ]
                ),
                1
            ),
        ])
    );
    assert_eq!(
        ole_object_control_classes,
        BTreeMap::from([
            (
                ("8bd21d10-ec42-11ce-9e0d-00aa006002f3".into(), false),
                (116, BTreeSet::from([76]))
            ),
            (
                ("8bd21d40-ec42-11ce-9e0d-00aa006002f3".into(), false),
                (4, BTreeSet::from([84, 96, 100]))
            ),
            (
                ("8bd21d50-ec42-11ce-9e0d-00aa006002f3".into(), false),
                (2, BTreeSet::from([100, 104]))
            ),
            (
                ("ae24fdae-03c6-11d1-8b76-0080c744f389".into(), true),
                (1, BTreeSet::from([146]))
            ),
            (
                ("d7053240-ce69-11cd-a777-00dd01143c57".into(), false),
                (1, BTreeSet::from([68]))
            ),
        ])
    );
    assert_eq!(morph_data_controls, 122);
    assert_eq!(
        morph_data_text_props_masks,
        BTreeMap::from([(0x35, 6), (0x37, 116)])
    );
    assert_eq!(morph_data_low_word_compatibility_strings, 1);
    assert_eq!(command_button_controls, 1);
    assert_eq!(
        command_button_shapes,
        BTreeMap::from([((0x28, 0x75, 68), 1)])
    );
    assert_eq!(single_stream_ole_controls, 1);
    assert_eq!(
        single_stream_ole_control_shapes,
        BTreeMap::from([(
            ("ae24fdae-03c6-11d1-8b76-0080c744f389".into(), 130, true),
            1
        )])
    );
    assert_eq!(
        morph_data_shapes,
        BTreeMap::from([
            (
                (
                    "8bd21d10-ec42-11ce-9e0d-00aa006002f3".into(),
                    0x8600_0117,
                    32,
                    32
                ),
                116
            ),
            (
                (
                    "8bd21d40-ec42-11ce-9e0d-00aa006002f3".into(),
                    0x80c0_0146,
                    44,
                    28
                ),
                2
            ),
            (
                (
                    "8bd21d40-ec42-11ce-9e0d-00aa006002f3".into(),
                    0x80c0_0146,
                    56,
                    28
                ),
                1
            ),
            (
                (
                    "8bd21d40-ec42-11ce-9e0d-00aa006002f3".into(),
                    0x80c0_0146,
                    60,
                    28
                ),
                1
            ),
            (
                (
                    "8bd21d50-ec42-11ce-9e0d-00aa006002f3".into(),
                    0x80c0_0146,
                    60,
                    28
                ),
                1
            ),
            (
                (
                    "8bd21d50-ec42-11ce-9e0d-00aa006002f3".into(),
                    0x80c0_0146,
                    64,
                    28
                ),
                1
            ),
        ])
    );
    assert_eq!(annotation_owner_sets, 13);
    assert_eq!(annotation_owners, 16);
    assert_eq!(annotation_owner_name_units, 142);
    assert_eq!(annotation_bookmark_sets, 9);
    assert_eq!(annotation_bookmarks, 73);
    assert_eq!(
        textbox_story_sets,
        BTreeMap::from([
            (TextboxDocumentPart::Main, 49),
            (TextboxDocumentPart::Header, 14),
        ])
    );
    assert_eq!(textbox_stories, 1_036);
    assert_eq!(reusable_textbox_stories, 261);
    assert_eq!(
        textbox_break_sets,
        BTreeMap::from([
            (TextboxDocumentPart::Main, 49),
            (TextboxDocumentPart::Header, 14),
        ])
    );
    assert_eq!(textbox_breaks, 1_036);
    assert_eq!(textbox_overflows, 5);
    assert_eq!(
        shape_anchor_sets,
        BTreeMap::from([
            (TextboxDocumentPart::Main, 90),
            (TextboxDocumentPart::Header, 28),
        ])
    );
    assert_eq!(shape_anchors, 429);
    assert_eq!(below_text_shapes, 42);
    assert_eq!(locked_shape_anchors, 168);
    assert_eq!(textbox_stories_without_anchor, 577);
    assert_eq!(office_art_contents, 301);
    assert_eq!(
        office_art_drawings,
        BTreeMap::from([
            (TextboxDocumentPart::Main, 301),
            (TextboxDocumentPart::Header, 71),
        ])
    );
    assert_eq!(office_art_records, 18_883);
    assert_eq!(office_art_atom_bytes, 66);
    assert_eq!(office_art_atom_shapes, BTreeMap::from([((0xf004, 66), 1)]));
    assert_eq!(office_art_graph_audit.graphs, 301);
    assert_eq!(office_art_graph_audit.partial_graphs, 1);
    assert_eq!(office_art_graph_audit.typed_graphs, 300);
    assert_eq!(office_art_graph_audit.strict_graphs, 10);
    assert_eq!(office_art_graph_audit.compatibility_graphs, 290);
    assert_eq!(office_art_graph_audit.missing_or_multiple_dgg, 0);
    assert_eq!(office_art_graph_audit.missing_or_multiple_fdg, 0);
    assert_eq!(office_art_graph_audit.duplicate_drawing_ids, 0);
    assert_eq!(office_art_graph_audit.duplicate_shape_ids, 0);
    assert_eq!(office_art_graph_audit.shape_cluster_zero, 0);
    assert_eq!(office_art_graph_audit.shape_cluster_missing, 0);
    assert_eq!(office_art_graph_audit.shape_cluster_drawing_mismatches, 0);
    assert_eq!(office_art_graph_audit.shapes, 2_893);
    assert_eq!(office_art_graph_audit.patriarch_shapes, 371);
    assert_eq!(office_art_graph_audit.deleted_shapes, 0);
    assert_eq!(office_art_graph_audit.blip_stores, 57);
    assert_eq!(office_art_graph_audit.blip_entries, 355);
    assert_eq!(office_art_graph_audit.blip_references, 373);
    assert_eq!(
        office_art_graph_audit.blip_reference_count_relations,
        BTreeMap::from([("EqualToActual".to_owned(), 355)])
    );
    assert_eq!(office_art_graph_audit.blip_store_entry_count_mismatches, 2);
    assert_eq!(
        office_art_graph_audit.blip_store_entry_count_mismatch_shapes,
        BTreeMap::from([((1, 2), 1), ((1, 3), 1)])
    );
    assert_eq!(office_art_graph_audit.blip_references_out_of_range, 0);
    assert_eq!(office_art_graph_audit.empty_blip_store_slots_referenced, 0);
    assert_eq!(
        office_art_graph_audit.fdg_shape_count_deltas,
        BTreeMap::from([(-1, 279), (0, 92)])
    );
    assert_eq!(
        office_art_graph_audit.dgg_saved_drawing_deltas,
        BTreeMap::from([(0, 300)])
    );
    assert!(
        !office_art_graph_audit
            .fdg_current_shape_relations
            .contains_key("below")
    );
    assert!(
        !office_art_graph_audit
            .dgg_max_shape_relations
            .contains_key("below")
    );
    assert!(
        !office_art_graph_audit
            .cluster_cursor_relations
            .contains_key("below")
    );
    eprintln!("DOC OfficeArt drawing graph: {office_art_graph_audit:#?}");
    assert_eq!(word_client_anchors, 423);
    assert_eq!(word_client_data, 2_521);
    assert_eq!(word_client_textboxes, 623);
    assert_eq!(word_client_anchor_invalid_indexes, 0);
    assert_eq!(word_client_textbox_invalid_indexes, 0);
    assert_eq!(list_definition_sets, 164);
    assert_eq!(list_definitions, 2_278);
    assert_eq!(simple_list_definitions, 1_132);
    assert_eq!(list_levels, 11_446);
    assert_eq!(list_level_paragraph_prls, 53_433);
    assert_eq!(list_level_character_prls, 28_258);
    assert_eq!(list_level_text_units, 27_418);
    assert_eq!(list_level_bytes, 759_880);
    assert_eq!(list_levels_in_declared_length, 0);
    assert_eq!(
        list_level_incomplete_tails,
        BTreeMap::from([(("character", 4), 1), (("character", 10), 1),])
    );
    assert_eq!(list_name_tables, 164);
    assert_eq!(list_name_entries, 3_068);
    assert_eq!(nonempty_list_names, 80);
    assert_eq!(list_name_units, 675);
    assert_eq!(maximum_list_name_length, 22);
    assert_eq!(
        list_name_count_shapes,
        BTreeMap::from([
            (1, 50),
            (2, 18),
            (3, 13),
            (4, 10),
            (5, 2),
            (6, 2),
            (8, 6),
            (9, 3),
            (10, 5),
            (11, 5),
            (12, 4),
            (13, 2),
            (14, 4),
            (15, 5),
            (18, 5),
            (19, 1),
            (20, 2),
            (21, 2),
            (22, 1),
            (23, 2),
            (25, 1),
            (29, 1),
            (31, 1),
            (32, 1),
            (37, 1),
            (38, 1),
            (39, 1),
            (43, 1),
            (46, 1),
            (47, 1),
            (48, 1),
            (50, 2),
            (61, 1),
            (78, 2),
            (79, 1),
            (87, 1),
            (111, 1),
            (151, 1),
            (308, 1),
            (769, 1),
        ])
    );
    assert_eq!(
        list_name_definition_count_differences,
        BTreeMap::from([(0, 159), (3, 1), (6, 1), (12, 1), (14, 1), (755, 1)])
    );
    assert_eq!(list_level_to_override_gaps, BTreeMap::from([(0, 164)]));
    assert_eq!(list_override_sets, 165);
    assert_eq!(list_overrides, 2_626);
    assert_eq!(list_override_levels, 481);
    assert_eq!(formatted_list_override_levels, 34);
    assert_eq!(list_override_level_prls, 201);
    assert_eq!(list_override_text_units, 42);
    assert_eq!(list_override_missing_definitions, 0);
    assert_eq!(office_art_partial_trees, 1);
    assert_eq!(shape_anchors_without_fsp, 0);
    assert_eq!(textbox_stories_without_fsp, 0);
    assert_eq!(style_upx_padding, BTreeMap::from([(0x00, 3_486)]));
    assert_eq!(
        style_upx_index_mismatches,
        BTreeMap::from([((0x000c, 0x0000), 32)])
    );
    assert_eq!(
        style_upx_unknown_sprms,
        BTreeSet::from([0x2404, 0x486b, 0x6654, 0xc63e])
    );
    assert_eq!(style_upx_raw_variable_operands, 0);
    assert!(
        style_upx_raw_variable_frequencies.is_empty(),
        "{style_upx_raw_variable_frequencies:#x?}"
    );
    assert!(
        style_upx_raw_variable_shapes.is_empty(),
        "{style_upx_raw_variable_shapes:#x?}"
    );
    assert_eq!(
        style_upx_static_variable_operands,
        BTreeMap::from([
            ("auto-numbered-list-data", 31),
            ("border", 956),
            ("conditional-formatting", 389),
            ("paragraph-change-tabs", 181),
            ("paragraph-change-tabs-papx", 816),
            ("shading", 291),
            ("table-borders", 107),
            ("table-cell-spacing", 586),
        ])
    );
    assert_eq!(
        style_kind_counts,
        BTreeMap::from([
            (StyleKind::Paragraph, 4_960),
            (StyleKind::Character, 2_874),
            (StyleKind::Table, 402),
            (StyleKind::Numbering, 292),
        ])
    );
    assert_eq!(
        style_cupx_shapes,
        BTreeMap::from([
            ((StyleKind::Paragraph, 2, false), 4_960),
            ((StyleKind::Character, 1, false), 2_874),
            ((StyleKind::Table, 3, false), 402),
            ((StyleKind::Numbering, 1, false), 292),
        ])
    );
    assert_eq!(latent_style_entries, 77_350);
    assert_eq!(standard_style_prls, 1_546);
    assert!(style_alignment_padding.is_empty());
    assert_eq!(section_tables, 403);
    assert_eq!(sections, 484);
    assert_eq!(default_sections, 0);
    assert_eq!(sepx_count, 484);
    assert_eq!(sepx_prls, 5_969);
    assert_eq!(sepx_unknown_sprms, BTreeSet::from([0x4231, 0xd238]));
    assert_eq!(
        sepx_unknown_fixed_shapes,
        BTreeMap::from([((0x4231, 360), 1), ((0xd238, u32::MAX), 2)])
    );
    assert_eq!(sepx_raw_variable_operands, 2);
    assert_eq!(sepx_raw_variable_frequencies, BTreeMap::from([(0xd238, 2)]));
    assert_eq!(
        sepx_raw_variable_shapes,
        BTreeMap::from([((0xd238, 36), 2)])
    );
    assert_eq!(
        sepx_static_variable_operands,
        BTreeMap::from([
            ("border", 20),
            ("outline-list-data", 3),
            ("section-header-footer-flags", 9),
        ])
    );
    assert_eq!(outline_list_restart_values, BTreeMap::from([(0, 3)]));
    assert_eq!(outline_list_reserved_shapes, BTreeSet::from([[0, 0, 0]]));
    assert_eq!(outline_list_nonzero_text_units, 37);
    assert_eq!(
        section_header_footer_flag_shapes,
        BTreeMap::from([
            (0x02, 1),
            (0x08, 2),
            (0x0a, 1),
            (0x20, 1),
            (0x38, 1),
            (0x3a, 2),
            (0x3f, 1),
        ])
    );
    assert!(sepx_trailing_bytes.is_empty());
    assert_eq!(table0, 4);
    assert_eq!(table1, 399);
    assert_eq!(versions.get(&0x00c1), Some(&19));
    assert_eq!(versions.get(&0x00c2), Some(&1));
    assert_eq!(versions.get(&0x00c3), Some(&1));
    assert_eq!(versions.get(&0x00d9), Some(&37));
    assert_eq!(versions.get(&0x0101), Some(&73));
    assert_eq!(versions.get(&0x010c), Some(&55));
    assert_eq!(versions.get(&0x0112), Some(&217));
    assert_eq!(
        fc_lcb_shapes,
        BTreeMap::from([
            ((0x00c1, 0x005d), 19),
            ((0x00c2, 0x005d), 1),
            ((0x00c3, 0x006c), 1),
            ((0x00d9, 0x006c), 37),
            ((0x0101, 0x0088), 73),
            ((0x010c, 0x00a4), 53),
            ((0x010c, 0x00b7), 2),
            ((0x0112, 0x00b7), 217),
        ])
    );
    assert_eq!(
        csw_new_shapes,
        BTreeMap::from([
            ((0x00c1, 0), 19),
            ((0x00c2, 0), 1),
            ((0x00c3, 4), 1),
            ((0x00d9, 2), 37),
            ((0x0101, 0), 54),
            ((0x0101, 2), 18),
            ((0x0101, 4), 1),
            ((0x010c, 2), 53),
            ((0x010c, 7), 2),
            ((0x0112, 5), 217),
        ])
    );
    assert_eq!(chpx_bte_count, 403);
    assert_eq!(chpx_pages, 1_317);
    assert_eq!(chpx_runs, 44_492);
    assert_eq!(chpx_default_runs, 2_325);
    assert_eq!(chpx_prls, 191_422);
    assert_eq!(chpx_sprm_frequencies.len(), 71);
    assert_eq!(chpx_unknown_sprms, BTreeSet::from([0x0000, 0x2a03]));
    assert_eq!(chpx_raw_variable_operands, 0);
    assert!(chpx_raw_variable_frequencies.is_empty());
    assert_eq!(
        chpx_static_variable_operands,
        BTreeMap::from([
            ("border", 20),
            ("property-revision-mark", 425),
            ("shading", 61),
        ])
    );
    assert_eq!(chpx_unused_bytes, 149_083);
    assert_eq!(papx_bte_count, 403);
    assert_eq!(papx_pages, 2_979);
    assert_eq!(papx_runs, 31_008);
    assert_eq!(papx_default_runs, 16);
    assert_eq!(papx_prls, 133_928);
    assert_eq!(papx_sprm_frequencies.len(), 116);
    assert_eq!(papx_unknown_sprms, BTreeSet::from([0x0000]));
    assert_eq!(papx_raw_variable_operands, 0);
    assert!(papx_raw_variable_frequencies.is_empty());
    assert_eq!(
        papx_static_variable_operands,
        BTreeMap::from([
            ("border", 333),
            ("paragraph-change-tabs-papx", 6_279),
            ("paragraph-number-revision", 231),
            ("paragraph-table-style-info", 254),
            ("property-revision-mark", 265),
            ("shading", 238),
            ("table-border-colors", 5_472),
            ("table-border", 239),
            ("table-borders", 861),
            ("table-borders-80", 1_094),
            ("table-cell-hide-mark", 5),
            ("table-cell-spacing", 5_098),
            ("table-definition", 2_394),
            ("table-shading", 831),
            ("table-shading-80", 329),
        ])
    );
    assert_eq!(papx_short_lengths, 11_152);
    assert_eq!(papx_extended_lengths, 19_840);
    assert_eq!(
        papx_trailing_bytes,
        BTreeMap::from([(0x00, 1), (0x09, 1), (0x12, 1)])
    );
    assert_eq!(papx_unused_bytes, 293_611);
    assert_eq!(clx_count, 403);
    assert_eq!(property_runs, 21);
    assert_eq!(pieces, 1_478);
    assert_eq!(compressed_pieces, 336);
    assert_eq!(simple_property_modifiers, 1_034);
    assert_eq!(complex_property_modifiers, 444);
    assert_eq!(prl_count, 55);
    assert_eq!(
        sprm_opcodes,
        BTreeMap::from([
            (0x0835, 6),
            (0x2407, 1),
            (0x2443, 4),
            (0x260a, 3),
            (0x2a3e, 2),
            (0x2a42, 2),
            (0x4600, 2),
            (0x460b, 3),
            (0x484e, 4),
            (0x4a30, 1),
            (0x4a43, 13),
            (0x664a, 2),
            (0xc615, 3),
            (0xc645, 4),
            (0xca47, 1),
            (0xca62, 4),
        ])
    );
    assert_eq!(sprm_groups, BTreeMap::from([(1, 22), (2, 33)]));
    assert_eq!(
        sprm_operand_shapes,
        BTreeMap::from([
            ("byte", 12),
            ("character-display-field-revision", 4),
            ("character-majority", 1),
            ("dword", 2),
            ("paragraph-change-tabs", 3),
            ("paragraph-number-revision", 4),
            ("toggle", 6),
            ("word", 23),
        ])
    );
    assert_eq!(variable_operand_bytes, 694);
    assert_eq!(text_characters, 1_336_946);
    assert_eq!(compressed_text_bytes, 1_119_724);
    assert_eq!(utf16_text_units, 217_222);
    assert!(
        unknown_sprm_kinds.is_empty(),
        "CLX PRCs contain untyped known SPRMs: {unknown_sprm_kinds:#x?}"
    );
    assert_eq!(legacy.get(&0x0000), Some(&2));
    assert_eq!(legacy.get(&0xa5dc), Some(&20));
    assert_eq!(legacy.get(&0xa697), Some(&1));
    assert_eq!(legacy.get(&0xa698), Some(&1));
    assert_eq!(legacy.get(&0xa699), Some(&4));
    eprintln!(
        "checked {checked} Word 97+ FIBs: versions {versions:#x?}; Fc/Lcb shapes {fc_lcb_shapes:#x?}; cswNew shapes {csw_new_shapes:#x?}; {table0} select 0Table/{table1} select 1Table; CHPX {chpx_bte_count} BTE/{chpx_pages} pages/{chpx_runs} runs ({chpx_default_runs} default)/{chpx_prls} PRLs/{} unknown SPRM types/{chpx_raw_variable_operands} raw variable operands/{chpx_unused_bytes} unused bytes; PAPX {papx_bte_count} BTE/{papx_pages} pages/{papx_runs} runs ({papx_default_runs} default)/{papx_prls} PRLs/{} unknown SPRM types/{papx_raw_variable_operands} raw variable operands/{papx_short_lengths} short + {papx_extended_lengths} extended lengths/trailing {papx_trailing_bytes:#x?}/{papx_unused_bytes} unused bytes; CLX {clx_count}/{property_runs} property runs/{prl_count} PRLs opcodes {sprm_opcodes:#x?}/groups {sprm_groups:?}/operands {sprm_operand_shapes:?}/{variable_operand_bytes} variable bytes/{pieces} pieces ({compressed_pieces} compressed, {simple_property_modifiers} simple PRM/{complex_property_modifiers} complex PRM), text {text_characters} characters/{compressed_text_bytes} compressed bytes/{utf16_text_units} UTF-16 units; exclusions {encrypted_exclusions} encrypted/{invalid_exclusions} invalid; legacy identifiers {legacy:#x?}",
        chpx_unknown_sprms.len(),
        papx_unknown_sprms.len()
    );
}

#[test]
#[ignore = "DOC stream-name casing corpus round-trip runs explicitly"]
fn word_document_stream_name_lookup_is_case_insensitive() {
    let root = olecfsdk_corpus_test_support::corpus_root().join("Apache-POI/test-data/document");
    for (file_name, physical_stream_name) in [
        ("47950_upper.doc", "/WORDDOCUMENT"),
        ("47950_lower.doc", "/worddocument"),
    ] {
        let path = root.join(file_name);
        let bytes = corpus_bytes(&path).unwrap_or_else(|error| panic!("{file_name}: {error}"));
        let compound = CompoundFile::from_bytes(&bytes)
            .unwrap_or_else(|error| panic!("{file_name}: open CFB: {error}"));
        let word_document = compound
            .entry("/WordDocument")
            .unwrap_or_else(|| panic!("{file_name}: case-insensitive lookup failed"));
        assert_eq!(word_document.path, Path::new(physical_stream_name));

        let file = DocFile::from_compound_file_compatible(compound.clone())
            .unwrap_or_else(|error| panic!("{file_name}: open typed DOC tree: {error}"))
            .value;
        let rebuilt = file
            .to_compound_file()
            .unwrap_or_else(|error| panic!("{file_name}: rebuild typed DOC tree: {error}"));
        assert!(
            compound.logical_eq(&rebuilt),
            "{file_name}: CFB graph changed"
        );
        assert_eq!(
            rebuilt
                .entry("/WordDocument")
                .expect("case-insensitive lookup after rebuild")
                .path,
            Path::new(physical_stream_name),
            "{file_name}: physical stream-name casing changed"
        );
    }
}

fn static_variable_shape(operand: &SprmOperand) -> Option<&'static str> {
    match operand {
        SprmOperand::ParagraphChangeTabs(_) => Some("paragraph-change-tabs"),
        SprmOperand::ParagraphChangeTabsPapx(_) => Some("paragraph-change-tabs-papx"),
        SprmOperand::Shading(_) => Some("shading"),
        SprmOperand::Border(_) => Some("border"),
        SprmOperand::PropertyRevisionMark(_) => Some("property-revision-mark"),
        SprmOperand::CharacterFitText(_) => Some("character-fit-text"),
        SprmOperand::TableCellSpacing(_) => Some("table-cell-spacing"),
        SprmOperand::TableBorderColors(_) => Some("table-border-colors"),
        SprmOperand::TableShading80(_) => Some("table-shading-80"),
        SprmOperand::TableShading(_) => Some("table-shading"),
        SprmOperand::TableCellHideMark(_) => Some("table-cell-hide-mark"),
        SprmOperand::TableCellWidth(_) => Some("table-cell-width"),
        SprmOperand::ParagraphTableStyleInfo(_) => Some("paragraph-table-style-info"),
        SprmOperand::TableBorders(_) => Some("table-borders"),
        SprmOperand::TableBorders80(_) => Some("table-borders-80"),
        SprmOperand::TableBorder(_) => Some("table-border"),
        SprmOperand::TableBorder80(_) => Some("table-border-80"),
        SprmOperand::TableDefinition(_) => Some("table-definition"),
        SprmOperand::ParagraphNumberRevisionMark(_) => Some("paragraph-number-revision"),
        SprmOperand::CharacterMajority(_) => Some("character-majority"),
        SprmOperand::CharacterDisplayFieldRevisionMark(_) => {
            Some("character-display-field-revision")
        }
        SprmOperand::ConditionalFormatting(_) => Some("conditional-formatting"),
        SprmOperand::AutoNumberedListData(_) => Some("auto-numbered-list-data"),
        SprmOperand::OutlineListData(_) => Some("outline-list-data"),
        SprmOperand::SectionHeaderFooterFlags(_) => Some("section-header-footer-flags"),
        _ => None,
    }
}

fn excluded_files(corpus: &Path) -> BTreeMap<PathBuf, ExpectationMode> {
    excluded_files_for_test(corpus, "doc_fib_roundtrip")
}

fn excluded_files_for_test(corpus: &Path, test: &str) -> BTreeMap<PathBuf, ExpectationMode> {
    let mut exclusions = BTreeMap::new();
    for source in ["Apache-POI", "LibreOffice"] {
        let root = corpus.join(source);
        let manifest = read_manifest(&root.join("manifest.toml"))
            .unwrap_or_else(|error| panic!("read {source} manifest: {error}"));
        for expectation in manifest.expectation {
            if expectation.test == test
                && matches!(
                    expectation.mode,
                    ExpectationMode::Invalid | ExpectationMode::RequiresPassword
                )
            {
                exclusions.insert(root.join(expectation.file), expectation.mode);
            }
        }
    }
    exclusions
}

fn bounded_slice<'a>(
    bytes: &'a [u8],
    offset: u32,
    length: u32,
    name: &str,
) -> Result<&'a [u8], String> {
    let start = usize::try_from(offset).map_err(|_| format!("{name} offset exceeds usize"))?;
    let length = usize::try_from(length).map_err(|_| format!("{name} length exceeds usize"))?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| format!("{name} bounds overflow"))?;
    bytes
        .get(start..end)
        .ok_or_else(|| format!("{name} extends beyond its stream"))
}

fn collect(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
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

fn is_word_document_case_fixture(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("47950_upper.doc" | "47950_lower.doc")
    )
}

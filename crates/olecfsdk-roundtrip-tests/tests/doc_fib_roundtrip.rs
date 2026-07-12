use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    cfb::CompoundFile,
    doc::{
        AnnotationBookmarks, AnnotationOwners, AnnotationReferenceTable, AssociatedStrings,
        Bookmarks, ChpxFkp, Clx, CommandCustomizationRecord, CommandCustomizations, CpOnlyTable,
        DocOfficeArtContent, DocumentProperties, Fib, FibBase, FibBaseFlags, FieldCharacter,
        FieldDocumentPart, FieldTable, FontTable, HeaderStoryBoundary, HeaderTextTable,
        ListDefinitions, ListOverrides, NoteReferenceTable, PapxFkp, PapxLengthEncoding, PlcBte,
        PlcfSed, Prm, RevisionAuthors, SelectionRange, SelectionState, SelectionStateExtension,
        SelectionStyle, Sepx, ShapeAnchorTable, SpellingStateKind, SpellingStateTable, SprmGroup,
        SprmKind, SprmOperand, StyleFormatting, StyleKind, StyleSheet, TextPieceCharacters,
        TextboxBreakTable, TextboxDocumentPart, TextboxStoryChain, TextboxStoryTable,
        WORD97_FILE_IDENTIFIER,
    },
    office_art::OfficeArtRecordData,
};
use olecfsdk_corpus_test_support::{
    corpus_bytes,
    manifest::{ExpectationMode, read_manifest},
};

#[test]
#[ignore = "DOC FIB corpus round-trip runs explicitly"]
fn legacy_word_fibs_round_trip() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);
    let exclusions = excluded_files(&corpus);
    let mut observed_exclusions = BTreeSet::new();

    let mut checked = 0usize;
    let mut legacy = BTreeMap::<u16, usize>::new();
    let mut versions = BTreeMap::<u16, usize>::new();
    let mut fc_lcb_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut csw_new_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut nonempty_fc_lcb = BTreeMap::<usize, usize>::new();
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
    let mut sepx_raw_variable_operands = 0usize;
    let mut sepx_raw_variable_frequencies = BTreeMap::<u16, usize>::new();
    let mut sepx_trailing_bytes = BTreeMap::<u8, usize>::new();
    let mut style_sheets = 0usize;
    let mut style_sheet_info_shapes = BTreeMap::<(usize, u16), usize>::new();
    let mut styles = 0usize;
    let mut empty_styles = 0usize;
    let mut style_definition_bytes = 0usize;
    let mut style_name_units = 0usize;
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
    let mut list_override_sets = 0usize;
    let mut list_overrides = 0usize;
    let mut list_override_levels = 0usize;
    let mut formatted_list_override_levels = 0usize;
    let mut list_override_level_prls = 0usize;
    let mut list_override_text_units = 0usize;
    let mut list_override_missing_definitions = 0usize;
    let mut document_property_shapes = BTreeMap::<(u16, u32), usize>::new();
    let mut font_tables = 0usize;
    let mut fonts = 0usize;
    let mut alternate_font_names = 0usize;
    let mut font_name_units = 0usize;
    let mut padded_font_names = 0usize;
    let mut font_name_padding_units = 0usize;
    let mut font_family_shapes = BTreeMap::<(u8, bool, u8), usize>::new();
    let mut font_character_sets = BTreeMap::<u8, usize>::new();
    let mut associated_string_tables = 0usize;
    let mut associated_string_units = 0usize;
    let mut nonempty_associated_strings = BTreeMap::<usize, usize>::new();
    let mut maximum_associated_string_lengths = BTreeMap::<usize, usize>::new();
    let mut associated_string_padding = BTreeMap::<u8, usize>::new();
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
    let mut shape_anchors_without_fsp = 0usize;
    let mut textbox_stories_without_fsp = 0usize;
    let mut failures = Vec::new();

    for path in files {
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

            let fib =
                Fib::from_word_document(&word_document.data).map_err(|error| error.to_string())?;
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
                    *nonempty_fc_lcb.entry(index).or_default() += 1;
                }
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
            for (part, location) in fib.field_table_locations() {
                if location.lcb == 0 {
                    continue;
                }
                let field_bytes = bounded_slice(table, location.fc, location.lcb, "Plcfld")?;
                let fields = FieldTable::from_bytes(field_bytes)
                    .map_err(|error| format!("{part:?} Plcfld: {error}"))?;
                if fields.to_bytes().map_err(|error| error.to_string())? != field_bytes {
                    return Err(format!("{part:?} Plcfld write changed physical bytes"));
                }
                *field_tables.entry(part).or_default() += 1;
                field_records += fields.fields.len();
                for field in fields.fields {
                    let (character, reserved, field_type) = match field.character {
                        FieldCharacter::Begin {
                            reserved,
                            field_type,
                        } => (0x13, reserved, Some(field_type)),
                        FieldCharacter::Separator { reserved, .. } => (0x14, reserved, None),
                        FieldCharacter::End { reserved, .. } => (0x15, reserved, None),
                    };
                    *field_character_counts.entry((part, character)).or_default() += 1;
                    *field_reserved_counts.entry(reserved).or_default() += 1;
                    if let Some(field_type) = field_type {
                        *field_type_counts.entry(field_type).or_default() += 1;
                    }
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
                    if let SprmKind::Other(opcode) = property.sprm.kind() {
                        sepx_unknown_sprms.insert(opcode);
                    }
                    if matches!(
                        property.operand,
                        SprmOperand::Variable8(_) | SprmOperand::Variable16PlusOne(_)
                    ) {
                        sepx_raw_variable_operands += 1;
                        *sepx_raw_variable_frequencies
                            .entry(property.sprm.opcode().unwrap())
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
                        SprmOperand::ConditionalFormatting(value) => (
                            "conditional-formatting",
                            2 + value.properties.to_bytes().unwrap().len(),
                        ),
                        SprmOperand::AutoNumberedListData(_) => ("auto-numbered-list-data", 84),
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
    assert_eq!(encrypted_exclusions, 3);
    assert_eq!(invalid_exclusions, 36);
    assert_eq!(checked, 403);
    assert_eq!(
        nonempty_fc_lcb,
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
    assert_eq!(revision_author_tables, 319);
    assert_eq!(revision_authors, 343);
    assert_eq!(revision_author_units, 2_441);
    assert_eq!(maximum_revision_author_length, 30);
    assert_eq!(revision_author_zero_placeholders, 1);
    assert_eq!(
        revision_author_count_shapes,
        BTreeMap::from([(0, 1), (1, 304), (2, 11), (3, 1), (4, 1), (10, 1)])
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
    assert_eq!(
        sepx_unknown_sprms,
        BTreeSet::from([0x3014, 0x4231, 0xd202, 0xd238])
    );
    assert_eq!(sepx_raw_variable_operands, 5);
    assert_eq!(
        sepx_raw_variable_frequencies,
        BTreeMap::from([(0xd202, 3), (0xd238, 2)])
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
        _ => None,
    }
}

fn excluded_files(corpus: &Path) -> BTreeMap<PathBuf, ExpectationMode> {
    let mut exclusions = BTreeMap::new();
    for source in ["Apache-POI", "LibreOffice"] {
        let root = corpus.join(source);
        let manifest = read_manifest(&root.join("manifest.toml"))
            .unwrap_or_else(|error| panic!("read {source} manifest: {error}"));
        for expectation in manifest.expectation {
            if expectation.test == "doc_fib_roundtrip"
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

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use olecfsdk::{
    ParseDiagnosticCode,
    cfb::CompoundFile,
    office_art::OfficeArtRecordData,
    ppt::{
        ANIMATION_INFO_ATOM, BLIP_ENTITY9_ATOM, BUILD_ATOM, COMMENT_INDEX10_ATOM, COMMENT10_ATOM,
        CurrentUserData, CurrentUserStream, DATE_TIME_META_CHARACTER_ATOM,
        DOC_TOOLBAR_STATES10_ATOM, END_DOCUMENT_ATOM, EXTERNAL_HYPERLINK_ATOM,
        EXTERNAL_HYPERLINK_FLAGS_ATOM, EXTERNAL_MEDIA_ATOM, EXTERNAL_OBJECT_LIST_ATOM,
        EXTERNAL_OLE_EMBED_ATOM, EXTERNAL_OLE_OBJECT_ATOM, EXTERNAL_WAV_AUDIO_EMBEDDED_ATOM,
        FILTER_PRIVACY_FLAGS10_ATOM, FONT_ENTITY_ATOM, FOOTER_META_CHARACTER_ATOM,
        GENERIC_DATE_META_CHARACTER_ATOM, GRID_SPACING_10_ATOM, GUIDE_ATOM,
        HANDOUT_COMPATIBILITY_ATOM, HASH_CODE_ATOM, HEADER_META_CHARACTER_ATOM,
        HTML_DOC_INFO9_ATOM, HTML_PUBLISH_INFO_ATOM, INTERACTIVE_INFO_ATOM, KINSOKU_ATOM,
        LEVEL_INFO_ATOM, MAC_LEGACY_PRINT_INFO_ATOM, MAC_PAGE_FORMAT_ATOM,
        MAC_PRINT_DRIVER_INFO_ATOM, MAC_PRINT_SETTINGS_ATOM, NORMAL_VIEW_SET_INFO_9_ATOM,
        OUTLINE_TEXT_PROPS_HEADER9_ATOM, PARA_BUILD_ATOM, PPT10_RESERVED_ATOM,
        PPT11_FONT_DESCRIPTOR_ATOM, PPT11_FONT_DESCRIPTOR_COLLECTION_ATOM,
        PRESENTATION_ADVISOR_FLAGS9_ATOM, PRINT_OPTIONS_ATOM, PicturesStream, PowerPointDocument,
        PptFile, PptRecordData, ProgrammableTagKind, RECOLOR_INFO_ATOM,
        ROUND_TRIP_ANIMATION_HASH_12_ATOM, ROUND_TRIP_COMPOSITE_MASTER_ID_12_ATOM,
        ROUND_TRIP_CONTENT_MASTER_ID_12_ATOM, ROUND_TRIP_DOC_FLAGS_12_ATOM,
        ROUND_TRIP_HEADER_FOOTER_DEFAULTS_12_ATOM, ROUND_TRIP_HF_PLACEHOLDER_12_ATOM,
        ROUND_TRIP_NOTES_MASTER_TEXT_STYLES_12_ATOM, ROUND_TRIP_ORIGINAL_MAIN_MASTER_ID_12_ATOM,
        ROUND_TRIP_SHAPE_CHECKSUM_12_ATOM, ROUND_TRIP_SHAPE_ID_12_ATOM, SLIDE_FLAGS10_ATOM,
        SLIDE_NUMBER_META_CHARACTER_ATOM, SLIDE_SHOW_DOC_INFO_ATOM, SLIDE_SHOW_SLIDE_INFO_ATOM,
        SLIDE_TIME_10_ATOM, SLIDE_VIEW_INFO_ATOM, SOUND_COLLECTION_ATOM, SOUND_DATA_BLOB,
        STYLE_TEXT_PROP10_ATOM, STYLE_TEXT_PROP11_ATOM, TEXT_BOOKMARK_ATOM, TEXT_CF_EXCEPTION_ATOM,
        TEXT_DEFAULTS10_ATOM, TEXT_INTERACTIVE_INFO_ATOM, TEXT_MASTER_STYLE9_ATOM,
        TEXT_MASTER_STYLE10_ATOM, TEXT_PF_EXCEPTION_ATOM, TEXT_SI_EXCEPTION_ATOM,
        TIME_ANIMATE_BEHAVIOR_ATOM, TIME_ANIMATION_VALUE_ATOM, TIME_BEHAVIOR_ATOM,
        TIME_COMMAND_BEHAVIOR_ATOM, TIME_CONDITION_ATOM, TIME_EFFECT_BEHAVIOR_ATOM,
        TIME_MODIFIER_ATOM, TIME_MOTION_BEHAVIOR_ATOM, TIME_NODE_ATOM, TIME_SCALE_BEHAVIOR_ATOM,
        TIME_SEQUENCE_DATA_ATOM, TIME_SET_BEHAVIOR_ATOM, TIME_VARIANT_ATOM, VBA_INFO_ATOM,
        VIEW_INFO_ATOM, VISUAL_PAGE_ATOM, VISUAL_SHAPE_ATOM,
    },
};
use olecfsdk_corpus_test_support::{
    corpus_bytes,
    manifest::{ExpectationMode, read_manifest},
};

#[test]
#[ignore = "PPT97 record corpus round-trip runs explicitly"]
fn legacy_powerpoint_document_streams_round_trip() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let exclusions = excluded_files(&corpus);
    let mut observed_exclusions = BTreeSet::new();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);

    let mut checked = 0usize;
    let mut pictures_streams = 0usize;
    let mut picture_records = 0usize;
    let mut picture_partial_streams = 0usize;
    let mut picture_incomplete_records = 0usize;
    let mut picture_unparsed_bytes = 0usize;
    let mut picture_typed_records = 0usize;
    let mut picture_opaque_records = 0usize;
    let mut picture_payload_bytes = 0usize;
    let mut current_user_streams = 0usize;
    let mut current_user_parsed = 0usize;
    let mut current_user_compatibility = 0usize;
    let mut current_user_truncated = 0usize;
    let mut current_user_trailing_bytes = 0usize;
    let mut current_user_trailing_shapes = BTreeMap::<(usize, bool), usize>::new();
    let mut current_user_samples = Vec::new();
    let mut current_edit_links = 0usize;
    let mut current_edit_broken_links = 0usize;
    let mut persist_chains = 0usize;
    let mut persist_chain_failures = 0usize;
    let mut persist_chain_edits = 0usize;
    let mut effective_persist_objects = 0usize;
    let mut record_count = 0usize;
    let mut container_count = 0usize;
    let mut prog_binary_tags = BTreeMap::<Option<ProgrammableTagKind>, usize>::new();
    let mut binary_tag_data_blobs = 0usize;
    let mut binary_tag_trailing_bytes = 0usize;
    let mut document_atoms = 0usize;
    let mut slide_atoms = 0usize;
    let mut notes_atoms = 0usize;
    let mut office_art_records = 0usize;
    let mut office_art_bytes = 0usize;
    let mut outline_text_refs = 0usize;
    let mut text_headers = 0usize;
    let mut text_chars_records = 0usize;
    let mut text_chars_units = 0usize;
    let mut text_bytes_records = 0usize;
    let mut text_bytes_characters = 0usize;
    let mut style_text_prop_atoms = 0usize;
    let mut style_paragraph_runs = 0usize;
    let mut style_character_runs = 0usize;
    let mut style_tab_stops = 0usize;
    let mut style_trailing_bytes = 0usize;
    let mut malformed_style_text_prop = 0usize;
    let mut malformed_style_text_prop_bytes = 0usize;
    let mut unresolved_style_text_prop = 0usize;
    let mut unresolved_style_text_prop_bytes = 0usize;
    let mut unresolved_style_samples = Vec::new();
    let mut style_text_prop9_atoms = 0usize;
    let mut style_text_prop9_runs = 0usize;
    let mut style_text_prop9_bullet_blips = 0usize;
    let mut style_text_prop9_auto_number_flags = 0usize;
    let mut style_text_prop9_auto_number_schemes = 0usize;
    let mut style_text_prop9_character_extensions = 0usize;
    let mut style_text_prop9_special_extensions = 0usize;
    let mut style_text_prop9_bidi = 0usize;
    let mut style_text_prop9_smart_tags = 0usize;
    let mut malformed_style_text_prop9 = 0usize;
    let mut malformed_style_text_prop9_bytes = 0usize;
    let mut c_string_records = 0usize;
    let mut c_string_units = 0usize;
    let mut slide_persist_atoms = 0usize;
    let mut color_scheme_atoms = 0usize;
    let mut external_object_refs = 0usize;
    let mut placeholder_atoms = 0usize;
    let mut headers_footers_atoms = 0usize;
    let mut master_text_prop_atoms = 0usize;
    let mut master_text_prop_runs = 0usize;
    let mut text_master_style_atoms = 0usize;
    let mut text_master_style_levels = 0usize;
    let mut text_master_style_trailing_bytes = 0usize;
    let mut text_master_style_truncated_tails = 0usize;
    let mut text_master_style_compatibility_tails = 0usize;
    let mut text_master_style_trailing_samples = Vec::new();
    let mut malformed_text_master_style = 0usize;
    let mut malformed_text_master_style_bytes = 0usize;
    let mut text_ruler_atoms = 0usize;
    let mut text_ruler_tab_stops = 0usize;
    let mut text_ruler_trailing_bytes = 0usize;
    let mut malformed_text_rulers = 0usize;
    let mut malformed_text_ruler_bytes = 0usize;
    let mut external_storage_atoms = 0usize;
    let mut external_storage_parsed_compressed = 0usize;
    let mut external_storage_parsed_uncompressed = 0usize;
    let mut external_storage_invalid_compressed = 0usize;
    let mut external_storage_invalid_uncompressed = 0usize;
    let mut external_storage_malformed_compressed = 0usize;
    let mut external_storage_unsupported_instance = 0usize;
    let mut external_storage_entries = 0usize;
    let mut external_storage_vba_shaped = 0usize;
    let mut external_storage_vba_parsed = 0usize;
    let mut external_storage_vba_invalid = 0usize;
    let mut external_storage_vba_modules = 0usize;
    let mut external_storage_invalid_samples = Vec::new();
    let mut content_master_packages = 0usize;
    let mut content_master_payload_bytes = 0usize;
    let mut theme_packages = 0usize;
    let mut theme_payload_bytes = 0usize;
    let mut color_mapping_payloads = 0usize;
    let mut color_mapping_payload_bytes = 0usize;
    let mut animation_packages = 0usize;
    let mut animation_payload_bytes = 0usize;
    let mut text_style_packages = 0usize;
    let mut notes_style_packages = 0usize;
    let mut table_style_packages = 0usize;
    let mut style_payload_bytes = 0usize;
    let mut text_special_info_atoms = 0usize;
    let mut text_special_info_runs = 0usize;
    let mut text_special_info_smart_tags = 0usize;
    let mut malformed_text_special_info = 0usize;
    let mut malformed_text_special_info_bytes = 0usize;
    let mut typed_time_atoms = BTreeMap::<u16, usize>::new();
    let mut malformed_time_variants = 0usize;
    let mut blip_entity9_types = BTreeMap::<u16, usize>::new();
    let mut malformed_blip_entity9 = 0usize;
    let mut malformed_blip_entity9_bytes = 0usize;
    let mut user_edit_count = 0usize;
    let mut persist_directory_count = 0usize;
    let mut persist_directory_entries = 0usize;
    let mut persist_offsets = 0usize;
    let mut atom_stats = BTreeMap::<u16, RecordStats>::new();
    let mut unknown_record_stats = BTreeMap::<u16, RecordStats>::new();
    let mut selected_atom_samples = Vec::new();
    let mut truncated_records = 0usize;
    let mut truncated_bytes = 0usize;
    let mut truncated_shapes = BTreeMap::<(u16, u32, usize), usize>::new();
    let mut truncated_samples = Vec::new();
    let mut trailing_header_bytes = 0usize;
    let mut encrypted_user_edits = 0usize;
    let mut encrypted_samples = Vec::new();
    let mut truncated_diagnostics = 0usize;
    let mut nonconforming_diagnostics = 0usize;
    let mut current_user_nonconforming_diagnostics = 0usize;
    let mut current_user_diagnostic_samples = Vec::new();
    let mut noncanonical_cfb_diagnostics = 0usize;
    let mut invalid_stream_diagnostics = 0usize;
    let mut invalid_reference_diagnostics = 0usize;
    let mut truncated_diagnostic_files = BTreeSet::<PathBuf>::new();
    let mut nonconforming_diagnostic_files = BTreeSet::<PathBuf>::new();
    let mut noncanonical_cfb_diagnostic_files = BTreeSet::<PathBuf>::new();
    let mut invalid_stream_diagnostic_files = BTreeSet::<PathBuf>::new();
    let mut invalid_reference_diagnostic_files = BTreeSet::<PathBuf>::new();
    let mut failures = Vec::new();

    for path in files {
        if exclusions.contains(&path) {
            observed_exclusions.insert(path);
            continue;
        }
        let result = (|| {
            let bytes = corpus_bytes(&path).map_err(|error| error.to_string())?;
            let cfb = CompoundFile::from_bytes(&bytes).map_err(|error| error.to_string())?;
            let Some(entry) = cfb
                .entries()
                .iter()
                .find(|entry| entry.is_stream() && entry.name == "PowerPoint Document")
            else {
                return Ok::<_, String>(false);
            };
            checked += 1;
            let document = PowerPointDocument::from_bytes(&entry.data)
                .map_err(|error| format!("parse PowerPoint Document: {error}"))?;
            let outcome = PptFile::from_compound_file_compatible(cfb.clone())
                .map_err(|error| format!("parse typed PPT file root: {error}"))?;
            for diagnostic in &outcome.diagnostics {
                if diagnostic.location.path.as_deref() == Some("/Current User")
                    && current_user_diagnostic_samples.len() < 20
                {
                    current_user_diagnostic_samples.push(format!(
                        "{}: {}: {}",
                        path.display(),
                        diagnostic.code.as_str(),
                        diagnostic.message
                    ));
                }
                match diagnostic.code {
                    ParseDiagnosticCode::TruncatedRecord => {
                        truncated_diagnostics += 1;
                        truncated_diagnostic_files.insert(path.clone());
                        if !matches!(
                            diagnostic.location.path.as_deref(),
                            Some("/PowerPoint Document" | "/Current User")
                        ) {
                            return Err(format!(
                                "truncated diagnostic has unexpected location: {diagnostic:?}"
                            ));
                        }
                    }
                    ParseDiagnosticCode::NonconformingRecord => {
                        nonconforming_diagnostics += 1;
                        nonconforming_diagnostic_files.insert(path.clone());
                        current_user_nonconforming_diagnostics += usize::from(
                            diagnostic.location.path.as_deref() == Some("/Current User"),
                        );
                    }
                    ParseDiagnosticCode::NoncanonicalCompoundFile => {
                        noncanonical_cfb_diagnostics += 1;
                        noncanonical_cfb_diagnostic_files.insert(path.clone());
                    }
                    ParseDiagnosticCode::InvalidStreamPreserved => {
                        invalid_stream_diagnostics += 1;
                        invalid_stream_diagnostic_files.insert(path.clone());
                        if diagnostic.location.path.as_deref() != Some("/Pictures") {
                            return Err(format!(
                                "invalid-stream diagnostic has unexpected location: {diagnostic:?}"
                            ));
                        }
                    }
                    ParseDiagnosticCode::InvalidReference => {
                        invalid_reference_diagnostics += 1;
                        invalid_reference_diagnostic_files.insert(path.clone());
                        if diagnostic.location.path.as_deref() != Some("/PowerPoint Document") {
                            return Err(format!(
                                "invalid-reference diagnostic has unexpected location: {diagnostic:?}"
                            ));
                        }
                    }
                }
            }
            let file = outcome.value;
            let root_document = cfb
                .stream("/PowerPoint Document")
                .ok_or_else(|| "typed PPT file root has no root document stream".to_owned())?;
            if file
                .document
                .to_bytes()
                .map_err(|error| format!("write typed PPT document tree: {error}"))?
                != root_document
            {
                return Err("typed PPT file root changed the root document tree".into());
            }
            let rebuilt = file
                .to_compound_file_preserving_compatibility()
                .map_err(|error| format!("write typed PPT file root: {error}"))?;
            if rebuilt.stream("/PowerPoint Document") != Some(root_document) {
                return Err("typed PPT file root changed the document stream".into());
            }
            let saved = document
                .to_bytes()
                .map_err(|error| format!("write PowerPoint Document: {error}"))?;
            if saved != entry.data {
                return Err("PowerPoint Document bytes changed after round-trip".into());
            }
            if PowerPointDocument::from_bytes(&saved)
                .map_err(|error| format!("reparse PowerPoint Document: {error}"))?
                != document
            {
                return Err("PowerPoint Document structure changed after round-trip".into());
            }
            if let Some(current_entry) = cfb
                .entries()
                .iter()
                .find(|entry| entry.is_stream() && entry.name == "Current User")
            {
                current_user_streams += 1;
                let current = CurrentUserStream::from_bytes(&current_entry.data)
                    .map_err(|error| format!("parse Current User: {error}"))?;
                if current
                    .to_bytes()
                    .map_err(|error| format!("write Current User: {error}"))?
                    != current_entry.data
                {
                    return Err("Current User bytes changed after round-trip".into());
                }
                current_user_trailing_bytes += current.padding.len();
                *current_user_trailing_shapes
                    .entry((
                        current.padding.len(),
                        current.padding.iter().all(|byte| *byte == 0),
                    ))
                    .or_default() += 1;
                match &current.data {
                    CurrentUserData::Parsed(atom) => {
                        current_user_parsed += 1;
                        let mut target_is_user_edit = false;
                        document.records.visit(&mut |record| {
                            if record.offset == u64::from(atom.offset_to_current_edit)
                                && matches!(record.data, PptRecordData::UserEdit(_))
                            {
                                target_is_user_edit = true;
                            }
                        });
                        if target_is_user_edit {
                            current_edit_links += 1;
                            match document.incremental_save_chain(atom) {
                                Ok(chain) => {
                                    persist_chains += 1;
                                    persist_chain_edits += chain.edits.len();
                                    effective_persist_objects += chain.persist_object_offsets.len();
                                }
                                Err(error) => {
                                    persist_chain_failures += 1;
                                    current_user_samples.push(format!(
                                        "{} invalid persist chain: {error}",
                                        path.display()
                                    ));
                                }
                            }
                        } else {
                            current_edit_broken_links += 1;
                            current_user_samples.push(format!(
                                "{} broken current-edit offset {} stream len {}",
                                path.display(),
                                atom.offset_to_current_edit,
                                current_entry.data.len()
                            ));
                        }
                    }
                    CurrentUserData::Compatibility(_) => current_user_compatibility += 1,
                    CurrentUserData::Truncated(bytes) => {
                        current_user_truncated += 1;
                        current_user_samples.push(format!(
                            "{} truncated CurrentUserAtom declared {} available {}",
                            path.display(),
                            current.header.declared_length,
                            bytes.len()
                        ));
                    }
                }
            }
            if let Some(pictures_entry) = cfb
                .entries()
                .iter()
                .find(|entry| entry.is_stream() && entry.name == "Pictures")
            {
                pictures_streams += 1;
                match PicturesStream::from_bytes(&pictures_entry.data)
                    .map_err(|error| format!("parse Pictures stream: {error}"))?
                {
                    PicturesStream::Complete(pictures) => {
                        if pictures
                            .to_bytes()
                            .map_err(|error| format!("write Pictures stream: {error}"))?
                            != pictures_entry.data
                        {
                            return Err("Pictures stream bytes changed after round-trip".into());
                        }
                        picture_records += pictures.records.len();
                        for record in &pictures.records {
                            picture_payload_bytes += record.header.declared_length as usize;
                            if matches!(record.data, OfficeArtRecordData::Atom(_)) {
                                picture_opaque_records += 1;
                            } else {
                                picture_typed_records += 1;
                            }
                        }
                    }
                    PicturesStream::Partial(partial) => {
                        if partial
                            .to_bytes()
                            .map_err(|error| format!("write partial Pictures stream: {error}"))?
                            != pictures_entry.data
                        {
                            return Err("partial Pictures bytes changed after round-trip".into());
                        }
                        picture_partial_streams += 1;
                        picture_records += partial.complete_record_count();
                        picture_incomplete_records += partial.incomplete_record_count();
                        picture_unparsed_bytes += partial.unparsed_byte_count();
                        partial.visit_complete(|record| {
                            picture_payload_bytes += record.header.declared_length as usize;
                            if matches!(record.data, OfficeArtRecordData::Atom(_)) {
                                picture_opaque_records += 1;
                            } else {
                                picture_typed_records += 1;
                            }
                        });
                    }
                }
            }
            audit_sequence(
                &document.records,
                &mut RecordAudit {
                    records: &mut record_count,
                    containers: &mut container_count,
                    prog_binary_tags: &mut prog_binary_tags,
                    binary_tag_data_blobs: &mut binary_tag_data_blobs,
                    binary_tag_trailing_bytes: &mut binary_tag_trailing_bytes,
                    document_atoms: &mut document_atoms,
                    slide_atoms: &mut slide_atoms,
                    notes_atoms: &mut notes_atoms,
                    office_art_records: &mut office_art_records,
                    office_art_bytes: &mut office_art_bytes,
                    outline_text_refs: &mut outline_text_refs,
                    text_headers: &mut text_headers,
                    text_chars_records: &mut text_chars_records,
                    text_chars_units: &mut text_chars_units,
                    text_bytes_records: &mut text_bytes_records,
                    text_bytes_characters: &mut text_bytes_characters,
                    style_text_prop_atoms: &mut style_text_prop_atoms,
                    style_paragraph_runs: &mut style_paragraph_runs,
                    style_character_runs: &mut style_character_runs,
                    style_tab_stops: &mut style_tab_stops,
                    style_trailing_bytes: &mut style_trailing_bytes,
                    malformed_style_text_prop: &mut malformed_style_text_prop,
                    malformed_style_text_prop_bytes: &mut malformed_style_text_prop_bytes,
                    unresolved_style_text_prop: &mut unresolved_style_text_prop,
                    unresolved_style_text_prop_bytes: &mut unresolved_style_text_prop_bytes,
                    unresolved_style_samples: &mut unresolved_style_samples,
                    style_text_prop9_atoms: &mut style_text_prop9_atoms,
                    style_text_prop9_runs: &mut style_text_prop9_runs,
                    style_text_prop9_bullet_blips: &mut style_text_prop9_bullet_blips,
                    style_text_prop9_auto_number_flags: &mut style_text_prop9_auto_number_flags,
                    style_text_prop9_auto_number_schemes: &mut style_text_prop9_auto_number_schemes,
                    style_text_prop9_character_extensions:
                        &mut style_text_prop9_character_extensions,
                    style_text_prop9_special_extensions: &mut style_text_prop9_special_extensions,
                    style_text_prop9_bidi: &mut style_text_prop9_bidi,
                    style_text_prop9_smart_tags: &mut style_text_prop9_smart_tags,
                    malformed_style_text_prop9: &mut malformed_style_text_prop9,
                    malformed_style_text_prop9_bytes: &mut malformed_style_text_prop9_bytes,
                    c_string_records: &mut c_string_records,
                    c_string_units: &mut c_string_units,
                    slide_persist_atoms: &mut slide_persist_atoms,
                    color_scheme_atoms: &mut color_scheme_atoms,
                    external_object_refs: &mut external_object_refs,
                    placeholder_atoms: &mut placeholder_atoms,
                    headers_footers_atoms: &mut headers_footers_atoms,
                    master_text_prop_atoms: &mut master_text_prop_atoms,
                    master_text_prop_runs: &mut master_text_prop_runs,
                    text_master_style_atoms: &mut text_master_style_atoms,
                    text_master_style_levels: &mut text_master_style_levels,
                    text_master_style_trailing_bytes: &mut text_master_style_trailing_bytes,
                    text_master_style_truncated_tails: &mut text_master_style_truncated_tails,
                    text_master_style_compatibility_tails:
                        &mut text_master_style_compatibility_tails,
                    text_master_style_trailing_samples: &mut text_master_style_trailing_samples,
                    malformed_text_master_style: &mut malformed_text_master_style,
                    malformed_text_master_style_bytes: &mut malformed_text_master_style_bytes,
                    text_ruler_atoms: &mut text_ruler_atoms,
                    text_ruler_tab_stops: &mut text_ruler_tab_stops,
                    text_ruler_trailing_bytes: &mut text_ruler_trailing_bytes,
                    malformed_text_rulers: &mut malformed_text_rulers,
                    malformed_text_ruler_bytes: &mut malformed_text_ruler_bytes,
                    external_storage_atoms: &mut external_storage_atoms,
                    external_storage_parsed_compressed: &mut external_storage_parsed_compressed,
                    external_storage_parsed_uncompressed: &mut external_storage_parsed_uncompressed,
                    external_storage_invalid_compressed: &mut external_storage_invalid_compressed,
                    external_storage_invalid_uncompressed:
                        &mut external_storage_invalid_uncompressed,
                    external_storage_malformed_compressed:
                        &mut external_storage_malformed_compressed,
                    external_storage_unsupported_instance:
                        &mut external_storage_unsupported_instance,
                    external_storage_entries: &mut external_storage_entries,
                    external_storage_vba_shaped: &mut external_storage_vba_shaped,
                    external_storage_vba_parsed: &mut external_storage_vba_parsed,
                    external_storage_vba_invalid: &mut external_storage_vba_invalid,
                    external_storage_vba_modules: &mut external_storage_vba_modules,
                    external_storage_invalid_samples: &mut external_storage_invalid_samples,
                    content_master_packages: &mut content_master_packages,
                    content_master_payload_bytes: &mut content_master_payload_bytes,
                    theme_packages: &mut theme_packages,
                    theme_payload_bytes: &mut theme_payload_bytes,
                    color_mapping_payloads: &mut color_mapping_payloads,
                    color_mapping_payload_bytes: &mut color_mapping_payload_bytes,
                    animation_packages: &mut animation_packages,
                    animation_payload_bytes: &mut animation_payload_bytes,
                    text_style_packages: &mut text_style_packages,
                    notes_style_packages: &mut notes_style_packages,
                    table_style_packages: &mut table_style_packages,
                    style_payload_bytes: &mut style_payload_bytes,
                    text_special_info_atoms: &mut text_special_info_atoms,
                    text_special_info_runs: &mut text_special_info_runs,
                    text_special_info_smart_tags: &mut text_special_info_smart_tags,
                    malformed_text_special_info: &mut malformed_text_special_info,
                    malformed_text_special_info_bytes: &mut malformed_text_special_info_bytes,
                    typed_time_atoms: &mut typed_time_atoms,
                    malformed_time_variants: &mut malformed_time_variants,
                    blip_entity9_types: &mut blip_entity9_types,
                    malformed_blip_entity9: &mut malformed_blip_entity9,
                    malformed_blip_entity9_bytes: &mut malformed_blip_entity9_bytes,
                    user_edits: &mut user_edit_count,
                    persist_directories: &mut persist_directory_count,
                    persist_entries: &mut persist_directory_entries,
                    persist_offsets: &mut persist_offsets,
                    atoms: &mut atom_stats,
                    unknown_records: &mut unknown_record_stats,
                    selected_atom_samples: &mut selected_atom_samples,
                    truncated_records: &mut truncated_records,
                    truncated_bytes: &mut truncated_bytes,
                    truncated_shapes: &mut truncated_shapes,
                    truncated_samples: &mut truncated_samples,
                    trailing_header_bytes: &mut trailing_header_bytes,
                    encrypted_user_edits: &mut encrypted_user_edits,
                    encrypted_samples: &mut encrypted_samples,
                    source: &path,
                },
                &mut Vec::new(),
                None,
            );
            Ok(true)
        })();
        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }

    let missing_exclusions: Vec<_> = exclusions.difference(&observed_exclusions).collect();
    assert!(
        missing_exclusions.is_empty(),
        "PPT exclusions no longer occur in the selected corpus: {missing_exclusions:?}"
    );
    assert!(checked > 0, "no PowerPoint Document streams found");
    assert!(record_count > 0, "PowerPoint streams contained no records");
    assert!(
        container_count > 0,
        "PowerPoint streams contained no containers"
    );
    assert!(
        user_edit_count > 0,
        "PowerPoint streams contained no UserEditAtom"
    );
    assert!(
        persist_directory_count > 0,
        "PowerPoint streams contained no PersistDirectoryAtom"
    );
    assert!(
        failures.is_empty(),
        "{} PPT97 corpus failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!(
        "PPT StyleTextProp9: {style_text_prop9_atoms} atoms/{style_text_prop9_runs} runs, {style_text_prop9_bullet_blips} blips/{style_text_prop9_auto_number_flags} auto-number flags/{style_text_prop9_auto_number_schemes} schemes, {style_text_prop9_character_extensions} CF extensions/{style_text_prop9_special_extensions} SI extensions/{style_text_prop9_bidi} bidi/{style_text_prop9_smart_tags} smart tags; {malformed_style_text_prop9} malformed/{malformed_style_text_prop9_bytes} bytes"
    );
    eprintln!(
        "PPT BlipEntity9: types {blip_entity9_types:#x?}; {malformed_blip_entity9} malformed/{malformed_blip_entity9_bytes} bytes"
    );
    eprintln!(
        "PPT color mapping: {color_mapping_payloads} payloads/{color_mapping_payload_bytes} bytes; notes styles {notes_style_packages}"
    );
    eprintln!("PPT animation OPC: {animation_packages} packages/{animation_payload_bytes} bytes");
    assert_eq!(checked, 176, "supported PPT97 stream coverage changed");
    assert_eq!(current_user_streams, 176);
    assert_eq!(current_user_parsed, 175);
    assert_eq!(current_user_compatibility, 0);
    assert_eq!(current_user_truncated, 1);
    assert_eq!(current_edit_links, 168);
    assert_eq!(current_edit_broken_links, 7);
    assert_eq!(current_user_trailing_bytes, 72_628);
    assert!(
        current_user_trailing_shapes
            .keys()
            .all(|(_, all_zero)| *all_zero),
        "Current User stream retained non-padding trailing bytes"
    );
    assert_eq!(persist_chains, 167);
    assert_eq!(persist_chain_failures, 1);
    assert_eq!(persist_chain_edits, 249);
    assert_eq!(effective_persist_objects, 2_338);
    eprintln!(
        "PPT tag kinds: {prog_binary_tags:?}; records {record_count}/containers {container_count}"
    );
    assert_eq!(record_count, 226_831);
    assert_eq!(container_count, 65_944);
    assert_eq!(binary_tag_data_blobs, 2_622);
    assert_eq!(binary_tag_trailing_bytes, 0);
    assert_eq!(prog_binary_tags.len(), 7);
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::Ppt9)),
        Some(&841)
    );
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::Ppt10)),
        Some(&1_504)
    );
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::Ppt11)),
        Some(&51)
    );
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::Ppt12)),
        Some(&89)
    );
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::PptMac11)),
        Some(&39)
    );
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::Ppt2001)),
        Some(&54)
    );
    assert_eq!(
        prog_binary_tags.get(&Some(ProgrammableTagKind::Other)),
        Some(&45)
    );
    assert_eq!(user_edit_count, 259);
    assert_eq!(encrypted_user_edits, 0);
    assert_eq!(persist_directory_count, 259);
    assert_eq!(persist_directory_entries, 728);
    assert_eq!(persist_offsets, 2_766);
    assert_eq!(document_atoms, 260);
    assert_eq!(slide_atoms, 1_541);
    assert_eq!(notes_atoms, 667);
    assert_eq!(office_art_records, 69_426);
    assert_eq!(office_art_bytes, 5_302_645);
    assert_eq!(outline_text_refs, 1_300);
    assert_eq!(text_headers, 11_190);
    assert_eq!(text_chars_records, 2_065);
    assert_eq!(text_chars_units, 233_624);
    assert_eq!(text_bytes_records, 7_401);
    assert_eq!(text_bytes_characters, 391_831);
    assert_eq!(style_text_prop_atoms, 8_553);
    assert_eq!(style_paragraph_runs, 12_290);
    assert_eq!(style_character_runs, 18_524);
    assert_eq!(style_tab_stops, 13);
    assert_eq!(style_trailing_bytes, 0);
    assert_eq!(malformed_style_text_prop, 0);
    assert_eq!(malformed_style_text_prop_bytes, 0);
    assert_eq!(unresolved_style_text_prop, 0);
    assert_eq!(unresolved_style_text_prop_bytes, 0);
    assert_eq!(style_text_prop9_atoms, 822);
    assert_eq!(style_text_prop9_runs, 1_586);
    assert_eq!(style_text_prop9_bullet_blips, 341);
    assert_eq!(style_text_prop9_auto_number_flags, 729);
    assert_eq!(style_text_prop9_auto_number_schemes, 21);
    assert_eq!(style_text_prop9_character_extensions, 284);
    assert_eq!(style_text_prop9_special_extensions, 11);
    assert_eq!(style_text_prop9_bidi, 10);
    assert_eq!(style_text_prop9_smart_tags, 0);
    assert_eq!(malformed_style_text_prop9, 0);
    assert_eq!(malformed_style_text_prop9_bytes, 0);
    assert_eq!(c_string_records, 4_937);
    assert_eq!(c_string_units, 61_376);
    assert_eq!(slide_persist_atoms, 3_960);
    assert_eq!(color_scheme_atoms, 3_772);
    assert_eq!(external_object_refs, 201);
    assert_eq!(placeholder_atoms, 4_228);
    assert_eq!(headers_footers_atoms, 565);
    assert_eq!(master_text_prop_atoms, 528);
    assert_eq!(master_text_prop_runs, 1_663);
    assert_eq!(text_master_style_atoms, 1_971);
    assert_eq!(text_master_style_levels, 8_242);
    assert_eq!(text_master_style_trailing_bytes, 32);
    assert_eq!(text_master_style_truncated_tails, 1);
    assert_eq!(text_master_style_compatibility_tails, 0);
    assert_eq!(malformed_text_master_style, 0);
    assert_eq!(malformed_text_master_style_bytes, 0);
    assert_eq!(text_ruler_atoms, 3_971);
    assert_eq!(text_ruler_tab_stops, 16_590);
    assert_eq!(text_ruler_trailing_bytes, 0);
    assert_eq!(malformed_text_rulers, 0);
    assert_eq!(malformed_text_ruler_bytes, 0);
    assert_eq!(external_storage_atoms, 249);
    assert_eq!(external_storage_parsed_compressed, 247);
    assert_eq!(external_storage_parsed_uncompressed, 0);
    assert_eq!(external_storage_invalid_compressed, 2);
    assert_eq!(external_storage_invalid_uncompressed, 0);
    assert_eq!(external_storage_malformed_compressed, 0);
    assert_eq!(external_storage_unsupported_instance, 0);
    assert_eq!(external_storage_entries, 1_370);
    assert_eq!(external_storage_vba_shaped, 5);
    assert_eq!(external_storage_vba_parsed, 5);
    assert_eq!(external_storage_vba_invalid, 0);
    assert_eq!(external_storage_vba_modules, 6);
    assert_eq!(content_master_packages, 602);
    assert_eq!(content_master_payload_bytes, 981_249);
    assert_eq!(theme_packages, 152);
    assert_eq!(theme_payload_bytes, 491_582);
    assert_eq!(color_mapping_payloads, 152);
    assert_eq!(color_mapping_payload_bytes, 47_767);
    assert_eq!(animation_packages, 31);
    assert_eq!(animation_payload_bytes, 31_844);
    assert_eq!(text_style_packages, 104);
    assert_eq!(notes_style_packages, 30);
    assert_eq!(table_style_packages, 53);
    assert_eq!(style_payload_bytes, 345_319);
    assert_eq!(text_special_info_atoms, 7_387);
    assert_eq!(text_special_info_runs, 15_367);
    assert_eq!(text_special_info_smart_tags, 0);
    assert_eq!(malformed_text_special_info, 2);
    assert_eq!(malformed_text_special_info_bytes, 110);
    assert_eq!(typed_time_atoms.len(), 86);
    assert_eq!(typed_time_atoms.get(&PPT11_FONT_DESCRIPTOR_ATOM), Some(&58));
    assert_eq!(
        typed_time_atoms.get(&PPT11_FONT_DESCRIPTOR_COLLECTION_ATOM),
        Some(&1)
    );
    assert_eq!(typed_time_atoms.get(&MAC_PRINT_SETTINGS_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&MAC_PAGE_FORMAT_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&PPT10_RESERVED_ATOM), Some(&43));
    assert_eq!(typed_time_atoms.get(&MAC_LEGACY_PRINT_INFO_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&MAC_PRINT_DRIVER_INFO_ATOM), Some(&1));
    assert_eq!(typed_time_atoms.get(&HANDOUT_COMPATIBILITY_ATOM), Some(&1));
    assert_eq!(typed_time_atoms.get(&TEXT_CF_EXCEPTION_ATOM), Some(&248));
    assert_eq!(typed_time_atoms.get(&TEXT_PF_EXCEPTION_ATOM), Some(&229));
    assert_eq!(typed_time_atoms.get(&TEXT_SI_EXCEPTION_ATOM), Some(&259));
    assert_eq!(typed_time_atoms.get(&TEXT_MASTER_STYLE9_ATOM), Some(&13));
    assert_eq!(typed_time_atoms.get(&STYLE_TEXT_PROP10_ATOM), Some(&115));
    assert_eq!(typed_time_atoms.get(&TEXT_MASTER_STYLE10_ATOM), Some(&1));
    assert_eq!(typed_time_atoms.get(&TEXT_DEFAULTS10_ATOM), Some(&1));
    assert_eq!(typed_time_atoms.get(&STYLE_TEXT_PROP11_ATOM), Some(&11));
    assert_eq!(typed_time_atoms.get(&RECOLOR_INFO_ATOM), Some(&71));
    assert_eq!(typed_time_atoms.get(&TIME_NODE_ATOM), Some(&1_838));
    assert_eq!(typed_time_atoms.get(&TIME_CONDITION_ATOM), Some(&1_275));
    assert_eq!(typed_time_atoms.get(&TIME_MODIFIER_ATOM), Some(&20));
    assert_eq!(typed_time_atoms.get(&TIME_BEHAVIOR_ATOM), Some(&542));
    assert_eq!(
        typed_time_atoms.get(&TIME_ANIMATE_BEHAVIOR_ATOM),
        Some(&203)
    );
    assert_eq!(typed_time_atoms.get(&TIME_EFFECT_BEHAVIOR_ATOM), Some(&58));
    assert_eq!(typed_time_atoms.get(&TIME_MOTION_BEHAVIOR_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&TIME_SCALE_BEHAVIOR_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&TIME_SET_BEHAVIOR_ATOM), Some(&276));
    assert_eq!(typed_time_atoms.get(&TIME_COMMAND_BEHAVIOR_ATOM), Some(&1));
    assert_eq!(typed_time_atoms.get(&TIME_SEQUENCE_DATA_ATOM), Some(&152));
    assert_eq!(typed_time_atoms.get(&TIME_ANIMATION_VALUE_ATOM), Some(&406));
    assert_eq!(typed_time_atoms.get(&TIME_VARIANT_ATOM), Some(&3_519));
    assert_eq!(malformed_time_variants, 0);
    assert_eq!(typed_time_atoms.get(&VISUAL_SHAPE_ATOM), Some(&542));
    assert_eq!(typed_time_atoms.get(&HASH_CODE_ATOM), Some(&455));
    assert_eq!(typed_time_atoms.get(&VISUAL_PAGE_ATOM), Some(&306));
    assert_eq!(typed_time_atoms.get(&BUILD_ATOM), Some(&67));
    assert_eq!(typed_time_atoms.get(&PARA_BUILD_ATOM), Some(&67));
    assert_eq!(typed_time_atoms.get(&LEVEL_INFO_ATOM), Some(&3));
    assert_eq!(typed_time_atoms.get(&SLIDE_TIME_10_ATOM), Some(&1_023));
    assert_eq!(typed_time_atoms.get(&FONT_ENTITY_ATOM), Some(&829));
    assert_eq!(typed_time_atoms.get(&EXTERNAL_OLE_OBJECT_ATOM), Some(&501));
    assert_eq!(typed_time_atoms.get(&EXTERNAL_OLE_EMBED_ATOM), Some(&501));
    assert_eq!(typed_time_atoms.get(&KINSOKU_ATOM), Some(&311));
    assert_eq!(typed_time_atoms.get(&EXTERNAL_HYPERLINK_ATOM), Some(&560));
    assert_eq!(
        typed_time_atoms.get(&EXTERNAL_HYPERLINK_FLAGS_ATOM),
        Some(&160)
    );
    assert_eq!(
        typed_time_atoms.get(&SLIDE_NUMBER_META_CHARACTER_ATOM),
        Some(&441)
    );
    assert_eq!(
        typed_time_atoms.get(&TEXT_INTERACTIVE_INFO_ATOM),
        Some(&204)
    );
    assert_eq!(typed_time_atoms.get(&ANIMATION_INFO_ATOM), Some(&480));
    assert_eq!(typed_time_atoms.get(&INTERACTIVE_INFO_ATOM), Some(&289));
    assert_eq!(
        typed_time_atoms.get(&DATE_TIME_META_CHARACTER_ATOM),
        Some(&30)
    );
    assert_eq!(
        typed_time_atoms.get(&GENERIC_DATE_META_CHARACTER_ATOM),
        Some(&254)
    );
    assert_eq!(typed_time_atoms.get(&HEADER_META_CHARACTER_ATOM), Some(&95));
    assert_eq!(
        typed_time_atoms.get(&FOOTER_META_CHARACTER_ATOM),
        Some(&184)
    );
    assert_eq!(typed_time_atoms.get(&VIEW_INFO_ATOM), Some(&813));
    assert_eq!(typed_time_atoms.get(&BLIP_ENTITY9_ATOM), Some(&58));
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_ANIMATION_HASH_12_ATOM),
        Some(&31)
    );
    assert_eq!(
        typed_time_atoms.get(&SLIDE_SHOW_SLIDE_INFO_ATOM),
        Some(&587)
    );
    assert_eq!(typed_time_atoms.get(&GUIDE_ATOM), Some(&767));
    assert_eq!(typed_time_atoms.get(&SLIDE_VIEW_INFO_ATOM), Some(&378));
    assert_eq!(typed_time_atoms.get(&VBA_INFO_ATOM), Some(&198));
    assert_eq!(typed_time_atoms.get(&SLIDE_SHOW_DOC_INFO_ATOM), Some(&68));
    assert_eq!(typed_time_atoms.get(&EXTERNAL_OBJECT_LIST_ATOM), Some(&85));
    assert_eq!(typed_time_atoms.get(&GRID_SPACING_10_ATOM), Some(&179));
    assert_eq!(
        typed_time_atoms.get(&NORMAL_VIEW_SET_INFO_9_ATOM),
        Some(&216)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_ORIGINAL_MAIN_MASTER_ID_12_ATOM),
        Some(&59)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_COMPOSITE_MASTER_ID_12_ATOM),
        Some(&70)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_SHAPE_ID_12_ATOM),
        Some(&214)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_HF_PLACEHOLDER_12_ATOM),
        Some(&118)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_CONTENT_MASTER_ID_12_ATOM),
        Some(&146)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_HEADER_FOOTER_DEFAULTS_12_ATOM),
        Some(&28)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_DOC_FLAGS_12_ATOM),
        Some(&61)
    );
    assert_eq!(
        typed_time_atoms.get(&ROUND_TRIP_SHAPE_CHECKSUM_12_ATOM),
        Some(&28)
    );
    assert_eq!(typed_time_atoms.get(&END_DOCUMENT_ATOM), Some(&259));
    assert_eq!(typed_time_atoms.get(&SOUND_COLLECTION_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&SOUND_DATA_BLOB), Some(&2));
    assert_eq!(typed_time_atoms.get(&TEXT_BOOKMARK_ATOM), Some(&9));
    assert_eq!(
        typed_time_atoms.get(&OUTLINE_TEXT_PROPS_HEADER9_ATOM),
        Some(&80)
    );
    assert_eq!(typed_time_atoms.get(&EXTERNAL_MEDIA_ATOM), Some(&1));
    assert_eq!(
        typed_time_atoms.get(&EXTERNAL_WAV_AUDIO_EMBEDDED_ATOM),
        Some(&1)
    );
    assert_eq!(typed_time_atoms.get(&PRINT_OPTIONS_ATOM), Some(&2));
    assert_eq!(
        typed_time_atoms.get(&PRESENTATION_ADVISOR_FLAGS9_ATOM),
        Some(&9)
    );
    assert_eq!(typed_time_atoms.get(&HTML_DOC_INFO9_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&HTML_PUBLISH_INFO_ATOM), Some(&2));
    assert_eq!(typed_time_atoms.get(&COMMENT10_ATOM), Some(&10));
    assert_eq!(typed_time_atoms.get(&COMMENT_INDEX10_ATOM), Some(&10));
    assert_eq!(typed_time_atoms.get(&SLIDE_FLAGS10_ATOM), Some(&36));
    assert_eq!(typed_time_atoms.get(&FILTER_PRIVACY_FLAGS10_ATOM), Some(&5));
    assert_eq!(typed_time_atoms.get(&DOC_TOOLBAR_STATES10_ATOM), Some(&4));
    assert_eq!(blip_entity9_types.len(), 1);
    assert_eq!(blip_entity9_types.get(&0xf01e), Some(&58));
    assert_eq!(malformed_blip_entity9, 0);
    assert_eq!(malformed_blip_entity9_bytes, 0);
    assert_eq!(pictures_streams, 86);
    assert_eq!(picture_partial_streams, 5);
    assert_eq!(picture_records, 1_724);
    assert_eq!(picture_incomplete_records, 2);
    assert_eq!(picture_typed_records, 1_724);
    assert_eq!(picture_opaque_records, 0);
    assert_eq!(picture_payload_bytes, 22_883_441);
    assert_eq!(picture_unparsed_bytes, 2_024);
    assert_eq!(atom_stats.len(), 0);
    assert_eq!(
        atom_stats.values().map(|stats| stats.bytes).sum::<usize>(),
        0
    );
    assert_eq!(unknown_record_stats.len(), 3);
    assert_eq!(unknown_record_stats[&0x0000].records, 381);
    assert_eq!(unknown_record_stats[&0x0000].bytes, 14);
    assert_eq!(unknown_record_stats[&0x0000].lengths, [0, 14].into());
    assert_eq!(unknown_record_stats[&0x0080].records, 1);
    assert_eq!(unknown_record_stats[&0x0080].bytes, 0);
    assert_eq!(unknown_record_stats[&0x0080].lengths, [0].into());
    assert_eq!(unknown_record_stats[&0x779f].records, 1);
    assert_eq!(unknown_record_stats[&0x779f].bytes, 4);
    assert_eq!(unknown_record_stats[&0x779f].lengths, [4].into());
    assert_eq!(truncated_records, 52);
    assert_eq!(truncated_bytes, 85_738);
    assert_eq!(trailing_header_bytes, 4);
    assert!(
        truncated_diagnostics >= truncated_records + current_user_truncated,
        "every retained truncated record requires a diagnostic"
    );
    assert!(
        trailing_header_bytes == 0
            || truncated_diagnostics > truncated_records + current_user_truncated,
        "retained RecordHeader prefixes require diagnostics"
    );
    assert_eq!(
        nonconforming_diagnostics,
        current_user_nonconforming_diagnostics
            + malformed_text_special_info
            + malformed_style_text_prop
            + unresolved_style_text_prop
            + malformed_text_master_style
            + malformed_text_rulers
            + malformed_style_text_prop9
            + malformed_time_variants
            + malformed_blip_entity9
            + atom_stats
                .values()
                .map(|stats| stats.records)
                .sum::<usize>()
            + external_storage_invalid_compressed
            + external_storage_invalid_uncompressed
            + external_storage_malformed_compressed
            + external_storage_unsupported_instance
    );
    assert!(
        current_user_nonconforming_diagnostics >= current_user_compatibility,
        "every compatibility CurrentUserAtom requires a nonconforming diagnostic"
    );
    assert_eq!(invalid_stream_diagnostics, picture_partial_streams);
    assert_eq!(
        invalid_reference_diagnostics,
        current_edit_broken_links + persist_chain_failures
    );
    let atom_bytes = atom_stats.values().map(|stats| stats.bytes).sum::<usize>();
    eprintln!(
        "PPT external storage: {external_storage_atoms} atoms, {external_storage_parsed_compressed} parsed compressed/{external_storage_parsed_uncompressed} parsed uncompressed, {external_storage_invalid_compressed} invalid compressed/{external_storage_invalid_uncompressed} invalid uncompressed/{external_storage_malformed_compressed} malformed compressed/{external_storage_unsupported_instance} unsupported instance, {external_storage_entries} CFB entries/{external_storage_vba_shaped} VBA-shaped/{external_storage_vba_parsed} VBA parsed/{external_storage_vba_invalid} VBA invalid/{external_storage_vba_modules} modules"
    );
    eprintln!(
        "PPT content-master OPC: {content_master_packages} packages/{content_master_payload_bytes} bytes"
    );
    eprintln!("PPT theme OPC: {theme_packages} packages/{theme_payload_bytes} bytes");
    eprintln!(
        "PPT style OPC: {text_style_packages} slide-style/{notes_style_packages} notes-style/{table_style_packages} table-style packages/{style_payload_bytes} bytes"
    );
    eprintln!(
        "checked {checked} PowerPoint Document streams: Current User {current_user_streams} streams/{current_user_parsed} parsed/{current_user_compatibility} compatibility/{current_user_truncated} truncated, {current_edit_links} valid/{current_edit_broken_links} broken current-edit links/{current_user_trailing_bytes} trailing bytes; Current User diagnostic samples {current_user_diagnostic_samples:#?}; {persist_chains} persist chains/{persist_chain_failures} failures/{persist_chain_edits} edits/{effective_persist_objects} effective objects; {record_count} records/{container_count} containers; core atoms Document {document_atoms}/Slide {slide_atoms}/Notes {notes_atoms}, {slide_persist_atoms} persist/{color_scheme_atoms} color schemes/{placeholder_atoms} placeholders/{headers_footers_atoms} header-footer; text {outline_text_refs} outline refs/{text_headers} headers/{text_chars_records} UTF-16/{text_bytes_records} byte strings/{c_string_records} CStrings/{style_text_prop_atoms} style-prop ({style_paragraph_runs} PF runs/{style_character_runs} CF runs/{style_tab_stops} tabs/{style_trailing_bytes} trailing bytes + {malformed_style_text_prop} malformed/{malformed_style_text_prop_bytes} bytes + {unresolved_style_text_prop} unresolved/{unresolved_style_text_prop_bytes} bytes)/{master_text_prop_atoms} master-prop/{text_master_style_atoms} master-style ({text_master_style_levels} levels/{text_master_style_trailing_bytes} trailing + {malformed_text_master_style} malformed/{malformed_text_master_style_bytes} bytes)/{text_ruler_atoms} rulers ({text_ruler_tab_stops} tabs/{text_ruler_trailing_bytes} trailing + {malformed_text_rulers} malformed/{malformed_text_ruler_bytes} bytes)/{text_special_info_atoms} special-info + {malformed_text_special_info} malformed; OfficeArt {office_art_records} typed records/{office_art_bytes} bytes; Pictures {pictures_streams} streams ({picture_partial_streams} partial)/{picture_records} complete + {picture_incomplete_records} incomplete records/{picture_typed_records} typed/{picture_opaque_records} opaque/{picture_payload_bytes} payload bytes/{picture_unparsed_bytes} unparsed bytes; {user_edit_count} UserEditAtom ({encrypted_user_edits} encrypted); {persist_directory_count} PersistDirectoryAtom with {persist_directory_entries} entries/{persist_offsets} offsets; {} malformed spec types/{atom_bytes} bytes; {} unknown record types; {truncated_records} truncated records/{truncated_bytes} bytes; {trailing_header_bytes} trailing header bytes; diagnostics {truncated_diagnostics} truncated in {} files/{nonconforming_diagnostics} nonconforming ({current_user_nonconforming_diagnostics} Current User) in {} files/{invalid_stream_diagnostics} invalid stream in {} files/{invalid_reference_diagnostics} invalid reference in {} files/{noncanonical_cfb_diagnostics} noncanonical CFB in {} files",
        atom_stats.len(),
        unknown_record_stats.len(),
        truncated_diagnostic_files.len(),
        nonconforming_diagnostic_files.len(),
        invalid_stream_diagnostic_files.len(),
        invalid_reference_diagnostic_files.len(),
        noncanonical_cfb_diagnostic_files.len()
    );
    if std::env::var_os("PPT_REPORT_ATOMS").is_some() {
        let mut atoms: Vec<_> = atom_stats.iter().collect();
        atoms.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.bytes));
        for (record_type, stats) in atoms {
            eprintln!(
                "PPT atom 0x{record_type:04x}: {} records/{} bytes/lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
    }
    if std::env::var_os("PPT_REPORT_ATOM_TYPE").is_some() {
        eprintln!(
            "PPT selected atom samples:\n{}",
            selected_atom_samples.join("\n")
        );
    }
    if std::env::var_os("PPT_REPORT_EXTERNAL").is_some() {
        eprintln!(
            "PPT external storage invalid samples:\n{}",
            external_storage_invalid_samples.join("\n")
        );
    }
    if std::env::var_os("PPT_REPORT_TRUNCATED").is_some() {
        eprintln!("PPT truncated shapes: {truncated_shapes:#06x?}");
        eprintln!("PPT truncated samples:\n{}", truncated_samples.join("\n"));
        eprintln!("PPT encrypted samples:\n{}", encrypted_samples.join("\n"));
        eprintln!("PPT Current User trailing shapes: {current_user_trailing_shapes:?}");
        eprintln!(
            "PPT Current User samples:\n{}",
            current_user_samples.join("\n")
        );
        eprintln!(
            "PPT unresolved StyleTextProp samples:\n{}",
            unresolved_style_samples.join("\n")
        );
        eprintln!(
            "PPT TextMasterStyle trailing samples:\n{}",
            text_master_style_trailing_samples.join("\n")
        );
        eprintln!(
            "PPT external storage invalid samples:\n{}",
            external_storage_invalid_samples.join("\n")
        );
    }
}

struct RecordAudit<'a> {
    records: &'a mut usize,
    containers: &'a mut usize,
    prog_binary_tags: &'a mut BTreeMap<Option<ProgrammableTagKind>, usize>,
    binary_tag_data_blobs: &'a mut usize,
    binary_tag_trailing_bytes: &'a mut usize,
    document_atoms: &'a mut usize,
    slide_atoms: &'a mut usize,
    notes_atoms: &'a mut usize,
    office_art_records: &'a mut usize,
    office_art_bytes: &'a mut usize,
    outline_text_refs: &'a mut usize,
    text_headers: &'a mut usize,
    text_chars_records: &'a mut usize,
    text_chars_units: &'a mut usize,
    text_bytes_records: &'a mut usize,
    text_bytes_characters: &'a mut usize,
    style_text_prop_atoms: &'a mut usize,
    style_paragraph_runs: &'a mut usize,
    style_character_runs: &'a mut usize,
    style_tab_stops: &'a mut usize,
    style_trailing_bytes: &'a mut usize,
    malformed_style_text_prop: &'a mut usize,
    malformed_style_text_prop_bytes: &'a mut usize,
    unresolved_style_text_prop: &'a mut usize,
    unresolved_style_text_prop_bytes: &'a mut usize,
    unresolved_style_samples: &'a mut Vec<String>,
    style_text_prop9_atoms: &'a mut usize,
    style_text_prop9_runs: &'a mut usize,
    style_text_prop9_bullet_blips: &'a mut usize,
    style_text_prop9_auto_number_flags: &'a mut usize,
    style_text_prop9_auto_number_schemes: &'a mut usize,
    style_text_prop9_character_extensions: &'a mut usize,
    style_text_prop9_special_extensions: &'a mut usize,
    style_text_prop9_bidi: &'a mut usize,
    style_text_prop9_smart_tags: &'a mut usize,
    malformed_style_text_prop9: &'a mut usize,
    malformed_style_text_prop9_bytes: &'a mut usize,
    c_string_records: &'a mut usize,
    c_string_units: &'a mut usize,
    slide_persist_atoms: &'a mut usize,
    color_scheme_atoms: &'a mut usize,
    external_object_refs: &'a mut usize,
    placeholder_atoms: &'a mut usize,
    headers_footers_atoms: &'a mut usize,
    master_text_prop_atoms: &'a mut usize,
    master_text_prop_runs: &'a mut usize,
    text_master_style_atoms: &'a mut usize,
    text_master_style_levels: &'a mut usize,
    text_master_style_trailing_bytes: &'a mut usize,
    text_master_style_truncated_tails: &'a mut usize,
    text_master_style_compatibility_tails: &'a mut usize,
    text_master_style_trailing_samples: &'a mut Vec<String>,
    malformed_text_master_style: &'a mut usize,
    malformed_text_master_style_bytes: &'a mut usize,
    text_ruler_atoms: &'a mut usize,
    text_ruler_tab_stops: &'a mut usize,
    text_ruler_trailing_bytes: &'a mut usize,
    malformed_text_rulers: &'a mut usize,
    malformed_text_ruler_bytes: &'a mut usize,
    external_storage_atoms: &'a mut usize,
    external_storage_parsed_compressed: &'a mut usize,
    external_storage_parsed_uncompressed: &'a mut usize,
    external_storage_invalid_compressed: &'a mut usize,
    external_storage_invalid_uncompressed: &'a mut usize,
    external_storage_malformed_compressed: &'a mut usize,
    external_storage_unsupported_instance: &'a mut usize,
    external_storage_entries: &'a mut usize,
    external_storage_vba_shaped: &'a mut usize,
    external_storage_vba_parsed: &'a mut usize,
    external_storage_vba_invalid: &'a mut usize,
    external_storage_vba_modules: &'a mut usize,
    external_storage_invalid_samples: &'a mut Vec<String>,
    content_master_packages: &'a mut usize,
    content_master_payload_bytes: &'a mut usize,
    theme_packages: &'a mut usize,
    theme_payload_bytes: &'a mut usize,
    color_mapping_payloads: &'a mut usize,
    color_mapping_payload_bytes: &'a mut usize,
    animation_packages: &'a mut usize,
    animation_payload_bytes: &'a mut usize,
    text_style_packages: &'a mut usize,
    notes_style_packages: &'a mut usize,
    table_style_packages: &'a mut usize,
    style_payload_bytes: &'a mut usize,
    text_special_info_atoms: &'a mut usize,
    text_special_info_runs: &'a mut usize,
    text_special_info_smart_tags: &'a mut usize,
    malformed_text_special_info: &'a mut usize,
    malformed_text_special_info_bytes: &'a mut usize,
    typed_time_atoms: &'a mut BTreeMap<u16, usize>,
    malformed_time_variants: &'a mut usize,
    blip_entity9_types: &'a mut BTreeMap<u16, usize>,
    malformed_blip_entity9: &'a mut usize,
    malformed_blip_entity9_bytes: &'a mut usize,
    user_edits: &'a mut usize,
    persist_directories: &'a mut usize,
    persist_entries: &'a mut usize,
    persist_offsets: &'a mut usize,
    atoms: &'a mut BTreeMap<u16, RecordStats>,
    unknown_records: &'a mut BTreeMap<u16, RecordStats>,
    selected_atom_samples: &'a mut Vec<String>,
    truncated_records: &'a mut usize,
    truncated_bytes: &'a mut usize,
    truncated_shapes: &'a mut BTreeMap<(u16, u32, usize), usize>,
    truncated_samples: &'a mut Vec<String>,
    trailing_header_bytes: &'a mut usize,
    encrypted_user_edits: &'a mut usize,
    encrypted_samples: &'a mut Vec<String>,
    source: &'a Path,
}

fn audit_sequence(
    sequence: &olecfsdk::ppt::PptRecordSequence,
    audit: &mut RecordAudit<'_>,
    ancestry: &mut Vec<(u16, u64)>,
    programmable_tag: Option<ProgrammableTagKind>,
) {
    *audit.trailing_header_bytes += sequence.trailing_header_bytes.len();
    for (index, record) in sequence.records.iter().enumerate() {
        *audit.records += 1;
        match &record.data {
            PptRecordData::Container(children) => {
                *audit.containers += 1;
                ancestry.push((record.header.record_type, record.offset));
                audit_sequence(children, audit, ancestry, programmable_tag);
                ancestry.pop();
            }
            PptRecordData::ProgBinaryTag(value) => {
                *audit.containers += 1;
                *audit.prog_binary_tags.entry(value.tag_kind()).or_default() += 1;
                ancestry.push((record.header.record_type, record.offset));
                audit_sequence(&value.records, audit, ancestry, value.tag_kind());
                ancestry.pop();
            }
            PptRecordData::ProgTags(children) => {
                *audit.containers += 1;
                ancestry.push((record.header.record_type, record.offset));
                audit_sequence(children, audit, ancestry, programmable_tag);
                ancestry.pop();
            }
            PptRecordData::BinaryTagData(olecfsdk::ppt::BinaryTagData::Records(children)) => {
                *audit.binary_tag_data_blobs += 1;
                *audit.binary_tag_trailing_bytes += children.trailing_header_bytes.len();
                ancestry.push((record.header.record_type, record.offset));
                audit_sequence(children, audit, ancestry, programmable_tag);
                ancestry.pop();
            }
            PptRecordData::BinaryTagData(olecfsdk::ppt::BinaryTagData::Opaque(_)) => {
                *audit.binary_tag_data_blobs += 1;
            }
            PptRecordData::Document(_) => *audit.document_atoms += 1,
            PptRecordData::Slide(_) => *audit.slide_atoms += 1,
            PptRecordData::Notes(_) => *audit.notes_atoms += 1,
            PptRecordData::OfficeArt(_) => {
                *audit.office_art_records += 1;
                *audit.office_art_bytes += record.header.declared_length as usize;
            }
            PptRecordData::OutlineTextRef(_) => *audit.outline_text_refs += 1,
            PptRecordData::TextHeader(_) => *audit.text_headers += 1,
            PptRecordData::TextChars(values) => {
                *audit.text_chars_records += 1;
                *audit.text_chars_units += values.len();
            }
            PptRecordData::TextBytes(values) => {
                *audit.text_bytes_records += 1;
                *audit.text_bytes_characters += values.len();
            }
            PptRecordData::StyleTextProp(value) => {
                *audit.style_text_prop_atoms += 1;
                *audit.style_paragraph_runs += value.paragraph_runs.len();
                *audit.style_character_runs += value.character_runs.len();
                *audit.style_tab_stops += value
                    .paragraph_runs
                    .iter()
                    .filter_map(|run| run.properties.tab_stops.as_ref())
                    .map(Vec::len)
                    .sum::<usize>();
                *audit.style_trailing_bytes += value.trailing.len();
            }
            PptRecordData::MalformedStyleTextProp(value) => {
                *audit.malformed_style_text_prop += 1;
                *audit.malformed_style_text_prop_bytes += value.body.len();
            }
            PptRecordData::UnresolvedStyleTextProp(bytes) => {
                *audit.unresolved_style_text_prop += 1;
                *audit.unresolved_style_text_prop_bytes += bytes.len();
                if audit.unresolved_style_samples.len() < 100 {
                    let preceding: Vec<_> = sequence.records[..index]
                        .iter()
                        .rev()
                        .take(5)
                        .map(|record| format!("0x{:04x}", record.header.record_type))
                        .collect();
                    audit.unresolved_style_samples.push(format!(
                        "{} offset {} len {} head {:02x?} preceding {:?}",
                        audit.source.display(),
                        record.offset,
                        bytes.len(),
                        &bytes[..bytes.len().min(32)],
                        preceding
                    ));
                }
            }
            PptRecordData::StyleTextProp9(value) => {
                *audit.style_text_prop9_atoms += 1;
                *audit.style_text_prop9_runs += value.runs.len();
                for run in &value.runs {
                    *audit.style_text_prop9_bullet_blips +=
                        usize::from(run.paragraph.bullet_blip_ref.is_some());
                    *audit.style_text_prop9_auto_number_flags +=
                        usize::from(run.paragraph.bullet_has_auto_number.is_some());
                    *audit.style_text_prop9_auto_number_schemes +=
                        usize::from(run.paragraph.auto_number_scheme.is_some());
                    *audit.style_text_prop9_character_extensions +=
                        usize::from(run.character.pp10_extension.is_some());
                    *audit.style_text_prop9_special_extensions +=
                        usize::from(run.special_info.pp10_extension.is_some());
                    *audit.style_text_prop9_bidi += usize::from(run.special_info.bidi.is_some());
                    *audit.style_text_prop9_smart_tags += run
                        .special_info
                        .smart_tag_indices
                        .as_ref()
                        .map_or(0, Vec::len);
                }
            }
            PptRecordData::MalformedStyleTextProp9(bytes) => {
                *audit.malformed_style_text_prop9 += 1;
                *audit.malformed_style_text_prop9_bytes += bytes.len();
            }
            PptRecordData::CString(values) => {
                *audit.c_string_records += 1;
                *audit.c_string_units += values.len();
            }
            PptRecordData::SlidePersist(_) => *audit.slide_persist_atoms += 1,
            PptRecordData::ColorScheme(_) => *audit.color_scheme_atoms += 1,
            PptRecordData::ExternalObjectRef(_) => *audit.external_object_refs += 1,
            PptRecordData::Placeholder(_) => *audit.placeholder_atoms += 1,
            PptRecordData::HeadersFooters(_) => *audit.headers_footers_atoms += 1,
            PptRecordData::MasterTextProp(value) => {
                *audit.master_text_prop_atoms += 1;
                *audit.master_text_prop_runs += value.runs.len();
            }
            PptRecordData::TextMasterStyle(value) => {
                *audit.text_master_style_atoms += 1;
                *audit.text_master_style_levels += value.levels.len();
                *audit.text_master_style_trailing_bytes += value.tail.physical_len();
                match &value.tail {
                    olecfsdk::ppt::TextMasterStyleTail::None => {}
                    olecfsdk::ppt::TextMasterStyleTail::TruncatedRecord { .. } => {
                        *audit.text_master_style_truncated_tails += 1;
                    }
                    olecfsdk::ppt::TextMasterStyleTail::Compatibility(_) => {
                        *audit.text_master_style_compatibility_tails += 1;
                    }
                }
                if value.tail.physical_len() != 0
                    && audit.text_master_style_trailing_samples.len() < 100
                {
                    audit.text_master_style_trailing_samples.push(format!(
                        "{} offset {} instance {} levels {} tail {:?}",
                        audit.source.display(),
                        record.offset,
                        record.header.instance,
                        value.levels.len(),
                        value.tail
                    ));
                }
            }
            PptRecordData::MalformedTextMasterStyle(bytes) => {
                *audit.malformed_text_master_style += 1;
                *audit.malformed_text_master_style_bytes += bytes.len();
            }
            PptRecordData::TextRuler(value) => {
                *audit.text_ruler_atoms += 1;
                *audit.text_ruler_tab_stops += value.tab_stops.as_ref().map_or(0, Vec::len);
                *audit.text_ruler_trailing_bytes += value.trailing.len();
            }
            PptRecordData::MalformedTextRuler(bytes) => {
                *audit.malformed_text_rulers += 1;
                *audit.malformed_text_ruler_bytes += bytes.len();
            }
            PptRecordData::ExternalStorage(value) => {
                use olecfsdk::ppt::{
                    ExternalStorageAtom, ExternalStorageEncoding, ExternalStorageVba,
                };
                *audit.external_storage_atoms += 1;
                match value {
                    ExternalStorageAtom::Parsed(storage) => {
                        match &storage.encoding {
                            ExternalStorageEncoding::Uncompressed => {
                                *audit.external_storage_parsed_uncompressed += 1;
                            }
                            ExternalStorageEncoding::Zlib { .. } => {
                                *audit.external_storage_parsed_compressed += 1;
                            }
                        }
                        *audit.external_storage_entries += storage.compound_file.entries().len();
                        if storage.compound_file.entries().iter().any(|entry| {
                            entry.name.eq_ignore_ascii_case("VBA")
                                || entry.name.eq_ignore_ascii_case("PROJECT")
                        }) {
                            *audit.external_storage_vba_shaped += 1;
                        }
                        match &storage.vba_project {
                            ExternalStorageVba::NotPresent => {}
                            ExternalStorageVba::Parsed(project) => {
                                *audit.external_storage_vba_parsed += 1;
                                *audit.external_storage_vba_modules += project.modules.len();
                            }
                            ExternalStorageVba::Invalid(reason) => {
                                *audit.external_storage_vba_invalid += 1;
                                if audit.external_storage_invalid_samples.len() < 100 {
                                    audit.external_storage_invalid_samples.push(format!(
                                        "{} offset {} VBA: {reason}",
                                        audit.source.display(),
                                        record.offset
                                    ));
                                }
                            }
                        }
                    }
                    ExternalStorageAtom::InvalidCompressed { reason, .. } => {
                        *audit.external_storage_invalid_compressed += 1;
                        if audit.external_storage_invalid_samples.len() < 100 {
                            audit.external_storage_invalid_samples.push(format!(
                                "{} offset {} compressed: {reason}",
                                audit.source.display(),
                                record.offset
                            ));
                        }
                    }
                    ExternalStorageAtom::InvalidUncompressed { reason, .. } => {
                        *audit.external_storage_invalid_uncompressed += 1;
                        if audit.external_storage_invalid_samples.len() < 100 {
                            audit.external_storage_invalid_samples.push(format!(
                                "{} offset {} uncompressed: {reason}",
                                audit.source.display(),
                                record.offset
                            ));
                        }
                    }
                    ExternalStorageAtom::MalformedCompressed { reason, .. } => {
                        *audit.external_storage_malformed_compressed += 1;
                        if audit.external_storage_invalid_samples.len() < 100 {
                            audit.external_storage_invalid_samples.push(format!(
                                "{} offset {} malformed: {reason}",
                                audit.source.display(),
                                record.offset
                            ));
                        }
                    }
                    ExternalStorageAtom::UnsupportedInstance { .. } => {
                        *audit.external_storage_unsupported_instance += 1;
                    }
                }
            }
            PptRecordData::RoundTripContentMasterInfo12(value) => {
                *audit.content_master_packages += 1;
                *audit.content_master_payload_bytes += value.package.physical_bytes.len();
            }
            PptRecordData::RoundTripColorMapping12(value) => {
                *audit.color_mapping_payloads += 1;
                *audit.color_mapping_payload_bytes += value.physical_xml.len();
            }
            PptRecordData::RoundTripAnimation12(value) => {
                *audit.animation_packages += 1;
                *audit.animation_payload_bytes += value.package.physical_bytes.len();
            }
            PptRecordData::RoundTripTheme12(value) => {
                *audit.theme_packages += 1;
                *audit.theme_payload_bytes += value.package.physical_bytes.len();
            }
            PptRecordData::RoundTripStyle12(value) => {
                match value.record_type {
                    ROUND_TRIP_NOTES_MASTER_TEXT_STYLES_12_ATOM => {
                        *audit.notes_style_packages += 1;
                    }
                    olecfsdk::ppt::ROUND_TRIP_OART_TEXT_STYLES_12_ATOM => {
                        *audit.text_style_packages += 1;
                    }
                    olecfsdk::ppt::ROUND_TRIP_CUSTOM_TABLE_STYLES_12_ATOM => {
                        *audit.table_style_packages += 1;
                    }
                    _ => unreachable!(),
                }
                *audit.style_payload_bytes += value.package.physical_bytes.len();
            }
            PptRecordData::TextSpecialInfo(value) => {
                *audit.text_special_info_atoms += 1;
                *audit.text_special_info_runs += value.runs.len();
                *audit.text_special_info_smart_tags += value
                    .runs
                    .iter()
                    .filter_map(|run| run.smart_tag_indices.as_ref())
                    .map(Vec::len)
                    .sum::<usize>();
            }
            PptRecordData::MalformedTextSpecialInfo(bytes) => {
                *audit.malformed_text_special_info += 1;
                *audit.malformed_text_special_info_bytes += bytes.len();
            }
            PptRecordData::TimeNode(_)
            | PptRecordData::TimeCondition(_)
            | PptRecordData::TimeModifier(_)
            | PptRecordData::TimeBehavior(_)
            | PptRecordData::TimeAnimateBehavior(_)
            | PptRecordData::TimeEffectBehavior(_)
            | PptRecordData::TimeMotionBehavior(_)
            | PptRecordData::TimeScaleBehavior(_)
            | PptRecordData::TimeSetBehavior(_)
            | PptRecordData::TimeCommandBehavior(_)
            | PptRecordData::TimeSequenceData(_)
            | PptRecordData::TimeAnimationValue(_)
            | PptRecordData::TimeVariant(_)
            | PptRecordData::VisualShape(_)
            | PptRecordData::HashCode(_)
            | PptRecordData::VisualPage(_)
            | PptRecordData::Build(_)
            | PptRecordData::ParaBuild(_)
            | PptRecordData::LevelInfo(_)
            | PptRecordData::SlideTime10(_)
            | PptRecordData::FontEntity(_)
            | PptRecordData::ExternalOleObject(_)
            | PptRecordData::ExternalOleEmbed(_)
            | PptRecordData::Kinsoku(_)
            | PptRecordData::ExternalHyperlinkId(_)
            | PptRecordData::ExternalHyperlinkFlags(_)
            | PptRecordData::SlideNumberMeta(_)
            | PptRecordData::TextInteractiveInfo(_)
            | PptRecordData::AnimationInfo(_)
            | PptRecordData::InteractiveInfo(_)
            | PptRecordData::DateTimeMeta(_)
            | PptRecordData::GenericDateMeta(_)
            | PptRecordData::HeaderMeta(_)
            | PptRecordData::FooterMeta(_)
            | PptRecordData::SlideShowSlideInfo(_)
            | PptRecordData::Guide(_)
            | PptRecordData::SlideViewInfo(_)
            | PptRecordData::VbaInfo(_)
            | PptRecordData::SlideShowDocInfo(_)
            | PptRecordData::ExternalObjectList(_)
            | PptRecordData::GridSpacing10(_)
            | PptRecordData::NormalViewSetInfo9(_)
            | PptRecordData::RoundTripOriginalMainMasterId12(_)
            | PptRecordData::RoundTripCompositeMasterId12(_)
            | PptRecordData::RoundTripShapeId12(_)
            | PptRecordData::RoundTripHfPlaceholder12(_)
            | PptRecordData::RoundTripContentMasterId12(_)
            | PptRecordData::RoundTripHeaderFooterDefaults12(_)
            | PptRecordData::RoundTripDocFlags12(_)
            | PptRecordData::RoundTripShapeChecksum12(_)
            | PptRecordData::EndDocument
            | PptRecordData::SoundCollection(_)
            | PptRecordData::SoundDataBlob(_)
            | PptRecordData::TextBookmark(_)
            | PptRecordData::TextCfException(_)
            | PptRecordData::TextPfException(_)
            | PptRecordData::TextSiException(_)
            | PptRecordData::TextMasterStyle9(_)
            | PptRecordData::StyleTextProp10(_)
            | PptRecordData::TextMasterStyle10(_)
            | PptRecordData::TextDefaults10(_)
            | PptRecordData::StyleTextProp11(_)
            | PptRecordData::RecolorInfo(_)
            | PptRecordData::MacPrintSettings(_)
            | PptRecordData::MacPageFormat(_)
            | PptRecordData::Ppt11FontDescriptors(_)
            | PptRecordData::Ppt11FontDescriptorCollection(_)
            | PptRecordData::Ppt10Reserved(_)
            | PptRecordData::MacLegacyPrintInfo(_)
            | PptRecordData::MacPrintDriverInfo(_)
            | PptRecordData::HandoutCompatibility(_)
            | PptRecordData::NamedShowSlides(_)
            | PptRecordData::BookmarkSeed(_)
            | PptRecordData::ShapeFlags(_)
            | PptRecordData::ShapeFlags10(_)
            | PptRecordData::RoundTripNewPlaceholderId12(_)
            | PptRecordData::FontEmbedDataBlob(_)
            | PptRecordData::BookmarkEntity(_)
            | PptRecordData::RtfDateTimeMeta(_)
            | PptRecordData::ChartBuild(_)
            | PptRecordData::DiagramBuild(_)
            | PptRecordData::LinkedShape10(_)
            | PptRecordData::LinkedSlide10(_)
            | PptRecordData::Diff10(_)
            | PptRecordData::SlideListTableSize10(_)
            | PptRecordData::SlideListEntry10(_)
            | PptRecordData::FontEmbedFlags10(_)
            | PptRecordData::PhotoAlbumInfo10(_)
            | PptRecordData::TimeIterateData(_)
            | PptRecordData::TextDefaults9(_)
            | PptRecordData::ExternalOleLink(_)
            | PptRecordData::ExternalOleControl(_)
            | PptRecordData::ExternalCdAudio(_)
            | PptRecordData::BroadcastDocInfo9(_)
            | PptRecordData::EnvelopeFlags9(_)
            | PptRecordData::EnvelopeData9(_)
            | PptRecordData::DocRoutingSlip(_)
            | PptRecordData::Metafile(_)
            | PptRecordData::RoundTripSlideSyncInfo12(_)
            | PptRecordData::TimeColorBehavior(_)
            | PptRecordData::TimeRotationBehavior(_)
            | PptRecordData::OutlineTextPropsHeader9(_)
            | PptRecordData::ExternalMedia(_)
            | PptRecordData::ExternalWavAudioEmbedded(_)
            | PptRecordData::PrintOptions(_)
            | PptRecordData::PresentationAdvisorFlags9(_)
            | PptRecordData::HtmlDocInfo9(_)
            | PptRecordData::HtmlPublishInfo(_)
            | PptRecordData::Comment10(_)
            | PptRecordData::CommentIndex10(_)
            | PptRecordData::SlideFlags10(_)
            | PptRecordData::FilterPrivacyFlags10(_)
            | PptRecordData::DocToolbarStates10(_) => {
                *audit
                    .typed_time_atoms
                    .entry(record.header.record_type)
                    .or_default() += 1;
            }
            PptRecordData::ViewInfo(_) => {
                *audit
                    .typed_time_atoms
                    .entry(record.header.record_type)
                    .or_default() += 1;
            }
            PptRecordData::BlipEntity9(value) => {
                *audit
                    .typed_time_atoms
                    .entry(record.header.record_type)
                    .or_default() += 1;
                *audit
                    .blip_entity9_types
                    .entry(value.blip.header.record_type)
                    .or_default() += 1;
            }
            PptRecordData::MalformedBlipEntity9 { body, .. } => {
                *audit.malformed_blip_entity9 += 1;
                *audit.malformed_blip_entity9_bytes += body.len();
            }
            PptRecordData::MalformedTimeVariant(_) => *audit.malformed_time_variants += 1,
            PptRecordData::RoundTripAnimationHash12(_) => {
                *audit
                    .typed_time_atoms
                    .entry(record.header.record_type)
                    .or_default() += 1;
            }
            PptRecordData::UserEdit(value) => {
                *audit.user_edits += 1;
                if value.encrypt_session_persist_id_ref.is_some() {
                    *audit.encrypted_user_edits += 1;
                    if audit.encrypted_samples.len() < 100 {
                        audit.encrypted_samples.push(format!(
                            "{} offset {} encrypt persist {:?}",
                            audit.source.display(),
                            record.offset,
                            value.encrypt_session_persist_id_ref
                        ));
                    }
                }
            }
            PptRecordData::PersistDirectory(value) => {
                *audit.persist_directories += 1;
                *audit.persist_entries += value.entries.len();
                *audit.persist_offsets += value
                    .entries
                    .iter()
                    .map(|entry| entry.stream_offsets.len())
                    .sum::<usize>();
            }
            PptRecordData::MalformedSpecRecord(value) => {
                let stats = audit.atoms.entry(record.header.record_type).or_default();
                stats.records += 1;
                stats.bytes += value.body.len();
                stats.lengths.insert(value.body.len());
                let selected_type = std::env::var("PPT_REPORT_ATOM_TYPE")
                    .ok()
                    .and_then(|value| u16::from_str_radix(value.trim_start_matches("0x"), 16).ok());
                if selected_type == Some(record.header.record_type)
                    && audit.selected_atom_samples.len() < 20
                {
                    let preceding: Vec<_> = sequence.records[..index]
                        .iter()
                        .rev()
                        .take(5)
                        .map(|record| format!("0x{:04x}", record.header.record_type))
                        .collect();
                    audit.selected_atom_samples.push(format!(
                        "{} offset {} instance {} len {} head {:02x?} preceding {:?} ancestry {:?} tag {:?}",
                        audit.source.display(),
                        record.offset,
                        record.header.instance,
                        value.body.len(),
                        &value.body[..value.body.len().min(64)],
                        preceding,
                        ancestry,
                        programmable_tag
                    ));
                }
            }
            PptRecordData::Unknown(value) => {
                let stats = audit.unknown_records.entry(value.record_type).or_default();
                stats.records += 1;
                stats.bytes += value.body.len();
                stats.lengths.insert(value.body.len());
            }
            PptRecordData::Truncated(bytes) => {
                *audit.truncated_records += 1;
                *audit.truncated_bytes += bytes.len();
                *audit
                    .truncated_shapes
                    .entry((
                        record.header.record_type,
                        record.header.declared_length,
                        bytes.len(),
                    ))
                    .or_default() += 1;
                if audit.truncated_samples.len() < 100 {
                    audit.truncated_samples.push(format!(
                        "{} offset {} type 0x{:04x} declared {} available {} head {:02x?}",
                        audit.source.display(),
                        record.offset,
                        record.header.record_type,
                        record.header.declared_length,
                        bytes.len(),
                        &bytes[..bytes.len().min(32)]
                    ));
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct RecordStats {
    records: usize,
    bytes: usize,
    lengths: BTreeSet<usize>,
}

fn excluded_files(corpus: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    for name in ["Apache-POI", "LibreOffice"] {
        let root = corpus.join(name);
        let manifest = read_manifest(&root.join("manifest.toml")).expect("read corpus manifest");
        for expectation in manifest.expectation {
            if matches!(
                expectation.test.as_str(),
                "cfb_roundtrip" | "ppt_record_roundtrip"
            ) && matches!(
                expectation.mode,
                ExpectationMode::Invalid
                    | ExpectationMode::Unsupported
                    | ExpectationMode::RequiresPassword
                    | ExpectationMode::KnownFailure
            ) && expectation
                .file
                .rsplit_once('.')
                .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("ppt"))
            {
                files.insert(root.join(expectation.file));
            }
        }
    }
    files
}

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read corpus directory") {
        let path = entry.expect("read corpus entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("ppt"))
        {
            files.push(path);
        }
    }
}

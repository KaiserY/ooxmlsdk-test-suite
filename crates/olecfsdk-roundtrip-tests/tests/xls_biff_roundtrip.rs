use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use olecfsdk::{
    ParseDiagnosticCode,
    cfb::CompoundFile,
    office_art::{
        OfficeArtBitmapData, OfficeArtComplexPropertyData, OfficeArtDrawingGraphIssue,
        OfficeArtIncompletePropertyEntry, OfficeArtIncompletePropertyTable,
        OfficeArtIncompleteRecordData, OfficeArtMetafileData, OfficeArtMetafileOpaqueReason,
        OfficeArtPropertyValue, OfficeArtRecord, OfficeArtRecordData, SoftMakerNativePropertyData,
    },
    xls::{
        AutoFilterOperandValue, BiffRecordData, BkHimImage, Cf12ConditionData, DConnConnection,
        DevModeWPublic, DxfN12, ExtPropertyData, ExtRstBody, ExternNameBody, FeatureData,
        FeatureHeaderData, HyperlinkObject, LhSubrecordData, MsoDrawingData, MsoDrawingHostData,
        ObjFormulaData, ObjSubrecordData, ParamQryData, PrinterSettings, RtdOperation,
        SortFieldParent, SstCompletion, SstExtensionData, SupBookLink, TxoContext, XfPropertyData,
        XlStringCharacters, XlsFile, XmlTkData,
    },
};
use olecfsdk_corpus_test_support::manifest::{ExpectationMode, read_manifest};

#[test]
#[ignore = "XLS BIFF corpus round-trip runs explicitly"]
fn legacy_office_workbook_streams_round_trip() {
    let corpus = olecfsdk_corpus_test_support::corpus_root();
    let mut files = Vec::new();
    collect(&corpus.join("Apache-POI"), &mut files);
    collect(&corpus.join("LibreOffice"), &mut files);
    let expected_invalid = expected_invalid_files(&corpus);
    let mut observed_invalid = BTreeSet::new();
    let mut checked = 0usize;
    let mut root_parsed_files = 0usize;
    let mut root_reopened_files = 0usize;
    let mut biff8 = 0usize;
    let mut legacy = 0usize;
    let mut unknown_types = BTreeSet::new();
    let mut unknown_stats = BTreeMap::<u16, UnknownStats>::new();
    let mut unknown_samples = BTreeMap::<u16, Vec<String>>::new();
    let mut newly_static_formal_records = BTreeMap::<&'static str, usize>::new();
    let mut null_compatibility_records = 0usize;
    let mut lh_records = 0usize;
    let mut lh_subrecords = 0usize;
    let mut lh_margins = 0usize;
    let mut lh_graph_views = 0usize;
    let mut lh_graph_compatibility_extensions = 0usize;
    let mut lh_reserved_words = 0usize;
    let mut lh_undocumented_type13 = 0usize;
    let mut font_id_compatibility_records = 0usize;
    let mut xf_id_compatibility_records = 0usize;
    let mut bound_sheet_id_compatibility_records = 0usize;
    let mut chart_series_id_compatibility_records = 0usize;
    let mut obj_id_compatibility_records = 0usize;
    let mut drawing_id_compatibility_segments = 0usize;
    let mut pls_samples = Vec::new();
    let mut pls_records = 0usize;
    let mut pls_continued = 0usize;
    let mut pls_windows_full = 0usize;
    let mut pls_windows_truncated = 0usize;
    let mut pls_windows_legacy212 = 0usize;
    let mut pls_windows_core100 = 0usize;
    let mut pls_platform_specific = 0usize;
    let mut pls_driver_extra_bytes = 0usize;
    let mut pls_truncated_driver_extra = 0usize;
    let mut pls_truncated_driver_extra_samples = Vec::new();
    let mut pls_public_extension_bytes = 0usize;
    let mut pls_truncated_public_bytes = 0usize;
    let mut pls_platform_specific_bytes = 0usize;
    let mut pls_trailing_bytes = 0usize;
    let mut pls_truncated_sizes = BTreeMap::<u16, usize>::new();
    let mut pls_platform_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut bk_him_records = 0usize;
    let mut bk_him_bitmaps = 0usize;
    let mut bk_him_native = 0usize;
    let mut bk_him_image_bytes = 0usize;
    let mut bk_him_continued = 0usize;
    let mut im_data_records = 0usize;
    let mut im_data_bitmap_bytes = 0usize;
    let mut real_time_data_records = 0usize;
    let mut real_time_data_malformed = 0usize;
    let mut real_time_data_corrupt_error_discriminators = 0usize;
    let mut real_time_data_cells = 0usize;
    let mut real_time_data_topic_segments = 0usize;
    let mut real_time_data_malformed_bytes = 0usize;
    let mut real_time_data_malformed_samples = Vec::new();
    let mut sort_records = 0usize;
    let mut sort_keys = 0usize;
    let mut sort_compressed_keys = 0usize;
    let mut sort_data_records = 0usize;
    let mut sort_data_conditions = 0usize;
    let mut sort_data_table = 0usize;
    let mut sort_data_auto_filter = 0usize;
    let mut sort_conditions = 0usize;
    let mut sort_conditions_descending = 0usize;
    let mut auto_filter_records = 0usize;
    let mut auto_filter_strings = 0usize;
    let mut auto_filter_string_characters = 0usize;
    let mut auto_filter_numbers = 0usize;
    let mut sx_format_records = 0usize;
    let mut sx_format_applied = 0usize;
    let mut wopt_records = 0usize;
    let mut wopt_component_characters = 0usize;
    let mut wopt_future_bytes = 0usize;
    let mut table_records = 0usize;
    let mut table_two_variable = 0usize;
    let mut table_compatibility_padding = 0usize;
    let mut standalone_obj_records = 0usize;
    let mut standalone_obj_raw_subrecords = 0usize;
    let mut standalone_obj_raw_shapes = BTreeMap::<(u16, usize), usize>::new();
    let mut standalone_obj_truncated_picture_flags = 0usize;
    let mut standalone_obj_trailing_bytes = 0usize;
    let mut standalone_obj_trailing_samples = Vec::new();
    let mut formula4_compatibility_records = 0usize;
    let mut formula4_unparsed_bytes = 0usize;
    let mut extern_count_records = 0usize;
    let mut qsi_records = 0usize;
    let mut qsi_name_characters = 0usize;
    let mut param_qry_records = 0usize;
    let mut param_qry_prompts = 0usize;
    let mut sx_select_records = 0usize;
    let mut sx_select_extended = 0usize;
    let mut standalone_txo_records = 0usize;
    let mut standalone_txo_undetermined = 0usize;
    let mut plv_mac_records = 0usize;
    let mut lnext_records = 0usize;
    let mut mkr_ext_records = 0usize;
    let mut crt_co_opt_records = 0usize;
    let mut crt_co_opt_compatibility_padding = 0usize;
    let mut frt_arch_id_records = 0usize;
    let mut frt_arch_ids = BTreeMap::<u32, usize>::new();
    let mut drawing_group_records = 0usize;
    let mut drawing_graphs = 0usize;
    let mut drawing_graphs_strict = 0usize;
    let mut drawing_graph_drawings = 0usize;
    let mut drawing_graph_shapes = 0usize;
    let mut drawing_graph_absent = 0usize;
    let mut drawing_graph_errors = BTreeMap::<String, usize>::new();
    let mut drawing_graph_issues = BTreeMap::<&'static str, usize>::new();
    let mut drawing_graph_shape_count_bases = BTreeMap::<String, usize>::new();
    let mut drawing_graph_current_shape_id_relations = BTreeMap::<String, usize>::new();
    let mut drawing_graph_maximum_shape_id_relations = BTreeMap::<String, usize>::new();
    let mut drawing_graph_saved_shape_count_relations = BTreeMap::<String, usize>::new();
    let mut drawing_graph_saved_drawing_count_relations = BTreeMap::<String, usize>::new();
    let mut drawing_graph_cluster_cursor_relations = BTreeMap::<String, usize>::new();
    let mut drawing_group_continued = 0usize;
    let mut drawing_group_complete = 0usize;
    let mut drawing_group_partial = 0usize;
    let mut drawing_group_partial_complete_records = 0usize;
    let mut drawing_group_partial_incomplete_records = 0usize;
    let mut drawing_group_partial_unparsed_bytes = 0usize;
    let mut drawing_group_underreported_fbse = 0usize;
    let mut drawing_group_partial_leaf_shapes = BTreeMap::<(u16, u16, u32, usize), usize>::new();
    let mut drawing_group_partial_leaf_samples = Vec::new();
    let mut drawing_group_partial_node_samples = Vec::new();
    let mut drawing_group_partial_header_tail_lengths = BTreeMap::<usize, usize>::new();
    let mut drawing_group_incomplete_bytes = 0usize;
    let mut drawing_group_incomplete = Vec::new();
    let mut drawing_records = 0usize;
    let mut drawing_segments = 0usize;
    let mut drawing_host_records = 0usize;
    let mut drawing_complete = 0usize;
    let mut drawing_partial = 0usize;
    let mut drawing_partial_complete_records = 0usize;
    let mut drawing_partial_incomplete_records = 0usize;
    let mut drawing_partial_unparsed_bytes = 0usize;
    let mut drawing_partial_boundaries = BTreeMap::<Option<u16>, usize>::new();
    let mut drawing_partial_leaf_shapes = BTreeMap::<(u16, u16, u32, usize), usize>::new();
    let mut drawing_partial_leaf_samples = Vec::new();
    let mut drawing_partial_header_tail_lengths = BTreeMap::<usize, usize>::new();
    let mut drawing_partial_unparsed_prefix_bytes = 0usize;
    let mut drawing_incomplete_bytes = 0usize;
    let mut drawing_incomplete = Vec::new();
    let mut drawing_incomplete_boundaries = BTreeMap::<Option<u16>, usize>::new();
    let mut drawing_txo_typed = 0usize;
    let mut drawing_txo_raw = 0usize;
    let mut drawing_txo_formula_bytes = 0usize;
    let mut drawing_txo_formula_shapes = BTreeMap::<usize, usize>::new();
    let mut drawing_txo_formula_typed = 0usize;
    let mut drawing_txo_formula_opaque = 0usize;
    let mut drawing_txo_trailing_bytes = 0usize;
    let mut drawing_txo_control_contexts = 0usize;
    let mut drawing_txo_reserved_contexts = 0usize;
    let mut drawing_txo_undetermined_contexts = 0usize;
    let mut drawing_note_typed = 0usize;
    let mut drawing_note_raw = 0usize;
    let mut drawing_obj_raw = 0usize;
    let mut drawing_obj_raw_bytes = 0usize;
    let mut drawing_obj_typed = 0usize;
    let mut drawing_obj_subrecord_raw = BTreeMap::<u16, UnknownStats>::new();
    let mut obj_picture_formulas = 0usize;
    let mut obj_picture_embed_info = 0usize;
    let mut obj_picture_positions = 0usize;
    let mut obj_picture_control_stream_sizes = 0usize;
    let mut obj_picture_keys = 0usize;
    let mut obj_picture_compatibility_bytes = 0usize;
    let mut office_art_compatibility_containers = BTreeMap::<u16, usize>::new();
    let mut office_art_incomplete_property_tables = 0usize;
    let mut office_art_incomplete_property_entries = 0usize;
    let mut office_art_incomplete_fixed_bytes = 0usize;
    let mut office_art_incomplete_low_words = 0usize;
    let mut office_art_incomplete_complex_bytes = 0usize;
    let mut office_art_incomplete_complex_unparsed_bytes = 0usize;
    let mut office_art_incomplete_array_fragments = 0usize;
    let mut office_art_incomplete_generic_fragment_bytes = 0usize;
    let mut office_art_incomplete_property_samples = Vec::new();
    let mut office_art_compatibility_property_tables = 0usize;
    let mut office_art_compatibility_anchors = BTreeMap::<u16, usize>::new();
    let mut office_art_compatibility_fbse = 0usize;
    let mut office_art_empty_compatibility_atoms = BTreeMap::<u16, usize>::new();
    let mut office_art_softmaker_native_records = 0usize;
    let mut office_art_softmaker_native_properties = 0usize;
    let mut office_art_softmaker_native_payload_bytes = 0usize;
    let mut office_art_softmaker_native_unparsed_bytes = 0usize;
    let mut office_art_softmaker_selector6 = 0usize;
    let mut office_art_softmaker_native_shapes = BTreeMap::<(u16, u16, usize), usize>::new();
    let mut office_art_atom_stats = BTreeMap::<u16, UnknownStats>::new();
    let mut office_art_atom_locations = Vec::new();
    let mut office_art_simple_properties = BTreeMap::<u16, usize>::new();
    let mut office_art_complex_properties = BTreeMap::<u16, UnknownStats>::new();
    let mut office_art_utf16_properties = BTreeMap::<u16, UnknownStats>::new();
    let mut office_art_empty_complex_properties = BTreeMap::<u16, usize>::new();
    let mut office_art_metro_blobs = UnknownStats::default();
    let mut office_art_metro_blob_entries = 0usize;
    let mut office_art_hyperlinks = UnknownStats::default();
    let mut office_art_hyperlink_nonparsed = 0usize;
    let mut office_art_hyperlink_trailing_bytes = 0usize;
    let mut office_art_array_headers = BTreeMap::<(u16, u16, u16, u16, u32, usize), usize>::new();
    let mut office_art_property_table_trailing_bytes = 0usize;
    let mut office_art_emf_typed = 0usize;
    let mut office_art_wmf_typed = 0usize;
    let mut office_art_pict_typed = 0usize;
    let mut office_art_dib_typed = 0usize;
    let mut office_art_dib_opaque = 0usize;
    let mut office_art_metafile_opaque = BTreeMap::<u16, UnknownStats>::new();
    let mut office_art_metafile_opaque_reasons =
        BTreeMap::<OfficeArtMetafileOpaqueReason, UnknownStats>::new();
    let mut formula_unparsed_rgce_bytes = 0usize;
    let mut formula_rgcb_bytes = 0usize;
    let mut formula_missing_extra = 0usize;
    let mut formula_tail_locations = Vec::new();
    let mut shared_formula_records = 0usize;
    let mut shared_formula_unparsed_rgce_bytes = 0usize;
    let mut shared_formula_rgcb_bytes = 0usize;
    let mut array_records = 0usize;
    let mut array_unparsed_rgce_bytes = 0usize;
    let mut array_rgcb_bytes = 0usize;
    let mut sup_book_records = 0usize;
    let mut sup_book_compatibility = 0usize;
    let mut sup_book_trailing_bytes = 0usize;
    let mut extern_name_records = 0usize;
    let mut extern_name_formula_records = 0usize;
    let mut extern_name_cached_link_records = 0usize;
    let mut extern_name_compatibility_records = 0usize;
    let mut extern_name_compatibility_bytes = 0usize;
    let mut extern_name_compatibility_samples = Vec::new();
    let mut hyperlink_records = 0usize;
    let mut hyperlink_compatibility_records = 0usize;
    let mut hyperlink_compatibility_bytes = 0usize;
    let mut hyperlink_trailing_bytes = 0usize;
    let mut hyperlink_truncated_records = 0usize;
    let mut hyperlink_truncated_bytes = 0usize;
    let mut hyperlink_truncated_url_records = 0usize;
    let mut hyperlink_truncated_url_address_bytes = 0usize;
    let mut hyperlink_truncated_samples = Vec::new();
    let mut hyperlink_compatibility_samples = Vec::new();
    let mut data_validation_records = 0usize;
    let mut data_validation_unparsed_rgce_bytes = 0usize;
    let mut data_validation_missing_extra = 0usize;
    let mut conditional_formatting_records = 0usize;
    let mut conditional_formatting_unparsed_rgce_bytes = 0usize;
    let mut conditional_formatting_missing_extra = 0usize;
    let mut linked_data_records = 0usize;
    let mut linked_data_unparsed_rgce_bytes = 0usize;
    let mut linked_data_missing_extra = 0usize;
    let mut feature_header_records = 0usize;
    let mut feature_header_none = 0usize;
    let mut feature_header_enhanced_protection = 0usize;
    let mut feature_header_property_bag_store = 0usize;
    let mut feature_header_malformed = 0usize;
    let mut feature_header_malformed_bytes = 0usize;
    let mut feature_header_malformed_samples = Vec::new();
    let mut feature_records = 0usize;
    let mut feature_protection = 0usize;
    let mut feature_formula_errors = 0usize;
    let mut feature_smart_tags = 0usize;
    let mut feature_security_descriptors = 0usize;
    let mut dconn_records = 0usize;
    let mut dconn_text = 0usize;
    let mut dconn_web = 0usize;
    let mut text_query_records = 0usize;
    let mut qsi_sx_tag_records = 0usize;
    let mut sx_view_ex9_records = 0usize;
    let mut db_query_ext_records = 0usize;
    let mut hyperlink_tooltip_records = 0usize;
    let mut continue_frt12_records = 0usize;
    let mut sx_addl_records = 0usize;
    let mut sx_addl_types = BTreeMap::<(u8, u8), usize>::new();
    let mut ent_ex_u2_records = 0usize;
    let mut ent_ex_u2_cache_bytes = 0usize;
    let mut feature11_records = 0usize;
    let mut feature11_fields = 0usize;
    let mut feature11_formats = 0usize;
    let mut feature11_auto_filters = 0usize;
    let mut feature11_embedded_auto_filters = 0usize;
    let mut feature11_xml_maps = 0usize;
    let mut feature11_formulas = 0usize;
    let mut feature11_total_formulas = 0usize;
    let mut feature11_total_array_formulas = 0usize;
    let mut feature11_total_texts = 0usize;
    let mut feature11_wss_info = 0usize;
    let mut feature11_query_fields = 0usize;
    let mut feature11_cached_headers = 0usize;
    let mut feature11_deleted_id_lists = 0usize;
    let mut feature11_changed_id_lists = 0usize;
    let mut feature11_invalid_cell_lists = 0usize;
    let mut name_records = 0usize;
    let mut name_unparsed_rgce_bytes = 0usize;
    let mut name_rgcb_tail_bytes = 0usize;
    let mut name_missing_extra = 0usize;
    let mut name_continued_records = 0usize;
    let mut name_tail_locations = Vec::new();
    let mut xf_ext_unparsed_bytes = 0usize;
    let mut xf_ext_compatibility_full_colors = 0usize;
    let mut xf_ext_unknown_types = BTreeSet::new();
    let mut xf_ext_stats = BTreeMap::<u16, UnknownStats>::new();
    let mut xf_ext_unknown_locations = Vec::new();
    let mut style_ext_unparsed_bytes = 0usize;
    let mut style_ext_stats = BTreeMap::<u16, UnknownStats>::new();
    let mut dxf_records = 0usize;
    let mut dxf_unparsed_bytes = 0usize;
    let mut dxf_stats = BTreeMap::<u16, UnknownStats>::new();
    let mut cfex_records = 0usize;
    let mut cfex_cf12_records = 0usize;
    let mut cfex_non_cf12_records = 0usize;
    let mut cfex_formats = 0usize;
    let mut cfex_extension_unparsed_bytes = 0usize;
    let mut cf12_records = 0usize;
    let mut cf12_types = BTreeMap::<u8, usize>::new();
    let mut cf12_unparsed_formula_bytes = 0usize;
    let mut crt_ml_frt_records = 0usize;
    let mut xml_tk_records = 0usize;
    let mut xml_tk_kinds = BTreeMap::<&'static str, usize>::new();
    let mut sst_records = 0usize;
    let mut sst_strings = 0usize;
    let mut sst_extension_bytes = 0usize;
    let mut sst_extension_unparsed_bytes = 0usize;
    let mut sst_truncated_phonetic_headers = 0usize;
    let mut sst_extension_unparsed_stats = BTreeMap::<&'static str, UnknownStats>::new();
    let mut sst_phonetic_trailing_samples = Vec::new();
    let mut sst_extension_unparsed_samples = Vec::new();
    let mut sst_trailing_bytes = 0usize;
    let mut sst_truncated = Vec::new();
    let mut root_truncated_diagnostics = 0usize;
    let mut root_nonconforming_diagnostics = 0usize;
    let mut root_workbook_structure_diagnostics = 0usize;
    let mut root_bof_diagnostics = 0usize;
    let mut root_bof_diagnostic_shapes = BTreeMap::<String, usize>::new();
    let mut root_workbook_diagnostic_shapes = BTreeMap::<(String, String), usize>::new();
    let mut root_workbook_diagnostic_samples = Vec::new();
    let mut root_invalid_stream_diagnostics = 0usize;
    let mut root_noncanonical_cfb_diagnostics = 0usize;
    let mut root_truncated_diagnostic_files = BTreeSet::new();
    let mut root_nonconforming_diagnostic_files = BTreeSet::new();
    let mut root_workbook_structure_diagnostic_files = BTreeSet::new();
    let mut root_bof_diagnostic_files = BTreeSet::new();
    let mut root_invalid_stream_diagnostic_files = BTreeSet::new();
    let mut root_noncanonical_cfb_diagnostic_files = BTreeSet::new();
    let mut failures = Vec::new();
    let filter = std::env::var("XLS_FILTER").ok();
    for path in files {
        if filter
            .as_ref()
            .is_some_and(|filter| !path.to_string_lossy().contains(filter))
        {
            continue;
        }
        let Ok(bytes) = olecfsdk_corpus_test_support::corpus_bytes(&path) else {
            continue;
        };
        let Ok(compound) = CompoundFile::from_bytes(&bytes) else {
            continue;
        };
        let Some(entry) = compound.entries().iter().find(|entry| {
            entry.is_stream()
                && entry.path.parent() == Some(Path::new("/"))
                && (entry.name.eq_ignore_ascii_case("Workbook")
                    || entry.name.eq_ignore_ascii_case("Book"))
        }) else {
            continue;
        };
        checked += 1;
        let result = (|| {
            let outcome = XlsFile::from_compound_file_compatible(compound.clone())?;
            for diagnostic in &outcome.diagnostics {
                match diagnostic.code {
                    ParseDiagnosticCode::TruncatedRecord => {
                        root_truncated_diagnostics += 1;
                        root_truncated_diagnostic_files.insert(path.clone());
                    }
                    ParseDiagnosticCode::NonconformingRecord => {
                        root_nonconforming_diagnostics += 1;
                        root_nonconforming_diagnostic_files.insert(path.clone());
                        root_workbook_structure_diagnostics += usize::from(matches!(
                            diagnostic.structure,
                            "Workbook Stream" | "Globals Substream"
                        ));
                        root_bof_diagnostics += usize::from(diagnostic.structure == "BOF");
                        if diagnostic.structure == "BOF" {
                            root_bof_diagnostic_files.insert(path.clone());
                            *root_bof_diagnostic_shapes
                                .entry(diagnostic.message.clone())
                                .or_default() += 1;
                        }
                        if matches!(
                            diagnostic.structure,
                            "Workbook Stream" | "Globals Substream"
                        ) {
                            root_workbook_structure_diagnostic_files.insert(path.clone());
                            *root_workbook_diagnostic_shapes
                                .entry((
                                    diagnostic.structure.to_owned(),
                                    diagnostic.message.clone(),
                                ))
                                .or_default() += 1;
                            if !diagnostic.message.starts_with("legacy ")
                                && root_workbook_diagnostic_samples.len() < 20
                            {
                                root_workbook_diagnostic_samples.push(format!(
                                    "{}: {}: {}",
                                    path.display(),
                                    diagnostic.structure,
                                    diagnostic.message
                                ));
                            }
                        }
                    }
                    ParseDiagnosticCode::InvalidStreamPreserved => {
                        root_invalid_stream_diagnostics += 1;
                        root_invalid_stream_diagnostic_files.insert(path.clone());
                    }
                    ParseDiagnosticCode::NoncanonicalCompoundFile => {
                        root_noncanonical_cfb_diagnostics += 1;
                        root_noncanonical_cfb_diagnostic_files.insert(path.clone());
                    }
                    ParseDiagnosticCode::InvalidReference => {
                        return Err(olecfsdk::Error::invalid(
                            diagnostic.location.offset.unwrap_or(0),
                            "XLS root emitted a PPT reference diagnostic",
                        ));
                    }
                }
            }
            let file = outcome.value;
            root_parsed_files += 1;
            let workbook = file
                .workbooks
                .iter()
                .find(|workbook| {
                    workbook
                        .name
                        .path()
                        .trim_start_matches('/')
                        .eq_ignore_ascii_case(&entry.name)
                })
                .ok_or_else(|| {
                    olecfsdk::Error::invalid(0, "typed XLS file root omitted a BIFF stream")
                })?;
            let parsed = &workbook.tree.stream;
            match workbook.drawing_graph() {
                Ok(Some(graph)) => {
                    drawing_graphs += 1;
                    drawing_graph_drawings += graph.drawings.len();
                    drawing_graph_shapes += graph
                        .drawings
                        .iter()
                        .map(|drawing| drawing.shapes.len())
                        .sum::<usize>();
                    *drawing_graph_maximum_shape_id_relations
                        .entry(format!("{:?}", graph.maximum_shape_id_relation))
                        .or_default() += 1;
                    *drawing_graph_saved_shape_count_relations
                        .entry(format!("{:?}", graph.saved_shape_count_relation))
                        .or_default() += 1;
                    *drawing_graph_saved_drawing_count_relations
                        .entry(format!("{:?}", graph.saved_drawing_count_relation))
                        .or_default() += 1;
                    for drawing in &graph.drawings {
                        *drawing_graph_shape_count_bases
                            .entry(format!("{:?}", drawing.shape_count_basis))
                            .or_default() += 1;
                        *drawing_graph_current_shape_id_relations
                            .entry(format!("{:?}", drawing.current_shape_id_relation))
                            .or_default() += 1;
                    }
                    for cluster in &graph.clusters {
                        if let Some(relation) = cluster.cursor_relation {
                            *drawing_graph_cluster_cursor_relations
                                .entry(format!("{relation:?}"))
                                .or_default() += 1;
                        }
                    }
                    drawing_graphs_strict += usize::from(graph.validate_strict().is_ok());
                    for issue in &graph.issues {
                        let kind = match issue {
                            OfficeArtDrawingGraphIssue::MaximumShapeIdOutOfRange { .. } => {
                                "maximum-shape-id-out-of-range"
                            }
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
                            OfficeArtDrawingGraphIssue::ShapeClusterDrawingMismatch { .. } => {
                                "shape-cluster-drawing-mismatch"
                            }
                        };
                        *drawing_graph_issues.entry(kind).or_default() += 1;
                    }
                }
                Ok(None) => drawing_graph_absent += 1,
                Err(error) => {
                    *drawing_graph_errors.entry(error.to_string()).or_default() += 1;
                }
            }
            let rebuilt = file.to_compound_file_preserving_compatibility()?;
            if rebuilt.stream(workbook.name.path()) != Some(entry.data.as_slice()) {
                return Err(olecfsdk::Error::invalid(
                    0,
                    "typed XLS file root changed the workbook stream",
                ));
            }
            if parsed.is_biff8() {
                biff8 += 1;
                unknown_types.extend(parsed.unknown_record_types());
                for (record_index, record) in parsed.records.iter().enumerate() {
                    match &record.data {
                        BiffRecordData::BigName(value) => {
                            *newly_static_formal_records.entry("BigName").or_default() += 1;
                            *newly_static_formal_records
                                .entry("ContinueBigName physical segments")
                                .or_default() += value.physical_segments.len().saturating_sub(1);
                        }
                        BiffRecordData::ContinueBigName(_) => {
                            *newly_static_formal_records
                                .entry("unaggregated ContinueBigName")
                                .or_default() += 1;
                        }
                        BiffRecordData::Lrng(_) => {
                            *newly_static_formal_records.entry("LRng").or_default() += 1;
                        }
                        BiffRecordData::ExtString(_) => {
                            *newly_static_formal_records.entry("ExtString").or_default() += 1;
                        }
                        BiffRecordData::OleDbConn(_) => {
                            *newly_static_formal_records.entry("OleDbConn").or_default() += 1;
                        }
                        BiffRecordData::Mdb(_) => {
                            *newly_static_formal_records.entry("MDB").or_default() += 1;
                        }
                        BiffRecordData::FrtFontList(_) => {
                            *newly_static_formal_records
                                .entry("FrtFontList")
                                .or_default() += 1;
                        }
                        BiffRecordData::DataLabExt(_) => {
                            *newly_static_formal_records.entry("DataLabExt").or_default() += 1;
                        }
                        BiffRecordData::AutoFilter12(value) => {
                            *newly_static_formal_records
                                .entry("AutoFilter12")
                                .or_default() += 1;
                            *newly_static_formal_records
                                .entry("AutoFilter12 criteria")
                                .or_default() += value.criteria.len();
                            *newly_static_formal_records
                                .entry("AutoFilter12 date groupings")
                                .or_default() += value.date_groupings.len();
                        }
                        BiffRecordData::CrErr(value) => {
                            *newly_static_formal_records.entry("CrErr").or_default() += 1;
                            *newly_static_formal_records
                                .entry("CrErr physical string chunks")
                                .or_default() += value.chunks.len();
                        }
                        BiffRecordData::DocRoute(value) => {
                            *newly_static_formal_records.entry("DocRoute").or_default() += 1;
                            *newly_static_formal_records
                                .entry("DocRoute recipients")
                                .or_default() += value.recipients.len();
                        }
                        BiffRecordData::FrtWrapper(_) => {
                            *newly_static_formal_records.entry("FrtWrapper").or_default() += 1;
                        }
                        BiffRecordData::Qsir(value) => {
                            *newly_static_formal_records.entry("Qsir").or_default() += 1;
                            *newly_static_formal_records.entry("Qsif").or_default() +=
                                value.fields.len();
                        }
                        BiffRecordData::Qsif(_) => {
                            *newly_static_formal_records
                                .entry("unaggregated Qsif")
                                .or_default() += 1;
                        }
                        BiffRecordData::SxViewLink(_) => {
                            *newly_static_formal_records.entry("SXViewLink").or_default() += 1;
                        }
                        BiffRecordData::WebPub(_) => {
                            *newly_static_formal_records.entry("WebPub").or_default() += 1;
                        }
                        BiffRecordData::Unknown {
                            record_type,
                            payload,
                        } => {
                            let stats = unknown_stats.entry(*record_type).or_default();
                            stats.records += 1;
                            stats.bytes += payload.len();
                            stats.lengths.insert(payload.len());
                            let samples = unknown_samples.entry(*record_type).or_default();
                            if samples.len() < 8 {
                                samples.push(format!(
                                    "{} offset {} prev {:?} next {:?} len {} payload {:02x?}",
                                    path.display(),
                                    record.offset,
                                    record_index.checked_sub(1).and_then(|index| {
                                        let offset = parsed.records[index].offset as usize;
                                        entry
                                            .data
                                            .get(offset..offset + 2)
                                            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                                    }),
                                    parsed.records.get(record_index + 1).and_then(|record| {
                                        let offset = record.offset as usize;
                                        entry
                                            .data
                                            .get(offset..offset + 2)
                                            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                                    }),
                                    payload.len(),
                                    payload
                                ));
                            }
                            if *record_type == 0x004d && pls_samples.len() < 40 {
                                pls_samples.push(format!(
                                    "{} offset {} len {} head {:02x?}",
                                    path.display(),
                                    record.offset,
                                    payload.len(),
                                    &payload[..payload.len().min(96)]
                                ));
                            }
                        }
                        BiffRecordData::Empty {
                            kind: olecfsdk::xls::EmptyRecordKind::NullCompatibility,
                            reserved,
                        } => {
                            null_compatibility_records += 1;
                            assert!(reserved.is_none());
                        }
                        BiffRecordData::Formula(formula) => {
                            formula_unparsed_rgce_bytes += formula.tokens.rgce.unparsed_tail.len();
                            formula_rgcb_bytes += formula.tokens.rgcb_tail.len();
                            let missing_extra = formula.tokens.rgce.missing_extra_count();
                            formula_missing_extra += missing_extra;
                            if missing_extra != 0 {
                                formula_tail_locations.push(format!(
                                    "{}: BIFF offset {}, {missing_extra} missing token extras",
                                    path.display(),
                                    record.offset
                                ));
                            }
                            if !formula.tokens.rgcb_tail.is_empty() {
                                formula_tail_locations.push(format!(
                                    "{}: BIFF offset {}, {} rgcb tail bytes {:02x?}",
                                    path.display(),
                                    record.offset,
                                    formula.tokens.rgcb_tail.len(),
                                    formula.tokens.rgcb_tail
                                ));
                            }
                        }
                        BiffRecordData::Formula4Compatibility(formula) => {
                            formula4_compatibility_records += 1;
                            formula4_unparsed_bytes += formula.tokens.rgce.unparsed_tail.len();
                            formula4_unparsed_bytes += formula.tokens.rgcb_tail.len();
                        }
                        BiffRecordData::ExternCount(_) => extern_count_records += 1,
                        BiffRecordData::Qsi(value) => {
                            qsi_records += 1;
                            qsi_name_characters += value.name.text.character_count();
                        }
                        BiffRecordData::ParamQry(value) => {
                            param_qry_records += 1;
                            param_qry_prompts +=
                                usize::from(matches!(value.data, ParamQryData::Prompt { .. }));
                        }
                        BiffRecordData::SxSelect(value) => {
                            sx_select_records += 1;
                            sx_select_extended += usize::from(value.options.extendable);
                        }
                        BiffRecordData::SharedFormula(formula) => {
                            shared_formula_records += 1;
                            shared_formula_unparsed_rgce_bytes +=
                                formula.tokens.rgce.unparsed_tail.len();
                            shared_formula_rgcb_bytes += formula.tokens.rgcb_tail.len();
                        }
                        BiffRecordData::Array(array) => {
                            array_records += 1;
                            array_unparsed_rgce_bytes += array.tokens.rgce.unparsed_tail.len();
                            array_rgcb_bytes += array.tokens.rgcb_tail.len();
                        }
                        BiffRecordData::SupBook(sup_book) => {
                            sup_book_records += 1;
                            match &sup_book.link {
                                SupBookLink::Compatibility { payload, .. } => {
                                    sup_book_compatibility += 1;
                                    sup_book_trailing_bytes += payload.len();
                                }
                                SupBookLink::VirtualPath { trailing, .. } => {
                                    sup_book_trailing_bytes += trailing.len();
                                }
                                SupBookLink::SelfReference | SupBookLink::AddInFunctions => {}
                            }
                        }
                        BiffRecordData::ExternName(extern_name) => {
                            extern_name_records += 1;
                            match &extern_name.body {
                                ExternNameBody::ParsedFormula { .. } => {
                                    extern_name_formula_records += 1
                                }
                                ExternNameBody::CachedLinkValues { .. } => {
                                    extern_name_cached_link_records += 1
                                }
                                ExternNameBody::Compatibility(bytes) => {
                                    extern_name_compatibility_records += 1;
                                    extern_name_compatibility_bytes += bytes.len();
                                    extern_name_compatibility_samples.push(format!(
                                        "{} offset {} flags {:#06x} name {:?} body {:02x?}",
                                        path.display(),
                                        record.offset,
                                        extern_name.flags.bits(),
                                        extern_name.name,
                                        bytes
                                    ));
                                }
                                ExternNameBody::Empty => {}
                            }
                        }
                        BiffRecordData::Hyperlink(hyperlink) => {
                            hyperlink_records += 1;
                            match &hyperlink.object {
                                HyperlinkObject::Parsed { trailing, .. } => {
                                    hyperlink_trailing_bytes += trailing.len()
                                }
                                HyperlinkObject::Compatibility(bytes) => {
                                    hyperlink_compatibility_records += 1;
                                    hyperlink_compatibility_bytes += bytes.len();
                                    hyperlink_compatibility_samples.push(format!(
                                        "{} offset {} len {} head {:02x?}",
                                        path.display(),
                                        record.offset,
                                        bytes.len(),
                                        &bytes[..bytes.len().min(96)]
                                    ));
                                }
                                HyperlinkObject::Truncated {
                                    stream_version,
                                    flags,
                                    payload,
                                } => {
                                    hyperlink_truncated_records += 1;
                                    hyperlink_truncated_bytes += payload.len();
                                    hyperlink_truncated_samples.push(format!(
                                        "{} offset {} version {stream_version} flags 0x{:08x} len {} head {:02x?} tail {:02x?}",
                                        path.display(),
                                        record.offset,
                                        flags.bits(),
                                        payload.len(),
                                        &payload[..payload.len().min(128)],
                                        &payload[payload.len().saturating_sub(128)..]
                                    ));
                                }
                                HyperlinkObject::TruncatedUrlMoniker {
                                    stream_version,
                                    flags,
                                    class_id: _,
                                    declared_byte_length,
                                    address,
                                } => {
                                    hyperlink_truncated_records += 1;
                                    hyperlink_truncated_url_records += 1;
                                    hyperlink_truncated_url_address_bytes += address.len() * 2;
                                    assert_eq!(*stream_version, 2);
                                    assert_eq!(flags.bits(), 0x0000_0003);
                                    assert_eq!(*declared_byte_length, 23_002);
                                }
                            }
                        }
                        BiffRecordData::DataValidation(validation) => {
                            data_validation_records += 1;
                            for formula in [&validation.formula1, &validation.formula2] {
                                data_validation_unparsed_rgce_bytes +=
                                    formula.tokens.unparsed_tail.len();
                                data_validation_missing_extra +=
                                    formula.tokens.missing_extra_count();
                            }
                        }
                        BiffRecordData::ConditionalFormatting(rule) => {
                            conditional_formatting_records += 1;
                            for formula in [&rule.formula1, &rule.formula2] {
                                conditional_formatting_unparsed_rgce_bytes +=
                                    formula.unparsed_tail.len();
                                conditional_formatting_missing_extra +=
                                    formula.missing_extra_count();
                            }
                        }
                        BiffRecordData::ChartLinkedData(link) => {
                            linked_data_records += 1;
                            linked_data_unparsed_rgce_bytes += link.formula.unparsed_tail.len();
                            linked_data_missing_extra += link.formula.missing_extra_count();
                        }
                        BiffRecordData::FeatureHeader(header) => {
                            feature_header_records += 1;
                            match &header.data {
                                FeatureHeaderData::None => feature_header_none += 1,
                                FeatureHeaderData::EnhancedProtection(_) => {
                                    feature_header_enhanced_protection += 1;
                                }
                                FeatureHeaderData::PropertyBagStore(_) => {
                                    feature_header_property_bag_store += 1;
                                }
                                FeatureHeaderData::Malformed { marker, payload } => {
                                    feature_header_malformed += 1;
                                    feature_header_malformed_bytes += payload.len();
                                    feature_header_malformed_samples.push(format!(
                                        "{}: BIFF offset {}, type 0x{:04x}, reserved 0x{:02x}, marker 0x{marker:08x}, {} bytes, head {:02x?}",
                                        path.display(),
                                        record.offset,
                                        header.shared_feature_type,
                                        header.reserved,
                                        payload.len(),
                                        &payload[..payload.len().min(128)]
                                    ));
                                }
                            }
                        }
                        BiffRecordData::Feature(feature) => {
                            feature_records += 1;
                            match &feature.data {
                                FeatureData::Protection(value) => {
                                    feature_protection += 1;
                                    feature_security_descriptors +=
                                        usize::from(value.security_descriptor.is_some());
                                }
                                FeatureData::FormulaErrors(_) => feature_formula_errors += 1,
                                FeatureData::SmartTags(_) => feature_smart_tags += 1,
                            }
                        }
                        BiffRecordData::DConn(connection) => {
                            dconn_records += 1;
                            match &connection.connection {
                                DConnConnection::Text(_) => dconn_text += 1,
                                DConnConnection::Web(_) => dconn_web += 1,
                                _ => {}
                            }
                        }
                        BiffRecordData::TextQuery(_) => text_query_records += 1,
                        BiffRecordData::QsiSxTag(_) => qsi_sx_tag_records += 1,
                        BiffRecordData::SxViewEx9(_) => sx_view_ex9_records += 1,
                        BiffRecordData::DbQueryExt(_) => db_query_ext_records += 1,
                        BiffRecordData::HyperlinkTooltip(_) => hyperlink_tooltip_records += 1,
                        BiffRecordData::ContinueFrt12(_) => continue_frt12_records += 1,
                        BiffRecordData::SxAddl(value) => {
                            sx_addl_records += 1;
                            *sx_addl_types
                                .entry((value.header.class, value.header.data_type))
                                .or_default() += 1;
                        }
                        BiffRecordData::EntExU2(value) => {
                            ent_ex_u2_records += 1;
                            ent_ex_u2_cache_bytes += value.cache.len();
                        }
                        BiffRecordData::BkHim(value) => {
                            bk_him_records += 1;
                            bk_him_continued +=
                                usize::from(value.physical_segment_lengths.len() > 1);
                            match &value.image {
                                BkHimImage::Bitmap(bitmap) => {
                                    bk_him_bitmaps += 1;
                                    bk_him_image_bytes += bitmap.len();
                                }
                                BkHimImage::Native(bytes) => {
                                    bk_him_native += 1;
                                    bk_him_image_bytes += bytes.len();
                                }
                            }
                        }
                        BiffRecordData::ImData(value) => {
                            im_data_records += 1;
                            let BkHimImage::Bitmap(bitmap) = &value.image else {
                                panic!("corpus ImData image format changed");
                            };
                            im_data_bitmap_bytes += bitmap.len();
                        }
                        BiffRecordData::RealTimeData(value) => {
                            real_time_data_records += 1;
                            real_time_data_topic_segments += value.topic.substrings.len();
                            real_time_data_cells += value.cells.len();
                            match &value.operation {
                                RtdOperation::ErrorWithCorruptDiscriminator {
                                    discriminator,
                                    value,
                                } => {
                                    real_time_data_corrupt_error_discriminators += 1;
                                    assert_eq!(*discriminator, 0x0000_dd10);
                                    assert_eq!(*value, 42);
                                }
                                RtdOperation::Malformed {
                                    discriminator,
                                    payload,
                                } => {
                                    real_time_data_malformed += 1;
                                    real_time_data_malformed_bytes += payload.len();
                                    real_time_data_malformed_samples.push(format!(
                                        "{}: BIFF offset {}, discriminator 0x{discriminator:08x}, {} bytes {:02x?}",
                                        path.display(),
                                        record.offset,
                                        payload.len(),
                                        payload
                                    ));
                                }
                                _ => {}
                            }
                        }
                        BiffRecordData::Sort(value) => {
                            sort_records += 1;
                            for key in value.keys.iter().flatten() {
                                sort_keys += 1;
                                sort_compressed_keys += usize::from(matches!(
                                    &key.characters,
                                    XlStringCharacters::Compressed(_)
                                ));
                            }
                        }
                        BiffRecordData::SortData(value) => {
                            sort_data_records += 1;
                            sort_data_conditions += value.condition_count as usize;
                            match value.options.parent {
                                SortFieldParent::Table => sort_data_table += 1,
                                SortFieldParent::AutoFilter => sort_data_auto_filter += 1,
                                _ => {}
                            }
                            sort_conditions += value.conditions.len();
                            sort_conditions_descending += value
                                .conditions
                                .iter()
                                .filter(|value| value.condition.descending)
                                .count();
                        }
                        BiffRecordData::AutoFilter(value) => {
                            auto_filter_records += 1;
                            for operand in &value.operands {
                                match operand.value {
                                    AutoFilterOperandValue::String { .. } => {
                                        auto_filter_strings += 1;
                                        auto_filter_string_characters += operand
                                            .string
                                            .as_ref()
                                            .expect("typed string operand has text")
                                            .character_count();
                                    }
                                    AutoFilterOperandValue::Number { .. } => {
                                        auto_filter_numbers += 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        BiffRecordData::SxFormat(value) => {
                            sx_format_records += 1;
                            sx_format_applied += usize::from(value.formatting_applied);
                        }
                        BiffRecordData::WOpt(value) => {
                            wopt_records += 1;
                            wopt_component_characters += value.component_location.characters.len();
                            wopt_future_bytes += value.future.len();
                        }
                        BiffRecordData::Table(value) => {
                            table_records += 1;
                            table_two_variable += usize::from(value.options.two_variable);
                            table_compatibility_padding +=
                                usize::from(value.compatibility_padding.is_some());
                        }
                        BiffRecordData::Obj(value) => {
                            standalone_obj_records += 1;
                            standalone_obj_trailing_bytes += value.trailing.len();
                            if !value.trailing.is_empty() {
                                standalone_obj_trailing_samples.push(format!(
                                    "{}: BIFF offset {}, subrecords {:?}, trailing {} bytes {:02x?}",
                                    path.display(),
                                    record.offset,
                                    value.subrecords,
                                    value.trailing.len(),
                                    value.trailing
                                ));
                            }
                            for subrecord in &value.subrecords {
                                standalone_obj_truncated_picture_flags += usize::from(matches!(
                                    subrecord.data,
                                    ObjSubrecordData::TruncatedPictureFlags { .. }
                                ));
                                if let ObjSubrecordData::PictureFormula(picture) = &subrecord.data {
                                    obj_picture_formulas += 1;
                                    obj_picture_embed_info += usize::from(matches!(
                                        &picture.formula.data,
                                        ObjFormulaData::Parsed {
                                            embed_info: Some(_),
                                            ..
                                        }
                                    ));
                                    obj_picture_positions +=
                                        usize::from(picture.control_stream_position.is_some());
                                    obj_picture_control_stream_sizes +=
                                        usize::from(picture.control_stream_size.is_some());
                                    obj_picture_keys += usize::from(picture.key.is_some());
                                    obj_picture_compatibility_bytes +=
                                        picture.compatibility_trailing.len();
                                }
                                if let ObjSubrecordData::Raw(bytes) = &subrecord.data {
                                    standalone_obj_raw_subrecords += 1;
                                    *standalone_obj_raw_shapes
                                        .entry((subrecord.subrecord_type, bytes.len()))
                                        .or_default() += 1;
                                }
                            }
                        }
                        BiffRecordData::ObjCompatibility { value, .. } => {
                            obj_id_compatibility_records += 1;
                            for subrecord in &value.subrecords {
                                if let ObjSubrecordData::PictureFormula(picture) = &subrecord.data {
                                    obj_picture_formulas += 1;
                                    obj_picture_embed_info += usize::from(matches!(
                                        &picture.formula.data,
                                        ObjFormulaData::Parsed {
                                            embed_info: Some(_),
                                            ..
                                        }
                                    ));
                                    obj_picture_positions +=
                                        usize::from(picture.control_stream_position.is_some());
                                    obj_picture_control_stream_sizes +=
                                        usize::from(picture.control_stream_size.is_some());
                                    obj_picture_keys += usize::from(picture.key.is_some());
                                    obj_picture_compatibility_bytes +=
                                        picture.compatibility_trailing.len();
                                }
                            }
                            assert!(value.subrecords.iter().all(|subrecord| !matches!(
                                subrecord.data,
                                ObjSubrecordData::Raw(_)
                            )));
                        }
                        BiffRecordData::FontCompatibility { value, .. } => {
                            font_id_compatibility_records += 1;
                            assert_eq!(value.name.character_count(), 5);
                        }
                        BiffRecordData::XfCompatibility { .. } => {
                            xf_id_compatibility_records += 1;
                        }
                        BiffRecordData::BoundSheet8Compatibility { value, .. } => {
                            bound_sheet_id_compatibility_records += 1;
                            assert_eq!(
                                match &value.name.characters {
                                    XlStringCharacters::Compressed(bytes) => bytes.len(),
                                    XlStringCharacters::Unicode(words) => words.len(),
                                },
                                7
                            );
                        }
                        BiffRecordData::ChartSeriesCompatibility { value, .. } => {
                            chart_series_id_compatibility_records += 1;
                            assert_eq!(value.category_count, 8);
                            assert_eq!(value.value_count, 8);
                        }
                        BiffRecordData::Txo(value) => {
                            standalone_txo_records += 1;
                            standalone_txo_undetermined += usize::from(matches!(
                                value.context,
                                TxoContext::Undetermined { .. }
                            ));
                        }
                        BiffRecordData::PlvMac(_) => plv_mac_records += 1,
                        BiffRecordData::Lnext(_) => lnext_records += 1,
                        BiffRecordData::MkrExt(_) => mkr_ext_records += 1,
                        BiffRecordData::CrtCoOpt(value) => {
                            crt_co_opt_records += 1;
                            crt_co_opt_compatibility_padding +=
                                usize::from(value.compatibility_padding.is_some());
                        }
                        BiffRecordData::FrtArchId(value) => {
                            frt_arch_id_records += 1;
                            *frt_arch_ids.entry(value.architecture_id).or_default() += 1;
                        }
                        BiffRecordData::LhRecord(value) => {
                            lh_records += 1;
                            lh_subrecords += value.subrecords.len();
                            for subrecord in &value.subrecords {
                                match subrecord {
                                    LhSubrecordData::Margin { .. } => lh_margins += 1,
                                    LhSubrecordData::GraphView(graph) => {
                                        lh_graph_views += 1;
                                        lh_graph_compatibility_extensions +=
                                            usize::from(graph.compatibility_extension.is_some());
                                        assert_eq!(graph.core.reference_defined, [0; 13]);
                                        assert_eq!(
                                            graph.core.graph_type,
                                            olecfsdk::xls::LhGraphType::Line
                                        );
                                        assert_eq!(graph.core.x_format, 0x71);
                                        assert_eq!(graph.core.y_format, 0x71);
                                        assert_eq!(graph.core.skip_factor, 1);
                                        assert_eq!(
                                            graph.compatibility_extension,
                                            Some([0, 104, 0])
                                        );
                                    }
                                    LhSubrecordData::Reserved { words, .. } => {
                                        lh_reserved_words += words.len();
                                    }
                                    LhSubrecordData::UndocumentedType13(_) => {
                                        lh_undocumented_type13 += 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        BiffRecordData::Feature11(feature) => {
                            feature11_records += 1;
                            feature11_fields += feature.feature.fields.len();
                            feature11_deleted_id_lists +=
                                usize::from(feature.feature.deleted_row_ids.is_some());
                            feature11_changed_id_lists +=
                                usize::from(feature.feature.changed_row_ids.is_some());
                            feature11_invalid_cell_lists +=
                                usize::from(feature.feature.invalid_cells.is_some());
                            for field in &feature.feature.fields {
                                feature11_formats += usize::from(field.aggregate_format.is_some())
                                    + usize::from(field.insert_row_format.is_some());
                                feature11_auto_filters += usize::from(field.auto_filter.is_some());
                                feature11_embedded_auto_filters += field
                                    .auto_filter
                                    .as_ref()
                                    .map_or(0, |value| usize::from(value.filter.is_some()));
                                feature11_xml_maps += usize::from(field.xml_map.is_some());
                                feature11_formulas += usize::from(field.formula.is_some());
                                feature11_total_formulas +=
                                    usize::from(field.total_formula.is_some());
                                feature11_total_array_formulas += usize::from(matches!(
                                    field.total_formula.as_ref(),
                                    Some(olecfsdk::xls::Feat11TotalFmla::ArrayFormula(_))
                                ));
                                feature11_total_texts += usize::from(field.total_text.is_some());
                                feature11_wss_info += usize::from(field.wss_info.is_some());
                                feature11_query_fields +=
                                    usize::from(field.query_field_id.is_some());
                                feature11_cached_headers +=
                                    usize::from(field.cached_header.is_some());
                            }
                        }
                        BiffRecordData::Name(name) => {
                            name_records += 1;
                            name_unparsed_rgce_bytes += name.formula.unparsed_tail.len();
                            name_rgcb_tail_bytes += name.formula_extra_tail.len();
                            name_missing_extra += name.formula.missing_extra_count();
                            name_continued_records +=
                                usize::from(name.physical_segment_lengths.len() > 1);
                            if !name.formula.unparsed_tail.is_empty()
                                || !name.formula_extra_tail.is_empty()
                                || name.formula.missing_extra_count() != 0
                            {
                                name_tail_locations.push(format!(
                                    "{}: BIFF offset {}, rgce tail {}, rgcb tail {}, missing extras {}",
                                    path.display(),
                                    record.offset,
                                    name.formula.unparsed_tail.len(),
                                    name.formula_extra_tail.len(),
                                    name.formula.missing_extra_count()
                                ));
                            }
                        }
                        BiffRecordData::Pls(pls) => {
                            pls_records += 1;
                            pls_continued += usize::from(pls.physical_segment_lengths.len() > 1);
                            match &pls.settings {
                                PrinterSettings::WindowsUnicode(devmode)
                                | PrinterSettings::LengthPrefixedWindowsUnicode {
                                    devmode, ..
                                } => {
                                    pls_driver_extra_bytes += devmode.driver_extra.len();
                                    pls_truncated_driver_extra +=
                                        usize::from(!devmode.driver_extra_complete);
                                    if !devmode.driver_extra_complete {
                                        pls_truncated_driver_extra_samples.push(format!(
                                            "{} offset {} declared {} available {}",
                                            path.display(),
                                            record.offset,
                                            devmode.declared_driver_extra_size,
                                            devmode.driver_extra.len()
                                        ));
                                    }
                                    pls_trailing_bytes += devmode.trailing.len();
                                    match &devmode.public_fields {
                                        DevModeWPublic::Full(public) => {
                                            pls_windows_full += 1;
                                            pls_public_extension_bytes +=
                                                public.public_extension.len();
                                        }
                                        DevModeWPublic::Truncated(public) => {
                                            pls_windows_truncated += 1;
                                            pls_truncated_public_bytes += public.len();
                                            *pls_truncated_sizes
                                                .entry(devmode.declared_public_size)
                                                .or_default() += 1;
                                        }
                                        DevModeWPublic::Legacy212(_) => {
                                            pls_windows_legacy212 += 1;
                                        }
                                        DevModeWPublic::Core100(_) => {
                                            pls_windows_core100 += 1;
                                        }
                                    }
                                }
                                PrinterSettings::PlatformSpecific(payload) => {
                                    pls_platform_specific += 1;
                                    pls_platform_specific_bytes += payload.len();
                                    *pls_platform_shapes
                                        .entry((pls.reserved, payload.len()))
                                        .or_default() += 1;
                                    if pls_samples.len() < 20 {
                                        pls_samples.push(format!(
                                            "{} offset {} reserved 0x{:04x} len {} head {:02x?}",
                                            path.display(),
                                            record.offset,
                                            pls.reserved,
                                            payload.len(),
                                            &payload[..payload.len().min(96)]
                                        ));
                                    }
                                }
                                PrinterSettings::MacXmlPlist(payload) => {
                                    pls_platform_specific += 1;
                                    pls_platform_specific_bytes += payload.len();
                                }
                                PrinterSettings::MacPrintRecord(_) => {
                                    pls_platform_specific += 1;
                                    pls_platform_specific_bytes += 120;
                                }
                                PrinterSettings::LegacyPageLayout(_) => {
                                    pls_platform_specific += 1;
                                    pls_platform_specific_bytes += 28;
                                }
                            }
                        }
                        BiffRecordData::MsoDrawingGroup(drawing) => {
                            drawing_group_records += 1;
                            drawing_group_continued +=
                                usize::from(drawing.physical_segments.len() > 1);
                            match &drawing.data {
                                MsoDrawingData::Complete(stream) => {
                                    drawing_group_complete += 1;
                                    stream.visit(|office_record| {
                                        audit_office_art_properties(
                                            office_record,
                                            OfficeArtPropertyAudit {
                                                simple: &mut office_art_simple_properties,
                                                complex: &mut office_art_complex_properties,
                                                utf16: &mut office_art_utf16_properties,
                                                empty_complex: &mut office_art_empty_complex_properties,
                                                metro_blobs: &mut office_art_metro_blobs,
                                                metro_blob_entries: &mut office_art_metro_blob_entries,
                                                hyperlinks: &mut office_art_hyperlinks,
                                                hyperlink_nonparsed: &mut office_art_hyperlink_nonparsed,
                                                hyperlink_trailing_bytes: &mut office_art_hyperlink_trailing_bytes,
                                                array_headers: &mut office_art_array_headers,
                                                trailing_bytes: &mut office_art_property_table_trailing_bytes,
                                            },
                                        );
                                        match &office_record.data {
                                        OfficeArtRecordData::CompatibilityContainer(_) => {
                                            *office_art_compatibility_containers
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                            assert_ne!(office_record.header.version, 0x0f);
                                        }
                                        OfficeArtRecordData::PropertyTable(table)
                                            if office_record.header.record_type == 0xf043 =>
                                        {
                                            office_art_compatibility_property_tables += 1;
                                            assert_eq!(office_record.header.instance, 0x0010);
                                            assert_eq!(table.properties.len(), 8);
                                        }
                                        OfficeArtRecordData::ChildAnchor(_)
                                            if matches!(
                                                office_record.header.record_type,
                                                0x0000 | 0xf0aa
                                            ) =>
                                        {
                                            *office_art_compatibility_anchors
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                        }
                                        OfficeArtRecordData::SoftMakerNativeProperties(value) => {
                                            office_art_softmaker_native_records += 1;
                                            office_art_softmaker_native_properties +=
                                                value.properties.len();
                                            for property in &value.properties {
                                                let length = property.data.encoded_len();
                                                office_art_softmaker_native_payload_bytes += length;
                                                office_art_softmaker_native_unparsed_bytes +=
                                                    property.data.unparsed_byte_count();
                                                if let SoftMakerNativePropertyData::Selector6 {
                                                    font_name,
                                                    trailing,
                                                    ..
                                                } = &property.data
                                                {
                                                    office_art_softmaker_selector6 += 1;
                                                    assert_eq!(
                                                        *font_name,
                                                        [b'A' as u16, b'r' as u16, b'i' as u16,
                                                         b'a' as u16, b'l' as u16, 0]
                                                    );
                                                    assert_eq!(*trailing, 0);
                                                }
                                                *office_art_softmaker_native_shapes
                                                    .entry((
                                                        property.selector,
                                                        property.reserved,
                                                        length,
                                                    ))
                                                    .or_default() += 1;
                                            }
                                        }
                                        OfficeArtRecordData::Fbse(_)
                                            if office_record.header.record_type == 0xe007 =>
                                        {
                                            office_art_compatibility_fbse += 1;
                                        }
                                        OfficeArtRecordData::EmptyCompatibilityAtom => {
                                            *office_art_empty_compatibility_atoms
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                            assert_eq!(office_record.header.declared_length, 0);
                                        }
                                        OfficeArtRecordData::Atom(payload) => {
                                            let stats = office_art_atom_stats
                                                .entry(office_record.header.record_type)
                                                .or_default();
                                            stats.records += 1;
                                                stats.bytes += payload.len();
                                                stats.lengths.insert(payload.len());
                                                if office_art_atom_locations.len() < 32 {
                                                    office_art_atom_locations.push(format!(
                                                        "{}: BIFF offset {}, OfficeArt v{:#x}/i{:#05x}/t0x{:04x}, {} bytes, head {:02x?}",
                                                        path.display(),
                                                        record.offset,
                                                        office_record.header.version,
                                                        office_record.header.instance,
                                                        office_record.header.record_type,
                                                        payload.len(),
                                                        &payload[..payload.len().min(96)]
                                                    ));
                                                }
                                        }
                                            OfficeArtRecordData::MetafileBlip(blip) => {
                                            match &blip.file_data {
                                                OfficeArtMetafileData::Emf { .. } => {
                                                    office_art_emf_typed += 1;
                                                }
                                                OfficeArtMetafileData::Wmf { .. } => {
                                                    office_art_wmf_typed += 1;
                                                }
                                                OfficeArtMetafileData::Pict { .. } => {
                                                    office_art_pict_typed += 1;
                                                }
                                                OfficeArtMetafileData::Opaque {
                                                    reason,
                                                    decoded,
                                                    original_encoded,
                                                } => {
                                                    let stats = office_art_metafile_opaque
                                                        .entry(office_record.header.record_type)
                                                        .or_default();
                                                    stats.records += 1;
                                                    stats.bytes += original_encoded.len();
                                                    stats.lengths.insert(original_encoded.len());
                                                    let decoded_len = decoded
                                                        .as_ref()
                                                        .map_or(original_encoded.len(), Vec::len);
                                                    let reason_stats = office_art_metafile_opaque_reasons
                                                        .entry(*reason)
                                                        .or_default();
                                                    reason_stats.records += 1;
                                                    reason_stats.bytes += decoded_len;
                                                    reason_stats.lengths.insert(decoded_len);
                                                }
                                                }
                                            }
                                            OfficeArtRecordData::BitmapBlip(blip)
                                                if office_record.header.record_type == 0xf01f =>
                                            {
                                                match &blip.file_data {
                                                    OfficeArtBitmapData::Dib(_) => {
                                                        office_art_dib_typed += 1;
                                                    }
                                                    OfficeArtBitmapData::Encoded(_) => {
                                                        office_art_dib_opaque += 1;
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    });
                                }
                                MsoDrawingData::Partial(partial) => {
                                    drawing_group_partial += 1;
                                    drawing_group_partial_complete_records +=
                                        partial.complete_record_count();
                                    drawing_group_partial_incomplete_records +=
                                        partial.incomplete_record_count();
                                    drawing_group_partial_unparsed_bytes +=
                                        partial.unparsed_byte_count();
                                    partial.visit_incomplete(|incomplete| {
                                        if let OfficeArtIncompleteRecordData::FbseWithUnderreportedLength(
                                            fbse,
                                        ) = &incomplete.data
                                        {
                                            drawing_group_underreported_fbse += 1;
                                            assert_eq!(incomplete.header.record_type, 0xf007);
                                            assert_eq!(incomplete.header.declared_length, 0x42a3);
                                            assert_eq!(fbse.declared_blip_size, 0x3427f);
                                            assert!(fbse.trailing.is_empty());
                                            let blip = fbse
                                                .embedded_blip
                                                .as_deref()
                                                .expect("recovered FBSE must contain its BLIP");
                                            assert_eq!(blip.header.record_type, 0xf01e);
                                            assert_eq!(blip.header.declared_length, 0x34277);
                                            let OfficeArtRecordData::BitmapBlip(bitmap) = &blip.data
                                            else {
                                                panic!("recovered FBSE child is not a bitmap BLIP");
                                            };
                                            let OfficeArtBitmapData::Encoded(png) = &bitmap.file_data
                                            else {
                                                panic!("recovered PNG BLIP used an unexpected bitmap representation");
                                            };
                                            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
                                            assert!(png.ends_with(b"IEND\xaeB`\x82"));
                                        }
                                        drawing_group_partial_node_samples.push(format!(
                                            "incomplete v{:#x}/i{:#05x}/t0x{:04x}, declared {}, data {:?}",
                                            incomplete.header.version,
                                            incomplete.header.instance,
                                            incomplete.header.record_type,
                                            incomplete.header.declared_length,
                                            match &incomplete.data {
                                                OfficeArtIncompleteRecordData::Container(sequence) =>
                                                    ("container", sequence.records.len()),
                                                OfficeArtIncompleteRecordData::FbseWithUnderreportedLength(_) =>
                                                    ("underreported-fbse", 0),
                                                OfficeArtIncompleteRecordData::PropertyTable(_) =>
                                                    ("property-table", 0),
                                                OfficeArtIncompleteRecordData::RecoveredSequence { sequence, .. } =>
                                                    ("recovered-sequence", sequence.records.len()),
                                                OfficeArtIncompleteRecordData::Atom { available_payload } =>
                                                    ("atom", available_payload.len()),
                                            }
                                        ));
                                        if let OfficeArtIncompleteRecordData::Atom {
                                            available_payload,
                                        } = &incomplete.data
                                        {
                                            *drawing_group_partial_leaf_shapes
                                                .entry((
                                                    incomplete.header.record_type,
                                                    incomplete.header.instance,
                                                    incomplete.header.declared_length,
                                                    available_payload.len(),
                                                ))
                                                .or_default() += 1;
                                            drawing_group_partial_leaf_samples.push(format!(
                                                "{}: BIFF offset {}, OfficeArt v{:#x}/i{:#05x}/t0x{:04x}, declared {} / available {}, head {:02x?}, tail {:02x?}",
                                                path.display(),
                                                record.offset,
                                                incomplete.header.version,
                                                incomplete.header.instance,
                                                incomplete.header.record_type,
                                                incomplete.header.declared_length,
                                                available_payload.len(),
                                                &available_payload[..available_payload.len().min(128)],
                                                &available_payload[available_payload.len().saturating_sub(128)..]
                                            ));
                                        }
                                    });
                                    for length in partial.trailing_header_lengths() {
                                        *drawing_group_partial_header_tail_lengths
                                            .entry(length)
                                            .or_default() += 1;
                                    }
                                    partial.visit_complete(|office_record| {
                                        audit_office_art_properties(
                                            office_record,
                                            OfficeArtPropertyAudit {
                                                simple: &mut office_art_simple_properties,
                                                complex: &mut office_art_complex_properties,
                                                utf16: &mut office_art_utf16_properties,
                                                empty_complex: &mut office_art_empty_complex_properties,
                                                metro_blobs: &mut office_art_metro_blobs,
                                                metro_blob_entries: &mut office_art_metro_blob_entries,
                                                hyperlinks: &mut office_art_hyperlinks,
                                                hyperlink_nonparsed: &mut office_art_hyperlink_nonparsed,
                                                hyperlink_trailing_bytes: &mut office_art_hyperlink_trailing_bytes,
                                                array_headers: &mut office_art_array_headers,
                                                trailing_bytes: &mut office_art_property_table_trailing_bytes,
                                            },
                                        );
                                        drawing_group_partial_node_samples.push(format!(
                                            "complete v{:#x}/i{:#05x}/t0x{:04x}, declared {}",
                                            office_record.header.version,
                                            office_record.header.instance,
                                            office_record.header.record_type,
                                            office_record.header.declared_length,
                                        ));
                                        match &office_record.data {
                                        OfficeArtRecordData::Atom(payload) => {
                                            let stats = office_art_atom_stats
                                                .entry(office_record.header.record_type)
                                                .or_default();
                                            stats.records += 1;
                                            stats.bytes += payload.len();
                                            stats.lengths.insert(payload.len());
                                            office_art_atom_locations.push(format!(
                                                "{}: BIFF offset {}, partial OfficeArt v{:#x}/i{:#05x}/t0x{:04x}, {} bytes, head {:02x?}",
                                                path.display(), record.offset,
                                                office_record.header.version,
                                                office_record.header.instance,
                                                office_record.header.record_type,
                                                payload.len(),
                                                &payload[..payload.len().min(96)]
                                            ));
                                        }
                                        OfficeArtRecordData::CompatibilityContainer(_) => {
                                            *office_art_compatibility_containers
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                            assert_ne!(office_record.header.version, 0x0f);
                                        }
                                        OfficeArtRecordData::IncompletePropertyTable(table) => {
                                            office_art_incomplete_property_tables += 1;
                                            office_art_incomplete_property_entries += table.entries.len();
                                            office_art_incomplete_fixed_bytes +=
                                                table.incomplete_fixed_entry.encoded_len();
                                            office_art_incomplete_complex_bytes +=
                                                table.available_complex_len();
                                            office_art_incomplete_complex_unparsed_bytes +=
                                                table.unparsed_complex_len();
                                            for fragment in &table.complex_fragments {
                                                match &fragment.data {
                                                    OfficeArtComplexPropertyData::Array(_) => {
                                                        office_art_incomplete_array_fragments += 1;
                                                    }
                                                    OfficeArtComplexPropertyData::Bytes(bytes) => {
                                                        office_art_incomplete_generic_fragment_bytes += bytes.len();
                                                    }
                                                }
                                            }
                                            office_art_incomplete_property_samples
                                                .push(incomplete_property_sample(table));
                                        }
                                        OfficeArtRecordData::Fbse(fbse) => {
                                            drawing_group_partial_node_samples.push(format!(
                                                "FBSE declared blip size {}, embedded {}, trailing {} bytes, trailing head {:02x?}",
                                                fbse.declared_blip_size,
                                                fbse.embedded_blip.is_some(),
                                                fbse.trailing.len(),
                                                &fbse.trailing[..fbse.trailing.len().min(64)]
                                            ));
                                        }
                                        OfficeArtRecordData::ChildAnchor(_)
                                            if office_record.header.record_type == 0xf00d =>
                                        {
                                            *office_art_compatibility_anchors
                                                .entry(0xf00d)
                                                .or_default() += 1;
                                        }
                                        OfficeArtRecordData::EmptyCompatibilityAtom => {
                                            *office_art_empty_compatibility_atoms
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                            assert_eq!(office_record.header.declared_length, 0);
                                        }
                                            _ => {}
                                        }
                                    });
                                    assert_eq!(
                                        partial.available_len(),
                                        partial.to_bytes().unwrap().len()
                                    );
                                }
                                MsoDrawingData::Incomplete { bytes, reason } => {
                                    drawing_group_incomplete_bytes += bytes.len();
                                    drawing_group_incomplete.push(format!(
                                        "{}: BIFF offset {}, {} bytes, head {:02x?}: {reason}",
                                        path.display(),
                                        record.offset,
                                        bytes.len(),
                                        &bytes[..bytes.len().min(32)]
                                    ));
                                }
                            }
                        }
                        BiffRecordData::MsoDrawing(drawing) => {
                            drawing_records += 1;
                            drawing_segments += drawing.physical_segments.len();
                            drawing_id_compatibility_segments += drawing
                                .physical_segments
                                .iter()
                                .filter(|segment| segment.record_type == 0x00ac)
                                .count();
                            drawing_host_records += drawing.host_records.len();
                            for host in &drawing.host_records {
                                match &host.data {
                                    MsoDrawingHostData::Obj(obj) => {
                                        drawing_obj_typed += 1;
                                        for subrecord in &obj.subrecords {
                                            if let ObjSubrecordData::PictureFormula(picture) =
                                                &subrecord.data
                                            {
                                                obj_picture_formulas += 1;
                                                obj_picture_embed_info += usize::from(matches!(
                                                    &picture.formula.data,
                                                    ObjFormulaData::Parsed {
                                                        embed_info: Some(_),
                                                        ..
                                                    }
                                                ));
                                                obj_picture_positions += usize::from(
                                                    picture.control_stream_position.is_some(),
                                                );
                                                obj_picture_control_stream_sizes += usize::from(
                                                    picture.control_stream_size.is_some(),
                                                );
                                                obj_picture_keys +=
                                                    usize::from(picture.key.is_some());
                                                obj_picture_compatibility_bytes +=
                                                    picture.compatibility_trailing.len();
                                            }
                                            if let ObjSubrecordData::Raw(payload) = &subrecord.data
                                            {
                                                let stats = drawing_obj_subrecord_raw
                                                    .entry(subrecord.subrecord_type)
                                                    .or_default();
                                                stats.records += 1;
                                                stats.bytes += payload.len();
                                                stats.lengths.insert(payload.len());
                                            }
                                        }
                                    }
                                    MsoDrawingHostData::ObjCompatibility { value, .. } => {
                                        obj_id_compatibility_records += 1;
                                        drawing_obj_typed += 1;
                                        for subrecord in &value.subrecords {
                                            if let ObjSubrecordData::PictureFormula(picture) =
                                                &subrecord.data
                                            {
                                                obj_picture_formulas += 1;
                                                obj_picture_embed_info += usize::from(matches!(
                                                    &picture.formula.data,
                                                    ObjFormulaData::Parsed {
                                                        embed_info: Some(_),
                                                        ..
                                                    }
                                                ));
                                                obj_picture_positions += usize::from(
                                                    picture.control_stream_position.is_some(),
                                                );
                                                obj_picture_control_stream_sizes += usize::from(
                                                    picture.control_stream_size.is_some(),
                                                );
                                                obj_picture_keys +=
                                                    usize::from(picture.key.is_some());
                                                obj_picture_compatibility_bytes +=
                                                    picture.compatibility_trailing.len();
                                            }
                                            if let ObjSubrecordData::Raw(payload) = &subrecord.data
                                            {
                                                let stats = drawing_obj_subrecord_raw
                                                    .entry(subrecord.subrecord_type)
                                                    .or_default();
                                                stats.records += 1;
                                                stats.bytes += payload.len();
                                                stats.lengths.insert(payload.len());
                                            }
                                        }
                                    }
                                    MsoDrawingHostData::Txo(txo) => {
                                        drawing_txo_typed += 1;
                                        match txo.context {
                                            TxoContext::Control(_) => {
                                                drawing_txo_control_contexts += 1
                                            }
                                            TxoContext::Reserved { .. } => {
                                                drawing_txo_reserved_contexts += 1
                                            }
                                            TxoContext::Undetermined { .. } => {
                                                drawing_txo_undetermined_contexts += 1
                                            }
                                        }
                                        drawing_txo_formula_bytes +=
                                            usize::from(txo.formula.declared_length);
                                        if txo.formula.declared_length != 0 {
                                            *drawing_txo_formula_shapes
                                                .entry(usize::from(txo.formula.declared_length))
                                                .or_default() += 1;
                                        }
                                        match &txo.formula.data {
                                            ObjFormulaData::Parsed { .. } => {
                                                drawing_txo_formula_typed += 1;
                                            }
                                            ObjFormulaData::Opaque(_) => {
                                                drawing_txo_formula_opaque += 1;
                                            }
                                            ObjFormulaData::Empty => {}
                                        }
                                        drawing_txo_trailing_bytes += txo.trailing.len();
                                    }
                                    MsoDrawingHostData::Note(_) => drawing_note_typed += 1,
                                    MsoDrawingHostData::Raw {
                                        record_type: 0x01b6,
                                        ..
                                    } => drawing_txo_raw += 1,
                                    MsoDrawingHostData::Raw {
                                        record_type: 0x001c,
                                        ..
                                    } => drawing_note_raw += 1,
                                    MsoDrawingHostData::Raw {
                                        record_type: 0x005d,
                                        payload,
                                    } => {
                                        drawing_obj_raw += 1;
                                        drawing_obj_raw_bytes += payload.len();
                                    }
                                    MsoDrawingHostData::Raw { .. } => {}
                                }
                            }
                            match &drawing.data {
                                MsoDrawingData::Complete(stream) => {
                                    drawing_complete += 1;
                                    stream.visit(|office_record| {
                                        audit_office_art_properties(
                                            office_record,
                                            OfficeArtPropertyAudit {
                                                simple: &mut office_art_simple_properties,
                                                complex: &mut office_art_complex_properties,
                                                utf16: &mut office_art_utf16_properties,
                                                empty_complex: &mut office_art_empty_complex_properties,
                                                metro_blobs: &mut office_art_metro_blobs,
                                                metro_blob_entries: &mut office_art_metro_blob_entries,
                                                hyperlinks: &mut office_art_hyperlinks,
                                                hyperlink_nonparsed: &mut office_art_hyperlink_nonparsed,
                                                hyperlink_trailing_bytes: &mut office_art_hyperlink_trailing_bytes,
                                                array_headers: &mut office_art_array_headers,
                                                trailing_bytes: &mut office_art_property_table_trailing_bytes,
                                            },
                                        );
                                        match &office_record.data {
                                            OfficeArtRecordData::CompatibilityContainer(_) => {
                                                *office_art_compatibility_containers
                                                    .entry(office_record.header.record_type)
                                                    .or_default() += 1;
                                                assert_ne!(office_record.header.version, 0x0f);
                                            }
                                            OfficeArtRecordData::PropertyTable(table)
                                                if office_record.header.record_type == 0xf043 =>
                                            {
                                                office_art_compatibility_property_tables += 1;
                                                assert_eq!(office_record.header.instance, 0x0010);
                                                assert_eq!(table.properties.len(), 8);
                                            }
                                            OfficeArtRecordData::ChildAnchor(_)
                                                if matches!(
                                                    office_record.header.record_type,
                                                    0x0000 | 0xf0aa
                                                ) =>
                                            {
                                                *office_art_compatibility_anchors
                                                    .entry(office_record.header.record_type)
                                                    .or_default() += 1;
                                            }
                                            OfficeArtRecordData::SoftMakerNativeProperties(
                                                value,
                                            ) => {
                                                office_art_softmaker_native_records += 1;
                                                office_art_softmaker_native_properties +=
                                                    value.properties.len();
                                                for property in &value.properties {
                                                    let length = property.data.encoded_len();
                                                    office_art_softmaker_native_payload_bytes +=
                                                        length;
                                                    office_art_softmaker_native_unparsed_bytes +=
                                                        property.data.unparsed_byte_count();
                                                    if let SoftMakerNativePropertyData::Selector6 {
                                                        font_name,
                                                        trailing,
                                                        ..
                                                    } = &property.data
                                                    {
                                                        office_art_softmaker_selector6 += 1;
                                                        assert_eq!(
                                                            *font_name,
                                                            [b'A' as u16, b'r' as u16,
                                                             b'i' as u16, b'a' as u16,
                                                             b'l' as u16, 0]
                                                        );
                                                        assert_eq!(*trailing, 0);
                                                    }
                                                    *office_art_softmaker_native_shapes
                                                        .entry((
                                                            property.selector,
                                                            property.reserved,
                                                            length,
                                                        ))
                                                        .or_default() += 1;
                                                }
                                            }
                                            OfficeArtRecordData::Fbse(_)
                                                if office_record.header.record_type == 0xe007 =>
                                            {
                                                office_art_compatibility_fbse += 1;
                                            }
                                            OfficeArtRecordData::EmptyCompatibilityAtom => {
                                                *office_art_empty_compatibility_atoms
                                                    .entry(office_record.header.record_type)
                                                    .or_default() += 1;
                                                assert_eq!(
                                                    office_record.header.declared_length,
                                                    0
                                                );
                                            }
                                            OfficeArtRecordData::Atom(payload) => {
                                                let stats = office_art_atom_stats
                                                    .entry(office_record.header.record_type)
                                                    .or_default();
                                                stats.records += 1;
                                                stats.bytes += payload.len();
                                                stats.lengths.insert(payload.len());
                                                if office_art_atom_locations.len() < 32 {
                                                    office_art_atom_locations.push(format!(
                                                        "{}: BIFF offset {}, OfficeArt v{:#x}/i{:#05x}/t0x{:04x}, {} bytes, head {:02x?}",
                                                        path.display(),
                                                        record.offset,
                                                        office_record.header.version,
                                                        office_record.header.instance,
                                                        office_record.header.record_type,
                                                        payload.len(),
                                                        &payload[..payload.len().min(96)]
                                                    ));
                                                }
                                            }
                                            OfficeArtRecordData::MetafileBlip(blip) => {
                                                match &blip.file_data {
                                                    OfficeArtMetafileData::Emf { .. } => {
                                                        office_art_emf_typed += 1;
                                                    }
                                                    OfficeArtMetafileData::Wmf { .. } => {
                                                        office_art_wmf_typed += 1;
                                                    }
                                                    OfficeArtMetafileData::Pict { .. } => {
                                                        office_art_pict_typed += 1;
                                                    }
                                                    OfficeArtMetafileData::Opaque {
                                                        reason,
                                                        decoded,
                                                        original_encoded,
                                                    } => {
                                                        let stats = office_art_metafile_opaque
                                                            .entry(
                                                                office_record.header.record_type,
                                                            )
                                                            .or_default();
                                                        stats.records += 1;
                                                        stats.bytes += original_encoded.len();
                                                        stats
                                                            .lengths
                                                            .insert(original_encoded.len());
                                                        let decoded_len = decoded.as_ref().map_or(
                                                            original_encoded.len(),
                                                            Vec::len,
                                                        );
                                                        let reason_stats = office_art_metafile_opaque_reasons
                                                            .entry(*reason)
                                                            .or_default();
                                                        reason_stats.records += 1;
                                                        reason_stats.bytes += decoded_len;
                                                        reason_stats.lengths.insert(decoded_len);
                                                    }
                                                }
                                            }
                                            OfficeArtRecordData::BitmapBlip(blip)
                                                if office_record.header.record_type == 0xf01f =>
                                            {
                                                match &blip.file_data {
                                                    OfficeArtBitmapData::Dib(_) => {
                                                        office_art_dib_typed += 1;
                                                    }
                                                    OfficeArtBitmapData::Encoded(_) => {
                                                        office_art_dib_opaque += 1;
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    });
                                }
                                MsoDrawingData::Partial(partial) => {
                                    drawing_partial += 1;
                                    *drawing_partial_boundaries
                                        .entry(drawing.following_record_type)
                                        .or_default() += 1;
                                    drawing_partial_complete_records +=
                                        partial.complete_record_count();
                                    drawing_partial_incomplete_records +=
                                        partial.incomplete_record_count();
                                    drawing_partial_unparsed_bytes += partial.unparsed_byte_count();
                                    partial.visit_incomplete(|office_record| match &office_record
                                        .data
                                    {
                                        OfficeArtIncompleteRecordData::Atom {
                                            available_payload,
                                        } => {
                                            *drawing_partial_leaf_shapes
                                                .entry((
                                                    office_record.header.record_type,
                                                    office_record.header.instance,
                                                    office_record.header.declared_length,
                                                    available_payload.len(),
                                                ))
                                                .or_default() += 1;
                                            drawing_partial_leaf_samples.push(format!(
                                                "{}: BIFF offset {}, v{:#x}/i{:#05x}/t0x{:04x} declared {} available {} {:02x?}",
                                                path.display(), record.offset,
                                                office_record.header.version,
                                                office_record.header.instance,
                                                office_record.header.record_type,
                                                office_record.header.declared_length,
                                                available_payload.len(), available_payload
                                            ));
                                        }
                                        OfficeArtIncompleteRecordData::PropertyTable(table) => {
                                            office_art_incomplete_property_tables += 1;
                                            office_art_incomplete_property_entries +=
                                                table.entries.len();
                                            office_art_incomplete_fixed_bytes +=
                                                table.incomplete_fixed_entry.encoded_len();
                                            office_art_incomplete_complex_bytes +=
                                                table.available_complex_len();
                                            office_art_incomplete_complex_unparsed_bytes +=
                                                table.unparsed_complex_len();
                                            for fragment in &table.complex_fragments {
                                                match &fragment.data {
                                                    OfficeArtComplexPropertyData::Array(_) => {
                                                        office_art_incomplete_array_fragments += 1;
                                                    }
                                                    OfficeArtComplexPropertyData::Bytes(bytes) => {
                                                        office_art_incomplete_generic_fragment_bytes += bytes.len();
                                                    }
                                                }
                                            }
                                            if let OfficeArtIncompletePropertyEntry::LowWord {
                                                property_id,
                                                is_blip_id,
                                                is_complex,
                                                value_low,
                                            } = &table.incomplete_fixed_entry
                                            {
                                                office_art_incomplete_low_words += 1;
                                                assert_eq!(*property_id, 0x01ff);
                                                assert!(!*is_blip_id);
                                                assert!(!*is_complex);
                                                assert_eq!(*value_low, 0x0010);
                                            }
                                            office_art_incomplete_property_samples
                                                .push(incomplete_property_sample(table));
                                        }
                                        OfficeArtIncompleteRecordData::RecoveredSequence {
                                            prefix,
                                            ..
                                        } => {
                                            drawing_partial_unparsed_prefix_bytes +=
                                                prefix.unparsed_byte_count();
                                        }
                                        OfficeArtIncompleteRecordData::Container(_) => {}
                                        OfficeArtIncompleteRecordData::FbseWithUnderreportedLength(_) => {}
                                    });
                                    for length in partial.trailing_header_lengths() {
                                        *drawing_partial_header_tail_lengths
                                            .entry(length)
                                            .or_default() += 1;
                                    }
                                    partial.visit_complete(|office_record| {
                                        audit_office_art_properties(
                                            office_record,
                                            OfficeArtPropertyAudit {
                                                simple: &mut office_art_simple_properties,
                                                complex: &mut office_art_complex_properties,
                                                utf16: &mut office_art_utf16_properties,
                                                empty_complex: &mut office_art_empty_complex_properties,
                                                metro_blobs: &mut office_art_metro_blobs,
                                                metro_blob_entries: &mut office_art_metro_blob_entries,
                                                hyperlinks: &mut office_art_hyperlinks,
                                                hyperlink_nonparsed: &mut office_art_hyperlink_nonparsed,
                                                hyperlink_trailing_bytes: &mut office_art_hyperlink_trailing_bytes,
                                                array_headers: &mut office_art_array_headers,
                                                trailing_bytes: &mut office_art_property_table_trailing_bytes,
                                            },
                                        );
                                        match &office_record.data {
                                        OfficeArtRecordData::Atom(payload) => {
                                            let stats = office_art_atom_stats
                                                .entry(office_record.header.record_type)
                                                .or_default();
                                            stats.records += 1;
                                            stats.bytes += payload.len();
                                            stats.lengths.insert(payload.len());
                                            office_art_atom_locations.push(format!(
                                                "{}: BIFF offset {}, partial OfficeArt v{:#x}/i{:#05x}/t0x{:04x}, {} bytes, head {:02x?}",
                                                path.display(), record.offset,
                                                office_record.header.version,
                                                office_record.header.instance,
                                                office_record.header.record_type,
                                                payload.len(),
                                                &payload[..payload.len().min(96)]
                                            ));
                                        }
                                        OfficeArtRecordData::CompatibilityContainer(_) => {
                                            *office_art_compatibility_containers
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                            assert_ne!(office_record.header.version, 0x0f);
                                        }
                                        OfficeArtRecordData::IncompletePropertyTable(table) => {
                                            office_art_incomplete_property_tables += 1;
                                            office_art_incomplete_property_entries += table.entries.len();
                                            office_art_incomplete_fixed_bytes +=
                                                table.incomplete_fixed_entry.encoded_len();
                                            office_art_incomplete_complex_bytes +=
                                                table.available_complex_len();
                                            office_art_incomplete_complex_unparsed_bytes +=
                                                table.unparsed_complex_len();
                                            for fragment in &table.complex_fragments {
                                                match &fragment.data {
                                                    OfficeArtComplexPropertyData::Array(_) => {
                                                        office_art_incomplete_array_fragments += 1;
                                                    }
                                                    OfficeArtComplexPropertyData::Bytes(bytes) => {
                                                        office_art_incomplete_generic_fragment_bytes += bytes.len();
                                                    }
                                                }
                                            }
                                            office_art_incomplete_property_samples
                                                .push(incomplete_property_sample(table));
                                        }
                                        OfficeArtRecordData::ChildAnchor(_)
                                            if office_record.header.record_type == 0xf00d =>
                                        {
                                            *office_art_compatibility_anchors
                                                .entry(0xf00d)
                                                .or_default() += 1;
                                        }
                                        OfficeArtRecordData::EmptyCompatibilityAtom => {
                                            *office_art_empty_compatibility_atoms
                                                .entry(office_record.header.record_type)
                                                .or_default() += 1;
                                            assert_eq!(office_record.header.declared_length, 0);
                                        }
                                            _ => {}
                                        }
                                    });
                                    assert_eq!(
                                        partial.available_len(),
                                        partial.to_bytes().unwrap().len()
                                    );
                                }
                                MsoDrawingData::Incomplete { bytes, reason } => {
                                    drawing_incomplete_bytes += bytes.len();
                                    *drawing_incomplete_boundaries
                                        .entry(drawing.following_record_type)
                                        .or_default() += 1;
                                    drawing_incomplete.push(format!(
                                        "{}: BIFF offset {}, {} bytes, next {:?}, head {:02x?}: {reason}",
                                        path.display(),
                                        record.offset,
                                        bytes.len(),
                                        drawing.following_record_type,
                                        &bytes[..bytes.len().min(32)]
                                    ));
                                }
                            }
                        }
                        BiffRecordData::XfExt(xf_ext) => {
                            for property in &xf_ext.properties {
                                match &property.data {
                                    ExtPropertyData::FullColor {
                                        property_type: 0x0048,
                                        color,
                                    } => {
                                        xf_ext_compatibility_full_colors += 1;
                                        assert_eq!(color.color_type, 3);
                                        assert_eq!(color.tint, 0x3fff);
                                        assert_eq!(color.color_value, 4);
                                        assert_eq!(color.unused, 0x0032_002d_20ac_0024);
                                    }
                                    ExtPropertyData::Gradient { payload } => {
                                        xf_ext_unparsed_bytes += payload.len();
                                        let stats = xf_ext_stats.entry(0x0006).or_default();
                                        stats.records += 1;
                                        stats.bytes += payload.len();
                                        stats.lengths.insert(payload.len());
                                    }
                                    ExtPropertyData::Unknown {
                                        property_type,
                                        payload,
                                    } => {
                                        xf_ext_unknown_types.insert(*property_type);
                                        xf_ext_unparsed_bytes += payload.len();
                                        let stats = xf_ext_stats.entry(*property_type).or_default();
                                        stats.records += 1;
                                        stats.bytes += payload.len();
                                        stats.lengths.insert(payload.len());
                                        xf_ext_unknown_locations.push(format!(
                                            "{}: BIFF offset {}, ExtProp 0x{property_type:04x} {:02x?}",
                                            path.display(),
                                            record.offset,
                                            payload
                                        ));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        BiffRecordData::StyleExt(style_ext) => {
                            for property in &style_ext.properties {
                                if let XfPropertyData::Unparsed(payload) = &property.data {
                                    style_ext_unparsed_bytes += payload.len();
                                    let stats =
                                        style_ext_stats.entry(property.property_type).or_default();
                                    stats.records += 1;
                                    stats.bytes += payload.len();
                                    stats.lengths.insert(payload.len());
                                }
                            }
                        }
                        BiffRecordData::Dxf(dxf) => {
                            dxf_records += 1;
                            for property in &dxf.properties {
                                if let XfPropertyData::Unparsed(payload) = &property.data {
                                    dxf_unparsed_bytes += payload.len();
                                    let stats =
                                        dxf_stats.entry(property.property_type).or_default();
                                    stats.records += 1;
                                    stats.bytes += payload.len();
                                    stats.lengths.insert(payload.len());
                                }
                            }
                        }
                        BiffRecordData::ConditionalFormattingExtension(cfex) => {
                            cfex_records += 1;
                            if cfex.is_cf12 == 0 {
                                cfex_non_cf12_records += 1;
                            } else {
                                cfex_cf12_records += 1;
                            }
                            if let Some(DxfN12::Formatting { extension, .. }) = cfex
                                .content
                                .as_ref()
                                .and_then(|content| content.format.as_ref())
                            {
                                cfex_formats += 1;
                                if let Some(extension) = extension {
                                    for property in &extension.properties {
                                        match &property.data {
                                            ExtPropertyData::Gradient { payload }
                                            | ExtPropertyData::Unknown { payload, .. } => {
                                                cfex_extension_unparsed_bytes += payload.len();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        BiffRecordData::ConditionalFormatting12(cf12) => {
                            cf12_records += 1;
                            *cf12_types.entry(cf12.condition_type).or_default() += 1;
                            cf12_unparsed_formula_bytes += cf12.formula1.unparsed_tail.len()
                                + cf12.formula2.unparsed_tail.len()
                                + cf12.active_formula.unparsed_tail.len();
                            match &cf12.condition_data {
                                Cf12ConditionData::Gradient(value) => {
                                    cf12_unparsed_formula_bytes += value
                                        .interpolation
                                        .iter()
                                        .map(|item| item.value.formula.unparsed_tail.len())
                                        .sum::<usize>();
                                }
                                Cf12ConditionData::DataBar(value) => {
                                    cf12_unparsed_formula_bytes +=
                                        value.minimum.formula.unparsed_tail.len()
                                            + value.maximum.formula.unparsed_tail.len();
                                }
                                Cf12ConditionData::Multistate(value) => {
                                    cf12_unparsed_formula_bytes += value
                                        .states
                                        .iter()
                                        .map(|item| item.value.formula.unparsed_tail.len())
                                        .sum::<usize>();
                                }
                                Cf12ConditionData::None | Cf12ConditionData::Filter(_) => {}
                            }
                        }
                        BiffRecordData::CrtMlFrt(crt_ml_frt) => {
                            crt_ml_frt_records += 1;
                            xml_tk_records += crt_ml_frt.chain.records.len();
                            for token in &crt_ml_frt.chain.records {
                                let kind = match token.data {
                                    XmlTkData::Start => "start",
                                    XmlTkData::End => "end",
                                    XmlTkData::Boolean { .. } => "boolean",
                                    XmlTkData::Double { .. } => "double",
                                    XmlTkData::DWord(_) => "dword",
                                    XmlTkData::String(_) => "string",
                                    XmlTkData::Token(_) => "token",
                                    XmlTkData::Blob(_) => "blob",
                                };
                                *xml_tk_kinds.entry(kind).or_default() += 1;
                            }
                        }
                        BiffRecordData::Sst(sst) => {
                            sst_records += 1;
                            sst_strings += sst.strings.len();
                            sst_extension_bytes += sst
                                .strings
                                .iter()
                                .map(|string| {
                                    usize::try_from(string.declared_extension_length.unwrap_or(0))
                                        .unwrap_or(usize::MAX)
                                })
                                .sum::<usize>();
                            sst_extension_unparsed_bytes += sst
                                .strings
                                .iter()
                                .map(|string| string.extension.unparsed_byte_count())
                                .sum::<usize>();
                            for string in &sst.strings {
                                let (kind, bytes) = match &string.extension {
                                    SstExtensionData::Unparsed(payload) => {
                                        if sst_extension_unparsed_samples.len() < 10 {
                                            sst_extension_unparsed_samples.push(format!(
                                                "{} offset {} unparsed {:02x?}",
                                                path.display(),
                                                record.offset,
                                                payload
                                            ));
                                        }
                                        ("unparsed", payload.len())
                                    }
                                    SstExtensionData::ExtRst(ext) => match &ext.body {
                                        ExtRstBody::OldStyle { payload } => {
                                            if sst_extension_unparsed_samples.len() < 10 {
                                                sst_extension_unparsed_samples.push(format!(
                                                    "{} offset {} old-style reserved {:#06x} payload {:02x?}",
                                                    path.display(), record.offset, ext.reserved, payload
                                                ));
                                            }
                                            ("old-style", payload.len())
                                        }
                                        ExtRstBody::InvalidMarker { payload } => {
                                            if sst_extension_unparsed_samples.len() < 10 {
                                                sst_extension_unparsed_samples.push(format!(
                                                    "{} offset {} invalid-marker reserved {:#06x} payload {:02x?}",
                                                    path.display(), record.offset, ext.reserved, payload
                                                ));
                                            }
                                            ("invalid-marker", payload.len())
                                        }
                                        ExtRstBody::TruncatedPhoneticHeader {
                                            declared_data_size,
                                            font_index,
                                            formatting_flags,
                                            declared_run_count,
                                            declared_character_count,
                                        } => {
                                            sst_truncated_phonetic_headers += 1;
                                            assert_eq!(ext.reserved, 0x000c);
                                            assert_eq!(*declared_data_size, 8);
                                            assert_eq!(*font_index, 0x0037);
                                            assert!(formatting_flags.is_empty());
                                            assert_eq!(*declared_run_count, 0);
                                            assert_eq!(*declared_character_count, 0);
                                            ("truncated-phonetic-header", 0)
                                        }
                                        ExtRstBody::Phonetic {
                                            inner_trailing,
                                            outer_trailing,
                                            ..
                                        } => {
                                            if (!inner_trailing.is_empty()
                                                || !outer_trailing.is_empty())
                                                && sst_phonetic_trailing_samples.len() < 10
                                            {
                                                sst_phonetic_trailing_samples.push(format!(
                                                    "{} offset {} inner {:02x?} outer {:02x?}",
                                                    path.display(),
                                                    record.offset,
                                                    inner_trailing,
                                                    outer_trailing
                                                ));
                                            }
                                            (
                                                "phonetic-trailing",
                                                inner_trailing.len() + outer_trailing.len(),
                                            )
                                        }
                                    },
                                    SstExtensionData::None => continue,
                                };
                                if bytes != 0 {
                                    let stats =
                                        sst_extension_unparsed_stats.entry(kind).or_default();
                                    stats.records += 1;
                                    stats.bytes += bytes;
                                    stats.lengths.insert(bytes);
                                }
                            }
                            sst_trailing_bytes += sst.trailing.len();
                            if let SstCompletion::Truncated {
                                first_unparsed_string,
                                reason,
                            } = &sst.completion
                            {
                                sst_truncated.push(format!(
                                    "{}: BIFF offset {}, stopped at string {first_unparsed_string}: {reason}",
                                    path.display(),
                                    record.offset
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                legacy += 1;
            }
            let saved = file.to_bytes_preserving_compatibility()?;
            let reopened = XlsFile::from_bytes_compatible(&saved)?.value;
            if reopened.workbooks != file.workbooks || reopened.revision_log != file.revision_log {
                return Err(olecfsdk::Error::invalid(
                    0,
                    "typed XLS file tree changed after file-root write and reopen",
                ));
            }
            root_reopened_files += 1;
            Ok::<_, olecfsdk::Error>(())
        })();
        if let Err(error) = result {
            if expected_invalid.contains(&path) {
                observed_invalid.insert(path.clone());
            } else {
                failures.push(format!(
                    "{}:{}: {error}",
                    path.display(),
                    entry.path.display()
                ));
            }
        }
    }
    assert!(checked > 0, "no Workbook/Book streams found in XLS corpus");
    assert_eq!(checked, 734, "XLS file-root coverage changed");
    assert_eq!(
        root_parsed_files, 725,
        "XLS file-root parse coverage changed"
    );
    assert_eq!(
        root_reopened_files, 725,
        "XLS file-root reopen coverage changed"
    );
    assert!(biff8 > 0, "no BIFF8 workbook streams found in XLS corpus");
    assert!(
        unknown_types.is_empty(),
        "unknown BIFF record types remain: {unknown_types:#06x?}"
    );
    assert_eq!(
        null_compatibility_records, 4,
        "BIFF8 null compatibility-marker coverage changed"
    );
    if !failures.is_empty() {
        eprintln!(
            "XLS parse failures before coverage assertions:\n{}",
            failures.join("\n")
        );
    }
    assert_eq!(
        hyperlink_truncated_records, 1,
        "truncated Hyperlink corpus coverage changed"
    );
    assert_eq!(
        hyperlink_truncated_bytes, 0,
        "truncated Hyperlink records retained generic payload bytes"
    );
    assert_eq!(
        hyperlink_truncated_url_records, 1,
        "truncated URL-moniker coverage changed"
    );
    assert_eq!(
        hyperlink_truncated_url_address_bytes, 8_168,
        "truncated URL-moniker typed address coverage changed"
    );
    assert_eq!(
        root_truncated_diagnostics,
        hyperlink_truncated_records
            + formula_missing_extra
            + data_validation_missing_extra
            + conditional_formatting_missing_extra
            + linked_data_missing_extra
            + name_missing_extra
            + pls_truncated_driver_extra
            + drawing_group_partial
            + drawing_partial
            + sst_truncated.len(),
        "every retained truncated XLS structure requires one root diagnostic; PLS samples: {pls_truncated_driver_extra_samples:?}"
    );
    assert_eq!(
        root_nonconforming_diagnostics,
        hyperlink_compatibility_records
            + feature_header_malformed
            + root_workbook_structure_diagnostics
            + root_bof_diagnostics,
        "every retained nonconforming XLS structure requires one root diagnostic"
    );
    assert_eq!(
        root_workbook_structure_diagnostics, 37,
        "Workbook stream name/version/topology compatibility coverage changed"
    );
    assert_eq!(
        root_bof_diagnostics, 756,
        "BOF MUST-field compatibility coverage changed"
    );
    assert_eq!(drawing_obj_raw, 0, "Obj records fell back to raw payloads");
    assert_eq!(
        formula_unparsed_rgce_bytes, 0,
        "Formula records retained unparsed rgce bytes"
    );
    assert_eq!(formula_rgcb_bytes, 0, "Formula records retained rgcb bytes");
    assert_eq!(
        formula_missing_extra, 1,
        "Formula missing-extra compatibility coverage changed"
    );
    assert_eq!(shared_formula_records, 1_346);
    assert_eq!(shared_formula_unparsed_rgce_bytes, 0);
    assert_eq!(shared_formula_rgcb_bytes, 0);
    assert_eq!(array_records, 145, "Array formula corpus coverage changed");
    assert_eq!(
        array_unparsed_rgce_bytes, 0,
        "Array formulas retained unparsed rgce bytes"
    );
    assert_eq!(
        array_rgcb_bytes, 0,
        "Array formulas retained unparsed rgcb bytes"
    );
    assert_eq!(
        data_validation_unparsed_rgce_bytes, 0,
        "DV formulas retained unparsed rgce bytes"
    );
    assert_eq!(
        data_validation_missing_extra, 0,
        "DV formulas require forbidden extra-data structures"
    );
    assert_eq!(
        conditional_formatting_unparsed_rgce_bytes, 0,
        "CF formulas retained unparsed rgce bytes"
    );
    assert_eq!(
        conditional_formatting_missing_extra, 0,
        "CF formulas require forbidden extra-data structures"
    );
    assert_eq!(
        linked_data_unparsed_rgce_bytes, 0,
        "LinkedData formulas retained unparsed rgce bytes"
    );
    assert_eq!(
        linked_data_missing_extra, 0,
        "LinkedData formulas require unsupported extra-data structures"
    );
    assert_eq!(sup_book_records, 781);
    assert_eq!(sup_book_compatibility, 0);
    assert_eq!(sup_book_trailing_bytes, 0);
    assert_eq!(extern_name_records, 730);
    assert_eq!(extern_name_compatibility_records, 0);
    assert_eq!(extern_name_compatibility_bytes, 0);
    assert_eq!(hyperlink_records, 409);
    assert_eq!(hyperlink_compatibility_records, 0);
    assert_eq!(hyperlink_compatibility_bytes, 0);
    assert_eq!(hyperlink_trailing_bytes, 0);
    assert_eq!(
        feature_header_malformed, 1,
        "FeatHdr malformed-state count changed"
    );
    assert_eq!(
        feature_header_malformed_bytes, 334,
        "FeatHdr malformed payload is no longer the known CVE fixture"
    );
    assert_eq!(feature11_records, 8, "Feature11 corpus coverage changed");
    assert_eq!(feature11_embedded_auto_filters, 0);
    assert_eq!(feature11_xml_maps, 0);
    assert_eq!(feature11_formulas, 0);
    assert_eq!(feature11_total_formulas, 0);
    assert_eq!(feature11_total_array_formulas, 0);
    assert_eq!(feature11_total_texts, 0);
    assert_eq!(feature11_wss_info, 0);
    assert_eq!(feature11_query_fields, 0);
    assert_eq!(feature11_cached_headers, 0);
    assert_eq!(feature11_deleted_id_lists, 0);
    assert_eq!(feature11_changed_id_lists, 0);
    assert_eq!(feature11_invalid_cell_lists, 0);
    assert_eq!(feature_records, 36, "Feat corpus coverage changed");
    assert_eq!(dconn_records, 4, "DConn corpus coverage changed");
    assert_eq!(text_query_records, 3, "TxtQry corpus coverage changed");
    assert_eq!(qsi_sx_tag_records, 28, "QsiSXTag corpus coverage changed");
    assert_eq!(sx_view_ex9_records, 24, "SXViewEx9 corpus coverage changed");
    assert_eq!(
        db_query_ext_records, 4,
        "DBQueryExt corpus coverage changed"
    );
    assert_eq!(
        hyperlink_tooltip_records, 3,
        "HLinkTooltip corpus coverage changed"
    );
    assert_eq!(
        continue_frt12_records, 0,
        "ContinueFrt12 corpus coverage changed"
    );
    assert_eq!(sx_addl_records, 329, "SXAddl corpus coverage changed");
    assert_eq!(ent_ex_u2_records, 154, "EntExU2 corpus coverage changed");
    assert_eq!(
        ent_ex_u2_cache_bytes, 7008,
        "EntExU2 cache byte coverage changed"
    );
    assert_eq!(bk_him_records, 4, "BkHim corpus coverage changed");
    assert_eq!(bk_him_bitmaps, 4, "BkHim bitmap coverage changed");
    assert_eq!(bk_him_native, 0, "BkHim native coverage changed");
    assert_eq!(bk_him_continued, 1, "BkHim continuation coverage changed");
    assert_eq!(
        bk_him_image_bytes, 70824,
        "BkHim image byte coverage changed"
    );
    assert_eq!(im_data_records, 1, "ImData corpus coverage changed");
    assert_eq!(
        im_data_bitmap_bytes, 6396,
        "ImData bitmap byte coverage changed"
    );
    assert_eq!(
        real_time_data_records, 1,
        "RealTimeData corpus coverage changed"
    );
    assert_eq!(
        real_time_data_topic_segments, 3,
        "RealTimeData topic coverage changed"
    );
    assert_eq!(real_time_data_malformed, 0);
    assert_eq!(real_time_data_malformed_bytes, 0);
    assert_eq!(real_time_data_corrupt_error_discriminators, 1);
    assert_eq!(real_time_data_cells, 1);
    assert_eq!(sort_records, 25, "Sort corpus coverage changed");
    assert_eq!(sort_keys, 32, "Sort key coverage changed");
    assert_eq!(
        sort_compressed_keys, 32,
        "Sort key encoding coverage changed"
    );
    assert_eq!(sort_data_records, 4, "SortData corpus coverage changed");
    assert_eq!(
        sort_data_conditions, 4,
        "SortData condition coverage changed"
    );
    assert_eq!(sort_data_table, 2, "SortData table coverage changed");
    assert_eq!(
        sort_data_auto_filter, 2,
        "SortData AutoFilter coverage changed"
    );
    assert_eq!(sort_conditions, 4, "SortCond12 corpus coverage changed");
    assert_eq!(
        sort_conditions_descending, 1,
        "SortCond12 descending coverage changed"
    );
    assert_eq!(auto_filter_records, 8, "AutoFilter corpus coverage changed");
    assert_eq!(auto_filter_strings, 7, "AutoFilter string coverage changed");
    assert_eq!(
        auto_filter_string_characters, 44,
        "AutoFilter string character coverage changed"
    );
    assert_eq!(auto_filter_numbers, 1, "AutoFilter number coverage changed");
    assert_eq!(sx_format_records, 60, "SxFormat corpus coverage changed");
    assert_eq!(sx_format_applied, 60, "SxFormat applied coverage changed");
    assert_eq!(wopt_records, 2, "WOpt corpus coverage changed");
    assert_eq!(
        wopt_component_characters, 3,
        "WOpt component-location coverage changed"
    );
    assert_eq!(wopt_future_bytes, 0, "WOpt future-byte coverage changed");
    assert_eq!(table_records, 8, "Table corpus coverage changed");
    assert_eq!(table_two_variable, 3, "Table two-variable coverage changed");
    assert_eq!(
        table_compatibility_padding, 1,
        "Table compatibility-padding coverage changed"
    );
    assert_eq!(
        standalone_obj_records, 4,
        "standalone Obj corpus coverage changed"
    );
    assert_eq!(
        standalone_obj_raw_subrecords, 0,
        "standalone Obj retained raw subrecords: {standalone_obj_raw_shapes:?}"
    );
    assert_eq!(
        standalone_obj_truncated_picture_flags, 1,
        "standalone Obj truncated FtPioGrbit coverage changed"
    );
    assert_eq!(
        standalone_obj_trailing_bytes, 45,
        "standalone Obj malformed trailing-byte coverage changed"
    );
    assert_eq!(
        formula4_compatibility_records, 3,
        "Formula4 compatibility corpus coverage changed"
    );
    assert_eq!(
        formula4_unparsed_bytes, 0,
        "Formula4 compatibility retained unparsed formula bytes"
    );
    assert_eq!(
        extern_count_records, 1,
        "ExternCount corpus coverage changed"
    );
    assert_eq!(qsi_records, 4, "Qsi corpus coverage changed");
    assert_eq!(qsi_name_characters, 59, "Qsi name coverage changed");
    assert_eq!(param_qry_records, 4, "ParamQry corpus coverage changed");
    assert_eq!(param_qry_prompts, 4, "ParamQry prompt coverage changed");
    assert_eq!(sx_select_records, 2, "SxSelect corpus coverage changed");
    assert_eq!(sx_select_extended, 1, "SxSelect extended coverage changed");
    assert_eq!(
        standalone_txo_records, 0,
        "standalone TxO corpus coverage changed"
    );
    assert_eq!(
        standalone_txo_undetermined, 0,
        "standalone TxO context coverage changed"
    );
    assert_eq!(plv_mac_records, 41, "PLV Mac corpus coverage changed");
    assert_eq!(lnext_records, 14, "LNEXT corpus coverage changed");
    assert_eq!(mkr_ext_records, 6, "MKREXT corpus coverage changed");
    assert_eq!(crt_co_opt_records, 4, "CRTCOOPT corpus coverage changed");
    assert_eq!(
        crt_co_opt_compatibility_padding, 1,
        "CRTCOOPT compatibility-padding coverage changed"
    );
    assert_eq!(
        frt_arch_id_records, 12,
        "FRTArchId$ corpus coverage changed"
    );
    assert_eq!(
        frt_arch_ids,
        BTreeMap::from([(2, 11), (4, 1)]),
        "FRTArchId$ architecture distribution changed"
    );
    assert_eq!(lh_records, 1, "LHRECORD corpus coverage changed");
    assert_eq!(lh_subrecords, 10, "LH subrecord coverage changed");
    assert_eq!(lh_margins, 4, "LH margin coverage changed");
    assert_eq!(lh_graph_views, 1, "LH graph-view coverage changed");
    assert_eq!(
        lh_graph_compatibility_extensions, 1,
        "LH graph compatibility-extension coverage changed"
    );
    assert_eq!(lh_reserved_words, 4, "LH reserved-word coverage changed");
    assert_eq!(
        lh_undocumented_type13, 1,
        "LH undocumented type-13 coverage changed"
    );
    assert_eq!(
        font_id_compatibility_records, 1,
        "Font record-ID compatibility coverage changed"
    );
    assert_eq!(
        xf_id_compatibility_records, 3,
        "XF record-ID compatibility coverage changed"
    );
    assert_eq!(
        bound_sheet_id_compatibility_records, 1,
        "BoundSheet8 record-ID compatibility coverage changed"
    );
    assert_eq!(
        chart_series_id_compatibility_records, 1,
        "ChartSeries record-ID compatibility coverage changed"
    );
    assert_eq!(
        obj_id_compatibility_records, 1,
        "Obj record-ID compatibility coverage changed"
    );
    assert_eq!(
        drawing_id_compatibility_segments, 1,
        "MsoDrawing record-ID compatibility coverage changed"
    );
    assert_eq!(
        dxf_unparsed_bytes, 0,
        "DXF records retained unparsed XFProp bytes"
    );
    assert_eq!(
        cfex_extension_unparsed_bytes, 0,
        "CFEx DXFN12 extensions retained unparsed ExtProp bytes"
    );
    assert_eq!(
        xf_ext_unparsed_bytes, 0,
        "XFExt records retained unparsed ExtProp bytes"
    );
    assert!(
        xf_ext_unknown_types.is_empty(),
        "XFExt records retained unknown property types: {xf_ext_unknown_types:#06x?}"
    );
    assert_eq!(
        xf_ext_compatibility_full_colors, 1,
        "XFExt compatibility FullColor coverage changed"
    );
    assert_eq!(style_ext_unparsed_bytes, 0);
    assert_eq!(
        sst_extension_unparsed_bytes, 0,
        "SST ExtRst values retained unparsed bytes"
    );
    assert_eq!(
        sst_truncated_phonetic_headers, 1,
        "truncated ExtRst phonetic-header coverage changed"
    );
    assert_eq!(cf12_records, 43, "CF12 corpus coverage changed");
    assert_eq!(
        cf12_unparsed_formula_bytes, 0,
        "CF12 or CFVO formulas retained unparsed token bytes"
    );
    assert_eq!(crt_ml_frt_records, 398, "CrtMlFrt corpus coverage changed");
    assert!(
        xml_tk_records > 0,
        "CrtMlFrt chains contained no XmlTk records"
    );
    assert_eq!(
        drawing_txo_undetermined_contexts, 0,
        "TxO records lost their preceding FtCmo context"
    );
    assert_eq!(drawing_txo_formula_opaque, 0);
    assert_eq!(drawing_txo_trailing_bytes, 0);
    assert_eq!(name_records, 8_145);
    assert_eq!(name_continued_records, 0);
    assert_eq!(name_unparsed_rgce_bytes, 0);
    assert_eq!(name_rgcb_tail_bytes, 0);
    assert_eq!(name_missing_extra, 0);
    assert!(
        drawing_obj_subrecord_raw.is_empty(),
        "Obj subrecords fell back to raw payloads: {drawing_obj_subrecord_raw:?}"
    );
    assert_eq!(obj_picture_formulas, 169, "FtPictFmla coverage changed");
    assert_eq!(
        obj_picture_embed_info, 169,
        "PictFmlaEmbedInfo coverage changed"
    );
    assert_eq!(obj_picture_positions, 169, "lPosInCtlStm coverage changed");
    assert_eq!(
        obj_picture_control_stream_sizes, 153,
        "cbBufInCtlStm coverage changed"
    );
    assert_eq!(obj_picture_keys, 153, "PictFmlaKey coverage changed");
    assert_eq!(
        obj_picture_compatibility_bytes, 0,
        "FtPictFmla retained compatibility bytes"
    );
    assert_eq!(
        drawing_graphs, 330,
        "complete XLS drawing-graph coverage changed"
    );
    assert_eq!(
        drawing_graphs_strict, 298,
        "strict XLS drawing-graph coverage changed"
    );
    assert_eq!(drawing_graph_drawings, 521);
    assert_eq!(drawing_graph_shapes, 3_690);
    assert_eq!(drawing_graph_absent, 380);
    assert_eq!(
        drawing_graph_shape_count_bases,
        BTreeMap::from([
            ("AllPresentShapes".to_owned(), 519),
            ("HistoricalHighWater".to_owned(), 2),
        ])
    );
    assert_eq!(
        drawing_graph_current_shape_id_relations,
        BTreeMap::from([
            ("AbovePresentTree".to_owned(), 59),
            ("BelowPresentTree".to_owned(), 1),
            ("EqualToPresentTree".to_owned(), 461),
        ])
    );
    assert_eq!(
        drawing_graph_maximum_shape_id_relations,
        BTreeMap::from([
            ("AbovePresentTree".to_owned(), 158),
            ("BelowPresentTree".to_owned(), 1),
            ("EmptyNonzero".to_owned(), 4),
            ("EqualToPresentTree".to_owned(), 167),
        ])
    );
    assert_eq!(
        drawing_graph_saved_shape_count_relations,
        BTreeMap::from([
            ("AbovePresentTree".to_owned(), 31),
            ("EqualToPresentTree".to_owned(), 299),
        ])
    );
    assert_eq!(
        drawing_graph_saved_drawing_count_relations,
        BTreeMap::from([
            ("AbovePresentTree".to_owned(), 5),
            ("EqualToPresentTree".to_owned(), 325),
        ])
    );
    assert_eq!(
        drawing_graph_cluster_cursor_relations,
        BTreeMap::from([
            ("AbovePresentTree".to_owned(), 61),
            ("EqualToPresentTree".to_owned(), 461),
        ])
    );
    assert_eq!(
        drawing_graph_issues,
        BTreeMap::from([
            ("shape-cluster-drawing-mismatch", 3),
            ("shape-cluster-missing", 1),
            ("shape-in-cluster-zero", 1),
        ])
    );
    assert_eq!(drawing_graph_errors.values().sum::<usize>(), 15);
    assert_eq!(
        drawing_graph_errors
            .iter()
            .filter(|(error, _)| error.contains("partial MsoDrawingGroup"))
            .map(|(_, count)| count)
            .sum::<usize>(),
        1
    );
    assert_eq!(
        drawing_graph_errors
            .iter()
            .filter(|(error, _)| {
                error.contains("partial MsoDrawing")
                    && !error.contains("partial MsoDrawingGroup")
            })
            .map(|(_, count)| count)
            .sum::<usize>(),
        13
    );
    assert_eq!(
        drawing_graph_errors
            .iter()
            .filter(|(error, _)| error.contains("invalid framing"))
            .map(|(_, count)| count)
            .sum::<usize>(),
        1
    );
    assert_eq!(
        drawing_group_partial, 1,
        "partial drawing-group coverage changed"
    );
    assert_eq!(drawing_group_partial_complete_records, 4);
    assert_eq!(drawing_group_partial_incomplete_records, 3);
    assert_eq!(drawing_group_partial_unparsed_bytes, 0);
    assert_eq!(drawing_group_underreported_fbse, 1);
    assert!(drawing_group_partial_leaf_shapes.is_empty());
    assert!(drawing_group_partial_header_tail_lengths.is_empty());
    assert!(drawing_group_incomplete.is_empty());
    assert_eq!(drawing_group_incomplete_bytes, 0);
    assert_eq!(drawing_partial, 28, "partial drawing coverage changed");
    assert_eq!(drawing_partial_complete_records, 1_169);
    assert_eq!(drawing_partial_incomplete_records, 70);
    assert_eq!(drawing_partial_unparsed_bytes, 0);
    assert_eq!(
        drawing_partial_boundaries,
        BTreeMap::from([(Some(0x00ff), 1), (Some(0x023e), 1), (Some(0x0809), 26)])
    );
    assert_eq!(
        drawing_partial_leaf_shapes,
        BTreeMap::from([((0xf011, 0x0000, 0x7a00_0000, 0), 1)])
    );
    assert!(drawing_partial_header_tail_lengths.is_empty());
    assert_eq!(drawing_partial_unparsed_prefix_bytes, 0);
    assert!(drawing_incomplete.is_empty());
    assert_eq!(drawing_incomplete_bytes, 0);
    assert!(drawing_incomplete_boundaries.is_empty());
    assert_eq!(
        office_art_compatibility_containers,
        BTreeMap::from([(0x0000, 1), (0xf002, 1), (0xf0f4, 1)]),
        "OfficeArt nonstandard-container recVer coverage changed"
    );
    assert_eq!(office_art_incomplete_property_tables, 3);
    assert_eq!(office_art_incomplete_property_entries, 121);
    assert_eq!(office_art_incomplete_fixed_bytes, 4);
    assert_eq!(office_art_incomplete_low_words, 1);
    assert_eq!(office_art_incomplete_complex_bytes, 86);
    assert_eq!(office_art_incomplete_complex_unparsed_bytes, 0);
    assert_eq!(office_art_incomplete_array_fragments, 2);
    assert_eq!(office_art_incomplete_generic_fragment_bytes, 0);
    assert_eq!(
        office_art_utf16_properties
            .values()
            .map(|stats| stats.records)
            .sum::<usize>(),
        1_133,
        "OfficeArt UTF-16 property coverage changed"
    );
    assert_eq!(
        office_art_utf16_properties
            .values()
            .map(|stats| stats.bytes)
            .sum::<usize>(),
        24_668,
        "OfficeArt UTF-16 property byte coverage changed"
    );
    assert!(
        [0x00c5, 0x0105, 0x0187, 0x01c6, 0x0380, 0x0381]
            .into_iter()
            .all(|property_id| !office_art_complex_properties.contains_key(&property_id)),
        "a known UTF-16 property fell back to generic complex bytes"
    );
    assert_eq!(office_art_property_table_trailing_bytes, 0);
    assert_eq!(
        office_art_empty_complex_properties.values().sum::<usize>(),
        134,
        "OfficeArt zero-length complex-property coverage changed"
    );
    assert_eq!(office_art_metro_blobs.records, 169);
    assert_eq!(office_art_metro_blobs.bytes, 1_045_975);
    assert_eq!(office_art_metro_blob_entries, 776);
    assert!(!office_art_complex_properties.contains_key(&0x03a9));
    assert_eq!(office_art_hyperlinks.records, 47);
    assert_eq!(office_art_hyperlinks.bytes, 6_674);
    assert_eq!(office_art_hyperlink_nonparsed, 0);
    assert_eq!(office_art_hyperlink_trailing_bytes, 0);
    assert!(
        office_art_complex_properties.is_empty(),
        "complete FOPT values retained generic complex bytes: {office_art_complex_properties:?}"
    );
    assert_eq!(
        office_art_array_headers.values().sum::<usize>(),
        124,
        "OfficeArt IMsoArray coverage changed"
    );
    assert_eq!(
        office_art_array_headers
            .iter()
            .map(|((_, _, _, _, _, encoded_len), count)| encoded_len * count)
            .sum::<usize>(),
        24_978,
        "OfficeArt IMsoArray encoded-byte coverage changed"
    );
    assert!(
        [
            0x0145, 0x0146, 0x0151, 0x0152, 0x0155, 0x0156, 0x0157, 0x0197, 0x01cf, 0x0383,
        ]
        .into_iter()
        .all(|property_id| !office_art_complex_properties.contains_key(&property_id)),
        "a known IMsoArray property fell back to generic complex bytes"
    );
    assert_eq!(
        office_art_compatibility_property_tables, 1,
        "OfficeArt damaged property-table record-ID coverage changed"
    );
    assert_eq!(
        office_art_compatibility_anchors,
        BTreeMap::from([(0x0000, 1), (0xf00d, 1), (0xf0aa, 1)]),
        "OfficeArt damaged child-anchor record-ID coverage changed"
    );
    assert_eq!(
        office_art_compatibility_fbse, 1,
        "OfficeArt damaged FBSE record-ID coverage changed"
    );
    assert_eq!(
        office_art_empty_compatibility_atoms,
        BTreeMap::from([(0xf051, 1), (0xf08d, 1), (0xf10d, 1)]),
        "OfficeArt empty compatibility-atom coverage changed"
    );
    assert!(
        office_art_atom_stats.is_empty(),
        "OfficeArt records fell back to generic atoms: {office_art_atom_stats:?}\n{}",
        office_art_atom_locations.join("\n")
    );
    assert_eq!(
        office_art_softmaker_native_records, 34,
        "SoftMaker native-properties record coverage changed"
    );
    assert_eq!(
        office_art_softmaker_native_properties, 251,
        "SoftMaker native-property coverage changed"
    );
    assert_eq!(
        office_art_softmaker_native_payload_bytes, 13_486,
        "SoftMaker native-property payload coverage changed"
    );
    assert_eq!(
        office_art_softmaker_native_unparsed_bytes, 0,
        "SoftMaker native-property unparsed coverage changed"
    );
    assert_eq!(
        office_art_softmaker_selector6, 1,
        "SoftMaker selector-6 coverage changed"
    );
    assert_eq!(
        office_art_softmaker_native_shapes,
        BTreeMap::from([
            ((0, 0, 37), 33),
            ((1, 0, 80), 28),
            ((2, 0, 140), 29),
            ((3, 0, 60), 29),
            ((4, 0, 96), 29),
            ((6, 0, 81), 1),
            ((8, 0, 20), 34),
            ((9, 0, 4), 34),
            ((12, 0, 16), 34),
        ]),
        "SoftMaker native-property shape coverage changed"
    );
    assert_eq!(
        office_art_metafile_opaque_reasons.len(),
        1,
        "OfficeArt opaque-metafile reason coverage changed"
    );
    assert_eq!(office_art_pict_typed, 3, "typed PICT coverage changed");
    let decode_failed =
        &office_art_metafile_opaque_reasons[&OfficeArtMetafileOpaqueReason::DecodeFailed];
    assert_eq!((decode_failed.records, decode_failed.bytes), (1, 12_045));
    assert_eq!(decode_failed.lengths, BTreeSet::from([12_045]));
    let missing: Vec<_> = expected_invalid.difference(&observed_invalid).collect();
    assert!(
        missing.is_empty(),
        "XLS BIFF invalid expectations no longer fail: {missing:?}"
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} workbook streams failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!(
        "Obj FtPictFmla: {obj_picture_formulas} formulas, {obj_picture_embed_info} embed-info values, {obj_picture_positions} positions, {obj_picture_control_stream_sizes} Ctls sizes, {obj_picture_keys} keys, {obj_picture_compatibility_bytes} compatibility bytes"
    );
    eprintln!(
        "XLS root diagnostics: {root_truncated_diagnostics} truncated in {} files/{root_nonconforming_diagnostics} nonconforming in {} files ({root_workbook_structure_diagnostics} Workbook structure in {} files {root_workbook_diagnostic_shapes:?}; samples {root_workbook_diagnostic_samples:#?}; {root_bof_diagnostics} BOF in {} files {root_bof_diagnostic_shapes:?})/{root_invalid_stream_diagnostics} invalid stream in {} files/{root_noncanonical_cfb_diagnostics} noncanonical CFB in {} files",
        root_truncated_diagnostic_files.len(),
        root_nonconforming_diagnostic_files.len(),
        root_workbook_structure_diagnostic_files.len(),
        root_bof_diagnostic_files.len(),
        root_invalid_stream_diagnostic_files.len(),
        root_noncanonical_cfb_diagnostic_files.len()
    );
    eprintln!(
        "XLS drawing graphs: {drawing_graphs} complete ({drawing_graphs_strict} strict), {drawing_graph_drawings} drawings/{drawing_graph_shapes} shapes, {drawing_graph_absent} absent; csp {drawing_graph_shape_count_bases:?}; spidCur {drawing_graph_current_shape_id_relations:?}; spidMax {drawing_graph_maximum_shape_id_relations:?}; cspSaved {drawing_graph_saved_shape_count_relations:?}; cdgSaved {drawing_graph_saved_drawing_count_relations:?}; IDCL cursor {drawing_graph_cluster_cursor_relations:?}; issues {drawing_graph_issues:?}, errors {drawing_graph_errors:?}"
    );
    eprintln!(
        "checked {checked} XLS file roots ({root_parsed_files} parsed/{root_reopened_files} written and reopened): {biff8} BIFF8, {legacy} legacy; {} unknown BIFF record types remain; newly static formal records {newly_static_formal_records:?}; Formula has {formula_unparsed_rgce_bytes} unparsed rgce bytes, {formula_rgcb_bytes} unparsed rgcb bytes and {formula_missing_extra} explicit missing-extra compatibility states; SharedFormula has {shared_formula_records} records, {shared_formula_unparsed_rgce_bytes} unparsed rgce and {shared_formula_rgcb_bytes} rgcb bytes; Array has {array_records} records, {array_unparsed_rgce_bytes} unparsed rgce and {array_rgcb_bytes} rgcb bytes; SupBook has {sup_book_records} records, {sup_book_compatibility} compatibility values and {sup_book_trailing_bytes} retained trailing bytes; ExternName has {extern_name_records} records ({extern_name_formula_records} formula/{extern_name_cached_link_records} cached-link/{extern_name_compatibility_records} compatibility with {extern_name_compatibility_bytes} bytes); Hyperlink has {hyperlink_records} records, {hyperlink_compatibility_records} compatibility/{hyperlink_compatibility_bytes} bytes, {hyperlink_truncated_records} truncated ({hyperlink_truncated_url_records} typed URL moniker/{hyperlink_truncated_url_address_bytes} address bytes, {hyperlink_truncated_bytes} generic bytes) and {hyperlink_trailing_bytes} trailing bytes; DV has {data_validation_records} records, {data_validation_unparsed_rgce_bytes} unparsed rgce bytes and {data_validation_missing_extra} missing extras; CF has {conditional_formatting_records} records, {conditional_formatting_unparsed_rgce_bytes} unparsed rgce bytes and {conditional_formatting_missing_extra} missing extras; CFEx has {cfex_records} records ({cfex_cf12_records} CF12/{cfex_non_cf12_records} non-CF12, {cfex_formats} DXFN12 and {cfex_extension_unparsed_bytes} unparsed extension bytes); CF12 has {cf12_records} records/types {cf12_types:?}/{cf12_unparsed_formula_bytes} unparsed formula bytes; CrtMlFrt has {crt_ml_frt_records} records/{xml_tk_records} XmlTk values {xml_tk_kinds:?}; LinkedData has {linked_data_records} records, {linked_data_unparsed_rgce_bytes} unparsed rgce bytes and {linked_data_missing_extra} missing extras; FeatHdr has {feature_header_records} records ({feature_header_none} empty/{feature_header_enhanced_protection} protection/{feature_header_property_bag_store} property-bag/{feature_header_malformed} malformed with {feature_header_malformed_bytes} bytes); Feat has {feature_records} records ({feature_protection} protection/{feature_formula_errors} formula-error/{feature_smart_tags} smart-tag, {feature_security_descriptors} security descriptors); DConn has {dconn_records} records ({dconn_text} text/{dconn_web} web); Feature11 has {feature11_records} records/{feature11_fields} fields/{feature11_formats} DXFN12List/{feature11_auto_filters} AutoFilter envelopes; Name has {name_records} records ({name_continued_records} continued), {name_unparsed_rgce_bytes} unparsed rgce bytes, {name_rgcb_tail_bytes} rgcb tail bytes and {name_missing_extra} missing extras; Pls has {pls_records} records ({pls_continued} continued), {pls_windows_full} full/{pls_windows_legacy212} legacy-212/{pls_windows_core100} core-100/{pls_windows_truncated} spec-truncated DEVMODEW and {pls_platform_specific} values; MsoDrawingGroup has {drawing_group_records} records ({drawing_group_continued} continued), {drawing_group_complete} complete/{drawing_group_partial} partial trees ({drawing_group_partial_complete_records} complete + {drawing_group_partial_incomplete_records} incomplete records/{drawing_group_partial_unparsed_bytes} unparsed bytes) and {} whole-byte incomplete/{drawing_group_incomplete_bytes} bytes; MsoDrawing has {drawing_records} aggregates/{drawing_segments} segments/{drawing_host_records} host records, {drawing_complete} complete/{drawing_partial} partial trees ({drawing_partial_complete_records} complete + {drawing_partial_incomplete_records} incomplete records/{drawing_partial_unparsed_bytes} unparsed bytes) and {} whole-byte incomplete/{drawing_incomplete_bytes} bytes; host types: TxO {drawing_txo_typed} typed/{drawing_txo_raw} raw with {drawing_txo_control_contexts} ControlInfo/{drawing_txo_reserved_contexts} reserved/{drawing_txo_undetermined_contexts} undetermined contexts, {drawing_txo_formula_typed} typed/{drawing_txo_formula_opaque} opaque ObjFmla ({drawing_txo_formula_bytes} bytes) and {drawing_txo_trailing_bytes} trailing bytes, Note {drawing_note_typed} typed/{drawing_note_raw} raw, Obj {drawing_obj_typed} typed/{drawing_obj_raw} raw/{drawing_obj_raw_bytes} bytes; XFExt has {xf_ext_unparsed_bytes} unparsed bytes across property types {xf_ext_unknown_types:#06x?}; StyleExt has {style_ext_unparsed_bytes} unparsed XFProp bytes; DXF has {dxf_records} records/{dxf_unparsed_bytes} unparsed XFProp bytes; SST has {sst_records} records/{sst_strings} parsed strings, {sst_extension_bytes} ExtRst bytes ({sst_extension_unparsed_bytes} unparsed), {sst_trailing_bytes} compatibility tail bytes and {} truncated tables",
        unknown_types.len(),
        drawing_group_incomplete.len(),
        drawing_incomplete.len(),
        sst_truncated.len(),
    );
    if std::env::var_os("XLS_REPORT_UNKNOWN").is_some() {
        eprintln!("unknown BIFF record types: {unknown_types:#06x?}");
        let mut by_bytes: Vec<_> = unknown_stats.iter().collect();
        by_bytes.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.bytes));
        eprintln!("unknown BIFF records by retained payload bytes:");
        for (record_type, stats) in by_bytes {
            eprintln!(
                "  0x{record_type:04x}: {} records, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        eprintln!("unknown BIFF record samples:");
        for (record_type, samples) in &unknown_samples {
            eprintln!("  0x{record_type:04x}:");
            for sample in samples {
                eprintln!("    {sample}");
            }
        }
        eprintln!("FRTArchId$ distribution: {frt_arch_ids:?}");
        eprintln!("typed SXAddl sxc/sxd distribution: {sx_addl_types:#04x?}");
    }
    if std::env::var_os("XLS_REPORT_EXTERN_NAME").is_some() {
        eprintln!(
            "ExternName compatibility samples:\n{}",
            extern_name_compatibility_samples.join("\n")
        );
    }
    if std::env::var_os("XLS_REPORT_HYPERLINK").is_some() {
        eprintln!(
            "Hyperlink compatibility samples:\n{}",
            hyperlink_compatibility_samples.join("\n")
        );
        eprintln!(
            "Hyperlink truncated samples:\n{}",
            hyperlink_truncated_samples.join("\n")
        );
    }
    if std::env::var_os("XLS_REPORT_PLS").is_some() && !pls_samples.is_empty() {
        eprintln!("PLS samples:\n{}", pls_samples.join("\n"));
    }
    if std::env::var_os("XLS_REPORT_PLS").is_some() {
        eprintln!(
            "Pls detail: {pls_driver_extra_bytes} driver-private bytes/{pls_truncated_driver_extra} truncated driver-extra values, {pls_public_extension_bytes} public-extension bytes, {pls_truncated_public_bytes} truncated-public bytes, {pls_platform_specific_bytes} platform-specific bytes, {pls_trailing_bytes} trailing bytes; truncated sizes {pls_truncated_sizes:?}; platform shapes {pls_platform_shapes:?}"
        );
    }
    if std::env::var_os("XLS_REPORT_OFFICE_ART").is_some() {
        eprintln!("TxO ObjFmla shapes: {drawing_txo_formula_shapes:?}");
        eprintln!(
            "OfficeArt simple properties: {} ids/{} values; property-table trailing bytes: {office_art_property_table_trailing_bytes}",
            office_art_simple_properties.len(),
            office_art_simple_properties.values().sum::<usize>()
        );
        for (property_id, stats) in &office_art_utf16_properties {
            eprintln!(
                "OfficeArt UTF-16 property 0x{property_id:04x}: {} values, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        eprintln!(
            "OfficeArt empty complex properties: {office_art_empty_complex_properties:#06x?}"
        );
        eprintln!(
            "OfficeArt metroBlob: {} packages/{} entries/{} bytes, lengths {:?}",
            office_art_metro_blobs.records,
            office_art_metro_blob_entries,
            office_art_metro_blobs.bytes,
            office_art_metro_blobs.lengths
        );
        eprintln!(
            "OfficeArt IHlink: {} values/{} bytes/{} trailing bytes, lengths {:?}",
            office_art_hyperlinks.records,
            office_art_hyperlinks.bytes,
            office_art_hyperlink_trailing_bytes,
            office_art_hyperlinks.lengths
        );
        eprintln!("OfficeArt array headers: {office_art_array_headers:#06x?}");
        let mut complex_properties: Vec<_> = office_art_complex_properties.iter().collect();
        complex_properties.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.bytes));
        for (property_id, stats) in complex_properties {
            eprintln!(
                "OfficeArt complex property 0x{property_id:04x}: {} values, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        eprintln!("partial drawing-group leaf shapes: {drawing_group_partial_leaf_shapes:#06x?}");
        eprintln!(
            "partial drawing-group leaf samples:\n{}",
            drawing_group_partial_leaf_samples.join("\n")
        );
        eprintln!(
            "partial drawing-group node samples:\n{}",
            drawing_group_partial_node_samples.join("\n")
        );
        eprintln!(
            "partial drawing-group header-tail lengths: {drawing_group_partial_header_tail_lengths:?}"
        );
        eprintln!("partial drawing leaf shapes: {drawing_partial_leaf_shapes:#06x?}");
        eprintln!(
            "partial drawing leaf samples:\n{}",
            drawing_partial_leaf_samples.join("\n")
        );
        eprintln!("partial drawing header-tail lengths: {drawing_partial_header_tail_lengths:?}");
        eprintln!(
            "incomplete OfficeArt property tables:\n{}",
            office_art_incomplete_property_samples.join("\n")
        );
        for (subrecord_type, stats) in &drawing_obj_subrecord_raw {
            eprintln!(
                "Obj subrecord 0x{subrecord_type:04x}: {} records, {} raw bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        eprintln!(
            "OfficeArt embedded graphics: {office_art_emf_typed} typed EMF, {office_art_wmf_typed} typed WMF, {office_art_pict_typed} typed PICT, {office_art_dib_typed} typed/{office_art_dib_opaque} opaque DIB"
        );
        eprintln!(
            "SoftMaker native properties: {office_art_softmaker_native_records} records/{office_art_softmaker_native_properties} properties/{office_art_softmaker_native_payload_bytes} payload bytes/{office_art_softmaker_native_unparsed_bytes} unparsed, shapes {office_art_softmaker_native_shapes:?}"
        );
        if !drawing_group_incomplete.is_empty() {
            eprintln!(
                "incomplete MsoDrawingGroup values:\n{}",
                drawing_group_incomplete.join("\n")
            );
        }
        if !drawing_incomplete.is_empty() {
            eprintln!(
                "incomplete MsoDrawing boundaries {drawing_incomplete_boundaries:#06x?}:\n{}",
                drawing_incomplete.join("\n")
            );
        }
        let mut by_bytes: Vec<_> = office_art_atom_stats.iter().collect();
        by_bytes.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.bytes));
        for (record_type, stats) in by_bytes {
            eprintln!(
                "OfficeArt atom 0x{record_type:04x}: {} records, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        if !office_art_atom_locations.is_empty() {
            eprintln!(
                "OfficeArt atom locations:\n{}",
                office_art_atom_locations.join("\n")
            );
        }
        for (record_type, stats) in office_art_metafile_opaque {
            eprintln!(
                "OfficeArt opaque metafile 0x{record_type:04x}: {} records, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        for (reason, stats) in &office_art_metafile_opaque_reasons {
            eprintln!(
                "OfficeArt opaque metafile {reason:?}: {} records, {} decoded-or-encoded bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
    }
    if std::env::var_os("XLS_REPORT_FEATURE_HEADER").is_some() {
        eprintln!(
            "malformed FeatHdr samples:\n{}",
            feature_header_malformed_samples.join("\n")
        );
    }
    if std::env::var_os("XLS_REPORT_REMAINDERS").is_some() {
        eprintln!(
            "standalone Obj trailing samples:\n{}",
            standalone_obj_trailing_samples.join("\n")
        );
        eprintln!(
            "malformed RTD samples:\n{}",
            real_time_data_malformed_samples.join("\n")
        );
    }
    if std::env::var_os("XLS_REPORT_FORMULA_TAILS").is_some() && !formula_tail_locations.is_empty()
    {
        eprintln!("Formula rgcb tails:\n{}", formula_tail_locations.join("\n"));
    }
    if std::env::var_os("XLS_REPORT_NAME").is_some() && !name_tail_locations.is_empty() {
        eprintln!("Name formula tails:\n{}", name_tail_locations.join("\n"));
    }
    if std::env::var_os("XLS_REPORT_XF_EXT").is_some() {
        for (property_type, stats) in xf_ext_stats {
            eprintln!(
                "XFExt property 0x{property_type:04x}: {} records, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        if !xf_ext_unknown_locations.is_empty() {
            eprintln!("{}", xf_ext_unknown_locations.join("\n"));
        }
    }
    if std::env::var_os("XLS_REPORT_STYLE_EXT").is_some() {
        for (property_type, stats) in style_ext_stats {
            eprintln!(
                "StyleExt XFProp 0x{property_type:04x}: {} records, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
    }
    if std::env::var_os("XLS_REPORT_DXF").is_some() {
        for (property_type, stats) in dxf_stats {
            eprintln!(
                "DXF XFProp 0x{property_type:04x}: {} records, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
    }
    if std::env::var_os("XLS_REPORT_SST").is_some() && !sst_truncated.is_empty() {
        eprintln!("truncated SST tables:\n{}", sst_truncated.join("\n"));
        for (kind, stats) in sst_extension_unparsed_stats {
            eprintln!(
                "SST ExtRst {kind}: {} values, {} bytes, lengths {:?}",
                stats.records, stats.bytes, stats.lengths
            );
        }
        if !sst_phonetic_trailing_samples.is_empty() {
            eprintln!(
                "SST ExtRst trailing samples:\n{}",
                sst_phonetic_trailing_samples.join("\n")
            );
        }
        if !sst_extension_unparsed_samples.is_empty() {
            eprintln!(
                "SST ExtRst unparsed samples:\n{}",
                sst_extension_unparsed_samples.join("\n")
            );
        }
    }
}

fn incomplete_property_sample(table: &OfficeArtIncompletePropertyTable) -> String {
    format!(
        "entries {} partial-fixed {:02x?} complex {:?} trailing {:02x?}",
        table.entries.len(),
        table.incomplete_fixed_entry,
        table
            .complex_fragments
            .iter()
            .map(|fragment| (
                fragment.entry_index,
                fragment.property_id,
                fragment.declared_length,
                fragment.data.encoded_len(),
                fragment.is_complete,
            ))
            .collect::<Vec<_>>(),
        table.trailing_data
    )
}

struct OfficeArtPropertyAudit<'a> {
    simple: &'a mut BTreeMap<u16, usize>,
    complex: &'a mut BTreeMap<u16, UnknownStats>,
    utf16: &'a mut BTreeMap<u16, UnknownStats>,
    empty_complex: &'a mut BTreeMap<u16, usize>,
    metro_blobs: &'a mut UnknownStats,
    metro_blob_entries: &'a mut usize,
    hyperlinks: &'a mut UnknownStats,
    hyperlink_nonparsed: &'a mut usize,
    hyperlink_trailing_bytes: &'a mut usize,
    array_headers: &'a mut BTreeMap<(u16, u16, u16, u16, u32, usize), usize>,
    trailing_bytes: &'a mut usize,
}

fn audit_office_art_properties(record: &OfficeArtRecord, audit: OfficeArtPropertyAudit<'_>) {
    let OfficeArtPropertyAudit {
        simple,
        complex,
        utf16,
        empty_complex,
        metro_blobs,
        metro_blob_entries,
        hyperlinks,
        hyperlink_nonparsed,
        hyperlink_trailing_bytes,
        array_headers,
        trailing_bytes,
    } = audit;
    let OfficeArtRecordData::PropertyTable(table) = &record.data else {
        return;
    };
    *trailing_bytes += table.trailing.len();
    for property in &table.properties {
        match &property.value {
            OfficeArtPropertyValue::Simple(_) => {
                *simple.entry(property.property_id).or_default() += 1;
            }
            OfficeArtPropertyValue::Complex {
                declared_length,
                data,
            } => {
                let stats = complex.entry(property.property_id).or_default();
                stats.records += 1;
                stats.bytes += data.len();
                stats.lengths.insert(data.len());
                if matches!(
                    property.property_id,
                    0x0145
                        | 0x0146
                        | 0x0151
                        | 0x0152
                        | 0x0155
                        | 0x0156
                        | 0x0157
                        | 0x0197
                        | 0x01cf
                        | 0x0383
                ) && data.len() >= 6
                {
                    *array_headers
                        .entry((
                            property.property_id,
                            u16::from_le_bytes([data[0], data[1]]),
                            u16::from_le_bytes([data[2], data[3]]),
                            u16::from_le_bytes([data[4], data[5]]),
                            *declared_length,
                            data.len(),
                        ))
                        .or_default() += 1;
                }
            }
            OfficeArtPropertyValue::Utf16String { code_units, .. } => {
                let length = code_units.len() * 2;
                let stats = utf16.entry(property.property_id).or_default();
                stats.records += 1;
                stats.bytes += length;
                stats.lengths.insert(length);
            }
            OfficeArtPropertyValue::EmptyComplex { declared_length } => {
                assert_eq!(*declared_length, 0);
                *empty_complex.entry(property.property_id).or_default() += 1;
            }
            OfficeArtPropertyValue::EmptyArray { declared_length } => {
                *array_headers
                    .entry((property.property_id, 0, 0, 0, *declared_length, 0))
                    .or_default() += 1;
            }
            OfficeArtPropertyValue::Array {
                declared_length,
                value,
                ..
            } => {
                let element_size = if value.encoded_element_size == 0xfff0 {
                    4
                } else {
                    usize::from(value.encoded_element_size)
                };
                let encoded_len = 6 + usize::from(value.element_count) * element_size;
                *array_headers
                    .entry((
                        property.property_id,
                        value.element_count,
                        value.allocated_element_count,
                        value.encoded_element_size,
                        *declared_length,
                        encoded_len,
                    ))
                    .or_default() += 1;
            }
            OfficeArtPropertyValue::MetroBlob { value, .. } => {
                let length = value.package_bytes.len();
                metro_blobs.records += 1;
                metro_blobs.bytes += length;
                metro_blobs.lengths.insert(length);
                *metro_blob_entries += value.directory.entries.len();
            }
            OfficeArtPropertyValue::Hyperlink {
                declared_length,
                object,
                ..
            } => {
                let length = usize::try_from(*declared_length).expect("u32 fits usize");
                hyperlinks.records += 1;
                hyperlinks.bytes += length;
                hyperlinks.lengths.insert(length);
                match object {
                    HyperlinkObject::Parsed { trailing, .. } => {
                        *hyperlink_trailing_bytes += trailing.len();
                    }
                    HyperlinkObject::Truncated { .. }
                    | HyperlinkObject::TruncatedUrlMoniker { .. }
                    | HyperlinkObject::Compatibility(_) => {
                        *hyperlink_nonparsed += 1;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct UnknownStats {
    records: usize,
    bytes: usize,
    lengths: BTreeSet<usize>,
}

fn expected_invalid_files(corpus: &Path) -> BTreeSet<std::path::PathBuf> {
    let mut files = BTreeSet::new();
    for name in ["Apache-POI", "LibreOffice"] {
        let root = corpus.join(name);
        let manifest = read_manifest(&root.join("manifest.toml")).expect("read corpus manifest");
        for expectation in manifest.expectation {
            if expectation.test == "xls_biff_roundtrip"
                && expectation.mode == ExpectationMode::Invalid
            {
                files.insert(root.join(expectation.file));
            }
        }
    }
    files
}

fn collect(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("read corpus directory") {
        let path = entry.expect("read corpus entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "xls" | "xlt"))
        {
            files.push(path);
        }
    }
}

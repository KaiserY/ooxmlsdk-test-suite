use std::io::{Cursor, Read};

use olecfsdk::{
    cfb::CompoundFile,
    forms::{FmDisplayStyle, MorphDataControl, OleColorType},
};
use ooxmlsdk::{
    parts::{
        embedded_control_persistence_part::EmbeddedControlPersistencePart,
        presentation_document::PresentationDocument,
    },
    schemas::schemas_microsoft_com_office_2006_active_x::ActiveXControlData,
    sdk::{SdkPart, SdkType},
};

#[test]
fn word_activex_textboxes_expose_omitted_persisted_font_names() {
    let path = olecfsdk_corpus_test_support::corpus_root()
        .join("LibreOffice/sw/qa/extras/ooxmlexport/data/activex_textbox.docx");
    let package = std::fs::read(path).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();

    for part_name in ["word/activeX/activeX1.bin", "word/activeX/activeX2.bin"] {
        let mut binary = Vec::new();
        archive
            .by_name(part_name)
            .unwrap()
            .read_to_end(&mut binary)
            .unwrap();
        let compound = CompoundFile::from_bytes(&binary).unwrap();
        let contents = compound
            .stream("contents")
            .or_else(|| compound.stream("Contents"))
            .unwrap();
        let control = MorphDataControl::from_bytes(contents).unwrap();

        assert!(control.data_block.value.is_some(), "{part_name}");
        assert!(control.extra_data_block.value.is_some(), "{part_name}");
        assert!(
            control.text_props.data_block.font_name.is_none(),
            "{part_name}"
        );
        assert!(
            control.text_props.extra_data_block.font_name.is_none(),
            "{part_name}"
        );
    }
}

#[test]
fn word_activex_option_buttons_expose_control_type_and_omitted_font_names() {
    let path = olecfsdk_corpus_test_support::corpus_root()
        .join("LibreOffice/sw/qa/extras/ooxmlexport/data/activex_option_button_group.docx");
    let package = std::fs::read(path).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();

    for part_name in ["word/activeX/activeX1.bin", "word/activeX/activeX2.bin"] {
        let mut binary = Vec::new();
        archive
            .by_name(part_name)
            .unwrap()
            .read_to_end(&mut binary)
            .unwrap();
        let compound = CompoundFile::from_bytes(&binary).unwrap();
        let contents = compound
            .stream("contents")
            .or_else(|| compound.stream("Contents"))
            .unwrap();
        let control = MorphDataControl::from_bytes(contents).unwrap();

        assert_eq!(
            control.data_block.display_style.as_ref().unwrap().value,
            FmDisplayStyle::OptionButton,
            "{part_name}"
        );
        assert!(
            control.text_props.data_block.font_name.is_none(),
            "{part_name}"
        );
        assert!(
            control.text_props.extra_data_block.font_name.is_none(),
            "{part_name}"
        );
    }
}

#[test]
fn powerpoint_activex_storage_exposes_typed_live_control_properties() {
    let path = olecfsdk_corpus_test_support::corpus_root()
        .join("LibreOffice/sd/qa/unit/data/pptx/activex_togglebutton.pptx");
    let package = std::fs::read(path).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();
    let mut binary = Vec::new();
    archive
        .by_name("ppt/activeX/activeX2.bin")
        .unwrap()
        .read_to_end(&mut binary)
        .unwrap();

    let compound = CompoundFile::from_bytes(&binary).unwrap();
    let contents = compound.stream("Contents").unwrap();
    let control = MorphDataControl::from_bytes(contents).unwrap();

    let back_color = control.data_block.back_color.as_ref().unwrap().value;
    assert_eq!(back_color.color_type, OleColorType::Default);
    assert_eq!(back_color.rgb_components(), Some((255, 128, 255)));

    let caption_descriptor = control.data_block.caption.as_ref().unwrap().value;
    let caption = control.extra_data_block.caption.as_ref().unwrap();
    assert_eq!(
        caption.decode(caption_descriptor).unwrap(),
        "Custom Caption"
    );
    assert_eq!(
        control
            .text_props
            .extra_data_block
            .font_name
            .as_ref()
            .unwrap()
            .decode(
                control
                    .text_props
                    .data_block
                    .font_name
                    .as_ref()
                    .unwrap()
                    .value
            )
            .unwrap(),
        "Arial"
    );
}

#[test]
fn powerpoint_slide_relationship_resolves_activex_xml_and_binary_parts() {
    let path = olecfsdk_corpus_test_support::corpus_root()
        .join("LibreOffice/sd/qa/unit/data/pptx/activex_togglebutton.pptx");
    let package = PresentationDocument::new_from_file(path).unwrap();
    let presentation_part = package.presentation_part().unwrap();
    let slide_part = presentation_part.slide_parts(&package).next().unwrap();
    let mut controls = slide_part
        .related_parts_of_type::<_, EmbeddedControlPersistencePart>(&package)
        .map(|related| {
            let xml = related.part().data_to_vec(&package).unwrap();
            let control = ActiveXControlData::from_bytes(&xml).unwrap();
            let binary = related
                .part()
                .embedded_control_persistence_binary_data_parts(&package)
                .next()
                .unwrap()
                .data_to_vec(&package)
                .unwrap();
            (
                related.relationship_id().to_string(),
                control.active_x_control_class_id,
                binary.len(),
            )
        })
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| left.0.cmp(&right.0));

    assert_eq!(
        controls,
        vec![
            (
                "rId2".into(),
                "{8BD21D60-EC42-11CE-9E0D-00AA006002F3}".into(),
                2560,
            ),
            (
                "rId3".into(),
                "{8BD21D60-EC42-11CE-9E0D-00AA006002F3}".into(),
                2560,
            ),
            (
                "rId4".into(),
                "{8BD21D60-EC42-11CE-9E0D-00AA006002F3}".into(),
                2560,
            ),
        ]
    );
}

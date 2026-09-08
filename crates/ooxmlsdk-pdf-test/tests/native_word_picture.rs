use std::fs::File;

use ooxmlsdk::parts::wordprocessing_document::WordprocessingDocument;
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
    OpenSettings,
};
use ooxmlsdk_layout::LayoutOptions;
use ooxmlsdk_layout::common::DisplayItem;
use ooxmlsdk_pdf_test::libreoffice_fixture;

#[test]
fn native_word_picture_realizes_full_density_without_pdf_layer_payloads() {
    let source = libreoffice_fixture("sw/qa/extras/ooxmlexport/data/TextEffects_Groupshapes.docx");
    let document = WordprocessingDocument::new_with_settings(
        File::open(source).unwrap(),
        OpenSettings {
            markup_compatibility_process_settings: MarkupCompatibilityProcessSettings {
                process_mode: MarkupCompatibilityProcessMode::ProcessLoadedPartsOnly,
                target_file_format_version: FileFormatVersion::Microsoft365,
            },
            ..Default::default()
        },
    )
    .unwrap();
    let fixed_options = LayoutOptions {
        fixed_output_raster_dpi: Some(96),
        ..Default::default()
    };
    let fixed = ooxmlsdk_layout::docx::layout_document(&document, &fixed_options).unwrap();
    let native = ooxmlsdk_layout::docx::layout_document(
        &document,
        &LayoutOptions {
            native_picture_dpi: Some(600),
            ..fixed_options
        },
    )
    .unwrap();
    drop(document);

    fn pictures(items: &[DisplayItem<'_>], out: &mut Vec<(u32, u32, Vec<[u8; 4]>)>) {
        for item in items {
            match item {
                DisplayItem::Group(group) => pictures(&group.items, out),
                DisplayItem::Image(image) if image.bytes.starts_with(b"\x89PNG\r\n\x1a\n") => {
                    let png = image::load_from_memory(&image.bytes).unwrap();
                    let mut chunks = Vec::new();
                    let mut offset = 8;
                    while offset < image.bytes.len() {
                        let length =
                            u32::from_be_bytes(image.bytes[offset..offset + 4].try_into().unwrap())
                                as usize;
                        chunks.push(image.bytes[offset + 4..offset + 8].try_into().unwrap());
                        offset += length + 12;
                    }
                    assert_eq!(offset, image.bytes.len());
                    out.push((png.width(), png.height(), chunks));
                }
                _ => {}
            }
        }
    }
    let mut fixed_images = Vec::new();
    let mut native_images = Vec::new();
    for page in &fixed.pages {
        pictures(&page.items, &mut fixed_images);
    }
    for page in &native.pages {
        pictures(&page.items, &mut native_images);
    }
    assert_eq!(fixed_images.len(), 1);
    assert_eq!(native_images.len(), 1);
    assert!(native_images[0].0 > fixed_images[0].0 * 2);
    assert!(native_images[0].1 > fixed_images[0].1 * 2);
    assert!(fixed_images[0].2.contains(b"oxPr"));
    for hidden in [b"oxPr", b"oxEr", b"oxMr", b"oxSr"] {
        assert!(!native_images[0].2.contains(hidden));
    }
}

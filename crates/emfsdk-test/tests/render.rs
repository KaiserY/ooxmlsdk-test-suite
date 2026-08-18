use std::io::{Cursor, Read};

use emfsdk::render::{
    MetafileVectorFillRule, RenderOptions, decode_metafile_as_raster_with_options,
    extract_metafile_solid_rects, extract_metafile_vector_scene,
    extract_metafile_vector_scene_with_options,
};
use emfsdk_test::corpus_bytes;
use image::ImageFormat;

struct RenderCase {
    source: &'static str,
    path: &'static str,
    content_type: Option<&'static str>,
}

const RENDER_CASES: &[RenderCase] = &[
    RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testDrawImagePointsTypeBitmap",
        path: "LibreOffice/emfio/qa/cppunit/emf/data/TestDrawImagePointsTypeBitmap.emf",
        content_type: Some("image/x-emf"),
    },
    RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testEmfPlusDrawBeziers",
        path: "LibreOffice/emfio/qa/cppunit/emf/data/TestEmfPlusDrawBeziers.emf",
        content_type: Some("image/x-emf"),
    },
    RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testEmfPlusSetPageTransform",
        path: "LibreOffice/emfio/qa/cppunit/emf/data/TestEmfPlusSetPageTransform.emf",
        content_type: Some("image/x-emf"),
    },
    RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testRoundRect",
        path: "LibreOffice/emfio/qa/cppunit/emf/data/TestRoundRect.emf",
        content_type: Some("image/x-emf"),
    },
    RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testMoveToLineToWMF",
        path: "LibreOffice/emfio/qa/cppunit/wmf/data/TestLineTo.wmf",
        content_type: Some("image/x-wmf"),
    },
    RenderCase {
        source: "../core/emfio/qa/cppunit/wmf/wmfimporttest.cxx::testPatternBrushWmf",
        path: "LibreOffice/emfio/qa/cppunit/wmf/data/TestPatternBrush.wmf",
        content_type: Some("image/x-wmf"),
    },
];

#[test]
fn libreoffice_emf_plus_draw_line_preserves_visible_ink() {
    let case = RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testDrawLine",
        path: "LibreOffice/emfio/qa/cppunit/emf/data/TestDrawLine.emf",
        content_type: Some("image/x-emf"),
    };
    let image = decode_case_png(&case, fixed_render_options()).unwrap();
    let source_color = [0xC0u8, 0x10, 0x02];
    let source_alpha = 0xDBu16;
    let composited_color = source_color.map(|channel| {
        ((u16::from(channel) * source_alpha + 255 * (255 - source_alpha) + 127) / 255) as u8
    });
    let visible = image
        .pixels()
        .filter(|pixel| pixel.0 != [255, 255, 255])
        .collect::<Vec<_>>();

    // LibreOffice's source assertion records #c01002 with alpha 0xdb and a
    // 50-unit World pen. On an opaque white target each covered pixel is
    // source-over composited once. A thick-line implementation that stamps
    // overlapping segment pixels repeatedly would make the same source
    // incorrectly approach opaque #c01002.
    assert!(
        visible.len() > 1_000,
        "line is unexpectedly thin or clipped"
    );
    assert!(
        visible.len() < 10_000,
        "line unexpectedly covers most of the page"
    );
    assert!(
        visible.iter().all(|pixel| pixel.0 == composited_color),
        "EMF+ ARGB alpha was discarded or compounded; expected {composited_color:?}"
    );
}

fn fixed_render_options() -> RenderOptions {
    RenderOptions {
        target_width_px: Some(160),
        target_height_px: Some(120),
        max_pixels: Some(160 * 120),
        filter_high_frequency_pattern_brushes: false,
        monochrome_dib_palette_override: None,
        ..RenderOptions::default()
    }
}

#[test]
fn libreoffice_metafile_render_cases_emit_visible_pngs() {
    let options = fixed_render_options();
    let mut failures = Vec::new();

    for case in RENDER_CASES {
        if let Err(error) = assert_visible_png(case, options) {
            failures.push(format!(
                "{}\nsource: {}\nerror: {error}",
                case.path, case.source
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} render cases failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn open_xml_sdk_ole_paint_preview_renders_its_embedded_dib() {
    const PACKAGE: &str = "Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/\
TestDataStorage/O14ISOStrict/Excel/OLE-O12-XL-OLE.xlsx";
    const PREVIEW_PART: &str = "xl/media/image1.emf";
    let package = corpus_bytes(&emfsdk_test::corpus_dir(PACKAGE)).expect("OLE fixture package");
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("OLE fixture ZIP");
    let mut preview = Vec::new();
    archive
        .by_name(PREVIEW_PART)
        .expect("Paint.Picture preview part")
        .read_to_end(&mut preview)
        .expect("Paint.Picture preview bytes");
    let options = RenderOptions {
        // The XLSX/PDF path paints this VML frame at 184.32 x 162.72 pt and
        // therefore asks the fixed-output metafile renderer for 512 x 452
        // samples at 200 dpi. Exercise that host request, rather than only
        // the EMF's native 250 x 250 DIB dimensions.
        target_width_px: Some(512),
        target_height_px: Some(452),
        max_pixels: Some(300 * 300 * 64),
        transparent_background: true,
        suppress_text: true,
        suppress_solid_pattern_rects: true,
        suppress_bitmap_layers: true,
        ..RenderOptions::default()
    };

    let raster = decode_metafile_as_raster_with_options(&preview, Some("image/x-emf"), options)
        .expect("decode Paint.Picture preview")
        .expect("recognize Paint.Picture preview");
    let image = image::load_from_memory_with_format(&raster.data, ImageFormat::Png)
        .expect("Paint.Picture preview PNG")
        .to_rgba8();
    assert_eq!(image.dimensions(), (512, 452));
    let non_white = image
        .pixels()
        .filter(|pixel| {
            let alpha = u16::from(pixel[3]);
            pixel.0[..3].iter().any(|channel| {
                (u16::from(*channel) * alpha + 255 * (255 - alpha) + 127) / 255 < 255
            })
        })
        .count();
    let dark = image
        .pixels()
        .filter(|pixel| {
            let alpha = u16::from(pixel[3]);
            pixel.0[..3]
                .iter()
                .all(|channel| (u16::from(*channel) * alpha + 255 * (255 - alpha) + 127) / 255 < 64)
        })
        .count();

    // Open XML SDK's Office-authored fixture stores a 250x250 bottom-up DIB
    // behind an anisotropic window whose vertical extent is -250. The gray
    // field and handwritten black marks both disappear if that extent is
    // treated as an unsigned canvas magnitude.
    assert!(
        non_white > 180_000 && dark > 300,
        "embedded OLE preview is blank: non_white={non_white}, dark={dark}"
    );
}

#[test]
fn apache_poi_emf_with_header_description_renders_its_vector_artwork() {
    const PACKAGE: &str = "Apache-POI/test-data/spreadsheet/WithDrawing.xlsx";
    const IMAGE_PART: &str = "xl/media/image4.emf";
    let package = corpus_bytes(&emfsdk_test::corpus_dir(PACKAGE)).expect("drawing fixture package");
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("drawing fixture ZIP");
    let mut metafile = Vec::new();
    archive
        .by_name(IMAGE_PART)
        .expect("wrench EMF part")
        .read_to_end(&mut metafile)
        .expect("wrench EMF bytes");

    // This Adobe-authored stream uses the legal 88-byte base header followed
    // by its UTF-16 producer description. The first drawing record starts at
    // nSize=116, not at sizeof(ENHMETAHEADER)=108.
    assert_eq!(u32::from_le_bytes(metafile[4..8].try_into().unwrap()), 116);
    let options = RenderOptions {
        target_width_px: Some(470),
        target_height_px: Some(286),
        max_pixels: Some(470 * 286),
        ..RenderOptions::default()
    };
    let raster = decode_metafile_as_raster_with_options(&metafile, Some("image/x-emf"), options)
        .expect("decode described EMF")
        .expect("recognize described EMF");
    let image = image::load_from_memory_with_format(&raster.data, ImageFormat::Png)
        .expect("wrench preview PNG")
        .to_rgb8();
    assert_eq!(image.dimensions(), (470, 286));
    let non_white = image
        .pixels()
        .filter(|pixel| pixel.0 != [255, 255, 255])
        .count();
    assert!(
        non_white > 20_000,
        "described EMF lost its vector records: non_white={non_white}"
    );
}

#[test]
fn apache_poi_ole_pdf_icon_replays_emr_alpha_blend() {
    const PACKAGE: &str = "Apache-POI/test-data/spreadsheet/58325_lt.xlsx";
    const PREVIEW_PART: &str = "xl/media/image1.emf";
    let package = corpus_bytes(&emfsdk_test::corpus_dir(PACKAGE)).expect("OLE fixture package");
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("OLE fixture ZIP");
    let mut preview = Vec::new();
    archive
        .by_name(PREVIEW_PART)
        .expect("Adobe PDF icon preview part")
        .read_to_end(&mut preview)
        .expect("Adobe PDF icon preview bytes");

    // This stream combines one EMR_ALPHABLEND bitmap with three text records.
    // It must stay on complete raster replay: accepting a partial solid-fill
    // scene would silently retain the label while dropping the Adobe icon.
    assert!(
        extract_metafile_vector_scene(&preview, Some("image/x-emf"))
            .expect("inspect Adobe PDF icon preview")
            .is_none()
    );
    let options = RenderOptions {
        // Match the 72.36 x 55.2 pt Excel fixed-output host at 200 DPI.
        target_width_px: Some(201),
        target_height_px: Some(153),
        max_pixels: Some(201 * 153),
        ..RenderOptions::default()
    };
    let raster = decode_metafile_as_raster_with_options(&preview, Some("image/x-emf"), options)
        .expect("decode Adobe PDF icon preview")
        .expect("recognize Adobe PDF icon preview");
    let image = image::load_from_memory_with_format(&raster.data, ImageFormat::Png)
        .expect("Adobe PDF icon preview PNG")
        .to_rgb8();
    let adobe_red = image
        .pixels()
        .filter(|pixel| {
            let [red, green, blue] = pixel.0;
            red > 160 && red.saturating_sub(green) > 40 && red.saturating_sub(blue) > 40
        })
        .count();

    // [MS-EMF] 2.3.1.1 and Win32 BLENDFUNCTION require 32-bpp AC_SRC_ALPHA
    // samples to be premultiplied. LibreOffice emfreader.cxx reconstructs a
    // DIBV5 alpha plane; Wine's primitives.c implements the same SourceOver
    // equation. The OneCodeTeam CSWinFormLayeredWindow sample independently
    // documents SourceConstantAlpha=255 for per-pixel alpha.
    assert!(
        adobe_red > 100,
        "EMR_ALPHABLEND lost the red Adobe artwork: adobe_red={adobe_red}"
    );
}

#[test]
fn apache_poi_solid_fill_metafiles_form_complete_vector_scenes() {
    const PACKAGE: &str = "Apache-POI/test-data/spreadsheet/WithDrawing.xlsx";
    const PARTS: &[(&str, &str, usize)] = &[
        ("xl/media/image2.emf", "image/x-emf", 85),
        ("xl/media/image4.emf", "image/x-emf", 35),
        ("xl/media/image5.wmf", "image/x-wmf", 90),
    ];
    let package = corpus_bytes(&emfsdk_test::corpus_dir(PACKAGE)).expect("drawing fixture package");
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("drawing fixture ZIP");

    for &(part, content_type, expected_fills) in PARTS {
        let mut metafile = Vec::new();
        archive
            .by_name(part)
            .unwrap_or_else(|_| panic!("missing metafile part {part}"))
            .read_to_end(&mut metafile)
            .unwrap_or_else(|_| panic!("cannot read metafile part {part}"));
        let scene = extract_metafile_vector_scene(&metafile, Some(content_type))
            .unwrap_or_else(|error| panic!("cannot inspect {part}: {error}"))
            .unwrap_or_else(|| panic!("{part} did not form a complete solid-fill scene"));

        assert_eq!(
            scene.fills.len(),
            expected_fills,
            "unexpected fills in {part}"
        );
        assert!(
            scene.fills.iter().all(|fill| {
                fill.fill_rule == MetafileVectorFillRule::Alternate
                    && fill.subpaths.len() == 1
                    && fill.subpaths[0].len() >= 3
            }),
            "{part} lost a closed ALTERNATE polygon"
        );
    }
}

#[test]
fn libreoffice_excel_ole_grid_has_no_residual_metafile_layer_after_decomposition() {
    const PACKAGE: &str = "LibreOffice/sw/qa/extras/ooxmlexport/data/fdo77759.docx";
    const PREVIEW_PART: &str = "word/media/image1.emf";
    let package = corpus_bytes(&emfsdk_test::corpus_dir(PACKAGE)).expect("OLE fixture package");
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("OLE fixture ZIP");
    let mut preview = Vec::new();
    archive
        .by_name(PREVIEW_PART)
        .expect("Excel OLE preview part")
        .read_to_end(&mut preview)
        .expect("Excel OLE preview bytes");

    // This Office-authored EMF emits each grid edge twice: first as a
    // one-device-pixel cosmetic LineTo, then as a same-color source-less
    // PATCOPY rectangle with exactly the same extent. [MS-EMF] defines the
    // latter as an opaque brush copy, so the later operation completely
    // replaces the line. The 19 rectangles and semantic text are lifted by
    // the DOCX/PDF host; accepting any other residual painter would duplicate
    // the grid through a raster backdrop.
    let rects = extract_metafile_solid_rects(&preview, Some("image/x-emf"));
    assert_eq!(rects.len(), 19);
    assert!(rects.iter().all(|rect| rect.color == [218, 220, 221]));
    assert!(
        extract_metafile_vector_scene(&preview, Some("image/x-emf"))
            .expect("inspect complete Excel OLE preview")
            .is_none(),
        "ordinary playback must retain its visible lines, text, and PATCOPY records"
    );

    let scene = extract_metafile_vector_scene_with_options(
        &preview,
        Some("image/x-emf"),
        RenderOptions {
            suppress_text: true,
            suppress_solid_pattern_rects: true,
            suppress_bitmap_layers: true,
            ..RenderOptions::default()
        },
    )
    .expect("inspect decomposed Excel OLE preview")
    .expect("all remaining records should be provably nonpainting");
    assert_eq!(scene.fills.len(), 0, "unexpected residual vector fill");
}

fn assert_visible_png(case: &RenderCase, options: RenderOptions) -> Result<(), String> {
    let image = decode_case_png(case, options)?;
    let non_white = image
        .pixels()
        .filter(|pixel| pixel.0 != [255, 255, 255])
        .count();

    if non_white == 0 {
        return Err(format!(
            "rendered image is effectively blank: {non_white} non-white pixels"
        ));
    }

    Ok(())
}

fn decode_case_png(case: &RenderCase, options: RenderOptions) -> Result<image::RgbImage, String> {
    let bytes = corpus_bytes(&emfsdk_test::corpus_dir(case.path))?;
    let raster = decode_metafile_as_raster_with_options(&bytes, case.content_type, options)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "metafile was not recognized".to_string())?;
    if raster.content_type != "image/png" {
        return Err(format!("expected PNG raster, got {}", raster.content_type));
    }

    let image = image::load_from_memory_with_format(&raster.data, ImageFormat::Png)
        .map_err(|error| error.to_string())?
        .to_rgb8();
    if image.dimensions() != (160, 120) {
        return Err(format!(
            "unexpected image dimensions {:?}",
            image.dimensions()
        ));
    }

    Ok(image)
}

use emfsdk::render::{RenderOptions, decode_metafile_as_raster_with_options};
use emfsdk_test::corpus_bytes;
use image::ImageFormat;

struct RenderCase {
    source: &'static str,
    path: &'static str,
    content_type: Option<&'static str>,
}

const RENDER_CASES: &[RenderCase] = &[
    RenderCase {
        source: "../core/emfio/qa/cppunit/emf/EmfImportTest.cxx::testDrawLine",
        path: "LibreOffice/emfio/qa/cppunit/emf/data/TestDrawLine.emf",
        content_type: Some("image/x-emf"),
    },
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
fn libreoffice_metafile_render_cases_emit_visible_pngs() {
    let options = RenderOptions {
        target_width_px: Some(160),
        target_height_px: Some(120),
        max_pixels: Some(160 * 120),
    };
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

fn assert_visible_png(case: &RenderCase, options: RenderOptions) -> Result<(), String> {
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

    let mut non_white = 0usize;
    for pixel in image.pixels() {
        let [r, g, b] = pixel.0;
        if [r, g, b] != [255, 255, 255] {
            non_white += 1;
        }
    }

    if non_white == 0 {
        return Err(format!(
            "rendered image is effectively blank: {non_white} non-white pixels"
        ));
    }

    Ok(())
}

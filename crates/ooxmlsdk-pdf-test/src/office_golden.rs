use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use icu_locale_core::Locale;
use icu_time::zone::WindowsParser;
use icu_time::zone::iana::IanaParserExtended;
use image::ImageFormat;
use jiff::{SignedDuration, Timestamp};
use ooxmlsdk_pdf::{FieldUpdateDateTime, PdfConversionDiagnostics, PdfFontAudit};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use unicode_bidi::{BidiClass, bidi_class};
use unicode_bidi_mirroring::get_mirrored;

use crate::pdf_extract::{
    ImageSummary, RenderedPagePairError, first_pdf_page_text_mismatch, pdf_font_structure,
    pdf_page_dimensions, pdftotext_page, visit_rendered_page_pairs,
};
use crate::{
    CalibrationError, PdfBounds, PdfSummary, PixelRect, RenderedPageImage, Result, parse_pdf_rect,
    workspace_root,
};

const RASTER_WIDTH: i32 = 1_333;
// Compare the first glyph's baseline origin and horizontal ink bounds rather
// than loose vertical edges: those edges include font-descriptor differences
// between Office's simple TrueType subsets and our CID subsets. Width stays at
// a tighter relative bound below.
// Preserve the original vector allowance for small pages, and express the
// wider-page allowance in samples at the same fixed-width raster used by the
// visible-output contract. Seven whole samples include one endpoint-
// quantization sample for the independently produced PDFs, while keeping A4,
// Letter, and widescreen pages comparable without making narrow custom pages
// stricter.
const TEXT_EDGE_TOLERANCE_MIN_PT: f32 = 2.5;
const TEXT_EDGE_TOLERANCE_RASTER_PIXELS: f32 = 7.0;
const EXACT_IMAGE_PLACEMENT_RASTER_PIXELS: f32 = 1.0;
const TEXT_WIDTH_TOLERANCE_RATIO: f32 = 0.01;
const EARLY_TEXT_PREFLIGHT_MIN_PAGES: usize = 8;
const EARLY_TEXT_PREFLIGHT_MIN_PDF_BYTES: usize = 4 * 1024 * 1024;
const TEXT_MASK_PADDING_PT: f32 = 1.0;
// ECMA-376 theme tint/shade uses an HSL round trip before the result is
// represented as 8-bit RGB. Accept one final quantization step between PDF
// producers; alpha remains exact.
const TEXT_COLOR_CHANNEL_TOLERANCE: u8 = 1;
// ISO paper sizes convert to fractional PDF points, while Office serializes
// the MediaBox through its fixed-output device grid. Keep this well below one
// rendered pixel while accepting the observed sub-tenth-point quantization.
const MEDIA_BOX_TOLERANCE_PT: f32 = 0.1;

#[derive(Clone, Copy, Debug)]
pub struct OfficeGoldenCase<'a> {
    pub id: &'a str,
    pub corpus: &'a str,
    pub source: &'a str,
    pub source_sha256: &'a str,
    pub golden_sha256: &'a str,
    pub environment_id: &'a str,
    pub ui_language: &'a str,
    pub format_locale: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct VisualTolerance {
    pub significant_channel_delta: u8,
    pub max_significant_pixel_fraction: f64,
    pub max_mean_absolute_channel_delta: f64,
}

impl VisualTolerance {
    /// Shared starting contract for Office golden comparisons.
    ///
    /// The significant-pixel threshold absorbs rasterizer antialiasing noise;
    /// the fraction and mean limits remain independent so a sparse large error
    /// and a broad low-contrast error are both observable.
    pub const OFFICE_FIXED_OUTPUT: Self = Self {
        significant_channel_delta: 16,
        max_significant_pixel_fraction: 0.01,
        max_mean_absolute_channel_delta: 1.5,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualDiffMetrics {
    pub page_pixels: usize,
    pub total_pixels: usize,
    pub masked_pixels: usize,
    pub significant_pixels: usize,
    pub significant_pixel_fraction: f64,
    pub mean_absolute_channel_delta: f64,
    pub max_channel_delta: u8,
    pub localized_graphics_regions: usize,
    pub max_localized_graphics_significant_pixel_fraction: f64,
    pub max_localized_graphics_mean_absolute_channel_delta: f64,
}

#[derive(Clone, Debug)]
pub struct OfficeGoldenReport {
    pub case_id: String,
    pub candidate: PdfSummary,
    pub golden: PdfSummary,
    pub page_diffs: Vec<VisualDiffMetrics>,
    pub artifact_dir: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OfficeGoldenComparisonLayer {
    Identity,
    Conversion,
    PdfExtraction,
    PageGeometry,
    Text,
    Font,
    VisibleOutput,
    ComparisonArtifact,
}

impl OfficeGoldenComparisonLayer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Conversion => "conversion",
            Self::PdfExtraction => "pdf-extraction",
            Self::PageGeometry => "page-geometry",
            Self::Text => "text",
            Self::Font => "font",
            Self::VisibleOutput => "visible-output",
            Self::ComparisonArtifact => "comparison-artifact",
        }
    }
}

impl std::str::FromStr for OfficeGoldenComparisonLayer {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "identity" => Ok(Self::Identity),
            "conversion" => Ok(Self::Conversion),
            "pdf-extraction" => Ok(Self::PdfExtraction),
            "page-geometry" => Ok(Self::PageGeometry),
            "text" => Ok(Self::Text),
            "font" => Ok(Self::Font),
            "visible-output" => Ok(Self::VisibleOutput),
            "comparison-artifact" => Ok(Self::ComparisonArtifact),
            _ => Err(format!("unknown Office golden comparison layer {value:?}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfficeGoldenFailure {
    pub layer: OfficeGoldenComparisonLayer,
    pub diagnostic_kind: OfficeGoldenDiagnosticKind,
    pub page_index: Option<usize>,
    pub line_index: Option<usize>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum OfficeGoldenDiagnosticKind {
    #[default]
    Unclassified,
    Identity,
    CandidateConversion,
    PdfExtraction,
    PageCount,
    PageGeometry,
    TextContent,
    TextStyle,
    TextReconstruction,
    TextLineCount,
    TextLineContent,
    TextHorizontalBounds,
    TextBaseline,
    FontIntegrity,
    FontAssignment,
    VisibleOutput,
    ComparisonArtifact,
}

impl OfficeGoldenDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unclassified => "unclassified",
            Self::Identity => "identity",
            Self::CandidateConversion => "candidate-conversion",
            Self::PdfExtraction => "pdf-extraction",
            Self::PageCount => "page-count",
            Self::PageGeometry => "page-geometry",
            Self::TextContent => "text-content",
            Self::TextStyle => "text-style",
            Self::TextReconstruction => "text-reconstruction",
            Self::TextLineCount => "text-line-count",
            Self::TextLineContent => "text-line-content",
            Self::TextHorizontalBounds => "text-horizontal-bounds",
            Self::TextBaseline => "text-baseline",
            Self::FontIntegrity => "font-integrity",
            Self::FontAssignment => "font-assignment",
            Self::VisibleOutput => "visible-output",
            Self::ComparisonArtifact => "comparison-artifact",
        }
    }
}

impl std::str::FromStr for OfficeGoldenDiagnosticKind {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "unclassified" => Ok(Self::Unclassified),
            "identity" => Ok(Self::Identity),
            "candidate-conversion" => Ok(Self::CandidateConversion),
            "pdf-extraction" => Ok(Self::PdfExtraction),
            "page-count" => Ok(Self::PageCount),
            "page-geometry" => Ok(Self::PageGeometry),
            "text-content" => Ok(Self::TextContent),
            "text-style" => Ok(Self::TextStyle),
            "text-reconstruction" => Ok(Self::TextReconstruction),
            "text-line-count" => Ok(Self::TextLineCount),
            "text-line-content" => Ok(Self::TextLineContent),
            "text-horizontal-bounds" => Ok(Self::TextHorizontalBounds),
            "text-baseline" => Ok(Self::TextBaseline),
            "font-integrity" => Ok(Self::FontIntegrity),
            "font-assignment" => Ok(Self::FontAssignment),
            "visible-output" => Ok(Self::VisibleOutput),
            "comparison-artifact" => Ok(Self::ComparisonArtifact),
            _ => Err(format!("unknown Office golden diagnostic kind {value:?}")),
        }
    }
}

impl OfficeGoldenFailure {
    fn new(layer: OfficeGoldenComparisonLayer, error: impl fmt::Display) -> Self {
        let diagnostic_kind = match layer {
            OfficeGoldenComparisonLayer::Identity => OfficeGoldenDiagnosticKind::Identity,
            OfficeGoldenComparisonLayer::Conversion => {
                OfficeGoldenDiagnosticKind::CandidateConversion
            }
            OfficeGoldenComparisonLayer::PdfExtraction => OfficeGoldenDiagnosticKind::PdfExtraction,
            OfficeGoldenComparisonLayer::PageGeometry => OfficeGoldenDiagnosticKind::PageGeometry,
            OfficeGoldenComparisonLayer::Text => OfficeGoldenDiagnosticKind::Unclassified,
            OfficeGoldenComparisonLayer::Font => OfficeGoldenDiagnosticKind::FontIntegrity,
            OfficeGoldenComparisonLayer::VisibleOutput => OfficeGoldenDiagnosticKind::VisibleOutput,
            OfficeGoldenComparisonLayer::ComparisonArtifact => {
                OfficeGoldenDiagnosticKind::ComparisonArtifact
            }
        };
        Self {
            layer,
            diagnostic_kind,
            page_index: None,
            line_index: None,
            message: error.to_string(),
        }
    }

    fn diagnostic(
        layer: OfficeGoldenComparisonLayer,
        diagnostic_kind: OfficeGoldenDiagnosticKind,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            layer,
            diagnostic_kind,
            page_index: None,
            line_index: None,
            message: error.to_string(),
        }
    }

    fn at(mut self, page_index: usize, line_index: Option<usize>) -> Self {
        self.page_index = Some(page_index);
        self.line_index = line_index;
        self
    }

    fn with_artifacts(mut self, artifact_dir: &Path) -> Self {
        self.message = format!("{}; artifacts={}", self.message, artifact_dir.display());
        self
    }
}

impl fmt::Display for OfficeGoldenFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.layer.as_str(), self.message)
    }
}

impl std::error::Error for OfficeGoldenFailure {}

type DetailedResult<T> = std::result::Result<T, OfficeGoldenFailure>;

pub fn compare_office_golden(
    case: OfficeGoldenCase<'_>,
    tolerance: VisualTolerance,
) -> Result<OfficeGoldenReport> {
    compare_office_golden_detailed(case, tolerance)
        .map_err(|error| CalibrationError::OfficeGolden(error.to_string()))
}

pub fn compare_office_golden_detailed(
    case: OfficeGoldenCase<'_>,
    tolerance: VisualTolerance,
) -> DetailedResult<OfficeGoldenReport> {
    compare_office_golden_detailed_with_artifacts(case, tolerance, true)
}

pub(crate) fn compare_office_golden_detailed_with_artifacts(
    case: OfficeGoldenCase<'_>,
    tolerance: VisualTolerance,
    write_failure_artifacts: bool,
) -> DetailedResult<OfficeGoldenReport> {
    compare_office_golden_detailed_inner(case, tolerance, write_failure_artifacts, None, true)
}

pub(crate) fn compare_office_golden_detailed_with_prevalidated_options(
    case: OfficeGoldenCase<'_>,
    options: ooxmlsdk_pdf::PdfOptions,
    tolerance: VisualTolerance,
    write_failure_artifacts: bool,
) -> DetailedResult<OfficeGoldenReport> {
    compare_office_golden_detailed_inner(
        case,
        tolerance,
        write_failure_artifacts,
        Some(options),
        false,
    )
}

fn compare_office_golden_detailed_inner(
    case: OfficeGoldenCase<'_>,
    tolerance: VisualTolerance,
    write_failure_artifacts: bool,
    configured_options: Option<ooxmlsdk_pdf::PdfOptions>,
    verify_conversion_manifest: bool,
) -> DetailedResult<OfficeGoldenReport> {
    let mut stage_trace = OfficeGoldenStageTrace::new(case);
    let root = workspace_root();
    let source_path = root.join("corpus").join(case.corpus).join(case.source);
    let golden_path = root
        .join("corpus_pdf_conv")
        .join(case.corpus)
        .join(format!("{}.pdf", case.source));
    let manifest_record = verify_conversion_manifest
        .then(|| verify_manifest_record(&root, case))
        .transpose()
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;

    let source_bytes = fs::read(&source_path)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    verify_sha256("source", &source_path, &source_bytes, case.source_sha256)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    let golden_pdf = fs::read(&golden_path)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    verify_sha256("golden", &golden_path, &golden_pdf, case.golden_sha256)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error))?;
    stage_trace.mark("identity");
    let options = if let Some(options) = configured_options {
        options
    } else {
        let jpeg_quality = source_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("docx")
                    || extension.eq_ignore_ascii_case("docm")
                    || extension.eq_ignore_ascii_case("dotx")
                    || extension.eq_ignore_ascii_case("dotm")
            })
            .then_some(75);
        let field_update_datetime = source_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("docx")
                    || extension.eq_ignore_ascii_case("docm")
                    || extension.eq_ignore_ascii_case("dotx")
                    || extension.eq_ignore_ascii_case("dotm")
                    || extension.eq_ignore_ascii_case("pptx")
                    || extension.eq_ignore_ascii_case("pptm")
                    || extension.eq_ignore_ascii_case("potx")
                    || extension.eq_ignore_ascii_case("potm")
                    || extension.eq_ignore_ascii_case("ppsx")
                    || extension.eq_ignore_ascii_case("ppsm")
            })
            .then(|| {
                reference_field_update_datetime(
                    &root,
                    manifest_record
                        .as_ref()
                        .expect("default options require a verified conversion manifest"),
                )
            })
            .transpose()
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error)
            })?;
        let field_update_time_zone = field_update_datetime
            .is_some()
            .then(|| {
                reference_field_update_time_zone(
                    &root,
                    manifest_record
                        .as_ref()
                        .expect("default options require a verified conversion manifest"),
                )
            })
            .transpose()
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Identity, error)
            })?;
        ooxmlsdk_pdf::PdfOptions {
            // Word's print-optimized ExportAsFixedFormat path recompresses JPEG
            // image XObjects at quality 75. The four independently converted
            // sdtContent.docx records preserve the source dimensions and 220-DPI
            // density while changing the embedded stream from quality 95 to 75.
            // Do not impose that Word-specific policy on PowerPoint: the
            // gridSpacing-cy-0 Office PDF preserves all decoded source-JPEG
            // samples exactly in a lossless image XObject.
            jpeg_quality,
            source_file_name: source_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string),
            ui_language: Some(case.ui_language.to_string()),
            format_locale: Some(case.format_locale.to_string()),
            field_update_datetime,
            field_update_time_zone,
            ..Default::default()
        }
    };
    let diagnostic_options = options.clone();
    let candidate_output = crate::render::render_fixture_pdf_with_font_audit(&source_path, options)
        .map_err(|error| {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Conversion, error)
        })?;
    let candidate_pdf = candidate_output.pdf;
    let candidate_font_audit = candidate_output.audit;
    let mut candidate_diagnostics = CandidateDiagnosticsState::Uncollected;
    stage_trace.mark("candidate-render");

    if let Err(error) = validate_candidate_font_contract(&candidate_pdf, &candidate_font_audit) {
        let error = OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Font, error);
        if !write_failure_artifacts {
            return Err(error);
        }
        collect_candidate_diagnostics(
            &source_path,
            &diagnostic_options,
            &mut candidate_diagnostics,
        );
        let artifact_dir = write_candidate_artifact(
            case.id,
            &candidate_pdf,
            &golden_pdf,
            &candidate_font_audit,
            &candidate_diagnostics,
            &[0],
        )
        .map_err(|artifact_error| {
            OfficeGoldenFailure::new(
                OfficeGoldenComparisonLayer::ComparisonArtifact,
                artifact_error,
            )
        })?;
        return Err(error.with_artifacts(&artifact_dir));
    }
    stage_trace.mark("font-contract");

    // Page-count mismatches are common while growing XLSX coverage. Detect
    // them with lopdf's page tree before PDFium walks every text character and
    // page object in multi-megabyte, thousand-row reference PDFs.
    let candidate_page_dimensions = pdf_page_dimensions(&candidate_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    let golden_page_dimensions = pdf_page_dimensions(&golden_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    stage_trace.mark("page-dimensions");
    let candidate_page_count = candidate_page_dimensions.len();
    let golden_page_count = golden_page_dimensions.len();
    if candidate_page_count != golden_page_count {
        let error = OfficeGoldenFailure::diagnostic(
            OfficeGoldenComparisonLayer::PageGeometry,
            OfficeGoldenDiagnosticKind::PageCount,
            CalibrationError::OfficeGolden(format!(
                "case {} page count mismatch: candidate={}, golden={}",
                case.id, candidate_page_count, golden_page_count
            )),
        );
        if !write_failure_artifacts {
            return Err(error);
        }
        collect_candidate_diagnostics(
            &source_path,
            &diagnostic_options,
            &mut candidate_diagnostics,
        );
        let trace_page = candidate_page_count
            .saturating_sub(1)
            .min(golden_page_count);
        let artifact_dir = write_candidate_artifact(
            case.id,
            &candidate_pdf,
            &golden_pdf,
            &candidate_font_audit,
            &candidate_diagnostics,
            &[trace_page],
        )
        .map_err(|artifact_error| {
            OfficeGoldenFailure::new(
                OfficeGoldenComparisonLayer::ComparisonArtifact,
                artifact_error,
            )
        })?;
        return Err(error.with_artifacts(&artifact_dir));
    }
    if let Some((page_index, (candidate, golden))) = candidate_page_dimensions
        .iter()
        .zip(&golden_page_dimensions)
        .enumerate()
        .find(
            |(_, ((candidate_width, candidate_height), (golden_width, golden_height)))| {
                (candidate_width - golden_width).abs() > MEDIA_BOX_TOLERANCE_PT
                    || (candidate_height - golden_height).abs() > MEDIA_BOX_TOLERANCE_PT
            },
        )
    {
        let error = OfficeGoldenFailure::diagnostic(
            OfficeGoldenComparisonLayer::PageGeometry,
            OfficeGoldenDiagnosticKind::PageGeometry,
            CalibrationError::OfficeGolden(format!(
                "case {} page {page_index} media box mismatch: candidate={candidate:?}, golden={golden:?}",
                case.id
            )),
        )
        .at(page_index, None);
        if !write_failure_artifacts {
            return Err(error);
        }
        collect_candidate_diagnostics(
            &source_path,
            &diagnostic_options,
            &mut candidate_diagnostics,
        );
        let artifact_dir = write_candidate_artifact(
            case.id,
            &candidate_pdf,
            &golden_pdf,
            &candidate_font_audit,
            &candidate_diagnostics,
            &[page_index],
        )
        .map_err(|artifact_error| {
            OfficeGoldenFailure::new(
                OfficeGoldenComparisonLayer::ComparisonArtifact,
                artifact_error,
            )
        })?;
        return Err(error.with_artifacts(&artifact_dir));
    }

    // Batch audits suppress artifacts for registered XFAILs, so an
    // authoritative text-content mismatch can stop before character geometry,
    // page objects, raw XObjects, and raster summaries. Passing and diagnostic
    // paths retain the size threshold to avoid reopening every small PDF.
    let run_early_text_preflight = !write_failure_artifacts
        || candidate_page_count >= EARLY_TEXT_PREFLIGHT_MIN_PAGES
        || candidate_pdf.len().max(golden_pdf.len()) >= EARLY_TEXT_PREFLIGHT_MIN_PDF_BYTES;
    let early_text_mismatch = if run_early_text_preflight {
        first_pdf_page_text_mismatch(
            &candidate_pdf,
            &golden_pdf,
            unordered_extracted_text_content,
        )
        .map_err(|error| {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
        })?
    } else {
        None
    };
    if let Some(mismatch) = early_text_mismatch {
        stage_trace.mark("page-text-preflight-mismatch");
        if !write_failure_artifacts {
            return Err(OfficeGoldenFailure::diagnostic(
                OfficeGoldenComparisonLayer::Text,
                OfficeGoldenDiagnosticKind::TextContent,
                format!(
                    "case {} page {} normalized text content mismatch: candidate={:?}, golden={:?}",
                    case.id, mismatch.page_index, mismatch.candidate, mismatch.golden
                ),
            )
            .at(mismatch.page_index, None));
        }
    }
    stage_trace.mark("page-text");

    let candidate = PdfSummary::from_bytes_for_golden(&candidate_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    let golden = PdfSummary::from_bytes_for_golden(&golden_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    stage_trace.mark("pdf-summary");

    let text_contract =
        match assert_page_geometry_contract(case.id, &candidate, &golden).and_then(|()| {
            assert_text_contract(case.id, &candidate, &golden, &candidate_pdf, &golden_pdf)
        }) {
            Ok(text_contract) => text_contract,
            Err(error) => {
                if !write_failure_artifacts {
                    return Err(error);
                }
                collect_candidate_diagnostics(
                    &source_path,
                    &diagnostic_options,
                    &mut candidate_diagnostics,
                );
                let artifact_dir = write_candidate_artifact(
                    case.id,
                    &candidate_pdf,
                    &golden_pdf,
                    &candidate_font_audit,
                    &candidate_diagnostics,
                    &[0],
                )
                .map_err(|artifact_error| {
                    OfficeGoldenFailure::new(
                        OfficeGoldenComparisonLayer::ComparisonArtifact,
                        artifact_error,
                    )
                })?;
                return Err(error.with_artifacts(&artifact_dir));
            }
        };
    stage_trace.mark("text-contract");

    if let Err(error) = assert_text_font_assignment_contract(
        case.id,
        &text_contract.candidate_lines,
        &text_contract.golden_lines,
        text_contract.pdftotext_confirmed_pdfium_mismatch,
        text_contract.pdftotext_confirmed_ordered_text,
    ) {
        if !write_failure_artifacts {
            return Err(error);
        }
        collect_candidate_diagnostics(
            &source_path,
            &diagnostic_options,
            &mut candidate_diagnostics,
        );
        let artifact_dir = write_candidate_artifact(
            case.id,
            &candidate_pdf,
            &golden_pdf,
            &candidate_font_audit,
            &candidate_diagnostics,
            &[0],
        )
        .map_err(|artifact_error| {
            OfficeGoldenFailure::new(
                OfficeGoldenComparisonLayer::ComparisonArtifact,
                artifact_error,
            )
        })?;
        return Err(error.with_artifacts(&artifact_dir));
    }
    stage_trace.mark("text-font-assignment");
    let text_masks = text_contract.masks;

    let mut page_diffs = Vec::with_capacity(golden.page_count);
    let mut visual_passes = true;
    let mut failing_pages = Vec::new();
    let write_page_artifacts =
        std::env::var("OOXMLSDK_GOLDEN_WRITE_PAGE_ARTIFACTS").is_ok_and(|value| value == "1");
    let mut artifact_dir = if write_page_artifacts {
        Some(
            write_candidate_artifact(
                case.id,
                &candidate_pdf,
                &golden_pdf,
                &candidate_font_audit,
                &candidate_diagnostics,
                &[],
            )
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::ComparisonArtifact, error)
            })?,
        )
    } else {
        None
    };
    let mut failure_artifact_metadata_written = false;
    visit_rendered_page_pairs(
        &candidate_pdf,
        &golden_pdf,
        RASTER_WIDTH,
        |page_index, candidate_page, golden_page| -> DetailedResult<()> {
            let page_bounds = parse_pdf_rect(&golden.media_boxes[page_index]).map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
            })?;
            let comparison_height_px = candidate_page.height_px.min(golden_page.height_px);
            let localized_graphics = localized_graphics_bounds(
                &candidate,
                &golden,
                page_index,
                page_bounds,
                candidate_page.width_px,
                comparison_height_px,
                tolerance,
            )
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
            })?;
            let metrics = visual_diff_metrics(
                &candidate_page,
                &golden_page,
                tolerance.significant_channel_delta,
                text_masks.get(page_index).map(Vec::as_slice).unwrap_or(&[]),
                &localized_graphics,
                page_bounds,
            )
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::VisibleOutput, error)
            })?;
            let page_passes = metrics.significant_pixel_fraction
                <= tolerance.max_significant_pixel_fraction
                && metrics.mean_absolute_channel_delta <= tolerance.max_mean_absolute_channel_delta
                && metrics.max_localized_graphics_significant_pixel_fraction
                    <= tolerance.max_significant_pixel_fraction
                && metrics.max_localized_graphics_mean_absolute_channel_delta
                    <= tolerance.max_mean_absolute_channel_delta;
            visual_passes &= page_passes;
            if write_page_artifacts {
                write_failure_page_artifacts(
                    artifact_dir
                        .as_deref()
                        .expect("page artifact directory should be initialized"),
                    page_index,
                    &candidate_page,
                    &golden_page,
                )
                .map_err(|error| {
                    OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::ComparisonArtifact, error)
                })?;
            }
            if !page_passes {
                if write_failure_artifacts {
                    collect_candidate_diagnostics(
                        &source_path,
                        &diagnostic_options,
                        &mut candidate_diagnostics,
                    );
                    if !failure_artifact_metadata_written {
                        artifact_dir = Some(
                            write_candidate_artifact(
                                case.id,
                                &candidate_pdf,
                                &golden_pdf,
                                &candidate_font_audit,
                                &candidate_diagnostics,
                                &[page_index],
                            )
                            .map_err(|error| {
                                OfficeGoldenFailure::new(
                                    OfficeGoldenComparisonLayer::ComparisonArtifact,
                                    error,
                                )
                            })?,
                        );
                        failure_artifact_metadata_written = true;
                    }
                    let artifact_dir = artifact_dir
                        .as_deref()
                        .expect("failure artifact directory should be initialized");
                    let mut trace_pages = failing_pages.clone();
                    trace_pages.push(page_index);
                    write_candidate_glyph_pages(artifact_dir, &candidate_diagnostics, &trace_pages)
                        .map_err(|error| {
                            OfficeGoldenFailure::new(
                                OfficeGoldenComparisonLayer::ComparisonArtifact,
                                error,
                            )
                        })?;
                    if !write_page_artifacts {
                        write_failure_page_artifacts(
                            artifact_dir,
                            page_index,
                            &candidate_page,
                            &golden_page,
                        )
                        .map_err(|error| {
                            OfficeGoldenFailure::new(
                                OfficeGoldenComparisonLayer::ComparisonArtifact,
                                error,
                            )
                        })?;
                    }
                }
                failing_pages.push(page_index);
            }
            page_diffs.push(metrics);
            Ok(())
        },
    )
    .map_err(|error| match error {
        RenderedPagePairError::Pdf(error) => {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
        }
        RenderedPagePairError::Visit(error) => error,
    })?;
    stage_trace.mark("visible-output");

    let report = OfficeGoldenReport {
        case_id: case.id.to_string(),
        candidate,
        golden,
        page_diffs,
        artifact_dir,
    };
    if visual_passes {
        Ok(report)
    } else {
        let max_significant_pixel_fraction = failing_pages
            .iter()
            .map(|&page_index| report.page_diffs[page_index].significant_pixel_fraction)
            .fold(0.0_f64, f64::max);
        let max_mean_absolute_channel_delta = failing_pages
            .iter()
            .map(|&page_index| report.page_diffs[page_index].mean_absolute_channel_delta)
            .fold(0.0_f64, f64::max);
        let max_channel_delta = failing_pages
            .iter()
            .map(|&page_index| report.page_diffs[page_index].max_channel_delta)
            .max()
            .unwrap_or(0);
        let max_localized_graphics_significant_pixel_fraction = failing_pages
            .iter()
            .map(|&page_index| {
                report.page_diffs[page_index].max_localized_graphics_significant_pixel_fraction
            })
            .fold(0.0_f64, f64::max);
        let max_localized_graphics_mean_absolute_channel_delta = failing_pages
            .iter()
            .map(|&page_index| {
                report.page_diffs[page_index].max_localized_graphics_mean_absolute_channel_delta
            })
            .fold(0.0_f64, f64::max);
        let first_failing_page = failing_pages.first().copied();
        let mut failure = OfficeGoldenFailure::new(
            OfficeGoldenComparisonLayer::VisibleOutput,
            format!(
                "case {} exceeds {:?}; failing pages={} ({} of {}); maxima: significant_pixel_fraction={}, mean_absolute_channel_delta={}, max_channel_delta={}, localized_graphics_significant_pixel_fraction={}, localized_graphics_mean_absolute_channel_delta={}; artifacts={}",
                case.id,
                tolerance,
                format_page_ranges(&failing_pages),
                failing_pages.len(),
                report.page_diffs.len(),
                max_significant_pixel_fraction,
                max_mean_absolute_channel_delta,
                max_channel_delta,
                max_localized_graphics_significant_pixel_fraction,
                max_localized_graphics_mean_absolute_channel_delta,
                report
                    .artifact_dir
                    .as_deref()
                    .map_or_else(|| "none".into(), |path| path.display().to_string())
            ),
        );
        if let Some(page_index) = first_failing_page {
            failure = failure.at(page_index, None);
        }
        Err(failure)
    }
}

struct OfficeGoldenStageTrace<'a> {
    enabled: bool,
    case: OfficeGoldenCase<'a>,
    started: Instant,
}

impl<'a> OfficeGoldenStageTrace<'a> {
    fn new(case: OfficeGoldenCase<'a>) -> Self {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        let enabled = *ENABLED.get_or_init(|| {
            std::env::var("OOXMLSDK_GOLDEN_TRACE_STAGES").is_ok_and(|value| value == "1")
        });
        Self {
            enabled,
            case,
            started: Instant::now(),
        }
    }

    fn mark(&mut self, stage: &str) {
        if !self.enabled {
            return;
        }
        eprintln!(
            "office-golden stage {}/{} stage={stage} elapsed_ms={}",
            self.case.corpus,
            self.case.source,
            self.started.elapsed().as_millis()
        );
        self.started = Instant::now();
    }
}

#[derive(Clone, Debug)]
struct ConversionManifestIdentity {
    status: String,
    reference_engine: String,
    source_sha256: String,
    output_sha256: String,
    environment_id: String,
    output: String,
    converted_at_utc: String,
    elapsed_ms: i64,
    attempts: u32,
}

type ConversionManifestIdentities =
    std::result::Result<BTreeMap<(String, String), ConversionManifestIdentity>, String>;

static CONVERSION_MANIFEST_IDENTITIES: OnceLock<ConversionManifestIdentities> = OnceLock::new();

fn verify_manifest_record(
    root: &Path,
    case: OfficeGoldenCase<'_>,
) -> Result<ConversionManifestIdentity> {
    let records = CONVERSION_MANIFEST_IDENTITIES
        .get_or_init(|| load_conversion_manifest_identities(root))
        .as_ref()
        .map_err(|error| CalibrationError::OfficeGolden(error.clone()))?;
    let key = (case.corpus.to_string(), case.source.to_string());
    let Some(record) = records.get(&key) else {
        return Err(CalibrationError::OfficeGolden(format!(
            "expected exactly one manifest record for {}/{}, found 0",
            case.corpus, case.source
        )));
    };
    for (field, actual, expected) in [
        ("status", record.status.as_str(), "converted"),
        (
            "reference_engine",
            record.reference_engine.as_str(),
            "Microsoft Office",
        ),
        (
            "source_sha256",
            record.source_sha256.as_str(),
            case.source_sha256,
        ),
        (
            "output_sha256",
            record.output_sha256.as_str(),
            case.golden_sha256,
        ),
        (
            "environment_id",
            record.environment_id.as_str(),
            case.environment_id,
        ),
    ] {
        if actual != expected {
            return Err(CalibrationError::OfficeGolden(format!(
                "manifest field {field} mismatch for {}: actual={actual:?}, expected={expected:?}",
                case.id
            )));
        }
    }
    let expected_output = format!("{}.pdf", case.source);
    if record.output != expected_output {
        return Err(CalibrationError::OfficeGolden(format!(
            "manifest output mismatch for {}",
            case.id
        )));
    }
    Ok(record.clone())
}

fn load_conversion_manifest_identities(
    root: &Path,
) -> std::result::Result<BTreeMap<(String, String), ConversionManifestIdentity>, String> {
    let conversion_root = root.join("corpus_pdf_conv");
    let mut manifest_paths = fs::read_dir(&conversion_root)
        .map_err(|error| format!("could not scan {}: {error}", conversion_root.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("manifest.jsonl"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    manifest_paths.sort();

    let mut records = BTreeMap::new();
    for manifest_path in manifest_paths {
        let corpus = manifest_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid corpus path {}", manifest_path.display()))?;
        let manifest = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
        for (line_index, line) in manifest.lines().enumerate() {
            let record: Value = serde_json::from_str(line).map_err(|error| {
                format!(
                    "invalid JSON at {}:{}: {error}",
                    manifest_path.display(),
                    line_index + 1
                )
            })?;
            let string = |field: &str| {
                record
                    .get(field)
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .ok_or_else(|| {
                        format!(
                            "missing string field {field:?} at {}:{}",
                            manifest_path.display(),
                            line_index + 1
                        )
                    })
            };
            let unsigned = |field: &str| {
                record.get(field).and_then(Value::as_u64).ok_or_else(|| {
                    format!(
                        "missing unsigned integer field {field:?} at {}:{}",
                        manifest_path.display(),
                        line_index + 1
                    )
                })
            };
            let source = string("file")?;
            let key = (corpus.to_string(), source.clone());
            let identity = ConversionManifestIdentity {
                status: string("status")?,
                reference_engine: string("reference_engine")?,
                source_sha256: string("source_sha256")?,
                output_sha256: string("output_sha256")?,
                environment_id: string("environment_id")?,
                output: string("output")?,
                converted_at_utc: string("converted_at_utc")?,
                elapsed_ms: i64::try_from(unsigned("elapsed_ms")?).map_err(|_| {
                    format!(
                        "elapsed_ms is out of range at {}:{}",
                        manifest_path.display(),
                        line_index + 1
                    )
                })?,
                attempts: u32::try_from(unsigned("attempts")?).map_err(|_| {
                    format!(
                        "attempts is out of range at {}:{}",
                        manifest_path.display(),
                        line_index + 1
                    )
                })?,
            };
            if records.insert(key, identity).is_some() {
                return Err(format!(
                    "duplicate conversion manifest record for {corpus}/{source}"
                ));
            }
        }
    }
    Ok(records)
}

#[derive(Debug)]
struct ReferenceEnvironmentClock {
    environment_id: String,
    culture: String,
    time_zone: String,
}

type ReferenceEnvironmentClockResult = std::result::Result<ReferenceEnvironmentClock, String>;

static REFERENCE_ENVIRONMENT_CLOCK: OnceLock<ReferenceEnvironmentClockResult> = OnceLock::new();

fn reference_field_update_datetime(
    root: &Path,
    manifest: &ConversionManifestIdentity,
) -> Result<FieldUpdateDateTime> {
    let environment = REFERENCE_ENVIRONMENT_CLOCK
        .get_or_init(|| load_reference_environment_clock(root))
        .as_ref()
        .map_err(|error| CalibrationError::OfficeGolden(error.clone()))?;
    if environment.environment_id != manifest.environment_id {
        return Err(CalibrationError::OfficeGolden(format!(
            "reference environment mismatch for field update time: manifest={:?}, environment={:?}",
            manifest.environment_id, environment.environment_id
        )));
    }
    if manifest.attempts != 1 {
        return parse_utc_datetime_in_reference_time_zone(
            &manifest.converted_at_utc,
            &environment.time_zone,
            &environment.culture,
        )
        .map_err(CalibrationError::OfficeGolden);
    }
    let completed_at = manifest
        .converted_at_utc
        .parse::<Timestamp>()
        .map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid Office golden UTC conversion time {:?}: {error}",
                manifest.converted_at_utc
            ))
        })?;
    // convert_office_corpus.ps1 records `$started` before a source's only
    // conversion attempt, but writes converted_at_utc only after Office has
    // exported the PDF and the harness has validated, copied, and hashed it.
    // A live DATE/TIME field is expanded inside that interval. For a
    // single-attempt record, its recorded start is therefore the stable
    // refresh boundary; with retries, the successful attempt's start is not
    // recoverable and the existing completion timestamp remains authoritative.
    let update_at = completed_at
        .checked_sub(SignedDuration::from_millis(manifest.elapsed_ms))
        .map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "could not recover Office golden conversion start from {:?} - {}ms: {error}",
                manifest.converted_at_utc, manifest.elapsed_ms
            ))
        })?;
    field_update_datetime_in_reference_time_zone(
        update_at,
        &environment.time_zone,
        &environment.culture,
    )
    .map_err(CalibrationError::OfficeGolden)
}

fn reference_field_update_time_zone(
    root: &Path,
    manifest: &ConversionManifestIdentity,
) -> Result<String> {
    let environment = REFERENCE_ENVIRONMENT_CLOCK
        .get_or_init(|| load_reference_environment_clock(root))
        .as_ref()
        .map_err(|error| CalibrationError::OfficeGolden(error.clone()))?;
    if environment.environment_id != manifest.environment_id {
        return Err(CalibrationError::OfficeGolden(format!(
            "reference environment mismatch for field update time zone: manifest={:?}, environment={:?}",
            manifest.environment_id, environment.environment_id
        )));
    }
    reference_iana_time_zone(&environment.time_zone, &environment.culture)
        .map_err(CalibrationError::OfficeGolden)
}

fn load_reference_environment_clock(root: &Path) -> ReferenceEnvironmentClockResult {
    let path = root.join("corpus_pdf_conv").join("environment.json");
    let bytes =
        fs::read(&path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let record: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))?;
    let environment_id = record
        .get("environment_id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing environment_id in {}", path.display()))?
        .to_string();
    let culture = record
        .pointer("/environment/locale/culture")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing environment.locale.culture in {}", path.display()))?
        .to_string();
    let time_zone = record
        .pointer("/environment/locale/time_zone")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing environment.locale.time_zone in {}", path.display()))?
        .to_string();
    Ok(ReferenceEnvironmentClock {
        environment_id,
        culture,
        time_zone,
    })
}

fn parse_utc_datetime_in_reference_time_zone(
    value: &str,
    time_zone: &str,
    culture: &str,
) -> std::result::Result<FieldUpdateDateTime, String> {
    let timestamp = value
        .parse::<Timestamp>()
        .map_err(|error| format!("invalid Office golden UTC conversion time {value:?}: {error}"))?;
    field_update_datetime_in_reference_time_zone(timestamp, time_zone, culture)
}

fn field_update_datetime_in_reference_time_zone(
    timestamp: Timestamp,
    time_zone: &str,
    culture: &str,
) -> std::result::Result<FieldUpdateDateTime, String> {
    let iana_time_zone = reference_iana_time_zone(time_zone, culture)?;
    let local = timestamp.in_tz(&iana_time_zone).map_err(|error| {
        format!(
            "could not apply Office golden time zone {time_zone:?} ({iana_time_zone:?}) to {timestamp:?}: {error}"
        )
    })?;
    Ok(FieldUpdateDateTime {
        year: u16::try_from(local.year()).map_err(|_| {
            format!(
                "Office golden conversion year is out of range: {}",
                local.year()
            )
        })?,
        month: u8::try_from(local.month()).expect("Jiff month is in range"),
        day: u8::try_from(local.day()).expect("Jiff day is in range"),
        hour: u8::try_from(local.hour()).expect("Jiff hour is in range"),
        minute: u8::try_from(local.minute()).expect("Jiff minute is in range"),
        second: u8::try_from(local.second()).expect("Jiff second is in range"),
    })
}

fn reference_iana_time_zone(
    system_time_zone: &str,
    culture: &str,
) -> std::result::Result<String, String> {
    let iana_parser = IanaParserExtended::new();
    let direct = iana_parser.parse(system_time_zone);
    if !direct.time_zone.is_unknown() {
        return Ok(direct.canonical.to_string());
    }

    let region = culture
        .parse::<Locale>()
        .ok()
        .and_then(|locale| locale.id.region);
    let time_zone = WindowsParser::new()
        .parse(system_time_zone, region)
        .ok_or_else(|| {
            format!(
                "unsupported Office golden time zone {system_time_zone:?} for culture {culture:?}"
            )
        })?;
    iana_parser
        .iter()
        .find(|candidate| candidate.time_zone == time_zone)
        .map(|candidate| candidate.canonical.to_string())
        .ok_or_else(|| {
            format!(
                "Office golden time zone {system_time_zone:?} mapped to CLDR identity {:?} without a canonical IANA zone",
                time_zone.as_str()
            )
        })
}

fn verify_sha256(label: &str, path: &Path, bytes: &[u8], expected: &str) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(CalibrationError::OfficeGolden(format!(
            "{label} SHA-256 mismatch for {}: actual={actual}, expected={expected}",
            path.display()
        )));
    }
    Ok(())
}

fn assert_page_geometry_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> DetailedResult<()> {
    if candidate.page_count != golden.page_count {
        return Err(OfficeGoldenFailure::diagnostic(
            OfficeGoldenComparisonLayer::PageGeometry,
            OfficeGoldenDiagnosticKind::PageCount,
            format!(
                "case {case_id} page count mismatch: candidate={}, golden={}",
                candidate.page_count, golden.page_count
            ),
        ));
    }
    if candidate.media_boxes.len() != golden.media_boxes.len() {
        return Err(OfficeGoldenFailure::diagnostic(
            OfficeGoldenComparisonLayer::PageGeometry,
            OfficeGoldenDiagnosticKind::PageCount,
            format!(
                "case {case_id} media box count mismatch: candidate={}, golden={}",
                candidate.media_boxes.len(),
                golden.media_boxes.len()
            ),
        ));
    }
    for (page_index, (candidate_box, golden_box)) in candidate
        .media_boxes
        .iter()
        .zip(&golden.media_boxes)
        .enumerate()
    {
        let candidate_box = parse_pdf_rect(candidate_box).map_err(|error| {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                .at(page_index, None)
        })?;
        let golden_box = parse_pdf_rect(golden_box).map_err(|error| {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                .at(page_index, None)
        })?;
        let max_delta = [
            (candidate_box.left - golden_box.left).abs(),
            (candidate_box.bottom - golden_box.bottom).abs(),
            (candidate_box.right - golden_box.right).abs(),
            (candidate_box.top - golden_box.top).abs(),
        ]
        .into_iter()
        .fold(0.0_f32, f32::max);
        if max_delta > MEDIA_BOX_TOLERANCE_PT {
            return Err(OfficeGoldenFailure::diagnostic(
                OfficeGoldenComparisonLayer::PageGeometry,
                OfficeGoldenDiagnosticKind::PageGeometry,
                format!(
                    "case {case_id} page {page_index} media box mismatch: candidate={candidate_box:?}, golden={golden_box:?}"
                ),
            )
            .at(page_index, None));
        }
    }
    Ok(())
}

fn assert_text_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
    candidate_pdf: &[u8],
    golden_pdf: &[u8],
) -> DetailedResult<TextContract> {
    let candidate_text = normalized_page_text(candidate);
    let golden_text = normalized_page_text(golden);
    let candidate_content = page_text_content_bags(&candidate_text);
    let golden_content = page_text_content_bags(&golden_text);
    let mut content_matches = candidate_content == golden_content;
    let mut pdftotext_confirmed_pdfium_mismatch = false;
    let mut pdftotext_confirmed_ordered_text = false;
    let mut pdftotext_checked_ordered_text = false;
    if !content_matches && candidate_content.len() == golden_content.len() {
        pdftotext_checked_ordered_text = true;
        content_matches = true;
        let mut ordered_text_matches = true;
        for page_index in 0..candidate_content.len() {
            let candidate_fallback =
                pdftotext_page(candidate_pdf, page_index).map_err(|error| {
                    OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                        .at(page_index, None)
                })?;
            let golden_fallback = pdftotext_page(golden_pdf, page_index).map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                    .at(page_index, None)
            })?;
            // The ordered-text confirmation enables mirror canonicalization in
            // later document-wide line and font comparisons, so Poppler must
            // independently confirm every page, including pages where PDFium's
            // unordered content already agrees.
            ordered_text_matches &= pdftotext_line_content_keys(&candidate_fallback)
                == pdftotext_line_content_keys(&golden_fallback);
            if candidate_content[page_index] != golden_content[page_index]
                && unordered_extracted_text_content(&candidate_fallback)
                    != unordered_extracted_text_content(&golden_fallback)
            {
                content_matches = false;
                break;
            }
        }
        pdftotext_confirmed_pdfium_mismatch = content_matches;
        pdftotext_confirmed_ordered_text = content_matches && ordered_text_matches;
    }
    if !content_matches {
        return Err(OfficeGoldenFailure::diagnostic(
            OfficeGoldenComparisonLayer::Text,
            OfficeGoldenDiagnosticKind::TextContent,
            format!(
                "case {case_id} normalized page text mismatch: candidate={candidate_text:?}, golden={golden_text:?}"
            ),
        ));
    }
    assert_text_style_contract(case_id, candidate, golden)?;
    let mut candidate_lines =
        text_line_contracts(candidate, TextLineGeometry::Pdfium).map_err(|error| {
            OfficeGoldenFailure::diagnostic(
                OfficeGoldenComparisonLayer::Text,
                OfficeGoldenDiagnosticKind::TextReconstruction,
                error,
            )
        })?;
    let mut golden_lines =
        text_line_contracts(golden, TextLineGeometry::Pdfium).map_err(|error| {
            OfficeGoldenFailure::diagnostic(
                OfficeGoldenComparisonLayer::Text,
                OfficeGoldenDiagnosticKind::TextReconstruction,
                error,
            )
        })?;
    if !pdftotext_checked_ordered_text
        && text_line_topology_differs(
            &candidate_lines,
            &golden_lines,
            pdftotext_confirmed_pdfium_mismatch,
            pdftotext_confirmed_ordered_text,
        )
    {
        // A mirrored pair can leave PDFium's unordered page character bag
        // unchanged, so the page-level fallback above is not entered. Require
        // Poppler to independently confirm every ordered line before enabling
        // the same narrowly-scoped mirror canonicalization for that case.
        pdftotext_confirmed_ordered_text =
            pdftotext_ordered_text_matches(candidate_pdf, golden_pdf, candidate_content.len())?;
        pdftotext_confirmed_pdfium_mismatch = pdftotext_confirmed_ordered_text;
    }
    if text_line_topology_differs(
        &candidate_lines,
        &golden_lines,
        pdftotext_confirmed_pdfium_mismatch,
        pdftotext_confirmed_ordered_text,
    ) {
        // PDFium exposes painted glyph bounds rather than source line
        // ownership. Two unrelated text objects hundreds of points apart can
        // therefore be merged when their font boxes overlap by a fraction of
        // a point on only one side of the comparison. Reconstruct from the
        // embedded font baseline not only when the line count differs, but
        // also when that unstable merge changes line content. This selects the
        // already-stricter 0.25pt origin test; it does not relax any geometry
        // or content tolerance.
        candidate_lines =
            text_line_contracts(candidate, TextLineGeometry::EmbeddedFont).map_err(|error| {
                OfficeGoldenFailure::diagnostic(
                    OfficeGoldenComparisonLayer::Text,
                    OfficeGoldenDiagnosticKind::TextReconstruction,
                    error,
                )
            })?;
        golden_lines =
            text_line_contracts(golden, TextLineGeometry::EmbeddedFont).map_err(|error| {
                OfficeGoldenFailure::diagnostic(
                    OfficeGoldenComparisonLayer::Text,
                    OfficeGoldenDiagnosticKind::TextReconstruction,
                    error,
                )
            })?;
    }
    align_golden_text_lines_by_content_and_position(
        &candidate_lines,
        &mut golden_lines,
        pdftotext_confirmed_pdfium_mismatch,
        pdftotext_confirmed_ordered_text,
    );
    let masks = assert_text_line_geometry(
        case_id,
        candidate,
        golden,
        &candidate_lines,
        &golden_lines,
        pdftotext_confirmed_pdfium_mismatch,
        pdftotext_confirmed_ordered_text,
    )?;
    Ok(TextContract {
        masks,
        candidate_lines,
        golden_lines,
        pdftotext_confirmed_pdfium_mismatch,
        pdftotext_confirmed_ordered_text,
    })
}

fn text_line_topology_differs(
    candidate: &[Vec<TextLineContract>],
    golden: &[Vec<TextLineContract>],
    accept_pdfium_bidi_mirroring: bool,
    accept_pdfium_ltr_mirroring: bool,
) -> bool {
    candidate.len() != golden.len()
        || candidate.iter().zip(golden).any(|(candidate, golden)| {
            candidate.len() != golden.len()
                || candidate.iter().zip(golden).any(|(candidate, golden)| {
                    extracted_text_line_content_key(
                        &candidate.text,
                        accept_pdfium_bidi_mirroring,
                        accept_pdfium_ltr_mirroring,
                    ) != extracted_text_line_content_key(
                        &golden.text,
                        accept_pdfium_bidi_mirroring,
                        accept_pdfium_ltr_mirroring,
                    )
                })
        })
}

fn align_golden_text_lines_by_content_and_position(
    candidate: &[Vec<TextLineContract>],
    golden: &mut [Vec<TextLineContract>],
    accept_pdfium_bidi_mirroring: bool,
    accept_pdfium_ltr_mirroring: bool,
) {
    for (candidate_page, golden_page) in candidate.iter().zip(golden.iter_mut()) {
        if candidate_page.len() != golden_page.len() {
            continue;
        }
        let mut unmatched = golden_page.iter().cloned().map(Some).collect::<Vec<_>>();
        let mut aligned = Vec::with_capacity(candidate_page.len());
        for candidate_line in candidate_page {
            let candidate_key = extracted_text_line_content_key(
                &candidate_line.text,
                accept_pdfium_bidi_mirroring,
                accept_pdfium_ltr_mirroring,
            );
            let Some((best_index, _)) = unmatched
                .iter()
                .enumerate()
                .filter_map(|(index, line)| line.as_ref().map(|line| (index, line)))
                .filter(|(_, line)| {
                    extracted_text_line_content_key(
                        &line.text,
                        accept_pdfium_bidi_mirroring,
                        accept_pdfium_ltr_mirroring,
                    ) == candidate_key
                })
                .map(|(index, line)| {
                    let dx = candidate_line.origin_x - line.origin_x;
                    let dy = candidate_line.origin_y - line.origin_y;
                    (index, dx * dx + dy * dy)
                })
                .min_by(|left, right| left.1.total_cmp(&right.1))
            else {
                return;
            };
            aligned.push(
                unmatched[best_index]
                    .take()
                    .expect("selected golden text line is unmatched"),
            );
        }
        // Line order inferred from PDF text enumeration is not semantic.
        // Pair equal-content lines by their nearest page position so distant
        // columns cannot reorder one another when their baselines differ by a
        // fraction of a point. Geometry is still checked immediately after
        // this alignment with the original strict tolerances.
        *golden_page = aligned;
    }
}

struct TextContract {
    masks: Vec<Vec<PdfBounds>>,
    candidate_lines: Vec<Vec<TextLineContract>>,
    golden_lines: Vec<Vec<TextLineContract>>,
    pdftotext_confirmed_pdfium_mismatch: bool,
    pdftotext_confirmed_ordered_text: bool,
}

#[derive(Clone, Debug)]
struct TextLineContract {
    text: String,
    font_runs: Vec<TextFontRunContract>,
    bounds: PdfBounds,
    origin_x: f32,
    origin_y: f32,
    writing_direction: TextWritingDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextFontRunContract {
    font_name: String,
    text: String,
}

#[derive(Clone, Copy, Debug)]
struct TextWritingDirection {
    angle_degrees: f32,
    advance_x: f32,
    advance_y: f32,
}

impl TextWritingDirection {
    fn from_degrees(angle_degrees: f32) -> Self {
        let angle_degrees = angle_degrees.rem_euclid(360.0);
        let angle_radians = angle_degrees.to_radians();
        Self {
            angle_degrees,
            advance_x: angle_radians.cos(),
            advance_y: angle_radians.sin(),
        }
    }

    fn advance_coordinate(self, x: f32, y: f32) -> f32 {
        x * self.advance_x + y * self.advance_y
    }

    fn cross_coordinate(self, x: f32, y: f32) -> f32 {
        x * -self.advance_y + y * self.advance_x
    }

    fn projected_bounds(self, bounds: PdfBounds, advance_axis: bool) -> (f32, f32) {
        let (axis_x, axis_y) = if advance_axis {
            (self.advance_x, self.advance_y)
        } else {
            (-self.advance_y, self.advance_x)
        };
        let center_x = (bounds.left + bounds.right) * 0.5;
        let center_y = (bounds.bottom + bounds.top) * 0.5;
        let radius = (bounds.right - bounds.left).abs() * 0.5 * axis_x.abs()
            + (bounds.top - bounds.bottom).abs() * 0.5 * axis_y.abs();
        let center = center_x * axis_x + center_y * axis_y;
        (center - radius, center + radius)
    }
}

#[derive(Clone, Debug)]
struct TextCharacterContract {
    text: String,
    font_name: String,
    bounds: PdfBounds,
    origin_x: f32,
    origin_y: f32,
    writing_direction: TextWritingDirection,
}

fn assert_text_style_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> DetailedResult<()> {
    use std::collections::BTreeSet;

    let style_set = |summary: &PdfSummary| {
        summary
            .text_objects
            .iter()
            .enumerate()
            .filter(|(_, object)| !object.text.trim().is_empty())
            .map(|(index, object)| {
                let object = pdfium_visible_text_style_object(&summary.text_objects, index)
                    .unwrap_or(object);
                // PDF /BaseFont is a producer-specific PostScript name (for
                // example, Arial versus ArialMT). Canonicalize separators and
                // the common MT/PSMT foundry suffixes while retaining explicit
                // Bold/Italic markers. The PDF font-family descriptor is not
                // used here because embedded subsets may describe Calibri
                // Light as either "Calibri" or "Calibri Light".
                (
                    canonical_pdf_base_font_name(&object.font_name),
                    object.render_mode.clone(),
                    object.fill_color.clone(),
                )
            })
            .collect::<BTreeSet<_>>()
    };
    let candidate_styles = style_set(candidate);
    let golden_styles = style_set(golden);
    if !text_style_sets_equivalent(&candidate_styles, &golden_styles) {
        return Err(OfficeGoldenFailure::diagnostic(
            OfficeGoldenComparisonLayer::Text,
            OfficeGoldenDiagnosticKind::TextStyle,
            format!(
                "case {case_id} text style set mismatch: candidate={candidate_styles:?}, golden={golden_styles:?}"
            ),
        ));
    }
    // Adobe's PDF reference defines final glyph placement through the complete
    // text rendering matrix. A producer's `Tf` and extracted vertical scale
    // are therefore not independent visible-output contracts. Effective size
    // remains constrained below by glyph baseline and horizontal ink bounds,
    // while family, explicit style, rendering mode, and color are checked here.
    Ok(())
}

fn pdfium_visible_text_style_object(
    objects: &[crate::pdf_extract::TextObjectSummary],
    index: usize,
) -> Option<&crate::pdf_extract::TextObjectSummary> {
    let shadow = objects.get(index)?;
    let foreground = objects.get(index + 1)?;
    if foreground.text.is_empty()
        && foreground.page_index == shadow.page_index
        && foreground.font_name == shadow.font_name
        && foreground.scaled_font_size == shadow.scaled_font_size
        && foreground.unscaled_font_size == shadow.unscaled_font_size
        && foreground.render_mode == shadow.render_mode
        && foreground
            .fill_color
            .as_deref()
            .and_then(parse_pdf_style_color)
            .is_some_and(|(_, alpha)| alpha == u8::MAX)
        && text_object_bounds_are_overlapping_translations(shadow, foreground)
    {
        // ECMA-376 17.3.2.31 places a classic run shadow beneath the text.
        // Word's fixed-format writer paints that shadow first and the opaque
        // foreground second. For tdf139549 it emits one object per layer and
        // glyph; PDFium retains the foreground object's paint and bounds but
        // returns an empty string for that overlapping duplicate. Select the
        // later visible layer for style only. Requiring an adjacent,
        // same-font, same-size, strongly overlapping translation excludes
        // unrelated empty objects and the distinct duplicated text exposed by
        // tdf154370's outline/imprint effects.
        Some(foreground)
    } else {
        None
    }
}

fn text_object_bounds_are_overlapping_translations(
    first: &crate::pdf_extract::TextObjectSummary,
    second: &crate::pdf_extract::TextObjectSummary,
) -> bool {
    const MAX_TRANSLATION_PT: f32 = 3.0;
    const SIZE_TOLERANCE_PT: f32 = 0.1;
    const MIN_OVERLAP_FRACTION: f32 = 0.75;

    let (Some(first), Some(second)) = (
        first
            .bounds
            .as_deref()
            .and_then(|value| parse_pdf_rect(value).ok()),
        second
            .bounds
            .as_deref()
            .and_then(|value| parse_pdf_rect(value).ok()),
    ) else {
        return false;
    };
    let first_width = (first.right - first.left).abs();
    let first_height = (first.top - first.bottom).abs();
    let second_width = (second.right - second.left).abs();
    let second_height = (second.top - second.bottom).abs();
    if first_width <= f32::EPSILON
        || first_height <= f32::EPSILON
        || (first_width - second_width).abs() > SIZE_TOLERANCE_PT
        || (first_height - second_height).abs() > SIZE_TOLERANCE_PT
    {
        return false;
    }

    let first_center_x = (first.left + first.right) * 0.5;
    let first_center_y = (first.bottom + first.top) * 0.5;
    let second_center_x = (second.left + second.right) * 0.5;
    let second_center_y = (second.bottom + second.top) * 0.5;
    if (first_center_x - second_center_x).abs() > MAX_TRANSLATION_PT
        || (first_center_y - second_center_y).abs() > MAX_TRANSLATION_PT
    {
        return false;
    }

    let overlap_width = first.right.min(second.right) - first.left.max(second.left);
    let overlap_height = first.top.min(second.top) - first.bottom.max(second.bottom);
    if overlap_width <= 0.0 || overlap_height <= 0.0 {
        return false;
    }
    let overlap_area = overlap_width * overlap_height;
    let smaller_area = (first_width * first_height).min(second_width * second_height);
    overlap_area / smaller_area >= MIN_OVERLAP_FRACTION
}

fn text_style_sets_equivalent(
    candidate: &std::collections::BTreeSet<(String, String, Option<String>)>,
    golden: &std::collections::BTreeSet<(String, String, Option<String>)>,
) -> bool {
    candidate.len() == golden.len()
        && candidate.iter().all(|candidate_style| {
            golden.iter().any(|golden_style| {
                candidate_style.0 == golden_style.0
                    && candidate_style.1 == golden_style.1
                    && pdf_style_colors_equivalent(&candidate_style.2, &golden_style.2)
            })
        })
        && golden.iter().all(|golden_style| {
            candidate.iter().any(|candidate_style| {
                candidate_style.0 == golden_style.0
                    && candidate_style.1 == golden_style.1
                    && pdf_style_colors_equivalent(&candidate_style.2, &golden_style.2)
            })
        })
}

fn pdf_style_colors_equivalent(candidate: &Option<String>, golden: &Option<String>) -> bool {
    match (candidate, golden) {
        (Some(candidate), Some(golden)) => {
            match (
                parse_pdf_style_color(candidate),
                parse_pdf_style_color(golden),
            ) {
                (Some((candidate_rgb, candidate_alpha)), Some((golden_rgb, golden_alpha))) => {
                    candidate_alpha == golden_alpha
                        && candidate_rgb
                            .into_iter()
                            .zip(golden_rgb)
                            .all(|(candidate, golden)| {
                                candidate.abs_diff(golden) <= TEXT_COLOR_CHANNEL_TOLERANCE
                            })
                }
                _ => candidate == golden,
            }
        }
        (None, None) => true,
        _ => false,
    }
}

fn parse_pdf_style_color(value: &str) -> Option<([u8; 3], u8)> {
    let (rgb, alpha) = value.strip_prefix('#')?.split_once('@')?;
    if rgb.len() != 6 || alpha.len() != 2 {
        return None;
    }
    Some((
        [
            u8::from_str_radix(&rgb[0..2], 16).ok()?,
            u8::from_str_radix(&rgb[2..4], 16).ok()?,
            u8::from_str_radix(&rgb[4..6], 16).ok()?,
        ],
        u8::from_str_radix(alpha, 16).ok()?,
    ))
}

fn canonical_pdf_base_font_name(font_name: &str) -> String {
    let mut normalized = font_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if let Some(without_suffix) = normalized.strip_suffix("psmt") {
        normalized.truncate(without_suffix.len());
    } else if let Some(without_suffix) = normalized.strip_suffix("mt") {
        normalized.truncate(without_suffix.len());
    }
    if let Some(without_suffix) = normalized.strip_suffix("regular") {
        normalized.truncate(without_suffix.len());
    }
    normalized
}

fn assert_text_line_geometry(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
    candidate_lines: &[Vec<TextLineContract>],
    golden_lines: &[Vec<TextLineContract>],
    accept_pdfium_bidi_mirroring: bool,
    accept_pdfium_ltr_mirroring: bool,
) -> DetailedResult<Vec<Vec<PdfBounds>>> {
    let mut masks = vec![Vec::new(); candidate.page_count];
    for page_index in 0..candidate.page_count {
        let candidate_page = &candidate_lines[page_index];
        let golden_page = &golden_lines[page_index];
        let golden_page_bounds =
            parse_pdf_rect(&golden.media_boxes[page_index]).map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                    .at(page_index, None)
            })?;
        let edge_tolerance = text_edge_tolerance_pt(golden_page_bounds);
        if candidate_page.len() != golden_page.len() {
            let line_summary = |lines: &[TextLineContract]| {
                lines
                    .iter()
                    .map(|line| {
                        format!(
                            "{:?}@({},{}),{}deg:{:?}",
                            line.text,
                            line.origin_x,
                            line.origin_y,
                            line.writing_direction.angle_degrees,
                            line.bounds
                        )
                    })
                    .collect::<Vec<_>>()
            };
            return Err(OfficeGoldenFailure::diagnostic(
                OfficeGoldenComparisonLayer::Text,
                OfficeGoldenDiagnosticKind::TextLineCount,
                format!(
                    "case {case_id} page {page_index} text line count mismatch: candidate={}, golden={}; candidate_lines={:?}; golden_lines={:?}",
                    candidate_page.len(),
                    golden_page.len(),
                    line_summary(candidate_page),
                    line_summary(golden_page),
                ),
            )
            .at(page_index, None));
        }
        for (line_index, (candidate_line, golden_line)) in
            candidate_page.iter().zip(golden_page).enumerate()
        {
            if extracted_text_line_content_key(
                &candidate_line.text,
                accept_pdfium_bidi_mirroring,
                accept_pdfium_ltr_mirroring,
            ) != extracted_text_line_content_key(
                &golden_line.text,
                accept_pdfium_bidi_mirroring,
                accept_pdfium_ltr_mirroring,
            ) {
                return Err(OfficeGoldenFailure::diagnostic(
                    OfficeGoldenComparisonLayer::Text,
                    OfficeGoldenDiagnosticKind::TextLineContent,
                    format!(
                        "case {case_id} page {page_index} line {line_index} text mismatch: candidate={:?}, golden={:?}",
                        candidate_line.text, golden_line.text
                    ),
                )
                .at(page_index, Some(line_index)));
            }
            if !same_writing_axis(
                candidate_line.writing_direction,
                golden_line.writing_direction,
            ) {
                return Err(OfficeGoldenFailure::diagnostic(
                    OfficeGoldenComparisonLayer::Text,
                    OfficeGoldenDiagnosticKind::TextReconstruction,
                    format!(
                        "case {case_id} page {page_index} line {line_index} writing-axis mismatch: candidate={}deg, golden={}deg; text={:?}",
                        candidate_line.writing_direction.angle_degrees,
                        golden_line.writing_direction.angle_degrees,
                        candidate_line.text
                    ),
                )
                .at(page_index, Some(line_index)));
            }
            // Compare in the golden line's coordinate system. For ordinary
            // horizontal text these are exactly the historical left, right,
            // and baseline-y checks. Rotated text uses the corresponding
            // advance start/end and cross-axis baseline instead of treating
            // horizontal and vertical content as the same line.
            let writing_direction = golden_line.writing_direction;
            let (candidate_start, candidate_end) =
                writing_direction.projected_bounds(candidate_line.bounds, true);
            let (golden_start, golden_end) =
                writing_direction.projected_bounds(golden_line.bounds, true);
            let candidate_cross_origin = writing_direction
                .cross_coordinate(candidate_line.origin_x, candidate_line.origin_y);
            let golden_cross_origin =
                writing_direction.cross_coordinate(golden_line.origin_x, golden_line.origin_y);
            let golden_extent = golden_end - golden_start;
            let extent_tolerance =
                (golden_extent.abs() * TEXT_WIDTH_TOLERANCE_RATIO).max(edge_tolerance);
            for (edge, candidate_value, golden_value, tolerance) in [
                (
                    "advance start",
                    candidate_start,
                    golden_start,
                    edge_tolerance,
                ),
                ("advance end", candidate_end, golden_end, extent_tolerance),
                (
                    "cross-axis baseline origin",
                    candidate_cross_origin,
                    golden_cross_origin,
                    edge_tolerance,
                ),
            ] {
                if (candidate_value - golden_value).abs() > tolerance {
                    let diagnostic_kind = if edge == "cross-axis baseline origin" {
                        OfficeGoldenDiagnosticKind::TextBaseline
                    } else {
                        OfficeGoldenDiagnosticKind::TextHorizontalBounds
                    };
                    return Err(OfficeGoldenFailure::diagnostic(
                        OfficeGoldenComparisonLayer::Text,
                        diagnostic_kind,
                        format!(
                            "case {case_id} page {page_index} line {line_index} {edge} mismatch: candidate={candidate_value}, golden={golden_value}, tolerance={tolerance}; text={:?}; candidate_bounds={:?}; golden_bounds={:?}",
                            candidate_line.text, candidate_line.bounds, golden_line.bounds
                        ),
                    )
                    .at(page_index, Some(line_index)));
                }
            }
            masks[page_index].push(union_pdf_bounds(candidate_line.bounds, golden_line.bounds));
        }
    }
    Ok(masks)
}

fn assert_text_font_assignment_contract(
    case_id: &str,
    candidate_lines: &[Vec<TextLineContract>],
    golden_lines: &[Vec<TextLineContract>],
    accept_pdfium_bidi_mirroring: bool,
    accept_pdfium_ltr_mirroring: bool,
) -> DetailedResult<()> {
    // MS-OI29500 17.3.2.26 assigns fonts by character class, while PDF
    // producers may split the same run into different text objects. Compare
    // the character-level font assignment after spatial line reconstruction.
    for (page_index, (candidate_page, golden_page)) in
        candidate_lines.iter().zip(golden_lines).enumerate()
    {
        for (line_index, (candidate_line, golden_line)) in
            candidate_page.iter().zip(golden_page).enumerate()
        {
            let canonicalize_mirrors = accept_pdfium_bidi_mirroring
                && (accept_pdfium_ltr_mirroring
                    || contains_strong_rtl_character(&candidate_line.text)
                    || contains_strong_rtl_character(&golden_line.text));
            let font_runs_match = candidate_line.font_runs.len() == golden_line.font_runs.len()
                && candidate_line
                    .font_runs
                    .iter()
                    .zip(&golden_line.font_runs)
                    .all(|(candidate_run, golden_run)| {
                        candidate_run.font_name == golden_run.font_name
                            && extracted_text_content_key_with_bidi_mirroring(
                                &candidate_run.text,
                                canonicalize_mirrors,
                            ) == extracted_text_content_key_with_bidi_mirroring(
                                &golden_run.text,
                                canonicalize_mirrors,
                            )
                    });
            if !font_runs_match {
                return Err(OfficeGoldenFailure::diagnostic(
                    OfficeGoldenComparisonLayer::Font,
                    OfficeGoldenDiagnosticKind::FontAssignment,
                    format!(
                        "case {case_id} page {page_index} line {line_index} font assignment mismatch: candidate={:?}, golden={:?}",
                        candidate_line.font_runs, golden_line.font_runs
                    ),
                )
                .at(page_index, Some(line_index)));
            }
        }
    }
    Ok(())
}

fn text_edge_tolerance_pt(page_bounds: PdfBounds) -> f32 {
    (page_bounds.width().abs() / RASTER_WIDTH as f32 * TEXT_EDGE_TOLERANCE_RASTER_PIXELS)
        .max(TEXT_EDGE_TOLERANCE_MIN_PT)
}

#[derive(Clone, Copy)]
enum TextLineGeometry {
    Pdfium,
    EmbeddedFont,
}

fn text_line_contracts(
    summary: &PdfSummary,
    geometry: TextLineGeometry,
) -> Result<Vec<Vec<TextLineContract>>> {
    let mut characters = vec![Vec::<TextCharacterContract>::new(); summary.page_count];
    for character in &summary.text_chars {
        if character.text.chars().all(is_extracted_whitespace) {
            continue;
        }
        let bounds = match geometry {
            TextLineGeometry::Pdfium => parse_pdf_rect(&character.bounds),
            TextLineGeometry::EmbeddedFont => parse_pdf_rect(
                character
                    .font_metric_bounds
                    .as_deref()
                    .unwrap_or(&character.bounds),
            ),
        }
        .map_err(CalibrationError::PdfiumExtraction)?;
        let origin_x = character.origin_x.parse::<f32>().map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid extracted text x origin {:?}: {error}",
                character.origin_x
            ))
        })?;
        let origin_y = character.origin_y.parse::<f32>().map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid extracted text y origin {:?}: {error}",
                character.origin_y
            ))
        })?;
        let angle_degrees = character.angle_degrees.parse::<f32>().map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid extracted text angle {:?}: {error}",
                character.angle_degrees
            ))
        })?;
        let page = characters.get_mut(character.page_index).ok_or_else(|| {
            CalibrationError::OfficeGolden(format!(
                "text character references missing page {}",
                character.page_index
            ))
        })?;
        page.push(TextCharacterContract {
            text: character.text.clone(),
            font_name: canonical_pdf_base_font_name(&character.font_name),
            bounds,
            origin_x,
            origin_y,
            writing_direction: TextWritingDirection::from_degrees(angle_degrees),
        });
    }

    let mut pages = Vec::with_capacity(characters.len());
    for mut page_characters in characters {
        page_characters.sort_by(|left, right| {
            right
                .origin_y
                .total_cmp(&left.origin_y)
                .then_with(|| left.origin_x.total_cmp(&right.origin_x))
                .then_with(|| {
                    left.writing_direction
                        .angle_degrees
                        .total_cmp(&right.writing_direction.angle_degrees)
                })
        });
        let mut line_characters = Vec::<Vec<TextCharacterContract>>::new();
        for character in page_characters {
            if let Some(line) = line_characters.iter_mut().find(|line| {
                line.first()
                    .is_some_and(|first| text_characters_share_line(first, &character, geometry))
            }) {
                line.push(character);
            } else {
                line_characters.push(vec![character]);
            }
        }
        let mut lines = line_characters
            .into_iter()
            .map(|mut characters| {
                let writing_direction = characters
                    .first()
                    .expect("a spatial text line always contains a character")
                    .writing_direction;
                characters.sort_by(|left, right| {
                    writing_direction
                        .advance_coordinate(left.origin_x, left.origin_y)
                        .total_cmp(
                            &writing_direction.advance_coordinate(right.origin_x, right.origin_y),
                        )
                        .then_with(|| left.origin_x.total_cmp(&right.origin_x))
                        .then_with(|| right.origin_y.total_cmp(&left.origin_y))
                });
                let first = characters
                    .first()
                    .expect("a spatial text line always contains a character");
                let mut line = TextLineContract {
                    text: String::new(),
                    font_runs: Vec::new(),
                    bounds: first.bounds,
                    origin_x: first.origin_x,
                    origin_y: first.origin_y,
                    writing_direction,
                };
                for character in characters {
                    line.text.push_str(&character.text);
                    if let Some(run) = line
                        .font_runs
                        .last_mut()
                        .filter(|run| run.font_name == character.font_name)
                    {
                        run.text.push_str(&character.text);
                    } else {
                        line.font_runs.push(TextFontRunContract {
                            font_name: character.font_name,
                            text: character.text.clone(),
                        });
                    }
                    line.bounds = union_pdf_bounds(line.bounds, character.bounds);
                }
                line
            })
            .collect::<Vec<_>>();
        lines.sort_by(|left, right| {
            right
                .origin_y
                .total_cmp(&left.origin_y)
                .then_with(|| left.origin_x.total_cmp(&right.origin_x))
                .then_with(|| {
                    left.writing_direction
                        .angle_degrees
                        .total_cmp(&right.writing_direction.angle_degrees)
                })
        });
        pages.push(lines);
    }
    Ok(pages)
}

fn text_characters_share_line(
    first: &TextCharacterContract,
    character: &TextCharacterContract,
    geometry: TextLineGeometry,
) -> bool {
    if !same_writing_axis(first.writing_direction, character.writing_direction) {
        return false;
    }
    match geometry {
        TextLineGeometry::Pdfium => {
            writing_cross_bounds_overlap(first.writing_direction, first.bounds, character.bounds)
        }
        TextLineGeometry::EmbeddedFont => same_text_line(
            first
                .writing_direction
                .cross_coordinate(first.origin_x, first.origin_y),
            first
                .writing_direction
                .cross_coordinate(character.origin_x, character.origin_y),
        ),
    }
}

fn same_writing_axis(left: TextWritingDirection, right: TextWritingDirection) -> bool {
    // PDFium reports an angle for each character, not source text-object
    // ownership. Angles are serialized to 0.01 degree; compare axes modulo
    // 180 degrees so reverse-direction runs still share a baseline.
    const EXTRACTED_ANGLE_TOLERANCE_DEGREES: f32 = 0.05;
    let delta = (left.angle_degrees - right.angle_degrees)
        .abs()
        .rem_euclid(180.0);
    delta.min(180.0 - delta) <= EXTRACTED_ANGLE_TOLERANCE_DEGREES
}

fn same_text_line(left_cross_origin: f32, right_cross_origin: f32) -> bool {
    // Layout engines such as Parley own line membership explicitly and expose
    // its baseline. A PDF extractor has no equivalent source ownership, so do
    // not infer it from overlapping font extents or relative font size.
    // PDFium origins are recorded at 0.01pt precision; 0.25pt covers numeric
    // extraction noise while keeping independently positioned text separate.
    (left_cross_origin - right_cross_origin).abs() <= 0.25
}

fn writing_cross_bounds_overlap(
    direction: TextWritingDirection,
    left: PdfBounds,
    right: PdfBounds,
) -> bool {
    let (left_start, left_end) = direction.projected_bounds(left, false);
    let (right_start, right_end) = direction.projected_bounds(right, false);
    left_end.min(right_end) > left_start.max(right_start)
}

fn union_pdf_bounds(left: PdfBounds, right: PdfBounds) -> PdfBounds {
    PdfBounds {
        left: left.left.min(right.left),
        bottom: left.bottom.min(right.bottom),
        right: left.right.max(right.right),
        top: left.top.max(right.top),
    }
}

fn normalized_page_text(summary: &PdfSummary) -> Vec<String> {
    normalized_page_text_from_parts(
        summary.page_count,
        summary
            .text_chars
            .iter()
            .map(|character| (character.page_index, character.text.as_str())),
        summary
            .text_segments
            .iter()
            .map(|segment| (segment.page_index, segment.text.as_str())),
    )
}

fn normalized_page_text_from_parts<'a>(
    page_count: usize,
    characters: impl IntoIterator<Item = (usize, &'a str)>,
    segments: impl IntoIterator<Item = (usize, &'a str)>,
) -> Vec<String> {
    let mut extracted_pages = vec![String::new(); page_count];
    let mut pages_with_characters = vec![false; page_count];
    for (page_index, text) in characters {
        if let Some(page) = extracted_pages.get_mut(page_index) {
            page.push_str(text);
            pages_with_characters[page_index] = true;
        }
    }
    for (page_index, text) in segments {
        if pages_with_characters.get(page_index).copied() == Some(false)
            && let Some(page) = extracted_pages.get_mut(page_index)
        {
            if !page.is_empty() {
                page.push(' ');
            }
            page.push_str(text);
        }
    }
    extracted_pages
        .into_iter()
        .map(|text| normalize_extracted_text(&text))
        .collect()
}

fn normalize_extracted_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        if is_extracted_whitespace(character) {
            pending_space = true;
            continue;
        }
        let previous_is_opening = normalized
            .chars()
            .next_back()
            .is_some_and(|previous| matches!(previous, '(' | '[' | '{'));
        let is_closing = matches!(
            character,
            '.' | ',' | ';' | ':' | '!' | '?' | '%' | ')' | ']' | '}'
        );
        if pending_space && !normalized.is_empty() && !previous_is_opening && !is_closing {
            normalized.push(' ');
        }
        normalized.push(character);
        pending_space = false;
    }
    normalized
}

fn unordered_extracted_text_content(text: &str) -> String {
    let mut characters = extracted_text_content_key(&normalize_extracted_text(text))
        .chars()
        .collect::<Vec<_>>();
    characters.sort_unstable();
    characters.into_iter().collect()
}

fn page_text_content_bags(pages: &[String]) -> Vec<String> {
    pages
        .iter()
        .map(|page| unordered_extracted_text_content(page))
        .collect()
}

fn extracted_text_content_key(text: &str) -> String {
    text.chars()
        .filter(|character| !is_extracted_whitespace(*character))
        .collect()
}

fn pdftotext_line_content_keys(text: &str) -> Vec<String> {
    text.lines()
        .map(normalize_extracted_text)
        .map(|line| extracted_text_content_key(&line))
        .filter(|line| !line.is_empty())
        .collect()
}

fn pdftotext_ordered_text_matches(
    candidate_pdf: &[u8],
    golden_pdf: &[u8],
    page_count: usize,
) -> DetailedResult<bool> {
    for page_index in 0..page_count {
        let candidate = pdftotext_page(candidate_pdf, page_index).map_err(|error| {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                .at(page_index, None)
        })?;
        let golden = pdftotext_page(golden_pdf, page_index).map_err(|error| {
            OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                .at(page_index, None)
        })?;
        if pdftotext_line_content_keys(&candidate) != pdftotext_line_content_keys(&golden) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn extracted_text_line_content_key(
    text: &str,
    accept_pdfium_bidi_mirroring: bool,
    accept_pdfium_ltr_mirroring: bool,
) -> String {
    extracted_text_content_key_with_bidi_mirroring(
        text,
        accept_pdfium_bidi_mirroring
            && (accept_pdfium_ltr_mirroring || contains_strong_rtl_character(text)),
    )
}

fn contains_strong_rtl_character(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(bidi_class(character), BidiClass::R | BidiClass::AL))
}

fn extracted_text_content_key_with_bidi_mirroring(
    text: &str,
    canonicalize_mirrors: bool,
) -> String {
    text.chars()
        .filter(|character| !is_extracted_whitespace(*character))
        .map(|character| {
            if canonicalize_mirrors {
                get_mirrored(character)
                    .map(|mirrored| character.min(mirrored))
                    .unwrap_or(character)
            } else {
                character
            }
        })
        .collect()
}

fn is_extracted_whitespace(character: char) -> bool {
    character.is_whitespace() || character == '\u{f020}'
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalizedGraphicsComparison {
    Pixels,
    SoftMaskStencil {
        solid_rgb: [u8; 3],
    },
    FlatPaletteSoftMask {
        colors: [[u8; 3]; 4],
        color_count: usize,
    },
    TranslucentSoftMask,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LocalizedGraphicRegion {
    bounds: PdfBounds,
    comparison: LocalizedGraphicsComparison,
}

fn localized_graphics_bounds(
    candidate: &PdfSummary,
    golden: &PdfSummary,
    page_index: usize,
    page_bounds: PdfBounds,
    raster_width_px: u32,
    raster_height_px: u32,
    tolerance: VisualTolerance,
) -> Result<Vec<LocalizedGraphicRegion>> {
    let page_area = page_bounds.width() * page_bounds.height();
    let mut bounds = Vec::new();
    let mut matched_candidate_images = vec![false; candidate.images.len()];
    for image in golden
        .images
        .iter()
        .filter(|image| image.page_index == page_index)
    {
        // A decoded-pixel, native-dimension, and placement match is stronger
        // evidence than comparing the same tiny image after two PDF producers
        // independently quantize it onto a three-pixel raster. Consume the
        // candidate objects as a multiset so one occurrence cannot stand in
        // for two. The page-wide raster contract remains active for adjacent
        // content, and every non-identical or vectorized image still takes the
        // localized visible-output path below.
        if let Some((candidate_index, _)) =
            candidate
                .images
                .iter()
                .enumerate()
                .find(|(candidate_index, candidate_image)| {
                    !matched_candidate_images[*candidate_index]
                        && exact_decoded_image_match(
                            candidate,
                            golden,
                            candidate_image,
                            image,
                            page_bounds,
                            raster_width_px,
                            raster_height_px,
                        )
                })
        {
            matched_candidate_images[candidate_index] = true;
            continue;
        }
        // Office's fixed-format writer decomposes thin dashed table borders
        // into repeated one-sample image tiles. They are a stroke
        // implementation detail, not semantic document images: vector PDF
        // producers legitimately preserve the same border as a dashed path.
        // Keep them under the page-wide visible-output contract, but do not
        // apply the semantic-image localized threshold to every individual
        // tile.
        if fixed_output_stroke_tile(image) {
            continue;
        }
        if std::env::var("OOXMLSDK_GOLDEN_TRACE_STAGES").is_ok_and(|value| value == "1") {
            let candidates = candidate
                .images
                .iter()
                .enumerate()
                .filter(|(_, candidate_image)| {
                    candidate_image.page_index == page_index
                        && candidate_image.width == image.width
                        && candidate_image.height == image.height
                        && image_placement_matches_at_comparison_grid(
                            candidate_image.bounds.as_deref(),
                            image.bounds.as_deref(),
                            page_bounds,
                            raster_width_px,
                            raster_height_px,
                        )
                })
                .map(|(candidate_index, candidate_image)| {
                    format!(
                        "#{candidate_index}:bounds={} samples={} semantic={} orientation={:?}",
                        candidate_image.bounds.as_deref().unwrap_or("none"),
                        decoded_image_samples_match(candidate_image, image),
                        semantic_soft_mask_diagnostic(candidate, golden, candidate_image, image,),
                        candidate_image.axis_aligned_orientation,
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            eprintln!(
                "office-golden unmatched-image page={page_index} golden-bounds={} orientation={:?} candidates=[{candidates}]",
                image.bounds.as_deref().unwrap_or("none"),
                image.axis_aligned_orientation,
            );
        }
        if let Some(rect) = image.bounds.as_deref() {
            let rect = parse_pdf_rect(rect).map_err(CalibrationError::OfficeGolden)?;
            let area = rect.width().max(0.0) * rect.height().max(0.0);
            // A complete omission of a larger image already exceeds the page
            // contract. Localize images small enough for that page-wide
            // denominator to hide. The comparison remains raster-based, so
            // the candidate may preserve the content as vectors or a form
            // instead of mirroring Office's PDF object decomposition.
            if page_area > 0.0
                && area / page_area <= tolerance.max_significant_pixel_fraction as f32
            {
                let comparison = soft_mask_uniform_color(golden, page_index, image).map_or_else(
                    || {
                        soft_mask_flat_palette(golden, image).map_or(
                            LocalizedGraphicsComparison::Pixels,
                            |(colors, color_count)| {
                                LocalizedGraphicsComparison::FlatPaletteSoftMask {
                                    colors,
                                    color_count,
                                }
                            },
                        )
                    },
                    |(solid_rgb, has_opaque_core)| {
                        if has_opaque_core {
                            LocalizedGraphicsComparison::SoftMaskStencil { solid_rgb }
                        } else {
                            LocalizedGraphicsComparison::TranslucentSoftMask
                        }
                    },
                );
                bounds.push(LocalizedGraphicRegion {
                    bounds: rect,
                    comparison,
                });
            }
        }
    }
    Ok(bounds)
}

fn fixed_output_stroke_tile(image: &ImageSummary) -> bool {
    const MAX_STROKE_THICKNESS_PT: f32 = 1.0;
    const MAX_STROKE_TILE_SAMPLES: u32 = 8;

    let Some(width) = image
        .width
        .as_deref()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(height) = image
        .height
        .as_deref()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(bounds) = image
        .bounds
        .as_deref()
        .and_then(|value| parse_pdf_rect(value).ok())
    else {
        return false;
    };
    let placed_width = bounds.width().abs();
    let placed_height = bounds.height().abs();

    (height == 1
        && width <= MAX_STROKE_TILE_SAMPLES
        && placed_height <= MAX_STROKE_THICKNESS_PT
        && placed_width >= placed_height)
        || (width == 1
            && height <= MAX_STROKE_TILE_SAMPLES
            && placed_width <= MAX_STROKE_THICKNESS_PT
            && placed_height >= placed_width)
}

fn exact_decoded_image_match(
    candidate_summary: &PdfSummary,
    golden_summary: &PdfSummary,
    candidate: &ImageSummary,
    golden: &ImageSummary,
    page_bounds: PdfBounds,
    raster_width_px: u32,
    raster_height_px: u32,
) -> bool {
    let same_identity = candidate.page_index == golden.page_index
        && candidate.width == golden.width
        && candidate.height == golden.height
        && image_placement_matches_at_comparison_grid(
            candidate.bounds.as_deref(),
            golden.bounds.as_deref(),
            page_bounds,
            raster_width_px,
            raster_height_px,
        );
    same_identity
        && (decoded_image_samples_match(candidate, golden)
            || semantic_soft_mask_images_match(
                candidate_summary,
                golden_summary,
                candidate,
                golden,
            ))
}

fn decoded_image_samples_match(candidate: &ImageSummary, golden: &ImageSummary) -> bool {
    if golden.decoded_pixel_sha256.is_some()
        && candidate.decoded_pixel_sha256 == golden.decoded_pixel_sha256
    {
        return true;
    }
    let Some((candidate_x, candidate_y)) = candidate.axis_aligned_orientation else {
        return false;
    };
    let Some((golden_x, golden_y)) = golden.axis_aligned_orientation else {
        return false;
    };
    candidate_x == golden_x
        && candidate_y == -golden_y
        && ((candidate.decoded_pixel_sha256.is_some()
            && candidate.decoded_pixel_sha256 == golden.decoded_vertical_flip_sha256)
            || (candidate.decoded_vertical_flip_sha256.is_some()
                && candidate.decoded_vertical_flip_sha256 == golden.decoded_pixel_sha256))
}

fn image_placement_matches_at_comparison_grid(
    candidate: Option<&str>,
    golden: Option<&str>,
    page_bounds: PdfBounds,
    raster_width_px: u32,
    raster_height_px: u32,
) -> bool {
    let Some(candidate_bounds) = candidate.and_then(|bounds| parse_pdf_rect(bounds).ok()) else {
        return false;
    };
    let Some(golden_bounds) = golden.and_then(|bounds| parse_pdf_rect(bounds).ok()) else {
        return false;
    };
    let candidate_pixels = pdf_bounds_to_pixel_rect(
        candidate_bounds,
        page_bounds,
        raster_width_px,
        raster_height_px,
    );
    let golden_pixels = pdf_bounds_to_pixel_rect(
        golden_bounds,
        page_bounds,
        raster_width_px,
        raster_height_px,
    );
    if candidate_pixels.is_some() && candidate_pixels == golden_pixels {
        return true;
    }
    if page_bounds.width() <= 0.0 || page_bounds.height() <= 0.0 {
        return false;
    }
    let x_scale = raster_width_px as f32 / page_bounds.width();
    let y_scale = raster_height_px as f32 / page_bounds.height();
    (candidate_bounds.left - golden_bounds.left).abs() * x_scale
        <= EXACT_IMAGE_PLACEMENT_RASTER_PIXELS
        && (candidate_bounds.right - golden_bounds.right).abs() * x_scale
            <= EXACT_IMAGE_PLACEMENT_RASTER_PIXELS
        && (candidate_bounds.bottom - golden_bounds.bottom).abs() * y_scale
            <= EXACT_IMAGE_PLACEMENT_RASTER_PIXELS
        && (candidate_bounds.top - golden_bounds.top).abs() * y_scale
            <= EXACT_IMAGE_PLACEMENT_RASTER_PIXELS
}

fn semantic_soft_mask_images_match(
    candidate_summary: &PdfSummary,
    golden_summary: &PdfSummary,
    candidate: &ImageSummary,
    golden: &ImageSummary,
) -> bool {
    let Some(candidate_image) = page_object_semantic_soft_mask_image(candidate_summary, candidate)
    else {
        return false;
    };
    let Some(golden_image) = page_object_semantic_soft_mask_image(golden_summary, golden) else {
        return false;
    };
    if candidate.axis_aligned_orientation == golden.axis_aligned_orientation {
        return semantic_soft_mask_samples_match(candidate_image, golden_image);
    }
    let Some((candidate_x, candidate_y)) = candidate.axis_aligned_orientation else {
        return false;
    };
    let Some((golden_x, golden_y)) = golden.axis_aligned_orientation else {
        return false;
    };
    let Some(width) = candidate
        .width
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return false;
    };
    candidate_x == golden_x
        && candidate_y == -golden_y
        && vertically_flipped_semantic_soft_mask_samples_match(candidate_image, golden_image, width)
}

fn semantic_soft_mask_samples_match(
    candidate: &crate::pdf_extract::SemanticSoftMaskImage,
    golden: &crate::pdf_extract::SemanticSoftMaskImage,
) -> bool {
    candidate.alpha == golden.alpha
        && candidate.black_matte_rgb.len() == golden.black_matte_rgb.len()
        && candidate
            .black_matte_rgb
            .iter()
            .zip(&golden.black_matte_rgb)
            // Both sides are 8-bit encodings of the PDF 1.5 section 7.5.4
            // preblending formula. A one-value difference is the complete
            // integer quantization interval, not a visible-output threshold.
            .all(|(candidate, golden)| candidate.abs_diff(*golden) <= 1)
}

fn vertically_flipped_semantic_soft_mask_samples_match(
    candidate: &crate::pdf_extract::SemanticSoftMaskImage,
    golden: &crate::pdf_extract::SemanticSoftMaskImage,
    width: usize,
) -> bool {
    if width == 0
        || candidate.alpha.len() != golden.alpha.len()
        || !candidate.alpha.len().is_multiple_of(width)
        || candidate.black_matte_rgb.len() != golden.black_matte_rgb.len()
        || candidate.black_matte_rgb.len() != candidate.alpha.len() * 3
    {
        return false;
    }
    candidate
        .alpha
        .chunks_exact(width)
        .rev()
        .flatten()
        .eq(golden.alpha.iter())
        && candidate
            .black_matte_rgb
            .chunks_exact(width * 3)
            .rev()
            .flatten()
            .zip(&golden.black_matte_rgb)
            .all(|(candidate, golden)| candidate.abs_diff(*golden) <= 1)
}

fn page_object_semantic_soft_mask_image<'a>(
    summary: &'a PdfSummary,
    image: &ImageSummary,
) -> Option<&'a crate::pdf_extract::SemanticSoftMaskImage> {
    let width = image.width.as_deref()?.parse::<u32>().ok()?;
    let height = image.height.as_deref()?.parse::<u32>().ok()?;
    let resource_name = page_object_image_resource_name(summary, image)?;
    let raw_page = summary
        .raw_pages
        .iter()
        .find(|page| page.page_index == image.page_index)?;
    raw_page
        .xobjects
        .iter()
        .find(|xobject| {
            xobject.name == *resource_name
                && xobject.subtype_name.as_deref() == Some("Image")
                && xobject.width_px == Some(width)
                && xobject.height_px == Some(height)
        })
        .and_then(|xobject| xobject.semantic_soft_mask_image.as_ref())
}

fn page_object_image_resource_name<'a>(
    summary: &'a PdfSummary,
    image: &ImageSummary,
) -> Option<&'a str> {
    let raw_page = summary
        .raw_pages
        .iter()
        .find(|page| page.page_index == image.page_index)?;
    let page_image_count = summary
        .images
        .iter()
        .filter(|candidate| candidate.page_index == image.page_index)
        .count();
    if raw_page.image_draw_names.len() != page_image_count {
        return None;
    }
    raw_page
        .image_draw_names
        .get(image.page_image_index)
        .map(String::as_str)
}

fn semantic_soft_mask_diagnostic(
    candidate_summary: &PdfSummary,
    golden_summary: &PdfSummary,
    candidate: &ImageSummary,
    golden: &ImageSummary,
) -> String {
    let candidate_name =
        page_object_image_resource_name(candidate_summary, candidate).unwrap_or("unassociated");
    let golden_name =
        page_object_image_resource_name(golden_summary, golden).unwrap_or("unassociated");
    let Some(candidate_image) = page_object_semantic_soft_mask_image(candidate_summary, candidate)
    else {
        return format!("{candidate_name}->{golden_name}:candidate-none");
    };
    let Some(golden_image) = page_object_semantic_soft_mask_image(golden_summary, golden) else {
        return format!("{candidate_name}->{golden_name}:golden-none");
    };
    let width = candidate
        .width
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default();
    format!(
        "{candidate_name}->{golden_name}:direct={},vflip={}",
        semantic_soft_mask_samples_match(candidate_image, golden_image),
        vertically_flipped_semantic_soft_mask_samples_match(candidate_image, golden_image, width,)
    )
}

fn soft_mask_uniform_color(
    golden: &PdfSummary,
    page_index: usize,
    image: &ImageSummary,
) -> Option<([u8; 3], bool)> {
    let width = image.width.as_deref()?.parse::<u32>().ok()?;
    let height = image.height.as_deref()?.parse::<u32>().ok()?;
    let raw_page = golden
        .raw_pages
        .iter()
        .find(|page| page.page_index == page_index)?;
    let images = raw_page
        .xobjects
        .iter()
        .filter(|xobject| {
            xobject.subtype_name.as_deref() == Some("Image")
                && xobject.width_px == Some(width)
                && xobject.height_px == Some(height)
        })
        .map(|xobject| {
            let solid_rgb = xobject
                .semantic_soft_mask_image
                .as_ref()
                .and_then(crate::pdf_extract::semantic_soft_mask_opaque_core_color)
                .or(xobject.solid_rgb)?;
            xobject
                .has_soft_mask
                .then_some((solid_rgb, xobject.soft_mask_has_opaque_core?))
        })
        .collect::<Option<Vec<_>>>()?;
    let first = *images.first()?;
    images
        .into_iter()
        .all(|image| image == first)
        .then_some(first)
}

fn soft_mask_flat_palette(
    summary: &PdfSummary,
    image: &ImageSummary,
) -> Option<([[u8; 3]; 4], usize)> {
    flat_soft_mask_palette(page_object_semantic_soft_mask_image(summary, image)?)
}

fn flat_soft_mask_palette(
    image: &crate::pdf_extract::SemanticSoftMaskImage,
) -> Option<([[u8; 3]; 4], usize)> {
    const MAX_COLORS: usize = 4;
    const MIN_CORE_SAMPLES: usize = 16;

    if image.black_matte_rgb.len() != image.alpha.len().checked_mul(3)? {
        return None;
    }
    let mut counts = BTreeMap::<[u8; 3], usize>::new();
    let mut opaque_samples = 0usize;
    for (&alpha, rgb) in image
        .alpha
        .iter()
        .zip(image.black_matte_rgb.chunks_exact(3))
    {
        if alpha != u8::MAX {
            continue;
        }
        opaque_samples += 1;
        *counts.entry([rgb[0], rgb[1], rgb[2]]).or_default() += 1;
    }
    let minimum_count = (opaque_samples / 100).max(MIN_CORE_SAMPLES);
    let mut frequent = counts
        .into_iter()
        .filter(|(color, count)| {
            *count >= minimum_count
                // White paint is indistinguishable from the fixed white page
                // behind this localized comparison.
                && color.iter().any(|component| u8::MAX - *component > 16)
        })
        .map(|(color, count)| (count, color))
        .collect::<Vec<_>>();
    frequent.sort_unstable_by(|left, right| right.cmp(left));
    if frequent.is_empty() || frequent.len() > MAX_COLORS {
        return None;
    }
    let mut colors = [[0; 3]; MAX_COLORS];
    let color_count = frequent.len();
    for (slot, (_, color)) in colors.iter_mut().zip(frequent) {
        *slot = color;
    }
    Some((colors, color_count))
}

fn visual_diff_metrics(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    significant_channel_delta: u8,
    text_masks: &[PdfBounds],
    localized_graphics: &[LocalizedGraphicRegion],
    page_bounds: PdfBounds,
) -> Result<VisualDiffMetrics> {
    if candidate.width_px != golden.width_px || candidate.height_px.abs_diff(golden.height_px) > 1 {
        return Err(CalibrationError::OfficeGolden(format!(
            "rendered page dimensions differ: candidate={}x{}, golden={}x{}",
            candidate.width_px, candidate.height_px, golden.width_px, golden.height_px
        )));
    }
    // The media-box contract has already accepted the vector page geometry.
    // At a fixed raster width, sub-point height differences can round to one
    // pixel; compare the shared rows instead of rejecting equivalent pages.
    let comparison_height_px = candidate.height_px.min(golden.height_px);
    let page_pixels = (candidate.width_px * comparison_height_px) as usize;
    let mut total_pixels = 0usize;
    let mut significant_pixels = 0usize;
    let mut absolute_delta_sum = 0u64;
    let mut max_channel_delta = 0u8;
    let text_mask_rows = text_mask_x_spans_by_row(
        candidate.width_px,
        golden.height_px,
        page_bounds,
        text_masks,
    );
    let row_bytes = candidate.width_px as usize * 4;
    for ((candidate_row, golden_row), text_mask_spans) in candidate
        .rgba
        .chunks_exact(row_bytes)
        .zip(golden.rgba.chunks_exact(row_bytes))
        .zip(&text_mask_rows)
        .take(comparison_height_px as usize)
    {
        let mut text_mask_index = 0usize;
        for (pixel_x, (candidate_pixel, golden_pixel)) in candidate_row
            .chunks_exact(4)
            .zip(golden_row.chunks_exact(4))
            .enumerate()
        {
            while text_mask_spans
                .get(text_mask_index)
                .is_some_and(|span| span.end <= pixel_x as u32)
            {
                text_mask_index += 1;
            }
            if text_mask_spans
                .get(text_mask_index)
                .is_some_and(|span| pixel_x as u32 >= span.start)
            {
                continue;
            }
            total_pixels += 1;
            let mut pixel_max = 0u8;
            for channel in 0..3 {
                let delta = candidate_pixel[channel].abs_diff(golden_pixel[channel]);
                absolute_delta_sum += u64::from(delta);
                pixel_max = pixel_max.max(delta);
                max_channel_delta = max_channel_delta.max(delta);
            }
            if pixel_max > significant_channel_delta {
                significant_pixels += 1;
            }
        }
    }
    let mut localized_graphics_regions = 0usize;
    let mut max_localized_graphics_significant_pixel_fraction = 0.0_f64;
    let mut max_localized_graphics_mean_absolute_channel_delta = 0.0_f64;
    let mut pixel_regions = localized_graphics
        .iter()
        .filter_map(|region| {
            pdf_bounds_to_pixel_rect(
                region.bounds,
                page_bounds,
                candidate.width_px,
                comparison_height_px,
            )
            .map(|rect| (rect, region.comparison))
        })
        .collect::<Vec<_>>();
    pixel_regions.sort_unstable_by_key(|(rect, comparison)| {
        (
            rect.top,
            rect.left,
            rect.height,
            rect.width,
            match comparison {
                LocalizedGraphicsComparison::Pixels => (0, [0, 0, 0]),
                LocalizedGraphicsComparison::SoftMaskStencil { solid_rgb } => (1, *solid_rgb),
                LocalizedGraphicsComparison::FlatPaletteSoftMask {
                    colors,
                    color_count,
                } => (
                    2,
                    colors[..*color_count].first().copied().unwrap_or_default(),
                ),
                LocalizedGraphicsComparison::TranslucentSoftMask => (3, [0, 0, 0]),
            },
        )
    });
    pixel_regions.dedup();
    for (rect, comparison) in pixel_regions {
        let localized = match comparison {
            LocalizedGraphicsComparison::Pixels => localized_visual_diff_metrics(
                candidate,
                golden,
                rect,
                significant_channel_delta,
                comparison_height_px,
            ),
            LocalizedGraphicsComparison::SoftMaskStencil { solid_rgb } => {
                localized_stencil_diff_metrics(
                    candidate,
                    golden,
                    rect,
                    solid_rgb,
                    significant_channel_delta,
                    comparison_height_px,
                    TEXT_EDGE_TOLERANCE_RASTER_PIXELS.ceil() as u32,
                )
            }
            LocalizedGraphicsComparison::FlatPaletteSoftMask {
                colors,
                color_count,
            } => localized_flat_palette_stencil_diff_metrics(
                candidate,
                golden,
                rect,
                &colors[..color_count],
                significant_channel_delta,
                comparison_height_px,
                TEXT_EDGE_TOLERANCE_RASTER_PIXELS.ceil() as u32,
            ),
            LocalizedGraphicsComparison::TranslucentSoftMask => {
                localized_visual_diff_metrics_with_text_masks(
                    candidate,
                    golden,
                    rect,
                    significant_channel_delta,
                    comparison_height_px,
                    &text_mask_rows,
                )
            }
        };
        if std::env::var("OOXMLSDK_GOLDEN_TRACE_STAGES").is_ok_and(|value| value == "1") {
            eprintln!(
                "office-golden localized-image rect=[{} {} {} {}] comparison={comparison:?} significant_fraction={} mean_delta={}",
                rect.left,
                rect.top,
                rect.width,
                rect.height,
                localized.significant_pixel_fraction,
                localized.mean_absolute_channel_delta,
            );
        }
        localized_graphics_regions += 1;
        max_localized_graphics_significant_pixel_fraction =
            max_localized_graphics_significant_pixel_fraction
                .max(localized.significant_pixel_fraction);
        max_localized_graphics_mean_absolute_channel_delta =
            max_localized_graphics_mean_absolute_channel_delta
                .max(localized.mean_absolute_channel_delta);
    }
    let masked_pixels = page_pixels - total_pixels;
    Ok(VisualDiffMetrics {
        page_pixels,
        total_pixels,
        masked_pixels,
        significant_pixels,
        significant_pixel_fraction: ratio(significant_pixels as u64, total_pixels as u64),
        mean_absolute_channel_delta: ratio(absolute_delta_sum, total_pixels as u64 * 3),
        max_channel_delta,
        localized_graphics_regions,
        max_localized_graphics_significant_pixel_fraction,
        max_localized_graphics_mean_absolute_channel_delta,
    })
}

#[derive(Clone, Copy, Debug)]
struct TextMaskXSpan {
    start: u32,
    end: u32,
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn pixel_center_span(
    first_center_coordinate: f32,
    last_center_coordinate: f32,
    extent_start: f32,
    extent_end: f32,
    pixels: u32,
) -> Option<TextMaskXSpan> {
    if pixels == 0 || extent_end <= extent_start {
        return None;
    }
    let scale = pixels as f32 / (extent_end - extent_start);
    let first = ((first_center_coordinate - extent_start) * scale - 0.5).ceil() as i64;
    let last = ((last_center_coordinate - extent_start) * scale - 0.5).floor() as i64;
    let start = first.max(0).min(i64::from(pixels)) as u32;
    let end = (last + 1).max(0).min(i64::from(pixels)) as u32;
    (start < end).then_some(TextMaskXSpan { start, end })
}

fn horizontal_pixel_center_span(
    left: f32,
    right: f32,
    page: PdfBounds,
    width_px: u32,
) -> Option<TextMaskXSpan> {
    pixel_center_span(left, right, page.left, page.right, width_px)
}

fn vertical_pixel_center_span(
    bottom: f32,
    top: f32,
    page: PdfBounds,
    height_px: u32,
) -> Option<TextMaskXSpan> {
    // Raster rows grow down from the PDF top edge.
    pixel_center_span(
        page.top - top,
        page.top - bottom,
        0.0,
        page.top - page.bottom,
        height_px,
    )
}

fn pdf_bounds_to_pixel_rect(
    bounds: PdfBounds,
    page: PdfBounds,
    width_px: u32,
    height_px: u32,
) -> Option<PixelRect> {
    let x = horizontal_pixel_center_span(bounds.left, bounds.right, page, width_px)?;
    let y = vertical_pixel_center_span(bounds.bottom, bounds.top, page, height_px)?;
    Some(PixelRect {
        left: x.start,
        top: y.start,
        width: x.end - x.start,
        height: y.end - y.start,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct LocalizedVisualDiffMetrics {
    significant_pixel_fraction: f64,
    mean_absolute_channel_delta: f64,
}

fn localized_visual_diff_metrics(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    rect: PixelRect,
    significant_channel_delta: u8,
    comparison_height_px: u32,
) -> LocalizedVisualDiffMetrics {
    localized_visual_diff_metrics_with_optional_text_masks(
        candidate,
        golden,
        rect,
        significant_channel_delta,
        comparison_height_px,
        None,
    )
}

fn localized_visual_diff_metrics_with_text_masks(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    rect: PixelRect,
    significant_channel_delta: u8,
    comparison_height_px: u32,
    text_mask_rows: &[Vec<TextMaskXSpan>],
) -> LocalizedVisualDiffMetrics {
    localized_visual_diff_metrics_with_optional_text_masks(
        candidate,
        golden,
        rect,
        significant_channel_delta,
        comparison_height_px,
        Some(text_mask_rows),
    )
}

fn localized_visual_diff_metrics_with_optional_text_masks(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    rect: PixelRect,
    significant_channel_delta: u8,
    comparison_height_px: u32,
    text_mask_rows: Option<&[Vec<TextMaskXSpan>]>,
) -> LocalizedVisualDiffMetrics {
    // Image XObject bounds are independently rounded to each producer's
    // output grid. Exclude exactly one fixed-raster sample at the perimeter
    // when the region has a real interior. The page comparison still observes
    // that perimeter, while the localized omission guard compares the stable
    // image content rather than a subpixel resampling edge.
    let rect = inset_pixel_rect(rect, 1).unwrap_or(rect);
    // Office frequently resamples a source image while writing PDF. For
    // tdf170095 the Office and candidate placements differ by less than
    // 0.55 pt, which becomes one raster-pixel phase at the fixed golden
    // width even though the visible image content is unchanged. Select one
    // global subpixel alignment for the complete localized region; do not
    // search independently per block, which could hide missing or displaced
    // image content.
    (-1..=1)
        .flat_map(|offset_y| (-1..=1).map(move |offset_x| (offset_x, offset_y)))
        .filter(|&(offset_x, offset_y)| {
            shifted_pixel_rect_fits(
                rect,
                offset_x,
                offset_y,
                candidate.width_px,
                comparison_height_px,
            )
        })
        .map(|(offset_x, offset_y)| {
            localized_visual_diff_metrics_at_offset(
                candidate,
                golden,
                rect,
                significant_channel_delta,
                comparison_height_px,
                PixelOffset {
                    x: offset_x,
                    y: offset_y,
                },
                text_mask_rows,
            )
        })
        .min_by(|left, right| {
            left.significant_pixel_fraction
                .total_cmp(&right.significant_pixel_fraction)
                .then_with(|| {
                    left.mean_absolute_channel_delta
                        .total_cmp(&right.mean_absolute_channel_delta)
                })
        })
        .unwrap_or_default()
}

fn inset_pixel_rect(rect: PixelRect, amount: u32) -> Option<PixelRect> {
    let inset = amount.checked_mul(2)?;
    Some(PixelRect {
        left: rect.left.checked_add(amount)?,
        top: rect.top.checked_add(amount)?,
        width: rect.width.checked_sub(inset)?,
        height: rect.height.checked_sub(inset)?,
    })
    .filter(|rect| rect.width > 0 && rect.height > 0)
}

fn shifted_pixel_rect_fits(
    rect: PixelRect,
    offset_x: i32,
    offset_y: i32,
    width_px: u32,
    height_px: u32,
) -> bool {
    let left = i64::from(rect.left) + i64::from(offset_x);
    let top = i64::from(rect.top) + i64::from(offset_y);
    left >= 0
        && top >= 0
        && left + i64::from(rect.width) <= i64::from(width_px)
        && top + i64::from(rect.height) <= i64::from(height_px)
}

#[derive(Clone, Copy)]
struct PixelOffset {
    x: i32,
    y: i32,
}

fn localized_visual_diff_metrics_at_offset(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    rect: PixelRect,
    significant_channel_delta: u8,
    comparison_height_px: u32,
    candidate_offset: PixelOffset,
    text_mask_rows: Option<&[Vec<TextMaskXSpan>]>,
) -> LocalizedVisualDiffMetrics {
    const BLOCK_SIZE: u32 = 4;
    let right = (rect.left + rect.width).min(candidate.width_px);
    let bottom = (rect.top + rect.height).min(comparison_height_px);
    let mut significant = 0u64;
    let mut absolute_delta_sum = 0u64;
    let mut samples = 0u64;
    for block_top in (rect.top..bottom).step_by(BLOCK_SIZE as usize) {
        for block_left in (rect.left..right).step_by(BLOCK_SIZE as usize) {
            let block_right = (block_left + BLOCK_SIZE).min(right);
            let block_bottom = (block_top + BLOCK_SIZE).min(bottom);
            let mut candidate_sum = [0u32; 3];
            let mut golden_sum = [0u32; 3];
            let mut pixels = 0u32;
            for y in block_top..block_bottom {
                for x in block_left..block_right {
                    if text_mask_rows.is_some_and(|rows| {
                        rows.get(y as usize)
                            .is_some_and(|spans| pixel_is_in_text_mask(spans, x))
                    }) {
                        continue;
                    }
                    let candidate_pixel = pixel_rgb(
                        candidate,
                        (i64::from(x) + i64::from(candidate_offset.x)) as u32,
                        (i64::from(y) + i64::from(candidate_offset.y)) as u32,
                    );
                    let golden_pixel = pixel_rgb(golden, x, y);
                    for channel in 0..3 {
                        candidate_sum[channel] += u32::from(candidate_pixel[channel]);
                        golden_sum[channel] += u32::from(golden_pixel[channel]);
                    }
                    pixels += 1;
                }
            }
            if pixels == 0 {
                continue;
            }
            let deltas = [
                (candidate_sum[0] / pixels).abs_diff(golden_sum[0] / pixels) as u8,
                (candidate_sum[1] / pixels).abs_diff(golden_sum[1] / pixels) as u8,
                (candidate_sum[2] / pixels).abs_diff(golden_sum[2] / pixels) as u8,
            ];
            significant +=
                u64::from(deltas.into_iter().max().unwrap_or_default() > significant_channel_delta);
            absolute_delta_sum += deltas.into_iter().map(u64::from).sum::<u64>();
            samples += 1;
        }
    }
    LocalizedVisualDiffMetrics {
        significant_pixel_fraction: ratio(significant, samples),
        mean_absolute_channel_delta: ratio(absolute_delta_sum, samples * 3),
    }
}

fn pixel_is_in_text_mask(spans: &[TextMaskXSpan], x: u32) -> bool {
    spans
        .iter()
        .take_while(|span| span.start <= x)
        .any(|span| x < span.end)
}

fn localized_stencil_diff_metrics(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    rect: PixelRect,
    solid_rgb: [u8; 3],
    significant_channel_delta: u8,
    comparison_height_px: u32,
    radius: u32,
) -> LocalizedVisualDiffMetrics {
    let right = (rect.left + rect.width).min(candidate.width_px);
    let bottom = (rect.top + rect.height).min(comparison_height_px);
    let width = right.saturating_sub(rect.left);
    let height = bottom.saturating_sub(rect.top);
    if width == 0 || height == 0 {
        return LocalizedVisualDiffMetrics::default();
    }

    let candidate_mask = stencil_core_mask(
        candidate,
        rect.left,
        rect.top,
        width,
        height,
        solid_rgb,
        significant_channel_delta,
    );
    let golden_mask = stencil_core_mask(
        golden,
        rect.left,
        rect.top,
        width,
        height,
        solid_rgb,
        significant_channel_delta,
    );
    let candidate_pixels = candidate_mask.iter().filter(|&&pixel| pixel).count() as u64;
    let golden_pixels = golden_mask.iter().filter(|&&pixel| pixel).count() as u64;
    let stencil_pixels = candidate_pixels + golden_pixels;
    if stencil_pixels == 0 {
        // A fully transparent stencil has no visible contract of its own; the
        // surrounding page comparison still covers the rendered background.
        return LocalizedVisualDiffMetrics::default();
    }

    let unmatched = unmatched_stencil_pixels(&candidate_mask, &golden_mask, width, height, radius)
        + unmatched_stencil_pixels(&golden_mask, &candidate_mask, width, height, radius);
    LocalizedVisualDiffMetrics {
        significant_pixel_fraction: ratio(unmatched, stencil_pixels),
        // Paint/color differences produce no matching stencil core and are
        // represented by the significant fraction. Do not mix that
        // foreground-normalized value with a per-channel page mean.
        mean_absolute_channel_delta: 0.0,
    }
}

fn localized_flat_palette_stencil_diff_metrics(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    rect: PixelRect,
    colors: &[[u8; 3]],
    significant_channel_delta: u8,
    comparison_height_px: u32,
    radius: u32,
) -> LocalizedVisualDiffMetrics {
    colors
        .iter()
        .map(|&solid_rgb| {
            localized_stencil_diff_metrics(
                candidate,
                golden,
                rect,
                solid_rgb,
                significant_channel_delta,
                comparison_height_px,
                radius,
            )
        })
        .max_by(|left, right| {
            left.significant_pixel_fraction
                .total_cmp(&right.significant_pixel_fraction)
                .then_with(|| {
                    left.mean_absolute_channel_delta
                        .total_cmp(&right.mean_absolute_channel_delta)
                })
        })
        .unwrap_or_default()
}

fn stencil_core_mask(
    image: &RenderedPageImage,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    solid_rgb: [u8; 3],
    channel_tolerance: u8,
) -> Vec<bool> {
    let mut mask = Vec::with_capacity((width * height) as usize);
    for y in top..top + height {
        for x in left..left + width {
            let pixel = pixel_rgb(image, x, y);
            mask.push(pixel_matches_stencil_paint(
                pixel,
                solid_rgb,
                channel_tolerance,
            ));
        }
    }
    mask
}

fn pixel_matches_stencil_paint(actual: [u8; 3], solid_rgb: [u8; 3], channel_tolerance: u8) -> bool {
    const WHITE: u8 = u8::MAX;

    // The fixed-width page raster can resample a one-device-pixel Office
    // stencil until no fully opaque sample survives. Recognize the exact PDF
    // alpha-compositing line between the stencil paint and the white page
    // backdrop, instead of requiring the straight paint color itself.
    let Some(reference_channel) = solid_rgb
        .iter()
        .enumerate()
        .max_by_key(|(_, component)| WHITE - **component)
        .filter(|(_, component)| **component != WHITE)
        .map(|(channel, _)| channel)
    else {
        return false;
    };
    let reference_contrast = u16::from(WHITE - solid_rgb[reference_channel]);
    let reference_delta = u16::from(WHITE - actual[reference_channel]);
    if reference_delta <= u16::from(channel_tolerance) {
        return false;
    }
    let alpha = ((reference_delta * u16::from(WHITE) + reference_contrast / 2)
        / reference_contrast)
        .min(u16::from(WHITE));
    actual.into_iter().zip(solid_rgb).all(|(actual, solid)| {
        let contrast = u16::from(WHITE - solid);
        let expected = WHITE - ((contrast * alpha + u16::from(WHITE) / 2) / u16::from(WHITE)) as u8;
        actual.abs_diff(expected) <= channel_tolerance
    })
}

fn unmatched_stencil_pixels(
    source: &[bool],
    target: &[bool],
    width: u32,
    height: u32,
    radius: u32,
) -> u64 {
    let stride = width as usize + 1;
    let mut integral = vec![0u32; stride * (height as usize + 1)];
    for y in 0..height as usize {
        let mut row_sum = 0u32;
        for x in 0..width as usize {
            row_sum += u32::from(target[y * width as usize + x]);
            integral[(y + 1) * stride + x + 1] = integral[y * stride + x + 1] + row_sum;
        }
    }

    let mut unmatched = 0u64;
    for y in 0..height {
        for x in 0..width {
            if !source[(y * width + x) as usize] {
                continue;
            }
            let left = x.saturating_sub(radius) as usize;
            let top = y.saturating_sub(radius) as usize;
            let right = (x + radius + 1).min(width) as usize;
            let bottom = (y + radius + 1).min(height) as usize;
            let target_pixels = integral[bottom * stride + right] + integral[top * stride + left]
                - integral[top * stride + right]
                - integral[bottom * stride + left];
            unmatched += u64::from(target_pixels == 0);
        }
    }
    unmatched
}

#[inline]
fn pixel_rgb(image: &RenderedPageImage, x: u32, y: u32) -> [u8; 3] {
    let offset = ((y * image.width_px + x) * 4) as usize;
    [
        image.rgba[offset],
        image.rgba[offset + 1],
        image.rgba[offset + 2],
    ]
}

fn text_mask_x_spans_by_row(
    width_px: u32,
    height_px: u32,
    page: PdfBounds,
    masks: &[PdfBounds],
) -> Vec<Vec<TextMaskXSpan>> {
    let mut rows = vec![Vec::new(); height_px as usize];
    let page_width = page.right - page.left;
    let page_height = page.top - page.bottom;
    if page_width <= 0.0 || page_height <= 0.0 {
        return rows;
    }
    for mask in masks {
        let Some(x) = horizontal_pixel_center_span(
            mask.left - TEXT_MASK_PADDING_PT,
            mask.right + TEXT_MASK_PADDING_PT,
            page,
            width_px,
        ) else {
            continue;
        };
        let Some(y) = vertical_pixel_center_span(
            mask.bottom - TEXT_MASK_PADDING_PT,
            mask.top + TEXT_MASK_PADDING_PT,
            page,
            height_px,
        ) else {
            continue;
        };
        for row in &mut rows[y.start as usize..y.end as usize] {
            row.push(x);
        }
    }
    for row in &mut rows {
        row.sort_unstable_by_key(|span| span.start);
        if row.len() < 2 {
            continue;
        }
        let mut merged = 0usize;
        for index in 1..row.len() {
            if row[index].start <= row[merged].end {
                row[merged].end = row[merged].end.max(row[index].end);
            } else {
                merged += 1;
                row[merged] = row[index];
            }
        }
        row.truncate(merged + 1);
    }
    rows
}

fn write_failure_page_artifacts(
    artifact_dir: &Path,
    page_index: usize,
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
) -> Result<()> {
    write_png(
        &artifact_dir.join(format!("page-{page_index}-candidate.png")),
        candidate.width_px,
        candidate.height_px,
        &candidate.rgba,
    )?;
    write_png(
        &artifact_dir.join(format!("page-{page_index}-golden.png")),
        golden.width_px,
        golden.height_px,
        &golden.rgba,
    )?;
    let comparison_height_px = candidate.height_px.min(golden.height_px);
    let diff = candidate
        .rgba
        .chunks_exact(4)
        .zip(golden.rgba.chunks_exact(4))
        .flat_map(|(candidate_pixel, golden_pixel)| {
            let delta = (0..3)
                .map(|channel| candidate_pixel[channel].abs_diff(golden_pixel[channel]))
                .max()
                .unwrap_or(0);
            [delta, 0, 0, 255]
        })
        .collect::<Vec<_>>();
    write_png(
        &artifact_dir.join(format!("page-{page_index}-diff.png")),
        candidate.width_px,
        comparison_height_px,
        &diff,
    )?;
    Ok(())
}

fn format_page_ranges(page_indices: &[usize]) -> String {
    let Some((&first, rest)) = page_indices.split_first() else {
        return "none".to_string();
    };
    let mut ranges = Vec::new();
    let mut start = first;
    let mut end = first;
    for &page_index in rest {
        if page_index == end + 1 {
            end = page_index;
            continue;
        }
        ranges.push(format_page_range(start, end));
        start = page_index;
        end = page_index;
    }
    ranges.push(format_page_range(start, end));
    ranges.join(",")
}

fn format_page_range(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

enum CandidateDiagnosticsState {
    Uncollected,
    Collected(PdfConversionDiagnostics),
    Failed(String),
}

fn validate_candidate_font_contract(candidate_pdf: &[u8], audit: &PdfFontAudit) -> Result<()> {
    if !audit.issues.is_empty() {
        let samples = audit
            .issues
            .iter()
            .take(8)
            .map(|issue| {
                format!(
                    "{}@page:{}/run:{}/portion:{:?}/glyph:{:?}: {}",
                    issue.kind.as_str(),
                    issue.page_index,
                    issue.text_run_index,
                    issue.portion_index,
                    issue.glyph_index,
                    issue.detail
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(CalibrationError::OfficeGolden(format!(
            "candidate font audit found {} issue(s): {samples}",
            audit.issues.len()
        )));
    }

    let structure = pdf_font_structure(candidate_pdf).map_err(|error| {
        CalibrationError::OfficeGolden(format!(
            "candidate PDF font structure could not be read: {error}"
        ))
    })?;
    if structure.actual_text_span_count < audit.actual_text_cluster_count {
        return Err(CalibrationError::OfficeGolden(format!(
            "candidate has {} multi-glyph text cluster(s) but only {} ActualText span(s)",
            audit.actual_text_cluster_count, structure.actual_text_span_count
        )));
    }
    let fonts = structure
        .pages
        .iter()
        .flat_map(|page| &page.fonts)
        .collect::<Vec<_>>();

    let mut issues = Vec::new();
    for font in fonts {
        if font.subtype.as_deref() == Some("Type0") && font.descendant_subtype.is_none() {
            issues.push(format!(
                "{} Type0 font has no descendant font",
                font.resource_path
            ));
        }
        if font.subtype.as_deref() == Some("Type0")
            && let (Some(base), Some(descendant)) = (&font.base_font, &font.descendant_base_font)
            && !type0_base_font_matches_descendant(
                base,
                descendant,
                font.descendant_subtype.as_deref(),
                font.encoding.as_deref(),
            )
        {
            issues.push(format!(
                "{} Type0 BaseFont is incompatible with its descendant: {base:?}, descendant={descendant:?}, subtype={:?}, encoding={:?}",
                font.resource_path, font.descendant_subtype, font.encoding
            ));
        }
        // Krilla represents color/bitmap glyphs through Type3 char procedures;
        // other font resources are expected to carry an embedded font stream.
        if font.subtype.as_deref() != Some("Type3") && font.embedded_font_kind.is_none() {
            issues.push(format!(
                "{} {:?} font has no embedded font stream",
                font.resource_path, font.subtype
            ));
        }
        if font.subtype.as_deref() != Some("Type3") {
            for (name, missing) in [
                ("Flags", font.descriptor_flags.is_none()),
                ("FontBBox", font.font_bounds.is_none()),
                ("Ascent", font.ascent.is_none()),
                ("Descent", font.descent.is_none()),
            ] {
                if missing {
                    issues.push(format!(
                        "{} {:?} font descriptor has no {name}",
                        font.resource_path, font.subtype
                    ));
                }
            }
        }
        if !font.has_to_unicode {
            issues.push(format!(
                "{} {:?} font has no ToUnicode map",
                font.resource_path, font.subtype
            ));
        } else if let Some(error) = &font.to_unicode_error {
            issues.push(format!(
                "{} {:?} font has invalid ToUnicode map: {error}",
                font.resource_path, font.subtype
            ));
        } else if font
            .to_unicode_mapping_count
            .is_none_or(|mapping_count| mapping_count == 0)
        {
            issues.push(format!(
                "{} {:?} font has an empty ToUnicode map",
                font.resource_path, font.subtype
            ));
        }
        if issues.len() >= 16 {
            break;
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(CalibrationError::OfficeGolden(format!(
            "candidate PDF font integrity failed: {}",
            issues.join("; ")
        )))
    }
}

fn type0_base_font_matches_descendant(
    base_font: &str,
    descendant_base_font: &str,
    descendant_subtype: Option<&str>,
    encoding: Option<&str>,
) -> bool {
    match descendant_subtype {
        // ISO 32000-1:2008, 9.7.6.1: a CIDFontType0 root name is the
        // descendant BaseFont followed by a hyphen and the CMap name.
        Some("CIDFontType0") => encoding
            .is_some_and(|encoding| base_font == format!("{descendant_base_font}-{encoding}")),
        // The same clause requires a CIDFontType2 root name to equal the
        // descendant BaseFont name.
        Some("CIDFontType2") | None => base_font == descendant_base_font,
        Some(_) => false,
    }
}

fn collect_candidate_diagnostics(
    source_path: &Path,
    options: &ooxmlsdk_pdf::PdfOptions,
    state: &mut CandidateDiagnosticsState,
) {
    if !matches!(state, CandidateDiagnosticsState::Uncollected) {
        return;
    }
    *state = match crate::render::render_fixture_pdf_with_diagnostics(source_path, options.clone())
    {
        Ok(output) => CandidateDiagnosticsState::Collected(output.diagnostics),
        Err(error) => CandidateDiagnosticsState::Failed(error.to_string()),
    };
}

fn write_candidate_artifact(
    case_id: &str,
    candidate_pdf: &[u8],
    golden_pdf: &[u8],
    font_audit: &PdfFontAudit,
    diagnostics: &CandidateDiagnosticsState,
    diagnostic_pages: &[usize],
) -> Result<PathBuf> {
    let artifact_dir = workspace_root().join("target/office-golden").join(case_id);
    fs::create_dir_all(&artifact_dir)?;
    fs::write(artifact_dir.join("candidate.pdf"), candidate_pdf)?;
    write_pretty_json(
        &artifact_dir.join("candidate-font-audit.json"),
        &font_audit_json(font_audit),
    )?;
    let font_structure = json!({
        "schema_version": 1,
        "candidate": font_structure_json(candidate_pdf),
        "golden": font_structure_json(golden_pdf),
    });
    write_pretty_json(
        &artifact_dir.join("pdf-font-structure.json"),
        &font_structure,
    )?;
    if let CandidateDiagnosticsState::Collected(diagnostics) = diagnostics {
        let fonts = diagnostics
            .fonts
            .iter()
            .enumerate()
            .map(|(font_index, font)| {
                json!({
                    "font_index": font_index,
                    "font_id": font.font_id,
                    "face_index": font.face_index,
                    "data_len": font.data_len,
                    "parse_error": font.parse_error,
                    "checksum_adjustment": font.checksum_adjustment,
                    "postscript_name": font.postscript_name,
                    "family_names": font.family_names,
                    "style_name": font.style_name,
                    "units_per_em": font.units_per_em,
                    "glyph_count": font.glyph_count,
                    "ascender_em": font.ascender_em,
                    "descender_em": font.descender_em,
                    "cap_height_em": font.cap_height_em,
                    "global_bounds_em": glyph_bounds_json(font.global_bounds_em),
                    "monospaced": font.monospaced,
                })
            })
            .collect::<Vec<_>>();
        write_pretty_json(
            &artifact_dir.join("candidate-font-selection.json"),
            &json!({ "schema_version": 1, "status": "ok", "fonts": fonts }),
        )?;
    } else if let CandidateDiagnosticsState::Failed(error) = diagnostics {
        write_pretty_json(
            &artifact_dir.join("candidate-font-selection.json"),
            &json!({ "schema_version": 1, "status": "error", "error": error }),
        )?;
    }
    write_candidate_glyph_pages(&artifact_dir, diagnostics, diagnostic_pages)?;
    Ok(artifact_dir)
}

fn font_audit_json(audit: &PdfFontAudit) -> Value {
    let issues = audit
        .issues
        .iter()
        .map(|issue| {
            json!({
                "kind": issue.kind.as_str(),
                "page_index": issue.page_index,
                "text_run_index": issue.text_run_index,
                "portion_index": issue.portion_index,
                "glyph_run_index": issue.glyph_run_index,
                "glyph_index": issue.glyph_index,
                "detail": issue.detail,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "status": if issues.is_empty() { "ok" } else { "error" },
        "font_count": audit.fonts.len(),
        "text_portion_count": audit.text_portion_count,
        "painted_text_portion_count": audit.painted_text_portion_count,
        "explicit_glyph_portion_count": audit.explicit_glyph_portion_count,
        "glyph_run_count": audit.glyph_run_count,
        "glyph_count": audit.glyph_count,
        "explicit_symbol_notdef_glyph_count": audit.explicit_symbol_notdef_glyph_count,
        "actual_text_cluster_count": audit.actual_text_cluster_count,
        "issues": issues,
    })
}

fn font_structure_json(pdf: &[u8]) -> Value {
    match pdf_font_structure(pdf) {
        Ok(summary) => json!({ "status": "ok", "summary": summary }),
        Err(error) => json!({ "status": "error", "error": error }),
    }
}

fn write_candidate_glyph_pages(
    artifact_dir: &Path,
    diagnostics: &CandidateDiagnosticsState,
    diagnostic_pages: &[usize],
) -> Result<()> {
    let CandidateDiagnosticsState::Collected(diagnostics) = diagnostics else {
        return Ok(());
    };
    let selected = diagnostic_pages
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let trace_dir = artifact_dir.join("candidate-glyph-trace");
    fs::create_dir_all(&trace_dir)?;
    let mut written_pages = Vec::new();
    for page_index in selected {
        let Some(page) = diagnostics.pages.get(page_index) else {
            continue;
        };
        written_pages.push(page_index);
        write_pretty_json(
            &trace_dir.join(format!("page-{page_index}.json")),
            &glyph_page_json(page),
        )?;
    }
    write_pretty_json(
        &trace_dir.join("index.json"),
        &json!({
            "schema_version": 1,
            "page_count": diagnostics.pages.len(),
            "written_pages": written_pages,
        }),
    )
}

fn glyph_page_json(page: &ooxmlsdk_pdf::PdfPageDiagnostics) -> Value {
    let text_runs = page
        .text_runs
        .iter()
        .map(|text| {
            let portions = text
                .portions
                .iter()
                .map(|portion| {
                    let glyph_runs = portion
                        .glyph_runs
                        .iter()
                        .map(|run| {
                            let glyphs = run
                                .glyphs
                                .iter()
                                .map(|glyph| {
                                    json!({
                                        "glyph_id": glyph.glyph_id,
                                        "text_range": [glyph.text_range_start, glyph.text_range_end],
                                        "x_advance_em": glyph.x_advance_em,
                                        "x_offset_em": glyph.x_offset_em,
                                        "y_offset_em": glyph.y_offset_em,
                                        "y_advance_em": glyph.y_advance_em,
                                        "bounds_em": glyph.bounds_em.map(glyph_bounds_json),
                                    })
                                })
                                .collect::<Vec<_>>();
                            json!({
                                "font_index": run.font_index,
                                "x_offset_pt": run.x_offset_pt,
                                "synthetic_bold": run.synthetic_bold,
                                "synthetic_italic": run.synthetic_italic,
                                "glyphs": glyphs,
                            })
                        })
                        .collect::<Vec<_>>();
                    json!({
                        "kind": format!("{:?}", portion.kind).to_ascii_lowercase(),
                        "text_range": [portion.text_range_start, portion.text_range_end],
                        "x_pt": portion.x_pt,
                        "baseline_y_pt": portion.baseline_y_pt,
                        "width_pt": portion.width_pt,
                        "has_explicit_glyphs": portion.has_explicit_glyphs,
                        "glyph_runs": glyph_runs,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "text": text.text,
                "source_frame_index": text.source_frame_index,
                "source_line_index": text.source_line_index,
                "source_path": text.source_path,
                "x_pt": text.x_pt,
                "y_pt": text.y_pt,
                "baseline_y_pt": text.baseline_y_pt,
                "line_height_pt": text.line_height_pt,
                "width_pt": text.width_pt,
                "font_size_pt": text.font_size_pt,
                "character_spacing_pt": text.character_spacing_pt,
                "baseline_shift_pt": text.baseline_shift_pt,
                "requested_font_family": text.requested_font_family,
                "requested_east_asia_font_family": text.requested_east_asia_font_family,
                "requested_complex_font_family": text.requested_complex_font_family,
                "bold": text.bold,
                "italic": text.italic,
                "small_caps": text.small_caps,
                "portions": portions,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "page_index": page.page_index,
        "width_pt": page.width_pt,
        "height_pt": page.height_pt,
        "text_runs": text_runs,
    })
}

fn glyph_bounds_json(bounds: ooxmlsdk_pdf::PdfGlyphBoundsDiagnostics) -> Value {
    json!({
        "x_min_em": bounds.x_min_em,
        "y_min_em": bounds.y_min_em,
        "x_max_em": bounds.x_max_em,
        "y_max_em": bounds.y_max_em,
    })
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    let data = serde_json::to_vec_pretty(value).map_err(|error| {
        CalibrationError::OfficeGolden(format!(
            "could not serialize diagnostic artifact {}: {error}",
            path.display()
        ))
    })?;
    fs::write(path, data)?;
    Ok(())
}

fn write_png(path: &Path, width_px: u32, height_px: u32, rgba: &[u8]) -> Result<()> {
    image::save_buffer_with_format(
        path,
        rgba,
        width_px,
        height_px,
        image::ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|error| {
        CalibrationError::OfficeGolden(format!("could not write {}: {error}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use lopdf::{Document, Object, Stream, dictionary};
    use ooxmlsdk_pdf::PdfFontAudit;

    use crate::pdf_extract::ImageSummary;

    use super::{
        LocalizedGraphicRegion, LocalizedGraphicsComparison, PdfBounds, PixelRect,
        RenderedPageImage, TEXT_MASK_PADDING_PT, TextCharacterContract, TextLineGeometry,
        TextWritingDirection, VisualTolerance, canonical_pdf_base_font_name,
        extracted_text_content_key, extracted_text_line_content_key, fixed_output_stroke_tile,
        flat_soft_mask_palette, format_page_ranges, image_placement_matches_at_comparison_grid,
        localized_flat_palette_stencil_diff_metrics, localized_visual_diff_metrics,
        normalize_extracted_text, normalized_page_text_from_parts,
        parse_utc_datetime_in_reference_time_zone, pdf_style_colors_equivalent,
        pdftotext_line_content_keys, pixel_matches_stencil_paint, same_text_line,
        same_writing_axis, semantic_soft_mask_samples_match, text_characters_share_line,
        text_edge_tolerance_pt, text_mask_x_spans_by_row, type0_base_font_matches_descendant,
        unordered_extracted_text_content, validate_candidate_font_contract, visual_diff_metrics,
    };

    #[test]
    fn office_conversion_timestamp_uses_the_recorded_reference_time_zone() {
        assert_eq!(
            parse_utc_datetime_in_reference_time_zone(
                "2026-07-12T12:19:54.5613645Z",
                "China Standard Time",
                "zh-CN",
            )
            .unwrap(),
            ooxmlsdk_pdf::FieldUpdateDateTime {
                year: 2026,
                month: 7,
                day: 12,
                hour: 20,
                minute: 19,
                second: 54,
            }
        );
        assert_eq!(
            parse_utc_datetime_in_reference_time_zone(
                "2026-12-31T20:30:00Z",
                "China Standard Time",
                "zh-CN",
            )
            .unwrap(),
            ooxmlsdk_pdf::FieldUpdateDateTime {
                year: 2027,
                month: 1,
                day: 1,
                hour: 4,
                minute: 30,
                second: 0,
            }
        );
        assert_eq!(
            parse_utc_datetime_in_reference_time_zone(
                "2026-01-15T12:00:00Z",
                "Eastern Standard Time",
                "en-US",
            )
            .unwrap()
            .hour,
            7
        );
        assert_eq!(
            parse_utc_datetime_in_reference_time_zone(
                "2026-07-15T12:00:00Z",
                "Eastern Standard Time",
                "en-US",
            )
            .unwrap()
            .hour,
            8
        );
    }

    fn blank_pdf() -> Vec<u8> {
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Contents" => content_id,
            "Resources" => dictionary! {},
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn off_page_paint_intent_without_pdf_fonts_is_not_a_font_integrity_error() {
        let audit = PdfFontAudit {
            text_portion_count: 1,
            painted_text_portion_count: 1,
            ..PdfFontAudit::default()
        };

        validate_candidate_font_contract(&blank_pdf(), &audit).unwrap();
    }

    #[test]
    fn type0_base_font_names_follow_cidfont_subtype_rules() {
        assert!(type0_base_font_matches_descendant(
            "NotoSansCJKjp-Regular-Identity-H",
            "NotoSansCJKjp-Regular",
            Some("CIDFontType0"),
            Some("Identity-H")
        ));
        assert!(type0_base_font_matches_descendant(
            "SimSun",
            "SimSun",
            Some("CIDFontType2"),
            Some("Identity-H")
        ));
        assert!(!type0_base_font_matches_descendant(
            "NotoSansCJKjp-Regular",
            "NotoSansCJKjp-Regular",
            Some("CIDFontType0"),
            Some("Identity-H")
        ));
        assert!(!type0_base_font_matches_descendant(
            "SimSun-Identity-H",
            "SimSun",
            Some("CIDFontType2"),
            Some("Identity-H")
        ));
    }

    #[test]
    fn extracted_text_normalization_ignores_pdf_object_spacing_around_punctuation() {
        assert_eq!(
            normalize_extracted_text("slideMaster .  Two shapes ( one )"),
            "slideMaster. Two shapes (one)"
        );
        assert_eq!(
            normalize_extracted_text("slideMaster. Two shapes (one)"),
            "slideMaster. Two shapes (one)"
        );
        assert_eq!(
            normalize_extracted_text("\u{f061}\u{f064}\u{f064}\u{f020}\u{f074}"),
            "\u{f061}\u{f064}\u{f064} \u{f074}"
        );
    }

    #[test]
    fn page_text_prefers_characters_over_overlapping_segments() {
        let pages = normalized_page_text_from_parts(
            1,
            [(0, "\u{f045}"), (0, "\u{f04a}"), (0, "\u{f049}")],
            [
                (0, "\u{f045}\u{f04a}"),
                (0, "\u{f045}\u{f04a}"),
                (0, "\u{f049}"),
            ],
        );

        assert_eq!(pages, ["\u{f045}\u{f04a}\u{f049}"]);
    }

    #[test]
    fn page_text_falls_back_to_segments_without_character_data() {
        let pages =
            normalized_page_text_from_parts(1, std::iter::empty(), [(0, "Hello"), (0, "world")]);

        assert_eq!(pages, ["Hello world"]);
    }

    #[test]
    fn text_content_key_ignores_pdf_whitespace_segmentation_but_not_content() {
        assert_eq!(
            extracted_text_content_key("professionally produced"),
            extracted_text_content_key("pro fessionally\nproduced")
        );
        assert_ne!(
            extracted_text_content_key("Page 1 of 1"),
            extracted_text_content_key("Page 1 of 2")
        );
        assert_ne!(
            extracted_text_content_key("End Date"),
            extracted_text_content_key("End Date ate")
        );
    }

    #[test]
    fn independently_confirmed_rtl_text_tolerates_only_bidi_mirror_identity() {
        let candidate = "=defined)ةعقوتمةجيتن)11";
        let golden = "=defined)ةعقوتمةجيتن(11";
        assert_ne!(
            extracted_text_line_content_key(candidate, false, false),
            extracted_text_line_content_key(golden, false, false)
        );
        assert_eq!(
            extracted_text_line_content_key(candidate, true, false),
            extracted_text_line_content_key(golden, true, false)
        );
        assert_ne!(
            extracted_text_line_content_key("LTR ) text", true, false),
            extracted_text_line_content_key("LTR ( text", true, false)
        );
        assert_ne!(
            extracted_text_line_content_key("ن)", true, false),
            extracted_text_line_content_key("ن()", true, false)
        );
    }

    #[test]
    fn independently_confirmed_ordered_text_tolerates_ltr_pdfium_mirroring_only() {
        assert_eq!(
            pdftotext_line_content_keys("  [Type text]\n\nnext line  \n"),
            vec!["[Typetext]", "nextline"]
        );
        assert_ne!(
            pdftotext_line_content_keys("[Type text]\nnext line"),
            pdftotext_line_content_keys("[Type text] next line")
        );
        assert_eq!(
            extracted_text_line_content_key("[Type text[", true, true),
            extracted_text_line_content_key("[Type text]", true, true)
        );
        assert_ne!(
            extracted_text_line_content_key("[Type text[", true, true),
            extracted_text_line_content_key("[Type texts]", true, true)
        );
    }

    #[test]
    fn base_font_name_ignores_foundry_syntax_but_preserves_style() {
        assert_eq!(canonical_pdf_base_font_name("ArialMT"), "arial");
        assert_eq!(canonical_pdf_base_font_name("Arial"), "arial");
        assert_eq!(canonical_pdf_base_font_name("Arial-BoldMT"), "arialbold");
        assert_eq!(canonical_pdf_base_font_name("Arial,Bold"), "arialbold");
        assert_eq!(canonical_pdf_base_font_name("OpenSans-Regular"), "opensans");
        assert_eq!(canonical_pdf_base_font_name("OpenSans"), "opensans");
        assert_eq!(
            canonical_pdf_base_font_name("Aptos Narrow,Italic"),
            "aptosnarrowitalic"
        );
        assert_ne!(
            canonical_pdf_base_font_name("Arial-BoldMT"),
            canonical_pdf_base_font_name("ArialMT")
        );
    }

    #[test]
    fn style_color_allows_one_rgb_quantization_step_but_keeps_alpha_exact() {
        let color = |value: &str| Some(value.to_string());
        assert!(pdf_style_colors_equivalent(
            &color("#b4c6e7@ff"),
            &color("#b4c7e7@ff")
        ));
        assert!(!pdf_style_colors_equivalent(
            &color("#b4c5e7@ff"),
            &color("#b4c7e7@ff")
        ));
        assert!(!pdf_style_colors_equivalent(
            &color("#b4c6e7@fe"),
            &color("#b4c7e7@ff")
        ));
    }

    #[test]
    fn unordered_text_content_ignores_pdf_object_order_but_preserves_multiplicity() {
        assert_eq!(
            unordered_extracted_text_content("header body footer"),
            unordered_extracted_text_content("body footer header")
        );
        assert_ne!(
            unordered_extracted_text_content("header body footer"),
            unordered_extracted_text_content("header body body footer")
        );
    }

    #[test]
    fn page_range_summary_coalesces_consecutive_failures() {
        assert_eq!(format_page_ranges(&[]), "none");
        assert_eq!(format_page_ranges(&[0]), "0");
        assert_eq!(format_page_ranges(&[0, 1, 2, 4, 7, 8]), "0-2,4,7-8");
    }

    #[test]
    fn text_edge_tolerance_represents_seven_fixed_raster_samples() {
        let a4 = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: 595.32,
            top: 841.92,
        };
        let widescreen_slide = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: 960.0,
            top: 540.0,
        };
        let narrow_custom_page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: 250.0,
            top: 500.0,
        };

        assert!((text_edge_tolerance_pt(a4) - 3.13).abs() < 0.01);
        assert!((text_edge_tolerance_pt(widescreen_slide) - 5.04).abs() < 0.01);
        assert_eq!(text_edge_tolerance_pt(narrow_custom_page), 2.5);
    }

    #[test]
    fn text_lines_use_baseline_identity_instead_of_overlapping_extents() {
        assert!(same_text_line(100.0, 100.2));
        assert!(!same_text_line(100.0, 100.3));
    }

    #[test]
    fn text_lines_follow_the_extracted_writing_axis_at_content_crossings() {
        let character = |text: &str, bounds: PdfBounds, origin_x, origin_y, angle_degrees| {
            TextCharacterContract {
                text: text.to_owned(),
                font_name: "arial".to_owned(),
                bounds,
                origin_x,
                origin_y,
                writing_direction: TextWritingDirection::from_degrees(angle_degrees),
            }
        };
        let vertical_y = character(
            "y",
            PdfBounds {
                left: 84.9,
                bottom: 684.0,
                right: 97.6,
                top: 693.0,
            },
            87.3,
            686.7,
            270.0,
        );
        let vertical_next = character(
            "z",
            PdfBounds {
                left: 84.9,
                bottom: 674.0,
                right: 97.6,
                top: 683.0,
            },
            87.4,
            676.7,
            270.0,
        );
        let horizontal_endnote = character(
            "i",
            PdfBounds {
                left: 164.0,
                bottom: 685.5,
                right: 169.0,
                top: 691.0,
            },
            164.0,
            687.2,
            0.0,
        );

        assert!(same_writing_axis(
            vertical_y.writing_direction,
            TextWritingDirection::from_degrees(90.0)
        ));
        assert!(text_characters_share_line(
            &vertical_y,
            &vertical_next,
            TextLineGeometry::Pdfium
        ));
        assert!(text_characters_share_line(
            &vertical_y,
            &vertical_next,
            TextLineGeometry::EmbeddedFont
        ));
        assert!(!text_characters_share_line(
            &vertical_y,
            &horizontal_endnote,
            TextLineGeometry::Pdfium
        ));
        assert!(!text_characters_share_line(
            &vertical_y,
            &horizontal_endnote,
            TextLineGeometry::EmbeddedFont
        ));
    }

    #[test]
    fn row_text_mask_spans_preserve_pixel_center_membership() {
        let width = 37u32;
        let height = 53u32;
        let page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: 612.0,
            top: 792.0,
        };
        let masks = [
            PdfBounds {
                left: 72.25,
                bottom: 700.5,
                right: 210.75,
                top: 724.25,
            },
            PdfBounds {
                left: 180.0,
                bottom: 699.0,
                right: 420.0,
                top: 716.0,
            },
            PdfBounds {
                left: 90.0,
                bottom: 95.0,
                right: 540.0,
                top: 114.0,
            },
        ];
        let rows = text_mask_x_spans_by_row(width, height, page, &masks);

        for (pixel_y, spans) in rows.iter().enumerate().take(height as usize) {
            let y = page.top - (pixel_y as f32 + 0.5) / height as f32 * (page.top - page.bottom);
            for pixel_x in 0..width as usize {
                let x =
                    page.left + (pixel_x as f32 + 0.5) / width as f32 * (page.right - page.left);
                let expected = masks.iter().any(|mask| {
                    x >= mask.left - TEXT_MASK_PADDING_PT
                        && x <= mask.right + TEXT_MASK_PADDING_PT
                        && y >= mask.bottom - TEXT_MASK_PADDING_PT
                        && y <= mask.top + TEXT_MASK_PADDING_PT
                });
                let actual = spans
                    .iter()
                    .any(|span| pixel_x as u32 >= span.start && (pixel_x as u32) < span.end);
                assert_eq!(actual, expected, "pixel ({pixel_x}, {pixel_y})");
            }
        }
    }

    fn white_page(width: u32, height: u32) -> RenderedPageImage {
        let mut rgba = vec![255; width as usize * height as usize * 4];
        for alpha in rgba.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        RenderedPageImage {
            page_index: 0,
            width_px: width,
            height_px: height,
            page_width_pt: width as f32,
            page_height_pt: height as f32,
            rgba_crc32: String::new(),
            rgba,
        }
    }

    fn fill_black(image: &mut RenderedPageImage, left: u32, top: u32, width: u32, height: u32) {
        fill_rgb(image, left, top, width, height, [0, 0, 0]);
    }

    fn fill_rgb(
        image: &mut RenderedPageImage,
        left: u32,
        top: u32,
        width: u32,
        height: u32,
        rgb: [u8; 3],
    ) {
        for y in top..top + height {
            for x in left..left + width {
                let offset = ((y * image.width_px + x) * 4) as usize;
                image.rgba[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }

    #[test]
    fn flat_soft_mask_palette_keeps_repeated_core_colors_and_rejects_gradients() {
        let mut alpha = Vec::new();
        let mut black_matte_rgb = Vec::new();
        let mut append = |color: [u8; 3], count: usize| {
            alpha.extend(std::iter::repeat_n(u8::MAX, count));
            black_matte_rgb.extend(std::iter::repeat_n(color, count).flatten());
        };
        append([255, 255, 255], 100);
        append([217, 217, 217], 100);
        append([5, 5, 5], 100);
        append([80, 80, 80], 15);
        let image = crate::pdf_extract::SemanticSoftMaskImage {
            alpha,
            black_matte_rgb,
        };

        let (colors, color_count) = flat_soft_mask_palette(&image).expect("flat palette");
        assert_eq!(color_count, 2);
        assert!(colors[..color_count].contains(&[217, 217, 217]));
        assert!(colors[..color_count].contains(&[5, 5, 5]));

        let mut alpha = Vec::new();
        let mut black_matte_rgb = Vec::new();
        for color in [20, 60, 100, 140, 180] {
            alpha.extend(std::iter::repeat_n(u8::MAX, 20));
            black_matte_rgb.extend(std::iter::repeat_n([color; 3], 20).flatten());
        }
        assert!(
            flat_soft_mask_palette(&crate::pdf_extract::SemanticSoftMaskImage {
                alpha,
                black_matte_rgb,
            })
            .is_none()
        );
    }

    #[test]
    fn flat_palette_stencil_accepts_edge_phase_but_rejects_missing_or_displaced_layers() {
        let width = 100;
        let height = 100;
        let mut golden = white_page(width, height);
        let mut one_pixel_shift = white_page(width, height);
        let mut missing_gray = white_page(width, height);
        let mut displaced = white_page(width, height);
        fill_rgb(&mut golden, 40, 40, 20, 20, [217, 217, 217]);
        fill_black(&mut golden, 40, 40, 5, 20);
        fill_rgb(&mut one_pixel_shift, 41, 40, 20, 20, [217, 217, 217]);
        fill_black(&mut one_pixel_shift, 41, 40, 5, 20);
        fill_black(&mut missing_gray, 40, 40, 5, 20);
        fill_rgb(&mut displaced, 44, 40, 20, 20, [217, 217, 217]);
        fill_black(&mut displaced, 44, 40, 5, 20);
        let rect = PixelRect {
            left: 35,
            top: 35,
            width: 35,
            height: 30,
        };
        let colors = [[5, 5, 5], [217, 217, 217]];
        let compare = |candidate| {
            localized_flat_palette_stencil_diff_metrics(
                candidate,
                &golden,
                rect,
                &colors,
                VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta,
                height,
                1,
            )
        };

        assert_eq!(compare(&one_pixel_shift).significant_pixel_fraction, 0.0);
        assert!(
            compare(&missing_gray).significant_pixel_fraction
                > VisualTolerance::OFFICE_FIXED_OUTPUT.max_significant_pixel_fraction
        );
        assert!(
            compare(&displaced).significant_pixel_fraction
                > VisualTolerance::OFFICE_FIXED_OUTPUT.max_significant_pixel_fraction
        );
    }

    fn pixel_region(bounds: PdfBounds) -> LocalizedGraphicRegion {
        LocalizedGraphicRegion {
            bounds,
            comparison: LocalizedGraphicsComparison::Pixels,
        }
    }

    fn image_summary(width: &str, height: &str, bounds: &str) -> ImageSummary {
        ImageSummary {
            page_index: 0,
            page_image_index: 0,
            width: Some(width.to_string()),
            height: Some(height.to_string()),
            bounds: Some(bounds.to_string()),
            decoded_pixel_sha256: None,
            decoded_vertical_flip_sha256: None,
            axis_aligned_orientation: Some((1, 1)),
        }
    }

    #[test]
    fn one_sample_fixed_output_border_tiles_are_not_semantic_images() {
        assert!(fixed_output_stroke_tile(&image_summary(
            "6",
            "1",
            "[66.86 719.52 69.26 720.00]",
        )));
        assert!(fixed_output_stroke_tile(&image_summary(
            "1",
            "6",
            "[305.81 717.12 306.29 719.52]",
        )));
        assert!(!fixed_output_stroke_tile(&image_summary(
            "6",
            "2",
            "[66.86 718.00 69.26 720.00]",
        )));
        assert!(!fixed_output_stroke_tile(&image_summary(
            "60",
            "1",
            "[66.86 719.52 90.86 720.00]",
        )));
    }

    #[test]
    fn localized_graphics_reject_missing_patch_hidden_by_page_average() {
        let width = 1_333;
        let height = 1_885;
        let candidate = white_page(width, height);
        let mut golden = white_page(width, height);
        fill_black(&mut golden, 100, 100, 120, 120);
        let page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: width as f32,
            top: height as f32,
        };
        let graphic = PdfBounds {
            left: 100.0,
            bottom: height as f32 - 220.0,
            right: 220.0,
            top: height as f32 - 100.0,
        };
        let tolerance = VisualTolerance::OFFICE_FIXED_OUTPUT;
        let metrics = visual_diff_metrics(
            &candidate,
            &golden,
            tolerance.significant_channel_delta,
            &[],
            &[pixel_region(graphic)],
            page,
        )
        .unwrap();

        assert!(
            metrics.significant_pixel_fraction <= tolerance.max_significant_pixel_fraction,
            "the legacy page fraction should demonstrate the leak: {metrics:?}"
        );
        assert!(
            metrics.mean_absolute_channel_delta <= tolerance.max_mean_absolute_channel_delta,
            "the legacy page mean should demonstrate the leak: {metrics:?}"
        );
        assert!(
            metrics.max_localized_graphics_significant_pixel_fraction > 0.98,
            "{metrics:?}"
        );
        assert!(
            metrics.max_localized_graphics_mean_absolute_channel_delta > 250.0,
            "{metrics:?}"
        );
    }

    #[test]
    fn localized_graphics_accept_one_raster_pixel_phase_but_reject_larger_displacement() {
        let width = 100;
        let height = 100;
        let mut golden = white_page(width, height);
        let mut one_pixel_shift = white_page(width, height);
        let mut four_pixel_shift = white_page(width, height);
        fill_black(&mut golden, 40, 40, 20, 20);
        fill_black(&mut one_pixel_shift, 41, 40, 20, 20);
        fill_black(&mut four_pixel_shift, 44, 40, 20, 20);
        let rect = PixelRect {
            left: 40,
            top: 40,
            width: 20,
            height: 20,
        };
        let significant_channel_delta =
            VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta;

        let aligned = localized_visual_diff_metrics(
            &one_pixel_shift,
            &golden,
            rect,
            significant_channel_delta,
            height,
        );
        let displaced = localized_visual_diff_metrics(
            &four_pixel_shift,
            &golden,
            rect,
            significant_channel_delta,
            height,
        );

        assert_eq!(aligned.significant_pixel_fraction, 0.0, "{aligned:?}");
        assert!(
            displaced.significant_pixel_fraction
                > VisualTolerance::OFFICE_FIXED_OUTPUT.max_significant_pixel_fraction,
            "{displaced:?}"
        );
    }

    #[test]
    fn exact_image_identity_uses_the_fixed_comparison_grid_for_placement() {
        let page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: 595.32,
            top: 841.92,
        };
        assert!(image_placement_matches_at_comparison_grid(
            Some("[95.15 747.20 115.15 767.20]"),
            Some("[95.15 747.22 115.15 767.22]"),
            page,
            1_333,
            1_885,
        ));
        assert!(!image_placement_matches_at_comparison_grid(
            Some("[72.00 708.75 83.25 720.00]"),
            Some("[72.00 706.55 83.30 717.85]"),
            page,
            1_333,
            1_885,
        ));
    }

    #[test]
    fn localized_graphics_are_not_silenced_by_text_masks() {
        let width = 200;
        let height = 200;
        let candidate = white_page(width, height);
        let mut golden = white_page(width, height);
        fill_black(&mut golden, 40, 60, 40, 30);
        let page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: width as f32,
            top: height as f32,
        };
        let graphic = PdfBounds {
            left: 40.0,
            bottom: 110.0,
            right: 80.0,
            top: 140.0,
        };
        let metrics = visual_diff_metrics(
            &candidate,
            &golden,
            VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta,
            &[graphic],
            &[pixel_region(graphic)],
            page,
        )
        .unwrap();

        assert_eq!(metrics.significant_pixels, 0);
        assert!(metrics.masked_pixels > 0);
        assert!(
            metrics.max_localized_graphics_significant_pixel_fraction > 0.9,
            "{metrics:?}"
        );
    }

    #[test]
    fn soft_mask_stencil_accepts_raster_vector_edge_variation_but_not_displacement() {
        let width = 100;
        let height = 100;
        let mut aligned_candidate = white_page(width, height);
        let mut displaced_candidate = white_page(width, height);
        let mut golden = white_page(width, height);
        fill_black(&mut golden, 40, 20, 10, 60);
        fill_black(&mut aligned_candidate, 42, 21, 10, 58);
        fill_black(&mut displaced_candidate, 65, 20, 10, 60);
        let page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: width as f32,
            top: height as f32,
        };
        let stencil = LocalizedGraphicRegion {
            bounds: page,
            comparison: LocalizedGraphicsComparison::SoftMaskStencil {
                solid_rgb: [0, 0, 0],
            },
        };
        let aligned = visual_diff_metrics(
            &aligned_candidate,
            &golden,
            VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta,
            &[],
            &[stencil],
            page,
        )
        .unwrap();
        let displaced = visual_diff_metrics(
            &displaced_candidate,
            &golden,
            VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta,
            &[],
            &[stencil],
            page,
        )
        .unwrap();

        assert_eq!(
            aligned.max_localized_graphics_significant_pixel_fraction, 0.0,
            "{aligned:?}"
        );
        assert!(
            displaced.max_localized_graphics_significant_pixel_fraction > 0.99,
            "{displaced:?}"
        );
    }

    #[test]
    fn stencil_paint_recognizes_resampled_alpha_without_accepting_wrong_color() {
        let tolerance = VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta;

        assert!(pixel_matches_stencil_paint(
            [255, 0, 0],
            [255, 0, 0],
            tolerance
        ));
        assert!(pixel_matches_stencil_paint(
            [253, 71, 71],
            [255, 0, 0],
            tolerance
        ));
        assert!(!pixel_matches_stencil_paint(
            [255, 255, 255],
            [255, 0, 0],
            tolerance
        ));
        assert!(!pixel_matches_stencil_paint(
            [0, 0, 0],
            [255, 0, 0],
            tolerance
        ));
        assert!(!pixel_matches_stencil_paint(
            [0, 0, 255],
            [255, 0, 0],
            tolerance
        ));
    }

    #[test]
    fn translucent_soft_mask_ignores_text_foreground_but_still_requires_visible_effect() {
        let width = 100;
        let height = 100;
        let mut golden = white_page(width, height);
        let mut aligned_candidate = white_page(width, height);
        let mut missing_effect_candidate = white_page(width, height);
        fill_black(&mut golden, 40, 30, 20, 40);
        fill_black(&mut aligned_candidate, 41, 30, 20, 40);
        fill_black(&mut missing_effect_candidate, 41, 30, 20, 40);
        fill_black(&mut golden, 30, 72, 40, 20);
        fill_black(&mut aligned_candidate, 30, 72, 40, 20);
        let page = PdfBounds {
            left: 0.0,
            bottom: 0.0,
            right: width as f32,
            top: height as f32,
        };
        let text = PdfBounds {
            left: 40.0,
            bottom: 30.0,
            right: 60.0,
            top: 70.0,
        };
        let effect = LocalizedGraphicRegion {
            bounds: page,
            comparison: LocalizedGraphicsComparison::TranslucentSoftMask,
        };
        let aligned = visual_diff_metrics(
            &aligned_candidate,
            &golden,
            VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta,
            &[text],
            &[effect],
            page,
        )
        .unwrap();
        let missing = visual_diff_metrics(
            &missing_effect_candidate,
            &golden,
            VisualTolerance::OFFICE_FIXED_OUTPUT.significant_channel_delta,
            &[text],
            &[effect],
            page,
        )
        .unwrap();

        assert_eq!(
            aligned.max_localized_graphics_significant_pixel_fraction, 0.0,
            "{aligned:?}"
        );
        assert!(
            missing.max_localized_graphics_significant_pixel_fraction
                > VisualTolerance::OFFICE_FIXED_OUTPUT.max_significant_pixel_fraction,
            "{missing:?}"
        );
    }

    #[test]
    fn semantic_soft_mask_match_allows_only_eight_bit_preblend_quantization() {
        use crate::pdf_extract::SemanticSoftMaskImage;

        let office = SemanticSoftMaskImage {
            alpha: vec![124, 255],
            black_matte_rgb: vec![87, 64, 66, 17, 34, 51],
        };
        let equivalent = SemanticSoftMaskImage {
            alpha: vec![124, 255],
            black_matte_rgb: vec![88, 65, 66, 17, 34, 51],
        };
        let wrong_color = SemanticSoftMaskImage {
            alpha: vec![124, 255],
            black_matte_rgb: vec![89, 65, 66, 17, 34, 51],
        };
        let wrong_alpha = SemanticSoftMaskImage {
            alpha: vec![123, 255],
            black_matte_rgb: office.black_matte_rgb.clone(),
        };

        assert!(semantic_soft_mask_samples_match(&equivalent, &office));
        assert!(!semantic_soft_mask_samples_match(&wrong_color, &office));
        assert!(!semantic_soft_mask_samples_match(&wrong_alpha, &office));
    }
}

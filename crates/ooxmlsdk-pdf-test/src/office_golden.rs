use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use image::ImageFormat;
use ooxmlsdk_pdf::{PdfConversionDiagnostics, PdfFontAudit};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::pdf_extract::{
    RenderedPagePairError, first_pdf_page_text_mismatch, pdf_font_structure, pdf_page_dimensions,
    visit_rendered_page_pairs,
};
use crate::{
    CalibrationError, PdfBounds, PdfSummary, RenderedPageImage, Result, parse_pdf_rect,
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
    pub total_pixels: usize,
    pub significant_pixels: usize,
    pub significant_pixel_fraction: f64,
    pub mean_absolute_channel_delta: f64,
    pub max_channel_delta: u8,
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
    pub message: String,
}

impl OfficeGoldenFailure {
    fn new(layer: OfficeGoldenComparisonLayer, error: impl fmt::Display) -> Self {
        Self {
            layer,
            message: error.to_string(),
        }
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
    let mut stage_trace = OfficeGoldenStageTrace::new(case);
    let root = workspace_root();
    let source_path = root.join("corpus").join(case.corpus).join(case.source);
    let golden_path = root
        .join("corpus_pdf_conv")
        .join(case.corpus)
        .join(format!("{}.pdf", case.source));
    verify_manifest_record(&root, case)
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
    let options = ooxmlsdk_pdf::PdfOptions {
        // Word's print-optimized ExportAsFixedFormat path recompresses JPEG
        // image XObjects at quality 75. The four independently converted
        // sdtContent.docx records preserve the source dimensions and 220-DPI
        // density while changing the embedded stream from quality 95 to 75.
        jpeg_quality: Some(75),
        source_file_name: source_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string),
        ui_language: Some(case.ui_language.to_string()),
        ..Default::default()
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
        return Err(OfficeGoldenFailure {
            layer: error.layer,
            message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
        });
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
        let error = OfficeGoldenFailure::new(
            OfficeGoldenComparisonLayer::PageGeometry,
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
        return Err(OfficeGoldenFailure {
            layer: error.layer,
            message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
        });
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
        let error = OfficeGoldenFailure::new(
            OfficeGoldenComparisonLayer::PageGeometry,
            CalibrationError::OfficeGolden(format!(
                "case {} page {page_index} media box mismatch: candidate={candidate:?}, golden={golden:?}",
                case.id
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
        return Err(OfficeGoldenFailure {
            layer: error.layer,
            message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
        });
    }

    // The full summary below performs the same text verdict. Keep the early
    // PDFium pass only for large documents where failing before object and
    // character collection saves meaningful work; reopening every small PDF
    // merely duplicates the hot path for passing corpus cases.
    let run_early_text_preflight = candidate_page_count >= EARLY_TEXT_PREFLIGHT_MIN_PAGES
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
        stage_trace.mark("page-text");
        let error = OfficeGoldenFailure::new(
            OfficeGoldenComparisonLayer::Text,
            CalibrationError::OfficeGolden(format!(
                "case {} page {} normalized text mismatch: candidate={:?}, golden={:?}",
                case.id, mismatch.page_index, mismatch.candidate, mismatch.golden
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
        let artifact_dir = write_candidate_artifact(
            case.id,
            &candidate_pdf,
            &golden_pdf,
            &candidate_font_audit,
            &candidate_diagnostics,
            &[mismatch.page_index],
        )
        .map_err(|artifact_error| {
            OfficeGoldenFailure::new(
                OfficeGoldenComparisonLayer::ComparisonArtifact,
                artifact_error,
            )
        })?;
        return Err(OfficeGoldenFailure {
            layer: error.layer,
            message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
        });
    }
    stage_trace.mark("page-text");

    let candidate = PdfSummary::from_bytes_for_golden(&candidate_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    let golden = PdfSummary::from_bytes_for_golden(&golden_pdf).map_err(|error| {
        OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
    })?;
    stage_trace.mark("pdf-summary");

    let text_contract = match assert_page_geometry_contract(case.id, &candidate, &golden)
        .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PageGeometry, error))
        .and_then(|()| {
            assert_text_contract(case.id, &candidate, &golden)
                .map_err(|error| OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::Text, error))
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
            return Err(OfficeGoldenFailure {
                layer: error.layer,
                message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
            });
        }
    };
    stage_trace.mark("text-contract");

    if let Err(error) = assert_text_font_assignment_contract(
        case.id,
        &text_contract.candidate_lines,
        &text_contract.golden_lines,
    ) {
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
        return Err(OfficeGoldenFailure {
            layer: error.layer,
            message: format!("{}; artifacts={}", error.message, artifact_dir.display()),
        });
    }
    stage_trace.mark("text-font-assignment");
    let text_masks = text_contract.masks;

    let mut page_diffs = Vec::with_capacity(golden.page_count);
    let mut visual_passes = true;
    let mut failing_pages = Vec::new();
    let mut artifact_dir = None;
    visit_rendered_page_pairs(
        &candidate_pdf,
        &golden_pdf,
        RASTER_WIDTH,
        |page_index, candidate_page, golden_page| -> DetailedResult<()> {
            let metrics = visual_diff_metrics(
                &candidate_page,
                &golden_page,
                tolerance.significant_channel_delta,
                text_masks.get(page_index).map(Vec::as_slice).unwrap_or(&[]),
                parse_pdf_rect(&golden.media_boxes[page_index]).map_err(|error| {
                    OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::PdfExtraction, error)
                })?,
            )
            .map_err(|error| {
                OfficeGoldenFailure::new(OfficeGoldenComparisonLayer::VisibleOutput, error)
            })?;
            let page_passes = metrics.significant_pixel_fraction
                <= tolerance.max_significant_pixel_fraction
                && metrics.mean_absolute_channel_delta <= tolerance.max_mean_absolute_channel_delta;
            visual_passes &= page_passes;
            if !page_passes {
                if write_failure_artifacts {
                    collect_candidate_diagnostics(
                        &source_path,
                        &diagnostic_options,
                        &mut candidate_diagnostics,
                    );
                    if artifact_dir.is_none() {
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
        Err(OfficeGoldenFailure::new(
            OfficeGoldenComparisonLayer::VisibleOutput,
            format!(
                "case {} exceeds {:?}; failing pages={} ({} of {}); maxima: significant_pixel_fraction={}, mean_absolute_channel_delta={}, max_channel_delta={}; artifacts={}",
                case.id,
                tolerance,
                format_page_ranges(&failing_pages),
                failing_pages.len(),
                report.page_diffs.len(),
                max_significant_pixel_fraction,
                max_mean_absolute_channel_delta,
                max_channel_delta,
                report
                    .artifact_dir
                    .as_deref()
                    .map_or_else(|| "none".into(), |path| path.display().to_string())
            ),
        ))
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

#[derive(Debug)]
struct ConversionManifestIdentity {
    status: String,
    reference_engine: String,
    source_sha256: String,
    output_sha256: String,
    environment_id: String,
    output: String,
}

type ConversionManifestIdentities =
    std::result::Result<BTreeMap<(String, String), ConversionManifestIdentity>, String>;

static CONVERSION_MANIFEST_IDENTITIES: OnceLock<ConversionManifestIdentities> = OnceLock::new();

fn verify_manifest_record(root: &Path, case: OfficeGoldenCase<'_>) -> Result<()> {
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
    Ok(())
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
            let source = string("file")?;
            let key = (corpus.to_string(), source.clone());
            let identity = ConversionManifestIdentity {
                status: string("status")?,
                reference_engine: string("reference_engine")?,
                source_sha256: string("source_sha256")?,
                output_sha256: string("output_sha256")?,
                environment_id: string("environment_id")?,
                output: string("output")?,
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
) -> Result<()> {
    if candidate.page_count != golden.page_count {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} page count mismatch: candidate={}, golden={}",
            candidate.page_count, golden.page_count
        )));
    }
    if candidate.media_boxes.len() != golden.media_boxes.len() {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} media box count mismatch: candidate={}, golden={}",
            candidate.media_boxes.len(),
            golden.media_boxes.len()
        )));
    }
    for (page_index, (candidate_box, golden_box)) in candidate
        .media_boxes
        .iter()
        .zip(&golden.media_boxes)
        .enumerate()
    {
        let candidate_box =
            parse_pdf_rect(candidate_box).map_err(CalibrationError::PdfiumExtraction)?;
        let golden_box = parse_pdf_rect(golden_box).map_err(CalibrationError::PdfiumExtraction)?;
        let max_delta = [
            (candidate_box.left - golden_box.left).abs(),
            (candidate_box.bottom - golden_box.bottom).abs(),
            (candidate_box.right - golden_box.right).abs(),
            (candidate_box.top - golden_box.top).abs(),
        ]
        .into_iter()
        .fold(0.0_f32, f32::max);
        if max_delta > MEDIA_BOX_TOLERANCE_PT {
            return Err(CalibrationError::OfficeGolden(format!(
                "case {case_id} page {page_index} media box mismatch: candidate={candidate_box:?}, golden={golden_box:?}"
            )));
        }
    }
    Ok(())
}

fn assert_text_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<TextContract> {
    let candidate_text = normalized_page_text(candidate);
    let golden_text = normalized_page_text(golden);
    if page_text_content_bags(&candidate_text) != page_text_content_bags(&golden_text) {
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} normalized page text mismatch: candidate={candidate_text:?}, golden={golden_text:?}"
        )));
    }
    assert_text_style_contract(case_id, candidate, golden)?;
    let candidate_lines = text_line_contracts(candidate)?;
    let golden_lines = text_line_contracts(golden)?;
    let masks =
        assert_text_line_geometry(case_id, candidate, golden, &candidate_lines, &golden_lines)?;
    Ok(TextContract {
        masks,
        candidate_lines,
        golden_lines,
    })
}

struct TextContract {
    masks: Vec<Vec<PdfBounds>>,
    candidate_lines: Vec<Vec<TextLineContract>>,
    golden_lines: Vec<Vec<TextLineContract>>,
}

#[derive(Clone, Debug)]
struct TextLineContract {
    text: String,
    font_runs: Vec<TextFontRunContract>,
    bounds: PdfBounds,
    origin_y: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextFontRunContract {
    font_name: String,
    text: String,
}

fn assert_text_style_contract(
    case_id: &str,
    candidate: &PdfSummary,
    golden: &PdfSummary,
) -> Result<()> {
    use std::collections::BTreeSet;

    let style_set = |summary: &PdfSummary| {
        summary
            .text_objects
            .iter()
            .filter(|object| !object.text.trim().is_empty())
            .map(|object| {
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
        return Err(CalibrationError::OfficeGolden(format!(
            "case {case_id} text style set mismatch: candidate={candidate_styles:?}, golden={golden_styles:?}"
        )));
    }
    // Adobe's PDF reference defines final glyph placement through the complete
    // text rendering matrix. A producer's `Tf` and extracted vertical scale
    // are therefore not independent visible-output contracts. Effective size
    // remains constrained below by glyph baseline and horizontal ink bounds,
    // while family, explicit style, rendering mode, and color are checked here.
    Ok(())
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
    let parse = |value: &str| {
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
    };
    match (candidate, golden) {
        (Some(candidate), Some(golden)) => match (parse(candidate), parse(golden)) {
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
        },
        (None, None) => true,
        _ => false,
    }
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
) -> Result<Vec<Vec<PdfBounds>>> {
    let mut masks = vec![Vec::new(); candidate.page_count];
    for page_index in 0..candidate.page_count {
        let candidate_page = &candidate_lines[page_index];
        let golden_page = &golden_lines[page_index];
        let golden_page_bounds = parse_pdf_rect(&golden.media_boxes[page_index])
            .map_err(CalibrationError::PdfiumExtraction)?;
        let edge_tolerance = text_edge_tolerance_pt(golden_page_bounds);
        if candidate_page.len() != golden_page.len() {
            return Err(CalibrationError::OfficeGolden(format!(
                "case {case_id} page {page_index} text line count mismatch: candidate={}, golden={}",
                candidate_page.len(),
                golden_page.len()
            )));
        }
        for (line_index, (candidate_line, golden_line)) in
            candidate_page.iter().zip(golden_page).enumerate()
        {
            if extracted_text_content_key(&candidate_line.text)
                != extracted_text_content_key(&golden_line.text)
            {
                return Err(CalibrationError::OfficeGolden(format!(
                    "case {case_id} page {page_index} line {line_index} text mismatch: candidate={:?}, golden={:?}",
                    candidate_line.text, golden_line.text
                )));
            }
            let golden_width = golden_line.bounds.right - golden_line.bounds.left;
            let width_tolerance =
                (golden_width.abs() * TEXT_WIDTH_TOLERANCE_RATIO).max(edge_tolerance);
            for (edge, candidate_value, golden_value, tolerance) in [
                (
                    "left",
                    candidate_line.bounds.left,
                    golden_line.bounds.left,
                    edge_tolerance,
                ),
                (
                    "right",
                    candidate_line.bounds.right,
                    golden_line.bounds.right,
                    width_tolerance,
                ),
                (
                    "baseline origin",
                    candidate_line.origin_y,
                    golden_line.origin_y,
                    edge_tolerance,
                ),
            ] {
                if (candidate_value - golden_value).abs() > tolerance {
                    return Err(CalibrationError::OfficeGolden(format!(
                        "case {case_id} page {page_index} line {line_index} {edge} mismatch: candidate={candidate_value}, golden={golden_value}, tolerance={tolerance}"
                    )));
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
) -> Result<()> {
    // MS-OI29500 17.3.2.26 assigns fonts by character class, while PDF
    // producers may split the same run into different text objects. Compare
    // the character-level font assignment after spatial line reconstruction.
    for (page_index, (candidate_page, golden_page)) in
        candidate_lines.iter().zip(golden_lines).enumerate()
    {
        for (line_index, (candidate_line, golden_line)) in
            candidate_page.iter().zip(golden_page).enumerate()
        {
            if candidate_line.font_runs != golden_line.font_runs {
                return Err(CalibrationError::OfficeGolden(format!(
                    "case {case_id} page {page_index} line {line_index} font assignment mismatch: candidate={:?}, golden={:?}",
                    candidate_line.font_runs, golden_line.font_runs
                )));
            }
        }
    }
    Ok(())
}

fn text_edge_tolerance_pt(page_bounds: PdfBounds) -> f32 {
    (page_bounds.width().abs() / RASTER_WIDTH as f32 * TEXT_EDGE_TOLERANCE_RASTER_PIXELS)
        .max(TEXT_EDGE_TOLERANCE_MIN_PT)
}

fn text_line_contracts(summary: &PdfSummary) -> Result<Vec<Vec<TextLineContract>>> {
    #[derive(Clone, Debug)]
    struct TextCharacterContract {
        text: String,
        font_name: String,
        bounds: PdfBounds,
        origin_y: f32,
    }

    let mut characters = vec![Vec::<TextCharacterContract>::new(); summary.page_count];
    for character in &summary.text_chars {
        if character.text.chars().all(char::is_whitespace) {
            continue;
        }
        let bounds =
            parse_pdf_rect(&character.bounds).map_err(CalibrationError::PdfiumExtraction)?;
        let origin_y = character.origin_y.parse::<f32>().map_err(|error| {
            CalibrationError::OfficeGolden(format!(
                "invalid extracted text origin {:?}: {error}",
                character.origin_y
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
            origin_y,
        });
    }

    let mut pages = Vec::with_capacity(characters.len());
    for mut page_characters in characters {
        page_characters.sort_by(|left, right| {
            right
                .origin_y
                .total_cmp(&left.origin_y)
                .then_with(|| left.bounds.left.total_cmp(&right.bounds.left))
        });
        let mut line_characters = Vec::<Vec<TextCharacterContract>>::new();
        for character in page_characters {
            if let Some(line) = line_characters.iter_mut().find(|line| {
                line.first()
                    .is_some_and(|first| vertical_bounds_overlap(first.bounds, character.bounds))
            }) {
                line.push(character);
            } else {
                line_characters.push(vec![character]);
            }
        }
        let mut lines = line_characters
            .into_iter()
            .map(|mut characters| {
                characters.sort_by(|left, right| {
                    left.bounds
                        .left
                        .total_cmp(&right.bounds.left)
                        .then_with(|| right.origin_y.total_cmp(&left.origin_y))
                });
                let first = characters
                    .first()
                    .expect("a spatial text line always contains a character");
                let mut line = TextLineContract {
                    text: String::new(),
                    font_runs: Vec::new(),
                    bounds: first.bounds,
                    origin_y: first.origin_y,
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
                .then_with(|| left.bounds.left.total_cmp(&right.bounds.left))
        });
        pages.push(lines);
    }
    Ok(pages)
}

fn vertical_bounds_overlap(left: PdfBounds, right: PdfBounds) -> bool {
    left.top.min(right.top) > left.bottom.max(right.bottom)
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
    let mut extracted_pages = vec![String::new(); summary.page_count];
    for segment in &summary.text_segments {
        if let Some(page) = extracted_pages.get_mut(segment.page_index) {
            if !page.is_empty() {
                page.push(' ');
            }
            page.push_str(&segment.text);
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
        if character.is_whitespace() {
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
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn visual_diff_metrics(
    candidate: &RenderedPageImage,
    golden: &RenderedPageImage,
    significant_channel_delta: u8,
    text_masks: &[PdfBounds],
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
    let total_pixels = (candidate.width_px * comparison_height_px) as usize;
    let mut significant_pixels = 0usize;
    let mut absolute_delta_sum = 0u64;
    let mut max_channel_delta = 0u8;
    let text_mask_rows = text_mask_x_spans_by_row(golden.height_px, page_bounds, text_masks);
    let row_bytes = candidate.width_px as usize * 4;
    let page_width = page_bounds.right - page_bounds.left;
    let pixel_x_points = (0..candidate.width_px)
        .map(|pixel_x| {
            page_bounds.left + (pixel_x as f32 + 0.5) / candidate.width_px as f32 * page_width
        })
        .collect::<Vec<_>>();
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
            let x = pixel_x_points[pixel_x];
            while text_mask_spans
                .get(text_mask_index)
                .is_some_and(|span| span.right < x)
            {
                text_mask_index += 1;
            }
            if text_mask_spans
                .get(text_mask_index)
                .is_some_and(|span| x >= span.left)
            {
                continue;
            }
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
    Ok(VisualDiffMetrics {
        total_pixels,
        significant_pixels,
        significant_pixel_fraction: significant_pixels as f64 / total_pixels as f64,
        mean_absolute_channel_delta: absolute_delta_sum as f64 / (total_pixels * 3) as f64,
        max_channel_delta,
    })
}

#[derive(Clone, Copy, Debug)]
struct TextMaskXSpan {
    left: f32,
    right: f32,
}

fn text_mask_x_spans_by_row(
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
    for (pixel_y, row) in rows.iter_mut().enumerate() {
        let y = page.top - (pixel_y as f32 + 0.5) / height_px as f32 * page_height;
        row.extend(masks.iter().filter_map(|mask| {
            (y >= mask.bottom - TEXT_MASK_PADDING_PT && y <= mask.top + TEXT_MASK_PADDING_PT)
                .then_some(TextMaskXSpan {
                    left: mask.left - TEXT_MASK_PADDING_PT,
                    right: mask.right + TEXT_MASK_PADDING_PT,
                })
        }));
        row.sort_unstable_by(|left, right| left.left.total_cmp(&right.left));
        if row.len() < 2 {
            continue;
        }
        let mut merged = 0usize;
        for index in 1..row.len() {
            if row[index].left <= row[merged].right {
                row[merged].right = row[merged].right.max(row[index].right);
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
        candidate,
        &candidate.rgba,
    )?;
    write_png(
        &artifact_dir.join(format!("page-{page_index}-golden.png")),
        golden,
        &golden.rgba,
    )?;
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
        candidate,
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
    let has_visible_text = audit.painted_text_portion_count > 0;
    if has_visible_text && fonts.is_empty() {
        return Err(CalibrationError::OfficeGolden(
            "candidate has visible text but no PDF font resources".to_string(),
        ));
    }

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

fn write_png(path: &Path, page: &RenderedPageImage, rgba: &[u8]) -> Result<()> {
    image::save_buffer_with_format(
        path,
        rgba,
        page.width_px,
        page.height_px,
        image::ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|error| {
        CalibrationError::OfficeGolden(format!("could not write {}: {error}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PdfBounds, TEXT_MASK_PADDING_PT, canonical_pdf_base_font_name, extracted_text_content_key,
        format_page_ranges, normalize_extracted_text, pdf_style_colors_equivalent,
        text_edge_tolerance_pt, text_mask_x_spans_by_row, type0_base_font_matches_descendant,
        unordered_extracted_text_content,
    };

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
        let rows = text_mask_x_spans_by_row(height, page, &masks);

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
                let actual = spans.iter().any(|span| x >= span.left && x <= span.right);
                assert_eq!(actual, expected, "pixel ({pixel_x}, {pixel_y})");
            }
        }
    }
}

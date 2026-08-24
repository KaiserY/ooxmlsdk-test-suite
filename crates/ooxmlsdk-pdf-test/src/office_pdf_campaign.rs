use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ooxmlsdk_pdf::{
    FieldUpdateDateTime, PdfAttachment, PdfAttachmentAssociation, PdfDateTime, PdfDocumentKind,
    PdfFormSubmitFormat, PdfImageOptimizationPolicy, PdfLinkDefaultAction, PdfOptimizeFor,
    PdfOptions, PdfPageLayout, PdfStandard, PdfViewerMagnification, PdfViewerPageMode,
    resolve_pdf_options,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::office_golden::compare_office_golden_detailed_with_prevalidated_options;
use crate::{OfficeGoldenCase, VisualTolerance};

pub const CAMPAIGN_SCHEMA_VERSION: u32 = 1;
pub const CONVERSION_SCHEMA_VERSION: u32 = 3;
pub const CAMPAIGN_ID: &str = "office-ooxml-pdf-options-v1";
pub const ASSIGNMENT_ALGORITHM: &str = "sha256-balanced-marginals-v1";
pub const CAMPAIGN_SEED: &str = "ooxmlsdk-office-pdf-options-2026-08-18-v1";
pub const EXPECTED_ASSIGNMENT_COUNT: usize = 5_370;
pub const PILOT_PER_FAMILY: usize = 100;
pub const PILOT_ASSIGNMENT_COUNT: usize = 300;

const AUDIT_WORKER_POLL_INTERVAL: Duration = Duration::from_millis(10);

const LOCALES: [&str; 8] = [
    "zh-CN", "zh-TW", "ja-JP", "ko-KR", "en-US", "de-DE", "fr-FR", "es-ES",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficeFamily {
    Word,
    Excel,
    PowerPoint,
}

impl OfficeFamily {
    pub const ALL: [Self; 3] = [Self::Word, Self::Excel, Self::PowerPoint];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Excel => "excel",
            Self::PowerPoint => "powerpoint",
        }
    }

    pub const fn document_kind(self) -> PdfDocumentKind {
        match self {
            Self::Word => PdfDocumentKind::Docx,
            Self::Excel => PdfDocumentKind::Xlsx,
            Self::PowerPoint => PdfDocumentKind::Pptx,
        }
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "docx" | "docm" | "dotx" | "dotm" => Some(Self::Word),
            "xlsx" | "xlsm" | "xltx" | "xltm" => Some(Self::Excel),
            "pptx" | "pptm" | "ppsx" | "ppsm" | "potx" | "potm" => Some(Self::PowerPoint),
            _ => None,
        }
    }
}

impl std::fmt::Display for OfficeFamily {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignPdfStandard {
    Pdf14,
    Pdf15,
    Pdf16,
    Pdf17,
    Pdf20,
    PdfA3a,
    PdfUa1,
}

impl CampaignPdfStandard {
    fn into_pdf(self) -> PdfStandard {
        match self {
            Self::Pdf14 => PdfStandard::Pdf14,
            Self::Pdf15 => PdfStandard::Pdf15,
            Self::Pdf16 => PdfStandard::Pdf16,
            Self::Pdf17 => PdfStandard::Pdf17,
            Self::Pdf20 => PdfStandard::Pdf20,
            Self::PdfA3a => PdfStandard::PdfA3a,
            Self::PdfUa1 => PdfStandard::PdfUa1,
        }
    }

    fn from_pdf(value: PdfStandard) -> Result<Self, String> {
        match value {
            PdfStandard::Pdf14 => Ok(Self::Pdf14),
            PdfStandard::Pdf15 => Ok(Self::Pdf15),
            PdfStandard::Pdf16 => Ok(Self::Pdf16),
            PdfStandard::Pdf17 => Ok(Self::Pdf17),
            PdfStandard::Pdf20 => Ok(Self::Pdf20),
            PdfStandard::PdfA3a => Ok(Self::PdfA3a),
            PdfStandard::PdfUa1 => Ok(Self::PdfUa1),
            other => Err(format!(
                "campaign model cannot encode effective standard {other:?}"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignLinkAction {
    Uri,
    RemoveExternalLinks,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignViewerPageMode {
    Default,
    UseOutlines,
    UseThumbs,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignPageLayout {
    Default,
    SinglePage,
    Continuous,
    ContinuousFacing,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignMagnification {
    Default,
    FitInWindow,
    FitWidth,
    FitVisible,
    Zoom125,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignFieldUpdate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub time_zone: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub utc_offset_hour: i8,
    pub utc_offset_minute: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignImageOptions {
    pub use_lossless_compression: bool,
    pub jpeg_quality: Option<u8>,
    pub reduce_resolution: bool,
    pub max_resolution_dpi: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignViewerOptions {
    pub page_mode: CampaignViewerPageMode,
    pub page_layout: CampaignPageLayout,
    pub magnification: CampaignMagnification,
    pub initial_page: u32,
    pub hide_toolbar: bool,
    pub hide_menubar: bool,
    pub hide_window_controls: bool,
    pub fit_window: bool,
    pub center_window: bool,
    pub display_document_title: bool,
    pub full_screen: bool,
    pub first_page_left: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignMetadataOptions {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<CampaignDateTime>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignPdfOptions {
    pub standards: Vec<CampaignPdfStandard>,
    pub compress_content_streams: bool,
    pub ui_language: String,
    pub format_locale: String,
    pub document_language: String,
    pub field_update: Option<CampaignFieldUpdate>,
    pub tagged_pdf: bool,
    pub pdf_ua_compliance: bool,
    pub export_bookmarks: bool,
    pub open_bookmark_levels: Option<i32>,
    pub page_range: Option<String>,
    pub images: CampaignImageOptions,
    pub link_action: CampaignLinkAction,
    pub export_form_fields: bool,
    pub viewer: CampaignViewerOptions,
    pub metadata: CampaignMetadataOptions,
    pub embed_campaign_attachment: bool,
}

impl CampaignPdfOptions {
    fn baseline(source_key: &str) -> Self {
        Self {
            standards: vec![CampaignPdfStandard::Pdf17],
            compress_content_streams: true,
            ui_language: "zh-CN".to_string(),
            format_locale: "zh-CN".to_string(),
            document_language: "zh-CN".to_string(),
            field_update: None,
            tagged_pdf: false,
            pdf_ua_compliance: false,
            export_bookmarks: true,
            open_bookmark_levels: None,
            page_range: None,
            images: CampaignImageOptions {
                use_lossless_compression: false,
                jpeg_quality: Some(75),
                reduce_resolution: false,
                max_resolution_dpi: Some(300),
            },
            link_action: CampaignLinkAction::Uri,
            export_form_fields: false,
            viewer: CampaignViewerOptions {
                page_mode: CampaignViewerPageMode::Default,
                page_layout: CampaignPageLayout::Default,
                magnification: CampaignMagnification::Default,
                initial_page: 1,
                hide_toolbar: false,
                hide_menubar: false,
                hide_window_controls: false,
                fit_window: false,
                center_window: false,
                display_document_title: true,
                full_screen: false,
                first_page_left: false,
            },
            metadata: CampaignMetadataOptions {
                title: Some(format!("OOXMLSDK configured golden: {source_key}")),
                ..Default::default()
            },
            embed_campaign_attachment: false,
        }
    }

    pub fn to_pdf_options(&self, source_key: &str, source_file_name: &str) -> PdfOptions {
        let mut options = PdfOptions {
            standards: self
                .standards
                .iter()
                .copied()
                .map(CampaignPdfStandard::into_pdf)
                .collect(),
            compress_content_streams: self.compress_content_streams,
            source_file_name: Some(source_file_name.to_string()),
            ui_language: Some(self.ui_language.clone()),
            format_locale: Some(self.format_locale.clone()),
            default_document_language: Some(self.document_language.clone()),
            field_update_datetime: self.field_update.as_ref().map(|value| FieldUpdateDateTime {
                year: value.year,
                month: value.month,
                day: value.day,
                hour: value.hour,
                minute: value.minute,
                second: value.second,
            }),
            field_update_time_zone: self
                .field_update
                .as_ref()
                .map(|value| value.time_zone.clone()),
            ..Default::default()
        };
        options.general.tagged_pdf = self.tagged_pdf;
        options.general.pdf_ua_compliance = self.pdf_ua_compliance;
        options.general.export_bookmarks = self.export_bookmarks;
        options.general.open_bookmark_levels = self.open_bookmark_levels;
        options.general.page_range = self.page_range.clone();
        options.images.use_lossless_compression = self.images.use_lossless_compression;
        options.images.jpeg_quality = self.images.jpeg_quality;
        options.images.reduce_resolution = self.images.reduce_resolution;
        options.images.max_resolution_dpi = self.images.max_resolution_dpi;
        options.links.default_action = match self.link_action {
            CampaignLinkAction::Uri => PdfLinkDefaultAction::Uri,
            CampaignLinkAction::RemoveExternalLinks => PdfLinkDefaultAction::RemoveExternalLinks,
        };
        options.forms.export_form_fields = self.export_form_fields;
        options.forms.submit_format = PdfFormSubmitFormat::Pdf;
        options.viewer.page_mode = match self.viewer.page_mode {
            CampaignViewerPageMode::Default => PdfViewerPageMode::Default,
            CampaignViewerPageMode::UseOutlines => PdfViewerPageMode::UseOutlines,
            CampaignViewerPageMode::UseThumbs => PdfViewerPageMode::UseThumbs,
        };
        options.viewer.page_layout = match self.viewer.page_layout {
            CampaignPageLayout::Default => PdfPageLayout::Default,
            CampaignPageLayout::SinglePage => PdfPageLayout::SinglePage,
            CampaignPageLayout::Continuous => PdfPageLayout::Continuous,
            CampaignPageLayout::ContinuousFacing => PdfPageLayout::ContinuousFacing,
        };
        options.viewer.magnification = match self.viewer.magnification {
            CampaignMagnification::Default => PdfViewerMagnification::Default,
            CampaignMagnification::FitInWindow => PdfViewerMagnification::FitInWindow,
            CampaignMagnification::FitWidth => PdfViewerMagnification::FitWidth,
            CampaignMagnification::FitVisible => PdfViewerMagnification::FitVisible,
            CampaignMagnification::Zoom125 => PdfViewerMagnification::Zoom(1.25),
        };
        options.viewer.initial_page = self.viewer.initial_page;
        options.viewer.hide_toolbar = self.viewer.hide_toolbar;
        options.viewer.hide_menubar = self.viewer.hide_menubar;
        options.viewer.hide_window_controls = self.viewer.hide_window_controls;
        options.viewer.fit_window = self.viewer.fit_window;
        options.viewer.center_window = self.viewer.center_window;
        options.viewer.display_document_title = self.viewer.display_document_title;
        options.viewer.full_screen = self.viewer.full_screen;
        options.viewer.first_page_left = self.viewer.first_page_left;
        options.metadata.title = self.metadata.title.clone();
        options.metadata.author = self.metadata.author.clone();
        options.metadata.subject = self.metadata.subject.clone();
        options.metadata.keywords = self.metadata.keywords.clone();
        options.metadata.creator = self.metadata.creator.clone();
        options.metadata.producer = self.metadata.producer.clone();
        options.metadata.creation_date = self.metadata.creation_date.map(pdf_date_time);
        if self.embed_campaign_attachment {
            options.attachments.push(PdfAttachment {
                path: "ooxmlsdk-campaign.txt".to_string(),
                mime_type: "text/plain".to_string(),
                description: "OOXMLSDK configured golden assignment identity".to_string(),
                association: PdfAttachmentAssociation::Data,
                data: Arc::from(format!("{CAMPAIGN_ID}\n{source_key}\n").into_bytes()),
                modification_date: self.metadata.creation_date.map(pdf_date_time),
                compress: Some(true),
            });
        }
        options
    }

    fn from_effective(options: &PdfOptions) -> Result<Self, String> {
        Ok(Self {
            standards: options
                .standards
                .iter()
                .copied()
                .map(CampaignPdfStandard::from_pdf)
                .collect::<Result<Vec<_>, _>>()?,
            compress_content_streams: options.compress_content_streams,
            ui_language: options.ui_language.clone().unwrap_or_default(),
            format_locale: options.format_locale.clone().unwrap_or_default(),
            document_language: options
                .default_document_language
                .clone()
                .unwrap_or_default(),
            field_update: match (
                options.field_update_datetime,
                options.field_update_time_zone.as_deref(),
            ) {
                (Some(value), Some(time_zone)) => Some(CampaignFieldUpdate {
                    year: value.year,
                    month: value.month,
                    day: value.day,
                    hour: value.hour,
                    minute: value.minute,
                    second: value.second,
                    time_zone: campaign_time_zone(time_zone)?.to_string(),
                }),
                (None, None) => None,
                state => {
                    return Err(format!(
                        "campaign cannot encode field update state {state:?}"
                    ));
                }
            },
            tagged_pdf: options.general.tagged_pdf,
            pdf_ua_compliance: options.general.pdf_ua_compliance,
            export_bookmarks: options.general.export_bookmarks,
            open_bookmark_levels: options.general.open_bookmark_levels,
            page_range: options.general.page_range.clone(),
            images: CampaignImageOptions {
                use_lossless_compression: options.images.use_lossless_compression,
                jpeg_quality: options.images.jpeg_quality,
                reduce_resolution: options.images.reduce_resolution,
                max_resolution_dpi: options.images.max_resolution_dpi,
            },
            link_action: match options.links.default_action {
                PdfLinkDefaultAction::Uri => CampaignLinkAction::Uri,
                PdfLinkDefaultAction::RemoveExternalLinks => {
                    CampaignLinkAction::RemoveExternalLinks
                }
                action => return Err(format!("campaign cannot encode link action {action:?}")),
            },
            export_form_fields: options.forms.export_form_fields,
            viewer: CampaignViewerOptions {
                page_mode: match options.viewer.page_mode {
                    PdfViewerPageMode::Default => CampaignViewerPageMode::Default,
                    PdfViewerPageMode::UseOutlines => CampaignViewerPageMode::UseOutlines,
                    PdfViewerPageMode::UseThumbs => CampaignViewerPageMode::UseThumbs,
                },
                page_layout: match options.viewer.page_layout {
                    PdfPageLayout::Default => CampaignPageLayout::Default,
                    PdfPageLayout::SinglePage => CampaignPageLayout::SinglePage,
                    PdfPageLayout::Continuous => CampaignPageLayout::Continuous,
                    PdfPageLayout::ContinuousFacing => CampaignPageLayout::ContinuousFacing,
                },
                magnification: match options.viewer.magnification {
                    PdfViewerMagnification::Default => CampaignMagnification::Default,
                    PdfViewerMagnification::FitInWindow => CampaignMagnification::FitInWindow,
                    PdfViewerMagnification::FitWidth => CampaignMagnification::FitWidth,
                    PdfViewerMagnification::FitVisible => CampaignMagnification::FitVisible,
                    PdfViewerMagnification::Zoom(1.25) => CampaignMagnification::Zoom125,
                    value => {
                        return Err(format!("campaign cannot encode magnification {value:?}"));
                    }
                },
                initial_page: options.viewer.initial_page,
                hide_toolbar: options.viewer.hide_toolbar,
                hide_menubar: options.viewer.hide_menubar,
                hide_window_controls: options.viewer.hide_window_controls,
                fit_window: options.viewer.fit_window,
                center_window: options.viewer.center_window,
                display_document_title: options.viewer.display_document_title,
                full_screen: options.viewer.full_screen,
                first_page_left: options.viewer.first_page_left,
            },
            metadata: CampaignMetadataOptions {
                title: options.metadata.title.clone(),
                author: options.metadata.author.clone(),
                subject: options.metadata.subject.clone(),
                keywords: options.metadata.keywords.clone(),
                creator: options.metadata.creator.clone(),
                producer: options.metadata.producer.clone(),
                creation_date: options.metadata.creation_date.map(campaign_date_time),
            },
            embed_campaign_attachment: !options.attachments.is_empty(),
        })
    }
}

fn pdf_date_time(value: CampaignDateTime) -> PdfDateTime {
    PdfDateTime {
        year: value.year,
        month: Some(value.month),
        day: Some(value.day),
        hour: Some(value.hour),
        minute: Some(value.minute),
        second: Some(value.second),
        utc_offset_hour: Some(value.utc_offset_hour),
        utc_offset_minute: Some(value.utc_offset_minute),
    }
}

fn campaign_date_time(value: PdfDateTime) -> CampaignDateTime {
    CampaignDateTime {
        year: value.year,
        month: value.month.expect("campaign emits a complete PDF date"),
        day: value.day.expect("campaign emits a complete PDF date"),
        hour: value.hour.expect("campaign emits a complete PDF date"),
        minute: value.minute.expect("campaign emits a complete PDF date"),
        second: value.second.expect("campaign emits a complete PDF date"),
        utc_offset_hour: value
            .utc_offset_hour
            .expect("campaign emits a complete PDF UTC offset"),
        utc_offset_minute: value
            .utc_offset_minute
            .expect("campaign emits a complete PDF UTC offset"),
    }
}

fn campaign_time_zone(value: &str) -> Result<&'static str, String> {
    match value {
        "Asia/Shanghai" => Ok("Asia/Shanghai"),
        "Asia/Tokyo" => Ok("Asia/Tokyo"),
        "Europe/Berlin" => Ok("Europe/Berlin"),
        "America/New_York" => Ok("America/New_York"),
        value => Err(format!("campaign cannot encode time zone {value:?}")),
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignAdjustment {
    pub feature: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeExportOptions {
    pub quality: String,
    pub include_document_properties: bool,
    pub page_from: Option<u32>,
    pub page_to: Option<u32>,
    pub tagged_pdf: bool,
    pub bookmarks: String,
    pub pdf_a_1: bool,
    pub bitmap_missing_fonts: bool,
    pub print_hidden_slides: bool,
}

impl OfficeExportOptions {
    fn candidate_optimize_for(&self) -> Option<PdfOptimizeFor> {
        match self.quality.as_str() {
            "print" => Some(PdfOptimizeFor::Print),
            "screen" => Some(PdfOptimizeFor::Screen),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignAssignment {
    pub schema_version: u32,
    pub campaign_id: String,
    pub assignment_algorithm: String,
    pub seed: String,
    pub corpus: String,
    pub file: String,
    pub family: OfficeFamily,
    pub source_extension: String,
    pub source_bytes: u64,
    pub source_sha256: String,
    pub configuration_id: String,
    pub pilot_300: bool,
    pub requested: CampaignPdfOptions,
    pub effective: CampaignPdfOptions,
    pub adjustments: Vec<CampaignAdjustment>,
    pub office: OfficeExportOptions,
}

impl CampaignAssignment {
    pub fn source_key(&self) -> String {
        format!("{}/{}", self.corpus, self.file)
    }

    pub fn source_path(&self, root: &Path) -> PathBuf {
        root.join("corpus").join(&self.corpus).join(&self.file)
    }

    pub fn requested_pdf_options(&self) -> Result<PdfOptions, String> {
        let mut options = self.requested.to_pdf_options(
            &self.source_key(),
            Path::new(&self.file)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&self.file),
        );
        options.optimize_for = self
            .office
            .candidate_optimize_for()
            .ok_or_else(|| format!("invalid Office quality for {}", self.source_key()))?;
        options.images.optimization_policy =
            PdfImageOptimizationPolicy::MicrosoftOfficeFixedOutput(match self.family {
                OfficeFamily::Word => PdfDocumentKind::Docx,
                OfficeFamily::Excel => PdfDocumentKind::Xlsx,
                OfficeFamily::PowerPoint => PdfDocumentKind::Pptx,
            });
        Ok(options)
    }
}

#[derive(Clone, Debug)]
struct SourceIdentity {
    corpus: String,
    file: String,
    family: OfficeFamily,
    source_extension: String,
    source_bytes: u64,
    source_sha256: String,
}

impl SourceIdentity {
    fn key(&self) -> String {
        format!("{}/{}", self.corpus, self.file)
    }
}

#[derive(Clone, Debug)]
struct AssignmentDraft {
    source: SourceIdentity,
    requested: CampaignPdfOptions,
    office: OfficeExportOptions,
    office_bookmark_kind: usize,
    pilot_300: bool,
}

pub fn generate_campaign_assignments(root: &Path) -> Result<Vec<CampaignAssignment>, String> {
    let sources = collect_round_trip_sources(root)?;
    if sources.len() != EXPECTED_ASSIGNMENT_COUNT {
        return Err(format!(
            "round-trip source count drifted: got {}, expected {EXPECTED_ASSIGNMENT_COUNT}",
            sources.len()
        ));
    }
    let mut drafts = sources
        .into_iter()
        .map(|source| {
            let source_key = source.key();
            AssignmentDraft {
                source,
                requested: CampaignPdfOptions::baseline(&source_key),
                office: OfficeExportOptions {
                    quality: "print".to_string(),
                    include_document_properties: false,
                    page_from: None,
                    page_to: None,
                    tagged_pdf: false,
                    bookmarks: "none".to_string(),
                    pdf_a_1: false,
                    bitmap_missing_fonts: true,
                    print_hidden_slides: false,
                },
                office_bookmark_kind: 0,
                pilot_300: false,
            }
        })
        .collect::<Vec<_>>();

    assign_balanced(&mut drafts, "standards", 8, |draft, value| {
        draft.requested.standards = match value {
            0 => vec![CampaignPdfStandard::Pdf14],
            1 => vec![CampaignPdfStandard::Pdf15],
            2 => vec![CampaignPdfStandard::Pdf16],
            3 => vec![CampaignPdfStandard::Pdf17],
            4 => vec![CampaignPdfStandard::Pdf20],
            5 => vec![CampaignPdfStandard::PdfA3a],
            6 => vec![CampaignPdfStandard::Pdf17, CampaignPdfStandard::PdfUa1],
            7 => vec![CampaignPdfStandard::PdfA3a, CampaignPdfStandard::PdfUa1],
            _ => unreachable!(),
        };
    });
    assign_balanced(&mut drafts, "content-compression", 2, |draft, value| {
        draft.requested.compress_content_streams = value == 0;
    });
    assign_balanced(&mut drafts, "ui-language", LOCALES.len(), |draft, value| {
        draft.requested.ui_language = LOCALES[value].to_string();
    });
    assign_balanced(
        &mut drafts,
        "format-locale",
        LOCALES.len(),
        |draft, value| {
            draft.requested.format_locale = LOCALES[value].to_string();
        },
    );
    assign_balanced(
        &mut drafts,
        "document-language",
        LOCALES.len(),
        |draft, value| {
            draft.requested.document_language = LOCALES[value].to_string();
        },
    );
    assign_balanced(&mut drafts, "field-update", 5, |draft, value| {
        draft.requested.field_update = match value {
            0 => None,
            1 => Some(field_update("Asia/Shanghai")),
            2 => Some(field_update("Asia/Tokyo")),
            3 => Some(field_update("Europe/Berlin")),
            4 => Some(field_update("America/New_York")),
            _ => unreachable!(),
        };
    });
    assign_balanced(&mut drafts, "tagged-pdf", 2, |draft, value| {
        draft.requested.tagged_pdf = value == 0;
    });
    assign_balanced(&mut drafts, "bookmarks", 2, |draft, value| {
        draft.requested.export_bookmarks = value == 0;
    });
    assign_balanced(&mut drafts, "open-bookmark-levels", 3, |draft, value| {
        draft.requested.open_bookmark_levels = match value {
            0 => None,
            1 => Some(0),
            2 => Some(2),
            _ => unreachable!(),
        };
    });
    assign_balanced(&mut drafts, "page-range", 2, |draft, value| {
        draft.requested.page_range = (value == 0).then(|| "1".to_string());
    });
    assign_balanced(&mut drafts, "image-compression", 4, |draft, value| {
        let (lossless, quality) = match value {
            0 => (false, Some(60)),
            1 => (false, Some(75)),
            2 => (false, Some(90)),
            3 => (true, Some(90)),
            _ => unreachable!(),
        };
        draft.requested.images.use_lossless_compression = lossless;
        draft.requested.images.jpeg_quality = quality;
    });
    assign_balanced(&mut drafts, "image-downsampling", 4, |draft, value| {
        let (reduce, dpi) = match value {
            0 => (false, Some(600)),
            1 => (true, Some(96)),
            2 => (true, Some(150)),
            3 => (true, Some(300)),
            _ => unreachable!(),
        };
        draft.requested.images.reduce_resolution = reduce;
        draft.requested.images.max_resolution_dpi = dpi;
    });
    assign_balanced(&mut drafts, "link-action", 2, |draft, value| {
        draft.requested.link_action = if value == 0 {
            CampaignLinkAction::Uri
        } else {
            CampaignLinkAction::RemoveExternalLinks
        };
    });
    assign_balanced_for_family(
        &mut drafts,
        OfficeFamily::Word,
        "form-fields",
        2,
        |draft, value| draft.requested.export_form_fields = value == 0,
    );
    assign_balanced(&mut drafts, "viewer-page-mode", 3, |draft, value| {
        draft.requested.viewer.page_mode = match value {
            0 => CampaignViewerPageMode::Default,
            1 => CampaignViewerPageMode::UseOutlines,
            2 => CampaignViewerPageMode::UseThumbs,
            _ => unreachable!(),
        };
    });
    assign_balanced(&mut drafts, "viewer-layout", 4, |draft, value| {
        draft.requested.viewer.page_layout = match value {
            0 => CampaignPageLayout::Default,
            1 => CampaignPageLayout::SinglePage,
            2 => CampaignPageLayout::Continuous,
            3 => CampaignPageLayout::ContinuousFacing,
            _ => unreachable!(),
        };
        draft.requested.viewer.first_page_left = value == 3;
    });
    assign_balanced(&mut drafts, "viewer-magnification", 5, |draft, value| {
        draft.requested.viewer.magnification = match value {
            0 => CampaignMagnification::Default,
            1 => CampaignMagnification::FitInWindow,
            2 => CampaignMagnification::FitWidth,
            3 => CampaignMagnification::FitVisible,
            4 => CampaignMagnification::Zoom125,
            _ => unreachable!(),
        };
    });
    assign_balanced(&mut drafts, "viewer-flags", 8, |draft, value| {
        draft.requested.viewer.hide_toolbar = value & 1 != 0;
        draft.requested.viewer.hide_menubar = value & 2 != 0;
        draft.requested.viewer.center_window = value & 4 != 0;
        draft.requested.viewer.hide_window_controls = value % 3 == 0;
        draft.requested.viewer.fit_window = value % 3 == 1;
        draft.requested.viewer.display_document_title = value % 4 != 0;
        draft.requested.viewer.full_screen = value == 7;
    });
    assign_balanced(&mut drafts, "metadata", 4, |draft, value| {
        let source_key = draft.source.key();
        draft.requested.metadata = match value {
            0 => CampaignMetadataOptions::default(),
            1 => CampaignMetadataOptions {
                title: Some(format!("OOXMLSDK configured golden: {source_key}")),
                ..Default::default()
            },
            2 => CampaignMetadataOptions {
                title: Some(format!("OOXMLSDK configured golden: {source_key}")),
                author: Some("ooxmlsdk-test-suite".to_string()),
                subject: Some("Configured Office PDF fidelity".to_string()),
                keywords: Some("ooxmlsdk,office,golden".to_string()),
                ..Default::default()
            },
            3 => CampaignMetadataOptions {
                creator: Some("ooxmlsdk-pdf".to_string()),
                producer: Some("ooxmlsdk-pdf configured campaign".to_string()),
                ..Default::default()
            },
            _ => unreachable!(),
        };
    });
    assign_balanced(&mut drafts, "attachments", 2, |draft, value| {
        draft.requested.embed_campaign_attachment = value == 0;
    });
    assign_balanced(&mut drafts, "office-quality", 2, |draft, value| {
        draft.office.quality = if value == 0 { "print" } else { "screen" }.to_string();
    });
    assign_balanced(
        &mut drafts,
        "office-document-properties",
        2,
        |draft, value| draft.office.include_document_properties = value == 0,
    );
    assign_balanced(&mut drafts, "office-bitmap-fonts", 2, |draft, value| {
        draft.office.bitmap_missing_fonts = value == 0;
    });
    assign_balanced_for_family(
        &mut drafts,
        OfficeFamily::Word,
        "office-bookmarks",
        2,
        |draft, value| draft.office_bookmark_kind = value,
    );
    assign_balanced_for_family(
        &mut drafts,
        OfficeFamily::PowerPoint,
        "office-hidden-slides",
        2,
        |draft, value| draft.office.print_hidden_slides = value == 0,
    );

    for family in OfficeFamily::ALL {
        for wants_page_range in [true, false] {
            let mut indices = drafts
                .iter()
                .enumerate()
                .filter(|(_, draft)| {
                    draft.source.family == family
                        && draft.requested.page_range.is_some() == wants_page_range
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            indices.sort_by_key(|index| {
                assignment_digest("pilot-selection", &drafts[*index].source.key())
            });
            for index in indices.into_iter().take(PILOT_PER_FAMILY / 2) {
                drafts[index].pilot_300 = true;
            }
        }
    }

    let mut assignments = Vec::with_capacity(drafts.len());
    for mut draft in drafts {
        if draft
            .requested
            .standards
            .contains(&CampaignPdfStandard::PdfA3a)
        {
            draft.requested.metadata.creation_date = Some(fixed_creation_date());
        }
        let source_key = draft.source.key();
        let source_file_name = Path::new(&draft.source.file)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("source has no UTF-8 file name: {source_key}"))?;
        let requested_options = draft
            .requested
            .to_pdf_options(&source_key, source_file_name);
        let resolved = resolve_pdf_options(draft.source.family.document_kind(), &requested_options)
            .map_err(|error| format!("could not resolve assignment for {source_key}: {error}"))?;
        let effective = CampaignPdfOptions::from_effective(&resolved.effective)?;
        let adjustments = resolved
            .adjustments
            .iter()
            .map(|adjustment| CampaignAdjustment {
                feature: adjustment.feature.to_string(),
                reason: adjustment.reason.to_string(),
            })
            .collect::<Vec<_>>();

        draft.office.page_from = effective.page_range.as_ref().map(|_| 1);
        draft.office.page_to = effective.page_range.as_ref().map(|_| 1);
        draft.office.tagged_pdf = match draft.source.family {
            OfficeFamily::Excel => true,
            _ => effective.tagged_pdf,
        };
        draft.office.pdf_a_1 = draft.source.family != OfficeFamily::Excel
            && effective.standards.contains(&CampaignPdfStandard::PdfA3a);
        draft.office.bookmarks =
            if draft.source.family == OfficeFamily::Word && effective.export_bookmarks {
                if draft.office_bookmark_kind == 0 {
                    "headings"
                } else {
                    "word-bookmarks"
                }
                .to_string()
            } else {
                "none".to_string()
            };
        if draft.source.family == OfficeFamily::Excel {
            draft.office.bitmap_missing_fonts = true;
        }

        let configuration_id = configuration_id(
            &draft.source,
            &draft.requested,
            &effective,
            &adjustments,
            &draft.office,
        )?;
        assignments.push(CampaignAssignment {
            schema_version: CAMPAIGN_SCHEMA_VERSION,
            campaign_id: CAMPAIGN_ID.to_string(),
            assignment_algorithm: ASSIGNMENT_ALGORITHM.to_string(),
            seed: CAMPAIGN_SEED.to_string(),
            corpus: draft.source.corpus,
            file: draft.source.file,
            family: draft.source.family,
            source_extension: draft.source.source_extension,
            source_bytes: draft.source.source_bytes,
            source_sha256: draft.source.source_sha256,
            configuration_id,
            pilot_300: draft.pilot_300,
            requested: draft.requested,
            effective,
            adjustments,
            office: draft.office,
        });
    }
    assignments.sort_by(|left, right| {
        (&left.family, &left.corpus, &left.file).cmp(&(&right.family, &right.corpus, &right.file))
    });
    validate_assignments(root, &assignments)?;
    Ok(assignments)
}

fn field_update(time_zone: &str) -> CampaignFieldUpdate {
    CampaignFieldUpdate {
        year: 2026,
        month: 8,
        day: 18,
        hour: 12,
        minute: 34,
        second: 56,
        time_zone: time_zone.to_string(),
    }
}

fn fixed_creation_date() -> CampaignDateTime {
    CampaignDateTime {
        year: 2026,
        month: 8,
        day: 18,
        hour: 12,
        minute: 0,
        second: 0,
        utc_offset_hour: 8,
        utc_offset_minute: 0,
    }
}

fn assign_balanced(
    drafts: &mut [AssignmentDraft],
    feature: &str,
    cardinality: usize,
    mut assign: impl FnMut(&mut AssignmentDraft, usize),
) {
    for family in OfficeFamily::ALL {
        let mut indices = drafts
            .iter()
            .enumerate()
            .filter(|(_, draft)| draft.source.family == family)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        indices.sort_by_key(|index| assignment_digest(feature, &drafts[*index].source.key()));
        for (position, index) in indices.into_iter().enumerate() {
            assign(&mut drafts[index], position % cardinality);
        }
    }
}

fn assign_balanced_for_family(
    drafts: &mut [AssignmentDraft],
    family: OfficeFamily,
    feature: &str,
    cardinality: usize,
    mut assign: impl FnMut(&mut AssignmentDraft, usize),
) {
    let mut indices = drafts
        .iter()
        .enumerate()
        .filter(|(_, draft)| draft.source.family == family)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    indices.sort_by_key(|index| assignment_digest(feature, &drafts[*index].source.key()));
    for (position, index) in indices.into_iter().enumerate() {
        assign(&mut drafts[index], position % cardinality);
    }
}

fn assignment_digest(feature: &str, source_key: &str) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(CAMPAIGN_SEED.as_bytes());
    digest.update(b"\0");
    digest.update(feature.as_bytes());
    digest.update(b"\0");
    digest.update(source_key.as_bytes());
    digest.finalize().into()
}

fn configuration_id(
    source: &SourceIdentity,
    requested: &CampaignPdfOptions,
    effective: &CampaignPdfOptions,
    adjustments: &[CampaignAdjustment],
    office: &OfficeExportOptions,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct ConfigurationIdentity<'a> {
        campaign_id: &'static str,
        source: &'a str,
        source_sha256: &'a str,
        requested: &'a CampaignPdfOptions,
        effective: &'a CampaignPdfOptions,
        adjustments: &'a [CampaignAdjustment],
        office: &'a OfficeExportOptions,
    }
    let source_key = source.key();
    let json = serde_json::to_vec(&ConfigurationIdentity {
        campaign_id: CAMPAIGN_ID,
        source: &source_key,
        source_sha256: &source.source_sha256,
        requested,
        effective,
        adjustments,
        office,
    })
    .map_err(|error| format!("could not serialize configuration identity: {error}"))?;
    Ok(sha256_bytes(&json))
}

#[derive(Deserialize)]
struct WorkspaceManifest {
    #[serde(default)]
    corpus: Vec<WorkspaceCorpus>,
}

#[derive(Deserialize)]
struct WorkspaceCorpus {
    path: String,
    manifest: String,
}

#[derive(Deserialize)]
struct LocalCorpusManifest {
    #[serde(default)]
    expectation: Vec<LocalExpectation>,
}

#[derive(Deserialize)]
struct LocalExpectation {
    file: String,
    test: String,
    mode: String,
}

fn collect_round_trip_sources(root: &Path) -> Result<Vec<SourceIdentity>, String> {
    let workspace_manifest_path = root.join("corpus-manifest.toml");
    let raw = fs::read_to_string(&workspace_manifest_path).map_err(|error| {
        format!(
            "could not read workspace corpus manifest {}: {error}",
            workspace_manifest_path.display()
        )
    })?;
    let workspace_manifest: WorkspaceManifest = toml::from_str(&raw).map_err(|error| {
        format!(
            "could not parse workspace corpus manifest {}: {error}",
            workspace_manifest_path.display()
        )
    })?;
    let mut sources = Vec::new();
    for corpus in workspace_manifest.corpus {
        let corpus_dir = root.join(&corpus.path);
        let corpus_name = corpus_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid corpus directory {}", corpus_dir.display()))?
            .to_string();
        let manifest_path = root.join(&corpus.manifest);
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
        let manifest: LocalCorpusManifest = toml::from_str(&raw)
            .map_err(|error| format!("could not parse {}: {error}", manifest_path.display()))?;
        let expectations = manifest
            .expectation
            .into_iter()
            .filter(|expectation| expectation.test == "roundtrip")
            .map(|expectation| (expectation.file, expectation.mode))
            .collect::<BTreeMap<_, _>>();
        if !corpus_dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&corpus_dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy();
            if file_name.starts_with("~$") {
                continue;
            }
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let extension = extension.to_ascii_lowercase();
            let Some(family) = OfficeFamily::from_extension(&extension) else {
                continue;
            };
            let relative = path
                .strip_prefix(&corpus_dir)
                .map_err(|error| format!("could not relativize {}: {error}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            let mode = expectations
                .get(&relative)
                .map(String::as_str)
                .unwrap_or("round_trip");
            if mode != "round_trip" {
                continue;
            }
            let metadata = fs::metadata(path)
                .map_err(|error| format!("could not stat {}: {error}", path.display()))?;
            sources.push(SourceIdentity {
                corpus: corpus_name.clone(),
                file: relative,
                family,
                source_extension: extension,
                source_bytes: metadata.len(),
                source_sha256: sha256_file(path)?,
            });
        }
    }
    sources.sort_by(|left, right| {
        (&left.family, &left.corpus, &left.file).cmp(&(&right.family, &right.corpus, &right.file))
    });
    Ok(sources)
}

pub fn write_plan(path: &Path, assignments: &[CampaignAssignment]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension("jsonl.tmp");
    let file = File::create(&temporary)
        .map_err(|error| format!("could not create {}: {error}", temporary.display()))?;
    let mut writer = BufWriter::new(file);
    for assignment in assignments {
        serde_json::to_writer(&mut writer, assignment)
            .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    }
    writer
        .flush()
        .map_err(|error| format!("could not flush {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "could not promote {} to {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}

pub fn read_plan(path: &Path) -> Result<Vec<CampaignAssignment>, String> {
    let file = File::open(path)
        .map_err(|error| format!("could not open plan {}: {error}", path.display()))?;
    let mut assignments = Vec::new();
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        assignments.push(serde_json::from_str(&line).map_err(|error| {
            format!(
                "invalid assignment JSON at {}:{}: {error}",
                path.display(),
                line_index + 1
            )
        })?);
    }
    Ok(assignments)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlanValidationSummary {
    pub assignments: usize,
    pub pilot_assignments: usize,
    pub family_assignments: BTreeMap<String, usize>,
    pub pilot_family_assignments: BTreeMap<String, usize>,
}

pub fn validate_assignments(
    root: &Path,
    assignments: &[CampaignAssignment],
) -> Result<PlanValidationSummary, String> {
    if assignments.len() != EXPECTED_ASSIGNMENT_COUNT {
        return Err(format!(
            "plan contains {} assignments, expected {EXPECTED_ASSIGNMENT_COUNT}",
            assignments.len()
        ));
    }
    let mut sources = BTreeSet::new();
    let mut configurations = BTreeSet::new();
    let mut family_assignments = BTreeMap::new();
    let mut pilot_family_assignments = BTreeMap::new();
    let mut previous = None;
    for assignment in assignments {
        if assignment.schema_version != CAMPAIGN_SCHEMA_VERSION
            || assignment.campaign_id != CAMPAIGN_ID
            || assignment.assignment_algorithm != ASSIGNMENT_ALGORITHM
            || assignment.seed != CAMPAIGN_SEED
        {
            return Err(format!(
                "campaign metadata drift for {}",
                assignment.source_key()
            ));
        }
        let key = assignment.source_key();
        if !sources.insert(key.clone()) {
            return Err(format!("duplicate source identity in campaign plan: {key}"));
        }
        if !configurations.insert(assignment.configuration_id.clone()) {
            return Err(format!(
                "duplicate configuration identity in campaign plan: {}",
                assignment.configuration_id
            ));
        }
        let order_key = (&assignment.family, &assignment.corpus, &assignment.file);
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &order_key)
        {
            return Err(format!("campaign plan is not strictly sorted at {key}"));
        }
        previous = Some(order_key);
        let source_path = assignment.source_path(root);
        let metadata = fs::metadata(&source_path)
            .map_err(|error| format!("could not stat {}: {error}", source_path.display()))?;
        if metadata.len() != assignment.source_bytes {
            return Err(format!("source byte length drift for {key}"));
        }
        let source_sha256 = sha256_file(&source_path)?;
        if source_sha256 != assignment.source_sha256 {
            return Err(format!("source SHA-256 drift for {key}"));
        }
        let source = SourceIdentity {
            corpus: assignment.corpus.clone(),
            file: assignment.file.clone(),
            family: assignment.family,
            source_extension: assignment.source_extension.clone(),
            source_bytes: assignment.source_bytes,
            source_sha256: assignment.source_sha256.clone(),
        };
        let source_file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("source has no UTF-8 file name: {key}"))?;
        let requested = assignment.requested.to_pdf_options(&key, source_file_name);
        let resolved = resolve_pdf_options(assignment.family.document_kind(), &requested).map_err(
            |error| format!("requested configuration no longer resolves for {key}: {error}"),
        )?;
        let effective = CampaignPdfOptions::from_effective(&resolved.effective)?;
        let adjustments = resolved
            .adjustments
            .iter()
            .map(|adjustment| CampaignAdjustment {
                feature: adjustment.feature.to_string(),
                reason: adjustment.reason.to_string(),
            })
            .collect::<Vec<_>>();
        if effective != assignment.effective || adjustments != assignment.adjustments {
            return Err(format!("resolved configuration drift for {key}"));
        }
        let expected_configuration = configuration_id(
            &source,
            &assignment.requested,
            &assignment.effective,
            &assignment.adjustments,
            &assignment.office,
        )?;
        if expected_configuration != assignment.configuration_id {
            return Err(format!("configuration identity drift for {key}"));
        }
        validate_office_options(assignment)?;
        *family_assignments
            .entry(assignment.family.as_str().to_string())
            .or_insert(0) += 1;
        if assignment.pilot_300 {
            *pilot_family_assignments
                .entry(assignment.family.as_str().to_string())
                .or_insert(0) += 1;
        }
    }
    for family in OfficeFamily::ALL {
        let pilot = pilot_family_assignments
            .get(family.as_str())
            .copied()
            .unwrap_or_default();
        if pilot != PILOT_PER_FAMILY {
            return Err(format!(
                "pilot selection has {pilot} {} assignments, expected {PILOT_PER_FAMILY}",
                family.as_str()
            ));
        }
    }
    Ok(PlanValidationSummary {
        assignments: assignments.len(),
        pilot_assignments: assignments
            .iter()
            .filter(|assignment| assignment.pilot_300)
            .count(),
        family_assignments,
        pilot_family_assignments,
    })
}

fn validate_office_options(assignment: &CampaignAssignment) -> Result<(), String> {
    let options = &assignment.office;
    if options.candidate_optimize_for().is_none() {
        return Err(format!(
            "invalid Office quality for {}",
            assignment.source_key()
        ));
    }
    if options.page_from.is_some() != options.page_to.is_some()
        || options
            .page_from
            .zip(options.page_to)
            .is_some_and(|(from, to)| from == 0 || to < from)
    {
        return Err(format!(
            "invalid Office page range for {}",
            assignment.source_key()
        ));
    }
    match assignment.family {
        OfficeFamily::Word => {
            if !matches!(
                options.bookmarks.as_str(),
                "none" | "headings" | "word-bookmarks"
            ) || options.print_hidden_slides
            {
                return Err(format!(
                    "invalid Word Office options for {}",
                    assignment.source_key()
                ));
            }
        }
        OfficeFamily::Excel => {
            if options.bookmarks != "none"
                || options.pdf_a_1
                || !options.tagged_pdf
                || !options.bitmap_missing_fonts
                || options.print_hidden_slides
            {
                return Err(format!(
                    "invalid Excel Office options for {}",
                    assignment.source_key()
                ));
            }
        }
        OfficeFamily::PowerPoint => {
            if options.bookmarks != "none" {
                return Err(format!(
                    "invalid PowerPoint Office options for {}",
                    assignment.source_key()
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObservedPdfFacts {
    pub header_version: String,
    pub page_count: u32,
    pub tagged_pdf: bool,
    pub has_outlines: bool,
    pub pdf_a_part: Option<u32>,
    pub pdf_a_conformance: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CampaignConversionRecord {
    pub schema_version: u32,
    pub campaign_id: String,
    pub assignment_algorithm: String,
    pub file: String,
    pub family: OfficeFamily,
    pub source_extension: String,
    pub source_bytes: u64,
    pub source_sha256: String,
    pub configuration_id: String,
    pub requested: CampaignPdfOptions,
    pub effective: CampaignPdfOptions,
    pub adjustments: Vec<CampaignAdjustment>,
    pub office_options: OfficeExportOptions,
    pub status: String,
    pub reference_engine: String,
    pub environment_id: String,
    pub application: String,
    pub application_version: String,
    pub application_build: String,
    pub output: String,
    pub output_bytes: u64,
    pub output_sha256: String,
    pub observed_pdf: Option<ObservedPdfFacts>,
    pub converted_at_utc: String,
    pub elapsed_ms: u64,
    pub attempts: u32,
    #[serde(default)]
    pub annotations: Vec<serde_json::Value>,
    pub failure_class: String,
    pub error_code: String,
    pub error: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CampaignConversionSummary {
    pub selected: usize,
    pub converted: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub skipped: usize,
    pub attempts: usize,
    pub family_statuses: BTreeMap<String, BTreeMap<String, usize>>,
    pub environment_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeResult {
    schema_version: u32,
    file: String,
    application: String,
    application_version: String,
    application_build: String,
    options: OfficeExportOptions,
    source_sha256: String,
    output: String,
    output_bytes: u64,
    output_sha256: String,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct ProbePlanRecord<'a> {
    schema_version: u32,
    file: String,
    options: &'a OfficeExportOptions,
}

#[derive(Deserialize)]
struct EnvironmentDocument {
    environment_id: String,
}

struct PendingConversion {
    assignment: CampaignAssignment,
    attempt: u32,
    elapsed_ms: u64,
}

struct RunningConversion {
    child: Child,
    pending: PendingConversion,
    started: Instant,
    worker_root: PathBuf,
    result_path: PathBuf,
    pdf_path: PathBuf,
    process_id_path: PathBuf,
}

enum ConversionAttempt {
    Converted(Box<ProbeResult>, Vec<u8>),
    Failed(String),
    TimedOut(String),
}

pub fn convert_campaign(
    root: &Path,
    plan_path: &Path,
    pwsh: &str,
    pilot_only: bool,
    jobs: usize,
    timeout: Duration,
    max_attempts: u32,
) -> Result<CampaignConversionSummary, String> {
    if jobs == 0 || max_attempts == 0 {
        return Err("conversion jobs and max attempts must be positive".to_string());
    }
    let assignments = read_plan(plan_path)?;
    validate_assignments(root, &assignments)?;
    let selected = assignments
        .into_iter()
        .filter(|assignment| !pilot_only || assignment.pilot_300)
        .collect::<Vec<_>>();
    let expected = if pilot_only {
        PILOT_ASSIGNMENT_COUNT
    } else {
        EXPECTED_ASSIGNMENT_COUNT
    };
    if selected.len() != expected {
        return Err(format!(
            "conversion selection contains {} assignments, expected {expected}",
            selected.len()
        ));
    }

    let conversion_root = root.join("corpus_pdf_conv");
    fs::create_dir_all(&conversion_root).map_err(|error| {
        format!(
            "could not create conversion root {}: {error}",
            conversion_root.display()
        )
    })?;
    let plan_sha256 = sha256_file(plan_path)?;
    let probe_script = root.join("scripts/probe_office_pdf_options.ps1");
    let environment_script = root.join("scripts/probe_office_pdf_campaign_environment.ps1");
    let probe_sha256 = sha256_file(&probe_script)?;
    let work_root = PathBuf::from(format!(
        "/tmp/ooxmlsdk-office-pdf-campaign-{}",
        std::process::id()
    ));
    if work_root.exists() {
        let preserved = PathBuf::from(format!(
            "/tmp/ooxmlsdk-preserved-office-pdf-campaign-{}-{}",
            std::process::id(),
            jiff::Timestamp::now().as_nanosecond()
        ));
        fs::rename(&work_root, &preserved).map_err(|error| {
            format!(
                "could not preserve stale work root {} as {}: {error}",
                work_root.display(),
                preserved.display()
            )
        })?;
    }
    fs::create_dir_all(&work_root)
        .map_err(|error| format!("could not create {}: {error}", work_root.display()))?;

    let environment_path = work_root.join("environment.json");
    probe_reference_environment(
        pwsh,
        &environment_script,
        &environment_path,
        &plan_sha256,
        &probe_sha256,
    )?;
    let environment_bytes = fs::read(&environment_path)
        .map_err(|error| format!("could not read {}: {error}", environment_path.display()))?;
    let environment: EnvironmentDocument = serde_json::from_slice(&environment_bytes)
        .map_err(|error| format!("invalid {}: {error}", environment_path.display()))?;
    if environment.environment_id.len() != 64 {
        return Err("reference environment id is not a SHA-256 value".to_string());
    }

    let mut records = read_conversion_records(root)?;
    if let Some(existing_environment_id) = records
        .values()
        .map(|record| record.environment_id.as_str())
        .next()
        && existing_environment_id != environment.environment_id
    {
        return Err(format!(
            "refusing to mix reference environments: existing={existing_environment_id}, current={}",
            environment.environment_id
        ));
    }
    let promoted_environment_path = conversion_root.join("environment.json");
    if promoted_environment_path.exists() {
        let promoted_environment_bytes = fs::read(&promoted_environment_path).map_err(|error| {
            format!(
                "could not read promoted environment {}: {error}",
                promoted_environment_path.display()
            )
        })?;
        let promoted_environment: EnvironmentDocument =
            serde_json::from_slice(&promoted_environment_bytes).map_err(|error| {
                format!(
                    "invalid promoted environment {}: {error}",
                    promoted_environment_path.display()
                )
            })?;
        if promoted_environment.environment_id != environment.environment_id {
            return Err(format!(
                "refusing to replace reference environment: existing={}, current={}",
                promoted_environment.environment_id, environment.environment_id
            ));
        }
    } else {
        promote_bytes(&promoted_environment_path, &environment_bytes)?;
    }

    let mut pending = VecDeque::new();
    let mut skipped = 0;
    for assignment in selected {
        let key = (assignment.corpus.clone(), assignment.file.clone());
        if let Some(record) = records.get(&key) {
            validate_conversion_record(&assignment, record)?;
            if record.environment_id != environment.environment_id {
                return Err(format!("environment drift for {}", assignment.source_key()));
            }
            if record.status == "converted" {
                let output = conversion_root
                    .join(&assignment.corpus)
                    .join(&record.output);
                let bytes = fs::read(&output).map_err(|error| {
                    format!(
                        "could not read existing golden {}: {error}",
                        output.display()
                    )
                })?;
                if bytes.len() as u64 != record.output_bytes
                    || sha256_bytes(&bytes) != record.output_sha256
                {
                    return Err(format!(
                        "existing golden identity drift for {}",
                        assignment.source_key()
                    ));
                }
            }
            skipped += 1;
            continue;
        }
        pending.push_back(PendingConversion {
            assignment,
            attempt: 1,
            elapsed_ms: 0,
        });
    }

    let corpus_path = windows_path(&root.join("corpus"))?;
    let probe_script_path = windows_path(&probe_script)?;
    let mut running = Vec::<RunningConversion>::new();
    let mut attempts = 0;
    while !pending.is_empty() || !running.is_empty() {
        let mut deferred = VecDeque::new();
        while running.len() < jobs {
            let Some(task) = pending.pop_front() else {
                break;
            };
            if running
                .iter()
                .any(|worker| worker.pending.assignment.family == task.assignment.family)
            {
                deferred.push_back(task);
                continue;
            }
            attempts += 1;
            running.push(spawn_conversion_worker(
                pwsh,
                &probe_script_path,
                &corpus_path,
                &work_root,
                task,
            )?);
        }
        pending.extend(deferred);
        if running.is_empty() {
            return Err("conversion scheduler made no progress".to_string());
        }

        let mut index = 0;
        while index < running.len() {
            let status = running[index]
                .child
                .try_wait()
                .map_err(|error| format!("could not poll Office worker: {error}"))?;
            let attempt = if let Some(status) = status {
                let worker = running.swap_remove(index);
                let success = status.success();
                if !success {
                    stop_office_process(pwsh, &worker.process_id_path);
                }
                finish_conversion_worker(worker, success, false)?
            } else if running[index].started.elapsed() >= timeout {
                let mut worker = running.swap_remove(index);
                let _ = worker.child.kill();
                let _ = worker.child.wait();
                stop_office_process(pwsh, &worker.process_id_path);
                finish_conversion_worker(worker, false, true)?
            } else {
                index += 1;
                continue;
            };
            let (mut task, outcome) = attempt;
            match outcome {
                ConversionAttempt::Converted(result, pdf) => {
                    let record = converted_record(
                        root,
                        &task.assignment,
                        &environment.environment_id,
                        task.attempt,
                        task.elapsed_ms,
                        *result,
                        &pdf,
                    )?;
                    records.insert(
                        (task.assignment.corpus.clone(), task.assignment.file.clone()),
                        record,
                    );
                    write_conversion_manifests(root, &records)?;
                    println!(
                        "converted|{}|{}|attempt={}",
                        task.assignment.family,
                        task.assignment.source_key(),
                        task.attempt
                    );
                }
                ConversionAttempt::Failed(message) | ConversionAttempt::TimedOut(message)
                    if task.attempt < max_attempts =>
                {
                    println!(
                        "retry|{}|{}|attempt={}|{}",
                        task.assignment.family,
                        task.assignment.source_key(),
                        task.attempt,
                        one_line(&message)
                    );
                    task.attempt += 1;
                    pending.push_back(task);
                }
                ConversionAttempt::Failed(message) => {
                    let failure_class = if message.starts_with("invalid Office PDF:") {
                        "invalid-pdf-output"
                    } else {
                        "office-conversion-error"
                    };
                    let record = failed_record(
                        &task.assignment,
                        &environment.environment_id,
                        "failed",
                        failure_class,
                        task.attempt,
                        task.elapsed_ms,
                        message,
                    );
                    records.insert(
                        (task.assignment.corpus.clone(), task.assignment.file.clone()),
                        record,
                    );
                    write_conversion_manifests(root, &records)?;
                    println!(
                        "failed|{}|{}",
                        task.assignment.family,
                        task.assignment.source_key()
                    );
                }
                ConversionAttempt::TimedOut(message) => {
                    let record = failed_record(
                        &task.assignment,
                        &environment.environment_id,
                        "timeout",
                        "office-timeout",
                        task.attempt,
                        task.elapsed_ms,
                        message,
                    );
                    records.insert(
                        (task.assignment.corpus.clone(), task.assignment.file.clone()),
                        record,
                    );
                    write_conversion_manifests(root, &records)?;
                    println!(
                        "timeout|{}|{}",
                        task.assignment.family,
                        task.assignment.source_key()
                    );
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    summarize_conversions(
        root,
        plan_path,
        pilot_only,
        skipped,
        attempts,
        environment.environment_id,
    )
}

fn probe_reference_environment(
    pwsh: &str,
    script: &Path,
    output_path: &Path,
    plan_sha256: &str,
    probe_script_sha256: &str,
) -> Result<(), String> {
    let script = windows_path(script)?;
    let output_path = windows_path(output_path)?;
    let output = Command::new(pwsh)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script,
            "-OutputPath",
            &output_path,
            "-PlanSha256",
            plan_sha256,
            "-ProbeScriptSha256",
            probe_script_sha256,
        ])
        .output()
        .map_err(|error| format!("could not start PowerShell environment probe: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "PowerShell environment probe failed: {}",
            command_output_message(&output.stdout, &output.stderr)
        ));
    }
    Ok(())
}

fn spawn_conversion_worker(
    pwsh: &str,
    probe_script_path: &str,
    corpus_path: &str,
    work_root: &Path,
    pending: PendingConversion,
) -> Result<RunningConversion, String> {
    let worker_root = work_root.join(format!(
        "{}-attempt-{}",
        pending.assignment.configuration_id, pending.attempt
    ));
    let output_root = worker_root.join("output");
    fs::create_dir_all(&output_root)
        .map_err(|error| format!("could not create {}: {error}", output_root.display()))?;
    let plan_path = worker_root.join("plan.jsonl");
    let process_id_path = worker_root.join("office-process-id.txt");
    write_json_lines(
        &plan_path,
        &[ProbePlanRecord {
            schema_version: 1,
            file: pending.assignment.source_key(),
            options: &pending.assignment.office,
        }],
    )?;
    let plan_path = windows_path(&plan_path)?;
    let output_root_path = windows_path(&output_root)?;
    let process_id_file = windows_path(&process_id_path)?;
    let child = Command::new(pwsh)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            probe_script_path,
            "-CorpusRoot",
            corpus_path,
            "-PlanFile",
            &plan_path,
            "-OutputRoot",
            &output_root_path,
            "-ProcessIdFile",
            &process_id_file,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "could not start Office worker for {}: {error}",
                pending.assignment.source_key()
            )
        })?;
    Ok(RunningConversion {
        child,
        pending,
        started: Instant::now(),
        result_path: output_root.join("case-000.json"),
        pdf_path: output_root.join("case-000.pdf"),
        process_id_path,
        worker_root,
    })
}

fn finish_conversion_worker(
    worker: RunningConversion,
    success: bool,
    timed_out: bool,
) -> Result<(PendingConversion, ConversionAttempt), String> {
    let RunningConversion {
        child,
        mut pending,
        started,
        worker_root,
        result_path,
        pdf_path,
        process_id_path: _,
    } = worker;
    let output = child.wait_with_output().map_err(|error| {
        format!(
            "could not collect Office worker for {}: {error}",
            pending.assignment.source_key()
        )
    })?;
    pending.elapsed_ms = pending
        .elapsed_ms
        .saturating_add(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
    let message = command_output_message(&output.stdout, &output.stderr);
    let attempt = if timed_out {
        ConversionAttempt::TimedOut(format!(
            "Office worker exceeded its per-source timeout; {message}"
        ))
    } else if !success {
        ConversionAttempt::Failed(message)
    } else {
        let result_bytes = fs::read(&result_path)
            .map_err(|error| format!("could not read {}: {error}", result_path.display()))?;
        let result: ProbeResult = serde_json::from_slice(&result_bytes)
            .map_err(|error| format!("invalid {}: {error}", result_path.display()))?;
        let pdf = fs::read(&pdf_path)
            .map_err(|error| format!("could not read {}: {error}", pdf_path.display()))?;
        match observe_pdf(&pdf) {
            Ok(_) => ConversionAttempt::Converted(Box::new(result), pdf),
            Err(error) => ConversionAttempt::Failed(format!("invalid Office PDF: {error}")),
        }
    };
    let _ = worker_root;
    Ok((pending, attempt))
}

fn converted_record(
    root: &Path,
    assignment: &CampaignAssignment,
    environment_id: &str,
    attempts: u32,
    elapsed_ms: u64,
    result: ProbeResult,
    pdf: &[u8],
) -> Result<CampaignConversionRecord, String> {
    let source_key = assignment.source_key();
    if result.schema_version != 1
        || result.file != source_key
        || result.application != office_application(assignment.family)
        || result.options != assignment.office
        || result.source_sha256 != assignment.source_sha256
        || result.output != "case-000.pdf"
        || result.output_bytes != pdf.len() as u64
        || result.output_sha256 != sha256_bytes(pdf)
        || result.elapsed_ms > elapsed_ms
    {
        return Err(format!("Office probe result drift for {source_key}"));
    }
    let observed_pdf = observe_pdf(pdf)?;
    let output = format!("{}.pdf", assignment.file);
    let destination = root
        .join("corpus_pdf_conv")
        .join(&assignment.corpus)
        .join(&output);
    promote_bytes(&destination, pdf)?;
    Ok(CampaignConversionRecord {
        schema_version: CONVERSION_SCHEMA_VERSION,
        campaign_id: assignment.campaign_id.clone(),
        assignment_algorithm: assignment.assignment_algorithm.clone(),
        file: assignment.file.clone(),
        family: assignment.family,
        source_extension: assignment.source_extension.clone(),
        source_bytes: assignment.source_bytes,
        source_sha256: assignment.source_sha256.clone(),
        configuration_id: assignment.configuration_id.clone(),
        requested: assignment.requested.clone(),
        effective: assignment.effective.clone(),
        adjustments: assignment.adjustments.clone(),
        office_options: assignment.office.clone(),
        status: "converted".to_string(),
        reference_engine: "Microsoft Office".to_string(),
        environment_id: environment_id.to_string(),
        application: result.application,
        application_version: result.application_version,
        application_build: result.application_build,
        output,
        output_bytes: pdf.len() as u64,
        output_sha256: sha256_bytes(pdf),
        observed_pdf: Some(observed_pdf),
        converted_at_utc: jiff::Timestamp::now().to_string(),
        elapsed_ms,
        attempts,
        annotations: Vec::new(),
        failure_class: String::new(),
        error_code: String::new(),
        error: String::new(),
    })
}

fn failed_record(
    assignment: &CampaignAssignment,
    environment_id: &str,
    status: &str,
    failure_class: &str,
    attempts: u32,
    elapsed_ms: u64,
    error: String,
) -> CampaignConversionRecord {
    CampaignConversionRecord {
        schema_version: CONVERSION_SCHEMA_VERSION,
        campaign_id: assignment.campaign_id.clone(),
        assignment_algorithm: assignment.assignment_algorithm.clone(),
        file: assignment.file.clone(),
        family: assignment.family,
        source_extension: assignment.source_extension.clone(),
        source_bytes: assignment.source_bytes,
        source_sha256: assignment.source_sha256.clone(),
        configuration_id: assignment.configuration_id.clone(),
        requested: assignment.requested.clone(),
        effective: assignment.effective.clone(),
        adjustments: assignment.adjustments.clone(),
        office_options: assignment.office.clone(),
        status: status.to_string(),
        reference_engine: "Microsoft Office".to_string(),
        environment_id: environment_id.to_string(),
        application: office_application(assignment.family).to_string(),
        application_version: String::new(),
        application_build: String::new(),
        output: String::new(),
        output_bytes: 0,
        output_sha256: String::new(),
        observed_pdf: None,
        converted_at_utc: jiff::Timestamp::now().to_string(),
        elapsed_ms,
        attempts,
        annotations: Vec::new(),
        failure_class: failure_class.to_string(),
        error_code: String::new(),
        error: truncate_message(error),
    }
}

fn observe_pdf(pdf: &[u8]) -> Result<ObservedPdfFacts, String> {
    let header = pdf
        .strip_prefix(b"%PDF-")
        .and_then(|rest| rest.split(|byte| matches!(byte, b'\r' | b'\n')).next())
        .and_then(|version| std::str::from_utf8(version).ok())
        .filter(|version| !version.is_empty())
        .ok_or_else(|| "Office output does not have a valid PDF header".to_string())?;
    let document = lopdf::Document::load_mem(pdf)
        .map_err(|error| format!("Office output is not a readable PDF: {error}"))?;
    let page_count = u32::try_from(document.get_pages().len())
        .map_err(|_| "Office PDF has too many pages".to_string())?;
    if page_count == 0 {
        return Err("Office exported a zero-page PDF".to_string());
    }
    let mut searchable = pdf.to_vec();
    for object in document.objects.values() {
        if let lopdf::Object::Stream(stream) = object
            && let Ok(content) = stream.decompressed_content()
        {
            searchable.extend_from_slice(&content);
        }
    }
    let text = String::from_utf8_lossy(&searchable);
    let tagged_pdf = text.contains("/Marked true") || text.contains("/Marked true/");
    let has_outlines = document
        .catalog()
        .is_ok_and(|catalog| catalog.get(b"Outlines").is_ok());
    let pdf_a_part = xmp_value(&text, "pdfaid:part").and_then(|value| value.parse().ok());
    let pdf_a_conformance = xmp_value(&text, "pdfaid:conformance");
    Ok(ObservedPdfFacts {
        header_version: header.to_string(),
        page_count,
        tagged_pdf,
        has_outlines,
        pdf_a_part,
        pdf_a_conformance,
    })
}

fn xmp_value(text: &str, name: &str) -> Option<String> {
    let start_tag = format!("<{name}>");
    let end_tag = format!("</{name}>");
    if let Some(rest) = text.split_once(&start_tag).map(|(_, rest)| rest)
        && let Some((value, _)) = rest.split_once(&end_tag)
    {
        return Some(value.trim().to_string());
    }
    let attribute = format!("{name}=\"");
    text.split_once(&attribute)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value.trim().to_string())
}

fn summarize_conversions(
    root: &Path,
    plan_path: &Path,
    pilot_only: bool,
    skipped: usize,
    attempts: usize,
    environment_id: String,
) -> Result<CampaignConversionSummary, String> {
    let assignments = read_plan(plan_path)?;
    let selected = assignments
        .iter()
        .filter(|assignment| !pilot_only || assignment.pilot_300)
        .collect::<Vec<_>>();
    let records = read_conversion_records(root)?;
    let mut converted = 0;
    let mut failed = 0;
    let mut timed_out = 0;
    let mut family_statuses = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for assignment in &selected {
        let key = (assignment.corpus.clone(), assignment.file.clone());
        let record = records
            .get(&key)
            .ok_or_else(|| format!("missing conversion record for {}", assignment.source_key()))?;
        validate_conversion_record(assignment, record)?;
        if record.environment_id != environment_id {
            return Err(format!("environment drift for {}", assignment.source_key()));
        }
        match record.status.as_str() {
            "converted" => converted += 1,
            "failed" => failed += 1,
            "timeout" => timed_out += 1,
            _ => unreachable!(),
        }
        *family_statuses
            .entry(assignment.family.as_str().to_string())
            .or_default()
            .entry(record.status.clone())
            .or_default() += 1;
    }
    Ok(CampaignConversionSummary {
        selected: selected.len(),
        converted,
        failed,
        timed_out,
        skipped,
        attempts,
        family_statuses,
        environment_id,
    })
}

fn write_conversion_manifests(
    root: &Path,
    records: &BTreeMap<(String, String), CampaignConversionRecord>,
) -> Result<(), String> {
    let mut corpora = BTreeMap::<String, Vec<&CampaignConversionRecord>>::new();
    for ((corpus, _), record) in records {
        corpora.entry(corpus.clone()).or_default().push(record);
    }
    for (corpus, records) in corpora {
        let path = root
            .join("corpus_pdf_conv")
            .join(corpus)
            .join("manifest.jsonl");
        write_json_lines(&path, &records)?;
    }
    Ok(())
}

fn promote_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("output has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("output")
    ));
    fs::write(&temporary, bytes)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "could not promote {} to {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}

fn windows_path(path: &Path) -> Result<String, String> {
    let output = Command::new("wslpath")
        .arg("-w")
        .arg(path)
        .output()
        .map_err(|error| format!("could not run wslpath for {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "wslpath failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|path| path.trim().to_string())
        .map_err(|error| format!("wslpath returned non-UTF-8 output: {error}"))
}

fn stop_office_process(pwsh: &str, process_id_path: &Path) {
    let Ok(process_id) = fs::read_to_string(process_id_path) else {
        return;
    };
    let Ok(process_id) = process_id.trim().parse::<u32>() else {
        return;
    };
    let command = format!("Stop-Process -Id {process_id} -Force -ErrorAction SilentlyContinue");
    let _ = Command::new(pwsh)
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .status();
}

fn office_application(family: OfficeFamily) -> &'static str {
    match family {
        OfficeFamily::Word => "Word",
        OfficeFamily::Excel => "Excel",
        OfficeFamily::PowerPoint => "PowerPoint",
    }
}

fn command_output_message(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let message = format!("stdout={} stderr={}", stdout.trim(), stderr.trim());
    truncate_message(message)
}

fn truncate_message(mut message: String) -> String {
    const MAX_CHARS: usize = 4_000;
    if message.chars().count() > MAX_CHARS {
        message = message.chars().take(MAX_CHARS).collect();
        message.push('…');
    }
    message
}

fn one_line(message: &str) -> String {
    message.replace(['\r', '\n'], " ")
}

pub fn read_conversion_records(
    root: &Path,
) -> Result<BTreeMap<(String, String), CampaignConversionRecord>, String> {
    let conversion_root = root.join("corpus_pdf_conv");
    let mut records = BTreeMap::new();
    if !conversion_root.is_dir() {
        return Err(format!(
            "configured conversion root does not exist: {}",
            conversion_root.display()
        ));
    }
    for entry in fs::read_dir(&conversion_root)
        .map_err(|error| format!("could not scan {}: {error}", conversion_root.display()))?
    {
        let entry = entry.map_err(|error| {
            format!(
                "could not read entry below {}: {error}",
                conversion_root.display()
            )
        })?;
        let manifest_path = entry.path().join("manifest.jsonl");
        if !manifest_path.is_file() {
            continue;
        }
        let corpus = entry
            .file_name()
            .to_str()
            .ok_or_else(|| format!("non-UTF-8 corpus output path {}", entry.path().display()))?
            .to_string();
        let file = File::open(&manifest_path)
            .map_err(|error| format!("could not open {}: {error}", manifest_path.display()))?;
        for (line_index, line) in BufReader::new(file).lines().enumerate() {
            let line = line
                .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
            if line.trim().is_empty() {
                continue;
            }
            let record: CampaignConversionRecord =
                serde_json::from_str(&line).map_err(|error| {
                    format!(
                        "invalid conversion JSON at {}:{}: {error}",
                        manifest_path.display(),
                        line_index + 1
                    )
                })?;
            let key = (corpus.clone(), record.file.clone());
            if records.insert(key.clone(), record).is_some() {
                return Err(format!(
                    "duplicate conversion record for {}/{}",
                    key.0, key.1
                ));
            }
        }
    }
    Ok(records)
}

fn validate_conversion_record(
    assignment: &CampaignAssignment,
    record: &CampaignConversionRecord,
) -> Result<(), String> {
    if record.schema_version != CONVERSION_SCHEMA_VERSION
        || record.campaign_id != assignment.campaign_id
        || record.assignment_algorithm != assignment.assignment_algorithm
        || record.file != assignment.file
        || record.family != assignment.family
        || record.source_extension != assignment.source_extension
        || record.source_bytes != assignment.source_bytes
        || record.source_sha256 != assignment.source_sha256
        || record.configuration_id != assignment.configuration_id
        || record.requested != assignment.requested
        || record.effective != assignment.effective
        || record.adjustments != assignment.adjustments
        || record.office_options != assignment.office
    {
        return Err(format!(
            "conversion record drift for {}",
            assignment.source_key()
        ));
    }
    if record.reference_engine != "Microsoft Office" || record.environment_id.is_empty() {
        return Err(format!(
            "conversion provenance is incomplete for {}",
            assignment.source_key()
        ));
    }
    if record.status == "converted" {
        if record.output != format!("{}.pdf", assignment.file)
            || record.output_bytes == 0
            || record.output_sha256.len() != 64
            || record.observed_pdf.is_none()
            || record.application_version.is_empty()
        {
            return Err(format!(
                "converted record is incomplete for {}",
                assignment.source_key()
            ));
        }
    } else if !matches!(record.status.as_str(), "failed" | "timeout") {
        return Err(format!(
            "unknown conversion status {:?} for {}",
            record.status,
            assignment.source_key()
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuditTask {
    assignment: CampaignAssignment,
    conversion: CampaignConversionRecord,
}

#[derive(Debug, Deserialize, Serialize)]
struct AuditWorkerRequest {
    configuration_id: String,
    task_path: PathBuf,
    result_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct AuditWorkerResponse {
    configuration_id: String,
    error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CampaignAuditRecord {
    pub corpus: String,
    pub file: String,
    pub family: OfficeFamily,
    pub configuration_id: String,
    pub reference_status: String,
    pub verdict: String,
    pub layer: Option<String>,
    pub diagnostic_kind: Option<String>,
    pub page_index: Option<usize>,
    pub line_index: Option<usize>,
    pub elapsed_ms: u128,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CampaignAuditSummary {
    pub assigned: usize,
    pub reference_converted: usize,
    pub reference_failed: usize,
    pub reference_timed_out: usize,
    pub audited: usize,
    pub passed: usize,
    pub failed: usize,
    pub infrastructure_errors: usize,
    pub verdicts: BTreeMap<String, usize>,
    pub layers: BTreeMap<String, usize>,
    pub report_path: String,
}

pub fn audit_campaign_pilot(
    executable: &Path,
    root: &Path,
    plan_path: &Path,
    jobs: usize,
    timeout: Duration,
) -> Result<CampaignAuditSummary, String> {
    audit_campaign(executable, root, plan_path, true, jobs, timeout)
}

pub fn audit_campaign(
    executable: &Path,
    root: &Path,
    plan_path: &Path,
    pilot_only: bool,
    jobs: usize,
    timeout: Duration,
) -> Result<CampaignAuditSummary, String> {
    if jobs == 0 {
        return Err("audit jobs must be positive".to_string());
    }
    let assignments = read_plan(plan_path)?;
    validate_assignments(root, &assignments)?;
    let selected = assignments
        .into_iter()
        .filter(|assignment| !pilot_only || assignment.pilot_300)
        .collect::<Vec<_>>();
    let expected = if pilot_only {
        PILOT_ASSIGNMENT_COUNT
    } else {
        EXPECTED_ASSIGNMENT_COUNT
    };
    if selected.len() != expected {
        return Err(format!(
            "audit selection contains {} assignments, expected {expected}",
            selected.len()
        ));
    }
    let conversions = read_conversion_records(root)?;
    let environment_id = load_environment_id(root)?;
    let work_root = root.join("target/office-pdf-campaign");
    let selection_name = if pilot_only { "bootstrap" } else { "full" };
    let task_root = work_root.join(format!("{selection_name}-tasks"));
    let result_root = work_root.join(format!("{selection_name}-results"));
    fs::create_dir_all(&task_root)
        .map_err(|error| format!("could not create {}: {error}", task_root.display()))?;
    fs::create_dir_all(&result_root)
        .map_err(|error| format!("could not create {}: {error}", result_root.display()))?;

    let mut records = Vec::with_capacity(expected);
    let mut tasks = Vec::new();
    for assignment in selected {
        let key = (assignment.corpus.clone(), assignment.file.clone());
        let conversion = conversions.get(&key).ok_or_else(|| {
            format!(
                "audit assignment has no terminal conversion record: {}",
                assignment.source_key()
            )
        })?;
        validate_conversion_record(&assignment, conversion)?;
        if conversion.environment_id != environment_id {
            return Err(format!(
                "environment drift for {}: manifest={}, campaign={environment_id}",
                assignment.source_key(),
                conversion.environment_id
            ));
        }
        if conversion.status == "converted" {
            let golden_path = root
                .join("corpus_pdf_conv")
                .join(&assignment.corpus)
                .join(&conversion.output);
            let golden = fs::read(&golden_path).map_err(|error| {
                format!(
                    "could not read configured golden {}: {error}",
                    golden_path.display()
                )
            })?;
            if golden.len() as u64 != conversion.output_bytes
                || sha256_bytes(&golden) != conversion.output_sha256
            {
                return Err(format!(
                    "configured golden identity drift for {}",
                    assignment.source_key()
                ));
            }
            tasks.push(AuditTask {
                assignment,
                conversion: conversion.clone(),
            });
        } else {
            records.push(CampaignAuditRecord {
                corpus: assignment.corpus,
                file: assignment.file,
                family: assignment.family,
                configuration_id: assignment.configuration_id,
                reference_status: conversion.status.clone(),
                verdict: "REFERENCE_FAIL".to_string(),
                layer: None,
                diagnostic_kind: None,
                page_index: None,
                line_index: None,
                elapsed_ms: u128::from(conversion.elapsed_ms),
                message: format!("{}: {}", conversion.failure_class, conversion.error),
            });
        }
    }

    let report_path = work_root.join(format!("{selection_name}-audit.jsonl"));
    let serial_task_count = prioritize_audit_tasks(&mut tasks, &report_path, timeout);
    let mut pending = tasks
        .into_iter()
        .enumerate()
        .map(|(index, task)| {
            prepare_audit_task(&task_root, &result_root, task, index < serial_task_count)
        })
        .collect::<Result<VecDeque<_>, _>>()?;
    let desired_worker_count = jobs.min(pending.len());
    let initial_worker_count = if serial_task_count == 0 {
        desired_worker_count
    } else {
        1
    };
    let mut serial_tasks_remaining = serial_task_count;
    let mut workers = Vec::with_capacity(desired_worker_count);
    for _ in 0..initial_worker_count {
        match spawn_audit_worker(executable) {
            Ok(worker) => workers.push(worker),
            Err(error) => {
                shutdown_audit_workers(&mut workers, true);
                return Err(error);
            }
        }
    }
    for worker in &mut workers {
        if let Some(task) = pending.pop_front() {
            assign_audit_task(worker, task)?;
        }
    }

    loop {
        if pending.is_empty() && workers.iter().all(|worker| worker.active.is_none()) {
            break;
        }
        let mut index = 0;
        while index < workers.len() {
            match poll_audit_worker(&workers[index], timeout) {
                AuditWorkerEvent::Pending => {}
                AuditWorkerEvent::Completed(response) => {
                    let active = workers[index]
                        .active
                        .take()
                        .expect("completed audit worker must have an active task");
                    let expected_id = &active.task.assignment.configuration_id;
                    let (error, restart) = match response {
                        Ok(response) if response.configuration_id == *expected_id => {
                            (response.error, false)
                        }
                        Ok(response) => (
                            Some(format!(
                                "audit worker response identity mismatch: expected={expected_id}, actual={}",
                                response.configuration_id
                            )),
                            true,
                        ),
                        Err(error) => (Some(error), true),
                    };
                    let constrains_parallelism = active.constrains_parallelism;
                    records.push(finish_audit_worker(active, error)?);
                    if constrains_parallelism {
                        serial_tasks_remaining = serial_tasks_remaining.saturating_sub(1);
                    }
                    if restart {
                        stop_audit_worker(&mut workers[index], true);
                        workers[index] = spawn_audit_worker(executable)?;
                    }
                }
                AuditWorkerEvent::TimedOut => {
                    let active = workers[index]
                        .active
                        .take()
                        .expect("timed-out audit worker must have an active task");
                    let constrains_parallelism = active.constrains_parallelism;
                    records.push(audit_timeout_record(&active, timeout));
                    if constrains_parallelism {
                        serial_tasks_remaining = serial_tasks_remaining.saturating_sub(1);
                    }
                    stop_audit_worker(&mut workers[index], true);
                    workers[index] = spawn_audit_worker(executable)?;
                }
                AuditWorkerEvent::Disconnected => {
                    let active = workers[index]
                        .active
                        .take()
                        .expect("disconnected audit worker must have an active task");
                    let constrains_parallelism = active.constrains_parallelism;
                    records.push(finish_audit_worker(
                        active,
                        Some("audit worker exited before returning a response".to_string()),
                    )?);
                    if constrains_parallelism {
                        serial_tasks_remaining = serial_tasks_remaining.saturating_sub(1);
                    }
                    stop_audit_worker(&mut workers[index], true);
                    workers[index] = spawn_audit_worker(executable)?;
                }
            }
            index += 1;
        }
        if serial_tasks_remaining == 0 {
            while workers.len() < desired_worker_count {
                workers.push(spawn_audit_worker(executable)?);
            }
        }
        for worker in &mut workers {
            if worker.active.is_none()
                && let Some(task) = pending.pop_front()
            {
                assign_audit_task(worker, task)?;
            }
        }
        thread::sleep(AUDIT_WORKER_POLL_INTERVAL);
    }
    shutdown_audit_workers(&mut workers, false);

    records.sort_by(|left, right| {
        (&left.family, &left.corpus, &left.file).cmp(&(&right.family, &right.corpus, &right.file))
    });
    if records.len() != expected {
        return Err(format!(
            "audit accounting drift: got {} records, expected {expected}",
            records.len()
        ));
    }
    write_json_lines(&report_path, &records)?;
    let mut verdicts = BTreeMap::new();
    let mut layers = BTreeMap::new();
    for record in &records {
        *verdicts.entry(record.verdict.clone()).or_insert(0) += 1;
        if let Some(layer) = &record.layer {
            *layers.entry(layer.clone()).or_insert(0) += 1;
        }
    }
    let infrastructure_errors = records
        .iter()
        .filter(|record| record.verdict.starts_with("INFRA_"))
        .count();
    let summary = CampaignAuditSummary {
        assigned: records.len(),
        reference_converted: records
            .iter()
            .filter(|record| record.reference_status == "converted")
            .count(),
        reference_failed: records
            .iter()
            .filter(|record| record.reference_status == "failed")
            .count(),
        reference_timed_out: records
            .iter()
            .filter(|record| record.reference_status == "timeout")
            .count(),
        audited: records
            .iter()
            .filter(|record| matches!(record.verdict.as_str(), "PASS" | "FAIL"))
            .count(),
        passed: records
            .iter()
            .filter(|record| record.verdict == "PASS")
            .count(),
        failed: records
            .iter()
            .filter(|record| record.verdict == "FAIL")
            .count(),
        infrastructure_errors,
        verdicts,
        layers,
        report_path: report_path.display().to_string(),
    };
    let summary_path = work_root.join(format!("{selection_name}-audit-summary.json"));
    write_json(&summary_path, &summary)?;
    if summary.infrastructure_errors != 0 {
        return Err(format!(
            "configured audit had {} infrastructure errors; summary={}",
            summary.infrastructure_errors,
            summary_path.display()
        ));
    }
    Ok(summary)
}

struct PendingAudit {
    task: AuditTask,
    task_path: PathBuf,
    result_path: PathBuf,
    constrains_parallelism: bool,
}

struct RunningAudit {
    started: Instant,
    task: AuditTask,
    result_path: PathBuf,
    constrains_parallelism: bool,
}

struct AuditWorkerProcess {
    child: Option<Child>,
    input: Option<BufWriter<ChildStdin>>,
    responses: mpsc::Receiver<std::result::Result<AuditWorkerResponse, String>>,
    reader: Option<thread::JoinHandle<()>>,
    active: Option<RunningAudit>,
}

enum AuditWorkerEvent {
    Pending,
    Completed(std::result::Result<AuditWorkerResponse, String>),
    TimedOut,
    Disconnected,
}

fn prioritize_audit_tasks(tasks: &mut [AuditTask], report_path: &Path, timeout: Duration) -> usize {
    let Ok(file) = File::open(report_path) else {
        return 0;
    };
    let elapsed_by_id = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<CampaignAuditRecord>(&line).ok())
        .map(|record| (record.configuration_id, record.elapsed_ms))
        .collect::<BTreeMap<_, _>>();
    if elapsed_by_id.is_empty() {
        return 0;
    }
    tasks.sort_by(|left, right| {
        let left_elapsed = elapsed_by_id
            .get(&left.assignment.configuration_id)
            .copied()
            .unwrap_or_default();
        let right_elapsed = elapsed_by_id
            .get(&right.assignment.configuration_id)
            .copied()
            .unwrap_or_default();
        right_elapsed.cmp(&left_elapsed)
    });
    let serial_threshold = timeout.as_millis() / 2;
    tasks
        .iter()
        .take_while(|task| {
            elapsed_by_id
                .get(&task.assignment.configuration_id)
                .is_some_and(|elapsed| *elapsed >= serial_threshold)
        })
        .count()
}

fn prepare_audit_task(
    task_root: &Path,
    result_root: &Path,
    task: AuditTask,
    constrains_parallelism: bool,
) -> Result<PendingAudit, String> {
    let stem = &task.assignment.configuration_id;
    let task_path = task_root.join(format!("{stem}.json"));
    let result_path = result_root.join(format!("{stem}.json"));
    write_json(&task_path, &task)?;
    Ok(PendingAudit {
        task,
        task_path,
        result_path,
        constrains_parallelism,
    })
}

fn spawn_audit_worker(executable: &Path) -> Result<AuditWorkerProcess, String> {
    let mut child = Command::new(executable)
        .arg("audit-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            format!(
                "could not start audit worker {}: {error}",
                executable.display()
            )
        })?;
    let input = child
        .stdin
        .take()
        .ok_or_else(|| "audit worker has no stdin pipe".to_string())?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| "audit worker has no stdout pipe".to_string())?;
    let (sender, responses) = mpsc::channel();
    let reader = thread::spawn(move || {
        for (line_index, line) in BufReader::new(output).lines().enumerate() {
            let response = line
                .map_err(|error| format!("could not read audit worker response: {error}"))
                .and_then(|line| {
                    serde_json::from_str(&line).map_err(|error| {
                        format!(
                            "invalid audit worker response line {}: {error}",
                            line_index + 1
                        )
                    })
                });
            let stop = response.is_err();
            if sender.send(response).is_err() || stop {
                break;
            }
        }
    });
    Ok(AuditWorkerProcess {
        child: Some(child),
        input: Some(BufWriter::new(input)),
        responses,
        reader: Some(reader),
        active: None,
    })
}

fn assign_audit_task(worker: &mut AuditWorkerProcess, pending: PendingAudit) -> Result<(), String> {
    if worker.active.is_some() {
        return Err("cannot assign a second task to a busy audit worker".to_string());
    }
    let request = AuditWorkerRequest {
        configuration_id: pending.task.assignment.configuration_id.clone(),
        task_path: pending.task_path,
        result_path: pending.result_path.clone(),
    };
    let input = worker
        .input
        .as_mut()
        .ok_or_else(|| "audit worker stdin is closed".to_string())?;
    serde_json::to_writer(&mut *input, &request)
        .map_err(|error| format!("could not serialize audit worker request: {error}"))?;
    input
        .write_all(b"\n")
        .map_err(|error| format!("could not delimit audit worker request: {error}"))?;
    input
        .flush()
        .map_err(|error| format!("could not flush audit worker request: {error}"))?;
    worker.active = Some(RunningAudit {
        started: Instant::now(),
        task: pending.task,
        result_path: pending.result_path,
        constrains_parallelism: pending.constrains_parallelism,
    });
    Ok(())
}

fn poll_audit_worker(worker: &AuditWorkerProcess, timeout: Duration) -> AuditWorkerEvent {
    let Some(active) = &worker.active else {
        return AuditWorkerEvent::Pending;
    };
    match worker.responses.try_recv() {
        Ok(response) => AuditWorkerEvent::Completed(response),
        Err(mpsc::TryRecvError::Disconnected) => AuditWorkerEvent::Disconnected,
        Err(mpsc::TryRecvError::Empty) if active.started.elapsed() >= timeout => {
            AuditWorkerEvent::TimedOut
        }
        Err(mpsc::TryRecvError::Empty) => AuditWorkerEvent::Pending,
    }
}

fn stop_audit_worker(worker: &mut AuditWorkerProcess, force: bool) {
    worker.input.take();
    if let Some(mut child) = worker.child.take() {
        if force {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
    if let Some(reader) = worker.reader.take() {
        let _ = reader.join();
    }
}

fn shutdown_audit_workers(workers: &mut [AuditWorkerProcess], force: bool) {
    for worker in workers {
        stop_audit_worker(worker, force);
    }
}

fn finish_audit_worker(
    worker: RunningAudit,
    worker_error: Option<String>,
) -> Result<CampaignAuditRecord, String> {
    let result = if let Some(error) = worker_error {
        Err(error)
    } else {
        fs::read(&worker.result_path)
            .map_err(|error| format!("could not read {}: {error}", worker.result_path.display()))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|error| format!("invalid {}: {error}", worker.result_path.display()))
            })
    };
    result.or_else(|error| {
        Ok(CampaignAuditRecord {
            corpus: worker.task.assignment.corpus,
            file: worker.task.assignment.file,
            family: worker.task.assignment.family,
            configuration_id: worker.task.assignment.configuration_id,
            reference_status: worker.task.conversion.status,
            verdict: "INFRA_WORKER_ERROR".to_string(),
            layer: None,
            diagnostic_kind: None,
            page_index: None,
            line_index: None,
            elapsed_ms: worker.started.elapsed().as_millis(),
            message: error,
        })
    })
}

fn audit_timeout_record(worker: &RunningAudit, timeout: Duration) -> CampaignAuditRecord {
    CampaignAuditRecord {
        corpus: worker.task.assignment.corpus.clone(),
        file: worker.task.assignment.file.clone(),
        family: worker.task.assignment.family,
        configuration_id: worker.task.assignment.configuration_id.clone(),
        reference_status: worker.task.conversion.status.clone(),
        verdict: "INFRA_TIMEOUT".to_string(),
        layer: None,
        diagnostic_kind: None,
        page_index: None,
        line_index: None,
        elapsed_ms: worker.started.elapsed().as_millis(),
        message: format!("candidate audit exceeded {} seconds", timeout.as_secs()),
    }
}

pub fn audit_worker_loop() -> Result<(), String> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    for (line_index, line) in stdin.lock().lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "could not read audit worker request line {}: {error}",
                line_index + 1
            )
        })?;
        let request: AuditWorkerRequest = serde_json::from_str(&line).map_err(|error| {
            format!(
                "invalid audit worker request line {}: {error}",
                line_index + 1
            )
        })?;
        let response = AuditWorkerResponse {
            configuration_id: request.configuration_id,
            error: audit_one_with_artifacts(&request.task_path, &request.result_path, false).err(),
        };
        serde_json::to_writer(&mut output, &response)
            .map_err(|error| format!("could not serialize audit worker response: {error}"))?;
        output
            .write_all(b"\n")
            .map_err(|error| format!("could not delimit audit worker response: {error}"))?;
        output
            .flush()
            .map_err(|error| format!("could not flush audit worker response: {error}"))?;
    }
    Ok(())
}

pub fn audit_one(task_path: &Path, result_path: &Path) -> Result<(), String> {
    audit_one_with_artifacts(task_path, result_path, false)
}

pub fn prepare_audit_one(
    root: &Path,
    plan_path: &Path,
    configuration_id: &str,
    task_path: &Path,
) -> Result<(), String> {
    let assignments = read_plan(plan_path)?;
    validate_assignments(root, &assignments)?;
    let assignment = assignments
        .into_iter()
        .find(|assignment| assignment.configuration_id == configuration_id)
        .ok_or_else(|| {
            format!(
                "configured audit plan has no assignment with configuration ID {configuration_id}"
            )
        })?;
    let conversions = read_conversion_records(root)?;
    let key = (assignment.corpus.clone(), assignment.file.clone());
    let conversion = conversions.get(&key).cloned().ok_or_else(|| {
        format!(
            "configured audit assignment has no terminal conversion record: {}",
            assignment.source_key()
        )
    })?;
    validate_conversion_record(&assignment, &conversion)?;
    if conversion.environment_id != load_environment_id(root)? {
        return Err(format!("environment drift for {}", assignment.source_key()));
    }
    if conversion.status != "converted" {
        return Err(format!(
            "configured audit assignment has no Office golden: {} ({})",
            assignment.source_key(),
            conversion.status
        ));
    }
    let golden_path = root
        .join("corpus_pdf_conv")
        .join(&assignment.corpus)
        .join(&conversion.output);
    let golden = fs::read(&golden_path).map_err(|error| {
        format!(
            "could not read configured golden {}: {error}",
            golden_path.display()
        )
    })?;
    if golden.len() as u64 != conversion.output_bytes
        || sha256_bytes(&golden) != conversion.output_sha256
    {
        return Err(format!(
            "configured golden identity drift for {}",
            assignment.source_key()
        ));
    }
    write_json(
        task_path,
        &AuditTask {
            assignment,
            conversion,
        },
    )
}

pub fn audit_one_with_artifacts(
    task_path: &Path,
    result_path: &Path,
    write_artifacts: bool,
) -> Result<(), String> {
    let task_bytes = fs::read(task_path)
        .map_err(|error| format!("could not read audit task {}: {error}", task_path.display()))?;
    let task: AuditTask = serde_json::from_slice(&task_bytes)
        .map_err(|error| format!("invalid audit task {}: {error}", task_path.display()))?;
    let started = Instant::now();
    let assignment = &task.assignment;
    let conversion = &task.conversion;
    validate_conversion_record(assignment, conversion)?;
    if conversion.status != "converted" {
        return Err(format!(
            "audit task does not reference a converted record: {}",
            assignment.source_key()
        ));
    }
    let case = OfficeGoldenCase {
        id: &assignment.configuration_id,
        corpus: &assignment.corpus,
        source: &assignment.file,
        source_sha256: &assignment.source_sha256,
        golden_sha256: &conversion.output_sha256,
        environment_id: &conversion.environment_id,
        ui_language: &assignment.effective.ui_language,
        format_locale: &assignment.effective.format_locale,
    };
    let candidate_options = assignment.requested_pdf_options()?;
    let comparison = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compare_office_golden_detailed_with_prevalidated_options(
            case,
            candidate_options,
            VisualTolerance::OFFICE_FIXED_OUTPUT,
            write_artifacts,
            write_artifacts,
        )
    }));
    let result = match comparison {
        Ok(Ok(_)) => CampaignAuditRecord {
            corpus: assignment.corpus.clone(),
            file: assignment.file.clone(),
            family: assignment.family,
            configuration_id: assignment.configuration_id.clone(),
            reference_status: conversion.status.clone(),
            verdict: "PASS".to_string(),
            layer: None,
            diagnostic_kind: None,
            page_index: None,
            line_index: None,
            elapsed_ms: started.elapsed().as_millis(),
            message: String::new(),
        },
        Ok(Err(error)) => audit_failure_record(assignment, conversion, error, started.elapsed()),
        Err(payload) => CampaignAuditRecord {
            corpus: assignment.corpus.clone(),
            file: assignment.file.clone(),
            family: assignment.family,
            configuration_id: assignment.configuration_id.clone(),
            reference_status: conversion.status.clone(),
            verdict: "INFRA_PANIC".to_string(),
            layer: None,
            diagnostic_kind: None,
            page_index: None,
            line_index: None,
            elapsed_ms: started.elapsed().as_millis(),
            message: panic_message(payload),
        },
    };
    write_json(result_path, &result)
}

pub fn render_one_with_task(
    task_path: &Path,
    input_path: &Path,
    output_path: &Path,
) -> Result<(), String> {
    let task_bytes = fs::read(task_path)
        .map_err(|error| format!("could not read audit task {}: {error}", task_path.display()))?;
    let task: AuditTask = serde_json::from_slice(&task_bytes).map_err(|error| {
        format!(
            "could not parse audit task {}: {error}",
            task_path.display()
        )
    })?;
    validate_conversion_record(&task.assignment, &task.conversion)?;
    if task.conversion.status != "converted" {
        return Err(format!(
            "render task does not reference a converted record: {}",
            task.assignment.source_key()
        ));
    }
    let extension = input_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("minimum input has no extension: {}", input_path.display()))?;
    let family = OfficeFamily::from_extension(&extension)
        .ok_or_else(|| format!("unsupported minimum input extension: {extension}"))?;
    if family != task.assignment.family {
        return Err(format!(
            "minimum input family {family:?} differs from golden family {:?}",
            task.assignment.family
        ));
    }
    let options = task.assignment.requested_pdf_options()?;
    let pdf =
        crate::render::render_fixture_pdf_with_options(input_path, options).map_err(|error| {
            format!(
                "could not render minimum input {}: {error}",
                input_path.display()
            )
        })?;
    let mut output = File::options()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|error| format!("could not create {}: {error}", output_path.display()))?;
    output
        .write_all(&pdf)
        .map_err(|error| format!("could not write {}: {error}", output_path.display()))?;
    Ok(())
}

fn audit_failure_record(
    assignment: &CampaignAssignment,
    conversion: &CampaignConversionRecord,
    error: crate::OfficeGoldenFailure,
    elapsed: Duration,
) -> CampaignAuditRecord {
    CampaignAuditRecord {
        corpus: assignment.corpus.clone(),
        file: assignment.file.clone(),
        family: assignment.family,
        configuration_id: assignment.configuration_id.clone(),
        reference_status: conversion.status.clone(),
        verdict: "FAIL".to_string(),
        layer: Some(error.layer.as_str().to_string()),
        diagnostic_kind: Some(error.diagnostic_kind.as_str().to_string()),
        page_index: error.page_index,
        line_index: error.line_index,
        elapsed_ms: elapsed.as_millis(),
        message: error.message,
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|value| (*value).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "audit worker panicked with a non-string payload".to_string())
}

fn load_environment_id(root: &Path) -> Result<String, String> {
    let path = root.join("corpus_pdf_conv/environment.json");
    let bytes =
        fs::read(&path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    value
        .get("environment_id")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("missing environment_id in {}", path.display()))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension("json.tmp");
    let file = File::create(&temporary)
        .map_err(|error| format!("could not create {}: {error}", temporary.display()))?;
    serde_json::to_writer_pretty(BufWriter::new(file), value)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "could not promote {} to {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}

fn write_json_lines(path: &Path, values: &[impl Serialize]) -> Result<(), String> {
    let temporary = path.with_extension("jsonl.tmp");
    let file = File::create(&temporary)
        .map_err(|error| format!("could not create {}: {error}", temporary.display()))?;
    let mut writer = BufWriter::new(file);
    for value in values {
        serde_json::to_writer(&mut writer, value)
            .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    }
    writer
        .flush()
        .map_err(|error| format!("could not flush {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "could not promote {} to {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("could not open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_campaign_has_exact_round_trip_and_pilot_cardinality() {
        let root = crate::workspace_root();
        let assignments = generate_campaign_assignments(&root).unwrap();
        let summary = validate_assignments(&root, &assignments).unwrap();
        assert_eq!(summary.assignments, EXPECTED_ASSIGNMENT_COUNT);
        assert_eq!(summary.pilot_assignments, PILOT_ASSIGNMENT_COUNT);
        assert_eq!(summary.family_assignments["word"], 3_045);
        assert_eq!(summary.family_assignments["excel"], 1_345);
        assert_eq!(summary.family_assignments["powerpoint"], 980);
        assert_eq!(summary.pilot_family_assignments["word"], 100);
        assert_eq!(summary.pilot_family_assignments["excel"], 100);
        assert_eq!(summary.pilot_family_assignments["powerpoint"], 100);
    }

    #[test]
    fn generated_campaign_is_byte_deterministic() {
        let root = crate::workspace_root();
        let first = generate_campaign_assignments(&root).unwrap();
        let second = generate_campaign_assignments(&root).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn office_option_adapter_rejects_impossible_family_fields() {
        let root = crate::workspace_root();
        let assignments = generate_campaign_assignments(&root).unwrap();
        for assignment in assignments {
            validate_office_options(&assignment).unwrap();
        }
    }

    #[test]
    fn recorded_office_quality_owns_the_candidate_fixed_output_profile() {
        let root = crate::workspace_root();
        let assignments = generate_campaign_assignments(&root).unwrap();
        for assignment in assignments {
            let expected = assignment.office.candidate_optimize_for().unwrap();
            let candidate = assignment.requested_pdf_options().unwrap();
            assert_eq!(
                candidate.optimize_for,
                expected,
                "{}",
                assignment.source_key()
            );
            let expected_kind = match assignment.family {
                OfficeFamily::Word => PdfDocumentKind::Docx,
                OfficeFamily::Excel => PdfDocumentKind::Xlsx,
                OfficeFamily::PowerPoint => PdfDocumentKind::Pptx,
            };
            assert_eq!(
                candidate.images.optimization_policy,
                PdfImageOptimizationPolicy::MicrosoftOfficeFixedOutput(expected_kind),
                "{}",
                assignment.source_key(),
            );
        }
    }
}

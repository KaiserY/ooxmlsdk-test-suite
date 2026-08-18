use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufWriter, Read as _, Write as _};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::office_golden::compare_office_golden_detailed_with_artifacts;
use crate::{
    OfficeGoldenCase, OfficeGoldenComparisonLayer, OfficeGoldenDiagnosticKind, OfficeGoldenFailure,
    VisualTolerance, workspace_root,
};

const ERROR_MANIFEST_SCHEMA_VERSION: u32 = 1;
const FAILURE_SAMPLE_LIMIT: usize = 3;
const DEFAULT_AUDIT_PAGE_SIZE: usize = 32;
// Keep late-corpus pages bounded: large Office packages can take minutes per
// comparison, so a 32-record page persists useful classification progress
// without forcing a long tail to restart from the previous checkpoint.
const UNEXPECTED_FAILURE_LIMIT: usize = 32;
const SLOW_CASE_THRESHOLD: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuditLimit {
    Page(usize),
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AuditWindow {
    offset: usize,
    selected: usize,
    total: usize,
    next_offset: Option<usize>,
    exhaustive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OfficeGoldenFormat {
    Docx,
    Pptx,
    Xlsx,
}

impl OfficeGoldenFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Pptx => "pptx",
            Self::Xlsx => "xlsx",
        }
    }

    const fn ratchet_test_name(self) -> &'static str {
        match self {
            Self::Docx => "office_golden_docx_corpus_ratchet",
            Self::Pptx => "office_golden_pptx_corpus_ratchet",
            Self::Xlsx => "office_golden_xlsx_corpus_ratchet",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfficeGoldenCorpusReport {
    pub format: OfficeGoldenFormat,
    pub attempted: usize,
    pub passed: usize,
    pub expected_errors: usize,
}

#[derive(Debug, Deserialize)]
struct ConversionRecord {
    file: String,
    source_extension: String,
    source_bytes: u64,
    source_sha256: String,
    status: String,
    reference_engine: String,
    environment_id: String,
    output_bytes: u64,
    output_sha256: String,
}

#[derive(Debug)]
struct CorpusCandidate {
    corpus: String,
    record: ConversionRecord,
}

#[derive(Debug, Deserialize)]
struct EnvironmentRecord {
    environment_id: String,
    environment: ReferenceEnvironment,
}

#[derive(Debug, Deserialize)]
struct ReferenceEnvironment {
    locale: ReferenceLocale,
}

#[derive(Debug, Deserialize)]
struct ReferenceLocale {
    culture: String,
    ui_culture: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DiagnosticIndexRecord {
    corpus: String,
    source: String,
    diagnostic_kind: String,
    layer: String,
    #[serde(default)]
    page_index: Option<usize>,
    #[serde(default)]
    line_index: Option<usize>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ErrorManifest {
    schema_version: u32,
    #[serde(default)]
    class: Vec<ErrorClass>,
    #[serde(default)]
    error: Vec<KnownError>,
}

#[derive(Clone, Debug, Deserialize)]
struct ErrorClass {
    id: String,
    layer: String,
    reason: String,
    evidence: Vec<String>,
    #[serde(default)]
    skip_batch_audit: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct KnownError {
    corpus: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    sources: Vec<String>,
    class: String,
}

#[derive(Debug)]
struct FailureGroup {
    count: usize,
    samples: Vec<(String, String)>,
}

#[derive(Debug)]
struct UnexpectedFailureRecord {
    corpus: String,
    source: String,
    layer: OfficeGoldenComparisonLayer,
    diagnostic_kind: OfficeGoldenDiagnosticKind,
    page_index: Option<usize>,
    line_index: Option<usize>,
    message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaseVerdict {
    Pass,
    Fail,
    Xfail,
    Xpass,
}

impl CaseVerdict {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Xfail => "XFAIL",
            Self::Xpass => "XPASS",
        }
    }
}

fn classify_case_verdict(
    expected_layer: Option<OfficeGoldenComparisonLayer>,
    failure_layer: Option<OfficeGoldenComparisonLayer>,
) -> CaseVerdict {
    match (expected_layer, failure_layer) {
        (None, None) => CaseVerdict::Pass,
        (Some(_), None) => CaseVerdict::Xpass,
        (Some(expected), Some(actual)) if expected == actual => CaseVerdict::Xfail,
        (_, Some(_)) => CaseVerdict::Fail,
    }
}

const fn exact_case_pass_requirement_met(
    audit_errors: bool,
    passed: usize,
    expected_errors: usize,
) -> bool {
    if audit_errors {
        passed + expected_errors == 1
    } else {
        passed == 1
    }
}

type KnownErrors = BTreeMap<String, BTreeMap<String, ParsedKnownError>>;

struct CorpusIndex {
    environment: EnvironmentRecord,
    candidates: Vec<CorpusCandidate>,
    known_errors: KnownErrors,
}

static CORPUS_INDEX: OnceLock<std::result::Result<CorpusIndex, String>> = OnceLock::new();

pub fn run_office_golden_corpus(
    format: OfficeGoldenFormat,
    pass_target: usize,
) -> std::result::Result<OfficeGoldenCorpusReport, String> {
    if pass_target == 0 {
        return Err("Office golden pass target must be greater than zero".to_string());
    }

    let root = workspace_root();
    let index = corpus_index(&root)?;
    if let Some(task_path) = env::var_os("OOXMLSDK_GOLDEN_WORKER_TASK") {
        return run_office_golden_worker(format, &root, index, Path::new(&task_path));
    }
    let environment = &index.environment;
    let known_errors = &index.known_errors;
    let exact_case = env::var("OOXMLSDK_GOLDEN_CASE").ok();
    let corpus_filter = env::var("OOXMLSDK_GOLDEN_CORPUS").ok();
    let source_contains = env::var("OOXMLSDK_GOLDEN_SOURCE_CONTAINS").ok();
    let package_part_contains = env::var("OOXMLSDK_GOLDEN_PACKAGE_PART_CONTAINS").ok();
    let package_feature = env::var("OOXMLSDK_GOLDEN_PACKAGE_FEATURE").ok();
    let audit_errors = env::var("OOXMLSDK_GOLDEN_AUDIT_ERRORS").is_ok_and(|value| value == "1");
    let error_class_filter = env::var("OOXMLSDK_GOLDEN_ERROR_CLASS").ok();
    let diagnostic_kind_filter = env::var("OOXMLSDK_GOLDEN_DIAGNOSTIC_KIND")
        .ok()
        .map(|value| {
            value
                .parse::<OfficeGoldenDiagnosticKind>()
                .map(|kind| (value, kind))
        })
        .transpose()?;
    let audit_limit_value = env::var("OOXMLSDK_GOLDEN_AUDIT_LIMIT").ok();
    let audit_limit = audit_limit_value
        .as_deref()
        .map(parse_audit_limit)
        .transpose()?;
    let audit_offset = env::var("OOXMLSDK_GOLDEN_AUDIT_OFFSET")
        .ok()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid OOXMLSDK_GOLDEN_AUDIT_OFFSET {value:?}: {error}"))
        })
        .transpose()?
        .unwrap_or_default();
    let trace_cases = env::var("OOXMLSDK_GOLDEN_TRACE_CASES").is_ok_and(|value| value == "1");
    if (error_class_filter.is_some()
        || diagnostic_kind_filter.is_some()
        || audit_limit_value.is_some()
        || audit_offset != 0)
        && !audit_errors
    {
        return Err(
            "OOXMLSDK_GOLDEN_ERROR_CLASS, OOXMLSDK_GOLDEN_DIAGNOSTIC_KIND, OOXMLSDK_GOLDEN_AUDIT_LIMIT, and OOXMLSDK_GOLDEN_AUDIT_OFFSET require OOXMLSDK_GOLDEN_AUDIT_ERRORS=1"
                .to_string(),
        );
    }
    let diagnostic_cases = diagnostic_kind_filter
        .as_ref()
        .map(|(_, kind)| load_diagnostic_cases(&root, format, *kind))
        .transpose()?;
    let package_part_cases = package_part_contains
        .as_deref()
        .map(|needle| package_part_cases(&root, &index.candidates, format, needle))
        .transpose()?;
    let package_feature_cases = package_feature
        .as_deref()
        .map(|feature| package_feature_cases(&root, &index.candidates, format, feature))
        .transpose()?;

    let mut candidates = index
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.record.source_extension == format.extension()
                && candidate.record.status == "converted"
                && corpus_filter
                    .as_deref()
                    .is_none_or(|corpus| candidate.corpus == corpus)
                && source_contains
                    .as_deref()
                    .is_none_or(|needle| candidate.record.file.contains(needle))
                && package_part_cases.as_ref().is_none_or(|cases| {
                    cases.contains(&(candidate.corpus.clone(), candidate.record.file.clone()))
                })
                && package_feature_cases.as_ref().is_none_or(|cases| {
                    cases.contains(&(candidate.corpus.clone(), candidate.record.file.clone()))
                })
                && exact_case.as_deref().is_none_or(|case| {
                    case == format!("{}/{}", candidate.corpus, candidate.record.file)
                })
                && error_class_filter.as_deref().is_none_or(|class_id| {
                    known_errors
                        .get(&candidate.corpus)
                        .and_then(|sources| sources.get(&candidate.record.file))
                        .is_some_and(|error| error.class_id == class_id)
                })
                && diagnostic_cases.as_ref().is_none_or(|cases| {
                    cases.contains(&(candidate.corpus.clone(), candidate.record.file.clone()))
                })
        })
        .collect::<Vec<_>>();
    if exact_case.is_some() && candidates.is_empty() {
        return Err(format!(
            "OOXMLSDK_GOLDEN_CASE did not select a converted {} record",
            format.extension()
        ));
    }
    if source_contains.is_some() && candidates.is_empty() {
        return Err(format!(
            "OOXMLSDK_GOLDEN_SOURCE_CONTAINS did not select a converted {} record",
            format.extension()
        ));
    }
    if package_part_contains.is_some() && candidates.is_empty() {
        return Err(format!(
            "OOXMLSDK_GOLDEN_PACKAGE_PART_CONTAINS did not select a converted {} record",
            format.extension()
        ));
    }
    if package_feature.is_some() && candidates.is_empty() {
        return Err(format!(
            "OOXMLSDK_GOLDEN_PACKAGE_FEATURE did not select a converted {} record",
            format.extension()
        ));
    }
    if let Some((value, _)) = diagnostic_kind_filter.as_ref()
        && candidates.is_empty()
    {
        return Err(format!(
            "OOXMLSDK_GOLDEN_DIAGNOSTIC_KIND={value:?} did not select an indexed {} record",
            format.extension()
        ));
    }

    if audit_errors && exact_case.is_none() {
        let audit_limit = audit_limit.unwrap_or(AuditLimit::Page(DEFAULT_AUDIT_PAGE_SIZE));
        if !matches!(audit_limit, AuditLimit::All) {
            candidates.retain(|candidate| {
                known_errors
                    .get(&candidate.corpus)
                    .is_some_and(|sources| sources.contains_key(&candidate.record.file))
            });
        }
        let window = audit_window(candidates.len(), audit_offset, audit_limit)?;
        candidates = candidates
            .into_iter()
            .skip(window.offset)
            .take(window.selected)
            .collect();
        let require_pass_target = error_class_filter.is_none()
            && diagnostic_kind_filter.is_none()
            && matches!(audit_limit, AuditLimit::All)
            && audit_offset == 0
            && corpus_filter.is_none()
            && source_contains.is_none()
            && package_part_contains.is_none()
            && package_feature.is_none();
        return run_batch_audit(
            format,
            pass_target,
            &root,
            environment,
            known_errors,
            candidates,
            BatchAuditOptions {
                trace_cases,
                require_pass_target,
                window,
            },
        );
    }

    if exact_case.is_none() {
        return run_normal_ratchet(
            format,
            pass_target,
            &root,
            environment,
            known_errors,
            candidates,
            trace_cases,
        );
    }

    let mut attempted = 0usize;
    let mut passed = 0usize;
    let mut expected_errors = 0usize;
    let mut stale_errors = Vec::new();
    let mut unexpected = BTreeMap::<OfficeGoldenComparisonLayer, FailureGroup>::new();
    let mut unexpected_records = Vec::new();
    let mut expected_records = Vec::new();
    let mut unexpected_count = 0usize;

    for candidate in candidates {
        let expected_error = known_errors
            .get(&candidate.corpus)
            .and_then(|sources| sources.get(&candidate.record.file));
        if exact_case.is_none() && passed >= pass_target {
            if audit_errors && expected_error.is_some() {
                // Continue below and audit this exact known error.
            } else if audit_errors {
                continue;
            } else {
                break;
            }
        }
        attempted += 1;
        // Batch audits have no per-case watchdog. Keep explicitly marked
        // nonterminating failures executable through the exact-case lane,
        // where callers can provide an external timeout, without skipping
        // other conversion failures that may have become stale.
        if audit_errors
            && exact_case.is_none()
            && expected_error.is_some_and(|expected| expected.skip_batch_audit)
        {
            expected_errors += 1;
            continue;
        }
        if expected_error.is_some() && !audit_errors && exact_case.is_none() {
            expected_errors += 1;
            continue;
        }
        let case_id = format!(
            "{}_{}",
            candidate.corpus.to_ascii_lowercase().replace('-', "_"),
            candidate.record.source_sha256
        );
        let case = OfficeGoldenCase {
            id: &case_id,
            corpus: &candidate.corpus,
            source: &candidate.record.file,
            source_sha256: &candidate.record.source_sha256,
            golden_sha256: &candidate.record.output_sha256,
            environment_id: &candidate.record.environment_id,
            ui_language: &environment.environment.locale.ui_culture,
            format_locale: &environment.environment.locale.culture,
        };
        let write_failure_artifacts = exact_case.is_some() || expected_error.is_none();
        let case_key = format!("{}/{}", candidate.corpus, candidate.record.file);
        if trace_cases {
            eprintln!("office-golden start {case_key}");
        }
        let started = Instant::now();
        let comparison = catch_conversion_panic(|| {
            compare_office_golden_detailed_with_artifacts(
                case,
                VisualTolerance::OFFICE_FIXED_OUTPUT,
                write_failure_artifacts,
            )
        });
        let elapsed = started.elapsed();
        if trace_cases || elapsed >= SLOW_CASE_THRESHOLD {
            let status = comparison.as_ref().map_or_else(
                |failure| format!("error:{}", failure.layer.as_str()),
                |_| "pass".to_string(),
            );
            eprintln!(
                "office-golden finish {case_key} status={status} elapsed_ms={}",
                elapsed.as_millis()
            );
        }
        let verdict = classify_case_verdict(
            expected_error.map(|expected| expected.layer),
            comparison.as_ref().err().map(|failure| failure.layer),
        );
        if exact_case.is_some() {
            eprintln!("office-golden verdict {case_key} {}", verdict.as_str());
        }
        match (verdict, comparison) {
            (CaseVerdict::Pass, Ok(_)) => {
                passed += 1;
            }
            (CaseVerdict::Xpass, Ok(_)) => {
                passed += 1;
                stale_errors.push(format!("{}/{}", candidate.corpus, candidate.record.file));
            }
            (CaseVerdict::Xfail, Err(failure)) => {
                expected_records.push(UnexpectedFailureRecord {
                    corpus: candidate.corpus.clone(),
                    source: candidate.record.file.clone(),
                    layer: failure.layer,
                    diagnostic_kind: failure.diagnostic_kind,
                    page_index: failure.page_index,
                    line_index: failure.line_index,
                    message: failure.message,
                });
                expected_errors += 1;
            }
            (CaseVerdict::Fail, Err(failure)) => {
                unexpected_count += 1;
                unexpected_records.push(UnexpectedFailureRecord {
                    corpus: candidate.corpus.clone(),
                    source: candidate.record.file.clone(),
                    layer: failure.layer,
                    diagnostic_kind: failure.diagnostic_kind,
                    page_index: failure.page_index,
                    line_index: failure.line_index,
                    message: failure.message.clone(),
                });
                let group = unexpected.entry(failure.layer).or_insert(FailureGroup {
                    count: 0,
                    samples: Vec::new(),
                });
                group.count += 1;
                if group.samples.len() < FAILURE_SAMPLE_LIMIT {
                    group.samples.push((
                        format!("{}/{}", candidate.corpus, candidate.record.file),
                        failure.message,
                    ));
                }
                if unexpected_count >= UNEXPECTED_FAILURE_LIMIT {
                    break;
                }
            }
            _ => unreachable!("case verdict must match its comparison result"),
        }
    }

    let error_report_path = write_error_report(
        &root,
        ErrorReportMode {
            format,
            exact_case: exact_case.as_deref(),
            audit_errors,
        },
        &unexpected_records,
        &expected_records,
        &stale_errors,
        &BTreeSet::new(),
    )?;

    let pass_requirement_met = if exact_case.is_some() {
        exact_case_pass_requirement_met(audit_errors, passed, expected_errors)
    } else {
        passed >= pass_target
    };
    if !stale_errors.is_empty() || !unexpected.is_empty() || !pass_requirement_met {
        return Err(format_failure_summary(
            format,
            pass_target,
            ScanCounts {
                attempted,
                passed,
                expected_errors,
            },
            &stale_errors,
            &unexpected,
            &expected_records,
            &error_report_path,
        ));
    }

    Ok(OfficeGoldenCorpusReport {
        format,
        attempted,
        passed,
        expected_errors,
    })
}

#[derive(Default)]
struct AuditState {
    attempted: usize,
    passed: usize,
    expected_errors: usize,
    skipped_known_errors: usize,
    audited_cases: BTreeSet<(String, String)>,
    stale_errors: Vec<String>,
    unexpected: BTreeMap<OfficeGoldenComparisonLayer, FailureGroup>,
    unexpected_records: Vec<UnexpectedFailureRecord>,
    expected_records: Vec<UnexpectedFailureRecord>,
    unexpected_count: usize,
}

impl AuditState {
    fn record_unexpected(&mut self, corpus: &str, source: &str, failure: OfficeGoldenFailure) {
        self.unexpected_count += 1;
        let layer = failure.layer;
        let diagnostic_kind = failure.diagnostic_kind;
        let page_index = failure.page_index;
        let line_index = failure.line_index;
        let message = failure.message;
        self.unexpected_records.push(UnexpectedFailureRecord {
            corpus: corpus.to_string(),
            source: source.to_string(),
            layer,
            diagnostic_kind,
            page_index,
            line_index,
            message: message.clone(),
        });
        let group = self.unexpected.entry(layer).or_insert(FailureGroup {
            count: 0,
            samples: Vec::new(),
        });
        group.count += 1;
        if group.samples.len() < FAILURE_SAMPLE_LIMIT {
            group.samples.push((format!("{corpus}/{source}"), message));
        }
    }
}

enum NormalRatchetItem<'a> {
    ExpectedError,
    Candidate(&'a CorpusCandidate),
}

fn run_normal_ratchet(
    format: OfficeGoldenFormat,
    pass_target: usize,
    root: &Path,
    environment: &EnvironmentRecord,
    known_errors: &KnownErrors,
    candidates: Vec<&CorpusCandidate>,
    trace_cases: bool,
) -> std::result::Result<OfficeGoldenCorpusReport, String> {
    let jobs = batch_audit_jobs()?;
    let locale = &environment.environment.locale;
    let mut state = AuditState::default();
    let executable_limit = pass_target.saturating_add(UNEXPECTED_FAILURE_LIMIT);
    let mut items = Vec::new();
    let mut process_cases = Vec::new();
    for candidate in candidates {
        let expected_error = known_errors
            .get(&candidate.corpus)
            .and_then(|sources| sources.get(&candidate.record.file));
        if expected_error.is_some() {
            items.push(NormalRatchetItem::ExpectedError);
        } else {
            let order = items.len();
            items.push(NormalRatchetItem::Candidate(candidate));
            process_cases.push(ProcessAuditCase {
                order,
                candidate,
                write_failure_artifacts: true,
            });
            if process_cases.len() >= executable_limit {
                break;
            }
        }
    }
    let mut results = compare_candidates_in_processes(
        format,
        root,
        &locale.ui_culture,
        &locale.culture,
        trace_cases,
        jobs,
        &process_cases,
    )?
    .into_iter()
    .map(|result| (result.order, result))
    .collect::<BTreeMap<_, _>>();
    for (order, item) in items.into_iter().enumerate() {
        if state.passed >= pass_target || state.unexpected_count >= UNEXPECTED_FAILURE_LIMIT {
            break;
        }
        state.attempted += 1;
        match item {
            NormalRatchetItem::ExpectedError => state.expected_errors += 1,
            NormalRatchetItem::Candidate(candidate) => {
                let result = results
                    .remove(&order)
                    .expect("every normal ratchet candidate has one comparison result");
                match result.failure {
                    None => state.passed += 1,
                    Some(failure) => {
                        state.record_unexpected(&candidate.corpus, &candidate.record.file, failure)
                    }
                }
            }
        }
    }

    let error_report_path = write_error_report(
        root,
        ErrorReportMode {
            format,
            exact_case: None,
            audit_errors: false,
        },
        &state.unexpected_records,
        &[],
        &[],
        &BTreeSet::new(),
    )?;
    if !state.unexpected.is_empty() || state.passed < pass_target {
        return Err(format_failure_summary(
            format,
            pass_target,
            ScanCounts {
                attempted: state.attempted,
                passed: state.passed,
                expected_errors: state.expected_errors,
            },
            &[],
            &state.unexpected,
            &[],
            &error_report_path,
        ));
    }

    Ok(OfficeGoldenCorpusReport {
        format,
        attempted: state.attempted,
        passed: state.passed,
        expected_errors: state.expected_errors,
    })
}

struct KnownAuditCase<'a> {
    order: usize,
    candidate: &'a CorpusCandidate,
    expected: &'a ParsedKnownError,
}

struct KnownAuditResult {
    corpus: String,
    source: String,
    expected_layer: OfficeGoldenComparisonLayer,
    comparison: std::result::Result<(), OfficeGoldenFailure>,
}

#[derive(Deserialize, Serialize)]
struct GoldenWorkerTask {
    order: usize,
    corpus: String,
    source: String,
    write_failure_artifacts: bool,
}

#[derive(Deserialize, Serialize)]
struct GoldenWorkerFailure {
    layer: String,
    diagnostic_kind: String,
    page_index: Option<usize>,
    line_index: Option<usize>,
    message: String,
}

#[derive(Deserialize, Serialize)]
struct GoldenWorkerResult {
    order: usize,
    corpus: String,
    source: String,
    failure: Option<GoldenWorkerFailure>,
}

struct ProcessAuditCase<'a> {
    order: usize,
    candidate: &'a CorpusCandidate,
    write_failure_artifacts: bool,
}

struct ProcessAuditResult {
    order: usize,
    corpus: String,
    source: String,
    failure: Option<OfficeGoldenFailure>,
}

#[derive(Clone, Copy)]
struct BatchAuditOptions {
    trace_cases: bool,
    require_pass_target: bool,
    window: AuditWindow,
}

fn run_batch_audit(
    format: OfficeGoldenFormat,
    pass_target: usize,
    root: &Path,
    environment: &EnvironmentRecord,
    known_errors: &KnownErrors,
    candidates: Vec<&CorpusCandidate>,
    options: BatchAuditOptions,
) -> std::result::Result<OfficeGoldenCorpusReport, String> {
    let mut unclassified = Vec::new();
    let mut known = Vec::new();
    for (order, candidate) in candidates.into_iter().enumerate() {
        match known_errors
            .get(&candidate.corpus)
            .and_then(|sources| sources.get(&candidate.record.file))
        {
            Some(expected) => known.push(KnownAuditCase {
                order,
                candidate,
                expected,
            }),
            None => unclassified.push(candidate),
        }
    }

    let jobs = batch_audit_jobs()?;
    let locale = &environment.environment.locale;

    // A paged audit preserves the normal ratchet's deterministic small-file
    // prefix. An explicit `all` audit is a coverage claim, so it must execute
    // every selected unclassified record instead of stopping once the ratchet
    // target has been observed.
    let mut state = AuditState::default();
    let unclassified_limit =
        audit_unclassified_limit(unclassified.len(), pass_target, options.window.exhaustive);
    let unclassified_cases = unclassified
        .into_iter()
        .take(unclassified_limit)
        .enumerate()
        .map(|(order, candidate)| ProcessAuditCase {
            order,
            candidate,
            write_failure_artifacts: true,
        })
        .collect::<Vec<_>>();
    for result in compare_candidates_in_processes(
        format,
        root,
        &locale.ui_culture,
        &locale.culture,
        options.trace_cases,
        jobs,
        &unclassified_cases,
    )? {
        if !options.window.exhaustive
            && (state.passed >= pass_target || state.unexpected_count >= UNEXPECTED_FAILURE_LIMIT)
        {
            break;
        }
        state
            .audited_cases
            .insert((result.corpus.clone(), result.source.clone()));
        state.attempted += 1;
        match result.failure {
            None => state.passed += 1,
            Some(failure) => state.record_unexpected(&result.corpus, &result.source, failure),
        }
    }

    if options.window.exhaustive || state.unexpected_count < UNEXPECTED_FAILURE_LIMIT {
        let mut executable = Vec::with_capacity(known.len());
        for case in known {
            state.attempted += 1;
            if case.expected.skip_batch_audit {
                state.expected_errors += 1;
                state.skipped_known_errors += 1;
            } else {
                executable.push(case);
            }
        }

        let expected_layers = executable
            .iter()
            .map(|case| (case.order, case.expected.layer))
            .collect::<BTreeMap<_, _>>();
        let process_cases = executable
            .into_iter()
            .map(|case| ProcessAuditCase {
                order: case.order,
                candidate: case.candidate,
                write_failure_artifacts: false,
            })
            .collect::<Vec<_>>();
        let results = compare_candidates_in_processes(
            format,
            root,
            &locale.ui_culture,
            &locale.culture,
            options.trace_cases,
            jobs,
            &process_cases,
        )?
        .into_iter()
        .map(|result| {
            let order = result.order;
            let expected_layer = *expected_layers
                .get(&order)
                .ok_or_else(|| format!("Office golden worker returned unexpected order {order}"))?;
            Ok(KnownAuditResult {
                corpus: result.corpus,
                source: result.source,
                expected_layer,
                comparison: result.failure.map_or(Ok(()), Err),
            })
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;

        for result in results {
            state
                .audited_cases
                .insert((result.corpus.clone(), result.source.clone()));
            let verdict = classify_case_verdict(
                Some(result.expected_layer),
                result
                    .comparison
                    .as_ref()
                    .err()
                    .map(|failure| failure.layer),
            );
            match (verdict, result.comparison) {
                (CaseVerdict::Xpass, Ok(())) => {
                    state.passed += 1;
                    state
                        .stale_errors
                        .push(format!("{}/{}", result.corpus, result.source));
                }
                (CaseVerdict::Xfail, Err(failure)) => {
                    state.expected_records.push(UnexpectedFailureRecord {
                        corpus: result.corpus,
                        source: result.source,
                        layer: failure.layer,
                        diagnostic_kind: failure.diagnostic_kind,
                        page_index: failure.page_index,
                        line_index: failure.line_index,
                        message: failure.message,
                    });
                    state.expected_errors += 1;
                }
                (CaseVerdict::Fail, Err(failure))
                    if options.window.exhaustive
                        || state.unexpected_count < UNEXPECTED_FAILURE_LIMIT =>
                {
                    state.record_unexpected(&result.corpus, &result.source, failure);
                }
                (CaseVerdict::Fail, Err(_)) => {}
                _ => unreachable!("known-error verdict must match its comparison result"),
            }
        }
    }

    let error_report_path = write_error_report(
        root,
        ErrorReportMode {
            format,
            exact_case: None,
            audit_errors: true,
        },
        &state.unexpected_records,
        &state.expected_records,
        &state.stale_errors,
        &state.audited_cases,
    )?;
    eprintln!(
        "office-golden audit-window {} offset={} selected={} total={} next_offset={} mode={}",
        format.extension(),
        options.window.offset,
        options.window.selected,
        options.window.total,
        options
            .window
            .next_offset
            .map_or_else(|| "done".to_string(), |offset| offset.to_string()),
        if options.window.exhaustive {
            "all"
        } else {
            "page"
        }
    );
    if !state.stale_errors.is_empty()
        || !state.unexpected.is_empty()
        || (options.require_pass_target && state.passed < pass_target)
    {
        return Err(format_failure_summary(
            format,
            pass_target,
            ScanCounts {
                attempted: state.attempted,
                passed: state.passed,
                expected_errors: state.expected_errors,
            },
            &state.stale_errors,
            &state.unexpected,
            &state.expected_records,
            &error_report_path,
        ));
    }

    eprintln!(
        "office-golden audit {} attempted={} verdicts=PASS:{} XFAIL:{} XPASS:0 FAIL:0 skipped={} records={}",
        format.extension(),
        state.attempted,
        state.passed,
        state.expected_errors - state.skipped_known_errors,
        state.skipped_known_errors,
        error_report_path.display()
    );

    Ok(OfficeGoldenCorpusReport {
        format,
        attempted: state.attempted,
        passed: state.passed,
        expected_errors: state.expected_errors,
    })
}

fn audit_unclassified_limit(total: usize, pass_target: usize, exhaustive: bool) -> usize {
    if exhaustive {
        total
    } else {
        pass_target
            .saturating_add(UNEXPECTED_FAILURE_LIMIT)
            .min(total)
    }
}

fn parse_audit_limit(value: &str) -> std::result::Result<AuditLimit, String> {
    if value.eq_ignore_ascii_case("all") {
        return Ok(AuditLimit::All);
    }
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid OOXMLSDK_GOLDEN_AUDIT_LIMIT {value:?}: {error}"))
        .and_then(|limit| {
            (limit > 0)
                .then_some(AuditLimit::Page(limit))
                .ok_or_else(|| "OOXMLSDK_GOLDEN_AUDIT_LIMIT must be positive or `all`".to_string())
        })
}

fn audit_window(
    total: usize,
    offset: usize,
    limit: AuditLimit,
) -> std::result::Result<AuditWindow, String> {
    if offset >= total {
        return Err(format!(
            "Office golden audit offset {offset} selected no records from total={total}"
        ));
    }
    let (end, exhaustive) = match limit {
        AuditLimit::Page(limit) => (offset.saturating_add(limit).min(total), false),
        AuditLimit::All => (total, true),
    };
    Ok(AuditWindow {
        offset,
        selected: end - offset,
        total,
        next_offset: (end < total).then_some(end),
        exhaustive,
    })
}

fn batch_audit_jobs() -> std::result::Result<usize, String> {
    let default = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(4);
    env::var("OOXMLSDK_GOLDEN_JOBS")
        .ok()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid OOXMLSDK_GOLDEN_JOBS {value:?}: {error}"))
                .and_then(|jobs| {
                    (jobs > 0)
                        .then_some(jobs)
                        .ok_or_else(|| "OOXMLSDK_GOLDEN_JOBS must be positive".to_string())
                })
        })
        .transpose()
        .map(|jobs| jobs.unwrap_or(default))
}

fn compare_candidates_in_processes(
    format: OfficeGoldenFormat,
    root: &Path,
    ui_language: &str,
    format_locale: &str,
    trace_cases: bool,
    jobs: usize,
    cases: &[ProcessAuditCase<'_>],
) -> std::result::Result<Vec<ProcessAuditResult>, String> {
    if cases.is_empty() {
        return Ok(Vec::new());
    }
    if jobs == 1 {
        return Ok(cases
            .iter()
            .map(|case| ProcessAuditResult {
                order: case.order,
                corpus: case.candidate.corpus.clone(),
                source: case.candidate.record.file.clone(),
                failure: compare_candidate(
                    case.candidate,
                    ui_language,
                    format_locale,
                    trace_cases,
                    case.write_failure_artifacts,
                )
                .err(),
            })
            .collect());
    }

    let worker_count = jobs.min(cases.len());
    let mut buckets = (0..worker_count)
        .map(|_| (0_u64, Vec::<&ProcessAuditCase<'_>>::new()))
        .collect::<Vec<_>>();
    let mut weighted_cases = cases.iter().collect::<Vec<_>>();
    weighted_cases.sort_by_key(|case| {
        std::cmp::Reverse(
            case.candidate
                .record
                .source_bytes
                .saturating_add(case.candidate.record.output_bytes),
        )
    });
    for case in weighted_cases {
        let bucket = buckets
            .iter_mut()
            .min_by_key(|(weight, _)| *weight)
            .expect("at least one Office golden worker bucket");
        bucket.0 = bucket.0.saturating_add(
            case.candidate
                .record
                .source_bytes
                .saturating_add(case.candidate.record.output_bytes),
        );
        bucket.1.push(case);
    }

    static INVOCATION: AtomicU64 = AtomicU64::new(0);
    let invocation = INVOCATION.fetch_add(1, Ordering::Relaxed);
    let worker_root = root.join("target/office-golden/workers");
    fs::create_dir_all(&worker_root)
        .map_err(|error| format!("could not create {}: {error}", worker_root.display()))?;
    let executable = env::current_exe()
        .map_err(|error| format!("could not locate Office golden test executable: {error}"))?;
    let mut children = Vec::with_capacity(worker_count);

    for (worker_index, (_, bucket)) in buckets.into_iter().enumerate() {
        let stem = format!(
            "{}-{}-{}-{worker_index}",
            format.extension(),
            std::process::id(),
            invocation
        );
        let task_path = worker_root.join(format!("{stem}.tasks.json"));
        let result_path = worker_root.join(format!("{stem}.results.json"));
        let tasks = bucket
            .into_iter()
            .map(|case| GoldenWorkerTask {
                order: case.order,
                corpus: case.candidate.corpus.clone(),
                source: case.candidate.record.file.clone(),
                write_failure_artifacts: case.write_failure_artifacts,
            })
            .collect::<Vec<_>>();
        let task_file = File::create(&task_path)
            .map_err(|error| format!("could not create {}: {error}", task_path.display()))?;
        serde_json::to_writer(BufWriter::new(task_file), &tasks)
            .map_err(|error| format!("could not write {}: {error}", task_path.display()))?;

        let mut command = Command::new(&executable);
        command
            .arg(format.ratchet_test_name())
            .arg("--exact")
            .arg("--ignored")
            .arg("--nocapture")
            .arg("--test-threads=1")
            .env("OOXMLSDK_GOLDEN_WORKER_TASK", &task_path)
            .env("OOXMLSDK_GOLDEN_WORKER_RESULT", &result_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if trace_cases {
            command.env("OOXMLSDK_GOLDEN_TRACE_CASES", "1");
        } else {
            command.env_remove("OOXMLSDK_GOLDEN_TRACE_CASES");
        }
        let child = command.spawn().map_err(|error| {
            format!(
                "could not start Office golden worker {}: {error}",
                executable.display()
            )
        })?;
        children.push((child, task_path, result_path));
    }

    let mut results = Vec::with_capacity(cases.len());
    for (child, task_path, result_path) in children {
        let output = child.wait_with_output().map_err(|error| {
            format!(
                "could not wait for Office golden worker {}: {error}",
                executable.display()
            )
        })?;
        if !output.status.success() {
            return Err(format!(
                "Office golden worker exited with {}; stdout={}; stderr={}; task={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                task_path.display()
            ));
        }
        let result_file = File::open(&result_path)
            .map_err(|error| format!("could not open {}: {error}", result_path.display()))?;
        let worker_results: Vec<GoldenWorkerResult> = serde_json::from_reader(result_file)
            .map_err(|error| format!("could not read {}: {error}", result_path.display()))?;
        results.extend(
            worker_results
                .into_iter()
                .map(golden_worker_result_into_process_result)
                .collect::<std::result::Result<Vec<_>, _>>()?,
        );
        let _ = fs::remove_file(task_path);
        let _ = fs::remove_file(result_path);
    }
    results.sort_by_key(|result| result.order);
    if results.len() != cases.len() {
        return Err(format!(
            "Office golden workers returned {} results for {} tasks",
            results.len(),
            cases.len()
        ));
    }
    Ok(results)
}

fn run_office_golden_worker(
    format: OfficeGoldenFormat,
    _root: &Path,
    index: &CorpusIndex,
    task_path: &Path,
) -> std::result::Result<OfficeGoldenCorpusReport, String> {
    let result_path = env::var_os("OOXMLSDK_GOLDEN_WORKER_RESULT")
        .map(PathBuf::from)
        .ok_or_else(|| "OOXMLSDK_GOLDEN_WORKER_RESULT is required in worker mode".to_string())?;
    let task_file = File::open(task_path)
        .map_err(|error| format!("could not open {}: {error}", task_path.display()))?;
    let tasks: Vec<GoldenWorkerTask> = serde_json::from_reader(task_file)
        .map_err(|error| format!("could not read {}: {error}", task_path.display()))?;
    if tasks.is_empty() {
        return Err("Office golden worker received no tasks".to_string());
    }
    let candidates = index
        .candidates
        .iter()
        .filter(|candidate| candidate.record.source_extension == format.extension())
        .map(|candidate| {
            (
                (candidate.corpus.as_str(), candidate.record.file.as_str()),
                candidate,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let trace_cases = env::var("OOXMLSDK_GOLDEN_TRACE_CASES").is_ok_and(|value| value == "1");
    let locale = &index.environment.environment.locale;
    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        let candidate = candidates
            .get(&(task.corpus.as_str(), task.source.as_str()))
            .ok_or_else(|| {
                format!(
                    "Office golden worker task did not match a {} candidate: {}/{}",
                    format.extension(),
                    task.corpus,
                    task.source
                )
            })?;
        let failure = compare_candidate(
            candidate,
            &locale.ui_culture,
            &locale.culture,
            trace_cases,
            task.write_failure_artifacts,
        )
        .err()
        .map(|failure| GoldenWorkerFailure {
            layer: failure.layer.as_str().to_string(),
            diagnostic_kind: failure.diagnostic_kind.as_str().to_string(),
            page_index: failure.page_index,
            line_index: failure.line_index,
            message: failure.message,
        });
        results.push(GoldenWorkerResult {
            order: task.order,
            corpus: task.corpus,
            source: task.source,
            failure,
        });
    }
    let result_file = File::create(&result_path)
        .map_err(|error| format!("could not create {}: {error}", result_path.display()))?;
    serde_json::to_writer(BufWriter::new(result_file), &results)
        .map_err(|error| format!("could not write {}: {error}", result_path.display()))?;
    Ok(OfficeGoldenCorpusReport {
        format,
        attempted: results.len(),
        passed: results
            .iter()
            .filter(|result| result.failure.is_none())
            .count(),
        expected_errors: results
            .iter()
            .filter(|result| result.failure.is_some())
            .count(),
    })
}

fn golden_worker_result_into_process_result(
    result: GoldenWorkerResult,
) -> std::result::Result<ProcessAuditResult, String> {
    let failure = match result.failure {
        Some(failure) => Some(OfficeGoldenFailure {
            layer: failure.layer.parse()?,
            diagnostic_kind: failure.diagnostic_kind.parse()?,
            page_index: failure.page_index,
            line_index: failure.line_index,
            message: failure.message,
        }),
        None => None,
    };
    Ok(ProcessAuditResult {
        order: result.order,
        corpus: result.corpus,
        source: result.source,
        failure,
    })
}

fn compare_candidate(
    candidate: &CorpusCandidate,
    ui_language: &str,
    format_locale: &str,
    trace_cases: bool,
    write_failure_artifacts: bool,
) -> std::result::Result<(), OfficeGoldenFailure> {
    let case_id = format!(
        "{}_{}",
        candidate.corpus.to_ascii_lowercase().replace('-', "_"),
        candidate.record.source_sha256
    );
    let case = OfficeGoldenCase {
        id: &case_id,
        corpus: &candidate.corpus,
        source: &candidate.record.file,
        source_sha256: &candidate.record.source_sha256,
        golden_sha256: &candidate.record.output_sha256,
        environment_id: &candidate.record.environment_id,
        ui_language,
        format_locale,
    };
    let case_key = format!("{}/{}", candidate.corpus, candidate.record.file);
    if trace_cases {
        eprintln!("office-golden start {case_key}");
    }
    let started = Instant::now();
    let comparison = catch_conversion_panic(|| {
        compare_office_golden_detailed_with_artifacts(
            case,
            VisualTolerance::OFFICE_FIXED_OUTPUT,
            write_failure_artifacts,
        )
        .map(|_| ())
    });
    let elapsed = started.elapsed();
    if trace_cases || elapsed >= SLOW_CASE_THRESHOLD {
        let status = comparison.as_ref().map_or_else(
            |failure| format!("error:{}", failure.layer.as_str()),
            |_| "pass".to_string(),
        );
        eprintln!(
            "office-golden finish {case_key} status={status} elapsed_ms={}",
            elapsed.as_millis()
        );
    }
    comparison
}

fn catch_conversion_panic<T>(
    operation: impl FnOnce() -> std::result::Result<T, OfficeGoldenFailure>,
) -> std::result::Result<T, OfficeGoldenFailure> {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic payload");
        Err(OfficeGoldenFailure {
            layer: OfficeGoldenComparisonLayer::Conversion,
            diagnostic_kind: OfficeGoldenDiagnosticKind::CandidateConversion,
            page_index: None,
            line_index: None,
            message: format!("Office golden candidate conversion panicked: {message}"),
        })
    })
}

#[derive(Clone, Copy)]
struct ErrorReportMode<'a> {
    format: OfficeGoldenFormat,
    exact_case: Option<&'a str>,
    audit_errors: bool,
}

fn write_error_report(
    root: &Path,
    mode: ErrorReportMode<'_>,
    unexpected: &[UnexpectedFailureRecord],
    expected: &[UnexpectedFailureRecord],
    stale_errors: &[String],
    audited_cases: &BTreeSet<(String, String)>,
) -> std::result::Result<PathBuf, String> {
    let ErrorReportMode {
        format,
        exact_case,
        audit_errors,
    } = mode;
    let directory = root.join("target/office-golden");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let path = directory.join(error_report_file_name(format, exact_case, audit_errors));
    let file = fs::File::create(&path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    for failure in unexpected {
        serde_json::to_writer(
            &mut writer,
            &serde_json::json!({
                "status": "unexpected-error",
                "verdict": CaseVerdict::Fail.as_str(),
                "corpus": failure.corpus,
                "source": failure.source,
                "layer": failure.layer.as_str(),
                "diagnostic_kind": failure.diagnostic_kind.as_str(),
                "page_index": failure.page_index,
                "line_index": failure.line_index,
                "message": failure.message,
            }),
        )
        .map_err(|error| format!("could not serialize {}: {error}", path.display()))?;
        writeln!(writer).map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    for failure in expected {
        serde_json::to_writer(
            &mut writer,
            &serde_json::json!({
                "status": "expected-error",
                "verdict": CaseVerdict::Xfail.as_str(),
                "corpus": failure.corpus,
                "source": failure.source,
                "layer": failure.layer.as_str(),
                "diagnostic_kind": failure.diagnostic_kind.as_str(),
                "page_index": failure.page_index,
                "line_index": failure.line_index,
                "message": failure.message,
            }),
        )
        .map_err(|error| format!("could not serialize {}: {error}", path.display()))?;
        writeln!(writer).map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    for case in stale_errors {
        serde_json::to_writer(
            &mut writer,
            &serde_json::json!({
                "status": "stale-error",
                "verdict": CaseVerdict::Xpass.as_str(),
                "case": case,
            }),
        )
        .map_err(|error| format!("could not serialize {}: {error}", path.display()))?;
        writeln!(writer).map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    writer
        .flush()
        .map_err(|error| format!("could not flush {}: {error}", path.display()))?;
    drop(writer);
    if audit_errors && exact_case.is_none() {
        update_diagnostic_index(
            &directory,
            format,
            unexpected,
            expected,
            stale_errors,
            audited_cases,
        )?;
    }
    Ok(path)
}

fn error_report_file_name(
    format: OfficeGoldenFormat,
    exact_case: Option<&str>,
    audit_errors: bool,
) -> String {
    let report_kind = if exact_case.is_some() {
        "case"
    } else if audit_errors {
        "audit"
    } else {
        "scan"
    };
    format!("{report_kind}-{}-errors.jsonl", format.extension())
}

fn diagnostic_index_path(root: &Path, format: OfficeGoldenFormat) -> PathBuf {
    root.join("target/office-golden")
        .join(format!("diagnostic-index-{}.jsonl", format.extension()))
}

fn load_diagnostic_cases(
    root: &Path,
    format: OfficeGoldenFormat,
    diagnostic_kind: OfficeGoldenDiagnosticKind,
) -> std::result::Result<BTreeSet<(String, String)>, String> {
    let path = diagnostic_index_path(root, format);
    let contents = fs::read_to_string(&path).map_err(|error| {
        format!(
            "could not read diagnostic index {}: {error}; run an unfiltered known-error audit first",
            path.display()
        )
    })?;
    let mut cases = BTreeSet::new();
    for (line_index, line) in contents.lines().enumerate() {
        let record: DiagnosticIndexRecord = serde_json::from_str(line).map_err(|error| {
            format!(
                "invalid diagnostic index record at {}:{}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if record.diagnostic_kind == diagnostic_kind.as_str() {
            cases.insert((record.corpus, record.source));
        }
    }
    Ok(cases)
}

fn update_diagnostic_index(
    directory: &Path,
    format: OfficeGoldenFormat,
    unexpected: &[UnexpectedFailureRecord],
    expected: &[UnexpectedFailureRecord],
    stale_errors: &[String],
    audited_cases: &BTreeSet<(String, String)>,
) -> std::result::Result<(), String> {
    let path = directory.join(format!("diagnostic-index-{}.jsonl", format.extension()));
    let mut records = BTreeMap::<(String, String), DiagnosticIndexRecord>::new();
    if path.is_file() {
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        for (line_index, line) in contents.lines().enumerate() {
            let record: DiagnosticIndexRecord = serde_json::from_str(line).map_err(|error| {
                format!(
                    "invalid diagnostic index record at {}:{}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            records.insert((record.corpus.clone(), record.source.clone()), record);
        }
    }
    remove_audited_diagnostic_records(&mut records, audited_cases);
    for stale in stale_errors {
        if let Some((corpus, source)) = stale.split_once('/') {
            records.remove(&(corpus.to_string(), source.to_string()));
        }
    }
    for failure in unexpected.iter().chain(expected) {
        let record = DiagnosticIndexRecord {
            corpus: failure.corpus.clone(),
            source: failure.source.clone(),
            diagnostic_kind: failure.diagnostic_kind.as_str().to_string(),
            layer: failure.layer.as_str().to_string(),
            page_index: failure.page_index,
            line_index: failure.line_index,
            message: failure.message.clone(),
        };
        records.insert((record.corpus.clone(), record.source.clone()), record);
    }

    let file = fs::File::create(&path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    for record in records.values() {
        serde_json::to_writer(
            &mut writer,
            &serde_json::json!({
                "corpus": record.corpus,
                "source": record.source,
                "layer": record.layer,
                "diagnostic_kind": record.diagnostic_kind,
                "page_index": record.page_index,
                "line_index": record.line_index,
                "message": record.message,
            }),
        )
        .map_err(|error| format!("could not serialize {}: {error}", path.display()))?;
        writeln!(writer).map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    writer
        .flush()
        .map_err(|error| format!("could not flush {}: {error}", path.display()))
}

fn remove_audited_diagnostic_records(
    records: &mut BTreeMap<(String, String), DiagnosticIndexRecord>,
    audited_cases: &BTreeSet<(String, String)>,
) {
    for case in audited_cases {
        records.remove(case);
    }
}

fn corpus_index(root: &Path) -> std::result::Result<&'static CorpusIndex, String> {
    CORPUS_INDEX
        .get_or_init(|| load_corpus_index(root))
        .as_ref()
        .map_err(Clone::clone)
}

fn load_corpus_index(root: &Path) -> std::result::Result<CorpusIndex, String> {
    let environment = load_reference_environment(root)?;
    let mut candidates = load_candidates(root)?;
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.record.status == "converted")
    {
        if candidate.record.reference_engine != "Microsoft Office" {
            return Err(format!(
                "converted record {}/{} has reference engine {:?}",
                candidate.corpus, candidate.record.file, candidate.record.reference_engine
            ));
        }
        if candidate.record.environment_id != environment.environment_id {
            return Err(format!(
                "converted record {}/{} has environment {}, expected {}",
                candidate.corpus,
                candidate.record.file,
                candidate.record.environment_id,
                environment.environment_id
            ));
        }
    }
    // Small source/output records are visited first. This is a scheduling
    // choice only: every visited case still runs the full Office contract.
    candidates.sort_by(|left, right| {
        (
            left.record.source_bytes,
            left.record.output_bytes,
            &left.corpus,
            &left.record.file,
        )
            .cmp(&(
                right.record.source_bytes,
                right.record.output_bytes,
                &right.corpus,
                &right.record.file,
            ))
    });
    let known_errors = load_known_errors(root, &candidates)?;
    Ok(CorpusIndex {
        environment,
        candidates,
        known_errors,
    })
}

fn load_candidates(root: &Path) -> std::result::Result<Vec<CorpusCandidate>, String> {
    let conversion_root = root.join("corpus_pdf_conv");
    let mut manifest_paths = child_manifest_paths(&conversion_root)?;
    manifest_paths.sort();
    let mut candidates = Vec::new();
    let mut keys = BTreeSet::new();
    for manifest_path in manifest_paths {
        let corpus = corpus_name(&manifest_path)?;
        let contents = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
        for (line_index, line) in contents.lines().enumerate() {
            let record: ConversionRecord = serde_json::from_str(line).map_err(|error| {
                format!(
                    "invalid conversion record at {}:{}: {error}",
                    manifest_path.display(),
                    line_index + 1
                )
            })?;
            let key = (corpus.clone(), record.file.clone());
            if !keys.insert(key) {
                return Err(format!(
                    "duplicate conversion record for {corpus}/{}",
                    record.file
                ));
            }
            candidates.push(CorpusCandidate {
                corpus: corpus.clone(),
                record,
            });
        }
    }
    Ok(candidates)
}

fn package_part_cases(
    root: &Path,
    candidates: &[CorpusCandidate],
    format: OfficeGoldenFormat,
    needle: &str,
) -> std::result::Result<BTreeSet<(String, String)>, String> {
    let mut cases = BTreeSet::new();
    for candidate in candidates.iter().filter(|candidate| {
        candidate.record.status == "converted"
            && candidate.record.source_extension == format.extension()
    }) {
        let source_path = root
            .join("corpus")
            .join(&candidate.corpus)
            .join(&candidate.record.file);
        let source = File::open(&source_path)
            .map_err(|error| format!("could not open {}: {error}", source_path.display()))?;
        let archive = ZipArchive::new(source).map_err(|error| {
            format!("could not read {} as OOXML: {error}", source_path.display())
        })?;
        if archive.file_names().any(|name| name.contains(needle)) {
            cases.insert((candidate.corpus.clone(), candidate.record.file.clone()));
        }
    }
    Ok(cases)
}

fn package_feature_cases(
    root: &Path,
    candidates: &[CorpusCandidate],
    format: OfficeGoldenFormat,
    feature: &str,
) -> std::result::Result<BTreeSet<(String, String)>, String> {
    if feature != "ole-emf" {
        return Err(format!(
            "unknown OOXMLSDK_GOLDEN_PACKAGE_FEATURE {feature:?}; expected \"ole-emf\""
        ));
    }

    let mut cases = BTreeSet::new();
    for candidate in candidates.iter().filter(|candidate| {
        candidate.record.status == "converted"
            && candidate.record.source_extension == format.extension()
    }) {
        let source_path = root
            .join("corpus")
            .join(&candidate.corpus)
            .join(&candidate.record.file);
        let source = File::open(&source_path)
            .map_err(|error| format!("could not open {}: {error}", source_path.display()))?;
        let mut archive = ZipArchive::new(source).map_err(|error| {
            format!("could not read {} as OOXML: {error}", source_path.display())
        })?;
        if archive_contains_ole_or_metafile(&mut archive, &source_path)? {
            cases.insert((candidate.corpus.clone(), candidate.record.file.clone()));
        }
    }
    Ok(cases)
}

fn archive_contains_ole_or_metafile(
    archive: &mut ZipArchive<File>,
    source_path: &Path,
) -> std::result::Result<bool, String> {
    let mut relationship_indices = Vec::new();
    let mut binary_part_indices = Vec::new();
    for (index, name) in archive.file_names().enumerate() {
        if package_part_is_metafile(name) {
            return Ok(true);
        }
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".rels") {
            relationship_indices.push(index);
        }
        if lower.contains("/embeddings/") || lower.ends_with(".bin") {
            binary_part_indices.push(index);
        }
    }

    for index in relationship_indices {
        let mut relationship = archive.by_index(index).map_err(|error| {
            format!(
                "could not read relationship part in {}: {error}",
                source_path.display()
            )
        })?;
        let mut bytes = Vec::new();
        relationship.read_to_end(&mut bytes).map_err(|error| {
            format!(
                "could not read relationship bytes in {}: {error}",
                source_path.display()
            )
        })?;
        if relationship_contains_ole_object(&bytes) {
            return Ok(true);
        }
    }

    for index in binary_part_indices {
        let mut part = archive.by_index(index).map_err(|error| {
            format!(
                "could not read binary part in {}: {error}",
                source_path.display()
            )
        })?;
        let mut signature = [0_u8; 8];
        if part.read_exact(&mut signature).is_ok() && is_cfb_signature(&signature) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn package_part_is_metafile(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let package_path = lower.trim_start_matches('/');
    // The package thumbnail is application metadata, not a document visual.
    // Selecting it routes unrelated body/layout failures into the OLE/EMF
    // rendering lane even though the PDF exporter never consumes the part.
    if package_path.starts_with("docprops/thumbnail.") {
        return false;
    }
    package_path.ends_with(".emf") || package_path.ends_with(".wmf")
}

fn relationship_contains_ole_object(bytes: &[u8]) -> bool {
    bytes
        .windows(b"relationships/oleObject".len())
        .any(|window| window == b"relationships/oleObject")
}

fn is_cfb_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1])
}

fn child_manifest_paths(root: &Path) -> std::result::Result<Vec<PathBuf>, String> {
    fs::read_dir(root)
        .map_err(|error| format!("could not scan {}: {error}", root.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path().join("manifest.jsonl"))
                .map_err(|error| format!("could not scan {}: {error}", root.display()))
        })
        .filter_map(|result| match result {
            Ok(path) if path.is_file() => Some(Ok(path)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn corpus_name(manifest_path: &Path) -> std::result::Result<String, String> {
    manifest_path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .ok_or_else(|| format!("invalid corpus manifest path {}", manifest_path.display()))
}

fn load_reference_environment(root: &Path) -> std::result::Result<EnvironmentRecord, String> {
    let path = root.join("corpus_pdf_conv/environment.json");
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid reference environment {}: {error}", path.display()))
}

fn load_known_errors(
    root: &Path,
    candidates: &[CorpusCandidate],
) -> std::result::Result<KnownErrors, String> {
    let path = root.join("corpus_pdf_conv/golden-errors.toml");
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let manifest: ErrorManifest = toml::from_str(&contents)
        .map_err(|error| format!("invalid error manifest {}: {error}", path.display()))?;
    if manifest.schema_version != ERROR_MANIFEST_SCHEMA_VERSION {
        return Err(format!(
            "unsupported golden error manifest schema version {}",
            manifest.schema_version
        ));
    }
    let candidate_keys = candidates
        .iter()
        .filter(|candidate| candidate.record.status == "converted")
        .map(|candidate| (candidate.corpus.clone(), candidate.record.file.clone()))
        .collect::<BTreeSet<_>>();
    let mut classes = BTreeMap::new();
    for class in manifest.class {
        if class.id.trim().is_empty()
            || class.reason.trim().is_empty()
            || class.evidence.iter().all(|item| item.trim().is_empty())
        {
            return Err(format!(
                "golden error class {:?} must include an id, reason, and evidence",
                class.id
            ));
        }
        let layer = class.layer.parse::<OfficeGoldenComparisonLayer>()?;
        if class.skip_batch_audit && layer != OfficeGoldenComparisonLayer::Conversion {
            return Err(format!(
                "golden error class {:?} may skip batch audit only for conversion failures",
                class.id
            ));
        }
        let class_id = class.id.clone();
        if classes
            .insert(
                class.id,
                ParsedKnownError {
                    class_id: class_id.clone(),
                    layer,
                    skip_batch_audit: class.skip_batch_audit,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate golden error class {class_id:?}"));
        }
    }
    let mut errors = KnownErrors::new();
    let mut used_classes = BTreeSet::new();
    for error in manifest.error {
        let mut sources = error.sources;
        if let Some(source) = error.source {
            sources.push(source);
        }
        if sources.is_empty() {
            return Err(format!(
                "golden error for corpus {} must include source or sources",
                error.corpus
            ));
        }
        let parsed = classes.get(&error.class).cloned().ok_or_else(|| {
            format!(
                "golden error for corpus {} references unknown class {:?}",
                error.corpus, error.class
            )
        })?;
        used_classes.insert(error.class);
        for source in sources {
            let key = (error.corpus.clone(), source.clone());
            if !candidate_keys.contains(&key) {
                return Err(format!(
                    "golden error {}/{} does not reference a converted record",
                    error.corpus, source
                ));
            }
            if errors
                .entry(error.corpus.clone())
                .or_default()
                .insert(source.clone(), parsed.clone())
                .is_some()
            {
                return Err(format!(
                    "duplicate golden error for {}/{}",
                    error.corpus, source
                ));
            }
        }
    }
    if let Some(unused) = classes.keys().find(|class| !used_classes.contains(*class)) {
        return Err(format!("unused golden error class {unused:?}"));
    }
    Ok(errors)
}

#[derive(Clone, Debug)]
struct ParsedKnownError {
    class_id: String,
    layer: OfficeGoldenComparisonLayer,
    skip_batch_audit: bool,
}

#[derive(Clone, Copy)]
struct ScanCounts {
    attempted: usize,
    passed: usize,
    expected_errors: usize,
}

fn format_failure_summary(
    format: OfficeGoldenFormat,
    pass_target: usize,
    counts: ScanCounts,
    stale_errors: &[String],
    unexpected: &BTreeMap<OfficeGoldenComparisonLayer, FailureGroup>,
    expected: &[UnexpectedFailureRecord],
    error_report_path: &Path,
) -> String {
    let ScanCounts {
        attempted,
        passed,
        expected_errors,
    } = counts;
    let mut output = format!(
        "Office golden {} scan failed: target={pass_target}, attempted={attempted}, passed={passed}, expected_errors={expected_errors}",
        format.extension()
    );
    if !expected.is_empty() {
        let _ = write!(
            output,
            "\nXFAIL known errors: {} (do not count as golden passes)",
            expected.len()
        );
    }
    if !stale_errors.is_empty() {
        let _ = write!(
            output,
            "\nXPASS known-error entries (remove before promotion): {}",
            stale_errors.join(", ")
        );
    }
    for (layer, group) in unexpected {
        let _ = write!(
            output,
            "\nFAIL {} comparisons: {} (showing at most {})",
            layer.as_str(),
            group.count,
            FAILURE_SAMPLE_LIMIT
        );
        for (case, message) in &group.samples {
            let _ = write!(output, "\n  {case}: {message}");
        }
    }
    if unexpected.values().map(|group| group.count).sum::<usize>() >= UNEXPECTED_FAILURE_LIMIT {
        let _ = write!(
            output,
            "\nunexpected failure count reached or exceeded {UNEXPECTED_FAILURE_LIMIT}"
        );
    }
    let _ = write!(output, "\nerror records={}", error_report_path.display());
    output
}

#[cfg(test)]
mod tests {
    use super::{
        AuditLimit, AuditWindow, CaseVerdict, DiagnosticIndexRecord, OfficeGoldenComparisonLayer,
        OfficeGoldenFormat, audit_unclassified_limit, audit_window, catch_conversion_panic,
        classify_case_verdict, error_report_file_name, exact_case_pass_requirement_met,
        format_failure_summary, is_cfb_signature, package_part_is_metafile, parse_audit_limit,
        relationship_contains_ole_object, remove_audited_diagnostic_records,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    #[test]
    fn comparison_layers_use_stable_manifest_names() {
        for layer in [
            OfficeGoldenComparisonLayer::Identity,
            OfficeGoldenComparisonLayer::Conversion,
            OfficeGoldenComparisonLayer::PdfExtraction,
            OfficeGoldenComparisonLayer::PageGeometry,
            OfficeGoldenComparisonLayer::Text,
            OfficeGoldenComparisonLayer::Font,
            OfficeGoldenComparisonLayer::VisibleOutput,
            OfficeGoldenComparisonLayer::ComparisonArtifact,
        ] {
            assert_eq!(layer.as_str().parse(), Ok(layer));
        }
    }

    #[test]
    fn empty_failure_summary_does_not_expand_page_records() {
        let summary = format_failure_summary(
            super::OfficeGoldenFormat::Docx,
            10,
            super::ScanCounts {
                attempted: 4,
                passed: 4,
                expected_errors: 0,
            },
            &[],
            &BTreeMap::new(),
            &[],
            Path::new("target/office-golden/scan-docx-errors.jsonl"),
        );
        assert!(summary.contains("target=10, attempted=4, passed=4"));
    }

    #[test]
    fn candidate_panic_is_recorded_as_a_conversion_failure() {
        let failure =
            catch_conversion_panic::<()>(|| panic!("invalid paint document")).unwrap_err();

        assert_eq!(failure.layer, OfficeGoldenComparisonLayer::Conversion);
        assert!(failure.message.contains("invalid paint document"));
    }

    #[test]
    fn exact_case_report_does_not_replace_the_batch_scan_checkpoint() {
        assert_eq!(
            error_report_file_name(OfficeGoldenFormat::Xlsx, None, false),
            "scan-xlsx-errors.jsonl"
        );
        assert_eq!(
            error_report_file_name(OfficeGoldenFormat::Xlsx, Some("Corpus/file.xlsx"), false),
            "case-xlsx-errors.jsonl"
        );
        assert_eq!(
            error_report_file_name(OfficeGoldenFormat::Xlsx, None, true),
            "audit-xlsx-errors.jsonl"
        );
    }

    #[test]
    fn audited_pass_removes_stale_diagnostic_index_record() {
        let stale_key = ("LibreOffice".to_string(), "stale.docx".to_string());
        let retained_key = ("LibreOffice".to_string(), "retained.docx".to_string());
        let record = |(corpus, source): &(String, String)| DiagnosticIndexRecord {
            corpus: corpus.clone(),
            source: source.clone(),
            diagnostic_kind: "text-content".to_string(),
            layer: "text".to_string(),
            page_index: None,
            line_index: None,
            message: "old diagnostic".to_string(),
        };
        let mut records = BTreeMap::from([
            (stale_key.clone(), record(&stale_key)),
            (retained_key.clone(), record(&retained_key)),
        ]);

        remove_audited_diagnostic_records(&mut records, &BTreeSet::from([stale_key.clone()]));

        assert!(!records.contains_key(&stale_key));
        assert!(records.contains_key(&retained_key));
    }

    #[test]
    fn audit_limit_accepts_bounded_pages_and_explicit_all() {
        assert_eq!(parse_audit_limit("32"), Ok(AuditLimit::Page(32)));
        assert_eq!(parse_audit_limit("all"), Ok(AuditLimit::All));
        assert_eq!(parse_audit_limit("ALL"), Ok(AuditLimit::All));
        assert!(parse_audit_limit("0").is_err());
    }

    #[test]
    fn ole_emf_package_feature_uses_typed_part_evidence() {
        assert!(package_part_is_metafile("word/media/image1.EMF"));
        assert!(package_part_is_metafile("ppt/media/image2.wmf"));
        assert!(!package_part_is_metafile("docProps/thumbnail.emf"));
        assert!(!package_part_is_metafile("/docProps/thumbnail.WMF"));
        assert!(!package_part_is_metafile("word/media/image3.png"));
        assert!(relationship_contains_ole_object(
            br#"Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject""#,
        ));
        assert!(!relationship_contains_ole_object(
            br#"Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package""#,
        ));
        assert!(is_cfb_signature(&[
            0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1,
        ]));
        assert!(!is_cfb_signature(b"PK\x03\x04zip"));
    }

    #[test]
    fn exhaustive_audit_does_not_stop_at_the_ratchet_prefix() {
        assert_eq!(audit_unclassified_limit(2_707, 1_015, true), 2_707);
        assert_eq!(
            audit_unclassified_limit(2_707, 1_015, false),
            1_015 + super::UNEXPECTED_FAILURE_LIMIT
        );
    }

    #[test]
    fn audit_window_reports_the_next_deterministic_page() {
        assert_eq!(
            audit_window(70, 32, AuditLimit::Page(32)),
            Ok(AuditWindow {
                offset: 32,
                selected: 32,
                total: 70,
                next_offset: Some(64),
                exhaustive: false,
            })
        );
        assert_eq!(
            audit_window(70, 64, AuditLimit::Page(32)),
            Ok(AuditWindow {
                offset: 64,
                selected: 6,
                total: 70,
                next_offset: None,
                exhaustive: false,
            })
        );
        assert!(audit_window(70, 70, AuditLimit::Page(32)).is_err());
    }

    #[test]
    fn case_verdicts_separate_comparison_truth_from_known_error_metadata() {
        use OfficeGoldenComparisonLayer::{PageGeometry, Text};

        assert_eq!(classify_case_verdict(None, None), CaseVerdict::Pass);
        assert_eq!(classify_case_verdict(None, Some(Text)), CaseVerdict::Fail);
        assert_eq!(
            classify_case_verdict(Some(Text), Some(Text)),
            CaseVerdict::Xfail
        );
        assert_eq!(classify_case_verdict(Some(Text), None), CaseVerdict::Xpass);
        assert_eq!(
            classify_case_verdict(Some(Text), Some(PageGeometry)),
            CaseVerdict::Fail
        );
    }

    #[test]
    fn exact_case_requires_a_real_pass_unless_audit_is_explicit() {
        assert!(exact_case_pass_requirement_met(false, 1, 0));
        assert!(!exact_case_pass_requirement_met(false, 0, 1));
        assert!(exact_case_pass_requirement_met(true, 0, 1));
    }
}

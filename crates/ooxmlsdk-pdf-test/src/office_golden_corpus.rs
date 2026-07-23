use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufWriter, Write as _};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use serde::Deserialize;

use crate::office_golden::compare_office_golden_detailed_with_artifacts;
use crate::{
    OfficeGoldenCase, OfficeGoldenComparisonLayer, OfficeGoldenDiagnosticKind, OfficeGoldenFailure,
    VisualTolerance, workspace_root,
};

const ERROR_MANIFEST_SCHEMA_VERSION: u32 = 1;
const FAILURE_SAMPLE_LIMIT: usize = 3;
// Keep late-corpus pages bounded: large Office packages can take minutes per
// comparison, so a 32-record page persists useful classification progress
// without forcing a long tail to restart from the previous checkpoint.
const UNEXPECTED_FAILURE_LIMIT: usize = 32;
const SLOW_CASE_THRESHOLD: Duration = Duration::from_secs(2);

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
    let environment = &index.environment;
    let known_errors = &index.known_errors;
    let exact_case = env::var("OOXMLSDK_GOLDEN_CASE").ok();
    let corpus_filter = env::var("OOXMLSDK_GOLDEN_CORPUS").ok();
    let source_contains = env::var("OOXMLSDK_GOLDEN_SOURCE_CONTAINS").ok();
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
    let audit_limit = env::var("OOXMLSDK_GOLDEN_AUDIT_LIMIT")
        .ok()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid OOXMLSDK_GOLDEN_AUDIT_LIMIT {value:?}: {error}"))
                .and_then(|limit| {
                    (limit > 0)
                        .then_some(limit)
                        .ok_or_else(|| "OOXMLSDK_GOLDEN_AUDIT_LIMIT must be positive".to_string())
                })
        })
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
        || audit_limit.is_some()
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
    if audit_offset >= candidates.len() {
        candidates.clear();
    } else if audit_offset != 0 {
        candidates.drain(..audit_offset);
    }
    if let Some(limit) = audit_limit {
        candidates.truncate(limit);
    }

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
    if let Some((value, _)) = diagnostic_kind_filter.as_ref()
        && candidates.is_empty()
    {
        return Err(format!(
            "OOXMLSDK_GOLDEN_DIAGNOSTIC_KIND={value:?} did not select an indexed {} record",
            format.extension()
        ));
    }

    if audit_errors && exact_case.is_none() {
        let require_pass_target = error_class_filter.is_none()
            && diagnostic_kind_filter.is_none()
            && audit_limit.is_none()
            && audit_offset == 0
            && corpus_filter.is_none()
            && source_contains.is_none();
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
        match comparison {
            Ok(_) => {
                passed += 1;
                if expected_error.is_some() {
                    stale_errors.push(format!("{}/{}", candidate.corpus, candidate.record.file));
                }
            }
            Err(failure) => {
                if expected_error.is_some_and(|expected| expected.layer == failure.layer) {
                    if audit_errors {
                        expected_records.push(UnexpectedFailureRecord {
                            corpus: candidate.corpus.clone(),
                            source: candidate.record.file.clone(),
                            layer: failure.layer,
                            diagnostic_kind: failure.diagnostic_kind,
                            page_index: failure.page_index,
                            line_index: failure.line_index,
                            message: failure.message,
                        });
                    }
                    expected_errors += 1;
                    continue;
                }
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
        }
    }

    let error_report_path = write_error_report(
        &root,
        format,
        exact_case.as_deref(),
        audit_errors,
        &unexpected_records,
        &expected_records,
        &stale_errors,
    )?;

    let pass_requirement_met = if exact_case.is_some() {
        passed + expected_errors == 1
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
    let pool = ThreadPoolBuilder::new()
        .num_threads(jobs)
        .thread_name(|index| format!("office-golden-{index}"))
        .build()
        .map_err(|error| format!("could not create Office golden ratchet pool: {error}"))?;
    let ui_language = &environment.environment.locale.ui_culture;
    let mut state = AuditState::default();
    let mut cursor = 0usize;

    while cursor < candidates.len()
        && state.passed < pass_target
        && state.unexpected_count < UNEXPECTED_FAILURE_LIMIT
    {
        // Select in the original deterministic scan order, but compare the
        // next small window concurrently. Processing the results in the same
        // order preserves the exact pass target and failure cutoff; at most
        // `jobs - 1` comparisons are speculative.
        let mut items = Vec::new();
        let mut executable = Vec::new();
        while cursor < candidates.len() && executable.len() < jobs {
            let candidate = candidates[cursor];
            cursor += 1;
            let expected_error = known_errors
                .get(&candidate.corpus)
                .and_then(|sources| sources.get(&candidate.record.file));
            if expected_error.is_some() {
                items.push(NormalRatchetItem::ExpectedError);
            } else {
                items.push(NormalRatchetItem::Candidate(candidate));
                executable.push(candidate);
            }
        }

        let results = pool.install(|| {
            executable
                .into_par_iter()
                .map(|candidate| compare_candidate(candidate, ui_language, trace_cases, true))
                .collect::<Vec<_>>()
        });
        let mut results = results.into_iter();
        for item in items {
            if state.passed >= pass_target || state.unexpected_count >= UNEXPECTED_FAILURE_LIMIT {
                break;
            }
            state.attempted += 1;
            match item {
                NormalRatchetItem::ExpectedError => state.expected_errors += 1,
                NormalRatchetItem::Candidate(candidate) => {
                    let comparison = results
                        .next()
                        .expect("every normal ratchet candidate has one comparison result");
                    match comparison {
                        Ok(()) => state.passed += 1,
                        Err(failure) => state.record_unexpected(
                            &candidate.corpus,
                            &candidate.record.file,
                            failure,
                        ),
                    }
                }
            }
        }
    }

    let error_report_path = write_error_report(
        root,
        format,
        None,
        false,
        &state.unexpected_records,
        &[],
        &[],
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
    order: usize,
    corpus: String,
    source: String,
    expected_layer: OfficeGoldenComparisonLayer,
    comparison: std::result::Result<(), OfficeGoldenFailure>,
}

#[derive(Clone, Copy)]
struct BatchAuditOptions {
    trace_cases: bool,
    require_pass_target: bool,
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

    // Preserve the normal ratchet's deterministic small-file prefix. Known
    // errors do not contribute to that prefix; they are independently audited
    // below and can therefore use bounded parallelism without changing which
    // unclassified cases establish the pass target.
    let mut state = AuditState::default();
    for candidate in unclassified {
        if state.passed >= pass_target || state.unexpected_count >= UNEXPECTED_FAILURE_LIMIT {
            break;
        }
        state.attempted += 1;
        match compare_candidate(
            candidate,
            &environment.environment.locale.ui_culture,
            options.trace_cases,
            true,
        ) {
            Ok(()) => state.passed += 1,
            Err(failure) => {
                state.record_unexpected(&candidate.corpus, &candidate.record.file, failure)
            }
        }
    }

    if state.unexpected_count < UNEXPECTED_FAILURE_LIMIT {
        let mut executable = Vec::with_capacity(known.len());
        for case in known {
            state.attempted += 1;
            if case.expected.skip_batch_audit {
                state.expected_errors += 1;
            } else {
                executable.push(case);
            }
        }

        let jobs = batch_audit_jobs()?;
        let pool = ThreadPoolBuilder::new()
            .num_threads(jobs)
            .thread_name(|index| format!("office-golden-{index}"))
            .build()
            .map_err(|error| format!("could not create Office golden audit pool: {error}"))?;
        let ui_language = &environment.environment.locale.ui_culture;
        let mut results = pool.install(|| {
            executable
                .into_iter()
                .par_bridge()
                .map(|case| KnownAuditResult {
                    order: case.order,
                    corpus: case.candidate.corpus.clone(),
                    source: case.candidate.record.file.clone(),
                    expected_layer: case.expected.layer,
                    comparison: compare_candidate(
                        case.candidate,
                        ui_language,
                        options.trace_cases,
                        false,
                    ),
                })
                .collect::<Vec<_>>()
        });
        results.sort_by_key(|result| result.order);

        for result in results {
            match result.comparison {
                Ok(()) => {
                    state.passed += 1;
                    state
                        .stale_errors
                        .push(format!("{}/{}", result.corpus, result.source));
                }
                Err(failure) if failure.layer == result.expected_layer => {
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
                Err(failure) if state.unexpected_count < UNEXPECTED_FAILURE_LIMIT => {
                    state.record_unexpected(&result.corpus, &result.source, failure);
                }
                Err(_) => {}
            }
        }
    }

    let error_report_path = write_error_report(
        root,
        format,
        None,
        true,
        &state.unexpected_records,
        &state.expected_records,
        &state.stale_errors,
    )?;
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
            &error_report_path,
        ));
    }

    eprintln!(
        "office-golden audit {} attempted={} passed={} expected_errors={} records={}",
        format.extension(),
        state.attempted,
        state.passed,
        state.expected_errors,
        error_report_path.display()
    );

    Ok(OfficeGoldenCorpusReport {
        format,
        attempted: state.attempted,
        passed: state.passed,
        expected_errors: state.expected_errors,
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

fn compare_candidate(
    candidate: &CorpusCandidate,
    ui_language: &str,
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

fn write_error_report(
    root: &Path,
    format: OfficeGoldenFormat,
    exact_case: Option<&str>,
    audit_errors: bool,
    unexpected: &[UnexpectedFailureRecord],
    expected: &[UnexpectedFailureRecord],
    stale_errors: &[String],
) -> std::result::Result<PathBuf, String> {
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
        update_diagnostic_index(&directory, format, unexpected, expected, stale_errors)?;
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
    if !stale_errors.is_empty() {
        let _ = write!(
            output,
            "\nstale error entries (now pass): {}",
            stale_errors.join(", ")
        );
    }
    for (layer, group) in unexpected {
        let _ = write!(
            output,
            "\nunexpected {} failures: {} (showing at most {})",
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
            "\nscan stopped after {UNEXPECTED_FAILURE_LIMIT} unexpected failures"
        );
    }
    let _ = write!(output, "\nerror records={}", error_report_path.display());
    output
}

#[cfg(test)]
mod tests {
    use super::{
        OfficeGoldenComparisonLayer, OfficeGoldenFormat, catch_conversion_panic,
        error_report_file_name, format_failure_summary,
    };
    use std::collections::BTreeMap;
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
}

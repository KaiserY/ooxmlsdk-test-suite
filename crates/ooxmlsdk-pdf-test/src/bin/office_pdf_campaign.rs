use std::env;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use ooxmlsdk_pdf_test::{
    audit_campaign, audit_one_with_artifacts, audit_worker_loop, convert_campaign,
    generate_campaign_assignments, read_plan, scan_campaign_font_references,
    scan_local_font_candidates, validate_assignments, workspace_root, write_plan,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("office-pdf-campaign: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or_else(usage)?;
    let arguments = arguments.collect::<Vec<_>>();
    let root = workspace_root();
    match command.as_str() {
        "generate-plan" => {
            let plan_path = plan_path(&root, &arguments)?;
            let assignments = generate_campaign_assignments(&root)?;
            let summary = validate_assignments(&root, &assignments)?;
            write_plan(&plan_path, &assignments)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary)
                    .map_err(|error| format!("could not serialize plan summary: {error}"))?
            );
            println!("plan={}", plan_path.display());
        }
        "validate-plan" => {
            let plan_path = plan_path(&root, &arguments)?;
            let assignments = read_plan(&plan_path)?;
            let summary = validate_assignments(&root, &assignments)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary)
                    .map_err(|error| format!("could not serialize plan summary: {error}"))?
            );
        }
        "convert" => {
            let plan_path = optional_path(&root, &arguments, "--plan")?
                .unwrap_or_else(|| root.join("corpus_pdf_conv/plan.jsonl"));
            let selection =
                option_value(&arguments, "--selection")?.unwrap_or_else(|| "pilot-300".to_string());
            let pilot_only = match selection.as_str() {
                "pilot-300" => true,
                "full" => false,
                value => {
                    return Err(format!(
                        "invalid --selection {value:?}; expected pilot-300 or full"
                    ));
                }
            };
            let pwsh =
                option_value(&arguments, "--pwsh")?.unwrap_or_else(|| "pwsh.exe".to_string());
            let jobs = optional_number(&arguments, "--jobs")?.unwrap_or(3);
            let timeout_seconds = optional_number(&arguments, "--timeout-seconds")?.unwrap_or(120);
            let max_attempts = optional_number(&arguments, "--max-attempts")?.unwrap_or(2);
            reject_unknown_options(
                &arguments,
                &[
                    "--plan",
                    "--selection",
                    "--pwsh",
                    "--jobs",
                    "--timeout-seconds",
                    "--max-attempts",
                ],
            )?;
            let summary = convert_campaign(
                &root,
                &plan_path,
                &pwsh,
                pilot_only,
                jobs,
                Duration::from_secs(
                    timeout_seconds
                        .try_into()
                        .map_err(|_| "--timeout-seconds is too large".to_string())?,
                ),
                max_attempts
                    .try_into()
                    .map_err(|_| "--max-attempts is too large".to_string())?,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary)
                    .map_err(|error| format!("could not serialize conversion summary: {error}"))?
            );
        }
        "font-scan" => {
            let plan_path = optional_path(&root, &arguments, "--plan")?
                .unwrap_or_else(|| root.join("corpus_pdf_conv/plan.jsonl"));
            let output_path = optional_path(&root, &arguments, "--output")?
                .unwrap_or_else(|| root.join("corpus_pdf_conv/font-references.json"));
            reject_unknown_options(&arguments, &["--plan", "--output"])?;
            let report = scan_campaign_font_references(&root, &plan_path, &output_path)?;
            println!("assignments_scanned={}", report.assignments_scanned);
            println!("xml_parts_scanned={}", report.xml_parts_scanned);
            println!(
                "packages_with_embedded_fonts={}",
                report.packages_with_embedded_fonts
            );
            println!("referenced_font_count={}", report.referenced_font_count);
            println!("parse_errors={}", report.parse_errors.len());
            println!("report={}", output_path.display());
        }
        "local-font-candidates" => {
            let reference_path = optional_path(&root, &arguments, "--references")?
                .unwrap_or_else(|| root.join("corpus_pdf_conv/font-references.json"));
            let output_path = optional_path(&root, &arguments, "--output")?
                .unwrap_or_else(|| root.join("corpus_pdf_conv/local-font-candidates.json"));
            reject_unknown_options(arguments.as_slice(), &["--references", "--output"])?;
            let report = scan_local_font_candidates(&reference_path, &output_path)?;
            println!("candidate_font_count={}", report.candidate_font_count);
            println!("report={}", output_path.display());
        }
        "audit" => {
            let plan_path = optional_path(&root, &arguments, "--plan")?
                .unwrap_or_else(|| root.join("corpus_pdf_conv/plan.jsonl"));
            let selection =
                option_value(&arguments, "--selection")?.unwrap_or_else(|| "pilot-300".to_string());
            let pilot_only = match selection.as_str() {
                "pilot-300" => true,
                "full" => false,
                value => {
                    return Err(format!(
                        "invalid --selection {value:?}; expected pilot-300 or full"
                    ));
                }
            };
            let jobs = optional_number(&arguments, "--jobs")?.unwrap_or_else(default_audit_jobs);
            let timeout_seconds = optional_number(&arguments, "--timeout-seconds")?.unwrap_or(180);
            reject_unknown_options(
                &arguments,
                &["--plan", "--selection", "--jobs", "--timeout-seconds"],
            )?;
            let executable = env::current_exe()
                .map_err(|error| format!("could not locate campaign executable: {error}"))?;
            let summary = audit_campaign(
                &executable,
                &root,
                &plan_path,
                pilot_only,
                jobs,
                Duration::from_secs(
                    timeout_seconds
                        .try_into()
                        .map_err(|_| "--timeout-seconds is too large".to_string())?,
                ),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary)
                    .map_err(|error| format!("could not serialize audit summary: {error}"))?
            );
        }
        "audit-one" => {
            let task = required_path(&arguments, "--task")?;
            let result = required_path(&arguments, "--result")?;
            let write_artifacts = optional_bool(&arguments, "--write-artifacts")?.unwrap_or(false);
            reject_unknown_options(&arguments, &["--task", "--result", "--write-artifacts"])?;
            audit_one_with_artifacts(&task, &result, write_artifacts)?;
        }
        "audit-worker" => {
            reject_unknown_options(&arguments, &[])?;
            audit_worker_loop()?;
        }
        "help" | "--help" | "-h" => println!("{}", usage()),
        _ => return Err(format!("unknown command {command:?}\n{}", usage())),
    }
    Ok(())
}

fn plan_path(root: &Path, arguments: &[String]) -> Result<PathBuf, String> {
    let path = optional_path(root, arguments, "--plan")?
        .unwrap_or_else(|| root.join("corpus_pdf_conv/plan.jsonl"));
    reject_unknown_options(arguments, &["--plan"])?;
    Ok(path)
}

fn required_path(arguments: &[String], option: &str) -> Result<PathBuf, String> {
    option_value(arguments, option)?
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required option {option}"))
}

fn optional_path(
    root: &Path,
    arguments: &[String],
    option: &str,
) -> Result<Option<PathBuf>, String> {
    Ok(option_value(arguments, option)?.map(|value| {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            path
        } else {
            root.join(path)
        }
    }))
}

fn optional_number(arguments: &[String], option: &str) -> Result<Option<usize>, String> {
    option_value(arguments, option)?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid value for {option}: {value:?}: {error}"))
                .and_then(|value| {
                    if value == 0 {
                        Err(format!("{option} must be positive"))
                    } else {
                        Ok(value)
                    }
                })
        })
        .transpose()
}

fn optional_bool(arguments: &[String], option: &str) -> Result<Option<bool>, String> {
    option_value(arguments, option)?
        .map(|value| match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!(
                "invalid value for {option}: {value:?}; expected true or false"
            )),
        })
        .transpose()
}

fn default_audit_jobs() -> usize {
    let parallelism = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(4);
    audit_jobs_for_parallelism(parallelism)
}

fn audit_jobs_for_parallelism(parallelism: usize) -> usize {
    parallelism.clamp(1, 12)
}

fn option_value(arguments: &[String], option: &str) -> Result<Option<String>, String> {
    let mut value = None;
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == option {
            if value.is_some() {
                return Err(format!("option {option} was provided more than once"));
            }
            let next = arguments
                .get(index + 1)
                .ok_or_else(|| format!("option {option} requires a value"))?;
            if next.starts_with('-') {
                return Err(format!("option {option} requires a value"));
            }
            value = Some(next.clone());
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(value)
}

fn reject_unknown_options(arguments: &[String], known: &[&str]) -> Result<(), String> {
    let mut index = 0;
    while index < arguments.len() {
        let option = &arguments[index];
        if !known.contains(&option.as_str()) {
            return Err(format!("unexpected argument {option:?}"));
        }
        if index + 1 >= arguments.len() {
            return Err(format!("option {option} requires a value"));
        }
        index += 2;
    }
    Ok(())
}

fn usage() -> String {
    "usage:\n  office_pdf_campaign generate-plan [--plan PATH]\n  office_pdf_campaign validate-plan [--plan PATH]\n  office_pdf_campaign font-scan [--plan PATH] [--output PATH]\n  office_pdf_campaign local-font-candidates [--references PATH] [--output PATH]\n  office_pdf_campaign convert [--plan PATH] [--selection pilot-300|full] [--pwsh PATH] [--jobs N] [--timeout-seconds N] [--max-attempts N]\n  office_pdf_campaign audit [--plan PATH] [--selection pilot-300|full] [--jobs N] [--timeout-seconds N]\n  office_pdf_campaign audit-one --task PATH --result PATH [--write-artifacts true|false]\n  office_pdf_campaign audit-worker"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::audit_jobs_for_parallelism;

    #[test]
    fn default_audit_jobs_use_available_parallelism_with_a_safe_cap() {
        assert_eq!(audit_jobs_for_parallelism(0), 1);
        assert_eq!(audit_jobs_for_parallelism(8), 8);
        assert_eq!(audit_jobs_for_parallelism(16), 12);
        assert_eq!(audit_jobs_for_parallelism(64), 12);
    }
}

#!/usr/bin/env python3
"""Build, audit and archive the PASS union or full configured golden campaign."""

import argparse
from collections import Counter
from datetime import datetime, timezone
import fcntl
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def read_rows(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def validate_results(rows, baseline, *, full=False):
    actual = {row["configuration_id"]: row for row in rows}
    expected = {row["configuration_id"]: row for row in baseline}
    if (not expected or len(expected) != len(baseline)
            or len(actual) != len(rows) or set(actual) != set(expected)):
        raise ValueError("audit did not return the exact baseline identity set")
    for identity, row in actual.items():
        for field in ("file", "corpus", "family"):
            if field in expected[identity] and row.get(field) != expected[identity][field]:
                raise ValueError(f"audit {field} identity differs from baseline")
        allowed = {"PASS", "FAIL", "REFERENCE_FAIL"} if full else {"PASS", "FAIL"}
        if row["verdict"] not in allowed:
            raise ValueError("audit contains reference/infrastructure errors")
    return actual


def compare_results(previous, current):
    return {
        "regressions": [current[key] for key in sorted(current)
                        if previous[key]["verdict"] == "PASS"
                        and current[key]["verdict"] != "PASS"],
        "new_passes": [current[key] for key in sorted(current)
                       if previous[key]["verdict"] != "PASS"
                       and current[key]["verdict"] == "PASS"],
    }


def file_version(path):
    if not path.exists():
        return None
    stat = path.stat()
    return stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns


def snapshot(source, destination):
    digest = sha256(source)
    shutil.copyfile(source, destination)
    if sha256(destination) != digest or sha256(source) != digest:
        raise RuntimeError(f"{source} changed while snapshotting")
    return digest


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n")


def run_audit(args, suite):
    binary = suite / "target/release/office_pdf_campaign"
    baseline = args.baseline.resolve()
    output = args.output_root.resolve()
    work = suite / "target/office-pdf-campaign"
    work.mkdir(parents=True, exist_ok=True)
    # Keep the existing lock name: selected and full wrapper runs share workers.
    with (work / "selected-wrapper.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        output.mkdir(parents=True, exist_ok=False)
        try:
            return audit_locked(args, suite, binary, baseline, output, work)
        except Exception as error:
            write_json(output / "error.json", dict(accepted=False, error=str(error)))
            raise


def audit_locked(args, suite, binary, baseline, output, work):
    started = time.monotonic()
    selection = output / "baseline.jsonl"
    baseline_hash = snapshot(baseline, selection)
    rows = read_rows(selection)
    ids = [row["configuration_id"] for row in rows]
    if not ids or any(not identity for identity in ids) or len(ids) != len(set(ids)):
        raise ValueError("baseline must contain unique, nonempty configuration IDs")
    previous = None
    previous_hash = None
    expected = rows
    if args.previous_results:
        previous_path = output / "previous-results.jsonl"
        previous_hash = snapshot(args.previous_results, previous_path)
        previous_rows = read_rows(previous_path)
        expected = previous_rows if args.full else rows
        previous = validate_results(previous_rows, expected, full=args.full)
        # Baseline is a subset of the full campaign, never an alternative ID set.
        union_previous = [previous[key] for key in ids if key in previous]
        validate_results(union_previous, rows, full=args.full)
    build_seconds = 0.0
    if args.build:
        print(f"Building release campaign; log: {output / 'build.log'}", flush=True)
        build_started = time.monotonic()
        with (output / "build.log").open("w") as log:
            subprocess.run(["cargo", "build", "-p", "ooxmlsdk-pdf-test", "--bin",
                            "office_pdf_campaign", "--release"], cwd=suite,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        build_seconds = time.monotonic() - build_started
    manifest = dict(
        started_utc=datetime.now(timezone.utc).isoformat(),
        baseline_sha256=baseline_hash, binary_sha256=sha256(binary),
        scope="full" if args.full else "union", count=len(expected), jobs=args.jobs,
        timeout_seconds=args.timeout_seconds,
        comparison="canonical batch audit; original configured golden tolerance",
        previous_results_sha256=previous_hash,
    )
    write_json(output / "manifest.json", manifest)
    command = [str(binary), "audit", "--selection", "full",
               "--jobs", str(args.jobs),
               "--timeout-seconds", str(args.timeout_seconds)]
    if not args.full:
        command.extend(["--configuration-ids", str(selection)])
    prefix = "full" if args.full else "selected"
    reports = {work / f"{prefix}-audit.jsonl": output / "results.jsonl",
               work / f"{prefix}-audit-summary.json": output / "campaign-summary.json"}
    versions = {path: file_version(path) for path in reports}
    print(f"Auditing {len(expected)} exact IDs with {args.jobs} workers; log: {output / 'campaign.log'}", flush=True)
    campaign_started = time.monotonic()
    with (output / "campaign.log").open("w") as log:
        # Per-case timeouts and crashed-worker recovery belong to the native engine.
        process = subprocess.run(command, cwd=suite, stdout=log, stderr=subprocess.STDOUT)
    campaign_seconds = time.monotonic() - campaign_started
    for source, destination in reports.items():
        if file_version(source) is None or file_version(source) == versions[source]:
            raise RuntimeError(f"audit produced no fresh {source.name}; see campaign.log")
        snapshot(source, destination)
    if process.returncode:
        raise RuntimeError(f"campaign exited {process.returncode}; see archived reports/log")
    for path, key in [(baseline, "baseline_sha256"), (binary, "binary_sha256")]:
        if sha256(path) != manifest[key]:
            raise RuntimeError(f"{path} changed during audit")
    records = read_rows(output / "results.jsonl")
    counts = dict(Counter(row["verdict"] for row in records))
    actual = validate_results(records, expected, full=args.full)
    native_summary = json.loads((output / "campaign-summary.json").read_text())
    if native_summary["assigned"] != len(records) or native_summary["verdicts"] != counts:
        raise ValueError("native summary differs from archived result accounting")
    union = validate_results([actual[key] for key in ids], rows, full=args.full)
    unresolved = [row for row in union.values() if row["verdict"] != "PASS"]
    comparison = compare_results(previous, actual) if previous is not None else None
    accepted = not ((comparison and comparison["regressions"])
                    or ((args.full or args.require_all_pass) and unresolved))
    if previous is not None:
        fields = ("verdict", "layer", "diagnostic_kind", "page_index", "line_index", "message")
        deltas = [dict(configuration_id=key, previous=previous[key], current=actual[key])
                  for key in sorted(actual)
                  if any(previous[key].get(field) != actual[key].get(field) for field in fields)]
        write_json(output / "diagnostic-deltas.json", deltas)
    results_hash = sha256(output / "results.jsonl")
    elapsed = time.monotonic() - started
    wrapper_seconds = elapsed - campaign_seconds - build_seconds
    summary = dict(manifest=manifest, counts=counts, accepted=accepted,
                   union_counts=dict(Counter(row["verdict"] for row in union.values())),
                   results_sha256=results_hash,
                   finished_utc=datetime.now(timezone.utc).isoformat(),
                   elapsed_seconds=elapsed, campaign_seconds=campaign_seconds,
                   wrapper_seconds=wrapper_seconds, build_seconds=build_seconds,
                   changes=comparison,
                   unresolved_union=unresolved)
    write_json(output / "summary.json", summary)
    print(json.dumps(dict(accepted=accepted, counts=counts, union=summary["union_counts"],
                          changes={key: len(value) for key, value in (comparison or {}).items()},
                          campaign_seconds=round(campaign_seconds, 3),
                          wrapper_seconds=round(wrapper_seconds, 3),
                          archive=str(output))))
    return 0 if accepted else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--timeout-seconds", type=int, default=180)
    parser.add_argument("--previous-results", type=Path,
                        help="previous accepted results with exactly the same ID/file set")
    parser.add_argument("--full", action="store_true",
                        help="audit full campaign; require previous full results and a passing union")
    parser.add_argument("--build", action="store_true", help="build release first (logged, serial)")
    parser.add_argument("--require-all-pass", action="store_true",
                        help="require all union cases to PASS (implied by --full)")
    args = parser.parse_args()
    if args.jobs <= 0 or args.timeout_seconds <= 0:
        parser.error("jobs and timeout must be positive")
    if args.full and not args.previous_results:
        parser.error("--full requires --previous-results for exact coverage and PASS retention")
    try:
        return run_audit(args, Path(__file__).resolve().parents[1])
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.CalledProcessError) as error:
        parser.exit(2, f"audit error: {error}\n")


if __name__ == "__main__":
    raise SystemExit(main())

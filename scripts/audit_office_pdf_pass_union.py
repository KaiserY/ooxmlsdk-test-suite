#!/usr/bin/env python3
"""Audit an exact baseline PASS union using the canonical persistent workers."""

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


def validate_results(rows, baseline):
    actual = {row["configuration_id"]: row for row in rows}
    expected = {row["configuration_id"]: row for row in baseline}
    if len(actual) != len(rows) or set(actual) != set(expected):
        raise ValueError("audit did not return the exact baseline identity set")
    for identity, row in actual.items():
        if row["file"] != expected[identity]["file"]:
            raise ValueError("audit file identity differs from baseline")
        if row["verdict"] not in {"PASS", "FAIL"}:
            raise ValueError("audit contains reference/infrastructure errors")
    return actual


def compare_results(previous, current):
    return {
        "regressions": [current[key] for key in sorted(current)
                        if previous[key]["verdict"] == "PASS"
                        and current[key]["verdict"] == "FAIL"],
        "new_passes": [current[key] for key in sorted(current)
                       if previous[key]["verdict"] == "FAIL"
                       and current[key]["verdict"] == "PASS"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--timeout-seconds", type=int, default=180)
    parser.add_argument("--previous-results", type=Path,
                        help="compare a completed results.jsonl with the same exact ID set")
    parser.add_argument("--require-all-pass", action="store_true",
                        help="exit 1 if any baseline identity still fails")
    args = parser.parse_args()
    if args.jobs <= 0 or args.timeout_seconds <= 0:
        parser.error("jobs and timeout must be positive")
    suite = Path(__file__).resolve().parents[1]
    binary = suite / "target/release/office_pdf_campaign"
    baseline = args.baseline.resolve()
    output = args.output_root.resolve()
    rows = read_rows(baseline)
    ids = [row["configuration_id"] for row in rows]
    if not ids or len(ids) != len(set(ids)):
        raise ValueError("baseline must contain unique, nonempty configuration IDs")
    previous = None
    previous_hash = None
    if args.previous_results:
        previous_hash = sha256(args.previous_results)
        previous = validate_results(read_rows(args.previous_results), rows)
        if sha256(args.previous_results) != previous_hash:
            raise RuntimeError("previous results changed while reading")
    work = suite / "target/office-pdf-campaign"
    work.mkdir(parents=True, exist_ok=True)
    # Native selected audit filenames are shared. Never overlap wrapper runs.
    with (work / "selected-wrapper.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        output.mkdir(parents=True, exist_ok=False)
        started = time.monotonic()
        manifest = dict(
            started_utc=datetime.now(timezone.utc).isoformat(),
            baseline_sha256=sha256(baseline), binary_sha256=sha256(binary),
            configuration_ids=ids, count=len(ids), jobs=args.jobs,
            families=dict(Counter(row["family"] for row in rows)),
            comparison="canonical batch audit; original configured golden tolerance",
            previous_results_sha256=previous_hash,
        )
        (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        # Snapshot the selection so the exact audited input is retained.
        selection = output / "baseline.jsonl"
        shutil.copyfile(baseline, selection)
        if sha256(selection) != manifest["baseline_sha256"]:
            raise RuntimeError("baseline changed while snapshotting")
        command = [str(binary), "audit", "--selection", "full",
                   "--configuration-ids", str(selection), "--jobs", str(args.jobs),
                   "--timeout-seconds", str(args.timeout_seconds)]
        print(f"Auditing {len(ids)} exact IDs with {args.jobs} workers; log: {output / 'campaign.log'}", flush=True)
        campaign_started = time.monotonic()
        with (output / "campaign.log").open("w") as log:
            # Per-case timeouts and crashed-worker recovery belong to the native engine.
            subprocess.run(command, cwd=suite, stdout=log, stderr=subprocess.STDOUT, check=True)
        campaign_seconds = time.monotonic() - campaign_started
        for path, key in [(baseline, "baseline_sha256"), (binary, "binary_sha256")]:
            if sha256(path) != manifest[key]:
                raise RuntimeError(f"{path} changed during audit")
        records = read_rows(work / "selected-audit.jsonl")
        counts = dict(Counter(row["verdict"] for row in records))
        shutil.copyfile(work / "selected-audit.jsonl", output / "results.jsonl")
        shutil.copyfile(work / "selected-audit-summary.json", output / "campaign-summary.json")
        actual = validate_results(records, rows)
        comparison = compare_results(previous, actual) if previous is not None else None
        elapsed = time.monotonic() - started
        summary = dict(manifest=manifest, counts=counts,
                       finished_utc=datetime.now(timezone.utc).isoformat(),
                       elapsed_seconds=elapsed, campaign_seconds=campaign_seconds,
                       wrapper_seconds=elapsed - campaign_seconds,
                       changes=comparison,
                       unresolved=[row for row in records if row["verdict"] != "PASS"])
        (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps({key: summary[key] for key in
                          ["counts", "elapsed_seconds", "campaign_seconds", "wrapper_seconds", "changes"]}, indent=2))
        if ((comparison is not None and comparison["regressions"])
                or (args.require_all_pass and counts.get("FAIL", 0))):
            return 1
        return 0


if __name__ == "__main__":
    raise SystemExit(main())

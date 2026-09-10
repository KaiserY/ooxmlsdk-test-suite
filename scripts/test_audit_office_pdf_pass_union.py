"""Identity and verdict guards; no rendering or Office process required."""

import unittest
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch

from audit_office_pdf_pass_union import compare_results, run_audit, validate_results


def row(identity, verdict="PASS"):
    return dict(configuration_id=identity, file=identity + ".docx", verdict=verdict)


class ResultTests(unittest.TestCase):
    def setUp(self):
        self.baseline = [row("a"), row("b")]

    def test_order_is_irrelevant(self):
        self.assertEqual(set(validate_results(self.baseline[::-1], self.baseline)), {"a", "b"})

    def test_rejects_duplicate_missing_extra_and_wrong_file(self):
        for records in ([row("a"), row("a")], [row("a")],
                        [row("a"), row("b"), row("c")],
                        [row("a"), dict(row("b"), file="wrong.docx")]):
            with self.subTest(records=records), self.assertRaises(ValueError):
                validate_results(records, self.baseline)

    def test_rejects_infrastructure_and_reference_errors(self):
        for verdict in ("ERROR", "REFERENCE_FAIL", "UNKNOWN"):
            with self.subTest(verdict=verdict), self.assertRaises(ValueError):
                validate_results([row("a"), row("b", verdict)], self.baseline)

    def test_new_pass_cannot_cancel_regression(self):
        previous = validate_results([row("a"), row("b", "FAIL")], self.baseline)
        current = validate_results([row("a", "FAIL"), row("b")], self.baseline)
        changes = compare_results(previous, current)
        self.assertEqual(changes["regressions"], [row("a", "FAIL")])
        self.assertEqual(changes["new_passes"], [row("b")])

    def test_full_reference_failure_does_not_hide_lost_pass(self):
        current = validate_results([row("a", "REFERENCE_FAIL"), row("b")],
                                   self.baseline, full=True)
        previous = validate_results(self.baseline, self.baseline)
        self.assertEqual(len(compare_results(previous, current)["regressions"]), 1)

    def test_rejects_changed_corpus_family_and_duplicate_baseline(self):
        for field in ("corpus", "family"):
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_results([dict(row("a"), **{field: "new"})],
                                 [dict(row("a"), **{field: "old"})])
        with self.assertRaises(ValueError):
            validate_results([row("a")], [row("a"), row("a")])


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.suite = Path(self.temporary.name)
        binary = self.suite / "target/release/office_pdf_campaign"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"test binary")
        self.work = self.suite / "target/office-pdf-campaign"
        self.work.mkdir()
        self.baseline = self.suite / "baseline.jsonl"
        self.previous = self.suite / "previous.jsonl"
        self.write_rows(self.baseline, [row("a")])
        self.rows = [row("a"), row("b", "FAIL"), row("c", "REFERENCE_FAIL")]
        self.write_rows(self.previous, self.rows)
        self.args = argparse.Namespace(
            baseline=self.baseline, previous_results=self.previous,
            output_root=self.suite / "archive", full=True, build=False,
            jobs=4, timeout_seconds=180, require_all_pass=False)

    @staticmethod
    def write_rows(path, rows):
        path.write_text("".join(json.dumps(row) + "\n" for row in rows))

    def campaign(self, command, **kwargs):
        self.assertEqual(command[1:4], ["audit", "--selection", "full"])
        self.assertNotIn("--configuration-ids", command)
        self.write_rows(self.work / "full-audit.jsonl", self.rows)
        counts = {}
        for record in self.rows:
            verdict = record["verdict"]
            counts[verdict] = counts.get(verdict, 0) + 1
        (self.work / "full-audit-summary.json").write_text(json.dumps(
            dict(assigned=len(self.rows), verdicts=counts)))
        return subprocess.CompletedProcess(command, 0)

    def test_full_archives_exact_counts_and_allows_existing_reference_failures(self):
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=self.campaign):
            self.assertEqual(run_audit(self.args, self.suite), 0)
        summary = json.loads((self.args.output_root / "summary.json").read_text())
        self.assertTrue(summary["accepted"])
        self.assertEqual(summary["union_counts"], {"PASS": 1})
        self.assertEqual(summary["changes"], dict(regressions=[], new_passes=[]))
        self.assertTrue((self.args.output_root / "diagnostic-deltas.json").exists())

    def test_full_rejects_lost_pass_outside_union_even_with_a_gain(self):
        self.write_rows(self.previous, [row("a"), row("b"), row("c", "FAIL")])
        self.rows = [row("a"), row("b", "FAIL"), row("c")]
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=self.campaign):
            self.assertEqual(run_audit(self.args, self.suite), 1)

    def test_full_rejects_unresolved_union(self):
        self.rows[0] = row("a", "FAIL")
        self.write_rows(self.previous, self.rows)
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=self.campaign):
            self.assertEqual(run_audit(self.args, self.suite), 1)

    def test_rejects_incomplete_results_and_archives_error(self):
        self.rows.pop()
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=self.campaign):
            with self.assertRaises(ValueError):
                run_audit(self.args, self.suite)
        self.assertTrue((self.args.output_root / "error.json").exists())

    def test_does_not_accept_stale_reports_from_successful_process(self):
        self.campaign(["binary", "audit", "--selection", "full"])
        with patch("audit_office_pdf_pass_union.subprocess.run",
                   return_value=subprocess.CompletedProcess([], 0)):
            with self.assertRaisesRegex(RuntimeError, "no fresh"):
                run_audit(self.args, self.suite)

    def test_failed_process_preserves_new_reports_but_is_not_accepted(self):
        def failed(command, **kwargs):
            self.campaign(command)
            return subprocess.CompletedProcess(command, 1)
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=failed):
            with self.assertRaisesRegex(RuntimeError, "campaign exited 1"):
                run_audit(self.args, self.suite)
        self.assertTrue((self.args.output_root / "results.jsonl").exists())
        self.assertFalse((self.args.output_root / "summary.json").exists())

    def test_existing_archive_is_never_overwritten(self):
        self.args.output_root.mkdir()
        with patch("audit_office_pdf_pass_union.subprocess.run") as process:
            with self.assertRaises(FileExistsError):
                run_audit(self.args, self.suite)
        process.assert_not_called()

    def test_build_is_serial_before_audit(self):
        self.args.build = True
        calls = []
        def run(command, **kwargs):
            calls.append(command[0])
            if command[0] == "cargo":
                return subprocess.CompletedProcess(command, 0)
            return self.campaign(command, **kwargs)
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=run):
            self.assertEqual(run_audit(self.args, self.suite), 0)
        self.assertEqual(calls[0], "cargo")
        self.assertEqual(len(calls), 2)

    def test_union_mode_still_selects_exact_baseline(self):
        self.args.full = False
        self.args.previous_results = None
        self.args.require_all_pass = True
        def selected(command, **kwargs):
            selection = Path(command[command.index("--configuration-ids") + 1])
            self.assertEqual(selection.read_text(), self.baseline.read_text())
            self.write_rows(self.work / "selected-audit.jsonl", [row("a")])
            (self.work / "selected-audit-summary.json").write_text(json.dumps(
                dict(assigned=1, verdicts={"PASS": 1})))
            return subprocess.CompletedProcess(command, 0)
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=selected):
            self.assertEqual(run_audit(self.args, self.suite), 0)

    def test_inconsistent_native_summary_is_rejected(self):
        def inconsistent(command, **kwargs):
            result = self.campaign(command)
            (self.work / "full-audit-summary.json").write_text(json.dumps(
                dict(assigned=3, verdicts={"PASS": 3})))
            return result
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=inconsistent):
            with self.assertRaisesRegex(ValueError, "accounting"):
                run_audit(self.args, self.suite)

    def test_binary_change_during_audit_is_rejected(self):
        def modified(command, **kwargs):
            result = self.campaign(command)
            Path(command[0]).write_bytes(b"changed binary")
            return result
        with patch("audit_office_pdf_pass_union.subprocess.run", side_effect=modified):
            with self.assertRaisesRegex(RuntimeError, "changed during audit"):
                run_audit(self.args, self.suite)


if __name__ == "__main__":
    unittest.main()

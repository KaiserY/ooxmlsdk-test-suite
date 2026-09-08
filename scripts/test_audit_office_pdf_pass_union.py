"""Identity and verdict guards; no rendering or Office process required."""

import unittest

from audit_office_pdf_pass_union import compare_results, validate_results


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


if __name__ == "__main__":
    unittest.main()

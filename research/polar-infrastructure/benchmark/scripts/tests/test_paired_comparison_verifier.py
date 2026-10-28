import copy
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_paired_comparison import build_expected_report, verify_documents  # noqa: E402

HERE = Path(__file__).resolve().parents[1]
FIXTURE = HERE.parent / "fixtures" / "paired-comparison-v0.3.json"


class PairedComparisonVerifierTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = json.loads(FIXTURE.read_text())
        cls.reference_report = build_expected_report(cls.fixture)

    def verify(self, doc=None, report=None):
        verify_documents(doc or copy.deepcopy(self.fixture),
                         report or copy.deepcopy(self.reference_report))

    def test_reference_report_is_accepted(self):
        self.verify()

    def test_rejects_missing_controller_trajectory_row(self):
        doc = copy.deepcopy(self.fixture)
        doc["cases"].pop()
        with self.assertRaisesRegex(ValueError, "incomplete controller/trajectory matrix"):
            build_expected_report(doc)

    def test_rejects_trajectory_severity_tampering(self):
        doc = copy.deepcopy(self.fixture)
        doc["cases"][-1]["severity"]["storm_duration_h"] = 99.0
        with self.assertRaisesRegex(ValueError, "differ in trajectory metadata"):
            build_expected_report(doc)

    def test_rejects_corrupted_coverage_count(self):
        report = copy.deepcopy(self.reference_report)
        report["coverage"][0]["pass_count"] += 1
        with self.assertRaisesRegex(ValueError, "numeric mismatch|mismatch"):
            self.verify(report=report)

    def test_rejects_missing_run_fraudulently_marked_pass(self):
        doc = copy.deepcopy(self.fixture)
        row = next(c for c in doc["cases"] if c["case_id"] == "candidate-t5")
        row["gate_results"]["critical_energy"] = "pass"
        with self.assertRaisesRegex(ValueError, "pass needs nonnegative margin"):
            build_expected_report(doc)

    def test_rejects_pairwise_outcome_misclassification(self):
        report = copy.deepcopy(self.reference_report)
        report["paired_gate_comparisons"][0]["reference_breach_candidate_pass_count"] = 99
        with self.assertRaisesRegex(ValueError, "mismatch"):
            self.verify(report=report)

    def test_rejects_evidence_escalation(self):
        report = copy.deepcopy(self.reference_report)
        report["evidence_class"] = "field_validated"
        with self.assertRaisesRegex(ValueError, "mismatch|key mismatch"):
            self.verify(report=report)

    def test_rejects_unknown_report_field(self):
        report = copy.deepcopy(self.reference_report)
        report["physical_safety_certified"] = True
        with self.assertRaisesRegex(ValueError, "report keys"):
            self.verify(report=report)

    def test_rejects_duplicate_case_id(self):
        doc = copy.deepcopy(self.fixture)
        doc["cases"][-1]["case_id"] = doc["cases"][0]["case_id"]
        with self.assertRaisesRegex(ValueError, "duplicate case ID"):
            build_expected_report(doc)

    def test_rejects_missing_gate_margin_semantics(self):
        doc = copy.deepcopy(self.fixture)
        doc["gate_margin_contracts"].pop("communications")
        with self.assertRaisesRegex(ValueError, "margin contracts"):
            build_expected_report(doc)

    def test_rejects_status_margin_contradiction(self):
        doc = copy.deepcopy(self.fixture)
        doc["cases"][0]["signed_gate_margins"]["critical_energy"] = -1.0
        with self.assertRaisesRegex(ValueError, "pass needs nonnegative margin"):
            build_expected_report(doc)


if __name__ == "__main__":
    unittest.main()

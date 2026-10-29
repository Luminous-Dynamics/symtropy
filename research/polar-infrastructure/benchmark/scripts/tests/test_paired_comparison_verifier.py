import copy
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_paired_comparison import build_expected_report, verify_documents  # noqa: E402

HERE = Path(__file__).resolve().parents[1]
FIXTURE = HERE.parent / "fixtures" / "paired-comparison-v0.5.json"


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

    def test_rejects_trajectory_payload_tampering(self):
        doc = copy.deepcopy(self.fixture)
        doc["trajectory_artifacts"][0]["trajectory_payload"] += "-tampered"
        with self.assertRaisesRegex(ValueError, "trajectory payload SHA-256 mismatch"):
            build_expected_report(doc)

    def test_rejects_controller_payload_tampering(self):
        doc = copy.deepcopy(self.fixture)
        doc["controller_artifacts"][0]["controller_payload"] += "-tampered"
        with self.assertRaisesRegex(ValueError, "controller payload SHA-256 mismatch"):
            build_expected_report(doc)

    def test_rejects_case_referencing_wrong_trace_digest(self):
        doc = copy.deepcopy(self.fixture)
        doc["cases"][1]["trajectory_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "digest/ID binding"):
            build_expected_report(doc)

    def test_rejects_missing_controller_artifact(self):
        doc = copy.deepcopy(self.fixture)
        doc["controller_artifacts"].pop()
        with self.assertRaisesRegex(ValueError, "controller artifacts do not match"):
            build_expected_report(doc)

    def test_rejects_initial_state_id_with_multiple_digests(self):
        doc = copy.deepcopy(self.fixture)
        doc["trajectory_artifacts"][1]["initial_state_id"] = doc["trajectory_artifacts"][0]["initial_state_id"]
        with self.assertRaisesRegex(ValueError, "one initial_state_id references multiple payload digests"):
            build_expected_report(doc)

    def test_rejects_information_budget_digest_tampering(self):
        doc = copy.deepcopy(self.fixture)
        doc["information_budget_payload"] += " "
        with self.assertRaisesRegex(ValueError, "information-budget payload SHA-256 mismatch"):
            build_expected_report(doc)

    def test_rejects_case_with_unequal_information_budget(self):
        doc = copy.deepcopy(self.fixture)
        doc["cases"][0]["information_budget_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "case uses a different information budget"):
            build_expected_report(doc)

    def test_rejects_observation_trace_payload_tampering(self):
        doc = copy.deepcopy(self.fixture)
        doc["trajectory_artifacts"][0]["observation_trace_payload"] += " "
        with self.assertRaisesRegex(ValueError, "observation-trace payload SHA-256 mismatch"):
            build_expected_report(doc)

    def test_rejects_measurement_from_future(self):
        import hashlib
        doc = copy.deepcopy(self.fixture)
        artifact = doc["trajectory_artifacts"][0]
        trace = json.loads(artifact["observation_trace_payload"])
        decision = trace["decisions"][0]
        next(item for item in decision["information"] if item["kind"] == "measurement")["observed_at_s"] = decision["decision_time_s"] + 1
        artifact["observation_trace_payload"] = json.dumps(trace, separators=(",", ":"))
        artifact["observation_trace_sha256"] = hashlib.sha256(artifact["observation_trace_payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "look-ahead leak: measurement observed after decision time"):
            build_expected_report(doc)

    def test_rejects_forecast_issued_in_future(self):
        import hashlib
        doc = copy.deepcopy(self.fixture)
        artifact = doc["trajectory_artifacts"][0]
        trace = json.loads(artifact["observation_trace_payload"])
        decision = trace["decisions"][0]
        next(item for item in decision["information"] if item["kind"] == "forecast")["issued_at_s"] = decision["decision_time_s"] + 1
        artifact["observation_trace_payload"] = json.dumps(trace, separators=(",", ":"))
        artifact["observation_trace_sha256"] = hashlib.sha256(artifact["observation_trace_payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "look-ahead leak: forecast issued after decision time"):
            build_expected_report(doc)

    def test_rejects_forecast_beyond_declared_horizon(self):
        import hashlib
        doc = copy.deepcopy(self.fixture)
        artifact = doc["trajectory_artifacts"][0]
        trace = json.loads(artifact["observation_trace_payload"])
        decision = trace["decisions"][0]
        next(item for item in decision["information"] if item["kind"] == "forecast")["valid_time_s"] = decision["decision_time_s"] + 301
        artifact["observation_trace_payload"] = json.dumps(trace, separators=(",", ":"))
        artifact["observation_trace_sha256"] = hashlib.sha256(artifact["observation_trace_payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "forecast valid time exceeds the declared forecast horizon"):
            build_expected_report(doc)


if __name__ == "__main__":
    unittest.main()

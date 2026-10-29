import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from verify_execution_receipt import verify_receipt  # noqa: E402

RECEIPT = HERE.parent / "fixtures" / "controller-execution-receipt-template-2026.json"
PAIRED = HERE.parent / "fixtures" / "paired-comparison-v0.4.json"


class ExecutionReceiptVerifierTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.receipt = json.loads(RECEIPT.read_text())
        cls.paired = json.loads(PAIRED.read_text())

    def verify(self, receipt=None, paired=None):
        verify_receipt(copy.deepcopy(receipt or self.receipt), copy.deepcopy(paired or self.paired))

    def test_design_only_receipt_is_valid_but_not_executed(self):
        self.verify()

    def test_rejects_payload_byte_tampering(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["artifact_inventory"][0]["payload"] += "-tampered"
        with self.assertRaisesRegex(ValueError, "payload SHA-256 mismatch"):
            self.verify(receipt)

    def test_rejects_mismatched_paired_input_digest(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["paired_input_bindings"]["trajectory_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "disagree with the paired case row"):
            self.verify(receipt)

    def test_rejects_controller_artifact_inventory_drift(self):
        receipt = copy.deepcopy(self.receipt)
        artifact = next(a for a in receipt["artifact_inventory"] if a["role"] == "controller_artifact")
        artifact["sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "payload SHA-256 mismatch|controller artifact inventory digest"):
            self.verify(receipt)

    def test_rejects_resource_policy_not_matching_numeric_budget(self):
        receipt = copy.deepcopy(self.receipt)
        artifact = next(a for a in receipt["artifact_inventory"] if a["role"] == "resource_policy")
        artifact["payload"] = '{"wall_time_ms":999}'
        artifact["sha256"] = hashlib.sha256(artifact["payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "resource policy artifact must match canonical"):
            self.verify(receipt)

    def test_rejects_wrong_controller_for_case(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["controller_id"] = "rule_baseline"
        with self.assertRaisesRegex(ValueError, "controller does not match"):
            self.verify(receipt)

    def test_rejects_synthetic_receipt_promoted_to_scoring(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["scoring_eligible"] = True
        with self.assertRaisesRegex(ValueError, "cannot authorize scoring until signed execution attestation"):
            self.verify(receipt)

    def test_rejects_synthetic_receipt_claiming_execution(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["execution_performed"] = True
        with self.assertRaisesRegex(ValueError, "must remain synthetic and ineligible"):
            self.verify(receipt)

    def test_rejects_synthetic_receipt_relabelled_as_completed(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["status"] = "completed"
        receipt["execution_state"] = "completed"
        receipt["execution_performed"] = True
        with self.assertRaisesRegex(ValueError, "synthetic fixture cannot claim that execution occurred"):
            self.verify(receipt)

    def test_rejects_unknown_receipt_field(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["physical_safety_certified"] = True
        with self.assertRaisesRegex(ValueError, "strict v0.1 contract"):
            self.verify(receipt)

    def test_rejects_self_asserted_attestation_promotion(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["execution_attestation"]["scheme"] = "sigstore_bundle"
        receipt["execution_attestation"]["status"] = "verified"
        with self.assertRaisesRegex(ValueError, "signed-attestation gate to remain explicitly unimplemented"):
            self.verify(receipt)

    def test_rejects_missing_artifact_role(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["artifact_inventory"].pop()
        with self.assertRaisesRegex(ValueError, "exactly the required input roles"):
            self.verify(receipt)

    def test_rejects_weak_resource_fairness_policy(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["claim_policy"]["resource_limits_must_be_shared_across_comparison_arms"] = False
        with self.assertRaisesRegex(ValueError, "claim policy"):
            self.verify(receipt)

    def test_rejects_missing_execution_measurements_on_unexecuted_receipt(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["observed_resources"]["wall_time_ms"] = 10
        with self.assertRaisesRegex(ValueError, "cannot claim measured resources"):
            self.verify(receipt)

    def test_rejects_paired_case_missing_from_input(self):
        paired = copy.deepcopy(self.paired)
        paired["cases"] = [c for c in paired["cases"] if c["case_id"] != "candidate-t1"]
        with self.assertRaisesRegex(ValueError, "absent from paired input"):
            self.verify(paired=paired)


if __name__ == "__main__":
    unittest.main()

import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from verify_execution_receipt import verify_receipt  # noqa: E402

FIXTURES = HERE.parent / "fixtures"
RECEIPT_V02 = FIXTURES / "controller-execution-receipt-v0.2-template-2026.json"
PAIRED_V05 = FIXTURES / "paired-comparison-v0.5.json"
RECEIPT_V01 = FIXTURES / "controller-execution-receipt-template-2026.json"
PAIRED_V04 = FIXTURES / "paired-comparison-v0.4.json"


class ExecutionReceiptV02VerifierTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.receipt_v02 = json.loads(RECEIPT_V02.read_text())
        cls.paired_v05 = json.loads(PAIRED_V05.read_text())
        cls.receipt_v01 = json.loads(RECEIPT_V01.read_text())
        cls.paired_v04 = json.loads(PAIRED_V04.read_text())

    def verify(self, receipt=None, paired=None):
        verify_receipt(copy.deepcopy(receipt if receipt is not None else self.receipt_v02),
                       copy.deepcopy(paired if paired is not None else self.paired_v05))

    def test_v02_design_only_receipt_binds_v05_inputs_without_claiming_execution(self):
        self.verify()
        self.assertEqual(self.receipt_v02["schema_version"], "0.2.0")
        self.assertFalse(self.receipt_v02["execution_performed"])
        self.assertFalse(self.receipt_v02["scoring_eligible"])

    def test_v02_receipt_rejects_wrong_observation_trace_digest(self):
        receipt = copy.deepcopy(self.receipt_v02)
        receipt["paired_input_bindings"]["observation_trace_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "observation-trace binding disagrees"):
            self.verify(receipt=receipt)

    def test_v02_receipt_rejects_wrong_information_budget_digest(self):
        receipt = copy.deepcopy(self.receipt_v02)
        receipt["paired_input_bindings"]["information_budget_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "information-budget binding disagrees"):
            self.verify(receipt=receipt)

    def test_v02_receipt_rejects_observation_policy_artifact_drift(self):
        receipt = copy.deepcopy(self.receipt_v02)
        artifact = next(a for a in receipt["artifact_inventory"] if a["role"] == "observation_policy")
        artifact["payload"] = "different-budget-payload"
        artifact["sha256"] = hashlib.sha256(artifact["payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "observation policy artifact disagrees"):
            self.verify(receipt=receipt)

    def test_v02_receipt_rejects_tampered_trace_payload_in_paired_input(self):
        paired = copy.deepcopy(self.paired_v05)
        paired["trajectory_artifacts"][0]["observation_trace_payload"] += " "
        with self.assertRaisesRegex(ValueError, "observation-trace payload SHA-256 mismatch"):
            self.verify(paired=paired)

    def test_v02_receipt_cannot_be_verified_against_v04_input(self):
        with self.assertRaisesRegex(ValueError, "receipt version is incompatible"):
            self.verify(paired=self.paired_v04)

    def test_v01_receipt_stays_compatible_with_v04_input(self):
        verify_receipt(copy.deepcopy(self.receipt_v01), copy.deepcopy(self.paired_v04))

    def test_v01_receipt_cannot_silently_skip_v05_bindings(self):
        with self.assertRaisesRegex(ValueError, "receipt version is incompatible"):
            verify_receipt(copy.deepcopy(self.receipt_v01), copy.deepcopy(self.paired_v05))


if __name__ == "__main__":
    unittest.main()

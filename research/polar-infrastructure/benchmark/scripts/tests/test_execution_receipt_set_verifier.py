import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from verify_execution_receipt_set import verify_receipt_set  # noqa: E402

RECEIPT = HERE.parent / "fixtures" / "controller-execution-receipt-template-2026.json"
PAIRED = HERE.parent / "fixtures" / "paired-comparison-v0.4.json"


def digest_limits(receipt):
    canonical = json.dumps(receipt["resource_limits"], sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    receipt["resource_limits_sha256"] = hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def build_receipt_set(template, paired):
    receipts = []
    for case in paired["cases"]:
        receipt = copy.deepcopy(template)
        trajectory = next(a for a in paired["trajectory_artifacts"] if a["trajectory_id"] == case["trajectory_id"])
        controller = next(a for a in paired["controller_artifacts"] if a["controller_id"] == case["controller_id"])
        receipt["receipt_id"] = "receipt-" + case["case_id"]
        receipt["case_id"] = case["case_id"]
        receipt["controller_id"] = case["controller_id"]
        receipt["paired_input_bindings"] = {
            "trajectory_id": trajectory["trajectory_id"],
            "trajectory_sha256": trajectory["trajectory_sha256"],
            "initial_state_id": trajectory["initial_state_id"],
            "initial_state_sha256": trajectory["initial_state_sha256"],
            "controller_artifact_sha256": controller["controller_sha256"],
        }
        receipts.append(receipt)
    return receipts


class ExecutionReceiptSetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.template = json.loads(RECEIPT.read_text())
        cls.paired = json.loads(PAIRED.read_text())
        cls.receipts = build_receipt_set(cls.template, cls.paired)

    def verify(self, receipts=None):
        verify_receipt_set(copy.deepcopy(self.paired), copy.deepcopy(receipts or self.receipts))

    def test_accepts_complete_unexecuted_contract_matrix_without_performance_claim(self):
        self.verify()

    def test_rejects_missing_receipt(self):
        with self.assertRaisesRegex(ValueError, "one receipt per declared"):
            self.verify(self.receipts[:-1])

    def test_rejects_duplicate_receipt(self):
        receipts = copy.deepcopy(self.receipts)
        receipts[-1] = copy.deepcopy(receipts[0])
        with self.assertRaisesRegex(ValueError, "duplicate receipt for case"):
            self.verify(receipts)

    def test_rejects_different_numeric_resource_limits(self):
        receipts = copy.deepcopy(self.receipts)
        receipts[0]["resource_limits"]["wall_time_ms"] += 1
        digest_limits(receipts[0])
        with self.assertRaisesRegex(ValueError, "unequal numeric resource limits"):
            self.verify(receipts)

    def test_rejects_different_resource_policy_digest(self):
        receipts = copy.deepcopy(self.receipts)
        entry = next(a for a in receipts[0]["artifact_inventory"] if a["role"] == "resource_policy")
        entry["payload"] += ":different-budget"
        entry["sha256"] = hashlib.sha256(entry["payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "same resource_policy artifact digest"):
            self.verify(receipts)

    def test_rejects_different_observation_policy_digest(self):
        receipts = copy.deepcopy(self.receipts)
        entry = next(a for a in receipts[0]["artifact_inventory"] if a["role"] == "observation_policy")
        entry["payload"] += ":more-sensors"
        entry["sha256"] = hashlib.sha256(entry["payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "same observation_policy artifact digest"):
            self.verify(receipts)

    def test_rejects_different_plant_model_digest(self):
        receipts = copy.deepcopy(self.receipts)
        entry = next(a for a in receipts[0]["artifact_inventory"] if a["role"] == "plant_model")
        entry["payload"] += ":different-plant"
        entry["sha256"] = hashlib.sha256(entry["payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "same plant_model artifact digest"):
            self.verify(receipts)

    def test_rejects_receipt_with_wrong_trajectory_input(self):
        receipts = copy.deepcopy(self.receipts)
        receipts[0]["paired_input_bindings"]["trajectory_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "disagree with the paired case row"):
            self.verify(receipts)

    def test_rejects_controller_artifact_drift_between_trajectories(self):
        receipts = copy.deepcopy(self.receipts)
        receipts[0]["paired_input_bindings"]["controller_artifact_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "disagree with the paired case row"):
            self.verify(receipts)


if __name__ == "__main__":
    unittest.main()

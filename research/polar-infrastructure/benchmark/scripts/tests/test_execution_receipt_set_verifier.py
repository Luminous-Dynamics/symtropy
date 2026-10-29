import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from verify_execution_receipt_set import main as verify_receipt_set_main, verify_receipt_set  # noqa: E402

RECEIPT = HERE.parent / "fixtures" / "controller-execution-receipt-template-2026.json"
PAIRED = HERE.parent / "fixtures" / "paired-comparison-v0.4.json"
RECEIPT_V02 = HERE.parent / "fixtures" / "controller-execution-receipt-v0.2-template-2026.json"
PAIRED_V05 = HERE.parent / "fixtures" / "paired-comparison-v0.5.json"


def digest_limits(receipt):
    canonical = json.dumps(receipt["resource_limits"], sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    receipt["resource_limits_sha256"] = digest
    policy = next(a for a in receipt["artifact_inventory"] if a["role"] == "resource_policy")
    policy["payload"] = canonical
    policy["sha256"] = digest


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
        controller_entry = next(a for a in receipt["artifact_inventory"] if a["role"] == "controller_artifact")
        controller_entry["artifact_id"] = "controller-artifact-" + case["controller_id"]
        controller_entry["payload"] = controller["controller_payload"]
        controller_entry["sha256"] = controller["controller_sha256"]
        receipts.append(receipt)
    return receipts


def build_v02_receipt_set(template, paired):
    receipts = []
    for case in paired["cases"]:
        receipt = copy.deepcopy(template)
        trajectory = next(a for a in paired["trajectory_artifacts"] if a["trajectory_id"] == case["trajectory_id"])
        controller = next(a for a in paired["controller_artifacts"] if a["controller_id"] == case["controller_id"])
        receipt["receipt_id"] = "receipt-v02-" + case["case_id"]
        receipt["case_id"] = case["case_id"]
        receipt["controller_id"] = case["controller_id"]
        receipt["paired_input_bindings"] = {
            "trajectory_id": trajectory["trajectory_id"],
            "trajectory_sha256": trajectory["trajectory_sha256"],
            "initial_state_id": trajectory["initial_state_id"],
            "initial_state_sha256": trajectory["initial_state_sha256"],
            "controller_artifact_sha256": controller["controller_sha256"],
            "information_budget_sha256": paired["information_budget_sha256"],
            "observation_trace_sha256": trajectory["observation_trace_sha256"],
        }
        controller_entry = next(a for a in receipt["artifact_inventory"] if a["role"] == "controller_artifact")
        controller_entry["artifact_id"] = "controller-artifact-" + case["controller_id"]
        controller_entry["payload"] = controller["controller_payload"]
        controller_entry["sha256"] = controller["controller_sha256"]
        receipts.append(receipt)
    return receipts


class ExecutionReceiptSetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.template = json.loads(RECEIPT.read_text())
        cls.paired = json.loads(PAIRED.read_text())
        cls.receipts = build_receipt_set(cls.template, cls.paired)
        cls.template_v02 = json.loads(RECEIPT_V02.read_text())
        cls.paired_v05 = json.loads(PAIRED_V05.read_text())
        cls.receipts_v02 = build_v02_receipt_set(cls.template_v02, cls.paired_v05)

    def verify(self, receipts=None):
        verify_receipt_set(copy.deepcopy(self.paired), copy.deepcopy(receipts or self.receipts))

    def test_rejects_duplicate_json_keys_in_receipt_set_cli(self):
        import tempfile
        from contextlib import redirect_stderr
        import io
        with tempfile.TemporaryDirectory() as directory:
            paired_path = Path(directory) / "paired.json"
            receipt_path = Path(directory) / "receipt.json"
            paired_path.write_text('{"schema_version":"invalid"}')
            receipt_path.write_text('{"schema_version":"0.1.0","schema_version":"0.1.0"}')
            with redirect_stderr(io.StringIO()):
                self.assertEqual(verify_receipt_set_main(["verify_execution_receipt_set.py", str(paired_path), str(receipt_path)]), 1)

    def test_accepts_complete_unexecuted_contract_matrix_without_performance_claim(self):
        self.verify()

    def test_rejects_missing_receipt(self):
        with self.assertRaisesRegex(ValueError, "one receipt per declared"):
            self.verify(self.receipts[:-1])

    def test_rejects_tampered_paired_input_before_receipt_set_validation(self):
        paired = copy.deepcopy(self.paired)
        paired["trajectory_artifacts"][0]["trajectory_payload"] += "-tampered"
        with self.assertRaisesRegex(ValueError, "trajectory payload SHA-256 mismatch"):
            verify_receipt_set(paired, copy.deepcopy(self.receipts))

    def test_rejects_duplicate_receipt(self):
        receipts = copy.deepcopy(self.receipts)
        receipts[-1] = copy.deepcopy(receipts[0])
        with self.assertRaisesRegex(ValueError, "duplicate receipt for case"):
            self.verify(receipts)

    def test_rejects_different_numeric_resource_limits(self):
        receipts = copy.deepcopy(self.receipts)
        receipts[0]["resource_limits"]["wall_time_ms"] += 1
        digest_limits(receipts[0])
        with self.assertRaisesRegex(ValueError, "resource_policy artifact digest|unequal numeric resource limits"):
            self.verify(receipts)

    def test_rejects_different_resource_policy_digest(self):
        receipts = copy.deepcopy(self.receipts)
        entry = next(a for a in receipts[0]["artifact_inventory"] if a["role"] == "resource_policy")
        entry["payload"] += ":different-budget"
        entry["sha256"] = hashlib.sha256(entry["payload"].encode("utf-8")).hexdigest()
        with self.assertRaisesRegex(ValueError, "resource policy artifact must match canonical numeric resource limits and digest"):
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
        with self.assertRaisesRegex(ValueError, "controller artifact inventory digest does not match paired input binding"):
            self.verify(receipts)

    def test_accepts_complete_v02_receipt_set_for_v05_paired_input(self):
        verify_receipt_set(copy.deepcopy(self.paired_v05), copy.deepcopy(self.receipts_v02))

    def test_rejects_v02_receipt_set_with_mismatched_budget_digest(self):
        receipts = copy.deepcopy(self.receipts_v02)
        receipts[0]["paired_input_bindings"]["information_budget_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "information-budget binding disagrees"):
            verify_receipt_set(copy.deepcopy(self.paired_v05), receipts)

    def test_rejects_v02_receipt_set_with_different_observation_trace(self):
        receipts = copy.deepcopy(self.receipts_v02)
        receipts[1]["paired_input_bindings"]["observation_trace_sha256"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "observation-trace binding disagrees"):
            verify_receipt_set(copy.deepcopy(self.paired_v05), receipts)


if __name__ == "__main__":
    unittest.main()

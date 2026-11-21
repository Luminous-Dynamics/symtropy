import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CANONICAL_TOOLS = ROOT / "tools" / "av-modality-necessity-evaluator"
sys.path.insert(0, str(CANONICAL_TOOLS))

import long_horizon_evaluate as evaluator  # noqa: E402

BASE = ROOT / "research" / "av-long-horizon-evidence-v0" / "fixtures"


class LegacyEvaluatorCompatibilityTests(unittest.TestCase):
    def setUp(self):
        self.workload = evaluator.read_json(BASE / "agent" / "workload-v1.json")
        self.truth = evaluator.read_json(BASE / "evaluator-only" / "truth-v1.json")
        self.submission = evaluator.read_json(BASE / "agent" / "reference-submission-v1.json")

    def test_legacy_documented_evaluator_path_uses_current_11_case_contract(self):
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        self.assertEqual(score["totals"]["episode_count"], 11)
        self.assertEqual(score["totals"]["answer_correct_count"], 10)
        self.assertEqual(score["totals"]["decision_correct_count"], 11)
        self.assertEqual(score["totals"]["hard_violation_count"], 0)

    def test_pre_correction_checkpoint_uses_old_state(self):
        result = next(
            row for row in evaluator.evaluate(self.workload, self.truth, self.submission)["per_episode"]
            if row["episode_id"] == "lh-010-correction-before-arrival"
        )
        self.assertTrue(result["answer_correct"])
        self.assertTrue(result["provenance_complete"])

    def test_pre_revocation_checkpoint_still_accepts_evidence(self):
        result = next(
            row for row in evaluator.evaluate(self.workload, self.truth, self.submission)["per_episode"]
            if row["episode_id"] == "lh-011-revocation-before-effect"
        )
        self.assertTrue(result["answer_correct"])
        self.assertTrue(result["provenance_complete"])


if __name__ == "__main__":
    unittest.main()

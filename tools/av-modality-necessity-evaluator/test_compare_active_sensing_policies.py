import copy
import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("compare_active_sensing_policies.py")
spec = importlib.util.spec_from_file_location("active_sensing_policy_comparator", MODULE_PATH)
comparator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(comparator)

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "av-active-sensing-decision-value-v0" / "fixtures"


class ActiveSensingPolicyComparisonTests(unittest.TestCase):
    def setUp(self):
        self.workload = comparator.read_json(BASE / "agent" / "workload-v1.json")
        self.truth = comparator.read_json(BASE / "evaluator-only" / "action-truth-v1.json")
        self.decision_value = comparator.read_json(BASE / "agent" / "reference-submission-v1.json")
        self.max_eig = comparator.read_json(BASE / "agent" / "max-eig-reference-submission-v1.json")

    def test_decision_value_beats_greedy_eig_on_authored_counterexamples(self):
        report = comparator.compare(self.workload, self.truth, self.decision_value, self.max_eig)
        dv, eig = report["per_policy"]["decision_value"], report["per_policy"]["max_eig"]
        self.assertEqual(dv["answer_accuracy"], 1.0)
        self.assertEqual(eig["answer_accuracy"], 0.75)
        self.assertEqual(dv["safe_decision_accuracy"], 1.0)
        self.assertEqual(eig["safe_decision_accuracy"], 0.25)
        self.assertEqual(dv["acquisition_count"], 3)
        self.assertEqual(eig["acquisition_count"], 8)
        self.assertEqual(eig["wasted_acquisition_count"], 6)
        self.assertEqual(eig["late_acquisition_count"], 1)
        self.assertEqual(dv["decision_regret_total"], 0)
        self.assertEqual(eig["decision_regret_total"], 11.5)
        self.assertEqual(report["deltas_decision_value_minus_max_eig"]["decision_regret_reduction"], 11.5)
        self.assertEqual(report["paired"]["safe_decision_correctness"]["candidate_wins"], 6)

    def test_comparator_rejects_wrong_policy_family(self):
        broken = copy.deepcopy(self.max_eig)
        broken["policy_family"] = "myopic_evsi"
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare(self.workload, self.truth, self.decision_value, broken)


if __name__ == "__main__":
    unittest.main()
import copy
import importlib.util
import json
import unittest
from pathlib import Path

MODULE = Path(__file__).with_name("resource_frontier.py")
spec = importlib.util.spec_from_file_location("resource_frontier", MODULE)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "research" / "av-pareto-resource-frontier-v0" / "fixtures" / "synthetic-comparison-v1.json"


def load():
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


class ResourceFrontierTests(unittest.TestCase):
    def setUp(self):
        self.bundle = load()

    def test_frontier_preserves_real_tradeoff_and_excludes_dominated_run(self):
        score = evaluator.evaluate(self.bundle)
        self.assertEqual(score["pareto_frontier_run_ids"], ["run-adaptive", "run-minimal"])
        by_id = {row["run_id"]: row for row in score["runs"]}
        self.assertEqual(by_id["run-dominated"]["dominated_by"], ["run-adaptive", "run-minimal"])
        self.assertTrue(by_id["run-adaptive"]["pareto_frontier"])
        self.assertTrue(by_id["run-minimal"]["pareto_frontier"])

    def test_hard_resource_cap_excludes_high_quality_over_budget_policy(self):
        score = evaluator.evaluate(self.bundle)
        by_id = {row["run_id"]: row for row in score["runs"]}
        self.assertFalse(by_id["run-always-on"]["eligible"])
        self.assertIn("hard_constraint:energy_joules:max", by_id["run-always-on"]["reason_codes"])

    def test_budget_slices_return_distinct_frontiers(self):
        score = evaluator.evaluate(self.bundle)
        slices = {row["slice_id"]: row for row in score["budget_slices"]}
        self.assertEqual(slices["battery-small"]["frontier_run_ids"], ["run-adaptive", "run-minimal"])
        self.assertEqual(slices["latency-critical"]["frontier_run_ids"], ["run-minimal"])

    def test_no_weighted_aggregate_is_created(self):
        score = evaluator.evaluate(self.bundle)
        self.assertIsNone(score["totals"]["single_weighted_score"])
        self.assertEqual(score["totals"]["run_count"], 4)

    def test_missing_metric_does_not_become_zero_or_a_win(self):
        candidate = copy.deepcopy(self.bundle)
        row = next(x for x in candidate["runs"] if x["run_id"] == "run-minimal")
        row["metrics"] = [m for m in row["metrics"] if m["metric_id"] != "energy_joules"]
        score = evaluator.evaluate(candidate)
        result = next(x for x in score["runs"] if x["run_id"] == "run-minimal")
        self.assertFalse(result["eligible"])
        self.assertIn("missing_metric:energy_joules", result["reason_codes"])
        self.assertNotIn("run-minimal", score["pareto_frontier_run_ids"])

    def test_estimated_energy_cannot_enter_measured_frontier(self):
        candidate = copy.deepcopy(self.bundle)
        candidate["comparison"]["measurement_class"] = "measured"
        for run in candidate["runs"]:
            for metric in run["metrics"]:
                metric["status"] = "MEASURED"
                metric["evidence_sha256"] = "a" * 64
                metric["method"] = "instrumented test trace"
        always_on = next(run for run in candidate["runs"] if run["run_id"] == "run-always-on")
        energy = next(metric for metric in always_on["metrics"] if metric["metric_id"] == "energy_joules")
        energy["status"] = "ESTIMATED"
        score = evaluator.evaluate(candidate)
        by_id = {row["run_id"]: row for row in score["runs"]}
        self.assertFalse(by_id["run-always-on"]["eligible"])
        self.assertIn("not_measured:energy_joules", by_id["run-always-on"]["reason_codes"])
        self.assertEqual(score["pareto_frontier_run_ids"], ["run-adaptive", "run-minimal"])

    def test_measured_metrics_require_evidence_digest_and_method(self):
        candidate = copy.deepcopy(self.bundle)
        candidate["comparison"]["measurement_class"] = "measured"
        for run in candidate["runs"]:
            for metric in run["metrics"]:
                metric["status"] = "MEASURED"
                metric["evidence_sha256"] = "a" * 64
                metric["method"] = "instrumented test trace"
        candidate["runs"][0]["metrics"][0].pop("evidence_sha256")
        score = evaluator.evaluate(candidate)
        row = next(x for x in score["runs"] if x["run_id"] == "run-always-on")
        self.assertFalse(row["eligible"])
        self.assertTrue(any(item.startswith("missing_measurement_evidence:") for item in row["reason_codes"]))

    def test_cross_cohort_comparison_is_rejected(self):
        candidate = copy.deepcopy(self.bundle)
        candidate["runs"][0]["match_identity"]["cohort_sha256"] = "f" * 64
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(candidate)

    def test_duplicate_metric_is_rejected(self):
        candidate = copy.deepcopy(self.bundle)
        row = next(x for x in candidate["runs"] if x["run_id"] == "run-adaptive")
        row["metrics"].append(copy.deepcopy(row["metrics"][0]))
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(candidate)

    def test_tolerance_prevents_false_dominance_for_equal_metrics(self):
        candidate = copy.deepcopy(self.bundle)
        # The minimal arm's silence advantage is within tolerance; it should not
        # establish strict dominance on that axis by itself.
        spec = next(x for x in candidate["comparison"]["metric_manifest"] if x["metric_id"] == "silence_compliance")
        spec["tolerance"] = 0.1
        score = evaluator.evaluate(candidate)
        self.assertIn("run-adaptive", score["pareto_frontier_run_ids"])
        self.assertIn("run-minimal", score["pareto_frontier_run_ids"])


if __name__ == "__main__":
    unittest.main()

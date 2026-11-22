import math
import unittest

from anytime_valid_policy_eval import (
    EvaluationError,
    anytime_interval,
    evaluate_document,
    evaluate_paired_differences,
)


class AnytimeValidPolicyEvalTests(unittest.TestCase):
    def test_interval_is_clipped_to_known_outcome_range(self):
        lower, upper = anytime_interval(30, 0.2, 0.05)
        self.assertGreaterEqual(lower, -1.0)
        self.assertLessEqual(upper, 1.0)
        self.assertLess(lower, upper)

    def test_strong_positive_effect_certifies_before_cap(self):
        result = evaluate_paired_differences([1.0] * 1000, alpha=0.05, margin=0.05)
        self.assertEqual(result["status"], "SUPERIOR")
        self.assertGreater(result["lower"], 0.05)
        self.assertLess(result["looks_used"], 1000)

    def test_tie_is_inconclusive_not_equivalence(self):
        result = evaluate_paired_differences([0.0] * 100, alpha=0.05, margin=0.0)
        self.assertEqual(result["status"], "INCONCLUSIVE")

    def test_strong_negative_effect_certifies_non_superiority(self):
        result = evaluate_paired_differences([-1.0] * 1000, alpha=0.05, margin=0.0)
        self.assertEqual(result["status"], "NOT_SUPERIOR")
        self.assertLessEqual(result["upper"], 0.0)

    def test_invalid_values_fail_closed(self):
        for bad in (1.0001, -1.0001, math.nan, math.inf, True, "0.5", None, 10**10000):
            with self.subTest(value=repr(bad)[:40]):
                with self.assertRaises(EvaluationError):
                    evaluate_paired_differences([bad])

    def test_invalid_alpha_and_margin_fail_closed(self):
        for alpha in (0.0, 1.0, -0.1, math.nan, True, "0.05", 10**10000):
            with self.subTest(alpha=repr(alpha)[:40]):
                with self.assertRaises(EvaluationError):
                    evaluate_paired_differences([0.1], alpha=alpha)
        for margin in (1.01, math.nan, True, "0.0", 10**10000):
            with self.subTest(margin=repr(margin)[:40]):
                with self.assertRaises(EvaluationError):
                    evaluate_paired_differences([0.1], margin=margin)

    def test_document_rejects_reused_source_cluster(self):
        document = {
            "schema_version": 1,
            "sampling_unit": "independent_source_cluster",
            "observations": [
                {"cluster_id": "recording-1", "paired_difference": 0.2},
                {"cluster_id": "recording-1", "paired_difference": 0.1},
            ],
        }
        with self.assertRaises(EvaluationError):
            evaluate_document(document)

    def test_document_rejects_boolean_or_float_schema_version(self):
        for version in (True, 1.0):
            with self.subTest(version=version):
                document = {
                    "schema_version": version,
                    "sampling_unit": "independent_source_cluster",
                    "observations": [
                        {"cluster_id": "recording-1", "paired_difference": 0.2}
                    ],
                }
                with self.assertRaises(EvaluationError):
                    evaluate_document(document)

    def test_report_counts_only_prefix_consumed_at_stopping_look(self):
        document = {
            "schema_version": 1,
            "sampling_unit": "independent_source_cluster",
            "metric_id": "task_accuracy",
            "candidate_policy_id": "active",
            "reference_policy_id": "fixed",
            "observations": [
                {"cluster_id": f"source-{i}", "paired_difference": 1.0}
                for i in range(1000)
            ],
        }
        result = evaluate_document(document)
        self.assertEqual(result["status"], "SUPERIOR")
        self.assertEqual(result["independent_cluster_count"], result["looks_used"])
        self.assertEqual(result["observations_provided"], 1000)
        self.assertLess(result["independent_cluster_count"], result["observations_provided"])
        self.assertEqual(len(result["consumed_cluster_ids_sha256"]), 64)

    def test_document_emits_replayable_identity_and_assumptions(self):
        document = {
            "schema_version": 1,
            "sampling_unit": "independent_source_cluster",
            "metric_id": "task_accuracy",
            "candidate_policy_id": "active",
            "reference_policy_id": "fixed",
            "observations": [
                {"cluster_id": f"source-{i}", "paired_difference": 1.0}
                for i in range(1000)
            ],
        }
        result = evaluate_document(document)
        self.assertEqual(result["metric_id"], "task_accuracy")
        self.assertIn("rows are sampled from one frozen target population of independent source clusters", result["assumptions"])


if __name__ == "__main__":
    unittest.main()

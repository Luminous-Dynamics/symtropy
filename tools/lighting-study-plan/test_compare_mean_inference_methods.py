from __future__ import annotations

import random
import unittest

import compare_mean_inference_methods as study


class MeanInferenceMethodTests(unittest.TestCase):
    def test_wilson_interval_contains_point_estimate(self):
        for successes, trials in ((0, 200), (10, 200), (100, 200), (200, 200)):
            interval = study.wilson_interval(successes, trials)
            estimate = successes / trials
            self.assertLessEqual(interval["lower"], estimate)
            self.assertGreaterEqual(interval["upper"], estimate)

    def test_invalid_wilson_inputs_rejected(self):
        with self.assertRaises(study.MethodStudyError):
            study.wilson_interval(12, 10)

    def test_seeded_stress_distributions_are_deterministic(self):
        for dist in study.DISTRIBUTIONS:
            left = random.Random(42)
            right = random.Random(42)
            self.assertEqual([study.draw_standardized(left, dist) for _ in range(100)],
                             [study.draw_standardized(right, dist) for _ in range(100)])

    def test_invalid_student_t_df_rejected(self):
        with self.assertRaisesRegex(study.MethodStudyError, "exceed 2"):
            study.draw_standardized(random.Random(4), "student_t", 2)

    def test_all_primary_methods_target_arithmetic_mean(self):
        values = [4.0, 5.0, 6.0, 7.0, 8.0]
        bounds = study.mean_lower_bounds(values, margin=3.0, alpha=0.05,
                                         bootstrap_replicates=199,
                                         percentile_rng=random.Random(1),
                                         studentized_rng=random.Random(2))
        self.assertEqual(set(bounds), {"student_t", "percentile_bootstrap", "studentized_bootstrap_t"})
        self.assertTrue(all(value > 3.0 for value in bounds.values()))

    def test_degenerate_sample_fails_closed(self):
        bounds = study.mean_lower_bounds([4.0] * 10, margin=3.0, alpha=0.05,
                                         bootstrap_replicates=199,
                                         percentile_rng=random.Random(1),
                                         studentized_rng=random.Random(2))
        self.assertTrue(all(value == float("-inf") for value in bounds.values()))

    def test_invalid_alpha_and_bootstrap_count_rejected(self):
        with self.assertRaisesRegex(study.MethodStudyError, "alpha"):
            study.mean_lower_bounds([1.0, 2.0], margin=0.0, alpha=0.5,
                                    bootstrap_replicates=199, percentile_rng=random.Random(1),
                                    studentized_rng=random.Random(2))
        with self.assertRaisesRegex(study.MethodStudyError, "bootstrap_replicates"):
            study.mean_lower_bounds([1.0, 2.0], margin=0.0, alpha=0.05,
                                    bootstrap_replicates=20, percentile_rng=random.Random(1),
                                    studentized_rng=random.Random(2))

    def test_sign_test_is_separate_median_estimand(self):
        # 3/3 positive signs: exact upper-tail p = 1/8.
        self.assertAlmostEqual(study.sign_test_median_p_value([6.0, 7.0, 8.0], 5.0), 0.125)
        # Ties are treated as non-positive, not silently dropped.
        self.assertAlmostEqual(study.sign_test_median_p_value([6.0, 5.0, 5.0], 5.0), 0.875)

    def test_sign_test_rejects_nonfinite_values(self):
        with self.assertRaises(study.MethodStudyError):
            study.sign_test_median_p_value([1.0, float("nan")], 0.0)

    def test_simulation_schema_and_synthetic_claim_ceiling(self):
        report = study.simulate(replicates=200, bootstrap_replicates=99, base_seed=15)
        self.assertEqual(report["decision"], "synthetic_inference_method_diagnostics_no_empirical_claim")
        self.assertEqual(report["summary"]["scenario_count"], 24)
        self.assertEqual(report["summary"]["null_scenario_count"], 8)
        self.assertTrue(report["methods_are_not_declared_winners"])
        self.assertEqual(report["sampling_unit"], "independent_scene_family")

    def test_left_skew_null_stresses_all_methods(self):
        report = study.simulate(replicates=500, bootstrap_replicates=199, base_seed=20261028)
        # Detection benchmark: at least one mean procedure's individual MC interval
        # should show that the declared small-sample rule may be anti-conservative.
        flagged = report["summary"]["null_scenarios_with_rejection_interval_above_alpha"]
        self.assertTrue(any(flagged[name] for name in ("student_t", "percentile_bootstrap",
                                                       "studentized_bootstrap_t")))

    def test_quantile_boundary_values(self):
        self.assertEqual(study._quantile([1.0, 2.0, 3.0], 0.0), 1.0)
        self.assertEqual(study._quantile([1.0, 2.0, 3.0], 1.0), 3.0)

    def test_unknown_distribution_rejected(self):
        with self.assertRaisesRegex(study.MethodStudyError, "unsupported"):
            study.draw_standardized(random.Random(1), "mystery")

    def test_invalid_sensitivity_inputs_rejected(self):
        with self.assertRaises(study.MethodStudyError):
            study.simulate(replicates=199)
        with self.assertRaises(study.MethodStudyError):
            study.simulate(replicates=200, bootstrap_replicates=98)

    def test_bootstrap_result_deterministic_for_fixed_streams(self):
        values = [1.2, 2.4, 2.8, 3.1, 6.7, 9.4]
        def run():
            return study.mean_lower_bounds(values, margin=1.0, alpha=0.05,
                bootstrap_replicates=199, percentile_rng=random.Random(7),
                studentized_rng=random.Random(8))
        self.assertEqual(run(), run())


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations
import random
import unittest

import calibrate_mean_inference_confirmation as study


class ConfirmatoryCalibrationTests(unittest.TestCase):
    def test_standardized_distributions_are_deterministic(self):
        for dist in study.DISTRIBUTIONS:
            a = random.Random(122)
            b = random.Random(122)
            self.assertEqual([study.draw_standardized(a, dist) for _ in range(50)],
                             [study.draw_standardized(b, dist) for _ in range(50)])

    def test_two_point_stress_distributions_have_expected_support(self):
        rng = random.Random(2)
        left = {study.draw_standardized(rng, "left_skew_two_point") for _ in range(1000)}
        self.assertEqual(left, {-3.0, 1.0 / 3.0})

    def test_wilson_one_sided_contains_estimate(self):
        for k, n in ((0, 200), (10, 200), (100, 200), (200, 200)):
            interval = study.wilson_one_sided(k, n, 2.5)
            estimate = k / n
            self.assertLessEqual(interval["lower"], estimate)
            self.assertGreaterEqual(interval["upper"], estimate)

    def test_simultaneous_critical_value_increases_with_family_size(self):
        self.assertGreater(study.simultaneous_z(0.05, 24), study.simultaneous_z(0.05, 3))

    def test_optimized_bounds_match_reference_comparator(self):
        values = [-2.3, 0.4, 1.1, 5.7, 4.4, 3.0, -0.8, 2.2, 7.1, 1.4, 0.7, 3.8]
        reference = study.methods.mean_lower_bounds(
            values, margin=1.0, alpha=0.05, bootstrap_replicates=199,
            percentile_rng=random.Random(120), studentized_rng=random.Random(121))
        optimized = study.mean_lower_bounds_fast(
            values, margin=1.0, alpha=0.05, bootstrap_replicates=199,
            percentile_rng=random.Random(120), studentized_rng=random.Random(121))
        for name in study.METHODS:
            self.assertAlmostEqual(reference[name], optimized[name], places=7)

    @unittest.skipIf(study.np is None, "NumPy accelerator is optional")
    def test_numpy_bounds_are_finite_and_deterministic(self):
        values = [-2.3, 0.4, 1.1, 5.7, 4.4, 3.0, -0.8, 2.2, 7.1, 1.4, 0.7, 3.8]
        def run():
            return study.mean_lower_bounds_numpy(values, margin=1.0, alpha=0.05,
                bootstrap_replicates=199, percentile_rng=study.np.random.default_rng(8),
                studentized_rng=study.np.random.default_rng(9))
        left, right = run(), run()
        self.assertEqual(left, right)
        self.assertEqual(set(left), set(study.METHODS))
        self.assertTrue(all(value == value for value in left.values()))

    def test_single_cell_reports_all_methods_and_failures(self):
        row = study._run_cell(n=12, distribution="normal", mean=5.0, margin=5.0,
                              family_sd_pp=3.0, outer_replicates=250,
                              bootstrap_replicates=199, seed=20261029, backend="stdlib")
        self.assertEqual(set(row["methods"]), set(study.METHODS))
        for metric in row["methods"].values():
            self.assertIn("failed_or_nonfinite_replicates", metric)
            self.assertIn("finite_interval_coverage_rate", metric)
            self.assertIn("unresolved_lower_bound_rate", metric)
            self.assertIsNotNone(metric["false_superiority_rate"])
        self.assertEqual(row["condition"], "null_at_margin")

    def test_input_gates_reject_underpowered_confirmatory_design(self):
        with self.assertRaisesRegex(study.CalibrationError, "null_replicates"):
            study.calibrate(null_replicates=4999)
        with self.assertRaisesRegex(study.CalibrationError, "bootstrap_replicates"):
            study.calibrate(null_replicates=5000, target_replicates=500, bootstrap_replicates=98)

    def test_alternative_reports_power_not_false_positive(self):
        row = study._run_cell(n=24, distribution="normal", mean=9.5, margin=5.0,
                              family_sd_pp=3.0, outer_replicates=250,
                              bootstrap_replicates=199, seed=22, backend="stdlib")
        for metric in row["methods"].values():
            self.assertIsNotNone(metric["power"])
            self.assertIsNone(metric["false_superiority_rate"])
        self.assertEqual(row["condition"], "target_1_5_sd_above_margin")


if __name__ == "__main__":
    unittest.main(verbosity=2)

import math
import random
import unittest

import fitted_mar_interval_mnar as audit


class FittedMarIntervalMnarTests(unittest.TestCase):
    def test_logistic_fit_recovers_monotone_response(self):
        rng = random.Random(17)
        x = [i / 199 for i in range(200)]
        response = [int(rng.random() < audit.logistic(1.8 - 1.2 * xx)) for xx in x]
        fitted = audit.fit_logistic_response(x, response)
        self.assertTrue(fitted["converged"])
        self.assertEqual(len(fitted["probabilities"]), len(x))
        self.assertGreater(fitted["probabilities"][0], fitted["probabilities"][-1])
        self.assertTrue(all(0.0 < p < 1.0 for p in fitted["probabilities"]))

    def test_complete_data_does_not_require_unidentifiable_response_model(self):
        x = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
        y = [1.0, 2.2, 3.1, 3.9, 5.2, 6.1]
        result = audit.analyze(x, y, margin=0.0, sensitivity_deltas=(-2.0, 0.0, 2.0))
        self.assertTrue(result["diagnostics"]["eligible_for_mar_claim"])
        self.assertAlmostEqual(result["methods"]["fitted_mar_aipw"]["estimate_pct_saving"], sum(y) / len(y))
        self.assertEqual(result["n_missing"], 0)

    def test_delta_shifts_estimate_by_missing_fraction(self):
        rng = random.Random(31)
        x = [rng.random() for _ in range(80)]
        y_full = [5.5 + 2.0 * (0.5 - xx) + rng.gauss(0, 1.4) for xx in x]
        y = [None if (i % 4 == 0 or i % 7 == 0) else v for i, v in enumerate(y_full)]
        result = audit.analyze(x, y, margin=5.0, sensitivity_deltas=(-4.0, 0.0, 4.0))
        curve = {row["delta_pp_missing_minus_mar_prediction"]: row for row in result["pattern_mixture_sensitivity"]["curve"]}
        expected = 4.0 * result["missing_fraction"]
        observed = (curve[4.0]["mean_estimate_pct_saving"] - curve[0.0]["mean_estimate_pct_saving"])
        self.assertAlmostEqual(observed, expected, places=9)
        self.assertNotEqual(curve[-4.0]["lower_95_one_sided"], curve[4.0]["lower_95_one_sided"])

    def test_invalid_and_nonfinite_inputs_fail_closed(self):
        with self.assertRaises(audit.AnalysisError):
            audit.analyze([0.0, 1.0, math.nan, 0.2, 0.3], [1, 2, 3, 4, 5])
        with self.assertRaises(audit.AnalysisError):
            audit.fit_logistic_response([0, 1, 2, 3, 4, 5], [1, 1, 1, 0, 0, 0])

    def test_positivity_diagnostic_is_reported_not_hidden(self):
        rng = random.Random(77)
        x = [rng.random() for _ in range(120)]
        y = []
        for xx in x:
            observed = rng.random() < audit.logistic(3.0 - 5.0 * xx)
            y.append(2.0 + xx + rng.gauss(0, 0.5) if observed else None)
        result = audit.analyze(x, y, margin=0.0, positivity_floor=0.05)
        self.assertIn("positivity_supported", result["diagnostics"])
        self.assertIn("maximum_observed_inverse_probability_weight", result["diagnostics"])
        self.assertIn("eligible_for_mar_claim", result["diagnostics"])

    def test_simulation_report_is_deterministic_and_marks_synthetic_only(self):
        kwargs = dict(replicates=100, seed=913, sample_sizes=(20,),
                      distributions=("normal_clipped",),
                      mechanisms=("complete", "mar_logistic_difficulty"))
        a = audit.simulate(**kwargs)
        b = audit.simulate(**kwargs)
        self.assertEqual(a, b)
        self.assertEqual(a["summary"]["scenario_count"], 4)
        self.assertIn("no real lighting measurement", a["claim_scope"])
        self.assertIn("scenario_results", a)

    def test_delta_grid_is_explicit_and_sorted(self):
        x = [i / 24 for i in range(25)]
        y = [None if i in (2, 7, 12, 18, 22) else 4.0 + 3.0 * xx for i, xx in enumerate(x)]
        result = audit.analyze(x, y, margin=5.0, sensitivity_deltas=(2, -2, 0))
        ds = [row["delta_pp_missing_minus_mar_prediction"] for row in result["pattern_mixture_sensitivity"]["curve"]]
        self.assertEqual(ds, [-2.0, 0.0, 2.0])
        self.assertIn("negative_delta_tipping_point_on_supplied_grid_pp", result["pattern_mixture_sensitivity"])


if __name__ == "__main__":
    unittest.main(verbosity=2)

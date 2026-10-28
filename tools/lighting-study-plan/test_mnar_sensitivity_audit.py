from __future__ import annotations
import json
import random
import unittest
import mnar_sensitivity_audit as audit

class MnarSensitivityTests(unittest.TestCase):
    def test_all_noise_distributions_are_deterministic(self):
        for name in audit.DISTRIBUTIONS:
            a,b=random.Random(9),random.Random(9)
            self.assertEqual([audit.draw_noise(a,name) for _ in range(100)], [audit.draw_noise(b,name) for _ in range(100)])

    def test_skew_noise_has_expected_mean_and_variance_analytically(self):
        right_mean=.1*3+.9*(-1/3)
        right_var=.1*9+.9*(1/9)
        self.assertAlmostEqual(right_mean,0.0)
        self.assertAlmostEqual(right_var,1.0)
        self.assertAlmostEqual(.1*(-3)+.9*(1/3),0.0)

    def test_missingness_probabilities(self):
        self.assertEqual(audit.missing_probability("complete",0,0,5),0)
        self.assertEqual(audit.missing_probability("mcar_30pct",0,0,5),.3)
        self.assertLess(audit.missing_probability("mar_difficulty",0,.1,5), audit.missing_probability("mar_difficulty",0,.9,5))
        self.assertGreater(audit.missing_probability("mnar_low_outcome",0,0,5), audit.missing_probability("mnar_low_outcome",10,0,5))

    def test_hoeffding_bound_rejects_invalid_values_and_bounds(self):
        with self.assertRaises(audit.MissingnessAuditError): audit.bounded_hoeffding_lower_bound([], -15, 25, .05)
        with self.assertRaises(audit.MissingnessAuditError): audit.bounded_hoeffding_lower_bound([26], -15, 25, .05)
        with self.assertRaises(audit.MissingnessAuditError): audit.bounded_hoeffding_lower_bound([0], 10, -10, .05)

    def test_wilson_interval_contains_observed_fraction(self):
        for k,n in ((0,100),(12,100),(100,100)):
            ci=audit.wilson_interval(k,n)
            self.assertLessEqual(ci["lower"], k/n)
            self.assertGreaterEqual(ci["upper"], k/n)

    def test_bounded_hoeffding_never_exceeds_completed_mean(self):
        values=[-15,0,4,10,25]
        lower=audit.bounded_hoeffding_lower_bound(values,-15,25,.05)
        self.assertLess(lower, sum(values)/len(values))

    def test_simulation_returns_synthetic_only_report(self):
        report=audit.simulate(replicates=200,seed=42,sample_sizes=(10,20))
        self.assertEqual(report["decision"],"synthetic_missingness_sensitivity_diagnostics_no_empirical_claim")
        self.assertEqual(report["summary"]["scenario_count"],2*4*4*2)
        self.assertEqual(report["sampling_unit"],"independent_scene_family")

    def test_mnar_complete_case_shows_selection_bias_on_null(self):
        report=audit.simulate(replicates=1500,seed=20261028,sample_sizes=(20,))
        null=[r for r in report["scenario_results"] if r["distribution"]=="skewed_right_two_point" and r["effect_condition"]=="null_at_margin"]
        mcar=next(r for r in null if r["missingness_mechanism"]=="mcar_30pct")
        mnar=next(r for r in null if r["missingness_mechanism"]=="mnar_low_outcome")
        self.assertGreater(mnar["mean_observed_case_effect_pct"], mcar["mean_observed_case_effect_pct"])
        self.assertGreater(mnar["complete_case_student_t"]["rejection_rate"], mnar["bounded_worst_case_hoeffding"]["rejection_rate"])

    def test_bounded_method_has_no_observed_null_inflation_signal_in_smoke(self):
        report=audit.simulate(replicates=1000,seed=7,sample_sizes=(10,20))
        self.assertEqual(report["summary"]["bounded_null_scenarios_with_intervals_above_alpha"], [])

    def test_fixed_seed_report_is_deterministic(self):
        a=audit.simulate(replicates=200,seed=123,sample_sizes=(10,))
        b=audit.simulate(replicates=200,seed=123,sample_sizes=(10,))
        self.assertEqual(json.dumps(a,sort_keys=True),json.dumps(b,sort_keys=True))

    def test_invalid_configuration_rejected(self):
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(replicates=99)
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(alpha=0)
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(sample_sizes=(4,))
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(lower_bound_pct=0,upper_bound_pct=1)

if __name__=="__main__": unittest.main(verbosity=2)

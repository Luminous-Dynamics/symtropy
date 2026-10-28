from __future__ import annotations

import copy
import json
import random
import unittest
from pathlib import Path

import simulate_power_operating_characteristics as sim

HERE = Path(__file__).parent
FIXTURE = HERE / "fixtures" / "synthetic-plan-output-example.json"


class OperatingCharacteristicsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan = json.loads(FIXTURE.read_text(encoding="utf-8"))

    def test_wilson_interval_contains_observed_rate(self):
        for k, n in ((0, 100), (5, 100), (50, 100), (100, 100)):
            ci = sim.wilson_interval(k, n)
            self.assertLessEqual(ci["lower"], k / n)
            self.assertGreaterEqual(ci["upper"], k / n)

    def test_invalid_binomial_interval_rejected(self):
        with self.assertRaises(sim.SimulationInputError):
            sim.wilson_interval(11, 10)

    def test_point_feasibility_is_not_mistaken_for_confidence_gate(self):
        self.assertLess(sim.wilson_lower_bound(10, 10, confidence=0.95), 0.95)
        self.assertGreater(sim.wilson_lower_bound(100, 100, confidence=0.95),
                           sim.wilson_lower_bound(10, 10, confidence=0.95))

    def test_draw_distributions_are_deterministic_from_seed(self):
        for distribution in ("normal", "skewed_lognormal_right", "skewed_lognormal_left", "student_t"):
            a = random.Random(123)
            b = random.Random(123)
            self.assertEqual([sim._draw_standardized(a, distribution) for _ in range(30)],
                             [sim._draw_standardized(b, distribution) for _ in range(30)])

    def test_simulation_is_deterministic_and_synthetic_only(self):
        a = sim.simulate(self.plan, replicates=150, base_seed=99)
        b = sim.simulate(self.plan, replicates=150, base_seed=99)
        self.assertEqual(a, b)
        self.assertEqual(a["decision"], "synthetic_simulation_diagnostics_no_empirical_claim")
        self.assertGreater(a["summary"]["scenario_count"], 10)
        self.assertGreater(a["summary"]["near_margin_power_scenarios"], 0)
        self.assertGreater(a["summary"]["type_i_error_scenarios"], 0)
        self.assertIn("sd_x1__null_skewed_lognormal_left",
                      a["study_design_flags"]["screening_potential_type_i_inflation_scenarios_unadjusted_individual_mc_intervals"])

    def test_null_normal_type_one_error_is_near_alpha(self):
        spec = {"scenario_id": "null", "purpose": "type_i_error_at_margin", "distribution": "normal",
                "mean_effect_pct": 5.0, "sigma_pp": 3.0, "n_families": 30,
                "baseline_failure_probability": 0.0, "candidate_failure_probability": 0.0, "margin_pct": 5.0}
        row = sim._simulate_scenario(spec, replicates=3500, seed=2026, alpha=0.05,
                                     feasibility_threshold=0.95, feasibility_confidence=0.95, student_t_df=3)
        self.assertLess(abs(row["energy_superiority_rejection_rate"] - 0.05), 0.025)
        ci = row["energy_superiority_rejection_rate_monte_carlo_95pct_wilson_ci"]
        self.assertLessEqual(ci["lower"], 0.05)
        self.assertGreaterEqual(ci["upper"], 0.05)

    def test_power_increases_with_effect_at_fixed_variance(self):
        common = {"purpose": "power_at_assumed_effect", "distribution": "normal", "sigma_pp": 4.0,
                  "n_families": 25, "baseline_failure_probability": 0.0,
                  "candidate_failure_probability": 0.0, "margin_pct": 5.0}
        low = sim._simulate_scenario({**common, "scenario_id": "low", "mean_effect_pct": 6.0},
                                     replicates=500, seed=14, alpha=0.05, feasibility_threshold=0.95,
                                     feasibility_confidence=0.95, student_t_df=3)
        high = sim._simulate_scenario({**common, "scenario_id": "high", "mean_effect_pct": 10.0},
                                      replicates=500, seed=14, alpha=0.05, feasibility_threshold=0.95,
                                      feasibility_confidence=0.95, student_t_df=3)
        self.assertGreater(high["energy_superiority_rejection_rate"], low["energy_superiority_rejection_rate"])

    def test_more_multiple_comparisons_reduces_power_at_fixed_n(self):
        assumptions = self.plan["design_assumptions"]
        alpha = assumptions["alpha_per_comparison_one_sided"]
        base = {"purpose": "power_at_assumed_effect", "distribution": "normal", "sigma_pp": 3.0,
                "n_families": 25, "baseline_failure_probability": 0.0,
                "candidate_failure_probability": 0.0, "margin_pct": 5.0, "mean_effect_pct": 7.0}
        single = sim._simulate_scenario({**base, "scenario_id": "single"}, replicates=450, seed=321,
                                        alpha=0.05, feasibility_threshold=0.95, feasibility_confidence=0.95,
                                        student_t_df=3)
        adjusted = sim._simulate_scenario({**base, "scenario_id": "adjusted"}, replicates=450, seed=321,
                                          alpha=0.01, feasibility_threshold=0.95, feasibility_confidence=0.95,
                                          student_t_df=3)
        self.assertLessEqual(adjusted["energy_superiority_rejection_rate"], single["energy_superiority_rejection_rate"])
        self.assertEqual(alpha, 0.05 / 3)

    def test_failures_reduce_available_pairs_and_are_not_zero_imputed(self):
        spec = {"scenario_id": "fail", "purpose": "joint_energy_and_feasibility_diagnostic", "distribution": "normal",
                "mean_effect_pct": 20.0, "sigma_pp": 2.0, "n_families": 20,
                "baseline_failure_probability": 0.0, "candidate_failure_probability": 0.25, "margin_pct": 5.0}
        row = sim._simulate_scenario(spec, replicates=250, seed=11, alpha=0.05,
                                     feasibility_threshold=0.95, feasibility_confidence=0.95, student_t_df=3)
        self.assertLess(row["mean_complete_paired_families"], 20)
        self.assertLess(row["mean_candidate_feasibility_rate"], 0.95)
        self.assertNotEqual(row["joint_superiority_and_point_feasibility_pass_rate"], row["energy_superiority_rejection_rate"])
        self.assertLessEqual(row["candidate_feasibility_confidence_gate_pass_rate"],
                             row["candidate_feasibility_point_gate_pass_rate"])

    def test_planner_report_with_unresolved_n_is_rejected(self):
        bad = copy.deepcopy(self.plan)
        bad["sensitivity"][0]["required_new_independent_families"] = None
        with self.assertRaisesRegex(sim.SimulationInputError, "unresolved"):
            sim.simulate(bad, replicates=100)

    def test_wrong_planner_schema_is_rejected(self):
        with self.assertRaisesRegex(sim.SimulationInputError, "not a recognized"):
            sim.simulate({"analysis_kind": "anything", "design_assumptions": {}, "sensitivity": []})

    def test_invalid_replicate_count_is_rejected(self):
        with self.assertRaisesRegex(sim.SimulationInputError, "replicates"):
            sim.simulate(self.plan, replicates=99)

    def test_left_and_right_skew_scenarios_are_explicit(self):
        specs = sim._scenario_specs(self.plan["design_assumptions"], self.plan["sensitivity"], 0.5)
        ids = {row["scenario_id"] for row in specs}
        self.assertTrue(any("skewed_lognormal_left" in item for item in ids))
        self.assertTrue(any("skewed_lognormal_right" in item for item in ids))

    def test_family_shift_is_explicit_and_reduces_effect(self):
        a = sim._scenario_specs(self.plan["design_assumptions"], self.plan["sensitivity"], 0.0)
        b = sim._scenario_specs(self.plan["design_assumptions"], self.plan["sensitivity"], 0.5)
        a_row = next(x for x in a if x["scenario_id"].endswith("shifted_midpoint_normal"))
        b_row = next(x for x in b if x["scenario_id"].endswith("shifted_midpoint_normal"))
        self.assertLessEqual(a_row["mean_effect_pct"], b_row["mean_effect_pct"])
        self.assertLess(b_row["mean_effect_pct"], self.plan["design_assumptions"]["assumed_true_mean_savings_pct"])

    def test_monte_carlo_ci_valid_on_extremes(self):
        zero = sim.wilson_interval(0, 100)
        one = sim.wilson_interval(100, 100)
        self.assertEqual(zero["lower"], 0.0)
        self.assertEqual(one["upper"], 1.0)


if __name__ == "__main__":
    unittest.main()

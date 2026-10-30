from __future__ import annotations

import csv
import hashlib
import io
import tempfile
import unittest
from pathlib import Path

import plan_independent_scene_families as plan

FIELDS = [
    "scene_family", "seed", "baseline_status", "candidate_status",
    "baseline_energy_wh_per_compliant_task_hour", "candidate_energy_wh_per_compliant_task_hour",
]


def make_csv(*, families: int = 12, seeds: int = 4, candidate_failure: set[tuple[int, int]] | None = None,
             effect: float = 0.15, flat: bool = False) -> str:
    failures = candidate_failure or set()
    buf = io.StringIO(newline="")
    writer = csv.DictWriter(buf, fieldnames=FIELDS)
    writer.writeheader()
    for f in range(families):
        family_noise = 0.004 * f
        for seed in range(seeds):
            baseline = 10.0 + f * 0.3 + seed * 0.03
            saved = 0.0 if flat else effect + family_noise + seed * 0.001
            fails = (f, seed) in failures
            writer.writerow({
                "scene_family": f"family-{f:02d}", "seed": str(seed),
                "baseline_status": "compliant", "candidate_status": "timeout" if fails else "compliant",
                "baseline_energy_wh_per_compliant_task_hour": f"{baseline:.8f}",
                "candidate_energy_wh_per_compliant_task_hour": "" if fails else f"{baseline * (1-saved):.8f}",
            })
    return buf.getvalue()


class StudyPlanTests(unittest.TestCase):
    def load_text(self, text: str):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "pilot.csv"
            path.write_text(text, encoding="utf-8")
            return plan.load_pilot(path), path.read_bytes()

    def report(self, text: str, **overrides):
        rows, raw = self.load_text(text)
        defaults = dict(input_sha256=hashlib.sha256(raw).hexdigest(),
            source_sha256=hashlib.sha256(Path(plan.__file__).read_bytes()).hexdigest(),
            evidence_tier="calibrated_simulation", assumed_effect_pct=20.0,
            superiority_margin_pct=5.0, target_power=0.80, alpha=0.05,
            sd_multipliers=[1.0, 1.5, 2.0], min_families=10, min_new_families=10,
            min_pair_coverage=0.90, min_feasibility=0.95, max_families=500)
        defaults.update(overrides)
        return plan.build_report(rows, **defaults)

    def test_valid_pilot_generates_three_sensitivity_scenarios(self):
        report = self.report(make_csv())
        self.assertEqual(report["decision"], "planning_estimate_only_requires_preregistration")
        self.assertEqual(len(report["sensitivity"]), 3)
        self.assertTrue(all(x["required_new_independent_families"] >= 10 for x in report["sensitivity"]))

    def test_synthetic_tier_never_makes_empirical_claim(self):
        report = self.report(make_csv(), evidence_tier="synthetic_contract")
        self.assertEqual(report["decision"], "synthetic_planning_demo_no_empirical_claim")
        self.assertIn("no empirical", report["claim_scope"])

    def test_failed_rows_remain_in_coverage_and_feasibility_denominators(self):
        report = self.report(make_csv(candidate_failure={(0, 0)}), min_feasibility=0.95)
        self.assertEqual(report["pilot"]["total_family_seed_pairs"], 48)
        self.assertEqual(report["pilot"]["paired_energy_rows"], 47)
        self.assertAlmostEqual(report["pilot"]["candidate_feasibility_rate"], 47/48)
        self.assertEqual(report["pilot"]["candidate_noncompliant_status_counts"], {"timeout": 1})

    def test_any_undercovered_family_blocks_plan(self):
        report = self.report(make_csv(candidate_failure={(0, 0), (0, 1)}), min_pair_coverage=0.9)
        self.assertEqual(report["decision"], "UNRESOLVED_PILOT_QUALITY_OR_COVERAGE_GATES")
        self.assertIsNone(report["sensitivity"][0]["required_new_independent_families"])

    def test_low_candidate_feasibility_blocks_conditional_energy_plan(self):
        failed = {(f, s) for f in range(12) for s in range(4) if s == 0}
        report = self.report(make_csv(candidate_failure=failed), min_feasibility=0.95)
        self.assertFalse(report["pilot_gates"]["checks"]["candidate_feasibility_gate"])
        self.assertEqual(report["decision"], "UNRESOLVED_PILOT_QUALITY_OR_COVERAGE_GATES")

    def test_more_variance_never_reduces_required_families(self):
        report = self.report(make_csv(), sd_multipliers=[1, 1.5, 2])
        ns = [x["required_new_independent_families"] for x in report["sensitivity"]]
        self.assertEqual(ns, sorted(ns))

    def test_effect_not_above_margin_is_unresolved(self):
        report = self.report(make_csv(), assumed_effect_pct=5.0, superiority_margin_pct=5.0)
        self.assertTrue(all(x["status"] == "UNRESOLVED_EFFECT_NOT_ABOVE_MARGIN" for x in report["sensitivity"]))
        self.assertEqual(report["decision"], "UNRESOLVED_DESIGN_SENSITIVITY")

    def test_too_few_pilot_families_blocks_plan(self):
        report = self.report(make_csv(families=8), min_families=10)
        self.assertFalse(report["pilot_gates"]["checks"]["minimum_pilot_scene_families"])

    def test_duplicate_family_seed_is_rejected(self):
        text = make_csv() + make_csv(families=1, seeds=1).splitlines()[1] + "\n"
        with self.assertRaisesRegex(plan.PlanInputError, "duplicate"):
            self.load_text(text)

    def test_unbalanced_seed_schedules_are_rejected(self):
        rows = list(csv.DictReader(io.StringIO(make_csv())))
        text_buf = io.StringIO(newline="")
        writer = csv.DictWriter(text_buf, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows([r for r in rows if not (r["scene_family"] == "family-00" and r["seed"] == "3")])
        with self.assertRaisesRegex(plan.PlanInputError, "unbalanced"):
            self.load_text(text_buf.getvalue())

    def test_energy_must_match_status(self):
        rows = list(csv.DictReader(io.StringIO(make_csv())))
        rows[0]["candidate_status"] = "timeout"
        text_buf = io.StringIO(newline="")
        writer = csv.DictWriter(text_buf, fieldnames=FIELDS)
        writer.writeheader(); writer.writerows(rows)
        with self.assertRaisesRegex(plan.PlanInputError, "must be blank"):
            self.load_text(text_buf.getvalue())

    def test_compliant_row_needs_positive_finite_energy(self):
        rows = list(csv.DictReader(io.StringIO(make_csv())))
        rows[0]["candidate_energy_wh_per_compliant_task_hour"] = "nan"
        text_buf = io.StringIO(newline="")
        writer = csv.DictWriter(text_buf, fieldnames=FIELDS)
        writer.writeheader(); writer.writerows(rows)
        with self.assertRaisesRegex(plan.PlanInputError, "finite"):
            self.load_text(text_buf.getvalue())

    def test_input_digest_stable_for_fixed_bytes(self):
        report = self.report(make_csv())
        self.assertEqual(len(report["input_csv_sha256"]), 64)
        self.assertEqual(len(report["planner_source_sha256"]), 64)

    def test_zero_variance_does_not_emit_fictitious_precision(self):
        report = self.report(make_csv(flat=True))
        self.assertEqual(report["decision"], "UNRESOLVED_DESIGN_SENSITIVITY")
        self.assertTrue(all(x["status"] == "UNRESOLVED_VARIANCE_NOT_ESTIMABLE" for x in report["sensitivity"]))

    def test_student_t_critical_value_matches_known_reference(self):
        self.assertAlmostEqual(plan._t_critical(0.05, 9), 1.8331129327, places=8)

    def test_power_rises_with_more_independent_families(self):
        small = plan._approx_power(12, sigma=4.0, excess_effect=5.0, alpha=0.05)
        large = plan._approx_power(40, sigma=4.0, excess_effect=5.0, alpha=0.05)
        self.assertLess(small, large)

    def test_bad_sensitivity_multipliers_rejected(self):
        rows, raw = self.load_text(make_csv())
        with self.assertRaisesRegex(plan.PlanInputError, "unique"):
            plan.build_report(rows, input_sha256=hashlib.sha256(raw).hexdigest(), source_sha256="0"*64,
                evidence_tier="simulation_only", assumed_effect_pct=20, superiority_margin_pct=5,
                target_power=.8, alpha=.05, sd_multipliers=[1, 1], min_families=10, min_new_families=10,
                min_pair_coverage=.9, min_feasibility=.95, max_families=500)

    def test_comparison_family_adjustment_never_reduces_required_families(self):
        pilot = make_csv()
        unadjusted = self.report(
            pilot, assumed_effect_pct=6.0, superiority_margin_pct=5.0,
            sd_multipliers=[1.0], comparison_family_size=1,
        )
        adjusted = self.report(
            pilot, assumed_effect_pct=6.0, superiority_margin_pct=5.0,
            sd_multipliers=[1.0], comparison_family_size=5,
        )
        self.assertAlmostEqual(unadjusted["design_assumptions"]["alpha_per_comparison_one_sided"], 0.05)
        self.assertAlmostEqual(adjusted["design_assumptions"]["alpha_per_comparison_one_sided"], 0.01)
        self.assertGreater(
            adjusted["sensitivity"][0]["required_new_independent_families"],
            unadjusted["sensitivity"][0]["required_new_independent_families"],
        )

    def test_non_integer_comparison_family_size_is_rejected(self):
        rows, raw = self.load_text(make_csv())
        with self.assertRaisesRegex(plan.PlanInputError, "positive integer"):
            plan.build_report(rows, input_sha256=hashlib.sha256(raw).hexdigest(), source_sha256="0"*64,
                evidence_tier="simulation_only", assumed_effect_pct=20, superiority_margin_pct=5,
                target_power=.8, alpha=.05, sd_multipliers=[1],
                min_families=10, min_new_families=10, min_pair_coverage=.9,
                min_feasibility=.95, max_families=500, comparison_family_size=0)

    def test_seed_count_does_not_define_independent_sample_size(self):
        report = self.report(make_csv(seeds=20))
        self.assertEqual(report["pilot"]["scene_family_count"], 12)
        self.assertEqual(report["pilot"]["total_family_seed_pairs"], 240)
        self.assertEqual(report["sampling_unit"], "independent_scene_family")


if __name__ == "__main__":
    unittest.main()

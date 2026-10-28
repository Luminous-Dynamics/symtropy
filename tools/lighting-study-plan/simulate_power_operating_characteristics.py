#!/usr/bin/env python3
"""Simulation-based operating-characteristic audit for the lighting study planner.

All outputs are synthetic statistical diagnostics, never measured lighting
performance. Sampling unit is an independent scene family; seeds/grid points
are not resampled as independent inferential units.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import statistics
import sys
from pathlib import Path
from typing import Any

from plan_independent_scene_families import PlanInputError, _t_critical

Z_975 = 1.959963984540054
SCHEMA_VERSION = "0.1.0"


class SimulationInputError(ValueError):
    """Malformed plan or simulation settings."""


def wilson_interval(successes: int, trials: int, z: float = Z_975) -> dict[str, float]:
    if trials < 1 or successes < 0 or successes > trials:
        raise SimulationInputError("Wilson interval requires 0 <= successes <= trials and trials > 0")
    p = successes / trials
    z2 = z * z
    denom = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / denom
    half = z * math.sqrt((p * (1.0 - p) / trials) + (z2 / (4.0 * trials * trials))) / denom
    return {"lower": max(0.0, min(p, center - half)), "upper": min(1.0, max(p, center + half))}


def wilson_lower_bound(successes: int, trials: int, confidence: float = 0.95) -> float:
    """One-sided Wilson score lower bound for a binomial success probability."""
    if trials < 1 or successes < 0 or successes > trials or not (0.5 < confidence < 1.0):
        raise SimulationInputError("invalid Wilson lower-bound inputs")
    p = successes / trials
    z = __import__("statistics").NormalDist().inv_cdf(confidence)
    z2 = z * z
    denom = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / denom
    half = z * math.sqrt((p * (1.0 - p) / trials) + (z2 / (4.0 * trials * trials))) / denom
    return max(0.0, min(p, center - half))


def _draw_standardized(rng: random.Random, distribution: str, degrees_freedom: int = 3) -> float:
    if distribution == "normal":
        return rng.gauss(0.0, 1.0)
    if distribution in ("skewed_lognormal_right", "skewed_lognormal_left"):
        # Mean-zero, variance-one shifted lognormal with signed skew.
        shape = 0.8
        raw = math.exp(shape * rng.gauss(0.0, 1.0))
        mean = math.exp(shape * shape / 2.0)
        variance = (math.exp(shape * shape) - 1.0) * math.exp(shape * shape)
        standardized = (raw - mean) / math.sqrt(variance)
        return standardized if distribution == "skewed_lognormal_right" else -standardized
    if distribution == "student_t":
        if degrees_freedom <= 2:
            raise SimulationInputError("Student-t degrees of freedom must be > 2 for unit variance")
        z = rng.gauss(0.0, 1.0)
        chi_square = rng.gammavariate(degrees_freedom / 2.0, 2.0)
        raw_t = z / math.sqrt(chi_square / degrees_freedom)
        return raw_t * math.sqrt((degrees_freedom - 2.0) / degrees_freedom)
    raise SimulationInputError(f"unknown distribution: {distribution}")


def _studentized_test(values: list[float], margin: float, alpha: float,
                      critical_cache: dict[int, float]) -> bool:
    if len(values) < 2:
        return False
    mean = statistics.mean(values)
    sd = statistics.stdev(values)
    if not math.isfinite(mean) or not math.isfinite(sd) or sd <= 0.0:
        # Degenerate samples cannot certify superiority through an infinite statistic.
        return False
    n = len(values)
    t_stat = (mean - margin) / (sd / math.sqrt(n))
    critical = critical_cache.get(n)
    if critical is None:
        critical = _t_critical(alpha, n - 1)
        critical_cache[n] = critical
    return t_stat > critical


def _scenario_specs(assumptions: dict[str, Any], sensitivity: list[dict[str, Any]],
                    shifted_effect_fraction: float) -> list[dict[str, Any]]:
    margin = float(assumptions["superiority_margin_savings_pct"])
    target = float(assumptions["assumed_true_mean_savings_pct"])
    gap = target - margin
    midpoint = margin + gap * shifted_effect_fraction
    specs: list[dict[str, Any]] = []
    for row in sensitivity:
        n = row.get("required_new_independent_families")
        sigma = row.get("assumed_sd_pp")
        if not isinstance(n, int) or n < 2:
            raise SimulationInputError("every sensitivity row needs >= 2 planned independent families")
        if not isinstance(sigma, (int, float)) or not math.isfinite(float(sigma)) or float(sigma) <= 0:
            raise SimulationInputError("every sensitivity row needs a finite positive assumed_sd_pp")
        label = f"sd_x{row['pilot_sd_multiplier']:g}"
        for dist in ("normal", "skewed_lognormal_right", "skewed_lognormal_left", "student_t"):
            specs.append({"scenario_id": f"{label}__null_{dist}", "purpose": "type_i_error_at_margin",
                          "distribution": dist, "mean_effect_pct": margin, "sigma_pp": float(sigma),
                          "n_families": n, "baseline_failure_probability": 0.0,
                          "candidate_failure_probability": 0.0})
        for dist in ("normal", "skewed_lognormal_right", "skewed_lognormal_left", "student_t"):
            specs.append({"scenario_id": f"{label}__target_{dist}", "purpose": "power_at_assumed_effect",
                          "distribution": dist, "mean_effect_pct": target, "sigma_pp": float(sigma),
                          "n_families": n, "baseline_failure_probability": 0.0,
                          "candidate_failure_probability": 0.0})
        specs.append({"scenario_id": f"{label}__near_margin_one_sd_normal", "purpose": "power_near_margin_one_sd",
                      "distribution": "normal", "mean_effect_pct": margin + float(sigma), "sigma_pp": float(sigma),
                      "n_families": n, "baseline_failure_probability": 0.0,
                      "candidate_failure_probability": 0.0})
        specs.append({"scenario_id": f"{label}__shifted_midpoint_normal", "purpose": "power_under_family_shift",
                      "distribution": "normal", "mean_effect_pct": midpoint, "sigma_pp": float(sigma),
                      "n_families": n, "baseline_failure_probability": 0.0,
                      "candidate_failure_probability": 0.0})
        specs.append({"scenario_id": f"{label}__target_with_failures", "purpose": "joint_energy_and_feasibility_diagnostic",
                      "distribution": "normal", "mean_effect_pct": target, "sigma_pp": float(sigma),
                      "n_families": n, "baseline_failure_probability": 0.02,
                      "candidate_failure_probability": 0.05})
    return specs


def _simulate_scenario(spec: dict[str, Any], *, replicates: int, seed: int, alpha: float,
                       feasibility_threshold: float, feasibility_confidence: float,
                       student_t_df: int) -> dict[str, Any]:
    rng = random.Random(seed)
    n = int(spec["n_families"])
    mean_effect = float(spec["mean_effect_pct"])
    sigma = float(spec["sigma_pp"])
    baseline_failure_probability = float(spec["baseline_failure_probability"])
    candidate_failure_probability = float(spec["candidate_failure_probability"])
    if not (0 <= baseline_failure_probability < 1 and 0 <= candidate_failure_probability < 1):
        raise SimulationInputError("failure probabilities must be in [0,1)")

    critical_cache: dict[int, float] = {}
    reject_count = 0
    candidate_gate_count = 0
    baseline_gate_count = 0
    candidate_confident_gate_count = 0
    baseline_confident_gate_count = 0
    joint_count = 0
    joint_confident_count = 0
    paired_counts: list[int] = []
    candidate_feasible_rates: list[float] = []
    baseline_feasible_rates: list[float] = []

    for _ in range(replicates):
        effects: list[float] = []
        candidate_ok = 0
        baseline_ok = 0
        for _family in range(n):
            effect = mean_effect + sigma * _draw_standardized(rng, str(spec["distribution"]), student_t_df)
            b_ok = rng.random() >= baseline_failure_probability
            c_ok = rng.random() >= candidate_failure_probability
            baseline_ok += int(b_ok)
            candidate_ok += int(c_ok)
            # Missing/non-compliant outputs stay absent and count against feasibility.
            # The effect distribution is assumed independent of failure (MCAR) here.
            if b_ok and c_ok:
                effects.append(effect)
        candidate_rate = candidate_ok / n
        baseline_rate = baseline_ok / n
        energy_reject = _studentized_test(effects, float(spec["margin_pct"]), alpha, critical_cache) if "margin_pct" in spec else False
        candidate_gate = candidate_rate >= feasibility_threshold
        baseline_gate = baseline_rate >= feasibility_threshold
        candidate_confident_gate = wilson_lower_bound(candidate_ok, n, feasibility_confidence) >= feasibility_threshold
        baseline_confident_gate = wilson_lower_bound(baseline_ok, n, feasibility_confidence) >= feasibility_threshold
        reject_count += int(energy_reject)
        candidate_gate_count += int(candidate_gate)
        baseline_gate_count += int(baseline_gate)
        candidate_confident_gate_count += int(candidate_confident_gate)
        baseline_confident_gate_count += int(baseline_confident_gate)
        joint_count += int(energy_reject and candidate_gate and baseline_gate)
        joint_confident_count += int(energy_reject and candidate_confident_gate and baseline_confident_gate)
        paired_counts.append(len(effects))
        candidate_feasible_rates.append(candidate_rate)
        baseline_feasible_rates.append(baseline_rate)

    energy_ci = wilson_interval(reject_count, replicates)
    candidate_ci = wilson_interval(candidate_gate_count, replicates)
    baseline_ci = wilson_interval(baseline_gate_count, replicates)
    candidate_confident_ci = wilson_interval(candidate_confident_gate_count, replicates)
    baseline_confident_ci = wilson_interval(baseline_confident_gate_count, replicates)
    joint_ci = wilson_interval(joint_count, replicates)
    joint_confident_ci = wilson_interval(joint_confident_count, replicates)
    return {
        "scenario_id": spec["scenario_id"],
        "purpose": spec["purpose"],
        "distribution": spec["distribution"],
        "student_t_degrees_freedom": student_t_df if spec["distribution"] == "student_t" else None,
        "assumed_true_mean_savings_pct": mean_effect,
        "superiority_margin_pct": float(spec["margin_pct"]),
        "family_effect_sd_percentage_points": sigma,
        "planned_independent_scene_families": n,
        "replicates": replicates,
        "seed": seed,
        "alpha_per_comparison_one_sided": alpha,
        "baseline_failure_probability_assumed": baseline_failure_probability,
        "candidate_failure_probability_assumed": candidate_failure_probability,
        "failure_assumption": "missing completely at random independent of family effect; sensitivity only, not a general missing-data model",
        "energy_superiority_rejection_rate": reject_count / replicates,
        "energy_superiority_rejection_rate_monte_carlo_95pct_wilson_ci": energy_ci,
        "mean_complete_paired_families": statistics.mean(paired_counts),
        "candidate_feasibility_point_gate_pass_rate": candidate_gate_count / replicates,
        "candidate_feasibility_point_gate_pass_rate_monte_carlo_95pct_wilson_ci": candidate_ci,
        "candidate_feasibility_confidence_gate_pass_rate": candidate_confident_gate_count / replicates,
        "candidate_feasibility_confidence_gate_pass_rate_monte_carlo_95pct_wilson_ci": candidate_confident_ci,
        "baseline_feasibility_point_gate_pass_rate": baseline_gate_count / replicates,
        "baseline_feasibility_point_gate_pass_rate_monte_carlo_95pct_wilson_ci": baseline_ci,
        "baseline_feasibility_confidence_gate_pass_rate": baseline_confident_gate_count / replicates,
        "baseline_feasibility_confidence_gate_pass_rate_monte_carlo_95pct_wilson_ci": baseline_confident_ci,
        "joint_superiority_and_point_feasibility_pass_rate": joint_count / replicates,
        "joint_point_gate_pass_rate_monte_carlo_95pct_wilson_ci": joint_ci,
        "joint_superiority_and_confidence_bound_feasibility_pass_rate": joint_confident_count / replicates,
        "joint_confidence_gate_pass_rate_monte_carlo_95pct_wilson_ci": joint_confident_ci,
        "mean_candidate_feasibility_rate": statistics.mean(candidate_feasible_rates),
        "mean_baseline_feasibility_rate": statistics.mean(baseline_feasible_rates),
        "critical_value_cache_entries": len(critical_cache),
    }


def simulate(plan_report: dict[str, Any], *, replicates: int = 3000, base_seed: int = 271828,
             shifted_effect_fraction: float = 0.5, student_t_df: int = 3,
             feasibility_confidence: float = 0.95, max_scenarios: int = 100) -> dict[str, Any]:
    if plan_report.get("analysis_kind") != "pilot_based_independent_scene_family_power_planning":
        raise SimulationInputError("input is not a recognized independent-scene-family planner report")
    if replicates < 100 or replicates > 1_000_000:
        raise SimulationInputError("replicates must be in [100, 1000000]")
    if not isinstance(base_seed, int) or isinstance(base_seed, bool) or base_seed < 0:
        raise SimulationInputError("base_seed must be a non-negative integer")
    if not (0.0 <= shifted_effect_fraction <= 1.0):
        raise SimulationInputError("shifted_effect_fraction must be in [0,1]")
    if student_t_df <= 2:
        raise SimulationInputError("student_t_df must exceed 2 for finite variance")
    if not (0.5 < feasibility_confidence < 1.0):
        raise SimulationInputError("feasibility_confidence must be in (0.5,1.0)")
    assumptions = plan_report.get("design_assumptions")
    sensitivity = plan_report.get("sensitivity")
    if not isinstance(assumptions, dict) or not isinstance(sensitivity, list) or not sensitivity:
        raise SimulationInputError("planner report lacks design assumptions or sensitivity rows")
    if any(row.get("required_new_independent_families") is None for row in sensitivity):
        raise SimulationInputError("planner report includes unresolved sample-size sensitivities")
    alpha = float(assumptions.get("alpha_per_comparison_one_sided", 0))
    if not 0 < alpha < 0.5:
        raise SimulationInputError("planner report has invalid per-comparison alpha")
    margin = float(assumptions["superiority_margin_savings_pct"])
    feasibility_threshold = float(assumptions["min_feasibility_rate_per_method"])
    specs = _scenario_specs(assumptions, sensitivity, shifted_effect_fraction)
    if len(specs) > max_scenarios:
        raise SimulationInputError(f"scenario count {len(specs)} exceeds configured limit {max_scenarios}")
    results = []
    for index, spec in enumerate(specs):
        spec = {**spec, "margin_pct": margin}
        scenario_seed = base_seed + index * 1009
        results.append(_simulate_scenario(spec, replicates=replicates, seed=scenario_seed, alpha=alpha,
                                           feasibility_threshold=feasibility_threshold,
                                           feasibility_confidence=feasibility_confidence,
                                           student_t_df=student_t_df))
    by_purpose: dict[str, list[dict[str, Any]]] = {}
    for row in results:
        by_purpose.setdefault(row["purpose"], []).append(row)
    null_rows = [row for row in results if row["purpose"] == "type_i_error_at_margin"]
    flagged_type_i_inflation = [
        row["scenario_id"] for row in null_rows
        if row["energy_superiority_rejection_rate_monte_carlo_95pct_wilson_ci"]["lower"] > alpha
    ]
    conservative_null_scenarios = [
        row["scenario_id"] for row in null_rows
        if row["energy_superiority_rejection_rate_monte_carlo_95pct_wilson_ci"]["upper"] < alpha
    ]
    return {
        "schema_version": SCHEMA_VERSION,
        "analysis_kind": "simulation_based_operating_characteristic_audit",
        "decision": "synthetic_simulation_diagnostics_no_empirical_claim",
        "study_design_flags": {
            "screening_potential_type_i_inflation_scenarios_unadjusted_individual_mc_intervals": flagged_type_i_inflation,
            "screening_potentially_conservative_null_scenarios_unadjusted_individual_mc_intervals": conservative_null_scenarios,
            "interpretation": "exploratory diagnostic flags only; intervals are per-scenario, not simultaneous across the scenario family",
        },
        "claim_scope": "synthetic statistical operating-characteristic diagnostics only; never empirical lighting performance",
        "planner_input_sha256": plan_report.get("input_csv_sha256"),
        "planner_source_sha256": plan_report.get("planner_source_sha256"),
        "simulation_method_source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "sampling_unit": "independent_scene_family",
        "seed_and_grid_point_rule": "seeds/grid points are not independent inferential units; simulator draws family-level outcomes",
        "overall_configuration": {
            "replicates_per_scenario": replicates,
            "base_seed": base_seed,
            "family_wise_alpha": assumptions.get("alpha_familywise_one_sided"),
            "comparison_family_size": assumptions.get("comparison_family_size"),
            "alpha_per_comparison_one_sided": alpha,
            "student_t_degrees_freedom": student_t_df,
            "shifted_effect_fraction_of_gap_above_margin": shifted_effect_fraction,
            "minimum_feasibility_rate_per_method": feasibility_threshold,
            "feasibility_confidence_level_one_sided": feasibility_confidence,
            "feasibility_confidence_gate": "one-sided Wilson score lower confidence bound must exceed the minimum feasibility rate",
            "monte_carlo_interval": "95% Wilson score interval for each simulated proportion",
            "inferential_test": "one-sided one-sample Student-t test of family-level saving percentage against superiority margin",
        },
        "summary": {
            "scenario_count": len(results),
            "type_i_error_scenarios": sum(row["purpose"] == "type_i_error_at_margin" for row in results),
            "power_scenarios": sum(row["purpose"] == "power_at_assumed_effect" for row in results),
            "near_margin_power_scenarios": sum(row["purpose"] == "power_near_margin_one_sd" for row in results),
            "distribution_shift_scenarios": sum(row["purpose"] == "power_under_family_shift" for row in results),
            "failure_sensitivity_scenarios": sum(row["purpose"] == "joint_energy_and_feasibility_diagnostic" for row in results),
            "max_observed_null_rejection_rate": max(row["energy_superiority_rejection_rate"] for row in results if row["purpose"] == "type_i_error_at_margin"),
            "min_observed_target_power_rate": min(row["energy_superiority_rejection_rate"] for row in results if row["purpose"] == "power_at_assumed_effect"),
            "note": "Compare rejection rates with target power and alpha together with Monte Carlo intervals; use them to find failure modes, not to manufacture a pass criterion after observing results.",
        },
        "scenario_results": results,
        "grouped_diagnostics": {purpose: {"scenario_count": len(items),
            "min_rate": min(item["energy_superiority_rejection_rate"] for item in items),
            "max_rate": max(item["energy_superiority_rejection_rate"] for item in items)}
            for purpose, items in sorted(by_purpose.items())},
        "limitations": [
            "The simulator validates the declared one-sample Student-t analysis, not every possible confirmatory method.",
            "The left- and right-skewed and heavy-tailed families are standardized parametric stress distributions, not empirical distributions of real buildings.",
            "Failure probabilities are illustrative and missingness is modeled independent of family effect; outcome-dependent failures need separate sensitivity analysis.",
            "A fixed assumed family-level SD is supplied per planner sensitivity row; pilot SD uncertainty is not modeled by this version.",
            "The synthetic simulator cannot validate Radiance optical accuracy, lighting service, safety, product compliance, or energy savings.",
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("planner_report", type=Path, help="JSON from plan_independent_scene_families.py")
    parser.add_argument("--replicates", type=int, default=3000)
    parser.add_argument("--seed", type=int, default=271828)
    parser.add_argument("--shifted-effect-fraction", type=float, default=0.5)
    parser.add_argument("--student-t-df", type=int, default=3)
    parser.add_argument("--feasibility-confidence", type=float, default=0.95,
                        help="one-sided confidence level for feasibility Wilson lower bound")
    parser.add_argument("--max-scenarios", type=int, default=100)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        raw = args.planner_report.read_bytes()
        plan_report = json.loads(raw)
        if not isinstance(plan_report, dict):
            raise SimulationInputError("planner report JSON root must be an object")
        report = simulate(plan_report, replicates=args.replicates, base_seed=args.seed,
                          shifted_effect_fraction=args.shifted_effect_fraction,
                          student_t_df=args.student_t_df, feasibility_confidence=args.feasibility_confidence,
                          max_scenarios=args.max_scenarios)
        report["planner_report_raw_bytes_sha256"] = hashlib.sha256(raw).hexdigest()
    except (OSError, json.JSONDecodeError, SimulationInputError, PlanInputError, ValueError, KeyError, TypeError) as exc:
        print(f"lighting operating-characteristic simulation: FAIL: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(f"lighting operating-characteristic simulation: {report['decision']}")
    print(f"scenarios={report['summary']['scenario_count']} replicates_per_scenario={args.replicates}")
    print(f"max_null_rejection={report['summary']['max_observed_null_rejection_rate']:.4f} ")
    print(f"min_target_power={report['summary']['min_observed_target_power_rate']:.4f}")
    print(f"screening_potential_type_i_inflation={len(report['study_design_flags']['screening_potential_type_i_inflation_scenarios_unadjusted_individual_mc_intervals'])} scenario(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

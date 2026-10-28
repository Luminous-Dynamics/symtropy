#!/usr/bin/env python3
"""Sensitivity analysis for informative missingness in lighting scene-family studies.

Synthetic method diagnostic only. Independent scene families are the sampling units.
The robust arm assigns the predeclared lower outcome bound to missing outcomes and
uses Hoeffding's inequality to produce a distribution-free lower bound for the
mean, under the declared bounded-outcome and independent-family assumptions.
"""
from __future__ import annotations

import argparse
from functools import lru_cache
import hashlib
import json
import math
import random
import statistics
import sys
from pathlib import Path
from typing import Any

import plan_independent_scene_families as planner_module
from plan_independent_scene_families import _t_critical

SCHEMA_VERSION = "0.1.0"
DISTRIBUTIONS = ("clipped_normal_4sd", "skewed_right_two_point", "skewed_left_two_point", "rare_extremes")
MISSINGNESS = ("complete", "mcar_30pct", "mar_difficulty", "mnar_low_outcome")
SAMPLE_SIZES = (10, 20, 40)


class MissingnessAuditError(ValueError):
    """Invalid simulation configuration."""


def wilson_interval(successes: int, trials: int, z: float = 1.959963984540054) -> dict[str, float]:
    if trials < 1 or successes < 0 or successes > trials:
        raise MissingnessAuditError("Wilson interval requires 0 <= successes <= trials and trials > 0")
    p = successes / trials
    z2 = z * z
    denom = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / denom
    half = z * math.sqrt(p * (1.0 - p) / trials + z2 / (4.0 * trials * trials)) / denom
    return {"lower": max(0.0, min(p, center - half)), "upper": min(1.0, max(p, center + half))}


def draw_noise(rng: random.Random, distribution: str) -> float:
    """Mean-zero, unit-scale, bounded noise variants for stress testing."""
    if distribution == "clipped_normal_4sd":
        return max(-4.0, min(4.0, rng.gauss(0.0, 1.0)))
    if distribution == "skewed_right_two_point":
        # P(Z=3)=0.1 and P(Z=-1/3)=0.9; E[Z]=0, Var[Z]=1.
        return 3.0 if rng.random() < 0.1 else -1.0 / 3.0
    if distribution == "skewed_left_two_point":
        return -3.0 if rng.random() < 0.1 else 1.0 / 3.0
    if distribution == "rare_extremes":
        # P(-sqrt(10))=.05, P(0)=.90, P(+sqrt(10))=.05; E=0, Var=1.
        u = rng.random()
        if u < 0.05:
            return -math.sqrt(10.0)
        if u >= 0.95:
            return math.sqrt(10.0)
        return 0.0
    raise MissingnessAuditError(f"unsupported effect distribution {distribution!r}")


def missing_probability(mechanism: str, effect: float, difficulty: float, margin: float) -> float:
    if mechanism == "complete":
        return 0.0
    if mechanism == "mcar_30pct":
        return 0.30
    if mechanism == "mar_difficulty":
        # Difficulty is observed for every attempted family. The effect depends on it,
        # so a complete-case estimator that ignores it can be selection-biased.
        return 0.05 + 0.65 * difficulty
    if mechanism == "mnar_low_outcome":
        # Missingness depends directly on the latent, unobserved effect.
        return 0.65 if effect < margin else 0.05
    raise MissingnessAuditError(f"unsupported missingness mechanism {mechanism!r}")


@lru_cache(maxsize=512)
def _critical_t(alpha: float, degrees_freedom: int) -> float:
    return _t_critical(alpha, degrees_freedom)


def student_t_lower_bound(values: list[float], alpha: float) -> float:
    if len(values) < 2:
        return -math.inf
    mean = statistics.fmean(values)
    sd = statistics.stdev(values)
    if not math.isfinite(mean) or not math.isfinite(sd) or sd <= 0.0:
        return -math.inf
    return mean - _critical_t(alpha, len(values) - 1) * sd / math.sqrt(len(values))


def bounded_hoeffding_lower_bound(completed: list[float], lower: float, upper: float, alpha: float) -> float:
    """Distribution-free one-sided lower bound for the mean of bounded independent outcomes."""
    if not completed or any(not math.isfinite(x) or x < lower or x > upper for x in completed):
        raise MissingnessAuditError("completed outcomes must be finite and inside declared bounds")
    if not (math.isfinite(lower) and math.isfinite(upper) and lower < upper):
        raise MissingnessAuditError("outcome bounds must be finite and lower < upper")
    if not (0.0 < alpha < 1.0):
        raise MissingnessAuditError("alpha must be in (0,1)")
    radius = (upper - lower) * math.sqrt(math.log(1.0 / alpha) / (2.0 * len(completed)))
    return statistics.fmean(completed) - radius


def simulate(*, replicates: int = 2500, seed: int = 20261028, alpha: float = 0.05,
             margin_pct: float = 5.0, family_sd_pp: float = 3.0,
             lower_bound_pct: float = -15.0, upper_bound_pct: float = 25.0,
             difficulty_slope_pp: float = 3.0,
             sample_sizes: tuple[int, ...] = SAMPLE_SIZES) -> dict[str, Any]:
    if replicates < 200 or replicates > 1_000_000:
        raise MissingnessAuditError("replicates must be in [200,1000000]")
    if not isinstance(seed, int) or isinstance(seed, bool) or seed < 0:
        raise MissingnessAuditError("seed must be a non-negative integer")
    if not (0.0 < alpha < 0.5):
        raise MissingnessAuditError("alpha must be in (0,0.5)")
    if not all(math.isfinite(v) for v in (margin_pct, family_sd_pp, lower_bound_pct, upper_bound_pct, difficulty_slope_pp)):
        raise MissingnessAuditError("all numeric configuration values must be finite")
    if family_sd_pp <= 0.0 or lower_bound_pct >= upper_bound_pct or difficulty_slope_pp < 0.0:
        raise MissingnessAuditError("SD must be positive; outcome bounds ordered; difficulty slope nonnegative")
    if not sample_sizes or any(not isinstance(n, int) or isinstance(n, bool) or n < 5 for n in sample_sizes):
        raise MissingnessAuditError("sample sizes must be integers >= 5")
    max_noise = max(4.0, math.sqrt(10.0)) * family_sd_pp + difficulty_slope_pp / 2.0
    max_alt_center = margin_pct + 2.0 * family_sd_pp
    min_center = margin_pct
    if min_center - max_noise < lower_bound_pct or max_alt_center + max_noise > upper_bound_pct:
        raise MissingnessAuditError("declared bounds do not contain all stress effects; widen bounds or reduce scale")

    scenario_rows: list[dict[str, Any]] = []
    index = 0
    effect_conditions = (("null_at_margin", 0.0), ("target_two_sd_above_margin", 2.0))
    for n in sample_sizes:
        for distribution in DISTRIBUTIONS:
            for mechanism in MISSINGNESS:
                for condition, multiplier in effect_conditions:
                    center = margin_pct + multiplier * family_sd_pp
                    rng = random.Random(seed + index * 1009)
                    naive_rejects = 0
                    robust_rejects = 0
                    complete_counts: list[int] = []
                    mean_observed: list[float] = []
                    mean_completed: list[float] = []
                    missing_fracs: list[float] = []
                    for _ in range(replicates):
                        observed: list[float] = []
                        completed: list[float] = []
                        for _family in range(n):
                            difficulty = rng.random()
                            effect = center + family_sd_pp * draw_noise(rng, distribution)
                            effect += difficulty_slope_pp * (0.5 - difficulty)
                            if effect < lower_bound_pct - 1e-12 or effect > upper_bound_pct + 1e-12:
                                raise MissingnessAuditError("DGP generated an effect outside its frozen bounds")
                            p_miss = missing_probability(mechanism, effect, difficulty, margin_pct)
                            is_missing = rng.random() < p_miss
                            if is_missing:
                                completed.append(lower_bound_pct)
                            else:
                                observed.append(effect)
                                completed.append(effect)
                        naive_lower = student_t_lower_bound(observed, alpha)
                        robust_lower = bounded_hoeffding_lower_bound(completed, lower_bound_pct, upper_bound_pct, alpha)
                        naive_rejects += int(naive_lower > margin_pct)
                        robust_rejects += int(robust_lower > margin_pct)
                        complete_counts.append(len(observed))
                        if observed:
                            mean_observed.append(statistics.fmean(observed))
                        mean_completed.append(statistics.fmean(completed))
                        missing_fracs.append((n - len(observed)) / n)
                    naive_ci = wilson_interval(naive_rejects, replicates)
                    robust_ci = wilson_interval(robust_rejects, replicates)
                    scenario_rows.append({
                        "scenario_id": f"n{n}__{distribution}__{mechanism}__{condition}",
                        "independent_scene_families": n,
                        "distribution": distribution,
                        "missingness_mechanism": mechanism,
                        "effect_condition": condition,
                        "true_mean_effect_location_pct": center,
                        "superiority_margin_pct": margin_pct,
                        "family_sd_percentage_points": family_sd_pp,
                        "outcome_bounds_pct": {"lower": lower_bound_pct, "upper": upper_bound_pct},
                        "replicates": replicates,
                        "seed": seed + index * 1009,
                        "complete_case_student_t": {
                            "estimand_warning": "targets the observed-case mean; it is not generally the full-population mean under informative missingness",
                            "rejection_rate": naive_rejects / replicates,
                            "rejection_rate_monte_carlo_95pct_wilson_ci": naive_ci,
                        },
                        "bounded_worst_case_hoeffding": {
                            "estimand": "full_population_mean_savings under the assumption that every missing latent effect lies within the declared bounds",
                            "rejection_rate": robust_rejects / replicates,
                            "rejection_rate_monte_carlo_95pct_wilson_ci": robust_ci,
                            "bound_method": "replace each missing effect by the declared lower bound; apply one-sided Hoeffding lower confidence bound to all n independent completed values",
                        },
                        "mean_observed_case_effect_pct": statistics.fmean(mean_observed) if mean_observed else None,
                        "mean_lower_bound_completed_effect_pct": statistics.fmean(mean_completed),
                        "mean_missing_fraction": statistics.fmean(missing_fracs),
                        "mean_observed_family_count": statistics.fmean(complete_counts),
                    })
                    index += 1

    null_rows = [row for row in scenario_rows if row["effect_condition"] == "null_at_margin"]
    return {
        "schema_version": SCHEMA_VERSION,
        "analysis_kind": "bounded_outcome_informative_missingness_sensitivity",
        "decision": "synthetic_missingness_sensitivity_diagnostics_no_empirical_claim",
        "claim_scope": "synthetic statistical method diagnostics only; no real lighting performance claim",
        "sampling_unit": "independent_scene_family",
        "estimand": "mean latent percentage energy saving across all attempted independent scene families",
        "configuration": {
            "replicates_per_scenario": replicates, "base_seed": seed, "alpha_one_sided": alpha,
            "margin_pct": margin_pct, "family_sd_pp": family_sd_pp,
            "outcome_bounds_pct": {"lower": lower_bound_pct, "upper": upper_bound_pct},
            "difficulty_effect": "slope_pp * (0.5 - observed_difficulty); observed difficulty ranges from 0 to 1",
            "distributions": list(DISTRIBUTIONS), "missingness_mechanisms": list(MISSINGNESS),
            "effect_conditions": [{"name": name, "mean_shift_in_family_sd": multiplier} for name, multiplier in effect_conditions],
            "sample_sizes": list(sample_sizes),
            "monte_carlo_interval": "individual 95% Wilson score interval for each rejection rate; not simultaneous",
        },
        "summary": {
            "scenario_count": len(scenario_rows),
            "null_scenario_count": len(null_rows),
            "max_null_rejection_complete_case": max(r["complete_case_student_t"]["rejection_rate"] for r in null_rows),
            "max_null_rejection_bounded_hoeffding": max(r["bounded_worst_case_hoeffding"]["rejection_rate"] for r in null_rows),
            "complete_case_null_scenarios_with_intervals_above_alpha": [r["scenario_id"] for r in null_rows if r["complete_case_student_t"]["rejection_rate_monte_carlo_95pct_wilson_ci"]["lower"] > alpha],
            "bounded_null_scenarios_with_intervals_above_alpha": [r["scenario_id"] for r in null_rows if r["bounded_worst_case_hoeffding"]["rejection_rate_monte_carlo_95pct_wilson_ci"]["lower"] > alpha],
        },
        "scenario_results": scenario_rows,
        "limitations": [
            "All data are synthetic bounded effect distributions and are not calibrated from real buildings.",
            "The complete-case Student-t method is not valid for a full-population mean when missingness is informative; it is included as a failure-mode reference.",
            "The Hoeffding bound is conservative and only valid when independent family effects are genuinely bounded by the preregistered limits.",
            "Missingness is a technical non-observation of an otherwise defined latent energy-saving effect; physical non-compliance and feasibility require separate outcomes and must not be encoded as missing energy.",
            "MAR and MNAR labels describe the declared data-generating mechanisms; the audit does not establish which mechanism applies in field data.",
            "Scenario-wise Monte Carlo intervals are individual rather than simultaneous; method selection needs a separately frozen confirmation suite.",
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--replicates", type=int, default=2500)
    parser.add_argument("--seed", type=int, default=20261028)
    parser.add_argument("--alpha", type=float, default=0.05)
    parser.add_argument("--margin-pct", type=float, default=5.0)
    parser.add_argument("--family-sd-pp", type=float, default=3.0)
    parser.add_argument("--lower-bound-pct", type=float, default=-15.0)
    parser.add_argument("--upper-bound-pct", type=float, default=25.0)
    parser.add_argument("--difficulty-slope-pp", type=float, default=3.0)
    parser.add_argument("--sample-sizes", default="10,20,40")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        sizes = tuple(int(value.strip()) for value in args.sample_sizes.split(",") if value.strip())
        report = simulate(replicates=args.replicates, seed=args.seed, alpha=args.alpha,
                          margin_pct=args.margin_pct, family_sd_pp=args.family_sd_pp,
                          lower_bound_pct=args.lower_bound_pct, upper_bound_pct=args.upper_bound_pct,
                          difficulty_slope_pp=args.difficulty_slope_pp, sample_sizes=sizes)
        report["source_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        report["t_critical_helper_source_sha256"] = hashlib.sha256(Path(planner_module.__file__).read_bytes()).hexdigest()
    except (MissingnessAuditError, ValueError, OverflowError) as exc:
        print(f"lighting MNAR sensitivity audit: FAIL: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(f"lighting MNAR sensitivity audit: {report['decision']}")
    print(f"scenarios={report['summary']['scenario_count']} replicates_per_scenario={args.replicates}")
    print(f"max_null_rejection_complete_case={report['summary']['max_null_rejection_complete_case']:.4f}")
    print(f"max_null_rejection_bounded_hoeffding={report['summary']['max_null_rejection_bounded_hoeffding']:.4f}")
    print(f"screening_complete_case_inflation_flags={len(report['summary']['complete_case_null_scenarios_with_intervals_above_alpha'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

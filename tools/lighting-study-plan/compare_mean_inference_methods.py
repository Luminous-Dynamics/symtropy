#!/usr/bin/env python3
"""Compare one-sided lower confidence bounds for mean savings under stress DGPs.

This is an exploratory *statistical-method* benchmark. All generated data are
synthetic and each row is one independent scene-family effect. A sign test is
implemented separately for a median estimand and is never ranked against
methods targeting the arithmetic mean.
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

from plan_independent_scene_families import _t_critical

SCHEMA_VERSION = "0.1.0"
Z_975 = 1.959963984540054
DISTRIBUTIONS = ("normal", "skewed_lognormal_right", "skewed_lognormal_left", "student_t")
SAMPLE_SIZES = (10, 20)
EFFECT_CONDITIONS = (("null_at_margin", 0.0), ("near_margin_half_sd", 0.5), ("target_one_and_half_sd", 1.5))


class MethodStudyError(ValueError):
    """Invalid method comparison inputs."""


def wilson_interval(successes: int, trials: int, z: float = Z_975) -> dict[str, float]:
    if trials < 1 or successes < 0 or successes > trials:
        raise MethodStudyError("Wilson interval requires 0 <= successes <= trials and trials > 0")
    p = successes / trials
    z2 = z * z
    denom = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / denom
    half = z * math.sqrt(p * (1.0 - p) / trials + z2 / (4.0 * trials * trials)) / denom
    return {"lower": max(0.0, min(p, center - half)), "upper": min(1.0, max(p, center + half))}


def _mean(values: list[float]) -> float:
    return statistics.fmean(values)


def _sample_sd(values: list[float]) -> float:
    return statistics.stdev(values)


def _quantile(values: list[float], probability: float) -> float:
    if not values or not (0.0 <= probability <= 1.0):
        raise MethodStudyError("quantile requires values and probability in [0,1]")
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = int(position)
    fraction = position - lower
    upper = min(lower + 1, len(ordered) - 1)
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def draw_standardized(rng: random.Random, distribution: str, student_t_df: int = 3) -> float:
    """Draw an approximately mean-zero/unit-variance stress-distribution value."""
    if distribution == "normal":
        return rng.gauss(0.0, 1.0)
    if distribution in ("skewed_lognormal_right", "skewed_lognormal_left"):
        shape = 0.8
        raw = math.exp(shape * rng.gauss(0.0, 1.0))
        expected = math.exp(shape * shape / 2.0)
        variance = (math.exp(shape * shape) - 1.0) * math.exp(shape * shape)
        standardized = (raw - expected) / math.sqrt(variance)
        return standardized if distribution == "skewed_lognormal_right" else -standardized
    if distribution == "student_t":
        if student_t_df <= 2:
            raise MethodStudyError("Student-t degrees of freedom must exceed 2 for finite variance")
        z = rng.gauss(0.0, 1.0)
        chi_square = rng.gammavariate(student_t_df / 2.0, 2.0)
        raw_t = z / math.sqrt(chi_square / student_t_df)
        return raw_t * math.sqrt((student_t_df - 2.0) / student_t_df)
    raise MethodStudyError(f"unsupported distribution {distribution!r}")


def _bootstrap_means(values: list[float], replicates: int, rng: random.Random) -> list[float]:
    n = len(values)
    result = []
    for _ in range(replicates):
        result.append(math.fsum(values[rng.randrange(n)] for _ in range(n)) / n)
    return result


def _bootstrap_t_statistics(values: list[float], replicates: int, rng: random.Random) -> list[float]:
    n = len(values)
    observed_mean = _mean(values)
    result = []
    for _ in range(replicates):
        sample = [values[rng.randrange(n)] for _ in range(n)]
        bootstrap_mean = _mean(sample)
        try:
            bootstrap_sd = _sample_sd(sample)
        except statistics.StatisticsError:
            bootstrap_sd = 0.0
        if not math.isfinite(bootstrap_sd) or bootstrap_sd <= 0.0:
            # Conservative lower-bound handling: the upper critical value is +inf.
            result.append(math.inf)
        else:
            result.append((bootstrap_mean - observed_mean) / (bootstrap_sd / math.sqrt(n)))
    return result


def mean_lower_bounds(values: list[float], *, margin: float, alpha: float,
                      bootstrap_replicates: int, percentile_rng: random.Random,
                      studentized_rng: random.Random) -> dict[str, float]:
    """Return lower bounds for the arithmetic mean; all methods target the same estimand."""
    if len(values) < 2 or any(not math.isfinite(x) for x in values):
        raise MethodStudyError("at least two finite independent family effects are required")
    if not (0.0 < alpha < 0.5) or bootstrap_replicates < 99:
        raise MethodStudyError("alpha must be in (0,0.5) and bootstrap_replicates must be >=99")
    observed_mean = _mean(values)
    sample_sd = _sample_sd(values)
    if not math.isfinite(sample_sd) or sample_sd <= 0.0:
        # Degenerate sample: fail closed instead of declaring superiority.
        return {"student_t": -math.inf, "percentile_bootstrap": -math.inf,
                "studentized_bootstrap_t": -math.inf}
    standard_error = sample_sd / math.sqrt(len(values))
    student_t_bound = observed_mean - _t_critical(alpha, len(values) - 1) * standard_error
    bootstrap_means = _bootstrap_means(values, bootstrap_replicates, percentile_rng)
    percentile_bound = _quantile(bootstrap_means, alpha)
    bootstrap_t = _bootstrap_t_statistics(values, bootstrap_replicates, studentized_rng)
    upper_t_critical = _quantile(bootstrap_t, 1.0 - alpha)
    studentized_bound = -math.inf if math.isinf(upper_t_critical) else observed_mean - upper_t_critical * standard_error
    return {"student_t": student_t_bound, "percentile_bootstrap": percentile_bound,
            "studentized_bootstrap_t": studentized_bound}


def sign_test_median_p_value(values: list[float], margin: float) -> float:
    """One-sided exact sign-test p-value for median(effect) > margin, not for the mean."""
    if not values or any(not math.isfinite(x) for x in values) or not math.isfinite(margin):
        raise MethodStudyError("sign test requires finite values, a finite margin and a nonempty sample")
    positive = sum(value > margin for value in values)
    n = len(values)
    # Ties count as non-positive, making the upper-tail test conservative at the boundary.
    return min(1.0, math.fsum(math.comb(n, i) for i in range(positive, n + 1)) / (2 ** n))


def simulate(*, replicates: int = 500, bootstrap_replicates: int = 199,
             base_seed: int = 20261028, alpha: float = 0.05,
             margin_pct: float = 5.0, family_sd_pp: float = 1.0,
             near_margin_sd: float = 0.5, target_sd: float = 1.5,
             student_t_df: int = 3) -> dict[str, Any]:
    if replicates < 200 or replicates > 1_000_000:
        raise MethodStudyError("replicates must be in [200,1000000]")
    if bootstrap_replicates < 99 or bootstrap_replicates > 10000:
        raise MethodStudyError("bootstrap_replicates must be in [99,10000]")
    if not isinstance(base_seed, int) or isinstance(base_seed, bool) or base_seed < 0:
        raise MethodStudyError("base_seed must be a non-negative integer")
    if not (0.0 < alpha < 0.5):
        raise MethodStudyError("alpha must be in (0,0.5)")
    if not math.isfinite(margin_pct) or not math.isfinite(family_sd_pp) or family_sd_pp <= 0.0:
        raise MethodStudyError("margin must be finite and family_sd_pp must be finite and positive")
    if not math.isfinite(near_margin_sd) or not math.isfinite(target_sd) or near_margin_sd < 0 or target_sd <= near_margin_sd:
        raise MethodStudyError("effect multipliers must be finite, nonnegative and target_sd > near_margin_sd")
    if student_t_df <= 2:
        raise MethodStudyError("Student-t degrees of freedom must exceed 2")

    scenario_results: list[dict[str, Any]] = []
    scenario_index = 0
    method_names = ("student_t", "percentile_bootstrap", "studentized_bootstrap_t")
    for n in SAMPLE_SIZES:
        for distribution in DISTRIBUTIONS:
            for condition, multiplier in EFFECT_CONDITIONS:
                true_mean = margin_pct + multiplier * family_sd_pp
                scenario_seed = base_seed + scenario_index * 1009
                data_rng = random.Random(scenario_seed)
                counts = {name: {"reject": 0, "covers_mean": 0} for name in method_names}
                for rep in range(replicates):
                    values = [true_mean + family_sd_pp * draw_standardized(data_rng, distribution, student_t_df)
                              for _ in range(n)]
                    # Separate deterministic streams prevent bootstrap method order from changing results.
                    seed_base = scenario_seed * 1_000_003 + rep * 4 + 1
                    bounds = mean_lower_bounds(
                        values, margin=margin_pct, alpha=alpha,
                        bootstrap_replicates=bootstrap_replicates,
                        percentile_rng=random.Random(seed_base + 1),
                        studentized_rng=random.Random(seed_base + 2))
                    for name, lower_bound in bounds.items():
                        counts[name]["reject"] += int(lower_bound > margin_pct)
                        counts[name]["covers_mean"] += int(lower_bound <= true_mean)
                method_results = {}
                for name in method_names:
                    reject = counts[name]["reject"]
                    covers = counts[name]["covers_mean"]
                    method_results[name] = {
                        "estimand": "arithmetic_mean_of_independent_scene_family_savings_pct",
                        "procedure": {
                            "student_t": "one-sided Student-t lower confidence bound for mean",
                            "percentile_bootstrap": "one-sided percentile-bootstrap lower bound for mean",
                            "studentized_bootstrap_t": "one-sided studentized-bootstrap-t lower bound for mean"
                        }[name],
                        "rejection_rate": reject / replicates,
                        "rejection_rate_monte_carlo_95pct_wilson_ci": wilson_interval(reject, replicates),
                        "one_sided_mean_lower_bound_coverage_rate": covers / replicates,
                        "mean_lower_bound_coverage_monte_carlo_95pct_wilson_ci": wilson_interval(covers, replicates),
                    }
                scenario_results.append({
                    "scenario_id": f"n{n}__{distribution}__{condition}",
                    "independent_scene_families": n,
                    "distribution": distribution,
                    "student_t_degrees_freedom": student_t_df if distribution == "student_t" else None,
                    "condition": condition,
                    "true_mean_savings_pct": true_mean,
                    "superiority_margin_pct": margin_pct,
                    "family_effect_sd_percentage_points": family_sd_pp,
                    "alpha_per_method": alpha,
                    "replicates": replicates,
                    "bootstrap_replicates_per_method_per_sample": bootstrap_replicates,
                    "seed": scenario_seed,
                    "methods": method_results,
                })
                scenario_index += 1

    nulls = [row for row in scenario_results if row["condition"] == "null_at_margin"]
    return {
        "schema_version": SCHEMA_VERSION,
        "analysis_kind": "lighting_mean_inference_method_stress_benchmark",
        "decision": "synthetic_inference_method_diagnostics_no_empirical_claim",
        "claim_scope": "exploratory statistical-method simulation only; not real lighting performance",
        "sampling_unit": "independent_scene_family",
        "primary_estimand": "arithmetic_mean_savings_percentage_across_independent_scene_families",
        "methods_compared": list(method_names),
        "methods_are_not_declared_winners": True,
        "multiple_method_selection_policy": "exploratory stress grid only; any method choice requires a separately preregistered held-out DGM suite",
        "median_method_policy": "the exact sign-test helper targets the median and is deliberately excluded from arithmetic-mean method rankings",
        "configuration": {
            "replicates_per_scenario": replicates, "bootstrap_replicates": bootstrap_replicates,
            "base_seed": base_seed, "alpha_per_method": alpha, "margin_pct": margin_pct,
            "family_sd_pp": family_sd_pp, "near_margin_sd_multiplier": near_margin_sd,
            "target_sd_multiplier": target_sd, "student_t_degrees_freedom": student_t_df,
            "sample_sizes": list(SAMPLE_SIZES), "distributions": list(DISTRIBUTIONS),
            "conditions": [{"name": name, "mean_above_margin_in_family_sds": mult}
                           for name, mult in EFFECT_CONDITIONS],
            "bootstrap_quantile_rule": "linear interpolation at index (B-1)*p",
            "monte_carlo_interval": "individual 95% Wilson score intervals for every scenario-method proportion",
        },
        "summary": {
            "scenario_count": len(scenario_results),
            "null_scenario_count": len(nulls),
            "max_null_rejection_rate_by_method": {
                method: max(row["methods"][method]["rejection_rate"] for row in nulls)
                for method in method_names
            },
            "null_scenarios_with_rejection_interval_above_alpha": {
                method: [row["scenario_id"] for row in nulls
                         if row["methods"][method]["rejection_rate_monte_carlo_95pct_wilson_ci"]["lower"] > alpha]
                for method in method_names
            },
            "target_power_by_method": {
                method: {row["scenario_id"]: row["methods"][method]["rejection_rate"]
                         for row in scenario_results if row["condition"] == "target_one_and_half_sd"}
                for method in method_names
            },
        },
        "scenario_results": scenario_results,
        "limitations": [
            "All scenarios are parametric stress distributions, not empirical descriptions of lighting scenes.",
            "Percentile bootstrap and studentized bootstrap-t are approximations; neither is guaranteed to control type-I error at small n under skew.",
            "The reported per-scenario Monte Carlo intervals are individual, not simultaneous across all methods and scenarios.",
            "The simulator treats the true distribution scale as known only for generating scenarios; inferential methods do not use that true value.",
            "Pilot-variance uncertainty, outcome-dependent missingness, randomized-assignment inference and physical measurement uncertainty remain outside this version.",
            "The method-selection decision must be checked on a separate preregistered held-out scenario family; do not select and certify a procedure on the same stress matrix.",
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--replicates", type=int, default=500)
    parser.add_argument("--bootstrap-replicates", type=int, default=199)
    parser.add_argument("--seed", type=int, default=20261028)
    parser.add_argument("--alpha", type=float, default=0.05, help="per-method one-sided test level; pass the multiplicity-adjusted level where applicable")
    parser.add_argument("--margin-pct", type=float, default=5.0)
    parser.add_argument("--family-sd-pp", type=float, default=1.0)
    parser.add_argument("--student-t-df", type=int, default=3)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        report = simulate(replicates=args.replicates, bootstrap_replicates=args.bootstrap_replicates,
                          base_seed=args.seed, alpha=args.alpha, margin_pct=args.margin_pct,
                          family_sd_pp=args.family_sd_pp, student_t_df=args.student_t_df)
    except (MethodStudyError, ValueError, OverflowError) as exc:
        print(f"lighting mean-inference study: FAIL: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n"
    report["simulation_source_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    rendered = json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(f"lighting mean-inference study: {report['decision']}")
    print(f"scenarios={report['summary']['scenario_count']} null_scenarios={report['summary']['null_scenario_count']}")
    print("maximum null rejection by method: " + json.dumps(report["summary"]["max_null_rejection_rate_by_method"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

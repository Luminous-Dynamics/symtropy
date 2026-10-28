#!/usr/bin/env python3
"""Frozen mean-inference calibration bake-off for independent lighting scene families.

Synthetic-only. Compares Student-t, percentile bootstrap, and studentized bootstrap-t
lower confidence bounds. The development method implementation is imported unchanged;
this script owns a separately frozen DGP grid and multiplicity-adjusted Monte Carlo gates.
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
from typing import Any, Sequence

import compare_mean_inference_methods as methods
try:
    import numpy as np
except ImportError:  # Optional acceleration; the standard-library fallback remains available.
    np = None

SCHEMA_VERSION = "0.1.0"
METHODS = ("student_t", "percentile_bootstrap", "studentized_bootstrap_t")
DISTRIBUTIONS = ("normal", "left_skew_two_point", "right_skew_two_point", "symmetric_rare_extremes", "student_t3")
NULL_SAMPLE_SIZES = (12, 24)
NULL_DISTRIBUTIONS_AT_N24 = ("normal", "left_skew_two_point", "student_t3")
TARGET_SAMPLE_SIZE = 24
DEFAULT_NULL_REPLICATES = 5000
DEFAULT_TARGET_REPLICATES = 1000
DEFAULT_BOOTSTRAP_REPLICATES = 99
SIMULTANEOUS_ALPHA = 0.05
FALSE_SUPERIORITY_LIMIT = 0.06
COVERAGE_FLOOR = 0.94


class CalibrationError(ValueError):
    """Invalid calibration configuration."""


@lru_cache(maxsize=64)
def _cached_t_critical(alpha: float, degrees_freedom: int) -> float:
    """Cache an expensive numerical t-quantile reused in every Monte Carlo replicate."""
    return methods._t_critical(alpha, degrees_freedom)


def _quantile_with_infinity(values: Any, probability: float) -> float:
    """Linear quantile matching the reference helper's conservative infinite-value rule."""
    ordered = np.sort(values) if np is not None else sorted(values)
    position = (len(ordered) - 1) * probability
    lower = int(position)
    fraction = position - lower
    upper = min(lower + 1, len(ordered) - 1)
    left, right = float(ordered[lower]), float(ordered[upper])
    if fraction == 0.0:
        return left
    if math.isinf(left):
        return left
    if math.isinf(right):
        return right
    return left * (1.0 - fraction) + right * fraction


def draw_standardized(rng: random.Random, distribution: str) -> float:
    """Draw zero-mean, unit-variance stress outcomes, except for finite simulation noise."""
    if distribution == "normal":
        return rng.gauss(0.0, 1.0)
    if distribution == "left_skew_two_point":
        return -3.0 if rng.random() < 0.1 else 1.0 / 3.0
    if distribution == "right_skew_two_point":
        return 3.0 if rng.random() < 0.1 else -1.0 / 3.0
    if distribution == "symmetric_rare_extremes":
        u = rng.random()
        if u < 0.05:
            return -math.sqrt(10.0)
        if u >= 0.95:
            return math.sqrt(10.0)
        return 0.0
    if distribution == "student_t3":
        z = rng.gauss(0.0, 1.0)
        chi_square = rng.gammavariate(1.5, 2.0)
        return (z / math.sqrt(chi_square / 3.0)) / math.sqrt(3.0)
    raise CalibrationError(f"unknown distribution {distribution!r}")


def wilson_one_sided(successes: int, trials: int, z: float) -> dict[str, float]:
    """Wilson score interval using a one-sided critical value; used with Bonferroni z."""
    if trials < 1 or successes < 0 or successes > trials:
        raise CalibrationError("Wilson interval requires 0 <= successes <= trials and trials > 0")
    p = successes / trials
    z2 = z * z
    denom = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / denom
    half = z * math.sqrt((p * (1.0 - p) / trials) + z2 / (4.0 * trials * trials)) / denom
    return {"lower": max(0.0, center - half), "upper": min(1.0, center + half)}


def simultaneous_z(alpha: float, family_size: int) -> float:
    if not (0.0 < alpha < 0.5) or family_size < 1:
        raise CalibrationError("alpha must be in (0,0.5) and family_size must be positive")
    return statistics.NormalDist().inv_cdf(1.0 - alpha / family_size)


def _source_sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def mean_lower_bounds_fast(values: Sequence[float], *, margin: float, alpha: float,
                           bootstrap_replicates: int, percentile_rng: random.Random,
                           studentized_rng: random.Random) -> dict[str, float]:
    """Algorithmically equivalent optimized bootstrap bounds for the frozen comparator.

    The existing comparator is the reference implementation. This loop avoids allocating
    a Python list and calling statistics.stdev for every inner bootstrap draw, making the
    5,000-replicate confirmation suite tractable. Tests compare results to the reference.
    """
    n = len(values)
    if n < 2 or any(not math.isfinite(x) for x in values):
        raise CalibrationError("at least two finite family effects are required")
    if not (0.0 < alpha < 0.5) or bootstrap_replicates < 99:
        raise CalibrationError("alpha must be in (0,0.5) and bootstrap replicates >=99")
    observed_mean = statistics.fmean(values)
    observed_sd = statistics.stdev(values)
    if not math.isfinite(observed_sd) or observed_sd <= 0.0:
        return {name: -math.inf for name in METHODS}
    standard_error = observed_sd / math.sqrt(n)
    student_bound = observed_mean - _cached_t_critical(alpha, n - 1) * standard_error
    means = []
    for _ in range(bootstrap_replicates):
        total = 0.0
        for _ in range(n):
            total += values[percentile_rng.randrange(n)]
        means.append(total / n)
    percentile_bound = methods._quantile(means, alpha)
    t_stats = []
    root_n = math.sqrt(n)
    for _ in range(bootstrap_replicates):
        total = 0.0
        sum_squares = 0.0
        for _ in range(n):
            value = values[studentized_rng.randrange(n)]
            total += value
            sum_squares += value * value
        boot_mean = total / n
        variance_numerator = sum_squares - (total * total / n)
        boot_variance = max(0.0, variance_numerator / (n - 1))
        boot_sd = math.sqrt(boot_variance)
        if not math.isfinite(boot_sd) or boot_sd <= 0.0:
            t_stats.append(math.inf)
        else:
            t_stats.append((boot_mean - observed_mean) / (boot_sd / root_n))
    upper_t_critical = _quantile_with_infinity(t_stats, 1.0 - alpha)
    studentized_bound = -math.inf if math.isinf(upper_t_critical) else observed_mean - upper_t_critical * standard_error
    return {"student_t": student_bound, "percentile_bootstrap": percentile_bound,
            "studentized_bootstrap_t": studentized_bound}


def _resolve_backend(backend: str) -> str:
    if backend not in ("auto", "stdlib", "numpy"):
        raise CalibrationError("backend must be one of auto, stdlib, numpy")
    if backend == "numpy" and np is None:
        raise CalibrationError("numpy backend requested but NumPy is unavailable")
    if backend == "auto":
        return "numpy" if np is not None else "stdlib"
    return backend


def mean_lower_bounds_numpy(values: Sequence[float], *, margin: float, alpha: float,
                            bootstrap_replicates: int, percentile_rng: Any,
                            studentized_rng: Any) -> dict[str, float]:
    """Vectorized bootstrap implementation; statistically matches the reference procedure."""
    if np is None:
        raise CalibrationError("numpy backend requested but NumPy is unavailable")
    if len(values) < 2 or any(not math.isfinite(x) for x in values):
        raise CalibrationError("at least two finite family effects are required")
    if not (0.0 < alpha < 0.5) or bootstrap_replicates < 99:
        raise CalibrationError("alpha must be in (0,0.5) and bootstrap replicates >=99")
    array = np.asarray(values, dtype=np.float64)
    n = int(array.size)
    observed_mean = float(np.mean(array))
    observed_sd = float(np.std(array, ddof=1))
    if not math.isfinite(observed_sd) or observed_sd <= 0.0:
        return {name: -math.inf for name in METHODS}
    se = observed_sd / math.sqrt(n)
    student = observed_mean - _cached_t_critical(alpha, n - 1) * se
    indices = percentile_rng.integers(0, n, size=(bootstrap_replicates, n))
    means = array[indices].mean(axis=1)
    percentile = float(np.quantile(means, alpha, method="linear"))
    t_indices = studentized_rng.integers(0, n, size=(bootstrap_replicates, n))
    samples = array[t_indices]
    sample_means = samples.mean(axis=1)
    sample_sds = samples.std(axis=1, ddof=1)
    t_values = np.full(bootstrap_replicates, np.inf, dtype=np.float64)
    good = np.isfinite(sample_sds) & (sample_sds > 0.0)
    t_values[good] = (sample_means[good] - observed_mean) / (sample_sds[good] / math.sqrt(n))
    upper_critical = _quantile_with_infinity(t_values, 1.0 - alpha)
    studentized = -math.inf if math.isinf(upper_critical) else observed_mean - upper_critical * se
    return {"student_t": student, "percentile_bootstrap": percentile,
            "studentized_bootstrap_t": studentized}


def _run_cell(*, n: int, distribution: str, mean: float, margin: float,
              family_sd_pp: float, outer_replicates: int, bootstrap_replicates: int,
              seed: int, backend: str) -> dict[str, Any]:
    rng = random.Random(seed)
    if backend == "numpy":
        percentile_rng = np.random.default_rng(seed * 1_000_003 + 1)
        studentized_rng = np.random.default_rng(seed * 1_000_003 + 2)
    else:
        percentile_rng = random.Random(seed * 1_000_003 + 1)
        studentized_rng = random.Random(seed * 1_000_003 + 2)
    reject_counts = {name: 0 for name in METHODS}
    covered_counts = {name: 0 for name in METHODS}
    lower_bound_sums = {name: 0.0 for name in METHODS}
    method_failure_counts = {name: 0 for name in METHODS}
    unresolved_counts = {name: 0 for name in METHODS}
    finite_bound_counts = {name: 0 for name in METHODS}
    for _rep in range(outer_replicates):
        values = [mean + family_sd_pp * draw_standardized(rng, distribution) for _ in range(n)]
        try:
            bound_fn = mean_lower_bounds_numpy if backend == "numpy" else mean_lower_bounds_fast
            bounds = bound_fn(
                values, margin=margin, alpha=0.05, bootstrap_replicates=bootstrap_replicates,
                percentile_rng=percentile_rng, studentized_rng=studentized_rng,
            )
        except (ValueError, OverflowError, ArithmeticError):
            for name in METHODS:
                method_failure_counts[name] += 1
            continue
        for name in METHODS:
            lower = bounds[name]
            if lower == -math.inf:
                # A conservative non-finite lower bound is an unresolved decision, not a pass.
                unresolved_counts[name] += 1
                continue
            if not math.isfinite(lower):
                method_failure_counts[name] += 1
                continue
            finite_bound_counts[name] += 1
            lower_bound_sums[name] += lower
            reject_counts[name] += int(lower > margin)
            covered_counts[name] += int(lower <= mean)
    rows: dict[str, Any] = {}
    for name in METHODS:
        finite = finite_bound_counts[name]
        rows[name] = {
            "outer_replicates": outer_replicates,
            "valid_finite_bound_replicates": finite,
            "unresolved_lower_bound_replicates": unresolved_counts[name],
            "unresolved_lower_bound_rate": unresolved_counts[name] / outer_replicates,
            "failed_or_nonfinite_replicates": method_failure_counts[name],
            "failure_rate": method_failure_counts[name] / outer_replicates,
            "mean_lower_bound_pct_among_finite": lower_bound_sums[name] / finite if finite else None,
            "rejection_rate_unconditional": reject_counts[name] / outer_replicates,
            "false_superiority_rate": reject_counts[name] / outer_replicates if mean == margin else None,
            "conditional_false_superiority_rate_among_finite": reject_counts[name] / finite if finite and mean == margin else None,
            "power": reject_counts[name] / outer_replicates if mean > margin else None,
            "finite_interval_coverage_rate": covered_counts[name] / finite if finite else None,
            "finite_interval_coverage_count": covered_counts[name],
            "rejection_count": reject_counts[name],
            "covered_count": covered_counts[name],
        }
    return {
        "scenario_id": f"n{n}__{distribution}__{'null_at_margin' if mean == margin else 'target_1_5_sd_above_margin'}",
        "independent_scene_families": n, "distribution": distribution,
        "condition": "null_at_margin" if mean == margin else "target_1_5_sd_above_margin",
        "true_mean_pct": mean, "margin_pct": margin, "family_sd_pp": family_sd_pp,
        "outer_replicates": outer_replicates, "bootstrap_replicates": bootstrap_replicates,
        "seed": seed, "methods": rows,
    }


def calibrate(*, null_replicates: int = DEFAULT_NULL_REPLICATES,
              target_replicates: int = DEFAULT_TARGET_REPLICATES,
              bootstrap_replicates: int = DEFAULT_BOOTSTRAP_REPLICATES,
              seed: int = 20261029, margin_pct: float = 5.0,
              family_sd_pp: float = 3.0, backend: str = "auto") -> dict[str, Any]:
    if not isinstance(null_replicates, int) or isinstance(null_replicates, bool) or null_replicates < 5000:
        raise CalibrationError("null_replicates must be an integer >= 5000 for the confirmation suite")
    if not isinstance(target_replicates, int) or isinstance(target_replicates, bool) or target_replicates < 500:
        raise CalibrationError("target_replicates must be an integer >= 500")
    if not isinstance(bootstrap_replicates, int) or isinstance(bootstrap_replicates, bool) or not 99 <= bootstrap_replicates <= 2000:
        raise CalibrationError("bootstrap_replicates must be in [99,2000]")
    if not isinstance(seed, int) or isinstance(seed, bool) or seed < 0:
        raise CalibrationError("seed must be a non-negative integer")
    if not math.isfinite(margin_pct) or not math.isfinite(family_sd_pp) or family_sd_pp <= 0:
        raise CalibrationError("margin must be finite and family_sd_pp must be finite and positive")

    resolved_backend = _resolve_backend(backend)
    rows: list[dict[str, Any]] = []
    index = 0
    for n in NULL_SAMPLE_SIZES:
        applicable_distributions = DISTRIBUTIONS if n == 12 else NULL_DISTRIBUTIONS_AT_N24
        for distribution in applicable_distributions:
            rows.append(_run_cell(n=n, distribution=distribution, mean=margin_pct, margin=margin_pct,
                                  family_sd_pp=family_sd_pp, outer_replicates=null_replicates,
                                  bootstrap_replicates=bootstrap_replicates, seed=seed + index * 1009,
                                  backend=resolved_backend))
            index += 1
    for distribution in DISTRIBUTIONS:
        rows.append(_run_cell(n=TARGET_SAMPLE_SIZE, distribution=distribution,
                              mean=margin_pct + 1.5 * family_sd_pp, margin=margin_pct,
                              family_sd_pp=family_sd_pp, outer_replicates=target_replicates,
                              bootstrap_replicates=bootstrap_replicates, seed=seed + index * 1009,
                              backend=resolved_backend))
        index += 1

    null_rows = [r for r in rows if r["condition"] == "null_at_margin"]
    method_count = len(METHODS)
    rejection_family_size = len(null_rows) * method_count
    coverage_family_size = len(rows) * method_count
    z_reject = simultaneous_z(SIMULTANEOUS_ALPHA, rejection_family_size)
    z_coverage = simultaneous_z(SIMULTANEOUS_ALPHA, coverage_family_size)
    gate_by_method: dict[str, Any] = {}
    by_method: dict[str, Any] = {}
    for name in METHODS:
        null_upper_bounds = []
        target_power = []
        coverage_lower_bounds = []
        unresolved_rates = []
        failure_rates = []
        for row in rows:
            m = row["methods"][name]
            if row["condition"] == "null_at_margin":
                interval = wilson_one_sided(m["rejection_count"], m["outer_replicates"], z_reject)
                m["false_superiority_rate_simultaneous_95pct_mc_interval"] = interval
                null_upper_bounds.append(interval["upper"])
            else:
                target_power.append(m["power"])
                m["power_simultaneous_95pct_mc_interval"] = wilson_one_sided(
                    m["rejection_count"], m["outer_replicates"], z_coverage)
            finite = m["valid_finite_bound_replicates"]
            coverage_interval = wilson_one_sided(m["finite_interval_coverage_count"], finite, z_coverage) if finite else None
            m["finite_interval_coverage_simultaneous_95pct_mc_interval"] = coverage_interval
            if coverage_interval:
                coverage_lower_bounds.append(coverage_interval["lower"])
            unresolved_rates.append(m["unresolved_lower_bound_rate"])
            failure_rates.append(m["failure_rate"])
        max_upper = max(null_upper_bounds) if null_upper_bounds else None
        min_coverage_lcb = min(coverage_lower_bounds) if coverage_lower_bounds else None
        max_unresolved = max(unresolved_rates) if unresolved_rates else None
        max_failure = max(failure_rates) if failure_rates else None
        error_gate = max_upper is not None and max_upper <= FALSE_SUPERIORITY_LIMIT
        coverage_gate = min_coverage_lcb is not None and min_coverage_lcb >= COVERAGE_FLOOR
        unresolved_gate = max_unresolved is not None and max_unresolved <= 0.01
        failure_gate = max_failure is not None and max_failure <= 0.01
        by_method[name] = {
            "max_null_false_superiority_upper_simultaneous_bound": max_upper,
            "min_finite_interval_coverage_lower_simultaneous_bound": min_coverage_lcb,
            "mean_target_power": statistics.fmean(target_power) if target_power else None,
            "min_target_power": min(target_power) if target_power else None,
            "max_unresolved_lower_bound_rate": max_unresolved,
            "max_failed_or_nonfinite_rate": max_failure,
        }
        gate_by_method[name] = {
            "false_superiority_gate_pass": error_gate,
            "coverage_gate_pass": coverage_gate,
            "unresolved_lower_bound_gate_pass": unresolved_gate,
            "failed_or_nonfinite_gate_pass": failure_gate,
            "all_gates_pass": bool(error_gate and coverage_gate and unresolved_gate and failure_gate),
            "false_superiority_limit": FALSE_SUPERIORITY_LIMIT,
            "coverage_floor": COVERAGE_FLOOR,
            "max_unresolved_or_failure_rate": 0.01,
            "explanation": "Rejection/power rates use all attempted replicates; finite-interval coverage is conditional on a finite bound, while unresolved and failed rates are reported and gated separately.",
        }
    import compare_mean_inference_methods
    source_path = Path(__file__)
    dependency_path = Path(compare_mean_inference_methods.__file__)
    report = {
        "schema_version": SCHEMA_VERSION,
        "decision": "synthetic_confirmatory_inference_calibration_diagnostics_no_empirical_claim",
        "claim_scope": "synthetic statistical operating-characteristic calibration only; no measured lighting, Radiance, calibrated photometry or industry comparison",
        "sampling_unit": "independent_scene_family",
        "estimand": "arithmetic mean energy-saving effect across independent scene families",
        "hypotheses": {"null": "mean <= margin", "decision": "reject only when one-sided lower confidence bound exceeds margin"},
        "methods": list(METHODS),
        "frozen_design": {
            "design_tag": "LIGHTING-MEAN-INFERENCE-CALIBRATION-v0.1-confirmation",
            "seed": seed, "margin_pct": margin_pct, "family_sd_pp": family_sd_pp,
            "null_replicates_per_scenario": null_replicates,
            "target_replicates_per_scenario": target_replicates,
            "bootstrap_replicates_per_dataset": bootstrap_replicates,
            "bootstrap_backend": resolved_backend,
            "numpy_version": getattr(np, "__version__", None) if resolved_backend == "numpy" else None,
            "null_sample_sizes": list(NULL_SAMPLE_SIZES),
            "null_distributions_at_n24": list(NULL_DISTRIBUTIONS_AT_N24),
            "target_sample_size": TARGET_SAMPLE_SIZE, "target_distributions": list(DISTRIBUTIONS),
            "distributions": list(DISTRIBUTIONS),
            "target_effect": "margin + 1.5 * family SD; fixed before any method outcome is examined",
            "monte_carlo_control": {
                "familywise_alpha": SIMULTANEOUS_ALPHA,
                "method_scenario_tests_for_false_superiority": rejection_family_size,
                "method_scenario_coverage_intervals": coverage_family_size,
                "simultaneous_adjustment": "Bonferroni over one-sided Wilson score bounds using NormalDist critical values",
                "false_superiority_upper_bound_gate": FALSE_SUPERIORITY_LIMIT,
                "lower_bound_coverage_lower_bound_gate": COVERAGE_FLOOR,
                "max_unresolved_bound_or_failure_rate": 0.01,
            },
        },
        "scenario_count": len(rows),
        "summary_by_method": by_method,
        "gate_by_method": gate_by_method,
        "scenario_results": rows,
        "source_sha256": _source_sha(source_path),
        "method_comparator_source_sha256": _source_sha(dependency_path),
        "execution_provenance": {
            "method_comparator_source_path": dependency_path.name,
            "method_comparator_source_kind": "local_support_mirror" if "Local support mirror" in dependency_path.read_text(encoding="utf-8")[:500] else "repository_module",
            "accelerated_backend": resolved_backend,
            "numerical_backend_version": getattr(np, "__version__", None) if resolved_backend == "numpy" else None,
            "note": "The downloadable standalone package uses a source-mirrored comparator helper. When run in the repository, this script imports the repository's compare_mean_inference_methods.py and records that exact source hash.",
        },
        "limitations": [
            "Data are synthetic and do not establish optical or energy performance.",
            "Scenario-wise simultaneous bounds use Bonferroni and may be conservative; reported rates still have Monte Carlo uncertainty.",
            "The bootstrap procedures are approximations and cannot be assumed to control type-I error for all small-n distributions.",
            "Only complete outcomes are included here to isolate mean-inference calibration; missingness and MNAR remain separate benchmark dimensions.",
            "The gates are predeclared operating criteria, not evidence of SOTA; method pass/fail applies only to this declared DGP family.",
        ],
        "methods_are_not_declared_winners": True,
    }
    return report


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--null-replicates", type=int, default=DEFAULT_NULL_REPLICATES)
    parser.add_argument("--target-replicates", type=int, default=DEFAULT_TARGET_REPLICATES)
    parser.add_argument("--bootstrap-replicates", type=int, default=DEFAULT_BOOTSTRAP_REPLICATES)
    parser.add_argument("--seed", type=int, default=20261029)
    parser.add_argument("--margin-pct", type=float, default=5.0)
    parser.add_argument("--family-sd-pp", type=float, default=3.0)
    parser.add_argument("--backend", choices=("auto", "stdlib", "numpy"), default="auto")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        report = calibrate(null_replicates=args.null_replicates, target_replicates=args.target_replicates,
                           bootstrap_replicates=args.bootstrap_replicates, seed=args.seed,
                           margin_pct=args.margin_pct, family_sd_pp=args.family_sd_pp,
                           backend=args.backend)
    except (CalibrationError, ValueError, OverflowError, ArithmeticError) as exc:
        print(f"lighting inference calibration: FAIL: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(f"lighting inference calibration: {report['decision']}")
    print("summary=" + json.dumps(report["summary_by_method"], sort_keys=True))
    print("gates=" + json.dumps(report["gate_by_method"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

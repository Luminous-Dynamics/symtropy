#!/usr/bin/env python3
"""Fitted MAR weighting and interval-aware MNAR sensitivity for lighting studies.

Standard-library-only, synthetic-method diagnostic. Independent scene families are
sampling units. No empirical lighting or energy-saving claim is made.
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
from typing import Any, Sequence

SCHEMA_VERSION = "0.1.0"
Z_ONE_SIDED_95 = 1.6448536269514722
Z_TWO_SIDED_95 = 1.959963984540054
POSITIVITY_FLOOR = 0.05


class AnalysisError(ValueError):
    """Invalid input or failed model fit; callers must not quietly treat this as pass."""


def logistic(z: float) -> float:
    if z >= 30.0:
        return 1.0 - 1e-13
    if z <= -30.0:
        return 1e-13
    return 1.0 / (1.0 + math.exp(-z))


def wilson_interval(successes: int, trials: int, z: float = Z_TWO_SIDED_95) -> dict[str, float]:
    if trials < 1 or not (0 <= successes <= trials):
        raise AnalysisError("Wilson interval requires 0 <= successes <= trials and trials > 0")
    p = successes / trials
    z2 = z * z
    den = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / den
    half = z * math.sqrt(p * (1.0 - p) / trials + z2 / (4.0 * trials * trials)) / den
    return {"lower": max(0.0, center - half), "upper": min(1.0, center + half)}


def _validate_data(x: Sequence[float], outcomes: Sequence[float | None]) -> tuple[list[float], list[float | None]]:
    if len(x) != len(outcomes) or len(x) < 5:
        raise AnalysisError("x and outcomes must have equal length >= 5")
    xx = [float(v) for v in x]
    yy = [None if v is None else float(v) for v in outcomes]
    if any(not math.isfinite(v) for v in xx):
        raise AnalysisError("covariates must be finite")
    if any(v is not None and not math.isfinite(v) for v in yy):
        raise AnalysisError("observed outcomes must be finite")
    observed = sum(v is not None for v in yy)
    if observed < 3 or observed == len(yy):
        # Complete data are handled explicitly by the estimator; this guard is adjusted there.
        if observed == 0:
            raise AnalysisError("no observed outcomes")
    return xx, yy


def fit_logistic_response(x: Sequence[float], response: Sequence[int], *, max_iter: int = 100) -> dict[str, Any]:
    """Unpenalized 1-covariate logistic MLE, with convergence and separation checks.

    Response is 1 when the outcome was observed. The covariate is standardized for
    numerical stability. Perfect/near separation is a failure, not silently regularized.
    """
    if len(x) != len(response) or len(x) < 5 or any(v not in (0, 1) for v in response):
        raise AnalysisError("response model needs >=5 aligned binary observations")
    if sum(response) == 0 or sum(response) == len(response):
        raise AnalysisError("response model requires both observed and missing outcomes")
    mean_x = statistics.fmean(x)
    sd_x = statistics.pstdev(x)
    if sd_x < 1e-12:
        raise AnalysisError("response covariate has zero variance")
    z = [(v - mean_x) / sd_x for v in x]
    b0 = math.log(sum(response) / (len(response) - sum(response)))
    b1 = 0.0
    converged = False
    for iteration in range(1, max_iter + 1):
        ps = [logistic(b0 + b1 * zz) for zz in z]
        g0 = sum(r - p for r, p in zip(response, ps))
        g1 = sum((r - p) * zz for r, p, zz in zip(response, ps, z))
        h00 = sum(p * (1.0 - p) for p in ps)
        h01 = sum(p * (1.0 - p) * zz for p, zz in zip(ps, z))
        h11 = sum(p * (1.0 - p) * zz * zz for p, zz in zip(ps, z))
        det = h00 * h11 - h01 * h01
        if det <= 1e-14 or not math.isfinite(det):
            raise AnalysisError("response model Hessian is singular (possible separation)")
        step0 = (h11 * g0 - h01 * g1) / det
        step1 = (-h01 * g0 + h00 * g1) / det
        if not (math.isfinite(step0) and math.isfinite(step1)):
            raise AnalysisError("response model produced non-finite Newton step")
        # Damped Newton step prevents an uncontrolled jump into separation.
        scale = 1.0
        while scale > 1.0 / 128.0 and max(abs(scale * step0), abs(scale * step1)) > 5.0:
            scale *= 0.5
        b0 += scale * step0
        b1 += scale * step1
        if max(abs(scale * step0), abs(scale * step1)) < 1e-8:
            converged = True
            break
        if max(abs(b0), abs(b1)) > 25.0:
            raise AnalysisError("response model coefficients diverged (possible separation)")
    if not converged:
        raise AnalysisError("response model did not converge")
    ps = [logistic(b0 + b1 * zz) for zz in z]
    if any(not math.isfinite(p) for p in ps):
        raise AnalysisError("response model returned non-finite fitted probabilities")
    return {"probabilities": ps, "intercept_standardized": b0, "slope_standardized": b1,
            "x_mean": mean_x, "x_population_sd": sd_x, "iterations": iteration,
            "converged": True}


def fit_linear_outcome(x: Sequence[float], outcomes: Sequence[float | None]) -> tuple[list[float], dict[str, float]]:
    """OLS outcome regression y ~ 1 + x among observed outcomes; return all-unit predictions."""
    pairs = [(float(xi), float(yi)) for xi, yi in zip(x, outcomes) if yi is not None]
    if len(pairs) < 3:
        raise AnalysisError("outcome regression requires at least three observed outcomes")
    xm = statistics.fmean(a for a, _ in pairs)
    ym = statistics.fmean(b for _, b in pairs)
    den = sum((a - xm) ** 2 for a, _ in pairs)
    slope = sum((a - xm) * (b - ym) for a, b in pairs) / den if den > 1e-12 else 0.0
    intercept = ym - slope * xm
    predictions = [intercept + slope * float(xi) for xi in x]
    if any(not math.isfinite(v) for v in predictions):
        raise AnalysisError("outcome model returned non-finite predictions")
    return predictions, {"intercept": intercept, "slope": slope}


def _sample_se(values: Sequence[float]) -> float:
    if len(values) < 2:
        return math.inf
    return statistics.stdev(values) / math.sqrt(len(values))


def analyze(x: Sequence[float], outcomes: Sequence[float | None], *, margin: float = 5.0,
            sensitivity_deltas: Sequence[float] = tuple(range(-10, 11, 1)),
            positivity_floor: float = POSITIVITY_FLOOR) -> dict[str, Any]:
    """Fit MAR response/outcome models and return AIPW, IPW, diagnostics and MNAR intervals.

    AIPW score under the MAR identifying assumption is m(X)+R/e(X)*(Y-m(X)).
    A delta pattern-mixture sensitivity shifts the mean of missing outcomes relative
    to the fitted MAR conditional mean by delta percentage points. Its uncertainty
    is propagated with the empirical score including delta*(1-R); the 1.96/1.645
    normal critical values are asymptotic and must be qualified by held-out coverage tests.
    """
    xx, yy = _validate_data(x, outcomes)
    n = len(xx)
    r = [int(v is not None) for v in yy]
    nobs, nmiss = sum(r), n - sum(r)
    if not math.isfinite(margin):
        raise AnalysisError("margin must be finite")
    if any(not math.isfinite(float(d)) for d in sensitivity_deltas):
        raise AnalysisError("sensitivity deltas must be finite")
    observed_y = [float(v) for v in yy if v is not None]
    if nmiss == 0:
        # Complete data: no selection model is required or identifiable.
        scores = observed_y[:]
        props = [1.0] * n
        predictions, outcome_fit = fit_linear_outcome(xx, yy)
        response_fit = {"converged": True, "model": "not needed: no missing outcomes"}
    else:
        if nobs < 5 or nmiss < 2:
            raise AnalysisError("too few observed/missing families to fit response and outcome models")
        response_fit = fit_logistic_response(xx, r)
        props = response_fit["probabilities"]
        predictions, outcome_fit = fit_linear_outcome(xx, yy)
        scores = [predictions[i] + (yy[i] - predictions[i]) / props[i] if r[i] else predictions[i]
                  for i in range(n)]
    if any(not math.isfinite(v) for v in scores):
        raise AnalysisError("AIPW influence scores are non-finite")
    aipw = statistics.fmean(scores)
    se = _sample_se(scores)
    # Hájek stabilized inverse-probability-weighted estimate; available only with observed data.
    weights = [1.0 / props[i] for i in range(n) if r[i]]
    weighted_numer = sum(w * float(yy[i]) for i, w in zip([i for i in range(n) if r[i]], weights))
    ipw = weighted_numer / sum(weights) if weights and sum(weights) > 0 else math.nan
    ess = (sum(weights) ** 2 / sum(w * w for w in weights)) if weights else 0.0
    max_weight = max(weights) if weights else math.inf
    min_p, max_p = min(props), max(props)
    positivity_ok = (nmiss == 0) or (min_p >= positivity_floor and max_weight <= 1.0 / positivity_floor
                                     and max_p <= 1.0 - 1e-8)
    model_ok = bool(response_fit.get("converged", False))
    eligible = positivity_ok and model_ok
    # Influence-function approximation. It is a diagnostic, not exact finite-sample inference.
    aipw_ci = {"lower_95_two_sided": aipw - Z_TWO_SIDED_95 * se,
               "upper_95_two_sided": aipw + Z_TWO_SIDED_95 * se,
               "lower_95_one_sided": aipw - Z_ONE_SIDED_95 * se}
    delta_rows: list[dict[str, Any]] = []
    miss = [1 - v for v in r]
    for delta in sorted(set(float(d) for d in sensitivity_deltas)):
        adjusted_scores = [scores[i] + delta * miss[i] for i in range(n)]
        estimate = statistics.fmean(adjusted_scores)
        delta_se = _sample_se(adjusted_scores)
        lower = estimate - Z_ONE_SIDED_95 * delta_se
        upper_two = estimate + Z_TWO_SIDED_95 * delta_se
        delta_rows.append({"delta_pp_missing_minus_mar_prediction": delta,
                           "mean_estimate_pct_saving": estimate,
                           "standard_error_asymptotic": delta_se,
                           "lower_95_one_sided": lower,
                           "upper_95_two_sided": upper_two,
                           "lower_bound_exceeds_margin": lower > margin,
                           "margin_pct": margin})
    zero_row = next((row for row in delta_rows if abs(row["delta_pp_missing_minus_mar_prediction"]) < 1e-12), None)
    if zero_row is None:
        delta_zero = 0.0
        zero_lcb = aipw - Z_ONE_SIDED_95 * se
    else:
        delta_zero = zero_row["mean_estimate_pct_saving"]
        zero_lcb = zero_row["lower_95_one_sided"]
    negative_crossings = [row["delta_pp_missing_minus_mar_prediction"] for row in delta_rows
                          if row["delta_pp_missing_minus_mar_prediction"] < 0 and not row["lower_bound_exceeds_margin"]]
    negative_tipping = max(negative_crossings) if negative_crossings else None
    return {
        "n_independent_scene_families": n, "n_observed": nobs, "n_missing": nmiss,
        "missing_fraction": nmiss / n, "margin_pct": margin,
        "estimand": "arithmetic mean percentage-point energy saving across all attempted independent scene families",
        "methods": {
            "fitted_mar_aipw": {"estimate_pct_saving": aipw, "standard_error_asymptotic": se,
                                **aipw_ci,
                                "model_assumption": "MAR given observed pre-outcome covariate and correctly specified response or outcome nuisance model",
                                "inference_scope": "asymptotic influence-function diagnostic; finite-sample validity must be independently confirmed"},
            "fitted_mar_hajek_ipw": {"estimate_pct_saving": ipw,
                                     "estimand_warning": "stabilized inverse-probability weighted estimate; sensitive to estimated propensities and positivity"},
        },
        "diagnostics": {
            "response_model": response_fit,
            "outcome_model": outcome_fit,
            "propensity_min": min_p, "propensity_max": max_p,
            "maximum_observed_inverse_probability_weight": max_weight,
            "effective_sample_size_observed_weighted": ess,
            "effective_sample_size_fraction_of_n": ess / n,
            "positivity_floor": positivity_floor,
            "positivity_supported": positivity_ok,
            "eligible_for_mar_claim": eligible,
            "eligibility_reason": "eligible under declared one-covariate model diagnostics" if eligible else "FAIL_CLOSED: model convergence/positivity diagnostics do not support a MAR claim",
        },
        "pattern_mixture_sensitivity": {
            "delta_definition": "assumed missing-outcome mean minus fitted MAR conditional mean, in percentage points",
            "interval_method": "AIPW score plus delta times missing indicator; asymptotic influence-function standard error; one-sided 95% lower bound and two-sided 95% upper bound",
            "decision_at_delta_zero": {"estimate_pct_saving": delta_zero, "lower_95_one_sided": zero_lcb,
                                        "lower_bound_exceeds_margin": zero_lcb > margin},
            "negative_delta_tipping_point_on_supplied_grid_pp": negative_tipping,
            "tipping_point_scope": "coarse grid diagnostic only; None means no grid point at delta < 0 crossed the margin",
            "curve": delta_rows,
        },
        "claim_ceiling": "synthetic or observational statistical analysis only; no optical accuracy, measured energy saving, code compliance, safety, or SOTA claim",
    }


def _noise(rng: random.Random, family: str) -> float:
    if family == "normal_clipped":
        return max(-3.5, min(3.5, rng.gauss(0.0, 1.0)))
    if family == "right_skew_two_point":
        return 3.0 if rng.random() < 0.1 else -1.0 / 3.0
    if family == "rare_extremes":
        u = rng.random()
        return -math.sqrt(10.0) if u < 0.05 else math.sqrt(10.0) if u > 0.95 else 0.0
    if family == "left_skew_two_point":
        # P(Z=-3)=.1 and P(Z=1/3)=.9: mean zero, variance one.
        return -3.0 if rng.random() < 0.1 else 1.0 / 3.0
    if family == "student_t3_clipped":
        denom = math.sqrt(sum(rng.gauss(0.0, 1.0) ** 2 for _ in range(3)) / 3.0)
        return max(-4.0, min(4.0, rng.gauss(0.0, 1.0) / max(denom, 1e-9)))
    raise AnalysisError(f"unsupported distribution: {family}")


def _missing(rng: random.Random, mechanism: str, y: float, x: float, margin: float) -> bool:
    if mechanism == "complete":
        p_miss = 0.0
    elif mechanism == "mcar_30pct":
        p_miss = 0.30
    elif mechanism == "mcar_40pct":
        p_miss = 0.40
    elif mechanism == "mar_logistic_difficulty":
        p_response = logistic(1.7 - 1.3 * x)
        p_miss = 1.0 - p_response
    elif mechanism == "mar_nonlinear_difficulty":
        # Held-out MAR mechanism: response model is nonlinear in observed difficulty.
        p_response = logistic(1.8 - 1.3 * x + 2.0 * (x - 0.5) ** 2)
        p_miss = 1.0 - p_response
    elif mechanism == "mnar_low_outcome":
        p_miss = 0.65 if y < margin else 0.05
    else:
        raise AnalysisError(f"unsupported missingness mechanism: {mechanism}")
    return rng.random() < p_miss


def simulate(*, replicates: int = 500, seed: int = 20261028,
             sample_sizes: Sequence[int] = (20, 40, 80),
             distributions: Sequence[str] = ("normal_clipped", "right_skew_two_point", "rare_extremes"),
             mechanisms: Sequence[str] = ("complete", "mcar_30pct", "mar_logistic_difficulty", "mnar_low_outcome"),
             family_sd_pp: float = 3.0, difficulty_slope_pp: float = 3.0,
             margin_pct: float = 5.0) -> dict[str, Any]:
    if not isinstance(replicates, int) or isinstance(replicates, bool) or not (100 <= replicates <= 100_000):
        raise AnalysisError("replicates must be an integer in [100,100000]")
    if not isinstance(seed, int) or isinstance(seed, bool) or seed < 0:
        raise AnalysisError("seed must be a non-negative integer")
    if not sample_sizes or any(not isinstance(n, int) or isinstance(n, bool) or n < 10 for n in sample_sizes):
        raise AnalysisError("sample sizes must be integers >= 10")
    if family_sd_pp <= 0 or difficulty_slope_pp < 0 or not all(math.isfinite(v) for v in (family_sd_pp, difficulty_slope_pp, margin_pct)):
        raise AnalysisError("invalid finite effect configuration")
    rows: list[dict[str, Any]] = []
    ix = 0
    conditions = (("null_at_margin", 0.0), ("target_two_sd_above_margin", 2.0))
    for n in sample_sizes:
      for dist in distributions:
       for mechanism in mechanisms:
        for condition, multiplier in conditions:
         rng = random.Random(seed + ix * 7919)
         rejects = 0; cover = 0; eligible_count = 0; failures: dict[str, int] = {}
         estimates: list[float] = []; biases: list[float] = []; missing_fracs: list[float] = []
         for _ in range(replicates):
          x = [rng.random() for _ in range(n)]
          center = margin_pct + multiplier * family_sd_pp
          latent = [center + family_sd_pp * _noise(rng, dist) + difficulty_slope_pp * (0.5 - xx) for xx in x]
          observed: list[float | None] = []
          for xx, yy in zip(x, latent):
           observed.append(None if _missing(rng, mechanism, yy, xx, margin_pct) else yy)
          missing_fracs.append(sum(v is None for v in observed) / n)
          # Avoid inflating apparent success by silently dropping numerically ineligible samples.
          try:
           result = analyze(x, observed, margin=margin_pct, sensitivity_deltas=(-6.0, -3.0, 0.0, 3.0, 6.0))
          except AnalysisError as exc:
           key = str(exc).split(";")[0]
           failures[key] = failures.get(key, 0) + 1
           continue
          if not result["diagnostics"]["eligible_for_mar_claim"]:
           failures["positivity_or_model_diagnostic"] = failures.get("positivity_or_model_diagnostic", 0) + 1
           continue
          eligible_count += 1
          est = result["methods"]["fitted_mar_aipw"]["estimate_pct_saving"]
          se = result["methods"]["fitted_mar_aipw"]["standard_error_asymptotic"]
          estimates.append(est); biases.append(est - center)
          if est - Z_ONE_SIDED_95 * se > margin_pct:
           rejects += 1
          if est - Z_TWO_SIDED_95 * se <= center <= est + Z_TWO_SIDED_95 * se:
           cover += 1
         null_or_alt = condition == "null_at_margin"
         eligible_rate = eligible_count / replicates
         rows.append({
          "scenario_id": f"n{n}__{dist}__{mechanism}__{condition}", "n": n, "distribution": dist,
          "missingness": mechanism, "condition": condition, "target_mean_pct": center,
          "replicates": replicates, "eligible_replicates": eligible_count,
          "eligibility_rate": eligible_rate,
          "eligibility_rate_wilson_95pct": wilson_interval(eligible_count, replicates),
          "failure_reasons": failures,
          "aipw_mean_bias_pp_among_eligible": statistics.fmean(biases) if biases else None,
          "aipw_rmse_pp_among_eligible": math.sqrt(statistics.fmean(b*b for b in biases)) if biases else None,
          "aipw_two_sided_95pct_interval_coverage_among_eligible": cover / eligible_count if eligible_count else None,
          "aipw_two_sided_95pct_interval_coverage_wilson_95pct": wilson_interval(cover, eligible_count) if eligible_count else None,
          "one_sided_5pct_false_superiority_rate_if_null": rejects / eligible_count if eligible_count and null_or_alt else None,
          "one_sided_5pct_rejection_rate_if_target": rejects / eligible_count if eligible_count and not null_or_alt else None,
          "rejection_rate_wilson_95pct": wilson_interval(rejects, eligible_count) if eligible_count else None,
          "mean_missing_fraction": statistics.fmean(missing_fracs),
         })
         ix += 1
    nulls = [r for r in rows if r["condition"] == "null_at_margin" and r["eligibility_rate"] >= 0.8]
    eligible_rows = [r for r in rows if r["eligibility_rate"] >= 0.8]
    mechanism_summary: dict[str, Any] = {}
    for mechanism in mechanisms:
        mech_rows = [row for row in rows if row["missingness"] == mechanism]
        mech_nulls = [row for row in mech_rows if row["condition"] == "null_at_margin"]
        coverages = [row["aipw_two_sided_95pct_interval_coverage_among_eligible"] for row in mech_rows
                     if row["aipw_two_sided_95pct_interval_coverage_among_eligible"] is not None]
        biases = [row["aipw_mean_bias_pp_among_eligible"] for row in mech_nulls
                  if row["aipw_mean_bias_pp_among_eligible"] is not None]
        false_rates = [row["one_sided_5pct_false_superiority_rate_if_null"] for row in mech_nulls
                       if row["one_sided_5pct_false_superiority_rate_if_null"] is not None]
        mechanism_summary[mechanism] = {
            "scenario_count": len(mech_rows),
            "mean_eligibility_rate": statistics.fmean(row["eligibility_rate"] for row in mech_rows),
            "max_null_false_superiority_rate": max(false_rates) if false_rates else None,
            "min_two_sided_95pct_coverage": min(coverages) if coverages else None,
            "median_two_sided_95pct_coverage": statistics.median(coverages) if coverages else None,
            "mean_null_bias_pp": statistics.fmean(biases) if biases else None,
            "note": "false-superiority and coverage metrics under MNAR diagnose applying a MAR estimator outside its identifying assumptions; they are not a claim that MAR is expected to work under MNAR",
        }
    return {
      "schema_version": SCHEMA_VERSION,
      "analysis_kind": "fitted_mar_aipw_and_pattern_mixture_mnar_operating_characteristics",
      "decision": "synthetic_fitted_mar_mnar_interval_diagnostics_no_method_selected",
      "claim_scope": "synthetic statistical operating-characteristic experiment only; no real lighting measurement",
      "design": {"framework": "ADEMP", "estimand": "population mean percentage-point saving across independent families",
       "sampling_unit": "independent scene family", "margin_pct": margin_pct,
       "alpha_one_sided": 0.05, "family_sd_pp": family_sd_pp, "difficulty_slope_pp": difficulty_slope_pp,
       "distributions": list(distributions), "missingness_mechanisms": list(mechanisms),
       "conditions": [{"name": name, "mean_shift_in_sd": mult} for name, mult in conditions],
       "sample_sizes": list(sample_sizes), "replicates_per_scenario": replicates, "seed": seed,
       "MAR mechanism": "P(observed|difficulty)=logistic(1.7 - 1.3*difficulty); response probabilities remain bounded away from zero in the DGP",
       "MNAR mechanism": "missingness depends on latent outcome relative to decision margin; fitted MAR model is not assumed valid",
       "model": "fitted one-covariate logistic response model plus OLS outcome regression; AIPW score; asymptotic influence-function SE",
       "MC intervals": "individual 95% Wilson intervals; not simultaneous"},
      "summary": {
       "scenario_count": len(rows), "scenario_count_with_eligibility_rate_at_least_0_8": len(eligible_rows),
       "max_false_superiority_rate_among_eligible_null_scenarios_with_eligibility_at_least_0_8": max((r["one_sided_5pct_false_superiority_rate_if_null"] or 0.0 for r in nulls), default=None),
       "min_two_sided_interval_coverage_among_eligible_scenarios_with_eligibility_at_least_0_8": min((r["aipw_two_sided_95pct_interval_coverage_among_eligible"] for r in eligible_rows if r["aipw_two_sided_95pct_interval_coverage_among_eligible"] is not None), default=None),
       "note": "Rates condition on predeclared eligibility diagnostics; eligibility rates and failure reasons are reported separately. The worst-case summary spans MAR, MCAR, and MNAR; MAR estimator failure under MNAR is expected and must be addressed through sensitivity analysis. No method is selected from this grid.",
       "by_missingness_mechanism": mechanism_summary},
      "scenario_results": rows,
      "limitations": [
       "All outcomes are constructed; no measured buildings, calibrated photometry, or commercial solver output is used.",
       "The fitted MAR method requires response and outcome model assumptions; observed data cannot identify the MNAR departure without additional assumptions.",
       "The normal influence-function intervals are asymptotic and may undercover in small samples or heavy-tailed scenarios; the held-out suite is required before method selection.",
       "Eligibility conditioning can make conditional rejection rates appear better, so non-eligibility is reported independently and is itself a study-design failure mode.",
       "The single difficulty covariate is a deliberately limited prototype, not a complete missingness model for real lighting experiments.",
       "Scenario-wise Monte Carlo intervals are individual rather than simultaneous; results are exploratory and no winner is declared.",
      ]}


def main(argv: Sequence[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--replicates", type=int, default=500)
    p.add_argument("--seed", type=int, default=20261028)
    p.add_argument("--suite", choices=("development", "confirmation"), default="development")
    p.add_argument("--sample-sizes", default=None)
    p.add_argument("--output", type=Path)
    a = p.parse_args(argv)
    try:
        if a.suite == "confirmation":
            # Frozen independently from the development grid: different sizes, tails,
            # and a nonlinear-but-MAR response model. Do not tune on this suite.
            default_sizes = (30, 60, 120)
            distributions = ("left_skew_two_point", "student_t3_clipped", "rare_extremes")
            mechanisms = ("complete", "mcar_40pct", "mar_nonlinear_difficulty", "mnar_low_outcome")
        else:
            default_sizes = (20, 40, 80)
            distributions = ("normal_clipped", "right_skew_two_point", "rare_extremes")
            mechanisms = ("complete", "mcar_30pct", "mar_logistic_difficulty", "mnar_low_outcome")
        sizes = tuple(int(v.strip()) for v in a.sample_sizes.split(",") if v.strip()) if a.sample_sizes else default_sizes
        report = simulate(replicates=a.replicates, seed=a.seed, sample_sizes=sizes,
                          distributions=distributions, mechanisms=mechanisms)
        report["suite"] = a.suite
        report["source_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    except (AnalysisError, ValueError, OverflowError) as exc:
        print(f"fitted MAR/MNAR audit: FAIL: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n"
    if a.output:
        a.output.parent.mkdir(parents=True, exist_ok=True)
        a.output.write_text(rendered, encoding="utf-8")
    print(f"fitted MAR/MNAR audit: {report['decision']}")
    print(f"scenarios={report['summary']['scenario_count']} reps={a.replicates}")
    print("summary=" + json.dumps(report["summary"], sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

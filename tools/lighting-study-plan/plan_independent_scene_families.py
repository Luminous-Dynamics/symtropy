#!/usr/bin/env python3
"""Pilot-based, scene-family-aware sample-size planning for lighting benchmarks.

This is a planning approximation, not the confirmatory comparison engine. Seeds
and grid points are nested observations; the independent sampling unit is the
scene family. It never turns synthetic evidence into an empirical claim.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path
from statistics import NormalDist

STATUSES = {
    "compliant", "non_compliant", "unknown", "unsupported", "not_evaluated",
    "timeout", "invalid_output", "invalid_input", "over_budget",
}
TIERS = {
    "synthetic_contract", "simulation_only", "calibrated_simulation",
    "physical_prototype", "independently_tested_product",
}
REQUIRED = {
    "scene_family", "seed", "baseline_status", "candidate_status",
    "baseline_energy_wh_per_compliant_task_hour",
    "candidate_energy_wh_per_compliant_task_hour",
}


class PlanInputError(ValueError):
    """Input does not satisfy the frozen study-design contract."""


def _nonempty(row: dict[str, str], name: str, line: int) -> str:
    value = (row.get(name) or "").strip()
    if not value:
        raise PlanInputError(f"line {line}: {name} must not be blank")
    return value


def _energy(raw: str | None, status: str, name: str, line: int) -> float | None:
    value = (raw or "").strip()
    if status != "compliant":
        if value:
            raise PlanInputError(f"line {line}: {name} must be blank when status={status}")
        return None
    if not value:
        raise PlanInputError(f"line {line}: compliant status requires {name}")
    try:
        number = float(value)
    except ValueError as exc:
        raise PlanInputError(f"line {line}: {name} is not numeric") from exc
    if not math.isfinite(number) or number <= 0:
        raise PlanInputError(f"line {line}: {name} must be finite and > 0")
    return number


def load_pilot(path: Path) -> list[dict[str, object]]:
    try:
        stream = path.open("r", newline="", encoding="utf-8-sig")
    except OSError as exc:
        raise PlanInputError(f"cannot read pilot CSV: {exc}") from exc
    with stream:
        reader = csv.DictReader(stream)
        if reader.fieldnames is None:
            raise PlanInputError("CSV has no header")
        missing = sorted(REQUIRED - set(reader.fieldnames))
        if missing:
            raise PlanInputError("missing required columns: " + ", ".join(missing))
        rows: list[dict[str, object]] = []
        seen: set[tuple[str, str]] = set()
        for line, raw in enumerate(reader, start=2):
            if not raw or all(not (v or "").strip() for v in raw.values()):
                continue
            family = _nonempty(raw, "scene_family", line)
            seed = _nonempty(raw, "seed", line)
            key = (family, seed)
            if key in seen:
                raise PlanInputError(f"line {line}: duplicate scene-family/seed pair {key}")
            seen.add(key)
            base_status = _nonempty(raw, "baseline_status", line)
            cand_status = _nonempty(raw, "candidate_status", line)
            if base_status not in STATUSES or cand_status not in STATUSES:
                raise PlanInputError(f"line {line}: unsupported baseline/candidate status")
            base_energy = _energy(raw.get("baseline_energy_wh_per_compliant_task_hour"), base_status, "baseline energy", line)
            cand_energy = _energy(raw.get("candidate_energy_wh_per_compliant_task_hour"), cand_status, "candidate energy", line)
            rows.append({
                "scene_family": family, "seed": seed,
                "baseline_status": base_status, "candidate_status": cand_status,
                "baseline_energy": base_energy, "candidate_energy": cand_energy,
            })
    if not rows:
        raise PlanInputError("CSV has no pilot rows")
    seed_sets: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        seed_sets[str(row["scene_family"])].add(str(row["seed"]))
    known = list(seed_sets.values())
    if any(items != known[0] for items in known[1:]):
        raise PlanInputError("seed schedule is unbalanced across scene families")
    return rows


def _beta_continued_fraction(a: float, b: float, x: float) -> float:
    """Numerical Recipes continued fraction for the incomplete beta function."""
    max_iterations = 200
    epsilon = 3.0e-14
    tiny = 1.0e-300
    qab = a + b
    qap = a + 1.0
    qam = a - 1.0
    c = 1.0
    d = 1.0 - qab * x / qap
    if abs(d) < tiny:
        d = tiny
    d = 1.0 / d
    result = d
    for m in range(1, max_iterations + 1):
        m2 = 2 * m
        aa = m * (b - m) * x / ((qam + m2) * (a + m2))
        d = 1.0 + aa * d
        if abs(d) < tiny:
            d = tiny
        c = 1.0 + aa / c
        if abs(c) < tiny:
            c = tiny
        d = 1.0 / d
        result *= d * c
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2))
        d = 1.0 + aa * d
        if abs(d) < tiny:
            d = tiny
        c = 1.0 + aa / c
        if abs(c) < tiny:
            c = tiny
        d = 1.0 / d
        delta = d * c
        result *= delta
        if abs(delta - 1.0) < epsilon:
            return result
    raise PlanInputError("incomplete-beta continued fraction did not converge")


def _regularized_beta(x: float, a: float, b: float) -> float:
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    log_front = math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b) + a * math.log(x) + b * math.log1p(-x)
    front = math.exp(log_front)
    if x < (a + 1.0) / (a + b + 2.0):
        return min(1.0, max(0.0, front * _beta_continued_fraction(a, b, x) / a))
    return min(1.0, max(0.0, 1.0 - front * _beta_continued_fraction(b, a, 1.0 - x) / b))


def _student_t_cdf(value: float, degrees_of_freedom: int) -> float:
    if degrees_of_freedom < 1:
        raise PlanInputError("degrees of freedom must be positive")
    if value == 0.0:
        return 0.5
    beta_x = degrees_of_freedom / (degrees_of_freedom + value * value)
    regularized = _regularized_beta(beta_x, degrees_of_freedom / 2.0, 0.5)
    return 1.0 - 0.5 * regularized if value > 0.0 else 0.5 * regularized


def _t_critical(alpha: float, degrees_of_freedom: int) -> float:
    """Numerically invert the Student-t CDF for a one-sided critical value."""
    if not (0.0 < alpha < 0.5) or degrees_of_freedom < 1:
        raise PlanInputError("invalid alpha or degrees of freedom for t critical value")
    low, high = 0.0, 1.0
    while _student_t_cdf(high, degrees_of_freedom) < 1.0 - alpha:
        high *= 2.0
        if high > 1.0e8:
            raise PlanInputError("could not bracket Student-t critical value")
    for _ in range(100):
        middle = (low + high) / 2.0
        if _student_t_cdf(middle, degrees_of_freedom) < 1.0 - alpha:
            low = middle
        else:
            high = middle
    return (low + high) / 2.0


def _approx_power(n: int, sigma: float, excess_effect: float, alpha: float) -> float:
    # Plug-in normal-effect approximation. The pilot SD is treated as known;
    # this is deliberately a transparent screening estimate, not exact noncentral-t power.
    threshold = _t_critical(alpha, n - 1)
    noncentrality = excess_effect * math.sqrt(n) / sigma
    return 1.0 - NormalDist().cdf(threshold - noncentrality)


def _required_families(*, sigma: float, assumed_effect: float, margin: float,
                       target_power: float, alpha: float, min_new_families: int,
                       max_families: int) -> dict[str, object]:
    if assumed_effect <= margin:
        return {"status": "UNRESOLVED_EFFECT_NOT_ABOVE_MARGIN", "required_new_independent_families": None,
                "achieved_approx_power": None, "reason": "assumed effect must exceed the superiority margin"}
    excess = assumed_effect - margin
    for n in range(min_new_families, max_families + 1):
        power = _approx_power(n, sigma, excess, alpha)
        if power >= target_power:
            return {"status": "PLANNING_ESTIMATE", "required_new_independent_families": n,
                    "achieved_approx_power": power, "reason": None}
    return {"status": "UNRESOLVED_MAX_FAMILIES_EXCEEDED", "required_new_independent_families": None,
            "achieved_approx_power": _approx_power(max_families, sigma, excess, alpha),
            "reason": f"target power not reached by max_families={max_families}"}


def build_report(rows: list[dict[str, object]], *, input_sha256: str, source_sha256: str,
                 evidence_tier: str, assumed_effect_pct: float, superiority_margin_pct: float,
                 target_power: float, alpha: float, sd_multipliers: list[float],
                 min_families: int, min_new_families: int, min_pair_coverage: float,
                 min_feasibility: float, max_families: int,
                 comparison_family_size: int = 1) -> dict[str, object]:
    if evidence_tier not in TIERS:
        raise PlanInputError(f"unsupported evidence tier {evidence_tier!r}")
    if not (0.5 < target_power < 1.0) or not (0.0 < alpha < 0.2):
        raise PlanInputError("target_power must be in (0.5,1); alpha must be in (0,0.2)")
    if not isinstance(comparison_family_size, int) or isinstance(comparison_family_size, bool) or comparison_family_size < 1:
        raise PlanInputError("comparison_family_size must be a positive integer")
    alpha_per_comparison = alpha / comparison_family_size
    if not (0.0 <= min_pair_coverage <= 1.0 and 0.0 <= min_feasibility <= 1.0):
        raise PlanInputError("coverage and feasibility thresholds must be in [0,1]")
    if min_families < 10 or min_new_families < 10 or max_families < min_new_families:
        raise PlanInputError("require at least 10 pilot families, at least 10 new families, and max_families >= min_new_families")
    if not sd_multipliers or any(not math.isfinite(x) or x <= 0 for x in sd_multipliers):
        raise PlanInputError("SD sensitivity multipliers must be finite and positive")
    if len(set(sd_multipliers)) != len(sd_multipliers):
        raise PlanInputError("SD sensitivity multipliers must be unique")
    if not math.isfinite(assumed_effect_pct) or not math.isfinite(superiority_margin_pct):
        raise PlanInputError("effect and superiority margin must be finite")

    by_family: dict[str, list[dict[str, object]]] = defaultdict(list)
    for row in rows:
        by_family[str(row["scene_family"])].append(row)
    total_pairs = len(rows)
    paired_effects = 0
    baseline_feasible = 0
    candidate_feasible = 0
    baseline_failures: Counter[str] = Counter()
    candidate_failures: Counter[str] = Counter()
    family_effects: dict[str, float] = {}
    family_coverage: dict[str, float] = {}
    for family, family_rows in by_family.items():
        effect_values: list[float] = []
        pairable = 0
        for row in family_rows:
            b_ok = row["baseline_status"] == "compliant"
            c_ok = row["candidate_status"] == "compliant"
            baseline_feasible += int(b_ok)
            candidate_feasible += int(c_ok)
            if not b_ok:
                baseline_failures[str(row["baseline_status"])] += 1
            if not c_ok:
                candidate_failures[str(row["candidate_status"])] += 1
            if b_ok and c_ok:
                baseline = float(row["baseline_energy"])
                candidate = float(row["candidate_energy"])
                effect_values.append(100.0 * (baseline - candidate) / baseline)
                paired_effects += 1
                pairable += 1
        family_coverage[family] = pairable / len(family_rows)
        if effect_values:
            family_effects[family] = statistics.median(effect_values)

    scene_families = len(by_family)
    pair_coverage = paired_effects / total_pairs
    baseline_feasibility = baseline_feasible / total_pairs
    candidate_feasibility = candidate_feasible / total_pairs
    paired_family_values = list(family_effects.values())
    all_families_pairable = len(family_effects) == scene_families
    all_family_coverage_pass = all(value >= min_pair_coverage for value in family_coverage.values())
    gates = {
        "minimum_pilot_scene_families": scene_families >= min_families,
        "minimum_overall_paired_energy_coverage": pair_coverage >= min_pair_coverage,
        "minimum_coverage_in_every_scene_family": all_family_coverage_pass,
        "every_scene_family_has_paired_energy": all_families_pairable,
        "baseline_feasibility_gate": baseline_feasibility >= min_feasibility,
        "candidate_feasibility_gate": candidate_feasibility >= min_feasibility,
    }
    gating_pass = all(gates.values())
    mean_effect = statistics.mean(paired_family_values) if paired_family_values else None
    median_effect = statistics.median(paired_family_values) if paired_family_values else None
    sd_effect = statistics.stdev(paired_family_values) if len(paired_family_values) >= 2 else None
    sensitivity: list[dict[str, object]] = []
    if gating_pass and sd_effect is not None and sd_effect > 0:
        for multiplier in sd_multipliers:
            result = _required_families(
                sigma=sd_effect * multiplier,
                assumed_effect=assumed_effect_pct,
                margin=superiority_margin_pct,
                target_power=target_power,
                alpha=alpha_per_comparison,
                min_new_families=min_new_families,
                max_families=max_families,
            )
            sensitivity.append({"pilot_sd_multiplier": multiplier, "assumed_sd_pp": sd_effect * multiplier, **result})
    else:
        if not gating_pass:
            unresolved_status = "UNRESOLVED_PILOT_GATE_FAILED"
            unresolved_reason = "pilot gating requirements did not pass"
        else:
            unresolved_status = "UNRESOLVED_VARIANCE_NOT_ESTIMABLE"
            unresolved_reason = "family-level SD is zero or undefined; do not infer infinite precision"
        sensitivity = [{"pilot_sd_multiplier": multiplier, "assumed_sd_pp": sd_effect * multiplier if sd_effect is not None else None,
                        "status": unresolved_status, "required_new_independent_families": None,
                        "achieved_approx_power": None, "reason": unresolved_reason}
                       for multiplier in sd_multipliers]

    if evidence_tier == "synthetic_contract":
        decision = "synthetic_planning_demo_no_empirical_claim"
        claim_scope = "synthetic design-planning demonstration only; no empirical or superiority claim permitted"
    elif not gating_pass:
        decision = "UNRESOLVED_PILOT_QUALITY_OR_COVERAGE_GATES"
        claim_scope = "planning not qualified; pilot quality/coverage gates are not met"
    elif any(item["required_new_independent_families"] is None for item in sensitivity):
        decision = "UNRESOLVED_DESIGN_SENSITIVITY"
        claim_scope = "at least one preregistered SD sensitivity did not reach the target within the declared maximum"
    else:
        decision = "planning_estimate_only_requires_preregistration"
        claim_scope = "sample-size planning approximation for the declared energy endpoint; not proof of superiority or feasibility"

    return {
        "schema_version": "0.1.0",
        "analysis_kind": "pilot_based_independent_scene_family_power_planning",
        "decision": decision,
        "claim_scope": claim_scope,
        "evidence_tier": evidence_tier,
        "input_csv_sha256": input_sha256,
        "planner_source_sha256": source_sha256,
        "sampling_unit": "independent_scene_family",
        "nested_repeats": "seeds are aggregated to a within-family median effect; seeds and grid points are not independent n",
        "pilot": {
            "scene_family_count": scene_families,
            "total_family_seed_pairs": total_pairs,
            "paired_energy_rows": paired_effects,
            "paired_energy_coverage": pair_coverage,
            "baseline_feasibility_rate": baseline_feasibility,
            "candidate_feasibility_rate": candidate_feasibility,
            "family_effect_mean_savings_pct": mean_effect,
            "family_effect_median_savings_pct": median_effect,
            "family_effect_sd_percentage_points": sd_effect,
            "family_paired_coverage": dict(sorted(family_coverage.items())),
            "baseline_noncompliant_status_counts": dict(sorted(baseline_failures.items())),
            "candidate_noncompliant_status_counts": dict(sorted(candidate_failures.items())),
        },
        "design_assumptions": {
            "assumed_true_mean_savings_pct": assumed_effect_pct,
            "superiority_margin_savings_pct": superiority_margin_pct,
            "alpha_familywise_one_sided": alpha,
            "comparison_family_size": comparison_family_size,
            "alpha_per_comparison_one_sided": alpha_per_comparison,
            "target_power": target_power,
            "sd_sensitivity_multipliers": sd_multipliers,
            "min_pilot_scene_families": min_families,
            "min_new_confirmatory_scene_families": min_new_families,
            "min_paired_coverage": min_pair_coverage,
            "min_feasibility_rate_per_method": min_feasibility,
            "max_new_independent_families": max_families,
            "pilot_families_reused_in_confirmatory_set": False,
            "method": "normal-effect plug-in-SD power approximation with numerically inverted Student-t critical value",
            "limitations": [
                "Power is approximate and assumes a normal distribution of family-level effects with pilot SD treated as known.",
                "Uncertainty in the pilot SD and distribution tails are not fully represented; the sensitivity multipliers are mandatory, not a guarantee.",
                "This plans only the conditional energy-superiority endpoint; feasibility and other safety/quality endpoints require separate prospective power/design gates.",
                "The pilot family set must be independent of the confirmatory held-out scene-family set.",
                "Caller-declared evidence tier is not independently authenticated by this tool.",
            ],
        },
        "pilot_gates": {"all_pass": gating_pass, "checks": gates},
        "sensitivity": sensitivity,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pilot_csv", type=Path)
    parser.add_argument("--evidence-tier", required=True, choices=sorted(TIERS))
    parser.add_argument("--assumed-true-effect-pct", type=float, required=True,
                        help="preregistered design alternative, not automatically inferred from the pilot")
    parser.add_argument("--superiority-margin-pct", type=float, default=5.0)
    parser.add_argument("--target-power", type=float, default=0.80)
    parser.add_argument("--alpha", type=float, default=0.05, help="one-sided family-wise alpha across the declared comparison family")
    parser.add_argument("--comparison-family-size", type=int, default=1,
                        help="number of preregistered superiority comparisons adjusted using Bonferroni")
    parser.add_argument("--sd-multipliers", default="1,1.5,2", help="comma-separated mandatory pilot-SD sensitivity factors")
    parser.add_argument("--min-families", type=int, default=10, help="minimum independent pilot families")
    parser.add_argument("--min-new-families", type=int, default=10, help="minimum new held-out confirmatory families")
    parser.add_argument("--min-pair-coverage", type=float, default=0.90)
    parser.add_argument("--min-feasibility", type=float, default=0.95)
    parser.add_argument("--max-families", type=int, default=500)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        raw = args.pilot_csv.read_bytes()
        rows = load_pilot(args.pilot_csv)
        multipliers = [float(item.strip()) for item in args.sd_multipliers.split(",") if item.strip()]
        source_sha = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        report = build_report(rows, input_sha256=hashlib.sha256(raw).hexdigest(), source_sha256=source_sha,
                              evidence_tier=args.evidence_tier, assumed_effect_pct=args.assumed_true_effect_pct,
                              superiority_margin_pct=args.superiority_margin_pct, target_power=args.target_power,
                              alpha=args.alpha, sd_multipliers=multipliers, min_families=args.min_families,
                              min_new_families=args.min_new_families, min_pair_coverage=args.min_pair_coverage,
                              min_feasibility=args.min_feasibility,
                              max_families=args.max_families,
                              comparison_family_size=args.comparison_family_size)
    except (PlanInputError, OSError, ValueError, OverflowError) as exc:
        print(f"lighting study plan: FAIL: {exc}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(f"lighting study plan: {report['decision']}")
    print(f"independent_pilot_families={report['pilot']['scene_family_count']} "
          f"paired_coverage={report['pilot']['paired_energy_coverage']:.3f}")
    print(f"comparison_family_size={args.comparison_family_size} "
          f"alpha_per_comparison={args.alpha / args.comparison_family_size:.6g}")
    for result in report["sensitivity"]:
        print(f"sd_multiplier={result['pilot_sd_multiplier']} "
              f"new_independent_families={result['required_new_independent_families']} "
              f"status={result['status']}")
    return 0 if report["decision"] != "UNRESOLVED_PILOT_QUALITY_OR_COVERAGE_GATES" else 3


if __name__ == "__main__":
    raise SystemExit(main())

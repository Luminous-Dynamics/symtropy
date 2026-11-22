#!/usr/bin/env python3
"""Anytime-valid, margin-qualified Pareto certificates for matched policy scores.

All inputs are per-episode utility scores normalized to [0, 1] with higher
meaning better. Resource minimization axes must be converted by a frozen,
monotone transform declared in the metric manifest. The evaluator reports
margin-qualified dominance, not exact mathematical Pareto dominance.

For each ordered policy pair and metric, it tests:
  non-inferiority: mean(candidate - reference) > -delta
  material gain:   mean(candidate - reference) > epsilon

Each directional test is a bounded paired betting e-process. A union-bound
allocation across every ordered pair, axis, direction and time controls the
familywise false-certificate probability.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from typing import Any

BENCHMARK_ID = "av-pareto-anytime-cs-v0"
METHOD_ID = "paired-shifted-hoeffding-betting-eprocess-v0"
DEFAULT_LAMBDAS = (0.03, 0.06, 0.12, 0.25, 0.5)
PROVENANCE = {
    "MEASURED",
    "DERIVED_FROM_MEASUREMENT",
    "SYNTHETIC",
    "ESTIMATED",
    "NOT_MEASURED",
    "NOT_APPLICABLE",
}


class ParetoEvaluationError(ValueError):
    """Malformed or unsupported Pareto-CS input."""


def _finite_number(value: Any) -> bool:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    try:
        return math.isfinite(float(value))
    except (OverflowError, TypeError, ValueError):
        return False


def _logsumexp(values: list[float]) -> float:
    if not values:
        raise ParetoEvaluationError("logsumexp requires at least one value")
    maximum = max(values)
    if maximum == -math.inf:
        return -math.inf
    return maximum + math.log(sum(math.exp(value - maximum) for value in values))


def _canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def make_order_commitment(
    protocol_id: str,
    episode_ids: list[str],
    episode_order: list[str],
    seed_commitment: str,
) -> str:
    return hashlib.sha256(_canonical_json({
        "protocol_id": protocol_id,
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
    })).hexdigest()


class BettingEProcess:
    """Finite-cohort betting e-process for one bounded shifted paired difference."""

    def __init__(
        self,
        *,
        cohort_size: int,
        alpha: float,
        betting_fractions: list[float],
    ) -> None:
        self.cohort_size = cohort_size
        self.alpha = alpha
        self.betting_fractions = betting_fractions
        self.log_gamblers = [0.0 for _ in betting_fractions]
        self.cumulative_difference = 0.0
        self.looks = 0
        self.max_log_wealth = 0.0
        self.crossed = False
        self.log_threshold = math.log(1.0 / alpha)

    def update(self, value: float) -> dict[str, Any]:
        if not _finite_number(value) or not -1.0 <= value <= 1.0:
            raise ParetoEvaluationError("shifted paired difference must be finite and in [-1, 1]")
        self.looks += 1
        remaining = self.cohort_size - self.looks + 1
        if remaining < 1:
            raise ParetoEvaluationError("more observations supplied than finite cohort size")
        offset = max(
            -0.99,
            min(1.0, -self.cumulative_difference / remaining),
        )
        denominator = 1.0 + offset
        for index, betting_fraction in enumerate(self.betting_fractions):
            factor = 1.0 + betting_fraction * (float(value) - offset) / denominator
            if not math.isfinite(factor) or factor <= 0.0:
                raise ParetoEvaluationError("betting factor is non-positive or non-finite")
            self.log_gamblers[index] += math.log(factor)
        self.cumulative_difference += float(value)
        current_log_wealth = (
            _logsumexp(self.log_gamblers) - math.log(len(self.betting_fractions))
        )
        self.max_log_wealth = max(self.max_log_wealth, current_log_wealth)
        if self.max_log_wealth >= self.log_threshold:
            self.crossed = True
        return {
            "crossed": self.crossed,
            "log_wealth": current_log_wealth,
            "max_log_wealth": self.max_log_wealth,
            "offset": offset,
            "alpha_local": self.alpha,
        }


def _has_directed_cycle(policies: list[str], edges: list[dict[str, str]]) -> bool:
    graph = {policy: [] for policy in policies}
    for edge in edges:
        graph[edge["better"]].append(edge["worse"])
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(policy: str) -> bool:
        if policy in visiting:
            return True
        if policy in visited:
            return False
        visiting.add(policy)
        for neighbor in graph[policy]:
            if visit(neighbor):
                return True
        visiting.remove(policy)
        visited.add(policy)
        return False

    return any(visit(policy) for policy in policies if policy not in visited)


def _validate_metric_manifest(axes: Any) -> list[dict[str, Any]]:
    if not isinstance(axes, list) or not axes:
        raise ParetoEvaluationError("axes must be a non-empty array")
    identifiers: set[str] = set()
    validated = []
    for index, axis in enumerate(axes):
        if not isinstance(axis, dict):
            raise ParetoEvaluationError(f"axis {index} must be an object")
        metric_id = axis.get("metric_id")
        if not isinstance(metric_id, str) or not metric_id or metric_id in identifiers:
            raise ParetoEvaluationError("metric_id values must be non-empty and unique")
        identifiers.add(metric_id)
        if not isinstance(axis.get("unit"), str) or not axis["unit"]:
            raise ParetoEvaluationError(f"{metric_id}: raw unit is required")
        if axis.get("source_direction") not in {"maximize", "minimize"}:
            raise ParetoEvaluationError(f"{metric_id}: source_direction must be maximize/minimize")
        if axis.get("score_direction") != "higher_is_better":
            raise ParetoEvaluationError(f"{metric_id}: normalized score_direction must be higher_is_better")
        transform_id = axis.get("transform_id")
        if not isinstance(transform_id, str) or not transform_id:
            raise ParetoEvaluationError(f"{metric_id}: frozen transform_id is required")
        transform_digest = axis.get("transform_digest_sha256")
        if not isinstance(transform_digest, str) or len(transform_digest) != 64:
            raise ParetoEvaluationError(f"{metric_id}: transform code/spec requires a SHA-256 digest")
        try:
            int(transform_digest, 16)
        except ValueError as exc:
            raise ParetoEvaluationError(f"{metric_id}: invalid transform digest") from exc
        provenance = axis.get("provenance")
        if provenance not in PROVENANCE:
            raise ParetoEvaluationError(f"{metric_id}: invalid measurement provenance")
        if provenance == "NOT_APPLICABLE":
            raise ParetoEvaluationError(f"{metric_id}: remove not-applicable axes from the shared metric set")
        if provenance in {"MEASURED", "DERIVED_FROM_MEASUREMENT"}:
            method = axis.get("measurement_method")
            digest = axis.get("evidence_digest_sha256")
            if not isinstance(method, str) or not method:
                raise ParetoEvaluationError(f"{metric_id}: measured axis requires measurement_method")
            if not isinstance(digest, str) or len(digest) != 64:
                raise ParetoEvaluationError(f"{metric_id}: measured axis requires SHA-256 evidence digest")
            try:
                int(digest, 16)
            except ValueError as exc:
                raise ParetoEvaluationError(f"{metric_id}: invalid evidence digest") from exc
        delta = axis.get("no_worse_tolerance")
        epsilon = axis.get("material_improvement_margin")
        if not _finite_number(delta) or not 0.0 <= delta < 1.0:
            raise ParetoEvaluationError(f"{metric_id}: no_worse_tolerance must be in [0,1)")
        if not _finite_number(epsilon) or not 0.0 <= epsilon < 1.0:
            raise ParetoEvaluationError(f"{metric_id}: material_improvement_margin must be in [0,1)")
        validated.append({
            "metric_id": metric_id,
            "unit": axis["unit"],
            "source_direction": axis["source_direction"],
            "score_direction": "higher_is_better",
            "transform_id": transform_id,
            "transform_digest_sha256": transform_digest,
            "provenance": provenance,
            "measurement_method": axis.get("measurement_method"),
            "evidence_digest_sha256": axis.get("evidence_digest_sha256"),
            "no_worse_tolerance": float(delta),
            "material_improvement_margin": float(epsilon),
        })
    return validated


def evaluate_document(document: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise ParetoEvaluationError("input root must be an object")
    if document.get("benchmark_id") != BENCHMARK_ID:
        raise ParetoEvaluationError("benchmark_id mismatch")
    version = document.get("schema_version")
    if isinstance(version, bool) or not isinstance(version, int) or version != 1:
        raise ParetoEvaluationError("schema_version must be integer 1")
    protocol_id = document.get("protocol_id")
    if not isinstance(protocol_id, str) or not protocol_id:
        raise ParetoEvaluationError("protocol_id is required")

    policies = document.get("policies")
    if not isinstance(policies, list) or len(policies) < 2:
        raise ParetoEvaluationError("policies must contain at least two policy IDs")
    if any(not isinstance(policy, str) or not policy for policy in policies):
        raise ParetoEvaluationError("policy IDs must be non-empty strings")
    if len(set(policies)) != len(policies):
        raise ParetoEvaluationError("policy IDs must be unique")

    axes = _validate_metric_manifest(document.get("axes"))
    metric_ids = [axis["metric_id"] for axis in axes]
    metric_count = len(axes)
    policy_count = len(policies)

    alpha = document.get("alpha", 0.05)
    if not _finite_number(alpha) or not 0.0 < alpha < 1.0:
        raise ParetoEvaluationError("alpha must be finite and strictly between 0 and 1")
    alpha = float(alpha)
    betting_fractions = document.get("betting_fractions", list(DEFAULT_LAMBDAS))
    if not isinstance(betting_fractions, list) or not betting_fractions:
        raise ParetoEvaluationError("betting_fractions must be a non-empty array")
    if any(not _finite_number(value) or not 0.0 < value < 1.0 for value in betting_fractions):
        raise ParetoEvaluationError("every betting fraction must be finite and in (0,1)")
    betting_fractions = [float(value) for value in betting_fractions]
    if len(set(betting_fractions)) != len(betting_fractions):
        raise ParetoEvaluationError("betting_fractions must be unique")

    episode_ids = document.get("episode_ids")
    episode_order = document.get("episode_order")
    seed_commitment = document.get("seed_commitment")
    order_commitment = document.get("order_commitment_sha256")
    if not isinstance(episode_ids, list) or not episode_ids:
        raise ParetoEvaluationError("episode_ids must be a non-empty array")
    if any(not isinstance(item, str) or not item for item in episode_ids):
        raise ParetoEvaluationError("episode IDs must be non-empty strings")
    if len(set(episode_ids)) != len(episode_ids) or episode_ids != sorted(episode_ids):
        raise ParetoEvaluationError("episode IDs must be unique and canonically sorted")
    if not isinstance(episode_order, list) or len(episode_order) != len(episode_ids):
        raise ParetoEvaluationError("episode_order must contain every episode exactly once")
    if any(not isinstance(item, str) or not item for item in episode_order):
        raise ParetoEvaluationError("episode_order entries must be non-empty strings")
    if len(set(episode_order)) != len(episode_order) or set(episode_order) != set(episode_ids):
        raise ParetoEvaluationError("episode_order must be an exact permutation of episode_ids")
    if not isinstance(seed_commitment, str) or len(seed_commitment) != 64:
        raise ParetoEvaluationError("seed_commitment must be a SHA-256 digest")
    try:
        int(seed_commitment, 16)
    except ValueError as exc:
        raise ParetoEvaluationError("seed_commitment must be hexadecimal") from exc
    expected_commitment = make_order_commitment(
        protocol_id, episode_ids, episode_order, seed_commitment
    )
    if order_commitment != expected_commitment:
        raise ParetoEvaluationError("precommitted order digest mismatch")

    hard_gates = document.get("hard_gates")
    if not isinstance(hard_gates, dict) or set(hard_gates) != set(policies):
        raise ParetoEvaluationError("hard_gates must declare exactly one gate record per policy")
    eligibility: dict[str, dict[str, Any]] = {}
    for policy in policies:
        gate = hard_gates[policy]
        if not isinstance(gate, dict) or set(gate) != {"eligible", "failed_constraints"}:
            raise ParetoEvaluationError(f"{policy}: hard-gate record must contain only eligible and failed_constraints")
        if not isinstance(gate.get("eligible"), bool):
            raise ParetoEvaluationError(f"{policy}: hard-gate eligible must be boolean")
        failures = gate.get("failed_constraints")
        if not isinstance(failures, list) or any(not isinstance(reason, str) or not reason for reason in failures):
            raise ParetoEvaluationError(f"{policy}: failed_constraints must be an array of reason strings")
        if gate["eligible"] and failures:
            raise ParetoEvaluationError(f"{policy}: eligible policy cannot list failed constraints")
        if not gate["eligible"] and not failures:
            raise ParetoEvaluationError(f"{policy}: ineligible policy must list failed constraints")
        eligibility[policy] = {"eligible": gate["eligible"], "failed_constraints": failures}

    items = document.get("items")
    if not isinstance(items, list) or len(items) != len(episode_order):
        raise ParetoEvaluationError("items must contain one score vector per episode")
    eligible_policies = {
        policy for policy in policies if eligibility[policy]["eligible"]
    }
    for index, (item, expected_id) in enumerate(zip(items, episode_order)):
        if not isinstance(item, dict) or set(item) != {"episode_id", "scores"}:
            raise ParetoEvaluationError(f"item {index}: expected only episode_id and scores")
        if item["episode_id"] != expected_id:
            raise ParetoEvaluationError("items must follow the precommitted episode order")
        scores = item["scores"]
        if not isinstance(scores, dict) or set(scores) != eligible_policies:
            raise ParetoEvaluationError("each item must score every eligible policy and no ineligible policy")
        for policy in eligible_policies:
            vector = scores[policy]
            if not isinstance(vector, dict) or set(vector) != set(metric_ids):
                raise ParetoEvaluationError(f"item {index}, {policy}: score for every metric is required")
            for metric_id in metric_ids:
                value = vector[metric_id]
                if not _finite_number(value) or not 0.0 <= float(value) <= 1.0:
                    raise ParetoEvaluationError(f"item {index}, {policy}, {metric_id}: utility score must be in [0,1]")

    test_count = 2 * metric_count * policy_count * (policy_count - 1)
    alpha_local = alpha / test_count
    n_items = len(items)
    processes: dict[tuple[str, str, str, str], BettingEProcess] = {}
    for candidate in policies:
        for reference in policies:
            if candidate == reference:
                continue
            for axis in axes:
                metric_id = axis["metric_id"]
                processes[(candidate, reference, metric_id, "no_worse")] = BettingEProcess(
                    cohort_size=n_items,
                    alpha=alpha_local,
                    betting_fractions=betting_fractions,
                )
                processes[(candidate, reference, metric_id, "material_improvement")] = BettingEProcess(
                    cohort_size=n_items,
                    alpha=alpha_local,
                    betting_fractions=betting_fractions,
                )

    header = {
        "benchmark_id": BENCHMARK_ID,
        "protocol_id": protocol_id,
        "policies": policies,
        "axes": axes,
        "alpha": alpha,
        "alpha_local": alpha_local,
        "betting_fractions": betting_fractions,
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": order_commitment,
        "hard_gates": eligibility,
    }
    transcript_chain = hashlib.sha256(_canonical_json(header)).digest()
    looks = []

    for look_index, item in enumerate(items, start=1):
        scores = item["scores"]
        for candidate in policies:
            if not eligibility[candidate]["eligible"]:
                continue
            for reference in policies:
                if candidate == reference or not eligibility[reference]["eligible"]:
                    continue
                for axis in axes:
                    metric_id = axis["metric_id"]
                    difference = float(scores[candidate][metric_id]) - float(scores[reference][metric_id])
                    delta = axis["no_worse_tolerance"]
                    epsilon = axis["material_improvement_margin"]
                    z_no_worse = (difference + delta) / (1.0 + delta)
                    z_material = (difference - epsilon) / (1.0 + epsilon)
                    processes[(candidate, reference, metric_id, "no_worse")].update(z_no_worse)
                    processes[(candidate, reference, metric_id, "material_improvement")].update(z_material)

        pair_reports = {}
        certified_edges = []
        for candidate in policies:
            for reference in policies:
                if candidate == reference:
                    continue
                pair_key = f"{candidate}->{reference}"
                if not eligibility[candidate]["eligible"] or not eligibility[reference]["eligible"]:
                    pair_reports[pair_key] = {
                        "status": "INELIGIBLE",
                        "candidate_eligible": eligibility[candidate]["eligible"],
                        "reference_eligible": eligibility[reference]["eligible"],
                        "candidate_failures": eligibility[candidate]["failed_constraints"],
                        "reference_failures": eligibility[reference]["failed_constraints"],
                    }
                    continue

                no_worse = {}
                material = {}
                all_no_worse = True
                any_material = False
                for axis in axes:
                    metric_id = axis["metric_id"]
                    no_process = processes[(candidate, reference, metric_id, "no_worse")]
                    material_process = processes[(candidate, reference, metric_id, "material_improvement")]
                    no_worse[metric_id] = {
                        "certified": no_process.crossed,
                        "log_wealth": _logsumexp(no_process.log_gamblers) - math.log(len(betting_fractions)),
                        "max_log_wealth": no_process.max_log_wealth,
                        "tolerance": axis["no_worse_tolerance"],
                    }
                    material[metric_id] = {
                        "certified": material_process.crossed,
                        "log_wealth": _logsumexp(material_process.log_gamblers) - math.log(len(betting_fractions)),
                        "max_log_wealth": material_process.max_log_wealth,
                        "margin": axis["material_improvement_margin"],
                    }
                    all_no_worse = all_no_worse and no_process.crossed
                    any_material = any_material or material_process.crossed

                certified = all_no_worse and any_material
                status = "CERTIFIED_DOMINATES" if certified else "UNRESOLVED"
                pair_reports[pair_key] = {
                    "status": status,
                    "no_worse_by_metric": no_worse,
                    "material_improvement_by_metric": material,
                    "all_axes_noninferior": all_no_worse,
                    "at_least_one_material_improvement": any_material,
                }
                if certified:
                    certified_edges.append({"better": candidate, "worse": reference})

        dominated = {edge["worse"] for edge in certified_edges}
        frontier = {}
        for policy in policies:
            if not eligibility[policy]["eligible"]:
                status = "INELIGIBLE"
            elif policy in dominated:
                status = "CERTIFIED_DOMINATED"
            else:
                status = "POSSIBLY_NONDOMINATED"
            frontier[policy] = {
                "status": status,
                "failed_constraints": eligibility[policy]["failed_constraints"],
            }

        canonical_item = _canonical_json({
            "episode_id": item["episode_id"],
            "scores": {
                policy: {metric: float(scores[policy][metric]) for metric in metric_ids}
                for policy in sorted(eligible_policies)
            },
        })
        transcript_chain = hashlib.sha256(transcript_chain + canonical_item).digest()
        looks.append({
            "look": look_index,
            "episode_id": item["episode_id"],
            "pair_certificates": pair_reports,
            "certified_dominance_edges": certified_edges,
            "dominance_graph_has_cycle": _has_directed_cycle(policies, certified_edges),
            "inferential_frontier": frontier,
            "transcript_chain_sha256": transcript_chain.hex(),
        })

    qualified_provenance = {"MEASURED", "DERIVED_FROM_MEASUREMENT"}
    evidence_scope = (
        "MEASURED_AXES_ONLY"
        if all(axis["provenance"] in qualified_provenance for axis in axes)
        else "CONTAINS_SYNTHETIC_OR_UNMEASURED_AXES"
    )
    return {
        "benchmark_id": BENCHMARK_ID,
        "method_id": METHOD_ID,
        "protocol_id": protocol_id,
        "policies": policies,
        "axes": axes,
        "alpha": alpha,
        "alpha_local": alpha_local,
        "directional_test_count": test_count,
        "betting_fractions": betting_fractions,
        "sampling_model": "finite_cohort_random_order",
        "episode_count": n_items,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": order_commitment,
        "hard_gates": eligibility,
        "evidence_scope": evidence_scope,
        "score_transcript_sha256": transcript_chain.hex(),
        "looks": looks,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True)
    parser.add_argument("--output")
    args = parser.parse_args()
    try:
        with open(args.input, "r", encoding="utf-8") as source:
            document = json.load(source)
        result = evaluate_document(document)
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            with open(args.output, "w", encoding="utf-8") as destination:
                destination.write(rendered)
        else:
            print(rendered, end="")
    except (OSError, json.JSONDecodeError, ParetoEvaluationError) as exc:
        print(json.dumps({"status": "INVALID", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2) from exc


if __name__ == "__main__":
    main()

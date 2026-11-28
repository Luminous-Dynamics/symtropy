#!/usr/bin/env python3
"""Independent standard-library challenger for the synthetic EVSI contract.

This script is a test oracle only. Production selector semantics are implemented
in Rust. Integer EVSI arithmetic is re-derived independently from the scenario.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

DENOMINATOR = 10_000
JOINT_DENOMINATOR = DENOMINATOR * DENOMINATOR
MAX_LOSS = 1_000_000


def fail(message: str) -> None:
    raise ValueError(message)


def validate(scenario: dict[str, Any]) -> None:
    if scenario.get("schema_version") != 1:
        fail("unsupported schema_version")
    if not isinstance(scenario.get("scenario_id"), str) or not scenario["scenario_id"].strip():
        fail("scenario_id must not be empty")
    if not isinstance(scenario.get("mission"), str) or not scenario["mission"].strip():
        fail("mission must not be empty")
    if scenario.get("probability_denominator") != DENOMINATOR:
        fail("probability_denominator must be 10000")

    hypotheses = scenario.get("hypotheses")
    decisions = scenario.get("decisions")
    priors = scenario.get("prior_probability_bps")
    losses = scenario.get("decision_loss_units")
    proposals = scenario.get("proposals")
    if not isinstance(hypotheses, list) or not 2 <= len(hypotheses) <= 64:
        fail("hypotheses must contain 2..=64 entries")
    if not isinstance(decisions, list) or not 2 <= len(decisions) <= 32:
        fail("decisions must contain 2..=32 entries")
    if not isinstance(proposals, list) or not 1 <= len(proposals) <= 128:
        fail("proposals must contain 1..=128 entries")
    if not isinstance(priors, list) or len(priors) != len(hypotheses):
        fail("prior vector length mismatch")
    if sum(priors) != DENOMINATOR or any(not isinstance(x, int) or x < 0 for x in priors):
        fail("prior probabilities must be non-negative integers summing to 10000")

    def unique_ids(items: list[dict[str, Any]], label: str) -> None:
        ids = [item.get("id") for item in items]
        if any(not isinstance(item, str) or not item.strip() for item in ids):
            fail(f"{label} ids must be non-empty strings")
        if len(set(ids)) != len(ids):
            fail(f"duplicate {label} id")

    unique_ids(hypotheses, "hypothesis")
    unique_ids(decisions, "decision")
    unique_ids(proposals, "proposal")

    if not isinstance(losses, list) or len(losses) != len(hypotheses):
        fail("decision loss matrix row count mismatch")
    for row in losses:
        if not isinstance(row, list) or len(row) != len(decisions):
            fail("decision loss matrix column count mismatch")
        if any(not isinstance(x, int) or x < 0 or x > MAX_LOSS for x in row):
            fail("decision loss values must be in 0..=1000000")

    for proposal in proposals:
        if not isinstance(proposal.get("cost_units"), int) or not 0 <= proposal["cost_units"] <= MAX_LOSS:
            fail("proposal cost is outside bounds")
        if proposal.get("max_latency_us", 0) <= 0 or proposal.get("estimated_latency_us", 0) <= 0:
            fail("proposal latency bounds must be non-zero")
        outcomes = proposal.get("outcomes")
        if not isinstance(outcomes, list) or not 1 <= len(outcomes) <= 64:
            fail("proposal outcome count must be in 1..=64")
        unique_ids(outcomes, "outcome")
        sums = [0] * len(hypotheses)
        for outcome in outcomes:
            likelihoods = outcome.get("likelihood_bps")
            if not isinstance(likelihoods, list) or len(likelihoods) != len(hypotheses):
                fail("likelihood vector length mismatch")
            for index, value in enumerate(likelihoods):
                if not isinstance(value, int) or not 0 <= value <= DENOMINATOR:
                    fail("likelihood entries must be in 0..=10000")
                sums[index] += value
        if any(value != DENOMINATOR for value in sums):
            fail("likelihood columns must each sum to 10000")


def best_decision(scenario: dict[str, Any], weights: list[int]) -> tuple[int, int]:
    best_index = 0
    best_numerator: int | None = None
    for decision_index in range(len(scenario["decisions"])):
        numerator = sum(
            weight * scenario["decision_loss_units"][hypothesis_index][decision_index]
            for hypothesis_index, weight in enumerate(weights)
        )
        if best_numerator is None or numerator < best_numerator:
            best_index = decision_index
            best_numerator = numerator
    assert best_numerator is not None
    return best_index, best_numerator


def entropy_bits(probabilities: list[float]) -> float:
    return -sum(p * math.log2(p) for p in probabilities if p > 0.0)


def entropy_gain_millibits(scenario: dict[str, Any], proposal: dict[str, Any]) -> int:
    priors = [value / DENOMINATOR for value in scenario["prior_probability_bps"]]
    prior_entropy = entropy_bits(priors)
    conditional_entropy = 0.0
    for outcome in proposal["outcomes"]:
        joint = [
            prior * likelihood
            for prior, likelihood in zip(scenario["prior_probability_bps"], outcome["likelihood_bps"], strict=True)
        ]
        outcome_weight = sum(joint)
        if outcome_weight == 0:
            continue
        posterior = [weight / outcome_weight for weight in joint]
        conditional_entropy += (
            outcome_weight / JOINT_DENOMINATOR
        ) * entropy_bits(posterior)
    # Match Rust f64::round for non-negative values (half away from zero).
    return math.floor(max(0.0, prior_entropy - conditional_entropy) * 1000.0 + 0.5)


def reject_reason(proposal: dict[str, Any]) -> str | None:
    if not proposal["advisory_only"]:
        return "proposal must be advisory-only"
    if proposal["actuation_authority"]:
        return "proposal must not carry actuation authority"
    if not proposal["capability_available"]:
        return "required sensing capability is unavailable"
    if not proposal["preconditions_met"]:
        return "proposal preconditions are not satisfied"
    if not proposal["dependencies_known"]:
        return "proposal observation-model dependencies are unknown"
    if proposal["estimated_latency_us"] > proposal["max_latency_us"]:
        return "proposal exceeds its declared latency budget"
    return None


def evaluate(scenario: dict[str, Any]) -> dict[str, Any]:
    validate(scenario)
    priors = scenario["prior_probability_bps"]
    prior_best_index, prior_best_numerator = best_decision(scenario, priors)
    evaluations: list[dict[str, Any]] = []

    for proposal in scenario["proposals"]:
        reason = reject_reason(proposal)
        if reason is not None:
            evaluations.append({
                "proposal_id": proposal["id"],
                "disposition": "rejected",
                "rejection_reason": reason,
                "prior_best_decision_id": scenario["decisions"][prior_best_index]["id"],
                "gross_evsi_numerator": None,
                "cost_numerator": None,
                "net_evsi_numerator": None,
                "denominator": JOINT_DENOMINATOR,
                "entropy_gain_millibits": None,
            })
            continue

        posterior_loss_sum = 0
        for outcome in proposal["outcomes"]:
            joint_weights = [
                prior * likelihood
                for prior, likelihood in zip(priors, outcome["likelihood_bps"], strict=True)
            ]
            if sum(joint_weights) == 0:
                continue
            _, outcome_minimum = best_decision(scenario, joint_weights)
            posterior_loss_sum += outcome_minimum

        prior_scaled = prior_best_numerator * DENOMINATOR
        gross = max(0, prior_scaled - posterior_loss_sum)
        cost_numerator = proposal["cost_units"] * JOINT_DENOMINATOR
        net = gross - cost_numerator
        evaluations.append({
            "proposal_id": proposal["id"],
            "disposition": "admissible_positive_evsi" if net > 0 else "admissible_non_positive_evsi",
            "rejection_reason": None,
            "prior_best_decision_id": scenario["decisions"][prior_best_index]["id"],
            "gross_evsi_numerator": gross,
            "cost_numerator": cost_numerator,
            "net_evsi_numerator": net,
            "denominator": JOINT_DENOMINATOR,
            "entropy_gain_millibits": entropy_gain_millibits(scenario, proposal),
        })

    positive = [item for item in evaluations if item["disposition"] == "admissible_positive_evsi"]
    evsi_choice = None
    for item in positive:
        if evsi_choice is None or item["net_evsi_numerator"] >= evsi_choice["net_evsi_numerator"]:
            evsi_choice = item

    admissible = [item for item in evaluations if item["disposition"] != "rejected"]
    entropy_choice = None
    for item in admissible:
        if entropy_choice is None or item["entropy_gain_millibits"] >= entropy_choice["entropy_gain_millibits"]:
            entropy_choice = item

    return {
        "schema_version": 1,
        "scenario_id": scenario["scenario_id"],
        "mission": scenario["mission"],
        "prior_best_decision_id": scenario["decisions"][prior_best_index]["id"],
        "prior_expected_loss_numerator": prior_best_numerator,
        "prior_loss_denominator": DENOMINATOR,
        "evsi_recommendation_id": evsi_choice["proposal_id"] if evsi_choice else None,
        "information_gain_recommendation_id": entropy_choice["proposal_id"] if entropy_choice else None,
        "evaluations": evaluations,
        "observer_only": True,
        "hardware_action_issued": False,
        "actuation_authority": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--expect-evsi")
    parser.add_argument("--expect-information-gain")
    args = parser.parse_args()
    try:
        scenario = json.loads(Path(args.scenario).read_text(encoding="utf-8"))
        report = evaluate(scenario)
        print(json.dumps(report, indent=2, ensure_ascii=False))
        if args.expect_evsi is not None and report["evsi_recommendation_id"] != args.expect_evsi:
            fail(f"EVSI recommendation mismatch: expected {args.expect_evsi}, got {report['evsi_recommendation_id']}")
        if (
            args.expect_information_gain is not None
            and report["information_gain_recommendation_id"] != args.expect_information_gain
        ):
            fail(
                "information-gain recommendation mismatch: "
                f"expected {args.expect_information_gain}, got {report['information_gain_recommendation_id']}"
            )
        return 0
    except (OSError, json.JSONDecodeError, ValueError, KeyError, TypeError) as error:
        print(f"cfi-space-evsi-challenger: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Independent CI-only oracle and schema checks for AERO EVSI v0.1.

This script is not part of the production AERO/Symthaea path. It deliberately
recomputes the small published fixture using Python integer arithmetic so the
Rust kernel is not its own only test oracle.
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
FIXTURE_PATH = AVIATION / "examples" / "aero_evidence_action_planner_v0_1.json"
SCHEMA_PATH = AVIATION / "schemas" / "aero_evidence_action_planner_v0_1.schema.json"
PPM = 1_000_000
SCALE = PPM * PPM


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"FAIL: cannot load {path.relative_to(ROOT)}: {exc}") from exc


def best_decision(scores: dict[str, int]) -> tuple[str, int]:
    if not scores:
        raise ValueError("decision set is empty")
    decision_id = min(scores, key=lambda key: (-scores[key], key))
    return decision_id, scores[decision_id]


def validate_input(source: dict[str, Any]) -> None:
    if sum(state["prior_probability_ppm"] for state in source["world_states"]) != PPM:
        raise ValueError("prior probabilities do not sum to one")
    decisions = [item["decision_id"] for item in source["decision_alternatives"]]
    states = [item["state_id"] for item in source["world_states"]]
    if len(decisions) < 2 or len(set(decisions)) != len(decisions):
        raise ValueError("decision alternatives are not a unique set of size >= 2")
    if len(set(states)) != len(states) or not states:
        raise ValueError("world states are not a non-empty unique set")

    utility_keys = [(item["decision_id"], item["state_id"]) for item in source["utilities"]]
    expected_utility_keys = {(decision, state) for decision in decisions for state in states}
    if len(utility_keys) != len(set(utility_keys)) or set(utility_keys) != expected_utility_keys:
        raise ValueError("utility matrix is not a complete unique decision x state grid")

    action_ids = [action["action_id"] for action in source["actions"]]
    if len(action_ids) != len(set(action_ids)):
        raise ValueError("action IDs are not unique")
    for action in source["actions"]:
        authorization = action["authorization_receipt"]
        if action["authorization_status"] == "VERIFIED_PERMITTED" and authorization is None:
            raise ValueError(f"{action['action_id']}: verified action is missing its permission receipt")
        if authorization is not None and authorization["effective_from_unix_ms"] >= authorization["effective_until_unix_ms"]:
            raise ValueError(f"{action['action_id']}: malformed permission receipt interval")
        calibration = action["calibration_receipt"]
        if action["calibration_status"] == "QUALIFIED_FOR_DECLARED_PURPOSE" and calibration is None:
            raise ValueError(f"{action['action_id']}: qualified outcome model is missing its receipt")
        if calibration is not None and calibration["effective_from_unix_ms"] >= calibration["effective_until_unix_ms"]:
            raise ValueError(f"{action['action_id']}: malformed calibration receipt interval")
        keys = [(item["state_id"], item["outcome_id"]) for item in action["outcome_likelihoods"]]
        outcome_ids = {item["outcome_id"] for item in action["outcome_likelihoods"]}
        expected = {(state, outcome) for state in states for outcome in outcome_ids}
        if len(keys) != len(set(keys)) or set(keys) != expected:
            raise ValueError(f"{action['action_id']}: outcome likelihood grid is incomplete or duplicated")
        sums = {state: 0 for state in states}
        for item in action["outcome_likelihoods"]:
            if item["likelihood_ppm"] < 0 or item["likelihood_ppm"] > PPM:
                raise ValueError(f"{action['action_id']}: likelihood outside PPM range")
            sums[item["state_id"]] += item["likelihood_ppm"]
        if any(value != PPM for value in sums.values()):
            raise ValueError(f"{action['action_id']}: outcome likelihoods do not sum to one per state")


def calculate(source: dict[str, Any]) -> dict[str, Any]:
    validate_input(source)
    decisions = sorted(item["decision_id"] for item in source["decision_alternatives"])
    states = {item["state_id"]: item["prior_probability_ppm"] for item in source["world_states"]}
    utilities = {
        (item["decision_id"], item["state_id"]): item["utility_units"]
        for item in source["utilities"]
    }
    prior_scores = {
        decision: sum(states[state] * utilities[(decision, state)] for state in states)
        for decision in decisions
    }
    current_best, current_score = best_decision(prior_scores)

    ranked = []
    excluded = []
    for action in source["actions"]:
        auth = action["authorization_status"]
        calibration = action["calibration_status"]
        authorization = action["authorization_receipt"]
        calibration_receipt = action["calibration_receipt"]
        authorization_reason = None
        if auth == "DENIED":
            authorization_reason = "ACTION_AUTHORITY_DENIED"
        elif auth == "UNKNOWN":
            authorization_reason = "ACTION_AUTHORITY_UNKNOWN"
        elif auth != "VERIFIED_PERMITTED":
            raise ValueError(f"{action['action_id']}: unsupported authority state")
        elif (
            authorization["owner_namespace"] != action["owner_namespace"]
            or authorization["claim_id"] != source["claim_id"]
            or authorization["decision_context_sha256"] != source["decision_context_sha256"]
            or authorization["configuration_ref"] != source["configuration_ref"]
            or authorization["action_id"] != action["action_id"]
            or authorization["action_catalogue_entry_sha256"] != action["action_catalogue_entry_sha256"]
        ):
            authorization_reason = "ACTION_AUTHORIZATION_SCOPE_MISMATCH"
        elif authorization["receiver_first_seen_at_unix_ms"] > source["as_known_at_unix_ms"]:
            authorization_reason = "ACTION_AUTHORIZATION_NOT_KNOWN_AT_CUTOFF"
        elif (
            authorization["effective_from_unix_ms"] > source["as_known_at_unix_ms"]
            or authorization["effective_until_unix_ms"] <= source["as_known_at_unix_ms"]
        ):
            authorization_reason = "ACTION_AUTHORIZATION_NOT_EFFECTIVE_AT_CUTOFF"

        calibration_reason = None
        if calibration == "UNVERIFIED":
            calibration_reason = "ACTION_OUTCOME_MODEL_UNVERIFIED"
        elif calibration == "OUT_OF_DOMAIN":
            calibration_reason = "ACTION_OUTCOME_MODEL_OUT_OF_DOMAIN"
        elif calibration == "NOT_ESTIMABLE":
            calibration_reason = "ACTION_OUTCOME_MODEL_NOT_ESTIMABLE"
        elif calibration != "QUALIFIED_FOR_DECLARED_PURPOSE":
            raise ValueError(f"{action['action_id']}: unsupported calibration state")
        elif (
            calibration_receipt["claim_id"] != source["claim_id"]
            or calibration_receipt["decision_context_sha256"] != source["decision_context_sha256"]
            or calibration_receipt["configuration_ref"] != source["configuration_ref"]
            or calibration_receipt["action_id"] != action["action_id"]
            or calibration_receipt["outcome_model_sha256"] != source["outcome_model_sha256"]
        ):
            calibration_reason = "ACTION_CALIBRATION_SCOPE_MISMATCH"
        elif calibration_receipt["receiver_first_seen_at_unix_ms"] > source["as_known_at_unix_ms"]:
            calibration_reason = "ACTION_CALIBRATION_NOT_KNOWN_AT_CUTOFF"
        elif (
            calibration_receipt["effective_from_unix_ms"] > source["as_known_at_unix_ms"]
            or calibration_receipt["effective_until_unix_ms"] <= source["as_known_at_unix_ms"]
        ):
            calibration_reason = "ACTION_CALIBRATION_NOT_EFFECTIVE_AT_CUTOFF"

        exclusion_reason = authorization_reason or calibration_reason
        if exclusion_reason is not None:
            excluded.append({"action_id": action["action_id"], "reason_code": exclusion_reason})
            continue

        likelihoods = {
            (item["state_id"], item["outcome_id"]): item["likelihood_ppm"]
            for item in action["outcome_likelihoods"]
        }
        outcomes = sorted({item["outcome_id"] for item in action["outcome_likelihoods"]})
        posterior_value_scaled = 0
        for outcome in outcomes:
            outcome_scores = {
                decision: sum(
                    states[state] * likelihoods[(state, outcome)] * utilities[(decision, state)]
                    for state in states
                )
                for decision in decisions
            }
            _, best_score = best_decision(outcome_scores)
            posterior_value_scaled += best_score
        evsi_scaled = posterior_value_scaled - current_score * PPM
        if evsi_scaled < 0:
            raise ValueError(f"{action['action_id']}: negative EVSI under a normalized model")
        total_cost = action["expected_cost_units"] + action["expected_delay_cost_units"]
        net_scaled = evsi_scaled - total_cost * SCALE
        ranked.append({
            "action_id": action["action_id"],
            "owner_namespace": action["owner_namespace"],
            "target_obligation_ids": action["target_obligation_ids"],
            "action_catalogue_entry_sha256": action["action_catalogue_entry_sha256"],
            "authorization_receipt_sha256": action["authorization_receipt_sha256"],
            "expected_artifact_contract_sha256": action["expected_artifact_contract_sha256"],
            "evsi_scaled_numerator": str(evsi_scaled),
            "expected_cost_units": action["expected_cost_units"],
            "expected_delay_cost_units": action["expected_delay_cost_units"],
            "net_benefit_scaled_numerator": str(net_scaled),
            "_net": net_scaled,
            "_evsi": evsi_scaled,
        })

    ranked.sort(key=lambda item: (
        -item["_net"],
        -item["_evsi"],
        item["expected_cost_units"],
        item["expected_delay_cost_units"],
        item["action_id"],
    ))
    for rank, item in enumerate(ranked, 1):
        item["rank"] = rank
        item.pop("_net")
        item.pop("_evsi")
    excluded.sort(key=lambda item: item["action_id"])
    selected = ranked[0]["action_id"] if ranked and int(ranked[0]["net_benefit_scaled_numerator"]) > 0 else None
    if selected is not None:
        disposition = "RANKED_ACTIONS"
    elif not ranked:
        disposition = "ABSTAIN_NO_QUALIFIED_PERMITTED_ACTIONS"
    else:
        disposition = "NO_POSITIVE_NET_BENEFIT"

    return {
        "disposition": disposition,
        "current_best_decision_id": current_best,
        "selected_action_id": selected,
        "ranked_actions": ranked,
        "excluded_actions": excluded,
        "frozen_claim_disposition": source["frozen_claim_disposition"],
        "assurance_state_mutated": False,
        "physical_work_authorized": False,
        "engineering_acceptance_granted": False,
        "evsi_numerator_scale": SCALE,
    }


def main() -> int:
    fixture = load_json(FIXTURE_PATH)
    schema = load_json(SCHEMA_PATH)
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(fixture),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if errors:
        for error in errors:
            print(f"FAIL: schema at /{'/'.join(map(str, error.absolute_path))}: {error.message}")
        return 1

    source = fixture["input"]
    actual = calculate(source)
    expected = fixture["expected"]
    for field, expected_value in expected.items():
        if field == "ranked_actions":
            if len(actual[field]) != len(expected_value):
                raise SystemExit("FAIL: ranked action count differs from fixture")
            for actual_action, expected_action in zip(actual[field], expected_value):
                for key, value in expected_action.items():
                    if actual_action.get(key) != value:
                        raise SystemExit(
                            f"FAIL: ranked action {actual_action.get('action_id')} "
                            f"field {key} expected {value!r}, got {actual_action.get(key)!r}"
                        )
        elif actual[field] != expected_value:
            raise SystemExit(
                f"FAIL: expected {field}={expected_value!r}, got {actual[field]!r}"
            )

    # Mutation control 1: exact information with zero decision utility is not valuable.
    irrelevant = copy.deepcopy(source)
    for utility in irrelevant["utilities"]:
        utility["utility_units"] = 0
    irrelevant_result = calculate(irrelevant)
    if irrelevant_result["ranked_actions"][0]["evsi_scaled_numerator"] != "0":
        raise SystemExit("FAIL: information with no decision utility was assigned positive EVSI")
    if irrelevant_result["selected_action_id"] is not None:
        raise SystemExit("FAIL: zero-value action was recommended despite positive acquisition cost")

    # Mutation control 2: unknown permission must exclude an otherwise attractive action.
    unauthorized = copy.deepcopy(source)
    unauthorized["actions"][0]["authorization_status"] = "UNKNOWN"
    unauthorized_result = calculate(unauthorized)
    if unauthorized_result["ranked_actions"]:
        raise SystemExit("FAIL: unknown-permission action was ranked")
    if unauthorized_result["selected_action_id"] is not None:
        raise SystemExit("FAIL: unknown-permission action was selected")

    # Mutation control 3: probability mass is never silently normalized.
    malformed = copy.deepcopy(source)
    malformed["world_states"][0]["prior_probability_ppm"] -= 1
    try:
        calculate(malformed)
    except ValueError:
        pass
    else:
        raise SystemExit("FAIL: malformed prior probability mass was silently accepted")

    print(
        "PASS: AERO EVSI v0.1 schema and independent integer oracle "
        "(2 states, 2 decisions, 1 permitted action, 1 excluded action, "
        "3 mutation controls)."
    )
    print("NOTE: synthetic decision arithmetic is not physical evidence or action authorization.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

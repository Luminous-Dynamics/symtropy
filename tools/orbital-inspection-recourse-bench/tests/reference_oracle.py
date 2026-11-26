#!/usr/bin/env python3
"""Independent standard-library exhaustive oracle for inspection recourse v0.1."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-inspection-recourse.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-inspection-recourse.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-INSPECTION-RECOURSE-001"
CLAIM_CEILING = "synthetic_finite_contingent_policy_over_declared_outcome_support_only"


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def validate_case(case):
    if (
        not valid_text(case["case_id"])
        or not case["worlds"]
        or case["max_search_states"] <= 0
        or case["max_actions"] > 32
        or len(case["worlds"]) > 64
        or len(case["actions"]) > 64
    ):
        raise ValueError("CASE_LIMIT_OR_REQUIRED_COLLECTION_INVALID")

    world_ids = set()
    for world in case["worlds"]:
        if (
            not valid_text(world["world_id"])
            or not valid_text(world["decision_class"])
            or world["world_id"] in world_ids
        ):
            raise ValueError("WORLD_ID_OR_DECISION_INVALID")
        world_ids.add(world["world_id"])

    action_ids = set()
    for action in case["actions"]:
        if (
            not valid_text(action["action_id"])
            or action["action_id"] in action_ids
            or not valid_sha256(action["model_receipt_sha256"])
            or not action["gates"]
        ):
            raise ValueError("ACTION_ID_OR_MODEL_RECEIPT_INVALID")
        action_ids.add(action["action_id"])

        gate_ids = set()
        for gate in action["gates"]:
            if (
                not valid_text(gate["gate_id"])
                or not valid_sha256(gate["receipt_sha256"])
                or gate["gate_id"] in gate_ids
            ):
                raise ValueError("ACTION_GATE_EVIDENCE_INVALID")
            gate_ids.add(gate["gate_id"])

        mapped_ids = set()
        for mapping in action["outcomes_by_world"]:
            world_id = mapping["world_id"]
            if (
                world_id not in world_ids
                or world_id in mapped_ids
                or not valid_sha256(mapping["receipt_sha256"])
            ):
                raise ValueError("ACTION_WORLD_MAPPING_INVALID")
            mapped_ids.add(world_id)

            support = mapping["possible_outcomes"]
            if mapping["mapping_state"] == "KNOWN":
                if not support or any(not valid_text(item) for item in support):
                    raise ValueError("KNOWN_OUTCOME_SUPPORT_INVALID")
                if len(set(support)) != len(support):
                    raise ValueError("KNOWN_OUTCOME_SUPPORT_INVALID")
            elif mapping["mapping_state"] == "UNKNOWN":
                if support:
                    raise ValueError("UNKNOWN_MAPPING_MUST_NOT_ASSERT_OUTCOMES")
            else:
                raise ValueError("ACTION_WORLD_MAPPING_INVALID")

        if mapped_ids != world_ids:
            raise ValueError("ACTION_WORLD_MAPPING_INCOMPLETE")


def decision_classes(case, survivors):
    return {
        world["decision_class"]
        for world in case["worlds"]
        if world["world_id"] in survivors
    }


def action_rejections(case):
    rejected = {}
    for action in case["actions"]:
        reasons = set()
        for gate in action["gates"]:
            if gate["state"] == "FAIL":
                reasons.add("GATE_FAIL:" + gate["gate_id"])
            elif gate["state"] == "UNKNOWN":
                reasons.add("GATE_UNKNOWN:" + gate["gate_id"])
        if any(
            mapping["mapping_state"] == "UNKNOWN"
            for mapping in action["outcomes_by_world"]
        ):
            reasons.add("OUTCOME_MODEL_UNKNOWN")
        if reasons:
            rejected[action["action_id"]] = sorted(reasons)
    return {key: rejected[key] for key in sorted(rejected)}


def action_is_admissible(action):
    return (
        all(gate["state"] == "PASS" for gate in action["gates"])
        and all(
            mapping["mapping_state"] == "KNOWN"
            for mapping in action["outcomes_by_world"]
        )
    )


def solve_state(
    case,
    survivors,
    used_actions,
    remaining_time,
    remaining_energy,
    remaining_actions,
    counter,
):
    if counter["cap_hit"]:
        return None
    counter["states"] += 1
    if counter["states"] > case["max_search_states"]:
        counter["cap_hit"] = True
        return None

    decisions = decision_classes(case, survivors)
    if len(decisions) == 1:
        decision = min(decisions)
        return {
            "node": {
                "terminal_decision": decision,
                "action_id": None,
                "survivor_world_count": len(survivors),
                "action_evidence_sha256": [],
                "branches": [],
            },
            "objective": (0, 0, 0),
        }
    if not decisions or remaining_actions == 0:
        return None

    best = None
    for action in sorted(case["actions"], key=lambda item: item["action_id"]):
        action_id = action["action_id"]
        if (
            action_id in used_actions
            or not action_is_admissible(action)
            or action["duration_ms"] > remaining_time
            or action["energy_mj"] > remaining_energy
        ):
            continue

        outcomes = {}
        for mapping in action["outcomes_by_world"]:
            if mapping["world_id"] not in survivors:
                continue
            for outcome in mapping["possible_outcomes"]:
                outcomes.setdefault(outcome, set()).add(mapping["world_id"])

        if len(outcomes) < 2 or all(
            branch_worlds == survivors for branch_worlds in outcomes.values()
        ):
            continue

        next_used = set(used_actions)
        next_used.add(action_id)
        branches = []
        worst_child_time = 0
        worst_child_energy = 0
        worst_child_actions = 0
        complete = True

        for outcome in sorted(outcomes):
            child = solve_state(
                case,
                outcomes[outcome],
                next_used,
                remaining_time - action["duration_ms"],
                remaining_energy - action["energy_mj"],
                remaining_actions - 1,
                counter,
            )
            if child is None:
                complete = False
                break
            child_time, child_energy, child_actions = child["objective"]
            worst_child_time = max(worst_child_time, child_time)
            worst_child_energy = max(worst_child_energy, child_energy)
            worst_child_actions = max(worst_child_actions, child_actions)
            branches.append({"outcome": outcome, "child": child["node"]})

        if counter["cap_hit"]:
            return None
        if not complete or len(branches) < 2:
            continue

        evidence = {action["model_receipt_sha256"]}
        evidence.update(gate["receipt_sha256"] for gate in action["gates"])
        evidence.update(
            mapping["receipt_sha256"]
            for mapping in action["outcomes_by_world"]
            if mapping["world_id"] in survivors
        )

        objective = (
            action["duration_ms"] + worst_child_time,
            action["energy_mj"] + worst_child_energy,
            1 + worst_child_actions,
        )
        candidate = {
            "node": {
                "terminal_decision": None,
                "action_id": action_id,
                "survivor_world_count": len(survivors),
                "action_evidence_sha256": sorted(evidence),
                "branches": branches,
            },
            "objective": objective,
        }
        # Actions are visited in canonical ID order. Strict comparison preserves
        # the first canonical policy when objective vectors tie.
        if best is None or objective < best["objective"]:
            best = candidate
    return best


def evaluate_case(case):
    validate_case(case)
    survivors = {world["world_id"] for world in case["worlds"]}
    initial_decisions = decision_classes(case, survivors)
    counter = {"states": 0, "cap_hit": False}
    solution = solve_state(
        case,
        survivors,
        set(),
        case["budget_time_ms"],
        case["budget_energy_mj"],
        case["max_actions"],
        counter,
    )

    if counter["cap_hit"]:
        status = "SEARCH_LIMIT_NOT_PROVEN"
        exactness_proven = False
        policy = None
    elif solution is not None:
        status = "RESOLVED"
        exactness_proven = True
        duration, energy, actions = solution["objective"]
        policy = {
            "root": solution["node"],
            "objective": {
                "worst_case_duration_ms": duration,
                "worst_case_energy_mj": energy,
                "worst_case_action_count": actions,
            },
        }
    else:
        status = "ABSTAIN"
        exactness_proven = True
        policy = None

    root_action = (
        policy["root"]["action_id"] if policy is not None else None
    )
    result = {
        "case_id": case["case_id"],
        "status": status,
        "initial_world_count": len(survivors),
        "initial_decision_class_count": len(initial_decisions),
        "search_states_expanded": counter["states"],
        "exactness_proven": exactness_proven,
        "rejected_actions": action_rejections(case),
        "policy": policy,
    }
    return result, root_action


def audit_corpus(raw_bytes, source_commit):
    if (
        len(source_commit) != 40
        or any(character not in "0123456789abcdefABCDEF" for character in source_commit)
    ):
        raise ValueError("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw_bytes)
    if (
        corpus["schema"] != CORPUS_SCHEMA
        or corpus["benchmark_id"] != BENCHMARK_ID
        or corpus["evidence_class"] != "synthetic"
        or corpus["claim_ceiling"] != CLAIM_CEILING
        or not corpus["cases"]
        or len(corpus["cases"]) > 128
    ):
        raise ValueError("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")

    seen_case_ids = set()
    results = []
    expected_matches = 0
    for case in corpus["cases"]:
        if case["case_id"] in seen_case_ids:
            raise ValueError("CASE_ID_DUPLICATE")
        seen_case_ids.add(case["case_id"])
        result, root_action = evaluate_case(case)
        if (
            result["status"] == case["expected_status"]
            and root_action == case["expected_root_action"]
        ):
            expected_matches += 1
        results.append(result)

    return {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "evidence_class": corpus["evidence_class"],
        "claim_ceiling": corpus["claim_ceiling"],
        "case_count": len(results),
        "expected_outcome_matches": expected_matches,
        "expected_outcome_mismatches": len(results) - expected_matches,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": results,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit(
            "usage: reference_oracle.py <corpus.json> <40-hex-source-commit>"
        )
    raw_bytes = Path(sys.argv[1]).read_bytes()
    receipt = audit_corpus(raw_bytes, sys.argv[2])
    print(json.dumps(receipt, indent=2))
    if receipt["expected_outcome_mismatches"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

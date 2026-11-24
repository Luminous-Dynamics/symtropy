#!/usr/bin/env python3
"""Exact finite-distribution CVaR oracle for probability-qualified policy regret.

This is a research/reference oracle only. It does not authenticate a probability
model, prove calibration, infer probabilities from scenario counts, or authorize
a policy change. Missing/unqualified probability evidence causes ABSTAIN.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from fractions import Fraction
from pathlib import Path
from typing import Any

CORPUS_SCHEMA = "luminous.orbital-cvar-regret.corpus.v0.1"
MODEL_SCHEMA = "luminous.orbital-probability-model.receipt.v0.1"
CLAIM_CEILING = "probability-conditioned-synthetic-tail-risk-algebra-only"
MODEL_CLAIM_CEILING = "explicit-joint-probability-model-receipt-only"
SHA_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
MAX_COMPONENT = 10**12


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def digest_json(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def valid_id(value: Any) -> bool:
    return (
        isinstance(value, str)
        and bool(value.strip())
        and all(32 <= ord(char) <= 126 for char in value)
    )


def valid_sha(value: Any) -> bool:
    return isinstance(value, str) and SHA_RE.fullmatch(value) is not None


def exact_fraction(value: Any, *, allow_zero: bool = True) -> Fraction | None:
    if not isinstance(value, dict) or set(value) != {"numerator", "denominator"}:
        return None
    numerator, denominator = value.get("numerator"), value.get("denominator")
    if (
        type(numerator) is not int
        or type(denominator) is not int
        or numerator < (0 if allow_zero else 1)
        or denominator < 1
        or numerator > MAX_COMPONENT
        or denominator > MAX_COMPONENT
    ):
        return None
    return Fraction(numerator, denominator)


def fraction_json(value: Fraction) -> dict[str, str]:
    return {
        "numerator": str(value.numerator),
        "denominator": str(value.denominator),
    }


def canonical_weight_rows(weights: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for row in sorted(weights, key=lambda item: item["scenario_id"]):
        rows.append({
            "scenario_id": row["scenario_id"],
            "weight": {
                "numerator": row["weight"]["numerator"],
                "denominator": row["weight"]["denominator"],
            },
        })
    return rows


def probability_payload_digest(weights: list[dict[str, Any]]) -> str:
    return digest_json(canonical_weight_rows(weights))


def canonical_joint_rows(rows: list[dict[str, Any]]) -> list[dict[str, str]]:
    return [
        {
            "scenario_id": row["scenario_id"],
            "world_id": row["world_id"],
            "loss_model_id": row["loss_model_id"],
        }
        for row in sorted(rows, key=lambda item: item["scenario_id"])
    ]


def var_threshold(losses: list[tuple[Fraction, Fraction]], alpha: Fraction) -> Fraction:
    """Lower alpha-quantile; ties handled as one atom."""
    cumulative = Fraction(0, 1)
    for loss in sorted({loss for loss, _ in losses}):
        cumulative += sum((weight for value, weight in losses if value == loss), Fraction(0, 1))
        if cumulative >= alpha:
            return loss
    raise ValueError("probability distribution has no positive-mass support")


def cvar_upper_tail(losses: list[tuple[Fraction, Fraction]], alpha: Fraction) -> dict[str, Fraction]:
    """Exact upper-tail CVaR using the Rockafellar-Uryasev variational formula.

    CVaR_alpha(L) = min_eta [eta + E[(L-eta)_+] / (1-alpha)].
    The minimum of this piecewise-linear convex objective is attained at a
    support point, so checking each unique loss is exact for finite support.
    """
    if not 0 < alpha < 1:
        raise ValueError("CVaR alpha must be strictly between zero and one")
    if not losses or sum((weight for _, weight in losses), Fraction(0, 1)) != 1:
        raise ValueError("CVaR input weights must form a normalized nonempty distribution")

    candidates = sorted({loss for loss, _ in losses})
    objective = [
        eta + sum(
            (weight * max(loss - eta, Fraction(0, 1)) for loss, weight in losses),
            Fraction(0, 1),
        ) / (1 - alpha)
        for eta in candidates
    ]
    cvar = min(objective)
    var = var_threshold(losses, alpha)
    tail_mass = 1 - alpha
    mass_above = sum((weight for loss, weight in losses if loss > var), Fraction(0, 1))
    mass_at_var = sum((weight for loss, weight in losses if loss == var), Fraction(0, 1))
    required_at_var = tail_mass - mass_above
    atom_fraction = required_at_var / mass_at_var if mass_at_var else Fraction(0, 1)
    if not 0 <= atom_fraction <= 1:
        raise ValueError("tail atom accounting is inconsistent")
    return {
        "var": var,
        "cvar": cvar,
        "tail_mass": tail_mass,
        "var_atom_fraction": atom_fraction,
    }


def invalid(reason: str) -> dict[str, Any]:
    return {
        "status": "INVALID_INPUT",
        "reason": reason,
        "authority_granted": False,
    }


def abstain(reason: str) -> dict[str, Any]:
    return {
        "status": "ABSTAIN",
        "reason": reason,
        "authority_granted": False,
    }


def validate_receipt(case: dict[str, Any]) -> list[tuple[str, Fraction]]:
    receipt = case.get("probability_model")
    if receipt is None:
        raise LookupError("PROBABILITY_MODEL_RECEIPT_ABSENT")
    if not isinstance(receipt, dict):
        raise ValueError("probability model receipt must be an object")

    if (
        receipt.get("receipt_schema") != MODEL_SCHEMA
        or receipt.get("claim_ceiling") != MODEL_CLAIM_CEILING
        or not isinstance(receipt.get("source_commit"), str)
        or COMMIT_RE.fullmatch(receipt["source_commit"]) is None
        or not valid_id(receipt.get("probability_model_id"))
        or not valid_id(receipt.get("receipt_id"))
        or not valid_sha(receipt.get("receipt_sha256"))
        or receipt.get("source_case_id") != case.get("case_id")
        or receipt.get("scope_id") != case.get("scope_id")
        or receipt.get("scope_sha256") != case.get("scope_sha256")
        or receipt.get("decision_cutoff_id") != case.get("decision_cutoff_id")
        or receipt.get("world_set_receipt_id") != case.get("world_set_receipt_id")
        or receipt.get("joint_loss_receipt_id") != case.get("joint_loss_receipt_id")
        or receipt.get("joint_scenario_manifest_sha256") != case.get("joint_scenario_manifest_sha256")
        or receipt.get("expected_scenario_ids_sha256") != case.get("expected_scenario_ids_sha256")
        or not valid_id(receipt.get("calibration_receipt_id"))
        or not valid_sha(receipt.get("calibration_receipt_sha256"))
        or not valid_id(receipt.get("validation_split_id"))
        or not valid_sha(receipt.get("validation_split_sha256"))
        or not valid_sha(receipt.get("probability_payload_sha256"))
        or receipt.get("result") != "pass"
    ):
        raise ValueError("probability model receipt pins or provenance are malformed")

    if receipt.get("state") != "CURRENT":
        raise LookupError("PROBABILITY_MODEL_NOT_CURRENT")
    if receipt.get("calibration_state") != "QUALIFIED":
        raise LookupError("PROBABILITY_CALIBRATION_NOT_QUALIFIED")
    if receipt.get("coverage_state") != "ADEQUATE":
        raise LookupError("PROBABILITY_SUPPORT_COVERAGE_NOT_ADEQUATE")
    if receipt.get("model_family_state") != "ADEQUATE":
        raise LookupError("PROBABILITY_MODEL_FAMILY_NOT_ADEQUATE")

    assumptions = receipt.get("assumptions")
    if (
        not isinstance(assumptions, dict)
        or assumptions.get("sampling_design") != "EXPLICIT_JOINT_DISTRIBUTION"
        or assumptions.get("dependence_model") != "JOINT_SCENARIO_WEIGHTS"
    ):
        raise ValueError("probability model assumptions must declare an explicit joint distribution")

    expected_ids = case.get("expected_scenario_ids")
    if (
        not isinstance(expected_ids, list)
        or not expected_ids
        or any(not valid_id(value) for value in expected_ids)
        or len(expected_ids) != len(set(expected_ids))
        or sorted(expected_ids) != expected_ids
        or digest_json(expected_ids) != case.get("expected_scenario_ids_sha256")
    ):
        raise ValueError("expected joint scenario ID inventory is malformed or digest-inconsistent")
    joint_scenarios = case.get("joint_scenarios")
    if not isinstance(joint_scenarios, list) or not joint_scenarios:
        raise ValueError("joint scenario identity records are missing")
    joint_ids = [row.get("scenario_id") for row in joint_scenarios if isinstance(row, dict)]
    if (
        len(joint_ids) != len(joint_scenarios)
        or any(not valid_id(value) for value in joint_ids)
        or len(set(joint_ids)) != len(joint_ids)
        or sorted(joint_ids) != expected_ids
        or digest_json(canonical_joint_rows(joint_scenarios))
        != case.get("joint_scenario_manifest_sha256")
    ):
        raise ValueError("joint scenario identity records or manifest digest are inconsistent")

    weights = receipt.get("weights")
    if not isinstance(weights, list) or len(weights) != len(expected_ids):
        raise ValueError("probability weights do not cover the exact joint scenario inventory")
    weight_ids = [row.get("scenario_id") for row in weights if isinstance(row, dict)]
    if len(weight_ids) != len(weights) or sorted(weight_ids) != expected_ids or len(set(weight_ids)) != len(weight_ids):
        raise ValueError("probability weights contain missing, duplicate or unknown scenario IDs")

    parsed = []
    total = Fraction(0, 1)
    for row in weights:
        if not valid_id(row.get("scenario_id")):
            raise ValueError("probability weight has malformed scenario identity")
        weight = exact_fraction(row.get("weight"))
        if weight is None:
            raise ValueError("probability weight is not a bounded nonnegative rational")
        total += weight
        parsed.append((row["scenario_id"], weight))
    if total != 1:
        raise ValueError("probability weights do not sum exactly to one")
    if probability_payload_digest(weights) != receipt["probability_payload_sha256"]:
        raise ValueError("probability weight payload digest mismatch")
    return parsed


def evaluate_policy(
    policy: dict[str, Any],
    weights: dict[str, Fraction],
    expected_ids: set[str],
    alphas: list[Fraction],
) -> dict[str, Any]:
    policy_id = policy.get("policy_id")
    if not valid_id(policy_id):
        raise ValueError("policy identity is malformed")
    rows = policy.get("per_scenario")
    if not isinstance(rows, list) or len(rows) != len(expected_ids):
        raise ValueError(f"policy {policy_id} does not cover exact scenario inventory")

    seen = set()
    metric_values: dict[str, list[tuple[Fraction, Fraction]]] = {
        "wait_and_see_regret": [],
        "outcome_class_regret": [],
    }
    for row in rows:
        sid = row.get("scenario_id")
        if not valid_id(sid) or sid in seen or sid not in expected_ids:
            raise ValueError(f"policy {policy_id} has a missing, duplicate or unknown scenario row")
        seen.add(sid)
        for metric in metric_values:
            value = exact_fraction(row.get(metric))
            if value is None:
                raise ValueError(f"policy {policy_id} has malformed {metric}")
            metric_values[metric].append((value, weights[sid]))
    if seen != expected_ids:
        raise ValueError(f"policy {policy_id} does not match exact scenario support")

    output: dict[str, Any] = {"policy_id": policy_id, "metrics": {}}
    for metric, values in metric_values.items():
        expected_regret = sum(
            (value * weight for value, weight in values), Fraction(0, 1)
        )
        worst_regret = max(value for value, _ in values)
        alpha_reports = []
        for alpha in alphas:
            risk = cvar_upper_tail(values, alpha)
            alpha_reports.append({
                "alpha": fraction_json(alpha),
                "var": fraction_json(risk["var"]),
                "cvar": fraction_json(risk["cvar"]),
                "tail_mass": fraction_json(risk["tail_mass"]),
                "var_atom_fraction": fraction_json(risk["var_atom_fraction"]),
            })
        output["metrics"][metric] = {
            "expected_regret": fraction_json(expected_regret),
            "worst_case_regret": fraction_json(worst_regret),
            "cvar": alpha_reports,
        }
    return output


def evaluate_case(case: dict[str, Any], alphas: list[Fraction]) -> dict[str, Any]:
    if not valid_id(case.get("case_id")) or not valid_id(case.get("scope_id")):
        return invalid("case identity or scope is malformed")
    try:
        weight_rows = validate_receipt(case)
    except LookupError as error:
        return abstain(str(error))
    except (ValueError, TypeError, KeyError) as error:
        return invalid(str(error))

    weights = dict(weight_rows)
    expected_ids = set(weights)
    policies = case.get("policies")
    if not isinstance(policies, list) or not policies:
        return invalid("policy list is empty or malformed")
    ids = [item.get("policy_id") for item in policies]
    if any(not valid_id(value) for value in ids) or len(ids) != len(set(ids)):
        return invalid("policy IDs are malformed or duplicated")

    try:
        reports = [
            evaluate_policy(policy, weights, expected_ids, alphas)
            for policy in sorted(policies, key=lambda item: item["policy_id"])
        ]
    except (ValueError, TypeError, KeyError) as error:
        return invalid(str(error))

    return {
        "status": "PASS",
        "case_id": case["case_id"],
        "probability_model_id": case["probability_model"]["probability_model_id"],
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "scenario_count": len(weights),
        "probability_mass_exactly_one": True,
        "policy_count": len(reports),
        "policies": reports,
        "authority_granted": False,
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: cvar_regret_oracle.py <corpus.json>", file=sys.stderr)
        return 2
    corpus = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    if (
        corpus.get("schema") != CORPUS_SCHEMA
        or corpus.get("evidence_class") != "synthetic"
        or corpus.get("claim_ceiling") != CLAIM_CEILING
    ):
        print("unsupported CVaR regret corpus", file=sys.stderr)
        return 2

    alphas = []
    for value in corpus.get("alphas", []):
        alpha = exact_fraction(value, allow_zero=False)
        if alpha is None or alpha >= 1:
            print("alpha values must be exact rationals strictly between zero and one", file=sys.stderr)
            return 2
        alphas.append(alpha)
    if not alphas or len(set(alphas)) != len(alphas):
        print("alpha list must be nonempty and unique", file=sys.stderr)
        return 2

    results = [evaluate_case(case, alphas) for case in corpus.get("cases", [])]
    report = {
        "schema": "luminous.orbital-cvar-regret.report.v0.1",
        "result": "pass" if all(row["status"] in {"PASS", "ABSTAIN"} for row in results) else "fail",
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "case_count": len(results),
        "pass_cases": sum(row["status"] == "PASS" for row in results),
        "abstain_cases": sum(row["status"] == "ABSTAIN" for row in results),
        "invalid_cases": sum(row["status"] == "INVALID_INPUT" for row in results),
        "authority_grants": 0,
        "cases": results,
        "authority_granted": False,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    print(
        f"python-cvar-regret-oracle: {len(results)} case(s); exact Fraction; "
        "explicit joint weights only; no authority",
        file=sys.stderr,
    )
    return 0 if report["result"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())

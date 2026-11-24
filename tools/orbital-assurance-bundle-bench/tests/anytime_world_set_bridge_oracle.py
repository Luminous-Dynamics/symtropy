#!/usr/bin/env python3
"""Independent stdlib-only Python oracle for the incumbent world-set receipt bridge.

This is a test/reference implementation only. It does not generate or validate
the upstream statistical guarantee, authenticate issuers, or grant authority.
"""
from __future__ import annotations

import copy
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from fractions import Fraction
from pathlib import Path
from typing import Any

SCHEMA = "luminous.orbital-incumbent-world-set-bridge.receipt.v0.1"
CLAIM_CEILING = "source-receipt-structure-and-inventory-composition-only"
SHA_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
MAX_SAFE_INTEGER = (1 << 53) - 1

TOP_KEYS = {
    "schema", "evidence_class", "claim_ceiling", "source", "contract",
    "cutoff", "world_inventory", "qualification", "adapter_disposition",
    "authority_granted",
}
SOURCE_KEYS = {
    "owner_issue", "receipt_schema", "claim_ceiling", "source_commit",
    "source_case_id", "receipt_id", "receipt_sha256",
}
CONTRACT_KEYS = {
    "guarantee_type", "guarantee_disposition", "optional_stopping_covered",
    "predictable_selection", "adaptive_selection_covered", "process_id",
    "look_index", "stopping_state", "alpha_total", "alpha_allocated",
    "alpha_allocation_id", "alpha_allocation_sha256", "filtration_sha256",
    "selection_policy_sha256", "conditional_kernel_sha256",
}
CUTOFF_KEYS = {
    "cutoff_id", "event_watermark_sequence", "observed_max_sequence",
    "history_sha256", "knowledge_time_utc",
}
INVENTORY_KEYS = {
    "candidate_world_ids", "candidate_world_ids_sha256",
    "surviving_world_ids", "surviving_world_ids_sha256",
    "excluded_world_ids", "excluded_world_ids_sha256",
}
QUALIFICATION_KEYS = {
    "source_state", "model_family_state", "channel_dependence_state",
    "action_realization_state",
}


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical_json(value: Any) -> str:
    # IDs are constrained to printable ASCII, so this matches the compact
    # UTF-8 JSON-array digest contract without Unicode normalization ambiguity.
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def valid_id(value: Any) -> bool:
    return (
        isinstance(value, str)
        and bool(value.strip())
        and all(32 <= ord(ch) <= 126 for ch in value)
    )


def valid_sha(value: Any) -> bool:
    return isinstance(value, str) and SHA_RE.fullmatch(value) is not None


def exact_keys(value: Any, expected: set[str]) -> bool:
    return isinstance(value, dict) and set(value) == expected


def valid_int(value: Any, minimum: int = 0, maximum: int = MAX_SAFE_INTEGER) -> bool:
    # bool is an int subclass in Python; reject it explicitly for JSON integers.
    return type(value) is int and minimum <= value <= maximum


def sorted_unique_ids(value: Any) -> bool:
    return (
        isinstance(value, list)
        and all(valid_id(item) for item in value)
        and len(set(value)) == len(value)
        and value == sorted(value)
    )


def digest_matches(ids: list[str], digest: Any) -> bool:
    return valid_sha(digest) and sha256_text(canonical_json(ids)) == digest


def fraction_from_json(value: Any) -> Fraction | None:
    if not exact_keys(value, {"numerator", "denominator"}):
        return None
    numerator = value.get("numerator")
    denominator = value.get("denominator")
    if not valid_int(numerator, 1) or not valid_int(denominator, 1):
        return None
    return Fraction(numerator, denominator)


def invalid(reason: str) -> dict[str, str]:
    return {"status": "INVALID_INPUT", "reason": reason}


def abstain(reason: str) -> dict[str, str]:
    return {"status": "ABSTAIN", "reason": reason}


def validate_receipt(receipt: Any) -> dict[str, str]:
    if (
        not exact_keys(receipt, TOP_KEYS)
        or receipt.get("schema") != SCHEMA
        or receipt.get("evidence_class") != "synthetic"
        or receipt.get("claim_ceiling") != CLAIM_CEILING
        or receipt.get("authority_granted") is not False
        or receipt.get("adapter_disposition") not in {"CONSUMABLE", "ABSTAIN"}
    ):
        return invalid("envelope identity, claim ceiling, disposition or authority field is invalid")

    source = receipt["source"]
    if (
        not exact_keys(source, SOURCE_KEYS)
        or source.get("owner_issue") != "#6887"
        or not valid_id(source.get("receipt_schema"))
        or not valid_id(source.get("claim_ceiling"))
        or not isinstance(source.get("source_commit"), str)
        or COMMIT_RE.fullmatch(source["source_commit"]) is None
        or not valid_id(source.get("source_case_id"))
        or not valid_id(source.get("receipt_id"))
        or not valid_sha(source.get("receipt_sha256"))
    ):
        return invalid("upstream source identity or receipt pin is malformed")

    contract = receipt["contract"]
    if (
        not exact_keys(contract, CONTRACT_KEYS)
        or not valid_id(contract.get("process_id"))
        or not valid_int(contract.get("look_index"), 1)
        or contract.get("stopping_state") not in {"ACTIVE", "STOPPED"}
        or contract.get("guarantee_disposition") not in {"QUALIFIED", "UNQUALIFIED"}
        or type(contract.get("optional_stopping_covered")) is not bool
        or type(contract.get("predictable_selection")) is not bool
        or type(contract.get("adaptive_selection_covered")) is not bool
        or not valid_id(contract.get("alpha_allocation_id"))
        or not valid_sha(contract.get("alpha_allocation_sha256"))
        or not valid_sha(contract.get("filtration_sha256"))
        or not valid_sha(contract.get("selection_policy_sha256"))
        or not valid_sha(contract.get("conditional_kernel_sha256"))
    ):
        return invalid("sequential process, filtration, policy or kernel commitment is malformed")

    alpha_total = fraction_from_json(contract.get("alpha_total"))
    alpha_allocated = fraction_from_json(contract.get("alpha_allocated"))
    if (
        alpha_total is None
        or alpha_allocated is None
        or alpha_total > 1
        or alpha_allocated > alpha_total
    ):
        return invalid("allocated alpha is invalid or exceeds the declared campaign budget")

    cutoff = receipt["cutoff"]
    if (
        not exact_keys(cutoff, CUTOFF_KEYS)
        or not valid_id(cutoff.get("cutoff_id"))
        or not valid_int(cutoff.get("event_watermark_sequence"))
        or not valid_int(cutoff.get("observed_max_sequence"))
        or cutoff["observed_max_sequence"] > cutoff["event_watermark_sequence"]
        or not valid_sha(cutoff.get("history_sha256"))
        or not isinstance(cutoff.get("knowledge_time_utc"), str)
        or not cutoff["knowledge_time_utc"].endswith("Z")
    ):
        return invalid("decision cutoff or evidence-history boundary is malformed")
    try:
        parsed_time = datetime.fromisoformat(cutoff["knowledge_time_utc"].replace("Z", "+00:00"))
    except ValueError:
        return invalid("decision cutoff or evidence-history boundary is malformed")
    if parsed_time.tzinfo is None or parsed_time.utcoffset() != timezone.utc.utcoffset(parsed_time):
        return invalid("decision cutoff or evidence-history boundary is malformed")

    inventory = receipt["world_inventory"]
    if (
        not exact_keys(inventory, INVENTORY_KEYS)
        or not sorted_unique_ids(inventory.get("candidate_world_ids"))
        or not inventory["candidate_world_ids"]
        or not sorted_unique_ids(inventory.get("surviving_world_ids"))
        or not sorted_unique_ids(inventory.get("excluded_world_ids"))
        or not digest_matches(inventory["candidate_world_ids"], inventory.get("candidate_world_ids_sha256"))
        or not digest_matches(inventory["surviving_world_ids"], inventory.get("surviving_world_ids_sha256"))
        or not digest_matches(inventory["excluded_world_ids"], inventory.get("excluded_world_ids_sha256"))
    ):
        return invalid("world inventory is not canonical, unique and digest-consistent")

    candidate = set(inventory["candidate_world_ids"])
    surviving = set(inventory["surviving_world_ids"])
    excluded = set(inventory["excluded_world_ids"])
    if (
        surviving & excluded
        or surviving | excluded != candidate
    ):
        return invalid("surviving and excluded inventories do not partition the candidate set")

    qualification = receipt["qualification"]
    if (
        not exact_keys(qualification, QUALIFICATION_KEYS)
        or qualification.get("source_state") not in {
            "CURRENT", "STALE", "REVOKED", "CONTRADICTED", "INDETERMINATE"
        }
        or qualification.get("model_family_state") not in {
            "ADEQUATE", "INADEQUATE", "INDETERMINATE"
        }
        or qualification.get("channel_dependence_state") not in {
            "ADEQUATE", "UNRESOLVED", "INADEQUATE"
        }
        or qualification.get("action_realization_state") not in {
            "VERIFIED", "NOT_REQUIRED_BY_PROFILE", "UNKNOWN", "CONTRADICTED"
        }
    ):
        return invalid("qualification object has unknown fields or malformed state enums")

    if contract["guarantee_type"] != "ANYTIME_VALID_WORLD_EXCLUSION":
        computed = abstain("UPSTREAM_NOT_ANYTIME_VALID")
    elif contract["guarantee_disposition"] != "QUALIFIED":
        computed = abstain("UPSTREAM_GUARANTEE_NOT_QUALIFIED")
    elif not contract["optional_stopping_covered"]:
        computed = abstain("OPTIONAL_STOPPING_NOT_COVERED")
    elif not contract["predictable_selection"] or not contract["adaptive_selection_covered"]:
        computed = abstain("ADAPTIVE_SELECTION_NOT_COVERED")
    elif qualification["source_state"] != "CURRENT":
        computed = abstain("SOURCE_NOT_CURRENT")
    elif qualification["model_family_state"] != "ADEQUATE":
        computed = abstain("MODEL_FAMILY_NOT_ADEQUATE")
    elif qualification["channel_dependence_state"] != "ADEQUATE":
        computed = abstain("CHANNEL_DEPENDENCE_NOT_ADEQUATE")
    elif qualification["action_realization_state"] not in {"VERIFIED", "NOT_REQUIRED_BY_PROFILE"}:
        computed = abstain("ACTION_REALIZATION_NOT_QUALIFIED")
    elif not inventory["surviving_world_ids"]:
        computed = abstain("EMPTY_SURVIVING_WORLD_SET")
    else:
        computed = {
            "status": "CONSUMABLE",
            "reason": "SOURCE_BOUND_WORLD_SET_READY_FOR_DOWNSTREAM_GATE",
        }

    if receipt["adapter_disposition"] != computed["status"]:
        return invalid("declared adapter disposition does not match independently derived disposition")
    return computed


def set_digest(ids: list[str]) -> str:
    return sha256_text(canonical_json(ids))


def build_cases(fixture: dict[str, Any]) -> list[tuple[str, Any, str, str]]:
    def changed(name: str, mutate, status: str, reason: str):
        candidate = copy.deepcopy(fixture)
        mutate(candidate)
        return name, candidate, status, reason

    cases: list[tuple[str, Any, str, str]] = []
    cases.append(changed("unknown top-level property",
        lambda r: r.update(unreviewed_field="must reject"),
        "INVALID_INPUT", "envelope identity, claim ceiling, disposition or authority field is invalid"))
    cases.append(changed("missing source receipt digest",
        lambda r: r["source"].pop("receipt_sha256"),
        "INVALID_INPUT", "upstream source identity or receipt pin is malformed"))
    cases.append(changed("unknown nested contract property",
        lambda r: r["contract"].update(unreviewed_field=True),
        "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"))
    cases.append(changed("malformed stopping state enum",
        lambda r: r["contract"].update(stopping_state="MAYBE"),
        "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"))
    cases.append(changed("malformed guarantee disposition enum",
        lambda r: r["contract"].update(guarantee_disposition="MOSTLY_QUALIFIED"),
        "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"))
    cases.append(changed("malformed model qualification enum",
        lambda r: r["qualification"].update(model_family_state="PROBABLY_ADEQUATE"),
        "INVALID_INPUT", "qualification object has unknown fields or malformed state enums"))
    cases.append(("qualified adaptive source", copy.deepcopy(fixture),
        "CONSUMABLE", "SOURCE_BOUND_WORLD_SET_READY_FOR_DOWNSTREAM_GATE"))

    def empty_survivors(r):
        r["world_inventory"]["surviving_world_ids"] = []
        r["world_inventory"]["surviving_world_ids_sha256"] = set_digest([])
        excluded = list(r["world_inventory"]["candidate_world_ids"])
        r["world_inventory"]["excluded_world_ids"] = excluded
        r["world_inventory"]["excluded_world_ids_sha256"] = set_digest(excluded)
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("empty survivor set abstains", empty_survivors,
        "ABSTAIN", "EMPTY_SURVIVING_WORLD_SET"))

    def set_guarantee(r):
        r["contract"]["guarantee_type"] = "FIXED_LOOK_ONLY"
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("fixed-look test is not anytime-valid", set_guarantee,
        "ABSTAIN", "UPSTREAM_NOT_ANYTIME_VALID"))

    def no_optional_stopping(r):
        r["contract"]["optional_stopping_covered"] = False
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("optional stopping is not covered", no_optional_stopping,
        "ABSTAIN", "OPTIONAL_STOPPING_NOT_COVERED"))

    def no_adaptive_selection(r):
        r["contract"]["adaptive_selection_covered"] = False
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("adaptive selection is not covered", no_adaptive_selection,
        "ABSTAIN", "ADAPTIVE_SELECTION_NOT_COVERED"))

    cases.append(changed("alpha allocation exceeds campaign budget",
        lambda r: r["contract"].update(alpha_allocated={"numerator": 1, "denominator": 10}),
        "INVALID_INPUT", "allocated alpha is invalid or exceeds the declared campaign budget"))
    cases.append(changed("survivor digest tampering",
        lambda r: r["world_inventory"].update(surviving_world_ids_sha256="0" * 64),
        "INVALID_INPUT", "world inventory is not canonical, unique and digest-consistent"))
    cases.append(changed("duplicate survivor identity",
        lambda r: r["world_inventory"].update(surviving_world_ids=["world-A", "world-A"]),
        "INVALID_INPUT", "world inventory is not canonical, unique and digest-consistent"))

    def overlap(r):
        ids = ["world-B"]
        r["world_inventory"]["excluded_world_ids"] = ids
        r["world_inventory"]["excluded_world_ids_sha256"] = set_digest(ids)
    cases.append(changed("survivor/excluded overlap", overlap,
        "INVALID_INPUT", "surviving and excluded inventories do not partition the candidate set"))
    cases.append(changed("future evidence after cutoff",
        lambda r: r["cutoff"].update(
            observed_max_sequence=r["cutoff"]["event_watermark_sequence"] + 1),
        "INVALID_INPUT", "decision cutoff or evidence-history boundary is malformed"))

    def stale_source(r):
        r["qualification"]["source_state"] = "STALE"
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("stale source receipt", stale_source,
        "ABSTAIN", "SOURCE_NOT_CURRENT"))

    def inadequate_model(r):
        r["qualification"]["model_family_state"] = "INADEQUATE"
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("inadequate model family", inadequate_model,
        "ABSTAIN", "MODEL_FAMILY_NOT_ADEQUATE"))

    def unresolved_channel(r):
        r["qualification"]["channel_dependence_state"] = "UNRESOLVED"
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("unresolved common-cause channel", unresolved_channel,
        "ABSTAIN", "CHANNEL_DEPENDENCE_NOT_ADEQUATE"))

    def unknown_action_realization(r):
        r["qualification"]["action_realization_state"] = "UNKNOWN"
        r["adapter_disposition"] = "ABSTAIN"
    cases.append(changed("unknown action realization", unknown_action_realization,
        "ABSTAIN", "ACTION_REALIZATION_NOT_QUALIFIED"))
    cases.append(changed("missing selection-policy commitment",
        lambda r: r["contract"].update(selection_policy_sha256=None),
        "INVALID_INPUT", "sequential process, filtration, policy or kernel commitment is malformed"))
    cases.append(changed("malformed source receipt digest",
        lambda r: r["source"].update(receipt_sha256="not-a-digest"),
        "INVALID_INPUT", "upstream source identity or receipt pin is malformed"))
    cases.append(changed("unsorted world inventory",
        lambda r: r["world_inventory"].update(candidate_world_ids=["world-B", "world-A"]),
        "INVALID_INPUT", "world inventory is not canonical, unique and digest-consistent"))
    return cases


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: anytime_world_set_bridge_oracle.py <synthetic-receipt.json>", file=sys.stderr)
        return 2
    fixture = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    cases = build_cases(fixture)
    results = []
    mismatches = []
    for name, candidate, expected_status, expected_reason in cases:
        actual = validate_receipt(candidate)
        passed = actual.get("status") == expected_status and actual.get("reason") == expected_reason
        results.append({
            "case": name,
            "status": actual.get("status"),
            "reason": actual.get("reason"),
            "expected_status": expected_status,
            "expected_reason": expected_reason,
            "passed": passed,
        })
        if not passed:
            mismatches.append(name)
    report = {
        "oracle": "python-stdlib-independent-reference",
        "case_count": len(cases),
        "expected_result_mismatches": len(mismatches),
        "authority_grants": 0,
        "result": "pass" if not mismatches else "fail",
        "cases": results,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    if mismatches:
        print("failed cases: " + ", ".join(mismatches), file=sys.stderr)
        return 1
    print(
        f"python-anytime-world-set-oracle: {len(cases)} cases passed; "
        "Fraction exactness; no external dependencies; no authority",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

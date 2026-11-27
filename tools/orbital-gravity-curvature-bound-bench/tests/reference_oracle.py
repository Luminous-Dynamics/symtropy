#!/usr/bin/env python3
"""Independent exact-integer oracle for the orbital curvature-bound contract."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path

U64_MAX = (1 << 64) - 1
U128_MAX = (1 << 128) - 1
SHA_HEX = set("0123456789abcdefABCDEF")
RECEIPT_SCHEMA = "luminous.orbital-gravity-curvature-bound.receipt.v0.1"

SCOPE_FIELDS = (
    "scope_id",
    "action_id",
    "segment_id",
    "target_id",
    "chief_id",
    "servicer_id",
    "central_body_id",
    "keepout_profile_id",
    "keepout_geometry_revision",
    "transform_digest",
    "frame_id",
    "frame_revision",
    "time_basis_id",
    "dynamics_profile_revision",
    "start_tai_ns",
    "end_tai_ns",
)
PROVIDER_NAMES = (
    "mu",
    "chief_radius",
    "relative_separation",
    "differential_acceleration",
    "transform_norm",
)
U64_FIELDS = (
    "chief_radius_lower_mm",
    "relative_separation_upper_mm",
    "differential_acceleration_upper_mm_per_s2",
    "transform_operator_norm_upper_scaled",
    "start_tai_ns",
    "end_tai_ns",
)


def valid_text(value: str) -> bool:
    return bool(value.strip())


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(char in SHA_HEX for char in value)


def checked(value: int) -> int | None:
    return value if 0 <= value <= U128_MAX else None


def checked_mul(left: int, right: int) -> int | None:
    return checked(left * right)


def checked_add(left: int, right: int) -> int | None:
    return checked(left + right)


def ceil_div_nonnegative(numerator: int, denominator: int) -> int | None:
    if denominator <= 0 or numerator < 0:
        return None
    quotient, remainder = divmod(numerator, denominator)
    return checked(quotient if remainder == 0 else quotient + 1)


def scope_object(row: dict[str, str]) -> dict[str, object]:
    return {
        "scope_id": row["scope_id"],
        "action_id": row["action_id"],
        "segment_id": row["segment_id"],
        "target_id": row["target_id"],
        "chief_id": row["chief_id"],
        "servicer_id": row["servicer_id"],
        "central_body_id": row["central_body_id"],
        "keepout_profile_id": row["keepout_profile_id"],
        "keepout_geometry_revision": row["keepout_geometry_revision"],
        "transform_digest": row["transform_digest"],
        "frame_id": row["frame_id"],
        "frame_revision": row["frame_revision"],
        "time_basis_id": row["time_basis_id"],
        "dynamics_profile_revision": row["dynamics_profile_revision"],
        "start_tai_ns": int(row["start_tai_ns"]),
        "end_tai_ns": int(row["end_tai_ns"]),
    }


def provider_object(row: dict[str, str], name: str) -> dict[str, str]:
    return {
        "provider_id": row[f"{name}_provider_id"],
        "provider_revision": row[f"{name}_provider_revision"],
        "receipt_sha256": row[f"{name}_receipt_sha256"],
        "scope_id": row[f"{name}_scope_id"],
        "state": row[f"{name}_state"],
    }


def base_result(
    row: dict[str, str],
    disposition: str,
    reason: str | None,
) -> dict[str, object]:
    return {
        "case_id": row["case_id"],
        "scope": scope_object(row),
        "disposition": disposition,
        "central_gravity_numerator": None,
        "radial_domain_denominator": None,
        "differential_acceleration_upper_mm_per_s2": None,
        "transform_operator_norm_upper_scaled": None,
        "scaled_central_numerator": None,
        "scaled_perturbation_term": None,
        "m_upper_scaled_per_s2": None,
        "m_scale": 1_000_000,
        "m_unit": "s^-2",
        "radial_slack_mm": None,
        "perturbation_inventory_id": row["perturbation_inventory_id"],
        "perturbation_inventory_complete": row["perturbation_inventory_complete"] == "true",
        "evidence": {name: provider_object(row, name) for name in PROVIDER_NAMES},
        "reason_codes": [] if reason is None else [reason],
        "bound_ready": False,
        "source_bytes_authenticated": False,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
    }


def provider_shape_valid(evidence: dict[str, str]) -> bool:
    return (
        valid_text(evidence["provider_id"])
        and valid_text(evidence["provider_revision"])
        and valid_sha(evidence["receipt_sha256"])
        and valid_text(evidence["scope_id"])
    )


def evaluate(row: dict[str, str]) -> dict[str, object]:
    case_id = row["case_id"]
    scope = scope_object(row)
    scope_text = [scope[field] for field in SCOPE_FIELDS[:14]]
    start = int(row["start_tai_ns"])
    end = int(row["end_tai_ns"])

    if (
        not valid_text(case_id)
        or any(not valid_text(value) for value in scope_text)
        or start >= end
        or not valid_text(row["perturbation_inventory_id"])
    ):
        return base_result(row, "INVALID_INPUT", "CASE_SCOPE_OR_INVENTORY_INVALID")

    if row["transform_time_invariant"] != "true":
        return base_result(row, "OUT_OF_SCOPE", "TIME_VARYING_TRANSFORM_UNSUPPORTED")

    if row["perturbation_inventory_complete"] != "true":
        return base_result(row, "INDETERMINATE", "PERTURBATION_INVENTORY_INCOMPLETE")

    evidence = {name: provider_object(row, name) for name in PROVIDER_NAMES}
    if any(not provider_shape_valid(item) for item in evidence.values()):
        return base_result(row, "INVALID_INPUT", "PROVIDER_EVIDENCE_SHAPE_INVALID")

    if any(item["scope_id"] != row["scope_id"] for item in evidence.values()):
        return base_result(row, "OUT_OF_SCOPE", "PROVIDER_SCOPE_MISMATCH")

    states = [item["state"] for item in evidence.values()]
    if any(state == "FAIL" for state in states):
        return base_result(row, "PROVIDER_FAILED", "REQUIRED_BOUND_PROVIDER_FAILED")

    if any(state != "PASS" for state in states):
        return base_result(row, "INDETERMINATE", "REQUIRED_BOUND_PROVIDER_NOT_CURRENT")

    mu = int(row["mu_upper_mm3_per_s2"])
    chief_radius = int(row["chief_radius_lower_mm"])
    rho = int(row["relative_separation_upper_mm"])
    differential = int(row["differential_acceleration_upper_mm_per_s2"])
    transform = int(row["transform_operator_norm_upper_scaled"])

    if (
        mu == 0
        or chief_radius == 0
        or transform == 0
        or mu > U128_MAX
        or any(int(row[field]) > U64_MAX for field in U64_FIELDS)
    ):
        return base_result(row, "INVALID_INPUT", "POSITIVE_PHYSICAL_BOUND_REQUIRED")

    if chief_radius <= rho:
        return base_result(row, "OUT_OF_SCOPE", "RADIAL_DOMAIN_NOT_STRICTLY_POSITIVE")

    radial_slack = chief_radius - rho
    denominator_a = checked_mul(radial_slack, radial_slack)
    denominator = checked_mul(denominator_a, radial_slack) if denominator_a is not None else None
    if denominator is None:
        return base_result(row, "OVERFLOW", "RADIAL_CUBE_OVERFLOW")

    central_product = checked_mul(mu, rho)
    central_numerator = checked_mul(central_product, 2) if central_product is not None else None
    if central_numerator is None:
        return base_result(row, "OVERFLOW", "CENTRAL_GRAVITY_PRODUCT_OVERFLOW")

    scaled_central_numerator = checked_mul(transform, central_numerator)
    if scaled_central_numerator is None:
        return base_result(row, "OVERFLOW", "SCALED_CENTRAL_NUMERATOR_OVERFLOW")

    scaled_perturbation_term = checked_mul(transform, differential)
    if scaled_perturbation_term is None:
        return base_result(row, "OVERFLOW", "SCALED_PERTURBATION_TERM_OVERFLOW")

    central_m_upper = ceil_div_nonnegative(scaled_central_numerator, denominator)
    if central_m_upper is None:
        return base_result(row, "OVERFLOW", "CENTRAL_OUTWARD_DIVISION_FAILED")

    m_upper = checked_add(central_m_upper, scaled_perturbation_term)
    if m_upper is None:
        return base_result(row, "OVERFLOW", "OUTWARD_BOUND_SUM_OVERFLOW")

    result = base_result(row, "BOUND_READY", None)
    result.update(
        {
            "central_gravity_numerator": central_numerator,
            "radial_domain_denominator": denominator,
            "differential_acceleration_upper_mm_per_s2": differential,
            "transform_operator_norm_upper_scaled": transform,
            "scaled_central_numerator": scaled_central_numerator,
            "scaled_perturbation_term": scaled_perturbation_term,
            "m_upper_scaled_per_s2": m_upper,
            "radial_slack_mm": radial_slack,
            "bound_ready": True,
        }
    )
    return result


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>")

    corpus_path = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = corpus_path.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    with corpus_path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 36, f"expected 36 cases, found {len(rows)}"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"
    results = []
    for row in rows:
        result = evaluate(row)
        assert result["disposition"] == row["expected"], (
            f'{row["case_id"]}: expected {row["expected"]}, '
            f'received {result["disposition"]}'
        )
        results.append(result)

    receipt = {
        "schema": RECEIPT_SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": actual_digest,
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "bound_ready_count": sum(item["bound_ready"] for item in results),
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} source-bound curvature cases.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

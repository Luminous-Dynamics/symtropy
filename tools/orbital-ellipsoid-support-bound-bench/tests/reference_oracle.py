#!/usr/bin/env python3
"""Independent arbitrary-precision oracle for directional ellipsoid support."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path

U128_MAX = (1 << 128) - 1
SHA_HEX = set("0123456789abcdefABCDEF")
SCALE_UM_PER_M = 1_000_000
RECEIPT_SCHEMA = "luminous.orbital-ellipsoid-support-bound.receipt.v0.1"

SCOPE_FIELDS = (
    "scope_id",
    "terminal_action_id",
    "target_id",
    "keepout_profile_id",
    "keepout_geometry_revision",
    "matrix_sha256",
    "matrix_convention",
    "frame_id",
    "frame_revision",
    "transform_digest",
    "direction_sha256",
    "direction_convention",
    "length_unit",
    "terminal_epoch_tai_ns",
)

PROVIDERS = ("matrix_spd", "direction_unit", "quadratic_lower")


def valid_text(value: str) -> bool:
    return bool(value.strip())


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(character in SHA_HEX for character in value)


def scope_object(row: dict[str, str]) -> dict[str, object]:
    return {
        "scope_id": row["scope_id"],
        "terminal_action_id": row["terminal_action_id"],
        "target_id": row["target_id"],
        "keepout_profile_id": row["keepout_profile_id"],
        "keepout_geometry_revision": row["keepout_geometry_revision"],
        "matrix_sha256": row["matrix_sha256"],
        "matrix_convention": row["matrix_convention"],
        "frame_id": row["frame_id"],
        "frame_revision": row["frame_revision"],
        "transform_digest": row["transform_digest"],
        "direction_sha256": row["direction_sha256"],
        "direction_convention": row["direction_convention"],
        "length_unit": row["length_unit"],
        "terminal_epoch_tai_ns": int(row["terminal_epoch_tai_ns"]),
    }


def provider_object(row: dict[str, str], name: str) -> dict[str, str]:
    return {
        "provider_id": row[f"{name}_provider_id"],
        "provider_revision": row[f"{name}_provider_revision"],
        "receipt_sha256": row[f"{name}_receipt_sha256"],
        "scope_id": row[f"{name}_scope_id"],
        "state": row[f"{name}_state"],
    }


def result_base(
    row: dict[str, str],
    disposition: str,
    reason: str | None,
) -> dict[str, object]:
    return {
        "case_id": row["case_id"],
        "scope": scope_object(row),
        "disposition": disposition,
        "q_lower_numerator": int(row["q_lower_numerator"]),
        "q_lower_denominator": int(row["q_lower_denominator"]),
        "q_unit": "m^-2",
        "rho_y_upper_um": None,
        "rho_unit": "um",
        "scale_um_per_m": SCALE_UM_PER_M,
        "q_scaled_ceiling": None,
        "floor_sqrt": None,
        "perfect_square": None,
        "evidence": {name: provider_object(row, name) for name in PROVIDERS},
        "reason_codes": [] if reason is None else [reason],
        "bound_ready": False,
        "source_bytes_authenticated": False,
        "physical_geometry_validated": False,
        "authority_granted": False,
    }


def checked_u128(value: int) -> int | None:
    return value if 0 <= value <= U128_MAX else None


def ceil_div(numerator: int, denominator: int) -> int | None:
    if denominator == 0:
        return None
    quotient, remainder = divmod(numerator, denominator)
    return checked_u128(quotient if remainder == 0 else quotient + 1)


def floor_sqrt(value: int) -> int:
    # Independent arbitrary-precision binary search.
    low, high = 0, 1 << 64
    while low < high:
        middle = (low + high + 1) // 2
        if middle * middle <= value:
            low = middle
        else:
            high = middle - 1
    return low


def provider_valid(item: dict[str, str]) -> bool:
    return (
        valid_text(item["provider_id"])
        and valid_text(item["provider_revision"])
        and valid_sha(item["receipt_sha256"])
        and valid_text(item["scope_id"])
    )


def evaluate(row: dict[str, str]) -> dict[str, object]:
    case_id = row["case_id"]
    scope = scope_object(row)
    if (
        not valid_text(case_id)
        or any(not valid_text(str(scope[field])) for field in SCOPE_FIELDS[:-1])
        or row["length_unit"] != "metre"
        or row["direction_convention"] != "unit-along-track-positive-y"
        or not valid_sha(row["matrix_sha256"])
        or not valid_sha(row["direction_sha256"])
    ):
        return result_base(row, "INVALID_INPUT", "CASE_OR_SCOPE_INVALID")

    numerator = int(row["q_lower_numerator"])
    denominator = int(row["q_lower_denominator"])
    if numerator == 0 or denominator == 0 or numerator > U128_MAX or denominator > U128_MAX:
        return result_base(
            row,
            "INVALID_INPUT",
            "QUADRATIC_LOWER_BOUND_MUST_BE_STRICTLY_POSITIVE",
        )

    evidence = {name: provider_object(row, name) for name in PROVIDERS}
    if any(not provider_valid(item) for item in evidence.values()):
        return result_base(row, "INVALID_INPUT", "PROVIDER_EVIDENCE_SHAPE_INVALID")

    if any(item["scope_id"] != row["scope_id"] for item in evidence.values()):
        return result_base(row, "OUT_OF_SCOPE", "PROVIDER_SCOPE_MISMATCH")

    states = [item["state"] for item in evidence.values()]
    if any(state == "FAIL" for state in states):
        return result_base(row, "PROVIDER_FAILED", "REQUIRED_GEOMETRY_PROVIDER_FAILED")
    if any(state != "PASS" for state in states):
        return result_base(row, "INDETERMINATE", "REQUIRED_GEOMETRY_PROVIDER_NOT_CURRENT")

    scale_squared = checked_u128(SCALE_UM_PER_M * SCALE_UM_PER_M)
    if scale_squared is None:
        return result_base(row, "OVERFLOW", "OUTPUT_SCALE_SQUARE_OVERFLOW")
    scaled_numerator = checked_u128(scale_squared * denominator)
    if scaled_numerator is None:
        return result_base(row, "OVERFLOW", "SCALED_QUADRATIC_NUMERATOR_OVERFLOW")
    q_ceiling = ceil_div(scaled_numerator, numerator)
    if q_ceiling is None:
        return result_base(row, "OVERFLOW", "QUADRATIC_RATIO_OVERFLOW")

    floor = floor_sqrt(q_ceiling)
    perfect = floor * floor == q_ceiling
    rho = floor if perfect else floor + 1
    result = result_base(row, "BOUND_READY", None)
    result.update(
        {
            "rho_y_upper_um": rho,
            "q_scaled_ceiling": q_ceiling,
            "floor_sqrt": floor,
            "perfect_square": perfect,
            "bound_ready": True,
        }
    )
    return result


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>")

    path = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = path.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 36, f"expected 36 cases, found {len(rows)}"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"
    results = []
    for row in rows:
        result = evaluate(row)
        assert result["disposition"] == row["expected"], (
            f'{row["case_id"]}: expected {row["expected"]}, received {result["disposition"]}'
        )
        results.append(result)

    receipt = {
        "schema": RECEIPT_SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": actual_digest,
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "bound_ready_count": sum(bool(item["bound_ready"]) for item in results),
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "physical_geometry_validated": False,
        "authority_granted": False,
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} ellipsoid-support cases independently.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Independent standard-library oracle for eccentric passive-terminal conditions."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path

I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
U64_MAX = (1 << 64) - 1
SHA_HEX = set("0123456789abcdefABCDEF")
SCOPE_FIELDS = (
    "action_id",
    "target_id",
    "keepout_profile_id",
    "keepout_geometry_revision",
    "transform_digest",
    "frame_id",
    "frame_revision",
    "time_basis_id",
    "dynamics_profile_revision",
)
PERIOD_SCOPE_FIELDS = (
    "period_action_id",
    "period_target_id",
    "period_keepout_profile_id",
    "period_keepout_geometry_revision",
    "period_transform_digest",
    "period_frame_id",
    "period_frame_revision",
    "period_time_basis_id",
    "period_dynamics_profile_revision",
)
RECEIPT_SCHEMA = "luminous.orbital-passive-safety-terminal.receipt.v0.1"


def valid_text(value: str) -> bool:
    return bool(value.strip())


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(char in SHA_HEX for char in value)


def scope_tuple(row: dict[str, str]) -> tuple[str, ...]:
    return tuple(row[field] for field in SCOPE_FIELDS)


def period_scope_tuple(row: dict[str, str]) -> tuple[str, ...]:
    return tuple(row[field] for field in PERIOD_SCOPE_FIELDS)


def has_period_witness(row: dict[str, str]) -> bool:
    return any(row[field] != "" for field in (
        "period_horizon_id",
        "period_start_tai_ns",
        "period_end_tai_ns",
        "relative_period_ns",
        "period_complete_time_coverage",
        "period_disposition",
        "period_segment_count",
        "period_receipt_sha256",
        *PERIOD_SCOPE_FIELDS,
    ))


def outcome(
    case_id: str,
    disposition: str,
    branch: str,
    margin: int | None,
    reason: str | None,
    period_consumed: bool,
) -> dict[str, object]:
    return {
        "case_id": case_id,
        "disposition": disposition,
        "delta_a_branch": branch,
        "margin_scaled": margin,
        "reason_codes": [] if reason is None else [reason],
        "period_witness_consumed": period_consumed,
    }


def evaluate(row: dict[str, str]) -> dict[str, object]:
    case_id = row["case_id"]
    scope = scope_tuple(row)
    if not valid_text(case_id) or any(not valid_text(value) for value in scope):
        return outcome(
            case_id,
            "INVALID_INPUT",
            "UNCLASSIFIED",
            None,
            "TERMINAL_CASE_OR_SCOPE_INVALID",
            False,
        )

    delta_lo = int(row["delta_a_lower_scaled"])
    delta_hi = int(row["delta_a_upper_scaled"])
    y_lower = int(row["y_lower_scaled"])
    y_upper = int(row["y_upper_scaled"])
    rho = int(row["rho_y_upper_scaled"])
    provider_id = row["bounds_provider_id"]
    provider_revision = row["bounds_provider_revision"]
    receipt_sha = row["bounds_receipt_sha256"]
    state = row["bounds_state"]

    integer_values = (
        delta_lo,
        delta_hi,
        y_lower,
        y_upper,
        rho,
        int(row["terminal_epoch_tai_ns"]),
    )
    if (
        any(value < I64_MIN or value > I64_MAX for value in integer_values)
        or not valid_text(provider_id)
        or not valid_text(provider_revision)
        or not valid_sha(receipt_sha)
        or delta_lo > delta_hi
        or y_lower > y_upper
        or rho <= 0
    ):
        return outcome(
            case_id,
            "INVALID_INPUT",
            "UNCLASSIFIED",
            None,
            "BOUND_ENVELOPE_INVALID",
            False,
        )

    if state == "FAIL":
        return outcome(
            case_id,
            "NOT_PASSIVE_SAFE",
            "UPSTREAM_BOUND_FAIL",
            None,
            "UPSTREAM_BOUND_PROVIDER_FAILED",
            False,
        )
    if state in ("UNKNOWN", "STALE", "REVOKED"):
        return outcome(
            case_id,
            "INDETERMINATE",
            "UPSTREAM_BOUND_UNRESOLVED",
            None,
            "UPSTREAM_BOUND_PROVIDER_NOT_CURRENT",
            False,
        )
    if state != "PASS":
        return outcome(
            case_id,
            "INVALID_INPUT",
            "UNCLASSIFIED",
            None,
            "BOUND_SOURCE_STATE_INVALID",
            False,
        )

    period_present = has_period_witness(row)
    terminal_epoch = int(row["terminal_epoch_tai_ns"])

    if delta_lo == 0 and delta_hi == 0:
        if not period_present:
            return outcome(
                case_id,
                "PERIODIC_RELATIVE_MOTION_ONLY",
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIODIC_MOTION_REQUIRES_COMPLETE_PERIOD_CLEARANCE",
                False,
            )

        period_start = int(row["period_start_tai_ns"])
        period_end = int(row["period_end_tai_ns"])
        relative_period = int(row["relative_period_ns"])
        segment_count = int(row["period_segment_count"])
        period_id = row["period_horizon_id"]
        period_sha = row["period_receipt_sha256"]

        if (
            not valid_text(period_id)
            or not valid_sha(period_sha)
            or relative_period <= 0
            or relative_period > U64_MAX
            or segment_count <= 0
            or segment_count > (1 << 64) - 1
            or period_start < 0
            or period_end < 0
            or period_start > U64_MAX
            or period_end > U64_MAX
            or period_start >= period_end
        ):
            return outcome(
                case_id,
                "INVALID_INPUT",
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIOD_WITNESS_SHAPE_INVALID",
                True,
            )

        if period_scope_tuple(row) != scope:
            return outcome(
                case_id,
                "OUT_OF_SCOPE",
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIOD_WITNESS_SCOPE_MISMATCH",
                True,
            )

        if period_start != terminal_epoch:
            return outcome(
                case_id,
                "OUT_OF_SCOPE",
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIOD_WITNESS_DOES_NOT_START_AT_TERMINAL_EPOCH",
                True,
            )

        covered_duration = period_end - period_start
        if covered_duration < relative_period:
            return outcome(
                case_id,
                "INDETERMINATE",
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIOD_WITNESS_SHORTER_THAN_ONE_RELATIVE_PERIOD",
                True,
            )

        if (
            row["period_complete_time_coverage"] != "true"
            or row["period_disposition"] != "COMPLETE_CONDITIONAL_CLEAR"
        ):
            return outcome(
                case_id,
                "INDETERMINATE",
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIOD_WITNESS_NOT_COMPLETE_AND_CLEAR",
                True,
            )

        return outcome(
            case_id,
            "PERIODIC_PASSIVE_SAFE",
            "EXACT_ZERO_DELTA_A",
            None,
            None,
            True,
        )

    if period_present:
        return outcome(
            case_id,
            "INVALID_INPUT",
            "NONZERO_DELTA_A",
            None,
            "PERIODIC_WITNESS_PROVIDED_FOR_NONPERIODIC_BRANCH",
            False,
        )

    if delta_hi < 0:
        margin = y_lower - rho
        if margin > 0:
            return outcome(
                case_id,
                "SECULAR_OUTBOUND_PASSIVE_SAFE",
                "NEGATIVE_DELTA_A_MOVES_POSITIVE_Y",
                margin,
                None,
                False,
            )
        return outcome(
            case_id,
            "NOT_PASSIVE_SAFE",
            "NEGATIVE_DELTA_A_MOVES_POSITIVE_Y",
            margin,
            "LOWER_Y_BOUND_DOES_NOT_STRICTLY_EXCEED_KEEP_OUT_SUPPORT",
            False,
        )

    if delta_lo > 0:
        margin = -rho - y_upper
        if margin > 0:
            return outcome(
                case_id,
                "SECULAR_OUTBOUND_PASSIVE_SAFE",
                "POSITIVE_DELTA_A_MOVES_NEGATIVE_Y",
                margin,
                None,
                False,
            )
        return outcome(
            case_id,
            "NOT_PASSIVE_SAFE",
            "POSITIVE_DELTA_A_MOVES_NEGATIVE_Y",
            margin,
            "UPPER_Y_BOUND_DOES_NOT_STRICTLY_EXCEED_NEGATIVE_KEEP_OUT_SUPPORT",
            False,
        )

    return outcome(
        case_id,
        "INDETERMINATE",
        "DELTA_A_SIGN_UNRESOLVED",
        None,
        "DELTA_A_INTERVAL_TOUCHES_OR_CROSSES_ZERO",
        False,
    )


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>"
        )

    corpus_path = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = corpus_path.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    with corpus_path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 38, f"expected 38 cases, found {len(rows)}"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"

    results = []
    for row in rows:
        calculated = evaluate(row)
        assert calculated["disposition"] == row["expected"], (
            f'{row["case_id"]}: expected {row["expected"]}, '
            f'received {calculated["disposition"]}'
        )
        results.append(calculated)

    receipt = {
        "schema": RECEIPT_SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": actual_digest,
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} terminal cases independently.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Independent exact-integer oracle for piecewise inspection-horizon composition."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

SCALE = 1_000_000
CURVATURE_DENOMINATOR = 8_000_000_000_000_000_000
I128_MIN = -(1 << 127)
I128_MAX = (1 << 127) - 1
I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
U64_MAX = (1 << 64) - 1
RECEIPT_SCHEMA = "luminous.orbital-inspection-horizon-composition.receipt.v0.1"

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


def checked(value: int) -> int | None:
    return value if I128_MIN <= value <= I128_MAX else None


def checked_add(left: int, right: int) -> int | None:
    return checked(left + right)


def checked_mul(left: int, right: int) -> int | None:
    return checked(left * right)


def ceil_div_nonnegative(numerator: int, denominator: int) -> int | None:
    if numerator < 0 or denominator <= 0:
        return None
    quotient, remainder = divmod(numerator, denominator)
    return checked(quotient if remainder == 0 else quotient + 1)


def checked_dot(left: tuple[int, int, int], right: tuple[int, int, int]) -> int | None:
    total = 0
    for a, b in zip(left, right, strict=True):
        product = checked_mul(a, b)
        if product is None:
            return None
        total = checked_add(total, product)
        if total is None:
            return None
    return total


def chord_strictly_outside(
    start: tuple[int, int, int],
    end: tuple[int, int, int],
    tube_error: int,
) -> bool | None:
    direction = tuple(checked(b - a) for a, b in zip(start, end, strict=True))
    if any(value is None for value in direction):
        return None
    direction = tuple(int(value) for value in direction)

    direction_squared = checked_dot(direction, direction)
    start_squared = checked_dot(start, start)
    threshold = checked_add(SCALE, tube_error)
    if direction_squared is None or start_squared is None or threshold is None:
        return None
    threshold_squared = checked_mul(threshold, threshold)
    if threshold_squared is None:
        return None

    if direction_squared == 0:
        return start_squared > threshold_squared

    projection = checked_dot(start, direction)
    if projection is None:
        return None
    if projection >= 0:
        return start_squared > threshold_squared
    if projection <= -direction_squared:
        end_squared = checked_dot(end, end)
        if end_squared is None:
            return None
        return end_squared > threshold_squared

    projection_squared = checked_mul(projection, projection)
    start_times_direction = checked_mul(start_squared, direction_squared)
    if projection_squared is None or start_times_direction is None:
        return None
    distance_numerator = checked(start_times_direction - projection_squared)
    if distance_numerator is None or distance_numerator < 0:
        return None
    comparison = checked_mul(threshold_squared, direction_squared)
    if comparison is None:
        return None
    return distance_numerator > comparison


def verify_segment(row: dict[str, str]) -> dict[str, str]:
    segment_id = row["segment_id"]
    start = tuple(int(row[f"q0{axis}"]) for axis in "xyz")
    end = tuple(int(row[f"q1{axis}"]) for axis in "xyz")
    acceleration = int(row["acceleration_bound_scaled_per_s2"])
    duration = int(row["duration_ns"])
    endpoint_error = int(row["endpoint_error_bound_scaled"])

    if (
        not segment_id.strip()
        or duration <= 0
        or duration > U64_MAX
        or acceleration < 0
        or acceleration > I64_MAX
        or endpoint_error < 0
        or endpoint_error > I64_MAX
        or any(value < I64_MIN or value > I64_MAX for value in start + end)
    ):
        disposition = "INVALID_INPUT"
    else:
        first = checked_mul(acceleration, duration)
        numerator = checked_mul(first, duration) if first is not None else None
        if numerator is None:
            disposition = "OVERFLOW"
        else:
            curvature = ceil_div_nonnegative(numerator, CURVATURE_DENOMINATOR)
            if curvature is None:
                disposition = "OVERFLOW"
            else:
                total_error = checked_add(curvature, endpoint_error)
                if total_error is None:
                    disposition = "OVERFLOW"
                else:
                    outside = chord_strictly_outside(start, end, total_error)
                    if outside is None:
                        disposition = "OVERFLOW"
                    else:
                        disposition = "CONDITIONAL_CLEAR" if outside else "NOT_PROVEN"

    return {"segment_id": segment_id, "disposition": disposition}


def seam_overlap(left: dict[str, str], right: dict[str, str]) -> bool | None:
    l_end = tuple(int(left[f"q1{axis}"]) for axis in "xyz")
    r_start = tuple(int(right[f"q0{axis}"]) for axis in "xyz")
    differences = tuple(checked(b - a) for a, b in zip(l_end, r_start, strict=True))
    if any(value is None for value in differences):
        return None
    differences = tuple(int(value) for value in differences)
    distance_squared = checked_dot(differences, differences)
    combined_error = checked_add(
        int(left["endpoint_error_bound_scaled"]),
        int(right["endpoint_error_bound_scaled"]),
    )
    if distance_squared is None or combined_error is None:
        return None
    radius_squared = checked_mul(combined_error, combined_error)
    if radius_squared is None:
        return None
    return distance_squared <= radius_squared


def scope_tuple(row: dict[str, str]) -> tuple[str, ...]:
    return tuple(row[field] for field in SCOPE_FIELDS)


def evaluate_horizon(horizon_id: str, rows: list[dict[str, str]]) -> dict[str, object]:
    start = int(rows[0]["horizon_start_ns"])
    end = int(rows[0]["horizon_end_ns"])
    segment_results = [verify_segment(row) for row in rows]
    count = len(rows)

    def report(disposition: str, coverage: bool, seams: int) -> dict[str, object]:
        return {
            "horizon_id": horizon_id,
            "disposition": disposition,
            "complete_time_coverage": coverage,
            "segment_count": count,
            "seam_count": seams,
            "segments": segment_results,
        }

    if not horizon_id.strip() or start >= end or not rows:
        return report("INVALID_INPUT" if rows else "INCOMPLETE_COVERAGE", False, 0)

    base_scope = scope_tuple(rows[0])
    if any(not value.strip() for value in base_scope):
        return report("INVALID_INPUT", False, 0)

    ids: set[str] = set()
    seam_events: set[str] = set()

    for index, row in enumerate(rows):
        segment_id = row["segment_id"]
        segment_start = int(row["segment_start_ns"])
        segment_end = int(row["segment_end_ns"])
        duration = int(row["duration_ns"])
        start_event = row["start_event_id"]
        end_event = row["end_event_id"]

        if (
            not segment_id.strip()
            or segment_id in ids
            or not start_event.strip()
            or not end_event.strip()
            or segment_start >= segment_end
            or duration != segment_end - segment_start
            or int(row["acceleration_bound_scaled_per_s2"]) < 0
            or int(row["endpoint_error_bound_scaled"]) < 0
        ):
            return report("INVALID_INPUT", False, index)
        ids.add(segment_id)

        if any(not value.strip() for value in scope_tuple(row)):
            return report("INVALID_INPUT", False, index)
        if scope_tuple(row) != base_scope:
            return report("SCOPE_MISMATCH", False, index)

        boundary = row["start_boundary"]
        if index == 0:
            if boundary != "HORIZON_START" or segment_start != start:
                return report("INCOMPLETE_COVERAGE", False, 0)
        elif boundary not in ("SMOOTH_JOIN", "IMPULSIVE_VELOCITY_CHANGE"):
            return report("BOUNDARY_MISMATCH", False, index)

    if int(rows[-1]["segment_end_ns"]) != end:
        return report("INCOMPLETE_COVERAGE", False, count - 1)

    for index in range(1, count):
        left = rows[index - 1]
        right = rows[index]
        if int(left["segment_end_ns"]) != int(right["segment_start_ns"]):
            return report("INCOMPLETE_COVERAGE", False, index - 1)

        seam_event = left["end_event_id"]
        if seam_event != right["start_event_id"] or seam_event in seam_events:
            return report("BOUNDARY_MISMATCH", True, index)
        seam_events.add(seam_event)

        overlap = seam_overlap(left, right)
        if overlap is None:
            return report("OVERFLOW", True, index)
        if not overlap:
            return report("SEAM_INCOHERENT", True, index)

    dispositions = [item["disposition"] for item in segment_results]
    if "OVERFLOW" in dispositions:
        return report("OVERFLOW", True, count - 1)
    if "INVALID_INPUT" in dispositions:
        return report("INVALID_INPUT", True, count - 1)
    if "NOT_PROVEN" in dispositions:
        return report("NOT_PROVEN", True, count - 1)
    return report("COMPLETE_CONDITIONAL_CLEAR", True, count - 1)


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
        rows = list(csv.DictReader(line for line in handle if not line.startswith("#")))

    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    expected: dict[str, str] = {}
    for row in rows:
        horizon_id = row["horizon_id"]
        if horizon_id in expected:
            assert expected[horizon_id] == row["expected"], "inconsistent expected disposition"
        else:
            expected[horizon_id] = row["expected"]
        grouped[horizon_id].append(row)

    assert len(grouped) == 32, f"expected 32 horizons, found {len(grouped)}"
    results = [evaluate_horizon(key, grouped[key]) for key in sorted(grouped)]
    for result in results:
        assert result["disposition"] == expected[result["horizon_id"]], (
            f'{result["horizon_id"]}: expected {expected[result["horizon_id"]]}, '
            f'received {result["disposition"]}'
        )

    receipt = {
        "schema": RECEIPT_SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": actual_digest,
        "case_count": len(results),
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} horizon compositions independently.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Independent exact-integer/Fraction oracle for the chord-tube geometry kernel."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from fractions import Fraction
from pathlib import Path

SCALE = 1_000_000
CURVATURE_DENOMINATOR = 8_000_000_000_000_000_000
I128_MAX = (1 << 127) - 1
I128_MIN = -(1 << 127)
RECEIPT_SCHEMA = "luminous.orbital-inspection-chord-tube.receipt.v0.1"


def checked(value: int) -> int | None:
    return value if I128_MIN <= value <= I128_MAX else None


def checked_add(left: int, right: int) -> int | None:
    return checked(left + right)


def checked_mul(left: int, right: int) -> int | None:
    return checked(left * right)


def ceil_fraction(value: Fraction) -> int:
    if value < 0:
        raise ValueError("ceil_fraction expects a non-negative value")
    return (value.numerator + value.denominator - 1) // value.denominator


def dot(left: tuple[int, int, int], right: tuple[int, int, int]) -> int | None:
    total = 0
    for a, b in zip(left, right, strict=True):
        product = checked_mul(a, b)
        if product is None:
            return None
        total_next = checked_add(total, product)
        if total_next is None:
            return None
        total = total_next
    return total


def strictly_outside_chord_tube(
    start: tuple[int, int, int],
    end: tuple[int, int, int],
    total_error: int,
) -> bool | None:
    direction = tuple(checked(b - a) for a, b in zip(start, end, strict=True))
    if any(value is None for value in direction):
        return None
    direction = tuple(int(value) for value in direction)

    direction_squared = dot(direction, direction)
    start_squared = dot(start, start)
    threshold = checked_add(SCALE, total_error)
    if direction_squared is None or start_squared is None or threshold is None:
        return None
    threshold_squared = checked_mul(threshold, threshold)
    if threshold_squared is None:
        return None

    if direction_squared == 0:
        return start_squared > threshold_squared

    projection = dot(start, direction)
    if projection is None:
        return None
    if projection >= 0:
        return start_squared > threshold_squared
    if projection <= -direction_squared:
        end_squared = dot(end, end)
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
    comparison_bound = checked_mul(threshold_squared, direction_squared)
    if comparison_bound is None:
        return None
    return distance_numerator > comparison_bound


def verify(row: dict[str, str]) -> dict[str, object]:
    case_id = row["case_id"]
    start = tuple(int(row[f"q0{axis}"]) for axis in "xyz")
    end = tuple(int(row[f"q1{axis}"]) for axis in "xyz")
    acceleration = int(row["accel_bound_scaled_per_s2"])
    duration_ns = int(row["duration_ns"])
    endpoint_error = int(row["endpoint_error_bound_scaled"])

    if (
        not case_id.strip()
        or duration_ns <= 0
        or acceleration < 0
        or endpoint_error < 0
        or any(not (-(1 << 63) <= value < (1 << 63)) for value in start + end)
        or not (-(1 << 63) <= acceleration < (1 << 63))
        or not (-(1 << 63) <= endpoint_error < (1 << 63))
        or not (0 <= duration_ns < (1 << 64))
    ):
        disposition = "INVALID_INPUT"
        curvature_error = None
        total_error = None
    else:
        # Match the fixed-width Rust arithmetic before doing exact division.
        first_product = checked_mul(acceleration, duration_ns)
        curvature_numerator = (
            checked_mul(first_product, duration_ns)
            if first_product is not None
            else None
        )
        if curvature_numerator is None:
            disposition = "OVERFLOW"
            curvature_error = None
            total_error = None
        else:
            curvature_fraction = Fraction(curvature_numerator, CURVATURE_DENOMINATOR)
            curvature_error = ceil_fraction(curvature_fraction)
            if checked(curvature_error) is None:
                disposition = "OVERFLOW"
                curvature_error = None
                total_error = None
            else:
                total_error = checked_add(curvature_error, endpoint_error)
                if total_error is None:
                    disposition = "OVERFLOW"
                    curvature_error = None
                    total_error = None
                else:
                    outside = strictly_outside_chord_tube(start, end, total_error)
                    if outside is None:
                        disposition = "OVERFLOW"
                    else:
                        disposition = "CONDITIONAL_CLEAR" if outside else "NOT_PROVEN"

    expected = row["expected"]
    assert disposition == expected, (
        f"{case_id}: expected {expected}, computed {disposition}"
    )
    return {
        "case_id": case_id,
        "disposition": disposition,
        "curvature_error_scaled": curvature_error,
        "total_tube_error_scaled": total_error,
    }


def check_quadratic_remainder_identity() -> None:
    # For q(t) = chord(t) + c*t*(1-t), sup ||q''|| = 2||c||;
    # maximum chord deviation is ||c||/4 = M/8 at t=1/2.
    for c_norm in (1, 2, 5, 17, 101):
        acceleration_bound = 2 * c_norm
        exact_max_deviation = Fraction(c_norm, 4)
        theorem_bound = Fraction(acceleration_bound, 8)
        assert exact_max_deviation == theorem_bound
        for index in range(101):
            t = Fraction(index, 100)
            deviation = Fraction(c_norm) * t * (1 - t)
            assert deviation <= theorem_bound


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>")

    corpus_path = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = corpus_path.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    check_quadratic_remainder_identity()

    with corpus_path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(line for line in handle if not line.startswith("#")))

    assert rows, "empty corpus"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"
    results = [verify(row) for row in rows]
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
    print(f"Validated {len(results)} frozen cases and exact quadratic remainder identity.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

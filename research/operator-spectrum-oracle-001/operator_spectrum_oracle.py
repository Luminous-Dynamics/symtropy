#!/usr/bin/env python3
"""Exact-rational oracle and validator for the Rust metric operator spectrum probe.

This is a small-fixture research oracle. It uses exact integer/rational algebra
for rank and characteristic polynomials, then rational real-root isolation for
the smaller Gram matrix. It intentionally does not replace the one-sided
Jacobi SVD used by the Rust implementation: Gram matrices square the nonzero
spectral condition number.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from decimal import Decimal, ROUND_CEILING, ROUND_FLOOR, localcontext
from pathlib import Path
from typing import Any

import sympy as sp


def decimal_fraction(value: sp.Rational, rounding: str, precision: int) -> Decimal:
    with localcontext() as context:
        context.prec = precision
        context.rounding = rounding
        numerator = Decimal(int(sp.numer(value)))
        denominator = Decimal(int(sp.denom(value)))
        return numerator / denominator


def sqrt_interval(lower: sp.Rational, upper: sp.Rational, precision: int) -> dict[str, str]:
    if lower < 0 or upper < 0:
        raise ValueError(f"Gram eigenvalue interval is negative: [{lower}, {upper}]")
    with localcontext() as context:
        context.prec = precision
        context.rounding = ROUND_FLOOR
        lower_decimal = decimal_fraction(lower, ROUND_FLOOR, precision).sqrt()
    with localcontext() as context:
        context.prec = precision
        context.rounding = ROUND_CEILING
        upper_decimal = decimal_fraction(upper, ROUND_CEILING, precision).sqrt()
    return {"lower": format(lower_decimal, "f"), "upper": format(upper_decimal, "f")}


def ratio_interval(
    numerator: dict[str, str],
    denominator: dict[str, str],
    precision: int,
) -> dict[str, str] | None:
    numerator_lower = Decimal(numerator["lower"])
    numerator_upper = Decimal(numerator["upper"])
    denominator_lower = Decimal(denominator["lower"])
    denominator_upper = Decimal(denominator["upper"])
    if denominator_lower <= 0:
        return None
    with localcontext() as context:
        context.prec = precision
        context.rounding = ROUND_FLOOR
        lower = numerator_lower / denominator_upper
    with localcontext() as context:
        context.prec = precision
        context.rounding = ROUND_CEILING
        upper = numerator_upper / denominator_lower
    return {"lower": format(lower, "f"), "upper": format(upper, "f")}


def build_oracle(input_document: dict[str, Any]) -> dict[str, Any]:
    epsilon = sp.Rational(input_document["root_isolation_epsilon"])
    precision = int(input_document["singular_value_decimal_digits"])
    if precision < 30:
        raise ValueError("singular-value enclosure precision must be at least 30 digits")

    records: list[dict[str, Any]] = []
    for fixture in input_document["fixtures"]:
        fixture_id = fixture["fixture_id"]
        matrix = sp.Matrix(fixture["matrix"])
        if matrix.rows == 0 or matrix.cols == 0:
            raise ValueError(f"{fixture_id}: empty matrix")

        rows, columns = matrix.shape
        # Use the smaller exact Gram matrix to limit root-isolation cost.
        gram_kind = "AAT" if rows <= columns else "ATA"
        gram = matrix * matrix.T if rows <= columns else matrix.T * matrix
        polynomial = gram.charpoly().as_poly()
        coefficients = [str(value) for value in polynomial.all_coeffs()]
        exact_rank = int(matrix.rank())

        if exact_rank != int(fixture["expected_rank"]):
            raise AssertionError(
                f"{fixture_id}: exact rank {exact_rank} != frozen expected rank "
                f"{fixture['expected_rank']}"
            )
        if coefficients != fixture["expected_characteristic_polynomial_coefficients"]:
            raise AssertionError(
                f"{fixture_id}: characteristic polynomial drift: {coefficients}"
            )

        isolated = polynomial.intervals(eps=epsilon)
        eigenvalue_intervals: list[tuple[sp.Rational, sp.Rational]] = []
        for (lower, upper), multiplicity in isolated:
            eigenvalue_intervals.extend([(sp.Rational(lower), sp.Rational(upper))] * multiplicity)
        eigenvalue_intervals.reverse()

        expected_singular_count = min(rows, columns)
        if len(eigenvalue_intervals) != expected_singular_count:
            raise AssertionError(
                f"{fixture_id}: isolated {len(eigenvalue_intervals)} roots, "
                f"expected {expected_singular_count}"
            )
        singular_intervals = [
            sqrt_interval(lower, upper, precision)
            for lower, upper in eigenvalue_intervals
        ]

        nonzero_condition = (
            ratio_interval(singular_intervals[0], singular_intervals[exact_rank - 1], precision)
            if exact_rank > 0
            else None
        )
        # A map T_X -> T_Y is injective exactly when rank == dim(T_X).
        whole_map_condition = nonzero_condition if exact_rank == columns else None

        records.append(
            {
                "fixture_id": fixture_id,
                "matrix": fixture["matrix"],
                "rows": rows,
                "columns": columns,
                "gram_kind": gram_kind,
                "gram_matrix": [[int(value) for value in row] for row in gram.tolist()],
                "characteristic_polynomial_coefficients": coefficients,
                "exact_rank": exact_rank,
                "domain_nullity": columns - exact_rank,
                "codomain_nullity": rows - exact_rank,
                "singular_value_intervals": singular_intervals,
                "whole_map_condition_interval": whole_map_condition,
                "nonzero_spectrum_condition_interval": nonzero_condition,
            }
        )

    return {
        "schema_version": "luminous.operator_spectrum_exact_oracle.v0.1",
        "input_domain": "integer matrices interpreted exactly over Q",
        "gram_matrix_note": (
            "The smaller exact Gram matrix is used only for small-fixture eigenvalue "
            "isolation; this squares nonzero spectral condition numbers and is not "
            "the production SVD path."
        ),
        "root_isolation_epsilon": input_document["root_isolation_epsilon"],
        "singular_value_decimal_digits": precision,
        "fixtures": records,
    }


def within_interval(value: float, interval: dict[str, str], relative_tolerance: float) -> bool:
    if not math.isfinite(value):
        return False
    lower = float(interval["lower"])
    upper = float(interval["upper"])
    tolerance = max(1.0e-12, relative_tolerance * max(abs(lower), abs(upper), 1.0))
    return lower - tolerance <= value <= upper + tolerance


def validate_candidate(
    oracle: dict[str, Any],
    candidate: dict[str, Any],
    source_commit: str,
    relative_tolerance: float,
) -> dict[str, Any]:
    if candidate.get("schema_version") != "luminous.operator_spectrum_candidate.v0.1":
        raise AssertionError("candidate schema_version mismatch")
    if candidate.get("source_commit") != source_commit:
        raise AssertionError("candidate source commit does not match checked-out source")
    if not source_commit or source_commit == "local":
        raise AssertionError("candidate source commit is not admissible")
    rows = candidate.get("measurements")
    if not isinstance(rows, list):
        raise AssertionError("candidate measurements must be a list")

    expected_by_id = {item["fixture_id"]: item for item in oracle["fixtures"]}
    actual_by_id = {item.get("fixture_id"): item for item in rows}
    if len(actual_by_id) != len(rows) or set(actual_by_id) != set(expected_by_id):
        raise AssertionError(
            f"fixture ID mismatch: missing={set(expected_by_id)-set(actual_by_id)}, "
            f"extra={set(actual_by_id)-set(expected_by_id)}"
        )

    summaries = []
    for fixture_id, expected in expected_by_id.items():
        actual = actual_by_id[fixture_id]
        for field in ("numerical_rank", "domain_nullity", "codomain_nullity"):
            if actual.get(field) != expected[{
                "numerical_rank": "exact_rank",
                "domain_nullity": "domain_nullity",
                "codomain_nullity": "codomain_nullity",
            }[field]]:
                raise AssertionError(
                    f"{fixture_id}: {field}={actual.get(field)} does not match "
                    f"exact value {expected[field]}"
                )

        expected_status = (
            "RankDeficient"
            if expected["exact_rank"] < min(expected["rows"], expected["columns"])
            else "NonInjective"
            if expected["domain_nullity"] > 0
            else "Admitted"
        )
        if actual.get("status") != expected_status:
            raise AssertionError(
                f"{fixture_id}: status {actual.get('status')} != {expected_status}"
            )

        candidate_singulars = actual.get("singular_values")
        expected_intervals = expected["singular_value_intervals"]
        if not isinstance(candidate_singulars, list) or len(candidate_singulars) != len(expected_intervals):
            raise AssertionError(f"{fixture_id}: singular-value count mismatch")
        for index, (candidate_value, interval) in enumerate(zip(candidate_singulars, expected_intervals)):
            if not within_interval(float(candidate_value), interval, relative_tolerance):
                raise AssertionError(
                    f"{fixture_id}: singular value {index}={candidate_value} "
                    f"outside oracle interval {interval}"
                )

        for field, interval_field in (
            ("condition_estimate", "whole_map_condition_interval"),
            ("nonzero_spectrum_condition_estimate", "nonzero_spectrum_condition_interval"),
        ):
            expected_interval = expected[interval_field]
            candidate_value = actual.get(field)
            if expected_interval is None:
                if candidate_value is not None:
                    raise AssertionError(
                        f"{fixture_id}: {field} must be null, got {candidate_value}"
                    )
            else:
                if candidate_value is None or not within_interval(
                    float(candidate_value), expected_interval, relative_tolerance
                ):
                    raise AssertionError(
                        f"{fixture_id}: {field}={candidate_value} outside {expected_interval}"
                    )

        summaries.append(
            {
                "fixture_id": fixture_id,
                "exact_rank": expected["exact_rank"],
                "domain_nullity": expected["domain_nullity"],
                "codomain_nullity": expected["codomain_nullity"],
                "status": expected_status,
                "whole_map_condition_available": expected["whole_map_condition_interval"] is not None,
            }
        )

    return {
        "schema_version": "luminous.operator_spectrum_validation.v0.1",
        "source_commit": source_commit,
        "oracle": {
            "arithmetic": "exact integer/rational",
            "root_isolation_epsilon": oracle["root_isolation_epsilon"],
            "singular_value_decimal_digits": oracle["singular_value_decimal_digits"],
            "independent_method": "exact smaller-Gram characteristic polynomial plus rational real-root isolation",
        },
        "comparison": {
            "fixture_count": len(summaries),
            "relative_tolerance": relative_tolerance,
            "rank_mismatches": 0,
            "singular_values_outside_oracle": 0,
            "condition_semantics_mismatches": 0,
            "status_mismatches": 0,
            "result": "PASS",
        },
        "fixtures": summaries,
        "claim_boundary": (
            "Small-fixture calibration only. The exact Gram oracle squares nonzero "
            "condition numbers; it is not a replacement for one-sided Jacobi and "
            "does not certify arbitrary floating-point matrices."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--check-fixture", action="store_true")
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--oracle-output", type=Path)
    parser.add_argument("--validation-output", type=Path)
    parser.add_argument("--source-commit")
    parser.add_argument("--relative-tolerance", type=float, default=5.0e-10)
    args = parser.parse_args()

    input_document = json.loads(args.input.read_text(encoding="utf-8"))
    oracle = build_oracle(input_document)
    if args.check_fixture:
        print(json.dumps({
            "fixture_count": len(oracle["fixtures"]),
            "result": "PASS",
            "root_isolation_epsilon": oracle["root_isolation_epsilon"],
        }, sort_keys=True))
        return 0

    required = {
        "--candidate": args.candidate,
        "--oracle-output": args.oracle_output,
        "--validation-output": args.validation_output,
        "--source-commit": args.source_commit,
    }
    missing = [name for name, value in required.items() if value is None]
    if missing:
        parser.error("missing required arguments for candidate validation: " + ", ".join(missing))

    candidate_document = json.loads(args.candidate.read_text(encoding="utf-8"))
    validation = validate_candidate(
        oracle,
        candidate_document,
        args.source_commit,
        args.relative_tolerance,
    )
    args.oracle_output.write_text(json.dumps(oracle, indent=2) + "\n", encoding="utf-8")
    args.validation_output.write_text(json.dumps(validation, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(validation["comparison"], sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"operator-spectrum oracle failed: {error}", file=sys.stderr)
        raise

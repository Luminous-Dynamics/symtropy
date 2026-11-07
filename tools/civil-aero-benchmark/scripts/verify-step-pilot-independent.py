#!/usr/bin/env python3
"""Independent stdlib known-answer oracle for STEP7 and STEP24 pilot rows."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

PI = math.pi
RADIUS = 0.1


def evaluate_c2_dtlz2(x: list[float]) -> tuple[list[float], float]:
    g = sum((value - 0.5) ** 2 for value in x[1:])
    scale = 1.0 + g
    angle = 0.5 * PI * x[0]
    objectives = [scale * math.cos(angle), scale * math.sin(angle)]
    distances = [
        sum((value - (1.0 if index == axis else 0.0)) ** 2
            for index, value in enumerate(objectives))
        for axis in range(2)
    ]
    centre = 1.0 / math.sqrt(2.0)
    distances.append(sum((value - centre) ** 2 for value in objectives))
    margin = RADIUS ** 2 - min(distances)
    return objectives, margin


def evaluate_c1_dtlz1(x: list[float]) -> tuple[list[float], float]:
    m = 3
    k = len(x) - m + 1
    g = 100.0 * (
        k + sum((v - 0.5) ** 2 - math.cos(20.0 * PI * (v - 0.5))
                for v in x[m - 1:])
    )
    scale = 0.5 * (1.0 + g)
    objectives = [
        scale * x[0] * x[1],
        scale * x[0] * (1.0 - x[1]),
        scale * (1.0 - x[0]),
    ]
    margin = 1.0 - objectives[2] / 0.6 - (objectives[0] + objectives[1]) / 0.5
    return objectives, margin


def close(actual: float, expected: float, label: str) -> None:
    if not math.isclose(actual, expected, rel_tol=1e-10, abs_tol=1e-12):
        raise ValueError(f"{label}: {actual} != {expected}")


def check_row(row: dict[str, str], objectives: list[float], margin: float, label: str) -> None:
    actual = [float(value) for value in row["objectives"].split(";")]
    if len(actual) != len(objectives):
        raise ValueError(label + " objective count mismatch")
    for index, value in enumerate(objectives):
        close(actual[index], value, label + f" objective {index}")
    close(float(row["constraint_margins"]), margin, label + " constraint margin")
    close(float(row["aggregate_violation"]), max(-margin, 0.0), label + " aggregate violation")
    expected_feasible = margin >= -1e-12
    if row["feasible"].lower() != str(expected_feasible).lower():
        raise ValueError(label + " feasibility sign mismatch")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-step-pilot-independent.py output.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = {row["case_id"]: row for row in csv.DictReader(handle)}
    required = {
        "step7-x1-0.00", "step7-x1-0.25", "step7-x1-0.50",
        "step7-x1-0.75", "step7-x1-1.00", "step7-tail-shift",
        "step24-feasible-g0", "step24-distance-barrier",
    }
    if set(rows) != required:
        raise ValueError(f"unexpected case ids: {sorted(rows)}")

    for x1 in [0.0, 0.25, 0.5, 0.75, 1.0]:
        case = f"step7-x1-{x1:.2f}"
        vector = [x1, 0.5, 0.5, 0.5, 0.5]
        objectives, margin = evaluate_c2_dtlz2(vector)
        check_row(rows[case], objectives, margin, case)
    for case, vector in [
        ("step7-tail-shift", [0.25, 0.8, 0.5, 0.5, 0.5]),
    ]:
        objectives, margin = evaluate_c2_dtlz2(vector)
        check_row(rows[case], objectives, margin, case)
    for case, vector in [
        ("step24-feasible-g0", [0.2, 0.3, 0.5, 0.5, 0.5]),
        ("step24-distance-barrier", [0.2, 0.3, 0.0, 0.5, 0.5]),
    ]:
        objectives, margin = evaluate_c1_dtlz1(vector)
        check_row(rows[case], objectives, margin, case)

    close(float(rows["step24-feasible-g0"]["objectives"].split(";")[0]), 0.03, "C1-DTLZ1 f1 known answer")
    close(float(rows["step24-feasible-g0"]["objectives"].split(";")[1]), 0.07, "C1-DTLZ1 f2 known answer")
    close(float(rows["step24-feasible-g0"]["objectives"].split(";")[2]), 0.4, "C1-DTLZ1 f3 known answer")
    print("independent_step_pilot_oracle=pass tasks=2 rows=8 C2_components=3 C1_constraint_sign=checked")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_step_pilot_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)

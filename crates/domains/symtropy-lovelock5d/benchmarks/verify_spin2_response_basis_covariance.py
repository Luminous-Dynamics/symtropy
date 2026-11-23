#!/usr/bin/env python3
"""Dependency-free exact controls for 2x2 boundary-response basis covariance.

This is a synthetic linear-algebra control, not a black-hole calculation.
It verifies:
  * R -> G_inf^{-1} R G_H;
  * det(R') = det(G_inf)^{-1} det(R) det(G_H);
  * generalized singular-value invariants are unchanged when endpoint Gram
    matrices are transported with the bases;
  * raw Euclidean singular-value proxies can change under a basis change;
  * parameter-dependent basis derivatives contribute to R_eta.
"""

from __future__ import annotations

import argparse
import json
from fractions import Fraction as F
from pathlib import Path
from typing import Sequence

Matrix = list[list[F]]


def matrix(rows: Sequence[Sequence[int | F]]) -> Matrix:
    return [[x if isinstance(x, F) else F(x) for x in row] for row in rows]


def transpose(a: Matrix) -> Matrix:
    return [list(row) for row in zip(*a)]


def matmul(a: Matrix, b: Matrix) -> Matrix:
    if not a or not b or len(a[0]) != len(b):
        raise ValueError("incompatible matrix dimensions")
    return [
        [sum((a[i][k] * b[k][j] for k in range(len(b))), F(0))
         for j in range(len(b[0]))]
        for i in range(len(a))
    ]


def add(a: Matrix, b: Matrix) -> Matrix:
    return [[a[i][j] + b[i][j] for j in range(len(a[0]))]
            for i in range(len(a))]


def sub(a: Matrix, b: Matrix) -> Matrix:
    return [[a[i][j] - b[i][j] for j in range(len(a[0]))]
            for i in range(len(a))]


def det2(a: Matrix) -> F:
    if len(a) != 2 or len(a[0]) != 2:
        raise ValueError("det2 requires a 2x2 matrix")
    return a[0][0] * a[1][1] - a[0][1] * a[1][0]


def inv2(a: Matrix) -> Matrix:
    d = det2(a)
    if d == 0:
        raise ValueError("singular basis matrix")
    return [[a[1][1] / d, -a[0][1] / d],
            [-a[1][0] / d, a[0][0] / d]]


def trace2(a: Matrix) -> F:
    return a[0][0] + a[1][1]


def frobenius_sq(a: Matrix) -> F:
    return sum((x * x for row in a for x in row), F(0))


def generalized_spectrum_invariants(r: Matrix, wh: Matrix, winf: Matrix) -> tuple[F, F]:
    # Eigenvalues of WH^{-1} R^T W_inf R are the squared metric singular values.
    s = matmul(inv2(wh), matmul(transpose(r), matmul(winf, r)))
    return trace2(s), det2(s)


def encode_fraction(x: F) -> str:
    return str(x.numerator) if x.denominator == 1 else f"{x.numerator}/{x.denominator}"


def encode_matrix(a: Matrix) -> list[list[str]]:
    return [[encode_fraction(x) for x in row] for row in a]


def run_checks() -> dict[str, object]:
    r = matrix([[F(1, 2), 0], [0, 2]])
    gh = matrix([[2, 1], [1, 1]])
    gi = matrix([[1, 1], [0, 1]])
    wh = matrix([[1, 0], [0, 1]])
    wi = matrix([[1, 0], [0, 1]])

    det_gh, det_gi = det2(gh), det2(gi)
    if det_gh == 0 or det_gi == 0:
        raise AssertionError("positive control unexpectedly singular")

    rp = matmul(inv2(gi), matmul(r, gh))
    det_expected = det2(gi) ** -1 * det2(r) * det2(gh)
    if det2(rp) != det_expected:
        raise AssertionError("determinant covariance failed")

    whp = matmul(transpose(gh), matmul(wh, gh))
    wip = matmul(transpose(gi), matmul(wi, gi))
    spec_before = generalized_spectrum_invariants(r, wh, wi)
    spec_after = generalized_spectrum_invariants(rp, whp, wip)
    if spec_before != spec_after:
        raise AssertionError("metric-aware singular-value invariants changed")

    raw_before = frobenius_sq(r)
    raw_after = frobenius_sq(rp)
    if raw_before == raw_after:
        raise AssertionError("raw Euclidean proxy failed to expose basis dependence")

    # Exact derivative rule for parameter-dependent bases at eta=0,
    # with G_H(0)=G_inf(0)=I:
    # R_tilde' = R' - K_inf R + R K_H.
    r1 = matrix([[F(1, 2), F(-3, 4)], [0, 7]])
    kh = matrix([[0, 1], [0, 0]])
    ki = matrix([[0, 0], [1, 0]])
    r1_tilde = add(sub(r1, matmul(ki, r)), matmul(r, kh))
    det_slope_before = trace2(matmul(inv2(r), r1))
    det_slope_after = trace2(matmul(inv2(r), r1_tilde))
    if det_slope_before != det_slope_after:
        raise AssertionError("log-determinant derivative covariance failed")

    singular_rejected = False
    try:
        inv2(matrix([[1, 2], [2, 4]]))
    except ValueError:
        singular_rejected = True
    if not singular_rejected:
        raise AssertionError("singular basis negative control was not rejected")

    return {
        "schema_version": 1,
        "artifact_kind": "synthetic_boundary_response_basis_covariance_control",
        "status": "all_exact_fraction_controls_pass",
        "scope": "synthetic 2x2 algebra only; no EGB field equation or physical branch claim",
        "response_matrix": encode_matrix(r),
        "horizon_basis_change": encode_matrix(gh),
        "infinity_basis_change": encode_matrix(gi),
        "transformed_response_matrix": encode_matrix(rp),
        "determinants": {
            "original": encode_fraction(det2(r)),
            "transformed": encode_fraction(det2(rp)),
            "covariance_rhs": encode_fraction(det_expected),
        },
        "metric_transport": {
            "W_H_transformed": encode_matrix(whp),
            "W_infinity_transformed": encode_matrix(wip),
            "generalized_spectrum_invariants_before": [encode_fraction(x) for x in spec_before],
            "generalized_spectrum_invariants_after": [encode_fraction(x) for x in spec_after],
            "interpretation": "trace and determinant of the metric-normalized squared response are invariant; these determine the two squared singular values",
        },
        "coordinate_dependence": {
            "raw_frobenius_squared_before": encode_fraction(raw_before),
            "raw_frobenius_squared_after": encode_fraction(raw_after),
            "different": True,
        },
        "parameter_dependent_basis_derivative": {
            "R1_synthetic": encode_matrix(r1),
            "K_H": encode_matrix(kh),
            "K_infinity": encode_matrix(ki),
            "R1_transformed": encode_matrix(r1_tilde),
            "log_determinant_slope_before": encode_fraction(det_slope_before),
            "log_determinant_slope_after": encode_fraction(det_slope_after),
        },
        "negative_controls": {
            "singular_basis_rejected": singular_rejected,
        },
        "claim_ceiling": "This test validates basis-covariance algebra and metric transport for a synthetic 2x2 matrix only. It does not validate the gravitational boundary map, its analytic dependence, or finite-coupling invertibility.",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = run_checks()
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Exact projector and Frobenius-resonance reduction for the Einstein-limit residue.

Uses only the Python standard library.  It deliberately does not infer A1:
the singular-branch logarithm is decided only after the next coefficient
matrix of the reduced regular-singular system has been independently derived.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from fractions import Fraction
from pathlib import Path
from typing import Iterable

F = Fraction


def parse_matrix(values: list[list[str]]) -> list[list[Fraction]]:
    return [[F(value) for value in row] for row in values]


def matmul(a: list[list[Fraction]], b: list[list[Fraction]]) -> list[list[Fraction]]:
    if not a or not b or len(a[0]) != len(b):
        raise ValueError("matrix dimension mismatch")
    return [
        [sum((a[i][k] * b[k][j] for k in range(len(b))), F(0))
         for j in range(len(b[0]))]
        for i in range(len(a))
    ]


def add(a: list[list[Fraction]], b: list[list[Fraction]]) -> list[list[Fraction]]:
    return [[x + y for x, y in zip(ar, br)] for ar, br in zip(a, b)]


def scale(c: Fraction, a: list[list[Fraction]]) -> list[list[Fraction]]:
    return [[c * x for x in row] for row in a]


def transpose(a: list[list[Fraction]]) -> list[list[Fraction]]:
    return [list(col) for col in zip(*a)]


def identity(n: int) -> list[list[Fraction]]:
    return [[F(i == j) for j in range(n)] for i in range(n)]


def rank(a: list[list[Fraction]]) -> int:
    m = [row[:] for row in a]
    rows, cols = len(m), len(m[0])
    pivot_row = 0
    for col in range(cols):
        pivot = next((r for r in range(pivot_row, rows) if m[r][col]), None)
        if pivot is None:
            continue
        m[pivot_row], m[pivot] = m[pivot], m[pivot_row]
        p = m[pivot_row][col]
        m[pivot_row] = [x / p for x in m[pivot_row]]
        for r in range(rows):
            if r != pivot_row and m[r][col]:
                factor = m[r][col]
                m[r] = [x - factor * y for x, y in zip(m[r], m[pivot_row])]
        pivot_row += 1
        if pivot_row == rows:
            break
    return pivot_row


def zero(a: list[list[Fraction]]) -> bool:
    return all(x == 0 for row in a for x in row)


def render_matrix(a: list[list[Fraction]]) -> list[list[str]]:
    return [[str(x) for x in row] for row in a]


def render_vector(v: Iterable[Fraction]) -> list[str]:
    return [str(x) for x in v]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--indicial", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    source_bytes = args.indicial.read_bytes()
    data = json.loads(source_bytes)
    semantic_input = {
        "residue_A0": data["residue_A0"],
        "inhomogeneous_g0": data["inhomogeneous_g0"],
    }
    semantic_bytes = json.dumps(
        semantic_input, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    a0 = parse_matrix(data["residue_A0"])
    g0 = [F(x) for x in data["inhomogeneous_g0"]]
    if len(a0) != 4 or any(len(row) != 4 for row in a0):
        raise AssertionError("expected a 4x4 residue matrix")
    if len(g0) != 4:
        raise AssertionError("expected a four-vector source")

    u = [F(0), F(1), F(0), F(3)]
    w = [F(0), F(0), F(2), -F(1, 3)]
    u_col = [[x] for x in u]
    w_row = [w]
    a0_from_outer = matmul(u_col, w_row)
    if a0 != a0_from_outer:
        raise AssertionError("rank-one residue factorization changed")
    if sum((w[i] * u[i] for i in range(4)), F(0)) != -1:
        raise AssertionError("left/right eigenvector normalization changed")

    minus = scale(F(-1), a0)
    zero_projector = add(identity(4), a0)
    if not zero(add(matmul(minus, minus), scale(F(-1), minus))):
        raise AssertionError("P_- is not idempotent")
    if not zero(add(matmul(zero_projector, zero_projector),
                    scale(F(-1), zero_projector))):
        raise AssertionError("P_0 is not idempotent")
    if not zero(add(add(minus, zero_projector), scale(F(-1), identity(4)))):
        raise AssertionError("spectral projectors do not sum to identity")
    if not zero(matmul(minus, zero_projector)):
        raise AssertionError("spectral projectors are not orthogonal")
    if rank(a0) != 1:
        raise AssertionError("residue rank changed")
    if not zero(add(matmul(a0, a0), a0)):
        raise AssertionError("minimal-polynomial identity A0^2 + A0 = 0 failed")

    left_null = [
        [F(1), F(0), F(0), F(0)],
        [F(0), F(0), F(1), F(0)],
        [F(0), -F(3), F(0), F(1)],
    ]
    for covector in left_null:
        if not zero(matmul([covector], a0)):
            raise AssertionError("listed left-null covector is not in ker(A0^T)")
    if rank(left_null) != 3:
        raise AssertionError("left-null basis is not independent")

    # Verify the affine horizon compatibility and its duplicate fourth row.
    residue_source = [[x] for x in g0]
    # Verify that g0 is aligned with the -1 eigenvector.
    if g0 != [F(0), -F(8, 3), F(0), -F(8)]:
        raise AssertionError("inhomogeneous residue source changed")
    if not zero(add(residue_source, scale(F(8, 3), u_col))):
        raise AssertionError("g0 is not -(8/3)u")

    # Exact horizon-adapted coordinates: three regular coordinates and one
    # singular-mode coordinate. Columns of T are eigenvectors of A0.
    transform = [
        [F(1), F(0), F(0), F(0)],
        [F(0), F(1), F(0), F(1)],
        [F(0), F(0), F(1), F(0)],
        [F(0), F(0), F(6), F(3)],
    ]
    inverse_transform = [
        [F(1), F(0), F(0), F(0)],
        [F(0), F(1), F(2), -F(1, 3)],
        [F(0), F(0), F(1), F(0)],
        [F(0), F(0), -F(2), F(1, 3)],
    ]
    diagonal_residue = matmul(matmul(inverse_transform, a0), transform)
    expected_diagonal = [
        [F(0), F(0), F(0), F(0)],
        [F(0), F(0), F(0), F(0)],
        [F(0), F(0), F(0), F(0)],
        [F(0), F(0), F(0), F(-1)],
    ]
    if diagonal_residue != expected_diagonal:
        raise AssertionError("horizon-adapted basis failed to diagonalize A0")
    if rank(transform) != 4:
        raise AssertionError("horizon-adapted basis is not invertible")
    if matmul(inverse_transform, transform) != identity(4):
        raise AssertionError("horizon-adapted inverse transform is incorrect")
    transformed_source = matmul(inverse_transform, residue_source)
    expected_transformed_source = [[F(0)], [F(0)], [F(0)], [-F(8, 3)]]
    if transformed_source != expected_transformed_source:
        raise AssertionError("transformed inhomogeneous source changed")

    result = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_spin2_mp_frobenius_resonance_reduction",
        "status": "exact_resonance_obstruction_reduced_to_A1_projection",
        "input": {
            "artifact": args.indicial.name,
            "semantic_sha256": hashlib.sha256(semantic_bytes).hexdigest(),
            "public_git_blob_sha1": "c323e46bffbaba7602c40151db48926cfac3b180",
            "residue_A0": render_matrix(a0),
        },
        "residue_factorization": {
            "A0_equals_u_wT": True,
            "u": render_vector(u),
            "w": render_vector(w),
            "wT_u": "-1",
            "eigenvalue_minus_one_right_vector": render_vector(u),
            "eigenvalue_minus_one_left_covector": render_vector(w),
        },
        "spectral_projectors": {
            "P_minus_equals_minus_A0": render_matrix(minus),
            "P_zero_equals_I_plus_A0": render_matrix(zero_projector),
            "ranks": {"P_minus": 1, "P_zero": 3},
            "orthogonal_idempotent_decomposition": True,
        },
        "horizon_adapted_basis": {
            "definition": "U = T * U_tilde",
            "T_columns": render_matrix(transform),
            "T_inverse": render_matrix(inverse_transform),
            "transformed_residue": render_matrix(diagonal_residue),
            "transformed_source": render_vector([row[0] for row in transformed_source]),
            "transformed_state": [
                "Bhat2",
                "Bhat2_prime + 2*Hhat2 - Hhat2_prime/3",
                "Hhat2",
                "Hhat2_prime/3 - 2*Hhat2",
            ],
            "regular_coordinate_count": 3,
            "singular_coordinate": "Hhat2_prime/3 - 2*Hhat2",
            "inhomogeneous_horizon_value_of_singular_coordinate": "-8/3",
            "fixed_Bhat2_horizon_data": "U_tilde(0)=(0, Bhat2_prime(1)+8/3, Hhat2(1), -8/3)",
            "free_horizon_parameters_after_Bhat2_1_zero": [
                "Bhat2_prime(1)", "Hhat2(1)"
            ],
            "diagonalization_exact": True,
        },
        "analytic_branch": {
            "series": "U(y)=sum_(n>=0) u_n y^n",
            "source": "g(y)=sum_(n>=0) g_n y^n",
            "operator": "A(y)=sum_(n>=0) A_n y^n",
            "zeroth_order": "A0*u0 + g0 = 0",
            "recurrence": "(n*I-A0)*u_n = sum_(k=1..n) A_k*u_(n-k) + g_n, n>=1",
            "all_positive_order_matrices_invertible": True,
            "reason": "spec(A0)={0,-1}; n is a positive integer",
            "regular_solution_count_before_fixed_horizon_normalization": 3,
        },
        "singular_branch": {
            "leading_exponent": -1,
            "leading_vector": render_vector(u),
            "series": "U_sing=y^(-1)*(v0+v1*y+...) + r0*log(y) + ...",
            "first_resonant_equation_without_log": "-A0*v1 = A1*v0",
            "log_obstruction": "P_zero*A1*v0",
            "log_coefficient_convention": "r0 = P_zero*A1*v0 for the displayed +r0*log(y) ansatz",
            "log_free_iff": "P_zero*A1*u = 0, equivalently A1*u is in span(u)",
            "equivalent_exact_conditions": [
                "(A1*u)_0 = 0",
                "(A1*u)_2 = 0",
                "(A1*u)_3 - 3*(A1*u)_1 = 0",
            ],
            "higher_singular_recurrence_matrices_invertible_after_n_equals_1": True,
            "interpretation": "the only integer-gap obstruction is the n=1 recurrence of the exponent -1 branch",
        },
        "required_next_coefficient_data": {
            "A1_definition": "A1 = dA/dy at y=0 in y*U'=A(y)U+g(y)",
            "not_computed": ["A1", "log_obstruction"],
            "equivalent_reduced_ode_inputs": ["M2", "C1"],
            "where": "M=M0+y*M1+y^2*M2+..., C=C0+y*C1+...; d1=coefficient of y in det(M)=1/2",
            "formula": "K0=adj(M0)/d1; d2=det(M1)+tr(adj(M0)*M2); K1=adj(M1)/d1-(d2/d1^2)*adj(M0); A1_lower=-(K1*C0+K0*C1)",
        },
        "controls": {
            "rank_A0_one": True,
            "A0_squared_plus_A0_zero": True,
            "projectors_idempotent": True,
            "projectors_sum_to_identity": True,
            "projectors_product_zero": True,
            "left_null_basis_exact": True,
            "inhomogeneous_source_aligned_with_minus_one_eigenvector": True,
        },
        "claim_ceiling": "Exact linear-algebra/Frobenius consequence of the committed Einstein-limit residue. No value of A1 is claimed; the logarithmic status of the singular branch remains open until M2 and C1 are extracted from the source equations.",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
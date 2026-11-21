#!/usr/bin/env python3
"""Exact general-lambda horizon principal block after E_f algebraic reduction.

Input is emitted by derive_slow_rotation_second_order.py --emit-equations.
The reduced principal matrix is extracted from the original E_b/E_h rows,
using the algebraic E_f row to eliminate F2 and its differentiated chain rule.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import platform
import time
from pathlib import Path

import sympy as sp

META = {
    "b": (18348, "cf3f62d0ea589e3dbfcc2ac4eedce259f5759cb1c372da03d45c2930ebc832ff", "47e9f330a3d13ad1527dc8acccdc2e2b670495b144799753b0764bf99c53b1f8"),
    "f": (8174, "a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa", "31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"),
    "g": (41842, "9a0a56db534806aef37b42c41895d5dd54ab00586ea72e77d6735c64fcba015c", "8a6b1ee6df529fa2be2009a0757db2caca3aeb086f0ba243e154019e6949dbee"),
    "h": (38349, "6b874e5b02dc1a7314242a42ba39275f6ab93f5031d652b516ccc4db8567dc0d", "5dcbb3e55cd409b5a34bf15f90064db616076716c5fa7352a514acef11ac78e1"),
}


def exact_zero(value: sp.Expr, label: str) -> None:
    if sp.factor(value) != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(sp.factor(value))}")


def affine_coefficients(
    expression: sp.Expr,
    jets: tuple[sp.Symbol, ...],
    wanted: tuple[sp.Symbol, ...],
):
    formal = sp.symbols(f"z0:{len(jets)}")
    numerator, denominator = sp.together(
        expression.xreplace(dict(zip(jets, formal)))
    ).as_numer_denom()
    polynomial = sp.Poly(numerator, *formal, domain="EX")
    if polynomial.total_degree() > 1:
        raise AssertionError("equation is not affine in the field jets")
    coefficients = {}
    for jet in wanted:
        index = jets.index(jet)
        monomial = tuple(1 if j == index else 0 for j in range(len(jets)))
        coefficients[jet] = sp.cancel(
            polynomial.coeff_monomial(monomial) / denominator
        )
    return coefficients, sp.factor(denominator), len(polynomial.monoms())


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--equations", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    started = time.perf_counter()

    input_bytes = args.equations.read_bytes()
    data = json.loads(input_bytes)
    raw = data["equation_system"]["equations"]
    for key, (expected_length, expected_srepr, expected_text) in META.items():
        actual_text = hashlib.sha256(raw[key].encode("utf-8")).hexdigest()
        if len(raw[key]) != expected_length:
            raise AssertionError(f"E_{key}: equation length mismatch")
        if data["equation_system"]["equation_forms"][key]["equation_sha256"] != expected_srepr:
            raise AssertionError(f"E_{key}: symbolic fingerprint mismatch")
        if actual_text != expected_text:
            raise AssertionError(f"E_{key}: equation text SHA-256 mismatch")

    r, r_h, alpha = sp.symbols("r r_H alpha", positive=True, finite=True)
    rho, lam, qbar = sp.symbols("rho lambda Qbar", positive=True, finite=True)
    Bf, Ff, Hf = (sp.Function(name) for name in ("B2", "F2", "H2"))
    local = {
        "r": r, "r_H": r_h, "alpha": alpha,
        "B2": Bf, "F2": Ff, "H2": Hf,
        "sqrt": sp.sqrt, "Derivative": sp.Derivative,
    }
    B, Bp, Bpp, F, Fp, H, Hp, Hpp = sp.symbols("B Bp Bpp F Fp H Hp Hpp")
    jets = (B, Bp, Bpp, F, Fp, H, Hp, Hpp)
    jet_map = {
        Bf(r): B, sp.diff(Bf(r), r): Bp, sp.diff(Bf(r), r, 2): Bpp,
        Ff(r): F, sp.diff(Ff(r), r): Fp,
        Hf(r): H, sp.diff(Hf(r), r): Hp, sp.diff(Hf(r), r, 2): Hpp,
    }
    radical = sp.sqrt(1 + alpha * (alpha + 2*r_h**2) / r**4)
    substitution = {r_h: 1, alpha: lam, r: rho, radical: qbar/rho**2}
    equations = {
        key: sp.sympify(raw[key], locals=local).xreplace(jet_map).subs(substitution)
        for key in ("b", "f", "h")
    }

    delta = rho**2 + qbar - lam - 2
    scaled_f = sp.cancel(sp.together(delta**2 * equations["f"]))
    cf, _, mon_f = affine_coefficients(scaled_f, jets, (F, Bp, Hp))
    if cf[F] == 0:
        raise AssertionError("scaled E_f algebraic pivot vanished identically")
    f_bprime = sp.cancel(-cf[Bp] / cf[F])
    f_hprime = sp.cancel(-cf[Hp] / cf[F])

    cb, _, mon_b = affine_coefficients(equations["b"], jets, (Bpp, Hpp, Fp))
    ch, _, mon_h = affine_coefficients(equations["h"], jets, (Bpp, Hpp, Fp))
    matrix = sp.Matrix([
        [
            sp.cancel(cb[Bpp] + cb[Fp] * f_bprime),
            sp.cancel(cb[Hpp] + cb[Fp] * f_hprime),
        ],
        [
            sp.cancel(ch[Bpp] + ch[Fp] * f_bprime),
            sp.cancel(ch[Hpp] + ch[Fp] * f_hprime),
        ],
    ])

    # Exact total derivatives along Qbar(rho), evaluated at rho=1.
    horizon = {rho: 1, qbar: lam + 1}
    qprime_h = 2 / (lam + 1)
    matrix_0 = matrix.applyfunc(
        lambda value: sp.factor(sp.cancel(value.subs(horizon)))
    )
    matrix_1 = matrix.applyfunc(
        lambda value: sp.factor(sp.cancel(
            (sp.diff(value, rho) + qprime_h * sp.diff(value, qbar)).subs(horizon)
        ))
    )

    expected_0 = sp.Matrix([
        [sp.Rational(3, 4) * (lam + 1)**2, -(lam + 1)/4],
        [-(lam + 1)/4, sp.Rational(1, 12)],
    ])
    expected_1 = sp.Matrix([
        [-sp.Rational(3, 4) * (lam**2 - 3), (lam + 1)/4],
        [(lam + 1)/4, -(lam**2 + 4*lam - 3)/(12*(lam + 1)**2)],
    ])
    for i in range(2):
        for j in range(2):
            exact_zero(matrix_0[i, j] - expected_0[i, j], f"M0[{i},{j}]")
            exact_zero(matrix_1[i, j] - expected_1[i, j], f"M1[{i},{j}]")
    exact_zero(matrix_0.det(), "horizon determinant")

    determinant_linear = sp.factor(
        matrix_0[0, 0] * matrix_1[1, 1]
        + matrix_1[0, 0] * matrix_0[1, 1]
        - matrix_0[0, 1] * matrix_1[1, 0]
        - matrix_1[0, 1] * matrix_0[1, 0]
    )
    exact_zero(determinant_linear - sp.Rational(1, 2),
               "linear determinant coefficient")

    null_vector = sp.Matrix([1, 3*(lam + 1)])
    exact_zero((matrix_0 * null_vector)[0], "null-vector component 1")
    exact_zero((matrix_0 * null_vector)[1], "null-vector component 2")

    # Reduce the E_f pivot and principal-matrix denominators by the exact
    # radical relation Qbar^2=lambda^2+2lambda+rho^4.
    radical_relation = qbar**2 - (lam**2 + 2*lam + rho**4)
    positive_pivot = 2*rho**2 * (
        qbar*(lam**3 + lam**2*rho**2 + 4*lam**2 + 2*lam*rho**2
              + 4*lam + 4*rho**6)
        + lam**3*rho**2 + 3*lam**2*rho**4 + 4*lam**2*rho**2
        + 6*lam*rho**4 + 4*lam*rho**2 + 4*rho**8
    )
    matrix_denominators = [
        sp.factor(sp.denom(sp.together(matrix[i, j])))
        for i in range(2) for j in range(2)
    ]
    expected_denominators = [
        2*positive_pivot,
        2*rho*(qbar + rho**2)*positive_pivot,
        2*rho*(qbar + rho**2)*positive_pivot,
        6*qbar*rho**3*(qbar + rho**2)**3*positive_pivot,
    ]
    for index, (actual, expected) in enumerate(zip(matrix_denominators, expected_denominators)):
        remainder = sp.factor(sp.rem(
            sp.Poly(actual - expected, qbar, domain="EX"),
            sp.Poly(radical_relation, qbar, domain="EX"),
        ).as_expr())
        exact_zero(remainder, f"principal denominator {index}")

    # Prove that the original E_f coefficient of F2 has a positive numerator.
    cf_original, _, _ = affine_coefficients(equations["f"], jets, (F,))
    af_numerator, af_denominator = sp.together(cf_original[F]).as_numer_denom()
    af_remainder = sp.factor(sp.rem(
        sp.Poly(af_numerator, qbar, domain="EX"),
        sp.Poly(radical_relation, qbar, domain="EX"),
    ).as_expr())
    exact_zero(af_remainder - 3*positive_pivot, "E_f positive numerator")
    expected_af_denominator = 2*qbar*rho*(qbar + rho**2)*delta**2
    exact_zero(af_denominator - expected_af_denominator, "E_f pivot denominator")
    scaled_pivot_h = sp.factor(
        sp.cancel(sp.together(delta**2 * cf_original[F])).subs(horizon)
    )
    exact_zero(scaled_pivot_h - 3*(lam + 2)**2, "scaled E_f horizon pivot")
    exact_zero(
        positive_pivot.subs(horizon) - 2*(lam + 1)*(lam + 2)**3,
        "positive E_f numerator horizon value",
    )

    fractional_powers = [
        power for entry in matrix for power in entry.atoms(sp.Pow)
        if not power.exp.is_Integer
    ]
    if fractional_powers:
        raise AssertionError(f"fractional powers remain in reduced matrix: {fractional_powers}")

    result = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_spin2_reduced_principal_horizon_operator",
        "status": "exact_general_lambda_horizon_principal_block_derived",
        "input_equation_json_sha256": hashlib.sha256(input_bytes).hexdigest(),
        "source_equation_fingerprints_match": True,
        "reduction": {
            "algebraic_equation": "Delta_bar^2*E_f^(2)=0",
            "differential_equations": ["E_b^(2)=0", "E_h^(2)=0"],
            "state": ["Bhat2", "Hhat2"],
            "algebraically_reconstructed_field": "Fhat2",
            "principal_unknowns": ["Bhat2''", "Hhat2''"],
            "affine_jet_counts": {"scaled_E_f": mon_f, "E_b": mon_b, "E_h": mon_h},
        },
        "horizon": {
            "coordinate": "y=rho-1",
            "M0": [[sp.sstr(matrix_0[i, j]) for j in range(2)] for i in range(2)],
            "M1": [[sp.sstr(matrix_1[i, j]) for j in range(2)] for i in range(2)],
            "M0_factorization": "(1/12)*[3*(lambda+1),-1]^T*[3*(lambda+1),-1]",
            "rank_M0": 1,
            "right_null_vector": ["1", "3*(lambda+1)"],
            "det_M0": "0",
            "det_M_first_y_coefficient": "1/2",
            "det_M": "(1/2)*y+O(y^2)",
        },
        "pivot_and_denominator_certificate": {
            "positive_E_f_numerator": sp.sstr(positive_pivot),
            "unscaled_E_f_F2_coefficient": "3*P/[2*Qbar*rho*(Qbar+rho^2)*Delta_bar^2]",
            "scaled_E_f_horizon_value": "3*(lambda+2)^2",
            "P_horizon_value": "2*(lambda+1)*(lambda+2)^3",
            "positive_reduced_matrix_denominators_mod_radical_relation": [
                sp.sstr(value) for value in expected_denominators
            ],
            "radical_relation": "Qbar^2=lambda^2+2*lambda+rho^4",
            "Delta_bar_derivative": "2*rho+2*rho^3/Qbar>0 for rho>=1, lambda>=0",
            "domain": "rho>1, lambda>=0",
        },
        "controls": {
            "all_four_equation_lengths_and_hashes_match_public_metadata": True,
            "M0_general_lambda_identity": True,
            "M1_general_lambda_identity": True,
            "determinant_linear_coefficient_exactly_one_half": True,
            "null_vector_exact": True,
            "E_f_pivot_positive_numerator_exact": True,
            "principal_denominators_positive_representatives_exact": True,
            "no_fractional_powers_in_reduced_principal_matrix": True,
        },
        "execution": {
            "python": platform.python_version(),
            "sympy": sp.__version__,
            "elapsed_seconds": round(time.perf_counter() - started, 6),
            "arithmetic": "exact symbolic algebra in lambda, rho and Qbar",
        },
        "claim_ceiling": (
            "This establishes the general-lambda horizon value and first radial jet of the "
            "reduced principal matrix, including rank-one horizon degeneracy and a simple "
            "zero of its determinant. The positive E_f pivot and positive principal denominators "
            "exclude extra algebraic poles on the open exterior. It does not prove the full "
            "principal determinant has no additional zeros, the full lower-order operator is "
            "Fuchsian, the boundary index, a finite-coupling BVP solution, interval validation, "
            "or physical hyperbolicity."
        ),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

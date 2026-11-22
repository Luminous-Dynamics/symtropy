#!/usr/bin/env python3
"""Derive the next Einstein-limit Frobenius coefficient for the 5D spin-2 system.

The Gauss--Bonnet coupling is set to zero *before* Euler--Lagrange variation.
This avoids constructing the much larger generic-coupling equation strings.
The script still varies b,f,g,h,w independently and imposes g=r^2 only after
variation, matching the public action-derived convention. It then extracts
E_b^(2), E_f^(2), E_h^(2), eliminates Fhat2 with E_f, and computes A0,A1,g0,g1
for y U' = (A0 + y A1 + ...) U + g0 + y g1 + ..., y=r-1.

Dependencies: Python 3.12+, SymPy 1.14+.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import sympy as sp


def exact_zero(label: str, expression: sp.Expr | sp.MatrixBase) -> None:
    if isinstance(expression, sp.MatrixBase):
        value = expression.applyfunc(lambda item: sp.factor(sp.cancel(item)))
        passed = all(item == 0 for item in value)
    else:
        value = sp.factor(sp.cancel(expression))
        passed = value == 0
    if not passed:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def derive() -> dict[str, object]:
    r, eps = sp.symbols("r epsilon", positive=True)
    b, f, g, h, w = sp.symbols("b f g h w", positive=True)
    bp, fp, gp, hp, wp = sp.symbols("bp fp gp hp wp", real=True)
    bpp, fpp, gpp, hpp, wpp = sp.symbols("bpp fpp gpp hpp wpp", real=True)
    q0 = (b, f, g, h, w)
    q1 = (bp, fp, gp, hp, wp)
    q2 = (bpp, fpp, gpp, hpp, wpp)

    # Five-dimensional Einstein reduced action (alpha=0), exactly as in the
    # public slow-rotation generator, before radial gauge fixing.
    measure = sp.sqrt(f * h / b)
    lagrangian = measure * (
        bp * gp
        + g * bp * hp / (2 * h)
        + b * gp**2 / (2 * g)
        + b * gp * hp / h
        + g * h * wp**2 / 2
        + 2 * b / f * (4 - h / g)
    )

    def total_radial_derivative(expression: sp.Expr) -> sp.Expr:
        return (
            sum(sp.diff(expression, q) * q_prime for q, q_prime in zip(q0, q1))
            + sum(sp.diff(expression, q_prime) * q_second for q_prime, q_second in zip(q1, q2))
        )

    momenta = {
        name: sp.diff(lagrangian, q_prime)
        for name, q_prime in zip(("b", "f", "g", "h", "w"), q1)
    }
    euler_lagrange = {
        name: sp.diff(lagrangian, q) - total_radial_derivative(momenta[name])
        for name, q in zip(("b", "f", "g", "h", "w"), q0)
    }

    # The alpha=0 static background at r_H=1 and the exact O(a) frame-dragging mode.
    B2, F2, H2 = (sp.Function(name)(r) for name in ("B2", "F2", "H2"))
    F0 = 1 - r**-2
    W1 = r**-4
    ansatz = {
        b: F0 + eps**2 * B2,
        f: F0 + eps**2 * F2,
        g: r**2,
        h: r**2 + eps**2 * H2,
        w: eps * W1,
        bp: sp.diff(F0 + eps**2 * B2, r),
        fp: sp.diff(F0 + eps**2 * F2, r),
        gp: 2 * r,
        hp: sp.diff(r**2 + eps**2 * H2, r),
        wp: eps * sp.diff(W1, r),
        bpp: sp.diff(F0 + eps**2 * B2, r, 2),
        fpp: sp.diff(F0 + eps**2 * F2, r, 2),
        gpp: sp.Integer(2),
        hpp: sp.diff(r**2 + eps**2 * H2, r, 2),
        wpp: eps * sp.diff(W1, r, 2),
    }
    rows = {
        key: sp.diff(euler_lagrange[key].subs(ansatz), eps, 2).subs(eps, 0) / 2
        for key in ("b", "f", "h")
    }

    # Replace the radial functions and their jets by algebraic indeterminates.
    B, Bp, Bpp, F, Fp, H, Hp, Hpp = sp.symbols("B Bp Bpp F Fp H Hp Hpp")
    Fpp = sp.Symbol("Fpp")
    jet_map = {
        B2: B, F2: F, H2: H,
        sp.diff(B2, r): Bp, sp.diff(F2, r): Fp, sp.diff(H2, r): Hp,
        sp.diff(B2, r, 2): Bpp, sp.diff(F2, r, 2): Fpp, sp.diff(H2, r, 2): Hpp,
    }
    rows = {key: sp.cancel(value.xreplace(jet_map)) for key, value in rows.items()}
    jets = (B, Bp, Bpp, F, Fp, H, Hp, Hpp)

    def affine_coefficients(expression: sp.Expr) -> tuple[dict[sp.Symbol, sp.Expr], sp.Expr]:
        coefficients = {jet: sp.cancel(sp.diff(expression, jet)) for jet in jets}
        source = sp.cancel(expression - sum(coefficients[jet] * jet for jet in jets))
        return coefficients, source

    cf, sf = affine_coefficients(rows["f"])
    cb, sb = affine_coefficients(rows["b"])
    ch, sh = affine_coefficients(rows["h"])

    # Algebraically reconstruct F2 and differentiate it once to eliminate F2'.
    pivot = cf[F]
    if sp.factor(pivot) == 0:
        raise AssertionError("Einstein-limit E_f algebraic pivot vanished")
    k = {jet: sp.cancel(-cf[jet] / pivot) for jet in (B, Bp, H, Hp)}
    k_source = sp.cancel(-sf / pivot)
    F_reconstruction = sum(k[jet] * jet for jet in (B, Bp, H, Hp)) + k_source
    F_prime_reconstruction = sp.cancel(
        sum(sp.diff(k[jet], r) * jet for jet in (B, Bp, H, Hp))
        + sp.diff(k_source, r)
        + k[B] * Bp + k[Bp] * Bpp + k[H] * Hp + k[Hp] * Hpp
    )

    reduced: dict[str, tuple[dict[sp.Symbol, sp.Expr], sp.Expr]] = {}
    for name, coefficients, source in (("b", cb, sb), ("h", ch, sh)):
        expression = sum(coefficients[jet] * jet for jet in jets) + source
        expression = sp.cancel(expression.subs(
            {F: F_reconstruction, Fp: F_prime_reconstruction}, simultaneous=True
        ))
        reduced_coefficients = {
            jet: sp.cancel(sp.diff(expression, jet))
            for jet in (B, Bp, Bpp, H, Hp, Hpp)
        }
        reduced_source = sp.cancel(
            expression - sum(reduced_coefficients[jet] * jet for jet in reduced_coefficients)
        )
        reduced[name] = (reduced_coefficients, reduced_source)

    M = sp.Matrix([
        [reduced["b"][0][Bpp], reduced["b"][0][Hpp]],
        [reduced["h"][0][Bpp], reduced["h"][0][Hpp]],
    ])
    C = sp.Matrix([
        [reduced["b"][0][jet] for jet in (B, Bp, H, Hp)],
        [reduced["h"][0][jet] for jet in (B, Bp, H, Hp)],
    ])
    source = sp.Matrix([reduced["b"][1], reduced["h"][1]])
    y = sp.symbols("y")

    # Lower two rows of A(y),g(y). Kinematic rows are y*B'=y*U_1 and y*H'=y*U_3.
    lower_A = sp.cancel((-y * M.inv() * C).subs(r, 1 + y))
    lower_g = sp.cancel((-y * M.inv() * source).subs(r, 1 + y))
    full_A = sp.zeros(4, 4)
    full_g = sp.zeros(4, 1)
    full_A[0, 1] = y
    full_A[2, 3] = y
    for column in range(4):
        full_A[1, column] = lower_A[0, column]
        full_A[3, column] = lower_A[1, column]
    full_g[1, 0] = lower_g[0]
    full_g[3, 0] = lower_g[1]

    def value_and_derivative_at_horizon(expression: sp.Expr) -> tuple[sp.Expr, sp.Expr]:
        value = sp.factor(sp.limit(expression, y, 0))
        derivative = sp.factor(sp.limit(sp.diff(expression, y), y, 0))
        return value, derivative

    A0 = sp.Matrix(4, 4, lambda i, j: value_and_derivative_at_horizon(full_A[i, j])[0])
    A1 = sp.Matrix(4, 4, lambda i, j: value_and_derivative_at_horizon(full_A[i, j])[1])
    g0 = sp.Matrix([value_and_derivative_at_horizon(full_g[i, 0])[0] for i in range(4)])
    g1 = sp.Matrix([value_and_derivative_at_horizon(full_g[i, 0])[1] for i in range(4)])

    expected_A0 = sp.Matrix([
        [0, 0, 0, 0],
        [0, 0, 2, sp.Rational(-1, 3)],
        [0, 0, 0, 0],
        [0, 0, 6, -1],
    ])
    expected_g0 = sp.Matrix([0, sp.Rational(-8, 3), 0, -8])
    exact_zero("A0 matches the public Einstein-limit residue", A0 - expected_A0)
    exact_zero("g0 matches the public Einstein-limit source", g0 - expected_g0)

    # Projectors and the resonant exponent -1.
    P0 = sp.eye(4) + A0
    Pminus = -A0
    singular_vector = sp.Matrix([0, 1, 0, 3])
    exact_zero("singular eigenvector", A0 * singular_vector + singular_vector)
    exact_zero("zero projector idempotence", P0**2 - P0)
    exact_zero("minus projector idempotence", Pminus**2 - Pminus)
    exact_zero("projectors orthogonal", P0 * Pminus)
    log_coefficient = sp.simplify(P0 * A1 * singular_vector)
    exact_zero("log coefficient lies in regular eigenspace", A0 * log_coefficient)

    # The analytic branch has two free data after B(1)=0 and the leading compatibility.
    b1, h0 = sp.symbols("b1 h0")
    U0_regular = sp.Matrix([0, b1, h0, 6 * h0 - 8])
    U1_regular = sp.simplify((sp.eye(4) - A0).inv() * (A1 * U0_regular + g1))
    expected_U1 = sp.Matrix([b1, 24 - 3 * b1, 6 * h0 - 8, 22 * h0 - 16])
    exact_zero("analytic first recurrence", U1_regular - expected_U1)

    # Exact Myers--Perry control through the first recurrence.
    U0_mp = sp.Matrix([0, -4, 1, -2])
    U1_mp = sp.Matrix([-4, 36, -2, 6])
    exact_zero("Myers--Perry leading recurrence", A0 * U0_mp + g0)
    exact_zero("Myers--Perry first recurrence", U1_mp - A0 * U1_mp - A1 * U0_mp - g1)
    mp_substitution = {
        B: -r**-2 + r**-6,
        Bp: sp.diff(-r**-2 + r**-6, r),
        Bpp: sp.diff(-r**-2 + r**-6, r, 2),
        H: r**-2,
        Hp: sp.diff(r**-2, r),
        Hpp: sp.diff(r**-2, r, 2),
    }
    for name in ("b", "h"):
        coefficients, rhs = reduced[name]
        residual = sum(coefficients[jet] * jet for jet in (B, Bp, Bpp, H, Hp, Hpp)) + rhs
        exact_zero(f"Myers--Perry reduced E_{name}", residual.subs(mp_substitution))
    exact_zero("Myers--Perry algebraic F2 reconstruction", (F_reconstruction - (-r**-2 + r**-4)).subs(mp_substitution))

    return {
        "schema_version": 1,
        "artifact_kind": "einstein_limit_next_frobenius_coefficient",
        "status": "exact_alpha_zero_A1_extracted",
        "conventions": {
            "dimension": 5,
            "alpha": "0",
            "r_H": "1",
            "y": "r/r_H - 1",
            "state": ["Bhat2", "Bhat2_prime", "Hhat2", "Hhat2_prime"],
            "reduced_rows": ["E_b^(2)", "E_h^(2)"],
            "algebraic_row": "E_f^(2) reconstructs Fhat2",
            "gauge_order": "vary b,f,g,h,w independently, then impose g=r^2",
        },
        "residue_A0": [[sp.sstr(A0[i, j]) for j in range(4)] for i in range(4)],
        "next_coefficient_A1": [[sp.sstr(A1[i, j]) for j in range(4)] for i in range(4)],
        "source_g0": [sp.sstr(value) for value in g0],
        "source_g1": [sp.sstr(value) for value in g1],
        "projectors": {
            "regular": "P0=I+A0",
            "singular": "Pminus=-A0",
            "regular_rank": 3,
            "singular_rank": 1,
        },
        "singular_exponent_minus_one": {
            "eigenvector": [sp.sstr(value) for value in singular_vector],
            "forced_log_coefficient": [sp.sstr(value) for value in log_coefficient],
            "log_free": bool(log_coefficient == sp.zeros(4, 1)),
            "local_form": "U(y)=y^(-1)*u + c_log*log(y) + O(1)",
        },
        "analytic_horizon_branch": {
            "free_data_after_Bhat2_1_zero": ["Bhat2_prime(1)", "Hhat2(1)"],
            "compatibility": "Hhat2_prime(1)=6*Hhat2(1)-8",
            "U0": ["0", "b1", "h0", "6*h0-8"],
            "U1": [sp.sstr(value) for value in U1_regular],
            "implied_second_derivatives": {
                "Bhat2_second(1)": "24-3*Bhat2_prime(1)",
                "Hhat2_second(1)": "22*Hhat2(1)-16",
            },
        },
        "myers_perry_control": {
            "U0": [sp.sstr(value) for value in U0_mp],
            "U1": [sp.sstr(value) for value in U1_mp],
            "reduced_Eb_Eh_residuals": "exact zero",
            "Ef_reconstruction_error": "exact zero",
            "order_zero_and_order_one_recurrences": "exact zero",
        },
        "checks": {
            "A0_matches_public_indicial_result": True,
            "g0_matches_public_indicial_result": True,
            "singular_eigenvector_exact": True,
            "projectors_idempotent_and_orthogonal": True,
            "log_coefficient_in_regular_eigenspace": True,
            "analytic_first_recurrence_exact": True,
            "myers_perry_reduced_equations_exact": True,
        },
        "sympy_version": sp.__version__,
        "claim_ceiling": (
            "Exact next Frobenius coefficient at alpha=0, independently derived from the alpha-specialized "
            "five-dimensional reduced action. This does not determine finite-coupling residues, infinity modes, "
            "the global boundary map, a finite-coupling BVP, interval existence, or PDE hyperbolicity."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("spin2_mp_frobenius_A1_v1.json"))
    args = parser.parse_args()
    fixture = derive()
    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    print(json.dumps({"output": str(args.output), "sha256": hashlib.sha256(rendered.encode()).hexdigest(), "status": fixture["status"], "checks": fixture["checks"]}, indent=2))


if __name__ == "__main__":
    main()

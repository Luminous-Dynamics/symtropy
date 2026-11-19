#!/usr/bin/env python3
"""Derive the O(a^2) slow-spin EGB radial equations before solving them.

Source action: Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860,
Eqs. (2.8)-(2.12). The action is varied with b,f,g,h,w independent.
Only after obtaining all five Euler-Lagrange equations is g=r^2 imposed.

The spin expansion is
  b=F+a^2 B2+O(a^4), f=F+a^2 F2+O(a^4),
  h=r^2+a^2 H2+O(a^4), w=a W1+O(a^3), g=r^2.
The four even-sector equations are differentiated twice with respect to
the bookkeeping parameter and divided by two. This avoids the expensive
generic series expansion of nested square roots.

This derives a linear inhomogeneous ODE system. It does not solve the
finite-alpha BVP or establish a finite-spin rotating EGB branch.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import sympy as sp


def srepr_digest(expression: sp.Expr) -> str:
    return hashlib.sha256(sp.srepr(expression).encode("utf-8")).hexdigest()


def exact_zero(label: str, expression: sp.Expr) -> None:
    reduced = sp.cancel(expression)
    if sp.simplify(reduced) != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(reduced)}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--emit-equations",
        action="store_true",
        help="include full O(a^2) equations and source/operator splits in JSON",
    )
    args = parser.parse_args()

    r, r_h, alpha, spin = sp.symbols(
        "r r_H alpha a", positive=True, finite=True
    )
    b, f, g, h, w = sp.symbols("b f g h w", positive=True, finite=True)
    bp, fp, gp, hp, wp = sp.symbols(
        "b_p f_p g_p h_p w_p", real=True
    )
    bpp, fpp, gpp, hpp, wpp = sp.symbols(
        "b_pp f_pp g_pp h_pp w_pp", real=True
    )
    eps = sp.symbols("epsilon", real=True)

    # Independent radial functions and first derivatives.
    measure = sp.sqrt(f * h / b)
    L_einstein = measure * (
        bp * gp
        + g * bp * hp / (2 * h)
        + b * gp**2 / (2 * g)
        + b * gp * hp / h
        + g * h * wp**2 / 2
        + 2 * b / f * (4 - h / g)
    )
    L_gauss_bonnet = measure / g * (
        4 * h * bp * gp / g
        + 2 * (4 * g - 3 * h) * (bp * hp / h + h * wp**2)
        - f * bp * hp * gp**2 / (2 * h)
        - f * h * gp**2 * wp**2 / 2
    )
    L_eff = L_einstein + alpha * L_gauss_bonnet / 4

    zero_order = (b, f, g, h, w)
    first_order = (bp, fp, gp, hp, wp)
    second_order = (bpp, fpp, gpp, hpp, wpp)

    def total_derivative(expression: sp.Expr) -> sp.Expr:
        return (
            sum(
                sp.diff(expression, q) * q_prime
                for q, q_prime in zip(zero_order, first_order)
            )
            + sum(
                sp.diff(expression, q_prime) * q_second
                for q_prime, q_second in zip(first_order, second_order)
            )
        )

    momenta = {
        name: sp.diff(L_eff, q_prime)
        for name, q_prime in zip(("b", "f", "g", "h", "w"), first_order)
    }
    equations = {
        name: sp.diff(L_eff, q) - total_derivative(momenta[name])
        for name, q in zip(("b", "f", "g", "h", "w"), zero_order)
    }

    # Exact angular canonical momentum / Euler-Lagrange identity.
    P_w_general = wp * h * measure * (
        g - alpha / 4 * (f * gp**2 / g - 16 + 12 * h / g)
    )
    exact_zero(
        "canonical angular momentum",
        momenta["w"] - P_w_general,
    )
    exact_zero(
        "cyclic w Euler-Lagrange equation",
        equations["w"] + total_derivative(P_w_general),
    )
    assert momenta["f"] == 0

    # Gauge is imposed after all five equations have been varied.
    gauge = {g: r**2, gp: 2 * r, gpp: sp.Integer(2)}
    gauge_equations = {
        name: expression.subs(gauge)
        for name, expression in equations.items()
    }

    # Exact static EGB background and published first-order slow-rotation mode.
    C = 2 * r_h**2 + alpha
    radical = sp.sqrt(1 + alpha * C / r**4)
    F = 1 - C / (r**2 * (1 + radical))
    W1 = C / (r**4 * (1 + radical))
    W1_prime = sp.diff(W1, r)

    # Parity-respecting second-order spin ansatz.
    B2 = sp.Function("B2")(r)
    F2 = sp.Function("F2")(r)
    H2 = sp.Function("H2")(r)
    ansatz = {
        b: F + eps**2 * B2,
        f: F + eps**2 * F2,
        h: r**2 + eps**2 * H2,
        w: eps * W1,
        bp: sp.diff(F + eps**2 * B2, r),
        fp: sp.diff(F + eps**2 * F2, r),
        gp: 2 * r,
        hp: sp.diff(r**2 + eps**2 * H2, r),
        wp: eps * W1_prime,
        bpp: sp.diff(F + eps**2 * B2, r, 2),
        fpp: sp.diff(F + eps**2 * F2, r, 2),
        gpp: sp.Integer(2),
        hpp: sp.diff(r**2 + eps**2 * H2, r, 2),
        wpp: eps * sp.diff(W1, r, 2),
    }

    # O(epsilon^2) coefficients of the complete, post-variation EOM.
    # A direct second derivative is far more efficient than series() here.
    even_equations = {}
    for name in ("b", "f", "g", "h"):
        substituted = gauge_equations[name].subs(ansatz)
        even_equations[name] = sp.diff(substituted, eps, 2).subs(eps, 0) / 2

    # Split into the linear response operator and the known W1-squared source.
    zero_perturbations = {
        B2: sp.Integer(0),
        F2: sp.Integer(0),
        H2: sp.Integer(0),
        sp.diff(B2, r): sp.Integer(0),
        sp.diff(F2, r): sp.Integer(0),
        sp.diff(H2, r): sp.Integer(0),
        sp.diff(B2, r, 2): sp.Integer(0),
        sp.diff(F2, r, 2): sp.Integer(0),
        sp.diff(H2, r, 2): sp.Integer(0),
    }
    sources = {
        name: expression.subs(zero_perturbations)
        for name, expression in even_equations.items()
    }
    linear_operators = {
        name: even_equations[name] - sources[name]
        for name in even_equations
    }

    # Exact alpha=0 Myers-Perry O(a^2) known-answer control.
    u = (r_h / r) ** 2
    B2_mp = -u * (1 - u**2) / r_h**2
    F2_mp = -u * (1 - u) / r_h**2
    H2_mp = u
    mp_substitution = {
        alpha: sp.Integer(0),
        B2: B2_mp,
        F2: F2_mp,
        H2: H2_mp,
        sp.diff(B2, r): sp.diff(B2_mp, r),
        sp.diff(F2, r): sp.diff(F2_mp, r),
        sp.diff(H2, r): sp.diff(H2_mp, r),
        sp.diff(B2, r, 2): sp.diff(B2_mp, r, 2),
        sp.diff(F2, r, 2): sp.diff(F2_mp, r, 2),
        sp.diff(H2, r, 2): sp.diff(H2_mp, r, 2),
    }
    mp_checks = {}
    for name, expression in even_equations.items():
        residual = sp.cancel(expression.subs(mp_substitution))
        passed = sp.simplify(residual) == 0
        mp_checks[name] = passed
        if not passed:
            raise AssertionError(
                f"MP second-order control failed for E_{name}: {sp.sstr(residual)}"
            )

    # The first-order angular equation follows from the exact Noether current.
    K_slow = r**2 - alpha * (F - 1)
    exact_zero("slow-rotation K factorization", K_slow - r**2 * radical)
    P_w_slow = sp.factor(W1_prime * r**3 * K_slow)
    exact_zero("slow-rotation angular flux", P_w_slow + 2 * C)
    exact_zero("linear-spin angular equation", sp.diff(P_w_slow, r))

    # The action-level O(a^2) equations must be affine-linear in B2,F2,H2.
    # Their construction is the Frechet derivative of E[b,f,h,w] at the
    # static background, with the W1-squared terms isolated as sources.

    metadata = {}
    for name, expression in even_equations.items():
        metadata[name] = {
            "equation_chars": len(sp.sstr(expression)),
            "equation_sha256": srepr_digest(expression),
            "source_chars": len(sp.sstr(sources[name])),
            "source_sha256": srepr_digest(sources[name]),
            "linear_operator_chars": len(sp.sstr(linear_operators[name])),
            "linear_operator_sha256": srepr_digest(linear_operators[name]),
            "exact_myers_perry_alpha_zero_residual": "0",
        }

    fixture = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_slow_rotation_second_order_radial_system",
        "source": {
            "paper": "Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860",
            "equations": ["(2.8)", "(2.10)", "(2.11)", "(2.12)", "(2.21)"],
        },
        "conventions": {
            "dimension": 5,
            "signature": "(-++++)",
            "radial_gauge": "g(r)=r^2 imposed after the five Euler-Lagrange equations are formed",
            "spin_parameter": "Omega_H=a/r_H^2",
            "background": "Einstein-branch Boulware-Deser",
        },
        "expansion": {
            "b": "F+a^2*B2+O(a^4)",
            "f": "F+a^2*F2+O(a^4)",
            "g": "r^2",
            "h": "r^2+a^2*H2+O(a^4)",
            "w": "a*W1+O(a^3)",
            "C": "2*r_H^2+alpha",
            "s": "sqrt(1+alpha*C/r^4)",
            "F": "1-C/(r^2*(1+s))",
            "W1": "C/(r^4*(1+s))",
            "W1_prime": "-2*C/(r^5*s)",
        },
        "equation_system": {
            "even_order": "E_b^(2)=0, E_f^(2)=0, E_g^(2)=0, E_h^(2)=0",
            "equation_forms": metadata,
            "source_split": "E_q^(2)=L_q[B2,F2,H2]+S_q[W1^2]",
            "g_constraint_retained": True,
            "second_order_system_is_linear_in_B2_F2_H2": True,
            "E_w_order_one": "E_w^(1)=-dP_w^(1)/dr=0",
        },
        "angular_noether": {
            "K": "r^2-alpha*(F-1)=r^2*s",
            "P_w": "-2*a*(2*r_H^2+alpha)",
            "asymptotic_W": "a*(2*r_H^2+alpha)/2",
            "identity": "P_w=-4*W",
            "K_bound_for_alpha_nonnegative": "K>=r^2>0",
            "symbolic_checks": {
                "K_factorization": "passed",
                "P_w_constant": "passed",
                "E_w_linear_order_zero": "passed",
            },
        },
        "exact_alpha_zero_myers_perry_control": {
            "u": "(r_H/r)^2",
            "B2": "-u*(1-u^2)/r_H^2",
            "F2": "-u*(1-u)/r_H^2",
            "H2": "u",
            "equation_residuals": {name: "0" for name in mp_checks},
            "all_four_even_equations_pass": all(mp_checks.values()),
        },
        "equation_execution": {
            "sympy_version": sp.__version__,
            "coefficient_extraction": "d^2/d(epsilon)^2 at epsilon=0 divided by two",
            "generic_series_expansion_used": False,
            "clean_checkout_replay": False,
            "Rust_solver": "not implemented",
        },
        "claim_ceiling": (
            "The O(a^2) linear inhomogeneous radial system has been symbolically "
            "derived and its alpha=0 Myers-Perry coefficient functions exactly "
            "satisfy all four even equations. The finite-alpha O(a^2) BVP has "
            "not been solved. No finite-spin branch, uniform near-extremal "
            "validity, interval certificate or stability result is claimed."
        ),
    }
    if args.emit_equations:
        fixture["equation_system"]["equations"] = {
            name: sp.sstr(expression)
            for name, expression in even_equations.items()
        }
        fixture["equation_system"]["sources"] = {
            name: sp.sstr(expression)
            for name, expression in sources.items()
        }
        fixture["equation_system"]["linear_operators"] = {
            name: sp.sstr(expression)
            for name, expression in linear_operators.items()
        }

    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

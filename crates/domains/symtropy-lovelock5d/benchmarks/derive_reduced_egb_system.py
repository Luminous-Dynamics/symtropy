#!/usr/bin/env python3
"""Independent symbolic derivation of the 5D equal-spin EGB radial equations.

Research-only SymPy oracle. The effective radial Lagrangian is transcribed from
Brihaye, Kleihaus, Kunz & Radu, JHEP 11 (2010) 089, Eqs. (2.10)-(2.12).
The metric gauge g(r)=r^2 is applied only after varying the unfixed action.

This derives the cohomogeneity-one equations; it is not a generic 5D tensor
engine and does not certify a numerical rotating solution.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import sympy as sp


def digest(expression: sp.Expr) -> str:
    return hashlib.sha256(sp.srepr(expression).encode("utf-8")).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        help="optional path for the canonical JSON equation fixture",
    )
    parser.add_argument(
        "--emit-equations",
        action="store_true",
        help="include full SymPy strings for all gauge-sliced equations",
    )
    args = parser.parse_args()

    r, alpha = sp.symbols("r alpha", positive=True, finite=True)
    b, f, g, h, w = sp.symbols("b f g h w", positive=True, finite=True)
    bp, fp, gp, hp, wp = sp.symbols("b_p f_p g_p h_p w_p", real=True)
    bpp, fpp, gpp, hpp, wpp = sp.symbols(
        "b_pp f_pp g_pp h_pp w_pp", real=True
    )

    # Brihaye et al. (2010), Eq. (2.11). The square-root measure is common.
    radial_measure = sp.sqrt(f * h / b)
    L_einstein = radial_measure * (
        bp * gp
        + g * bp * hp / (2 * h)
        + b * gp**2 / (2 * g)
        + b * gp * hp / h
        + g * h * wp**2 / 2
        + 2 * b / f * (4 - h / g)
    )
    L_gauss_bonnet = radial_measure / g * (
        4 * h * bp * gp / g
        + 2 * (4 * g - 3 * h) * (bp * hp / h + h * wp**2)
        - f * bp * hp * gp**2 / (2 * h)
        - f * h * gp**2 * wp**2 / 2
    )
    L_eff = sp.factor(L_einstein + alpha * L_gauss_bonnet / 4)

    # Full radial total derivative. f has no f' term in the reduced action,
    # but its derivative is retained in D(expr) because the other equations
    # depend on f(r). This is variation before the metric gauge is imposed.
    def total_derivative(expression: sp.Expr) -> sp.Expr:
        zeroth = (b, f, g, h, w)
        first = (bp, fp, gp, hp, wp)
        second = (bpp, fpp, gpp, hpp, wpp)
        return sp.factor(
            sum(sp.diff(expression, q0) * q1 for q0, q1 in zip(zeroth, first))
            + sum(sp.diff(expression, q1) * q2 for q1, q2 in zip(first, second))
        )

    canonical_momenta = {
        "b": sp.diff(L_eff, bp),
        "f": sp.diff(L_eff, fp),
        "g": sp.diff(L_eff, gp),
        "h": sp.diff(L_eff, hp),
        "w": sp.diff(L_eff, wp),
    }
    equations = {
        "b": sp.factor(sp.diff(L_eff, b) - total_derivative(canonical_momenta["b"])),
        "f": sp.factor(sp.diff(L_eff, f) - total_derivative(canonical_momenta["f"])),
        "g": sp.factor(sp.diff(L_eff, g) - total_derivative(canonical_momenta["g"])),
        "h": sp.factor(sp.diff(L_eff, h) - total_derivative(canonical_momenta["h"])),
        "w": sp.factor(sp.diff(L_eff, w) - total_derivative(canonical_momenta["w"])),
    }

    # Eq. (2.12): w is cyclic, so d/dr(dL_eff/dw')=0.
    P_w = wp * h * radial_measure * (
        g - alpha / 4 * (f * gp**2 / g - 16 + 12 * h / g)
    )
    angular_momentum_identity = sp.simplify(canonical_momenta["w"] - P_w)
    assert angular_momentum_identity == 0
    w_euler_lagrange_identity = sp.simplify(
        equations["w"] + total_derivative(P_w)
    )
    assert w_euler_lagrange_identity == 0

    # Confirm f is nondynamical in this radial action and no equation exceeds
    # second radial derivatives, as expected for Lovelock field equations.
    assert canonical_momenta["f"] == 0
    second_derivatives = (bpp, fpp, gpp, hpp, wpp)
    derivative_inventory = {
        name: [str(variable) for variable in second_derivatives if expr.has(variable)]
        for name, expr in equations.items()
    }
    assert all(not variables or all(v in ("b_pp", "g_pp", "h_pp", "w_pp")
                                    for v in variables)
               for variables in derivative_inventory.values())

    # Impose the areal-base gauge only after the five equations have been
    # varied. Keep E_g as a residual/constraint for consistency checks.
    gauge = {g: r**2, gp: 2 * r, gpp: sp.Integer(2)}
    gauge_equations = {
        name: sp.factor(expression.subs(gauge))
        for name, expression in equations.items()
    }
    gauge_P_w = sp.factor(P_w.subs(gauge))
    expected_gauge_P_w = (
        wp * h * sp.sqrt(f * h / b)
        * (r**2 - alpha * (f - 4 + 3 * h / r**2))
    )
    assert sp.simplify(gauge_P_w - expected_gauge_P_w) == 0

    # Exact finite-alpha static Boulware-Deser control. All five equations,
    # including the retained post-gauge E_g constraint, must vanish.
    r_h = sp.symbols("r_H", positive=True, finite=True)
    amplitude = alpha + 2 * r_h**2
    radical = sp.sqrt(1 + alpha * amplitude / r**4)
    F_bd = sp.factor(1 - amplitude / (r**2 * (1 + radical)))
    bd_substitution = {
        b: F_bd, f: F_bd, h: r**2, w: sp.Integer(0),
        bp: sp.diff(F_bd, r), fp: sp.diff(F_bd, r),
        hp: 2 * r, wp: sp.Integer(0),
        bpp: sp.diff(F_bd, r, 2), fpp: sp.diff(F_bd, r, 2),
        hpp: sp.Integer(2), wpp: sp.Integer(0),
    }
    bd_residuals = {
        name: sp.factor(expression.subs(bd_substitution))
        for name, expression in gauge_equations.items()
    }
    assert all(expression == 0 for expression in bd_residuals.values())

    # Exact alpha=0 equal-spin Myers-Perry control in the same g=r^2 gauge.
    omega_h = sp.symbols("Omega_H", positive=True, finite=True)
    q_mp = r_h**2 * omega_h**2
    u_mp = (r_h / r)**2
    D_mp = 1 - q_mp + q_mp * u_mp**2
    b_mp = sp.factor(1 - u_mp / D_mp)
    f_mp = sp.factor(
        (1 - u_mp) * (1 - q_mp * (1 + u_mp)) / (1 - q_mp)
    )
    h_mp = sp.factor(r**2 + r_h**2 * q_mp * u_mp / (1 - q_mp))
    w_mp = sp.factor(omega_h * u_mp**2 / D_mp)
    mp_substitution = {
        b: b_mp, f: f_mp, h: h_mp, w: w_mp, alpha: sp.Integer(0),
        bp: sp.diff(b_mp, r), fp: sp.diff(f_mp, r),
        hp: sp.diff(h_mp, r), wp: sp.diff(w_mp, r),
        bpp: sp.diff(b_mp, r, 2), fpp: sp.diff(f_mp, r, 2),
        hpp: sp.diff(h_mp, r, 2), wpp: sp.diff(w_mp, r, 2),
    }
    mp_residuals = {
        name: sp.factor(expression.subs(mp_substitution))
        for name, expression in gauge_equations.items()
    }
    assert all(expression == 0 for expression in mp_residuals.values())

    # If one substitutes g=r^2 into the action before variation, the g
    # Euler-Lagrange equation is absent. This explicitly records the lost
    # radial equation rather than pretending a gauge-fixed action produced it.
    L_gauge_fixed = sp.factor(L_eff.subs(gauge))
    assert not L_gauge_fixed.has(g, gp, gpp)

    fixture = {
        "schema_version": 1,
        "artifact_kind": "symbolic_5d_equal_spin_egb_reduced_action_derivation",
        "source": {
            "citation": "Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860",
            "equations": ["(2.8)", "(2.9)", "(2.10)", "(2.11)", "(2.12)"],
            "action": "I=(16*pi*G)^-1 integral sqrt(-g)*(R+alpha*L_GB/4)",
            "coordinate_metric": (
                "ds^2=-b*dt^2+dr^2/f+g*dtheta^2"
                "+h*sin(theta)^2*(dphi_1-w*dt)^2"
                "+h*cos(theta)^2*(dphi_2-w*dt)^2"
                "+(g-h)*sin(theta)^2*cos(theta)^2*(dphi_1-dphi_2)^2"
            ),
            "left_invariant_metric": (
                "ds^2=-b*dt^2+dr^2/f"
                "+g/4*(sigma_1^2+sigma_2^2)"
                "+h/4*(sigma_3+2*w*dt)^2"
            ),
        },
        "conventions": {
            "dimension": 5,
            "signature": "(-++++)",
            "radial_variables": ["b", "f", "g", "h", "w"],
            "radial_derivatives": ["b_p", "f_p", "g_p", "h_p", "w_p"],
            "action_gauge": "unfixed while varying",
            "post_variation_gauge": "g(r)=r^2",
            "f_is_nondynamical": True,
            "full_radial_equations": list(equations.keys()),
        },
        "reduced_lagrangian": {
            "L_E": sp.sstr(sp.factor(L_einstein)),
            "L_GB": sp.sstr(sp.factor(L_gauss_bonnet)),
            "L_eff": "L_E + alpha*L_GB/4",
        },
        "canonical_momenta": {
            name: sp.sstr(sp.factor(value))
            for name, value in canonical_momenta.items()
        },
        "equation_digests_before_gauge": {
            name: digest(value) for name, value in equations.items()
        },
        "equation_sizes_before_gauge": {
            name: len(sp.sstr(value)) for name, value in equations.items()
        },
        "second_derivative_inventory": derivative_inventory,
        "angular_noether_current": {
            "general_gauge": sp.sstr(sp.factor(P_w)),
            "g_equals_r2": sp.sstr(gauge_P_w),
            "identity_with_dL_dwp": "dL_eff/dw_p = P_w",
            "w_euler_lagrange": "E_w = -dP_w/dr",
            "symbolic_identity_passed": True,
        },
        "gauge_sliced_equation_digests": {
            name: digest(value) for name, value in gauge_equations.items()
        },
        "gauge_sliced_equation_sizes": {
            name: len(sp.sstr(value)) for name, value in gauge_equations.items()
        },
        "gauge_fixed_action": sp.sstr(L_gauge_fixed),
        "gauge_order_rule": (
            "All E_b,E_f,E_g,E_h,E_w are derived before substituting g=r^2; "
            "the E_g radial constraint is retained after the substitution."
        ),
        "claim_ceiling": (
            "This is an independent symbolic reduction of the published "
            "cohomogeneity-one radial action. It is not a generic 5D tensor "
            "implementation, a nonzero-alpha rotating solution, an interval "
            "enclosure, or an existence/stability theorem."
        ),
    }
    if args.emit_equations:
        fixture["gauge_sliced_equations"] = {
            name: sp.sstr(value) for name, value in gauge_equations.items()
        }

    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

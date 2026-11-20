#!/usr/bin/env python3
"""Derive the horizon Laurent expansion of the spin-squared E_f -> F2 pivot.

This is not a horizon Frobenius solution. It quantifies the singularity of
the open-exterior algebraic elimination map and pins the correct scaling.
"""
from __future__ import annotations
import argparse, json, platform
from pathlib import Path
import sympy as sp


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()

    a, rh, x = sp.symbols("alpha r_H x", positive=True, finite=True)
    r = rh + x
    Q0 = a + rh**2
    q1 = 2*rh**3/Q0
    q2 = 3*rh**2/Q0 - 2*rh**6/Q0**3
    # Taylor expansion of sqrt(alpha^2+2 alpha r_H^2+r^4)
    # through x^2, sufficient for the x^-1 coefficient of the pivot.
    Q = Q0 + q1*x + q2*x**2
    delta = sp.expand(r**2 + Q - a - 2*rh**2)
    N = (
        Q*(a**3 + a**2*r**2 + 4*a**2*rh**2
           + 2*a*r**2*rh**2 + 4*a*rh**4 + 4*r**6)
        + a**3*r**2 + 3*a**2*r**4 + 4*a**2*r**2*rh**2
        + 6*a*r**4*rh**2 + 4*a*r**2*rh**4 + 4*r**8
    )
    pivot = 3*r*N/((r**2+Q)*Q*delta**2)

    delta1 = sp.factor(sp.diff(delta,x).subs(x,0))
    N_h = sp.factor(N.subs(x,0))
    c_m2 = sp.factor(sp.limit(x**2*pivot,x,0,dir="+"))
    c_m1 = sp.factor(sp.limit(x*(pivot-c_m2/x**2),x,0,dir="+"))

    expected_delta1 = 2*rh*(a+2*rh**2)/(a+rh**2)
    expected_N_h = (a+rh**2)*(a+2*rh**2)**3
    expected_m2 = 3*(a+rh**2)**2/(4*rh)
    expected_m1 = 3*rh**2
    assert sp.factor(delta1-expected_delta1) == 0
    assert sp.factor(N_h-expected_N_h) == 0
    assert sp.factor(c_m2-expected_m2) == 0
    assert sp.factor(c_m1-expected_m1) == 0

    lam, y = sp.symbols("lambda y", positive=True)
    dimensionless = {
        "alpha": "lambda*r_H^2",
        "r_minus_r_H": "y*r_H",
        "pivot_over_r_H": "3*(1+lambda)^2/(4*y^2) + 3/y + O(1)",
    }
    fixture = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_spin2_horizon_algebraic_pivot_laurent",
        "status": "exact_symbolic_laurent_coefficients",
        "source": {
            "parent_artifact": "spin2_constraint_structure_v1.json",
            "pivot": "c_F=3*r*N/((r^2+Q)*Q*Delta^2)",
            "Q": "sqrt(alpha^2+2*alpha*r_H^2+r^4)",
            "Delta": "r^2+Q-alpha-2*r_H^2",
        },
        "expansion": {
            "coordinate": "x=r-r_H",
            "Q_series": "alpha+r_H^2 + 2*r_H^3/(alpha+r_H^2)*x + (3*r_H^2/(alpha+r_H^2)-2*r_H^6/(alpha+r_H^2)^3)*x^2 + O(x^3)",
            "Delta_linear_coefficient": sp.sstr(delta1),
            "N_at_horizon": sp.sstr(N_h),
            "pivot": "c_F = c_-2/x^2 + c_-1/x + O(1)",
            "c_minus_2": sp.sstr(c_m2),
            "c_minus_1": sp.sstr(c_m1),
            "dimensionless": dimensionless,
        },
        "interpretation": {
            "open_exterior": "The algebraic E_f pivot is strictly positive for alpha>=0 and r>r_H.",
            "horizon": "The pivot has a double pole at x=0. This is a singularity of the algebraic elimination representation, not a physical divergence in the metric or a proof that F2 diverges.",
            "regularity_condition": "A finite F2 requires cancellation between the pivot term and the remaining E_f numerator. The full coupled horizon Frobenius recurrence must derive that cancellation; it is not supplied by this pivot-only calculation.",
            "solver_implication": "Use a horizon-regular series/variables on the inner patch and match to the algebraically reduced open-exterior DAE. Do not evaluate F2 by direct division at x=0.",
        },
        "validation": {
            "symbolic_engine": "SymPy "+sp.__version__,
            "python": platform.python_version(),
            "Q_series_order": 2,
            "delta_slope_exact": True,
            "N_horizon_factorization_exact": True,
            "Laurent_coefficients_exact": True,
        },
        "claim_ceiling": "Exact Laurent expansion of the algebraic E_f pivot only; not the full horizon Frobenius expansion, BVP solution, interval enclosure, or existence theorem.",
    }
    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

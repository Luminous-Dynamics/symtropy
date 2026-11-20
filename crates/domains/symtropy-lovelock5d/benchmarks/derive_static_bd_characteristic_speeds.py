#!/usr/bin/env python3
"""Specialize Reall-Tanahashi-Way characteristic speeds to 5D BD EGB.

Research-only exact SymPy derivation. This is the static, spherical branch
only; it is not a rotating principal-symbol calculation.
"""
from __future__ import annotations
import argparse, json, platform
from pathlib import Path
import sympy as sp


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()

    lam, rho = sp.symbols("lambda rho", positive=True, finite=True)
    z = lam * (lam + 2) / rho**4
    q2 = 1 + z

    # Match the Symtropy static BD metric to RTW Eq. (5.15).
    # RTW: F = 1 + rho^2*(1-q)/tilde_alpha_2,
    # q^2 = 1 + 2*tilde_alpha_2*mu/rho^4.
    # In the Symtropy convention: tilde_alpha_2=lambda,
    # mu/r_H^2=(2+lambda)/2, so q=sqrt(q2)=s.
    s = sp.sqrt(q2)
    F_symtropy = 1 - (2 + lam) / (rho**2 * (1 + s))
    F_rtw = 1 + rho**2 * (1 - s) / lam
    assert sp.simplify(F_symtropy - F_rtw) == 0

    # RTW Eqs. (5.13), (5.17), for d=5, kappa=1, Lambda=0.
    d = sp.Integer(5)
    A_source = (
        (sp.Rational(1, 2) + 1 / (d - 3)) / q2
        + sp.Rational(1, 2) - 1 / (d - 3)
    )
    B_source = sp.Integer(0)
    c_t_source = (
        -(1 + 1 / (d - 4)) * A_source
        -(1 - 1 / (d - 4)) / A_source
        + B_source + 3
    )
    c_v_source = A_source
    c_s_source = (
        3 * (1 - 1 / (d - 2)) * A_source
        + (1 - 3 / (d - 2)) / A_source
        - (1 - 2 / (d - 2)) * (B_source + 3)
    )

    c_t = sp.factor(c_t_source)
    c_v = sp.factor(c_v_source)
    c_s = sp.factor(c_s_source)
    expected = (
        sp.factor((1 + 3 * z) / (1 + z)),
        sp.factor(1 / (1 + z)),
        sp.factor((1 - z) / (1 + z)),
    )
    assert sp.simplify(c_t - expected[0]) == 0
    assert sp.simplify(c_v - expected[1]) == 0
    assert sp.simplify(c_s - expected[2]) == 0

    # Exact horizon threshold and degeneracy radius.
    z_h = sp.factor(z.subs(rho, 1))
    c_s_h = sp.factor(c_s.subs(rho, 1))
    lambda_critical = sp.sqrt(2) - 1
    rho_critical = (lam * (lam + 2)) ** sp.Rational(1, 4)

    assert sp.simplify(c_s_h - (2 / (1 + lam)**2 - 1)) == 0
    assert sp.simplify(z_h.subs(lam, lambda_critical) - 1) == 0
    assert [sp.limit(v, rho, sp.oo) for v in (c_t, c_v, c_s)] == [1, 1, 1]
    assert sp.simplify(sp.diff(c_s, rho) - (
        8 * lam * rho**3 * (lam + 2) /
        (lam**2 + 2*lam + rho**4)**2
    )) == 0

    fixture = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_static_bd_characteristic_speeds",
        "status": "exact_symbolic_specialization_of_published_static_lovelock_effective_metrics",
        "source": {
            "paper": "Reall, Tanahashi and Way, Causality and hyperbolicity of Lovelock theories, CQG 31 (2014) 205005",
            "arxiv": "1406.3379",
            "equations": ["(5.13)", "(5.14)", "(5.15)", "(5.17)"],
            "url": "https://arxiv.org/abs/1406.3379"
        },
        "conventions": {
            "dimension": 5,
            "geometry": "static, spherical, asymptotically flat Einstein-branch Boulware-Deser",
            "lambda": "alpha/r_H^2",
            "rho": "r/r_H",
            "z": "lambda*(lambda+2)/rho^4",
            "coupling_map": "RTW tilde_alpha_2=alpha; RTW mu/r_H^2=(2+lambda)/2",
            "Symtropy_static_lapse": "F=1-(2+lambda)/(rho^2*(1+sqrt(1+z)))",
            "RTW_static_lapse": "F=1+rho^2*(1-sqrt(1+z))/lambda"
        },
        "effective_speed_functions": {
            "A": "1/(1+z)",
            "B": "0",
            "tensor": sp.sstr(c_t),
            "vector": sp.sstr(c_v),
            "scalar": sp.sstr(c_s)
        },
        "hyperbolicity_result": {
            "tensor_positive_for": "lambda>=0, rho>=1",
            "vector_positive_for": "lambda>=0, rho>=1",
            "scalar_sign": "sign(1-z)",
            "horizon_scalar_speed": sp.sstr(c_s_h),
            "critical_coupling": sp.sstr(lambda_critical),
            "critical_coupling_decimal": "0.41421356237309504880168872420969807857",
            "critical_radius": sp.sstr(rho_critical),
            "classification": {
                "0<=lambda<sqrt(2)-1": "all three effective characteristic metrics are Lorentzian throughout the open static exterior",
                "lambda=sqrt(2)-1": "scalar characteristic metric reaches degeneracy at the horizon limit; it is positive for rho>1",
                "lambda>sqrt(2)-1": "scalar characteristic metric is non-Lorentzian for 1<rho<rho_c, where rho_c=[lambda*(lambda+2)]^(1/4)"
            },
            "minimum_open_exterior_scalar_margin": "inf_{rho>1} c_S(rho)=2/(1+lambda)^2-1"
        },
        "validation": {
            "symbolic_engine": "SymPy " + sp.__version__,
            "python": platform.python_version(),
            "Symtropy_to_RTW_lapse_match_exact": True,
            "all_three_speed_specializations_exact": True,
            "Einstein_limit_all_speeds_one": True,
            "critical_coupling_exact": True,
            "critical_radius_exact": True,
            "scalar_speed_monotonic_in_rho_for_lambda_nonnegative": True
        },
        "claim_ceiling": "Exact static spherical characteristic-sector specialization only. It establishes the scalar effective-metric signature change for the static 5D BD branch. It does not establish strong hyperbolicity of a chosen gauge-fixed evolution formulation, rotating-background characteristics, mode stability, EFT validity, or nonlinear branch behavior."
    }

    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

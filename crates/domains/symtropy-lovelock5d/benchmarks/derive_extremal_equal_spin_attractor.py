#!/usr/bin/env python3
"""Exact 5D equal-spin EGB extremal near-horizon attractor reference.

Source: Kleihaus, Kunz & Radu, Universe 9 (2023) 156, Eqs. (13)-(20).
This script performs symbolic convention matching and algebraic checks.
It does not solve the global asymptotically-flat boundary-value problem.
"""
from __future__ import annotations
import argparse
import json
import platform
from pathlib import Path
import sympy as sp


def exact_zero(name: str, expression: sp.Expr) -> None:
    reduced = sp.simplify(expression)
    if reduced != 0:
        raise AssertionError(f"{name} failed: {sp.sstr(reduced)}")


def bernstein_coefficients(poly: sp.Expr, var: sp.Symbol,
                           lower: sp.Rational, upper: sp.Rational,
                           degree: int) -> list[sp.Expr]:
    u = sp.symbols("u")
    coefficients = sp.symbols(f"c0:{degree + 1}")
    bernstein = sum(
        coefficients[k] * sp.binomial(degree, k) * u**k * (1-u)**(degree-k)
        for k in range(degree + 1)
    )
    transformed = sp.expand(poly.subs(var, lower + (upper-lower)*u))
    equations = sp.Poly(sp.expand(bernstein-transformed), u).all_coeffs()
    solution = sp.solve(equations, coefficients, dict=True)[0]
    return [sp.factor(solution[c]) for c in coefficients]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    v = sp.symbols("v3", positive=True, real=True)
    alpha_sym, alpha_nh = sp.symbols("alpha_sym alpha_NH", positive=True)
    r, r_h = sp.symbols("r r_H", positive=True)

    radicand = 5*v**4 - 34*v**3 + 73*v**2 - 56*v + 16
    polynomial = 2*v**2 - 7*v + 4
    root = sp.sqrt(radicand)

    # Source coupling is alpha_NH in R + alpha_NH L_GB.
    # Symtropy uses R + alpha_sym L_GB/4, hence alpha_sym=4 alpha_NH.
    V = sp.factor((root-polynomial)/(2-v))  # v2 / alpha_sym
    U1 = sp.factor(
        (V+4-3*v)*(3*V+4-v) /
        (2*(4-v)*(3*V+8-6*v))
    )  # v1 / alpha_sym

    # Exact source formulae after the factor-four coupling conversion.
    v2_source = 4*alpha_nh/(v-2)*(polynomial-root)
    v1_source = (
        (v2_source-4*alpha_nh*(3*v-4))
        *(3*v2_source+4*alpha_nh*(4-v))
        /(2*(4-v)*(3*v2_source+4*alpha_nh*(8-6*v)))
    )
    exact_zero("v2 coupling conversion",
               v2_source.subs(alpha_nh, alpha_sym/4)-alpha_sym*V)
    exact_zero("v1 coupling conversion",
               v1_source.subs(alpha_nh, alpha_sym/4)-alpha_sym*U1)

    # Static BD lapse convention bridge: exact rationalized identity.
    F_sym = 1-(2*r_h**2+alpha_sym)/(
        r**2*(1+sp.sqrt(1+alpha_sym*(alpha_sym+2*r_h**2)/r**4))
    )
    F_source = 1+r**2/(4*alpha_nh)*(
        1-sp.sqrt(1+8*alpha_nh*(r_h**2+2*alpha_nh)/r**4)
    )
    exact_zero("static BD lapse convention map",
               (F_source-F_sym).subs(alpha_sym, 4*alpha_nh))

    # The same source gives the static horizon thermodynamics (Eq. 10).
    mass_source = 3*sp.pi/8*(r_h**2+2*alpha_nh)
    temperature_source = r_h/(2*sp.pi*(r_h**2+4*alpha_nh))
    area_source = 2*sp.pi**2*r_h**3
    entropy_source = sp.pi**2*r_h**3/2*(1+12*alpha_nh/r_h**2)
    mass_sym = 3*sp.pi/16*(2*r_h**2+alpha_sym)
    temperature_sym = r_h/(2*sp.pi*(r_h**2+alpha_sym))
    area_sym = 2*sp.pi**2*r_h**3
    entropy_sym = sp.pi**2*r_h**3/2*(1+3*alpha_sym/r_h**2)
    exact_zero("static mass convention map",
               (mass_source-mass_sym).subs(alpha_sym,4*alpha_nh))
    exact_zero("static temperature convention map",
               (temperature_source-temperature_sym).subs(alpha_sym,4*alpha_nh))
    exact_zero("static area convention map", area_source-area_sym)
    exact_zero("static Wald entropy convention map",
               (entropy_source-entropy_sym).subs(alpha_sym,4*alpha_nh))

    exact_zero("static first law at fixed alpha",
               sp.diff(mass_sym,r_h)-temperature_sym*sp.diff(entropy_sym,r_h))
    x_source = 3*sp.pi/4*alpha_nh/mass_source
    x_sym = alpha_sym/(2*r_h**2+alpha_sym)
    exact_zero("dimensionless static coupling map",
               (x_source-x_sym).subs(alpha_sym,4*alpha_nh))

    # Exact algebraic positivity controls on 0 < v3 < 2.
    bern_01 = bernstein_coefficients(radicand, v, sp.Rational(0), sp.Rational(1), 4)
    bern_12 = bernstein_coefficients(radicand, v, sp.Rational(1), sp.Rational(2), 4)
    if bern_01 != [16, 2, sp.Rational(1,6), 2, 4]:
        raise AssertionError(f"unexpected Bernstein coefficients on [0,1]: {bern_01}")
    if bern_12 != [4, 6, sp.Rational(49,6), 7, 4]:
        raise AssertionError(f"unexpected Bernstein coefficients on [1,2]: {bern_12}")
    exact_zero("R-P^2 factorization",
               radicand-polynomial**2-v**2*(v-4)*(v-2))
    exact_zero("V positive form",
               V-(root-polynomial)/(2-v))
    exact_zero("first positive factor",
               V+4-3*v-(v**2-3*v+4+root)/(2-v))
    exact_zero("v1 denominator positive factor",
               3*V+8-6*v-(v+4+3*root)/(2-v))
    q_poly = 5*v**2-15*v+4
    exact_zero("v1 second numerator factor",
               3*V+4-v-(3*root-q_poly)/(2-v))
    exact_zero("9R-Q^2 factorization",
               9*radicand-q_poly**2
               -4*(v-4)*(v-2)*(v-1)*(5*v-4))

    # Dimensionless invariant functions: J=pi*alpha_sym^(3/2)*Jbar,
    # A_H=alpha_sym^(3/2)*Abar, S=alpha_sym^(3/2)*Sbar.
    Jbar = sp.factor(V*v*sp.sqrt((4-v)*(V+4-3*v))/4)
    Abar = 2*sp.pi**2*V*sp.sqrt(V*v)
    Sbar = sp.pi**2*sp.sqrt(V*v)*(V+4-v)/2
    kbar = sp.factor(
        2*U1*sp.sqrt(4-v)/sp.sqrt(V*v*(V+4-3*v))
    )
    exact_zero("entropy/area ratio",
               Sbar/Abar-(V+4-v)/(4*V))

    # Independent derivation of k from the source expression in Eq. (16).
    J_source = sp.pi/4*v2_source*v*sp.sqrt(
        (4-v)*(v2_source+4*alpha_nh*(4-3*v))
    )
    k_source = (4-v)*sp.sqrt(v2_source*v)*sp.pi*v1_source/(2*J_source)
    exact_zero("dimensionless near-horizon k",
               sp.simplify(k_source.subs(alpha_nh, alpha_sym/4)**2-kbar**2))

    # Critical endpoint: the source gives v3=0, v1=alpha_NH,
    # v2=J=A_H=S=0. v3=2 is a singular endpoint of this parameterization.
    limits = {
        "V_at_v3_zero": sp.limit(V, v, 0, dir="+"),
        "U1_at_v3_zero": sp.limit(U1, v, 0, dir="+"),
        "Jbar_at_v3_zero": sp.limit(Jbar, v, 0, dir="+"),
        "Abar_at_v3_zero": sp.limit(Abar, v, 0, dir="+"),
        "Sbar_at_v3_zero": sp.limit(Sbar, v, 0, dir="+"),
    }
    if limits != {
        "V_at_v3_zero": 0,
        "U1_at_v3_zero": sp.Rational(1,4),
        "Jbar_at_v3_zero": 0,
        "Abar_at_v3_zero": 0,
        "Sbar_at_v3_zero": 0,
    }:
        raise AssertionError(f"critical endpoint limits changed: {limits}")

    samples = []
    for index in range(1, 40):
        value = sp.Rational(index, 20)  # v3 = 0.05 ... 1.95
        samples.append({
            "v3": f"{index}/20",
            "V_v2_over_alpha_sym": str(sp.N(V.subs(v,value), 24)),
            "U1_v1_over_alpha_sym": str(sp.N(U1.subs(v,value), 24)),
            "J_over_pi_alpha_sym_3_2": str(sp.N(Jbar.subs(v,value), 24)),
            "area_over_alpha_sym_3_2": str(sp.N(Abar.subs(v,value), 24)),
            "entropy_over_alpha_sym_3_2": str(sp.N(Sbar.subs(v,value), 24)),
            "k": str(sp.N(kbar.subs(v,value), 24)),
        })

    fixture = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_extremal_equal_spin_near_horizon_attractor",
        "status": "exact_symbolic_specialization_with_dimensionless_reference_grid",
        "source": {
            "paper": "Kleihaus, Kunz and Radu, Phases of Rotating Black Objects in d=5 Einstein-Gauss-Bonnet Theory",
            "journal": "Universe 9 (2023) 156",
            "doi": "10.3390/universe9040156",
            "url": "https://www.mdpi.com/2218-1997/9/4/156",
            "equations": ["(13)", "(16)", "(17)", "(18)", "(19)", "(20)"]
        },
        "conventions": {
            "source_action": "R + alpha_NH * L_GB",
            "symtropy_action": "R + alpha_sym * L_GB/4",
            "exact_coupling_map": "alpha_sym = 4*alpha_NH",
            "dimension": 5,
            "geometry": "extremal equal-angular-momenta rotating squashed AdS2 x S3 near-horizon solution",
            "parameter": "v3 in (0,2); v3 measures homogeneous S3 squashing",
            "metric_data": {
                "f0": "v1*r^2",
                "f1": "v1/r^2",
                "f2": "v2",
                "f3": "v2*v3",
                "w": "-k*r"
            },
            "static_BD_lapse_map": {
                "source": "1 + r^2/(4*alpha_NH)*(1-sqrt(1+8*alpha_NH*(r_H^2+2*alpha_NH)/r^4))",
                "symtropy": "1-(2*r_H^2+alpha_sym)/(r^2*(1+sqrt(1+alpha_sym*(alpha_sym+2*r_H^2)/r^4)))",
                "identity": "exact after alpha_sym=4*alpha_NH"
            }
        },
        "dimensionless_functions": {
            "R": "5*v3^4-34*v3^3+73*v3^2-56*v3+16",
            "P": "2*v3^2-7*v3+4",
            "V": "v2/alpha_sym = (sqrt(R)-P)/(2-v3)",
            "U1": "v1/alpha_sym = (V+4-3*v3)*(3*V+4-v3)/(2*(4-v3)*(3*V+8-6*v3))",
            "Jbar": "J/(pi*alpha_sym^(3/2)) = V*v3*sqrt((4-v3)*(V+4-3*v3))/4",
            "Abar": "A_H/alpha_sym^(3/2) = 2*pi^2*V*sqrt(V*v3)",
            "Sbar": "S_ext/alpha_sym^(3/2) = pi^2*sqrt(V*v3)*(V+4-v3)/2",
            "k": "2*U1*sqrt(4-v3)/sqrt(V*v3*(V+4-3*v3))"
        },
        "exact_positivity_certificate": {
            "R_bernstein_on_0_1": [str(x) for x in bern_01],
            "R_bernstein_on_1_2": [str(x) for x in bern_12],
            "R_positive_on_0_2": True,
            "R_minus_P_squared": "v3^2*(v3-4)*(v3-2) > 0",
            "V_positive": True,
            "J_radical_factor_positive": "V+4-3*v3 = (v3^2-3*v3+4+sqrt(R))/(2-v3) > 0",
            "v1_denominator_factor_positive": "3*V+8-6*v3 = (v3+4+3*sqrt(R))/(2-v3) > 0",
            "v1_second_factor_positive": "3*V+4-v3 = (3*sqrt(R)-(5*v3^2-15*v3+4))/(2-v3) > 0",
            "last_factor_split_point": "(15-sqrt(145))/10",
            "last_factor_split_decimal": str(sp.N((15-sp.sqrt(145))/10, 18)),
            "last_factor_proof": "below the split point the polynomial is positive and 9R-Q^2=4*(v3-4)*(v3-2)*(v3-1)*(5*v3-4)>0; above it Q=5*v3^2-15*v3+4<=0",
            "U1_positive": True,
            "J_AH_S_real_and_positive_for_0_lt_v3_lt_2": True
        },
        "critical_endpoint": {
            "v3_to_zero": {
                "v1_over_alpha_sym": "1/4",
                "v1_equals": "alpha_NH",
                "v2": "0",
                "J": "0",
                "A_H": "0",
                "S_ext": "0"
            },
            "v3_equals_two": "singular endpoint of this parameterization; use a separate limiting procedure, never direct substitution"
        },
        "entropy_area_identity": "S_ext/A_H = (V+4-v3)/(4*V)",
        "reference_grid": {
            "v3_start": "0.05",
            "v3_end": "1.95",
            "step": "0.05",
            "point_count": len(samples),
            "samples": samples
        },
        "validation": {
            "symbolic_engine": "SymPy " + sp.__version__,
            "python": platform.python_version(),
            "action_coupling_conversion_exact": True,
            "static_BD_lapse_conversion_exact": True,
            "v1_v2_dimensionless_reductions_exact": True,
            "J_area_entropy_scalings_exact": True,
            "near_horizon_k_reduction_exact": True,
            "radicand_Bernstein_positivity_exact": True,
            "algebraic_positivity_identities_exact": True,
            "critical_endpoint_limits_exact": True
        },
        "claim_ceiling": "Exact local extremal near-horizon attractor specialization and algebraic reference grid only. It is not a global asymptotically-flat extremal black hole, a matched near-extremal sequence, a solved nonextremal BVP, or a hyperbolicity/stability result."
    }
    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

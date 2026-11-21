#!/usr/bin/env python3
"""Derive exact cubic horizon coefficients for the 5D EGB spin-2 rows."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import sympy as sp

META = {
    "b": (18348, "cf3f62d0ea589e3dbfcc2ac4eedce259f5759cb1c372da03d45c2930ebc832ff",
          "47e9f330a3d13ad1527dc8acccdc2e2b670495b144799753b0764bf99c53b1f8"),
    "f": (8174, "a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa",
          "31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"),
    "g": (41842, "9a0a56db534806aef37b42c41895d5dd54ab00586ea72e77d6735c64fcba015c",
          "8a6b1ee6df529fa2be2009a0757db2caca3aeb086f0ba243e154019e6949dbee"),
    "h": (38349, "6b874e5b02dc1a7314242a42ba39275f6ab93f5031d652b516ccc4db8567dc0d",
          "5dcbb3e55cd409b5a34bf15f90064db616076716c5fa7352a514acef11ac78e1"),
}


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def directional_derivatives(expr, rho, Q, lam):
    """Return d^n/d(rho)^n expr(rho,Q(rho)) at rho=1, n=0..3."""
    q1 = 2 / (lam + 1)
    q2 = 6 / (lam + 1) - 4 / (lam + 1) ** 3
    q3 = 12 / (lam + 1) - 36 / (lam + 1) ** 3 + 24 / (lam + 1) ** 5
    horizon = {rho: 1, Q: lam + 1}
    values = (
        expr,
        sp.diff(expr, rho) + q1 * sp.diff(expr, Q),
        sp.diff(expr, rho, 2) + 2*q1*sp.diff(expr, rho, Q)
        + q1**2*sp.diff(expr, Q, 2) + q2*sp.diff(expr, Q),
        sp.diff(expr, rho, 3) + 3*q1*sp.diff(expr, rho, 2, Q)
        + 3*q1**2*sp.diff(expr, rho, Q, 2) + q1**3*sp.diff(expr, Q, 3)
        + 3*q2*sp.diff(expr, rho, Q) + 3*q1*q2*sp.diff(expr, Q, 2)
        + q3*sp.diff(expr, Q),
    )
    return [sp.factor(value.subs(horizon)) for value in values]


def quotient_taylor(numerator, denominator, denominator_derivatives, rho, Q, lam):
    """Taylor coefficients (not derivatives) of numerator/denominator."""
    n0, n1, n2, n3 = directional_derivatives(numerator, rho, Q, lam)
    d0, d1, d2, d3 = denominator_derivatives
    return [
        sp.factor(n0 / d0),
        sp.factor(n1 / d0 - n0*d1/d0**2),
        sp.factor(n2/(2*d0) - n1*d1/d0**2
                  + n0*(d1**2/d0**3 - d2/(2*d0**2))),
        sp.factor(n3/(6*d0) - n2*d1/(2*d0**2)
                  + n1*(d1**2/d0**3 - d2/(2*d0**2))
                  + n0*(-d1**3/d0**4 + d1*d2/d0**3 - d3/(6*d0**2))),
    ]


def build_scaled_row(expression, key, symbols):
    rho, lam, Q, jets = symbols
    B, Bp, Bpp, F, Fp, H, Hp, Hpp = jets
    delta = rho**2 + Q - lam - 2
    S = sp.Symbol("S", positive=True)
    bases = {p.base for p in expression.atoms(sp.Pow) if not p.exp.is_Integer}
    if len(bases) != 1:
        raise AssertionError(f"{key}: expected exactly one fractional-power base")
    base = next(iter(bases))
    expected_base = 1 - (lam + 2)/(Q + rho**2)
    if sp.simplify(base - expected_base) != 0:
        raise AssertionError(f"{key}: unexpected fractional-power base")
    expanded = expression.xreplace({base: S})
    terms = sp.Add.make_args(sp.expand_mul(expanded))
    powers = {term.as_powers_dict().get(S, sp.Integer(0)) for term in terms}
    if not powers.issubset({sp.Integer(-2), sp.Integer(-1), sp.Integer(0), sp.Integer(1)}):
        raise AssertionError(f"{key}: unexpected fractional powers {powers}")
    regularized = sp.Add(*(term*S**2 for term in terms))
    regularized = regularized.xreplace({S: delta/(Q + rho**2)})
    return sp.cancel(sp.together(delta * regularized))


def row_taylor(expression, key, symbols, expected_denominator, expected_terms):
    rho, lam, Q, jets = symbols
    z = sp.symbols("z0:8")
    scaled = build_scaled_row(expression, key, symbols)
    numerator_expr = scaled.xreplace(dict(zip(jets, z)))
    numerator, denominator = sp.together(numerator_expr).as_numer_denom()
    denominator = sp.factor(denominator)
    if sp.cancel(denominator - expected_denominator) != 0:
        raise AssertionError(f"{key}: common denominator changed: {denominator}")
    polynomial = sp.Poly(numerator, *z, domain="EX")
    if polynomial.total_degree() != 1 or len(polynomial.monoms()) != expected_terms:
        raise AssertionError(f"{key}: expected affine row with {expected_terms} monomials")
    horizon_point = {rho: 1, Q: lam + 1}
    if any(sp.cancel(c.subs(horizon_point)) != 0 for c in polynomial.coeffs()):
        raise AssertionError(f"{key}: scaled coefficient is nonzero at the horizon")
    den_derivatives = directional_derivatives(denominator, rho, Q, lam)
    coefficient_series = {
        monomial: quotient_taylor(coefficient, denominator, den_derivatives, rho, Q, lam)
        for monomial, coefficient in zip(polynomial.monoms(), polynomial.coeffs())
    }
    return coefficient_series


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--equations", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    data = json.loads(args.equations.read_text(encoding="utf-8"))
    raw = data["equation_system"]["equations"]
    r, r_h, alpha = sp.symbols("r r_H alpha", positive=True, finite=True)
    rho, lam, Q = sp.symbols("rho lambda Q", positive=True, finite=True)
    Bf, Ff, Hf = (sp.Function(name) for name in ("B2", "F2", "H2"))
    local = {"r": r, "r_H": r_h, "alpha": alpha, "B2": Bf, "F2": Ff, "H2": Hf,
             "sqrt": sp.sqrt, "Derivative": sp.Derivative}
    parsed = {key: sp.sympify(raw[key], locals=local) for key in ("b", "f", "g", "h")}
    B, Bp, Bpp, F, Fp, H, Hp, Hpp = sp.symbols("B Bp Bpp F Fp H Hp Hpp")
    jets = (B, Bp, Bpp, F, Fp, H, Hp, Hpp)
    jet_map = {
        Bf(r): B, sp.diff(Bf(r), r): Bp, sp.diff(Bf(r), r, 2): Bpp,
        Ff(r): F, sp.diff(Ff(r), r): Fp,
        Hf(r): H, sp.diff(Hf(r), r): Hp, sp.diff(Hf(r), r, 2): Hpp,
    }
    radical = sp.sqrt(1 + alpha*(alpha + 2*r_h**2)/r**4)
    dimensionless = {
        key: value.xreplace(jet_map).subs({r_h: 1, alpha: lam, r: rho, radical: Q/rho**2})
        for key, value in parsed.items()
    }
    for key, expression in dimensionless.items():
        length, srepr_hash, text_hash = META[key]
        if len(raw[key]) != length:
            raise AssertionError(f"{key}: equation text length mismatch")
        if data["equation_system"]["equation_forms"][key]["equation_sha256"] != srepr_hash:
            raise AssertionError(f"{key}: source srepr digest mismatch")
        if sha256(raw[key]) != text_hash:
            raise AssertionError(f"{key}: source text digest mismatch")

    symbols = (rho, lam, Q, jets)
    den_g = Q**4*rho**5*(Q + rho**2)**8
    den_h = 2*Q**3*rho**5*(Q + rho**2)**7
    rows = {
        "g": row_taylor(dimensionless["g"], "g", symbols, den_g, 9),
        "h": row_taylor(dimensionless["h"], "h", symbols, den_h, 7),
    }

    b1, b2, f1, f2, h0, h1 = sp.symbols("b1 b2 f1 f2 h0 h1")
    mB   = (1, 0, 0, 0, 0, 0, 0, 0)
    mBp  = (0, 1, 0, 0, 0, 0, 0, 0)
    mBpp = (0, 0, 1, 0, 0, 0, 0, 0)
    mF   = (0, 0, 0, 1, 0, 0, 0, 0)
    mFp  = (0, 0, 0, 0, 1, 0, 0, 0)
    mH   = (0, 0, 0, 0, 0, 1, 0, 0)
    mHp  = (0, 0, 0, 0, 0, 0, 1, 0)
    mHpp = (0, 0, 0, 0, 0, 0, 0, 1)
    m0   = (0, 0, 0, 0, 0, 0, 0, 0)

    # The cubic assembly below omits several higher Taylor jets. Prove
    # their multipliers vanish at every order at which they could contribute.
    for key, series in rows.items():
        # H2 and H2' may contribute only through h0 and (for E_g) h1.
        for monomial in (mH, mHp):
            coeffs = series.get(monomial, [sp.Integer(0)] * 4)
            if any(sp.simplify(coeffs[n]) != 0 for n in (1, 2)):
                raise AssertionError(
                    f"{key}: H2/H2_prime contributes below cubic order"
                )
        hpp_coeffs = series.get(mHpp, [sp.Integer(0)] * 4)
        if any(sp.simplify(hpp_coeffs[n]) != 0 for n in (0, 1, 2, 3)):
            raise AssertionError(
                f"{key}: H2_second contributes to the cubic horizon coefficient"
            )
        if key == "h" and sp.simplify(series.get(mHp, [0]*4)[3]) != 0:
            raise AssertionError(
                "E_h cubic formula must include H2_prime if its cubic coefficient is nonzero"
            )

        # Exclude b3/f3 terms from B2', F2' and B2'' Taylor expansions.
        for monomial in (mBp, mFp):
            coeffs = series.get(monomial, [sp.Integer(0)] * 4)
            if sp.simplify(coeffs[1]) != 0:
                raise AssertionError(
                    f"{key}: first derivative of a first-jet coefficient is nonzero"
                )
        bpp_coeffs = series.get(mBpp, [sp.Integer(0)] * 4)
        if any(sp.simplify(bpp_coeffs[n]) != 0 for n in (0, 1, 2)):
            raise AssertionError(
                f"{key}: B2_second coefficient would introduce omitted higher jets"
            )

    cubic = {}
    for key, series in rows.items():
        quadratic = sp.factor(series[mB][1]*b1 + series[mBp][2]*b1
                              + series[mF][1]*f1 + series[mFp][2]*f1)
        if quadratic != 0:
            raise AssertionError(f"{key}: fixed-horizon quadratic coefficient did not cancel")
        value = (series[mB][1]*b2/2 + series[mB][2]*b1
                 + series[mBp][2]*b2 + series[mBp][3]*b1
                 + series[mBpp][3]*b2
                 + series[mF][1]*f2/2 + series[mF][2]*f1
                 + series[mFp][2]*f2 + series[mFp][3]*f1
                 + series[mH][3]*h0 + series[m0][3])
        if key == "g":
            value += series[mHp][3]*h1
        cubic[key] = sp.factor(value)

    expected_h_coefficient = -24*(lam + 2)*(3*lam**2 + 6*lam + 1)/(lam + 1)**6
    if sp.factor(sp.diff(cubic["h"], h0) - expected_h_coefficient) != 0:
        raise AssertionError("E_h cubic valuation witness changed")

    mp = {lam: 0, b1: -4, b2: 36, f1: -2, f2: 14, h0: 1, h1: -2}
    if sp.simplify(cubic["g"].subs(mp)) != 0 or sp.simplify(cubic["h"].subs(mp)) != 0:
        raise AssertionError("Myers-Perry positive control failed")
    mutated = {**mp, h1: -1}
    if sp.simplify(cubic["g"].subs(mutated)) == 0:
        raise AssertionError("wrong-sign horizon-derivative negative control was accepted")

    difference = sp.factor(cubic["g"] - 2*cubic["h"])
    expected_difference = (
        16*(lam + 2)/(lam + 1)**6
        * (12*h0*lam**2 + 26*h0*lam + 6*h0 - h1*lam - h1
           + 6*lam**4 + 30*lam**3 + 46*lam**2 + 16*lam - 8)
    )
    if sp.factor(difference - expected_difference) != 0:
        raise AssertionError("cross-equation cancellation identity changed")

    fixture = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_spin2_cubic_horizon_coefficients",
        "status": "exact_Eg_and_Eh_cubic_coefficients_derived",
        "definitions": {
            "y": "rho-1",
            "b1": "Bhat2'(1)", "b2": "Bhat2''(1)",
            "f1": "Fhat2'(1)", "f2": "Fhat2''(1)",
            "h0": "Hhat2(1)", "h1": "Hhat2'(1)",
        },
        "row_scalings": {
            "g": "F0^2*Delta_bar*E_g^(2)",
            "h": "F0^2*Delta_bar*E_h^(2)",
        },
        "common_denominators": {
            "g": sp.sstr(den_g), "h": sp.sstr(den_h),
        },
        "horizon_valuation": {
            "g": {"first_order_fixed_horizon": "zero", "second_order_fixed_horizon": "zero",
                  "first_nonzero_order": 3},
            "h": {"first_order_fixed_horizon": "zero", "second_order_fixed_horizon": "zero",
                  "first_nonzero_order": 3,
                  "cubic_Hhat2_coefficient": sp.sstr(expected_h_coefficient)},
        },
        "cubic_coefficients": {
            "g": sp.sstr(cubic["g"]), "h": sp.sstr(cubic["h"]),
        },
        "cross_equation_combination": {
            "expression": "coefficient_y3(R_g) - 2*coefficient_y3(R_h)",
            "value": sp.sstr(difference),
            "unknown_radial_derivatives_cancel": True,
            "interpretation": "horizon compatibility relation; independence must be classified using the full Noether constraint",
        },
        "controls": {
            "source_equation_fingerprints_match": True,
            "fixed_horizon_quadratic_coefficients_cancel": True,
            "myers_perry_g_and_h_cubic_coefficients_zero": True,
            "wrong_sign_horizon_derivative_rejected": True,
        },
        "source": {
            "generator": "derive_slow_rotation_second_order.py --emit-equations",
            "equation_text_sha256": {k: META[k][2] for k in META},
            "equation_srepr_sha256": {k: META[k][1] for k in META},
        },
        "claim_ceiling": (
            "Exact cubic horizon coefficients and controls for the scaled E_g and E_h rows. "
            "This does not yet establish the complete coupled Frobenius recurrence, boundary index, "
            "finite-coupling BVP, or existence/uniqueness."
        ),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(fixture, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()

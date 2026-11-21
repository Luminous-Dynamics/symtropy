#!/usr/bin/env python3
"""Verify analytic horizon extension of the full 5D EGB spin-2 E_f row.

Input: JSON emitted by derive_slow_rotation_second_order.py --emit-equations.
"""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

EQ_LEN = 8174
EQ_SREPR_SHA = "a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa"
EQ_TEXT_SHA = "31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"

def require_zero(expr: sp.Expr, label: str) -> None:
    if sp.cancel(sp.together(expr)) != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(expr)}")

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--equations", type=Path, required=True)
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    data = json.loads(args.equations.read_text(encoding="utf-8"))
    system = data["equation_system"]
    source = system["equations"]["f"]
    meta = system["equation_forms"]["f"]
    if len(source) != EQ_LEN or meta["equation_sha256"] != EQ_SREPR_SHA:
        raise AssertionError("E_f equation fingerprint changed")
    text_sha = hashlib.sha256(source.encode()).hexdigest()
    if text_sha != EQ_TEXT_SHA:
        raise AssertionError("E_f equation text hash changed")

    r, rh, alpha, rho, lam, q = sp.symbols(
        "r r_H alpha rho lambda q", positive=True, finite=True)
    fB, fF, fH = (sp.Function(n) for n in ("B2", "F2", "H2"))
    B, F, H = fB(r), fF(r), fH(r)
    E = sp.sympify(source, locals={
        "r": r, "r_H": rh, "alpha": alpha,
        "B2": fB, "F2": fF, "H2": fH,
        "sqrt": sp.sqrt, "Derivative": sp.Derivative})

    # For r>0, sqrt(1+alpha*(alpha+2*r_H^2)/r^4)=Q/r^2.
    S = sp.sqrt(1 + alpha*(alpha+2*rh**2)/r**4)
    E = E.xreplace({S: q/r**2, S**-1: r**2/q})
    if any(p.exp in (sp.Rational(1, 2), -sp.Rational(1, 2)) for p in E.atoms(sp.Pow)):
        raise AssertionError("unhandled square-root power remains")
    delta_bar = (r**2+q-alpha-2*rh**2)/rh**2
    G = delta_bar**2 * E

    jets = [F, B, sp.diff(B, r), H, sp.diff(H, r)]
    z = sp.symbols("z0:5")
    Gz = G.xreplace(dict(zip(jets, z)))
    num, den = sp.together(Gz).as_numer_denom()
    if den.has(*z):
        raise AssertionError("field jets occur in the denominator")
    poly = sp.Poly(num, *z, domain="EX")
    monomials = {(0,0,0,0,0),(1,0,0,0,0),(0,1,0,0,0),
                 (0,0,1,0,0),(0,0,0,1,0),(0,0,0,0,1)}
    if poly.total_degree() != 1 or set(poly.monoms()) != monomials:
        raise AssertionError("scaled E_f is not affine in the five expected jets")
    coeffs = {name: sp.cancel(poly.coeff_monomial(zi)/den)
              for name,zi in zip(("F2","B2","B2_prime","H2","H2_prime"),z)}
    source_term = sp.cancel(poly.coeff_monomial(1)/den)

    Qbar = sp.Symbol("Qbar", positive=True)
    dim = {rh:1, r:rho, alpha:lam, q:Qbar}
    coeffs = {k:sp.cancel(v.subs(dim)) for k,v in coeffs.items()}
    source_term = sp.cancel(source_term.subs(dim))
    expected_den = {
        "F2":2*Qbar*rho*(Qbar+rho**2),
        "B2":2*Qbar*rho*(Qbar+rho**2),
        "B2_prime":sp.Integer(2),
        "H2":2*Qbar*rho**3*(Qbar+rho**2)**2,
        "H2_prime":2*Qbar*rho**2*(Qbar+rho**2)**2}
    denoms = {}
    for k,v in coeffs.items():
        d = sp.factor(sp.denom(sp.together(v)))
        require_zero(d-expected_den[k], f"{k} denominator")
        denoms[k] = sp.sstr(d)
    source_den = sp.factor(sp.denom(sp.together(source_term)))
    require_zero(source_den-Qbar**2*rho**3*(Qbar+rho**2)**4, "source denominator")

    hsub = {rho:1, Qbar:lam+1}
    qprime = 2*rho**3/Qbar
    vals, slopes = {}, {}
    for k,v in coeffs.items():
        vals[k] = sp.factor(sp.powdenest(v.subs(hsub), force=True))
        total = sp.diff(v,rho)+sp.diff(v,Qbar)*qprime
        slopes[k] = sp.factor(sp.powdenest(total.subs(hsub), force=True))
    src0 = sp.factor(sp.powdenest(source_term.subs(hsub), force=True))
    src1 = sp.factor(sp.powdenest((sp.diff(source_term,rho)
                                  +sp.diff(source_term,Qbar)*qprime).subs(hsub), force=True))
    expected_vals = {"F2":3*(lam+2)**2,"B2":-3*(lam+2)**2,
                     "B2_prime":0,"H2":0,"H2_prime":0}
    expected_slopes = {
        "F2":3*(lam+2)**2*(lam**2+4*lam+5)/(lam+1)**2,
        "B2":-3*(lam+2)**2*(lam+3)/(lam+1),
        "B2_prime":3*(lam+2)**2,
        "H2":-2*lam*(lam+2)**2/(lam+1)**2,
        "H2_prime":(lam+2)**2/(lam+1)}
    for k in expected_vals:
        require_zero(vals[k]-expected_vals[k], f"{k} horizon value")
        require_zero(slopes[k]-expected_slopes[k], f"{k} horizon slope")
    require_zero(src0, "source horizon value")
    require_zero(src1-2*(lam+2)**4/(lam+1)**2, "source horizon slope")

    fp, bp, hh, hp = sp.symbols("F2pH B2pH H2H H2pH")
    C0 = 3*(lam+2)**2
    compatibility = (vals["F2"]*fp + vals["B2"]*bp
        + slopes["B2_prime"]*bp + slopes["H2"]*hh
        + slopes["H2_prime"]*hp + src1)/C0
    expected_compat = (fp-2*lam*hh/(3*(lam+1)**2)
                      +hp/(3*(lam+1))+2*(lam+2)**2/(3*(lam+1)**2))
    require_zero(compatibility-expected_compat, "first horizon compatibility")

    fixture = {
      "schema_version":1,
      "artifact_kind":"5d_egb_spin2_full_scaled_Ef_horizon_row",
      "status":"full_scaled_algebraic_row_analytic_extension_symbolically_verified",
      "source":{"equation_generator":"derive_slow_rotation_second_order.py --emit-equations",
        "equation_key":"equation_system.equations.f","equation_chars":len(source),
        "equation_srepr_sha256_from_generator":EQ_SREPR_SHA,"equation_text_sha256":text_sha,
        "dimensionless_variables":"rho=r/r_H, lambda=alpha/r_H^2",
        "dimensionless_fields":"Bhat2=r_H^2*B2, Fhat2=r_H^2*F2, Hhat2=H2"},
      "row_scaling":{"original_equation":"E_f^(2)=0",
        "scaled_equation":"G_f^(2)=Delta_bar^2*E_f^(2)=0",
        "Qbar":"sqrt(lambda^2+2*lambda+rho^4)",
        "Delta_bar":"rho^2+Qbar-lambda-2",
        "positive_radical_rewrite":"sqrt(1+lambda*(lambda+2)/rho^4)=Qbar/rho^2",
        "reduced_unknown_jets":["Fhat2","Bhat2","dBhat2/drho","Hhat2","dHhat2/drho"],
        "no_F2_derivative":True,"no_second_derivatives":True,"full_affine_decomposition_exact":True},
      "analyticity_certificate":{"coefficient_denominators":denoms,
        "source_denominator":sp.sstr(source_den),"Delta_bar_occurs_in_any_denominator":False,
        "positivity_domain":"rho>=1, lambda>=0: Qbar>0, rho>0, Qbar+rho^2>0",
        "conclusion":"All coefficients and the source of the complete scaled E_f row are rational in (rho,lambda,Qbar), with denominators strictly positive on the closed exterior. Qbar is positive real-analytic there, so the complete scaled row extends real-analytically to the horizon for analytic fields."},
      "horizon_jet":{"coordinate":"y=rho-1",
        "coefficient_values":{k:sp.sstr(v) for k,v in vals.items()},
        "coefficient_slopes":{k:sp.sstr(v) for k,v in slopes.items()},
        "source_value":sp.sstr(src0),"source_slope":sp.sstr(src1)},
      "first_nontrivial_horizon_compatibility":{"assumptions":["Bhat2(1)=0","Fhat2(1)=0","analytic Bhat2,Fhat2,Hhat2"],
        "unreduced":"3*(lambda+2)^2*Fhat2'(1)-2*lambda*(lambda+2)^2/(lambda+1)^2*Hhat2(1)+(lambda+2)^2/(lambda+1)*Hhat2'(1)+2*(lambda+2)^4/(lambda+1)^2=0",
        "reduced":sp.sstr(expected_compat),"Bhat2_prime_cancels":True,
        "interpretation":"One horizon compatibility condition from E_f only; E_b/E_h recurrence and independent E_g constraint remain open."},
      "validation":{"python":platform.python_version(),"sympy":sp.__version__,
        "generator_equation_fingerprint_matches_pinned_source":True,
        "equation_text_sha256":text_sha,"full_scaled_row_decomposition_exact":True,
        "all_denominators_exact_and_positive":True,"all_horizon_values_exact":True,
        "all_horizon_slopes_exact":True,"first_order_horizon_compatibility_exact":True},
      "claim_ceiling":"The complete scaled algebraic E_f^(2) row is analytically regular at the horizon and its first nontrivial horizon compatibility condition is derived. This is not the coupled Frobenius recurrence, a finite-alpha BVP solution, an interval enclosure, or an existence/uniqueness theorem."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)

if __name__ == "__main__":
    main()

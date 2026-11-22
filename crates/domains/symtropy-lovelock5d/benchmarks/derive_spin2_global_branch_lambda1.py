#!/usr/bin/env python3
"""Derive the first dimensionless-coupling derivative of the 5D EGB O(a^2) MP branch.

The Einstein and Gauss-Bonnet reduced actions are varied with b,f,g,h,w
independent. Gauge g=r^2 is imposed only after all Euler-Lagrange equations
are formed. The exact static background and O(a) rotational profile are
expanded at lambda=alpha/r_H^2=0. The O(lambda*a^2) equations are then solved
symbolically by an exact rational ansatz and checked in all four original
even-sector rows.

Requires Python 3.12+ and SymPy 1.14+.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import platform
import sympy as sp


def zero(label: str, expression: sp.Expr | sp.MatrixBase) -> None:
    vals = list(expression) if isinstance(expression, sp.MatrixBase) else [expression]
    vals = [sp.factor(sp.cancel(v)) for v in vals]
    if any(v != 0 for v in vals):
        raise AssertionError(f"{label}: {vals}")


def derive() -> dict[str, object]:
    r, eps, lam = sp.symbols("r epsilon lambda", positive=True)
    b, f, g, h, w = sp.symbols("b f g h w", positive=True)
    bp, fp, gp, hp, wp = sp.symbols("bp fp gp hp wp", real=True)
    bpp, fpp, gpp, hpp, wpp = sp.symbols("bpp fpp gpp hpp wpp", real=True)
    q0, q1, q2 = (b, f, g, h, w), (bp, fp, gp, hp, wp), (bpp, fpp, gpp, hpp, wpp)

    measure = sp.sqrt(f*h/b)
    L_e = measure * (
        bp*gp + g*bp*hp/(2*h) + b*gp**2/(2*g) + b*gp*hp/h
        + g*h*wp**2/2 + 2*b/f*(4-h/g)
    )
    L_gb = measure/g * (
        4*h*bp*gp/g
        + 2*(4*g-3*h)*(bp*hp/h+h*wp**2)
        - f*bp*hp*gp**2/(2*h)
        - f*h*gp**2*wp**2/2
    )
    L = L_e + lam*L_gb/4

    def total_derivative(expr: sp.Expr) -> sp.Expr:
        return (
            sum(sp.diff(expr, q)*q1p for q, q1p in zip(q0, q1))
            + sum(sp.diff(expr, q1p)*q2p for q1p, q2p in zip(q1, q2))
        )

    momenta = {
        name: sp.diff(L, q1p)
        for name, q1p in zip(("b","f","g","h","w"), q1)
    }
    equations = {
        name: sp.diff(L, q)-total_derivative(momenta[name])
        for name, q in zip(("b","f","g","h","w"), q0)
    }

    # Verify the exact background functions before expanding in lambda.
    Cbg = 2 + lam
    radical = sp.sqrt(1 + lam*Cbg/r**4)
    F_exact = 1 - Cbg/(r**2*(1+radical))
    W_exact = Cbg/(r**4*(1+radical))
    F0 = 1-r**-2
    F1 = -sp.Rational(1,2)*(r**-2-r**-6)
    W0 = r**-4
    W1 = sp.Rational(1,2)*(r**-4-r**-8)
    zero("static background first coupling coefficient", sp.diff(F_exact,lam).subs(lam,0)-F1)
    zero("linear-spin background first coupling coefficient", sp.diff(W_exact,lam).subs(lam,0)-W1)

    B0, F20, H0 = r**-6-r**-2, r**-4-r**-2, r**-2
    B1f, F21f, H1f = (sp.Function(name)(r) for name in ("B1","F21","H1"))
    Btotal, Ftotal, Htotal = B0+lam*B1f, F20+lam*F21f, H0+lam*H1f
    Fbg, Wbg = F0+lam*F1, W0+lam*W1

    # The radial gauge is imposed only in this post-variation ansatz.
    ansatz = {
        b:Fbg+eps**2*Btotal, f:Fbg+eps**2*Ftotal, g:r**2,
        h:r**2+eps**2*Htotal, w:eps*Wbg,
        bp:sp.diff(Fbg+eps**2*Btotal,r),
        fp:sp.diff(Fbg+eps**2*Ftotal,r),
        gp:2*r, hp:sp.diff(r**2+eps**2*Htotal,r),
        wp:eps*sp.diff(Wbg,r),
        bpp:sp.diff(Fbg+eps**2*Btotal,r,2),
        fpp:sp.diff(Fbg+eps**2*Ftotal,r,2),
        gpp:sp.Integer(2),
        hpp:sp.diff(r**2+eps**2*Htotal,r,2),
        wpp:eps*sp.diff(Wbg,r,2),
    }

    # Extract coefficient of eps^2*lambda from original Euler-Lagrange rows.
    rows = {}
    for name in ("b","f","g","h"):
        substituted = equations[name].subs(ansatz)
        even = sp.diff(substituted,eps,2).subs(eps,0)/2
        rows[name] = sp.factor(sp.diff(even,lam).subs(lam,0))

    B1sol = sp.factor((r**2-1)*(11*r**6+11*r**4+22*r**2+9)/(9*r**10))
    H1sol = sp.factor(-(11*r**4+14*r**2+3)/(6*r**6))
    F21sol = sp.factor((r**2-1)*(22*r**4-11*r**2-35)/(18*r**8))
    substitutions = {}
    for function, solution in ((B1f,B1sol),(F21f,F21sol),(H1f,H1sol)):
        substitutions[function] = solution
        substitutions[sp.diff(function,r)] = sp.diff(solution,r)
        substitutions[sp.diff(function,r,2)] = sp.diff(solution,r,2)
    residuals = {
        name: sp.factor(sp.cancel(expr.subs(substitutions)))
        for name, expr in rows.items()
    }
    for name, residual in residuals.items():
        zero(f"original order-lambda E_{name}",residual)

    bprime1 = sp.factor(sp.diff(B1sol,r).subs(r,1))
    fprime1 = sp.factor(sp.diff(F21sol,r).subs(r,1))
    hzero1 = sp.factor(H1sol.subs(r,1))
    hprime1 = sp.factor(sp.diff(H1sol,r).subs(r,1))
    asymptotic = {
        "r2_B1":sp.limit(r**2*B1sol,r,sp.oo),
        "r2_F21":sp.limit(r**2*F21sol,r,sp.oo),
        "r2_H1":sp.limit(r**2*H1sol,r,sp.oo),
    }
    zero("B1 horizon normalization",B1sol.subs(r,1))
    zero("F21 horizon normalization",F21sol.subs(r,1))
    zero("H1 horizon derivative relation",hprime1-16)
    zero("B1 prime",bprime1-sp.Rational(106,9))
    zero("F21 prime",fprime1+sp.Rational(8,3))
    zero("H1 horizon value",hzero1+sp.Rational(14,3))
    zero("H1 asymptotic coefficient",asymptotic["r2_H1"]+sp.Rational(11,6))
    zero("B1 asymptotic coefficient",asymptotic["r2_B1"]-sp.Rational(11,9))
    zero("F21 asymptotic coefficient",asymptotic["r2_F21"]-sp.Rational(11,9))

    rendered_rows = {name:sp.sstr(expr) for name,expr in rows.items()}
    result = {
        "schema_version":1,
        "artifact_kind":"5d_egb_spin2_asymptotically_flat_branch_coupling_derivative",
        "status":"exact_first_lambda_derivative_of_asymptotically_flat_Oa2_branch",
        "conventions":{
            "dimension":5,"horizon_radius":"1","lambda":"alpha/r_H^2",
            "spin_order":"O(a^2)","coupling_order":"O(lambda)",
            "gauge_order":"vary b,f,g,h,w independently; then impose g=r^2",
            "background_branch":"Einstein branch Boulware-Deser",
            "zeroth_order_solution":"equal-spin Myers-Perry"
        },
        "background_expansion":{
            "F_background":"1-r^(-2) + lambda*(-1/2)*(r^(-2)-r^(-6)) + O(lambda^2)",
            "W1":"r^(-4) + lambda*(1/2)*(r^(-4)-r^(-8)) + O(lambda^2)"
        },
        "first_coupling_correction":{
            "B21":sp.sstr(B1sol),
            "F21":sp.sstr(F21sol),
            "H21":sp.sstr(H1sol),
            "horizon_data":{
                "B21(1)":"0","B21_prime(1)":sp.sstr(bprime1),
                "F21(1)":"0","F21_prime(1)":sp.sstr(fprime1),
                "H21(1)":sp.sstr(hzero1),"H21_prime(1)":sp.sstr(hprime1)
            },
            "asymptotic_decay_coefficients":{k:sp.sstr(v) for k,v in asymptotic.items()},
            "asymptotically_flat_horizon_data":{
                "b1(lambda)":"-4 + (106/9)*lambda + O(lambda^2)",
                "h0(lambda)":"1 - (14/3)*lambda + O(lambda^2)"
            }
        },
        "equation_rows_lambda1":rendered_rows,
        "equation_residuals":{name:sp.sstr(value) for name,value in residuals.items()},
        "checks":{
            "background_F_first_derivative_exact":True,
            "background_W_first_derivative_exact":True,
            "all_four_original_even_rows_exact_zero":True,
            "horizon_normalizations_exact":True,
            "horizon_data_shifts_exact":True,
            "asymptotic_decay_coefficients_exact":True,
            "zeroth_order_MP_recovered_by_construction":True
        },
        "derivation":{
            "python":platform.python_version(),
            "sympy":sp.__version__,
            "source_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "arithmetic":"exact symbolic algebra over rational functions"
        },
        "claim_ceiling":(
            "This is the exact first derivative at lambda=0 of one asymptotically flat, horizon-regular "
            "five-dimensional O(a^2) EGB branch. All four order-lambda original Euler-Lagrange rows vanish. "
            "It does not prove an analytic finite-lambda branch, a finite-lambda interval existence theorem, "
            "the derivative of the full two-by-two boundary-map Jacobian, full nonlinear uniqueness, or "
            "PDE hyperbolicity."
        )
    }
    return result


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path,default=Path("spin2_global_branch_lambda1_v1.json"))
    args=parser.parse_args()
    result=derive()
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(rendered,encoding="utf-8")
    print(json.dumps({
        "status":result["status"],"output":str(args.output),
        "sha256":hashlib.sha256(rendered.encode()).hexdigest(),
        "source_sha256":result["derivation"]["source_sha256"],
        "checks":result["checks"]
    },indent=2))


if __name__=="__main__":
    main()

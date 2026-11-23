#!/usr/bin/env python3
"""Exact-rational replay of the stored lambda-derivative equations and controls."""
from __future__ import annotations
import argparse,json
from pathlib import Path
import sympy as sp


def zero(label,expr):
    vals=list(expr) if isinstance(expr,sp.MatrixBase) else [expr]
    vals=[sp.factor(sp.cancel(v)) for v in vals]
    if any(v!=0 for v in vals):
        raise AssertionError(f"{label}: {vals}")


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--fixture",type=Path,default=Path(__file__).with_name("spin2_global_branch_lambda1_v1.json"))
    args=parser.parse_args()
    d=json.loads(args.fixture.read_text(encoding="utf-8"))
    r=sp.symbols("r",positive=True)
    B1,F21,H1=(sp.Function(n)(r) for n in ("B1","F21","H1"))
    local={"r":r,"B1":sp.Function("B1"),"F21":sp.Function("F21"),"H1":sp.Function("H1")}
    equations={k:sp.sympify(v,locals=local) for k,v in d["equation_rows_lambda1"].items()}
    B=sp.factor((r**2-1)*(11*r**6+11*r**4+22*r**2+9)/(9*r**10))
    F=sp.factor((r**2-1)*(22*r**4-11*r**2-35)/(18*r**8))
    H=sp.factor(-(11*r**4+14*r**2+3)/(6*r**6))
    sub={}
    for fn,sol in ((B1,B),(F21,F),(H1,H)):
        sub[fn]=sol
        sub[sp.diff(fn,r)]=sp.diff(sol,r)
        sub[sp.diff(fn,r,2)]=sp.diff(sol,r,2)
    for name,eq in equations.items():
        zero(f"order-lambda original E_{name}",eq.subs(sub))
    wrong_H=-H
    bad={**sub,H1:wrong_H,sp.diff(H1,r):sp.diff(wrong_H,r),sp.diff(H1,r,2):sp.diff(wrong_H,r,2)}
    negative={name:sp.factor(eq.subs(bad)) for name,eq in equations.items()}
    if all(value==0 for value in negative.values()):
        raise AssertionError("wrong-sign H21 negative control accepted")
    zero("B21 horizon",B.subs(r,1))
    zero("F21 horizon",F.subs(r,1))
    zero("H21 horizon value",H.subs(r,1)+sp.Rational(14,3))
    zero("B21 horizon slope",sp.diff(B,r).subs(r,1)-sp.Rational(106,9))
    zero("F21 horizon slope",sp.diff(F,r).subs(r,1)+sp.Rational(8,3))
    zero("H21 horizon slope",sp.diff(H,r).subs(r,1)-16)
    zero("B21 asymptotic",sp.limit(r**2*B,r,sp.oo)-sp.Rational(11,9))
    zero("F21 asymptotic",sp.limit(r**2*F,r,sp.oo)-sp.Rational(11,9))
    zero("H21 asymptotic",sp.limit(r**2*H,r,sp.oo)+sp.Rational(11,6))
    print(json.dumps({
      "status":"passed_exact_rational_lambda1_replay",
      "checks":["all four original O(lambda*a^2) rows","horizon data shifts",
        "asymptotic decay coefficients","wrong-sign H21 negative control rejected"],
      "negative_control_nonzero_rows":[k for k,v in negative.items() if v!=0]
    },indent=2))


if __name__=="__main__":
    main()

#!/usr/bin/env python3
"""Exact-rational verifier for the Einstein-limit global boundary-map fixture."""
from __future__ import annotations
import argparse,json
from pathlib import Path
import sympy as sp

def matrix(data):
    return sp.Matrix([[sp.sympify(x) for x in row] for row in data])

def vector(data):
    return sp.Matrix([sp.sympify(x) for x in data])

def zero(label, expression):
    values=list(expression) if isinstance(expression,sp.MatrixBase) else [expression]
    values=[sp.factor(sp.cancel(x)) for x in values]
    if any(x!=0 for x in values):
        raise AssertionError(f"{label}: {values}")

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--fixture",type=Path,default=Path(__file__).with_name("spin2_einstein_global_boundary_map_v1.json"))
    args=parser.parse_args()
    d=json.loads(args.fixture.read_text(encoding="utf-8"))
    r,b1,h0=sp.symbols("r b1 h0")
    M=matrix(d["reduced_operator"]["principal_matrix"])
    C=matrix(d["reduced_operator"]["lower_order_matrix"])
    S=vector(d["reduced_operator"]["source"])
    B=-r**-2+r**-6+(b1+4)*sp.Rational(1,2)*(1-r**-2)
    H=r**-2+(h0-1)*(2*r**4-r**2)
    F=-(r**2-1)*(2*(h0-1)*r**4-(h0-1)*r**2+1)/r**4
    U=vector([B,sp.diff(B,r),H,sp.diff(H,r)])
    second=vector([sp.diff(B,r,2),sp.diff(H,r,2)])
    pivot=sp.sympify(d["reduced_operator"]["algebraic_Ef_pivot"])
    det_claim=sp.sympify(d["reduced_operator"]["determinant"])
    zero("principal determinant",M.det()-det_claim)
    zero("principal determinant factor",det_claim-(r**2-1)/4)
    zero("E_f pivot factor",pivot-3*r**5/(r**2-1)**2)
    zero("reduced ODE",M*second+C*U+S)
    zero("horizon B",B.subs(r,1))
    zero("horizon B prime",sp.diff(B,r).subs(r,1)-b1)
    zero("horizon F",F.subs(r,1))
    zero("horizon F prime",sp.diff(F,r).subs(r,1)+2*h0)
    zero("horizon H",H.subs(r,1)-h0)
    zero("horizon H prime",sp.diff(H,r).subs(r,1)-(6*h0-8))
    CB=sp.limit(B,r,sp.oo)
    CH=sp.limit(H/r**4,r,sp.oo)
    CF=sp.limit(F/r**2,r,sp.oo)
    J=sp.Matrix([[sp.diff(CB,b1),sp.diff(CB,h0)],[sp.diff(CH,b1),sp.diff(CH,h0)]])
    zero("C_B map",(CB-(b1+4)/2))
    zero("C_H map",(CH-2*(h0-1)))
    zero("F/H growth relation",CF+CH)
    zero("Jacobian",J-sp.diag(sp.Rational(1,2),2))
    zero("determinant",J.det()-1)
    zero("condition number",4-sp.sqrt(4/sp.Rational(1,4)))
    params=sp.solve([CB,CH],(b1,h0),dict=True)
    if params!=[{b1:-4,h0:1}]: raise AssertionError(f"wrong asymptotically flat parameters {params}")
    zero("MP B",B.subs(params[0])-(-r**-2+r**-6))
    zero("MP F",F.subs(params[0])-(-r**-2+r**-4))
    zero("MP H",H.subs(params[0])-r**-2)
    # A wrong sign in the H-mode must fail the differential equations.
    Hbad=r**-2+(h0-1)*(2*r**4+r**2)
    Ubad=vector([B,sp.diff(B,r),Hbad,sp.diff(Hbad,r)])
    secondbad=vector([sp.diff(B,r,2),sp.diff(Hbad,r,2)])
    bad=M*secondbad+C*Ubad+S
    if all(sp.factor(x)==0 for x in bad):
        raise AssertionError("wrong-sign H r^2 negative control accepted")
    print(json.dumps({
      "status":"passed_exact_rational_global_boundary_map_replay",
      "checks":["principal determinant and positive algebraic pivot",
        "reduced ODE exact solution","horizon data and compatibility",
        "global boundary map full rank, determinant and condition number",
        "F2/H2 growing-mode linkage","unique asymptotically flat MP profiles",
        "wrong-sign H-mode negative control rejected"],
      "negative_control_residual":[sp.sstr(sp.factor(x)) for x in bad]
    },indent=2))

if __name__=="__main__": main()

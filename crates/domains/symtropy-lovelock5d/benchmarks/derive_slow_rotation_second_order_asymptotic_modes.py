#!/usr/bin/env python3
"""Derive alpha=0 indicial modes from the O(a^2) action-generated operator.

Generate the input first with derive_slow_rotation_second_order.py --emit-equations.
This is a research-only asymptotic calculation, not a BVP solver.
"""
import argparse, hashlib, itertools, json
from functools import reduce
from pathlib import Path
import sympy as sp


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--equations", type=Path, required=True)
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    raw = json.loads(args.equations.read_text())
    ops = raw["equation_system"]["linear_operators"]

    r, rh, alpha, t = sp.symbols("r r_H alpha t", positive=True)
    p = sp.symbols("p", real=True)
    B, F, H = (sp.Function(n)(r) for n in ("B2", "F2", "H2"))
    b, f, h = sp.symbols("b f h")
    funcs = {"r": r, "r_H": rh, "alpha": alpha,
             "B2": sp.Function("B2"), "F2": sp.Function("F2"),
             "H2": sp.Function("H2"), "sqrt": sp.sqrt,
             "Derivative": sp.Derivative}
    ans = {
        B: b*r**p, F: f*r**p, H: h*r**(p+2),
        sp.diff(B,r): b*p*r**(p-1), sp.diff(F,r): f*p*r**(p-1),
        sp.diff(H,r): h*(p+2)*r**(p+1),
        sp.diff(B,r,2): b*p*(p-1)*r**(p-2),
        sp.diff(F,r,2): f*p*(p-1)*r**(p-2),
        sp.diff(H,r,2): h*(p+2)*(p+1)*r**p,
    }
    rows=[]
    for name in ("b", "f", "g", "h"):
        e=sp.sympify(ops[name], locals=funcs).subs(alpha,0).subs(ans)
        scale=r**(-p-1) if name in ("b","f") else r**(-p+1)
        e=sp.cancel((e*scale).subs(r,1/t))
        rows.append([sp.factor(sp.limit(sp.cancel(sp.diff(e,a)),t,0,dir="+"))
                     for a in (b,f,h)])
    M=sp.Matrix(rows)
    minors=[sp.factor(M[list(ix),:].det()) for ix in itertools.combinations(range(4),3)]
    common=sp.factor(reduce(sp.gcd,minors))
    roots=sorted(sp.solve(common,p))
    modes={str(int(root)):[int(v) for v in M.subs(p,root).nullspace()[0]] for root in roots}
    assert roots == [-4,-2,0,2], roots
    assert modes == {"-4":[0,1,1], "-2":[1,1,0], "0":[1,0,0], "2":[0,-1,1]}, modes

    result={
      "schema_version":1,
      "artifact_kind":"5d_egb_spin2_alpha_zero_asymptotic_indicial_modes",
      "status":"symbolically_derived_from_action_generated_homogeneous_operator",
      "input":{"sha256":hashlib.sha256(args.equations.read_bytes()).hexdigest(),
               "source_derivation_script_git_blob_sha1":"c5b30eaf3ee972a0a316d4193f13e2710eb4c677"},
      "conventions":{"background":"alpha=0 Einstein/Myers-Perry limit",
        "operator":"linear_operators only; W1^2 source excluded",
        "ansatz":"B2=b*r^p, F2=f*r^p, H2=h*r^(p+2)",
        "equations":["E_b","E_f","E_g","E_h"],
        "rescalings":{"E_b":"r^(-p-1)","E_f":"r^(-p-1)",
                       "E_g":"r^(-p+1)","E_h":"r^(-p+1)"}},
      "leading_matrix":{"columns":["b","f","h"],
        "rows":[[sp.sstr(v) for v in row] for row in rows]},
      "maximal_3x3_minors":[sp.sstr(v) for v in minors],
      "common_factor":sp.sstr(common),
      "indicial_roots":[int(v) for v in roots],
      "mode_directions":modes,
      "interpretation":{"-4":"decaying: F2~r^-4, H2~r^-2; B2 has no leading r^-4 term",
        "-2":"mass-like: B2 and F2 share r^-2; leading H2~r^0 coefficient is zero",
        "0":"constant lapse mode; removed by b(infinity)=1",
        "2":"growing F2/H2 mode; excluded by asymptotic flatness"},
      "validation":{"all_four_equations_included":True,
        "radial_constraint_Eg_included":True,
        "all_maximal_minors_and_nullspaces_checked":True,
        "sympy_version":sp.__version__},
      "claim_ceiling":"Exact alpha=0 spatial-infinity indicial analysis only; not a finite-alpha mode proof, complete Frobenius expansion, horizon analysis, BVP solution, or existence theorem."
    }
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    if args.output: args.output.write_text(rendered)
    else: print(rendered)

if __name__ == "__main__": main()

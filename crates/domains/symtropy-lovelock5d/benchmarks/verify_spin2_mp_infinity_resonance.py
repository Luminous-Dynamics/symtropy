#!/usr/bin/env python3
"""Exact-rational verifier for the Einstein-limit infinity n=2 resonance."""
from __future__ import annotations
import argparse,json
from pathlib import Path
import sympy as sp

def mat(rows):
    return sp.Matrix([[sp.sympify(x) for x in row] for row in rows])

def vec(values):
    return sp.Matrix([sp.sympify(x) for x in values])

def require_zero(label,value):
    values=list(value) if isinstance(value,sp.MatrixBase) else [value]
    if any(sp.factor(x)!=0 for x in values):
        raise AssertionError(f"{label}: {value}")

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--fixture",type=Path,default=Path(__file__).with_name("spin2_mp_infinity_resonance_v1.json"))
    args=parser.parse_args()
    d=json.loads(args.fixture.read_text(encoding="utf-8"))
    A0,A1,A2=(mat(x) for x in d["A_coefficients"])
    g0,g1,g2=(vec(x) for x in d["g_coefficients"])
    I=sp.eye(4)
    eB=vec(["1","0","0","0"])
    v0=sp.symbols("c")*eB
    require_zero("n=0 constant-mode compatibility",A0*v0+g0)
    N1=-I-A0
    require_zero("n=1 determinant",N1.det()-(-5))
    require_zero("n=1 RHS",(A1*v0+g1))
    require_zero("n=1 solution",N1.inv()*(A1*v0+g1))
    N2=-2*I-A0
    require_zero("n=2 rank",N2.rank()-2)
    require_zero("n=2 nullity",N2.nullspace().__len__()-2)
    require_zero("n=2 compatibility",A2*eB+g2)
    decays=[vec(["1","-2","0","0"]),vec(["0","0","1","-2"])]
    for i,v in enumerate(decays):
        require_zero(f"decay eigenmode {i}",A0*v+2*v)
        require_zero(f"n=2 free coefficient {i}",N2*v)
    const=vec(["1","0","0","0"])
    grow=vec(["0","0","1","4"])
    require_zero("constant mode",A0*const)
    require_zero("growing mode",A0*grow-4*grow)
    P=sp.Matrix.hstack(decays[0],decays[1],const,grow)
    require_zero("mode basis determinant",P.det()-(-12))
    print(json.dumps({
      "status":"passed_exact_rational_infinity_resonance_replay",
      "n1_matrix_determinant":"-5",
      "n2_matrix_rank":2,
      "n2_matrix_nullity":2,
      "bounded_n2_resonance_compatible":True,
      "two_independent_r_minus_2_modes":True
    },indent=2))

if __name__=="__main__":
    main()

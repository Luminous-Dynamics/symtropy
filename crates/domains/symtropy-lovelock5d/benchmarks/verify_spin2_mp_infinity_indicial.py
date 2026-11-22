#!/usr/bin/env python3
"""Exact-rational verifier for the Einstein-limit infinity indicial anchor."""
from __future__ import annotations
import argparse,json
from pathlib import Path
import sympy as sp

def require_zero(label,value):
    values=list(value) if isinstance(value,sp.MatrixBase) else [value]
    if any(sp.factor(x)!=0 for x in values):
        raise AssertionError(f"{label}: {value}")

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--fixture",type=Path,default=Path(__file__).with_name("spin2_mp_infinity_indicial_v1.json"))
    args=parser.parse_args()
    d=json.loads(args.fixture.read_text(encoding="utf-8"))
    A=sp.Matrix([[sp.sympify(x) for x in row] for row in d["log_radius_matrix"]])
    g=sp.Matrix([sp.sympify(x) for x in d["log_radius_source"]])
    sigma=sp.symbols("sigma")
    require_zero("leading source",g)
    require_zero("characteristic polynomial",(sigma*sp.eye(4)-A).det()-sigma*(sigma-4)*(sigma+2)**2)
    vectors=[sp.Matrix(v) for m in d["modes"] for v in m["eigenvectors"]]
    values=[sp.Integer(m["sigma"]) for m in d["modes"] for _ in m["eigenvectors"]]
    for i,(v,eig) in enumerate(zip(vectors,values)):
        require_zero(f"eigenvector {i}",A*v-eig*v)
    P=sp.Matrix.hstack(*vectors)
    require_zero("eigenvector determinant",P.det()-(-12))
    D=P.inv()*A*P
    require_zero("diagonalization",D-sp.diag(-2,-2,0,4))
    print(json.dumps({"status":"passed_exact_rational_infinity_replay","exponents":{"-2":2,"0":1,"4":1},"diagonalizable":True},indent=2))

if __name__=="__main__":
    main()

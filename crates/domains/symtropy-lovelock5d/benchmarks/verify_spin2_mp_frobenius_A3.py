#!/usr/bin/env python3
"""Exact-rational recurrence verifier for spin2_mp_frobenius_A3_v1.json."""
from __future__ import annotations
import argparse, json
from pathlib import Path
import sympy as sp

def mat(rows):
    return sp.Matrix([[sp.sympify(x) for x in row] for row in rows])

def vec(values):
    return sp.Matrix([sp.sympify(x) for x in values])

def require_zero(label, value):
    values = list(value) if isinstance(value, sp.MatrixBase) else [value]
    if any(sp.factor(x) != 0 for x in values):
        raise AssertionError(f"{label}: {value}")

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--fixture",type=Path,default=Path(__file__).with_name("spin2_mp_frobenius_A3_v1.json"))
    args=parser.parse_args()
    d=json.loads(args.fixture.read_text(encoding="utf-8"))
    A0,A1,A2,A3=(mat(d[k]) for k in ("A0","A1","A2","A3"))
    g0,g1,g2,g3=(vec(d[k]) for k in ("g0","g1","g2","g3"))
    b1,h0=sp.symbols("b1 h0")
    U0=vec(["0","b1","h0","6*h0-8"])
    U1=vec(["b1","24-3*b1","6*h0-8","22*h0-16"])
    U2=vec(["12-3*b1/2","6*b1-132","11*h0-8","24*h0-36"])
    U3=vec(["2*b1-44","444-10*b1","8*h0-12","8*h0+12"])
    require_zero("analytic n=0",A0*U0+g0)
    require_zero("analytic n=1",U1-A0*U1-A1*U0-g1)
    require_zero("analytic n=2",2*U2-A0*U2-A1*U1-A2*U0-g2)
    require_zero("analytic n=3",3*U3-A0*U3-A1*U2-A2*U1-A3*U0-g3)
    u=vec(d["singular_branch"]["eigenvector"])
    clog=vec(d["singular_branch"]["forced_log_coefficient"])
    P0=sp.eye(4)+A0
    require_zero("singular eigenvector",A0*u+u)
    require_zero("log obstruction",P0*A1*u-clog)
    require_zero("log coefficient regular subspace",A0*clog)
    require_zero("projector idempotence",P0**2-P0)
    require_zero("projector rank",P0.rank()-3)
    mp=d["myers_perry"]
    M0,M1,M2,M3=(vec(mp[k]) for k in ("U0","U1","U2","U3"))
    require_zero("MP n=0",A0*M0+g0)
    require_zero("MP n=1",M1-A0*M1-A1*M0-g1)
    require_zero("MP n=2",2*M2-A0*M2-A1*M1-A2*M0-g2)
    require_zero("MP n=3",3*M3-A0*M3-A1*M2-A2*M1-A3*M0-g3)
    print(json.dumps({"status":"passed_exact_rational_recurrence_replay","checks":["analytic recurrences n=0..3","singular logarithmic obstruction","regular projector","Myers-Perry recurrences n=0..3"]},indent=2))

if __name__=="__main__":
    main()

#!/usr/bin/env python3
"""Exact recurrence verifier for spin2_mp_frobenius_A2_v1.json.

Checks the horizon coefficient recurrences and the Myers–Perry control.
Uses SymPy exact rational arithmetic; it does not regenerate the action-level
equations or independently derive A0/A1/A2.
"""
from __future__ import annotations
import argparse, json
from pathlib import Path
import sympy as sp

def matrix(rows):
    return sp.Matrix([[sp.sympify(x) for x in row] for row in rows])

def vector(values):
    return sp.Matrix([sp.sympify(x) for x in values])

def require_zero(label, value):
    values = list(value) if isinstance(value, sp.MatrixBase) else [value]
    if any(sp.factor(x) != 0 for x in values):
        raise AssertionError(f"{label}: {value}")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", type=Path, default=Path(__file__).with_name("spin2_mp_frobenius_A2_v1.json"))
    args = parser.parse_args()
    data = json.loads(args.fixture.read_text(encoding="utf-8"))
    A0, A1, A2 = (matrix(data[k]) for k in ("A0", "A1", "A2"))
    g0, g1, g2 = (vector(data[k]) for k in ("g0", "g1", "g2"))
    b1, h0 = sp.symbols("b1 h0")
    U0 = vector(["0", "b1", "h0", "6*h0-8"])
    U1 = vector(["b1", "24-3*b1", "6*h0-8", "22*h0-16"])
    U2 = vector(["12-3*b1/2", "6*b1-132", "11*h0-8", "24*h0-36"])
    require_zero("n=0 analytic recurrence", A0*U0 + g0)
    require_zero("n=1 analytic recurrence", U1 - A0*U1 - A1*U0 - g1)
    require_zero("n=2 analytic recurrence", 2*U2 - A0*U2 - A1*U1 - A2*U0 - g2)
    u = vector(data["singular_branch"]["eigenvector"])
    c_log = vector(data["singular_branch"]["forced_log_coefficient"])
    P0 = sp.eye(4) + A0
    require_zero("singular exponent", A0*u + u)
    require_zero("log obstruction", P0*A1*u - c_log)
    require_zero("log coefficient is regular-sector", A0*c_log)
    require_zero("projector idempotence", P0*P0 - P0)
    require_zero("projector rank", sp.Matrix(P0).rank() - 3)
    mp = data["myers_perry"]
    U0m, U1m, U2m = (vector(mp[k]) for k in ("U0","U1","U2"))
    require_zero("MP n=0 recurrence", A0*U0m + g0)
    require_zero("MP n=1 recurrence", U1m - A0*U1m - A1*U0m - g1)
    require_zero("MP n=2 recurrence", 2*U2m - A0*U2m - A1*U1m - A2*U0m - g2)
    print(json.dumps({
        "status":"passed_exact_rational_recurrence_replay",
        "checks": [
          "analytic n=0,1,2",
          "singular exponent and logarithmic obstruction",
          "regular projector",
          "Myers-Perry n=0,1,2"
        ],
        "fixture":str(args.fixture)
    }, indent=2))

if __name__ == "__main__":
    main()

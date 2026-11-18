#!/usr/bin/env python3
"""Reproduce the exact Chebyshev expansion and tail of the MK inverse-radius term.

Research-only reference script. It does not solve the Bach equation and does
not prove a BVP existence result. The analytic geometric-series formula is the
tail proof; sampled point evaluations are regression checks only.

Requires SymPy and mpmath. Run:
    python3 mk_chebyshev_tail_reference.py
"""

from __future__ import annotations

import json
import sympy as sp
import mpmath as mp

mp.mp.dps = 80

a = sp.Rational(3)
b = sp.Rational(6)
c = (a + b) / 2
d = (b - a) / 2
kappa = sp.simplify(c / d)
q = sp.simplify(kappa - sp.sqrt(kappa**2 - 1))

c_inverse = -sp.Rational(17, 10)
c_constant = sp.Rational(7, 10)
c_linear = sp.Rational(1, 10)
c_quadratic = -sp.Rational(1, 100)
nu = sp.Integer(2)

assert kappa == 3
assert sp.simplify(q - (3 - 2 * sp.sqrt(2))) == 0
assert sp.simplify(1 - q * nu) > 0

a0 = sp.simplify(c_inverse / (d * sp.sqrt(kappa**2 - 1)))
an_prefactor = sp.simplify(2 * c_inverse / (d * sp.sqrt(kappa**2 - 1)))
tail_prefactor = sp.simplify(abs(an_prefactor))
tail_q = sp.simplify(q * nu)

assert sp.simplify(a0 + sp.Rational(17, 30) / sp.sqrt(2)) == 0
assert sp.simplify(an_prefactor + sp.Rational(17, 15) / sp.sqrt(2)) == 0

def tail_bound(n: int) -> sp.Expr:
    return sp.simplify(tail_prefactor * tail_q ** (n + 1) / (1 - tail_q))

def chebyshev_partial_sum(x: mp.mpf, n: int) -> mp.mpf:
    q_mp = mp.mpf(str(sp.N(q, 90)))
    d_mp = mp.mpf("1.5")
    kappa_mp = mp.mpf("3")
    c_inv_mp = mp.mpf("-1.7")
    denom = d_mp * mp.sqrt(kappa_mp**2 - 1)
    result = c_inv_mp / denom
    t_nm1 = mp.mpf(1)
    if n >= 1:
        result += (2 * c_inv_mp / denom) * (-q_mp) * x
    t_n = x
    for degree in range(2, n + 1):
        t_np1 = 2 * x * t_n - t_nm1
        result += (2 * c_inv_mp / denom) * (-q_mp) ** degree * t_np1
        t_nm1, t_n = t_n, t_np1
    return result

def exact_reciprocal(x: mp.mpf) -> mp.mpf:
    r = mp.mpf("4.5") + mp.mpf("1.5") * x
    return mp.mpf("-1.7") / r

tail_values = {}
for n in (4, 8, 12, 16, 24, 32):
    exact = tail_bound(n)
    tail_values[str(n)] = {
        "exact_expression": str(exact),
        "decimal": mp.nstr(mp.mpf(str(sp.N(exact, 100))), 50),
    }
    for x in (mp.mpf(-1), mp.mpf("-0.75"), mp.mpf("-0.25"), mp.mpf(0),
              mp.mpf("0.25"), mp.mpf("0.75"), mp.mpf(1)):
        error = abs(exact_reciprocal(x) - chebyshev_partial_sum(x, n))
        assert error <= mp.mpf(str(sp.N(exact, 100))) * (1 + mp.mpf("1e-70"))

B = c_inverse / sp.Symbol("r") + c_constant + c_linear * sp.Symbol("r") + c_quadratic * sp.Symbol("r")**2
B3 = sp.simplify(B.subs(sp.Symbol("r"), 3))
B6 = sp.simplify(B.subs(sp.Symbol("r"), 6))
assert B3 == sp.Rational(103, 300)
assert B6 == sp.Rational(197, 300)

fixture = {
    "annulus": {"a": str(a), "b": str(b), "c": str(c), "d": str(d)},
    "kappa": str(kappa),
    "q": str(q),
    "q_nu": str(tail_q),
    "a0": str(a0),
    "an_prefactor": str(an_prefactor),
    "tail_bound_formula": str(tail_prefactor) + f" * ({tail_q})^(N+1) / (1-{tail_q})",
    "lapse_at_3": str(B3),
    "lapse_at_6": str(B6),
    "tail_values": tail_values,
    "claim_ceiling": "Analytic inverse-radius Chebyshev tail only; not a full BVP residual enclosure."
}
print(json.dumps(fixture, indent=2, sort_keys=True))

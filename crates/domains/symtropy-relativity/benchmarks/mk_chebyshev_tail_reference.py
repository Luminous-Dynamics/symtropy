#!/usr/bin/env python3
"""Reproduce the exact Chebyshev expansion and tail of the MK inverse-radius term.

Research-only reference script. It does not solve the Bach equation and does
not prove a BVP existence result. The analytic geometric-series formula is the
tail proof; sampled point evaluations are regression checks only.

Requires SymPy and mpmath. Run:
    python3 mk_chebyshev_tail_reference.py
"""

from __future__ import annotations

from fractions import Fraction
from math import isqrt
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

def bilateral_norm_head(n: int) -> sp.Expr:
    """Exact finite symmetric-Laurent norm induced by the physical T_n series."""
    a0 = sp.simplify(c_inverse / (d * sp.sqrt(kappa**2 - 1)))
    one_sided = sp.Abs(a0)
    bilateral = sp.Abs(a0)
    for degree in range(1, n + 1):
        a_n_magnitude = sp.simplify(tail_prefactor * q**degree)
        one_sided += a_n_magnitude * nu**degree
        c_positive = a_n_magnitude / 2
        c_negative = a_n_magnitude / 2
        bilateral += (c_positive + c_negative) * nu**degree
    assert sp.simplify(one_sided - bilateral) == 0
    return sp.simplify(bilateral)

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

# Convert sqrt(2) into an exact rational enclosure using integer arithmetic.
# If m=floor(sqrt(2)*S), then m^2 < 2*S^2 < (m+1)^2.
scale = 10**30
sqrt2_floor = isqrt(2 * scale * scale)
sqrt2_lower = Fraction(sqrt2_floor, scale)
assert sqrt2_floor**2 < 2 * scale**2 < (sqrt2_floor + 1)**2

# q=3-2sqrt(2) and 17/(15sqrt(2)) are bounded above by exact rationals.
q_upper = Fraction(3, 1) - 2 * sqrt2_lower
q_nu_upper = 2 * q_upper
prefactor_upper = Fraction(17, 15) / sqrt2_lower
assert Fraction(0, 1) < q_nu_upper < Fraction(1, 1)

def rational_tail_upper(n: int) -> Fraction:
    # x^(n+1)/(1-x) is increasing on (0,1), so these rational
    # upper bounds enclose the exact geometric tail.
    return prefactor_upper * q_nu_upper ** (n + 1) / (1 - q_nu_upper)

tail_values = {}
for n in (4, 8, 12, 16, 24, 32):
    exact = tail_bound(n)
    exact_decimal = mp.mpf(str(sp.N(exact, 100)))
    rational_upper = rational_tail_upper(n)
    rational_upper_decimal = mp.mpf(rational_upper.numerator) / rational_upper.denominator
    # Independent high-precision check that the exact rational upper fraction
    # dominates the analytic real-valued geometric tail.
    assert rational_upper_decimal >= exact_decimal
    tail_values[str(n)] = {
        "exact_expression": str(exact),
        "decimal": mp.nstr(exact_decimal, 50),
        "rational_upper": str(rational_upper),
        "rational_upper_decimal": mp.nstr(rational_upper_decimal, 50),
    }
    for x in (mp.mpf(-1), mp.mpf("-0.75"), mp.mpf("-0.25"), mp.mpf(0),
              mp.mpf("0.25"), mp.mpf("0.75"), mp.mpf(1)):
        error = abs(exact_reciprocal(x) - chebyshev_partial_sum(x, n))
        assert error <= exact_decimal * (1 + mp.mpf("1e-70"))

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
    "exact_rational_enclosure": {
        "scale": str(scale),
        "sqrt2_lower": str(sqrt2_lower),
        "sqrt2_upper": str(Fraction(sqrt2_floor + 1, scale)),
        "integer_square_witness": {
            "lower": str(sqrt2_floor**2),
            "middle": str(2 * scale**2),
            "upper": str((sqrt2_floor + 1)**2)
        },
        "q_upper": str(q_upper),
        "q_times_nu_upper": str(q_nu_upper),
        "prefactor_upper": str(prefactor_upper),
        "tail_upper_by_degree": {
            str(n): str(rational_tail_upper(n)) for n in (4, 8, 12, 16, 24, 32)
        },
        "rational_bounds_enclose_mpmath_reference": True
    },
    "upstream_bilateral_conversion": {
        "coefficient_map": "c_0=a_0, c_n=c_-n=a_n/2 for n>=1",
        "one_sided_weighted_norm_equals_bilateral_symmetric_norm": True,
        "finite_head_norm_identity_checked_for_N": [4, 8, 12, 16, 24, 32],
        "finite_head_bilateral_norms": {
            str(n): str(bilateral_norm_head(n)) for n in (4, 8, 12, 16, 24, 32)
        }
    },
    "claim_ceiling": "Analytic inverse-radius Chebyshev tail only; not a full BVP residual enclosure."
}
print(json.dumps(fixture, indent=2, sort_keys=True))

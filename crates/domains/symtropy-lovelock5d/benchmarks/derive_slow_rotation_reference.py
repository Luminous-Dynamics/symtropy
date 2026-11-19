#!/usr/bin/env python3
"""Independent symbolic oracle for 5D EGB slow rotation at O(a).

Source: Brihaye, Kleihaus, Kunz & Radu, JHEP 11 (2010) 089,
arXiv:1010.0860, Eqs. (2.20), (2.21), (2.12), and (3.8).

This proves identities of the published first-order perturbative profile.
It does not solve or approximate the fully nonlinear finite-spin branch.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import sympy as sp


def exact_zero(label: str, expression: sp.Expr) -> None:
    result = sp.simplify(sp.factor(expression))
    assert result == 0, f"{label} failed: {result}"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    r, r_h, alpha, spin = sp.symbols(
        "r r_H alpha a", positive=True, finite=True
    )
    C = 2 * r_h**2 + alpha
    s = sp.sqrt(1 + alpha * C / r**4)
    F = sp.factor(1 - C / (r**2 * (1 + s)))
    w = sp.factor(spin * C / (r**4 * (1 + s)))
    w_prime = sp.factor(sp.diff(w, r))
    w_second = sp.factor(sp.diff(w, r, 2))

    expected_w_prime = -2 * spin * C / (r**5 * s)
    expected_w_second = expected_w_prime * (
        -5 / r + 2 * alpha * C / (s**2 * r**5)
    )
    exact_zero("w prime", w_prime - expected_w_prime)
    exact_zero("w second", w_second - expected_w_second)

    # Static BD background: b=f=F, h=g=r^2, so sqrt(f*h/b)=r.
    # The EGB angular Noether bracket is K=r^2-alpha(F-1).
    K = sp.factor(r**2 - alpha * (F - 1))
    exact_zero("K=r^2*s", K - r**2 * s)
    P_w = sp.factor(w_prime * r**3 * K)
    exact_zero("constant angular flux", P_w + 2 * spin * C)

    W = sp.limit(r**4 * w, r, sp.oo)
    exact_zero("asymptotic W", W - spin * C / 2)
    exact_zero("P_w=-4W", P_w + 4 * W)

    # On the physical alpha>=0 branch, C>0 and s>=1, giving a global
    # analytic lower bound K(r)>=r^2 for every exterior r>0.
    # This is stronger than a point-sampled denominator check.

    # Horizon values use the positive root s_H=1+alpha/r_H^2.
    s_h = 1 + alpha / r_h**2
    w_h = spin * C / (r_h**4 * (1 + s_h))
    w1_h = -2 * spin * C / (r_h**5 * s_h)
    b1_h = 2 * r_h / (r_h**2 + alpha)
    exact_zero("horizon angular velocity", w_h - spin / r_h**2)
    exact_zero(
        "horizon derivative",
        w1_h + 2 * spin * C / (r_h**3 * (r_h**2 + alpha)),
    )

    # Independently recover the asymptotic rotational parameter from the
    # horizon relation in Brihaye et al., Eq. (3.8).
    W_horizon = sp.factor(
        sp.Rational(1, 4)
        * r_h**2
        * sp.sqrt(b1_h * r_h**2 / b1_h)
        * w1_h
        * (-r_h**2 - alpha)
    )
    exact_zero("horizon/global angular coefficient", W_horizon - W)

    # At alpha=0, compare with exact MP using Omega_H=a/r_H^2.
    u = (r_h / r) ** 2
    q = spin**2 / r_h**2
    D = 1 - q + q * u**2
    w_mp = spin / r_h**2 * u**2 / D
    w_linear = w.subs(alpha, 0)
    mp_cubic_remainder = sp.factor(w_mp - w_linear)
    expected_mp_cubic = (
        spin**3 / r_h**4 * u**2 * (1 - u**2) / D
    )
    exact_zero("exact MP cubic remainder", mp_cubic_remainder - expected_mp_cubic)

    # Exact rational benchmark point, r_H=1, alpha=7/10, a=1/50, r=2.
    control = {
        r_h: sp.Integer(1),
        alpha: sp.Rational(7, 10),
        spin: sp.Rational(1, 50),
        r: sp.Integer(2),
    }
    sample = {
        "r": "2",
        "s": str(sp.simplify(s.subs(control))),
        "F": str(sp.radsimp(F.subs(control))),
        "w": str(sp.radsimp(w.subs(control))),
        "w_prime": str(sp.radsimp(w_prime.subs(control))),
        "w_second": str(sp.radsimp(w_second.subs(control))),
        "W": str(sp.simplify(W.subs(control))),
        "P_w": str(sp.simplify(P_w.subs(control))),
    }

    fixture = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_slow_rotation_exact_reference",
        "source": {
            "paper": "Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860",
            "equations": ["(2.12)", "(2.20)", "(2.21)", "(3.8)"]
        },
        "expansion": {
            "metric": "b=f=F+O(a^2), h=r^2+O(a^2), g=r^2",
            "rotation": "w=a*W_1(r)+O(a^3)",
            "C": "2*r_H^2+alpha",
            "s": "sqrt(1+alpha*C/r^4)",
            "F": "1-C/(r^2*(1+s))",
            "W_1": "C/(r^4*(1+s))",
            "W_1_prime": "-2*C/(r^5*s)",
            "W_1_second": "W_1_prime*(-5/r+2*alpha*C/(s^2*r^5))"
        },
        "angular_noether": {
            "K": "r^2-alpha*(F-1)=r^2*s",
            "P_w": "-2*a*C",
            "W_asymptotic": "a*C/2",
            "identity": "P_w=-4*W",
            "physical_alpha_nonnegative_bound": "K(r)>=r^2>0 for r>0 and alpha>=0",
            "quadrature": "w_prime=P_w/(r^3*K)"
        },
        "horizon": {
            "Omega_H": "a/r_H^2",
            "w1": "-2*a*C/(r_H^3*(r_H^2+alpha))",
            "b1_equals_f1": "2*r_H/(r_H^2+alpha)",
            "W_from_horizon_relation": "a*C/2"
        },
        "alpha_zero_myers_perry": {
            "u": "(r_H/r)^2",
            "q": "a^2/r_H^2",
            "D": "1-q+q*u^2",
            "Omega_H": "a/r_H^2",
            "exact_w": "a/r_H^2*u^2/D",
            "linear_w": "a/r_H^2*u^2",
            "exact_cubic_remainder": "a^3/r_H^4*u^2*(1-u^2)/D"
        },
        "exact_control_rH1_alpha0_7_a0_02_r2": sample,
        "symbolic_checks": {
            "w_prime": "passed",
            "w_second": "passed",
            "K_equals_r2_times_s": "passed",
            "P_w_constant": "passed",
            "P_w_equals_minus_4W": "passed",
            "asymptotic_W": "passed",
            "horizon_Omega_H": "passed",
            "horizon_w1": "passed",
            "horizon_global_W_relation": "passed",
            "MP_exact_cubic_remainder": "passed"
        },
        "claim_ceiling": (
            "Published first-order slow-rotation profile and exact analytic "
            "identities only. The even-spin metric backreaction starts at O(a^2) "
            "and is not supplied by this fixture. This does not establish a "
            "finite-spin nonzero-alpha solution or uniform validity near extremality."
        )
    }

    rendered = json.dumps(fixture, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

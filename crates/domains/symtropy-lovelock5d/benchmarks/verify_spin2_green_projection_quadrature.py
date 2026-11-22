#!/usr/bin/env python3
"""Independent arbitrary-precision quadrature for the spin-2 Green projection.

This deliberately does not import SymPy, the action-row fixture, or the
Green--Lagrange symbolic verifier. It evaluates the transformed integral
numerically with mpmath and compares it to the closed-form pi^2 amplitudes.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import mpmath as mp

# Taylor coefficients of
# (2/s + (2-s)*log(1-s)/s**2) * (1/s + 12 - 3*s)
# about s=0, stored as exact rationals to avoid precision contamination.
SERIES = [
    (-1, 6), (-13, 6), (-33, 20), (-43, 30), (-533, 420),
    (-159, 140), (-517, 504), (-1177, 1260), (-377, 440),
    (-87, 110), (-6299, 8580), (-1371, 2002), (-3503, 5460),
    (-6589, 10920),
]


def transformed_integrand(t: mp.mpf) -> mp.mpf:
    """s=1-t^2 removes the integrable logarithmic endpoint at s=1."""
    if t == 0:
        return mp.mpf(0)
    s = 1-t*t
    if abs(s) < mp.mpf("1e-3"):
        value = mp.fsum((mp.mpf(num)/den)*s**k
                        for k, (num, den) in enumerate(SERIES))
    else:
        # Since 1-s=t^2, use 2*log(t) to avoid cancellation when t is small.
        log_one_minus_s = 2*mp.log(t)
        kernel = 2/s + (2-s)*log_one_minus_s/s**2
        value = kernel*(1/s+12-3*s)
    return 2*t*value


def calculate(dps: int) -> dict[str, str | int]:
    mp.mp.dps = dps
    points = [mp.mpf(0), mp.mpf("0.1"), mp.mpf("0.5"),
              mp.sqrt(mp.mpf("0.5")), mp.mpf("0.999"), mp.mpf(1)]
    integral = mp.quad(transformed_integrand, points)
    c_h = -2*integral
    c_b = (c_h-10)/3
    c_h_exact = 67-6*mp.pi**2
    c_b_exact = 19-2*mp.pi**2
    return {
        "dps": dps,
        "integral": mp.nstr(integral, dps-8),
        "C_H_numeric": mp.nstr(c_h, dps-8),
        "C_H_exact": mp.nstr(c_h_exact, dps-8),
        "C_H_absolute_error": mp.nstr(abs(c_h-c_h_exact), 8),
        "C_B_numeric": mp.nstr(c_b, dps-8),
        "C_B_exact": mp.nstr(c_b_exact, dps-8),
        "C_B_absolute_error": mp.nstr(abs(c_b-c_b_exact), 8),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dps", type=int, nargs="+", default=[40, 60, 80])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if any(d < 30 or d > 300 for d in args.dps):
        raise SystemExit("dps must be in [30, 300]")
    runs = [calculate(d) for d in args.dps]
    # At least 25 digits must agree between the independent quadrature and
    # closed form for every requested precision. Precision may be limited by
    # quadrature convergence, but cannot silently pass at low accuracy.
    errors_h = [mp.mpf(row["C_H_absolute_error"]) for row in runs]
    errors_b = [mp.mpf(row["C_B_absolute_error"]) for row in runs]
    if any(error > mp.mpf("1e-25") for error in errors_h + errors_b):
        raise ArithmeticError(f"Green projection quadrature did not converge: H={errors_h}, B={errors_b}")
    result = {
        "schema_version": 1,
        "artifact_kind": "spin2_green_projection_independent_quadrature",
        "status": "independent_mpmath_quadrature_pass",
        "method": "mpmath tanh-sinh quadrature after s=1-t^2 endpoint regularization",
        "precision_runs": runs,
        "acceptance": {
            "all_C_H_errors_below_1e_minus_25": True,
            "all_C_B_errors_below_1e_minus_25": True,
            "C_B_reconstructed_from_C_H": True,
            "independent_of_SymPy_and_action_row_fixture": True
        },
        "environment": {
            "python": platform.python_version(),
            "mpmath": mp.__version__,
            "arithmetic": "arbitrary-precision numerical quadrature"
        },
        "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "claim_ceiling": "Independent numerical evaluation of the Green integral and its B-amplitude reconstruction only; not an action-row replay, finite-lambda BVP, interval enclosure, or existence theorem."
    }
    rendered = json.dumps(result, indent=2, sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()

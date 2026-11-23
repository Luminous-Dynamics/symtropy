#!/usr/bin/env python3
"""Composite Gauss-Legendre oracle with analytic endpoint-log subtraction.

Independent of SymPy, the action-row fixture, and the tanh-sinh oracle.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import mpmath as mp

# Exact coefficients of the non-log factor in the transformed integrand:
# (2/s + (2-s)log(1-s)/s^2)(1/s + 12 - 3s), expanded at s=0.
S_SERIES = [
    (-1, 6), (-13, 6), (-33, 20), (-43, 30), (-533, 420),
    (-159, 140), (-517, 504), (-1177, 1260), (-377, 440),
    (-87, 110), (-6299, 8580), (-1371, 2002), (-3503, 5460),
    (-6589, 10920),
]

# Coefficients of log(t) in the s=1-t^2 transformed integrand:
# L(t) = 4 t (1+t^2)(3t^4+6t^2-10)/(1-t^2)^3.
# L(t) = sum_k LOG_COEFFS[k] * t^(2k+1).
LOG_COEFFS = [40, 136, 252, 376, 508, 648, 796, 952, 1116, 1288, 1468, 1656, 1852]


def transformed_integrand(t: mp.mpf) -> mp.mpf:
    """Original Green integrand after s=1-t^2, including ds/dt."""
    if t == 0:
        return mp.mpf(0)
    s = 1 - t*t
    if abs(s) < mp.mpf("1e-3"):
        product = mp.fsum((mp.mpf(a)/b)*s**k for k, (a, b) in enumerate(S_SERIES))
    else:
        log_one_minus_s = 2*mp.log(t)
        kernel = 2/s + (2-s)*log_one_minus_s/s**2
        product = kernel*(1/s + 12 - 3*s)
    return 2*t*product


def log_subtracted_integrand(t: mp.mpf) -> mp.mpf:
    """Remove the first 13 exact odd-power log terms at t=0."""
    if t == 0:
        return mp.mpf(0)
    singular_model = mp.fsum(
        mp.mpf(a)*t**(2*k+1)*mp.log(t)
        for k, a in enumerate(LOG_COEFFS)
    )
    return transformed_integrand(t) - singular_model


def composite_gauss_legendre(f, panels: int, order: int) -> mp.mpf:
    nodes, weights = mp.gauss_quadrature(order, "legendre")
    total = mp.mpf(0)
    for panel in range(panels):
        left = mp.mpf(panel)/panels
        right = mp.mpf(panel+1)/panels
        midpoint = (left+right)/2
        halfwidth = (right-left)/2
        total += halfwidth*mp.fsum(
            weights[i]*f(midpoint+halfwidth*nodes[i])
            for i in range(order)
        )
    return total


def calculate(dps: int, panels: int, order: int) -> dict:
    mp.mp.dps = dps
    regular_part = composite_gauss_legendre(log_subtracted_integrand, panels, order)
    # Integral_0^1 t^(2k+1) log(t) dt = -1/(2k+2)^2.
    analytic_log_integral = mp.fsum(
        mp.mpf(a)/mp.mpf((2*k+2)**2)
        for k, a in enumerate(LOG_COEFFS)
    )
    integral = regular_part - analytic_log_integral
    c_h = -2*integral
    c_b = (c_h-10)/3
    c_h_exact = 67-6*mp.pi**2
    c_b_exact = 19-2*mp.pi**2
    return {
        "dps": dps,
        "panels": panels,
        "gauss_legendre_order_per_panel": order,
        "integral": mp.nstr(integral, dps-8),
        "C_H_numeric": mp.nstr(c_h, dps-8),
        "C_H_exact": mp.nstr(c_h_exact, dps-8),
        "C_H_absolute_error": mp.nstr(abs(c_h-c_h_exact), 12),
        "C_B_numeric": mp.nstr(c_b, dps-8),
        "C_B_exact": mp.nstr(c_b_exact, dps-8),
        "C_B_absolute_error": mp.nstr(abs(c_b-c_b_exact), 12),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dps", type=int, nargs="+", default=[40, 60, 80])
    parser.add_argument("--panels", type=int, default=4)
    parser.add_argument("--order", type=int, default=24)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if any(d < 30 or d > 300 for d in args.dps):
        raise SystemExit("dps must be in [30, 300]")
    if not (1 <= args.panels <= 256 and 8 <= args.order <= 128):
        raise SystemExit("panels must be 1..256 and order 8..128")

    runs = [calculate(d, args.panels, args.order) for d in args.dps]
    errors_h = [mp.mpf(row["C_H_absolute_error"]) for row in runs]
    errors_b = [mp.mpf(row["C_B_absolute_error"]) for row in runs]
    for row, err_h, err_b in zip(runs, errors_h, errors_b):
        threshold = mp.mpf("1e-30") if row["dps"] <= 40 else mp.mpf("1e-45")
        if err_h > threshold or err_b > threshold:
            raise ArithmeticError(
                f"Gauss-Legendre comparison failed at {row['dps']} dps: "
                f"C_H={err_h}, C_B={err_b}, threshold={threshold}"
            )

    result = {
        "schema_version": 1,
        "artifact_kind": "spin2_green_projection_composite_gauss_legendre",
        "status": "independent_composite_gauss_legendre_pass",
        "method": (
            "composite Gauss-Legendre quadrature after s=1-t^2, "
            "subtracting 13 analytic odd-power log terms and adding their "
            "integrals back exactly"
        ),
        "precision_runs": runs,
        "acceptance": {
            "all_C_H_errors_within_precision_threshold": True,
            "all_C_B_errors_within_precision_threshold": True,
            "analytic_log_subtraction_restored_exactly": True,
            "independent_of_SymPy_and_action_row_fixture": True,
            "different_quadrature_family_from_tanh_sinh_oracle": True
        },
        "environment": {
            "python": platform.python_version(),
            "mpmath": mp.__version__,
            "arithmetic": "arbitrary-precision composite Gauss-Legendre quadrature"
        },
        "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "claim_ceiling": (
            "Independent numerical quadrature of the Green endpoint integral "
            "and B-amplitude reconstruction only. Numerical discrepancies are "
            "not rigorous interval enclosures. No action-row replay, finite-lambda "
            "BVP, or existence theorem is claimed."
        )
    }
    rendered = json.dumps(result, indent=2, sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()

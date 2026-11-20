#!/usr/bin/env python3
"""Dependency-free pseudo-arclength fold control for BH branch-atlas work.

Research-only scalar control: F(u, lambda) = u^2 - lambda = 0.
Traces the complete connected parabola through its simple fold at (0, 0).
It is a method control, not a black-hole continuation solver.
"""
from __future__ import annotations
import argparse
import json
import math
from pathlib import Path


def solve_2x2(a11, a12, a21, a22, b1, b2):
    det = a11 * a22 - a12 * a21
    if not math.isfinite(det) or abs(det) < 1e-14:
        raise ArithmeticError(f"singular bordered Jacobian: det={det!r}")
    return ((b1 * a22 - a12 * b2) / det,
            (a11 * b2 - b1 * a21) / det)


def weighted_tangent(u, wu, wl):
    # Orient from positive u through the fold toward negative u.
    norm = math.sqrt(wu * wu + (2.0 * u * wl) ** 2)
    return -1.0 / norm, -2.0 * u / norm


def corrector(up, lp, tu, tl, wu, wl, tol=2e-13, max_iter=20):
    u, lam = up, lp
    for iteration in range(1, max_iter + 1):
        f = u * u - lam
        arc = tu * wu * wu * (u - up) + tl * wl * wl * (lam - lp)
        if max(abs(f), abs(arc)) <= tol:
            det = 2.0 * u * tl * wl * wl + tu * wu * wu
            return u, lam, iteration - 1, det
        du, dl = solve_2x2(
            2.0 * u, -1.0,
            tu * wu * wu, tl * wl * wl,
            -f, -arc,
        )
        u += du
        lam += dl
        if not (math.isfinite(u) and math.isfinite(lam)):
            raise ArithmeticError("non-finite corrector iterate")
    raise ArithmeticError("pseudo-arclength corrector did not converge")


def trace_fold(wu, wl, ds=0.04, start_u=1.0, end_u=-1.0):
    if wu <= 0 or wl <= 0 or ds <= 0:
        raise ValueError("weights and step must be positive")
    u, lam = start_u, start_u * start_u
    tu, tl = weighted_tangent(u, wu, wl)
    points = [{"u": u, "lambda": lam, "t_u": tu, "t_lambda": tl,
               "newton_iterations": 0, "bordered_det": None}]
    max_residual = abs(u * u - lam)
    min_abs_bordered_det = math.inf
    max_iterations = 0
    crossed = False
    for _ in range(5000):
        up, lp = u + ds * tu, lam + ds * tl
        un, ln, iterations, det = corrector(up, lp, tu, tl, wu, wl)
        tun, tln = weighted_tangent(un, wu, wl)
        if tun * tu * wu * wu + tln * tl * wl * wl < 0:
            tun, tln = -tun, -tln
        residual = abs(un * un - ln)
        max_residual = max(max_residual, residual)
        min_abs_bordered_det = min(min_abs_bordered_det, abs(det))
        max_iterations = max(max_iterations, iterations)
        points.append({"u": un, "lambda": ln, "t_u": tun, "t_lambda": tln,
                       "newton_iterations": iterations, "bordered_det": det})
        if u > 0 >= un:
            crossed = True
        u, lam, tu, tl = un, ln, tun, tln
        if u <= end_u:
            break
    else:
        raise RuntimeError("continuation exceeded step budget")
    if not crossed or points[-1]["u"] > end_u:
        raise AssertionError("branch did not traverse the fold and reach negative-u side")
    if any(p["lambda"] < -1e-10 for p in points):
        raise AssertionError("continuation left the exact parabola")
    tangent_lambdas = [p["t_lambda"] for p in points]
    if not (min(tangent_lambdas) < 0 < max(tangent_lambdas)):
        raise AssertionError("parameter tangent did not turn through zero")
    return {
        "arclength_metric_weights": {"u": wu, "lambda": wl},
        "step_size": ds,
        "point_count": len(points),
        "fold_crossed": crossed,
        "u_min": min(p["u"] for p in points),
        "u_max": max(p["u"] for p in points),
        "lambda_min": min(p["lambda"] for p in points),
        "lambda_max": max(p["lambda"] for p in points),
        "lambda_tangent_min": min(tangent_lambdas),
        "lambda_tangent_max": max(tangent_lambdas),
        "maximum_original_residual": max_residual,
        "minimum_absolute_bordered_determinant": min_abs_bordered_det,
        "maximum_corrector_iterations": max_iterations,
        "samples": points,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    runs = [trace_fold(1.0, 1.0), trace_fold(10.0, 0.1)]
    for run in runs:
        assert run["fold_crossed"]
        assert run["maximum_original_residual"] < 1e-11
        assert run["minimum_absolute_bordered_determinant"] > 0
    result = {
        "schema_version": 1,
        "artifact_kind": "pseudo_arclength_simple_fold_control",
        "status": "executed_dependency_free_numeric_reference",
        "equation": "F(u,lambda)=u^2-lambda=0",
        "exact_branch": "lambda=u^2",
        "exact_fold": {"u": 0, "lambda": 0},
        "method": "weighted pseudo-arclength predictor-corrector with bordered Newton solve",
        "runs": runs,
        "acceptance": {
            "both_metrics_cross_fold": all(r["fold_crossed"] for r in runs),
            "both_metrics_reach_negative_u_branch": all(r["u_min"] < 0 for r in runs),
            "all_original_residuals_below_1e_minus_11": all(r["maximum_original_residual"] < 1e-11 for r in runs),
            "all_bordered_solves_nonsingular": all(r["minimum_absolute_bordered_determinant"] > 0 for r in runs),
        },
        "claim_ceiling": "This validates a scalar pseudo-arclength method control only; it is not a nonlinear EGB continuation result, fold discovery, or branch-completeness proof.",
    }
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

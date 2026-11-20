#!/usr/bin/env python3
"""Strong-coupling asymptotics of the isotropic BD horizon coordinate.

Research-only analysis. Requires mpmath and the sibling isotropic reference
module. This is not a production solver or interval proof.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import mpmath as mp
import isotropic_gauge_reference as gauge


def inner_constant() -> mp.mpf:
    root2 = mp.sqrt(2)
    return (
        -1
        - (2 + root2) * mp.log(1 + 1/root2) / 4
        - (2 - root2) * mp.log(1 - 1/root2) / 4
        + mp.log(2)
    )


def build(dps: int) -> dict:
    mp.mp.dps = dps
    c_inner = inner_constant()
    prefactor = mp.exp(-c_inner)
    rows = []
    previous_error = mp.inf
    for lam_s in ("100", "1000", "10000", "100000", "1000000"):
        lam = mp.mpf(lam_s)
        i_h = gauge.horizon_log_normalization(lam)
        xi_h = mp.exp(-i_h)
        xi_asymptotic = prefactor * mp.sqrt(lam) * mp.exp(-mp.pi*mp.sqrt(lam)/2)
        ratio = xi_h / xi_asymptotic
        error = abs(1 - ratio)
        if not (xi_h > 0 and 0 < ratio < 1.1):
            raise AssertionError(f"invalid asymptotic horizon ratio at lambda={lam}: {ratio}")
        if error >= previous_error:
            raise AssertionError(
                f"asymptotic prefactor convergence did not improve at lambda={lam}"
            )
        if error >= mp.mpf("0.05"):
            raise AssertionError(
                f"strong-coupling asymptotic mismatch {error} exceeds 0.05 at lambda={lam}"
            )
        previous_error = error
        rows.append({
            "lambda": lam_s,
            "log_xi_horizon": mp.nstr(-i_h, 55),
            "xi_horizon": mp.nstr(xi_h, 55),
            "leading_asymptotic_xi_horizon": mp.nstr(xi_asymptotic, 55),
            "exact_to_asymptotic_ratio": mp.nstr(ratio, 35),
            "relative_asymptotic_error": mp.nstr(error, 12),
        })
    return {
        "schema_version": 1,
        "artifact_kind": "5d_bd_isotropic_horizon_strong_coupling_asymptotic",
        "precision": {"mpmath_dps": dps},
        "matched_asymptotic": {
            "I_lambda": "-log(xi_H)=pi*sqrt(lambda)/2 - log(lambda)/2 + C_inner + o(1)",
            "xi_H": "A_inner*sqrt(lambda)*exp(-pi*sqrt(lambda)/2)*(1+o(1))",
            "C_inner_exact": (
                "-1 -(2+sqrt(2))*log(1+1/sqrt(2))/4 "
                "-(2-sqrt(2))*log(1-1/sqrt(2))/4 + log(2)"
            ),
            "C_inner_numeric": mp.nstr(c_inner, 55),
            "A_inner_exact": "exp(-C_inner)",
            "A_inner_numeric": mp.nstr(prefactor, 55),
            "inner_lapse_inverse_sqrt": "A(t)=sqrt((1+t^2+sqrt(1+t^4))/2), t=sqrt(lambda)*y",
            "inner_constant_integral": "C_inner=lim(T->inf)[integral_0^T (A(t)-1)/t dt - T + log(T)]",
            "inner_substitution": "t^2=2*u^2*(u^2-1)/(2*u^2-1), u=A(t)",
            "horizon_log_chart": "eta=log(xi/xi_H), d(eta)/d(rho)=1/(rho*sqrt(F))",
        },
        "cases": rows,
        "checks": {
            "positive_horizon_radius": True,
            "asymptotic_ratio_converges_monotonically_on_frozen_grid": True,
            "max_relative_error_on_grid": rows[0]["relative_asymptotic_error"],
            "all_relative_errors_below_0_05": True,
        },
        "claim_ceiling": (
            "Matched-asymptotic prediction for the static BD isotropic horizon coordinate, "
            "numerically checked on the listed lambda grid. The coordinate collapse is gauge-related, "
            "not a physical decrease of the areal horizon radius. The o(1) remainder is not an interval bound."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dps", type=int, default=90)
    args = parser.parse_args()
    if args.dps < 50:
        raise SystemExit("--dps must be at least 50")
    result = build(args.dps)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True)+"\n", encoding="utf-8")
    print(json.dumps(result["checks"], indent=2))
    print("A_inner =", result["matched_asymptotic"]["A_inner_numeric"])
    for row in result["cases"]:
        print(row["lambda"], row["exact_to_asymptotic_ratio"])


if __name__ == "__main__":
    main()

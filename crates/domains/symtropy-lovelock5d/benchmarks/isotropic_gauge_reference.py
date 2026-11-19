#!/usr/bin/env python3
"""High-precision reference for the 5D static BD areal/isotropic map.

Research/benchmark code only; not a production dependency or a certified
interval integrator. Requires mpmath. The isotropic variables follow the
notation in footnote 2 of Brihaye et al. (2010).
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import mpmath as mp


def lapse_y(y: mp.mpf, lam: mp.mpf) -> mp.mpf:
    """Stable BD Einstein-branch lapse with y = 1/rho."""
    return 2 * (1 - y*y) / (
        1 + lam*y*y + mp.sqrt(1 + lam*(2 + lam)*y**4)
    )


def jacobian_integrand(y: mp.mpf, lam: mp.mpf) -> mp.mpf:
    if y == 0:
        return mp.mpf("0")
    return (1/mp.sqrt(lapse_y(y, lam)) - 1) / y


def log_normalization(y: mp.mpf, lam: mp.mpf) -> mp.mpf:
    return mp.quad(lambda z: jacobian_integrand(z, lam), [0, y])


def horizon_log_normalization(lam: mp.mpf) -> mp.mpf:
    # u=sqrt(1-y) removes the integrable inverse-square-root endpoint.
    # The transformed integrand tends to sqrt(2*(1+lambda)) at u=0.
    def regularized(u: mp.mpf) -> mp.mpf:
        if u < mp.mpf("1e-30"):
            return mp.sqrt(2*(1 + lam))
        return 2*u*jacobian_integrand(1-u*u, lam)
    return mp.quad(regularized, [0, 1])


def xi_of_rho(rho: mp.mpf, lam: mp.mpf) -> mp.mpf:
    return rho*mp.exp(-log_normalization(1/rho, lam))


def sample(rho: mp.mpf, lam: mp.mpf) -> dict[str, str]:
    C = 2 + lam
    y = 1/rho
    F = lapse_y(y, lam)
    xi = xi_of_rho(rho, lam)
    f_iso = (xi/rho)**2
    m = F/f_iso
    n = F*rho**2/xi**2
    s = mp.sqrt(1 + lam*C/rho**4)
    W1 = C/(rho**4*(1+s))
    return {
        "rho": mp.nstr(rho, 12),
        "F_areal": mp.nstr(F, 55),
        "xi_isotropic": mp.nstr(xi, 55),
        "f_isotropic": mp.nstr(f_iso, 55),
        "m": mp.nstr(m, 55),
        "n": mp.nstr(n, 55),
        "bar_w_over_a": mp.nstr(xi*W1, 55),
    }


def build_fixture(dps: int) -> dict:
    mp.mp.dps = dps
    cases = []
    max_inverse_error = mp.mpf("0")
    max_derivative_defect = mp.mpf("0")
    max_radial_defect = mp.mpf("0")
    max_mp_error = mp.mpf("0")
    for lam_s in ("0", "0.7", "3"):
        lam = mp.mpf(lam_s)
        xi_h = mp.exp(-horizon_log_normalization(lam))
        rows = []
        for rho_s in ("1.1", "2", "10"):
            rho = mp.mpf(rho_s)
            vals = sample(rho, lam)
            xi = xi_of_rho(rho, lam)
            F = lapse_y(1/rho, lam)
            dxi = mp.diff(lambda R: xi_of_rho(R, lam), rho)
            dxi_rhs = xi/(rho*mp.sqrt(F))
            derivative_defect = abs(dxi/dxi_rhs - 1)
            f_iso = (xi/rho)**2
            radial_defect = abs((dxi**2/f_iso)/(1/F) - 1)
            inverse = mp.findroot(
                lambda R: xi_of_rho(R, lam) - xi,
                (rho*(1-mp.mpf("1e-8")), rho*(1+mp.mpf("1e-8"))),
            )
            inverse_error = abs(inverse-rho)
            max_derivative_defect = max(max_derivative_defect, derivative_defect)
            max_radial_defect = max(max_radial_defect, radial_defect)
            max_inverse_error = max(max_inverse_error, inverse_error)
            if lam == 0:
                xi_exact = (rho + mp.sqrt(rho**2 - 1))/2
                mp_error = abs(xi-xi_exact)
                max_mp_error = max(max_mp_error, mp_error)
            else:
                mp_error = mp.mpf("0")
            vals.update({
                "d_xi_d_rho": mp.nstr(dxi, 55),
                "derivative_relative_defect": mp.nstr(derivative_defect, 8),
                "radial_metric_relative_defect": mp.nstr(radial_defect, 8),
                "inverse_radius_absolute_error": mp.nstr(inverse_error, 8),
                "alpha_zero_exact_map_absolute_error": mp.nstr(mp_error, 8),
            })
            if rho_s == "2":
                rows.append(vals)
        cases.append({
            "lambda": lam_s,
            "C": mp.nstr(2+lam, 20),
            "xi_horizon": mp.nstr(xi_h, 55),
            "samples": rows,
        })
    return {
        "schema_version": 1,
        "artifact_kind": "5d_static_boulware_deser_areal_isotropic_gauge_reference",
        "precision": {"mpmath_dps": dps, "sample_value_digits": 55},
        "source": {
            "paper": "Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860",
            "location": "metric ansatz Eq. (2.8), isotropic-gauge transformation in page-4 footnote 2",
        },
        "dimensionless_conventions": {
            "rho": "R/r_H", "xi": "x/r_H", "lambda": "alpha/r_H^2", "r_H": "1",
            "normalization": "xi/rho -> 1 as rho -> infinity",
        },
        "derived_map": {
            "F_y": "2*(1-y^2)/(1+lambda*y^2+sqrt(1+lambda*(2+lambda)*y^4))",
            "J_y": "(1/sqrt(F_y)-1)/y",
            "xi": "rho*exp(-integral_0^(1/rho) J_y dy)",
            "d_log_xi_d_rho": "1/(rho*sqrt(F))",
            "bar_f": "b_A",
            "f_I": "xi^2/rho^2",
            "m": "b_A*rho^2/xi^2",
            "n": "b_A*h_A/xi^2",
            "bar_w": "xi*w_A",
            "g_I_identity": "xi^2*m/bar_f = rho^2",
            "h_I_identity": "xi^2*n/bar_f = h_A",
            "w_I_identity": "bar_w/xi = w_A",
            "radial_metric_identity": "(d xi/d rho)^2/f_I = 1/F",
            "alpha_zero_exact_map": "xi=(rho+sqrt(rho^2-1))/2; rho=xi+1/(4*xi)",
            "horizon_lapse": "F=2*(rho-1)/(1+lambda)+O((rho-1)^2)",
            "horizon_map": "xi-xi_H=xi_H*sqrt(2*(1+lambda)*(rho-1))+O(rho-1)",
            "asymptotic_map": "xi=rho-(2+lambda)/(8*rho)+O(rho^-3)",
            "linear_spin_transform": "bar_w/a=xi*(2+lambda)/(rho^4*(1+sqrt(1+lambda*(2+lambda)/rho^4)))+O(a^2)",
        },
        "cases": cases,
        "execution": {
            "high_precision_map_and_inverse_evaluated": True,
            "derivative_ode_checked_with_mpmath_diff": True,
            "radial_metric_identity_checked": True,
            "alpha_zero_closed_form_checked": True,
            "validation_radii": ["1.1", "2", "10"],
            "max_inverse_radius_absolute_error": mp.nstr(max_inverse_error, 8),
            "max_derivative_relative_defect": mp.nstr(max_derivative_defect, 8),
            "max_radial_metric_relative_defect": mp.nstr(max_radial_defect, 8),
            "max_alpha_zero_closed_form_absolute_error": mp.nstr(max_mp_error, 8),
        },
        "claim_ceiling": (
            "High-precision research reference for the static 5D Einstein-branch BD map and its "
            "linear-spin coordinate transform. It is not interval-certified, does not derive an "
            "independent isotropic-gauge EGB residual, and does not compare a nonlinear rotating solution."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dps", type=int, default=90)
    args = parser.parse_args()
    if args.dps < 40:
        raise SystemExit("--dps must be at least 40")
    data = build_fixture(args.dps)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(data, indent=2, sort_keys=True)+"\n", encoding="utf-8")
    print(json.dumps(data["execution"], indent=2))
    for case in data["cases"]:
        print(f"lambda={case['lambda']} xi_H={case['xi_horizon']}")


if __name__ == "__main__":
    main()

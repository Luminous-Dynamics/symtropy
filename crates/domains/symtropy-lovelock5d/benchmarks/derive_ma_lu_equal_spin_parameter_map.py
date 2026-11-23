#!/usr/bin/env python3
"""Exact equal-spin 5D Myers-Perry parameter map for Ma-Lu Eq. (2.9)."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp


def zero(label: str, expr: sp.Expr) -> None:
    residual = sp.factor(sp.cancel(expr))
    if residual != 0:
        raise AssertionError(f"{label}: {sp.sstr(residual)}")


def derive() -> dict:
    q, nu, rh = sp.symbols("q nu r_h", positive=True)
    mu = rh**2 * (1 + q**2)**2 / 2
    nu_q = q / (1 + q**2)
    chi_q = sp.factor(q * rh / sp.sqrt(mu))
    zero("MP horizon relation", (rh**2 + (q*rh)**2)**2/rh**2 - 2*mu)
    zero("chi=a/sqrt(mu)", chi_q - sp.sqrt(2)*q/(1+q**2))
    zero("chi=sqrt(2)*nu", chi_q - sp.sqrt(2)*nu_q)

    c = sp.symbols("c0:6")
    q_ansatz = sum(c[n]*nu**(2*n+1) for n in range(6))
    residual = sp.series(q_ansatz/(1+q_ansatz**2)-nu, nu, 0, 12).removeO().expand()
    sol = sp.solve([residual.coeff(nu, 2*n+1) for n in range(6)], c, dict=True)
    if len(sol) != 1:
        raise AssertionError(f"inverse series not unique: {sol}")
    q_nu = sp.expand(q_ansatz.subs(sol[0]))
    zero("inverse q(nu) through nu^11", sp.series(q_nu/(1+q_nu**2)-nu, nu, 0, 12).removeO())

    chi = sp.Symbol("chi")
    mu_chi = sp.series((1+q_nu**2)**2/2, nu, 0, 8).removeO().subs(nu, chi/sp.sqrt(2))
    zero("Ma-Lu Eq. (3.9) equal-spin mass expansion",
         sp.series(mu_chi-(sp.Rational(1,2)+chi**2/2+5*chi**4/8+7*chi**6/8), chi, 0, 8).removeO())

    agb, ar, tensor = sp.symbols("alpha_GB alpha_R T")
    coupling = sp.solve(sp.Eq(agb*tensor/4, ar*tensor), ar)
    if coupling != [agb/4]:
        raise AssertionError(f"coupling map: {coupling}")
    zero("first-coupling normalization", (ar*tensor-agb*tensor/4).subs(ar, coupling[0]))

    return {
        "schema_version": 1,
        "artifact_kind": "ma_lu_equal_spin_parameter_map",
        "status": "exact_symbolic_parameter_relations_derived",
        "source_equations": {
            "ma_lu_eq_2_1": "Delta_r=((r^2+a^2)(r^2+b^2))/r^2-2*mu",
            "ma_lu_eq_2_2": "Omega_a=a/(r_h^2+a^2), Omega_b=b/(r_h^2+b^2)",
            "ma_lu_eq_2_9": "chi_a=a/sqrt(mu), chi_b=b/sqrt(mu)",
            "ma_lu_eq_3_9_equal_spin": "mu/r_h^2=1/2+chi^2/2+5*chi^4/8+7*chi^6/8+O(chi^8)"
        },
        "definitions": {"q":"a/r_h", "nu":"Omega_H*r_h", "chi":"a/sqrt(mu)",
                        "symtropy_spin":"a_S/r_H=Omega_H*r_H=nu"},
        "exact_equal_spin_relations": {
            "mu_over_rh2": sp.sstr(sp.factor(mu/rh**2)),
            "nu_of_q": sp.sstr(nu_q),
            "chi_of_q": sp.sstr(chi_q),
            "chi_of_nu": "sqrt(2)*nu",
            "q_of_nu_exact_small_root": "(1-sqrt(1-4*nu^2))/(2*nu)",
            "valid_subextremal_branch": "0<=nu<1/2, 0<=q<1"
        },
        "series_about_zero_spin": {
            "q_of_nu_through_nu11": sp.sstr(q_nu),
            "chi_of_nu_exact": "sqrt(2)*nu",
            "mu_over_rh2_of_chi_through_chi6": "1/2+chi^2/2+5*chi^4/8+7*chi^6/8",
            "q_minus_nu_order": "O(nu^3)"
        },
        "coupling_map": {
            "symtropy_action": "R+alpha_GB*L_GB/4", "ma_lu_action": "R+alpha_R*Riemann^2",
            "candidate_first_coupling_map": "alpha_R=alpha_GB/4",
            "dimensionless_map_for_same_horizon": "alpha_R/r_h^2=lambda/4",
            "scope": "First-order correction about Ricci-flat Myers-Perry only"
        },
        "checks": {"horizon_relation_exact": True, "angular_velocity_relation_exact": True,
                    "chi_equals_a_over_sqrt_mu": True, "chi_equals_sqrt2_nu_exact": True,
                    "inverse_series_exact_through_nu11": True,
                    "eq_3_9_equal_spin_mass_series": True,
                    "coupling_coefficient_identity": True},
        "claim_ceiling": "Exact kinematic map and coupling normalization only; no corrected metric/gauge comparison or finite-coupling EGB result is claimed."
    }


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    result = derive()
    result["source_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    result["environment"] = {"python": platform.python_version(), "sympy": sp.__version__}
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()

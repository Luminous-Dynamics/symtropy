#!/usr/bin/env python3
"""Exact equal-spin 5D Myers-Perry parameter map for the Ma-Lu benchmark.

Eq. (2.9) of Ma-Lu defines chi_a=a/sqrt(mu), chi_b=b/sqrt(mu).
This tool derives the exact equal-spin relation to nu=Omega_H*r_h and its
inverse small-spin expansion. It does not compare corrected metrics.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expr: sp.Expr) -> None:
    reduced = sp.factor(sp.cancel(expr))
    if reduced != 0:
        raise AssertionError(f"{label}: {sp.sstr(reduced)}")


def derive() -> dict:
    q, nu, r_h = sp.symbols("q nu r_h", positive=True)
    alpha_gb, alpha_r, tensor = sp.symbols("alpha_GB alpha_R T")

    # Ma-Lu Eq. (2.1): Delta_r(r_h)=0, with equal spins a=b=q*r_h.
    mu_horizon = r_h**2 * (1 + q**2)**2 / 2
    exact_zero(
        "equal-spin horizon equation",
        (r_h**2 + (q*r_h)**2)**2 / r_h**2 - 2*mu_horizon,
    )

    # Ma-Lu Eq. (2.2): Omega_H=a/(r_h^2+a^2).
    nu_of_q = q / (1 + q**2)

    # Ma-Lu Eq. (2.9): chi=a/sqrt(mu), exactly sqrt(2)*nu at equal spin.
    chi_of_q = sp.factor(q*r_h / sp.sqrt(mu_horizon))
    exact_zero("printed chi=a/sqrt(mu) relation",
               chi_of_q - sp.sqrt(2)*q/(1+q**2))
    exact_zero("chi=sqrt(2)*nu exactly",
               chi_of_q - sp.sqrt(2)*nu_of_q)

    # Invert nu(q) on the small, subextremal root through nu^11.
    coeffs = sp.symbols("c0:6")
    q_ansatz = sum(coeffs[n] * nu**(2*n+1) for n in range(6))
    residual = sp.series(q_ansatz/(1+q_ansatz**2)-nu, nu, 0, 12).removeO().expand()
    equations = [residual.coeff(nu, 2*n+1) for n in range(6)]
    solution = sp.solve(equations, coeffs, dict=True)
    if len(solution) != 1:
        raise AssertionError(f"inverse series not uniquely fixed: {solution}")
    q_of_nu = sp.expand(q_ansatz.subs(solution[0]))
    exact_zero(
        "inverse series through nu^11",
        sp.series(q_of_nu/(1+q_of_nu**2)-nu, nu, 0, 12).removeO(),
    )

    # The paper's Eq. (3.9) equal-spin mu expansion is an additional control.
    mu_series_in_chi = sp.series(
        (1+q_of_nu**2)**2/2, nu, 0, 8
    ).removeO().subs(nu, sp.Symbol("chi")/sp.sqrt(2))
    chi = sp.Symbol("chi")
    exact_zero(
        "Eq. (3.9) equal-spin mass series",
        sp.series(mu_series_in_chi - (sp.Rational(1,2)
          + chi**2/2 + 5*chi**4/8 + 7*chi**6/8), chi, 0, 8).removeO(),
    )

    # First-order coefficient matching on a Ricci-flat MP background.
    coupling_solution = sp.solve(
        sp.Eq(alpha_gb*tensor/4, alpha_r*tensor), alpha_r
    )
    if coupling_solution != [alpha_gb/4]:
        raise AssertionError(f"unexpected coupling map: {coupling_solution}")
    exact_zero(
        "coupling coefficient match",
        (alpha_r*tensor-alpha_gb*tensor/4).subs(alpha_r, coupling_solution[0]),
    )

    return {
        "schema_version": 1,
        "artifact_kind": "ma_lu_equal_spin_parameter_map",
        "status": "exact_symbolic_parameter_relations_derived",
        "source_equations": {
            "ma_lu_eq_2_1": "Delta_r=((r^2+a^2)(r^2+b^2))/r^2-2*mu",
            "ma_lu_eq_2_2_equal_spin": "Omega_H=a/(r_h^2+a^2)",
            "ma_lu_eq_2_9_printed": "chi_a=a/sqrt(mu), chi_b=b/sqrt(mu)",
            "ma_lu_eq_3_9_equal_spin_crosscheck":
                "mu/r_h^2=1/2+chi^2/2+5*chi^4/8+7*chi^6/8+O(chi^8)",
        },
        "definitions": {
            "q": "a/r_h",
            "nu": "Omega_H*r_h",
            "chi": "a/sqrt(mu) (paper's Eq. 2.9)",
            "symtropy_spin": "a_S/r_H=Omega_H*r_H=nu",
        },
        "exact_equal_spin_relations": {
            "mu_over_rh2": sp.sstr(sp.factor(mu_horizon/r_h**2)),
            "nu_of_q": sp.sstr(nu_of_q),
            "chi_of_q": sp.sstr(chi_of_q),
            "chi_of_nu": "sqrt(2)*nu",
            "q_of_nu_exact_small_root": "(1-sqrt(1-4*nu^2))/(2*nu)",
            "valid_real_subextremal_branch": "0<=nu<1/2, 0<=q<1",
        },
        "series_about_zero_spin": {
            "q_of_nu_through_nu11": sp.sstr(q_of_nu),
            "chi_of_nu_exact": "sqrt(2)*nu",
            "mu_over_rh2_of_chi_through_chi6":
                "1/2+chi^2/2+5*chi^4/8+7*chi^6/8",
            "q_minus_nu_order": "O(nu^3)",
        },
        "coupling_map": {
            "symtropy_action": "R + alpha_GB*L_GB/4",
            "ma_lu_action": "R + alpha_R*Riemann^2",
            "candidate_first_coupling_map": sp.sstr(
                sp.Eq(alpha_r, coupling_solution[0])
            ).replace(" ", ""),
            "scope": "First-order correction about a Ricci-flat Myers-Perry background only",
        },
        "checks": {
            "horizon_relation_exact": True,
            "angular_velocity_relation_exact": True,
            "printed_chi_relation_exact": True,
            "chi_equals_sqrt2_nu_exact": True,
            "inverse_series_exact_through_nu11": True,
            "eq_3_9_equal_spin_mass_series": True,
            "coupling_coefficient_identity": True,
        },
        "claim_ceiling": (
            "Exact kinematic parameter relations and a symbolic series inversion only. "
            "No corrected metric/gauge map, field-equation coefficient comparison, "
            "thermodynamic match, or finite-coupling EGB result is claimed."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = derive()
    result["source_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    result["environment"] = {
        "python": platform.python_version(),
        "sympy": sp.__version__,
    }
    rendered = json.dumps(result, indent=2, sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()

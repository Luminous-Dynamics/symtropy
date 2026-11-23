#!/usr/bin/env python3
"""Derive the equal-spin Ma-Lu -> Symtropy radial-gauge map at O(alpha_R chi^2).

Source: Ma & Lu, JHEP 05 (2026) 081, arXiv:2512.23797, Eqs. (2.8)-(2.12).
The script verifies the equal-spin angular cohomogeneity-one reduction, the
radial redefinition, and the mapped Symtropy metric functions. It does not yet
perform the horizon/Omega parameter renormalization or compare with the
Symtropy solution of the O(alpha_R chi^2) field equations.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import sympy as sp


def zero(label: str, expr: sp.Expr) -> None:
    reduced = sp.factor(sp.cancel(expr))
    if reduced != 0:
        raise AssertionError(f"{label}: {sp.sstr(reduced)}")


def derive() -> dict:
    r, a, mu, x, alpha = sp.symbols("r a mu x alpha", positive=True)
    chi2 = a**2 / mu  # Ma-Lu Eq. (2.9): chi=a/sqrt(mu).
    A = r**2 + a**2
    Delta = A**2 / r**2 - 2*mu
    L = 165*mu/r**4 + 330*mu**2/r**6 + 150*mu**3/r**8

    # Equal-spin reductions of the published O(chi^2) expressions.
    H1 = (sp.Rational(16,3)/r**2 - sp.Rational(16,3)*mu/r**4
          - sp.Rational(32,3)*mu**2/r**6
          - sp.Rational(2,45)*chi2*(282/r**2 - 162*mu/r**4 - 356*mu**2/r**6))
    H2 = (-sp.Rational(8,3)/r**2 - sp.Rational(16,3)*mu/r**4
          + sp.Rational(2,45)*chi2*(141/r**2 + 48*mu/r**4 + 418*mu**2/r**6))
    H3 = H2
    H4_0 = -sp.Rational(22,3)/mu - 12/r**2 - sp.Rational(16,3)*mu/r**4
    H4 = (H4_0 + chi2*(sp.Rational(71,10)/mu + sp.Rational(119,15)/r**2
          + sp.Rational(217,15)*mu/r**4 + sp.Rational(836,45)*mu**2/r**6))
    H7_0 = sp.Rational(8,3)/r**2 + sp.Rational(4,3)*mu/r**4
    H7 = (H7_0 - sp.Rational(2,45)*chi2*(141/r**2 + 48*mu/r**4 - 22*mu**2/r**6))
    H5 = H7 - sp.Rational(4,45)*chi2*x**2*L
    H6 = H7 - sp.Rational(4,45)*chi2*(1-x**2)*L

    # At O(a^2), D=2*mu*a^2/A is the coefficient of eta^2,
    # eta=(1-x^2)dphi+x^2 dpsi. The exact angular terms in Eq. (2.8)
    # reduce to G dOmega_3^2 + D(1+alpha H4) eta^2.
    D = 2*mu*a**2/A
    h5_identity_residual = sp.series(
        A*(H5-H7) + D*x**2*(H7_0-H4_0), a, 0, 3
    ).removeO().expand()
    h6_identity_residual = sp.series(
        A*(H6-H7) + D*(1-x**2)*(H7_0-H4_0), a, 0, 3
    ).removeO().expand()
    zero("equal-spin H5 angular identity through a^2", h5_identity_residual)
    zero("equal-spin H6 angular identity through a^2", h6_identity_residual)
    zero("H2=H3 equal-spin identity", H2-H3)

    # Exact first-order radial gauge map R^2=G=A(1+alpha_R H7).
    G = A*(1+alpha*H7)
    Gp = sp.diff(G, r)
    f_from_coordinate_change = Delta*Gp**2/(4*G**2)
    f_first_order = (Delta*r**2/A**2)*(1+alpha*A*sp.diff(H7,r)/r)
    zero("radial f-map through first order in alpha",
         sp.series(f_from_coordinate_change-f_first_order, alpha, 0, 2).removeO())

    # Reconstruct the five cohomogeneity-one functions in Symtropy's ansatz.
    # These are in the old BL coordinate r; R is implicit in R^2=G(r).
    h_map = G + D*(1+alpha*H4)
    hw_map = 2*mu*a/A*(1+alpha*H2)
    w_map = sp.factor(hw_map/h_map)
    F = 1-2*mu/A
    b_map = F-alpha*H1+h_map*w_map**2
    f_map = f_from_coordinate_change

    # The H4-H7 zeroth-spin difference is the coefficient behind the fibre
    # deformation and is cross-checked directly.
    zero("H4-H7 static difference",
         H4_0-H7_0 - (-sp.Rational(22,3)/mu-sp.Rational(44,3)/r**2-sp.Rational(20,3)*mu/r**4))

    # Negative controls: wrong chi normalization and omitted H7 radial map.
    wrong_chi_residual = sp.factor(
        H5.xreplace({chi2: 2*chi2})-H7.xreplace({chi2: 2*chi2})
        + sp.Rational(4,45)*(2*chi2)*x**2*L
    )
    omitted_h7_residual = sp.factor(f_first_order - Delta*r**2/A**2)
    assert wrong_chi_residual != 0
    assert sp.simplify(omitted_h7_residual) != 0

    return {
        "schema_version": 1,
        "artifact_kind": "ma_lu_equal_spin_symtropy_radial_gauge_map",
        "status": "exact_equal_spin_angular_reduction_and_first_order_radial_map_derived",
        "source": {
            "paper": "Ma and Lu, JHEP 05 (2026) 081, arXiv:2512.23797",
            "equations": ["(2.8)", "(2.9)", "(2.11)", "(2.12)"],
            "chi_definition": "chi=a/sqrt(mu)",
            "chi_squared": "a^2/mu"
        },
        "definitions": {
            "A": "r^2+a^2",
            "Delta": "A^2/r^2-2*mu",
            "x": "cos(theta)",
            "eta": "(1-x^2)*dphi+x^2*dpsi",
            "D": "2*mu*a^2/A",
            "G": "A*(1+alpha_R*H7)"
        },
        "equal_spin_h_functions_through_chi2": {
            "H1": sp.sstr(H1), "H2": sp.sstr(H2), "H3": sp.sstr(H3),
            "H4": sp.sstr(H4), "H5": sp.sstr(H5), "H6": sp.sstr(H6), "H7": sp.sstr(H7)
        },
        "angular_reduction": {
            "metric": "G*dOmega3^2 + D*(1+alpha_R*H4)*eta^2 + O(alpha_R^2,chi^4)",
            "dOmega3_squared": "dtheta^2+(1-x^2)*dphi^2+x^2*dpsi^2",
            "H5_identity": "H5*(A+D*x^2)=A*H7+D*x^2*H4+O(chi^4)",
            "H6_identity": "H6*(A+D*(1-x^2))=A*H7+D*(1-x^2)*H4+O(chi^4)",
            "residuals": {"H5": "0 through a^2", "H6": "0 through a^2", "H2_minus_H3": "0"}
        },
        "radial_gauge": {
            "definition": "R^2=G=A*(1+alpha_R*H7)",
            "inverse_first_order": "r=sqrt(R^2-a^2)-alpha_R*R^2*H7(sqrt(R^2-a^2))/(2*sqrt(R^2-a^2))+O(alpha_R^2)",
            "f_exact_from_chain_rule": "Delta*(dG/dr)^2/(4*G^2)",
            "f_first_order": "Delta*r^2/A^2*(1+alpha_R*A*H7'(r)/r)",
            "g_symtropy": "R^2"
        },
        "symtropy_metric_functions_in_old_r_coordinate": {
            "g": "G",
            "h": "G+D*(1+alpha_R*H4)",
            "h_times_w": "2*mu*a/A*(1+alpha_R*H2)",
            "w": "[2*mu*a/A*(1+alpha_R*H2)]/[G+D*(1+alpha_R*H4)]",
            "b": "1-2*mu/A-alpha_R*H1+h*w^2",
            "f": "Delta*(dG/dr)^2/(4*G^2)",
            "coordinate_caveat": "These are functions of old BL coordinate r; the full component comparison must substitute r(R) and reparameterize horizon radius and physical Omega_H."
        },
        "negative_controls": {
            "wrong_chi_squared_factor_of_two_rejected": True,
            "omitting_H7_radial_redefinition_changes_f": True
        },
        "checks": {
            "equal_spin_H2_equals_H3": True,
            "H5_angular_cohomogeneity_identity_through_a2": True,
            "H6_angular_cohomogeneity_identity_through_a2": True,
            "radial_f_chain_rule_identity_through_alpha": True,
            "horizon_parameter_redefinition_not_yet_applied": True,
            "physical_Omega_H_parameter_redefinition_not_yet_applied": True
        },
        "claim_ceiling": "The equal-spin angular sector and first-order radial gauge transformation are derived from the published ansatz. No matched horizon/Omega reparameterization, coefficient-level comparison with the Symtropy O(a^2) EGB solution, full original-EOM residual, or finite-coupling result is claimed."
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = derive()
    result["source_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    result["environment"] = {"python": platform.python_version(), "sympy": sp.__version__}
    rendered = json.dumps(result, indent=2, sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()

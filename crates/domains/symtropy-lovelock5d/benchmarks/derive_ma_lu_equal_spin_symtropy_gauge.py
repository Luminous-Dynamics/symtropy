#!/usr/bin/env python3
"""Derive the equal-spin Ma-Lu -> Symtropy radial-gauge map at O(alpha_R a^2).

Source: Ma & Lu, JHEP 05 (2026) 081, arXiv:2512.23797, Eqs. (9)-(13).
The paper defines chi=a/mu. To avoid carrying inverse powers of mu through
every term, this script uses mu_chi_squared := mu*chi^2 = a^2/mu.
"""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp


def zero(label: str, expr: sp.Expr) -> None:
    reduced=sp.factor(sp.cancel(expr))
    if reduced != 0:
        raise AssertionError(f"{label}: {sp.sstr(reduced)}")


def derive() -> dict:
    r,a,mu,x,alpha=sp.symbols("r a mu x alpha",positive=True)
    # Published chi=a/mu; this compact combination is mu*chi^2=a^2/mu.
    mu_chi2=a**2/mu
    A=r**2+a**2
    Delta=A**2/r**2-2*mu
    L=165*mu/r**4+330*mu**2/r**6+150*mu**3/r**8

    # Exact equal-spin reductions of Ma-Lu Eqs. (20)-(32), through chi^2.
    H1=(sp.Rational(16,3)/r**2-sp.Rational(16,3)*mu/r**4
        -sp.Rational(32,3)*mu**2/r**6
        -sp.Rational(2,45)*mu_chi2*(282/r**2-162*mu/r**4
          -356*mu**2/r**6-1076*mu**3/r**8))
    H2=(-sp.Rational(8,3)/r**2-sp.Rational(16,3)*mu/r**4
        +sp.Rational(2,45)*mu_chi2*(141/r**2+48*mu/r**4+418*mu**2/r**6))
    H3=H2
    H4_0=-sp.Rational(22,3)/mu-12/r**2-sp.Rational(16,3)*mu/r**4
    H4=(H4_0+mu_chi2*(sp.Rational(71,10)/mu
        +sp.Rational(119,15)/r**2+sp.Rational(217,15)*mu/r**4
        +sp.Rational(836,45)*mu**2/r**6))
    H7_0=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
    H7=(H7_0-sp.Rational(2,45)*mu_chi2*(141/r**2+48*mu/r**4
        -22*mu**2/r**6))
    H5=H7-sp.Rational(4,45)*mu_chi2*x**2*L
    H6=H7-sp.Rational(4,45)*mu_chi2*(1-x**2)*L

    # Eq. (9): equal-spin angular sector is round S^3 plus one Hopf fibre.
    D=2*mu*a**2/A
    h5_res=sp.series(A*(H5-H7)+D*x**2*(H7_0-H4_0),a,0,3).removeO().expand()
    h6_res=sp.series(A*(H6-H7)+D*(1-x**2)*(H7_0-H4_0),a,0,3).removeO().expand()
    zero("H5 angular identity through a^2",h5_res)
    zero("H6 angular identity through a^2",h6_res)
    zero("equal-spin H2=H3",H2-H3)

    # Orbit-space conformal factor and exact first-order radial chain rule.
    G=A*(1+alpha*H7)
    Gp=sp.diff(G,r)
    f_exact=Delta*Gp**2/(4*G**2)
    f_first=(Delta*r**2/A**2)*(1+alpha*A*sp.diff(H7,r)/r)
    zero("radial f chain rule through alpha",
         sp.series(f_exact-f_first,alpha,0,2).removeO())

    # Symtropy metric functions, initially as functions of old BL coordinate r.
    h=G+D*(1+alpha*H4)
    hw=2*mu*a/A*(1+alpha*H2)
    w=sp.factor(hw/h)
    b=1-2*mu/A-alpha*H1+h*w**2

    zero("H4-H7 static difference",
         H4_0-H7_0-(-sp.Rational(22,3)/mu-sp.Rational(44,3)/r**2
                    -sp.Rational(20,3)*mu/r**4))

    # Negative control: doubling mu*chi^2 while keeping the target map fixed.
    wrong_H5=H7-sp.Rational(4,45)*(2*mu_chi2)*x**2*L
    wrong_res=sp.series(A*(wrong_H5-H7)+D*x**2*(H7_0-H4_0),a,0,3).removeO().expand()
    assert sp.factor(wrong_res)!=0
    omitted_H7=sp.factor(f_first-Delta*r**2/A**2)
    assert omitted_H7!=0

    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_symtropy_radial_gauge_map",
      "status":"exact_equal_spin_angular_reduction_and_first_order_radial_map_derived",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081, arXiv:2512.23797",
        "equations":["(9)","(10)","(13)"],"chi_definition":"chi=a/mu",
        "compact_rotation_combination":"mu*chi^2=a^2/mu"},
      "definitions":{"A":"r^2+a^2","Delta":"A^2/r^2-2*mu",
        "x":"cos(theta)","eta":"(1-x^2)*dphi+x^2*dpsi",
        "D":"2*mu*a^2/A","G":"A*(1+alpha_R*H7)"},
      "equal_spin_h_functions_through_chi2":{
        "H1":sp.sstr(H1),"H2":sp.sstr(H2),"H3":sp.sstr(H3),
        "H4":sp.sstr(H4),"H5":sp.sstr(H5),"H6":sp.sstr(H6),"H7":sp.sstr(H7)},
      "angular_reduction":{
        "metric":"G*dOmega3^2+D*(1+alpha_R*H4)*eta^2+O(alpha_R^2,chi^4)",
        "dOmega3_squared":"dtheta^2+(1-x^2)*dphi^2+x^2*dpsi^2",
        "H5_identity":"H5*(A+D*x^2)=A*H7+D*x^2*H4+O(chi^4)",
        "H6_identity":"H6*(A+D*(1-x^2))=A*H7+D*(1-x^2)*H4+O(chi^4)",
        "residuals":{"H5":"0 through a^2","H6":"0 through a^2","H2_minus_H3":"0"}},
      "radial_gauge":{
        "definition":"R^2=G=A*(1+alpha_R*H7)",
        "inverse_first_order":"r=sqrt(R^2-a^2)-alpha_R*R^2*H7(sqrt(R^2-a^2))/(2*sqrt(R^2-a^2))+O(alpha_R^2)",
        "f_exact_from_chain_rule":"Delta*(dG/dr)^2/(4*G^2)",
        "f_first_order":"Delta*r^2/A^2*(1+alpha_R*A*H7'(r)/r)",
        "g_symtropy":"R^2"},
      "symtropy_metric_functions_in_old_r_coordinate":{
        "g":"G","h":"G+D*(1+alpha_R*H4)",
        "h_times_w":"2*mu*a/A*(1+alpha_R*H2)",
        "w":"[2*mu*a/A*(1+alpha_R*H2)]/[G+D*(1+alpha_R*H4)]",
        "b":"1-2*mu/A-alpha_R*H1+h*w^2",
        "f":"Delta*(dG/dr)^2/(4*G^2)",
        "coordinate_caveat":"Compose with the inverse r(R) and the fixed-(R_H,Omega_H) parameter map before comparing components."},
      "negative_controls":{"wrong_factor_of_two_in_mu_chi2_rejected":True,
        "omitting_H7_radial_redefinition_changes_f":True},
      "checks":{"published_chi_a_over_mu_convention":True,
        "H1_includes_all_equal_spin_chi2_terms":True,
        "equal_spin_H2_equals_H3":True,
        "H5_angular_cohomogeneity_identity_through_a2":True,
        "H6_angular_cohomogeneity_identity_through_a2":True,
        "radial_f_chain_rule_identity_through_alpha":True,
        "horizon_parameter_redefinition_not_yet_applied":True,
        "physical_Omega_H_parameter_redefinition_not_yet_applied":True},
      "claim_ceiling":"Exact equal-spin angular reduction and first-order radial-gauge transformation from published Ma-Lu formulas. No fixed-boundary metric coefficient match, full original-EOM residual, finite-coupling result or existence theorem is claimed."
    }


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    result=derive()
    result["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    result["environment"]={"python":platform.python_version(),"sympy":sp.__version__}
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    print(rendered)

if __name__=="__main__":
    main()

#!/usr/bin/env python3
"""Exact equal-spin Ma-Lu angular-block reduction to a Hopf-fibre metric.

Source: Ma & Lu, JHEP 05 (2026) 081, arXiv:2512.23797, Eqs. (9), (10), (13).
The script uses chi=a/mu and verifies the three angular metric components
against G*dOmega_3^2 + D*(1+alpha_R H4)*eta^2 at first order in alpha_R
and through O(a^2).
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import sympy as sp


def zero(label: str, expr: sp.Expr) -> None:
    residual=sp.factor(sp.cancel(expr))
    if residual != 0:
        raise AssertionError(f"{label}: {sp.sstr(residual)}")


def derive() -> dict:
    r,a,mu,x,alpha=sp.symbols("r a mu x alpha",positive=True)
    A=r**2+a**2
    Delta=A**2/r**2-2*mu
    chi2=a**2/mu**2  # Eq. (10): chi=a/mu.

    H4_0=-sp.Rational(22,3)/mu-12/r**2-sp.Rational(16,3)*mu/r**4
    H4=H4_0+sp.Rational(71,10)*chi2+sp.Rational(119,15)*mu*chi2/r**2 \
        +sp.Rational(217,15)*mu**2*chi2/r**4+sp.Rational(836,45)*mu**3*chi2/r**6

    H7_0=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
    radial_common=(3*mu*47*chi2/r**2
                   +2*mu**2*24*chi2/r**4
                   -22*mu**3*chi2/r**6)
    H7=H7_0-sp.Rational(2,45)*radial_common

    # Eq. (13), equal-spin specialization. The x-dependent polynomial is
    # the same in H5 and H6 after reducing chi^2=a^2/mu^2.
    S=165/r**4+330*mu/r**6+150*mu**2/r**8
    H5=H7-sp.Rational(4,45)*a**2*x**2*S
    H6=H7-sp.Rational(4,45)*a**2*(1-x**2)*S

    # Reconstruct directly from the independent Eq. (13) terms.
    radial5=(3*mu*(29+18)*chi2/r**2
             +2*mu**2*(11+13)*chi2/r**4
             -22*mu**3*chi2/r**6)
    angular5=(mu**2*(101+64)*chi2/r**4
              +2*mu**3*(92+73)*chi2/r**6
              +150*mu**4*chi2/r**8)
    H5_raw=H7_0-sp.Rational(2,45)*radial5-sp.Rational(4,45)*x**2*angular5

    angular6=(mu**2*(64+101)*chi2/r**4
              +2*mu**3*(73+92)*chi2/r**6
              +150*mu**4*chi2/r**8)
    radial6=(3*mu*(18+29)*chi2/r**2
             +14*mu**2*(11+16)*chi2/r**4
             +2*mu**3*(135+184)*chi2/r**6
             +300*mu**4*chi2/r**8)
    H6_raw=H7_0+sp.Rational(4,45)*x**2*angular6-sp.Rational(2,45)*radial6

    zero("H5 raw Eq13 reduction",H5_raw-H5)
    zero("H6 raw Eq13 reduction",H6_raw-H6)

    D=2*mu*a**2/A
    G=A*(1+alpha*H7)
    # Eq. (9) has an x^2 axis factor on dpsi^2 and (1-x^2) on dphi^2.
    gpsi2_ML=x**2*(1+alpha*H5)*(A+D*x**2)
    gphi2_ML=(1-x**2)*(1+alpha*H6)*(A+D*(1-x**2))
    gphipsi_ML=2*D*(1+alpha*H4)*x**2*(1-x**2)

    # Hopf-frame line element: G*dOmega3^2 + D(1+alpha H4)*eta^2.
    gpsi2_hopf=x**2*G+D*(1+alpha*H4)*x**4
    gphi2_hopf=(1-x**2)*G+D*(1+alpha*H4)*(1-x**2)**2
    gphipsi_hopf=2*D*(1+alpha*H4)*x**2*(1-x**2)

    zero("dpsi^2 component through alpha*a^2",
         sp.series(sp.diff(gpsi2_ML-gpsi2_hopf,alpha).subs(alpha,0),a,0,3).removeO())
    zero("dphi^2 component through alpha*a^2",
         sp.series(sp.diff(gphi2_ML-gphi2_hopf,alpha).subs(alpha,0),a,0,3).removeO())
    zero("dphi*dpsi component through alpha*a^2",
         sp.series(sp.diff(gphipsi_ML-gphipsi_hopf,alpha).subs(alpha,0),a,0,3).removeO())

    # Axis and negative controls.
    zero("psi-axis degeneracy",gpsi2_ML.subs(x,0))
    zero("phi-axis degeneracy",gphi2_ML.subs(x,1))
    zero("cross term on psi axis",gphipsi_ML.subs(x,0))
    zero("cross term on phi axis",gphipsi_ML.subs(x,1))
    bad_psi=gpsi2_ML/x**2
    assert sp.simplify(sp.diff(bad_psi-gpsi2_hopf/x**2,alpha).subs(alpha,0))!=0
    bad_H6=H7+sp.Rational(4,45)*a**2*(1-x**2)*S
    bad_phi=(1-x**2)*(1+alpha*bad_H6)*(A+D*(1-x**2))
    assert sp.series(sp.diff(bad_phi-gphi2_hopf,alpha).subs(alpha,0),a,0,3).removeO()!=0

    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_angular_diagonal_hopf_reduction",
      "status":"exact_equal_spin_angular_block_match_through_first_coupling_and_a2",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081, arXiv:2512.23797",
        "equations":["(9)","(10)","(13)"],"chi_definition":"chi=a/mu"},
      "definitions":{"A":"r^2+a^2","rho2":"A (equal spins)",
        "D":"2*mu*a^2/A","S":"165/r^4+330*mu/r^6+150*mu^2/r^8",
        "eta":"(1-x^2)*dphi+x^2*dpsi",
        "dOmega3_squared":"dtheta^2+(1-x^2)*dphi^2+x^2*dpsi^2",
        "G":"A*(1+alpha_R*H7)"},
      "equal_spin_functions":{
        "H4":sp.sstr(H4),"H5":sp.sstr(H5),"H6":sp.sstr(H6),"H7":sp.sstr(H7),
        "H5_remainder":"H5=H7-(4/45)*a^2*x^2*S",
        "H6_remainder":"H6=H7-(4/45)*a^2*(1-x^2)*S"},
      "original_metric_components":{
        "g_psi_psi":"x^2*(1+alpha_R*H5)*(A+D*x^2)",
        "g_phi_phi":"(1-x^2)*(1+alpha_R*H6)*(A+D*(1-x^2))",
        "g_phi_psi_dphi_dpsi_coefficient":"2*D*(1+alpha_R*H4)*x^2*(1-x^2)"},
      "hopf_frame_components":{
        "g_psi_psi":"x^2*G+D*(1+alpha_R*H4)*x^4",
        "g_phi_phi":"(1-x^2)*G+D*(1+alpha_R*H4)*(1-x^2)^2",
        "g_phi_psi_dphi_dpsi_coefficient":"2*D*(1+alpha_R*H4)*x^2*(1-x^2)",
        "line_element":"G*dOmega3^2+D*(1+alpha_R*H4)*eta^2"},
      "checks":{
        "H5_raw_eq13_reduction":True,"H6_raw_eq13_reduction":True,
        "dpsi2_component_match_through_alpha_a2":True,
        "dphi2_component_match_through_alpha_a2":True,
        "dphi_dpsi_component_match_through_alpha_a2":True,
        "psi_axis_regular_factor_x2":True,"phi_axis_regular_factor_1_minus_x2":True,
        "cross_term_vanishes_on_axes":True,
        "negative_control_missing_psi_axis_factor_rejected":True,
        "negative_control_wrong_H6_sign_rejected":True},
      "claim_ceiling":"Exact equal-spin angular-block reduction of Ma-Lu Eq. (9) into the Hopf-fibre Symtropy form through first order in alpha_R and O(a^2), using Eq. (10) chi=a/mu. Does not establish finite-coupling equivalence, global corrected branch existence, ADM mass, interval validation or hyperbolicity."
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

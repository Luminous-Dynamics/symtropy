#!/usr/bin/env python3
"""Verify the equal-spin angular block from the JHEP version of record.

Canonical source: Ma & Lu, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081,
Eqs. (2.8), (2.9), and (2.12). The version-of-record convention is
chi=a/sqrt(mu), so chi^2=a^2/mu. This supersedes the earlier arXiv-v1
transcription and all a/mu variants.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import sympy as sp


def zero(label: str, expr: sp.Expr) -> None:
    value=sp.factor(sp.cancel(expr))
    if value != 0:
        raise AssertionError(f"{label}: {sp.sstr(value)}")


def derive() -> dict:
    r,a,mu,x,alpha=sp.symbols("r a mu x alpha",positive=True)
    A=r**2+a**2
    chi2=a**2/mu

    # Equal-spin specialization of the publisher's Eq. (2.12).
    H4=(-sp.Rational(22,3)/mu-12/r**2-sp.Rational(16,3)*mu/r**4
        +chi2*(sp.Rational(71,10)/mu+sp.Rational(119,15)/r**2
        +sp.Rational(217,15)*mu/r**4+sp.Rational(836,45)*mu**2/r**6))
    H7=(sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
        -sp.Rational(2,45)*chi2*(141/r**2+48*mu/r**4-22*mu**2/r**6))
    S=165/r**4+330*mu/r**6+150*mu**2/r**8
    H5=H7-sp.Rational(4,45)*a**2*x**2*S
    H6=H7-sp.Rational(4,45)*a**2*(1-x**2)*S

    # Direct independent transcription of Eq. (2.12), after imposing a=b.
    H5_raw=(sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
        -sp.Rational(2,45)*chi2*(141/r**2+48*mu/r**4-22*mu**2/r**6)
        -sp.Rational(4,45)*a**2*x**2*S)
    H6_raw=(sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
        +sp.Rational(4,45)*a**2*x**2*S
        -sp.Rational(2,45)*chi2*(141/r**2+378*mu/r**4
          +638*mu**2/r**6+300*mu**3/r**8))
    zero("H5 direct Eq. (2.12) reduction",H5_raw-H5)
    zero("H6 direct Eq. (2.12) reduction",H6_raw-H6)

    D=2*mu*a**2/A
    G=A*(1+alpha*H7)

    # Full angular components from VOR Eq. (2.8), including the axis factors.
    ml_psi=x**2*(1+alpha*H5)*(A+D*x**2)
    ml_phi=(1-x**2)*(1+alpha*H6)*(A+D*(1-x**2))
    ml_cross=2*D*(1+alpha*H4)*x**2*(1-x**2)

    # Hopf-fibre form of the same block.
    hopf_psi=x**2*G+D*(1+alpha*H4)*x**4
    hopf_phi=(1-x**2)*G+D*(1+alpha*H4)*(1-x**2)**2
    hopf_cross=2*D*(1+alpha*H4)*x**2*(1-x**2)

    for name,delta in [
        ("dpsi^2",ml_psi-hopf_psi),
        ("dphi^2",ml_phi-hopf_phi),
        ("dphi*dpsi",ml_cross-hopf_cross)
    ]:
        zero(name+" component through O(alpha*a^2)",
             sp.series(sp.diff(delta,alpha).subs(alpha,0),a,0,3).removeO())

    # Axis regularity is checked in the un-divided components.
    zero("psi-axis x=0",ml_psi.subs(x,0))
    zero("phi-axis x=1",ml_phi.subs(x,1))
    zero("cross on psi axis",ml_cross.subs(x,0))
    zero("cross on phi axis",ml_cross.subs(x,1))

    # Negative controls: wrong chi normalization, missing x^2, and wrong H6 sign.
    wrong_chi2=a**2/mu**2
    wrong_H5=H7.subs(chi2,wrong_chi2)-sp.Rational(4,45)*a**2*x**2*S
    if sp.factor(wrong_H5-H5)==0:
        raise AssertionError("negative control failed: chi=a/mu normalization accepted")
    bad_psi=(1+alpha*H5)*(A+D*x**2)
    if sp.diff(bad_psi-hopf_psi/x**2,alpha).subs(alpha,0)==0:
        raise AssertionError("negative control failed: missing x^2 axis factor accepted")
    bad_H6=H7+sp.Rational(4,45)*a**2*(1-x**2)*S
    bad_phi=(1-x**2)*(1+alpha*bad_H6)*(A+D*(1-x**2))
    if sp.series(sp.diff(bad_phi-hopf_phi,alpha).subs(alpha,0),a,0,3).removeO()==0:
        raise AssertionError("negative control failed: wrong H6 sign accepted")

    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_angular_diagonal_hopf_reduction",
      "status":"exact_version_of_record_angular_block_match_through_first_coupling_and_a2",
      "source":{
        "paper":"Ma and Lu, JHEP 05 (2026) 081",
        "doi":"10.1007/JHEP05(2026)081",
        "version":"JHEP version of record",
        "published":"2026-05-08",
        "equations":["(2.8)","(2.9)","(2.12)"],
        "chi_definition":"chi=a/sqrt(mu)",
        "chi_squared":"a^2/mu"
      },
      "definitions":{
        "A":"r^2+a^2",
        "D":"2*mu*a^2/A",
        "S":"165/r^4+330*mu/r^6+150*mu^2/r^8",
        "eta":"(1-x^2)*dphi+x^2*dpsi",
        "dOmega3_squared":"dtheta^2+(1-x^2)*dphi^2+x^2*dpsi^2",
        "G":"A*(1+alpha_R*H7)"
      },
      "equal_spin_functions":{
        "H4":sp.sstr(H4),
        "H5":sp.sstr(H5),
        "H6":sp.sstr(H6),
        "H7":sp.sstr(H7)
      },
      "angular_block":{
        "line_element":"G*dOmega3^2+D*(1+alpha_R*H4)*eta^2+O(alpha_R^2,a^4)",
        "components":{
          "dpsi2":"x^2*G+D*(1+alpha_R*H4)*x^4",
          "dphi2":"(1-x^2)*G+D*(1+alpha_R*H4)*(1-x^2)^2",
          "dphi_dpsi":"2*D*(1+alpha_R*H4)*x^2*(1-x^2)"
        }
      },
      "checks":{
        "H5_direct_Eq2_12":True,
        "H6_direct_Eq2_12":True,
        "dpsi2_component_match":True,
        "dphi2_component_match":True,
        "dphi_dpsi_component_match":True,
        "both_axis_factors_regular":True,
        "negative_control_wrong_chi_normalization":True,
        "negative_control_missing_x2_axis_factor":True,
        "negative_control_wrong_H6_sign":True
      },
      "claim_ceiling":"Exact equal-spin angular-block reduction of the JHEP version-of-record metric through first order in alpha_R and O(a^2). Does not imply finite-coupling equivalence, global branch existence, interval validation, or hyperbolicity."
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

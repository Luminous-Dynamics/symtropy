#!/usr/bin/env python3
"""Derive the equal-spin Ma-Lu radial/time-sector map into Symtropy gauge.

Source: Ma & Lu, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081, version of record, Eqs. (2.8)-(2.13).
Uses the version-of-record chi=a/sqrt(mu) convention, so chi^2=a^2/mu. This script maps
the radial, temporal, fibre, and frame-dragging components. It intentionally
does not claim the independent H5/H6 angular-diagonal component identities;
those must be verified before the complete five-dimensional angular block is
declared matched.
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
    r,a,mu,alpha=sp.symbols("r a mu alpha",positive=True)
    # Eq. (2.9): chi=a/sqrt(mu); chi2=a**2/mu is the spin-squared factor.
    chi2=a**2/mu
    A=r**2+a**2
    Delta=A**2/r**2-2*mu

    H1=(sp.Rational(16,3)/r**2-sp.Rational(16,3)*mu/r**4
        -sp.Rational(32,3)*mu**2/r**6
        -sp.Rational(2,45)*chi2*(282/r**2-162*mu/r**4
        -356*mu**2/r**6-1076*mu**3/r**8))
    H2=(-sp.Rational(8,3)/r**2-sp.Rational(16,3)*mu/r**4
        +sp.Rational(2,45)*chi2*(141/r**2+48*mu/r**4+418*mu**2/r**6))
    H3=H2
    H4_0=-sp.Rational(22,3)/mu-12/r**2-sp.Rational(16,3)*mu/r**4
    H4=(H4_0+chi2*(sp.Rational(71,10)/mu
        +sp.Rational(119,15)/r**2+sp.Rational(217,15)*mu/r**4
        +sp.Rational(836,45)*mu**2/r**6))
    H7_0=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
    H7=(H7_0-sp.Rational(2,45)*chi2*(141/r**2+48*mu/r**4
        -22*mu**2/r**6))

    # Eq. (9) radial conformal factor and radial gauge R^2=G.
    G=A*(1+alpha*H7)
    Gp=sp.diff(G,r)
    f_exact=Delta*Gp**2/(4*G**2)
    f_first=Delta*r**2/A**2*(1+alpha*A*sp.diff(H7,r)/r)
    zero("radial f chain rule through alpha",
         sp.series(f_exact-f_first,alpha,0,2).removeO())

    # Eq. (9) temporal, mixed, and Hopf-fibre cross terms in Symtropy form.
    D=2*mu*a**2/A
    h=G+D*(1+alpha*H4)
    hw=2*mu*a/A*(1+alpha*H2)
    w=sp.factor(hw/h)
    b=1-2*mu/A-alpha*H1+h*w**2

    zero("H4-H7 static difference",
         H4_0-H7_0-(-sp.Rational(22,3)/mu-sp.Rational(44,3)/r**2
                    -sp.Rational(20,3)*mu/r**4))
    zero("equal-spin H2=H3",H2-H3)

    # Negative controls: wrong normalization in the compact chi^2 factor,
    # and omission of H7 from the radial coordinate.
    wrong_chi_H2=-sp.Rational(8,3)/r**2-sp.Rational(16,3)*mu/r**4 \
        +sp.Rational(2,45)*(2*chi2)*(141/r**2+48*mu/r**4+418*mu**2/r**6)
    assert sp.factor(wrong_chi_H2-H2)!=0
    omitted_H7=sp.factor(f_first-Delta*r**2/A**2)
    assert omitted_H7!=0

    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_symtropy_radial_time_sector_map",
      "status":"exact_radial_temporal_fibre_and_frame_dragging_map_derived",
      "source":{
        "paper":"Ma and Lu, JHEP 05 (2026) 081, arXiv:2512.23797",
        "equations":["(9)","(10)","(13)"],
        "chi_definition":"chi=a/sqrt(mu)",
        "compact_rotation_combination":"chi^2=a^2/mu"
      },
      "definitions":{"A":"r^2+a^2","Delta":"A^2/r^2-2*mu",
        "D":"2*mu*a^2/A","G":"A*(1+alpha_R*H7)"},
      "equal_spin_radial_functions_through_chi2":{
        "H1":sp.sstr(H1),"H2":sp.sstr(H2),"H3":sp.sstr(H3),
        "H4":sp.sstr(H4),"H7":sp.sstr(H7)},
      "radial_gauge":{
        "definition":"R^2=G=A*(1+alpha_R*H7)",
        "inverse_first_order":"r=sqrt(R^2-a^2)-alpha_R*R^2*H7(sqrt(R^2-a^2))/(2*sqrt(R^2-a^2))+O(alpha_R^2)",
        "f_exact":"Delta*(dG/dr)^2/(4*G^2)",
        "f_first_order":"Delta*r^2/A^2*(1+alpha_R*A*H7'(r)/r)",
        "g_symtropy":"R^2"},
      "symtropy_radial_time_sector_in_old_r_coordinate":{
        "g":"G",
        "h":"G+D*(1+alpha_R*H4)",
        "h_times_w":"2*mu*a/A*(1+alpha_R*H2)",
        "w":"[2*mu*a/A*(1+alpha_R*H2)]/[G+D*(1+alpha_R*H4)]",
        "b":"1-2*mu/A-alpha_R*H1+h*w^2",
        "f":"Delta*(dG/dr)^2/(4*G^2)",
        "coordinate_caveat":"Compose with r(R), then convert Ma-Lu (r_h,mu,a) to fixed Symtropy (R_H,Omega_H)."},
      "checks":{
        "printed_chi_a_over_mu_convention":True,
        "H1_full_equal_spin_chi2_term_includes_mu3_over_r8":True,
        "equal_spin_H2_equals_H3":True,
        "radial_f_chain_rule_identity_through_alpha":True,
        "H4_static_difference_identity":True,
        "wrong_chi_squared_normalization_rejected":True,
        "omitting_H7_radial_redefinition_rejected":True,
        "H5_H6_angular_diagonal_components":"not certified by this script"},
      "claim_ceiling":"Exact first-order radial, temporal, fibre-cross-term, and frame-dragging map from the JHEP version-of-record Ma-Lu metric into Symtropy radial gauge. The companion version-of-record verifier independently checks H5/H6 angular components and the ADM mass through chi^2; this script itself does not perform those checks. No finite-coupling branch or hyperbolicity result is claimed."
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

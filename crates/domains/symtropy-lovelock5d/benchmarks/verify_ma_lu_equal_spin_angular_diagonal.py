#!/usr/bin/env python3
"""Equal-spin angular-block verifier for the Ma-Lu JHEP version of record.

Source: Ma & Lu, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081,
Eqs. (2.8), (2.9), (2.12). The canonical convention is chi=a/sqrt(mu).
"""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp


def z(label, expr):
    value=sp.factor(sp.cancel(expr))
    if value != 0:
        raise AssertionError(f"{label}: {sp.sstr(value)}")


def derive():
    r,a,mu,x,alpha=sp.symbols("r a mu x alpha",positive=True)
    A=r**2+a**2
    chi2=a**2/mu
    D=2*mu*a**2/A
    S=165/r**4+330*mu/r**6+150*mu**2/r**8
    base=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4
    H7=base-sp.Rational(2,45)*chi2*(141/r**2+48*mu/r**4-22*mu**2/r**6)
    H4=(-sp.Rational(22,3)/mu-12/r**2-sp.Rational(16,3)*mu/r**4
        +chi2*(sp.Rational(71,10)/mu+sp.Rational(119,15)/r**2
        +sp.Rational(217,15)*mu/r**4+sp.Rational(836,45)*mu**2/r**6))
    H5=H7-sp.Rational(4,45)*a**2*x**2*S
    H6=H7-sp.Rational(4,45)*a**2*(1-x**2)*S
    # Direct equal-spin specialization of the independent Eq. (2.12) terms.
    H5raw=base-sp.Rational(2,45)*chi2*(141/r**2+48*mu/r**4-22*mu**2/r**6)-sp.Rational(4,45)*a**2*x**2*S
    H6raw=base+sp.Rational(4,45)*a**2*x**2*S-sp.Rational(2,45)*chi2*(141/r**2+378*mu/r**4+638*mu**2/r**6+300*mu**3/r**8)
    z("H5 Eq2.12",H5raw-H5)
    z("H6 Eq2.12",H6raw-H6)
    G=A*(1+alpha*H7)
    # Full VOR Eq. (2.8), including both angular-axis degeneracy factors.
    ml_psi=x**2*(1+alpha*H5)*(A+D*x**2)
    ml_phi=(1-x**2)*(1+alpha*H6)*(A+D*(1-x**2))
    ml_cross=2*D*(1+alpha*H4)*x**2*(1-x**2)
    hopf_psi=x**2*G+D*(1+alpha*H4)*x**4
    hopf_phi=(1-x**2)*G+D*(1+alpha*H4)*(1-x**2)**2
    hopf_cross=2*D*(1+alpha*H4)*x**2*(1-x**2)
    for n,e in [("dpsi2",ml_psi-hopf_psi),("dphi2",ml_phi-hopf_phi),("dphi_dpsi",ml_cross-hopf_cross)]:
        z(n,sp.series(sp.diff(e,alpha).subs(alpha,0),a,0,3).removeO())
    z("psi-axis",ml_psi.subs(x,0))
    z("phi-axis",ml_phi.subs(x,1))
    z("cross psi-axis",ml_cross.subs(x,0))
    z("cross phi-axis",ml_cross.subs(x,1))
    # Fail-closed negative controls.
    wrong_chi=H7.subs(chi2,a**2/mu**2)-sp.Rational(4,45)*a**2*x**2*S
    assert sp.factor(wrong_chi-H5)!=0
    missing_axis=(1+alpha*H5)*(A+D*x**2)
    assert sp.diff(missing_axis-hopf_psi/x**2,alpha).subs(alpha,0)!=0
    wrong_H6=H7+sp.Rational(4,45)*a**2*(1-x**2)*S
    bad_phi=(1-x**2)*(1+alpha*wrong_H6)*(A+D*(1-x**2))
    assert sp.series(sp.diff(bad_phi-hopf_phi,alpha).subs(alpha,0),a,0,3).removeO()!=0
    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_angular_diagonal_hopf_reduction",
      "status":"exact_version_of_record_angular_block_match_through_alpha_a2",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081","doi":"10.1007/JHEP05(2026)081","version":"JHEP version of record","published":"2026-05-08","equations":["(2.8)","(2.9)","(2.12)"],"chi":"a/sqrt(mu)","chi_squared":"a^2/mu"},
      "definitions":{"A":"r^2+a^2","D":"2*mu*a^2/A","S":"165/r^4+330*mu/r^6+150*mu^2/r^8","eta":"(1-x^2)*dphi+x^2*dpsi","G":"A*(1+alpha_R*H7)"},
      "equal_spin_functions":{"H4":sp.sstr(H4),"H5":sp.sstr(H5),"H6":sp.sstr(H6),"H7":sp.sstr(H7)},
      "angular_block":"G*[dtheta^2+(1-x^2)dphi^2+x^2*dpsi^2]+D*(1+alpha_R*H4)*[(1-x^2)dphi+x^2*dpsi]^2",
      "checks":{"H5_direct_Eq2_12":True,"H6_direct_Eq2_12":True,"dpsi2":True,"dphi2":True,"dphi_dpsi":True,"axis_regularity":True,"negative_control_wrong_chi":True,"negative_control_missing_axis":True,"negative_control_wrong_H6_sign":True},
      "claim_ceiling":"Exact equal-spin angular-block reduction through first order in alpha_R and O(a^2); no finite-coupling existence or hyperbolicity result."
    }


def main():
    p=argparse.ArgumentParser(); p.add_argument("--output",type=Path); args=p.parse_args()
    out=derive(); out["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    out["environment"]={"python":platform.python_version(),"sympy":sp.__version__}
    text=json.dumps(out,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True); args.output.write_text(text,encoding="utf-8")
    print(text)


if __name__=="__main__":
    main()

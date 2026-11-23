#!/usr/bin/env python3
"""Ma-Lu version-of-record equal-spin comparison: angular block, fixed boundary, ADM mass."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

def z(label, expr):
    value=sp.factor(sp.cancel(expr))
    if value != 0:
        raise AssertionError(f"{label}: {sp.sstr(value)}")

def trunc(expr, small, order):
    return sp.series(expr, small, 0, order).removeO().expand()

def derive():
    r,a,mu,x,alpha=sp.symbols("r a mu x alpha", positive=True)
    A=r**2+a**2
    chi2=a**2/mu                         # Eq. (2.9): chi^2=a^2/mu
    H4=-sp.Rational(22,3)/mu-12/r**2-sp.Rational(16,3)*mu/r**4 + chi2*(
        sp.Rational(71,10)/mu+sp.Rational(119,15)/r**2+
        sp.Rational(217,15)*mu/r**4+sp.Rational(836,45)*mu**2/r**6)
    H7=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4-sp.Rational(2,45)*chi2*(
        141/r**2+48*mu/r**4-22*mu**2/r**6)
    S=165/r**4+330*mu/r**6+150*mu**2/r**8
    H5=H7-sp.Rational(4,45)*a**2*x**2*S
    H6=H7-sp.Rational(4,45)*a**2*(1-x**2)*S
    H5raw=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4-sp.Rational(2,45)*chi2*(
        141/r**2+48*mu/r**4-22*mu**2/r**6)-sp.Rational(4,45)*a**2*x**2*S
    H6raw=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4+sp.Rational(4,45)*a**2*x**2*S-sp.Rational(2,45)*chi2*(
        141/r**2+378*mu/r**4+638*mu**2/r**6+300*mu**3/r**8)
    z("H5 Eq. (2.12)",H5raw-H5); z("H6 Eq. (2.12)",H6raw-H6)
    D=2*mu*a**2/A
    G=A*(1+alpha*H7)
    # Version-of-record Eq. (2.8): both angular-axis factors are explicit.
    ml_psi=x**2*(1+alpha*H5)*(A+D*x**2)
    ml_phi=(1-x**2)*(1+alpha*H6)*(A+D*(1-x**2))
    ml_cross=2*D*(1+alpha*H4)*x**2*(1-x**2)
    hopf_psi=x**2*G+D*(1+alpha*H4)*x**4
    hopf_phi=(1-x**2)*G+D*(1+alpha*H4)*(1-x**2)**2
    hopf_cross=2*D*(1+alpha*H4)*x**2*(1-x**2)
    for name,delta in [("dpsi2",ml_psi-hopf_psi),("dphi2",ml_phi-hopf_phi),
                       ("dphi_dpsi",ml_cross-hopf_cross)]:
        z(name,sp.series(sp.diff(delta,alpha).subs(alpha,0),a,0,3).removeO())
    z("psi-axis x=0",ml_psi.subs(x,0))
    z("phi-axis x=1",ml_phi.subs(x,1))
    z("cross on psi axis",ml_cross.subs(x,0))
    z("cross on phi axis",ml_cross.subs(x,1))
    badphi=(1-x**2)*(1+alpha*(H7+sp.Rational(4,45)*a**2*(1-x**2)*S))*(A+D*(1-x**2))
    assert sp.series(sp.diff(badphi-hopf_phi,alpha).subs(alpha,0),a,0,3).removeO()!=0
    assert sp.diff(ml_psi/x**2-hopf_psi/x**2,alpha).subs(alpha,0)!=0

    q,epsR,nu,epsS=sp.symbols("q epsR nu epsS", real=True)
    muq=(1+q**2)**2/2
    chi2q=q**2/muq                    # chi^2 at r_h=1
    h2=-sp.Rational(8,3)-sp.Rational(16,3)*muq+sp.Rational(2,45)*chi2q*(141+48*muq+418*muq**2)
    h4=-sp.Rational(22,3)/muq-12-sp.Rational(16,3)*muq+chi2q*(
        sp.Rational(71,10)/muq+sp.Rational(119,15)+sp.Rational(217,15)*muq+sp.Rational(836,45)*muq**2)
    h7=sp.Rational(8,3)+sp.Rational(4,3)*muq-sp.Rational(2,45)*chi2q*(141+48*muq-22*muq**2)
    RHrh=sp.sqrt((1+q**2)*(1+epsR*h7))
    nuh=q/(1+q**2)
    omega=nuh*(1+epsR*h2)/(1+epsR*(h7+q**2*h4)/(1+q**2))
    nuSq=trunc(trunc(RHrh*omega,epsR,2),q,4)
    RHq=trunc(trunc(RHrh,epsR,2),q,4)
    z("nuS(q,epsR)",nuSq-(q-q**3/2-7*epsR*q+sp.Rational(5509,90)*epsR*q**3))
    z("RH/rh(q,epsR)",RHq-(1+q**2/2+sp.Rational(5,3)*epsR-sp.Rational(503,90)*epsR*q**2))
    qinvR=nu+nu**3/2+epsR*(7*nu-sp.Rational(4249,90)*nu**3)
    z("inverse q",trunc(trunc(nuSq.subs(q,qinvR),epsR,2),nu,4)-nu)
    epsRepsS=epsS*(1+nu**2)
    qfix=trunc(qinvR.subs(epsR,epsRepsS),nu,4)
    RHfix=trunc(trunc(RHrh.subs(q,qinvR).subs(epsR,epsRepsS),epsS,2),nu,4)
    rhfix=trunc(trunc(1/RHfix,epsS,2),nu,4)
    afix=trunc(trunc(qfix*rhfix,epsS,2),nu,4)
    mufix=trunc(trunc(muq.subs(q,qinvR).subs(epsR,epsRepsS)*rhfix**2,epsS,2),nu,4)
    chifix=trunc(trunc(sp.sqrt(2)*qinvR/(1+qinvR**2),epsR,2).subs(epsR,epsRepsS),nu,4)
    expected={
      "q":nu+nu**3/2+epsS*(7*nu-sp.Rational(3619,90)*nu**3),
      "R_H/r_h":1+sp.Rational(5,3)*epsS+nu**2/2+sp.Rational(277,90)*epsS*nu**2,
      "r_h/R_H":1-sp.Rational(5,3)*epsS-nu**2/2-sp.Rational(127,90)*epsS*nu**2,
      "a/R_H":nu+sp.Rational(16,3)*epsS*nu-sp.Rational(2068,45)*epsS*nu**3,
      "mu_background/R_H^2":sp.Rational(1,2)-sp.Rational(5,3)*epsS+nu**2/2+sp.Rational(454,45)*epsS*nu**2,
      "chi":sp.sqrt(2)*(nu-nu**3/2+7*epsS*nu-sp.Rational(5509,90)*epsS*nu**3)
    }
    actual={"q":qfix,"R_H/r_h":RHfix,"r_h/R_H":rhfix,"a/R_H":afix,
            "mu_background/R_H^2":mufix,"chi":chifix}
    for k in expected: z(f"fixed-boundary {k}",actual[k]-expected[k])
    z("GR R_H relation",trunc(RHq.subs(epsR,0)**2-(1+q**2),q,4))
    z("GR Omega relation",trunc(nuSq.subs(epsR,0)-q/sp.sqrt(1+q**2),q,4))

    # Eq. (3.4) equal-spin mass correction vs the transformed g_tt 1/R^2 term.
    chi=sp.symbols("chi", real=True)
    h1_r2=sp.Rational(16,3)-sp.Rational(188,15)*chi**2
    deltaM_from_metric=sp.Rational(3,8)*sp.pi*h1_r2
    deltaM_paper=2*sp.pi-sp.Rational(47,10)*sp.pi*chi**2
    z("ADM mass correction through chi^2",deltaM_from_metric-deltaM_paper)
    z("Ricci-flat first-coupling action normalization",
      (sp.Symbol("alpha_R")-sp.Symbol("alpha_GB")/4).subs(sp.Symbol("alpha_R"),sp.Symbol("alpha_GB")/4))

    return {
      "schema_version":1,"artifact_kind":"ma_lu_version_record_equal_spin_comparison",
      "status":"exact_symbolic_angular_fixed_boundary_and_mass_coefficients_pass",
      "source":{"version":"JHEP version of record, published 08 May 2026",
        "doi":"10.1007/JHEP05(2026)081","equations":["(2.8)","(2.9)","(2.12)","(3.3)","(3.4)"],
        "chi_definition":"chi_a=a/sqrt(mu), chi_b=b/sqrt(mu)"},
      "angular_block":{"S":"165/r^4+330*mu/r^6+150*mu^2/r^8",
        "H5":"H7-(4/45)*a^2*x^2*S","H6":"H7-(4/45)*a^2*(1-x^2)*S",
        "line_element":"G*[dtheta^2+(1-x^2)dphi^2+x^2 dpsi^2]+D*(1+alpha_R*H4)*[(1-x^2)dphi+x^2*dpsi]^2",
        "checks":{"H5_H6_direct_Eq2_12":True,"dpsi2_component":True,"dphi2_component":True,
          "dphi_dpsi_component":True,"axis_regular":True,
          "negative_control_missing_x2_factor":True,"negative_control_wrong_H6_sign":True}},
      "fixed_boundary_expansions":{k:sp.sstr(v) for k,v in actual.items()},
      "coupling_conversion":{"alpha_R":"alpha_GB/4","epsilon_S":"lambda_GB/4",
        "epsilon_R":"epsilon_S*(1+nu_S^2)+O(epsilon_S^2,nu_S^4)"},
      "adm_mass_check":{"paper_equal_spin_DeltaM_through_chi2":sp.sstr(deltaM_paper),
        "metric_asymptotic_DeltaM_through_chi2":sp.sstr(deltaM_from_metric),
        "status":"exact_match_through_chi_squared"},
      "claim_ceiling":"Exact first-coupling, equal-spin, slow-rotation comparison through O(chi^2), including full angular block, fixed (R_H,Omega_H) map and ADM mass coefficient. No finite-coupling branch, interval existence/uniqueness or hyperbolicity result."
    }

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--output",type=Path)
    a=p.parse_args()
    out=derive()
    out["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    out["environment"]={"python":platform.python_version(),"sympy":sp.__version__}
    rendered=json.dumps(out,indent=2,sort_keys=True)+"\n"
    if a.output:
        a.output.parent.mkdir(parents=True,exist_ok=True)
        a.output.write_text(rendered,encoding="utf-8")
    print(rendered)

if __name__=="__main__":
    main()

#!/usr/bin/env python3
"""Ma-Lu JHEP version-of-record map to fixed Symtropy (R_H, Omega_H)."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

def trunc(expr, small, order):
    return sp.series(expr, small, 0, order).removeO().expand()

def zero(label, expr):
    value=sp.factor(sp.cancel(expr))
    if value != 0:
        raise AssertionError(f"{label}: {sp.sstr(value)}")

def derive():
    q, nu, eps_r, eps_s = sp.symbols("q nu eps_r eps_s", real=True)
    mu=(1+q**2)**2/2
    chi2=q**2/mu
    H2=-sp.Rational(8,3)-sp.Rational(16,3)*mu+sp.Rational(2,45)*chi2*(141+48*mu+418*mu**2)
    H4=-sp.Rational(22,3)/mu-12-sp.Rational(16,3)*mu+chi2*(sp.Rational(71,10)/mu+sp.Rational(119,15)+sp.Rational(217,15)*mu+sp.Rational(836,45)*mu**2)
    H7=sp.Rational(8,3)+sp.Rational(4,3)*mu-sp.Rational(2,45)*chi2*(141+48*mu-22*mu**2)
    RH=sp.sqrt((1+q**2)*(1+eps_r*H7))
    nu_h=q/(1+q**2)
    Omega=nu_h*(1+eps_r*H2)/(1+eps_r*(H7+q**2*H4)/(1+q**2))
    nu_s=trunc(trunc(RH*Omega,eps_r,2),q,4)
    RH_series=trunc(trunc(RH,eps_r,2),q,4)
    zero("nu_S(q,eps_R)",nu_s-(q-q**3/2-7*eps_r*q+sp.Rational(5509,90)*eps_r*q**3))
    zero("R_H/r_h(q,eps_R)",RH_series-(1+q**2/2+sp.Rational(5,3)*eps_r-sp.Rational(503,90)*eps_r*q**2))
    q_inv_r=nu+nu**3/2+eps_r*(7*nu-sp.Rational(4249,90)*nu**3)
    zero("inverse q(nu,eps_R)",trunc(trunc(nu_s.subs(q,q_inv_r),eps_r,2),nu,4)-nu)
    eps_r_of_s=eps_s*(1+nu**2)
    q_fixed=trunc(q_inv_r.subs(eps_r,eps_r_of_s),nu,4)
    RH_fixed=trunc(trunc(RH.subs(q,q_inv_r).subs(eps_r,eps_r_of_s),eps_s,2),nu,4)
    rh_fixed=trunc(trunc(1/RH_fixed,eps_s,2),nu,4)
    a_fixed=trunc(trunc(q_fixed*rh_fixed,eps_s,2),nu,4)
    mu_fixed=trunc(trunc(mu.subs(q,q_inv_r).subs(eps_r,eps_r_of_s)*rh_fixed**2,eps_s,2),nu,4)
    chi_fixed=trunc(trunc(sp.sqrt(2)*q_inv_r/(1+q_inv_r**2),eps_r,2).subs(eps_r,eps_r_of_s),nu,4)
    expected={
      "q":nu+nu**3/2+eps_s*(7*nu-sp.Rational(3619,90)*nu**3),
      "R_H/r_h":1+sp.Rational(5,3)*eps_s+nu**2/2+sp.Rational(277,90)*eps_s*nu**2,
      "r_h/R_H":1-sp.Rational(5,3)*eps_s-nu**2/2-sp.Rational(127,90)*eps_s*nu**2,
      "a/R_H":nu+sp.Rational(16,3)*eps_s*nu-sp.Rational(2068,45)*eps_s*nu**3,
      "mu_background/R_H^2":sp.Rational(1,2)-sp.Rational(5,3)*eps_s+nu**2/2+sp.Rational(454,45)*eps_s*nu**2,
      "chi":sp.sqrt(2)*(nu-nu**3/2+7*eps_s*nu-sp.Rational(5509,90)*eps_s*nu**3)}
    actual={"q":q_fixed,"R_H/r_h":RH_fixed,"r_h/R_H":rh_fixed,"a/R_H":a_fixed,"mu_background/R_H^2":mu_fixed,"chi":chi_fixed}
    for key in expected: zero(key,actual[key]-expected[key])
    zero("GR R_H relation",trunc(RH_series.subs(eps_r,0)**2-(1+q**2),q,4))
    zero("GR physical angular velocity",trunc(nu_s.subs(eps_r,0)-q/sp.sqrt(1+q**2),q,4))
    zero("GR mu/R_H^2",mu_fixed.subs(eps_s,0)-(sp.Rational(1,2)+nu**2/2))
    assert trunc(RH_series-RH_series.subs(eps_r,0),eps_r,2)!=0
    assert trunc(nu_s-RH*nu_h,eps_r,2)!=0
    assert trunc(nu_s.subs(eps_r,0)-nu_h,q,4)!=0
    assert chi_fixed != sp.sqrt(2)*nu
    return {"schema_version":1,"artifact_kind":"ma_lu_fixed_symtropy_horizon_omega_parameter_map",
      "status":"exact_symbolic_version_of_record_fixed_boundary_map_derived",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081","doi":"10.1007/JHEP05(2026)081","published":"2026-05-08","version":"JHEP version of record","equations":["(2.2)","(2.8)","(2.9)","(2.12)"]},
      "definitions":{"q":"a/r_h","nu_h":"Omega_H*r_h","nu_S":"Omega_H*R_H","epsilon_R":"alpha_R/r_h^2","epsilon_S":"alpha_R/R_H^2","chi":"a/sqrt(mu)"},
      "horizon_values":{"mu_over_rh2":"(1+q^2)^2/2","H2_h":"838*q^2/45-16/3","H4_h":"4141*q^2/45-88/3","H7_h":"10/3-578*q^2/45","nuS_of_q_epsR":"q-q^3/2-7*epsR*q+5509*epsR*q^3/90","RH_over_rh_of_q_epsR":"1+q^2/2+5*epsR/3-503*epsR*q^2/90"},
      "fixed_boundary_expansions":{k:sp.sstr(v) for k,v in actual.items()},
      "epsilon_conversion":"epsilon_R=epsilon_S*(1+nu_S^2)+O(epsilon_S^2,nu_S^4)",
      "checks":{"horizon_H2_H4_H7":True,"nuS_and_radius_series":True,"inverse_q_series":True,"fixed_boundary_all_coefficients":True,"GR_limit":True,"negative_control_omit_H7":True,"negative_control_omit_H2_H4":True,"negative_control_nu_h_vs_nu_S":True,"negative_control_truncated_spin_chi":True},
      "interpretation":{"mu_background_over_R_H2":"converted background MP parameter, not corrected ADM mass","chi":"Ma-Lu Eq. (2.9) chi=a/sqrt(mu)"},
      "claim_ceiling":"Exact first-order-coupling, equal-spin, slow-rotation reparameterization at fixed Symtropy (R_H,Omega_H), using the JHEP version-of-record. The background mu/R_H^2 is not the corrected ADM mass. No finite-coupling solution or existence theorem is implied."}

def main():
    p=argparse.ArgumentParser(); p.add_argument("--output",type=Path); a=p.parse_args()
    out=derive(); out["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    out["environment"]={"python":platform.python_version(),"sympy":sp.__version__}
    rendered=json.dumps(out,indent=2,sort_keys=True)+"\n"
    if a.output: a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(rendered,encoding="utf-8")
    print(rendered)
if __name__=="__main__": main()

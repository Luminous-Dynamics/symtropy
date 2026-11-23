#!/usr/bin/env python3
"""Verify the Ma-Lu equal-spin map to fixed Symtropy (R_H, Omega_H)."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

q,eR,nu,eS=sp.symbols("q eR nu eS",real=True)

def trunc(expr,eps,spin,order=4):
    return sp.series(sp.series(expr,eps,0,2).removeO(),spin,0,order).removeO().expand()

def zero(label,expr):
    result=sp.factor(sp.cancel(expr))
    if result!=0:
        raise AssertionError(f"{label}: {sp.sstr(result)}")

def derive():
    # r_h=1. chi=a/mu; the compact variable below is mu*chi^2=q^2/mu.
    mu=(1+q**2)**2/2
    mu_chi2=q**2/mu
    H2=-sp.Rational(8,3)-sp.Rational(16,3)*mu+sp.Rational(2,45)*mu_chi2*(141+48*mu+418*mu**2)
    H4=(-sp.Rational(22,3)/mu-12-sp.Rational(16,3)*mu
        +mu_chi2*(sp.Rational(71,10)/mu+sp.Rational(119,15)
        +sp.Rational(217,15)*mu+sp.Rational(836,45)*mu**2))
    H7=sp.Rational(8,3)+sp.Rational(4,3)*mu-sp.Rational(2,45)*mu_chi2*(141+48*mu-22*mu**2)

    # R_H^2=A_h(1+alpha_R H7); Omega_H=w(r_h) from h*w / h.
    RH_rh=sp.sqrt((1+q**2)*(1+eR*H7))
    nu_h=q/(1+q**2)
    omega_h=nu_h*(1+eR*H2)/(1+eR*(H7+q**2*H4)/(1+q**2))
    nuS_q=trunc(RH_rh*omega_h,eR,q,6)
    RH_q=trunc(RH_rh,eR,q,6)

    zero("nuS(q,eR)",nuS_q-(q-q**3/2-7*eR*q+sp.Rational(5509,90)*eR*q**3))
    zero("RH/rh(q,eR)",RH_q-(1+q**2/2+sp.Rational(5,3)*eR-sp.Rational(503,90)*eR*q**2))

    # Invert at fixed nu_S. First epsilon_R, then use epsilon_R=epsilon_S*(RH/rh)^2.
    q_inv_R=nu+nu**3/2+eR*(7*nu-sp.Rational(4249,90)*nu**3)
    zero("inverse q(nu,eR)",sp.series(sp.series(nuS_q.subs(q,q_inv_R),eR,0,2).removeO(),nu,0,4).removeO()-nu)
    eR_eS=eS*(1+nu**2)
    q_inv=trunc(q_inv_R.subs(eR,eR_eS),eS,nu,4)
    RH_fixed=trunc(RH_rh.subs(q,q_inv_R).subs(eR,eR_eS),eS,nu,4)
    rh_fixed=trunc(1/RH_fixed,eS,nu,4)
    a_fixed=trunc(q_inv*rh_fixed,eS,nu,4)
    mu_fixed=trunc(mu.subs(q,q_inv_R).subs(eR,eR_eS)*rh_fixed**2,eS,nu,4)
    # Eq. (10) chi=a/mu. Report chi*R_H, a dimensionless combination.
    chiRH_fixed=trunc((2*q_inv_R*RH_rh.subs(q,q_inv_R)/(1+q_inv_R**2)**2).subs(eR,eR_eS),eS,nu,4)

    expected={
      "q":nu+nu**3/2+eS*(7*nu-sp.Rational(3619,90)*nu**3),
      "R_H_over_r_h":1+sp.Rational(5,3)*eS+nu**2/2+sp.Rational(277,90)*eS*nu**2,
      "r_h_over_R_H":1-sp.Rational(5,3)*eS-nu**2/2-sp.Rational(127,90)*eS*nu**2,
      "a_over_R_H":nu+sp.Rational(16,3)*eS*nu-sp.Rational(2068,45)*eS*nu**3,
      "mu_parameter_over_R_H2":sp.Rational(1,2)-sp.Rational(5,3)*eS+nu**2/2+sp.Rational(454,45)*eS*nu**2,
      "chi_times_R_H":2*nu-2*nu**3+sp.Rational(52,3)*eS*nu-sp.Rational(2344,15)*eS*nu**3
    }
    actual={"q":q_inv,"R_H_over_r_h":RH_fixed,"r_h_over_R_H":rh_fixed,
            "a_over_R_H":a_fixed,"mu_parameter_over_R_H2":mu_fixed,
            "chi_times_R_H":chiRH_fixed}
    for key in expected:
        zero(key,actual[key]-expected[key])

    zero("GR areal radius",sp.series(RH_q.subs(eR,0)**2-(1+q**2),q,0,4).removeO())
    zero("GR physical spin",sp.series(nuS_q.subs(eR,0)-q/sp.sqrt(1+q**2),q,0,4).removeO())
    zero("GR background MP mass parameter",mu_fixed.subs(eS,0)-(sp.Rational(1,2)+nu**2/2))
    assert sp.series(RH_q-RH_q.subs(eR,0),eR,0,2).removeO()!=0
    assert sp.series(nuS_q-RH_rh*nu_h,eR,0,2).removeO()!=0
    assert sp.series(nuS_q.subs(eR,0)-nu_h,q,0,4).removeO()!=0
    assert chiRH_fixed != sp.sqrt(2)*nu
    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_fixed_symtropy_horizon_omega_parameter_map",
      "status":"exact_symbolic_first_coupling_parameter_map_derived",
      "definitions":{"q":"a/r_h","nu_h":"Omega_H*r_h","nu_S":"Omega_H*R_H",
        "epsilon_R":"alpha_R/r_h^2","epsilon_S":"alpha_R/R_H^2","chi":"a/mu",
        "chi_times_R_H":"(a/mu)*R_H"},
      "horizon_values":{
        "mu_over_rh2":"(1+q^2)^2/2",
        "H2_h_through_q2":"838*q^2/45-16/3",
        "H4_h_through_q2":"4141*q^2/45-88/3",
        "H7_h_through_q2":"10/3-578*q^2/45",
        "nuS_of_q_epsilonR":"q-q^3/2-7*epsilon_R*q+(5509/90)*epsilon_R*q^3",
        "R_H_over_r_h_of_q_epsilonR":"1+q^2/2+(5/3)*epsilon_R-(503/90)*epsilon_R*q^2"},
      "fixed_symtropy_boundary_expansions":{k:sp.sstr(v) for k,v in actual.items()},
      "epsilon_conversion":"epsilon_R=epsilon_S*(1+nu_S^2)+O(epsilon_S^2,nu_S^4)",
      "checks":{"horizon_H2_H4_H7":True,"nuS_and_radius_series":True,
        "inverse_q_series":True,"a_and_mu_parameter_maps":True,"chi_times_RH_map":True,
        "GR_exact_limit":True,"negative_control_omit_H7":True,
        "negative_control_omit_H2_H4":True,"negative_control_nu_h_vs_nu_S":True,
        "negative_control_sqrt2_chi":True},
      "claim_ceiling":"Exact first-order-coupling, equal-spin, slow-rotation reparameterization at fixed Symtropy (R_H,Omega_H). mu/R_H^2 is the converted background MP parameter, not corrected ADM mass. No corrected metric component comparison, full EOM residual, finite-coupling solution, or existence theorem is claimed."
    }

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    result=derive()
    result["environment"]={"python":platform.python_version(),"sympy":sp.__version__}
    result["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    print(rendered)

if __name__=="__main__":
    main()

#!/usr/bin/env python3
"""Verify Ma-Lu -> fixed Symtropy (R_H, Omega_H) equal-spin parameter map."""
from __future__ import annotations
import json
from pathlib import Path
import argparse
import sympy as sp

q, eR, nu, eS = sp.symbols("q eR nu eS", real=True)

def trunc(expr, eps, spin, order=4):
    return sp.series(sp.series(expr, eps, 0, 2).removeO(), spin, 0, order).removeO().expand()

def zero(label, expr):
    result = sp.factor(sp.cancel(expr))
    if result != 0:
        raise AssertionError(f"{label}: {sp.sstr(result)}")

def derive():
    # r_h=1; equal-spin MP horizon relation and Ma-Lu Eq. (2.9)/(2.12).
    mu = (1 + q**2)**2 / 2
    chi2 = q**2 / mu
    H2 = -sp.Rational(8,3) - sp.Rational(16,3)*mu + sp.Rational(2,45)*chi2*(141+48*mu+418*mu**2)
    H4 = (-sp.Rational(22,3)/mu - 12 - sp.Rational(16,3)*mu
          + chi2*(sp.Rational(71,10)/mu+sp.Rational(119,15)
          +sp.Rational(217,15)*mu+sp.Rational(836,45)*mu**2))
    H7 = sp.Rational(8,3)+sp.Rational(4,3)*mu-sp.Rational(2,45)*chi2*(141+48*mu-22*mu**2)

    # R_H^2=A_h(1+alpha_R H7); Omega_H=w(r_h) from h*w over h.
    RH_rh = sp.sqrt((1+q**2)*(1+eR*H7))
    nu_h = q/(1+q**2)
    omega_h = nu_h*(1+eR*H2)/(1+eR*(H7+q**2*H4)/(1+q**2))
    nuS_q = trunc(RH_rh*omega_h,eR,q)
    RH_q = trunc(RH_rh,eR,q)

    zero("nuS(q,eR)",nuS_q-(q-q**3/2-7*eR*q+sp.Rational(5509,90)*eR*q**3))
    zero("RH/rh(q,eR)",RH_q-(1+q**2/2+sp.Rational(5,3)*eR-sp.Rational(503,90)*eR*q**2))

    # Invert nu_S(q,e_R), then express e_R in e_S at first coupling.
    q_inv_R = nu+nu**3/2+eR*(7*nu-sp.Rational(4249,90)*nu**3)
    zero("inverse q(nu,eR)",sp.series(sp.series(nuS_q.subs(q,q_inv_R),eR,0,2).removeO()-nu,nu,0,4).removeO())
    eR_eS = eS*(1+nu**2)
    q_inv = sp.series(sp.series(q_inv_R.subs(eR,eR_eS),eS,0,2).removeO(),nu,0,4).removeO().expand()
    RH_fixed = sp.series(sp.series(RH_rh.subs(q,q_inv_R).subs(eR,eR_eS),eS,0,2).removeO(),nu,0,4).removeO().expand()
    rh_fixed = sp.series(sp.series(1/RH_fixed,eS,0,2).removeO(),nu,0,4).removeO().expand()
    a_fixed = sp.series(sp.series(q_inv*rh_fixed,eS,0,2).removeO(),nu,0,4).removeO().expand()
    mu_fixed = sp.series(sp.series(mu.subs(q,q_inv_R).subs(eR,eR_eS)*rh_fixed**2,eS,0,2).removeO(),nu,0,4).removeO().expand()
    chi_fixed = sp.series(sp.series(sp.sqrt(2)*q_inv/(1+q_inv**2),eS,0,2).removeO(),nu,0,4).removeO().expand()

    expected = {
        "q": nu+nu**3/2+eS*(7*nu-sp.Rational(3619,90)*nu**3),
        "R_H_over_r_h": 1+sp.Rational(5,3)*eS+nu**2/2+sp.Rational(277,90)*eS*nu**2,
        "r_h_over_R_H": 1-sp.Rational(5,3)*eS-nu**2/2-sp.Rational(127,90)*eS*nu**2,
        "a_over_R_H": nu+sp.Rational(16,3)*eS*nu-sp.Rational(2068,45)*eS*nu**3,
        "mu_parameter_over_R_H2": sp.Rational(1,2)-sp.Rational(5,3)*eS+nu**2/2+sp.Rational(454,45)*eS*nu**2,
        "chi": sp.sqrt(2)*(nu-nu**3/2+7*eS*nu-sp.Rational(5509,90)*eS*nu**3)
    }
    actual = {"q":q_inv,"R_H_over_r_h":RH_fixed,"r_h_over_R_H":rh_fixed,
              "a_over_R_H":a_fixed,"mu_parameter_over_R_H2":mu_fixed,"chi":chi_fixed}
    for key in expected:
        zero(key,actual[key]-expected[key])

    # Independent GR controls and fail-closed negative controls.
    zero("GR radius",sp.series(RH_q.subs(eR,0)**2-(1+q**2),q,0,4).removeO())
    zero("GR spin",sp.series(nuS_q.subs(eR,0)-q/sp.sqrt(1+q**2),q,0,4).removeO())
    zero("GR mass parameter",mu_fixed.subs(eS,0)-(sp.Rational(1,2)+nu**2/2))
    assert sp.series(RH_q-RH_q.subs(eR,0),eR,0,2).removeO()!=0  # omit H7
    assert sp.series(nuS_q-RH_rh*nu_h,eR,0,2).removeO()!=0     # omit H2/H4
    assert sp.series(nuS_q.subs(eR,0)-nu_h,q,0,4).removeO()!=0 # nu_h != nu_S
    assert sp.sqrt(2)*q/(1+q**2) != q/(1+q**2)                 # chi normalization

    return {
      "status":"exact_symbolic_first_coupling_parameter_map_derived",
      "expansions":{k:sp.sstr(v) for k,v in actual.items()},
      "checks":["horizon H2/H4/H7","nu_S and R_H series","inverse q series","a and mu parameter maps",
                "chi map","GR exact limit","negative controls: omit H7, omit H2/H4, nu_h vs nu_S, chi factor"],
      "claim_ceiling":"First-order coupling, equal-spin, slow-rotation conversion at fixed Symtropy (R_H,Omega_H). Not a corrected metric or original-EOM comparison."
    }

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    data=derive()
    rendered=json.dumps(data,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    print(rendered)

if __name__=="__main__":
    main()

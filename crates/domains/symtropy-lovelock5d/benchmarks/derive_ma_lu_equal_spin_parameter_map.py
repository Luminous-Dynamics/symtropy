#!/usr/bin/env python3
"""Exact equal-spin 5D Myers-Perry parameter map for Ma-Lu Eq. (10)."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

def zero(label: str, expr: sp.Expr) -> None:
    residual=sp.factor(sp.cancel(expr))
    if residual != 0:
        raise AssertionError(f"{label}: {sp.sstr(residual)}")

def derive() -> dict:
    q,nu,rh,y=sp.symbols("q nu r_h y",positive=True)
    alpha_gb,alpha_r,tensor=sp.symbols("alpha_GB alpha_R T")
    mu_over_rh2=(1+q**2)**2/2
    nu_of_q=q/(1+q**2)
    # Published Eq. (10): chi=a/mu (no square root).
    chi_rh_of_q=sp.factor(2*q/(1+q**2)**2)
    zero("MP horizon equation",(rh**2+(q*rh)**2)**2/rh**2-2*rh**2*mu_over_rh2)
    zero("printed chi=a/mu",(q*rh/rh**2/mu_over_rh2)*rh-chi_rh_of_q)
    coeffs=sp.symbols("c0:6")
    q_ansatz=sum(coeffs[n]*nu**(2*n+1) for n in range(6))
    residual=sp.series(q_ansatz/(1+q_ansatz**2)-nu,nu,0,12).removeO().expand()
    solution=sp.solve([residual.coeff(nu,2*n+1) for n in range(6)],coeffs,dict=True)
    if len(solution)!=1:
        raise AssertionError(f"inverse series not unique: {solution}")
    q_of_nu=sp.expand(q_ansatz.subs(solution[0]))
    zero("inverse q(nu) through nu^11",sp.series(q_of_nu/(1+q_of_nu**2)-nu,nu,0,12).removeO())
    chi_rh_of_nu=sp.series(2*q_of_nu/(1+q_of_nu**2)**2,nu,0,12).removeO().expand()
    zero("chi*r_h series through nu^11",sp.series(chi_rh_of_nu-(2*nu-2*nu**3-2*nu**5-4*nu**7-10*nu**9-28*nu**11),nu,0,12).removeO())
    # Eq. (50): y=chi*r_h; invert and reproduce mu/r_h^2.
    q_y=y/2+y**3/4+sp.Rational(9,32)*y**5
    zero("inverse q(y) through y^5",sp.series(2*q_y/(1+q_y**2)**2-y,y,0,6).removeO())
    mu_y=sp.series(mu_over_rh2.subs(q,q_y),y,0,6).removeO()
    zero("Eq. (50) equal-spin mass series",sp.series(mu_y-(sp.Rational(1,2)+y**2/4+sp.Rational(9,32)*y**4),y,0,6).removeO())
    coupling=sp.solve(sp.Eq(alpha_gb*tensor/4,alpha_r*tensor),alpha_r)
    if coupling != [alpha_gb/4]:
        raise AssertionError(f"coupling map: {coupling}")
    zero("first-coupling normalization",(alpha_r*tensor-alpha_gb*tensor/4).subs(alpha_r,coupling[0]))
    return {
      "schema_version":1,"artifact_kind":"ma_lu_equal_spin_parameter_map",
      "status":"exact_symbolic_parameter_relations_derived",
      "source_equations":{
        "ma_lu_eq_4":"Delta_r=((r^2+a^2)(r^2+b^2))/r^2-2*mu",
        "ma_lu_eq_6":"Omega_H=a/(r_h^2+a^2)",
        "ma_lu_eq_10":"chi_a=a/mu, chi_b=b/mu",
        "ma_lu_eq_50":"mu/r_h^2=1/2+(chi*r_h)^2/4+9*(chi*r_h)^4/32+O((chi*r_h)^6)"},
      "definitions":{"q":"a/r_h","nu_h":"Omega_H*r_h=q/(1+q^2)","chi":"a/mu",
                     "chi_times_r_h":"2*q/(1+q^2)^2","mu_over_rh2":"(1+q^2)^2/2"},
      "exact_equal_spin_relations":{
        "mu_over_rh2":sp.sstr(mu_over_rh2),"nu_h_of_q":sp.sstr(nu_of_q),
        "chi_times_rh_of_q":sp.sstr(chi_rh_of_q),
        "q_of_nu_exact_small_root":"(1-sqrt(1-4*nu_h^2))/(2*nu_h)",
        "valid_subextremal_branch":"0<=nu_h<1/2, 0<=q<1"},
      "series_about_zero_spin":{
        "q_of_nu_h_through_nu11":sp.sstr(q_of_nu),
        "chi_times_rh_of_nu_h_through_nu11":sp.sstr(chi_rh_of_nu),
        "q_minus_nu_h_order":"O(nu_h^3)","chi_rh_series_variable":"y=chi*r_h"},
      "coupling_map":{
        "symtropy_action":"R+alpha_GB*L_GB/4","ma_lu_action":"R+alpha_R*Riemann^2",
        "first_order_map":"alpha_R=alpha_GB/4","dimensionless_same_horizon":"alpha_R/r_h^2=lambda/4",
        "scope":"First-order correction about a Ricci-flat Myers-Perry background only"},
      "checks":{"horizon_relation_exact":True,"angular_velocity_relation_exact":True,
        "printed_chi_a_over_mu_exact":True,"inverse_q_series_exact_through_nu11":True,
        "chi_rh_series_exact_through_nu11":True,"eq50_equal_spin_mass_series_through_y4":True,
        "coupling_coefficient_identity":True},
      "claim_ceiling":"Exact kinematic parameter relations and first-coupling coefficient normalization only. No corrected metric/gauge comparison or finite-coupling EGB result is claimed."
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

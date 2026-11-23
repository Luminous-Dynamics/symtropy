#!/usr/bin/env python3
"""Ma-Lu JHEP VOR equal-spin thermodynamic charge series, through spin^10.

Checks Eq. (3.9) horizon mass-parameter conversion, Eq. (3.4) ADM/Komar
mass correction, the H1 asymptotic 1/r^2 coefficient through chi^2,
and transports mass/angular momentum to fixed Symtropy (R_H, Omega_H)
to the orders supported by the current H2/H4/H7 map.
"""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp


def zero(label: str, expr: sp.Expr) -> None:
    value=sp.factor(sp.cancel(expr))
    if value != 0:
        raise AssertionError(f"{label}: {sp.sstr(value)}")


def derive() -> dict:
    chi, nu, eps = sp.symbols("chi nu eps", positive=True)
    pi=sp.pi

    # Exact equal-spin MP kinematics from Eqs. (2.1), (2.2), (2.9).
    q_chi=(1-sp.sqrt(1-2*chi**2))/(sp.sqrt(2)*chi)
    mu_over_rh2=(1+q_chi**2)**2/2
    mu_series=sp.series(mu_over_rh2,chi,0,12).removeO().expand()
    mu_expected=(sp.Rational(1,2)+chi**2/2+sp.Rational(5,8)*chi**4
        +sp.Rational(7,8)*chi**6+sp.Rational(21,16)*chi**8
        +sp.Rational(33,16)*chi**10)
    zero("Eq. (3.9) equal-spin mass parameter through chi^10",mu_series-mu_expected)

    # Eq. (3.4): specialize the full two-spin Delta M polynomial to chi_a=chi_b=chi.
    deltaM=(
        2*pi
        -sp.Rational(47,20)*pi*(2*chi**2)
        +pi/sp.Integer(280)*(401+881+401)*chi**4
        -pi*(2*chi**2)/1680*(2753-1412+2753)*chi**4
        +pi/sp.Integer(12320)*(18927+25095-12202+25095+18927)*chi**8
        -pi*(2*chi**2)/320320*(508397-444516+593094-444516+508397)*chi**8
    )
    deltaM=sp.expand(deltaM)
    expected_deltaM=(2*pi-sp.Rational(47,10)*pi*chi**2
        +sp.Rational(1683,280)*pi*chi**4
        -sp.Rational(2047,420)*pi*chi**6
        +sp.Rational(37921,6160)*pi*chi**8
        -sp.Rational(90107,20020)*pi*chi**10)
    zero("Eq. (3.4) equal-spin DeltaM through chi^10",deltaM-expected_deltaM)

    # Eq. (2.12) H1 has the asymptotic form
    # H1=(16/3 - 188 chi^2/15)/r^2 + O(r^-4) for equal spin.
    h1_r2=sp.Rational(16,3)-sp.Rational(188,15)*chi**2
    deltaM_from_metric=sp.Rational(3,8)*pi*h1_r2
    zero("H1 asymptotic mass coefficient matches Eq. (3.4) through chi^2",
         deltaM_from_metric-(2*pi-sp.Rational(47,10)*pi*chi**2))

    # At zero coupling, nu_S=Omega_H R_H, chi=sqrt(2)*nu_h,
    # and nu_h=nu_S*sqrt(1-nu_S^2). In alpha*DeltaM, O(alpha)
    # corrections to chi would contribute only at O(alpha^2).
    chi_nu0=sp.sqrt(2)*nu*sp.sqrt(1-nu**2)
    deltaM_nu=sp.series(deltaM.subs(chi,chi_nu0),nu,0,12).removeO().expand()
    expected_nu=(2*pi-sp.Rational(47,5)*pi*nu**2
        +sp.Rational(2341,70)*pi*nu**4
        -sp.Rational(9143,105)*pi*nu**6
        +sp.Rational(184423,770)*pi*nu**8
        -sp.Rational(655638,1001)*pi*nu**10)
    zero("Eq. (3.4) DeltaM in fixed-boundary GR spin through nu^10",
         deltaM_nu-expected_nu)

    # The fixed-boundary H2/H4/H7 map currently gives the background MP
    # parameter through nu_S^2 at O(eps_S). The converted mu is NOT ADM M.
    mu_background=(1/(2*(1-nu**2))
        +eps*(-sp.Rational(5,3)+sp.Rational(454,45)*nu**2))
    # M/R_H^2 = (3pi/4)*mu_background/R_H^2 + eps*DeltaM(chi_0).
    # Only the total coefficient through nu^2 is fully controlled because
    # the O(eps*nu^4+) background parameter redefinition is not yet derived.
    M_total_to_nu2=sp.series(
        sp.Rational(3,4)*pi*mu_background+eps*deltaM_nu,nu,0,3
    ).removeO().expand()
    expected_M=(3*pi/(8*(1-nu**2))+eps*(sp.Rational(3,4)*pi-sp.Rational(11,6)*pi*nu**2))
    zero("fixed-boundary total ADM mass through eps*nu^2",
         sp.series(M_total_to_nu2-expected_M,nu,0,3).removeO())

    # Eq. (3.2): J_a=pi*mu*a/2 and is unchanged by alpha at fixed (mu,a).
    # Reparameterize with the verified fixed-(R_H,Omega_H) map.
    a_over_R=nu+eps*(sp.Rational(16,3)*nu-sp.Rational(2068,45)*nu**3)
    J_over_R3=sp.series(sp.Rational(1,2)*pi*mu_background*a_over_R,nu,0,4).removeO().expand()
    expected_J=pi*nu/(4*(1-nu**2))+eps*pi*(nu/2-sp.Rational(46,9)*nu**3)
    zero("fixed-boundary angular momentum through eps*nu^3",
         sp.series(J_over_R3-expected_J,nu,0,4).removeO())

    # Negative controls: drop sqrt(2) in chi(nu), or use H1 without
    # the equal-spin chi^2 term. Both must disagree with Eq. (3.4).
    wrong_chi_delta=sp.series(deltaM.subs(chi,nu*sp.sqrt(1-nu**2)),nu,0,6).removeO()
    if sp.factor(wrong_chi_delta-sp.series(deltaM_nu,nu,0,6).removeO())==0:
        raise AssertionError("negative control failed: missing sqrt(2) accepted")
    wrong_h1=sp.Rational(3,8)*pi*sp.Rational(16,3)
    if sp.simplify(wrong_h1-2*pi)==0:
        raise AssertionError("negative control failed: omitted H1 spin term accepted")

    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_thermodynamic_charge_series",
      "status":"exact_symbolic_equal_spin_mass_through_spin10_and_fixed_boundary_charges",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081",
        "doi":"10.1007/JHEP05(2026)081","version":"JHEP version of record",
        "published":"2026-05-08","equations":["(2.9)","(2.12)","(3.2)","(3.3)","(3.4)","(3.9)"]},
      "equal_spin_series":{
        "mu_over_rh2_from_Eq3_9":sp.sstr(mu_series),
        "DeltaM_Eq3_4_in_chi":sp.sstr(deltaM),
        "DeltaM_Eq3_4_in_nu_S_at_zero_coupling":sp.sstr(deltaM_nu),
        "H1_asymptotic_1_over_r2_coefficient":sp.sstr(h1_r2),
        "metric_ADM_mass_prefactor":"3*pi/8",
        "metric_DeltaM_from_H1_through_chi2":sp.sstr(deltaM_from_metric)},
      "fixed_boundary_charges":{
        "definitions":{"nu_S":"Omega_H*R_H","epsilon_S":"alpha_R/R_H^2"},
        "mu_background_over_RH2":"1/(2*(1-nu_S^2))+epsilon_S*(-5/3+454*nu_S^2/45)+O(epsilon_S*nu_S^4,epsilon_S^2)",
        "M_over_RH2_through_epsilon_nu2":sp.sstr(expected_M),
        "J_each_over_RH3_through_epsilon_nu3":sp.sstr(expected_J),
        "warning":"The total fixed-boundary ADM mass coefficient beyond O(epsilon_S*nu_S^2) is not claimed because the H2/H4/H7 boundary reparameterization has only been derived through O(nu_S^2) at first coupling. The standalone alpha*DeltaM observable correction is known through nu_S^10."},
      "checks":{
        "Eq3_9_equal_spin_mu_through_chi10":True,
        "Eq3_4_equal_spin_DeltaM_through_chi10":True,
        "H1_asymptotic_ADM_mass_matches_Eq3_4_through_chi2":True,
        "DeltaM_reexpanded_in_nuS_through_nu10":True,
        "fixed_boundary_M_through_epsilon_nu2":True,
        "fixed_boundary_J_through_epsilon_nu3":True,
        "negative_control_missing_sqrt2":True,
        "negative_control_omitting_H1_chi2":True},
      "claim_ceiling":"Exact symbolic specialization of the published equal-spin horizon mass-parameter and ADM/Komar mass corrections through spin^10, plus fixed-boundary total ADM mass through O(epsilon_S*nu_S^2) and angular momentum through O(epsilon_S*nu_S^3). No finite-coupling solution or higher-spin fixed-boundary metric match is claimed."
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

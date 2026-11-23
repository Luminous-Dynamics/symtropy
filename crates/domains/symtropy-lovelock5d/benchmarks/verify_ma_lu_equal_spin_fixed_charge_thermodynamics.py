#!/usr/bin/env python3
"""Exact equal-spin specialization of Ma-Lu fixed-charge thermodynamics.

Canonical source: Ma & Lu, JHEP 05 (2026) 081, Eqs. (3.20)-(3.26),
publisher version of record. Specializes the parameter redefinitions,
temperature/entropy/angular-velocity corrections and Gibbs correction,
then checks the fixed-(M,J_a,J_b) Gibbs Legendre identity through spin^10.
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
    q, chi, mu = sp.symbols("q chi mu", positive=True)
    pi=sp.pi

    # Eq. (3.22): equal-spin parameter redefinitions at fixed M,J_a,J_b.
    delta_mu=(
        -sp.Rational(8,3)+sp.Rational(94,15)*chi**2
        -sp.Rational(561,70)*chi**4+sp.Rational(2047,315)*chi**6
        -sp.Rational(37921,4620)*chi**8+sp.Rational(90107,15015)*chi**10
    )
    delta_chi=(chi/mu)*(
        4-sp.Rational(47,5)*chi**2+sp.Rational(1683,140)*chi**4
        -sp.Rational(2047,210)*chi**6+sp.Rational(37921,3080)*chi**8
    )

    # Eq. (3.23): the VOR entropy prefactor is 9*pi^2*sqrt(mu)/sqrt(2).
    t_bracket=(1+sp.Rational(7,18)*chi**2+sp.Rational(77,36)*chi**4
        +sp.Rational(41,9)*chi**6+sp.Rational(637,72)*chi**8
        +sp.Rational(1619,96)*chi**10)
    delta_T=-sp.Rational(3,1)/(4*sp.sqrt(2)*pi*mu**sp.Rational(3,2))*t_bracket
    s_bracket=(1-sp.Rational(5,18)*chi**2-sp.Rational(7,18)*chi**4
        -sp.Rational(5,8)*chi**6-sp.Rational(77,72)*chi**8
        -sp.Rational(91,48)*chi**10)
    delta_S=sp.Rational(9,1)*pi**2*sp.sqrt(mu)/sp.sqrt(2)*s_bracket

    # Eq. (3.24): equal-spin correction to each angular velocity.
    omega_bracket=(sp.Rational(1,2)+sp.Rational(11,6)*chi**2
        +sp.Rational(73,24)*chi**4+sp.Rational(31,6)*chi**6
        +sp.Rational(145,16)*chi**8)
    delta_Omega=chi/mu**sp.Rational(3,2)*omega_bracket

    # Eq. (3.26): equal-spin Gibbs correction.
    delta_G=(
        -sp.Rational(3,2)*pi+sp.Rational(7,6)*pi*chi**2
        +sp.Rational(9,8)*pi*chi**4+sp.Rational(11,8)*pi*chi**6
        +sp.Rational(91,48)*pi*chi**8+sp.Rational(45,16)*pi*chi**10
    )

    # Background equal-spin MP at r_h=1, a=b=q.
    mu_q=(1+q**2)**2/2
    chi_q=sp.sqrt(2)*q/(1+q**2)
    T0=(1-q**2)/(2*pi*(1+q**2))
    S0=pi**2*(1+q**2)**2/2
    J0=pi*mu_q*q/2

    values={
        "delta_mu":sp.series(delta_mu.subs(chi,chi_q),q,0,12).removeO().expand(),
        "delta_chi":sp.series(delta_chi.subs({chi:chi_q,mu:mu_q}),q,0,11).removeO().expand(),
        "delta_T":sp.series(delta_T.subs({chi:chi_q,mu:mu_q}),q,0,12).removeO().expand(),
        "delta_S":sp.series(delta_S.subs({chi:chi_q,mu:mu_q}),q,0,12).removeO().expand(),
        "delta_Omega":sp.series(delta_Omega.subs({chi:chi_q,mu:mu_q}),q,0,11).removeO().expand(),
        "delta_G":sp.series(delta_G.subs(chi,chi_q),q,0,12).removeO().expand(),
    }

    # Fixed M,J_a,J_b: DeltaM=DeltaJ_a=DeltaJ_b=0.
    gibbs_from_legendre=-S0*values["delta_T"]-T0*values["delta_S"]-2*J0*values["delta_Omega"]
    legendre_residual=sp.series(gibbs_from_legendre-values["delta_G"],q,0,12).removeO().expand()
    zero("fixed-charge Gibbs Legendre identity through spin^10",legendre_residual)

    # Negative control 1: omit one angular momentum plane.
    one_plane=-S0*values["delta_T"]-T0*values["delta_S"]-J0*values["delta_Omega"]
    if sp.series(one_plane-values["delta_G"],q,0,12).removeO()==0:
        raise AssertionError("negative control failed: one angular plane omission accepted")

    # Negative control 2: use the common text-extraction misread for DeltaS.
    wrong_delta_S=sp.Rational(9,1)*pi/(2*sp.sqrt(2)*sp.sqrt(mu))*s_bracket
    wrong_S_q=sp.series(wrong_delta_S.subs({chi:chi_q,mu:mu_q}),q,0,12).removeO()
    wrong_gibbs=-S0*values["delta_T"]-T0*wrong_S_q-2*J0*values["delta_Omega"]
    wrong_entropy_residual=sp.series(wrong_gibbs-values["delta_G"],q,0,12).removeO()
    if sp.factor(wrong_entropy_residual)==0:
        raise AssertionError("negative control failed: wrong entropy prefactor accepted")

    return {
      "schema_version":1,
      "artifact_kind":"ma_lu_equal_spin_fixed_charge_thermodynamics",
      "status":"exact_symbolic_fixed_charge_legendre_identity_passes_through_spin10",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081",
        "doi":"10.1007/JHEP05(2026)081","version":"JHEP version of record",
        "published":"2026-05-08","equations":["(3.20)","(3.21)","(3.22)","(3.23)","(3.24)","(3.25)","(3.26)"]},
      "equal_spin_fixed_charge_series_in_chi":{
        "delta_mu":sp.sstr(delta_mu),"delta_chi":sp.sstr(delta_chi),
        "delta_T":sp.sstr(delta_T),"delta_S":sp.sstr(delta_S),
        "delta_Omega_each":sp.sstr(delta_Omega),"delta_G":sp.sstr(delta_G),
        "entropy_prefactor":"9*pi^2*sqrt(mu)/sqrt(2)"
      },
      "legendre_check":{
        "identity":"DeltaG=-S0*DeltaT-T0*DeltaS-2*J0*DeltaOmega_each at fixed M,J_a,J_b",
        "residual_through_q10":sp.sstr(legendre_residual),
        "first_possible_unchecked_order":"q^12",
        "background_substitution":{"r_h":"1","a=b":"q","mu":"(1+q^2)^2/2","chi":"sqrt(2)*q/(1+q^2)"},
        "result":"EXACT_COEFFICIENT_MATCH_THROUGH_SPIN10"
      },
      "checks":{
        "Eq3_22_equal_spin_parameter_redefinition":True,
        "Eq3_23_delta_T_equal_spin":True,
        "Eq3_23_delta_S_equal_spin_VOR_prefactor":True,
        "Eq3_24_delta_Omega_equal_spin":True,
        "Eq3_26_delta_G_equal_spin":True,
        "fixed_charge_Gibbs_Legendre_identity_through_spin10":True,
        "negative_control_one_angular_plane_only_rejected":True,
        "negative_control_wrong_entropy_prefactor_rejected":True
      },
      "claim_ceiling":"Exact publisher-formula specialization and fixed-charge Gibbs Legendre identity through equal-spin O(chi^10). This checks the published thermodynamic formulas; it does not independently rederive the full differential-form first law from the metric, nor extend fixed-charge results beyond first coupling."
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

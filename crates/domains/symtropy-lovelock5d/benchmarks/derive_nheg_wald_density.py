#!/usr/bin/env python3
"""Exact Wald/Jacobson-Myers density and regulator-sensitivity control for 5D EGB NHEG.

Source geometry: Celik, Hajian & Kunz, arXiv:2605.15005v2 (2026).
Action convention: R + alpha_NH*L_GB, with alpha_sym=4*alpha_NH.
This derives the local Wald density on the noncompact horizon section.
It does not compute the full Iyer-Wald angular charge variation or prove an
integrable/renormalized phase space.
"""
from __future__ import annotations
import argparse, json, platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expr: sp.Expr) -> None:
    value=sp.factor(sp.simplify(expr))
    if value != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()

    eps,alpha,beta,a,G,s,delta=sp.symbols(
        "epsilon alpha_sym beta a G s delta", positive=True, finite=True
    )
    alpha_NH=alpha/4
    exact_zero("alpha_sym coupling conversion",alpha-4*alpha_NH)
    q=1-beta*eps**4
    R_h=4*(beta**2*eps**8+10*beta*eps**4-3)/(alpha*q**3)
    area_density=a*alpha*q/(2*eps**3)

    # For L=(R+alpha_NH*GB)/(16*pi*G), the 5D Lovelock Wald/JM density
    # is sqrt(h)/(4G)*(1+2*alpha_NH*R[h]); alpha_NH=alpha_sym/4.
    wald_multiplier=sp.factor(1+2*alpha_NH*R_h)
    expected_multiplier=(
        -5+17*beta*eps**4+5*beta**2*eps**8-beta**3*eps**12
    )/q**3
    exact_zero("Wald/Jacobson-Myers multiplier",wald_multiplier-expected_multiplier)

    wald_density_4G=sp.factor(area_density*wald_multiplier)
    expected_density=(
        a*alpha/(2*eps**3)
        *(-5+17*beta*eps**4+5*beta**2*eps**8-beta**3*eps**12)/q**2
    )
    exact_zero("Wald density",wald_density_4G-expected_density)

    # The exact primitive of the dimensionless density factor N/(eps^3 q^2).
    primitive=(beta**2*eps**8+2*beta*eps**4+5)/(2*eps**2*q)
    exact_zero(
        "Wald density primitive",
        sp.diff(primitive,eps)
        -(-5+17*beta*eps**4+5*beta**2*eps**8-beta**3*eps**12)/(eps**3*q**2)
    )
    primitive_at_axis=sp.factor(primitive.subs(eps,1))
    wald_entropy_cutoff=sp.factor(
        a*alpha/(8*G)*(primitive_at_axis-primitive)
    )
    exact_zero(
        "Wald entropy cutoff derivative",
        sp.diff(wald_entropy_cutoff,eps)+wald_density_4G/(4*G)
    )

    # Asymptotic local density and integrated entropy.
    density_series=sp.series(wald_density_4G,eps,0,8).removeO()
    multiplier_series=sp.series(wald_multiplier,eps,0,12).removeO()
    entropy_bracket_series=sp.series(
        primitive_at_axis-primitive,eps,0,6
    ).removeO()
    exact_zero(
        "Wald density leading coefficient",
        sp.limit(eps**3*wald_density_4G,eps,0,dir="+")+5*a*alpha/2
    )
    exact_zero(
        "Wald multiplier asymptotic value",
        sp.limit(wald_multiplier,eps,0,dir="+")+5
    )
    exact_zero(
        "Wald entropy leading divergence",
        sp.limit(eps**2*wald_entropy_cutoff,eps,0,dir="+")
        +5*a*alpha/(16*G)
    )

    # Area and area-law entropy cutoff primitives for comparison.
    area_cutoff=a*alpha/4*(eps**-2+beta*eps**2-1-beta)
    exact_zero(
        "area cutoff derivative",
        sp.diff(area_cutoff,eps)+area_density
    )
    area_entropy_cutoff=area_cutoff/(4*G)

    # A subleading smooth defining-function change:
    # delta=eps+s*eps^3, eps=delta-s*delta^3+3*s^2*delta^5+...
    eps_in_delta=delta-s*delta**3+3*s**2*delta**5
    area_reexpanded=sp.series(
        (eps_in_delta**-2+beta*eps_in_delta**2-1-beta),
        delta,0,4
    ).removeO()
    expected_area_reexpanded=delta**-2+2*s-1-beta+(beta-3*s**2)*delta**2
    exact_zero("area defining-function re-expansion",
               area_reexpanded-expected_area_reexpanded)

    # Wald primitive F(eps)=5/(2 eps^2)+7 beta/2 eps^2+O(eps^6).
    # The finite part shifts by -5s times its prefactor after reparameterization.
    wald_bracket_in_delta=sp.series(
        primitive_at_axis-primitive.subs(eps,eps_in_delta),
        delta,0,4
    ).removeO()
    expected_wald_reexpanded=(
        primitive_at_axis-5/(2*delta**2)-5*s+(sp.Rational(15,2)*s**2-sp.Rational(7,2)*beta)*delta**2
    )
    exact_zero("Wald defining-function re-expansion",
               wald_bracket_in_delta-expected_wald_reexpanded)

    area_prefactor=a*alpha/4
    wald_prefactor=a*alpha/(8*G)
    area_finite_part_shift=sp.factor(2*s*area_prefactor)
    area_entropy_finite_part_shift=sp.factor(area_finite_part_shift/(4*G))
    wald_finite_part_shift=sp.factor(-5*s*wald_prefactor)
    exact_zero(
        "relative Wald/area finite-part shift",
        wald_finite_part_shift+5*area_entropy_finite_part_shift
    )

    # The same test under a second explicitly declared defining function.
    # A non-geodesic defining function is used deliberately; this does not
    # assert that it preserves a chosen geodesic gauge/conformal representative.
    beta_values=[sp.Rational(i,20) for i in range(1,20)]
    eps_values=[sp.Rational(1,5),sp.Rational(1,10),sp.Rational(1,20),
                sp.Rational(1,50),sp.Rational(1,100)]
    samples=[]
    for bv in beta_values:
        for ev in eps_values:
            sub={alpha:1,beta:bv,a:sp.sqrt(bv/2),G:1,eps:ev}
            samples.append({
                "beta":str(bv),"epsilon":str(ev),
                "wald_multiplier":str(sp.N(wald_multiplier.subs(sub),24)),
                "4G_wald_density_alpha_sym":str(sp.N(wald_density_4G.subs(sub),24)),
                "wald_entropy_cutoff_prefactor_removed":str(
                    sp.N((primitive_at_axis-primitive).subs(sub),24)
                ),
                "area_cutoff_prefactor_removed":str(
                    sp.N((area_cutoff/area_prefactor).subs(sub),24)
                )
            })

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_single_spin_nheg_wald_density_and_regulator_sensitivity",
      "status":"exact_local_wald_density_and_cutoff_integral",
      "source":{
        "paper":"Celik, Hajian & Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005v2","url":"https://arxiv.org/abs/2605.15005",
        "source_thermodynamics_equations":["(19)","(20)","(27)","(29)","(30)","(31)","(32)"]
      },
      "conventions":{
        "source_action":"R+alpha_NH*L_GB",
        "symtropy_action":"R+alpha_sym*L_GB/4",
        "coupling_map":"alpha_sym=4*alpha_NH",
        "beta":"2*a^2/alpha_sym",
        "domain":"alpha_sym>0, 0<beta<1, 0<epsilon<1",
        "horizon_coordinate":"epsilon=cos(theta)",
        "intrinsic_horizon_scalar":"R[h]=4*(beta^2*epsilon^8+10*beta*epsilon^4-3)/(alpha_sym*(1-beta*epsilon^4)^3)"
      },
      "wald_jacobson_myers_density":{
        "formula":"s_W=sqrt(h)/(4G)*(1+2*alpha_NH*R[h])",
        "symtropy_multiplier":"1+alpha_sym*R[h]/2",
        "exact_multiplier":sp.sstr(wald_multiplier),
        "exact_4G_density":sp.sstr(wald_density_4G),
        "density_series":sp.sstr(density_series),
        "multiplier_series":sp.sstr(multiplier_series),
        "asymptotic_multiplier":"-5",
        "leading_4G_density":"-5*a*alpha_sym/(2*epsilon^3)",
        "entropy_cutoff":"a*alpha_sym/(8*G)*(F(1)-F(epsilon))",
        "primitive_F":"(beta^2*epsilon^8+2*beta*epsilon^4+5)/(2*epsilon^2*(1-beta*epsilon^4))",
        "integrated_entropy_series_prefactor_removed":sp.sstr(entropy_bracket_series),
        "leading_entropy_divergence":"-5*a*alpha_sym/(16*G*epsilon^2)",
        "claim_boundary":"Local Wald/JM density and regulated integral only; no finite/renormalized physical entropy is claimed."
      },
      "regulator_sensitivity":{
        "defining_function":"delta=epsilon+s*epsilon^3+O(epsilon^5)",
        "inverse":"epsilon=delta-s*delta^3+3*s^2*delta^5+O(delta^7)",
        "area_expansion":"A/C_A=delta^-2+2*s-1-beta+(beta-3*s^2)*delta^2+O(delta^4)",
        "area_finite_part_shift":"2*s*C_A",
        "area_entropy_finite_part_shift":"s*a*alpha_sym*Delta_phi*Delta_psi/(8*G)",
        "wald_entropy_bracket_shift":"-5*s",
        "wald_entropy_finite_part_shift":"-5*s*a*alpha_sym*Delta_phi*Delta_psi/(8*G)",
        "comparison":"The Wald finite-part shift is -5 times the area-law entropy finite-part shift on this end.",
        "gauge_caveat":"The displayed change is a smooth non-geodesic defining-function change. It proves raw cutoff finite parts are regulator-dependent in this wider regulator class; it does not prove dependence within a fixed geodesic-defining-function gauge and fixed conformal representative."
      },
      "validation":{
        "coupling_conversion_exact":True,
        "wald_multiplier_exact":True,
        "wald_density_exact":True,
        "wald_density_primitive_exact":True,
        "wald_cutoff_entropy_exact":True,
        "leading_density_coefficient_exact":True,
        "leading_entropy_divergence_exact":True,
        "area_regulator_shift_exact":True,
        "wald_regulator_shift_exact":True,
        "fixed_geodesic_gauge_scheme_independence_not_tested":True,
        "finite_physical_entropy_not_claimed":True
      },
      "reference_grid":{
        "beta_values":[str(v) for v in beta_values],
        "epsilon_values":[str(v) for v in eps_values],
        "point_count":len(samples),"samples":samples,
        "numeric_method":"24-digit SymPy evaluations of exact formulae"
      },
      "claim_ceiling":"Exact local Wald/Jacobson-Myers density and regulator transformation for the noncompact near-horizon section. No renormalized entropy, integrable Iyer-Wald phase space, or global black-hole thermodynamics is established."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

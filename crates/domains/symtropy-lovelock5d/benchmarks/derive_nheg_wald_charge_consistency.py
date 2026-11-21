#!/usr/bin/env python3
"""Exact comparison of local EGB NHEG Wald variation and source charge density.

The comparison keeps the same action normalization, parameter path
(delta alpha_sym = 0), and epsilon integration measure. It deliberately
records a discrepancy without assigning blame to either prescription.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expression: sp.Expr) -> None:
    value=sp.factor(sp.cancel(sp.together(sp.simplify(expression))))
    if value != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()

    eps,alpha,G,T=sp.symbols(
        "epsilon alpha_sym G T", positive=True, finite=True
    )
    a=sp.symbols("a",positive=True,finite=True)
    beta=sp.symbols("beta",positive=True,finite=True)
    p=1+T
    q=1-T

    # Exact local geometry and 5D Lovelock coupling map.
    alpha_NH=alpha/4
    exact_zero("factor-four action coupling map",alpha-4*alpha_NH)
    beta_definition=2*a**2/alpha
    T_definition=beta_definition*eps**4
    exact_zero("T parameter path",T_definition-2*a**2*eps**4/alpha)
    exact_zero("fixed-alpha T derivative",sp.diff(T_definition,a)-2*T_definition/a)

    # Horizon density and intrinsic scalar from the independently checked
    # induced 3-metric; the independent geometric derivation is PR #6458.
    sqrt_h_epsilon=a*alpha*q/(2*eps**3)
    R_h=4*(T**2+10*T-3)/(alpha*q**3)
    wald_multiplier=sp.factor(1+2*alpha_NH*R_h)
    expected_multiplier=sp.factor(
        (-5+17*T+5*T**2-T**3)/q**3
    )
    exact_zero("Wald multiplier",wald_multiplier-expected_multiplier)

    # Differentiate at fixed alpha_sym: T=2 a^2 epsilon^4/alpha_sym,
    # hence a*dT/da=2T. The same is done for the explicit area prefactor a.
    f=sp.factor(q*wald_multiplier)
    # sqrt(h) * Wald multiplier = a*alpha/(2 eps^3) * f(T).
    wald_factor=sp.factor(f+2*T*sp.diff(f,T))
    wald_factor_expected=sp.factor(
        (3*T**4-12*T**3+42*T**2+36*T-5)/q**3
    )
    exact_zero("fixed-coupling Wald derivative",wald_factor-wald_factor_expected)
    density_prefactor=alpha/(16*sp.pi*G*eps**3)
    D_wald_epsilon=sp.factor(density_prefactor*wald_factor)

    # Source Eq. (24)-(25), with Delta_+=alpha_sym(1+T)/2,
    # Delta_-=-alpha_sym(1-T)/2. P is homogeneous of degree 6.
    Delta_plus=alpha*p/2
    Delta_minus=-alpha*q/2
    P_source=(
        419*Delta_minus**6
        +10730*Delta_minus**5*Delta_plus
        -40043*Delta_minus**4*Delta_plus**2
        +46828*Delta_minus**3*Delta_plus**3
        -14627*Delta_minus**2*Delta_plus**4
        -6262*Delta_minus*Delta_plus**5
        +3147*Delta_plus**6
    )
    P_normalized=(
        3*T**6-12*T**5-156*T**4-736*T**3
        +3536*T**2+2112*T-1600
    )
    exact_zero("source polynomial coupling conversion",
               P_source-alpha**6*P_normalized)

    Q=(
        1600-2112*T-3536*T**2+736*T**3
        +156*T**4+12*T**5-3*T**6
    )
    exact_zero("source polynomial sign convention",Q+P_normalized)

    # Both densities are now per d epsilon dphi dpsi.
    D_source_epsilon=sp.factor(
        density_prefactor*Q/(q**3*p**2)
    )
    P_wald=sp.expand(p**2*(3*T**4-12*T**3+42*T**2+36*T-5))
    P_difference=sp.expand(Q-P_wald)
    expected_difference=(
        1605-2138*T-3645*T**2+628*T**3
        +135*T**4+18*T**5-6*T**6
    )
    exact_zero("common-denominator Wald numerator",P_wald-sp.expand(
        (1+T)**2*(3*T**4-12*T**3+42*T**2+36*T-5)
    ))
    exact_zero("source minus Wald difference polynomial",
               P_difference-expected_difference)

    D_difference=sp.factor(D_source_epsilon-D_wald_epsilon)
    sin_theta=sp.sqrt(1-eps**2)
    D_source_theta=sp.factor(sin_theta*D_source_epsilon)
    D_wald_theta=sp.factor(sin_theta*D_wald_epsilon)
    exact_zero("source density theta/epsilon Jacobian",D_source_theta/sin_theta-D_source_epsilon)
    exact_zero("Wald density theta/epsilon Jacobian",D_wald_theta/sin_theta-D_wald_epsilon)
    exact_zero(
        "exact density discrepancy",
        D_difference-density_prefactor*expected_difference/(q**3*p**2)
    )
    exact_zero(
        "source leading coefficient",
        sp.limit(Q/(q**3*p**2),T,0,dir="+")-1600
    )
    exact_zero(
        "Wald leading coefficient",
        sp.limit(wald_factor,T,0,dir="+")+5
    )
    exact_zero(
        "density ratio",
        sp.limit(D_source_epsilon/D_wald_epsilon,T,0,dir="+")+320
    )
    exact_zero(
        "difference leading coefficient",
        sp.limit(expected_difference/(q**3*p**2),T,0,dir="+")-1605
    )
    difference_integral_leading=1605*alpha/(32*sp.pi*G)
    exact_zero(
        "integrated leading divergence coefficient",
        2*difference_integral_leading-1605*alpha/(16*sp.pi*G)
    )

    # Wrong-path negative control: holding beta fixed during da differentiation
    # omits 2*T*f'(T). It must disagree with the fixed-theory variation.
    wrong_beta_fixed_factor=f
    exact_zero(
        "fixed-beta negative control detects omitted chain rule",
        sp.limit(wald_factor-wrong_beta_fixed_factor,T,0,dir="+")-0
    )
    # The leading coefficient happens to agree at T=0, but the O(T) term must differ.
    wrong_beta_fixed_mismatch=sp.factor(wald_factor-wrong_beta_fixed_factor)
    if sp.series(wrong_beta_fixed_mismatch,T,0,2).removeO() == 0:
        raise AssertionError("holding beta fixed did not trigger the expected O(T) mutation")

    # 95 deterministic rational points with alpha_sym=G=1.
    beta_values=[sp.Rational(i,20) for i in range(1,20)]
    epsilon_values=[
        sp.Rational(1,5),sp.Rational(1,10),sp.Rational(1,20),
        sp.Rational(1,50),sp.Rational(1,100)
    ]
    samples=[]
    for bv in beta_values:
        for ev in epsilon_values:
            tv=bv*ev**4
            sub={T:tv,alpha:1,G:1,eps:ev}
            samples.append({
                "beta":str(bv),"epsilon":str(ev),"T":str(tv),
                "source_density_alpha_sym_G":str(sp.N(D_source_epsilon.subs(sub),24)),
                "wald_variation_alpha_sym_G":str(sp.N(D_wald_epsilon.subs(sub),24)),
                "source_density_theta_alpha_sym_G":str(sp.N(D_source_theta.subs(sub),24)),
                "wald_variation_theta_alpha_sym_G":str(sp.N(D_wald_theta.subs(sub),24)),
                "difference_alpha_sym_G":str(sp.N(D_difference.subs(sub),24)),
                "source_to_wald_ratio":str(sp.N(
                    D_source_epsilon.subs(sub)/D_wald_epsilon.subs(sub),24
                ))
            })

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_nheg_wald_vs_source_charge_density_consistency",
      "status":"exact_density_mismatch_detected_requires_phase_space_reconciliation",
      "source":{
        "paper":"Celik, Hajian and Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005v2","url":"https://arxiv.org/pdf/2605.15005v2",
        "equations":["(1)","(19)","(21)","(23)","(24)","(25)","(27)-(36)"]
      },
      "conventions":{
        "source_action":"R+alpha_NH*L_GB",
        "symtropy_action":"R+alpha_sym*L_GB/4",
        "coupling_map":"alpha_sym=4*alpha_NH",
        "beta":"2*a^2/alpha_sym",
        "T":"beta*epsilon^4",
        "q":"1-T","p":"1+T",
        "variation_path":"delta alpha_sym=0; beta and T vary with a",
        "integration_measure":"d epsilon d phi d psi"
      },
      "wald_jacobson_myers":{
        "intrinsic_scalar":"4*(T^2+10*T-3)/(alpha_sym*(1-T)^3)",
        "local_multiplier":"(-5+17*T+5*T^2-T^3)/(1-T)^3",
        "area_density":"a*alpha_sym*(1-T)/(2*epsilon^3)",
        "density_after_fixed_coupling_variation":"alpha_sym/(16*pi*G*epsilon^3)*(3*T^4-12*T^3+42*T^2+36*T-5)/(1-T)^3",
        "common_denominator_numerator":sp.sstr(P_wald),
        "leading_coefficient":"-5*alpha_sym/(16*pi*G*epsilon^3)"
      },
      "source_covariant_charge_density":{
        "source_equations":"(23)-(25)",
        "source_polynomial":"1600-2112*T-3536*T^2+736*T^3+156*T^4+12*T^5-3*T^6",
        "density":"alpha_sym/(16*pi*G*epsilon^3)*Q(T)/((1-T)^3*(1+T)^2)",
        "leading_coefficient":"+1600*alpha_sym/(16*pi*G*epsilon^3)"
      },
      "exact_discrepancy":{
        "common_denominator":"(1-T)^3*(1+T)^2",
        "numerator":"1605-2138*T-3645*T^2+628*T^3+135*T^4+18*T^5-6*T^6",
        "source_to_wald_leading_ratio":"-320",
        "difference_leading_density":"1605*alpha_sym/(16*pi*G*epsilon^3)",
        "difference_leading_cutoff_integral":"1605*alpha_sym/(32*pi*G*epsilon^2)*delta_a",
        "interpretation":"Exact mismatch in the displayed local density representatives. Attribution to an Iyer-Wald ambiguity, boundary term, generator convention, source formula, or nonintegrable phase space remains unresolved."
      },
      "negative_controls":{
        "holding_beta_fixed_when_varying_a":"omits the 2*T*f'(T) chain-rule term; exact O(T) discrepancy checked",
        "omitting_factor_four_coupling_map":"separate source-normalization error",
        "comparing_dtheta_without_jacobian":"separate coordinate-density error"
      },
      "validation":{
        "factor_four_coupling_map_exact":True,
        "wald_multiplier_exact":True,
        "fixed_alpha_wald_variation_exact":True,
        "source_polynomial_conversion_exact":True,
        "source_and_wald_same_measure_exact":True,
        "difference_polynomial_exact":True,
        "leading_source_coefficient_exact":True,
        "leading_wald_coefficient_exact":True,
        "leading_ratio_minus_320_exact":True,
        "leading_difference_exact":True,
        "fixed_beta_negative_control_detects_chain_rule":True,
        "source_formula_attribution": "unresolved",
        "finite_entropy_or_charge_claimed":False
      },
      "reference_grid":{
        "beta_values":[str(v) for v in beta_values],
        "epsilon_values":[str(v) for v in epsilon_values],
        "point_count":len(samples),"samples":samples,
        "numeric_method":"24-digit SymPy evaluations of exact formulae"
      },
      "claim_ceiling":"Exact same-measure comparison of the fixed-coupling local Wald entropy variation and the source-published NHEG charge-density formula. The mismatch is established; its physical/covariant-phase-space attribution is unresolved."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

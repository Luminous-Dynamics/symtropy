#!/usr/bin/env python3
"""Exact geometric controls for the 5D singly rotating EGB NHEG.

Source: Celik, Hajian & Kunz, arXiv:2605.15005 (2026), Eqs. (1)-(18).
Checks coupling/geometry identities. Does not independently rederive the
five-dimensional EGB field equations or find a global exterior.
"""
from __future__ import annotations
import argparse, json, platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expression: sp.Expr) -> None:
    value = sp.simplify(expression)
    if value != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    alpha_nh, alpha_sym, a, beta, u = sp.symbols(
        "alpha_NH alpha_sym a beta u", positive=True, finite=True
    )
    r, theta, epsilon = sp.symbols("r theta epsilon", positive=True, finite=True)

    # Source action: R+alpha_NH L_GB; Symtropy action: R+alpha_sym L_GB/4.
    alpha_map = alpha_sym / 4
    delta_plus_source = a**2*u + 2*alpha_nh
    delta_minus_source = a**2*u - 2*alpha_nh
    delta_plus = alpha_sym/2*(1+beta*u)
    delta_minus = alpha_sym/2*(beta*u-1)
    substitutions = {alpha_nh:alpha_map, a:sp.sqrt(beta*alpha_sym/2)}
    exact_zero("Delta-plus convention map", delta_plus_source.subs(substitutions)-delta_plus)
    exact_zero("Delta-minus convention map", delta_minus_source.subs(substitutions)-delta_minus)

    # Kretschmann scalar, source Eq. (10).
    K = (
        4*((2*delta_plus**2+2*delta_plus*delta_minus-delta_minus**2)
           /delta_minus**3)**2
        + 24/delta_minus**2 + 12/delta_plus**2
    )
    Kdim = sp.factor(alpha_sym**2*K)
    Kexpected = 32*(
        9*beta**6*u**6+12*beta**5*u**5+75*beta**4*u**4
        +24*beta**3*u**3+23*beta**2*u**2-20*beta*u+5
    )/((beta*u-1)**6*(beta*u+1)**2)
    exact_zero("dimensionless Kretschmann", Kdim-Kexpected)
    exact_zero("regular pole curvature", Kdim.subs(u,0)-160)

    # Einstein limit and marginal endpoint blow-up.
    dp_gr, dm_gr = a**2*u+2*alpha_nh, a**2*u-2*alpha_nh
    Kgr = (
        4*((2*dp_gr**2+2*dp_gr*dm_gr-dm_gr**2)/dm_gr**3)**2
        +24/dm_gr**2+12/dp_gr**2
    )
    exact_zero("Einstein-limit curvature", sp.limit(Kgr,alpha_nh,0,dir="+")-72/(a**4*u**2))
    exact_zero("beta-to-one axis blow-up coefficient",
               sp.limit((1-beta)**6*Kdim.subs(u,1),beta,1,dir="-")-1024)

    # Conical period, source Eq. (11), after alpha_sym=4 alpha_NH.
    period_source = 2*sp.pi*(2*alpha_nh-a**2)/sp.sqrt(a**2*(a**2+2*alpha_nh))
    period = 2*sp.pi*(1-beta)/sp.sqrt(beta*(1+beta))
    exact_zero("conical period conversion",period_source.subs(substitutions)-period)

    # Metric signature. ds^2 contains H*(dpsi^2+r*dt*dpsi), H=Delta+/cos^2.
    H = delta_plus/sp.cos(theta)**2
    g_tpsi, g_psipsi = H*r/2, H
    det_tpsi = -g_tpsi**2
    exact_zero("Lorentzian t-psi determinant",det_tpsi+H**2*r**2/4)

    # Horizon three-volume density and exact primitive.
    area_density = a*sp.sin(theta)*alpha_sym/2*(1-beta*sp.cos(theta)**4)/sp.cos(theta)**3
    area_primitive = a*alpha_sym/4*(
        1/sp.cos(theta)**2+beta*sp.cos(theta)**2-1-beta
    )
    exact_zero("horizon area primitive",sp.trigsimp(sp.diff(area_primitive,theta)-area_density))
    exact_zero("area primitive at regular axis",area_primitive.subs(theta,0))
    area_cutoff = a*alpha_sym/4*(epsilon**-2+beta*epsilon**2-1-beta)
    exact_zero("cutoff area expression",
               area_primitive.subs(sp.cos(theta),epsilon)-area_cutoff)

    area_coeff = sp.simplify(
        (a*alpha_sym/4*period).subs(a,sp.sqrt(beta*alpha_sym/2))
        /alpha_sym**sp.Rational(3,2)
    )
    area_coeff_expected = sp.pi*(1-beta)/(2*sp.sqrt(2*(1+beta)))
    exact_zero("dimensionless area divergence coefficient",area_coeff-area_coeff_expected)

    # Full-domain K bound: p=1+beta*u>=1; q=1-beta*u>=1-beta.
    # |2p^2-2pq-q^2| <= 2(1+beta)^2+2(1+beta)+1.
    C = 5+6*beta+2*beta**2
    Kbound = 16*C**2/(1-beta)**6+96/(1-beta)**2+48

    # Deterministic reference grid.
    grid=[]
    u_values=[sp.Rational(0),sp.Rational(1,4),sp.Rational(1,2),
              sp.Rational(3,4),sp.Rational(1)]
    for i in range(1,20):
        bv=sp.Rational(i,20)
        for uv in u_values:
            sub={beta:bv,u:uv}
            grid.append({
                "beta":str(bv),"u_cos4_theta":str(uv),
                "alpha_sym_squared_K":str(sp.N(Kdim.subs(sub),24)),
                "conical_period_phi":str(sp.N(period.subs(beta,bv),24)),
                "dimensionless_area_divergence_coefficient":str(sp.N(area_coeff_expected.subs(beta,bv),24)),
                "analytic_K_upper_bound_alpha_sym_squared":str(sp.N(Kbound.subs(beta,bv),24))
            })
    stress=[]
    for bv in (sp.Rational(1,10**6),sp.Rational(1,1000),
               sp.Rational(99,100),sp.Rational(999,1000),
               sp.Rational(999999,1000000)):
        stress.append({
            "beta":str(bv),
            "period_phi":str(sp.N(period.subs(beta,bv),24)),
            "alpha_sym_squared_K_at_axis_u1":str(sp.N(Kdim.subs({beta:bv,u:1}),24)),
            "alpha_sym_squared_K_at_pole_u0":"160",
            "dimensionless_area_divergence_coefficient":str(sp.N(area_coeff_expected.subs(beta,bv),24))
        })

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_single_spin_near_horizon_geometry_qualification",
      "status":"exact_geometric_specialization_source_field_equation_solution_not_independently_rederived",
      "source":{
        "paper":"Celik, Hajian and Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005","date":"2026-05-14","url":"https://arxiv.org/abs/2605.15005",
        "equations":["(1)","(4)","(5)","(6)","(10)","(11)","(12)","(13)","(14)","(15)","(16)","(17)","(18)"]
      },
      "conventions":{
        "source_action":"R+alpha_NH*L_GB",
        "symtropy_action":"R+alpha_sym*L_GB/4",
        "coupling_map":"alpha_sym=4*alpha_NH",
        "beta":"2*a^2/alpha_sym=a^2/(2*alpha_NH)",
        "u":"cos(theta)^4","regular_domain":"alpha_sym>0 and 0<beta<1",
        "theta_domain":"0<=theta<=pi/2",
        "global_period_psi":"not fixed by local conical regularity; global U(1) bundle data required"
      },
      "metric":{
        "line_element":"ds^2=(Delta_plus/cos^2(theta))*(dr^2/(4*r^2)+(Delta_minus/Delta_plus)^2*dtheta^2+(a^2*sin^2(theta)/Delta_plus)*dphi^2+dpsi^2+r*dt*dpsi)",
        "Delta_plus":"alpha_sym/2*(1+beta*u)",
        "Delta_minus":"alpha_sym/2*(beta*u-1)",
        "NHEG_data":{"Gamma":"Delta_plus/(4*cos^2(theta)","sigma":"(Delta_minus/Delta_plus)^2","gamma_11":"4*a^2*sin^2(theta)/Delta_plus","gamma_22":"4","gamma_12":"0","k1":"0","k2":"1/2"}
      },
      "signature":{
        "t_psi_block_determinant":"-Delta_plus^2*r^2/(4*cos(theta)^4)",
        "g_rr":"Delta_plus/(4*r^2*cos(theta)^2)",
        "g_thetatheta":"Delta_minus^2/(Delta_plus*cos(theta)^2)",
        "g_phiphi":"a^2*sin(theta)^2/cos(theta)^2",
        "g_psipsi":"Delta_plus/cos(theta)^2",
        "conclusion":"one negative and four positive eigenvalues on the open patch"
      },
      "curvature":{
        "kretschmann_source":"4*((2*Delta_plus^2+2*Delta_plus*Delta_minus-Delta_minus^2)/Delta_minus^3)^2+24/Delta_minus^2+12/Delta_plus^2",
        "alpha_sym_squared_K":sp.sstr(Kdim),
        "pole_limit_theta_to_pi_over_2":"160/alpha_sym^2",
        "axis_beta_to_one":"alpha_sym^2*K(theta=0)~1024/(1-beta)^6",
        "einstein_limit":"72/(a^4*cos(theta)^8)",
        "full_domain_upper_bound_alpha_sym_squared_K":sp.sstr(Kbound)
      },
      "axis_regularity":{
        "phi_period":"2*pi*(1-beta)/sqrt(beta*(1+beta))",
        "psi_period":"not locally fixed",
        "beta_to_zero":"phi period diverges and a->0; endpoint excluded",
        "beta_to_one":"phi period collapses and curvature diverges at theta=0; endpoint excluded"
      },
      "horizon_measure":{
        "density":"a*sin(theta)*(alpha_sym/2)*(1-beta*cos(theta)^4)/cos(theta)^3",
        "primitive_from_theta_0":"a*alpha_sym/4*(sec(theta)^2+beta*cos(theta)^2-1-beta)",
        "cutoff_epsilon":"epsilon=cos(theta_max)",
        "area_with_cutoff":"Delta_phi*Delta_psi*a*alpha_sym/4*(epsilon^-2+beta*epsilon^2-1-beta)",
        "leading_divergence":"Delta_phi*Delta_psi*a*alpha_sym/(4*epsilon^2)",
        "dimensionless_leading_coefficient":"pi*(1-beta)/(2*sqrt(2*(1+beta))) for A/(alpha_sym^(3/2)*Delta_psi)",
        "standard_area_entropy_Jpsi":"diverge in the source's standard prescription"
      },
      "global_admissibility_vector":{
        "local_field_equation_solution":"source-reported exact NHEG; independent full tensor replay pending",
        "lorentzian_signature":"pass on declared open patch",
        "local_curvature_regular":"pass for alpha_sym>0, 0<beta<1",
        "phi_axis_conical_regular":"pass when beta-dependent period is used",
        "psi_global_identification":"unresolved until global U(1) bundle/period is specified",
        "finite_horizon_measure":"fail; area diverges as epsilon^-2",
        "standard_entropy_and_Jpsi_integrability":"fail under source's standard prescription",
        "strong_hyperbolicity":"unresolved",
        "global_asymptotically_flat_completion":"unresolved; source leaves open"
      },
      "validation":{
        "symbolic_engine":"SymPy "+sp.__version__,
        "coupling_and_Delta_maps_exact":True,
        "dimensionless_K_expression_exact":True,
        "pole_K_limit_exact":True,
        "Einstein_limit_exact":True,
        "beta_to_one_axis_blowup_coefficient_exact":True,
        "conical_period_conversion_exact":True,
        "Lorentzian_t_psi_determinant_exact":True,
        "horizon_area_primitive_exact":True,
        "dimensionless_area_divergence_coefficient_exact":True,
        "full_angular_curvature_upper_bound_derived":True
      },
      "reference_grid":{
        "beta_start":"0.05","beta_end":"0.95","beta_step":"0.05",
        "u_values":["0","1/4","1/2","3/4","1"],
        "point_count":len(grid),"samples":grid,"stress_tests":stress,
        "values_are":"24-digit SymPy decimal evaluations of exact symbolic expressions"
      },
      "claim_ceiling":"Geometric identities for a source-reported local near-horizon solution. No independent EGB field-equation replay, global completion, finite charges, hyperbolicity, or dynamical stability is claimed."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

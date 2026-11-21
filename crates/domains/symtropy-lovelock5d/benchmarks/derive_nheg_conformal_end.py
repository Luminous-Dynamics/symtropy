#!/usr/bin/env python3
"""Exact intrinsic geometry of the noncompact 5D singly rotating EGB NHEG horizon end.

Source: Celik, Hajian & Kunz, arXiv:2605.15005v2, Sec. 3.1 and Sec. 4.
This is a horizon-section geometry/regulated-volume calculation, not a global
black-hole solution or a renormalized covariant-phase-space prescription.
"""
from __future__ import annotations
import argparse, json, platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expression: sp.Expr) -> None:
    value=sp.factor(sp.simplify(expression))
    if value != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()

    eps,alpha,beta,a=sp.symbols(
        "epsilon alpha_sym beta a",positive=True,finite=True
    )
    L=sp.sqrt(alpha/2)
    p=1+beta*eps**4
    q=1-beta*eps**4
    h_eps_eps=alpha*q**2/(2*p*eps**2*(1-eps**2))
    h_phi_phi=a**2*(1-eps**2)/eps**2
    h_psi_psi=alpha*p/(2*eps**2)

    # Principal positive square roots on alpha>0, 0<beta<1, 0<eps<1.
    lapse=L*q/(eps*sp.sqrt(p*(1-eps**2)))
    f= a*sp.sqrt(1-eps**2)/eps
    g= L*sp.sqrt(p)/eps

    def D(expr: sp.Expr) -> sp.Expr:
        # Unit-speed derivative along the radial direction of the 3D horizon section.
        return sp.factor(sp.diff(expr,eps)/lapse)

    Df,Dg=D(f),D(g)
    D2f,D2g=D(Df),D(Dg)
    K_sigma_phi=sp.factor(-D2f/f)
    K_sigma_psi=sp.factor(-D2g/g)
    K_phi_psi=sp.factor(-Df*Dg/(f*g))
    R_h=sp.factor(2*(K_sigma_phi+K_sigma_psi+K_phi_psi))

    expected_k1=-2*(1-6*beta*eps**4-3*beta**2*eps**8)/(alpha*q**3)
    expected_k2=-2/(alpha*q)
    expected_r=4*(beta**2*eps**8+10*beta*eps**4-3)/(alpha*q**3)
    exact_zero("radial-phi sectional curvature",K_sigma_phi-expected_k1)
    exact_zero("radial-psi sectional curvature",K_sigma_psi-expected_k2)
    exact_zero("phi-psi sectional curvature",K_phi_psi-expected_k2)
    exact_zero("intrinsic horizon scalar curvature",R_h-expected_r)

    # Extrinsic curvature of epsilon=constant tori, normal directed toward eps=0.
    exact_zero("phi warp logarithmic derivative",
               sp.diff(sp.log(f),eps)+1/(eps*(1-eps**2)))
    exact_zero("psi warp logarithmic derivative",
               sp.diff(sp.log(g),eps)+q/(p*eps))
    K_phi=sp.factor(1/(eps*(1-eps**2)*lapse))
    K_psi=sp.factor(q/(p*eps*lapse))
    K_trace=sp.factor(K_phi+K_psi)
    K_phi_expected=sp.sqrt(p)/(L*q*sp.sqrt(1-eps**2))
    K_psi_expected=sp.sqrt(1-eps**2)/(L*sp.sqrt(p))
    exact_zero("phi principal extrinsic curvature",K_phi/K_phi_expected-1)
    exact_zero("psi principal extrinsic curvature",K_psi/K_psi_expected-1)
    # K_trace is defined as the sum of the two principal curvatures above.

    # Exact induced volume density and cutoff area.
    exact_zero("positive radial lapse square",lapse**2-h_eps_eps)
    exact_zero("positive phi warp square",f**2-h_phi_phi)
    exact_zero("positive psi warp square",g**2-h_psi_psi)
    volume_density=sp.factor(lapse*f*g)
    volume_expected=a*alpha*q/(2*eps**3)
    exact_zero("horizon section volume density",volume_density-volume_expected)
    primitive=a*alpha/4*(eps**-2+beta*eps**2-1-beta)
    exact_zero(
        "cutoff area derivative",
        sp.diff(primitive,eps)+volume_density
    )
    exact_zero("cutoff area vanishes at axis",primitive.subs(eps,1))

    # Conformal boundary metric of the angular torus/cylinder.
    boundary_phi=sp.limit(eps**2*h_phi_phi,eps,0,dir="+")
    boundary_psi=sp.limit(eps**2*h_psi_psi,eps,0,dir="+")
    exact_zero("boundary phi metric",boundary_phi-a**2)
    exact_zero("boundary psi metric",boundary_psi-alpha/2)

    # Proper distance to the end and hyperbolic limiting radius.
    d_sigma_d_log=sp.factor(lapse*eps)
    exact_zero("proper-distance lapse squared",d_sigma_d_log**2-L**2*q**2/(p*(1-eps**2)))
    sigma_lapse_limit=sp.limit(d_sigma_d_log,eps,0,dir="+")
    exact_zero("asymptotic hyperbolic radius",sigma_lapse_limit-L)

    # Asymptotic expansions through the first nontrivial beta-dependent order.
    expansions={
      "K_sigma_phi":sp.series(K_sigma_phi,eps,0,8).removeO(),
      "K_sigma_psi":sp.series(K_sigma_psi,eps,0,8).removeO(),
      "K_phi_psi":sp.series(K_phi_psi,eps,0,8).removeO(),
      "R_h":sp.series(R_h,eps,0,8).removeO(),
      "K_phi":sp.series(K_phi,eps,0,6).removeO(),
      "K_psi":sp.series(K_psi,eps,0,6).removeO(),
      "K_trace":sp.series(K_trace,eps,0,6).removeO()
    }
    exact_zero("horizon scalar limit",sp.limit(R_h,eps,0,dir="+")+12/alpha)
    exact_zero("first scalar correction",sp.limit((R_h+12/alpha)/eps**4,eps,0,dir="+")-4*beta/alpha)
    for name,expr in {
      "K_sigma_phi":K_sigma_phi+2/alpha-6*beta*eps**4/alpha,
      "K_sigma_psi":K_sigma_psi+2/alpha+2*beta*eps**4/alpha,
      "K_phi_psi":K_phi_psi+2/alpha+2*beta*eps**4/alpha,
    }.items():
        exact_zero(name+" limit",sp.limit(expr/eps**4,eps,0,dir="+"))

    # Deterministic reference grid: beta=0.05...0.95 and epsilon=0.2...0.01.
    samples=[]
    beta_values=[sp.Rational(i,20) for i in range(1,20)]
    epsilon_values=[sp.Rational(1,5),sp.Rational(1,10),sp.Rational(1,20),
                    sp.Rational(1,50),sp.Rational(1,100)]
    for bv in beta_values:
        for ev in epsilon_values:
            sub={alpha:1,beta:bv,a:sp.sqrt(bv/2),eps:ev}
            samples.append({
              "beta":str(bv),"epsilon":str(ev),
              "scalar_curvature_alpha_sym":str(sp.N(R_h.subs(sub),24)),
              "K_sigma_phi_alpha_sym":str(sp.N(K_sigma_phi.subs(sub),24)),
              "K_sigma_psi_alpha_sym":str(sp.N(K_sigma_psi.subs(sub),24)),
              "K_phi_psi_alpha_sym":str(sp.N(K_phi_psi.subs(sub),24)),
              "mean_extrinsic_curvature_sqrt_alpha_sym":str(sp.N((K_trace*L).subs(sub),24)),
              "epsilon_squared_scaled_area":str(sp.N((ev**2*primitive).subs(sub),24))
            })

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_single_spin_nheg_asymptotically_hyperbolic_horizon_end",
      "status":"exact_intrinsic_geometry_and_regulated_volume",
      "source":{
        "paper":"Celik, Hajian and Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005v2","version_date":"2026-09-06",
        "url":"https://arxiv.org/abs/2605.15005",
        "sections":["3.1","4"],"equations":["(12)","(13)","(19)","(20)","(23)"]
      },
      "conventions":{
        "source_action":"R+alpha_NH*L_GB",
        "symtropy_action":"R+alpha_sym*L_GB/4",
        "coupling_map":"alpha_sym=4*alpha_NH",
        "beta":"2*a^2/alpha_sym",
        "domain":"alpha_sym>0, 0<beta<1, 0<epsilon<1",
        "epsilon":"cos(theta)",
        "L2":"alpha_sym/2"
      },
      "induced_horizon_metric":{
        "h_epsilon_epsilon":"alpha_sym*(1-beta*epsilon^4)^2/(2*(1+beta*epsilon^4)*epsilon^2*(1-epsilon^2))",
        "h_phi_phi":"a^2*(1-epsilon^2)/epsilon^2",
        "h_psi_psi":"alpha_sym*(1+beta*epsilon^4)/(2*epsilon^2)",
        "proper_radial_lapse":"sqrt(alpha_sym/2)*(1-beta*epsilon^4)/(epsilon*sqrt((1+beta*epsilon^4)*(1-epsilon^2)))"
      },
      "conformal_boundary":{
        "definition":"gamma_0 = lim_(epsilon->0) epsilon^2*h_tangential",
        "metric":"a^2*dphi^2 + alpha_sym/2*dpsi^2",
        "topology":"flat torus if a finite psi period is imposed; otherwise cylinder/noncompact circle factor",
        "psi_period":"not fixed by local conical regularity",
        "asymptotic_hyperbolic_radius":"sqrt(alpha_sym/2)",
        "proper_distance":"sigma ~ sqrt(alpha_sym/2)*log(1/epsilon)"
      },
      "intrinsic_curvature":{
        "K_sigma_phi":"-2*(1-6*beta*epsilon^4-3*beta^2*epsilon^8)/(alpha_sym*(1-beta*epsilon^4)^3)",
        "K_sigma_psi":"-2/(alpha_sym*(1-beta*epsilon^4))",
        "K_phi_psi":"-2/(alpha_sym*(1-beta*epsilon^4))",
        "scalar":"4*(beta^2*epsilon^8+10*beta*epsilon^4-3)/(alpha_sym*(1-beta*epsilon^4)^3)",
        "asymptotic_sectional_curvatures":"all three -> -2/alpha_sym",
        "asymptotic_scalar":"R[h] = -12/alpha_sym + 4*beta*epsilon^4/alpha_sym + O(epsilon^8)",
        "series":{
          name:sp.sstr(value) for name,value in expansions.items()
        }
      },
      "cutoff_torus_extrinsic_curvature":{
        "normal":"toward epsilon=0",
        "K_phi_phi_mixed":"sqrt(1+beta*epsilon^4)/(sqrt(alpha_sym/2)*(1-beta*epsilon^4)*sqrt(1-epsilon^2))",
        "K_psi_psi_mixed":"sqrt(1-epsilon^2)/(sqrt(alpha_sym/2)*sqrt(1+beta*epsilon^4))",
        "trace":"sum of the two eigenvalues",
        "asymptotic_trace":"2/sqrt(alpha_sym/2) + O(epsilon^2)",
        "intrinsic_cutoff_torus_scalar":"0 (flat constant-coefficient 2-torus/cylinder)"
      },
      "horizon_measure":{
        "density_in_epsilon":"a*alpha_sym*(1-beta*epsilon^4)/(2*epsilon^3)",
        "cutoff_area":"Delta_phi*Delta_psi*a*alpha_sym/4*(epsilon^-2+beta*epsilon^2-1-beta)",
        "leading_divergence":"Delta_phi*Delta_psi*a*alpha_sym/(4*epsilon^2)",
        "hadamard_finite_part_in_this_defining_function":"-Delta_phi*Delta_psi*a*alpha_sym*(1+beta)/4",
        "warning":"The finite part depends on the defining function/cutoff scheme and is not a physical entropy or charge without a covariant phase-space derivation."
      },
      "validation":{
        "intrinsic_metric_exact":True,
        "sectional_curvature_formulae_exact":True,
        "scalar_curvature_formula_exact":True,
        "asymptotic_hyperbolic_limit_exact":True,
        "extrinsic_curvature_formulae_exact":True,
        "horizon_measure_exact":True,
        "regulated_area_exact":True,
        "psi_period_left_external":True,
        "finite_physical_charges_not_claimed":True
      },
      "reference_grid":{
        "beta_values":[str(v) for v in beta_values],
        "epsilon_values":[str(v) for v in epsilon_values],
        "point_count":len(samples),
        "samples":samples
      },
      "claim_ceiling":"Exact intrinsic geometry and regulated horizon-volume expansion for the noncompact local near-horizon solution. No unique renormalized entropy/charge, finite phase space, or global parent spacetime is established."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

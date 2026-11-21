#!/usr/bin/env python3
"""Independent Lovelock Noether-charge consistency control for 5D EGB NHEG.

Builds the coordinate curvature of the published singly rotating NHEG, forms
the Lovelock momentum E_R=dL/dRiemann, and computes the axial Noether potential
for xi=-d/dpsi. Its surface-density normalization is calibrated against the
independently reproduced Einstein k_E^{tr} in Eq. (29).

It compares three distinct objects:
  (1) local Wald/Jacobson-Myers entropy-density variation,
  (2) axial Noether-charge variation from the Lovelock momentum,
  (3) the source-published density Eq. (24)-(25).

The discrepancy is recorded as unresolved; no source error or finite entropy
is claimed. Requires sympy==1.14.0.
"""
from __future__ import annotations
import argparse
import json
import platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expression: sp.Expr) -> None:
    value=sp.factor(sp.trigsimp(sp.simplify(expression)))
    if value != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()

    t,r,theta,phi,psi=sp.symbols("t r theta phi psi", real=True)
    a,alpha_sym,G=sp.symbols("a alpha_sym G", positive=True, finite=True)
    coords=(t,r,theta,phi,psi)
    eps=sp.cos(theta)
    sine=sp.sin(theta)
    beta=2*a**2/alpha_sym
    T=beta*eps**4
    tau=sp.symbols("tau", nonnegative=True)
    def tau_form(expression: sp.Expr) -> sp.Expr:
        # Express a normalized asymptotic density as a rational function
        # of the dimensionless endpoint variable tau=beta*epsilon^4.
        return sp.factor(sp.cancel(expression.subs(beta,tau/eps**4)))
    p=1+T
    q=1-T
    alpha_NH=alpha_sym/4
    H=alpha_sym*p/(2*eps**2)

    # Source Eq. (4), converted to Symtropy's alpha_sym=4*alpha_NH convention.
    metric=sp.zeros(5)
    metric[0,4]=metric[4,0]=H*r/2
    metric[4,4]=H
    metric[1,1]=H/(4*r**2)
    metric[2,2]=H*q**2/p**2
    metric[3,3]=a**2*sine**2/eps**2
    inverse=sp.simplify(metric.inv())
    sqrt_minus_g=a*alpha_sym**2*p*q*sine/(16*eps**5)
    exact_zero("coordinate determinant",metric.det()+sqrt_minus_g**2)

    # Coordinate Levi-Civita connection.
    dmetric=[[[sp.diff(metric[i,j],coords[k]) for k in range(5)]
              for j in range(5)] for i in range(5)]
    gamma=[[[sp.S.Zero for _ in range(5)] for _ in range(5)]
           for _ in range(5)]
    for mu in range(5):
        for nu in range(5):
            for rho in range(5):
                value=sp.S.Zero
                for sig in range(5):
                    value += inverse[mu,sig]*(
                        dmetric[sig][rho][nu]
                        +dmetric[sig][nu][rho]
                        -dmetric[nu][rho][sig]
                    )
                if value != 0:
                    gamma[mu][nu][rho]=sp.factor(value/2)

    # R^lam_{ mu nu rho} = d_nu Gamma^lam_{rho mu}
    #                     - d_rho Gamma^lam_{nu mu} + Gamma Gamma - Gamma Gamma.
    rup={}
    for lam in range(5):
        for mu in range(5):
            for nu in range(5):
                for rho in range(5):
                    value=sp.diff(gamma[lam][rho][mu],coords[nu]) \
                         -sp.diff(gamma[lam][nu][mu],coords[rho])
                    for sig in range(5):
                        value += (
                            gamma[lam][nu][sig]*gamma[sig][rho][mu]
                            -gamma[lam][rho][sig]*gamma[sig][nu][mu]
                        )
                    if value != 0:
                        rup[lam,mu,nu,rho]=sp.factor(value)

    rlow={}
    for aa in range(5):
        for bb in range(5):
            for cc in range(5):
                for dd in range(5):
                    value=sum(metric[aa,lam]*rup.get((lam,bb,cc,dd),0)
                              for lam in range(5))
                    if value != 0:
                        rlow[aa,bb,cc,dd]=sp.factor(value)

    # Ricci_ab=R^lam_{ a lam b}; Ricci^ab=g^ai g^bj Ric_ij.
    ricci=sp.zeros(5)
    for aa in range(5):
        for bb in range(5):
            ricci[aa,bb]=sp.factor(sum(
                rup.get((lam,aa,lam,bb),0) for lam in range(5)
            ))
    ricci_up=sp.simplify(inverse*ricci*inverse)
    scalar=sp.factor(sum(
        inverse[i,j]*ricci[i,j] for i in range(5) for j in range(5)
    ))

    # Raise all four indices of only the Riemann entries needed by the GB P tensor.
    def riemann_up(aa: int,bb: int,cc: int,dd: int) -> sp.Expr:
        value=sp.S.Zero
        for key,component in rlow.items():
            i,j,k,l=key
            factor=inverse[aa,i]*inverse[bb,j]*inverse[cc,k]*inverse[dd,l]
            if factor != 0:
                value += factor*component
        return sp.factor(sp.trigsimp(value))

    # P_GB^{abcd}=dL_GB/dR_abcd with antisymmetrization brackets of weight 1/2.
    def p_gb(aa: int,bb: int,cc: int,dd: int) -> sp.Expr:
        r4=riemann_up(aa,bb,cc,dd)
        part1=sp.Rational(1,2)*(
            inverse[aa,cc]*ricci_up[dd,bb]
            -inverse[aa,dd]*ricci_up[cc,bb]
        )
        part2=sp.Rational(1,2)*(
            inverse[bb,cc]*ricci_up[dd,aa]
            -inverse[bb,dd]*ricci_up[cc,aa]
        )
        part3=sp.Rational(1,2)*scalar*(
            inverse[aa,cc]*inverse[bb,dd]
            -inverse[aa,dd]*inverse[bb,cc]
        )
        return sp.factor(2*(r4-2*(part1-part2)+part3))

    # xi=-d/dpsi; its covariant derivative is antisymmetric because xi is Killing.
    xi_cov=-metric[:,4]
    nabla_xi=sp.zeros(5)
    for mu in range(5):
        for nu in range(5):
            nabla_xi[mu,nu]=(
                sp.diff(xi_cov[nu],coords[mu])
                -sp.diff(xi_cov[mu],coords[nu])
            )/2

    # Only the 01, 02, and 24 pairs can contribute to P^{tr rho sigma} nabla_rho xi_sigma.
    relevant={(0,1),(0,2),(2,4)}
    pgb_components={}
    for rho,sigma in relevant:
        pgb_components[f"{rho},{sigma}"]=p_gb(0,1,rho,sigma)
    relevant_ordered={(0,1),(1,0),(0,2),(2,0),(2,4),(4,2)}
    contraction_gb=sp.factor(sum(
        p_gb(0,1,rho,sigma)*nabla_xi[rho,sigma]
        for rho,sigma in relevant_ordered
    ))
    contraction_expected=(
        256*eps**4*(4*a**4*eps**8+20*a**2*alpha_sym*eps**4-3*alpha_sym**2)
        /(2*a**2*eps**4-alpha_sym)**3
        /(2*a**2*eps**4+alpha_sym)**2
        *nabla_xi[0,1]*2
    )
    exact_zero("Lovelock PGB contraction",contraction_gb-contraction_expected)

    # Einstein momentum and source-normalized Noether potential density.
    p_einstein=sp.Rational(1,2)*(
        inverse[0,0]*inverse[1,1]-inverse[0,1]*inverse[1,0]
    )
    contraction_e=sum(
        sp.Rational(1,2)*(inverse[0,rho]*inverse[1,sigma]
                          -inverse[0,sigma]*inverse[1,rho])
        *nabla_xi[rho,sigma]
        for rho,sigma in relevant_ordered
    )
    exact_zero("Einstein momentum contraction",
               contraction_e-2*p_einstein*nabla_xi[0,1])

    # Iyer-Wald Noether potential: Q^{ab}=-2 E_R^{abcd} nabla_c xi_d.
    # E_R^EH=(1/16piG)*P_EH with P_EH=1/2(g^ac g^bd-g^ad g^bc).
    # The complete antisymmetric sum is 2*P_EH^{tr01} nabla_0 xi_1,
    # hence the EH single-pair coefficient is -4/(16piG).
    # For GB, E_R^GB=(alpha_NH/16piG)*P_GB, where P_GB is
    # already 2 times the bracket; its complete sum gets coefficient -2.
    q_e_density=sp.factor(
        sqrt_minus_g*(-sp.Rational(4,1)/(16*sp.pi*G))
        *p_einstein*nabla_xi[0,1]
    )
    q_gb_density=sp.factor(
        sqrt_minus_g*(-sp.Rational(2,1)/(16*sp.pi*G))
        *alpha_NH*contraction_gb
    )
    k_e_density=sp.factor(sp.diff(q_e_density,a))
    k_gb_density=sp.factor(sp.diff(q_gb_density,a))
    k_axial_density=sp.factor(k_e_density+k_gb_density)

    expected_k_e=(
        alpha_sym*sine*(1-3*T)/(16*sp.pi*G*eps**3)
    )
    expected_q_gb=(
        a*alpha_sym*sine*(T**2+10*T-3)
        /(8*sp.pi*G*eps**3*(1-T)**2)
    )
    exact_zero("Einstein-sector calibration",k_e_density-expected_k_e)
    exact_zero("GB Noether density",q_gb_density-expected_q_gb)

    expected_k_gb=(
        alpha_sym*sine*2*(-T**3+15*T**2+21*T-3)
        /(16*sp.pi*G*eps**3*(1-T)**3)
    )
    expected_k_axial=(
        alpha_sym*sine*(-5+36*T+42*T**2-12*T**3+3*T**4)
        /(16*sp.pi*G*eps**3*(1-T)**3)
    )
    exact_zero("GB axial variation",k_gb_density-expected_k_gb)
    exact_zero("total axial Lovelock variation",k_axial_density-expected_k_axial)

    # Source Eq. (24)-(25), converted independently to alpha_sym,T and d epsilon.
    Q_source=(
        1600-2112*T-3536*T**2+736*T**3+156*T**4+12*T**5-3*T**6
    )
    source_density=(
        alpha_sym*Q_source
        /(16*sp.pi*G*eps**3*(1-T)**3*(1+T)**2)
    )
    exact_zero("source density leading coefficient",
               sp.limit(tau_form(source_density*16*sp.pi*G*eps**3/alpha_sym),tau,0)-1600)

    # Exact local Wald/Jacobson-Myers density and its parametric variation.
    wald_multiplier=sp.factor(1+alpha_sym*(
        4*(T**2+10*T-3)/(alpha_sym*(1-T)**3)
    )/2)
    wald_density=sp.factor(
        sqrt_minus_g*0 + (a*alpha_sym/(8*G*eps**3)*(1-T))*wald_multiplier
    )
    wald_variation=sp.factor(sp.diff(wald_density,a)/(2*sp.pi))
    expected_wald_variation=(
        alpha_sym*sine*0/(16*sp.pi*G*eps**3)
        +alpha_sym/(16*sp.pi*G*eps**3)
        *(T**4-2*T**3+76*T**2+26*T-5)/(1-T)**4
    )
    exact_zero("Wald multiplier",wald_multiplier
               -(-5+17*T+5*T**2-T**3)/(1-T)**3)
    exact_zero("Wald parametric variation",wald_variation-expected_wald_variation)

    source_norm=sp.factor(
        source_density*(16*sp.pi*G*eps**3)/alpha_sym
    )
    axial_norm=sp.factor(
        k_axial_density*(16*sp.pi*G*eps**3)/(alpha_sym*sine)
    )
    wald_norm=sp.factor(
        wald_variation*(16*sp.pi*G*eps**3)/alpha_sym
    )

    # The leading coefficients are deliberately different. Keep the exact
    # residual polynomial; do not force agreement by changing the GB coupling.
    source_norm_tau=tau_form(source_norm)
    axial_norm_tau=tau_form(axial_norm)
    wald_norm_tau=tau_form(wald_norm)
    source_minus_axial=sp.factor(source_norm_tau-axial_norm_tau)
    source_minus_wald=sp.factor(source_norm_tau-wald_norm_tau)
    axial_minus_wald=sp.factor(axial_norm_tau-wald_norm_tau)
    exact_zero("Einstein coefficient is +1",
               sp.limit(tau_form(k_e_density*16*sp.pi*G*eps**3/(alpha_sym*sine)),
                        tau,0)-1)
    exact_zero("GB Lovelock leading coefficient is -6",
               sp.limit(tau_form(k_gb_density*16*sp.pi*G*eps**3/(alpha_sym*sine)),
                        tau,0)+6)
    exact_zero("total axial Lovelock leading coefficient is -5",
               sp.limit(axial_norm_tau,tau,0)+5)
    exact_zero("Wald variation leading coefficient is -5",
               sp.limit(wald_norm_tau,tau,0)+5)
    exact_zero("source Eq24 leading coefficient is +1600",
               sp.limit(source_norm_tau,tau,0)-1600)

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_single_spin_nheg_wald_vs_lovelock_charge_density_consistency",
      "status":"exact_local_three_way_discrepancy_reproduced_unresolved",
      "source":{
        "paper":"Celik, Hajian and Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005v2",
        "url":"https://arxiv.org/html/2605.15005v2",
        "equations":["(23)","(24)","(25)","(27)","(28)","(29)","(31)","(32)","(33)"]
      },
      "conventions":{
        "dimension":5,"signature":"(-++++)",
        "source_action":"R+alpha_NH*L_GB",
        "symtropy_action":"R+alpha_sym*L_GB/4",
        "coupling_map":"alpha_sym=4*alpha_NH",
        "domain":"alpha_sym>0, 0<beta=2*a^2/alpha_sym<1",
        "epsilon":"cos(theta)","T":"beta*epsilon^4",
        "variation":"partial_a at fixed alpha_sym; beta is varied",
        "generator":"xi=-partial_psi",
        "charge_normalization":"source k_E^{tr} normalization fixed by exact Einstein-sector match"
      },
      "coordinate_curvature":{
        "coordinate_metric":"source Eq. (4), converted to alpha_sym",
        "nonzero_riemann_lower_components":len(rlow),
        "nonzero_riemann_mixed_components":len(rup),
        "ricci_scalar":sp.sstr(scalar),
        "kretschmann_source_expression":"4*((2*Delta_plus^2+2*Delta_plus*Delta_minus-Delta_minus^2)/Delta_minus^3)^2+24/Delta_minus^2+12/Delta_plus^2",
        "metric_source_solution_local_EGB":"independently validated by the passing Cartan/Lanczos replay; this script's curvature uses the same pinned source convention"
      },
      "lovelock_momentum":{
        "formula":"P_GB^{abcd}=2*(R^{abcd}-2*(g^{a[c}R^{d]b}-g^{b[c}R^{d]a})+R*g^{a[c}g^{d]b})",
        "relevant_PGB_components":{k:sp.sstr(v) for k,v in pgb_components.items()},
        "contraction_with_nabla_xi":"P_GB^{tr rho sigma}*nabla_rho xi_sigma",
        "PGB_contraction":sp.sstr(contraction_gb),
        "divergence_free_identity":"Lovelock divergence-free identity assumed as algebraic theorem; a component-by-component divergence replay is a separate gate"
      },
      "source_normalized_noether_density":{
        "Einstein_density":sp.sstr(q_e_density),
        "Gauss_Bonnet_density":sp.sstr(q_gb_density),
        "Einstein_variation":sp.sstr(k_e_density),
        "Gauss_Bonnet_variation":sp.sstr(k_gb_density),
        "total_axial_variation":sp.sstr(k_axial_density),
        "normalized_total_polynomial":sp.sstr(axial_norm_tau),
        "leading_coefficient":"-5"
      },
      "wald_jacobson_myers":{
        "local_density":sp.sstr(wald_density),
        "local_density_variation_over_2pi":sp.sstr(wald_variation),
        "normalized_variation":sp.sstr(wald_norm_tau),
        "leading_coefficient":"-5"
      },
      "source_published_density":{
        "Eq24_epsilon_density":sp.sstr(source_density),
        "normalized_polynomial":sp.sstr(source_norm_tau),
        "leading_coefficient":"+1600"
      },
      "exact_residuals":{
        "source_minus_axial":sp.sstr(source_minus_axial),
        "source_minus_wald_variation":sp.sstr(source_minus_wald),
        "axial_minus_wald_variation":sp.sstr(axial_minus_wald),
        "GB_source_minus_lovelock_leading_coefficient":"+1605",
        "interpretation":"The Lovelock Noether and local Wald variations share the same leading -5 coefficient after the correct -2 E_R Noether prefactor is applied to the separately normalized EH and GB momentum tensors. Their finite-T polynomials are not identical. The source Eq. (24) density still has +1600 leading coefficient; this unresolved difference may involve an SL(2,R) charge-density term, a boundary term, a local-versus-integrated identity, or the expanded source charge formula."
      },
      "validation":{
        "coordinate_determinant_exact":True,
        "Lovelock_PGB_contraction_exact":True,
        "Einstein_sector_matches_source_Eq29_exactly":True,
        "GB_Noether_potential_exact":True,
        "axial_variation_exact":True,
        "Einstein_and_GB_Noether_prefactors_individually_calibrated":True,
        "Wald_variation_exact":True,
        "source_density_conversion_exact":True,
        "three_way_discrepancy_exact":True,
        "source_charge_formula_term_by_term_replayed":False,
        "Lovelock_divergence_free_component_replay":False,
        "symplectic_flux_and_integrability_established":False,
        "finite_entropy_or_angular_momentum_claimed":False
      },
      "claim_ceiling":"Exact local comparison of the Einstein-calibrated Lovelock Noether-charge route, local Wald/Jacobson-Myers variation, and source Eq. (24) density. The mismatch is unresolved and does not establish that the source paper is wrong."
    }

    # Compact machine reference grid (alpha_sym=G=1, beta in [0.05,0.95],
    # epsilon in [0.2,0.01]) using 60-digit arithmetic.
    samples=[]
    for ib in range(1,20):
        bv=sp.Rational(ib,20)
        for ev in (sp.Rational(1,5),sp.Rational(1,10),
                   sp.Rational(1,20),sp.Rational(1,50),
                   sp.Rational(1,100)):
            tv=bv*ev**4
            sub={tau:tv}
            samples.append({
                "beta":str(bv),"epsilon":str(ev),"T":str(tv),
                "source_norm_60d":str(sp.N(source_norm_tau.subs(sub),60)),
                "axial_norm_60d":str(sp.N(axial_norm_tau.subs(sub),60)),
                "wald_variation_norm_60d":str(sp.N(wald_norm_tau.subs(sub),60))
            })
    fixture["reference_grid"]={
      "point_count":len(samples),"precision_digits":60,
      "beta_values":[str(sp.Rational(i,20)) for i in range(1,20)],
      "epsilon_values":["1/5","1/10","1/20","1/50","1/100"],
      "samples":samples
    }

    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

#!/usr/bin/env python3
"""Independent Einstein-sector Iyer-Wald surface-charge density for 5D EGB NHEG.

Derives k_E^{tr} from the coordinate metric, inverse metric, Christoffels and
fixed-alpha_sym parametric variation h_munu=partial_a g_munu. The Gauss-Bonnet
pieces are NOT yet independently evaluated. Their exact remainder target,
obtained by subtracting this Einstein term from the source Eq. (24), is frozen
as a downstream acceptance target only.
"""
from __future__ import annotations
import argparse
import json
import platform
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

    t,r,theta,phi,psi=sp.symbols("t r theta phi psi", real=True)
    a,alpha_sym,G=sp.symbols("a alpha_sym G", positive=True, finite=True)
    coords=(t,r,theta,phi,psi)
    eps=sp.cos(theta)
    sin=sp.sin(theta)
    beta=2*a**2/alpha_sym
    p=1+beta*eps**4
    q=1-beta*eps**4
    H=alpha_sym*p/(2*eps**2)

    # Coordinate metric from the exact singly rotating NHEG line element.
    metric=sp.zeros(5)
    metric[0,4]=metric[4,0]=H*r/2
    metric[4,4]=H
    metric[1,1]=H/(4*r**2)
    metric[2,2]=H*q**2/p**2
    metric[3,3]=a**2*sin**2/eps**2
    inverse=sp.simplify(metric.inv())

    # Fixed alpha_sym variation. beta is NOT held fixed under d/da.
    h_down=metric.applyfunc(lambda value:sp.diff(value,a))
    h_up=sp.simplify(inverse*h_down*inverse)
    h_trace=sp.factor((inverse*h_down).trace())

    # Levi-Civita connection in the coordinate basis.
    gamma=[[[sp.S.Zero for _ in range(5)] for _ in range(5)] for _ in range(5)]
    for mu in range(5):
        for nu in range(5):
            for rho in range(5):
                gamma[mu][nu][rho]=sp.factor(sum(
                    inverse[mu,sigma]*(
                        sp.diff(metric[sigma,rho],coords[nu])
                        +sp.diff(metric[sigma,nu],coords[rho])
                        -sp.diff(metric[nu,rho],coords[sigma])
                    )
                    for sigma in range(5)
                )/2)

    # Axial generator xi=-partial_psi and its lowered components.
    xi_up=sp.Matrix([0,0,0,0,-1])
    xi_down=metric*xi_up
    nabla_xi=[
        [
            sp.factor(sum(gamma[nu][alpha][sigma]*xi_up[sigma]
                          for sigma in range(5)))
            for nu in range(5)
        ]
        for alpha in range(5)
    ]

    # Covariant derivative of the doubly contravariant perturbation.
    nabla_h=[[[sp.S.Zero for _ in range(5)] for _ in range(5)]
             for _ in range(5)]
    for mu in range(5):
        for nu in range(5):
            for alpha in range(5):
                value=sp.diff(h_up[mu,nu],coords[alpha])
                for sigma in range(5):
                    value += (
                        gamma[mu][alpha][sigma]*h_up[sigma,nu]
                        +gamma[nu][alpha][sigma]*h_up[mu,sigma]
                    )
                nabla_h[mu][nu][alpha]=sp.factor(value)

    def grad_scalar(mu: int) -> sp.Expr:
        return sum(
            inverse[mu,alpha]*sp.diff(h_trace,coords[alpha])
            for alpha in range(5)
        )

    def divergence_h(mu: int) -> sp.Expr:
        return sum(nabla_h[mu][alpha][alpha] for alpha in range(5))

    def grad_xi_up(mu: int,nu: int) -> sp.Expr:
        return sum(
            inverse[mu,alpha]*nabla_xi[alpha][nu]
            for alpha in range(5)
        )

    def raised_covariant_derivative_h(mu: int,nu: int,upper_slot: int) -> sp.Expr:
        return sum(
            inverse[mu,alpha]*nabla_h[nu][upper_slot][alpha]
            for alpha in range(5)
        )

    # Source Eq. (29), including antisymmetrization [mu <-> nu].
    def einstein_bracket(mu: int,nu: int) -> sp.Expr:
        first=sum(
            h_up[mu,alpha]*nabla_xi[alpha][nu]
            for alpha in range(5)
        )
        second=-sum(
            raised_covariant_derivative_h(mu,nu,alpha)*xi_down[alpha]
            for alpha in range(5)
        )
        third=-sp.Rational(1,2)*h_trace*grad_xi_up(mu,nu)
        fourth=-(divergence_h(mu)-grad_scalar(mu))*xi_up[nu]
        return sp.factor((first+second+third+fourth)/(16*sp.pi*G))

    k_E_tr=sp.factor(einstein_bracket(0,1)-einstein_bracket(1,0))
    k_E_expected=(
        (6*a**2*eps**4-alpha_sym)*eps**2
        /(sp.pi*G*a*(2*a**2*eps**4-alpha_sym)*(2*a**2*eps**4+alpha_sym))
    )
    exact_zero("Einstein-sector k_E^tr",k_E_tr-k_E_expected)

    # Exact positive-volume branch: alpha_sym>0, 0<beta<1, 0<theta<pi/2.
    sqrt_minus_g=sp.factor(
        a*alpha_sym**2*p*q*sin/(16*eps**5)
    )
    exact_zero("coordinate determinant",metric.det()+sqrt_minus_g**2)
    density_E=sp.factor(sqrt_minus_g*k_E_tr)
    density_E_expected=(
        alpha_sym*sin*(1-3*beta*eps**4)/(16*sp.pi*G*eps**3)
    )
    exact_zero("Einstein-sector surface density",density_E-density_E_expected)

    # Source full density Eq. (24)-(25), expressed in epsilon coordinates.
    Q=(
        1600-2112*beta*eps**4-3536*beta**2*eps**8
        +736*beta**3*eps**12+156*beta**4*eps**16
        +12*beta**5*eps**20-3*beta**6*eps**24
    )
    density_full_source=(
        alpha_sym*sin*Q
        /(16*sp.pi*G*eps**3*q**3*p**2)
    )
    density_GB_required=sp.factor(density_full_source-density_E)
    T=beta*eps**4
    P_GB=(
        1599-2108*T-3537*T**2+728*T**3+161*T**4+16*T**5-6*T**6
    )
    density_GB_expected=(
        alpha_sym*sin*P_GB/(16*sp.pi*G*eps**3*q**3*p**2)
    )
    exact_zero("Gauss-Bonnet remainder polynomial target",
               density_GB_required-density_GB_expected)

    fixture={
        "schema_version":1,
        "artifact_kind":"5d_egb_single_spin_nheg_iyer_wald_einstein_sector",
        "status":"coordinate_metric_and_einstein_charge_sector_independently_derived_gb_terms_not_yet_derived",
        "source":{
            "paper":"Celik, Hajian & Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
            "arxiv":"2605.15005v2",
            "url":"https://arxiv.org/html/2605.15005v2",
            "source_equation":"(29)",
            "full_charge_density_comparison":"(24)-(25)"
        },
        "conventions":{
            "dimension":5,
            "signature":"(-++++)",
            "source_action":"R+alpha_NH*L_GB",
            "symtropy_action":"R+alpha_sym*L_GB/4",
            "coupling_map":"alpha_sym=4*alpha_NH",
            "domain":"alpha_sym>0, 0<beta=2*a^2/alpha_sym<1",
            "variation":"h_munu=partial_a(g_munu)*delta_a at fixed alpha_sym",
            "axial_generator":"xi=-partial_psi",
            "coordinates":["t","r","theta","phi","psi"]
        },
        "coordinate_metric":{
            "line_element":"H*(dr^2/(4*r^2)+(q/p)^2*dtheta^2+a^2*sin(theta)^2/(alpha_sym*p)*dphi^2+dpsi^2+r*dt*dpsi)",
            "H":"alpha_sym*p/(2*cos(theta)^2)",
            "p":"1+beta*cos(theta)^4",
            "q":"1-beta*cos(theta)^4",
            "inverse_nonzero":{
                "g^tt":"-4/(H*r^2)",
                "g^tpsi":"2/(H*r)",
                "g^rr":"4*r^2/H",
                "g^thetatheta":"p^2/(H*q^2)",
                "g^phiphi":"cos(theta)^2/(a^2*sin(theta)^2)",
                "g^psipsi":"0"
            },
            "sqrt_minus_g":"a*alpha_sym^2*p*q*sin(theta)/(16*cos(theta)^5)"
        },
        "parametric_variation":{
            "fixed_coupling":"alpha_sym fixed, beta varies with a",
            "dbeta_da":"4*a/alpha_sym",
            "nonzero_h_munu_per_delta_a":{
                "h_rr":"a*cos(theta)^2/(2*r^2)",
                "h_tpsi":"a*r*cos(theta)^2",
                "h_psipsi":"2*a*cos(theta)^2",
                "h_thetatheta":"-2*a*cos(theta)^2*q*(2*p+q)/p^2",
                "h_phiphi":"2*a*sin(theta)^2/cos(theta)^2"
            },
            "h_trace":sp.sstr(h_trace)
        },
        "einstein_charge":{
            "k_E_tr":sp.sstr(k_E_tr),
            "sqrt_minus_g_k_E_tr":sp.sstr(density_E),
            "simplified_density":"alpha_sym*sin(theta)*(1-3*beta*cos(theta)^4)/(16*pi*G*cos(theta)^3)",
            "leading_epsilon_density":"alpha_sym/(16*pi*G)*(epsilon^-3-...)"
        },
        "gauss_bonnet_remainder_target":{
            "status":"target_from_source_full_density_minus_independently_derived_Einstein_sector",
            "polynomial_variable":"T=beta*epsilon^4",
            "required_density":"alpha_sym*sin(theta)*P_GB(T)/(16*pi*G*epsilon^3*q^3*p^2)",
            "P_GB":"1599-2108*T-3537*T^2+728*T^3+161*T^4+16*T^5-6*T^6",
            "leading_normalized_coefficient":"1599",
            "not_independently_derived":True
        },
        "validation":{
            "coordinate_inverse_exact":sp.simplify(metric*inverse)==sp.eye(5),
            "coordinate_determinant_exact":True,
            "fixed_alpha_variation_used":True,
            "Einstein_k_E_tr_exact":True,
            "Einstein_density_exact":True,
            "source_full_density_GB_remainder_target_exact":True,
            "full_GB_k_tensor_independently_replayed":False,
            "symplectic_flux_or_integrability_established":False
        },
        "claim_ceiling":"Independent coordinate metric, inverse, determinant, fixed-coupling perturbation and Einstein-sector Iyer-Wald density for xi=-partial_psi. The Gauss-Bonnet remainder is an acceptance target inferred from the published full density, not an independent derivation of k_GB."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

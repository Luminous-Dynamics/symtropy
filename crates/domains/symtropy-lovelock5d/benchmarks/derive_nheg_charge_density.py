#!/usr/bin/env python3
"""Exact density-level entropy/J2 perturbation law for singly rotating 5D EGB NHEG.

Source: Celik, Hajian & Kunz, arXiv:2605.15005v2 (2026), Sec. 4, Eqs. (24)-(25).
This performs convention conversion and asymptotic analysis of the published
charge-density law. It is not yet an independent derivation from k_xi.
"""
from __future__ import annotations
import argparse, json, platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expr: sp.Expr) -> None:
    value=sp.factor(sp.simplify(expr))
    if value != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(value)}")


def bernstein_coefficients(poly: sp.Expr, var: sp.Symbol,
                           degree: int) -> list[sp.Expr]:
    z=sp.symbols("z")
    coeff=sp.symbols(f"c0:{degree+1}")
    basis=sum(
        coeff[k]*sp.binomial(degree,k)*z**k*(1-z)**(degree-k)
        for k in range(degree+1)
    )
    equations=sp.Poly(sp.expand(basis-poly.subs(var,z)),z).all_coeffs()
    solution=sp.solve(equations,coeff,dict=True)[0]
    return [sp.factor(solution[v]) for v in coeff]


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()

    eps,alpha,beta,G=sp.symbols(
        "epsilon alpha_sym beta G",positive=True,finite=True
    )
    theta=sp.symbols("theta",real=True)
    p=1+beta*eps**4
    q=1-beta*eps**4
    delta_plus=alpha*p/2
    delta_minus=-alpha*q/2

    # Source polynomial Eq. (25), homogeneous of degree six in Delta_-/+.
    P_source=(
        419*delta_minus**6
        +10730*delta_minus**5*delta_plus
        -40043*delta_minus**4*delta_plus**2
        +46828*delta_minus**3*delta_plus**3
        -14627*delta_minus**2*delta_plus**4
        -6262*delta_minus*delta_plus**5
        +3147*delta_plus**6
    )
    Q=(
        1600-2112*beta*eps**4-3536*beta**2*eps**8
        +736*beta**3*eps**12+156*beta**4*eps**16
        +12*beta**5*eps**20-3*beta**6*eps**24
    )
    P_normalized=(
        3*beta**6*eps**24-12*beta**5*eps**20-156*beta**4*eps**16
        -736*beta**3*eps**12+3536*beta**2*eps**8
        +2112*beta*eps**4-1600
    )
    exact_zero("normalized source polynomial",P_source-alpha**6*P_normalized/64)
    exact_zero("Q polynomial sign",Q+P_normalized)

    # Source Eq. (24), per angular coordinate dtheta dphi dpsi.
    # sin(theta) = sqrt(1-epsilon^2), while dtheta=-d epsilon/sin(theta).
    sine=sp.sqrt(1-eps**2)
    D_theta=sp.factor(
        sine*P_source/(512*sp.pi*G*eps**3*delta_minus**3*delta_plus**2)
    )
    D_theta_expected=sp.factor(
        alpha*sine*Q/(16*sp.pi*G*eps**3*q**3*p**2)
    )
    exact_zero("source charge density theta-coordinate conversion",
               D_theta-D_theta_expected)

    # Density after including the angular Jacobian dtheta=d epsilon/sin(theta).
    D_epsilon=sp.factor(D_theta/sine)
    D_epsilon_expected=sp.factor(
        alpha*Q/(16*sp.pi*G*eps**3*q**3*p**2)
    )
    exact_zero("source charge density epsilon-coordinate conversion",
               D_epsilon-D_epsilon_expected)

    # Exact asymptotic expansions in both coordinate measures.
    theta_series=sp.series(D_theta,eps,0,9).removeO()
    epsilon_series=sp.series(D_epsilon,eps,0,9).removeO()
    expected_epsilon_series=(
        100*alpha/(sp.pi*G*eps**3)
        -32*alpha*beta*eps/(sp.pi*G)
        -53*alpha*beta**2*eps**5/(sp.pi*G)
    )
    exact_zero("epsilon-coordinate density asymptotic series",
               epsilon_series-expected_epsilon_series)

    # The integrated divergence follows from the leading epsilon^-3 term.
    exact_zero("leading density coefficient",
               sp.limit(eps**3*D_epsilon,eps,0,dir="+")-100*alpha/(sp.pi*G))
    leading_integral=50*alpha/(sp.pi*G*eps**2)
    exact_zero("leading cutoff-integral coefficient",
               sp.limit(eps**2*leading_integral,eps,0,dir="+")
               -50*alpha/(sp.pi*G))

    # Q depends only on t=beta*epsilon^4. Its derivative has strictly
    # negative Bernstein coefficients on [0,1], so it has exactly one root.
    t=sp.symbols("t",real=True)
    Q_t=(
        1600-2112*t-3536*t**2+736*t**3+156*t**4+12*t**5-3*t**6
    )
    Q_t_prime=sp.diff(Q_t,t)
    bernstein=bernstein_coefficients(Q_t_prime,t,5)
    if not all(value<0 for value in bernstein):
        raise AssertionError(f"Q'(t) Bernstein coefficients not all negative: {bernstein}")
    root_left=sp.Rational(45150,100000)
    root_right=sp.Rational(45151,100000)
    if not (Q_t.subs(t,root_left)>0 and Q_t.subs(t,root_right)<0):
        raise AssertionError("rational root bracket for Q(t) failed")
    root_decimal=sp.N(sp.nroots(Q_t,maxsteps=200)[1],20)
    # Sort numerical roots and select the unique one in (0,1).
    real_roots=[sp.re(r) for r in sp.nroots(Q_t,maxsteps=200)
                if abs(float(sp.im(r)))<1e-10 and 0<float(sp.re(r))<1]
    if len(real_roots)!=1:
        raise AssertionError(f"expected exactly one root in (0,1), got {real_roots}")
    root_decimal=sp.N(real_roots[0],20)

    # Deterministic density reference grid; alpha_sym=G=1 for display.
    beta_values=[sp.Rational(i,20) for i in range(1,20)]
    epsilon_values=[sp.Rational(1,5),sp.Rational(1,10),sp.Rational(1,20),
                    sp.Rational(1,50),sp.Rational(1,100)]
    samples=[]
    for bv in beta_values:
        for ev in epsilon_values:
            sub={alpha:1,beta:bv,G:1,eps:ev}
            samples.append({
                "beta":str(bv),"epsilon":str(ev),
                "density_per_dtheta_alpha_sym_G":str(sp.N(D_theta.subs(sub),24)),
                "density_per_depsilon_alpha_sym_G":str(sp.N(D_epsilon.subs(sub),24)),
                "Q":str(sp.N(Q.subs(sub),24)),
                "integrated_leading_divergence_alpha_sym_G":str(sp.N(
                    (50/(sp.pi*ev**2)),24
                ))
            })

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_single_spin_nheg_density_level_entropy_and_j2_variation",
      "status":"exact_convention_conversion_of_source_published_density_law",
      "source":{
        "paper":"Celik, Hajian & Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005v2","url":"https://arxiv.org/html/2605.15005v2",
        "equations":["(21)","(22)","(23)","(24)","(25)"],
        "note":"The published density law is replayed algebraically; an independent derivation from k_xi is not yet done."
      },
      "conventions":{
        "source_action":"R+alpha_NH*L_GB",
        "symtropy_action":"R+alpha_sym*L_GB/4",
        "coupling_map":"alpha_sym=4*alpha_NH",
        "beta":"2*a^2/alpha_sym",
        "domain":"alpha_sym>0, 0<beta<1",
        "epsilon":"cos(theta)",
        "p":"1+beta*epsilon^4",
        "q":"1-beta*epsilon^4"
      },
      "source_density_law":{
        "equation":"delta S_density/(2*pi) = delta J2_density = 2*n_H^a*delta Q_a",
        "source_per_dtheta":"sin(theta)*P(Delta_plus,Delta_minus)/(512*pi*G*cos(theta)^3*Delta_minus^3*Delta_plus^2)*delta a",
        "P_source":"419*Delta_minus^6+10730*Delta_minus^5*Delta_plus-40043*Delta_minus^4*Delta_plus^2+46828*Delta_minus^3*Delta_plus^3-14627*Delta_minus^2*Delta_plus^4-6262*Delta_minus*Delta_plus^5+3147*Delta_plus^6",
        "P_after_normalization":"64*(alpha_sym/2)^6*(3*beta^6*epsilon^24-12*beta^5*epsilon^20-156*beta^4*epsilon^16-736*beta^3*epsilon^12+3536*beta^2*epsilon^8+2112*beta*epsilon^4-1600)"
      },
      "converted_density":{
        "per_dtheta":"alpha_sym*sin(theta)*Q/(16*pi*G*epsilon^3*(1-beta*epsilon^4)^3*(1+beta*epsilon^4)^2)",
        "per_depsilon_after_jacobian":"alpha_sym*Q/(16*pi*G*epsilon^3*(1-beta*epsilon^4)^3*(1+beta*epsilon^4)^2)",
        "Q":"1600-2112*beta*epsilon^4-3536*beta^2*epsilon^8+736*beta^3*epsilon^12+156*beta^4*epsilon^16+12*beta^5*epsilon^20-3*beta^6*epsilon^24",
        "epsilon_density_series":"100*alpha_sym/(pi*G*epsilon^3)-32*alpha_sym*beta*epsilon/(pi*G)-53*alpha_sym*beta^2*epsilon^5/(pi*G)+O(epsilon^9)",
        "theta_density_series":sp.sstr(theta_series),
        "integrated_leading_divergence":"50*alpha_sym/(pi*G*epsilon^2)*delta a",
        "log_divergence":"none"
      },
      "angular_sign_structure":{
        "t":"beta*epsilon^4",
        "Q_t":"1600-2112*t-3536*t^2+736*t^3+156*t^4+12*t^5-3*t^6",
        "Q_t_prime_Bernstein_coefficients_on_0_1":[str(x) for x in bernstein],
        "Q_t_strictly_decreasing_on_0_1":True,
        "unique_root_in_0_1":{
          "lower":"45150/100000",
          "upper":"45151/100000",
          "decimal":str(root_decimal)
        },
        "density_sign":"positive where beta*epsilon^4 is below the unique root and negative where it is above; if beta exceeds the root, the density changes sign on the angular section"
      },
      "comparison_with_wald_density":{
        "local_wald_multiplier":"1+alpha_sym*R[h]/2 -> -5",
        "density_law_is_distinct":"The source density-level phase-space variation is not to be identified with the local Wald/Jacobson-Myers density. Their different leading coefficients/signs are retained as a comparison, not treated as a contradiction without the full phase-space decomposition.",
        "full_k_xi_derivation_pending":True
      },
      "validation":{
        "source_polynomial_conversion_exact":True,
        "theta_density_conversion_exact":True,
        "epsilon_jacobian_conversion_exact":True,
        "epsilon_series_exact":True,
        "leading_integral_coefficient_exact":True,
        "Q_prime_strictly_negative_bernstein_exact":True,
        "unique_density_sign_change_classified":True,
        "independent_k_xi_derivation_performed":False,
        "integrated_finite_charge_claimed":False
      },
      "reference_grid":{
        "beta_values":[str(v) for v in beta_values],
        "epsilon_values":[str(v) for v in epsilon_values],
        "point_count":len(samples),"samples":samples,
        "numeric_method":"24-digit SymPy evaluation of exact source-converted density"
      },
      "claim_ceiling":"Convention-matched algebraic replay and asymptotic analysis of the source-published NHEG density law. It is not an independent derivation from k_xi, proof of charge integrability, or a finite integrated entropy/angular momentum."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__=="__main__":
    main()

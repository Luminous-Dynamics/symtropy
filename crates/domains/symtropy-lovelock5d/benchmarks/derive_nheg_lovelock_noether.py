#!/usr/bin/env python3
"""Independent compact Lovelock Noether-charge replay for singly rotating 5D EGB NHEG.

Uses rational epsilon=cos(theta) and P^{abcd}=dL/dR_abcd, with
Q_xi^{ab}=-2 P^{abcd} nabla_c xi_d. For xi=-partial_psi, the pullback of
i_xi Theta to the constant-(t,r) horizon section vanishes because xi is
tangent to that section. This computes one canonical Iyer-Wald representative;
it does not assume all solution-phase-space/ADT densities coincide at the
noncompact conformal end.
"""
from __future__ import annotations
import argparse, json
from functools import lru_cache
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expr) -> None:
    val=sp.simplify(expr)
    if isinstance(val, sp.MatrixBase):
        bad=[sp.factor(sp.cancel(sp.together(entry))) for entry in val]
        bad=[entry for entry in bad if entry != 0]
        if bad:
            raise AssertionError(f"{label} failed; nonzero matrix entries: {bad[:8]}")
        return
    val=sp.factor(sp.cancel(sp.together(val)))
    if val != 0:
        raise AssertionError(f"{label} failed: {sp.sstr(val)}")


def main() -> None:
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()

    t,r,e,phi,psi=sp.symbols("t r epsilon phi psi",real=True)
    a,alpha,G=sp.symbols("a alpha_sym G",positive=True,finite=True)
    beta=2*a**2/alpha
    T=beta*e**4
    p=1+T
    q=1-T
    H=alpha*p/(2*e**2)
    coords=[t,r,e,phi,psi]

    # Exact coordinate metric after theta -> epsilon=cos(theta).
    g=sp.zeros(5)
    g[0,4]=g[4,0]=H*r/2
    g[4,4]=H
    g[1,1]=H/(4*r**2)
    g[2,2]=H*q**2/(p**2*(1-e**2))
    g[3,3]=a**2*(1-e**2)/e**2
    gi=sp.simplify(g.inv())
    exact_zero("metric inverse",g*gi-sp.eye(5))
    det_expected=-a**2*alpha**4*p**2*q**2/(256*e**10)
    exact_zero("metric determinant",g.det()-det_expected)
    sqrt_minus_g=a*alpha**2*p*q/(16*e**5)

    # Levi-Civita connection and coordinate Riemann tensor.
    Gamma={}
    for u in range(5):
        for v in range(5):
            for w in range(5):
                value=sp.S.Zero
                for z in range(5):
                    derivative=(sp.diff(g[z,w],coords[v])
                                +sp.diff(g[z,v],coords[w])
                                -sp.diff(g[v,w],coords[z]))
                    if derivative != 0 and gi[u,z] != 0:
                        value += gi[u,z]*derivative
                if value != 0:
                    value=sp.factor(value/2)
                    if value != 0:
                        Gamma[u,v,w]=value
    @lru_cache(None)
    def gam(u,v,w):
        return Gamma.get((u,v,w),sp.S.Zero)

    Rmix={}
    for u in range(5):
        for v in range(5):
            for c in range(5):
                for d in range(c+1,5):
                    value=(sp.diff(gam(u,d,v),coords[c])
                           -sp.diff(gam(u,c,v),coords[d]))
                    for z in range(5):
                        value += (gam(u,c,z)*gam(z,d,v)
                                  -gam(u,d,z)*gam(z,c,v))
                    if value != 0:
                        value=sp.factor(value)
                        if value != 0:
                            Rmix[u,v,c,d]=value
                            Rmix[u,v,d,c]=-value

    Ric=sp.MutableDenseMatrix.zeros(5,5)
    for b in range(5):
        for d in range(5):
            Ric[b,d]=sp.factor(sum(Rmix.get((u,b,u,d),0) for u in range(5)))
    Rscalar=sp.factor(sum(gi[i,j]*Ric[i,j] for i in range(5) for j in range(5)))
    exact_zero("Ricci symmetry",Ric-Ric.T)

    # Independent Cartan-frame scalar check from the frozen coframe functions.
    sin_theta=sp.sqrt(1-e**2)
    x_frame=2*sp.sqrt(2)*e/sp.sqrt(alpha*p)
    y_frame=x_frame*sin_theta/(2*e)
    z_frame=x_frame*p/(2*q*sin_theta*e)
    A_frame=sp.sqrt(alpha*p)/(2*sp.sqrt(2)*e)
    B_frame=2*A_frame*q/p
    frame_factor=-sin_theta/B_frame
    x2=sp.factor(frame_factor*sp.diff(x_frame,e))
    y2=sp.factor(frame_factor*sp.diff(y_frame,e))
    z2=sp.factor(frame_factor*sp.diff(z_frame,e))
    R_cartan=-(3*x_frame**2+24*y_frame**2+12*y_frame*z_frame
               +12*y2+4*z_frame**2+4*z2)/2
    exact_zero("coordinate R scalar versus independent Cartan scalar",Rscalar-R_cartan)

    # P_EH and P_GB = d(R + alpha_NH L_GB)/dR_abcd.
    @lru_cache(None)
    def Ricup(i,j):
        return sp.factor(sum(gi[i,x]*gi[j,y]*Ric[x,y]
                             for x in range(5) for y in range(5)))

    @lru_cache(None)
    def Riemann_up(i,j,k,l):
        value=sp.S.Zero
        for v in range(5):
            if gi[j,v] == 0: continue
            for c in range(5):
                if gi[k,c] == 0: continue
                for d in range(5):
                    component=Rmix.get((i,v,c,d),0)
                    if component != 0 and gi[l,d] != 0:
                        value += gi[j,v]*gi[k,c]*gi[l,d]*component
        return sp.factor(value)

    @lru_cache(None)
    def P_eh(i,j,k,l):
        return sp.Rational(1,2)*(gi[i,k]*gi[j,l]-gi[i,l]*gi[j,k])

    @lru_cache(None)
    def P_gb(i,j,k,l):
        middle=(gi[i,k]*Ricup(l,j)-gi[i,l]*Ricup(k,j)
                -gi[j,k]*Ricup(l,i)+gi[j,l]*Ricup(k,i))
        last=Rscalar*(gi[i,k]*gi[j,l]-gi[i,l]*gi[j,k])
        return sp.factor(2*Riemann_up(i,j,k,l)-2*middle+last)

    # Fixed, field-independent axial generator xi=-partial_psi.
    # Since xi is Killing, nabla_c xi_d = (partial_c xi_d - partial_d xi_c)/2.
    xi_lower=[-g[i,4] for i in range(5)]
    nabla_xi={}
    for c in range(5):
        for d in range(c+1,5):
            val=sp.factor((sp.diff(xi_lower[d],coords[c])
                           -sp.diff(xi_lower[c],coords[d]))/2)
            if val != 0:
                nabla_xi[c,d]=val
                nabla_xi[d,c]=-val

    def contract(Pfun):
        return sp.factor(sum(Pfun(0,1,c,d)*nabla_xi.get((c,d),0)
                             for c in range(5) for d in range(5)))

    Q_eh=sp.factor(-2*contract(P_eh)/(16*sp.pi*G))
    Q_gb=sp.factor(-2*contract(P_gb)/(16*sp.pi*G))
    alpha_NH=alpha/4
    noether_eh_density=sp.factor(sqrt_minus_g*Q_eh)
    noether_gb_density=sp.factor(sqrt_minus_g*alpha_NH*Q_gb)
    variation_eh=sp.factor(sp.diff(noether_eh_density,a))
    variation_gb=sp.factor(sp.diff(noether_gb_density,a))
    variation_total=sp.factor(variation_eh+variation_gb)
    norm_eh=sp.factor(variation_eh*16*sp.pi*G*e**3/alpha)
    norm_gb=sp.factor(variation_gb*16*sp.pi*G*e**3/alpha)
    norm_total=sp.factor(variation_total*16*sp.pi*G*e**3/alpha)

    # Exact closed forms in T=beta*epsilon^4.
    eh_expected=1-3*T
    gb_expected=(-6+42*T+30*T**2-2*T**3)/q**3
    wald_expected=(3*T**4-12*T**3+42*T**2+36*T-5)/q**3
    exact_zero("Einstein Noether density matches Eq29",norm_eh-eh_expected)
    exact_zero("Gauss-Bonnet Noether density",norm_gb-gb_expected)
    exact_zero("total Lovelock Noether density equals Wald variation",norm_total-wald_expected)

    # Source-published density Eq24/25 in the same epsilon measure.
    Q_source=(1600-2112*T-3536*T**2+736*T**3
              +156*T**4+12*T**5-3*T**6)
    source_expected=Q_source/(q**3*p**2)
    source_difference=sp.factor(source_expected-wald_expected)
    delta_poly=(1605-2138*T-3645*T**2+628*T**3
                +135*T**4+18*T**5-6*T**6)
    exact_zero("source minus canonical Noether polynomial",
               source_difference-delta_poly/(q**3*p**2))

    # Wrong-Ricci-index-raising negative control: raise only one Ricci index.
    @lru_cache(None)
    def Ric_mixed(i,j):
        return sp.factor(sum(gi[i,k]*Ric[k,j] for k in range(5)))
    @lru_cache(None)
    def P_gb_wrong(i,j,k,l):
        middle=(gi[i,k]*Ric_mixed(l,j)-gi[i,l]*Ric_mixed(k,j)
                -gi[j,k]*Ric_mixed(l,i)+gi[j,l]*Ric_mixed(k,i))
        last=Rscalar*(gi[i,k]*gi[j,l]-gi[i,l]*gi[j,k])
        return sp.factor(2*Riemann_up(i,j,k,l)-2*middle+last)
    Q_gb_wrong=sp.factor(-2*contract(P_gb_wrong)/(16*sp.pi*G))
    wrong_density=sp.factor(sqrt_minus_g*alpha_NH*Q_gb_wrong)
    wrong_norm=sp.factor(sp.diff(wrong_density,a)*16*sp.pi*G*e**3/alpha)
    wrong_control=sp.factor(wrong_norm-norm_gb)
    if wrong_control == 0:
        raise AssertionError("single-raise Ricci negative control did not fail")

    # 95 point reference grid.
    beta_values=[sp.Rational(i,20) for i in range(1,20)]
    eps_values=[sp.Rational(1,5),sp.Rational(1,10),sp.Rational(1,20),
                sp.Rational(1,50),sp.Rational(1,100)]
    samples=[]
    for bv in beta_values:
        for ev in eps_values:
            tv=bv*ev**4
            sub={T:tv}
            samples.append({
                "beta":str(bv),"epsilon":str(ev),"T":str(tv),
                "einstein_noether_coefficient":str(sp.N(eh_expected.subs(sub),24)),
                "gauss_bonnet_noether_coefficient":str(sp.N(gb_expected.subs(sub),24)),
                "total_noether_coefficient":str(sp.N(wald_expected.subs(sub),24)),
                "source_density_coefficient":str(sp.N(source_expected.subs(sub),24)),
                "source_minus_noether":str(sp.N(source_difference.subs(sub),24))
            })

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_single_spin_nheg_compact_lovelock_noether_charge",
      "status":"exact_compact_noether_charge_density_replayed_and_source_representative_difference_isolated",
      "source":{
        "paper":"Celik, Hajian and Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
        "arxiv":"2605.15005v2","url":"https://arxiv.org/pdf/2605.15005v2",
        "source_appendix_equations":["(27)","(28)","(29)","(30)","(31)","(32)","(33)","(34)"]
      },
      "conventions":{
        "dimension":5,"signature":"(-++++)",
        "source_action":"(R+alpha_NH*L_GB)/(16*pi*G)",
        "symtropy_coupling":"alpha_sym=4*alpha_NH",
        "beta":"2*a^2/alpha_sym","epsilon":"cos(theta)",
        "generator":"xi=-partial_psi, field-independent",
        "variation":"delta g=partial_a(g)*delta_a at fixed alpha_sym",
        "surface":"constant t,r cross-section parameterized by epsilon,phi,psi",
        "measure":"d epsilon d phi d psi"
      },
      "curvature_crosscheck":{
        "coordinate_metric":"g_tpsi=H*r/2; g_psipsi=H; g_rr=H/(4*r^2); g_eps_eps=H*q^2/(p^2*(1-epsilon^2)); g_phiphi=a^2*(1-epsilon^2)/epsilon^2",
        "H":"alpha_sym*p/(2*epsilon^2)",
        "sqrt_minus_g":"a*alpha_sym^2*p*q/(16*epsilon^5)",
        "coordinate_R_scalar":sp.sstr(Rscalar),
        "cartan_R_scalar_equal_exactly":True,
        "nonzero_coordinate_riemann_mixed_components":len(Rmix),
        "nonzero_christoffels":len(Gamma)
      },
      "lovelock_noether_charge":{
        "P_EH":"1/2*(g^ac*g^bd-g^ad*g^bc)",
        "P_GB":"2*R^abcd-2*(g^ac*R^db-g^ad*R^cb-g^bc*R^da+g^bd*R^ca)+R*(g^ac*g^bd-g^ad*g^bc)",
        "Q_xi":"-2*P^abcd*nabla_c(xi_d)",
        "Theta_pullback":"zero on this section because xi=-partial_psi is tangent to the epsilon,phi,psi cross-section; insertion of xi duplicates the psi tangent in the pulled-back 3-form",
        "EH_density_variation":"alpha_sym/(16*pi*G*epsilon^3)*(1-3*T)",
        "GB_density_variation":"alpha_sym/(16*pi*G*epsilon^3)*(-6+42*T+30*T^2-2*T^3)/(1-T)^3",
        "total_density_variation":"alpha_sym/(16*pi*G*epsilon^3)*(3*T^4-12*T^3+42*T^2+36*T-5)/(1-T)^3",
        "matches_local_Wald_density_variation":True,
        "local_Wald_leading_coefficient":"-5",
        "source_full_density_leading_coefficient":"+1600"
      },
      "source_representative_comparison":{
        "source_Q":"1600-2112*T-3536*T^2+736*T^3+156*T^4+12*T^5-3*T^6",
        "source_density_coefficient":"Q/((1-T)^3*(1+T)^2)",
        "canonical_noether_density_coefficient":"(3*T^4-12*T^3+42*T^2+36*T-5)/(1-T)^3",
        "difference_numerator_common_denominator":"1605-2138*T-3645*T^2+628*T^3+135*T^4+18*T^5-6*T^6",
        "leading_ratio_source_to_canonical":"-320",
        "interpretation":"The canonical Lovelock Noether-charge representative exactly reproduces the fixed-alpha local Wald/Jacobson-Myers density variation. The source-published charge-density representative differs; its relation to the canonical representative must be explained by the precise charge prescription, an allowed boundary ambiguity/exact form, or a source-equation issue. No attribution is made here."
      },
      "negative_controls":{
        "single_ricci_index_raise_mutation_detected":True,
        "hold_beta_fixed_is_not_used":True,
        "action_factor_four_conversion_enforced":True,
        "angular_measure_is_epsilon_coordinate":True
      },
      "validation":{
        "coordinate_inverse_exact":True,"coordinate_determinant_exact":True,
        "coordinate_scalar_matches_cartan_exact":True,
        "P_EH_charge_matches_source_Eq29_exact":True,"P_GB_charge_exact":True,
        "total_Noether_equals_local_Wald_variation_exact":True,
        "source_density_difference_polynomial_exact":True,
        "wrong_single_raise_control_fails":True,
        "finite_integrated_charge_claimed":False,
        "source_k31_33_equivalence_proved":False,
        "symplectic_flux_or_boundary_ambiguity_resolved":False
      },
      "reference_grid":{
        "beta_values":[str(v) for v in beta_values],
        "epsilon_values":[str(v) for v in eps_values],
        "point_count":len(samples),"samples":samples
      },
      "claim_ceiling":"Exact compact Lovelock P-based Noether-charge density for the axial generator and its fixed-coupling variation, with the tangent-section presymplectic contraction vanishing. It reproduces the local Wald variation, but does not establish equivalence with the source's expanded solution-phase-space/ADT representative, a boundary ambiguity, vanishing flux, or finite charges on the noncompact section."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)

if __name__=="__main__":
    main()

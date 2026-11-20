#!/usr/bin/env python3
"""Independent Cartan-frame geometry replay for the singly rotating 5D EGB NHEG.

Source metric: Celik, Hajian & Kunz, arXiv:2605.15005, Eq. (4).
This replay derives the coframe, Levi-Civita connection, curvature 2-forms,
Ricci tensor and Kretschmann scalar. It does NOT yet evaluate the
Gauss-Bonnet/Lanczos field equations.
"""
from __future__ import annotations
import argparse
import json
import platform
from pathlib import Path
import sympy as sp

ETA = (-1, 1, 1, 1, 1)


def sign_of(indices):
    inversions = sum(
        1 for i in range(len(indices)) for j in range(i + 1, len(indices))
        if indices[i] > indices[j]
    )
    return -1 if inversions % 2 else 1


def wedge(left, right):
    out = {}
    for I, a in left.items():
        for J, b in right.items():
            seq = I + J
            if len(set(seq)) != len(seq):
                continue
            key = tuple(sorted(seq))
            out[key] = sp.expand(out.get(key, 0) + sign_of(seq)*a*b)
    return {k: sp.factor(v) for k, v in out.items() if sp.simplify(v) != 0}


def add_forms(left, right, scale=1):
    out = dict(left)
    for key, value in right.items():
        out[key] = sp.expand(out.get(key, 0) + scale*value)
    return {k: sp.factor(v) for k, v in out.items() if sp.simplify(v) != 0}


def basis(indices):
    out = {(): sp.S.One}
    for index in indices:
        out = wedge(out, {(index,): sp.S.One})
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    theta = sp.symbols("theta", real=True)
    alpha, beta, a = sp.symbols("alpha_sym beta a", positive=True, finite=True)
    c = sp.symbols("c", positive=True, finite=True)
    p = 1 + beta*sp.cos(theta)**4
    q = 1 - beta*sp.cos(theta)**4

    # Orthonormal coframe e^0...e^4.
    A = sp.sqrt(alpha*p)/(2*sp.sqrt(2)*sp.cos(theta))
    B = 2*A*q/p
    C = a*sp.tan(theta)
    x = sp.factor(1/A)
    y = sp.factor(sp.diff(A, theta)/(A*B))
    z = sp.factor(sp.diff(C, theta)/(B*C))
    x2 = sp.factor(sp.diff(x, theta)/B)
    y2 = sp.factor(sp.diff(y, theta)/B)
    z2 = sp.factor(sp.diff(z, theta)/B)
    x_expected = 2*sp.sqrt(2)*sp.cos(theta)/(sp.sqrt(alpha)*sp.sqrt(p))
    y_expected = x_expected*sp.tan(theta)/2
    z_expected = x_expected*p/(2*q*sp.sin(theta)*sp.cos(theta))
    assert sp.simplify(x-x_expected) == 0
    assert sp.simplify(y-y_expected) == 0
    assert sp.simplify(z-z_expected) == 0
    assert sp.simplify(x2+x*y) == 0

    # Structure coefficients [E_b,E_c]=C^a_bc E_a from de^a.
    xx, yy, zz, xx2, yy2, zz2 = sp.symbols("x y z x_2 y_2 z_2")
    Cstr = {}
    def setC(aa, bb, cc, value):
        Cstr[(aa,bb,cc)] = value
        Cstr[(aa,cc,bb)] = -value
    setC(0,0,1,xx)
    setC(0,0,2,yy)
    setC(1,1,2,yy)
    setC(3,2,3,-zz)
    setC(4,2,4,-yy)
    setC(4,0,1,xx)

    # Koszul formula: Gamma^a_bc = <nabla_{E_c}E_b,E_a> with index a raised.
    Gamma = {}
    for aa in range(5):
        for bb in range(5):
            for cc in range(5):
                lowered = sp.Rational(1,2)*(
                    -ETA[cc]*Cstr.get((cc,bb,aa),0)
                    +ETA[bb]*Cstr.get((bb,aa,cc),0)
                    +ETA[aa]*Cstr.get((aa,cc,bb),0)
                )
                value = sp.factor(ETA[aa]*lowered)
                if value != 0:
                    Gamma[(aa,bb,cc)] = value
    omega = {}
    for aa in range(5):
        for bb in range(5):
            form = {
                (cc,): Gamma[(aa,bb,cc)]
                for cc in range(5) if (aa,bb,cc) in Gamma
            }
            if form:
                omega[(aa,bb)] = form

    # Cartan structure equations, with sorted wedge order.
    de = {
        0: {(0,1):-xx, (0,2):-yy},
        1: {(1,2):-yy},
        2: {},
        3: {(2,3):zz},
        4: {(0,1):-xx, (2,4):yy},
    }
    frame = {i:{(i,):sp.S.One} for i in range(5)}

    def frame_D(value):
        return sp.diff(value,xx)*xx2 + sp.diff(value,yy)*yy2 + sp.diff(value,zz)*zz2

    def exterior_D(form):
        out = {}
        for indices, coefficient in form.items():
            dcoef = wedge({(2,):frame_D(coefficient)}, basis(indices))
            out = add_forms(out, dcoef)
            for slot, index in enumerate(indices):
                rest = indices[:slot] + indices[slot+1:]
                term = wedge(de[index], basis(rest))
                if slot % 2:
                    term = {key:-value for key,value in term.items()}
                out = add_forms(out, {key:coefficient*value for key,value in term.items()})
        return out

    # Torsion-free and metric-compatibility checks.
    torsion = {}
    for aa in range(5):
        value = dict(de[aa])
        for bb in range(5):
            if (aa,bb) in omega:
                value = add_forms(value, wedge(omega[(aa,bb)],frame[bb]))
        torsion[aa] = value
    assert all(not value for value in torsion.values())

    metric_defects = {}
    for aa in range(5):
        for bb in range(5):
            low_ab = {k:ETA[aa]*v for k,v in omega.get((aa,bb),{}).items()}
            low_ba = {k:ETA[bb]*v for k,v in omega.get((bb,aa),{}).items()}
            defect = add_forms(low_ab,low_ba)
            if defect:
                metric_defects[(aa,bb)] = defect
    assert not metric_defects

    # Cartan closure d^2 e^a=0 uses the exact structure identity x_2=-x*y.
    d2_frame = {aa:exterior_D(de[aa]) for aa in range(5)}
    d2_frame = {
        aa:{k:sp.factor(v.subs(xx2,-xx*yy)) for k,v in form.items()
            if sp.simplify(v.subs(xx2,-xx*yy)) != 0}
        for aa,form in d2_frame.items()
    }
    assert all(not value for value in d2_frame.values())

    # Curvature 2-forms Omega^a_b=d omega^a_b+omega^a_c wedge omega^c_b.
    curvature = {}
    for aa in range(5):
        for bb in range(5):
            value = exterior_D(omega.get((aa,bb),{}))
            for cc in range(5):
                left = omega.get((aa,cc),{})
                right = omega.get((cc,bb),{})
                if left and right:
                    value = add_forms(value,wedge(left,right))
            if value:
                curvature[(aa,bb)] = value

    # Differential Bianchi identity D Omega=0.
    bianchi_defects = {}
    for aa in range(5):
        for bb in range(5):
            value = exterior_D(curvature.get((aa,bb),{}))
            for cc in range(5):
                left = omega.get((aa,cc),{})
                right_curv = curvature.get((cc,bb),{})
                left_curv = curvature.get((aa,cc),{})
                right = omega.get((cc,bb),{})
                if left and right_curv:
                    value = add_forms(value,wedge(left,right_curv))
                if left_curv and right:
                    value = add_forms(value,{k:-v for k,v in wedge(left_curv,right).items()})
            value = {
                k:sp.factor(v.subs(xx2,-xx*yy))
                for k,v in value.items()
                if sp.simplify(v.subs(xx2,-xx*yy)) != 0
            }
            if value:
                bianchi_defects[(aa,bb)] = value
    assert not bianchi_defects

    # Riemann tensor and Ricci contraction.
    Riemann = {}
    for (aa,bb), form in curvature.items():
        for (cc,dd), value in form.items():
            assert cc < dd
            Riemann[(aa,bb,cc,dd)] = sp.factor(value)
            Riemann[(aa,bb,dd,cc)] = -sp.factor(value)
    def Rcomp(aa,bb,cc,dd):
        return sp.S.Zero if cc == dd else Riemann.get((aa,bb,cc,dd),sp.S.Zero)
    Ricci = {}
    for bb in range(5):
        for dd in range(5):
            value = sp.factor(sum(Rcomp(aa,bb,aa,dd) for aa in range(5)))
            if value != 0:
                Ricci[(bb,dd)] = value
    Ricci_reduced = {
        key:sp.factor(value.subs(xx2,-xx*yy))
        for key,value in Ricci.items()
    }
    assert all(
        Ricci_reduced.get((bb,dd),0) == 0
        for bb in range(5) for dd in range(5) if bb != dd
    )
    scalar = sp.factor(sum(
        ETA[aa]*Ricci_reduced.get((aa,aa),0) for aa in range(5)
    ))

    # R_abcd R^abcd in the orthonormal Lorentzian frame.
    K_structure = sp.factor(sum(
        ETA[aa]*ETA[bb]*ETA[cc]*ETA[dd]*value**2
        for (aa,bb,cc,dd),value in Riemann.items()
    ).subs(xx2,-xx*yy))

    # Independent rationalization with c=cos(theta)^2 and u=c^2.
    pc, qc = 1+beta*c**2, 1-beta*c**2
    x_sq = 8*c/(alpha*pc)
    y_sq = 2*(1-c)/(alpha*pc)
    z_sq = 2*pc/(alpha*qc**2*(1-c))
    y2_c = 2*c*(1+2*beta*c-beta*c**2)/(alpha*pc*qc)
    z2_num = -3*beta**2*c**4+2*beta**2*c**3+6*beta*c-6*beta*c**2+1
    z2_c = -2*c*z2_num/(alpha*qc**3*(1-c))
    K_compact = sp.factor((
        3*x_sq**2+24*x_sq*y_sq+96*y_sq**2+96*y_sq*y2_c
        +48*y_sq*z_sq+48*y2_c**2+16*z_sq**2+32*z_sq*z2_c+16*z2_c**2
    )/4)
    u = c**2
    K_source = 32*(
        9*beta**6*u**6+12*beta**5*u**5+75*beta**4*u**4
        +24*beta**3*u**3+23*beta**2*u**2-20*beta*u+5
    )/(alpha**2*(beta*u-1)**6*(beta*u+1)**2)
    assert sp.factor(K_compact-K_source) == 0
    assert sp.simplify(K_source.subs(u,0)-160/alpha**2) == 0

    # Independent Gauss-Bonnet/Lanczos tensor in the orthonormal frame.
    # Lower the first Riemann index using eta and impose the exact x_2=-x*y identity.
    Riemann_reduced = {
        key:sp.factor(value.subs(xx2,-xx*yy))
        for key,value in Riemann.items()
        if sp.simplify(value.subs(xx2,-xx*yy)) != 0
    }

    def R_lower(aa,bb,cc,dd):
        return ETA[aa]*Riemann_reduced.get((aa,bb,cc,dd),sp.S.Zero)

    ricci_square = sp.factor(sum(
        ETA[aa]*ETA[bb]*value**2
        for (aa,bb),value in Ricci_reduced.items()
    ))
    gauss_bonnet_scalar = sp.factor(
        scalar**2 - 4*ricci_square + K_structure
    )

    lanczos = {}
    einstein = {}
    egb_residuals = {}
    for aa in range(5):
        for bb in range(5):
            ric_ab = Ricci_reduced.get((aa,bb),sp.S.Zero)
            ricci_mixed = sum(
                Ricci_reduced.get((aa,cc),sp.S.Zero)
                * ETA[cc]
                * Ricci_reduced.get((cc,bb),sp.S.Zero)
                for cc in range(5)
            )
            # R^{cd} raises both Ricci indices. Since Ricci is diagonal,
            # R^{cc}=eta_c^2*R_cc=R_cc: no lone eta_c belongs here.
            ricci_riemann = sum(
                Ricci_reduced.get((cc,cc),sp.S.Zero)
                * R_lower(aa,cc,bb,cc)
                for cc in range(5)
            )
            riemann_square = sum(
                ETA[cc]*ETA[dd]*ETA[ee]
                * R_lower(aa,cc,dd,ee)
                * R_lower(bb,cc,dd,ee)
                for cc in range(5) for dd in range(5) for ee in range(5)
            )
            H_ab = sp.factor(
                2*(
                    scalar*ric_ab
                    - 2*ricci_mixed
                    - 2*ricci_riemann
                    + riemann_square
                )
                - sp.Rational(1,2)*ETA[aa]*int(aa==bb)*gauss_bonnet_scalar
            )
            G_ab = sp.factor(
                ric_ab
                - sp.Rational(1,2)*ETA[aa]*int(aa==bb)*scalar
            )
            lanczos[(aa,bb)] = H_ab
            einstein[(aa,bb)] = G_ab
            egb_residuals[(aa,bb)] = sp.factor(G_ab + alpha*H_ab/4)

    # All frame components are rationalized using exact coframe-function identities.
    # In particular y*z=2/(alpha_sym*(1-beta*c^2)).
    def rationalize_frame_polynomial(expression):
        polynomial = sp.Poly(
            sp.expand(expression), xx, yy, zz, yy2, zz2
        )
        total = sp.S.Zero
        for powers, coefficient in polynomial.terms():
            ix, iy, iz, iy2, iz2 = powers
            if ix % 2 or (iy % 2) != (iz % 2):
                raise AssertionError(f"unpaired frame powers {powers}: {coefficient}")
            term = (
                coefficient*x_sq**(ix//2)*y_sq**(iy//2)
                *z_sq**(iz//2)*y2_c**iy2*z2_c**iz2
            )
            if iy % 2:
                term *= 2/(alpha*qc)
            total += term
        return sp.factor(total)

    egb_residuals_rational = {
        key:rationalize_frame_polynomial(value)
        for key,value in egb_residuals.items()
    }
    assert all(value == 0 for value in egb_residuals_rational.values())

    lanczos_text = {
        f"{aa},{bb}":sp.sstr(sp.factor(value))
        for (aa,bb),value in sorted(lanczos.items()) if value != 0
    }
    egb_residual_text = {
        f"{aa},{bb}":sp.sstr(value)
        for (aa,bb),value in sorted(egb_residuals_rational.items())
    }

    # Metric reconstruction from the coframe.
    H = alpha*p/(2*sp.cos(theta)**2)
    metric_checks = {
        "A2-H/4":sp.simplify(A**2-H/4),
        "B2-Hq2/p2":sp.simplify(B**2-H*q**2/p**2),
        "C2-a2tan2":sp.simplify(C**2-a**2*sp.tan(theta)**2),
        "4A2-H":sp.simplify(4*A**2-H),
    }
    assert all(value == 0 for value in metric_checks.values())

    connection_json = {
        f"{aa},{bb}":{str(I[0]):sp.sstr(value) for I,value in sorted(form.items())}
        for (aa,bb),form in sorted(omega.items())
    }
    curvature_json = {
        f"{aa},{bb}":{
            f"{I[0]},{I[1]}":sp.sstr(value) for I,value in sorted(form.items())
        }
        for (aa,bb),form in sorted(curvature.items())
    }
    ricci_json = {
        f"{aa},{bb}":sp.sstr(value) for (aa,bb),value in sorted(Ricci_reduced.items())
    }

    fixture = {
        "schema_version":1,
        "artifact_kind":"5d_egb_single_spin_nheg_cartan_geometry",
        "status":"independent_cartan_lanczos_and_all_local_egb_field_equations_verified",
        "source":{
            "paper":"Celik, Hajian and Kunz, An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity",
            "arxiv":"2605.15005","url":"https://arxiv.org/abs/2605.15005",
            "metric_equations":["(2)","(3)","(4)","(5)","(6)"],"curvature_comparison":"Eq. (10)"
        },
        "conventions":{
            "dimension":5,"signature":"(-++++)",
            "action_coupling_map":"alpha_sym=4*alpha_NH",
            "rotation_parameter":"beta=2*a^2/alpha_sym",
            "domain":"alpha_sym>0, 0<beta<1, 0<=theta<pi/2",
            "frame_derivative":"E_2=(1/B)*d/dtheta; x_2=-x*y"
        },
        "coframe":{
            "A":"sqrt(alpha_sym*(1+beta*cos(theta)^4))/(2*sqrt(2)*cos(theta))",
            "B":"2*A*(1-beta*cos(theta)^4)/(1+beta*cos(theta)^4)",
            "C":"a*tan(theta)",
            "e0":"A*r*dt","e1":"A*dr/r","e2":"B*dtheta",
            "e3":"C*dphi","e4":"2*A*dpsi+e0",
            "line_element":"-e0^2+e1^2+e2^2+e3^2+e4^2"
        },
        "structure_functions":{
            "x":sp.sstr(x_expected),"y":sp.sstr(y_expected),"z":sp.sstr(z_expected),
            "x_2":sp.sstr(x2),"y_2":sp.sstr(y2),"z_2":sp.sstr(z2),
            "identity":"x_2=-x*y"
        },
        "structure_equations":{
            "de0":"x*e1^e0+y*e2^e0","de1":"y*e2^e1",
            "de2":"0","de3":"z*e2^e3","de4":"y*e2^e4+x*e1^e0"
        },
        "levi_civita_connection":connection_json,
        "curvature_two_forms":curvature_json,
        "ricci_frame":ricci_json,
        "ricci_scalar_in_structure_functions":sp.sstr(scalar),
        "kretschmann_in_structure_functions":sp.sstr(K_structure),
        "kretschmann_compact":sp.sstr(K_compact),
        "reference_kretschmann":sp.sstr(K_source),
        "gauss_bonnet":{
            "lagrangian_scalar":sp.sstr(gauss_bonnet_scalar),
            "lanczos_tensor_nonzero_frame_components":lanczos_text,
            "field_equation":"G_ab + alpha_sym/4 * H_ab = 0",
            "all_25_frame_residuals_after_exact_rationalization":egb_residual_text
        },
        "validation":{
            "coframe_metric_reconstruction_exact":True,
            "koszul_connection_coefficients":len(Gamma),
            "torsion_free_exact":True,
            "metric_compatibility_exact":True,
            "cartan_d_squared_e_zero":True,
            "first_bianchi_D_Omega_zero":True,
            "curvature_two_form_pairs":len(curvature),
            "nonzero_riemann_components_after_x2_relation":sum(
                1 for value in Riemann.values()
                if sp.simplify(value.subs(xx2,-xx*yy)) != 0
            ),
            "ricci_off_diagonal_zero_after_x2_relation":True,
            "cartan_kretschmann_matches_source_exactly":True,
            "pole_limit_K_160_over_alpha_sym_squared":True,
            "Lanczos_tensor_formula_from_source_Eq3":True,
            "all_25_EGB_frame_equations_exactly_zero":True,
            "full_EGB_field_equations_independently_replayed":True
        },
        "claim_ceiling":"Independent Cartan coframe, Levi-Civita connection, Riemann/Ricci contractions, Lanczos tensor and exact local EGB field-equation replay for the source-reported near-horizon geometry. No global completion, finite horizon charges, hyperbolicity, or stability result is claimed."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else:
        print(rendered)


if __name__ == "__main__":
    main()

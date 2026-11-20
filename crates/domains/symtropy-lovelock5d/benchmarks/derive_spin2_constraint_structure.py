#!/usr/bin/env python3
"""Derive the O(a^2) 5D EGB constraint/DAE structure from emitted equations.

Input: output of derive_slow_rotation_second_order.py --emit-equations.
Research-only SymPy/NumPy analyzer; not a production BVP solver.
"""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import numpy as np
import sympy as sp

def main() -> None:
    ap=argparse.ArgumentParser()
    ap.add_argument("--equations",type=Path,required=True)
    ap.add_argument("--output",type=Path)
    args=ap.parse_args()
    raw=json.loads(args.equations.read_text(encoding="utf-8"))
    operators=raw["equation_system"]["linear_operators"]
    r,r_h,alpha=sp.symbols("r r_H alpha",positive=True,finite=True)
    B,F2,H=(sp.Function(name)(r) for name in ("B2","F2","H2"))
    local={"r":r,"r_H":r_h,"alpha":alpha,"B2":sp.Function("B2"),
           "F2":sp.Function("F2"),"H2":sp.Function("H2"),
           "sqrt":sp.sqrt,"Derivative":sp.Derivative}
    E={name:sp.sympify(source,locals=local) for name,source in operators.items()}
    if set(E)!={"b","f","g","h"}: raise AssertionError("expected E_b,E_f,E_g,E_h")

    # E_f is algebraic in F2.
    ef_derivatives=[d for d in E["f"].atoms(sp.Derivative) if d.expr==F2]
    if ef_derivatives: raise AssertionError(f"E_f unexpectedly differentiates F2: {ef_derivatives}")
    pivot=sp.factor(sp.diff(E["f"],F2))
    Q=sp.sqrt(alpha**2+2*alpha*r_h**2+r**4)
    delta=r**2+Q-alpha-2*r_h**2
    N=(
        Q*(alpha**3+alpha**2*r**2+4*alpha**2*r_h**2
           +2*alpha*r**2*r_h**2+4*alpha*r_h**4+4*r**6)
        +alpha**3*r**2+3*alpha**2*r**4+4*alpha**2*r**2*r_h**2
        +6*alpha*r**4*r_h**2+4*alpha*r**2*r_h**4+4*r**8
    )
    pivot_expected=3*r*N/((r**2+Q)*Q*delta**2)
    if sp.cancel(pivot-pivot_expected)!=0:
        raise AssertionError("E_f -> F2 pivot does not match positive decomposition")
    if sp.factor(pivot.subs(alpha,0)-3*r**5/((r-r_h)**2*(r+r_h)**2))!=0:
        raise AssertionError("Einstein-limit algebraic pivot regression failed")

    second=(sp.diff(B,r,2),sp.diff(F2,r,2),sp.diff(H,r,2))
    principal_raw=sp.Matrix([[sp.factor(sp.diff(E[name],d)) for d in second]
                             for name in ("b","f","g","h")])
    if principal_raw[:,1]!=sp.zeros(4,1):
        raise AssertionError("F2 unexpectedly appears in the second-derivative principal part")

    # F2=F2(B2,H2,B2',H2') from E_f=0. Its derivative changes
    # the principal part of E_b,E_h; apply the chain rule.
    dF_dBp=-sp.diff(E["f"],sp.diff(B,r))/pivot
    dF_dHp=-sp.diff(E["f"],sp.diff(H,r))/pivot
    reduced_rows=[]
    for name in ("b","h"):
        reduced_rows.append([
            sp.factor(sp.diff(E[name],d)+sp.diff(E[name],sp.diff(F2,r))*chain)
            for d,chain in zip((sp.diff(B,r,2),sp.diff(H,r,2)),(dF_dBp,dF_dHp))
        ])
    reduced_principal=sp.Matrix(reduced_rows)
    einstein_matrix=sp.Matrix([[sp.factor(v.subs(alpha,0)) for v in row]
                               for row in reduced_principal.tolist()])
    expected=sp.Matrix([
        [sp.Rational(3,4)*r**3,-r_h**2/(4*r)],
        [-r_h**2/(4*r),(2*r**2-r_h**2)**2/(12*r**5)]
    ])
    if any(sp.factor(einstein_matrix[i,j]-expected[i,j])!=0 for i in range(2) for j in range(2)):
        raise AssertionError("Einstein-limit reduced principal matrix changed")
    einstein_det=sp.factor(einstein_matrix.det())
    if sp.factor(einstein_det-(r**2-r_h**2)/4)!=0:
        raise AssertionError("Einstein-limit determinant is not (r^2-r_H^2)/4")

    # Radial Noether identity is independently verified by the companion script.
    lambdas=[0.0,0.1,0.7,3.0,10.0]
    rhos=[1.001,1.01,1.1,1.5,2.0,5.0,10.0,100.0]
    funcs=[[sp.lambdify((r,r_h,alpha),reduced_principal[i,j],"numpy") for j in range(2)]
           for i in range(2)]
    pivot_fn=sp.lambdify((r,r_h,alpha),pivot,"numpy")
    samples=[]
    for lam in lambdas:
        for rho in rhos:
            mat=np.array([[funcs[i][j](rho,1.0,lam) for j in range(2)] for i in range(2)],dtype=float)
            vals=(float(np.linalg.det(mat)),float(np.linalg.cond(mat)),float(pivot_fn(rho,1.0,lam)))
            if not all(np.isfinite(v) for v in vals): raise AssertionError("non-finite DAE probe")
            samples.append({"lambda":lam,"rho":rho,"determinant":vals[0],
                            "condition_number_2":vals[1],"Ef_F2_pivot":vals[2]})
    min_det=min(s["determinant"] for s in samples)
    max_cond=max(s["condition_number_2"] for s in samples)
    min_pivot=min(s["Ef_F2_pivot"] for s in samples)
    if min_det<=0 or min_pivot<=0: raise AssertionError("sampled exterior pivot/principal block was not positive")

    result={
        "schema_version":1,
        "artifact_kind":"5d_egb_spin2_dae_and_radial_constraint_structure",
        "status":"symbolically_derived_with_finite_grid_conditioning_probes",
        "input":{"equation_json_sha256":hashlib.sha256(args.equations.read_bytes()).hexdigest(),
                 "operator_keys":sorted(operators)},
        "radial_noether_identity":{
            "off_shell":"E_b*b' + E_f*f' + E_g*g' + E_h*h' + E_w*w' + 2*d(f*E_f)/dr = 0",
            "after_g_equals_r_squared":"2*r*E_g + b'*E_b + f'*E_f + h'*E_h + w'*E_w + 2*d(f*E_f)/dr = 0",
            "spin_squared":"2*r*E_g^(2) + F'*E_b^(2) + F'*E_f^(2) + 2*r*E_h^(2) + W1'*E_w^(1) + 2*d(F*E_f^(2))/dr = 0",
            "spin_squared_when_first_order_angular_equation_holds":"2*r*E_g^(2) + F'*E_b^(2) + F'*E_f^(2) + 2*r*E_h^(2) + 2*d(F*E_f^(2))/dr = 0",
            "off_shell_symbolic_zero":"verified by companion verify_radial_noether_identity.py",
            "radial_constraint_may_be_dropped_from_qualification":False},
        "differential_algebraic_structure":{
            "E_f_contains_F2_derivatives":False,
            "E_f_F2_coefficient":sp.sstr(pivot),
            "E_f_F2_coefficient_positive_decomposition":{
                "Q":"sqrt(alpha^2+2*alpha*r_H^2+r^4)",
                "Delta":"r^2+Q-alpha-2*r_H^2",
                "N":sp.sstr(N),
                "coefficient":"3*r*N/((r^2+Q)*Q*Delta^2)",
                "domain":"alpha>=0 and r>r_H>0",
                "proof_obligations":["Q>0",
                  "Delta(r_H)=0 and dDelta/dr=2*r+2*r^3/Q>0 for r>r_H",
                  "N is a sum of nonnegative monomials with strictly positive 4*Q*r^6+4*r^8 contribution"],
                "conclusion":"E_f determines F2 pointwise on the open exterior; the coefficient is singular as Delta^-2 at the horizon, so horizon Frobenius regularization is required."},
            "principal_derivative_columns":["B2''","F2''","H2''"],
            "raw_principal_matrix":[[sp.sstr(v) for v in row] for row in principal_raw.tolist()],
            "reduced_unknowns_after_Ef_elimination":["B2","H2"],
            "reduced_equations":["E_b","E_h"],
            "reduced_principal_matrix_alpha_zero":[[sp.sstr(v) for v in row] for row in einstein_matrix.tolist()],
            "reduced_principal_determinant_alpha_zero":sp.sstr(einstein_det),
            "reduced_principal_alpha_zero_nonsingular_for_r_gt_rH":True},
        "finite_alpha_exterior_probe":{
            "lambda_grid":lambdas,"rho_grid":rhos,"sample_count":len(samples),
            "minimum_reduced_principal_determinant":min_det,
            "maximum_reduced_principal_condition_number_2":max_cond,
            "minimum_Ef_F2_pivot":min_pivot,"samples":samples,
            "interpretation":"Binary64 floating-point conditioning probes only; not a global nonsingularity proof or interval enclosure."},
        "solver_design":{
            "recommended_formulation":"Treat the spin-squared equations as a differential-algebraic boundary problem: solve E_f algebraically for F2 in the open exterior, integrate E_b and E_h for B2 and H2, and monitor E_g using the radial Noether identity as an independent constraint residual.",
            "horizon_treatment":"Do not evaluate the algebraic quotient naively at r=r_H. Derive a regular horizon Frobenius series and match it to the open-exterior DAE formulation.",
            "conditioning_treatment":"Use horizon-relative/adapted coordinates and scaled dependent variables; the probe grid shows severe conditioning growth at large coupling and near the horizon.",
            "acceptance":"All four even equations plus E_w^(1), horizon data and asymptotic data remain qualification gates. Constraint reduction is a solver optimization, not permission to discard residuals."},
        "validation":{
            "symbolic_engine":"SymPy "+sp.__version__,"python":platform.python_version(),"numpy":np.__version__,
            "off_shell_radial_noether_identity_exact_zero":"companion artifact",
            "E_f_has_no_F2_derivatives":True,"E_f_pivot_matches_positive_closed_form":True,
            "Einstein_limit_reduced_principal_matrix_exact":True,
            "Einstein_limit_reduced_principal_determinant_exact":True},
        "claim_ceiling":"Equation-structure analysis only; no solved finite-alpha BVP, interval-certified determinant, exact nonlinear black-hole solution, or existence/uniqueness theorem."
    }
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else: print(rendered)

if __name__=="__main__": main()

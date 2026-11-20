#!/usr/bin/env python3
"""Standard-library control for non-symmetric generalized eigenvalue perturbation.

This is an algorithm control only; it is not a gravitational principal symbol.
"""
import json, math, platform, sys
from pathlib import Path

def mm(a,b):
    return [[sum(a[i][k]*b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]
def inv2(a):
    d=a[0][0]*a[1][1]-a[0][1]*a[1][0]
    if abs(d)<1e-14: raise ArithmeticError("singular 2x2 matrix")
    return [[a[1][1]/d,-a[0][1]/d],[-a[1][0]/d,a[0][0]/d]]
def eig2(a):
    tr=a[0][0]+a[1][1]
    det=a[0][0]*a[1][1]-a[0][1]*a[1][0]
    disc=tr*tr-4*det
    if disc<0: raise ArithmeticError("control expected a real, simple spectrum")
    root=math.sqrt(disc)
    return sorted(((tr-root)/2,(tr+root)/2))
def add_scaled(a,b,s):
    return [[a[i][j]+s*b[i][j] for j in range(2)] for i in range(2)]
def main(out=Path(".")):
    omega0=2.0
    A1=[[0.7,2.6],[0.0,0.2]]
    B1=[[0.1,0.3],[-0.05,0.2]]
    M=[[A1[i][j]-omega0*B1[i][j] for j in range(2)] for i in range(2)]
    R=[[1.0,2.0],[0.5,1.5]]
    L=inv2(R)
    Meff=mm(mm(L,M),R)
    first=eig2(M)
    transformed=eig2(Meff)
    basis_defect=max(abs(x-y) for x,y in zip(first,transformed))
    diagonal_original=sorted([M[0][0],M[1][1]])
    diagonal_transformed=sorted([Meff[0][0],Meff[1][1]])
    sweep=[]
    for eps in (0.1,0.05,0.025,0.0125):
        A=add_scaled([[omega0,0.0],[0.0,omega0]],A1,eps)
        B=add_scaled([[1.0,0.0],[0.0,1.0]],B1,eps)
        BinvA=mm(inv2(B),A)
        exact=eig2(BinvA)
        actual_shift=sorted([(w-omega0)/eps for w in exact])
        error=max(abs(x-y) for x,y in zip(actual_shift,first))
        sweep.append({
            "epsilon":eps,
            "generalized_eigenvalues":exact,
            "measured_first_order_shifts":actual_shift,
            "predicted_first_order_shifts":first,
            "shift_error":error,
            "absolute_eigenvalue_error_over_epsilon_squared":error/eps
        })
    max_scaled=max(x["absolute_eigenvalue_error_over_epsilon_squared"] for x in sweep)
    diagonal_error_original=max(abs(x-y) for x,y in zip(diagonal_original,first))
    diagonal_error_transformed=max(abs(x-y) for x,y in zip(diagonal_transformed,first))
    assert abs((L[0][0]*R[0][0]+L[0][1]*R[1][0])-1.0)<1e-14
    assert basis_defect<1e-12 and max_scaled<0.11
    assert diagonal_error_original>0.2 and diagonal_error_transformed>3.0
    fixture={
      "schema_version":1,
      "artifact_kind":"nonsymmetric_generalized_eigenvalue_perturbation_control",
      "status":"executed_generalized_pencil_method_control_not_gravitational_symbol",
      "pencil":{
        "problem":"A(epsilon) v = omega B(epsilon) v",
        "A0":"2 I","B0":"I","omega0":omega0,
        "A1":A1,"B1":B1,
        "first_order_effective_matrix":"L^T (A1 - omega0 B1) R",
        "canonical_effective_matrix":M,
        "right_basis_R":R,"left_basis_L_transpose":L,
        "biorthogonality":"L^T R = I",
        "transformed_effective_matrix":Meff
      },
      "first_order_shifts":first,
      "basis_covariance":{"transformed_shifts":transformed,"max_eigenvalue_defect":basis_defect},
      "diagonal_only_negative_control":{
        "canonical_basis_diagonal":diagonal_original,
        "nonorthogonal_basis_diagonal":diagonal_transformed,
        "max_shift_error_canonical_basis":diagonal_error_original,
        "max_shift_error_transformed_basis":diagonal_error_transformed,
        "basis_dependent_and_rejected":True
      },
      "epsilon_sweep":sweep,
      "summary":{
        "max_absolute_eigenvalue_error_over_epsilon_squared":max_scaled,
        "basis_covariance_defect":basis_defect,
        "diagonal_only_is_basis_dependent":True
      },
      "runtime":{"python":platform.python_version(),"dependencies":"stdlib only"},
      "claim_ceiling":"Generalized non-symmetric eigenvalue method control only; no EGB principal symbol, physical polarization quotient, symmetrizer, or rotating-background result."
    }
    out.mkdir(parents=True,exist_ok=True)
    (out/"nonsymmetric_generalized_characteristic_perturbation_fixture_v1.json").write_text(json.dumps(fixture,indent=2,sort_keys=True)+"\n")
    print(json.dumps(fixture["summary"],indent=2))
if __name__=="__main__":
    main(Path(sys.argv[1]) if len(sys.argv)>1 else Path("."))

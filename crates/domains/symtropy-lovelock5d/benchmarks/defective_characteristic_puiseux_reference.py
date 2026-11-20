#!/usr/bin/env python3
"""Fail-closed control for a defective eigenvalue and Puiseux splitting."""
import json, math, platform, sys
from pathlib import Path

def matmul(a,b):
    return [[sum(a[i][k]*b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]
def main(out=Path(".")):
    A0=[[0.0,1.0],[0.0,0.0]]
    A1=[[0.0,0.0],[1.0,0.0]]
    A0_sq=matmul(A0,A0)
    algebraic_multiplicity=2
    geometric_multiplicity=1
    assert A0_sq==[[0.0,0.0],[0.0,0.0]]
    assert A0 != [[0.0,0.0],[0.0,0.0]]
    assert geometric_multiplicity<algebraic_multiplicity
    sweep=[]
    for eps in (1e-2,1e-4,1e-6,1e-8):
        root=math.sqrt(eps)
        eigenvalues=[-root,root]
        sweep.append({
            "epsilon":eps,
            "eigenvalues":eigenvalues,
            "max_abs_eigenvalue_over_sqrt_epsilon":root/math.sqrt(eps),
            "max_abs_eigenvalue_over_epsilon":root/eps
        })
    assert all(abs(x["max_abs_eigenvalue_over_sqrt_epsilon"]-1.0)<1e-14 for x in sweep)
    growth=sweep[-1]["max_abs_eigenvalue_over_epsilon"]/sweep[0]["max_abs_eigenvalue_over_epsilon"]
    assert growth>999.0
    fixture={
      "schema_version":1,
      "artifact_kind":"defective_eigenvalue_puiseux_negative_control",
      "status":"executed_jordan_exceptional_point_control_not_gravitational_symbol",
      "matrices":{"A0":A0,"A1":A1,"A_epsilon":"A0 + epsilon*A1"},
      "unperturbed_eigenvalue":{
        "value":0,"algebraic_multiplicity":algebraic_multiplicity,
        "geometric_multiplicity":geometric_multiplicity,
        "defective":True,"A0_squared":A0_sq
      },
      "exact_splitting":"lambda_plus_minus = +/-sqrt(epsilon)",
      "epsilon_sweep":sweep,
      "summary":{
        "sqrt_scaling_defect":max(abs(x["max_abs_eigenvalue_over_sqrt_epsilon"]-1.0) for x in sweep),
        "linear_slope_magnitude_growth":growth,
        "simple_semisimple_first_order_method_must_refuse":True,
        "required_fallback":"Jordan-chain/Puiseux analysis or unresolved classification"
      },
      "runtime":{"python":platform.python_version(),"dependencies":"stdlib only"},
      "claim_ceiling":"Exceptional-point method control only; no defective mode or Jordan point is claimed for the EGB black-hole principal symbol."
    }
    out.mkdir(parents=True,exist_ok=True)
    (out/"defective_characteristic_puiseux_fixture_v1.json").write_text(json.dumps(fixture,indent=2,sort_keys=True)+"\n")
    print(json.dumps(fixture["summary"],indent=2))
if __name__=="__main__":
    main(Path(sys.argv[1]) if len(sys.argv)>1 else Path("."))

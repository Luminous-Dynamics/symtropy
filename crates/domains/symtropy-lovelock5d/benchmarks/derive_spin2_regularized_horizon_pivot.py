#!/usr/bin/env python3
"""Exact pivot-only horizon desingularization for the 5D EGB spin-2 DAE."""
from __future__ import annotations
import argparse, json, math, platform
from pathlib import Path
import sympy as sp


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    rho, lam = sp.symbols("rho lambda", positive=True, finite=True)
    Q = sp.sqrt(lam**2 + 2*lam + rho**4)
    D = rho**2 + Q - lam - 2
    N = (Q*(lam**3 + lam**2*rho**2 + 4*lam**2 + 2*lam*rho**2
            + 4*lam + 4*rho**6)
         + lam**3*rho**2 + 3*lam**2*rho**4 + 4*lam**2*rho**2
         + 6*lam*rho**4 + 4*lam*rho**2 + 4*rho**8)
    c = 3*rho*N/((rho**2+Q)*Q*D**2)  # c_F/r_H
    c_reg = sp.cancel(D**2*c)
    expected = 3*rho*N/((rho**2+Q)*Q)
    assert sp.cancel(c_reg-expected) == 0

    radical_h = sp.sqrt(lam**2+2*lam+1)
    def at_horizon(expr):
        return sp.factor(sp.simplify(expr.subs(rho,1)).subs(radical_h,lam+1))
    N_h = at_horizon(N)
    c0 = at_horizon(c_reg)
    c1 = at_horizon(sp.diff(c_reg,rho))
    assert sp.factor(N_h-(lam+1)*(lam+2)**3) == 0
    assert sp.factor(c0-3*(lam+2)**2) == 0
    c1_expected = 3*(lam+2)**2*(lam**2+4*lam+5)/(lam+1)**2
    assert sp.factor(c1-c1_expected) == 0

    d1 = 2*(lam+2)/(lam+1)
    d2 = 1+3/(lam+1)-2/(lam+1)**3
    lm2 = sp.factor(c0/d1**2)
    lm1 = sp.factor(c1/d1**2-2*c0*d2/d1**3)
    assert sp.factor(lm2-3*(lam+1)**2/4) == 0
    assert sp.factor(lm1-3) == 0

    # Deterministic binary64 smoke check of the rescaled coefficient alone.
    def c_reg_float(r, a):
        q=math.sqrt(a*a+2*a+r**4)
        n=(q*(a**3+a*a*r*r+4*a*a+2*a*r*r+4*a+4*r**6)
           +a**3*r*r+3*a*a*r**4+4*a*a*r*r+6*a*r**4+4*a*r*r+4*r**8)
        return 3*r*n/((r*r+q)*q)
    lambdas=[0.0,0.1,0.7,3.0,10.0,100.0]
    rhos=[1.0,1.000001,1.001,1.01,1.1,1.5,2.0,5.0,10.0,100.0]
    values=[c_reg_float(r,a) for a in lambdas for r in rhos]
    assert all(math.isfinite(v) and v>0 for v in values)

    fixture={
      "schema_version":1,
      "artifact_kind":"5d_egb_spin2_regularized_horizon_algebraic_pivot",
      "status":"exact_symbolic_pivot_rescaling_and_horizon_jet_derived",
      "source":{"parent_artifact":"spin2_constraint_structure_v1.json",
        "source_pivot":"c_F=3*r*N/((r^2+Q)*Q*Delta^2)",
        "dimensionless_variables":"rho=r/r_H, lambda=alpha/r_H^2"},
      "definitions":{"Q_bar":"sqrt(lambda^2+2*lambda+rho^4)",
        "Delta_bar":"rho^2+Q_bar-lambda-2",
        "dimensionless_original_pivot":"cbar_F=3*rho*N_bar/((rho^2+Q_bar)*Q_bar*Delta_bar^2)",
        "regularized_pivot":"cbar_reg=Delta_bar^2*cbar_F=3*rho*N_bar/((rho^2+Q_bar)*Q_bar)"},
      "exact_horizon_expansion":{"coordinate":"y=rho-1",
        "Q_bar_at_horizon":"lambda+1",
        "N_bar_at_horizon":"(lambda+1)*(lambda+2)^3",
        "Delta_bar":"2*(lambda+2)/(lambda+1)*y+(1+3/(lambda+1)-2/(lambda+1)^3)*y^2+O(y^3)",
        "regularized_pivot":"3*(lambda+2)^2+3*(lambda+2)^2*(lambda^2+4*lambda+5)/(lambda+1)^2*y+O(y^2)",
        "regularized_horizon_value":"3*(lambda+2)^2",
        "regularized_horizon_derivative":"3*(lambda+2)^2*(lambda^2+4*lambda+5)/(lambda+1)^2",
        "recovered_original_pivot":"3*(lambda+1)^2/(4*y^2)+3/y+O(1)"},
      "positivity":{"domain":"lambda>=0 and rho>=1","Q_positive":True,
        "Delta_positive_open_exterior":"Delta_bar(1)=0; derivative=2*rho+2*rho^3/Q_bar>0",
        "N_positive":"all terms nonnegative; 4*Q_bar*rho^6 and 4*rho^8 strictly positive",
        "regularized_pivot_positive":True,"horizon_pivot_nonzero":True},
      "numerical_smoke_grid":{"lambda_grid":lambdas,"rho_grid":rhos,
        "sample_count":len(values),"minimum_sampled_regularized_pivot":min(values),
        "interpretation":"Binary64 finite/positive smoke check for the rescaled coefficient only; not an interval enclosure."},
      "solver_implication":{"candidate_row_scaling":"Replace E_f^(2)=0 by Delta_bar^2*E_f^(2)=0 on rho>1; the F2 coefficient becomes cbar_reg.",
        "horizon_use":"The rescaled F2 coefficient has a finite, strictly positive horizon limit.",
        "mandatory_next_check":"Prove that the complete rescaled remainder Delta_bar^2*R_f and the coupled equations extend smoothly to rho=1 before using this as a horizon equation."},
      "validation":{"symbolic_engine":"SymPy "+sp.__version__,"python":platform.python_version(),
        "regularized_identity_exact":True,"horizon_value_exact":True,
        "horizon_derivative_exact":True,"recovered_laurent_coefficients_exact":True,
        "binary64_samples":len(values)},
      "claim_ceiling":"Exact desingularization of the algebraic F2 coefficient and exact recovery of its known Laurent terms only. No full-row smoothness, Frobenius recurrence, finite-alpha BVP, interval enclosure, or existence/uniqueness theorem is claimed."
    }
    output=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output: args.output.parent.mkdir(parents=True,exist_ok=True); args.output.write_text(output)
    else: print(output)

if __name__=="__main__": main()

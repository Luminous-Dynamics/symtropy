#!/usr/bin/env python3
"""Exact horizon-row desingularization for the 5D EGB spin-2 equations."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

META={
 "b":(18348,"cf3f62d0ea589e3dbfcc2ac4eedce259f5759cb1c372da03d45c2930ebc832ff",
      "47e9f330a3d13ad1527dc8acccdc2e2b670495b144799753b0764bf99c53b1f8"),
 "f":(8174,"a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa",
      "31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"),
 "g":(41842,"9a0a56db534806aef37b42c41895d5dd54ab00586ea72e77d6735c64fcba015c",
      "8a6b1ee6df529fa2be2009a0757db2caca3aeb086f0ba243e154019e6949dbee"),
 "h":(38349,"6b874e5b02dc1a7314242a42ba39275f6ab93f5031d652b516ccc4db8567dc0d",
      "5dcbb3e55cd409b5a34bf15f90064db616076716c5fa7352a514acef11ac78e1")}
def sha(s): return hashlib.sha256(s.encode()).hexdigest()

def main():
 ap=argparse.ArgumentParser()
 ap.add_argument("--equations",type=Path,required=True)
 ap.add_argument("--output",type=Path,required=True)
 args=ap.parse_args()
 data=json.loads(args.equations.read_text())
 raw=data["equation_system"]["equations"]
 r,rh,a=sp.symbols("r r_H alpha",positive=True,finite=True)
 rho,lam,Q=sp.symbols("rho lambda Q",positive=True,finite=True)
 Bf,Ff,Hf=(sp.Function(n) for n in ("B2","F2","H2"))
 loc={"r":r,"r_H":rh,"alpha":a,"B2":Bf,"F2":Ff,"H2":Hf,"sqrt":sp.sqrt,"Derivative":sp.Derivative}
 parsed={k:sp.sympify(raw[k],locals=loc) for k in ("b","f","g","h")}
 B,Bp,Bpp,F,Fp,H,Hp,Hpp=sp.symbols("B Bp Bpp F Fp H Hp Hpp")
 jm={Bf(r):B,sp.diff(Bf(r),r):Bp,sp.diff(Bf(r),r,2):Bpp,
     Ff(r):F,sp.diff(Ff(r),r):Fp,Hf(r):H,sp.diff(Hf(r),r):Hp,sp.diff(Hf(r),r,2):Hpp}
 rad=sp.sqrt(1+a*(a+2*rh**2)/r**4)
 sub={rh:1,a:lam,r:rho,rad:Q/rho**2}
 E={}
 for k,e in parsed.items():
  n,srepr,txt=META[k]
  assert len(raw[k])==n and data["equation_system"]["equation_forms"][k]["equation_sha256"]==srepr and sha(raw[k])==txt
  E[k]=e.xreplace(jm).subs(sub)
 delta=rho**2+Q-lam-2
 S=sp.Symbol("S",positive=True)
 z=sp.symbols("z0:8")
 jets=(B,Bp,Bpp,F,Fp,H,Hp,Hpp)
 jet_names=("B2","B2_prime","B2_second","F2","F2_prime","H2","H2_prime","H2_second")
 extra={"b":2,"g":1,"h":1}
 den_expected={
  "b":2*Q**2*rho**3*(Q+rho**2)**6,
  "g":Q**4*rho**5*(Q+rho**2)**8,
  "h":2*Q**3*rho**5*(Q+rho**2)**7}
 rows={}
 for k in ("b","g","h"):
  bases={p.base for p in E[k].atoms(sp.Pow) if not p.exp.is_Integer}
  assert len(bases)==1
  base=next(iter(bases))
  assert sp.simplify(base-(1-(lam+2)/(Q+rho**2)))==0
  ex=E[k].xreplace({base:S})
  terms=sp.Add.make_args(sp.expand_mul(ex))
  powers={term.as_powers_dict().get(S,sp.Integer(0)) for term in terms}
  assert powers.issubset({sp.Integer(-2),sp.Integer(-1),sp.Integer(0),sp.Integer(1)})
  f2row=sp.Add(*(term*S**2 for term in terms))
  f2row=f2row.xreplace({S:delta/(Q+rho**2)})
  row=sp.cancel(sp.together(delta**extra[k]*f2row))
  assert all(p.exp.is_Integer for p in row.atoms(sp.Pow))
  rz=row.xreplace(dict(zip(jets,z)))
  num,den=sp.together(rz).as_numer_denom()
  den=sp.factor(den)
  assert sp.cancel(den-den_expected[k])==0
  poly=sp.Poly(num,*z,domain="EX")
  expected_degrees={"b":7,"g":9,"h":7}
  assert poly.total_degree()==1 and len(poly.monoms())==expected_degrees[k]
  # All coefficient values of the complete scaled row vanish at the horizon.
  horizon_coefficients=sp.Poly(num.subs({rho:1,Q:lam+1}),*z,domain="EX")
  if any(sp.cancel(c)!=0 for c in horizon_coefficients.coeffs()):
   raise AssertionError(f"{k}: a scaled-row coefficient is nonzero at the horizon")

  # Exact Myers-Perry positive control and wrong-sign F2 negative control.
  mpB=-rho**-2+rho**-6
  mpF=-rho**-2+rho**-4
  mpH=rho**-2
  mp={lam:0,Q:rho**2,B:mpB,Bp:sp.diff(mpB,rho),Bpp:sp.diff(mpB,rho,2),
      F:mpF,Fp:sp.diff(mpF,rho),H:mpH,Hp:sp.diff(mpH,rho),Hpp:sp.diff(mpH,rho,2)}
  assert sp.simplify(sp.cancel(row.subs(mp)))==0
  bad={**mp,F:-mpF,Fp:-sp.diff(mpF,rho)}
  assert sp.simplify(sp.cancel(row.subs(bad)))!=0
  rows[k]={
   "row_scaling":f"F0^2*Delta_bar^{extra[k]}*E_{k}^(2)",
   "equivalent_multiplier":f"Delta_bar^{2+extra[k]}/(Qbar+rho^2)^2",
   "common_denominator":sp.sstr(den),
   "affine_jet_names":list(jet_names),
   "horizon_row_value":"identically zero coefficient-by-coefficient",
   "positive_denominator_factors":["Qbar","rho","Qbar+rho^2"],
   "jet_polynomial_degree":poly.total_degree(),
   "jet_monomial_count":len(poly.monoms()),
   "myers_perry_exact_zero":True,
   "wrong_sign_F2_negative_control_rejected":True}

 fixture={
  "schema_version":1,
  "artifact_kind":"5d_egb_spin2_all_even_rows_horizon_regularization",
  "status":"all_four_even_equations_have_analytic_horizon_row_forms",
  "definitions":{"rho":"r/r_H","lambda":"alpha/r_H^2",
    "Qbar":"sqrt(lambda^2+2*lambda+rho^4)","Delta_bar":"rho^2+Qbar-lambda-2",
    "F0":"Delta_bar/(Qbar+rho^2)",
    "Delta_bar_positive":"Delta_bar(1)=0; derivative=2*rho+2*rho^3/Qbar>0"},
  "row_scaling":{"E_b":"F0^2*Delta_bar^2*E_b^(2)",
    "E_f":"Delta_bar^2*E_f^(2) (verified separately in spin2_scaled_ef_horizon_row_v1.json)",
    "E_g":"F0^2*Delta_bar*E_g^(2)","E_h":"F0^2*Delta_bar*E_h^(2)",
    "all_multipliers_positive_open_exterior":True},
  "regularized_rows":rows,
  "analyticity_certificate":{
    "denominators":{k:v["common_denominator"] for k,v in rows.items()},
    "no_Delta_bar_denominator":True,
    "no_fractional_powers_after_scaling":True,
    "all_rows_affine_in_field_jets":True,
    "all_coefficient_values_vanish_at_horizon":True,
    "reason":"Qbar is positive real-analytic on rho>=1, lambda>=0; all denominator factors Qbar, rho, Qbar+rho^2 are strictly positive. Thus each complete scaled row is real-analytic through the horizon for analytic fields."},
  "source":{"equation_generator":"derive_slow_rotation_second_order.py --emit-equations",
    "equation_text_sha256":{k:META[k][2] for k in META},
    "equation_srepr_sha256":{k:META[k][1] for k in META},
    "python":platform.python_version(),"sympy":sp.__version__,
    "provenance":"Local equation emission is a SymPy reconstruction matching the public equation fingerprints. Byte-for-byte clean-checkout execution of the public generator remains outstanding."},
  "validation":{"source_equation_fingerprints_match":True,
    "full_scaled_rows_affine_degree_one":True,"exact_positive_denominators":True,
    "horizon_coefficient_values_exact_zero":True,
    "exact_myers_perry_controls_for_b_g_h":True,
    "wrong_sign_F2_negative_controls_rejected":True},
  "claim_ceiling":"This derives analytic extensions and exact controls for E_b,E_g,E_h after row scaling. The coupled horizon Frobenius recurrence, finite-alpha BVP, interval enclosure, and existence/uniqueness theorem remain open."
 }
 args.output.parent.mkdir(parents=True,exist_ok=True)
 args.output.write_text(json.dumps(fixture,indent=2,sort_keys=True)+"\n")
if __name__=="__main__": main()

#!/usr/bin/env python3
"""Exact lambda=0 spot check for the scaled E_h cubic horizon coefficient."""
from __future__ import annotations
import argparse, hashlib, json, platform, time
from pathlib import Path
import sympy as sp

META = {
 "b":(18348,"cf3f62d0ea589e3dbfcc2ac4eedce259f5759cb1c372da03d45c2930ebc832ff","47e9f330a3d13ad1527dc8acccdc2e2b670495b144799753b0764bf99c53b1f8"),
 "f":(8174,"a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa","31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"),
 "g":(41842,"9a0a56db534806aef37b42c41895d5dd54ab00586ea72e77d6735c64fcba015c","8a6b1ee6df529fa2be2009a0757db2caca3aeb086f0ba243e154019e6949dbee"),
 "h":(38349,"6b874e5b02dc1a7314242a42ba39275f6ab93f5031d652b516ccc4db8567dc0d","5dcbb3e55cd409b5a34bf15f90064db616076716c5fa7352a514acef11ac78e1")}
def main():
 ap=argparse.ArgumentParser()
 ap.add_argument("--equations",type=Path,required=True)
 ap.add_argument("--output",type=Path,required=True)
 args=ap.parse_args(); start=time.perf_counter()
 input_bytes=args.equations.read_bytes(); data=json.loads(input_bytes)
 raw=data["equation_system"]["equations"]
 for key,(nchars,srepr_hash,text_hash) in META.items():
  if len(raw[key])!=nchars or data["equation_system"]["equation_forms"][key]["equation_sha256"]!=srepr_hash or hashlib.sha256(raw[key].encode()).hexdigest()!=text_hash:
   raise AssertionError(f"{key}: source equation fingerprint mismatch")
 r,rh,a,rho=sp.symbols("r r_H alpha rho",positive=True,finite=True)
 Bf,Ff,Hf=(sp.Function(n) for n in ("B2","F2","H2"))
 B,Bp,Bpp,F,Fp,H,Hp,Hpp=sp.symbols("B Bp Bpp F Fp H Hp Hpp")
 jets=(B,Bp,Bpp,F,Fp,H,Hp,Hpp)
 expr=sp.sympify(raw["h"],locals={"r":r,"r_H":rh,"alpha":a,"B2":Bf,"F2":Ff,"H2":Hf,"sqrt":sp.sqrt,"Derivative":sp.Derivative})
 jetmap={Bf(r):B,sp.diff(Bf(r),r):Bp,sp.diff(Bf(r),r,2):Bpp,Ff(r):F,sp.diff(Ff(r),r):Fp,Hf(r):H,sp.diff(Hf(r),r):Hp,sp.diff(Hf(r),r,2):Hpp}
 expr=expr.xreplace(jetmap)
 radical=sp.sqrt(1+a*(a+2*rh**2)/r**4)
 expr=expr.subs({rh:1,a:0,r:rho,radical:1})
 bases={p.base for p in expr.atoms(sp.Pow) if not p.exp.is_Integer}
 if len(bases)!=1: raise AssertionError(f"expected one fractional-power base, got {bases}")
 base=next(iter(bases))
 if sp.simplify(base-(1-rho**-2))!=0: raise AssertionError("unexpected lambda=0 fractional-power base")
 S=sp.Symbol("S",positive=True); expanded=expr.xreplace({base:S})
 terms=sp.Add.make_args(sp.expand_mul(expanded))
 powers={term.as_powers_dict().get(S,sp.Integer(0)) for term in terms}
 if not powers.issubset({sp.Integer(-2),sp.Integer(-1),sp.Integer(0),sp.Integer(1)}): raise AssertionError(f"unexpected fractional powers {powers}")
 delta=2*(rho**2-1); f0=1-rho**-2
 scaled=sp.Add(*(term*S**2 for term in terms)).xreplace({S:f0})
 row=sp.cancel(sp.together(delta*scaled))
 z=sp.symbols("z0:8")
 numerator,denominator=sp.together(row.xreplace(dict(zip(jets,z)))).as_numer_denom()
 polynomial=sp.Poly(numerator,*z,domain="EX")
 if polynomial.total_degree()!=1 or len(polynomial.monoms())!=7: raise AssertionError("unexpected scaled E_h affine jet structure")
 mons={"B":(1,0,0,0,0,0,0,0),"Bp":(0,1,0,0,0,0,0,0),"Bpp":(0,0,1,0,0,0,0,0),
       "F":(0,0,0,1,0,0,0,0),"Fp":(0,0,0,0,1,0,0,0),"H":(0,0,0,0,0,1,0,0),
       "Hp":(0,0,0,0,0,0,1,0),"Hpp":(0,0,0,0,0,0,0,1),"source":(0,)*8}
 series={}
 for m,c in zip(polynomial.monoms(),polynomial.coeffs()):
  coeff=sp.cancel(c/denominator)
  series[m]=[sp.factor(sp.diff(coeff,rho,n).subs(rho,1)/sp.factorial(n)) for n in range(4)]
 b1,b2,b3,b4,b5,f1,f2,f3,f4,h0,h1,h2,h3,h4,h5=sp.symbols("b1 b2 b3 b4 b5 f1 f2 f3 f4 h0 h1 h2 h3 h4 h5")
 fields={"B":[0,b1,b2/2,b3/6],"Bp":[b1,b2,b3/2,b4/6],"Bpp":[b2,b3,b4/2,b5/6],
  "F":[0,f1,f2/2,f3/6],"Fp":[f1,f2,f3/2,f4/6],"H":[h0,h1,h2/2,h3/6],
  "Hp":[h1,h2,h3/2,h4/6],"Hpp":[h2,h3,h4/2,h5/6]}
 def c(name,n): return series.get(mons[name],(0,0,0,0))[n]
 total=[]
 for n in range(4):
  value=sp.Integer(0)
  for name in fields:
   for j in range(n+1): value += c(name,j)*fields[name][n-j]
  value += c("source",n)
  total.append(sp.factor(value))
 if any(sp.simplify(total[n])!=0 for n in (0,1,2)): raise AssertionError(f"lower horizon coefficients nonzero: {total[:3]}")
 actual=sp.factor(total[3])
 expected=sp.Integer(2)*(-sp.Integer(9)*b1-sp.Integer(3)*b2-sp.Integer(7)*f1-f2-sp.Integer(24)*h0+sp.Integer(96))
 if sp.factor(actual-expected)!=0: raise AssertionError(f"cubic formula mismatch: {sp.factor(actual-expected)}")
 high=(b3,b4,b5,f3,f4,h1,h2,h3,h4,h5)
 if any(actual.has(v) for v in high): raise AssertionError("an omitted higher Taylor jet remains in the cubic coefficient")
 mp={b1:-4,b2:36,f1:-2,f2:14,h0:1}
 if sp.simplify(actual.subs(mp))!=0: raise AssertionError("Myers-Perry positive control failed")
 bad={**mp,f1:2}
 if sp.simplify(actual.subs(bad))==0: raise AssertionError("wrong-sign F2-prime negative control passed")
 fixture={"schema_version":1,"artifact_kind":"5d_egb_spin2_Eh_cubic_lambda0_spotcheck",
  "status":"exact_lambda0_cubic_identity_and_higher_jet_cancellation_passed",
  "equation_json_sha256":hashlib.sha256(input_bytes).hexdigest(),"source_equation_fingerprints_match_public_metadata":True,
  "row":"E_h^(2)","coupling":"lambda=0","row_scaling":"F0^2*Delta_bar*E_h^(2)",
  "scaled_row_common_denominator":sp.sstr(sp.factor(denominator)),"affine_monomials":len(polynomial.monoms()),
  "orders_0_1_2_vanish_under_fixed_horizon_data":True,"cubic_actual":sp.sstr(actual),"cubic_candidate":sp.sstr(expected),
  "cubic_formula_matches":True,"omitted_higher_jets_cancel":True,"myers_perry_positive_control":True,
  "wrong_sign_F2_prime_negative_control_rejected":True,
  "execution":{"python":platform.python_version(),"sympy":sp.__version__,"elapsed_seconds":round(time.perf_counter()-start,6),"arithmetic":"exact rational symbolic arithmetic"},
  "provenance_boundary":"Input equations must be emitted from the action-level generator; this verifier checks all four equation lengths, symbolic fingerprints, and emitted equation text SHA-256 values. A clean-checkout generator replay is a separate gate.",
  "claim_ceiling":"Exact E_h cubic formula spot check at lambda=0 only. No general-lambda identity, complete recurrence, boundary index, finite-coupling BVP, interval enclosure, or existence theorem is established."}
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(fixture,indent=2,sort_keys=True)+"\n")
 print(json.dumps(fixture,indent=2,sort_keys=True))
if __name__=="__main__": main()

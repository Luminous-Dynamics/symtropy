#!/usr/bin/env python3
"""Exact rational spot checks for the scaled E_g cubic horizon coefficient."""
from __future__ import annotations
import argparse, hashlib, json, platform, time
from pathlib import Path
import sympy as sp

META = {
 "b": (18348,"cf3f62d0ea589e3dbfcc2ac4eedce259f5759cb1c372da03d45c2930ebc832ff","47e9f330a3d13ad1527dc8acccdc2e2b670495b144799753b0764bf99c53b1f8"),
 "f": (8174,"a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa","31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"),
 "g": (41842,"9a0a56db534806aef37b42c41895d5dd54ab00586ea72e77d6735c64fcba015c","8a6b1ee6df529fa2be2009a0757db2caca3aeb086f0ba243e154019e6949dbee"),
 "h": (38349,"6b874e5b02dc1a7314242a42ba39275f6ab93f5031d652b516ccc4db8567dc0d","5dcbb3e55cd409b5a34bf15f90064db616076716c5fa7352a514acef11ac78e1")}
def main():
 ap=argparse.ArgumentParser()
 ap.add_argument("--equations",type=Path,required=True)
 ap.add_argument("--output",type=Path,required=True)
 args=ap.parse_args()
 start=time.perf_counter()
 input_bytes=args.equations.read_bytes()
 data=json.loads(input_bytes); raw=data["equation_system"]["equations"]
 for key,(nchars,srepr_hash,text_hash) in META.items():
  if len(raw[key])!=nchars or data["equation_system"]["equation_forms"][key]["equation_sha256"]!=srepr_hash or hashlib.sha256(raw[key].encode()).hexdigest()!=text_hash:
   raise AssertionError(f"{key}: source equation fingerprint mismatch")
 r,rh,a=sp.symbols("r r_H alpha",positive=True,finite=True)
 rho,lam,Q=sp.symbols("rho lambda Q",positive=True,finite=True)
 Bf,Ff,Hf=(sp.Function(n) for n in ("B2","F2","H2"))
 local={"r":r,"r_H":rh,"alpha":a,"B2":Bf,"F2":Ff,"H2":Hf,"sqrt":sp.sqrt,"Derivative":sp.Derivative}
 B,Bp,Bpp,F,Fp,H,Hp,Hpp=sp.symbols("B Bp Bpp F Fp H Hp Hpp")
 jets=(B,Bp,Bpp,F,Fp,H,Hp,Hpp)
 jetmap={Bf(r):B,sp.diff(Bf(r),r):Bp,sp.diff(Bf(r),r,2):Bpp,Ff(r):F,sp.diff(Ff(r),r):Fp,Hf(r):H,sp.diff(Hf(r),r):Hp,sp.diff(Hf(r),r,2):Hpp}
 rad=sp.sqrt(1+a*(a+2*rh**2)/r**4)
 expr=sp.sympify(raw["g"],locals=local).xreplace(jetmap).subs({rh:1,a:lam,r:rho,rad:Q/rho**2})
 S=sp.Symbol("S",positive=True); delta=rho**2+Q-lam-2
 bases={p.base for p in expr.atoms(sp.Pow) if not p.exp.is_Integer}
 if len(bases)!=1: raise AssertionError(f"expected one fractional-power base, got {bases}")
 base=next(iter(bases))
 if sp.simplify(base-(1-(lam+2)/(Q+rho**2)))!=0: raise AssertionError("unexpected E_g fractional-power base")
 expanded=expr.xreplace({base:S})
 terms=sp.Add.make_args(sp.expand_mul(expanded))
 powers={term.as_powers_dict().get(S,sp.Integer(0)) for term in terms}
 if not powers.issubset({sp.Integer(-2),sp.Integer(-1),sp.Integer(0),sp.Integer(1)}):
  raise AssertionError(f"unexpected fractional powers: {powers}")
 scaled=sp.Add(*(term*S**2 for term in terms)).xreplace({S:delta/(Q+rho**2)})
 row=sp.cancel(sp.together(delta*scaled))
 z=sp.symbols("z0:8")
 num,den=sp.together(row.xreplace(dict(zip(jets,z)))).as_numer_denom()
 den=sp.factor(den); poly=sp.Poly(num,*z,domain="EX")
 if poly.total_degree()!=1 or len(poly.monoms())!=9: raise AssertionError("unexpected scaled E_g jet structure")
 mon={name:tuple(1 if j==i else 0 for j in range(8)) for i,name in enumerate(["B","Bp","Bpp","F","Fp","H","Hp","Hpp"])}
 mon["S"]=(0,)*8
 b1,b2,b3,b4,b5,f1,f2,f3,f4,h0,h1,h2,h3,h4,h5=sp.symbols("b1 b2 b3 b4 b5 f1 f2 f3 f4 h0 h1 h2 h3 h4 h5")
 Pb=3*lam**4+6*lam**3-6*lam**2-18*lam-9
 Pf=3*lam**4+6*lam**3+10*lam**2+14*lam+7
 candidate=2*(lam+2)/(lam+1)**6*(Pb*b1-3*(lam+1)**4*b2-Pf*f1-(lam+1)**4*f2+8*(3*lam**2+8*lam+3)*h0-8*(lam+1)*h1+8*(3*lam**4+18*lam**3+37*lam**2+28*lam+4))
 results=[]
 coeff_expr={m:c for m,c in zip(poly.monoms(),poly.coeffs())}
 for lv in (sp.Rational(0),sp.Rational(1,2)):
  q0=lv+1; q1=sp.Rational(2,1)/q0; q2=6/q0-4/q0**3; q3=12/q0-36/q0**3+24/q0**5
  point={rho:1,Q:q0}; d=den.subs(lam,lv)
  d0=d.subs(point); d1=(sp.diff(d,rho)+q1*sp.diff(d,Q)).subs(point)
  d2=(sp.diff(d,rho,2)+2*q1*sp.diff(d,rho,Q)+q1**2*sp.diff(d,Q,2)+q2*sp.diff(d,Q)).subs(point)
  d3=(sp.diff(d,rho,3)+3*q1*sp.diff(d,rho,2,Q)+3*q1**2*sp.diff(d,rho,Q,2)+q1**3*sp.diff(d,Q,3)+3*q2*sp.diff(d,rho,Q)+3*q1*q2*sp.diff(d,Q,2)+q3*sp.diff(d,Q)).subs(point)
  ds=[d0,d1,d2/2,d3/6]; series={}
  for m,c in coeff_expr.items():
   c=c.subs(lam,lv); n0=c.subs(point)
   n1=(sp.diff(c,rho)+q1*sp.diff(c,Q)).subs(point)
   n2=(sp.diff(c,rho,2)+2*q1*sp.diff(c,rho,Q)+q1**2*sp.diff(c,Q,2)+q2*sp.diff(c,Q)).subs(point)
   n3=(sp.diff(c,rho,3)+3*q1*sp.diff(c,rho,2,Q)+3*q1**2*sp.diff(c,rho,Q,2)+q1**3*sp.diff(c,Q,3)+3*q2*sp.diff(c,rho,Q)+3*q1*q2*sp.diff(c,Q,2)+q3*sp.diff(c,Q)).subs(point)
   ns=[n0,n1,n2/2,n3/6]
   a0=sp.cancel(ns[0]/ds[0]); a1=sp.cancel((ns[1]-ds[1]*a0)/ds[0])
   a2=sp.cancel((ns[2]-ds[1]*a1-ds[2]*a0)/ds[0])
   a3=sp.cancel((ns[3]-ds[1]*a2-ds[2]*a1-ds[3]*a0)/ds[0])
   series[m]=(a0,a1,a2,a3)
  def c(m,n): return series.get(m,(0,0,0,0))[n]
  actual=(c(mon["B"],0)*b3/6+c(mon["B"],1)*b2/2+c(mon["B"],2)*b1
   +c(mon["Bp"],0)*b4/6+c(mon["Bp"],1)*b3/2+c(mon["Bp"],2)*b2+c(mon["Bp"],3)*b1
   +c(mon["Bpp"],0)*b5/6+c(mon["Bpp"],1)*b4/2+c(mon["Bpp"],2)*b3+c(mon["Bpp"],3)*b2
   +c(mon["F"],0)*f3/6+c(mon["F"],1)*f2/2+c(mon["F"],2)*f1
   +c(mon["Fp"],0)*f4/6+c(mon["Fp"],1)*f3/2+c(mon["Fp"],2)*f2+c(mon["Fp"],3)*f1
   +c(mon["H"],0)*h3/6+c(mon["H"],1)*h2/2+c(mon["H"],2)*h1+c(mon["H"],3)*h0
   +c(mon["Hp"],0)*h4/6+c(mon["Hp"],1)*h3/2+c(mon["Hp"],2)*h2+c(mon["Hp"],3)*h1
   +c(mon["Hpp"],0)*h5/6+c(mon["Hpp"],1)*h4/2+c(mon["Hpp"],2)*h3+c(mon["Hpp"],3)*h2
   +c(mon["S"],3))
  diff=sp.factor(actual-candidate.subs(lam,lv))
  if diff!=0: raise AssertionError(f"lambda={lv}: cubic mismatch {sp.sstr(diff)}")
  high=(b3,b4,b5,f3,f4,h2,h3,h4,h5)
  if any(actual.has(v) for v in high): raise AssertionError(f"lambda={lv}: omitted high jets remain")
  results.append({"row":"g","lambda":str(lv),"candidate_match":True,"higher_jets_cancel":True})
 fixture={"schema_version":1,"artifact_kind":"5d_egb_spin2_Eg_cubic_rational_spotchecks",
  "status":"exact_rational_specializations_passed","input_equation_json_sha256":hashlib.sha256(input_bytes).hexdigest(),
  "source_equation_fingerprints_checked":True,"checks":results,
  "execution":{"python":platform.python_version(),"sympy":sp.__version__,"elapsed_seconds":round(time.perf_counter()-start,6),"arithmetic":"exact rational symbolic differentiation"},
  "provenance_boundary":"Input equation JSON must be emitted from the action-level generator; this verifier checks the four equation lengths, symbolic fingerprints, and emitted text SHA-256 values. A clean-checkout generator replay is a separate gate.",
  "claim_ceiling":"Exact E_g cubic formula spot checks at lambda=0 and 1/2 only, with omitted higher Taylor jets cancelling in those specializations. No general-lambda identity, full recurrence, finite-coupling BVP, boundary index, interval enclosure, or existence theorem is established."}
 args.output.parent.mkdir(parents=True,exist_ok=True); args.output.write_text(json.dumps(fixture,indent=2,sort_keys=True)+"\n")
 print(json.dumps(fixture,indent=2,sort_keys=True))

if __name__=="__main__": main()

#!/usr/bin/env python3
"""Exact equal-spin MP kinematics from Ma-Lu JHEP version of record."""
from __future__ import annotations
import argparse,hashlib,json,platform
from pathlib import Path
import sympy as sp
def zero(k,e):
    e=sp.factor(sp.cancel(e))
    if e!=0: raise AssertionError(f"{k}: {sp.sstr(e)}")
def derive():
    q,nu,chi=sp.symbols("q nu chi",positive=True)
    ag,ar,T=sp.symbols("alpha_GB alpha_R T")
    mu=(1+q**2)**2/2
    nuq=q/(1+q**2)                 # Eq. (2.2)
    chq=sp.sqrt(2)*q/(1+q**2)     # Eq. (2.9): chi=a/sqrt(mu)
    c=sp.symbols("c0:6"); qs=sum(c[n]*nu**(2*n+1) for n in range(6))
    rr=sp.series(qs/(1+qs**2)-nu,nu,0,12).removeO().expand()
    sols=sp.solve([rr.coeff(nu,2*n+1) for n in range(6)],c,dict=True)
    if len(sols)!=1: raise AssertionError("inverse q series is not unique")
    qnu=sp.expand(qs.subs(sols[0]))
    zero("inverse q(nu)",sp.series(qnu/(1+qnu**2)-nu,nu,0,12).removeO())
    zero("chi=sqrt(2)*nu through nu^11",sp.series(chq.subs(q,qnu)-sp.sqrt(2)*nu,nu,0,12).removeO())
    qchi=chi/sp.sqrt(2)+chi**3/(2*sp.sqrt(2))+chi**5/(2*sp.sqrt(2))+5*chi**7/(8*sp.sqrt(2))
    mus=sp.series(mu.subs(q,qchi),chi,0,8).removeO()
    zero("Eq3.9 equal-spin mass",sp.series(mus-(sp.Rational(1,2)+chi**2/2+sp.Rational(5,8)*chi**4+sp.Rational(7,8)*chi**6),chi,0,8).removeO())
    sol=sp.solve(sp.Eq(ag*T/4,ar*T),ar)
    if sol != [ag/4]: raise AssertionError(sol)
    return {"schema_version":1,"artifact_kind":"ma_lu_equal_spin_parameter_map",
      "status":"exact_symbolic_version_of_record_relations_derived",
      "source":{"paper":"Ma and Lu, JHEP 05 (2026) 081","doi":"10.1007/JHEP05(2026)081","published":"2026-05-08","equations":["(2.1)","(2.2)","(2.9)","(3.9)"]},
      "definitions":{"q":"a/r_h","nu_h":"Omega_H*r_h=q/(1+q^2)","chi":"a/sqrt(mu)","mu_over_rh2":"(1+q^2)^2/2"},
      "exact":{"mu_over_rh2":sp.sstr(mu),"nu_h_of_q":sp.sstr(nuq),"chi_of_q":sp.sstr(chq),"chi_equals_sqrt2_nu_h":"exact"},
      "series":{"q_of_nu_h_through_nu11":sp.sstr(qnu),"chi_of_nu_h":"sqrt(2)*nu_h","mu_over_rh2_of_chi":"1/2+chi^2/2+5chi^4/8+7chi^6/8+O(chi^8)"},
      "coupling":{"alpha_R":"alpha_GB/4","scope":"first-order source identity on Ricci-flat MP only"},
      "checks":{"inverse_q_through_nu11":True,"chi_sqrt2_nu_exact":True,"Eq3_9_mass_through_chi6":True,"action_normalization":True},
      "claim_ceiling":"Exact equal-spin MP kinematics and first-coupling normalization only; no corrected metric or finite-coupling branch is claimed."}
def main():
    p=argparse.ArgumentParser(); p.add_argument("--output",type=Path); a=p.parse_args()
    o=derive(); o["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    o["environment"]={"python":platform.python_version(),"sympy":sp.__version__}
    s=json.dumps(o,indent=2,sort_keys=True)+"\n"
    if a.output: a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(s,encoding="utf-8")
    print(s)
if __name__=="__main__": main()

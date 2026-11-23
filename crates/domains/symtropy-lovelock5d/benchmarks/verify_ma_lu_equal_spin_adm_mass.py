#!/usr/bin/env python3
"""Exact equal-spin ADM mass cross-check against Ma-Lu JHEP Eq. (3.4)."""
from __future__ import annotations
import argparse,hashlib,json,platform
from pathlib import Path
import sympy as sp

def zero(label,expr):
    value=sp.factor(sp.cancel(expr))
    if value!=0: raise AssertionError(f"{label}: {sp.sstr(value)}")

def derive():
    r,mu,chi,alpha=sp.symbols('r mu chi alpha', positive=True)
    H1=(sp.Rational(16,3)/r**2-sp.Rational(16,3)*mu/r**4-sp.Rational(32,3)*mu**2/r**6
        -sp.Rational(2,45)*chi**2*(282/r**2-162*mu/r**4-356*mu**2/r**6-1076*mu**3/r**8))
    h_minus2=sp.limit(r**2*H1,r,sp.oo)
    deltaM_metric=sp.Rational(3,8)*sp.pi*h_minus2
    deltaM_paper=2*sp.pi-sp.Rational(47,10)*sp.pi*chi**2
    zero('Eq. (3.4) ADM mass through chi^2',deltaM_metric-deltaM_paper)
    H7=sp.Rational(8,3)/r**2+sp.Rational(4,3)*mu/r**4-sp.Rational(2,45)*chi**2*(141/r**2+48*mu/r**4-22*mu**2/r**6)
    R2=r**2+sp.Symbol('a2')+alpha*r**2*H7
    radial_ratio=sp.limit(r**2/R2,r,sp.oo)
    zero('radial map preserves leading inverse-square coefficient',radial_ratio-1)
    bad=sp.Rational(3,8)*sp.pi*sp.Rational(16,3)
    assert sp.simplify(bad-deltaM_paper)!=0
    return {'schema_version':1,'artifact_kind':'ma_lu_equal_spin_adm_mass_asymptotic_check',
      'status':'exact_equal_spin_ADM_mass_match_through_chi_squared',
      'source':{'paper':'Ma and Lu, JHEP 05 (2026) 081','doi':'10.1007/JHEP05(2026)081','equations':['(2.8)','(2.12)','(3.3)','(3.4)']},
      'asymptotic':{'H1':'16/(3*r^2)-16*mu/(3*r^4)-32*mu^2/(3*r^6)-(2/45)*chi^2*(282/r^2-162*mu/r^4-356*mu^2/r^6-1076*mu^3/r^8)',
        'coefficient_1_over_r2':'16/3-188*chi^2/15','Komar_mass_prefactor':'3*pi/8',
        'metric_deltaM':'2*pi-47*pi*chi^2/10+O(chi^4)','paper_deltaM':'2*pi-47*pi*chi^2/10+O(chi^4)'},
      'checks':{'metric_mass_equals_Eq3_4_through_chi2':True,'radial_map_preserves_leading_1_over_r2_coefficient':True,'negative_control_omit_chi2_H1_rejected':True},
      'claim_ceiling':'Exact equality of the first-coupling equal-spin asymptotic ADM/Komar mass correction through O(chi^2). Does not validate higher spin orders, finite coupling, or global solution existence.'}

def main():
    p=argparse.ArgumentParser(); p.add_argument('--output',type=Path); a=p.parse_args()
    out=derive(); out['source_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    out['environment']={'python':platform.python_version(),'sympy':sp.__version__}
    s=json.dumps(out,indent=2,sort_keys=True)+'\n'
    if a.output: a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(s,encoding='utf-8')
    print(s)
if __name__=='__main__': main()

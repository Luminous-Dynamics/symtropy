#!/usr/bin/env python3
"""Standard-library control: deflated roots and the exact cubic cusp locus.
This tests numerical machinery only; it does not classify the EGB solution set.
"""
import json, math, platform, sys
from pathlib import Path

def f(x): return x*x*x-x
def fp(x): return 3*x*x-1

def solve(seed, roots):
    x=float(seed)
    for k in range(80):
        p, lp = 1.0, 0.0
        try:
            for r in roots:
                d=x-r
                if abs(d)<1e-9: return None
                q=1+abs(d)**-2
                p*=q; lp += -2*math.copysign(1,d)*abs(d)**-3/q
        except (OverflowError, ZeroDivisionError): return None
        g=f(x)*p; dg=(fp(x)+f(x)*lp)*p
        if not (math.isfinite(g) and math.isfinite(dg)) or abs(dg)<1e-12: return None
        if abs(g)<1e-11: return x,k
        x-=max(-1.0,min(1.0,g/dg))
        if not math.isfinite(x) or abs(x)>10: return None
    return None

def main(out=Path('.')):
    seeds=[2.0,-2.0,0.3]; roots=[]; attempts=[]
    for seed in seeds:
        r=solve(seed,roots); item={'seed':seed,'accepted':False}
        if r is not None:
            x,it=r; residual=abs(f(x)); dist=min([abs(x-y) for y in roots],default=None)
            item.update(candidate=x,iterations=it,original_residual=residual,
                        nearest_known_root_distance=dist,
                        accepted=residual<1e-9 and (dist is None or dist>1e-5))
            if item['accepted']: roots.append(x)
        attempts.append(item)
    roots.sort(); exact=[-1.0,0.0,1.0]
    errors=[abs(x-y) for x,y in zip(roots,exact)]
    assert len(roots)==3 and max(errors)<1e-9
    cusp=[]
    for u in [-1.0,-0.5,0.0,0.5,1.0]:
        mu=-3*u*u; nu=2*u**3
        cusp.append({'u':u,'mu':mu,'nu':nu,'F':u**3+mu*u+nu,
                     'Fu':3*u*u+mu,'discriminant':4*mu**3+27*nu**2})
    assert max(abs(p[k]) for p in cusp for k in ('F','Fu','discriminant'))<1e-12
    bad={'u':0.5,'mu':-0.75,'mutated_nu':-0.25,'original_residual':-0.5}
    injected={'x':0.2,'original_residual':abs(f(0.2))}
    assert abs(bad['original_residual'])>0.1 and injected['original_residual']>0.1
    fixture={'schema_version':1,'status':'executed_polynomial_controls_not_egb_branch_classification',
      'deflation':{'equation':'x^3-x=0','exact_roots':exact,'seeds':seeds,
        'factor':'product_i (1+abs(x-r_i)^(-2))','attempts':attempts,
        'discovered_roots':roots,'max_root_error':max(errors),
        'original_equation_requalification':True,'finite_seeds_are_not_completeness_proof':True},
      'cusp':{'equation':'u^3+mu*u+nu=0','fold_locus':'mu=-3*u^2, nu=2*u^3',
        'discriminant':'4*mu^3+27*nu^2=0','points':cusp,
        'cusp_at_origin':'F=Fu=Fuu=0; Fuuu=6','sign_mutation':bad},
      'negative_control':{'injected_nonroot':injected,'both_rejected_against_original_equations':True},
      'runtime':{'python':platform.python_version(),'dependencies':'stdlib only'},
      'claim_ceiling':'Finite-dimensional method controls only; no EGB fold, cusp, disconnected branch, or completeness result.'}
    out.mkdir(parents=True,exist_ok=True)
    (out/'branch_atlas_cusp_deflation_fixture_v1.json').write_text(json.dumps(fixture,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'roots':roots,'max_root_error':max(errors),'attempts':attempts,
                      'cusp_points':len(cusp),'mutated_fold_residual':bad['original_residual'],
                      'injected_nonroot_residual':injected['original_residual']},indent=2))
if __name__=='__main__': main(Path(sys.argv[1]) if len(sys.argv)>1 else Path('.'))

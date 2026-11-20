#!/usr/bin/env python3
"""Standard-library control for first-order splitting of a degenerate block.
This is a symmetric-matrix method control, not an EGB principal symbol.
"""
import json, math, platform, sys
from pathlib import Path

def eig(a):
    m=[list(map(float,r)) for r in a]; n=len(m)
    for _ in range(100*n*n):
        p,q=max(((i,j) for i in range(n) for j in range(i+1,n)),key=lambda ij:abs(m[ij[0]][ij[1]]))
        if abs(m[p][q])<1e-15: return sorted(m[i][i] for i in range(n))
        tau=(m[q][q]-m[p][p])/(2*m[p][q]); t=math.copysign(1,tau)/(abs(tau)+math.sqrt(1+tau*tau)) if tau else 1
        c=1/math.sqrt(1+t*t); s=t*c; ap,aq,off=m[p][p],m[q][q],m[p][q]
        m[p][p]=ap-t*off; m[q][q]=aq+t*off; m[p][q]=m[q][p]=0
        for k in range(n):
            if k not in (p,q):
                x,y=m[k][p],m[k][q]; m[k][p]=m[p][k]=c*x-s*y; m[k][q]=m[q][k]=s*x+c*y
    raise RuntimeError('Jacobi eigensolver failed')

def mm(a,b): return [[sum(a[i][k]*b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]
def tr(a): return [list(x) for x in zip(*a)]

def main(out=Path('.')):
    A0=[[0,0,0],[0,0,0],[0,0,4]]
    V=[[.2,.6,.25],[.6,-.1,-.2],[.25,-.2,.3]]
    shifts=eig([r[:2] for r in V[:2]])
    naive=sorted([V[0][0],V[1][1]])
    basis=[]
    for th in [0,.37,.91,1.41]:
        c,s=math.cos(th),math.sin(th); Q=[[c,-s,0],[s,c,0],[0,0,1]]
        Vr=mm(mm(tr(Q),V),Q); sp=eig([r[:2] for r in Vr[:2]])
        basis.append({'theta':th,'shifts':sp,'defect':max(abs(x-y) for x,y in zip(shifts,sp))})
    sweep=[]
    for e in [.1,.05,.025,.0125]:
        M=[[A0[i][j]+e*V[i][j] for j in range(3)] for i in range(3)]
        exact=eig(M); pred=sorted([e*shifts[0],e*shifts[1],4+e*V[2][2]])
        diag=sorted([e*naive[0],e*naive[1],4+e*V[2][2]])
        err=max(abs(x-y) for x,y in zip(exact,pred)); derr=max(abs(x-y) for x,y in zip(exact,diag))
        sweep.append({'epsilon':e,'exact':exact,'prediction':pred,'error':err,
                      'error_over_epsilon_squared':err/e**2,'diagonal_only_error':derr,
                      'diagonal_only_error_over_epsilon':derr/e})
    bdef=max(x['defect'] for x in basis); qerr=max(x['error_over_epsilon_squared'] for x in sweep)
    nerr=min(x['diagonal_only_error_over_epsilon'] for x in sweep)
    assert bdef<2e-14 and qerr<.03 and nerr>.25
    fixture={'schema_version':1,'status':'executed_symmetric_matrix_control_not_egb_symbol_result',
      'unperturbed_matrix':A0,'perturbation_matrix':V,'degenerate_dimension':2,
      'first_order_shifts':shifts,'diagonal_only_shifts':naive,'basis_checks':basis,'epsilon_sweep':sweep,
      'summary':{'max_basis_defect':bdef,'max_error_over_epsilon_squared':qerr,
                 'min_diagonal_error_over_epsilon':nerr},
      'runtime':{'python':platform.python_version(),'dependencies':'stdlib only'},
      'claim_ceiling':'Method control only; does not derive an EGB principal symbol, physical subspace, symmetrizer, or rotating-background classification.'}
    out.mkdir(parents=True,exist_ok=True)
    (out/'degenerate_characteristic_perturbation_fixture_v1.json').write_text(json.dumps(fixture,indent=2,sort_keys=True)+'\n')
    print(json.dumps(fixture['summary'],indent=2))
if __name__=='__main__': main(Path(sys.argv[1]) if len(sys.argv)>1 else Path('.'))

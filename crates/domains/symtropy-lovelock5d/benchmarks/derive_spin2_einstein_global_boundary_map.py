#!/usr/bin/env python3
"""Exact action-derived global boundary map for the 5D Einstein-limit O(a^2) spin-2 system.

Vary b,f,g,h,w independently, impose g=r^2 only after variation, and derive
the complete horizon-regular family and its map to forbidden infinity modes.

Requires Python 3.12+ and SymPy 1.14+.
"""
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path
import sympy as sp


def zero(label, expr):
    vals = list(expr) if isinstance(expr, sp.MatrixBase) else [expr]
    vals = [sp.factor(sp.cancel(v)) for v in vals]
    if any(v != 0 for v in vals):
        raise AssertionError(f"{label}: {vals}")


def derive():
    r,e = sp.symbols("r epsilon", positive=True)
    b,f,g,h,w = sp.symbols("b f g h w", positive=True)
    bp,fp,gp,hp,wp = sp.symbols("bp fp gp hp wp", real=True)
    bpp,fpp,gpp,hpp,wpp = sp.symbols("bpp fpp gpp hpp wpp", real=True)
    q0=(b,f,g,h,w); q1=(bp,fp,gp,hp,wp); q2=(bpp,fpp,gpp,hpp,wpp)
    m=sp.sqrt(f*h/b)
    L=m*(bp*gp+g*bp*hp/(2*h)+b*gp**2/(2*g)+b*gp*hp/h
         +g*h*wp**2/2+2*b/f*(4-h/g))
    def D(z):
        return (sum(sp.diff(z,x)*y for x,y in zip(q0,q1))
                +sum(sp.diff(z,x)*y for x,y in zip(q1,q2)))
    mom={n:sp.diff(L,x) for n,x in zip(("b","f","g","h","w"),q1)}
    EL={n:sp.diff(L,x)-D(mom[n]) for n,x in zip(("b","f","g","h","w"),q0)}
    Bf,Ff,Hf=(sp.Function(n)(r) for n in ("B2","F2","H2"))
    F0=1-r**-2; W1=r**-4
    ans={b:F0+e**2*Bf,f:F0+e**2*Ff,g:r**2,h:r**2+e**2*Hf,w:e*W1,
         bp:sp.diff(F0+e**2*Bf,r),fp:sp.diff(F0+e**2*Ff,r),gp:2*r,
         hp:sp.diff(r**2+e**2*Hf,r),wp:e*sp.diff(W1,r),
         bpp:sp.diff(F0+e**2*Bf,r,2),fpp:sp.diff(F0+e**2*Ff,r,2),
         gpp:sp.Integer(2),hpp:sp.diff(r**2+e**2*Hf,r,2),wpp:e*sp.diff(W1,r,2)}
    rows={n:sp.diff(EL[n].subs(ans),e,2).subs(e,0)/2 for n in ("b","f","g","h")}
    B,Bp,Bpp,F,Fp,H,Hp,Hpp,Fpp=sp.symbols("B Bp Bpp F Fp H Hp Hpp Fpp")
    jets=(B,Bp,Bpp,F,Fp,H,Hp,Hpp)
    jm={Bf:B,sp.diff(Bf,r):Bp,sp.diff(Bf,r,2):Bpp,
        Ff:F,sp.diff(Ff,r):Fp,sp.diff(Ff,r,2):Fpp,
        Hf:H,sp.diff(Hf,r):Hp,sp.diff(Hf,r,2):Hpp}
    Ej={n:sp.cancel(v.xreplace(jm)) for n,v in rows.items()}
    def affine(z):
        c={x:sp.cancel(sp.diff(z,x)) for x in jets}
        s=sp.cancel(z-sum(c[x]*x for x in jets))
        return c,s
    cf,sf=affine(Ej["f"]); cb,sb=affine(Ej["b"]); ch,sh=affine(Ej["h"])
    pivot=sp.factor(cf[F])
    if pivot==0: raise AssertionError("E_f algebraic pivot vanished")
    k={x:sp.cancel(-cf[x]/pivot) for x in (B,Bp,H,Hp)}
    ks=sp.cancel(-sf/pivot)
    Frec=sum(k[x]*x for x in k)+ks
    Fprec=sp.cancel(sum(sp.diff(k[x],r)*x for x in k)+sp.diff(ks,r)
        +k[B]*Bp+k[Bp]*Bpp+k[H]*Hp+k[Hp]*Hpp)
    red={}
    for n,c,s in (("b",cb,sb),("h",ch,sh)):
        z=sp.cancel((sum(c[x]*x for x in jets)+s).subs(
            {F:Frec,Fp:Fprec},simultaneous=True))
        wanted=(B,Bp,Bpp,H,Hp,Hpp)
        cc={x:sp.cancel(sp.diff(z,x)) for x in wanted}
        ss=sp.cancel(z-sum(cc[x]*x for x in wanted))
        red[n]=(cc,ss)
    M=sp.Matrix([[red["b"][0][Bpp],red["b"][0][Hpp]],
                 [red["h"][0][Bpp],red["h"][0][Hpp]]])
    C=sp.Matrix([[red[n][0][x] for x in (B,Bp,H,Hp)] for n in ("b","h")])
    S=sp.Matrix([red["b"][1],red["h"][1]])
    detM=sp.factor(M.det())
    zero("det(M)",detM-(r**2-1)/4)

    b1,h0=sp.symbols("b1 h0", real=True)
    db=b1+4; dh=h0-1
    Bsol=-r**-2+r**-6+db*sp.Rational(1,2)*(1-r**-2)
    Hsol=r**-2+dh*(2*r**4-r**2)
    Fsol=sp.factor(-(r**2-1)*(2*dh*r**4-dh*r**2+1)/r**4)
    sub={Bf:Bsol,Ff:Fsol,Hf:Hsol,
         sp.diff(Bf,r):sp.diff(Bsol,r),sp.diff(Ff,r):sp.diff(Fsol,r),sp.diff(Hf,r):sp.diff(Hsol,r),
         sp.diff(Bf,r,2):sp.diff(Bsol,r,2),sp.diff(Ff,r,2):sp.diff(Fsol,r,2),
         sp.diff(Hf,r,2):sp.diff(Hsol,r,2)}
    residuals={n:sp.factor(sp.cancel(v.subs(sub))) for n,v in rows.items()}
    for n,v in residuals.items(): zero(f"original E_{n}",v)

    zero("positive E_f pivot form",pivot-3*r**5/(r**2-1)**2)
    MP={Bf:-r**-2+r**-6,Ff:-r**-2+r**-4,Hf:r**-2}
    modes=(
        {"B":sp.Rational(1,2)*(1-r**-2),"F":sp.Integer(0),"H":sp.Integer(0)},
        {"B":sp.Integer(0),"F":-2*r**2+3-r**-2,"H":2*r**4-r**2},
    )
    for i,mode in enumerate(modes):
        sm={Bf:MP[Bf]+mode["B"],Ff:MP[Ff]+mode["F"],Hf:MP[Hf]+mode["H"]}
        for X in (Bf,Ff,Hf): sm[sp.diff(X,r)]=sp.diff(sm[X],r); sm[sp.diff(X,r,2)]=sp.diff(sm[X],r,2)
        mp_sub={Bf:MP[Bf],Ff:MP[Ff],Hf:MP[Hf]}
        for X in (Bf,Ff,Hf): mp_sub[sp.diff(X,r)]=sp.diff(MP[X],r); mp_sub[sp.diff(X,r,2)]=sp.diff(MP[X],r,2)
        for n,v in rows.items(): zero(f"homogeneous mode {i} E_{n}",v.subs(sm)-v.subs(mp_sub))
    zero("B horizon",Bsol.subs(r,1))
    zero("B prime horizon",sp.diff(Bsol,r).subs(r,1)-b1)
    zero("F horizon",Fsol.subs(r,1))
    zero("F prime horizon",sp.diff(Fsol,r).subs(r,1)+2*h0)
    zero("H horizon",Hsol.subs(r,1)-h0)
    zero("H prime compatibility",sp.diff(Hsol,r).subs(r,1)-(6*h0-8))
    CB=sp.factor(sp.limit(Bsol,r,sp.oo))
    CH=sp.factor(sp.limit(Hsol/r**4,r,sp.oo))
    CF=sp.factor(sp.limit(Fsol/r**2,r,sp.oo))
    J=sp.Matrix([[sp.diff(CB,b1),sp.diff(CB,h0)],
                 [sp.diff(CH,b1),sp.diff(CH,h0)]])
    zero("boundary outputs",sp.Matrix([CB-(b1+4)/2,CH-2*(h0-1)]))
    zero("F growth output",CF+2*(h0-1))
    zero("F/H growth relation",CF+CH)
    zero("boundary Jacobian",J-sp.diag(sp.Rational(1,2),2))
    zero("boundary determinant",J.det()-1)
    zero("condition number",sp.sqrt(4/(sp.Rational(1,4)))-4)
    params=sp.solve([CB,CH],(b1,h0),dict=True)
    if params != [{b1:-4,h0:1}]: raise AssertionError(f"asymptotically flat data: {params}")
    zero("MP B",Bsol.subs(params[0])-MP[Bf])
    zero("MP F",Fsol.subs(params[0])-MP[Ff])
    zero("MP H",Hsol.subs(params[0])-MP[Hf])
    sh=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    return {
      "schema_version":1,
      "artifact_kind":"5d_einstein_limit_spin2_exact_global_boundary_map",
      "status":"exact_global_horizon_to_infinity_map_derived",
      "conventions":{"dimension":5,"alpha":"0","r_H":"1","spin_order":"O(a^2)",
        "gauge_order":"vary b,f,g,h,w independently; then impose g=r^2",
        "horizon_normalization":"Bhat2(1)=0","free_data":["b1=Bhat2'(1)","h0=Hhat2(1)"]},
      "reduced_operator":{
        "principal_matrix":[[sp.sstr(M[i,j]) for j in range(2)] for i in range(2)],
        "determinant":sp.sstr(detM),"algebraic_Ef_pivot":sp.sstr(pivot),
        "lower_order_matrix":[[sp.sstr(C[i,j]) for j in range(4)] for i in range(2)],
        "source":[sp.sstr(x) for x in S]},
      "exact_global_solution":{
        "Bhat2":sp.sstr(Bsol),"Fhat2":sp.sstr(Fsol),"Hhat2":sp.sstr(Hsol),
        "horizon_data":{"B2_1":"0","B2_prime_1":"b1","F2_1":"0","F2_prime_1":"-2*h0","H2_1":"h0","H2_prime_1":"6*h0-8"},
        "homogeneous_basis":[
          {"Bhat2":"(1-r^(-2))/2","Fhat2":"0","Hhat2":"0","infinity_image":["1/2","0"]},
          {"Bhat2":"0","Fhat2":"-2*r^2+3-r^(-2)","Hhat2":"2*r^4-r^2","infinity_image":["0","2"]}]},
      "global_boundary_map":{
        "outputs":["C_B=lim Bhat2","C_H=lim Hhat2/r^4"],
        "map":["C_B=(b1+4)/2","C_H=2*(h0-1)"],
        "additional_asymptotic_consistency":{"C_F2=lim Fhat2/r^2":"-2*(h0-1)","C_F2_plus_C_H":"0"},
        "jacobian":[["1/2","0"],["0","2"]],"determinant":"1",
        "singular_values":["1/2","2"],"euclidean_condition_number":"4",
        "inverse":["b1=2*C_B-4","h0=C_H/2+1"],
        "asymptotically_flat_parameters":{"b1":"-4","h0":"1"},
        "unique_asymptotically_flat_profiles":{
          "Bhat2":"-r^(-2)+r^(-6)","Fhat2":"-r^(-2)+r^(-4)","Hhat2":"r^(-2)"}},
      "checks":{"all_four_original_even_rows_exact_zero":True,
        "both_homogeneous_modes_exact_zero":True,"principal_determinant_exact":True,
        "principal_matrix_positive_determinant_for_r_gt_1":True,
        "Ef_algebraic_pivot_positive_for_r_gt_1":True,"horizon_compatibility_exact":True,
        "F2_H2_growing_modes_consistent":True,"global_boundary_map_exact":True,"global_map_full_rank":True,
        "global_map_determinant_one":True,"global_map_condition_number_four":True,
        "asymptotically_flat_solution_unique_in_horizon_regular_family":True,
        "Myers_Perry_known_answer_recovered":True},
      "derivation":{"source_sha256":sh,"sympy_version":sp.__version__,"arithmetic":"exact symbolic algebra"},
      "claim_ceiling":"Exact global map and unique asymptotically flat analytic-horizon solution for the 5D alpha=0 O(a^2) action-derived perturbative system with Bhat2(1)=0. This does not prove finite-coupling EGB uniqueness, full nonlinear uniqueness, interval existence, a finite-coupling validated branch, or PDE hyperbolicity."
    }


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path,default=Path("spin2_einstein_global_boundary_map_v1.json"))
    args=parser.parse_args()
    result=derive()
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(rendered,encoding="utf-8")
    print(json.dumps({"status":result["status"],"output":str(args.output),
      "sha256":hashlib.sha256(rendered.encode()).hexdigest(),
      "source_sha256":result["derivation"]["source_sha256"],"checks":result["checks"]},indent=2))


if __name__=="__main__":
    main()

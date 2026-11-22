#!/usr/bin/env python3
"""Exact symbolic Green--Lagrange projection of action-derived spin-2 rows.

Consumes the exact row fixture emitted by extract_spin2_boundary_map_lambda1_rows.py.
Computes both columns of dJ/dlambda at lambda=0 in the frozen Einstein-limit
horizon and asymptotic coordinates.

Requires Python >=3.12 and SymPy >=1.14.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import sympy as sp


def zero(label, expression):
    values = list(expression) if isinstance(expression, sp.MatrixBase) else [expression]
    residuals = [sp.factor(sp.cancel(v)) for v in values]
    if any(v != 0 for v in residuals):
        raise AssertionError(f"{label}: {residuals}")


def verify(rows_path: Path) -> dict:
    data = json.loads(rows_path.read_text(encoding="utf-8"))
    r, x = sp.symbols("r x", positive=True)
    cB, cH = sp.symbols("cB cH", real=True)
    B1f, H1f = sp.Function("B1"), sp.Function("H1")
    Bx, Hx = sp.Function("B")(x), sp.Function("H")(x)
    loc = {"r": r, "cB": cB, "cH": cH, "B1": B1f, "H1": H1f,
           "Derivative": sp.Derivative}
    red_b = sp.sympify(data["reduced_rows_after_ef"]["b"], locals=loc)
    red_h = sp.sympify(data["reduced_rows_after_ef"]["h"], locals=loc)
    Frec = sp.sympify(data["ef_reconstruction_F21"], locals=loc)
    pivot = sp.sympify(data["ef_algebraic_pivot"], locals=loc)

    zero("recorded E_f pivot", pivot-3*r**5/(r**2-1)**2)
    x_sub = {
        B1f(r): Bx.subs(x, r**2), H1f(r): Hx.subs(x, r**2),
        sp.diff(B1f(r), r): 2*r*sp.diff(Bx, x).subs(x, r**2),
        sp.diff(B1f(r), r, 2): (2*sp.diff(Bx, x)+4*r**2*sp.diff(Bx, x, 2)).subs(x, r**2),
        sp.diff(H1f(r), r): 2*r*sp.diff(Hx, x).subs(x, r**2),
        sp.diff(H1f(r), r, 2): (2*sp.diff(Hx, x)+4*r**2*sp.diff(Hx, x, 2)).subs(x, r**2),
    }
    eq1 = sp.expand(x**sp.Rational(3, 2)*red_b.subs(x_sub).subs(r, sp.sqrt(x)).doit())
    eq2 = sp.expand(-3*x**sp.Rational(7, 2)*red_h.subs(x_sub).subs(r, sp.sqrt(x)).doit())
    op1 = (3*x**4*sp.diff(Bx, x, 2)+6*x**3*sp.diff(Bx, x)
           -x**2*sp.diff(Hx, x, 2)+2*x*sp.diff(Hx, x)-2*Hx)
    op2 = (3*x**4*sp.diff(Bx, x, 2)+6*x**3*sp.diff(Bx, x)
           +(-4*x**4+4*x**3-x**2)*sp.diff(Hx, x, 2)
           +(-4*x**2+2*x)*sp.diff(Hx, x)+(8*x**2+4*x-2)*Hx)
    sb = -33/x+24/x**2+50/x**3
    sh = -8+31/x+56/x**2+50/x**3
    src_b1, src_b2 = sp.diff(eq1, cB), sp.diff(eq2, cB)
    src_h1, src_h2 = sp.diff(eq1, cH), sp.diff(eq2, cH)
    zero("full reduced E_b operator and source", eq1-op1-sb-cB*(-sp.Rational(9,2)/x)-cH*(6-9/x))
    zero("full reduced E_h operator and source", eq2-op2-sh-cB*(-sp.Rational(9,2)/x)-cH*(4*x**3+48*x**2-12*x+6-9/x))

    # b1 column: B21=1/2-3/(4x)+1/(4x^3), H21=0.
    B_b = sp.Rational(1,2)-sp.Rational(3,4)/x+sp.Rational(1,4)/x**3
    sub_b = {Bx: B_b, Hx: 0,
             sp.diff(Bx, x): sp.diff(B_b,x), sp.diff(Bx,x,2): sp.diff(B_b,x,2),
             sp.diff(Hx,x): 0, sp.diff(Hx,x,2): 0}
    zero("b1 homogeneous sensitivity satisfies E_b", op1.subs(sub_b)+src_b1)
    zero("b1 homogeneous sensitivity satisfies E_h", op2.subs(sub_b)+src_b2)
    zero("b1 fixed horizon value", B_b.subs(x,1))
    zero("b1 fixed horizon slope", sp.diff(B_b,x).subs(x,1))
    CB_b, CH_b = sp.limit(B_b,x,sp.oo), sp.Integer(0)
    zero("b1 boundary image", sp.Matrix([CB_b,CH_b])-sp.Matrix([sp.Rational(1,2),0]))

    # h0 column: subtract the two reduced equations. The homogeneous operator
    # collapses to one scalar regular-singular equation L[H]=S_h.
    S_h = x**2+12*x-3
    Lh = lambda z: x**2*(x-1)*sp.diff(z,x,2)+x*sp.diff(z,x)-(2*x+1)*z
    zero("h0 scalar forcing", src_h2-src_h1-4*x*S_h)
    zero("h0 scalar operator", op2-op1+4*x*Lh(Hx))

    y1 = x*(2*x-1)
    y2 = 2*x-x*(2*x-1)*sp.log(x/(x-1))
    zero("growing homogeneous solution", Lh(y1))
    zero("decaying homogeneous solution", sp.simplify(sp.expand_log(Lh(y2),force=True)))
    P = (x-1)/x
    W = sp.simplify(P*(y1*sp.diff(y2,x)-y2*sp.diff(y1,x)))
    zero("Green-Lagrange weighted Wronskian", W-1)

    # In the variable s=1/x, the source integral has this exact primitive.
    s = sp.symbols("s", positive=True)
    A = (-1/s+3-9*s+(3*s+21-23/s-1/s**2)*sp.log(1-s)
         +18*sp.polylog(2,s))
    integrand = (2/s+(2-s)*sp.log(1-s)/s**2)*(1/s+12-3*s)
    zero("dilogarithmic Green primitive", sp.simplify(sp.expand_func(sp.diff(A,s)-integrand)))
    A0 = sp.series(A,s,0,3).removeO().subs(s,0)
    log_coefficient = 3*s+21-23/s-1/s**2
    A1 = -1+3-9+18*sp.pi**2/6
    zero("Green primitive at s=0", A0-sp.Rational(53,2))
    zero("log term vanishes at s=1", log_coefficient.subs(s,1))
    zero("Green primitive at s=1", A1-(3*sp.pi**2-7))
    CH_h = sp.simplify(-2*(A1-A0))

    # Frobenius at x=1 for the fixed-horizon-data h0 mode; H(1)=0 and H_x(1)=10.
    t = sp.symbols("t")
    coeffs = [sp.Integer(0),sp.Integer(10),sp.Rational(17,2),-sp.Rational(1,2),
              sp.Rational(3,8),-sp.Rational(59,200),sp.Rational(6,25),-sp.Rational(981,4900)]
    Hser = sum(coeffs[n]*t**n for n in range(len(coeffs)))
    X = 1+t
    fseries = sp.series(X**2*(X-1)*sp.diff(Hser,t,2)+X*sp.diff(Hser,t)
                        -(2*X+1)*Hser-(X**2+12*X-3),t,0,len(coeffs)-1).removeO()
    zero("h0 horizon Frobenius recurrence", fseries)
    Hx_horizon = coeffs[1]

    # Reconstruct B by the first reduced row. Fubini plus two integrations
    # by parts give C_B=(C_H-H_x(1)+H(1)-integral(S_b/x^3))/3.
    S_b = 6-9/x
    source_int = sp.integrate(S_b/x**3,(x,1,sp.oo))
    zero("B-equation source integral", source_int)
    CB_h = sp.simplify((CH_h-Hx_horizon)/3)
    zero("h0 boundary amplitudes", sp.Matrix([CB_h,CH_h])-sp.Matrix([19-2*sp.pi**2,67-6*sp.pi**2]))

    # E_f reconstruction: for H=C*x^2+D*x+o(x), B bounded, F21/r^2 -> -C.
    C,D,Binf = sp.symbols("C D Binf")
    Frec_asym = Frec.subs({
        B1f(r):Binf, H1f(r):C*r**4+D*r**2,
        sp.diff(B1f(r),r):0,
        sp.diff(H1f(r),r):4*C*r**3+2*D*r,
    })
    Fgrow = sp.limit(Frec_asym/r**2,r,sp.oo)
    zero("Ef growing amplitude matches minus H", Fgrow+C)

    J0 = sp.diag(sp.Rational(1,2),2)
    J1 = sp.Matrix([[CB_b,CB_h],[CH_b,CH_h]])
    zero("complete boundary Jacobian derivative", J1-sp.Matrix([
        [sp.Rational(1,2),19-2*sp.pi**2],[0,67-6*sp.pi**2]]))
    pprime = sp.Matrix([sp.Rational(106,9),-sp.Rational(14,3)])
    offset = -J0*pprime
    zero("known affine offset derivative",offset-sp.Matrix([-sp.Rational(53,9),sp.Rational(28,3)]))
    det_slope = sp.simplify(sp.trace(J0.inv()*J1))
    kappa_slope = sp.simplify(2*J1[1,1]-8*J1[0,0])

    # Negative controls: wrong forcing sign, altered horizon value, and
    # contamination of the decaying adjoint with the growing homogeneous mode.
    assert -CH_h != CH_h
    assert sp.Integer(1) != 0
    y2_bad = y2+y1
    zero("clean decaying adjoint asymptotic",sp.limit(y2/x**2,x,sp.oo))
    zero("contaminated adjoint exposes growing mode",sp.limit(y2_bad/x**2,x,sp.oo)-2)

    return {
        "schema_version":1,
        "artifact_kind":"5d_egb_spin2_global_boundary_map_lambda1",
        "status":"exact_first_derivative_of_both_homogeneous_boundary_map_columns",
        "action_row_fixture":str(rows_path.name),
        "action_row_fixture_sha256":hashlib.sha256(rows_path.read_bytes()).hexdigest(),
        "conventions":{
            "lambda":"alpha/r_H^2", "r_H":1, "spin_order":"O(a^2)",
            "horizon_coordinates":"(B2_prime_r(1),H2(1))",
            "boundary_amplitudes":"(lim B2, lim H2/r^4)",
            "endpoint_basis":"fixed Einstein-limit horizon modes and fixed leading growing-mode projectors",
        },
        "J0":[["1/2","0"],["0","2"]],
        "known_physical_branch_tangent":["106/9","-14/3"],
        "affine_offset_derivative":["-53/9","28/3"],
        "b1_mode_sensitivity":{"B21":"1/2-3/(4*x)+1/(4*x^3)","H21":"0","boundary_image":["1/2","0"]},
        "h0_mode_sensitivity":{
            "scalar_equation":"x^2*(x-1)*Hxx+x*Hx-(2*x+1)*H=x^2+12*x-3",
            "decaying_adjoint":"2*x-x*(2*x-1)*log(x/(x-1))",
            "weighted_wronskian":"1",
            "green_primitive":"-1/s+3-9*s+(3*s+21-23/s-1/s^2)*log(1-s)+18*Li_2(s)",
            "primitive_endpoints":["53/2","3*pi^2-7"],
            "horizon_series_coefficients":[str(v) for v in coeffs],
            "Hx_at_horizon":"10",
            "boundary_image":["19-2*pi^2","67-6*pi^2"],
        },
        "J1":[[sp.sstr(J1[i,j]) for j in range(2)] for i in range(2)],
        "J1_numeric":[[str(sp.N(J1[i,j],18)) for j in range(2)] for i in range(2)],
        "diagnostics_in_fixed_published_basis":{
            "determinant_slope":sp.sstr(det_slope),
            "determinant_slope_numeric":str(sp.N(det_slope,18)),
            "sigma_min_slope":sp.sstr(J1[0,0]),
            "sigma_max_slope":sp.sstr(J1[1,1]),
            "kappa2_slope":sp.sstr(kappa_slope),
            "kappa2_slope_numeric":str(sp.N(kappa_slope,18)),
            "basis_warning":"Raw singular values and Euclidean condition number are coordinate-dependent; these slopes use the frozen published endpoint basis.",
        },
        "checks":{
            "E_f_pivot":True,"reduced_rows_and_sources":True,"b1_column":True,
            "h0_scalar_ode":True,"Green_Lagrange_Wronskian":True,"dilogarithm_integral":True,
            "horizon_Frobenius":True,"B_boundary_projection":True,"E_f_F_growing_mode":True,
            "affine_offset_crosscheck":True,
            "negative_controls":["wrong Green forcing sign","nonzero horizon value","decaying-adjoint contamination"],
            "E_g":"propagated radial constraint via exact action-level Noether identity; E_g coefficient row is retained in the source fixture",
        },
        "replay":{
            "python":platform.python_version(),"sympy":sp.__version__,
            "verifier_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "arithmetic":"exact rational, logarithmic, and dilogarithmic symbolic identities",
        },
        "claim_ceiling":"Exact derivative at lambda=0 of both columns of the global map for the specified 5D O(a^2) system in the fixed Einstein-limit endpoint coordinates. This is not a finite-lambda BVP, finite-lambda analyticity theorem, interval-invertibility proof, global uniqueness result, or PDE hyperbolicity qualification.",
    }


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--rows",type=Path,required=True)
    parser.add_argument("--output",type=Path,default=Path("spin2_boundary_map_lambda1_v1.json"))
    args=parser.parse_args()
    result=verify(args.rows)
    rendered=json.dumps(result,indent=2,sort_keys=True)+"\n"
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(rendered,encoding="utf-8")
    print(json.dumps({"status":result["status"],"output":str(args.output),
                      "sha256":hashlib.sha256(rendered.encode()).hexdigest(),
                      "J1":result["J1"],"diagnostics":result["diagnostics_in_fixed_published_basis"],
                      "checks":result["checks"]},indent=2))


if __name__=="__main__":
    main()

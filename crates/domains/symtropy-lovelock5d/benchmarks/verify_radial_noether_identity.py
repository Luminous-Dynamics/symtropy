#!/usr/bin/env python3
"""Verify the radial-diffeomorphism Noether identity of the 5D EGB action."""
from __future__ import annotations
import argparse, hashlib, json, platform
from pathlib import Path
import sympy as sp

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    r, alpha = sp.symbols("r alpha", positive=True, finite=True)
    b, f, g, h, w = sp.symbols("b f g h w", positive=True, finite=True)
    bp, fp, gp, hp, wp = sp.symbols("bp fp gp hp wp", real=True)
    bpp, fpp, gpp, hpp, wpp = sp.symbols("bpp fpp gpp hpp wpp", real=True)
    q, qp, qpp = (b, f, g, h, w), (bp, fp, gp, hp, wp), (bpp, fpp, gpp, hpp, wpp)
    measure = sp.sqrt(f * h / b)
    L_eh = measure * (
        bp*gp + g*bp*hp/(2*h) + b*gp**2/(2*g) + b*gp*hp/h
        + g*h*wp**2/2 + 2*b/f*(4-h/g)
    )
    L_gb = measure/g * (
        4*h*bp*gp/g + 2*(4*g-3*h)*(bp*hp/h+h*wp**2)
        - f*bp*hp*gp**2/(2*h) - f*h*gp**2*wp**2/2
    )
    L = L_eh + alpha*L_gb/4
    def D(expr):
        return (sum(sp.diff(expr, qi)*qpi for qi,qpi in zip(q,qp))
                + sum(sp.diff(expr,qpi)*qppi for qpi,qppi in zip(qp,qpp)))
    E = {name:sp.diff(L,qi)-D(sp.diff(L,qpi))
         for name,qi,qpi in zip(("b","f","g","h","w"),q,qp)}
    # g_rr=1/f implies delta f=xi*f'-2*f*xi'; b,g,h,w are scalars.
    identity = sum(E[name]*qpi for name,qpi in zip(("b","f","g","h","w"),qp)) + 2*D(f*E["f"])
    if sp.cancel(identity) != 0:
        raise AssertionError("off-shell radial Noether identity did not cancel")
    fixture = {
        "schema_version":1,
        "artifact_kind":"5d_egb_radial_diffeomorphism_noether_identity",
        "status":"off_shell_identity_symbolically_verified",
        "source":{"paper":"Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860",
                  "action":"I=(16*pi*G)^-1 integral sqrt(-g)*(R+alpha*L_GB/4)"},
        "radial_diffeomorphism":{
            "generator":"xi(r)",
            "scalar_profile_variations":{"delta_b":"xi*b'","delta_g":"xi*g'","delta_h":"xi*h'","delta_w":"xi*w'"},
            "inverse_radial_metric_variation":"delta_f=xi*f'-2*f*xi' for g_rr=1/f"},
        "off_shell_identity":"E_b*b' + E_f*f' + E_g*g' + E_h*h' + E_w*w' + 2*d(f*E_f)/dr = 0",
        "gauge_fixed_identity":"2*r*E_g + b'*E_b + f'*E_f + h'*E_h + w'*E_w + 2*d(f*E_f)/dr = 0, after g=r^2",
        "spin_squared_projection":"2*r*E_g^(2) + F'*E_b^(2) + F'*E_f^(2) + 2*r*E_h^(2) + W1'*E_w^(1) + 2*d(F*E_f^(2))/dr = 0",
        "spin_squared_projection_if_linear_angular_equation_holds":"2*r*E_g^(2) + F'*E_b^(2) + F'*E_f^(2) + 2*r*E_h^(2) + 2*d(F*E_f^(2))/dr = 0, when E_w^(1)=0",
        "reduction_contract":{
            "radial_constraint_retained":True,
            "permitted_solver_reduction":"Use the identity to select an independent equation subset only after specifying a nonsingular pivot; always evaluate E_g as a separate constraint residual.",
            "forbidden":"Do not omit E_g from candidate qualification or infer it from small residuals without bounding the identity defect.",
            "angular_term":"W1'*E_w^(1) must be retained off shell; it vanishes only when the exact first-order angular equation is satisfied."},
        "validation":{
            "symbolic_engine":"SymPy "+sp.__version__,"python":platform.python_version(),
            "off_shell_identity_exact_zero":True,"gauss_bonnet_coupling_symbolic":True,
            "all_five_euler_lagrange_residuals_included":True,
            "simplified_operations":int(sp.count_ops(identity)),
            "reduced_identity_sha256":hashlib.sha256(sp.srepr(sp.cancel(identity)).encode()).hexdigest()},
        "claim_ceiling":"This verifies the radial-diffeomorphism identity of the published reduced one-dimensional action. It does not solve the finite-alpha spin-squared BVP, certify a numerical solution, prove existence, or qualify a nonlinear branch."
    }
    rendered=json.dumps(fixture,indent=2,sort_keys=True)+"\n"
    if args.output:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(rendered,encoding="utf-8")
    else: print(rendered)
if __name__=="__main__": main()

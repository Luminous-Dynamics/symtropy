#!/usr/bin/env python3
"""Extract action-derived O(lambda*a^2) rows for both Einstein-limit modes.

All five radial fields are varied before the radial gauge g=r^2 is imposed.
The output is an exact-rational coefficient-row fixture used by the independent
Green--Lagrange endpoint projector.

Requires Python >=3.12 and SymPy >=1.14.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import platform
from pathlib import Path
import sympy as sp


def exact_zero(label: str, expression) -> None:
    values = list(expression) if isinstance(expression, sp.MatrixBase) else [expression]
    values = [sp.factor(sp.cancel(value)) for value in values]
    if any(value != 0 for value in values):
        raise AssertionError(f"{label}: {values}")


def derive() -> dict[str, object]:
    r, eps, lam = sp.symbols("r epsilon lambda", positive=True)
    cB, cH = sp.symbols("cB cH", real=True)
    b, f, g, h, w = sp.symbols("b f g h w", positive=True)
    bp, fp, gp, hp, wp = sp.symbols("bp fp gp hp wp", real=True)
    bpp, fpp, gpp, hpp, wpp = sp.symbols("bpp fpp gpp hpp wpp", real=True)
    q0, q1, q2 = (b, f, g, h, w), (bp, fp, gp, hp, wp), (bpp, fpp, gpp, hpp, wpp)
    measure = sp.sqrt(f*h/b)

    L_e = measure * (
        bp*gp + g*bp*hp/(2*h) + b*gp**2/(2*g) + b*gp*hp/h
        + g*h*wp**2/2 + 2*b/f*(4-h/g)
    )
    L_gb = measure/g * (
        4*h*bp*gp/g + 2*(4*g-3*h)*(bp*hp/h+h*wp**2)
        - f*bp*hp*gp**2/(2*h) - f*h*gp**2*wp**2/2
    )
    L = L_e+lam*L_gb/4

    def total_derivative(expression):
        return (sum(sp.diff(expression, q)*q1p for q, q1p in zip(q0, q1))
                + sum(sp.diff(expression, q1p)*q2p for q1p, q2p in zip(q1, q2)))

    print("a", flush=True)
    momenta = {name: sp.diff(L, q1p)
               for name, q1p in zip(("b", "f", "g", "h", "w"), q1)}
    EL = {name: sp.diff(L, q)-total_derivative(momenta[name])
          for name, q in zip(("b", "f", "g", "h", "w"), q0)}

    print("b", flush=True)
    F0 = 1-r**-2
    F1 = -sp.Rational(1, 2)*(r**-2-r**-6)
    W0 = r**-4
    W1 = sp.Rational(1, 2)*(r**-4-r**-8)
    F_exact = 1-(2+lam)/(r**2*(1+sp.sqrt(1+lam*(2+lam)/r**4)))
    W_exact = (2+lam)/(r**4*(1+sp.sqrt(1+lam*(2+lam)/r**4)))
    exact_zero("static background coefficient", sp.diff(F_exact, lam).subs(lam, 0)-F1)
    exact_zero("linear-spin coefficient", sp.diff(W_exact, lam).subs(lam, 0)-W1)

    Bmp, Fmp, Hmp = r**-6-r**-2, r**-4-r**-2, r**-2
    mb = sp.Rational(1, 2)*(1-r**-2)
    mhF, mhH = -2*r**2+3-r**-2, 2*r**4-r**2
    Bbase, Fbase, Hbase = Bmp+cB*mb, Fmp+cH*mhF, Hmp+cH*mhH
    B1f, F21f, H1f = (sp.Function(name)(r) for name in ("B1", "F21", "H1"))
    Btotal, Ftotal, Htotal = Bbase+lam*B1f, Fbase+lam*F21f, Hbase+lam*H1f
    Fbg, Wbg = F0+lam*F1, W0+lam*W1
    ansatz = {
        b: Fbg+eps**2*Btotal, f: Fbg+eps**2*Ftotal, g: r**2,
        h: r**2+eps**2*Htotal, w: eps*Wbg,
        bp: sp.diff(Fbg+eps**2*Btotal, r),
        fp: sp.diff(Fbg+eps**2*Ftotal, r), gp: 2*r,
        hp: sp.diff(r**2+eps**2*Htotal, r), wp: eps*sp.diff(Wbg, r),
        bpp: sp.diff(Fbg+eps**2*Btotal, r, 2),
        fpp: sp.diff(Fbg+eps**2*Ftotal, r, 2), gpp: sp.Integer(2),
        hpp: sp.diff(r**2+eps**2*Htotal, r, 2),
        wpp: eps*sp.diff(Wbg, r, 2),
    }
    print("c", flush=True)
    rows = {}
    for name in ("b", "f", "g", "h"):
        substituted = EL[name].subs(ansatz)
        even = sp.diff(substituted, eps, 2).subs(eps, 0)/2
        rows[name] = sp.factor(sp.diff(even, lam).subs(lam, 0))


    print("d", flush=True)
    pivot = sp.factor(sp.diff(rows["f"], F21f))
    exact_zero("positive algebraic Ef pivot", pivot-3*r**5/(r**2-1)**2)
    Frec = sp.cancel(-(rows["f"]-pivot*F21f)/pivot)
    Frec_r = sp.diff(Frec, r)
    print("e", flush=True)
    red_b = sp.factor(sp.cancel(rows["b"].subs(
        {F21f: Frec, sp.diff(F21f, r): Frec_r}, simultaneous=True)))
    print("f", flush=True)
    red_h = sp.factor(sp.cancel(rows["h"].subs(
        {F21f: Frec, sp.diff(F21f, r): Frec_r}, simultaneous=True)))

    print("g", flush=True)
    return {
        "schema_version": 1,
        "artifact_kind": "5d_egb_spin2_boundary_map_lambda1_action_rows",
        "status": "action_derived_exact_lambda_a2_coefficient_rows_for_two_horizon_modes",
        "conventions": {
            "dimension": 5, "r_H": 1, "lambda": "alpha/r_H^2", "spin_order": "O(a^2)",
            "fields_varied_independently": ["b", "f", "g", "h", "w"],
            "radial_gauge_imposed_after_variation": "g=r^2",
            "horizon_coordinates": "cB=delta(B2_prime(1)); cH=delta(H2(1))",
            "Einstein_limit_modes": {
                "b1": ["(1-r^-2)/2", "0", "0"],
                "h0": ["0", "-2*r^2+3-r^-2", "2*r^4-r^2"],
            },
        },
        "equation_rows_lambda_a2": {name: sp.sstr(expression) for name, expression in rows.items()},
        "ef_algebraic_pivot": sp.sstr(pivot),
        "ef_reconstruction_F21": sp.sstr(Frec),
        "reduced_rows_after_ef": {"b": sp.sstr(red_b), "h": sp.sstr(red_h)},
        "checks": {
            "five_fields_varied_before_gauge_fixing": True,
            "static_background_first_lambda_coefficient": True,
            "linear_spin_first_lambda_coefficient": True,
            "all_four_original_even_rows_extracted": True,
            "Ew_first_order_lambda_row": "not re-expanded in this artifact; W1 is an exact action-derived lower-order control",
            "Ef_pivot_exact_and_positive_for_r_gt_1": True,
            "Ef_reconstruction_substituted_into_Eb_Eh": True,
        },
        "provenance": {
            "python": platform.python_version(), "sympy": sp.__version__,
            "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "arithmetic": "exact symbolic arithmetic over rational functions",
        },
        "claim_ceiling": "Action-derived coefficient rows only. The endpoint Jacobian derivative is computed by the separate Green-Lagrange verifier; no finite-lambda BVP or existence theorem follows.",
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("spin2_boundary_map_lambda1_action_rows_v1.json"))
    args = parser.parse_args()
    result = derive()
    rendered = json.dumps(result, indent=2, sort_keys=True)+"\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    print(json.dumps({"status": result["status"], "output": str(args.output),
                      "sha256": hashlib.sha256(rendered.encode()).hexdigest(),
                      "source_sha256": result["provenance"]["source_sha256"],
                      "checks": result["checks"]}, indent=2))


if __name__ == "__main__":
    main()

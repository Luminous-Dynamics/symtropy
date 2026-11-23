#!/usr/bin/env python3
"""Verify the Ma-Lu fixed-boundary O(lambda a^2) profiles in all original rows.

Inputs are the public action-derived row fixture and the independently derived
fixed-(R_H,Omega_H) profiles. The horizon-mode coordinates cB,cH are the
zeroth-order MP homogeneous-mode offsets; the MP-connected branch is cB=cH=0.
The resulting horizon sensitivities are outputs, not substitutions for cB,cH.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import sympy as sp

r = sp.symbols("r", positive=True)
B = sp.Function("B1")(r)
F = sp.Function("F21")(r)
H = sp.Function("H1")(r)
cB, cH = sp.symbols("cB cH")


def exact_zero(label: str, expr: sp.Expr) -> None:
    reduced = sp.factor(sp.cancel(expr))
    if reduced != 0:
        raise AssertionError(f"{label}: {sp.sstr(reduced)}")


def derive_profiles() -> tuple[dict, dict]:
    Bexpr = (r**2-1)*(11*r**6+11*r**4+22*r**2+9)/(9*r**10)
    Fexpr = (r**2-1)*(22*r**4-11*r**2-35)/(18*r**8)
    Hexpr = -(r**2+1)*(11*r**2+3)/(6*r**6)
    subs = {
        B: Bexpr, F: Fexpr, H: Hexpr,
        sp.diff(B, r): sp.diff(Bexpr, r),
        sp.diff(B, r, 2): sp.diff(Bexpr, r, 2),
        sp.diff(F, r): sp.diff(Fexpr, r),
        sp.diff(H, r): sp.diff(Hexpr, r),
        sp.diff(H, r, 2): sp.diff(Hexpr, r, 2),
    }
    profiles = {"B1": Bexpr, "F21": Fexpr, "H1": Hexpr}
    return profiles, subs


def load_rows(path: Path) -> tuple[dict, sp.Expr]:
    data = json.loads(path.read_text(encoding="utf-8"))
    raw = data["equation_rows_lambda_a2"]
    local = {
        "r": r, "B1": sp.Function("B1"), "F21": sp.Function("F21"),
        "H1": sp.Function("H1"), "cB": cB, "cH": cH,
    }
    rows = {name: sp.sympify(value, locals=local) for name, value in raw.items()}
    frec = sp.sympify(data["ef_reconstruction_F21"], locals=local)
    if set(rows) != {"b", "f", "g", "h"}:
        raise AssertionError(f"expected four original rows, got {sorted(rows)}")
    return rows, frec


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rows", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    rows, frec = load_rows(args.rows)
    profiles, subs = derive_profiles()
    branch_subs = {**subs, cB: sp.Integer(0), cH: sp.Integer(0)}

    # All four original action-derived rows, including the radial constraint Eg.
    row_results = {}
    for name in ("b", "f", "g", "h"):
        residual = sp.factor(sp.cancel(rows[name].subs(branch_subs)))
        exact_zero(f"original E_{name} row", residual)
        row_results[name] = sp.sstr(residual)

    # Independent Ef algebraic reconstruction must recover the same F21.
    frec_residual = sp.factor(sp.cancel(frec.subs(branch_subs)-profiles["F21"]))
    exact_zero("Ef reconstruction of F21", frec_residual)

    # The horizon derivatives are outputs of these profiles, not cB/cH inputs.
    b1_horizon_slope = sp.simplify(sp.diff(profiles["B1"], r).subs(r, 1))
    h1_horizon_value = sp.simplify(profiles["H1"].subs(r, 1))
    exact_zero("B1'(1)=106/9", b1_horizon_slope-sp.Rational(106, 9))
    exact_zero("H1(1)=-14/3", h1_horizon_value+sp.Rational(14, 3))

    # Negative control 1: using the tangent outputs as zeroth-order mode offsets
    # must fail. This catches the bookkeeping mistake made in an earlier probe.
    wrong_mode_rows = {
        name: sp.factor(sp.cancel(rows[name].subs({
            **subs, cB: sp.Rational(106, 9), cH: -sp.Rational(14, 3)
        })))
        for name in rows
    }
    if not any(value != 0 for value in wrong_mode_rows.values()):
        raise AssertionError("negative control failed: tangent was accepted as cB/cH")

    # Negative control 2: perturb a true profile coefficient; original rows fail.
    bad_H = profiles["H1"] + r**-7
    bad_subs = dict(branch_subs)
    bad_subs[H] = bad_H
    bad_subs[sp.diff(H, r)] = sp.diff(bad_H, r)
    bad_subs[sp.diff(H, r, 2)] = sp.diff(bad_H, r, 2)
    bad_rows = [sp.factor(sp.cancel(rows[name].subs(bad_subs))) for name in rows]
    if not any(value != 0 for value in bad_rows):
        raise AssertionError("negative control failed: mutated H1 profile was accepted")

    result = {
        "schema_version": 1,
        "artifact_kind": "ma_lu_fixed_boundary_spin2_original_row_verification",
        "status": "all_four_original_even_rows_exact_zero",
        "branch_coordinates": {
            "cB": "0 (MP-connected zeroth-order mode offset)",
            "cH": "0 (MP-connected zeroth-order mode offset)",
            "note": "B1'(1) and H1(1) are outputs of the first-coupling sensitivity profiles, not cB/cH inputs"
        },
        "profiles": {
            "B1": sp.sstr(profiles["B1"]),
            "F21": sp.sstr(profiles["F21"]),
            "H1": sp.sstr(profiles["H1"]),
            "B1_prime_at_horizon": sp.sstr(b1_horizon_slope),
            "H1_at_horizon": sp.sstr(h1_horizon_value)
        },
        "original_equation_rows": row_results,
        "algebraic_Ef_reconstruction_residual": sp.sstr(frec_residual),
        "negative_controls": {
            "tangent_misused_as_mode_offsets_rejected": True,
            "mutated_H1_profile_rejected": True
        },
        "claim_ceiling": (
            "Exact rational identity check of the four original O(lambda a^2) "
            "even-sector action rows on the fixed-(R_H,Omega_H) Ma-Lu profiles. "
            "This is a formal first-coupling/slow-spin comparison only; it is not "
            "a finite-coupling solution, interval existence proof, or hyperbolicity result."
        )
    }
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()

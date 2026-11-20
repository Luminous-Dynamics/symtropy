#!/usr/bin/env python3
"""Independent symbolic derivation of the static spherical Bach vacuum family.

Research-only SymPy check. It derives the two reduced projections for generic
F(r), integrates the first equation analytically, and substitutes the general
integrated solution into the second equation. It is not a Lean proof and does
not address rotating solutions.

Run:
    python3 static_bach_vacuum_classification.py
"""

from __future__ import annotations

import json
import sympy as sp

r = sp.symbols("r", positive=True)
F = sp.Function("F")(r)
A = (1 - F) / r**2 + sp.diff(F, r, 2) / 2
E = sp.Rational(1, 2) * (
    (1 - F) / r**2 + sp.diff(F, r) / r - sp.diff(F, r, 2) / 2
)
X = sp.diff(A, r) + A / r

b_plus = sp.factor(
    -2 * F * X / r - sp.Rational(2, 3) * F * sp.diff(X, r)
)
b_minus = sp.factor(
    -sp.Rational(8, 3) * A * E
    - sp.Rational(4, 3) * X * (sp.diff(F, r) + F / r)
    - sp.Rational(4, 3) * F * sp.diff(X, r)
)

b_plus_factor = -sp.Rational(2, 3) * F * (
    sp.diff(X, r) + 3 * X / r
)
assert sp.simplify(b_plus - b_plus_factor) == 0

# Integrate B_+=0 on a connected interval with r>0 and F>0:
# X=C/r^3, A=D/r-C/r^2, then solve the Euler equation.
C, D, c_minus, c0, c1, c2 = sp.symbols(
    "C D c_minus c0 c1 c2", real=True
)
X_solution = C / r**3
A_solution = D / r - C / r**2
F_solution = c_minus / r + (C + 1) - D * r + c2 * r**2

A_check = sp.simplify((1 - F_solution) / r**2 + sp.diff(F_solution, r, 2) / 2)
X_check = sp.simplify(sp.diff(A_check, r) + A_check / r)
assert sp.simplify(A_check - A_solution) == 0
assert sp.simplify(X_check - X_solution) == 0

# Convert integration constants to the metric coefficients.
F_solution = F_solution.subs({C: c0 - 1, D: -c1})
substitutions = {
    F: F_solution,
    sp.diff(F, r): sp.diff(F_solution, r),
    sp.diff(F, r, 2): sp.diff(F_solution, r, 2),
    sp.diff(F, r, 3): sp.diff(F_solution, r, 3),
    sp.diff(F, r, 4): sp.diff(F_solution, r, 4),
}
b_minus_solution = sp.factor(b_minus.subs(substitutions))
Q = c0**2 - 3 * c1 * c_minus - 1
expected_b_minus = 4 * Q / (3 * r**4)
assert sp.simplify(b_minus_solution - expected_b_minus) == 0

# Independent full-component on-shell/off-shell component formula controls.
full_components = {
    "B_tt": Q * F_solution / (3 * r**4),
    "B_rr": -Q / (3 * r**4 * F_solution),
    "B_theta_theta": Q / (3 * r**2),
    "B_phi_phi_over_sin_squared": Q / (3 * r**2),
}
for expression in full_components.values():
    assert sp.simplify(expression.subs({c0: sp.Rational(7, 10),
                                         c1: sp.Rational(1, 10),
                                         c_minus: sp.Rational(-17, 10),
                                         c2: sp.Rational(-1, 100)})) == 0

off_shell = {
    c0: sp.Rational(11, 10),
    c1: sp.Rational(1, 10),
    c_minus: sp.Rational(-17, 10),
    c2: sp.Rational(-1, 100),
}
Q_off = sp.simplify(Q.subs(off_shell))
assert Q_off == sp.Rational(18, 25)
bminus_r3 = sp.simplify(expected_b_minus.subs(off_shell).subs(r, 3))
bminus_r6 = sp.simplify(expected_b_minus.subs(off_shell).subs(r, 6))
assert bminus_r3 == sp.Rational(8, 675)
assert bminus_r6 == sp.Rational(1, 1350)
assert sp.simplify(bminus_r3 / bminus_r6) == 16

result = {
    "schema_version": 1,
    "artifact_kind": "independent_symbolic_static_bach_vacuum_classification",
    "sympy_version": sp.__version__,
    "static_domain_assumptions": ["r>0", "F(r)>0", "connected interval", "classical derivatives through order four"],
    "generic_reduction": {
        "A": str(A),
        "E": str(E),
        "X": str(X),
        "B1_plus_B2": str(b_plus),
        "B1_plus_B2_factorization": str(b_plus_factor),
        "first_equation": "X' + 3 X/r = 0",
        "integrated_X": "X=C/r^3",
        "integrated_A": "A=D/r-C/r^2",
        "euler_equation": "r^2 F'' - 2F = 2Dr - 2C - 2"
    },
    "complete_solution_form": "F=c_minus/r+c0+c1*r+c2*r^2",
    "integration_constant_map": {"c0": "C+1", "c1": "-D", "c_minus": "free", "c2": "free"},
    "second_equation": str(b_minus_solution),
    "vacuum_constraint": "c0^2-3*c1*c_minus-1=0",
    "full_components": {key: str(value) for key, value in full_components.items()},
    "fixtures": {
        "schwarzschild": {"c_minus": "-2", "c0": "1", "c1": "0", "c2": "0", "Q": "0"},
        "mannheim_kazanas": {"c_minus": "-17/10", "c0": "7/10", "c1": "1/10", "c2": "-1/100", "Q": "0"},
        "off_shell": {
            "c_minus": "-17/10", "c0": "11/10", "c1": "1/10", "c2": "-1/100",
            "Q": str(Q_off), "B1_minus_B2_r3": str(bminus_r3),
            "B1_minus_B2_r6": str(bminus_r6), "ratio_r3_r6": "16"
        }
    },
    "validation": {
        "generic_B1_plus_B2_factorization_exact": True,
        "integration_candidate_reconstructs_A_and_X_exactly": True,
        "second_equation_reduces_to_Q_constraint_exactly": True,
        "on_shell_full_components_vanish": True,
        "off_shell_negative_control_exact": True,
        "lean_formalization_executed": False
    },
    "claim_ceiling": "Symbolic reduction and exact family controls only; not a Lean formal proof or a rotating-solution result."
}
print(json.dumps(result, indent=2, sort_keys=True))

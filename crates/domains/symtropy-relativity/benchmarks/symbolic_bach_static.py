#!/usr/bin/env python3
"""Research-only independent symbolic Bach-tensor oracle for a static spherical metric.

This script is NOT a production Symtropy module and is not used by the Rust
kernel. It derives the full covariant Bach tensor from the metric, connection,
Riemann, Ricci and Weyl tensors, then compares the result with the published
Mantica-Molinari static-spherical scalar reduction.

Requires SymPy 1.14.0 (the version recorded with the generated fixture).
Run:
    python3 symbolic_bach_static.py > bach_static_symbolic_fixture_v1.json

Conventions:
    ds^2 = -F(r) dt^2 + dr^2/F(r) + r^2 dOmega^2, signature (-+++)
    R^a_(bcd) = d_c Gamma^a_(db) - d_d Gamma^a_(cb)
                + Gamma^a_(ce) Gamma^e_(db) - Gamma^a_(de) Gamma^e_(cb)
    R_bd = R^a_(bad)
    Bach_kl = 2 nabla^j nabla^m C_jklm + R^jm C_jklm

The implementation specializes only after the full tensor has been derived
for a generic symbolic F(r). It is an independent static-spherical reference,
not a general arbitrary-metric Bach implementation.
"""

from __future__ import annotations

from itertools import product
import json
import sympy as sp


def clean(value: sp.Expr) -> sp.Expr:
    return sp.trigsimp(sp.simplify(value), method="fu")


def covariant_derivative(
    tensor: dict[tuple[int, ...], sp.Expr],
    indices: tuple[int, ...],
    derivative_index: int,
    gamma: dict[tuple[int, int, int], sp.Expr],
) -> sp.Expr:
    """Covariant derivative of an all-covariant sparse tensor."""
    value = sp.diff(tensor.get(indices, sp.Integer(0)), coordinates[derivative_index])
    for slot, old_index in enumerate(indices):
        for replacement in range(4):
            changed = list(indices)
            changed[slot] = replacement
            value -= (
                gamma.get((replacement, derivative_index, old_index), sp.Integer(0))
                * tensor.get(tuple(changed), sp.Integer(0))
            )
    return clean(value)


t, r, theta, phi = sp.symbols("t r theta phi", real=True)
coordinates = (t, r, theta, phi)
F = sp.Function("F")(r)
metric = sp.diag(-F, 1 / F, r**2, r**2 * sp.sin(theta)**2)
inverse = sp.diag(-1 / F, F, 1 / r**2, 1 / (r**2 * sp.sin(theta)**2))
axes = range(4)

# Levi-Civita connection.
gamma: dict[tuple[int, int, int], sp.Expr] = {}
for a, b, c in product(axes, repeat=3):
    value = sp.Integer(0)
    for d in axes:
        value += inverse[a, d] * (
            sp.diff(metric[d, c], coordinates[b])
            + sp.diff(metric[d, b], coordinates[c])
            - sp.diff(metric[b, c], coordinates[d])
        ) / 2
    value = clean(value)
    if value != 0:
        gamma[a, b, c] = value

# Riemann tensor with one leading index raised.
riemann_mixed: dict[tuple[int, int, int, int], sp.Expr] = {}
for a, b, c, d in product(axes, repeat=4):
    value = sp.diff(gamma.get((a, d, b), 0), coordinates[c])
    value -= sp.diff(gamma.get((a, c, b), 0), coordinates[d])
    for e in axes:
        value += (
            gamma.get((a, c, e), 0) * gamma.get((e, d, b), 0)
            - gamma.get((a, d, e), 0) * gamma.get((e, c, b), 0)
        )
    value = clean(value)
    if value != 0:
        riemann_mixed[a, b, c, d] = value

# Ricci tensor, scalar curvature, and fully covariant Riemann.
ricci = sp.MutableDenseMatrix.zeros(4, 4)
for a, b in product(axes, repeat=2):
    ricci[a, b] = clean(sum(riemann_mixed.get((c, a, c, b), 0) for c in axes))

scalar_curvature = clean(sum(
    inverse[a, b] * ricci[a, b] for a, b in product(axes, repeat=2)
))

riemann_covariant: dict[tuple[int, int, int, int], sp.Expr] = {}
for a, b, c, d in product(axes, repeat=4):
    value = clean(sum(metric[a, e] * riemann_mixed.get((e, b, c, d), 0) for e in axes))
    if value != 0:
        riemann_covariant[a, b, c, d] = value

# Four-dimensional Weyl tensor, all indices lowered.
weyl: dict[tuple[int, int, int, int], sp.Expr] = {}
for a, b, c, d in product(axes, repeat=4):
    value = riemann_covariant.get((a, b, c, d), sp.Integer(0))
    value -= sp.Rational(1, 2) * (
        metric[a, c] * ricci[b, d]
        - metric[a, d] * ricci[b, c]
        - metric[b, c] * ricci[a, d]
        + metric[b, d] * ricci[a, c]
    )
    value += scalar_curvature / 6 * (
        metric[a, c] * metric[b, d] - metric[a, d] * metric[b, c]
    )
    value = clean(value)
    if value != 0:
        weyl[a, b, c, d] = value

# D_jkl = nabla^m C_jklm.
weyl_divergence: dict[tuple[int, int, int], sp.Expr] = {}
for j, k, ell in product(axes, repeat=3):
    value = sum(
        inverse[m, n] * covariant_derivative(weyl, (j, k, ell, m), n, gamma)
        for m, n in product(axes, repeat=2)
        if inverse[m, n] != 0
    )
    value = clean(value)
    if value != 0:
        weyl_divergence[j, k, ell] = value

# Full covariant Bach tensor.
bach: dict[tuple[int, int], sp.Expr] = {}
for k, ell in product(axes, repeat=2):
    double_divergence = 2 * sum(
        inverse[j, a] * covariant_derivative(weyl_divergence, (j, k, ell), a, gamma)
        for j, a in product(axes, repeat=2)
        if inverse[j, a] != 0
    )
    ricci_weyl = sum(
        inverse[j, a] * inverse[m, b] * ricci[a, b] * weyl.get((j, k, ell, m), 0)
        for j, m, a, b in product(axes, repeat=4)
    )
    value = clean(double_divergence + ricci_weyl)
    if value != 0:
        bach[k, ell] = value

# Independent algebraic/differential identities for arbitrary F(r).
bach_trace = clean(sum(
    inverse[a, b] * bach.get((a, b), 0) for a, b in product(axes, repeat=2)
))
bach_divergence = []
for ell in axes:
    value = sum(
        inverse[k, a] * covariant_derivative(bach, (k, ell), a, gamma)
        for k, a in product(axes, repeat=2)
        if inverse[k, a] != 0
    )
    bach_divergence.append(clean(value))

# Published static-spherical scalar reduction, used ONLY as a cross-check.
B1_plus_B2 = clean(bach.get((0, 0), 0) / F + F * bach.get((1, 1), 0))
B1_minus_B2 = clean(4 * bach.get((2, 2), 0) / r**2)

A = (1 - F) / r**2 + sp.diff(F, r, 2) / 2
E = sp.Rational(1, 2) * (
    (1 - F) / r**2 + sp.diff(F, r) / r - sp.diff(F, r, 2) / 2
)
X = sp.diff(A, r) + A / r
published_sum = clean(-2 * F * X / r - sp.Rational(2, 3) * F * sp.diff(X, r))
published_difference = clean(
    -sp.Rational(8, 3) * A * E
    - sp.Rational(4, 3) * X * (sp.diff(F, r) + F / r)
    - sp.Rational(4, 3) * F * sp.diff(X, r)
)

# Four-term metric: specialize only after deriving the generic full tensor.
c_inverse, c_constant, c_linear, c_quadratic = sp.symbols(
    "c_inverse c_constant c_linear c_quadratic", real=True
)
four_term = c_inverse / r + c_constant + c_linear * r + c_quadratic * r**2
substitutions = {F: four_term}
for derivative_order in range(1, 5):
    substitutions[sp.diff(F, r, derivative_order)] = sp.diff(four_term, r, derivative_order)

bach_four_term = {
    index: sp.factor(value.subs(substitutions))
    for index, value in bach.items()
}
constraint = c_constant**2 - 3 * c_linear * c_inverse - 1
expected_components = {
    (0, 0): constraint * four_term / (3 * r**4),
    (1, 1): -constraint / (3 * r**4 * four_term),
    (2, 2): constraint / (3 * r**2),
    (3, 3): constraint * sp.sin(theta)**2 / (3 * r**2),
}

component_checks = {
    str(index): clean(bach_four_term.get(index, 0) - expected)
    for index, expected in expected_components.items()
}
off_diagonal = [
    clean(value.subs(substitutions))
    for index, value in bach.items()
    if index[0] != index[1]
]

on_shell = {
    c_inverse: sp.Rational(-17, 10),
    c_constant: sp.Rational(7, 10),
    c_linear: sp.Rational(1, 10),
    c_quadratic: sp.Rational(-1, 100),
}
off_shell = dict(on_shell)
off_shell[c_constant] = sp.Rational(11, 10)


def sample_record(coefficients: dict[sp.Symbol, sp.Expr], radius: int) -> dict[str, str | bool]:
    b_value = sp.factor(four_term.subs(coefficients).subs(r, radius))
    b_prime = sp.factor(sp.diff(four_term, r).subs(coefficients).subs(r, radius))
    q_value = sp.factor(constraint.subs(coefficients))
    entries = {
        "radius": str(radius),
        "lapse": str(b_value),
        "lapse_derivative": str(b_prime),
        "vacuum_constraint_Q": str(q_value),
        "B_tt": str(bach_four_term.get((0, 0), 0).subs(coefficients).subs(r, radius)),
        "B_rr": str(bach_four_term.get((1, 1), 0).subs(coefficients).subs(r, radius)),
        "B_theta_theta": str(bach_four_term.get((2, 2), 0).subs(coefficients).subs(r, radius)),
        "B1_plus_B2": str(B1_plus_B2.subs(substitutions).subs(coefficients).subs(r, radius)),
        "B1_minus_B2": str(B1_minus_B2.subs(substitutions).subs(coefficients).subs(r, radius)),
        "static_frame_domain_valid": bool(b_value > 0),
    }
    return entries


assert bach_trace == 0
assert all(value == 0 for value in bach_divergence)
assert clean(B1_plus_B2 - published_sum) == 0
assert clean(B1_minus_B2 - published_difference) == 0
assert all(value == 0 for value in component_checks.values())
assert all(value == 0 for value in off_diagonal)

fixture = {
    "schema_version": 1,
    "artifact_kind": "research_only_independent_symbolic_static_bach_oracle",
    "sympy_version": sp.__version__,
    "metric": "diag(-F(r), 1/F(r), r^2, r^2 sin(theta)^2)",
    "signature": "(-+++)",
    "riemann_convention": "R^a_(bcd)=d_c Gamma^a_(db)-d_d Gamma^a_(cb)+Gamma^a_(ce)Gamma^e_(db)-Gamma^a_(de)Gamma^e_(cb)",
    "ricci_convention": "R_bd=R^a_(bad)",
    "bach_convention": "B_kl=2 nabla^j nabla^m C_jklm + R^jm C_jklm",
    "derivation_scope": "generic static spherical F(r), then four-term specialization",
    "generic_metric_identities": {
        "bach_trace_identically_zero": bach_trace == 0,
        "covariant_divergence_components_identically_zero": [value == 0 for value in bach_divergence],
        "nonzero_bach_components": [str(index) for index in sorted(bach)],
    },
    "published_reduction": {
        "B1_plus_B2": str(B1_plus_B2),
        "B1_minus_B2": str(B1_minus_B2),
        "matches_published_B1_plus_B2": clean(B1_plus_B2 - published_sum) == 0,
        "matches_published_B1_minus_B2": clean(B1_minus_B2 - published_difference) == 0,
    },
    "four_term_ansatz": {
        "B_of_r": str(four_term),
        "vacuum_constraint_Q": str(constraint),
        "B_tt": str(bach_four_term.get((0, 0), 0)),
        "B_rr": str(bach_four_term.get((1, 1), 0)),
        "B_theta_theta": str(bach_four_term.get((2, 2), 0)),
        "B_phi_phi": str(bach_four_term.get((3, 3), 0)),
        "component_formula_checks_zero": all(value == 0 for value in component_checks.values()),
        "off_diagonal_components_zero": all(value == 0 for value in off_diagonal),
    },
    "fixtures": {
        "on_shell_MK_coefficients": {
            "c_inverse": str(on_shell[c_inverse]),
            "c_constant": str(on_shell[c_constant]),
            "c_linear": str(on_shell[c_linear]),
            "c_quadratic": str(on_shell[c_quadratic]),
        },
        "on_shell": [sample_record(on_shell, radius) for radius in (3, 6)],
        "off_shell_perturb_c_constant": {
            "c_inverse": str(off_shell[c_inverse]),
            "c_constant": str(off_shell[c_constant]),
            "c_linear": str(off_shell[c_linear]),
            "c_quadratic": str(off_shell[c_quadratic]),
        },
        "off_shell": [sample_record(off_shell, radius) for radius in (3, 6)],
        "off_shell_residual_ratio_r3_over_r6": str(
            sp.factor(
                (4 * constraint.subs(off_shell) / (3 * sp.Integer(3)**4))
                / (4 * constraint.subs(off_shell) / (3 * sp.Integer(6)**4))
            )
        ),
    },
    "claim_ceiling": (
        "Independent symbolic verification of the full static-spherical Bach tensor "
        "for a generic F(r), its trace/divergence identities, and its four-term "
        "specialization. This is not a general arbitrary-metric numerical evaluator, "
        "not a compiled Rust test, and not rotating-solution or observational validation."
    ),
}

print(json.dumps(fixture, indent=2, sort_keys=True))

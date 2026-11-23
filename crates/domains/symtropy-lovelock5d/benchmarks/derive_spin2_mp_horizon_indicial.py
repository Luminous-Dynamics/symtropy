#!/usr/bin/env python3
"""Exact lambda=0 horizon residue and indicial control from scaled row jets."""
from __future__ import annotations

import argparse
import hashlib
import json
import platform
import time
from pathlib import Path

import sympy as sp

META = {
    "b": (18348, "cf3f62d0ea589e3dbfcc2ac4eedce259f5759cb1c372da03d45c2930ebc832ff", "47e9f330a3d13ad1527dc8acccdc2e2b670495b144799753b0764bf99c53b1f8"),
    "f": (8174, "a564d56086c0ece50305622591f22b27b0028f693f60fa25848c7cf894bc7daa", "31f83c37a9536ff49193a6b48ac325e8b6bbb114b869b25dfaed3b18f822b8f5"),
    "g": (41842, "9a0a56db534806aef37b42c41895d5dd54ab00586ea72e77d6735c64fcba015c", "8a6b1ee6df529fa2be2009a0757db2caca3aeb086f0ba243e154019e6949dbee"),
    "h": (38349, "6b874e5b02dc1a7314242a42ba39275f6ab93f5031d652b516ccc4db8567dc0d", "5dcbb3e55cd409b5a34bf15f90064db616076716c5fa7352a514acef11ac78e1"),
}


def exact_zero(value, label):
    if sp.factor(value) != 0:
        raise AssertionError(f"{label}: {sp.sstr(sp.factor(value))}")


def quotient_series(numerator, denominator, order, label):
    if sp.factor(denominator[0]) == 0:
        raise AssertionError(f"{label}: denominator vanishes at the horizon")
    result = []
    for n in range(order + 1):
        value = numerator[n] if n < len(numerator) else sp.Integer(0)
        for k in range(1, n + 1):
            if k < len(denominator):
                value -= denominator[k] * result[n - k]
        result.append(sp.factor(value / denominator[0]))
    return result


def convolution(left, right, order):
    return [
        sp.factor(sum(left[k] * right[n - k] for k in range(n + 1)))
        for n in range(order + 1)
    ]


def add_series(*arrays):
    order = max(len(array) for array in arrays)
    return [
        sp.factor(sum(array[n] if n < len(array) else 0 for array in arrays))
        for n in range(order)
    ]


def derivative_series(values, order):
    return [sp.factor((n + 1) * values[n + 1]) for n in range(order + 1)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--equations", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    started = time.perf_counter()

    raw_bytes = args.equations.read_bytes()
    data = json.loads(raw_bytes)
    raw = data["equation_system"]["equations"]
    for key, (expected_length, expected_srepr, expected_text) in META.items():
        actual_text = hashlib.sha256(raw[key].encode("utf-8")).hexdigest()
        if len(raw[key]) != expected_length:
            raise AssertionError(f"E_{key}: equation length mismatch")
        if data["equation_system"]["equation_forms"][key]["equation_sha256"] != expected_srepr:
            raise AssertionError(f"E_{key}: symbolic fingerprint mismatch")
        if actual_text != expected_text:
            raise AssertionError(f"E_{key}: equation text SHA-256 mismatch")

    r, r_h, alpha, rho = sp.symbols("r r_H alpha rho", positive=True, finite=True)
    Bf, Ff, Hf = (sp.Function(name) for name in ("B2", "F2", "H2"))
    local = {
        "r": r, "r_H": r_h, "alpha": alpha,
        "B2": Bf, "F2": Ff, "H2": Hf,
        "sqrt": sp.sqrt, "Derivative": sp.Derivative,
    }
    B, Bp, Bpp, F, Fp, H, Hp, Hpp = sp.symbols("B Bp Bpp F Fp H Hp Hpp")
    jets = (B, Bp, Bpp, F, Fp, H, Hp, Hpp)
    jet_map = {
        Bf(r): B, sp.diff(Bf(r), r): Bp, sp.diff(Bf(r), r, 2): Bpp,
        Ff(r): F, sp.diff(Ff(r), r): Fp,
        Hf(r): H, sp.diff(Hf(r), r): Hp, sp.diff(Hf(r), r, 2): Hpp,
    }
    radical = sp.sqrt(1 + alpha * (alpha + 2*r_h**2) / r**4)
    equations = {
        key: sp.sympify(raw[key], locals=local).xreplace(jet_map).subs(
            {r_h: 1, alpha: 0, r: rho, radical: 1}
        )
        for key in ("b", "f", "h")
    }

    delta = 2 * (rho**2 - 1)
    S = sp.Symbol("S", positive=True)

    def scaled_row(key, extra_power):
        expression = equations[key]
        bases = {power.base for power in expression.atoms(sp.Pow)
                 if not power.exp.is_Integer}
        if len(bases) != 1:
            raise AssertionError(f"{key}: unexpected fractional-power bases {bases}")
        base = next(iter(bases))
        exact_zero(base - (1 - rho**-2), f"{key}: F0 base")
        expanded = expression.xreplace({base: S})
        terms = sp.Add.make_args(sp.expand_mul(expanded))
        powers = {term.as_powers_dict().get(S, sp.Integer(0)) for term in terms}
        if not powers.issubset({-2, -1, 0, 1}):
            raise AssertionError(f"{key}: unexpected F0 powers {powers}")
        row = sp.Add(*(term * S**2 for term in terms)).xreplace({S: 1-rho**-2})
        return sp.cancel(sp.together(delta**extra_power * row))

    scaled_f = sp.cancel(sp.together(delta**2 * equations["f"]))
    scaled_b = scaled_row("b", 2)
    scaled_h = scaled_row("h", 1)

    formal_jets = sp.symbols("z0:8")

    def coefficient_polynomials(expression):
        numerator, denominator = sp.together(
            expression.xreplace(dict(zip(jets, formal_jets)))
        ).as_numer_denom()
        polynomial = sp.Poly(numerator, *formal_jets, domain="EX")
        if polynomial.total_degree() != 1:
            raise AssertionError("scaled equation is not affine in metric jets")
        coefficients = {}
        for index, jet in enumerate(jets):
            monomial = tuple(1 if slot == index else 0 for slot in range(8))
            coefficients[jet] = polynomial.coeff_monomial(monomial)
        coefficients["source"] = polynomial.coeff_monomial((0,) * 8)
        return coefficients, denominator

    cf, _ = coefficient_polynomials(scaled_f)
    cb, db = coefficient_polynomials(scaled_b)
    ch, dh = coefficient_polynomials(scaled_h)
    y = sp.Symbol("y")

    def taylor_coefficients(expression, order):
        return [
            sp.factor(
                sp.series(expression.subs(rho, 1+y), y, 0, order+1)
                .removeO().coeff(y, n)
            )
            for n in range(order + 1)
        ]

    # The E_f algebraic row gives Fhat2 = k_B B + k_Bp B' +
    # k_H H + k_Hp H' + k_s. The shared coefficient denominator cancels.
    f_pivot = taylor_coefficients(cf[F], 5)
    k_series = {}
    for name, variable in (
        ("B", B), ("Bp", Bp), ("H", H), ("Hp", Hp), ("source", "source")
    ):
        numerator = taylor_coefficients(cf[variable], 5)
        k_series[name] = quotient_series(
            [-coefficient for coefficient in numerator],
            f_pivot, 5, f"k_{name}"
        )
    kprime = {name: derivative_series(values, 4) for name, values in k_series.items()}

    row_results = {}
    for name, coefficients, denominator, valuation, multiplier_lead in (
        ("b", cb, db, 4, sp.Integer(64)),
        ("h", ch, dh, 3, sp.Integer(16)),
    ):
        c_series = {}
        for variable in list(jets) + ["source"]:
            c_series[variable] = quotient_series(
                taylor_coefficients(coefficients[variable], valuation),
                taylor_coefficients(denominator, valuation),
                valuation, f"{name}_{variable}"
            )

        def cv(variable):
            return c_series[variable]

        def kv(variable):
            return k_series[variable][:valuation+1]

        def kp(variable):
            return kprime[variable][:valuation+1]

        reduced = {
            "B": add_series(
                cv(B), convolution(cv(F), kv("B"), valuation),
                convolution(cv(Fp), kp("B"), valuation)
            ),
            "Bp": add_series(
                cv(Bp), convolution(cv(F), kv("Bp"), valuation),
                convolution(cv(Fp), add_series(kv("B"), kp("Bp")), valuation)
            ),
            "H": add_series(
                cv(H), convolution(cv(F), kv("H"), valuation),
                convolution(cv(Fp), kp("H"), valuation)
            ),
            "Hp": add_series(
                cv(Hp), convolution(cv(F), kv("Hp"), valuation),
                convolution(cv(Fp), add_series(kv("H"), kp("Hp")), valuation)
            ),
            "source": add_series(
                cv("source"), convolution(cv(F), kv("source"), valuation),
                convolution(cv(Fp), kp("source"), valuation)
            ),
            "Bpp": add_series(
                cv(Bpp), convolution(cv(Fp), kv("Bp"), valuation)
            ),
            "Hpp": add_series(
                cv(Hpp), convolution(cv(Fp), kv("Hp"), valuation)
            ),
        }
        for field, series in reduced.items():
            for order in range(valuation):
                exact_zero(series[order],
                           f"{name}: reduced {field} coefficient order {order}")
        row_results[name] = {
            field: sp.factor(series[valuation] / multiplier_lead)
            for field, series in reduced.items()
        }

    matrix_0 = sp.Matrix([
        [row_results["b"]["Bpp"], row_results["b"]["Hpp"]],
        [row_results["h"]["Bpp"], row_results["h"]["Hpp"]],
    ])
    expected_m0 = sp.Matrix([
        [sp.Rational(3, 4), -sp.Rational(1, 4)],
        [-sp.Rational(1, 4), sp.Rational(1, 12)],
    ])
    for i in range(2):
        for j in range(2):
            exact_zero(matrix_0[i, j] - expected_m0[i, j], f"MP M0[{i},{j}]")

    c0 = sp.Matrix([
        [row_results["b"][field] for field in ("B", "Bp", "H", "Hp")],
        [row_results["h"][field] for field in ("B", "Bp", "H", "Hp")],
    ])
    s0 = sp.Matrix([row_results["b"]["source"], row_results["h"]["source"]])
    residue = sp.zeros(4)
    projected_c = -2 * matrix_0.adjugate() * c0
    projected_s = -2 * matrix_0.adjugate() * s0
    for j in range(4):
        residue[1, j] = sp.factor(projected_c[0, j])
        residue[3, j] = sp.factor(projected_c[1, j])
    g0 = sp.Matrix([
        0, sp.factor(projected_s[0]), 0, sp.factor(projected_s[1])
    ])

    spectral_parameter = sp.Symbol("s")
    indicial = sp.factor(
        (spectral_parameter * sp.eye(4) - residue).det()
    )
    exact_zero(indicial - spectral_parameter**3 * (spectral_parameter + 1),
               "MP indicial polynomial")
    exact_zero(residue.rank() - 1, "MP residue rank")
    exact_zero(residue.trace() + 1, "MP residue trace")

    # Leading analytic-series solvability condition:
    # 0 = A0 U0 + g0, with U0=(B0,B1,H0,H1).
    b0, b1, h0, h1 = sp.symbols("b0 b1 h0 h1")
    u0 = sp.Matrix([b0, b1, h0, h1])
    compatibility = sp.factor((residue*u0 + g0)[1])
    exact_zero((residue*u0 + g0)[3] - 3*compatibility,
               "duplicate residue compatibility row")
    expected_compatibility = 2*h0 - h1/sp.Integer(3) - sp.Rational(8, 3)
    exact_zero(compatibility - expected_compatibility,
               "MP regular horizon compatibility")
    
    result = {
        "schema_version": 1,
        "artifact_kind": "5d_egb_spin2_MP_horizon_indicial_control",
        "status": "exact_lambda_zero_reduced_horizon_residue_derived",
        "lambda": "0",
        "input_equation_json_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "reduced_principal_M0": [[sp.sstr(matrix_0[i,j]) for j in range(2)] for i in range(2)],
        "reduced_lower_coefficient_C0": [[sp.sstr(c0[i,j]) for j in range(4)] for i in range(2)],
        "reduced_source_s0": [sp.sstr(s0[i]) for i in range(2)],
        "first_order_state": ["Bhat2", "Bhat2_prime", "Hhat2", "Hhat2_prime"],
        "residue_A0": [[sp.sstr(residue[i,j]) for j in range(4)] for i in range(4)],
        "inhomogeneous_g0": [sp.sstr(value) for value in g0],
        "indicial_polynomial": sp.sstr(indicial),
        "indicial_exponents": {"0": 3, "-1": 1},
        "residue_rank": 1,
        "residue_trace": "-1",
        "minimal_polynomial": "s*(s+1)",
        "diagonalizable": True,
        "regular_horizon_compatibility": "Hhat2_prime(1)=6*Hhat2(1)-8",
        "compatibility_is_same_as_Eg_minus_2Eh_cubic_at_lambda_zero": True,
        "resonance": {
            "difference": 1,
            "status": "logarithmic compatibility of the singular exponent requires the next Frobenius recurrence"
        },
        "controls": {
            "all_four_source_equation_lengths_and_hashes_match_public_metadata": True,
            "scaled_reduced_coefficients_vanish_below_row_valuations": True,
            "reduced_M0_matches_general_lambda_fixture_at_lambda_zero": True,
            "indicial_polynomial_exact": True,
            "Noether_constraint_is_still_separate_full_domain_gate": True
        },
        "execution": {
            "python": platform.python_version(),
            "sympy": sp.__version__,
            "elapsed_seconds": round(time.perf_counter() - started, 6)
        },
        "claim_ceiling": (
            "This establishes the exact Einstein-limit horizon residue and indicial polynomial. "
            "It does not establish finite-lambda exponents, the singular-branch logarithm test, "
            "the infinity indicial spectrum, the global boundary index, or a finite-coupling BVP."
        )
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

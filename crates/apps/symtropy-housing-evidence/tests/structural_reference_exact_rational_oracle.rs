// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation-diverse exact-rational oracle for the structural reference
//! corpus. This test-only oracle performs formula arithmetic with reduced i128
//! fractions, not f64, and does not call the production structural kernel.
//! It cross-checks the corpus used by the kernel tests; it is not an external
//! verifier or a Rust-to-Lean refinement proof.

use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/structural-reference-contract-vectors-v1.json"
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(mut numerator: i128, mut denominator: i128) -> Self {
        assert_ne!(denominator, 0, "rational denominator must be nonzero");
        if denominator < 0 {
            numerator = numerator.checked_neg().expect("bounded rational sign");
            denominator = denominator.checked_neg().expect("bounded rational sign");
        }
        let common = gcd_u128(numerator.unsigned_abs(), denominator as u128) as i128;
        Self {
            numerator: numerator / common,
            denominator: denominator / common,
        }
    }

    fn integer(value: i128) -> Self {
        Self::new(value, 1)
    }

    fn fraction(numerator: i128, denominator: i128) -> Self {
        Self::new(numerator, denominator)
    }

    fn decimal(value: &str) -> Self {
        let exponent_split = value.find(|ch| ch == 'e' || ch == 'E');
        let (mantissa, exponent) = match exponent_split {
            Some(index) => (
                &value[..index],
                value[index + 1..]
                    .parse::<i32>()
                    .expect("valid decimal exponent in JSON number"),
            ),
            None => (value, 0),
        };
        let (negative, unsigned_mantissa) = match mantissa.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        };
        let (whole, fractional) = unsigned_mantissa
            .split_once('.')
            .unwrap_or((unsigned_mantissa, ""));
        let digits = format!("{whole}{fractional}");
        let mut numerator = digits
            .parse::<i128>()
            .expect("bounded decimal digits in the reference corpus");
        if negative {
            numerator = numerator.checked_neg().expect("bounded decimal sign");
        }
        let scale = i32::try_from(fractional.len()).expect("bounded fraction length") - exponent;
        if scale >= 0 {
            Self::new(numerator, ten_pow(scale as u32))
        } else {
            Self::new(
                numerator
                    .checked_mul(ten_pow((-scale) as u32))
                    .expect("bounded decimal exponent"),
                1,
            )
        }
    }

    fn add(self, other: Self) -> Self {
        let common = gcd_u128(self.denominator as u128, other.denominator as u128) as i128;
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_factor)
            .expect("bounded rational addition");
        let right = other
            .numerator
            .checked_mul(right_factor)
            .expect("bounded rational addition");
        let numerator = left.checked_add(right).expect("bounded rational addition");
        let denominator = self
            .denominator
            .checked_mul(left_factor)
            .expect("bounded rational addition denominator");
        Self::new(numerator, denominator)
    }

    fn sub(self, other: Self) -> Self {
        self.add(Self::new(
            other.numerator.checked_neg().expect("bounded rational subtraction"),
            other.denominator,
        ))
    }

    fn mul(self, other: Self) -> Self {
        let cancel_left = gcd_u128(self.numerator.unsigned_abs(), other.denominator as u128) as i128;
        let cancel_right =
            gcd_u128(other.numerator.unsigned_abs(), self.denominator as u128) as i128;
        let numerator = (self.numerator / cancel_left)
            .checked_mul(other.numerator / cancel_right)
            .expect("bounded rational multiplication");
        let denominator = (self.denominator / cancel_right)
            .checked_mul(other.denominator / cancel_left)
            .expect("bounded rational multiplication denominator");
        Self::new(numerator, denominator)
    }

    fn div(self, other: Self) -> Self {
        assert_ne!(other.numerator, 0, "rational division by zero");
        self.mul(Self::new(other.denominator, other.numerator))
    }

    fn pow(self, exponent: u32) -> Self {
        let mut result = Self::integer(1);
        for _ in 0..exponent {
            result = result.mul(self);
        }
        result
    }

    fn abs(self) -> Self {
        Self::new(
            self.numerator.checked_abs().expect("bounded rational absolute value"),
            self.denominator,
        )
    }

    fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    fn magnitude_cmp(self, other: Self) -> Ordering {
        cmp_nonnegative_fractions(
            self.numerator.unsigned_abs(),
            self.denominator as u128,
            other.numerator.unsigned_abs(),
            other.denominator as u128,
        )
    }
}

fn ten_pow(power: u32) -> i128 {
    let mut result = 1_i128;
    for _ in 0..power {
        result = result.checked_mul(10).expect("bounded decimal power of ten");
    }
    result
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Compare nonnegative fractions without cross-multiplication overflow.
fn cmp_nonnegative_fractions(
    mut left_numerator: u128,
    mut left_denominator: u128,
    mut right_numerator: u128,
    mut right_denominator: u128,
) -> Ordering {
    let mut reverse_order = false;
    loop {
        let left_whole = left_numerator / left_denominator;
        let right_whole = right_numerator / right_denominator;
        if left_whole != right_whole {
            let order = left_whole.cmp(&right_whole);
            return if reverse_order { order.reverse() } else { order };
        }

        let left_remainder = left_numerator % left_denominator;
        let right_remainder = right_numerator % right_denominator;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reverse_order {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if reverse_order {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {
                left_numerator = left_denominator;
                left_denominator = left_remainder;
                right_numerator = right_denominator;
                right_denominator = right_remainder;
                reverse_order = !reverse_order;
            }
        }
    }
}

fn q(numerator: i128, denominator: i128) -> Rational {
    Rational::fraction(numerator, denominator)
}

fn bind_input(inputs: &Value, field: &str, pinned: Rational) -> Rational {
    let observed = inputs
        .get(field)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("expected numeric input {field}"));
    assert_eq!(
        observed,
        pinned.to_f64(),
        "corpus input {field} drifted from the separately pinned rational primitive"
    );
    pinned
}

#[derive(Clone, Copy)]
struct Section {
    area: Rational,
    inertia: Rational,
    extreme_fiber: Rational,
}

fn section(width: Rational, depth: Rational) -> Section {
    Section {
        area: width.mul(depth),
        inertia: width.mul(depth.pow(3)).div(q(12, 1)),
        extreme_fiber: depth.div(q(2, 1)),
    }
}

fn put(outputs: &mut BTreeMap<String, Rational>, name: &str, value: Rational) {
    outputs.insert(name.to_string(), value);
}

fn oracle_outputs(vector: &Value) -> BTreeMap<String, Rational> {
    let vector_id = vector["vector_id"]
        .as_str()
        .expect("vector_id must be a string");
    let inputs = &vector["inputs"];
    let mut outputs = BTreeMap::new();

    match vector_id {
        "geometry.rectangular-section.cantilever-fixture.v1" => {
            assert_eq!(
                vector["contract_id"].as_str(),
                Some("geometry.rectangular_section.second_moment.v1")
            );
            let width = bind_input(inputs, "width_m", q(96, 1_000));
            let depth = bind_input(inputs, "depth_m", q(1, 10));
            let s = section(width, depth);
            put(&mut outputs, "area_m2", s.area);
            put(&mut outputs, "second_moment_area_m4", s.inertia);
            put(&mut outputs, "extreme_fiber_distance_m", s.extreme_fiber);
        }
        "mechanics.axial-bar.fixture.v1" => {
            assert_eq!(
                vector["contract_id"].as_str(),
                Some("mechanics.axial_bar.extension.v1")
            );
            let force = bind_input(inputs, "load_N", q(12_000, 1));
            let length = bind_input(inputs, "length_m", q(3, 1));
            let modulus = bind_input(inputs, "elastic_modulus_Pa", q(200_000_000_000, 1));
            let width = bind_input(inputs, "width_m", q(1, 20));
            let depth = bind_input(inputs, "depth_m", q(1, 10));
            let s = section(width, depth);
            put(&mut outputs, "area_m2", s.area);
            put(&mut outputs, "axial_stress_Pa", force.div(s.area));
            put(
                &mut outputs,
                "free_end_extension_m",
                force.mul(length).div(modulus.mul(s.area)),
            );
            put(&mut outputs, "support_reaction_magnitude_N", force);
        }
        "mechanics.cantilever.tip-load.fixture.v1" => {
            assert_eq!(
                vector["contract_id"].as_str(),
                Some("mechanics.cantilever.tip_deflection.v1")
            );
            let force = bind_input(inputs, "load_N", q(1_000, 1));
            let span = bind_input(inputs, "span_m", q(2, 1));
            let modulus = bind_input(inputs, "elastic_modulus_Pa", q(200_000_000_000, 1));
            let width = bind_input(inputs, "width_m", q(96, 1_000));
            let depth = bind_input(inputs, "depth_m", q(1, 10));
            let s = section(width, depth);
            let moment = force.mul(span);
            put(&mut outputs, "area_m2", s.area);
            put(&mut outputs, "second_moment_area_m4", s.inertia);
            put(&mut outputs, "extreme_fiber_distance_m", s.extreme_fiber);
            put(
                &mut outputs,
                "maximum_deflection_m",
                force.mul(span.pow(3)).div(q(3, 1).mul(modulus).mul(s.inertia)),
            );
            put(&mut outputs, "maximum_bending_moment_Nm", moment);
            put(
                &mut outputs,
                "maximum_bending_stress_Pa",
                moment.mul(s.extreme_fiber).div(s.inertia),
            );
            put(&mut outputs, "support_reaction_magnitude_N", force);
        }
        "mechanics.simply-supported.midpoint-load.fixture.v1" => {
            assert_eq!(
                vector["contract_id"].as_str(),
                Some("mechanics.simply_supported.midpoint_point_load.v1")
            );
            let span = bind_input(inputs, "span_m", q(4, 1));
            let modulus = bind_input(inputs, "elastic_modulus_Pa", q(200_000_000_000, 1));
            let width = bind_input(inputs, "width_m", q(1, 10));
            let depth = bind_input(inputs, "depth_m", q(1, 5));
            let force = bind_input(&vector["load_case"], "force_N", q(5_000, 1));
            let s = section(width, depth);
            let moment = force.mul(span).div(q(4, 1));
            put(&mut outputs, "area_m2", s.area);
            put(&mut outputs, "second_moment_area_m4", s.inertia);
            put(&mut outputs, "extreme_fiber_distance_m", s.extreme_fiber);
            put(
                &mut outputs,
                "maximum_deflection_m",
                force.mul(span.pow(3)).div(q(48, 1).mul(modulus).mul(s.inertia)),
            );
            put(&mut outputs, "maximum_bending_moment_Nm", moment);
            put(
                &mut outputs,
                "maximum_bending_stress_Pa",
                moment.mul(s.extreme_fiber).div(s.inertia),
            );
            put(
                &mut outputs,
                "support_reaction_magnitude_N",
                force.div(q(2, 1)),
            );
        }
        "mechanics.simply-supported.uniform-load.fixture.v1" => {
            assert_eq!(
                vector["contract_id"].as_str(),
                Some("mechanics.simply_supported.uniform_load.v1")
            );
            let span = bind_input(inputs, "span_m", q(4, 1));
            let modulus = bind_input(inputs, "elastic_modulus_Pa", q(200_000_000_000, 1));
            let width = bind_input(inputs, "width_m", q(1, 10));
            let depth = bind_input(inputs, "depth_m", q(1, 5));
            let load = bind_input(&vector["load_case"], "load_N_per_m", q(1_000, 1));
            let s = section(width, depth);
            let moment = load.mul(span.pow(2)).div(q(8, 1));
            put(&mut outputs, "area_m2", s.area);
            put(&mut outputs, "second_moment_area_m4", s.inertia);
            put(&mut outputs, "extreme_fiber_distance_m", s.extreme_fiber);
            put(
                &mut outputs,
                "maximum_deflection_m",
                q(5, 1)
                    .mul(load)
                    .mul(span.pow(4))
                    .div(q(384, 1).mul(modulus).mul(s.inertia)),
            );
            put(&mut outputs, "maximum_bending_moment_Nm", moment);
            put(
                &mut outputs,
                "maximum_bending_stress_Pa",
                moment.mul(s.extreme_fiber).div(s.inertia),
            );
            put(
                &mut outputs,
                "support_reaction_magnitude_N",
                load.mul(span).div(q(2, 1)),
            );
        }
        other => panic!("unknown pinned vector ID: {other}"),
    }

    outputs
}

fn json_rational(value: &Value) -> Rational {
    let number = value.as_number().expect("expected numeric JSON value");
    Rational::decimal(&number.to_string())
}

fn within_relative_tolerance(actual: Rational, expected: Rational, tolerance: Rational) -> bool {
    let error = actual.sub(expected).abs();
    let allowed = tolerance.mul(expected.abs());
    error.magnitude_cmp(allowed) != Ordering::Greater
}

#[test]
fn independent_rational_oracle_matches_all_28_pinned_outputs() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("pinned vector corpus must be valid JSON");
    assert_eq!(
        corpus["corpus_id"].as_str(),
        Some("symtropy.structural-reference-contract-vectors.v1")
    );
    assert_eq!(
        corpus["status"].as_str(),
        Some("ANALYTICAL_REFERENCE_CONFORMANCE_ONLY")
    );
    let tolerance = json_rational(&corpus["numeric_policy"]["relative_tolerance"]);
    let vectors = corpus["vectors"].as_array().expect("vectors array");
    assert_eq!(vectors.len(), 5, "all five contract families must be covered");

    let mut vector_ids = BTreeSet::new();
    let mut output_count = 0usize;
    for vector in vectors {
        let id = vector["vector_id"]
            .as_str()
            .expect("vector ID string");
        assert!(vector_ids.insert(id), "duplicate vector ID {id}");
        let outputs = oracle_outputs(vector);
        let expected = vector["expected"]
            .as_object()
            .expect("expected output object");
        let expected_keys = expected.keys().cloned().collect::<BTreeSet<_>>();
        let actual_keys = outputs.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(actual_keys, expected_keys, "{id}: exact output-key coverage");

        for (name, observed) in outputs {
            let expected_value = json_rational(
                expected
                    .get(&name)
                    .unwrap_or_else(|| panic!("{id}: missing oracle output {name}")),
            );
            assert!(
                within_relative_tolerance(observed, expected_value, tolerance),
                "{id}: exact rational oracle disagrees for {name}: computed {observed:?}, corpus {expected_value:?}, relative tolerance {tolerance:?}"
            );
            output_count += 1;
        }
    }
    assert_eq!(vector_ids.len(), 5);
    assert_eq!(output_count, 28, "every declared numeric output must be independently recomputed");
}

#[test]
fn exact_rational_oracle_sensitivity_rejects_four_missing_length_powers() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("pinned vector corpus must be valid JSON");
    let vectors = corpus["vectors"].as_array().expect("vectors array");
    let tolerance = json_rational(&corpus["numeric_policy"]["relative_tolerance"]);
    let mutants = [
        (
            "mechanics.axial-bar.fixture.v1",
            "free_end_extension_m",
            q(3, 1),
            "axial extension missing its length factor",
        ),
        (
            "mechanics.cantilever.tip-load.fixture.v1",
            "maximum_deflection_m",
            q(2, 1),
            "cantilever deflection missing one span power",
        ),
        (
            "mechanics.simply-supported.midpoint-load.fixture.v1",
            "maximum_deflection_m",
            q(4, 1),
            "midpoint-load deflection missing one span power",
        ),
        (
            "mechanics.simply-supported.uniform-load.fixture.v1",
            "maximum_deflection_m",
            q(4, 1),
            "uniform-load deflection missing one span power",
        ),
    ];

    for (vector_id, output_key, length_factor, label) in mutants {
        let vector = vectors
            .iter()
            .find(|candidate| candidate["vector_id"].as_str() == Some(vector_id))
            .unwrap_or_else(|| panic!("missing sensitivity vector {vector_id}"));
        let correct = oracle_outputs(vector)
            .get(output_key)
            .copied()
            .unwrap_or_else(|| panic!("{vector_id}: missing output {output_key}"));
        let expected = json_rational(&vector["expected"][output_key]);
        let mutated = correct.div(length_factor);
        assert!(
            !within_relative_tolerance(mutated, expected, tolerance),
            "tolerance unexpectedly accepted seeded mutant: {label}"
        );
    }
}

#[cfg(test)]
mod rational_arithmetic_tests {
    use super::*;

    #[test]
    fn fraction_comparison_does_not_need_cross_products() {
        assert_eq!(
            cmp_nonnegative_fractions(1, 2, 2, 3),
            Ordering::Less
        );
        assert_eq!(
            cmp_nonnegative_fractions(2, 3, 1, 2),
            Ordering::Greater
        );
        assert_eq!(
            cmp_nonnegative_fractions(u128::MAX - 1, u128::MAX, 1, 1),
            Ordering::Less
        );
    }

    #[test]
    fn decimal_parser_handles_scientific_notation_exactly() {
        assert_eq!(Rational::decimal("1e-12"), q(1, 1_000_000_000_000));
        assert_eq!(Rational::decimal("2.5e2"), q(250, 1));
        assert_eq!(Rational::decimal("0.0005"), q(1, 2_000));
    }
}

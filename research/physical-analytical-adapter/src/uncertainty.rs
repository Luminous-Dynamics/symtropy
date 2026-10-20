// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative interval propagation for the qualified closed-form cases.
//!
//! Bounds describe admissible input values, not probability distributions. Each
//! arithmetic operation rounds outwards by one representable f64 value where
//! possible. The module does not qualify model-form, measurement, or solver error.

use std::collections::{BTreeMap, BTreeSet};

pub type Check<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub fn new(lower: f64, upper: f64) -> Check<Self> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err("interval endpoints must be finite".to_owned());
        }
        if lower > upper {
            return Err("interval lower bound exceeds upper bound".to_owned());
        }
        Ok(Self { lower, upper })
    }

    pub fn exact(value: f64) -> Check<Self> {
        Self::new(value, value)
    }

    pub fn contains(&self, value: f64) -> bool {
        value.is_finite() && self.lower <= value && value <= self.upper
    }

    pub fn require_positive(&self, name: &str) -> Check<()> {
        if self.lower > 0.0 {
            Ok(())
        } else {
            Err(format!("input bound {name} must be strictly positive"))
        }
    }

    fn multiply(self, rhs: Self) -> Check<Self> {
        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];
        finite_values(&products, "interval multiplication")?;
        let minimum = products.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lower = outward_down(minimum)?;
        let upper = outward_up(maximum)?;
        // Preserve known non-negative products (e.g. absolute stress) instead
        // of allowing outward rounding to create a negative subnormal bound.
        let known_nonnegative =
            (self.lower >= 0.0 && rhs.lower >= 0.0) || (self.upper <= 0.0 && rhs.upper <= 0.0);
        let lower = if known_nonnegative {
            lower.max(0.0)
        } else {
            lower
        };
        Self::new(lower, upper)
    }

    fn divide(self, rhs: Self) -> Check<Self> {
        if rhs.lower <= 0.0 && rhs.upper >= 0.0 {
            return Err("interval divisor must not include zero".to_owned());
        }
        let quotients = [
            self.lower / rhs.lower,
            self.lower / rhs.upper,
            self.upper / rhs.lower,
            self.upper / rhs.upper,
        ];
        finite_values(&quotients, "interval division")?;
        let minimum = quotients.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = quotients.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lower = outward_down(minimum)?;
        let upper = outward_up(maximum)?;
        let lower = if self.lower >= 0.0 && rhs.lower > 0.0 {
            lower.max(0.0)
        } else {
            lower
        };
        Self::new(lower, upper)
    }

    fn absolute(self) -> Check<Self> {
        let maximum = self.lower.abs().max(self.upper.abs());
        let upper = outward_up(maximum)?;
        let lower = if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            outward_down(self.lower.abs().min(self.upper.abs()))?.max(0.0)
        };
        Self::new(lower, upper)
    }

    fn positive_power(self, exponent: i32, name: &str) -> Check<Self> {
        if self.lower <= 0.0 {
            return Err(format!("input bound {name} must be strictly positive"));
        }
        if exponent <= 0 {
            return Err("interval positive_power exponent must be positive".to_owned());
        }
        let mut result = Self::exact(1.0)?;
        for _ in 0..exponent {
            result = result.multiply(self)?;
        }
        Ok(result)
    }

    fn scale_positive(self, factor: f64) -> Check<Self> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err("interval scale factor must be positive and finite".to_owned());
        }
        self.multiply(Self::exact(factor)?)
    }
}

/// Propagate independent bounds through the named formula. The nominal map is
/// checked against every interval, but the returned interval does not imply
/// statistical coverage or include unmodeled uncertainty sources.
pub fn propagate(
    case_id: &str,
    nominal: &BTreeMap<String, f64>,
    bounds: &BTreeMap<String, Interval>,
) -> Check<BTreeMap<String, Interval>> {
    let (expected_keys, positive_keys): (&[&str], &[&str]) = match case_id {
        "axial_bar" => (
            &["force_n", "length_m", "area_m2", "youngs_modulus_pa"],
            &["length_m", "area_m2", "youngs_modulus_pa"],
        ),
        "cantilever_tip_load" => (
            &[
                "force_n",
                "length_m",
                "youngs_modulus_pa",
                "second_moment_area_m4",
                "outer_fiber_distance_m",
            ],
            &[
                "length_m",
                "youngs_modulus_pa",
                "second_moment_area_m4",
                "outer_fiber_distance_m",
            ],
        ),
        "simply_supported_uniform_load" => (
            &[
                "load_n_per_m",
                "length_m",
                "youngs_modulus_pa",
                "second_moment_area_m4",
                "outer_fiber_distance_m",
            ],
            &[
                "length_m",
                "youngs_modulus_pa",
                "second_moment_area_m4",
                "outer_fiber_distance_m",
            ],
        ),
        other => return Err(format!("unsupported analytical case '{other}'")),
    };

    exact_keys(nominal, expected_keys, "nominal inputs")?;
    exact_interval_keys(bounds, expected_keys, "input bounds")?;

    for key in expected_keys {
        let value = nominal
            .get(*key)
            .copied()
            .ok_or_else(|| format!("missing nominal input {key}"))?;
        if !value.is_finite() {
            return Err(format!("nominal input {key} must be finite"));
        }
        let interval = bounds
            .get(*key)
            .ok_or_else(|| format!("missing input bound {key}"))?;
        Interval::new(interval.lower, interval.upper)
            .map_err(|error| format!("input bound {key}: {error}"))?;
        if !interval.contains(value) {
            return Err(format!("nominal input {key} lies outside its bounds"));
        }
    }
    for key in positive_keys {
        bounds
            .get(*key)
            .ok_or_else(|| format!("missing positive input bound {key}"))?
            .require_positive(key)?;
    }

    match case_id {
        "axial_bar" => {
            let force = bounds["force_n"];
            let length = bounds["length_m"];
            let area = bounds["area_m2"];
            let modulus = bounds["youngs_modulus_pa"];
            let stress = force.divide(area)?;
            let strain = stress.divide(modulus)?;
            let extension = force.multiply(length)?.divide(area)?.divide(modulus)?;
            Ok(BTreeMap::from([
                ("axial_stress_pa".to_owned(), stress),
                ("axial_strain".to_owned(), strain),
                ("extension_m".to_owned(), extension),
            ]))
        }
        "cantilever_tip_load" => {
            let force = bounds["force_n"];
            let length_cubed = bounds["length_m"].positive_power(3, "length_m")?;
            let modulus = bounds["youngs_modulus_pa"];
            let inertia = bounds["second_moment_area_m4"];
            let outer_fiber = bounds["outer_fiber_distance_m"];
            let deflection = force
                .multiply(length_cubed)?
                .divide(modulus)?
                .divide(inertia)?
                .divide(Interval::exact(3.0)?)?;
            let stress = force
                .absolute()?
                .multiply(bounds["length_m"])?
                .multiply(outer_fiber)?
                .divide(inertia)?;
            Ok(BTreeMap::from([
                ("tip_deflection_m".to_owned(), deflection),
                ("maximum_bending_stress_pa".to_owned(), stress),
            ]))
        }
        "simply_supported_uniform_load" => {
            let load = bounds["load_n_per_m"];
            let length_squared = bounds["length_m"].positive_power(2, "length_m")?;
            let length_fourth = bounds["length_m"].positive_power(4, "length_m")?;
            let modulus = bounds["youngs_modulus_pa"];
            let inertia = bounds["second_moment_area_m4"];
            let outer_fiber = bounds["outer_fiber_distance_m"];
            let deflection = load
                .multiply(length_fourth)?
                .scale_positive(5.0)?
                .divide(modulus)?
                .divide(inertia)?
                .divide(Interval::exact(384.0)?)?;
            let stress = load
                .absolute()?
                .multiply(length_squared)?
                .multiply(outer_fiber)?
                .divide(inertia)?
                .divide(Interval::exact(8.0)?)?;
            Ok(BTreeMap::from([
                ("midspan_deflection_m".to_owned(), deflection),
                ("maximum_bending_stress_pa".to_owned(), stress),
            ]))
        }
        _ => unreachable!("case id was checked before evaluation"),
    }
}

fn exact_keys(actual: &BTreeMap<String, f64>, expected: &[&str], label: &str) -> Check<()> {
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    compare_key_sets(&actual_set, &expected_set, label)
}

fn exact_interval_keys(
    actual: &BTreeMap<String, Interval>,
    expected: &[&str],
    label: &str,
) -> Check<()> {
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    compare_key_sets(&actual_set, &expected_set, label)
}

fn compare_key_sets(actual: &BTreeSet<&str>, expected: &BTreeSet<&str>, label: &str) -> Check<()> {
    let missing: Vec<_> = expected.difference(actual).copied().collect();
    let unknown: Vec<_> = actual.difference(expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{label}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn finite_values(values: &[f64], operation: &str) -> Check<()> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(format!(
            "{operation} overflowed or produced non-finite values"
        ))
    }
}

fn outward_down(value: f64) -> Check<f64> {
    let result = value.next_down();
    if result.is_finite() {
        Ok(result)
    } else {
        Err("outward lower rounding produced a non-finite endpoint".to_owned())
    }
}

fn outward_up(value: f64) -> Check<f64> {
    let result = value.next_up();
    if result.is_finite() {
        Ok(result)
    } else {
        Err("outward upper rounding produced a non-finite endpoint".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use physical_transformation_reference::mechanics::{
        axial_bar, cantilever_tip_load, simply_supported_uniform_load,
    };

    fn fixture(
        entries: &[(&str, f64, f64, f64)],
    ) -> (BTreeMap<String, f64>, BTreeMap<String, Interval>) {
        let nominal = entries
            .iter()
            .map(|(key, value, _, _)| ((*key).to_owned(), *value))
            .collect();
        let bounds = entries
            .iter()
            .map(|(key, _, lower, upper)| {
                (
                    (*key).to_owned(),
                    Interval::new(*lower, *upper).expect("fixture interval"),
                )
            })
            .collect();
        (nominal, bounds)
    }

    fn assert_enclosed(interval: Interval, value: f64) {
        assert!(
            interval.contains(value),
            "{interval:?} did not enclose {value:.17e}"
        );
    }

    #[test]
    fn axial_bar_bounds_enclose_independent_formula_at_nominal_inputs() {
        let (nominal, bounds) = fixture(&[
            ("force_n", 1_000.0, 900.0, 1_100.0),
            ("length_m", 2.0, 1.9, 2.1),
            ("area_m2", 0.01, 0.009, 0.011),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
        ]);
        let ranges = propagate("axial_bar", &nominal, &bounds).expect("valid bar bounds");
        let exact = axial_bar(1_000.0, 2.0, 0.01, 200.0e9).expect("nominal formula");
        assert_enclosed(ranges["axial_stress_pa"], exact.axial_stress_pa);
        assert_enclosed(ranges["axial_strain"], exact.axial_strain);
        assert_enclosed(ranges["extension_m"], exact.extension_m);
    }

    #[test]
    fn cantilever_bounds_enclose_reference_and_zero_crossing_stress_includes_zero() {
        let (nominal, bounds) = fixture(&[
            ("force_n", 0.0, -100.0, 100.0),
            ("length_m", 2.0, 1.9, 2.1),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
            ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
            ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
        ]);
        let ranges =
            propagate("cantilever_tip_load", &nominal, &bounds).expect("valid cantilever bounds");
        assert_eq!(ranges["maximum_bending_stress_pa"].lower, 0.0);
        let exact = cantilever_tip_load(0.0, 2.0, 200.0e9, 8.0e-6, 0.1).expect("nominal formula");
        assert_enclosed(ranges["tip_deflection_m"], exact.tip_deflection_m);
        assert_enclosed(
            ranges["maximum_bending_stress_pa"],
            exact.maximum_bending_stress_pa,
        );
    }

    #[test]
    fn simply_supported_bounds_enclose_reference_at_nominal_inputs() {
        let (nominal, bounds) = fixture(&[
            ("load_n_per_m", 1_000.0, 900.0, 1_100.0),
            ("length_m", 4.0, 3.8, 4.2),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
            ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
            ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
        ]);
        let ranges = propagate("simply_supported_uniform_load", &nominal, &bounds)
            .expect("valid simply supported bounds");
        let exact = simply_supported_uniform_load(1_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)
            .expect("nominal formula");
        assert_enclosed(ranges["midspan_deflection_m"], exact.midspan_deflection_m);
        assert_enclosed(
            ranges["maximum_bending_stress_pa"],
            exact.maximum_bending_stress_pa,
        );
    }

    #[test]
    fn invalid_bounds_nominal_mismatch_and_unknown_keys_fail_closed() {
        assert!(Interval::new(2.0, 1.0).is_err());
        assert!(Interval::new(f64::NAN, 1.0).is_err());
        let (nominal_for_invalid, mut invalid_bounds) = fixture(&[
            ("force_n", 1_000.0, 900.0, 1_100.0),
            ("length_m", 2.0, 1.9, 2.1),
            ("area_m2", 0.01, 0.009, 0.011),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
        ]);
        invalid_bounds.insert(
            "area_m2".to_owned(),
            Interval {
                lower: 0.02,
                upper: 0.01,
            },
        );
        assert!(propagate("axial_bar", &nominal_for_invalid, &invalid_bounds).is_err());
        invalid_bounds.insert(
            "area_m2".to_owned(),
            Interval {
                lower: 0.009,
                upper: f64::INFINITY,
            },
        );
        assert!(propagate("axial_bar", &nominal_for_invalid, &invalid_bounds).is_err());

        let (mut nominal, bounds) = fixture(&[
            ("force_n", 1_000.0, 900.0, 1_100.0),
            ("length_m", 2.0, 1.9, 2.1),
            ("area_m2", 0.01, 0.009, 0.011),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
        ]);
        nominal.insert("force_n".to_owned(), 2_000.0);
        assert!(propagate("axial_bar", &nominal, &bounds).is_err());

        let (nominal, mut bounds) = fixture(&[
            ("force_n", 1_000.0, 900.0, 1_100.0),
            ("length_m", 2.0, 1.9, 2.1),
            ("area_m2", 0.01, 0.009, 0.011),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
        ]);
        bounds.insert(
            "unreviewed_override".to_owned(),
            Interval::new(1.0, 2.0).unwrap(),
        );
        assert!(propagate("axial_bar", &nominal, &bounds).is_err());
    }

    #[test]
    fn denominator_bounds_must_remain_positive() {
        let (nominal, mut bounds) = fixture(&[
            ("force_n", 1_000.0, 900.0, 1_100.0),
            ("length_m", 2.0, 1.9, 2.1),
            ("area_m2", 0.01, 0.0, 0.011),
            ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
        ]);
        bounds.insert("area_m2".to_owned(), Interval::new(0.0, 0.011).unwrap());
        assert!(propagate("axial_bar", &nominal, &bounds).is_err());
    }

    #[test]
    fn interval_overflow_and_unsupported_cases_fail_closed() {
        let (nominal, bounds) = fixture(&[
            ("force_n", 1.0e308, 9.0e307, 1.0e308),
            ("length_m", 1.0e308, 9.0e307, 1.0e308),
            ("area_m2", 1.0, 0.9, 1.1),
            ("youngs_modulus_pa", 2.0, 1.9, 2.1),
        ]);
        assert!(propagate("axial_bar", &nominal, &bounds).is_err());
        assert!(propagate("phase_field_fracture", &BTreeMap::new(), &BTreeMap::new()).is_err());
    }

    type FixtureEntry = (&'static str, f64, f64, f64);
    type CaseFixture = (&'static str, Vec<FixtureEntry>);

    #[test]
    fn endpoint_combinations_for_all_cases_are_enclosed() {
        let cases: Vec<CaseFixture> = vec![
            (
                "axial_bar",
                vec![
                    ("force_n", 1_000.0, -1_100.0, 1_100.0),
                    ("length_m", 2.0, 1.9, 2.1),
                    ("area_m2", 0.01, 0.009, 0.011),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                ],
            ),
            (
                "cantilever_tip_load",
                vec![
                    ("force_n", 100.0, -110.0, 110.0),
                    ("length_m", 2.0, 1.9, 2.1),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                    ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
                    ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
                ],
            ),
            (
                "simply_supported_uniform_load",
                vec![
                    ("load_n_per_m", 1_000.0, -1_100.0, 1_100.0),
                    ("length_m", 4.0, 3.8, 4.2),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                    ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
                    ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
                ],
            ),
        ];

        for (case_id, entries) in cases {
            let (nominal, bounds) = fixture(&entries);
            let ranges = propagate(case_id, &nominal, &bounds).expect("valid interval fixture");
            let n = entries.len();
            for mask in 0..(1_usize << n) {
                let point: BTreeMap<String, f64> = entries
                    .iter()
                    .enumerate()
                    .map(|(index, (key, _, lower, upper))| {
                        let value = if mask & (1 << index) == 0 {
                            *lower
                        } else {
                            *upper
                        };
                        ((*key).to_owned(), value)
                    })
                    .collect();
                let values = evaluate_point(case_id, &point);
                for (output_key, value) in values {
                    assert_enclosed(ranges[&output_key], value);
                }
            }
        }
    }

    fn evaluate_point(case_id: &str, inputs: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
        match case_id {
            "axial_bar" => {
                let result = axial_bar(
                    inputs["force_n"],
                    inputs["length_m"],
                    inputs["area_m2"],
                    inputs["youngs_modulus_pa"],
                )
                .expect("endpoint formula");
                BTreeMap::from([
                    ("axial_stress_pa".to_owned(), result.axial_stress_pa),
                    ("axial_strain".to_owned(), result.axial_strain),
                    ("extension_m".to_owned(), result.extension_m),
                ])
            }
            "cantilever_tip_load" => {
                let result = cantilever_tip_load(
                    inputs["force_n"],
                    inputs["length_m"],
                    inputs["youngs_modulus_pa"],
                    inputs["second_moment_area_m4"],
                    inputs["outer_fiber_distance_m"],
                )
                .expect("endpoint formula");
                BTreeMap::from([
                    ("tip_deflection_m".to_owned(), result.tip_deflection_m),
                    (
                        "maximum_bending_stress_pa".to_owned(),
                        result.maximum_bending_stress_pa,
                    ),
                ])
            }
            "simply_supported_uniform_load" => {
                let result = simply_supported_uniform_load(
                    inputs["load_n_per_m"],
                    inputs["length_m"],
                    inputs["youngs_modulus_pa"],
                    inputs["second_moment_area_m4"],
                    inputs["outer_fiber_distance_m"],
                )
                .expect("endpoint formula");
                BTreeMap::from([
                    (
                        "midspan_deflection_m".to_owned(),
                        result.midspan_deflection_m,
                    ),
                    (
                        "maximum_bending_stress_pa".to_owned(),
                        result.maximum_bending_stress_pa,
                    ),
                ])
            }
            _ => unreachable!("case fixture is supported"),
        }
    }
    #[test]
    fn seeded_interior_samples_are_enclosed_for_every_formula_family() {
        let cases: Vec<CaseFixture> = vec![
            (
                "axial_bar",
                vec![
                    ("force_n", 1_000.0, -1_100.0, 1_100.0),
                    ("length_m", 2.0, 1.9, 2.1),
                    ("area_m2", 0.01, 0.009, 0.011),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                ],
            ),
            (
                "cantilever_tip_load",
                vec![
                    ("force_n", 100.0, -110.0, 110.0),
                    ("length_m", 2.0, 1.9, 2.1),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                    ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
                    ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
                ],
            ),
            (
                "simply_supported_uniform_load",
                vec![
                    ("load_n_per_m", 1_000.0, -1_100.0, 1_100.0),
                    ("length_m", 4.0, 3.8, 4.2),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                    ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
                    ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
                ],
            ),
        ];

        let mut seed = 0x5eed_1788_d15c_a11u64;
        for (case_id, entries) in cases {
            let (nominal, bounds) = fixture(&entries);
            let ranges = propagate(case_id, &nominal, &bounds).expect("bounded test case");
            for _ in 0..512 {
                let point: BTreeMap<String, f64> = entries
                    .iter()
                    .map(|(key, _, lower, upper)| {
                        let fraction = next_unit(&mut seed);
                        let value = lower + (upper - lower) * fraction;
                        ((*key).to_owned(), value)
                    })
                    .collect();
                for (output, value) in evaluate_point(case_id, &point) {
                    assert_enclosed(ranges[&output], value);
                }
            }
        }
    }

    fn next_unit(seed: &mut u64) -> f64 {
        // Fixed xorshift64* stream: deterministic, dependency-free exploratory
        // coverage. These samples supplement the corner/oracle tests; they are
        // not a proof that the interval extension is sound for all inputs.
        *seed ^= *seed >> 12;
        *seed ^= *seed << 25;
        *seed ^= *seed >> 27;
        let bits = seed.wrapping_mul(0x2545_f491_4f6c_dd1d);
        ((bits >> 11) as f64) / ((1_u64 << 53) as f64)
    }

}

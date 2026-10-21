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

    fn add(self, rhs: Self) -> Check<Self> {
        let lower = self.lower + rhs.lower;
        let upper = self.upper + rhs.upper;
        finite_values(&[lower, upper], "interval addition")?;
        let rounded_lower = outward_down(lower)?;
        // Addition of two known non-negative intervals remains non-negative;
        // do not let outward rounding introduce a negative subnormal at zero.
        let rounded_lower = if self.lower >= 0.0 && rhs.lower >= 0.0 {
            rounded_lower.max(0.0)
        } else {
            rounded_lower
        };
        Self::new(rounded_lower, outward_up(upper)?)
    }

    fn subtract(self, rhs: Self) -> Check<Self> {
        let lower = self.lower - rhs.upper;
        let upper = self.upper - rhs.lower;
        finite_values(&[lower, upper], "interval subtraction")?;
        Self::new(outward_down(lower)?, outward_up(upper)?)
    }

    fn square(self) -> Check<Self> {
        let endpoint_squares = [self.lower * self.lower, self.upper * self.upper];
        finite_values(&endpoint_squares, "interval square")?;
        let maximum = endpoint_squares.iter().copied().fold(0.0_f64, f64::max);
        let minimum = if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            endpoint_squares.iter().copied().fold(f64::INFINITY, f64::min)
        };
        let lower = if minimum == 0.0 {
            0.0
        } else {
            outward_down(minimum)?.max(0.0)
        };
        let upper = outward_up(maximum)?;
        Self::new(lower, upper)
    }

    fn sqrt_nonnegative(self) -> Check<Self> {
        if self.lower < 0.0 {
            return Err("interval square root requires a non-negative lower endpoint".to_owned());
        }
        let lower_root = self.lower.sqrt();
        let upper_root = self.upper.sqrt();
        finite_values(&[lower_root, upper_root], "interval square root")?;
        let lower = if lower_root == 0.0 {
            0.0
        } else {
            outward_down(lower_root)?.max(0.0)
        };
        Self::new(lower, outward_up(upper_root)?)
    }

    fn scale_positive(self, factor: f64) -> Check<Self> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err("interval scale factor must be positive and finite".to_owned());
        }
        self.multiply(Self::exact(factor)?)
    }
}

/// An interval enclosure for the free-apex response of a two-bar planar truss.
/// Every field is computed from the same declared parameter box; widths may be
/// conservative because interval dependencies are not discarded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanarTrussEnclosure {
    pub kxx_n_per_m: Interval,
    pub kxy_n_per_m: Interval,
    pub kyy_n_per_m: Interval,
    pub determinant_n2_per_m2: Interval,
    pub displacement_x_m: Interval,
    pub displacement_y_m: Interval,
    pub condition_infinity_upper: f64,
    pub condition_times_machine_epsilon_upper: f64,
}

const PLANAR_TRUSS_INPUT_KEYS: &[&str] = &[
    "span_m",
    "apex_x_m",
    "apex_height_m",
    "axial_stiffness_a_n_per_m",
    "axial_stiffness_b_n_per_m",
    "force_x_n",
    "force_y_n",
];

/// Propagate declared finite input intervals through the reduced two-DOF
/// stiffness model for an asymmetric two-bar truss.
///
/// The determinant is computed using the geometric identity
/// det(K)=kA*kB*(span*h/(LA*LB))^2 rather than Kxx*Kyy-Kxy^2; the latter
/// loses correlation and can be much less informative near collinearity.
/// If the resulting determinant enclosure does not prove strict positivity,
/// the function returns an error and no displacement enclosure is issued.
pub fn propagate_planar_truss_two_bar(
    nominal: &BTreeMap<String, f64>,
    bounds: &BTreeMap<String, Interval>,
) -> Check<PlanarTrussEnclosure> {
    exact_keys(nominal, PLANAR_TRUSS_INPUT_KEYS, "nominal truss inputs")?;
    exact_interval_keys(bounds, PLANAR_TRUSS_INPUT_KEYS, "truss input bounds")?;

    for key in PLANAR_TRUSS_INPUT_KEYS {
        let value = nominal
            .get(*key)
            .copied()
            .ok_or_else(|| format!("missing nominal truss input {key}"))?;
        if !value.is_finite() {
            return Err(format!("nominal truss input {key} must be finite"));
        }
        let interval = bounds
            .get(*key)
            .copied()
            .ok_or_else(|| format!("missing truss input bound {key}"))?;
        Interval::new(interval.lower, interval.upper)
            .map_err(|error| format!("truss input bound {key}: {error}"))?;
        if !interval.contains(value) {
            return Err(format!("nominal truss input {key} lies outside its bounds"));
        }
    }

    for key in [
        "span_m",
        "apex_height_m",
        "axial_stiffness_a_n_per_m",
        "axial_stiffness_b_n_per_m",
    ] {
        bounds[key].require_positive(key)?;
    }

    let span = bounds["span_m"];
    let apex_x = bounds["apex_x_m"];
    let height = bounds["apex_height_m"];
    let stiffness_a = bounds["axial_stiffness_a_n_per_m"];
    let stiffness_b = bounds["axial_stiffness_b_n_per_m"];
    let force_x = bounds["force_x_n"];
    let force_y = bounds["force_y_n"];

    let dx_a = apex_x;
    let dx_b = apex_x.subtract(span)?;
    let length_a = dx_a
        .square()?
        .add(height.square()?)?
        .sqrt_nonnegative()?;
    let length_b = dx_b
        .square()?
        .add(height.square()?)?
        .sqrt_nonnegative()?;
    length_a.require_positive("bar A length")?;
    length_b.require_positive("bar B length")?;

    let cosine_a = dx_a.divide(length_a)?;
    let sine_a = height.divide(length_a)?;
    let cosine_b = dx_b.divide(length_b)?;
    let sine_b = height.divide(length_b)?;

    let kxx = stiffness_a
        .multiply(cosine_a.square()?)?
        .add(stiffness_b.multiply(cosine_b.square()?)?)?;
    let kxy = stiffness_a
        .multiply(cosine_a.multiply(sine_a)?)?
        .add(stiffness_b.multiply(cosine_b.multiply(sine_b)?)?)?;
    let kyy = stiffness_a
        .multiply(sine_a.square()?)?
        .add(stiffness_b.multiply(sine_b.square()?)?)?;

    // For this geometry:
    // cA*sB - sA*cB = span*h/(LA*LB).
    // This expression is algebraically equivalent but avoids subtracting two
    // nearly equal interval products in the determinant calculation.
    let direction_cross = span
        .multiply(height)?
        .divide(length_a.multiply(length_b)?)?;
    let determinant = stiffness_a
        .multiply(stiffness_b)?
        .multiply(direction_cross.square()?)?;
    if !determinant.lower.is_finite()
        || !determinant.upper.is_finite()
        || determinant.lower <= 0.0
    {
        return Err(
            "truss determinant interval includes zero or cannot be proven strictly positive"
                .to_owned(),
        );
    }

    let displacement_x = kyy
        .multiply(force_x)?
        .subtract(kxy.multiply(force_y)?)?
        .divide(determinant)?;
    let displacement_y = kxx
        .multiply(force_y)?
        .subtract(kxy.multiply(force_x)?)?
        .divide(determinant)?;

    // Conservative condition upper bound from interval row-sum bounds for K
    // and its 2x2 inverse. This may be loose; it is a warning indicator, not
    // a sharp condition estimate or forward-error certificate.
    let k_norm_upper = kxx
        .absolute()?
        .add(kxy.absolute()?)?
        .upper
        .max(kxy.absolute()?.add(kyy.absolute()?)?.upper);
    let inv_xx_abs = kyy.absolute()?.divide(determinant)?;
    let inv_xy_abs = kxy.absolute()?.divide(determinant)?;
    let inv_yy_abs = kxx.absolute()?.divide(determinant)?;
    let inv_norm_upper = inv_xx_abs
        .add(inv_xy_abs)?
        .upper
        .max(inv_xy_abs.add(inv_yy_abs)?.upper);
    let condition_product = k_norm_upper * inv_norm_upper;
    if !condition_product.is_finite() {
        return Err("interval condition upper bound overflowed; abstaining".to_owned());
    }
    let condition_infinity_upper = outward_up(condition_product)?;
    let condition_times_machine_epsilon_upper =
        outward_up(condition_infinity_upper * f64::EPSILON)?;
    if !condition_times_machine_epsilon_upper.is_finite() {
        return Err("condition-times-machine-epsilon upper bound overflowed; abstaining".to_owned());
    }

    Ok(PlanarTrussEnclosure {
        kxx_n_per_m: kxx,
        kxy_n_per_m: kxy,
        kyy_n_per_m: kyy,
        determinant_n2_per_m2: determinant,
        displacement_x_m: displacement_x,
        displacement_y_m: displacement_y,
        condition_infinity_upper,
        condition_times_machine_epsilon_upper,
    })
}

/// Independent pointwise analytical reference for the reduced free-apex system.
/// Input keys and units match propagate_planar_truss_two_bar.
pub fn evaluate_planar_truss_two_bar_point(
    inputs: &BTreeMap<String, f64>,
) -> Check<[f64; 8]> {
    exact_keys(inputs, PLANAR_TRUSS_INPUT_KEYS, "point truss inputs")?;
    for key in PLANAR_TRUSS_INPUT_KEYS {
        if !inputs[key].is_finite() {
            return Err(format!("point truss input {key} must be finite"));
        }
    }
    let span = inputs["span_m"];
    let apex_x = inputs["apex_x_m"];
    let height = inputs["apex_height_m"];
    let stiffness_a = inputs["axial_stiffness_a_n_per_m"];
    let stiffness_b = inputs["axial_stiffness_b_n_per_m"];
    let force_x = inputs["force_x_n"];
    let force_y = inputs["force_y_n"];
    for (key, value) in [
        ("span_m", span),
        ("apex_height_m", height),
        ("axial_stiffness_a_n_per_m", stiffness_a),
        ("axial_stiffness_b_n_per_m", stiffness_b),
    ] {
        if value <= 0.0 {
            return Err(format!("point truss input {key} must be positive"));
        }
    }

    let length_a = apex_x.hypot(height);
    let dx_b = apex_x - span;
    let length_b = dx_b.hypot(height);
    if !length_a.is_finite() || !length_b.is_finite() || length_a <= 0.0 || length_b <= 0.0 {
        return Err("point truss geometry has invalid member lengths".to_owned());
    }
    let cosine_a = apex_x / length_a;
    let sine_a = height / length_a;
    let cosine_b = dx_b / length_b;
    let sine_b = height / length_b;

    let kxx = stiffness_a * cosine_a * cosine_a
        + stiffness_b * cosine_b * cosine_b;
    let kxy = stiffness_a * cosine_a * sine_a
        + stiffness_b * cosine_b * sine_b;
    let kyy = stiffness_a * sine_a * sine_a
        + stiffness_b * sine_b * sine_b;
    let direction_cross = span * height / (length_a * length_b);
    let determinant = stiffness_a * stiffness_b * direction_cross * direction_cross;
    if ![
        kxx,
        kxy,
        kyy,
        determinant,
        force_x,
        force_y,
    ]
    .iter()
    .all(|value| value.is_finite())
        || determinant <= 0.0
    {
        return Err("point truss stiffness/determinant is non-finite or singular".to_owned());
    }
    let ux = (kyy * force_x - kxy * force_y) / determinant;
    let uy = (kxx * force_y - kxy * force_x) / determinant;
    if !ux.is_finite() || !uy.is_finite() {
        return Err("point truss displacement is non-finite".to_owned());
    }
    Ok([kxx, kxy, kyy, determinant, ux, uy, length_a, length_b])
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

        let mut seed = 0x05ee_d178_8d15_ca11_u64;
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

    #[test]
    fn doubling_force_or_distributed_load_doubles_all_linear_response_enclosures() {
        let cases: Vec<CaseFixture> = vec![
            (
                "axial_bar",
                vec![
                    ("force_n", 1_000.0, 900.0, 1_100.0),
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
                    ("load_n_per_m", 1_000.0, 900.0, 1_100.0),
                    ("length_m", 4.0, 3.8, 4.2),
                    ("youngs_modulus_pa", 200.0e9, 195.0e9, 205.0e9),
                    ("second_moment_area_m4", 8.0e-6, 7.0e-6, 9.0e-6),
                    ("outer_fiber_distance_m", 0.1, 0.09, 0.11),
                ],
            ),
        ];

        for (case_id, entries) in cases {
            let (nominal, bounds) = fixture(&entries);
            let base = propagate(case_id, &nominal, &bounds).expect("base interval case");
            let load_key = if case_id == "simply_supported_uniform_load" {
                "load_n_per_m"
            } else {
                "force_n"
            };
            let mut scaled_nominal = nominal.clone();
            *scaled_nominal.get_mut(load_key).expect("load input exists") *= 2.0;
            let mut scaled_bounds = bounds.clone();
            let load = scaled_bounds
                .get_mut(load_key)
                .expect("load interval exists");
            load.lower *= 2.0;
            load.upper *= 2.0;
            let doubled =
                propagate(case_id, &scaled_nominal, &scaled_bounds).expect("doubled interval case");

            assert_eq!(
                base.keys().collect::<Vec<_>>(),
                doubled.keys().collect::<Vec<_>>()
            );
            for (output, interval) in &base {
                let doubled_interval = doubled.get(output).expect("matching output");
                assert_near(doubled_interval.lower, interval.lower * 2.0);
                assert_near(doubled_interval.upper, interval.upper * 2.0);
            }
        }
    }

    fn truss_fixture(
        height_bounds: (f64, f64),
    ) -> (BTreeMap<String, f64>, BTreeMap<String, Interval>) {
        let nominal = BTreeMap::from([
            ("span_m".to_owned(), 4.0),
            ("apex_x_m".to_owned(), 1.0),
            ("apex_height_m".to_owned(), (height_bounds.0 + height_bounds.1) / 2.0),
            ("axial_stiffness_a_n_per_m".to_owned(), 1.0e9),
            ("axial_stiffness_b_n_per_m".to_owned(), 2.0e9),
            ("force_x_n".to_owned(), 1_000.0),
            ("force_y_n".to_owned(), -500.0),
        ]);
        let bounds = BTreeMap::from([
            ("span_m".to_owned(), Interval::new(3.99, 4.01).unwrap()),
            ("apex_x_m".to_owned(), Interval::new(0.99, 1.01).unwrap()),
            ("apex_height_m".to_owned(), Interval::new(height_bounds.0, height_bounds.1).unwrap()),
            (
                "axial_stiffness_a_n_per_m".to_owned(),
                Interval::new(0.99e9, 1.01e9).unwrap(),
            ),
            (
                "axial_stiffness_b_n_per_m".to_owned(),
                Interval::new(1.98e9, 2.02e9).unwrap(),
            ),
            ("force_x_n".to_owned(), Interval::new(990.0, 1_010.0).unwrap()),
            ("force_y_n".to_owned(), Interval::new(-510.0, -490.0).unwrap()),
        ]);
        (nominal, bounds)
    }

    #[test]
    fn truss_point_intervals_enclose_independent_2x2_known_answer() {
        let (nominal, bounds) = truss_fixture((0.74, 0.76));
        let enclosure = propagate_planar_truss_two_bar(&nominal, &bounds)
            .expect("valid narrow truss parameter bounds");
        let exact = evaluate_planar_truss_two_bar_point(&nominal)
            .expect("pointwise analytical truss reference");
        assert_enclosed(enclosure.kxx_n_per_m, exact[0]);
        assert_enclosed(enclosure.kxy_n_per_m, exact[1]);
        assert_enclosed(enclosure.kyy_n_per_m, exact[2]);
        assert_enclosed(enclosure.determinant_n2_per_m2, exact[3]);
        assert_enclosed(enclosure.displacement_x_m, exact[4]);
        assert_enclosed(enclosure.displacement_y_m, exact[5]);
        assert!(enclosure.condition_infinity_upper.is_finite());
        assert!(enclosure.condition_infinity_upper >= 1.0);
        assert!(enclosure.condition_times_machine_epsilon_upper.is_finite());
    }

    #[test]
    fn truss_widened_bounds_enclose_every_frozen_corner() {
        let nominal = BTreeMap::from([
            ("span_m".to_owned(), 4.0),
            ("apex_x_m".to_owned(), 1.0),
            ("apex_height_m".to_owned(), 0.75),
            ("axial_stiffness_a_n_per_m".to_owned(), 1.0e9),
            ("axial_stiffness_b_n_per_m".to_owned(), 2.0e9),
            ("force_x_n".to_owned(), 1_000.0),
            ("force_y_n".to_owned(), -500.0),
        ]);
        let bounds = BTreeMap::from([
            ("span_m".to_owned(), Interval::new(3.9, 4.1).unwrap()),
            ("apex_x_m".to_owned(), Interval::new(0.8, 1.2).unwrap()),
            ("apex_height_m".to_owned(), Interval::new(0.6, 0.9).unwrap()),
            ("axial_stiffness_a_n_per_m".to_owned(), Interval::new(0.9e9, 1.1e9).unwrap()),
            ("axial_stiffness_b_n_per_m".to_owned(), Interval::new(1.8e9, 2.2e9).unwrap()),
            ("force_x_n".to_owned(), Interval::new(900.0, 1_100.0).unwrap()),
            ("force_y_n".to_owned(), Interval::new(-600.0, -400.0).unwrap()),
        ]);
        let enclosure = propagate_planar_truss_two_bar(&nominal, &bounds)
            .expect("wide but invertible truss bounds");
        for mask in 0..(1_usize << PLANAR_TRUSS_INPUT_KEYS.len()) {
            let point: BTreeMap<String, f64> = PLANAR_TRUSS_INPUT_KEYS
                .iter()
                .enumerate()
                .map(|(index, key)| {
                    let interval = bounds[*key];
                    let value = if mask & (1 << index) == 0 {
                        interval.lower
                    } else {
                        interval.upper
                    };
                    ((*key).to_owned(), value)
                })
                .collect();
            let exact = evaluate_planar_truss_two_bar_point(&point)
                .expect("each frozen corner is an admissible point");
            assert_enclosed(enclosure.kxx_n_per_m, exact[0]);
            assert_enclosed(enclosure.kxy_n_per_m, exact[1]);
            assert_enclosed(enclosure.kyy_n_per_m, exact[2]);
            assert_enclosed(enclosure.determinant_n2_per_m2, exact[3]);
            assert_enclosed(enclosure.displacement_x_m, exact[4]);
            assert_enclosed(enclosure.displacement_y_m, exact[5]);
        }
    }

    #[test]
    fn truss_invalid_height_or_nonpositive_member_stiffness_abstains() {
        let (nominal, _) = truss_fixture((0.01, 0.02));
        let mut crossing_zero = truss_fixture((0.0, 0.02)).1;
        assert!(
            propagate_planar_truss_two_bar(&nominal, &crossing_zero)
                .unwrap_err()
                .contains("strictly positive")
        );
        crossing_zero.insert("apex_height_m".to_owned(), Interval::new(-0.01, 0.02).unwrap());
        assert!(propagate_planar_truss_two_bar(&nominal, &crossing_zero).is_err());

        let (_, mut bad_stiffness) = truss_fixture((0.01, 0.02));
        bad_stiffness.insert(
            "axial_stiffness_a_n_per_m".to_owned(),
            Interval::new(0.0, 1.01e9).unwrap(),
        );
        assert!(propagate_planar_truss_two_bar(&nominal, &bad_stiffness).is_err());
    }

    #[test]
    fn truss_determinant_underflow_abstains_without_displacement_bounds() {
        let nominal = BTreeMap::from([
            ("span_m".to_owned(), 4.0),
            ("apex_x_m".to_owned(), 1.0),
            ("apex_height_m".to_owned(), 1.0e-320),
            ("axial_stiffness_a_n_per_m".to_owned(), 1.0e9),
            ("axial_stiffness_b_n_per_m".to_owned(), 2.0e9),
            ("force_x_n".to_owned(), 1.0),
            ("force_y_n".to_owned(), -2.0),
        ]);
        let bounds = BTreeMap::from([
            ("span_m".to_owned(), Interval::new(4.0, 4.0).unwrap()),
            ("apex_x_m".to_owned(), Interval::new(1.0, 1.0).unwrap()),
            ("apex_height_m".to_owned(), Interval::new(1.0e-320, 1.1e-320).unwrap()),
            ("axial_stiffness_a_n_per_m".to_owned(), Interval::new(1.0e9, 1.0e9).unwrap()),
            ("axial_stiffness_b_n_per_m".to_owned(), Interval::new(2.0e9, 2.0e9).unwrap()),
            ("force_x_n".to_owned(), Interval::new(1.0, 1.0).unwrap()),
            ("force_y_n".to_owned(), Interval::new(-2.0, -2.0).unwrap()),
        ]);
        let error = propagate_planar_truss_two_bar(&nominal, &bounds).unwrap_err();
        assert!(
            error.contains("determinant interval includes zero")
                || error.contains("non-finite")
                || error.contains("overflowed"),
            "unexpected underflow abstention reason: {error}"
        );
    }

    #[test]
    fn truss_wider_input_box_contains_narrower_displacement_enclosure() {
        let (wide_nominal, wide_bounds) = truss_fixture((0.7, 0.8));
        let wide = propagate_planar_truss_two_bar(&wide_nominal, &wide_bounds).unwrap();
        let (narrow_nominal, mut narrow_bounds) = truss_fixture((0.74, 0.76));
        narrow_bounds.insert("span_m".to_owned(), Interval::new(3.995, 4.005).unwrap());
        narrow_bounds.insert("apex_x_m".to_owned(), Interval::new(0.995, 1.005).unwrap());
        narrow_bounds.insert("axial_stiffness_a_n_per_m".to_owned(), Interval::new(0.995e9, 1.005e9).unwrap());
        narrow_bounds.insert("axial_stiffness_b_n_per_m".to_owned(), Interval::new(1.99e9, 2.01e9).unwrap());
        narrow_bounds.insert("force_x_n".to_owned(), Interval::new(995.0, 1_005.0).unwrap());
        narrow_bounds.insert("force_y_n".to_owned(), Interval::new(-505.0, -495.0).unwrap());
        let narrow = propagate_planar_truss_two_bar(&narrow_nominal, &narrow_bounds).unwrap();
        assert!(wide.displacement_x_m.lower <= narrow.displacement_x_m.lower);
        assert!(wide.displacement_x_m.upper >= narrow.displacement_x_m.upper);
        assert!(wide.displacement_y_m.lower <= narrow.displacement_y_m.lower);
        assert!(wide.displacement_y_m.upper >= narrow.displacement_y_m.upper);
        assert!(
            wide.condition_infinity_upper >= narrow.condition_infinity_upper,
            "wider uncertainty box should not understate the conservative condition indicator"
        );
    }

    fn assert_near(observed: f64, expected: f64) {
        let tolerance = (expected.abs() * 1.0e-12).max(1.0e-15);
        assert!(
            (observed - expected).abs() <= tolerance,
            "observed {observed:.17e}, expected {expected:.17e}, tolerance {tolerance:.17e}"
        );
    }

    #[test]
    fn deterministic_interior_samples_are_enclosed_for_all_cases() {
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

        // Fixed-seed xorshift generator keeps the exploration reproducible and
        // avoids making this test depend on an external random crate.
        let mut state = 0x6a09_e667_f3bc_c909_u64;
        for (case_id, entries) in cases {
            let (nominal, bounds) = fixture(&entries);
            let ranges = propagate(case_id, &nominal, &bounds).expect("valid interval fixture");
            for _ in 0..512 {
                let mut point = BTreeMap::new();
                for (key, _, lower, upper) in &entries {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    let unit = (state >> 11) as f64 / ((1_u64 << 53) as f64);
                    let value = lower + unit * (upper - lower);
                    point.insert((*key).to_owned(), value);
                }
                for (output_key, value) in evaluate_point(case_id, &point) {
                    assert_enclosed(ranges[&output_key], value);
                }
            }
        }
    }
}

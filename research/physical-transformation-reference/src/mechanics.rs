// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed-form, linear-elastic reference cases for the first qualification tier.
//!
//! These formulas are not a general finite-element solver. They apply only to
//! the explicitly declared idealized bar and Euler-Bernoulli cantilever cases.

use crate::Check;

const RELATIVE_TOLERANCE: f64 = 1.0e-12;
const ABSOLUTE_TOLERANCE: f64 = 1.0e-15;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxialBarResult {
    pub axial_stress_pa: f64,
    pub axial_strain: f64,
    pub extension_m: f64,
}

/// Linear-elastic uniform axial bar under a signed axial force.
///
/// Units: force N, length m, area m^2, Young's modulus Pa.
/// Tension is positive; compression is negative.
pub fn axial_bar(
    force_n: f64,
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
) -> Check<AxialBarResult> {
    require_finite(force_n, "force")?;
    require_positive_finite(length_m, "length")?;
    require_positive_finite(area_m2, "area")?;
    require_positive_finite(youngs_modulus_pa, "Young's modulus")?;

    let denominator = area_m2 * youngs_modulus_pa;
    let numerator = force_n * length_m;
    require_finite(denominator, "area times Young's modulus")?;
    require_finite(numerator, "force times length")?;

    let axial_stress_pa = force_n / area_m2;
    let axial_strain = axial_stress_pa / youngs_modulus_pa;
    let extension_m = numerator / denominator;
    require_finite(axial_stress_pa, "axial stress")?;
    require_finite(axial_strain, "axial strain")?;
    require_finite(extension_m, "extension")?;

    Ok(AxialBarResult {
        axial_stress_pa,
        axial_strain,
        extension_m,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CantileverTipLoadResult {
    pub tip_deflection_m: f64,
    pub maximum_bending_stress_pa: f64,
}

/// Cantilever with a point force applied at its free tip.
///
/// Assumptions: static, small-deflection, linear-elastic Euler-Bernoulli beam;
/// constant section; force perpendicular to the beam axis. Sign of the force
/// determines deflection direction, while maximum stress is reported as a
/// non-negative magnitude.
///
/// Units: force N, length m, Young's modulus Pa, second moment m^4,
/// outer-fiber distance m.
pub fn cantilever_tip_load(
    force_n: f64,
    length_m: f64,
    youngs_modulus_pa: f64,
    second_moment_area_m4: f64,
    outer_fiber_distance_m: f64,
) -> Check<CantileverTipLoadResult> {
    require_finite(force_n, "force")?;
    require_positive_finite(length_m, "length")?;
    require_positive_finite(youngs_modulus_pa, "Young's modulus")?;
    require_positive_finite(second_moment_area_m4, "second moment of area")?;
    require_positive_finite(outer_fiber_distance_m, "outer-fiber distance")?;

    let length_squared = length_m * length_m;
    let length_cubed = length_squared * length_m;
    let flexural_denominator = 3.0 * youngs_modulus_pa * second_moment_area_m4;
    let deflection_numerator = force_n * length_cubed;
    let bending_moment_magnitude = force_n.abs() * length_m * outer_fiber_distance_m;
    require_finite(length_cubed, "length cubed")?;
    require_positive_finite(flexural_denominator, "flexural denominator")?;
    require_finite(deflection_numerator, "deflection numerator")?;
    require_finite(bending_moment_magnitude, "bending moment")?;

    let tip_deflection_m = deflection_numerator / flexural_denominator;
    let maximum_bending_stress_pa = bending_moment_magnitude / second_moment_area_m4;
    require_finite(tip_deflection_m, "tip deflection")?;
    require_finite(maximum_bending_stress_pa, "maximum bending stress")?;

    Ok(CantileverTipLoadResult {
        tip_deflection_m,
        maximum_bending_stress_pa,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimplySupportedUniformLoadResult {
    pub midspan_deflection_m: f64,
    pub maximum_bending_stress_pa: f64,
}

/// Simply supported prismatic beam with a uniform load across the full span.
///
/// Assumptions: static, small-deflection, linear-elastic Euler-Bernoulli beam;
/// constant section; load is uniform across the span. Positive load produces
/// positive deflection by this API convention; stress is a non-negative
/// maximum magnitude.
///
/// Units: load N/m, length m, Young's modulus Pa, second moment m^4,
/// outer-fiber distance m.
pub fn simply_supported_uniform_load(
    load_n_per_m: f64,
    length_m: f64,
    youngs_modulus_pa: f64,
    second_moment_area_m4: f64,
    outer_fiber_distance_m: f64,
) -> Check<SimplySupportedUniformLoadResult> {
    require_finite(load_n_per_m, "uniform load")?;
    require_positive_finite(length_m, "length")?;
    require_positive_finite(youngs_modulus_pa, "Young's modulus")?;
    require_positive_finite(second_moment_area_m4, "second moment of area")?;
    require_positive_finite(outer_fiber_distance_m, "outer-fiber distance")?;

    let length_squared = length_m * length_m;
    let length_fourth = length_squared * length_squared;
    let flexural_denominator = 384.0 * youngs_modulus_pa * second_moment_area_m4;
    let deflection_numerator = 5.0 * load_n_per_m * length_fourth;
    let maximum_bending_moment = load_n_per_m.abs() * length_squared / 8.0;
    require_finite(length_fourth, "length to fourth power")?;
    require_positive_finite(flexural_denominator, "flexural denominator")?;
    require_finite(deflection_numerator, "deflection numerator")?;
    require_finite(maximum_bending_moment, "maximum bending moment")?;

    let midspan_deflection_m = deflection_numerator / flexural_denominator;
    let maximum_bending_stress_pa =
        maximum_bending_moment * outer_fiber_distance_m / second_moment_area_m4;
    require_finite(midspan_deflection_m, "midspan deflection")?;
    require_finite(maximum_bending_stress_pa, "maximum bending stress")?;

    Ok(SimplySupportedUniformLoadResult {
        midspan_deflection_m,
        maximum_bending_stress_pa,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MechanicsFixtureOutcome {
    pub axial_stress_pa: f64,
    pub axial_strain: f64,
    pub axial_extension_m: f64,
    pub cantilever_tip_deflection_m: f64,
    pub cantilever_maximum_bending_stress_pa: f64,
    pub simply_supported_midspan_deflection_m: f64,
    pub simply_supported_maximum_bending_stress_pa: f64,
}

/// Evaluate two frozen analytical cases and reject the fixture if a result
/// falls outside the declared f64 comparison tolerance.
pub fn run_reference_fixtures() -> Check<MechanicsFixtureOutcome> {
    let bar = axial_bar(1_000.0, 2.0, 0.01, 200.0e9)?;
    assert_close("bar stress", bar.axial_stress_pa, 100_000.0)?;
    assert_close("bar strain", bar.axial_strain, 5.0e-7)?;
    assert_close("bar extension", bar.extension_m, 1.0e-6)?;

    let cantilever = cantilever_tip_load(100.0, 2.0, 200.0e9, 8.0e-6, 0.1)?;
    assert_close(
        "cantilever tip deflection",
        cantilever.tip_deflection_m,
        1.0 / 6_000.0,
    )?;
    assert_close(
        "cantilever maximum bending stress",
        cantilever.maximum_bending_stress_pa,
        2_500_000.0,
    )?;

    let simply_supported = simply_supported_uniform_load(1_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)?;
    assert_close(
        "simply supported beam midspan deflection",
        simply_supported.midspan_deflection_m,
        1.0 / 480.0,
    )?;
    assert_close(
        "simply supported beam maximum bending stress",
        simply_supported.maximum_bending_stress_pa,
        25_000_000.0,
    )?;

    Ok(MechanicsFixtureOutcome {
        axial_stress_pa: bar.axial_stress_pa,
        axial_strain: bar.axial_strain,
        axial_extension_m: bar.extension_m,
        cantilever_tip_deflection_m: cantilever.tip_deflection_m,
        cantilever_maximum_bending_stress_pa: cantilever.maximum_bending_stress_pa,
        simply_supported_midspan_deflection_m: simply_supported.midspan_deflection_m,
        simply_supported_maximum_bending_stress_pa: simply_supported.maximum_bending_stress_pa,
    })
}

fn require_finite(value: f64, label: &str) -> Check<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{label} must be finite"))
    }
}

fn require_positive_finite(value: f64, label: &str) -> Check<()> {
    require_finite(value, label)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(format!("{label} must be positive"))
    }
}

fn assert_close(label: &str, observed: f64, expected: f64) -> Check<()> {
    let error = (observed - expected).abs();
    let allowed = (expected.abs() * RELATIVE_TOLERANCE).max(ABSOLUTE_TOLERANCE);
    if error <= allowed {
        Ok(())
    } else {
        Err(format!(
            "{label} outside tolerance: observed={observed:.17e}, expected={expected:.17e}, allowed_abs_error={allowed:.17e}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_bar_and_cantilever_fixtures_match_closed_form_values() {
        let result = run_reference_fixtures().expect("known-answer fixtures should pass");
        assert!((result.axial_stress_pa - 100_000.0).abs() <= 1.0e-8);
        assert!((result.axial_strain - 5.0e-7).abs() <= ABSOLUTE_TOLERANCE);
        assert!((result.axial_extension_m - 1.0e-6).abs() <= ABSOLUTE_TOLERANCE);
        assert!((result.cantilever_tip_deflection_m - (1.0 / 6_000.0)).abs() <= ABSOLUTE_TOLERANCE);
        assert!((result.cantilever_maximum_bending_stress_pa - 2_500_000.0).abs() <= 1.0e-6);
    }

    #[test]
    fn simply_supported_uniform_load_matches_closed_form_and_scaling_laws() {
        let base = simply_supported_uniform_load(1_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)
            .expect("valid simply supported beam");
        assert!((base.midspan_deflection_m - (1.0 / 480.0)).abs() <= ABSOLUTE_TOLERANCE);
        assert!((base.maximum_bending_stress_pa - 25_000_000.0).abs() <= 1.0e-6);

        let doubled_span = simply_supported_uniform_load(1_000.0, 8.0, 200.0e9, 8.0e-6, 0.1)
            .expect("valid doubled-span beam");
        assert!(
            (doubled_span.midspan_deflection_m / base.midspan_deflection_m - 16.0).abs() <= 1.0e-12
        );
        assert!(
            (doubled_span.maximum_bending_stress_pa / base.maximum_bending_stress_pa - 4.0).abs()
                <= 1.0e-12
        );

        let reversed_load = simply_supported_uniform_load(-1_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)
            .expect("valid reversed beam load");
        assert!(
            (reversed_load.midspan_deflection_m + base.midspan_deflection_m).abs()
                <= ABSOLUTE_TOLERANCE
        );
        assert!(
            (reversed_load.maximum_bending_stress_pa - base.maximum_bending_stress_pa).abs()
                <= 1.0e-9
        );
    }

    #[test]
    fn compression_reverses_bar_response_sign() {
        let result = axial_bar(-1_000.0, 2.0, 0.01, 200.0e9).expect("valid compressive bar case");
        assert!(result.axial_stress_pa < 0.0);
        assert!(result.axial_strain < 0.0);
        assert!(result.extension_m < 0.0);
    }

    #[test]
    fn invalid_units_domains_and_non_finite_values_fail_closed() {
        assert!(axial_bar(1.0, 0.0, 0.01, 200.0e9).is_err());
        assert!(axial_bar(1.0, 1.0, -0.01, 200.0e9).is_err());
        assert!(axial_bar(f64::NAN, 1.0, 0.01, 200.0e9).is_err());
        assert!(axial_bar(f64::MAX, f64::MAX, 0.01, 200.0e9).is_err());

        assert!(cantilever_tip_load(1.0, 1.0, 0.0, 1.0, 0.1).is_err());
        assert!(cantilever_tip_load(1.0, 1.0, 200.0e9, 0.0, 0.1).is_err());
        assert!(cantilever_tip_load(1.0, f64::INFINITY, 200.0e9, 1.0, 0.1).is_err());
        assert!(simply_supported_uniform_load(1.0, 0.0, 200.0e9, 1.0, 0.1).is_err());
        assert!(simply_supported_uniform_load(f64::NAN, 4.0, 200.0e9, 1.0, 0.1).is_err());
    }

    #[test]
    fn cantilever_force_sign_changes_deflection_not_stress_magnitude() {
        let positive =
            cantilever_tip_load(100.0, 2.0, 200.0e9, 8.0e-6, 0.1).expect("valid positive load");
        let negative =
            cantilever_tip_load(-100.0, 2.0, 200.0e9, 8.0e-6, 0.1).expect("valid negative load");
        assert!(
            (positive.tip_deflection_m + negative.tip_deflection_m).abs() <= ABSOLUTE_TOLERANCE
        );
        assert!(
            (positive.maximum_bending_stress_pa - negative.maximum_bending_stress_pa).abs()
                <= 1.0e-9
        );
    }
}

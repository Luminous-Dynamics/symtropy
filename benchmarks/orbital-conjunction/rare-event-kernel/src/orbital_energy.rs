// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! SI-unit orbital energy along a peak-overlap center-of-linearization path.
//!
//! NASA CARA AdjustPOPCoL moves the current expansion center rs toward the
//! Gaussian peak-overlap position mup. At fraction f it evaluates
//!
//!   r(f) = rs + f (mup - rs)
//!   v(f) = vu + beta (r(f) - ru)
//!   E(f) = |v(f)|^2 / 2 - GM / |r(f)|
//!
//! where ru/vu are the current conditional state and beta = B A^-1.
//! This module isolates the energy path from the bounded scalar optimizer and
//! surrounding PeakOverlapPos iteration.
//!
//! All lengths are metres, velocities metres/second, time seconds and GM in
//! m^3/s^2. The caller must supply a unit-consistent GM explicitly; there is
//! no implicit kilometre/metre fallback.

use crate::KernelError;

/// Current and target center-of-linearization states for the POP energy path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalEnergyPathInput {
    /// Current expansion-center Cartesian position rs, metres.
    pub current_center_position_m: [f64; 3],
    /// Target fixed-Gaussian peak-overlap position mup, metres.
    pub target_mode_position_m: [f64; 3],
    /// Current conditional/reference Cartesian position ru, metres.
    pub reference_position_m: [f64; 3],
    /// Current conditional/reference velocity vu, metres/second.
    pub reference_velocity_mps: [f64; 3],
    /// Conditional velocity gain beta = B A^-1, inverse seconds.
    pub conditional_velocity_gain_per_second: [[f64; 3]; 3],
    /// Gravitational parameter, metres cubed per second squared.
    pub gravitational_parameter_m3_s2: f64,
}

/// Specific orbital energy at one point along the CoL segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalEnergyPoint {
    /// Interpolation fraction along current-to-target center segment.
    pub fraction: f64,
    /// Interpolated Cartesian position, metres.
    pub position_m: [f64; 3],
    /// Conditional velocity at this position, metres/second.
    pub conditional_velocity_mps: [f64; 3],
    /// Position radius from the central body, metres.
    pub radius_m: f64,
    /// Squared conditional speed, square metres per second squared.
    pub speed_squared_m2_s2: f64,
    /// Specific two-body orbital energy, square metres per second squared.
    pub specific_energy_m2_s2: f64,
}

/// Start/target energy evidence for one bounded CoL path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalEnergyPathReport {
    /// Energy at the current expansion centre (fraction zero).
    pub current: OrbitalEnergyPoint,
    /// Energy at the requested peak-overlap position (fraction one).
    pub target: OrbitalEnergyPoint,
    /// Whether the current reference orbit is bound (specific energy < 0).
    pub current_orbit_is_bound: bool,
    /// Relative change abs(E_target - E_current) / abs(E_current).
    /// None when the current reference orbit is nonnegative-energy.
    pub target_relative_energy_change: Option<f64>,
}

/// Evaluate specific two-body orbital energy in explicitly SI units.
///
/// Returns |v|^2/2 - mu/|r|. Rejects zero radius, nonpositive GM and
/// non-finite input rather than silently repairing the state.
pub fn specific_orbital_energy_m2_s2(
    position_m: [f64; 3],
    velocity_mps: [f64; 3],
    gravitational_parameter_m3_s2: f64,
) -> Result<f64, KernelError> {
    validate_vector(position_m)?;
    validate_vector(velocity_mps)?;
    validate_gravitational_parameter(gravitational_parameter_m3_s2)?;

    let radius_squared = dot(position_m, position_m)?;
    if radius_squared <= 0.0 {
        return Err(KernelError::InvalidOrbitalEnergyState);
    }
    let radius = radius_squared.sqrt();
    let speed_squared = dot(velocity_mps, velocity_mps)?;
    finite(0.5 * speed_squared - gravitational_parameter_m3_s2 / radius)
}

/// Evaluate NASA's conditional-velocity / linear-position path at fraction f.
///
/// The fraction is restricted to [0, 1]. A non-finite or zero-radius point
/// fails closed rather than being silently projected or repaired.
pub fn evaluate_orbital_energy_path_point(
    input: &OrbitalEnergyPathInput,
    fraction: f64,
) -> Result<OrbitalEnergyPoint, KernelError> {
    validate_path_input(input)?;
    if !fraction.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    if !(0.0..=1.0).contains(&fraction) {
        return Err(KernelError::InvalidOrbitalEnergyParameter);
    }

    let displacement = subtract(input.target_mode_position_m, input.current_center_position_m);
    let position = add_scaled(input.current_center_position_m, displacement, fraction)?;
    let reference_offset = subtract(position, input.reference_position_m);
    let conditional_offset =
        matrix_vector(input.conditional_velocity_gain_per_second, reference_offset)?;
    let conditional_velocity = add(input.reference_velocity_mps, conditional_offset)?;

    let radius_squared = dot(position, position)?;
    if radius_squared <= 0.0 {
        return Err(KernelError::InvalidOrbitalEnergyState);
    }
    let radius = radius_squared.sqrt();
    let speed_squared = dot(conditional_velocity, conditional_velocity)?;
    let specific_energy =
        finite(0.5 * speed_squared - input.gravitational_parameter_m3_s2 / radius)?;

    Ok(OrbitalEnergyPoint {
        fraction,
        position_m: position,
        conditional_velocity_mps: conditional_velocity,
        radius_m: radius,
        speed_squared_m2_s2: speed_squared,
        specific_energy_m2_s2: specific_energy,
    })
}

/// Evaluate current and target orbital energies for a CoL adjustment.
///
/// For a bound current orbit, reports abs(E_target - E_current) / abs(E_current).
/// A non-bound current orbit is reported explicitly with no relative change.
pub fn evaluate_orbital_energy_path(
    input: &OrbitalEnergyPathInput,
) -> Result<OrbitalEnergyPathReport, KernelError> {
    let current = evaluate_orbital_energy_path_point(input, 0.0)?;
    let target = evaluate_orbital_energy_path_point(input, 1.0)?;
    let current_orbit_is_bound = current.specific_energy_m2_s2 < 0.0;
    let target_relative_energy_change = if current_orbit_is_bound {
        let value = (target.specific_energy_m2_s2 - current.specific_energy_m2_s2).abs()
            / current.specific_energy_m2_s2.abs();
        Some(finite(value)?)
    } else {
        None
    };

    Ok(OrbitalEnergyPathReport {
        current,
        target,
        current_orbit_is_bound,
        target_relative_energy_change,
    })
}

fn validate_path_input(input: &OrbitalEnergyPathInput) -> Result<(), KernelError> {
    validate_vector(input.current_center_position_m)?;
    validate_vector(input.target_mode_position_m)?;
    validate_vector(input.reference_position_m)?;
    validate_vector(input.reference_velocity_mps)?;
    for row in input.conditional_velocity_gain_per_second {
        for value in row {
            if !value.is_finite() {
                return Err(KernelError::NonFiniteValue);
            }
        }
    }
    validate_gravitational_parameter(input.gravitational_parameter_m3_s2)
}

fn validate_vector(vector: [f64; 3]) -> Result<(), KernelError> {
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NonFiniteValue);
    }
    Ok(())
}

fn validate_gravitational_parameter(value: f64) -> Result<(), KernelError> {
    if !value.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    if value <= 0.0 {
        return Err(KernelError::InvalidOrbitalEnergyParameter);
    }
    Ok(())
}

fn matrix_vector(
    matrix: [[f64; 3]; 3],
    vector: [f64; 3],
) -> Result<[f64; 3], KernelError> {
    let result = [
        dot(matrix[0], vector)?,
        dot(matrix[1], vector)?,
        dot(matrix[2], vector)?,
    ];
    if result.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(result)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> Result<f64, KernelError> {
    finite(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
}

fn add_scaled(
    origin: [f64; 3],
    displacement: [f64; 3],
    fraction: f64,
) -> Result<[f64; 3], KernelError> {
    add(origin, scale(displacement, fraction)?)
}

fn add(left: [f64; 3], right: [f64; 3]) -> Result<[f64; 3], KernelError> {
    let result = [left[0] + right[0], left[1] + right[1], left[2] + right[2]];
    if result.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(result)
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> Result<[f64; 3], KernelError> {
    let result = [vector[0] * factor, vector[1] * factor, vector[2] * factor];
    if result.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(result)
}

fn finite(value: f64) -> Result<f64, KernelError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_input() -> OrbitalEnergyPathInput {
        OrbitalEnergyPathInput {
            current_center_position_m: [7_000_000.0, 0.0, 0.0],
            target_mode_position_m: [7_000_000.0, 100.0, 0.0],
            reference_position_m: [7_000_000.0, 0.0, 0.0],
            reference_velocity_mps: [0.0, 7_500.0, 0.0],
            conditional_velocity_gain_per_second: [
                [0.0, 0.0, 0.0],
                [0.0, 0.001, 0.0],
                [0.0, 0.0, 0.0],
            ],
            gravitational_parameter_m3_s2: 3.986_004_418e14,
        }
    }

    #[test]
    fn circular_two_body_orbit_has_expected_negative_specific_energy() {
        let radius = 7_000_000.0_f64;
        let mu = 3.986_004_418e14;
        let circular_speed = (mu / radius).sqrt();
        let energy = specific_orbital_energy_m2_s2(
            [radius, 0.0, 0.0],
            [0.0, circular_speed, 0.0],
            mu,
        )
        .expect("valid circular orbit");
        let expected = -mu / (2.0 * radius);
        assert!((energy - expected).abs() <= expected.abs() * 1.0e-14);
    }

    #[test]
    fn path_endpoints_match_current_and_requested_overlap_centers() {
        let input = path_input();
        let start = evaluate_orbital_energy_path_point(&input, 0.0).expect("start point");
        let target = evaluate_orbital_energy_path_point(&input, 1.0).expect("target point");
        assert_eq!(start.position_m, input.current_center_position_m);
        assert_eq!(target.position_m, input.target_mode_position_m);
    }

    #[test]
    fn conditional_velocity_uses_beta_times_position_offset() {
        let input = path_input();
        let point = evaluate_orbital_energy_path_point(&input, 1.0).expect("target point");
        assert_eq!(point.conditional_velocity_mps, [0.0, 7_500.1, 0.0]);
    }

    #[test]
    fn energy_change_is_reported_relative_to_bound_current_orbit() {
        let report = evaluate_orbital_energy_path(&path_input()).expect("valid energy path");
        assert!(report.current_orbit_is_bound);
        let expected = (report.target.specific_energy_m2_s2 - report.current.specific_energy_m2_s2)
            .abs()
            / report.current.specific_energy_m2_s2.abs();
        assert!((report.target_relative_energy_change.expect("relative change") - expected).abs() < 1.0e-15);
    }

    #[test]
    fn nonnegative_energy_reference_is_reported_as_unbound() {
        let mut input = path_input();
        input.reference_velocity_mps = [0.0, 12_000.0, 0.0];
        let report = evaluate_orbital_energy_path(&input).expect("finite unbound path");
        assert!(!report.current_orbit_is_bound);
        assert_eq!(report.target_relative_energy_change, None);
    }

    #[test]
    fn out_of_segment_fraction_is_rejected() {
        let input = path_input();
        assert_eq!(
            evaluate_orbital_energy_path_point(&input, 1.01),
            Err(KernelError::InvalidOrbitalEnergyParameter)
        );
    }

    #[test]
    fn zero_radius_and_invalid_gravitational_parameters_fail_closed() {
        assert_eq!(
            specific_orbital_energy_m2_s2([0.0; 3], [0.0; 3], 3.986_004_418e14),
            Err(KernelError::InvalidOrbitalEnergyState)
        );
        assert_eq!(
            specific_orbital_energy_m2_s2([7_000_000.0, 0.0, 0.0], [0.0; 3], 0.0),
            Err(KernelError::InvalidOrbitalEnergyParameter)
        );
    }

    #[test]
    fn non_finite_path_inputs_fail_closed() {
        let mut input = path_input();
        input.conditional_velocity_gain_per_second[0][0] = f64::NAN;
        assert_eq!(
            evaluate_orbital_energy_path(&input),
            Err(KernelError::NonFiniteValue)
        );
    }
}

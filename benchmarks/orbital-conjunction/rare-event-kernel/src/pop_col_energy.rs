// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conditional velocity and specific orbital energy at a PeakOverlapPos
//! center-of-linearity (CoL) candidate.
//!
//! Pinned NASA CARA helper: `PeakOverlapPos.m::CalcCoLEnergy` computes
//!
//! `v_col = v_u + beta * (r_col - r_u)`
//! `E_col = 0.5 * |v_col|^2 - GM / |r_col|`
//!
//! with `beta = B * A^-1`, positions in kilometres, velocities in km/s,
//! and `GM` in km^3/s^2. This module implements only that local numerical
//! primitive. It does not select the CoL adjustment fraction, propagate an
//! orbit, update an equinoctial state/Jacobian, or implement the full
//! PeakOverlapPos iteration.

use crate::KernelError;
use crate::conditional_velocity::Matrix3;

/// Inputs to the pinned CARA center-of-linearity energy calculation.
///
/// All vectors use the same inertial frame and epoch. Units are km, km/s,
/// seconds and km^3/s^2 as documented on the fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopColEnergyInput {
    /// Current expansion-center position (r_u), in kilometres.
    pub expansion_center_position_km: [f64; 3],
    /// Current expansion-center velocity (v_u), in kilometres per second.
    pub expansion_center_velocity_km_per_s: [f64; 3],
    /// Candidate CoL position (r_{col}), in kilometres.
    pub candidate_col_position_km: [f64; 3],
    /// Conditional velocity sensitivity (eta = B A^{-1}), in inverse seconds.
    pub beta_per_second: Matrix3,
    /// Central-body gravitational parameter, in km^3/s^2.
    pub gravitational_parameter_km3_per_s2: f64,
}

/// Auditable output of one CoL conditional-velocity / energy evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopColEnergyReport {
    /// Candidate CoL position, in kilometres.
    pub candidate_col_position_km: [f64; 3],
    /// Displacement (r_{col}-r_u), in kilometres.
    pub displacement_from_expansion_center_km: [f64; 3],
    /// Conditional velocity (v_u + \beta(r_{col}-r_u)), in kilometres per second.
    pub conditional_velocity_km_per_s: [f64; 3],
    /// Candidate radial distance (|r_{col}|), in kilometres.
    pub radius_km: f64,
    /// Specific kinetic energy (0.5 |v_{col}|^2), in km^2/s^2.
    pub kinetic_specific_energy_km2_per_s2: f64,
    /// Specific gravitational potential energy (-GM/|r_{col}|), in km^2/s^2.
    pub potential_specific_energy_km2_per_s2: f64,
    /// Total specific orbital energy, in km^2/s^2. Negative means bound.
    pub specific_orbital_energy_km2_per_s2: f64,
}

/// Evaluate the conditional velocity at a candidate CoL position.
///
/// This is the linear conditional-velocity relation used by CARA's
/// `CalcCoLEnergy` helper. The function is side-effect free and makes no
/// claim that the local linear model is physically valid outside its declared
/// expansion region.
pub fn conditional_velocity_at_col(
    expansion_center_position_km: [f64; 3],
    expansion_center_velocity_km_per_s: [f64; 3],
    candidate_col_position_km: [f64; 3],
    beta_per_second: Matrix3,
) -> Result<([f64; 3], [f64; 3]), KernelError> {
    if !finite_vector(expansion_center_position_km)
        || !finite_vector(expansion_center_velocity_km_per_s)
        || !finite_vector(candidate_col_position_km)
        || !finite_matrix(beta_per_second)
    {
        return Err(KernelError::NonFiniteValue);
    }

    let displacement = subtract(candidate_col_position_km, expansion_center_position_km);
    let sensitivity = matrix_vector(beta_per_second, displacement);
    let conditional_velocity = add(expansion_center_velocity_km_per_s, sensitivity);
    if !finite_vector(displacement) || !finite_vector(conditional_velocity) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((displacement, conditional_velocity))
}

/// Calculate the conditional specific orbital energy at a CoL candidate.
///
/// This follows the pinned CARA `CalcCoLEnergy` equations but uses stable
/// three-vector norms and explicitly rejects a zero-radius candidate. The
/// return report separates kinetic and potential terms so an auditor can
/// inspect the energy decomposition instead of receiving only a scalar.
pub fn calculate_pop_col_energy(
    input: PopColEnergyInput,
) -> Result<PopColEnergyReport, KernelError> {
    if !input.gravitational_parameter_km3_per_s2.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    if input.gravitational_parameter_km3_per_s2 <= 0.0 {
        return Err(KernelError::InvalidGravitationalParameter);
    }

    let (displacement, conditional_velocity) = conditional_velocity_at_col(
        input.expansion_center_position_km,
        input.expansion_center_velocity_km_per_s,
        input.candidate_col_position_km,
        input.beta_per_second,
    )?;

    let radius_km = vector_norm(input.candidate_col_position_km);
    if !radius_km.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if radius_km <= 0.0 {
        return Err(KernelError::InvalidRadius);
    }

    let speed_km_per_s = vector_norm(conditional_velocity);
    if !speed_km_per_s.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let kinetic_specific_energy_km2_per_s2 = 0.5 * speed_km_per_s * speed_km_per_s;
    let potential_specific_energy_km2_per_s2 =
        -input.gravitational_parameter_km3_per_s2 / radius_km;
    let specific_orbital_energy_km2_per_s2 =
        kinetic_specific_energy_km2_per_s2 + potential_specific_energy_km2_per_s2;

    if !kinetic_specific_energy_km2_per_s2.is_finite()
        || !potential_specific_energy_km2_per_s2.is_finite()
        || !specific_orbital_energy_km2_per_s2.is_finite()
    {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(PopColEnergyReport {
        candidate_col_position_km: input.candidate_col_position_km,
        displacement_from_expansion_center_km: displacement,
        conditional_velocity_km_per_s: conditional_velocity,
        radius_km,
        kinetic_specific_energy_km2_per_s2,
        potential_specific_energy_km2_per_s2,
        specific_orbital_energy_km2_per_s2,
    })
}

fn finite_vector(vector: [f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn finite_matrix(matrix: Matrix3) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn matrix_vector(matrix: Matrix3, vector: [f64; 3]) -> [f64; 3] {
    let mut output = [0.0; 3];
    for (row, value) in output.iter_mut().enumerate() {
        *value =
            matrix[row][0] * vector[0] + matrix[row][1] * vector[1] + matrix[row][2] * vector[2];
    }
    output
}

fn vector_norm(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARTH_GM_KM3_PER_S2: f64 = 3.986_004_418e5;

    fn zero_beta() -> Matrix3 {
        [[0.0; 3]; 3]
    }

    fn input(
        expansion_center_position_km: [f64; 3],
        expansion_center_velocity_km_per_s: [f64; 3],
        candidate_col_position_km: [f64; 3],
        beta_per_second: Matrix3,
        gravitational_parameter_km3_per_s2: f64,
    ) -> PopColEnergyInput {
        PopColEnergyInput {
            expansion_center_position_km,
            expansion_center_velocity_km_per_s,
            candidate_col_position_km,
            beta_per_second,
            gravitational_parameter_km3_per_s2,
        }
    }

    #[test]
    fn conditional_velocity_matches_pinned_linear_relation() {
        let beta = [[0.001, 0.0, 0.0], [0.0, 0.002, 0.0], [0.0, 0.0, 0.003]];
        let (displacement, velocity) = conditional_velocity_at_col(
            [7000.0, 0.0, 0.0],
            [0.0, 7.5, 0.0],
            [7010.0, 20.0, -30.0],
            beta,
        )
        .expect("finite conditional state");
        assert_eq!(displacement, [10.0, 20.0, -30.0]);
        assert_eq!(velocity, [0.01, 7.54, -0.09]);
    }

    #[test]
    fn zero_beta_preserves_expansion_center_velocity() {
        let (displacement, velocity) = conditional_velocity_at_col(
            [7000.0, 0.0, 0.0],
            [1.0, -2.0, 3.0],
            [7100.0, 20.0, -30.0],
            zero_beta(),
        )
        .expect("zero beta");
        assert_eq!(displacement, [100.0, 20.0, -30.0]);
        assert_eq!(velocity, [1.0, -2.0, 3.0]);
    }

    #[test]
    fn circular_orbit_energy_matches_vis_viva_known_answer() {
        let radius = 7000.0;
        let circular_speed = (EARTH_GM_KM3_PER_S2 / radius).sqrt();
        let report = calculate_pop_col_energy(input(
            [radius, 0.0, 0.0],
            [0.0, circular_speed, 0.0],
            [radius, 0.0, 0.0],
            zero_beta(),
            EARTH_GM_KM3_PER_S2,
        ))
        .expect("circular orbit");
        let expected_energy = -EARTH_GM_KM3_PER_S2 / (2.0 * radius);
        assert!((report.specific_orbital_energy_km2_per_s2 - expected_energy).abs() < 1.0e-12);
        assert!(report.specific_orbital_energy_km2_per_s2 < 0.0);
    }

    #[test]
    fn candidate_col_energy_uses_adjusted_conditional_velocity() {
        let report = calculate_pop_col_energy(input(
            [7000.0, 0.0, 0.0],
            [0.0, 7.5, 0.0],
            [7010.0, 20.0, -30.0],
            [[0.001, 0.0, 0.0], [0.0, 0.002, 0.0], [0.0, 0.0, 0.003]],
            EARTH_GM_KM3_PER_S2,
        ))
        .expect("adjusted CoL energy");
        assert_eq!(report.conditional_velocity_km_per_s, [0.01, 7.54, -0.09]);
        let speed = vector_norm(report.conditional_velocity_km_per_s);
        let expected_kinetic = 0.5 * speed * speed;
        let expected_potential =
            -EARTH_GM_KM3_PER_S2 / vector_norm(report.candidate_col_position_km);
        assert!((report.kinetic_specific_energy_km2_per_s2 - expected_kinetic).abs() < 1.0e-14);
        assert!((report.potential_specific_energy_km2_per_s2 - expected_potential).abs() < 1.0e-14);
        assert!(
            (report.specific_orbital_energy_km2_per_s2 - expected_kinetic - expected_potential)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn positive_specific_energy_is_preserved_for_unbound_candidate() {
        let report = calculate_pop_col_energy(input(
            [7000.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
            [7000.0, 0.0, 0.0],
            zero_beta(),
            EARTH_GM_KM3_PER_S2,
        ))
        .expect("finite unbound state");
        assert!(report.specific_orbital_energy_km2_per_s2 > 0.0);
    }

    #[test]
    fn zero_radius_is_rejected_without_division() {
        assert_eq!(
            calculate_pop_col_energy(input(
                [7000.0, 0.0, 0.0],
                [0.0, 7.5, 0.0],
                [0.0; 3],
                zero_beta(),
                EARTH_GM_KM3_PER_S2,
            )),
            Err(KernelError::InvalidRadius)
        );
    }

    #[test]
    fn nonpositive_gravitational_parameter_is_rejected() {
        assert_eq!(
            calculate_pop_col_energy(input(
                [7000.0, 0.0, 0.0],
                [0.0, 7.5, 0.0],
                [7000.0, 0.0, 0.0],
                zero_beta(),
                0.0,
            )),
            Err(KernelError::InvalidGravitationalParameter)
        );
    }

    #[test]
    fn nonfinite_kinematic_input_is_rejected() {
        assert_eq!(
            calculate_pop_col_energy(input(
                [7000.0, 0.0, 0.0],
                [0.0, 7.5, 0.0],
                [7000.0, 0.0, 0.0],
                [[f64::NAN, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
                EARTH_GM_KM3_PER_S2,
            )),
            Err(KernelError::NonFiniteValue)
        );
    }
}

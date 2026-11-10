//! Analytic low-Earth-orbit drag-law reference kernel.
//!
//! This crate intentionally does not model thermospheric density, spacecraft
//! attitude, atmospheric co-rotation, orbital propagation, radiation effects,
//! or conjunction probability. It evaluates only the declared drag equation
//! for a supplied local density and spacecraft-relative velocity.
//!
//! SI units are required throughout:
//! - density: kg/m^3
//! - area: m^2
//! - mass: kg
//! - velocity: m/s
//! - acceleration: m/s^2
//! - force: N
//! - power: W
//!
//! For ballistic coefficient beta = m / (Cd * A):
//! a_drag = -0.5 * rho / beta * |v_rel| * v_rel

use std::error::Error;
use std::fmt;

/// Inputs for one instantaneous drag evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragInput {
    pub density_kg_m3: f64,
    pub drag_coefficient: f64,
    pub reference_area_m2: f64,
    pub spacecraft_mass_kg: f64,
    pub relative_velocity_m_s: [f64; 3],
}

/// Vector and scalar results of the instantaneous drag evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragResult {
    pub ballistic_coefficient_kg_m2: f64,
    pub relative_speed_m_s: f64,
    pub acceleration_m_s2: [f64; 3],
    pub acceleration_magnitude_m_s2: f64,
    pub force_n: [f64; 3],
    /// Power dissipated relative to the local atmospheric frame. This is not
    /// inertial-frame spacecraft mechanical power when the atmosphere moves.
    pub relative_frame_dissipation_w: f64,
}

/// Closed set of input and numerical failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragError {
    NonFiniteInput,
    NonPositiveDensity,
    NonPositiveDragCoefficient,
    NonPositiveArea,
    NonPositiveMass,
    NonPositiveBallisticCoefficient,
    InvalidEnvelopeOrdering,
    NumericalOverflow,
}

impl fmt::Display for DragError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFiniteInput => "all drag inputs must be finite",
            Self::NonPositiveDensity => "density must be greater than zero",
            Self::NonPositiveDragCoefficient => "drag coefficient must be greater than zero",
            Self::NonPositiveArea => "reference area must be greater than zero",
            Self::NonPositiveMass => "spacecraft mass must be greater than zero",
            Self::NonPositiveBallisticCoefficient => "ballistic coefficient must be greater than zero",
            Self::InvalidEnvelopeOrdering => "envelope minima must not exceed maxima",
            Self::NumericalOverflow => "drag calculation produced a non-finite result",
        };
        formatter.write_str(message)
    }
}

impl Error for DragError {}

/// Bounds on local density and ballistic coefficient for a fixed velocity.
///
/// This produces an interval for drag acceleration magnitude only. It does
/// not account for correlation between density, attitude, area, mass or
/// velocity, and it is not a probability distribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragEnvelopeInput {
    pub density_min_kg_m3: f64,
    pub density_max_kg_m3: f64,
    pub ballistic_coefficient_min_kg_m2: f64,
    pub ballistic_coefficient_max_kg_m2: f64,
    pub relative_velocity_m_s: [f64; 3],
}

/// Conservative interval for the drag-acceleration magnitude.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragEnvelope {
    pub relative_speed_m_s: f64,
    pub min_acceleration_m_s2: f64,
    pub max_acceleration_m_s2: f64,
}

/// Evaluate the instantaneous aerodynamic drag law.
///
/// The force and acceleration point opposite to the spacecraft's velocity
/// relative to the atmosphere. The caller is responsible for supplying a
/// physically valid relative velocity, not merely inertial velocity.
pub fn evaluate_drag(input: DragInput) -> Result<DragResult, DragError> {
    let scalars = [
        input.density_kg_m3,
        input.drag_coefficient,
        input.reference_area_m2,
        input.spacecraft_mass_kg,
        input.relative_velocity_m_s[0],
        input.relative_velocity_m_s[1],
        input.relative_velocity_m_s[2],
    ];
    if scalars.iter().any(|value| !value.is_finite()) {
        return Err(DragError::NonFiniteInput);
    }
    if input.density_kg_m3 <= 0.0 {
        return Err(DragError::NonPositiveDensity);
    }
    if input.drag_coefficient <= 0.0 {
        return Err(DragError::NonPositiveDragCoefficient);
    }
    if input.reference_area_m2 <= 0.0 {
        return Err(DragError::NonPositiveArea);
    }
    if input.spacecraft_mass_kg <= 0.0 {
        return Err(DragError::NonPositiveMass);
    }

    let ballistic_coefficient =
        input.spacecraft_mass_kg / (input.drag_coefficient * input.reference_area_m2);
    let speed = norm(input.relative_velocity_m_s);
    let factor = -0.5 * input.density_kg_m3 / ballistic_coefficient * speed;
    let acceleration = scale(input.relative_velocity_m_s, factor);
    let acceleration_magnitude = norm(acceleration);
    let force = scale(acceleration, input.spacecraft_mass_kg);
    let dissipated_power = -dot(force, input.relative_velocity_m_s);

    let results = [
        ballistic_coefficient,
        speed,
        acceleration[0],
        acceleration[1],
        acceleration[2],
        acceleration_magnitude,
        force[0],
        force[1],
        force[2],
        dissipated_power,
    ];
    if results.iter().any(|value| !value.is_finite()) {
        return Err(DragError::NumericalOverflow);
    }

    Ok(DragResult {
        ballistic_coefficient_kg_m2: ballistic_coefficient,
        relative_speed_m_s: speed,
        acceleration_m_s2: acceleration,
        acceleration_magnitude_m_s2: acceleration_magnitude,
        force_n: force,
        relative_frame_dissipation_w: dissipated_power.max(0.0),
    })
}

/// Bound drag acceleration magnitude for independent density and ballistic
/// coefficient intervals at a fixed spacecraft-relative velocity.
///
/// Minimum acceleration uses minimum density and maximum ballistic
/// coefficient. Maximum acceleration uses maximum density and minimum
/// ballistic coefficient.
pub fn evaluate_drag_envelope(
    input: DragEnvelopeInput,
) -> Result<DragEnvelope, DragError> {
    let values = [
        input.density_min_kg_m3,
        input.density_max_kg_m3,
        input.ballistic_coefficient_min_kg_m2,
        input.ballistic_coefficient_max_kg_m2,
        input.relative_velocity_m_s[0],
        input.relative_velocity_m_s[1],
        input.relative_velocity_m_s[2],
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(DragError::NonFiniteInput);
    }
    if input.density_min_kg_m3 <= 0.0 {
        return Err(DragError::NonPositiveDensity);
    }
    if input.ballistic_coefficient_min_kg_m2 <= 0.0 {
        return Err(DragError::NonPositiveBallisticCoefficient);
    }
    if input.density_min_kg_m3 > input.density_max_kg_m3
        || input.ballistic_coefficient_min_kg_m2 > input.ballistic_coefficient_max_kg_m2
    {
        return Err(DragError::InvalidEnvelopeOrdering);
    }

    let speed = norm(input.relative_velocity_m_s);
    let speed_squared = speed * speed;
    let min_acceleration = 0.5
        * input.density_min_kg_m3
        * speed_squared
        / input.ballistic_coefficient_max_kg_m2;
    let max_acceleration = 0.5
        * input.density_max_kg_m3
        * speed_squared
        / input.ballistic_coefficient_min_kg_m2;

    if !speed.is_finite()
        || !min_acceleration.is_finite()
        || !max_acceleration.is_finite()
    {
        return Err(DragError::NumericalOverflow);
    }

    Ok(DragEnvelope {
        relative_speed_m_s: speed,
        min_acceleration_m_s2: min_acceleration,
        max_acceleration_m_s2: max_acceleration,
    })
}

fn norm(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

fn scale(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [
        vector[0] * scalar,
        vector[1] * scalar,
        vector[2] * scalar,
    ]
}

fn dot(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    lhs[0] * rhs[0] + lhs[1] * rhs[1] + lhs[2] * rhs[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN_DENSITY: f64 = 1.0e-12;
    const KNOWN_SPEED: f64 = 7_500.0;
    const KNOWN_CD: f64 = 2.2;
    const KNOWN_AREA: f64 = 10.0;
    const KNOWN_MASS: f64 = 100.0;

    fn known_input() -> DragInput {
        DragInput {
            density_kg_m3: KNOWN_DENSITY,
            drag_coefficient: KNOWN_CD,
            reference_area_m2: KNOWN_AREA,
            spacecraft_mass_kg: KNOWN_MASS,
            relative_velocity_m_s: [KNOWN_SPEED, 0.0, 0.0],
        }
    }

    fn close(actual: f64, expected: f64, relative_tolerance: f64) {
        let scale = expected.abs().max(1.0e-30);
        assert!(
            (actual - expected).abs() <= relative_tolerance * scale,
            "actual={actual:.16e}, expected={expected:.16e}"
        );
    }

    #[test]
    fn matches_hand_computable_drag_acceleration_and_force() {
        let result = evaluate_drag(known_input()).unwrap();
        let expected_acceleration = 6.1875e-6;
        let expected_force = 6.1875e-4;

        close(result.ballistic_coefficient_kg_m2, 100.0 / 22.0, 1.0e-14);
        close(result.relative_speed_m_s, KNOWN_SPEED, 1.0e-14);
        close(result.acceleration_magnitude_m_s2, expected_acceleration, 1.0e-14);
        close(result.acceleration_m_s2[0], -expected_acceleration, 1.0e-14);
        close(result.force_n[0], -expected_force, 1.0e-14);
    }

    #[test]
    fn acceleration_always_opposes_relative_atmospheric_velocity() {
        let mut input = known_input();
        input.relative_velocity_m_s = [6_000.0, 4_500.0, 0.0];
        let result = evaluate_drag(input).unwrap();

        assert!(dot(result.acceleration_m_s2, input.relative_velocity_m_s) < 0.0);
        close(result.relative_speed_m_s, KNOWN_SPEED, 1.0e-14);
        close(result.acceleration_magnitude_m_s2, 6.1875e-6, 1.0e-14);
    }

    #[test]
    fn relative_frame_dissipation_is_non_negative_and_zero_at_zero_relative_speed() {
        let moving = evaluate_drag(known_input()).unwrap();
        assert!(moving.relative_frame_dissipation_w > 0.0);

        let mut stationary = known_input();
        stationary.relative_velocity_m_s = [0.0; 3];
        let result = evaluate_drag(stationary).unwrap();
        assert_eq!(result.acceleration_m_s2, [0.0; 3]);
        assert_eq!(result.force_n, [0.0; 3]);
        assert_eq!(result.relative_frame_dissipation_w, 0.0);
    }

    #[test]
    fn equivalent_ballistic_coefficients_produce_equivalent_acceleration() {
        let first = evaluate_drag(known_input()).unwrap();

        let second = evaluate_drag(DragInput {
            density_kg_m3: KNOWN_DENSITY,
            drag_coefficient: 1.1,
            reference_area_m2: 20.0,
            spacecraft_mass_kg: KNOWN_MASS,
            relative_velocity_m_s: [KNOWN_SPEED, 0.0, 0.0],
        })
        .unwrap();

        close(
            first.acceleration_magnitude_m_s2,
            second.acceleration_magnitude_m_s2,
            1.0e-14,
        );
    }

    #[test]
    fn drag_acceleration_scales_linearly_with_density() {
        let baseline = evaluate_drag(known_input()).unwrap();
        let mut doubled_density = known_input();
        doubled_density.density_kg_m3 *= 2.0;
        let doubled = evaluate_drag(doubled_density).unwrap();

        close(
            doubled.acceleration_magnitude_m_s2,
            2.0 * baseline.acceleration_magnitude_m_s2,
            1.0e-14,
        );
    }

    #[test]
    fn drag_envelope_uses_conservative_density_and_ballistic_bounds() {
        let envelope = evaluate_drag_envelope(DragEnvelopeInput {
            density_min_kg_m3: 0.5e-12,
            density_max_kg_m3: 1.5e-12,
            ballistic_coefficient_min_kg_m2: 4.0,
            ballistic_coefficient_max_kg_m2: 5.0,
            relative_velocity_m_s: [KNOWN_SPEED, 0.0, 0.0],
        })
        .unwrap();

        close(
            envelope.min_acceleration_m_s2,
            0.5 * 0.5e-12 * KNOWN_SPEED.powi(2) / 5.0,
            1.0e-14,
        );
        close(
            envelope.max_acceleration_m_s2,
            0.5 * 1.5e-12 * KNOWN_SPEED.powi(2) / 4.0,
            1.0e-14,
        );
        assert!(envelope.min_acceleration_m_s2 <= envelope.max_acceleration_m_s2);
    }

    #[test]
    fn rejects_non_finite_and_non_physical_inputs() {
        let mut input = known_input();
        input.density_kg_m3 = f64::NAN;
        assert_eq!(evaluate_drag(input), Err(DragError::NonFiniteInput));

        let mut input = known_input();
        input.density_kg_m3 = 0.0;
        assert_eq!(evaluate_drag(input), Err(DragError::NonPositiveDensity));

        let mut input = known_input();
        input.spacecraft_mass_kg = -1.0;
        assert_eq!(evaluate_drag(input), Err(DragError::NonPositiveMass));
    }

    #[test]
    fn rejects_reversed_or_invalid_uncertainty_bounds() {
        let input = DragEnvelopeInput {
            density_min_kg_m3: 2.0e-12,
            density_max_kg_m3: 1.0e-12,
            ballistic_coefficient_min_kg_m2: 4.0,
            ballistic_coefficient_max_kg_m2: 5.0,
            relative_velocity_m_s: [KNOWN_SPEED, 0.0, 0.0],
        };
        assert_eq!(
            evaluate_drag_envelope(input),
            Err(DragError::InvalidEnvelopeOrdering)
        );
    }
}

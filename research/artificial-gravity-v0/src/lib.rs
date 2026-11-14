//! Reference kinematics for rotation-generated artificial gravity.
//!
//! This crate is an analytic reference model, not a structural solver,
//! human-rating tool, clinical prescription, or spacecraft flight model.
//! All vector accelerations use a right-handed Cartesian coordinate system.

use std::fmt;

/// Conventional standard gravity in metres per second squared.
pub const STANDARD_GRAVITY_M_S2: f64 = 9.806_65;
const RPM_TO_RAD_S: f64 = std::f64::consts::TAU / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    NonFiniteInput(&'static str),
    NonPositiveInput(&'static str),
    NegativeInput(&'static str),
    BodyCrossesRotationAxis,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput(name) => write!(f, "{name} must be finite"),
            Self::NonPositiveInput(name) => write!(f, "{name} must be greater than zero"),
            Self::NegativeInput(name) => write!(f, "{name} must be non-negative"),
            Self::BodyCrossesRotationAxis => {
                write!(f, "radial body extent may not cross the rotation axis")
            }
        }
    }
}

impl std::error::Error for ModelError {}

fn require_finite(value: f64, name: &'static str) -> Result<(), ModelError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NonFiniteInput(name))
    }
}

fn require_positive(value: f64, name: &'static str) -> Result<(), ModelError> {
    require_finite(value, name)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(ModelError::NonPositiveInput(name))
    }
}

fn require_non_negative(value: f64, name: &'static str) -> Result<(), ModelError> {
    require_finite(value, name)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(ModelError::NegativeInput(name))
    }
}

/// Convert rotations per minute to angular velocity in radians per second.
pub fn angular_velocity_rad_s(rpm: f64) -> Result<f64, ModelError> {
    require_non_negative(rpm, "rpm")?;
    Ok(rpm * RPM_TO_RAD_S)
}

/// Magnitude of rotation-generated radial acceleration at `radius_m`.
pub fn centrifugal_acceleration_m_s2(
    radius_m: f64,
    rpm: f64,
) -> Result<f64, ModelError> {
    require_non_negative(radius_m, "radius_m")?;
    let omega = angular_velocity_rad_s(rpm)?;
    Ok(omega * omega * radius_m)
}

/// Local apparent gravity as a multiple of standard Earth gravity.
pub fn gravity_level_g(radius_m: f64, rpm: f64) -> Result<f64, ModelError> {
    Ok(centrifugal_acceleration_m_s2(radius_m, rpm)? / STANDARD_GRAVITY_M_S2)
}

/// Radius required to produce `target_g` at `rpm`.
pub fn radius_for_gravity_m(target_g: f64, rpm: f64) -> Result<f64, ModelError> {
    require_positive(target_g, "target_g")?;
    require_positive(rpm, "rpm")?;
    let omega = angular_velocity_rad_s(rpm)?;
    Ok(target_g * STANDARD_GRAVITY_M_S2 / (omega * omega))
}

/// Rotation rate required to produce `target_g` at `radius_m`.
pub fn rpm_for_gravity(target_g: f64, radius_m: f64) -> Result<f64, ModelError> {
    require_positive(target_g, "target_g")?;
    require_positive(radius_m, "radius_m")?;
    let omega = (target_g * STANDARD_GRAVITY_M_S2 / radius_m).sqrt();
    Ok(omega / RPM_TO_RAD_S)
}

/// Gravity at the inboard and outboard ends of a radially aligned body.
///
/// The feet are assumed to be farther from the rotation axis than the head.
/// This model captures the ideal rigid-body gravity gradient only; it does
/// not model posture, local motion, vestibular response, or physiology.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadialGravityProfile {
    pub head_g: f64,
    pub centre_of_mass_g: f64,
    pub feet_g: f64,
    /// `(feet_g - head_g) / centre_of_mass_g`, expressed as a fraction.
    pub head_to_feet_gradient_fraction: f64,
}

pub fn radial_gravity_profile(
    centre_radius_m: f64,
    body_length_m: f64,
    rpm: f64,
) -> Result<RadialGravityProfile, ModelError> {
    require_positive(centre_radius_m, "centre_radius_m")?;
    require_positive(body_length_m, "body_length_m")?;
    require_positive(rpm, "rpm")?;

    let half_length = body_length_m / 2.0;
    if centre_radius_m < half_length {
        return Err(ModelError::BodyCrossesRotationAxis);
    }

    let head_radius = centre_radius_m - half_length;
    let feet_radius = centre_radius_m + half_length;
    let head_g = gravity_level_g(head_radius, rpm)?;
    let centre_of_mass_g = gravity_level_g(centre_radius_m, rpm)?;
    let feet_g = gravity_level_g(feet_radius, rpm)?;
    // For this ideal radial geometry, (g_feet - g_head) / g_com = L / r.
    let gradient = body_length_m / centre_radius_m;

    Ok(RadialGravityProfile {
        head_g,
        centre_of_mass_g,
        feet_g,
        head_to_feet_gradient_fraction: gradient,
    })
}

pub type Vec3 = [f64; 3];

/// Apparent acceleration in a uniformly translating, accelerating frame.
///
/// The local pseudo-acceleration is the negative of the frame acceleration.
/// This is the kinematic equivalence-principle baseline; it does not model
/// propulsion, propellant, trajectory, tidal curvature, or a global field.
pub fn translating_frame_apparent_acceleration(
    frame_acceleration_m_s2: Vec3,
) -> Result<Vec3, ModelError> {
    require_finite_vec3(frame_acceleration_m_s2, "frame_acceleration_m_s2")?;
    Ok([
        -frame_acceleration_m_s2[0],
        -frame_acceleration_m_s2[1],
        -frame_acceleration_m_s2[2],
    ])
}

fn require_finite_vec3(value: Vec3, name: &'static str) -> Result<(), ModelError> {
    for component in value {
        require_finite(component, name)?;
    }
    Ok(())
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Centrifugal apparent acceleration in a rotating frame: `-ω × (ω × r)`.
pub fn centrifugal_apparent_acceleration(
    omega_rad_s: Vec3,
    position_m: Vec3,
) -> Result<Vec3, ModelError> {
    require_finite_vec3(omega_rad_s, "omega_rad_s")?;
    require_finite_vec3(position_m, "position_m")?;
    let nested = cross(omega_rad_s, cross(omega_rad_s, position_m));
    Ok([-nested[0], -nested[1], -nested[2]])
}

/// Coriolis apparent acceleration in the rotating frame: `-2ω × v_relative`.
pub fn coriolis_apparent_acceleration(
    omega_rad_s: Vec3,
    velocity_relative_m_s: Vec3,
) -> Result<Vec3, ModelError> {
    require_finite_vec3(omega_rad_s, "omega_rad_s")?;
    require_finite_vec3(velocity_relative_m_s, "velocity_relative_m_s")?;
    let value = cross(omega_rad_s, velocity_relative_m_s);
    Ok([-2.0 * value[0], -2.0 * value[1], -2.0 * value[2]])
}

/// Euler apparent acceleration during angular acceleration: `-α × r`.
pub fn euler_apparent_acceleration(
    alpha_rad_s2: Vec3,
    position_m: Vec3,
) -> Result<Vec3, ModelError> {
    require_finite_vec3(alpha_rad_s2, "alpha_rad_s2")?;
    require_finite_vec3(position_m, "position_m")?;
    let value = cross(alpha_rad_s2, position_m);
    Ok([-value[0], -value[1], -value[2]])
}

/// Ideal rotational kinetic energy, ½ Iω², in joules.
///
/// This is stored spin-up energy; it is not continuous operating power.
pub fn rotational_kinetic_energy_j(
    moment_of_inertia_kg_m2: f64,
    rpm: f64,
) -> Result<f64, ModelError> {
    require_non_negative(moment_of_inertia_kg_m2, "moment_of_inertia_kg_m2")?;
    let omega = angular_velocity_rad_s(rpm)?;
    Ok(0.5 * moment_of_inertia_kg_m2 * omega * omega)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.12}, expected={expected:.12}, tolerance={tolerance}"
        );
    }

    #[test]
    fn reproduces_standard_habitat_design_points() {
        close(radius_for_gravity_m(1.0, 2.0).unwrap(), 223.5648, 0.01);
        close(radius_for_gravity_m(1.0, 4.0).unwrap(), 55.8912, 0.01);
        close(radius_for_gravity_m(0.38, 2.0).unwrap(), 84.9546, 0.01);
        close(radius_for_gravity_m(0.16, 2.0).unwrap(), 35.7704, 0.01);
    }

    #[test]
    fn forward_and_inverse_calculations_agree() {
        let radius = radius_for_gravity_m(0.38, 2.0).unwrap();
        close(gravity_level_g(radius, 2.0).unwrap(), 0.38, 1e-12);
        close(rpm_for_gravity(0.38, radius).unwrap(), 2.0, 1e-12);
    }

    #[test]
    fn radial_gradient_matches_body_length_over_radius() {
        let profile = radial_gravity_profile(100.0, 2.0, 2.0).unwrap();
        close(profile.head_to_feet_gradient_fraction, 0.02, 1e-12);
        close(
            profile.feet_g - profile.head_g,
            0.02 * profile.centre_of_mass_g,
            1e-12,
        );
    }

    #[test]
    fn vector_centrifugal_acceleration_points_outward() {
        let result = centrifugal_apparent_acceleration(
            [0.0, 0.0, 2.0],
            [3.0, 0.0, 0.0],
        )
        .unwrap();
        close(result[0], 12.0, 1e-12);
        close(result[1], 0.0, 1e-12);
        close(result[2], 0.0, 1e-12);
    }

    #[test]
    fn vector_coriolis_acceleration_uses_rotating_frame_sign() {
        let result = coriolis_apparent_acceleration(
            [0.0, 0.0, 2.0],
            [1.0, 0.0, 0.0],
        )
        .unwrap();
        close(result[0], 0.0, 1e-12);
        close(result[1], -4.0, 1e-12);
        close(result[2], 0.0, 1e-12);
    }

    #[test]
    fn translating_frame_apparent_acceleration_opposes_frame_acceleration() {
        let result = translating_frame_apparent_acceleration([
            0.0,
            0.0,
            STANDARD_GRAVITY_M_S2,
        ])
        .unwrap();
        close(result[0], 0.0, 1e-12);
        close(result[1], 0.0, 1e-12);
        close(result[2], -STANDARD_GRAVITY_M_S2, 1e-12);
    }

    #[test]
    fn euler_acceleration_is_zero_at_axis() {
        let result = euler_apparent_acceleration(
            [0.0, 0.0, 3.0],
            [0.0, 0.0, 0.0],
        )
        .unwrap();
        assert_eq!(result, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn spin_energy_is_stored_energy_not_power() {
        let expected = 0.5 * 10.0 * (std::f64::consts::TAU / 60.0).powi(2);
        close(rotational_kinetic_energy_j(10.0, 1.0).unwrap(), expected, 1e-12);
    }

    #[test]
    fn rejects_invalid_inputs() {
        assert!(radius_for_gravity_m(1.0, 0.0).is_err());
        assert!(gravity_level_g(-1.0, 2.0).is_err());
        assert!(angular_velocity_rad_s(f64::NAN).is_err());
        assert!(radial_gravity_profile(0.5, 2.0, 2.0).is_err());
        assert!(radial_gravity_profile(100.0, 2.0, 0.0).is_err());
    }
}

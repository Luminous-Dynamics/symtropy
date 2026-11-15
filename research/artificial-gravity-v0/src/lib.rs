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
    NonFiniteResult(&'static str),
    NonPositiveResult(&'static str),
    NonPositiveInput(&'static str),
    NegativeInput(&'static str),
    BodyCrossesRotationAxis,
    OuterRadiusLessThanInner,
    EmptyRotorAssembly,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput(name) => write!(f, "{name} must be finite"),
            Self::NonFiniteResult(name) => write!(f, "{name} result is not finite"),
            Self::NonPositiveResult(name) => write!(f, "{name} result is not positive"),
            Self::NonPositiveInput(name) => write!(f, "{name} must be greater than zero"),
            Self::NegativeInput(name) => write!(f, "{name} must be non-negative"),
            Self::BodyCrossesRotationAxis => {
                write!(f, "radial body extent may not cross the rotation axis")
            }
            Self::OuterRadiusLessThanInner => {
                write!(f, "outer_radius_m must be greater than or equal to inner_radius_m")
            }
            Self::EmptyRotorAssembly => write!(f, "rotor assembly must contain at least one module"),
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

fn require_finite_result(value: f64, name: &'static str) -> Result<(), ModelError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NonFiniteResult(name))
    }
}

fn require_positive_result(value: f64, name: &'static str) -> Result<(), ModelError> {
    require_finite_result(value, name)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(ModelError::NonPositiveResult(name))
    }
}

fn require_finite_result_vec3(value: Vec3, name: &'static str) -> Result<(), ModelError> {
    for component in value {
        require_finite_result(component, name)?;
    }
    Ok(())
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
    let omega = rpm * RPM_TO_RAD_S;
    require_finite_result(omega, "angular_velocity_rad_s")?;
    Ok(omega)
}

/// Magnitude of rotation-generated radial acceleration at `radius_m`.
pub fn centrifugal_acceleration_m_s2(
    radius_m: f64,
    rpm: f64,
) -> Result<f64, ModelError> {
    require_non_negative(radius_m, "radius_m")?;
    let omega = angular_velocity_rad_s(rpm)?;
    let acceleration = omega * omega * radius_m;
    require_finite_result(acceleration, "centrifugal_acceleration_m_s2")?;
    Ok(acceleration)
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
    let radius = target_g * STANDARD_GRAVITY_M_S2 / (omega * omega);
    require_positive_result(radius, "radius_for_gravity_m")?;
    Ok(radius)
}

/// Rotation rate required to produce `target_g` at `radius_m`.
pub fn rpm_for_gravity(target_g: f64, radius_m: f64) -> Result<f64, ModelError> {
    require_positive(target_g, "target_g")?;
    require_positive(radius_m, "radius_m")?;
    let omega = (target_g * STANDARD_GRAVITY_M_S2 / radius_m).sqrt();
    let rpm = omega / RPM_TO_RAD_S;
    require_positive_result(rpm, "rpm_for_gravity")?;
    Ok(rpm)
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


/// Radial acceleration envelope over an occupied annular region.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadialGravityEnvelope {
    pub inner_radius_m: f64,
    pub outer_radius_m: f64,
    pub inner_g: f64,
    pub outer_g: f64,
    pub spread_g: f64,
    /// (outer_g - inner_g) / midpoint_g.
    pub relative_spread_fraction: f64,
}

/// Evaluate ideal gravity variation over a radial habitat interval.
///
/// For fixed angular velocity, local rotation-generated acceleration is
/// linear in radius. This returns the endpoint range; it does not model
/// structural deformation or occupied-volume geometry away from the radial
/// line.
pub fn radial_gravity_envelope(
    inner_radius_m: f64,
    outer_radius_m: f64,
    rpm: f64,
) -> Result<RadialGravityEnvelope, ModelError> {
    require_positive(inner_radius_m, "inner_radius_m")?;
    require_positive(outer_radius_m, "outer_radius_m")?;
    require_positive(rpm, "rpm")?;
    if outer_radius_m < inner_radius_m {
        return Err(ModelError::OuterRadiusLessThanInner);
    }

    let inner_g = gravity_level_g(inner_radius_m, rpm)?;
    let outer_g = gravity_level_g(outer_radius_m, rpm)?;
    let spread_g = outer_g - inner_g;
    let midpoint_g = outer_g / 2.0 + inner_g / 2.0;
    let relative_spread_fraction = spread_g / midpoint_g;
    require_finite_result(spread_g, "radial_gravity_envelope.spread_g")?;
    require_finite_result(
        relative_spread_fraction,
        "radial_gravity_envelope.relative_spread_fraction",
    )?;

    Ok(RadialGravityEnvelope {
        inner_radius_m,
        outer_radius_m,
        inner_g,
        outer_g,
        spread_g,
        relative_spread_fraction,
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
    let apparent = [
        -frame_acceleration_m_s2[0],
        -frame_acceleration_m_s2[1],
        -frame_acceleration_m_s2[2],
    ];
    require_finite_result_vec3(apparent, "translating_frame_apparent_acceleration")?;
    Ok(apparent)
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
    let apparent = [-nested[0], -nested[1], -nested[2]];
    require_finite_result_vec3(apparent, "centrifugal_apparent_acceleration")?;
    Ok(apparent)
}

/// Coriolis apparent acceleration in the rotating frame: `-2ω × v_relative`.
pub fn coriolis_apparent_acceleration(
    omega_rad_s: Vec3,
    velocity_relative_m_s: Vec3,
) -> Result<Vec3, ModelError> {
    require_finite_vec3(omega_rad_s, "omega_rad_s")?;
    require_finite_vec3(velocity_relative_m_s, "velocity_relative_m_s")?;
    let value = cross(omega_rad_s, velocity_relative_m_s);
    let apparent = [-2.0 * value[0], -2.0 * value[1], -2.0 * value[2]];
    require_finite_result_vec3(apparent, "coriolis_apparent_acceleration")?;
    Ok(apparent)
}


/// Decomposed apparent acceleration at one point in a rotating reference frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotatingFrameAcceleration {
    pub centrifugal_m_s2: Vec3,
    pub coriolis_m_s2: Vec3,
    pub euler_m_s2: Vec3,
    pub motion_disturbance_m_s2: Vec3,
    pub apparent_total_m_s2: Vec3,
    pub motion_disturbance_magnitude_m_s2: f64,
    pub apparent_total_magnitude_m_s2: f64,
}

/// Combine centrifugal, Coriolis, and Euler apparent accelerations.
///
/// `position_m` and `velocity_relative_m_s` are measured in the rotating
/// frame. Angular velocity and angular acceleration are expressed in the
/// same right-handed frame. The result is local kinematics, not physiology.
pub fn rotating_frame_apparent_acceleration(
    omega_rad_s: Vec3,
    alpha_rad_s2: Vec3,
    position_m: Vec3,
    velocity_relative_m_s: Vec3,
) -> Result<RotatingFrameAcceleration, ModelError> {
    let centrifugal = centrifugal_apparent_acceleration(omega_rad_s, position_m)?;
    let coriolis = coriolis_apparent_acceleration(omega_rad_s, velocity_relative_m_s)?;
    let euler = euler_apparent_acceleration(alpha_rad_s2, position_m)?;
    let motion_disturbance = [
        coriolis[0] + euler[0],
        coriolis[1] + euler[1],
        coriolis[2] + euler[2],
    ];
    let apparent_total = [
        centrifugal[0] + motion_disturbance[0],
        centrifugal[1] + motion_disturbance[1],
        centrifugal[2] + motion_disturbance[2],
    ];
    require_finite_result_vec3(motion_disturbance, "motion_disturbance_m_s2")?;
    require_finite_result_vec3(apparent_total, "apparent_total_m_s2")?;
    let motion_disturbance_magnitude_m_s2 = vector_magnitude(motion_disturbance);
    let apparent_total_magnitude_m_s2 = vector_magnitude(apparent_total);
    require_finite_result(
        motion_disturbance_magnitude_m_s2,
        "motion_disturbance_magnitude_m_s2",
    )?;
    require_finite_result(
        apparent_total_magnitude_m_s2,
        "apparent_total_magnitude_m_s2",
    )?;

    Ok(RotatingFrameAcceleration {
        centrifugal_m_s2: centrifugal,
        coriolis_m_s2: coriolis,
        euler_m_s2: euler,
        motion_disturbance_m_s2: motion_disturbance,
        apparent_total_m_s2: apparent_total,
        motion_disturbance_magnitude_m_s2,
        apparent_total_magnitude_m_s2,
    })
}

/// Euclidean magnitude of a finite three-dimensional vector.
pub fn vector_magnitude(value: Vec3) -> f64 {
    value[0].hypot(value[1]).hypot(value[2])
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantAxisSpinState {
    pub time_s: f64,
    pub angle_rad: f64,
    pub angular_velocity_rad_s: f64,
    pub position_m: Vec3,
    pub velocity_m_s: Vec3,
    pub acceleration_m_s2: Vec3,
}

/// Exact inertial state of a point fixed in a body rotating about +z with
/// constant signed angular acceleration.
///
/// The initial position is expressed in the inertial frame at t=0.
/// This closed-form reference is useful for checking numerical spin-up and
/// spin-down integrations; it does not model a flexible or extended body.
pub fn constant_axis_spin_state(
    initial_position_m: Vec3,
    initial_omega_rad_s: f64,
    angular_acceleration_rad_s2: f64,
    time_s: f64,
) -> Result<ConstantAxisSpinState, ModelError> {
    require_finite_vec3(initial_position_m, "initial_position_m")?;
    require_finite(initial_omega_rad_s, "initial_omega_rad_s")?;
    require_finite(angular_acceleration_rad_s2, "angular_acceleration_rad_s2")?;
    require_non_negative(time_s, "time_s")?;

    let angle_rad =
        initial_omega_rad_s * time_s
            + 0.5 * angular_acceleration_rad_s2 * time_s * time_s;
    let angular_velocity_rad_s =
        initial_omega_rad_s + angular_acceleration_rad_s2 * time_s;
    require_finite_result(angle_rad, "spin.angle_rad")?;
    require_finite_result(angular_velocity_rad_s, "spin.angular_velocity_rad_s")?;

    let (sin_angle, cos_angle) = angle_rad.sin_cos();
    let position_m = [
        cos_angle * initial_position_m[0] - sin_angle * initial_position_m[1],
        sin_angle * initial_position_m[0] + cos_angle * initial_position_m[1],
        initial_position_m[2],
    ];
    let velocity_m_s = [
        -angular_velocity_rad_s * position_m[1],
        angular_velocity_rad_s * position_m[0],
        0.0,
    ];
    let omega_squared = angular_velocity_rad_s * angular_velocity_rad_s;
    let acceleration_m_s2 = [
        -angular_acceleration_rad_s2 * position_m[1] - omega_squared * position_m[0],
        angular_acceleration_rad_s2 * position_m[0] - omega_squared * position_m[1],
        0.0,
    ];

    require_finite_result_vec3(position_m, "spin.position_m")?;
    require_finite_result_vec3(velocity_m_s, "spin.velocity_m_s")?;
    require_finite_result_vec3(acceleration_m_s2, "spin.acceleration_m_s2")?;

    Ok(ConstantAxisSpinState {
        time_s,
        angle_rad,
        angular_velocity_rad_s,
        position_m,
        velocity_m_s,
        acceleration_m_s2,
    })
}

/// Euler apparent acceleration during angular acceleration: `-α × r`.
pub fn euler_apparent_acceleration(
    alpha_rad_s2: Vec3,
    position_m: Vec3,
) -> Result<Vec3, ModelError> {
    require_finite_vec3(alpha_rad_s2, "alpha_rad_s2")?;
    require_finite_vec3(position_m, "position_m")?;
    let value = cross(alpha_rad_s2, position_m);
    let apparent = [-value[0], -value[1], -value[2]];
    require_finite_result_vec3(apparent, "euler_apparent_acceleration")?;
    Ok(apparent)
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRadiusConstraint {
    BodyGradient,
    MaximumRotationRate,
    CoriolisMotionEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignRadiusEnvelope {
    pub gradient_min_radius_m: f64,
    pub rpm_min_radius_m: f64,
    pub coriolis_min_radius_m: f64,
    pub minimum_radius_m: f64,
    pub active_constraint: ActiveRadiusConstraint,
    pub resulting_rpm: f64,
    pub resulting_body_gradient_fraction: f64,
    pub resulting_coriolis_fraction: f64,
}

pub fn minimum_radius_for_body_gradient_m(
    body_length_m: f64,
    maximum_gradient_fraction: f64,
) -> Result<f64, ModelError> {
    require_positive(body_length_m, "body_length_m")?;
    require_positive(maximum_gradient_fraction, "maximum_gradient_fraction")?;
    let radius = body_length_m / maximum_gradient_fraction;
    require_positive_result(radius, "minimum_radius_for_body_gradient_m")?;
    Ok(radius)
}

/// This is a parameterized kinematic constraint, not a human tolerance threshold.
pub fn minimum_radius_for_coriolis_fraction_m(
    target_g: f64,
    maximum_coriolis_fraction: f64,
    transverse_speed_m_s: f64,
) -> Result<f64, ModelError> {
    require_positive(target_g, "target_g")?;
    require_positive(maximum_coriolis_fraction, "maximum_coriolis_fraction")?;
    require_non_negative(transverse_speed_m_s, "transverse_speed_m_s")?;
    if transverse_speed_m_s == 0.0 {
        return Ok(0.0);
    }

    let target_acceleration = target_g * STANDARD_GRAVITY_M_S2;
    require_positive_result(target_acceleration, "target_acceleration_m_s2")?;
    let numerator = 4.0 * transverse_speed_m_s * transverse_speed_m_s;
    let denominator = maximum_coriolis_fraction
        * maximum_coriolis_fraction
        * target_acceleration;
    let radius = numerator / denominator;
    require_positive_result(radius, "minimum_radius_for_coriolis_fraction_m")?;
    Ok(radius)
}

/// Intersect body-gradient, maximum-RPM and optional movement-disturbance bounds.
/// The Coriolis fraction is an input design assumption, not a medical limit.
pub fn minimum_radius_for_design_envelope(
    target_g: f64,
    body_length_m: f64,
    maximum_gradient_fraction: f64,
    maximum_rpm: f64,
    maximum_coriolis_fraction: f64,
    transverse_speed_m_s: f64,
) -> Result<DesignRadiusEnvelope, ModelError> {
    require_positive(target_g, "target_g")?;
    require_positive(maximum_rpm, "maximum_rpm")?;

    let gradient_min_radius_m =
        minimum_radius_for_body_gradient_m(body_length_m, maximum_gradient_fraction)?;
    let rpm_min_radius_m = radius_for_gravity_m(target_g, maximum_rpm)?;
    let coriolis_min_radius_m = minimum_radius_for_coriolis_fraction_m(
        target_g,
        maximum_coriolis_fraction,
        transverse_speed_m_s,
    )?;

    let (minimum_radius_m, active_constraint) =
        if gradient_min_radius_m >= rpm_min_radius_m
            && gradient_min_radius_m >= coriolis_min_radius_m
        {
            (gradient_min_radius_m, ActiveRadiusConstraint::BodyGradient)
        } else if rpm_min_radius_m >= coriolis_min_radius_m {
            (rpm_min_radius_m, ActiveRadiusConstraint::MaximumRotationRate)
        } else {
            (
                coriolis_min_radius_m,
                ActiveRadiusConstraint::CoriolisMotionEnvelope,
            )
        };

    let target_acceleration = target_g * STANDARD_GRAVITY_M_S2;
    require_positive_result(target_acceleration, "target_acceleration_m_s2")?;
    let omega = (target_acceleration / minimum_radius_m).sqrt();
    let resulting_rpm = omega / RPM_TO_RAD_S;
    let resulting_body_gradient_fraction = body_length_m / minimum_radius_m;
    let resulting_coriolis_fraction =
        (2.0 * omega * transverse_speed_m_s) / target_acceleration;

    require_positive_result(resulting_rpm, "design_envelope.resulting_rpm")?;
    require_finite_result(
        resulting_body_gradient_fraction,
        "design_envelope.resulting_body_gradient_fraction",
    )?;
    require_finite_result(
        resulting_coriolis_fraction,
        "design_envelope.resulting_coriolis_fraction",
    )?;

    Ok(DesignRadiusEnvelope {
        gradient_min_radius_m,
        rpm_min_radius_m,
        coriolis_min_radius_m,
        minimum_radius_m,
        active_constraint,
        resulting_rpm,
        resulting_body_gradient_fraction,
        resulting_coriolis_fraction,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingScalingMetrics {
    pub moving_mass_kg: f64,
    pub radius_m: f64,
    pub angular_velocity_rad_s: f64,
    pub tangential_speed_m_s: f64,
    pub angular_momentum_kg_m2_s: f64,
    pub rotational_energy_j: f64,
    pub ideal_hoop_tension_n: f64,
}

pub fn ring_scaling_for_fixed_mass(
    moving_mass_kg: f64,
    radius_m: f64,
    target_g: f64,
) -> Result<RingScalingMetrics, ModelError> {
    require_positive(moving_mass_kg, "moving_mass_kg")?;
    require_positive(radius_m, "radius_m")?;
    require_positive(target_g, "target_g")?;

    let acceleration = target_g * STANDARD_GRAVITY_M_S2;
    require_positive_result(acceleration, "target_acceleration_m_s2")?;
    let omega = (acceleration / radius_m).sqrt();
    let tangential_speed = (acceleration * radius_m).sqrt();
    let angular_momentum = moving_mass_kg * radius_m * tangential_speed;
    let rotational_energy = 0.5 * moving_mass_kg * tangential_speed * tangential_speed;
    let ideal_hoop_tension = moving_mass_kg * acceleration / std::f64::consts::TAU;

    require_positive_result(omega, "ring.angular_velocity_rad_s")?;
    require_positive_result(tangential_speed, "ring.tangential_speed_m_s")?;
    require_finite_result(angular_momentum, "ring.angular_momentum_kg_m2_s")?;
    require_finite_result(rotational_energy, "ring.rotational_energy_j")?;
    require_finite_result(ideal_hoop_tension, "ring.ideal_hoop_tension_n")?;

    Ok(RingScalingMetrics {
        moving_mass_kg,
        radius_m,
        angular_velocity_rad_s: omega,
        tangential_speed_m_s: tangential_speed,
        angular_momentum_kg_m2_s: angular_momentum,
        rotational_energy_j: rotational_energy,
        ideal_hoop_tension_n: ideal_hoop_tension,
    })
}

pub fn ring_scaling_for_fixed_line_density(
    line_mass_density_kg_m: f64,
    radius_m: f64,
    target_g: f64,
) -> Result<RingScalingMetrics, ModelError> {
    require_positive(line_mass_density_kg_m, "line_mass_density_kg_m")?;
    require_positive(radius_m, "radius_m")?;
    let moving_mass = line_mass_density_kg_m * std::f64::consts::TAU * radius_m;
    require_positive_result(moving_mass, "ring.moving_mass_kg")?;
    ring_scaling_for_fixed_mass(moving_mass, radius_m, target_g)
}

/// Scalar axial momentum and energy accounting for coaxial rotor modules.
///
/// This deliberately limited reference assumes every module rotates about the
/// same declared axis. It is not a 3D attitude, gyroscopic-stability, bearing,
/// structural-load, or control-system solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotorMomentumBalance {
    pub module_count: usize,
    /// Signed total axial angular momentum, kg·m²/s.
    pub net_angular_momentum_kg_m2_s: f64,
    /// Sum of the absolute per-module axial momenta, kg·m²/s.
    pub sum_absolute_module_momenta_kg_m2_s: f64,
    /// |net H| / sum(|H_i|); zero when all modules are stopped.
    pub net_momentum_fraction: f64,
    /// Sum of positive stored rotational kinetic energies, joules.
    pub stored_rotational_energy_j: f64,
}

/// Evaluate signed angular momentum and stored energy for coaxial modules.
///
/// Each tuple is (moment_of_inertia_kg_m2, signed_angular_velocity_rad_s).
/// Positive and negative angular velocities represent opposite spin directions.
pub fn rotor_momentum_balance(
    modules: &[(f64, f64)],
) -> Result<RotorMomentumBalance, ModelError> {
    if modules.is_empty() {
        return Err(ModelError::EmptyRotorAssembly);
    }

    let mut net_momentum = 0.0;
    let mut absolute_momentum = 0.0;
    let mut stored_energy = 0.0;

    for &(moment_of_inertia, omega) in modules {
        require_positive(moment_of_inertia, "rotor_moment_of_inertia_kg_m2")?;
        require_finite(omega, "rotor_angular_velocity_rad_s")?;

        let module_momentum = moment_of_inertia * omega;
        let module_energy = 0.5 * moment_of_inertia * omega * omega;
        require_finite_result(module_momentum, "module_angular_momentum")?;
        require_finite_result(module_energy, "module_rotational_energy")?;

        net_momentum += module_momentum;
        absolute_momentum += module_momentum.abs();
        stored_energy += module_energy;
        require_finite_result(net_momentum, "net_angular_momentum")?;
        require_finite_result(absolute_momentum, "sum_absolute_module_momenta")?;
        require_finite_result(stored_energy, "stored_rotational_energy")?;
    }

    let net_momentum_fraction = if absolute_momentum == 0.0 {
        0.0
    } else {
        net_momentum.abs() / absolute_momentum
    };
    require_finite_result(net_momentum_fraction, "net_momentum_fraction")?;

    Ok(RotorMomentumBalance {
        module_count: modules.len(),
        net_angular_momentum_kg_m2_s: net_momentum,
        sum_absolute_module_momenta_kg_m2_s: absolute_momentum,
        net_momentum_fraction,
        stored_rotational_energy_j: stored_energy,
    })
}

/// Signed angular velocity required for a second coaxial rotor to cancel the
/// first rotor's angular momentum exactly in the ideal scalar model.
pub fn angular_velocity_for_zero_net_momentum(
    primary_moment_of_inertia_kg_m2: f64,
    primary_omega_rad_s: f64,
    counter_rotor_moment_of_inertia_kg_m2: f64,
) -> Result<f64, ModelError> {
    require_positive(
        primary_moment_of_inertia_kg_m2,
        "primary_moment_of_inertia_kg_m2",
    )?;
    require_finite(primary_omega_rad_s, "primary_omega_rad_s")?;
    require_positive(
        counter_rotor_moment_of_inertia_kg_m2,
        "counter_rotor_moment_of_inertia_kg_m2",
    )?;

    let counter_omega = -primary_moment_of_inertia_kg_m2
        * primary_omega_rad_s
        / counter_rotor_moment_of_inertia_kg_m2;
    require_finite_result(counter_omega, "counter_rotor_omega_rad_s")?;
    Ok(counter_omega)
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
    let energy = 0.5 * moment_of_inertia_kg_m2 * omega * omega;
    require_finite_result(energy, "rotational_kinetic_energy_j")?;
    Ok(energy)
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
    fn radial_envelope_reports_exact_endpoint_gradient() {
        let envelope = radial_gravity_envelope(58.0, 350.0, 1.6).unwrap();
        close(envelope.spread_g, envelope.outer_g - envelope.inner_g, 1e-12);
        close(
            envelope.relative_spread_fraction,
            2.0 * (350.0 - 58.0) / (350.0 + 58.0),
            1e-12,
        );
        assert!(radial_gravity_envelope(350.0, 58.0, 1.6).is_err());
    }

    #[test]
    fn one_metre_per_second_coriolis_reference_points_match() {
        let omega = angular_velocity_rad_s(1.6).unwrap();
        let coriolis = coriolis_apparent_acceleration(
            [0.0, 0.0, omega],
            [1.0, 0.0, 0.0],
        )
        .unwrap();
        close(vector_magnitude(coriolis), 0.33510321638291124, 1e-12);
        close(
            vector_magnitude(coriolis) / STANDARD_GRAVITY_M_S2,
            0.03417101827667055,
            1e-12,
        );
    }

    #[test]
    fn combined_rotating_frame_terms_are_decomposed_and_composed() {
        let result = rotating_frame_apparent_acceleration(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 0.5],
            [3.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        )
        .unwrap();
        close(result.centrifugal_m_s2[0], 12.0, 1e-12);
        close(result.coriolis_m_s2[1], -4.0, 1e-12);
        close(result.euler_m_s2[1], -1.5, 1e-12);
        close(result.apparent_total_m_s2[0], 12.0, 1e-12);
        close(result.apparent_total_m_s2[1], -5.5, 1e-12);
        close(result.motion_disturbance_magnitude_m_s2, 5.5, 1e-12);
    }

    #[test]
    fn rejects_non_finite_composed_motion_result() {
        assert!(rotating_frame_apparent_acceleration(
            [0.0, 0.0, 1e308],
            [0.0; 3],
            [1e308, 0.0, 0.0],
            [1e308, 0.0, 0.0],
        )
        .is_err());
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
    fn constant_axis_spin_state_matches_spin_up_reference() {
        let state = constant_axis_spin_state([50.0, 3.0, 2.0], 0.1, 0.002, 5.0).unwrap();
        close(state.angle_rad, 0.525, 1e-12);
        close(state.angular_velocity_rad_s, 0.11, 1e-12);
        close(state.position_m[0], 41.762558067126, 1e-9);
        close(state.position_m[1], 27.656622058559, 1e-9);
        close(state.position_m[2], 2.0, 1e-12);
        close(state.velocity_m_s[0], -3.042228426441, 1e-9);
        close(state.velocity_m_s[1], 4.593881387384, 1e-9);
        close(state.acceleration_m_s2[0], -0.560640196729, 1e-9);
        close(state.acceleration_m_s2[1], -0.251120010774, 1e-9);
    }

    #[test]
    fn constant_axis_spin_state_cross_checks_rotating_frame_pseudo_acceleration() {
        let alpha = -0.01;
        let state = constant_axis_spin_state([20.0, 0.0, 1.0], 0.3, alpha, 20.0).unwrap();
        let apparent = rotating_frame_apparent_acceleration(
            [0.0, 0.0, state.angular_velocity_rad_s],
            [0.0, 0.0, alpha],
            state.position_m,
            [0.0, 0.0, 0.0],
        )
        .unwrap();

        for axis in 0..3 {
            close(
                state.acceleration_m_s2[axis] + apparent.apparent_total_m_s2[axis],
                0.0,
                1e-11,
            );
        }
        close(state.angle_rad, 4.0, 1e-12);
        close(state.angular_velocity_rad_s, 0.1, 1e-12);
    }

    #[test]
    fn constant_axis_spin_state_preserves_initial_state_at_zero_time() {
        let initial_position = [4.0, -3.0, 7.0];
        let state = constant_axis_spin_state(initial_position, -0.2, 0.03, 0.0).unwrap();
        assert_eq!(state.position_m, initial_position);
        close(state.angle_rad, 0.0, 1e-12);
        close(state.angular_velocity_rad_s, -0.2, 1e-12);
        close(state.velocity_m_s[0], -0.6, 1e-12);
        close(state.velocity_m_s[1], -0.8, 1e-12);
    }

    #[test]
    fn constant_axis_spin_state_rejects_invalid_and_overflowed_inputs() {
        assert!(constant_axis_spin_state([0.0; 3], 0.0, 0.0, -1.0).is_err());
        assert!(constant_axis_spin_state([f64::NAN, 0.0, 0.0], 0.0, 0.0, 1.0).is_err());
        assert!(constant_axis_spin_state([1.0, 0.0, 0.0], 1e308, 1e308, 1e308).is_err());
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
    fn reproduces_published_human_mechanics_constraint_intersection() {
        let result =
            minimum_radius_for_design_envelope(0.5, 1.75, 0.02, 2.0, 0.10, 1.0).unwrap();

        close(result.gradient_min_radius_m, 87.5, 1e-12);
        close(result.rpm_min_radius_m, 111.78240587618983, 1e-10);
        close(result.coriolis_min_radius_m, 81.57729703823425, 1e-10);
        close(result.minimum_radius_m, 111.78240587618983, 1e-10);
        assert_eq!(
            result.active_constraint,
            ActiveRadiusConstraint::MaximumRotationRate
        );
        close(result.resulting_rpm, 2.0, 1e-12);
        close(result.resulting_body_gradient_fraction, 0.01565541541428532, 1e-12);
        close(result.resulting_coriolis_fraction, 0.08542754569167638, 1e-12);
    }

    #[test]
    fn constraint_intersection_identifies_gradient_and_coriolis_limits() {
        let gradient_limited =
            minimum_radius_for_design_envelope(0.5, 2.0, 0.01, 4.0, 0.5, 0.0).unwrap();
        assert_eq!(
            gradient_limited.active_constraint,
            ActiveRadiusConstraint::BodyGradient
        );
        close(gradient_limited.minimum_radius_m, 200.0, 1e-12);

        let coriolis_limited =
            minimum_radius_for_design_envelope(0.5, 1.75, 0.05, 4.0, 0.01, 1.5).unwrap();
        assert_eq!(
            coriolis_limited.active_constraint,
            ActiveRadiusConstraint::CoriolisMotionEnvelope
        );
        close(coriolis_limited.resulting_coriolis_fraction, 0.01, 1e-12);
    }

    #[test]
    fn zero_transverse_speed_disables_coriolis_radius_bound() {
        assert_eq!(
            minimum_radius_for_coriolis_fraction_m(0.5, 0.05, 0.0).unwrap(),
            0.0
        );
    }

    #[test]
    fn fixed_mass_ring_obeys_ideal_radius_scaling() {
        let small = ring_scaling_for_fixed_mass(10_000.0, 100.0, 1.0).unwrap();
        let large = ring_scaling_for_fixed_mass(10_000.0, 200.0, 1.0).unwrap();
        close(
            large.angular_momentum_kg_m2_s / small.angular_momentum_kg_m2_s,
            2.0_f64.powf(1.5),
            1e-12,
        );
        close(large.rotational_energy_j / small.rotational_energy_j, 2.0, 1e-12);
        close(
            large.ideal_hoop_tension_n / small.ideal_hoop_tension_n,
            1.0,
            1e-12,
        );
    }

    #[test]
    fn fixed_line_density_ring_obeys_its_distinct_scaling_law() {
        let small = ring_scaling_for_fixed_line_density(100.0, 100.0, 1.0).unwrap();
        let large = ring_scaling_for_fixed_line_density(100.0, 200.0, 1.0).unwrap();
        close(large.moving_mass_kg / small.moving_mass_kg, 2.0, 1e-12);
        close(
            large.angular_momentum_kg_m2_s / small.angular_momentum_kg_m2_s,
            2.0_f64.powf(2.5),
            1e-12,
        );
        close(large.rotational_energy_j / small.rotational_energy_j, 4.0, 1e-12);
        close(large.ideal_hoop_tension_n / small.ideal_hoop_tension_n, 2.0, 1e-12);
    }

    #[test]
    fn constraint_and_ring_scaling_functions_reject_invalid_values() {
        assert!(minimum_radius_for_body_gradient_m(1.75, 0.0).is_err());
        assert!(minimum_radius_for_coriolis_fraction_m(0.5, 0.05, -1.0).is_err());
        assert!(minimum_radius_for_design_envelope(0.5, 1.75, 0.02, 0.0, 0.1, 1.0).is_err());
        assert!(ring_scaling_for_fixed_mass(-1.0, 100.0, 1.0).is_err());
        assert!(ring_scaling_for_fixed_line_density(100.0, f64::INFINITY, 1.0).is_err());
    }

    #[test]
    fn equal_counter_rotors_cancel_net_momentum_not_energy() {
        let balance = rotor_momentum_balance(&[(10.0, 2.0), (10.0, -2.0)]).unwrap();
        close(balance.net_angular_momentum_kg_m2_s, 0.0, 1e-12);
        close(balance.sum_absolute_module_momenta_kg_m2_s, 40.0, 1e-12);
        close(balance.net_momentum_fraction, 0.0, 1e-12);
        close(balance.stored_rotational_energy_j, 40.0, 1e-12);
    }

    #[test]
    fn unequal_counter_rotor_uses_inertia_weighted_speed() {
        let counter_omega =
            angular_velocity_for_zero_net_momentum(10.0, 2.0, 5.0).unwrap();
        close(counter_omega, -4.0, 1e-12);
        let balance = rotor_momentum_balance(&[(10.0, 2.0), (5.0, counter_omega)]).unwrap();
        close(balance.net_angular_momentum_kg_m2_s, 0.0, 1e-12);
        close(balance.stored_rotational_energy_j, 60.0, 1e-12);
    }

    #[test]
    fn stopped_rotors_have_zero_momentum_and_energy() {
        let balance = rotor_momentum_balance(&[(5.0, 0.0), (7.0, 0.0)]).unwrap();
        assert_eq!(balance.net_angular_momentum_kg_m2_s, 0.0);
        assert_eq!(balance.net_momentum_fraction, 0.0);
        assert_eq!(balance.stored_rotational_energy_j, 0.0);
    }

    #[test]
    fn rotor_balance_rejects_empty_and_non_finite_assemblies() {
        assert!(rotor_momentum_balance(&[]).is_err());
        assert!(rotor_momentum_balance(&[(1.0, f64::NAN)]).is_err());
        assert!(rotor_momentum_balance(&[(0.0, 1.0)]).is_err());
        assert!(rotor_momentum_balance(&[(1e308, 1e308)]).is_err());
    }

    #[test]
    fn spin_energy_is_stored_energy_not_power() {
        let expected = 0.5 * 10.0 * (std::f64::consts::TAU / 60.0).powi(2);
        close(rotational_kinetic_energy_j(10.0, 1.0).unwrap(), expected, 1e-12);
    }

    #[test]
    fn rejects_overflowed_or_underflowed_results() {
        assert!(centrifugal_acceleration_m_s2(f64::MAX, f64::MAX).is_err());
        assert!(radius_for_gravity_m(1e-300, 1e-300).is_err());
        assert!(coriolis_apparent_acceleration([0.0, 0.0, 1e308], [1e308, 0.0, 0.0]).is_err());
        assert!(rotational_kinetic_energy_j(1e308, 100.0).is_err());
        assert!(radius_for_gravity_m(1.0, f64::MAX).is_err());
        assert!(rpm_for_gravity(f64::MIN_POSITIVE, f64::MAX).is_err());
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

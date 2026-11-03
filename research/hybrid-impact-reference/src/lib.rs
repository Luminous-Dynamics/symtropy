//! Analytic first-impact reference for a one-dimensional bouncing point mass.
//!
//! This crate is a small verification oracle for PHYS-HYBRID-STRUCTURE-001.
//! It is not a time-stepper or a general contact solver.
//! Sign convention: position is positive away from the plane; gravity is
//! negative; an incoming impact velocity is negative.

/// Parameters for a point mass falling toward a fixed plane at position zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactParameters {
    /// Mass in kilograms.
    pub mass_kg: f64,
    /// Positive gravitational acceleration magnitude in metres per second squared.
    pub gravity_m_s2: f64,
    /// Initial height above the plane in metres.
    pub initial_height_m: f64,
    /// Initial vertical velocity in metres per second; positive is away from the plane.
    pub initial_velocity_m_s: f64,
    /// Newton coefficient of normal restitution, constrained to [0, 1].
    pub restitution: f64,
}

/// A validated analytic solution for the first impact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstImpact {
    /// Time from the initial condition to first contact, in seconds.
    pub impact_time_s: f64,
    /// Position at impact. The reference plane is at zero.
    pub impact_position_m: f64,
    /// Velocity immediately before impact; negative under the documented convention.
    pub pre_impact_velocity_m_s: f64,
    /// Velocity immediately after impact.
    pub post_impact_velocity_m_s: f64,
    /// Kinetic energy immediately before impact, in joules.
    pub pre_impact_kinetic_energy_j: f64,
    /// Kinetic energy immediately after impact, in joules.
    pub post_impact_kinetic_energy_j: f64,
    /// Physical kinetic-energy dissipation at impact, in joules.
    pub impact_dissipation_j: f64,
    /// Initial mechanical energy relative to zero potential at the plane, in joules.
    pub initial_mechanical_energy_j: f64,
    /// Initial energy minus post-impact kinetic energy and impact dissipation, in joules.
    pub energy_balance_residual_j: f64,
    /// Newton restitution residual: v_plus + e * v_minus, in metres per second.
    pub restitution_residual_m_s: f64,
}

/// Input or numerical failure while constructing the analytic reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactError {
    NonFiniteMass,
    NonPositiveMass,
    NonFiniteGravity,
    NonPositiveGravity,
    NonFiniteInitialHeight,
    NonPositiveInitialHeight,
    NonFiniteInitialVelocity,
    NonFiniteRestitution,
    RestitutionOutOfRange,
    NumericalOverflow,
    InvalidImpactNumber,
    NoFurtherImpactAfterRest,
    TimeResolutionExhausted,
}

pub mod bounce_sequence;
pub mod event_localization;
pub mod event_order;
pub mod frictional_contact;
pub mod projection_receipt;

fn finite(value: f64) -> bool {
    value.is_finite()
}

fn finite_or_overflow(value: f64) -> Result<f64, ImpactError> {
    if finite(value) {
        Ok(value)
    } else {
        Err(ImpactError::NumericalOverflow)
    }
}

/// Validate only the physical input domain for the first-impact model.
///
/// This function deliberately does not compute the closed-form impact time or
/// any derived trajectory result, so numerical event-location baselines can
/// share model validation without depending on the analytical oracle.
pub(crate) fn validate_impact_parameters(parameters: ImpactParameters) -> Result<(), ImpactError> {
    if !finite(parameters.mass_kg) {
        return Err(ImpactError::NonFiniteMass);
    }
    if parameters.mass_kg <= 0.0 {
        return Err(ImpactError::NonPositiveMass);
    }
    if !finite(parameters.gravity_m_s2) {
        return Err(ImpactError::NonFiniteGravity);
    }
    if parameters.gravity_m_s2 <= 0.0 {
        return Err(ImpactError::NonPositiveGravity);
    }
    if !finite(parameters.initial_height_m) {
        return Err(ImpactError::NonFiniteInitialHeight);
    }
    if parameters.initial_height_m <= 0.0 {
        return Err(ImpactError::NonPositiveInitialHeight);
    }
    if !finite(parameters.initial_velocity_m_s) {
        return Err(ImpactError::NonFiniteInitialVelocity);
    }
    if !finite(parameters.restitution) {
        return Err(ImpactError::NonFiniteRestitution);
    }
    if !(0.0..=1.0).contains(&parameters.restitution) {
        return Err(ImpactError::RestitutionOutOfRange);
    }
    Ok(())
}

/// Compute the exact-formula first-impact solution for constant gravity.
///
/// Free flight follows y(t) = h + v0*t - 0.5*g*t^2. The positive impact
/// time uses a cancellation-resistant expression when the initial velocity
/// points toward the plane. The incoming velocity is computed from the
/// analytic discriminant rather than by subtracting nearly equal values.
///
/// The energy ledger uses zero gravitational potential at the impact plane:
/// E_initial = 0.5*m*v0^2 + m*g*h.
/// D_impact = 0.5*m*(1-e^2)*v_minus^2.
///
/// The returned energy residual is E_initial - K_post - D_impact.
pub fn first_impact(parameters: ImpactParameters) -> Result<FirstImpact, ImpactError> {
    validate_impact_parameters(parameters)?;
    let ImpactParameters {
        mass_kg: mass,
        gravity_m_s2: gravity,
        initial_height_m: height,
        initial_velocity_m_s: initial_velocity,
        restitution,
    } = parameters;

    let velocity_squared = finite_or_overflow(initial_velocity * initial_velocity)?;
    let twice_gh = finite_or_overflow(2.0 * gravity * height)?;
    let discriminant = finite_or_overflow(velocity_squared + twice_gh)?;
    let speed_at_impact = finite_or_overflow(discriminant.sqrt())?;

    // Both expressions are algebraically identical. The second avoids
    // cancellation when initial_velocity is negative and speed is close to
    // -initial_velocity.
    let impact_time = if initial_velocity >= 0.0 {
        finite_or_overflow((initial_velocity + speed_at_impact) / gravity)?
    } else {
        finite_or_overflow((2.0 * height) / (speed_at_impact - initial_velocity))?
    };

    if impact_time <= 0.0 {
        return Err(ImpactError::NumericalOverflow);
    }

    let pre_velocity = -speed_at_impact;
    let post_velocity = finite_or_overflow(-restitution * pre_velocity)?;

    let pre_kinetic = finite_or_overflow(0.5 * mass * discriminant)?;
    let post_velocity_squared = finite_or_overflow(post_velocity * post_velocity)?;
    let post_kinetic = finite_or_overflow(0.5 * mass * post_velocity_squared)?;

    let potential_initial = finite_or_overflow(mass * gravity * height)?;
    let initial_kinetic = finite_or_overflow(0.5 * mass * velocity_squared)?;
    let initial_energy = finite_or_overflow(potential_initial + initial_kinetic)?;

    let restitution_squared = finite_or_overflow(restitution * restitution)?;
    let one_minus_e_squared = finite_or_overflow(1.0 - restitution_squared)?;
    let dissipation = finite_or_overflow(0.5 * mass * one_minus_e_squared * discriminant)?;

    let energy_residual = finite_or_overflow(initial_energy - (post_kinetic + dissipation))?;
    let restitution_residual = finite_or_overflow(post_velocity + restitution * pre_velocity)?;

    Ok(FirstImpact {
        impact_time_s: impact_time,
        impact_position_m: 0.0,
        pre_impact_velocity_m_s: pre_velocity,
        post_impact_velocity_m_s: post_velocity,
        pre_impact_kinetic_energy_j: pre_kinetic,
        post_impact_kinetic_energy_j: post_kinetic,
        impact_dissipation_j: dissipation,
        initial_mechanical_energy_j: initial_energy,
        energy_balance_residual_j: energy_residual,
        restitution_residual_m_s: restitution_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(restitution: f64) -> ImpactParameters {
        ImpactParameters {
            mass_kg: 3.0,
            gravity_m_s2: 2.0,
            initial_height_m: 1.0,
            initial_velocity_m_s: 0.0,
            restitution,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:?} differs from expected {expected:?} by more than {tolerance:?}"
        );
    }

    #[test]
    fn analytic_fixture_matches_hand_calculation() {
        let result = first_impact(fixture(0.5)).expect("valid fixture");
        close(result.impact_time_s, 1.0, 1.0e-14);
        close(result.impact_position_m, 0.0, 0.0);
        close(result.pre_impact_velocity_m_s, -2.0, 1.0e-14);
        close(result.post_impact_velocity_m_s, 1.0, 1.0e-14);
        close(result.pre_impact_kinetic_energy_j, 6.0, 1.0e-14);
        close(result.post_impact_kinetic_energy_j, 1.5, 1.0e-14);
        close(result.impact_dissipation_j, 4.5, 1.0e-14);
        close(result.initial_mechanical_energy_j, 6.0, 1.0e-14);
        close(result.energy_balance_residual_j, 0.0, 1.0e-14);
        close(result.restitution_residual_m_s, 0.0, 1.0e-14);
    }

    #[test]
    fn perfectly_inelastic_impact_has_zero_rebound_speed() {
        let result = first_impact(fixture(0.0)).expect("valid fixture");
        close(result.post_impact_velocity_m_s, 0.0, 0.0);
        close(result.impact_dissipation_j, 6.0, 1.0e-14);
        close(result.energy_balance_residual_j, 0.0, 1.0e-14);
    }

    #[test]
    fn perfectly_elastic_impact_has_no_impact_dissipation() {
        let result = first_impact(fixture(1.0)).expect("valid fixture");
        close(result.post_impact_velocity_m_s, 2.0, 1.0e-14);
        close(result.impact_dissipation_j, 0.0, 0.0);
        close(result.energy_balance_residual_j, 0.0, 1.0e-14);
    }

    #[test]
    fn upward_initial_velocity_uses_positive_quadratic_root() {
        let mut parameters = fixture(0.5);
        parameters.initial_velocity_m_s = 2.0;
        let result = first_impact(parameters).expect("valid fixture");
        let expected_time = (2.0 + 8.0_f64.sqrt()) / 2.0;
        close(result.impact_time_s, expected_time, 1.0e-14);
        close(result.pre_impact_velocity_m_s, -8.0_f64.sqrt(), 1.0e-14);
        close(result.energy_balance_residual_j, 0.0, 1.0e-13);
    }

    #[test]
    fn downward_initial_velocity_uses_stable_time_expression() {
        let mut parameters = fixture(0.25);
        parameters.gravity_m_s2 = 9.81;
        parameters.initial_velocity_m_s = -1.0e8;
        let result = first_impact(parameters).expect("valid fixture");
        assert!(result.impact_time_s > 0.0);
        assert!(result.impact_time_s < 1.0e-7);
        assert!(result.pre_impact_velocity_m_s < 0.0);
        let scale = result.initial_mechanical_energy_j.abs().max(1.0);
        let tolerance = 64.0 * f64::EPSILON * scale;
        assert!(result.energy_balance_residual_j.abs() <= tolerance);
    }

    #[test]
    fn rejects_non_positive_mass() {
        let mut parameters = fixture(0.5);
        parameters.mass_kg = 0.0;
        assert_eq!(first_impact(parameters), Err(ImpactError::NonPositiveMass));
    }

    #[test]
    fn rejects_non_positive_gravity() {
        let mut parameters = fixture(0.5);
        parameters.gravity_m_s2 = -1.0;
        assert_eq!(
            first_impact(parameters),
            Err(ImpactError::NonPositiveGravity)
        );
    }

    #[test]
    fn rejects_non_positive_height() {
        let mut parameters = fixture(0.5);
        parameters.initial_height_m = 0.0;
        assert_eq!(
            first_impact(parameters),
            Err(ImpactError::NonPositiveInitialHeight)
        );
    }

    #[test]
    fn rejects_restitution_outside_unit_interval() {
        for restitution in [-0.01, 1.01] {
            assert_eq!(
                first_impact(fixture(restitution)),
                Err(ImpactError::RestitutionOutOfRange)
            );
        }
    }

    #[test]
    fn rejects_non_finite_inputs() {
        let mut parameters = fixture(0.5);
        parameters.mass_kg = f64::NAN;
        assert_eq!(first_impact(parameters), Err(ImpactError::NonFiniteMass));

        parameters = fixture(0.5);
        parameters.initial_velocity_m_s = f64::INFINITY;
        assert_eq!(
            first_impact(parameters),
            Err(ImpactError::NonFiniteInitialVelocity)
        );
    }

    #[test]
    fn reports_derived_overflow_instead_of_non_finite_evidence() {
        let mut parameters = fixture(0.5);
        parameters.initial_velocity_m_s = f64::MAX;
        assert_eq!(
            first_impact(parameters),
            Err(ImpactError::NumericalOverflow)
        );
    }
}

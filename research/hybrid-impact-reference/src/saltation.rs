//! Branch-local saltation reference for a one-dimensional Newton impact.
//!
//! State convention: x = (y, v), positive y points away from the plane,
//! gravity has positive magnitude g, and incoming impacts have v_minus < 0.
//!
//! The reset Jacobian alone is not the saltation matrix. The latter also
//! includes the perturbation in the time at which the guard y = 0 is crossed.

/// Post-event continuous mode for the one-dimensional point-mass model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostImpactMode {
    /// The body rebounds and follows free flight under gravity.
    FreeFlight,
    /// A perfectly inelastic impact leaves the body supported at the plane.
    PersistentContact,
}

/// Typed failure while constructing a point-impact saltation reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaltationError {
    NonFiniteIncomingVelocity,
    NonIncomingVelocity,
    NonFiniteGravity,
    NonPositiveGravity,
    NonFiniteRestitution,
    RestitutionOutOfRange,
    InvalidTransversalityThreshold,
    GrazingOrBelowThreshold,
    NonFiniteDerivedValue,
}

/// Analytical event-map and first-order transport data for one impact branch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointImpactSaltation {
    pub pre_impact_velocity_m_s: f64,
    pub post_impact_velocity_m_s: f64,
    pub gravity_m_s2: f64,
    pub restitution: f64,
    /// Absolute incoming normal speed; the guard normal is (1, 0).
    pub transversality_speed_m_s: f64,
    /// User-declared minimum admissible incoming normal speed.
    pub minimum_transversality_speed_m_s: f64,
    /// d(event time)/d(initial height), in seconds per metre.
    pub event_time_derivative_wrt_height_s_per_m: f64,
    /// Jacobian of R(y, v) = (0, -e*v).
    pub reset_jacobian: [[f64; 2]; 2],
    /// Pre-event vector field (y_dot, v_dot).
    pub pre_event_vector_field: [f64; 2],
    /// Post-event vector field for the declared post-impact mode.
    pub post_event_vector_field: [f64; 2],
    pub post_impact_mode: PostImpactMode,
    /// Fixed-time first-order perturbation map across the event.
    pub saltation_matrix: [[f64; 2]; 2],
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

fn is_zero_restitution(value: f64) -> bool {
    value.to_bits() & 0x7fff_ffff_ffff_ffff == 0
}

fn finite_or_error(value: f64) -> Result<f64, SaltationError> {
    if finite(value) {
        Ok(value)
    } else {
        Err(SaltationError::NonFiniteDerivedValue)
    }
}

/// Construct the saltation map for one transversal point impact.
///
/// For 0 < e <= 1 the post-impact mode is free flight. For e = 0 the post-impact
/// mode is persistent contact, so its vector field is (0, 0), not the free-flight
/// field (0, -g). The distinction changes the saltation map.
///
/// The standard time-independent guard/reset formula is used:
///
/// S = D R + (f_plus - D R f_minus) * n^T / (n^T f_minus)
///
/// with guard y = 0 and n = (1, 0). A declared minimum incoming normal speed
/// excludes grazing and poorly conditioned events. The threshold is a caller
/// policy, not a silently selected numerical epsilon.
pub fn point_impact_saltation(
    pre_impact_velocity_m_s: f64,
    gravity_m_s2: f64,
    restitution: f64,
    minimum_transversality_speed_m_s: f64,
) -> Result<PointImpactSaltation, SaltationError> {
    if !finite(pre_impact_velocity_m_s) {
        return Err(SaltationError::NonFiniteIncomingVelocity);
    }
    if !finite(gravity_m_s2) {
        return Err(SaltationError::NonFiniteGravity);
    }
    if gravity_m_s2 <= 0.0 {
        return Err(SaltationError::NonPositiveGravity);
    }
    if !finite(restitution) {
        return Err(SaltationError::NonFiniteRestitution);
    }
    if !(0.0..=1.0).contains(&restitution) {
        return Err(SaltationError::RestitutionOutOfRange);
    }
    if !finite(minimum_transversality_speed_m_s) || minimum_transversality_speed_m_s < 0.0 {
        return Err(SaltationError::InvalidTransversalityThreshold);
    }

    let transversality_speed = pre_impact_velocity_m_s.abs();
    if transversality_speed <= minimum_transversality_speed_m_s {
        return Err(SaltationError::GrazingOrBelowThreshold);
    }
    if pre_impact_velocity_m_s >= 0.0 {
        return Err(SaltationError::NonIncomingVelocity);
    }

    let post_impact_velocity_m_s = finite_or_error(-restitution * pre_impact_velocity_m_s)?;
    let post_impact_mode = if is_zero_restitution(restitution) {
        PostImpactMode::PersistentContact
    } else {
        PostImpactMode::FreeFlight
    };

    let reset_jacobian = [[0.0, 0.0], [0.0, -restitution]];
    let pre_event_vector_field = [pre_impact_velocity_m_s, -gravity_m_s2];
    let post_event_vector_field = match post_impact_mode {
        PostImpactMode::FreeFlight => [post_impact_velocity_m_s, -gravity_m_s2],
        PostImpactMode::PersistentContact => [0.0, 0.0],
    };

    // D R f_minus = (0, e*g). The guard normal is (1, 0), so only
    // the first column receives the event-time correction.
    let reset_times_pre_field = [0.0, restitution * gravity_m_s2];
    let jump_correction_0 = finite_or_error(
        (post_event_vector_field[0] - reset_times_pre_field[0]) / pre_impact_velocity_m_s,
    )?;
    let jump_correction_1 = finite_or_error(
        (post_event_vector_field[1] - reset_times_pre_field[1]) / pre_impact_velocity_m_s,
    )?;

    let saltation_matrix = [[jump_correction_0, 0.0], [jump_correction_1, -restitution]];
    let event_time_derivative_wrt_height_s_per_m = finite_or_error(-1.0 / pre_impact_velocity_m_s)?;

    Ok(PointImpactSaltation {
        pre_impact_velocity_m_s,
        post_impact_velocity_m_s,
        gravity_m_s2,
        restitution,
        transversality_speed_m_s: transversality_speed,
        minimum_transversality_speed_m_s,
        event_time_derivative_wrt_height_s_per_m,
        reset_jacobian,
        pre_event_vector_field,
        post_event_vector_field,
        post_impact_mode,
        saltation_matrix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:?} differs from expected {expected:?} by more than {tolerance:?}"
        );
    }

    fn post_event_terminal_state(
        initial_height_m: f64,
        initial_velocity_m_s: f64,
        gravity_m_s2: f64,
        restitution: f64,
        terminal_time_s: f64,
    ) -> [f64; 2] {
        let speed = (initial_velocity_m_s * initial_velocity_m_s
            + 2.0 * gravity_m_s2 * initial_height_m)
            .sqrt();
        let event_time = if initial_height_m <= 0.0 {
            0.0
        } else if initial_velocity_m_s >= 0.0 {
            (initial_velocity_m_s + speed) / gravity_m_s2
        } else {
            2.0 * initial_height_m / (speed - initial_velocity_m_s)
        };
        let incoming_velocity = -speed;
        let outgoing_velocity = -restitution * incoming_velocity;
        let elapsed_after_event = terminal_time_s - event_time;
        [
            outgoing_velocity * elapsed_after_event
                - 0.5 * gravity_m_s2 * elapsed_after_event * elapsed_after_event,
            outgoing_velocity - gravity_m_s2 * elapsed_after_event,
        ]
    }

    #[test]
    fn intermediate_restitution_matches_analytic_saltation_matrix() {
        let map = point_impact_saltation(-2.0, 2.0, 0.5, 0.0).expect("transversal impact");
        assert_eq!(map.post_impact_mode, PostImpactMode::FreeFlight);
        close(map.post_impact_velocity_m_s, 1.0, 0.0);
        close(map.event_time_derivative_wrt_height_s_per_m, 0.5, 0.0);
        assert_eq!(map.reset_jacobian, [[0.0, 0.0], [0.0, -0.5]]);
        assert_eq!(map.saltation_matrix, [[-0.5, 0.0], [1.5, -0.5]]);
    }

    #[test]
    fn elastic_impact_has_expected_saltation_matrix() {
        let map = point_impact_saltation(-2.0, 2.0, 1.0, 0.0).expect("transversal impact");
        assert_eq!(map.saltation_matrix, [[-1.0, 0.0], [2.0, -1.0]]);
    }

    #[test]
    fn perfectly_inelastic_impact_uses_persistent_contact_vector_field() {
        let map = point_impact_saltation(-2.0, 2.0, 0.0, 0.0).expect("transversal impact");
        assert_eq!(map.post_impact_mode, PostImpactMode::PersistentContact);
        assert_eq!(map.post_event_vector_field, [0.0, 0.0]);
        assert_eq!(map.saltation_matrix, [[0.0, 0.0], [0.0, 0.0]]);
    }

    #[test]
    fn saltation_matches_one_sided_height_and_central_velocity_perturbations() {
        let map = point_impact_saltation(-2.0, 2.0, 0.5, 0.0).expect("transversal impact");
        let terminal_time = 0.25;
        let epsilon = 1.0e-5;
        let base = post_event_terminal_state(0.0, -2.0, 2.0, 0.5, terminal_time);

        // Positive height perturbation delays the guard crossing. The derivative
        // is one-sided because negative initial height is outside the domain.
        let height_perturbed = post_event_terminal_state(epsilon, -2.0, 2.0, 0.5, terminal_time);
        let height_column = [
            (height_perturbed[0] - base[0]) / epsilon,
            (height_perturbed[1] - base[1]) / epsilon,
        ];

        // A velocity perturbation retains the event at t=0, so central
        // differences independently check the reset derivative / second column.
        let velocity_plus = post_event_terminal_state(0.0, -2.0 + epsilon, 2.0, 0.5, terminal_time);
        let velocity_minus =
            post_event_terminal_state(0.0, -2.0 - epsilon, 2.0, 0.5, terminal_time);
        let velocity_column = [
            (velocity_plus[0] - velocity_minus[0]) / (2.0 * epsilon),
            (velocity_plus[1] - velocity_minus[1]) / (2.0 * epsilon),
        ];

        // Propagate the saltation map from the event to the terminal time:
        // Phi(T) = [[1, T], [0, 1]] for the constant-gravity variational flow.
        let expected_height_column = [
            map.saltation_matrix[0][0] + terminal_time * map.saltation_matrix[1][0],
            map.saltation_matrix[1][0],
        ];
        let expected_velocity_column = [
            map.saltation_matrix[0][1] + terminal_time * map.saltation_matrix[1][1],
            map.saltation_matrix[1][1],
        ];

        close(height_column[0], expected_height_column[0], 2.0e-4);
        close(height_column[1], expected_height_column[1], 2.0e-4);
        close(velocity_column[0], expected_velocity_column[0], 2.0e-7);
        close(velocity_column[1], expected_velocity_column[1], 2.0e-7);
    }

    #[test]
    fn rejects_grazing_and_near_grazing_events_by_declared_threshold() {
        assert_eq!(
            point_impact_saltation(-1.0e-8, 9.81, 0.5, 1.0e-6),
            Err(SaltationError::GrazingOrBelowThreshold)
        );
        assert_eq!(
            point_impact_saltation(0.0, 9.81, 0.5, 0.0),
            Err(SaltationError::GrazingOrBelowThreshold)
        );
    }

    #[test]
    fn rejects_outgoing_velocity_as_an_impact_input() {
        assert_eq!(
            point_impact_saltation(1.0, 9.81, 0.5, 0.0),
            Err(SaltationError::NonIncomingVelocity)
        );
    }

    #[test]
    fn rejects_invalid_model_parameters_and_thresholds() {
        assert_eq!(
            point_impact_saltation(-2.0, 0.0, 0.5, 0.0),
            Err(SaltationError::NonPositiveGravity)
        );
        assert_eq!(
            point_impact_saltation(-2.0, 9.81, 1.1, 0.0),
            Err(SaltationError::RestitutionOutOfRange)
        );
        assert_eq!(
            point_impact_saltation(-2.0, 9.81, 0.5, f64::NAN),
            Err(SaltationError::InvalidTransversalityThreshold)
        );
    }
    #[test]
    fn saltation_transforms_covariantly_under_length_and_time_unit_changes() {
        let base = point_impact_saltation(-2.0, 2.0, 0.5, 0.0).expect("base transversal impact");
        let length_scale = 100.0;
        let time_scale = 10.0;
        let velocity_scale = length_scale / time_scale;
        let transformed = point_impact_saltation(
            base.pre_impact_velocity_m_s * velocity_scale,
            base.gravity_m_s2 * length_scale / (time_scale * time_scale),
            base.restitution,
            0.0,
        )
        .expect("transformed transversal impact");

        let saltation_lower_left_scale = 1.0 / time_scale;
        let event_time_derivative_scale = time_scale / length_scale;
        close(
            transformed.saltation_matrix[0][0],
            base.saltation_matrix[0][0],
            1.0e-14,
        );
        close(
            transformed.saltation_matrix[0][1],
            base.saltation_matrix[0][1],
            1.0e-14,
        );
        close(
            transformed.saltation_matrix[1][0],
            saltation_lower_left_scale * base.saltation_matrix[1][0],
            1.0e-14,
        );
        close(
            transformed.saltation_matrix[1][1],
            base.saltation_matrix[1][1],
            1.0e-14,
        );
        close(
            transformed.event_time_derivative_wrt_height_s_per_m,
            event_time_derivative_scale * base.event_time_derivative_wrt_height_s_per_m,
            1.0e-14,
        );
    }
}

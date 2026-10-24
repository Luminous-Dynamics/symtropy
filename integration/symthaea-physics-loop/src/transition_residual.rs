//! Causal one-step dynamics-residual targets for hybrid flight research.
//!
//! This module derives a supervised target from consecutive state observations and
//! the command applied during that interval. It intentionally does not accept an
//! environment's hidden disturbance label: wind, payload effects, and model mismatch
//! must be inferred from observable transitions.
//!
//! These targets are not themselves a learned model, uncertainty estimate, controller,
//! stability proof, or safety certificate. The nominal dynamics parameters are explicit
//! so experiment receipts can record which model generated each target.

use symthaea_multirotor::{FlightState, QuadrotorCommand};

/// Parameters of the transparent nominal translational model.
///
/// linear_drag_n_s_per_m is the coefficient in F_drag = -c * velocity.
/// These parameters must be fixed before held-out evaluation, not tuned on test cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NominalLinearDynamics {
    pub mass_kg: f64,
    pub gravity_m_s2: f64,
    pub linear_drag_n_s_per_m: f64,
}

/// Error between measured linear acceleration and the nominal model's prediction.
///
/// The convention is world-frame acceleration and a target of
/// observed_acceleration minus nominal_acceleration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearTransitionResidual {
    pub dt_s: f64,
    pub observed_acceleration_world_m_s2: [f64; 3],
    pub nominal_acceleration_world_m_s2: [f64; 3],
    pub residual_acceleration_world_m_s2: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualTargetError {
    InvalidNominalParameters,
    NonFiniteState,
    InvalidQuaternion,
    NonFiniteCommand,
    NonIncreasingTimestamp,
    NonFiniteDerivedTarget,
}

impl LinearTransitionResidual {
    /// Derive one causal target using state at t, the action held over [t, t+dt),
    /// and state at t+dt. Timestamps are authoritative; dt is never caller supplied.
    ///
    /// The nominal model uses the clamped/applied thrust, starting attitude, gravity,
    /// and linear drag. Any integration, attitude-change, disturbance, actuator, or
    /// model discrepancy not represented by this simple model remains in the target.
    pub fn from_transition(
        before: &FlightState,
        requested_action: &QuadrotorCommand,
        after: &FlightState,
        nominal: NominalLinearDynamics,
    ) -> Result<Self, ResidualTargetError> {
        if !nominal.mass_kg.is_finite()
            || nominal.mass_kg <= 0.0
            || !nominal.gravity_m_s2.is_finite()
            || nominal.gravity_m_s2 < 0.0
            || !nominal.linear_drag_n_s_per_m.is_finite()
            || nominal.linear_drag_n_s_per_m < 0.0
        {
            return Err(ResidualTargetError::InvalidNominalParameters);
        }

        if !state_is_finite(before) || !state_is_finite(after) {
            return Err(ResidualTargetError::NonFiniteState);
        }
        if !quaternion_has_valid_norm(&before.quaternion)
            || !quaternion_has_valid_norm(&after.quaternion)
        {
            return Err(ResidualTargetError::InvalidQuaternion);
        }

        if !requested_action.to_ctrl().iter().all(|v| v.is_finite()) {
            return Err(ResidualTargetError::NonFiniteCommand);
        }

        let dt_s = after.timestamp - before.timestamp;
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(ResidualTargetError::NonIncreasingTimestamp);
        }

        let q = before.quaternion;
        let q_norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !q_norm.is_finite() || q_norm < 1.0e-9 {
            return Err(ResidualTargetError::InvalidQuaternion);
        }
        // FlightState convention is [w, x, y, z].
        let [w, x, y, z] = q.map(|v| v / q_norm);

        // R(q) * e_z, matching the quaternion-to-rotation convention used by
        // SymtropyFlightSimulator. Normalize q first to prevent scale artifacts.
        let body_z_world = [
            2.0 * (x * z + y * w),
            2.0 * (y * z - x * w),
            1.0 - 2.0 * (x * x + y * y),
        ];

        // The action argument is the clamped command sent to the plant interface.
        // Under the optional actuator-lag stress model, effective physical thrust can
        // differ from this command; that discrepancy intentionally remains in the
        // residual target instead of being leaked as a feature.
        let commanded_thrust_n = requested_action.clamped().thrust as f64;
        let drag_accel_scale = nominal.linear_drag_n_s_per_m / nominal.mass_kg;
        let thrust_accel_scale = commanded_thrust_n / nominal.mass_kg;

        let mut observed = [0.0; 3];
        let mut predicted = [0.0; 3];
        let mut residual = [0.0; 3];
        for axis in 0..3 {
            observed[axis] =
                (after.linear_velocity[axis] - before.linear_velocity[axis]) / dt_s;
            predicted[axis] = thrust_accel_scale * body_z_world[axis]
                - drag_accel_scale * before.linear_velocity[axis];
            if axis == 2 {
                predicted[axis] -= nominal.gravity_m_s2;
            }
            residual[axis] = observed[axis] - predicted[axis];
        }

        if !observed
            .iter()
            .chain(predicted.iter())
            .chain(residual.iter())
            .all(|v| v.is_finite())
        {
            return Err(ResidualTargetError::NonFiniteDerivedTarget);
        }

        Ok(Self {
            dt_s,
            observed_acceleration_world_m_s2: observed,
            nominal_acceleration_world_m_s2: predicted,
            residual_acceleration_world_m_s2: residual,
        })
    }
}

fn quaternion_has_valid_norm(q: &[f64; 4]) -> bool {
    let norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    norm.is_finite() && norm >= 1.0e-9
}

fn state_is_finite(state: &FlightState) -> bool {
    state.timestamp.is_finite()
        && state.position.iter().all(|v| v.is_finite())
        && state.quaternion.iter().all(|v| v.is_finite())
        && state.linear_velocity.iter().all(|v| v.is_finite())
        && state.angular_velocity.iter().all(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal() -> NominalLinearDynamics {
        NominalLinearDynamics {
            mass_kg: 0.027,
            gravity_m_s2: 9.81,
            linear_drag_n_s_per_m: 0.01,
        }
    }

    #[test]
    fn hover_transition_has_zero_residual_for_exact_nominal_model() {
        let before = FlightState::hover(0.1);
        let mut after = before.clone();
        after.timestamp = 0.01;
        let action = QuadrotorCommand::from_raw([0.027 * 9.81, 0.0, 0.0, 0.0]);

        let target =
            LinearTransitionResidual::from_transition(&before, &action, &after, nominal())
                .expect("finite increasing transition");

        assert_eq!(target.dt_s, 0.01);
        for value in target.residual_acceleration_world_m_s2 {
            assert!(value.abs() < 1.0e-5, "residual component={value}");
        }
    }

    #[test]
    fn residual_is_measured_acceleration_minus_nominal_acceleration() {
        let mut before = FlightState::hover(0.1);
        before.linear_velocity = [1.0, 0.0, 0.0];
        let mut after = before.clone();
        after.timestamp = 0.1;
        after.linear_velocity = [1.2, 0.0, 0.0];
        let action = QuadrotorCommand::from_raw([0.027 * 9.81, 0.0, 0.0, 0.0]);

        let target =
            LinearTransitionResidual::from_transition(&before, &action, &after, nominal())
                .expect("valid transition");

        assert!((target.observed_acceleration_world_m_s2[0] - 2.0).abs() < 1.0e-9);
        assert!((target.nominal_acceleration_world_m_s2[0] + 0.01 / 0.027).abs() < 1.0e-9);
        assert!(
            (target.residual_acceleration_world_m_s2[0] - (2.0 + 0.01 / 0.027)).abs()
                < 1.0e-9
        );
    }

    #[test]
    fn uses_applied_clamped_thrust_not_unbounded_request() {
        let before = FlightState::hover(0.1);
        let mut after = before.clone();
        after.timestamp = 0.01;
        let action = QuadrotorCommand::from_raw([1.0e6, 0.0, 0.0, 0.0]);

        let target =
            LinearTransitionResidual::from_transition(&before, &action, &after, nominal())
                .expect("finite request is clamped by the actuator interface");
        let expected = QuadrotorCommand::MAX_THRUST as f64 / nominal().mass_kg
            - nominal().gravity_m_s2;

        assert!((target.nominal_acceleration_world_m_s2[2] - expected).abs() < 1.0e-6);
    }

    #[test]
    fn rejects_non_monotonic_time_and_non_finite_commands() {
        let before = FlightState::hover(0.1);
        let mut after = before.clone();
        after.timestamp = before.timestamp;
        assert_eq!(
            LinearTransitionResidual::from_transition(
                &before,
                &QuadrotorCommand::hover(),
                &after,
                nominal()
            ),
            Err(ResidualTargetError::NonIncreasingTimestamp)
        );

        after.timestamp = 0.01;
        let malformed = QuadrotorCommand::from_raw([f32::NAN, 0.0, 0.0, 0.0]);
        assert_eq!(
            LinearTransitionResidual::from_transition(&before, &malformed, &after, nominal()),
            Err(ResidualTargetError::NonFiniteCommand)
        );
    }

    #[test]
    fn rejects_invalid_model_parameters_and_invalid_quaternion() {
        let before = FlightState::hover(0.1);
        let mut after = before.clone();
        after.timestamp = 0.01;
        let invalid_model = NominalLinearDynamics {
            mass_kg: 0.0,
            ..nominal()
        };
        assert_eq!(
            LinearTransitionResidual::from_transition(
                &before,
                &QuadrotorCommand::hover(),
                &after,
                invalid_model
            ),
            Err(ResidualTargetError::InvalidNominalParameters)
        );

        let mut invalid_state = before.clone();
        invalid_state.quaternion = [0.0; 4];
        assert_eq!(
            LinearTransitionResidual::from_transition(
                &invalid_state,
                &QuadrotorCommand::hover(),
                &after,
                nominal()
            ),
            Err(ResidualTargetError::InvalidQuaternion)
        );

        let mut invalid_next = after.clone();
        invalid_next.quaternion = [0.0; 4];
        assert_eq!(
            LinearTransitionResidual::from_transition(
                &before,
                &QuadrotorCommand::hover(),
                &invalid_next,
                nominal()
            ),
            Err(ResidualTargetError::InvalidQuaternion)
        );
    }
}

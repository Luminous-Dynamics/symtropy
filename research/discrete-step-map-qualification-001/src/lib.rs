pub mod impact;

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent qualification oracle for a stiff one-dimensional mechanical
//! step map. This crate does not depend on Symtropy internals or a simulator.

use std::error::Error;
use std::fmt;

/// A one-dimensional linear mass-spring-damper system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearOscillator {
    mass_kg: f64,
    damping_ns_per_m: f64,
    stiffness_n_per_m: f64,
}

/// Position and velocity state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State1D {
    pub position_m: f64,
    pub velocity_m_s: f64,
}

/// The Jacobian of a one-step transition in [position, velocity] coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepJacobian {
    pub dq_dq: f64,
    pub dq_dv: f64,
    pub dv_dq: f64,
    pub dv_dv: f64,
}

/// Dimensionless benchmark parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DimensionlessParameters {
    /// Timestep times the undamped natural frequency.
    pub rho: f64,
    /// Viscous damping ratio.
    pub zeta: f64,
}

/// Errors from model construction, transition evaluation or oracle evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelError {
    InvalidMass,
    InvalidDamping,
    InvalidStiffness,
    InvalidTimestep,
    InvalidState,
    InvalidTime,
    NonFiniteResult,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMass => write!(f, "mass must be finite and strictly positive"),
            Self::InvalidDamping => write!(f, "damping must be finite and nonnegative"),
            Self::InvalidStiffness => write!(f, "stiffness must be finite and nonnegative"),
            Self::InvalidTimestep => write!(f, "timestep must be finite and strictly positive"),
            Self::InvalidState => write!(f, "state must contain only finite values"),
            Self::InvalidTime => write!(f, "time must be finite and nonnegative"),
            Self::NonFiniteResult => write!(f, "step map produced a non-finite result"),
        }
    }
}

impl Error for ModelError {}

impl LinearOscillator {
    /// Construct a validated SI-unit mass-spring-damper model.
    pub fn try_new(
        mass_kg: f64,
        damping_ns_per_m: f64,
        stiffness_n_per_m: f64,
    ) -> Result<Self, ModelError> {
        if !mass_kg.is_finite() || mass_kg <= 0.0 {
            return Err(ModelError::InvalidMass);
        }
        if !damping_ns_per_m.is_finite() || damping_ns_per_m < 0.0 {
            return Err(ModelError::InvalidDamping);
        }
        if !stiffness_n_per_m.is_finite() || stiffness_n_per_m < 0.0 {
            return Err(ModelError::InvalidStiffness);
        }
        Ok(Self {
            mass_kg,
            damping_ns_per_m,
            stiffness_n_per_m,
        })
    }

    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    pub fn damping_ns_per_m(&self) -> f64 {
        self.damping_ns_per_m
    }

    pub fn stiffness_n_per_m(&self) -> f64 {
        self.stiffness_n_per_m
    }

    /// Return rho=h*sqrt(k/m) and zeta=c/(2*sqrt(m*k)).
    pub fn dimensionless_parameters(&self, dt_s: f64) -> Result<DimensionlessParameters, ModelError> {
        validate_timestep(dt_s)?;
        if self.stiffness_n_per_m <= 0.0 {
            return Err(ModelError::InvalidStiffness);
        }
        let omega_n = (self.stiffness_n_per_m / self.mass_kg).sqrt();
        let zeta = self.damping_ns_per_m
            / (2.0 * (self.mass_kg * self.stiffness_n_per_m).sqrt());
        let result = DimensionlessParameters {
            rho: dt_s * omega_n,
            zeta,
        };
        if result.rho.is_finite() && result.zeta.is_finite() {
            Ok(result)
        } else {
            Err(ModelError::NonFiniteResult)
        }
    }

    /// Fused implicit discrete step-map update.
    ///
    /// For f(q,v)=-kq-cv, the effective scalar metric is:
    /// m_hat = m + h*c + h^2*k
    /// m_hat*a = -k*q - (c+h*k)*v
    /// v_next = v + h*a
    /// q_next = q + h*v_next
    ///
    /// The returned step acceleration is (v_next-v)/h, not the continuous
    /// instantaneous acceleration.
    pub fn discrete_step(&self, state: State1D, dt_s: f64) -> Result<State1D, ModelError> {
        validate_state(state)?;
        validate_timestep(dt_s)?;

        let h = dt_s;
        let m_hat = self.mass_kg
            + h * self.damping_ns_per_m
            + h * h * self.stiffness_n_per_m;
        if !m_hat.is_finite() || m_hat <= 0.0 {
            return Err(ModelError::NonFiniteResult);
        }

        let rhs = -self.stiffness_n_per_m * state.position_m
            - (self.damping_ns_per_m + h * self.stiffness_n_per_m) * state.velocity_m_s;
        let step_acceleration = rhs / m_hat;
        let velocity_next = state.velocity_m_s + h * step_acceleration;
        let position_next = state.position_m + h * velocity_next;
        let result = State1D {
            position_m: position_next,
            velocity_m_s: velocity_next,
        };
        validate_result(result)?;
        Ok(result)
    }

    /// Semi-implicit Euler baseline with current-state force evaluation.
    pub fn semi_implicit_euler_step(
        &self,
        state: State1D,
        dt_s: f64,
    ) -> Result<State1D, ModelError> {
        validate_state(state)?;
        validate_timestep(dt_s)?;
        let acceleration = (-self.stiffness_n_per_m * state.position_m
            - self.damping_ns_per_m * state.velocity_m_s)
            / self.mass_kg;
        let velocity_next = state.velocity_m_s + dt_s * acceleration;
        let position_next = state.position_m + dt_s * velocity_next;
        let result = State1D {
            position_m: position_next,
            velocity_m_s: velocity_next,
        };
        validate_result(result)?;
        Ok(result)
    }

    /// Exact continuous-time state for underdamped, critical and overdamped cases.
    pub fn exact_state(&self, initial: State1D, time_s: f64) -> Result<State1D, ModelError> {
        validate_state(initial)?;
        if !time_s.is_finite() || time_s < 0.0 {
            return Err(ModelError::InvalidTime);
        }
        if self.stiffness_n_per_m == 0.0 {
            return self.exact_zero_stiffness(initial, time_s);
        }

        let omega_n = (self.stiffness_n_per_m / self.mass_kg).sqrt();
        let gamma = self.damping_ns_per_m / (2.0 * self.mass_kg);
        let discriminant = omega_n * omega_n - gamma * gamma;
        let threshold = 64.0 * f64::EPSILON * omega_n * omega_n;

        let result = if discriminant > threshold {
            let omega_d = discriminant.sqrt();
            let decay = (-gamma * time_s).exp();
            let cosine = (omega_d * time_s).cos();
            let sine = (omega_d * time_s).sin();
            let b = (initial.velocity_m_s + gamma * initial.position_m) / omega_d;
            let position = decay * (initial.position_m * cosine + b * sine);
            let velocity = decay
                * ((-gamma * initial.position_m + b * omega_d) * cosine
                    + (-gamma * b - initial.position_m * omega_d) * sine);
            State1D {
                position_m: position,
                velocity_m_s: velocity,
            }
        } else if discriminant.abs() <= threshold {
            let decay = (-gamma * time_s).exp();
            let b = initial.velocity_m_s + gamma * initial.position_m;
            let position = decay * (initial.position_m + b * time_s);
            let velocity = decay * (b - gamma * (initial.position_m + b * time_s));
            State1D {
                position_m: position,
                velocity_m_s: velocity,
            }
        } else {
            let root = (-discriminant).sqrt();
            let r1 = -gamma + root;
            let r2 = -gamma - root;
            let a = (initial.velocity_m_s - r2 * initial.position_m) / (r1 - r2);
            let b = (r1 * initial.position_m - initial.velocity_m_s) / (r1 - r2);
            let e1 = (r1 * time_s).exp();
            let e2 = (r2 * time_s).exp();
            State1D {
                position_m: a * e1 + b * e2,
                velocity_m_s: r1 * a * e1 + r2 * b * e2,
            }
        };
        validate_result(result)?;
        Ok(result)
    }

    fn exact_zero_stiffness(
        &self,
        initial: State1D,
        time_s: f64,
    ) -> Result<State1D, ModelError> {
        if self.damping_ns_per_m == 0.0 {
            return Ok(State1D {
                position_m: initial.position_m + time_s * initial.velocity_m_s,
                velocity_m_s: initial.velocity_m_s,
            });
        }
        let rate = self.damping_ns_per_m / self.mass_kg;
        let decay = (-rate * time_s).exp();
        let velocity = initial.velocity_m_s * decay;
        let position = initial.position_m
            + initial.velocity_m_s * (1.0 - decay) / rate;
        let result = State1D {
            position_m: position,
            velocity_m_s: velocity,
        };
        validate_result(result)?;
        Ok(result)
    }

    /// Analytic Jacobian of the complete discrete transition.
    pub fn discrete_step_jacobian(&self, dt_s: f64) -> Result<StepJacobian, ModelError> {
        validate_timestep(dt_s)?;
        let h = dt_s;
        let m_hat = self.mass_kg
            + h * self.damping_ns_per_m
            + h * h * self.stiffness_n_per_m;
        let dv_dq = -h * self.stiffness_n_per_m / m_hat;
        let dv_dv = 1.0
            - h * (self.damping_ns_per_m + h * self.stiffness_n_per_m) / m_hat;
        let dq_dq = 1.0 + h * dv_dq;
        let dq_dv = h * dv_dv;
        let result = StepJacobian {
            dq_dq,
            dq_dv,
            dv_dq,
            dv_dv,
        };
        if [
            result.dq_dq,
            result.dq_dv,
            result.dv_dq,
            result.dv_dv,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            Ok(result)
        } else {
            Err(ModelError::NonFiniteResult)
        }
    }

    /// Mechanical energy in joules.
    pub fn energy_joules(&self, state: State1D) -> Result<f64, ModelError> {
        validate_state(state)?;
        let energy = 0.5 * self.mass_kg * state.velocity_m_s * state.velocity_m_s
            + 0.5 * self.stiffness_n_per_m * state.position_m * state.position_m;
        if energy.is_finite() && energy >= 0.0 {
            Ok(energy)
        } else {
            Err(ModelError::NonFiniteResult)
        }
    }

    /// Exact one-step energy change for the implicit discrete update.
    pub fn discrete_energy_change_identity(
        &self,
        before: State1D,
        after: State1D,
        dt_s: f64,
    ) -> Result<f64, ModelError> {
        validate_state(before)?;
        validate_state(after)?;
        validate_timestep(dt_s)?;
        let delta_v = after.velocity_m_s - before.velocity_m_s;
        let delta_q = after.position_m - before.position_m;
        let predicted = -0.5 * self.mass_kg * delta_v * delta_v
            - 0.5 * self.stiffness_n_per_m * delta_q * delta_q
            - dt_s * self.damping_ns_per_m * after.velocity_m_s * after.velocity_m_s;
        if predicted.is_finite() {
            Ok(predicted)
        } else {
            Err(ModelError::NonFiniteResult)
        }
    }
}

fn validate_state(state: State1D) -> Result<(), ModelError> {
    if state.position_m.is_finite() && state.velocity_m_s.is_finite() {
        Ok(())
    } else {
        Err(ModelError::InvalidState)
    }
}

fn validate_timestep(dt_s: f64) -> Result<(), ModelError> {
    if dt_s.is_finite() && dt_s > 0.0 {
        Ok(())
    } else {
        Err(ModelError::InvalidTimestep)
    }
}

fn validate_result(result: State1D) -> Result<(), ModelError> {
    if result.position_m.is_finite() && result.velocity_m_s.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.16e} ~= {b:.16e} within {tolerance:e}"
        );
    }

    fn fixture(damping: f64) -> LinearOscillator {
        LinearOscillator::try_new(1.0, damping, 1.0).unwrap()
    }

    #[test]
    fn discrete_step_jacobian_matches_central_difference_of_full_map() {
        let model = LinearOscillator::try_new(1.7, 0.23, 11.0).unwrap();
        let state = State1D {
            position_m: -0.42,
            velocity_m_s: 1.31,
        };
        let dt = 0.37;
        let jacobian = model.discrete_step_jacobian(dt).unwrap();
        let step = 1.0e-6;

        let plus_q = State1D {
            position_m: state.position_m + step,
            ..state
        };
        let minus_q = State1D {
            position_m: state.position_m - step,
            ..state
        };
        let plus_v = State1D {
            velocity_m_s: state.velocity_m_s + step,
            ..state
        };
        let minus_v = State1D {
            velocity_m_s: state.velocity_m_s - step,
            ..state
        };

        let q_plus = model.discrete_step(plus_q, dt).unwrap();
        let q_minus = model.discrete_step(minus_q, dt).unwrap();
        let v_plus = model.discrete_step(plus_v, dt).unwrap();
        let v_minus = model.discrete_step(minus_v, dt).unwrap();

        close(jacobian.dq_dq, (q_plus.position_m - q_minus.position_m) / (2.0 * step), 2.0e-9);
        close(jacobian.dv_dq, (q_plus.velocity_m_s - q_minus.velocity_m_s) / (2.0 * step), 2.0e-9);
        close(jacobian.dq_dv, (v_plus.position_m - v_minus.position_m) / (2.0 * step), 2.0e-9);
        close(jacobian.dv_dv, (v_plus.velocity_m_s - v_minus.velocity_m_s) / (2.0 * step), 2.0e-9);
    }

    #[test]
    fn discrete_step_is_unconditionally_bounded_for_stiff_undamped_fixture() {
        let model = fixture(0.0);
        let mut state = State1D {
            position_m: 1.0,
            velocity_m_s: 0.0,
        };
        let initial_energy = model.energy_joules(state).unwrap();
        let dt = 4.0; // rho = 4, outside semi-implicit Euler's stability interval.
        for _ in 0..256 {
            state = model.discrete_step(state, dt).unwrap();
            assert!(model.energy_joules(state).unwrap() <= initial_energy);
        }
        assert!(model.energy_joules(state).unwrap() < initial_energy * 1.0e-20);
    }

    #[test]
    fn semi_implicit_euler_is_unstable_beyond_its_harmonic_oscillator_limit() {
        let model = fixture(0.0);
        let mut state = State1D {
            position_m: 1.0,
            velocity_m_s: 0.0,
        };
        let initial_energy = model.energy_joules(state).unwrap();
        for _ in 0..32 {
            state = model.semi_implicit_euler_step(state, 4.0).unwrap();
        }
        assert!(model.energy_joules(state).unwrap() > initial_energy * 1.0e20);
    }

    #[test]
    fn discrete_energy_balance_identity_holds_with_and_without_damping() {
        for damping in [0.0, 0.1, 2.0, 20.0] {
            let model = LinearOscillator::try_new(2.0, damping, 7.0).unwrap();
            let before = State1D {
                position_m: 0.8,
                velocity_m_s: -1.4,
            };
            let dt = 0.73;
            let after = model.discrete_step(before, dt).unwrap();
            let measured = model.energy_joules(after).unwrap()
                - model.energy_joules(before).unwrap();
            let predicted = model
                .discrete_energy_change_identity(before, after, dt)
                .unwrap();
            close(measured, predicted, 5.0e-13);
            assert!(measured <= 1.0e-12);
        }
    }

    #[test]
    fn discrete_step_converges_to_independent_exact_trajectory() {
        let model = LinearOscillator::try_new(1.0, 0.15, 9.0).unwrap();
        let initial = State1D {
            position_m: 0.9,
            velocity_m_s: -0.2,
        };
        let horizon = 1.0;
        let exact = model.exact_state(initial, horizon).unwrap();
        let mut errors = Vec::new();

        for dt in [0.02, 0.01, 0.005] {
            let mut state = initial;
            let steps = (horizon / dt).round() as usize;
            for _ in 0..steps {
                state = model.discrete_step(state, dt).unwrap();
            }
            let error = (state.position_m - exact.position_m)
                .hypot(state.velocity_m_s - exact.velocity_m_s);
            errors.push(error);
        }
        assert!(errors[1] < errors[0] * 0.65, "errors={errors:?}");
        assert!(errors[2] < errors[1] * 0.65, "errors={errors:?}");
    }

    #[test]
    fn exact_oracle_handles_underdamped_critical_and_overdamped_regimes() {
        let initial = State1D {
            position_m: 0.7,
            velocity_m_s: -0.3,
        };
        for damping in [0.2, 2.0, 8.0] {
            let model = LinearOscillator::try_new(1.0, damping, 1.0).unwrap();
            let at_zero = model.exact_state(initial, 0.0).unwrap();
            close(at_zero.position_m, initial.position_m, 1.0e-14);
            close(at_zero.velocity_m_s, initial.velocity_m_s, 1.0e-14);
            let at_time = model.exact_state(initial, 0.5).unwrap();
            assert!(at_time.position_m.is_finite());
            assert!(at_time.velocity_m_s.is_finite());
        }
    }

    #[test]
    fn dimensionless_parameters_are_unit_invariant() {
        let si = LinearOscillator::try_new(2.0, 0.8, 18.0).unwrap();
        let time_scale = 7.0;
        let transformed = LinearOscillator::try_new(
            2.0,
            0.8 / time_scale,
            18.0 / (time_scale * time_scale),
        )
        .unwrap();
        let p_si = si.dimensionless_parameters(0.03).unwrap();
        let p_scaled = transformed
            .dimensionless_parameters(0.03 * time_scale)
            .unwrap();
        close(p_si.rho, p_scaled.rho, 1.0e-14);
        close(p_si.zeta, p_scaled.zeta, 1.0e-14);
    }

    #[test]
    fn invalid_model_state_and_timestep_inputs_fail_closed() {
        assert_eq!(
            LinearOscillator::try_new(0.0, 0.0, 1.0),
            Err(ModelError::InvalidMass)
        );
        assert_eq!(
            LinearOscillator::try_new(1.0, -1.0, 1.0),
            Err(ModelError::InvalidDamping)
        );
        assert_eq!(
            LinearOscillator::try_new(1.0, 0.0, -1.0),
            Err(ModelError::InvalidStiffness)
        );

        let model = fixture(0.0);
        let state = State1D {
            position_m: f64::NAN,
            velocity_m_s: 0.0,
        };
        assert_eq!(model.discrete_step(state, 0.1), Err(ModelError::InvalidState));
        assert_eq!(
            model.discrete_step(
                State1D {
                    position_m: 0.0,
                    velocity_m_s: 0.0
                },
                0.0
            ),
            Err(ModelError::InvalidTimestep)
        );
    }
}

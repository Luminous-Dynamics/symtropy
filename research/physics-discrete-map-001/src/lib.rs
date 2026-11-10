// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent scalar oracle for a stiffness- and damping-aware discrete step.
//!
//! End-of-step equations:
//!   m (v_next - v) / h = -k x_next - c v_next + u
//!   x_next = x + h v_next
//!
//! Closed form:
//!   den = m + h c + h^2 k
//!   v_next = (m v - h k x + h u) / den
//!   x_next = x + h v_next
//!
//! This is an independent reference kernel, not a production Symtropy
//! integrator or a MuJoCo reimplementation. Stability and physical accuracy
//! are reported separately.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub position_m: f64,
    pub velocity_m_s: f64,
}

impl State {
    pub fn new(position_m: f64, velocity_m_s: f64) -> Self {
        Self { position_m, velocity_m_s }
    }

    fn is_finite(self) -> bool {
        self.position_m.is_finite() && self.velocity_m_s.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    pub mass_kg: f64,
    pub damping_ns_m: f64,
    pub stiffness_n_m: f64,
    pub step_s: f64,
}

impl Parameters {
    pub fn new(mass_kg: f64, damping_ns_m: f64, stiffness_n_m: f64, step_s: f64) -> Self {
        Self { mass_kg, damping_ns_m, stiffness_n_m, step_s }
    }

    fn validate(self) -> Result<(), StepError> {
        if !self.mass_kg.is_finite()
            || !self.damping_ns_m.is_finite()
            || !self.stiffness_n_m.is_finite()
            || !self.step_s.is_finite()
        {
            return Err(StepError::NonFiniteInput);
        }
        if self.mass_kg <= 0.0 {
            return Err(StepError::InvalidMass);
        }
        if self.step_s <= 0.0 {
            return Err(StepError::InvalidStep);
        }
        if self.damping_ns_m < 0.0 {
            return Err(StepError::NegativeDamping);
        }
        if self.stiffness_n_m < 0.0 {
            return Err(StepError::NegativeStiffness);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub state: State,
    pub parameters: Parameters,
    pub force_n: f64,
}

impl Input {
    pub fn new(state: State, parameters: Parameters, force_n: f64) -> Self {
        Self { state, parameters, force_n }
    }

    fn validate(self) -> Result<(), StepError> {
        self.parameters.validate()?;
        if !self.state.is_finite() || !self.force_n.is_finite() {
            return Err(StepError::NonFiniteInput);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    NonFiniteInput,
    InvalidMass,
    InvalidStep,
    NegativeDamping,
    NegativeStiffness,
    NonFiniteArithmetic,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "state, force, or parameter is non-finite"),
            Self::InvalidMass => write!(f, "mass must be strictly positive"),
            Self::InvalidStep => write!(f, "step size must be strictly positive"),
            Self::NegativeDamping => write!(f, "damping must be nonnegative"),
            Self::NegativeStiffness => write!(f, "stiffness must be nonnegative"),
            Self::NonFiniteArithmetic => write!(f, "discrete transition overflowed or was non-finite"),
        }
    }
}

impl Error for StepError {}

/// Derivatives of [x_next, v_next].
/// State columns are [x, v]. Parameter columns are [m, c, k, u, h].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepJacobian {
    pub state: [[f64; 2]; 2],
    pub parameters: [[f64; 5]; 2],
}

impl StepJacobian {
    pub fn state_spectral_radius(self) -> f64 {
        spectral_radius_2x2(self.state)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub state_next: State,
    /// Finite-difference step acceleration (v_next - v) / h.
    pub step_acceleration_m_s2: f64,
    /// Effective scalar metric m + h c + h^2 k.
    pub effective_metric_kg: f64,
    pub jacobian: StepJacobian,
}

/// Evaluate one implicit, stiffness-aware discrete transition and analytic
/// state/parameter Jacobians of the implemented map.
pub fn step(input: Input) -> Result<Transition, StepError> {
    input.validate()?;
    let state = input.state;
    let p = input.parameters;
    let m = p.mass_kg;
    let c = p.damping_ns_m;
    let k = p.stiffness_n_m;
    let h = p.step_s;
    let u = input.force_n;

    let h2 = h * h;
    let denominator = m + h * c + h2 * k;
    let numerator = m * state.velocity_m_s - h * k * state.position_m + h * u;
    if !denominator.is_finite()
        || denominator <= 0.0
        || !numerator.is_finite()
        || !h2.is_finite()
    {
        return Err(StepError::NonFiniteArithmetic);
    }

    let velocity_next = numerator / denominator;
    let position_next = state.position_m + h * velocity_next;
    let step_acceleration = (velocity_next - state.velocity_m_s) / h;
    let denominator_squared = denominator * denominator;
    if !velocity_next.is_finite()
        || !position_next.is_finite()
        || !step_acceleration.is_finite()
        || !denominator_squared.is_finite()
        || denominator_squared <= 0.0
    {
        return Err(StepError::NonFiniteArithmetic);
    }

    let dv_dx = -h * k / denominator;
    let dv_dv = m / denominator;
    let state_jacobian = [
        [1.0 + h * dv_dx, h * dv_dv],
        [dv_dx, dv_dv],
    ];

    let dv_dm = (state.velocity_m_s * denominator - numerator) / denominator_squared;
    let dv_dc = -h * numerator / denominator_squared;
    let dv_dk =
        (-h * state.position_m * denominator - h2 * numerator) / denominator_squared;
    let dv_du = h / denominator;
    let denominator_h = c + 2.0 * h * k;
    let numerator_h = -k * state.position_m + u;
    let dv_dh =
        (numerator_h * denominator - numerator * denominator_h) / denominator_squared;
    let parameter_jacobian = [
        [
            h * dv_dm,
            h * dv_dc,
            h * dv_dk,
            h * dv_du,
            velocity_next + h * dv_dh,
        ],
        [dv_dm, dv_dc, dv_dk, dv_du, dv_dh],
    ];

    let values = [
        state_jacobian[0][0], state_jacobian[0][1],
        state_jacobian[1][0], state_jacobian[1][1],
        parameter_jacobian[0][0], parameter_jacobian[0][1],
        parameter_jacobian[0][2], parameter_jacobian[0][3],
        parameter_jacobian[0][4], parameter_jacobian[1][0],
        parameter_jacobian[1][1], parameter_jacobian[1][2],
        parameter_jacobian[1][3], parameter_jacobian[1][4],
    ];
    if !values.iter().all(|value| value.is_finite()) {
        return Err(StepError::NonFiniteArithmetic);
    }

    Ok(Transition {
        state_next: State::new(position_next, velocity_next),
        step_acceleration_m_s2: step_acceleration,
        effective_metric_kg: denominator,
        jacobian: StepJacobian {
            state: state_jacobian,
            parameters: parameter_jacobian,
        },
    })
}

pub fn spectral_radius_2x2(matrix: [[f64; 2]; 2]) -> f64 {
    let trace = matrix[0][0] + matrix[1][1];
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    let discriminant = trace * trace - 4.0 * determinant;
    if !trace.is_finite() || !determinant.is_finite() || !discriminant.is_finite() {
        return f64::INFINITY;
    }
    if discriminant < 0.0 {
        if determinant < 0.0 {
            f64::INFINITY
        } else {
            determinant.sqrt()
        }
    } else {
        let root = discriminant.sqrt();
        (0.5 * (trace + root)).abs().max((0.5 * (trace - root)).abs())
    }
}

pub fn explicit_euler_matrix(mass_kg: f64, stiffness_n_m: f64, step_s: f64) -> [[f64; 2]; 2] {
    let omega_squared = stiffness_n_m / mass_kg;
    [[1.0, step_s], [-step_s * omega_squared, 1.0]]
}

pub fn semi_implicit_euler_matrix(
    mass_kg: f64,
    stiffness_n_m: f64,
    step_s: f64,
) -> [[f64; 2]; 2] {
    let omega_squared = stiffness_n_m / mass_kg;
    [
        [1.0 - step_s * step_s * omega_squared, step_s],
        [-step_s * omega_squared, 1.0],
    ]
}

/// Exact unforced continuous-time solution for under/critical/overdamped and
/// zero-stiffness cases. This is the independent trajectory oracle.
pub fn exact_unforced(
    initial: State,
    mass_kg: f64,
    damping_ns_m: f64,
    stiffness_n_m: f64,
    elapsed_s: f64,
) -> Result<State, StepError> {
    if !initial.is_finite()
        || !mass_kg.is_finite()
        || !damping_ns_m.is_finite()
        || !stiffness_n_m.is_finite()
        || !elapsed_s.is_finite()
    {
        return Err(StepError::NonFiniteInput);
    }
    if mass_kg <= 0.0 {
        return Err(StepError::InvalidMass);
    }
    if damping_ns_m < 0.0 {
        return Err(StepError::NegativeDamping);
    }
    if stiffness_n_m < 0.0 {
        return Err(StepError::NegativeStiffness);
    }
    if elapsed_s < 0.0 {
        return Err(StepError::InvalidStep);
    }
    if elapsed_s == 0.0 {
        return Ok(initial);
    }

    let alpha = damping_ns_m / (2.0 * mass_kg);
    let omega_squared = stiffness_n_m / mass_kg;
    if stiffness_n_m == 0.0 {
        if alpha == 0.0 {
            return finite_state(
                initial.position_m + initial.velocity_m_s * elapsed_s,
                initial.velocity_m_s,
            );
        }
        let decay = (-2.0 * alpha * elapsed_s).exp();
        let velocity = initial.velocity_m_s * decay;
        let position = initial.position_m
            + initial.velocity_m_s * (1.0 - decay) / (2.0 * alpha);
        return finite_state(position, velocity);
    }

    let discriminant = alpha * alpha - omega_squared;
    let scale = alpha * alpha + omega_squared + 1.0;
    let critical_tolerance = 64.0 * f64::EPSILON * scale;
    if discriminant < -critical_tolerance {
        let omega_d = (-discriminant).sqrt();
        let phase_velocity = (initial.velocity_m_s + alpha * initial.position_m) / omega_d;
        let cosine = (omega_d * elapsed_s).cos();
        let sine = (omega_d * elapsed_s).sin();
        let envelope = (-alpha * elapsed_s).exp();
        let oscillatory = initial.position_m * cosine + phase_velocity * sine;
        let position = envelope * oscillatory;
        let velocity = envelope
            * (-alpha * oscillatory - initial.position_m * omega_d * sine
                + phase_velocity * omega_d * cosine);
        finite_state(position, velocity)
    } else if discriminant.abs() <= critical_tolerance {
        let coefficient = initial.velocity_m_s + alpha * initial.position_m;
        let linear = initial.position_m + coefficient * elapsed_s;
        let envelope = (-alpha * elapsed_s).exp();
        finite_state(envelope * linear, envelope * (coefficient - alpha * linear))
    } else {
        let root = discriminant.sqrt();
        let r1 = -alpha + root;
        let r2 = -alpha - root;
        let a = (initial.velocity_m_s - r2 * initial.position_m) / (r1 - r2);
        let b = (r1 * initial.position_m - initial.velocity_m_s) / (r1 - r2);
        let e1 = (r1 * elapsed_s).exp();
        let e2 = (r2 * elapsed_s).exp();
        finite_state(a * e1 + b * e2, a * r1 * e1 + b * r2 * e2)
    }
}

fn finite_state(position_m: f64, velocity_m_s: f64) -> Result<State, StepError> {
    if position_m.is_finite() && velocity_m_s.is_finite() {
        Ok(State::new(position_m, velocity_m_s))
    } else {
        Err(StepError::NonFiniteArithmetic)
    }
}

/// Coherent base-unit rescaling. L, T and M are new-unit scales relative to
/// the old units; x'=Lx, t'=Tt, m'=Mm, c'=Mc/T, k'=Mk/T², force'=ML/T².
pub fn rescale_units(
    input: Input,
    length_scale: f64,
    time_scale: f64,
    mass_scale: f64,
) -> Result<Input, StepError> {
    input.validate()?;
    if !length_scale.is_finite()
        || !time_scale.is_finite()
        || !mass_scale.is_finite()
        || length_scale <= 0.0
        || time_scale <= 0.0
        || mass_scale <= 0.0
    {
        return Err(StepError::NonFiniteInput);
    }
    let l = length_scale;
    let t = time_scale;
    let m = mass_scale;
    let transformed = Input::new(
        State::new(l * input.state.position_m, (l / t) * input.state.velocity_m_s),
        Parameters::new(
            m * input.parameters.mass_kg,
            (m / t) * input.parameters.damping_ns_m,
            (m / (t * t)) * input.parameters.stiffness_n_m,
            t * input.parameters.step_s,
        ),
        (m * l / (t * t)) * input.force_n,
    );
    transformed.validate()?;
    Ok(transformed)
}

pub fn rollout(
    mut state: State,
    parameters: Parameters,
    force_n: f64,
    steps: usize,
) -> Result<State, StepError> {
    Input::new(state, parameters, force_n).validate()?;
    for _ in 0..steps {
        state = step(Input::new(state, parameters, force_n))?.state_next;
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        let error = (actual - expected).abs();
        assert!(
            error <= tolerance * (1.0 + expected.abs()),
            "actual={actual:.16e}, expected={expected:.16e}, abs_error={error:.4e}"
        );
    }

    fn fixture() -> Input {
        Input::new(
            State::new(0.37, -0.82),
            Parameters::new(1.7, 0.43, 12.5, 0.037),
            2.3,
        )
    }

    fn with_coordinate(input: Input, coordinate: usize, value: f64) -> Input {
        let mut result = input;
        match coordinate {
            0 => result.state.position_m = value,
            1 => result.state.velocity_m_s = value,
            2 => result.parameters.mass_kg = value,
            3 => result.parameters.damping_ns_m = value,
            4 => result.parameters.stiffness_n_m = value,
            5 => result.force_n = value,
            6 => result.parameters.step_s = value,
            _ => unreachable!("test coordinate is in range"),
        }
        result
    }

    fn coordinate_value(input: Input, coordinate: usize) -> f64 {
        match coordinate {
            0 => input.state.position_m,
            1 => input.state.velocity_m_s,
            2 => input.parameters.mass_kg,
            3 => input.parameters.damping_ns_m,
            4 => input.parameters.stiffness_n_m,
            5 => input.force_n,
            6 => input.parameters.step_s,
            _ => unreachable!("test coordinate is in range"),
        }
    }

    fn analytic_derivative(transition: Transition, output: usize, coordinate: usize) -> f64 {
        match coordinate {
            0 => transition.jacobian.state[output][0],
            1 => transition.jacobian.state[output][1],
            2 => transition.jacobian.parameters[output][0],
            3 => transition.jacobian.parameters[output][1],
            4 => transition.jacobian.parameters[output][2],
            5 => transition.jacobian.parameters[output][3],
            6 => transition.jacobian.parameters[output][4],
            _ => unreachable!("test coordinate is in range"),
        }
    }

    fn output_value(transition: Transition, output: usize) -> f64 {
        match output {
            0 => transition.state_next.position_m,
            1 => transition.state_next.velocity_m_s,
            _ => unreachable!("test output is in range"),
        }
    }

    fn central_difference(input: Input, output: usize, coordinate: usize, scale: f64) -> f64 {
        let value = coordinate_value(input, coordinate);
        let epsilon = scale * value.abs().max(1.0);
        let plus = with_coordinate(input, coordinate, value + epsilon);
        let minus = with_coordinate(input, coordinate, value - epsilon);
        let y_plus = output_value(step(plus).expect("positive perturbation valid"), output);
        let y_minus = output_value(step(minus).expect("negative perturbation valid"), output);
        (y_plus - y_minus) / (2.0 * epsilon)
    }

    #[test]
    fn closed_form_satisfies_end_of_step_equations() {
        let input = fixture();
        let result = step(input).unwrap();
        let p = input.parameters;
        let h = p.step_s;
        let acceleration = (result.state_next.velocity_m_s - input.state.velocity_m_s) / h;
        close(
            p.mass_kg * acceleration,
            -p.stiffness_n_m * result.state_next.position_m
                - p.damping_ns_m * result.state_next.velocity_m_s
                + input.force_n,
            2.0e-14,
        );
        close(
            result.state_next.position_m,
            input.state.position_m + h * result.state_next.velocity_m_s,
            2.0e-14,
        );
        close(result.step_acceleration_m_s2, acceleration, 1.0e-14);
    }

    #[test]
    fn analytic_state_and_parameter_jacobians_match_step_ladder() {
        let input = fixture();
        let result = step(input).unwrap();
        for coordinate in 0..7 {
            for output in 0..2 {
                let analytic = analytic_derivative(result, output, coordinate);
                let mut best_error = f64::INFINITY;
                for scale in [1.0e-4, 3.0e-5, 1.0e-5, 3.0e-6, 1.0e-6] {
                    let measured = central_difference(input, output, coordinate, scale);
                    let relative_error =
                        (analytic - measured).abs() / (1.0 + analytic.abs().max(measured.abs()));
                    best_error = best_error.min(relative_error);
                }
                assert!(
                    best_error < 2.0e-8,
                    "coordinate={coordinate}, output={output}, best relative error={best_error:e}"
                );
            }
        }
    }

    #[test]
    fn step_acceleration_is_not_continuous_acceleration() {
        let input = fixture();
        let transition = step(input).unwrap();
        let continuous_acceleration = (-input.parameters.stiffness_n_m * input.state.position_m
            - input.parameters.damping_ns_m * input.state.velocity_m_s
            + input.force_n)
            / input.parameters.mass_kg;
        assert!(
            (transition.step_acceleration_m_s2 - continuous_acceleration).abs() > 1.0e-4
        );
        close(
            transition.step_acceleration_m_s2,
            (transition.state_next.velocity_m_s - input.state.velocity_m_s)
                / input.parameters.step_s,
            1.0e-14,
        );
    }

    #[test]
    fn implicit_step_is_spectral_stable_for_positive_scalar_parameters() {
        let mut rng = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..10_000 {
            let mass = sample_log(&mut rng, 1.0e-4, 1.0e4);
            let damping = sample_linear(&mut rng, 0.0, 1.0e5);
            let stiffness = sample_log(&mut rng, 1.0e-6, 1.0e8);
            let step_s = sample_log(&mut rng, 1.0e-6, 1.0e3);
            let input = Input::new(
                State::new(sample_signed(&mut rng, 1.0e4), sample_signed(&mut rng, 1.0e4)),
                Parameters::new(mass, damping, stiffness, step_s),
                sample_signed(&mut rng, 1.0e4),
            );
            let result = step(input).expect("bounded positive parameters must yield finite state");
            let radius = result.jacobian.state_spectral_radius();
            assert!(
                radius <= 1.0 + 2.0e-10,
                "spectral radius={radius:e}, parameters={:?}",
                input.parameters
            );
        }
    }

    #[test]
    fn explicit_and_semi_implicit_negative_controls_expose_large_step_instability() {
        let mass = 1.0;
        let stiffness = 100.0;
        let h = 1.0;
        let explicit_radius = spectral_radius_2x2(explicit_euler_matrix(mass, stiffness, h));
        let semi_radius = spectral_radius_2x2(semi_implicit_euler_matrix(mass, stiffness, h));
        let implicit_radius = step(Input::new(
            State::new(1.0, 0.0),
            Parameters::new(mass, 0.0, stiffness, h),
            0.0,
        ))
        .unwrap()
        .jacobian
        .state_spectral_radius();
        assert!(explicit_radius > 1.0);
        assert!(semi_radius > 1.0);
        assert!(implicit_radius < 1.0);
    }

    #[test]
    fn stable_does_not_mean_accurate_at_large_timestep() {
        let initial = State::new(1.0, 0.0);
        let parameters = Parameters::new(1.0, 0.0, 100.0, 1.0);
        let result = rollout(initial, parameters, 0.0, 1).unwrap();
        let reference = exact_unforced(initial, 1.0, 0.0, 100.0, 1.0).unwrap();
        let error = (result.position_m - reference.position_m).abs()
            + (result.velocity_m_s - reference.velocity_m_s).abs();
        let radius = step(Input::new(initial, parameters, 0.0))
            .unwrap()
            .jacobian
            .state_spectral_radius();
        assert!(radius < 1.0);
        assert!(error > 1.0);
    }

    #[test]
    fn global_error_converges_under_step_refinement() {
        let initial = State::new(0.7, -0.2);
        let mass = 1.3;
        let damping = 0.6;
        let stiffness = 17.0;
        let horizon = 1.0;
        let reference = exact_unforced(initial, mass, damping, stiffness, horizon).unwrap();
        let mut errors = Vec::new();
        for steps in [100, 200, 400, 800] {
            let parameters = Parameters::new(mass, damping, stiffness, horizon / steps as f64);
            let result = rollout(initial, parameters, 0.0, steps).unwrap();
            errors.push(
                (result.position_m - reference.position_m).abs()
                    + (result.velocity_m_s - reference.velocity_m_s).abs(),
            );
        }
        assert!(errors.windows(2).all(|pair| pair[1] < pair[0]));
        let order = (errors[0] / errors[1]).ln() / 2.0_f64.ln();
        assert!(
            (0.75..=1.25).contains(&order),
            "expected first-order convergence, observed order={order}"
        );
    }

    #[test]
    fn coherent_si_unit_rescaling_preserves_physical_transition() {
        let input = fixture();
        let baseline = step(input).unwrap();
        let scaled = rescale_units(input, 100.0, 0.01, 1000.0).unwrap();
        let transformed = step(scaled).unwrap();
        close(transformed.state_next.position_m, 100.0 * baseline.state_next.position_m, 2.0e-13);
        close(transformed.state_next.velocity_m_s, 10_000.0 * baseline.state_next.velocity_m_s, 2.0e-13);
        close(transformed.step_acceleration_m_s2, 1_000_000.0 * baseline.step_acceleration_m_s2, 2.0e-12);
    }

    #[test]
    fn invalid_inputs_fail_closed() {
        let good = fixture();
        assert_eq!(
            step(Input::new(good.state, Parameters::new(0.0, 1.0, 1.0, 0.1), 0.0)),
            Err(StepError::InvalidMass)
        );
        assert_eq!(
            step(Input::new(good.state, Parameters::new(1.0, -1.0, 1.0, 0.1), 0.0)),
            Err(StepError::NegativeDamping)
        );
        assert_eq!(
            step(Input::new(good.state, Parameters::new(1.0, 1.0, -1.0, 0.1), 0.0)),
            Err(StepError::NegativeStiffness)
        );
        assert_eq!(
            step(Input::new(good.state, Parameters::new(1.0, 1.0, 1.0, 0.0), 0.0)),
            Err(StepError::InvalidStep)
        );
        assert_eq!(
            step(Input::new(State::new(f64::NAN, 0.0), good.parameters, 0.0)),
            Err(StepError::NonFiniteInput)
        );
    }

    #[test]
    fn extreme_arithmetic_fails_closed() {
        let input = Input::new(
            State::new(f64::MAX, f64::MAX),
            Parameters::new(1.0e-300, 0.0, 1.0e300, 1.0e300),
            0.0,
        );
        assert_eq!(step(input), Err(StepError::NonFiniteArithmetic));
    }

    fn next_u64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = *state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn sample_unit(state: &mut u64) -> f64 {
        let mantissa = next_u64(state) >> 11;
        mantissa as f64 / ((1_u64 << 53) as f64)
    }

    fn sample_log(state: &mut u64, minimum: f64, maximum: f64) -> f64 {
        (minimum.ln() + sample_unit(state) * (maximum.ln() - minimum.ln())).exp()
    }

    fn sample_linear(state: &mut u64, minimum: f64, maximum: f64) -> f64 {
        minimum + sample_unit(state) * (maximum - minimum)
    }

    fn sample_signed(state: &mut u64, magnitude: f64) -> f64 {
        (2.0 * sample_unit(state) - 1.0) * magnitude
    }
}

// Fixed 4D tensor contractions retain explicit Einstein-index loops.
#![allow(clippy::needless_range_loop)]
// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Schwarzschild timelike and null geodesics on the exterior chart.
//!
//! Coordinates are x^mu = (c*t, r, theta, phi), with c*t and r in metres.
//! The integration parameter is a length:
//! - TimelikeProperLength means lambda = c*tau, so g_mu_nu u^mu u^nu = -1.
//! - NullAffine means an affine parameter with a caller-declared tangent
//!   normalization, conventionally g_mu_nu u^mu u^nu = 0.
//!
//! Coordinate time is x^0/c and is never substituted for proper time.
//! Schwarzschild coordinates exclude the horizon; this integrator reports a
//! domain error rather than crossing a coordinate singularity. Horizon-regular
//! charts and Kerr geodesics are later work.

use core::fmt;

use super::{
    PhysicsDomainError, SchwarzschildBlackHole,
    connection::{ConnectionError, LeviCivitaConnection4, MetricDerivatives4},
    metric::{MetricComponents4, MetricError, schwarzschild_exterior_metric},
};

/// Meaning of the geodesic parameter and its normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeodesicParameterization {
    /// lambda = c * proper_time, measured in metres; timelike norm is -1.
    TimelikeProperLength,
    /// An affine parameter measured in the caller's chosen length scale.
    /// The tangent normalization is expected to be null (zero).
    NullAffine,
}

impl GeodesicParameterization {
    const fn expected_norm(self) -> f64 {
        match self {
            Self::TimelikeProperLength => -1.0,
            Self::NullAffine => 0.0,
        }
    }
}

/// Position and tangent in the declared Schwarzschild coordinate basis.
///
/// Coordinates: [ct (m), r (m), theta (rad), phi (rad)].
/// Tangent: dx^mu/dlambda for the parameterization declared by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicState {
    pub coordinates: [f64; 4],
    pub tangent: [f64; 4],
}

impl GeodesicState {
    pub const fn new(coordinates: [f64; 4], tangent: [f64; 4]) -> Self {
        Self {
            coordinates,
            tangent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KillingConstants {
    /// Conserved specific Killing energy, E = -p_t for the declared tangent.
    pub energy: f64,
    /// Conserved axial angular momentum, L_z = p_phi.
    pub angular_momentum_z_m: f64,
}

/// Component-wise absolute tolerances for adaptive geodesic integration.
///
/// Coordinate tolerances use [m, m, rad, rad] for [ct, r, theta, phi].
/// Tangent tolerances use the corresponding units [1, 1, 1/m, 1/m] when
/// lambda is a length. Keeping these separate avoids applying one absolute
/// tolerance to dimensionally different state components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicTolerances {
    pub relative: f64,
    pub coordinate_absolute: [f64; 4],
    pub tangent_absolute: [f64; 4],
}

impl GeodesicTolerances {
    pub fn new(
        relative: f64,
        coordinate_absolute: [f64; 4],
        tangent_absolute: [f64; 4],
    ) -> Result<Self, GeodesicError> {
        if !relative.is_finite() || relative <= 0.0 {
            return Err(GeodesicError::InvalidTolerance {
                group: "relative",
                index: None,
                value: relative,
            });
        }
        for (index, value) in coordinate_absolute.iter().enumerate() {
            if !value.is_finite() || *value <= 0.0 {
                return Err(GeodesicError::InvalidTolerance {
                    group: "coordinate_absolute",
                    index: Some(index),
                    value: *value,
                });
            }
        }
        for (index, value) in tangent_absolute.iter().enumerate() {
            if !value.is_finite() || *value <= 0.0 {
                return Err(GeodesicError::InvalidTolerance {
                    group: "tangent_absolute",
                    index: Some(index),
                    value: *value,
                });
            }
        }
        Ok(Self {
            relative,
            coordinate_absolute,
            tangent_absolute,
        })
    }
}

/// Adaptive integration summary. Returned states include the initial state and
/// every accepted step endpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveGeodesicResult {
    pub states: Vec<GeodesicState>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub final_step_m: f64,
}

/// Canonical Hamiltonian state (x^mu, p_mu) for an independent
/// first-order formulation of the same Schwarzschild geodesic problem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HamiltonianGeodesicState {
    pub coordinates: [f64; 4],
    pub canonical_momenta: [f64; 4],
}

impl HamiltonianGeodesicState {
    pub const fn new(coordinates: [f64; 4], canonical_momenta: [f64; 4]) -> Self {
        Self {
            coordinates,
            canonical_momenta,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct HamiltonianDerivative {
    coordinates: [f64; 4],
    momenta: [f64; 4],
}

#[derive(Debug, Clone, Copy)]
struct StateDerivative {
    coordinates: [f64; 4],
    tangent: [f64; 4],
}

/// Fixed-background Schwarzschild geodesic integrator.
///
/// The first implementation uses classical RK4 and step-doubling adaptivity.
/// Metric and connection coefficients are evaluated from the analytic
/// Schwarzschild exterior at each RK stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchwarzschildGeodesicIntegrator {
    black_hole: SchwarzschildBlackHole,
}

impl SchwarzschildGeodesicIntegrator {
    pub const fn new(black_hole: SchwarzschildBlackHole) -> Self {
        Self { black_hole }
    }

    pub const fn black_hole(self) -> SchwarzschildBlackHole {
        self.black_hole
    }

    fn validate_state(&self, state: &GeodesicState) -> Result<(), GeodesicError> {
        for (index, value) in state.coordinates.iter().enumerate() {
            if !value.is_finite() {
                return Err(GeodesicError::NonFiniteCoordinate { index });
            }
        }
        for (index, value) in state.tangent.iter().enumerate() {
            if !value.is_finite() {
                return Err(GeodesicError::NonFiniteTangent { index });
            }
        }

        let radius = state.coordinates[1];
        let theta = state.coordinates[2];
        let horizon = self.black_hole.horizon_radius_m();

        if !radius.is_finite() || radius <= horizon {
            return Err(GeodesicError::OutsideExteriorChart {
                radius_m: radius,
                horizon_m: horizon,
            });
        }
        if !theta.is_finite() || theta <= 0.0 || theta >= core::f64::consts::PI {
            return Err(GeodesicError::OutsidePolarChart { theta_rad: theta });
        }

        Ok(())
    }

    fn metric_at(&self, state: &GeodesicState) -> Result<MetricComponents4, GeodesicError> {
        self.validate_state(state)?;
        Ok(schwarzschild_exterior_metric(
            self.black_hole,
            state.coordinates[1],
            state.coordinates[2],
        )?)
    }

    fn state_derivative(&self, state: &GeodesicState) -> Result<StateDerivative, GeodesicError> {
        self.validate_state(state)?;

        let radius = state.coordinates[1];
        let theta = state.coordinates[2];
        let metric = schwarzschild_exterior_metric(self.black_hole, radius, theta)?;
        let derivatives =
            MetricDerivatives4::schwarzschild_exterior(self.black_hole, radius, theta)?;
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives)?;

        let mut acceleration = [0.0; 4];
        for rho in 0..4 {
            let mut value = 0.0;
            for mu in 0..4 {
                for nu in 0..4 {
                    value +=
                        connection.coefficient(rho, mu, nu) * state.tangent[mu] * state.tangent[nu];
                }
            }
            acceleration[rho] = -value;
            if !acceleration[rho].is_finite() {
                return Err(GeodesicError::NonFiniteAcceleration { index: rho });
            }
        }

        Ok(StateDerivative {
            coordinates: state.tangent,
            tangent: acceleration,
        })
    }

    fn shifted_state(
        state: GeodesicState,
        derivative: StateDerivative,
        scale: f64,
    ) -> GeodesicState {
        let mut coordinates = state.coordinates;
        let mut tangent = state.tangent;

        for index in 0..4 {
            coordinates[index] += scale * derivative.coordinates[index];
            tangent[index] += scale * derivative.tangent[index];
        }

        GeodesicState::new(coordinates, tangent)
    }

    fn rk4_step(&self, state: GeodesicState, step_m: f64) -> Result<GeodesicState, GeodesicError> {
        if !step_m.is_finite() || step_m == 0.0 {
            return Err(GeodesicError::InvalidStep(step_m));
        }

        let k1 = self.state_derivative(&state)?;
        let k2_state = Self::shifted_state(state, k1, 0.5 * step_m);
        let k2 = self.state_derivative(&k2_state)?;
        let k3_state = Self::shifted_state(state, k2, 0.5 * step_m);
        let k3 = self.state_derivative(&k3_state)?;
        let k4_state = Self::shifted_state(state, k3, step_m);
        let k4 = self.state_derivative(&k4_state)?;

        let mut next_coordinates = state.coordinates;
        let mut next_tangent = state.tangent;

        for index in 0..4 {
            next_coordinates[index] += (step_m / 6.0)
                * (k1.coordinates[index]
                    + 2.0 * k2.coordinates[index]
                    + 2.0 * k3.coordinates[index]
                    + k4.coordinates[index]);
            next_tangent[index] += (step_m / 6.0)
                * (k1.tangent[index]
                    + 2.0 * k2.tangent[index]
                    + 2.0 * k3.tangent[index]
                    + k4.tangent[index]);
        }

        let result = GeodesicState::new(next_coordinates, next_tangent);
        self.validate_state(&result)?;
        Ok(result)
    }

    /// Take one classical fourth-order Runge-Kutta step.
    pub fn step_rk4(
        &self,
        state: GeodesicState,
        step_m: f64,
    ) -> Result<GeodesicState, GeodesicError> {
        self.rk4_step(state, step_m)
    }

    /// Integrate a fixed number of equal RK4 steps. Returned states include
    /// the initial state and every step endpoint.
    pub fn integrate_fixed(
        &self,
        initial: GeodesicState,
        step_m: f64,
        steps: usize,
    ) -> Result<Vec<GeodesicState>, GeodesicError> {
        if !step_m.is_finite() || step_m == 0.0 {
            return Err(GeodesicError::InvalidStep(step_m));
        }
        self.validate_state(&initial)?;

        let mut states = Vec::with_capacity(steps.saturating_add(1));
        states.push(initial);
        let mut current = initial;

        for _ in 0..steps {
            current = self.rk4_step(current, step_m)?;
            states.push(current);
        }

        Ok(states)
    }

    /// Integrate over a positive affine/proper-length interval using RK4
    /// step-doubling error control. The estimated normalized local error is
    /// the maximum scaled difference between one full step and two half steps.
    ///
    /// This is a deterministic controller, not a rigorous global error bound.
    pub fn integrate_adaptive(
        &self,
        initial: GeodesicState,
        parameter_span_m: f64,
        initial_step_m: f64,
        tolerances: GeodesicTolerances,
        max_attempts: usize,
    ) -> Result<AdaptiveGeodesicResult, GeodesicError> {
        self.validate_state(&initial)?;

        for (name, value) in [
            ("parameter_span_m", parameter_span_m),
            ("initial_step_m", initial_step_m),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(GeodesicError::InvalidPositiveParameter { name, value });
            }
        }
        if max_attempts == 0 {
            return Err(GeodesicError::ZeroMaximumAttempts);
        }

        let mut states = vec![initial];
        let mut current = initial;
        let mut advanced = 0.0;
        let mut step = initial_step_m;
        let mut accepted = 0;
        let mut rejected = 0;
        let minimum_step = 64.0 * f64::EPSILON * parameter_span_m.max(initial_step_m);

        for _ in 0..max_attempts {
            let endpoint_tolerance = 32.0 * f64::EPSILON * parameter_span_m.max(initial_step_m);
            if advanced >= parameter_span_m || parameter_span_m - advanced <= endpoint_tolerance {
                return Ok(AdaptiveGeodesicResult {
                    states,
                    accepted_steps: accepted,
                    rejected_steps: rejected,
                    final_step_m: step,
                });
            }

            let remaining = parameter_span_m - advanced;
            let trial_step = step.min(remaining);
            if trial_step < minimum_step {
                return Err(GeodesicError::StepSizeUnderflow {
                    step_m: trial_step,
                    minimum_step_m: minimum_step,
                });
            }

            let full = self.rk4_step(current, trial_step)?;
            let first_half = self.rk4_step(current, 0.5 * trial_step)?;
            let two_half = self.rk4_step(first_half, 0.5 * trial_step)?;
            let error = self.step_doubling_error(full, two_half, tolerances);

            if error <= 1.0 {
                current = two_half;
                advanced += trial_step;
                accepted += 1;
                states.push(current);

                let factor = if error == 0.0 {
                    5.0
                } else {
                    (0.9 * error.powf(-0.2)).clamp(0.2, 5.0)
                };
                step = (trial_step * factor).max(minimum_step);
            } else {
                rejected += 1;
                let factor = (0.9 * error.powf(-0.2)).clamp(0.1, 0.5);
                step = trial_step * factor;
            }
        }

        Err(GeodesicError::MaximumAttemptsExceeded {
            accepted_steps: accepted,
            rejected_steps: rejected,
            advanced_parameter_m: advanced,
            requested_parameter_m: parameter_span_m,
        })
    }

    fn step_doubling_error(
        &self,
        full: GeodesicState,
        two_half: GeodesicState,
        tolerances: GeodesicTolerances,
    ) -> f64 {
        let mut maximum: f64 = 0.0;

        for index in 0..4 {
            let coordinate_a = full.coordinates[index];
            let coordinate_b = two_half.coordinates[index];
            let coordinate_scale = tolerances.coordinate_absolute[index]
                + tolerances.relative * coordinate_a.abs().max(coordinate_b.abs());
            maximum = maximum.max((coordinate_a - coordinate_b).abs() / coordinate_scale);

            let tangent_a = full.tangent[index];
            let tangent_b = two_half.tangent[index];
            let tangent_scale = tolerances.tangent_absolute[index]
                + tolerances.relative * tangent_a.abs().max(tangent_b.abs());
            maximum = maximum.max((tangent_a - tangent_b).abs() / tangent_scale);
        }

        maximum
    }

    /// Convert a coordinate-velocity state (x^mu, u^mu) into canonical
    /// variables p_mu = g_mu_nu u^nu.
    pub fn to_hamiltonian_state(
        &self,
        state: GeodesicState,
    ) -> Result<HamiltonianGeodesicState, GeodesicError> {
        let metric = self.metric_at(&state)?;
        let g = metric.covariant();
        let mut momenta = [0.0; 4];

        for mu in 0..4 {
            for nu in 0..4 {
                momenta[mu] += g[mu][nu] * state.tangent[nu];
            }
            if !momenta[mu].is_finite() {
                return Err(GeodesicError::NonFiniteCanonicalMomentum { index: mu });
            }
        }

        Ok(HamiltonianGeodesicState::new(state.coordinates, momenta))
    }

    /// Convert canonical variables back to coordinate velocity using
    /// u^mu = g^mu nu p_nu.
    pub fn from_hamiltonian_state(
        &self,
        state: HamiltonianGeodesicState,
    ) -> Result<GeodesicState, GeodesicError> {
        self.validate_hamiltonian_state(&state)?;
        let metric = schwarzschild_exterior_metric(
            self.black_hole,
            state.coordinates[1],
            state.coordinates[2],
        )?;
        let inverse = metric.inverse()?;
        let mut tangent = [0.0; 4];

        for mu in 0..4 {
            for nu in 0..4 {
                tangent[mu] += inverse[mu][nu] * state.canonical_momenta[nu];
            }
            if !tangent[mu].is_finite() {
                return Err(GeodesicError::NonFiniteTangent { index: mu });
            }
        }

        Ok(GeodesicState::new(state.coordinates, tangent))
    }

    fn validate_hamiltonian_state(
        &self,
        state: &HamiltonianGeodesicState,
    ) -> Result<(), GeodesicError> {
        self.validate_state(&GeodesicState::new(state.coordinates, [0.0; 4]))?;
        for (index, value) in state.canonical_momenta.iter().enumerate() {
            if !value.is_finite() {
                return Err(GeodesicError::NonFiniteCanonicalMomentum { index });
            }
        }
        Ok(())
    }

    fn hamiltonian_derivative(
        &self,
        state: &HamiltonianGeodesicState,
    ) -> Result<HamiltonianDerivative, GeodesicError> {
        self.validate_hamiltonian_state(state)?;
        let radius = state.coordinates[1];
        let theta = state.coordinates[2];
        let metric = schwarzschild_exterior_metric(self.black_hole, radius, theta)?;
        let inverse = metric.inverse()?;
        let derivatives =
            MetricDerivatives4::schwarzschild_exterior(self.black_hole, radius, theta)?;
        let d = derivatives.components();

        let mut velocity = [0.0; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                velocity[mu] += inverse[mu][nu] * state.canonical_momenta[nu];
            }
        }

        // Hamilton's equations:
        // dx^mu/dlambda = g^mu nu p_nu,
        // dp_mu/dlambda = +1/2 (partial_mu g_alpha beta) u^alpha u^beta.
        let mut momentum_rate = [0.0; 4];
        for mu in 0..4 {
            for alpha in 0..4 {
                for beta in 0..4 {
                    momentum_rate[mu] +=
                        0.5 * d[mu][alpha][beta] * velocity[alpha] * velocity[beta];
                }
            }
            if !velocity[mu].is_finite() || !momentum_rate[mu].is_finite() {
                return Err(GeodesicError::NonFiniteHamiltonianDerivative { index: mu });
            }
        }

        Ok(HamiltonianDerivative {
            coordinates: velocity,
            momenta: momentum_rate,
        })
    }

    fn shifted_hamiltonian_state(
        state: HamiltonianGeodesicState,
        derivative: HamiltonianDerivative,
        scale: f64,
    ) -> HamiltonianGeodesicState {
        let mut coordinates = state.coordinates;
        let mut momenta = state.canonical_momenta;
        for index in 0..4 {
            coordinates[index] += scale * derivative.coordinates[index];
            momenta[index] += scale * derivative.momenta[index];
        }
        HamiltonianGeodesicState::new(coordinates, momenta)
    }

    fn hamiltonian_rk4_step(
        &self,
        state: HamiltonianGeodesicState,
        step_m: f64,
    ) -> Result<HamiltonianGeodesicState, GeodesicError> {
        if !step_m.is_finite() || step_m == 0.0 {
            return Err(GeodesicError::InvalidStep(step_m));
        }
        let k1 = self.hamiltonian_derivative(&state)?;
        let k2 =
            self.hamiltonian_derivative(&Self::shifted_hamiltonian_state(state, k1, 0.5 * step_m))?;
        let k3 =
            self.hamiltonian_derivative(&Self::shifted_hamiltonian_state(state, k2, 0.5 * step_m))?;
        let k4 =
            self.hamiltonian_derivative(&Self::shifted_hamiltonian_state(state, k3, step_m))?;

        let mut coordinates = state.coordinates;
        let mut momenta = state.canonical_momenta;
        for index in 0..4 {
            coordinates[index] += (step_m / 6.0)
                * (k1.coordinates[index]
                    + 2.0 * k2.coordinates[index]
                    + 2.0 * k3.coordinates[index]
                    + k4.coordinates[index]);
            momenta[index] += (step_m / 6.0)
                * (k1.momenta[index]
                    + 2.0 * k2.momenta[index]
                    + 2.0 * k3.momenta[index]
                    + k4.momenta[index]);
        }

        let result = HamiltonianGeodesicState::new(coordinates, momenta);
        self.validate_hamiltonian_state(&result)?;
        Ok(result)
    }

    /// Take one classical RK4 step using Hamilton's first-order canonical
    /// equations. This is an independent formulation of the second-order
    /// Christoffel geodesic equation above.
    pub fn step_hamiltonian_rk4(
        &self,
        state: HamiltonianGeodesicState,
        step_m: f64,
    ) -> Result<HamiltonianGeodesicState, GeodesicError> {
        self.hamiltonian_rk4_step(state, step_m)
    }

    /// Integrate canonical Hamiltonian variables with equal RK4 steps.
    pub fn integrate_hamiltonian_fixed(
        &self,
        initial: HamiltonianGeodesicState,
        step_m: f64,
        steps: usize,
    ) -> Result<Vec<HamiltonianGeodesicState>, GeodesicError> {
        if !step_m.is_finite() || step_m == 0.0 {
            return Err(GeodesicError::InvalidStep(step_m));
        }
        self.validate_hamiltonian_state(&initial)?;
        let mut states = Vec::with_capacity(steps.saturating_add(1));
        let mut current = initial;
        states.push(current);
        for _ in 0..steps {
            current = self.hamiltonian_rk4_step(current, step_m)?;
            states.push(current);
        }
        Ok(states)
    }

    /// Evaluate H = 1/2 g^mu nu p_mu p_nu. Timelike proper-length
    /// parameterization has H=-1/2; null affine trajectories have H=0.
    pub fn hamiltonian_value(&self, state: HamiltonianGeodesicState) -> Result<f64, GeodesicError> {
        self.validate_hamiltonian_state(&state)?;
        let metric = schwarzschild_exterior_metric(
            self.black_hole,
            state.coordinates[1],
            state.coordinates[2],
        )?;
        let inverse = metric.inverse()?;
        let mut value = 0.0;
        for mu in 0..4 {
            for nu in 0..4 {
                value += 0.5
                    * inverse[mu][nu]
                    * state.canonical_momenta[mu]
                    * state.canonical_momenta[nu];
            }
        }
        if value.is_finite() {
            Ok(value)
        } else {
            Err(GeodesicError::NonFiniteHamiltonian)
        }
    }

    /// Compute g_mu_nu u^mu u^nu for a state in the Schwarzschild exterior.
    pub fn normalization(&self, state: GeodesicState) -> Result<f64, GeodesicError> {
        let metric = self.metric_at(&state)?;
        let g = metric.covariant();
        let mut value = 0.0;

        for mu in 0..4 {
            for nu in 0..4 {
                value += g[mu][nu] * state.tangent[mu] * state.tangent[nu];
            }
        }

        if value.is_finite() {
            Ok(value)
        } else {
            Err(GeodesicError::NonFiniteNormalization)
        }
    }

    /// Absolute deviation from -1 (timelike proper-length parameter) or zero
    /// (null affine parameter).
    pub fn normalization_residual(
        &self,
        state: GeodesicState,
        parameterization: GeodesicParameterization,
    ) -> Result<f64, GeodesicError> {
        Ok((self.normalization(state)? - parameterization.expected_norm()).abs())
    }

    /// Conserved Schwarzschild Killing energy and axial angular momentum
    /// for the supplied state and declared coordinate basis.
    pub fn killing_constants(
        &self,
        state: GeodesicState,
    ) -> Result<KillingConstants, GeodesicError> {
        let metric = self.metric_at(&state)?;
        let g = metric.covariant();

        let mut p_t = 0.0;
        let mut p_phi = 0.0;
        for nu in 0..4 {
            p_t += g[0][nu] * state.tangent[nu];
            p_phi += g[3][nu] * state.tangent[nu];
        }

        if p_t.is_finite() && p_phi.is_finite() {
            Ok(KillingConstants {
                energy: -p_t,
                angular_momentum_z_m: p_phi,
            })
        } else {
            Err(GeodesicError::NonFiniteKillingConstant)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeodesicError {
    PhysicsDomain(PhysicsDomainError),
    Metric(MetricError),
    Connection(ConnectionError),
    InvalidStep(f64),
    InvalidPositiveParameter {
        name: &'static str,
        value: f64,
    },
    ZeroMaximumAttempts,
    InvalidTolerance {
        group: &'static str,
        index: Option<usize>,
        value: f64,
    },
    NonFiniteCoordinate {
        index: usize,
    },
    NonFiniteTangent {
        index: usize,
    },
    NonFiniteCanonicalMomentum {
        index: usize,
    },
    NonFiniteHamiltonianDerivative {
        index: usize,
    },
    NonFiniteHamiltonian,
    OutsideExteriorChart {
        radius_m: f64,
        horizon_m: f64,
    },
    OutsidePolarChart {
        theta_rad: f64,
    },
    NonFiniteAcceleration {
        index: usize,
    },
    NonFiniteNormalization,
    NonFiniteKillingConstant,
    StepSizeUnderflow {
        step_m: f64,
        minimum_step_m: f64,
    },
    MaximumAttemptsExceeded {
        accepted_steps: usize,
        rejected_steps: usize,
        advanced_parameter_m: f64,
        requested_parameter_m: f64,
    },
}

impl From<PhysicsDomainError> for GeodesicError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}
impl From<MetricError> for GeodesicError {
    fn from(value: MetricError) -> Self {
        Self::Metric(value)
    }
}
impl From<ConnectionError> for GeodesicError {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl fmt::Display for GeodesicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::Metric(error) => write!(f, "{error}"),
            Self::Connection(error) => write!(f, "{error}"),
            Self::InvalidStep(value) => {
                write!(f, "geodesic step {value} m must be finite and nonzero")
            }
            Self::InvalidPositiveParameter { name, value } => {
                write!(f, "{name} must be finite and positive, got {value}")
            }
            Self::ZeroMaximumAttempts => write!(f, "maximum adaptive attempts must be nonzero"),
            Self::InvalidTolerance {
                group,
                index,
                value,
            } => {
                write!(f, "invalid {group} tolerance at {index:?}: {value}")
            }
            Self::NonFiniteCoordinate { index } => {
                write!(f, "geodesic coordinate x[{index}] is non-finite")
            }
            Self::NonFiniteTangent { index } => {
                write!(f, "geodesic tangent u[{index}] is non-finite")
            }
            Self::NonFiniteCanonicalMomentum { index } => {
                write!(f, "canonical momentum p[{index}] is non-finite")
            }
            Self::NonFiniteHamiltonianDerivative { index } => {
                write!(f, "Hamiltonian derivative component {index} is non-finite")
            }
            Self::NonFiniteHamiltonian => write!(f, "Hamiltonian value is non-finite"),
            Self::OutsideExteriorChart {
                radius_m,
                horizon_m,
            } => write!(
                f,
                "Schwarzschild chart requires r > r_s; received r={radius_m} m, r_s={horizon_m} m"
            ),
            Self::OutsidePolarChart { theta_rad } => {
                write!(
                    f,
                    "Schwarzschild spherical chart requires 0 < theta < pi, got {theta_rad}"
                )
            }
            Self::NonFiniteAcceleration { index } => {
                write!(f, "geodesic acceleration component {index} is non-finite")
            }
            Self::NonFiniteNormalization => write!(f, "geodesic normalization is non-finite"),
            Self::NonFiniteKillingConstant => write!(f, "geodesic Killing constant is non-finite"),
            Self::StepSizeUnderflow {
                step_m,
                minimum_step_m,
            } => write!(
                f,
                "adaptive geodesic step {step_m} m fell below minimum {minimum_step_m} m"
            ),
            Self::MaximumAttemptsExceeded {
                accepted_steps,
                rejected_steps,
                advanced_parameter_m,
                requested_parameter_m,
            } => write!(
                f,
                "adaptive integration exhausted attempts after {accepted_steps} accepted / {rejected_steps} rejected steps; advanced {advanced_parameter_m} of {requested_parameter_m} m"
            ),
        }
    }
}
impl std::error::Error for GeodesicError {}

#[cfg(test)]
mod tests {
    use super::super::MassKg;
    use super::*;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn integrator() -> SchwarzschildGeodesicIntegrator {
        SchwarzschildGeodesicIntegrator::new(SchwarzschildBlackHole::new(
            MassKg::new(SOLAR_MASS_KG).unwrap(),
        ))
    }

    fn circular_timelike_state(
        integrator: SchwarzschildGeodesicIntegrator,
        radius_multiple: f64,
    ) -> GeodesicState {
        let rg = integrator.black_hole().gravitational_radius_m();
        let radius = radius_multiple * rg;
        let u0 = 1.0 / (1.0 - 3.0 / radius_multiple).sqrt();
        let uphi = (1.0 / (radius_multiple.powi(3) * rg * rg)).sqrt()
            / (1.0 - 3.0 / radius_multiple).sqrt();

        GeodesicState::new(
            [0.0, radius, core::f64::consts::FRAC_PI_2, 0.0],
            [u0, 0.0, 0.0, uphi],
        )
    }

    #[test]
    fn circular_timelike_orbit_preserves_norm_energy_and_angular_momentum() {
        let integrator = integrator();
        let rg = integrator.black_hole().gravitational_radius_m();
        let initial = circular_timelike_state(integrator, 6.0);
        assert!(
            integrator
                .normalization_residual(initial, GeodesicParameterization::TimelikeProperLength)
                .unwrap()
                < 2e-14
        );

        let initial_constants = integrator.killing_constants(initial).unwrap();
        let states = integrator.integrate_fixed(initial, 0.05 * rg, 200).unwrap();
        let final_state = *states.last().unwrap();
        let final_constants = integrator.killing_constants(final_state).unwrap();

        assert!((final_state.coordinates[1] - initial.coordinates[1]).abs() < 2e-12 * rg);
        assert!((final_state.coordinates[2] - initial.coordinates[2]).abs() < 2e-13);
        assert!(
            integrator
                .normalization_residual(final_state, GeodesicParameterization::TimelikeProperLength)
                .unwrap()
                < 2e-12
        );
        assert!((final_constants.energy - initial_constants.energy).abs() < 2e-12);
        assert!(
            (final_constants.angular_momentum_z_m - initial_constants.angular_momentum_z_m).abs()
                < 2e-12 * rg
        );
    }

    #[test]
    fn radial_null_geodesic_preserves_null_normalization_and_killing_energy() {
        let integrator = integrator();
        let rg = integrator.black_hole().gravitational_radius_m();
        let radius = 8.0 * rg;
        let f = 1.0 - 2.0 * rg / radius;
        let initial = GeodesicState::new(
            [0.0, radius, core::f64::consts::FRAC_PI_2, 0.0],
            [1.0 / f, 1.0, 0.0, 0.0],
        );
        assert!(
            integrator
                .normalization_residual(initial, GeodesicParameterization::NullAffine)
                .unwrap()
                < 2e-14
        );
        let e0 = integrator.killing_constants(initial).unwrap().energy;
        let states = integrator.integrate_fixed(initial, 0.02 * rg, 100).unwrap();
        let final_state = *states.last().unwrap();
        assert!(
            integrator
                .normalization_residual(final_state, GeodesicParameterization::NullAffine)
                .unwrap()
                < 2e-10
        );
        let e1 = integrator.killing_constants(final_state).unwrap().energy;
        assert!((e1 - e0).abs() < 2e-10);
        assert!(final_state.coordinates[1] > radius);
    }

    #[test]
    fn adaptive_rk4_step_doubling_tracks_analytic_circular_orbit() {
        let integrator = integrator();
        let rg = integrator.black_hole().gravitational_radius_m();
        let initial = circular_timelike_state(integrator, 6.0);
        let span = 10.0 * rg;
        let tolerances = GeodesicTolerances::new(
            1e-10,
            [1e-8 * rg, 1e-8 * rg, 1e-10, 1e-10],
            [1e-10, 1e-10, 1e-12 / rg, 1e-12 / rg],
        )
        .unwrap();
        let result = integrator
            .integrate_adaptive(initial, span, rg, tolerances, 2000)
            .unwrap();
        let final_state = *result.states.last().unwrap();

        let u0 = initial.tangent[0];
        let uphi = initial.tangent[3];
        assert!((final_state.coordinates[0] - u0 * span).abs() < 2e-9 * rg);
        assert!((final_state.coordinates[3] - uphi * span).abs() < 2e-11);
        assert!((final_state.coordinates[1] - initial.coordinates[1]).abs() < 2e-10 * rg);
        assert!(result.accepted_steps > 0);
        assert_eq!(result.states.len(), result.accepted_steps + 1);
    }

    #[test]
    fn proper_length_parameter_is_not_coordinate_time() {
        let integrator = integrator();
        let state = circular_timelike_state(integrator, 6.0);
        assert!(state.tangent[0] > 1.0);
        assert!((integrator.normalization(state).unwrap() + 1.0).abs() < 2e-14);
        // For a circular Schwarzschild geodesic, dx^0/d(c*tau)=dt/dtau > 1.
        // Coordinate time is x^0/c, not the integration parameter.
    }

    #[test]
    fn integrator_rejects_horizon_axis_and_invalid_steps() {
        let integrator = integrator();
        let hole = integrator.black_hole();
        let horizon_state = GeodesicState::new(
            [0.0, hole.horizon_radius_m(), 1.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        );
        assert!(matches!(
            integrator.step_rk4(horizon_state, 1.0),
            Err(GeodesicError::OutsideExteriorChart { .. })
        ));

        let axis_state = GeodesicState::new(
            [0.0, 4.0 * hole.gravitational_radius_m(), 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        );
        assert!(matches!(
            integrator.step_rk4(axis_state, 1.0),
            Err(GeodesicError::OutsidePolarChart { .. })
        ));
        let valid = circular_timelike_state(integrator, 6.0);
        assert!(matches!(
            integrator.step_rk4(valid, 0.0),
            Err(GeodesicError::InvalidStep(0.0))
        ));
    }

    #[test]
    fn canonical_conversion_preserves_timelike_state_and_hamiltonian() {
        let integrator = integrator();
        let initial = circular_timelike_state(integrator, 6.0);
        let canonical = integrator.to_hamiltonian_state(initial).unwrap();
        let round_trip = integrator.from_hamiltonian_state(canonical).unwrap();

        for index in 0..4 {
            assert!((round_trip.coordinates[index] - initial.coordinates[index]).abs() < 1e-12);
            assert!((round_trip.tangent[index] - initial.tangent[index]).abs() < 2e-14);
        }
        assert!((integrator.hamiltonian_value(canonical).unwrap() + 0.5).abs() < 2e-14);
    }

    #[test]
    fn hamiltonian_and_christoffel_formulations_agree_for_circular_orbit() {
        let integrator = integrator();
        let rg = integrator.black_hole().gravitational_radius_m();
        let initial = circular_timelike_state(integrator, 6.0);
        let h0 = integrator.to_hamiltonian_state(initial).unwrap();

        let coordinate_states = integrator.integrate_fixed(initial, 0.05 * rg, 200).unwrap();
        let hamiltonian_states = integrator
            .integrate_hamiltonian_fixed(h0, 0.05 * rg, 200)
            .unwrap();
        let coordinate_final = *coordinate_states.last().unwrap();
        let hamiltonian_final = integrator
            .from_hamiltonian_state(*hamiltonian_states.last().unwrap())
            .unwrap();

        for index in 0..4 {
            let scale = if index < 2 { rg } else { 1.0 };
            assert!(
                (coordinate_final.coordinates[index] - hamiltonian_final.coordinates[index]).abs()
                    < 2e-10 * scale
            );
            let tangent_absolute_tolerance = if index < 2 { 2e-10 } else { 2e-10 / rg };
            assert!(
                (coordinate_final.tangent[index] - hamiltonian_final.tangent[index]).abs()
                    < tangent_absolute_tolerance
            );
        }
        assert!(
            (integrator
                .hamiltonian_value(*hamiltonian_states.last().unwrap())
                .unwrap()
                + 0.5)
                .abs()
                < 2e-10
        );
    }

    #[test]
    fn hamiltonian_and_christoffel_formulations_agree_for_radial_null_ray() {
        let integrator = integrator();
        let rg = integrator.black_hole().gravitational_radius_m();
        let radius = 8.0 * rg;
        let f = 1.0 - 2.0 * rg / radius;
        let initial = GeodesicState::new(
            [0.0, radius, core::f64::consts::FRAC_PI_2, 0.0],
            [1.0 / f, 1.0, 0.0, 0.0],
        );
        let canonical = integrator.to_hamiltonian_state(initial).unwrap();
        let coordinate_final = *integrator
            .integrate_fixed(initial, 0.02 * rg, 100)
            .unwrap()
            .last()
            .unwrap();
        let hamiltonian_states = integrator
            .integrate_hamiltonian_fixed(canonical, 0.02 * rg, 100)
            .unwrap();
        let canonical_final = *hamiltonian_states.last().unwrap();
        let hamiltonian_final = integrator.from_hamiltonian_state(canonical_final).unwrap();

        for index in 0..4 {
            let coordinate_scale = if index < 2 { rg } else { 1.0 };
            assert!(
                (coordinate_final.coordinates[index] - hamiltonian_final.coordinates[index]).abs()
                    < 2e-10 * coordinate_scale
            );
            let tangent_scale = if index < 2 { 1.0 } else { 1.0 / rg };
            assert!(
                (coordinate_final.tangent[index] - hamiltonian_final.tangent[index]).abs()
                    < 2e-10 * tangent_scale
            );
        }
        // Time-translation and axial-rotation symmetries imply constant p_t
        // and p_phi in this stationary, axisymmetric background.
        assert!(
            (canonical_final.canonical_momenta[0] - canonical.canonical_momenta[0]).abs() < 2e-10
        );
        assert!(
            (canonical_final.canonical_momenta[3] - canonical.canonical_momenta[3]).abs()
                < 2e-10 * rg
        );
        assert!(integrator.hamiltonian_value(canonical_final).unwrap().abs() < 2e-10);
    }

    #[test]
    fn adaptive_integrator_rejects_nonpositive_component_tolerances() {
        assert!(matches!(
            GeodesicTolerances::new(1e-8, [1e-8, 1e-8, 1e-10, 1e-10], [1e-8, 1e-8, 1e-12, 0.0],),
            Err(GeodesicError::InvalidTolerance {
                group: "tangent_absolute",
                index: Some(3),
                ..
            })
        ));
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Transparent CPU continuum-fluid reference solver.
//!
//! This module is a numerical validation instrument, not the production water
//! model. It implements a deliberately small two-dimensional periodic MAC grid
//! with donor-cell upwind advection, explicit viscosity, and a fixed-iteration
//! Jacobi pressure projection. Its purpose is to establish smooth/control rungs
//! for the continuum validation ladder before any singular-flow campaign.

use std::fmt;

use crate::validation::{
    CONTINUUM_DIAGNOSTIC_SCHEMA_VERSION, ContinuumDiagnosticSample, DiagnosticUnavailableReason,
    DiagnosticValueError, NonNegativeDiagnostic,
};

/// Semantic profile for the V0.1 CPU reference solver. The scheme identifiers
/// below are part of this profile's definition; changing one requires a profile
/// version change.
pub const PERIODIC_MAC2D_PROFILE_ID: &str = "periodic-mac2d-reference-v0.1";
pub const ADVECTION_SCHEME_ID: &str = "donor-cell-upwind-v0.1";
pub const DIFFUSION_SCHEME_ID: &str = "explicit-centered-laplacian-v0.1";
pub const PROJECTION_SCHEME_ID: &str = "periodic-jacobi-fixed-iterations-v0.1";
pub const DIVERGENCE_SCHEME_ID: &str = "mac-face-flux-divergence-v0.1";
pub const MAX_REFERENCE_CELLS: usize = 4_194_304;
pub const MAX_PRESSURE_ITERATIONS: usize = 100_000;
/// Sufficient monotonicity gate for the explicit donor-cell + diffusion update.
pub const MAX_COMBINED_EXPLICIT_NUMBER: f64 = 1.0;

/// Configuration of a two-dimensional periodic MAC-grid reference problem.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicMacConfig {
    pub nx: usize,
    pub ny: usize,
    pub length_x_m: f64,
    pub length_y_m: f64,
    /// Physical thickness used only to turn the 2D slice's energy integral into
    /// joules. It does not make this solver a 3D flow model.
    pub slab_depth_m: f64,
    pub density_kg_m3: f64,
    pub kinematic_viscosity_m2_s: f64,
    /// Fixed count preserves transparent, deterministic iteration semantics.
    pub pressure_iterations: usize,
    /// Conservative admission limit for the donor-cell explicit step.
    pub max_advective_cfl: f64,
    /// Limit for `nu * dt * (1/dx^2 + 1/dy^2)` in the explicit diffusion step.
    pub max_diffusion_number: f64,
}

impl Default for PeriodicMacConfig {
    fn default() -> Self {
        Self {
            nx: 32,
            ny: 32,
            length_x_m: std::f64::consts::TAU,
            length_y_m: std::f64::consts::TAU,
            slab_depth_m: 1.0,
            density_kg_m3: 1.0,
            kinematic_viscosity_m2_s: 0.01,
            pressure_iterations: 400,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }
}

impl PeriodicMacConfig {
    pub fn validate(&self) -> Result<(), ReferenceConfigError> {
        if self.nx < 4 || self.ny < 4 {
            return Err(ReferenceConfigError::GridTooSmall);
        }
        let cells = self
            .nx
            .checked_mul(self.ny)
            .ok_or(ReferenceConfigError::GridTooLarge)?;
        if cells > MAX_REFERENCE_CELLS {
            return Err(ReferenceConfigError::GridTooLarge);
        }
        for (name, value) in [
            ("length_x_m", self.length_x_m),
            ("length_y_m", self.length_y_m),
            ("slab_depth_m", self.slab_depth_m),
            ("density_kg_m3", self.density_kg_m3),
            ("max_advective_cfl", self.max_advective_cfl),
            ("max_diffusion_number", self.max_diffusion_number),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ReferenceConfigError::ExpectedPositiveFinite(name));
            }
        }
        if self.max_advective_cfl > MAX_COMBINED_EXPLICIT_NUMBER {
            return Err(ReferenceConfigError::AdvectiveLimitTooHigh);
        }
        if self.max_diffusion_number > 0.5 {
            return Err(ReferenceConfigError::DiffusionLimitTooHigh);
        }
        if !self.kinematic_viscosity_m2_s.is_finite() || self.kinematic_viscosity_m2_s < 0.0 {
            return Err(ReferenceConfigError::ExpectedNonNegativeFinite(
                "kinematic_viscosity_m2_s",
            ));
        }
        if self.pressure_iterations == 0 || self.pressure_iterations > MAX_PRESSURE_ITERATIONS {
            return Err(ReferenceConfigError::InvalidPressureIterations);
        }
        Ok(())
    }

    pub fn dx(&self) -> f64 {
        self.length_x_m / self.nx as f64
    }

    pub fn dy(&self) -> f64 {
        self.length_y_m / self.ny as f64
    }

    /// Stable compact identity for the variable numerical assumptions of this
    /// exact config. Scheme choices are frozen by [`PERIODIC_MAC2D_PROFILE_ID`].
    /// Floating-point values use IEEE-754 bits, not display rounding.
    pub fn profile_identity(&self) -> String {
        format!(
            "{PERIODIC_MAC2D_PROFILE_ID};n={}x{};L={:016x},{:016x};d={:016x};r={:016x};v={:016x};i={};c={:016x};q={:016x}",
            self.nx,
            self.ny,
            self.length_x_m.to_bits(),
            self.length_y_m.to_bits(),
            self.slab_depth_m.to_bits(),
            self.density_kg_m3.to_bits(),
            self.kinematic_viscosity_m2_s.to_bits(),
            self.pressure_iterations,
            self.max_advective_cfl.to_bits(),
            self.max_diffusion_number.to_bits(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceConfigError {
    GridTooSmall,
    GridTooLarge,
    ExpectedPositiveFinite(&'static str),
    ExpectedNonNegativeFinite(&'static str),
    AdvectiveLimitTooHigh,
    DiffusionLimitTooHigh,
    InvalidPressureIterations,
}

impl fmt::Display for ReferenceConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridTooSmall => write!(f, "reference grid requires nx, ny >= 4"),
            Self::GridTooLarge => write!(f, "reference grid exceeds the V0 cell bound"),
            Self::ExpectedPositiveFinite(name) => write!(f, "{name} must be finite and > 0"),
            Self::ExpectedNonNegativeFinite(name) => {
                write!(f, "{name} must be finite and >= 0")
            }
            Self::AdvectiveLimitTooHigh => write!(f, "max_advective_cfl must be <= 1"),
            Self::DiffusionLimitTooHigh => write!(f, "max_diffusion_number must be <= 0.5"),
            Self::InvalidPressureIterations => write!(
                f,
                "pressure_iterations must be between 1 and {MAX_PRESSURE_ITERATIONS}"
            ),
        }
    }
}

impl std::error::Error for ReferenceConfigError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionReport {
    pub divergence_rms_before_per_s: f64,
    pub divergence_rms_after_per_s: f64,
    pub pressure_residual_rms_pa_per_m2: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceStepReport {
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub dt_s: f64,
    /// Maximum of the admitted pre-step and post-predictor advective CFL.
    pub max_advective_cfl: f64,
    pub diffusion_number: f64,
    /// Maximum of the pre-step and predictor `C + 2r` quantities.
    pub combined_explicit_number: f64,
    pub projection: ProjectionReport,
    pub non_finite_state_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReferenceStepError {
    InvalidDt,
    CflLimitExceeded { observed: f64, limit: f64 },
    DiffusionLimitExceeded { observed: f64, limit: f64 },
    CombinedExplicitLimitExceeded { observed: f64 },
    NonFiniteAcceleration { component: &'static str },
    NonFinitePredictor { component: &'static str },
    PredictorCflLimitExceeded { observed: f64, limit: f64 },
    PredictorCombinedExplicitLimitExceeded { observed: f64 },
    ProjectionScaleNonFinite,
    NonFiniteProjection,
}

impl fmt::Display for ReferenceStepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDt => write!(f, "dt must be finite and > 0"),
            Self::CflLimitExceeded { observed, limit } => {
                write!(
                    f,
                    "advective CFL {observed} exceeds configured limit {limit}"
                )
            }
            Self::DiffusionLimitExceeded { observed, limit } => write!(
                f,
                "explicit diffusion number {observed} exceeds configured limit {limit}"
            ),
            Self::CombinedExplicitLimitExceeded { observed } => write!(
                f,
                "combined explicit number {observed} exceeds monotonicity limit {MAX_COMBINED_EXPLICIT_NUMBER}"
            ),
            Self::NonFiniteAcceleration { component } => {
                write!(f, "forcing returned non-finite {component} acceleration")
            }
            Self::NonFinitePredictor { component } => {
                write!(
                    f,
                    "explicit predictor produced non-finite {component} velocity"
                )
            }
            Self::PredictorCflLimitExceeded { observed, limit } => write!(
                f,
                "post-predictor advective CFL {observed} exceeds configured limit {limit}"
            ),
            Self::PredictorCombinedExplicitLimitExceeded { observed } => write!(
                f,
                "post-predictor combined explicit number {observed} exceeds monotonicity limit {MAX_COMBINED_EXPLICIT_NUMBER}"
            ),
            Self::ProjectionScaleNonFinite => write!(
                f,
                "pressure projection scaling is non-finite for the supplied dt/density"
            ),
            Self::NonFiniteProjection => {
                write!(
                    f,
                    "pressure projection produced non-finite state or diagnostics"
                )
            }
        }
    }
}

impl std::error::Error for ReferenceStepError {}

#[derive(Clone, Debug, PartialEq)]
pub enum ReferenceStateError {
    Config(ReferenceConfigError),
    WrongFaceCount,
    NonFiniteFaceVelocity,
    InvalidTaylorGreenAmplitude,
}

impl fmt::Display for ReferenceStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(source) => write!(f, "invalid reference configuration: {source}"),
            Self::WrongFaceCount => write!(f, "u/v face arrays must each contain nx * ny values"),
            Self::NonFiniteFaceVelocity => write!(f, "face velocity arrays must be finite"),
            Self::InvalidTaylorGreenAmplitude => {
                write!(f, "Taylor-Green amplitude must be finite")
            }
        }
    }
}

impl std::error::Error for ReferenceStateError {}

impl From<ReferenceConfigError> for ReferenceStateError {
    fn from(value: ReferenceConfigError) -> Self {
        Self::Config(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReferenceDiagnosticError {
    InvalidMetric {
        name: &'static str,
        source: DiagnosticValueError,
    },
}

impl fmt::Display for ReferenceDiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetric { name, source } => write!(f, "invalid {name}: {source}"),
        }
    }
}

impl std::error::Error for ReferenceDiagnosticError {}

#[derive(Clone, Debug)]
struct ProjectionCandidate {
    u_faces: Vec<f64>,
    v_faces: Vec<f64>,
    pressure_pa: Vec<f64>,
    report: ProjectionReport,
}

/// Two-dimensional periodic staggered-grid state.
///
/// Both face arrays contain `nx * ny` values. `u(i,j)` is the x-velocity at
/// `(i*dx, (j+1/2)*dy)` and `v(i,j)` is the y-velocity at
/// `((i+1/2)*dx, j*dy)`. Pressure is cell-centered.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicMac2d {
    config: PeriodicMacConfig,
    u_faces: Vec<f64>,
    v_faces: Vec<f64>,
    pressure_pa: Vec<f64>,
    time_s: f64,
    last_step: Option<ReferenceStepReport>,
}

impl PeriodicMac2d {
    pub fn zeros(config: PeriodicMacConfig) -> Result<Self, ReferenceStateError> {
        config.validate()?;
        let cells = config.nx * config.ny;
        Ok(Self {
            config,
            u_faces: vec![0.0; cells],
            v_faces: vec![0.0; cells],
            pressure_pa: vec![0.0; cells],
            time_s: 0.0,
            last_step: None,
        })
    }

    pub fn uniform(
        config: PeriodicMacConfig,
        u_mps: f64,
        v_mps: f64,
    ) -> Result<Self, ReferenceStateError> {
        if !u_mps.is_finite() || !v_mps.is_finite() {
            return Err(ReferenceStateError::NonFiniteFaceVelocity);
        }
        let mut state = Self::zeros(config)?;
        state.u_faces.fill(u_mps);
        state.v_faces.fill(v_mps);
        Ok(state)
    }

    pub fn from_faces(
        config: PeriodicMacConfig,
        u_faces: Vec<f64>,
        v_faces: Vec<f64>,
    ) -> Result<Self, ReferenceStateError> {
        config.validate()?;
        let cells = config.nx * config.ny;
        if u_faces.len() != cells || v_faces.len() != cells {
            return Err(ReferenceStateError::WrongFaceCount);
        }
        if u_faces
            .iter()
            .chain(&v_faces)
            .any(|value| !value.is_finite())
        {
            return Err(ReferenceStateError::NonFiniteFaceVelocity);
        }
        Ok(Self {
            config,
            u_faces,
            v_faces,
            pressure_pa: vec![0.0; cells],
            time_s: 0.0,
            last_step: None,
        })
    }

    /// A Taylor-Green-like periodic mode sampled directly on MAC faces.
    ///
    /// The y-velocity amplitude uses the ratio of the two discrete centered
    /// derivative symbols. This makes the initializer divergence-free under the
    /// exact MAC divergence operator used by this profile, including rectangular
    /// grids/domains, while approaching the analytic `kx/ky` ratio with grid
    /// refinement.
    pub fn taylor_green(
        config: PeriodicMacConfig,
        amplitude_mps: f64,
    ) -> Result<Self, ReferenceStateError> {
        if !amplitude_mps.is_finite() {
            return Err(ReferenceStateError::InvalidTaylorGreenAmplitude);
        }
        config.validate()?;
        let mut u_faces = vec![0.0; config.nx * config.ny];
        let mut v_faces = vec![0.0; config.nx * config.ny];
        let dx = config.dx();
        let dy = config.dy();
        let kx = std::f64::consts::TAU / config.length_x_m;
        let ky = std::f64::consts::TAU / config.length_y_m;
        let discrete_kx = 2.0 * (0.5 * kx * dx).sin() / dx;
        let discrete_ky = 2.0 * (0.5 * ky * dy).sin() / dy;
        let v_amplitude = amplitude_mps * discrete_kx / discrete_ky;

        for j in 0..config.ny {
            for i in 0..config.nx {
                let index = j * config.nx + i;
                let xu = i as f64 * dx;
                let yu = (j as f64 + 0.5) * dy;
                let xv = (i as f64 + 0.5) * dx;
                let yv = j as f64 * dy;
                u_faces[index] = amplitude_mps * (kx * xu).sin() * (ky * yu).cos();
                v_faces[index] = -v_amplitude * (kx * xv).cos() * (ky * yv).sin();
            }
        }

        Self::from_faces(config, u_faces, v_faces)
    }

    pub fn config(&self) -> &PeriodicMacConfig {
        &self.config
    }

    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    pub fn u_faces(&self) -> &[f64] {
        &self.u_faces
    }

    pub fn v_faces(&self) -> &[f64] {
        &self.v_faces
    }

    pub fn pressure_pa(&self) -> &[f64] {
        &self.pressure_pa
    }

    pub fn last_step(&self) -> Option<&ReferenceStepReport> {
        self.last_step.as_ref()
    }

    pub fn divergence_rms_per_s(&self) -> f64 {
        rms(&self.divergence_field_for(&self.u_faces, &self.v_faces))
    }

    /// Project the current velocity field transactionally. On every error the
    /// complete state is unchanged.
    pub fn project_velocity(&mut self, dt_s: f64) -> Result<ProjectionReport, ReferenceStepError> {
        let candidate = self.project_candidate(&self.u_faces, &self.v_faces, dt_s)?;
        self.u_faces = candidate.u_faces;
        self.v_faces = candidate.v_faces;
        self.pressure_pa = candidate.pressure_pa;
        Ok(candidate.report)
    }

    pub fn step(&mut self, dt_s: f64) -> Result<ReferenceStepReport, ReferenceStepError> {
        self.step_with_acceleration(dt_s, |_, _| [0.0, 0.0])
    }

    /// Advance one explicit predictor + pressure-projection step.
    ///
    /// `acceleration(position_m, time_s)` returns body acceleration in m/s^2 at
    /// the queried face position. It is intentionally not called a generic
    /// "force" to avoid a hidden density convention. The callback is `Fn`
    /// rather than `FnMut` so evaluation order cannot change forcing state.
    ///
    /// The entire step is transactional: predictor and projection are completed
    /// and validated in temporary buffers before velocity, pressure, time, or
    /// the last-step report are committed.
    pub fn step_with_acceleration<F>(
        &mut self,
        dt_s: f64,
        acceleration: F,
    ) -> Result<ReferenceStepReport, ReferenceStepError>
    where
        F: Fn([f64; 2], f64) -> [f64; 2],
    {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(ReferenceStepError::InvalidDt);
        }

        let dx = self.config.dx();
        let dy = self.config.dy();
        let pre_cfl = advective_cfl(&self.u_faces, &self.v_faces, dt_s, dx, dy);
        if pre_cfl > self.config.max_advective_cfl {
            return Err(ReferenceStepError::CflLimitExceeded {
                observed: pre_cfl,
                limit: self.config.max_advective_cfl,
            });
        }

        let diffusion_number =
            self.config.kinematic_viscosity_m2_s * dt_s * (1.0 / dx.powi(2) + 1.0 / dy.powi(2));
        if !diffusion_number.is_finite() {
            return Err(ReferenceStepError::NonFinitePredictor {
                component: "diffusion_number",
            });
        }
        if diffusion_number > self.config.max_diffusion_number {
            return Err(ReferenceStepError::DiffusionLimitExceeded {
                observed: diffusion_number,
                limit: self.config.max_diffusion_number,
            });
        }
        let pre_combined = pre_cfl + 2.0 * diffusion_number;
        if pre_combined > MAX_COMBINED_EXPLICIT_NUMBER {
            return Err(ReferenceStepError::CombinedExplicitLimitExceeded {
                observed: pre_combined,
            });
        }

        let mut next_u = vec![0.0; self.u_faces.len()];
        let mut next_v = vec![0.0; self.v_faces.len()];
        let nu = self.config.kinematic_viscosity_m2_s;

        for j in 0..self.config.ny {
            let south = previous(j, self.config.ny);
            let north = next(j, self.config.ny);
            for i in 0..self.config.nx {
                let west = previous(i, self.config.nx);
                let east = next(i, self.config.nx);
                let index = self.index(i, j);

                let u = self.u_faces[index];
                let v_at_u = 0.25
                    * (self.v_faces[self.index(west, j)]
                        + self.v_faces[index]
                        + self.v_faces[self.index(west, north)]
                        + self.v_faces[self.index(i, north)]);
                let du_dx = if u >= 0.0 {
                    (u - self.u_faces[self.index(west, j)]) / dx
                } else {
                    (self.u_faces[self.index(east, j)] - u) / dx
                };
                let du_dy = if v_at_u >= 0.0 {
                    (u - self.u_faces[self.index(i, south)]) / dy
                } else {
                    (self.u_faces[self.index(i, north)] - u) / dy
                };
                let lap_u = (self.u_faces[self.index(east, j)] - 2.0 * u
                    + self.u_faces[self.index(west, j)])
                    / dx.powi(2)
                    + (self.u_faces[self.index(i, north)] - 2.0 * u
                        + self.u_faces[self.index(i, south)])
                        / dy.powi(2);
                let position = [i as f64 * dx, (j as f64 + 0.5) * dy];
                let a = acceleration(position, self.time_s);
                if !a[0].is_finite() {
                    return Err(ReferenceStepError::NonFiniteAcceleration { component: "x" });
                }
                let predicted_u = u + dt_s * (-u * du_dx - v_at_u * du_dy + nu * lap_u + a[0]);
                if !predicted_u.is_finite() {
                    return Err(ReferenceStepError::NonFinitePredictor { component: "u" });
                }
                next_u[index] = predicted_u;

                let v = self.v_faces[index];
                let u_at_v = 0.25
                    * (self.u_faces[self.index(i, south)]
                        + self.u_faces[self.index(east, south)]
                        + self.u_faces[index]
                        + self.u_faces[self.index(east, j)]);
                let dv_dx = if u_at_v >= 0.0 {
                    (v - self.v_faces[self.index(west, j)]) / dx
                } else {
                    (self.v_faces[self.index(east, j)] - v) / dx
                };
                let dv_dy = if v >= 0.0 {
                    (v - self.v_faces[self.index(i, south)]) / dy
                } else {
                    (self.v_faces[self.index(i, north)] - v) / dy
                };
                let lap_v = (self.v_faces[self.index(east, j)] - 2.0 * v
                    + self.v_faces[self.index(west, j)])
                    / dx.powi(2)
                    + (self.v_faces[self.index(i, north)] - 2.0 * v
                        + self.v_faces[self.index(i, south)])
                        / dy.powi(2);
                let position = [(i as f64 + 0.5) * dx, j as f64 * dy];
                let a = acceleration(position, self.time_s);
                if !a[1].is_finite() {
                    return Err(ReferenceStepError::NonFiniteAcceleration { component: "y" });
                }
                let predicted_v = v + dt_s * (-u_at_v * dv_dx - v * dv_dy + nu * lap_v + a[1]);
                if !predicted_v.is_finite() {
                    return Err(ReferenceStepError::NonFinitePredictor { component: "v" });
                }
                next_v[index] = predicted_v;
            }
        }

        let predictor_cfl = advective_cfl(&next_u, &next_v, dt_s, dx, dy);
        if !predictor_cfl.is_finite() {
            return Err(ReferenceStepError::NonFinitePredictor { component: "cfl" });
        }
        if predictor_cfl > self.config.max_advective_cfl {
            return Err(ReferenceStepError::PredictorCflLimitExceeded {
                observed: predictor_cfl,
                limit: self.config.max_advective_cfl,
            });
        }
        let predictor_combined = predictor_cfl + 2.0 * diffusion_number;
        if predictor_combined > MAX_COMBINED_EXPLICIT_NUMBER {
            return Err(ReferenceStepError::PredictorCombinedExplicitLimitExceeded {
                observed: predictor_combined,
            });
        }

        let projection = self.project_candidate(&next_u, &next_v, dt_s)?;
        let start_time_s = self.time_s;
        let end_time_s = start_time_s + dt_s;
        if !end_time_s.is_finite() {
            return Err(ReferenceStepError::InvalidDt);
        }

        let max_advective_cfl = pre_cfl.max(predictor_cfl);
        let combined_explicit_number = pre_combined.max(predictor_combined);
        let report = ReferenceStepReport {
            start_time_s,
            end_time_s,
            dt_s,
            max_advective_cfl,
            diffusion_number,
            combined_explicit_number,
            projection: projection.report.clone(),
            non_finite_state_count: 0,
        };

        self.u_faces = projection.u_faces;
        self.v_faces = projection.v_faces;
        self.pressure_pa = projection.pressure_pa;
        self.time_s = end_time_s;
        self.last_step = Some(report.clone());
        Ok(report)
    }

    pub fn diagnostics(&self) -> Result<ContinuumDiagnosticSample, ReferenceDiagnosticError> {
        let (cell_u, cell_v) = self.cell_centered_velocity();
        let cell_volume = self.config.dx() * self.config.dy() * self.config.slab_depth_m;
        let kinetic_energy = cell_u
            .iter()
            .zip(&cell_v)
            .map(|(u, v)| 0.5 * self.config.density_kg_m3 * (u * u + v * v) * cell_volume)
            .sum::<f64>();
        let max_speed = cell_u
            .iter()
            .zip(&cell_v)
            .map(|(u, v)| (u * u + v * v).sqrt())
            .fold(0.0, f64::max);
        let (max_vorticity, max_strain) = self.vorticity_and_strain_max(&cell_u, &cell_v);
        let max_pressure_gradient = self.max_pressure_gradient_pa_per_m();
        let last_cfl = self
            .last_step
            .as_ref()
            .map(|report| report.max_advective_cfl);
        let last_residual = self
            .last_step
            .as_ref()
            .map(|report| report.projection.pressure_residual_rms_pa_per_m2);

        let sample = ContinuumDiagnosticSample {
            schema_version: CONTINUUM_DIAGNOSTIC_SCHEMA_VERSION,
            diagnostic_profile: self.config.profile_identity(),
            time_s: self.time_s,
            max_resolved_speed_mps: measured("max_resolved_speed_mps", max_speed)?,
            kinetic_energy_j: measured("kinetic_energy_j", kinetic_energy)?,
            divergence_rms_per_s: measured("divergence_rms_per_s", self.divergence_rms_per_s())?,
            max_vorticity_per_s: measured("max_vorticity_per_s", max_vorticity)?,
            max_strain_rate_per_s: measured("max_strain_rate_per_s", max_strain)?,
            max_pressure_gradient_pa_per_m: measured(
                "max_pressure_gradient_pa_per_m",
                max_pressure_gradient,
            )?,
            max_cfl: optional_measured("max_cfl", last_cfl)?,
            minimum_resolved_length_m: measured(
                "minimum_resolved_length_m",
                self.config.dx().min(self.config.dy()),
            )?,
            concentration_scale_m: NonNegativeDiagnostic::unavailable(
                DiagnosticUnavailableReason::NotApplicable,
            ),
            solver_residual: optional_measured("solver_residual", last_residual)?,
            forcing_residual: NonNegativeDiagnostic::unavailable(
                DiagnosticUnavailableReason::NotApplicable,
            ),
            non_finite_state_count: self.non_finite_state_count(),
        };
        Ok(sample)
    }

    fn project_candidate(
        &self,
        u_faces: &[f64],
        v_faces: &[f64],
        dt_s: f64,
    ) -> Result<ProjectionCandidate, ReferenceStepError> {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(ReferenceStepError::InvalidDt);
        }
        if u_faces
            .iter()
            .chain(v_faces)
            .any(|value| !value.is_finite())
        {
            return Err(ReferenceStepError::NonFiniteProjection);
        }

        let before = self.divergence_field_for(u_faces, v_faces);
        if before.iter().any(|value| !value.is_finite()) {
            return Err(ReferenceStepError::NonFiniteProjection);
        }
        let before_rms = rms(&before);
        let rho_over_dt = self.config.density_kg_m3 / dt_s;
        if !rho_over_dt.is_finite() {
            return Err(ReferenceStepError::ProjectionScaleNonFinite);
        }
        let mut rhs: Vec<f64> = before.iter().map(|value| rho_over_dt * value).collect();
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err(ReferenceStepError::NonFiniteProjection);
        }
        let rhs_mean = mean(&rhs);
        if !rhs_mean.is_finite() {
            return Err(ReferenceStepError::NonFiniteProjection);
        }
        for value in &mut rhs {
            *value -= rhs_mean;
        }

        let cells = self.config.nx * self.config.ny;
        let mut pressure_pa = vec![0.0; cells];
        let mut next_pressure = vec![0.0; cells];
        let inv_dx2 = 1.0 / self.config.dx().powi(2);
        let inv_dy2 = 1.0 / self.config.dy().powi(2);
        let denominator = 2.0 * (inv_dx2 + inv_dy2);
        if !inv_dx2.is_finite() || !inv_dy2.is_finite() || !denominator.is_finite() {
            return Err(ReferenceStepError::ProjectionScaleNonFinite);
        }

        for _ in 0..self.config.pressure_iterations {
            for j in 0..self.config.ny {
                let south = previous(j, self.config.ny);
                let north = next(j, self.config.ny);
                for i in 0..self.config.nx {
                    let west = previous(i, self.config.nx);
                    let east = next(i, self.config.nx);
                    let index = self.index(i, j);
                    let value = ((pressure_pa[self.index(east, j)]
                        + pressure_pa[self.index(west, j)])
                        * inv_dx2
                        + (pressure_pa[self.index(i, north)] + pressure_pa[self.index(i, south)])
                            * inv_dy2
                        - rhs[index])
                        / denominator;
                    if !value.is_finite() {
                        return Err(ReferenceStepError::NonFiniteProjection);
                    }
                    next_pressure[index] = value;
                }
            }
            std::mem::swap(&mut pressure_pa, &mut next_pressure);
        }

        let pressure_mean = mean(&pressure_pa);
        if !pressure_mean.is_finite() {
            return Err(ReferenceStepError::NonFiniteProjection);
        }
        for pressure in &mut pressure_pa {
            *pressure -= pressure_mean;
        }

        let pressure_residual = self.pressure_residual_rms_for(&pressure_pa, &rhs);
        if !pressure_residual.is_finite() {
            return Err(ReferenceStepError::NonFiniteProjection);
        }

        let dt_over_rho = dt_s / self.config.density_kg_m3;
        if !dt_over_rho.is_finite() {
            return Err(ReferenceStepError::ProjectionScaleNonFinite);
        }
        let dx = self.config.dx();
        let dy = self.config.dy();
        let mut projected_u = u_faces.to_vec();
        let mut projected_v = v_faces.to_vec();
        for j in 0..self.config.ny {
            let south = previous(j, self.config.ny);
            for i in 0..self.config.nx {
                let west = previous(i, self.config.nx);
                let index = self.index(i, j);
                let corrected_u = u_faces[index]
                    - dt_over_rho * (pressure_pa[index] - pressure_pa[self.index(west, j)]) / dx;
                let corrected_v = v_faces[index]
                    - dt_over_rho * (pressure_pa[index] - pressure_pa[self.index(i, south)]) / dy;
                if !corrected_u.is_finite() || !corrected_v.is_finite() {
                    return Err(ReferenceStepError::NonFiniteProjection);
                }
                projected_u[index] = corrected_u;
                projected_v[index] = corrected_v;
            }
        }

        let after = self.divergence_field_for(&projected_u, &projected_v);
        if after.iter().any(|value| !value.is_finite()) {
            return Err(ReferenceStepError::NonFiniteProjection);
        }
        let after_rms = rms(&after);
        if !before_rms.is_finite() || !after_rms.is_finite() {
            return Err(ReferenceStepError::NonFiniteProjection);
        }

        Ok(ProjectionCandidate {
            u_faces: projected_u,
            v_faces: projected_v,
            pressure_pa,
            report: ProjectionReport {
                divergence_rms_before_per_s: before_rms,
                divergence_rms_after_per_s: after_rms,
                pressure_residual_rms_pa_per_m2: pressure_residual,
            },
        })
    }

    fn index(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    fn divergence_field_for(&self, u_faces: &[f64], v_faces: &[f64]) -> Vec<f64> {
        let dx = self.config.dx();
        let dy = self.config.dy();
        let mut divergence = vec![0.0; u_faces.len()];
        for j in 0..self.config.ny {
            let north = next(j, self.config.ny);
            for i in 0..self.config.nx {
                let east = next(i, self.config.nx);
                let index = self.index(i, j);
                divergence[index] = (u_faces[self.index(east, j)] - u_faces[index]) / dx
                    + (v_faces[self.index(i, north)] - v_faces[index]) / dy;
            }
        }
        divergence
    }

    fn pressure_residual_rms_for(&self, pressure_pa: &[f64], rhs: &[f64]) -> f64 {
        let inv_dx2 = 1.0 / self.config.dx().powi(2);
        let inv_dy2 = 1.0 / self.config.dy().powi(2);
        let mut residual_squared = 0.0;
        for j in 0..self.config.ny {
            let south = previous(j, self.config.ny);
            let north = next(j, self.config.ny);
            for i in 0..self.config.nx {
                let west = previous(i, self.config.nx);
                let east = next(i, self.config.nx);
                let index = self.index(i, j);
                let laplacian = (pressure_pa[self.index(east, j)] - 2.0 * pressure_pa[index]
                    + pressure_pa[self.index(west, j)])
                    * inv_dx2
                    + (pressure_pa[self.index(i, north)] - 2.0 * pressure_pa[index]
                        + pressure_pa[self.index(i, south)])
                        * inv_dy2;
                let residual = laplacian - rhs[index];
                residual_squared += residual * residual;
            }
        }
        (residual_squared / pressure_pa.len() as f64).sqrt()
    }

    fn cell_centered_velocity(&self) -> (Vec<f64>, Vec<f64>) {
        let mut u = vec![0.0; self.u_faces.len()];
        let mut v = vec![0.0; self.v_faces.len()];
        for j in 0..self.config.ny {
            let north = next(j, self.config.ny);
            for i in 0..self.config.nx {
                let east = next(i, self.config.nx);
                let index = self.index(i, j);
                u[index] = 0.5 * (self.u_faces[index] + self.u_faces[self.index(east, j)]);
                v[index] = 0.5 * (self.v_faces[index] + self.v_faces[self.index(i, north)]);
            }
        }
        (u, v)
    }

    fn vorticity_and_strain_max(&self, u: &[f64], v: &[f64]) -> (f64, f64) {
        let dx2 = 2.0 * self.config.dx();
        let dy2 = 2.0 * self.config.dy();
        let mut max_vorticity = 0.0_f64;
        let mut max_strain = 0.0_f64;
        for j in 0..self.config.ny {
            let south = previous(j, self.config.ny);
            let north = next(j, self.config.ny);
            for i in 0..self.config.nx {
                let west = previous(i, self.config.nx);
                let east = next(i, self.config.nx);
                let du_dx = (u[self.index(east, j)] - u[self.index(west, j)]) / dx2;
                let du_dy = (u[self.index(i, north)] - u[self.index(i, south)]) / dy2;
                let dv_dx = (v[self.index(east, j)] - v[self.index(west, j)]) / dx2;
                let dv_dy = (v[self.index(i, north)] - v[self.index(i, south)]) / dy2;
                max_vorticity = max_vorticity.max((dv_dx - du_dy).abs());
                let shear = 0.5 * (du_dy + dv_dx);
                let strain_frobenius = (du_dx * du_dx + dv_dy * dv_dy + 2.0 * shear * shear).sqrt();
                max_strain = max_strain.max(strain_frobenius);
            }
        }
        (max_vorticity, max_strain)
    }

    fn max_pressure_gradient_pa_per_m(&self) -> f64 {
        let dx2 = 2.0 * self.config.dx();
        let dy2 = 2.0 * self.config.dy();
        let mut maximum = 0.0_f64;
        for j in 0..self.config.ny {
            let south = previous(j, self.config.ny);
            let north = next(j, self.config.ny);
            for i in 0..self.config.nx {
                let west = previous(i, self.config.nx);
                let east = next(i, self.config.nx);
                let dp_dx = (self.pressure_pa[self.index(east, j)]
                    - self.pressure_pa[self.index(west, j)])
                    / dx2;
                let dp_dy = (self.pressure_pa[self.index(i, north)]
                    - self.pressure_pa[self.index(i, south)])
                    / dy2;
                maximum = maximum.max((dp_dx * dp_dx + dp_dy * dp_dy).sqrt());
            }
        }
        maximum
    }

    fn non_finite_state_count(&self) -> u64 {
        self.u_faces
            .iter()
            .chain(&self.v_faces)
            .chain(&self.pressure_pa)
            .filter(|value| !value.is_finite())
            .count() as u64
    }
}

fn previous(index: usize, length: usize) -> usize {
    if index == 0 { length - 1 } else { index - 1 }
}

fn next(index: usize, length: usize) -> usize {
    if index + 1 == length { 0 } else { index + 1 }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn rms(values: &[f64]) -> f64 {
    (values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64).sqrt()
}

fn max_abs(values: &[f64]) -> f64 {
    values.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn advective_cfl(u_faces: &[f64], v_faces: &[f64], dt_s: f64, dx: f64, dy: f64) -> f64 {
    dt_s * (max_abs(u_faces) / dx + max_abs(v_faces) / dy)
}

fn measured(
    name: &'static str,
    value: f64,
) -> Result<NonNegativeDiagnostic, ReferenceDiagnosticError> {
    NonNegativeDiagnostic::measured(value)
        .map_err(|source| ReferenceDiagnosticError::InvalidMetric { name, source })
}

fn optional_measured(
    name: &'static str,
    value: Option<f64>,
) -> Result<NonNegativeDiagnostic, ReferenceDiagnosticError> {
    match value {
        Some(value) => measured(name, value),
        None => Ok(NonNegativeDiagnostic::unavailable(
            DiagnosticUnavailableReason::NotApplicable,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PeriodicMacConfig {
        PeriodicMacConfig {
            nx: 16,
            ny: 16,
            pressure_iterations: 600,
            ..PeriodicMacConfig::default()
        }
    }

    #[test]
    fn config_rejects_unbounded_or_nonphysical_inputs() {
        let mut config = test_config();
        config.nx = 1;
        assert_eq!(config.validate(), Err(ReferenceConfigError::GridTooSmall));

        let mut config = test_config();
        config.density_kg_m3 = 0.0;
        assert_eq!(
            config.validate(),
            Err(ReferenceConfigError::ExpectedPositiveFinite(
                "density_kg_m3"
            ))
        );
    }

    #[test]
    fn zero_flow_is_preserved_exactly() {
        let mut state = PeriodicMac2d::zeros(test_config()).unwrap();
        let before = state.clone();
        let report = state.step(0.01).unwrap();
        assert_eq!(state.u_faces(), before.u_faces());
        assert_eq!(state.v_faces(), before.v_faces());
        assert_eq!(report.projection.divergence_rms_after_per_s, 0.0);
        assert_eq!(state.time_s(), 0.01);
    }

    #[test]
    fn uniform_flow_is_preserved_exactly() {
        let mut state = PeriodicMac2d::uniform(test_config(), 0.75, -0.25).unwrap();
        let u = state.u_faces().to_vec();
        let v = state.v_faces().to_vec();
        state.step(0.01).unwrap();
        assert_eq!(state.u_faces(), u.as_slice());
        assert_eq!(state.v_faces(), v.as_slice());
    }

    #[test]
    fn constant_acceleration_produces_uniform_divergence_free_flow() {
        let mut config = test_config();
        config.kinematic_viscosity_m2_s = 0.0;
        let mut state = PeriodicMac2d::zeros(config).unwrap();
        state
            .step_with_acceleration(0.01, |_, _| [1.0, -2.0])
            .unwrap();
        assert!(
            state
                .u_faces()
                .iter()
                .all(|value| (*value - 0.01).abs() < 1.0e-15)
        );
        assert!(
            state
                .v_faces()
                .iter()
                .all(|value| (*value + 0.02).abs() < 1.0e-15)
        );
        assert!(state.divergence_rms_per_s() < 1.0e-14);
    }

    #[test]
    fn taylor_green_is_discretely_divergence_free() {
        let mut config = test_config();
        config.nx = 20;
        config.ny = 12;
        config.length_x_m = 7.0;
        config.length_y_m = 5.0;
        let state = PeriodicMac2d::taylor_green(config, 1.0).unwrap();
        assert!(state.divergence_rms_per_s() < 1.0e-13);
    }

    #[test]
    fn pressure_projection_strongly_reduces_a_periodic_divergence_mode() {
        let config = test_config();
        let dx = config.dx();
        let mut u = vec![0.0; config.nx * config.ny];
        let v = vec![0.0; config.nx * config.ny];
        for j in 0..config.ny {
            for i in 0..config.nx {
                u[j * config.nx + i] = (i as f64 * dx).sin();
            }
        }
        let mut state = PeriodicMac2d::from_faces(config, u, v).unwrap();
        let before = state.divergence_rms_per_s();
        let report = state.project_velocity(0.01).unwrap();
        assert!(before > 0.1);
        assert!(report.divergence_rms_after_per_s < before * 1.0e-5);
        assert!(report.pressure_residual_rms_pa_per_m2.is_finite());
    }

    #[test]
    fn cfl_rejection_occurs_before_state_mutation() {
        let mut state = PeriodicMac2d::uniform(test_config(), 1000.0, 0.0).unwrap();
        let before = state.clone();
        assert!(matches!(
            state.step(1.0),
            Err(ReferenceStepError::CflLimitExceeded { .. })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn acceleration_driven_predictor_cfl_rejection_is_transactional() {
        let mut config = test_config();
        config.kinematic_viscosity_m2_s = 0.0;
        let mut state = PeriodicMac2d::zeros(config).unwrap();
        let before = state.clone();
        assert!(matches!(
            state.step_with_acceleration(0.01, |_, _| [10_000.0, 0.0]),
            Err(ReferenceStepError::PredictorCflLimitExceeded { .. })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn finite_acceleration_that_overflows_predictor_is_transactional() {
        let mut config = test_config();
        config.kinematic_viscosity_m2_s = 0.0;
        config.max_advective_cfl = 1.0;
        let mut state = PeriodicMac2d::zeros(config).unwrap();
        let before = state.clone();
        assert!(matches!(
            state.step_with_acceleration(2.0, |_, _| [f64::MAX, 0.0]),
            Err(ReferenceStepError::NonFinitePredictor { .. })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn tiny_dt_projection_scale_rejection_is_transactional() {
        let config = test_config();
        let dx = config.dx();
        let mut u = vec![0.0; config.nx * config.ny];
        let v = vec![0.0; config.nx * config.ny];
        for j in 0..config.ny {
            for i in 0..config.nx {
                u[j * config.nx + i] = (i as f64 * dx).sin();
            }
        }
        let mut state = PeriodicMac2d::from_faces(config, u, v).unwrap();
        let before = state.clone();
        assert_eq!(
            state.project_velocity(1.0e-320),
            Err(ReferenceStepError::ProjectionScaleNonFinite)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn non_finite_acceleration_is_transactional() {
        let mut config = test_config();
        config.kinematic_viscosity_m2_s = 0.0;
        let mut state = PeriodicMac2d::zeros(config).unwrap();
        let before = state.clone();
        assert!(matches!(
            state.step_with_acceleration(0.01, |_, _| [f64::NAN, 0.0]),
            Err(ReferenceStepError::NonFiniteAcceleration { .. })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn taylor_green_short_step_emits_valid_diagnostics() {
        let mut state = PeriodicMac2d::taylor_green(test_config(), 0.5).unwrap();
        let report = state.step(0.005).unwrap();
        assert!(report.projection.divergence_rms_after_per_s.is_finite());
        let diagnostics = state.diagnostics().unwrap();
        assert_eq!(diagnostics.validate(), Ok(()));
        assert_eq!(diagnostics.non_finite_state_count, 0);
        assert!(diagnostics.max_resolved_speed_mps.measured_value().unwrap() > 0.0);
        assert!(diagnostics.kinetic_energy_j.measured_value().unwrap() > 0.0);
    }

    #[test]
    fn profile_identity_is_bounded_and_changes_with_numerical_semantics() {
        let a = test_config();
        assert!(a.profile_identity().len() <= crate::validation::MAX_DIAGNOSTIC_PROFILE_BYTES);

        let mut b = a.clone();
        b.pressure_iterations += 1;
        assert_ne!(a.profile_identity(), b.profile_identity());

        let mut c = a.clone();
        c.kinematic_viscosity_m2_s *= 2.0;
        assert_ne!(a.profile_identity(), c.profile_identity());
    }
}

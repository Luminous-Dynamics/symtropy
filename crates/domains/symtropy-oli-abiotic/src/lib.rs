// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic abiotic mechanism references for OLI.
//!
//! This crate supplies narrow, known-truth synthetic physical controls.
//! Pattern formation is not a life, reproduction, agency, intelligence, or
//! consciousness claim.

use std::error::Error;
use std::fmt;

pub const GENERATOR_ID: &str = "OLI-ABIOTIC-GRAY-SCOTT-001";
pub const EVIDENCE_CLASS: &str = "SYNTHETIC_KNOWN_TRUTH";
pub const DIFFUSION_COURANT_LIMIT_2D: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrayScottParameters {
    pub diffusion_u: f64,
    pub diffusion_v: f64,
    pub feed: f64,
    pub kill: f64,
    pub dt: f64,
}

impl GrayScottParameters {
    pub const fn reference_pattern() -> Self {
        Self {
            diffusion_u: 0.16,
            diffusion_v: 0.08,
            feed: 0.04,
            kill: 0.06,
            dt: 1.0,
        }
    }

    pub fn validate(self) -> Result<(), SimulationError> {
        for (name, value) in [
            ("diffusion_u", self.diffusion_u),
            ("diffusion_v", self.diffusion_v),
            ("feed", self.feed),
            ("kill", self.kill),
            ("dt", self.dt),
        ] {
            if !value.is_finite() {
                return Err(SimulationError::NonFiniteParameter(name));
            }
        }
        if self.diffusion_u < 0.0 {
            return Err(SimulationError::NegativeParameter("diffusion_u"));
        }
        if self.diffusion_v < 0.0 {
            return Err(SimulationError::NegativeParameter("diffusion_v"));
        }
        if self.feed < 0.0 {
            return Err(SimulationError::NegativeParameter("feed"));
        }
        if self.kill < 0.0 {
            return Err(SimulationError::NegativeParameter("kill"));
        }
        if self.dt <= 0.0 {
            return Err(SimulationError::NonPositiveParameter("dt"));
        }
        for (field, diffusion) in [
            ("U", self.diffusion_u),
            ("V", self.diffusion_v),
        ] {
            let courant = diffusion * self.dt;
            if courant > DIFFUSION_COURANT_LIMIT_2D {
                return Err(SimulationError::DiffusionStabilityViolation {
                    field,
                    courant,
                    maximum: DIFFUSION_COURANT_LIMIT_2D,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitialCondition {
    pub seed: u32,
    pub noise_amplitude: f64,
    pub patch_radius: usize,
    pub patch_u: f64,
    pub patch_v: f64,
}

impl Default for InitialCondition {
    fn default() -> Self {
        Self {
            seed: 20_261_110,
            noise_amplitude: 0.005,
            patch_radius: 4,
            patch_u: 0.5,
            patch_v: 0.25,
        }
    }
}

impl InitialCondition {
    fn validate(self, width: usize, height: usize) -> Result<(), SimulationError> {
        if !self.noise_amplitude.is_finite()
            || !self.patch_u.is_finite()
            || !self.patch_v.is_finite()
        {
            return Err(SimulationError::NonFiniteInitialCondition);
        }
        if !(0.0..=0.05).contains(&self.noise_amplitude) {
            return Err(SimulationError::InvalidInitialCondition(
                "noise_amplitude must be in [0, 0.05]",
            ));
        }
        if !(0.0..=1.0).contains(&self.patch_u)
            || !(0.0..=1.0).contains(&self.patch_v)
        {
            return Err(SimulationError::InvalidInitialCondition(
                "patch concentrations must be in [0, 1]",
            ));
        }
        if self.patch_radius == 0
            || self.patch_radius >= width / 2
            || self.patch_radius >= height / 2
        {
            return Err(SimulationError::InvalidInitialCondition(
                "patch_radius must be positive and smaller than half each grid dimension",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    GrayScott,
    DiffusionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldSummary {
    pub width: usize,
    pub height: usize,
    pub steps: usize,
    pub mean_u: f64,
    pub mean_v: f64,
    pub variance_u: f64,
    pub variance_v: f64,
    pub min_u: f64,
    pub max_u: f64,
    pub min_v: f64,
    pub max_v: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimulationError {
    InvalidGridDimensions,
    NonFiniteParameter(&'static str),
    NegativeParameter(&'static str),
    NonPositiveParameter(&'static str),
    DiffusionStabilityViolation {
        field: &'static str,
        courant: f64,
        maximum: f64,
    },
    NonFiniteInitialCondition,
    InvalidInitialCondition(&'static str),
    NonFiniteState {
        field: &'static str,
        index: usize,
    },
    ConcentrationOutOfRange {
        field: &'static str,
        index: usize,
        value: f64,
    },
}

impl fmt::Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGridDimensions => {
                write!(formatter, "grid dimensions must both be at least 3")
            }
            Self::NonFiniteParameter(name) => {
                write!(formatter, "parameter {name} must be finite")
            }
            Self::NegativeParameter(name) => {
                write!(formatter, "parameter {name} must be non-negative")
            }
            Self::NonPositiveParameter(name) => {
                write!(formatter, "parameter {name} must be positive")
            }
            Self::DiffusionStabilityViolation {
                field,
                courant,
                maximum,
            } => write!(
                formatter,
                "field {field} diffusion Courant value {courant} exceeds explicit 2D limit {maximum}"
            ),
            Self::NonFiniteInitialCondition => {
                write!(formatter, "initial-condition values must be finite")
            }
            Self::InvalidInitialCondition(reason) => write!(formatter, "{reason}"),
            Self::NonFiniteState { field, index } => {
                write!(formatter, "field {field} became non-finite at cell {index}")
            }
            Self::ConcentrationOutOfRange { field, index, value } => write!(
                formatter,
                "field {field} concentration {value} at cell {index} is outside [0, 1]"
            ),
        }
    }
}

impl Error for SimulationError {}

#[derive(Debug, Clone)]
pub struct GrayScottGrid {
    width: usize,
    height: usize,
    u: Vec<f64>,
    v: Vec<f64>,
    steps: usize,
}

impl GrayScottGrid {
    pub fn new(width: usize, height: usize) -> Result<Self, SimulationError> {
        if width < 3 || height < 3 {
            return Err(SimulationError::InvalidGridDimensions);
        }
        let cells = width
            .checked_mul(height)
            .ok_or(SimulationError::InvalidGridDimensions)?;
        Ok(Self {
            width,
            height,
            u: vec![1.0; cells],
            v: vec![0.0; cells],
            steps: 0,
        })
    }

    /// Apply a deterministic, seeded positive perturbation and central seed patch.
    ///
    /// The seed and noise amplitude are part of the benchmark configuration;
    /// there is no hidden or ambient randomness.
    pub fn seed_central_patch(
        &mut self,
        initial: InitialCondition,
    ) -> Result<(), SimulationError> {
        initial.validate(self.width, self.height)?;
        let mut rng = DeterministicLcg::new(initial.seed);

        for (u, v) in self.u.iter_mut().zip(self.v.iter_mut()) {
            *u = 1.0 - rng.next_f64() * initial.noise_amplitude;
            *v = rng.next_f64() * initial.noise_amplitude;
        }

        let center_x = self.width / 2;
        let center_y = self.height / 2;
        for y in center_y - initial.patch_radius..=center_y + initial.patch_radius {
            for x in center_x - initial.patch_radius..=center_x + initial.patch_radius {
                let index = self.index(x, y);
                self.u[index] = initial.patch_u;
                self.v[index] = initial.patch_v;
            }
        }
        self.steps = 0;
        Ok(())
    }

    /// Advance one explicit Euler step using periodic boundaries.
    ///
    /// The update is transactional: an invalid or unstable update leaves the
    /// previous grid untouched.
    pub fn step(
        &mut self,
        parameters: GrayScottParameters,
        mode: StepMode,
    ) -> Result<(), SimulationError> {
        parameters.validate()?;
        let mut next_u = vec![0.0; self.u.len()];
        let mut next_v = vec![0.0; self.v.len()];

        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                let u = self.u[index];
                let v = self.v[index];
                let lap_u = self.laplacian(&self.u, x, y);
                let lap_v = self.laplacian(&self.v, x, y);
                let reaction = u * v * v;

                let (du, dv) = match mode {
                    StepMode::GrayScott => (
                        parameters.diffusion_u * lap_u - reaction
                            + parameters.feed * (1.0 - u),
                        parameters.diffusion_v * lap_v + reaction
                            - (parameters.feed + parameters.kill) * v,
                    ),
                    StepMode::DiffusionOnly => (
                        parameters.diffusion_u * lap_u,
                        parameters.diffusion_v * lap_v,
                    ),
                };

                next_u[index] = u + parameters.dt * du;
                next_v[index] = v + parameters.dt * dv;

                validate_concentration("U", index, next_u[index])?;
                validate_concentration("V", index, next_v[index])?;
            }
        }

        self.u = next_u;
        self.v = next_v;
        self.steps += 1;
        Ok(())
    }

    pub fn run(
        &mut self,
        parameters: GrayScottParameters,
        mode: StepMode,
        steps: usize,
    ) -> Result<(), SimulationError> {
        for _ in 0..steps {
            self.step(parameters, mode)?;
        }
        Ok(())
    }

    pub fn summary(&self) -> FieldSummary {
        let mean_u = mean(&self.u);
        let mean_v = mean(&self.v);
        let variance_u = variance(&self.u, mean_u);
        let variance_v = variance(&self.v, mean_v);
        let min_u = self.u.iter().copied().fold(f64::INFINITY, f64::min);
        let max_u = self.u.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_v = self.v.iter().copied().fold(f64::INFINITY, f64::min);
        let max_v = self.v.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        FieldSummary {
            width: self.width,
            height: self.height,
            steps: self.steps,
            mean_u,
            mean_v,
            variance_u,
            variance_v,
            min_u,
            max_u,
            min_v,
            max_v,
        }
    }

    pub fn u_values(&self) -> &[f64] {
        &self.u
    }

    pub fn v_values(&self) -> &[f64] {
        &self.v
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn laplacian(&self, field: &[f64], x: usize, y: usize) -> f64 {
        let left = (x + self.width - 1) % self.width;
        let right = (x + 1) % self.width;
        let up = (y + self.height - 1) % self.height;
        let down = (y + 1) % self.height;
        let center = field[self.index(x, y)];
        field[self.index(left, y)]
            + field[self.index(right, y)]
            + field[self.index(x, up)]
            + field[self.index(x, down)]
            - 4.0 * center
    }
}

fn validate_concentration(
    field: &'static str,
    index: usize,
    value: f64,
) -> Result<(), SimulationError> {
    if !value.is_finite() {
        return Err(SimulationError::NonFiniteState { field, index });
    }
    let tolerance = 1.0e-12;
    if !(-tolerance..=1.0 + tolerance).contains(&value) {
        return Err(SimulationError::ConcentrationOutOfRange {
            field,
            index,
            value,
        });
    }
    Ok(())
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn variance(values: &[f64], mean: f64) -> f64 {
    values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64
}

struct DeterministicLcg {
    state: u32,
}

impl DeterministicLcg {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state as f64 / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 64;
    const HEIGHT: usize = 64;
    const STEPS: usize = 500;

    fn seeded_grid() -> GrayScottGrid {
        let mut grid = GrayScottGrid::new(WIDTH, HEIGHT).expect("valid grid");
        grid.seed_central_patch(InitialCondition::default())
            .expect("valid initial condition");
        grid
    }

    #[test]
    fn driven_pattern_separates_from_matched_diffusion_only_control() {
        let parameters = GrayScottParameters::reference_pattern();
        let mut driven = seeded_grid();
        let mut diffusion_only = seeded_grid();

        driven
            .run(parameters, StepMode::GrayScott, STEPS)
            .expect("driven model should remain stable");
        diffusion_only
            .run(parameters, StepMode::DiffusionOnly, STEPS)
            .expect("diffusion-only control should remain stable");

        let driven_summary = driven.summary();
        let control_summary = diffusion_only.summary();
        assert_eq!(driven_summary.steps, STEPS);
        assert_eq!(control_summary.steps, STEPS);
        assert!(driven_summary.variance_v > 0.005);
        assert!(driven_summary.variance_v > control_summary.variance_v * 50.0);
        assert!(driven_summary.min_u >= 0.0);
        assert!(driven_summary.max_u <= 1.0);
        assert!(driven_summary.min_v >= 0.0);
        assert!(driven_summary.max_v <= 1.0);
    }

    #[test]
    fn seeded_reference_replays_exactly() {
        let parameters = GrayScottParameters::reference_pattern();
        let mut first = seeded_grid();
        let mut second = seeded_grid();

        first
            .run(parameters, StepMode::GrayScott, STEPS)
            .expect("first replay should run");
        second
            .run(parameters, StepMode::GrayScott, STEPS)
            .expect("second replay should run");
        assert_eq!(first.summary(), second.summary());
        assert_eq!(first.u_values(), second.u_values());
        assert_eq!(first.v_values(), second.v_values());
    }

    #[test]
    fn unstable_diffusion_is_rejected_without_mutating_the_grid() {
        let mut grid = seeded_grid();
        let before = grid.summary();
        let mut parameters = GrayScottParameters::reference_pattern();
        parameters.diffusion_u = 0.251;

        let result = grid.step(parameters, StepMode::GrayScott);
        assert!(matches!(
            result,
            Err(SimulationError::DiffusionStabilityViolation { .. })
        ));
        assert_eq!(grid.summary(), before);
    }

    #[test]
    fn non_finite_parameters_are_rejected() {
        let mut parameters = GrayScottParameters::reference_pattern();
        parameters.feed = f64::NAN;
        assert!(matches!(
            parameters.validate(),
            Err(SimulationError::NonFiniteParameter("feed"))
        ));
    }

    #[test]
    fn zero_feed_intervention_is_executable_without_assuming_its_outcome() {
        let mut grid = seeded_grid();
        let mut parameters = GrayScottParameters::reference_pattern();
        parameters.feed = 0.0;
        grid.run(parameters, StepMode::GrayScott, 100)
            .expect("zero-feed intervention should remain numerically valid");
        assert_eq!(grid.summary().steps, 100);
        assert!(grid.summary().variance_v.is_finite());
    }

    #[test]
    fn undersized_grid_is_rejected() {
        assert!(matches!(
            GrayScottGrid::new(2, 64),
            Err(SimulationError::InvalidGridDimensions)
        ));
    }
}

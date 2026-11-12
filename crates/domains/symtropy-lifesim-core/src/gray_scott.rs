// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic CPU reference for the Gray–Scott reaction–diffusion system.
//!
//! This module is an explicit **abiotic pattern-formation control** for OLI. It can produce
//! persistent spots, stripes, and other spatial structures under driven reaction–diffusion
//! dynamics. The generator's mechanism is known and declared; pattern complexity is not
//! evidence of biological life, agency, intelligence, or consciousness.
//!
//! The solver is intentionally CPU-first, fixed-step, and dependency-light. It uses a
//! nine-point Laplacian, no-flux (clamped-neighbour) boundaries, explicit Euler integration,
//! and bounded state values. It is a benchmark reference, not a validated plasma, geochemical,
//! or laboratory model. Physical systems may require different equations and boundary
//! conditions.
//!
//! All observation outputs are simulation measurements. They must be tagged as such when
//! exported to the OLI evidence contract.

use std::error::Error;
use std::fmt;

/// Parameters for one normalized Gray–Scott reaction–diffusion system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrayScottParameters {
    /// Diffusion coefficient of the feed species U.
    pub diffusion_u: f64,
    /// Diffusion coefficient of the autocatalytic species V.
    pub diffusion_v: f64,
    /// Feed rate for U.
    pub feed: f64,
    /// Removal / kill rate for V.
    pub kill: f64,
    /// Fixed simulation timestep.
    pub dt: f64,
}

impl Default for GrayScottParameters {
    fn default() -> Self {
        Self {
            diffusion_u: 0.16,
            diffusion_v: 0.08,
            feed: 0.035,
            kill: 0.065,
            dt: 1.0,
        }
    }
}

impl GrayScottParameters {
    fn validate(self) -> Result<(), GrayScottError> {
        for (name, value) in [
            ("diffusion_u", self.diffusion_u),
            ("diffusion_v", self.diffusion_v),
            ("feed", self.feed),
            ("kill", self.kill),
            ("dt", self.dt),
        ] {
            if !value.is_finite() {
                return Err(GrayScottError::InvalidParameter { name, value });
            }
        }
        if self.diffusion_u < 0.0 {
            return Err(GrayScottError::InvalidParameter {
                name: "diffusion_u",
                value: self.diffusion_u,
            });
        }
        if self.diffusion_v < 0.0 {
            return Err(GrayScottError::InvalidParameter {
                name: "diffusion_v",
                value: self.diffusion_v,
            });
        }
        if !(0.0..=1.0).contains(&self.feed) {
            return Err(GrayScottError::InvalidParameter {
                name: "feed",
                value: self.feed,
            });
        }
        if !(0.0..=1.0).contains(&self.kill) {
            return Err(GrayScottError::InvalidParameter {
                name: "kill",
                value: self.kill,
            });
        }
        if self.dt <= 0.0 || self.dt > 1.0 {
            return Err(GrayScottError::InvalidParameter {
                name: "dt",
                value: self.dt,
            });
        }

        // Conservative stability guard for this stencil and explicit integrator.
        let diffusion_cfl = self.dt * self.diffusion_u.max(self.diffusion_v);
        if diffusion_cfl > 0.25 {
            return Err(GrayScottError::DiffusionStepTooLarge {
                dt: self.dt,
                max_diffusion: self.diffusion_u.max(self.diffusion_v),
                product: diffusion_cfl,
            });
        }
        Ok(())
    }
}

/// Structural or numerical error in the reference solver.
#[derive(Debug, Clone, PartialEq)]
pub enum GrayScottError {
    GridTooSmall {
        width: usize,
        height: usize,
    },
    GridSizeOverflow {
        width: usize,
        height: usize,
    },
    InvalidParameter {
        name: &'static str,
        value: f64,
    },
    DiffusionStepTooLarge {
        dt: f64,
        max_diffusion: f64,
        product: f64,
    },
    PatchOutOfBounds {
        x: usize,
        y: usize,
        patch_width: usize,
        patch_height: usize,
        grid_width: usize,
        grid_height: usize,
    },
    PatchValueOutOfRange {
        name: &'static str,
        value: f64,
    },
    PatchNoiseOutOfRange {
        name: &'static str,
        center: f64,
        amplitude: f64,
    },
    InvalidObservationThreshold(f64),
    TickOverflow,
}

impl fmt::Display for GrayScottError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridTooSmall { width, height } => {
                write!(
                    f,
                    "Gray–Scott grid must be at least 3x3, got {width}x{height}"
                )
            }
            Self::GridSizeOverflow { width, height } => {
                write!(f, "Gray–Scott grid size overflows usize: {width}x{height}")
            }
            Self::InvalidParameter { name, value } => {
                write!(f, "invalid Gray–Scott parameter {name}={value}")
            }
            Self::DiffusionStepTooLarge {
                dt,
                max_diffusion,
                product,
            } => write!(
                f,
                "explicit diffusion step is too large: dt={dt}, max_diffusion={max_diffusion}, product={product} > 0.25"
            ),
            Self::PatchOutOfBounds {
                x,
                y,
                patch_width,
                patch_height,
                grid_width,
                grid_height,
            } => write!(
                f,
                "patch ({x},{y}) {patch_width}x{patch_height} exceeds grid {grid_width}x{grid_height}"
            ),
            Self::PatchValueOutOfRange { name, value } => {
                write!(f, "patch value {name}={value} must be finite and in [0, 1]")
            }
            Self::PatchNoiseOutOfRange {
                name,
                center,
                amplitude,
            } => write!(
                f,
                "patch noise for {name} is out of range: center={center}, amplitude={amplitude}; center ± amplitude/2 must remain in [0, 1]"
            ),
            Self::InvalidObservationThreshold(value) => write!(
                f,
                "observation threshold must be finite and in [0, 1], got {value}"
            ),
            Self::TickOverflow => write!(f, "Gray–Scott tick counter overflow"),
        }
    }
}

impl Error for GrayScottError {}

/// Field summary derived from the current simulated state.
///
/// These are descriptive observables. They are not a life, agency, or intelligence score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrayScottMetrics {
    pub tick: u64,
    pub mean_u: f64,
    pub mean_v: f64,
    pub variance_v: f64,
    /// Mean squared difference across horizontal and vertical neighbour pairs.
    pub gradient_energy_v: f64,
    /// Fraction of cells where V exceeds the caller's declared threshold.
    pub high_v_fraction: f64,
}

/// Per-step numerical and reaction accounting.
///
/// This ledger is specific to the declared discrete update equations. It is not a claim of
/// thermodynamic energy conservation; it records reaction/feed/removal terms and numerical
/// clipping so that state changes are auditable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrayScottStepReport {
    pub tick: u64,
    pub u_total_before: f64,
    pub v_total_before: f64,
    pub u_total_after: f64,
    pub v_total_after: f64,
    /// Integrated autocatalytic transfer U -> V over this step.
    pub reaction_transfer: f64,
    /// Integrated feed contribution to U over this step.
    pub feed_input: f64,
    /// Integrated V removal term over this step.
    pub v_removal: f64,
    /// Sum of the U diffusion derivative over all cells, multiplied by dt.
    pub u_diffusion_net: f64,
    /// Sum of the V diffusion derivative over all cells, multiplied by dt.
    pub v_diffusion_net: f64,
    /// Absolute correction introduced by clamping raw U values to [0, 1].
    pub u_clipping_adjustment: f64,
    /// Absolute correction introduced by clamping raw V values to [0, 1].
    pub v_clipping_adjustment: f64,
}

/// CPU reference field for a two-species reaction–diffusion system.
#[derive(Debug, Clone, PartialEq)]
pub struct GrayScottField {
    width: usize,
    height: usize,
    parameters: GrayScottParameters,
    u: Vec<f64>,
    v: Vec<f64>,
    next_u: Vec<f64>,
    next_v: Vec<f64>,
    tick: u64,
}

impl GrayScottField {
    /// Create a uniform initial state: U=1 and V=0 everywhere.
    pub fn new(
        width: usize,
        height: usize,
        parameters: GrayScottParameters,
    ) -> Result<Self, GrayScottError> {
        if width < 3 || height < 3 {
            return Err(GrayScottError::GridTooSmall { width, height });
        }
        let cells = width
            .checked_mul(height)
            .ok_or(GrayScottError::GridSizeOverflow { width, height })?;
        parameters.validate()?;
        Ok(Self {
            width,
            height,
            parameters,
            u: vec![1.0; cells],
            v: vec![0.0; cells],
            next_u: vec![1.0; cells],
            next_v: vec![0.0; cells],
            tick: 0,
        })
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub const fn parameters(&self) -> GrayScottParameters {
        self.parameters
    }

    /// Read-only row-major U concentration values.
    pub fn u(&self) -> &[f64] {
        &self.u
    }

    /// Read-only row-major V concentration values.
    pub fn v(&self) -> &[f64] {
        &self.v
    }

    /// Set a rectangular perturbation. This is a controlled simulation intervention.
    ///
    /// The caller should record the patch geometry, values, and tick in its experiment
    /// manifest. The method does not label the response as repair, memory, or agency.
    pub fn set_patch(
        &mut self,
        x: usize,
        y: usize,
        patch_width: usize,
        patch_height: usize,
        u_value: f64,
        v_value: f64,
    ) -> Result<(), GrayScottError> {
        if patch_width == 0
            || patch_height == 0
            || x.checked_add(patch_width)
                .is_none_or(|end| end > self.width)
            || y.checked_add(patch_height)
                .is_none_or(|end| end > self.height)
        {
            return Err(GrayScottError::PatchOutOfBounds {
                x,
                y,
                patch_width,
                patch_height,
                grid_width: self.width,
                grid_height: self.height,
            });
        }
        for (name, value) in [("u_value", u_value), ("v_value", v_value)] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(GrayScottError::PatchValueOutOfRange { name, value });
            }
        }

        for patch_y in y..(y + patch_height) {
            for patch_x in x..(x + patch_width) {
                let idx = self.index(patch_x, patch_y);
                self.u[idx] = u_value;
                self.v[idx] = v_value;
            }
        }
        Ok(())
    }

    /// Set a rectangular patch with deterministic, explicitly seeded perturbations.
    ///
    /// Noise amplitudes specify the full peak-to-peak interval: values are drawn from
    /// `center - amplitude/2` through `center + amplitude/2`. The amplitudes and resulting
    /// ranges must remain within [0, 1]. This is useful for reproducible reaction–diffusion
    /// pattern seeding; the seed is an explicit experiment input, not hidden solver state.
    pub fn set_seeded_patch(
        &mut self,
        x: usize,
        y: usize,
        patch_width: usize,
        patch_height: usize,
        u_center: f64,
        v_center: f64,
        u_noise_amplitude: f64,
        v_noise_amplitude: f64,
        seed: u64,
    ) -> Result<(), GrayScottError> {
        if patch_width == 0
            || patch_height == 0
            || x.checked_add(patch_width)
                .is_none_or(|end| end > self.width)
            || y.checked_add(patch_height)
                .is_none_or(|end| end > self.height)
        {
            return Err(GrayScottError::PatchOutOfBounds {
                x,
                y,
                patch_width,
                patch_height,
                grid_width: self.width,
                grid_height: self.height,
            });
        }

        for (name, center) in [("u_center", u_center), ("v_center", v_center)] {
            if !center.is_finite() || !(0.0..=1.0).contains(&center) {
                return Err(GrayScottError::PatchValueOutOfRange {
                    name,
                    value: center,
                });
            }
        }
        for (name, center, amplitude) in [
            ("u", u_center, u_noise_amplitude),
            ("v", v_center, v_noise_amplitude),
        ] {
            if !amplitude.is_finite()
                || amplitude < 0.0
                || center - amplitude * 0.5 < 0.0
                || center + amplitude * 0.5 > 1.0
            {
                return Err(GrayScottError::PatchNoiseOutOfRange {
                    name,
                    center,
                    amplitude,
                });
            }
        }

        let mut rng = SeededPatchRng::new(seed);
        for patch_y in y..(y + patch_height) {
            for patch_x in x..(x + patch_width) {
                let idx = self.index(patch_x, patch_y);
                self.u[idx] = u_center + u_noise_amplitude * (rng.next_unit() - 0.5);
                self.v[idx] = v_center + v_noise_amplitude * (rng.next_unit() - 0.5);
            }
        }
        Ok(())
    }

    /// Advance one fixed timestep using an explicit nine-point Laplacian.
    ///
    /// The boundaries use a no-flux approximation by clamping neighbour coordinates to
    /// the edge cell. U and V are clamped to [0, 1] after each numerical update; the total
    /// absolute correction is returned in the step report rather than hidden.
    pub fn step(&mut self) -> Result<GrayScottStepReport, GrayScottError> {
        let next_tick = self
            .tick
            .checked_add(1)
            .ok_or(GrayScottError::TickOverflow)?;
        let dt = self.parameters.dt;
        let u_total_before = sum(&self.u);
        let v_total_before = sum(&self.v);

        let mut reaction_transfer = 0.0;
        let mut feed_input = 0.0;
        let mut v_removal = 0.0;
        let mut u_diffusion_net = 0.0;
        let mut v_diffusion_net = 0.0;
        let mut u_clipping_adjustment = 0.0;
        let mut v_clipping_adjustment = 0.0;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                let u = self.u[idx];
                let v = self.v[idx];
                let lap_u = self.laplacian(&self.u, x, y);
                let lap_v = self.laplacian(&self.v, x, y);
                let reaction = u * v * v;
                let feed = self.parameters.feed * (1.0 - u);
                let removal = (self.parameters.feed + self.parameters.kill) * v;
                let diffusion_u = self.parameters.diffusion_u * lap_u;
                let diffusion_v = self.parameters.diffusion_v * lap_v;

                let raw_u = u + dt * (diffusion_u - reaction + feed);
                let raw_v = v + dt * (diffusion_v + reaction - removal);
                let next_u = raw_u.clamp(0.0, 1.0);
                let next_v = raw_v.clamp(0.0, 1.0);

                self.next_u[idx] = next_u;
                self.next_v[idx] = next_v;
                reaction_transfer += reaction * dt;
                feed_input += feed * dt;
                v_removal += removal * dt;
                u_diffusion_net += diffusion_u * dt;
                v_diffusion_net += diffusion_v * dt;
                u_clipping_adjustment += (next_u - raw_u).abs();
                v_clipping_adjustment += (next_v - raw_v).abs();
            }
        }

        std::mem::swap(&mut self.u, &mut self.next_u);
        std::mem::swap(&mut self.v, &mut self.next_v);
        self.tick = next_tick;

        Ok(GrayScottStepReport {
            tick: self.tick,
            u_total_before,
            v_total_before,
            u_total_after: sum(&self.u),
            v_total_after: sum(&self.v),
            reaction_transfer,
            feed_input,
            v_removal,
            u_diffusion_net,
            v_diffusion_net,
            u_clipping_adjustment,
            v_clipping_adjustment,
        })
    }

    /// Run a fixed number of deterministic timesteps.
    pub fn run_steps(&mut self, steps: usize) -> Result<Vec<GrayScottStepReport>, GrayScottError> {
        let mut reports = Vec::with_capacity(steps);
        for _ in 0..steps {
            reports.push(self.step()?);
        }
        Ok(reports)
    }

    /// Compute spatial observables without assigning a biological interpretation.
    pub fn metrics(&self, high_v_threshold: f64) -> Result<GrayScottMetrics, GrayScottError> {
        if !high_v_threshold.is_finite() || !(0.0..=1.0).contains(&high_v_threshold) {
            return Err(GrayScottError::InvalidObservationThreshold(
                high_v_threshold,
            ));
        }
        let cells = self.u.len() as f64;
        let mean_u = sum(&self.u) / cells;
        let mean_v = sum(&self.v) / cells;
        let variance_v = self
            .v
            .iter()
            .map(|value| {
                let delta = value - mean_v;
                delta * delta
            })
            .sum::<f64>()
            / cells;

        let mut gradient_energy_v = 0.0;
        let mut neighbor_pairs = 0usize;
        let mut high_v_cells = 0usize;
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.v[idx] > high_v_threshold {
                    high_v_cells += 1;
                }
                if x + 1 < self.width {
                    let delta = self.v[idx] - self.v[self.index(x + 1, y)];
                    gradient_energy_v += delta * delta;
                    neighbor_pairs += 1;
                }
                if y + 1 < self.height {
                    let delta = self.v[idx] - self.v[self.index(x, y + 1)];
                    gradient_energy_v += delta * delta;
                    neighbor_pairs += 1;
                }
            }
        }

        Ok(GrayScottMetrics {
            tick: self.tick,
            mean_u,
            mean_v,
            variance_v,
            gradient_energy_v: gradient_energy_v / neighbor_pairs as f64,
            high_v_fraction: high_v_cells as f64 / cells,
        })
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn sample_no_flux(&self, field: &[f64], x: isize, y: isize) -> f64 {
        let x = x.clamp(0, (self.width - 1) as isize) as usize;
        let y = y.clamp(0, (self.height - 1) as isize) as usize;
        field[self.index(x, y)]
    }

    fn laplacian(&self, field: &[f64], x: usize, y: usize) -> f64 {
        let x = x as isize;
        let y = y as isize;
        let center = self.sample_no_flux(field, x, y);
        let axial = self.sample_no_flux(field, x - 1, y)
            + self.sample_no_flux(field, x + 1, y)
            + self.sample_no_flux(field, x, y - 1)
            + self.sample_no_flux(field, x, y + 1);
        let diagonal = self.sample_no_flux(field, x - 1, y - 1)
            + self.sample_no_flux(field, x + 1, y - 1)
            + self.sample_no_flux(field, x - 1, y + 1)
            + self.sample_no_flux(field, x + 1, y + 1);
        (-10.0 / 3.0) * center + (2.0 / 3.0) * axial + (1.0 / 6.0) * diagonal
    }
}

/// Local deterministic PRNG for explicit patch initialization.
/// The algorithm and seed are part of the simulation input and replay contract.
#[derive(Debug, Clone, Copy)]
struct SeededPatchRng {
    state: u64,
}

impl SeededPatchRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

fn sum(values: &[f64]) -> f64 {
    values.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_point_laplacian_recovers_laplacian_of_quadratic_field() {
        let mut field = GrayScottField::new(9, 9, GrayScottParameters::default()).unwrap();
        for y in 0..field.height {
            for x in 0..field.width {
                let xf = x as f64;
                let yf = y as f64;
                field.u[field.index(x, y)] = xf * xf + yf * yf;
            }
        }

        // For u(x,y)=x^2+y^2, the continuous Laplacian is exactly 4.
        // The interior nine-point stencil must reproduce that value on the unit grid.
        let observed = field.laplacian(&field.u, 4, 4);
        assert!((observed - 4.0).abs() < 1.0e-12, "observed={observed}");
    }

    #[test]
    fn no_flux_diffusion_preserves_total_v_when_reaction_terms_are_disabled() {
        let mut field = GrayScottField::new(
            16,
            16,
            GrayScottParameters {
                diffusion_u: 0.0,
                diffusion_v: 0.08,
                feed: 0.0,
                kill: 0.0,
                dt: 1.0,
            },
        )
        .unwrap();
        field.u.fill(0.0);
        field.set_patch(5, 5, 6, 6, 0.0, 1.0).unwrap();
        let before = field.v.iter().sum::<f64>();

        let reports = field.run_steps(50).unwrap();
        let after = field.v.iter().sum::<f64>();

        assert!(
            (after - before).abs() < 1.0e-10,
            "before={before}, after={after}"
        );
        assert!(
            reports
                .iter()
                .all(|report| report.v_diffusion_net.abs() < 1.0e-10)
        );
    }

    #[test]
    fn uniform_initial_state_is_a_fixed_point() {
        let mut field = GrayScottField::new(16, 16, GrayScottParameters::default()).unwrap();
        let before_u = field.u().to_vec();
        let before_v = field.v().to_vec();

        field.run_steps(10).unwrap();

        assert_eq!(field.u(), before_u);
        assert_eq!(field.v(), before_v);
        assert_eq!(field.tick(), 10);
    }

    #[test]
    fn same_seed_patch_and_parameters_produce_identical_replay() {
        let mut left = GrayScottField::new(64, 64, GrayScottParameters::default()).unwrap();
        let mut right = GrayScottField::new(64, 64, GrayScottParameters::default()).unwrap();
        left.set_patch(29, 29, 6, 6, 0.5, 0.25).unwrap();
        right.set_patch(29, 29, 6, 6, 0.5, 0.25).unwrap();

        let left_reports = left.run_steps(40).unwrap();
        let right_reports = right.run_steps(40).unwrap();

        assert_eq!(left.u(), right.u());
        assert_eq!(left.v(), right.v());
        assert_eq!(left_reports, right_reports);
    }

    #[test]
    fn bounded_state_and_observables_remain_finite() {
        let mut field = GrayScottField::new(48, 48, GrayScottParameters::default()).unwrap();
        field.set_patch(20, 20, 8, 8, 0.5, 0.25).unwrap();

        let reports = field.run_steps(100).unwrap();
        let metrics = field.metrics(0.1).unwrap();

        assert_eq!(reports.len(), 100);
        assert!(
            field
                .u()
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        );
        assert!(
            field
                .v()
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        );
        assert!(metrics.mean_u.is_finite());
        assert!(metrics.mean_v.is_finite());
        assert!(metrics.variance_v.is_finite());
        assert!(metrics.gradient_energy_v.is_finite());
        assert!((0.0..=1.0).contains(&metrics.high_v_fraction));
        assert!(reports.iter().all(|report| {
            report.reaction_transfer.is_finite()
                && report.feed_input.is_finite()
                && report.v_removal.is_finite()
                && report.u_clipping_adjustment.is_finite()
                && report.v_clipping_adjustment.is_finite()
        }));
    }

    #[test]
    fn seeded_patch_produces_multiple_reaction_diffusion_domains() {
        let mut first = GrayScottField::new(80, 80, GrayScottParameters::default()).unwrap();
        let mut second = GrayScottField::new(80, 80, GrayScottParameters::default()).unwrap();
        for field in [&mut first, &mut second] {
            field
                .set_seeded_patch(24, 24, 32, 32, 0.5, 0.25, 0.2, 0.2, 42)
                .unwrap();
        }

        first.run_steps(3_000).unwrap();
        second.run_steps(3_000).unwrap();
        assert_eq!(first.u(), second.u());
        assert_eq!(first.v(), second.v());

        let metrics = first.metrics(0.1).unwrap();
        assert!(metrics.variance_v > 0.0);
        assert!(metrics.gradient_energy_v > 0.0);
        assert!(metrics.high_v_fraction > 0.0);

        // Count four-connected components above the pre-registered V threshold.
        // Multiple separated domains are stronger evidence of pattern formation than
        // a merely non-uniform residual from the initial perturbation.
        let mask: Vec<bool> = first.v().iter().map(|value| *value > 0.1).collect();
        let mut visited = vec![false; mask.len()];
        let mut components = 0usize;
        for start in 0..mask.len() {
            if !mask[start] || visited[start] {
                continue;
            }
            components += 1;
            let mut stack = vec![start];
            visited[start] = true;
            while let Some(index) = stack.pop() {
                let x = index % first.width();
                let y = index / first.width();
                let neighbors = [
                    (x.checked_sub(1), Some(y)),
                    ((x + 1 < first.width()).then_some(x + 1), Some(y)),
                    (Some(x), y.checked_sub(1)),
                    (Some(x), (y + 1 < first.height()).then_some(y + 1)),
                ];
                for (nx, ny) in neighbors {
                    let (Some(nx), Some(ny)) = (nx, ny) else {
                        continue;
                    };
                    let neighbor = ny * first.width() + nx;
                    if mask[neighbor] && !visited[neighbor] {
                        visited[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }
        }
        assert!(
            components >= 2,
            "seeded reaction-diffusion control should produce separated V domains; got {components}"
        );
    }

    #[test]
    fn seeded_patch_rejects_noise_ranges_outside_unit_interval() {
        let mut field = GrayScottField::new(16, 16, GrayScottParameters::default()).unwrap();
        assert!(matches!(
            field.set_seeded_patch(2, 2, 4, 4, 0.5, 0.25, 1.2, 0.2, 1),
            Err(GrayScottError::PatchNoiseOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_grid_size_overflow_before_allocation() {
        assert!(matches!(
            GrayScottField::new(usize::MAX, 3, GrayScottParameters::default()),
            Err(GrayScottError::GridSizeOverflow { .. })
        ));
    }

    #[test]
    fn reports_invalid_dimensions_parameters_and_patch_geometry() {
        assert!(matches!(
            GrayScottField::new(2, 4, GrayScottParameters::default()),
            Err(GrayScottError::GridTooSmall { .. })
        ));
        assert!(matches!(
            GrayScottField::new(
                4,
                4,
                GrayScottParameters {
                    diffusion_u: 1.0,
                    dt: 1.0,
                    ..GrayScottParameters::default()
                }
            ),
            Err(GrayScottError::DiffusionStepTooLarge { .. })
        ));

        let mut field = GrayScottField::new(8, 8, GrayScottParameters::default()).unwrap();
        assert!(matches!(
            field.set_patch(7, 7, 2, 2, 0.5, 0.25),
            Err(GrayScottError::PatchOutOfBounds { .. })
        ));
        assert!(matches!(
            field.set_patch(1, 1, 1, 1, f64::NAN, 0.25),
            Err(GrayScottError::PatchValueOutOfRange { .. })
        ));
        assert!(matches!(
            field.metrics(f64::INFINITY),
            Err(GrayScottError::InvalidObservationThreshold(_))
        ));
    }
}

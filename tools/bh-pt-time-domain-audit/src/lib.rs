//! Independent direct time-domain Pöschl–Teller reference evolution.
//!
//! Solves
//!
//!   (d_t^2 - d_x^2 + V(x)) u(t,x) = S(t,x),
//!   V(x) = lambda / width^2 * sech^2(x / width),
//!
//! with zero initial data and fixed, distant Dirichlet boundaries. The
//! integration is a method-of-lines discretization (second-order centered
//! spatial operator, classical RK4 in time). This crate has no dependency on
//! the frequency-domain Jost / Green-function implementation.
//!
//! Boundary reflections are not absorbed. Every waveform carries the
//! earliest source-to-boundary-to-observer return time so comparisons can be
//! restricted to a demonstrably reflection-free time window.

use core::fmt;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CompactPulse {
    pub center_x: f64,
    pub spatial_half_width: f64,
    pub center_time: f64,
    pub temporal_half_width: f64,
    pub amplitude: f64,
}

impl CompactPulse {
    fn validate(self) -> Result<(), WaveError> {
        if !self.center_x.is_finite()
            || !self.spatial_half_width.is_finite()
            || !self.center_time.is_finite()
            || !self.temporal_half_width.is_finite()
            || !self.amplitude.is_finite()
            || self.spatial_half_width <= 0.0
            || self.temporal_half_width <= 0.0
            || self.center_time < self.temporal_half_width
        {
            return Err(WaveError::InvalidSource);
        }
        Ok(())
    }

    fn spatial_value(self, x: f64) -> f64 {
        compact_bump((x - self.center_x) / self.spatial_half_width)
    }

    fn temporal_value(self, time: f64) -> f64 {
        compact_bump((time - self.center_time) / self.temporal_half_width)
    }

    fn value(self, time: f64, x: f64) -> f64 {
        self.amplitude * self.temporal_value(time) * self.spatial_value(x)
    }

    pub fn start_time(self) -> f64 {
        self.center_time - self.temporal_half_width
    }

    pub fn end_time(self) -> f64 {
        self.center_time + self.temporal_half_width
    }
}

/// Numerical and physical parameters for the direct PDE reference.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SimulationConfig {
    pub width: f64,
    pub lambda: f64,
    pub x_min: f64,
    pub x_max: f64,
    pub dx: f64,
    pub dt: f64,
    pub final_time: f64,
    pub output_interval: f64,
    pub observer_x: f64,
    pub source: CompactPulse,
}

impl SimulationConfig {
    pub fn validate(self) -> Result<(), WaveError> {
        if !self.width.is_finite()
            || !self.lambda.is_finite()
            || !self.x_min.is_finite()
            || !self.x_max.is_finite()
            || !self.dx.is_finite()
            || !self.dt.is_finite()
            || !self.final_time.is_finite()
            || !self.output_interval.is_finite()
            || !self.observer_x.is_finite()
            || self.width <= 0.0
            || self.lambda < 0.0
            || self.x_max <= self.x_min
            || self.dx <= 0.0
            || self.dt <= 0.0
            || self.final_time <= 0.0
            || self.output_interval <= 0.0
            || self.observer_x <= self.x_min
            || self.observer_x >= self.x_max
        {
            return Err(WaveError::InvalidConfiguration);
        }
        self.source.validate()?;

        let spatial_cells = (self.x_max - self.x_min) / self.dx;
        if !is_integer_within_roundoff(spatial_cells) {
            return Err(WaveError::GridDoesNotTileDomain);
        }
        let time_steps = self.final_time / self.dt;
        let output_steps = self.output_interval / self.dt;
        if !is_integer_within_roundoff(time_steps) || !is_integer_within_roundoff(output_steps) {
            return Err(WaveError::TimeGridDoesNotTileInterval);
        }
        if !is_integer_within_roundoff(time_steps / output_steps) {
            return Err(WaveError::OutputGridDoesNotTileRun);
        }

        // Conservative RK4 stability guard for the largest eigenfrequency of
        // the centered 1-D Laplacian plus the maximum barrier height.
        if self.dt / self.dx > 0.4
            || self.dt * (4.0 / self.dx.powi(2) + self.lambda / self.width.powi(2)).sqrt() > 2.5
        {
            return Err(WaveError::UnstableTimeStep);
        }
        if self.source.center_x - self.source.spatial_half_width <= self.x_min
            || self.source.center_x + self.source.spatial_half_width >= self.x_max
        {
            return Err(WaveError::SourceOutsideDomain);
        }

        Ok(())
    }

    /// Earliest time at which a reflection from either fixed boundary can
    /// travel from the source support to the observer.
    pub fn boundary_reflection_free_until(self) -> f64 {
        let left_path = (self.source.center_x - self.source.spatial_half_width - self.x_min)
            + (self.observer_x - self.x_min);
        let right_path = (self.x_max - self.source.center_x - self.source.spatial_half_width)
            + (self.x_max - self.observer_x);
        self.source.start_time() + left_path.min(right_path)
    }

    pub fn first_physical_arrival(self) -> f64 {
        self.source.start_time()
            + ((self.observer_x - self.source.center_x).abs() - self.source.spatial_half_width)
                .max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ObserverSample {
    pub time: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EnergyDiagnostics {
    pub final_energy: f64,
    pub source_work: f64,
    pub relative_balance_error: f64,
    pub boundary_reflection_free_until: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    pub samples: Vec<ObserverSample>,
    pub dx: f64,
    pub dt: f64,
    pub output_interval: f64,
    pub observer_x: f64,
    pub source_x: f64,
    pub lambda: f64,
    pub width: f64,
    pub diagnostics: EnergyDiagnostics,
}

impl Waveform {
    pub fn max_abs_before(&self, final_time: f64) -> f64 {
        self.samples
            .iter()
            .filter(|sample| sample.time <= final_time)
            .map(|sample| sample.value.abs())
            .fold(0.0, f64::max)
    }

    pub fn max_abs(&self) -> f64 {
        self.samples
            .iter()
            .map(|sample| sample.value.abs())
            .fold(0.0, f64::max)
    }

    pub fn rms_difference(&self, other: &Self) -> Result<f64, WaveError> {
        if self.samples.len() != other.samples.len()
            || self.samples.is_empty()
            || self
                .samples
                .iter()
                .zip(&other.samples)
                .any(|(left, right)| (left.time - right.time).abs() > 1.0e-12)
        {
            return Err(WaveError::IncompatibleWaveforms);
        }
        let sum = self
            .samples
            .iter()
            .zip(&other.samples)
            .map(|(left, right)| (left.value - right.value).powi(2))
            .sum::<f64>();
        Ok((sum / self.samples.len() as f64).sqrt())
    }

    pub fn value_at(&self, time: f64) -> Result<f64, WaveError> {
        if !time.is_finite() || self.samples.is_empty() {
            return Err(WaveError::InvalidSampleTime);
        }
        let first = self.samples[0];
        let last = *self.samples.last().expect("non-empty samples checked");
        if time < first.time || time > last.time {
            return Err(WaveError::SampleTimeOutsideWaveform);
        }
        if self.samples.len() == 1 {
            return Ok(first.value);
        }
        let position = (time - first.time) / self.output_interval;
        let lower = (position.floor() as usize).min(self.samples.len() - 1);
        let upper = (lower + 1).min(self.samples.len() - 1);
        let fraction = position - lower as f64;
        Ok(self.samples[lower].value * (1.0 - fraction) + self.samples[upper].value * fraction)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveError {
    InvalidConfiguration,
    InvalidSource,
    GridDoesNotTileDomain,
    TimeGridDoesNotTileInterval,
    OutputGridDoesNotTileRun,
    UnstableTimeStep,
    SourceOutsideDomain,
    IncompatibleWaveforms,
    InvalidSampleTime,
    SampleTimeOutsideWaveform,
    FreeReferenceNotAfterFullPulse,
}

impl fmt::Display for WaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "invalid wave simulation configuration"),
            Self::InvalidSource => write!(f, "invalid compact source pulse"),
            Self::GridDoesNotTileDomain => write!(f, "dx does not tile the spatial domain"),
            Self::TimeGridDoesNotTileInterval => {
                write!(f, "dt does not tile final_time and output_interval")
            }
            Self::OutputGridDoesNotTileRun => {
                write!(f, "output_interval does not tile final_time")
            }
            Self::UnstableTimeStep => {
                write!(f, "time step exceeds the conservative RK4 stability guard")
            }
            Self::SourceOutsideDomain => write!(f, "source support touches or exceeds a boundary"),
            Self::IncompatibleWaveforms => {
                write!(f, "waveforms do not share an identical time-sampling grid")
            }
            Self::InvalidSampleTime => write!(f, "sample time must be finite"),
            Self::SampleTimeOutsideWaveform => write!(f, "sample time is outside the waveform"),
            Self::FreeReferenceNotAfterFullPulse => {
                write!(
                    f,
                    "requested time is before the complete free-wave pulse has arrived"
                )
            }
        }
    }
}

impl std::error::Error for WaveError {}

/// Solve the forced Pöschl–Teller wave equation with zero initial data.
///
/// The boundaries are fixed at zero. Callers must restrict comparisons to
/// times before the computed boundary-reflection window; this function does
/// not disguise boundary reflections as an absorbing-boundary approximation.
pub fn evolve(config: SimulationConfig) -> Result<Waveform, WaveError> {
    config.validate()?;

    let cell_count = ((config.x_max - config.x_min) / config.dx).round() as usize + 1;
    let step_count = (config.final_time / config.dt).round() as usize;
    let output_stride = (config.output_interval / config.dt).round() as usize;
    let x: Vec<f64> = (0..cell_count)
        .map(|index| config.x_min + index as f64 * config.dx)
        .collect();
    let potential: Vec<f64> = x
        .iter()
        .map(|position| {
            config.lambda / config.width.powi(2) / (position / config.width).cosh().powi(2)
        })
        .collect();
    let observer_grid_position = (config.observer_x - config.x_min) / config.dx;

    let mut u = vec![0.0; cell_count];
    let mut velocity = vec![0.0; cell_count];
    let mut samples = Vec::with_capacity(step_count / output_stride + 1);
    samples.push(ObserverSample {
        time: 0.0,
        value: interpolate_grid(&u, observer_grid_position),
    });
    let mut source_work = 0.0;

    for step in 0..step_count {
        let time = step as f64 * config.dt;
        let (k1_u, k1_v) = rhs(&u, &velocity, time, &x, &potential, config);
        let u2 = add_scaled(&u, &k1_u, 0.5 * config.dt);
        let v2 = add_scaled(&velocity, &k1_v, 0.5 * config.dt);
        let (k2_u, k2_v) = rhs(&u2, &v2, time + 0.5 * config.dt, &x, &potential, config);
        let u3 = add_scaled(&u, &k2_u, 0.5 * config.dt);
        let v3 = add_scaled(&velocity, &k2_v, 0.5 * config.dt);
        let (k3_u, k3_v) = rhs(&u3, &v3, time + 0.5 * config.dt, &x, &potential, config);
        let u4 = add_scaled(&u, &k3_u, config.dt);
        let v4 = add_scaled(&velocity, &k3_v, config.dt);
        let (k4_u, k4_v) = rhs(&u4, &v4, time + config.dt, &x, &potential, config);

        source_work += config.dt / 6.0
            * (source_power(&velocity, time, &x, config)
                + 2.0 * source_power(&v2, time + 0.5 * config.dt, &x, config)
                + 2.0 * source_power(&v3, time + 0.5 * config.dt, &x, config)
                + source_power(&v4, time + config.dt, &x, config));

        let sixth_dt = config.dt / 6.0;
        for index in 1..cell_count - 1 {
            u[index] +=
                sixth_dt * (k1_u[index] + 2.0 * k2_u[index] + 2.0 * k3_u[index] + k4_u[index]);
            velocity[index] +=
                sixth_dt * (k1_v[index] + 2.0 * k2_v[index] + 2.0 * k3_v[index] + k4_v[index]);
        }

        if (step + 1).is_multiple_of(output_stride) {
            let output_time = (step + 1) as f64 * config.dt;
            samples.push(ObserverSample {
                time: output_time,
                value: interpolate_grid(&u, observer_grid_position),
            });
        }
    }

    let final_energy = discrete_energy(&u, &velocity, &potential, config.dx);
    let relative_balance_error =
        (final_energy - source_work).abs() / final_energy.abs().max(source_work.abs()).max(1.0e-30);

    Ok(Waveform {
        samples,
        dx: config.dx,
        dt: config.dt,
        output_interval: config.output_interval,
        observer_x: config.observer_x,
        source_x: config.source.center_x,
        lambda: config.lambda,
        width: config.width,
        diagnostics: EnergyDiagnostics {
            final_energy,
            source_work,
            relative_balance_error,
            boundary_reflection_free_until: config.boundary_reflection_free_until(),
        },
    })
}

fn rhs(
    u: &[f64],
    velocity: &[f64],
    time: f64,
    x: &[f64],
    potential: &[f64],
    config: SimulationConfig,
) -> (Vec<f64>, Vec<f64>) {
    let mut du = vec![0.0; u.len()];
    let mut dv = vec![0.0; u.len()];
    let inv_dx2 = 1.0 / config.dx.powi(2);
    for index in 1..u.len() - 1 {
        du[index] = velocity[index];
        let laplacian = (u[index - 1] - 2.0 * u[index] + u[index + 1]) * inv_dx2;
        dv[index] = laplacian - potential[index] * u[index] + config.source.value(time, x[index]);
    }
    (du, dv)
}

fn add_scaled(base: &[f64], increment: &[f64], scale: f64) -> Vec<f64> {
    base.iter()
        .zip(increment)
        .map(|(value, delta)| value + scale * delta)
        .collect()
}

fn source_power(velocity: &[f64], time: f64, x: &[f64], config: SimulationConfig) -> f64 {
    velocity
        .iter()
        .zip(x)
        .map(|(speed, position)| speed * config.source.value(time, *position))
        .sum::<f64>()
        * config.dx
}

fn discrete_energy(u: &[f64], velocity: &[f64], potential: &[f64], dx: f64) -> f64 {
    let kinetic = 0.5 * dx * velocity.iter().map(|value| value.powi(2)).sum::<f64>();
    let barrier = 0.5
        * dx
        * u.iter()
            .zip(potential)
            .map(|(value, v)| v * value.powi(2))
            .sum::<f64>();
    let gradient = 0.5 / dx
        * u.windows(2)
            .map(|pair| (pair[1] - pair[0]).powi(2))
            .sum::<f64>();
    kinetic + barrier + gradient
}

fn interpolate_grid(values: &[f64], position: f64) -> f64 {
    let lower = (position.floor() as usize).min(values.len() - 1);
    let upper = (lower + 1).min(values.len() - 1);
    let fraction = position - lower as f64;
    values[lower] * (1.0 - fraction) + values[upper] * fraction
}

fn compact_bump(value: f64) -> f64 {
    if value.abs() >= 1.0 {
        0.0
    } else {
        (1.0 - 1.0 / (1.0 - value * value)).exp()
    }
}

fn is_integer_within_roundoff(value: f64) -> bool {
    value.is_finite() && (value - value.round()).abs() <= 1.0e-9 * value.abs().max(1.0)
}

/// Independent closed-form Green-function control for the free 1-D wave
/// equation (lambda = 0), evaluated only after the whole compact source pulse
/// lies inside the observer's past light cone.
///
///   u(t,x_o) = (1/2) integral dx' dt' S(t',x').
///
/// The two compact bump integrals are evaluated by composite Simpson
/// quadrature, independently of the PDE evolution grid.
pub fn free_full_pulse_reference(
    config: SimulationConfig,
    time: f64,
    quadrature_intervals: usize,
) -> Result<f64, WaveError> {
    config.validate()?;
    if config.lambda != 0.0
        || !time.is_finite()
        || quadrature_intervals < 2
        || !quadrature_intervals.is_multiple_of(2)
    {
        return Err(WaveError::InvalidConfiguration);
    }
    let maximum_distance =
        (config.observer_x - config.source.center_x).abs() + config.source.spatial_half_width;
    if time < config.source.end_time() + maximum_distance {
        return Err(WaveError::FreeReferenceNotAfterFullPulse);
    }

    let source = config.source;
    let spatial_integral = simpson(
        |x| source.spatial_value(x),
        source.center_x - source.spatial_half_width,
        source.center_x + source.spatial_half_width,
        quadrature_intervals,
    );
    let temporal_integral = simpson(
        |t| source.temporal_value(t),
        source.start_time(),
        source.end_time(),
        quadrature_intervals,
    );
    Ok(0.5 * source.amplitude * spatial_integral * temporal_integral)
}

fn simpson(function: impl Fn(f64) -> f64, lower: f64, upper: f64, intervals: usize) -> f64 {
    let step = (upper - lower) / intervals as f64;
    let mut sum = function(lower) + function(upper);
    for index in 1..intervals {
        let weight = if index.is_multiple_of(2) { 2.0 } else { 4.0 };
        sum += weight * function(lower + index as f64 * step);
    }
    sum * step / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(dx: f64, dt: f64, lambda: f64) -> SimulationConfig {
        SimulationConfig {
            width: 1.0,
            lambda,
            x_min: -16.0,
            x_max: 16.0,
            dx,
            dt,
            final_time: 8.0,
            output_interval: 0.01,
            observer_x: 2.0,
            source: CompactPulse {
                center_x: -1.0,
                spatial_half_width: 0.25,
                center_time: 0.6,
                temporal_half_width: 0.3,
                amplitude: 1.0,
            },
        }
    }

    #[test]
    fn rejects_unstable_and_untiled_grids() {
        let mut unstable = config(0.1, 0.05, 0.75);
        unstable.output_interval = 0.1;
        assert_eq!(unstable.validate(), Err(WaveError::UnstableTimeStep));
        assert_eq!(
            config(0.13, 0.0025, 0.75).validate(),
            Err(WaveError::GridDoesNotTileDomain)
        );
        assert_eq!(
            config(0.1, 0.003, 0.75).validate(),
            Err(WaveError::TimeGridDoesNotTileInterval)
        );
    }

    #[test]
    fn fixed_boundary_reflection_window_is_explicit_and_safe() {
        let value = config(0.1, 0.0025, 0.75);
        value.validate().unwrap();
        assert!(value.boundary_reflection_free_until() > value.final_time);
        assert!(value.first_physical_arrival() > 3.0);
    }

    #[test]
    fn free_wave_response_matches_independent_retarded_integral() {
        let value = config(0.05, 0.0025, 0.0);
        let waveform = evolve(value).unwrap();
        let reference = free_full_pulse_reference(value, 6.0, 1024).unwrap();
        let actual = waveform.value_at(6.0).unwrap();
        let relative_error = (actual - reference).abs() / reference.abs();
        assert!(
            relative_error < 4.0e-3,
            "actual={actual:e}, reference={reference:e}, relative_error={relative_error:e}"
        );
        assert!(
            waveform.diagnostics.relative_balance_error < 2.0e-5,
            "energy/work balance={:e}",
            waveform.diagnostics.relative_balance_error
        );
    }

    #[test]
    fn source_response_has_bounded_pre_front_leakage() {
        let value = config(0.1, 0.0025, 0.75);
        let waveform = evolve(value).unwrap();
        let relative_leakage = waveform.max_abs_before(2.5) / waveform.max_abs();
        assert!(
            relative_leakage <= 1.0e-6,
            "relative pre-front leakage={relative_leakage:e}"
        );

        let finer = evolve(config(0.05, 0.0025, 0.75)).unwrap();
        let finer_relative_leakage = finer.max_abs_before(2.5) / finer.max_abs();
        assert!(
            finer_relative_leakage < relative_leakage,
            "pre-front leakage did not decrease under spatial refinement: coarse={relative_leakage:e}, fine={finer_relative_leakage:e}"
        );
    }

    #[test]
    fn spatial_refinement_converges_at_fixed_time_step() {
        let coarse = evolve(config(0.2, 0.0025, 0.75)).unwrap();
        let medium = evolve(config(0.1, 0.0025, 0.75)).unwrap();
        let fine = evolve(config(0.05, 0.0025, 0.75)).unwrap();
        let coarse_error = coarse.rms_difference(&fine).unwrap();
        let medium_error = medium.rms_difference(&fine).unwrap();
        assert!(
            medium_error < 0.55 * coarse_error,
            "spatial refinement failed: coarse={coarse_error:e}, medium={medium_error:e}"
        );
    }

    #[test]
    fn time_refinement_converges_at_fixed_spatial_grid() {
        let coarse = evolve(config(0.1, 0.005, 0.75)).unwrap();
        let medium = evolve(config(0.1, 0.0025, 0.75)).unwrap();
        let fine = evolve(config(0.1, 0.00125, 0.75)).unwrap();
        let coarse_error = coarse.rms_difference(&fine).unwrap();
        let medium_error = medium.rms_difference(&fine).unwrap();
        assert!(
            medium_error < 0.55 * coarse_error,
            "time refinement failed: coarse={coarse_error:e}, medium={medium_error:e}"
        );
    }
}

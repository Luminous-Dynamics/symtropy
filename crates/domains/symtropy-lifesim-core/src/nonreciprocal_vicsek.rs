// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic reference for the two-dimensional nonreciprocal Vicsek model.
//!
//! The interaction matrix is row-major with receiver rows and source columns:
//! W[receiver, source] is the influence weight of a source on a receiver.
//! The absolute interaction weight scales the wrapped angular difference in the
//! receiver's update. Model-defined influence components are reported separately
//! from statistical information-transfer estimators such as transfer entropy.
//!
//! This is a synthetic known-mechanism control based on Pang & Wang (2026),
//! Phys. Rev. E 114, 024111, DOI 10.1103/rf2v-sy5d. It is not a model of all
//! animal collectives and is not evidence of life, agency, or intelligence.

use std::error::Error;
use std::fmt;

use crate::vicsek::VicsekParticle;

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;

/// Parameters for a nonreciprocal two-dimensional Vicsek system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonreciprocalVicsekConfig {
    pub box_size: f64,
    pub sensing_radius: f64,
    pub speed: f64,
    pub dt: f64,
    /// Full width of the independent uniform angular-noise interval in radians.
    pub angular_noise_width_rad: f64,
}

impl Default for NonreciprocalVicsekConfig {
    fn default() -> Self {
        Self {
            box_size: 5.0,
            sensing_radius: 5.0,
            speed: 2.0,
            dt: 0.05,
            angular_noise_width_rad: 0.0,
        }
    }
}

impl NonreciprocalVicsekConfig {
    fn validate(self) -> Result<(), NonreciprocalVicsekError> {
        for (name, value) in [
            ("box_size", self.box_size),
            ("sensing_radius", self.sensing_radius),
            ("speed", self.speed),
            ("dt", self.dt),
            ("angular_noise_width_rad", self.angular_noise_width_rad),
        ] {
            if !value.is_finite() {
                return Err(NonreciprocalVicsekError::InvalidParameter { name, value });
            }
        }
        if self.box_size <= 0.0
            || self.sensing_radius < 0.0
            || self.speed < 0.0
            || self.dt <= 0.0
            || !(0.0..=TAU).contains(&self.angular_noise_width_rad)
            || !(self.speed * self.dt).is_finite()
        {
            return Err(NonreciprocalVicsekError::InvalidConfiguration);
        }
        Ok(())
    }
}

/// Validation or state error in the nonreciprocal reference model.
#[derive(Debug, Clone, PartialEq)]
pub enum NonreciprocalVicsekError {
    EmptyParticleSet,
    InvalidParameter {
        name: &'static str,
        value: f64,
    },
    InvalidConfiguration,
    InteractionMatrixSize {
        expected: usize,
        actual: usize,
    },
    NoiseProfileSize {
        expected: usize,
        actual: usize,
    },
    InvalidNoiseWidth {
        particle: usize,
        value: f64,
    },
    NonFiniteInteractionWeight {
        index: usize,
        value: f64,
    },
    NonFiniteParticlePosition {
        particle: usize,
    },
    ParticlePositionOutOfBounds {
        particle: usize,
    },
    NonFiniteParticleHeading {
        particle: usize,
    },
    ParticleIndexOutOfBounds {
        particle: usize,
        particle_count: usize,
    },
    TickOverflow,
    InfluenceFrameOverflow,
}

impl fmt::Display for NonreciprocalVicsekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyParticleSet => write!(f, "particle set must not be empty"),
            Self::InvalidParameter { name, value } => {
                write!(f, "invalid nonreciprocal Vicsek parameter {name}={value}")
            }
            Self::InvalidConfiguration => write!(f, "invalid nonreciprocal Vicsek configuration"),
            Self::InteractionMatrixSize { expected, actual } => write!(
                f,
                "interaction matrix must contain {expected} values, got {actual}"
            ),
            Self::NoiseProfileSize { expected, actual } => write!(
                f,
                "noise profile must contain {expected} per-particle widths, got {actual}"
            ),
            Self::InvalidNoiseWidth { particle, value } => write!(
                f,
                "noise width for particle {particle} must be finite and in [0, 2π], got {value}"
            ),
            Self::NonFiniteInteractionWeight { index, value } => write!(
                f,
                "interaction matrix entry {index} must be finite, got {value}"
            ),
            Self::NonFiniteParticlePosition { particle } => {
                write!(f, "particle {particle} has a non-finite position")
            }
            Self::ParticlePositionOutOfBounds { particle } => {
                write!(f, "particle {particle} is outside the periodic box")
            }
            Self::NonFiniteParticleHeading { particle } => {
                write!(f, "particle {particle} has a non-finite heading")
            }
            Self::ParticleIndexOutOfBounds {
                particle,
                particle_count,
            } => write!(
                f,
                "particle index {particle} is outside population size {particle_count}"
            ),
            Self::TickOverflow => write!(f, "nonreciprocal Vicsek tick counter overflow"),
            Self::InfluenceFrameOverflow => write!(f, "influence frame counter overflow"),
        }
    }
}

impl Error for NonreciprocalVicsekError {}

/// Receipt for an explicit heading intervention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonreciprocalHeadingIntervention {
    pub tick: u64,
    pub particle_index: usize,
    pub previous_heading_rad: f64,
    pub imposed_heading_rad: f64,
}

/// Time-averaged source-defined influence summaries for one directed pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairwiseInfluenceSummary {
    pub receiver: usize,
    pub source: usize,
    pub frames: u64,
    /// Source A_at: mean absolute per-frame model contribution.
    pub mean_absolute_component_rad: Option<f64>,
    /// Source A_ta: absolute net signed per-frame model contribution.
    pub absolute_mean_signed_component_rad: Option<f64>,
}

/// Descriptive collective-motion statistics for a nonreciprocal system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonreciprocalVicsekMetrics {
    pub tick: u64,
    pub particle_count: usize,
    pub polar_order: f64,
    pub directed_interaction_edge_count: usize,
    pub mean_out_degree: f64,
    pub out_degree_variance: f64,
}

/// Seeded deterministic RNG. Initialization and angular-noise draws share one stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
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

/// Nonreciprocal active-particle reference model.
#[derive(Debug, Clone, PartialEq)]
pub struct NonreciprocalVicsekModel {
    config: NonreciprocalVicsekConfig,
    particles: Vec<VicsekParticle>,
    /// Row-major matrix: row = receiver, column = source.
    interaction_weights: Vec<f64>,
    /// Per-particle full-width angular-noise intervals; source-faithful constructors fill this with the scalar config value.
    noise_widths_rad: Vec<f64>,
    rng: XorShift64,
    tick: u64,
    last_influence_components: Vec<f64>,
    cumulative_absolute_components: Vec<f64>,
    cumulative_signed_components: Vec<f64>,
    influence_frames: u64,
}

impl NonreciprocalVicsekModel {
    /// Construct from explicit particle state, interaction matrix, and seed.
    pub fn new(
        particles: Vec<VicsekParticle>,
        interaction_weights: Vec<f64>,
        config: NonreciprocalVicsekConfig,
        seed: u64,
    ) -> Result<Self, NonreciprocalVicsekError> {
        let noise_widths_rad = vec![config.angular_noise_width_rad; particles.len()];
        Self::new_with_noise_profile(
            particles,
            interaction_weights,
            config,
            noise_widths_rad,
            seed,
        )
    }

    /// Construct with an explicit per-particle angular-noise profile.
    ///
    /// This is an OLI extension for separating influencer and follower noise.
    /// The source-faithful scalar profile remains available through new() and
    /// random_uniform().
    pub fn new_with_noise_profile(
        mut particles: Vec<VicsekParticle>,
        interaction_weights: Vec<f64>,
        config: NonreciprocalVicsekConfig,
        noise_widths_rad: Vec<f64>,
        seed: u64,
    ) -> Result<Self, NonreciprocalVicsekError> {
        config.validate()?;
        if particles.is_empty() {
            return Err(NonreciprocalVicsekError::EmptyParticleSet);
        }
        if noise_widths_rad.len() != particles.len() {
            return Err(NonreciprocalVicsekError::NoiseProfileSize {
                expected: particles.len(),
                actual: noise_widths_rad.len(),
            });
        }
        for (particle, width) in noise_widths_rad.iter().copied().enumerate() {
            if !width.is_finite() || !(0.0..=TAU).contains(&width) {
                return Err(NonreciprocalVicsekError::InvalidNoiseWidth {
                    particle,
                    value: width,
                });
            }
        }
        let matrix_len = particles
            .len()
            .checked_mul(particles.len())
            .ok_or(NonreciprocalVicsekError::InvalidConfiguration)?;
        if interaction_weights.len() != matrix_len {
            return Err(NonreciprocalVicsekError::InteractionMatrixSize {
                expected: matrix_len,
                actual: interaction_weights.len(),
            });
        }
        for (index, weight) in interaction_weights.iter().copied().enumerate() {
            if !weight.is_finite() {
                return Err(NonreciprocalVicsekError::NonFiniteInteractionWeight {
                    index,
                    value: weight,
                });
            }
        }
        for (index, particle) in particles.iter_mut().enumerate() {
            if !particle.x.is_finite() || !particle.y.is_finite() {
                return Err(NonreciprocalVicsekError::NonFiniteParticlePosition {
                    particle: index,
                });
            }
            if !(0.0..config.box_size).contains(&particle.x)
                || !(0.0..config.box_size).contains(&particle.y)
            {
                return Err(NonreciprocalVicsekError::ParticlePositionOutOfBounds {
                    particle: index,
                });
            }
            if !particle.heading_rad.is_finite() {
                return Err(NonreciprocalVicsekError::NonFiniteParticleHeading { particle: index });
            }
            particle.heading_rad = wrap_angle(particle.heading_rad);
        }
        Ok(Self {
            config,
            last_influence_components: vec![0.0; matrix_len],
            cumulative_absolute_components: vec![0.0; matrix_len],
            cumulative_signed_components: vec![0.0; matrix_len],
            influence_frames: 0,
            particles,
            interaction_weights,
            noise_widths_rad,
            rng: XorShift64::new(seed),
            tick: 0,
        })
    }

    /// Create a random population with one continued deterministic RNG stream.
    pub fn random_uniform(
        particle_count: usize,
        interaction_weights: Vec<f64>,
        config: NonreciprocalVicsekConfig,
        seed: u64,
    ) -> Result<Self, NonreciprocalVicsekError> {
        config.validate()?;
        if particle_count == 0 {
            return Err(NonreciprocalVicsekError::EmptyParticleSet);
        }
        let mut rng = XorShift64::new(seed);
        let particles = (0..particle_count)
            .map(|_| VicsekParticle {
                x: rng.next_unit() * config.box_size,
                y: rng.next_unit() * config.box_size,
                heading_rad: rng.next_unit() * TAU,
            })
            .collect();
        let mut model = Self::new(particles, interaction_weights, config, seed)?;
        model.rng = rng;
        Ok(model)
    }

    pub const fn config(&self) -> NonreciprocalVicsekConfig {
        self.config
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn particles(&self) -> &[VicsekParticle] {
        &self.particles
    }

    pub fn interaction_weights(&self) -> &[f64] {
        &self.interaction_weights
    }

    /// Per-particle full-width angular-noise intervals used by subsequent updates.
    pub fn noise_widths_rad(&self) -> &[f64] {
        &self.noise_widths_rad
    }

    pub fn last_influence_components(&self) -> &[f64] {
        &self.last_influence_components
    }

    /// Matrix indexing is receiver row, source column.
    pub fn interaction_weight(
        &self,
        receiver: usize,
        source: usize,
    ) -> Result<f64, NonreciprocalVicsekError> {
        let count = self.particles.len();
        if receiver >= count {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: receiver,
                particle_count: count,
            });
        }
        if source >= count {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: source,
                particle_count: count,
            });
        }
        Ok(self.interaction_weights[receiver * count + source])
    }

    /// Apply an explicit intervention without advancing or resetting the RNG.
    pub fn intervene_heading(
        &mut self,
        particle_index: usize,
        imposed_heading_rad: f64,
    ) -> Result<NonreciprocalHeadingIntervention, NonreciprocalVicsekError> {
        if !imposed_heading_rad.is_finite() {
            return Err(NonreciprocalVicsekError::NonFiniteParticleHeading {
                particle: particle_index,
            });
        }
        let particle_count = self.particles.len();
        let Some(particle) = self.particles.get_mut(particle_index) else {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: particle_index,
                particle_count,
            });
        };
        let previous_heading_rad = particle.heading_rad;
        let imposed_heading_rad = wrap_angle(imposed_heading_rad);
        particle.heading_rad = imposed_heading_rad;
        Ok(NonreciprocalHeadingIntervention {
            tick: self.tick,
            particle_index,
            previous_heading_rad,
            imposed_heading_rad,
        })
    }

    /// Advance one synchronous update under the declared weighted-angle equation.
    ///
    /// For receiver i, source j contributes the weighted, wrapped angular difference
    /// divided by the total visible interaction weight. Noise is sampled independently
    /// by receiver from the explicit seeded stream.
    pub fn step(&mut self) -> Result<NonreciprocalVicsekMetrics, NonreciprocalVicsekError> {
        let next_tick = self
            .tick
            .checked_add(1)
            .ok_or(NonreciprocalVicsekError::TickOverflow)?;
        let next_influence_frames = self
            .influence_frames
            .checked_add(1)
            .ok_or(NonreciprocalVicsekError::InfluenceFrameOverflow)?;
        let previous = self.particles.clone();
        let count = previous.len();
        let mut next = Vec::with_capacity(count);
        let mut components = vec![0.0; count * count];
        let mut next_abs = self.cumulative_absolute_components.clone();
        let mut next_signed = self.cumulative_signed_components.clone();

        for receiver in 0..count {
            let focal = previous[receiver];
            let mut maximum_weight: f64 = 0.0;
            let mut neighbor_weights = vec![0.0; count];
            let mut differences = vec![0.0; count];

            for source in 0..count {
                let neighbor = previous[source];
                let (dx, dy) = periodic_displacement(
                    focal.x,
                    focal.y,
                    neighbor.x,
                    neighbor.y,
                    self.config.box_size,
                );
                let adjacent = dx.hypot(dy) <= self.config.sensing_radius;
                let weight = if adjacent {
                    self.interaction_weights[receiver * count + source].abs()
                } else {
                    0.0
                };
                neighbor_weights[source] = weight;
                differences[source] = wrap_signed_angle(neighbor.heading_rad - focal.heading_rad);
                maximum_weight = maximum_weight.max(weight);
            }

            let scaled_denominator = if maximum_weight > 0.0 {
                neighbor_weights
                    .iter()
                    .map(|weight| weight / maximum_weight)
                    .sum::<f64>()
            } else {
                0.0
            };
            let mut normalized_sum = 0.0;
            if scaled_denominator > 0.0 {
                for source in 0..count {
                    let scaled_weight = neighbor_weights[source] / maximum_weight;
                    let contribution = if scaled_weight > 0.0 {
                        scaled_weight * differences[source] / scaled_denominator
                    } else {
                        0.0
                    };
                    let matrix_index = receiver * count + source;
                    components[matrix_index] = contribution;
                    normalized_sum += contribution;
                    next_abs[matrix_index] += contribution.abs();
                    next_signed[matrix_index] += contribution;
                }
            }

            let noise = self.noise_widths_rad[receiver] * (self.rng.next_unit() - 0.5);
            let heading = wrap_angle(focal.heading_rad + normalized_sum + noise);
            let distance = self.config.speed * self.config.dt;
            next.push(VicsekParticle {
                x: add_periodic(focal.x, distance * heading.cos(), self.config.box_size),
                y: add_periodic(focal.y, distance * heading.sin(), self.config.box_size),
                heading_rad: heading,
            });
        }

        self.particles = next;
        self.tick = next_tick;
        self.last_influence_components = components;
        self.cumulative_absolute_components = next_abs;
        self.cumulative_signed_components = next_signed;
        self.influence_frames = next_influence_frames;
        Ok(self.metrics())
    }

    pub fn run_steps(
        &mut self,
        steps: usize,
    ) -> Result<Vec<NonreciprocalVicsekMetrics>, NonreciprocalVicsekError> {
        let mut metrics = Vec::with_capacity(steps);
        for _ in 0..steps {
            metrics.push(self.step()?);
        }
        Ok(metrics)
    }

    pub fn metrics(&self) -> NonreciprocalVicsekMetrics {
        let count = self.particles.len();
        let mut degrees = vec![0usize; count];
        let mut edges = 0usize;
        for (receiver, degree) in degrees.iter_mut().enumerate() {
            for source in 0..count {
                if receiver == source {
                    continue;
                }
                let a = self.particles[receiver];
                let b = self.particles[source];
                let (dx, dy) = periodic_displacement(a.x, a.y, b.x, b.y, self.config.box_size);
                if dx.hypot(dy) <= self.config.sensing_radius
                    && self.interaction_weights[receiver * count + source].abs() > 0.0
                {
                    *degree += 1;
                    edges += 1;
                }
            }
        }
        let sum_cos = self
            .particles
            .iter()
            .map(|p| p.heading_rad.cos())
            .sum::<f64>();
        let sum_sin = self
            .particles
            .iter()
            .map(|p| p.heading_rad.sin())
            .sum::<f64>();
        let mean_degree = degrees.iter().sum::<usize>() as f64 / count as f64;
        let out_degree_variance = degrees
            .iter()
            .map(|degree| {
                let delta = *degree as f64 - mean_degree;
                delta * delta
            })
            .sum::<f64>()
            / count as f64;
        NonreciprocalVicsekMetrics {
            tick: self.tick,
            particle_count: count,
            polar_order: (sum_cos.hypot(sum_sin) / count as f64).clamp(0.0, 1.0),
            directed_interaction_edge_count: edges,
            mean_out_degree: mean_degree,
            out_degree_variance,
        }
    }

    /// Clear cumulative source-style influence statistics after a declared burn-in.
    ///
    /// The current source study discards the first 1,000 steps before computing its
    /// information/influence summaries. Call this at the registered analysis start tick
    /// to make the subsequent A_at/A_ta summaries use only post-burn-in frames.
    pub fn reset_influence_accumulator(&mut self) {
        self.cumulative_absolute_components.fill(0.0);
        self.cumulative_signed_components.fill(0.0);
        self.influence_frames = 0;
    }

    pub fn influence_component(
        &self,
        receiver: usize,
        source: usize,
    ) -> Result<f64, NonreciprocalVicsekError> {
        let count = self.particles.len();
        if receiver >= count {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: receiver,
                particle_count: count,
            });
        }
        if source >= count {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: source,
                particle_count: count,
            });
        }
        Ok(self.last_influence_components[receiver * count + source])
    }

    pub fn influence_summary(
        &self,
        receiver: usize,
        source: usize,
    ) -> Result<PairwiseInfluenceSummary, NonreciprocalVicsekError> {
        let count = self.particles.len();
        if receiver >= count {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: receiver,
                particle_count: count,
            });
        }
        if source >= count {
            return Err(NonreciprocalVicsekError::ParticleIndexOutOfBounds {
                particle: source,
                particle_count: count,
            });
        }
        if self.influence_frames == 0 {
            return Ok(PairwiseInfluenceSummary {
                receiver,
                source,
                frames: 0,
                mean_absolute_component_rad: None,
                absolute_mean_signed_component_rad: None,
            });
        }
        let index = receiver * count + source;
        Ok(PairwiseInfluenceSummary {
            receiver,
            source,
            frames: self.influence_frames,
            mean_absolute_component_rad: Some(
                self.cumulative_absolute_components[index] / self.influence_frames as f64,
            ),
            absolute_mean_signed_component_rad: Some(
                (self.cumulative_signed_components[index] / self.influence_frames as f64).abs(),
            ),
        })
    }
}

fn periodic_displacement(
    x_from: f64,
    y_from: f64,
    x_to: f64,
    y_to: f64,
    box_size: f64,
) -> (f64, f64) {
    (
        minimum_image(x_to - x_from, box_size),
        minimum_image(y_to - y_from, box_size),
    )
}

fn minimum_image(displacement: f64, box_size: f64) -> f64 {
    let wrapped = displacement.rem_euclid(box_size);
    if wrapped > 0.5 * box_size {
        wrapped - box_size
    } else {
        wrapped
    }
}

fn add_periodic(position: f64, displacement: f64, box_size: f64) -> f64 {
    let wrapped = displacement.rem_euclid(box_size);
    let distance_to_boundary = box_size - position;
    if wrapped >= distance_to_boundary {
        wrapped - distance_to_boundary
    } else {
        position + wrapped
    }
}

fn wrap_angle(angle: f64) -> f64 {
    angle.rem_euclid(TAU)
}

/// Wrap to (-π, π], matching the source paper's angle convention.
fn wrap_signed_angle(angle: f64) -> f64 {
    let wrapped = (angle + PI).rem_euclid(TAU) - PI;
    if wrapped <= -PI { PI } else { wrapped }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair_config() -> NonreciprocalVicsekConfig {
        NonreciprocalVicsekConfig {
            box_size: 5.0,
            sensing_radius: 5.0,
            speed: 0.0,
            dt: 0.05,
            angular_noise_width_rad: 0.0,
        }
    }

    fn pair(
        leader_heading: f64,
        follower_heading: f64,
        weights: Vec<f64>,
    ) -> NonreciprocalVicsekModel {
        NonreciprocalVicsekModel::new(
            vec![
                VicsekParticle {
                    x: 1.0,
                    y: 1.0,
                    heading_rad: leader_heading,
                },
                VicsekParticle {
                    x: 2.0,
                    y: 1.0,
                    heading_rad: follower_heading,
                },
            ],
            weights,
            pair_config(),
            42,
        )
        .unwrap()
    }

    #[test]
    fn receiver_row_source_column_interaction_semantics_are_explicit() {
        // Leader 0 receives from follower 1 with weight 1; follower 1 receives
        // from leader 0 with weight 4.
        let model = pair(0.0, std::f64::consts::FRAC_PI_2, vec![1.0, 1.0, 4.0, 1.0]);
        assert_eq!(model.interaction_weight(0, 1).unwrap(), 1.0);
        assert_eq!(model.interaction_weight(1, 0).unwrap(), 4.0);
    }

    #[test]
    fn nonreciprocal_update_matches_analytic_weighted_angle_equation() {
        let mut model = pair(0.0, std::f64::consts::FRAC_PI_2, vec![1.0, 1.0, 4.0, 1.0]);
        let metrics = model.step().unwrap();

        let expected_leader = std::f64::consts::FRAC_PI_4;
        let expected_follower = std::f64::consts::FRAC_PI_2 - (2.0 * std::f64::consts::PI / 5.0);
        assert!((model.particles()[0].heading_rad - expected_leader).abs() < 1.0e-12);
        assert!((model.particles()[1].heading_rad - expected_follower).abs() < 1.0e-12);
        assert_eq!(metrics.directed_interaction_edge_count, 2);
        assert!(
            (model.influence_component(0, 1).unwrap() - std::f64::consts::FRAC_PI_4).abs()
                < 1.0e-12
        );
        assert!(
            (model.influence_component(1, 0).unwrap() + (2.0 * std::f64::consts::PI / 5.0)).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn source_at_and_ta_summaries_are_distinct_for_sign_changing_components() {
        let mut model = pair(0.0, 0.8, vec![1.0, 2.0, 2.0, 1.0]);
        model.step().unwrap();
        model.intervene_heading(1, -0.8).unwrap();

        model.step().unwrap();
        let summary = model.influence_summary(0, 1).unwrap();
        assert_eq!(summary.frames, 2);
        let at = summary.mean_absolute_component_rad.unwrap();
        let ta = summary.absolute_mean_signed_component_rad.unwrap();
        assert!(at >= ta);
        assert!(at > 0.0);
        assert!(at > ta);
    }

    #[test]
    fn influence_accumulator_can_restart_after_declared_burn_in() {
        let mut model = pair(0.0, 0.8, vec![1.0, 2.0, 2.0, 1.0]);
        model.run_steps(4).unwrap();
        assert_eq!(model.influence_summary(0, 1).unwrap().frames, 4);

        model.reset_influence_accumulator();
        let reset = model.influence_summary(0, 1).unwrap();
        assert_eq!(reset.frames, 0);
        assert!(reset.mean_absolute_component_rad.is_none());
        assert!(reset.absolute_mean_signed_component_rad.is_none());

        model.run_steps(3).unwrap();
        assert_eq!(model.influence_summary(0, 1).unwrap().frames, 3);
    }

    #[test]
    fn zero_interaction_weight_does_not_create_a_directed_edge() {
        let mut model = pair(0.0, std::f64::consts::FRAC_PI_2, vec![1.0, 0.0, 0.0, 1.0]);
        let before = model.particles().to_vec();
        let metrics = model.step().unwrap();
        assert_eq!(metrics.directed_interaction_edge_count, 0);
        assert_eq!(model.particles()[0].heading_rad, before[0].heading_rad);
        assert_eq!(model.particles()[1].heading_rad, before[1].heading_rad);
    }

    #[test]
    fn huge_finite_interaction_weights_remain_numerically_bounded() {
        let huge = f64::MAX * 0.25;
        let mut model = pair(
            0.0,
            std::f64::consts::FRAC_PI_2,
            vec![huge, huge, huge, huge],
        );
        let metrics = model.step().unwrap();
        assert!(metrics.polar_order.is_finite());
        assert!((0.0..=1.0).contains(&metrics.polar_order));
        assert!(model.particles().iter().all(|particle| {
            particle.heading_rad.is_finite() && (0.0..TAU).contains(&particle.heading_rad)
        }));
        assert!(
            model
                .last_influence_components()
                .iter()
                .all(|component| component.is_finite())
        );
    }

    #[test]
    fn seeded_initialization_and_noise_replay_are_exact() {
        let weights = vec![1.0, 1.0, 4.0, 1.0];
        let mut a = NonreciprocalVicsekModel::random_uniform(2, weights.clone(), pair_config(), 77)
            .unwrap();
        let mut b =
            NonreciprocalVicsekModel::random_uniform(2, weights, pair_config(), 77).unwrap();
        let a_metrics = a.run_steps(32).unwrap();
        let b_metrics = b.run_steps(32).unwrap();
        assert_eq!(a, b);
        assert_eq!(a_metrics, b_metrics);
    }

    #[test]
    fn paired_intervention_clone_has_common_random_numbers() {
        let weights = vec![1.0, 1.0, 4.0, 1.0];
        let mut treatment = pair(0.0, 0.1, weights);
        let mut sham = treatment.clone();

        treatment
            .intervene_heading(0, std::f64::consts::FRAC_PI_2)
            .unwrap();
        for _ in 0..8 {
            treatment.step().unwrap();
            sham.step().unwrap();
        }

        assert_ne!(
            treatment.particles()[1].heading_rad,
            sham.particles()[1].heading_rad
        );
        assert_eq!(treatment.tick(), sham.tick());
        assert_eq!(treatment.rng, sham.rng);
    }

    #[test]
    fn per_particle_noise_profile_is_distinct_from_source_scalar_mode() {
        let particles = vec![
            VicsekParticle {
                x: 1.0,
                y: 1.0,
                heading_rad: 0.0,
            },
            VicsekParticle {
                x: 2.0,
                y: 1.0,
                heading_rad: 0.0,
            },
        ];
        let weights = vec![1.0, 1.0, 4.0, 1.0];
        let mut model = NonreciprocalVicsekModel::new_with_noise_profile(
            particles,
            weights,
            pair_config(),
            vec![0.0, std::f64::consts::PI],
            123,
        )
        .unwrap();

        assert_eq!(model.noise_widths_rad(), &[0.0, std::f64::consts::PI]);
        let before = model.particles().to_vec();
        model.step().unwrap();
        assert_eq!(model.particles()[0].heading_rad, before[0].heading_rad);
        assert_ne!(model.particles()[1].heading_rad, before[1].heading_rad);
    }

    #[test]
    fn per_particle_noise_profile_rejects_wrong_length_and_invalid_values() {
        let particles = vec![
            VicsekParticle {
                x: 1.0,
                y: 1.0,
                heading_rad: 0.0,
            },
            VicsekParticle {
                x: 2.0,
                y: 1.0,
                heading_rad: 0.0,
            },
        ];
        let weights = vec![1.0, 1.0, 4.0, 1.0];
        assert!(matches!(
            NonreciprocalVicsekModel::new_with_noise_profile(
                particles.clone(),
                weights.clone(),
                pair_config(),
                vec![0.1],
                1,
            ),
            Err(NonreciprocalVicsekError::NoiseProfileSize { .. })
        ));
        assert!(matches!(
            NonreciprocalVicsekModel::new_with_noise_profile(
                particles,
                weights,
                pair_config(),
                vec![0.1, f64::NAN],
                1,
            ),
            Err(NonreciprocalVicsekError::InvalidNoiseWidth { .. })
        ));
    }

    #[test]
    fn rejects_nonfinite_weights_and_wrong_matrix_size() {
        let particle = vec![VicsekParticle {
            x: 0.0,
            y: 0.0,
            heading_rad: 0.0,
        }];
        assert!(matches!(
            NonreciprocalVicsekModel::new(particle.clone(), vec![], pair_config(), 1),
            Err(NonreciprocalVicsekError::InteractionMatrixSize { .. })
        ));
        assert!(matches!(
            NonreciprocalVicsekModel::new(particle, vec![f64::NAN], pair_config(), 1),
            Err(NonreciprocalVicsekError::NonFiniteInteractionWeight { .. })
        ));
    }
}

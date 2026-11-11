// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic CPU reference for the two-dimensional Vicsek active-particle model.
//!
//! This is a synthetic known-mechanism collective-order control for the OLI benchmark
//! programme. Particles move at fixed speed in a periodic square, align their headings
//! with the circular mean of neighbours inside a metric interaction radius, and receive
//! a bounded angular-noise perturbation.
//!
//! The model has no biological metabolism, reproduction, heredity, memory, learning,
//! or intelligence mechanism. Polar order is a collective-motion observable, not a life,
//! agency, intelligence, or consciousness verdict.
//!
//! All particles are updated synchronously from the previous state. The implementation
//! is deliberately CPU-first, dependency-light, and deterministic for a fixed platform,
//! seed, initial state, configuration, and step count. It is an O(N^2) reference solver,
//! not a high-performance spatial-index implementation or an empirical animal-flocking
//! model.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

/// One self-propelled particle on a periodic two-dimensional square.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VicsekParticle {
    /// Horizontal position in [0, box_size).
    pub x: f64,
    /// Vertical position in [0, box_size).
    pub y: f64,
    /// Heading in radians. Construction normalizes finite angles to [0, 2π).
    pub heading_rad: f64,
}

/// Parameters for one discrete-time Vicsek system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VicsekConfig {
    /// Side length of the periodic square.
    pub box_size: f64,
    /// Metric interaction radius; the particle itself is included as a neighbour.
    pub interaction_radius: f64,
    /// Constant particle speed, in box-length units per time unit.
    pub speed: f64,
    /// Fixed integration interval.
    pub dt: f64,
    /// Full width of the uniform angular-noise interval, in radians.
    ///
    /// At each update, a perturbation is sampled uniformly from
    /// [-angular_noise_width_rad/2, +angular_noise_width_rad/2].
    pub angular_noise_width_rad: f64,
}

impl Default for VicsekConfig {
    fn default() -> Self {
        Self {
            box_size: 32.0,
            interaction_radius: 1.0,
            speed: 0.03,
            dt: 1.0,
            angular_noise_width_rad: 0.2,
        }
    }
}

impl VicsekConfig {
    fn validate(self) -> Result<(), VicsekError> {
        for (name, value) in [
            ("box_size", self.box_size),
            ("interaction_radius", self.interaction_radius),
            ("speed", self.speed),
            ("dt", self.dt),
            ("angular_noise_width_rad", self.angular_noise_width_rad),
        ] {
            if !value.is_finite() {
                return Err(VicsekError::InvalidParameter { name, value });
            }
        }
        if self.box_size <= 0.0 {
            return Err(VicsekError::InvalidParameter {
                name: "box_size",
                value: self.box_size,
            });
        }
        if self.interaction_radius < 0.0 {
            return Err(VicsekError::InvalidParameter {
                name: "interaction_radius",
                value: self.interaction_radius,
            });
        }
        if self.speed < 0.0 {
            return Err(VicsekError::InvalidParameter {
                name: "speed",
                value: self.speed,
            });
        }
        if self.dt <= 0.0 {
            return Err(VicsekError::InvalidParameter {
                name: "dt",
                value: self.dt,
            });
        }
        if !(0.0..=TAU).contains(&self.angular_noise_width_rad) {
            return Err(VicsekError::InvalidParameter {
                name: "angular_noise_width_rad",
                value: self.angular_noise_width_rad,
            });
        }
        if !(self.speed * self.dt).is_finite() {
            return Err(VicsekError::InvalidParameter {
                name: "speed_times_dt",
                value: self.speed * self.dt,
            });
        }
        Ok(())
    }
}

/// Structural or numerical error in the reference model.
#[derive(Debug, Clone, PartialEq)]
pub enum VicsekError {
    EmptyParticleSet,
    InvalidParameter { name: &'static str, value: f64 },
    NonFinitePosition { particle: usize, coordinate: &'static str },
    PositionOutOfBounds {
        particle: usize,
        coordinate: &'static str,
        value: f64,
        box_size: f64,
    },
    NonFiniteHeading { particle: usize },
    TickOverflow,
}

impl fmt::Display for VicsekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyParticleSet => write!(f, "Vicsek model requires at least one particle"),
            Self::InvalidParameter { name, value } => {
                write!(f, "invalid Vicsek parameter {name}={value}")
            }
            Self::NonFinitePosition { particle, coordinate } => write!(
                f,
                "particle {particle} has a non-finite {coordinate} position"
            ),
            Self::PositionOutOfBounds {
                particle,
                coordinate,
                value,
                box_size,
            } => write!(
                f,
                "particle {particle} {coordinate}={value} is outside [0, {box_size})"
            ),
            Self::NonFiniteHeading { particle } => {
                write!(f, "particle {particle} has a non-finite heading")
            }
            Self::TickOverflow => write!(f, "Vicsek tick counter overflow"),
        }
    }
}

impl Error for VicsekError {}

/// Descriptive observables for the current particle configuration.
///
/// These metrics characterize collective motion and interaction structure only. They
/// must not be interpreted as evidence of biological life or higher-order cognition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VicsekMetrics {
    pub tick: u64,
    pub particle_count: usize,
    /// Magnitude of the mean unit heading vector, in [0, 1].
    pub polar_order: f64,
    /// Mean number of other particles inside the interaction radius.
    pub mean_neighbors_excluding_self: f64,
    /// Population variance of the per-particle neighbour counts.
    pub neighbor_count_variance: f64,
    /// Number of connected components in the metric-neighbour graph.
    pub cluster_count: usize,
    /// Fraction of all particles in the largest connected component.
    pub largest_cluster_fraction: f64,
}

/// Periodic two-dimensional Vicsek active-particle reference model.
#[derive(Debug, Clone, PartialEq)]
pub struct VicsekModel {
    config: VicsekConfig,
    particles: Vec<VicsekParticle>,
    rng: XorShift64,
    tick: u64,
}

impl VicsekModel {
    /// Construct from an explicit particle configuration and PRNG seed.
    ///
    /// Positions must already lie inside the periodic box. Finite headings are normalized
    /// to [0, 2π). No random initialization is performed by this constructor.
    pub fn new(
        particles: Vec<VicsekParticle>,
        config: VicsekConfig,
        seed: u64,
    ) -> Result<Self, VicsekError> {
        Self::from_particles_and_rng(particles, config, XorShift64::new(seed))
    }

    fn from_particles_and_rng(
        mut particles: Vec<VicsekParticle>,
        config: VicsekConfig,
        rng: XorShift64,
    ) -> Result<Self, VicsekError> {
        config.validate()?;
        if particles.is_empty() {
            return Err(VicsekError::EmptyParticleSet);
        }

        for (index, particle) in particles.iter_mut().enumerate() {
            for (coordinate, value) in [("x", particle.x), ("y", particle.y)] {
                if !value.is_finite() {
                    return Err(VicsekError::NonFinitePosition {
                        particle: index,
                        coordinate,
                    });
                }
                if !(0.0..config.box_size).contains(&value) {
                    return Err(VicsekError::PositionOutOfBounds {
                        particle: index,
                        coordinate,
                        value,
                        box_size: config.box_size,
                    });
                }
            }
            if !particle.heading_rad.is_finite() {
                return Err(VicsekError::NonFiniteHeading { particle: index });
            }
            particle.heading_rad = wrap_angle(particle.heading_rad);
        }

        Ok(Self {
            config,
            particles,
            rng,
            tick: 0,
        })
    }

    /// Create an explicitly seeded uniform random particle configuration.
    pub fn random_uniform(
        particle_count: usize,
        config: VicsekConfig,
        seed: u64,
    ) -> Result<Self, VicsekError> {
        config.validate()?;
        if particle_count == 0 {
            return Err(VicsekError::EmptyParticleSet);
        }
        let mut rng = XorShift64::new(seed);
        let particles = (0..particle_count)
            .map(|_| VicsekParticle {
                x: rng.next_unit() * config.box_size,
                y: rng.next_unit() * config.box_size,
                heading_rad: rng.next_unit() * TAU,
            })
            .collect();
        // Continue the initialization stream for later angular-noise draws. This avoids
        // restarting the same seed at the beginning of the simulation-noise sequence.
        Self::from_particles_and_rng(particles, config, rng)
    }

    pub const fn config(&self) -> VicsekConfig {
        self.config
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn particles(&self) -> &[VicsekParticle] {
        &self.particles
    }

    /// Advance one synchronous Vicsek update.
    ///
    /// Every new heading is calculated from the previous particle configuration. Only
    /// after all headings are computed are positions advanced and wrapped periodically.
    pub fn step(&mut self) -> Result<VicsekMetrics, VicsekError> {
        let next_tick = self.tick.checked_add(1).ok_or(VicsekError::TickOverflow)?;
        let previous = self.particles.clone();
        let radius_squared = self.config.interaction_radius * self.config.interaction_radius;
        let mut next = Vec::with_capacity(previous.len());

        for particle in &previous {
            let mut sum_cos = 0.0;
            let mut sum_sin = 0.0;
            for neighbor in &previous {
                let (dx, dy) = periodic_displacement(
                    particle.x,
                    particle.y,
                    neighbor.x,
                    neighbor.y,
                    self.config.box_size,
                );
                if dx * dx + dy * dy <= radius_squared {
                    sum_cos += neighbor.heading_rad.cos();
                    sum_sin += neighbor.heading_rad.sin();
                }
            }

            // The particle itself is always a neighbour for a non-negative radius, so
            // this vector should not be exactly empty. Retain the old heading if numerical
            // cancellation makes its magnitude indistinguishable from zero.
            let alignment = if sum_cos.abs() <= f64::EPSILON && sum_sin.abs() <= f64::EPSILON {
                particle.heading_rad
            } else {
                sum_sin.atan2(sum_cos)
            };
            let noise = self.config.angular_noise_width_rad * (self.rng.next_unit() - 0.5);
            let heading = wrap_angle(alignment + noise);
            let distance = self.config.speed * self.config.dt;
            let x = (particle.x + distance * heading.cos()).rem_euclid(self.config.box_size);
            let y = (particle.y + distance * heading.sin()).rem_euclid(self.config.box_size);
            next.push(VicsekParticle {
                x,
                y,
                heading_rad: heading,
            });
        }

        self.particles = next;
        self.tick = next_tick;
        Ok(self.metrics())
    }

    /// Advance a fixed number of updates and return one metric row per completed tick.
    pub fn run_steps(&mut self, steps: usize) -> Result<Vec<VicsekMetrics>, VicsekError> {
        let mut reports = Vec::with_capacity(steps);
        for _ in 0..steps {
            reports.push(self.step()?);
        }
        Ok(reports)
    }

    /// Compute polar order, neighbour statistics and connected-component sizes.
    pub fn metrics(&self) -> VicsekMetrics {
        let count = self.particles.len();
        let mut sum_cos = 0.0;
        let mut sum_sin = 0.0;
        for particle in &self.particles {
            sum_cos += particle.heading_rad.cos();
            sum_sin += particle.heading_rad.sin();
        }
        let polar_order = (sum_cos.hypot(sum_sin) / count as f64).clamp(0.0, 1.0);

        let radius_squared = self.config.interaction_radius * self.config.interaction_radius;
        let mut neighbor_counts = vec![0usize; count];
        let mut parents: Vec<usize> = (0..count).collect();

        for left in 0..count {
            for right in (left + 1)..count {
                let a = self.particles[left];
                let b = self.particles[right];
                let (dx, dy) = periodic_displacement(
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    self.config.box_size,
                );
                if dx * dx + dy * dy <= radius_squared {
                    neighbor_counts[left] += 1;
                    neighbor_counts[right] += 1;
                    union(&mut parents, left, right);
                }
            }
        }

        let mean_neighbors = neighbor_counts.iter().sum::<usize>() as f64 / count as f64;
        let neighbor_count_variance = neighbor_counts
            .iter()
            .map(|neighbors| {
                let delta = *neighbors as f64 - mean_neighbors;
                delta * delta
            })
            .sum::<f64>()
            / count as f64;

        let mut component_sizes = vec![0usize; count];
        for index in 0..count {
            let root = find_root(&mut parents, index);
            component_sizes[root] += 1;
        }
        let cluster_count = component_sizes.iter().filter(|size| **size > 0).count();
        let largest_cluster = component_sizes.iter().copied().max().unwrap_or(0);

        VicsekMetrics {
            tick: self.tick,
            particle_count: count,
            polar_order,
            mean_neighbors_excluding_self: mean_neighbors,
            neighbor_count_variance,
            cluster_count,
            largest_cluster_fraction: largest_cluster as f64 / count as f64,
        }
    }
}

fn periodic_displacement(
    x_from: f64,
    y_from: f64,
    x_to: f64,
    y_to: f64,
    box_size: f64,
) -> (f64, f64) {
    let dx = (x_to - x_from + 0.5 * box_size).rem_euclid(box_size) - 0.5 * box_size;
    let dy = (y_to - y_from + 0.5 * box_size).rem_euclid(box_size) - 0.5 * box_size;
    (dx, dy)
}

fn wrap_angle(angle: f64) -> f64 {
    angle.rem_euclid(TAU)
}

fn find_root(parents: &mut [usize], index: usize) -> usize {
    let mut root = index;
    while parents[root] != root {
        root = parents[root];
    }
    let mut current = index;
    while parents[current] != current {
        let next = parents[current];
        parents[current] = root;
        current = next;
    }
    root
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left_root = find_root(parents, left);
    let right_root = find_root(parents, right);
    if left_root != right_root {
        parents[right_root] = left_root;
    }
}

/// Deterministic xorshift64 generator with an explicit zero-seed substitution.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn particles_from_headings(headings: &[f64], box_size: f64) -> Vec<VicsekParticle> {
        headings
            .iter()
            .enumerate()
            .map(|(index, heading_rad)| VicsekParticle {
                x: (index as f64 + 0.5) * box_size / headings.len() as f64,
                y: box_size * 0.5,
                heading_rad: *heading_rad,
            })
            .collect()
    }

    #[test]
    fn all_to_all_zero_noise_alignment_produces_global_polar_order() {
        let initial = particles_from_headings(&[0.0, 0.7, 1.4, 2.1, 3.2, 4.0], 10.0);
        let mut model = VicsekModel::new(
            initial,
            VicsekConfig {
                box_size: 10.0,
                interaction_radius: 8.0,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
            },
            7,
        )
        .unwrap();

        let metrics = model.step().unwrap();
        assert!(
            metrics.polar_order > 1.0 - 1.0e-12,
            "polar order should be 1 under all-to-all, zero-noise alignment: {metrics:?}"
        );
        assert_eq!(metrics.cluster_count, 1);
        assert_eq!(metrics.largest_cluster_fraction, 1.0);
    }

    #[test]
    fn self_only_zero_noise_control_preserves_headings_and_order() {
        let initial = particles_from_headings(&[0.0, 1.0, 2.0, 3.0, 4.0], 10.0);
        let mut model = VicsekModel::new(
            initial,
            VicsekConfig {
                box_size: 10.0,
                interaction_radius: 0.0,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
            },
            11,
        )
        .unwrap();
        let before = model.metrics().polar_order;
        let metrics = model.step().unwrap();

        assert_eq!(metrics.polar_order, before);
        assert_eq!(metrics.mean_neighbors_excluding_self, 0.0);
        assert_eq!(metrics.cluster_count, 5);
    }

    #[test]
    fn seeded_initialization_and_replay_are_deterministic() {
        let config = VicsekConfig::default();
        let mut left = VicsekModel::random_uniform(64, config, 0x0A11_CE).unwrap();
        let mut right = VicsekModel::random_uniform(64, config, 0x0A11_CE).unwrap();

        let left_reports = left.run_steps(40).unwrap();
        let right_reports = right.run_steps(40).unwrap();

        assert_eq!(left, right);
        assert_eq!(left_reports, right_reports);
    }

    #[test]
    fn particles_remain_inside_periodic_box_and_metrics_are_finite() {
        let config = VicsekConfig {
            box_size: 4.0,
            interaction_radius: 1.2,
            speed: 3.5,
            dt: 2.0,
            angular_noise_width_rad: std::f64::consts::PI,
        };
        let mut model = VicsekModel::random_uniform(48, config, 42).unwrap();
        let reports = model.run_steps(100).unwrap();

        assert_eq!(reports.len(), 100);
        assert!(model.particles().iter().all(|particle| {
            particle.x.is_finite()
                && particle.y.is_finite()
                && particle.heading_rad.is_finite()
                && (0.0..config.box_size).contains(&particle.x)
                && (0.0..config.box_size).contains(&particle.y)
                && (0.0..TAU).contains(&particle.heading_rad)
        }));
        assert!(reports.iter().all(|metrics| {
            metrics.polar_order.is_finite()
                && (0.0..=1.0).contains(&metrics.polar_order)
                && metrics.mean_neighbors_excluding_self.is_finite()
                && metrics.neighbor_count_variance.is_finite()
                && metrics.largest_cluster_fraction.is_finite()
        }));
    }

    #[test]
    fn rejects_invalid_parameters_and_particle_states() {
        assert!(matches!(
            VicsekModel::random_uniform(
                4,
                VicsekConfig {
                    box_size: 0.0,
                    ..VicsekConfig::default()
                },
                1
            ),
            Err(VicsekError::InvalidParameter { name: "box_size", .. })
        ));
        assert!(matches!(
            VicsekModel::random_uniform(
                4,
                VicsekConfig {
                    angular_noise_width_rad: TAU + 0.01,
                    ..VicsekConfig::default()
                },
                1
            ),
            Err(VicsekError::InvalidParameter {
                name: "angular_noise_width_rad",
                ..
            })
        ));
        assert!(matches!(
            VicsekModel::random_uniform(
                4,
                VicsekConfig {
                    speed: f64::MAX,
                    dt: 2.0,
                    ..VicsekConfig::default()
                },
                1
            ),
            Err(VicsekError::InvalidParameter { name: "speed_times_dt", .. })
        ));
        assert!(matches!(
            VicsekModel::new(
                vec![VicsekParticle {
                    x: 10.0,
                    y: 0.0,
                    heading_rad: 0.0,
                }],
                VicsekConfig::default(),
                1
            ),
            Err(VicsekError::PositionOutOfBounds { .. })
        ));
        assert!(matches!(
            VicsekModel::new(Vec::new(), VicsekConfig::default(), 1),
            Err(VicsekError::EmptyParticleSet)
        ));
    }
}

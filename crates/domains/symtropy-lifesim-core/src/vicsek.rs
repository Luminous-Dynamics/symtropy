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

use std::collections::VecDeque;
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

/// Neighbour-time convention used by the synchronous heading update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VicsekUpdateRule {
    /// Select neighbours from positions at time t, align, then move using the new heading.
    CurrentPositionAlignment,
    /// First move using headings at time t, select neighbours at t+dt, then align using
    /// the neighbours' headings at time t. This is the next-position-neighbour convention
    /// discussed in the 2026 Vicsek cluster-topology erratum.
    NextPositionAlignment,
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
    /// Exact update convention; do not compare results across conventions without tagging it.
    pub update_rule: VicsekUpdateRule,
}

impl Default for VicsekConfig {
    fn default() -> Self {
        Self {
            box_size: 32.0,
            interaction_radius: 1.0,
            speed: 0.03,
            dt: 1.0,
            angular_noise_width_rad: 0.2,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
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
    NonFiniteCommonFieldImpulse(f64),
    ParticleIndexOutOfBounds { particle: usize, particle_count: usize },
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
            Self::NonFiniteCommonFieldImpulse(value) => {
                write!(f, "common-field heading impulse must be finite, got {value}")
            }
            Self::ParticleIndexOutOfBounds {
                particle,
                particle_count,
            } => write!(
                f,
                "intervention particle index {particle} is outside population size {particle_count}"
            ),
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
    /// Neighbour-time convention used by the simulation that produced this snapshot.
    pub update_rule: VicsekUpdateRule,
    pub particle_count: usize,
    /// Magnitude of the mean unit heading vector, in [0, 1].
    pub polar_order: f64,
    /// Mean number of other particles inside the interaction radius.
    pub mean_neighbors_excluding_self: f64,
    /// Population variance of the per-particle neighbour counts.
    pub neighbor_count_variance: f64,
    /// Fraction of particles with at least one other particle in their interaction neighbourhood.
    pub local_order_coverage_fraction: f64,
    /// Mean local polar order over particles with at least one other neighbour.
    ///
    /// None means that no particle has a multi-particle neighbourhood; self-alignment
    /// alone is not treated as evidence of local collective order.
    pub mean_local_polar_order: Option<f64>,
    /// Population variance of supported local polar-order values.
    pub local_polar_order_variance: Option<f64>,
    /// Mean size of connected components in the metric-neighbour graph.
    pub mean_cluster_size: f64,
    /// Population variance of connected-component sizes.
    pub cluster_size_variance: f64,
    /// Mean polar order calculated separately within each connected component, including
    /// singleton components. This is descriptive only: singleton order is trivially one.
    pub mean_cluster_polar_order: f64,
    /// Population variance of per-component polar-order values, including singletons.
    pub cluster_polar_order_variance: f64,
    /// Fraction of particles belonging to a connected component containing at least two particles.
    /// A system of isolated particles has zero nontrivial-cluster coverage.
    pub nontrivial_cluster_coverage_fraction: f64,
    /// Mean polar order across components containing at least two particles; None if
    /// every component is a singleton.
    pub mean_nontrivial_cluster_polar_order: Option<f64>,
    /// Variance of nontrivial-component polar order; None if there are no such components.
    pub nontrivial_cluster_polar_order_variance: Option<f64>,
    /// Whether the optional all-pairs graph topology metrics were computed for this snapshot.
    pub topology_metrics_computed: bool,
    /// Mean local clustering coefficient over all particles; isolated/degree-one vertices contribute zero.
    /// None means the topology analysis was not requested for this snapshot.
    pub mean_clustering_coefficient: Option<f64>,
    /// Mean shortest-path length over all connected unordered particle pairs.
    /// None means no connected pair exists, or topology analysis was not requested.
    pub mean_shortest_path_length: Option<f64>,
    /// Fraction of all unordered particle pairs that are in the same connected component.
    /// None means the topology analysis was not requested for this snapshot.
    pub connected_pair_fraction: Option<f64>,
    /// Number of connected components in the metric-neighbour graph.
    pub cluster_count: usize,
    /// Fraction of all particles in the largest connected component.
    pub largest_cluster_fraction: f64,
}

/// Receipt for a direct heading intervention applied at the current model tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadingIntervention {
    /// Model tick at which the intervention was imposed, before the next update.
    pub tick: u64,
    pub particle_index: usize,
    pub previous_heading_rad: f64,
    pub imposed_heading_rad: f64,
}

/// Receipt for a spatially shared exogenous heading impulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonFieldIntervention {
    /// Model tick at which the field was applied.
    pub tick: u64,
    /// Shared angular displacement applied to every particle.
    pub heading_impulse_rad: f64,
    /// Number of particles receiving the common field input.
    pub affected_particle_count: usize,
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

    /// Directly set one particle's heading as a controlled intervention.
    ///
    /// The method returns a receipt with the previous and imposed headings and does not
    /// advance or reset the PRNG stream. Clone the pre-intervention model to construct a
    /// matched sham control with identical state and future noise draws.
    pub fn intervene_heading(
        &mut self,
        particle_index: usize,
        imposed_heading_rad: f64,
    ) -> Result<HeadingIntervention, VicsekError> {
        if !imposed_heading_rad.is_finite() {
            return Err(VicsekError::NonFiniteHeading {
                particle: particle_index,
            });
        }
        let particle_count = self.particles.len();
        let Some(particle) = self.particles.get_mut(particle_index) else {
            return Err(VicsekError::ParticleIndexOutOfBounds {
                particle: particle_index,
                particle_count,
            });
        };
        let previous_heading_rad = particle.heading_rad;
        let imposed_heading_rad = wrap_angle(imposed_heading_rad);
        particle.heading_rad = imposed_heading_rad;
        Ok(HeadingIntervention {
            tick: self.tick,
            particle_index,
            previous_heading_rad,
            imposed_heading_rad,
        })
    }

    /// Apply the same exogenous angular impulse to every particle.
    ///
    /// This models a shared environmental driver, not an interaction between particles.
    /// It changes no positions and consumes no random numbers. A matched sham can be made
    /// by cloning the pre-intervention model and not applying the common field.
    pub fn apply_common_heading_impulse(
        &mut self,
        heading_impulse_rad: f64,
    ) -> Result<CommonFieldIntervention, VicsekError> {
        if !heading_impulse_rad.is_finite() {
            return Err(VicsekError::NonFiniteCommonFieldImpulse(heading_impulse_rad));
        }
        let heading_impulse_rad = wrap_signed_angle(heading_impulse_rad);
        for particle in &mut self.particles {
            particle.heading_rad = wrap_angle(particle.heading_rad + heading_impulse_rad);
        }
        Ok(CommonFieldIntervention {
            tick: self.tick,
            heading_impulse_rad,
            affected_particle_count: self.particles.len(),
        })
    }

    /// Advance one synchronous Vicsek update under the configured neighbour-time rule.
    ///
    /// `CurrentPositionAlignment` uses neighbours at t, aligns, then moves using the new
    /// heading. `NextPositionAlignment` first predicts positions at t+dt using headings at
    /// t, chooses neighbours from those future positions, then computes each new heading
    /// from the neighbours' headings at t. Both modes update all particles from one frozen
    /// previous state; neither mode observes partially updated particles.
    pub fn step(&mut self) -> Result<VicsekMetrics, VicsekError> {
        let next_tick = self.tick.checked_add(1).ok_or(VicsekError::TickOverflow)?;
        let previous = self.particles.clone();
        let distance = self.config.speed * self.config.dt;

        let neighbour_positions: Vec<(f64, f64)> = match self.config.update_rule {
            VicsekUpdateRule::CurrentPositionAlignment => previous
                .iter()
                .map(|particle| (particle.x, particle.y))
                .collect(),
            VicsekUpdateRule::NextPositionAlignment => previous
                .iter()
                .map(|particle| {
                    (
                        add_periodic(
                            particle.x,
                            distance * particle.heading_rad.cos(),
                            self.config.box_size,
                        ),
                        add_periodic(
                            particle.y,
                            distance * particle.heading_rad.sin(),
                            self.config.box_size,
                        ),
                    )
                })
                .collect(),
        };

        let mut next = Vec::with_capacity(previous.len());
        for (index, particle) in previous.iter().enumerate() {
            let mut sum_cos = 0.0;
            let mut sum_sin = 0.0;
            for (neighbor_index, neighbor) in previous.iter().enumerate() {
                let (neighbor_x, neighbor_y) = neighbour_positions[neighbor_index];
                let (dx, dy) = periodic_displacement(
                    neighbour_positions[index].0,
                    neighbour_positions[index].1,
                    neighbor_x,
                    neighbor_y,
                    self.config.box_size,
                );
                if dx.hypot(dy) <= self.config.interaction_radius {
                    // In either convention, the interaction graph is selected according
                    // to the configured position time, while alignment uses headings from t.
                    sum_cos += neighbor.heading_rad.cos();
                    sum_sin += neighbor.heading_rad.sin();
                }
            }

            // The particle itself is always a neighbour for a non-negative radius. Retain
            // the old heading if numerical cancellation makes the vector indistinguishable
            // from zero.
            let alignment = if sum_cos.abs() <= f64::EPSILON && sum_sin.abs() <= f64::EPSILON {
                particle.heading_rad
            } else {
                sum_sin.atan2(sum_cos)
            };
            let noise = self.config.angular_noise_width_rad * (self.rng.next_unit() - 0.5);
            let heading = wrap_angle(alignment + noise);

            let (x, y) = match self.config.update_rule {
                VicsekUpdateRule::CurrentPositionAlignment => (
                    add_periodic(
                        particle.x,
                        distance * heading.cos(),
                        self.config.box_size,
                    ),
                    add_periodic(
                        particle.y,
                        distance * heading.sin(),
                        self.config.box_size,
                    ),
                ),
                VicsekUpdateRule::NextPositionAlignment => neighbour_positions[index],
            };
            next.push(VicsekParticle {
                x,
                y,
                heading_rad: heading,
            });
        }

        self.particles = next;
        self.tick = next_tick;
        Ok(self.fast_metrics())
    }

    /// Advance a fixed number of updates and return one metric row per completed tick.
    pub fn run_steps(&mut self, steps: usize) -> Result<Vec<VicsekMetrics>, VicsekError> {
        let mut reports = Vec::with_capacity(steps);
        for _ in 0..steps {
            reports.push(self.step()?);
        }
        Ok(reports)
    }

    /// Compute trajectory-scale metrics without all-pairs network path analysis.
    ///
    /// This is the method used by `step` and `run_steps`. It computes polar/local order,
    /// neighbour distributions and connected components in O(N²) time, but leaves optional
    /// clustering/path topology metrics unset so that long trajectories remain tractable.
    pub fn fast_metrics(&self) -> VicsekMetrics {
        self.metrics_impl(false)
    }

    /// Compute the complete metric set, including graph clustering and shortest paths.
    ///
    /// Call at registered checkpoints rather than on every frame for large systems. The
    /// full network analysis is more expensive than the O(N²) trajectory update.
    pub fn metrics(&self) -> VicsekMetrics {
        self.metrics_impl(true)
    }

    fn metrics_impl(&self, include_topology: bool) -> VicsekMetrics {
        let count = self.particles.len();
        let mut sum_cos = 0.0;
        let mut sum_sin = 0.0;
        for particle in &self.particles {
            sum_cos += particle.heading_rad.cos();
            sum_sin += particle.heading_rad.sin();
        }
        let polar_order = (sum_cos.hypot(sum_sin) / count as f64).clamp(0.0, 1.0);

        let mut neighbor_counts = vec![0usize; count];
        let mut adjacency = include_topology.then(|| vec![Vec::<usize>::new(); count]);
        let mut local_sum_cos: Vec<f64> = self
            .particles
            .iter()
            .map(|particle| particle.heading_rad.cos())
            .collect();
        let mut local_sum_sin: Vec<f64> = self
            .particles
            .iter()
            .map(|particle| particle.heading_rad.sin())
            .collect();
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
                if dx.hypot(dy) <= self.config.interaction_radius {
                    neighbor_counts[left] += 1;
                    neighbor_counts[right] += 1;
                    if let Some(graph) = &mut adjacency {
                        graph[left].push(right);
                        graph[right].push(left);
                    }
                    local_sum_cos[left] += b.heading_rad.cos();
                    local_sum_cos[right] += a.heading_rad.cos();
                    local_sum_sin[left] += b.heading_rad.sin();
                    local_sum_sin[right] += a.heading_rad.sin();
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

        // Local order is only defined for particles with at least one other
        // neighbour. A one-particle neighbourhood has order 1 by construction and
        // would otherwise make an isolated random gas look perfectly locally ordered.
        let supported_orders: Vec<f64> = (0..count)
            .filter(|index| neighbor_counts[*index] > 0)
            .map(|index| {
                (local_sum_cos[index].hypot(local_sum_sin[index])
                    / (neighbor_counts[index] + 1) as f64)
                    .clamp(0.0, 1.0)
            })
            .collect();
        let local_order_coverage_fraction = supported_orders.len() as f64 / count as f64;
        let mean_local_polar_order = if supported_orders.is_empty() {
            None
        } else {
            Some(supported_orders.iter().sum::<f64>() / supported_orders.len() as f64)
        };
        let local_polar_order_variance = mean_local_polar_order.map(|mean| {
            supported_orders
                .iter()
                .map(|order| {
                    let delta = *order - mean;
                    delta * delta
                })
                .sum::<f64>()
                / supported_orders.len() as f64
        });

        let mut component_sizes = vec![0usize; count];
        let mut component_sum_cos = vec![0.0f64; count];
        let mut component_sum_sin = vec![0.0f64; count];
        for (index, particle) in self.particles.iter().enumerate() {
            let root = find_root(&mut parents, index);
            component_sizes[root] += 1;
            component_sum_cos[root] += particle.heading_rad.cos();
            component_sum_sin[root] += particle.heading_rad.sin();
        }
        let cluster_sizes: Vec<usize> = component_sizes
            .iter()
            .copied()
            .filter(|size| *size > 0)
            .collect();
        let cluster_count = cluster_sizes.len();
        let largest_cluster = cluster_sizes.iter().copied().max().unwrap_or(0);
        let mean_cluster_size = count as f64 / cluster_count as f64;
        let cluster_size_variance = cluster_sizes
            .iter()
            .map(|size| {
                let delta = *size as f64 - mean_cluster_size;
                delta * delta
            })
            .sum::<f64>()
            / cluster_count as f64;
        let cluster_orders: Vec<f64> = (0..count)
            .filter(|root| component_sizes[*root] > 0)
            .map(|root| {
                (component_sum_cos[root].hypot(component_sum_sin[root])
                    / component_sizes[root] as f64)
                    .clamp(0.0, 1.0)
            })
            .collect();
        let mean_cluster_polar_order =
            cluster_orders.iter().sum::<f64>() / cluster_count as f64;
        let cluster_polar_order_variance = cluster_orders
            .iter()
            .map(|order| {
                let delta = *order - mean_cluster_polar_order;
                delta * delta
            })
            .sum::<f64>()
            / cluster_count as f64;

        // A singleton connected component has polar order 1 by construction, regardless
        // of whether it interacts with anything. Keep the all-component statistic for
        // source compatibility, but expose support and order for nontrivial clusters
        // separately so an isolated gas cannot appear perfectly cluster-coherent.
        let nontrivial_cluster_particle_count: usize = component_sizes
            .iter()
            .filter(|size| **size > 1)
            .sum();
        let nontrivial_cluster_orders: Vec<f64> = (0..count)
            .filter(|root| component_sizes[*root] > 1)
            .map(|root| {
                (component_sum_cos[root].hypot(component_sum_sin[root])
                    / component_sizes[root] as f64)
                    .clamp(0.0, 1.0)
            })
            .collect();
        let mean_nontrivial_cluster_polar_order = if nontrivial_cluster_orders.is_empty() {
            None
        } else {
            Some(
                nontrivial_cluster_orders.iter().sum::<f64>()
                    / nontrivial_cluster_orders.len() as f64,
            )
        };
        let nontrivial_cluster_polar_order_variance =
            mean_nontrivial_cluster_polar_order.map(|mean| {
                nontrivial_cluster_orders
                    .iter()
                    .map(|order| {
                        let delta = *order - mean;
                        delta * delta
                    })
                    .sum::<f64>()
                    / nontrivial_cluster_orders.len() as f64
            });
        let nontrivial_cluster_coverage_fraction =
            nontrivial_cluster_particle_count as f64 / count as f64;

        // Optional network topology analysis. Its cost is paid only when the caller
        // explicitly requests a full checkpoint snapshot.
        let graph_metrics = adjacency.map(|graph| {
            let mut clustering_sum = 0.0;
            for neighbors in &graph {
                let degree = neighbors.len();
                if degree < 2 {
                    continue;
                }
                let mut connected_neighbor_pairs = 0usize;
                for left in 0..degree {
                    for right in (left + 1)..degree {
                        if graph[neighbors[left]].contains(&neighbors[right]) {
                            connected_neighbor_pairs += 1;
                        }
                    }
                }
                let possible_neighbor_pairs = degree * (degree - 1) / 2;
                clustering_sum +=
                    connected_neighbor_pairs as f64 / possible_neighbor_pairs as f64;
            }
            let mean_clustering_coefficient = clustering_sum / count as f64;

            // Mean shortest path is computed over connected unordered pairs only. The
            // disconnected-pair burden is reported separately.
            let total_unordered_pairs = count.saturating_mul(count.saturating_sub(1)) / 2;
            let mut connected_pairs = 0usize;
            let mut shortest_path_sum = 0usize;
            for source in 0..count {
                let mut distances = vec![usize::MAX; count];
                let mut queue = VecDeque::with_capacity(count);
                distances[source] = 0;
                queue.push_back(source);
                while let Some(current) = queue.pop_front() {
                    let next_distance = distances[current] + 1;
                    for &neighbor in &graph[current] {
                        if distances[neighbor] == usize::MAX {
                            distances[neighbor] = next_distance;
                            queue.push_back(neighbor);
                        }
                    }
                }
                for distance in distances.iter().skip(source + 1) {
                    if *distance != usize::MAX {
                        connected_pairs += 1;
                        shortest_path_sum += *distance;
                    }
                }
            }
            let mean_shortest_path_length = if connected_pairs == 0 {
                None
            } else {
                Some(shortest_path_sum as f64 / connected_pairs as f64)
            };
            let connected_pair_fraction = if total_unordered_pairs == 0 {
                0.0
            } else {
                connected_pairs as f64 / total_unordered_pairs as f64
            };
            (
                mean_clustering_coefficient,
                mean_shortest_path_length,
                connected_pair_fraction,
            )
        });

        VicsekMetrics {
            tick: self.tick,
            update_rule: self.config.update_rule,
            particle_count: count,
            polar_order,
            mean_neighbors_excluding_self: mean_neighbors,
            neighbor_count_variance,
            local_order_coverage_fraction,
            mean_local_polar_order,
            local_polar_order_variance,
            mean_cluster_size,
            cluster_size_variance,
            mean_cluster_polar_order,
            cluster_polar_order_variance,
            nontrivial_cluster_coverage_fraction,
            mean_nontrivial_cluster_polar_order,
            nontrivial_cluster_polar_order_variance,
            topology_metrics_computed: graph_metrics.is_some(),
            mean_clustering_coefficient: graph_metrics.map(|metrics| metrics.0),
            mean_shortest_path_length: graph_metrics.and_then(|metrics| metrics.1),
            connected_pair_fraction: graph_metrics.map(|metrics| metrics.2),
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
    let dx = minimum_image(x_to - x_from, box_size);
    let dy = minimum_image(y_to - y_from, box_size);
    (dx, dy)
}

fn minimum_image(displacement: f64, box_size: f64) -> f64 {
    let wrapped = displacement.rem_euclid(box_size);
    if wrapped > 0.5 * box_size {
        wrapped - box_size
    } else {
        wrapped
    }
}

/// Add a signed displacement on a periodic interval without overflowing an intermediate
/// position-plus-displacement sum when the displacement is very large.
fn add_periodic(position: f64, displacement: f64, box_size: f64) -> f64 {
    let wrapped = displacement.rem_euclid(box_size);
    let distance_to_boundary = box_size - position;
    if wrapped >= distance_to_boundary {
        wrapped - distance_to_boundary
    } else {
        position + wrapped
    }
}

fn wrap_signed_angle(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(TAU) - std::f64::consts::PI
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
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
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
        let full_metrics = model.metrics();
        assert_eq!(full_metrics.mean_clustering_coefficient, Some(1.0));
        assert_eq!(full_metrics.mean_shortest_path_length, Some(1.0));
        assert_eq!(full_metrics.connected_pair_fraction, Some(1.0));
        assert_eq!(metrics.local_order_coverage_fraction, 1.0);
        assert!((metrics.mean_local_polar_order.unwrap() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn next_position_neighbour_rule_detects_newly_adjacent_particles() {
        let initial = vec![
            VicsekParticle { x: 1.0, y: 1.0, heading_rad: 0.0 },
            VicsekParticle {
                x: 3.0,
                y: 1.0,
                heading_rad: std::f64::consts::FRAC_PI_2,
            },
        ];
        let base = VicsekConfig {
            box_size: 10.0,
            interaction_radius: 1.5,
            speed: 0.8,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
        };
        let mut current_rule = VicsekModel::new(initial.clone(), base, 23).unwrap();
        let mut next_rule = VicsekModel::new(
            initial,
            VicsekConfig {
                update_rule: VicsekUpdateRule::NextPositionAlignment,
                ..base
            },
            23,
        )
        .unwrap();

        let current_metrics = current_rule.step().unwrap();
        let next_metrics = next_rule.step().unwrap();
        assert_eq!(
            current_metrics.update_rule,
            VicsekUpdateRule::CurrentPositionAlignment
        );
        assert_eq!(
            next_metrics.update_rule,
            VicsekUpdateRule::NextPositionAlignment
        );

        // The pair is 2 units apart at t (outside R=1.5), but their predicted positions
        // are within R after moving with their old headings. Both post-update snapshots
        // therefore have one connected component at t+dt; only NextPositionAlignment
        // uses that newly formed edge to calculate the current heading update.
        assert_eq!(current_metrics.cluster_count, 1);
        assert_eq!(next_metrics.cluster_count, 1);
        assert!((current_metrics.polar_order - std::f64::consts::FRAC_1_SQRT_2).abs() < 1.0e-12);
        assert!((next_metrics.polar_order - 1.0).abs() < 1.0e-12);
        assert!((next_metrics.mean_cluster_polar_order - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn cluster_and_local_order_detect_coherent_groups_with_opposing_headings() {
        let particles = vec![
            VicsekParticle { x: 1.0, y: 1.0, heading_rad: 0.0 },
            VicsekParticle { x: 1.5, y: 1.0, heading_rad: 0.0 },
            VicsekParticle { x: 10.0, y: 10.0, heading_rad: std::f64::consts::PI },
            VicsekParticle { x: 10.5, y: 10.0, heading_rad: std::f64::consts::PI },
        ];
        let model = VicsekModel::new(
            particles,
            VicsekConfig {
                box_size: 20.0,
                interaction_radius: 1.0,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            17,
        )
        .unwrap();

        let metrics = model.metrics();
        assert!(metrics.polar_order < 1.0e-12);
        assert!((metrics.mean_local_polar_order.unwrap() - 1.0).abs() < 1.0e-12);
        assert!((metrics.mean_cluster_polar_order - 1.0).abs() < 1.0e-12);
        assert_eq!(metrics.cluster_count, 2);
        assert_eq!(metrics.nontrivial_cluster_coverage_fraction, 1.0);
        assert!((metrics.mean_nontrivial_cluster_polar_order.unwrap() - 1.0).abs() < 1.0e-12);
        assert_eq!(metrics.nontrivial_cluster_polar_order_variance, Some(0.0));
        assert_eq!(metrics.mean_cluster_size, 2.0);
        assert_eq!(metrics.cluster_size_variance, 0.0);
        assert_eq!(metrics.largest_cluster_fraction, 0.5);
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
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            11,
        )
        .unwrap();
        let before = model.metrics().polar_order;
        let metrics = model.step().unwrap();

        assert_eq!(metrics.polar_order, before);
        assert_eq!(metrics.mean_neighbors_excluding_self, 0.0);
        assert_eq!(metrics.local_order_coverage_fraction, 0.0);
        assert_eq!(metrics.mean_local_polar_order, None);
        assert_eq!(metrics.local_polar_order_variance, None);
        assert_eq!(metrics.topology_metrics_computed, false);
        let full_metrics = model.metrics();
        assert_eq!(full_metrics.mean_clustering_coefficient, Some(0.0));
        assert_eq!(full_metrics.mean_shortest_path_length, None);
        assert_eq!(full_metrics.connected_pair_fraction, Some(0.0));
        assert_eq!(metrics.cluster_count, 5);
        assert_eq!(metrics.nontrivial_cluster_coverage_fraction, 0.0);
        assert_eq!(metrics.mean_nontrivial_cluster_polar_order, None);
        assert_eq!(metrics.nontrivial_cluster_polar_order_variance, None);
    }

    #[test]
    fn opposed_local_flocks_cancel_global_order_but_preserve_local_order() {
        let particles = vec![
            VicsekParticle { x: 2.0, y: 2.0, heading_rad: 0.0 },
            VicsekParticle { x: 2.5, y: 2.0, heading_rad: 0.0 },
            VicsekParticle { x: 2.0, y: 2.5, heading_rad: 0.0 },
            VicsekParticle { x: 12.0, y: 12.0, heading_rad: std::f64::consts::PI },
            VicsekParticle { x: 12.5, y: 12.0, heading_rad: std::f64::consts::PI },
            VicsekParticle { x: 12.0, y: 12.5, heading_rad: std::f64::consts::PI },
        ];
        let model = VicsekModel::new(
            particles,
            VicsekConfig {
                box_size: 20.0,
                interaction_radius: 1.0,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            13,
        )
        .unwrap();

        let metrics = model.fast_metrics();
        assert!(metrics.polar_order < 1.0e-12, "{metrics:?}");
        assert_eq!(metrics.cluster_count, 2);
        assert_eq!(metrics.largest_cluster_fraction, 0.5);
        assert_eq!(metrics.topology_metrics_computed, false);
        let full_metrics = model.metrics();
        assert_eq!(full_metrics.mean_clustering_coefficient, Some(1.0));
        assert_eq!(full_metrics.mean_shortest_path_length, Some(1.0));
        assert!((full_metrics.connected_pair_fraction.unwrap() - 0.4).abs() < 1.0e-12);
        assert_eq!(metrics.local_order_coverage_fraction, 1.0);
        assert!(
            (metrics.mean_local_polar_order.unwrap() - 1.0).abs() < 1.0e-12,
            "{metrics:?}"
        );
    }

    #[test]
    fn path_graph_has_known_shortest_path_and_zero_clustering() {
        let model = VicsekModel::new(
            vec![
                VicsekParticle { x: 2.0, y: 5.0, heading_rad: 0.0 },
                VicsekParticle { x: 3.0, y: 5.0, heading_rad: 0.0 },
                VicsekParticle { x: 4.0, y: 5.0, heading_rad: 0.0 },
            ],
            VicsekConfig {
                box_size: 10.0,
                interaction_radius: 1.01,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            29,
        )
        .unwrap();

        let metrics = model.metrics();
        assert_eq!(metrics.cluster_count, 1);
        assert_eq!(metrics.mean_shortest_path_length, Some(4.0 / 3.0));
        assert_eq!(metrics.mean_clustering_coefficient, Some(0.0));
        assert_eq!(metrics.connected_pair_fraction, Some(1.0));
    }

    #[test]
    fn all_to_all_heading_intervention_changes_untreated_particles_vs_sham() {
        let particles = (0..6)
            .map(|index| VicsekParticle {
                x: (index as f64 + 0.5) * 10.0 / 6.0,
                y: 5.0,
                heading_rad: 0.0,
            })
            .collect();
        let config = VicsekConfig {
            box_size: 10.0,
            interaction_radius: 8.0,
            speed: 0.0,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::NextPositionAlignment,
        };
        let baseline = VicsekModel::new(particles, config, 31).unwrap();
        let mut sham = baseline.clone();
        let mut treated = baseline;

        let receipt = treated.intervene_heading(0, std::f64::consts::FRAC_PI_2).unwrap();
        assert_eq!(receipt.tick, 0);
        assert_eq!(receipt.particle_index, 0);
        assert_eq!(receipt.previous_heading_rad, 0.0);
        assert_eq!(receipt.imposed_heading_rad, std::f64::consts::FRAC_PI_2);

        sham.step().unwrap();
        treated.step().unwrap();

        // Five unperturbed headings point along +x and one imposed heading points +y.
        // The all-to-all alignment rule makes every particle adopt atan2(1, 5).
        let expected = 1.0_f64.atan2(5.0);
        assert!((treated.particles()[1].heading_rad - expected).abs() < 1.0e-12);
        assert_eq!(sham.particles()[1].heading_rad, 0.0);
        assert!(treated.particles()[1].heading_rad > sham.particles()[1].heading_rad);
    }

    #[test]
    fn self_only_heading_intervention_does_not_change_other_particles() {
        let particles = vec![
            VicsekParticle { x: 1.0, y: 1.0, heading_rad: 0.0 },
            VicsekParticle { x: 5.0, y: 5.0, heading_rad: 1.0 },
            VicsekParticle { x: 9.0, y: 9.0, heading_rad: 2.0 },
        ];
        let config = VicsekConfig {
            box_size: 12.0,
            interaction_radius: 0.0,
            speed: 0.0,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
        };
        let baseline = VicsekModel::new(particles, config, 37).unwrap();
        let mut sham = baseline.clone();
        let mut treated = baseline;
        treated
            .intervene_heading(0, std::f64::consts::FRAC_PI_2)
            .unwrap();

        sham.step().unwrap();
        treated.step().unwrap();

        assert_eq!(treated.particles()[0].heading_rad, std::f64::consts::FRAC_PI_2);
        assert_eq!(treated.particles()[1].heading_rad, sham.particles()[1].heading_rad);
        assert_eq!(treated.particles()[2].heading_rad, sham.particles()[2].heading_rad);
    }

    #[test]
    fn shared_environmental_heading_field_correlates_isolated_particles_without_edges() {
        let particles = vec![
            VicsekParticle { x: 1.0, y: 1.0, heading_rad: 0.25 },
            VicsekParticle { x: 5.0, y: 5.0, heading_rad: 0.25 },
            VicsekParticle { x: 9.0, y: 9.0, heading_rad: 0.25 },
        ];
        let config = VicsekConfig {
            box_size: 12.0,
            interaction_radius: 0.0,
            speed: 0.0,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
        };
        let mut model = VicsekModel::new(particles, config, 43).unwrap();
        let sham = model.clone();
        let before_order = model.fast_metrics();

        let receipt = model
            .apply_common_heading_impulse(std::f64::consts::FRAC_PI_3)
            .unwrap();
        assert_eq!(receipt.tick, 0);
        assert_eq!(receipt.affected_particle_count, 3);
        assert!((receipt.heading_impulse_rad - std::f64::consts::FRAC_PI_3).abs() < 1.0e-12);

        let after = model.fast_metrics();
        assert_eq!(before_order.cluster_count, 3);
        assert_eq!(after.cluster_count, 3);
        assert_eq!(after.mean_neighbors_excluding_self, 0.0);
        assert!(after.mean_local_polar_order.is_none());
        assert!((after.polar_order - before_order.polar_order).abs() < 1.0e-12);

        let common_deltas: Vec<f64> = model
            .particles()
            .iter()
            .zip(sham.particles())
            .map(|(treated, control)| {
                wrap_signed_angle(treated.heading_rad - control.heading_rad)
            })
            .collect();
        assert!(common_deltas.iter().all(|delta| {
            (*delta - std::f64::consts::FRAC_PI_3).abs() < 1.0e-12
        }));
    }

    #[test]
    fn shared_heading_field_rejects_nonfinite_impulse() {
        let mut model = VicsekModel::random_uniform(3, VicsekConfig::default(), 47).unwrap();
        assert!(matches!(
            model.apply_common_heading_impulse(f64::INFINITY),
            Err(VicsekError::NonFiniteCommonFieldImpulse(value)) if value.is_infinite()
        ));
    }

    #[test]
    fn heading_intervention_rejects_nonfinite_and_out_of_range_inputs() {
        let mut model = VicsekModel::random_uniform(2, VicsekConfig::default(), 41).unwrap();
        assert!(matches!(
            model.intervene_heading(0, f64::NAN),
            Err(VicsekError::NonFiniteHeading { particle: 0 })
        ));
        assert!(matches!(
            model.intervene_heading(2, 0.0),
            Err(VicsekError::ParticleIndexOutOfBounds {
                particle: 2,
                particle_count: 2
            })
        ));
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
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
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
                && metrics.local_order_coverage_fraction.is_finite()
                && (0.0..=1.0).contains(&metrics.local_order_coverage_fraction)
                && metrics.mean_local_polar_order.is_none_or(|value| {
                    value.is_finite() && (0.0..=1.0).contains(&value)
                })
                && metrics.local_polar_order_variance.is_none_or(|value| {
                    value.is_finite() && value >= 0.0
                })
                && metrics.mean_cluster_size.is_finite()
                && metrics.cluster_size_variance.is_finite()
                && metrics.mean_cluster_polar_order.is_finite()
                && (0.0..=1.0).contains(&metrics.mean_cluster_polar_order)
                && metrics.cluster_polar_order_variance.is_finite()
                && metrics.topology_metrics_computed == false
                && metrics.mean_clustering_coefficient.is_none()
                && metrics.mean_shortest_path_length.is_none()
                && metrics.connected_pair_fraction.is_none()
                && metrics.largest_cluster_fraction.is_finite()
        }));
    }

    #[test]
    fn periodic_minimum_image_uses_short_path_across_box_boundary() {
        let (dx, dy) = periodic_displacement(0.1, 5.0, 9.9, 5.0, 10.0);
        assert!((dx + 0.2).abs() < 1.0e-12);
        assert_eq!(dy, 0.0);
    }

    #[test]
    fn large_finite_displacements_wrap_without_intermediate_overflow() {
        let mut model = VicsekModel::new(
            vec![VicsekParticle {
                x: 0.5,
                y: 0.5,
                heading_rad: 0.0,
            }],
            VicsekConfig {
                box_size: 10.0,
                interaction_radius: 0.0,
                speed: f64::MAX / 4.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            1,
        )
        .unwrap();
        let metrics = model.step().unwrap();
        assert_eq!(metrics.polar_order, 1.0);
        assert!(model.particles()[0].x.is_finite());
        assert!((0.0..10.0).contains(&model.particles()[0].x));
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

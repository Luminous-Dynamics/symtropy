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

use std::collections::{HashMap, VecDeque};
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

/// Geometry used by an agent to decide which nearby particles it can perceive.
///
/// MetricRadius is reciprocal. VisualSector is directional: visibility depends on the
/// focal particle's heading, a fixed visual-field offset, and a half-aperture. The
/// configured interaction radius remains a radial range limit for both modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VicsekPerception {
    MetricRadius,
    VisualSector {
        half_aperture_rad: f64,
        offset_rad: f64,
    },
}

impl VicsekPerception {
    fn normalized(self) -> Result<Self, VicsekError> {
        match self {
            Self::MetricRadius => Ok(Self::MetricRadius),
            Self::VisualSector { half_aperture_rad, offset_rad } => {
                if !half_aperture_rad.is_finite()
                    || !(0.0..=std::f64::consts::PI).contains(&half_aperture_rad)
                {
                    return Err(VicsekError::InvalidParameter {
                        name: "visual_half_aperture_rad",
                        value: half_aperture_rad,
                    });
                }
                if !offset_rad.is_finite() {
                    return Err(VicsekError::InvalidParameter {
                        name: "visual_offset_rad",
                        value: offset_rad,
                    });
                }
                Ok(Self::VisualSector {
                    half_aperture_rad,
                    offset_rad: wrap_angle(offset_rad),
                })
            }
        }
    }
}

/// Graph convention used for cluster and topology metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VicsekGraphSemantics {
    /// Metric-radius interactions are reciprocal and the interaction graph is undirected.
    UndirectedMetric,
    /// For directional perception, components and graph topology use the weakly connected
    /// projection: an undirected edge exists when either endpoint sees the other.
    WeaklyConnectedProjectionOfDirectedVisibility,
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
    /// Agent perception geometry used to construct neighbour sets.
    pub perception: VicsekPerception,
    /// Graph semantics used for component and topology metrics.
    pub graph_semantics: VicsekGraphSemantics,
    pub particle_count: usize,
    /// Magnitude of the mean unit heading vector, in [0, 1].
    pub polar_order: f64,
    /// Mean number of other particles visible to each focal particle (mean out-degree).
    pub mean_neighbors_excluding_self: f64,
    /// Population variance of focal-agent visible-neighbour counts (out-degree variance).
    pub neighbor_count_variance: f64,
    /// Number of directed visibility edges across all unordered particle pairs.
    pub directed_interaction_edge_count: usize,
    /// Number of unordered pairs for which visibility is mutual.
    pub reciprocal_interaction_pair_count: usize,
    /// Fraction of directed visibility edges that have a reciprocal counterpart; None if no edges.
    pub interaction_reciprocity_fraction: Option<f64>,
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

/// Per-tick response from a paired heading-intervention replay.
///
/// Treatment and sham begin as exact clones, including the PRNG state. The same random
/// variates are therefore applied to corresponding particles on every subsequent tick;
/// differences in untreated particles are attributable to the intervention under this
/// declared synthetic model and update rule, not independent noise realizations.
#[derive(Debug, Clone, PartialEq)]
pub struct PairedInterventionStep {
    pub tick: u64,
    pub sham_metrics: VicsekMetrics,
    pub treatment_metrics: VicsekMetrics,
    /// Mean absolute circular heading difference for all untreated particles.
    /// None when the intervention targets the only particle.
    pub mean_untreated_heading_effect_rad: Option<f64>,
    /// Fraction of untreated particles whose heading differs by more than 1e-12 radians.
    pub untreated_heading_response_fraction: Option<f64>,
    /// Mean minimum-image displacement difference for untreated particles.
    pub mean_untreated_position_effect: Option<f64>,
    /// Fraction of untreated particles displaced by more than 1e-12 box-length units.
    pub untreated_position_response_fraction: Option<f64>,
}

/// Reproducible treatment/sham trajectory pair for one direct heading intervention.
#[derive(Debug, Clone, PartialEq)]
pub struct PairedInterventionResult {
    pub intervention: HeadingIntervention,
    pub update_rule: VicsekUpdateRule,
    pub perception: VicsekPerception,
    pub initial_sham_metrics: VicsekMetrics,
    pub initial_treatment_metrics: VicsekMetrics,
    pub common_random_numbers: bool,
    pub response_threshold: f64,
    pub steps: Vec<PairedInterventionStep>,
}

/// Periodic two-dimensional Vicsek active-particle reference model.
#[derive(Debug, Clone, PartialEq)]
pub struct VicsekModel {
    config: VicsekConfig,
    perception: VicsekPerception,
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
        Self::new_with_perception(particles, config, seed, VicsekPerception::MetricRadius)
    }

    /// Construct a model with an explicit metric or visual-sector perception rule.
    pub fn new_with_perception(
        particles: Vec<VicsekParticle>,
        config: VicsekConfig,
        seed: u64,
        perception: VicsekPerception,
    ) -> Result<Self, VicsekError> {
        Self::from_particles_and_rng(
            particles,
            config,
            XorShift64::new(seed),
            perception,
        )
    }

    fn from_particles_and_rng(
        mut particles: Vec<VicsekParticle>,
        config: VicsekConfig,
        rng: XorShift64,
        perception: VicsekPerception,
    ) -> Result<Self, VicsekError> {
        config.validate()?;
        let perception = perception.normalized()?;
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
            perception,
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
        Self::random_uniform_with_perception(
            particle_count,
            config,
            seed,
            VicsekPerception::MetricRadius,
        )
    }

    /// Create a seeded random population with an explicit perception geometry.
    pub fn random_uniform_with_perception(
        particle_count: usize,
        config: VicsekConfig,
        seed: u64,
        perception: VicsekPerception,
    ) -> Result<Self, VicsekError> {
        config.validate()?;
        let perception = perception.normalized()?;
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
        // Continue the initialization stream for later angular-noise draws.
        Self::from_particles_and_rng(particles, config, rng, perception)
    }

    pub const fn config(&self) -> VicsekConfig {
        self.config
    }

    pub const fn perception(&self) -> VicsekPerception {
        self.perception
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn particles(&self) -> &[VicsekParticle] {
        &self.particles
    }

    /// Whether a focal particle sees a target at the supplied positions.
    ///
    /// The target position is already selected under the configured neighbour-time rule.
    /// Visual-sector perception uses the focal heading at the beginning of the update.
    fn perceives(
        &self,
        focal_position: (f64, f64),
        focal_heading_rad: f64,
        target_position: (f64, f64),
    ) -> bool {
        let (dx, dy) = periodic_displacement(
            focal_position.0,
            focal_position.1,
            target_position.0,
            target_position.1,
            self.config.box_size,
        );
        if dx.hypot(dy) > self.config.interaction_radius {
            return false;
        }
        match self.perception {
            VicsekPerception::MetricRadius => true,
            VicsekPerception::VisualSector {
                half_aperture_rad,
                offset_rad,
            } => {
                if dx.abs() <= f64::EPSILON && dy.abs() <= f64::EPSILON {
                    return true;
                }
                let bearing = dy.atan2(dx);
                let visual_axis = wrap_angle(focal_heading_rad + offset_rad);
                wrap_signed_angle(bearing - visual_axis).abs() <= half_aperture_rad
            }
        }
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

        // The periodic cell index prunes distant candidates but never changes the
        // exact visibility predicate. Candidate indices are sorted before summation so
        // floating-point accumulation order matches the brute-force reference.
        let spatial_index = PeriodicNeighborIndex::new(
            &neighbour_positions,
            self.config.box_size,
            self.config.interaction_radius,
        );

        let mut next = Vec::with_capacity(previous.len());
        for (index, particle) in previous.iter().enumerate() {
            let mut sum_cos = 0.0;
            let mut sum_sin = 0.0;
            let candidates = spatial_index.as_ref().map_or_else(
                || (0..previous.len()).collect::<Vec<_>>(),
                |grid| grid.candidates(neighbour_positions[index]),
            );
            for neighbor_index in candidates {
                let neighbor = previous[neighbor_index];
                let (neighbor_x, neighbor_y) = neighbour_positions[neighbor_index];
                if neighbor_index == index
                    || self.perceives(
                        neighbour_positions[index],
                        particle.heading_rad,
                        (neighbor_x, neighbor_y),
                    )
                {
                    // Visibility is focal-agent-specific. In visual-sector mode, A may see
                    // B without B seeing A; each focal agent still aligns on neighbour
                    // headings from the frozen previous state.
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
        let mut directed_interaction_edge_count = 0usize;
        let mut reciprocal_interaction_pair_count = 0usize;
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
        let positions: Vec<(f64, f64)> = self
            .particles
            .iter()
            .map(|particle| (particle.x, particle.y))
            .collect();
        let spatial_index = PeriodicNeighborIndex::new(
            &positions,
            self.config.box_size,
            self.config.interaction_radius,
        );

        for left in 0..count {
            let candidates = spatial_index.as_ref().map_or_else(
                || (left + 1..count).collect::<Vec<_>>(),
                |grid| {
                    grid.candidates(positions[left])
                        .into_iter()
                        .filter(|right| *right > left)
                        .collect()
                },
            );
            for right in candidates {
                let a = self.particles[left];
                let b = self.particles[right];
                let left_sees_right = self.perceives((a.x, a.y), a.heading_rad, (b.x, b.y));
                let right_sees_left = self.perceives((b.x, b.y), b.heading_rad, (a.x, a.y));

                if left_sees_right {
                    neighbor_counts[left] += 1;
                    directed_interaction_edge_count += 1;
                    local_sum_cos[left] += b.heading_rad.cos();
                    local_sum_sin[left] += b.heading_rad.sin();
                }
                if right_sees_left {
                    neighbor_counts[right] += 1;
                    directed_interaction_edge_count += 1;
                    local_sum_cos[right] += a.heading_rad.cos();
                    local_sum_sin[right] += a.heading_rad.sin();
                }
                if left_sees_right && right_sees_left {
                    reciprocal_interaction_pair_count += 1;
                }

                // Topology statistics use the weakly connected projection for visual-sector
                // perception: an undirected graph edge exists if either directed visibility
                // relation exists. Out-degree and local order remain focal-agent-specific.
                if left_sees_right || right_sees_left {
                    if let Some(graph) = &mut adjacency {
                        graph[left].push(right);
                        graph[right].push(left);
                    }
                    union(&mut parents, left, right);
                }
            }
        }

        let interaction_reciprocity_fraction = if directed_interaction_edge_count == 0 {
            None
        } else {
            Some(
                (2 * reciprocal_interaction_pair_count) as f64
                    / directed_interaction_edge_count as f64,
            )
        };
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
            perception: self.perception,
            graph_semantics: match self.perception {
                VicsekPerception::MetricRadius => VicsekGraphSemantics::UndirectedMetric,
                VicsekPerception::VisualSector { .. } => {
                    VicsekGraphSemantics::WeaklyConnectedProjectionOfDirectedVisibility
                }
            },
            particle_count: count,
            polar_order,
            mean_neighbors_excluding_self: mean_neighbors,
            neighbor_count_variance,
            directed_interaction_edge_count,
            reciprocal_interaction_pair_count,
            interaction_reciprocity_fraction,
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

/// Run a matched sham and treatment trajectory from the same pre-intervention state.
///
/// The function clones `baseline` twice. Only the treatment copy receives the
/// heading intervention. Since the intervention does not consume random numbers, both
/// simulations retain identical PRNG state and receive common random numbers at every
/// subsequent update. This is the preferred synthetic counterfactual primitive for OLI.
///
/// The reported downstream response excludes the directly treated particle. Response
/// fractions use a declared numerical threshold of 1e-12 in radians and box-length units.
/// A real experimental adapter must replace this with instrument-specific uncertainty.
pub fn run_paired_heading_intervention(
    baseline: &VicsekModel,
    particle_index: usize,
    imposed_heading_rad: f64,
    steps: usize,
) -> Result<PairedInterventionResult, VicsekError> {
    const RESPONSE_THRESHOLD: f64 = 1.0e-12;

    let mut sham = baseline.clone();
    let mut treatment = baseline.clone();
    let initial_sham_metrics = sham.fast_metrics();
    let intervention = treatment.intervene_heading(particle_index, imposed_heading_rad)?;
    let initial_treatment_metrics = treatment.fast_metrics();
    let update_rule = baseline.config.update_rule;
    let perception = baseline.perception;
    let untreated_count = baseline.particles.len().saturating_sub(1);
    let mut response_rows = Vec::with_capacity(steps);

    for _ in 0..steps {
        let sham_metrics = sham.step()?;
        let treatment_metrics = treatment.step()?;
        let mut heading_effect_sum = 0.0;
        let mut position_effect_sum = 0.0;
        let mut heading_responders = 0usize;
        let mut position_responders = 0usize;

        for index in 0..baseline.particles.len() {
            if index == particle_index {
                continue;
            }
            let sham_particle = sham.particles[index];
            let treatment_particle = treatment.particles[index];
            let heading_effect = wrap_signed_angle(
                treatment_particle.heading_rad - sham_particle.heading_rad,
            )
            .abs();
            let (dx, dy) = periodic_displacement(
                sham_particle.x,
                sham_particle.y,
                treatment_particle.x,
                treatment_particle.y,
                baseline.config.box_size,
            );
            let position_effect = dx.hypot(dy);
            heading_effect_sum += heading_effect;
            position_effect_sum += position_effect;
            if heading_effect > RESPONSE_THRESHOLD {
                heading_responders += 1;
            }
            if position_effect > RESPONSE_THRESHOLD {
                position_responders += 1;
            }
        }

        let (mean_untreated_heading_effect_rad, untreated_heading_response_fraction,
            mean_untreated_position_effect, untreated_position_response_fraction) =
            if untreated_count == 0 {
                (None, None, None, None)
            } else {
                (
                    Some(heading_effect_sum / untreated_count as f64),
                    Some(heading_responders as f64 / untreated_count as f64),
                    Some(position_effect_sum / untreated_count as f64),
                    Some(position_responders as f64 / untreated_count as f64),
                )
            };

        response_rows.push(PairedInterventionStep {
            tick: treatment.tick,
            sham_metrics,
            treatment_metrics,
            mean_untreated_heading_effect_rad,
            untreated_heading_response_fraction,
            mean_untreated_position_effect,
            untreated_position_response_fraction,
        });
    }

    Ok(PairedInterventionResult {
        intervention,
        update_rule,
        perception,
        initial_sham_metrics,
        initial_treatment_metrics,
        common_random_numbers: true,
        response_threshold: RESPONSE_THRESHOLD,
        steps: response_rows,
    })
}

/// Deterministic periodic cell index for candidate-neighbour pruning.
///
/// A cell is at least as wide as the interaction radius, so the 3×3 stencil around a
/// focal cell contains every possible neighbour. Extra candidates are filtered by the
/// exact minimum-image distance/visual-sector predicate. The cell map is sparse; for
/// extremely small radii where the required grid would be unreasonable, construction
/// returns None and the caller uses the all-pairs reference scan.
#[derive(Debug, Clone)]
struct PeriodicNeighborIndex {
    cell_count: usize,
    cell_width: f64,
    buckets: HashMap<(usize, usize), Vec<usize>>,
}

impl PeriodicNeighborIndex {
    const MAX_CELL_COUNT_PER_AXIS: usize = 1_000_000;

    fn new(
        positions: &[(f64, f64)],
        box_size: f64,
        interaction_radius: f64,
    ) -> Option<Self> {
        if interaction_radius <= 0.0 {
            return None;
        }
        let ratio = (box_size / interaction_radius).floor().max(1.0);
        if !ratio.is_finite() || ratio > Self::MAX_CELL_COUNT_PER_AXIS as f64 {
            return None;
        }
        let cell_count = ratio as usize;
        let cell_width = box_size / cell_count as f64;
        if !cell_width.is_finite() || cell_width < interaction_radius {
            return None;
        }

        let mut buckets: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for (index, &(x, y)) in positions.iter().enumerate() {
            let cell = (
                cell_coordinate(x, cell_width, cell_count),
                cell_coordinate(y, cell_width, cell_count),
            );
            buckets.entry(cell).or_default().push(index);
        }
        Some(Self {
            cell_count,
            cell_width,
            buckets,
        })
    }

    fn candidates(&self, position: (f64, f64)) -> Vec<usize> {
        let center_x = cell_coordinate(position.0, self.cell_width, self.cell_count) as isize;
        let center_y = cell_coordinate(position.1, self.cell_width, self.cell_count) as isize;
        let count = self.cell_count as isize;
        let mut cells = Vec::with_capacity(9);
        for offset_y in -1isize..=1 {
            for offset_x in -1isize..=1 {
                let cell = (
                    (center_x + offset_x).rem_euclid(count) as usize,
                    (center_y + offset_y).rem_euclid(count) as usize,
                );
                cells.push(cell);
            }
        }
        cells.sort_unstable();
        cells.dedup();

        let mut candidates = Vec::new();
        for cell in cells {
            if let Some(indices) = self.buckets.get(&cell) {
                candidates.extend(indices.iter().copied());
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        candidates
    }
}

fn cell_coordinate(position: f64, cell_width: f64, cell_count: usize) -> usize {
    ((position / cell_width).floor() as usize).min(cell_count - 1)
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
        assert_eq!(metrics.directed_interaction_edge_count, 30);
        assert_eq!(metrics.reciprocal_interaction_pair_count, 15);
        assert_eq!(metrics.interaction_reciprocity_fraction, Some(1.0));
        let full_metrics = model.metrics();
        assert_eq!(full_metrics.mean_clustering_coefficient, Some(1.0));
        assert_eq!(full_metrics.mean_shortest_path_length, Some(1.0));
        assert_eq!(full_metrics.connected_pair_fraction, Some(1.0));
        assert_eq!(metrics.local_order_coverage_fraction, 1.0);
        assert!((metrics.mean_local_polar_order.unwrap() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn visual_sector_forward_and_flipped_offsets_select_opposite_neighbours() {
        let config = VicsekConfig {
            box_size: 12.0,
            interaction_radius: 3.0,
            speed: 0.0,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
        };
        let visual = VicsekModel::new_with_perception(
            vec![VicsekParticle { x: 5.0, y: 5.0, heading_rad: 0.0 }],
            config,
            61,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_4,
                offset_rad: 0.0,
            },
        )
        .unwrap();

        assert!(visual.perceives((5.0, 5.0), 0.0, (6.0, 5.0)));
        assert!(!visual.perceives((5.0, 5.0), 0.0, (4.0, 5.0)));

        let flipped = VicsekModel::new_with_perception(
            vec![VicsekParticle { x: 5.0, y: 5.0, heading_rad: 0.0 }],
            config,
            61,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_4,
                offset_rad: std::f64::consts::PI,
            },
        )
        .unwrap();

        assert!(!flipped.perceives((5.0, 5.0), 0.0, (6.0, 5.0)));
        assert!(flipped.perceives((5.0, 5.0), 0.0, (4.0, 5.0)));
    }

    #[test]
    fn visual_sector_builds_directed_out_neighbours_and_weak_cluster_projection() {
        let model = VicsekModel::new_with_perception(
            vec![
                VicsekParticle { x: 5.0, y: 5.0, heading_rad: 0.0 },
                VicsekParticle { x: 6.0, y: 5.0, heading_rad: 0.0 },
            ],
            VicsekConfig {
                box_size: 12.0,
                interaction_radius: 3.0,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            67,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_4,
                offset_rad: 0.0,
            },
        )
        .unwrap();

        let metrics = model.metrics();
        assert_eq!(
            metrics.perception,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_4,
                offset_rad: 0.0,
            }
        );
        assert_eq!(
            metrics.graph_semantics,
            VicsekGraphSemantics::WeaklyConnectedProjectionOfDirectedVisibility
        );
        // The left particle sees the right particle; the right particle faces away.
        assert_eq!(metrics.mean_neighbors_excluding_self, 0.5);
        assert_eq!(metrics.neighbor_count_variance, 0.25);
        assert_eq!(metrics.directed_interaction_edge_count, 1);
        assert_eq!(metrics.reciprocal_interaction_pair_count, 0);
        assert_eq!(metrics.interaction_reciprocity_fraction, Some(0.0));
        assert_eq!(metrics.local_order_coverage_fraction, 0.5);
        assert_eq!(metrics.mean_local_polar_order, Some(1.0));
        // For topology only, either directed edge connects the weak component.
        assert_eq!(metrics.cluster_count, 1);
        assert_eq!(metrics.mean_cluster_size, 2.0);
        assert_eq!(metrics.mean_shortest_path_length, Some(1.0));
        assert_eq!(metrics.connected_pair_fraction, Some(1.0));
    }

    #[test]
    fn asymmetric_visual_visibility_predicts_directional_intervention_effect() {
        let baseline = VicsekModel::new_with_perception(
            vec![
                VicsekParticle { x: 5.0, y: 5.0, heading_rad: 0.0 },
                VicsekParticle { x: 6.0, y: 5.0, heading_rad: 0.0 },
            ],
            VicsekConfig {
                box_size: 12.0,
                interaction_radius: 3.0,
                speed: 0.0,
                dt: 1.0,
                angular_noise_width_rad: 0.0,
                update_rule: VicsekUpdateRule::CurrentPositionAlignment,
            },
            73,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_4,
                offset_rad: 0.0,
            },
        )
        .unwrap();

        // Particle 0 sees particle 1, but particle 1 does not see particle 0.
        let metrics = baseline.metrics();
        assert_eq!(metrics.directed_interaction_edge_count, 1);
        assert_eq!(metrics.reciprocal_interaction_pair_count, 0);
        assert_eq!(metrics.interaction_reciprocity_fraction, Some(0.0));

        // Change the unseen particle (0): particle 1 must remain exactly on its sham path.
        let unseen_intervention = run_paired_heading_intervention(
            &baseline,
            0,
            std::f64::consts::FRAC_PI_2,
            1,
        )
        .unwrap();
        assert_eq!(
            unseen_intervention.steps[0].mean_untreated_heading_effect_rad,
            Some(0.0)
        );
        assert_eq!(
            unseen_intervention.steps[0].untreated_heading_response_fraction,
            Some(0.0)
        );

        // Change the visible particle (1): particle 0 sees its new heading and rotates
        // halfway toward it, so the downstream effect is analytically PI/4.
        let visible_intervention = run_paired_heading_intervention(
            &baseline,
            1,
            std::f64::consts::FRAC_PI_2,
            1,
        )
        .unwrap();
        assert!(
            (visible_intervention.steps[0].mean_untreated_heading_effect_rad
                .unwrap()
                - std::f64::consts::FRAC_PI_4)
            .abs()
                < 1.0e-12
        );
        assert_eq!(
            visible_intervention.steps[0].untreated_heading_response_fraction,
            Some(1.0)
        );
    }

    #[test]
    fn visual_sector_rejects_invalid_aperture_and_offset() {
        let particles = vec![VicsekParticle { x: 1.0, y: 1.0, heading_rad: 0.0 }];
        let config = VicsekConfig::default();
        assert!(matches!(
            VicsekModel::new_with_perception(
                particles.clone(),
                config,
                71,
                VicsekPerception::VisualSector {
                    half_aperture_rad: std::f64::consts::PI + 0.01,
                    offset_rad: 0.0,
                }
            ),
            Err(VicsekError::InvalidParameter {
                name: "visual_half_aperture_rad",
                ..
            })
        ));
        assert!(matches!(
            VicsekModel::new_with_perception(
                particles,
                config,
                71,
                VicsekPerception::VisualSector {
                    half_aperture_rad: 0.5,
                    offset_rad: f64::NAN,
                }
            ),
            Err(VicsekError::InvalidParameter {
                name: "visual_offset_rad",
                ..
            })
        ));
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
    fn paired_intervention_replay_matches_all_to_all_analytic_response_and_replays() {
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
            speed: 0.25,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
        };
        let baseline = VicsekModel::new(particles, config, 53).unwrap();

        let first = run_paired_heading_intervention(
            &baseline,
            0,
            std::f64::consts::FRAC_PI_2,
            5,
        )
        .unwrap();
        let second = run_paired_heading_intervention(
            &baseline,
            0,
            std::f64::consts::FRAC_PI_2,
            5,
        )
        .unwrap();

        assert_eq!(first, second);
        assert!(first.common_random_numbers);
        assert_eq!(first.update_rule, VicsekUpdateRule::CurrentPositionAlignment);
        assert_eq!(first.intervention.tick, 0);
        assert_eq!(first.steps.len(), 5);
        let first_step = &first.steps[0];
        assert_eq!(first_step.tick, 1);
        assert!((first_step.treatment_metrics.polar_order - 1.0).abs() < 1.0e-12);
        assert!(first_step.sham_metrics.polar_order > 1.0 - 1.0e-12);
        assert!(
            (first_step.mean_untreated_heading_effect_rad.unwrap()
                - 1.0_f64.atan2(5.0))
            .abs()
                < 1.0e-12
        );
        assert_eq!(first_step.untreated_heading_response_fraction, Some(1.0));
        assert!(first_step.mean_untreated_position_effect.unwrap() > 0.0);
        assert_eq!(first_step.untreated_position_response_fraction, Some(1.0));
    }

    #[test]
    fn paired_intervention_uses_common_noise_and_has_zero_untreated_effect_without_coupling() {
        let particles = vec![
            VicsekParticle { x: 1.0, y: 1.0, heading_rad: 0.0 },
            VicsekParticle { x: 5.0, y: 5.0, heading_rad: 1.0 },
            VicsekParticle { x: 9.0, y: 9.0, heading_rad: 2.0 },
        ];
        let config = VicsekConfig {
            box_size: 12.0,
            interaction_radius: 0.0,
            speed: 0.1,
            dt: 1.0,
            angular_noise_width_rad: 0.2,
            update_rule: VicsekUpdateRule::NextPositionAlignment,
        };
        let baseline = VicsekModel::new(particles, config, 59).unwrap();
        let result = run_paired_heading_intervention(
            &baseline,
            0,
            std::f64::consts::FRAC_PI_2,
            20,
        )
        .unwrap();

        assert!(result.common_random_numbers);
        assert_eq!(result.update_rule, VicsekUpdateRule::NextPositionAlignment);
        assert!(result.steps.iter().all(|row| {
            row.mean_untreated_heading_effect_rad == Some(0.0)
                && row.untreated_heading_response_fraction == Some(0.0)
                && row.mean_untreated_position_effect == Some(0.0)
                && row.untreated_position_response_fraction == Some(0.0)
        }));
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
    fn periodic_cell_index_never_omits_brute_force_radius_neighbours() {
        let fixtures = [
            (10.0, 1.0, 100, true),
            (10.0, 4.9, 50, true),
            (10.0, 8.0, 20, true),
            (10.0, 0.0, 20, false),
            (10.0, 1.0e-12, 20, false),
            (10.0, 11.0, 20, false),
        ];
        for (box_size, radius, count, expect_index) in fixtures {
            let mut rng = XorShift64::new(0xC311_1D3);
            let positions: Vec<(f64, f64)> = (0..count)
                .map(|_| (rng.next_unit() * box_size, rng.next_unit() * box_size))
                .collect();
            let index = PeriodicNeighborIndex::new(&positions, box_size, radius);
            assert_eq!(index.is_some(), expect_index, "box={box_size} radius={radius}");
            if let Some(index) = index {
                for (focal, &position) in positions.iter().enumerate() {
                    let candidates = index.candidates(position);
                    assert!(candidates.windows(2).all(|pair| pair[0] < pair[1]));
                    assert!(candidates.contains(&focal));
                    for (target, &target_position) in positions.iter().enumerate() {
                        let (dx, dy) = periodic_displacement(
                            position.0,
                            position.1,
                            target_position.0,
                            target_position.1,
                            box_size,
                        );
                        if dx.hypot(dy) <= radius {
                            assert!(
                                candidates.binary_search(&target).is_ok(),
                                "radius neighbour omitted: box={box_size} radius={radius} focal={focal} target={target}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn indexed_metrics_match_bruteforce_directed_visibility_statistics() {
        let config = VicsekConfig {
            box_size: 20.0,
            interaction_radius: 3.2,
            speed: 0.0,
            dt: 1.0,
            angular_noise_width_rad: 0.0,
            update_rule: VicsekUpdateRule::CurrentPositionAlignment,
        };
        let particles: Vec<VicsekParticle> = (0..97)
            .map(|index| VicsekParticle {
                x: ((index * 19 % 101) as f64 / 101.0) * config.box_size,
                y: ((index * 37 % 103) as f64 / 103.0) * config.box_size,
                heading_rad: ((index * 29 % 107) as f64 / 107.0) * TAU,
            })
            .collect();

        for perception in [
            VicsekPerception::MetricRadius,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_5,
                offset_rad: std::f64::consts::PI,
            },
        ] {
            let model = VicsekModel::new_with_perception(
                particles.clone(),
                config,
                0xA11CE,
                perception,
            )
            .unwrap();
            let indexed = model.fast_metrics();

            let count = model.particles.len();
            let mut degrees = vec![0usize; count];
            let mut directed_edges = 0usize;
            let mut reciprocal_pairs = 0usize;
            let mut local_orders = Vec::new();
            for left in 0..count {
                for right in (left + 1)..count {
                    let a = model.particles[left];
                    let b = model.particles[right];
                    let left_sees_right =
                        model.perceives((a.x, a.y), a.heading_rad, (b.x, b.y));
                    let right_sees_left =
                        model.perceives((b.x, b.y), b.heading_rad, (a.x, a.y));
                    if left_sees_right {
                        degrees[left] += 1;
                        directed_edges += 1;
                    }
                    if right_sees_left {
                        degrees[right] += 1;
                        directed_edges += 1;
                    }
                    if left_sees_right && right_sees_left {
                        reciprocal_pairs += 1;
                    }
                }
            }

            for focal in 0..count {
                if degrees[focal] == 0 {
                    continue;
                }
                let a = model.particles[focal];
                let mut sum_cos = a.heading_rad.cos();
                let mut sum_sin = a.heading_rad.sin();
                for target in 0..count {
                    if target == focal {
                        continue;
                    }
                    let b = model.particles[target];
                    if model.perceives((a.x, a.y), a.heading_rad, (b.x, b.y)) {
                        sum_cos += b.heading_rad.cos();
                        sum_sin += b.heading_rad.sin();
                    }
                }
                local_orders.push(
                    (sum_cos.hypot(sum_sin) / (degrees[focal] + 1) as f64).clamp(0.0, 1.0),
                );
            }
            let mean_degree = degrees.iter().sum::<usize>() as f64 / count as f64;
            let degree_variance = degrees
                .iter()
                .map(|degree| {
                    let delta = *degree as f64 - mean_degree;
                    delta * delta
                })
                .sum::<f64>()
                / count as f64;
            let local_mean = if local_orders.is_empty() {
                None
            } else {
                Some(local_orders.iter().sum::<f64>() / local_orders.len() as f64)
            };
            let local_variance = local_mean.map(|mean| {
                local_orders
                    .iter()
                    .map(|order| {
                        let delta = *order - mean;
                        delta * delta
                    })
                    .sum::<f64>()
                    / local_orders.len() as f64
            });

            assert_eq!(indexed.directed_interaction_edge_count, directed_edges);
            assert_eq!(indexed.reciprocal_interaction_pair_count, reciprocal_pairs);
            assert_eq!(indexed.mean_neighbors_excluding_self, mean_degree);
            assert!((indexed.neighbor_count_variance - degree_variance).abs() < 1.0e-12);
            assert_eq!(
                indexed.local_order_coverage_fraction,
                local_orders.len() as f64 / count as f64
            );
            assert_eq!(indexed.mean_local_polar_order, local_mean);
            assert_eq!(indexed.local_polar_order_variance, local_variance);
        }
    }

    #[test]
    fn indexed_step_matches_brute_force_for_metric_and_visual_perception() {
        let config = VicsekConfig {
            box_size: 12.0,
            interaction_radius: 2.5,
            speed: 0.35,
            dt: 1.0,
            angular_noise_width_rad: 0.4,
            update_rule: VicsekUpdateRule::NextPositionAlignment,
        };
        let particles: Vec<VicsekParticle> = (0..32)
            .map(|index| VicsekParticle {
                x: ((index * 7 % 31) as f64 / 31.0) * config.box_size,
                y: ((index * 13 % 29) as f64 / 29.0) * config.box_size,
                heading_rad: ((index * 17 % 37) as f64 / 37.0) * TAU,
            })
            .collect();
        for perception in [
            VicsekPerception::MetricRadius,
            VicsekPerception::VisualSector {
                half_aperture_rad: std::f64::consts::FRAC_PI_3,
                offset_rad: std::f64::consts::PI,
            },
        ] {
            let model = VicsekModel::new_with_perception(
                particles.clone(),
                config,
                0xC311_1D3,
                perception,
            )
            .unwrap();
            let indexed = model.clone();
            let mut expected = model.clone();
            let mut actual = indexed.clone();

            // Test-only all-pairs reference update preserving the same PRNG stream.
            let previous = expected.particles.clone();
            let distance = expected.config.speed * expected.config.dt;
            let neighbour_positions: Vec<(f64, f64)> = match expected.config.update_rule {
                VicsekUpdateRule::CurrentPositionAlignment => previous
                    .iter()
                    .map(|particle| (particle.x, particle.y))
                    .collect(),
                VicsekUpdateRule::NextPositionAlignment => previous
                    .iter()
                    .map(|particle| (
                        add_periodic(particle.x, distance * particle.heading_rad.cos(), expected.config.box_size),
                        add_periodic(particle.y, distance * particle.heading_rad.sin(), expected.config.box_size),
                    ))
                    .collect(),
            };
            let mut next = Vec::with_capacity(previous.len());
            for (focal_index, focal) in previous.iter().enumerate() {
                let mut sum_cos = 0.0;
                let mut sum_sin = 0.0;
                for (target_index, target) in previous.iter().enumerate() {
                    if target_index == focal_index
                        || expected.perceives(
                            neighbour_positions[focal_index],
                            focal.heading_rad,
                            neighbour_positions[target_index],
                        )
                    {
                        sum_cos += target.heading_rad.cos();
                        sum_sin += target.heading_rad.sin();
                    }
                }
                let alignment = if sum_cos.abs() <= f64::EPSILON && sum_sin.abs() <= f64::EPSILON {
                    focal.heading_rad
                } else {
                    sum_sin.atan2(sum_cos)
                };
                let noise = expected.config.angular_noise_width_rad * (expected.rng.next_unit() - 0.5);
                let heading = wrap_angle(alignment + noise);
                let (x, y) = match expected.config.update_rule {
                    VicsekUpdateRule::CurrentPositionAlignment => (
                        add_periodic(focal.x, distance * heading.cos(), expected.config.box_size),
                        add_periodic(focal.y, distance * heading.sin(), expected.config.box_size),
                    ),
                    VicsekUpdateRule::NextPositionAlignment => neighbour_positions[focal_index],
                };
                next.push(VicsekParticle { x, y, heading_rad: heading });
            }
            expected.particles = next;
            expected.tick += 1;

            actual.step().unwrap();
            assert_eq!(actual.particles, expected.particles);
            assert_eq!(actual.rng, expected.rng);
        }
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

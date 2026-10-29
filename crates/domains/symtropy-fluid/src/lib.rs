// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Correctness-first weakly-compressible SPH reference path.
//!
//! This CPU reference uses density summation, a linearized equation of state,
//! symmetric pressure acceleration, a regularized physical-viscosity operator,
//! explicit timestep limits, and a transactional semi-implicit Euler update.
//! Neighbor discovery is deliberately O(N²), serving as the oracle for later
//! accelerated neighbor search. This is not yet a production CFD solver.

pub mod kernel;
pub use kernel::{Poly6Kernel, SphKernelError};

use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

const ACOUSTIC_CFL: f64 = 0.25;
const VISCOSITY_CFL: f64 = 0.125;
const VISCOSITY_REGULARIZATION: f64 = 0.01;

#[derive(Clone, Debug, PartialEq)]
pub enum SphError {
    InvalidDimension,
    InvalidConfig {
        field: &'static str,
    },
    InvalidTimeStep,
    NonFiniteGravity {
        axis: usize,
    },
    InvalidParticleField {
        particle: usize,
        field: &'static str,
        axis: Option<usize>,
    },
    NonPositiveDensity {
        particle: usize,
    },
    NumericRange {
        particle: Option<usize>,
        field: &'static str,
    },
    Kernel(SphKernelError),
    TimeStepTooLarge {
        requested: f64,
        maximum: f64,
    },
}

impl fmt::Display for SphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "SPH fluid requires D >= 1"),
            Self::InvalidConfig { field } => {
                write!(f, "invalid SPH configuration field: {field}")
            }
            Self::InvalidTimeStep => write!(f, "time step must be finite and positive"),
            Self::NonFiniteGravity { axis } => {
                write!(f, "non-finite gravity component on axis {axis}")
            }
            Self::InvalidParticleField {
                particle,
                field,
                axis,
            } => {
                write!(f, "invalid particle {particle} field {field}")?;
                if let Some(axis) = axis {
                    write!(f, " on axis {axis}")?;
                }
                Ok(())
            }
            Self::NonPositiveDensity { particle } => {
                write!(f, "particle {particle} has non-positive SPH density")
            }
            Self::NumericRange { particle, field } => {
                write!(f, "numeric range failure in {field}")?;
                if let Some(particle) = particle {
                    write!(f, " for particle {particle}")?;
                }
                Ok(())
            }
            Self::Kernel(error) => write!(f, "SPH kernel error: {error}"),
            Self::TimeStepTooLarge { requested, maximum } => {
                write!(
                    f,
                    "time step {requested} exceeds configured stability limit {maximum}"
                )
            }
        }
    }
}

impl Error for SphError {}

impl From<SphKernelError> for SphError {
    fn from(value: SphKernelError) -> Self {
        Self::Kernel(value)
    }
}

/// A single SPH particle. SI units are intended for D=3. Other dimensions
/// are mathematical models until their unit convention and reference cases
/// are explicitly declared. Force stores physical force, not acceleration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FluidParticle<const D: usize> {
    pub position: SVector<f64, D>,
    pub velocity: SVector<f64, D>,
    pub force: SVector<f64, D>,
    pub density: f64,
    pub pressure: f64,
    pub mass: f64,
}

impl<const D: usize> FluidParticle<D> {
    pub fn new(
        position: SVector<f64, D>,
        velocity: SVector<f64, D>,
        mass: f64,
    ) -> Result<Self, SphError> {
        if !mass.is_finite() || mass <= 0.0 {
            return Err(SphError::InvalidParticleField {
                particle: 0,
                field: "mass",
                axis: None,
            });
        }
        for axis in 0..D {
            if !position[axis].is_finite() {
                return Err(SphError::InvalidParticleField {
                    particle: 0,
                    field: "position",
                    axis: Some(axis),
                });
            }
            if !velocity[axis].is_finite() {
                return Err(SphError::InvalidParticleField {
                    particle: 0,
                    field: "velocity",
                    axis: Some(axis),
                });
            }
        }
        Ok(Self {
            position,
            velocity,
            force: SVector::<f64, D>::zeros(),
            density: 0.0,
            pressure: 0.0,
            mass,
        })
    }
}

/// Weakly-compressible SPH parameters.
///
/// pressure_stiffness is c0² (squared artificial sound speed, [m²/s²] in 3D
/// SI units). viscosity is dynamic viscosity μ ([Pa·s] in 3D), not an
/// arbitrary velocity-smoothing coefficient.
#[derive(Clone, Debug)]
pub struct SphConfig {
    pub smoothing_radius: f64,
    pub rest_density: f64,
    pub pressure_stiffness: f64,
    pub viscosity: f64,
}

impl Default for SphConfig {
    fn default() -> Self {
        Self {
            smoothing_radius: 1.0,
            rest_density: 1000.0,
            pressure_stiffness: 2000.0,
            viscosity: 0.001,
        }
    }
}

impl SphConfig {
    pub fn validate<const D: usize>(&self) -> Result<(), SphError> {
        if D == 0 {
            return Err(SphError::InvalidDimension);
        }
        Poly6Kernel::<D>::new(self.smoothing_radius)?;
        if !self.rest_density.is_finite() || self.rest_density <= 0.0 {
            return Err(SphError::InvalidConfig {
                field: "rest_density",
            });
        }
        if !self.pressure_stiffness.is_finite() || self.pressure_stiffness <= 0.0 {
            return Err(SphError::InvalidConfig {
                field: "pressure_stiffness (c0^2)",
            });
        }
        if !self.viscosity.is_finite() || self.viscosity < 0.0 {
            return Err(SphError::InvalidConfig {
                field: "viscosity (dynamic viscosity)",
            });
        }
        Ok(())
    }
}

/// Diagnostics from a successful transactional step.
#[derive(Clone, Debug)]
pub struct FluidStepReport<const D: usize> {
    pub particles: usize,
    pub total_mass: f64,
    pub momentum_before: SVector<f64, D>,
    pub momentum_after: SVector<f64, D>,
    pub min_density: f64,
    pub max_density: f64,
    pub max_speed_before: f64,
    pub max_speed_after: f64,
    pub max_force: f64,
    pub maximum_stable_dt: Option<f64>,
}

pub struct FluidSimulation<const D: usize> {
    pub particles: Vec<FluidParticle<D>>,
    pub config: SphConfig,
}

impl<const D: usize> FluidSimulation<D> {
    pub fn new(config: SphConfig) -> Self {
        Self {
            particles: Vec::new(),
            config,
        }
    }

    pub fn add_particle(&mut self, particle: FluidParticle<D>) -> Result<usize, SphError> {
        self.validate_particle(self.particles.len(), &particle)?;
        let index = self.particles.len();
        self.particles.push(particle);
        Ok(index)
    }

    /// Advance one step. Any validation/arithmetic/stability error aborts
    /// without committing density, pressure, force, velocity, or position.
    pub fn step(
        &mut self,
        dt: f64,
        gravity: &SVector<f64, D>,
    ) -> Result<FluidStepReport<D>, SphError> {
        self.config.validate::<D>()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(SphError::InvalidTimeStep);
        }
        for axis in 0..D {
            if !gravity[axis].is_finite() {
                return Err(SphError::NonFiniteGravity { axis });
            }
        }
        for (index, particle) in self.particles.iter().enumerate() {
            self.validate_particle(index, particle)?;
        }

        let count = self.particles.len();
        if count == 0 {
            return Ok(FluidStepReport {
                particles: 0,
                total_mass: 0.0,
                momentum_before: SVector::<f64, D>::zeros(),
                momentum_after: SVector::<f64, D>::zeros(),
                min_density: 0.0,
                max_density: 0.0,
                max_speed_before: 0.0,
                max_speed_after: 0.0,
                max_force: 0.0,
                maximum_stable_dt: None,
            });
        }

        let kernel = Poly6Kernel::<D>::new(self.config.smoothing_radius)?;
        let mut densities = vec![0.0; count];
        let mut pressures = vec![0.0; count];

        // Density summation; under-density is observable and is never clamped.
        for i in 0..count {
            let mut density = 0.0;
            for j in 0..count {
                let displacement = self.particles[i].position - self.particles[j].position;
                density += self.particles[j].mass * kernel.value(&displacement)?;
            }
            if !density.is_finite() {
                return Err(SphError::NumericRange {
                    particle: Some(i),
                    field: "density sum",
                });
            }
            if density <= 0.0 {
                return Err(SphError::NonPositiveDensity { particle: i });
            }
            let pressure = self.config.pressure_stiffness * (density - self.config.rest_density);
            if !pressure.is_finite() {
                return Err(SphError::NumericRange {
                    particle: Some(i),
                    field: "equation of state",
                });
            }
            densities[i] = density;
            pressures[i] = pressure;
        }

        let mut forces = vec![SVector::<f64, D>::zeros(); count];
        let h = self.config.smoothing_radius;
        let h_squared = h * h;
        let regularization = VISCOSITY_REGULARIZATION * VISCOSITY_REGULARIZATION * h_squared;

        for i in 0..count {
            let mut pressure_acceleration = SVector::<f64, D>::zeros();
            let mut viscosity_acceleration = SVector::<f64, D>::zeros();
            let rho_i_squared = densities[i] * densities[i];
            if !rho_i_squared.is_finite() || rho_i_squared <= 0.0 {
                return Err(SphError::NumericRange {
                    particle: Some(i),
                    field: "density squared",
                });
            }

            for j in 0..count {
                let displacement = self.particles[i].position - self.particles[j].position;
                let gradient = kernel.gradient(&displacement)?;
                let rho_j_squared = densities[j] * densities[j];
                if !rho_j_squared.is_finite() || rho_j_squared <= 0.0 {
                    return Err(SphError::NumericRange {
                        particle: Some(j),
                        field: "density squared",
                    });
                }

                let pressure_coefficient =
                    pressures[i] / rho_i_squared + pressures[j] / rho_j_squared;
                pressure_acceleration -= gradient * (self.particles[j].mass * pressure_coefficient);

                if self.config.viscosity > 0.0 {
                    let r_squared = displacement.norm_squared();
                    if !r_squared.is_finite() {
                        return Err(SphError::NumericRange {
                            particle: Some(i),
                            field: "pair distance",
                        });
                    }
                    if r_squared > 0.0 {
                        let rho_product = densities[i] * densities[j];
                        if !rho_product.is_finite() || rho_product <= 0.0 {
                            return Err(SphError::NumericRange {
                                particle: Some(i),
                                field: "density product",
                            });
                        }
                        let r_dot_gradient = displacement.dot(&gradient);
                        let denominator = r_squared + regularization;
                        // Morris-style regularized physical-viscosity term. This
                        // config is a uniform dynamic viscosity, so the pair
                        // sum is mu_i + mu_j = 2 * mu. The form and sign must
                        // remain independently verified against shear-flow
                        // references before this profile is called validated.
                        let coefficient = self.particles[j].mass * (2.0 * self.config.viscosity)
                            / rho_product
                            * r_dot_gradient
                            / denominator;
                        if !coefficient.is_finite() || !denominator.is_finite() {
                            return Err(SphError::NumericRange {
                                particle: Some(i),
                                field: "viscosity coefficient",
                            });
                        }
                        let relative_velocity =
                            self.particles[i].velocity - self.particles[j].velocity;
                        viscosity_acceleration += relative_velocity * coefficient;
                    }
                }
            }

            let force =
                (pressure_acceleration + viscosity_acceleration + gravity) * self.particles[i].mass;
            if !force.iter().all(|component| component.is_finite()) {
                return Err(SphError::NumericRange {
                    particle: Some(i),
                    field: "net force",
                });
            }
            forces[i] = force;
        }

        let speed_before = max_speed(&self.particles)?;
        let sound_speed = self.config.pressure_stiffness.sqrt();
        let acoustic_dt = ACOUSTIC_CFL * h / (sound_speed + speed_before);
        let kinematic_viscosity = self.config.viscosity / self.config.rest_density;
        let viscous_dt = if kinematic_viscosity > 0.0 {
            VISCOSITY_CFL * h_squared / kinematic_viscosity
        } else {
            f64::INFINITY
        };

        let mut max_acceleration: f64 = 0.0;
        let mut max_force: f64 = 0.0;
        for (index, force) in forces.iter().enumerate() {
            let magnitude = force.norm();
            let acceleration = magnitude / self.particles[index].mass;
            if !magnitude.is_finite() || !acceleration.is_finite() {
                return Err(SphError::NumericRange {
                    particle: Some(index),
                    field: "force magnitude",
                });
            }
            max_force = max_force.max(magnitude);
            max_acceleration = max_acceleration.max(acceleration);
        }
        let force_dt = if max_acceleration > 0.0 {
            ACOUSTIC_CFL * (h / max_acceleration).sqrt()
        } else {
            f64::INFINITY
        };
        let maximum_stable_dt = acoustic_dt.min(viscous_dt).min(force_dt);
        if !maximum_stable_dt.is_finite() || maximum_stable_dt <= 0.0 {
            return Err(SphError::NumericRange {
                particle: None,
                field: "timestep stability limit",
            });
        }
        if dt > maximum_stable_dt {
            return Err(SphError::TimeStepTooLarge {
                requested: dt,
                maximum: maximum_stable_dt,
            });
        }

        // Calculate the full next state and all diagnostics before any mutation.
        let mut new_velocities = Vec::with_capacity(count);
        let mut new_positions = Vec::with_capacity(count);
        let mut speed_after: f64 = 0.0;
        for (index, particle) in self.particles.iter().enumerate() {
            let velocity = particle.velocity + forces[index] * (dt / particle.mass);
            let position = particle.position + velocity * dt;
            if !velocity.iter().all(|component| component.is_finite()) {
                return Err(SphError::NumericRange {
                    particle: Some(index),
                    field: "integrated velocity",
                });
            }
            if !position.iter().all(|component| component.is_finite()) {
                return Err(SphError::NumericRange {
                    particle: Some(index),
                    field: "integrated position",
                });
            }
            let speed = velocity.norm();
            if !speed.is_finite() {
                return Err(SphError::NumericRange {
                    particle: Some(index),
                    field: "post-step speed",
                });
            }
            speed_after = speed_after.max(speed);
            new_velocities.push(velocity);
            new_positions.push(position);
        }

        let mut total_mass = 0.0;
        let mut momentum_before = SVector::<f64, D>::zeros();
        let mut momentum_after = SVector::<f64, D>::zeros();
        let mut min_density = f64::INFINITY;
        let mut max_density: f64 = 0.0;
        for i in 0..count {
            let mass = self.particles[i].mass;
            total_mass += mass;
            momentum_before += self.particles[i].velocity * mass;
            momentum_after += new_velocities[i] * mass;
            min_density = min_density.min(densities[i]);
            max_density = max_density.max(densities[i]);
        }
        if !total_mass.is_finite()
            || !momentum_before.iter().all(|x| x.is_finite())
            || !momentum_after.iter().all(|x| x.is_finite())
        {
            return Err(SphError::NumericRange {
                particle: None,
                field: "step diagnostics",
            });
        }

        for i in 0..count {
            self.particles[i].density = densities[i];
            self.particles[i].pressure = pressures[i];
            self.particles[i].force = forces[i];
            self.particles[i].velocity = new_velocities[i];
            self.particles[i].position = new_positions[i];
        }

        Ok(FluidStepReport {
            particles: count,
            total_mass,
            momentum_before,
            momentum_after,
            min_density,
            max_density,
            max_speed_before: speed_before,
            max_speed_after: speed_after,
            max_force,
            maximum_stable_dt: Some(maximum_stable_dt),
        })
    }

    fn validate_particle(&self, index: usize, particle: &FluidParticle<D>) -> Result<(), SphError> {
        if !particle.mass.is_finite() || particle.mass <= 0.0 {
            return Err(SphError::InvalidParticleField {
                particle: index,
                field: "mass",
                axis: None,
            });
        }
        for axis in 0..D {
            if !particle.position[axis].is_finite() {
                return Err(SphError::InvalidParticleField {
                    particle: index,
                    field: "position",
                    axis: Some(axis),
                });
            }
            if !particle.velocity[axis].is_finite() {
                return Err(SphError::InvalidParticleField {
                    particle: index,
                    field: "velocity",
                    axis: Some(axis),
                });
            }
        }
        Ok(())
    }
}

fn max_speed<const D: usize>(particles: &[FluidParticle<D>]) -> Result<f64, SphError> {
    let mut result: f64 = 0.0;
    for (index, particle) in particles.iter().enumerate() {
        let speed = particle.velocity.norm();
        if !speed.is_finite() {
            return Err(SphError::NumericRange {
                particle: Some(index),
                field: "speed",
            });
        }
        result = result.max(speed);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{FluidParticle, FluidSimulation, SphConfig, SphError};
    use nalgebra::SVector;

    fn config(viscosity: f64, rest_density: f64) -> SphConfig {
        SphConfig {
            smoothing_radius: 2.0,
            rest_density,
            pressure_stiffness: 1.0,
            viscosity,
        }
    }

    fn particle(position: [f64; 2], velocity: [f64; 2], mass: f64) -> FluidParticle<2> {
        FluidParticle::new(SVector::from(position), SVector::from(velocity), mass).unwrap()
    }

    #[test]
    fn under_density_is_not_clamped_to_rest_density() {
        let mut sim = FluidSimulation::<2>::new(config(0.0, 1000.0));
        sim.add_particle(particle([0.0, 0.0], [0.0, 0.0], 0.1))
            .unwrap();
        let report = sim.step(1e-5, &SVector::zeros()).unwrap();
        assert!(sim.particles[0].density < sim.config.rest_density);
        assert_eq!(report.particles, 1);
        assert!(sim.particles[0].pressure < 0.0);
    }

    #[test]
    fn gravity_uses_force_and_mass_consistently() {
        let mut sim = FluidSimulation::<2>::new(config(0.0, 1.0));
        sim.add_particle(particle([0.0, 0.0], [0.0, 0.0], 1.0))
            .unwrap();
        let dt = 1e-4;
        let gravity = SVector::from([0.0, -9.81]);
        sim.step(dt, &gravity).unwrap();
        assert!((sim.particles[0].velocity[1] + 9.81 * dt).abs() < 1e-10);
        assert!((sim.particles[0].position[1] + 9.81 * dt * dt).abs() < 1e-10);
    }

    #[test]
    fn pressure_pair_respects_action_reaction_in_symmetric_fixture() {
        let mut sim = FluidSimulation::<2>::new(config(0.0, 0.1));
        sim.add_particle(particle([-0.5, 0.0], [0.0, 0.0], 1.0))
            .unwrap();
        sim.add_particle(particle([0.5, 0.0], [0.0, 0.0], 1.0))
            .unwrap();
        let report = sim.step(1e-4, &SVector::zeros()).unwrap();
        assert!(report.momentum_after.norm() < 1e-10);
        assert!((sim.particles[0].force[0] + sim.particles[1].force[0]).abs() < 1e-10);
        assert!((sim.particles[0].force[1] + sim.particles[1].force[1]).abs() < 1e-10);
    }

    #[test]
    fn viscosity_damps_relative_shear_velocity() {
        let mut sim = FluidSimulation::<2>::new(config(0.5, 0.1));
        sim.add_particle(particle([0.0, 0.0], [0.0, 1.0], 1.0))
            .unwrap();
        sim.add_particle(particle([0.5, 0.0], [0.0, -1.0], 1.0))
            .unwrap();
        sim.step(1e-4, &SVector::zeros()).unwrap();
        assert!(sim.particles[0].velocity[1] < 1.0);
        assert!(sim.particles[1].velocity[1] > -1.0);
    }

    #[test]
    fn oversized_timestep_fails_without_partial_state_mutation() {
        let mut sim = FluidSimulation::<2>::new(config(0.0, 1.0));
        sim.add_particle(particle([0.0, 0.0], [0.0, 0.0], 1.0))
            .unwrap();
        let before = sim.particles[0].clone();
        assert!(matches!(
            sim.step(1.0, &SVector::zeros()),
            Err(SphError::TimeStepTooLarge { .. })
        ));
        assert_eq!(sim.particles[0].position, before.position);
        assert_eq!(sim.particles[0].velocity, before.velocity);
        assert_eq!(sim.particles[0].force, before.force);
        assert_eq!(sim.particles[0].density, before.density);
        assert_eq!(sim.particles[0].pressure, before.pressure);
    }

    #[test]
    fn invalid_configuration_is_a_typed_error() {
        let mut sim = FluidSimulation::<2>::new(SphConfig {
            smoothing_radius: 0.0,
            rest_density: 1.0,
            pressure_stiffness: 1.0,
            viscosity: 0.0,
        });
        assert!(matches!(
            sim.step(1e-4, &SVector::zeros()),
            Err(SphError::Kernel(_))
        ));
    }

    #[test]
    fn invalid_timestep_is_rejected() {
        let mut sim = FluidSimulation::<2>::new(config(0.0, 1.0));
        assert!(matches!(
            sim.step(f64::NAN, &SVector::zeros()),
            Err(SphError::InvalidTimeStep)
        ));
    }
}

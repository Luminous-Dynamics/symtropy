// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic Newtonian celestial dynamics as a scientific reference model.
//!
//! This crate is deliberately separate from Symtropy's rigid-body collision
//! engine. It provides direct O(N²) Newtonian point-mass gravity integrated by
//! velocity-Verlet, with energy, momentum, angular-momentum, and center-of-mass
//! diagnostics. The small-N implementation is intended as a correctness oracle
//! and benchmark substrate before any faster or GPU backend is introduced.
//!
//! It is **not** a relativistic integrator, a cosmological N-body code, an
//! ephemeris, or a validated model of close encounters, collisions, tides, or
//! extended bodies. Plummer softening is optional and changes the force law.

use std::collections::HashSet;
use std::fmt;

pub const GRAVITATIONAL_CONSTANT_M3_KG_S2: f64 = 6.674_30e-11;

/// A point mass in SI units. The system owns bodies and only exposes a shared
/// slice, so a validated state cannot be mutated behind its integrator.
#[derive(Clone, Debug, PartialEq)]
pub struct CelestialBody {
    pub id: u64,
    pub mass_kg: f64,
    pub position_m: [f64; 3],
    pub velocity_m_s: [f64; 3],
}

impl CelestialBody {
    pub const fn new(id: u64, mass_kg: f64, position_m: [f64; 3], velocity_m_s: [f64; 3]) -> Self {
        Self {
            id,
            mass_kg,
            position_m,
            velocity_m_s,
        }
    }
}

/// Deterministic diagnostics for a closed gravitational system.
#[derive(Clone, Debug, PartialEq)]
pub struct InvariantSnapshot {
    pub time_s: f64,
    pub step_count: u64,
    pub total_energy_j: f64,
    pub linear_momentum_kg_m_s: [f64; 3],
    pub angular_momentum_kg_m2_s: [f64; 3],
    pub center_of_mass_m: [f64; 3],
}

/// Invalid setup, integration, or numerical state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CelestialError {
    TooFewBodies,
    DuplicateBodyId,
    NonPositiveMass,
    InvalidSoftening,
    InvalidTimeStep,
    NonFiniteState,
    SingularConfiguration,
    NumericalOverflow,
}

impl fmt::Display for CelestialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooFewBodies => "celestial system requires at least two bodies",
            Self::DuplicateBodyId => "celestial body identifiers must be unique",
            Self::NonPositiveMass => "body masses must be finite and positive",
            Self::InvalidSoftening => "softening length must be finite and non-negative",
            Self::InvalidTimeStep => "time step must be finite and positive",
            Self::NonFiniteState => "body state must contain only finite values",
            Self::SingularConfiguration => {
                "coincident point masses with zero softening are singular"
            }
            Self::NumericalOverflow => "celestial dynamics calculation overflowed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CelestialError {}

/// Direct Newtonian N-body system with a fixed softening length.
///
/// With zero softening, this is the point-mass Newtonian force law. Positive
/// softening uses Plummer-softened pair forces and the matching softened
/// potential. The integration method is second-order velocity-Verlet; energy
/// is not expected to be exactly constant per step, so drift must be measured
/// against a frozen initial state.
#[derive(Clone, Debug, PartialEq)]
pub struct NBodySystem {
    bodies: Vec<CelestialBody>,
    softening_m: f64,
    time_s: f64,
    step_count: u64,
}

impl NBodySystem {
    pub fn new(bodies: Vec<CelestialBody>, softening_m: f64) -> Result<Self, CelestialError> {
        validate_bodies(&bodies)?;
        if !softening_m.is_finite() || softening_m < 0.0 {
            return Err(CelestialError::InvalidSoftening);
        }
        Ok(Self {
            bodies,
            softening_m,
            time_s: 0.0,
            step_count: 0,
        })
    }

    pub fn bodies(&self) -> &[CelestialBody] {
        &self.bodies
    }

    pub fn softening_m(&self) -> f64 {
        self.softening_m
    }

    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Advance one atomic velocity-Verlet step.
    ///
    /// Candidate positions and velocities are calculated before the stored
    /// state is changed. Invalid inputs or a singular candidate configuration
    /// return an error without partially committing that step.
    pub fn step(&mut self, dt_s: f64) -> Result<(), CelestialError> {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(CelestialError::InvalidTimeStep);
        }
        let next_time = self.time_s + dt_s;
        if !next_time.is_finite() {
            return Err(CelestialError::NumericalOverflow);
        }
        let next_step = self
            .step_count
            .checked_add(1)
            .ok_or(CelestialError::NumericalOverflow)?;

        let accelerations_0 = accelerations_for(
            &self.bodies,
            self.bodies.iter().map(|body| body.position_m).collect(),
            self.softening_m,
        )?;

        let mut half_velocities = Vec::with_capacity(self.bodies.len());
        let mut next_positions = Vec::with_capacity(self.bodies.len());
        for (body, acceleration) in self.bodies.iter().zip(accelerations_0.iter()) {
            let half_velocity = add(body.velocity_m_s, scale(*acceleration, 0.5 * dt_s));
            let position = add(body.position_m, scale(half_velocity, dt_s));
            if !is_finite_vector(half_velocity) || !is_finite_vector(position) {
                return Err(CelestialError::NumericalOverflow);
            }
            half_velocities.push(half_velocity);
            next_positions.push(position);
        }

        let accelerations_1 =
            accelerations_for(&self.bodies, next_positions.clone(), self.softening_m)?;
        let mut next_velocities = Vec::with_capacity(self.bodies.len());
        for (half_velocity, acceleration) in half_velocities.iter().zip(accelerations_1.iter()) {
            let velocity = add(*half_velocity, scale(*acceleration, 0.5 * dt_s));
            if !is_finite_vector(velocity) {
                return Err(CelestialError::NumericalOverflow);
            }
            next_velocities.push(velocity);
        }

        for ((body, position), velocity) in self
            .bodies
            .iter_mut()
            .zip(next_positions)
            .zip(next_velocities)
        {
            body.position_m = position;
            body.velocity_m_s = velocity;
        }
        self.time_s = next_time;
        self.step_count = next_step;
        Ok(())
    }

    pub fn kinetic_energy_j(&self) -> Result<f64, CelestialError> {
        let mut energy = 0.0;
        for body in &self.bodies {
            energy += 0.5 * body.mass_kg * dot(body.velocity_m_s, body.velocity_m_s);
        }
        finite_scalar(energy)
    }

    pub fn potential_energy_j(&self) -> Result<f64, CelestialError> {
        let mut energy = 0.0;
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let displacement = sub(self.bodies[j].position_m, self.bodies[i].position_m);
                let radius_squared =
                    dot(displacement, displacement) + self.softening_m * self.softening_m;
                if !radius_squared.is_finite() {
                    return Err(CelestialError::NumericalOverflow);
                }
                if radius_squared <= 0.0 {
                    return Err(CelestialError::SingularConfiguration);
                }
                energy -= GRAVITATIONAL_CONSTANT_M3_KG_S2
                    * self.bodies[i].mass_kg
                    * self.bodies[j].mass_kg
                    / radius_squared.sqrt();
            }
        }
        finite_scalar(energy)
    }

    pub fn total_energy_j(&self) -> Result<f64, CelestialError> {
        finite_scalar(self.kinetic_energy_j()? + self.potential_energy_j()?)
    }

    pub fn linear_momentum_kg_m_s(&self) -> Result<[f64; 3], CelestialError> {
        let mut momentum = [0.0; 3];
        for body in &self.bodies {
            momentum = add(momentum, scale(body.velocity_m_s, body.mass_kg));
        }
        finite_vector(momentum)
    }

    pub fn angular_momentum_kg_m2_s(&self) -> Result<[f64; 3], CelestialError> {
        let mut momentum = [0.0; 3];
        for body in &self.bodies {
            let linear = scale(body.velocity_m_s, body.mass_kg);
            momentum = add(momentum, cross(body.position_m, linear));
        }
        finite_vector(momentum)
    }

    pub fn center_of_mass_m(&self) -> Result<[f64; 3], CelestialError> {
        let total_mass: f64 = self.bodies.iter().map(|body| body.mass_kg).sum();
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(CelestialError::NumericalOverflow);
        }
        let mut weighted_position = [0.0; 3];
        for body in &self.bodies {
            weighted_position = add(weighted_position, scale(body.position_m, body.mass_kg));
        }
        finite_vector(scale(weighted_position, 1.0 / total_mass))
    }

    pub fn invariant_snapshot(&self) -> Result<InvariantSnapshot, CelestialError> {
        Ok(InvariantSnapshot {
            time_s: self.time_s,
            step_count: self.step_count,
            total_energy_j: self.total_energy_j()?,
            linear_momentum_kg_m_s: self.linear_momentum_kg_m_s()?,
            angular_momentum_kg_m2_s: self.angular_momentum_kg_m2_s()?,
            center_of_mass_m: self.center_of_mass_m()?,
        })
    }
}

fn validate_bodies(bodies: &[CelestialBody]) -> Result<(), CelestialError> {
    if bodies.len() < 2 {
        return Err(CelestialError::TooFewBodies);
    }
    let mut ids = HashSet::with_capacity(bodies.len());
    for body in bodies {
        if !body.mass_kg.is_finite() || body.mass_kg <= 0.0 {
            return Err(CelestialError::NonPositiveMass);
        }
        if !is_finite_vector(body.position_m) || !is_finite_vector(body.velocity_m_s) {
            return Err(CelestialError::NonFiniteState);
        }
        if !ids.insert(body.id) {
            return Err(CelestialError::DuplicateBodyId);
        }
    }
    Ok(())
}

fn accelerations_for(
    bodies: &[CelestialBody],
    positions: Vec<[f64; 3]>,
    softening_m: f64,
) -> Result<Vec<[f64; 3]>, CelestialError> {
    if positions.len() != bodies.len() {
        return Err(CelestialError::NonFiniteState);
    }
    let mut accelerations = vec![[0.0; 3]; bodies.len()];
    let softening_squared = softening_m * softening_m;
    if !softening_squared.is_finite() {
        return Err(CelestialError::NumericalOverflow);
    }

    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            let displacement = sub(positions[j], positions[i]);
            let radius_squared = dot(displacement, displacement) + softening_squared;
            if !radius_squared.is_finite() {
                return Err(CelestialError::NumericalOverflow);
            }
            if radius_squared <= 0.0 {
                return Err(CelestialError::SingularConfiguration);
            }
            let denominator = radius_squared * radius_squared.sqrt();
            let scale_i = GRAVITATIONAL_CONSTANT_M3_KG_S2 * bodies[j].mass_kg / denominator;
            let scale_j = GRAVITATIONAL_CONSTANT_M3_KG_S2 * bodies[i].mass_kg / denominator;
            accelerations[i] = add(accelerations[i], scale(displacement, scale_i));
            accelerations[j] = sub(accelerations[j], scale(displacement, scale_j));
            if !is_finite_vector(accelerations[i]) || !is_finite_vector(accelerations[j]) {
                return Err(CelestialError::NumericalOverflow);
            }
        }
    }
    Ok(accelerations)
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn is_finite_vector(vector: [f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn finite_vector(vector: [f64; 3]) -> Result<[f64; 3], CelestialError> {
    if is_finite_vector(vector) {
        Ok(vector)
    } else {
        Err(CelestialError::NumericalOverflow)
    }
}

fn finite_scalar(value: f64) -> Result<f64, CelestialError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CelestialError::NumericalOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEPARATION_M: f64 = 1.0e7;
    const BODY_MASS_KG: f64 = 1.0e20;

    fn circular_binary() -> (NBodySystem, f64, f64) {
        let speed = (GRAVITATIONAL_CONSTANT_M3_KG_S2 * BODY_MASS_KG / (2.0 * SEPARATION_M)).sqrt();
        let bodies = vec![
            CelestialBody::new(
                1,
                BODY_MASS_KG,
                [-SEPARATION_M / 2.0, 0.0, 0.0],
                [0.0, -speed, 0.0],
            ),
            CelestialBody::new(
                2,
                BODY_MASS_KG,
                [SEPARATION_M / 2.0, 0.0, 0.0],
                [0.0, speed, 0.0],
            ),
        ];
        let period = 2.0
            * std::f64::consts::PI
            * (SEPARATION_M.powi(3) / (GRAVITATIONAL_CONSTANT_M3_KG_S2 * 2.0 * BODY_MASS_KG))
                .sqrt();
        (NBodySystem::new(bodies, 0.0).unwrap(), period, speed)
    }

    fn eccentric_binary() -> (NBodySystem, f64) {
        let eccentricity = 0.6;
        let semi_major_axis_m = 1.0e7;
        let total_mass_kg = 2.0 * BODY_MASS_KG;
        let gravitational_parameter = GRAVITATIONAL_CONSTANT_M3_KG_S2 * total_mass_kg;
        let periapsis_m = semi_major_axis_m * (1.0 - eccentricity);
        let relative_speed_m_s = (gravitational_parameter * (1.0 + eccentricity)
            / (semi_major_axis_m * (1.0 - eccentricity)))
            .sqrt();
        let bodies = vec![
            CelestialBody::new(
                1,
                BODY_MASS_KG,
                [-periapsis_m / 2.0, 0.0, 0.0],
                [0.0, -relative_speed_m_s / 2.0, 0.0],
            ),
            CelestialBody::new(
                2,
                BODY_MASS_KG,
                [periapsis_m / 2.0, 0.0, 0.0],
                [0.0, relative_speed_m_s / 2.0, 0.0],
            ),
        ];
        let period_s = 2.0
            * std::f64::consts::PI
            * (semi_major_axis_m.powi(3) / gravitational_parameter).sqrt();
        (NBodySystem::new(bodies, 0.0).unwrap(), period_s)
    }

    fn eccentric_orbit_closure_error(steps: usize) -> (f64, f64) {
        let (mut system, period) = eccentric_binary();
        let start_positions: Vec<[f64; 3]> =
            system.bodies().iter().map(|body| body.position_m).collect();
        let start_energy = system.total_energy_j().unwrap();
        let dt = period / steps as f64;
        for _ in 0..steps {
            system.step(dt).unwrap();
        }
        let position_error_squared: f64 = system
            .bodies()
            .iter()
            .zip(start_positions.iter())
            .map(|(body, initial)| {
                let delta = sub(body.position_m, *initial);
                dot(delta, delta)
            })
            .sum();
        let initial_scale_squared: f64 = start_positions
            .iter()
            .map(|position| dot(*position, *position))
            .sum();
        let position_error = (position_error_squared / initial_scale_squared).sqrt();
        let relative_energy_drift =
            (system.total_energy_j().unwrap() - start_energy).abs() / start_energy.abs();
        (position_error, relative_energy_drift)
    }

    fn orbit_closure_error(steps: usize) -> (f64, f64) {
        let (mut system, period, _) = circular_binary();
        let start_positions: Vec<[f64; 3]> =
            system.bodies().iter().map(|body| body.position_m).collect();
        let start_energy = system.total_energy_j().unwrap();
        let dt = period / steps as f64;
        for _ in 0..steps {
            system.step(dt).unwrap();
        }
        let position_error_squared: f64 = system
            .bodies()
            .iter()
            .zip(start_positions.iter())
            .map(|(body, initial)| {
                let delta = sub(body.position_m, *initial);
                dot(delta, delta)
            })
            .sum();
        let initial_scale_squared: f64 = start_positions.iter().map(|p| dot(*p, *p)).sum();
        let position_error = (position_error_squared / initial_scale_squared).sqrt();
        let relative_energy_drift =
            (system.total_energy_j().unwrap() - start_energy).abs() / start_energy.abs();
        (position_error, relative_energy_drift)
    }

    #[test]
    fn circular_two_body_orbit_closes_and_bounds_energy_drift() {
        let (position_error, relative_energy_drift) = orbit_closure_error(1_000);
        assert!(
            position_error < 1.1e-4,
            "one-period position error was {position_error}"
        );
        assert!(
            relative_energy_drift < 1e-10,
            "relative energy drift was {relative_energy_drift}"
        );
    }

    #[test]
    fn halving_timestep_reduces_orbit_closure_error() {
        let (coarse_error, _) = orbit_closure_error(128);
        let (fine_error, _) = orbit_closure_error(256);
        assert!(
            fine_error < coarse_error * 0.3,
            "coarse={coarse_error}, fine={fine_error}"
        );
    }

    #[test]
    fn eccentric_two_body_orbit_shows_second_order_refinement() {
        // The two-body period is analytic even though the numerical trajectory
        // is integrated. This prevents circular-only fixtures from hiding
        // error growth near periapsis.
        let (coarse_error, _) = eccentric_orbit_closure_error(1_024);
        let (fine_error, fine_energy_drift) = eccentric_orbit_closure_error(2_048);
        assert!(
            fine_error < coarse_error * 0.3,
            "eccentric orbit did not converge under timestep halving: coarse={coarse_error}, fine={fine_error}"
        );
        assert!(
            fine_energy_drift < 2e-9,
            "eccentric relative energy drift was {fine_energy_drift}"
        );
    }

    #[test]
    fn closed_binary_conserves_momentum_center_and_angular_momentum() {
        let (mut system, period, _) = circular_binary();
        let initial = system.invariant_snapshot().unwrap();
        for _ in 0..500 {
            system.step(period / 1_000.0).unwrap();
        }
        let final_state = system.invariant_snapshot().unwrap();
        let initial_momentum_scale = BODY_MASS_KG
            * 2.0
            * (GRAVITATIONAL_CONSTANT_M3_KG_S2 * BODY_MASS_KG / (2.0 * SEPARATION_M)).sqrt();

        let momentum_error = norm(sub(
            final_state.linear_momentum_kg_m_s,
            initial.linear_momentum_kg_m_s,
        ));
        let angular_error = norm(sub(
            final_state.angular_momentum_kg_m2_s,
            initial.angular_momentum_kg_m2_s,
        ));
        let center_error = norm(sub(final_state.center_of_mass_m, initial.center_of_mass_m));
        assert!(momentum_error < initial_momentum_scale * 1e-12);
        assert!(angular_error / norm(initial.angular_momentum_kg_m2_s) < 1e-12);
        assert!(center_error / SEPARATION_M < 1e-12);
        assert_eq!(final_state.step_count, 500);
    }

    #[test]
    fn singular_step_fails_without_partially_committing_state() {
        let mut system = NBodySystem::new(
            vec![
                CelestialBody::new(1, 1.0, [0.0; 3], [0.0; 3]),
                CelestialBody::new(2, 2.0, [0.0; 3], [1.0, 0.0, 0.0]),
            ],
            0.0,
        )
        .unwrap();
        let before = system.clone();
        assert_eq!(system.step(0.1), Err(CelestialError::SingularConfiguration));
        assert_eq!(system, before);
    }

    #[test]
    fn invalid_setup_and_time_step_fail_closed() {
        assert_eq!(
            NBodySystem::new(vec![CelestialBody::new(1, 1.0, [0.0; 3], [0.0; 3])], 0.0)
                .unwrap_err(),
            CelestialError::TooFewBodies
        );
        assert_eq!(
            NBodySystem::new(
                vec![
                    CelestialBody::new(1, 1.0, [0.0; 3], [0.0; 3]),
                    CelestialBody::new(2, 0.0, [1.0, 0.0, 0.0], [0.0; 3]),
                ],
                0.0,
            )
            .unwrap_err(),
            CelestialError::NonPositiveMass
        );
        let (mut system, _, _) = circular_binary();
        assert_eq!(system.step(0.0), Err(CelestialError::InvalidTimeStep));
        assert_eq!(system.step(f64::NAN), Err(CelestialError::InvalidTimeStep));
    }

    fn norm(vector: [f64; 3]) -> f64 {
        dot(vector, vector).sqrt()
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fluid dynamics primitives for Symtropy.
//!
//! The current gameplay/runtime implementation is an early SPH scaffold:
//! density uses an all-pairs reference loop and pressure/viscosity gradients are
//! not yet a production solver. [`validation`] is solver-independent,
//! [`evidence`] binds executable results to exact source/profile subjects,
//! [`reference`] is a deliberately small CPU continuum solver,
//! [`convergence`] provides analytical/error measurements,
//! [`manufactured`] adds a time-dependent manufactured solution,
//! [`verification_ladder`] preserves strict machine-readable spatial/temporal
//! refinement and energy evidence, [`falsification`] preserves expected
//! numerical failures as typed campaign evidence rather than erasing the run,
//! [`diagnostic_trace`] retains stepwise diagnostics and error evolution,
//! [`numerical_observability`] measures signed stability margins plus controlled
//! pressure-projection residual convergence, [`iterative_error`] measures
//! projection-only solution change on a controlled compressive fixture,
//! [`verification_regime`] summarizes within-fixture refinement trends without a
//! promotion verdict, [`manufactured_iterative`] measures pressure-iteration
//! sensitivity on the exact manufactured Taylor-Green trajectory itself,
//! [`excess_energy_decay`] measures resolved kinetic-energy loss relative to the
//! exact passive Taylor-Green viscous decay in the same discrete quadrature,
//! [`manufactured_update_defect`] measures the one-step discrete operator defect
//! against the exact forced manufactured trajectory,
//! [`smooth_scale_separation`] records analytical grid/mode separation for the
//! smooth Taylor-Green control, [`vorticity_concentration_scale`] measures a
//! vorticity/enstrophy-palinstrophy gradient length without assigning refinement
//! or promotion authority, [`concentration_scale_drift`] records same-run
//! concentration-scale drift together with excess resolved-energy loss and
//! numerical-health context for the passive Taylor-Green control,
//! [`refinement_policy_contract`] defines a non-authoritative typed boundary for
//! future semantic refinement decisions without embedding renderer/backend load,
//! [`problem_revision_envelope`] binds future evaluations to exact canonical
//! state/snapshot/source/forcing/boundary/process revisions without granting
//! refinement authority, [`validity_evidence_lineage`] preserves append-only
//! continuum validity history so later refinement cannot erase earlier
//! under-resolution or solver-failure evidence, and [`process_error_requirements`]
//! binds process-owned numerical tolerances to exact error evidence identities
//! without evaluating them or creating universal resolution thresholds.
//! None of these modules should be read as a claim that Symtropy currently
//! reproduces the 2026 singular construction or has a production Earth-water
//! CFD backend.

pub mod concentration_scale_drift;
pub mod convergence;
pub mod diagnostic_trace;
pub mod evidence;
pub mod excess_energy_decay;
pub mod falsification;
pub mod iterative_error;
pub mod manufactured;
pub mod manufactured_iterative;
pub mod manufactured_update_defect;
pub mod numerical_observability;
pub mod problem_revision_envelope;
pub mod process_error_requirements;
pub mod reference;
pub mod refinement_policy_contract;
pub mod smooth_scale_separation;
pub mod validation;
pub mod validity_evidence_lineage;
pub mod verification_ladder;
pub mod verification_regime;
pub mod vorticity_concentration_scale;

use nalgebra::SVector;
use serde::{Deserialize, Serialize};

/// A single fluid particle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FluidParticle<const D: usize> {
    pub position: SVector<f64, D>,
    pub velocity: SVector<f64, D>,
    pub force: SVector<f64, D>,
    pub density: f64,
    pub pressure: f64,
    pub mass: f64,
}

/// SPH Fluid simulation parameters.
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
            viscosity: 0.1,
        }
    }
}

/// A fluid simulation instance.
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

    /// Step the fluid simulation.
    pub fn step(&mut self, dt: f64, gravity: &SVector<f64, D>) {
        if dt <= 0.0 || self.particles.is_empty() {
            return;
        }

        // 1. Calculate Density and Pressure
        self.calculate_density_pressure();

        // 2. Calculate Forces (Pressure + Viscosity)
        self.calculate_forces(gravity);

        // 3. Integrate
        for p in &mut self.particles {
            p.velocity += p.force * (dt / p.density);
            p.position += p.velocity * dt;
        }
    }

    fn calculate_density_pressure(&mut self) {
        let h = self.config.smoothing_radius;
        let h2 = h * h;
        // kernel normalization factor (simplified)
        let poly6 = 315.0 / (64.0 * std::f64::consts::PI * h.powi(9));

        for i in 0..self.particles.len() {
            let mut density = 0.0;
            let pi = self.particles[i].position;

            for j in 0..self.particles.len() {
                let r = pi - self.particles[j].position;
                let r2 = r.norm_squared();
                if r2 < h2 {
                    density += self.particles[j].mass * poly6 * (h2 - r2).powi(3);
                }
            }

            self.particles[i].density = density.max(self.config.rest_density);
            self.particles[i].pressure = self.config.pressure_stiffness
                * (self.particles[i].density - self.config.rest_density);
        }
    }

    fn calculate_forces(&mut self, gravity: &SVector<f64, D>) {
        // Gradients/Laplacians for pressure/viscosity remain placeholders.
        // Keep this explicit until #452 replaces the gameplay scaffold with a
        // separately qualified local formulation. The #511 reference solver is
        // deliberately not a drop-in gameplay replacement.
        for i in 0..self.particles.len() {
            let f_press = SVector::<f64, D>::zeros();
            let f_visc = SVector::<f64, D>::zeros();

            self.particles[i].force = f_press + f_visc + gravity * self.particles[i].density;
        }
    }
}

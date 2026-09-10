// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-head evidence generator for same-case pressure-iteration sensitivity.
//!
//! The sweep deliberately matches the finest manufactured temporal verification
//! case at 24x24 cells, 16 steps, and T=0.004 s. Its 400-iteration point is the
//! production point being examined; 800 iterations is only a higher numerical
//! reference. No ratio, acceptance threshold, or convergence verdict is frozen
//! by this executable.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::manufactured::ManufacturedTaylorGreenProfile;
use symtropy_fluid::manufactured_iterative::{
    ManufacturedIterativeSweepReport, run_manufactured_iterative_sensitivity_sweep,
};
use symtropy_fluid::reference::PeriodicMacConfig;

const EVIDENCE_DOCUMENT_SCHEMA_ID: &str = "manufactured-iterative-evidence-document-v0.1";
const CASE_PROFILE_ID: &str = "manufactured-taylor-green-iterative-sensitivity-evidence-v0.1";
const MATCHED_PRESSURE_ITERATIONS: usize = 400;
const NUMERICAL_REFERENCE_PRESSURE_ITERATIONS: usize = 800;

#[derive(Debug, Serialize)]
struct ManufacturedIterativeEvidenceDocument {
    schema_id: &'static str,
    source_revision: String,
    matched_pressure_iterations: usize,
    numerical_reference_pressure_iterations: usize,
    evidence: ContinuumEvidenceEnvelope<ManufacturedIterativeSweepReport>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;
    let profile = ManufacturedTaylorGreenProfile {
        base_amplitude_mps: 0.08,
        modulation_fraction: 0.2,
        angular_frequency_rad_s: 1.5,
    };
    let report = run_manufactured_iterative_sensitivity_sweep(
        case_config(),
        profile,
        0.004,
        16,
        &[1, 4, 16, 64, 256, 400, 800],
    )?;

    if report.reference_pressure_iterations != NUMERICAL_REFERENCE_PRESSURE_ITERATIONS {
        return Err("manufactured iterative numerical reference drifted from 800 iterations".into());
    }
    if !report
        .points
        .iter()
        .any(|point| point.pressure_iterations == MATCHED_PRESSURE_ITERATIONS)
    {
        return Err("manufactured iterative sweep no longer contains the 400-iteration match".into());
    }

    let execution_profiles = report
        .points
        .iter()
        .map(|point| point.solver_profile.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let subject = ContinuumEvidenceSubject {
        schema_version: CONTINUUM_EVIDENCE_SCHEMA_VERSION,
        source_revision: source_revision.clone(),
        execution_profiles,
        case_profile: CASE_PROFILE_ID.to_owned(),
        benchmark_id: None,
        fixture_digest: None,
    };
    let evidence = subject.bind(report)?;
    let document = ManufacturedIterativeEvidenceDocument {
        schema_id: EVIDENCE_DOCUMENT_SCHEMA_ID,
        source_revision,
        matched_pressure_iterations: MATCHED_PRESSURE_ITERATIONS,
        numerical_reference_pressure_iterations: NUMERICAL_REFERENCE_PRESSURE_ITERATIONS,
        evidence,
    };

    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}

fn case_config() -> PeriodicMacConfig {
    PeriodicMacConfig {
        nx: 24,
        ny: 24,
        length_x_m: std::f64::consts::TAU,
        length_y_m: std::f64::consts::TAU,
        slab_depth_m: 1.0,
        density_kg_m3: 1.0,
        kinematic_viscosity_m2_s: 0.01,
        pressure_iterations: MATCHED_PRESSURE_ITERATIONS,
        max_advective_cfl: 0.5,
        max_diffusion_number: 0.24,
    }
}

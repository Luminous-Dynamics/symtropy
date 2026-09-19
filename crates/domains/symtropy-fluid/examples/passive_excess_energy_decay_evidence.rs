// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-head evidence for passive Taylor-Green excess resolved-energy loss.

use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::excess_energy_decay::{
    ExcessEnergyDecayReport, run_passive_taylor_green_excess_energy_decay,
};
use symtropy_fluid::reference::PeriodicMacConfig;

const DOCUMENT_SCHEMA_ID: &str = "passive-excess-energy-decay-evidence-document-v0.1";
const CASE_PROFILE_ID: &str = "passive-taylor-green-excess-energy-decay-evidence-v0.1";

#[derive(Debug, Serialize)]
struct PassiveExcessEnergyDecayEvidenceDocument {
    schema_id: &'static str,
    source_revision: String,
    evidence: ContinuumEvidenceEnvelope<ExcessEnergyDecayReport>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;
    let report = run_passive_taylor_green_excess_energy_decay(case_config(), 0.08, 0.00025, 16)?;
    let execution_profiles = vec![report.solver_profile.clone()];
    let subject = ContinuumEvidenceSubject {
        schema_version: CONTINUUM_EVIDENCE_SCHEMA_VERSION,
        source_revision: source_revision.clone(),
        execution_profiles,
        case_profile: CASE_PROFILE_ID.to_owned(),
        benchmark_id: None,
        fixture_digest: None,
    };
    let evidence = subject.bind(report)?;
    let document = PassiveExcessEnergyDecayEvidenceDocument {
        schema_id: DOCUMENT_SCHEMA_ID,
        source_revision,
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
        pressure_iterations: 400,
        max_advective_cfl: 0.5,
        max_diffusion_number: 0.24,
    }
}

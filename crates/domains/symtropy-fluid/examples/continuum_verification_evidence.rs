// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::manufactured::ManufacturedTaylorGreenProfile;
use symtropy_fluid::reference::PeriodicMacConfig;
use symtropy_fluid::verification_ladder::{
    EnergyTraceReport, LadderCaseSpec, RefinementAxis, VerificationLadderReport,
    run_manufactured_taylor_green_ladder, run_passive_taylor_green_ladder,
    run_unforced_energy_trace,
};

const EVIDENCE_BUNDLE_SCHEMA_ID: &str = "continuum-verification-evidence-bundle-v0.1";

#[derive(Debug, Serialize)]
struct EvidenceBundle {
    schema_id: &'static str,
    source_revision: String,
    passive_spatial: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    passive_temporal: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    manufactured_temporal: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    unforced_energy: ContinuumEvidenceEnvelope<EnergyTraceReport>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;
    let base = base_config();

    let passive_spatial = run_passive_taylor_green_ladder(
        base.clone(),
        0.08,
        0.004,
        RefinementAxis::Spatial,
        &[
            LadderCaseSpec {
                resolution: 8,
                steps: 16,
            },
            LadderCaseSpec {
                resolution: 16,
                steps: 16,
            },
            LadderCaseSpec {
                resolution: 32,
                steps: 16,
            },
        ],
    )?;

    let passive_temporal = run_passive_taylor_green_ladder(
        base.clone(),
        0.08,
        0.004,
        RefinementAxis::Temporal,
        &[
            LadderCaseSpec {
                resolution: 24,
                steps: 4,
            },
            LadderCaseSpec {
                resolution: 24,
                steps: 8,
            },
            LadderCaseSpec {
                resolution: 24,
                steps: 16,
            },
        ],
    )?;

    let manufactured_profile = ManufacturedTaylorGreenProfile {
        base_amplitude_mps: 0.08,
        modulation_fraction: 0.2,
        angular_frequency_rad_s: 1.5,
    };
    let manufactured_temporal = run_manufactured_taylor_green_ladder(
        base.clone(),
        manufactured_profile,
        0.004,
        RefinementAxis::Temporal,
        &[
            LadderCaseSpec {
                resolution: 24,
                steps: 4,
            },
            LadderCaseSpec {
                resolution: 24,
                steps: 8,
            },
            LadderCaseSpec {
                resolution: 24,
                steps: 16,
            },
        ],
    )?;

    let mut energy_config = base;
    energy_config.nx = 24;
    energy_config.ny = 24;
    let unforced_energy = run_unforced_energy_trace(energy_config, 0.08, 0.00025, 16)?;

    let bundle = EvidenceBundle {
        schema_id: EVIDENCE_BUNDLE_SCHEMA_ID,
        source_revision: source_revision.clone(),
        passive_spatial: bind_ladder(
            &source_revision,
            "passive-taylor-green-spatial-v0.1",
            passive_spatial,
        )?,
        passive_temporal: bind_ladder(
            &source_revision,
            "passive-taylor-green-temporal-v0.1",
            passive_temporal,
        )?,
        manufactured_temporal: bind_ladder(
            &source_revision,
            "manufactured-taylor-green-temporal-v0.1",
            manufactured_temporal,
        )?,
        unforced_energy: bind_energy(
            &source_revision,
            "passive-taylor-green-energy-trace-v0.1",
            unforced_energy,
        )?,
    };

    println!("{}", serde_json::to_string_pretty(&bundle)?);
    Ok(())
}

fn bind_ladder(
    source_revision: &str,
    case_profile: &str,
    report: VerificationLadderReport,
) -> Result<ContinuumEvidenceEnvelope<VerificationLadderReport>, Box<dyn Error>> {
    let execution_profiles = report
        .points
        .iter()
        .map(|point| point.solver_profile.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(subject(source_revision, case_profile, execution_profiles).bind(report)?)
}

fn bind_energy(
    source_revision: &str,
    case_profile: &str,
    report: EnergyTraceReport,
) -> Result<ContinuumEvidenceEnvelope<EnergyTraceReport>, Box<dyn Error>> {
    let execution_profiles = vec![report.solver_profile.clone()];
    Ok(subject(source_revision, case_profile, execution_profiles).bind(report)?)
}

fn subject(
    source_revision: &str,
    case_profile: &str,
    execution_profiles: Vec<String>,
) -> ContinuumEvidenceSubject {
    ContinuumEvidenceSubject {
        schema_version: CONTINUUM_EVIDENCE_SCHEMA_VERSION,
        source_revision: source_revision.to_owned(),
        execution_profiles,
        case_profile: case_profile.to_owned(),
        benchmark_id: None,
        fixture_digest: None,
    }
}

fn base_config() -> PeriodicMacConfig {
    PeriodicMacConfig {
        nx: 16,
        ny: 16,
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

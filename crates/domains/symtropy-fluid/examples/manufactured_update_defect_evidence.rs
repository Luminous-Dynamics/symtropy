// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-head evidence generator for the manufactured one-step update defect.
//!
//! It retains temporal refinement, spatial refinement, and forcing-phase samples
//! without assigning an observed-order threshold or promotion verdict.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::manufactured::{
    ManufacturedTaylorGreenProfile, forcing_amplitude_mps2,
};
use symtropy_fluid::manufactured_update_defect::{
    ManufacturedUpdateDefectReport, measure_manufactured_one_step_update_defect,
    measure_manufactured_one_step_update_defect_at_phase,
};
use symtropy_fluid::reference::PeriodicMacConfig;

const DOCUMENT_SCHEMA_ID: &str = "manufactured-update-defect-evidence-document-v0.1";
const CASE_PROFILE_ID: &str = "manufactured-update-defect-refinement-campaign-v0.1";
const SPATIAL_DT_S: f64 = 0.000125;
const PHASE_RESOLUTION: usize = 24;
const PHASE_DT_S: f64 = 0.000125;

#[derive(Debug, Serialize)]
struct ManufacturedPhaseProbe {
    cycle_fraction: f64,
    exact_start_amplitude_mps: f64,
    exact_end_amplitude_mps: f64,
    exact_start_solenoidal_forcing_amplitude_mps2: f64,
    exact_end_solenoidal_forcing_amplitude_mps2: f64,
    report: ManufacturedUpdateDefectReport,
}

#[derive(Debug, Serialize)]
struct ManufacturedUpdateDefectCampaign {
    temporal_fixed_resolution: usize,
    temporal_points: Vec<ManufacturedUpdateDefectReport>,
    spatial_fixed_dt_s: f64,
    spatial_points: Vec<ManufacturedUpdateDefectReport>,
    phase_fixed_resolution: usize,
    phase_fixed_dt_s: f64,
    phase_points: Vec<ManufacturedPhaseProbe>,
}

#[derive(Debug, Serialize)]
struct ManufacturedUpdateDefectEvidenceDocument {
    schema_id: &'static str,
    source_revision: String,
    evidence: ContinuumEvidenceEnvelope<ManufacturedUpdateDefectCampaign>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;
    let profile = ManufacturedTaylorGreenProfile {
        base_amplitude_mps: 0.08,
        modulation_fraction: 0.2,
        angular_frequency_rad_s: 1.5,
    };

    let temporal_resolution = 24;
    let temporal_points = [0.001, 0.0005, 0.00025, 0.000125]
        .into_iter()
        .map(|dt_s| {
            measure_manufactured_one_step_update_defect(
                case_config(temporal_resolution),
                profile,
                dt_s,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let spatial_points = [12, 16, 24, 32]
        .into_iter()
        .map(|resolution| {
            measure_manufactured_one_step_update_defect(
                case_config(resolution),
                profile,
                SPATIAL_DT_S,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let phase_config = case_config(PHASE_RESOLUTION);
    let period_s = std::f64::consts::TAU / profile.angular_frequency_rad_s;
    let phase_points = [0.0, 0.25, 0.5, 0.75]
        .into_iter()
        .map(|cycle_fraction| {
            let start_time_s = cycle_fraction * period_s;
            let report = measure_manufactured_one_step_update_defect_at_phase(
                phase_config.clone(),
                profile,
                start_time_s,
                PHASE_DT_S,
            )?;
            let end_time_s = report.manufactured_end_time_s;
            Ok::<_, Box<dyn Error>>(ManufacturedPhaseProbe {
                cycle_fraction,
                exact_start_amplitude_mps: profile.amplitude_mps(start_time_s)?,
                exact_end_amplitude_mps: profile.amplitude_mps(end_time_s)?,
                exact_start_solenoidal_forcing_amplitude_mps2: forcing_amplitude_mps2(
                    &phase_config,
                    profile,
                    start_time_s,
                )?,
                exact_end_solenoidal_forcing_amplitude_mps2: forcing_amplitude_mps2(
                    &phase_config,
                    profile,
                    end_time_s,
                )?,
                report,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let execution_profiles = temporal_points
        .iter()
        .chain(&spatial_points)
        .map(|point| point.solver_profile.clone())
        .chain(
            phase_points
                .iter()
                .map(|point| point.report.solver_profile.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let campaign = ManufacturedUpdateDefectCampaign {
        temporal_fixed_resolution: temporal_resolution,
        temporal_points,
        spatial_fixed_dt_s: SPATIAL_DT_S,
        spatial_points,
        phase_fixed_resolution: PHASE_RESOLUTION,
        phase_fixed_dt_s: PHASE_DT_S,
        phase_points,
    };
    let subject = ContinuumEvidenceSubject {
        schema_version: CONTINUUM_EVIDENCE_SCHEMA_VERSION,
        source_revision: source_revision.clone(),
        execution_profiles,
        case_profile: CASE_PROFILE_ID.to_owned(),
        benchmark_id: None,
        fixture_digest: None,
    };
    let evidence = subject.bind(campaign)?;
    let document = ManufacturedUpdateDefectEvidenceDocument {
        schema_id: DOCUMENT_SCHEMA_ID,
        source_revision,
        evidence,
    };

    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}

fn case_config(resolution: usize) -> PeriodicMacConfig {
    PeriodicMacConfig {
        nx: resolution,
        ny: resolution,
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

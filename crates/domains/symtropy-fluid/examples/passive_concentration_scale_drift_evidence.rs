// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-head evidence for same-run passive Taylor-Green concentration-scale
//! drift and excess resolved-energy loss.
//!
//! The campaign retains spatial and temporal refinement ladders over the same
//! physical duration. It records raw reports only and freezes no false-smoothness
//! threshold, convergence-order requirement, or refinement verdict.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::concentration_scale_drift::{
    PassiveConcentrationScaleDriftReport, run_passive_taylor_green_concentration_scale_drift,
};
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::reference::PeriodicMacConfig;

const DOCUMENT_SCHEMA_ID: &str = "passive-concentration-scale-drift-evidence-document-v0.1";
const CASE_PROFILE_ID: &str = "passive-taylor-green-same-run-scale-energy-drift-refinement-v0.1";
const INITIAL_AMPLITUDE_MPS: f64 = 0.08;
const TARGET_TIME_S: f64 = 0.004;

#[derive(Debug, Serialize)]
struct SpatialDriftPoint {
    resolution: usize,
    report: PassiveConcentrationScaleDriftReport,
}

#[derive(Debug, Serialize)]
struct TemporalDriftPoint {
    dt_s: f64,
    steps: usize,
    report: PassiveConcentrationScaleDriftReport,
}

#[derive(Debug, Serialize)]
struct PassiveConcentrationScaleDriftCampaign {
    target_time_s: f64,
    spatial_dt_s: f64,
    spatial_points: Vec<SpatialDriftPoint>,
    temporal_resolution: usize,
    temporal_points: Vec<TemporalDriftPoint>,
}

#[derive(Debug, Serialize)]
struct PassiveConcentrationScaleDriftEvidenceDocument {
    schema_id: &'static str,
    source_revision: String,
    evidence: ContinuumEvidenceEnvelope<PassiveConcentrationScaleDriftCampaign>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;

    let spatial_dt_s = 0.00025;
    let spatial_steps = exact_step_count(TARGET_TIME_S, spatial_dt_s)?;
    let spatial_points = [12, 16, 24, 32]
        .into_iter()
        .map(|resolution| {
            Ok(SpatialDriftPoint {
                resolution,
                report: run_passive_taylor_green_concentration_scale_drift(
                    case_config(resolution),
                    INITIAL_AMPLITUDE_MPS,
                    spatial_dt_s,
                    spatial_steps,
                )?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    let temporal_resolution = 24;
    let temporal_points = [0.001, 0.0005, 0.00025, 0.000125]
        .into_iter()
        .map(|dt_s| {
            let steps = exact_step_count(TARGET_TIME_S, dt_s)?;
            Ok(TemporalDriftPoint {
                dt_s,
                steps,
                report: run_passive_taylor_green_concentration_scale_drift(
                    case_config(temporal_resolution),
                    INITIAL_AMPLITUDE_MPS,
                    dt_s,
                    steps,
                )?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    let execution_profiles = spatial_points
        .iter()
        .map(|point| point.report.solver_profile.clone())
        .chain(
            temporal_points
                .iter()
                .map(|point| point.report.solver_profile.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let campaign = PassiveConcentrationScaleDriftCampaign {
        target_time_s: TARGET_TIME_S,
        spatial_dt_s,
        spatial_points,
        temporal_resolution,
        temporal_points,
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
    let document = PassiveConcentrationScaleDriftEvidenceDocument {
        schema_id: DOCUMENT_SCHEMA_ID,
        source_revision,
        evidence,
    };

    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}

fn exact_step_count(target_time_s: f64, dt_s: f64) -> Result<usize, Box<dyn Error>> {
    if !target_time_s.is_finite() || target_time_s <= 0.0 || !dt_s.is_finite() || dt_s <= 0.0 {
        return Err("target time and dt must be finite and positive".into());
    }
    let quotient = target_time_s / dt_s;
    let rounded = quotient.round();
    if !rounded.is_finite() || rounded < 1.0 || (quotient - rounded).abs() > 1.0e-12 {
        return Err("target time must be an integer multiple of dt".into());
    }
    Ok(rounded as usize)
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

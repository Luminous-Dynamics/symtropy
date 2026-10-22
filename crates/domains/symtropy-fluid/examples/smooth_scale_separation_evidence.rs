// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-head evidence generator for smooth-control scale separation.
//!
//! This is an analytical calibration ladder for the known Taylor-Green
//! fundamental mode. It does not infer a dynamic concentration scale and does
//! not assign a resolved/unresolved threshold.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::reference::PeriodicMacConfig;
use symtropy_fluid::smooth_scale_separation::{
    SmoothControlScaleSeparationReport, measure_taylor_green_smooth_scale_separation,
};

const DOCUMENT_SCHEMA_ID: &str = "smooth-control-scale-separation-evidence-document-v0.1";
const CASE_PROFILE_ID: &str = "taylor-green-smooth-scale-separation-ladder-v0.1";

#[derive(Debug, Serialize)]
struct SmoothScaleSeparationCampaign {
    resolutions: Vec<usize>,
    points: Vec<SmoothControlScaleSeparationReport>,
}

#[derive(Debug, Serialize)]
struct SmoothScaleSeparationEvidenceDocument {
    schema_id: &'static str,
    source_revision: String,
    evidence: ContinuumEvidenceEnvelope<SmoothScaleSeparationCampaign>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;
    let resolutions = vec![8, 12, 16, 24, 32, 48];
    let points = resolutions
        .iter()
        .copied()
        .map(|resolution| measure_taylor_green_smooth_scale_separation(&case_config(resolution)))
        .collect::<Result<Vec<_>, _>>()?;

    let execution_profiles = points
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
    let campaign = SmoothScaleSeparationCampaign {
        resolutions,
        points,
    };
    let evidence = subject.bind(campaign)?;
    let document = SmoothScaleSeparationEvidenceDocument {
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

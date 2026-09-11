// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-head calibration evidence for the vorticity-gradient concentration scale.
//!
//! The Taylor-Green fixture has an exact discrete MAC/forward-difference scale,
//! so the estimator can be checked against the operator it actually uses. This
//! campaign also retains amplitude-invariance probes, a zero-vorticity null
//! control, and an alternating-face grid-scale stress control. It freezes
//! measurements only; no refinement threshold or promotion verdict is assigned.

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::reference::{PeriodicMac2d, PeriodicMacConfig};
use symtropy_fluid::vorticity_concentration_scale::{
    VorticityConcentrationScaleReport, VorticityConcentrationScaleUnavailableReason,
    VorticityConcentrationScaleValue, measure_vorticity_concentration_scale,
};

const DOCUMENT_SCHEMA_ID: &str = "vorticity-concentration-scale-evidence-document-v0.1";
const CASE_PROFILE_ID: &str = "taylor-green-vorticity-gradient-scale-calibration-v0.1";
const REFERENCE_AMPLITUDE_MPS: f64 = 0.08;
const AMPLITUDE_PROBE_RESOLUTION: usize = 24;
const CHECKERBOARD_RESOLUTION: usize = 16;

#[derive(Debug, Serialize)]
struct TaylorGreenScaleCalibrationPoint {
    resolution: usize,
    amplitude_mps: f64,
    expected_exact_discrete_length_m: f64,
    expected_continuum_length_m: f64,
    measured_length_m: f64,
    relative_error_to_exact_discrete: f64,
    relative_deviation_from_continuum: f64,
    report: VorticityConcentrationScaleReport,
}

#[derive(Debug, Serialize)]
struct GridScaleStressControl {
    resolution: usize,
    expected_exact_discrete_length_m: f64,
    measured_length_m: f64,
    relative_error_to_exact_discrete: f64,
    report: VorticityConcentrationScaleReport,
}

#[derive(Debug, Serialize)]
struct VorticityConcentrationScaleCampaign {
    resolution_points: Vec<TaylorGreenScaleCalibrationPoint>,
    amplitude_invariance_points: Vec<TaylorGreenScaleCalibrationPoint>,
    uniform_flow_null_control: VorticityConcentrationScaleReport,
    alternating_face_grid_scale_control: GridScaleStressControl,
}

#[derive(Debug, Serialize)]
struct VorticityConcentrationScaleEvidenceDocument {
    schema_id: &'static str,
    source_revision: String,
    evidence: ContinuumEvidenceEnvelope<VorticityConcentrationScaleCampaign>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_revision = env::var("SYMTROPY_EVIDENCE_SOURCE_REVISION")?;

    let resolution_points = [8, 12, 16, 24, 32, 48]
        .into_iter()
        .map(|resolution| calibration_point(resolution, REFERENCE_AMPLITUDE_MPS))
        .collect::<Result<Vec<_>, _>>()?;

    let amplitude_invariance_points = [0.02, 0.04, 0.08, 0.16]
        .into_iter()
        .map(|amplitude_mps| calibration_point(AMPLITUDE_PROBE_RESOLUTION, amplitude_mps))
        .collect::<Result<Vec<_>, _>>()?;

    let uniform_state =
        PeriodicMac2d::uniform(case_config(AMPLITUDE_PROBE_RESOLUTION), 0.08, -0.03)?;
    let uniform_flow_null_control = measure_vorticity_concentration_scale(&uniform_state)?;
    if uniform_flow_null_control.scale
        != VorticityConcentrationScaleValue::Unavailable(
            VorticityConcentrationScaleUnavailableReason::ZeroVorticity,
        )
    {
        return Err(std::io::Error::other(
            "uniform-flow null control did not produce typed ZeroVorticity evidence",
        )
        .into());
    }

    let alternating_face_grid_scale_control = checkerboard_control()?;

    let execution_profiles = resolution_points
        .iter()
        .chain(&amplitude_invariance_points)
        .map(|point| point.report.solver_profile.clone())
        .chain([
            uniform_flow_null_control.solver_profile.clone(),
            alternating_face_grid_scale_control
                .report
                .solver_profile
                .clone(),
        ])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let campaign = VorticityConcentrationScaleCampaign {
        resolution_points,
        amplitude_invariance_points,
        uniform_flow_null_control,
        alternating_face_grid_scale_control,
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
    let document = VorticityConcentrationScaleEvidenceDocument {
        schema_id: DOCUMENT_SCHEMA_ID,
        source_revision,
        evidence,
    };

    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}

fn calibration_point(
    resolution: usize,
    amplitude_mps: f64,
) -> Result<TaylorGreenScaleCalibrationPoint, Box<dyn Error>> {
    let config = case_config(resolution);
    let dx_m = config.dx();
    let k_per_m = std::f64::consts::TAU / config.length_x_m;
    let discrete_k_per_m = 2.0 * (0.5 * k_per_m * dx_m).sin() / dx_m;
    let expected_exact_discrete_length_m =
        1.0 / (2.0_f64.sqrt() * discrete_k_per_m.abs());
    let expected_continuum_length_m = 1.0 / (2.0_f64.sqrt() * k_per_m);

    let state = PeriodicMac2d::taylor_green(config, amplitude_mps)?;
    let report = measure_vorticity_concentration_scale(&state)?;
    let measured_length_m = measured_length(&report, "Taylor-Green calibration")?;
    let relative_error_to_exact_discrete =
        (measured_length_m - expected_exact_discrete_length_m).abs()
            / expected_exact_discrete_length_m;
    let relative_deviation_from_continuum =
        (measured_length_m - expected_continuum_length_m).abs() / expected_continuum_length_m;

    validate_nonnegative_finite(
        "expected_exact_discrete_length_m",
        expected_exact_discrete_length_m,
    )?;
    validate_nonnegative_finite("expected_continuum_length_m", expected_continuum_length_m)?;
    validate_nonnegative_finite("measured_length_m", measured_length_m)?;
    validate_nonnegative_finite(
        "relative_error_to_exact_discrete",
        relative_error_to_exact_discrete,
    )?;
    validate_nonnegative_finite(
        "relative_deviation_from_continuum",
        relative_deviation_from_continuum,
    )?;

    Ok(TaylorGreenScaleCalibrationPoint {
        resolution,
        amplitude_mps,
        expected_exact_discrete_length_m,
        expected_continuum_length_m,
        measured_length_m,
        relative_error_to_exact_discrete,
        relative_deviation_from_continuum,
        report,
    })
}

fn checkerboard_control() -> Result<GridScaleStressControl, Box<dyn Error>> {
    let config = case_config(CHECKERBOARD_RESOLUTION);
    let dx_m = config.dx();
    let cells = config.nx * config.ny;
    let u_faces = vec![0.0; cells];
    let mut v_faces = vec![0.0; cells];
    for j in 0..config.ny {
        for i in 0..config.nx {
            v_faces[j * config.nx + i] = if i % 2 == 0 { 0.08 } else { -0.08 };
        }
    }
    let state = PeriodicMac2d::from_faces(config, u_faces, v_faces)?;
    let report = measure_vorticity_concentration_scale(&state)?;
    let measured_length_m = measured_length(&report, "alternating-face grid-scale control")?;
    let expected_exact_discrete_length_m = 0.5 * dx_m;
    let relative_error_to_exact_discrete =
        (measured_length_m - expected_exact_discrete_length_m).abs()
            / expected_exact_discrete_length_m;
    validate_nonnegative_finite(
        "checkerboard_expected_exact_discrete_length_m",
        expected_exact_discrete_length_m,
    )?;
    validate_nonnegative_finite("checkerboard_measured_length_m", measured_length_m)?;
    validate_nonnegative_finite(
        "checkerboard_relative_error_to_exact_discrete",
        relative_error_to_exact_discrete,
    )?;

    Ok(GridScaleStressControl {
        resolution: CHECKERBOARD_RESOLUTION,
        expected_exact_discrete_length_m,
        measured_length_m,
        relative_error_to_exact_discrete,
        report,
    })
}

fn measured_length(
    report: &VorticityConcentrationScaleReport,
    context: &'static str,
) -> Result<f64, Box<dyn Error>> {
    match report.scale {
        VorticityConcentrationScaleValue::Measured { length_m, .. } => Ok(length_m),
        VorticityConcentrationScaleValue::Unavailable(reason) => Err(std::io::Error::other(
            format!("{context} concentration scale unavailable: {reason:?}"),
        )
        .into()),
    }
}

fn validate_nonnegative_finite(name: &'static str, value: f64) -> Result<(), Box<dyn Error>> {
    if !value.is_finite() || value < 0.0 {
        return Err(std::io::Error::other(format!(
            "non-finite/negative calibration metric {name}"
        ))
        .into());
    }
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

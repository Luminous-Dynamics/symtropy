// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeSet;
use std::env;
use std::error::Error;

use serde::Serialize;
use symtropy_fluid::diagnostic_trace::{
    ContinuumDiagnosticTraceReport, run_manufactured_taylor_green_diagnostic_trace,
    run_passive_taylor_green_diagnostic_trace,
};
use symtropy_fluid::evidence::{
    CONTINUUM_EVIDENCE_SCHEMA_VERSION, ContinuumEvidenceEnvelope, ContinuumEvidenceSubject,
};
use symtropy_fluid::falsification::{
    VerificationCampaignReport, run_passive_taylor_green_campaign,
};
use symtropy_fluid::iterative_error::{
    IterativeErrorSweepReport, run_projection_iterative_error_sweep,
};
use symtropy_fluid::manufactured::ManufacturedTaylorGreenProfile;
use symtropy_fluid::numerical_observability::{
    ProjectionIterationSweepReport, StabilityProbeReport, run_passive_taylor_green_stability_probe,
    run_projection_iteration_sweep,
};
use symtropy_fluid::reference::PeriodicMacConfig;
use symtropy_fluid::verification_ladder::{
    EnergyTraceReport, LadderCaseSpec, RefinementAxis, VerificationLadderReport,
    run_manufactured_taylor_green_ladder, run_passive_taylor_green_ladder,
    run_unforced_energy_trace,
};

const EVIDENCE_BUNDLE_SCHEMA_ID: &str = "continuum-verification-evidence-bundle-v0.5";

#[derive(Debug, Serialize)]
struct EvidenceBundle {
    schema_id: &'static str,
    source_revision: String,
    passive_spatial: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    passive_temporal: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    manufactured_spatial: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    manufactured_temporal: ContinuumEvidenceEnvelope<VerificationLadderReport>,
    unforced_energy: ContinuumEvidenceEnvelope<EnergyTraceReport>,
    passive_stability_campaign: ContinuumEvidenceEnvelope<VerificationCampaignReport>,
    passive_diagnostic_trace: ContinuumEvidenceEnvelope<ContinuumDiagnosticTraceReport>,
    manufactured_diagnostic_trace: ContinuumEvidenceEnvelope<ContinuumDiagnosticTraceReport>,
    passive_stability_probe: ContinuumEvidenceEnvelope<StabilityProbeReport>,
    projection_iteration_sweep: ContinuumEvidenceEnvelope<ProjectionIterationSweepReport>,
    projection_iterative_error_sweep: ContinuumEvidenceEnvelope<IterativeErrorSweepReport>,
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

    // Hold final time and step count constant across this ladder so the measured
    // change is spatial refinement rather than a hidden mixture of dx and dt.
    let manufactured_spatial = run_manufactured_taylor_green_ladder(
        base.clone(),
        manufactured_profile,
        0.004,
        RefinementAxis::Spatial,
        &[
            LadderCaseSpec {
                resolution: 8,
                steps: 64,
            },
            LadderCaseSpec {
                resolution: 16,
                steps: 64,
            },
            LadderCaseSpec {
                resolution: 32,
                steps: 64,
            },
        ],
    )?;

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

    let mut energy_config = base.clone();
    energy_config.nx = 24;
    energy_config.ny = 24;
    let unforced_energy = run_unforced_energy_trace(energy_config, 0.08, 0.00025, 16)?;

    // A deliberately mixed temporal campaign: the first large-dt case is
    // expected to violate the declared CFL envelope, while the refined cases
    // should be admissible. The failure is retained as evidence rather than
    // aborting or being confused with mathematical singular behavior.
    let stability_cases = [
        LadderCaseSpec {
            resolution: 12,
            steps: 1,
        },
        LadderCaseSpec {
            resolution: 12,
            steps: 64,
        },
        LadderCaseSpec {
            resolution: 12,
            steps: 128,
        },
    ];
    let passive_stability_campaign = run_passive_taylor_green_campaign(
        base.clone(),
        0.5,
        0.5,
        RefinementAxis::Temporal,
        &stability_cases,
    )?;

    // Small admitted traces retain within-case evolution rather than only the
    // final ladder point. They intentionally use the same transparent CPU
    // reference profile and are still measurement-only evidence.
    let mut trace_config = base.clone();
    trace_config.nx = 16;
    trace_config.ny = 16;
    let passive_diagnostic_trace = run_passive_taylor_green_diagnostic_trace(
        trace_config.clone(),
        0.08,
        0.0005,
        8,
    )?;
    let manufactured_diagnostic_trace = run_manufactured_taylor_green_diagnostic_trace(
        trace_config,
        manufactured_profile,
        0.0005,
        8,
    )?;

    // Probe candidate timesteps from the exact same initial state. The first is
    // deliberately outside the declared CFL envelope; the smaller values retain
    // signed distance to every explicit stability boundary.
    let mut observability_config = base.clone();
    observability_config.nx = 12;
    observability_config.ny = 12;
    let passive_stability_probe = run_passive_taylor_green_stability_probe(
        observability_config.clone(),
        0.5,
        &[0.5, 0.05, 0.005],
    )?;

    // Project the same deterministic divergent field using only different fixed
    // Jacobi iteration counts. This records residual/divergence convergence.
    let projection_iteration_sweep = run_projection_iteration_sweep(
        observability_config.clone(),
        0.08,
        0.001,
        &[1, 4, 16, 64, 256],
    )?;

    // Repeat that controlled sweep while retaining the projected face velocities.
    // Differences are measured against the highest-iteration result in the same
    // sweep, which is a numerical reference and explicitly not continuum truth.
    let projection_iterative_error_sweep = run_projection_iterative_error_sweep(
        observability_config,
        0.08,
        0.001,
        &[1, 4, 16, 64, 256],
    )?;

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
        manufactured_spatial: bind_ladder(
            &source_revision,
            "manufactured-taylor-green-spatial-v0.1",
            manufactured_spatial,
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
        passive_stability_campaign: bind_campaign(
            &source_revision,
            "passive-taylor-green-stability-campaign-v0.1",
            &base,
            &stability_cases,
            passive_stability_campaign,
        )?,
        passive_diagnostic_trace: bind_trace(
            &source_revision,
            "passive-taylor-green-diagnostic-trace-v0.1",
            passive_diagnostic_trace,
        )?,
        manufactured_diagnostic_trace: bind_trace(
            &source_revision,
            "manufactured-taylor-green-diagnostic-trace-v0.1",
            manufactured_diagnostic_trace,
        )?,
        passive_stability_probe: bind_stability_probe(
            &source_revision,
            "passive-taylor-green-stability-probe-v0.1",
            passive_stability_probe,
        )?,
        projection_iteration_sweep: bind_projection_sweep(
            &source_revision,
            "periodic-compressive-projection-iteration-sweep-v0.1",
            projection_iteration_sweep,
        )?,
        projection_iterative_error_sweep: bind_iterative_error_sweep(
            &source_revision,
            "periodic-compressive-projection-iterative-error-v0.1",
            projection_iterative_error_sweep,
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

fn bind_campaign(
    source_revision: &str,
    case_profile: &str,
    base_config: &PeriodicMacConfig,
    cases: &[LadderCaseSpec],
    report: VerificationCampaignReport,
) -> Result<ContinuumEvidenceEnvelope<VerificationCampaignReport>, Box<dyn Error>> {
    let execution_profiles = cases
        .iter()
        .map(|case| {
            let mut config = base_config.clone();
            config.nx = case.resolution;
            config.ny = case.resolution;
            config.profile_identity()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(subject(source_revision, case_profile, execution_profiles).bind(report)?)
}

fn bind_trace(
    source_revision: &str,
    case_profile: &str,
    report: ContinuumDiagnosticTraceReport,
) -> Result<ContinuumEvidenceEnvelope<ContinuumDiagnosticTraceReport>, Box<dyn Error>> {
    let execution_profiles = vec![report.solver_profile.clone()];
    Ok(subject(source_revision, case_profile, execution_profiles).bind(report)?)
}

fn bind_stability_probe(
    source_revision: &str,
    case_profile: &str,
    report: StabilityProbeReport,
) -> Result<ContinuumEvidenceEnvelope<StabilityProbeReport>, Box<dyn Error>> {
    let execution_profiles = vec![report.solver_profile.clone()];
    Ok(subject(source_revision, case_profile, execution_profiles).bind(report)?)
}

fn bind_projection_sweep(
    source_revision: &str,
    case_profile: &str,
    report: ProjectionIterationSweepReport,
) -> Result<ContinuumEvidenceEnvelope<ProjectionIterationSweepReport>, Box<dyn Error>> {
    let execution_profiles = report
        .points
        .iter()
        .map(|point| point.solver_profile.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(subject(source_revision, case_profile, execution_profiles).bind(report)?)
}

fn bind_iterative_error_sweep(
    source_revision: &str,
    case_profile: &str,
    report: IterativeErrorSweepReport,
) -> Result<ContinuumEvidenceEnvelope<IterativeErrorSweepReport>, Box<dyn Error>> {
    let execution_profiles = report
        .points
        .iter()
        .map(|point| point.solver_profile.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
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

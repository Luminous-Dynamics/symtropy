use liu_ice_control_response_evaluator::maintenance_effect::{
    AssignedArm, AssignmentMethod, BlockedRandomizedMaintenanceTrial, CalibrationStatus,
    MeasurementQuality, OutcomeDirection, RandomizedUnitObservation, StratumAllocation,
};
use liu_ice_control_response_evaluator::monte_carlo_randomization::{
    MonteCarloRandomizationConfig, estimate_monte_carlo_randomization_p_value,
};
use std::error::Error;
use std::time::Instant;

const DRAW_COUNT: usize = 1_000;
const SEED: u64 = 0x2026_11_14;

fn digest(value: u64) -> String {
    format!("{value:064x}")
}

fn synthetic_trial(unit_count: usize) -> BlockedRandomizedMaintenanceTrial {
    assert!(unit_count >= 4);
    let treatment_count = unit_count / 2;
    let control_count = unit_count - treatment_count;
    let mut observations = Vec::with_capacity(unit_count);

    for index in 0..unit_count {
        let id = format!("benchmark-asset-{index:05}");
        let suffix = (index as u64).saturating_mul(8).saturating_add(10);
        let assigned_arm = if index < treatment_count {
            AssignedArm::Treatment
        } else {
            AssignedArm::Control
        };
        let baseline_value = 75.0 + (index % 17) as f64;
        let observed_outcome_value = baseline_value
            + if assigned_arm == AssignedArm::Treatment {
                2.5
            } else {
                0.0
            };

        observations.push(RandomizedUnitObservation {
            unit_id: id.clone(),
            stratum_id: "benchmark-load-band".into(),
            assigned_arm,
            assignment_evidence_sha256: digest(suffix),
            assignment_available_at_seconds: 99.0,
            baseline_value,
            baseline_unit: "normalized_capability".into(),
            baseline_observed_at_seconds: 80.0,
            baseline_available_at_seconds: 90.0,
            baseline_evidence_sha256: digest(suffix + 1),
            baseline_source_id: format!("baseline-source-{id}"),
            baseline_measurement_principle_id: "calibrated-shaft-torque".into(),
            baseline_dependency_domain_id: format!("baseline-daq-{index}"),
            baseline_quality: MeasurementQuality::Valid,
            baseline_calibration_status: CalibrationStatus::Verified,
            baseline_calibration_record_sha256: digest(suffix + 2),
            baseline_calibration_valid_from_seconds: 0.0,
            baseline_calibration_valid_until_seconds: 100_000.0,
            baseline_uncertainty_abs: 0.1,
            observed_outcome_value,
            outcome_unit: "normalized_capability".into(),
            outcome_observed_at_seconds: 210.0,
            outcome_available_at_seconds: 220.0,
            outcome_evidence_sha256: digest(suffix + 3),
            outcome_source_id: format!("outcome-source-{id}"),
            outcome_measurement_principle_id: "calibrated-shaft-torque".into(),
            outcome_dependency_domain_id: format!("outcome-daq-{index}"),
            outcome_quality: MeasurementQuality::Valid,
            outcome_calibration_status: CalibrationStatus::Verified,
            outcome_calibration_record_sha256: digest(suffix + 4),
            outcome_calibration_valid_from_seconds: 0.0,
            outcome_calibration_valid_until_seconds: 100_000.0,
            outcome_uncertainty_abs: 0.25,
        });
    }

    BlockedRandomizedMaintenanceTrial {
        schema_version: "symtropy-marine-maintenance-effect-v0.1".into(),
        trial_id: format!("monte-carlo-throughput-{unit_count}"),
        assignment_method: AssignmentMethod::BlockRandomized,
        treatment_label: "synthetic-repair".into(),
        control_label: "synthetic-control".into(),
        outcome_metric_id: "normalized-propulsion-capability".into(),
        outcome_unit: "normalized_capability".into(),
        outcome_measurement_principle_id: "calibrated-shaft-torque".into(),
        outcome_direction: OutcomeDirection::HigherIsBetter,
        assignment_at_seconds: 100.0,
        followup_horizon_seconds: 100.0,
        analysis_as_of_seconds: 250.0,
        confidence_level: 0.95,
        assignment_plan_sha256: "a".repeat(64),
        assignment_audit_ref: "synthetic-benchmark-assignment-plan".into(),
        stratum_allocations: vec![StratumAllocation {
            stratum_id: "benchmark-load-band".into(),
            planned_treatment_count: treatment_count,
            planned_control_count: control_count,
        }],
        observations,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let unit_counts = [16usize, 32, 64, 128, 256, 1_000, 10_000];
    for unit_count in unit_counts {
        let trial = synthetic_trial(unit_count);
        let config = MonteCarloRandomizationConfig {
            seed_u64: SEED,
            draw_count: DRAW_COUNT,
        };

        let started = Instant::now();
        let result = estimate_monte_carlo_randomization_p_value(&trial, config)?;
        let elapsed = started.elapsed();
        let seconds = elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
        let unit_draws = unit_count.saturating_mul(DRAW_COUNT);

        println!(
            "{}",
            serde_json::json!({
                "unit_count": unit_count,
                "draw_count": result.draw_count,
                "seed_u64": result.seed_u64,
                "algorithm": result.algorithm,
                "unit_draws": unit_draws,
                "elapsed_micros": elapsed.as_micros(),
                "draws_per_second": DRAW_COUNT as f64 / seconds,
                "unit_draws_per_second": unit_draws as f64 / seconds,
                "observed_treatment_minus_control": result.observed_treatment_minus_control,
                "exceedance_count": result.exceedance_count,
                "monte_carlo_p_value_plus_one": result.p_value_plus_one,
                "tail_probability_estimate": result.tail_probability_estimate,
                "monte_carlo_standard_error": result.monte_carlo_standard_error,
                "tail_probability_wilson_95": result.tail_probability_wilson_95,
                "randomization_frame_sha256": result.randomization_frame_sha256,
                "sampled_assignments_sha256": result.sampled_assignments_sha256
            })
        );
    }
    Ok(())
}

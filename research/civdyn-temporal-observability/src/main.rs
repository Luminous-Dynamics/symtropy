// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_temporal_observability::{
    evaluate_temporal_observability, EvidenceView, LatentOutcome, MissingCause,
    ObservationRecord, TemporalObservabilityConfig, DECISION_CUTOFF_TICK,
    RETROSPECTIVE_AS_OF_TICK,
};
use std::error::Error;

fn fixture() -> (TemporalObservabilityConfig, Vec<LatentOutcome>, Vec<ObservationRecord>) {
    let config = TemporalObservabilityConfig {
        scenario_id: "temporal-observability-four-period-v1".to_owned(),
        source_digest: "sha256-temporal-observability-fixture".to_owned(),
        service_unit: "service_units".to_owned(),
        event_time_unit: "logical_ticks".to_owned(),
        decision_cutoff_tick: DECISION_CUTOFF_TICK,
        retrospective_as_of_tick: RETROSPECTIVE_AS_OF_TICK,
    };
    let mut outcomes = Vec::new();
    let mut observations = Vec::new();

    for world in ["moderate_decay", "severe_decay"] {
        for tick in 0..4 {
            outcomes.push(LatentOutcome {
                policy_id: "transparent_audit".to_owned(),
                world_id: world.to_owned(),
                event_tick: tick,
                true_service_units: 60,
            });
            observations.push(ObservationRecord {
                policy_id: "transparent_audit".to_owned(),
                world_id: world.to_owned(),
                event_tick: tick,
                observed_service_units: Some(60),
                ingest_tick: Some(tick),
                missing_cause: None,
                capture_receipt_digest: format!("sha256-capture-transparent-{world}-{tick}"),
                delivery_receipt_digest: Some(format!("sha256-delivery-transparent-{world}-{tick}")),
            });
        }
    }

    for (world, trajectory) in [
        ("moderate_decay", [100, 20, 20, 50]),
        ("severe_decay", [100, 0, 0, 50]),
    ] {
        for tick in 0..4 {
            let value = trajectory[tick as usize];
            outcomes.push(LatentOutcome {
                policy_id: "defer_suppress".to_owned(),
                world_id: world.to_owned(),
                event_tick: tick,
                true_service_units: value,
            });
            let (observed, ingest_tick, missing_cause) = match tick {
                0 => (Some(value), Some(0), None),
                1 | 2 => (None, None, Some(MissingCause::PolicySuppressed)),
                3 => (Some(value), Some(5), None),
                _ => unreachable!(),
            };
            observations.push(ObservationRecord {
                policy_id: "defer_suppress".to_owned(),
                world_id: world.to_owned(),
                event_tick: tick,
                observed_service_units: observed,
                ingest_tick,
                missing_cause,
                capture_receipt_digest: format!("sha256-capture-defer-{world}-{tick}"),
                delivery_receipt_digest: observed.map(|_| format!("sha256-delivery-defer-{world}-{tick}")),
            });
        }
    }
    (config, outcomes, observations)
}

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn emit(fields: &[String]) {
    println!("{}", fields.iter().map(|x| csv_field(x)).collect::<Vec<_>>().join(","));
}

fn evaluate_fixture() -> Result<civdyn_temporal_observability::TemporalObservabilityReport, String> {
    let (config, outcomes, observations) = fixture();
    evaluate_temporal_observability(&config, &outcomes, &observations).map_err(|error| format!("{error:?}"))
}

fn emit_summary() -> Result<(), String> {
    let report = evaluate_fixture()?;
    println!("scenario_id,policy_id,world_id,evidence_view,true_mean,complete_case_mean,missing_as_zero_mean,identified_lower,identified_upper,truth_contained,observed_periods,total_periods,coverage,missing_periods,late_not_yet_available_periods,observability_debt_service_periods,robust_comparison");
    for row in &report.summaries {
        let comparison = match row.view {
            EvidenceView::DecisionTime => report.decision_time_comparison_by_world[&row.world_id].id(),
            EvidenceView::Retrospective => report.retrospective_comparison_by_world[&row.world_id].id(),
        };
        emit(&[
            row.scenario_id.clone(),
            row.policy_id.clone(),
            row.world_id.clone(),
            row.view.id().to_owned(),
            format!("{:.3}", row.true_mean_service_units),
            row.complete_case_mean_service_units.map_or_else(String::new, |v| format!("{v:.3}")),
            format!("{:.3}", row.missing_as_zero_mean_service_units),
            format!("{:.3}", row.identified_interval.lower_mean()),
            format!("{:.3}", row.identified_interval.upper_mean()),
            row.truth_contained.to_string(),
            row.observed_periods.to_string(),
            row.total_periods.to_string(),
            format!("{:.3}", row.coverage),
            row.missing_periods.to_string(),
            row.late_not_yet_available_periods.to_string(),
            row.observability_debt_service_periods.to_string(),
            comparison.to_owned(),
        ]);
    }
    Ok(())
}

fn emit_candidate() -> Result<(), String> {
    let report = evaluate_fixture()?;
    println!("scenario_id,policy_id,event_tick,decision_status,observed_service_units,decision_cutoff_tick");
    for row in &report.candidate_observations {
        emit(&[
            row.scenario_id.clone(),
            row.policy_id.clone(),
            row.event_tick.to_string(),
            row.decision_status.clone(),
            row.observed_service_units.map_or_else(String::new, |v| v.to_string()),
            row.decision_cutoff_tick.to_string(),
        ]);
    }
    Ok(())
}

fn emit_evaluator_receipts() -> Result<(), String> {
    let report = evaluate_fixture()?;
    println!("policy_id,world_id,event_tick,decision_status,retrospective_status,ingest_tick,missing_cause,capture_receipt_digest,delivery_receipt_digest,scenario_id");
    for row in &report.evaluator_event_receipts {
        emit(&[
            row.policy_id.clone(),
            row.world_id.clone(),
            row.event_tick.to_string(),
            row.decision_status.clone(),
            row.retrospective_status.clone(),
            row.ingest_tick.map_or_else(String::new, |v| v.to_string()),
            row.missing_cause.as_ref().map_or_else(|| "none".to_owned(), |cause| cause.id().to_owned()),
            row.capture_receipt_digest.clone(),
            row.delivery_receipt_digest.clone().unwrap_or_default(),
            report.scenario_id.clone(),
        ]);
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let mode = std::env::args().nth(1);
    match mode.as_deref() {
        None | Some("--summary") => emit_summary(),
        Some("--candidate") => emit_candidate(),
        Some("--evaluator-receipts") => emit_evaluator_receipts(),
        Some(other) => Err(format!(
            "unknown output mode {other}; expected --summary, --candidate or --evaluator-receipts"
        )),
    }
}
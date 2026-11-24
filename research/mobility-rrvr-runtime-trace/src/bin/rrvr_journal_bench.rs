// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic, non-mutating storage benchmark for the RRVR durable journal.
//!
//! This measures append-plus-sync and read-only replay/projection costs. It
//! never calls ROS, Gazebo, a fault injector or native authority. Results are
//! workload measurements, not durability or physical-safety certification.

use mobility_rrvr_runtime_trace::durable_journal::{
    DispatchIntentRecord, DispatchOutcomeRecord, DispatchOutcomeSource, DispatchOutcomeStatus,
    Journal, ResetIntentRecord, ResetOutcomeRecord, ResetOutcomeSource, ResetOutcomeStatus,
    recovery_projection_file,
};
use mobility_rrvr_runtime_trace::gazebo_adapter::{
    GazeboDispatchRequest, dispatch_request_fingerprint,
};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const PLAN_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PROFILE_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const INITIAL_STATE_SHA256: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OBSERVATION_SHA256: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PARAMETERS_SHA256: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn percentile_ns(samples: &[u128], percentile: f64) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = ((percentile * sorted.len() as f64).ceil() as usize).max(1);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn default_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "rrvr-journal-bench-{}-{}.jsonl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ))
}

fn usage() -> ExitCode {
    eprintln!("usage: rrvr_journal_bench <dispatch-count> [journal.jsonl]");
    ExitCode::from(2)
}

fn run(dispatch_count: u32, path: PathBuf) -> Result<serde_json::Value, String> {
    if dispatch_count > 1_000_000 {
        return Err("dispatch-count exceeds the 1,000,000 safety bound".into());
    }
    let expected_entries = u64::from(dispatch_count)
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .ok_or_else(|| "expected entry count overflow".to_owned())?;

    let mut append_samples = Vec::with_capacity(dispatch_count as usize * 2 + 2);
    let append_started = Instant::now();
    {
        let mut journal = Journal::open(&path).map_err(|error| error.to_string())?;

        let started = Instant::now();
        journal
            .append_reset_intent(ResetIntentRecord {
                reset_id: "bench-reset-1".into(),
                plan_sha256: PLAN_SHA256.into(),
                simulator_profile_sha256: PROFILE_SHA256.into(),
                world_id: "bench-world".into(),
                prior_world_epoch: "bench-epoch-0".into(),
                clock_id: "bench-clock".into(),
                expected_initial_state_sha256: INITIAL_STATE_SHA256.into(),
                run_id: "bench-run".into(),
                run_epoch: "bench-run-epoch".into(),
            })
            .map_err(|error| error.to_string())?;
        append_samples.push(started.elapsed().as_nanos());

        let started = Instant::now();
        journal
            .append_reset_outcome(ResetOutcomeRecord {
                reset_id: "bench-reset-1".into(),
                source: ResetOutcomeSource::ReadOnlyObservation,
                status: ResetOutcomeStatus::Verified,
                observed_world_epoch: Some("bench-epoch-1".into()),
                observed_clock_id: Some("bench-clock".into()),
                observed_initial_state_sha256: Some(INITIAL_STATE_SHA256.into()),
                observation_artifact_sha256: Some(OBSERVATION_SHA256.into()),
                transport_response_artifact_sha256: None,
            })
            .map_err(|error| error.to_string())?;
        append_samples.push(started.elapsed().as_nanos());

        for index in 0..dispatch_count {
            let idempotency_key = format!("rrvr-bench-key-{index:010}");
            let fault_sequence = index;
            let scheduled_step = u64::from(index);
            let request = GazeboDispatchRequest {
                plan_sha256: PLAN_SHA256.into(),
                campaign_id: "bench-campaign".into(),
                pair_id: "bench-pair".into(),
                scenario_id: "bench-scenario".into(),
                fault_id: "bench-fault".into(),
                fault_sequence,
                world_epoch: "bench-epoch-1".into(),
                clock_id: "bench-clock".into(),
                target_id: "bench-target".into(),
                fault_domain_id: "bench-injector".into(),
                scheduled_simulation_step: scheduled_step,
                parameters_sha256: PARAMETERS_SHA256.into(),
                payload_sha256: String::new(),
                idempotency_key: idempotency_key.clone(),
            };
            let payload_sha256 = dispatch_request_fingerprint(&request)
                .map_err(|error| format!("dispatch request fingerprint: {error:?}"))?;

            let started = Instant::now();
            journal
                .append_dispatch_intent(DispatchIntentRecord {
                    idempotency_key: idempotency_key.clone(),
                    payload_sha256: payload_sha256.clone(),
                    reset_id: "bench-reset-1".into(),
                    plan_sha256: PLAN_SHA256.into(),
                    simulator_profile_sha256: PROFILE_SHA256.into(),
                    world_id: "bench-world".into(),
                    logical_effect_id: format!("bench-effect-{index:010}"),
                    attempt_id: format!("bench-attempt-{index:010}"),
                    campaign_id: "bench-campaign".into(),
                    pair_id: "bench-pair".into(),
                    scenario_id: "bench-scenario".into(),
                    fault_id: "bench-fault".into(),
                    fault_sequence,
                    run_id: "bench-run".into(),
                    run_epoch: "bench-run-epoch".into(),
                    world_epoch: "bench-epoch-1".into(),
                    clock_id: "bench-clock".into(),
                    target_id: "bench-target".into(),
                    fault_domain_id: "bench-injector".into(),
                    scheduled_simulation_step: scheduled_step,
                    parameters_sha256: PARAMETERS_SHA256.into(),
                })
                .map_err(|error| error.to_string())?;
            append_samples.push(started.elapsed().as_nanos());

            let started = Instant::now();
            journal
                .append_dispatch_outcome(DispatchOutcomeRecord {
                    idempotency_key,
                    payload_sha256,
                    source: DispatchOutcomeSource::TransportResponse,
                    status: DispatchOutcomeStatus::ResponseUnknown,
                    dispatch_id: None,
                    transport_response_artifact_sha256: None,
                    ledger_artifact_sha256: None,
                })
                .map_err(|error| error.to_string())?;
            append_samples.push(started.elapsed().as_nanos());
        }
        if journal.summary().entry_count != expected_entries {
            return Err("journal in-memory entry count differs from expected count".into());
        }
    }
    let append_total_ns = append_started.elapsed().as_nanos();

    let replay_started = Instant::now();
    let projection = recovery_projection_file(&path).map_err(|error| error.to_string())?;
    let replay_ns = replay_started.elapsed().as_nanos();
    if projection.entry_count != expected_entries {
        return Err("replayed entry count differs from expected count".into());
    }
    if projection.dispatch_ledger_reconciliation_required.len() != dispatch_count as usize {
        return Err("recovery projection lost unresolved dispatch identities".into());
    }
    let file_bytes = fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len();
    let report = json!({
        "schema_version": "0.1",
        "benchmark": "RRVR-JRN-SYNTHETIC-APPEND-REPLAY",
        "journal_path": path.display().to_string(),
        "dispatch_count": dispatch_count,
        "journal_entry_count": expected_entries,
        "journal_bytes": file_bytes,
        "append_total_ns": append_total_ns,
        "append_records_per_second": if append_total_ns == 0 {
            0.0
        } else {
            expected_entries as f64 * 1_000_000_000.0 / append_total_ns as f64
        },
        "append_sync_latency_ns": {
            "p50": percentile_ns(&append_samples, 0.50),
            "p95": percentile_ns(&append_samples, 0.95),
            "p99": percentile_ns(&append_samples, 0.99),
            "p999": percentile_ns(&append_samples, 0.999)
        },
        "replay_projection_ns": replay_ns,
        "bytes_per_entry": file_bytes as f64 / expected_entries.max(1) as f64,
        "recovery_projection": {
            "resets": projection.resets.len(),
            "dispatches": projection.dispatches.len(),
            "reset_reconciliation_required": projection.reset_reconciliation_required.len(),
            "dispatch_ledger_reconciliation_required": projection.dispatch_ledger_reconciliation_required.len(),
            "dispatch_execution_evidence_pending": projection.dispatch_execution_evidence_pending.len(),
            "authority_created": projection.authority_created,
            "mutation_permitted": projection.mutation_permitted
        },
        "transport_calls_performed": false,
        "physical_safety_established": false,
        "claim_ceiling": "Synthetic local file workload only; not a comparative SQLite result, power-cut test, remote injector guarantee, or physical robotics result."
    });
    Ok(report)
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(dispatch_count) = args.next() else {
        return usage();
    };
    let Ok(dispatch_count) = dispatch_count.parse::<u32>() else {
        return usage();
    };
    let path = args.next().map(PathBuf::from).unwrap_or_else(default_path);
    if args.next().is_some() {
        return usage();
    }
    match run(dispatch_count, path) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("RRVR journal benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}

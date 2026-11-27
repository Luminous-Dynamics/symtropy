// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic synthetic scale probe for the three-axis replay kernel.
//!
//! Usage:
//! cargo run --locked -p symtropy-aero-material-qual --example temporal_replay_scale -- 10000 7
//!
//! Measures JSON event-envelope replay only. It excludes source ingestion,
//! durable receipt/checkpoint storage, signature/TSA verification, trust-root
//! distribution and physical observation.

use serde_json::{Value, json};
use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};

use symtropy_aero_material_qual::temporal_replay_v0_1::{
    TemporalQueryMode, evaluate_temporal_replay_case,
};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn parse_positive(value: Option<String>, default: usize, name: &str) -> Result<usize, String> {
    let value = value
        .unwrap_or_else(|| default.to_string())
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if value == 0 {
        return Err(format!("{name} must be a positive integer"));
    }
    Ok(value)
}

fn snapshot(watermark: u64, policy_id: &str, policy_digest: &str) -> Value {
    json!({
        "evaluated_at_unix_ms": watermark,
        "receiver_receipt_watermark_unix_ms": watermark,
        "trust_policy_id": policy_id,
        "trust_status_as_of_unix_ms": watermark,
        "trust_policy_digest_sha256": policy_digest,
        "evidence_checkpoint_status": "SYNTHETIC_VALID",
        "evidence_checkpoint_reference": format!("synthetic-checkpoint:{watermark}"),
        "evidence_checkpoint_digest_sha256": DIGEST,
        "evidence_checkpoint_watermark_unix_ms": watermark,
        "observer_coverage_status": "SYNTHETIC_QUALIFIED_COMPLETE",
        "observer_coverage_profile_id": "synthetic-observer-profile-v0.1",
        "observer_coverage_receipt_reference": format!("synthetic-coverage:{watermark}"),
        "observer_coverage_receipt_digest_sha256": DIGEST,
        "observer_coverage_watermark_unix_ms": watermark,
        "evidence_view_digest_sha256": DIGEST
    })
}

fn build_case(event_count: usize) -> Value {
    let events = (0..event_count)
        .map(|index| {
            let event_id = format!("EV-{index:010}");
            let payload_digest = format!("{index:064x}");
            json!({
                "event_id": event_id,
                "source_namespace": "lab-a",
                "certificate_id": "CAL-A",
                "event_type": "issued",
                "payload_sha256": payload_digest,
                "effective_at_unix_ms": 80,
                "issuer_recorded_at_unix_ms": 90,
                "receiver_first_seen_at_unix_ms": 100,
                "signature_status": "valid",
                "trusted_timestamp_status": "absent",
                "trusted_timestamp_at_unix_ms": null,
                "issuer_authority_status": "authorized",
                "sequence_status": "not_applicable",
                "source_clock_status": "consistent",
                "issuer_key_status_at_snapshot": "good",
                "issuer_key_status_current": "good",
                "compromise_effective_at_unix_ms": null,
                "corrects_event_id": null,
                "receiver_receipt_status": "SYNTHETIC_VALID",
                "receiver_receipt_reference": format!("receipt:lab-a:{event_id}"),
                "receiver_receipt_digest_sha256": payload_digest,
                "trusted_timestamp_imprint_sha256": null,
                "issuer_identity_id": "issuer:lab-a",
                "source_namespace_binding_status": "SYNTHETIC_VALID"
            })
        })
        .collect::<Vec<_>>();

    json!({
        "case_id": format!("synthetic-replay-scale-{event_count}"),
        "calibration_validity": {
            "valid_from_unix_ms": 100,
            "valid_until_unix_ms": 200
        },
        "acquisition_time_interval": {
            "earliest_unix_ms": 140,
            "latest_unix_ms": 140
        },
        "snapshot": snapshot(150, "trust-policy-v1", DIGEST),
        "current_reassessment": snapshot(220, "trust-policy-current-v1", DIGEST),
        "events": events
    })
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len().saturating_sub(1) * percentile).div_ceil(100);
    sorted[index]
}

fn run_mode(
    case: &Value,
    mode: TemporalQueryMode,
    event_count: usize,
    iterations: usize,
) -> Result<(Duration, Duration, Duration), String> {
    for _ in 0..2 {
        black_box(evaluate_temporal_replay_case(black_box(case), mode));
    }

    let mut samples = Vec::with_capacity(iterations);
    let mut final_report = None;
    for _ in 0..iterations {
        let start = Instant::now();
        let report = evaluate_temporal_replay_case(black_box(case), mode);
        samples.push(start.elapsed());
        final_report = Some(report);
    }

    let report = final_report.expect("positive iteration count");
    if report.disposition != "ACCEPTED_FOR_BINDING_ONLY"
        || report.applied_event_ids.len() != event_count
        || !report.excluded_event_ids.is_empty()
    {
        return Err(format!(
            "{} generated an unexpected result: disposition={}, applied={}, excluded={}, reasons={:?}",
            mode.as_str(),
            report.disposition,
            report.applied_event_ids.len(),
            report.excluded_event_ids.len(),
            report.reason_codes
        ));
    }

    Ok((
        percentile(&samples, 50),
        percentile(&samples, 95),
        percentile(&samples, 99),
    ))
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let event_count = parse_positive(args.next(), 10_000, "event_count")?;
    let iterations = parse_positive(args.next(), 21, "iterations")?;
    if args.next().is_some() {
        return Err("usage: temporal_replay_scale [event_count] [iterations]".to_owned());
    }

    let case = build_case(event_count);
    println!("mode,events,iterations,p50_us,p95_us,p99_us,status");
    for mode in [
        TemporalQueryMode::AsKnownAt,
        TemporalQueryMode::RecomputedWithCurrentTrust,
        TemporalQueryMode::RecomputedWithCurrentEvidenceAndTrust,
    ] {
        let (p50, p95, p99) = run_mode(&case, mode, event_count, iterations)?;
        println!(
            "{},{event_count},{iterations},{:.3},{:.3},{:.3},ACCEPTED_FOR_BINDING_ONLY",
            mode.as_str(),
            p50.as_secs_f64() * 1_000_000.0,
            p95.as_secs_f64() * 1_000_000.0,
            p99.as_secs_f64() * 1_000_000.0
        );
    }
    Ok(())
}

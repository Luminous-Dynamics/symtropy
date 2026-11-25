// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic-only scale benchmark for the AERO temporal replay reference model.
//!
//! Examples:
//! cargo bench -p symtropy-aero-material-qual --bench temporal_replay_scale -- 10000 3
//! cargo bench -p symtropy-aero-material-qual --bench temporal_replay_scale -- 10000,100000 5
//!
//! The generator is deterministic. Digests and receipt labels are synthetic
//! fixtures, not cryptographic evidence. This measures in-memory evaluator
//! cost only; it does not measure persistence, TSA, source capture, or NDE.

use serde_json::{Value, json};
use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};

use symtropy_aero_material_qual::temporal_replay_v0_1::{ReplayMode, evaluate_case};

const HISTORICAL_POLICY: &str =
    "23b6c1fc30a18db6991c1eb295ba2b186a8ad98ebcd2d4ffca7694b81dba71b7";
const CURRENT_POLICY: &str =
    "5c5005d198495cbc4455f67db17d91b0ffdea6f78ff70354270f9ccf068005f8";
const HISTORICAL_VIEW: &str =
    "1d67f61b0a1980fe23a9d3b1c7b6a7c4a6db8910e9a73a5fbc7042d6f182ad31";
const CURRENT_VIEW: &str =
    "2e78a72c1b2a90df34ba5b2d8c7b8d5b7aec9a4f0ba84b6fcd8153e7a293be42";
const HISTORICAL_COVERAGE: &str =
    "3f89b83d2c3b01e045bac6c9d8c9e6d8b7fda5b01c9c7e6c8a9c8a1d5e3f7b92";
const CURRENT_COVERAGE: &str =
    "4a90c94e3d4c12f156cbd7e0e9d0f7e9c80eb6c12dad8f7d9bad9b2e6f408ca3";

fn synthetic_event(index: usize) -> Value {
    let event_id = format!("EV-{index:012}");
    let namespace = format!("lab-{:02}", index % 32);
    let digest = format!("{index:064x}");
    json!({
        "event_id": event_id,
        "source_namespace": namespace,
        "certificate_id": "CAL-BENCH",
        "event_type": "issued",
        "payload_sha256": digest,
        "effective_at_unix_ms": 10_u64,
        "issuer_recorded_at_unix_ms": 20_u64,
        "receiver_first_seen_at_unix_ms": 30_u64,
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
        "receiver_receipt_reference": format!("bench-receipt:{namespace}:{event_id}"),
        "receiver_receipt_digest_sha256": digest,
        "trusted_timestamp_imprint_sha256": null,
        "issuer_identity_id": namespace,
        "source_namespace_binding_status": "SYNTHETIC_VALID"
    })
}

fn synthetic_case(count: usize) -> Value {
    let events = (0..count).map(synthetic_event).collect::<Vec<_>>();
    json!({
        "case_id": format!("temporal-scale-{count}"),
        "calibration_validity": {
            "valid_from_unix_ms": 0_u64,
            "valid_until_unix_ms": 1000_u64
        },
        "acquisition_time_interval": {
            "earliest_unix_ms": 100_u64,
            "latest_unix_ms": 100_u64
        },
        "snapshot": {
            "evaluated_at_unix_ms": 150_u64,
            "receiver_receipt_watermark_unix_ms": 100_u64,
            "trust_status_as_of_unix_ms": 120_u64,
            "trust_policy_id": "bench-historical-v1",
            "trust_policy_digest_sha256": HISTORICAL_POLICY,
            "evidence_checkpoint_status": "SYNTHETIC_VALID",
            "evidence_checkpoint_reference": "bench-checkpoint:historical",
            "evidence_checkpoint_digest_sha256": HISTORICAL_VIEW,
            "evidence_view_digest_sha256": HISTORICAL_VIEW,
            "evidence_checkpoint_watermark_unix_ms": 100_u64,
            "observer_coverage_status": "SYNTHETIC_QUALIFIED_COMPLETE",
            "observer_coverage_profile_id": "bench-observer-v1",
            "observer_coverage_receipt_reference": "bench-coverage:historical",
            "observer_coverage_receipt_digest_sha256": HISTORICAL_COVERAGE,
            "observer_coverage_watermark_unix_ms": 100_u64
        },
        "current_reassessment": {
            "receiver_receipt_watermark_unix_ms": 200_u64,
            "trust_status_as_of_unix_ms": 220_u64,
            "trust_policy_id": "bench-current-v1",
            "trust_policy_digest_sha256": CURRENT_POLICY,
            "evidence_checkpoint_status": "SYNTHETIC_VALID",
            "evidence_checkpoint_reference": "bench-checkpoint:current",
            "evidence_checkpoint_digest_sha256": CURRENT_VIEW,
            "evidence_view_digest_sha256": CURRENT_VIEW,
            "evidence_checkpoint_watermark_unix_ms": 200_u64,
            "observer_coverage_status": "SYNTHETIC_QUALIFIED_COMPLETE",
            "observer_coverage_profile_id": "bench-observer-v1",
            "observer_coverage_receipt_reference": "bench-coverage:current",
            "observer_coverage_receipt_digest_sha256": CURRENT_COVERAGE,
            "observer_coverage_watermark_unix_ms": 200_u64
        },
        "verifier_profile_id": "aero-temporal-replay-reference-v0.1",
        "events": events
    })
}

fn parse_counts(value: Option<String>) -> Result<Vec<usize>, String> {
    let value = value.unwrap_or_else(|| "10000".to_owned());
    let values = value.split(',')
        .map(|part| part.parse::<usize>().map_err(|_| format!("invalid event count: {part}")))
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() || values.iter().any(|count| *count == 0) {
        return Err("event counts must be positive integers".to_owned());
    }
    Ok(values)
}

fn median(values: &mut [Duration]) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let counts = parse_counts(args.next())?;
    let iterations = args.next().unwrap_or_else(|| "3".to_owned())
        .parse::<usize>().map_err(|_| "iterations must be a positive integer".to_owned())?;
    if iterations == 0 || args.next().is_some() {
        return Err("usage: temporal_replay_scale [counts_csv] [iterations]".to_owned());
    }

    eprintln!("synthetic_only=true; no real cryptography, receipts, observer qualification, or physical truth");
    println!("events,mode,iterations,median_ms,min_ms,disposition");

    let modes = [
        ReplayMode::AsKnownAt,
        ReplayMode::RecomputedWithCurrentTrust,
        ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
    ];
    for count in counts {
        let case = synthetic_case(count);
        for mode in modes {
            let mut elapsed = Vec::with_capacity(iterations);
            let mut status = String::new();
            for _ in 0..iterations {
                let start = Instant::now();
                let report = evaluate_case(black_box(&case), mode);
                elapsed.push(start.elapsed());
                status = report.disposition;
                black_box(report);
            }
            let min = *elapsed.iter().min().expect("positive iterations");
            let med = median(&mut elapsed);
            let label = match mode {
                ReplayMode::AsKnownAt => "AS_KNOWN_AT",
                ReplayMode::RecomputedWithCurrentTrust => "CURRENT_TRUST",
                ReplayMode::RecomputedWithCurrentEvidenceAndTrust => "CURRENT_EVIDENCE_AND_TRUST",
            };
            println!(
                "{count},{label},{iterations},{:.3},{:.3},{status}",
                med.as_secs_f64() * 1000.0,
                min.as_secs_f64() * 1000.0
            );
        }
    }
    Ok(())
}

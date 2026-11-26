// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic synthetic scale probe for the vector-watermark validator.
//!
//! Usage:
//! cargo run --locked -p symtropy-aero-material-qual --example vector_cut_scale -- 10000 7
//!
//! This measures typed in-memory validation only. It does not measure durable
//! storage, real receipt verification, source observation or physical truth.

use std::env;
use std::hint::black_box;
use std::time::Duration;
use std::time::Instant;

use symtropy_aero_material_qual::vector_watermark_v0_1::{
    CutAssessment, CutEvent, EventDependency, EvidenceCut, PartitionWatermark, validate_causal_cut,
};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PARTITIONS: usize = 32;

fn generate_cut(event_count: usize) -> EvidenceCut {
    let mut watermarks = Vec::with_capacity(PARTITIONS);
    let mut events = Vec::with_capacity(event_count);
    let mut identities = Vec::with_capacity(PARTITIONS);
    let partition_counts = (0..PARTITIONS)
        .map(|source| {
            event_count / PARTITIONS
                + if source < event_count % PARTITIONS {
                    1
                } else {
                    0
                }
        })
        .collect::<Vec<_>>();

    for source in 0..PARTITIONS {
        identities.push(format!("sensor-{source:02}"));
        watermarks.push(PartitionWatermark {
            source_id: identities[source].clone(),
            partition_id: "0".to_owned(),
            stream_epoch: "epoch-1".to_owned(),
            last_contiguous_sequence: partition_counts[source] as u64,
            receiver_inclusion_offset: (partition_counts[source] as u64) * 8,
            checkpoint_digest_sha256: DIGEST.to_owned(),
        });
    }

    for source in 0..PARTITIONS {
        for sequence in 1..=partition_counts[source] {
            let event_ref = format!("{}-event-{sequence:08}", identities[source]);
            let mut dependencies = Vec::with_capacity(2);

            if sequence > 1 {
                dependencies.push(EventDependency {
                    event_ref: format!("{}-event-{:08}", identities[source], sequence - 1),
                    source_id: identities[source].clone(),
                    partition_id: "0".to_owned(),
                    stream_epoch: "epoch-1".to_owned(),
                    sequence: (sequence - 1) as u64,
                });
            }

            if source > 0 && sequence > 1 && sequence % 64 == 0 {
                dependencies.push(EventDependency {
                    event_ref: format!("{}-event-{:08}", identities[source - 1], sequence - 1),
                    source_id: identities[source - 1].clone(),
                    partition_id: "0".to_owned(),
                    stream_epoch: "epoch-1".to_owned(),
                    sequence: (sequence - 1) as u64,
                });
            }

            events.push(CutEvent {
                event_ref,
                source_id: identities[source].clone(),
                partition_id: "0".to_owned(),
                stream_epoch: "epoch-1".to_owned(),
                sequence: sequence as u64,
                receiver_inclusion_offset: (sequence as u64) * 8,
                dependencies,
            });
        }
    }

    EvidenceCut {
        cut_id: format!("synthetic-vector-cut-{event_count}"),
        watermarks,
        events,
    }
}

fn percentile(values: &[Duration], percentile: usize) -> Duration {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len().saturating_sub(1) * percentile).div_ceil(100);
    sorted[index]
}

fn parse_positive(value: Option<String>, default: usize, name: &str) -> Result<usize, String> {
    let parsed = value
        .unwrap_or_else(|| default.to_string())
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be a positive integer"));
    }
    Ok(parsed)
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let event_count = parse_positive(args.next(), 10_000, "event_count")?;
    let iterations = parse_positive(args.next(), 101, "iterations")?;
    if args.next().is_some() {
        return Err("usage: vector_cut_scale [event_count] [iterations]".to_owned());
    }

    let cut = generate_cut(event_count);
    eprintln!(
        "synthetic_only=true; events={}; partitions={PARTITIONS}; iterations={iterations}",
        cut.events.len()
    );
    println!("events,partitions,iterations,p50_us,p95_us,p99_us,status,dependencies");

    for _ in 0..3 {
        black_box(validate_causal_cut(black_box(&cut)));
    }

    let mut samples = Vec::with_capacity(iterations);
    let mut final_report: Option<CutAssessment> = None;
    for _ in 0..iterations {
        let start = Instant::now();
        let report = validate_causal_cut(black_box(&cut));
        samples.push(start.elapsed());
        final_report = Some(black_box(report));
    }

    let report = final_report.expect("positive iteration count");
    println!(
        "{},{},{iterations},{:.3},{:.3},{:.3},{},{}",
        cut.events.len(),
        cut.watermarks.len(),
        percentile(&samples, 50).as_secs_f64() * 1000.0,
        percentile(&samples, 95).as_secs_f64() * 1000.0,
        percentile(&samples, 99).as_secs_f64() * 1000.0,
        report.status.as_str(),
        report.dependency_count
    );
    if report.status.as_str() != "CAUSALLY_CLOSED" {
        return Err(format!(
            "synthetic benchmark generated a non-closed cut: {:?}",
            report.reason_codes
        ));
    }
    Ok(())
}

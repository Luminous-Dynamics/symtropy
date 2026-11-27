use engineering_claim_closure::{
    AttestationStatus, Claim, ClaimClosureInput, ClosureDisposition, DependencyBinding,
    EvidenceRecord, EvidenceRequirement, ProducerAuthority, TrustSnapshot, UncertaintyBudget,
    evaluate, INPUT_SCHEMA_VERSION,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::time::Instant;

const ITERATIONS: usize = 15;

fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn peak_rss_bytes() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kilobytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    kilobytes.checked_mul(1024)
}

fn build_graph(nodes: usize, fanout: usize) -> Result<(ClaimClosureInput, usize, usize), String> {
    if nodes == 0 || fanout == 0 {
        return Err("node and fan-out counts must be nonzero".to_owned());
    }

    let references = (0..nodes)
        .map(|index| format!("evidence:node-{index:05}"))
        .collect::<Vec<_>>();
    let record_digests = (0..nodes)
        .map(|index| digest(&format!("claim-closure-benchmark-record-{index}")))
        .collect::<Vec<_>>();
    let mut records = Vec::with_capacity(nodes);
    let mut depths = vec![0_usize; nodes];
    let mut edge_count = 0_usize;

    for index in 0..nodes {
        let start = index
            .checked_mul(fanout)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| "benchmark child index overflow".to_owned())?;
        let end = start.saturating_add(fanout).min(nodes);
        let mut dependencies = Vec::with_capacity(end.saturating_sub(start));
        for child in start..end {
            dependencies.push(DependencyBinding {
                evidence_ref: references[child].clone(),
                expected_record_sha256: record_digests[child].clone(),
            });
            depths[child] = depths[index] + 1;
            edge_count += 1;
        }

        let is_root = index == 0;
        records.push(EvidenceRecord {
            evidence_ref: references[index].clone(),
            record_sha256: record_digests[index].clone(),
            role: if is_root { "model" } else { "supporting_evidence" }.to_owned(),
            subject_ref: if is_root {
                "part:benchmark-subject".to_owned()
            } else {
                format!("artifact:upstream-{index}")
            },
            configuration_ref: "configuration:benchmark-v0.1".to_owned(),
            scope_ref: "scope:synthetic-benchmark".to_owned(),
            recorded_at_unix_ms: 100,
            effective_from_unix_ms: 90,
            effective_until_exclusive_unix_ms: Some(130),
            producer_authority: ProducerAuthority {
                identity_ref: format!("producer:benchmark-{index}"),
                status: AttestationStatus::Verified,
                as_of_unix_ms: Some(100),
                record_sha256: Some(digest("synthetic-producer-authority-receipt")),
            },
            trust_snapshot: TrustSnapshot {
                policy_ref: "policy:benchmark-v0.1".to_owned(),
                status: AttestationStatus::Verified,
                as_of_unix_ms: Some(100),
                policy_sha256: Some(digest("synthetic-trust-policy")),
            },
            uncertainty_budget: UncertaintyBudget {
                budget_ref: format!("uncertainty:benchmark-{index}"),
                digest_sha256: digest(&format!("synthetic-budget-{index}")),
                resolved: true,
            },
            dependencies,
        });
    }

    let max_depth = depths.into_iter().max().unwrap_or(0);
    let input = ClaimClosureInput {
        schema_version: INPUT_SCHEMA_VERSION.to_owned(),
        claim: Claim {
            claim_id: format!("claim:scale-{nodes}-fanout-{fanout}"),
            subject_ref: "part:benchmark-subject".to_owned(),
            configuration_ref: "configuration:benchmark-v0.1".to_owned(),
            scope_ref: "scope:synthetic-benchmark".to_owned(),
            acquisition_start_unix_ms: 100,
            acquisition_end_unix_ms: 120,
            knowledge_cut_unix_ms: 150,
            evaluated_at_unix_ms: 160,
            requirements: vec![EvidenceRequirement {
                evidence_ref: references[0].clone(),
                expected_record_sha256: record_digests[0].clone(),
                role: "model".to_owned(),
                uncertainty_budget_ref: "uncertainty:benchmark-0".to_owned(),
                uncertainty_budget_sha256: digest("synthetic-budget-0"),
            }],
        },
        evidence_records: records,
    };
    Ok((input, edge_count, max_depth))
}

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn run_case(nodes: usize, fanout: usize) -> Result<Value, String> {
    let (input, edges, depth) = build_graph(nodes, fanout)?;
    for _ in 0..2 {
        let report = evaluate(&input)?;
        if report.disposition != ClosureDisposition::ClosedForIndependentReview {
            return Err(format!(
                "benchmark graph unexpectedly blocked: nodes={nodes}, fanout={fanout}, findings={:?}",
                report.findings
            ));
        }
    }

    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        let report = evaluate(&input)?;
        let elapsed = started.elapsed().as_nanos();
        if report.disposition != ClosureDisposition::ClosedForIndependentReview {
            return Err(format!(
                "benchmark graph unexpectedly blocked during measurement: nodes={nodes}, fanout={fanout}"
            ));
        }
        samples.push(elapsed);
    }
    samples.sort_unstable();
    let p50_ns = percentile(&samples, 50);
    let p95_ns = percentile(&samples, 95);
    let p99_ns = percentile(&samples, 99);
    let seconds = p50_ns as f64 / 1_000_000_000.0;
    let records_per_second = nodes as f64 / seconds.max(f64::MIN_POSITIVE);
    let edges_per_second = edges as f64 / seconds.max(f64::MIN_POSITIVE);

    Ok(json!({
        "nodes": nodes,
        "dependency_edges": edges,
        "fanout": fanout,
        "maximum_depth": depth,
        "iterations": ITERATIONS,
        "p50_latency_us": p50_ns as f64 / 1_000.0,
        "p95_latency_us": p95_ns as f64 / 1_000.0,
        "p99_latency_us": p99_ns as f64 / 1_000.0,
        "records_per_second_at_p50": records_per_second,
        "dependency_edges_per_second_at_p50": edges_per_second,
        "process_peak_rss_bytes_observed": peak_rss_bytes(),
        "disposition": "CLOSED_FOR_INDEPENDENT_REVIEW"
    }))
}

fn run() -> Result<(), String> {
    let cases = [
        (100_usize, 1_usize),
        (100, 3),
        (100, 8),
        (1_000, 3),
        (1_000, 8),
        (10_000, 3),
        (10_000, 8),
    ];
    let mut results = Vec::with_capacity(cases.len());
    for (nodes, fanout) in cases {
        results.push(run_case(nodes, fanout)?);
    }
    let report = json!({
        "benchmark_schema_version": "claim-closure-scale-benchmark-v0.1",
        "evaluator_version": "0.1.0",
        "source_commit_sha": std::env::var("GITHUB_SHA").ok(),
        "sample_count_per_case": ITERATIONS,
        "timing_method": "std::time::Instant; two warm-up evaluations excluded",
        "memory_method": "Linux /proc/self/status VmHWM when available; process high-water mark, not isolated per-case allocation",
        "performance_thresholds_enforced": false,
        "results": results
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("benchmark JSON serialization failed: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

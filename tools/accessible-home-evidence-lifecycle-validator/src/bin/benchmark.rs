// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reproducible synthetic replay benchmark. This measures evaluator time, not fixture generation.

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration as StdDuration, Instant};

use accessible_home_evidence_lifecycle_validator::{evaluate_bytes, VALIDATOR_VERSION};

#[derive(Debug)]
struct Options {
    claims: usize,
    events: usize,
    paths: usize,
    iterations: usize,
    warmup: usize,
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        claims: 10_000,
        events: 1_000,
        paths: 1_000,
        iterations: 30,
        warmup: 3,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let slot = match arg.as_str() {
            "--claims" => &mut options.claims,
            "--events" => &mut options.events,
            "--paths" => &mut options.paths,
            "--iterations" => &mut options.iterations,
            "--warmup" => &mut options.warmup,
            "--help" | "-h" => {
                println!("Usage: accessible-home-evidence-lifecycle-benchmark [--claims N] [--events N] [--paths N] [--iterations N] [--warmup N]");
                println!("Defaults: 10000 claims, 1000 events, 1000 paths, 30 timed iterations, 3 warmups.");
                return Err(String::new());
            }
            _ => return Err(format!("unknown argument {arg:?}; use --help")),
        };
        let raw = args.next().ok_or_else(|| format!("missing value after {arg}"))?;
        *slot = raw.parse().map_err(|_| format!("invalid positive integer {raw:?} after {arg}"))?;
    }

    if options.claims == 0 || options.events == 0 || options.paths == 0 {
        return Err("claims, events, and paths must be positive".into());
    }
    if options.events > options.claims {
        return Err("events must not exceed claims".into());
    }
    if options.paths > options.claims {
        return Err("paths must not exceed claims".into());
    }
    if options.iterations < 2 {
        return Err("iterations must be at least 2 to report a timing distribution".into());
    }
    if options.iterations > 10_000 || options.warmup > 10_000 {
        return Err("iterations and warmups are capped at 10000".into());
    }
    Ok(options)
}

fn build_fixture(options: &Options) -> Result<Vec<u8>, String> {
    let eval_time = DateTime::parse_from_rfc3339("2026-10-16T00:00:00Z")
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let event_start = DateTime::parse_from_rfc3339("2026-10-10T00:00:00Z")
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);

    let facets: Vec<Value> = (0..options.events)
        .map(|i| {
            json!({
                "facet_id": format!("bench-facet-{i:05}"),
                "initial_revision_id": format!("bench-rev-a-{i:05}"),
                "scope_ref": format!("synthetic://benchmark/facet-{i:05}"),
                "description": "Generated synthetic benchmark facet; no real home is represented."
            })
        })
        .collect();

    let claims: Vec<Value> = (0..options.claims)
        .map(|i| {
            let facet = i % options.events;
            let revision = if i < options.events { "a" } else { "b" };
            json!({
                "claim_id": format!("bench-claim-{i:05}"),
                "assertion_key": format!("bench-assertion-{i:05}"),
                "scope_ref": format!("synthetic://benchmark/claim-{i:05}"),
                "bindings": [{
                    "facet_id": format!("bench-facet-{facet:05}"),
                    "revision_id": format!("bench-rev-{revision}-{facet:05}")
                }],
                "evidence_state": "verified_for_declared_scope",
                "source_ref": format!("synthetic://benchmark/evidence-{i:05}"),
                "review_role": "synthetic benchmark reviewer",
                "observed_at": "2026-09-01T00:00:00Z",
                "review_due": "2027-09-01T00:00:00Z",
                "supersedes_claim_id": null,
                "expected_currentness": if i < options.events { "needs_revalidation" } else { "current" },
                "note": "Generated synthetic benchmark claim."
            })
        })
        .collect();

    let events: Vec<Value> = (0..options.events)
        .map(|i| {
            let at = event_start + Duration::seconds(i as i64);
            json!({
                "event_id": format!("bench-event-{i:05}"),
                "occurred_at": at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "source_ref": format!("synthetic://benchmark/event-{i:05}"),
                "changes": [{
                    "facet_id": format!("bench-facet-{i:05}"),
                    "from_revision_id": format!("bench-rev-a-{i:05}"),
                    "to_revision_id": format!("bench-rev-b-{i:05}")
                }],
                "note": "Generated synthetic revision change."
            })
        })
        .collect();

    let paths: Vec<Value> = (0..options.paths)
        .map(|i| {
            let stale = i < options.events;
            json!({
                "path_id": format!("bench-path-{i:05}"),
                "required_claim_ids": [format!("bench-claim-{i:05}")],
                "expected_assessment": if stale { "not_evaluable" } else { "evidence_complete_for_fixture" },
                "note": "Generated synthetic benchmark path."
            })
        })
        .collect();

    let fixture = json!({
        "schema_version": "0.1.0",
        "fixture_kind": "synthetic_evidence_lifecycle",
        "lifecycle_id": "synthetic-benchmark-run",
        "claim_boundary": "Synthetic benchmark input only. No real resident, building, device, or accessibility outcome is represented.",
        "evaluation_time": eval_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "facets": facets,
        "claims": claims,
        "change_events": events,
        "paths": paths
    });
    serde_json::to_vec(&fixture).map_err(|e| format!("could not encode generated fixture: {e}"))
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    let index = ((sorted.len() as f64 * p).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted[index]
}

fn run() -> Result<(), String> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(error) if error.is_empty() => return Ok(()),
        Err(error) => return Err(error),
    };
    let input = build_fixture(&options)?;
    let baseline = evaluate_bytes(&input)
        .map_err(|errors| format!("generated fixture rejected:\n- {}", errors.join("\n- ")))?;
    if baseline.claim_count != options.claims
        || baseline.event_count != options.events
        || baseline.path_count != options.paths
    {
        return Err("validator report counts disagree with the generated fixture".into());
    }

    let mut durations = Vec::<u128>::with_capacity(options.iterations);
    for _ in 0..options.warmup {
        black_box(
            evaluate_bytes(black_box(&input))
                .map_err(|errors| format!("warmup replay failed: {}", errors.join("; ")))?,
        );
    }
    for _ in 0..options.iterations {
        let start = Instant::now();
        let report = evaluate_bytes(black_box(&input))
            .map_err(|errors| format!("timed replay failed: {}", errors.join("; ")))?;
        let elapsed = start.elapsed();
        if report.input_sha256 != baseline.input_sha256 || report != baseline {
            return Err("replay output changed between iterations".into());
        }
        black_box(report);
        durations.push(elapsed.as_nanos());
    }
    durations.sort_unstable();
    let median_ns = percentile(&durations, 0.50);
    let p95_ns = percentile(&durations, 0.95);
    let digest = baseline.input_sha256.clone();
    let output = json!({
        "benchmark_schema_version": "0.1.0",
        "synthetic_only": true,
        "validator_version": VALIDATOR_VERSION,
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
        "profile_note": "Record cargo invocation and rustc --version --verbose alongside this report.",
        "claims": options.claims,
        "events": options.events,
        "paths": options.paths,
        "input_bytes": input.len(),
        "input_sha256": digest,
        "warmup_iterations": options.warmup,
        "timed_iterations": options.iterations,
        "min_elapsed_ns": durations[0],
        "median_elapsed_ns": median_ns,
        "p95_elapsed_ns": p95_ns,
        "max_elapsed_ns": *durations.last().unwrap_or(&0),
        "median_claims_per_second": if median_ns > 0 { options.claims as f64 * 1_000_000_000.0 / median_ns as f64 } else { Value::Null.as_f64().unwrap_or(0.0) },
        "peak_rss_bytes": null,
        "peak_rss_note": "Not available portably from this binary; collect with /usr/bin/time -v or an equivalent OS tool.",
        "measurement_boundary": "Wall time covers evaluate_bytes only; fixture generation and JSON serialization are excluded."
    });
    println!("{}", serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accessible_home_evidence_lifecycle_validator::Assessment;

    #[test]
    fn generated_benchmark_fixture_has_expected_counts_and_replays() {
        let options = Options { claims: 100, events: 10, paths: 10, iterations: 2, warmup: 0 };
        let input = build_fixture(&options).expect("fixture generation");
        let report = evaluate_bytes(&input).expect("generated fixture should replay");
        assert_eq!(report.claim_count, 100);
        assert_eq!(report.event_count, 10);
        assert_eq!(report.path_count, 10);
        assert_eq!(report.path_results[0].assessment, Assessment::NotEvaluable);
        assert_eq!(report.path_results[9].assessment, Assessment::NotEvaluable);
    }

    #[test]
    fn benchmark_rejects_more_events_than_claims() {
        let options = Options { claims: 2, events: 3, paths: 1, iterations: 2, warmup: 0 };
        assert!(build_fixture(&options).is_ok());
        assert!(options.events > options.claims);
    }
}

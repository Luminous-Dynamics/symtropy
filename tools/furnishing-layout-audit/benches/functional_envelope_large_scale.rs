//! Candidate-only scale probe for the indexed oracle beyond the matched 10k baseline.
//!
//! Run:
//!   cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_large_scale
//!
//! Synthetic sparse grids only. The baseline all-pairs implementation is not run
//! at 25k/50k/100k because the matched 10k baseline already took tens of seconds.
//! This run explores the indexed implementation's next scale boundary, not a speedup.

use furnishing_layout_audit::audit_functional_envelopes;
use serde_json::{Value, json};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn generated_sparse_corpus(item_count: usize) -> Value {
    let side = (item_count as f64).sqrt().ceil() as usize;
    let room_side = (side as f64) * 2.0 + 1.0;
    let mut catalog = Vec::with_capacity(item_count);
    let mut requirements = Vec::with_capacity(item_count);
    let mut placements = Vec::with_capacity(item_count);
    let mut expected_checks = Vec::with_capacity(item_count);

    for index in 0..item_count {
        let item_id = format!("scale-item-{index}");
        let envelope_id = format!("scale-front-envelope-{index}");
        let row = index / side.max(1);
        let column = index % side.max(1);
        catalog.push(json!({
            "item_id": item_id.clone(),
            "kind": "cabinet",
            "width_m": 0.4,
            "depth_m": 0.4,
            "functional_envelopes": [{
                "envelope_id": envelope_id.clone(),
                "task_kind": "service_access",
                "evidence_state": "synthetic_known_truth",
                "rect_local_item_frame": {
                    "x_m": 0.0,
                    "y_m": 0.4,
                    "width_m": 0.4,
                    "depth_m": 0.2
                }
            }]
        }));
        requirements.push(json!({
            "requirement_id": format!("scale-access-{index}"),
            "kind": "functional_envelope_clear",
            "target_envelope_id": envelope_id,
            "must_be_satisfied_to_advance": true
        }));
        placements.push(json!({
            "item_id": item_id,
            "x_m": 0.25 + (column as f64) * 2.0,
            "y_m": 0.25 + (row as f64) * 2.0,
            "rotation_degrees": 0
        }));
        expected_checks.push(json!({
            "requirement_id": format!("scale-access-{index}"),
            "outcome": "pass"
        }));
    }

    json!({
        "schema_version": "0.2.0",
        "benchmark_id": "furnishing-functional-envelope-known-truth-001",
        "status": "synthetic_specification_not_run",
        "units": { "length": "m", "area": "m2" },
        "coordinate_frame": {
            "kind": "room_local_floor_plan",
            "origin": "south_west_floor_corner",
            "x_axis": "east",
            "y_axis": "north",
            "geometry_model": "axis_aligned_2d_rectangles",
            "rotation_degrees_allowed": [0, 90, 180, 270]
        },
        "scenarios": [{
            "scenario_id": format!("LARGE-SPARSE-{item_count}"),
            "title": "Large sparse synthetic scale probe",
            "scope": "synthetic_only",
            "room": { "width_m": room_side, "depth_m": room_side },
            "catalog": catalog,
            "requirements": requirements,
            "alternatives": [{
                "alternative_id": "sparse-grid",
                "placements": placements,
                "expected_checks": expected_checks,
                "promotion_disposition": "candidate_only"
            }]
        }]
    })
}

fn percentile(samples: &[Duration], percent: usize) -> Duration {
    assert!(!samples.is_empty(), "percentile needs at least one sample");
    // Nearest-rank empirical quantile: index = ceil(n * p) - 1.
    let rank = samples.len().saturating_mul(percent).saturating_add(99) / 100;
    samples[rank.saturating_sub(1).min(samples.len() - 1)]
}

fn run_tier(item_count: usize, iterations: usize) {
    let frozen = generated_sparse_corpus(item_count);
    let mut samples = Vec::with_capacity(iterations);
    let mut findings_count = usize::MAX;

    for _ in 0..iterations {
        let started = Instant::now();
        let findings = audit_functional_envelopes(black_box(&frozen));
        let elapsed = started.elapsed();
        assert!(
            findings.is_empty(),
            "large sparse known-truth corpus unexpectedly emitted findings: {findings:#?}"
        );
        findings_count = findings.len();
        samples.push(elapsed);
        black_box(&findings);
    }

    samples.sort_unstable();
    if iterations == 1 {
        println!(
            "scale=sparse_grid items={item_count} iterations={iterations} findings={findings_count} single_sample_ms={:.3}",
            samples[0].as_secs_f64() * 1_000.0
        );
    } else {
        println!(
            "scale=sparse_grid items={item_count} iterations={iterations} findings={findings_count} p50_ms={:.3} p95_ms={:.3}",
            percentile(&samples, 50).as_secs_f64() * 1_000.0,
            percentile(&samples, 95).as_secs_f64() * 1_000.0
        );
    }
}

fn main() {
    // Repeated lower tiers give a noise estimate. Larger tiers use a single sample
    // to avoid turning this diagnostic scale probe into a long-running CI job.
    for (item_count, iterations) in [(10_000, 3), (25_000, 5), (50_000, 1), (100_000, 1)] {
        run_tier(item_count, iterations);
    }
}

//! Dependency-free scale probe for the functional-envelope oracle.
//!
//! Run from the repository root:
//!   cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_scaling
//!
//! All generated dimensions/coordinates are synthetic. Timed calls include the
//! audit of the frozen in-memory JSON document, but exclude corpus construction.

use furnishing_layout_audit::audit_functional_envelopes;
use serde_json::{Value, json};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn generated_corpus(check_count: usize) -> Value {
    let side = (check_count as f64).sqrt().ceil() as usize;
    let room_side = (side as f64) * 2.0 + 1.0;
    let mut catalog = Vec::with_capacity(check_count);
    let mut requirements = Vec::with_capacity(check_count);
    let mut placements = Vec::with_capacity(check_count);
    let mut expected_checks = Vec::with_capacity(check_count);

    for index in 0..check_count {
        let item_id = format!("item-{index}");
        let envelope_id = format!("front-envelope-{index}");
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
            "requirement_id": format!("access-{index}"),
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
            "requirement_id": format!("access-{index}"),
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
            "scenario_id": format!("SCALE-{check_count}"),
            "title": "Synthetic sparse grid for local oracle scaling",
            "scope": "synthetic_only",
            "room": { "width_m": room_side, "depth_m": room_side },
            "catalog": catalog,
            "requirements": requirements,
            "alternatives": [{
                "alternative_id": "grid-layout",
                "placements": placements,
                "expected_checks": expected_checks,
                "promotion_disposition": "candidate_only"
            }]
        }]
    })
}

fn percentile(samples: &[Duration], percent: usize) -> Duration {
    let index = samples.len().saturating_sub(1) * percent / 100;
    samples[index]
}

fn main() {
    for (checks, iterations) in [(25, 100), (100, 100), (1_000, 30), (10_000, 10)] {
        let frozen = generated_corpus(checks);
        let mut samples = Vec::with_capacity(iterations);
        let mut findings = 0usize;
        for _ in 0..iterations {
            let started = Instant::now();
            let result = audit_functional_envelopes(black_box(&frozen));
            samples.push(started.elapsed());
            findings = result.len();
            black_box(findings);
        }
        samples.sort_unstable();
        println!(
            "tasks={checks} items={checks} iterations={iterations} findings={findings} p50_ms={:.3} p95_ms={:.3}",
            percentile(&samples, 50).as_secs_f64() * 1_000.0,
            percentile(&samples, 95).as_secs_f64() * 1_000.0
        );
    }
}

//! Adversarial workload probe: sparse scenes, tight clearances, and true footprint overlaps.
//!
//! Run:
//!   cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_stress
//!
//! Sparse and tight-clearance workload tiers must produce no audit findings.
//! Dense-overlap workloads deliberately contain intersecting physical rectangles;
//! the expected number of collision findings is computed from grid neighbors and
//! checked exactly. All geometry is synthetic; no accessibility or safety claim.

use furnishing_layout_audit::audit_functional_envelopes;
use serde_json::{Value, json};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

const ITEM_WIDTH: f64 = 0.4;
const ITEM_DEPTH: f64 = 0.4;
const ENVELOPE_DEPTH: f64 = 0.2;
const GEOMETRY_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug)]
enum Workload {
    SparseGrid,
    TightFunctionalClearance,
    DenseFootprintOverlap,
}

impl Workload {
    fn name(self) -> &'static str {
        match self {
            Self::SparseGrid => "sparse_grid",
            Self::TightFunctionalClearance => "tight_functional_clearance",
            Self::DenseFootprintOverlap => "dense_footprint_overlap",
        }
    }

    fn pitch(self) -> f64 {
        match self {
            Self::SparseGrid => 2.0,
            // No footprint overlap, but a following row intersects the front task envelope.
            Self::TightFunctionalClearance => 0.41,
            // Physical footprints overlap by 50 mm in adjacent grid directions.
            Self::DenseFootprintOverlap => 0.35,
        }
    }

    fn expected_envelope_outcome(
        self,
        index: usize,
        side: usize,
        item_count: usize,
    ) -> &'static str {
        let has_next_row_same_column = index.saturating_add(side) < item_count;
        if self.pitch() < ITEM_DEPTH + ENVELOPE_DEPTH - GEOMETRY_EPSILON && has_next_row_same_column
        {
            "fail"
        } else {
            "pass"
        }
    }

    fn promotion_disposition(self, item_count: usize, side: usize) -> &'static str {
        if self.pitch() < ITEM_DEPTH + ENVELOPE_DEPTH - GEOMETRY_EPSILON && item_count > side {
            "blocked"
        } else {
            "candidate_only"
        }
    }
}

fn generated_corpus(item_count: usize, workload: Workload) -> Value {
    let side = (item_count as f64).sqrt().ceil() as usize;
    let pitch = workload.pitch();
    let room_side = (side as f64) * pitch + 1.0;
    let mut catalog = Vec::with_capacity(item_count);
    let mut requirements = Vec::with_capacity(item_count);
    let mut placements = Vec::with_capacity(item_count);
    let mut expected_checks = Vec::with_capacity(item_count);

    for index in 0..item_count {
        let item_id = format!("item-{index}");
        let envelope_id = format!("front-envelope-{index}");
        let row = index / side.max(1);
        let column = index % side.max(1);
        catalog.push(json!({
            "item_id": item_id.clone(),
            "kind": "cabinet",
            "width_m": ITEM_WIDTH,
            "depth_m": ITEM_DEPTH,
            "functional_envelopes": [{
                "envelope_id": envelope_id.clone(),
                "task_kind": "service_access",
                "evidence_state": "synthetic_known_truth",
                "rect_local_item_frame": {
                    "x_m": 0.0,
                    "y_m": ITEM_DEPTH,
                    "width_m": ITEM_WIDTH,
                    "depth_m": ENVELOPE_DEPTH
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
            "x_m": 0.25 + (column as f64) * pitch,
            "y_m": 0.25 + (row as f64) * pitch,
            "rotation_degrees": 0
        }));
        expected_checks.push(json!({
            "requirement_id": format!("access-{index}"),
            "outcome": workload.expected_envelope_outcome(index, side, item_count)
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
            "scenario_id": format!("STRESS-{}-{item_count}", workload.name()),
            "title": format!("Synthetic {} stress scene", workload.name()),
            "scope": "synthetic_only",
            "room": { "width_m": room_side, "depth_m": room_side },
            "catalog": catalog,
            "requirements": requirements,
            "alternatives": [{
                "alternative_id": "generated-grid",
                "placements": placements,
                "expected_checks": expected_checks,
                "promotion_disposition": workload.promotion_disposition(item_count, side)
            }]
        }]
    })
}

fn expected_collision_pairs(item_count: usize, side: usize, pitch: f64) -> usize {
    let mut pairs = 0usize;
    for index in 0..item_count {
        let row = index / side.max(1);
        let column = index % side.max(1);
        for delta_row in -1isize..=1 {
            for delta_column in -1isize..=1 {
                if delta_row == 0 && delta_column == 0 {
                    continue;
                }
                let next_row = row as isize + delta_row;
                let next_column = column as isize + delta_column;
                if next_row < 0 || next_column < 0 {
                    continue;
                }
                let next_row = next_row as usize;
                let next_column = next_column as usize;
                if next_row >= side || next_column >= side {
                    continue;
                }
                let next_index = next_row * side + next_column;
                if next_index <= index || next_index >= item_count {
                    continue;
                }
                let overlaps_x =
                    (delta_column.unsigned_abs() as f64) * pitch < ITEM_WIDTH - GEOMETRY_EPSILON;
                let overlaps_y =
                    (delta_row.unsigned_abs() as f64) * pitch < ITEM_DEPTH - GEOMETRY_EPSILON;
                if overlaps_x && overlaps_y {
                    pairs += 1;
                }
            }
        }
    }
    pairs
}

fn percentile(samples: &[Duration], percent: usize) -> Duration {
    let index = samples.len().saturating_sub(1) * percent / 100;
    samples[index]
}

fn run_workload(item_count: usize, iterations: usize, workload: Workload) {
    let frozen = generated_corpus(item_count, workload);
    let side = (item_count as f64).sqrt().ceil() as usize;
    let expected_collisions = expected_collision_pairs(item_count, side, workload.pitch());
    let mut samples = Vec::with_capacity(iterations);
    let mut actual_collisions = 0usize;
    let mut unexpected_findings = 0usize;

    for _ in 0..iterations {
        let started = Instant::now();
        let findings = audit_functional_envelopes(black_box(&frozen));
        samples.push(started.elapsed());
        actual_collisions = findings
            .iter()
            .filter(|finding| {
                finding.contains("physical footprints") && finding.contains("overlap")
            })
            .count();
        unexpected_findings = findings.len().saturating_sub(actual_collisions);
        assert_eq!(
            actual_collisions,
            expected_collisions,
            "{} at {item_count} items: expected physical overlap findings",
            workload.name()
        );
        assert_eq!(
            unexpected_findings,
            0,
            "{} at {item_count} items: unexpected oracle findings: {findings:#?}",
            workload.name()
        );
        black_box(&findings);
    }

    samples.sort_unstable();
    println!(
        "workload={} items={} iterations={} expected_collisions={} observed_collisions={} unexpected_findings={} p50_ms={:.3} p95_ms={:.3}",
        workload.name(),
        item_count,
        iterations,
        expected_collisions,
        actual_collisions,
        unexpected_findings,
        percentile(&samples, 50).as_secs_f64() * 1_000.0,
        percentile(&samples, 95).as_secs_f64() * 1_000.0,
    );
}

fn main() {
    let workloads = [
        Workload::SparseGrid,
        Workload::TightFunctionalClearance,
        Workload::DenseFootprintOverlap,
    ];
    for workload in workloads {
        for (item_count, iterations) in [(25, 40), (100, 30), (1_000, 15), (10_000, 5)] {
            run_workload(item_count, iterations, workload);
        }
    }
}

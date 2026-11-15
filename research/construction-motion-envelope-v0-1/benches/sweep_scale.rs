use construction_motion_envelope::{
    AxisMm, Context, EvidenceRefs, MarginsMm, Point3Mm, Segment, build,
};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn context() -> Context {
    Context {
        project_id: "p".into(),
        work_package_id: "wp".into(),
        task_id: "t".into(),
        trajectory_id: "traj".into(),
        robot_id: "r".into(),
        platform_config_sha256: "a".repeat(64),
        tool_config_sha256: "b".repeat(64),
        payload_config_sha256: "c".repeat(64),
        collision_model_sha256: "d".repeat(64),
        primitive_set_ref: "benchmark-primitive-manifest".into(),
        expected_primitive_ids: vec!["base".into(), "tool".into(), "payload".into()],
        calibration_ref: "cal".into(),
        controller_profile_ref: "controller".into(),
        frame_id: "frame".into(),
        transform_sha256: "e".repeat(64),
        design_revision_sha256: "f".repeat(64),
        site_epoch: "epoch".into(),
        clock_domain_id: "clock".into(),
        trajectory_profile_sha256: "1".repeat(64),
    }
}
fn evidence() -> EvidenceRefs {
    EvidenceRefs {
        path_deviation: "p".into(),
        localization: "l".into(),
        tracking: "t".into(),
        model_uncertainty: "m".into(),
        numerical_enclosure: "n".into(),
        stopping_reachability: "s".into(),
        policy_clearance: "c".into(),
    }
}
fn input(n: usize) -> Vec<Segment> {
    let mut v = Vec::with_capacity(n * 3);
    for i in 0..n {
        let x = i as i64 * 5;
        for (id, y, r) in [
            ("base", 0_i64, 300_u64),
            ("tool", 500, 150),
            ("payload", 800, 200),
        ] {
            v.push(Segment {
                slice_index: i as u32,
                primitive_id: id.into(),
                start_ms: i as u64 * 10,
                end_ms: i as u64 * 10 + 10,
                from_mm: Point3Mm { x, y, z: 0 },
                to_mm: Point3Mm { x: x + 5, y, z: 0 },
                radius_mm: r,
                margins: MarginsMm {
                    path_deviation: AxisMm { x: 2, y: 2, z: 2 },
                    localization: AxisMm { x: 3, y: 3, z: 3 },
                    tracking: AxisMm { x: 3, y: 3, z: 3 },
                    model_uncertainty: AxisMm { x: 2, y: 2, z: 2 },
                    numerical_enclosure: AxisMm { x: 1, y: 1, z: 1 },
                    stopping_reachability: AxisMm { x: 20, y: 10, z: 5 },
                    policy_clearance: AxisMm {
                        x: 50,
                        y: 50,
                        z: 50,
                    },
                },
                evidence: evidence(),
            });
        }
    }
    v
}
fn pct(v: &[Duration], n: usize) -> Duration {
    v[((v.len() - 1) * n).div_ceil(100)]
}
fn main() {
    for count in [4_usize, 334, 3_334, 33_334] {
        let c = context();
        let segs = input(count);
        let mut times = Vec::with_capacity(25);
        for _ in 0..25 {
            let t = Instant::now();
            let r = build(black_box(&c), black_box(&segs)).unwrap();
            black_box(r.canonical_bytes());
            times.push(t.elapsed());
        }
        times.sort_unstable();
        println!(
            "cells={} p50_us={:.2} p95_us={:.2} p99_us={:.2}",
            count * 3,
            pct(&times, 50).as_secs_f64() * 1e6,
            pct(&times, 95).as_secs_f64() * 1e6,
            pct(&times, 99).as_secs_f64() * 1e6
        );
    }
}

//! Dependency-light host performance probe for the deterministic software renderer.
//!
//! Usage:
//!   cargo run --release --example renderer_benchmark -- 1000
//!
//! Receipt clock values are deterministic synthetic simulator-clock values.
//! Host render durations are measured separately with std::time::Instant and
//! must not be substituted for the receipt's externally supplied event times.

use std::error::Error;
use std::time::Instant;

use action_conditioned_offscreen_camera::{
    ActorClass, ActorCuboid, AppliedActionRef, CameraCalibration, Q16_ONE, RenderClocks,
    RoadMarkingKind, RoadPolyline, SoftwareCameraRenderer, WorldRenderSnapshot,
    RENDERER_REVISION,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const WARMUP_FRAMES: u64 = 20;
const MAX_ITERATIONS: usize = 100_000;

fn camera() -> CameraCalibration {
    CameraCalibration {
        camera_id: "front-rgb".to_string(),
        position_world_mm: [0, 0, 1_500],
        world_to_camera_q16: [
            [Q16_ONE as i32, 0, 0],
            [0, 0, -(Q16_ONE as i32)],
            [0, Q16_ONE as i32, 0],
        ],
        focal_x_millipx: 450_000,
        focal_y_millipx: 450_000,
        principal_x_millipx: WIDTH * 500,
        principal_y_millipx: HEIGHT * 500,
        width: WIDTH,
        height: HEIGHT,
        near_plane_mm: 100,
        far_plane_mm: 200_000,
    }
}

fn road_marking(marking_id: &str, lane_offset_mm: i64, kind: RoadMarkingKind) -> RoadPolyline {
    let points_world_mm = (0_i64..48)
        .map(|index| {
            let forward_mm = 4_000 + index * 2_500;
            let curve_mm = index * index * 12;
            [lane_offset_mm + curve_mm, forward_mm, 0]
        })
        .collect();

    RoadPolyline {
        road_id: "curved-road-0".to_string(),
        marking_id: marking_id.to_string(),
        kind,
        points_world_mm,
    }
}

fn snapshot(tick: u64) -> WorldRenderSnapshot {
    let actors = vec![
        ActorCuboid {
            actor_id: "lead-car".to_string(),
            actor_policy_revision: "idm-actor-v1".to_string(),
            class: ActorClass::Car,
            center_world_mm: [1_200, 24_000, 700],
            half_extents_mm: [900, 2_000, 700],
        },
        ActorCuboid {
            actor_id: "adjacent-bus".to_string(),
            actor_policy_revision: "bus-actor-v1".to_string(),
            class: ActorClass::Bus,
            center_world_mm: [5_200, 39_000, 1_400],
            half_extents_mm: [1_250, 5_500, 1_400],
        },
        ActorCuboid {
            actor_id: "pedestrian".to_string(),
            actor_policy_revision: "pedestrian-actor-v1".to_string(),
            class: ActorClass::Pedestrian,
            center_world_mm: [-1_400, 17_000, 900],
            half_extents_mm: [300, 300, 900],
        },
    ];

    WorldRenderSnapshot {
        episode_id: "renderer-benchmark-episode".to_string(),
        scenario_id: "curved-road-three-actors".to_string(),
        agent_id: "ego".to_string(),
        tick,
        world_timestamp_us: 1_000_000 + tick * 33_333,
        clock_id: "benchmark-sim-clock".to_string(),
        simulator_revision: "synthetic-snapshot-v1".to_string(),
        applied_action: Some(AppliedActionRef {
            receipt_id: format!("action-receipt-{tick}"),
            receipt_sha256: "a".repeat(64),
            command_id: format!("control-command-{tick}"),
            applied_tick: tick,
        }),
        camera: camera(),
        road_markings: vec![
            road_marking("left-lane-edge", -3_500, RoadMarkingKind::LaneWhite),
            road_marking("center-divider", 0, RoadMarkingKind::LaneYellow),
            road_marking("right-lane-edge", 3_500, RoadMarkingKind::LaneWhite),
        ],
        actors,
    }
}

fn render_sample(tick: u64) -> Result<(u64, String), Box<dyn Error>> {
    let world = snapshot(tick);
    let capture_timestamp_us = world.world_timestamp_us;
    let clocks = RenderClocks {
        capture_timestamp_us,
        render_completed_timestamp_us: capture_timestamp_us + 500,
        delivered_timestamp_us: capture_timestamp_us + 800,
    };

    let started = Instant::now();
    let output = SoftwareCameraRenderer::render(&world, tick, clocks)?;
    let elapsed_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    Ok((elapsed_us, output.policy_receipt.frame_sha256))
}

fn percentile(sorted: &[u64], percentile: usize) -> u64 {
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted[rank.min(sorted.len()) - 1]
}

fn main() -> Result<(), Box<dyn Error>> {
    let iterations = std::env::args()
        .nth(1)
        .map(|argument| argument.parse::<usize>())
        .transpose()?
        .unwrap_or(1_000);
    if iterations == 0 || iterations > MAX_ITERATIONS {
        return Err(format!("iterations must be in 1..={MAX_ITERATIONS}").into());
    }

    for tick in 1..=WARMUP_FRAMES {
        let _ = render_sample(tick)?;
    }

    let mut samples_us = Vec::with_capacity(iterations);
    let mut final_frame_sha256 = String::new();
    for index in 0..iterations {
        let tick = WARMUP_FRAMES + index as u64 + 1;
        let (elapsed_us, frame_sha256) = render_sample(tick)?;
        samples_us.push(elapsed_us);
        final_frame_sha256 = frame_sha256;
    }

    samples_us.sort_unstable();
    let total_us: u128 = samples_us.iter().map(|value| u128::from(*value)).sum();
    let mean_us = total_us as f64 / samples_us.len() as f64;
    let frames_per_second = if mean_us > 0.0 {
        1_000_000.0 / mean_us
    } else {
        f64::INFINITY
    };
    let frame_bytes = usize::try_from(WIDTH).unwrap()
        * usize::try_from(HEIGHT).unwrap()
        * 3;

    println!(
        concat!(
            "{{\n",
            "  \"renderer_revision\": \"{}\",\n",
            "  \"host_os\": \"{}\",\n",
            "  \"host_arch\": \"{}\",\n",
            "  \"resolution\": \"{}x{}\",\n",
            "  \"pixel_format\": \"rgb8_srgb\",\n",
            "  \"frame_bytes\": {},\n",
            "  \"warmup_frames\": {},\n",
            "  \"measured_frames\": {},\n",
            "  \"host_render_p50_us\": {},\n",
            "  \"host_render_p95_us\": {},\n",
            "  \"host_render_p99_us\": {},\n",
            "  \"host_render_mean_us\": {:.2},\n",
            "  \"host_frames_per_second\": {:.2},\n",
            "  \"last_frame_sha256\": \"{}\"\n",
            "}}"
        ),
        RENDERER_REVISION,
        std::env::consts::OS,
        std::env::consts::ARCH,
        WIDTH,
        HEIGHT,
        frame_bytes,
        WARMUP_FRAMES,
        iterations,
        percentile(&samples_us, 50),
        percentile(&samples_us, 95),
        percentile(&samples_us, 99),
        mean_us,
        frames_per_second,
        final_frame_sha256
    );
    Ok(())
}

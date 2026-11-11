//! End-to-end renderer -> Symthaea bridge episode fixture.
//!
//! This is a deterministic integration fixture, not a traffic simulator or driver.
//! It consumes caller-supplied BridgeConfig and VisionConfig JSON, creates immutable
//! snapshots and RGB frames, invokes the actual Symthaea bridge executable, and
//! verifies the complete receipt/artifact chain.

use action_conditioned_offscreen_camera::{
    sha256_hex, ActorClass, ActorCuboid, AppliedActionRef, CameraCalibration,
    MissingFrameReason, MissingObservationEvent, ObservationDisposition,
    ObservationStreamGuard, RenderClocks,
    RoadMarkingKind, RoadPolyline, SoftwareCameraRenderer, TimingPolicy, WorldRenderSnapshot,
    Q16_ONE, RENDERER_REVISION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EPISODE_ID: &str = "fresh-process-driving-episode-v0-1";
const SCENARIO_ID: &str = "straight-to-curved-road-two-actors-v0-1";
const AGENT_ID: &str = "ego";
const CAMERA_ID: &str = "front-rgb";
const CLOCK_ID: &str = "sim-monotonic-driving-fixture-01";
const SIMULATOR_REVISION: &str = "pre-recorded-kinematic-snapshot-fixture-v1";
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const CHANNELS: usize = 3;
const EXPECTED_TICKS: u64 = 20;
const MISSING_TICK: u64 = 11;
const DELAYED_TICK: u64 = 7;
const BASE_TIMESTAMP_US: u64 = 1_000_000;
const TICK_PERIOD_US: u64 = 33_333;
const MAX_RENDER_LATENCY_US: u64 = 2_000;
const MAX_DELIVERY_AGE_US: u64 = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplayInput {
    records: Vec<ReplaySnapshotRecord>,
    missing_events: Vec<MissingObservationEvent>,
    dispositions: Vec<DispositionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplaySnapshotRecord {
    sequence: u64,
    snapshot: WorldRenderSnapshot,
    clocks: RenderClocks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReplayOutput {
    rendered_frames: Vec<ReplaySummary>,
    missing_events_sha256: String,
    dispositions_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReplaySummary {
    sequence: u64,
    tick: u64,
    frame_byte_len: usize,
    frame_sha256: String,
    observation_id: String,
    policy_receipt_sha256: String,
    world_snapshot_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DispositionRecord {
    sequence: u64,
    tick: u64,
    disposition: ObservationDisposition,
    capture_timestamp_us: u64,
    render_completed_timestamp_us: u64,
    delivered_timestamp_us: u64,
}

#[derive(Debug, Clone, Serialize)]
struct BridgeNegativeControlRecord {
    control_id: String,
    expected_error: String,
    exit_code: Option<i32>,
    receipt_file_written: bool,
    policy_input_artifact_count: usize,
    expected_error_observed: bool,
    fail_closed: bool,
    stderr_excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
struct AppliedActionRecord {
    action_receipt_id: String,
    action_receipt_sha256: String,
    command_id: String,
    applied_tick: u64,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
struct TransitionRecord {
    tick: u64,
    applied_action_receipt_id: String,
    world_snapshot_sha256: String,
    simulator_revision: String,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct CounterfactualBranchRecord {
    branch_id: String,
    parent_decision_tick: u64,
    changed_action_applied_tick: u64,
    changed_action_receipt_id: String,
    changed_action_receipt_sha256: String,
    base_frame_sha256: String,
    branch_frame_sha256: String,
    base_world_snapshot_sha256: String,
    branch_world_snapshot_sha256: String,
    actor_response_mode: String,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct EpisodeReport {
    schema_version: u32,
    status: String,
    episode_id: String,
    scenario_id: String,
    renderer_revision: String,
    simulator_backend: String,
    expected_ticks: u64,
    applied_actions_recorded: usize,
    world_snapshot_transition_records: usize,
    counterfactual_branches: usize,
    delivered_observations: usize,
    missing_observations: usize,
    delayed_observations: usize,
    fresh_observations: usize,
    fresh_process_renderer_replay_exact: bool,
    fresh_process_symthaea_bridge_replay_exact: bool,
    bridge_negative_control_count: usize,
    bridge_negative_controls_fail_closed: bool,
    branch_evidence_audit_valid: bool,
    stale_actor_replay_negative_control_rejected: bool,
    branch_evidence_finding_count: usize,
    degradation_recovery_audit_valid: bool,
    missing_frame_acceptance_negative_control_rejected: bool,
    degradation_recovery_event_count: usize,
    degradation_recovery_finding_count: usize,
    symthaea_bridge_invoked: bool,
    symthaea_bridge_processed_observations: usize,
    symthaea_receipt_chain_valid: bool,
    policy_input_artifacts_valid: bool,
    policy_input_artifact_count: usize,
    bridge_vision_learning_frozen: bool,
    bridge_training_steps_before: u64,
    bridge_training_steps_after: u64,
    counterfactual_branch_frame_changed: bool,
    claim_ceiling: String,
}

fn world_timestamp(tick: u64) -> u64 {
    BASE_TIMESTAMP_US + tick.saturating_sub(1) * TICK_PERIOD_US
}

fn camera() -> CameraCalibration {
    CameraCalibration {
        camera_id: CAMERA_ID.to_string(),
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

fn road_marking(marking_id: &str, offset_mm: i64, kind: RoadMarkingKind) -> RoadPolyline {
    let points_world_mm = (0_i64..48)
        .map(|index| {
            let forward_mm = 4_000 + index * 2_500;
            let curve_mm = index * index * 12;
            [offset_mm + curve_mm, forward_mm, 0]
        })
        .collect();

    RoadPolyline {
        road_id: "road-main".to_string(),
        marking_id: marking_id.to_string(),
        kind,
        points_world_mm,
    }
}

fn action_ref(tick: u64, branch: bool) -> AppliedActionRef {
    let action_kind = if branch { "counterfactual-brake-and-yield" } else { "recorded-cruise-control" };
    let command_id = format!("{action_kind}-{tick}");
    let receipt_id = format!("action-receipt-{action_kind}-{tick}");
    let receipt_material = format!(
        "{EPISODE_ID}|{tick}|{command_id}|declared-fixture-command-v1"
    );
    AppliedActionRef {
        receipt_sha256: sha256_hex(receipt_material.as_bytes()),
        receipt_id,
        command_id,
        applied_tick: tick,
    }
}

fn snapshot(tick: u64, branch: bool) -> WorldRenderSnapshot {
    let time = world_timestamp(tick);
    let branch_shift = if branch { 850 } else { 0 };
    let branch_longitudinal = if branch { -400 } else { 0 };

    let actors = vec![
        ActorCuboid {
            actor_id: "lead-car".to_string(),
            actor_policy_revision: "declared-lead-actor-fixture-v1".to_string(),
            class: ActorClass::Car,
            center_world_mm: [
                350 + branch_shift,
                18_000 + tick as i64 * 320 + branch_longitudinal,
                700,
            ],
            half_extents_mm: [900, 2_000, 700],
        },
        ActorCuboid {
            actor_id: "crossing-pedestrian".to_string(),
            actor_policy_revision: "declared-pedestrian-fixture-v1".to_string(),
            class: ActorClass::Pedestrian,
            center_world_mm: [
                -2_000 + tick as i64 * 85 - branch_shift / 2,
                25_000 + branch_longitudinal,
                900,
            ],
            half_extents_mm: [300, 300, 900],
        },
    ];

    WorldRenderSnapshot {
        episode_id: EPISODE_ID.to_string(),
        scenario_id: SCENARIO_ID.to_string(),
        agent_id: AGENT_ID.to_string(),
        tick,
        world_timestamp_us: time,
        clock_id: CLOCK_ID.to_string(),
        simulator_revision: SIMULATOR_REVISION.to_string(),
        applied_action: Some(action_ref(tick, branch)),
        camera: camera(),
        road_markings: vec![
            road_marking("left-road-edge", -5_250, RoadMarkingKind::RoadEdge),
            road_marking("left-lane-divider", -1_750, RoadMarkingKind::LaneWhite),
            road_marking("center-line", 0, RoadMarkingKind::LaneYellow),
            road_marking("right-lane-divider", 1_750, RoadMarkingKind::LaneWhite),
            road_marking("right-road-edge", 5_250, RoadMarkingKind::RoadEdge),
        ],
        actors,
    }
}

fn clocks(tick: u64) -> RenderClocks {
    let capture = world_timestamp(tick);
    let (render_latency, delivery_age) = if tick == DELAYED_TICK {
        (2_500, 3_500)
    } else {
        (800, 1_000)
    };
    RenderClocks {
        capture_timestamp_us: capture,
        render_completed_timestamp_us: capture + render_latency,
        delivered_timestamp_us: capture + delivery_age,
    }
}

fn vehicle_state(tick: u64) -> Value {
    let timestamp_s = world_timestamp(tick) as f64 / 1_000_000.0;
    let speed = 9.0 + tick as f64 * 0.035;
    let steering = if tick < 12 { 0.012 } else { 0.028 };
    json!({
        "speed": speed,
        "lateral_velocity": 0.04,
        "yaw_rate": 0.006 + tick as f64 * 0.0002,
        "heading": tick as f64 * 0.0004,
        "position_x": tick as f64 * 0.18,
        "position_y": tick as f64 * 0.025,
        "steering_angle": steering,
        "throttle_position": 0.21,
        "brake_pressure": 0.0,
        "longitudinal_accel": 0.035,
        "lateral_accel": 0.12,
        "tire_slip_front": 0.004,
        "tire_slip_rear": 0.003,
        "pitch": -0.001,
        "lateral_offset": tick as f64 * 0.025,
        "curvature_ahead": 0.0009,
        "nearest_peer_distance": 18.0,
        "peer_relative_speed": -0.6,
        "mesh_brake_density": 0.0,
        "mesh_confidence": 0.85,
        "timestamp": timestamp_s
    })
}

fn render_summary(record: &ReplaySnapshotRecord) -> Result<ReplaySummary, Box<dyn Error>> {
    let rendered = SoftwareCameraRenderer::render(
        &record.snapshot,
        record.sequence,
        record.clocks,
    )?;
    let audit = SoftwareCameraRenderer::audit_snapshot(&record.snapshot)?;
    Ok(ReplaySummary {
        sequence: record.sequence,
        tick: record.snapshot.tick,
        frame_byte_len: rendered.pixels_rgb.len(),
        frame_sha256: rendered.policy_receipt.frame_sha256,
        observation_id: rendered.policy_receipt.observation_id,
        policy_receipt_sha256: rendered.policy_receipt.receipt_sha256,
        world_snapshot_sha256: audit.world_snapshot_sha256,
    })
}

fn replay_records(records: &[ReplaySnapshotRecord]) -> Result<Vec<ReplaySummary>, Box<dyn Error>> {
    records.iter().map(render_summary).collect()
}

fn validate_stream_ledger(input: &ReplayInput) -> Result<(), Box<dyn Error>> {
    let mut seen = std::collections::BTreeSet::new();
    for record in &input.records {
        if record.sequence != record.snapshot.tick || !seen.insert(record.sequence) {
            return Err("rendered observation sequence/tick identity is invalid or duplicated".into());
        }
    }
    for missing in &input.missing_events {
        if missing.sequence != missing.tick || !seen.insert(missing.sequence) {
            return Err("missing observation sequence/tick identity is invalid or duplicated".into());
        }
    }
    if seen.len() != EXPECTED_TICKS as usize
        || seen.iter().copied().ne(1..=EXPECTED_TICKS)
        || input.dispositions.len() != input.records.len()
    {
        return Err("expected/observed stream denominator is incomplete".into());
    }
    let disposition_sequences = input
        .dispositions
        .iter()
        .map(|record| record.sequence)
        .collect::<std::collections::BTreeSet<_>>();
    let rendered_sequences = input.records.iter().map(|record| record.sequence).collect::<std::collections::BTreeSet<_>>();
    if disposition_sequences != rendered_sequences
        || input.missing_events.iter().any(|event| disposition_sequences.contains(&event.sequence))
    {
        return Err("missing and delivered observations overlap or lack dispositions".into());
    }
    if input.missing_events.len() != 1
        || input.missing_events[0].sequence != MISSING_TICK
        || input.dispositions.iter().filter(|item| item.disposition == ObservationDisposition::Delayed).count() != 1
        || !input.dispositions.iter().any(|item| item.sequence == DELAYED_TICK && item.disposition == ObservationDisposition::Delayed)
    {
        return Err("fixture missing/delayed event contract is not satisfied".into());
    }
    Ok(())
}

fn replay_output(input: &ReplayInput) -> Result<ReplayOutput, Box<dyn Error>> {
    validate_stream_ledger(input)?;
    Ok(ReplayOutput {
        rendered_frames: replay_records(&input.records)?,
        missing_events_sha256: sha256_hex(&serde_json::to_vec(&input.missing_events)?),
        dispositions_sha256: sha256_hex(&serde_json::to_vec(&input.dispositions)?),
    })
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn validate_bridge_configuration(
    bridge_config: &Value,
    vision_config: &Value,
) -> Result<(), Box<dyn Error>> {
    let identity = bridge_config
        .get("identity")
        .ok_or("bridge_config.identity is required")?;
    for (field, expected) in [
        ("episode_id", EPISODE_ID),
        ("scenario_id", SCENARIO_ID),
        ("agent_id", AGENT_ID),
        ("camera_id", CAMERA_ID),
        ("simulator_revision", SIMULATOR_REVISION),
    ] {
        if identity.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!(
                "bridge_config.identity.{field} must equal {expected:?}"
            )
            .into());
        }
    }
    let max_width = bridge_config["max_width"].as_u64().unwrap_or(0);
    let max_height = bridge_config["max_height"].as_u64().unwrap_or(0);
    let max_bytes = bridge_config["max_frame_bytes"].as_u64().unwrap_or(0);
    let initial_dt = bridge_config["initial_dt_us"].as_u64().unwrap_or(0);
    let max_gap = bridge_config["maximum_frame_gap_us"].as_u64().unwrap_or(0);
    if max_width < u64::from(WIDTH)
        || max_height < u64::from(HEIGHT)
        || max_bytes < u64::from(WIDTH) * u64::from(HEIGHT) * CHANNELS as u64
        || initial_dt != TICK_PERIOD_US
        || max_gap < TICK_PERIOD_US * 2
    {
        return Err("bridge_config frame/time bounds do not cover this 640x360, one-dropped-frame fixture".into());
    }
    if vision_config["hdc_dim"].as_u64() != Some(16_384)
        || vision_config["enable_depth"].as_bool() != Some(false)
    {
        return Err("vision_config must pin hdc_dim=16384 and enable_depth=false".into());
    }
    Ok(())
}

fn save_render_artifacts(
    episode_dir: &Path,
    records: &[ReplaySnapshotRecord],
    missing_events: &[MissingObservationEvent],
    dispositions: &[DispositionRecord],
) -> Result<Vec<Value>, Box<dyn Error>> {
    let frames_dir = episode_dir.join("frames");
    let policy_dir = episode_dir.join("policy-receipts");
    let audit_dir = episode_dir.join("evaluator-audits");
    fs::create_dir_all(&frames_dir)?;
    fs::create_dir_all(&policy_dir)?;
    fs::create_dir_all(&audit_dir)?;

    let mut bridge_observations = Vec::with_capacity(records.len());
    for record in records {
        let rendered = SoftwareCameraRenderer::render(
            &record.snapshot,
            record.sequence,
            record.clocks,
        )?;
        let audit = SoftwareCameraRenderer::audit_snapshot(&record.snapshot)?;
        let frame_name = format!("frame_{:04}.rgb", record.sequence);
        let frame_relative = format!("frames/{frame_name}");
        fs::write(episode_dir.join(&frame_relative), &rendered.pixels_rgb)?;

        write_json(
            &policy_dir.join(format!("policy_receipt_{:04}.json", record.sequence)),
            &rendered.policy_receipt,
        )?;
        write_json(
            &audit_dir.join(format!("evaluator_audit_{:04}.json", record.sequence)),
            &audit,
        )?;

        bridge_observations.push(json!({
            "sequence": record.sequence,
            "timestamp_us": record.clocks.capture_timestamp_us,
            "vehicle_state_timestamp_us": record.clocks.capture_timestamp_us,
            "pixels_file": frame_relative,
            "vehicle_state": vehicle_state(record.snapshot.tick),
            "upstream": rendered.policy_receipt
        }));
    }

    write_json(
        &episode_dir.join("missing-observation-events.json"),
        &missing_events,
    )?;
    write_json(
        &episode_dir.join("observation-dispositions.json"),
        &dispositions,
    )?;
    Ok(bridge_observations)
}

fn make_changed_action_branch(
    episode_dir: &Path,
) -> Result<CounterfactualBranchRecord, Box<dyn Error>> {
    let base_snapshot = snapshot(12, false);
    let branch_snapshot = snapshot(12, true);
    let base_clocks = clocks(12);
    let base_render = SoftwareCameraRenderer::render(&base_snapshot, 12, base_clocks)?;
    let branch_render = SoftwareCameraRenderer::render(&branch_snapshot, 12, base_clocks)?;
    let base_audit = SoftwareCameraRenderer::audit_snapshot(&base_snapshot)?;
    let branch_audit = SoftwareCameraRenderer::audit_snapshot(&branch_snapshot)?;

    let branch_dir = episode_dir.join("counterfactual-branch");
    fs::create_dir_all(&branch_dir)?;
    fs::write(
        branch_dir.join("branch_frame_0012.rgb"),
        &branch_render.pixels_rgb,
    )?;
    write_json(
        &branch_dir.join("branch_policy_receipt_0012.json"),
        &branch_render.policy_receipt,
    )?;
    write_json(
        &branch_dir.join("branch_evaluator_audit_0012.json"),
        &branch_audit,
    )?;

    let action = branch_snapshot
        .applied_action
        .as_ref()
        .ok_or("counterfactual branch lacks its applied-action receipt")?;
    Ok(CounterfactualBranchRecord {
        branch_id: "alternate-action-at-tick-12-v0-1".to_string(),
        parent_decision_tick: 11,
        changed_action_applied_tick: 12,
        changed_action_receipt_id: action.receipt_id.clone(),
        changed_action_receipt_sha256: action.receipt_sha256.clone(),
        base_frame_sha256: base_render.policy_receipt.frame_sha256,
        branch_frame_sha256: branch_render.policy_receipt.frame_sha256,
        base_world_snapshot_sha256: base_audit.world_snapshot_sha256,
        branch_world_snapshot_sha256: branch_audit.world_snapshot_sha256,
        actor_response_mode: "synthetic_recomputed_response_fixture".to_string(),
        note: "Actor poses are explicitly changed in the alternate immutable snapshot. This validates receipt/branch separation only; no actor-policy simulator is executed.".to_string(),
    })
}

fn invoke_bridge_process(
    bridge_binary: &Path,
    request_path: &Path,
    receipts_path: &Path,
    policy_inputs_dir: &Path,
) -> Result<Value, Box<dyn Error>> {
    fs::create_dir_all(
        receipts_path
            .parent()
            .ok_or("bridge receipts path must have a parent directory")?,
    )?;
    fs::create_dir_all(policy_inputs_dir)?;
    let status = Command::new(bridge_binary)
        .arg(request_path)
        .arg(receipts_path)
        .arg(policy_inputs_dir)
        .status()?;
    if !status.success() {
        return Err(format!("Symthaea camera-driving-bridge exited with status {status}").into());
    }
    Ok(read_json(receipts_path)?)
}

fn validate_bridge_output(
    bridge_output: &Value,
    observations: &[Value],
    policy_inputs_dir: &Path,
) -> Result<(bool, bool), Box<dyn Error>> {
    let receipts = bridge_output["receipts"]
        .as_array()
        .ok_or("Symthaea bridge output is missing receipts[]")?;
    let artifacts = bridge_output["policy_input_artifacts"]
        .as_array()
        .ok_or("Symthaea bridge output is missing policy_input_artifacts[]")?;
    let expected_count = observations.len();
    if bridge_output["expected_observations"].as_u64() != Some(expected_count as u64)
        || bridge_output["processed_observations"].as_u64() != Some(expected_count as u64)
        || bridge_output["frames_processed"].as_u64() != Some(expected_count as u64)
        || receipts.len() != expected_count
        || artifacts.len() != expected_count
    {
        return Err("Symthaea bridge output denominator/count mismatch".into());
    }
    if bridge_output["vision_learning_frozen"].as_bool() != Some(true)
        || bridge_output["vision_training_steps_before"]
            != bridge_output["vision_training_steps_after"]
    {
        return Err("Symthaea bridge did not preserve frozen-learning evidence".into());
    }

    let mut receipt_chain_valid = true;
    let mut artifact_chain_valid = true;
    for (index, (bridge_receipt, artifact)) in receipts.iter().zip(artifacts).enumerate() {
        let observation = &observations[index];
        let upstream = &observation["upstream"];

        for (field, expected) in [
            ("upstream_observation_id", upstream["observation_id"].as_str().unwrap_or("")),
            ("upstream_receipt_sha256", upstream["receipt_sha256"].as_str().unwrap_or("")),
            ("upstream_frame_sha256", upstream["frame_sha256"].as_str().unwrap_or("")),
        ] {
            if bridge_receipt[field].as_str() != Some(expected) {
                receipt_chain_valid = false;
            }
        }
        if bridge_receipt["capture_sequence"] != observation["sequence"]
            || bridge_receipt["capture_timestamp_us"] != observation["timestamp_us"]
        {
            receipt_chain_valid = false;
        }
        if index == 0 {
            if !bridge_receipt["previous_receipt_sha256"].is_null() {
                receipt_chain_valid = false;
            }
        } else if bridge_receipt["previous_receipt_sha256"]
            != receipts[index - 1]["receipt_sha256"]
        {
            receipt_chain_valid = false;
        }

        let fused_digest = bridge_receipt["fused_policy_input_sha256"]
            .as_str()
            .unwrap_or("");
        if artifact["sha256"].as_str() != Some(fused_digest)
            || artifact["dimension"].as_u64() != Some(16_384)
            || artifact["sequence"] != observation["sequence"]
        {
            artifact_chain_valid = false;
        }
        let relative_name = artifact["relative_path"].as_str().unwrap_or("");
        if relative_name.is_empty()
            || Path::new(relative_name).components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            artifact_chain_valid = false;
            continue;
        }
        let bytes = fs::read(policy_inputs_dir.join(relative_name))?;
        if bytes.len() != 8 + 16_384 * 4
            || sha256_hex(&bytes) != fused_digest
            || artifact["byte_length"].as_u64() != Some(bytes.len() as u64)
        {
            artifact_chain_valid = false;
        }
    }
    if !receipt_chain_valid || !artifact_chain_valid {
        return Err("Symthaea bridge receipt or HDC artifact chain failed reconciliation".into());
    }
    Ok((receipt_chain_valid, artifact_chain_valid))
}

fn count_regular_files(path: &Path) -> Result<usize, Box<dyn Error>> {
    if !path.exists() {
        return Ok(0);
    }
    let mut count = 0_usize;
    for entry in fs::read_dir(path)? {
        if entry?.file_type()?.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

fn run_bridge_negative_controls(
    episode_dir: &Path,
    bridge_binary: &Path,
    base_request: &Value,
) -> Result<(usize, bool), Box<dyn Error>> {
    let negative_root = episode_dir.join("bridge-negative-controls");
    fs::create_dir_all(&negative_root)?;

    let mut corrupt_bytes = fs::read(episode_dir.join("frames/frame_0001.rgb"))?;
    if corrupt_bytes.is_empty() {
        return Err("cannot construct pixel-corruption negative control from an empty frame".into());
    }
    corrupt_bytes[0] ^= 0x01;
    fs::write(negative_root.join("pixel-corrupt.rgb"), &corrupt_bytes)?;

    let mut controls: Vec<(String, String, Value)> = Vec::new();

    // Reuse tick 1's actual pixel bytes under tick 2's receipt. The second
    // observation must be rejected, and the episode CLI must commit no HDC files.
    let mut stale_replay = base_request.clone();
    stale_replay["observations"][1]["pixels_file"] = json!("frames/frame_0001.rgb");
    controls.push((
        "stale-frame-reuse".to_string(),
        "UpstreamFrameMismatch".to_string(),
        json!({
            "schema_version": 1,
            "bridge_config": base_request["bridge_config"].clone(),
            "vision_config": base_request["vision_config"].clone(),
            "observations": [
                base_request["observations"][0].clone(),
                stale_replay["observations"][1].clone()
            ]
        }),
    ));

    let mut pixel_corruption = base_request.clone();
    pixel_corruption["observations"][0]["pixels_file"] =
        json!("bridge-negative-controls/pixel-corrupt.rgb");
    controls.push((
        "pixel-corruption".to_string(),
        "UpstreamFrameMismatch".to_string(),
        pixel_corruption,
    ));

    let mut calibration_mismatch = base_request.clone();
    calibration_mismatch["observations"][0]["upstream"]["camera_calibration_sha256"] =
        json!("f".repeat(64));
    controls.push((
        "camera-calibration-mismatch".to_string(),
        "UpstreamCalibrationMismatch".to_string(),
        calibration_mismatch,
    ));

    let mut clock_regression = base_request.clone();
    clock_regression["observations"][0]["timestamp_us"] = json!(0);
    controls.push((
        "capture-clock-mismatch".to_string(),
        "UpstreamTimestampMismatch".to_string(),
        clock_regression,
    ));

    let mut sequence_zero = base_request.clone();
    sequence_zero["observations"][0]["sequence"] = json!(0);
    sequence_zero["observations"][0]["upstream"]["sequence"] = json!(0);
    controls.push((
        "zero-sequence".to_string(),
        "UpstreamSequenceMismatch".to_string(),
        sequence_zero,
    ));

    let mut receipt_tamper = base_request.clone();
    receipt_tamper["observations"][0]["upstream"]["receipt_sha256"] = json!("e".repeat(64));
    controls.push((
        "receipt-commitment-tamper".to_string(),
        "UpstreamReceiptCommitmentMismatch".to_string(),
        receipt_tamper,
    ));

    // Produce a valid renderer receipt for a sequence rollback: the upstream
    // world tick advances from 2 to 3, while its observation sequence falls
    // from 2 to 1. Upstream receipt identity remains internally consistent.
    let rollback_snapshot = snapshot(3, false);
    let rollback_clocks = clocks(3);
    let rollback_render =
        SoftwareCameraRenderer::render(&rollback_snapshot, 1, rollback_clocks)?;
    fs::write(
        negative_root.join("sequence-rollback.rgb"),
        &rollback_render.pixels_rgb,
    )?;
    let mut rollback_observation = base_request["observations"][2].clone();
    rollback_observation["sequence"] = json!(1);
    rollback_observation["pixels_file"] = json!("bridge-negative-controls/sequence-rollback.rgb");
    rollback_observation["upstream"] = serde_json::to_value(rollback_render.policy_receipt)?;
    let mut sequence_rollback = base_request.clone();
    sequence_rollback["observations"] = json!([
        base_request["observations"][1].clone(),
        rollback_observation
    ]);
    controls.push((
        "sequence-rollback".to_string(),
        "SequenceRegression".to_string(),
        sequence_rollback,
    ));

    // Keep the upstream clock receipt valid while moving tick 3's capture
    // timestamp behind tick 2. The bridge must reject before any output commit.
    let mut regressed_world = snapshot(3, false);
    regressed_world.world_timestamp_us = world_timestamp(2) - 1;
    let regressed_capture = regressed_world.world_timestamp_us;
    let regressed_clocks = RenderClocks {
        capture_timestamp_us: regressed_capture,
        render_completed_timestamp_us: regressed_capture + 800,
        delivered_timestamp_us: regressed_capture + 1_000,
    };
    let regressed_render =
        SoftwareCameraRenderer::render(&regressed_world, 3, regressed_clocks)?;
    fs::write(
        negative_root.join("capture-clock-regression.rgb"),
        &regressed_render.pixels_rgb,
    )?;
    let mut regressed_observation = base_request["observations"][2].clone();
    regressed_observation["sequence"] = json!(3);
    regressed_observation["timestamp_us"] = json!(regressed_capture);
    regressed_observation["vehicle_state_timestamp_us"] = json!(regressed_capture);
    regressed_observation["vehicle_state"]["timestamp"] =
        json!(regressed_capture as f64 / 1_000_000.0);
    regressed_observation["pixels_file"] =
        json!("bridge-negative-controls/capture-clock-regression.rgb");
    regressed_observation["upstream"] = serde_json::to_value(regressed_render.policy_receipt)?;
    let mut clock_regression = base_request.clone();
    clock_regression["observations"] = json!([
        base_request["observations"][1].clone(),
        regressed_observation
    ]);
    controls.push((
        "capture-clock-regression".to_string(),
        "TimestampRegression".to_string(),
        clock_regression,
    ));

    let mut records = Vec::with_capacity(controls.len());
    let mut all_fail_closed = true;

    for (control_id, expected_error, request) in controls {
        let request_path = episode_dir.join(format!("negative-{control_id}-request.json"));
        let control_dir = negative_root.join(&control_id);
        let receipt_path = control_dir.join("receipts.json");
        let policy_inputs_dir = control_dir.join("policy-inputs");
        write_json(&request_path, &request)?;

        let output = Command::new(bridge_binary)
            .arg(&request_path)
            .arg(&receipt_path)
            .arg(&policy_inputs_dir)
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let receipt_file_written = receipt_path.exists();
        let artifact_count = count_regular_files(&policy_inputs_dir)?;
        let expected_error_observed = stderr.contains(&expected_error);
        let fail_closed = !output.status.success()
            && !receipt_file_written
            && artifact_count == 0
            && expected_error_observed;
        if !fail_closed {
            all_fail_closed = false;
        }
        records.push(BridgeNegativeControlRecord {
            control_id,
            expected_error,
            exit_code: output.status.code(),
            receipt_file_written,
            policy_input_artifact_count: artifact_count,
            expected_error_observed,
            fail_closed,
            stderr_excerpt: stderr.chars().take(512).collect(),
        });
    }

    write_json(
        &negative_root.join("negative-control-results.json"),
        &records,
    )?;
    if !all_fail_closed {
        return Err(
            "one or more Symthaea bridge negative controls did not fail before receipt/HDC artifact creation"
                .into(),
        );
    }
    Ok((records.len(), all_fail_closed))
}

fn invoke_symthaea_bridge(
    episode_dir: &Path,
    bridge_config: Value,
    vision_config: Value,
    observations: Vec<Value>,
    bridge_binary: &Path,
) -> Result<(usize, bool, u64, u64, bool, bool, usize, bool), Box<dyn Error>> {
    let request_path = episode_dir.join("episode-request.json");
    let bridge_dir = episode_dir.join("symthaea-bridge");
    let first_receipts = bridge_dir.join("bridge-receipts.json");
    let first_policy_inputs = bridge_dir.join("policy-inputs");
    let replay_dir = bridge_dir.join("fresh-process-replay");
    let replay_receipts = replay_dir.join("bridge-receipts.json");
    let replay_policy_inputs = replay_dir.join("policy-inputs");

    let mut bridge_config = bridge_config;
    let expected_calibration = observations
        .first()
        .and_then(|observation| observation["upstream"]["camera_calibration_sha256"].as_str())
        .ok_or("first renderer observation is missing its calibration digest")?;
    bridge_config["expected_camera_calibration_sha256"] = json!(expected_calibration);
    let request = json!({
        "schema_version": 1,
        "bridge_config": bridge_config,
        "vision_config": vision_config,
        "observations": observations.clone()
    });
    write_json(&request_path, &request)?;

    // Each invocation is a new OS process and reconstructs a new Vision Manifold.
    let first_output = invoke_bridge_process(
        bridge_binary,
        &request_path,
        &first_receipts,
        &first_policy_inputs,
    )?;
    let (receipt_valid, artifact_valid) =
        validate_bridge_output(&first_output, &observations, &first_policy_inputs)?;

    let replay_output = invoke_bridge_process(
        bridge_binary,
        &request_path,
        &replay_receipts,
        &replay_policy_inputs,
    )?;
    let (replay_receipt_valid, replay_artifact_valid) =
        validate_bridge_output(&replay_output, &observations, &replay_policy_inputs)?;

    let receipt_replay_exact = first_output == replay_output
        && receipt_valid
        && replay_receipt_valid
        && artifact_valid
        && replay_artifact_valid;

    let first_artifacts = first_output["policy_input_artifacts"]
        .as_array()
        .ok_or("first bridge output is missing policy_input_artifacts[]")?;
    let replay_artifacts = replay_output["policy_input_artifacts"]
        .as_array()
        .ok_or("replay bridge output is missing policy_input_artifacts[]")?;
    let mut byte_replay_exact = first_artifacts.len() == replay_artifacts.len();
    for (left, right) in first_artifacts.iter().zip(replay_artifacts) {
        if left["relative_path"] != right["relative_path"] || left["sha256"] != right["sha256"] {
            byte_replay_exact = false;
            continue;
        }
        let relative = left["relative_path"].as_str().unwrap_or("");
        let first_bytes = fs::read(first_policy_inputs.join(relative))?;
        let replay_bytes = fs::read(replay_policy_inputs.join(relative))?;
        if first_bytes != replay_bytes {
            byte_replay_exact = false;
        }
    }

    let replay_exact = receipt_replay_exact && byte_replay_exact;
    if !replay_exact {
        return Err("fresh-process Symthaea bridge replay diverged in receipts or HDC artifacts".into());
    }

    let receipts = first_output["receipts"]
        .as_array()
        .ok_or("first bridge output is missing receipts[]")?;
    let frozen = first_output["vision_learning_frozen"].as_bool() == Some(true);
    let before = first_output["vision_training_steps_before"].as_u64().unwrap_or(u64::MAX);
    let after = first_output["vision_training_steps_after"].as_u64().unwrap_or(u64::MAX);
    if !frozen || before != after {
        return Err("Symthaea bridge did not preserve frozen-learning evidence".into());
    }

    let (negative_control_count, negative_controls_fail_closed) =
        run_bridge_negative_controls(episode_dir, bridge_binary, &request)?;

    write_json(
        &bridge_dir.join("fresh-process-replay-comparison.json"),
        &json!({
            "receipt_json_exact": receipt_replay_exact,
            "policy_input_artifact_bytes_exact": byte_replay_exact,
            "fresh_process_replay_exact": replay_exact,
            "observation_count": receipts.len()
        }),
    )?;

    Ok((
        receipts.len(),
        frozen,
        before,
        after,
        receipt_valid && artifact_valid,
        replay_exact,
        negative_control_count,
        negative_controls_fail_closed,
    ))
}

fn evidence_branch(branch_id: &str, parent_id: Option<&str>, branch_shift: bool, stale_replay: bool) -> Value {
    let lead_base = "lead-car-policy-input-at-tick-11";
    let lead_input = if branch_shift {
        "lead-car-policy-input-after-alternate-action"
    } else {
        lead_base
    };
    let lead_origin = if stale_replay { "replayed" } else { "recomputed" };
    let child = parent_id.is_some();
    json!({
        "branch_id": branch_id,
        "episode_id": EPISODE_ID,
        "parent_branch_id": parent_id,
        "checkpoint_digest": "checkpoint-at-tick-11-v1",
        "recurrent_state_policy_id": "untrained-input-policy-state-v1",
        "observation_history_digest": "policy-visible-observation-history-through-tick-11",
        "simulator_revision": SIMULATOR_REVISION,
        "decision_tick": 11,
        "outcome_tick": if child { 12 } else { 11 },
        "intervention_digest": if child { "alternate-action-receipt-digest" } else { "base-action-receipt-digest" },
        "expected_actor_ids": ["lead-car", "crossing-pedestrian"],
        "actor_responses": [
            {
                "actor_id": "lead-car",
                "policy_revision_digest": "lead-actor-policy-v1",
                "policy_input_digest": lead_input,
                "action_digest": if child && !stale_replay { "lead-car-recomputed-response" } else { "lead-car-base-response" },
                "randomness_digest": "lead-car-rng-fixture-11",
                "origin": lead_origin
            },
            {
                "actor_id": "crossing-pedestrian",
                "policy_revision_digest": "pedestrian-actor-policy-v1",
                "policy_input_digest": if child { "pedestrian-input-after-ego-intervention" } else { "pedestrian-input-base" },
                "action_digest": if child { "pedestrian-recomputed-response" } else { "pedestrian-base-response" },
                "randomness_digest": "pedestrian-rng-fixture-11",
                "origin": "recomputed"
            }
        ],
        "expected_random_event_ids": ["ambient-weather"],
        "random_events": [{
            "event_id": "ambient-weather",
            "shared_with_parent": child,
            "event_class": "exogenous",
            "invariant_under_intervention": true,
            "key_digest": "ambient-weather-exogenous-key-v1",
            "sampled_value_digest": "ambient-weather-value-v1"
        }],
        "policy_input_fields": [
            {"field_id":"front-camera-frame-at-tick-11","source_role":"policy_visible","available_tick":11},
            {"field_id":"ego-proprioception-at-tick-11","source_role":"policy_visible","available_tick":11}
        ],
        "query": null
    })
}

fn branch_audit_campaign(stale_actor_replay: bool) -> Value {
    json!({
        "expected_episode_ids": [EPISODE_ID],
        "expected_branch_ids": ["base-at-tick-11", "alternate-action-at-tick-12"],
        "episodes": [{
            "episode_id": EPISODE_ID,
            "disposition": "completed"
        }],
        "branches": [
            evidence_branch("base-at-tick-11", None, false, false),
            evidence_branch(
                "alternate-action-at-tick-12",
                Some("base-at-tick-11"),
                true,
                stale_actor_replay
            )
        ],
        "evaluation_controls": {
            "frozen": true,
            "teacher_blending_enabled": false,
            "online_updates_enabled": false,
            "exploration_noise_enabled": false,
            "teacher_inputs_visible": false,
            "future_outcomes_visible": false
        }
    })
}

fn invoke_branch_evidence_auditor(
    episode_dir: &Path,
    auditor_binary: &Path,
) -> Result<(bool, bool, usize), Box<dyn Error>> {
    let audit_dir = episode_dir.join("branch-evidence-auditor");
    fs::create_dir_all(&audit_dir)?;
    let valid_input = audit_dir.join("campaign-valid.json");
    let valid_output = audit_dir.join("report-valid.json");
    let stale_input = audit_dir.join("campaign-stale-actor-negative-control.json");
    let stale_output = audit_dir.join("report-stale-actor-negative-control.json");

    write_json(&valid_input, &branch_audit_campaign(false))?;
    write_json(&stale_input, &branch_audit_campaign(true))?;

    let valid_status = Command::new(auditor_binary)
        .arg(&valid_input)
        .arg(&valid_output)
        .status()?;
    if !valid_status.success() {
        return Err("branch auditor rejected the valid recomputed-actor campaign fixture".into());
    }
    let valid_report = read_json(&valid_output)?;
    let branch_audit_valid = valid_report["disposition"].as_str() == Some("valid")
        && valid_report["finding_count"].as_u64() == Some(0);
    if !branch_audit_valid {
        return Err("branch auditor valid fixture did not produce an empty finding set".into());
    }

    let stale_status = Command::new(auditor_binary)
        .arg(&stale_input)
        .arg(&stale_output)
        .status()?;
    if stale_status.success() {
        return Err("branch auditor accepted the stale actor replay negative control".into());
    }
    let stale_report = read_json(&stale_output)?;
    let findings = stale_report["findings"]
        .as_array()
        .ok_or("branch auditor negative-control report lacks findings[]")?;
    let stale_rejected = stale_report["disposition"].as_str() == Some("rejected")
        && findings.iter().any(|finding| {
            finding.as_str() == Some("ReplayedActorInputChanged")
        });
    if !stale_rejected {
        return Err("branch auditor negative control did not produce ReplayedActorInputChanged".into());
    }
    let finding_count = valid_report["finding_count"].as_u64().unwrap_or(0) as usize
        + stale_report["finding_count"].as_u64().unwrap_or(0) as usize;
    write_json(
        &audit_dir.join("audit-summary.json"),
        &json!({
            "valid_fixture_disposition": valid_report["disposition"],
            "valid_fixture_findings": valid_report["findings"],
            "stale_actor_negative_control_disposition": stale_report["disposition"],
            "stale_actor_negative_control_findings": stale_report["findings"],
            "stale_actor_replay_rejected": stale_rejected
        }),
    )?;
    Ok((branch_audit_valid, stale_rejected, finding_count))
}

fn degradation_trace_tick(
    tick: u64,
    condition: &str,
    accepted: bool,
    detected_event: Option<&str>,
    containment_event: Option<&str>,
    normal_mode: bool,
    degraded_mode: bool,
    fallback_active: bool,
) -> Value {
    let capture = world_timestamp(tick);
    let delivered = if tick == DELAYED_TICK {
        capture + clocks(tick).delivered_timestamp_us - clocks(tick).capture_timestamp_us
    } else {
        capture
    };
    json!({
        "tick": tick,
        "capture_timestamp_us": capture,
        "delivered_timestamp_us": delivered,
        "conditions": [condition],
        "policy_input_accepted": accepted,
        "fault_detection_event_ids": detected_event.into_iter().collect::<Vec<_>>(),
        "containment_event_ids": containment_event.into_iter().collect::<Vec<_>>(),
        "degraded_mode": degraded_mode,
        "fallback_active": fallback_active,
        "safe_stop_complete": false,
        "normal_mode": normal_mode,
        "decision_latency_us": 15_000 + tick * 100,
        "inference_energy_uj": 50 + tick
    })
}

fn degradation_campaign_value() -> Value {
    let clean_ticks = (1..=EXPECTED_TICKS)
        .map(|tick| {
            degradation_trace_tick(
                tick, "clean", true, None, None, true, false, false,
            )
        })
        .collect::<Vec<_>>();

    let faulted_ticks = (1..=EXPECTED_TICKS)
        .map(|tick| match tick {
            DELAYED_TICK => degradation_trace_tick(
                tick,
                "delayed",
                true,
                Some("frame-delay-007"),
                Some("frame-delay-007"),
                false,
                true,
                true,
            ),
            8 | 9 => degradation_trace_tick(
                tick,
                "clean",
                true,
                None,
                Some("frame-delay-007"),
                false,
                true,
                true,
            ),
            10 => degradation_trace_tick(
                tick, "clean", true, None, None, true, false, false,
            ),
            MISSING_TICK => degradation_trace_tick(
                tick,
                "missing",
                false,
                Some("camera-drop-011"),
                Some("camera-drop-011"),
                false,
                true,
                true,
            ),
            12 | 13 => degradation_trace_tick(
                tick,
                "clean",
                true,
                None,
                Some("camera-drop-011"),
                false,
                true,
                true,
            ),
            _ => degradation_trace_tick(
                tick, "clean", true, None, None, true, false, false,
            ),
        })
        .collect::<Vec<_>>();

    let scenario_seed = sha256_hex(b"fresh-process-driving-scenario-seed-v1");
    let environment_randomness =
        sha256_hex(b"fresh-process-driving-exogenous-randomness-v1");
    json!({
        "schema_version": 1,
        "campaign_id": "fresh-process-renderer-bridge-degradation-v1",
        "model_id": "symthaea-camera-driving-input-fixture",
        "model_status": "untrained_input_bridge",
        "model_revision": "symthaea-camera-driving-bridge-v0.1",
        "checkpoint_sha256": null,
        "hardware_id": "synthetic-fixture-host-not-measured",
        "inference_precision": "fp32-fixture",
        "power_profile": "not-measured-fixture",
        "energy_measurement_method": "synthetic-fixture-values-not-hardware-measurement",
        "policy_visible_inputs": ["front-camera-rgb8", "vehicle-state-vector"],
        "benchmark_revision": "degradation-recovery-envelope-v0.1",
        "evaluator_revision": "degradation-recovery-auditor-v1",
        "clock_id": CLOCK_ID,
        "policy_deadline_us": 100_000,
        "fault_schedule_visible_to_policy": false,
        "clean_control": {
            "run_id": "renderer-bridge-clean-control",
            "disposition": "complete",
            "scenario_seed_sha256": scenario_seed,
            "environment_randomness_sha256": environment_randomness,
            "expected_tick_count": EXPECTED_TICKS,
            "ticks": clean_ticks
        },
        "faulted_run": {
            "run_id": "renderer-bridge-faulted",
            "disposition": "complete",
            "scenario_seed_sha256": scenario_seed,
            "environment_randomness_sha256": environment_randomness,
            "expected_tick_count": EXPECTED_TICKS,
            "ticks": faulted_ticks
        },
        "events": [
            {
                "event_id": "frame-delay-007",
                "causality": "independent",
                "causal_group_id": null,
                "parent_event_id": null,
                "kind": "frame_delay",
                "start_tick": DELAYED_TICK,
                "end_tick": DELAYED_TICK,
                "severity_ppm": 500_000,
                "detection_deadline_ticks": 0,
                "containment_deadline_ticks": 0,
                "max_accepted_frame_age_us": 5_000,
                "recovery_healthy_quorum_ticks": 2
            },
            {
                "event_id": "camera-drop-011",
                "causality": "independent",
                "causal_group_id": null,
                "parent_event_id": null,
                "kind": "camera_drop",
                "start_tick": MISSING_TICK,
                "end_tick": MISSING_TICK,
                "severity_ppm": 1_000_000,
                "detection_deadline_ticks": 0,
                "containment_deadline_ticks": 0,
                "max_accepted_frame_age_us": 0,
                "recovery_healthy_quorum_ticks": 2
            }
        ]
    })
}

fn invoke_degradation_recovery_auditor(
    episode_dir: &Path,
    auditor_binary: &Path,
) -> Result<(bool, bool, usize, usize), Box<dyn Error>> {
    let audit_dir = episode_dir.join("degradation-recovery-auditor");
    fs::create_dir_all(&audit_dir)?;
    let valid_input = audit_dir.join("campaign-from-renderer-bridge.json");
    let valid_output = audit_dir.join("report-valid.json");
    let negative_input = audit_dir.join("campaign-missing-frame-accepted-negative-control.json");
    let negative_output = audit_dir.join("report-missing-frame-accepted-negative-control.json");

    let campaign = degradation_campaign_value();
    write_json(&valid_input, &campaign)?;
    let valid_status = Command::new(auditor_binary)
        .arg(&valid_input)
        .arg(&valid_output)
        .status()?;
    if !valid_status.success() {
        return Err("degradation/recovery auditor rejected the paired renderer/bridge campaign".into());
    }
    let valid_report = read_json(&valid_output)?;
    let audit_valid = valid_report["disposition"].as_str() == Some("valid")
        && valid_report["findings"].as_array().is_some_and(Vec::is_empty)
        && valid_report["expected_ticks"].as_u64() == Some(EXPECTED_TICKS)
        && valid_report["faulted_observed_ticks"].as_u64() == Some(EXPECTED_TICKS)
        && valid_report["fault_event_count"].as_u64() == Some(2);
    let event_reports = valid_report["events"].as_array()
        .ok_or("degradation auditor report lacks events[]")?;
    if !audit_valid
        || event_reports[0]["detection_latency_ticks"].as_u64() != Some(0)
        || event_reports[0]["containment_latency_ticks"].as_u64() != Some(0)
        || event_reports[0]["recovery_tick"].as_u64() != Some(10)
        || event_reports[1]["detection_latency_ticks"].as_u64() != Some(0)
        || event_reports[1]["containment_latency_ticks"].as_u64() != Some(0)
        || event_reports[1]["recovery_tick"].as_u64() != Some(14)
    {
        return Err("degradation/recovery positive campaign did not produce the expected event metrics".into());
    }

    let mut negative_campaign = campaign;
    negative_campaign["faulted_run"]["ticks"][(MISSING_TICK - 1) as usize]
        ["policy_input_accepted"] = json!(true);
    write_json(&negative_input, &negative_campaign)?;
    let negative_status = Command::new(auditor_binary)
        .arg(&negative_input)
        .arg(&negative_output)
        .status()?;
    if negative_status.code() != Some(1) {
        return Err("degradation auditor did not reject missing-frame acceptance with campaign-rejected exit code 1".into());
    }
    let negative_report = read_json(&negative_output)?;
    let findings = negative_report["findings"]
        .as_array()
        .ok_or("degradation negative-control report lacks findings[]")?;
    let negative_control_rejected = negative_report["disposition"].as_str() == Some("rejected")
        && findings.iter().any(|finding| {
            finding.as_str() == Some("forbidden_policy_input_accepted")
        });
    if !negative_control_rejected {
        return Err("degradation negative control did not produce forbidden_policy_input_accepted".into());
    }

    let finding_count = valid_report["findings"].as_array().map_or(0, Vec::len)
        + findings.len();
    write_json(
        &audit_dir.join("audit-summary.json"),
        &json!({
            "valid_renderer_bridge_campaign": valid_report,
            "missing_frame_acceptance_negative_control": negative_report,
            "missing_frame_acceptance_rejected": negative_control_rejected,
            "event_count": event_reports.len()
        }),
    )?;
    Ok((audit_valid, negative_control_rejected, event_reports.len(), finding_count))
}

fn run_parent(
    episode_dir: &Path,
    bridge_config_path: &Path,
    vision_config_path: &Path,
    bridge_binary: &Path,
    branch_auditor_binary: &Path,
    degradation_auditor_binary: &Path,
) -> Result<EpisodeReport, Box<dyn Error>> {
    if episode_dir.exists() {
        let mut existing = fs::read_dir(episode_dir)?;
        if existing.next().is_some() {
            return Err(
                "episode output directory must be empty; use a new directory to prevent stale-artifact reuse"
                    .into(),
            );
        }
    } else {
        fs::create_dir_all(episode_dir)?;
    }
    let episode_dir = fs::canonicalize(episode_dir)?;
    let bridge_binary = fs::canonicalize(bridge_binary)?;
    if !bridge_binary.is_file() {
        return Err("Symthaea bridge binary is not a regular file".into());
    }
    let branch_auditor_binary = fs::canonicalize(branch_auditor_binary)?;
    if !branch_auditor_binary.is_file() {
        return Err("recoverable-driving-evidence auditor binary is not a regular file".into());
    }
    let degradation_auditor_binary = fs::canonicalize(degradation_auditor_binary)?;
    if !degradation_auditor_binary.is_file() {
        return Err("degradation-recovery-envelope auditor binary is not a regular file".into());
    }

    let bridge_config = read_json(bridge_config_path)?;
    let vision_config = read_json(vision_config_path)?;
    validate_bridge_configuration(&bridge_config, &vision_config)?;

    let mut guard = ObservationStreamGuard::new(TimingPolicy {
        maximum_render_latency_us: MAX_RENDER_LATENCY_US,
        maximum_delivery_age_us: MAX_DELIVERY_AGE_US,
    });
    let mut missing_events = Vec::new();
    let mut dispositions = Vec::new();
    let mut replay_records = Vec::new();
    let mut action_records = Vec::new();
    let mut transition_records = Vec::new();
    let mut fresh_count = 0_usize;
    let mut delayed_count = 0_usize;

    for tick in 1..=EXPECTED_TICKS {
        let world = snapshot(tick, false);
        let capture_time = world.world_timestamp_us;
        let action = world
            .applied_action
            .as_ref()
            .ok_or("episode snapshot missing applied action")?;
        action_records.push(AppliedActionRecord {
            action_receipt_id: action.receipt_id.clone(),
            action_receipt_sha256: action.receipt_sha256.clone(),
            command_id: action.command_id.clone(),
            applied_tick: action.applied_tick,
            source: "declared_pre_recorded_fixture_command".to_string(),
        });
        let audit = SoftwareCameraRenderer::audit_snapshot(&world)?;
        transition_records.push(TransitionRecord {
            tick,
            applied_action_receipt_id: action.receipt_id.clone(),
            world_snapshot_sha256: audit.world_snapshot_sha256,
            simulator_revision: world.simulator_revision.clone(),
            note: "Immutable fixture snapshot declaration; no vehicle-dynamics engine is executed by this harness.".to_string(),
        });

        if tick == MISSING_TICK {
            let event = guard.record_missing(
                EPISODE_ID,
                AGENT_ID,
                CLOCK_ID,
                tick,
                tick,
                capture_time,
                capture_time + 1_200,
                MissingFrameReason::CameraDrop,
            )?;
            missing_events.push(event);
            continue;
        }

        let render_clocks = clocks(tick);
        let disposition = guard.admit(
            CLOCK_ID,
            tick,
            render_clocks.capture_timestamp_us,
            render_clocks.render_completed_timestamp_us,
            render_clocks.delivered_timestamp_us,
        )?;
        match disposition {
            ObservationDisposition::Fresh => fresh_count += 1,
            ObservationDisposition::Delayed => delayed_count += 1,
            ObservationDisposition::Stale => {
                return Err(format!("fixture unexpectedly produced stale frame at tick {tick}").into())
            }
        }
        dispositions.push(DispositionRecord {
            sequence: tick,
            tick,
            disposition,
            capture_timestamp_us: render_clocks.capture_timestamp_us,
            render_completed_timestamp_us: render_clocks.render_completed_timestamp_us,
            delivered_timestamp_us: render_clocks.delivered_timestamp_us,
        });
        replay_records.push(ReplaySnapshotRecord {
            sequence: tick,
            snapshot: world,
            clocks: render_clocks,
        });
    }

    if missing_events.len() != 1 || delayed_count != 1 || replay_records.len() != 19 {
        return Err("fixture event denominator did not match the 20-tick plan".into());
    }

    let bridge_observations = save_render_artifacts(
        &episode_dir,
        &replay_records,
        &missing_events,
        &dispositions,
    )?;
    write_json(&episode_dir.join("applied-actions.json"), &action_records)?;
    write_json(&episode_dir.join("world-transition-records.json"), &transition_records)?;

    let changed_branch = make_changed_action_branch(&episode_dir)?;
    let (branch_audit_valid, stale_actor_rejected, branch_audit_findings) =
        invoke_branch_evidence_auditor(&episode_dir, &branch_auditor_binary)?;
    let (
        degradation_audit_valid,
        missing_frame_negative_control_rejected,
        degradation_event_count,
        degradation_finding_count,
    ) = invoke_degradation_recovery_auditor(&episode_dir, &degradation_auditor_binary)?;
    if changed_branch.base_frame_sha256 == changed_branch.branch_frame_sha256
        || changed_branch.base_world_snapshot_sha256 == changed_branch.branch_world_snapshot_sha256
    {
        return Err("alternate action branch did not change the declared observation/world state".into());
    }
    write_json(
        &episode_dir.join("counterfactual-branch-record.json"),
        &changed_branch,
    )?;

    // First-process expected render summaries.
    let replay_input = ReplayInput {
        records: replay_records.clone(),
        missing_events: missing_events.clone(),
        dispositions: dispositions.clone(),
    };
    let expected_replay = replay_output(&replay_input)?;
    let replay_input_path = episode_dir.join("fresh-process-replay-input.json");
    let replay_output_path = episode_dir.join("fresh-process-replay-output.json");
    write_json(&replay_input_path, &replay_input)?;
    let _ = fs::remove_file(&replay_output_path);

    // A distinct OS process reconstructs all delivered frames from immutable snapshots.
    let executable = env::current_exe()?;
    let status = Command::new(executable)
        .arg("--replay-child")
        .arg(&replay_input_path)
        .arg(&replay_output_path)
        .status()?;
    if !status.success() {
        return Err("fresh-process renderer child failed".into());
    }
    let actual_replay: ReplayOutput =
        serde_json::from_slice(&fs::read(&replay_output_path)?)?;
    let replay_exact = actual_replay == expected_replay;
    if !replay_exact {
        return Err("fresh-process renderer replay mismatch".into());
    }
    write_json(
        &episode_dir.join("fresh-process-replay-comparison.json"),
        &json!({
            "expected_count": expected_replay.rendered_frames.len(),
            "actual_count": actual_replay.rendered_frames.len(),
            "missing_events_sha256": actual_replay.missing_events_sha256,
            "dispositions_sha256": actual_replay.dispositions_sha256,
            "exact_match": replay_exact,
            "replay": actual_replay
        }),
    )?;

    let (
        bridge_count,
        frozen,
        training_before,
        training_after,
        bridge_chain_valid,
        bridge_replay_exact,
        bridge_negative_control_count,
        bridge_negative_controls_fail_closed,
    ) = invoke_symthaea_bridge(
            &episode_dir,
            bridge_config,
            vision_config,
            bridge_observations,
            &bridge_binary,
        )?;

    let report = EpisodeReport {
        schema_version: 1,
        status: "complete_for_declared_non_learning_fixture".to_string(),
        episode_id: EPISODE_ID.to_string(),
        scenario_id: SCENARIO_ID.to_string(),
        renderer_revision: RENDERER_REVISION.to_string(),
        simulator_backend: SIMULATOR_REVISION.to_string(),
        expected_ticks: EXPECTED_TICKS,
        applied_actions_recorded: action_records.len(),
        world_snapshot_transition_records: transition_records.len(),
        counterfactual_branches: 1,
        delivered_observations: replay_records.len(),
        missing_observations: missing_events.len(),
        delayed_observations: delayed_count,
        fresh_observations: fresh_count,
        fresh_process_renderer_replay_exact: replay_exact,
        fresh_process_symthaea_bridge_replay_exact: bridge_replay_exact,
        bridge_negative_control_count,
        bridge_negative_controls_fail_closed,
        branch_evidence_audit_valid: branch_audit_valid,
        stale_actor_replay_negative_control_rejected: stale_actor_rejected,
        branch_evidence_finding_count: branch_audit_findings,
        degradation_recovery_audit_valid: degradation_audit_valid,
        missing_frame_acceptance_negative_control_rejected: missing_frame_negative_control_rejected,
        degradation_recovery_event_count: degradation_event_count,
        degradation_recovery_finding_count: degradation_finding_count,
        symthaea_bridge_invoked: true,
        symthaea_bridge_processed_observations: bridge_count,
        symthaea_receipt_chain_valid: bridge_chain_valid,
        policy_input_artifacts_valid: bridge_chain_valid,
        policy_input_artifact_count: bridge_count,
        bridge_vision_learning_frozen: frozen,
        bridge_training_steps_before: training_before,
        bridge_training_steps_after: training_after,
        counterfactual_branch_frame_changed: true,
        claim_ceiling: "Finite immutable snapshot/render/bridge fixture only. The harness does not run vehicle dynamics, actor policies, a driving controller or physical actuation, and does not establish learned visual competence or safety.".to_string(),
    };
    write_json(&episode_dir.join("episode-report.json"), &report)?;
    Ok(report)
}

fn run_replay_child(input_path: &Path, output_path: &Path) -> Result<(), Box<dyn Error>> {
    let input: ReplayInput = serde_json::from_slice(&fs::read(input_path)?)?;
    let replay = replay_output(&input)?;
    write_json(output_path, &replay)?;
    Ok(())
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = if args.first().is_some_and(|arg| arg == "--replay-child") {
        if args.len() != 3 {
            Err("usage: --replay-child <input.json> <output.json>".into())
        } else {
            run_replay_child(Path::new(&args[1]), Path::new(&args[2]))
        }
    } else if args.len() == 6 {
        run_parent(
            Path::new(&args[0]),
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
            Path::new(&args[4]),
            Path::new(&args[5]),
        )
        .map(|report| {
            println!(
                "episode {}: {} / {} observations processed; missing={}, delayed={}; renderer_replay_exact={}",
                report.episode_id,
                report.symthaea_bridge_processed_observations,
                report.expected_ticks,
                report.missing_observations,
                report.delayed_observations,
                report.fresh_process_renderer_replay_exact
            );
        })
    } else {
        Err(
            "usage: fresh-process-driving-replay <episode-dir> <bridge-config.json> <vision-config.json> <camera-driving-bridge-binary> <recoverable-driving-evidence-binary> <degradation-recovery-envelope-binary>"
                .into(),
        )
    };

    if let Err(error) = result {
        eprintln!("fresh-process-driving-replay: {error}");
        std::process::exit(2);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_has_twenty_monotonic_ticks() {
        let timestamps = (1..=EXPECTED_TICKS)
            .map(world_timestamp)
            .collect::<Vec<_>>();
        assert_eq!(timestamps.len(), 20);
        assert!(timestamps.windows(2).all(|pair| pair[1] > pair[0]));
    }

    #[test]
    fn tick_seven_is_delayed_but_not_stale() {
        let clocks = clocks(DELAYED_TICK);
        assert_eq!(
            clocks.render_completed_timestamp_us - clocks.capture_timestamp_us,
            2_500
        );
        assert_eq!(
            clocks.delivered_timestamp_us - clocks.capture_timestamp_us,
            3_500
        );
        assert!(clocks.delivered_timestamp_us - clocks.capture_timestamp_us
            <= MAX_DELIVERY_AGE_US);
        assert!(clocks.render_completed_timestamp_us - clocks.capture_timestamp_us
            > MAX_RENDER_LATENCY_US);
    }

    #[test]
    fn vehicle_state_clock_uses_same_episode_monotonic_origin() {
        for tick in 1..=EXPECTED_TICKS {
            let state = vehicle_state(tick);
            let state_timestamp = state["timestamp"].as_f64().unwrap();
            let expected = world_timestamp(tick) as f64 / 1_000_000.0;
            assert!((state_timestamp - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn alternate_action_branch_changes_action_world_and_render_digests() {
        let base = snapshot(12, false);
        let alternative = snapshot(12, true);
        assert_ne!(
            base.applied_action.as_ref().unwrap().receipt_id,
            alternative.applied_action.as_ref().unwrap().receipt_id
        );
        let base_render = SoftwareCameraRenderer::render(&base, 12, clocks(12)).unwrap();
        let branch_render =
            SoftwareCameraRenderer::render(&alternative, 12, clocks(12)).unwrap();
        let base_audit = SoftwareCameraRenderer::audit_snapshot(&base).unwrap();
        let branch_audit = SoftwareCameraRenderer::audit_snapshot(&alternative).unwrap();
        assert_ne!(
            base_render.policy_receipt.frame_sha256,
            branch_render.policy_receipt.frame_sha256
        );
        assert_ne!(
            base_render.policy_receipt.observation_id,
            branch_render.policy_receipt.observation_id
        );
        assert_ne!(
            base_audit.world_snapshot_sha256,
            branch_audit.world_snapshot_sha256
        );
    }

    #[test]
    fn action_receipts_are_deterministic_and_digest_bound() {
        let first = action_ref(4, false);
        let second = action_ref(4, false);
        let changed = action_ref(4, true);
        assert_eq!(first, second);
        assert_ne!(first.receipt_id, changed.receipt_id);
        assert_ne!(first.receipt_sha256, changed.receipt_sha256);
        assert_eq!(first.receipt_sha256.len(), 64);
    }

    #[test]
    fn missing_and_delayed_events_survive_stream_ledger_replay() {
        let records = (1..=EXPECTED_TICKS)
            .filter(|tick| *tick != MISSING_TICK)
            .map(|tick| ReplaySnapshotRecord {
                sequence: tick,
                snapshot: snapshot(tick, false),
                clocks: clocks(tick),
            })
            .collect::<Vec<_>>();
        let missing_events = vec![MissingObservationEvent {
            episode_id: EPISODE_ID.to_string(),
            agent_id: AGENT_ID.to_string(),
            clock_id: CLOCK_ID.to_string(),
            tick: MISSING_TICK,
            sequence: MISSING_TICK,
            capture_timestamp_us: world_timestamp(MISSING_TICK),
            noticed_timestamp_us: world_timestamp(MISSING_TICK) + 1_200,
            reason: MissingFrameReason::CameraDrop,
        }];
        let dispositions = records
            .iter()
            .map(|record| DispositionRecord {
                sequence: record.sequence,
                tick: record.snapshot.tick,
                disposition: if record.sequence == DELAYED_TICK {
                    ObservationDisposition::Delayed
                } else {
                    ObservationDisposition::Fresh
                },
                capture_timestamp_us: record.clocks.capture_timestamp_us,
                render_completed_timestamp_us: record.clocks.render_completed_timestamp_us,
                delivered_timestamp_us: record.clocks.delivered_timestamp_us,
            })
            .collect();
        let input = ReplayInput {
            records,
            missing_events,
            dispositions,
        };
        assert!(validate_stream_ledger(&input).is_ok());
        assert!(replay_output(&input).is_ok());
    }

    #[test]
    fn branch_audit_payload_changes_actor_input_and_marks_stale_replay_negative_control() {
        let base = branch_audit_campaign(false);
        let stale = branch_audit_campaign(true);
        let parent = &base["branches"][0]["actor_responses"][0];
        let recomputed = &base["branches"][1]["actor_responses"][0];
        let stale_replayed = &stale["branches"][1]["actor_responses"][0];
        assert_ne!(parent["policy_input_digest"], recomputed["policy_input_digest"]);
        assert_eq!(recomputed["origin"], "recomputed");
        assert_eq!(stale_replayed["policy_input_digest"], recomputed["policy_input_digest"]);
        assert_eq!(stale_replayed["origin"], "replayed");
        assert_eq!(base["expected_branch_ids"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn degradation_campaign_preserves_real_fixture_drop_delay_and_recovery_events() {
        let campaign = degradation_campaign_value();
        assert_eq!(campaign["faulted_run"]["expected_tick_count"], EXPECTED_TICKS);
        let ticks = campaign["faulted_run"]["ticks"].as_array().unwrap();
        assert_eq!(ticks.len(), 20);
        let delayed = &ticks[(DELAYED_TICK - 1) as usize];
        assert_eq!(delayed["conditions"], json!(["delayed"]));
        assert_eq!(delayed["policy_input_accepted"], true);
        assert_eq!(
            delayed["delivered_timestamp_us"].as_u64().unwrap()
                - delayed["capture_timestamp_us"].as_u64().unwrap(),
            3_500
        );
        let missing = &ticks[(MISSING_TICK - 1) as usize];
        assert_eq!(missing["conditions"], json!(["missing"]));
        assert_eq!(missing["policy_input_accepted"], false);
        assert_eq!(
            missing["fault_detection_event_ids"],
            json!(["camera-drop-011"])
        );
        assert_eq!(campaign["events"].as_array().unwrap().len(), 2);
        assert_eq!(campaign["fault_schedule_visible_to_policy"], false);
    }

    #[test]
    fn degradation_negative_control_changes_only_missing_frame_acceptance() {
        let mut campaign = degradation_campaign_value();
        let missing = &mut campaign["faulted_run"]["ticks"][(MISSING_TICK - 1) as usize];
        assert_eq!(missing["conditions"], json!(["missing"]));
        assert_eq!(missing["policy_input_accepted"], false);
        missing["policy_input_accepted"] = json!(true);
        assert_eq!(missing["conditions"], json!(["missing"]));
        assert_eq!(missing["policy_input_accepted"], true);
    }

    #[test]
    fn bridge_configuration_requires_the_declared_camera_episode() {
        let bridge_config = json!({
            "identity": {
                "run_id":"run-1",
                "episode_id":EPISODE_ID,
                "scenario_id":SCENARIO_ID,
                "agent_id":AGENT_ID,
                "camera_id":CAMERA_ID,
                "simulator_revision":SIMULATOR_REVISION,
                "policy_revision":"policy-v1"
            },
            "max_width":640,
            "max_height":360,
            "max_frame_bytes":691200,
            "initial_dt_us":33333,
            "maximum_frame_gap_us":66666,
            "maximum_sensor_clock_skew_us":1000,
            "visual_weight_ppm":500000,
            "vehicle_encoder_seed_phrase":"fixture-seed"
        });
        let vision_config = json!({"hdc_dim":16384,"enable_depth":false});
        assert!(validate_bridge_configuration(&bridge_config, &vision_config).is_ok());
        let mut invalid = bridge_config;
        invalid["identity"]["camera_id"] = json!("rear");
        assert!(validate_bridge_configuration(&invalid, &vision_config).is_err());
    }
}

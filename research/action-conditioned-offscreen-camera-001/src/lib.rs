//! Deterministic offscreen perspective camera from immutable world snapshots.
//!
//! The crate owns rendering and observation receipt boundaries only. It does not
//! advance vehicle dynamics, run actor policies, choose actions, or certify safety.

// Indexed loops mirror fixed-width camera matrices and the SHA-256 schedule.
#![allow(clippy::needless_range_loop)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub const RENDERER_SCHEMA_VERSION: u32 = 1;
pub const RENDERER_REVISION: &str = "software-camera-q16-rgb-v1";
pub const Q16_ONE: i64 = 65_536;
pub const MAX_FRAME_WIDTH: u32 = 1_280;
pub const MAX_FRAME_HEIGHT: u32 = 720;
pub const MAX_FRAME_BYTES: usize = 1280 * 720 * 3;
pub const MAX_ID_BYTES: usize = 256;
pub const MAX_ROAD_MARKINGS: usize = 256;
pub const MAX_POINTS_PER_MARKING: usize = 8_192;
pub const MAX_TOTAL_ROAD_POINTS: usize = 32_768;
pub const MAX_ACTORS: usize = 128;
pub const MIN_NEAR_PLANE_MM: u32 = 100;
pub const MAX_FAR_PLANE_MM: u32 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedActionRef {
    pub receipt_id: String,
    pub receipt_sha256: String,
    pub command_id: String,
    pub applied_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum RoadMarkingKind {
    LaneWhite,
    LaneYellow,
    RoadEdge,
    StopLine,
}

impl RoadMarkingKind {
    fn color(self) -> [u8; 3] {
        match self {
            Self::LaneWhite => [236, 236, 224],
            Self::LaneYellow => [242, 205, 56],
            Self::RoadEdge => [214, 218, 222],
            Self::StopLine => [248, 248, 248],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadPolyline {
    pub road_id: String,
    pub marking_id: String,
    pub kind: RoadMarkingKind,
    pub points_world_mm: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ActorClass {
    Car,
    Bus,
    Bicycle,
    Pedestrian,
    Other,
}

impl ActorClass {
    fn color(self) -> [u8; 3] {
        match self {
            Self::Car => [214, 72, 58],
            Self::Bus => [235, 142, 45],
            Self::Bicycle => [54, 180, 116],
            Self::Pedestrian => [65, 134, 226],
            Self::Other => [190, 94, 205],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCuboid {
    pub actor_id: String,
    pub actor_policy_revision: String,
    pub class: ActorClass,
    pub center_world_mm: [i64; 3],
    pub half_extents_mm: [u32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraCalibration {
    pub camera_id: String,
    pub position_world_mm: [i64; 3],
    /// World-to-camera rotation matrix, signed Q16.16, camera axes [right, down, forward].
    pub world_to_camera_q16: [[i32; 3]; 3],
    /// Focal lengths and principal point in thousandths of one pixel.
    pub focal_x_millipx: u32,
    pub focal_y_millipx: u32,
    pub principal_x_millipx: u32,
    pub principal_y_millipx: u32,
    pub width: u32,
    pub height: u32,
    pub near_plane_mm: u32,
    pub far_plane_mm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRenderSnapshot {
    pub episode_id: String,
    pub scenario_id: String,
    pub agent_id: String,
    pub tick: u64,
    pub world_timestamp_us: u64,
    pub clock_id: String,
    pub simulator_revision: String,
    /// None is permitted only for the initial tick (tick 0).
    pub applied_action: Option<AppliedActionRef>,
    pub camera: CameraCalibration,
    pub road_markings: Vec<RoadPolyline>,
    pub actors: Vec<ActorCuboid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderClocks {
    pub capture_timestamp_us: u64,
    pub render_completed_timestamp_us: u64,
    pub delivered_timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingPolicy {
    pub maximum_render_latency_us: u64,
    pub maximum_delivery_age_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ObservationDisposition {
    Fresh,
    Delayed,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingFrameReason {
    CameraDrop,
    CameraUnavailable,
    RendererOverrun,
    TransportDrop,
    MalformedCapture,
    Censored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingObservationEvent {
    pub episode_id: String,
    pub agent_id: String,
    pub clock_id: String,
    pub tick: u64,
    pub sequence: u64,
    pub capture_timestamp_us: u64,
    pub noticed_timestamp_us: u64,
    pub reason: MissingFrameReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderError {
    EmptyIdentity,
    IdentityTooLong,
    InitialActionMustBeAbsent,
    AppliedActionMissing,
    AppliedActionTickMismatch,
    InvalidActionReceiptDigest,
    InvalidFrameGeometry,
    InvalidFocalLength,
    InvalidNearFarPlane,
    InvalidCameraTransform,
    InvalidObservationSequence,
    TooManyRoadMarkings,
    TooManyRoadPoints,
    TooManyActors,
    DuplicateMarkingIdentity,
    DuplicateActorIdentity,
    InvalidWorldCoordinate,
    InvalidActorExtent,
    InvalidPolyline,
    ClockCaptureMismatch,
    RenderBeforeCapture,
    DeliveryBeforeRender,
    ClockIdentityChanged,
    MissingEventBeforeCapture,
    SequenceRegression,
    CaptureClockRegression,
    DeliveryClockRegression,
    FrameAllocationExceeded,
    ProjectionOverflow,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RenderError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormat {
    Rgb8Srgb,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyObservationReceipt {
    pub schema_version: u32,
    pub observation_id: String,
    pub pixel_format: PixelFormat,
    pub episode_id: String,
    pub agent_id: String,
    pub clock_id: String,
    pub tick: u64,
    pub sequence: u64,
    pub applied_action_receipt_id: Option<String>,
    pub applied_action_receipt_sha256: Option<String>,
    pub applied_action_command_id: Option<String>,
    pub applied_action_tick: Option<u64>,
    pub camera_id: String,
    pub camera_calibration_sha256: String,
    pub renderer_revision: String,
    pub capture_timestamp_us: u64,
    pub render_completed_timestamp_us: u64,
    pub delivered_timestamp_us: u64,
    pub frame_width: u32,
    pub frame_height: u32,
    pub channels: u8,
    pub frame_byte_len: usize,
    pub frame_sha256: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorObservationAudit {
    pub episode_id: String,
    pub scenario_id: String,
    pub agent_id: String,
    pub clock_id: String,
    pub tick: u64,
    pub simulator_revision: String,
    pub world_snapshot_sha256: String,
    pub road_ids: Vec<String>,
    pub actor_ids: Vec<String>,
    pub actor_policy_revisions: Vec<String>,
    pub renderer_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedObservation {
    /// RGB8 sRGB pixel buffer. No evaluator-only world metadata is included.
    pub pixels_rgb: Vec<u8>,
    pub policy_receipt: PolicyObservationReceipt,
}

pub struct ObservationStreamGuard {
    timing: TimingPolicy,
    clock_id: Option<String>,
    last_sequence: Option<u64>,
    last_capture_timestamp_us: Option<u64>,
    last_delivery_timestamp_us: Option<u64>,
}

impl ObservationStreamGuard {
    pub fn new(timing: TimingPolicy) -> Self {
        Self {
            timing,
            clock_id: None,
            last_sequence: None,
            last_capture_timestamp_us: None,
            last_delivery_timestamp_us: None,
        }
    }

    /// Admit order/clock state atomically. Fresh, delayed and stale are explicit
    /// outcomes. Delayed/stale frames still consume their sequence number so
    /// replaying them later cannot silently promote them to fresh observations.
    pub fn admit(
        &mut self,
        clock_id: &str,
        sequence: u64,
        capture_timestamp_us: u64,
        render_completed_timestamp_us: u64,
        delivered_timestamp_us: u64,
    ) -> Result<ObservationDisposition, RenderError> {
        validate_id(clock_id)?;
        if self.clock_id.as_deref().is_some_and(|expected| expected != clock_id) {
            return Err(RenderError::ClockIdentityChanged);
        }
        if self.last_sequence.is_some_and(|last| sequence <= last) {
            return Err(RenderError::SequenceRegression);
        }
        if self
            .last_capture_timestamp_us
            .is_some_and(|last| capture_timestamp_us <= last)
        {
            return Err(RenderError::CaptureClockRegression);
        }
        if self
            .last_delivery_timestamp_us
            .is_some_and(|last| delivered_timestamp_us < last)
        {
            return Err(RenderError::DeliveryClockRegression);
        }
        if render_completed_timestamp_us < capture_timestamp_us {
            return Err(RenderError::RenderBeforeCapture);
        }
        if delivered_timestamp_us < render_completed_timestamp_us {
            return Err(RenderError::DeliveryBeforeRender);
        }
        let render_latency = render_completed_timestamp_us - capture_timestamp_us;
        let delivery_age = delivered_timestamp_us - capture_timestamp_us;
        let disposition = if delivery_age > self.timing.maximum_delivery_age_us {
            ObservationDisposition::Stale
        } else if render_latency > self.timing.maximum_render_latency_us {
            ObservationDisposition::Delayed
        } else {
            ObservationDisposition::Fresh
        };
        self.clock_id = Some(clock_id.to_string());
        self.last_sequence = Some(sequence);
        self.last_capture_timestamp_us = Some(capture_timestamp_us);
        self.last_delivery_timestamp_us = Some(delivered_timestamp_us);
        Ok(disposition)
    }

    /// Record a missing observation as an ordered stream event. A later copy
    /// of this sequence cannot be replayed as a fresh frame.
    pub fn record_missing(
        &mut self,
        episode_id: &str,
        agent_id: &str,
        clock_id: &str,
        tick: u64,
        sequence: u64,
        capture_timestamp_us: u64,
        noticed_timestamp_us: u64,
        reason: MissingFrameReason,
    ) -> Result<MissingObservationEvent, RenderError> {
        validate_id(episode_id)?;
        validate_id(agent_id)?;
        validate_id(clock_id)?;
        if self.clock_id.as_deref().is_some_and(|expected| expected != clock_id) {
            return Err(RenderError::ClockIdentityChanged);
        }
        if self.last_sequence.is_some_and(|last| sequence <= last) {
            return Err(RenderError::SequenceRegression);
        }
        if self
            .last_capture_timestamp_us
            .is_some_and(|last| capture_timestamp_us <= last)
        {
            return Err(RenderError::CaptureClockRegression);
        }
        if noticed_timestamp_us < capture_timestamp_us {
            return Err(RenderError::MissingEventBeforeCapture);
        }
        if self
            .last_delivery_timestamp_us
            .is_some_and(|last| noticed_timestamp_us < last)
        {
            return Err(RenderError::DeliveryClockRegression);
        }
        self.clock_id = Some(clock_id.to_string());
        self.last_sequence = Some(sequence);
        self.last_capture_timestamp_us = Some(capture_timestamp_us);
        self.last_delivery_timestamp_us = Some(noticed_timestamp_us);
        Ok(MissingObservationEvent {
            episode_id: episode_id.to_string(),
            agent_id: agent_id.to_string(),
            clock_id: clock_id.to_string(),
            tick,
            sequence,
            capture_timestamp_us,
            noticed_timestamp_us,
            reason,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ProjectedPoint {
    x: i64,
    y: i64,
    z_mm: i64,
}

pub struct SoftwareCameraRenderer;

impl SoftwareCameraRenderer {
    pub fn render(
        snapshot: &WorldRenderSnapshot,
        sequence: u64,
        clocks: RenderClocks,
    ) -> Result<RenderedObservation, RenderError> {
        if sequence == 0 {
            return Err(RenderError::InvalidObservationSequence);
        }
        validate_snapshot(snapshot)?;
        if clocks.capture_timestamp_us != snapshot.world_timestamp_us {
            return Err(RenderError::ClockCaptureMismatch);
        }
        if clocks.render_completed_timestamp_us < clocks.capture_timestamp_us {
            return Err(RenderError::RenderBeforeCapture);
        }
        if clocks.delivered_timestamp_us < clocks.render_completed_timestamp_us {
            return Err(RenderError::DeliveryBeforeRender);
        }

        let width = snapshot.camera.width as usize;
        let height = snapshot.camera.height as usize;
        let byte_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(3))
            .ok_or(RenderError::FrameAllocationExceeded)?;
        if byte_len > MAX_FRAME_BYTES {
            return Err(RenderError::FrameAllocationExceeded);
        }

        let mut pixels = vec![0_u8; byte_len];
        let mut z_buffer = vec![i64::MAX; width * height];
        fill_background(&mut pixels, width, height);
        draw_road_markings(snapshot, &mut pixels, &mut z_buffer)?;
        draw_actors(snapshot, &mut pixels, &mut z_buffer)?;

        let camera_digest = camera_calibration_digest(&snapshot.camera);
        let frame_digest = sha256_hex(&pixels);
        let action = snapshot.applied_action.as_ref();
        let mut receipt = PolicyObservationReceipt {
            schema_version: RENDERER_SCHEMA_VERSION,
            observation_id: String::new(),
            pixel_format: PixelFormat::Rgb8Srgb,
            episode_id: snapshot.episode_id.clone(),
            agent_id: snapshot.agent_id.clone(),
            clock_id: snapshot.clock_id.clone(),
            tick: snapshot.tick,
            sequence,
            applied_action_receipt_id: action.map(|a| a.receipt_id.to_string()),
            applied_action_receipt_sha256: action.map(|a| a.receipt_sha256.to_string()),
            applied_action_command_id: action.map(|a| a.command_id.to_string()),
            applied_action_tick: action.map(|a| a.applied_tick),
            camera_id: snapshot.camera.camera_id.clone(),
            camera_calibration_sha256: camera_digest,
            renderer_revision: RENDERER_REVISION.to_string(),
            capture_timestamp_us: clocks.capture_timestamp_us,
            render_completed_timestamp_us: clocks.render_completed_timestamp_us,
            delivered_timestamp_us: clocks.delivered_timestamp_us,
            frame_width: snapshot.camera.width,
            frame_height: snapshot.camera.height,
            channels: 3,
            frame_byte_len: byte_len,
            frame_sha256: frame_digest,
            receipt_sha256: String::new(),
        };
        receipt.observation_id = observation_identity_digest(&receipt);
        receipt.receipt_sha256 = policy_receipt_digest(&receipt);

        Ok(RenderedObservation {
            pixels_rgb: pixels,
            policy_receipt: receipt,
        })
    }

    /// Produce a separate evaluator-only artifact. This is intentionally not
    /// returned alongside the policy observation, preventing accidental
    /// privilege mixing in the normal camera-to-policy call path.
    pub fn audit_snapshot(
        snapshot: &WorldRenderSnapshot,
    ) -> Result<EvaluatorObservationAudit, RenderError> {
        validate_snapshot(snapshot)?;
        let mut actor_records = snapshot.actors.iter().collect::<Vec<_>>();
        actor_records.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));
        Ok(EvaluatorObservationAudit {
            episode_id: snapshot.episode_id.clone(),
            scenario_id: snapshot.scenario_id.clone(),
            agent_id: snapshot.agent_id.clone(),
            clock_id: snapshot.clock_id.clone(),
            tick: snapshot.tick,
            simulator_revision: snapshot.simulator_revision.clone(),
            world_snapshot_sha256: world_snapshot_digest(snapshot),
            road_ids: snapshot
                .road_markings
                .iter()
                .map(|road| road.road_id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            actor_ids: actor_records
                .iter()
                .map(|actor| actor.actor_id.clone())
                .collect(),
            actor_policy_revisions: actor_records
                .iter()
                .map(|actor| actor.actor_policy_revision.clone())
                .collect(),
            renderer_revision: RENDERER_REVISION.to_string(),
        })
    }
}

fn validate_snapshot(snapshot: &WorldRenderSnapshot) -> Result<(), RenderError> {
    for id in [
        snapshot.episode_id.as_str(),
        snapshot.scenario_id.as_str(),
        snapshot.agent_id.as_str(),
        snapshot.clock_id.as_str(),
        snapshot.simulator_revision.as_str(),
        snapshot.camera.camera_id.as_str(),
    ] {
        validate_id(id)?;
    }
    if snapshot.tick == 0 {
        if snapshot.applied_action.is_some() {
            return Err(RenderError::InitialActionMustBeAbsent);
        }
    } else {
        let action = snapshot.applied_action.as_ref().ok_or(RenderError::AppliedActionMissing)?;
        if action.receipt_id.trim().is_empty() || action.command_id.trim().is_empty() {
            return Err(RenderError::EmptyIdentity);
        }
        if !is_sha256_hex(&action.receipt_sha256) {
            return Err(RenderError::InvalidActionReceiptDigest);
        }
        if action.receipt_id.len() > MAX_ID_BYTES || action.command_id.len() > MAX_ID_BYTES {
            return Err(RenderError::IdentityTooLong);
        }
        if action.applied_tick != snapshot.tick {
            return Err(RenderError::AppliedActionTickMismatch);
        }
    }
    let camera = &snapshot.camera;
    if camera.width == 0
        || camera.height == 0
        || camera.width > MAX_FRAME_WIDTH
        || camera.height > MAX_FRAME_HEIGHT
    {
        return Err(RenderError::InvalidFrameGeometry);
    }
    let frame_bytes = (camera.width as usize)
        .checked_mul(camera.height as usize)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or(RenderError::FrameAllocationExceeded)?;
    if frame_bytes > MAX_FRAME_BYTES {
        return Err(RenderError::FrameAllocationExceeded);
    }
    if camera.focal_x_millipx == 0 || camera.focal_y_millipx == 0 {
        return Err(RenderError::InvalidFocalLength);
    }
    if camera.principal_x_millipx > camera.width.saturating_mul(1_000)
        || camera.principal_y_millipx > camera.height.saturating_mul(1_000)
    {
        return Err(RenderError::InvalidFocalLength);
    }
    if camera.near_plane_mm < MIN_NEAR_PLANE_MM
        || camera.far_plane_mm <= camera.near_plane_mm
        || camera.far_plane_mm > MAX_FAR_PLANE_MM
    {
        return Err(RenderError::InvalidNearFarPlane);
    }
    validate_rotation(camera.world_to_camera_q16)?;
    for coordinate in camera.position_world_mm {
        validate_coordinate(coordinate)?;
    }
    if snapshot.road_markings.len() > MAX_ROAD_MARKINGS {
        return Err(RenderError::TooManyRoadMarkings);
    }
    if snapshot.actors.len() > MAX_ACTORS {
        return Err(RenderError::TooManyActors);
    }

    let mut marking_ids = BTreeSet::new();
    let mut total_road_points = 0_usize;
    for road in &snapshot.road_markings {
        validate_id(&road.road_id)?;
        validate_id(&road.marking_id)?;
        if !marking_ids.insert(road.marking_id.clone()) {
            return Err(RenderError::DuplicateMarkingIdentity);
        }
        if road.points_world_mm.len() < 2 {
            return Err(RenderError::InvalidPolyline);
        }
        if road.points_world_mm.len() > MAX_POINTS_PER_MARKING {
            return Err(RenderError::TooManyRoadPoints);
        }
        total_road_points = total_road_points
            .checked_add(road.points_world_mm.len())
            .ok_or(RenderError::TooManyRoadPoints)?;
        if total_road_points > MAX_TOTAL_ROAD_POINTS {
            return Err(RenderError::TooManyRoadPoints);
        }
        for point in &road.points_world_mm {
            for coordinate in point {
                validate_coordinate(*coordinate)?;
            }
        }
    }
    let mut actor_ids = BTreeSet::new();
    for actor in &snapshot.actors {
        validate_id(&actor.actor_id)?;
        validate_id(&actor.actor_policy_revision)?;
        if !actor_ids.insert(actor.actor_id.clone()) {
            return Err(RenderError::DuplicateActorIdentity);
        }
        for coordinate in actor.center_world_mm {
            validate_coordinate(coordinate)?;
        }
        if actor.half_extents_mm.iter().any(|extent| *extent == 0 || *extent > 100_000) {
            return Err(RenderError::InvalidActorExtent);
        }
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_id(value: &str) -> Result<(), RenderError> {
    if value.trim().is_empty() {
        return Err(RenderError::EmptyIdentity);
    }
    if value.len() > MAX_ID_BYTES {
        return Err(RenderError::IdentityTooLong);
    }
    Ok(())
}

fn validate_coordinate(value: i64) -> Result<(), RenderError> {
    if value.unsigned_abs() > 1_000_000_000 {
        return Err(RenderError::InvalidWorldCoordinate);
    }
    Ok(())
}

fn validate_rotation(matrix: [[i32; 3]; 3]) -> Result<(), RenderError> {
    let target = i128::from(Q16_ONE) * i128::from(Q16_ONE);
    let tolerance = target / 10_000;
    for row in 0..3 {
        let norm = matrix[row]
            .iter()
            .map(|value| i128::from(*value) * i128::from(*value))
            .sum::<i128>();
        if (norm - target).abs() > tolerance {
            return Err(RenderError::InvalidCameraTransform);
        }
        for other in (row + 1)..3 {
            let dot = (0..3)
                .map(|column| i128::from(matrix[row][column]) * i128::from(matrix[other][column]))
                .sum::<i128>();
            if dot.abs() > tolerance {
                return Err(RenderError::InvalidCameraTransform);
            }
        }
    }
    let a = i128::from(matrix[0][0]);
    let b = i128::from(matrix[0][1]);
    let c = i128::from(matrix[0][2]);
    let d = i128::from(matrix[1][0]);
    let e = i128::from(matrix[1][1]);
    let f = i128::from(matrix[1][2]);
    let g = i128::from(matrix[2][0]);
    let h = i128::from(matrix[2][1]);
    let i = i128::from(matrix[2][2]);
    let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if determinant <= 0 {
        return Err(RenderError::InvalidCameraTransform);
    }
    Ok(())
}

fn fill_background(pixels: &mut [u8], width: usize, height: usize) {
    let horizon = height * 2 / 5;
    for y in 0..height {
        let color = if y < horizon { [108, 159, 202] } else { [72, 86, 74] };
        for x in 0..width {
            let offset = (y * width + x) * 3;
            pixels[offset..offset + 3].copy_from_slice(&color);
        }
    }
}

fn transform_point(camera: &CameraCalibration, world: [i64; 3]) -> [i64; 3] {
    let delta = [
        world[0] - camera.position_world_mm[0],
        world[1] - camera.position_world_mm[1],
        world[2] - camera.position_world_mm[2],
    ];
    let mut output = [0_i64; 3];
    for row in 0..3 {
        let sum = (0..3)
            .map(|column| i128::from(camera.world_to_camera_q16[row][column]) * i128::from(delta[column]))
            .sum::<i128>();
        output[row] = (sum / i128::from(Q16_ONE)) as i64;
    }
    output
}

fn clip_intersection_at_depth(from: [i64; 3], to: [i64; 3], depth: i64) -> [i64; 3] {
    let denominator = i128::from(to[2]) - i128::from(from[2]);
    let numerator = i128::from(depth) - i128::from(from[2]);
    let mut intersection = [0_i64; 3];
    for axis in 0..3 {
        let delta = i128::from(to[axis]) - i128::from(from[axis]);
        let value = i128::from(from[axis]) + delta * numerator / denominator;
        intersection[axis] = value as i64;
    }
    intersection[2] = depth;
    intersection
}

fn project_camera_point(
    camera: &CameraCalibration,
    point: [i64; 3],
) -> Option<ProjectedPoint> {
    let z = point[2];
    if z < i64::from(camera.near_plane_mm) || z > i64::from(camera.far_plane_mm) {
        return None;
    }
    let x_milli = i128::from(camera.principal_x_millipx)
        + i128::from(camera.focal_x_millipx) * i128::from(point[0]) / i128::from(z);
    let y_milli = i128::from(camera.principal_y_millipx)
        + i128::from(camera.focal_y_millipx) * i128::from(point[1]) / i128::from(z);
    let max_x = i128::from(camera.width) * 3_000;
    let max_y = i128::from(camera.height) * 3_000;
    if x_milli < -max_x || x_milli > max_x || y_milli < -max_y || y_milli > max_y {
        return None;
    }
    Some(ProjectedPoint {
        x: (x_milli / 1_000) as i64,
        y: (y_milli / 1_000) as i64,
        z_mm: z,
    })
}

/// Clip a world-space segment against the camera near/far planes before
/// perspective division. Segments crossing a clip plane remain visible rather
/// than disappearing because one endpoint lies outside the camera frustum.
fn project_segment(
    camera: &CameraCalibration,
    world_start: [i64; 3],
    world_end: [i64; 3],
) -> Option<(ProjectedPoint, ProjectedPoint)> {
    let mut start = transform_point(camera, world_start);
    let mut end = transform_point(camera, world_end);
    let near = i64::from(camera.near_plane_mm);
    let far = i64::from(camera.far_plane_mm);

    if (start[2] < near && end[2] < near) || (start[2] > far && end[2] > far) {
        return None;
    }
    if start[2] < near {
        start = clip_intersection_at_depth(start, end, near);
    }
    if end[2] < near {
        end = clip_intersection_at_depth(end, start, near);
    }
    if start[2] > far {
        start = clip_intersection_at_depth(start, end, far);
    }
    if end[2] > far {
        end = clip_intersection_at_depth(end, start, far);
    }

    Some((
        project_camera_point(camera, start)?,
        project_camera_point(camera, end)?,
    ))
}

fn draw_road_markings(
    snapshot: &WorldRenderSnapshot,
    pixels: &mut [u8],
    z_buffer: &mut [i64],
) -> Result<(), RenderError> {
    let width = snapshot.camera.width as usize;
    let height = snapshot.camera.height as usize;
    let mut roads = snapshot.road_markings.iter().collect::<Vec<_>>();
    roads.sort_by(|left, right| {
        left.road_id
            .cmp(&right.road_id)
            .then_with(|| left.marking_id.cmp(&right.marking_id))
    });

    for road in roads {
        let color = road.kind.color();
        for segment in road.points_world_mm.windows(2) {
            let Some((start, end)) =
                project_segment(&snapshot.camera, segment[0], segment[1])
            else {
                continue;
            };
            draw_line(pixels, z_buffer, width, height, start, end, color)?;
        }
    }
    Ok(())
}

fn draw_actors(
    snapshot: &WorldRenderSnapshot,
    pixels: &mut [u8],
    z_buffer: &mut [i64],
) -> Result<(), RenderError> {
    const EDGES: [[usize; 2]; 12] = [
        [0, 1],
        [0, 2],
        [0, 4],
        [1, 3],
        [1, 5],
        [2, 3],
        [2, 6],
        [3, 7],
        [4, 5],
        [4, 6],
        [5, 7],
        [6, 7],
    ];
    let width = snapshot.camera.width as usize;
    let height = snapshot.camera.height as usize;
    let mut actors = snapshot.actors.iter().collect::<Vec<_>>();
    actors.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));

    for actor in actors {
        let [x, y, z] = actor.center_world_mm;
        let [hx, hy, hz] = actor.half_extents_mm.map(i64::from);
        let corners = [
            [x - hx, y - hy, z - hz],
            [x + hx, y - hy, z - hz],
            [x - hx, y + hy, z - hz],
            [x + hx, y + hy, z - hz],
            [x - hx, y - hy, z + hz],
            [x + hx, y - hy, z + hz],
            [x - hx, y + hy, z + hz],
            [x + hx, y + hy, z + hz],
        ];
        for [start_index, end_index] in EDGES {
            let Some((start, end)) = project_segment(
                &snapshot.camera,
                corners[start_index],
                corners[end_index],
            ) else {
                continue;
            };
            draw_line(
                pixels,
                z_buffer,
                width,
                height,
                start,
                end,
                actor.class.color(),
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    pixels: &mut [u8],
    z_buffer: &mut [i64],
    width: usize,
    height: usize,
    start: ProjectedPoint,
    end: ProjectedPoint,
    color: [u8; 3],
) -> Result<(), RenderError> {
    let mut x0 = start.x;
    let mut y0 = start.y;
    let x1 = end.x;
    let y1 = end.y;
    let width_limit = width as i64;
    let height_limit = height as i64;

    if x0 < -width_limit * 2
        || x0 > width_limit * 3
        || x1 < -width_limit * 2
        || x1 > width_limit * 3
        || y0 < -height_limit * 2
        || y0 > height_limit * 3
        || y1 < -height_limit * 2
        || y1 > height_limit * 3
    {
        return Ok(());
    }

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    let steps = dx.max(-dy).min((width + height) as i64 * 6);

    if steps == 0 {
        plot_pixel(
            pixels,
            z_buffer,
            width,
            height,
            x0,
            y0,
            start.z_mm,
            color,
        );
        return Ok(());
    }

    let mut index = 0_i64;
    loop {
        let depth = start.z_mm + (end.z_mm - start.z_mm) * index / steps;
        plot_pixel(pixels, z_buffer, width, height, x0, y0, depth, color);
        if x0 == x1 && y0 == y1 {
            break;
        }

        let twice_error = 2 * error;
        if twice_error >= dy {
            error += dy;
            x0 += sx;
        }
        if twice_error <= dx {
            error += dx;
            y0 += sy;
        }
        index += 1;
        if index > steps {
            break;
        }
    }
    Ok(())
}

fn plot_pixel(
    pixels: &mut [u8],
    z_buffer: &mut [i64],
    width: usize,
    height: usize,
    x: i64,
    y: i64,
    depth: i64,
    color: [u8; 3],
) {
    if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
        return;
    }
    let index = y as usize * width + x as usize;
    if depth < z_buffer[index] {
        z_buffer[index] = depth;
        pixels[index * 3..index * 3 + 3].copy_from_slice(&color);
    }
}

fn camera_calibration_digest(camera: &CameraCalibration) -> String {
    let mut bytes = Vec::new();
    append_string(&mut bytes, &camera.camera_id);
    for value in camera.position_world_mm {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for row in camera.world_to_camera_q16 {
        for value in row {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [
        camera.focal_x_millipx,
        camera.focal_y_millipx,
        camera.principal_x_millipx,
        camera.principal_y_millipx,
        camera.width,
        camera.height,
        camera.near_plane_mm,
        camera.far_plane_mm,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn world_snapshot_digest(snapshot: &WorldRenderSnapshot) -> String {
    let mut bytes = Vec::new();
    for value in [
        &snapshot.episode_id,
        &snapshot.scenario_id,
        &snapshot.agent_id,
        &snapshot.clock_id,
        &snapshot.simulator_revision,
    ] {
        append_string(&mut bytes, value);
    }
    bytes.extend_from_slice(&snapshot.tick.to_le_bytes());
    bytes.extend_from_slice(&snapshot.world_timestamp_us.to_le_bytes());

    if let Some(action) = snapshot.applied_action.as_ref() {
        bytes.push(1);
        append_string(&mut bytes, &action.receipt_id);
        append_string(&mut bytes, &action.receipt_sha256);
        append_string(&mut bytes, &action.command_id);
        bytes.extend_from_slice(&action.applied_tick.to_le_bytes());
    } else {
        bytes.push(0);
    }
    append_string(
        &mut bytes,
        &camera_calibration_digest(&snapshot.camera),
    );

    let mut roads = snapshot.road_markings.iter().collect::<Vec<_>>();
    roads.sort_by(|left, right| {
        left.road_id
            .cmp(&right.road_id)
            .then_with(|| left.marking_id.cmp(&right.marking_id))
    });
    for road in roads {
        append_string(&mut bytes, &road.road_id);
        append_string(&mut bytes, &road.marking_id);
        bytes.push(road.kind as u8);
        bytes.extend_from_slice(&(road.points_world_mm.len() as u64).to_le_bytes());
        for point in &road.points_world_mm {
            for value in point {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }

    let mut actors = snapshot.actors.iter().collect::<Vec<_>>();
    actors.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));
    for actor in actors {
        append_string(&mut bytes, &actor.actor_id);
        append_string(&mut bytes, &actor.actor_policy_revision);
        bytes.push(actor.class as u8);
        for value in actor.center_world_mm {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in actor.half_extents_mm {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    sha256_hex(&bytes)
}

fn observation_identity_digest(receipt: &PolicyObservationReceipt) -> String {
    let mut bytes = Vec::new();
    append_string(&mut bytes, &receipt.episode_id);
    append_string(&mut bytes, &receipt.agent_id);
    append_string(&mut bytes, &receipt.clock_id);
    bytes.extend_from_slice(&receipt.tick.to_le_bytes());
    bytes.extend_from_slice(&receipt.sequence.to_le_bytes());
    append_string(
        &mut bytes,
        receipt.applied_action_receipt_id.as_deref().unwrap_or(""),
    );
    append_string(
        &mut bytes,
        receipt.applied_action_receipt_sha256.as_deref().unwrap_or(""),
    );
    append_string(&mut bytes, &receipt.frame_sha256);
    sha256_hex(&bytes)
}

fn policy_receipt_digest(receipt: &PolicyObservationReceipt) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&receipt.schema_version.to_le_bytes());
    append_string(&mut bytes, &receipt.observation_id);
    bytes.push(receipt.pixel_format as u8);
    for value in [&receipt.episode_id, &receipt.agent_id, &receipt.clock_id] {
        append_string(&mut bytes, value);
    }
    bytes.extend_from_slice(&receipt.tick.to_le_bytes());
    bytes.extend_from_slice(&receipt.sequence.to_le_bytes());
    for value in [
        receipt.applied_action_receipt_id.as_deref().unwrap_or(""),
        receipt.applied_action_receipt_sha256.as_deref().unwrap_or(""),
        receipt.applied_action_command_id.as_deref().unwrap_or(""),
        receipt.camera_id.as_str(),
        receipt.camera_calibration_sha256.as_str(),
        receipt.renderer_revision.as_str(),
        receipt.frame_sha256.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    bytes.push(u8::from(receipt.applied_action_tick.is_some()));
    bytes.extend_from_slice(&receipt.applied_action_tick.unwrap_or(0).to_le_bytes());
    for value in [
        receipt.capture_timestamp_us,
        receipt.render_completed_timestamp_us,
        receipt.delivered_timestamp_us,
        u64::from(receipt.frame_width),
        u64::from(receipt.frame_height),
        u64::from(receipt.channels),
        receipt.frame_byte_len as u64,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut data = input.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    let mut hash = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];

    for chunk in data.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        for index in 0..16 {
            let offset = index * 4;
            schedule[index] = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let sigma_zero = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let sigma_one = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(sigma_zero)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(sigma_one);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for index in 0..64 {
            let upper_sigma_one = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let temp_one = h
                .wrapping_add(upper_sigma_one)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);

            let upper_sigma_zero = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp_two = upper_sigma_zero.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp_one);
            d = c;
            c = b;
            b = a;
            a = temp_one.wrapping_add(temp_two);
        }

        hash[0] = hash[0].wrapping_add(a);
        hash[1] = hash[1].wrapping_add(b);
        hash[2] = hash[2].wrapping_add(c);
        hash[3] = hash[3].wrapping_add(d);
        hash[4] = hash[4].wrapping_add(e);
        hash[5] = hash[5].wrapping_add(f);
        hash[6] = hash[6].wrapping_add(g);
        hash[7] = hash[7].wrapping_add(h);
    }

    let mut output = String::with_capacity(64);
    for word in hash {
        let _ = write!(&mut output, "{word:08x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> CameraCalibration {
        CameraCalibration {
            camera_id: "front".into(),
            position_world_mm: [0, 0, 1_500],
            world_to_camera_q16: [
                [Q16_ONE as i32, 0, 0],
                [0, 0, -(Q16_ONE as i32)],
                [0, Q16_ONE as i32, 0],
            ],
            focal_x_millipx: 450_000,
            focal_y_millipx: 450_000,
            principal_x_millipx: 320_000,
            principal_y_millipx: 180_000,
            width: 640,
            height: 360,
            near_plane_mm: 100,
            far_plane_mm: 200_000,
        }
    }

    fn snapshot() -> WorldRenderSnapshot {
        WorldRenderSnapshot {
            episode_id: "episode-1".into(),
            scenario_id: "straight-road".into(),
            agent_id: "ego".into(),
            tick: 1,
            world_timestamp_us: 20_000,
            clock_id: "sim-monotonic-0".into(),
            simulator_revision: "sim-fixture-v1".into(),
            applied_action: Some(AppliedActionRef {
                receipt_id: "action-receipt-1".into(),
                receipt_sha256: "a".repeat(64),
                command_id: "command-1".into(),
                applied_tick: 1,
            }),
            camera: camera(),
            road_markings: vec![
                RoadPolyline {
                    road_id: "road-1".into(),
                    marking_id: "center-left".into(),
                    kind: RoadMarkingKind::LaneWhite,
                    points_world_mm: vec![
                        [-2_000, 10_000, 0],
                        [-2_000, 20_000, 0],
                        [-2_000, 40_000, 0],
                    ],
                },
                RoadPolyline {
                    road_id: "road-1".into(),
                    marking_id: "center-right".into(),
                    kind: RoadMarkingKind::LaneYellow,
                    points_world_mm: vec![
                        [2_000, 10_000, 0],
                        [2_000, 20_000, 0],
                        [2_000, 40_000, 0],
                    ],
                },
            ],
            actors: vec![
                ActorCuboid {
                    actor_id: "lead".into(),
                    actor_policy_revision: "actor-policy-v1".into(),
                    class: ActorClass::Car,
                    center_world_mm: [0, 15_000, 700],
                    half_extents_mm: [900, 2_000, 700],
                },
                ActorCuboid {
                    actor_id: "pedestrian-1".into(),
                    actor_policy_revision: "pedestrian-policy-v1".into(),
                    class: ActorClass::Pedestrian,
                    center_world_mm: [2_000, 22_000, 900],
                    half_extents_mm: [350, 350, 900],
                },
            ],
        }
    }

    fn clocks() -> RenderClocks {
        RenderClocks {
            capture_timestamp_us: 20_000,
            render_completed_timestamp_us: 20_800,
            delivered_timestamp_us: 21_000,
        }
    }

    fn timing_policy() -> TimingPolicy {
        TimingPolicy {
            maximum_render_latency_us: 1_000,
            maximum_delivery_age_us: 3_000,
        }
    }

    #[test]
    fn policy_receipt_json_round_trip_is_policy_scoped() {
        let rendered = SoftwareCameraRenderer::render(&snapshot(), 1, clocks()).unwrap();
        let value = serde_json::to_value(&rendered.policy_receipt).unwrap();
        assert!(value.get("scenario_id").is_none());
        assert!(value.get("simulator_revision").is_none());
        assert!(value.get("world_snapshot_sha256").is_none());
        assert_eq!(value["pixel_format"], "rgb8_srgb");
        let decoded: PolicyObservationReceipt = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, rendered.policy_receipt);
    }

    #[test]
    fn evaluator_audit_serializes_as_a_separate_record() {
        let audit = SoftwareCameraRenderer::audit_snapshot(&snapshot()).unwrap();
        let value = serde_json::to_value(&audit).unwrap();
        assert_eq!(value["scenario_id"], "straight-road");
        assert!(value.get("frame_sha256").is_none());
        let decoded: EvaluatorObservationAudit = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, audit);
    }

    #[test]
    fn missing_frame_event_has_stable_snake_case_json() {
        let mut guard = ObservationStreamGuard::new(timing_policy());
        let event = guard
            .record_missing(
                "episode-1",
                "ego",
                "clock-1",
                2,
                2,
                40_000,
                40_500,
                MissingFrameReason::CameraDrop,
            )
            .unwrap();
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["reason"], "camera_drop");
    }

    #[test]
    fn versioned_json_schemas_are_parseable_and_have_required_fields() {
        let policy: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/policy-observation-receipt-v1.schema.json"
        ))
        .unwrap();
        let audit: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/evaluator-observation-audit-v1.schema.json"
        ))
        .unwrap();
        let missing: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/missing-observation-event-v1.schema.json"
        ))
        .unwrap();

        assert_eq!(policy["properties"]["schema_version"]["const"], 1);
        assert!(policy["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "frame_sha256"));
        assert!(policy["properties"].get("world_snapshot_sha256").is_none());
        assert!(audit["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "world_snapshot_sha256"));
        assert_eq!(missing["properties"]["reason"]["enum"][0], "camera_drop");
    }

    #[test]
    fn sha256_empty_known_answer() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_multiblock_nist_known_answer() {
        let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            sha256_hex(input),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_abc_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn renders_deterministic_rgb_frame_and_receipt() {
        let first = SoftwareCameraRenderer::render(&snapshot(), 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&snapshot(), 1, clocks()).unwrap();
        assert_eq!(first.pixels_rgb, second.pixels_rgb);
        assert_eq!(
            first.policy_receipt.frame_sha256,
            second.policy_receipt.frame_sha256
        );
        assert_eq!(
            first.policy_receipt.observation_id,
            second.policy_receipt.observation_id
        );
        assert_eq!(
            first.policy_receipt.receipt_sha256,
            second.policy_receipt.receipt_sha256
        );
        assert_eq!(first.policy_receipt.pixel_format, PixelFormat::Rgb8Srgb);
        assert_eq!(first.policy_receipt.channels, 3);
    }

    #[test]
    fn frame_contains_road_and_actor_geometry() {
        let result = SoftwareCameraRenderer::render(&snapshot(), 1, clocks()).unwrap();
        assert!(result
            .pixels_rgb
            .chunks_exact(3)
            .any(|pixel| pixel == [236, 236, 224]));
        assert!(result
            .pixels_rgb
            .chunks_exact(3)
            .any(|pixel| pixel == [214, 72, 58]));
        assert!(result
            .pixels_rgb
            .chunks_exact(3)
            .any(|pixel| pixel == [65, 134, 226]));
    }

    #[test]
    fn camera_pose_change_changes_frame() {
        let first_snapshot = snapshot();
        let mut second_snapshot = snapshot();
        second_snapshot.camera.position_world_mm[0] = 250;
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&second_snapshot, 1, clocks()).unwrap();
        assert_ne!(
            first.policy_receipt.frame_sha256,
            second.policy_receipt.frame_sha256
        );
    }

    #[test]
    fn intrinsics_change_changes_calibration_and_frame() {
        let first_snapshot = snapshot();
        let mut second_snapshot = snapshot();
        second_snapshot.camera.focal_x_millipx = 480_000;
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&second_snapshot, 1, clocks()).unwrap();
        assert_ne!(
            first.policy_receipt.camera_calibration_sha256,
            second.policy_receipt.camera_calibration_sha256
        );
        assert_ne!(
            first.policy_receipt.frame_sha256,
            second.policy_receipt.frame_sha256
        );
    }

    #[test]
    fn actor_pose_change_changes_evaluator_digest_and_pixels() {
        let first_snapshot = snapshot();
        let mut second_snapshot = snapshot();
        second_snapshot.actors[0].center_world_mm[0] = 1_000;
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&second_snapshot, 1, clocks()).unwrap();
        let first_audit = SoftwareCameraRenderer::audit_snapshot(&first_snapshot).unwrap();
        let second_audit = SoftwareCameraRenderer::audit_snapshot(&second_snapshot).unwrap();
        assert_ne!(
            first_audit.world_snapshot_sha256,
            second_audit.world_snapshot_sha256
        );
        assert_ne!(
            first.policy_receipt.frame_sha256,
            second.policy_receipt.frame_sha256
        );
    }

    #[test]
    fn curved_road_polyline_changes_projected_observation() {
        let straight = snapshot();
        let mut curved = snapshot();
        curved.road_markings[0].points_world_mm = vec![
            [-2_000, 5_000, 0],
            [-1_900, 10_000, 0],
            [-1_200, 15_000, 0],
            [-200, 20_000, 0],
            [900, 25_000, 0],
        ];
        let straight_render = SoftwareCameraRenderer::render(&straight, 1, clocks()).unwrap();
        let curved_render = SoftwareCameraRenderer::render(&curved, 1, clocks()).unwrap();
        assert_ne!(
            straight_render.policy_receipt.frame_sha256,
            curved_render.policy_receipt.frame_sha256
        );
    }

    #[test]
    fn total_road_point_budget_is_enforced() {
        let mut world = snapshot();
        world.road_markings.clear();
        let points = vec![[0, 10_000, 0], [0, 11_000, 0]];
        let per_marking = MAX_POINTS_PER_MARKING;
        let mut remaining = MAX_TOTAL_ROAD_POINTS + 2;
        let mut index = 0_u32;
        while remaining > 0 {
            let count = remaining.min(per_marking);
            let mut polyline_points = vec![[0, 10_000, 0]; count];
            polyline_points[0] = points[0];
            polyline_points[count - 1] = points[1];
            world.road_markings.push(RoadPolyline {
                road_id: format!("road-{index}"),
                marking_id: format!("marking-{index}"),
                kind: RoadMarkingKind::LaneWhite,
                points_world_mm: polyline_points,
            });
            remaining -= count;
            index += 1;
        }
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::TooManyRoadPoints)
        );
    }

    #[test]
    fn road_segment_crossing_near_plane_is_clipped_and_rendered() {
        let mut world = snapshot();
        world.road_markings = vec![RoadPolyline {
            road_id: "near-road".into(),
            marking_id: "near-line".into(),
            kind: RoadMarkingKind::LaneWhite,
            points_world_mm: vec![[-100, 50, 1_400], [-100, 1_000, 1_400]],
        }];
        let rendered = SoftwareCameraRenderer::render(&world, 1, clocks()).unwrap();
        assert!(rendered
            .pixels_rgb
            .chunks_exact(3)
            .any(|pixel| pixel == [236, 236, 224]));
    }

    #[test]
    fn road_geometry_change_changes_pixels() {
        let first_snapshot = snapshot();
        let mut second_snapshot = snapshot();
        second_snapshot.road_markings[0].points_world_mm[0][0] -= 1_000;
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&second_snapshot, 1, clocks()).unwrap();
        assert_ne!(
            first.policy_receipt.frame_sha256,
            second.policy_receipt.frame_sha256
        );
    }

    #[test]
    fn reordered_road_and_actor_vectors_preserve_frame_and_receipts() {
        let first_snapshot = snapshot();
        let mut reordered = snapshot();
        reordered.road_markings.reverse();
        reordered.actors.reverse();
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&reordered, 1, clocks()).unwrap();
        assert_eq!(first.pixels_rgb, second.pixels_rgb);
        assert_eq!(first.policy_receipt, second.policy_receipt);
        assert_eq!(
            SoftwareCameraRenderer::audit_snapshot(&first_snapshot).unwrap(),
            SoftwareCameraRenderer::audit_snapshot(&reordered).unwrap()
        );
    }

    #[test]
    fn changed_applied_action_identity_changes_receipt_not_image() {
        let first_snapshot = snapshot();
        let mut second_snapshot = snapshot();
        second_snapshot.applied_action = Some(AppliedActionRef {
            receipt_id: "different-action".into(),
            receipt_sha256: "a".repeat(64),
            command_id: "command-1".into(),
            applied_tick: 1,
        });
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&second_snapshot, 1, clocks()).unwrap();
        assert_eq!(
            first.policy_receipt.frame_sha256,
            second.policy_receipt.frame_sha256
        );
        assert_ne!(
            first.policy_receipt.observation_id,
            second.policy_receipt.observation_id
        );
        assert_ne!(
            first.policy_receipt.receipt_sha256,
            second.policy_receipt.receipt_sha256
        );
    }

    #[test]
    fn uppercase_action_receipt_digest_is_rejected_as_noncanonical() {
        let mut world = snapshot();
        world.applied_action.as_mut().unwrap().receipt_sha256 = "A".repeat(64);
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::InvalidActionReceiptDigest)
        );
    }

    #[test]
    fn malformed_applied_action_receipt_digest_is_rejected() {
        let mut world = snapshot();
        world.applied_action.as_mut().unwrap().receipt_sha256 = "not-a-sha256".into();
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::InvalidActionReceiptDigest)
        );
    }

    #[test]
    fn applied_action_receipt_digest_is_bound_to_observation_identity() {
        let first_snapshot = snapshot();
        let mut second_snapshot = snapshot();
        second_snapshot
            .applied_action
            .as_mut()
            .unwrap()
            .receipt_sha256 = "b".repeat(64);
        let first = SoftwareCameraRenderer::render(&first_snapshot, 1, clocks()).unwrap();
        let second = SoftwareCameraRenderer::render(&second_snapshot, 1, clocks()).unwrap();
        assert_eq!(first.policy_receipt.frame_sha256, second.policy_receipt.frame_sha256);
        assert_ne!(first.policy_receipt.observation_id, second.policy_receipt.observation_id);
        assert_ne!(first.policy_receipt.receipt_sha256, second.policy_receipt.receipt_sha256);
    }

    #[test]
    fn observation_sequence_zero_is_rejected() {
        assert_eq!(
            SoftwareCameraRenderer::render(&snapshot(), 0, clocks()).err(),
            Some(RenderError::InvalidObservationSequence)
        );
    }

    #[test]
    fn applied_action_tick_must_match_world_tick() {
        let mut world = snapshot();
        world.applied_action = Some(AppliedActionRef {
            receipt_id: "r".into(),
            receipt_sha256: "b".repeat(64),
            command_id: "c".into(),
            applied_tick: 0,
        });
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::AppliedActionTickMismatch)
        );
    }

    #[test]
    fn initial_tick_has_no_applied_action() {
        let mut world = snapshot();
        world.tick = 0;
        world.applied_action = None;
        assert!(SoftwareCameraRenderer::render(&world, 1, clocks()).is_ok());
    }

    #[test]
    fn future_action_receipt_is_rejected() {
        let mut world = snapshot();
        world.applied_action = Some(AppliedActionRef {
            receipt_id: "r".into(),
            command_id: "c".into(),
            applied_tick: 2,
        });
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::AppliedActionTickMismatch)
        );
    }

    #[test]
    fn actor_and_road_truth_are_only_in_evaluator_audit() {
        let result = SoftwareCameraRenderer::render(&snapshot(), 1, clocks()).unwrap();
        let policy_debug = format!("{:?}", result.policy_receipt);
        assert!(!policy_debug.contains("lead"));
        assert!(!policy_debug.contains("road-1"));
        let audit = SoftwareCameraRenderer::audit_snapshot(&snapshot()).unwrap();
        assert_eq!(audit.actor_ids, vec!["lead", "pedestrian-1"]);
        assert_eq!(audit.road_ids, vec!["road-1"]);
    }

    #[test]
    fn malformed_rotation_is_rejected() {
        let mut world = snapshot();
        world.camera.world_to_camera_q16[0] = [0, 0, 0];
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::InvalidCameraTransform)
        );
    }

    #[test]
    fn reflected_rotation_is_rejected() {
        let mut world = snapshot();
        world.camera.world_to_camera_q16[0][0] = -(Q16_ONE as i32);
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::InvalidCameraTransform)
        );
    }

    #[test]
    fn oversize_frame_is_rejected_before_allocation() {
        let mut world = snapshot();
        world.camera.width = MAX_FRAME_WIDTH + 1;
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::InvalidFrameGeometry)
        );
    }

    #[test]
    fn duplicate_actor_identity_is_rejected() {
        let mut world = snapshot();
        world.actors.push(world.actors[0].clone());
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::DuplicateActorIdentity)
        );
    }

    #[test]
    fn duplicate_road_marking_identity_is_rejected() {
        let mut world = snapshot();
        world.road_markings.push(world.road_markings[0].clone());
        assert_eq!(
            SoftwareCameraRenderer::render(&world, 1, clocks()).err(),
            Some(RenderError::DuplicateMarkingIdentity)
        );
    }

    #[test]
    fn capture_clock_must_match_snapshot_clock() {
        let mut render_clocks = clocks();
        render_clocks.capture_timestamp_us += 1;
        assert_eq!(
            SoftwareCameraRenderer::render(&snapshot(), 1, render_clocks).err(),
            Some(RenderError::ClockCaptureMismatch)
        );
    }

    #[test]
    fn render_cannot_precede_capture() {
        let mut render_clocks = clocks();
        render_clocks.render_completed_timestamp_us = 19_000;
        assert_eq!(
            SoftwareCameraRenderer::render(&snapshot(), 1, render_clocks).err(),
            Some(RenderError::RenderBeforeCapture)
        );
    }

    #[test]
    fn delivery_cannot_precede_render() {
        let mut render_clocks = clocks();
        render_clocks.delivered_timestamp_us = 20_700;
        assert_eq!(
            SoftwareCameraRenderer::render(&snapshot(), 1, render_clocks).err(),
            Some(RenderError::DeliveryBeforeRender)
        );
    }

    #[test]
    fn stream_guard_distinguishes_fresh_delayed_and_stale() {
        let mut guard = ObservationStreamGuard::new(timing_policy());
        assert_eq!(
            guard.admit("clock-1", 1, 1_000, 1_500, 2_000),
            Ok(ObservationDisposition::Fresh)
        );
        assert_eq!(
            guard.admit("clock-1", 2, 2_000, 3_500, 4_000),
            Ok(ObservationDisposition::Delayed)
        );
        assert_eq!(
            guard.admit("clock-1", 3, 3_000, 4_000, 7_000),
            Ok(ObservationDisposition::Stale)
        );
    }

    #[test]
    fn stream_guard_rejects_duplicate_sequence_without_mutation() {
        let mut guard = ObservationStreamGuard::new(timing_policy());
        assert!(guard.admit("clock-1", 2, 2_000, 2_500, 3_000).is_ok());
        assert_eq!(
            guard.admit("clock-1", 2, 3_000, 3_100, 3_200),
            Err(RenderError::SequenceRegression)
        );
        assert!(guard.admit("clock-1", 3, 3_000, 3_100, 3_200).is_ok());
    }

    #[test]
    fn stream_guard_rejects_capture_and_delivery_clock_regression() {
        let mut guard = ObservationStreamGuard::new(timing_policy());
        guard.admit("clock-1", 1, 2_000, 2_200, 2_500).unwrap();
        assert_eq!(
            guard.admit("clock-1", 2, 1_900, 2_200, 2_600),
            Err(RenderError::CaptureClockRegression)
        );
        assert_eq!(
            guard.admit("clock-1", 2, 3_000, 3_200, 2_400),
            Err(RenderError::DeliveryClockRegression)
        );
    }

    #[test]
    fn stream_guard_rejects_clock_identity_switch_without_mutation() {
        let mut guard = ObservationStreamGuard::new(timing_policy());
        assert!(guard.admit("sim-clock-a", 1, 1_000, 1_200, 1_500).is_ok());
        assert_eq!(
            guard.admit("sim-clock-b", 2, 2_000, 2_200, 2_500),
            Err(RenderError::ClockIdentityChanged)
        );
        assert_eq!(
            guard.admit("sim-clock-a", 2, 2_000, 2_200, 2_500),
            Ok(ObservationDisposition::Fresh)
        );
    }

    #[test]
    fn missing_frame_event_is_explicit_and_consumes_sequence() {
        let mut guard = ObservationStreamGuard::new(timing_policy());
        let event = guard
            .record_missing(
                "episode-1",
                "ego",
                "clock-1",
                2,
                2,
                40_000,
                40_500,
                MissingFrameReason::CameraDrop,
            )
            .unwrap();
        assert_eq!(event.reason, MissingFrameReason::CameraDrop);
        assert_eq!(event.tick, 2);
        assert_eq!(event.sequence, 2);
        assert_eq!(
            guard.admit("clock-1", 2, 50_000, 50_500, 50_800),
            Err(RenderError::SequenceRegression)
        );
        assert_eq!(
            guard.admit("clock-1", 3, 50_000, 50_500, 50_800),
            Ok(ObservationDisposition::Fresh)
        );
    }

    #[test]
    fn policy_receipt_does_not_include_evaluator_world_digest() {
        let result = SoftwareCameraRenderer::render(&snapshot(), 1, clocks()).unwrap();
        let policy_debug = format!("{:?}", result.policy_receipt);
        let audit = SoftwareCameraRenderer::audit_snapshot(&snapshot()).unwrap();
        assert!(!policy_debug.contains(&audit.world_snapshot_sha256));
    }
}

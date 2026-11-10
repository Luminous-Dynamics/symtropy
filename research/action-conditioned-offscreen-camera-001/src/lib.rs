//! Deterministic offscreen perspective camera from immutable world snapshots.
//!
//! The crate owns rendering and observation receipt boundaries only. It does not
//! advance vehicle dynamics, run actor policies, choose actions, or certify safety.

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
pub const MAX_ACTORS: usize = 128;
pub const MIN_NEAR_PLANE_MM: u32 = 100;
pub const MAX_FAR_PLANE_MM: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedActionRef {
    pub receipt_id: &'static str,
    pub command_id: &'static str,
    pub applied_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadPolyline {
    pub road_id: String,
    pub marking_id: String,
    pub kind: RoadMarkingKind,
    pub points_world_mm: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorCuboid {
    pub actor_id: String,
    pub actor_policy_revision: String,
    pub class: ActorClass,
    pub center_world_mm: [i64; 3],
    pub half_extents_mm: [u32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraCalibration {
    pub camera_id: String,
    pub position_world_mm: [i64; 3],
    /// World-to-camera rotation matrix, signed Q16.16, camera axes [right, up, forward].
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRenderSnapshot {
    pub episode_id: String,
    pub scenario_id: String,
    pub agent_id: String,
    pub tick: u64,
    pub world_timestamp_us: u64,
    pub simulator_revision: String,
    /// None is permitted only for the initial tick (tick 0).
    pub applied_action: Option<AppliedActionRef>,
    pub camera: CameraCalibration,
    pub road_markings: Vec<RoadPolyline>,
    pub actors: Vec<ActorCuboid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderClocks {
    pub capture_timestamp_us: u64,
    pub render_completed_timestamp_us: u64,
    pub delivered_timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingPolicy {
    pub maximum_render_latency_us: u64,
    pub maximum_delivery_age_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationDisposition {
    Fresh,
    Delayed,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingFrameReason {
    CameraDrop,
    CameraUnavailable,
    RendererOverrun,
    TransportDrop,
    MalformedCapture,
    Censored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingObservationEvent {
    pub episode_id: String,
    pub agent_id: String,
    pub tick: u64,
    pub sequence: u64,
    pub capture_timestamp_us: u64,
    pub reason: MissingFrameReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    EmptyIdentity,
    IdentityTooLong,
    InitialActionMustBeAbsent,
    AppliedActionMissing,
    AppliedActionTickMismatch,
    InvalidFrameGeometry,
    InvalidFocalLength,
    InvalidNearFarPlane,
    InvalidCameraTransform,
    TooManyRoadMarkings,
    TooManyRoadPoints,
    TooManyActors,
    DuplicateRoadIdentity,
    DuplicateActorIdentity,
    InvalidWorldCoordinate,
    InvalidActorExtent,
    InvalidPolyline,
    ClockCaptureMismatch,
    RenderBeforeCapture,
    DeliveryBeforeRender,
    SequenceRegression,
    CaptureClockRegression,
    DeliveryClockRegression,
    SnapshotIdentityMismatch,
    FrameAllocationExceeded,
    ProjectionOverflow,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RenderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyObservationReceipt {
    pub schema_version: u32,
    pub episode_id: String,
    pub scenario_id: String,
    pub agent_id: String,
    pub tick: u64,
    pub sequence: u64,
    pub applied_action_receipt_id: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorObservationAudit {
    pub episode_id: String,
    pub scenario_id: String,
    pub agent_id: String,
    pub tick: u64,
    pub simulator_revision: String,
    pub world_snapshot_sha256: String,
    pub road_ids: Vec<String>,
    pub actor_ids: Vec<String>,
    pub actor_policy_revisions: Vec<String>,
    pub renderer_revision: String,
}

pub struct RenderedObservation {
    pub pixels_rgb: Vec<u8>,
    pub policy_receipt: PolicyObservationReceipt,
    /// Evaluator-only. Do not expose this record to the policy.
    pub evaluator_audit: EvaluatorObservationAudit,
}

pub struct ObservationStreamGuard {
    timing: TimingPolicy,
    last_sequence: Option<u64>,
    last_capture_timestamp_us: Option<u64>,
    last_delivery_timestamp_us: Option<u64>,
}

impl ObservationStreamGuard {
    pub fn new(timing: TimingPolicy) -> Self {
        Self {
            timing,
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
        sequence: u64,
        capture_timestamp_us: u64,
        render_completed_timestamp_us: u64,
        delivered_timestamp_us: u64,
    ) -> Result<ObservationDisposition, RenderError> {
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
        self.last_sequence = Some(sequence);
        self.last_capture_timestamp_us = Some(capture_timestamp_us);
        self.last_delivery_timestamp_us = Some(delivered_timestamp_us);
        Ok(disposition)
    }

    pub fn missing_event(
        &self,
        episode_id: &str,
        agent_id: &str,
        tick: u64,
        sequence: u64,
        capture_timestamp_us: u64,
        reason: MissingFrameReason,
    ) -> Result<MissingObservationEvent, RenderError> {
        validate_id(episode_id)?;
        validate_id(agent_id)?;
        Ok(MissingObservationEvent {
            episode_id: episode_id.to_string(),
            agent_id: agent_id.to_string(),
            tick,
            sequence,
            capture_timestamp_us,
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
        let action = snapshot.applied_action;
        let mut receipt = PolicyObservationReceipt {
            schema_version: RENDERER_SCHEMA_VERSION,
            episode_id: snapshot.episode_id.clone(),
            scenario_id: snapshot.scenario_id.clone(),
            agent_id: snapshot.agent_id.clone(),
            tick: snapshot.tick,
            sequence,
            applied_action_receipt_id: action.map(|a| a.receipt_id.to_string()),
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
        receipt.receipt_sha256 = policy_receipt_digest(&receipt);

        let evaluator_audit = EvaluatorObservationAudit {
            episode_id: snapshot.episode_id.clone(),
            scenario_id: snapshot.scenario_id.clone(),
            agent_id: snapshot.agent_id.clone(),
            tick: snapshot.tick,
            simulator_revision: snapshot.simulator_revision.clone(),
            world_snapshot_sha256: world_snapshot_digest(snapshot),
            road_ids: snapshot.road_markings.iter().map(|road| road.road_id.clone()).collect(),
            actor_ids: snapshot.actors.iter().map(|actor| actor.actor_id.clone()).collect(),
            actor_policy_revisions: snapshot.actors.iter().map(|actor| actor.actor_policy_revision.clone()).collect(),
            renderer_revision: RENDERER_REVISION.to_string(),
        };
        Ok(RenderedObservation {
            pixels_rgb: pixels,
            policy_receipt: receipt,
            evaluator_audit,
        })
    }
}

fn validate_snapshot(snapshot: &WorldRenderSnapshot) -> Result<(), RenderError> {
    for id in [
        snapshot.episode_id.as_str(),
        snapshot.scenario_id.as_str(),
        snapshot.agent_id.as_str(),
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
        let action = snapshot.applied_action.ok_or(RenderError::AppliedActionMissing)?;
        if action.receipt_id.trim().is_empty() || action.command_id.trim().is_empty() {
            return Err(RenderError::EmptyIdentity);
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

    let mut road_ids = BTreeSet::new();
    let mut marking_ids = BTreeSet::new();
    for road in &snapshot.road_markings {
        validate_id(&road.road_id)?;
        validate_id(&road.marking_id)?;
        if !road_ids.insert(road.road_id.clone()) || !marking_ids.insert(road.marking_id.clone()) {
            return Err(RenderError::DuplicateRoadIdentity);
        }
        if road.points_world_mm.len() < 2 {
            return Err(RenderError::InvalidPolyline);
        }
        if road.points_world_mm.len() > MAX_POINTS_PER_MARKING {
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
    if value.abs() > 1_000_000_000 {
        return Err(RenderError::InvalidWorldCoordinate);
    }
    Ok(())
}

fn validate_rotation(matrix: [[i32; 3]; 3]) -> Result<(), RenderError> {
    let target = i128::from(Q16_ONE) * i128::from(Q16_ONE);
    let tolerance = target / 100;
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

fn project_point(
    camera: &CameraCalibration,
    world: [i64; 3],
) -> Result<Option<ProjectedPoint>, RenderError> {
    let point = transform_point(camera, world);
    let z = point[2];
    if z < i64::from(camera.near_plane_mm) || z > i64::from(camera.far_plane_mm) {
        return Ok(None);
    }
    let x_milli = i128::from(camera.principal_x_millipx)
        + i128::from(camera.focal_x_millipx) * i128::from(point[0]) / i128::from(z);
    let y_milli = i128::from(camera.principal_y_millipx)
        - i128::from(camera.focal_y_millipx) * i128::from(point[1]) / i128::from(z);
    let max_x = i128::from(camera.width) * 3_000;
    let max_y = i128::from(camera.height) * 3_000;
    if x_milli < -max_x || x_milli > max_x || y_milli < -max_y || y_milli > max_y {
        return Ok(None);
    }
    Ok(Some(ProjectedPoint {
        x: (x_milli / 1_000) as i64,
        y: (y_milli / 1_000) as i64,
        z_mm: z,
    }))
}

fn draw_road_markings(
    snapshot: &WorldRenderSnapshot,
    pixels: &mut [u8],
    z_buffer: &mut [i64],
) -> Result<(), RenderError> {
    let width = snapshot.camera.width as usize;
    let height = snapshot.camera.height as usize;
    for road in &snapshot.road_markings {
        let color = road.kind.color();
        for segment in road.points_world_mm.windows(2) {
            let Some(start) = project_point(&snapshot.camera, segment[0])? else { continue };
            let Some(end) = project_point(&snapshot.camera, segment[1])? else { continue };
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
        [0,1],[0,2],[0,4],[1,3],[1,5],[2,3],[2,6],[3,7],[4,5],[4,6],[5,7],[6,7],
    ];
    let width = snapshot.camera.width as usize;
    let height = snapshot.camera.height as usize;
    for actor in &snapshot.actors {
        let [x, y, z] = actor.center_world_mm;
        let [hx, hy, hz] = actor.half_extents_mm.map(i64::from);
        let corners = [
            [x-hx,y-hy,z-hz],
            [x+hx,y-hy,z-hz],
            [x-hx,y+hy,z-hz],
            [x+hx,y+hy,z-hz],
            [x-hx,y-hy,z+hz],
            [x+hx,y-hy,z+hz],
            [x-hx,y+hy,z+hz],
            [x+hx,y+hy,z+hz],
        ];
        let projected = corners.map(|corner| project_point(&snapshot.camera, corner));
        for [a,b] in EDGES {
            let (Some(Ok(Some(start))),Some(Ok(Some(end)))) = (projected[a],projected[b]) else {
                continue;
            };
            draw_line(pixels,z_buffer,width,height,start,end,actor.class.color())?;
        }
    }
    Ok(())
}

fn draw_line(
    pixels: &mut [u8],
    z_buffer: &mut [i64],
    width: usize,
    height: usize,
    start: ProjectedPoint,
    end: ProjectedPoint,
    color: [u8;3],
) -> Result<(), RenderError> {
    let mut x0=start.x;
    let mut y0=start.y;
    let x1=end.x;
    let y1=end.y;
    if x0 < -(width as i64)*2 || x0 > width as i64*3 || x1 < -(width as i64)*2 || x1 > width as i64*3
        || y0 < -(height as i64)*2 || y0 > height as i64*3 || y1 < -(height as i64)*2 || y1 > height as i64*3 {
        return Ok(());
    }
    let dx=(x1-x0).abs();
    let sx=if x0<x1 {1} else {-1};
    let dy=-(y1-y0).abs();
    let sy=if y0<y1 {1} else {-1};
    let mut error=dx+dy;
    let steps=dx.max(-dy).min((width+height) as i64*6);
    if steps==0 {
        plot_pixel(pixels,z_buffer,width,height,x0,y0,start.z_mm,color);
        return Ok(());
    }
    let mut index=0_i64;
    loop {
        let z = start.z_mm + (end.z_mm-start.z_mm)*index/steps;
        plot_pixel(pixels,z_buffer,width,height,x0,y0,z,color);
        if x0==x1 && y0==y1 {break;}
        let twice=2*error;
        if twice>=dy {error+=dy;x0+=sx;}
        if twice<=dx {error+=dx;y0+=sy;}
        index+=1;
        if index>steps {break;}
    }
    Ok(())
}

fn plot_pixel(
    pixels:&mut [u8],
    z_buffer:&mut [i64],
    width:usize,
    height:usize,
    x:i64,
    y:i64,
    depth:i64,
    color:[u8;3],
){
    if x<0||y<0||x>=width as i64||y>=height as i64{return;}
    let index=y as usize*width+x as usize;
    if depth<z_buffer[index]{
        z_buffer[index]=depth;
        pixels[index*3..index*3+3].copy_from_slice(&color);
    }
}

fn camera_calibration_digest(camera:&CameraCalibration)->String{
    let mut bytes=Vec::new();
    append_string(&mut bytes,&camera.camera_id);
    for value in camera.position_world_mm{bytes.extend_from_slice(&value.to_le_bytes());}
    for row in camera.world_to_camera_q16{for value in row{bytes.extend_from_slice(&value.to_le_bytes());}}
    for value in [camera.focal_x_millipx,camera.focal_y_millipx,camera.principal_x_millipx,camera.principal_y_millipx,camera.width,camera.height,camera.near_plane_mm,camera.far_plane_mm]{
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn world_snapshot_digest(snapshot:&WorldRenderSnapshot)->String{
    let mut bytes=Vec::new();
    for value in [&snapshot.episode_id,&snapshot.scenario_id,&snapshot.agent_id,&snapshot.simulator_revision]{
        append_string(&mut bytes,value);
    }
    bytes.extend_from_slice(&snapshot.tick.to_le_bytes());
    bytes.extend_from_slice(&snapshot.world_timestamp_us.to_le_bytes());
    if let Some(action)=snapshot.applied_action{
        bytes.push(1);
        append_string(&mut bytes,action.receipt_id);
        append_string(&mut bytes,action.command_id);
        bytes.extend_from_slice(&action.applied_tick.to_le_bytes());
    }else{bytes.push(0);}
    append_string(&mut bytes,&camera_calibration_digest(&snapshot.camera));
    for road in &snapshot.road_markings{
        append_string(&mut bytes,&road.road_id);
        append_string(&mut bytes,&road.marking_id);
        bytes.push(road.kind as u8);
        bytes.extend_from_slice(&(road.points_world_mm.len() as u64).to_le_bytes());
        for point in &road.points_world_mm{for value in point{bytes.extend_from_slice(&value.to_le_bytes());}}
    }
    for actor in &snapshot.actors{
        append_string(&mut bytes,&actor.actor_id);
        append_string(&mut bytes,&actor.actor_policy_revision);
        bytes.push(actor.class as u8);
        for value in actor.center_world_mm{bytes.extend_from_slice(&value.to_le_bytes());}
        for value in actor.half_extents_mm{bytes.extend_from_slice(&value.to_le_bytes());}
    }
    sha256_hex(&bytes)
}

fn policy_receipt_digest(receipt:&PolicyObservationReceipt)->String{
    let mut bytes=Vec::new();
    bytes.extend_from_slice(&receipt.schema_version.to_le_bytes());
    for value in [&receipt.episode_id,&receipt.scenario_id,&receipt.agent_id] {append_string(&mut bytes,value);}
    bytes.extend_from_slice(&receipt.tick.to_le_bytes());
    bytes.extend_from_slice(&receipt.sequence.to_le_bytes());
    for value in [
        receipt.applied_action_receipt_id.as_deref().unwrap_or(""),
        receipt.applied_action_command_id.as_deref().unwrap_or(""),
        &receipt.camera_id,
        &receipt.camera_calibration_sha256,
        &receipt.renderer_revision,
        &receipt.frame_sha256,
    ] {append_string(&mut bytes,value);}
    bytes.push(u8::from(receipt.applied_action_tick.is_some()));
    bytes.extend_from_slice(&receipt.applied_action_tick.unwrap_or(0).to_le_bytes());
    for value in [receipt.capture_timestamp_us,receipt.render_completed_timestamp_us,receipt.delivered_timestamp_us,u64::from(receipt.frame_width),u64::from(receipt.frame_height),u64::from(receipt.channels),receipt.frame_byte_len as u64]{
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn append_string(bytes:&mut Vec<u8>,value:&str){
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

pub fn sha256_hex(input:&[u8])->String{
    const K:[u32;64]=[
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    ];
    let mut data=input.to_vec();
    let bit_len=(data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len()%64!=56{data.push(0);}
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut h=[0x6a09e667_u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    for chunk in data.chunks_exact(64){
        let mut w=[0_u32;64];
        for i in 0..16{w[i]=u32::from_be_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]);}
        for i in 16..64{
            let s0=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3);
            let s1=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10);
            w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh]=h;
        for i in 0..64{
            let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25);
            let ch=(e&f)^(!e&g);
            let temp1=hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22);
            let maj=(a&b)^(a&c)^(b&c);
            let temp2=s0.wrapping_add(maj);
            hh=g;g=f;f=e;e=d.wrapping_add(temp1);d=c;c=b;b=a;a=temp1.wrapping_add(temp2);
        }
        h[0]=h[0].wrapping_add(a);h[1]=h[1].wrapping_add(b);h[2]=h[2].wrapping_add(c);h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e);h[5]=h[5].wrapping_add(f);h[6]=h[6].wrapping_add(g);h[7]=h[7].wrapping_add(hh);
    }
    let mut output=String::with_capacity(64);
    for word in h{let _=write!(&mut output,"{word:08x}");}
    output
}

#[cfg(test)]
mod tests{
    use super::*;

    fn camera()->CameraCalibration{
        CameraCalibration{
            camera_id:"front".into(),
            position_world_mm:[0,1_500,0],
            world_to_camera_q16:[[Q16_ONE as i32,0,0],[0,0,Q16_ONE as i32],[0,-(Q16_ONE as i32),0]],
            focal_x_millipx:45_000,
            focal_y_millipx:45_000,
            principal_x_millipx:320_000,
            principal_y_millipx:180_000,
            width:640,
            height:360,
            near_plane_mm:100,
            far_plane_mm:200_000,
        }
    }

    fn snapshot()->WorldRenderSnapshot{
        WorldRenderSnapshot{
            episode_id:"episode-1".into(),
            scenario_id:"straight-road".into(),
            agent_id:"ego".into(),
            tick:1,
            world_timestamp_us:20_000,
            simulator_revision:"sim-fixture-v1".into(),
            applied_action:Some(AppliedActionRef{receipt_id:"action-receipt-1",command_id:"command-1",applied_tick:1}),
            camera:camera(),
            road_markings:vec![
                RoadPolyline{road_id:"road-1".into(),marking_id:"center-left".into(),kind:RoadMarkingKind::LaneWhite,points_world_mm:vec![[-2_000,0,10_000],[-2_000,0,20_000],[-2_000,0,40_000]]},
                RoadPolyline{road_id:"road-1".into(),marking_id:"center-right".into(),kind:RoadMarkingKind::LaneYellow,points_world_mm:vec![[2_000,0,10_000],[2_000,0,20_000],[2_000,0,40_000]]},
            ],
            actors:vec![ActorCuboid{actor_id:"lead".into(),actor_policy_revision:"actor-policy-v1".into(),class:ActorClass::Car,center_world_mm:[0,0,15_000],half_extents_mm:[900,700,2_000]}],
        }
    }

    fn clocks()->RenderClocks{RenderClocks{capture_timestamp_us:20_000,render_completed_timestamp_us:20_800,delivered_timestamp_us:21_000}}

    #[test] fn sha256_empty_known_answer(){assert_eq!(sha256_hex(b""),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");}
    #[test] fn sha256_abc_known_answer(){assert_eq!(sha256_hex(b"abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");}
    #[test] fn renders_deterministic_rgb_frame(){let a=SoftwareCameraRenderer::render(&snapshot(),1,clocks()).unwrap();let b=SoftwareCameraRenderer::render(&snapshot(),1,clocks()).unwrap();assert_eq!(a.pixels_rgb,b.pixels_rgb);assert_eq!(a.policy_receipt.frame_sha256,b.policy_receipt.frame_sha256);assert_eq!(a.policy_receipt.receipt_sha256,b.policy_receipt.receipt_sha256);}
    #[test] fn frame_contains_non_background_geometry(){let result=SoftwareCameraRenderer::render(&snapshot(),1,clocks()).unwrap();assert!(result.pixels_rgb.chunks_exact(3).any(|p|p==[236,236,224]));assert!(result.pixels_rgb.chunks_exact(3).any(|p|p==[214,72,58]));}
    #[test] fn camera_pose_change_changes_frame(){let mut a=snapshot();let mut b=snapshot();b.camera.position_world_mm[0]=250;let x=SoftwareCameraRenderer::render(&a,1,clocks()).unwrap();let y=SoftwareCameraRenderer::render(&b,1,clocks()).unwrap();assert_ne!(x.policy_receipt.frame_sha256,y.policy_receipt.frame_sha256);}
    #[test] fn intrinsics_change_changes_calibration_and_frame(){let mut a=snapshot();let mut b=snapshot();b.camera.focal_x_millipx=48_000;let x=SoftwareCameraRenderer::render(&a,1,clocks()).unwrap();let y=SoftwareCameraRenderer::render(&b,1,clocks()).unwrap();assert_ne!(x.policy_receipt.camera_calibration_sha256,y.policy_receipt.camera_calibration_sha256);assert_ne!(x.policy_receipt.frame_sha256,y.policy_receipt.frame_sha256);}
    #[test] fn actor_pose_change_changes_evaluator_digest_and_pixels(){let mut a=snapshot();let mut b=snapshot();b.actors[0].center_world_mm[0]=1_000;let x=SoftwareCameraRenderer::render(&a,1,clocks()).unwrap();let y=SoftwareCameraRenderer::render(&b,1,clocks()).unwrap();assert_ne!(x.evaluator_audit.world_snapshot_sha256,y.evaluator_audit.world_snapshot_sha256);assert_ne!(x.policy_receipt.frame_sha256,y.policy_receipt.frame_sha256);}
    #[test] fn road_geometry_change_changes_pixels(){let mut a=snapshot();let mut b=snapshot();b.road_markings[0].points_world_mm[0][0]-=1_000;let x=SoftwareCameraRenderer::render(&a,1,clocks()).unwrap();let y=SoftwareCameraRenderer::render(&b,1,clocks()).unwrap();assert_ne!(x.policy_receipt.frame_sha256,y.policy_receipt.frame_sha256);}
    #[test] fn applied_action_tick_must_match_world_tick(){let mut s=snapshot();s.applied_action=Some(AppliedActionRef{receipt_id:"r",command_id:"c",applied_tick:0});assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::AppliedActionTickMismatch));}
    #[test] fn initial_tick_has_no_applied_action(){let mut s=snapshot();s.tick=0;s.applied_action=None;s.world_timestamp_us=20_000;assert!(SoftwareCameraRenderer::render(&s,1,clocks()).is_ok());}
    #[test] fn future_action_receipt_rejected(){let mut s=snapshot();s.applied_action=Some(AppliedActionRef{receipt_id:"r",command_id:"c",applied_tick:2});assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::AppliedActionTickMismatch));}
    #[test] fn actor_and_road_truth_are_only_in_evaluator_audit(){let result=SoftwareCameraRenderer::render(&snapshot(),1,clocks()).unwrap();assert!(!result.policy_receipt.receipt_sha256.contains("lead"));assert_eq!(result.evaluator_audit.actor_ids,vec!["lead"]);}
    #[test] fn malformed_rotation_rejected(){let mut s=snapshot();s.camera.world_to_camera_q16[0]=[0,0,0];assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::InvalidCameraTransform));}
    #[test] fn reflected_rotation_rejected(){let mut s=snapshot();s.camera.world_to_camera_q16[0][0]=-(Q16_ONE as i32);assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::InvalidCameraTransform));}
    #[test] fn oversize_frame_rejected_before_allocation(){let mut s=snapshot();s.camera.width=MAX_FRAME_WIDTH+1;assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::InvalidFrameGeometry));}
    #[test] fn malformed_duplicate_actor_identity_rejected(){let mut s=snapshot();s.actors.push(s.actors[0].clone());assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::DuplicateActorIdentity));}
    #[test] fn duplicate_road_marking_identity_rejected(){let mut s=snapshot();s.road_markings.push(s.road_markings[0].clone());assert_eq!(SoftwareCameraRenderer::render(&s,1,clocks()).err(),Some(RenderError::DuplicateRoadIdentity));}
    #[test] fn capture_clock_must_match_snapshot_clock(){let mut c=clocks();c.capture_timestamp_us+=1;assert_eq!(SoftwareCameraRenderer::render(&snapshot(),1,c).err(),Some(RenderError::ClockCaptureMismatch));}
    #[test] fn render_cannot_precede_capture(){let mut c=clocks();c.render_completed_timestamp_us=19_000;assert_eq!(SoftwareCameraRenderer::render(&snapshot(),1,c).err(),Some(RenderError::RenderBeforeCapture));}
    #[test] fn delivery_cannot_precede_render(){let mut c=clocks();c.delivered_timestamp_us=20_700;assert_eq!(SoftwareCameraRenderer::render(&snapshot(),1,c).err(),Some(RenderError::DeliveryBeforeRender));}
    #[test] fn stream_guard_distinguishes_fresh_delayed_and_stale(){let mut g=ObservationStreamGuard::new(TimingPolicy{maximum_render_latency_us:1_000,maximum_delivery_age_us:3_000});assert_eq!(g.admit(1,1_000,1_500,2_000),Ok(ObservationDisposition::Fresh));assert_eq!(g.admit(2,2_000,3_500,4_000),Ok(ObservationDisposition::Delayed));assert_eq!(g.admit(3,3_000,4_000,7_000),Ok(ObservationDisposition::Stale));}
    #[test] fn stream_guard_rejects_duplicate_or_out_of_order_sequences_without_mutation(){let mut g=ObservationStreamGuard::new(TimingPolicy{maximum_render_latency_us:1_000,maximum_delivery_age_us:3_000});assert!(g.admit(2,2_000,2_500,3_000).is_ok());assert_eq!(g.admit(2,3_000,3_100,3_200),Err(RenderError::SequenceRegression));assert!(g.admit(3,3_000,3_100,3_200).is_ok());}
    #[test] fn stream_guard_rejects_capture_and_delivery_clock_regression(){let mut g=ObservationStreamGuard::new(TimingPolicy{maximum_render_latency_us:1_000,maximum_delivery_age_us:3_000});g.admit(1,2_000,2_200,2_500).unwrap();assert_eq!(g.admit(2,1_900,2_200,2_600),Err(RenderError::CaptureClockRegression));assert_eq!(g.admit(2,3_000,3_200,2_400),Err(RenderError::DeliveryClockRegression));}
    #[test] fn missing_frame_event_is_explicit(){let g=ObservationStreamGuard::new(TimingPolicy{maximum_render_latency_us:1_000,maximum_delivery_age_us:3_000});let event=g.missing_event("episode-1","ego",2,2,40_000,MissingFrameReason::CameraDrop).unwrap();assert_eq!(event.reason,MissingFrameReason::CameraDrop);assert_eq!(event.tick,2);}
    #[test] fn policy_receipt_does_not_include_evaluator_world_digest(){let result=SoftwareCameraRenderer::render(&snapshot(),1,clocks()).unwrap();assert!(!format!("{:?}",result.policy_receipt).contains(&result.evaluator_audit.world_snapshot_sha256));}
}

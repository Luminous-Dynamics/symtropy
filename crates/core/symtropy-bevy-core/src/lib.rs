// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! # symtropy-bevy-core
//!
//! Permissively-licensed Bevy plugin for [`symtropy-physics`] N-dimensional physics.
//! Zero AGPL dependencies.
//!
//! Use [`BevyPhysicsPlugin`] for drop-in physics with no coupling, or
//! [`BevyPhysicsCorePlugin`] + [`step_physics`] with your own
//! [`PhysicsCallback`] resource to couple any per-body metric to physics.
//!
//! For Φ (integrated information) coupling, see the AGPL sibling crate
//! `symtropy-bevy` — same API, adds `ConsciousnessField` integration.

use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*, window::CursorGrabMode};
use nalgebra::SVector;
use symtropy_physics::{CollisionEvent, PhysicsCallback, PhysicsWorld, body::BodyHandle};

// --- First-person input intent spine ---

/// Sensitivity defaults for first-person mouse look.
pub const DEFAULT_MOUSE_SENSITIVITY: Vec2 = Vec2::new(0.0022, 0.0018);

/// Component marking an entity as controlled by the reusable first-person spine.
#[derive(Component, Debug, Clone, Copy)]
pub struct FirstPersonController {
    pub walk_speed: f32,
    pub field_deck_speed: f32,
    pub sprint_multiplier: f32,
    pub crouch_multiplier: f32,
    pub mouse_sensitivity: Vec2,
}

impl Default for FirstPersonController {
    fn default() -> Self {
        Self {
            walk_speed: 5.0,
            field_deck_speed: 1.0,
            sprint_multiplier: 1.65,
            crouch_multiplier: 0.45,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }
}

/// Optional kinematic response profile for first-person movement.
///
/// Attach this component alongside FirstPersonController to opt into acceleration
/// and braking. Controllers without it preserve legacy instant-response behavior.
/// This is kinematic presentation movement, not collision-resolved character physics.
#[derive(Component, Debug, Clone, Copy)]
pub struct FirstPersonMotion {
    /// Current horizontal velocity in world units per second.
    pub horizontal_velocity: Vec3,
    /// Maximum rate of velocity change while a movement target is requested.
    pub acceleration: f32,
    /// Maximum rate of velocity change when releasing movement.
    pub deceleration: f32,
}

impl Default for FirstPersonMotion {
    fn default() -> Self {
        Self {
            horizontal_velocity: Vec3::ZERO,
            acceleration: 18.0,
            deceleration: 24.0,
        }
    }
}

impl FirstPersonMotion {
    /// Reset motion immediately when entering a non-moving control mode.
    pub fn stop(&mut self) {
        self.horizontal_velocity = Vec3::ZERO;
    }

    /// Advance horizontal velocity toward a requested velocity and return displacement.
    ///
    /// Trapezoidal displacement integration reduces frame-rate sensitivity for constant
    /// acceleration. Invalid/non-positive delta times or response rates fail closed.
    /// Non-finite velocity inputs fail closed to a stationary target.
    pub fn advance(&mut self, desired_velocity: Vec3, delta_secs: f32) -> Vec3 {
        if !delta_secs.is_finite() || delta_secs <= 0.0 {
            return Vec3::ZERO;
        }

        let previous = if self.horizontal_velocity.is_finite() {
            Vec3::new(self.horizontal_velocity.x, 0.0, self.horizontal_velocity.z)
        } else {
            Vec3::ZERO
        };
        let target = if desired_velocity.is_finite() {
            Vec3::new(desired_velocity.x, 0.0, desired_velocity.z)
        } else {
            Vec3::ZERO
        };

        let delta = target - previous;
        let distance = delta.length();
        if !distance.is_finite() {
            self.horizontal_velocity = Vec3::ZERO;
            return Vec3::ZERO;
        }

        let response_rate = if target.length_squared() > f32::EPSILON {
            self.acceleration
        } else {
            self.deceleration
        };
        if !response_rate.is_finite() || response_rate <= 0.0 {
            self.stop();
            return Vec3::ZERO;
        }
        let max_delta = response_rate * delta_secs;

        let (next, displacement) = if distance <= max_delta || distance <= f32::EPSILON {
            // If velocity reaches the target before this frame ends, integrate the
            // transition only until that instant and then travel at target velocity.
            let transition_time = if distance <= f32::EPSILON {
                0.0
            } else {
                (distance / response_rate).min(delta_secs)
            };
            let transition_displacement = (previous + target) * (0.5 * transition_time);
            let steady_displacement = target * (delta_secs - transition_time);
            (target, transition_displacement + steady_displacement)
        } else {
            let next = previous + delta * (max_delta / distance);
            (next, (previous + next) * (0.5 * delta_secs))
        };

        self.horizontal_velocity = next;
        displacement
    }
}

/// Optional static-world collision profile for a kinematic first-person controller.
/// Vertical offsets are measured from the controller Transform (the eye/reference point).
#[derive(Component, Debug, Clone, Copy)]
pub struct FirstPersonCollision {
    /// Horizontal body radius in world units.
    pub radius: f32,
    /// Distance from Transform origin down to the player's feet.
    pub feet_offset: f32,
    /// Distance from Transform origin up to the player's head.
    pub head_offset: f32,
    /// Safety margin retained from obstacles.
    pub skin_width: f32,
    /// Maximum sweep/slide passes per requested displacement.
    pub max_slide_iterations: u8,
}
impl Default for FirstPersonCollision {
    fn default() -> Self {
        Self {
            radius: 0.35,
            feet_offset: 1.35,
            head_offset: 0.22,
            skin_width: 0.025,
            max_slide_iterations: 4,
        }
    }
}

/// Static axis-aligned obstacle. Half-extents are explicit world-space values;
/// rendered mesh scale and entity rotation are deliberately not inferred.
#[derive(Component, Debug, Clone, Copy)]
pub struct FirstPersonObstacle {
    pub half_extents: Vec3,
}
impl FirstPersonObstacle {
    pub const fn new(half_extents: Vec3) -> Self {
        Self { half_extents }
    }
}

/// Result of a swept kinematic movement request.
#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMoveResult {
    pub displacement: Vec3,
    pub blocking_normals: Vec<Vec2>,
    /// True when invalid input or obstacle geometry forced a fail-closed stop.
    pub fail_closed: bool,
}

/// Resolve horizontal movement against static axis-aligned obstacle bounds.
///
/// Each tuple is (world center, half-extents). Vertical filtering prevents overhead
/// props/floor decals from becoming invisible walls. Expanding each AABB by the body
/// radius is conservative near corners and approximates a round horizontal footprint.
/// This is not a general-purpose shape cast or a replacement for dynamic physics.
pub fn resolve_first_person_motion(
    start: Vec3,
    desired_displacement: Vec3,
    collision: FirstPersonCollision,
    obstacles: &[(Vec3, Vec3)],
) -> FirstPersonMoveResult {
    let stopped = || FirstPersonMoveResult {
        displacement: Vec3::ZERO,
        blocking_normals: Vec::new(),
        fail_closed: true,
    };
    if !start.is_finite() || !desired_displacement.is_finite() {
        return stopped();
    }
    if !collision.radius.is_finite()
        || collision.radius < 0.0
        || !collision.feet_offset.is_finite()
        || collision.feet_offset < 0.0
        || !collision.head_offset.is_finite()
        || collision.head_offset < 0.0
        || !collision.skin_width.is_finite()
        || collision.skin_width < 0.0
    {
        return stopped();
    }

    let mut applied = Vec3::ZERO;
    let mut remaining = Vec3::new(desired_displacement.x, 0.0, desired_displacement.z);
    let mut blocking_normals: Vec<Vec2> = Vec::new();
    if remaining.length_squared() <= f32::EPSILON {
        return FirstPersonMoveResult {
            displacement: Vec3::ZERO,
            blocking_normals,
            fail_closed: false,
        };
    }

    for _ in 0..collision.max_slide_iterations.max(1) {
        let distance = remaining.length();
        if !distance.is_finite() {
            return stopped();
        }
        if distance <= f32::EPSILON {
            break;
        }
        let current = start + applied;
        let player_min_y = current.y - collision.feet_offset;
        let player_max_y = current.y + collision.head_offset;
        let origin = Vec2::new(current.x, current.z);
        let delta = Vec2::new(remaining.x, remaining.z);
        let mut earliest_hit: Option<(f32, Vec2)> = None;

        for (center, half_extents) in obstacles {
            if !center.is_finite()
                || !half_extents.is_finite()
                || half_extents.x < 0.0
                || half_extents.y < 0.0
                || half_extents.z < 0.0
            {
                // Unknown obstacle geometry must not be silently treated as empty space.
                return stopped();
            }
            let obstacle_min_y = center.y - half_extents.y;
            let obstacle_max_y = center.y + half_extents.y;
            if !obstacle_min_y.is_finite() || !obstacle_max_y.is_finite() {
                return stopped();
            }
            if player_max_y <= obstacle_min_y || player_min_y >= obstacle_max_y {
                continue;
            }
            let min = Vec2::new(
                center.x - half_extents.x - collision.radius,
                center.z - half_extents.z - collision.radius,
            );
            let max = Vec2::new(
                center.x + half_extents.x + collision.radius,
                center.z + half_extents.z + collision.radius,
            );
            if !min.is_finite() || !max.is_finite() {
                return stopped();
            }
            // Initial overlap is not an ordinary sweep hit: without depenetration,
            // continuing could report a successful zero-displacement move and leave
            // callers with stale movement state. Fail closed before resolving contacts.
            if origin.x > min.x && origin.x < max.x && origin.y > min.y && origin.y < max.y {
                return stopped();
            }
            if let Some((toi, normal)) = sweep_point_aabb(origin, delta, min, max)
                && earliest_hit.is_none_or(|(best, _)| toi < best)
            {
                earliest_hit = Some((toi, normal));
            }
        }

        let Some((toi, normal)) = earliest_hit else {
            applied += remaining;
            break;
        };
        let safe_distance = (toi * distance - collision.skin_width).max(0.0);
        let fraction = (safe_distance / distance).clamp(0.0, 1.0);
        let step = remaining * fraction;
        applied += step;
        remaining -= step;

        let normal3 = Vec3::new(normal.x, 0.0, normal.y);
        let into_wall = remaining.dot(normal3);
        if into_wall < 0.0 {
            remaining -= normal3 * into_wall;
            if !blocking_normals
                .iter()
                .any(|old| (*old - normal).length_squared() < 1.0e-6)
            {
                blocking_normals.push(normal);
            }
        } else {
            break;
        }
    }
    FirstPersonMoveResult {
        displacement: applied,
        blocking_normals,
        fail_closed: false,
    }
}

fn sweep_point_aabb(origin: Vec2, delta: Vec2, min: Vec2, max: Vec2) -> Option<(f32, Vec2)> {
    let inside = origin.x > min.x && origin.x < max.x && origin.y > min.y && origin.y < max.y;
    if inside {
        let distances = [
            (origin.x - min.x, Vec2::new(-1.0, 0.0)),
            (max.x - origin.x, Vec2::new(1.0, 0.0)),
            (origin.y - min.y, Vec2::new(0.0, -1.0)),
            (max.y - origin.y, Vec2::new(0.0, 1.0)),
        ];
        let normal = distances.into_iter().min_by(|a, b| a.0.total_cmp(&b.0))?.1;
        // Starting overlapped is not a valid "no hit": fail closed rather than
        // allowing motion through the obstacle. Depenetration is a separate feature.
        return Some((0.0, normal));
    }

    let mut entry = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    let mut entry_normal = Vec2::ZERO;
    for axis in 0..2 {
        let coordinate = if axis == 0 { origin.x } else { origin.y };
        let movement = if axis == 0 { delta.x } else { delta.y };
        let lower = if axis == 0 { min.x } else { min.y };
        let upper = if axis == 0 { max.x } else { max.y };
        if movement.abs() <= f32::EPSILON {
            if coordinate < lower || coordinate > upper {
                return None;
            }
            continue;
        }
        let first = (lower - coordinate) / movement;
        let second = (upper - coordinate) / movement;
        let near = first.min(second);
        let far = first.max(second);
        if near > entry {
            entry = near;
            entry_normal = if axis == 0 {
                Vec2::new(if movement > 0.0 { -1.0 } else { 1.0 }, 0.0)
            } else {
                Vec2::new(0.0, if movement > 0.0 { -1.0 } else { 1.0 })
            };
        }
        exit = exit.min(far);
        if entry > exit {
            return None;
        }
    }
    if !(0.0..=1.0).contains(&entry) || exit < 0.0 {
        None
    } else {
        Some((entry, entry_normal))
    }
}

fn remove_velocity_into_blocking_normals(velocity: &mut Vec3, normals: &[Vec2]) {
    for normal in normals {
        let normal3 = Vec3::new(normal.x, 0.0, normal.y);
        let into_wall = velocity.dot(normal3);
        if into_wall < 0.0 {
            *velocity -= normal3 * into_wall;
        }
    }
}

/// Centralized keyboard bindings for the default first-person control scheme.
#[derive(Resource, Debug, Clone)]
pub struct InputBindings {
    pub move_forward: KeyCode,
    pub move_back: KeyCode,
    pub move_left: KeyCode,
    pub move_right: KeyCode,
    pub sprint: KeyCode,
    pub crouch: KeyCode,
    pub jump: KeyCode,
    pub interact: KeyCode,
    pub focus_inspect: KeyCode,
    pub toggle_field_deck: KeyCode,
    pub quick_tool: KeyCode,
    pub repair_tool: KeyCode,
    pub build_mode: KeyCode,
    pub chronicle_panel: KeyCode,
    pub basin_map: KeyCode,
    pub cycle_view_mode: KeyCode,
    pub scan_visualization: KeyCode,
    pub previous_scan_mode: KeyCode,
    pub next_scan_mode: KeyCode,
    pub pause_or_release: KeyCode,
    pub controls_overlay: KeyCode,
    pub command_palette: KeyCode,
    pub dev_scenario_panel: KeyCode,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            move_forward: KeyCode::KeyW,
            move_back: KeyCode::KeyS,
            move_left: KeyCode::KeyA,
            move_right: KeyCode::KeyD,
            sprint: KeyCode::ShiftLeft,
            crouch: KeyCode::ControlLeft,
            jump: KeyCode::Space,
            interact: KeyCode::KeyE,
            focus_inspect: KeyCode::KeyF,
            toggle_field_deck: KeyCode::Tab,
            quick_tool: KeyCode::KeyQ,
            repair_tool: KeyCode::KeyR,
            build_mode: KeyCode::KeyB,
            chronicle_panel: KeyCode::KeyC,
            basin_map: KeyCode::KeyM,
            cycle_view_mode: KeyCode::F5,
            scan_visualization: KeyCode::KeyV,
            previous_scan_mode: KeyCode::BracketLeft,
            next_scan_mode: KeyCode::BracketRight,
            pause_or_release: KeyCode::Escape,
            controls_overlay: KeyCode::F1,
            command_palette: KeyCode::KeyK,
            dev_scenario_panel: KeyCode::F10,
        }
    }
}

/// Per-frame named input state. Gameplay systems should consume this instead
/// of reading raw keys directly.
#[derive(Resource, Default, Debug, Clone)]
pub struct IntentFrame {
    pub movement: Vec2,
    pub look_delta: Vec2,
    pub pressed: Vec<InputIntent>,
    pub just_pressed: Vec<InputIntent>,
}

impl IntentFrame {
    pub fn clear(&mut self) {
        self.movement = Vec2::ZERO;
        self.look_delta = Vec2::ZERO;
        self.pressed.clear();
        self.just_pressed.clear();
    }

    pub fn pressed(&self, intent: InputIntent) -> bool {
        self.pressed.contains(&intent)
    }

    pub fn just_pressed(&self, intent: InputIntent) -> bool {
        self.just_pressed.contains(&intent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputIntent {
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    Sprint,
    Crouch,
    Jump,
    Interact,
    FocusInspect,
    OpenFieldDeck,
    CycleFieldDeckModePrev,
    CycleFieldDeckModeNext,
    EquipToolSlot(u8),
    QuickTool,
    RepairTool,
    BuildMode,
    OpenChroniclePanel,
    OpenBasinMap,
    CycleViewMode,
    ToggleScanVisualization,
    PauseOrRelease,
    OpenControlsOverlay,
    OpenCommandPalette,
    OpenDevScenarioPanel,
    PauseSimulation,
    StepSimulation,
    ResetScenario,
    CaptureReplay,
}

#[derive(Resource, Debug, Clone)]
pub struct ControlsState {
    pub mode: ControlMode,
    pub selected_tool_slot: u8,
    pub scan_mode: ScanMode,
    pub show_controls: bool,
    pub show_dev_panel: bool,
    pub mouse_captured: bool,
}

impl Default for ControlsState {
    fn default() -> Self {
        Self {
            mode: ControlMode::FirstPerson,
            selected_tool_slot: 1,
            scan_mode: ScanMode::Ecology,
            show_controls: false,
            show_dev_panel: false,
            mouse_captured: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    FirstPerson,
    FieldDeck,
    Console,
    DevScenario,
    Pause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Infrastructure,
    Ecology,
    MachineDiagnostics,
    CivicClaims,
    NullSignalCorruption,
    RepairPreview,
    ChronicleEvidence,
}

impl ScanMode {
    pub const ALL: [ScanMode; 7] = [
        ScanMode::Infrastructure,
        ScanMode::Ecology,
        ScanMode::MachineDiagnostics,
        ScanMode::CivicClaims,
        ScanMode::NullSignalCorruption,
        ScanMode::RepairPreview,
        ScanMode::ChronicleEvidence,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ScanMode::Infrastructure => "Infrastructure",
            ScanMode::Ecology => "Ecology",
            ScanMode::MachineDiagnostics => "Machine Diagnostics",
            ScanMode::CivicClaims => "Civic Claims",
            ScanMode::NullSignalCorruption => "Null / Signal Corruption",
            ScanMode::RepairPreview => "Repair Preview",
            ScanMode::ChronicleEvidence => "Chronicle Evidence",
        }
    }

    pub fn offset(self, delta: isize) -> Self {
        let current = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0) as isize;
        let len = Self::ALL.len() as isize;
        Self::ALL[((current + delta).rem_euclid(len)) as usize]
    }
}

/// Player-facing camera rig selection for embodied scenes.
///
/// This is intentionally separate from [`ControlMode`]. Control modes describe
/// overlays and interaction context; view modes describe how the world camera
/// is positioned and how player movement should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerViewMode {
    FirstPerson,
    ThirdPerson,
    TacticalOverview,
    BasinMap,
    Globe,
    DebugRenderGate,
}

impl PlayerViewMode {
    /// The normal in-game cycle. Specialist modes like basin map, globe, and
    /// render-gate are entered by explicit systems instead of the basic view key.
    pub const PLAYABLE: [Self; 3] = [Self::FirstPerson, Self::ThirdPerson, Self::TacticalOverview];

    pub fn label(self) -> &'static str {
        match self {
            Self::FirstPerson => "First Person",
            Self::ThirdPerson => "Third Person",
            Self::TacticalOverview => "Tactical Overview",
            Self::BasinMap => "Basin Map",
            Self::Globe => "Globe",
            Self::DebugRenderGate => "Debug Render Gate",
        }
    }

    pub fn next_playable(self) -> Self {
        let Some(current) = Self::PLAYABLE.iter().position(|mode| *mode == self) else {
            return Self::FirstPerson;
        };
        Self::PLAYABLE[(current + 1) % Self::PLAYABLE.len()]
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerViewState {
    pub mode: PlayerViewMode,
}

impl Default for PlayerViewState {
    fn default() -> Self {
        Self {
            mode: PlayerViewMode::FirstPerson,
        }
    }
}

// --- Resource wrapping the physics world ---

/// Bevy resource wrapping an N-dimensional [`PhysicsWorld`].
///
/// Access this in your systems to add bodies, step simulation manually,
/// or query state.
#[derive(Resource)]
pub struct BevyPhysics<const D: usize> {
    /// The N-dimensional rigid body world.
    pub world: PhysicsWorld<D>,
}

impl<const D: usize> Default for BevyPhysics<D> {
    fn default() -> Self {
        Self {
            world: PhysicsWorld::new(SVector::zeros()),
        }
    }
}

impl<const D: usize> BevyPhysics<D> {
    /// Create with custom gravity.
    pub fn with_gravity(gravity: [f64; D]) -> Self {
        Self {
            world: PhysicsWorld::new(SVector::from(gravity)),
        }
    }
}

// --- Linking component ---

/// Bevy component linking an entity to a physics body.
///
/// Attach to a Bevy entity with a `Transform`; `sync_transforms` writes the
/// physics body's position into it each `FixedUpdate`.
#[derive(Component)]
pub struct PhysicsBody {
    /// Handle to the body in the physics world.
    pub handle: BodyHandle,
    /// Visual radius for debug rendering.
    pub visual_radius: f32,
}

impl PhysicsBody {
    /// Create a new component for a given body handle and visual radius.
    pub fn new(handle: BodyHandle, visual_radius: f32) -> Self {
        Self {
            handle,
            visual_radius,
        }
    }
}

// --- Default no-coupling callback (resource flavor) ---

/// Bevy `Resource` form of the identity "no-coupling" callback.
///
/// Forces, impulses, and friction pass through unchanged. Use when you want
/// N-dimensional physics without any per-body state coupling. Implements
/// [`PhysicsCallback<D>`] for all `D`.
#[derive(Resource, Default)]
pub struct NoCouplingResource;

impl<const D: usize> PhysicsCallback<D> for NoCouplingResource {
    fn modulate_force(&self, _: BodyHandle, force: &SVector<f64, D>) -> SVector<f64, D> {
        *force
    }
    fn modulate_impulse(&self, impulse: f64, _: &SVector<f64, D>) -> f64 {
        impulse
    }
    fn friction_multiplier(&self, _: &SVector<f64, D>, _: BodyHandle) -> f64 {
        1.0
    }
    fn on_collision(&mut self, _: &CollisionEvent<D>) {}
    fn record_dissipation(&mut self, _: f64) {}
    fn record_work(&mut self, _: BodyHandle, _: f64) {}
    fn apply_trauma(&mut self, _: &CollisionEvent<D>) {}
}

// --- Systems ---

/// Translate raw keyboard/mouse state into named first-person intents.
pub fn input_intent_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    bindings: Res<InputBindings>,
    mut intents: ResMut<IntentFrame>,
) {
    intents.clear();
    intents.look_delta = mouse_motion.delta;

    if keyboard.pressed(bindings.move_forward) {
        intents.movement.y += 1.0;
        intents.pressed.push(InputIntent::MoveForward);
    }
    if keyboard.pressed(bindings.move_back) {
        intents.movement.y -= 1.0;
        intents.pressed.push(InputIntent::MoveBack);
    }
    if keyboard.pressed(bindings.move_left) {
        intents.movement.x -= 1.0;
        intents.pressed.push(InputIntent::MoveLeft);
    }
    if keyboard.pressed(bindings.move_right) {
        intents.movement.x += 1.0;
        intents.pressed.push(InputIntent::MoveRight);
    }
    if keyboard.pressed(bindings.sprint) || keyboard.pressed(KeyCode::ShiftRight) {
        intents.pressed.push(InputIntent::Sprint);
    }
    if keyboard.pressed(bindings.crouch) || keyboard.pressed(KeyCode::ControlRight) {
        intents.pressed.push(InputIntent::Crouch);
    }

    macro_rules! add_just {
        ($key:expr, $intent:expr) => {
            if keyboard.just_pressed($key) {
                intents.just_pressed.push($intent);
            }
        };
    }
    macro_rules! add_just_ctrl {
        ($key:expr, $intent:expr) => {
            if (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
                && keyboard.just_pressed($key)
            {
                intents.just_pressed.push($intent);
            }
        };
    }

    add_just!(bindings.jump, InputIntent::Jump);
    add_just!(bindings.interact, InputIntent::Interact);
    add_just!(bindings.focus_inspect, InputIntent::FocusInspect);
    add_just!(bindings.toggle_field_deck, InputIntent::OpenFieldDeck);
    add_just!(bindings.quick_tool, InputIntent::QuickTool);
    add_just!(bindings.repair_tool, InputIntent::RepairTool);
    add_just!(bindings.build_mode, InputIntent::BuildMode);
    add_just!(bindings.chronicle_panel, InputIntent::OpenChroniclePanel);
    add_just!(bindings.basin_map, InputIntent::OpenBasinMap);
    add_just!(bindings.cycle_view_mode, InputIntent::CycleViewMode);
    add_just!(
        bindings.scan_visualization,
        InputIntent::ToggleScanVisualization
    );
    add_just!(
        bindings.previous_scan_mode,
        InputIntent::CycleFieldDeckModePrev
    );
    add_just!(bindings.next_scan_mode, InputIntent::CycleFieldDeckModeNext);
    add_just!(bindings.pause_or_release, InputIntent::PauseOrRelease);
    add_just!(bindings.controls_overlay, InputIntent::OpenControlsOverlay);
    add_just_ctrl!(bindings.command_palette, InputIntent::OpenCommandPalette);
    add_just!(
        bindings.dev_scenario_panel,
        InputIntent::OpenDevScenarioPanel
    );

    for (key, slot) in [
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
        (KeyCode::Digit4, 4),
        (KeyCode::Digit5, 5),
        (KeyCode::Digit6, 6),
    ] {
        if keyboard.just_pressed(key) {
            intents.just_pressed.push(InputIntent::EquipToolSlot(slot));
        }
    }

    add_just!(KeyCode::KeyP, InputIntent::PauseSimulation);
    add_just!(KeyCode::Period, InputIntent::StepSimulation);
    add_just!(KeyCode::F8, InputIntent::ResetScenario);
    add_just!(KeyCode::F9, InputIntent::CaptureReplay);

    if mouse.just_pressed(MouseButton::Left) {
        intents.just_pressed.push(InputIntent::FocusInspect);
    }
}

/// Apply generic menu/tool/Field Deck mode changes from named intents.
pub fn control_mode_system(intents: Res<IntentFrame>, mut controls: ResMut<ControlsState>) {
    if intents.just_pressed(InputIntent::PauseOrRelease) {
        controls.mode = ControlMode::Pause;
        controls.mouse_captured = false;
        controls.show_dev_panel = false;
    }

    if intents.just_pressed(InputIntent::OpenControlsOverlay) {
        controls.show_controls = !controls.show_controls;
    }

    if intents.just_pressed(InputIntent::OpenDevScenarioPanel) {
        controls.show_dev_panel = !controls.show_dev_panel;
        controls.mode = if controls.show_dev_panel {
            controls.mouse_captured = false;
            ControlMode::DevScenario
        } else {
            controls.mouse_captured = true;
            ControlMode::FirstPerson
        };
    }

    if intents.just_pressed(InputIntent::OpenFieldDeck) {
        controls.show_dev_panel = false;
        match controls.mode {
            ControlMode::FieldDeck => {
                controls.mode = ControlMode::FirstPerson;
                controls.mouse_captured = true;
            }
            _ => {
                controls.mode = ControlMode::FieldDeck;
                controls.mouse_captured = false;
            }
        }
    }

    if intents.just_pressed(InputIntent::CycleFieldDeckModePrev) {
        controls.scan_mode = controls.scan_mode.offset(-1);
    }
    if intents.just_pressed(InputIntent::CycleFieldDeckModeNext) {
        controls.scan_mode = controls.scan_mode.offset(1);
    }

    for intent in &intents.just_pressed {
        match *intent {
            InputIntent::EquipToolSlot(slot) => controls.selected_tool_slot = slot,
            InputIntent::FocusInspect if controls.mode == ControlMode::Pause => {
                controls.mode = ControlMode::FirstPerson;
                controls.mouse_captured = true;
            }
            _ => {}
        }
    }
}

/// Sync the OS cursor with current first-person capture state.
pub fn cursor_capture_system(
    controls: Res<ControlsState>,
    mut query: Query<&mut bevy::window::CursorOptions>,
) {
    if !controls.is_changed() {
        return;
    }
    let Ok(mut cursor_options) = query.single_mut() else {
        return;
    };
    cursor_options.visible = !controls.mouse_captured;
    cursor_options.grab_mode = if controls.mouse_captured {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
}

type FirstPersonCharacterQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static FirstPersonController,
        Option<&'static mut FirstPersonMotion>,
        Option<&'static FirstPersonCollision>,
    ),
    (With<FirstPersonController>, Without<FirstPersonObstacle>),
>;

type FirstPersonObstacleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static FirstPersonObstacle),
    (With<FirstPersonObstacle>, Without<FirstPersonController>),
>;

/// Move all entities carrying [`FirstPersonController`] from the current intent frame.
pub fn first_person_move_system(
    intents: Res<IntentFrame>,
    time: Res<Time>,
    controls: Res<ControlsState>,
    mut characters: FirstPersonCharacterQuery,
    obstacles: FirstPersonObstacleQuery,
) {
    let obstacle_bounds: Vec<(Vec3, Vec3)> = obstacles
        .iter()
        .map(|(transform, obstacle)| (transform.translation, obstacle.half_extents))
        .collect();

    for (mut transform, controller, mut motion, collision) in &mut characters {
        let base_speed = match controls.mode {
            ControlMode::FirstPerson => controller.walk_speed,
            ControlMode::FieldDeck => controller.field_deck_speed,
            ControlMode::Console | ControlMode::DevScenario | ControlMode::Pause => 0.0,
        };
        if base_speed == 0.0 {
            if let Some(motion) = motion.as_deref_mut() {
                motion.stop();
            }
            continue;
        }

        let mut speed = base_speed;
        if intents.pressed(InputIntent::Sprint) && controls.mode == ControlMode::FirstPerson {
            speed *= controller.sprint_multiplier;
        }
        if intents.pressed(InputIntent::Crouch) {
            speed *= controller.crouch_multiplier;
        }
        let mut direction =
            *transform.forward() * intents.movement.y + *transform.right() * intents.movement.x;
        direction.y = 0.0;
        let desired_velocity = if direction.length_squared() > 0.0 {
            direction.normalize() * speed
        } else {
            Vec3::ZERO
        };
        let dt = time.delta_secs();
        let desired_displacement = if let Some(motion) = motion.as_deref_mut() {
            motion.advance(desired_velocity, dt)
        } else {
            desired_velocity * dt
        };

        let actual_displacement = if let Some(collision) = collision {
            let result = resolve_first_person_motion(
                transform.translation,
                desired_displacement,
                *collision,
                &obstacle_bounds,
            );
            if let Some(motion) = motion.as_deref_mut() {
                if result.fail_closed {
                    // Do not preserve momentum through an invalid collision world.
                    motion.stop();
                } else {
                    remove_velocity_into_blocking_normals(
                        &mut motion.horizontal_velocity,
                        &result.blocking_normals,
                    );
                }
            }
            result.displacement
        } else {
            desired_displacement
        };
        transform.translation += actual_displacement;
    }
}

/// Rotate first-person entities from accumulated mouse movement.
pub fn first_person_mouse_look_system(
    intents: Res<IntentFrame>,
    controls: Res<ControlsState>,
    mut query: Query<(&mut Transform, &FirstPersonController)>,
) {
    if !controls.mouse_captured || controls.mode != ControlMode::FirstPerson {
        return;
    }
    let delta = intents.look_delta;
    if delta == Vec2::ZERO {
        return;
    }

    for (mut transform, controller) in &mut query {
        let delta_yaw = -delta.x * controller.mouse_sensitivity.x;
        let delta_pitch = -delta.y * controller.mouse_sensitivity.y;
        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let pitch_limit = std::f32::consts::FRAC_PI_2 - 0.01;
        transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            yaw + delta_yaw,
            (pitch + delta_pitch).clamp(-pitch_limit, pitch_limit),
            roll,
        );
    }
}

/// Reusable first-person input plugin. Add [`FirstPersonController`] to the
/// player/camera entity and read [`IntentFrame`] from game systems.
#[derive(Default)]
pub struct FirstPersonInputPlugin;

impl Plugin for FirstPersonInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputBindings>()
            .init_resource::<IntentFrame>()
            .init_resource::<ControlsState>()
            .add_systems(
                Update,
                (
                    input_intent_system,
                    control_mode_system,
                    cursor_capture_system,
                    first_person_move_system,
                    first_person_mouse_look_system,
                )
                    .chain(),
            );
    }
}

/// Step the physics world with a generic callback `Resource`.
///
/// Bring your own `C: PhysicsCallback<D> + Resource` to couple a custom metric
/// to physics, or use [`NoCouplingResource`] for uncoupled physics-only behavior.
pub fn step_physics<
    const D: usize,
    C: PhysicsCallback<D>
        + Resource
        + bevy::ecs::component::Component<Mutability = bevy::ecs::component::Mutable>,
>(
    mut physics: ResMut<BevyPhysics<D>>,
    mut cb: ResMut<C>,
    time: Res<Time<Fixed>>,
) {
    physics
        .world
        .step_with_callback(time.delta_secs_f64(), &mut *cb);
}

/// Sync physics body positions to Bevy `Transform`s.
///
/// 2D: writes `(x, y)` to `translation.x`/`.y`.
/// 3D: writes `(x, y, z)` to `translation`.
/// 4D: writes `(x, y, z)` to `translation` (w dropped — use `symtropy-render-bridge`
/// for cross-section projection).
pub fn sync_transforms<const D: usize>(
    physics: Res<BevyPhysics<D>>,
    mut query: Query<(&PhysicsBody, &mut Transform)>,
) {
    for (body_comp, mut transform) in &mut query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos = body.position();
            if D >= 1 {
                transform.translation.x = pos[0] as f32;
            }
            if D >= 2 {
                transform.translation.y = pos[1] as f32;
            }
            if D >= 3 {
                transform.translation.z = pos[2] as f32;
            }
        }
    }
}

// --- Plugins ---

/// Minimal Bevy plugin: registers [`BevyPhysics<D>`] + [`sync_transforms`] only.
///
/// Use this when you're supplying your own step system with a custom
/// [`PhysicsCallback`] resource.
pub struct BevyPhysicsCorePlugin<const D: usize> {
    /// Initial gravity.
    pub gravity: SVector<f64, D>,
}

impl<const D: usize> Default for BevyPhysicsCorePlugin<D> {
    fn default() -> Self {
        Self {
            gravity: SVector::zeros(),
        }
    }
}

impl<const D: usize> BevyPhysicsCorePlugin<D> {
    /// Create with the given gravity vector.
    pub fn with_gravity(gravity: [f64; D]) -> Self {
        Self {
            gravity: SVector::from(gravity),
        }
    }
}

/// Full Bevy plugin: registers [`BevyPhysics<D>`] + [`NoCouplingResource`] + a
/// default [`step_physics`] system + [`sync_transforms`].
///
/// Drop in for N-dimensional physics with no coupling. For custom couplings,
/// use [`BevyPhysicsCorePlugin`] and register your own step system.
pub struct BevyPhysicsPlugin<const D: usize> {
    /// Initial gravity.
    pub gravity: SVector<f64, D>,
}

impl<const D: usize> Default for BevyPhysicsPlugin<D> {
    fn default() -> Self {
        Self {
            gravity: SVector::zeros(),
        }
    }
}

impl<const D: usize> BevyPhysicsPlugin<D> {
    /// Create with the given gravity vector.
    pub fn with_gravity(gravity: [f64; D]) -> Self {
        Self {
            gravity: SVector::from(gravity),
        }
    }
}

// Per-dim Plugin impls — Bevy's Plugin trait isn't const-generic.

macro_rules! impl_core_plugin {
    ($d:literal) => {
        impl Plugin for BevyPhysicsCorePlugin<$d> {
            fn build(&self, app: &mut App) {
                app.insert_resource(BevyPhysics::<$d> {
                    world: PhysicsWorld::new(self.gravity),
                });
                app.add_systems(FixedUpdate, sync_transforms::<$d>);
            }
        }
    };
}

macro_rules! impl_full_plugin {
    ($d:literal) => {
        impl Plugin for BevyPhysicsPlugin<$d> {
            fn build(&self, app: &mut App) {
                app.insert_resource(BevyPhysics::<$d> {
                    world: PhysicsWorld::new(self.gravity),
                });
                app.insert_resource(NoCouplingResource);
                app.add_systems(
                    FixedUpdate,
                    (
                        step_physics::<$d, NoCouplingResource>,
                        sync_transforms::<$d>,
                    )
                        .chain(),
                );
            }
        }
    };
}

impl_core_plugin!(2);
impl_core_plugin!(3);
impl_core_plugin!(4);

impl_full_plugin!(2);
impl_full_plugin!(3);
impl_full_plugin!(4);

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;

    fn simulate_motion_at_hz(hz: u32) -> (Vec3, Vec3) {
        let mut motion = FirstPersonMotion::default();
        let mut position = Vec3::ZERO;
        let dt = 1.0 / hz as f32;
        for _ in 0..hz {
            position += motion.advance(Vec3::new(0.0, 0.0, -3.6), dt);
        }
        (position, motion.horizontal_velocity)
    }

    #[test]
    fn first_person_motion_is_nearly_frame_rate_invariant() {
        let (position_30, velocity_30) = simulate_motion_at_hz(30);
        let (position_60, velocity_60) = simulate_motion_at_hz(60);
        let (position_144, velocity_144) = simulate_motion_at_hz(144);

        assert!((position_30 - position_144).length() < 0.03);
        assert!((position_60 - position_144).length() < 0.02);
        assert!((velocity_30 - velocity_144).length() < 0.05);
        assert!((velocity_60 - velocity_144).length() < 0.03);
        assert!((velocity_144.z + 3.6).abs() < 0.001);
    }

    #[test]
    fn first_person_motion_brakes_instead_of_stopping_instantly() {
        let mut motion = FirstPersonMotion::default();
        let target = Vec3::new(0.0, 0.0, -3.6);
        for _ in 0..60 {
            motion.advance(target, 1.0 / 60.0);
        }
        assert!(motion.horizontal_velocity.length() > 3.5);

        let mut total_braking_distance = 0.0;
        for _ in 0..15 {
            total_braking_distance += motion.advance(Vec3::ZERO, 1.0 / 60.0).length();
        }

        assert!(total_braking_distance > 0.0);
        assert!(total_braking_distance < 0.5);
        assert_eq!(motion.horizontal_velocity, Vec3::ZERO);
    }

    #[test]
    fn first_person_motion_integrates_partial_frame_braking_without_overshoot() {
        let mut motion = FirstPersonMotion {
            horizontal_velocity: Vec3::new(0.0, 0.0, -0.2),
            ..default()
        };
        let displacement = motion.advance(Vec3::ZERO, 1.0 / 60.0);
        let expected_stop_distance = (0.2_f32 * 0.2) / (2.0 * motion.deceleration);

        assert!((displacement.length() - expected_stop_distance).abs() < 1.0e-5);
        assert_eq!(motion.horizontal_velocity, Vec3::ZERO);
    }

    #[test]
    fn first_person_motion_reversal_ramps_through_zero() {
        let mut motion = FirstPersonMotion {
            horizontal_velocity: Vec3::new(0.0, 0.0, -3.6),
            ..default()
        };
        let first_step = motion.advance(Vec3::new(0.0, 0.0, 3.6), 1.0 / 60.0);
        assert!(
            first_step.z < 0.0,
            "first displacement should retain prior momentum"
        );
        assert!(
            motion.horizontal_velocity.z < 0.0,
            "velocity should not reverse instantly"
        );
        for _ in 0..30 {
            motion.advance(Vec3::new(0.0, 0.0, 3.6), 1.0 / 60.0);
        }
        assert!(
            motion.horizontal_velocity.z > 0.0,
            "the velocity should eventually reverse"
        );
    }

    #[test]
    fn first_person_motion_ignores_invalid_delta_and_sanitizes_non_finite_target() {
        let mut motion = FirstPersonMotion {
            horizontal_velocity: Vec3::new(0.0, 0.0, -2.0),
            ..default()
        };
        let previous = motion.horizontal_velocity;
        assert_eq!(
            motion.advance(Vec3::ZERO, f32::NAN),
            Vec3::ZERO,
            "invalid delta must not move the transform"
        );
        assert_eq!(motion.horizontal_velocity, previous);

        motion.advance(Vec3::splat(f32::INFINITY), 1.0 / 60.0);
        assert!(motion.horizontal_velocity.is_finite());
        assert!(motion.horizontal_velocity.z < 0.0);
    }

    #[test]
    fn first_person_motion_stops_on_invalid_response_rate() {
        let mut motion = FirstPersonMotion {
            horizontal_velocity: Vec3::new(0.0, 0.0, -2.0),
            deceleration: f32::NAN,
            ..default()
        };
        assert_eq!(motion.advance(Vec3::ZERO, 1.0 / 60.0), Vec3::ZERO);
        assert_eq!(motion.horizontal_velocity, Vec3::ZERO);
    }

    fn obstacle(center: Vec3, half_extents: Vec3) -> (Vec3, Vec3) {
        (center, half_extents)
    }

    #[test]
    fn kinematic_sweep_stops_before_a_wall_without_tunneling() {
        let wall = obstacle(Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.5, 1.0, 3.0));
        let result = resolve_first_person_motion(
            Vec3::new(0.0, 1.45, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            FirstPersonCollision::default(),
            &[wall],
        );
        let contact_center_x = 1.0 - 0.5 - FirstPersonCollision::default().radius;
        assert!(result.displacement.x < contact_center_x);
        assert!(result.displacement.x > contact_center_x - 0.1);
        assert!(result.displacement.z.abs() < 1.0e-6);
        assert!(result.blocking_normals.contains(&Vec2::new(-1.0, 0.0)));
    }

    #[test]
    fn kinematic_sweep_slides_along_a_wall_and_cancels_inward_velocity() {
        let wall = obstacle(Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.5, 1.0, 4.0));
        let result = resolve_first_person_motion(
            Vec3::new(0.0, 1.45, 0.0),
            Vec3::new(1.0, 0.0, 2.0),
            FirstPersonCollision::default(),
            &[wall],
        );
        let mut velocity = Vec3::new(3.0, 0.0, 3.0);
        remove_velocity_into_blocking_normals(&mut velocity, &result.blocking_normals);
        assert!(result.displacement.x < 0.15);
        assert!(result.displacement.z > 1.9);
        assert!(
            velocity.x.abs() < 1.0e-6,
            "wall-normal velocity must be removed"
        );
        assert!(velocity.z > 0.0, "tangential velocity must survive");
    }

    #[test]
    fn kinematic_sweep_fails_closed_when_starting_inside_an_obstacle() {
        let wall = obstacle(Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.5, 1.0, 3.0));
        let result = resolve_first_person_motion(
            Vec3::new(1.6, 1.45, 0.0),
            Vec3::new(0.1, 0.0, 0.0),
            FirstPersonCollision::default(),
            &[wall],
        );

        assert_eq!(result.displacement, Vec3::ZERO);
        assert!(result.fail_closed);
    }

    #[test]
    fn kinematic_sweep_traverses_a_narrow_passage_without_sticking() {
        let left = obstacle(Vec3::new(-0.6, 1.0, 0.0), Vec3::new(0.1, 1.0, 5.0));
        let right = obstacle(Vec3::new(0.6, 1.0, 0.0), Vec3::new(0.1, 1.0, 5.0));
        let result = resolve_first_person_motion(
            Vec3::new(0.0, 1.45, -3.0),
            Vec3::new(0.0, 0.0, 6.0),
            FirstPersonCollision::default(),
            &[left, right],
        );
        assert!(result.displacement.z > 5.9);
        assert!(result.displacement.x.abs() < 1.0e-6);
        assert!(result.blocking_normals.is_empty());
    }

    #[test]
    fn kinematic_sweep_ignores_obstacles_above_the_player() {
        let ceiling_pipe = obstacle(Vec3::new(1.0, 3.0, 0.0), Vec3::new(0.2, 0.1, 2.0));
        let result = resolve_first_person_motion(
            Vec3::new(0.0, 1.45, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            FirstPersonCollision::default(),
            &[ceiling_pipe],
        );
        assert_eq!(result.displacement, Vec3::new(2.0, 0.0, 0.0));
        assert!(result.blocking_normals.is_empty());
    }

    #[test]
    fn kinematic_sweep_fails_closed_on_invalid_obstacle_bounds() {
        let invalid_obstacles = [
            obstacle(Vec3::new(f32::NAN, 1.0, 0.0), Vec3::new(0.5, 1.0, 3.0)),
            obstacle(Vec3::new(1.0, 1.0, 0.0), Vec3::new(-0.5, 1.0, 3.0)),
            obstacle(Vec3::splat(f32::MAX), Vec3::splat(f32::MAX)),
        ];

        for invalid in invalid_obstacles {
            let result = resolve_first_person_motion(
                Vec3::ZERO,
                Vec3::new(2.0, 0.0, 0.0),
                FirstPersonCollision::default(),
                &[invalid],
            );
            assert_eq!(
                result.displacement,
                Vec3::ZERO,
                "invalid obstacle bounds must not be treated as empty space",
            );
            assert!(result.fail_closed);
        }
    }

    #[test]
    fn kinematic_sweep_fails_closed_when_finite_displacement_overflows_distance() {
        let result = resolve_first_person_motion(
            Vec3::ZERO,
            Vec3::new(f32::MAX, 0.0, f32::MAX),
            FirstPersonCollision::default(),
            &[],
        );

        assert_eq!(result.displacement, Vec3::ZERO);
        assert!(result.fail_closed);
    }

    #[test]
    fn kinematic_sweep_rejects_non_finite_requests_and_bad_collider_settings() {
        let wall = obstacle(Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.5, 1.0, 3.0));
        let invalid_delta = resolve_first_person_motion(
            Vec3::ZERO,
            Vec3::splat(f32::NAN),
            FirstPersonCollision::default(),
            &[wall],
        );
        let invalid_collider = resolve_first_person_motion(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            FirstPersonCollision {
                radius: f32::NAN,
                ..default()
            },
            &[wall],
        );
        assert_eq!(invalid_delta.displacement, Vec3::ZERO);
        assert_eq!(invalid_collider.displacement, Vec3::ZERO);
    }

    #[test]
    fn bevy_physics_default_has_zero_gravity_2d() {
        let p = BevyPhysics::<2>::default();
        let g = p.world.gravity;
        assert_eq!(g[0], 0.0);
        assert_eq!(g[1], 0.0);
    }

    #[test]
    fn bevy_physics_with_gravity_stores_gravity() {
        let p = BevyPhysics::<3>::with_gravity([0.0, -9.81, 0.0]);
        let g = p.world.gravity;
        assert!((g[1] - (-9.81)).abs() < 1e-9);
    }

    #[test]
    fn physics_body_new_stores_handle_and_radius() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let h = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let b = PhysicsBody::new(h, 0.5);
        assert_eq!(b.handle, h);
        assert!((b.visual_radius - 0.5).abs() < 1e-9);
    }

    #[test]
    fn default_input_bindings_match_first_person_spine() {
        let bindings = InputBindings::default();
        assert_eq!(bindings.move_forward, KeyCode::KeyW);
        assert_eq!(bindings.interact, KeyCode::KeyE);
        assert_eq!(bindings.toggle_field_deck, KeyCode::Tab);
        assert_eq!(bindings.cycle_view_mode, KeyCode::F5);
        assert_eq!(bindings.dev_scenario_panel, KeyCode::F10);
    }

    #[test]
    fn scan_mode_cycles_deterministically() {
        assert_eq!(ScanMode::Ecology.offset(1), ScanMode::MachineDiagnostics);
        assert_eq!(
            ScanMode::Infrastructure.offset(-1),
            ScanMode::ChronicleEvidence
        );
    }

    #[test]
    fn player_view_modes_cycle_playable_rigs() {
        assert_eq!(
            PlayerViewMode::FirstPerson.next_playable(),
            PlayerViewMode::ThirdPerson
        );
        assert_eq!(
            PlayerViewMode::ThirdPerson.next_playable(),
            PlayerViewMode::TacticalOverview
        );
        assert_eq!(
            PlayerViewMode::TacticalOverview.next_playable(),
            PlayerViewMode::FirstPerson
        );
        assert_eq!(
            PlayerViewMode::BasinMap.next_playable(),
            PlayerViewMode::FirstPerson
        );
    }

    #[test]
    fn player_view_labels_are_player_facing() {
        assert_eq!(PlayerViewMode::FirstPerson.label(), "First Person");
        assert_eq!(PlayerViewMode::ThirdPerson.label(), "Third Person");
        assert_eq!(
            PlayerViewMode::TacticalOverview.label(),
            "Tactical Overview"
        );
        assert_eq!(PlayerViewMode::DebugRenderGate.label(), "Debug Render Gate");
    }

    #[test]
    fn intent_frame_tracks_named_intents() {
        let mut frame = IntentFrame::default();
        frame.pressed.push(InputIntent::Sprint);
        frame.just_pressed.push(InputIntent::Interact);
        frame.just_pressed.push(InputIntent::CycleViewMode);

        assert!(frame.pressed(InputIntent::Sprint));
        assert!(frame.just_pressed(InputIntent::Interact));
        assert!(frame.just_pressed(InputIntent::CycleViewMode));

        frame.clear();
        assert!(!frame.pressed(InputIntent::Sprint));
        assert!(!frame.just_pressed(InputIntent::Interact));
        assert!(!frame.just_pressed(InputIntent::CycleViewMode));
    }

    #[test]
    fn no_coupling_modulate_force_is_identity() {
        let cb = NoCouplingResource;
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let h = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let f_in = SVector::from([3.2, -2.7]);
        let f_out = <NoCouplingResource as PhysicsCallback<2>>::modulate_force(&cb, h, &f_in);
        assert_eq!(f_in, f_out);
    }

    #[test]
    fn no_coupling_friction_multiplier_is_one() {
        let cb = NoCouplingResource;
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let h = world.add_sphere(Point::new([0.0, 0.0, 0.0]), 1.0, 1.0);
        let point = SVector::from([0.0, 0.0, 0.0]);
        let mu = <NoCouplingResource as PhysicsCallback<3>>::friction_multiplier(&cb, &point, h);
        assert!((mu - 1.0).abs() < 1e-9);
    }

    #[test]
    fn core_plugin_2d_registers_resource() {
        let mut app = App::new();
        BevyPhysicsCorePlugin::<2>::default().build(&mut app);
        assert!(app.world().contains_resource::<BevyPhysics<2>>());
    }

    #[test]
    fn full_plugin_2d_registers_both_resources() {
        let mut app = App::new();
        BevyPhysicsPlugin::<2>::default().build(&mut app);
        assert!(app.world().contains_resource::<BevyPhysics<2>>());
        assert!(app.world().contains_resource::<NoCouplingResource>());
    }

    #[test]
    fn full_plugin_3d_registers_both_resources() {
        let mut app = App::new();
        BevyPhysicsPlugin::<3>::default().build(&mut app);
        assert!(app.world().contains_resource::<BevyPhysics<3>>());
        assert!(app.world().contains_resource::<NoCouplingResource>());
    }

    #[test]
    fn full_plugin_4d_registers_both_resources() {
        let mut app = App::new();
        BevyPhysicsPlugin::<4>::default().build(&mut app);
        assert!(app.world().contains_resource::<BevyPhysics<4>>());
        assert!(app.world().contains_resource::<NoCouplingResource>());
    }

    #[test]
    fn plugin_with_gravity_resource_accessible() {
        let mut app = App::new();
        BevyPhysicsPlugin::<2>::with_gravity([0.0, -9.81]).build(&mut app);
        let res = app.world().resource::<BevyPhysics<2>>();
        let g = res.world.gravity;
        assert!((g[1] - (-9.81)).abs() < 1e-9);
    }

    #[test]
    fn manual_step_with_no_coupling_gravity_pulls_body_down() {
        let mut p = BevyPhysics::<2>::with_gravity([0.0, -9.81]);
        let h = p.world.add_sphere(Point::new([0.0, 10.0]), 1.0, 1.0);
        let y0 = p.world.body(h).unwrap().position()[1];
        let mut cb = NoCouplingResource;
        for _ in 0..10 {
            p.world.step_with_callback(1.0 / 60.0, &mut cb);
        }
        let y1 = p.world.body(h).unwrap().position()[1];
        assert!(y1 < y0, "gravity should pull body down: y0={y0}, y1={y1}");
    }

    #[test]
    fn can_add_multiple_bodies() {
        let mut p = BevyPhysics::<3>::default();
        let h1 = p.world.add_sphere(Point::new([0.0, 0.0, 0.0]), 1.0, 1.0);
        let h2 = p.world.add_sphere(Point::new([5.0, 0.0, 0.0]), 1.0, 1.0);
        assert!(p.world.body(h1).is_some());
        assert!(p.world.body(h2).is_some());
    }
}

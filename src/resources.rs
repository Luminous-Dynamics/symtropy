// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Global ECS resources for Symtropy.

use bevy::prelude::*;
use std::collections::HashMap;
#[cfg(feature = "biometrics-runtime")]
pub use symthaea_biometrics::input_telemetry::{
    InputTelemetryEncoder, KeystrokeSample as BiometricKeystrokeSample,
    MouseSample as BiometricMouseSample,
};
#[cfg(feature = "biometrics-runtime")]
pub use symthaea_biometrics::stress_model::PlayerStressModel;
// TODO: re-enable when symthaea-muse compiles with muse-live
// use symthaea_muse::live_output::LiveMuseOutput;

#[cfg(feature = "consciousness-runtime")]
pub use symtropy_consciousness_physics::{ConsciousnessField, SafetyTier};

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyTier {
    Green,
    Yellow,
    Red,
}

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone)]
pub struct ThermodynamicConstants {
    pub consciousness_maintenance_per_tick: f64,
    pub ambient_regen_rate: f64,
    pub energy_well_regen_rate: f64,
    pub harmony_range: f64,
}

#[cfg(not(feature = "consciousness-runtime"))]
impl Default for ThermodynamicConstants {
    fn default() -> Self {
        Self {
            consciousness_maintenance_per_tick: 0.02,
            ambient_regen_rate: 0.01,
            energy_well_regen_rate: 0.3,
            harmony_range: 120.0,
        }
    }
}

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone)]
pub struct ConsciousnessEnergy {
    pub available: f64,
    pub capacity: f64,
    pub max_energy: f64,
    pub consumed_this_tick: f64,
    pub regenerated_this_tick: f64,
}

#[cfg(not(feature = "consciousness-runtime"))]
impl ConsciousnessEnergy {
    pub fn new(capacity: f64) -> Self {
        Self {
            available: capacity,
            capacity,
            max_energy: capacity,
            consumed_this_tick: 0.0,
            regenerated_this_tick: 0.0,
        }
    }

    pub fn tick_reset(&mut self) {
        self.consumed_this_tick = 0.0;
        self.regenerated_this_tick = 0.0;
    }

    pub fn consume(&mut self, amount: f64) {
        let amount = amount.max(0.0);
        let consumed = self.available.min(amount);
        self.available -= consumed;
        self.consumed_this_tick += consumed;
    }

    pub fn regenerate(&mut self, amount: f64) {
        let amount = amount.max(0.0);
        let regenerated = (self.capacity - self.available).max(0.0).min(amount);
        self.available += regenerated;
        self.regenerated_this_tick += regenerated;
    }

    pub fn fraction_remaining(&self) -> f64 {
        if self.capacity <= 1e-10 {
            0.0
        } else {
            (self.available / self.capacity).clamp(0.0, 1.0)
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.available <= 1e-10
    }
}

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone)]
pub struct EntityConsciousness {
    pub energy: ConsciousnessEnergy,
    pub harmony_activations: [f64; 9],
    pub prediction_error: f64,
    pub motor_precision: f64,
    pub safety_tier: SafetyTier,
}

#[cfg(not(feature = "consciousness-runtime"))]
impl EntityConsciousness {
    pub fn new(capacity: f64) -> Self {
        Self {
            energy: ConsciousnessEnergy::new(capacity),
            harmony_activations: [0.5; 9],
            prediction_error: 0.0,
            motor_precision: 1.0,
            safety_tier: SafetyTier::Green,
        }
    }

    pub fn phi(&self) -> f64 {
        let harmony = self.harmony_activations.iter().sum::<f64>() / 9.0;
        (self.energy.fraction_remaining() * 0.65 + harmony * 0.35).clamp(0.0, 1.0)
    }

    pub fn total_harmony_energy(&self) -> f64 {
        self.harmony_activations.iter().sum()
    }
}

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone)]
pub struct Sanctuary<const D: usize> {
    pub center: symtropy_math::Point<D>,
}

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone, Default)]
pub struct ThermodynamicLedger {
    pub dissipated: f64,
}

#[cfg(not(feature = "consciousness-runtime"))]
impl ThermodynamicLedger {
    pub fn record_dissipation(&mut self, amount: f64) {
        self.dissipated += amount.max(0.0);
    }
}

#[cfg(not(feature = "consciousness-runtime"))]
#[derive(Debug, Clone)]
pub struct ConsciousnessField<const D: usize> {
    pub entities: HashMap<symtropy_physics::BodyHandle, EntityConsciousness>,
    pub sanctuaries: HashMap<symtropy_physics::BodyHandle, Sanctuary<D>>,
    pub constants: ThermodynamicConstants,
    pub ledger: ThermodynamicLedger,
    pub collective_phi: f64,
}

#[cfg(not(feature = "consciousness-runtime"))]
impl<const D: usize> Default for ConsciousnessField<D> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "consciousness-runtime"))]
impl<const D: usize> ConsciousnessField<D> {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            sanctuaries: HashMap::new(),
            constants: ThermodynamicConstants::default(),
            ledger: ThermodynamicLedger::default(),
            collective_phi: 0.5,
        }
    }

    pub fn register(
        &mut self,
        handle: symtropy_physics::BodyHandle,
        energy_capacity: f64,
        _harmony_range: f64,
    ) {
        self.entities
            .insert(handle, EntityConsciousness::new(energy_capacity));
        self.sanctuaries.insert(
            handle,
            Sanctuary {
                center: symtropy_math::Point::origin(),
            },
        );
        self.update_collective_phi();
    }

    pub fn phi(&self, handle: symtropy_physics::BodyHandle) -> f64 {
        self.entities
            .get(&handle)
            .map(EntityConsciousness::phi)
            .unwrap_or(0.5)
    }

    pub fn resource_regeneration_multiplier(&self) -> f64 {
        (0.75 + self.collective_phi * 0.5).clamp(0.5, 1.5)
    }

    pub fn tick_thermodynamics(&mut self) -> f64 {
        self.update_collective_phi();
        self.collective_phi
    }

    fn update_collective_phi(&mut self) {
        if self.entities.is_empty() {
            self.collective_phi = 0.5;
            return;
        }
        self.collective_phi = self
            .entities
            .values()
            .map(EntityConsciousness::phi)
            .sum::<f64>()
            / self.entities.len() as f64;
    }
}

#[cfg(not(feature = "biometrics-runtime"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct BiometricMouseSample {
    pub x: f32,
    pub y: f32,
    pub timestamp_ms: u64,
}

#[cfg(not(feature = "biometrics-runtime"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct BiometricKeystrokeSample {
    pub key_down: bool,
    pub timestamp_ms: u64,
}

#[cfg(not(feature = "biometrics-runtime"))]
#[derive(Debug, Clone, Copy)]
pub struct StressVector {
    pub arousal: f32,
    pub valence: f32,
    pub dominance: f32,
}

#[cfg(not(feature = "biometrics-runtime"))]
impl Default for StressVector {
    fn default() -> Self {
        Self {
            arousal: 0.1,
            valence: 0.1,
            dominance: 0.5,
        }
    }
}

#[cfg(not(feature = "biometrics-runtime"))]
#[derive(Debug, Clone)]
pub struct InputTelemetryEncoder {
    last_mouse: Option<BiometricMouseSample>,
    mouse_surprise: f32,
    key_pressure: f32,
}

#[cfg(not(feature = "biometrics-runtime"))]
impl InputTelemetryEncoder {
    pub fn new() -> Self {
        Self {
            last_mouse: None,
            mouse_surprise: 0.0,
            key_pressure: 0.0,
        }
    }

    pub fn push_mouse(&mut self, sample: BiometricMouseSample) {
        if let Some(prev) = self.last_mouse {
            let dx = sample.x - prev.x;
            let dy = sample.y - prev.y;
            let dt_ms = sample.timestamp_ms.saturating_sub(prev.timestamp_ms).max(1) as f32;
            self.mouse_surprise = ((dx * dx + dy * dy).sqrt() * 1000.0 / dt_ms).clamp(0.0, 1.0);
        }
        self.last_mouse = Some(sample);
    }

    pub fn push_keystroke(&mut self, sample: BiometricKeystrokeSample) {
        let direction = if sample.key_down { 0.18 } else { -0.10 };
        self.key_pressure = (self.key_pressure + direction).clamp(0.0, 1.0);
        let _ = sample.timestamp_ms;
    }

    pub fn compute_stress_vector(&self) -> StressVector {
        let arousal = (self.mouse_surprise * 0.55 + self.key_pressure * 0.45).clamp(0.0, 1.0);
        StressVector {
            arousal,
            valence: (0.35 - arousal * 0.4).clamp(-1.0, 1.0),
            dominance: (0.6 - arousal * 0.25).clamp(0.0, 1.0),
        }
    }

    pub fn velocity_surprise(&self) -> f32 {
        self.mouse_surprise
    }

    pub fn reset(&mut self) {
        self.last_mouse = None;
        self.mouse_surprise = 0.0;
        self.key_pressure = 0.0;
    }
}

#[cfg(not(feature = "biometrics-runtime"))]
impl Default for InputTelemetryEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "biometrics-runtime"))]
#[derive(Debug, Clone)]
pub struct PlayerStressModel {
    pub allostatic_load: f32,
}

#[cfg(not(feature = "biometrics-runtime"))]
impl Default for PlayerStressModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "biometrics-runtime"))]
impl PlayerStressModel {
    pub fn new() -> Self {
        Self {
            allostatic_load: 0.0,
        }
    }

    pub fn tick(&mut self, stress: &StressVector, dt: f32) {
        let target = stress.arousal.clamp(0.0, 1.0);
        let rate = if target > self.allostatic_load {
            0.35
        } else {
            0.12
        };
        self.allostatic_load += (target - self.allostatic_load) * (rate * dt).clamp(0.0, 1.0);
        self.allostatic_load = self.allostatic_load.clamp(0.0, 1.0);
    }

    pub fn reset(&mut self) {
        self.allostatic_load = 0.0;
    }
}

/// Player behavioral biometrics state.
#[derive(Resource, Default)]
pub struct BiometricsCtx {
    pub encoder: InputTelemetryEncoder,
    pub model: PlayerStressModel,
}

/// Leviathan sleep phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepPhase {
    /// Safe — no audio effect, free exploration.
    Dormant,
    /// Warning — sub-bass hum, lights flicker.
    Stirring,
    /// Danger — full horror audio, doors seal.
    Awake,
    /// Lethal — Leviathan hunts the player.
    Hunting,
}

/// Global Leviathan state.
#[derive(Resource)]
pub struct LeviathanState {
    /// Current sleep phase.
    pub phase: SleepPhase,
    /// Accumulated noise from all sources [0, ∞).
    pub noise_accumulator: f32,
    /// Noise threshold to transition Dormant → Stirring.
    pub threshold: f32,
    /// Seconds the Leviathan has been in Stirring phase.
    pub stirring_duration: f32,
    /// Seconds of quiet since last noise.
    pub quiet_duration: f32,
    /// Grace period (seconds) at game start — noise ignored while > 0.
    pub grace_timer: f32,
}

/// Grace period before the Leviathan starts listening (seconds).
const LEVIATHAN_GRACE_SECS: f32 = 20.0;

impl Default for LeviathanState {
    fn default() -> Self {
        Self {
            phase: SleepPhase::Dormant,
            noise_accumulator: 0.0,
            threshold: 10.0, // high threshold — player must actively make noise to wake it
            stirring_duration: 0.0,
            quiet_duration: 0.0,
            grace_timer: LEVIATHAN_GRACE_SECS,
        }
    }
}

/// Dungeon seed for reproducible levels.
#[derive(Resource)]
pub struct DungeonSeed(pub u64);

impl Default for DungeonSeed {
    fn default() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(42),
        )
    }
}

/// Game state for phase management.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GamePhase {
    /// Title screen with menu.
    #[default]
    MainMenu,
    /// Generating the dungeon.
    Loading,
    /// Active gameplay.
    Playing,
    /// Active 3D gameplay (Milestone H1.5).
    Playing3D,
    /// Governance council session — gameplay paused for voting.
    Council,
    /// Leviathan caught the player.
    GameOver,
    /// Player escaped with the core.
    Victory,
    /// Sol Atlas globe view — planetary coordination layer.
    #[cfg(feature = "atlas")]
    GlobeView,
    /// Walkable ground-level view inside a fully-drilled-into H3 cell.
    /// Only reachable by drilling all the way in (see cell_entry.rs's
    /// zoom-floor detection) — every other zoom level stays the orbital
    /// GlobeView, per design: "only walkable if you're already in the hex,
    /// otherwise just an overhead view."
    #[cfg(feature = "atlas")]
    CellWalk,
    /// Phase 11: City-Scale Governance Demonstration.
    CityScale,
    /// Muse: Thermodynamic Visualizer.
    Muse,
}

// ============================================================================
// Governance / Economy event log (Phase A0)
// ============================================================================

/// Log of governance events for the HUD and post-game analysis.
#[derive(Resource, Default)]
pub struct GovernanceLog {
    /// Recent governance event messages (ring buffer, max 20).
    pub messages: std::collections::VecDeque<GovernanceMessage>,
}

/// A timestamped governance message.
pub struct GovernanceMessage {
    /// Game time when the event occurred.
    pub time_secs: f32,
    /// Human-readable event description.
    pub text: String,
    /// Severity: 0=info, 1=warning (oppression), 2=critical (crisis).
    pub severity: u8,
}

impl GovernanceLog {
    /// Push a message, keeping max 20 entries.
    pub fn push(&mut self, time_secs: f32, text: String, severity: u8) {
        self.messages.push_back(GovernanceMessage {
            time_secs,
            text,
            severity,
        });
        if self.messages.len() > 20 {
            self.messages.pop_front();
        }
    }
}

// ============================================================================
// Tile Grid (H1.1 — wall collision)
// ============================================================================

/// Spatial lookup for tile walkability. O(1) collision checks.
#[derive(Resource, Default)]
pub struct TileGrid {
    /// Map from (grid_x, grid_y) → walkable.
    pub cells: HashMap<(i32, i32), bool>,
    /// Tile size in pixels.
    pub tile_size: f32,
    /// Grid origin offset (half-width, half-height in tiles).
    pub origin_col: i32,
    pub origin_row: i32,
    pub cols: i32,
    pub rows: i32,
}

impl TileGrid {
    /// Check if a world-space position is walkable.
    pub fn is_walkable(&self, world_x: f32, world_y: f32) -> bool {
        let col = ((world_x / self.tile_size) + self.origin_col as f32).round() as i32;
        let row = (self.origin_row as f32 - (world_y / self.tile_size)).round() as i32;
        self.cells.get(&(col, row)).copied().unwrap_or(false)
    }
}

// ============================================================================
// Physics Engine (Symtropy consciousness-physics runtime)
// ============================================================================

/// 2D physics world resource — wraps the ND physics engine AND consciousness field.
///
/// The consciousness field is wired INTO the physics step via `step_with_callback`,
/// making Φ a real physical force that modulates impulses, friction, and energy.
#[derive(Resource)]
pub struct PhysicsWorldRes {
    pub world: symtropy_physics::PhysicsWorld<2>,
    pub consciousness: ConsciousnessField<2>,
}

impl Default for PhysicsWorldRes {
    fn default() -> Self {
        Self {
            world: symtropy_physics::PhysicsWorld::new(nalgebra::SVector::from([0.0, 0.0])),
            consciousness: ConsciousnessField::new(),
        }
    }
}

/// Buffered player input — written in Update, consumed in FixedUpdate.
///
/// This bridges the gap between Bevy's Update (where `just_pressed` works)
/// and FixedUpdate (where physics must run at a consistent timestep).
#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Desired movement direction (not normalized — magnitude encodes intent).
    pub direction: Vec2,
    /// Whether the player is sprinting.
    pub sprinting: bool,
}

// ============================================================================
// Settlement Metrics (Firstlight Basin Vertical Slice)
// ============================================================================

/// Global metrics for the Firstlight Basin settlement.
/// These metrics drive NPC behavior and the First Public Vote.
#[derive(Resource, Debug, Clone)]
pub struct SettlementMetrics {
    /// Power stability [0, 1].
    pub power: f32,
    /// Water availability [0, 1].
    pub water: f32,
    /// Food reserves [0, 1].
    pub food: f32,
    /// Infrastructure repair quality [0, 1].
    pub repair: f32,
    /// Collective trust in leadership/player [0, 1].
    pub trust: f32,
    /// Institutional legitimacy [0, 1].
    pub legitimacy: f32,
    /// Physical safety [0, 1].
    pub safety: f32,
    /// Systemic entropy [0, 1].
    pub entropy: f32,
}

impl Default for SettlementMetrics {
    fn default() -> Self {
        Self {
            power: 0.2,      // unstable
            water: 0.1,      // critical
            food: 0.3,       // low
            repair: 0.2,     // poor
            trust: 0.4,      // fragile
            legitimacy: 0.3, // provisional
            safety: 0.3,     // weak
            entropy: 0.6,    // rising
        }
    }
}

// ============================================================================
// Governance Vote (Firstlight Basin Vertical Slice)
// ============================================================================

/// A strategic decision for the settlement.
#[derive(Debug, Clone)]
pub struct VoteOption {
    pub label: String,
    pub description: String,
    pub effect_text: String,
}

/// Active governance vote state.
#[derive(Resource, Debug, Clone, Default)]
pub struct GovernanceVote {
    pub is_active: bool,
    pub question: String,
    pub options: Vec<VoteOption>,
    pub selected_index: usize,
}

impl GovernanceVote {
    pub fn new_water_crisis_vote() -> Self {
        Self {
            is_active: true,
            question: "What should Seedworks become after surviving the water crisis?".to_string(),
            options: vec![
                VoteOption {
                    label: "Public Repair".to_string(),
                    description: "Reinforce shared infrastructure and housing.".to_string(),
                    effect_text: "Trust rises, repair costs decrease.".to_string(),
                },
                VoteOption {
                    label: "Factory Overdrive".to_string(),
                    description: "Prioritize fabrication and machine expansion.".to_string(),
                    effect_text: "Production rises, pollution risk begins.".to_string(),
                },
                VoteOption {
                    label: "Perimeter Defense".to_string(),
                    description: "Build walls and floodlights.".to_string(),
                    effect_text: "Safety improves, trust may split.".to_string(),
                },
                VoteOption {
                    label: "Archive Recovery".to_string(),
                    description: "Investigate the Ghost Civic Center.".to_string(),
                    effect_text: "Knowledge increases, old systems awaken.".to_string(),
                },
            ],
            selected_index: 0,
        }
    }
}

// ============================================================================
// Energy Wells (thermodynamic life sources)
// ============================================================================

/// Spatial energy source in the dungeon. Entities within radius regenerate energy.
/// Wells have finite capacity — they deplete over time (forcing migration).
#[derive(Component)]
pub struct EnergyWell {
    /// Joules per tick for entities in range.
    pub regen_rate: f64,
    /// Effect radius in world units.
    pub radius: f32,
    /// Remaining Joules in this well.
    pub remaining: f64,
    /// Maximum capacity (for display).
    pub max_capacity: f64,
}

impl EnergyWell {
    pub fn new(regen_rate: f64, radius: f32, capacity: f64) -> Self {
        Self {
            regen_rate,
            radius,
            remaining: capacity,
            max_capacity: capacity,
        }
    }

    /// Fraction of energy remaining [0, 1].
    pub fn fraction_remaining(&self) -> f64 {
        if self.max_capacity < 1e-10 {
            return 0.0;
        }
        self.remaining / self.max_capacity
    }

    /// Whether this well still has energy.
    pub fn is_active(&self) -> bool {
        self.remaining > 1e-10
    }
}

// ============================================================================
// Live Audio Output (H1.2)
// ============================================================================

// AudioOutput moved to systems/audio.rs as AudioState

// ============================================================================
// Shared Presentation / Simulation Site Layout
// ============================================================================

/// Global site layout describing dungeon room centers and tiles.
#[derive(Resource, Clone, Debug, Default)]
pub struct SiteLayout {
    pub site_id: String,
    pub width: usize,
    pub height: usize,
    /// 0=wall, 1=floor, 2=core_room, 3=player_start
    pub tiles: Vec<Vec<u8>>,
    /// Center of rooms
    pub room_centers: Vec<(usize, usize)>,
    /// Coordinates of special points (player spawn, core, etc.) in world space
    pub player_start: Vec2,
    pub core_pos: Vec2,
}

// ============================================================================
// Playable Tutorial Scenario (Causality Engine)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Reflect)]
pub enum TutorialStep {
    #[default]
    Inactive,
    PumpSabotaged,
    PR4Repairing,
    CoopRepairing,
    LeoStressRising,
    LeoStabilizing,
    Completed,
}

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct TutorialScenarioRes {
    pub step: TutorialStep,
    pub pump_entity: Option<Entity>,
}

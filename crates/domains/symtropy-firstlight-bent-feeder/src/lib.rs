// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic Bent Feeder repair, refusal, resident need, and delayed consequence.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_game_state::{EventChain, StableId, StateError};
use symtropy_residents::ResidentNeed;

/// Canonical Firstlight operation identifier.
pub const OPERATION_ID: &str = "firstlight.bent-feeder-recovery";

/// Current operation phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationPhase {
    /// Player has not yet established a safe working picture.
    Observe,
    /// Hazards are being isolated and verified.
    MakeSafe,
    /// Repair, bypass, relocation, or refusal is in progress.
    RepairOrRefuse,
    /// The selected configuration is being tested under bounded load.
    Test,
    /// Operation has reached a persistent outcome.
    Complete,
}

/// Physical feeder state that remains after the player leaves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeederState {
    /// Whether upstream power is connected.
    pub energized: bool,
    /// Whether the local isolation device is open and locked.
    pub isolated: bool,
    /// Whether a local instrument verified the conductors de-energized.
    pub verified_dead: bool,
    /// Lateral support movement measured in millimetres.
    pub support_displacement_mm: u16,
    /// Insulation damage from 0 to 10,000.
    pub insulation_damage: u16,
    /// Standing water in the work zone.
    pub water_depth_mm: u16,
    /// Conductor temperature in tenths of a degree Celsius.
    pub conductor_temperature_deci_c: i16,
    /// Whether the permanent support was braced.
    pub support_braced: bool,
    /// Whether the damaged insulation assembly was replaced.
    pub insulator_replaced: bool,
    /// Whether a temporary bypass is carrying service.
    pub temporary_bypass_installed: bool,
    /// Whether non-critical downstream loads were shed.
    pub noncritical_load_shed: bool,
    /// Whether an arc or converter fire occurred.
    pub arc_fire: bool,
    /// Whether the site is restricted pending later work.
    pub restricted_service: bool,
}

impl Default for FeederState {
    fn default() -> Self {
        Self {
            energized: true,
            isolated: false,
            verified_dead: false,
            support_displacement_mm: 34,
            insulation_damage: 7_800,
            water_depth_mm: 22,
            conductor_temperature_deci_c: 468,
            support_braced: false,
            insulator_replaced: false,
            temporary_bypass_installed: false,
            noncritical_load_shed: false,
            arc_fire: false,
            restricted_service: false,
        }
    }
}

/// Critical resident service affected by the feeder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentService {
    /// Resident whose continuity need makes the operation concrete.
    pub resident_id: StableId,
    /// Need visible to authorized participants.
    pub need: ResidentNeed,
    /// Remaining battery reserve while grid service is unavailable.
    pub battery_reserve_ticks: u64,
    /// Whether service currently receives safe electrical power.
    pub powered: bool,
    /// Whether a safe relocation was completed.
    pub relocated: bool,
    /// Whether service loss caused harm.
    pub harmed: bool,
}

/// Persistent end state of the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationOutcome {
    /// Permanent support and insulation repair passed bounded testing.
    DurableRepair,
    /// Temporary bypass preserves critical service with restricted capacity.
    RestrictedBypass,
    /// Work was safely refused and the resident was relocated.
    SafeRefusalAndRelocation,
    /// Operation ended with service unavailable but no immediate injury.
    UnresolvedServiceLoss,
    /// Unsafe live work caused an arc/fire and possible harm.
    ArcFire,
}

/// Player or NPC action accepted by the headless operation kernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BentFeederAction {
    /// Inspect local geometry, heat, water, and service loads.
    Inspect,
    /// Exclude bystanders and establish wet-work boundaries.
    EstablishPerimeter,
    /// Open and lock upstream isolation.
    IsolateUpstream,
    /// Verify absence of dangerous voltage locally.
    VerifyDeEnergized,
    /// Install structural bracing.
    BraceSupport,
    /// Replace the damaged insulation assembly.
    ReplaceInsulator,
    /// Install a temporary bypass around the damaged assembly.
    InstallTemporaryBypass,
    /// Disconnect non-critical downstream loads.
    ShedNoncriticalLoad,
    /// Restore upstream power for a bounded test.
    EnergizeForTest,
    /// Refuse unsafe work and relocate the critical resident service.
    SafeRefusalAndRelocation,
    /// Attempt live manipulation without isolation.
    UnsafeLiveShortcut,
    /// Advance deterministic time while the world continues.
    AdvanceTicks(u64),
}

/// Why an action is currently unavailable to the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionBlockReason {
    /// The operation has reached a terminal outcome.
    OperationComplete,
    /// A work perimeter must be established before work begins.
    WorkPerimeterRequired,
    /// Upstream isolation must be open before testing or repair.
    IsolationRequired,
    /// Local absence-of-voltage verification is still required.
    DeEnergizedVerificationRequired,
    /// A required part is absent or depleted.
    MissingPart(String),
    /// The configuration has no verified isolation history.
    VerifiedIsolationRequired,
    /// Neither a complete durable repair nor a valid bypass is ready.
    RepairOrBypassRequired,
    /// A temporary bypass requires non-critical load shedding.
    NoncriticalLoadMustBeShed,
}

impl ActionBlockReason {
    fn into_error(&self) -> BentFeederError {
        match self {
            Self::OperationComplete => BentFeederError::AlreadyComplete,
            Self::WorkPerimeterRequired => {
                BentFeederError::UnsafeSequence("establish the work perimeter before feeder work")
            }
            Self::IsolationRequired => {
                BentFeederError::UnsafeSequence("isolate upstream before this action")
            }
            Self::DeEnergizedVerificationRequired => {
                BentFeederError::UnsafeSequence("verify the conductors de-energized before repair")
            }
            Self::MissingPart(part) => BentFeederError::MissingPart(part.clone()),
            Self::VerifiedIsolationRequired => {
                BentFeederError::UnsafeSequence("test requires a verified isolation history")
            }
            Self::RepairOrBypassRequired => BentFeederError::UnsafeSequence(
                "no complete repair or load-shed bypass configuration is ready for test",
            ),
            Self::NoncriticalLoadMustBeShed => BentFeederError::UnsafeSequence(
                "shed non-critical downstream load before testing a temporary bypass",
            ),
        }
    }
}

/// Semantic cue an interactive client can render after a corresponding action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackCue {
    /// Initial inspection yields physical readings.
    InspectionReadout,
    /// Physical work boundary is visibly and audibly established.
    WorkBoundaryConfirmed,
    /// Isolation and downstream service state have changed.
    IsolationStateChanged,
    /// Local instrument confirms absence of dangerous voltage.
    VoltageAbsenceConfirmed,
    /// Player can perceive the support moving into its braced state.
    SupportMovementConfirmed,
    /// Replacement insulation is seated and acknowledged.
    InsulationReplacementConfirmed,
    /// Temporary bypass status changes.
    BypassStateChanged,
    /// Non-critical demand has been shed.
    LoadShedConfirmed,
    /// Thermal trend becomes legible after bounded energization or elapsed time.
    ThermalTrendChanged,
    /// Critical resident service changes state.
    CriticalServiceStatusChanged,
    /// An unsafe live-work incident occurs.
    SafetyIncidentAlarm,
    /// Remaining service battery reserve changes.
    BatteryReserveChanged,
    /// A persistent operation outcome has been committed.
    OutcomeCommitted,
}

/// Presentation channels for gameplay feedback. Haptics are supplemental, never mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackChannel {
    /// Shape, motion, iconography, text, or other non-color-only visual.
    Visual,
    /// Audio cue.
    Audio,
    /// Optional controller vibration or other haptic output.
    Haptic,
    /// Explicit instrument-style reading or numeric/text status.
    InstrumentReadout,
}

impl FeedbackCue {
    /// Minimum independent channels. Visuals must not rely on color alone.
    pub const fn minimum_channels(self) -> &'static [FeedbackChannel] {
        match self {
            Self::InspectionReadout => &[
                FeedbackChannel::Visual,
                FeedbackChannel::Audio,
                FeedbackChannel::InstrumentReadout,
            ],
            Self::WorkBoundaryConfirmed | Self::SupportMovementConfirmed => {
                &[FeedbackChannel::Visual, FeedbackChannel::Audio]
            }
            Self::IsolationStateChanged
            | Self::LoadShedConfirmed
            | Self::CriticalServiceStatusChanged
            | Self::SafetyIncidentAlarm
            | Self::BatteryReserveChanged
            | Self::OutcomeCommitted => &[
                FeedbackChannel::Visual,
                FeedbackChannel::Audio,
                FeedbackChannel::InstrumentReadout,
            ],
            Self::VoltageAbsenceConfirmed
            | Self::InsulationReplacementConfirmed
            | Self::BypassStateChanged
            | Self::ThermalTrendChanged => &[
                FeedbackChannel::Visual,
                FeedbackChannel::Audio,
                FeedbackChannel::InstrumentReadout,
            ],
        }
    }

    /// Optional channel additions; devices without haptics remain fully supported.
    pub const fn optional_channels(self) -> &'static [FeedbackChannel] {
        match self {
            Self::SupportMovementConfirmed
            | Self::InsulationReplacementConfirmed
            | Self::BypassStateChanged => &[FeedbackChannel::Haptic],
            _ => &[],
        }
    }
}

/// Read-only, renderer-independent explanation of one action's current affordance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPreview {
    /// The action being previewed.
    pub action: BentFeederAction,
    /// Whether it is ready or the first actionable blocker.
    pub readiness: ActionReadiness,
    /// Semantic feedback for a successful application, not proof that it happened.
    pub feedback_cues: Vec<FeedbackCue>,
}

/// Whether an action may be applied in the current authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionReadiness {
    /// Required preconditions are satisfied.
    Ready,
    /// Action is blocked by a known prerequisite.
    Blocked {
        /// First prerequisite the player can address.
        reason: ActionBlockReason,
    },
}

/// Event payload persisted for replay and delayed explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BentFeederEvent {
    /// Action applied.
    pub action: BentFeederAction,
    /// Phase after applying the action.
    pub resulting_phase: OperationPhase,
    /// Concise physical result.
    pub result: String,
}

/// Complete deterministic operation state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BentFeederOperation {
    /// Scenario seed used for deterministic identities.
    pub seed: u64,
    /// Current authoritative tick.
    pub tick: u64,
    /// Current operation phase.
    pub phase: OperationPhase,
    /// Physical feeder configuration.
    pub feeder: FeederState,
    /// Critical resident service.
    pub resident_service: ResidentService,
    /// Parts and tools available at the site.
    pub inventory: BTreeMap<String, u32>,
    /// Whether the work perimeter was established.
    pub perimeter_established: bool,
    /// Persistent outcome after completion.
    pub outcome: Option<OperationOutcome>,
    /// Replayable causal event chain.
    pub events: EventChain<BentFeederEvent>,
}

impl BentFeederOperation {
    /// Creates the canonical vertical-slice operation.
    pub fn canonical(seed: u64) -> Self {
        Self {
            seed,
            tick: 0,
            phase: OperationPhase::Observe,
            feeder: FeederState::default(),
            resident_service: ResidentService {
                resident_id: StableId::derive("resident", seed, 0),
                need: ResidentNeed {
                    kind: "powered-respiratory-support".into(),
                    urgency: 9_200,
                    preferred_location: Some(StableId::derive("service", seed, 0)),
                    consequence: "battery exhaustion interrupts respiratory support".into(),
                },
                battery_reserve_ticks: 900,
                powered: true,
                relocated: false,
                harmed: false,
            },
            inventory: BTreeMap::from([
                ("support-brace".into(), 1),
                ("insulator-kit".into(), 1),
                ("temporary-bypass".into(), 1),
            ]),
            perimeter_established: false,
            outcome: None,
            events: EventChain::new(OPERATION_ID, seed),
        }
    }

    /// Previews an action without changing world state or appending an event.
    pub fn preview_action(&self, action: &BentFeederAction) -> ActionPreview {
        let readiness = match self.action_block_reason(action) {
            Some(reason) => ActionReadiness::Blocked { reason },
            None => ActionReadiness::Ready,
        };
        ActionPreview {
            action: action.clone(),
            readiness,
            feedback_cues: feedback_cues_for_action(action),
        }
    }

    fn action_block_reason(&self, action: &BentFeederAction) -> Option<ActionBlockReason> {
        if self.phase == OperationPhase::Complete {
            return Some(ActionBlockReason::OperationComplete);
        }

        match action {
            BentFeederAction::Inspect
            | BentFeederAction::EstablishPerimeter
            | BentFeederAction::ShedNoncriticalLoad
            | BentFeederAction::SafeRefusalAndRelocation
            | BentFeederAction::UnsafeLiveShortcut
            | BentFeederAction::AdvanceTicks(_) => None,
            BentFeederAction::IsolateUpstream => {
                (!self.perimeter_established).then_some(ActionBlockReason::WorkPerimeterRequired)
            }
            BentFeederAction::VerifyDeEnergized => {
                if !self.feeder.isolated || self.feeder.energized {
                    Some(ActionBlockReason::IsolationRequired)
                } else {
                    None
                }
            }
            BentFeederAction::BraceSupport => self
                .safe_work_block_reason()
                .or_else(|| self.part_block_reason("support-brace")),
            BentFeederAction::ReplaceInsulator => self
                .safe_work_block_reason()
                .or_else(|| self.part_block_reason("insulator-kit")),
            BentFeederAction::InstallTemporaryBypass => self
                .safe_work_block_reason()
                .or_else(|| self.part_block_reason("temporary-bypass")),
            BentFeederAction::EnergizeForTest => self.test_block_reason(),
        }
    }

    fn safe_work_block_reason(&self) -> Option<ActionBlockReason> {
        if !self.perimeter_established {
            Some(ActionBlockReason::WorkPerimeterRequired)
        } else if !self.feeder.isolated || self.feeder.energized {
            Some(ActionBlockReason::IsolationRequired)
        } else if !self.feeder.verified_dead {
            Some(ActionBlockReason::DeEnergizedVerificationRequired)
        } else {
            None
        }
    }

    fn part_block_reason(&self, item: &str) -> Option<ActionBlockReason> {
        (self.inventory.get(item).copied().unwrap_or(0) == 0)
            .then(|| ActionBlockReason::MissingPart(item.to_owned()))
    }

    fn test_block_reason(&self) -> Option<ActionBlockReason> {
        if !self.feeder.isolated || self.feeder.energized || !self.feeder.verified_dead {
            return Some(ActionBlockReason::VerifiedIsolationRequired);
        }
        let durable = self.feeder.support_braced && self.feeder.insulator_replaced;
        let bypass = self.feeder.temporary_bypass_installed && self.feeder.noncritical_load_shed;
        if !durable && self.feeder.temporary_bypass_installed && !self.feeder.noncritical_load_shed
        {
            return Some(ActionBlockReason::NoncriticalLoadMustBeShed);
        }
        if !durable && !bypass {
            return Some(ActionBlockReason::RepairOrBypassRequired);
        }
        None
    }

    /// Applies one action and records the resulting causal event.
    pub fn apply(&mut self, action: BentFeederAction) -> Result<String, BentFeederError> {
        if let Some(reason) = self.action_block_reason(&action) {
            return Err(reason.into_error());
        }
        let result = match action.clone() {
            BentFeederAction::Inspect => {
                self.phase = OperationPhase::MakeSafe;
                format!(
                    "support displaced {} mm; insulation damage {}; water depth {} mm",
                    self.feeder.support_displacement_mm,
                    self.feeder.insulation_damage,
                    self.feeder.water_depth_mm
                )
            }
            BentFeederAction::EstablishPerimeter => {
                self.perimeter_established = true;
                self.phase = OperationPhase::MakeSafe;
                "wet-work perimeter established".into()
            }
            BentFeederAction::IsolateUpstream => {
                self.feeder.energized = false;
                self.feeder.isolated = true;
                self.feeder.verified_dead = false;
                self.resident_service.powered = false;
                self.phase = OperationPhase::MakeSafe;
                "upstream isolation opened and locked; critical service is on battery reserve"
                    .into()
            }
            BentFeederAction::VerifyDeEnergized => {
                self.feeder.verified_dead = true;
                self.phase = OperationPhase::RepairOrRefuse;
                "local instrument verified conductors de-energized".into()
            }
            BentFeederAction::BraceSupport => {
                consume(&mut self.inventory, "support-brace")?;
                self.feeder.support_braced = true;
                self.feeder.support_displacement_mm = 6;
                "support braced and displacement reduced".into()
            }
            BentFeederAction::ReplaceInsulator => {
                consume(&mut self.inventory, "insulator-kit")?;
                self.feeder.insulator_replaced = true;
                self.feeder.insulation_damage = 700;
                "damaged insulation assembly replaced".into()
            }
            BentFeederAction::InstallTemporaryBypass => {
                consume(&mut self.inventory, "temporary-bypass")?;
                self.feeder.temporary_bypass_installed = true;
                self.feeder.restricted_service = true;
                "temporary bypass installed with restricted capacity".into()
            }
            BentFeederAction::ShedNoncriticalLoad => {
                self.feeder.noncritical_load_shed = true;
                "non-critical downstream load shed".into()
            }
            BentFeederAction::EnergizeForTest => self.energize_for_test()?,
            BentFeederAction::SafeRefusalAndRelocation => {
                self.feeder.energized = false;
                self.feeder.isolated = true;
                self.resident_service.powered = true;
                self.resident_service.relocated = true;
                self.phase = OperationPhase::Complete;
                self.outcome = Some(OperationOutcome::SafeRefusalAndRelocation);
                "unsafe work refused; resident service relocated to protected supply".into()
            }
            BentFeederAction::UnsafeLiveShortcut => {
                self.feeder.arc_fire = true;
                self.feeder.energized = false;
                self.resident_service.powered = false;
                self.resident_service.harmed =
                    self.feeder.water_depth_mm > 0 || !self.perimeter_established;
                self.phase = OperationPhase::Complete;
                self.outcome = Some(OperationOutcome::ArcFire);
                "live manipulation caused an arc and converter fire".into()
            }
            BentFeederAction::AdvanceTicks(ticks) => self.advance_ticks(ticks),
        };
        self.events
            .append(
                self.tick,
                "bent-feeder.action",
                None,
                None,
                Vec::new(),
                BentFeederEvent {
                    action,
                    resulting_phase: self.phase,
                    result: result.clone(),
                },
            )
            .map_err(BentFeederError::State)?;
        Ok(result)
    }

    fn energize_for_test(&mut self) -> Result<String, BentFeederError> {
        let durable = self.feeder.support_braced && self.feeder.insulator_replaced;
        let bypass = self.feeder.temporary_bypass_installed && self.feeder.noncritical_load_shed;
        self.phase = OperationPhase::Test;
        self.feeder.isolated = false;
        self.feeder.energized = true;
        self.resident_service.powered = true;
        self.feeder.conductor_temperature_deci_c = if durable { 338 } else { 421 };
        self.phase = OperationPhase::Complete;
        if durable {
            self.feeder.restricted_service = false;
            self.outcome = Some(OperationOutcome::DurableRepair);
            Ok("bounded re-energization passed; durable repair entered service".into())
        } else {
            self.feeder.restricted_service = true;
            self.outcome = Some(OperationOutcome::RestrictedBypass);
            Ok("bounded re-energization passed at restricted bypass capacity".into())
        }
    }

    fn advance_ticks(&mut self, ticks: u64) -> String {
        self.tick = self.tick.saturating_add(ticks);
        if !self.resident_service.powered && !self.resident_service.relocated {
            self.resident_service.battery_reserve_ticks = self
                .resident_service
                .battery_reserve_ticks
                .saturating_sub(ticks);
            if self.resident_service.battery_reserve_ticks == 0 {
                self.resident_service.harmed = true;
                if self.outcome.is_none() {
                    self.outcome = Some(OperationOutcome::UnresolvedServiceLoss);
                }
            }
        }
        if self.feeder.temporary_bypass_installed
            && !self.feeder.support_braced
            && self.feeder.energized
            && ticks >= 600
        {
            self.feeder.insulation_damage = self
                .feeder
                .insulation_damage
                .saturating_add(1_500)
                .min(10_000);
            self.feeder.conductor_temperature_deci_c = 612;
            self.feeder.restricted_service = true;
        }
        format!("advanced {ticks} ticks; physical and resident state persisted")
    }
}

fn feedback_cues_for_action(action: &BentFeederAction) -> Vec<FeedbackCue> {
    match action {
        BentFeederAction::Inspect => vec![FeedbackCue::InspectionReadout],
        BentFeederAction::EstablishPerimeter => vec![FeedbackCue::WorkBoundaryConfirmed],
        BentFeederAction::IsolateUpstream => vec![
            FeedbackCue::IsolationStateChanged,
            FeedbackCue::CriticalServiceStatusChanged,
            FeedbackCue::BatteryReserveChanged,
        ],
        BentFeederAction::VerifyDeEnergized => vec![FeedbackCue::VoltageAbsenceConfirmed],
        BentFeederAction::BraceSupport => vec![FeedbackCue::SupportMovementConfirmed],
        BentFeederAction::ReplaceInsulator => vec![FeedbackCue::InsulationReplacementConfirmed],
        BentFeederAction::InstallTemporaryBypass => vec![FeedbackCue::BypassStateChanged],
        BentFeederAction::ShedNoncriticalLoad => vec![FeedbackCue::LoadShedConfirmed],
        BentFeederAction::EnergizeForTest => vec![
            FeedbackCue::ThermalTrendChanged,
            FeedbackCue::CriticalServiceStatusChanged,
            FeedbackCue::OutcomeCommitted,
        ],
        BentFeederAction::SafeRefusalAndRelocation => vec![
            FeedbackCue::CriticalServiceStatusChanged,
            FeedbackCue::OutcomeCommitted,
        ],
        BentFeederAction::UnsafeLiveShortcut => vec![
            FeedbackCue::SafetyIncidentAlarm,
            FeedbackCue::CriticalServiceStatusChanged,
            FeedbackCue::OutcomeCommitted,
        ],
        BentFeederAction::AdvanceTicks(_) => vec![
            FeedbackCue::BatteryReserveChanged,
            FeedbackCue::ThermalTrendChanged,
            FeedbackCue::CriticalServiceStatusChanged,
        ],
    }
}

fn consume(inventory: &mut BTreeMap<String, u32>, item: &str) -> Result<(), BentFeederError> {
    let count = inventory
        .get_mut(item)
        .ok_or_else(|| BentFeederError::MissingPart(item.into()))?;
    if *count == 0 {
        return Err(BentFeederError::MissingPart(item.into()));
    }
    *count -= 1;
    Ok(())
}

/// Invalid or unsafe operation transitions.
#[derive(Debug)]
pub enum BentFeederError {
    /// Action attempted after a terminal outcome.
    AlreadyComplete,
    /// Action violates a required safety sequence.
    UnsafeSequence(&'static str),
    /// Required physical part is unavailable.
    MissingPart(String),
    /// Event-chain persistence failed.
    State(StateError),
}

impl fmt::Display for BentFeederError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyComplete => {
                formatter.write_str("Bent Feeder operation is already complete")
            }
            Self::UnsafeSequence(reason) => {
                write!(formatter, "unsafe Bent Feeder sequence: {reason}")
            }
            Self::MissingPart(part) => write!(formatter, "missing required part: {part}"),
            Self::State(error) => write!(formatter, "Bent Feeder event chain failed: {error}"),
        }
    }
}

impl Error for BentFeederError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(error) => Some(error),
            Self::AlreadyComplete | Self::UnsafeSequence(_) | Self::MissingPart(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_safe(operation: &mut BentFeederOperation) {
        operation.apply(BentFeederAction::Inspect).expect("inspect");
        operation
            .apply(BentFeederAction::EstablishPerimeter)
            .expect("perimeter");
        operation
            .apply(BentFeederAction::IsolateUpstream)
            .expect("isolate");
        operation
            .apply(BentFeederAction::VerifyDeEnergized)
            .expect("verify");
    }

    #[test]
    fn durable_repair_closes_with_parts_and_safe_sequence() {
        let mut operation = BentFeederOperation::canonical(7);
        make_safe(&mut operation);
        operation
            .apply(BentFeederAction::BraceSupport)
            .expect("brace");
        operation
            .apply(BentFeederAction::ReplaceInsulator)
            .expect("replace");
        operation
            .apply(BentFeederAction::EnergizeForTest)
            .expect("test");
        assert_eq!(operation.outcome, Some(OperationOutcome::DurableRepair));
        assert!(operation.resident_service.powered);
        operation.events.verify().expect("event chain verifies");
    }

    #[test]
    fn bypass_requires_load_shedding_and_remains_restricted() {
        let mut operation = BentFeederOperation::canonical(8);
        make_safe(&mut operation);
        operation
            .apply(BentFeederAction::InstallTemporaryBypass)
            .expect("bypass");
        assert!(operation.apply(BentFeederAction::EnergizeForTest).is_err());
        operation
            .apply(BentFeederAction::ShedNoncriticalLoad)
            .expect("shed load");
        operation
            .apply(BentFeederAction::EnergizeForTest)
            .expect("test");
        assert_eq!(operation.outcome, Some(OperationOutcome::RestrictedBypass));
        assert!(operation.feeder.restricted_service);
    }

    #[test]
    fn unsafe_live_shortcut_has_persistent_hazard() {
        let mut operation = BentFeederOperation::canonical(9);
        operation
            .apply(BentFeederAction::UnsafeLiveShortcut)
            .expect("unsafe action resolves");
        assert_eq!(operation.outcome, Some(OperationOutcome::ArcFire));
        assert!(operation.feeder.arc_fire);
        assert!(operation.resident_service.harmed);
    }

    #[test]
    fn safe_refusal_is_a_valid_outcome() {
        let mut operation = BentFeederOperation::canonical(10);
        operation.apply(BentFeederAction::Inspect).expect("inspect");
        operation
            .apply(BentFeederAction::SafeRefusalAndRelocation)
            .expect("relocate");
        assert_eq!(
            operation.outcome,
            Some(OperationOutcome::SafeRefusalAndRelocation)
        );
        assert!(operation.resident_service.relocated);
        assert!(!operation.resident_service.harmed);
    }

    #[test]
    fn action_preview_is_pure_and_names_the_first_fixable_prerequisite() {
        let mut operation = BentFeederOperation::canonical(17);
        let before = operation.clone();

        let preview = operation.preview_action(&BentFeederAction::BraceSupport);
        assert_eq!(
            preview.readiness,
            ActionReadiness::Blocked {
                reason: ActionBlockReason::WorkPerimeterRequired,
            }
        );
        assert!(
            preview
                .feedback_cues
                .contains(&FeedbackCue::SupportMovementConfirmed)
        );
        assert_eq!(
            operation, before,
            "preview must not mutate state or history"
        );

        operation
            .apply(BentFeederAction::EstablishPerimeter)
            .expect("establish perimeter");
        assert_eq!(
            operation
                .preview_action(&BentFeederAction::IsolateUpstream)
                .readiness,
            ActionReadiness::Ready
        );
        operation
            .apply(BentFeederAction::IsolateUpstream)
            .expect("isolate upstream");
        assert_eq!(
            operation
                .preview_action(&BentFeederAction::VerifyDeEnergized)
                .readiness,
            ActionReadiness::Ready
        );
    }

    #[test]
    fn preview_and_apply_share_bypass_load_shedding_gate() {
        let mut operation = BentFeederOperation::canonical(18);
        make_safe(&mut operation);
        operation
            .apply(BentFeederAction::InstallTemporaryBypass)
            .expect("install bypass");
        let before = operation.clone();

        assert_eq!(
            operation
                .preview_action(&BentFeederAction::EnergizeForTest)
                .readiness,
            ActionReadiness::Blocked {
                reason: ActionBlockReason::NoncriticalLoadMustBeShed,
            }
        );
        assert!(operation.apply(BentFeederAction::EnergizeForTest).is_err());
        assert_eq!(
            operation, before,
            "blocked actions must not partially apply"
        );

        operation
            .apply(BentFeederAction::ShedNoncriticalLoad)
            .expect("shed non-critical load");
        assert_eq!(
            operation
                .preview_action(&BentFeederAction::EnergizeForTest)
                .readiness,
            ActionReadiness::Ready
        );
    }

    #[test]
    fn critical_feedback_cues_have_multiple_required_channels() {
        let cues = [
            FeedbackCue::InspectionReadout,
            FeedbackCue::WorkBoundaryConfirmed,
            FeedbackCue::IsolationStateChanged,
            FeedbackCue::VoltageAbsenceConfirmed,
            FeedbackCue::SupportMovementConfirmed,
            FeedbackCue::InsulationReplacementConfirmed,
            FeedbackCue::BypassStateChanged,
            FeedbackCue::LoadShedConfirmed,
            FeedbackCue::ThermalTrendChanged,
            FeedbackCue::CriticalServiceStatusChanged,
            FeedbackCue::SafetyIncidentAlarm,
            FeedbackCue::BatteryReserveChanged,
            FeedbackCue::OutcomeCommitted,
        ];
        for cue in cues {
            assert!(
                cue.minimum_channels().len() >= 2,
                "{cue:?} must not depend on one channel"
            );
            assert!(cue.minimum_channels().contains(&FeedbackChannel::Visual));
            assert!(cue.minimum_channels().contains(&FeedbackChannel::Audio));
        }
        assert!(
            FeedbackCue::SupportMovementConfirmed
                .optional_channels()
                .contains(&FeedbackChannel::Haptic)
        );
    }

    #[test]
    fn completed_operation_previews_terminal_block_without_mutation() {
        let mut operation = BentFeederOperation::canonical(19);
        operation
            .apply(BentFeederAction::UnsafeLiveShortcut)
            .expect("unsafe action resolves");
        let before = operation.clone();

        assert_eq!(
            operation
                .preview_action(&BentFeederAction::Inspect)
                .readiness,
            ActionReadiness::Blocked {
                reason: ActionBlockReason::OperationComplete,
            }
        );
        assert_eq!(operation, before);
    }
}

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

    /// Applies one action and records the resulting causal event.
    pub fn apply(&mut self, action: BentFeederAction) -> Result<String, BentFeederError> {
        if self.phase == OperationPhase::Complete {
            return Err(BentFeederError::AlreadyComplete);
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
                if !self.perimeter_established {
                    return Err(BentFeederError::UnsafeSequence(
                        "establish the work perimeter before operating isolation",
                    ));
                }
                self.feeder.energized = false;
                self.feeder.isolated = true;
                self.feeder.verified_dead = false;
                self.resident_service.powered = false;
                self.phase = OperationPhase::MakeSafe;
                "upstream isolation opened and locked; critical service is on battery reserve"
                    .into()
            }
            BentFeederAction::VerifyDeEnergized => {
                if !self.feeder.isolated || self.feeder.energized {
                    return Err(BentFeederError::UnsafeSequence(
                        "isolate upstream before verifying absence of voltage",
                    ));
                }
                self.feeder.verified_dead = true;
                self.phase = OperationPhase::RepairOrRefuse;
                "local instrument verified conductors de-energized".into()
            }
            BentFeederAction::BraceSupport => {
                self.require_safe_work_state()?;
                consume(&mut self.inventory, "support-brace")?;
                self.feeder.support_braced = true;
                self.feeder.support_displacement_mm = 6;
                "support braced and displacement reduced".into()
            }
            BentFeederAction::ReplaceInsulator => {
                self.require_safe_work_state()?;
                consume(&mut self.inventory, "insulator-kit")?;
                self.feeder.insulator_replaced = true;
                self.feeder.insulation_damage = 700;
                "damaged insulation assembly replaced".into()
            }
            BentFeederAction::InstallTemporaryBypass => {
                self.require_safe_work_state()?;
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

    fn require_safe_work_state(&self) -> Result<(), BentFeederError> {
        if self.perimeter_established
            && self.feeder.isolated
            && self.feeder.verified_dead
            && !self.feeder.energized
        {
            Ok(())
        } else {
            Err(BentFeederError::UnsafeSequence(
                "repair requires perimeter, isolation, and local de-energized verification",
            ))
        }
    }

    fn energize_for_test(&mut self) -> Result<String, BentFeederError> {
        if !self.feeder.isolated || !self.feeder.verified_dead {
            return Err(BentFeederError::UnsafeSequence(
                "test requires a verified isolation history",
            ));
        }
        let durable = self.feeder.support_braced && self.feeder.insulator_replaced;
        let bypass = self.feeder.temporary_bypass_installed && self.feeder.noncritical_load_shed;
        if !durable && !bypass {
            return Err(BentFeederError::UnsafeSequence(
                "no complete repair or load-shed bypass configuration is ready for test",
            ));
        }
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
}

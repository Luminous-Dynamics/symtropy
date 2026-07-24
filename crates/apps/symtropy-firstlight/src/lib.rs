// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authoritative headless composition of the Firstlight product lane.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_crawler_core::{
    CargoItem, CrawlerError, CrawlerState, RouteQualification, RouteSegment,
};
use symtropy_firstlight_bent_feeder::{
    BentFeederAction, BentFeederError, BentFeederOperation, OperationOutcome,
};
use symtropy_firstlight_catastrophe::{
    FirstlightCatastrophe, InfrastructureKind, ObservationDomain, Preparation,
};
use symtropy_game_state::{EventChain, SimulationClock, StableId, StateError};
use symtropy_iris::{Hypothesis, IrisAssessment, IrisState};
use symtropy_persistence::{PersistenceError, SaveSnapshot, SaveStore};
use symtropy_residents::{
    ClaimPrivacy, DisclosureContext, KnowledgeBase, KnowledgeClaim, Obligation, Resident,
    ResidentContext, ResidentIntent, ResidentNeed,
};

/// Product content version written into snapshots and evidence.
pub const FIRSTLIGHT_CONTENT_VERSION: &str = "firstlight-product-spine-v0.1";

/// Top-level progression state for the opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningPhase {
    /// Ordinary life and repair work before the acute catastrophe.
    BeforeBreaking,
    /// Coupled infrastructure failure is active.
    Breaking,
    /// Loading, route qualification, and departure are active.
    ContinuanceDeparture,
    /// First camp or stationary refuge state after the title transition.
    PostTitleContinuity,
}

/// Cross-system event recorded by the authoritative application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstlightEvent {
    /// Concise machine-readable kind.
    pub kind: String,
    /// Human- and tool-readable summary.
    pub summary: String,
}

/// Serializable authoritative world snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstlightWorld {
    /// Deterministic scenario seed.
    pub seed: u64,
    /// Fixed-step simulation time.
    pub clock: SimulationClock,
    /// Current opening phase.
    pub phase: OpeningPhase,
    /// Bent Feeder operation state.
    pub bent_feeder: BentFeederOperation,
    /// FC-1 catastrophe state.
    pub catastrophe: FirstlightCatastrophe,
    /// CLV-7 Crawler state.
    pub crawler: CrawlerState,
    /// Persistent residents by identity.
    pub residents: BTreeMap<StableId, Resident>,
    /// Bounded assistant state.
    pub iris: IrisState,
}

/// Authoritative headless session with one cross-system causal chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstlightSession {
    /// World state.
    pub world: FirstlightWorld,
    /// Cross-system event chain.
    pub events: EventChain<FirstlightEvent>,
}

impl FirstlightSession {
    /// Creates a deterministic reference opening.
    pub fn canonical(seed: u64) -> Result<Self, StateError> {
        let resident = canonical_resident(seed);
        let resident_id = resident.id.clone();
        let iris_id = StableId::derive("iris", seed, 0);
        let mut residents = BTreeMap::new();
        residents.insert(resident_id, resident);
        let mut session = Self {
            world: FirstlightWorld {
                seed,
                clock: SimulationClock::from_hz(20)?,
                phase: OpeningPhase::BeforeBreaking,
                bent_feeder: BentFeederOperation::canonical(seed),
                catastrophe: FirstlightCatastrophe::canonical(seed),
                crawler: CrawlerState::reference(seed),
                residents,
                iris: IrisState {
                    id: iris_id.clone(),
                    knowledge: KnowledgeBase::new(iris_id),
                    permissions: BTreeSet::new(),
                    hypotheses: BTreeMap::new(),
                },
            },
            events: EventChain::new("firstlight-session", seed),
        };
        session.refresh_iris_from_bent_feeder();
        session.record("session.created", "canonical Firstlight session created")?;
        Ok(session)
    }

    /// Applies one Bent Feeder action and propagates its persistent consequences.
    pub fn apply_bent_feeder(
        &mut self,
        action: BentFeederAction,
    ) -> Result<String, FirstlightError> {
        let result = self
            .world
            .bent_feeder
            .apply(action)
            .map_err(FirstlightError::BentFeeder)?;
        self.refresh_iris_from_bent_feeder();
        if let Some(outcome) = self.world.bent_feeder.outcome {
            self.apply_feeder_continuity(outcome);
        }
        self.record("bent-feeder.action", &result)
            .map_err(FirstlightError::State)?;
        Ok(result)
    }

    /// Applies a pre-catastrophe preparation.
    pub fn prepare(&mut self, preparation: Preparation) -> Result<(), FirstlightError> {
        self.world
            .catastrophe
            .prepare(preparation)
            .map_err(FirstlightError::State)?;
        self.record(
            "firstlight.preparation",
            &format!("completed {preparation:?}"),
        )
        .map_err(FirstlightError::State)
    }

    /// Advances every active world system, including off-screen catastrophe state.
    pub fn advance(
        &mut self,
        ticks: u64,
    ) -> Result<Vec<(StableId, ResidentIntent)>, FirstlightError> {
        self.world
            .clock
            .advance_by(ticks)
            .map_err(FirstlightError::State)?;
        if self.world.bent_feeder.outcome.is_none() {
            self.world
                .bent_feeder
                .apply(BentFeederAction::AdvanceTicks(ticks))
                .map_err(FirstlightError::BentFeeder)?;
        }
        self.world
            .catastrophe
            .advance(ticks)
            .map_err(FirstlightError::State)?;
        self.world.phase = if self.world.catastrophe.tick >= 100 {
            OpeningPhase::Breaking
        } else {
            self.world.phase
        };
        let context = ResidentContext {
            current_tick: self.world.clock.tick(),
            lookahead_ticks: 600,
            urgent_need_threshold: 8_000,
        };
        let intents = self
            .world
            .residents
            .values()
            .map(|resident| (resident.id.clone(), resident.choose_intent(&context)))
            .collect::<Vec<_>>();
        self.record("session.advance", &format!("advanced {ticks} ticks"))
            .map_err(FirstlightError::State)?;
        Ok(intents)
    }

    /// Returns a bounded IRIS assessment of the feeder fault.
    pub fn assess_bent_feeder(&self) -> IrisAssessment {
        let mut consented_claim_ids = BTreeSet::new();
        consented_claim_ids.insert(StableId::derive("claim-feeder", self.world.seed, 2));
        self.world.iris.assess_subject(
            &StableId::derive("fault-bent-feeder", self.world.seed, 0),
            self.world.clock.tick(),
            &DisclosureContext {
                requester_id: StableId::derive("player", self.world.seed, 0),
                requester_household_id: None,
                consented_claim_ids,
                life_safety_emergency: self.world.phase == OpeningPhase::Breaking,
            },
        )
    }

    /// Loads physical cargo into the Crawler and records custody.
    pub fn load_crawler_cargo(&mut self, cargo: CargoItem) -> Result<(), FirstlightError> {
        let summary = format!(
            "loaded {} kg cargo {} under custody {}",
            cargo.mass_kg, cargo.id, cargo.custodian_id
        );
        self.world
            .crawler
            .load_cargo(cargo)
            .map_err(FirstlightError::Crawler)?;
        self.record("crawler.cargo.loaded", &summary)
            .map_err(FirstlightError::State)
    }

    /// Qualifies the Crawler against a physical route segment.
    pub fn qualify_departure_route(
        &mut self,
        route: &RouteSegment,
    ) -> Result<RouteQualification, FirstlightError> {
        let result = self.world.crawler.qualify_route(route);
        self.record(
            "crawler.route.qualified",
            &format!(
                "route {} classified {:?}: {}",
                route.id,
                result.class,
                result.reasons.join("; ")
            ),
        )
        .map_err(FirstlightError::State)?;
        Ok(result)
    }

    /// Produces a versioned snapshot anchored to the cross-system event chain.
    pub fn snapshot(&self) -> SaveSnapshot<FirstlightWorld> {
        SaveSnapshot::new(
            StableId::derive("save-firstlight", self.world.seed, 0),
            FIRSTLIGHT_CONTENT_VERSION,
            self.world.clock.tick(),
            self.events.head_hash(),
            self.world.clone(),
        )
    }

    /// Writes a snapshot and complete cross-system journal to an empty evidence store.
    pub fn write_evidence(&self, store: &SaveStore) -> Result<(), FirstlightError> {
        store
            .write_snapshot(&self.snapshot())
            .map_err(FirstlightError::Persistence)?;
        for event in self.events.events() {
            store
                .append_event(event)
                .map_err(FirstlightError::Persistence)?;
        }
        Ok(())
    }

    /// Moves the product state into the departure lane after Crawler safety checks.
    pub fn begin_departure(&mut self) -> Result<(), FirstlightError> {
        if !self.world.crawler.unsecured_cargo().is_empty() {
            return Err(FirstlightError::DepartureBlocked(
                "Crawler contains unsecured cargo".into(),
            ));
        }
        if self.world.crawler.occupants > self.world.crawler.profile.evacuation_positions {
            return Err(FirstlightError::DepartureBlocked(
                "occupants exceed safe evacuation positions".into(),
            ));
        }
        self.world.phase = OpeningPhase::ContinuanceDeparture;
        self.record("departure.started", "Crawler departure lane entered")
            .map_err(FirstlightError::State)
    }

    fn apply_feeder_continuity(&mut self, outcome: OperationOutcome) {
        let Some(grid) = self
            .world
            .catastrophe
            .infrastructure
            .get_mut(&InfrastructureKind::Grid)
        else {
            return;
        };
        match outcome {
            OperationOutcome::DurableRepair => {
                grid.health = grid.health.saturating_add(500).min(10_000);
                grid.load = grid.load.saturating_sub(400);
            }
            OperationOutcome::RestrictedBypass => {
                grid.local_control = true;
                grid.load = grid.load.saturating_sub(900);
            }
            OperationOutcome::SafeRefusalAndRelocation => {
                grid.load = grid.load.saturating_sub(300);
            }
            OperationOutcome::UnresolvedServiceLoss => {
                grid.health = grid.health.saturating_sub(300);
            }
            OperationOutcome::ArcFire => {
                grid.health = grid.health.saturating_sub(1_800);
                grid.failed = grid.health < 2_500;
            }
        }
    }

    fn refresh_iris_from_bent_feeder(&mut self) {
        let subject = StableId::derive("fault-bent-feeder", self.world.seed, 0);
        let feeder = &self.world.bent_feeder.feeder;
        for (ordinal, proposition, confidence, privacy) in [
            (
                0,
                format!(
                    "support displacement measured at {} mm",
                    feeder.support_displacement_mm
                ),
                9_200,
                ClaimPrivacy::Public,
            ),
            (
                1,
                format!(
                    "insulation damage index is approximately {} of 10000",
                    feeder.insulation_damage
                ),
                7_600,
                ClaimPrivacy::Public,
            ),
            (
                2,
                "a resident depends on continuity of protected electrical service".into(),
                10_000,
                ClaimPrivacy::ConsentRequired,
            ),
        ] {
            self.world.iris.knowledge.remember(KnowledgeClaim {
                id: StableId::derive("claim-feeder", self.world.seed, ordinal),
                subject_id: subject.clone(),
                proposition,
                confidence,
                source_id: StableId::derive("observer-feeder", self.world.seed, ordinal),
                observed_tick: self.world.clock.tick(),
                stale_after_tick: Some(self.world.clock.tick().saturating_add(240)),
                privacy,
            });
        }
        self.world.iris.hypotheses.insert(
            StableId::derive("hypothesis-feeder", self.world.seed, 0),
            Hypothesis {
                id: StableId::derive("hypothesis-feeder", self.world.seed, 0),
                subject_id: subject,
                proposition: "support movement is the leading cause of insulation damage".into(),
                confidence: 7_200,
                supporting_claim_ids: BTreeSet::from([
                    StableId::derive("claim-feeder", self.world.seed, 0),
                    StableId::derive("claim-feeder", self.world.seed, 1),
                ]),
                contradicting_claim_ids: BTreeSet::new(),
                recommended_observation: Some(
                    "isolate locally and inspect the support-to-insulator load path".into(),
                ),
            },
        );
    }

    fn record(&mut self, kind: &str, summary: &str) -> Result<(), StateError> {
        self.events.append(
            self.world.clock.tick(),
            kind,
            None,
            None,
            Vec::new(),
            FirstlightEvent {
                kind: kind.into(),
                summary: summary.into(),
            },
        )?;
        Ok(())
    }
}

fn canonical_resident(seed: u64) -> Resident {
    let id = StableId::derive("resident", seed, 0);
    Resident {
        id: id.clone(),
        name: "Mina Adebayo".into(),
        household_id: Some(StableId::derive("household", seed, 0)),
        location_id: StableId::derive("place-bent-feeder", seed, 0),
        capabilities: BTreeSet::from([
            "self-advocacy".into(),
            "local-infrastructure-knowledge".into(),
        ]),
        needs: vec![ResidentNeed {
            kind: "powered-respiratory-support".into(),
            urgency: 9_200,
            preferred_location: Some(StableId::derive("service-clinic-power", seed, 0)),
            consequence: "battery exhaustion interrupts respiratory support".into(),
        }],
        obligations: vec![Obligation {
            id: StableId::derive("obligation", seed, 0),
            target_id: StableId::derive("household", seed, 0),
            due_tick: 800,
            priority: 8_800,
            reason: "remain reachable to a younger household member during evacuation".into(),
        }],
        knowledge: KnowledgeBase::new(id),
        conscious_and_mobile: true,
    }
}

/// Cross-system product integration failure.
#[derive(Debug)]
pub enum FirstlightError {
    /// Deterministic state or event failure.
    State(StateError),
    /// Bent Feeder action failure.
    BentFeeder(BentFeederError),
    /// Crawler physical-state failure.
    Crawler(CrawlerError),
    /// Snapshot or journal failure.
    Persistence(PersistenceError),
    /// Departure gate rejected current state.
    DepartureBlocked(String),
}

impl std::fmt::Display for FirstlightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(error) => write!(formatter, "Firstlight state failed: {error}"),
            Self::BentFeeder(error) => write!(formatter, "Bent Feeder failed: {error}"),
            Self::Crawler(error) => write!(formatter, "Crawler failed: {error}"),
            Self::Persistence(error) => write!(formatter, "Firstlight persistence failed: {error}"),
            Self::DepartureBlocked(reason) => write!(formatter, "departure blocked: {reason}"),
        }
    }
}

impl std::error::Error for FirstlightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::State(error) => Some(error),
            Self::BentFeeder(error) => Some(error),
            Self::Crawler(error) => Some(error),
            Self::Persistence(error) => Some(error),
            Self::DepartureBlocked(_) => None,
        }
    }
}

/// Canonical bridge-cribbing cargo used by the departure proof.
pub fn canonical_bridge_cribbing(seed: u64) -> CargoItem {
    CargoItem {
        id: StableId::derive("cargo-bridge-cribbing", seed, 0),
        mass_kg: 5_100,
        volume_l: 14_000,
        longitudinal_mm: 29_200,
        lateral_mm: 0,
        vertical_mm: 1_100,
        restraint_rating_n: 240_000,
        secured: false,
        custodian_id: StableId::derive("continuity-engineering-crew", seed, 0),
        access_priority: "en-route".into(),
    }
}

/// Canonical east floodway route segment.
pub fn canonical_service_span(seed: u64) -> RouteSegment {
    RouteSegment {
        id: StableId::derive("route-service-span-4", seed, 0),
        clear_width_mm: 8_200,
        clear_height_mm: 20_000,
        bearing_capacity_pa: 92_000,
        cross_slope_millirad: 30,
        grade_millirad: 10,
        confidence: 4_600,
    }
}

/// Runs the deterministic reference sequence used by the CLI and proof harness.
pub fn run_reference_sequence(seed: u64) -> Result<FirstlightSession, FirstlightError> {
    let mut session = FirstlightSession::canonical(seed).map_err(FirstlightError::State)?;
    for preparation in [
        Preparation::LocalClockFallback,
        Preparation::PumpManualTeams,
        Preparation::DrainageClearance,
    ] {
        session.prepare(preparation)?;
    }
    for action in [
        BentFeederAction::Inspect,
        BentFeederAction::EstablishPerimeter,
        BentFeederAction::IsolateUpstream,
        BentFeederAction::VerifyDeEnergized,
        BentFeederAction::BraceSupport,
        BentFeederAction::ReplaceInsulator,
        BentFeederAction::EnergizeForTest,
    ] {
        session.apply_bent_feeder(action)?;
    }
    session.advance(120)?;
    let cargo = canonical_bridge_cribbing(seed);
    let cargo_id = cargo.id.clone();
    session.load_crawler_cargo(cargo)?;
    session
        .world
        .crawler
        .secure_cargo(&cargo_id)
        .map_err(FirstlightError::Crawler)?;
    session.qualify_departure_route(&canonical_service_span(seed))?;
    session.begin_departure()?;
    let _ = session
        .world
        .catastrophe
        .observe(ObservationDomain::VisibleOptical, 0);
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use symtropy_persistence::JournalLoad;

    #[test]
    fn reference_sequence_is_deterministic() {
        let first = run_reference_sequence(42).expect("first run");
        let second = run_reference_sequence(42).expect("second run");
        assert_eq!(first, second);
        first.events.verify().expect("session event chain verifies");
    }

    #[test]
    fn feeder_choice_changes_catastrophe_grid_state() {
        let mut safe = FirstlightSession::canonical(7).expect("session");
        let mut fire = FirstlightSession::canonical(7).expect("session");
        for action in [
            BentFeederAction::Inspect,
            BentFeederAction::EstablishPerimeter,
            BentFeederAction::IsolateUpstream,
            BentFeederAction::VerifyDeEnergized,
            BentFeederAction::BraceSupport,
            BentFeederAction::ReplaceInsulator,
            BentFeederAction::EnergizeForTest,
        ] {
            safe.apply_bent_feeder(action).expect("safe action");
        }
        fire.apply_bent_feeder(BentFeederAction::UnsafeLiveShortcut)
            .expect("unsafe action resolves");
        let safe_health = safe.world.catastrophe.infrastructure[&InfrastructureKind::Grid].health;
        let fire_health = fire.world.catastrophe.infrastructure[&InfrastructureKind::Grid].health;
        assert!(safe_health > fire_health);
    }

    #[test]
    fn evidence_snapshot_and_journal_recover() {
        let session = run_reference_sequence(99).expect("reference sequence");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("symtropy-firstlight-evidence-{nonce}"));
        let store = SaveStore::open(&root).expect("create evidence store");
        session.write_evidence(&store).expect("write evidence");
        let snapshot: SaveSnapshot<FirstlightWorld> = store.read_snapshot().expect("read snapshot");
        let journal: JournalLoad<FirstlightEvent> = store
            .load_journal("firstlight-session", 99)
            .expect("read journal");
        store
            .verify_snapshot_anchor(&snapshot, &journal)
            .expect("anchor exists");
        assert_eq!(snapshot.state, session.world);
        fs::remove_dir_all(root).expect("remove evidence store");
    }

    #[test]
    fn iris_assessment_is_bounded_not_a_solution_marker() {
        let session = FirstlightSession::canonical(101).expect("session");
        let assessment = session.assess_bent_feeder();
        assert!(!assessment.refused);
        assert!(!assessment.evidence_claim_ids.is_empty());
        assert!(
            assessment
                .uncertainty
                .iter()
                .any(|line| line.contains("observation"))
        );
    }
}

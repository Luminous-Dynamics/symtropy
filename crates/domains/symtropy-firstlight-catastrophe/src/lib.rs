// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic FC-1 Rain Curvature field, observations, preparations, and failures.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_game_state::{EventChain, StableId, StateError};

/// Canonical catastrophe identifier.
pub const CATASTROPHE_ID: &str = "firstlight.rain-curvature.fc-1";

/// Authored preparation that changes the catastrophe without preventing all loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Preparation {
    /// District systems can operate from trusted local clocks.
    LocalClockFallback,
    /// Pump crews are assigned and trained for manual local operation.
    PumpManualTeams,
    /// Heavy traffic is removed from the east viaduct before the acute phase.
    ReduceViaductLoad,
    /// East drainage and wetland overflow paths are cleared.
    DrainageClearance,
    /// Raw observations are mirrored outside disputed authority systems.
    EvidenceMirrors,
    /// The Continuance Crawler begins departure with warm systems and charged reserves.
    CrawlerPreheat,
}

/// Coupled infrastructure domain affected by FC-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InfrastructureKind {
    /// Bent Feeder and local grid coordination.
    Grid,
    /// Basin Pump Court and drainage control.
    Water,
    /// East viaduct and continuity corridors.
    Transit,
    /// Civic fibre, mesh, and time distribution.
    Communications,
    /// Continuance yard and Crawler departure readiness.
    CrawlerYard,
}

/// Threshold-preserving state for one infrastructure domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfrastructureState {
    /// Health from 0 to 10,000.
    pub health: u16,
    /// Current load from 0 to 10,000.
    pub load: u16,
    /// Quality of remote synchronization from 0 to 10,000.
    pub synchronization_quality: u16,
    /// Whether safe local control has been established.
    pub local_control: bool,
    /// Whether the domain has crossed a mission-changing failure threshold.
    pub failed: bool,
}

/// Explicitly speculative field with bounded measurable side effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RainCurvatureField {
    /// Leading edge distance from the wetland reference origin.
    pub leading_edge_m: i32,
    /// Spatial extent of the strongest gradient.
    pub extent_m: u32,
    /// Effective lateral acceleration change in micrometres per second squared.
    pub lateral_acceleration_um_s2: i32,
    /// Relative clock offset across the active district in microseconds.
    pub clock_offset_us: i64,
    /// Round-trip delay residual in microseconds.
    pub network_delay_residual_us: i64,
    /// Rainfall intensity in tenths of a millimetre per hour.
    pub rainfall_deci_mm_h: u16,
}

impl RainCurvatureField {
    fn canonical(seed: u64) -> Self {
        let seed_bias = i32::try_from(seed % 17).unwrap_or(0) - 8;
        Self {
            leading_edge_m: -2_800,
            extent_m: 1_840,
            lateral_acceleration_um_s2: 18_000 + seed_bias * 250,
            clock_offset_us: -24_000,
            network_delay_residual_us: 31_000,
            rainfall_deci_mm_h: 286,
        }
    }
}

/// Observer domain for non-omniscient measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationDomain {
    /// Stereo optical tracking of falling water.
    VisibleOptical,
    /// Structural or vehicle inertial measurement.
    Inertial,
    /// Fibre and clock-relation measurement.
    ChronometricNetwork,
    /// Human report from ordinary situated experience.
    CommunityWitness,
}

/// Measurement or witness report available to an observer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatastropheObservation {
    /// Stable observation identity.
    pub id: StableId,
    /// Observer or team that produced the record.
    pub observer_id: StableId,
    /// Instrument, witness, or data source.
    pub instrument_id: StableId,
    /// Measurement domain.
    pub domain: ObservationDomain,
    /// Observer-local tick.
    pub local_tick: u64,
    /// Quantity being reported.
    pub quantity: String,
    /// Signed integer value in the declared unit.
    pub value: i64,
    /// Unit text.
    pub unit: String,
    /// Confidence from 0 to 10,000.
    pub confidence: u16,
    /// Known limitations and disagreements.
    pub quality_flags: Vec<String>,
}

/// Catastrophe progression phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatastrophePhase {
    /// Strange but locally manageable precursor observations.
    Precursor,
    /// Infrastructure begins disagreeing and crossing stressed thresholds.
    CoordinationFailure,
    /// Multiple coupled fronts require independent local action.
    AcuteBreaking,
    /// Survivors and systems continue under irreversible changed conditions.
    Aftermath,
}

/// Event payload for replay and causal explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatastropheEvent {
    /// Event summary.
    pub summary: String,
    /// Field offset at this point.
    pub clock_offset_us: i64,
    /// Domains that have failed.
    pub failed_domains: Vec<InfrastructureKind>,
}

/// Complete headless catastrophe state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstlightCatastrophe {
    /// Deterministic scenario seed.
    pub seed: u64,
    /// Authoritative simulation tick.
    pub tick: u64,
    /// Current phase.
    pub phase: CatastrophePhase,
    /// Speculative field state.
    pub field: RainCurvatureField,
    /// Coupled infrastructure state.
    pub infrastructure: BTreeMap<InfrastructureKind, InfrastructureState>,
    /// Preparations completed before or during the event.
    pub preparations: BTreeSet<Preparation>,
    /// Water level above the east drainage target in millimetres.
    pub east_drainage_excess_mm: u32,
    /// Event history.
    pub events: EventChain<CatastropheEvent>,
}

impl FirstlightCatastrophe {
    /// Creates the canonical FC-1 reference state.
    pub fn canonical(seed: u64) -> Self {
        Self {
            seed,
            tick: 0,
            phase: CatastrophePhase::Precursor,
            field: RainCurvatureField::canonical(seed),
            infrastructure: BTreeMap::from([
                (InfrastructureKind::Grid, domain(9_200, 6_600)),
                (InfrastructureKind::Water, domain(9_000, 6_800)),
                (InfrastructureKind::Transit, domain(8_700, 7_300)),
                (InfrastructureKind::Communications, domain(9_300, 5_800)),
                (InfrastructureKind::CrawlerYard, domain(9_500, 4_700)),
            ]),
            preparations: BTreeSet::new(),
            east_drainage_excess_mm: 0,
            events: EventChain::new(CATASTROPHE_ID, seed),
        }
    }

    /// Applies an authored preparation while preserving its event provenance.
    pub fn prepare(&mut self, preparation: Preparation) -> Result<(), StateError> {
        self.preparations.insert(preparation);
        match preparation {
            Preparation::LocalClockFallback => {
                for kind in [
                    InfrastructureKind::Grid,
                    InfrastructureKind::Water,
                    InfrastructureKind::Communications,
                ] {
                    if let Some(state) = self.infrastructure.get_mut(&kind) {
                        state.local_control = true;
                    }
                }
            }
            Preparation::ReduceViaductLoad => {
                if let Some(transit) = self.infrastructure.get_mut(&InfrastructureKind::Transit) {
                    transit.load = transit.load.saturating_sub(2_500);
                }
            }
            Preparation::CrawlerPreheat => {
                if let Some(yard) = self
                    .infrastructure
                    .get_mut(&InfrastructureKind::CrawlerYard)
                {
                    yard.health = yard.health.saturating_add(300).min(10_000);
                    yard.load = yard.load.saturating_sub(800);
                }
            }
            Preparation::PumpManualTeams
            | Preparation::DrainageClearance
            | Preparation::EvidenceMirrors => {}
        }
        self.record(format!("preparation completed: {preparation:?}"))
    }

    /// Advances the world off-screen as well as near the player.
    pub fn advance(&mut self, ticks: u64) -> Result<(), StateError> {
        for _ in 0..ticks {
            self.tick = self.tick.saturating_add(1);
            self.field.leading_edge_m = self.field.leading_edge_m.saturating_add(7);
            self.field.lateral_acceleration_um_s2 =
                self.field.lateral_acceleration_um_s2.saturating_add(42);
            self.field.clock_offset_us = self.field.clock_offset_us.saturating_sub(1_850);
            self.field.network_delay_residual_us =
                self.field.network_delay_residual_us.saturating_add(2_900);
            self.field.rainfall_deci_mm_h =
                self.field.rainfall_deci_mm_h.saturating_add(1).min(620);
            self.update_phase();
            self.update_infrastructure();
        }
        self.record(format!("catastrophe advanced by {ticks} ticks"))
    }

    /// Produces an observer-specific measurement with deterministic disagreement.
    pub fn observe(&self, domain: ObservationDomain, ordinal: u64) -> CatastropheObservation {
        let local_bias = match domain {
            ObservationDomain::VisibleOptical => 3,
            ObservationDomain::Inertial => -2,
            ObservationDomain::ChronometricNetwork => 7,
            ObservationDomain::CommunityWitness => -11,
        };
        let (quantity, value, unit, confidence, flags) = match domain {
            ObservationDomain::VisibleOptical => (
                "rain_lateral_acceleration",
                i64::from(self.field.lateral_acceleration_um_s2 + local_bias * 210),
                "um/s2",
                9_100,
                vec!["heavy-rain".into(), "lens-heater-active".into()],
            ),
            ObservationDomain::Inertial => (
                "effective_lateral_acceleration",
                i64::from(self.field.lateral_acceleration_um_s2 + local_bias * 760),
                "um/s2",
                8_400,
                vec![
                    "support-vibration-present".into(),
                    "clock-relation-degraded".into(),
                ],
            ),
            ObservationDomain::ChronometricNetwork => (
                "round_trip_delay_residual",
                self.field.network_delay_residual_us + i64::from(local_bias * 1_200),
                "us",
                7_800,
                vec![
                    "path-asymmetry-suspected".into(),
                    "remote-clock-untrusted".into(),
                ],
            ),
            ObservationDomain::CommunityWitness => (
                "pump_sound_arrival_order_anomaly",
                i64::from(local_bias),
                "ordinal-report",
                5_600,
                vec!["retrospective-report".into()],
            ),
        };
        CatastropheObservation {
            id: StableId::derive("fc1-observation", self.seed, ordinal),
            observer_id: StableId::derive("fc1-observer", self.seed, domain as u64),
            instrument_id: StableId::derive("fc1-instrument", self.seed, domain as u64),
            domain,
            local_tick: self.tick.saturating_add_signed(i64::from(local_bias)),
            quantity: quantity.into(),
            value,
            unit: unit.into(),
            confidence,
            quality_flags: flags,
        }
    }

    /// Returns all domains currently beyond their mission-changing threshold.
    pub fn failed_domains(&self) -> Vec<InfrastructureKind> {
        self.infrastructure
            .iter()
            .filter_map(|(kind, state)| state.failed.then_some(*kind))
            .collect()
    }

    fn update_phase(&mut self) {
        self.phase = match self.tick {
            0..=39 => CatastrophePhase::Precursor,
            40..=99 => CatastrophePhase::CoordinationFailure,
            100..=219 => CatastrophePhase::AcuteBreaking,
            _ => CatastrophePhase::Aftermath,
        };
    }

    fn update_infrastructure(&mut self) {
        let clock_stress = u16::try_from(self.field.clock_offset_us.unsigned_abs() / 25_000)
            .unwrap_or(u16::MAX)
            .min(700);
        let gradient_stress = u16::try_from(self.field.lateral_acceleration_um_s2.max(0) / 700)
            .unwrap_or(u16::MAX)
            .min(700);

        for (kind, state) in &mut self.infrastructure {
            let local_multiplier = if state.local_control { 1 } else { 3 };
            state.synchronization_quality = state
                .synchronization_quality
                .saturating_sub(clock_stress.saturating_mul(local_multiplier));
            let damage = match kind {
                InfrastructureKind::Grid => {
                    clock_stress.saturating_mul(if state.local_control { 1 } else { 2 })
                }
                InfrastructureKind::Water => {
                    let manual = self.preparations.contains(&Preparation::PumpManualTeams);
                    clock_stress.saturating_mul(if manual { 1 } else { 2 })
                }
                InfrastructureKind::Transit => {
                    if self.preparations.contains(&Preparation::ReduceViaductLoad) {
                        gradient_stress / 4 + state.load / 200
                    } else {
                        gradient_stress.saturating_add(state.load / 80)
                    }
                }
                InfrastructureKind::Communications => clock_stress.saturating_mul(2),
                InfrastructureKind::CrawlerYard => {
                    if self.preparations.contains(&Preparation::CrawlerPreheat) {
                        gradient_stress / 3
                    } else {
                        gradient_stress
                    }
                }
            };
            state.health = state.health.saturating_sub(damage);
            state.failed = state.health < 2_500 || state.synchronization_quality < 900;
        }

        let rainfall = u32::from(self.field.rainfall_deci_mm_h);
        let drainage_divisor = if self.preparations.contains(&Preparation::DrainageClearance) {
            11
        } else {
            4
        };
        self.east_drainage_excess_mm = self
            .east_drainage_excess_mm
            .saturating_add(rainfall / drainage_divisor);
        if self.preparations.contains(&Preparation::PumpManualTeams) {
            self.east_drainage_excess_mm = self.east_drainage_excess_mm.saturating_sub(18);
        }
    }

    fn record(&mut self, summary: String) -> Result<(), StateError> {
        self.events.append(
            self.tick,
            "firstlight.catastrophe",
            None,
            None,
            Vec::new(),
            CatastropheEvent {
                summary,
                clock_offset_us: self.field.clock_offset_us,
                failed_domains: self.failed_domains(),
            },
        )?;
        Ok(())
    }
}

fn domain(health: u16, load: u16) -> InfrastructureState {
    InfrastructureState {
        health,
        load,
        synchronization_quality: 10_000,
        local_control: false,
        failed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preparation_changes_counterfactual_outcomes() {
        let mut unprepared = FirstlightCatastrophe::canonical(11);
        let mut prepared = FirstlightCatastrophe::canonical(11);
        for preparation in [
            Preparation::LocalClockFallback,
            Preparation::PumpManualTeams,
            Preparation::ReduceViaductLoad,
            Preparation::DrainageClearance,
            Preparation::CrawlerPreheat,
        ] {
            prepared.prepare(preparation).expect("record preparation");
        }
        unprepared.advance(160).expect("advance unprepared");
        prepared.advance(160).expect("advance prepared");
        assert!(prepared.failed_domains().len() < unprepared.failed_domains().len());
        assert!(prepared.east_drainage_excess_mm < unprepared.east_drainage_excess_mm);
    }

    #[test]
    fn world_advances_without_player_proximity() {
        let mut catastrophe = FirstlightCatastrophe::canonical(12);
        catastrophe.advance(240).expect("advance catastrophe");
        assert_eq!(catastrophe.phase, CatastrophePhase::Aftermath);
        assert!(catastrophe.field.leading_edge_m > -2_800);
        assert!(catastrophe.events.events().len() == 1);
    }

    #[test]
    fn instruments_disagree_without_randomness() {
        let catastrophe = FirstlightCatastrophe::canonical(13);
        let optical = catastrophe.observe(ObservationDomain::VisibleOptical, 1);
        let inertial = catastrophe.observe(ObservationDomain::Inertial, 2);
        assert_ne!(optical.value, inertial.value);
        assert_eq!(
            optical,
            catastrophe.observe(ObservationDomain::VisibleOptical, 1)
        );
    }

    #[test]
    fn event_chain_remains_verifiable() {
        let mut catastrophe = FirstlightCatastrophe::canonical(14);
        catastrophe
            .prepare(Preparation::EvidenceMirrors)
            .expect("prepare");
        catastrophe.advance(20).expect("advance");
        catastrophe.events.verify().expect("verify event chain");
    }
}

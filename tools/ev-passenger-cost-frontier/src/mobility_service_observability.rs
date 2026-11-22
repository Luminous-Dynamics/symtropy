// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-bound operating scorecard for sustainable mobility service.
//!
//! The reducer preserves independent denominators and energy boundaries.
//! It computes descriptive metrics from supplied observations; it is not a
//! simulator, dispatch controller, sensor calibrator or vendor-performance audit.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_SITES: usize = 4;
const MAX_ASSETS: usize = 16;
const MAX_EVENTS: usize = 10_000;
const MAX_WINDOW_SECONDS: u64 = 32 * 24 * 60 * 60;
const MAX_ENERGY_WH: u64 = 1_000_000_000_000_000;
const MAX_COST_MINOR: u64 = 1_000_000_000_000_000;
const CLAIM_CEILING: &str = "Descriptive scorecard over supplied event observations only; no independent sensor calibration, supplier audit, operational permission, mission authorization, market settlement or physical actuation authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAssetKind {
    Aircraft,
    BatteryPack,
    Charger,
    StationaryEss,
    Vehicle,
    Dock,
    MobileUnit,
    Trailer,
    Amr,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcome {
    Completed,
    Failed,
    Unserved,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OperationalOutcome {
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityState {
    Available,
    Unavailable,
    Unknown,
    NotScheduled,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EnergyBoundary {
    VehicleBatteryTerminal,
    AircraftBatteryTerminal,
    ChargerInput,
    StationaryEssTerminal,
    SiteMeterImport,
    SiteMeterExport,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GridServiceOutcome {
    Delivered,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceSite {
    pub site_id: String,
    pub configuration_digest: String,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceAsset {
    pub asset_id: String,
    pub site_id: String,
    pub configuration_digest: String,
    pub kind: ServiceAssetKind,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissionObservation {
    pub event_id: String,
    pub mission_id: String,
    pub aircraft_id: String,
    pub aircraft_configuration_digest: String,
    pub committed_at_unix_ms: u64,
    pub deadline_at_unix_ms: u64,
    pub outcome: MissionOutcome,
    pub completed_at_unix_ms: Option<u64>,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SwapObservation {
    pub event_id: String,
    pub battery_pack_id: String,
    pub battery_configuration_digest: String,
    pub site_id: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: Option<u64>,
    pub outcome: OperationalOutcome,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnaroundObservation {
    pub event_id: String,
    pub aircraft_id: String,
    pub aircraft_configuration_digest: String,
    pub site_id: String,
    pub landed_at_unix_ms: u64,
    pub next_takeoff_at_unix_ms: Option<u64>,
    pub outcome: OperationalOutcome,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeObservation {
    pub event_id: String,
    pub battery_pack_id: String,
    pub battery_configuration_digest: String,
    pub charger_id: String,
    pub charger_configuration_digest: String,
    pub site_id: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: Option<u64>,
    pub outcome: OperationalOutcome,
    pub charger_input_energy_wh: Option<u64>,
    pub charger_input_evidence: Option<EvidenceEnvelope>,
    pub pack_terminal_delta_energy_wh: Option<u64>,
    pub pack_terminal_evidence: Option<EvidenceEnvelope>,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityObservation {
    pub event_id: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub start_at_unix_ms: u64,
    pub end_at_unix_ms: u64,
    pub state: AvailabilityState,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnergyObservation {
    pub event_id: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub boundary: EnergyBoundary,
    pub start_at_unix_ms: u64,
    pub end_at_unix_ms: u64,
    pub energy_wh: u64,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GridServiceObservation {
    pub event_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub started_at_unix_ms: u64,
    pub response_at_unix_ms: Option<u64>,
    pub ended_at_unix_ms: Option<u64>,
    pub requested_power_w: u32,
    pub requested_energy_wh: u64,
    pub outcome: GridServiceOutcome,
    pub metered_delivered_energy_wh: Option<u64>,
    pub request_evidence: EvidenceEnvelope,
    pub meter_evidence: Option<EvidenceEnvelope>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CostObservation {
    pub event_id: String,
    pub owner_id: String,
    pub owner_configuration_digest: String,
    pub site_id: String,
    pub category: String,
    pub currency_code: String,
    pub amount_minor_units: u64,
    pub start_at_unix_ms: u64,
    pub end_at_unix_ms: u64,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceObservabilityInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub decision_cutoff_unix_ms: u64,
    pub window_start_unix_ms: u64,
    pub window_end_exclusive_unix_ms: u64,
    pub sites: Vec<ServiceSite>,
    pub assets: Vec<ServiceAsset>,
    pub missions: Vec<MissionObservation>,
    pub swaps: Vec<SwapObservation>,
    pub turnarounds: Vec<TurnaroundObservation>,
    pub charges: Vec<ChargeObservation>,
    pub availability: Vec<AvailabilityObservation>,
    pub energy: Vec<EnergyObservation>,
    pub grid_service: Vec<GridServiceObservation>,
    pub costs: Vec<CostObservation>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DurationStatistics {
    pub sample_count: usize,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub p99_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AvailabilitySummary {
    pub asset_id: String,
    pub scheduled_seconds: u64,
    pub available_seconds: u64,
    pub unavailable_seconds: u64,
    pub unknown_seconds: u64,
    pub not_scheduled_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BoundaryEnergyTotal {
    pub boundary: EnergyBoundary,
    pub observation_count: usize,
    pub total_energy_wh: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OptionalEnergyTotal {
    pub measured_count: usize,
    pub missing_count: usize,
    pub measured_total_wh: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MissionScore {
    pub committed: usize,
    pub completed: usize,
    pub on_time: usize,
    pub late: usize,
    pub failed: usize,
    pub unserved: usize,
    pub cancelled: usize,
    pub on_time_numerator: usize,
    pub on_time_denominator: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SwapScore {
    pub attempts: usize,
    pub completed: usize,
    pub failed: usize,
    pub interrupted: usize,
    pub completed_duration: DurationStatistics,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TurnaroundScore {
    pub attempts: usize,
    pub completed: usize,
    pub failed: usize,
    pub interrupted: usize,
    pub completed_duration: DurationStatistics,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChargeScore {
    pub attempts: usize,
    pub completed: usize,
    pub failed: usize,
    pub interrupted: usize,
    pub completed_duration: DurationStatistics,
    pub charger_input_energy: OptionalEnergyTotal,
    pub pack_terminal_delta_energy: OptionalEnergyTotal,
    pub paired_energy_measurements: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GridServiceScore {
    pub requests: usize,
    pub delivered: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub requested_energy_wh: u64,
    pub metered_delivered_energy_wh: Option<u64>,
    pub unmeasured_delivery_count: usize,
    pub response_latency: DurationStatistics,
    pub sustain_duration: DurationStatistics,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CostScore {
    pub currency_code: Option<String>,
    pub observation_count: usize,
    pub total_amount_minor_units: Option<u64>,
    pub cost_per_on_time_mission_numerator_minor_units: Option<u64>,
    pub cost_per_on_time_mission_denominator: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ServiceObservabilityReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub decision_cutoff_unix_ms: u64,
    pub window_start_unix_ms: u64,
    pub window_end_exclusive_unix_ms: u64,
    pub site_count: usize,
    pub asset_count: usize,
    pub mission: MissionScore,
    pub swap: SwapScore,
    pub turnaround: TurnaroundScore,
    pub charge: ChargeScore,
    pub availability: Vec<AvailabilitySummary>,
    pub energy_by_boundary: Vec<BoundaryEnergyTotal>,
    pub grid_service: GridServiceScore,
    pub cost: CostScore,
    pub all_evidence_source_grade: bool,
    pub all_evidence_synthetic: bool,
    pub evidence_record_count: usize,
    pub missing_observation_count: usize,
    pub dispatch_authority_granted: bool,
    pub market_authority_granted: bool,
    pub physical_mutation_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn evidence_valid(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    interval_start_unix_ms: u64,
    interval_end_inclusive_unix_ms: u64,
    decision_cutoff_unix_ms: u64,
) -> bool {
    if evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
        || evidence.asset_id.trim().is_empty()
        || evidence.configuration_digest.trim().is_empty()
        || evidence.producer_id.trim().is_empty()
        || evidence.method_id.trim().is_empty()
        || evidence.evidence_reference.trim().is_empty()
        || evidence.evidence_class == EvidenceClass::Unknown
        || interval_end_inclusive_unix_ms < interval_start_unix_ms
    {
        return false;
    }
    match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed), Some(valid_from), Some(valid_until)) => {
            observed >= interval_start_unix_ms
                && observed <= interval_end_inclusive_unix_ms
                && observed <= decision_cutoff_unix_ms
                && valid_from <= interval_start_unix_ms
                && interval_end_inclusive_unix_ms <= valid_until
                && valid_from <= observed
                && observed <= valid_until
        }
        _ => false,
    }
}

fn percentile_nearest_rank(samples: &[u64], percentile: usize) -> Option<u64> {
    if samples.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    let rank_numerator = percentile.checked_mul(samples.len())?;
    let rank = rank_numerator.checked_add(99)? / 100;
    let index = rank.max(1).checked_sub(1)?;
    samples.get(index).copied()
}

fn duration_statistics(mut samples: Vec<u64>) -> DurationStatistics {
    samples.sort_unstable();
    DurationStatistics {
        sample_count: samples.len(),
        p50_ms: percentile_nearest_rank(&samples, 50),
        p95_ms: percentile_nearest_rank(&samples, 95),
        p99_ms: percentile_nearest_rank(&samples, 99),
    }
}

fn increment(value: &mut usize) -> Option<()> {
    *value = value.checked_add(1)?;
    Some(())
}

fn energy_boundary_matches_asset(boundary: EnergyBoundary, kind: ServiceAssetKind) -> bool {
    matches!(
        (boundary, kind),
        (EnergyBoundary::VehicleBatteryTerminal, ServiceAssetKind::Vehicle)
            | (EnergyBoundary::AircraftBatteryTerminal, ServiceAssetKind::Aircraft)
            | (EnergyBoundary::ChargerInput, ServiceAssetKind::Charger)
            | (EnergyBoundary::StationaryEssTerminal, ServiceAssetKind::StationaryEss)
    )
}

pub fn evaluate_service_observability(
    input: &ServiceObservabilityInput,
) -> Result<ServiceObservabilityReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "MOBILITY-SERVICE-OBSERVABILITY-001"
        || input.decision_cutoff_unix_ms == 0
        || input.window_start_unix_ms >= input.window_end_exclusive_unix_ms
        || input.window_end_exclusive_unix_ms > input.decision_cutoff_unix_ms.saturating_add(1)
        || input.window_end_exclusive_unix_ms - input.window_start_unix_ms
            > MAX_WINDOW_SECONDS.saturating_mul(1_000)
        || input.claim_ceiling.trim().is_empty()
        || input.sites.is_empty()
        || input.sites.len() > MAX_SITES
        || input.assets.is_empty()
        || input.assets.len() > MAX_ASSETS
        || input.missions.len() > MAX_EVENTS
        || input.swaps.len() > MAX_EVENTS
        || input.turnarounds.len() > MAX_EVENTS
        || input.charges.len() > MAX_EVENTS
        || input.availability.len() > MAX_EVENTS
        || input.energy.len() > MAX_EVENTS
        || input.grid_service.len() > MAX_EVENTS
        || input.costs.len() > MAX_EVENTS
    {
        add_error(&mut errors, "scorecard_identity_window_or_bound_invalid");
        return Err(errors);
    }

    let window_end_inclusive = input.window_end_exclusive_unix_ms - 1;
    let mut site_map = BTreeMap::new();
    let mut asset_map = BTreeMap::new();
    let mut all_source_grade = true;
    let mut all_synthetic = true;
    let mut evidence_record_count = 0_usize;
    let mut missing_observation_count = 0_usize;

    for site in &input.sites {
        if site.site_id.trim().is_empty()
            || site.configuration_digest.trim().is_empty()
            || site_map.insert(site.site_id.as_str(), site).is_some()
            || !evidence_valid(
                &site.evidence,
                &site.site_id,
                &site.configuration_digest,
                input.window_start_unix_ms,
                window_end_inclusive,
                input.decision_cutoff_unix_ms,
            )
        {
            add_error(&mut errors, format!("site_identity_or_evidence_invalid:{}", site.site_id));
        }
        all_source_grade &= matches!(
            site.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= site.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    for asset in &input.assets {
        if asset.asset_id.trim().is_empty()
            || asset.configuration_digest.trim().is_empty()
            || !site_map.contains_key(asset.site_id.as_str())
            || asset_map.insert(asset.asset_id.as_str(), asset).is_some()
            || !evidence_valid(
                &asset.evidence,
                &asset.asset_id,
                &asset.configuration_digest,
                input.window_start_unix_ms,
                window_end_inclusive,
                input.decision_cutoff_unix_ms,
            )
        {
            add_error(&mut errors, format!("asset_identity_or_evidence_invalid:{}", asset.asset_id));
        }
        all_source_grade &= matches!(
            asset.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= asset.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    let mut event_ids = BTreeSet::new();
    let mut mission = MissionScore {
        committed: 0,
        completed: 0,
        on_time: 0,
        late: 0,
        failed: 0,
        unserved: 0,
        cancelled: 0,
        on_time_numerator: 0,
        on_time_denominator: 0,
    };
    let mut swap_attempts = 0;
    let mut swap_completed = 0;
    let mut swap_failed = 0;
    let mut swap_interrupted = 0;
    let mut swap_durations = Vec::new();
    let mut turnaround_attempts = 0;
    let mut turnaround_completed = 0;
    let mut turnaround_failed = 0;
    let mut turnaround_interrupted = 0;
    let mut turnaround_durations = Vec::new();
    let mut charge_attempts = 0;
    let mut charge_completed = 0;
    let mut charge_failed = 0;
    let mut charge_interrupted = 0;
    let mut charge_durations = Vec::new();
    let mut charge_input_measured = 0;
    let mut charge_input_missing = 0;
    let mut charge_input_total = 0_u64;
    let mut pack_delta_measured = 0;
    let mut pack_delta_missing = 0;
    let mut pack_delta_total = 0_u64;
    let mut paired_charge_measurements = 0;
    let mut energy_totals: BTreeMap<EnergyBoundary, (usize, u64)> = BTreeMap::new();
    let mut grid_requests = 0;
    let mut grid_delivered = 0;
    let mut grid_failed = 0;
    let mut grid_cancelled = 0;
    let mut grid_requested_wh = 0_u64;
    let mut grid_delivered_wh = 0_u64;
    let mut grid_metered_count = 0;
    let mut grid_unmeasured_count = 0;
    let mut grid_response_durations = Vec::new();
    let mut grid_sustain_durations = Vec::new();
    let mut cost_currency: Option<String> = None;
    let mut cost_total = 0_u64;
    let mut cost_count = 0_usize;

    for item in &input.missions {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        let asset = asset_map.get(item.aircraft_id.as_str()).copied();
        let observed_at = item.completed_at_unix_ms.unwrap_or(input.decision_cutoff_unix_ms);
        let Some(asset) = asset else {
            add_error(&mut errors, format!("mission_aircraft_unknown:{}", item.mission_id));
            continue;
        };
        if asset.kind != ServiceAssetKind::Aircraft
            || asset.configuration_digest != item.aircraft_configuration_digest
            || item.committed_at_unix_ms < input.window_start_unix_ms
            || item.committed_at_unix_ms > item.deadline_at_unix_ms
            || item.deadline_at_unix_ms > window_end_inclusive
            || item.completed_at_unix_ms.is_some_and(|time| {
                time < item.committed_at_unix_ms || time > window_end_inclusive
            })
        {
            add_error(&mut errors, format!("mission_timing_or_asset_invalid:{}", item.mission_id));
        }
        if !evidence_valid(
            &item.evidence,
            &item.aircraft_id,
            &item.aircraft_configuration_digest,
            item.committed_at_unix_ms.min(observed_at),
            observed_at,
            input.decision_cutoff_unix_ms,
        ) {
            add_error(&mut errors, format!("mission_evidence_invalid:{}", item.mission_id));
        }
        mission.committed += 1;
        match item.outcome {
            MissionOutcome::Completed => {
                mission.completed += 1;
                if let Some(completed_at) = item.completed_at_unix_ms {
                    if completed_at <= item.deadline_at_unix_ms {
                        mission.on_time += 1;
                    } else {
                        mission.late += 1;
                    }
                } else {
                    add_error(&mut errors, format!("completed_mission_missing_completion_time:{}", item.mission_id));
                }
            }
            MissionOutcome::Failed => {
                mission.failed += 1;
                if item.completed_at_unix_ms.is_some() {
                    add_error(&mut errors, format!("failed_mission_has_completion_time:{}", item.mission_id));
                }
            }
            MissionOutcome::Unserved => {
                mission.unserved += 1;
                if item.completed_at_unix_ms.is_some() {
                    add_error(&mut errors, format!("unserved_mission_has_completion_time:{}", item.mission_id));
                }
            }
            MissionOutcome::Cancelled => {
                mission.cancelled += 1;
                if item.completed_at_unix_ms.is_some() {
                    add_error(&mut errors, format!("cancelled_mission_has_completion_time:{}", item.mission_id));
                }
            }
        }
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }
    mission.on_time_numerator = mission.on_time;
    mission.on_time_denominator = mission.committed;

    for item in &input.swaps {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        let asset = asset_map.get(item.battery_pack_id.as_str()).copied();
        let Some(asset) = asset else {
            add_error(&mut errors, format!("swap_pack_unknown:{}", item.event_id));
            continue;
        };
        if asset.kind != ServiceAssetKind::BatteryPack
            || asset.configuration_digest != item.battery_configuration_digest
            || !site_map.contains_key(item.site_id.as_str())
            || asset.site_id != item.site_id
            || item.started_at_unix_ms < input.window_start_unix_ms
            || item.started_at_unix_ms >= input.window_end_exclusive_unix_ms
            || item.completed_at_unix_ms.is_some_and(|end| {
                end < item.started_at_unix_ms || end > window_end_inclusive
            })
            || (item.outcome == OperationalOutcome::Completed && item.completed_at_unix_ms.is_none())
        {
            add_error(&mut errors, format!("swap_identity_or_timing_invalid:{}", item.event_id));
        }
        let observation_end = item.completed_at_unix_ms.unwrap_or(item.started_at_unix_ms);
        if !evidence_valid(
            &item.evidence,
            &item.battery_pack_id,
            &item.battery_configuration_digest,
            item.started_at_unix_ms,
            observation_end,
            input.decision_cutoff_unix_ms,
        ) {
            add_error(&mut errors, format!("swap_evidence_invalid:{}", item.event_id));
        }
        swap_attempts += 1;
        match item.outcome {
            OperationalOutcome::Completed => {
                swap_completed += 1;
                if let Some(end) = item.completed_at_unix_ms {
                    swap_durations.push(end - item.started_at_unix_ms);
                }
            }
            OperationalOutcome::Failed => swap_failed += 1,
            OperationalOutcome::Interrupted => swap_interrupted += 1,
        }
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    for item in &input.turnarounds {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        let asset = asset_map.get(item.aircraft_id.as_str()).copied();
        let Some(asset) = asset else {
            add_error(&mut errors, format!("turnaround_aircraft_unknown:{}", item.event_id));
            continue;
        };
        if asset.kind != ServiceAssetKind::Aircraft
            || asset.configuration_digest != item.aircraft_configuration_digest
            || !site_map.contains_key(item.site_id.as_str())
            || asset.site_id != item.site_id
            || item.landed_at_unix_ms < input.window_start_unix_ms
            || item.landed_at_unix_ms >= input.window_end_exclusive_unix_ms
            || item.next_takeoff_at_unix_ms.is_some_and(|end| {
                end < item.landed_at_unix_ms || end > window_end_inclusive
            })
            || (item.outcome == OperationalOutcome::Completed && item.next_takeoff_at_unix_ms.is_none())
        {
            add_error(&mut errors, format!("turnaround_identity_or_timing_invalid:{}", item.event_id));
        }
        let observation_end = item.next_takeoff_at_unix_ms.unwrap_or(item.landed_at_unix_ms);
        if !evidence_valid(
            &item.evidence,
            &item.aircraft_id,
            &item.aircraft_configuration_digest,
            item.landed_at_unix_ms,
            observation_end,
            input.decision_cutoff_unix_ms,
        ) {
            add_error(&mut errors, format!("turnaround_evidence_invalid:{}", item.event_id));
        }
        turnaround_attempts += 1;
        match item.outcome {
            OperationalOutcome::Completed => {
                turnaround_completed += 1;
                if let Some(end) = item.next_takeoff_at_unix_ms {
                    turnaround_durations.push(end - item.landed_at_unix_ms);
                }
            }
            OperationalOutcome::Failed => turnaround_failed += 1,
            OperationalOutcome::Interrupted => turnaround_interrupted += 1,
        }
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    for item in &input.charges {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        let pack = asset_map.get(item.battery_pack_id.as_str()).copied();
        let charger = asset_map.get(item.charger_id.as_str()).copied();
        let (Some(pack), Some(charger)) = (pack, charger) else {
            add_error(&mut errors, format!("charge_asset_unknown:{}", item.event_id));
            continue;
        };
        if pack.kind != ServiceAssetKind::BatteryPack
            || charger.kind != ServiceAssetKind::Charger
            || pack.configuration_digest != item.battery_configuration_digest
            || charger.configuration_digest != item.charger_configuration_digest
            || pack.site_id != item.site_id
            || charger.site_id != item.site_id
            || !site_map.contains_key(item.site_id.as_str())
            || item.started_at_unix_ms < input.window_start_unix_ms
            || item.started_at_unix_ms >= input.window_end_exclusive_unix_ms
            || item.completed_at_unix_ms.is_some_and(|end| {
                end < item.started_at_unix_ms || end > window_end_inclusive
            })
            || (item.outcome == OperationalOutcome::Completed && item.completed_at_unix_ms.is_none())
        {
            add_error(&mut errors, format!("charge_identity_or_timing_invalid:{}", item.event_id));
        }
        let observation_end = item.completed_at_unix_ms.unwrap_or(item.started_at_unix_ms);
        if !evidence_valid(
            &item.evidence,
            &item.battery_pack_id,
            &item.battery_configuration_digest,
            item.started_at_unix_ms,
            observation_end,
            input.decision_cutoff_unix_ms,
        ) {
            add_error(&mut errors, format!("charge_session_evidence_invalid:{}", item.event_id));
        }
        if let Some(energy) = item.charger_input_energy_wh {
            if energy > MAX_ENERGY_WH {
                add_error(&mut errors, format!("charge_input_energy_out_of_bounds:{}", item.event_id));
            }
            let evidence = item.charger_input_evidence.as_ref();
            if evidence.is_none_or(|evidence| {
                !evidence_valid(
                    evidence,
                    &item.charger_id,
                    &item.charger_configuration_digest,
                    item.started_at_unix_ms,
                    observation_end,
                    input.decision_cutoff_unix_ms,
                )
            }) {
                add_error(&mut errors, format!("charger_input_evidence_missing_or_invalid:{}", item.event_id));
            }
            charge_input_measured += 1;
            charge_input_total = match charge_input_total.checked_add(energy) {
                Some(total) => total,
                None => {
                    add_error(&mut errors, "charge_input_energy_overflow");
                    charge_input_total
                }
            };
        } else {
            charge_input_missing += 1;
            missing_observation_count += 1;
            if item.charger_input_evidence.is_some() {
                add_error(&mut errors, format!("charger_input_evidence_without_value:{}", item.event_id));
            }
        }
        if let Some(energy) = item.pack_terminal_delta_energy_wh {
            if energy > MAX_ENERGY_WH {
                add_error(&mut errors, format!("pack_terminal_energy_out_of_bounds:{}", item.event_id));
            }
            let evidence = item.pack_terminal_evidence.as_ref();
            if evidence.is_none_or(|evidence| {
                !evidence_valid(
                    evidence,
                    &item.battery_pack_id,
                    &item.battery_configuration_digest,
                    item.started_at_unix_ms,
                    observation_end,
                    input.decision_cutoff_unix_ms,
                )
            }) {
                add_error(&mut errors, format!("pack_terminal_evidence_missing_or_invalid:{}", item.event_id));
            }
            pack_delta_measured += 1;
            pack_delta_total = match pack_delta_total.checked_add(energy) {
                Some(total) => total,
                None => {
                    add_error(&mut errors, "pack_terminal_energy_overflow");
                    pack_delta_total
                }
            };
        } else {
            pack_delta_missing += 1;
            missing_observation_count += 1;
            if item.pack_terminal_evidence.is_some() {
                add_error(&mut errors, format!("pack_terminal_evidence_without_value:{}", item.event_id));
            }
        }
        if item.charger_input_energy_wh.is_some() && item.pack_terminal_delta_energy_wh.is_some() {
            paired_charge_measurements += 1;
        }
        charge_attempts += 1;
        match item.outcome {
            OperationalOutcome::Completed => {
                charge_completed += 1;
                if let Some(end) = item.completed_at_unix_ms {
                    charge_durations.push(end - item.started_at_unix_ms);
                }
            }
            OperationalOutcome::Failed => charge_failed += 1,
            OperationalOutcome::Interrupted => charge_interrupted += 1,
        }
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
        for evidence in [
            item.charger_input_evidence.as_ref(),
            item.pack_terminal_evidence.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            all_source_grade &= matches!(
                evidence.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            );
            all_synthetic &= evidence.evidence_class == EvidenceClass::Synthetic;
            evidence_record_count = evidence_record_count.saturating_add(1);
        }
    }

    let mut availability_by_asset: BTreeMap<String, Vec<&AvailabilityObservation>> =
        BTreeMap::new();
    for item in &input.availability {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        let asset = asset_map.get(item.asset_id.as_str()).copied();
        let Some(asset) = asset else {
            add_error(&mut errors, format!("availability_asset_unknown:{}", item.event_id));
            continue;
        };
        if asset.configuration_digest != item.asset_configuration_digest
            || item.start_at_unix_ms < input.window_start_unix_ms
            || item.start_at_unix_ms >= item.end_at_unix_ms
            || item.end_at_unix_ms > input.window_end_exclusive_unix_ms
            || !evidence_valid(
                &item.evidence,
                &item.asset_id,
                &item.asset_configuration_digest,
                item.start_at_unix_ms,
                item.end_at_unix_ms - 1,
                input.decision_cutoff_unix_ms,
            )
        {
            add_error(&mut errors, format!("availability_interval_or_evidence_invalid:{}", item.event_id));
        }
        availability_by_asset
            .entry(item.asset_id.clone())
            .or_default()
            .push(item);
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    let mut availability_summaries = Vec::new();
    let window_duration_seconds =
        (input.window_end_exclusive_unix_ms - input.window_start_unix_ms) / 1_000;
    for asset in &input.assets {
        let mut segments = availability_by_asset
            .remove(&asset.asset_id)
            .unwrap_or_default();
        segments.sort_by_key(|segment| segment.start_at_unix_ms);
        let mut previous_end = input.window_start_unix_ms;
        let mut summary = AvailabilitySummary {
            asset_id: asset.asset_id.clone(),
            scheduled_seconds: window_duration_seconds,
            available_seconds: 0,
            unavailable_seconds: 0,
            unknown_seconds: 0,
            not_scheduled_seconds: 0,
        };
        for segment in segments {
            if segment.start_at_unix_ms < previous_end {
                add_error(&mut errors, format!("availability_overlap:{}", asset.asset_id));
                continue;
            }
            let gap_seconds = (segment.start_at_unix_ms - previous_end) / 1_000;
            summary.unknown_seconds = match summary.unknown_seconds.checked_add(gap_seconds) {
                Some(total) => total,
                None => {
                    add_error(&mut errors, "availability_unknown_seconds_overflow");
                    summary.unknown_seconds
                }
            };
            let duration_seconds = (segment.end_at_unix_ms - segment.start_at_unix_ms) / 1_000;
            let target = match segment.state {
                AvailabilityState::Available => &mut summary.available_seconds,
                AvailabilityState::Unavailable => &mut summary.unavailable_seconds,
                AvailabilityState::Unknown => &mut summary.unknown_seconds,
                AvailabilityState::NotScheduled => &mut summary.not_scheduled_seconds,
            };
            *target = match target.checked_add(duration_seconds) {
                Some(total) => total,
                None => {
                    add_error(&mut errors, "availability_state_seconds_overflow");
                    *target
                }
            };
            previous_end = segment.end_at_unix_ms;
        }
        let trailing_unknown =
            (input.window_end_exclusive_unix_ms - previous_end) / 1_000;
        summary.unknown_seconds = match summary.unknown_seconds.checked_add(trailing_unknown) {
            Some(total) => total,
            None => {
                add_error(&mut errors, "availability_trailing_unknown_overflow");
                summary.unknown_seconds
            }
        };
        let accounted = summary.available_seconds
            + summary.unavailable_seconds
            + summary.unknown_seconds
            + summary.not_scheduled_seconds;
        if accounted != summary.scheduled_seconds {
            add_error(&mut errors, format!("availability_partition_mismatch:{}", asset.asset_id));
        }
        summary.scheduled_seconds = summary
            .scheduled_seconds
            .saturating_sub(summary.not_scheduled_seconds);
        availability_summaries.push(summary);
    }
    availability_summaries.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));

    for item in &input.energy {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        if item.start_at_unix_ms < input.window_start_unix_ms
            || item.start_at_unix_ms >= item.end_at_unix_ms
            || item.end_at_unix_ms > input.window_end_exclusive_unix_ms
            || item.energy_wh > MAX_ENERGY_WH
        {
            add_error(&mut errors, format!("energy_observation_interval_or_value_invalid:{}", item.event_id));
        }

        let expected = match item.boundary {
            EnergyBoundary::SiteMeterImport | EnergyBoundary::SiteMeterExport => {
                site_map.get(item.asset_id.as_str()).map(|site| {
                    (&site.site_id, &site.configuration_digest, true)
                })
            }
            _ => asset_map.get(item.asset_id.as_str()).and_then(|asset| {
                energy_boundary_matches_asset(item.boundary, asset.kind)
                    .then_some((&asset.asset_id, &asset.configuration_digest, false))
            }),
        };
        let Some((expected_id, expected_digest, _is_site)) = expected else {
            add_error(&mut errors, format!("energy_boundary_asset_kind_mismatch:{}", item.event_id));
            continue;
        };
        if item.asset_configuration_digest != *expected_digest
            || !evidence_valid(
                &item.evidence,
                expected_id,
                expected_digest,
                item.start_at_unix_ms,
                item.end_at_unix_ms - 1,
                input.decision_cutoff_unix_ms,
            )
        {
            add_error(&mut errors, format!("energy_observation_evidence_invalid:{}", item.event_id));
        }
        let entry = energy_totals.entry(item.boundary).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = match entry.1.checked_add(item.energy_wh) {
            Some(total) => total,
            None => {
                add_error(&mut errors, "energy_boundary_total_overflow");
                entry.1
            }
        };
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    for item in &input.grid_service {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        let Some(site) = site_map.get(item.site_id.as_str()) else {
            add_error(&mut errors, format!("grid_service_site_unknown:{}", item.event_id));
            continue;
        };
        if item.site_configuration_digest != site.configuration_digest
            || item.started_at_unix_ms < input.window_start_unix_ms
            || item.started_at_unix_ms >= input.window_end_exclusive_unix_ms
            || item.requested_power_w == 0
            || item.requested_energy_wh > MAX_ENERGY_WH
            || item.response_at_unix_ms.is_some_and(|time| {
                time < item.started_at_unix_ms || time > window_end_inclusive
            })
            || item.ended_at_unix_ms.is_some_and(|time| {
                time < item.response_at_unix_ms.unwrap_or(item.started_at_unix_ms)
                    || time > window_end_inclusive
            })
        {
            add_error(&mut errors, format!("grid_service_request_or_timing_invalid:{}", item.event_id));
        }
        if !evidence_valid(
            &item.request_evidence,
            &item.site_id,
            &item.site_configuration_digest,
            item.started_at_unix_ms,
            item.started_at_unix_ms,
            input.decision_cutoff_unix_ms,
        ) {
            add_error(&mut errors, format!("grid_service_request_evidence_invalid:{}", item.event_id));
        }
        grid_requests += 1;
        grid_requested_wh = match grid_requested_wh.checked_add(item.requested_energy_wh) {
            Some(total) => total,
            None => {
                add_error(&mut errors, "grid_requested_energy_overflow");
                grid_requested_wh
            }
        };
        match item.outcome {
            GridServiceOutcome::Delivered => {
                grid_delivered += 1;
                let (Some(delivered), Some(response), Some(end), Some(meter_evidence)) = (
                    item.metered_delivered_energy_wh,
                    item.response_at_unix_ms,
                    item.ended_at_unix_ms,
                    item.meter_evidence.as_ref(),
                ) else {
                    add_error(&mut errors, format!("delivered_grid_service_missing_meter_or_timing:{}", item.event_id));
                    grid_unmeasured_count += 1;
                    missing_observation_count += 1;
                    continue;
                };
                if delivered > item.requested_energy_wh
                    || !evidence_valid(
                        meter_evidence,
                        &item.site_id,
                        &item.site_configuration_digest,
                        response,
                        end,
                        input.decision_cutoff_unix_ms,
                    )
                    || !matches!(
                        meter_evidence.evidence_class,
                        EvidenceClass::Measured | EvidenceClass::IndependentTest | EvidenceClass::Synthetic
                    )
                {
                    add_error(&mut errors, format!("grid_meter_evidence_or_delivery_invalid:{}", item.event_id));
                }
                grid_metered_count += 1;
                grid_delivered_wh = match grid_delivered_wh.checked_add(delivered) {
                    Some(total) => total,
                    None => {
                        add_error(&mut errors, "grid_delivered_energy_overflow");
                        grid_delivered_wh
                    }
                };
                grid_response_durations.push(response - item.started_at_unix_ms);
                grid_sustain_durations.push(end - response);
            }
            GridServiceOutcome::Failed => {
                grid_failed += 1;
                if item.metered_delivered_energy_wh.is_some() != item.meter_evidence.is_some() {
                    add_error(&mut errors, format!("failed_grid_meter_evidence_pair_invalid:{}", item.event_id));
                }
                if item.metered_delivered_energy_wh.is_some() {
                    grid_unmeasured_count = grid_unmeasured_count.saturating_add(0);
                } else {
                    grid_unmeasured_count += 1;
                }
            }
            GridServiceOutcome::Cancelled => {
                grid_cancelled += 1;
                if item.metered_delivered_energy_wh.is_some() || item.meter_evidence.is_some() {
                    add_error(&mut errors, format!("cancelled_grid_request_has_delivery_record:{}", item.event_id));
                }
            }
        }
        all_source_grade &= matches!(
            item.request_evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.request_evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
        if let Some(meter_evidence) = &item.meter_evidence {
            all_source_grade &= matches!(
                meter_evidence.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            );
            all_synthetic &= meter_evidence.evidence_class == EvidenceClass::Synthetic;
            evidence_record_count = evidence_record_count.saturating_add(1);
        }
    }

    for item in &input.costs {
        if !event_ids.insert(item.event_id.as_str()) {
            add_error(&mut errors, format!("duplicate_event_id:{}", item.event_id));
        }
        if item.currency_code.len() != 3
            || !item.currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
            || item.category.trim().is_empty()
            || item.amount_minor_units > MAX_COST_MINOR
            || item.start_at_unix_ms < input.window_start_unix_ms
            || item.start_at_unix_ms >= item.end_at_unix_ms
            || item.end_at_unix_ms > input.window_end_exclusive_unix_ms
        {
            add_error(&mut errors, format!("cost_identity_currency_or_window_invalid:{}", item.event_id));
        }
        let expected = if let Some(site) = site_map.get(item.owner_id.as_str()) {
            Some((&site.site_id, &site.configuration_digest))
        } else {
            asset_map.get(item.owner_id.as_str()).map(|asset| {
                (&asset.asset_id, &asset.configuration_digest)
            })
        };
        let Some((asset_id, config_digest)) = expected else {
            add_error(&mut errors, format!("cost_owner_unknown:{}", item.event_id));
            continue;
        };
        if item.owner_configuration_digest != *config_digest
            || !evidence_valid(
                &item.evidence,
                asset_id,
                config_digest,
                item.start_at_unix_ms,
                item.end_at_unix_ms - 1,
                input.decision_cutoff_unix_ms,
            )
        {
            add_error(&mut errors, format!("cost_evidence_invalid:{}", item.event_id));
        }
        if let Some(currency) = &cost_currency {
            if currency != &item.currency_code {
                add_error(&mut errors, "mixed_currency_cost_ledger_not_admissible");
            }
        } else {
            cost_currency = Some(item.currency_code.clone());
        }
        cost_total = match cost_total.checked_add(item.amount_minor_units) {
            Some(total) => total,
            None => {
                add_error(&mut errors, "cost_total_overflow");
                cost_total
            }
        };
        cost_count += 1;
        all_source_grade &= matches!(
            item.evidence.evidence_class,
            EvidenceClass::Measured | EvidenceClass::IndependentTest
        );
        all_synthetic &= item.evidence.evidence_class == EvidenceClass::Synthetic;
        evidence_record_count = evidence_record_count.saturating_add(1);
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let energy_by_boundary = energy_totals
        .into_iter()
        .map(|(boundary, (observation_count, total_energy_wh))| BoundaryEnergyTotal {
            boundary,
            observation_count,
            total_energy_wh,
        })
        .collect();
    let availability = availability_summaries;
    let report = ServiceObservabilityReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        decision_cutoff_unix_ms: input.decision_cutoff_unix_ms,
        window_start_unix_ms: input.window_start_unix_ms,
        window_end_exclusive_unix_ms: input.window_end_exclusive_unix_ms,
        site_count: input.sites.len(),
        asset_count: input.assets.len(),
        mission,
        swap: SwapScore {
            attempts: swap_attempts,
            completed: swap_completed,
            failed: swap_failed,
            interrupted: swap_interrupted,
            completed_duration: duration_statistics(swap_durations),
        },
        turnaround: TurnaroundScore {
            attempts: turnaround_attempts,
            completed: turnaround_completed,
            failed: turnaround_failed,
            interrupted: turnaround_interrupted,
            completed_duration: duration_statistics(turnaround_durations),
        },
        charge: ChargeScore {
            attempts: charge_attempts,
            completed: charge_completed,
            failed: charge_failed,
            interrupted: charge_interrupted,
            completed_duration: duration_statistics(charge_durations),
            charger_input_energy: OptionalEnergyTotal {
                measured_count: charge_input_measured,
                missing_count: charge_input_missing,
                measured_total_wh: (charge_input_measured > 0).then_some(charge_input_total),
            },
            pack_terminal_delta_energy: OptionalEnergyTotal {
                measured_count: pack_delta_measured,
                missing_count: pack_delta_missing,
                measured_total_wh: (pack_delta_measured > 0).then_some(pack_delta_total),
            },
            paired_energy_measurements: paired_charge_measurements,
        },
        availability,
        energy_by_boundary,
        grid_service: GridServiceScore {
            requests: grid_requests,
            delivered: grid_delivered,
            failed: grid_failed,
            cancelled: grid_cancelled,
            requested_energy_wh: grid_requested_wh,
            metered_delivered_energy_wh: (grid_metered_count > 0).then_some(grid_delivered_wh),
            unmeasured_delivery_count: grid_unmeasured_count,
            response_latency: duration_statistics(grid_response_durations),
            sustain_duration: duration_statistics(grid_sustain_durations),
        },
        cost: CostScore {
            currency_code: cost_currency,
            observation_count: cost_count,
            total_amount_minor_units: (cost_count > 0).then_some(cost_total),
            cost_per_on_time_mission_numerator_minor_units: (mission.on_time > 0 && cost_count > 0)
                .then_some(cost_total),
            cost_per_on_time_mission_denominator: mission.on_time,
        },
        all_evidence_source_grade: all_source_grade,
        all_evidence_synthetic: all_synthetic,
        evidence_record_count,
        missing_observation_count,
        dispatch_authority_granted: false,
        market_authority_granted: false,
        physical_mutation_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    };
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ServiceObservabilityInput {
        serde_json::from_str(include_str!("../fixtures/mobility_service_observability_v0_1.json"))
            .expect("frozen service observability fixture must parse")
    }

    fn independent_percentile(samples: &[u64], percentile: usize) -> Option<u64> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let rank = (percentile * sorted.len() + 99) / 100;
        sorted.get(rank.max(1) - 1).copied()
    }

    #[test]
    fn known_answer_preserves_all_six_service_denominators() {
        let report = evaluate_service_observability(&fixture()).expect("known-answer fixture");
        assert_eq!(report.mission.committed, 5);
        assert_eq!(report.mission.completed, 2);
        assert_eq!(report.mission.on_time, 1);
        assert_eq!(report.mission.late, 1);
        assert_eq!(report.mission.failed, 1);
        assert_eq!(report.mission.unserved, 1);
        assert_eq!(report.mission.cancelled, 1);
        assert_eq!(report.swap.attempts, 3);
        assert_eq!(report.swap.completed, 2);
        assert_eq!(report.swap.failed, 1);
        assert_eq!(report.swap.completed_duration.p50_ms, Some(20_000));
        assert_eq!(report.swap.completed_duration.p95_ms, Some(30_000));
        assert_eq!(report.turnaround.completed_duration.p50_ms, Some(40_000));
        assert_eq!(report.turnaround.completed_duration.p95_ms, Some(50_000));
        assert_eq!(report.charge.completed_duration.p50_ms, Some(1_620_000));
        assert_eq!(report.grid_service.requests, 2);
        assert_eq!(report.grid_service.metered_delivered_energy_wh, Some(800));
        assert_eq!(report.grid_service.unmeasured_delivery_count, 1);
        assert_eq!(report.cost.total_amount_minor_units, Some(12_000));
        assert_eq!(report.cost.cost_per_on_time_mission_denominator, 1);
        assert!(report.all_evidence_synthetic);
        assert!(!report.all_evidence_source_grade);
        assert!(!report.dispatch_authority_granted);
        assert!(!report.market_authority_granted);
        assert!(!report.physical_mutation_authority_granted);
    }

    #[test]
    fn percentiles_match_independent_nearest_rank_reference() {
        let samples = vec![30, 10, 50, 20, 40];
        let stats = duration_statistics(samples.clone());
        assert_eq!(stats.p50_ms, independent_percentile(&samples, 50));
        assert_eq!(stats.p95_ms, independent_percentile(&samples, 95));
        assert_eq!(stats.p99_ms, independent_percentile(&samples, 99));
    }

    #[test]
    fn empty_duration_sample_emits_none_not_zero() {
        let stats = duration_statistics(Vec::new());
        assert_eq!(stats.sample_count, 0);
        assert_eq!(stats.p50_ms, None);
        assert_eq!(stats.p95_ms, None);
        assert_eq!(stats.p99_ms, None);
    }

    #[test]
    fn completed_mission_without_completion_timestamp_is_rejected() {
        let mut input = fixture();
        input.missions[0].completed_at_unix_ms = None;
        assert!(evaluate_service_observability(&input).is_err());
    }

    #[test]
    fn unserved_mission_cannot_disappear_from_committed_denominator() {
        let mut input = fixture();
        input.missions[2].outcome = MissionOutcome::Cancelled;
        let report = evaluate_service_observability(&input).expect("cancelled remains in denominator");
        assert_eq!(report.mission.committed, 5);
        assert_eq!(report.mission.unserved, 0);
        assert_eq!(report.mission.cancelled, 2);
    }

    #[test]
    fn overlapping_availability_intervals_are_rejected() {
        let mut input = fixture();
        input.availability[1].start_at_unix_ms += 1;
        assert!(evaluate_service_observability(&input).is_err());
    }

    #[test]
    fn site_meter_energy_cannot_be_relabelled_as_aircraft_terminal_energy() {
        let mut input = fixture();
        input.energy[0].boundary = EnergyBoundary::AircraftBatteryTerminal;
        input.energy[0].asset_id = "site-01".to_owned();
        input.energy[0].asset_configuration_digest = "sha256:site-01-v01".to_owned();
        assert!(evaluate_service_observability(&input).is_err());
    }

    #[test]
    fn failed_swap_remains_in_attempt_denominator() {
        let mut input = fixture();
        let report = evaluate_service_observability(&input).expect("fixture");
        assert_eq!(report.swap.attempts, 3);
        assert_eq!(report.swap.completed + report.swap.failed + report.swap.interrupted, 3);
    }

    #[test]
    fn mixed_cost_currencies_are_rejected() {
        let mut input = fixture();
        input.costs[1].currency_code = "USD".to_owned();
        assert!(evaluate_service_observability(&input).is_err());
    }

    #[test]
    fn grid_requested_energy_is_not_reported_as_delivered_energy() {
        let mut input = fixture();
        input.grid_service[0].metered_delivered_energy_wh = None;
        input.grid_service[0].meter_evidence = None;
        let result = evaluate_service_observability(&input);
        assert!(result.is_err());
    }

    #[test]
    fn future_evidence_is_rejected() {
        let mut input = fixture();
        input.swaps[0].evidence.observed_at_unix_ms =
            Some(input.decision_cutoff_unix_ms + 1);
        assert!(evaluate_service_observability(&input).is_err());
    }

    #[test]
    fn input_order_permutation_preserves_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.sites.reverse();
        permuted.assets.reverse();
        permuted.missions.reverse();
        permuted.swaps.reverse();
        permuted.turnarounds.reverse();
        permuted.charges.reverse();
        permuted.availability.reverse();
        permuted.energy.reverse();
        permuted.grid_service.reverse();
        permuted.costs.reverse();
        let a = evaluate_service_observability(&input).expect("first input");
        let b = evaluate_service_observability(&permuted).expect("permuted input");
        assert_eq!(
            serde_json::to_string(&a).expect("serialize first"),
            serde_json::to_string(&b).expect("serialize second")
        );
    }
}

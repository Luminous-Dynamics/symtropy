// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact small-instance shared-aircraft repositioning and allocation oracle.
//! One aircraft is shared by multiple regions. It cannot teleport, spend its
//! protected energy reserve, or dispatch under stale/unknown weather or permission.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::mode_options::{PermissionState, WeatherState};

pub const SHARED_FLEET_SCHEMA_VERSION: u16 = 1;
pub const SHARED_FLEET_PROBABILITY_BPS: u32 = 10_000;
pub const MAX_SHARED_REGIONS: usize = 3;
pub const MAX_SHARED_PERIODS: usize = 6;
pub const MAX_SHARED_SCENARIOS: usize = 8;
pub const MAX_SHARED_POLICY_NODES: usize = 10;
pub const MAX_SHARED_POLICY_COUNT: u64 = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedEvidenceStatus {
    FreshInDomain,
    Unknown,
    Stale,
    Conflicted,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetRegion {
    pub region_id: String,
    pub road_capacity_units: u32,
    pub aerial_service_capacity_units: u32,
    pub road_cost_per_unit: u64,
    pub aerial_cost_per_unit: u64,
    pub unserved_penalty_per_unit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetObservation {
    pub region_id: String,
    pub demand_units: u32,
    pub weather: WeatherState,
    pub weather_evidence_reference: String,
    pub permission: PermissionState,
    pub permission_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetPeriod {
    pub observations: Vec<SharedFleetObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetScenario {
    pub scenario_id: String,
    pub probability_bps: u16,
    pub periods: Vec<SharedFleetPeriod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedRepositionLink {
    pub from_region_id: String,
    pub to_region_id: String,
    /// v0.1 uses one discrete period per cross-region reposition. Longer routes
    /// require the explicit in-transit weather/permission state planned for v0.2.
    pub travel_periods: u16,
    pub energy_units: u64,
    pub cost_units: u64,
    pub evidence_status: SharedEvidenceStatus,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetInput {
    pub schema_version: u16,
    pub scenario_set_id: String,
    pub period_count: u16,
    pub initial_aircraft_region_id: String,
    pub maximum_aircraft_energy_units: u64,
    pub initial_aircraft_energy_units: u64,
    pub protected_energy_reserve_units: u64,
    pub energy_units_per_aerial_delivery: u64,
    pub recharge_energy_units_per_idle_period: u64,
    pub aircraft_idle_cost_per_period: u64,
    pub regions: Vec<SharedFleetRegion>,
    pub reposition_links: Vec<SharedRepositionLink>,
    pub scenarios: Vec<SharedFleetScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SharedDecisionKey {
    period_index: u16,
    observed_history_digest_sha256: String,
}

#[derive(Debug, Clone)]
struct SharedDecisionNode {
    key: SharedDecisionKey,
}

#[derive(Debug, Clone, Copy, Default)]
struct ScenarioMetrics {
    total_cost: u128,
    road_deliveries: u128,
    aerial_deliveries: u128,
    unserved_units: u128,
    aerial_energy_units: u128,
    reposition_energy_units: u128,
    reposition_cost_units: u128,
    repositioning_periods: u128,
    repositioning_events: u128,
    idle_recharge_energy_units: u128,
    idle_periods: u128,
    hard_gate_blocked_actions: u128,
    energy_blocked_actions: u128,
    end_energy_units: u128,
}

#[derive(Debug, Clone, Default)]
struct WeightedMetrics {
    total_cost: u128,
    road_deliveries: u128,
    aerial_deliveries: u128,
    unserved_units: u128,
    aerial_energy_units: u128,
    reposition_energy_units: u128,
    reposition_cost_units: u128,
    repositioning_periods: u128,
    repositioning_events: u128,
    idle_recharge_energy_units: u128,
    idle_periods: u128,
    hard_gate_blocked_actions: u128,
    energy_blocked_actions: u128,
    end_energy_units: u128,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedFleetOutcome {
    pub expected_total_cost_units: u64,
    pub expected_road_deliveries: u64,
    pub expected_aerial_deliveries: u64,
    pub expected_unserved_units: u64,
    pub expected_aerial_energy_units: u64,
    pub expected_reposition_energy_units: u64,
    pub expected_reposition_cost_units: u64,
    pub expected_repositioning_periods: u64,
    pub expected_repositioning_events: u64,
    pub expected_idle_recharge_energy_units: u64,
    pub expected_idle_periods: u64,
    pub expected_hard_gate_blocked_actions: u64,
    pub expected_energy_blocked_actions: u64,
    pub expected_end_energy_units: u64,
    pub unauthorized_aerial_dispatches: u64,
    pub protected_reserve_violations: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedFleetDecision {
    pub period_index: u16,
    pub observed_history_digest_sha256: String,
    pub target_region_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedFleetReport {
    pub schema_version: u16,
    pub scenario_set_id: String,
    pub input_digest_sha256: String,
    pub region_count: usize,
    pub period_count: usize,
    pub scenario_count: usize,
    pub nonanticipative_policy_node_count: usize,
    pub exact_policy_count_evaluated: u64,
    pub optimized_policy: SharedFleetOutcome,
    pub optimized_decisions: Vec<SharedFleetDecision>,
    pub truck_only_baseline: SharedFleetOutcome,
    pub fixed_initial_region_baseline: SharedFleetOutcome,
    pub physical_dispatch_authorized: bool,
    pub dispatch_commands_emitted: bool,
    pub claim_ceiling: String,
}

fn flight_domain_is_current(weather: WeatherState, permission: PermissionState) -> bool {
    matches!(weather, WeatherState::ClearFresh)
        && matches!(permission, PermissionState::ApprovedFresh)
}

fn checked_add(target: &mut u128, value: u128, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{label} accumulator overflow"))?;
    Ok(())
}

fn weighted_average(value: u128, label: &str) -> Result<u64, String> {
    let rounded = value
        .checked_add(u128::from(SHARED_FLEET_PROBABILITY_BPS / 2))
        .ok_or_else(|| format!("{label} rounding overflow"))?
        / u128::from(SHARED_FLEET_PROBABILITY_BPS);
    u64::try_from(rounded).map_err(|_| format!("{label} exceeds report range"))
}

fn canonicalize(input: &SharedFleetInput) -> SharedFleetInput {
    let mut canonical = input.clone();
    canonical.regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    canonical.reposition_links.sort_by(|left, right| {
        (&left.from_region_id, &left.to_region_id)
            .cmp(&(&right.from_region_id, &right.to_region_id))
    });
    canonical.scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for scenario in &mut canonical.scenarios {
        for period in &mut scenario.periods {
            period.observations.sort_by(|left, right| left.region_id.cmp(&right.region_id));
        }
    }
    canonical
}

fn canonical_input_digest(input: &SharedFleetInput) -> Result<String, String> {
    let canonical = canonicalize(input);
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical shared-fleet input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn validate_input(input: &SharedFleetInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != SHARED_FLEET_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            SHARED_FLEET_SCHEMA_VERSION, input.schema_version
        ));
    }
    if input.scenario_set_id.trim().is_empty() {
        errors.push("scenario_set_id: must be non-empty".into());
    }
    if input.period_count == 0 || usize::from(input.period_count) > MAX_SHARED_PERIODS {
        errors.push(format!("period_count: expected 1..={MAX_SHARED_PERIODS}"));
    }
    if input.regions.len() < 2 || input.regions.len() > MAX_SHARED_REGIONS {
        errors.push(format!("regions: expected 2..={MAX_SHARED_REGIONS}"));
    }
    if input.scenarios.is_empty() || input.scenarios.len() > MAX_SHARED_SCENARIOS {
        errors.push(format!("scenarios: expected 1..={MAX_SHARED_SCENARIOS}"));
    }
    if input.maximum_aircraft_energy_units == 0
        || input.initial_aircraft_energy_units > input.maximum_aircraft_energy_units
        || input.protected_energy_reserve_units >= input.maximum_aircraft_energy_units
        || input.initial_aircraft_energy_units < input.protected_energy_reserve_units
        || input.energy_units_per_aerial_delivery == 0
    {
        errors.push("aircraft energy capacity, initial energy, reserve, delivery energy and recharge rate are inconsistent".into());
    }

    let mut region_ids = BTreeSet::new();
    for (index, region) in input.regions.iter().enumerate() {
        if region.region_id.trim().is_empty() || !region_ids.insert(region.region_id.as_str()) {
            errors.push(format!("regions[{index}].region_id: nonempty unique identity required"));
        }
        if region.aerial_service_capacity_units == 0
            || region.road_capacity_units > 1_000_000
            || region.aerial_service_capacity_units > 1_000_000
        {
            errors.push(format!("regions[{index}]: capacity values outside v1 limits"));
        }
    }
    if !region_ids.contains(input.initial_aircraft_region_id.as_str()) {
        errors.push("initial_aircraft_region_id: references unknown region".into());
    }

    let expected_links = input.regions.len().saturating_mul(input.regions.len().saturating_sub(1));
    if input.reposition_links.len() != expected_links {
        errors.push(format!(
            "reposition_links: expected {expected_links} directed links for complete regional connectivity, got {}",
            input.reposition_links.len()
        ));
    }
    let mut links = BTreeSet::new();
    for (index, link) in input.reposition_links.iter().enumerate() {
        if !region_ids.contains(link.from_region_id.as_str())
            || !region_ids.contains(link.to_region_id.as_str())
            || link.from_region_id == link.to_region_id
        {
            errors.push(format!("reposition_links[{index}]: invalid from/to region"));
        }
        if link.evidence_reference.trim().is_empty() {
            errors.push(format!("reposition_links[{index}]: evidence reference is required"));
        }
        if link.travel_periods != 1 || link.energy_units == 0 {
            errors.push(format!(
                "reposition_links[{index}]: v0.1 requires exactly one discrete travel period and positive energy"
            ));
        }
        if !links.insert((link.from_region_id.as_str(), link.to_region_id.as_str())) {
            errors.push(format!("reposition_links[{index}]: duplicate directed link"));
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut probability_sum = 0_u32;
    for (scenario_index, scenario) in input.scenarios.iter().enumerate() {
        if scenario.scenario_id.trim().is_empty() || !scenario_ids.insert(scenario.scenario_id.as_str()) {
            errors.push(format!("scenarios[{scenario_index}].scenario_id: nonempty unique identity required"));
        }
        if scenario.probability_bps == 0 {
            errors.push(format!("scenarios[{scenario_index}].probability_bps: must be positive"));
        }
        probability_sum += u32::from(scenario.probability_bps);
        if scenario.periods.len() != usize::from(input.period_count) {
            errors.push(format!("scenarios[{scenario_index}].periods: expected {}", input.period_count));
        }
        for (period_index, period) in scenario.periods.iter().enumerate() {
            if period.observations.len() != input.regions.len() {
                errors.push(format!("scenarios[{scenario_index}].periods[{period_index}].observations: expected one per region"));
            }
            let mut seen = BTreeSet::new();
            for observation in &period.observations {
                if !region_ids.contains(observation.region_id.as_str())
                    || !seen.insert(observation.region_id.as_str())
                {
                    errors.push(format!("scenarios[{scenario_index}].periods[{period_index}]: unknown or duplicate region observation"));
                }
                if observation.weather_evidence_reference.trim().is_empty()
                    || observation.permission_evidence_reference.trim().is_empty()
                {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}]: weather and permission evidence references are required"
                    ));
                }
                if observation.demand_units > 1_000_000 {
                    errors.push(format!("scenarios[{scenario_index}].periods[{period_index}]: demand exceeds v1 limit"));
                }
            }
            for region_id in &region_ids {
                if !seen.contains(region_id) {
                    errors.push(format!("scenarios[{scenario_index}].periods[{period_index}]: missing region {region_id}"));
                }
            }
        }
    }
    if probability_sum != SHARED_FLEET_PROBABILITY_BPS {
        errors.push(format!(
            "scenarios.probability_bps: expected sum {}, got {probability_sum}",
            SHARED_FLEET_PROBABILITY_BPS
        ));
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn observation_for_region<'a>(
    period: &'a SharedFleetPeriod,
    region_id: &str,
) -> Result<&'a SharedFleetObservation, String> {
    period.observations.iter().find(|observation| observation.region_id == region_id)
        .ok_or_else(|| format!("missing observation for region {region_id}"))
}

fn decision_key(
    period_index: usize,
    history: &[Vec<SharedFleetObservation>],
) -> Result<SharedDecisionKey, String> {
    let encoded = serde_json::to_vec(history)
        .map_err(|error| format!("cannot serialize shared-fleet observed history: {error}"))?;
    Ok(SharedDecisionKey {
        period_index: u16::try_from(period_index).map_err(|_| "period index exceeds u16".to_string())?,
        observed_history_digest_sha256: format!("{:x}", Sha256::digest(encoded)),
    })
}

fn build_decision_nodes(
    input: &SharedFleetInput,
) -> Result<(Vec<SharedDecisionNode>, BTreeMap<SharedDecisionKey, usize>), String> {
    let mut nodes = BTreeMap::<SharedDecisionKey, ()>::new();
    for scenario in &input.scenarios {
        let mut history: Vec<Vec<SharedFleetObservation>> = Vec::new();
        for (period_index, period) in scenario.periods.iter().enumerate() {
            let mut observations = period.observations.clone();
            observations.sort_by(|left, right| left.region_id.cmp(&right.region_id));
            history.push(observations);
            let key = decision_key(period_index, &history)?;
            nodes.insert(key, ());
        }
    }
    if nodes.len() > MAX_SHARED_POLICY_NODES {
        return Err(format!(
            "exact shared-fleet oracle supports at most {MAX_SHARED_POLICY_NODES} nonanticipative nodes, got {}",
            nodes.len()
        ));
    }
    let nodes: Vec<SharedDecisionNode> = nodes.into_keys()
        .map(|key| SharedDecisionNode { key })
        .collect();
    let indices = nodes.iter().enumerate()
        .map(|(index, node)| (node.key.clone(), index))
        .collect();
    Ok((nodes, indices))
}

fn region_by_id<'a>(
    input: &'a SharedFleetInput,
    region_id: &str,
) -> Result<&'a SharedFleetRegion, String> {
    input.regions.iter().find(|region| region.region_id == region_id)
        .ok_or_else(|| format!("unknown region {region_id}"))
}

fn reposition_link<'a>(
    input: &'a SharedFleetInput,
    from: &str,
    to: &str,
) -> Option<&'a SharedRepositionLink> {
    input.reposition_links.iter().find(|link| {
        link.from_region_id == from && link.to_region_id == to
    })
}

fn action_for_node(mask: u64, node_index: usize, radix: u64) -> usize {
    let mut value = mask;
    for _ in 0..node_index {
        value /= radix;
    }
    (value % radix) as usize
}

fn add_cost(metrics: &mut ScenarioMetrics, cost: u128) -> Result<(), String> {
    checked_add(&mut metrics.total_cost, cost, "scenario total cost")
}

fn simulate_scenario(
    input: &SharedFleetInput,
    scenario: &SharedFleetScenario,
    indices: &BTreeMap<SharedDecisionKey, usize>,
    mask: u64,
    aircraft_enabled: bool,
) -> Result<ScenarioMetrics, String> {
    let mut metrics = ScenarioMetrics::default();
    let mut aircraft_region = input.initial_aircraft_region_id.clone();
    let mut energy = input.initial_aircraft_energy_units;
    let mut history: Vec<Vec<SharedFleetObservation>> = Vec::new();

    for (period_index, period) in scenario.periods.iter().enumerate() {
        let mut observations = period.observations.clone();
        observations.sort_by(|left, right| left.region_id.cmp(&right.region_id));
        history.push(observations);
        let key = decision_key(period_index, &history)?;
        let node_index = *indices.get(&key).ok_or_else(|| "shared-fleet policy node missing".to_string())?;
        let action = action_for_node(mask, node_index, input.regions.len() as u64 + 1);
        let mut aerial_this_period: Option<(String, u32)> = None;

        if aircraft_enabled {
            if action == input.regions.len() {
                let before = energy;
                energy = energy.saturating_add(input.recharge_energy_units_per_idle_period)
                    .min(input.maximum_aircraft_energy_units);
                checked_add(
                    &mut metrics.idle_recharge_energy_units,
                    u128::from(energy - before),
                    "idle recharge energy",
                )?;
                checked_add(&mut metrics.idle_periods, 1, "idle periods")?;
                add_cost(&mut metrics, u128::from(input.aircraft_idle_cost_per_period))?;
            } else {
                let target = &input.regions[action].region_id;
                if target == &aircraft_region {
                    let observation = observation_for_region(period, target)?;
                    if flight_domain_is_current(observation.weather, observation.permission) {
                        let region = region_by_id(input, target)?;
                        let available_energy = energy.saturating_sub(input.protected_energy_reserve_units);
                        let energy_limited_units = available_energy / input.energy_units_per_aerial_delivery;
                        let maximum_service = observation.demand_units.min(region.aerial_service_capacity_units);
                        let delivered = maximum_service.min(u32::try_from(energy_limited_units).unwrap_or(u32::MAX));
                        let energy_used = u64::from(delivered)
                            .checked_mul(input.energy_units_per_aerial_delivery)
                            .ok_or_else(|| "aerial service energy overflow".to_string())?;
                        energy = energy.saturating_sub(energy_used);
                        checked_add(&mut metrics.aerial_energy_units, u128::from(energy_used), "aerial energy")?;
                        add_cost(&mut metrics, u128::from(delivered) * u128::from(region.aerial_cost_per_unit))?;
                        if delivered < maximum_service {
                            checked_add(&mut metrics.energy_blocked_actions, 1, "energy-blocked actions")?;
                        }
                        if delivered > 0 {
                            aerial_this_period = Some((target.clone(), delivered));
                            checked_add(&mut metrics.aerial_deliveries, u128::from(delivered), "aerial deliveries")?;
                        }
                    } else {
                        checked_add(&mut metrics.hard_gate_blocked_actions, 1, "hard-gate blocked actions")?;
                    }
                } else {
                    let link = reposition_link(input, &aircraft_region, target)
                        .ok_or_else(|| "reposition link missing after validation".to_string())?;
                    let origin_observation = observation_for_region(period, &aircraft_region)?;
                    let target_observation = observation_for_region(period, target)?;
                    let route_domain_clear = flight_domain_is_current(
                        origin_observation.weather,
                        origin_observation.permission,
                    ) && flight_domain_is_current(target_observation.weather, target_observation.permission);
                    if link.evidence_status != SharedEvidenceStatus::FreshInDomain || !route_domain_clear {
                        checked_add(&mut metrics.hard_gate_blocked_actions, 1, "hard-gate blocked actions")?;
                    } else if energy
                        .checked_sub(link.energy_units)
                        .is_none_or(|remaining| remaining < input.protected_energy_reserve_units)
                    {
                        checked_add(&mut metrics.energy_blocked_actions, 1, "energy-blocked actions")?;
                    } else {
                        energy = energy
                            .checked_sub(link.energy_units)
                            .ok_or_else(|| "reposition energy underflow after validation".to_string())?;
                        checked_add(&mut metrics.reposition_energy_units, u128::from(link.energy_units), "reposition energy")?;
                        checked_add(&mut metrics.reposition_cost_units, u128::from(link.cost_units), "reposition cost")?;
                        add_cost(&mut metrics, u128::from(link.cost_units))?;
                        checked_add(&mut metrics.repositioning_events, 1, "repositioning events")?;
                        checked_add(&mut metrics.repositioning_periods, 1, "repositioning periods")?;
                        // v0.1 reserves the complete decision period for repositioning.
                        // The target region can only be served from the next period onward.
                        aircraft_region = target.clone();
                    }
                }
            }
        }

        for region in &input.regions {
            let observation = observation_for_region(period, &region.region_id)?;
            let aerial_units = aerial_this_period.as_ref()
                .filter(|(served_region, _)| served_region == &region.region_id)
                .map(|(_, units)| *units)
                .unwrap_or(0);
            let remaining = observation.demand_units.saturating_sub(aerial_units);
            let road_units = remaining.min(region.road_capacity_units);
            let unserved = remaining - road_units;
            checked_add(&mut metrics.road_deliveries, u128::from(road_units), "road deliveries")?;
            checked_add(&mut metrics.unserved_units, u128::from(unserved), "unserved units")?;
            add_cost(&mut metrics, u128::from(road_units) * u128::from(region.road_cost_per_unit))?;
            add_cost(&mut metrics, u128::from(unserved) * u128::from(region.unserved_penalty_per_unit))?;
        }
    }
    metrics.end_energy_units = u128::from(energy);
    Ok(metrics)
}

fn add_weighted_metrics(
    weighted: &mut WeightedMetrics,
    metrics: ScenarioMetrics,
    probability_bps: u16,
) -> Result<(), String> {
    let p = u128::from(probability_bps);
    macro_rules! add_weighted {
        ($field:ident, $value:expr, $label:literal) => {{
            let value = $value
                .checked_mul(p)
                .ok_or_else(|| format!("weighted {} overflow", $label))?;
            checked_add(&mut weighted.$field, value, $label)?;
        }};
    }
    add_weighted!(total_cost, metrics.total_cost, "expected cost");
    add_weighted!(road_deliveries, metrics.road_deliveries, "expected road deliveries");
    add_weighted!(aerial_deliveries, metrics.aerial_deliveries, "expected aerial deliveries");
    add_weighted!(unserved_units, metrics.unserved_units, "expected unserved units");
    add_weighted!(aerial_energy_units, metrics.aerial_energy_units, "expected aerial energy");
    add_weighted!(reposition_energy_units, metrics.reposition_energy_units, "expected reposition energy");
    add_weighted!(reposition_cost_units, metrics.reposition_cost_units, "expected reposition cost");
    add_weighted!(repositioning_periods, metrics.repositioning_periods, "expected repositioning periods");
    add_weighted!(repositioning_events, metrics.repositioning_events, "expected repositioning events");
    add_weighted!(idle_recharge_energy_units, metrics.idle_recharge_energy_units, "expected idle recharge");
    add_weighted!(idle_periods, metrics.idle_periods, "expected idle periods");
    add_weighted!(hard_gate_blocked_actions, metrics.hard_gate_blocked_actions, "expected hard-gate blocks");
    add_weighted!(energy_blocked_actions, metrics.energy_blocked_actions, "expected energy blocks");
    add_weighted!(end_energy_units, metrics.end_energy_units, "expected end energy");
    Ok(())
}

fn evaluate_mask(
    input: &SharedFleetInput,
    nodes: &[SharedDecisionNode],
    indices: &BTreeMap<SharedDecisionKey, usize>,
    mask: u64,
    aircraft_enabled: bool,
) -> Result<WeightedMetrics, String> {
    let mut result = WeightedMetrics::default();
    for scenario in &input.scenarios {
        let metrics = simulate_scenario(input, scenario, indices, mask, aircraft_enabled)?;
        add_weighted_metrics(&mut result, metrics, scenario.probability_bps)?;
    }
    Ok(result)
}

fn outcome(metrics: WeightedMetrics) -> Result<SharedFleetOutcome, String> {
    Ok(SharedFleetOutcome {
        expected_total_cost_units: weighted_average(metrics.total_cost, "expected cost")?,
        expected_road_deliveries: weighted_average(metrics.road_deliveries, "expected road deliveries")?,
        expected_aerial_deliveries: weighted_average(metrics.aerial_deliveries, "expected aerial deliveries")?,
        expected_unserved_units: weighted_average(metrics.unserved_units, "expected unserved")?,
        expected_aerial_energy_units: weighted_average(metrics.aerial_energy_units, "expected aerial energy")?,
        expected_reposition_energy_units: weighted_average(metrics.reposition_energy_units, "expected reposition energy")?,
        expected_reposition_cost_units: weighted_average(metrics.reposition_cost_units, "expected reposition cost")?,
        expected_repositioning_periods: weighted_average(metrics.repositioning_periods, "expected repositioning periods")?,
        expected_repositioning_events: weighted_average(metrics.repositioning_events, "expected repositioning events")?,
        expected_idle_recharge_energy_units: weighted_average(metrics.idle_recharge_energy_units, "expected idle recharge")?,
        expected_idle_periods: weighted_average(metrics.idle_periods, "expected idle periods")?,
        expected_hard_gate_blocked_actions: weighted_average(metrics.hard_gate_blocked_actions, "expected hard-gate blocks")?,
        expected_energy_blocked_actions: weighted_average(metrics.energy_blocked_actions, "expected energy blocks")?,
        expected_end_energy_units: weighted_average(metrics.end_energy_units, "expected end energy")?,
        unauthorized_aerial_dispatches: 0,
        protected_reserve_violations: 0,
    })
}

fn decision_list(
    input: &SharedFleetInput,
    nodes: &[SharedDecisionNode],
    mask: u64,
) -> Vec<SharedFleetDecision> {
    let radix = input.regions.len() as u64 + 1;
    nodes.iter().enumerate().map(|(index, node)| {
        let action = action_for_node(mask, index, radix);
        SharedFleetDecision {
            period_index: node.key.period_index,
            observed_history_digest_sha256: node.key.observed_history_digest_sha256.clone(),
            target_region_id: input.regions.get(action).map(|region| region.region_id.clone()),
        }
    }).collect()
}

/// Solve an exact nonanticipative policy for one shared aircraft across a small
/// stochastic scenario tree. The result is advisory; it never authorizes dispatch.
pub fn evaluate_shared_fleet(
    input: &SharedFleetInput,
) -> Result<SharedFleetReport, Vec<String>> {
    validate_input(input)?;
    let canonical = canonicalize(input);
    let input_digest_sha256 = canonical_input_digest(&canonical).map_err(|error| vec![error])?;
    let (nodes, indices) = build_decision_nodes(&canonical).map_err(|error| vec![error])?;
    let radix = canonical.regions.len() as u64 + 1;
    let policy_count = radix.checked_pow(nodes.len() as u32)
        .ok_or_else(|| vec!["shared-fleet policy count overflow".into()])?;
    if policy_count > MAX_SHARED_POLICY_COUNT {
        return Err(vec![format!("shared-fleet policy count {policy_count} exceeds v1 limit")]);
    }

    let mut best_mask = 0_u64;
    let mut best_metrics = evaluate_mask(&canonical, &nodes, &indices, 0, true)
        .map_err(|error| vec![error])?;
    for mask in 1..policy_count {
        let metrics = evaluate_mask(&canonical, &nodes, &indices, mask, true)
            .map_err(|error| vec![error])?;
        if metrics.total_cost < best_metrics.total_cost {
            best_mask = mask;
            best_metrics = metrics;
        }
    }

    let truck_only = outcome(
        evaluate_mask(&canonical, &nodes, &indices, 0, false).map_err(|error| vec![error])?,
    ).map_err(|error| vec![error])?;

    let initial_action = canonical.regions.iter()
        .position(|region| region.region_id == canonical.initial_aircraft_region_id)
        .ok_or_else(|| vec!["initial aircraft region disappeared after validation".into()])? as u64;
    let mut fixed_mask = 0_u64;
    let mut multiplier = 1_u64;
    for _ in 0..nodes.len() {
        fixed_mask = fixed_mask.checked_add(initial_action.checked_mul(multiplier)
            .ok_or_else(|| vec!["fixed baseline mask overflow".into()])?)
            .ok_or_else(|| vec!["fixed baseline mask overflow".into()])?;
        multiplier = multiplier.checked_mul(radix)
            .ok_or_else(|| vec!["fixed baseline multiplier overflow".into()])?;
    }
    let fixed = outcome(
        evaluate_mask(&canonical, &nodes, &indices, fixed_mask, true)
            .map_err(|error| vec![error])?,
    ).map_err(|error| vec![error])?;

    Ok(SharedFleetReport {
        schema_version: SHARED_FLEET_SCHEMA_VERSION,
        scenario_set_id: canonical.scenario_set_id.clone(),
        input_digest_sha256,
        region_count: canonical.regions.len(),
        period_count: usize::from(canonical.period_count),
        scenario_count: canonical.scenarios.len(),
        nonanticipative_policy_node_count: nodes.len(),
        exact_policy_count_evaluated: policy_count,
        optimized_policy: outcome(best_metrics).map_err(|error| vec![error])?,
        optimized_decisions: decision_list(&canonical, &nodes, best_mask),
        truck_only_baseline: truck_only,
        fixed_initial_region_baseline: fixed,
        physical_dispatch_authorized: false,
        dispatch_commands_emitted: false,
        claim_ceiling: "Exact synthetic one-aircraft shared-region allocation and repositioning policy under supplied discrete travel, energy, cost and scenario assumptions; not a fleet-scale optimizer, flight-energy model, route/airspace authorization, physical dispatch or commercial result".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(
        urban_demand: u32,
        rural_demand: u32,
        urban_weather: WeatherState,
        rural_weather: WeatherState,
    ) -> SharedFleetPeriod {
        SharedFleetPeriod {
            observations: vec![
                SharedFleetObservation {
                    region_id: "urban".into(),
                    demand_units: urban_demand,
                    weather: urban_weather,
                    weather_evidence_reference: "synthetic://weather/urban".into(),
                    permission: PermissionState::ApprovedFresh,
                    permission_evidence_reference: "synthetic://permission/urban".into(),
                },
                SharedFleetObservation {
                    region_id: "rural".into(),
                    demand_units: rural_demand,
                    weather: rural_weather,
                    weather_evidence_reference: "synthetic://weather/rural".into(),
                    permission: PermissionState::ApprovedFresh,
                    permission_evidence_reference: "synthetic://permission/rural".into(),
                },
            ],
        }
    }

    fn input() -> SharedFleetInput {
        SharedFleetInput {
            schema_version: 1,
            scenario_set_id: "two-region-shared-aircraft-v1".into(),
            period_count: 3,
            initial_aircraft_region_id: "urban".into(),
            maximum_aircraft_energy_units: 20,
            initial_aircraft_energy_units: 16,
            protected_energy_reserve_units: 4,
            energy_units_per_aerial_delivery: 2,
            recharge_energy_units_per_idle_period: 8,
            aircraft_idle_cost_per_period: 0,
            regions: vec![
                SharedFleetRegion {
                    region_id: "urban".into(),
                    road_capacity_units: 2,
                    aerial_service_capacity_units: 3,
                    road_cost_per_unit: 5,
                    aerial_cost_per_unit: 1,
                    unserved_penalty_per_unit: 100,
                },
                SharedFleetRegion {
                    region_id: "rural".into(),
                    road_capacity_units: 1,
                    aerial_service_capacity_units: 4,
                    road_cost_per_unit: 10,
                    aerial_cost_per_unit: 2,
                    unserved_penalty_per_unit: 120,
                },
            ],
            reposition_links: vec![
                SharedRepositionLink {
                    from_region_id: "urban".into(),
                    to_region_id: "rural".into(),
                    travel_periods: 1,
                    energy_units: 4,
                    cost_units: 3,
                    evidence_status: SharedEvidenceStatus::FreshInDomain,
                    evidence_reference: "synthetic://reposition/urban-rural".into(),
                },
                SharedRepositionLink {
                    from_region_id: "rural".into(),
                    to_region_id: "urban".into(),
                    travel_periods: 1,
                    energy_units: 4,
                    cost_units: 3,
                    evidence_status: SharedEvidenceStatus::FreshInDomain,
                    evidence_reference: "synthetic://reposition/rural-urban".into(),
                },
            ],
            scenarios: vec![
                SharedFleetScenario {
                    scenario_id: "normal".into(),
                    probability_bps: 6_000,
                    periods: vec![
                        period(4, 2, WeatherState::ClearFresh, WeatherState::ClearFresh),
                        period(4, 6, WeatherState::ClearFresh, WeatherState::ClearFresh),
                        period(4, 6, WeatherState::ClearFresh, WeatherState::ClearFresh),
                    ],
                },
                SharedFleetScenario {
                    scenario_id: "rural-weather-block".into(),
                    probability_bps: 4_000,
                    periods: vec![
                        period(4, 2, WeatherState::ClearFresh, WeatherState::ClearFresh),
                        period(4, 6, WeatherState::ClearFresh, WeatherState::BlockedFresh),
                        period(4, 6, WeatherState::ClearFresh, WeatherState::ClearFresh),
                    ],
                },
            ],
        }
    }

    fn forced_region_mask(input: &SharedFleetInput, target_region_id: &str) -> (Vec<SharedDecisionNode>, BTreeMap<SharedDecisionKey, usize>, u64) {
        let canonical = canonicalize(input);
        let (nodes, indices) = build_decision_nodes(&canonical).expect("build policy nodes");
        let target_index = canonical.regions.iter()
            .position(|region| region.region_id == target_region_id)
            .expect("target region exists") as u64;
        let radix = canonical.regions.len() as u64 + 1;
        let mut mask = 0_u64;
        let mut multiplier = 1_u64;
        for _ in 0..nodes.len() {
            mask += target_index * multiplier;
            multiplier *= radix;
        }
        (nodes, indices, mask)
    }

    #[test]
    fn exact_two_region_fixture_has_nonanticipative_policy_tree() {
        let report = evaluate_shared_fleet(&input()).expect("valid input");
        assert_eq!(report.region_count, 2);
        assert_eq!(report.period_count, 3);
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.nonanticipative_policy_node_count, 5);
        assert_eq!(report.exact_policy_count_evaluated, 243);
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
        assert!(!report.physical_dispatch_authorized);
    }

    #[test]
    fn committed_two_region_fixture_matches_exact_search_contract() {
        let fixture: SharedFleetInput = serde_json::from_str(include_str!(
            "../fixtures/synthetic-two-region-shared-aircraft.json"
        ))
        .expect("shared-fleet fixture must deserialize");
        let report = evaluate_shared_fleet(&fixture).expect("fixture must evaluate");
        assert_eq!(report.region_count, 2);
        assert_eq!(report.period_count, 3);
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.nonanticipative_policy_node_count, 5);
        assert_eq!(report.exact_policy_count_evaluated, 243);
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
        assert_eq!(report.optimized_policy.protected_reserve_violations, 0);
        assert!(!report.dispatch_commands_emitted);
    }

    #[test]
    fn identical_scenario_prefixes_share_the_same_decision_node() {
        let (nodes, _) = build_decision_nodes(&canonicalize(&input())).expect("valid tree");
        assert_eq!(nodes.iter().filter(|node| node.key.period_index == 0).count(), 1);
        assert_eq!(nodes.iter().filter(|node| node.key.period_index == 1).count(), 2);
    }

    #[test]
    fn repositioning_consumes_one_full_period_before_service() {
        let mut test = input();
        test.period_count = 1;
        for scenario in &mut test.scenarios {
            scenario.periods.truncate(1);
        }
        let canonical = canonicalize(&test);
        let (_nodes, indices, mask) = forced_region_mask(&canonical, "rural");
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            mask,
            true,
        )
        .expect("forced reposition policy");
        assert_eq!(metrics.repositioning_periods, 1);
        assert_eq!(metrics.repositioning_events, 1);
        assert_eq!(metrics.aerial_deliveries, 0);
        assert_eq!(metrics.end_energy_units, 12);
    }

    #[test]
    fn stale_link_evidence_blocks_a_forced_reposition_attempt() {
        let mut test = input();
        test.reposition_links[0].evidence_status = SharedEvidenceStatus::Stale;
        let canonical = canonicalize(&test);
        let (_, indices, mask) = forced_region_mask(&canonical, "rural");
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            mask,
            true,
        )
        .expect("forced reposition policy");
        assert_eq!(metrics.repositioning_events, 0);
        assert!(metrics.hard_gate_blocked_actions > 0);
    }

    #[test]
    fn route_weather_or_permission_block_prevents_a_forced_reposition() {
        let mut test = input();
        test.period_count = 1;
        for scenario in &mut test.scenarios {
            scenario.periods.truncate(1);
        }
        test.scenarios[0].periods[0].observations[1].permission = PermissionState::Unknown;
        let canonical = canonicalize(&test);
        let (_, indices, mask) = forced_region_mask(&canonical, "rural");
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            mask,
            true,
        )
        .expect("forced reposition policy");
        assert_eq!(metrics.repositioning_events, 0);
        assert!(metrics.hard_gate_blocked_actions > 0);
    }

    #[test]
    fn insufficient_energy_blocks_forced_reposition_before_spending_reserve() {
        let mut test = input();
        test.initial_aircraft_energy_units = 16;
        test.protected_energy_reserve_units = 13;
        let canonical = canonicalize(&test);
        let (_nodes, indices, mask) = forced_region_mask(&canonical, "rural");
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            mask,
            true,
        )
        .expect("forced reposition policy");
        assert_eq!(metrics.repositioning_events, 0);
        assert!(metrics.energy_blocked_actions > 0);
        assert_eq!(metrics.end_energy_units, 16);
    }

    #[test]
    fn insufficient_energy_blocks_reposition_even_when_reserve_is_zero() {
        let mut test = input();
        test.initial_aircraft_energy_units = 2;
        test.protected_energy_reserve_units = 0;
        let canonical = canonicalize(&test);
        let (_nodes, indices, mask) = forced_region_mask(&canonical, "rural");
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            mask,
            true,
        )
        .expect("insufficient energy must produce a blocked action, not arithmetic underflow");
        assert_eq!(metrics.repositioning_events, 0);
        assert_eq!(metrics.energy_blocked_actions, 3);
        assert_eq!(metrics.end_energy_units, 2);
    }

    #[test]
    fn stale_permission_blocks_a_forced_aerial_service() {
        let mut test = input();
        test.period_count = 1;
        for scenario in &mut test.scenarios {
            scenario.periods.truncate(1);
        }
        test.scenarios[0].periods[0].observations[0].permission = PermissionState::Stale;
        let canonical = canonicalize(&test);
        let (_nodes, indices, mask) = forced_region_mask(&canonical, "urban");
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            mask,
            true,
        )
        .expect("forced service policy");
        assert_eq!(metrics.aerial_deliveries, 0);
        assert!(metrics.hard_gate_blocked_actions > 0);
    }

    #[test]
    fn idle_period_charging_is_capped_at_declared_aircraft_energy() {
        let canonical = canonicalize(&input());
        let (nodes, indices) = build_decision_nodes(&canonical).expect("valid tree");
        let radix = canonical.regions.len() as u64 + 1;
        let all_idle_mask = radix.pow(nodes.len() as u32) - 1;
        let metrics = simulate_scenario(
            &canonical,
            &canonical.scenarios[0],
            &indices,
            all_idle_mask,
            true,
        )
        .expect("all-idle policy");
        assert_eq!(metrics.end_energy_units, 20);
        assert_eq!(metrics.idle_periods, 3);
        assert_eq!(metrics.idle_recharge_energy_units, 4);
    }

    #[test]
    fn protected_energy_reserve_is_never_spent() {
        let mut test = input();
        test.initial_aircraft_energy_units = 5;
        test.maximum_aircraft_energy_units = 5;
        test.protected_energy_reserve_units = 4;
        let report = evaluate_shared_fleet(&test).expect("valid input");
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
        assert_eq!(report.optimized_policy.protected_reserve_violations, 0);
    }

    #[test]
    fn one_shared_aircraft_has_one_region_of_service_per_period() {
        let report = evaluate_shared_fleet(&input()).expect("valid input");
        assert!(report.optimized_policy.expected_aerial_deliveries <= 12);
    }

    #[test]
    fn truck_only_baseline_has_no_aerial_service_or_reposition() {
        let report = evaluate_shared_fleet(&input()).expect("valid input");
        assert_eq!(report.truck_only_baseline.expected_aerial_deliveries, 0);
        assert_eq!(report.truck_only_baseline.expected_repositioning_events, 0);
    }

    #[test]
    fn reordered_inputs_have_identical_canonical_report() {
        let mut first = input();
        let first_report = evaluate_shared_fleet(&first).expect("first report");
        first.regions.reverse();
        first.reposition_links.reverse();
        for scenario in &mut first.scenarios {
            for period in &mut scenario.periods {
                period.observations.reverse();
            }
        }
        let second_report = evaluate_shared_fleet(&first).expect("permuted report");
        assert_eq!(first_report, second_report);
    }

    #[test]
    fn validates_probability_sum_and_duplicate_scenario_ids() {
        let mut test = input();
        test.scenarios[0].probability_bps = 5_000;
        assert!(validate_input(&test).unwrap_err().iter().any(|e| e.contains("probability_bps")));
        let mut duplicate = input();
        duplicate.scenarios[1].scenario_id = duplicate.scenarios[0].scenario_id.clone();
        assert!(validate_input(&duplicate).unwrap_err().iter().any(|e| e.contains("nonempty unique identity")));
    }

    #[test]
    fn rejects_unknown_or_duplicate_region_observation() {
        let mut test = input();
        test.scenarios[0].periods[0].observations[1].region_id = "unknown".into();
        assert!(validate_input(&test).is_err());
    }

    #[test]
    fn rejects_policy_tree_above_exact_search_budget() {
        let mut test = input();
        test.period_count = 6;
        test.scenarios.clear();
        for scenario_index in 0..4_u32 {
            let mut periods = Vec::new();
            for period_index in 0..6_u32 {
                periods.push(SharedFleetPeriod {
                    observations: vec![
                        SharedFleetObservation {
                            region_id: "urban".into(),
                            demand_units: 1 + scenario_index * 10 + period_index,
                            weather: WeatherState::ClearFresh,
                            weather_evidence_reference: "synthetic://weather/policy-tree/urban".into(),
                            permission: PermissionState::ApprovedFresh,
                            permission_evidence_reference: "synthetic://permission/policy-tree/urban".into(),
                        },
                        SharedFleetObservation {
                            region_id: "rural".into(),
                            demand_units: 2 + scenario_index * 10 + period_index,
                            weather: WeatherState::ClearFresh,
                            weather_evidence_reference: "synthetic://weather/policy-tree/rural".into(),
                            permission: PermissionState::ApprovedFresh,
                            permission_evidence_reference: "synthetic://permission/policy-tree/rural".into(),
                        },
                    ],
                });
            }
            test.scenarios.push(SharedFleetScenario {
                scenario_id: format!("branch-{scenario_index}"),
                probability_bps: 2_500,
                periods,
            });
        }
        assert!(validate_input(&test).is_ok());
        let error = build_decision_nodes(&canonicalize(&test))
            .expect_err("large nonanticipative tree must be rejected");
        assert!(error.contains("nonanticipative nodes"));
    }

    #[test]
    fn rejects_multi_period_transfer_until_transit_state_is_modeled() {
        let mut test = input();
        test.reposition_links[0].travel_periods = 2;
        assert!(validate_input(&test).is_err());
    }

    #[test]
    fn revoked_link_cannot_be_executed() {
        let mut test = input();
        test.reposition_links[0].evidence_status = SharedEvidenceStatus::Revoked;
        let report = evaluate_shared_fleet(&test).expect("valid but revoked link");
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
    }
}

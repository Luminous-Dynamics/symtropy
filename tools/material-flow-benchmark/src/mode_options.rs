// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact finite-horizon mode activation benchmark for optional logistics
//! capacity. This is a small scenario-tree oracle, not a production dispatcher.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MODE_OPTIONS_SCHEMA_VERSION: u16 = 1;
pub const PROBABILITY_TOTAL_BPS: u32 = 10_000;
pub const MAX_PERIODS: usize = 16;
pub const MAX_REGIONS: usize = 8;
pub const MAX_SCENARIOS: usize = 32;
pub const MAX_POLICY_NODES: usize = 20;
pub const MAX_UNITS_PER_PERIOD: u32 = 1_000_000;
pub const MAX_COST_PER_UNIT: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherState {
    ClearFresh,
    BlockedFresh,
    Unknown,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    ApprovedFresh,
    NotApprovedFresh,
    Unknown,
    Stale,
    Revoked,
}

impl PermissionState {
    fn permits_dispatch(self) -> bool {
        self == Self::ApprovedFresh
    }
}

impl WeatherState {
    fn permits_dispatch(self) -> bool {
        self == Self::ClearFresh
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionModeConfig {
    pub region_id: String,
    pub initial_aerial_active: bool,
    pub road_capacity_units: u32,
    pub aerial_capacity_units: u32,
    pub aerial_activation_threshold_units: u32,
    pub road_cost_per_unit: u64,
    pub aerial_cost_per_unit: u64,
    pub unserved_penalty_per_unit: u64,
    pub aerial_activation_cost: u64,
    pub aerial_deactivation_cost: u64,
    pub aerial_standby_cost_per_period: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionObservation {
    pub region_id: String,
    pub demand_units: u32,
    pub weather: WeatherState,
    pub permission: PermissionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPeriod {
    pub observations: Vec<RegionObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DemandWeatherScenario {
    pub scenario_id: String,
    /// Integer probability in basis points; all scenarios sum to 10,000.
    pub probability_bps: u16,
    pub periods: Vec<ScenarioPeriod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeOptionsInput {
    pub schema_version: u16,
    pub period_count: u16,
    pub regions: Vec<RegionModeConfig>,
    pub scenarios: Vec<DemandWeatherScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModePolicyDecision {
    pub period_index: u16,
    pub region_id: String,
    /// Digest of the region's observations available through this decision.
    /// Identical observed histories share one policy action across scenarios.
    pub observed_history_digest_sha256: String,
    pub aerial_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModePolicyOutcome {
    pub expected_total_cost_units: u64,
    pub expected_road_deliveries: u64,
    pub expected_aerial_deliveries: u64,
    pub expected_unserved_units: u64,
    pub expected_aerial_activations: u64,
    pub expected_aerial_deactivations: u64,
    pub expected_aerial_active_periods: u64,
    pub unauthorized_aerial_dispatches: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeOptionsReport {
    pub schema_version: u16,
    pub scenario_id: String,
    pub input_digest_sha256: String,
    pub region_count: usize,
    pub period_count: usize,
    pub scenario_count: usize,
    pub nonanticipative_policy_node_count: usize,
    pub exact_policy_count_evaluated: u64,
    pub optimized_policy: ModePolicyOutcome,
    pub optimized_decisions: Vec<ModePolicyDecision>,
    pub static_truck_only: ModePolicyOutcome,
    pub static_aerial_enabled: ModePolicyOutcome,
    pub immediate_demand_threshold: ModePolicyOutcome,
    pub physical_dispatch_authorized: bool,
    pub dispatch_commands_emitted: bool,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DecisionKey {
    period_index: u16,
    region_id: String,
    observed_history_digest_sha256: String,
}

#[derive(Debug, Clone)]
struct DecisionNode {
    key: DecisionKey,
    observation: RegionObservation,
}

#[derive(Debug, Clone, Copy, Default)]
struct WeightedMetrics {
    total_cost: u128,
    road_deliveries: u128,
    aerial_deliveries: u128,
    unserved_units: u128,
    activations: u128,
    deactivations: u128,
    active_periods: u128,
    unauthorized_dispatches: u128,
}

fn checked_add(target: &mut u128, value: u128, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{label} accumulator overflow"))?;
    Ok(())
}

fn weighted_average(value: u128, label: &str) -> Result<u64, String> {
    let rounded = value
        .checked_add(u128::from(PROBABILITY_TOTAL_BPS / 2))
        .ok_or_else(|| format!("{label} rounding overflow"))?
        / u128::from(PROBABILITY_TOTAL_BPS);
    u64::try_from(rounded).map_err(|_| format!("{label} exceeds report range"))
}

fn canonical_input_digest(input: &ModeOptionsInput) -> Result<String, String> {
    let mut canonical = input.clone();
    canonical.regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    canonical.scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for scenario in &mut canonical.scenarios {
        for period in &mut scenario.periods {
            period
                .observations
                .sort_by(|left, right| left.region_id.cmp(&right.region_id));
        }
    }
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical mode-options input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn validate_input(input: &ModeOptionsInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != MODE_OPTIONS_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            MODE_OPTIONS_SCHEMA_VERSION, input.schema_version
        ));
    }
    if input.period_count == 0 || usize::from(input.period_count) > MAX_PERIODS {
        errors.push(format!("period_count: expected 1..={MAX_PERIODS}"));
    }
    if input.regions.is_empty() || input.regions.len() > MAX_REGIONS {
        errors.push(format!("regions: expected 1..={MAX_REGIONS}"));
    }
    if input.scenarios.is_empty() || input.scenarios.len() > MAX_SCENARIOS {
        errors.push(format!("scenarios: expected 1..={MAX_SCENARIOS}"));
    }

    let mut region_ids = BTreeSet::new();
    for (index, region) in input.regions.iter().enumerate() {
        if region.region_id.trim().is_empty() {
            errors.push(format!("regions[{index}].region_id: must be non-empty"));
        }
        if !region_ids.insert(region.region_id.as_str()) {
            errors.push(format!("regions[{index}].region_id: duplicate identity"));
        }
        if region.road_capacity_units > MAX_UNITS_PER_PERIOD
            || region.aerial_capacity_units > MAX_UNITS_PER_PERIOD
            || region.aerial_activation_threshold_units > MAX_UNITS_PER_PERIOD
        {
            errors.push(format!("regions[{index}]: capacity and threshold exceed v1 limits"));
        }
        for (field, value) in [
            ("road_cost_per_unit", region.road_cost_per_unit),
            ("aerial_cost_per_unit", region.aerial_cost_per_unit),
            ("unserved_penalty_per_unit", region.unserved_penalty_per_unit),
            ("aerial_activation_cost", region.aerial_activation_cost),
            ("aerial_deactivation_cost", region.aerial_deactivation_cost),
            ("aerial_standby_cost_per_period", region.aerial_standby_cost_per_period),
        ] {
            if value > MAX_COST_PER_UNIT {
                errors.push(format!("regions[{index}].{field}: exceeds v1 cost ceiling"));
            }
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut probability_sum = 0_u32;
    for (scenario_index, scenario) in input.scenarios.iter().enumerate() {
        if scenario.scenario_id.trim().is_empty() {
            errors.push(format!("scenarios[{scenario_index}].scenario_id: must be non-empty"));
        }
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            errors.push(format!("scenarios[{scenario_index}].scenario_id: duplicate identity"));
        }
        if scenario.probability_bps == 0 {
            errors.push(format!("scenarios[{scenario_index}].probability_bps: must be positive"));
        }
        probability_sum += u32::from(scenario.probability_bps);
        if scenario.periods.len() != usize::from(input.period_count) {
            errors.push(format!(
                "scenarios[{scenario_index}].periods: expected {}, got {}",
                input.period_count,
                scenario.periods.len()
            ));
        }

        for (period_index, period) in scenario.periods.iter().enumerate() {
            if period.observations.len() != input.regions.len() {
                errors.push(format!(
                    "scenarios[{scenario_index}].periods[{period_index}].observations: expected one observation per region"
                ));
            }
            let mut observed_regions = BTreeSet::new();
            for (observation_index, observation) in period.observations.iter().enumerate() {
                if !region_ids.contains(observation.region_id.as_str()) {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}].observations[{observation_index}]: unknown region"
                    ));
                }
                if !observed_regions.insert(observation.region_id.as_str()) {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}].observations: duplicate region"
                    ));
                }
                if observation.demand_units > MAX_UNITS_PER_PERIOD {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}].observations[{observation_index}].demand_units: exceeds v1 limit"
                    ));
                }
            }
            for region_id in &region_ids {
                if !observed_regions.contains(region_id) {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}].observations: missing region {region_id}"
                    ));
                }
            }
        }
    }
    if probability_sum != PROBABILITY_TOTAL_BPS {
        errors.push(format!(
            "scenarios.probability_bps: expected sum {PROBABILITY_TOTAL_BPS}, got {probability_sum}"
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn history_key(
    period_index: usize,
    region_id: &str,
    history: &[RegionObservation],
) -> Result<DecisionKey, String> {
    let encoded = serde_json::to_vec(history)
        .map_err(|error| format!("cannot serialize observed history: {error}"))?;
    Ok(DecisionKey {
        period_index: u16::try_from(period_index)
            .map_err(|_| "period index exceeds u16".to_string())?,
        region_id: region_id.to_string(),
        observed_history_digest_sha256: format!("{:x}", Sha256::digest(encoded)),
    })
}

fn observation_for_region<'a>(
    period: &'a ScenarioPeriod,
    region_id: &str,
) -> Result<&'a RegionObservation, String> {
    period
        .observations
        .iter()
        .find(|observation| observation.region_id == region_id)
        .ok_or_else(|| format!("missing observation for region {region_id}"))
}

/// Build policy nodes from observed history, not scenario IDs. Two scenarios
/// with identical observations through a period necessarily share one action.
fn build_decision_nodes(
    input: &ModeOptionsInput,
) -> Result<(Vec<DecisionNode>, BTreeMap<DecisionKey, usize>), String> {
    let mut nodes_by_key = BTreeMap::<DecisionKey, RegionObservation>::new();
    for scenario in &input.scenarios {
        for region in &input.regions {
            let mut history = Vec::with_capacity(scenario.periods.len());
            for (period_index, period) in scenario.periods.iter().enumerate() {
                let observation = observation_for_region(period, &region.region_id)?.clone();
                history.push(observation.clone());
                let key = history_key(period_index, &region.region_id, &history)?;
                if let Some(previous) = nodes_by_key.get(&key) {
                    if previous != &observation {
                        return Err("identical observation history produced conflicting decision state".into());
                    }
                } else {
                    nodes_by_key.insert(key, observation);
                }
            }
        }
    }

    if nodes_by_key.len() > MAX_POLICY_NODES {
        return Err(format!(
            "exact finite-horizon oracle supports at most {MAX_POLICY_NODES} nonanticipative policy nodes, got {}",
            nodes_by_key.len()
        ));
    }

    let nodes: Vec<DecisionNode> = nodes_by_key
        .into_iter()
        .map(|(key, observation)| DecisionNode { key, observation })
        .collect();
    let indices = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.key.clone(), index))
        .collect();
    Ok((nodes, indices))
}

fn add_policy_cost(
    metrics: &mut WeightedMetrics,
    cost: u128,
    probability_bps: u16,
) -> Result<(), String> {
    let weighted = cost
        .checked_mul(u128::from(probability_bps))
        .ok_or_else(|| "weighted cost overflow".to_string())?;
    checked_add(&mut metrics.total_cost, weighted, "expected total cost")
}

fn evaluate_policy_mask(
    input: &ModeOptionsInput,
    nodes: &[DecisionNode],
    indices: &BTreeMap<DecisionKey, usize>,
    mask: u64,
) -> Result<WeightedMetrics, String> {
    let mut weighted = WeightedMetrics::default();

    for scenario in &input.scenarios {
        let mut scenario_metrics = WeightedMetrics::default();

        for region in &input.regions {
            let mut aerial_active = region.initial_aerial_active;
            let mut history = Vec::with_capacity(scenario.periods.len());

            for (period_index, period) in scenario.periods.iter().enumerate() {
                let observation = observation_for_region(period, &region.region_id)?;
                history.push(observation.clone());
                let key = history_key(period_index, &region.region_id, &history)?;
                let node_index = *indices
                    .get(&key)
                    .ok_or_else(|| "policy node was not indexed".to_string())?;
                let aerial_enabled = ((mask >> node_index) & 1) == 1;

                let mut period_cost = 0_u128;
                if aerial_enabled != aerial_active {
                    let switching_cost = if aerial_enabled {
                        region.aerial_activation_cost
                    } else {
                        region.aerial_deactivation_cost
                    };
                    period_cost = period_cost
                        .checked_add(u128::from(switching_cost))
                        .ok_or_else(|| "switching cost overflow".to_string())?;
                    if aerial_enabled {
                        checked_add(
                            &mut scenario_metrics.activations,
                            1,
                            "activation count",
                        )?;
                    } else {
                        checked_add(
                            &mut scenario_metrics.deactivations,
                            1,
                            "deactivation count",
                        )?;
                    }
                }
                aerial_active = aerial_enabled;

                if aerial_active {
                    period_cost = period_cost
                        .checked_add(u128::from(region.aerial_standby_cost_per_period))
                        .ok_or_else(|| "standby cost overflow".to_string())?;
                    checked_add(
                        &mut scenario_metrics.active_periods,
                        1,
                        "active period count",
                    )?;
                }

                let aerial_permitted = aerial_active
                    && observation.weather.permits_dispatch()
                    && observation.permission.permits_dispatch();
                let aerial_units = if aerial_permitted {
                    observation.demand_units.min(region.aerial_capacity_units)
                } else {
                    0
                };
                let remaining_demand = observation.demand_units - aerial_units;
                let road_units = remaining_demand.min(region.road_capacity_units);
                let unserved_units = remaining_demand - road_units;

                period_cost = period_cost
                    .checked_add(
                        u128::from(road_units)
                            .checked_mul(u128::from(region.road_cost_per_unit))
                            .ok_or_else(|| "road service cost overflow".to_string())?,
                    )
                    .and_then(|value| {
                        value.checked_add(
                            u128::from(aerial_units)
                                .checked_mul(u128::from(region.aerial_cost_per_unit))?,
                        )
                    })
                    .and_then(|value| {
                        value.checked_add(
                            u128::from(unserved_units)
                                .checked_mul(u128::from(region.unserved_penalty_per_unit))?,
                        )
                    })
                    .ok_or_else(|| "period logistics cost overflow".to_string())?;

                checked_add(
                    &mut scenario_metrics.road_deliveries,
                    u128::from(road_units),
                    "road deliveries",
                )?;
                checked_add(
                    &mut scenario_metrics.aerial_deliveries,
                    u128::from(aerial_units),
                    "aerial deliveries",
                )?;
                checked_add(
                    &mut scenario_metrics.unserved_units,
                    u128::from(unserved_units),
                    "unserved units",
                )?;

                if aerial_units > 0 && !aerial_permitted {
                    checked_add(
                        &mut scenario_metrics.unauthorized_dispatches,
                        1,
                        "unauthorized dispatch count",
                    )?;
                }
                checked_add(
                    &mut scenario_metrics.total_cost,
                    period_cost,
                    "scenario cost",
                )?;
            }
        }

        let probability = u128::from(scenario.probability_bps);
        checked_add(
            &mut weighted.total_cost,
            scenario_metrics.total_cost
                .checked_mul(probability)
                .ok_or_else(|| "weighted total cost overflow".to_string())?,
            "expected total cost",
        )?;
        checked_add(
            &mut weighted.road_deliveries,
            scenario_metrics.road_deliveries
                .checked_mul(probability)
                .ok_or_else(|| "weighted road deliveries overflow".to_string())?,
            "expected road deliveries",
        )?;
        checked_add(
            &mut weighted.aerial_deliveries,
            scenario_metrics.aerial_deliveries
                .checked_mul(probability)
                .ok_or_else(|| "weighted aerial deliveries overflow".to_string())?,
            "expected aerial deliveries",
        )?;
        checked_add(
            &mut weighted.unserved_units,
            scenario_metrics.unserved_units
                .checked_mul(probability)
                .ok_or_else(|| "weighted unserved units overflow".to_string())?,
            "expected unserved units",
        )?;
        checked_add(
            &mut weighted.activations,
            scenario_metrics.activations
                .checked_mul(probability)
                .ok_or_else(|| "weighted activation count overflow".to_string())?,
            "expected activation count",
        )?;
        checked_add(
            &mut weighted.deactivations,
            scenario_metrics.deactivations
                .checked_mul(probability)
                .ok_or_else(|| "weighted deactivation count overflow".to_string())?,
            "expected deactivation count",
        )?;
        checked_add(
            &mut weighted.active_periods,
            scenario_metrics.active_periods
                .checked_mul(probability)
                .ok_or_else(|| "weighted active-period count overflow".to_string())?,
            "expected active-period count",
        )?;
        checked_add(
            &mut weighted.unauthorized_dispatches,
            scenario_metrics.unauthorized_dispatches
                .checked_mul(probability)
                .ok_or_else(|| "weighted unauthorized dispatch count overflow".to_string())?,
            "expected unauthorized dispatch count",
        )?;
    }

    Ok(weighted)
}

fn public_outcome(metrics: WeightedMetrics) -> Result<ModePolicyOutcome, String> {
    Ok(ModePolicyOutcome {
        expected_total_cost_units: weighted_average(metrics.total_cost, "expected cost")?,
        expected_road_deliveries: weighted_average(metrics.road_deliveries, "expected road deliveries")?,
        expected_aerial_deliveries: weighted_average(metrics.aerial_deliveries, "expected aerial deliveries")?,
        expected_unserved_units: weighted_average(metrics.unserved_units, "expected unserved units")?,
        expected_aerial_activations: weighted_average(metrics.activations, "expected activations")?,
        expected_aerial_deactivations: weighted_average(metrics.deactivations, "expected deactivations")?,
        expected_aerial_active_periods: weighted_average(metrics.active_periods, "expected active periods")?,
        unauthorized_aerial_dispatches: weighted_average(
            metrics.unauthorized_dispatches,
            "unauthorized dispatches",
        )?,
    })
}

/// Exhaustively optimize activation decisions on a small finite scenario tree.
/// Hard flight eligibility is outside the economic objective: only fresh,
/// approved and weather-clear observations permit aerial service.
pub fn evaluate_mode_options(
    input: &ModeOptionsInput,
) -> Result<ModeOptionsReport, Vec<String>> {
    validate_input(input)?;
    let input_digest_sha256 = canonical_input_digest(input).map_err(|error| vec![error])?;
    let (nodes, indices) = build_decision_nodes(input).map_err(|error| vec![error])?;

    let policy_count = 1_u64
        .checked_shl(u32::try_from(nodes.len()).map_err(|_| vec!["policy node count exceeds u32".into()])?)
        .ok_or_else(|| vec!["policy count overflows".into()])?;
    let mut best_mask = 0_u64;
    let mut best_metrics = evaluate_policy_mask(input, &nodes, &indices, 0)
        .map_err(|error| vec![error])?;

    for mask in 1..policy_count {
        let metrics = evaluate_policy_mask(input, &nodes, &indices, mask)
            .map_err(|error| vec![error])?;
        if metrics.total_cost < best_metrics.total_cost {
            best_mask = mask;
            best_metrics = metrics;
        }
    }

    let all_enabled_mask = policy_count - 1;
    let static_off = public_outcome(
        evaluate_policy_mask(input, &nodes, &indices, 0).map_err(|error| vec![error])?,
    )
    .map_err(|error| vec![error])?;
    let static_on = public_outcome(
        evaluate_policy_mask(input, &nodes, &indices, all_enabled_mask)
            .map_err(|error| vec![error])?,
    )
    .map_err(|error| vec![error])?;

    let mut threshold_mask = 0_u64;
    for (index, node) in nodes.iter().enumerate() {
        let region = input
            .regions
            .iter()
            .find(|region| region.region_id == node.key.region_id)
            .ok_or_else(|| vec!["policy node references unknown region".into()])?;
        if node.observation.demand_units >= region.aerial_activation_threshold_units {
            threshold_mask |= 1_u64 << index;
        }
    }
    let threshold = public_outcome(
        evaluate_policy_mask(input, &nodes, &indices, threshold_mask)
            .map_err(|error| vec![error])?,
    )
    .map_err(|error| vec![error])?;

    let optimized_decisions = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ModePolicyDecision {
            period_index: node.key.period_index,
            region_id: node.key.region_id.clone(),
            observed_history_digest_sha256: node.key.observed_history_digest_sha256.clone(),
            aerial_enabled: ((best_mask >> index) & 1) == 1,
        })
        .collect();

    Ok(ModeOptionsReport {
        schema_version: MODE_OPTIONS_SCHEMA_VERSION,
        scenario_id: "exact-finite-horizon-logistics-mode-options-v1".into(),
        input_digest_sha256,
        region_count: input.regions.len(),
        period_count: usize::from(input.period_count),
        scenario_count: input.scenarios.len(),
        nonanticipative_policy_node_count: nodes.len(),
        exact_policy_count_evaluated: policy_count,
        optimized_policy: public_outcome(best_metrics).map_err(|error| vec![error])?,
        optimized_decisions,
        static_truck_only: static_off,
        static_aerial_enabled: static_on,
        immediate_demand_threshold: threshold,
        physical_dispatch_authorized: false,
        dispatch_commands_emitted: false,
        claim_ceiling: "Exact synthetic finite-horizon activation policy under supplied scenario and cost assumptions; not calibrated fleet economics, route feasibility, flight authorization or dispatch".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(region_id: &str) -> RegionModeConfig {
        RegionModeConfig {
            region_id: region_id.into(),
            initial_aerial_active: false,
            road_capacity_units: 2,
            aerial_capacity_units: 4,
            aerial_activation_threshold_units: 4,
            road_cost_per_unit: 5,
            aerial_cost_per_unit: 1,
            unserved_penalty_per_unit: 100,
            aerial_activation_cost: 10,
            aerial_deactivation_cost: 4,
            aerial_standby_cost_per_period: 1,
        }
    }

    fn observation(
        region_id: &str,
        demand_units: u32,
        weather: WeatherState,
        permission: PermissionState,
    ) -> RegionObservation {
        RegionObservation {
            region_id: region_id.into(),
            demand_units,
            weather,
            permission,
        }
    }

    fn simple_input() -> ModeOptionsInput {
        ModeOptionsInput {
            schema_version: 1,
            period_count: 2,
            regions: vec![region("urban")],
            scenarios: vec![DemandWeatherScenario {
                scenario_id: "base".into(),
                probability_bps: 10_000,
                periods: vec![
                    ScenarioPeriod {
                        observations: vec![observation(
                            "urban",
                            6,
                            WeatherState::ClearFresh,
                            PermissionState::ApprovedFresh,
                        )],
                    },
                    ScenarioPeriod {
                        observations: vec![observation(
                            "urban",
                            6,
                            WeatherState::ClearFresh,
                            PermissionState::ApprovedFresh,
                        )],
                    },
                ],
            }],
        }
    }

    #[test]
    fn two_region_four_period_fixture_is_valid_and_exact() {
        let input: ModeOptionsInput = serde_json::from_str(include_str!(
            "../fixtures/synthetic-two-region-mode-options.json"
        ))
        .expect("mode-options fixture must deserialize");
        let report = evaluate_mode_options(&input).expect("fixture must evaluate");
        assert_eq!(report.region_count, 2);
        assert_eq!(report.period_count, 4);
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.nonanticipative_policy_node_count, 13);
        assert_eq!(report.exact_policy_count_evaluated, 8_192);
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
        assert!(!report.physical_dispatch_authorized);
        assert!(!report.dispatch_commands_emitted);
    }

    #[test]
    fn exact_policy_activates_aerial_capacity_when_it_reduces_total_cost() {
        let report = evaluate_mode_options(&simple_input()).expect("valid input");
        assert!(report.optimized_policy.expected_aerial_deliveries > 0);
        assert!(
            report.optimized_policy.expected_total_cost_units
                < report.static_truck_only.expected_total_cost_units
        );
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
    }

    #[test]
    fn exact_policy_keeps_aerial_capacity_off_when_road_is_cheaper() {
        let mut test = simple_input();
        test.regions[0].road_capacity_units = 20;
        test.regions[0].road_cost_per_unit = 1;
        test.regions[0].aerial_cost_per_unit = 20;
        let report = evaluate_mode_options(&test).expect("valid input");
        assert_eq!(report.optimized_policy.expected_aerial_deliveries, 0);
        assert_eq!(
            report.optimized_policy.expected_total_cost_units,
            report.static_truck_only.expected_total_cost_units
        );
    }

    #[test]
    fn weather_block_prevents_aerial_dispatch_even_when_asset_is_enabled() {
        let mut test = simple_input();
        for period in &mut test.scenarios[0].periods {
            period.observations[0].weather = WeatherState::BlockedFresh;
        }
        let report = evaluate_mode_options(&test).expect("valid input");
        assert_eq!(report.static_aerial_enabled.expected_aerial_deliveries, 0);
        assert_eq!(report.static_aerial_enabled.unauthorized_aerial_dispatches, 0);
    }

    #[test]
    fn unknown_or_stale_permission_prevents_aerial_dispatch() {
        let mut test = simple_input();
        test.scenarios[0].periods[0].observations[0].permission = PermissionState::Unknown;
        test.scenarios[0].periods[1].observations[0].permission = PermissionState::Stale;
        let report = evaluate_mode_options(&test).expect("valid input");
        assert_eq!(report.static_aerial_enabled.expected_aerial_deliveries, 0);
        assert_eq!(report.static_aerial_enabled.unauthorized_aerial_dispatches, 0);
    }

    #[test]
    fn identical_observed_prefixes_share_one_nonanticipative_decision() {
        let mut test = simple_input();
        test.scenarios.push(DemandWeatherScenario {
            scenario_id: "storm".into(),
            probability_bps: 4_000,
            periods: vec![
                ScenarioPeriod {
                    observations: vec![observation(
                        "urban",
                        6,
                        WeatherState::ClearFresh,
                        PermissionState::ApprovedFresh,
                    )],
                },
                ScenarioPeriod {
                    observations: vec![observation(
                        "urban",
                        6,
                        WeatherState::BlockedFresh,
                        PermissionState::ApprovedFresh,
                    )],
                },
            ],
        });
        test.scenarios[0].probability_bps = 6_000;
        let (nodes, _) = build_decision_nodes(&test).expect("build scenario tree");
        assert_eq!(nodes.len(), 3);
        assert_eq!(
            nodes.iter().filter(|node| node.key.period_index == 0).count(),
            1
        );
    }

    #[test]
    fn canonical_digest_and_report_ignore_scenario_and_observation_input_order() {
        let mut test = simple_input();
        test.scenarios.push(DemandWeatherScenario {
            scenario_id: "storm".into(),
            probability_bps: 4_000,
            periods: vec![
                ScenarioPeriod {
                    observations: vec![observation(
                        "urban",
                        6,
                        WeatherState::ClearFresh,
                        PermissionState::ApprovedFresh,
                    )],
                },
                ScenarioPeriod {
                    observations: vec![observation(
                        "urban",
                        6,
                        WeatherState::BlockedFresh,
                        PermissionState::ApprovedFresh,
                    )],
                },
            ],
        });
        test.scenarios[0].probability_bps = 6_000;
        let first = evaluate_mode_options(&test).expect("first input");
        test.scenarios.reverse();
        let second = evaluate_mode_options(&test).expect("permuted scenario order");
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_probability_sum_outside_ten_thousand_basis_points() {
        let mut test = simple_input();
        test.scenarios[0].probability_bps = 9_999;
        let errors = evaluate_mode_options(&test).expect_err("invalid probability sum");
        assert!(errors.iter().any(|error| error.contains("expected sum 10000")));
    }

    #[test]
    fn rejects_missing_region_observation() {
        let mut test = simple_input();
        test.regions.push(region("rural"));
        let errors = evaluate_mode_options(&test).expect_err("missing rural observation");
        assert!(errors.iter().any(|error| error.contains("missing region rural")));
    }

    #[test]
    fn report_never_authorizes_physical_dispatch() {
        let report = evaluate_mode_options(&simple_input()).expect("valid input");
        assert!(!report.physical_dispatch_authorized);
        assert!(!report.dispatch_commands_emitted);
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
    }

    #[test]
    fn exact_oracle_reports_every_policy_evaluated() {
        let report = evaluate_mode_options(&simple_input()).expect("valid input");
        assert_eq!(
            report.exact_policy_count_evaluated,
            1_u64 << report.nonanticipative_policy_node_count
        );
    }

    #[test]
    fn stale_weather_is_not_a_dispatch_eligible_observation() {
        let mut test = simple_input();
        test.scenarios[0].periods[0].observations[0].weather = WeatherState::Stale;
        let report = evaluate_mode_options(&test).expect("valid input");
        assert_eq!(report.static_aerial_enabled.expected_aerial_deliveries, 4);
    }
}

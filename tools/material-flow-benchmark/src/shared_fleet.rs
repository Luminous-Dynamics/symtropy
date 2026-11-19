// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact small-instance allocation oracle for shared aircraft across multiple
//! carrier regions. Repositioning consumes whole discrete periods; it is not
//! free or instantaneous. This is a benchmark, not a dispatch controller.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SHARED_FLEET_SCHEMA_VERSION: u16 = 1;
pub const PROBABILITY_TOTAL_BPS: u32 = 10_000;
pub const MAX_PERIODS: usize = 8;
pub const MAX_REGIONS: usize = 4;
pub const MAX_AIRCRAFT: usize = 3;
pub const MAX_SCENARIOS: usize = 16;
pub const MAX_DEMAND_UNITS: u32 = 10_000;
pub const MAX_COST_PER_UNIT: u64 = 1_000_000;
pub const MAX_DP_STATES: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedWeatherState {
    ClearFresh,
    BlockedFresh,
    Unknown,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedPermissionState {
    ApprovedFresh,
    NotApprovedFresh,
    Unknown,
    Stale,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedEvidenceStatus {
    FreshInDomain,
    Stale,
    Conflicted,
    Revoked,
    Unsupported,
}

impl SharedWeatherState {
    fn permits_aerial_service(self) -> bool {
        self == Self::ClearFresh
    }
}

impl SharedPermissionState {
    fn permits_aerial_service(self) -> bool {
        self == Self::ApprovedFresh
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedRegionConfig {
    pub region_id: String,
    pub road_capacity_units: u32,
    pub road_cost_per_unit: u64,
    pub aerial_cost_per_unit: u64,
    pub unserved_penalty_per_unit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedAircraftConfig {
    pub aircraft_id: String,
    pub initial_region_id: String,
    /// Qualified maximum useful service units per aircraft per period.
    /// Battery, flight duration and route co-availability are upstream inputs.
    pub maximum_service_units_per_period: u32,
    pub capability_evidence: SharedEvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedRegionObservation {
    pub region_id: String,
    pub demand_units: u32,
    pub weather: SharedWeatherState,
    pub permission: SharedPermissionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedScenarioPeriod {
    pub observations: Vec<SharedRegionObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedDemandScenario {
    pub scenario_id: String,
    /// Integer probability in basis points; all scenarios sum to 10,000.
    pub probability_bps: u16,
    pub periods: Vec<SharedScenarioPeriod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetInput {
    pub schema_version: u16,
    pub period_count: u16,
    pub regions: Vec<SharedRegionConfig>,
    pub aircraft: Vec<SharedAircraftConfig>,
    /// Matrix rows and columns follow regions[] order. Off-diagonal travel
    /// times must be positive, so no cross-region teleport is representable.
    pub reposition_periods: Vec<Vec<u16>>,
    pub reposition_cost: Vec<Vec<u64>>,
    pub scenarios: Vec<SharedDemandScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedAircraftTarget {
    pub aircraft_id: String,
    /// None means intentionally idle; a region ID means stay or reposition.
    pub target_region_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedFleetDecision {
    pub period_index: u16,
    pub observed_history_digest_sha256: String,
    pub fleet_state_digest_sha256: String,
    pub aircraft_targets: Vec<SharedAircraftTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedFleetOutcome {
    /// Exact expected-cost numerator divided by probability denominator 10,000.
    pub expected_total_cost_numerator_bps: u128,
    pub expected_total_cost_denominator_bps: u32,
    pub expected_total_cost_units: f64,
    pub expected_aerial_deliveries: f64,
    pub expected_road_deliveries: f64,
    pub expected_unserved_units: f64,
    pub expected_reposition_operations: f64,
    pub expected_repositioned_aircraft_periods: f64,
    pub unauthorized_aerial_dispatches: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedFleetReport {
    pub schema_version: u16,
    pub scenario_id: String,
    pub input_digest_sha256: String,
    pub region_count: usize,
    pub aircraft_count: usize,
    pub period_count: usize,
    pub scenario_count: usize,
    pub nonanticipative_tree_node_count: usize,
    pub dynamic_program_state_count: usize,
    pub action_count_evaluated: u64,
    pub optimized_policy: SharedFleetOutcome,
    pub truck_only: SharedFleetOutcome,
    pub fixed_initial_assignment: SharedFleetOutcome,
    pub optimized_decisions: Vec<SharedFleetDecision>,
    pub dispatch_authorized: bool,
    pub physical_commands_emitted: bool,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AircraftState {
    region_index: usize,
    ready_period: u16,
}

type FleetState = Vec<AircraftState>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StateKey {
    node_id: usize,
    fleet_state: FleetState,
}

#[derive(Debug, Clone)]
struct TreeNode {
    node_id: usize,
    period_index: Option<usize>,
    probability_bps: u32,
    observations: Vec<SharedRegionObservation>,
    history_digest_sha256: String,
    children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Copy, Default)]
struct WeightedMetrics {
    total_cost_bps: u128,
    aerial_deliveries_bps: u128,
    road_deliveries_bps: u128,
    unserved_units_bps: u128,
    reposition_operations_bps: u128,
    repositioned_aircraft_periods_bps: u128,
}

#[derive(Debug, Clone, Copy, Default)]
struct StepMetrics {
    total_cost: u128,
    aerial_deliveries: u128,
    road_deliveries: u128,
    unserved_units: u128,
    reposition_operations: u128,
    repositioned_aircraft_periods: u128,
}

struct SolverStats {
    action_count: u64,
}

fn checked_add(target: &mut u128, value: u128, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{label} accumulator overflow"))?;
    Ok(())
}

impl WeightedMetrics {
    fn add_assign(&mut self, other: Self) -> Result<(), String> {
        checked_add(&mut self.total_cost_bps, other.total_cost_bps, "weighted total cost")?;
        checked_add(
            &mut self.aerial_deliveries_bps,
            other.aerial_deliveries_bps,
            "weighted aerial deliveries",
        )?;
        checked_add(
            &mut self.road_deliveries_bps,
            other.road_deliveries_bps,
            "weighted road deliveries",
        )?;
        checked_add(
            &mut self.unserved_units_bps,
            other.unserved_units_bps,
            "weighted unserved units",
        )?;
        checked_add(
            &mut self.reposition_operations_bps,
            other.reposition_operations_bps,
            "weighted reposition operations",
        )?;
        checked_add(
            &mut self.repositioned_aircraft_periods_bps,
            other.repositioned_aircraft_periods_bps,
            "weighted repositioned aircraft periods",
        )?;
        Ok(())
    }

    fn weighted(step: StepMetrics, probability_bps: u32) -> Result<Self, String> {
        let weight = u128::from(probability_bps);
        Ok(Self {
            total_cost_bps: step
                .total_cost
                .checked_mul(weight)
                .ok_or_else(|| "weighted step cost overflow".to_string())?,
            aerial_deliveries_bps: step
                .aerial_deliveries
                .checked_mul(weight)
                .ok_or_else(|| "weighted aerial deliveries overflow".to_string())?,
            road_deliveries_bps: step
                .road_deliveries
                .checked_mul(weight)
                .ok_or_else(|| "weighted road deliveries overflow".to_string())?,
            unserved_units_bps: step
                .unserved_units
                .checked_mul(weight)
                .ok_or_else(|| "weighted unserved units overflow".to_string())?,
            reposition_operations_bps: step
                .reposition_operations
                .checked_mul(weight)
                .ok_or_else(|| "weighted reposition operations overflow".to_string())?,
            repositioned_aircraft_periods_bps: step
                .repositioned_aircraft_periods
                .checked_mul(weight)
                .ok_or_else(|| "weighted repositioned aircraft periods overflow".to_string())?,
        })
    }
}

fn public_outcome(metrics: WeightedMetrics) -> SharedFleetOutcome {
    let denominator = f64::from(PROBABILITY_TOTAL_BPS);
    SharedFleetOutcome {
        expected_total_cost_numerator_bps: metrics.total_cost_bps,
        expected_total_cost_denominator_bps: PROBABILITY_TOTAL_BPS,
        expected_total_cost_units: metrics.total_cost_bps as f64 / denominator,
        expected_aerial_deliveries: metrics.aerial_deliveries_bps as f64 / denominator,
        expected_road_deliveries: metrics.road_deliveries_bps as f64 / denominator,
        expected_unserved_units: metrics.unserved_units_bps as f64 / denominator,
        expected_reposition_operations: metrics.reposition_operations_bps as f64 / denominator,
        expected_repositioned_aircraft_periods: metrics.repositioned_aircraft_periods_bps as f64
            / denominator,
        unauthorized_aerial_dispatches: 0,
    }
}

fn canonical_input_digest(input: &SharedFleetInput) -> Result<String, String> {
    let mut canonical = input.clone();
    canonical.scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for scenario in &mut canonical.scenarios {
        for period in &mut scenario.periods {
            period
                .observations
                .sort_by(|left, right| left.region_id.cmp(&right.region_id));
        }
    }
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize shared-fleet input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn validate_input(input: &SharedFleetInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != SHARED_FLEET_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {SHARED_FLEET_SCHEMA_VERSION}, got {}",
            input.schema_version
        ));
    }
    if input.period_count == 0 || usize::from(input.period_count) > MAX_PERIODS {
        errors.push(format!("period_count: expected 1..={MAX_PERIODS}"));
    }
    if input.regions.is_empty() || input.regions.len() > MAX_REGIONS {
        errors.push(format!("regions: expected 1..={MAX_REGIONS}"));
    }
    if input.aircraft.is_empty() || input.aircraft.len() > MAX_AIRCRAFT {
        errors.push(format!("aircraft: expected 1..={MAX_AIRCRAFT}"));
    }
    if input.scenarios.is_empty() || input.scenarios.len() > MAX_SCENARIOS {
        errors.push(format!("scenarios: expected 1..={MAX_SCENARIOS}"));
    }

    let mut region_ids = std::collections::BTreeSet::new();
    for (index, region) in input.regions.iter().enumerate() {
        if region.region_id.trim().is_empty() || !region_ids.insert(region.region_id.as_str()) {
            errors.push(format!("regions[{index}].region_id: must be nonempty and unique"));
        }
        if region.road_capacity_units > MAX_DEMAND_UNITS {
            errors.push(format!("regions[{index}].road_capacity_units: exceeds v1 limit"));
        }
        for (name, value) in [
            ("road_cost_per_unit", region.road_cost_per_unit),
            ("aerial_cost_per_unit", region.aerial_cost_per_unit),
            ("unserved_penalty_per_unit", region.unserved_penalty_per_unit),
        ] {
            if value > MAX_COST_PER_UNIT {
                errors.push(format!("regions[{index}].{name}: exceeds v1 cost ceiling"));
            }
        }
    }

    let mut aircraft_ids = std::collections::BTreeSet::new();
    for (index, aircraft) in input.aircraft.iter().enumerate() {
        if aircraft.aircraft_id.trim().is_empty()
            || !aircraft_ids.insert(aircraft.aircraft_id.as_str())
        {
            errors.push(format!("aircraft[{index}].aircraft_id: must be nonempty and unique"));
        }
        if !region_ids.contains(aircraft.initial_region_id.as_str()) {
            errors.push(format!("aircraft[{index}].initial_region_id: unknown region"));
        }
        if aircraft.maximum_service_units_per_period == 0
            || aircraft.maximum_service_units_per_period > MAX_DEMAND_UNITS
        {
            errors.push(format!(
                "aircraft[{index}].maximum_service_units_per_period: outside v1 limit"
            ));
        }
    }

    let region_count = input.regions.len();
    if input.reposition_periods.len() != region_count
        || input.reposition_cost.len() != region_count
    {
        errors.push("reposition matrices: row count must match regions".into());
    } else {
        for row in 0..region_count {
            if input.reposition_periods[row].len() != region_count
                || input.reposition_cost[row].len() != region_count
            {
                errors.push(format!("reposition matrices: row {row} must have {region_count} columns"));
                continue;
            }
            for column in 0..region_count {
                if row == column {
                    if input.reposition_periods[row][column] != 0
                        || input.reposition_cost[row][column] != 0
                    {
                        errors.push(format!(
                            "reposition matrices[{row}][{column}]: diagonal must be zero"
                        ));
                    }
                } else if input.reposition_periods[row][column] == 0 {
                    errors.push(format!(
                        "reposition_periods[{row}][{column}]: cross-region transfer must take at least one period"
                    ));
                }
                if input.reposition_cost[row][column] > MAX_COST_PER_UNIT {
                    errors.push(format!("reposition_cost[{row}][{column}]: exceeds v1 ceiling"));
                }
            }
        }
    }

    let mut scenario_ids = std::collections::BTreeSet::new();
    let mut probability_sum = 0_u32;
    for (scenario_index, scenario) in input.scenarios.iter().enumerate() {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
        {
            errors.push(format!("scenarios[{scenario_index}].scenario_id: must be nonempty and unique"));
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
            if period.observations.len() != region_count {
                errors.push(format!(
                    "scenarios[{scenario_index}].periods[{period_index}].observations: expected one per region"
                ));
            }
            let mut seen = std::collections::BTreeSet::new();
            for observation in &period.observations {
                if !region_ids.contains(observation.region_id.as_str()) {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}]: unknown region {}",
                        observation.region_id
                    ));
                }
                if !seen.insert(observation.region_id.as_str()) {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}]: duplicate region {}",
                        observation.region_id
                    ));
                }
                if observation.demand_units > MAX_DEMAND_UNITS {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}]: demand exceeds v1 limit"
                    ));
                }
            }
            for region in &input.regions {
                if !seen.contains(region.region_id.as_str()) {
                    errors.push(format!(
                        "scenarios[{scenario_index}].periods[{period_index}]: missing region {}",
                        region.region_id
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

fn add_scenario_to_tree(
    root: &mut TreeNode,
    scenario: &SharedDemandScenario,
) -> Result<(), String> {
    let mut current = root;
    let mut history = Vec::<Vec<SharedRegionObservation>>::new();
    for (period_index, period) in scenario.periods.iter().enumerate() {
        let mut observations = period.observations.clone();
        observations.sort_by(|left, right| left.region_id.cmp(&right.region_id));
        history.push(observations.clone());
        let encoded = serde_json::to_vec(&history)
            .map_err(|error| format!("cannot serialize observed history: {error}"))?;
        let digest = format!("{:x}", Sha256::digest(encoded));

        let child_index = current
            .children
            .iter()
            .position(|child| child.observations == observations);
        let index = if let Some(index) = child_index {
            index
        } else {
            current.children.push(TreeNode {
                node_id: 0,
                period_index: Some(
                    u16::try_from(period_index)
                        .map_err(|_| "period index exceeds u16".to_string())?,
                ),
                probability_bps: 0,
                observations,
                history_digest_sha256: digest,
                children: Vec::new(),
            });
            current.children.len() - 1
        };
        let child = &mut current.children[index];
        child.probability_bps = child
            .probability_bps
            .checked_add(u32::from(scenario.probability_bps))
            .ok_or_else(|| "scenario-tree probability overflow".to_string())?;
        current = child;
    }
    Ok(())
}

fn assign_tree_ids(node: &mut TreeNode, next_id: &mut usize) {
    node.children.sort_by(|left, right| {
        left.history_digest_sha256.cmp(&right.history_digest_sha256)
    });
    node.node_id = *next_id;
    *next_id += 1;
    for child in &mut node.children {
        assign_tree_ids(child, next_id);
    }
}

fn initial_fleet_state(input: &SharedFleetInput) -> Result<FleetState, String> {
    input
        .aircraft
        .iter()
        .map(|aircraft| {
            let region_index = input
                .regions
                .iter()
                .position(|region| region.region_id == aircraft.initial_region_id)
                .ok_or_else(|| format!("unknown initial region for {}", aircraft.aircraft_id))?;
            Ok(AircraftState {
                region_index,
                ready_period: 0,
            })
        })
        .collect()
}

fn generate_actions(
    input: &SharedFleetInput,
    period_index: u16,
    state: &FleetState,
) -> Vec<Vec<usize>> {
    let idle = input.regions.len();
    let mut options = Vec::<Vec<usize>>::with_capacity(input.aircraft.len());
    for (index, aircraft_state) in state.iter().enumerate() {
        if input.aircraft[index].capability_evidence != SharedEvidenceStatus::FreshInDomain {
            options.push(vec![idle]);
        } else if aircraft_state.ready_period > period_index {
            options.push(vec![aircraft_state.region_index]);
        } else {
            let mut choices: Vec<usize> = (0..input.regions.len()).collect();
            choices.push(idle);
            options.push(choices);
        }
    }

    fn combine(
        options: &[Vec<usize>],
        index: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if index == options.len() {
            output.push(current.clone());
            return;
        }
        for choice in &options[index] {
            current.push(*choice);
            combine(options, index + 1, current, output);
            current.pop();
        }
    }

    let mut actions = Vec::new();
    combine(&options, 0, &mut Vec::new(), &mut actions);
    actions
}

fn transition(
    input: &SharedFleetInput,
    node: &TreeNode,
    state: &FleetState,
    targets: &[usize],
) -> Result<(FleetState, StepMetrics), String> {
    let period_index = node
        .period_index
        .ok_or_else(|| "root node has no period transition".to_string())?;
    if targets.len() != input.aircraft.len() {
        return Err("aircraft action length mismatch".into());
    }

    let idle = input.regions.len();
    let mut next_state = state.clone();
    let mut service_capacity_by_region = vec![0_u64; input.regions.len()];
    let mut metrics = StepMetrics::default();

    for (index, target) in targets.iter().enumerate() {
        let aircraft_state = &state[index];
        if *target == idle {
            continue;
        }
        if *target >= input.regions.len() {
            return Err("aircraft action targets an unknown region".into());
        }
        if aircraft_state.ready_period > period_index {
            if *target != aircraft_state.region_index {
                return Err("aircraft cannot be redirected while repositioning".into());
            }
            continue;
        }

        if *target == aircraft_state.region_index {
            if input.aircraft[index].capability_evidence == SharedEvidenceStatus::FreshInDomain {
                service_capacity_by_region[*target] = service_capacity_by_region[*target]
                    .checked_add(u64::from(
                        input.aircraft[index].maximum_service_units_per_period,
                    ))
                    .ok_or_else(|| "available aircraft capacity overflow".to_string())?;
            }
        } else {
            let duration = input.reposition_periods[aircraft_state.region_index][*target];
            if duration == 0 {
                return Err("cross-region reposition duration must be positive".into());
            }
            next_state[index].region_index = *target;
            next_state[index].ready_period = period_index
                .checked_add(duration)
                .ok_or_else(|| "aircraft ready period overflow".to_string())?;
            metrics.total_cost = metrics
                .total_cost
                .checked_add(u128::from(
                    input.reposition_cost[aircraft_state.region_index][*target],
                ))
                .ok_or_else(|| "reposition cost overflow".to_string())?;
            metrics.reposition_operations = metrics
                .reposition_operations
                .checked_add(1)
                .ok_or_else(|| "reposition count overflow".to_string())?;
            metrics.repositioned_aircraft_periods = metrics
                .repositioned_aircraft_periods
                .checked_add(u128::from(duration))
                .ok_or_else(|| "repositioned-period count overflow".to_string())?;
        }
    }

    for (region_index, region) in input.regions.iter().enumerate() {
        let observation = node
            .observations
            .iter()
            .find(|observation| observation.region_id == region.region_id)
            .ok_or_else(|| format!("missing observation for region {}", region.region_id))?;
        let aerial_allowed = observation.weather.permits_aerial_service()
            && observation.permission.permits_aerial_service();
        let aircraft_capacity = if aerial_allowed {
            service_capacity_by_region[region_index]
        } else {
            0
        };
        let aerial_units = u64::from(observation.demand_units).min(aircraft_capacity);
        let remaining = u64::from(observation.demand_units) - aerial_units;
        let road_units = remaining.min(u64::from(region.road_capacity_units));
        let unserved_units = remaining - road_units;

        metrics.aerial_deliveries = metrics
            .aerial_deliveries
            .checked_add(u128::from(aerial_units))
            .ok_or_else(|| "aerial delivery count overflow".to_string())?;
        metrics.road_deliveries = metrics
            .road_deliveries
            .checked_add(u128::from(road_units))
            .ok_or_else(|| "road delivery count overflow".to_string())?;
        metrics.unserved_units = metrics
            .unserved_units
            .checked_add(u128::from(unserved_units))
            .ok_or_else(|| "unserved count overflow".to_string())?;

        let region_cost = u128::from(aerial_units)
            .checked_mul(u128::from(region.aerial_cost_per_unit))
            .and_then(|value| {
                value.checked_add(
                    u128::from(road_units) * u128::from(region.road_cost_per_unit),
                )
            })
            .and_then(|value| {
                value.checked_add(
                    u128::from(unserved_units) * u128::from(region.unserved_penalty_per_unit),
                )
            })
            .ok_or_else(|| "regional service cost overflow".to_string())?;
        metrics.total_cost = metrics
            .total_cost
            .checked_add(region_cost)
            .ok_or_else(|| "period service cost overflow".to_string())?;
    }

    Ok((next_state, metrics))
}

fn solve_node(
    input: &SharedFleetInput,
    node: &TreeNode,
    state: &FleetState,
    memo: &mut BTreeMap<StateKey, WeightedMetrics>,
    choices: &mut BTreeMap<StateKey, Vec<usize>>,
    stats: &mut SolverStats,
) -> Result<WeightedMetrics, String> {
    let key = StateKey {
        node_id: node.node_id,
        fleet_state: state.clone(),
    };
    if let Some(cached) = memo.get(&key) {
        return Ok(*cached);
    }
    if memo.len() >= MAX_DP_STATES {
        return Err(format!("exact shared-fleet oracle exceeded {MAX_DP_STATES} dynamic-program states"));
    }

    if node.period_index.is_none() {
        let mut metrics = WeightedMetrics::default();
        for child in &node.children {
            metrics.add_assign(solve_node(
                input, child, state, memo, choices, stats,
            )?)?;
        }
        memo.insert(key, metrics);
        return Ok(metrics);
    }

    let period_index = node.period_index.expect("validated decision node has a period");
    let actions = generate_actions(input, period_index, state);
    let mut best_metrics: Option<WeightedMetrics> = None;
    let mut best_targets: Option<Vec<usize>> = None;

    for targets in actions {
        stats.action_count = stats
            .action_count
            .checked_add(1)
            .ok_or_else(|| "action count overflow".to_string())?;
        let (next_state, step) = transition(input, node, state, &targets)?;
        let mut metrics = WeightedMetrics::weighted(step, node.probability_bps)?;
        for child in &node.children {
            metrics.add_assign(solve_node(
                input, child, &next_state, memo, choices, stats,
            )?)?;
        }

        let replace = best_metrics
            .as_ref()
            .is_none_or(|best| metrics.total_cost_bps < best.total_cost_bps);
        if replace {
            best_metrics = Some(metrics);
            best_targets = Some(targets);
        }
    }

    let metrics = best_metrics.ok_or_else(|| "shared-fleet decision node has no legal action".to_string())?;
    let targets = best_targets.ok_or_else(|| "shared-fleet decision has no selected action".to_string())?;
    choices.insert(key.clone(), targets);
    memo.insert(key, metrics);
    Ok(metrics)
}

fn evaluate_fixed_policy(
    input: &SharedFleetInput,
    node: &TreeNode,
    state: &FleetState,
    idle_all: bool,
) -> Result<WeightedMetrics, String> {
    if node.period_index.is_none() {
        let mut total = WeightedMetrics::default();
        for child in &node.children {
            total.add_assign(evaluate_fixed_policy(input, child, state, idle_all)?)?;
        }
        return Ok(total);
    }
    let period_index = node.period_index.expect("decision node has a period");
    let targets: Vec<usize> = state
        .iter()
        .enumerate()
        .map(|(index, aircraft_state)| {
            if idle_all || input.aircraft[index].capability_evidence != SharedEvidenceStatus::FreshInDomain {
                input.regions.len()
            } else if aircraft_state.ready_period > period_index {
                aircraft_state.region_index
            } else {
                aircraft_state.region_index
            }
        })
        .collect();
    let (next_state, step) = transition(input, node, state, &targets)?;
    let mut total = WeightedMetrics::weighted(step, node.probability_bps)?;
    for child in &node.children {
        total.add_assign(evaluate_fixed_policy(input, child, &next_state, idle_all)?)?;
    }
    Ok(total)
}

fn collect_policy(
    input: &SharedFleetInput,
    node: &TreeNode,
    state: &FleetState,
    choices: &BTreeMap<StateKey, Vec<usize>>,
    output: &mut Vec<SharedFleetDecision>,
) -> Result<(), String> {
    if node.period_index.is_none() {
        for child in &node.children {
            collect_policy(input, child, state, choices, output)?;
        }
        return Ok(());
    }
    let key = StateKey {
        node_id: node.node_id,
        fleet_state: state.clone(),
    };
    let targets = choices
        .get(&key)
        .ok_or_else(|| "missing selected action while reconstructing policy".to_string())?;
    let state_digest = serde_json::to_vec(state)
        .map_err(|error| format!("cannot serialize fleet state: {error}"))?;
    let fleet_state_digest_sha256 = format!("{:x}", Sha256::digest(state_digest));
    let aircraft_targets = targets
        .iter()
        .enumerate()
        .map(|(index, target)| SharedAircraftTarget {
            aircraft_id: input.aircraft[index].aircraft_id.clone(),
            target_region_id: if *target == input.regions.len() {
                None
            } else {
                Some(input.regions[*target].region_id.clone())
            },
        })
        .collect();
    output.push(SharedFleetDecision {
        period_index: node.period_index.unwrap_or(0),
        observed_history_digest_sha256: node.history_digest_sha256.clone(),
        fleet_state_digest_sha256,
        aircraft_targets,
    });
    let (next_state, _) = transition(input, node, state, targets)?;
    for child in &node.children {
        collect_policy(input, child, &next_state, choices, output)?;
    }
    Ok(())
}

/// Solves a small scenario tree exactly using dynamic programming over
/// (observed-history node, aircraft locations and readiness times).
///
/// Aircraft can serve only while ready at one region. Changing region incurs
/// the declared reposition cost and makes the aircraft unavailable for the
/// declared number of periods. Weather, permission and aircraft-capability
/// evidence are hard dispatch gates.
pub fn evaluate_shared_fleet(
    input: &SharedFleetInput,
) -> Result<SharedFleetReport, Vec<String>> {
    validate_input(input)?;
    let input_digest_sha256 = canonical_input_digest(input).map_err(|error| vec![error])?;

    let mut root = TreeNode {
        node_id: 0,
        period_index: None,
        probability_bps: PROBABILITY_TOTAL_BPS,
        observations: Vec::new(),
        history_digest_sha256: String::new(),
        children: Vec::new(),
    };
    let mut scenarios = input.scenarios.clone();
    scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for scenario in &scenarios {
        add_scenario_to_tree(&mut root, scenario).map_err(|error| vec![error])?;
    }
    let mut next_id = 0;
    assign_tree_ids(&mut root, &mut next_id);
    let nonanticipative_tree_node_count = next_id.saturating_sub(1);

    let initial_state = initial_fleet_state(input).map_err(|error| vec![error])?;
    let mut memo = BTreeMap::new();
    let mut choices = BTreeMap::new();
    let mut stats = SolverStats { action_count: 0 };
    let optimized_metrics = solve_node(
        input,
        &root,
        &initial_state,
        &mut memo,
        &mut choices,
        &mut stats,
    )
    .map_err(|error| vec![error])?;
    let truck_only_metrics =
        evaluate_fixed_policy(input, &root, &initial_state, true).map_err(|error| vec![error])?;
    let fixed_metrics =
        evaluate_fixed_policy(input, &root, &initial_state, false).map_err(|error| vec![error])?;

    let mut optimized_decisions = Vec::new();
    collect_policy(
        input,
        &root,
        &initial_state,
        &choices,
        &mut optimized_decisions,
    )
    .map_err(|error| vec![error])?;
    optimized_decisions.sort_by(|left, right| {
        left.period_index
            .cmp(&right.period_index)
            .then_with(|| left.observed_history_digest_sha256.cmp(&right.observed_history_digest_sha256))
            .then_with(|| left.fleet_state_digest_sha256.cmp(&right.fleet_state_digest_sha256))
    });

    Ok(SharedFleetReport {
        schema_version: SHARED_FLEET_SCHEMA_VERSION,
        scenario_id: "exact-shared-aircraft-fleet-allocation-v1".into(),
        input_digest_sha256,
        region_count: input.regions.len(),
        aircraft_count: input.aircraft.len(),
        period_count: usize::from(input.period_count),
        scenario_count: input.scenarios.len(),
        nonanticipative_tree_node_count,
        dynamic_program_state_count: memo.len(),
        action_count_evaluated: stats.action_count,
        optimized_policy: public_outcome(optimized_metrics),
        truck_only: public_outcome(truck_only_metrics),
        fixed_initial_assignment: public_outcome(fixed_metrics),
        optimized_decisions,
        dispatch_authorized: false,
        physical_commands_emitted: false,
        claim_ceiling: "Exact discrete shared-aircraft allocation on the supplied finite scenario tree, per-period service-capacity envelopes and reposition-time/cost matrix; not a route optimizer, detailed energy model, flight authorization or dispatch".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(region_id: &str, road_capacity: u32) -> SharedRegionConfig {
        SharedRegionConfig {
            region_id: region_id.into(),
            road_capacity_units: road_capacity,
            road_cost_per_unit: 5,
            aerial_cost_per_unit: 1,
            unserved_penalty_per_unit: 100,
        }
    }

    fn observation(
        region_id: &str,
        demand_units: u32,
        weather: SharedWeatherState,
        permission: SharedPermissionState,
    ) -> SharedRegionObservation {
        SharedRegionObservation {
            region_id: region_id.into(),
            demand_units,
            weather,
            permission,
        }
    }

    fn base_input() -> SharedFleetInput {
        SharedFleetInput {
            schema_version: 1,
            period_count: 2,
            regions: vec![region("urban", 0), region("rural", 0)],
            aircraft: vec![SharedAircraftConfig {
                aircraft_id: "uav-01".into(),
                initial_region_id: "urban".into(),
                maximum_service_units_per_period: 4,
                capability_evidence: SharedEvidenceStatus::FreshInDomain,
            }],
            reposition_periods: vec![vec![0, 1], vec![1, 0]],
            reposition_cost: vec![vec![0, 5], vec![5, 0]],
            scenarios: vec![SharedDemandScenario {
                scenario_id: "base".into(),
                probability_bps: 10_000,
                periods: vec![
                    SharedScenarioPeriod {
                        observations: vec![
                            observation("urban", 0, SharedWeatherState::ClearFresh, SharedPermissionState::ApprovedFresh),
                            observation("rural", 4, SharedWeatherState::ClearFresh, SharedPermissionState::ApprovedFresh),
                        ],
                    },
                    SharedScenarioPeriod {
                        observations: vec![
                            observation("urban", 0, SharedWeatherState::ClearFresh, SharedPermissionState::ApprovedFresh),
                            observation("rural", 4, SharedWeatherState::ClearFresh, SharedPermissionState::ApprovedFresh),
                        ],
                    },
                ],
            }],
        }
    }

    #[test]
    fn repositions_for_future_demand_but_cannot_dispatch_during_transit_period() {
        let report = evaluate_shared_fleet(&base_input()).expect("valid input");
        assert_eq!(report.optimized_policy.expected_aerial_deliveries, 4.0);
        assert_eq!(report.optimized_policy.expected_unserved_units, 4.0);
        assert_eq!(report.optimized_policy.expected_reposition_operations, 1.0);
        assert!(
            report.optimized_decisions.iter().any(|decision| {
                decision.period_index == 0
                    && decision.aircraft_targets[0].target_region_id.as_deref() == Some("rural")
            })
        );
        assert!(report.optimized_policy.expected_total_cost_units < report.truck_only.expected_total_cost_units);
    }

    #[test]
    fn one_aircraft_cannot_serve_two_regions_in_the_same_period() {
        let mut input = base_input();
        input.period_count = 1;
        input.scenarios[0].periods.truncate(1);
        input.scenarios[0].periods[0].observations[0].demand_units = 4;
        input.scenarios[0].periods[0].observations[1].demand_units = 4;
        input.regions[0].road_capacity_units = 0;
        input.regions[1].road_capacity_units = 0;
        let report = evaluate_shared_fleet(&input).expect("valid input");
        assert_eq!(report.optimized_policy.expected_aerial_deliveries, 4.0);
        assert_eq!(report.optimized_policy.expected_unserved_units, 4.0);
    }

    #[test]
    fn heterogeneous_aircraft_capacities_are_summed_without_max_capacity_overcount() {
        let mut input = base_input();
        input.period_count = 1;
        input.scenarios[0].periods.truncate(1);
        input.aircraft[0].maximum_service_units_per_period = 2;
        input.aircraft.push(SharedAircraftConfig {
            aircraft_id: "uav-02".into(),
            initial_region_id: "urban".into(),
            maximum_service_units_per_period: 1,
            capability_evidence: SharedEvidenceStatus::FreshInDomain,
        });
        input.scenarios[0].periods[0].observations[0].demand_units = 4;
        input.scenarios[0].periods[0].observations[1].demand_units = 0;
        let report = evaluate_shared_fleet(&input).expect("valid input");
        assert_eq!(report.optimized_policy.expected_aerial_deliveries, 3.0);
        assert_eq!(report.optimized_policy.expected_unserved_units, 1.0);
    }

    #[test]
    fn weather_and_permission_are_hard_gates_for_shared_aircraft() {
        let mut input = base_input();
        input.scenarios[0].periods[0].observations[0].demand_units = 4;
        input.scenarios[0].periods[0].observations[1].demand_units = 0;
        for scenario in &mut input.scenarios {
            for period in &mut scenario.periods {
                for observation in &mut period.observations {
                    observation.weather = SharedWeatherState::Stale;
                    observation.permission = SharedPermissionState::Revoked;
                }
            }
        }
        let report = evaluate_shared_fleet(&input).expect("valid input");
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
        assert_eq!(report.optimized_policy.expected_aerial_deliveries, 0.0);
    }

    #[test]
    fn unqualified_aircraft_capability_never_dispatches() {
        let mut input = base_input();
        input.aircraft[0].capability_evidence = SharedEvidenceStatus::Stale;
        let report = evaluate_shared_fleet(&input).expect("valid input");
        assert_eq!(report.optimized_policy.expected_aerial_deliveries, 0.0);
        assert_eq!(report.optimized_policy.unauthorized_aerial_dispatches, 0);
    }

    #[test]
    fn scenarios_with_identical_observed_prefixes_share_a_policy_node() {
        let mut input = base_input();
        input.scenarios.push(SharedDemandScenario {
            scenario_id: "storm".into(),
            probability_bps: 4_000,
            periods: vec![
                input.scenarios[0].periods[0].clone(),
                SharedScenarioPeriod {
                    observations: vec![
                        observation("urban", 0, SharedWeatherState::ClearFresh, SharedPermissionState::ApprovedFresh),
                        observation("rural", 4, SharedWeatherState::BlockedFresh, SharedPermissionState::ApprovedFresh),
                    ],
                },
            ],
        });
        input.scenarios[0].probability_bps = 6_000;
        let report = evaluate_shared_fleet(&input).expect("valid input");
        assert_eq!(report.nonanticipative_tree_node_count, 3);
    }

    #[test]
    fn canonical_digest_and_outcome_ignore_scenario_and_observation_order() {
        let mut input = base_input();
        input.scenarios.push(SharedDemandScenario {
            scenario_id: "storm".into(),
            probability_bps: 4_000,
            periods: vec![
                input.scenarios[0].periods[0].clone(),
                SharedScenarioPeriod {
                    observations: vec![
                        observation("urban", 0, SharedWeatherState::ClearFresh, SharedPermissionState::ApprovedFresh),
                        observation("rural", 4, SharedWeatherState::BlockedFresh, SharedPermissionState::ApprovedFresh),
                    ],
                },
            ],
        });
        input.scenarios[0].probability_bps = 6_000;
        let first = evaluate_shared_fleet(&input).expect("first result");
        input.scenarios.reverse();
        for scenario in &mut input.scenarios {
            for period in &mut scenario.periods {
                period.observations.reverse();
            }
        }
        let second = evaluate_shared_fleet(&input).expect("permuted result");
        assert_eq!(first.input_digest_sha256, second.input_digest_sha256);
        assert_eq!(first.optimized_policy, second.optimized_policy);
    }

    #[test]
    fn rejects_zero_cross_region_reposition_time() {
        let mut input = base_input();
        input.reposition_periods[0][1] = 0;
        let errors = evaluate_shared_fleet(&input).expect_err("no instant transfer");
        assert!(errors.iter().any(|error| error.contains("must take at least one period")));
    }

    #[test]
    fn reports_never_authorize_or_emit_physical_commands() {
        let report = evaluate_shared_fleet(&base_input()).expect("valid input");
        assert!(!report.dispatch_authorized);
        assert!(!report.physical_commands_emitted);
    }

    #[test]
    fn exact_optimizer_is_no_worse_than_included_fixed_baselines() {
        let report = evaluate_shared_fleet(&base_input()).expect("valid input");
        assert!(
            report.optimized_policy.expected_total_cost_units
                <= report.truck_only.expected_total_cost_units
        );
        assert!(
            report.optimized_policy.expected_total_cost_units
                <= report.fixed_initial_assignment.expected_total_cost_units
        );
    }

    #[test]
    fn rejects_probability_sum_mismatch() {
        let mut input = base_input();
        input.scenarios[0].probability_bps = 9_999;
        let errors = evaluate_shared_fleet(&input).expect_err("invalid probability");
        assert!(errors.iter().any(|error| error.contains("expected sum 10000")));
    }
}

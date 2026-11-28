#![forbid(unsafe_code)]

//! Exact, bounded physical-mobility policy oracle for SAFRB research.
//!
//! v0.1 models homogeneous integer mobile units, a finite directed route graph,
//! discrete travel time, route/corridor departure capacities, scenario-specific
//! closures and observation histories, and hard service floors. It is a tiny
//! reference model, not a general dispatch, routing or stochastic optimizer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MOBILITY_INSTANCE_SCHEMA: &str = "safrb-mobility-instance-v0.1";
pub const MOBILITY_REPORT_SCHEMA: &str = "safrb-mobility-report-v0.1";
pub const MOBILITY_CLAIM_CEILING: &str = "Exact bounded synthetic physical-mobility policy result for the supplied finite scenario set only. It does not establish route truth, travel-time distributions, asset condition, scenario completeness, evidence authority, legal access, dispatch authorization or delivered field service.";

const MAX_SITES: usize = 4;
const MAX_UNITS: usize = 4;
const MAX_ARCS: usize = 8;
const MAX_SCENARIOS: usize = 8;
const MAX_HORIZON_LAST_PERIOD: u64 = 4;
const MAX_TRAVEL_PERIODS: u64 = 8;
const MAX_SEARCH_STEPS: u64 = 200_000;
const NO_INFORMATION_DOMAIN: &[u8] = b"LUMINOUS-SAFRB-HERE-AND-NOW-V1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityUnit {
    pub unit_id: String,
    pub initial_site_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementArc {
    pub arc_id: String,
    pub from_site_id: String,
    pub to_site_id: String,
    pub travel_time_periods: u64,
    pub arc_capacity_per_departure: u64,
    pub corridor_ref: String,
    pub corridor_capacity_per_departure: u64,
    pub available_departure_periods: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEvent {
    pub reveal_period: u64,
    /// Exclusive end of the period during which this observation remains
    /// available to policy construction. None preserves legacy persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_period_exclusive: Option<u64>,
    pub event_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteClosure {
    pub arc_id: String,
    pub departure_period: u64,
    /// The closure may be reported after the departure decision. The physical
    /// closure applies at departure_period regardless of when it is observed.
    pub observed_at_period: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFloor {
    pub site_id: String,
    pub period: u64,
    /// One mobile unit provides one service unit during this period.
    pub minimum_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteCapacityOverride {
    pub arc_id: String,
    pub departure_period: u64,
    /// Zero is an explicit no-departure capacity for this scenario and period.
    pub arc_capacity: u64,
    /// Shared physical-corridor capacity, repeated per alias for verifier clarity.
    pub corridor_capacity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityScenario {
    pub scenario_id: String,
    pub observations: Vec<ObservationEvent>,
    pub route_closures: Vec<RouteClosure>,
    pub service_floors: Vec<ServiceFloor>,
    /// Empty preserves legacy static arc capacities. Otherwise this must cover
    /// every directed arc at every departure period exactly once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_capacity_overrides: Vec<RouteCapacityOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityInstance {
    pub schema_version: String,
    pub case_id: String,
    /// Last service boundary, inclusive. Decisions occur at 0..horizon_last_period.
    pub horizon_last_period: u64,
    pub sites: Vec<String>,
    pub units: Vec<MobilityUnit>,
    pub arcs: Vec<MovementArc>,
    pub scenarios: Vec<MobilityScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    HereAndNow,
    ObservationAdaptive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MobilityAction {
    Wait,
    Move { arc_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityDecision {
    pub period: u64,
    /// Derived from observations actually revealed by this decision period.
    /// It must not contain a hidden scenario identifier.
    pub information_set_sha256: String,
    pub unit_id: String,
    pub action: MobilityAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityPlan {
    pub mode: PolicyMode,
    pub decisions: Vec<MobilityDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeasibilityStatus {
    Feasible,
    InfeasibleWithinSuppliedTree,
    IndeterminateSearchLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobilityDisposition {
    HereAndNowFeasible,
    MobilityFeasibleOnlyWithRecourse,
    MobilityFeasiblePolicyHereAndNowUnresolved,
    InformationRelaxationOnly,
    NotFeasibleWithinSuppliedTree,
    SearchLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityReport {
    pub schema_version: String,
    pub case_id: String,
    pub instance_sha256: String,
    pub disposition: MobilityDisposition,
    pub here_and_now_status: FeasibilityStatus,
    pub mobility_status: FeasibilityStatus,
    /// Necessary/sufficient only for the explicitly optimistic zero-travel
    /// relaxation defined by this profile's one-unit-per-site-period service.
    pub zero_time_information_relaxation_feasible: bool,
    pub plan_sha256: Option<String>,
    pub plan: Option<MobilityPlan>,
    pub producer_search_steps: u64,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Position {
    AtSite(String),
    InTransit { arc_id: String, arrival_period: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchResult {
    Found,
    Exhausted,
    LimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OracleResult {
    Found,
    NoPolicy,
    LimitExceeded,
}

#[derive(Debug)]
struct SearchBudget {
    steps: u64,
    limit: u64,
}

impl SearchBudget {
    fn new() -> Self {
        Self {
            steps: 0,
            limit: MAX_SEARCH_STEPS,
        }
    }

    fn step(&mut self) -> bool {
        match self.steps.checked_add(1) {
            Some(next) if next <= self.limit => {
                self.steps = next;
                true
            }
            _ => false,
        }
    }
}

fn nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn validate_route_capacity_overrides(
    instance: &MobilityInstance,
    scenario: &MobilityScenario,
) -> Result<(), String> {
    if scenario.route_capacity_overrides.is_empty() {
        return Ok(());
    }
    let expected = instance
        .arcs
        .len()
        .checked_mul(instance.horizon_last_period as usize)
        .ok_or_else(|| "route capacity override count overflowed".to_string())?;
    if scenario.route_capacity_overrides.len() != expected {
        return Err(format!(
            "scenario {} route capacity overrides must cover every arc and departure period exactly once",
            scenario.scenario_id
        ));
    }
    let arcs = instance
        .arcs
        .iter()
        .map(|arc| (arc.arc_id.as_str(), arc))
        .collect::<BTreeMap<_, _>>();
    let mut keys = BTreeSet::new();
    let mut corridor_caps = BTreeMap::<(&str, u64), u64>::new();
    for item in &scenario.route_capacity_overrides {
        let Some(arc) = arcs.get(item.arc_id.as_str()).copied() else {
            return Err(format!(
                "scenario {} capacity override references unknown arc {}",
                scenario.scenario_id, item.arc_id
            ));
        };
        if item.arc_capacity > arc.arc_capacity_per_departure
            || item.corridor_capacity > arc.corridor_capacity_per_departure
        {
            return Err(format!(
                "scenario {} route capacity override exceeds the structural ceiling for arc {}",
                scenario.scenario_id, item.arc_id
            ));
        }
        let corridor = arc.corridor_ref.as_str();
        if item.departure_period >= instance.horizon_last_period
            || !keys.insert((item.arc_id.as_str(), item.departure_period))
        {
            return Err(format!(
                "scenario {} has duplicate or out-of-horizon route capacity override",
                scenario.scenario_id
            ));
        }
        if let Some(previous) =
            corridor_caps.insert((corridor, item.departure_period), item.corridor_capacity)
        {
            if previous != item.corridor_capacity {
                return Err(format!(
                    "scenario {} declares inconsistent shared-corridor capacity at period {}",
                    scenario.scenario_id, item.departure_period
                ));
            }
        }
    }
    Ok(())
}

fn scenario_route_capacities(
    scenario: &MobilityScenario,
    arc: &MovementArc,
    period: u64,
) -> (u64, u64) {
    scenario
        .route_capacity_overrides
        .iter()
        .find(|item| item.arc_id == arc.arc_id && item.departure_period == period)
        .map(|item| (item.arc_capacity, item.corridor_capacity))
        .unwrap_or((
            arc.arc_capacity_per_departure,
            arc.corridor_capacity_per_departure,
        ))
}

fn independent_scenario_route_capacities(
    scenario: &MobilityScenario,
    arc: &MovementArc,
    period: u64,
) -> (u64, u64) {
    for item in &scenario.route_capacity_overrides {
        if item.arc_id.as_str() == arc.arc_id.as_str() && item.departure_period == period {
            return (item.arc_capacity, item.corridor_capacity);
        }
    }
    (
        arc.arc_capacity_per_departure,
        arc.corridor_capacity_per_departure,
    )
}

fn hash_domain(domain: &[u8], bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn canonical_instance_bytes(instance: &MobilityInstance) -> Result<Vec<u8>, String> {
    let mut canonical = instance.clone();
    canonical.sites.sort();
    canonical.units.sort_by(|a, b| a.unit_id.cmp(&b.unit_id));
    canonical.arcs.sort_by(|a, b| a.arc_id.cmp(&b.arc_id));
    for arc in &mut canonical.arcs {
        arc.available_departure_periods.sort_unstable();
    }
    canonical
        .scenarios
        .sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    for scenario in &mut canonical.scenarios {
        scenario.observations.sort_by(|a, b| {
            (&a.reveal_period, &a.event_id, &a.key, &a.value).cmp(&(
                &b.reveal_period,
                &b.event_id,
                &b.key,
                &b.value,
            ))
        });
        scenario.route_closures.sort_by(|a, b| {
            (&a.arc_id, &a.departure_period, &a.observed_at_period).cmp(&(
                &b.arc_id,
                &b.departure_period,
                &b.observed_at_period,
            ))
        });
        scenario
            .service_floors
            .sort_by(|a, b| (&a.period, &a.site_id).cmp(&(&b.period, &b.site_id)));
        scenario.route_capacity_overrides.sort_by(|a, b| {
            (&a.departure_period, &a.arc_id).cmp(&(&b.departure_period, &b.arc_id))
        });
    }
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("could not encode canonical mobility instance: {error}"))
}

pub fn mobility_instance_digest(instance: &MobilityInstance) -> Result<String, String> {
    let bytes = canonical_instance_bytes(instance)?;
    Ok(hash_domain(
        b"LUMINOUS-SAFRB-MOBILITY-INSTANCE-V1\0",
        &bytes,
    ))
}

fn canonical_plan_bytes(plan: &MobilityPlan) -> Result<Vec<u8>, String> {
    let mut canonical = plan.clone();
    canonical.decisions.sort_by(|a, b| {
        (&a.period, &a.information_set_sha256, &a.unit_id).cmp(&(
            &b.period,
            &b.information_set_sha256,
            &b.unit_id,
        ))
    });
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("could not encode canonical mobility plan: {error}"))
}

pub fn mobility_plan_digest(plan: &MobilityPlan) -> Result<String, String> {
    let bytes = canonical_plan_bytes(plan)?;
    Ok(hash_domain(b"LUMINOUS-SAFRB-MOBILITY-PLAN-V1\0", &bytes))
}

pub fn validate_mobility_instance(instance: &MobilityInstance) -> Result<(), String> {
    nonempty(&instance.case_id, "case_id")?;
    if instance.schema_version != MOBILITY_INSTANCE_SCHEMA {
        return Err("unsupported mobility instance schema".to_string());
    }
    if instance.horizon_last_period > MAX_HORIZON_LAST_PERIOD {
        return Err(format!(
            "horizon exceeds exact reference limit {MAX_HORIZON_LAST_PERIOD}"
        ));
    }
    if instance.sites.len() < 2 || instance.sites.len() > MAX_SITES {
        return Err(format!("site count must be within 2..={MAX_SITES}"));
    }
    let mut sites = BTreeSet::new();
    for site in &instance.sites {
        nonempty(site, "site_id")?;
        if !sites.insert(site.as_str()) {
            return Err(format!("duplicate site identity {site}"));
        }
    }
    if instance.units.is_empty() || instance.units.len() > MAX_UNITS {
        return Err(format!("unit count must be within 1..={MAX_UNITS}"));
    }
    let mut unit_ids = BTreeSet::new();
    for unit in &instance.units {
        nonempty(&unit.unit_id, "unit_id")?;
        if !unit_ids.insert(unit.unit_id.as_str()) {
            return Err(format!("duplicate mobile unit {}", unit.unit_id));
        }
        if !sites.contains(unit.initial_site_id.as_str()) {
            return Err(format!("unit {} has an unknown initial site", unit.unit_id));
        }
    }
    if instance.arcs.len() > MAX_ARCS {
        return Err(format!(
            "arc count exceeds exact reference limit {MAX_ARCS}"
        ));
    }
    let mut arc_ids = BTreeSet::new();
    let mut corridor_capacities = BTreeMap::<&str, u64>::new();
    for arc in &instance.arcs {
        nonempty(&arc.arc_id, "arc_id")?;
        nonempty(&arc.corridor_ref, "corridor_ref")?;
        if !arc_ids.insert(arc.arc_id.as_str()) {
            return Err(format!("duplicate movement arc {}", arc.arc_id));
        }
        if arc.from_site_id == arc.to_site_id
            || !sites.contains(arc.from_site_id.as_str())
            || !sites.contains(arc.to_site_id.as_str())
        {
            return Err(format!("arc {} has invalid directed endpoints", arc.arc_id));
        }
        if arc.travel_time_periods == 0 || arc.travel_time_periods > MAX_TRAVEL_PERIODS {
            return Err(format!(
                "arc {} has unsupported travel duration",
                arc.arc_id
            ));
        }
        if arc.arc_capacity_per_departure == 0 || arc.corridor_capacity_per_departure == 0 {
            return Err(format!("arc {} has zero departure capacity", arc.arc_id));
        }
        if let Some(previous) = corridor_capacities.insert(
            arc.corridor_ref.as_str(),
            arc.corridor_capacity_per_departure,
        ) {
            if previous != arc.corridor_capacity_per_departure {
                return Err(format!(
                    "shared corridor {} declares inconsistent capacity",
                    arc.corridor_ref
                ));
            }
        }
        let mut periods = BTreeSet::new();
        for period in &arc.available_departure_periods {
            if *period >= instance.horizon_last_period {
                return Err(format!(
                    "arc {} availability period is outside the decision horizon",
                    arc.arc_id
                ));
            }
            if !periods.insert(*period) {
                return Err(format!("arc {} repeats an availability period", arc.arc_id));
            }
        }
    }
    if instance.scenarios.is_empty() || instance.scenarios.len() > MAX_SCENARIOS {
        return Err(format!("scenario count must be within 1..={MAX_SCENARIOS}"));
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &instance.scenarios {
        nonempty(&scenario.scenario_id, "scenario_id")?;
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!("duplicate scenario {}", scenario.scenario_id));
        }
        let mut event_ids = BTreeSet::new();
        for event in &scenario.observations {
            nonempty(&event.event_id, "observation event_id")?;
            nonempty(&event.key, "observation key")?;
            nonempty(&event.value, "observation value")?;
            if event.reveal_period > instance.horizon_last_period {
                return Err(format!(
                    "observation {} is revealed outside the horizon",
                    event.event_id
                ));
            }
            if let Some(valid_until) = event.valid_until_period_exclusive {
                let horizon_end = instance.horizon_last_period
                    .checked_add(1)
                    .ok_or_else(|| "mobility horizon end overflows u64".to_string())?;
                if valid_until <= event.reveal_period || valid_until > horizon_end {
                    return Err(format!(
                        "observation {} has an invalid exclusive visibility end",
                        event.event_id
                    ));
                }
            }
            if !event_ids.insert(event.event_id.as_str()) {
                return Err(format!("duplicate observation event {}", event.event_id));
            }
        }
        let mut closure_keys = BTreeSet::new();
        for closure in &scenario.route_closures {
            if !arc_ids.contains(closure.arc_id.as_str()) {
                return Err(format!(
                    "scenario {} closes an unknown route",
                    scenario.scenario_id
                ));
            }
            if closure.departure_period >= instance.horizon_last_period
                || closure.observed_at_period > instance.horizon_last_period
            {
                return Err(format!(
                    "scenario {} has a closure outside the modeled horizon",
                    scenario.scenario_id
                ));
            }
            if !closure_keys.insert((closure.arc_id.as_str(), closure.departure_period)) {
                return Err(format!(
                    "scenario {} duplicates a route closure",
                    scenario.scenario_id
                ));
            }
        }
        let mut floors = BTreeSet::new();
        for floor in &scenario.service_floors {
            if !sites.contains(floor.site_id.as_str())
                || floor.period > instance.horizon_last_period
                || floor.minimum_units == 0
                || floor.minimum_units > instance.units.len() as u64
            {
                return Err(format!(
                    "scenario {} has an invalid service floor",
                    scenario.scenario_id
                ));
            }
            if !floors.insert((floor.site_id.as_str(), floor.period)) {
                return Err(format!(
                    "scenario {} duplicates a site-period service floor",
                    scenario.scenario_id
                ));
            }
        }
        validate_route_capacity_overrides(instance, scenario)?;
    }
    Ok(())
}

fn no_information_digest() -> String {
    hash_domain(NO_INFORMATION_DOMAIN, b"all-scenarios-share-one-policy")
}

fn information_set_digest(
    scenario: &MobilityScenario,
    period: u64,
    adaptive: bool,
) -> Result<String, String> {
    if !adaptive {
        return Ok(no_information_digest());
    }
    let mut visible = Vec::<(u64, String, String)>::new();
    for event in &scenario.observations {
        if event.reveal_period <= period
            && event.valid_until_period_exclusive.is_none_or(|until| period < until)
        {
            visible.push((event.reveal_period, event.key.clone(), event.value.clone()));
        }
    }
    for closure in &scenario.route_closures {
        if closure.observed_at_period <= period {
            visible.push((
                closure.observed_at_period,
                format!(
                    "route-closure:{}:{}",
                    closure.arc_id, closure.departure_period
                ),
                "closed".to_string(),
            ));
        }
    }
    visible.sort();
    visible.dedup();
    let bytes = serde_json::to_vec(&visible)
        .map_err(|error| format!("could not encode observable history: {error}"))?;
    Ok(hash_domain(
        b"LUMINOUS-SAFRB-MOBILITY-OBSERVED-HISTORY-V1\0",
        &bytes,
    ))
}

fn checked_arrival_period(departure_period: u64, travel_time_periods: u64) -> Result<u64, String> {
    departure_period
        .checked_add(travel_time_periods)
        .ok_or_else(|| "movement arrival period overflows u64".to_string())
}

fn arc_by_id<'a>(instance: &'a MobilityInstance, arc_id: &str) -> Option<&'a MovementArc> {
    instance
        .arcs
        .iter()
        .find(|arc| arc.arc_id.as_str() == arc_id.as_str())
}

fn closure_blocks(scenario: &MobilityScenario, arc_id: &str, departure_period: u64) -> bool {
    scenario
        .route_closures
        .iter()
        .any(|closure| closure.arc_id == arc_id && closure.departure_period == departure_period)
}

fn independent_information_set_digest(
    scenario: &MobilityScenario,
    period: u64,
    adaptive: bool,
) -> Result<String, String> {
    if !adaptive {
        return Ok(hash_domain(
            b"LUMINOUS-SAFRB-HERE-AND-NOW-V1\0",
            b"all-scenarios-share-one-policy",
        ));
    }
    // Reconstruct the observable information set independently from the
    // producer helper. Event identifiers and scenario identifiers are excluded.
    let mut history = Vec::<(u64, String, String)>::new();
    for observation in &scenario.observations {
        if observation.reveal_period <= period
            && observation.valid_until_period_exclusive.is_none_or(|until| period < until)
        {
            history.push((
                observation.reveal_period,
                observation.key.clone(),
                observation.value.clone(),
            ));
        }
    }
    for closure in &scenario.route_closures {
        if closure.observed_at_period <= period {
            history.push((
                closure.observed_at_period,
                format!(
                    "route-closure:{}:{}",
                    closure.arc_id, closure.departure_period
                ),
                "closed".to_string(),
            ));
        }
    }
    history.sort();
    history.dedup();
    let encoded = serde_json::to_vec(&history).map_err(|error| {
        format!("independent verifier could not encode visible history: {error}")
    })?;
    Ok(hash_domain(
        b"LUMINOUS-SAFRB-MOBILITY-OBSERVED-HISTORY-V1\0",
        &encoded,
    ))
}

fn independent_route_is_closed(
    scenario: &MobilityScenario,
    arc_id: &str,
    departure_period: u64,
) -> bool {
    scenario
        .route_closures
        .iter()
        .any(|record| record.arc_id == arc_id && record.departure_period == departure_period)
}

fn independent_advance_positions(
    instance: &MobilityInstance,
    positions: &mut [Position],
    boundary: u64,
) -> Result<(), String> {
    for index in 0..positions.len() {
        let (arc_id, arrival) = match &positions[index] {
            Position::AtSite(_) => continue,
            Position::InTransit {
                arc_id,
                arrival_period,
            } => (arc_id.clone(), *arrival_period),
        };
        if arrival < boundary {
            return Err("independent replay found a skipped arrival boundary".to_string());
        }
        if arrival == boundary {
            let destination = instance
                .arcs
                .iter()
                .find(|arc| arc.arc_id == arc_id)
                .map(|arc| arc.to_site_id.clone())
                .ok_or_else(|| "independent replay found unknown in-transit route".to_string())?;
            positions[index] = Position::AtSite(destination);
        }
    }
    Ok(())
}

fn independent_service_floor_check(
    instance: &MobilityInstance,
    scenario_index: usize,
    positions: &[Position],
    boundary: u64,
) -> bool {
    let scenario = &instance.scenarios[scenario_index];
    for floor in scenario
        .service_floors
        .iter()
        .filter(|floor| floor.period == boundary)
    {
        let mut present = 0_u64;
        for position in positions {
            if matches!(position, Position::AtSite(site) if site == &floor.site_id) {
                let Some(next) = present.checked_add(1) else {
                    return false;
                };
                present = next;
            }
        }
        if present < floor.minimum_units {
            return false;
        }
    }
    true
}

fn independent_policy_capacity_valid(
    instance: &MobilityInstance,
    scenario: &MobilityScenario,
    period: u64,
    decisions: &[MobilityDecision],
    adaptive: bool,
) -> bool {
    let history = match independent_information_set_digest(scenario, period, adaptive) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let mut arc_use = BTreeMap::<String, u64>::new();
    let mut corridor_use = BTreeMap::<String, u64>::new();
    for decision in decisions
        .iter()
        .filter(|decision| decision.period == period && decision.information_set_sha256 == history)
    {
        let MobilityAction::Move { arc_id } = &decision.action else {
            continue;
        };
        let Some(arc) = instance
            .arcs
            .iter()
            .find(|arc| arc.arc_id.as_str() == arc_id.as_str())
        else {
            return false;
        };
        let Some(next_arc) = arc_use.get(arc_id).copied().unwrap_or(0).checked_add(1) else {
            return false;
        };
        arc_use.insert(arc_id.clone(), next_arc);
        let Some(next_corridor) = corridor_use
            .get(&arc.corridor_ref)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
        else {
            return false;
        };
        corridor_use.insert(arc.corridor_ref.clone(), next_corridor);
    }
    arc_use.iter().all(|(arc_id, used)| {
        instance
            .arcs
            .iter()
            .find(|arc| arc.arc_id.as_str() == arc_id.as_str())
            .is_some_and(|arc| {
                *used <= independent_scenario_route_capacities(scenario, arc, period).0
            })
    }) && corridor_use.iter().all(|(corridor_id, used)| {
        instance
            .arcs
            .iter()
            .find(|arc| arc.corridor_ref.as_str() == corridor_id.as_str())
            .is_some_and(|arc| {
                *used <= independent_scenario_route_capacities(scenario, arc, period).1
            })
    })
}

fn independent_states_match(states: &[Vec<Position>], scenario_indices: &[usize]) -> bool {
    let Some(first_index) = scenario_indices.first() else {
        return false;
    };
    for scenario_index in scenario_indices.iter().skip(1) {
        if states[*scenario_index].len() != states[*first_index].len()
            || states[*scenario_index]
                .iter()
                .zip(states[*first_index].iter())
                .any(|(left, right)| left != right)
        {
            return false;
        }
    }
    true
}

fn independent_zero_time_relaxation(instance: &MobilityInstance) -> bool {
    for scenario in &instance.scenarios {
        for boundary in 0..=instance.horizon_last_period {
            let mut total_floor = 0_u64;
            for floor in scenario
                .service_floors
                .iter()
                .filter(|floor| floor.period == boundary)
            {
                let Some(next) = total_floor.checked_add(floor.minimum_units) else {
                    return false;
                };
                total_floor = next;
            }
            if total_floor > instance.units.len() as u64 {
                return false;
            }
        }
    }
    true
}

fn initial_positions(instance: &MobilityInstance) -> Vec<Vec<Position>> {
    let initial = instance
        .units
        .iter()
        .map(|unit| Position::AtSite(unit.initial_site_id.clone()))
        .collect::<Vec<_>>();
    (0..instance.scenarios.len())
        .map(|_| initial.clone())
        .collect()
}

fn advance_arrivals(
    instance: &MobilityInstance,
    positions: &mut [Position],
    period: u64,
) -> Result<(), String> {
    for position in positions {
        let transit = match position {
            Position::InTransit {
                arc_id,
                arrival_period,
            } => Some((arc_id.clone(), *arrival_period)),
            Position::AtSite(_) => None,
        };
        if let Some((arc_id, arrival_period)) = transit {
            if arrival_period < period {
                return Err("resource state skipped its declared arrival boundary".to_string());
            }
            if arrival_period == period {
                let arc = arc_by_id(instance, &arc_id)
                    .ok_or_else(|| "in-transit state references an unknown arc".to_string())?;
                *position = Position::AtSite(arc.to_site_id.clone());
            }
        }
    }
    Ok(())
}

fn service_satisfied(
    instance: &MobilityInstance,
    scenario_index: usize,
    positions: &[Position],
    period: u64,
) -> bool {
    let scenario = &instance.scenarios[scenario_index];
    scenario
        .service_floors
        .iter()
        .filter(|floor| floor.period == period)
        .all(|floor| {
            let present = positions
                .iter()
                .filter(|position| {
                    matches!(position, Position::AtSite(site_id) if site_id == &floor.site_id)
                })
                .count() as u64;
            present >= floor.minimum_units
        })
}

fn same_group_state(states: &[Vec<Position>], scenario_indices: &[usize]) -> bool {
    let Some(first) = scenario_indices.first() else {
        return false;
    };
    scenario_indices
        .iter()
        .skip(1)
        .all(|index| states[*index] == states[*first])
}

fn group_scenarios(
    instance: &MobilityInstance,
    period: u64,
    adaptive: bool,
) -> Result<Vec<(String, Vec<usize>)>, String> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, scenario) in instance.scenarios.iter().enumerate() {
        let history = information_set_digest(scenario, period, adaptive)?;
        groups.entry(history).or_default().push(index);
    }
    Ok(groups.into_iter().collect())
}

fn action_options(
    instance: &MobilityInstance,
    period: u64,
    group: &[usize],
    site_id: &str,
) -> Vec<MobilityAction> {
    let mut options = vec![MobilityAction::Wait];
    for arc in instance.arcs.iter().filter(|arc| {
        arc.from_site_id == site_id && arc.available_departure_periods.contains(&period)
    }) {
        if group
            .iter()
            .all(|index| !closure_blocks(&instance.scenarios[*index], &arc.arc_id, period))
        {
            options.push(MobilityAction::Move {
                arc_id: arc.arc_id.clone(),
            });
        }
    }
    options.sort_by(|left, right| {
        let key = |action: &MobilityAction| match action {
            MobilityAction::Wait => (0_u8, String::new()),
            MobilityAction::Move { arc_id } => (1_u8, arc_id.clone()),
        };
        key(left).cmp(&key(right))
    });
    options
}

fn period_capacity_valid(
    instance: &MobilityInstance,
    period: u64,
    scenario_index: usize,
    actions: &[MobilityDecision],
    adaptive: bool,
) -> bool {
    let scenario = &instance.scenarios[scenario_index];
    let Ok(history) = information_set_digest(scenario, period, adaptive) else {
        return false;
    };
    let mut arc_counts = BTreeMap::<&str, u64>::new();
    let mut corridor_counts = BTreeMap::<&str, u64>::new();
    for unit in &instance.units {
        let decision = actions.iter().find(|decision| {
            decision.period == period
                && decision.information_set_sha256 == history
                && decision.unit_id == unit.unit_id
        });
        let Some(decision) = decision else {
            continue;
        };
        if let MobilityAction::Move { arc_id } = &decision.action {
            let Some(arc) = arc_by_id(instance, arc_id) else {
                return false;
            };
            let arc_count = arc_counts.entry(arc.arc_id.as_str()).or_default();
            *arc_count = match arc_count.checked_add(1) {
                Some(value) => value,
                None => return false,
            };
            let corridor_count = corridor_counts
                .entry(arc.corridor_ref.as_str())
                .or_default();
            *corridor_count = match corridor_count.checked_add(1) {
                Some(value) => value,
                None => return false,
            };
        }
    }
    arc_counts.iter().all(|(arc_id, count)| {
        arc_by_id(instance, arc_id).is_some_and(|arc| {
            *count <= scenario_route_capacities(scenario, arc, period).0
        })
    }) && corridor_counts.iter().all(|(corridor, count)| {
        instance
            .arcs
            .iter()
            .find(|arc| arc.corridor_ref == *corridor)
            .is_some_and(|arc| *count <= scenario_route_capacities(scenario, arc, period).1)
    })
}

fn producer_search(
    instance: &MobilityInstance,
    adaptive: bool,
    budget: &mut SearchBudget,
) -> Result<(SearchResult, Option<MobilityPlan>), String> {
    let mut decisions = Vec::new();
    let states = initial_positions(instance);
    let result = search_period(instance, 0, states, adaptive, &mut decisions, budget)?;
    if result == SearchResult::Found {
        decisions.sort_by(|a, b| {
            (&a.period, &a.information_set_sha256, &a.unit_id).cmp(&(
                &b.period,
                &b.information_set_sha256,
                &b.unit_id,
            ))
        });
        Ok((
            result,
            Some(MobilityPlan {
                mode: if adaptive {
                    PolicyMode::ObservationAdaptive
                } else {
                    PolicyMode::HereAndNow
                },
                decisions,
            }),
        ))
    } else {
        Ok((result, None))
    }
}

fn search_period(
    instance: &MobilityInstance,
    period: u64,
    mut states: Vec<Vec<Position>>,
    adaptive: bool,
    decisions: &mut Vec<MobilityDecision>,
    budget: &mut SearchBudget,
) -> Result<SearchResult, String> {
    for (scenario_index, scenario_positions) in states.iter_mut().enumerate() {
        advance_arrivals(instance, scenario_positions, period)?;
        if !service_satisfied(instance, scenario_index, scenario_positions, period) {
            return Ok(SearchResult::Exhausted);
        }
    }
    if period == instance.horizon_last_period {
        return Ok(SearchResult::Found);
    }
    let groups = group_scenarios(instance, period, adaptive)?;
    search_groups(
        instance, period, &groups, 0, states, adaptive, decisions, budget,
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn search_groups(
    instance: &MobilityInstance,
    period: u64,
    groups: &[(String, Vec<usize>)],
    group_index: usize,
    states: Vec<Vec<Position>>,
    adaptive: bool,
    decisions: &mut Vec<MobilityDecision>,
    budget: &mut SearchBudget,
) -> Result<SearchResult, String> {
    if group_index == groups.len() {
        for scenario_index in 0..instance.scenarios.len() {
            if !period_capacity_valid(instance, period, scenario_index, decisions, adaptive) {
                return Ok(SearchResult::Exhausted);
            }
        }
        return search_period(instance, period + 1, states, adaptive, decisions, budget);
    }

    let (history, scenario_indices) = &groups[group_index];
    if !same_group_state(&states, scenario_indices) {
        return Ok(SearchResult::Exhausted);
    }
    let first_scenario = scenario_indices[0];
    let active_units = states[first_scenario]
        .iter()
        .enumerate()
        .filter_map(|(unit_index, position)| match position {
            Position::AtSite(site_id) => Some((unit_index, site_id.clone())),
            Position::InTransit { .. } => None,
        })
        .collect::<Vec<_>>();
    let options = active_units
        .iter()
        .map(|(_, site_id)| action_options(instance, period, scenario_indices, site_id))
        .collect::<Vec<_>>();
    let mut selected = Vec::<(usize, MobilityAction)>::new();
    enumerate_group_actions(
        instance,
        period,
        groups,
        group_index,
        states,
        adaptive,
        history,
        &active_units,
        &options,
        0,
        &mut selected,
        decisions,
        budget,
    )
}

#[allow(clippy::too_many_arguments)]
fn enumerate_group_actions(
    instance: &MobilityInstance,
    period: u64,
    groups: &[(String, Vec<usize>)],
    group_index: usize,
    states: Vec<Vec<Position>>,
    adaptive: bool,
    history: &str,
    active_units: &[(usize, String)],
    options: &[Vec<MobilityAction>],
    option_index: usize,
    selected: &mut Vec<(usize, MobilityAction)>,
    decisions: &mut Vec<MobilityDecision>,
    budget: &mut SearchBudget,
) -> Result<SearchResult, String> {
    if option_index == active_units.len() {
        if !budget.step() {
            return Ok(SearchResult::LimitExceeded);
        }
        let (_, scenario_indices) = &groups[group_index];
        let mut next_states = states;
        let old_len = decisions.len();
        for (unit_index, action) in selected.iter() {
            let unit_id = &instance.units[*unit_index].unit_id;
            decisions.push(MobilityDecision {
                period,
                information_set_sha256: history.to_string(),
                unit_id: unit_id.clone(),
                action: action.clone(),
            });
            if let MobilityAction::Move { arc_id } = action {
                let arc = arc_by_id(instance, arc_id)
                    .ok_or_else(|| "producer selected an unknown movement arc".to_string())?;
                let arrival_period = checked_arrival_period(period, arc.travel_time_periods)?;
                for scenario_index in scenario_indices {
                    next_states[*scenario_index][*unit_index] = Position::InTransit {
                        arc_id: arc_id.clone(),
                        arrival_period,
                    };
                }
            }
        }
        let result = search_groups(
            instance,
            period,
            groups,
            group_index + 1,
            next_states,
            adaptive,
            decisions,
            budget,
        )?;
        if result != SearchResult::Found {
            decisions.truncate(old_len);
        }
        return Ok(result);
    }

    let mut saw_limit = false;
    for action in &options[option_index] {
        selected.push((active_units[option_index].0, action.clone()));
        let result = enumerate_group_actions(
            instance,
            period,
            groups,
            group_index,
            states.clone(),
            adaptive,
            history,
            active_units,
            options,
            option_index + 1,
            selected,
            decisions,
            budget,
        )?;
        selected.pop();
        match result {
            SearchResult::Found => return Ok(SearchResult::Found),
            SearchResult::LimitExceeded => saw_limit = true,
            SearchResult::Exhausted => {}
        }
    }
    Ok(if saw_limit {
        SearchResult::LimitExceeded
    } else {
        SearchResult::Exhausted
    })
}

fn zero_time_information_relaxation(instance: &MobilityInstance) -> bool {
    instance.scenarios.iter().all(|scenario| {
        (0..=instance.horizon_last_period).all(|period| {
            let required = scenario
                .service_floors
                .iter()
                .filter(|floor| floor.period == period)
                .try_fold(0_u64, |total, floor| total.checked_add(floor.minimum_units));
            required.is_some_and(|total| total <= instance.units.len() as u64)
        })
    })
}

pub fn solve_mobility_policy(instance: &MobilityInstance) -> Result<MobilityReport, String> {
    validate_mobility_instance(instance)?;
    let instance_sha256 = mobility_instance_digest(instance)?;
    let relaxation_feasible = zero_time_information_relaxation(instance);

    let mut here_budget = SearchBudget::new();
    let (here_result, here_plan) = producer_search(instance, false, &mut here_budget)?;
    if here_result == SearchResult::Found {
        let plan = here_plan.ok_or_else(|| "here-and-now search omitted its plan".to_string())?;
        let plan_sha256 = mobility_plan_digest(&plan)?;
        return Ok(MobilityReport {
            schema_version: MOBILITY_REPORT_SCHEMA.to_string(),
            case_id: instance.case_id.clone(),
            instance_sha256,
            disposition: MobilityDisposition::HereAndNowFeasible,
            here_and_now_status: FeasibilityStatus::Feasible,
            mobility_status: FeasibilityStatus::Feasible,
            zero_time_information_relaxation_feasible: relaxation_feasible,
            plan_sha256: Some(plan_sha256),
            plan: Some(plan),
            producer_search_steps: here_budget.steps,
            claim_ceiling: MOBILITY_CLAIM_CEILING.to_string(),
        });
    }

    let mut adaptive_budget = SearchBudget::new();
    let (adaptive_result, adaptive_plan) = producer_search(instance, true, &mut adaptive_budget)?;
    let mut steps = here_budget.steps.saturating_add(adaptive_budget.steps);
    let (disposition, here_status, mobility_status, plan) = match adaptive_result {
        SearchResult::Found => {
            let plan = adaptive_plan
                .ok_or_else(|| "adaptive search omitted its constructive policy".to_string())?;
            if here_result == SearchResult::Exhausted {
                (
                    MobilityDisposition::MobilityFeasibleOnlyWithRecourse,
                    FeasibilityStatus::InfeasibleWithinSuppliedTree,
                    FeasibilityStatus::Feasible,
                    Some(plan),
                )
            } else {
                (
                    MobilityDisposition::MobilityFeasiblePolicyHereAndNowUnresolved,
                    FeasibilityStatus::IndeterminateSearchLimit,
                    FeasibilityStatus::Feasible,
                    Some(plan),
                )
            }
        }
        SearchResult::Exhausted => {
            if relaxation_feasible {
                (
                    MobilityDisposition::InformationRelaxationOnly,
                    FeasibilityStatus::InfeasibleWithinSuppliedTree,
                    FeasibilityStatus::InfeasibleWithinSuppliedTree,
                    None,
                )
            } else {
                (
                    MobilityDisposition::NotFeasibleWithinSuppliedTree,
                    FeasibilityStatus::InfeasibleWithinSuppliedTree,
                    FeasibilityStatus::InfeasibleWithinSuppliedTree,
                    None,
                )
            }
        }
        SearchResult::LimitExceeded => (
            MobilityDisposition::SearchLimitExceeded,
            if here_result == SearchResult::Exhausted {
                FeasibilityStatus::InfeasibleWithinSuppliedTree
            } else {
                FeasibilityStatus::IndeterminateSearchLimit
            },
            FeasibilityStatus::IndeterminateSearchLimit,
            None,
        ),
    };
    if adaptive_result != SearchResult::Found {
        steps = steps.max(here_budget.steps);
    }
    let plan_sha256 = plan.as_ref().map(mobility_plan_digest).transpose()?;
    Ok(MobilityReport {
        schema_version: MOBILITY_REPORT_SCHEMA.to_string(),
        case_id: instance.case_id.clone(),
        instance_sha256,
        disposition,
        here_and_now_status: here_status,
        mobility_status,
        zero_time_information_relaxation_feasible: relaxation_feasible,
        plan_sha256,
        plan,
        producer_search_steps: steps,
        claim_ceiling: MOBILITY_CLAIM_CEILING.to_string(),
    })
}

fn replay_policy(instance: &MobilityInstance, plan: &MobilityPlan) -> Result<(), String> {
    let adaptive = plan.mode == PolicyMode::ObservationAdaptive;
    let mut decisions = BTreeMap::<(u64, String, String), &MobilityAction>::new();
    for decision in &plan.decisions {
        if decision.period >= instance.horizon_last_period {
            return Err("policy contains a decision outside the movement horizon".to_string());
        }
        nonempty(&decision.unit_id, "policy unit_id")?;
        if !instance
            .units
            .iter()
            .any(|unit| unit.unit_id == decision.unit_id)
        {
            return Err("policy references an unknown mobile unit".to_string());
        }
        if !decision.information_set_sha256.starts_with("sha256:") {
            return Err("policy contains an invalid information-set digest".to_string());
        }
        let key = (
            decision.period,
            decision.information_set_sha256.clone(),
            decision.unit_id.clone(),
        );
        if decisions.insert(key, &decision.action).is_some() {
            return Err("policy contains duplicate decision identity".to_string());
        }
    }

    let mut expected = BTreeSet::<(u64, String, String)>::new();
    let mut states = initial_positions(instance);
    for period in 0..=instance.horizon_last_period {
        for (scenario_index, positions) in states.iter_mut().enumerate() {
            independent_advance_positions(instance, positions, period)?;
            if !independent_service_floor_check(instance, scenario_index, positions, period) {
                return Err(format!(
                    "policy misses a service floor in scenario {} at period {period}",
                    instance.scenarios[scenario_index].scenario_id
                ));
            }
        }
        if period == instance.horizon_last_period {
            break;
        }
        let mut groups = BTreeMap::<String, Vec<usize>>::new();
        for (scenario_index, scenario) in instance.scenarios.iter().enumerate() {
            let history = independent_information_set_digest(scenario, period, adaptive)?;
            groups.entry(history).or_default().push(scenario_index);
        }
        for (history, scenario_indices) in &groups {
            if !independent_states_match(&states, scenario_indices) {
                return Err(
                    "equal observable histories have different physical resource states"
                        .to_string(),
                );
            }
            let first = scenario_indices[0];
            for (unit_index, position) in states[first].iter().enumerate() {
                if matches!(position, Position::AtSite(_)) {
                    let unit_id = instance.units[unit_index].unit_id.clone();
                    let key = (period, history.clone(), unit_id);
                    expected.insert(key.clone());
                    let action = decisions
                        .get(&key)
                        .copied()
                        .ok_or_else(|| "policy omitted an available-unit decision".to_string())?;
                    if let MobilityAction::Move { arc_id } = action {
                        let arc = arc_by_id(instance, arc_id)
                            .ok_or_else(|| "policy selected an unknown movement arc".to_string())?;
                        let site_id = match &states[first][unit_index] {
                            Position::AtSite(site_id) => site_id.clone(),
                            Position::InTransit { .. } => {
                                return Err(
                                    "policy move references a unit already in transit".to_string()
                                );
                            }
                        };
                        if arc.from_site_id != site_id
                            || !arc.available_departure_periods.contains(&period)
                            || scenario_indices.iter().any(|scenario_index| {
                                independent_route_is_closed(
                                    &instance.scenarios[*scenario_index],
                                    arc_id,
                                    period,
                                )
                            })
                        {
                            return Err(format!(
                                "policy uses unavailable or closed route {arc_id} at period {period}"
                            ));
                        }
                        let arrival_period =
                            checked_arrival_period(period, arc.travel_time_periods)?;
                        for scenario_index in scenario_indices {
                            states[*scenario_index][unit_index] = Position::InTransit {
                                arc_id: arc_id.clone(),
                                arrival_period,
                            };
                        }
                    }
                }
            }
        }
        for scenario_index in 0..instance.scenarios.len() {
            if !independent_policy_capacity_valid(
                instance,
                &instance.scenarios[scenario_index],
                period,
                &plan.decisions,
                adaptive,
            ) {
                return Err(format!(
                    "policy exceeds a route or shared-corridor capacity in scenario {} at period {period}",
                    instance.scenarios[scenario_index].scenario_id
                ));
            }
        }
    }
    let submitted = decisions.keys().cloned().collect::<BTreeSet<_>>();
    if submitted != expected {
        return Err(
            "policy decision set does not exactly match required information-set actions"
                .to_string(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ScenarioPath {
    decisions: Vec<MobilityDecision>,
}

fn enumerate_scenario_paths(
    instance: &MobilityInstance,
    scenario_index: usize,
    adaptive: bool,
    budget: &mut SearchBudget,
) -> Result<(Vec<ScenarioPath>, bool), String> {
    let mut output = Vec::new();
    let positions = instance
        .units
        .iter()
        .map(|unit| Position::AtSite(unit.initial_site_id.clone()))
        .collect::<Vec<_>>();
    let mut path = Vec::new();
    let limit_hit = enumerate_one_scenario_period(
        instance,
        scenario_index,
        0,
        positions,
        adaptive,
        &mut path,
        &mut output,
        budget,
    )?;
    Ok((output, limit_hit))
}

#[allow(clippy::too_many_arguments)]
fn enumerate_one_scenario_period(
    instance: &MobilityInstance,
    scenario_index: usize,
    period: u64,
    mut positions: Vec<Position>,
    adaptive: bool,
    path: &mut Vec<MobilityDecision>,
    output: &mut Vec<ScenarioPath>,
    budget: &mut SearchBudget,
) -> Result<bool, String> {
    independent_advance_positions(instance, &mut positions, period)?;
    if !independent_service_floor_check(instance, scenario_index, &positions, period) {
        return Ok(false);
    }
    if period == instance.horizon_last_period {
        output.push(ScenarioPath {
            decisions: path.clone(),
        });
        return Ok(false);
    }
    let scenario = &instance.scenarios[scenario_index];
    let history = independent_information_set_digest(scenario, period, adaptive)?;
    let active_units = positions
        .iter()
        .enumerate()
        .filter_map(|(unit_index, position)| match position {
            Position::AtSite(site_id) => Some((unit_index, site_id.clone())),
            Position::InTransit { .. } => None,
        })
        .collect::<Vec<_>>();
    let mut options = Vec::new();
    for (_, site_id) in &active_units {
        let mut unit_options = vec![MobilityAction::Wait];
        for arc in instance.arcs.iter().filter(|arc| {
            arc.from_site_id == *site_id
                && arc.available_departure_periods.contains(&period)
                && !independent_route_is_closed(scenario, &arc.arc_id, period)
        }) {
            unit_options.push(MobilityAction::Move {
                arc_id: arc.arc_id.clone(),
            });
        }
        unit_options.sort_by(|left, right| {
            let key = |action: &MobilityAction| match action {
                MobilityAction::Wait => (0_u8, String::new()),
                MobilityAction::Move { arc_id } => (1_u8, arc_id.clone()),
            };
            key(left).cmp(&key(right))
        });
        options.push(unit_options);
    }
    let mut selected = Vec::new();
    enumerate_single_scenario_actions(
        instance,
        scenario_index,
        period,
        &history,
        &active_units,
        &options,
        0,
        &mut selected,
        positions,
        adaptive,
        path,
        output,
        budget,
    )
}

#[allow(clippy::too_many_arguments)]
fn enumerate_single_scenario_actions(
    instance: &MobilityInstance,
    scenario_index: usize,
    period: u64,
    history: &str,
    active_units: &[(usize, String)],
    options: &[Vec<MobilityAction>],
    option_index: usize,
    selected: &mut Vec<(usize, MobilityAction)>,
    positions: Vec<Position>,
    adaptive: bool,
    path: &mut Vec<MobilityDecision>,
    output: &mut Vec<ScenarioPath>,
    budget: &mut SearchBudget,
) -> Result<bool, String> {
    if option_index == active_units.len() {
        if !budget.step() {
            return Ok(true);
        }
        let mut next_positions = positions;
        let old_len = path.len();
        for (unit_index, action) in selected.iter() {
            path.push(MobilityDecision {
                period,
                information_set_sha256: history.to_string(),
                unit_id: instance.units[*unit_index].unit_id.clone(),
                action: action.clone(),
            });
            if let MobilityAction::Move { arc_id } = action {
                let arc = arc_by_id(instance, arc_id)
                    .ok_or_else(|| "independent oracle selected unknown route".to_string())?;
                let arrival_period =
                    period.checked_add(arc.travel_time_periods).ok_or_else(|| {
                        "independent trajectory oracle arrival overflows u64".to_string()
                    })?;
                next_positions[*unit_index] = Position::InTransit {
                    arc_id: arc_id.clone(),
                    arrival_period,
                };
            }
        }
        let mut arc_counts = BTreeMap::<&str, u64>::new();
        let mut corridor_counts = BTreeMap::<&str, u64>::new();
        for (_, action) in selected.iter() {
            if let MobilityAction::Move { arc_id } = action {
                let arc = arc_by_id(instance, arc_id)
                    .ok_or_else(|| "independent oracle references unknown route".to_string())?;
                *arc_counts.entry(arc.arc_id.as_str()).or_default() += 1;
                *corridor_counts
                    .entry(arc.corridor_ref.as_str())
                    .or_default() += 1;
            }
        }
        let capacity_ok = arc_counts.iter().all(|(arc_id, count)| {
            arc_by_id(instance, arc_id).is_some_and(|arc| {
                *count <= independent_scenario_route_capacities(scenario, arc, period).0
            })
        }) && corridor_counts.iter().all(|(corridor, count)| {
            instance
                .arcs
                .iter()
                .find(|arc| arc.corridor_ref == *corridor)
                .is_some_and(|arc| {
                    *count <= independent_scenario_route_capacities(scenario, arc, period).1
                })
        });
        let result = if capacity_ok {
            enumerate_one_scenario_period(
                instance,
                scenario_index,
                period + 1,
                next_positions,
                adaptive,
                path,
                output,
                budget,
            )?
        } else {
            false
        };
        path.truncate(old_len);
        return Ok(result);
    }

    for action in &options[option_index] {
        selected.push((active_units[option_index].0, action.clone()));
        let limited = enumerate_single_scenario_actions(
            instance,
            scenario_index,
            period,
            history,
            active_units,
            options,
            option_index + 1,
            selected,
            positions.clone(),
            adaptive,
            path,
            output,
            budget,
        )?;
        selected.pop();
        if limited {
            return Ok(true);
        }
    }
    Ok(false)
}

fn join_scenario_paths(
    path_sets: &[Vec<ScenarioPath>],
    scenario_order: &[usize],
    scenario_position: usize,
    combined: &mut BTreeMap<(u64, String, String), MobilityAction>,
    budget: &mut SearchBudget,
) -> OracleResult {
    if scenario_position == scenario_order.len() {
        return OracleResult::Found;
    }
    let scenario_index = scenario_order[scenario_position];
    for candidate in &path_sets[scenario_index] {
        if !budget.step() {
            return OracleResult::LimitExceeded;
        }
        let mut additions = Vec::new();
        let mut compatible = true;
        for decision in &candidate.decisions {
            let key = (
                decision.period,
                decision.information_set_sha256.clone(),
                decision.unit_id.clone(),
            );
            if let Some(existing) = combined.get(&key) {
                if existing != &decision.action {
                    compatible = false;
                    break;
                }
            } else {
                combined.insert(key.clone(), decision.action.clone());
                additions.push(key);
            }
        }
        if compatible {
            let result = join_scenario_paths(
                path_sets,
                scenario_order,
                scenario_position + 1,
                combined,
                budget,
            );
            if result == OracleResult::Found {
                return result;
            }
            if result == OracleResult::LimitExceeded {
                for key in additions {
                    combined.remove(&key);
                }
                return result;
            }
        }
        for key in additions {
            combined.remove(&key);
        }
    }
    OracleResult::NoPolicy
}

fn independent_policy_oracle(
    instance: &MobilityInstance,
    adaptive: bool,
) -> Result<OracleResult, String> {
    let mut budget = SearchBudget::new();
    let mut path_sets = Vec::new();
    for scenario_index in 0..instance.scenarios.len() {
        let (paths, limit_hit) =
            enumerate_scenario_paths(instance, scenario_index, adaptive, &mut budget)?;
        if limit_hit {
            return Ok(OracleResult::LimitExceeded);
        }
        if paths.is_empty() {
            return Ok(OracleResult::NoPolicy);
        }
        path_sets.push(paths);
    }
    let mut order = (0..path_sets.len()).collect::<Vec<_>>();
    order.sort_by_key(|index| path_sets[*index].len());
    Ok(join_scenario_paths(
        &path_sets,
        &order,
        0,
        &mut BTreeMap::new(),
        &mut budget,
    ))
}

pub fn verify_mobility_report(
    instance: &MobilityInstance,
    report: &MobilityReport,
) -> Result<(), String> {
    validate_mobility_instance_for_verification(instance)?;
    if report.schema_version != MOBILITY_REPORT_SCHEMA
        || report.case_id != instance.case_id
        || report.instance_sha256 != mobility_instance_digest(instance)?
        || report.claim_ceiling != MOBILITY_CLAIM_CEILING
    {
        return Err("mobility report identity or claim ceiling mismatch".to_string());
    }
    let expected_relaxation = independent_zero_time_relaxation(instance);
    if report.zero_time_information_relaxation_feasible != expected_relaxation {
        return Err("zero-time relaxation summary mismatch".to_string());
    }
    match (&report.plan, &report.plan_sha256) {
        (Some(plan), Some(digest)) => {
            if mobility_plan_digest(plan)? != digest.as_str() {
                return Err("mobility policy digest mismatch".to_string());
            }
            replay_policy(instance, plan)?;
        }
        (None, None) => {}
        _ => return Err("mobility policy and digest must be present together".to_string()),
    }

    match report.disposition {
        MobilityDisposition::HereAndNowFeasible => {
            let plan = report
                .plan
                .as_ref()
                .ok_or_else(|| "here-and-now result omitted its policy".to_string())?;
            if plan.mode != PolicyMode::HereAndNow
                || report.here_and_now_status != FeasibilityStatus::Feasible
                || report.mobility_status != FeasibilityStatus::Feasible
            {
                return Err("here-and-now disposition/status mismatch".to_string());
            }
            if independent_policy_oracle(instance, false)? != OracleResult::Found {
                return Err(
                    "independent oracle did not confirm here-and-now feasibility".to_string(),
                );
            }
        }
        MobilityDisposition::MobilityFeasibleOnlyWithRecourse => {
            let plan = report
                .plan
                .as_ref()
                .ok_or_else(|| "recourse result omitted its policy".to_string())?;
            if plan.mode != PolicyMode::ObservationAdaptive
                || report.here_and_now_status != FeasibilityStatus::InfeasibleWithinSuppliedTree
                || report.mobility_status != FeasibilityStatus::Feasible
            {
                return Err("recourse disposition/status mismatch".to_string());
            }
            if independent_policy_oracle(instance, false)? != OracleResult::NoPolicy
                || independent_policy_oracle(instance, true)? != OracleResult::Found
            {
                return Err(
                    "independent policy-path oracle disagrees with recourse disposition"
                        .to_string(),
                );
            }
        }
        MobilityDisposition::MobilityFeasiblePolicyHereAndNowUnresolved => {
            let plan = report
                .plan
                .as_ref()
                .ok_or_else(|| "mobility-feasible result omitted its policy".to_string())?;
            if plan.mode != PolicyMode::ObservationAdaptive
                || report.mobility_status != FeasibilityStatus::Feasible
                || report.here_and_now_status != FeasibilityStatus::IndeterminateSearchLimit
            {
                return Err("mobility-feasible unresolved disposition/status mismatch".to_string());
            }
            if independent_policy_oracle(instance, true)? != OracleResult::Found {
                return Err("independent oracle did not confirm the adaptive policy".to_string());
            }
        }
        MobilityDisposition::InformationRelaxationOnly => {
            if report.plan.is_some()
                || report.here_and_now_status != FeasibilityStatus::InfeasibleWithinSuppliedTree
                || report.mobility_status != FeasibilityStatus::InfeasibleWithinSuppliedTree
                || !expected_relaxation
                || independent_policy_oracle(instance, true)? != OracleResult::NoPolicy
            {
                return Err(
                    "information-relaxation-only result was not independently established"
                        .to_string(),
                );
            }
        }
        MobilityDisposition::NotFeasibleWithinSuppliedTree => {
            if report.plan.is_some()
                || report.here_and_now_status != FeasibilityStatus::InfeasibleWithinSuppliedTree
                || report.mobility_status != FeasibilityStatus::InfeasibleWithinSuppliedTree
                || expected_relaxation
                || independent_policy_oracle(instance, true)? != OracleResult::NoPolicy
            {
                return Err("not-feasible result was not independently established".to_string());
            }
        }
        MobilityDisposition::SearchLimitExceeded => {
            if report.plan.is_some()
                || report.mobility_status != FeasibilityStatus::IndeterminateSearchLimit
            {
                return Err("search-limit disposition/status mismatch".to_string());
            }
        }
    }
    Ok(())
}

// This verifier-side validator intentionally reconstructs basic admission
// conditions separately from validate_mobility_instance.
pub(crate) fn validate_mobility_instance_for_verification(
    instance: &MobilityInstance,
) -> Result<(), String> {
    // Re-admit the instance independently instead of delegating to the
    // producer's validator. Keep the checks explicit so a producer regression
    // cannot silently certify the same malformed input through shared logic.
    if instance.schema_version != MOBILITY_INSTANCE_SCHEMA
        || instance.case_id.trim().is_empty()
        || instance.horizon_last_period > MAX_HORIZON_LAST_PERIOD
        || !(2..=MAX_SITES).contains(&instance.sites.len())
        || instance.units.is_empty()
        || instance.units.len() > MAX_UNITS
        || instance.arcs.len() > MAX_ARCS
        || instance.scenarios.is_empty()
        || instance.scenarios.len() > MAX_SCENARIOS
    {
        return Err("independent verifier rejected mobility instance envelope".to_string());
    }

    let sites = instance
        .sites
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if sites.len() != instance.sites.len()
        || instance.sites.iter().any(|site| site.trim().is_empty())
    {
        return Err("independent verifier found invalid or duplicate site identity".to_string());
    }

    let units = instance
        .units
        .iter()
        .map(|unit| unit.unit_id.as_str())
        .collect::<BTreeSet<_>>();
    if units.len() != instance.units.len()
        || instance.units.iter().any(|unit| {
            unit.unit_id.trim().is_empty() || !sites.contains(unit.initial_site_id.as_str())
        })
    {
        return Err(
            "independent verifier rejected mobile-unit identity or initial position".to_string(),
        );
    }

    let arc_ids = instance
        .arcs
        .iter()
        .map(|arc| arc.arc_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut corridor_capacities = BTreeMap::<&str, u64>::new();
    if arc_ids.len() != instance.arcs.len() {
        return Err("independent verifier found duplicate movement-arc identity".to_string());
    }
    for arc in &instance.arcs {
        if arc.arc_id.trim().is_empty()
            || arc.corridor_ref.trim().is_empty()
            || arc.from_site_id == arc.to_site_id
            || !sites.contains(arc.from_site_id.as_str())
            || !sites.contains(arc.to_site_id.as_str())
            || arc.travel_time_periods == 0
            || arc.travel_time_periods > MAX_TRAVEL_PERIODS
            || arc.arc_capacity_per_departure == 0
            || arc.corridor_capacity_per_departure == 0
        {
            return Err("independent verifier rejected movement-arc semantics".to_string());
        }
        if corridor_capacities
            .insert(
                arc.corridor_ref.as_str(),
                arc.corridor_capacity_per_departure,
            )
            .is_some_and(|previous| previous != arc.corridor_capacity_per_departure)
        {
            return Err(
                "independent verifier found inconsistent shared-corridor capacity".to_string(),
            );
        }
        let departure_periods = arc
            .available_departure_periods
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if departure_periods.len() != arc.available_departure_periods.len()
            || departure_periods
                .iter()
                .any(|period| *period >= instance.horizon_last_period)
        {
            return Err("independent verifier rejected route departure availability".to_string());
        }
    }

    let scenarios = instance
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    if scenarios.len() != instance.scenarios.len()
        || instance
            .scenarios
            .iter()
            .any(|scenario| scenario.scenario_id.trim().is_empty())
    {
        return Err(
            "independent verifier found invalid or duplicate scenario identity".to_string(),
        );
    }
    for scenario in &instance.scenarios {
        let event_ids = scenario
            .observations
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        if event_ids.len() != scenario.observations.len()
            || scenario.observations.iter().any(|event| {
                event.event_id.trim().is_empty()
                    || event.key.trim().is_empty()
                    || event.value.trim().is_empty()
                    || event.reveal_period > instance.horizon_last_period
                    || event.valid_until_period_exclusive.is_some_and(|until| {
                        until <= event.reveal_period
                            || until > instance.horizon_last_period.saturating_add(1)
                    })
            })
        {
            return Err("independent verifier rejected observation event inventory".to_string());
        }

        let closure_keys = scenario
            .route_closures
            .iter()
            .map(|closure| (closure.arc_id.as_str(), closure.departure_period))
            .collect::<BTreeSet<_>>();
        if closure_keys.len() != scenario.route_closures.len()
            || scenario.route_closures.iter().any(|closure| {
                !arc_ids.contains(closure.arc_id.as_str())
                    || closure.departure_period >= instance.horizon_last_period
                    || closure.observed_at_period > instance.horizon_last_period
            })
        {
            return Err("independent verifier rejected route-closure inventory".to_string());
        }

        let floor_keys = scenario
            .service_floors
            .iter()
            .map(|floor| (floor.site_id.as_str(), floor.period))
            .collect::<BTreeSet<_>>();
        if floor_keys.len() != scenario.service_floors.len()
            || scenario.service_floors.iter().any(|floor| {
                !sites.contains(floor.site_id.as_str())
                    || floor.period > instance.horizon_last_period
                    || floor.minimum_units == 0
                    || floor.minimum_units > instance.units.len() as u64
            })
        {
            return Err("independent verifier rejected service-floor inventory".to_string());
        }

        // Independent reconstruction of the complete scenario-specific
        // departure-capacity table. Zero is allowed as an explicit closure.
        if !scenario.route_capacity_overrides.is_empty() {
            let expected = instance.arcs.len() * instance.horizon_last_period as usize;
            if scenario.route_capacity_overrides.len() != expected {
                return Err(
                    "independent verifier found incomplete route-capacity override coverage"
                        .to_string(),
                );
            }
            let arc_by_id = instance
                .arcs
                .iter()
                .map(|arc| (arc.arc_id.as_str(), arc))
                .collect::<BTreeMap<_, _>>();
            let mut override_keys = BTreeSet::new();
            let mut physical_caps = BTreeMap::<(&str, u64), u64>::new();
            for item in &scenario.route_capacity_overrides {
                let Some(arc) = arc_by_id.get(item.arc_id.as_str()).copied() else {
                    return Err("independent verifier found override for unknown route".to_string());
                };
                if item.arc_capacity > arc.arc_capacity_per_departure
                    || item.corridor_capacity > arc.corridor_capacity_per_departure
                {
                    return Err(
                        "independent verifier found route capacity above its structural ceiling"
                            .to_string(),
                    );
                }
                let corridor = arc.corridor_ref.as_str();
                if item.departure_period >= instance.horizon_last_period
                    || !override_keys.insert((item.arc_id.as_str(), item.departure_period))
                {
                    return Err(
                        "independent verifier found duplicate or out-of-range route override"
                            .to_string(),
                    );
                }
                if let Some(previous) =
                    physical_caps.insert((corridor, item.departure_period), item.corridor_capacity)
                {
                    if previous != item.corridor_capacity {
                        return Err(
                            "independent verifier found conflicting shared-corridor capacities"
                                .to_string(),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arc(
        id: &str,
        from: &str,
        to: &str,
        travel: u64,
        corridor: &str,
        corridor_capacity: u64,
        available: &[u64],
    ) -> MovementArc {
        MovementArc {
            arc_id: id.to_string(),
            from_site_id: from.to_string(),
            to_site_id: to.to_string(),
            travel_time_periods: travel,
            arc_capacity_per_departure: 4,
            corridor_ref: corridor.to_string(),
            corridor_capacity_per_departure: corridor_capacity,
            available_departure_periods: available.to_vec(),
        }
    }

    fn unit(id: &str, site: &str) -> MobilityUnit {
        MobilityUnit {
            unit_id: id.to_string(),
            initial_site_id: site.to_string(),
        }
    }

    fn floor(site: &str, period: u64, units: u64) -> ServiceFloor {
        ServiceFloor {
            site_id: site.to_string(),
            period,
            minimum_units: units,
        }
    }

    fn scenario(id: &str, service_floors: Vec<ServiceFloor>) -> MobilityScenario {
        MobilityScenario {
            scenario_id: id.to_string(),
            observations: Vec::new(),
            route_closures: Vec::new(),
            service_floors,
            route_capacity_overrides: Vec::new(),
        }
    }

    fn instance(
        horizon: u64,
        sites: &[&str],
        units: Vec<MobilityUnit>,
        arcs: Vec<MovementArc>,
        scenarios: Vec<MobilityScenario>,
    ) -> MobilityInstance {
        MobilityInstance {
            schema_version: MOBILITY_INSTANCE_SCHEMA.to_string(),
            case_id: "mobility-known-answer".to_string(),
            horizon_last_period: horizon,
            sites: sites.iter().map(|site| (*site).to_string()).collect(),
            units,
            arcs,
            scenarios,
        }
    }

    #[test]
    fn dynamic_route_capacity_cannot_exceed_static_structural_ceiling() {
        let mut model = instance(
            1,
            &["depot", "site-a"],
            vec![unit("pump-1", "depot")],
            vec![arc("depot-a", "depot", "site-a", 1, "road", 1, &[0])],
            vec![scenario("world", vec![floor("site-a", 1, 1)])],
        );
        model.scenarios[0].route_capacity_overrides = vec![RouteCapacityOverride {
            arc_id: "depot-a".to_string(),
            departure_period: 0,
            arc_capacity: 5,
            corridor_capacity: 1,
        }];
        assert!(validate_mobility_instance(&model).is_err());
        assert!(validate_mobility_instance_for_verification(&model).is_err());
    }

    #[test]
    fn scenario_capacity_overrides_are_bound_into_canonical_instance_identity() {
        let mut model = instance(
            1,
            &["depot", "site-a"],
            vec![unit("pump-1", "depot")],
            vec![arc("depot-a", "depot", "site-a", 1, "road", 1, &[0])],
            vec![scenario("world", vec![floor("site-a", 1, 1)])],
        );
        let static_digest = mobility_instance_digest(&model).unwrap();
        model.scenarios[0].route_capacity_overrides = vec![RouteCapacityOverride {
            arc_id: "depot-a".to_string(),
            departure_period: 0,
            arc_capacity: 1,
            corridor_capacity: 1,
        }];
        let dynamic_digest = mobility_instance_digest(&model).unwrap();
        assert_ne!(static_digest, dynamic_digest);
    }

    #[test]
    fn duplicate_canonical_evidence_records_do_not_split_information_sets() {
        let mut candidate = scenario("world", vec![floor("site-a", 1, 1)]);
        candidate.observations.push(ObservationEvent {
            reveal_period: 0,
            valid_until_period_exclusive: Some(3),
            event_id: "record-1".to_string(),
            key: "capacity:route:road-7".to_string(),
            value: "sha256:canonical-fact".to_string(),
        });
        let one_record = information_set_digest(&candidate, 1, true).unwrap();
        candidate.observations.push(ObservationEvent {
            reveal_period: 0,
            valid_until_period_exclusive: Some(3),
            event_id: "different-record-id".to_string(),
            key: "capacity:route:road-7".to_string(),
            value: "sha256:canonical-fact".to_string(),
        });
        let two_records = information_set_digest(&candidate, 1, true).unwrap();
        assert_eq!(one_record, two_records);
        assert_eq!(
            two_records,
            independent_information_set_digest(&candidate, 1, true).unwrap()
        );
    }

    #[test]
    fn expiring_observations_are_visible_only_inside_their_half_open_window() {
        let mut candidate = scenario("world", vec![floor("site-a", 1, 1)]);
        let no_fact_at_zero = information_set_digest(&candidate, 0, true).unwrap();
        candidate.observations.push(ObservationEvent {
            reveal_period: 1,
            valid_until_period_exclusive: Some(3),
            event_id: "capacity-proof".to_string(),
            key: "capacity:route:road-7".to_string(),
            value: "sha256:capacity-fact".to_string(),
        });
        validate_mobility_instance(&instance(
            3,
            &["depot", "site-a"],
            vec![unit("unit-1", "depot")],
            vec![arc("road-7", "depot", "site-a", 1, "road-7", 1, &[0, 1, 2])],
            vec![candidate.clone()],
        ))
        .unwrap();

        assert_eq!(
            information_set_digest(&candidate, 0, true).unwrap(),
            no_fact_at_zero
        );
        let visible = information_set_digest(&candidate, 1, true).unwrap();
        assert_ne!(visible, no_fact_at_zero);
        assert_eq!(
            information_set_digest(&candidate, 2, true).unwrap(),
            visible
        );
        assert_eq!(
            information_set_digest(&candidate, 3, true).unwrap(),
            no_fact_at_zero
        );
        for period in 0..=3 {
            assert_eq!(
                information_set_digest(&candidate, period, true).unwrap(),
                independent_information_set_digest(&candidate, period, true).unwrap()
            );
        }
        let mut invalid = candidate;
        invalid.observations[0].valid_until_period_exclusive = Some(1);
        assert!(
            validate_mobility_instance(&instance(
                3,
                &["depot", "site-a"],
                vec![unit("unit-1", "depot")],
                vec![arc("road-7", "depot", "site-a", 1, "road-7", 1, &[0, 1, 2])],
                vec![invalid],
            ))
            .is_err()
        );
    }

    #[test]
    fn zero_time_relaxation_does_not_prove_a_late_arrival_feasible() {
        let model = instance(
            1,
            &["depot", "water-site"],
            vec![unit("pump-1", "depot")],
            vec![arc(
                "depot-water",
                "depot",
                "water-site",
                2,
                "road-1",
                1,
                &[0],
            )],
            vec![scenario("world", vec![floor("water-site", 1, 1)])],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        assert!(report.zero_time_information_relaxation_feasible);
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn arrival_exactly_on_service_boundary_is_available() {
        let model = instance(
            2,
            &["depot", "site-a"],
            vec![unit("pump-1", "depot")],
            vec![arc("depot-a", "depot", "site-a", 2, "road", 1, &[0])],
            vec![scenario("world", vec![floor("site-a", 2, 1)])],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(report.disposition, MobilityDisposition::HereAndNowFeasible);
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn reverse_route_is_not_inferred_from_a_declared_forward_arc() {
        let model = instance(
            1,
            &["depot", "site-a"],
            vec![unit("pump-1", "site-a")],
            vec![arc("depot-to-a", "depot", "site-a", 1, "road", 1, &[0])],
            vec![scenario("world", vec![floor("depot", 1, 1)])],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert!(report.zero_time_information_relaxation_feasible);
        assert_eq!(
            report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn one_common_policy_is_feasible_when_all_scenarios_need_the_same_site() {
        let mut alpha = scenario("alpha", vec![floor("site-a", 1, 1)]);
        alpha.observations.push(ObservationEvent {
            reveal_period: 1,
            event_id: "alpha-demand".to_string(),
            key: "service-site".to_string(),
            value: "site-a".to_string(),
        });
        let mut beta = scenario("beta", vec![floor("site-a", 1, 1)]);
        beta.observations.push(ObservationEvent {
            reveal_period: 1,
            event_id: "beta-demand".to_string(),
            key: "service-site".to_string(),
            value: "site-a".to_string(),
        });
        let model = instance(
            1,
            &["hub", "site-a", "site-b"],
            vec![unit("mobile-pump", "hub")],
            vec![
                arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
            ],
            vec![alpha, beta],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(report.disposition, MobilityDisposition::HereAndNowFeasible);
        assert_eq!(report.plan.as_ref().unwrap().mode, PolicyMode::HereAndNow);
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn adaptive_policy_can_diverge_after_the_scenario_is_observed() {
        let mut alpha = scenario("alpha", vec![floor("site-a", 1, 1)]);
        alpha.observations.push(ObservationEvent {
            reveal_period: 0,
            event_id: "demand-alpha".to_string(),
            key: "affected-site".to_string(),
            value: "site-a".to_string(),
        });
        let mut beta = scenario("beta", vec![floor("site-b", 1, 1)]);
        beta.observations.push(ObservationEvent {
            reveal_period: 0,
            event_id: "demand-beta".to_string(),
            key: "affected-site".to_string(),
            value: "site-b".to_string(),
        });
        let model = instance(
            1,
            &["hub", "site-a", "site-b"],
            vec![unit("mobile-pump", "hub")],
            vec![
                arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
            ],
            vec![alpha, beta],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            MobilityDisposition::MobilityFeasibleOnlyWithRecourse
        );
        let plan = report.plan.as_ref().unwrap();
        assert_eq!(plan.mode, PolicyMode::ObservationAdaptive);
        assert_eq!(plan.decisions.len(), 2);
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn hidden_future_demand_cannot_change_the_departure_action() {
        let mut alpha = scenario("alpha", vec![floor("site-a", 1, 1)]);
        alpha.observations.push(ObservationEvent {
            reveal_period: 1,
            event_id: "demand-alpha".to_string(),
            key: "affected-site".to_string(),
            value: "site-a".to_string(),
        });
        let mut beta = scenario("beta", vec![floor("site-b", 1, 1)]);
        beta.observations.push(ObservationEvent {
            reveal_period: 1,
            event_id: "demand-beta".to_string(),
            key: "affected-site".to_string(),
            value: "site-b".to_string(),
        });
        let model = instance(
            1,
            &["hub", "site-a", "site-b"],
            vec![unit("mobile-pump", "hub")],
            vec![
                arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
            ],
            vec![alpha, beta],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn transit_resource_is_not_available_at_either_endpoint() {
        let model = instance(
            1,
            &["depot", "site-a"],
            vec![unit("pump-1", "depot"), unit("pump-2", "depot")],
            vec![arc("depot-a", "depot", "site-a", 2, "road", 2, &[0])],
            vec![scenario(
                "world",
                vec![floor("depot", 1, 1), floor("site-a", 1, 1)],
            )],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert!(report.zero_time_information_relaxation_feasible);
        assert_eq!(
            report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn total_service_floors_above_the_unit_count_fail_even_under_relaxation() {
        let model = instance(
            1,
            &["site-a", "site-b"],
            vec![unit("pump-1", "site-a")],
            Vec::new(),
            vec![scenario(
                "world",
                vec![floor("site-a", 1, 1), floor("site-b", 1, 1)],
            )],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert!(!report.zero_time_information_relaxation_feasible);
        assert_eq!(
            report.disposition,
            MobilityDisposition::NotFeasibleWithinSuppliedTree
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn parallel_route_aliases_share_one_physical_corridor_capacity() {
        let model = instance(
            1,
            &["depot", "site-a"],
            vec![unit("pump-1", "depot"), unit("pump-2", "depot")],
            vec![
                arc(
                    "logical-route-1",
                    "depot",
                    "site-a",
                    1,
                    "physical-corridor",
                    1,
                    &[0],
                ),
                arc(
                    "logical-route-2",
                    "depot",
                    "site-a",
                    1,
                    "physical-corridor",
                    1,
                    &[0],
                ),
            ],
            vec![scenario("world", vec![floor("site-a", 1, 2)])],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn observed_route_closure_can_change_a_later_policy_action() {
        let mut alpha = scenario("alpha", vec![floor("site-a", 1, 1)]);
        alpha.observations.push(ObservationEvent {
            reveal_period: 0,
            event_id: "site-a-open".to_string(),
            key: "route-choice".to_string(),
            value: "a-open".to_string(),
        });
        let mut beta = scenario("beta", vec![floor("site-b", 1, 1)]);
        beta.observations.push(ObservationEvent {
            reveal_period: 0,
            event_id: "site-a-closed".to_string(),
            key: "route-choice".to_string(),
            value: "a-closed".to_string(),
        });
        beta.route_closures.push(RouteClosure {
            arc_id: "hub-a".to_string(),
            departure_period: 0,
            observed_at_period: 0,
        });
        let model = instance(
            1,
            &["hub", "site-a", "site-b"],
            vec![unit("mobile-pump", "hub")],
            vec![
                arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
            ],
            vec![alpha, beta],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            MobilityDisposition::MobilityFeasibleOnlyWithRecourse
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn a_late_reported_closure_cannot_be_ignored_at_departure() {
        let open = scenario("open", vec![floor("site-a", 1, 1)]);
        let mut closed = scenario("closed", vec![floor("site-a", 1, 1)]);
        closed.route_closures.push(RouteClosure {
            arc_id: "hub-a".to_string(),
            departure_period: 0,
            observed_at_period: 1,
        });
        let model = instance(
            1,
            &["hub", "site-a"],
            vec![unit("mobile-pump", "hub")],
            vec![arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0])],
            vec![open, closed],
        );
        let report = solve_mobility_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&model, &report).unwrap();
    }

    #[test]
    fn forged_history_digest_and_route_action_are_rejected() {
        let mut alpha = scenario("alpha", vec![floor("site-a", 1, 1)]);
        alpha.observations.push(ObservationEvent {
            reveal_period: 0,
            event_id: "demand-alpha".to_string(),
            key: "affected-site".to_string(),
            value: "site-a".to_string(),
        });
        let mut beta = scenario("beta", vec![floor("site-b", 1, 1)]);
        beta.observations.push(ObservationEvent {
            reveal_period: 0,
            event_id: "demand-beta".to_string(),
            key: "affected-site".to_string(),
            value: "site-b".to_string(),
        });
        let model = instance(
            1,
            &["hub", "site-a", "site-b"],
            vec![unit("mobile-pump", "hub")],
            vec![
                arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
            ],
            vec![alpha, beta],
        );
        let mut report = solve_mobility_policy(&model).unwrap();
        let plan = report.plan.as_mut().unwrap();
        plan.decisions[0].information_set_sha256 = no_information_digest();
        report.plan_sha256 = Some(mobility_plan_digest(plan).unwrap());
        assert!(
            verify_mobility_report(&model, &report)
                .unwrap_err()
                .contains("policy omitted")
        );
    }

    #[test]
    fn model_digest_is_independent_of_input_array_order() {
        let model = instance(
            1,
            &["hub", "site-a", "site-b"],
            vec![unit("pump", "hub")],
            vec![
                arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
            ],
            vec![
                scenario("alpha", vec![floor("site-a", 1, 1)]),
                scenario("beta", vec![floor("site-b", 1, 1)]),
            ],
        );
        let digest = mobility_instance_digest(&model).unwrap();
        let mut reordered = model.clone();
        reordered.sites.reverse();
        reordered.units.reverse();
        reordered.arcs.reverse();
        reordered.scenarios.reverse();
        assert_eq!(digest, mobility_instance_digest(&reordered).unwrap());
    }

    #[test]
    fn seeded_tiny_policy_corpus_matches_independent_path_oracle() {
        let mut rng = 0x0A11_CE55_5AFB_2026_u64;
        for seed in 0..250_u64 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let scenario_count = 1 + ((rng >> 32) % 4) as usize;
            let mut scenarios = Vec::with_capacity(scenario_count);
            for index in 0..scenario_count {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let site = if (rng >> 32) % 2 == 0 {
                    "site-a"
                } else {
                    "site-b"
                };
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let reveal_period = (rng >> 32) % 2;
                let mut candidate = scenario(&format!("world-{index}"), vec![floor(site, 1, 1)]);
                candidate.observations.push(ObservationEvent {
                    reveal_period,
                    event_id: format!("observation-{index}"),
                    key: "service-site".to_string(),
                    value: site.to_string(),
                });
                scenarios.push(candidate);
            }
            let model = instance(
                1,
                &["hub", "site-a", "site-b"],
                vec![unit("mobile-pump", "hub")],
                vec![
                    arc("hub-a", "hub", "site-a", 1, "corridor-a", 1, &[0]),
                    arc("hub-b", "hub", "site-b", 1, "corridor-b", 1, &[0]),
                ],
                scenarios,
            );
            let report = solve_mobility_policy(&model).unwrap_or_else(|error| {
                panic!("seed {seed}: producer rejected generated model: {error}")
            });
            verify_mobility_report(&model, &report).unwrap_or_else(|error| {
                panic!("seed {seed}: independent policy oracle disagreed: {error}")
            });
        }
    }

    #[test]
    fn seeded_two_leg_policies_preserve_nonanticipativity_across_transit() {
        let mut rng = 0x51A7_EB00_7876_2026_u64;
        for seed in 0..100_u64 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let scenario_count = 1 + ((rng >> 32) % 3) as usize;
            let mut scenarios = Vec::with_capacity(scenario_count);
            let mut targets = Vec::with_capacity(scenario_count);
            for index in 0..scenario_count {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let target = if (rng >> 32) % 2 == 0 {
                    "site-a"
                } else {
                    "site-b"
                };
                targets.push(target.to_string());
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let reveal_period = (rng >> 32) % 2;
                let mut candidate = scenario(&format!("world-{index}"), vec![floor(target, 2, 1)]);
                candidate.observations.push(ObservationEvent {
                    reveal_period,
                    event_id: format!("target-{index}"),
                    key: "service-target".to_string(),
                    value: target.to_string(),
                });
                scenarios.push(candidate);
            }
            let same_target = targets.iter().all(|target| target == &targets[0]);
            let model = instance(
                2,
                &["hub", "site-a", "site-b"],
                vec![unit("mobile-unit", "hub")],
                vec![
                    arc("hub-a", "hub", "site-a", 1, "corridor-hub-a", 1, &[0, 1]),
                    arc("a-b", "site-a", "site-b", 1, "corridor-a-b", 1, &[1]),
                ],
                scenarios,
            );
            let report = solve_mobility_policy(&model).unwrap_or_else(|error| {
                panic!("seed {seed}: two-leg producer rejected model: {error}")
            });
            let expected = if same_target {
                MobilityDisposition::HereAndNowFeasible
            } else {
                MobilityDisposition::MobilityFeasibleOnlyWithRecourse
            };
            assert_eq!(report.disposition, expected, "seed {seed}");
            verify_mobility_report(&model, &report).unwrap_or_else(|error| {
                panic!("seed {seed}: independent multi-period oracle disagreed: {error}")
            });
        }
    }

    #[test]
    fn zero_travel_and_zero_duration_arcs_are_rejected() {
        let mut model = instance(
            1,
            &["depot", "site-a"],
            vec![unit("pump", "depot")],
            vec![arc("route", "depot", "site-a", 1, "corridor", 1, &[0])],
            vec![scenario("world", vec![floor("site-a", 1, 1)])],
        );
        model.arcs[0].travel_time_periods = 0;
        assert!(validate_mobility_instance(&model).is_err());
    }

    #[test]
    fn checked_arrival_arithmetic_rejects_u64_overflow() {
        assert!(checked_arrival_period(u64::MAX, 1).is_err());
        assert_eq!(checked_arrival_period(12, 3).unwrap(), 15);
    }

    #[test]
    fn published_schema_files_are_valid_json_documents() {
        let input_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/mobility-instance-v0.1.schema.json"
        ))
        .unwrap();
        let report_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/mobility-report-v0.1.schema.json"))
                .unwrap();
        assert_eq!(
            input_schema["properties"]["schema_version"]["const"],
            MOBILITY_INSTANCE_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["schema_version"]["const"],
            MOBILITY_REPORT_SCHEMA
        );
        assert_eq!(input_schema["additionalProperties"], false);
        assert_eq!(report_schema["additionalProperties"], false);
    }
}

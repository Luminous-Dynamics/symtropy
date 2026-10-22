use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

const FIXTURE_BYTES: &[u8] = include_bytes!("../fixtures/orbital_foundry_v0.json");
const SCENARIO_IDS: [&str; 5] = [
    "ROBOT_JOINT_FAILURE",
    "INTERFACE_MISMATCH",
    "POWER_DOMAIN_FAILURE",
    "DELAYED_RESUPPLY",
    "COMMON_CAUSE_FAILURE",
];

type Check<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FaultKind {
    RobotJointFailure,
    InterfaceMismatch,
    PowerDomainFailure,
    ResupplyDelay,
    CommonCausePowerFailure,
}

impl FaultKind {
    fn parse(value: &str, at: &str) -> Check<Self> {
        match value {
            "robot_joint_failure" => Ok(Self::RobotJointFailure),
            "interface_mismatch" => Ok(Self::InterfaceMismatch),
            "power_domain_failure" => Ok(Self::PowerDomainFailure),
            "resupply_delay" => Ok(Self::ResupplyDelay),
            "common_cause_power_failure" => Ok(Self::CommonCausePowerFailure),
            _ => Err(format!("{at}: unsupported fault event kind {value:?}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::RobotJointFailure => "robot_joint_failure",
            Self::InterfaceMismatch => "interface_mismatch",
            Self::PowerDomainFailure => "power_domain_failure",
            Self::ResupplyDelay => "resupply_delay",
            Self::CommonCausePowerFailure => "common_cause_power_failure",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FaultEvent {
    at_tick: u32,
    kind: FaultKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScenarioContract {
    id: String,
    disturbance: String,
    claim_ceiling: String,
    events: Vec<FaultEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Contract {
    mass_unit: String,
    time_unit: String,
    max_worst_case_service_loss_ticks: u32,
    end_of_life_plan_required: bool,
    scenarios: Vec<ScenarioContract>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DesignSpec {
    standardized_interfaces: Option<bool>,
    isolated_power_domains: Option<bool>,
    spare_pack: Option<bool>,
    end_of_life_plan: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Design {
    standardized_interfaces: bool,
    isolated_power_domains: bool,
    spare_pack: bool,
    end_of_life_plan: bool,
}

impl DesignSpec {
    fn resolve(self) -> Check<Design> {
        Ok(Design {
            standardized_interfaces: self
                .standardized_interfaces
                .ok_or_else(|| "standardized_interfaces is unknown".to_string())?,
            isolated_power_domains: self
                .isolated_power_domains
                .ok_or_else(|| "isolated_power_domains is unknown".to_string())?,
            spare_pack: self
                .spare_pack
                .ok_or_else(|| "spare_pack policy is unknown".to_string())?,
            end_of_life_plan: self
                .end_of_life_plan
                .ok_or_else(|| "end_of_life_plan status is unknown".to_string())?,
        })
    }
}

impl Design {
    fn id(self) -> String {
        format!(
            "std={}|power_isolated={}|spares={}|eol_plan={}",
            u8::from(self.standardized_interfaces),
            u8::from(self.isolated_power_domains),
            u8::from(self.spare_pack),
            u8::from(self.end_of_life_plan)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ServiceKind {
    AssemblyRobot,
    AssemblyInterface,
    PowerBus,
    Supply,
}

impl ServiceKind {
    fn for_fault(kind: FaultKind) -> Self {
        match kind {
            FaultKind::RobotJointFailure => Self::AssemblyRobot,
            FaultKind::InterfaceMismatch => Self::AssemblyInterface,
            FaultKind::PowerDomainFailure | FaultKind::CommonCausePowerFailure => Self::PowerBus,
            FaultKind::ResupplyDelay => Self::Supply,
        }
    }

    fn slot(self) -> usize {
        match self {
            Self::AssemblyRobot => 0,
            Self::AssemblyInterface => 1,
            Self::PowerBus => 2,
            Self::Supply => 3,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::AssemblyRobot => "assembly_robot",
            Self::AssemblyInterface => "assembly_interface",
            Self::PowerBus => "power_bus",
            Self::Supply => "supply",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventOutcome {
    at_tick: u32,
    kind: FaultKind,
    service: ServiceKind,
    repair_start_tick: u32,
    repair_end_tick: u32,
    queue_delay_ticks: u32,
    spare_units_before: u32,
    spare_units_after: u32,
    recovery_ticks: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScenarioSimulation {
    service_loss_ticks: u32,
    recovery_elapsed_ticks: u32,
    events: Vec<EventOutcome>,
    spare_units_remaining: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Metrics {
    launch_mass_smu: u32,
    assembly_ticks: u32,
    interface_families: u32,
    scenario_loss_ticks: [u32; 5],
    scenario_recovery_elapsed_ticks: [u32; 5],
    sum_service_loss_ticks_across_scenarios: u32,
    worst_case_service_loss_ticks: u32,
    worst_case_recovery_elapsed_ticks: u32,
    end_of_life_plan_declared: bool,
    hard_gates_pass: bool,
}

#[derive(Clone, Debug)]
struct ResultRow {
    design: Design,
    metrics: Metrics,
}

fn object_at<'a>(value: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected a JSON object"))
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = object_at(value, at)?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<&str> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<&str> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object_at(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn string_field(value: &Value, key: &str, at: &str) -> Check<String> {
    field(value, key, at)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn u32_field(value: &Value, key: &str, at: &str) -> Check<u32> {
    let raw = field(value, key, at)?
        .as_u64()
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))?;
    u32::try_from(raw).map_err(|_| format!("{at}.{key}: integer exceeds u32"))
}

fn bool_field(value: &Value, key: &str, at: &str) -> Check<bool> {
    field(value, key, at)?
        .as_bool()
        .ok_or_else(|| format!("{at}.{key}: expected boolean"))
}

fn expected_event_kinds(scenario_id: &str) -> Option<&'static [&'static str]> {
    match scenario_id {
        "ROBOT_JOINT_FAILURE" => Some(&["robot_joint_failure"]),
        "INTERFACE_MISMATCH" => Some(&["interface_mismatch"]),
        "POWER_DOMAIN_FAILURE" => Some(&["power_domain_failure"]),
        "DELAYED_RESUPPLY" => Some(&["resupply_delay"]),
        "COMMON_CAUSE_FAILURE" => Some(&[
            "robot_joint_failure",
            "power_domain_failure",
            "common_cause_power_failure",
            "resupply_delay",
        ]),
        _ => None,
    }
}

fn parse_contract(bytes: &[u8]) -> Check<Contract> {
    let root: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("fixture JSON: {error}"))?;
    exact_keys(
        &root,
        &[
            "schema_version",
            "evidence_class",
            "model",
            "units",
            "hard_gates",
            "scenarios",
        ],
        "root",
    )?;

    if string_field(&root, "schema_version", "root")? != "0.2.0"
        || string_field(&root, "evidence_class", "root")? != "synthetic"
        || string_field(&root, "model", "root")?
            != "event_driven_discrete_lifecycle_contract_not_physics"
    {
        return Err("root: unsupported schema, evidence class, or model claim".to_string());
    }

    let units = field(&root, "units", "root")?;
    exact_keys(units, &["mass", "time"], "units")?;
    let mass_unit = string_field(units, "mass", "units")?;
    let time_unit = string_field(units, "time", "units")?;
    if mass_unit != "synthetic_mass_units" || time_unit != "synthetic_ticks" {
        return Err("units: this reference model requires declared synthetic units".to_string());
    }

    let gates = field(&root, "hard_gates", "root")?;
    exact_keys(
        gates,
        &[
            "max_worst_case_service_loss_ticks",
            "end_of_life_plan_required",
        ],
        "hard_gates",
    )?;
    let max_worst_case_service_loss_ticks =
        u32_field(gates, "max_worst_case_service_loss_ticks", "hard_gates")?;
    let end_of_life_plan_required = bool_field(gates, "end_of_life_plan_required", "hard_gates")?;
    if max_worst_case_service_loss_ticks != 12 || !end_of_life_plan_required {
        return Err("hard_gates: unsupported or weakened synthetic contract".to_string());
    }

    let scenario_values = field(&root, "scenarios", "root")?
        .as_array()
        .ok_or_else(|| "scenarios: expected array".to_string())?;
    if scenario_values.len() != SCENARIO_IDS.len() {
        return Err(format!(
            "scenarios: expected {} frozen scenarios, got {}",
            SCENARIO_IDS.len(),
            scenario_values.len()
        ));
    }

    let mut scenarios = Vec::with_capacity(scenario_values.len());
    for (index, scenario) in scenario_values.iter().enumerate() {
        let at = format!("scenarios[{index}]");
        exact_keys(
            scenario,
            &["id", "disturbance", "claim_ceiling", "events"],
            &at,
        )?;
        let id = string_field(scenario, "id", &at)?;
        let disturbance = string_field(scenario, "disturbance", &at)?;
        let claim_ceiling = string_field(scenario, "claim_ceiling", &at)?;
        if id != SCENARIO_IDS[index] || !claim_ceiling.starts_with("synthetic ") {
            return Err(format!(
                "{at}: scenario order/identity or synthetic claim ceiling changed"
            ));
        }

        let expected_kinds = expected_event_kinds(&id)
            .ok_or_else(|| format!("{at}: unsupported scenario identity {id:?}"))?;
        let event_values = field(scenario, "events", &at)?
            .as_array()
            .ok_or_else(|| format!("{at}.events: expected array"))?;
        if event_values.len() != expected_kinds.len() {
            return Err(format!(
                "{at}.events: expected {} events, got {}",
                expected_kinds.len(),
                event_values.len()
            ));
        }

        let mut events = Vec::with_capacity(event_values.len());
        let mut previous_tick = None;
        for (event_index, event_value) in event_values.iter().enumerate() {
            let event_at = format!("{at}.events[{event_index}]");
            exact_keys(event_value, &["at_tick", "kind"], &event_at)?;
            let at_tick = u32_field(event_value, "at_tick", &event_at)?;
            if previous_tick.is_some_and(|previous| at_tick < previous) {
                return Err(format!(
                    "{event_at}: events must be ordered by nondecreasing tick"
                ));
            }
            previous_tick = Some(at_tick);

            let kind_text = string_field(event_value, "kind", &event_at)?;
            if kind_text != expected_kinds[event_index] {
                return Err(format!(
                    "{event_at}: event kind/order differs from the frozen scenario contract"
                ));
            }
            events.push(FaultEvent {
                at_tick,
                kind: FaultKind::parse(&kind_text, &event_at)?,
            });
        }

        scenarios.push(ScenarioContract {
            id,
            disturbance,
            claim_ceiling,
            events,
        });
    }

    Ok(Contract {
        mass_unit,
        time_unit,
        max_worst_case_service_loss_ticks,
        end_of_life_plan_required,
        scenarios,
    })
}

fn add(a: u32, b: u32, at: &str) -> Check<u32> {
    a.checked_add(b)
        .ok_or_else(|| format!("{at}: integer overflow"))
}

fn sum_checked(values: &[u32], at: &str) -> Check<u32> {
    values
        .iter()
        .try_fold(0_u32, |sum, value| add(sum, *value, at))
}

fn transition(design: Design, spare_units_remaining: &mut u32, kind: FaultKind) -> u32 {
    match kind {
        FaultKind::RobotJointFailure => {
            if *spare_units_remaining > 0 {
                *spare_units_remaining -= 1;
                2
            } else {
                6
            }
        }
        FaultKind::InterfaceMismatch => {
            if design.standardized_interfaces {
                1
            } else {
                7
            }
        }
        FaultKind::PowerDomainFailure => {
            if design.isolated_power_domains {
                1
            } else {
                8
            }
        }
        FaultKind::ResupplyDelay => {
            if *spare_units_remaining > 0 {
                *spare_units_remaining -= 1;
                2
            } else {
                6
            }
        }
        FaultKind::CommonCausePowerFailure => {
            if design.isolated_power_domains {
                3
            } else {
                8
            }
        }
    }
}

fn simulate_scenario(design: Design, scenario: &ScenarioContract) -> Check<ScenarioSimulation> {
    let mut spare_units_remaining = if design.spare_pack { 2 } else { 0 };
    let mut service_available_at_tick = [0_u32; 4];
    let mut service_loss_ticks = 0_u32;
    let mut outcomes = Vec::with_capacity(scenario.events.len());
    let first_event_tick = scenario
        .events
        .first()
        .ok_or_else(|| format!("{}: empty event sequence", scenario.id))?
        .at_tick;

    for event in &scenario.events {
        let service = ServiceKind::for_fault(event.kind);
        let service_slot = service.slot();
        let repair_start_tick = event.at_tick.max(service_available_at_tick[service_slot]);
        let queue_delay_ticks = repair_start_tick - event.at_tick;
        let spare_units_before = spare_units_remaining;
        let recovery_ticks = transition(design, &mut spare_units_remaining, event.kind);
        let repair_end_tick = add(repair_start_tick, recovery_ticks, "repair end tick")?;
        service_available_at_tick[service_slot] = repair_end_tick;
        service_loss_ticks = add(service_loss_ticks, recovery_ticks, "scenario service loss")?;
        outcomes.push(EventOutcome {
            at_tick: event.at_tick,
            kind: event.kind,
            service,
            repair_start_tick,
            repair_end_tick,
            queue_delay_ticks,
            spare_units_before,
            spare_units_after: spare_units_remaining,
            recovery_ticks,
        });
    }

    let last_repair_end_tick = outcomes
        .iter()
        .map(|event| event.repair_end_tick)
        .max()
        .ok_or_else(|| format!("{}: no repair outcomes", scenario.id))?;

    Ok(ScenarioSimulation {
        service_loss_ticks,
        recovery_elapsed_ticks: last_repair_end_tick - first_event_tick,
        events: outcomes,
        spare_units_remaining,
    })
}

fn evaluate_model(design: Design, contract: &Contract) -> Check<Metrics> {
    let launch_mass_smu = sum_checked(
        &[
            100,
            if design.standardized_interfaces { 8 } else { 0 },
            if design.isolated_power_domains { 18 } else { 0 },
            if design.spare_pack { 12 } else { 0 },
            if design.end_of_life_plan { 5 } else { 0 },
        ],
        "launch mass",
    )?;

    let assembly_ticks = 12_u32
        .checked_sub(if design.standardized_interfaces { 2 } else { 0 })
        .ok_or_else(|| "assembly duration underflow".to_string())?;
    let assembly_ticks = add(
        assembly_ticks,
        if design.isolated_power_domains { 2 } else { 0 },
        "assembly duration",
    )?;
    let assembly_ticks = add(
        assembly_ticks,
        if design.spare_pack { 1 } else { 0 },
        "assembly duration",
    )?;
    let assembly_ticks = add(
        assembly_ticks,
        if design.end_of_life_plan { 1 } else { 0 },
        "assembly duration",
    )?;

    let simulations: Vec<ScenarioSimulation> = contract
        .scenarios
        .iter()
        .map(|scenario| simulate_scenario(design, scenario))
        .collect::<Check<Vec<_>>>()?;
    let loss_values: Vec<u32> = simulations
        .iter()
        .map(|result| result.service_loss_ticks)
        .collect();
    let elapsed_values: Vec<u32> = simulations
        .iter()
        .map(|result| result.recovery_elapsed_ticks)
        .collect();
    let scenario_loss_ticks: [u32; 5] = loss_values
        .try_into()
        .map_err(|_| "scenario count changed during metric computation".to_string())?;
    let scenario_recovery_elapsed_ticks: [u32; 5] = elapsed_values
        .try_into()
        .map_err(|_| "scenario elapsed count changed during metric computation".to_string())?;
    let sum_service_loss_ticks_across_scenarios =
        sum_checked(&scenario_loss_ticks, "scenario loss sum")?;
    let worst_case_service_loss_ticks = *scenario_loss_ticks
        .iter()
        .max()
        .ok_or_else(|| "empty scenario set".to_string())?;
    let worst_case_recovery_elapsed_ticks = *scenario_recovery_elapsed_ticks
        .iter()
        .max()
        .ok_or_else(|| "empty elapsed scenario set".to_string())?;

    Ok(Metrics {
        launch_mass_smu,
        assembly_ticks,
        interface_families: if design.standardized_interfaces { 1 } else { 4 },
        scenario_loss_ticks,
        scenario_recovery_elapsed_ticks,
        sum_service_loss_ticks_across_scenarios,
        worst_case_service_loss_ticks,
        worst_case_recovery_elapsed_ticks,
        end_of_life_plan_declared: design.end_of_life_plan,
        hard_gates_pass: (!contract.end_of_life_plan_required || design.end_of_life_plan)
            && worst_case_service_loss_ticks <= contract.max_worst_case_service_loss_ticks,
    })
}

// This oracle does not call transition() or simulate_scenario(). It independently
// recomputes the fixed event semantics and spare inventory consumption.
fn oracle_scenario_result(design: Design, scenario: &ScenarioContract) -> (u32, u32) {
    // Deliberately separate from simulate_scenario()/transition(): duplicate the
    // tiny domain semantics, then recompute service-local queueing from event time.
    let mut spares = if design.spare_pack { 2_u32 } else { 0_u32 };
    let mut available_at = [0_u32; 4];
    let mut service_loss = 0_u32;
    let first_tick = scenario
        .events
        .first()
        .map(|event| event.at_tick)
        .unwrap_or(0);
    let mut final_tick = first_tick;

    for event in &scenario.events {
        let (service_slot, duration) = match event.kind {
            FaultKind::RobotJointFailure => {
                let duration = if spares > 0 {
                    spares -= 1;
                    2
                } else {
                    6
                };
                (0, duration)
            }
            FaultKind::InterfaceMismatch => (1, if design.standardized_interfaces { 1 } else { 7 }),
            FaultKind::PowerDomainFailure => (2, if design.isolated_power_domains { 1 } else { 8 }),
            FaultKind::ResupplyDelay => {
                let duration = if spares > 0 {
                    spares -= 1;
                    2
                } else {
                    6
                };
                (3, duration)
            }
            FaultKind::CommonCausePowerFailure => {
                (2, if design.isolated_power_domains { 3 } else { 8 })
            }
        };

        let actual_start = if event.at_tick > available_at[service_slot] {
            event.at_tick
        } else {
            available_at[service_slot]
        };
        let end_tick = actual_start + duration;
        available_at[service_slot] = end_tick;
        service_loss += duration;
        final_tick = final_tick.max(end_tick);
    }

    (service_loss, final_tick - first_tick)
}

fn oracle_metrics(design: Design, contract: &Contract) -> Metrics {
    let launch_mass_smu = 100
        + 8 * (design.standardized_interfaces as u32)
        + 18 * (design.isolated_power_domains as u32)
        + 12 * (design.spare_pack as u32)
        + 5 * (design.end_of_life_plan as u32);
    let assembly_ticks = 12 - 2 * u32::from(design.standardized_interfaces)
        + 2 * u32::from(design.isolated_power_domains)
        + u32::from(design.spare_pack)
        + u32::from(design.end_of_life_plan);

    let scenario_loss_ticks: [u32; 5] =
        std::array::from_fn(|index| oracle_scenario_result(design, &contract.scenarios[index]).0);
    let scenario_recovery_elapsed_ticks: [u32; 5] =
        std::array::from_fn(|index| oracle_scenario_result(design, &contract.scenarios[index]).1);
    let sum_service_loss_ticks_across_scenarios = scenario_loss_ticks.iter().sum();
    let worst_case_service_loss_ticks = scenario_loss_ticks.iter().copied().max().unwrap_or(0);
    let worst_case_recovery_elapsed_ticks = scenario_recovery_elapsed_ticks
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
    let end_of_life_plan_declared = design.end_of_life_plan;
    let hard_gates_pass = (!contract.end_of_life_plan_required || end_of_life_plan_declared)
        && worst_case_service_loss_ticks <= contract.max_worst_case_service_loss_ticks;

    Metrics {
        launch_mass_smu,
        assembly_ticks,
        interface_families: if design.standardized_interfaces { 1 } else { 4 },
        scenario_loss_ticks,
        scenario_recovery_elapsed_ticks,
        sum_service_loss_ticks_across_scenarios,
        worst_case_service_loss_ticks,
        worst_case_recovery_elapsed_ticks,
        end_of_life_plan_declared,
        hard_gates_pass,
    }
}

fn verify_metrics(design: Design, actual: &Metrics, contract: &Contract) -> Check<()> {
    let expected = oracle_metrics(design, contract);
    if *actual != expected {
        return Err(format!(
            "{}: verifier disagreement; expected {expected:?}, got {actual:?}",
            design.id()
        ));
    }
    Ok(())
}

fn all_design_specs() -> Vec<DesignSpec> {
    let mut designs = Vec::with_capacity(16);
    for standardized_interfaces in [false, true] {
        for isolated_power_domains in [false, true] {
            for spare_pack in [false, true] {
                for end_of_life_plan in [false, true] {
                    designs.push(DesignSpec {
                        standardized_interfaces: Some(standardized_interfaces),
                        isolated_power_domains: Some(isolated_power_domains),
                        spare_pack: Some(spare_pack),
                        end_of_life_plan: Some(end_of_life_plan),
                    });
                }
            }
        }
    }
    designs
}

fn evaluate_candidate(spec: DesignSpec, contract: &Contract) -> Check<ResultRow> {
    let design = spec.resolve()?;
    let metrics = evaluate_model(design, contract)?;
    verify_metrics(design, &metrics, contract)?;
    Ok(ResultRow { design, metrics })
}

fn dominates(a: &Metrics, b: &Metrics) -> bool {
    let a_objectives = [
        a.launch_mass_smu,
        a.assembly_ticks,
        a.worst_case_service_loss_ticks,
        a.worst_case_recovery_elapsed_ticks,
        a.sum_service_loss_ticks_across_scenarios,
        a.interface_families,
    ];
    let b_objectives = [
        b.launch_mass_smu,
        b.assembly_ticks,
        b.worst_case_service_loss_ticks,
        b.worst_case_recovery_elapsed_ticks,
        b.sum_service_loss_ticks_across_scenarios,
        b.interface_families,
    ];
    a_objectives
        .iter()
        .zip(b_objectives.iter())
        .all(|(left, right)| left <= right)
        && a_objectives
            .iter()
            .zip(b_objectives.iter())
            .any(|(left, right)| left < right)
}

fn pareto_frontier(rows: &[ResultRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.metrics.hard_gates_pass)
        .filter(|candidate| {
            !rows.iter().any(|other| {
                other.metrics.hard_gates_pass
                    && other.design != candidate.design
                    && dominates(&other.metrics, &candidate.metrics)
            })
        })
        .map(|row| row.design.id())
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn validate_source_commit(value: &str) -> Check<()> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character hexadecimal commit SHA".to_string());
    }
    Ok(())
}

fn report(source_commit: Option<&str>) -> Check<Value> {
    let contract = parse_contract(FIXTURE_BYTES)?;
    let rows: Vec<ResultRow> = all_design_specs()
        .into_iter()
        .map(|spec| evaluate_candidate(spec, &contract))
        .collect::<Check<Vec<_>>>()?;

    let mut candidate_reports = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut scenario_reports = Vec::with_capacity(contract.scenarios.len());
        for ((scenario, expected_loss), expected_elapsed) in contract
            .scenarios
            .iter()
            .zip(row.metrics.scenario_loss_ticks)
            .zip(row.metrics.scenario_recovery_elapsed_ticks)
        {
            let simulation = simulate_scenario(row.design, scenario)?;
            if simulation.service_loss_ticks != expected_loss
                || simulation.recovery_elapsed_ticks != expected_elapsed
            {
                return Err(format!("{}: event trace and metric disagree", scenario.id));
            }
            let event_trace: Vec<Value> = simulation
                .events
                .iter()
                .map(|event| {
                    json!({
                        "at_tick": event.at_tick,
                        "kind": event.kind.as_str(),
                        "affected_service": event.service.as_str(),
                        "repair_start_tick": event.repair_start_tick,
                        "repair_end_tick": event.repair_end_tick,
                        "queue_delay_ticks": event.queue_delay_ticks,
                        "spare_units_before": event.spare_units_before,
                        "spare_units_after": event.spare_units_after,
                        "recovery_ticks": event.recovery_ticks
                    })
                })
                .collect();

            scenario_reports.push(json!({
                "scenario_id": &scenario.id,
                "disturbance": &scenario.disturbance,
                "claim_ceiling": &scenario.claim_ceiling,
                "service_loss_ticks": simulation.service_loss_ticks,
                "recovery_elapsed_ticks": simulation.recovery_elapsed_ticks,
                "spare_units_remaining": simulation.spare_units_remaining,
                "event_trace": event_trace
            }));
        }

        candidate_reports.push(json!({
            "design_id": row.design.id(),
            "design": {
                "standardized_interfaces": row.design.standardized_interfaces,
                "isolated_power_domains": row.design.isolated_power_domains,
                "spare_pack": row.design.spare_pack,
                "end_of_life_plan_declared": row.design.end_of_life_plan
            },
            "metrics": {
                "launch_mass_smu": row.metrics.launch_mass_smu,
                "assembly_ticks": row.metrics.assembly_ticks,
                "interface_families": row.metrics.interface_families,
                "scenario_recovery_elapsed_ticks": row.metrics.scenario_recovery_elapsed_ticks,
                "sum_service_loss_ticks_across_scenarios": row.metrics.sum_service_loss_ticks_across_scenarios,
                "worst_case_service_loss_ticks": row.metrics.worst_case_service_loss_ticks,
                "worst_case_recovery_elapsed_ticks": row.metrics.worst_case_recovery_elapsed_ticks,
                "end_of_life_plan_declared": row.metrics.end_of_life_plan_declared,
                "hard_gates_pass": row.metrics.hard_gates_pass,
                "scenario_losses": scenario_reports
            },
            "independent_oracle": "pass"
        }));
    }

    let eligible_count = rows
        .iter()
        .filter(|row| row.metrics.hard_gates_pass)
        .count();
    let frontier = pareto_frontier(&rows);
    Ok(json!({
        "schema_version": "0.2.0",
        "result": "pass",
        "evidence_class": "synthetic",
        "model": "event_driven_discrete_lifecycle_contract_not_physics",
        "claim_ceiling": "This validates arithmetic and finite event/state logic over a synthetic design space; it is not a physical LEO simulation, safety case, or flight qualification.",
        "fixture_sha256": sha256_hex(FIXTURE_BYTES),
        "source_commit": source_commit,
        "units": {
            "mass": contract.mass_unit,
            "time": contract.time_unit,
            "service_loss": "additive_synthetic_service_loss_ticks"
        },
        "objective_directions": {
            "launch_mass_smu": "minimize",
            "assembly_ticks": "minimize",
            "worst_case_service_loss_ticks": "minimize",
            "worst_case_recovery_elapsed_ticks": "minimize",
            "sum_service_loss_ticks_across_scenarios": "minimize",
            "interface_families": "minimize"
        },
        "candidate_count": rows.len(),
        "verified_candidate_count": rows.len(),
        "hard_gate_eligible_count": eligible_count,
        "pareto_frontier_design_ids": frontier,
        "candidates": candidate_reports
    }))
}

fn parse_args() -> Check<Option<String>> {
    let mut args = env::args().skip(1);
    let mut source_commit: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg != "--source-commit" {
            return Err(format!("unknown argument: {arg}"));
        }
        if source_commit.is_some() {
            return Err("--source-commit may be specified once".to_string());
        }
        let value = args
            .next()
            .ok_or_else(|| "--source-commit requires a value".to_string())?;
        validate_source_commit(&value)?;
        source_commit = Some(value);
    }
    Ok(source_commit)
}

fn main() -> ExitCode {
    let outcome = (|| -> Check<()> {
        let source_commit = parse_args()?;
        let output = report(source_commit.as_deref())?;
        let serialized = serde_json::to_string_pretty(&output)
            .map_err(|error| format!("report serialization failed: {error}"))?;
        println!("{serialized}");
        Ok(())
    })();

    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orbital-foundry-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> Contract {
        parse_contract(FIXTURE_BYTES).expect("frozen fixture must parse")
    }

    #[test]
    fn sha256_uses_the_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn fixture_is_strict_and_synthetic() {
        let contract = contract();
        assert_eq!(contract.scenarios.len(), 5);
        assert_eq!(contract.max_worst_case_service_loss_ticks, 12);

        let mut value: Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("quietly_weakened".to_string(), json!(true));
        assert!(parse_contract(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn weakened_service_floor_contract_fails_closed() {
        let mut value: Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["hard_gates"]["max_worst_case_service_loss_ticks"] = json!(13);
        assert!(parse_contract(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn frozen_event_sequences_are_explicit_and_ordered() {
        let contract = contract();
        assert_eq!(contract.scenarios[4].events.len(), 4);
        assert_eq!(contract.scenarios[4].events[0].at_tick, 2);
        assert_eq!(contract.scenarios[4].events[1].at_tick, 2);
        assert_eq!(contract.scenarios[4].events[2].at_tick, 3);
        assert_eq!(contract.scenarios[4].events[3].at_tick, 3);
    }

    #[test]
    fn nonmonotonic_event_time_fails_closed() {
        let mut value: Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["scenarios"][4]["events"][3]["at_tick"] = json!(1);
        assert!(parse_contract(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn unsupported_event_kind_fails_closed() {
        let mut value: Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["scenarios"][0]["events"][0]["kind"] = json!("unknown_failure");
        assert!(parse_contract(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn finite_design_space_has_sixteen_unique_candidates() {
        let specs = all_design_specs();
        assert_eq!(specs.len(), 16);
        let ids: BTreeSet<String> = specs
            .into_iter()
            .map(|spec| spec.resolve().unwrap().id())
            .collect();
        assert_eq!(ids.len(), 16);
    }

    #[test]
    fn every_candidate_matches_the_independent_oracle() {
        let contract = contract();
        for spec in all_design_specs() {
            let row = evaluate_candidate(spec, &contract).unwrap();
            verify_metrics(row.design, &row.metrics, &contract).unwrap();
        }
    }

    #[test]
    fn unknown_configuration_fails_closed() {
        let unknown = DesignSpec {
            standardized_interfaces: Some(true),
            isolated_power_domains: None,
            spare_pack: Some(true),
            end_of_life_plan: Some(true),
        };
        assert!(unknown.resolve().is_err());
    }

    #[test]
    fn missing_end_of_life_plan_cannot_pass_the_gate() {
        let contract = contract();
        let design = Design {
            standardized_interfaces: true,
            isolated_power_domains: true,
            spare_pack: true,
            end_of_life_plan: false,
        };
        let metrics = evaluate_model(design, &contract).unwrap();
        assert!(!metrics.hard_gates_pass);
        assert!(!metrics.end_of_life_plan_declared);
    }

    #[test]
    fn tampered_metrics_are_rejected_by_the_independent_verifier() {
        let contract = contract();
        let design = Design {
            standardized_interfaces: true,
            isolated_power_domains: true,
            spare_pack: true,
            end_of_life_plan: true,
        };
        let mut metrics = evaluate_model(design, &contract).unwrap();
        metrics.launch_mass_smu += 1;
        assert!(verify_metrics(design, &metrics, &contract).is_err());
    }

    #[test]
    fn compound_faults_consume_shared_spare_inventory_in_event_order() {
        let contract = contract();
        let design = Design {
            standardized_interfaces: true,
            isolated_power_domains: true,
            spare_pack: true,
            end_of_life_plan: true,
        };
        let simulation = simulate_scenario(design, &contract.scenarios[4]).unwrap();
        assert_eq!(simulation.service_loss_ticks, 8);
        assert_eq!(simulation.recovery_elapsed_ticks, 4);
        assert_eq!(simulation.spare_units_remaining, 0);
        assert_eq!(simulation.events[0].spare_units_before, 2);
        assert_eq!(simulation.events[0].spare_units_after, 1);
        assert_eq!(simulation.events[1].spare_units_before, 1);
        assert_eq!(simulation.events[1].spare_units_after, 1);
        assert_eq!(simulation.events[2].spare_units_before, 1);
        assert_eq!(simulation.events[2].spare_units_after, 1);
        assert_eq!(simulation.events[3].spare_units_before, 1);
        assert_eq!(simulation.events[3].spare_units_after, 0);
    }

    #[test]
    fn repeated_shared_power_fault_waits_for_prior_recovery() {
        let contract = contract();
        let shared_power = Design {
            standardized_interfaces: true,
            isolated_power_domains: false,
            spare_pack: true,
            end_of_life_plan: true,
        };
        let simulation = simulate_scenario(shared_power, &contract.scenarios[4]).unwrap();
        assert_eq!(simulation.events[1].repair_start_tick, 2);
        assert_eq!(simulation.events[1].repair_end_tick, 10);
        assert_eq!(simulation.events[2].at_tick, 3);
        assert_eq!(simulation.events[2].repair_start_tick, 10);
        assert_eq!(simulation.events[2].queue_delay_ticks, 7);
        assert_eq!(simulation.events[2].repair_end_tick, 18);
        assert_eq!(simulation.service_loss_ticks, 20);
        assert_eq!(simulation.recovery_elapsed_ticks, 16);
        assert!(
            !evaluate_model(shared_power, &contract)
                .unwrap()
                .hard_gates_pass
        );
    }

    #[test]
    fn output_is_reproducible_without_an_external_commit_binding() {
        let a = serde_json::to_vec(&report(None).unwrap()).unwrap();
        let b = serde_json::to_vec(&report(None).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn frontier_preserves_tradeoffs_instead_of_collapsing_to_one_score() {
        let contract = contract();
        let rows: Vec<ResultRow> = all_design_specs()
            .into_iter()
            .map(|spec| evaluate_candidate(spec, &contract).unwrap())
            .collect();
        assert_eq!(
            rows.iter()
                .filter(|row| row.metrics.hard_gates_pass)
                .count(),
            2
        );
        let frontier = pareto_frontier(&rows);
        assert_eq!(frontier.len(), 2);
        assert!(
            frontier
                .iter()
                .any(|id| id.as_str() == "std=1|power_isolated=1|spares=1|eol_plan=1")
        );
        assert!(
            frontier
                .iter()
                .any(|id| id.as_str() == "std=0|power_isolated=1|spares=1|eol_plan=1")
        );
    }

    #[test]
    fn metamorphic_spares_never_increase_synthetic_service_loss() {
        let contract = contract();
        for mut design in [
            Design {
                standardized_interfaces: false,
                isolated_power_domains: false,
                spare_pack: false,
                end_of_life_plan: false,
            },
            Design {
                standardized_interfaces: false,
                isolated_power_domains: true,
                spare_pack: false,
                end_of_life_plan: true,
            },
            Design {
                standardized_interfaces: true,
                isolated_power_domains: false,
                spare_pack: false,
                end_of_life_plan: true,
            },
            Design {
                standardized_interfaces: true,
                isolated_power_domains: true,
                spare_pack: false,
                end_of_life_plan: true,
            },
        ] {
            let without_spares = evaluate_model(design, &contract).unwrap();
            design.spare_pack = true;
            let with_spares = evaluate_model(design, &contract).unwrap();
            for (before, after) in without_spares
                .scenario_loss_ticks
                .iter()
                .zip(with_spares.scenario_loss_ticks.iter())
            {
                assert!(
                    after <= before,
                    "spare pack worsened service loss: {design:?}"
                );
            }
        }
    }

    #[test]
    fn metamorphic_power_isolation_never_worsens_fault_metrics() {
        let contract = contract();
        for standardized_interfaces in [false, true] {
            for spare_pack in [false, true] {
                for end_of_life_plan in [false, true] {
                    let shared = Design {
                        standardized_interfaces,
                        isolated_power_domains: false,
                        spare_pack,
                        end_of_life_plan,
                    };
                    let isolated = Design {
                        isolated_power_domains: true,
                        ..shared
                    };
                    let shared_metrics = evaluate_model(shared, &contract).unwrap();
                    let isolated_metrics = evaluate_model(isolated, &contract).unwrap();
                    for (before, after) in shared_metrics
                        .scenario_loss_ticks
                        .iter()
                        .zip(isolated_metrics.scenario_loss_ticks.iter())
                    {
                        assert!(after <= before, "isolation worsened service loss");
                    }
                    for (before, after) in shared_metrics
                        .scenario_recovery_elapsed_ticks
                        .iter()
                        .zip(isolated_metrics.scenario_recovery_elapsed_ticks.iter())
                    {
                        assert!(after <= before, "isolation worsened recovery elapsed time");
                    }
                }
            }
        }
    }

    #[test]
    fn metamorphic_interface_standardization_exposes_mass_performance_tradeoff() {
        let contract = contract();
        let bespoke = Design {
            standardized_interfaces: false,
            isolated_power_domains: true,
            spare_pack: true,
            end_of_life_plan: true,
        };
        let standardized = Design {
            standardized_interfaces: true,
            ..bespoke
        };
        let bespoke_metrics = evaluate_model(bespoke, &contract).unwrap();
        let standard_metrics = evaluate_model(standardized, &contract).unwrap();
        assert_eq!(
            standard_metrics.launch_mass_smu,
            bespoke_metrics.launch_mass_smu + 8
        );
        assert_eq!(
            standard_metrics.assembly_ticks + 2,
            bespoke_metrics.assembly_ticks
        );
        assert_eq!(standard_metrics.interface_families, 1);
        assert_eq!(bespoke_metrics.interface_families, 4);
        assert_eq!(bespoke_metrics.scenario_loss_ticks[1], 7);
        assert_eq!(standard_metrics.scenario_loss_ticks[1], 1);
        for i in [0_usize, 2, 3, 4] {
            assert_eq!(
                standard_metrics.scenario_loss_ticks[i],
                bespoke_metrics.scenario_loss_ticks[i]
            );
        }
    }

    #[test]
    fn source_commit_binding_rejects_malformed_sha() {
        assert!(validate_source_commit(&"g".repeat(40)).is_err());
        assert!(validate_source_commit(&"a".repeat(39)).is_err());
        assert!(validate_source_commit(&"a".repeat(40)).is_ok());
    }
}

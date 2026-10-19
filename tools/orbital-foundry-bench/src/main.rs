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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Contract {
    mass_unit: String,
    time_unit: String,
    max_worst_case_service_loss_ticks: u32,
    end_of_life_plan_required: bool,
    scenario_ids: Vec<String>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Metrics {
    launch_mass_smu: u32,
    assembly_ticks: u32,
    interface_families: u32,
    scenario_loss_ticks: [u32; 5],
    sum_service_loss_ticks_across_scenarios: u32,
    worst_case_service_loss_ticks: u32,
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

    if string_field(&root, "schema_version", "root")? != "0.1.0"
        || string_field(&root, "evidence_class", "root")? != "synthetic"
        || string_field(&root, "model", "root")? != "discrete_lifecycle_contract_not_physics"
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

    let scenarios = field(&root, "scenarios", "root")?
        .as_array()
        .ok_or_else(|| "scenarios: expected array".to_string())?;
    if scenarios.len() != SCENARIO_IDS.len() {
        return Err(format!(
            "scenarios: expected {} frozen scenarios, got {}",
            SCENARIO_IDS.len(),
            scenarios.len()
        ));
    }

    let mut scenario_ids = Vec::with_capacity(scenarios.len());
    for (index, scenario) in scenarios.iter().enumerate() {
        let at = format!("scenarios[{index}]");
        exact_keys(scenario, &["id", "disturbance", "claim_ceiling"], &at)?;
        let id = string_field(scenario, "id", &at)?;
        let _disturbance = string_field(scenario, "disturbance", &at)?;
        let ceiling = string_field(scenario, "claim_ceiling", &at)?;
        if id != SCENARIO_IDS[index] || !ceiling.starts_with("synthetic ") {
            return Err(format!(
                "{at}: scenario order/identity or synthetic claim ceiling changed"
            ));
        }
        scenario_ids.push(id);
    }

    Ok(Contract {
        mass_unit,
        time_unit,
        max_worst_case_service_loss_ticks,
        end_of_life_plan_required,
        scenario_ids,
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

    let robot_loss = if design.spare_pack { 2 } else { 6 };
    let interface_loss = if design.standardized_interfaces { 1 } else { 7 };
    let power_loss = if design.isolated_power_domains { 1 } else { 8 };
    let supply_loss = if design.spare_pack { 2 } else { 6 };
    let common_cause_loss = sum_checked(
        &[
            robot_loss,
            if design.isolated_power_domains { 3 } else { 8 },
            supply_loss,
        ],
        "common-cause service loss",
    )?;
    let scenario_loss_ticks = [
        robot_loss,
        interface_loss,
        power_loss,
        supply_loss,
        common_cause_loss,
    ];
    let sum_service_loss_ticks_across_scenarios =
        sum_checked(&scenario_loss_ticks, "scenario loss sum")?;
    let worst_case_service_loss_ticks = *scenario_loss_ticks
        .iter()
        .max()
        .ok_or_else(|| "empty scenario set".to_string())?;

    Ok(Metrics {
        launch_mass_smu,
        assembly_ticks,
        interface_families: if design.standardized_interfaces { 1 } else { 4 },
        scenario_loss_ticks,
        sum_service_loss_ticks_across_scenarios,
        worst_case_service_loss_ticks,
        end_of_life_plan_declared: design.end_of_life_plan,
        hard_gates_pass: (!contract.end_of_life_plan_required || design.end_of_life_plan)
            && worst_case_service_loss_ticks <= contract.max_worst_case_service_loss_ticks,
    })
}

// This oracle intentionally does not call evaluate_model or its arithmetic helpers.
// It recomputes each field through a separate, simple expression over the tiny finite domain.
fn oracle_metrics(design: Design, contract: &Contract) -> Metrics {
    let launch_mass_smu = 100
        + (design.standardized_interfaces as u32) * 8
        + (design.isolated_power_domains as u32) * 18
        + (design.spare_pack as u32) * 12
        + (design.end_of_life_plan as u32) * 5;

    let assembly_ticks = 12 - 2 * (design.standardized_interfaces as u32)
        + 2 * (design.isolated_power_domains as u32)
        + (design.spare_pack as u32)
        + (design.end_of_life_plan as u32);

    let robot_loss = match design.spare_pack {
        true => 2,
        false => 6,
    };
    let interface_loss = match design.standardized_interfaces {
        true => 1,
        false => 7,
    };
    let power_loss = match design.isolated_power_domains {
        true => 1,
        false => 8,
    };
    let resupply_loss = if design.spare_pack { 2 } else { 6 };
    let common_cause_power_loss = if design.isolated_power_domains { 3 } else { 8 };
    let common_cause_loss = robot_loss + common_cause_power_loss + resupply_loss;
    let scenario_loss_ticks = [
        robot_loss,
        interface_loss,
        power_loss,
        resupply_loss,
        common_cause_loss,
    ];
    let sum_service_loss_ticks_across_scenarios = scenario_loss_ticks.iter().sum();
    let worst_case_service_loss_ticks = scenario_loss_ticks.iter().copied().max().unwrap_or(0);
    let end_of_life_plan_declared = design.end_of_life_plan;
    let hard_gates_pass = (!contract.end_of_life_plan_required || end_of_life_plan_declared)
        && worst_case_service_loss_ticks <= contract.max_worst_case_service_loss_ticks;

    Metrics {
        launch_mass_smu,
        assembly_ticks,
        interface_families: if design.standardized_interfaces { 1 } else { 4 },
        scenario_loss_ticks,
        sum_service_loss_ticks_across_scenarios,
        worst_case_service_loss_ticks,
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
        a.sum_service_loss_ticks_across_scenarios,
        a.interface_families,
    ];
    let b_objectives = [
        b.launch_mass_smu,
        b.assembly_ticks,
        b.worst_case_service_loss_ticks,
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
    let specs = all_design_specs();
    let mut rows = Vec::with_capacity(specs.len());
    for spec in specs {
        rows.push(evaluate_candidate(spec, &contract)?);
    }

    let scenario_reports: Vec<Value> = rows
        .iter()
        .map(|row| {
            let scenarios: Vec<Value> = contract
                .scenario_ids
                .iter()
                .zip(row.metrics.scenario_loss_ticks.iter())
                .map(|(scenario_id, loss)| {
                    json!({
                        "scenario_id": scenario_id,
                        "service_loss_ticks": loss
                    })
                })
                .collect();
            json!({
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
                    "sum_service_loss_ticks_across_scenarios": row.metrics.sum_service_loss_ticks_across_scenarios,
                    "worst_case_service_loss_ticks": row.metrics.worst_case_service_loss_ticks,
                    "end_of_life_plan_declared": row.metrics.end_of_life_plan_declared,
                    "hard_gates_pass": row.metrics.hard_gates_pass,
                    "scenario_losses": scenarios
                },
                "independent_oracle": "pass"
            })
        })
        .collect();

    let eligible_count = rows
        .iter()
        .filter(|row| row.metrics.hard_gates_pass)
        .count();
    let verified_count = rows.len(); // evaluate_candidate fails unless every row verifies.
    let frontier = pareto_frontier(&rows);
    Ok(json!({
        "schema_version": "0.1.0",
        "result": "pass",
        "evidence_class": "synthetic",
        "model": "discrete_lifecycle_contract_not_physics",
        "claim_ceiling": "This validates arithmetic and search logic over a synthetic finite design space; it is not a physical LEO simulation, safety case, or flight qualification.",
        "fixture_sha256": sha256_hex(FIXTURE_BYTES),
        "source_commit": source_commit,
        "units": {
            "mass": contract.mass_unit,
            "time": contract.time_unit,
            "service_loss": contract.time_unit
        },
        "objective_directions": {
            "launch_mass_smu": "minimize",
            "assembly_ticks": "minimize",
            "worst_case_service_loss_ticks": "minimize",
            "sum_service_loss_ticks_across_scenarios": "minimize",
            "interface_families": "minimize"
        },
        "candidate_count": rows.len(),
        "verified_candidate_count": verified_count,
        "hard_gate_eligible_count": eligible_count,
        "pareto_frontier_design_ids": frontier,
        "candidates": scenario_reports
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
        assert_eq!(contract.scenario_ids.len(), 5);
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
            4
        );
        let frontier = pareto_frontier(&rows);
        assert_eq!(frontier.len(), 4);
        assert!(
            frontier
                .iter()
                .any(|id| id.as_str() == "std=1|power_isolated=1|spares=1|eol_plan=1")
        );
        assert!(
            frontier
                .iter()
                .any(|id| id.as_str() == "std=1|power_isolated=0|spares=1|eol_plan=1")
        );
    }

    #[test]
    fn source_commit_binding_rejects_malformed_sha() {
        assert!(validate_source_commit(&"g".repeat(40)).is_err());
        assert!(validate_source_commit(&"a".repeat(39)).is_err());
        assert!(validate_source_commit(&"a".repeat(40)).is_ok());
    }
}

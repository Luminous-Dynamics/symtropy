use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process,
};

const EVALUATOR_VERSION: &str = "civ-res-ind-oracle/0.1.0";
const MANIFEST_SCHEMA: &str = "civ-resilience-restart-manifest/0.1";
const FIXTURE_SCHEMA: &str = "civ-resilience-restart-fixture/0.1";
const SUPPORTED_EXPECTED_FIELDS: &[&str] = &[
    "service_delivery_by_step",
    "service_floors_met_by_step",
    "conforming_output_by_step",
    "total_conforming_output",
    "first_step_all_service_floors_met",
    "first_conforming_output_step",
    "food_preservation_unmet_units",
    "blocked_production_steps",
];

fn main() {
    if let Err(error) = run_cli() {
        eprintln!("civ-res-ind-oracle: {error}");
        process::exit(1);
    }
}

fn run_cli() -> Result<(), String> {
    let args = parse_args(env::args().skip(1))?;
    let fixture_bytes = fs::read(&args.fixture)
        .map_err(|error| format!("{}: cannot read fixture: {error}", args.fixture.display()))?;
    let manifest_bytes = fs::read(&args.manifest)
        .map_err(|error| format!("{}: cannot read manifest: {error}", args.manifest.display()))?;
    let source_bytes = fs::read(&args.source).map_err(|error| {
        format!(
            "{}: cannot read evaluator source: {error}",
            args.source.display()
        )
    })?;
    let lock_bytes = fs::read(&args.lock)
        .map_err(|error| format!("{}: cannot read Cargo.lock: {error}", args.lock.display()))?;

    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("manifest: invalid JSON: {error}"))?;
    verify_manifest(&manifest, &args, &fixture_bytes, &source_bytes, &lock_bytes)?;

    let fixture: Value = serde_json::from_slice(&fixture_bytes)
        .map_err(|error| format!("fixture: invalid JSON: {error}"))?;
    let report = evaluate_fixture(&fixture, &sha256_hex(&fixture_bytes), &manifest)?;
    let mut encoded = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("report serialization failed: {error}"))?;
    encoded.push('\n');

    if let Some(output) = args.output {
        fs::write(&output, encoded)
            .map_err(|error| format!("{}: cannot write report: {error}", output.display()))?;
    } else {
        print!("{encoded}");
    }
    Ok(())
}

#[derive(Debug)]
struct CliArgs {
    fixture: PathBuf,
    manifest: PathBuf,
    source: PathBuf,
    lock: PathBuf,
    output: Option<PathBuf>,
}

fn parse_args<I>(mut args: I) -> Result<CliArgs, String>
where
    I: Iterator<Item = String>,
{
    let mut fixture = None;
    let mut manifest = None;
    let mut source = None;
    let mut lock = None;
    let mut output = None;

    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            return Err(
                "usage: civ-res-ind-oracle --fixture PATH --manifest PATH --source PATH --lock PATH [--output PATH]"
                    .to_owned(),
            );
        }
        let value = args
            .next()
            .ok_or_else(|| format!("{flag}: expected a path argument"))?;
        match flag.as_str() {
            "--fixture" => fixture = Some(PathBuf::from(value)),
            "--manifest" => manifest = Some(PathBuf::from(value)),
            "--source" => source = Some(PathBuf::from(value)),
            "--lock" => lock = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            unknown => return Err(format!("unknown argument {unknown}")),
        }
    }

    Ok(CliArgs {
        fixture: fixture.ok_or_else(|| "missing --fixture".to_owned())?,
        manifest: manifest.ok_or_else(|| "missing --manifest".to_owned())?,
        source: source.ok_or_else(|| "missing --source".to_owned())?,
        lock: lock.ok_or_else(|| "missing --lock".to_owned())?,
        output,
    })
}

fn verify_manifest(
    manifest: &Value,
    args: &CliArgs,
    fixture_bytes: &[u8],
    source_bytes: &[u8],
    lock_bytes: &[u8],
) -> Result<(), String> {
    let schema = string_at(manifest, "$manifest", "schema_version")?;
    if schema != MANIFEST_SCHEMA {
        return Err(format!(
            "$manifest.schema_version: expected {MANIFEST_SCHEMA}, got {schema}"
        ));
    }
    let evaluator = string_at(manifest, "$manifest", "evaluator_version")?;
    if evaluator != EVALUATOR_VERSION {
        return Err(format!(
            "$manifest.evaluator_version: expected {EVALUATOR_VERSION}, got {evaluator}"
        ));
    }

    verify_declared_path(manifest, "fixture_path", &args.fixture)?;
    verify_declared_path(manifest, "manifest_path", &args.manifest)?;
    verify_declared_path(manifest, "evaluator_path", &args.source)?;
    verify_declared_path(manifest, "cargo_lock_path", &args.lock)?;

    verify_sha_field(manifest, "fixture_sha256", fixture_bytes)?;
    verify_sha_field(manifest, "evaluator_source_sha256", source_bytes)?;
    verify_sha_field(manifest, "cargo_lock_sha256", lock_bytes)?;

    let policy = string_at(manifest, "$manifest", "policy_id")?;
    if policy.trim().is_empty() {
        return Err("$manifest.policy_id: must not be empty".to_owned());
    }
    Ok(())
}

fn verify_declared_path(manifest: &Value, key: &str, actual: &Path) -> Result<(), String> {
    let declared = string_at(manifest, "$manifest", key)?;
    let actual = actual.to_string_lossy();
    if declared != actual.as_ref() {
        return Err(format!(
            "$manifest.{key}: path mismatch; declared {declared:?}, received {actual:?}"
        ));
    }
    Ok(())
}

fn verify_sha_field(manifest: &Value, key: &str, bytes: &[u8]) -> Result<(), String> {
    let expected = string_at(manifest, "$manifest", key)?;
    if !is_sha256_hex(expected) {
        return Err(format!(
            "$manifest.{key}: must be exactly 64 lowercase hexadecimal characters"
        ));
    }
    let actual = sha256_hex(bytes);
    if expected != actual {
        return Err(format!(
            "$manifest.{key}: digest mismatch; expected {expected}, actual {actual}"
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn evaluate_fixture(
    fixture: &Value,
    fixture_digest: &str,
    manifest: &Value,
) -> Result<Value, String> {
    let schema = string_at(fixture, "$fixture", "schema_version")?;
    if schema != FIXTURE_SCHEMA {
        return Err(format!(
            "$fixture.schema_version: expected {FIXTURE_SCHEMA}, got {schema}"
        ));
    }
    let rules = field(fixture, "$fixture", "reference_rules")?;
    let order_values = array_at(
        rules,
        "$fixture.reference_rules",
        "service_allocation_order",
    )?;
    let mut service_order = Vec::with_capacity(order_values.len());
    let mut unique_services = BTreeSet::new();
    for (index, item) in order_values.iter().enumerate() {
        let service = item.as_str().ok_or_else(|| {
            format!("$fixture.reference_rules.service_allocation_order[{index}]: expected string")
        })?;
        if service.trim().is_empty() {
            return Err(format!(
                "$fixture.reference_rules.service_allocation_order[{index}]: empty service ID"
            ));
        }
        if !unique_services.insert(service.to_owned()) {
            return Err(format!(
                "$fixture.reference_rules.service_allocation_order[{index}]: duplicate service {service}"
            ));
        }
        service_order.push(service.to_owned());
    }
    if service_order.is_empty() {
        return Err(
            "$fixture.reference_rules.service_allocation_order: must not be empty".to_owned(),
        );
    }

    let demands_value = field(
        rules,
        "$fixture.reference_rules",
        "service_demands_per_step",
    )?;
    let demands_obj = demands_value.as_object().ok_or_else(|| {
        "$fixture.reference_rules.service_demands_per_step: expected object".to_owned()
    })?;
    if demands_obj.len() != service_order.len() {
        return Err(
            "$fixture.reference_rules.service_demands_per_step: keys must exactly match allocation order"
                .to_owned(),
        );
    }
    let mut demands = Vec::with_capacity(service_order.len());
    for service in &service_order {
        let demand = demands_obj.get(service).ok_or_else(|| {
            format!("$fixture.reference_rules.service_demands_per_step.{service}: missing demand")
        })?;
        demands.push(positive_u64(
            demand,
            &format!("$fixture.reference_rules.service_demands_per_step.{service}"),
        )?);
    }

    let service_power = positive_u64(
        field(
            rules,
            "$fixture.reference_rules",
            "service_power_units_per_service_unit",
        )?,
        "$fixture.reference_rules.service_power_units_per_service_unit",
    )?;
    let production_energy = positive_u64(
        field(
            rules,
            "$fixture.reference_rules",
            "workshop_energy_required_per_reference_unit",
        )?,
        "$fixture.reference_rules.workshop_energy_required_per_reference_unit",
    )?;
    let production_cap = positive_u64(
        field(
            rules,
            "$fixture.reference_rules",
            "max_conforming_reference_units_per_step",
        )?,
        "$fixture.reference_rules.max_conforming_reference_units_per_step",
    )?;

    let scenarios = array_at(fixture, "$fixture", "scenarios")?;
    if scenarios.is_empty() {
        return Err("$fixture.scenarios: must contain at least one scenario".to_owned());
    }
    let mut scenario_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(scenarios.len());
    for (index, scenario) in scenarios.iter().enumerate() {
        let path = format!("$fixture.scenarios[{index}]");
        let id = string_at(scenario, &path, "id")?;
        if id.trim().is_empty() {
            return Err(format!("{path}.id: must not be empty"));
        }
        if !scenario_ids.insert(id.to_owned()) {
            return Err(format!("{path}.id: duplicate scenario ID {id}"));
        }
        let result = evaluate_scenario(
            scenario,
            &path,
            &service_order,
            &demands,
            service_power,
            production_energy,
            production_cap,
        )?;
        results.push(result);
    }
    results.sort_by(|left, right| {
        left["scenario_id"]
            .as_str()
            .cmp(&right["scenario_id"].as_str())
    });

    let causal = field(fixture, "$fixture", "causality_pair")?;
    let causal_path = "$fixture.causality_pair";
    let causal_id = string_at(causal, causal_path, "id")?;
    if causal_id != "S06_FUTURE_BLINDNESS" {
        return Err(format!(
            "{causal_path}.id: expected S06_FUTURE_BLINDNESS, got {causal_id}"
        ));
    }
    let shared_step = u64_at(causal, causal_path, "shared_observations_through_step")?;
    let left_hidden = string_at(causal, causal_path, "left_hidden_event_at_step_2")?;
    let right_hidden = string_at(causal, causal_path, "right_hidden_event_at_step_2")?;
    if left_hidden == right_hidden {
        return Err(format!(
            "{causal_path}: hidden event pair must distinguish the two campaigns"
        ));
    }
    let causal_expected = string_at(causal, causal_path, "expected")?;
    if causal_expected.trim().is_empty() {
        return Err(format!("{causal_path}.expected: must not be empty"));
    }
    let trace_report = verify_policy_trace_pair(causal, shared_step, causal_id)?;

    let policy_id = string_at(manifest, "$manifest", "policy_id")?;
    Ok(json!({
        "status": "PASS_SYNTHETIC_REFERENCE_ONLY",
        "claim_ceiling": "Declared synthetic accounting semantics only; not evidence of physical performance, safety, savings, or real-world resilience.",
        "evaluator_version": EVALUATOR_VERSION,
        "evaluator_source_sha256": string_at(manifest, "$manifest", "evaluator_source_sha256")?,
        "cargo_lock_sha256": string_at(manifest, "$manifest", "cargo_lock_sha256")?,
        "fixture_sha256": fixture_digest,
        "fixture_schema_version": schema,
        "policy_id": policy_id,
        "scenario_count": results.len(),
        "scenario_results": results,
        "future_blindness_contract": trace_report
    }))
}

const POLICY_TRACE_PAIR_SCHEMA_V1: &str = "civ-res-policy-trace-pair/0.1";

/// Verifies a paired action trace using only policy-visible observation histories.
/// This validates supplied synthetic receipts; it does not execute or sandbox a deployed policy.
fn verify_policy_trace_pair(
    causal: &Value,
    shared_step: u64,
    causal_id: &str,
) -> Result<Value, String> {
    let pair_path = "$fixture.causality_pair.policy_trace_pair";
    let pair = field(causal, "$fixture.causality_pair", "policy_trace_pair")?;
    let schema = string_at(pair, pair_path, "schema_version")?;
    if schema != POLICY_TRACE_PAIR_SCHEMA_V1 {
        return Err(format!(
            "{pair_path}.schema_version: expected {POLICY_TRACE_PAIR_SCHEMA_V1}, got {schema}"
        ));
    }

    let left = field(pair, pair_path, "left")?;
    let right = field(pair, pair_path, "right")?;
    let left_path = format!("{pair_path}.left");
    let right_path = format!("{pair_path}.right");
    let left_policy_id = string_at(left, &left_path, "policy_id")?;
    let right_policy_id = string_at(right, &right_path, "policy_id")?;
    let left_policy_version = string_at(left, &left_path, "policy_version")?;
    let right_policy_version = string_at(right, &right_path, "policy_version")?;
    if left_policy_id.trim().is_empty() || left_policy_version.trim().is_empty() {
        return Err(format!(
            "{pair_path}: policy identity/version must not be empty"
        ));
    }
    if left_policy_id != right_policy_id || left_policy_version != right_policy_version {
        return Err(format!(
            "{pair_path}: paired traces must use the same policy ID and version"
        ));
    }

    let left_steps = array_at(left, &left_path, "steps")?;
    let right_steps = array_at(right, &right_path, "steps")?;
    if left_steps.len() != right_steps.len() || left_steps.len() <= shared_step as usize {
        return Err(format!(
            "{pair_path}: trace lengths must match and cover the declared shared prefix"
        ));
    }

    let mut first_observation_divergence: Option<u64> = None;
    let mut first_action_divergence: Option<u64> = None;
    let mut trace_step_receipts: Vec<Value> = Vec::with_capacity(left_steps.len());
    for (index, (left_step, right_step)) in left_steps.iter().zip(right_steps).enumerate() {
        let step_path = format!("{pair_path}.step[{index}]");
        let left_step_path = format!("{step_path}.left");
        let right_step_path = format!("{step_path}.right");
        let left_step_no = u64_at(left_step, &left_step_path, "step")?;
        let right_step_no = u64_at(right_step, &right_step_path, "step")?;
        if left_step_no != index as u64 || right_step_no != index as u64 {
            return Err(format!(
                "{step_path}: step indices must be contiguous and start at zero"
            ));
        }

        let left_history = field(left_step, &left_step_path, "visible_history")?;
        let right_history = field(right_step, &right_step_path, "visible_history")?;
        validate_visible_history(
            left_history,
            left_step_no,
            &format!("{left_step_path}.visible_history"),
        )?;
        validate_visible_history(
            right_history,
            right_step_no,
            &format!("{right_step_path}.visible_history"),
        )?;

        let left_input = field(left_step, &left_step_path, "policy_input")?;
        let right_input = field(right_step, &right_step_path, "policy_input")?;
        validate_policy_input(
            left_input,
            left_step_no,
            left_history,
            &format!("{left_step_path}.policy_input"),
        )?;
        validate_policy_input(
            right_input,
            right_step_no,
            right_history,
            &format!("{right_step_path}.policy_input"),
        )?;

        if left_history != right_history {
            first_observation_divergence.get_or_insert(left_step_no);
            if left_step_no <= shared_step {
                return Err(format!(
                    "{step_path}: visible histories differ within the declared shared prefix"
                ));
            }
        }
        let left_action = field(left_step, &left_step_path, "action")?;
        let right_action = field(right_step, &right_step_path, "action")?;
        let recomputed_left_action = criticality_heuristic_action(left_input)?;
        let recomputed_right_action = criticality_heuristic_action(right_input)?;
        if left_action != &recomputed_left_action {
            return Err(format!(
                "{left_step_path}.action: supplied action does not match the visible-input reference policy"
            ));
        }
        if right_action != &recomputed_right_action {
            return Err(format!(
                "{right_step_path}.action: supplied action does not match the visible-input reference policy"
            ));
        }
        let left_input_bytes = serde_json::to_vec(left_input)
            .map_err(|error| format!("{left_step_path}.policy_input: serialize failed: {error}"))?;
        let right_input_bytes = serde_json::to_vec(right_input).map_err(|error| {
            format!("{right_step_path}.policy_input: serialize failed: {error}")
        })?;
        let left_action_bytes = serde_json::to_vec(left_action)
            .map_err(|error| format!("{left_step_path}.action: serialize failed: {error}"))?;
        let right_action_bytes = serde_json::to_vec(right_action)
            .map_err(|error| format!("{right_step_path}.action: serialize failed: {error}"))?;
        let left_input_sha256 = sha256_hex(&left_input_bytes);
        let right_input_sha256 = sha256_hex(&right_input_bytes);
        let left_action_sha256 = sha256_hex(&left_action_bytes);
        let right_action_sha256 = sha256_hex(&right_action_bytes);
        if left_action != right_action {
            first_action_divergence.get_or_insert(left_step_no);
            if left_history == right_history {
                return Err(format!(
                    "{step_path}: actions differ despite identical policy-visible histories"
                ));
            }
        }
        trace_step_receipts.push(json!({
            "step": left_step_no,
            "policy_input_sha256_left": left_input_sha256,
            "policy_input_sha256_right": right_input_sha256,
            "policy_input_digest_equal": left_input_sha256 == right_input_sha256,
            "action_sha256_left": left_action_sha256,
            "action_sha256_right": right_action_sha256,
            "action_digest_equal": left_action_sha256 == right_action_sha256
        }));
    }

    if first_observation_divergence.is_none() {
        return Err(format!(
            "{pair_path}: paired futures never produce a differing visible history"
        ));
    }
    Ok(json!({
        "id": causal_id,
        "status": "RECORDED_TRACE_PREFIX_MATCHED_SYNTHETIC",
        "policy_id": left_policy_id,
        "policy_version": left_policy_version,
        "shared_observations_through_step": shared_step,
        "matched_prefix_actions": shared_step + 1,
        "first_visible_history_divergence_step": first_observation_divergence,
        "first_action_divergence_step": first_action_divergence,
        "trace_step_receipts": trace_step_receipts,
        "claim_ceiling": "SHA-256 digests bind deterministic serde_json serializations for replay comparison, not signatures or source authenticity. Checks fixture-supplied traces only; it does not execute/sandbox a live policy, prove absence of runtime side channels, or establish physical performance, safety, savings, or real-world resilience."
    }))
}

fn validate_visible_history(history: &Value, step: u64, path: &str) -> Result<(), String> {
    let entries = history
        .as_array()
        .ok_or_else(|| format!("{path}: expected an array"))?;
    if entries.len() != step as usize + 1 {
        return Err(format!(
            "{path}: must contain exactly the observation prefix through this step"
        ));
    }
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        let observed_step = u64_at(entry, &entry_path, "step")?;
        if observed_step != index as u64 {
            return Err(format!(
                "{entry_path}.step: history must be contiguous and ordered"
            ));
        }
        let object = entry.as_object().ok_or_else(|| {
            format!("{entry_path}: expected object with only step and observation fields")
        })?;
        if object.len() != 2 || !object.contains_key("step") || !object.contains_key("observation")
        {
            return Err(format!(
                "{entry_path}: expected only step and observation fields"
            ));
        }
        let observation = field(entry, &entry_path, "observation")?;
        if !observation.is_object() {
            return Err(format!("{entry_path}.observation: expected an object"));
        }
    }
    Ok(())
}

fn validate_policy_input(
    input: &Value,
    step: u64,
    visible_history: &Value,
    path: &str,
) -> Result<(), String> {
    reject_forbidden_policy_fields(input, path)?;
    let object = input
        .as_object()
        .ok_or_else(|| format!("{path}: expected an object"))?;
    if object.len() != 2
        || !object.contains_key("step")
        || !object.contains_key("observation_history")
    {
        return Err(format!(
            "{path}: policy input must contain only step and observation_history; hidden/future fields are forbidden"
        ));
    }
    let input_step = u64_at(input, path, "step")?;
    if input_step != step {
        return Err(format!(
            "{path}.step: does not match the current decision step"
        ));
    }
    let history = field(input, path, "observation_history")?;
    if history != visible_history {
        return Err(format!(
            "{path}.observation_history: does not match the exact visible history"
        ));
    }
    Ok(())
}

/// A transparent reference policy, deliberately restricted to the exact visible-input envelope.
/// This is a test baseline, not an industrial scheduler or a safety controller.
fn criticality_heuristic_action(input: &Value) -> Result<Value, String> {
    let path = "$policy_input";
    reject_forbidden_policy_fields(input, path)?;
    let object = input
        .as_object()
        .ok_or_else(|| format!("{path}: expected an object"))?;
    if object.len() != 2
        || !object.contains_key("step")
        || !object.contains_key("observation_history")
    {
        return Err(format!(
            "{path}: policy input must contain only step and observation_history"
        ));
    }
    let step = u64_at(input, path, "step")?;
    let history = field(input, path, "observation_history")?;
    validate_visible_history(history, step, &format!("{path}.observation_history"))?;
    let current = history
        .as_array()
        .and_then(|entries| entries.last())
        .ok_or_else(|| format!("{path}.observation_history: current observation missing"))?;
    let observation = field(current, &format!("{path}.observation_history[{step}]"), "observation")?;

    if observation.get("controller_operational").and_then(Value::as_bool) == Some(false) {
        return Ok(json!({"kind": "request_controller_repair"}));
    }
    if observation.get("service_floors_met").and_then(Value::as_bool) == Some(false) {
        return Ok(json!({"kind": "preserve_essential_service_floors"}));
    }
    if step == 0 {
        return Ok(json!({"kind": "preserve_essential_service_floors"}));
    }
    if observation.get("service_floors_met").and_then(Value::as_bool) == Some(true) {
        if step == 1 {
            return Ok(json!({"kind": "reserve_repair_stock"}));
        }
        return Ok(json!({"kind": "produce_if_gates_pass"}));
    }

    // Missing current-step floor evidence is not favorable evidence.
    Ok(json!({"kind": "request_service_floor_observation"}))
}

fn reject_forbidden_policy_fields(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let normalized = key.to_ascii_lowercase().replace('-', "_");
                if normalized.contains("future")
                    || normalized.contains("hidden_event")
                    || normalized == "scenario_id"
                    || normalized == "world_state"
                    || normalized == "oracle_state"
                {
                    return Err(format!(
                        "{path}.{key}: forbidden future/hidden policy input"
                    ));
                }
                reject_forbidden_policy_fields(child, &format!("{path}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_forbidden_policy_fields(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn evaluate_scenario(
    scenario: &Value,
    path: &str,
    service_order: &[String],
    demands: &[u64],
    service_power: u64,
    production_energy: u64,
    production_cap: u64,
) -> Result<Value, String> {
    let id = string_at(scenario, path, "id")?;
    let power_budgets = array_at(scenario, path, "power_budget_by_step")?;
    if power_budgets.is_empty() {
        return Err(format!("{path}.power_budget_by_step: must not be empty"));
    }
    let mut budgets = Vec::with_capacity(power_budgets.len());
    for (step, value) in power_budgets.iter().enumerate() {
        budgets.push(u64_value(
            value,
            &format!("{path}.power_budget_by_step[{step}]"),
        )?);
    }

    let pre = field(scenario, path, "preconditions")?;
    let machine = precondition(pre, &format!("{path}.preconditions"), "machine_operational")?;
    let grade = precondition(
        pre,
        &format!("{path}.preconditions"),
        "material_grade_matches",
    )?;
    let calibration = precondition(pre, &format!("{path}.preconditions"), "calibration_valid")?;
    let inspection = precondition(
        pre,
        &format!("{path}.preconditions"),
        "inspection_route_valid",
    )?;
    let authorization = precondition(
        pre,
        &format!("{path}.preconditions"),
        "human_authorization_present",
    )?;
    let controller_step = u64_at(
        pre,
        &format!("{path}.preconditions"),
        "controller_available_from_step",
    )?;
    let gate_states = [
        ("machine_operational", machine),
        ("material_grade_matches", grade),
        ("calibration_valid", calibration),
        ("inspection_route_valid", inspection),
        ("human_authorization_present", authorization),
    ];

    let expected = field(scenario, path, "expected")?;
    let expected_obj = expected
        .as_object()
        .ok_or_else(|| format!("{path}.expected: expected object"))?;
    for required in [
        "conforming_output_by_step",
        "total_conforming_output",
        "blocked_production_steps",
    ] {
        if !expected_obj.contains_key(required) {
            return Err(format!(
                "{path}.expected.{required}: missing required expected field"
            ));
        }
    }
    if !expected_obj.contains_key("service_delivery_by_step")
        && !expected_obj.contains_key("service_floors_met_by_step")
    {
        return Err(format!(
            "{path}.expected: require service_delivery_by_step or service_floors_met_by_step"
        ));
    }
    for key in expected_obj.keys() {
        if !SUPPORTED_EXPECTED_FIELDS.contains(&key.as_str()) {
            return Err(format!("{path}.expected.{key}: unsupported expected field"));
        }
    }

    let mut service_rows = Vec::with_capacity(budgets.len());
    let mut floor_rows = Vec::with_capacity(budgets.len());
    let mut outputs = Vec::with_capacity(budgets.len());
    let mut blocks = Vec::new();
    let mut total_output: u64 = 0;
    let mut total_food_unmet: u64 = 0;
    let mut first_all_floors = None;
    let mut first_output = None;

    for (step_index, budget) in budgets.iter().enumerate() {
        let mut remaining = *budget;
        let mut delivery = Map::new();
        let mut all_met = true;
        let mut food_unmet_this_step = 0_u64;

        for (service_index, service) in service_order.iter().enumerate() {
            let demand = demands[service_index];
            let delivered = demand.min(remaining / service_power);
            let used = delivered
                .checked_mul(service_power)
                .ok_or_else(|| format!("{path}.step[{step_index}]: service energy overflow"))?;
            remaining = remaining
                .checked_sub(used)
                .ok_or_else(|| format!("{path}.step[{step_index}]: service energy underflow"))?;
            if delivered < demand {
                all_met = false;
            }
            if service == "food_preservation" {
                food_unmet_this_step = demand - delivered;
            }
            delivery.insert(service.clone(), json!(delivered));
        }
        total_food_unmet = total_food_unmet
            .checked_add(food_unmet_this_step)
            .ok_or_else(|| format!("{path}: unmet food total overflow"))?;
        if all_met && first_all_floors.is_none() {
            first_all_floors = Some(step_index as u64);
        }

        let mut blocked_reason = if !all_met {
            Some("essential_service_floors_not_met".to_owned())
        } else {
            None
        };
        if blocked_reason.is_none() {
            for (gate, state) in gate_states {
                if state != Precondition::True {
                    blocked_reason = Some(format!("{gate}={}", state.label()));
                    break;
                }
            }
        }
        if blocked_reason.is_none() && (step_index as u64) < controller_step {
            blocked_reason = Some("controller_not_available".to_owned());
        }

        let mut produced = 0_u64;
        if blocked_reason.is_none() {
            produced = production_cap.min(remaining / production_energy);
            if produced == 0 {
                blocked_reason = Some("insufficient_remaining_power".to_owned());
            } else {
                // Since this is the final allocation for the step, output is computed
                // from remaining / production_energy; no later consumer needs the remainder.
                // The division bound ensures produced * production_energy <= remaining.
            }
        }
        if produced > 0 && first_output.is_none() {
            first_output = Some(step_index as u64);
        }
        total_output = total_output
            .checked_add(produced)
            .ok_or_else(|| format!("{path}: total output overflow"))?;
        if let Some(reason) = blocked_reason {
            blocks.push(json!({"step": step_index, "reason": reason}));
        }
        service_rows.push(Value::Object(delivery));
        floor_rows.push(json!(all_met));
        outputs.push(json!(produced));
    }

    let computed = json!({
        "service_delivery_by_step": service_rows,
        "service_floors_met_by_step": floor_rows,
        "conforming_output_by_step": outputs,
        "total_conforming_output": total_output,
        "first_step_all_service_floors_met": first_all_floors,
        "first_conforming_output_step": first_output,
        "food_preservation_unmet_units": total_food_unmet,
        "blocked_production_steps": blocks,
    });

    compare_expected(expected, &computed, &format!("{path}.expected"))?;
    Ok(json!({
        "scenario_id": id,
        "result": computed,
        "expected_matches": true
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Precondition {
    True,
    False,
    Unknown,
}

impl Precondition {
    fn label(self) -> &'static str {
        match self {
            Self::True => "true",
            Self::False => "false",
            Self::Unknown => "unknown",
        }
    }
}

fn precondition(value: &Value, path: &str, key: &str) -> Result<Precondition, String> {
    let item = field(value, path, key)?;
    match item {
        Value::Bool(true) => Ok(Precondition::True),
        Value::Bool(false) => Ok(Precondition::False),
        Value::String(state) if state == "unknown" => Ok(Precondition::Unknown),
        _ => Err(format!(
            "{path}.{key}: expected boolean or the literal string unknown"
        )),
    }
}

fn compare_expected(expected: &Value, computed: &Value, path: &str) -> Result<(), String> {
    let expected_obj = expected
        .as_object()
        .ok_or_else(|| format!("{path}: expected object"))?;
    for (key, wanted) in expected_obj {
        let actual = computed
            .get(key)
            .ok_or_else(|| format!("{path}.{key}: evaluator did not produce this field"))?;
        if key == "blocked_production_steps" {
            compare_blocked_steps(wanted, actual, &format!("{path}.{key}"))?;
        } else if wanted != actual {
            return Err(format!(
                "{path}.{key}: expected {}, actual {}",
                compact_json(wanted),
                compact_json(actual)
            ));
        }
    }
    Ok(())
}

fn compare_blocked_steps(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    let expected_steps = expected
        .as_array()
        .ok_or_else(|| format!("{path}: expected array"))?;
    let actual_steps = actual
        .as_array()
        .ok_or_else(|| format!("{path}: evaluator result is not an array"))?;
    if expected_steps.len() != actual_steps.len() {
        return Err(format!(
            "{path}: expected {} blocked steps, actual {}",
            expected_steps.len(),
            actual_steps.len()
        ));
    }
    for (index, (wanted, got)) in expected_steps.iter().zip(actual_steps).enumerate() {
        let got_obj = got
            .as_object()
            .ok_or_else(|| format!("{path}[{index}]: evaluator result is not an object"))?;
        if let Some(step) = wanted.as_u64() {
            let actual_step = got_obj
                .get("step")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{path}[{index}].step: missing evaluator step"))?;
            if step != actual_step {
                return Err(format!(
                    "{path}[{index}].step: expected {step}, actual {actual_step}"
                ));
            }
        } else if let Some(want_obj) = wanted.as_object() {
            let want_step = want_obj
                .get("step")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{path}[{index}].step: expected integer"))?;
            let got_step = got_obj
                .get("step")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{path}[{index}].step: missing evaluator step"))?;
            if want_step != got_step {
                return Err(format!(
                    "{path}[{index}].step: expected {want_step}, actual {got_step}"
                ));
            }
            let want_reason = want_obj
                .get("reason")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{path}[{index}].reason: expected string"))?;
            let got_reason = got_obj
                .get("reason")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{path}[{index}].reason: missing evaluator reason"))?;
            if want_reason != got_reason {
                return Err(format!(
                    "{path}[{index}].reason: expected {want_reason}, actual {got_reason}"
                ));
            }
        } else {
            return Err(format!(
                "{path}[{index}]: expected an integer step or a step/reason object"
            ));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, path: &str, key: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path}: expected object"))?
        .get(key)
        .ok_or_else(|| format!("{path}.{key}: missing required field"))
}

fn string_at<'a>(value: &'a Value, path: &str, key: &str) -> Result<&'a str, String> {
    field(value, path, key)?
        .as_str()
        .ok_or_else(|| format!("{path}.{key}: expected string"))
}

fn array_at<'a>(value: &'a Value, path: &str, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, path, key)?
        .as_array()
        .ok_or_else(|| format!("{path}.{key}: expected array"))
}

fn u64_at(value: &Value, path: &str, key: &str) -> Result<u64, String> {
    u64_value(field(value, path, key)?, &format!("{path}.{key}"))
}

fn u64_value(value: &Value, path: &str) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| format!("{path}: expected non-negative integer"))
}

fn positive_u64(value: &Value, path: &str) -> Result<u64, String> {
    let result = u64_value(value, path)?;
    if result == 0 {
        return Err(format!("{path}: must be greater than zero"));
    }
    Ok(result)
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unserializable>".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_and_manifest() -> (Value, Value) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture_path = root.join("docs/research/CIV_RESILIENCE_RESTART_FIXTURE_V0_1.json");
        let manifest_path = root.join("docs/research/CIV_RESILIENCE_RESTART_MANIFEST_V0_1.json");
        let fixture: Value =
            serde_json::from_slice(&fs::read(fixture_path).expect("fixture bytes"))
                .expect("fixture JSON");
        let manifest: Value =
            serde_json::from_slice(&fs::read(manifest_path).expect("manifest bytes"))
                .expect("manifest JSON");
        (fixture, manifest)
    }

    #[test]
    fn pinned_fixture_source_and_lock_hashes_match_manifest() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture_bytes =
            fs::read(root.join("docs/research/CIV_RESILIENCE_RESTART_FIXTURE_V0_1.json"))
                .expect("fixture bytes");
        let source_bytes =
            fs::read(root.join("tools/civ-res-ind-oracle/src/main.rs")).expect("source bytes");
        let lock_bytes = fs::read(root.join("Cargo.lock")).expect("lock bytes");
        let manifest_path = root.join("docs/research/CIV_RESILIENCE_RESTART_MANIFEST_V0_1.json");
        let manifest: Value =
            serde_json::from_slice(&fs::read(manifest_path).expect("manifest bytes"))
                .expect("manifest JSON");
        for (key, bytes) in [
            ("fixture_sha256", fixture_bytes),
            ("evaluator_source_sha256", source_bytes),
            ("cargo_lock_sha256", lock_bytes),
        ] {
            verify_sha_field(&manifest, key, &bytes)
                .expect("manifest digest must match exact bytes");
        }
    }

    #[test]
    fn hand_calculated_nominal_golden_uses_no_generic_evaluator() {
        // Independent arithmetic: 6 energy units satisfy 2 water + 1 sanitation
        // + 1 food-preservation unit, leaving 2 for one reference operation.
        let demands = [
            ("water", 2_u64),
            ("sanitation", 1_u64),
            ("food_preservation", 1_u64),
        ];
        let service_energy_per_unit = 1_u64;
        let operation_energy = 2_u64;
        let mut remaining = 6_u64;
        let mut delivered = Vec::new();
        for (_service, demand) in demands {
            let amount = demand.min(remaining / service_energy_per_unit);
            remaining -= amount * service_energy_per_unit;
            delivered.push(amount);
        }
        assert_eq!(delivered, vec![2, 1, 1]);
        assert_eq!(remaining, 2);
        let conforming_units = 1_u64.min(remaining / operation_energy);
        assert_eq!(conforming_units, 1);
        assert_eq!(remaining - conforming_units * operation_energy, 0);
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn frozen_fixture_matches_independent_accounting_rules() {
        let (fixture, manifest) = fixture_and_manifest();
        let digest = string_at(&manifest, "$manifest", "fixture_sha256").expect("digest");
        let report = evaluate_fixture(&fixture, digest, &manifest).expect("reference evaluation");
        assert_eq!(report["status"], "PASS_SYNTHETIC_REFERENCE_ONLY");
        assert_eq!(report["scenario_count"], 6);
        assert_eq!(report["scenario_results"][0]["scenario_id"], "S00_NOMINAL");
        assert_eq!(
            report["scenario_results"][0]["result"]["conforming_output_by_step"],
            json!([1, 1, 1])
        );
        assert_eq!(
            report["scenario_results"][1]["result"]["food_preservation_unmet_units"],
            json!(1)
        );
    }

    #[test]
    fn invalid_calibration_cannot_claim_positive_output() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let s04 = fixture["scenarios"]
            .as_array_mut()
            .expect("scenarios")
            .iter_mut()
            .find(|item| item["id"] == "S04_CALIBRATION_UNKNOWN_OR_INVALID")
            .expect("S04");
        s04["expected"]["conforming_output_by_step"] = json!([1, 1, 1]);
        s04["expected"]["total_conforming_output"] = json!(3);
        let error = evaluate_fixture(&fixture, "test-mutated-fixture", &manifest)
            .expect_err("forged output must fail");
        assert!(error.contains("conforming_output_by_step"), "{error}");
    }

    #[test]
    fn material_grade_mismatch_cannot_claim_positive_output() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let s05 = fixture["scenarios"]
            .as_array_mut()
            .expect("scenarios")
            .iter_mut()
            .find(|item| item["id"] == "S05_MATERIAL_GRADE_MISMATCH")
            .expect("S05");
        s05["expected"]["conforming_output_by_step"] = json!([1, 1, 1]);
        let error = evaluate_fixture(&fixture, "test-mutated-fixture", &manifest)
            .expect_err("forged output must fail");
        assert!(error.contains("conforming_output_by_step"), "{error}");
    }

    #[test]
    fn duplicate_scenario_ids_reject() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let scenarios = fixture["scenarios"].as_array_mut().expect("scenarios");
        let duplicate_id = scenarios[0]["id"].clone();
        scenarios[1]["id"] = duplicate_id;
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("duplicate ID must fail");
        assert!(error.contains("duplicate scenario ID"), "{error}");
    }

    #[test]
    fn missing_expected_field_rejects() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let s00 = fixture["scenarios"]
            .as_array_mut()
            .expect("scenarios")
            .first_mut()
            .expect("S00");
        s00["expected"]
            .as_object_mut()
            .expect("expected object")
            .remove("total_conforming_output");
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("missing expected must fail");
        assert!(error.contains("total_conforming_output"), "{error}");
    }

    #[test]
    fn scenario_list_order_does_not_change_normalized_results() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let original = evaluate_fixture(&fixture, "test-fixture", &manifest).expect("original");
        fixture["scenarios"]
            .as_array_mut()
            .expect("scenarios")
            .reverse();
        let reversed = evaluate_fixture(&fixture, "test-fixture", &manifest).expect("reversed");
        assert_eq!(original["scenario_results"], reversed["scenario_results"]);
    }

    #[test]
    fn service_priority_order_is_semantic_and_must_not_be_silently_changed() {
        let (mut fixture, manifest) = fixture_and_manifest();
        fixture["reference_rules"]["service_allocation_order"] =
            json!(["food_preservation", "water", "sanitation"]);
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("allocation priority changes declared results");
        assert!(
            error.contains("food_preservation_unmet_units")
                || error.contains("service_delivery_by_step")
                || error.contains("service_floors_met_by_step"),
            "priority mutation must alter a derived expected field: {error}"
        );
    }

    #[test]
    fn changed_fixture_bytes_fail_the_pinned_digest_check() {
        let (_, manifest) = fixture_and_manifest();
        let expected = string_at(&manifest, "$manifest", "fixture_sha256").expect("digest");
        let mutated = b"{\"changed\":true}";
        assert_ne!(expected, sha256_hex(mutated));
        let error = verify_sha_field(&manifest, "fixture_sha256", mutated)
            .expect_err("stale digest must fail");
        assert!(error.contains("digest mismatch"), "{error}");
    }

    #[test]
    fn unknown_preconditions_are_blocking_not_favorable_defaults() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let s00 = fixture["scenarios"]
            .as_array_mut()
            .expect("scenarios")
            .first_mut()
            .expect("S00");
        s00["preconditions"]["calibration_valid"] = json!("unknown");
        s00["expected"]["conforming_output_by_step"] = json!([0, 0, 0]);
        s00["expected"]["total_conforming_output"] = json!(0);
        s00["expected"]["blocked_production_steps"] = json!([
            {"step": 0, "reason": "calibration_valid=unknown"},
            {"step": 1, "reason": "calibration_valid=unknown"},
            {"step": 2, "reason": "calibration_valid=unknown"}
        ]);
        let report = evaluate_fixture(&fixture, "test-mutated-fixture", &manifest)
            .expect("unknown calibration is a valid blocked disposition");
        assert_eq!(
            report["scenario_results"][0]["result"]["conforming_output_by_step"],
            json!([0, 0, 0])
        );
    }


    #[test]
    fn criticality_heuristic_recomputes_actions_from_only_current_visible_history() {
        let first = json!({
            "step": 0,
            "observation_history": [
                {"step": 0, "observation": {"power_available": 6, "controller_operational": true}}
            ]
        });
        assert_eq!(
            criticality_heuristic_action(&first).expect("first action"),
            json!({"kind": "preserve_essential_service_floors"})
        );

        let second = json!({
            "step": 1,
            "observation_history": [
                {"step": 0, "observation": {"power_available": 6, "controller_operational": true}},
                {"step": 1, "observation": {"power_available": 6, "service_floors_met": true}}
            ]
        });
        assert_eq!(
            criticality_heuristic_action(&second).expect("second action"),
            json!({"kind": "reserve_repair_stock"})
        );

        let third_without_current_floor_evidence = json!({
            "step": 2,
            "observation_history": [
                {"step": 0, "observation": {"power_available": 6, "controller_operational": true}},
                {"step": 1, "observation": {"power_available": 6, "service_floors_met": true}},
                {"step": 2, "observation": {"power_available": 6, "controller_operational": true}}
            ]
        });
        assert_eq!(
            criticality_heuristic_action(&third_without_current_floor_evidence)
                .expect("unknown current floor disposition"),
            json!({"kind": "request_service_floor_observation"})
        );
    }

    #[test]
    fn reference_policy_rejects_hidden_future_fields_before_deciding() {
        let input = json!({
            "step": 0,
            "observation_history": [
                {"step": 0, "observation": {"power_available": 6}}
            ],
            "future_event_schedule": ["controller-fails-at-step-2"]
        });
        let error = criticality_heuristic_action(&input).expect_err("hidden future is not policy input");
        assert!(error.contains("forbidden future/hidden policy input"), "{error}");
    }

    #[test]
    fn future_blindness_trace_checks_identical_prefix_actions_and_later_divergence() {
        let (fixture, manifest) = fixture_and_manifest();
        let report = evaluate_fixture(&fixture, "test-fixture", &manifest).expect("evaluation");
        let trace = &report["future_blindness_contract"];
        assert_eq!(trace["status"], "RECORDED_TRACE_PREFIX_MATCHED_SYNTHETIC");
        assert_eq!(trace["policy_id"], "criticality-heuristic");
        assert_eq!(trace["policy_version"], "fixture-policy/1");
        assert_eq!(trace["shared_observations_through_step"], 1);
        assert_eq!(trace["matched_prefix_actions"], 2);
        assert_eq!(trace["first_visible_history_divergence_step"], 2);
        assert_eq!(trace["first_action_divergence_step"], 2);
        let receipts = trace["trace_step_receipts"]
            .as_array()
            .expect("step receipts");
        assert_eq!(receipts.len(), 3);
        assert_eq!(
            receipts[0]["policy_input_sha256_left"],
            receipts[0]["policy_input_sha256_right"]
        );
        assert_eq!(
            receipts[0]["action_sha256_left"],
            receipts[0]["action_sha256_right"]
        );
        assert!(
            receipts[0]["policy_input_digest_equal"]
                .as_bool()
                .expect("input digest equality")
        );
        assert!(
            receipts[1]["action_digest_equal"]
                .as_bool()
                .expect("action digest equality")
        );
        assert_ne!(
            receipts[2]["policy_input_sha256_left"],
            receipts[2]["policy_input_sha256_right"]
        );
        assert_ne!(
            receipts[2]["action_sha256_left"],
            receipts[2]["action_sha256_right"]
        );
        assert!(
            trace["claim_ceiling"]
                .as_str()
                .expect("ceiling")
                .contains("does not execute/sandbox a live policy")
        );
    }

    #[test]
    fn future_blindness_rejects_action_divergence_under_identical_visible_history() {
        let (mut fixture, manifest) = fixture_and_manifest();
        fixture["causality_pair"]["policy_trace_pair"]["left"]["steps"][1]["action"] =
            json!({"kind": "illegally_different_action"});
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("divergent action with the same policy input must fail");
        assert!(
            error.contains("does not match the visible-input reference policy")
                || error.contains("actions differ despite identical policy-visible histories"),
            "{error}"
        );
    }

    #[test]
    fn future_blindness_rejects_hidden_future_policy_inputs() {
        let (mut fixture, manifest) = fixture_and_manifest();
        fixture["causality_pair"]["policy_trace_pair"]["right"]["steps"][0]["policy_input"]["scenario_id"] =
            json!("S02_COMMON_MODE_OUTAGE");
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("scenario identity must not enter policy inputs");
        assert!(
            error.contains("forbidden future/hidden policy input"),
            "{error}"
        );
    }

    #[test]
    fn future_blindness_rejects_divergent_actions_if_observations_never_diverge() {
        let (mut fixture, manifest) = fixture_and_manifest();
        let left_history =
            fixture["causality_pair"]["policy_trace_pair"]["left"]["steps"][2]["visible_history"]
                .clone();
        fixture["causality_pair"]["policy_trace_pair"]["right"]["steps"][2]["visible_history"] =
            left_history.clone();
        fixture["causality_pair"]["policy_trace_pair"]["right"]["steps"][2]["policy_input"]["observation_history"] =
            left_history;
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("different action under identical observations must fail");
        assert!(
            error.contains("does not match the visible-input reference policy")
                || error.contains("actions differ despite identical policy-visible histories"),
            "{error}"
        );
    }

    #[test]
    fn future_blindness_rejects_mismatched_policy_versions() {
        let (mut fixture, manifest) = fixture_and_manifest();
        fixture["causality_pair"]["policy_trace_pair"]["right"]["policy_version"] =
            json!("fixture-policy/2");
        let error = evaluate_fixture(&fixture, "test-fixture", &manifest)
            .expect_err("paired traces must use one policy version");
        assert!(error.contains("same policy ID and version"), "{error}");
    }
}

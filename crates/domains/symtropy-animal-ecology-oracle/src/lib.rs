// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed structural validation for the frozen ecological-transfer oracle.
//!
//! Contract consistency is not a simulation run and is not biological validation.

use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const ORACLE_SCHEMA_VERSION: &str = "ANIMAL-ECOLOGICAL-TRANSFER-ORACLE-V1";
pub const EXPECTED_CASE_COUNT: usize = 51;

pub const COMMITTED_ORACLE_JSON: &str =
    include_str!("../../../../docs/living-world/ANIMAL_ECOLOGICAL_TRANSFER_ORACLE_V1.json");
pub const COMMITTED_SCHEMA_JSON: &str =
    include_str!("../../../../docs/living-world/ANIMAL_ECOLOGICAL_TRANSFER_ORACLE_V1.schema.json");

pub const PILOT_SCHEMA_VERSION: &str = "ANIMAL-FLOOD-FORAGE-RISK-PILOT-V0";
pub const COMMITTED_PILOT_JSON: &str =
    include_str!("../../../../docs/living-world/ANIMAL_FLOOD_FORAGE_RISK_PILOT_V0.json");
pub const COMMITTED_PILOT_SCHEMA_JSON: &str =
    include_str!("../../../../docs/living-world/ANIMAL_FLOOD_FORAGE_RISK_PILOT_V0.schema.json");

const PILOT_ROOT_KEYS: &[&str] = &[
    "schema_version",
    "status",
    "tracking_issue",
    "scenario_id",
    "independent_unit",
    "note",
    "randomization",
    "horizon",
    "world",
    "refuges",
    "corridors",
    "model_registry",
    "agent",
    "sensory_contract",
    "budget_contract",
    "policy_comparison",
    "primary_metrics",
    "ablations",
    "hard_invariants",
];
const RANDOMIZATION_KEYS: &[&str] = &[
    "seed_family",
    "seeds",
    "paired_exogenous_noise",
    "pairing_scope",
];
const HORIZON_KEYS: &[&str] = &["ticks", "tick_duration_ms"];
const WORLD_KEYS: &[&str] = &[
    "width_cells",
    "height_cells",
    "initial_water_depth_cm",
    "initial_resource_stock_units",
    "resource_replenishment_units_per_tick",
    "flood",
    "resource_shortage",
    "held_out_combination",
];
const FLOOD_KEYS: &[&str] = &[
    "event_id",
    "start_tick",
    "end_tick",
    "water_depth_cm",
    "current_xy",
    "closes_corridor_id",
];
const SHORTAGE_KEYS: &[&str] = &[
    "event_id",
    "start_tick",
    "end_tick",
    "replenishment_units_per_tick",
];
const REFUGE_KEYS: &[&str] = &["id", "x_cell", "y_cell"];
const CORRIDOR_KEYS: &[&str] = &[
    "id",
    "from_refuge",
    "to_refuge",
    "traversable_before_flood",
    "traversable_during_flood",
];
const MODEL_KEYS: &[&str] = &[
    "id",
    "role",
    "reference_history_fit",
    "forage_recovery_lag_ticks",
    "policy_visible_model_id",
    "revealed_before_evaluation",
    "parameter_source",
    "parameters_disclosed_in_public_fixture",
];
const AGENT_KEYS: &[&str] = &[
    "profile_id",
    "biological_claim_scope",
    "initial_energy_units",
    "active_sensing_actions_per_episode",
    "max_policy_actions",
];
const SENSORY_KEYS: &[&str] = &[
    "permitted_observations",
    "denied_inputs",
    "cue_delivery_latency_ticks",
    "renderer_independent",
];
const BUDGET_KEYS: &[&str] = &[
    "simulated_tick_budget",
    "max_policy_actions",
    "active_sensing_actions_per_episode",
    "compute_budget_units",
    "matched_budget_required",
    "failed_selected_episodes_remain_in_denominator",
];
const POLICY_COMPARISON_KEYS: &[&str] = &[
    "qualifying_baselines",
    "candidates",
    "nonqualifying_upper_bounds",
    "exact_policy_identity_required",
    "same_initial_state_required",
    "paired_exogenous_noise_required_within_model",
    "report_distributional_comparison_across_model_structures",
];
const HARD_INVARIANT_KEYS: &[&str] = &[
    "hidden_state_access_allowed",
    "resource_accounting_violation_limit",
    "selected_failures_remain_in_denominator",
    "missing_metric_disposition",
    "synthetic_success_claim_scope",
    "renderer_is_world_authority",
];

const TOP_LEVEL_KEYS: &[&str] = &[
    "schema_version",
    "status",
    "research_snapshot",
    "tracking_issue",
    "note",
    "case_count",
    "independent_unit",
    "required_run_identity",
    "comparison_policy",
    "cases",
];

const CASE_KEYS: &[&str] = &[
    "id",
    "kind",
    "setup",
    "perturbation",
    "expected",
    "fail_if",
    "reason",
];

const EXPECTED_KEYS: &[&str] = &["required_properties"];

const REQUIRED_RUN_IDENTITY: &[&str] = &[
    "source_commit",
    "simulator_version",
    "policy_version",
    "ecology_config_sha256",
    "scenario_manifest_sha256",
    "independent_unit_manifest_sha256",
    "train_split_sha256",
    "validation_split_sha256",
    "test_split_sha256",
    "evaluator_version",
    "seed_family",
    "sensor_action_contract",
    "simulated_horizon",
    "interaction_budget",
    "compute_budget",
    "hardware_runtime",
    "uncertainty_method",
    "preregistered_margins",
];

const REQUIRED_COMPARISON_POLICY: &[&str] = &[
    "matched_budget_is_default",
    "extra_budget_is_cost_tradeoff",
    "missing_metric_is_not_zero",
    "failed_selected_rollout_remains_in_accounting",
    "synthetic_pass_is_not_empirical_validation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub schema_version: String,
    pub case_count: usize,
    pub unique_case_ids: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotValidationReport {
    pub scenario_id: String,
    pub seed_count: usize,
    pub model_count: usize,
}

/// Validate the machine-readable oracle without claiming to execute its scenarios.
pub fn validate_oracle(input: &str) -> Result<ValidationReport, ValidationError> {
    let root: Value = serde_json::from_str(input)
        .map_err(|error| ValidationError::new(format!("oracle JSON is invalid: {error}")))?;
    let root_object = root
        .as_object()
        .ok_or_else(|| ValidationError::new("oracle root must be an object"))?;
    check_exact_keys(root_object, TOP_LEVEL_KEYS, "oracle")?;

    let schema_version = required_string(&root, "schema_version", "oracle")?.to_owned();
    if schema_version != ORACLE_SCHEMA_VERSION {
        return Err(ValidationError::new(format!(
            "schema_version must be {ORACLE_SCHEMA_VERSION}, got {schema_version}"
        )));
    }
    require_exact_string(&root, "status", "synthetic_contract_fixture", "oracle")?;
    let research_snapshot = required_string(&root, "research_snapshot", "oracle")?;
    if !is_valid_iso_date(research_snapshot) {
        return Err(ValidationError::new(
            "oracle.research_snapshot must be a real YYYY-MM-DD calendar date",
        ));
    }
    require_exact_string(
        &root,
        "tracking_issue",
        "https://github.com/Luminous-Dynamics/symtropy/issues/2214",
        "oracle",
    )?;
    require_nonempty_string(&root, "note", "oracle")?;
    require_nonempty_string(&root, "independent_unit", "oracle")?;

    let declared_count = root
        .get("case_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| ValidationError::new("oracle.case_count must be an integer"))?;
    if declared_count != EXPECTED_CASE_COUNT as u64 {
        return Err(ValidationError::new(format!(
            "oracle.case_count must be {EXPECTED_CASE_COUNT}, got {declared_count}"
        )));
    }

    require_unique_nonempty_string_array(
        &root,
        "required_run_identity",
        "oracle",
        REQUIRED_RUN_IDENTITY,
    )?;
    require_unique_nonempty_string_array(
        &root,
        "comparison_policy",
        "oracle",
        REQUIRED_COMPARISON_POLICY,
    )?;

    let cases = root
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("oracle.cases must be an array"))?;
    if cases.len() != EXPECTED_CASE_COUNT {
        return Err(ValidationError::new(format!(
            "oracle.cases must contain {EXPECTED_CASE_COUNT} records, got {}",
            cases.len()
        )));
    }

    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let path = format!("oracle.cases[{index}]");
        let object = case
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path} must be an object")))?;
        check_exact_keys(object, CASE_KEYS, &path)?;

        let id = required_string(case, "id", &path)?;
        if !is_snake_case_id(id) {
            return Err(ValidationError::new(format!(
                "{path}.id must match ^[a-z][a-z0-9_]*$, got {id}"
            )));
        }
        if !ids.insert(id.to_owned()) {
            return Err(ValidationError::new(format!(
                "duplicate oracle case id: {id}"
            )));
        }

        for key in ["kind", "setup", "perturbation", "reason"] {
            require_nonempty_string(case, key, &path)?;
        }
        let expected = case
            .get("expected")
            .ok_or_else(|| ValidationError::new(format!("{path}.expected is required")))?;
        let expected_object = expected
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path}.expected must be an object")))?;
        check_exact_keys(expected_object, EXPECTED_KEYS, &format!("{path}.expected"))?;
        require_unique_nonempty_string_array(
            expected,
            "required_properties",
            &format!("{path}.expected"),
            &[],
        )?;
        require_unique_nonempty_string_array(case, "fail_if", &path, &[])?;
    }

    Ok(ValidationReport {
        schema_version,
        case_count: cases.len(),
        unique_case_ids: ids.len(),
    })
}

/// Validate the committed schema's critical invariants and the oracle against it.
///
/// This is a focused validator for this versioned contract, not a general JSON Schema engine.
pub fn validate_artifacts(
    oracle_json: &str,
    schema_json: &str,
) -> Result<ValidationReport, ValidationError> {
    let report = validate_oracle(oracle_json)?;
    let schema: Value = serde_json::from_str(schema_json)
        .map_err(|error| ValidationError::new(format!("schema JSON is invalid: {error}")))?;

    require_exact_string(
        &schema,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
        "schema",
    )?;
    require_exact_string(
        &schema,
        "$id",
        "https://github.com/Luminous-Dynamics/symtropy/blob/main/docs/living-world/ANIMAL_ECOLOGICAL_TRANSFER_ORACLE_V1.schema.json",
        "schema",
    )?;
    require_exact_string(&schema, "type", "object", "schema")?;
    require_schema_bool(&schema, "additionalProperties", false, "schema")?;
    require_schema_required_fields(&schema, TOP_LEVEL_KEYS, "schema")?;

    let properties = schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("schema.properties is required"))?;
    let property_object = properties
        .as_object()
        .ok_or_else(|| ValidationError::new("schema.properties must be an object"))?;
    check_exact_keys(property_object, TOP_LEVEL_KEYS, "schema.properties")?;

    require_schema_const(properties, "schema_version", ORACLE_SCHEMA_VERSION)?;
    require_schema_const(properties, "status", "synthetic_contract_fixture")?;
    require_schema_const(properties, "case_count", "51")?;
    require_schema_string(properties, "schema_version", "type", "string")?;
    require_schema_string(properties, "status", "type", "string")?;

    for key in ["note", "tracking_issue", "independent_unit"] {
        require_schema_string(properties, key, "type", "string")?;
        let property_schema = properties
            .get(key)
            .ok_or_else(|| ValidationError::new(format!("schema.properties.{key} is required")))?;
        require_schema_integer(property_schema, "minLength", 1)?;
    }
    require_schema_string(properties, "research_snapshot", "type", "string")?;
    require_schema_string(properties, "research_snapshot", "format", "date")?;
    let snapshot_schema = properties
        .get("research_snapshot")
        .ok_or_else(|| ValidationError::new("schema.properties.research_snapshot is required"))?;
    require_schema_integer(snapshot_schema, "minLength", 10)?;
    require_schema_string(properties, "tracking_issue", "format", "uri")?;
    require_schema_string(properties, "case_count", "type", "integer")?;
    require_string_array_schema(
        properties.get("required_run_identity").ok_or_else(|| {
            ValidationError::new("schema.properties.required_run_identity is required")
        })?,
        "schema.properties.required_run_identity",
    )?;
    require_string_array_schema(
        properties.get("comparison_policy").ok_or_else(|| {
            ValidationError::new("schema.properties.comparison_policy is required")
        })?,
        "schema.properties.comparison_policy",
    )?;

    let cases_schema = properties
        .get("cases")
        .ok_or_else(|| ValidationError::new("schema.properties.cases is required"))?;
    require_exact_string(cases_schema, "type", "array", "schema.properties.cases")?;
    require_schema_integer(cases_schema, "minItems", EXPECTED_CASE_COUNT)?;
    require_schema_integer(cases_schema, "maxItems", EXPECTED_CASE_COUNT)?;

    let item_schema = cases_schema
        .get("items")
        .ok_or_else(|| ValidationError::new("schema.properties.cases.items is required"))?;
    require_exact_string(
        item_schema,
        "type",
        "object",
        "schema.properties.cases.items",
    )?;
    require_schema_bool(
        item_schema,
        "additionalProperties",
        false,
        "schema.properties.cases.items",
    )?;
    require_schema_required_fields(item_schema, CASE_KEYS, "schema.properties.cases.items")?;

    let item_properties = item_schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("schema cases.items.properties is required"))?;
    let item_property_object = item_properties
        .as_object()
        .ok_or_else(|| ValidationError::new("schema cases.items.properties must be an object"))?;
    check_exact_keys(
        item_property_object,
        CASE_KEYS,
        "schema cases.items.properties",
    )?;

    require_schema_string(item_properties, "id", "type", "string")?;
    require_schema_string(item_properties, "id", "pattern", "^[a-z][a-z0-9_]*$")?;
    for key in ["kind", "setup", "perturbation", "reason"] {
        require_schema_string(item_properties, key, "type", "string")?;
        let property_schema = item_properties.get(key).ok_or_else(|| {
            ValidationError::new(format!("schema case property {key} is required"))
        })?;
        require_schema_integer(property_schema, "minLength", 1)?;
    }

    let expected_schema = item_properties
        .get("expected")
        .ok_or_else(|| ValidationError::new("schema case property expected is required"))?;
    require_exact_string(
        expected_schema,
        "type",
        "object",
        "schema case property expected",
    )?;
    require_schema_bool(
        expected_schema,
        "additionalProperties",
        false,
        "schema case property expected",
    )?;
    require_schema_required_fields(
        expected_schema,
        EXPECTED_KEYS,
        "schema case property expected",
    )?;
    let expected_properties = expected_schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("schema expected.properties is required"))?;
    let expected_property_object = expected_properties
        .as_object()
        .ok_or_else(|| ValidationError::new("schema expected.properties must be an object"))?;
    check_exact_keys(
        expected_property_object,
        EXPECTED_KEYS,
        "schema expected.properties",
    )?;
    require_string_array_schema(
        expected_properties
            .get("required_properties")
            .ok_or_else(|| ValidationError::new("schema required_properties is required"))?,
        "schema expected.required_properties",
    )?;
    require_string_array_schema(
        item_properties
            .get("fail_if")
            .ok_or_else(|| ValidationError::new("schema case property fail_if is required"))?,
        "schema case.fail_if",
    )?;

    Ok(report)
}

fn require_schema_bool(
    value: &Value,
    key: &str,
    expected: bool,
    path: &str,
) -> Result<(), ValidationError> {
    let actual = value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be boolean")))?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{path}.{key} must be {expected}"
        )));
    }
    Ok(())
}

fn require_schema_required_fields(
    value: &Value,
    expected: &[&str],
    path: &str,
) -> Result<(), ValidationError> {
    let array = value
        .get("required")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new(format!("{path}.required must be an array")))?;
    let mut actual = BTreeSet::new();
    for item in array {
        let name = item
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                ValidationError::new(format!("{path}.required entries must be nonempty strings"))
            })?;
        if !actual.insert(name) {
            return Err(ValidationError::new(format!(
                "{path}.required contains duplicate entry {name}"
            )));
        }
    }
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected_set {
        return Err(ValidationError::new(format!(
            "{path}.required does not match the frozen required-field set"
        )));
    }
    Ok(())
}

fn require_string_array_schema(value: &Value, path: &str) -> Result<(), ValidationError> {
    require_exact_string(value, "type", "array", path)?;
    require_schema_integer(value, "minItems", 1)?;
    require_schema_bool(value, "uniqueItems", true, path)?;
    let items = value
        .get("items")
        .ok_or_else(|| ValidationError::new(format!("{path}.items is required")))?;
    require_exact_string(items, "type", "string", &format!("{path}.items"))?;
    require_schema_integer(items, "minLength", 1)?;
    Ok(())
}

fn is_valid_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let year = match date[..4].parse::<u32>() {
        Ok(value) if value > 0 => value,
        _ => return false,
    };
    let month = match date[5..7].parse::<u32>() {
        Ok(value) if (1..=12).contains(&value) => value,
        _ => return false,
    };
    let day = match date[8..].parse::<u32>() {
        Ok(value) => value,
        _ => return false,
    };
    let leap = year.is_multiple_of(4)
        && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let month_days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=month_days).contains(&day)
}


/// Validate the committed synthetic flood–forage–risk pilot contract.
///
/// This checks scenario-manifest semantics and schema identity. It does not run
/// the simulated world or establish empirical ecology.
pub fn validate_pilot_manifest(
    input: &str,
    schema_input: &str,
) -> Result<PilotValidationReport, ValidationError> {
    let root: Value = serde_json::from_str(input)
        .map_err(|error| ValidationError::new(format!("pilot JSON is invalid: {error}")))?;
    let root_object = root
        .as_object()
        .ok_or_else(|| ValidationError::new("pilot root must be an object"))?;
    check_exact_keys(root_object, PILOT_ROOT_KEYS, "pilot")?;
    require_exact_string(&root, "schema_version", PILOT_SCHEMA_VERSION, "pilot")?;
    require_exact_string(&root, "status", "synthetic_scenario_manifest", "pilot")?;
    require_exact_string(
        &root,
        "tracking_issue",
        "https://github.com/Luminous-Dynamics/symtropy/issues/2214",
        "pilot",
    )?;
    require_exact_string(
        &root,
        "scenario_id",
        "flood_forage_risk_corridor_v0",
        "pilot",
    )?;
    require_exact_string(&root, "independent_unit", "episode_seed_pair", "pilot")?;
    require_nonempty_string(&root, "note", "pilot")?;

    let randomization = object_at(&root, "randomization", "pilot")?;
    check_exact_keys(
        randomization.as_object().ok_or_else(|| {
            ValidationError::new("pilot.randomization must be an object")
        })?,
        RANDOMIZATION_KEYS,
        "pilot.randomization",
    )?;
    require_nonempty_string(randomization, "seed_family", "pilot.randomization")?;
    require_nonempty_string(randomization, "pairing_scope", "pilot.randomization")?;
    require_exact_bool(
        randomization,
        "paired_exogenous_noise",
        true,
        "pilot.randomization",
    )?;
    let seeds = randomization
        .get("seeds")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("pilot.randomization.seeds must be an array"))?;
    if seeds.len() < 10 {
        return Err(ValidationError::new(
            "pilot.randomization.seeds must contain at least 10 seeds",
        ));
    }
    let mut unique_seeds = BTreeSet::new();
    for (index, seed) in seeds.iter().enumerate() {
        let seed = seed.as_u64().ok_or_else(|| {
            ValidationError::new(format!(
                "pilot.randomization.seeds[{index}] must be a nonnegative integer"
            ))
        })?;
        if !unique_seeds.insert(seed) {
            return Err(ValidationError::new(format!(
                "pilot.randomization.seeds contains duplicate seed {seed}"
            )));
        }
    }

    let horizon = object_at(&root, "horizon", "pilot")?;
    check_exact_keys(
        horizon
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.horizon must be an object"))?,
        HORIZON_KEYS,
        "pilot.horizon",
    )?;
    let horizon_ticks = required_u64(horizon, "ticks", "pilot.horizon")?;
    if horizon_ticks != 1200 {
        return Err(ValidationError::new("pilot.horizon.ticks must be 1200"));
    }
    if required_u64(horizon, "tick_duration_ms", "pilot.horizon")? == 0 {
        return Err(ValidationError::new(
            "pilot.horizon.tick_duration_ms must be positive",
        ));
    }

    let world = object_at(&root, "world", "pilot")?;
    check_exact_keys(
        world
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.world must be an object"))?,
        WORLD_KEYS,
        "pilot.world",
    )?;
    let width = required_u64(world, "width_cells", "pilot.world")?;
    let height = required_u64(world, "height_cells", "pilot.world")?;
    if width == 0 || height == 0 {
        return Err(ValidationError::new(
            "pilot.world dimensions must be positive",
        ));
    }
    require_number_min(world, "initial_water_depth_cm", 0.0, "pilot.world")?;
    require_number_min(world, "initial_resource_stock_units", 0.0, "pilot.world")?;
    require_number_min(world, "resource_replenishment_units_per_tick", 0.0, "pilot.world")?;
    require_exact_bool(world, "held_out_combination", true, "pilot.world")?;

    let flood = object_at(world, "flood", "pilot.world")?;
    check_exact_keys(
        flood
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.world.flood must be an object"))?,
        FLOOD_KEYS,
        "pilot.world.flood",
    )?;
    require_nonempty_string(flood, "event_id", "pilot.world.flood")?;
    require_nonempty_string(flood, "closes_corridor_id", "pilot.world.flood")?;
    require_number_min(flood, "water_depth_cm", 0.0, "pilot.world.flood")?;
    let flood_start = required_u64(flood, "start_tick", "pilot.world.flood")?;
    let flood_end = required_u64(flood, "end_tick", "pilot.world.flood")?;
    if flood_start >= flood_end || flood_end > horizon_ticks {
        return Err(ValidationError::new(
            "pilot flood interval must be ordered and within the simulation horizon",
        ));
    }
    let current = flood
        .get("current_xy")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("pilot.world.flood.current_xy must be an array"))?;
    if current.len() != 2 || current.iter().any(|value| !value.is_number()) {
        return Err(ValidationError::new(
            "pilot.world.flood.current_xy must contain exactly two numeric components",
        ));
    }

    let shortage = object_at(world, "resource_shortage", "pilot.world")?;
    check_exact_keys(
        shortage
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.world.resource_shortage must be an object"))?,
        SHORTAGE_KEYS,
        "pilot.world.resource_shortage",
    )?;
    require_nonempty_string(shortage, "event_id", "pilot.world.resource_shortage")?;
    let shortage_start = required_u64(shortage, "start_tick", "pilot.world.resource_shortage")?;
    let shortage_end = required_u64(shortage, "end_tick", "pilot.world.resource_shortage")?;
    require_number_min(
        shortage,
        "replenishment_units_per_tick",
        0.0,
        "pilot.world.resource_shortage",
    )?;
    if shortage_start >= shortage_end || shortage_end > horizon_ticks {
        return Err(ValidationError::new(
            "pilot resource-shortage interval must be ordered and within the simulation horizon",
        ));
    }
    if flood_start.max(shortage_start) >= flood_end.min(shortage_end) {
        return Err(ValidationError::new(
            "pilot flood and resource-shortage intervals must overlap to form the held-out combination",
        ));
    }

    let refuges = root
        .get("refuges")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("pilot.refuges must be an array"))?;
    if refuges.len() != 2 {
        return Err(ValidationError::new("pilot.refuges must contain exactly two refuges"));
    }
    let mut refuge_ids = BTreeSet::new();
    for (index, refuge) in refuges.iter().enumerate() {
        let path = format!("pilot.refuges[{index}]");
        let object = refuge
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path} must be an object")))?;
        check_exact_keys(object, REFUGE_KEYS, &path)?;
        let id = required_string(refuge, "id", &path)?;
        if !refuge_ids.insert(id.to_owned()) {
            return Err(ValidationError::new(format!("duplicate refuge ID: {id}")));
        }
        let x = required_u64(refuge, "x_cell", &path)?;
        let y = required_u64(refuge, "y_cell", &path)?;
        if x >= width || y >= height {
            return Err(ValidationError::new(format!(
                "{path} lies outside the declared world dimensions"
            )));
        }
    }

    let corridors = root
        .get("corridors")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| ValidationError::new("pilot.corridors must be a nonempty array"))?;
    let mut corridor_ids = BTreeSet::new();
    let mut flood_closes_declared_corridor = false;
    for (index, corridor) in corridors.iter().enumerate() {
        let path = format!("pilot.corridors[{index}]");
        let object = corridor
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path} must be an object")))?;
        check_exact_keys(object, CORRIDOR_KEYS, &path)?;
        let id = required_string(corridor, "id", &path)?;
        if !corridor_ids.insert(id.to_owned()) {
            return Err(ValidationError::new(format!("duplicate corridor ID: {id}")));
        }
        let from = required_string(corridor, "from_refuge", &path)?;
        let to = required_string(corridor, "to_refuge", &path)?;
        if !refuge_ids.contains(from) || !refuge_ids.contains(to) || from == to {
            return Err(ValidationError::new(format!(
                "{path} must connect two distinct declared refuges"
            )));
        }
        let before = required_bool(corridor, "traversable_before_flood", &path)?;
        let during = required_bool(corridor, "traversable_during_flood", &path)?;
        if id == required_string(flood, "closes_corridor_id", "pilot.world.flood")? {
            flood_closes_declared_corridor = before && !during;
        }
    }
    if !flood_closes_declared_corridor {
        return Err(ValidationError::new(
            "flood must close a corridor that is traversable before but not during the flood",
        ));
    }

    let models = root
        .get("model_registry")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("pilot.model_registry must be an array"))?;
    if models.len() != 3 {
        return Err(ValidationError::new(
            "pilot.model_registry must contain two candidate structures and one blind challenger",
        ));
    }
    let mut model_ids = BTreeSet::new();
    let mut lags = BTreeSet::new();
    let mut candidate_count = 0;
    let mut challenger_count = 0;
    for (index, model) in models.iter().enumerate() {
        let path = format!("pilot.model_registry[{index}]");
        let object = model
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path} must be an object")))?;
        check_exact_keys(object, MODEL_KEYS, &path)?;
        let id = required_string(model, "id", &path)?;
        if !model_ids.insert(id.to_owned()) {
            return Err(ValidationError::new(format!("duplicate model ID: {id}")));
        }
        require_exact_string(model, "reference_history_fit", "observationally_compatible", &path)?;
        require_exact_bool(model, "policy_visible_model_id", false, &path)?;
        let disclosed = required_bool(model, "parameters_disclosed_in_public_fixture", &path)?;
        let parameter_source = required_string(model, "parameter_source", &path)?;
        let lag_value = model
            .get("forage_recovery_lag_ticks")
            .ok_or_else(|| ValidationError::new(format!("{path}.forage_recovery_lag_ticks is required")))?;
        let revealed = required_bool(model, "revealed_before_evaluation", &path)?;
        match required_string(model, "role", &path)? {
            "candidate_structure" => {
                candidate_count += 1;
                let lag = lag_value.as_u64().ok_or_else(|| {
                    ValidationError::new(format!("{path}.forage_recovery_lag_ticks must be a positive integer for candidate structures"))
                })?;
                if lag == 0 || !lags.insert(lag) {
                    return Err(ValidationError::new(
                        "candidate structures must have distinct positive forage-recovery lags",
                    ));
                }
                if !revealed {
                    return Err(ValidationError::new(
                        "registered candidate structures must be visible to the evaluator before evaluation",
                    ));
                }
                if !disclosed || parameter_source != "public_candidate_manifest" {
                    return Err(ValidationError::new(
                        "candidate structure parameters must match the public candidate manifest",
                    ));
                }
            }
            "blind_challenger" => {
                challenger_count += 1;
                if revealed {
                    return Err(ValidationError::new(
                        "blind challenger must remain unrevealed before evaluation",
                    ));
                }
                if !lag_value.is_null()
                    || disclosed
                    || parameter_source != "sealed_evaluator_manifest"
                {
                    return Err(ValidationError::new(
                        "blind challenger parameters must remain absent from the public fixture and come from a sealed evaluator manifest",
                    ));
                }
            }
            role => {
                return Err(ValidationError::new(format!(
                    "{path}.role is unsupported: {role}"
                )));
            }
        }
    }
    if candidate_count != 2 || challenger_count != 1 {
        return Err(ValidationError::new(
            "pilot must register exactly two candidate structures and one blind challenger",
        ));
    }

    let agent = object_at(&root, "agent", "pilot")?;
    check_exact_keys(
        agent
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.agent must be an object"))?,
        AGENT_KEYS,
        "pilot.agent",
    )?;
    require_exact_string(agent, "biological_claim_scope", "synthetic_only", "pilot.agent")?;
    require_nonempty_string(agent, "profile_id", "pilot.agent")?;
    require_number_min(agent, "initial_energy_units", 0.0, "pilot.agent")?;

    let sensory = object_at(&root, "sensory_contract", "pilot")?;
    check_exact_keys(
        sensory
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.sensory_contract must be an object"))?,
        SENSORY_KEYS,
        "pilot.sensory_contract",
    )?;
    let required_observations = [
        "local_flow_estimate",
        "local_water_depth",
        "local_resource_cue",
        "local_predator_cue",
        "refuge_visibility",
    ];
    let required_denied_inputs = [
        "global_current_vector_field",
        "future_flood_state",
        "intervention_label",
        "hidden_model_id",
        "exact_global_resource_coordinates",
    ];
    require_unique_nonempty_string_array(
        sensory,
        "permitted_observations",
        "pilot.sensory_contract",
        &required_observations,
    )?;
    require_unique_nonempty_string_array(
        sensory,
        "denied_inputs",
        "pilot.sensory_contract",
        &required_denied_inputs,
    )?;
    require_exact_bool(sensory, "renderer_independent", true, "pilot.sensory_contract")?;
    for permitted in sensory["permitted_observations"].as_array().ok_or_else(|| {
        ValidationError::new("pilot permitted_observations must be an array")
    })? {
        if let Some(name) = permitted.as_str()
            && required_denied_inputs.contains(&name)
        {
            return Err(ValidationError::new(format!(
                "sensor input {name} is both permitted and denied"
            )));
        }
    }

    let budget = object_at(&root, "budget_contract", "pilot")?;
    check_exact_keys(
        budget
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.budget_contract must be an object"))?,
        BUDGET_KEYS,
        "pilot.budget_contract",
    )?;
    if required_u64(budget, "simulated_tick_budget", "pilot.budget_contract")? != horizon_ticks
        || required_u64(budget, "max_policy_actions", "pilot.budget_contract")?
            != required_u64(agent, "max_policy_actions", "pilot.agent")?
        || required_u64(
            budget,
            "active_sensing_actions_per_episode",
            "pilot.budget_contract",
        )? != required_u64(
            agent,
            "active_sensing_actions_per_episode",
            "pilot.agent",
        )?
        || required_u64(budget, "compute_budget_units", "pilot.budget_contract")? == 0
    {
        return Err(ValidationError::new(
            "pilot policy/world budgets must be positive and internally consistent",
        ));
    }
    require_exact_bool(budget, "matched_budget_required", true, "pilot.budget_contract")?;
    require_exact_bool(
        budget,
        "failed_selected_episodes_remain_in_denominator",
        true,
        "pilot.budget_contract",
    )?;

    let comparison = object_at(&root, "policy_comparison", "pilot")?;
    check_exact_keys(
        comparison
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.policy_comparison must be an object"))?,
        POLICY_COMPARISON_KEYS,
        "pilot.policy_comparison",
    )?;
    require_unique_nonempty_string_array(
        comparison,
        "qualifying_baselines",
        "pilot.policy_comparison",
        &["reactive_heuristic"],
    )?;
    require_unique_nonempty_string_array(
        comparison,
        "candidates",
        "pilot.policy_comparison",
        &["adaptive_bounded_sensing"],
    )?;
    require_unique_nonempty_string_array(
        comparison,
        "nonqualifying_upper_bounds",
        "pilot.policy_comparison",
        &["privileged_planner_upper_bound"],
    )?;
    require_exact_bool(comparison, "exact_policy_identity_required", true, "pilot.policy_comparison")?;
    require_exact_bool(comparison, "same_initial_state_required", true, "pilot.policy_comparison")?;
    require_exact_bool(
        comparison,
        "paired_exogenous_noise_required_within_model",
        true,
        "pilot.policy_comparison",
    )?;
    require_exact_bool(
        comparison,
        "report_distributional_comparison_across_model_structures",
        true,
        "pilot.policy_comparison",
    )?;
    let baseline_set = string_array_set(comparison, "qualifying_baselines", "pilot.policy_comparison")?;
    let candidate_set = string_array_set(comparison, "candidates", "pilot.policy_comparison")?;
    let upper_bound_set = string_array_set(
        comparison,
        "nonqualifying_upper_bounds",
        "pilot.policy_comparison",
    )?;
    if !baseline_set.is_disjoint(&candidate_set)
        || !baseline_set.is_disjoint(&upper_bound_set)
        || !candidate_set.is_disjoint(&upper_bound_set)
    {
        return Err(ValidationError::new(
            "pilot qualifying baselines, candidates, and nonqualifying upper bounds must be disjoint",
        ));
    }

    let metrics = root
        .get("primary_metrics")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("pilot.primary_metrics must be an array"))?;
    require_unique_nonempty_string_array(&root, "primary_metrics", "pilot", &[
        "route_feasibility",
        "normalized_decision_regret",
        "integrated_performance_deficit",
        "sensing_cost",
        "calibration_or_abstention_quality",
        "resource_accounting_residual",
        "per_individual_outcome_distribution",
        "receipt_completeness",
    ])?;
    if metrics.len() < 8 {
        return Err(ValidationError::new(
            "pilot.primary_metrics must contain at least eight distinct measures",
        ));
    }
    require_unique_nonempty_string_array(
        &root,
        "ablations",
        "pilot",
        &[
            "disable_active_sensing",
            "remove_flood_preserve_other_random_streams",
            "remove_resource_shortage_preserve_flood",
            "swap_observationally_compatible_model_structures",
            "withhold_future_and_intervention_labels",
            "change_rendering_without_changing_world_or_authorized_observations",
            "compare_larger_compute_budget_as_cost_tradeoff",
        ],
    )?;

    let invariants = object_at(&root, "hard_invariants", "pilot")?;
    check_exact_keys(
        invariants
            .as_object()
            .ok_or_else(|| ValidationError::new("pilot.hard_invariants must be an object"))?,
        HARD_INVARIANT_KEYS,
        "pilot.hard_invariants",
    )?;
    require_exact_bool(invariants, "hidden_state_access_allowed", false, "pilot.hard_invariants")?;
    require_exact_bool(
        invariants,
        "selected_failures_remain_in_denominator",
        true,
        "pilot.hard_invariants",
    )?;
    require_exact_bool(invariants, "renderer_is_world_authority", false, "pilot.hard_invariants")?;
    if required_u64(invariants, "resource_accounting_violation_limit", "pilot.hard_invariants")? != 0 {
        return Err(ValidationError::new(
            "pilot hard resource-accounting violation limit must be zero",
        ));
    }
    require_exact_string(
        invariants,
        "missing_metric_disposition",
        "incomplete_or_not_assessed",
        "pilot.hard_invariants",
    )?;
    require_exact_string(
        invariants,
        "synthetic_success_claim_scope",
        "contract_consistency_only",
        "pilot.hard_invariants",
    )?;

    let schema: Value = serde_json::from_str(schema_input)
        .map_err(|error| ValidationError::new(format!("pilot schema JSON is invalid: {error}")))?;
    require_exact_string(
        &schema,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
        "pilot schema",
    )?;
    require_exact_string(
        &schema,
        "$id",
        "https://github.com/Luminous-Dynamics/symtropy/blob/main/docs/living-world/ANIMAL_FLOOD_FORAGE_RISK_PILOT_V0.schema.json",
        "pilot schema",
    )?;
    require_exact_string(&schema, "type", "object", "pilot schema")?;
    require_schema_bool(&schema, "additionalProperties", false, "pilot schema")?;
    require_schema_required_fields(&schema, PILOT_ROOT_KEYS, "pilot schema")?;
    let schema_properties = schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("pilot schema.properties is required"))?;
    let schema_property_object = schema_properties
        .as_object()
        .ok_or_else(|| ValidationError::new("pilot schema.properties must be an object"))?;
    check_exact_keys(schema_property_object, PILOT_ROOT_KEYS, "pilot schema.properties")?;
    require_schema_const(schema_properties, "schema_version", PILOT_SCHEMA_VERSION)?;
    require_schema_const(schema_properties, "status", "synthetic_scenario_manifest")?;
    require_schema_const(
        schema_properties,
        "tracking_issue",
        "https://github.com/Luminous-Dynamics/symtropy/issues/2214",
    )?;
    require_schema_const(schema_properties, "scenario_id", "flood_forage_risk_corridor_v0")?;
    require_schema_const(schema_properties, "independent_unit", "episode_seed_pair")?;
    require_schema_string(schema_properties, "schema_version", "type", "string")?;
    let randomization_schema = schema_properties
        .get("randomization")
        .ok_or_else(|| ValidationError::new("pilot schema.properties.randomization is required"))?;
    require_exact_string(
        randomization_schema,
        "type",
        "object",
        "pilot schema.properties.randomization",
    )?;
    require_schema_bool(
        randomization_schema,
        "additionalProperties",
        false,
        "pilot schema.properties.randomization",
    )?;
    require_schema_required_fields(
        randomization_schema,
        RANDOMIZATION_KEYS,
        "pilot schema.properties.randomization",
    )?;
    let randomization_properties = randomization_schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("pilot randomization schema properties are required"))?;
    let seed_schema = randomization_properties
        .get("seeds")
        .ok_or_else(|| ValidationError::new("pilot seed schema is required"))?;
    require_schema_integer(seed_schema, "minItems", 10)?;
    require_schema_bool(seed_schema, "uniqueItems", true, "pilot seed schema")?;

    let model_registry_schema = schema_properties
        .get("model_registry")
        .ok_or_else(|| ValidationError::new("pilot schema.properties.model_registry is required"))?;
    require_schema_integer(model_registry_schema, "minItems", 3)?;
    require_schema_integer(model_registry_schema, "maxItems", 3)?;
    let model_items = model_registry_schema
        .get("items")
        .ok_or_else(|| ValidationError::new("pilot model schema items are required"))?;
    require_schema_required_fields(model_items, MODEL_KEYS, "pilot model schema items")?;
    let model_properties = model_items
        .get("properties")
        .ok_or_else(|| ValidationError::new("pilot model schema properties are required"))?;
    let model_property_object = model_properties
        .as_object()
        .ok_or_else(|| ValidationError::new("pilot model schema properties must be an object"))?;
    check_exact_keys(model_property_object, MODEL_KEYS, "pilot model schema properties")?;
    let lag_schema = model_properties
        .get("forage_recovery_lag_ticks")
        .ok_or_else(|| ValidationError::new("pilot recovery lag schema is required"))?;
    let lag_alternatives = lag_schema
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("pilot recovery lag schema must declare oneOf"))?;
    if lag_alternatives.len() != 2
        || !lag_alternatives.iter().any(|value| {
            value.get("type").and_then(Value::as_str) == Some("integer")
                && value.get("minimum").and_then(Value::as_u64) == Some(1)
        })
        || !lag_alternatives.iter().any(|value| {
            value.get("type").and_then(Value::as_str) == Some("null")
        })
    {
        return Err(ValidationError::new(
            "pilot recovery lag schema must allow positive candidate integers and a null sealed challenger value",
        ));
    }

    Ok(PilotValidationReport {
        scenario_id: required_string(&root, "scenario_id", "pilot")?.to_owned(),
        seed_count: seeds.len(),
        model_count: models.len(),
    })
}

fn object_at<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, ValidationError> {
    let object = value
        .get(key)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} is required")))?;
    if !object.is_object() {
        return Err(ValidationError::new(format!("{path}.{key} must be an object")));
    }
    Ok(object)
}

fn required_u64(value: &Value, key: &str, path: &str) -> Result<u64, ValidationError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be a nonnegative integer")))
}

fn required_bool(value: &Value, key: &str, path: &str) -> Result<bool, ValidationError> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be boolean")))
}

fn require_exact_bool(
    value: &Value,
    key: &str,
    expected: bool,
    path: &str,
) -> Result<(), ValidationError> {
    let actual = required_bool(value, key, path)?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{path}.{key} must be {expected}"
        )));
    }
    Ok(())
}

fn require_number_min(
    value: &Value,
    key: &str,
    minimum: f64,
    path: &str,
) -> Result<(), ValidationError> {
    let actual = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be numeric")))?;
    if !actual.is_finite() || actual < minimum {
        return Err(ValidationError::new(format!(
            "{path}.{key} must be finite and >= {minimum}"
        )));
    }
    Ok(())
}

fn check_exact_keys(
    object: &serde_json::Map<String, Value>,
    expected: &[&str],
    path: &str,
) -> Result<(), ValidationError> {
    for key in expected {
        if !object.contains_key(*key) {
            return Err(ValidationError::new(format!(
                "{path} is missing required key {key}"
            )));
        }
    }
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            return Err(ValidationError::new(format!(
                "{path} has unexpected key {key}"
            )));
        }
    }
    Ok(())
}

fn required_string<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
) -> Result<&'a str, ValidationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be a nonempty string")))
}

fn require_nonempty_string(value: &Value, key: &str, path: &str) -> Result<(), ValidationError> {
    required_string(value, key, path).map(|_| ())
}

fn require_exact_string(
    value: &Value,
    key: &str,
    expected: &str,
    path: &str,
) -> Result<(), ValidationError> {
    let actual = required_string(value, key, path)?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{path}.{key} must be {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn string_array_set(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<BTreeSet<String>, ValidationError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be an array")))?;
    let mut values = BTreeSet::new();
    for (index, item) in array.iter().enumerate() {
        let text = item
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| {
                ValidationError::new(format!("{path}.{key}[{index}] must be a nonempty string"))
            })?;
        if !values.insert(text.to_owned()) {
            return Err(ValidationError::new(format!(
                "{path}.{key} contains duplicate entry {text}"
            )));
        }
    }
    Ok(values)
}

fn require_unique_nonempty_string_array(
    value: &Value,
    key: &str,
    path: &str,
    required_members: &[&str],
) -> Result<(), ValidationError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be an array")))?;
    if array.is_empty() {
        return Err(ValidationError::new(format!(
            "{path}.{key} must not be empty"
        )));
    }

    let mut members = BTreeSet::new();
    for (index, item) in array.iter().enumerate() {
        let string = item
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| {
                ValidationError::new(format!("{path}.{key}[{index}] must be a nonempty string"))
            })?;
        if !members.insert(string) {
            return Err(ValidationError::new(format!(
                "{path}.{key} contains duplicate entry {string}"
            )));
        }
    }
    for required in required_members {
        if !members.contains(required) {
            return Err(ValidationError::new(format!(
                "{path}.{key} is missing required entry {required}"
            )));
        }
    }
    Ok(())
}

fn require_schema_const(
    properties: &Value,
    key: &str,
    expected: &str,
) -> Result<(), ValidationError> {
    let actual = properties
        .get(key)
        .and_then(|value| value.get("const"))
        .ok_or_else(|| {
            ValidationError::new(format!("schema.properties.{key}.const is required"))
        })?;
    if actual.as_str() != Some(expected)
        && !(key == "case_count" && actual.as_u64() == Some(EXPECTED_CASE_COUNT as u64))
    {
        return Err(ValidationError::new(format!(
            "schema.properties.{key}.const does not pin the expected contract"
        )));
    }
    Ok(())
}

fn require_schema_integer(
    value: &Value,
    key: &str,
    expected: usize,
) -> Result<(), ValidationError> {
    let actual = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| ValidationError::new(format!("schema {key} must be an integer")))?;
    if actual != expected as u64 {
        return Err(ValidationError::new(format!(
            "schema {key} must be {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn require_schema_string(
    properties: &Value,
    key: &str,
    constraint: &str,
    expected: &str,
) -> Result<(), ValidationError> {
    let actual = properties
        .get(key)
        .and_then(|value| value.get(constraint))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ValidationError::new(format!(
                "schema property {key} is missing string constraint {constraint}"
            ))
        })?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "schema property {key}.{constraint} must be {expected}"
        )));
    }
    Ok(())
}

fn is_snake_case_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn committed_oracle_and_schema_are_structurally_consistent() {
        let report = validate_artifacts(COMMITTED_ORACLE_JSON, COMMITTED_SCHEMA_JSON)
            .expect("committed benchmark artifacts validate");
        assert_eq!(report.schema_version, ORACLE_SCHEMA_VERSION);
        assert_eq!(report.case_count, EXPECTED_CASE_COUNT);
        assert_eq!(report.unique_case_ids, EXPECTED_CASE_COUNT);
    }

    #[test]
    fn committed_pilot_manifest_and_schema_are_consistent() {
        let report = validate_pilot_manifest(COMMITTED_PILOT_JSON, COMMITTED_PILOT_SCHEMA_JSON)
            .expect("committed synthetic pilot manifest validates");
        assert_eq!(report.scenario_id, "flood_forage_risk_corridor_v0");
        assert_eq!(report.seed_count, 10);
        assert_eq!(report.model_count, 3);
    }

    #[test]
    fn pilot_shock_intervals_must_overlap() {
        let mut pilot: Value =
            serde_json::from_str(COMMITTED_PILOT_JSON).expect("pilot fixture parses");
        pilot["world"]["resource_shortage"]["start_tick"] = json!(800);
        let input = serde_json::to_string(&pilot).expect("mutated pilot serializes");
        assert!(validate_pilot_manifest(&input, COMMITTED_PILOT_SCHEMA_JSON).is_err());
    }

    #[test]
    fn blind_challenger_must_not_be_revealed_early() {
        let mut pilot: Value =
            serde_json::from_str(COMMITTED_PILOT_JSON).expect("pilot fixture parses");
        pilot["model_registry"][2]["revealed_before_evaluation"] = json!(true);
        let input = serde_json::to_string(&pilot).expect("mutated pilot serializes");
        assert!(validate_pilot_manifest(&input, COMMITTED_PILOT_SCHEMA_JSON).is_err());
    }

    #[test]
    fn privileged_planner_cannot_be_a_qualifying_baseline() {
        let mut pilot: Value =
            serde_json::from_str(COMMITTED_PILOT_JSON).expect("pilot fixture parses");
        pilot["policy_comparison"]["qualifying_baselines"]
            .as_array_mut()
            .expect("baseline list is an array")
            .push(json!("privileged_planner_upper_bound"));
        let input = serde_json::to_string(&pilot).expect("mutated pilot serializes");
        assert!(validate_pilot_manifest(&input, COMMITTED_PILOT_SCHEMA_JSON).is_err());
    }

    #[test]
    fn hidden_state_denial_is_required() {
        let mut pilot: Value =
            serde_json::from_str(COMMITTED_PILOT_JSON).expect("pilot fixture parses");
        pilot["sensory_contract"]["denied_inputs"] = json!([
            "global_current_vector_field",
            "intervention_label",
            "hidden_model_id",
            "exact_global_resource_coordinates",
            "other_hidden_input"
        ]);
        let input = serde_json::to_string(&pilot).expect("mutated pilot serializes");
        assert!(validate_pilot_manifest(&input, COMMITTED_PILOT_SCHEMA_JSON).is_err());
    }

    #[test]
    fn blind_challenger_parameters_must_remain_sealed() {
        let mut pilot: Value =
            serde_json::from_str(COMMITTED_PILOT_JSON).expect("pilot fixture parses");
        pilot["model_registry"][2]["forage_recovery_lag_ticks"] = json!(180);
        pilot["model_registry"][2]["parameters_disclosed_in_public_fixture"] = json!(true);
        let input = serde_json::to_string(&pilot).expect("mutated pilot serializes");
        assert!(validate_pilot_manifest(&input, COMMITTED_PILOT_SCHEMA_JSON).is_err());
    }

    #[test]
    fn model_registry_schema_cannot_drop_sealed_parameter_fields() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_PILOT_SCHEMA_JSON).expect("pilot schema parses");
        schema["properties"]["model_registry"]["items"]["required"]
            .as_array_mut()
            .expect("model required is an array")
            .retain(|value| value.as_str() != Some("parameter_source"));
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_pilot_manifest(COMMITTED_PILOT_JSON, &input).is_err());
    }

    #[test]
    fn resource_accounting_violation_limit_cannot_be_relaxed() {
        let mut pilot: Value =
            serde_json::from_str(COMMITTED_PILOT_JSON).expect("pilot fixture parses");
        pilot["hard_invariants"]["resource_accounting_violation_limit"] = json!(1);
        let input = serde_json::to_string(&pilot).expect("mutated pilot serializes");
        assert!(validate_pilot_manifest(&input, COMMITTED_PILOT_SCHEMA_JSON).is_err());
    }

    #[test]
    fn pilot_schema_root_must_remain_closed() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_PILOT_SCHEMA_JSON).expect("pilot schema parses");
        schema["additionalProperties"] = json!(true);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_pilot_manifest(COMMITTED_PILOT_JSON, &input).is_err());
    }

    #[test]
    fn duplicate_case_id_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        let first_id = oracle["cases"][0]["id"]
            .as_str()
            .expect("first case has an ID")
            .to_owned();
        oracle["cases"][1]["id"] = json!(first_id);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn declared_case_count_must_match_frozen_version() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["case_count"] = json!(50);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn missing_failure_condition_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["fail_if"] = json!([]);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn undeclared_case_field_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["unexpected"] = json!(true);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn invalid_case_id_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["id"] = json!("Animal Case One");
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn omitted_run_identity_requirement_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["required_run_identity"] = json!(["source_commit"]);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn weakened_schema_case_count_fails_closed() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["properties"]["case_count"]["const"] = json!(50);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn schema_cannot_allow_extra_case_fields() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["properties"]["cases"]["items"]["additionalProperties"] = json!(true);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn schema_cannot_drop_a_required_root_field() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["required"]
            .as_array_mut()
            .expect("required is an array")
            .retain(|value| value.as_str() != Some("cases"));
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn schema_cannot_weaken_failure_condition_requirement() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["properties"]["cases"]["items"]["properties"]["fail_if"]["minItems"] = json!(0);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn invalid_calendar_date_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["research_snapshot"] = json!("2026-02-30");
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn wrong_tracking_issue_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["tracking_issue"] = json!("https://github.com/Luminous-Dynamics/symtropy/issues/1");
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn oracle_and_schema_cannot_jointly_redefine_v1_case_count() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        oracle["case_count"] = json!(52);
        schema["properties"]["case_count"]["const"] = json!(52);
        schema["properties"]["cases"]["minItems"] = json!(52);
        schema["properties"]["cases"]["maxItems"] = json!(52);
        let oracle_input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        let schema_input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(&oracle_input, &schema_input).is_err());
    }

    #[test]
    fn empty_expected_property_list_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["expected"]["required_properties"] = json!([]);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn schema_identity_cannot_be_retargeted() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["$id"] = json!("https://example.invalid/schema.json");
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn root_schema_cannot_allow_undeclared_fields() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["additionalProperties"] = json!(true);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn calendar_validation_handles_leap_year_boundaries() {
        assert!(is_valid_iso_date("2024-02-29"));
        assert!(!is_valid_iso_date("2023-02-29"));
        assert!(!is_valid_iso_date("1900-02-29"));
        assert!(is_valid_iso_date("2000-02-29"));
    }
}

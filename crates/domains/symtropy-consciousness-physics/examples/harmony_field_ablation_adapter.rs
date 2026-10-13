// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execute the versioned HarmonyField crossing-traffic smoke task using the real
//! symtropy-physics::PhysicsWorld and ConsciousnessField callback path.
//!
//! This is a harness smoke task, not a confirmatory utility benchmark. It emits
//! actual simulator traces and records failed/unqualified runs rather than
//! manufacturing successful result records.

use nalgebra::SVector;
use serde_json_runtime::{json, Number, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::panic::{catch_unwind, AssertUnwindSafe};
use symthaea_consciousness_equation::ConsciousnessInputs;
use symtropy_consciousness_physics::{
    ConsciousnessField, HarmonyFieldMechanismToggles,
};
use symtropy_math::Point;
use symtropy_physics::world::PhysicsCallback;
use symtropy_physics::{BodyHandle, PhysicsWorld};

const TASK_ID: &str = "harmony-field-crossing-traffic-smoke-v1";
const PLAN_SCHEMA: &str = "luminous.harmony-field-ablation-plan.v1";
const RESULT_SCHEMA: &str = "luminous.harmony-field-ablation-result.v1";
const PROTOCOL_VERSION: &str = "0.1";
const DT: f64 = 1.0 / 120.0;
const MAX_STEPS: usize = 720;
const AGENT_COUNT: usize = 4;
const BODY_RADIUS: f64 = 0.75;
const KP: f64 = 2.0;
const KD: f64 = 2.5;
const FORCE_CAP: f64 = 30.0;
const GOAL_RADIUS: f64 = 2.0;
const MANIFEST_TOOL_PATH: &str = "scripts/research/harmony_field_ablation.py";
const SCENARIO_PATH: &str = "scripts/research/scenarios/harmony_field_crossing_traffic_smoke.json";
const RNG_EVIDENCE_PATH: &str = "scripts/research/scenarios/harmony_field_rng_alignment.md";
const RNG_EVIDENCE_RELATIVE: &str = "rng-alignment-review.md";
const RNG_EVIDENCE_SHA256: &str =
    "80b285acb54be72e604d35a4bc6c58ebe6cb17be82bd113680aa18efdb91a06c";
const TASK_DESCRIPTION: &str = "Four-agent deterministic crossing-traffic task: each sphere is driven toward its assigned opposite-corner goal under a bounded proportional-derivative controller; the F/R/E switches govern field-friction coupling, resonance-scaled collision surprise, and proximity-weighted emotional contagion.";
const RNG_ALIGNMENT_DESCRIPTION: &str = "One seeded LCG is consumed only while constructing initial positions, before treatment switches. No RNG is consumed inside the deterministic integration loop. The adapter hashes the complete initial state, checks same-seed equality across all arms and parameter variants, and requires byte-identical same-config replay. This is matched initial-state blocking and deterministic replay, not event-level common-random-number alignment.";

const STARTS: [[f64; 3]; AGENT_COUNT] = [
    [-12.0, -12.0, 0.0],
    [-12.0, 12.0, 0.0],
    [12.0, -12.0, 0.0],
    [12.0, 12.0, 0.0],
];
const GOALS: [[f64; 3]; AGENT_COUNT] = [
    [12.0, 12.0, 0.0],
    [12.0, -12.0, 0.0],
    [-12.0, 12.0, 0.0],
    [-12.0, -12.0, 0.0],
];
const HARMONY_PROFILES: [[f64; 9]; AGENT_COUNT] = [
    [0.90, 0.10, 0.20, 0.10, 0.15, 0.25, 0.10, 0.80, 0.85],
    [0.15, 0.88, 0.15, 0.20, 0.10, 0.35, 0.80, 0.15, 0.15],
    [0.20, 0.10, 0.92, 0.15, 0.30, 0.80, 0.10, 0.20, 0.55],
    [0.35, 0.25, 0.30, 0.90, 0.65, 0.25, 0.30, 0.45, 0.30],
];

#[derive(Clone, Copy)]
struct AgentInit {
    start: [f64; 3],
    goal: [f64; 3],
    harmonies: [f64; 9],
}

struct SimulationOutput {
    trace: Value,
    initial_state_sha256: String,
    primary_endpoint: f64,
    secondary_metrics: BTreeMap<String, f64>,
    validation_failure_count: u64,
    nonfinite_count: u64,
    invariant_drift: f64,
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, String> {
    fn write_value(value: &Value, out: &mut String) -> Result<(), String> {
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => out.push_str(&value.to_string()),
            Value::String(value) => {
                out.push_str(&serde_json_runtime::to_string(value).map_err(|e| e.to_string())?);
            }
            Value::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_value(value, out)?;
                }
                out.push(']');
            }
            Value::Object(values) => {
                out.push('{');
                let mut keys: Vec<&String> = values.keys().collect();
                keys.sort();
                for (index, key) in keys.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(
                        &serde_json_runtime::to_string(*key).map_err(|e| e.to_string())?,
                    );
                    out.push(':');
                    let child = values
                        .get(*key)
                        .ok_or_else(|| "canonical JSON object key disappeared".to_string())?;
                    write_value(child, out)?;
                }
                out.push('}');
            }
        }
        Ok(())
    }

    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out.into_bytes())
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_value(value: &Value) -> Result<String, String> {
    Ok(sha256(&canonical_json(value)?))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn value_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field} must be a string"))
}

fn value_f64(value: &Value, field: &str) -> Result<f64, String> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{field} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{field} must be finite"));
    }
    Ok(number)
}

fn expected_arms() -> Vec<Value> {
    HarmonyFieldMechanismToggles::factorial_arms()
        .iter()
        .enumerate()
        .map(|(index, arm)| {
            json!({
                "arm_id": format!("A{}{}{}", arm.friction_modulation as u8,
                                  arm.resonance_prediction_error as u8,
                                  arm.emotional_contagion as u8),
                "arm_index": index,
                "friction_modulation": arm.friction_modulation,
                "resonance_prediction_error": arm.resonance_prediction_error,
                "emotional_contagion": arm.emotional_contagion
            })
        })
        .collect()
}

fn expected_endpoint() -> Value {
    json!({
        "name": "goal_completion_fraction",
        "units": "fraction_of_4_agents_within_2_model_units_of_goal_at_terminal_tick",
        "direction": "higher_is_better",
        "meaningful_effect_threshold": 0.25
    })
}

fn expected_solver() -> Value {
    json!({
        "solver": "symtropy-physics::PhysicsWorld::<3>::step_with_callback",
        "timestep_seconds": 0.008333333333333333,
        "max_steps": 720,
        "agents": 4,
        "body_radius": 0.75,
        "controller": "bounded_PD_Kp_2.0_Kd_2.5_force_norm_cap_30.0"
    })
}

fn expected_rng_contract() -> Value {
    json!({
        "strategy": "single_stateful_stream",
        "treatment_alignment": "same_seed_only",
        "alignment_evidence": RNG_ALIGNMENT_DESCRIPTION,
        "alignment_evidence_path": RNG_EVIDENCE_RELATIVE,
        "alignment_evidence_sha256": RNG_EVIDENCE_SHA256
    })
}

fn number_list(value: &Value, field: &str) -> Result<Vec<f64>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{field} must be an array"))?;
    if values.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, item)| value_f64(item, &format!("{field}[{index}]")))
        .collect()
}

fn seed_list(value: &Value, field: &str) -> Result<Vec<u64>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{field} must be an array"))?;
    if values.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_u64()
                .ok_or_else(|| format!("{field}[{index}] must be a non-negative integer"))
        })
        .collect()
}

fn validate_plan(plan: &Value) -> Result<(), String> {
    if value_string(plan, "schema_version")? != PLAN_SCHEMA {
        return Err("unsupported plan schema_version".to_string());
    }
    if value_string(plan, "plan_status")? != "planned_not_executed" {
        return Err("plan must be explicitly marked planned_not_executed".to_string());
    }
    if value_string(plan, "protocol_version")? != PROTOCOL_VERSION {
        return Err("unsupported protocol_version".to_string());
    }
    if value_string(plan, "task_id")? != TASK_ID {
        return Err(format!("adapter only accepts task_id={TASK_ID}"));
    }
    let candidate = value_string(plan, "candidate_commit")?;
    if candidate.len() != 40 || !candidate.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err("candidate_commit must be a lowercase full 40-character Git SHA".to_string());
    }
    let tool_sha = value_string(plan, "tool_sha256")?;
    if !is_sha256(tool_sha) {
        return Err("tool_sha256 is missing or malformed".to_string());
    }
    let plan_sha = value_string(plan, "plan_sha256")?;
    if !is_sha256(plan_sha) {
        return Err("plan_sha256 is missing or malformed".to_string());
    }
    let mut unsigned = plan.clone();
    unsigned
        .as_object_mut()
        .ok_or_else(|| "plan must be a JSON object".to_string())?
        .remove("plan_sha256");
    if digest_value(&unsigned)? != plan_sha {
        return Err("plan_sha256 does not match canonical plan bytes".to_string());
    }

    let scenario = plan
        .get("scenario_config")
        .ok_or_else(|| "scenario_config is missing".to_string())?;
    if value_string(scenario, "task_description")? != TASK_DESCRIPTION {
        return Err("scenario task_description does not match the frozen smoke task".to_string());
    }
    if scenario.get("primary_endpoint") != Some(&expected_endpoint()) {
        return Err("primary_endpoint differs from the frozen smoke endpoint".to_string());
    }
    if scenario.get("solver_and_timestep") != Some(&expected_solver()) {
        return Err("solver_and_timestep differs from the frozen smoke configuration".to_string());
    }
    if scenario.get("random_stream_contract") != Some(&expected_rng_contract()) {
        return Err("random_stream_contract differs from the audited same-seed-only policy".to_string());
    }
    let scenario_sha = digest_value(scenario)?;
    if value_string(plan, "scenario_config_sha256")? != scenario_sha {
        return Err("scenario_config_sha256 mismatch".to_string());
    }
    if plan.get("factorial_arms") != Some(&Value::Array(expected_arms())) {
        return Err("factorial_arms are not canonical A000..A111".to_string());
    }

    let partition = plan
        .get("seed_partition")
        .ok_or_else(|| "seed_partition is missing".to_string())?;
    let development = seed_list(
        partition
            .get("development")
            .ok_or_else(|| "development seeds are missing".to_string())?,
        "seed_partition.development",
    )?;
    let held_out = seed_list(
        partition
            .get("held_out")
            .ok_or_else(|| "held_out seeds are missing".to_string())?,
        "seed_partition.held_out",
    )?;
    let development_set: BTreeSet<u64> = development.iter().copied().collect();
    let held_out_set: BTreeSet<u64> = held_out.iter().copied().collect();
    if development_set.len() != development.len()
        || held_out_set.len() != held_out.len()
        || !development_set.is_disjoint(&held_out_set)
    {
        return Err("development/held-out seeds must be unique and disjoint".to_string());
    }

    let grid = plan
        .get("parameter_grid")
        .ok_or_else(|| "parameter_grid is missing".to_string())?;
    let dimensions = seed_list(
        grid.get("dimensions")
            .ok_or_else(|| "dimensions are missing".to_string())?,
        "parameter_grid.dimensions",
    )?;
    if dimensions != vec![3] {
        return Err("this versioned adapter supports dimension 3 only".to_string());
    }
    if value_string(grid, "kernel_exponent_rule")? != "max(dimension - 1, 1)"
        || grid.get("kernel_exponent_is_runtime_parameter") != Some(&Value::Bool(false))
    {
        return Err("kernel exponent must remain the recorded derived rule, not a runtime knob".to_string());
    }
    let strengths = number_list(
        grid.get("source_strengths")
            .ok_or_else(|| "source_strengths are missing".to_string())?,
        "parameter_grid.source_strengths",
    )?;
    let radii = number_list(
        grid.get("influence_radii")
            .ok_or_else(|| "influence_radii are missing".to_string())?,
        "parameter_grid.influence_radii",
    )?;
    if strengths.iter().any(|x| *x < 0.0 || *x > 1_000_000.0)
        || radii.iter().any(|x| *x < 0.0001 || *x > 1_000_000.0)
    {
        return Err("source strengths must be in [0, 1e6] and radii in [1e-4, 1e6] for this smoke adapter".to_string());
    }
    if strengths.iter().enumerate().any(|(i, x)| strengths[..i].contains(x))
        || radii.iter().enumerate().any(|(i, x)| radii[..i].contains(x))
    {
        return Err("parameter grid values must not contain duplicates".to_string());
    }

    let planned_runs = plan
        .get("planned_runs")
        .and_then(Value::as_array)
        .ok_or_else(|| "planned_runs must be an array".to_string())?;
    let run_count = plan
        .get("run_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "run_count must be a non-negative integer".to_string())?;
    let seeds: Vec<(u64, &str)> = development
        .iter()
        .map(|seed| (*seed, "development"))
        .chain(held_out.iter().map(|seed| (*seed, "held_out")))
        .collect();
    let arms = expected_arms();
    let expected_count = seeds.len() * dimensions.len() * strengths.len() * radii.len() * arms.len();
    if run_count as usize != planned_runs.len() || planned_runs.len() != expected_count {
        return Err(format!(
            "run_count/coverage mismatch: declared {run_count}, records {}, expected {expected_count}",
            planned_runs.len()
        ));
    }

    let mut expected_hashes = BTreeSet::new();
    for (seed, partition_name) in &seeds {
        for dimension in &dimensions {
            for strength in &strengths {
                for radius in &radii {
                    for arm in &arms {
                        let cfg = json!({
                            "protocol_version": PROTOCOL_VERSION,
                            "candidate_commit": candidate,
                            "task_id": TASK_ID,
                            "scenario_config_sha256": scenario_sha,
                            "seed": seed,
                            "seed_partition": partition_name,
                            "dimension": dimension,
                            "source_strength": strength,
                            "influence_radius": radius,
                            "arm_id": arm["arm_id"],
                            "arm_index": arm["arm_index"],
                            "mechanisms": {
                                "friction_modulation": arm["friction_modulation"],
                                "resonance_prediction_error": arm["resonance_prediction_error"],
                                "emotional_contagion": arm["emotional_contagion"]
                            }
                        });
                        expected_hashes.insert(digest_value(&cfg)?);
                    }
                }
            }
        }
    }

    let mut observed_hashes = BTreeSet::new();
    for (index, record) in planned_runs.iter().enumerate() {
        if value_string(record, "run_status")? != "planned" {
            return Err(format!("planned_runs[{index}] is not marked planned"));
        }
        let cfg = record
            .get("configuration")
            .ok_or_else(|| format!("planned_runs[{index}] has no configuration"))?;
        let cfg_hash = digest_value(cfg)?;
        if value_string(record, "configuration_sha256")? != cfg_hash {
            return Err(format!("planned_runs[{index}] configuration SHA mismatch"));
        }
        if value_string(record, "run_id")? != format!("run-{cfg_hash}") {
            return Err(format!("planned_runs[{index}] run_id mismatch"));
        }
        if value_string(cfg, "candidate_commit")? != candidate
            || value_string(cfg, "task_id")? != TASK_ID
            || value_string(cfg, "scenario_config_sha256")? != scenario_sha
        {
            return Err(format!("planned_runs[{index}] identity mismatch"));
        }
        if cfg.get("dimension").and_then(Value::as_u64) != Some(3) {
            return Err(format!("planned_runs[{index}] is not dimension 3"));
        }
        let arm_index = cfg
            .get("arm_index")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("planned_runs[{index}] arm_index missing"))? as usize;
        let arm = arms
            .get(arm_index)
            .ok_or_else(|| format!("planned_runs[{index}] arm_index out of range"))?;
        if cfg.get("arm_id") != arm.get("arm_id")
            || cfg.get("mechanisms")
                != Some(&json!({
                    "friction_modulation": arm["friction_modulation"],
                    "resonance_prediction_error": arm["resonance_prediction_error"],
                    "emotional_contagion": arm["emotional_contagion"]
                }))
        {
            return Err(format!("planned_runs[{index}] arm ID/switch mismatch"));
        }
        if !observed_hashes.insert(cfg_hash) {
            return Err(format!("planned_runs[{index}] duplicates a configuration"));
        }
    }
    if observed_hashes != expected_hashes {
        return Err(format!(
            "planned run grid differs from complete expected factorial grid ({} missing, {} unexpected)",
            expected_hashes.difference(&observed_hashes).count(),
            observed_hashes.difference(&expected_hashes).count()
        ));
    }
    Ok(())
}

fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 11) as f64 / (1u64 << 53) as f64
}

fn layout(seed: u64) -> [AgentInit; AGENT_COUNT] {
    let mut rng = seed;
    std::array::from_fn(|index| {
        let mut start = STARTS[index];
        // Consume all RNG only during initialization; integration has no RNG path.
        for coordinate in &mut start {
            *coordinate += (lcg_next(&mut rng) - 0.5) * 0.5;
        }
        AgentInit {
            start,
            goal: GOALS[index],
            harmonies: HARMONY_PROFILES[index],
        }
    })
}

fn initial_state_value(seed: u64) -> Value {
    let agents: Vec<Value> = layout(seed)
        .iter()
        .enumerate()
        .map(|(index, init)| {
            json!({
                "agent_index": index,
                "initial_position": init.start,
                "goal_position": init.goal,
                "initial_velocity": [0.0, 0.0, 0.0],
                "harmony_activations": init.harmonies
            })
        })
        .collect();
    json!({
        "task_id": TASK_ID,
        "seed": seed,
        "dt": DT,
        "max_steps": MAX_STEPS,
        "agents": agents
    })
}

fn point_for(position: &SVector<f64, 3>) -> Point<3> {
    Point::new([position[0], position[1], position[2]])
}

fn energy_kinetic(world: &PhysicsWorld<3>, handles: &[BodyHandle], nonfinite: &mut u64) -> f64 {
    let mut total = 0.0;
    for handle in handles {
        if let Some(body) = world.body(*handle) {
            let energy = 0.5 * body.mass * body.linear_velocity.norm_squared();
            if energy.is_finite() {
                total += energy;
            } else {
                *nonfinite += 1;
            }
        } else {
            *nonfinite += 1;
        }
    }
    if total.is_finite() {
        total
    } else {
        *nonfinite += 1;
        0.0
    }
}

fn finite_json(value: f64, nonfinite: &mut u64) -> Value {
    if !value.is_finite() {
        *nonfinite += 1;
        return Value::Null;
    }
    Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or_else(|| {
            *nonfinite += 1;
            Value::Null
        })
}

fn finite_metric(value: f64, nonfinite: &mut u64) -> f64 {
    if value.is_finite() {
        value
    } else {
        *nonfinite += 1;
        0.0
    }
}

fn capture_states(
    world: &PhysicsWorld<3>,
    field: &ConsciousnessField<3>,
    handles: &[BodyHandle],
    goals: &[[f64; 3]],
    nonfinite: &mut u64,
) -> Value {
    let mut states = Vec::with_capacity(handles.len());
    for (index, handle) in handles.iter().enumerate() {
        let body = world.body(*handle);
        let entity = field.entities.get(handle);
        match (body, entity) {
            (Some(body), Some(entity)) => {
                let pos = body.position();
                let vel = body.linear_velocity;
                let goal = SVector::from(goals[index]);
                let distance = (goal - pos).norm();
                states.push(json!({
                    "handle": handle.0,
                    "position": [
                        finite_json(pos[0], nonfinite),
                        finite_json(pos[1], nonfinite),
                        finite_json(pos[2], nonfinite)
                    ],
                    "velocity": [
                        finite_json(vel[0], nonfinite),
                        finite_json(vel[1], nonfinite),
                        finite_json(vel[2], nonfinite)
                    ],
                    "goal_distance": finite_json(distance, nonfinite),
                    "energy_available": finite_json(entity.energy.available, nonfinite),
                    "prediction_error": finite_json(entity.prediction_error, nonfinite),
                    "motor_precision": finite_json(entity.motor_precision, nonfinite),
                    "emotional_activation": finite_json(entity.harmony_activations[8], nonfinite)
                }));
            }
            _ => {
                *nonfinite += 1;
                states.push(json!({ "handle": handle.0, "missing_state": true }));
            }
        }
    }
    Value::Array(states)
}

fn run_simulation(
    configuration: &Value,
    run_id: &str,
    candidate_commit: &str,
    plan_sha: &str,
) -> Result<SimulationOutput, String> {
    let seed = configuration
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| "run seed is missing or malformed".to_string())?;
    let strength = value_f64(
        configuration.get("source_strength").ok_or("source_strength missing")?,
        "source_strength",
    )?;
    let radius = value_f64(
        configuration.get("influence_radius").ok_or("influence_radius missing")?,
        "influence_radius",
    )?;
    let mechanisms = configuration
        .get("mechanisms")
        .ok_or_else(|| "mechanisms object missing".to_string())?;
    let toggles = HarmonyFieldMechanismToggles {
        friction_modulation: mechanisms
            .get("friction_modulation")
            .and_then(Value::as_bool)
            .ok_or_else(|| "friction_modulation switch missing".to_string())?,
        resonance_prediction_error: mechanisms
            .get("resonance_prediction_error")
            .and_then(Value::as_bool)
            .ok_or_else(|| "resonance_prediction_error switch missing".to_string())?,
        emotional_contagion: mechanisms
            .get("emotional_contagion")
            .and_then(Value::as_bool)
            .ok_or_else(|| "emotional_contagion switch missing".to_string())?,
    };
    let cfg_sha = digest_value(configuration)?;
    let initial_state = initial_state_value(seed);
    let initial_state_sha256 = digest_value(&initial_state)?;

    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let mut field = ConsciousnessField::<3>::new();
    field.mechanism_toggles = toggles;
    field.constants.harmony_range = radius;
    let initial_agents = layout(seed);
    let mut handles = Vec::with_capacity(AGENT_COUNT);
    let mut goals = Vec::with_capacity(AGENT_COUNT);

    for init in &initial_agents {
        let handle = world.add_sphere(Point::new(init.start), BODY_RADIUS, 1.0);
        if let Some(body) = world.body_mut(handle) {
            body.friction = 0.6;
            body.restitution = 0.05;
            body.linear_damping = 0.1;
            body.linear_velocity = SVector::zeros();
        } else {
            return Err("PhysicsWorld failed to return a newly added body".to_string());
        }
        field.register(handle, 100.0, radius);
        if let Some(entity) = field.entities.get_mut(&handle) {
            entity.harmony_activations = init.harmonies;
        } else {
            return Err("ConsciousnessField failed to register a body".to_string());
        }
        handles.push(handle);
        goals.push(init.goal);
    }

    let inputs = ConsciousnessInputs {
        phi: 0.8,
        broadcast: 0.8,
        working_memory: 0.6,
        attention: 0.7,
        recurrence: 0.65,
        embodiment: 0.8,
        knowledge: 0.6,
        synchrony: 0.7,
    };
    let mut nonfinite_count = 0_u64;
    let mut validation_failure_count = 0_u64;
    let mut cumulative_collisions = 0_u64;
    let mut cumulative_collision_impulse = 0.0_f64;
    let mut max_normalized_positive_energy_residual = 0.0_f64;
    let mut frames = Vec::with_capacity(MAX_STEPS);

    for tick in 0..MAX_STEPS {
        let positions: Vec<(BodyHandle, Point<3>)> = handles
            .iter()
            .filter_map(|handle| world.body(*handle).map(|body| (*handle, point_for(&body.position()))))
            .collect();
        if positions.len() != AGENT_COUNT {
            return Err(format!("missing body at tick {tick}"));
        }

        for (handle, position) in &positions {
            field.update_entity(*handle, &inputs, *position);
        }
        field.spread_emotional_contagion(&positions, DT);
        field.rebuild_harmony_field(&positions);
        for source in &mut field.harmony_field.sources {
            // The planned parameter grid is applied to actual checked sources.
            source.strength = strength;
            source.radius = radius;
        }

        let mut effective_forces = Vec::with_capacity(AGENT_COUNT);
        let mut before_positions = Vec::with_capacity(AGENT_COUNT);
        for (index, handle) in handles.iter().enumerate() {
            let body = world
                .body(*handle)
                .ok_or_else(|| format!("body disappeared before tick {tick}"))?;
            let position = body.position();
            let velocity = body.linear_velocity;
            let goal = SVector::from(goals[index]);
            let mut force = (goal - position) * KP - velocity * KD;
            let norm = force.norm();
            if !norm.is_finite() || !force.iter().all(|component| component.is_finite()) {
                nonfinite_count += 1;
                force = SVector::zeros();
            } else if norm > FORCE_CAP {
                force *= FORCE_CAP / norm;
            }
            let effective = PhysicsCallback::<3>::modulate_force(&field, *handle, &force);
            if effective.iter().all(|component| component.is_finite()) {
                effective_forces.push(effective);
            } else {
                nonfinite_count += 1;
                effective_forces.push(SVector::zeros());
            }
            before_positions.push(position);
            world
                .body_mut(*handle)
                .ok_or_else(|| format!("body disappeared while applying force at tick {tick}"))?
                .apply_force(force);
        }

        let energy_before = energy_kinetic(&world, &handles, &mut nonfinite_count);
        world.step_with_callback(DT, &mut field);
        let energy_after = energy_kinetic(&world, &handles, &mut nonfinite_count);
        let mut work_estimate = 0.0;
        for (index, handle) in handles.iter().enumerate() {
            if let Some(body) = world.body(*handle) {
                let displacement = body.position() - before_positions[index];
                work_estimate += effective_forces[index].dot(&displacement);
            }
        }
        let residual = ((energy_after - energy_before) - work_estimate).max(0.0)
            / energy_before.max(1.0e-12);
        if residual.is_finite() {
            max_normalized_positive_energy_residual =
                max_normalized_positive_energy_residual.max(residual);
        } else {
            nonfinite_count += 1;
        }

        let collision_count = world.collision_events.len() as u64;
        let collision_impulse: f64 = world
            .collision_events
            .iter()
            .map(|event| event.impulse.abs())
            .sum();
        if !collision_impulse.is_finite() {
            nonfinite_count += 1;
        } else {
            cumulative_collision_impulse += collision_impulse;
        }
        cumulative_collisions += collision_count;
        validation_failure_count += field.take_harmony_field_validation_failures();

        let positions_after: Vec<(BodyHandle, Point<3>)> = handles
            .iter()
            .filter_map(|handle| world.body(*handle).map(|body| (*handle, point_for(&body.position()))))
            .collect();
        let states = capture_states(&world, &field, &handles, &goals, &mut nonfinite_count);
        frames.push(json!({
            "tick": tick,
            "time_seconds": (tick + 1) as f64 * DT,
            "collision_count": collision_count,
            "collision_impulse": finite_json(collision_impulse, &mut nonfinite_count),
            "kinetic_energy_before": finite_json(energy_before, &mut nonfinite_count),
            "kinetic_energy_after": finite_json(energy_after, &mut nonfinite_count),
            "normalized_positive_energy_residual": finite_json(residual, &mut nonfinite_count),
            "states": states
        }));
        field.tick_prediction_errors();
        // Keep this binding explicit: it proves positions were captured from the
        // actual post-step world and remains useful when reviewing the trace.
        if positions_after.len() != AGENT_COUNT {
            nonfinite_count += 1;
        }
    }

    let mut completed = 0_u64;
    let mut sum_goal_distance = 0.0;
    let mut terminal_prediction_error = 0.0;
    let mut terminal_motor_precision = 0.0;
    let mut terminal_emotion = 0.0;
    for (index, handle) in handles.iter().enumerate() {
        if let (Some(body), Some(entity)) = (world.body(*handle), field.entities.get(handle)) {
            let distance = (SVector::from(goals[index]) - body.position()).norm();
            if distance <= GOAL_RADIUS {
                completed += 1;
            }
            sum_goal_distance += finite_metric(distance, &mut nonfinite_count);
            terminal_prediction_error += finite_metric(entity.prediction_error, &mut nonfinite_count);
            terminal_motor_precision += finite_metric(entity.motor_precision, &mut nonfinite_count);
            terminal_emotion += finite_metric(entity.harmony_activations[8], &mut nonfinite_count);
        } else {
            nonfinite_count += 1;
        }
    }

    let primary_endpoint = completed as f64 / AGENT_COUNT as f64;
    let terminal_mean_goal_distance = sum_goal_distance / AGENT_COUNT as f64;
    let terminal_total_kinetic_energy = energy_kinetic(&world, &handles, &mut nonfinite_count);
    let mean_terminal_prediction_error = terminal_prediction_error / AGENT_COUNT as f64;
    let mean_terminal_motor_precision = terminal_motor_precision / AGENT_COUNT as f64;
    let mean_terminal_emotional_activation = terminal_emotion / AGENT_COUNT as f64;

    let mut secondary_metrics = BTreeMap::new();
    secondary_metrics.insert("terminal_mean_goal_distance".to_string(), finite_metric(terminal_mean_goal_distance, &mut nonfinite_count));
    secondary_metrics.insert("terminal_total_kinetic_energy".to_string(), finite_metric(terminal_total_kinetic_energy, &mut nonfinite_count));
    secondary_metrics.insert("cumulative_collisions".to_string(), cumulative_collisions as f64);
    secondary_metrics.insert("cumulative_collision_impulse".to_string(), finite_metric(cumulative_collision_impulse, &mut nonfinite_count));
    secondary_metrics.insert("mean_terminal_prediction_error".to_string(), finite_metric(mean_terminal_prediction_error, &mut nonfinite_count));
    secondary_metrics.insert("mean_terminal_motor_precision".to_string(), finite_metric(mean_terminal_motor_precision, &mut nonfinite_count));
    secondary_metrics.insert("mean_terminal_emotional_activation".to_string(), finite_metric(mean_terminal_emotional_activation, &mut nonfinite_count));
    secondary_metrics.insert("max_normalized_positive_energy_residual".to_string(), finite_metric(max_normalized_positive_energy_residual, &mut nonfinite_count));

    let trace = json!({
        "schema_version": RESULT_SCHEMA,
        "task_id": TASK_ID,
        "run_id": run_id,
        "candidate_commit": candidate_commit,
        "plan_sha256": plan_sha,
        "configuration_sha256": cfg_sha,
        "configuration": configuration,
        "initial_state": initial_state,
        "initial_state_sha256": initial_state_sha256,
        "solver": "symtropy-physics::PhysicsWorld::<3>::step_with_callback",
        "timestep_seconds": DT,
        "termination_reason": "fixed_tick_budget_completed",
        "frames": frames,
        "summary": {
            "goal_completion_fraction": primary_endpoint,
            "terminal_mean_goal_distance": terminal_mean_goal_distance,
            "terminal_total_kinetic_energy": terminal_total_kinetic_energy,
            "cumulative_collisions": cumulative_collisions,
            "cumulative_collision_impulse": cumulative_collision_impulse,
            "validation_failure_count": validation_failure_count,
            "nonfinite_count": nonfinite_count,
            "max_normalized_positive_energy_residual": max_normalized_positive_energy_residual
        }
    });

    Ok(SimulationOutput {
        trace,
        initial_state_sha256,
        primary_endpoint: finite_metric(primary_endpoint, &mut nonfinite_count),
        secondary_metrics,
        validation_failure_count,
        nonfinite_count,
        invariant_drift: finite_metric(max_normalized_positive_energy_residual, &mut nonfinite_count),
    })
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else {
        "simulation panicked with a non-string payload".to_string()
    }
}

fn failed_record(
    planned: &Value,
    candidate_commit: &str,
    plan_sha: &str,
    status: &str,
    reason: &str,
) -> Value {
    let configuration = planned.get("configuration").unwrap_or(&Value::Null);
    json!({
        "schema_version": RESULT_SCHEMA,
        "run_id": planned.get("run_id").and_then(Value::as_str).unwrap_or("invalid-run-id"),
        "candidate_commit": candidate_commit,
        "plan_sha256": plan_sha,
        "configuration_sha256": planned.get("configuration_sha256").and_then(Value::as_str).unwrap_or(""),
        "seed": configuration.get("seed").and_then(Value::as_u64).unwrap_or(0),
        "run_status": status,
        "termination_reason": reason,
        "primary_endpoint": null,
        "secondary_metrics": {},
        "validation_failure_count": 0,
        "nonfinite_count": 0,
        "invariant_drift": 0.0,
        "replay_status": "not_tested"
    })
}

fn write_trace(
    artifact_root: &Path,
    run_id: &str,
    suffix: &str,
    trace: &Value,
) -> Result<(String, String, Vec<u8>), String> {
    if !run_id.starts_with("run-")
        || run_id.len() != 68
        || !run_id[4..].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("refusing unsafe run_id when writing an artifact".to_string());
    }
    let bytes = serde_json_runtime::to_vec(trace).map_err(|e| e.to_string())?;
    let relative = format!("runs/{run_id}.{suffix}.json");
    let path = artifact_root.join(&relative);
    fs::write(&path, &bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    let digest = sha256(&bytes);
    Ok((relative, digest, bytes))
}

fn validate_source_checkout(repo_root: &Path, expected_commit: &str) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("cannot run git rev-parse: {e}"))?;
    if !output.status.success() {
        return Err("git rev-parse HEAD failed".to_string());
    }
    let actual_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if actual_commit != expected_commit {
        return Err(format!(
            "checkout commit mismatch: plan requires {expected_commit}, checkout is {actual_commit}"
        ));
    }
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .map_err(|e| format!("cannot run git status: {e}"))?;
    if !output.status.success() {
        return Err("git status failed".to_string());
    }
    if !String::from_utf8_lossy(&output.stdout).trim().is_empty() {
        return Err("working tree is dirty; the adapter refuses to execute an unbound source tree".to_string());
    }
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut args = env::args().skip(1);
    let mut plan = None;
    let mut artifact_root = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--plan" => {
                plan = Some(PathBuf::from(args.next().ok_or("--plan needs a path")?));
            }
            "--artifact-root" => {
                artifact_root = Some(PathBuf::from(args.next().ok_or("--artifact-root needs a path")?));
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run -p symtropy-consciousness-physics --example harmony_field_ablation_adapter -- --plan PLAN.json --artifact-root EMPTY_DIR"
                );
                process::exit(0);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok((
        plan.ok_or("--plan is required")?,
        artifact_root.ok_or("--artifact-root is required")?,
    ))
}

fn execute() -> Result<(), String> {
    let (plan_path, artifact_arg) = parse_args()?;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .join("../../../")
        .canonicalize()
        .map_err(|e| format!("cannot resolve repository root: {e}"))?;
    let plan_path = plan_path
        .canonicalize()
        .map_err(|e| format!("cannot resolve plan file: {e}"))?;
    let plan_bytes = fs::read(&plan_path)
        .map_err(|e| format!("cannot read plan file {}: {e}", plan_path.display()))?;
    let plan: Value = serde_json_runtime::from_slice(&plan_bytes)
        .map_err(|e| format!("plan is not valid JSON: {e}"))?;
    validate_plan(&plan)?;

    let candidate_commit = value_string(&plan, "candidate_commit")?.to_string();
    let plan_sha = value_string(&plan, "plan_sha256")?.to_string();
    validate_source_checkout(&repo_root, &candidate_commit)?;

    // Bind the inline plan scenario and planner identity to the exact files in
    // this checkout rather than trusting self-consistent but externally edited JSON.
    let scenario_bytes = fs::read(repo_root.join(SCENARIO_PATH))
        .map_err(|e| format!("cannot read frozen scenario file: {e}"))?;
    let tracked_scenario: Value = serde_json_runtime::from_slice(&scenario_bytes)
        .map_err(|e| format!("tracked scenario file is not valid JSON: {e}"))?;
    if plan.get("scenario_config") != Some(&tracked_scenario) {
        return Err("plan scenario_config differs from the committed frozen scenario file".to_string());
    }
    let manifest_tool_bytes = fs::read(repo_root.join(MANIFEST_TOOL_PATH))
        .map_err(|e| format!("cannot read required manifest tool {MANIFEST_TOOL_PATH}: {e}"))?;
    let actual_tool_sha = sha256(&manifest_tool_bytes);
    if value_string(&plan, "tool_sha256")? != actual_tool_sha {
        return Err("plan tool_sha256 does not match the committed manifest-tool bytes".to_string());
    }

    let audit_path = repo_root.join(RNG_EVIDENCE_PATH);
    let audit_bytes = fs::read(&audit_path)
        .map_err(|e| format!("cannot read RNG evidence {}: {e}", audit_path.display()))?;
    if sha256(&audit_bytes) != RNG_EVIDENCE_SHA256 {
        return Err("tracked RNG alignment evidence bytes do not match the frozen SHA-256".to_string());
    }

    let artifact_root = if artifact_arg.is_absolute() {
        artifact_arg
    } else {
        env::current_dir()
            .map_err(|e| e.to_string())?
            .join(artifact_arg)
    };
    fs::create_dir_all(&artifact_root)
        .map_err(|e| format!("cannot create artifact root: {e}"))?;
    let artifact_root = artifact_root
        .canonicalize()
        .map_err(|e| format!("cannot resolve artifact root: {e}"))?;
    if artifact_root.starts_with(&repo_root) || repo_root.starts_with(&artifact_root) {
        return Err("artifact root must be disjoint from the source repository".to_string());
    }
    if fs::read_dir(&artifact_root)
        .map_err(|e| format!("cannot inspect artifact root: {e}"))?
        .next()
        .is_some()
    {
        return Err("artifact root must be empty; use a new directory for each execution".to_string());
    }
    if plan_path.starts_with(&artifact_root) {
        return Err("plan file must not be inside the initially empty artifact root".to_string());
    }
    fs::create_dir_all(artifact_root.join("runs"))
        .map_err(|e| format!("cannot create runs directory: {e}"))?;
    fs::write(artifact_root.join(RNG_EVIDENCE_RELATIVE), &audit_bytes)
        .map_err(|e| format!("cannot copy RNG alignment evidence: {e}"))?;

    let planned_runs = plan
        .get("planned_runs")
        .and_then(Value::as_array)
        .ok_or_else(|| "planned_runs must be an array".to_string())?;
    let mut records: Vec<Value> = Vec::with_capacity(planned_runs.len());
    let mut initial_hash_by_seed: BTreeMap<u64, String> = BTreeMap::new();
    let mut invalid_seed_hashes = BTreeSet::new();

    for planned in planned_runs {
        let run_id = value_string(planned, "run_id")?.to_string();
        let cfg = planned
            .get("configuration")
            .ok_or_else(|| format!("{run_id} configuration missing"))?;
        let seed = cfg.get("seed").and_then(Value::as_u64).unwrap_or(0);
        let first = catch_unwind(AssertUnwindSafe(|| run_simulation(cfg, &run_id, &candidate_commit, &plan_sha)));
        let replay = catch_unwind(AssertUnwindSafe(|| run_simulation(cfg, &run_id, &candidate_commit, &plan_sha)));

        let first_output = match first {
            Ok(Ok(output)) => Some(output),
            Ok(Err(error)) => {
                records.push(failed_record(planned, &candidate_commit, &plan_sha, "failed", &error));
                continue;
            }
            Err(payload) => {
                records.push(failed_record(
                    planned,
                    &candidate_commit,
                    &plan_sha,
                    "failed",
                    &format!("primary simulation panic: {}", panic_message(payload.as_ref())),
                ));
                continue;
            }
        };
        let replay_output = match replay {
            Ok(Ok(output)) => Some(output),
            Ok(Err(error)) => {
                let _ = first_output.as_ref();
                records.push(failed_record(
                    planned,
                    &candidate_commit,
                    &plan_sha,
                    "failed",
                    &format!("replay execution error: {error}"),
                ));
                continue;
            }
            Err(payload) => {
                records.push(failed_record(
                    planned,
                    &candidate_commit,
                    &plan_sha,
                    "failed",
                    &format!("replay simulation panic: {}", panic_message(payload.as_ref())),
                ));
                continue;
            }
        };
        let first_output = first_output.ok_or("primary output missing after success")?;
        let replay_output = replay_output.ok_or("replay output missing after success")?;
        let first_bytes = serde_json_runtime::to_vec(&first_output.trace).map_err(|e| e.to_string())?;
        let replay_bytes = serde_json_runtime::to_vec(&replay_output.trace).map_err(|e| e.to_string())?;
        let replay_exact = first_bytes == replay_bytes;

        if first_output.initial_state_sha256 != replay_output.initial_state_sha256 {
            invalid_seed_hashes.insert(seed);
        }
        if let Some(previous) = initial_hash_by_seed.get(&seed) {
            if previous != &first_output.initial_state_sha256 {
                invalid_seed_hashes.insert(seed);
            }
        } else {
            initial_hash_by_seed.insert(seed, first_output.initial_state_sha256.clone());
        }

        let (artifact_path, artifact_hash, _) =
            write_trace(&artifact_root, &run_id, "trace", &first_output.trace)?;
        let (replay_path, replay_hash, _) =
            write_trace(&artifact_root, &run_id, "replay", &replay_output.trace)?;

        let validation_failure_count =
            first_output.validation_failure_count + replay_output.validation_failure_count;
        let nonfinite_count = first_output.nonfinite_count + replay_output.nonfinite_count;
        let status = if !replay_exact
            || validation_failure_count > 0
            || nonfinite_count > 0
        {
            "unqualified"
        } else {
            "completed"
        };
        let reason = if !replay_exact {
            "replay_trace_mismatch"
        } else if validation_failure_count > 0 {
            "nonzero_harmony_field_validation_failure_count"
        } else if nonfinite_count > 0 {
            "nonfinite_simulation_state_or_metric"
        } else {
            "fixed_tick_budget_completed_and_replay_exact"
        };
        let primary_endpoint = if status == "completed" {
            json!(first_output.primary_endpoint)
        } else {
            Value::Null
        };
        let secondary_metrics = if status == "completed" {
            serde_json_runtime::to_value(&first_output.secondary_metrics).map_err(|e| e.to_string())?
        } else {
            json!({})
        };
        records.push(json!({
            "schema_version": RESULT_SCHEMA,
            "run_id": run_id,
            "candidate_commit": candidate_commit,
            "plan_sha256": plan_sha,
            "configuration_sha256": value_string(planned, "configuration_sha256")?,
            "seed": seed,
            "run_status": status,
            "termination_reason": reason,
            "primary_endpoint": primary_endpoint,
            "secondary_metrics": secondary_metrics,
            "validation_failure_count": validation_failure_count,
            "nonfinite_count": nonfinite_count,
            "invariant_drift": first_output.invariant_drift,
            "artifact_path": artifact_path,
            "artifact_sha256": artifact_hash,
            "replay_status": if replay_exact { "exact_match" } else { "mismatch" },
            "replay_evidence_path": replay_path,
            "replay_evidence_sha256": replay_hash,
            "initial_state_sha256": first_output.initial_state_sha256
        }));
    }

    // A per-seed initial-state mismatch invalidates every treatment/parameter
    // record for that seed, including records written before the mismatch surfaced.
    for record in &mut records {
        let seed = record.get("seed").and_then(Value::as_u64).unwrap_or(0);
        if invalid_seed_hashes.contains(&seed) {
            if let Some(object) = record.as_object_mut() {
                object.insert("run_status".to_string(), json!("unqualified"));
                object.insert("primary_endpoint".to_string(), Value::Null);
                object.insert(
                    "termination_reason".to_string(),
                    json!("same_seed_initial_state_hash_mismatch_across_factorial_grid"),
                );
            }
        }
    }

    if records.len() != planned_runs.len() {
        return Err(format!(
            "internal result count mismatch: {} records for {} planned runs",
            records.len(),
            planned_runs.len()
        ));
    }
    let mut jsonl = Vec::new();
    let mut status_counts: BTreeMap<String, u64> = BTreeMap::new();
    for record in &records {
        let encoded = serde_json_runtime::to_vec(record).map_err(|e| e.to_string())?;
        jsonl.extend_from_slice(&encoded);
        jsonl.push(b'\n');
        let status = value_string(record, "run_status")?.to_string();
        *status_counts.entry(status).or_default() += 1;
    }
    fs::write(artifact_root.join("results.jsonl"), &jsonl)
        .map_err(|e| format!("cannot write results.jsonl: {e}"))?;

    let summary = json!({
        "adapter_execution_status": "records_written",
        "task_id": TASK_ID,
        "candidate_commit": candidate_commit,
        "plan_sha256": plan_sha,
        "planned_run_count": planned_runs.len(),
        "recorded_run_count": records.len(),
        "run_status_counts": status_counts,
        "invalid_seed_initial_state_hashes": invalid_seed_hashes,
        "random_stream_claim": "same_seed_only; initialization matched and integration has no RNG; no event-level CRN claim",
        "artifact_root": artifact_root,
        "qualification_note": "Run records require the independent manifest validator; even a mechanically qualified smoke manifest is not evidence of HarmonyField utility."
    });
    println!("{}", serde_json_runtime::to_string_pretty(&summary).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("harmony_field_ablation_adapter: {error}");
        process::exit(2);
    }
}

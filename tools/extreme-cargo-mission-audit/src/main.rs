use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, path::Path};

const CORPUS: &str =
    include_str!("../../../fixtures/extreme_environment/extreme_cargo_mission_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_payload_contract_audit_is_not_physical_or_clinical_qualification";
const AXES: [&str; 3] = ["x", "y", "z"];
const DIMENSIONS: [&str; 3] = ["length", "width", "height"];
const PAYLOAD_CLASSES: [&str; 8] = [
    "bulk_cargo", "cold_chain", "fragile_instrument", "contained_liquid",
    "seated_passengers", "wheelchair_user", "stretcher_patient", "field_crew",
];
const OCCUPANT_CLASSES: [&str; 5] = [
    "none", "seated", "wheelchair", "stretcher_patient", "field_crew",
];
const MODES: [&str; 6] = [
    "watercraft", "wheeled_rover", "tracked_carrier", "sled",
    "ground_carrier", "aerial",
];

#[derive(Clone, Debug)]
struct Range {
    lo: f64,
    hi: f64,
}

#[derive(Clone, Debug)]
struct Payload {
    id: String,
    class: String,
    mass: Range,
    dims: [f64; 3],
    center_fraction: [f64; 3],
    temperature: Option<Range>,
    shock_g: f64,
    tilt_deg: f64,
    vibration: f64,
    containment: bool,
    securement: bool,
    occupants: u64,
    occupant_class: String,
    restraints: bool,
    accessible: bool,
    wheelchair_securement: bool,
    patient_support: bool,
    caregiver_access: bool,
    egress: bool,
    privacy_sensitive: bool,
}

#[derive(Clone, Debug)]
struct Platform {
    id: String,
    mode: String,
    environments: BTreeSet<String>,
    payload_classes: BTreeSet<String>,
    max_payload_kg: f64,
    bay: [f64; 3],
    ambient: Range,
    controlled_temperature: Option<Range>,
    max_shock_g: f64,
    maximum_safe_route_tilt_deg: f64,
    max_vibration: f64,
    center_limits: [Range; 3],
    securement: bool,
    containment: bool,
    occupant_restraints: bool,
    accessible_boarding: bool,
    wheelchair_securement: bool,
    patient_support: bool,
    caregiver_access: bool,
    egress: bool,
    privacy_controls: bool,
    recovery_plan: bool,
    emergency_stop: bool,
}

#[derive(Clone, Debug)]
struct Mission {
    id: String,
    environment: String,
    class: String,
    payload_ids: Vec<String>,
    allowed_modes: BTreeSet<String>,
    required_gates: BTreeSet<String>,
    ambient: Range,
    required_route_tilt_deg: f64,
    human_transport: bool,
}

#[derive(Clone, Debug)]
struct Evaluated {
    id: String,
    mission_id: String,
    platform_id: String,
    disposition: String,
    failed_gates: Vec<String>,
    unknown_gates: Vec<String>,
    mismatch_reasons: Vec<String>,
    unknown_metrics: Vec<String>,
    metrics: BTreeMap<String, Option<Range>>,
    layout: BTreeMap<String, [f64; 3]>,
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    let Some(object) = value.as_object() else { return false; };
    object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
}

fn push(errors: &mut Vec<String>, path: &str, message: &str) {
    errors.push(format!("{path}: {message}"));
}

fn read_string(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<String> {
    match value.get(key).and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Some(text.to_string()),
        Some(_) => {
            push(errors, &format!("{path}.{key}"), "must not be empty");
            None
        }
        None => {
            push(errors, &format!("{path}.{key}"), "expected string");
            None
        }
    }
}

fn read_number(value: &Value, path: &str, errors: &mut Vec<String>) -> Option<f64> {
    match value.as_f64() {
        Some(number) if number.is_finite() => Some(number),
        _ => {
            push(errors, path, "expected finite number");
            None
        }
    }
}

fn read_bool(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<bool> {
    match value.get(key).and_then(Value::as_bool) {
        Some(result) => Some(result),
        None => {
            push(errors, &format!("{path}.{key}"), "expected boolean");
            None
        }
    }
}

fn read_range(value: &Value, path: &str, errors: &mut Vec<String>) -> Option<Range> {
    if !exact_keys(value, &["min", "max"]) {
        push(errors, path, "expected exactly min and max");
        return None;
    }
    let lo = read_number(&value["min"], &format!("{path}.min"), errors);
    let hi = read_number(&value["max"], &format!("{path}.max"), errors);
    match (lo, hi) {
        (Some(lo), Some(hi)) if lo <= hi => Some(Range { lo, hi }),
        (Some(_), Some(_)) => {
            push(errors, path, "minimum must be <= maximum");
            None
        }
        _ => None,
    }
}

fn read_nullable_range(
    value: &Value,
    path: &str,
    errors: &mut Vec<String>,
) -> Option<Option<Range>> {
    if value.is_null() {
        return Some(None);
    }
    read_range(value, path, errors).map(Some)
}

fn read_dimensions(value: &Value, path: &str, errors: &mut Vec<String>) -> Option<[f64; 3]> {
    if !exact_keys(value, &DIMENSIONS) {
        push(errors, path, "expected length, width and height");
        return None;
    }
    let mut dims = [0.0; 3];
    let mut valid = true;
    for (index, key) in DIMENSIONS.iter().enumerate() {
        match value.get(*key).and_then(Value::as_u64) {
            Some(n) if n > 0 => dims[index] = n as f64,
            _ => {
                push(errors, &format!("{path}.{key}"), "expected positive integer millimetres");
                valid = false;
            }
        }
    }
    valid.then_some(dims)
}

fn read_fraction_vector(value: &Value, path: &str, errors: &mut Vec<String>) -> Option<[f64; 3]> {
    if !exact_keys(value, &AXES) {
        push(errors, path, "expected x, y and z fractions");
        return None;
    }
    let mut result = [0.0; 3];
    let mut valid = true;
    for (index, axis) in AXES.iter().enumerate() {
        match value.get(*axis).and_then(Value::as_f64) {
            Some(n) if n.is_finite() && (0.0..=1.0).contains(&n) => result[index] = n,
            _ => {
                push(errors, &format!("{path}.{axis}"), "expected finite fraction in [0,1]");
                valid = false;
            }
        }
    }
    valid.then_some(result)
}

fn read_center_limits(value: &Value, path: &str, errors: &mut Vec<String>) -> Option<[Range; 3]> {
    if !exact_keys(value, &AXES) {
        push(errors, path, "expected x, y and z bounds");
        return None;
    }
    let mut ranges = Vec::new();
    for axis in AXES {
        let item = &value[axis];
        match read_range(item, &format!("{path}.{axis}"), errors) {
            Some(range) if range.lo >= 0.0 && range.hi <= 1.0 => ranges.push(range),
            Some(_) => {
                push(errors, &format!("{path}.{axis}"), "center-of-mass fractions must lie in [0,1]");
                ranges.push(Range { lo: 0.0, hi: 1.0 });
            }
            None => ranges.push(Range { lo: 0.0, hi: 1.0 }),
        }
    }
    Some([
        ranges[0].clone(),
        ranges[1].clone(),
        ranges[2].clone(),
    ])
}

fn read_string_set(
    value: &Value,
    path: &str,
    errors: &mut Vec<String>,
    minimum: usize,
) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    match value.as_array() {
        Some(items) if items.len() >= minimum => {
            for (index, item) in items.iter().enumerate() {
                match item.as_str() {
                    Some(text) if !text.trim().is_empty() => {
                        if !result.insert(text.to_string()) {
                            push(errors, &format!("{path}[{index}]"), "duplicate identifier");
                        }
                    }
                    _ => push(errors, &format!("{path}[{index}]"), "expected non-empty string"),
                }
            }
        }
        Some(_) => push(errors, path, "list is shorter than minimum length"),
        None => push(errors, path, "expected array"),
    }
    result
}

fn read_ids(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Vec<String> {
    let set = read_string_set(value.get(key).unwrap_or(&Value::Null), &format!("{path}.{key}"), errors, 1);
    set.into_iter().collect()
}

fn validate_contract(root: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    const ROOT_KEYS: &[&str] = &[
        "schema_version", "benchmark_id", "provenance", "claim_ceiling",
        "environments", "payload_profiles", "platform_profiles", "mission_scenarios",
        "gate_definitions", "metric_definitions", "assessments",
    ];
    if !exact_keys(root, ROOT_KEYS) {
        push(&mut errors, "root", "missing or unknown fields");
    }
    if root.get("schema_version").and_then(Value::as_str) != Some("0.1") {
        push(&mut errors, "schema_version", "expected 0.1");
    }
    if root.get("benchmark_id").and_then(Value::as_str) != Some("extreme-cargo-mission-contract-v0.1") {
        push(&mut errors, "benchmark_id", "unexpected benchmark identity");
    }
    if root.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING) {
        push(&mut errors, "claim_ceiling", "missing or incompatible claim ceiling");
    }
    let provenance = &root["provenance"];
    if !exact_keys(provenance, &["kind", "external_real_world_data_included", "notes"]) {
        push(&mut errors, "provenance", "missing or unknown fields");
    }
    if provenance.get("kind").and_then(Value::as_str) != Some("synthetic_only") {
        push(&mut errors, "provenance.kind", "only synthetic fixture data are allowed in v0.1");
    }
    if provenance.get("external_real_world_data_included").and_then(Value::as_bool) != Some(false) {
        push(&mut errors, "provenance.external_real_world_data_included", "must be false");
    }
    read_string(provenance, "notes", "provenance", &mut errors);

    let mut environment_ids = BTreeSet::new();
    if let Some(items) = root.get("environments").and_then(Value::as_array) {
        if items.len() < 5 { push(&mut errors, "environments", "expected at least five environments"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("environments[{i}]");
            if !exact_keys(item, &["environment_id", "min_ambient_temp_c", "max_ambient_temp_c"]) {
                push(&mut errors, &path, "missing or unknown fields");
            }
            if let Some(id) = read_string(item, "environment_id", &path, &mut errors) {
                if !environment_ids.insert(id) { push(&mut errors, &path, "duplicate environment ID"); }
            }
            let min = item.get("min_ambient_temp_c").and_then(Value::as_f64);
            let max = item.get("max_ambient_temp_c").and_then(Value::as_f64);
            if !min.is_some_and(f64::is_finite) || !max.is_some_and(f64::is_finite) {
                push(&mut errors, &path, "ambient endpoints must be finite numbers");
            } else if min.unwrap() > max.unwrap() {
                push(&mut errors, &path, "ambient minimum must be <= maximum");
            }
        }
    } else {
        push(&mut errors, "environments", "expected array");
    }

    let mut payload_ids = BTreeSet::new();
    if let Some(items) = root.get("payload_profiles").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "payload_profiles", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("payload_profiles[{i}]");
            const KEYS: &[&str] = &[
                "payload_id", "payload_class", "mass_kg", "dimensions_mm", "center_of_mass_fraction",
                "temperature_range_c", "max_shock_g", "max_tilt_deg", "max_vibration_rms_m_s2",
                "containment_required", "securement_required", "occupants", "occupant_class",
                "restraint_required", "accessible_boarding_required", "wheelchair_securement_required",
                "patient_support_required", "caregiver_access_required", "emergency_egress_required",
                "privacy_sensitive",
            ];
            if !exact_keys(item, KEYS) { push(&mut errors, &path, "missing or unknown fields"); }
            if let Some(id) = read_string(item, "payload_id", &path, &mut errors) {
                if !payload_ids.insert(id) { push(&mut errors, &path, "duplicate payload ID"); }
            }
            let payload_class = item.get("payload_class").and_then(Value::as_str);
            match payload_class {
                Some(class) if PAYLOAD_CLASSES.contains(&class) => {}
                _ => push(&mut errors, &format!("{path}.payload_class"), "unknown payload class"),
            }
            match read_range(&item["mass_kg"], &format!("{path}.mass_kg"), &mut errors) {
                Some(range) if range.lo > 0.0 => {}
                Some(_) => push(&mut errors, &format!("{path}.mass_kg"), "payload mass range must have a strictly positive minimum"),
                None => {}
            }
            read_dimensions(&item["dimensions_mm"], &format!("{path}.dimensions_mm"), &mut errors);
            read_fraction_vector(&item["center_of_mass_fraction"], &format!("{path}.center_of_mass_fraction"), &mut errors);
            read_nullable_range(&item["temperature_range_c"], &format!("{path}.temperature_range_c"), &mut errors);
            for key in ["max_shock_g", "max_tilt_deg", "max_vibration_rms_m_s2"] {
                if let Some(number) = item.get(key).and_then(Value::as_f64) {
                    if !number.is_finite() || number <= 0.0 {
                        push(&mut errors, &format!("{path}.{key}"), "must be positive and finite");
                    }
                    if key == "max_tilt_deg" && number > 90.0 {
                        push(&mut errors, &format!("{path}.{key}"), "must be <= 90 degrees");
                    }
                } else {
                    push(&mut errors, &format!("{path}.{key}"), "expected finite number");
                }
            }
            let bool_fields = [
                "containment_required", "securement_required", "restraint_required",
                "accessible_boarding_required", "wheelchair_securement_required",
                "patient_support_required", "caregiver_access_required",
                "emergency_egress_required", "privacy_sensitive",
            ];
            for key in bool_fields { read_bool(item, key, &path, &mut errors); }

            let occupants = item.get("occupants").and_then(Value::as_u64);
            if occupants.is_none() { push(&mut errors, &format!("{path}.occupants"), "expected non-negative integer"); }
            let occupant_class = item.get("occupant_class").and_then(Value::as_str);
            if !occupant_class.is_some_and(|class| OCCUPANT_CLASSES.contains(&class)) {
                push(&mut errors, &format!("{path}.occupant_class"), "unknown occupant class");
            }
            let human = occupants.unwrap_or(0) > 0;
            let class_is_human = occupant_class.is_some_and(|class| class != "none");
            if human != class_is_human {
                push(&mut errors, &path, "occupant count and occupant_class disagree");
            }
            let expected_occupant_class = match payload_class {
                Some("bulk_cargo" | "cold_chain" | "fragile_instrument" | "contained_liquid") => Some("none"),
                Some("seated_passengers") => Some("seated"),
                Some("wheelchair_user") => Some("wheelchair"),
                Some("stretcher_patient") => Some("stretcher_patient"),
                Some("field_crew") => Some("field_crew"),
                _ => None,
            };
            if expected_occupant_class.is_some() && occupant_class != expected_occupant_class {
                push(&mut errors, &path, "payload_class and occupant_class are incompatible");
            }
            let get_flag = |key: &str| item.get(key).and_then(Value::as_bool) == Some(true);
            if human && (!get_flag("restraint_required") || !get_flag("emergency_egress_required")) {
                push(&mut errors, &path, "human payloads require restraint and emergency-egress declarations");
            }
            if occupant_class == Some("wheelchair")
                && (!get_flag("accessible_boarding_required") || !get_flag("wheelchair_securement_required"))
            {
                push(&mut errors, &path, "wheelchair users require accessible boarding and wheelchair securement");
            }
            if occupant_class == Some("stretcher_patient")
                && (!get_flag("patient_support_required") || !get_flag("caregiver_access_required"))
            {
                push(&mut errors, &path, "stretcher patients require patient support and caregiver access");
            }
        }
    } else {
        push(&mut errors, "payload_profiles", "expected array");
    }

    let mut gate_ids = BTreeSet::new();
    if let Some(items) = root.get("gate_definitions").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "gate_definitions", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("gate_definitions[{i}]");
            if !exact_keys(item, &["gate_id", "meaning", "hard_gate"]) {
                push(&mut errors, &path, "missing or unknown fields");
            }
            if let Some(id) = read_string(item, "gate_id", &path, &mut errors) {
                if !gate_ids.insert(id) { push(&mut errors, &path, "duplicate gate ID"); }
            }
            read_string(item, "meaning", &path, &mut errors);
            if item.get("hard_gate").and_then(Value::as_bool) != Some(true) {
                push(&mut errors, &format!("{path}.hard_gate"), "all mission gates must be hard gates");
            }
        }
    } else {
        push(&mut errors, "gate_definitions", "expected array");
    }

    let mut metric_defs: BTreeMap<String, String> = BTreeMap::new();
    if let Some(items) = root.get("metric_definitions").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "metric_definitions", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("metric_definitions[{i}]");
            if !exact_keys(item, &["metric_id", "unit", "direction", "meaning"]) {
                push(&mut errors, &path, "missing or unknown fields");
            }
            let id = read_string(item, "metric_id", &path, &mut errors);
            let unit = read_string(item, "unit", &path, &mut errors);
            let direction = read_string(item, "direction", &path, &mut errors);
            read_string(item, "meaning", &path, &mut errors);
            if let Some(direction) = direction.as_deref() {
                if !["minimize", "maximize"].contains(&direction) {
                    push(&mut errors, &format!("{path}.direction"), "expected minimize or maximize");
                }
            }
            if let (Some(id), Some(unit), Some(direction)) = (id, unit, direction) {
                if metric_defs.insert(id.clone(), direction).is_some() {
                    push(&mut errors, &path, "duplicate metric ID");
                }
                let _ = unit;
            }
        }
    } else {
        push(&mut errors, "metric_definitions", "expected array");
    }

    let mut platform_ids = BTreeSet::new();
    if let Some(items) = root.get("platform_profiles").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "platform_profiles", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("platform_profiles[{i}]");
            const KEYS: &[&str] = &[
                "platform_id", "mode", "environment_ids", "supported_payload_classes",
                "max_payload_kg", "bay_dimensions_mm", "ambient_temp_min_c", "ambient_temp_max_c",
                "controlled_temp_min_c", "controlled_temp_max_c", "max_payload_shock_g",
                "maximum_safe_route_tilt_deg", "max_payload_vibration_rms_m_s2",
                "center_of_mass_limits_fraction", "has_securement", "has_containment",
                "has_occupant_restraints", "has_accessible_boarding", "has_wheelchair_securement",
                "has_patient_support", "has_caregiver_access", "has_emergency_egress",
                "has_privacy_controls", "has_recovery_plan", "has_emergency_stop",
            ];
            if !exact_keys(item, KEYS) { push(&mut errors, &path, "missing or unknown fields"); }
            if let Some(id) = read_string(item, "platform_id", &path, &mut errors) {
                if !platform_ids.insert(id) { push(&mut errors, &path, "duplicate platform ID"); }
            }
            match item.get("mode").and_then(Value::as_str) {
                Some(mode) if MODES.contains(&mode) => {}
                _ => push(&mut errors, &format!("{path}.mode"), "unknown transport mode"),
            }
            let envs = read_ids(item, "environment_ids", &path, &mut errors);
            for env in envs {
                if !environment_ids.contains(&env) {
                    push(&mut errors, &format!("{path}.environment_ids"), "undeclared environment ID");
                }
            }
            let classes = read_string_set(&item["supported_payload_classes"], &format!("{path}.supported_payload_classes"), &mut errors, 1);
            for class in classes {
                if !PAYLOAD_CLASSES.contains(&class.as_str()) {
                    push(&mut errors, &format!("{path}.supported_payload_classes"), "unknown payload class");
                }
            }
            if let Some(capacity) = item.get("max_payload_kg").and_then(Value::as_f64) {
                if !capacity.is_finite() || capacity <= 0.0 {
                    push(&mut errors, &format!("{path}.max_payload_kg"), "must be positive and finite");
                }
            } else {
                push(&mut errors, &format!("{path}.max_payload_kg"), "expected finite number");
            }
            read_dimensions(&item["bay_dimensions_mm"], &format!("{path}.bay_dimensions_mm"), &mut errors);
            for key in ["ambient_temp_min_c", "ambient_temp_max_c", "max_payload_shock_g", "maximum_safe_route_tilt_deg", "max_payload_vibration_rms_m_s2"] {
                if let Some(number) = item.get(key).and_then(Value::as_f64) {
                    if !number.is_finite() || (key.contains("shock") || key.contains("vibration")) && number <= 0.0 {
                        push(&mut errors, &format!("{path}.{key}"), "invalid numeric value");
                    }
                    if key == "maximum_safe_route_tilt_deg" && !(0.0..=90.0).contains(&number) {
                        push(&mut errors, &format!("{path}.{key}"), "must lie in [0,90]");
                    }
                } else {
                    push(&mut errors, &format!("{path}.{key}"), "expected finite number");
                }
            }
            let amb_min = item.get("ambient_temp_min_c").and_then(Value::as_f64);
            let amb_max = item.get("ambient_temp_max_c").and_then(Value::as_f64);
            if amb_min.is_some_and(f64::is_finite) && amb_max.is_some_and(f64::is_finite) && amb_min.unwrap() > amb_max.unwrap() {
                push(&mut errors, &path, "ambient minimum must be <= maximum");
            }
            let control_min = &item["controlled_temp_min_c"];
            let control_max = &item["controlled_temp_max_c"];
            match (control_min.is_null(), control_max.is_null()) {
                (true, true) => {}
                (false, false) => {
                    let lo = control_min.as_f64();
                    let hi = control_max.as_f64();
                    if !lo.is_some_and(f64::is_finite) || !hi.is_some_and(f64::is_finite) {
                        push(&mut errors, &path, "controlled temperature endpoints must be finite");
                    } else if lo.unwrap() > hi.unwrap() {
                        push(&mut errors, &path, "controlled temperature minimum must be <= maximum");
                    }
                }
                _ => push(&mut errors, &path, "controlled temperature endpoints must both be null or both be numbers"),
            }
            read_center_limits(&item["center_of_mass_limits_fraction"], &format!("{path}.center_of_mass_limits_fraction"), &mut errors);
            for key in [
                "has_securement", "has_containment", "has_occupant_restraints", "has_accessible_boarding",
                "has_wheelchair_securement", "has_patient_support", "has_caregiver_access",
                "has_emergency_egress", "has_privacy_controls", "has_recovery_plan", "has_emergency_stop",
            ] { read_bool(item, key, &path, &mut errors); }
        }
    } else {
        push(&mut errors, "platform_profiles", "expected array");
    }

    let mut mission_ids = BTreeSet::new();
    let mut mission_gate_inventory: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(items) = root.get("mission_scenarios").and_then(Value::as_array) {
        if items.len() < 8 { push(&mut errors, "mission_scenarios", "expected at least eight mission scenarios"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("mission_scenarios[{i}]");
            const KEYS: &[&str] = &[
                "mission_id", "environment_id", "mission_class", "payload_ids", "allowed_modes",
                "required_gate_ids", "route_envelope", "requires_human_transport", "operating_notes",
            ];
            if !exact_keys(item, KEYS) { push(&mut errors, &path, "missing or unknown fields"); }
            let id = read_string(item, "mission_id", &path, &mut errors);
            if let Some(id) = id.as_deref() {
                if !mission_ids.insert(id.to_string()) { push(&mut errors, &path, "duplicate mission ID"); }
            }
            if let Some(env) = read_string(item, "environment_id", &path, &mut errors) {
                if !environment_ids.contains(&env) { push(&mut errors, &format!("{path}.environment_id"), "undeclared environment ID"); }
            }
            read_string(item, "mission_class", &path, &mut errors);
            read_string(item, "operating_notes", &path, &mut errors);
            let payloads = read_string_set(&item["payload_ids"], &format!("{path}.payload_ids"), &mut errors, 1);
            if payloads.len() > 12 { push(&mut errors, &format!("{path}.payload_ids"), "v0.1 bounds exhaustive load-layout checks to at most 12 payloads"); }
            let allowed_modes = read_string_set(&item["allowed_modes"], &format!("{path}.allowed_modes"), &mut errors, 1);
            for mode in &allowed_modes {
                if !MODES.contains(&mode.as_str()) {
                    push(&mut errors, &format!("{path}.allowed_modes"), "unknown transport mode");
                }
            }
            for payload in &payloads {
                if !payload_ids.contains(payload) {
                    push(&mut errors, &format!("{path}.payload_ids"), "undeclared payload ID");
                }
            }
            let gates = read_string_set(&item["required_gate_ids"], &format!("{path}.required_gate_ids"), &mut errors, 1);
            for gate in &gates {
                if !gate_ids.contains(gate) { push(&mut errors, &format!("{path}.required_gate_ids"), "undeclared gate ID"); }
            }
            for always_required in [
                "route_or_waterway_authorization",
                "environmental_protection",
                "recovery_plan",
                "payload_layout_and_load_distribution",
            ] {
                if !gates.contains(always_required) {
                    push(&mut errors, &format!("{path}.required_gate_ids"), &format!("mandatory mission gate {always_required} is missing"));
                }
            }
            let mission_payload_records: Vec<&Value> = root.get("payload_profiles")
                .and_then(Value::as_array)
                .map(|profiles| profiles.iter()
                    .filter(|payload| payload["payload_id"].as_str().is_some_and(|pid| payloads.contains(pid)))
                    .collect())
                .unwrap_or_default();
            let has_human_payload = mission_payload_records.iter()
                .any(|payload| payload["occupants"].as_u64().unwrap_or(0) > 0);
            let requires_human = item.get("requires_human_transport").and_then(Value::as_bool).unwrap_or(false);
            if has_human_payload != requires_human {
                push(&mut errors, &path, "requires_human_transport must match the mission's occupant payloads");
            }
            let needs_securement = mission_payload_records.iter()
                .any(|payload| payload["securement_required"].as_bool() == Some(true));
            if needs_securement && !gates.contains("payload_securement") {
                push(&mut errors, &path, "securement-required payload is missing the payload_securement gate");
            }
            let needs_condition_control = mission_payload_records.iter()
                .any(|payload| !payload["temperature_range_c"].is_null());
            if needs_condition_control && !gates.contains("condition_control") {
                push(&mut errors, &path, "temperature-constrained payload is missing condition_control");
            }
            let needs_accessibility = mission_payload_records.iter()
                .any(|payload| payload["accessible_boarding_required"].as_bool() == Some(true));
            if needs_accessibility && !gates.contains("accessible_transfer") {
                push(&mut errors, &path, "accessible passenger mission is missing accessible_transfer");
            }
            let needs_patient_care = mission_payload_records.iter()
                .any(|payload| payload["patient_support_required"].as_bool() == Some(true)
                    || payload["caregiver_access_required"].as_bool() == Some(true));
            if needs_patient_care && !gates.contains("patient_care_access") {
                push(&mut errors, &path, "patient payload is missing patient_care_access");
            }
            let needs_privacy = mission_payload_records.iter()
                .any(|payload| payload["privacy_sensitive"].as_bool() == Some(true));
            if needs_privacy && !gates.contains("privacy_and_data_minimization") {
                push(&mut errors, &path, "privacy-sensitive payload is missing privacy_and_data_minimization");
            }
            if has_human_payload && !gates.contains("occupant_restraints_and_egress") {
                push(&mut errors, &path, "human transport is missing occupant_restraints_and_egress");
            }
            if has_human_payload && !gates.contains("human_supervision_and_stop") {
                push(&mut errors, &path, "human transport is missing human_supervision_and_stop");
            }
            if item["environment_id"].as_str() == Some("afghanistan")
                && !gates.contains("explosive_hazard_route_evidence") {
                push(&mut errors, &path, "Afghanistan mission is missing the explosive-hazard route evidence gate");
            }
            if let Some(id) = id { mission_gate_inventory.insert(id, gates); }
            let route = &item["route_envelope"];
            if !exact_keys(route, &["min_ambient_temp_c", "max_ambient_temp_c", "required_route_tilt_deg"]) {
                push(&mut errors, &format!("{path}.route_envelope"), "missing or unknown fields");
            }
            let route_min = route.get("min_ambient_temp_c").and_then(Value::as_f64);
            let route_max = route.get("max_ambient_temp_c").and_then(Value::as_f64);
            if !route_min.is_some_and(f64::is_finite) || !route_max.is_some_and(f64::is_finite) {
                push(&mut errors, &format!("{path}.route_envelope"), "ambient endpoints must be finite");
            } else if route_min.unwrap() > route_max.unwrap() {
                push(&mut errors, &format!("{path}.route_envelope"), "ambient minimum must be <= maximum");
            } else if let Some(env_profile) = root["environments"].as_array().and_then(|items| items.iter()
                .find(|env| env["environment_id"].as_str() == item["environment_id"].as_str())) {
                let env_min = env_profile["min_ambient_temp_c"].as_f64();
                let env_max = env_profile["max_ambient_temp_c"].as_f64();
                if env_min.is_some_and(f64::is_finite) && env_max.is_some_and(f64::is_finite)
                    && (route_min.unwrap() < env_min.unwrap() || route_max.unwrap() > env_max.unwrap()) {
                    push(&mut errors, &format!("{path}.route_envelope"), "mission route temperatures must sit within the declared environment envelope");
                }
            }
            match route.get("required_route_tilt_deg").and_then(Value::as_f64) {
                Some(deg) if deg.is_finite() && (0.0..=90.0).contains(&deg) => {}
                _ => push(&mut errors, &format!("{path}.route_envelope.required_route_tilt_deg"), "expected value in [0,90]"),
            }
            if item.get("requires_human_transport").and_then(Value::as_bool).is_none() {
                push(&mut errors, &format!("{path}.requires_human_transport"), "expected boolean");
            }
            let _ = read_bool(item, "requires_human_transport", &path, &mut errors);
            // All mission gates are non-tradeable; route, payload and person-related gates are explicitly enumerated.
        }
    } else {
        push(&mut errors, "mission_scenarios", "expected array");
    }

    if let Some(items) = root.get("assessments").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "assessments", "must not be empty"); }
        let mut ids = BTreeSet::new();
        for (i, item) in items.iter().enumerate() {
            let path = format!("assessments[{i}]");
            if !exact_keys(item, &["assessment_id", "mission_id", "platform_id", "gate_results", "metric_intervals", "load_layout"]) {
                push(&mut errors, &path, "missing or unknown fields");
            }
            if let Some(id) = read_string(item, "assessment_id", &path, &mut errors) {
                if !ids.insert(id) { push(&mut errors, &path, "duplicate assessment ID"); }
            }
            let mission_id = read_string(item, "mission_id", &path, &mut errors);
            if !mission_id.as_ref().is_some_and(|id| mission_ids.contains(id)) {
                push(&mut errors, &format!("{path}.mission_id"), "undeclared mission ID");
            }
            if !read_string(item, "platform_id", &path, &mut errors)
                .as_ref().is_some_and(|id| platform_ids.contains(id)) {
                push(&mut errors, &format!("{path}.platform_id"), "undeclared platform ID");
            }
            let required_gates = mission_id.as_ref().and_then(|id| mission_gate_inventory.get(id)).cloned().unwrap_or_default();
            let mut seen_gates = BTreeSet::new();
            if let Some(gate_results) = item.get("gate_results").and_then(Value::as_array) {
                for (j, result) in gate_results.iter().enumerate() {
                    let gp = format!("{path}.gate_results[{j}]");
                    if !exact_keys(result, &["gate_id", "status"]) { push(&mut errors, &gp, "missing or unknown fields"); }
                    if let Some(gate_id) = read_string(result, "gate_id", &gp, &mut errors) {
                        if !gate_ids.contains(&gate_id) { push(&mut errors, &gp, "undeclared gate ID"); }
                        if !required_gates.contains(&gate_id) { push(&mut errors, &gp, "gate is not required by this mission"); }
                        if !seen_gates.insert(gate_id) { push(&mut errors, &gp, "duplicate gate result"); }
                    }
                    match result.get("status").and_then(Value::as_str) {
                        Some("pass" | "fail" | "unknown") => {}
                        _ => push(&mut errors, &format!("{gp}.status"), "expected pass, fail, or unknown"),
                    }
                }
            } else {
                push(&mut errors, &format!("{path}.gate_results"), "expected array");
            }
            if seen_gates != required_gates {
                push(&mut errors, &format!("{path}.gate_results"), "must contain every required mission gate exactly once");
            }

            let mut seen_metrics = BTreeSet::new();
            if let Some(records) = item.get("metric_intervals").and_then(Value::as_array) {
                for (j, record) in records.iter().enumerate() {
                    let mp = format!("{path}.metric_intervals[{j}]");
                    if !exact_keys(record, &["metric_id", "unit", "status", "lower", "upper"]) {
                        push(&mut errors, &mp, "missing or unknown fields");
                    }
                    let metric_id = read_string(record, "metric_id", &mp, &mut errors);
                    let unit = read_string(record, "unit", &mp, &mut errors);
                    if let Some(id) = metric_id.as_deref() {
                        if !metric_defs.contains_key(id) { push(&mut errors, &mp, "undeclared metric ID"); }
                        if !seen_metrics.insert(id.to_string()) { push(&mut errors, &mp, "duplicate metric ID"); }
                    }
                    if let Some(id) = metric_id.as_deref() {
                        let expected_unit = root["metric_definitions"].as_array().and_then(|defs| {
                            defs.iter().find(|d| d["metric_id"].as_str() == Some(id))
                                .and_then(|d| d["unit"].as_str())
                        });
                        if unit.as_deref() != expected_unit {
                            push(&mut errors, &format!("{mp}.unit"), "unit does not match metric definition");
                        }
                    }
                    match record.get("status").and_then(Value::as_str) {
                        Some("synthetic_interval") => {
                            let lo = record.get("lower").and_then(Value::as_f64);
                            let hi = record.get("upper").and_then(Value::as_f64);
                            match (lo, hi) {
                                (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() && lo <= hi => {}
                                (Some(lo), Some(hi)) if !lo.is_finite() || !hi.is_finite() => push(&mut errors, &mp, "interval endpoints must be finite"),
                                (Some(_), Some(_)) => push(&mut errors, &mp, "lower endpoint must be <= upper endpoint"),
                                _ => push(&mut errors, &mp, "synthetic interval requires numeric endpoints"),
                            }
                        }
                        Some("unknown") => {
                            if !record.get("lower").is_some_and(Value::is_null) || !record.get("upper").is_some_and(Value::is_null) {
                                push(&mut errors, &mp, "unknown interval endpoints must be null");
                            }
                        }
                        _ => push(&mut errors, &format!("{mp}.status"), "expected synthetic_interval or unknown"),
                    }
                }
            } else {
                push(&mut errors, &format!("{path}.metric_intervals"), "expected array");
            }
            let expected_metrics: BTreeSet<String> = metric_defs.keys().cloned().collect();
            if seen_metrics != expected_metrics {
                push(&mut errors, &format!("{path}.metric_intervals"), "must contain every declared metric exactly once");
            }

            let required_payloads: BTreeSet<String> = mission_id.as_ref()
                .and_then(|id| root["mission_scenarios"].as_array().and_then(|ms| ms.iter().find(|m| m["mission_id"].as_str() == Some(id.as_str()))))
                .map(|m| read_string_set(&m["payload_ids"], &format!("{path}.mission.payload_ids"), &mut Vec::new(), 1))
                .unwrap_or_default();
            let mut placed = BTreeSet::new();
            if let Some(layout) = item.get("load_layout").and_then(Value::as_array) {
                if layout.is_empty() { push(&mut errors, &format!("{path}.load_layout"), "must not be empty"); }
                for (j, place) in layout.iter().enumerate() {
                    let lp = format!("{path}.load_layout[{j}]");
                    if !exact_keys(place, &["payload_id", "position_mm"]) { push(&mut errors, &lp, "missing or unknown fields"); }
                    if let Some(pid) = read_string(place, "payload_id", &lp, &mut errors) {
                        if !required_payloads.contains(&pid) { push(&mut errors, &lp, "payload is not part of this mission"); }
                        if !placed.insert(pid) { push(&mut errors, &lp, "duplicate payload placement"); }
                    }
                    let pos = &place["position_mm"];
                    if !exact_keys(pos, &AXES) { push(&mut errors, &format!("{lp}.position_mm"), "expected x, y and z"); }
                    for axis in AXES {
                        match pos.get(axis).and_then(Value::as_u64) {
                            Some(_) => {}
                            None => push(&mut errors, &format!("{lp}.position_mm.{axis}"), "expected non-negative integer millimetres"),
                        }
                    }
                }
            } else {
                push(&mut errors, &format!("{path}.load_layout"), "expected array");
            }
            if placed != required_payloads {
                push(&mut errors, &format!("{path}.load_layout"), "must position each mission payload exactly once");
            }
        }
    } else {
        push(&mut errors, "assessments", "expected array");
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        Err(errors)
    }
}

fn range_value(value: &Value) -> Range {
    Range { lo: value["min"].as_f64().unwrap(), hi: value["max"].as_f64().unwrap() }
}

fn nullable_range_value(value: &Value) -> Option<Range> {
    if value.is_null() { None } else { Some(range_value(value)) }
}

fn dims_value(value: &Value) -> [f64; 3] {
    [
        value["length"].as_u64().unwrap() as f64,
        value["width"].as_u64().unwrap() as f64,
        value["height"].as_u64().unwrap() as f64,
    ]
}

fn center_value(value: &Value) -> [f64; 3] {
    [value["x"].as_f64().unwrap(), value["y"].as_f64().unwrap(), value["z"].as_f64().unwrap()]
}

fn bool_value(value: &Value, key: &str) -> bool {
    value[key].as_bool().unwrap()
}

fn number_value(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap()
}

fn payload_from(value: &Value) -> Payload {
    Payload {
        id: value["payload_id"].as_str().unwrap().to_string(),
        class: value["payload_class"].as_str().unwrap().to_string(),
        mass: range_value(&value["mass_kg"]),
        dims: dims_value(&value["dimensions_mm"]),
        center_fraction: center_value(&value["center_of_mass_fraction"]),
        temperature: nullable_range_value(&value["temperature_range_c"]),
        shock_g: number_value(value, "max_shock_g"),
        tilt_deg: number_value(value, "max_tilt_deg"),
        vibration: number_value(value, "max_vibration_rms_m_s2"),
        containment: bool_value(value, "containment_required"),
        securement: bool_value(value, "securement_required"),
        occupants: value["occupants"].as_u64().unwrap(),
        occupant_class: value["occupant_class"].as_str().unwrap().to_string(),
        restraints: bool_value(value, "restraint_required"),
        accessible: bool_value(value, "accessible_boarding_required"),
        wheelchair_securement: bool_value(value, "wheelchair_securement_required"),
        patient_support: bool_value(value, "patient_support_required"),
        caregiver_access: bool_value(value, "caregiver_access_required"),
        egress: bool_value(value, "emergency_egress_required"),
        privacy_sensitive: bool_value(value, "privacy_sensitive"),
    }
}

fn platform_from(value: &Value) -> Platform {
    let read_set = |key: &str| -> BTreeSet<String> {
        value[key].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
    };
    let controlled_temperature = match (&value["controlled_temp_min_c"], &value["controlled_temp_max_c"]) {
        (lo, hi) if lo.is_null() && hi.is_null() => None,
        (lo, hi) => Some(Range { lo: lo.as_f64().unwrap(), hi: hi.as_f64().unwrap() }),
    };
    let limits = read_center_limits(&value["center_of_mass_limits_fraction"], "ignored", &mut Vec::new()).unwrap();
    Platform {
        id: value["platform_id"].as_str().unwrap().to_string(),
        mode: value["mode"].as_str().unwrap().to_string(),
        environments: read_set("environment_ids"),
        payload_classes: read_set("supported_payload_classes"),
        max_payload_kg: number_value(value, "max_payload_kg"),
        bay: dims_value(&value["bay_dimensions_mm"]),
        ambient: Range {
            lo: number_value(value, "ambient_temp_min_c"),
            hi: number_value(value, "ambient_temp_max_c"),
        },
        controlled_temperature,
        max_shock_g: number_value(value, "max_payload_shock_g"),
        maximum_safe_route_tilt_deg: number_value(value, "maximum_safe_route_tilt_deg"),
        max_vibration: number_value(value, "max_payload_vibration_rms_m_s2"),
        center_limits: limits,
        securement: bool_value(value, "has_securement"),
        containment: bool_value(value, "has_containment"),
        occupant_restraints: bool_value(value, "has_occupant_restraints"),
        accessible_boarding: bool_value(value, "has_accessible_boarding"),
        wheelchair_securement: bool_value(value, "has_wheelchair_securement"),
        patient_support: bool_value(value, "has_patient_support"),
        caregiver_access: bool_value(value, "has_caregiver_access"),
        egress: bool_value(value, "has_emergency_egress"),
        privacy_controls: bool_value(value, "has_privacy_controls"),
        recovery_plan: bool_value(value, "has_recovery_plan"),
        emergency_stop: bool_value(value, "has_emergency_stop"),
    }
}

fn mission_from(value: &Value) -> Mission {
    let required_gates = value["required_gate_ids"].as_array().unwrap().iter()
        .map(|item| item.as_str().unwrap().to_string()).collect();
    Mission {
        id: value["mission_id"].as_str().unwrap().to_string(),
        environment: value["environment_id"].as_str().unwrap().to_string(),
        class: value["mission_class"].as_str().unwrap().to_string(),
        payload_ids: value["payload_ids"].as_array().unwrap().iter()
            .map(|item| item.as_str().unwrap().to_string()).collect(),
        allowed_modes: value["allowed_modes"].as_array().unwrap().iter()
            .map(|item| item.as_str().unwrap().to_string()).collect(),
        required_gates,
        ambient: Range {
            lo: number_value(&value["route_envelope"], "min_ambient_temp_c"),
            hi: number_value(&value["route_envelope"], "max_ambient_temp_c"),
        },
        required_route_tilt_deg: number_value(&value["route_envelope"], "required_route_tilt_deg"),
        human_transport: bool_value(value, "requires_human_transport"),
    }
}

fn platform_mismatches(
    mission: &Mission,
    platform: &Platform,
    payloads: &BTreeMap<String, Payload>,
    layout: &BTreeMap<String, [f64; 3]>,
) -> Vec<String> {
    let mut findings = BTreeSet::new();
    if !platform.environments.contains(&mission.environment) {
        findings.insert("platform_not_declared_for_environment".to_string());
    }
    if !mission.allowed_modes.contains(&platform.mode) {
        findings.insert(format!("transport_mode_not_allowed:{}", platform.mode));
    }
    if mission.ambient.lo < platform.ambient.lo || mission.ambient.hi > platform.ambient.hi {
        findings.insert("mission_ambient_outside_platform_envelope".to_string());
    }
    if mission.required_route_tilt_deg > platform.maximum_safe_route_tilt_deg {
        findings.insert("route_tilt_exceeds_platform_capability".to_string());
    }

    let mut payload_refs = Vec::new();
    let mut total_mass_max = 0.0;
    for payload_id in &mission.payload_ids {
        let payload = &payloads[payload_id];
        payload_refs.push(payload);
        total_mass_max += payload.mass.hi;
        if !platform.payload_classes.contains(&payload.class) {
            findings.insert(format!("unsupported_payload_class:{payload_id}"));
        }
        for axis in 0..3 {
            if payload.dims[axis] > platform.bay[axis] {
                findings.insert(format!("payload_exceeds_bay_dimension:{}:{}", payload_id, DIMENSIONS[axis]));
            }
            let position = layout[payload_id][axis];
            if position + payload.dims[axis] > platform.bay[axis] {
                findings.insert(format!("load_outside_bay:{}:{}", payload_id, AXES[axis]));
            }
        }
        if platform.max_shock_g > payload.shock_g {
            findings.insert(format!("shock_limit_exceeded:{payload_id}"));
        }
        if platform.max_vibration > payload.vibration {
            findings.insert(format!("vibration_limit_exceeded:{payload_id}"));
        }
        if mission.required_route_tilt_deg > payload.tilt_deg {
            findings.insert(format!("payload_tilt_limit_exceeded:{payload_id}"));
        }
        if let Some(allowed) = &payload.temperature {
            match &platform.controlled_temperature {
                Some(control) if control.lo >= allowed.lo && control.hi <= allowed.hi => {}
                _ => {
                    findings.insert(format!("temperature_control_incompatible:{payload_id}"));
                }
            };
        }
        if payload.containment && !platform.containment {
            findings.insert(format!("containment_unavailable:{payload_id}"));
        }
        if payload.securement && !platform.securement {
            findings.insert(format!("securement_unavailable:{payload_id}"));
        }
        if payload.occupants > 0 && (!platform.occupant_restraints || !platform.egress) {
            findings.insert(format!("occupant_restraint_or_egress_missing:{payload_id}"));
        }
        if payload.accessible && !platform.accessible_boarding {
            findings.insert(format!("accessible_boarding_unavailable:{payload_id}"));
        }
        if payload.wheelchair_securement && !platform.wheelchair_securement {
            findings.insert(format!("wheelchair_securement_unavailable:{payload_id}"));
        }
        if payload.patient_support && !platform.patient_support {
            findings.insert(format!("patient_support_unavailable:{payload_id}"));
        }
        if payload.caregiver_access && !platform.caregiver_access {
            findings.insert(format!("caregiver_access_unavailable:{payload_id}"));
        }
        if payload.egress && !platform.egress {
            findings.insert(format!("emergency_egress_unavailable:{payload_id}"));
        }
        if payload.privacy_sensitive && !platform.privacy_controls {
            findings.insert(format!("privacy_controls_unavailable:{payload_id}"));
        }
    }
    if total_mass_max > platform.max_payload_kg {
        findings.insert("combined_payload_mass_exceeds_capacity".to_string());
    }
    if !platform.recovery_plan {
        findings.insert("recovery_plan_unavailable".to_string());
    }
    if !platform.emergency_stop {
        findings.insert("emergency_stop_unavailable".to_string());
    }

    // Axis-aligned box checks are deliberately conservative in v0.1. Rotation,
    // deformable packages and load-by-load packing optimization require a new,
    // explicit layout schema rather than hidden assumptions.
    for i in 0..payload_refs.len() {
        for j in (i + 1)..payload_refs.len() {
            let a = payload_refs[i];
            let b = payload_refs[j];
            let apos = layout[&a.id];
            let bpos = layout[&b.id];
            let overlap = (0..3).all(|axis| {
                apos[axis] < bpos[axis] + b.dims[axis]
                    && bpos[axis] < apos[axis] + a.dims[axis]
            });
            if overlap {
                findings.insert(format!("load_layout_overlap:{}:{}", a.id, b.id));
            }
        }
    }

    // Evaluate all endpoint combinations of mass intervals. This bounds the
    // combined centre of mass without inventing a probability distribution.
    if payload_refs.len() <= 12 && platform.bay.iter().all(|v| *v > 0.0) {
        let combinations = 1usize << payload_refs.len();
        for mask in 0..combinations {
            let masses: Vec<f64> = payload_refs.iter().enumerate().map(|(i, payload)| {
                if mask & (1usize << i) == 0 { payload.mass.lo } else { payload.mass.hi }
            }).collect();
            let total: f64 = masses.iter().sum();
            if total <= 0.0 { continue; }
            for axis in 0..3 {
                let weighted: f64 = payload_refs.iter().enumerate().map(|(i, payload)| {
                    let center = layout[&payload.id][axis] + payload.dims[axis] * payload.center_fraction[axis];
                    masses[i] * center
                }).sum();
                let fraction = weighted / total / platform.bay[axis];
                if fraction < platform.center_limits[axis].lo - 1e-9
                    || fraction > platform.center_limits[axis].hi + 1e-9 {
                    findings.insert(format!("combined_center_of_mass_outside_envelope:{}", AXES[axis]));
                    break;
                }
            }
        }
    } else {
        findings.insert("combined_center_of_mass_not_assessed".to_string());
    }
    findings.into_iter().collect()
}

fn dominates_robust(
    a: &Evaluated,
    b: &Evaluated,
    metric_directions: &BTreeMap<String, String>,
) -> bool {
    let mut strict = false;
    for (metric_id, direction) in metric_directions {
        let (Some(Some(a)), Some(Some(b))) = (a.metrics.get(metric_id), b.metrics.get(metric_id)) else {
            return false;
        };
        match direction.as_str() {
            "minimize" => {
                if a.hi > b.lo { return false; }
                if a.hi < b.lo { strict = true; }
            }
            "maximize" => {
                if a.lo < b.hi { return false; }
                if a.lo > b.hi { strict = true; }
            }
            _ => return false,
        }
    }
    strict
}

fn evaluate(root: &Value) -> Result<Value, Vec<String>> {
    validate_contract(root)?;
    let payloads: BTreeMap<String, Payload> = root["payload_profiles"].as_array().unwrap().iter()
        .map(payload_from).map(|p| (p.id.clone(), p)).collect();
    let platforms: BTreeMap<String, Platform> = root["platform_profiles"].as_array().unwrap().iter()
        .map(platform_from).map(|p| (p.id.clone(), p)).collect();
    let missions: BTreeMap<String, Mission> = root["mission_scenarios"].as_array().unwrap().iter()
        .map(mission_from).map(|m| (m.id.clone(), m)).collect();
    let metric_directions: BTreeMap<String, String> = root["metric_definitions"].as_array().unwrap().iter()
        .map(|m| (m["metric_id"].as_str().unwrap().to_string(), m["direction"].as_str().unwrap().to_string())).collect();

    let mut assessed = Vec::new();
    for item in root["assessments"].as_array().unwrap() {
        let id = item["assessment_id"].as_str().unwrap().to_string();
        let mission_id = item["mission_id"].as_str().unwrap().to_string();
        let platform_id = item["platform_id"].as_str().unwrap().to_string();
        let mission = &missions[&mission_id];
        let platform = &platforms[&platform_id];
        let mut failed_gates = BTreeSet::new();
        let mut unknown_gates = BTreeSet::new();
        for result in item["gate_results"].as_array().unwrap() {
            let gate_id = result["gate_id"].as_str().unwrap().to_string();
            match result["status"].as_str().unwrap() {
                "fail" => { failed_gates.insert(gate_id); }
                "unknown" => { unknown_gates.insert(gate_id); }
                _ => {}
            }
        }
        let layout: BTreeMap<String, [f64; 3]> = item["load_layout"].as_array().unwrap().iter().map(|entry| {
            let position = &entry["position_mm"];
            (entry["payload_id"].as_str().unwrap().to_string(), [
                position["x"].as_u64().unwrap() as f64,
                position["y"].as_u64().unwrap() as f64,
                position["z"].as_u64().unwrap() as f64,
            ])
        }).collect();
        let mismatch_reasons = platform_mismatches(mission, platform, &payloads, &layout);
        let mut metrics = BTreeMap::new();
        let mut unknown_metrics = BTreeSet::new();
        for record in item["metric_intervals"].as_array().unwrap() {
            let metric_id = record["metric_id"].as_str().unwrap().to_string();
            let status = record["status"].as_str().unwrap();
            let interval = if status == "synthetic_interval" {
                Some(Range { lo: record["lower"].as_f64().unwrap(), hi: record["upper"].as_f64().unwrap() })
            } else {
                unknown_metrics.insert(metric_id.clone());
                None
            };
            metrics.insert(metric_id, interval);
        }
        let metrics_complete = metrics.values().all(Option::is_some)
            && metrics.len() == metric_directions.len();
        let disposition = if !failed_gates.is_empty() {
            "blocked_hard_gate"
        } else if !mismatch_reasons.is_empty() {
            "blocked_platform_mismatch"
        } else if !unknown_gates.is_empty() {
            "needs_gate_evidence"
        } else if !metrics_complete {
            "not_assessable_incomplete_metrics"
        } else {
            "eligible_for_human_review"
        }.to_string();
        assessed.push(Evaluated {
            id, mission_id, platform_id, disposition,
            failed_gates: failed_gates.into_iter().collect(),
            unknown_gates: unknown_gates.into_iter().collect(),
            mismatch_reasons,
            unknown_metrics: unknown_metrics.into_iter().collect(),
            metrics,
            layout,
        });
    }
    assessed.sort_by(|a, b| a.id.cmp(&b.id));

    let mut robust_frontier_by_mission: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for candidate in &assessed {
        if candidate.disposition != "eligible_for_human_review" { continue; }
        let others = assessed.iter().filter(|other| {
            other.mission_id == candidate.mission_id
                && other.id != candidate.id
                && other.disposition == "eligible_for_human_review"
        });
        let dominated = others.into_iter().any(|other| {
            dominates_robust(other, candidate, &metric_directions)
        });
        if !dominated {
            robust_frontier_by_mission.entry(candidate.mission_id.clone()).or_default()
                .insert(candidate.id.clone());
        }
    }

    let report_items: Vec<Value> = assessed.iter().map(|candidate| {
        let metric_intervals: BTreeMap<String, Value> = candidate.metrics.iter().map(|(id, range)| {
            let value = range.as_ref().map(|r| json!({"lower":r.lo,"upper":r.hi})).unwrap_or(Value::Null);
            (id.clone(), value)
        }).collect();
        json!({
            "assessment_id": candidate.id,
            "mission_id": candidate.mission_id,
            "platform_id": candidate.platform_id,
            "readiness_disposition": candidate.disposition,
            "failed_gate_ids": candidate.failed_gates,
            "unknown_gate_ids": candidate.unknown_gates,
            "platform_mismatch_reasons": candidate.mismatch_reasons,
            "unknown_metric_ids": candidate.unknown_metrics,
            "metric_intervals": metric_intervals,
            "load_layout_payload_ids": candidate.layout.keys().cloned().collect::<Vec<_>>(),
            "on_robust_pareto_frontier": robust_frontier_by_mission.get(&candidate.mission_id).is_some_and(|set| set.contains(&candidate.id)),
            "operational_authorization_granted": false,
            "patient_care_authorization_granted": false,
            "physical_or_clinical_qualification_claimed": false
        })
    }).collect();

    let scenarios: Vec<Value> = missions.values().map(|mission| {
        let items: Vec<&Evaluated> = assessed.iter().filter(|item| item.mission_id == mission.id).collect();
        let count = |state: &str| items.iter().filter(|item| item.disposition == state).count();
        let frontier: Vec<String> = robust_frontier_by_mission.get(&mission.id)
            .map(|set| set.iter().cloned().collect()).unwrap_or_default();
        json!({
            "mission_id": mission.id,
            "environment_id": mission.environment,
            "mission_class": mission.class,
            "human_transport_included": mission.human_transport,
            "candidate_count": items.len(),
            "eligible_for_human_review_count": count("eligible_for_human_review"),
            "blocked_hard_gate_count": count("blocked_hard_gate"),
            "blocked_platform_mismatch_count": count("blocked_platform_mismatch"),
            "needs_gate_evidence_count": count("needs_gate_evidence"),
            "not_assessable_incomplete_metrics_count": count("not_assessable_incomplete_metrics"),
            "robust_pareto_frontier_assessment_ids": frontier,
            "universal_winner_produced": false
        })
    }).collect();

    let count = |state: &str| assessed.iter().filter(|item| item.disposition == state).count();
    Ok(json!({
        "report_version": "0.1",
        "benchmark_id": "extreme-cargo-mission-contract-v0.1",
        "claim_ceiling": CLAIM_CEILING,
        "payload_profile_count": payloads.len(),
        "platform_profile_count": platforms.len(),
        "mission_scenario_count": missions.len(),
        "assessment_count": assessed.len(),
        "eligible_for_human_review_count": count("eligible_for_human_review"),
        "blocked_hard_gate_count": count("blocked_hard_gate"),
        "blocked_platform_mismatch_count": count("blocked_platform_mismatch"),
        "needs_gate_evidence_count": count("needs_gate_evidence"),
        "not_assessable_incomplete_metrics_count": count("not_assessable_incomplete_metrics"),
        "mission_summaries": scenarios,
        "assessment_reports": report_items,
        "weighted_utility_score_produced": false,
        "universal_winner_produced": false,
        "operational_authorization_granted": false,
        "patient_care_authorization_granted": false,
        "autonomous_hazard_handling_authorized": false,
        "interpretation": "Candidate eligibility means only a synthetic payload/platform contract fit plus complete declared passing gates and metric intervals. It is not field evidence, a vehicle recommendation, medical advice, route clearance, physical qualification or deployment authorization. Load boxes are axis-aligned in v0.1; no rotations or deformable packing are inferred."
    }))
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        "fixtures/extreme_environment/extreme_cargo_mission_scenarios_v0_1.json".to_string()
    });
    if args.next().is_some() {
        eprintln!("usage: extreme-cargo-mission-audit [path/to/fixture.json]");
        std::process::exit(2);
    }
    let source = if path == "-" {
        CORPUS.to_string()
    } else if Path::new(&path).exists() {
        match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("FAIL: unable to read {path}: {error}");
                std::process::exit(2);
            }
        }
    } else {
        eprintln!("FAIL: file not found: {path}");
        std::process::exit(2);
    };
    let input: Value = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("FAIL: invalid JSON: {error}");
            std::process::exit(2);
        }
    };
    match evaluate(&input) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("report serializes")),
        Err(errors) => {
            eprintln!("FAIL: {} contract finding(s)", errors.len());
            for error in errors { eprintln!("- {error}"); }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(CORPUS).expect("embedded fixture parses")
    }

    fn report(root: &Value) -> Value {
        evaluate(root).expect("fixture should evaluate")
    }

    fn get_assessment_mut<'a>(root: &'a mut Value, id: &str) -> &'a mut Value {
        root["assessments"].as_array_mut().unwrap().iter_mut()
            .find(|item| item["assessment_id"] == id).unwrap()
    }

    fn disposition(root: &Value, id: &str) -> String {
        report(root)["assessment_reports"].as_array().unwrap().iter()
            .find(|item| item["assessment_id"] == id).unwrap()["readiness_disposition"]
            .as_str().unwrap().to_string()
    }

    #[test]
    fn fixture_baseline_distinguishes_eligible_mismatch_hard_stop_and_unknown_route() {
        let output = report(&fixture());
        assert_eq!(output["payload_profile_count"], 8);
        assert_eq!(output["platform_profile_count"], 11);
        assert_eq!(output["mission_scenario_count"], 10);
        assert_eq!(output["assessment_count"], 21);
        assert_eq!(output["eligible_for_human_review_count"], 9);
        assert_eq!(output["blocked_hard_gate_count"], 8);
        assert_eq!(output["blocked_platform_mismatch_count"], 2);
        assert_eq!(output["needs_gate_evidence_count"], 2);
        assert_eq!(output["not_assessable_incomplete_metrics_count"], 0);
        assert_eq!(output["operational_authorization_granted"], false);
        assert_eq!(output["patient_care_authorization_granted"], false);
    }

    #[test]
    fn passenger_transport_requires_accessibility_restraints_and_egress() {
        let root = fixture();
        assert_eq!(disposition(&root, "amz-accessible-passengers-01-riverlink_accessible_ferry"), "eligible_for_human_review");
        assert_eq!(disposition(&root, "amz-accessible-passengers-01-riverlink_cargo_boat"), "blocked_hard_gate");
    }

    #[test]
    fn cargo_fits_geometry_mass_environment_and_stability_not_only_payload_rating() {
        let mut root = fixture();
        let a = get_assessment_mut(&mut root, "amz-coldchain-01-riverlink_cargo_boat");
        a["load_layout"][1]["position_mm"]["x"] = json!(0);
        let output = report(&root);
        let row = output["assessment_reports"].as_array().unwrap().iter()
            .find(|item| item["assessment_id"] == "amz-coldchain-01-riverlink_cargo_boat").unwrap();
        assert_eq!(row["readiness_disposition"], "blocked_platform_mismatch");
        assert!(row["platform_mismatch_reasons"].as_array().unwrap().iter()
            .any(|v| v.as_str().unwrap().starts_with("load_layout_overlap:")));
    }

    #[test]
    fn over_capacity_fails_even_if_other_metrics_are_attractive() {
        let mut root = fixture();
        root["payload_profiles"].as_array_mut().unwrap().iter_mut()
            .find(|p| p["payload_id"] == "bulk_medical_supplies").unwrap()["mass_kg"]["max"] = json!(1000.0);
        assert_eq!(disposition(&root, "amz-coldchain-01-riverlink_cargo_boat"), "blocked_platform_mismatch");
    }

    #[test]
    fn unknown_route_or_explosive_hazard_evidence_never_becomes_authorization() {
        let root = fixture();
        assert_eq!(disposition(&root, "afghanistan-medical-supply-01-aidrover_supply"), "needs_gate_evidence");
        let output = report(&root);
        assert_eq!(output["operational_authorization_granted"], false);
        assert_eq!(output["autonomous_hazard_handling_authorized"], false);
    }

    #[test]
    fn unknown_metric_is_not_imputed_to_zero() {
        let mut root = fixture();
        let item = get_assessment_mut(&mut root, "congo-science-samples-01-congo_low_disturbance_rover");
        let metric = item["metric_intervals"].as_array_mut().unwrap().iter_mut()
            .find(|m| m["metric_id"] == "energy_per_mission_wh").unwrap();
        metric["status"] = json!("unknown");
        metric["lower"] = Value::Null;
        metric["upper"] = Value::Null;
        let output = report(&root);
        let row = output["assessment_reports"].as_array().unwrap().iter()
            .find(|item| item["assessment_id"] == "congo-science-samples-01-congo_low_disturbance_rover").unwrap();
        assert_eq!(row["readiness_disposition"], "not_assessable_incomplete_metrics");
        assert_eq!(row["metric_intervals"]["energy_per_mission_wh"], Value::Null);
    }

    #[test]
    fn reversed_and_nonfinite_intervals_fail_closed() {
        let mut root = fixture();
        root["assessments"][0]["metric_intervals"][0]["lower"] = json!(9.0);
        root["assessments"][0]["metric_intervals"][0]["upper"] = json!(1.0);
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn payload_mass_must_be_positive() {
        let mut root = fixture();
        root["payload_profiles"][0]["mass_kg"]["min"] = json!(0.0);
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["payload_profiles"][0]["mass_kg"]["min"] = json!(-1.0);
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn payload_contract_rejects_wheelchair_profile_without_accessibility_and_securement() {
        let mut root = fixture();
        let payload = root["payload_profiles"].as_array_mut().unwrap().iter_mut()
            .find(|p| p["payload_id"] == "wheelchair_passenger").unwrap();
        payload["accessible_boarding_required"] = json!(false);
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn payload_class_cannot_be_mislabeled_to_bypass_human_transport_gates() {
        let mut root = fixture();
        let payload = root["payload_profiles"].as_array_mut().unwrap().iter_mut()
            .find(|p| p["payload_id"] == "stretcher_patient").unwrap();
        payload["payload_class"] = json!("bulk_cargo");
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn mission_mode_must_match_the_declared_route_mode() {
        let mut root = fixture();
        let mission = root["mission_scenarios"].as_array_mut().unwrap().iter_mut()
            .find(|m| m["mission_id"] == "AMZ-COLDCHAIN-01").unwrap();
        mission["allowed_modes"] = json!(["aerial"]);
        assert_eq!(disposition(&root, "amz-coldchain-01-riverlink_cargo_boat"), "blocked_platform_mismatch");
    }

    #[test]
    fn combined_center_of_mass_must_remain_inside_declared_envelope() {
        let mut root = fixture();
        let platform = root["platform_profiles"].as_array_mut().unwrap().iter_mut()
            .find(|p| p["platform_id"] == "riverlink_cargo_boat").unwrap();
        platform["center_of_mass_limits_fraction"]["x"]["min"] = json!(0.9);
        platform["center_of_mass_limits_fraction"]["x"]["max"] = json!(1.0);
        assert_eq!(disposition(&root, "amz-coldchain-01-riverlink_cargo_boat"), "blocked_platform_mismatch");
    }

    #[test]
    fn input_reordering_yields_byte_identical_report() {
        let mut root = fixture();
        let before = serde_json::to_string(&report(&root)).unwrap();
        root["environments"].as_array_mut().unwrap().reverse();
        root["payload_profiles"].as_array_mut().unwrap().reverse();
        root["platform_profiles"].as_array_mut().unwrap().reverse();
        root["mission_scenarios"].as_array_mut().unwrap().reverse();
        root["gate_definitions"].as_array_mut().unwrap().reverse();
        root["metric_definitions"].as_array_mut().unwrap().reverse();
        root["assessments"].as_array_mut().unwrap().reverse();
        for mission in root["mission_scenarios"].as_array_mut().unwrap() {
            mission["payload_ids"].as_array_mut().unwrap().reverse();
            mission["required_gate_ids"].as_array_mut().unwrap().reverse();
        }
        for assessment in root["assessments"].as_array_mut().unwrap() {
            assessment["gate_results"].as_array_mut().unwrap().reverse();
            assessment["metric_intervals"].as_array_mut().unwrap().reverse();
            assessment["load_layout"].as_array_mut().unwrap().reverse();
        }
        let after = serde_json::to_string(&report(&root)).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn all_243_combinations_of_five_gate_states_match_reference_disposition() {
        let mut root = fixture();
        let index = root["assessments"].as_array().unwrap().iter()
            .position(|item| item["assessment_id"] == "amz-coldchain-01-riverlink_cargo_boat").unwrap();
        let gate_order = ["route_or_waterway_authorization", "environmental_protection", "recovery_plan", "payload_securement", "condition_control"];
        let original = root["assessments"][index]["gate_results"].clone();
        for code in 0..243usize {
            let mut n = code;
            let mut failed = false;
            let mut unknown = false;
            for gate_id in gate_order {
                let state = n % 3;
                n /= 3;
                let status = match state {
                    0 => "pass",
                    1 => { failed = true; "fail" },
                    _ => { unknown = true; "unknown" },
                };
                let gate_index = root["assessments"][index]["gate_results"].as_array().unwrap().iter()
                    .position(|entry| entry["gate_id"] == gate_id).unwrap();
                root["assessments"][index]["gate_results"][gate_index]["status"] = json!(status);
            }
            let expected = if failed { "blocked_hard_gate" }
                else if unknown { "needs_gate_evidence" }
                else { "eligible_for_human_review" };
            assert_eq!(disposition(&root, "amz-coldchain-01-riverlink_cargo_boat"), expected, "gate code {code}");
        }
        root["assessments"][index]["gate_results"] = original;
    }

    #[test]
    fn incompatible_temperature_control_blocks_fragile_cargo() {
        let mut root = fixture();
        let platform = root["platform_profiles"].as_array_mut().unwrap().iter_mut()
            .find(|p| p["platform_id"] == "congo_low_disturbance_rover").unwrap();
        platform["controlled_temp_max_c"] = json!(36.0);
        assert_eq!(disposition(&root, "congo-science-samples-01-congo_low_disturbance_rover"), "blocked_platform_mismatch");
    }

    #[test]
    fn unknown_root_fields_and_missing_load_positions_fail_closed() {
        let mut root = fixture();
        root["universal_best_platform"] = json!("some platform");
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["assessments"][0]["load_layout"].as_array_mut().unwrap().pop();
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["payload_profiles"] = json!("not-an-array");
        assert!(evaluate(&root).is_err());
    }
}
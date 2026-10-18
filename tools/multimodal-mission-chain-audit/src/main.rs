use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, path::Path};

const CORPUS: &str =
    include_str!("../../../fixtures/transport/multimodal_mission_chain_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_chain_feasibility_is_not_route_authorization_or_real_world_transport_validation";
const CLASSES: [&str; 8] = [
    "bulk_cargo", "cold_chain", "fragile_instrument", "contained_liquid",
    "seated_passengers", "wheelchair_user", "stretcher_patient", "field_crew",
];
const MODES: [&str; 6] = [
    "watercraft", "wheeled_rover", "tracked_carrier", "sled", "ground_carrier", "aerial",
];
const STATUS_FIELDS: [&str; 4] = [
    "route_authorization_status", "environmental_protection_status",
    "recovery_status", "hazard_status",
];
const HANDOFF_STATUS_FIELDS: [&str; 10] = [
    "authorization_status", "environmental_protection_status", "securement_status",
    "condition_control_status", "accessibility_status", "patient_support_status",
    "privacy_custody_status", "operator_status", "recovery_status", "hazard_status",
];

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

fn finite_number(value: &Value, path: &str, errors: &mut Vec<String>) -> Option<f64> {
    match value.as_f64() {
        Some(number) if number.is_finite() => Some(number),
        _ => {
            push(errors, path, "expected finite number");
            None
        }
    }
}

fn read_string_list(
    value: &Value,
    path: &str,
    errors: &mut Vec<String>,
    minimum: usize,
) -> Vec<String> {
    let mut ids = BTreeSet::new();
    match value.as_array() {
        Some(items) if items.len() >= minimum => {
            for (index, item) in items.iter().enumerate() {
                match item.as_str() {
                    Some(text) if !text.trim().is_empty() => {
                        if !ids.insert(text.to_string()) {
                            push(errors, &format!("{path}[{index}]"), "duplicate value");
                        }
                    }
                    _ => push(errors, &format!("{path}[{index}]"), "expected non-empty string"),
                }
            }
        }
        Some(_) => push(errors, path, "array shorter than required minimum"),
        None => push(errors, path, "expected array"),
    }
    ids.into_iter().collect()
}

fn range_valid(value: &Value, path: &str, errors: &mut Vec<String>) {
    if !exact_keys(value, &["min", "max"]) {
        push(errors, path, "expected exactly min and max");
        return;
    }
    let lo = finite_number(&value["min"], &format!("{path}.min"), errors);
    let hi = finite_number(&value["max"], &format!("{path}.max"), errors);
    if let (Some(lo), Some(hi)) = (lo, hi) {
        if lo > hi {
            push(errors, path, "minimum must be <= maximum");
        }
    }
}

fn nullable_range_valid(value: &Value, path: &str, errors: &mut Vec<String>) {
    if !value.is_null() {
        range_valid(value, path, errors);
    }
}

fn read_status(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) {
    match value.get(key).and_then(Value::as_str) {
        Some("pass" | "fail" | "unknown") => {}
        _ => push(errors, &format!("{path}.{key}"), "expected pass, fail, or unknown"),
    }
}

fn require_bool_fields(value: &Value, fields: &[&str], path: &str, errors: &mut Vec<String>) {
    for field in fields {
        if value.get(*field).and_then(Value::as_bool).is_none() {
            push(errors, &format!("{path}.{field}"), "expected boolean");
        }
    }
}

fn validate_input(root: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    const ROOT: &[&str] = &[
        "schema_version", "benchmark_id", "provenance", "claim_ceiling",
        "environments", "payload_profiles", "platform_profiles", "hub_profiles",
        "mission_scenarios", "assessments",
    ];
    if !exact_keys(root, ROOT) {
        push(&mut errors, "root", "missing or unknown fields");
    }
    if root.get("schema_version").and_then(Value::as_str) != Some("0.1") {
        push(&mut errors, "schema_version", "expected 0.1");
    }
    if root.get("benchmark_id").and_then(Value::as_str) != Some("multimodal-mission-chain-v0.1") {
        push(&mut errors, "benchmark_id", "unexpected benchmark identity");
    }
    if root.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING) {
        push(&mut errors, "claim_ceiling", "missing or incompatible claim ceiling");
    }
    let provenance = &root["provenance"];
    if !exact_keys(provenance, &["kind", "external_real_world_data_included", "notes"]) {
        push(&mut errors, "provenance", "missing or unknown fields");
    }
    if provenance.get("kind").and_then(Value::as_str) != Some("synthetic_only")
        || provenance.get("external_real_world_data_included").and_then(Value::as_bool) != Some(false) {
        push(&mut errors, "provenance", "v0.1 admits synthetic-only values");
    }
    read_string(provenance, "notes", "provenance", &mut errors);

    let mut env_ids = BTreeSet::new();
    if let Some(items) = root.get("environments").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "environments", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("environments[{i}]");
            if !exact_keys(item, &["environment_id", "min_ambient_temp_c", "max_ambient_temp_c"]) {
                push(&mut errors, &path, "missing or unknown fields");
            }
            if let Some(id) = read_string(item, "environment_id", &path, &mut errors) {
                if !env_ids.insert(id) { push(&mut errors, &path, "duplicate environment ID"); }
            }
            let lo = finite_number(&item["min_ambient_temp_c"], &format!("{path}.min_ambient_temp_c"), &mut errors);
            let hi = finite_number(&item["max_ambient_temp_c"], &format!("{path}.max_ambient_temp_c"), &mut errors);
            if let (Some(lo), Some(hi)) = (lo, hi) {
                if lo > hi { push(&mut errors, &path, "ambient minimum must be <= maximum"); }
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
                "payload_id", "payload_class", "mass_kg_max", "temperature_range_c",
                "securement_required", "occupants", "occupant_class", "accessible_transfer_required",
                "wheelchair_securement_required", "patient_support_required", "caregiver_access_required",
                "privacy_sensitive",
            ];
            if !exact_keys(item, KEYS) { push(&mut errors, &path, "missing or unknown fields"); }
            if let Some(id) = read_string(item, "payload_id", &path, &mut errors) {
                if !payload_ids.insert(id) { push(&mut errors, &path, "duplicate payload ID"); }
            }
            match item.get("payload_class").and_then(Value::as_str) {
                Some(class) if CLASSES.contains(&class) => {}
                _ => push(&mut errors, &format!("{path}.payload_class"), "unknown payload class"),
            }
            match finite_number(&item["mass_kg_max"], &format!("{path}.mass_kg_max"), &mut errors) {
                Some(mass) if mass > 0.0 => {}
                Some(_) => push(&mut errors, &format!("{path}.mass_kg_max"), "must be positive"),
                None => {}
            }
            nullable_range_valid(&item["temperature_range_c"], &format!("{path}.temperature_range_c"), &mut errors);
            require_bool_fields(item, &[
                "securement_required", "accessible_transfer_required", "wheelchair_securement_required",
                "patient_support_required", "caregiver_access_required", "privacy_sensitive",
            ], &path, &mut errors);
            let occupants = item.get("occupants").and_then(Value::as_u64);
            if occupants.is_none() { push(&mut errors, &format!("{path}.occupants"), "expected non-negative integer"); }
            let class = item.get("occupant_class").and_then(Value::as_str);
            if !class.is_some_and(|value| ["none", "seated", "wheelchair", "stretcher_patient", "field_crew"].contains(&value)) {
                push(&mut errors, &format!("{path}.occupant_class"), "unknown occupant class");
            }
            let has_people = occupants.unwrap_or(0) > 0;
            if has_people != class.is_some_and(|value| value != "none") {
                push(&mut errors, &path, "occupant count and occupant class disagree");
            }
            let expected_class = match item.get("payload_class").and_then(Value::as_str) {
                Some("bulk_cargo" | "cold_chain" | "fragile_instrument" | "contained_liquid") => Some("none"),
                Some("seated_passengers") => Some("seated"),
                Some("wheelchair_user") => Some("wheelchair"),
                Some("stretcher_patient") => Some("stretcher_patient"),
                Some("field_crew") => Some("field_crew"),
                _ => None,
            };
            if expected_class.is_some() && class != expected_class {
                push(&mut errors, &path, "payload class and occupant class disagree");
            }
            if class == Some("wheelchair") && (
                item["accessible_transfer_required"].as_bool() != Some(true)
                    || item["wheelchair_securement_required"].as_bool() != Some(true)
            ) {
                push(&mut errors, &path, "wheelchair payload requires accessible transfer and wheelchair securement");
            }
            if class == Some("stretcher_patient") && (
                item["patient_support_required"].as_bool() != Some(true)
                    || item["caregiver_access_required"].as_bool() != Some(true)
            ) {
                push(&mut errors, &path, "stretcher patient requires patient support and caregiver access");
            }
        }
    } else {
        push(&mut errors, "payload_profiles", "expected array");
    }

    let mut platform_ids = BTreeSet::new();
    if let Some(items) = root.get("platform_profiles").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "platform_profiles", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("platform_profiles[{i}]");
            const KEYS: &[&str] = &[
                "platform_id", "mode", "environment_ids", "supported_payload_classes",
                "max_payload_kg", "ambient_temp_min_c", "ambient_temp_max_c",
                "controlled_temp_min_c", "controlled_temp_max_c", "has_securement",
                "has_occupant_restraints", "has_accessible_transfer", "has_wheelchair_securement",
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
            let envs = read_string_list(&item["environment_ids"], &format!("{path}.environment_ids"), &mut errors, 1);
            for env in envs {
                if !env_ids.contains(&env) { push(&mut errors, &format!("{path}.environment_ids"), "undeclared environment"); }
            }
            let classes = read_string_list(&item["supported_payload_classes"], &format!("{path}.supported_payload_classes"), &mut errors, 1);
            for class in classes {
                if !CLASSES.contains(&class.as_str()) { push(&mut errors, &format!("{path}.supported_payload_classes"), "unknown payload class"); }
            }
            for key in ["max_payload_kg"] {
                match finite_number(&item[key], &format!("{path}.{key}"), &mut errors) {
                    Some(number) if number > 0.0 => {}
                    Some(_) => push(&mut errors, &format!("{path}.{key}"), "must be positive"),
                    None => {}
                }
            }
            let amb_lo = finite_number(&item["ambient_temp_min_c"], &format!("{path}.ambient_temp_min_c"), &mut errors);
            let amb_hi = finite_number(&item["ambient_temp_max_c"], &format!("{path}.ambient_temp_max_c"), &mut errors);
            if let (Some(lo), Some(hi)) = (amb_lo, amb_hi) {
                if lo > hi { push(&mut errors, &path, "ambient minimum must be <= maximum"); }
            }
            match (&item["controlled_temp_min_c"], &item["controlled_temp_max_c"]) {
                (lo, hi) if lo.is_null() && hi.is_null() => {}
                (lo, hi) if lo.as_f64().is_some_and(f64::is_finite) && hi.as_f64().is_some_and(f64::is_finite) => {
                    if lo.as_f64().unwrap() > hi.as_f64().unwrap() {
                        push(&mut errors, &path, "controlled temperature minimum must be <= maximum");
                    }
                }
                _ => push(&mut errors, &path, "controlled temperature endpoints must both be null or finite numbers"),
            }
            require_bool_fields(item, &[
                "has_securement", "has_occupant_restraints", "has_accessible_transfer",
                "has_wheelchair_securement", "has_patient_support", "has_caregiver_access",
                "has_emergency_egress", "has_privacy_controls", "has_recovery_plan", "has_emergency_stop",
            ], &path, &mut errors);
        }
    } else {
        push(&mut errors, "platform_profiles", "expected array");
    }

    let mut hub_ids = BTreeSet::new();
    if let Some(items) = root.get("hub_profiles").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "hub_profiles", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("hub_profiles[{i}]");
            const KEYS: &[&str] = &[
                "hub_id", "environment_id", "accepted_payload_classes", "max_handling_mass_kg",
                "storage_temperature_zones", "has_securement", "has_condition_control",
                "has_accessible_transfer", "has_patient_support", "has_caregiver_access",
                "has_privacy_custody", "has_operator_staffing", "has_recovery_support",
            ];
            if !exact_keys(item, KEYS) { push(&mut errors, &path, "missing or unknown fields"); }
            if let Some(id) = read_string(item, "hub_id", &path, &mut errors) {
                if !hub_ids.insert(id) { push(&mut errors, &path, "duplicate hub ID"); }
            }
            if !item.get("environment_id").and_then(Value::as_str).is_some_and(|id| env_ids.contains(id)) {
                push(&mut errors, &format!("{path}.environment_id"), "undeclared environment");
            }
            let classes = read_string_list(&item["accepted_payload_classes"], &format!("{path}.accepted_payload_classes"), &mut errors, 1);
            for class in classes {
                if !CLASSES.contains(&class.as_str()) { push(&mut errors, &format!("{path}.accepted_payload_classes"), "unknown payload class"); }
            }
            match finite_number(&item["max_handling_mass_kg"], &format!("{path}.max_handling_mass_kg"), &mut errors) {
                Some(mass) if mass > 0.0 => {}
                Some(_) => push(&mut errors, &format!("{path}.max_handling_mass_kg"), "must be positive"),
                None => {}
            }
            match item.get("storage_temperature_zones").and_then(Value::as_array) {
                Some(zones) => {
                    for (j, zone) in zones.iter().enumerate() {
                        range_valid(zone, &format!("{path}.storage_temperature_zones[{j}]"), &mut errors);
                    }
                }
                None => push(&mut errors, &format!("{path}.storage_temperature_zones"), "expected array"),
            }
            require_bool_fields(item, &[
                "has_securement", "has_condition_control", "has_accessible_transfer",
                "has_patient_support", "has_caregiver_access", "has_privacy_custody",
                "has_operator_staffing", "has_recovery_support",
            ], &path, &mut errors);
        }
    } else {
        push(&mut errors, "hub_profiles", "expected array");
    }

    let mut mission_ids = BTreeSet::new();
    if let Some(items) = root.get("mission_scenarios").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "mission_scenarios", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("mission_scenarios[{i}]");
            const KEYS: &[&str] = &[
                "mission_id", "mission_class", "origin_hub_id", "destination_hub_id",
                "payload_ids", "deadline_minutes", "operating_notes",
            ];
            if !exact_keys(item, KEYS) { push(&mut errors, &path, "missing or unknown fields"); }
            if let Some(id) = read_string(item, "mission_id", &path, &mut errors) {
                if !mission_ids.insert(id) { push(&mut errors, &path, "duplicate mission ID"); }
            }
            read_string(item, "mission_class", &path, &mut errors);
            read_string(item, "operating_notes", &path, &mut errors);
            for key in ["origin_hub_id", "destination_hub_id"] {
                if !item.get(key).and_then(Value::as_str).is_some_and(|id| hub_ids.contains(id)) {
                    push(&mut errors, &format!("{path}.{key}"), "undeclared hub ID");
                }
            }
            let payloads = read_string_list(&item["payload_ids"], &format!("{path}.payload_ids"), &mut errors, 1);
            for payload in payloads {
                if !payload_ids.contains(&payload) { push(&mut errors, &format!("{path}.payload_ids"), "undeclared payload ID"); }
            }
            match finite_number(&item["deadline_minutes"], &format!("{path}.deadline_minutes"), &mut errors) {
                Some(value) if value > 0.0 => {}
                Some(_) => push(&mut errors, &format!("{path}.deadline_minutes"), "must be positive"),
                None => {}
            }
        }
    } else {
        push(&mut errors, "mission_scenarios", "expected array");
    }

    let mut assessment_ids = BTreeSet::new();
    if let Some(items) = root.get("assessments").and_then(Value::as_array) {
        if items.is_empty() { push(&mut errors, "assessments", "must not be empty"); }
        for (i, item) in items.iter().enumerate() {
            let path = format!("assessments[{i}]");
            if !exact_keys(item, &["assessment_id", "mission_id", "legs", "handoffs"]) {
                push(&mut errors, &path, "missing or unknown fields");
            }
            if let Some(id) = read_string(item, "assessment_id", &path, &mut errors) {
                if !assessment_ids.insert(id) { push(&mut errors, &path, "duplicate assessment ID"); }
            }
            let mission_id = read_string(item, "mission_id", &path, &mut errors);
            let mission = mission_id.as_ref().and_then(|id| {
                root["mission_scenarios"].as_array().and_then(|missions| {
                    missions.iter().find(|m| m["mission_id"].as_str() == Some(id.as_str()))
                })
            });
            if mission.is_none() { push(&mut errors, &format!("{path}.mission_id"), "undeclared mission ID"); }
            let legs = item.get("legs").and_then(Value::as_array);
            let handoffs = item.get("handoffs").and_then(Value::as_array);
            if legs.is_none_or(|x| x.is_empty()) { push(&mut errors, &format!("{path}.legs"), "expected non-empty array"); }
            if handoffs.is_none() { push(&mut errors, &format!("{path}.handoffs"), "expected array"); }
            if let (Some(legs), Some(handoffs), Some(mission)) = (legs, handoffs, mission) {
                if handoffs.len() != legs.len().saturating_sub(1) {
                    push(&mut errors, &format!("{path}.handoffs"), "requires exactly one handoff between each adjacent pair of legs");
                }
                let mut leg_ids = BTreeSet::new();
                for (j, leg) in legs.iter().enumerate() {
                    let lp = format!("{path}.legs[{j}]");
                    const LEG_KEYS: &[&str] = &[
                        "leg_id", "platform_id", "origin_hub_id", "destination_hub_id",
                        "duration_minutes_min", "duration_minutes_max", "route_authorization_status",
                        "environmental_protection_status", "recovery_status", "hazard_status",
                    ];
                    if !exact_keys(leg, LEG_KEYS) { push(&mut errors, &lp, "missing or unknown fields"); }
                    if let Some(id) = read_string(leg, "leg_id", &lp, &mut errors) {
                        if !leg_ids.insert(id) { push(&mut errors, &lp, "duplicate leg ID"); }
                    }
                    if !leg.get("platform_id").and_then(Value::as_str).is_some_and(|id| platform_ids.contains(id)) {
                        push(&mut errors, &format!("{lp}.platform_id"), "undeclared platform");
                    }
                    for key in ["origin_hub_id", "destination_hub_id"] {
                        if !leg.get(key).and_then(Value::as_str).is_some_and(|id| hub_ids.contains(id)) {
                            push(&mut errors, &format!("{lp}.{key}"), "undeclared hub");
                        }
                    }
                    let lo = finite_number(&leg["duration_minutes_min"], &format!("{lp}.duration_minutes_min"), &mut errors);
                    let hi = finite_number(&leg["duration_minutes_max"], &format!("{lp}.duration_minutes_max"), &mut errors);
                    if let (Some(lo), Some(hi)) = (lo, hi) {
                        if lo < 0.0 || hi < 0.0 || lo > hi { push(&mut errors, &lp, "duration bounds invalid"); }
                    }
                    for key in STATUS_FIELDS { read_status(leg, key, &lp, &mut errors); }
                }
                for (j, handoff) in handoffs.iter().enumerate() {
                    let hp = format!("{path}.handoffs[{j}]");
                    const HANDOFF_KEYS: &[&str] = &[
                        "handoff_id", "from_leg_id", "to_leg_id", "hub_id", "duration_minutes_min",
                        "duration_minutes_max", "authorization_status", "environmental_protection_status",
                        "securement_status", "condition_control_status", "accessibility_status",
                        "patient_support_status", "privacy_custody_status", "operator_status",
                        "recovery_status", "hazard_status",
                    ];
                    if !exact_keys(handoff, HANDOFF_KEYS) { push(&mut errors, &hp, "missing or unknown fields"); }
                    for key in ["handoff_id"] { read_string(handoff, key, &hp, &mut errors); }
                    for key in ["from_leg_id", "to_leg_id"] {
                        if !handoff.get(key).and_then(Value::as_str).is_some_and(|id| leg_ids.contains(id)) {
                            push(&mut errors, &format!("{hp}.{key}"), "undeclared leg ID");
                        }
                    }
                    if !handoff.get("hub_id").and_then(Value::as_str).is_some_and(|id| hub_ids.contains(id)) {
                        push(&mut errors, &format!("{hp}.hub_id"), "undeclared hub ID");
                    }
                    let lo = finite_number(&handoff["duration_minutes_min"], &format!("{hp}.duration_minutes_min"), &mut errors);
                    let hi = finite_number(&handoff["duration_minutes_max"], &format!("{hp}.duration_minutes_max"), &mut errors);
                    if let (Some(lo), Some(hi)) = (lo, hi) {
                        if lo < 0.0 || hi < 0.0 || lo > hi { push(&mut errors, &hp, "duration bounds invalid"); }
                    }
                    for key in HANDOFF_STATUS_FIELDS { read_status(handoff, key, &hp, &mut errors); }
                }
                if let (Some(first), Some(last)) = (legs.first(), legs.last()) {
                    if first["origin_hub_id"] != mission["origin_hub_id"] {
                        push(&mut errors, &format!("{path}.legs[0]"), "chain origin does not match mission origin");
                    }
                    if last["destination_hub_id"] != mission["destination_hub_id"] {
                        push(&mut errors, &format!("{path}.legs[last]"), "chain destination does not match mission destination");
                    }
                }
                for j in 0..legs.len().saturating_sub(1).min(handoffs.len()) {
                    let prev = &legs[j];
                    let next = &legs[j + 1];
                    let handoff = &handoffs[j];
                    if prev["destination_hub_id"] != next["origin_hub_id"] {
                        push(&mut errors, &format!("{path}.legs[{}]", j + 1), "broken hub-to-hub continuity between adjacent legs");
                    }
                    if handoff["from_leg_id"] != prev["leg_id"] || handoff["to_leg_id"] != next["leg_id"] {
                        push(&mut errors, &format!("{path}.handoffs[{j}]"), "handoff leg order does not match the route chain");
                    }
                    if handoff["hub_id"] != prev["destination_hub_id"] || handoff["hub_id"] != next["origin_hub_id"] {
                        push(&mut errors, &format!("{path}.handoffs[{j}]"), "handoff hub must equal adjacent leg endpoint");
                    }
                }
            }
        }
    } else {
        push(&mut errors, "assessments", "expected array");
    }

    if errors.is_empty() { Ok(()) } else { errors.sort(); Err(errors) }
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn num(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or(0.0)
}

fn string(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_string()
}

fn list_set(value: &Value, key: &str) -> BTreeSet<String> {
    value[key].as_array().map(|items| items.iter()
        .filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn temp_zone_compatible(hub_or_platform: &Value, payload: &Value, is_hub: bool) -> bool {
    if payload["temperature_range_c"].is_null() { return true; }
    let allowed_lo = num(&payload["temperature_range_c"], "min");
    let allowed_hi = num(&payload["temperature_range_c"], "max");
    if is_hub {
        if !flag(hub_or_platform, "has_condition_control") { return false; }
        return hub_or_platform["storage_temperature_zones"].as_array().is_some_and(|zones| {
            zones.iter().any(|zone| num(zone, "min") >= allowed_lo && num(zone, "max") <= allowed_hi)
        });
    }
    let lo = &hub_or_platform["controlled_temp_min_c"];
    let hi = &hub_or_platform["controlled_temp_max_c"];
    !lo.is_null() && !hi.is_null()
        && lo.as_f64().is_some_and(|x| x >= allowed_lo)
        && hi.as_f64().is_some_and(|x| x <= allowed_hi)
}

fn evaluate(root: &Value) -> Result<Value, Vec<String>> {
    validate_input(root)?;
    let envs: BTreeMap<String, &Value> = root["environments"].as_array().unwrap().iter()
        .map(|v| (string(v, "environment_id"), v)).collect();
    let payloads: BTreeMap<String, &Value> = root["payload_profiles"].as_array().unwrap().iter()
        .map(|v| (string(v, "payload_id"), v)).collect();
    let platforms: BTreeMap<String, &Value> = root["platform_profiles"].as_array().unwrap().iter()
        .map(|v| (string(v, "platform_id"), v)).collect();
    let hubs: BTreeMap<String, &Value> = root["hub_profiles"].as_array().unwrap().iter()
        .map(|v| (string(v, "hub_id"), v)).collect();
    let missions: BTreeMap<String, &Value> = root["mission_scenarios"].as_array().unwrap().iter()
        .map(|v| (string(v, "mission_id"), v)).collect();

    let mut reports = Vec::new();
    for assessment in root["assessments"].as_array().unwrap() {
        let id = string(assessment, "assessment_id");
        let mission_id = string(assessment, "mission_id");
        let mission = missions[&mission_id];
        let mut payload_ids: Vec<String> = mission["payload_ids"].as_array().unwrap().iter()
            .map(|v| v.as_str().unwrap().to_string()).collect();
        payload_ids.sort();
        let payload_list: Vec<&Value> = payload_ids.iter().map(|pid| payloads[pid]).collect();
        let total_mass: f64 = payload_list.iter().map(|p| num(p, "mass_kg_max")).sum();
        let needs_condition = payload_list.iter().any(|p| !p["temperature_range_c"].is_null());
        let needs_securement = payload_list.iter().any(|p| flag(p, "securement_required"));
        let needs_accessibility = payload_list.iter().any(|p| flag(p, "accessible_transfer_required") || num(p, "occupants") > 0.0);
        let needs_patient = payload_list.iter().any(|p| flag(p, "patient_support_required") || flag(p, "caregiver_access_required"));
        let needs_privacy = payload_list.iter().any(|p| flag(p, "privacy_sensitive"));

        let mut gate_failures = BTreeSet::new();
        let mut unknown_evidence = BTreeSet::new();
        let mut mismatch_reasons = BTreeSet::new();
        let mut total_min = 0.0;
        let mut total_max = 0.0;
        let legs = assessment["legs"].as_array().unwrap();
        let handoffs = assessment["handoffs"].as_array().unwrap();

        for leg in legs.iter() {
            let leg_id = string(leg, "leg_id");
            let platform = platforms[&string(leg, "platform_id")];
            let origin = hubs[&string(leg, "origin_hub_id")];
            let destination = hubs[&string(leg, "destination_hub_id")];
            let origin_env = envs[&string(origin, "environment_id")];
            let destination_env = envs[&string(destination, "environment_id")];
            let duration_min = num(leg, "duration_minutes_min");
            let duration_max = num(leg, "duration_minutes_max");
            total_min += duration_min;
            total_max += duration_max;

            if !list_set(platform, "environment_ids").contains(&string(origin, "environment_id"))
                || !list_set(platform, "environment_ids").contains(&string(destination, "environment_id")) {
                mismatch_reasons.insert(format!("{leg_id}:platform_environment_not_supported"));
            }
            if num(platform, "ambient_temp_min_c") > num(origin_env, "min_ambient_temp_c")
                || num(platform, "ambient_temp_max_c") < num(origin_env, "max_ambient_temp_c")
                || num(platform, "ambient_temp_min_c") > num(destination_env, "min_ambient_temp_c")
                || num(platform, "ambient_temp_max_c") < num(destination_env, "max_ambient_temp_c") {
                mismatch_reasons.insert(format!("{leg_id}:platform_ambient_envelope_insufficient"));
            }
            if num(platform, "max_payload_kg") < total_mass {
                mismatch_reasons.insert(format!("{leg_id}:combined_payload_mass_exceeds_platform_capacity"));
            }
            let supported = list_set(platform, "supported_payload_classes");
            for payload in &payload_list {
                let pid = string(payload, "payload_id");
                let class = string(payload, "payload_class");
                if !supported.contains(&class) {
                    mismatch_reasons.insert(format!("{leg_id}:unsupported_payload_class:{pid}"));
                }
                if !temp_zone_compatible(platform, payload, false) {
                    mismatch_reasons.insert(format!("{leg_id}:payload_temperature_not_supported:{pid}"));
                }
                if flag(payload, "securement_required") && !flag(platform, "has_securement") {
                    mismatch_reasons.insert(format!("{leg_id}:payload_securement_unavailable:{pid}"));
                }
                if flag(payload, "privacy_sensitive") && !flag(platform, "has_privacy_controls") {
                    mismatch_reasons.insert(format!("{leg_id}:privacy_controls_unavailable:{pid}"));
                }
                if num(payload, "occupants") > 0.0
                    && (!flag(platform, "has_occupant_restraints")
                        || !flag(platform, "has_emergency_egress")
                        || !flag(platform, "has_accessible_transfer")) {
                    mismatch_reasons.insert(format!("{leg_id}:occupant_access_restraints_or_egress_unavailable:{pid}"));
                }
                if flag(payload, "accessible_transfer_required") && !flag(platform, "has_accessible_transfer") {
                    mismatch_reasons.insert(format!("{leg_id}:accessible_transfer_unavailable:{pid}"));
                }
                if flag(payload, "wheelchair_securement_required") && !flag(platform, "has_wheelchair_securement") {
                    mismatch_reasons.insert(format!("{leg_id}:wheelchair_securement_unavailable:{pid}"));
                }
                if flag(payload, "patient_support_required") && !flag(platform, "has_patient_support") {
                    mismatch_reasons.insert(format!("{leg_id}:patient_support_unavailable:{pid}"));
                }
                if flag(payload, "caregiver_access_required") && !flag(platform, "has_caregiver_access") {
                    mismatch_reasons.insert(format!("{leg_id}:caregiver_access_unavailable:{pid}"));
                }
                if num(payload, "occupants") > 0.0 && !flag(platform, "has_emergency_stop") {
                    mismatch_reasons.insert(format!("{leg_id}:emergency_stop_unavailable:{pid}"));
                }
            }
            if !flag(platform, "has_emergency_stop") {
                mismatch_reasons.insert(format!("{leg_id}:platform_emergency_stop_unavailable"));
            }
            for key in STATUS_FIELDS {
                match string(leg, key).as_str() {
                    "fail" => { gate_failures.insert(format!("{leg_id}:{key}")); }
                    "unknown" => { unknown_evidence.insert(format!("{leg_id}:{key}")); }
                    _ => {}
                }
            }
            if !flag(platform, "has_recovery_plan") {
                mismatch_reasons.insert(format!("{leg_id}:platform_recovery_plan_missing"));
            }
            let has_people = payload_list
                .iter()
                .any(|payload| num(payload, "occupants") > 0.0);
            if has_people && !flag(platform, "has_emergency_egress") {
                mismatch_reasons.insert(format!("{leg_id}:human_transport_egress_missing"));
            }
        }

        let mut used_hub_ids: BTreeSet<String> = BTreeSet::new();
        for leg in legs {
            used_hub_ids.insert(string(leg, "origin_hub_id"));
            used_hub_ids.insert(string(leg, "destination_hub_id"));
        }
        for handoff in handoffs {
            used_hub_ids.insert(string(handoff, "hub_id"));
            let handoff_id = string(handoff, "handoff_id");
            total_min += num(handoff, "duration_minutes_min");
            total_max += num(handoff, "duration_minutes_max");

            for (key, applies) in [
                ("authorization_status", true),
                ("environmental_protection_status", true),
                ("securement_status", needs_securement),
                ("condition_control_status", needs_condition),
                ("accessibility_status", needs_accessibility),
                ("patient_support_status", needs_patient),
                ("privacy_custody_status", needs_privacy),
                ("operator_status", true),
                ("recovery_status", true),
                ("hazard_status", true),
            ] {
                if !applies { continue; }
                match string(handoff, key).as_str() {
                    "fail" => { gate_failures.insert(format!("{handoff_id}:{key}")); }
                    "unknown" => { unknown_evidence.insert(format!("{handoff_id}:{key}")); }
                    _ => {}
                }
            }
        }

        for hub_id in used_hub_ids {
            let hub = hubs[&hub_id];
            if num(hub, "max_handling_mass_kg") < total_mass {
                mismatch_reasons.insert(format!("{hub_id}:combined_payload_mass_exceeds_hub_handling_capacity"));
            }
            let accepted = list_set(hub, "accepted_payload_classes");
            for payload in &payload_list {
                let pid = string(payload, "payload_id");
                let class = string(payload, "payload_class");
                if !accepted.contains(&class) {
                    mismatch_reasons.insert(format!("{hub_id}:unsupported_payload_class:{pid}"));
                }
                if !temp_zone_compatible(hub, payload, true) {
                    mismatch_reasons.insert(format!("{hub_id}:payload_condition_storage_unavailable:{pid}"));
                }
                if flag(payload, "securement_required") && !flag(hub, "has_securement") {
                    mismatch_reasons.insert(format!("{hub_id}:securement_unavailable:{pid}"));
                }
                if flag(payload, "accessible_transfer_required") && !flag(hub, "has_accessible_transfer") {
                    mismatch_reasons.insert(format!("{hub_id}:accessible_transfer_unavailable:{pid}"));
                }
                if flag(payload, "patient_support_required") && !flag(hub, "has_patient_support") {
                    mismatch_reasons.insert(format!("{hub_id}:patient_support_unavailable:{pid}"));
                }
                if flag(payload, "caregiver_access_required") && !flag(hub, "has_caregiver_access") {
                    mismatch_reasons.insert(format!("{hub_id}:caregiver_access_unavailable:{pid}"));
                }
                if flag(payload, "privacy_sensitive") && !flag(hub, "has_privacy_custody") {
                    mismatch_reasons.insert(format!("{hub_id}:privacy_custody_unavailable:{pid}"));
                }
            }
            for capability in ["has_operator_staffing", "has_recovery_support"] {
                if !flag(hub, capability) {
                    mismatch_reasons.insert(format!("{hub_id}:{capability}_missing"));
                }
            }
        }

        let deadline = num(mission, "deadline_minutes");
        if total_max > deadline {
            mismatch_reasons.insert("worst_case_chain_duration_exceeds_deadline".to_string());
        }
        let disposition = if !gate_failures.is_empty() {
            "blocked_hard_gate"
        } else if !mismatch_reasons.is_empty() {
            "blocked_chain_mismatch"
        } else if !unknown_evidence.is_empty() {
            "needs_gate_evidence"
        } else {
            "eligible_for_human_review"
        };
        reports.push(json!({
            "assessment_id": id,
            "mission_id": mission_id,
            "chain_disposition": disposition,
            "leg_count": legs.len(),
            "handoff_count": handoffs.len(),
            "payload_ids": payload_ids,
            "total_payload_mass_kg_max": total_mass,
            "chain_duration_minutes_min": total_min,
            "chain_duration_minutes_max": total_max,
            "deadline_minutes": deadline,
            "deadline_guaranteed": total_max <= deadline,
            "failed_gate_evidence": gate_failures,
            "unknown_gate_evidence": unknown_evidence,
            "chain_mismatch_reasons": mismatch_reasons,
            "route_or_deployment_authorized": false,
            "patient_care_authorized": false,
            "autonomous_hazard_handling_authorized": false
        }));
    }
    reports.sort_by(|a, b| a["assessment_id"].as_str().cmp(&b["assessment_id"].as_str()));
    let count = |state: &str| reports.iter().filter(|r| r["chain_disposition"] == state).count();
    let summary: Vec<Value> = missions.iter().map(|(mission_id, mission)| {
        let assessments: Vec<&Value> = reports.iter().filter(|r| r["mission_id"] == mission_id.as_str()).collect();
        json!({
            "mission_id": mission_id,
            "mission_class": mission["mission_class"],
            "candidate_chain_count": assessments.len(),
            "eligible_for_human_review_count": assessments.iter().filter(|r| r["chain_disposition"] == "eligible_for_human_review").count(),
            "blocked_hard_gate_count": assessments.iter().filter(|r| r["chain_disposition"] == "blocked_hard_gate").count(),
            "blocked_chain_mismatch_count": assessments.iter().filter(|r| r["chain_disposition"] == "blocked_chain_mismatch").count(),
            "needs_gate_evidence_count": assessments.iter().filter(|r| r["chain_disposition"] == "needs_gate_evidence").count(),
            "universal_winner_produced": false
        })
    }).collect();
    Ok(json!({
        "report_version": "0.1",
        "benchmark_id": "multimodal-mission-chain-v0.1",
        "claim_ceiling": CLAIM_CEILING,
        "environment_count": envs.len(),
        "payload_profile_count": payloads.len(),
        "platform_profile_count": platforms.len(),
        "hub_profile_count": hubs.len(),
        "mission_count": missions.len(),
        "assessment_count": reports.len(),
        "eligible_for_human_review_count": count("eligible_for_human_review"),
        "blocked_hard_gate_count": count("blocked_hard_gate"),
        "blocked_chain_mismatch_count": count("blocked_chain_mismatch"),
        "needs_gate_evidence_count": count("needs_gate_evidence"),
        "mission_summaries": summary,
        "chain_assessments": reports,
        "weighted_utility_score_produced": false,
        "universal_winner_produced": false,
        "route_or_deployment_authorized": false,
        "patient_care_authorized": false,
        "autonomous_hazard_handling_authorized": false,
        "interpretation": "The audit checks synthetic end-to-end chain structure and declared compatibility. Eligibility is not real route clearance, product qualification, clinical advice, dispatch or deployment authorization."
    }))
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        "fixtures/transport/multimodal_mission_chain_scenarios_v0_1.json".to_string()
    });
    if args.next().is_some() {
        eprintln!("usage: multimodal-mission-chain-audit [path/to/fixture.json]");
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
        evaluate(root).expect("fixture evaluates")
    }

    fn disposition(root: &Value, id: &str) -> String {
        report(root)["chain_assessments"].as_array().unwrap().iter()
            .find(|x| x["assessment_id"] == id).unwrap()["chain_disposition"].as_str().unwrap().to_string()
    }

    fn assessment_mut<'a>(root: &'a mut Value, id: &str) -> &'a mut Value {
        root["assessments"].as_array_mut().unwrap().iter_mut()
            .find(|x| x["assessment_id"] == id).unwrap()
    }

    #[test]
    fn fixture_baseline_proves_chain_specific_dispositions() {
        let out = report(&fixture());
        assert_eq!(out["environment_count"], 5);
        assert_eq!(out["payload_profile_count"], 8);
        assert_eq!(out["platform_profile_count"], 14);
        assert_eq!(out["hub_profile_count"], 21);
        assert_eq!(out["mission_count"], 7);
        assert_eq!(out["assessment_count"], 8);
        assert_eq!(out["eligible_for_human_review_count"], 4);
        assert_eq!(out["blocked_hard_gate_count"], 1);
        assert_eq!(out["blocked_chain_mismatch_count"], 1);
        assert_eq!(out["needs_gate_evidence_count"], 2);
        assert_eq!(out["route_or_deployment_authorized"], false);
    }

    #[test]
    fn broken_transfer_conditions_block_cold_chain_even_when_both_legs_fit() {
        let root = fixture();
        assert_eq!(disposition(&root, "amz-coldchain-good"), "eligible_for_human_review");
        assert_eq!(disposition(&root, "congo-sample-temperature-break-negative"), "blocked_hard_gate");
    }

    #[test]
    fn worst_case_chain_duration_must_fit_the_deadline() {
        let root = fixture();
        assert_eq!(disposition(&root, "sahara-deadline-negative"), "blocked_chain_mismatch");
    }

    #[test]
    fn unknown_afghanistan_route_or_hazard_status_never_authorizes_a_chain() {
        let root = fixture();
        assert_eq!(disposition(&root, "afg-medical-route-unknown"), "needs_gate_evidence");
        assert_eq!(disposition(&root, "afg-patient-route-unknown"), "needs_gate_evidence");
        let report = report(&root);
        assert_eq!(report["route_or_deployment_authorized"], false);
        assert_eq!(report["autonomous_hazard_handling_authorized"], false);
    }

    #[test]
    fn wheelchair_capability_must_exist_across_every_leg() {
        let mut root = fixture();
        let assessment = assessment_mut(&mut root, "amz-accessible-passenger-good");
        assessment["legs"][1]["platform_id"] = json!("coldchain_lastmile_van");
        assert_eq!(disposition(&root, "amz-accessible-passenger-good"), "blocked_chain_mismatch");
    }

    #[test]
    fn route_chain_requires_adjacent_leg_endpoints_to_match() {
        let mut root = fixture();
        assessment_mut(&mut root, "amz-coldchain-good")["legs"][1]["origin_hub_id"] = json!("amz_river_landing");
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn handoff_must_connect_the_adjacent_leg_pair_in_order() {
        let mut root = fixture();
        assessment_mut(&mut root, "amz-coldchain-good")["handoffs"][0]["to_leg_id"] = json!("wrong-leg");
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn handoff_intervals_are_part_of_total_duration() {
        let mut root = fixture();
        assessment_mut(&mut root, "amz-coldchain-good")["handoffs"][0]["duration_minutes_max"] = json!(200.0);
        assert_eq!(disposition(&root, "amz-coldchain-good"), "blocked_chain_mismatch");
    }

    #[test]
    fn input_order_permutations_have_identical_report() {
        let mut root = fixture();
        let before = serde_json::to_string(&report(&root)).unwrap();
        root["environments"].as_array_mut().unwrap().reverse();
        root["payload_profiles"].as_array_mut().unwrap().reverse();
        root["platform_profiles"].as_array_mut().unwrap().reverse();
        root["hub_profiles"].as_array_mut().unwrap().reverse();
        root["mission_scenarios"].as_array_mut().unwrap().reverse();
        root["assessments"].as_array_mut().unwrap().reverse();
        let after = serde_json::to_string(&report(&root)).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn all_81_leg_gate_combinations_match_independent_disposition_oracle() {
        let mut root = fixture();
        let original = assessment_mut(&mut root, "amz-coldchain-good")["legs"][0].clone();
        for code in 0..81usize {
            let mut n = code;
            let mut fail = false;
            let mut unknown = false;
            for key in STATUS_FIELDS {
                let state = n % 3;
                n /= 3;
                let value = match state {
                    0 => "pass",
                    1 => { fail = true; "fail" },
                    _ => { unknown = true; "unknown" },
                };
                assessment_mut(&mut root, "amz-coldchain-good")["legs"][0][key] = json!(value);
            }
            let expected = if fail { "blocked_hard_gate" } else if unknown { "needs_gate_evidence" } else { "eligible_for_human_review" };
            assert_eq!(disposition(&root, "amz-coldchain-good"), expected, "gate-state code {code}");
        }
        assessment_mut(&mut root, "amz-coldchain-good")["legs"][0] = original;
    }

    #[test]
    fn malformed_duration_and_unknown_fields_fail_closed() {
        let mut root = fixture();
        assessment_mut(&mut root, "amz-coldchain-good")["legs"][0]["duration_minutes_min"] = json!(200.0);
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["assessments"][0]["unsafe_winner"] = json!(true);
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn patient_support_and_caregiver_access_must_survive_the_handoff() {
        let mut root = fixture();
        assessment_mut(&mut root, "afg-patient-route-unknown")["handoffs"][0]["patient_support_status"] = json!("fail");
        assert_eq!(disposition(&root, "afg-patient-route-unknown"), "blocked_hard_gate");
    }

    #[test]
    fn payload_condition_range_must_be_supported_by_every_leg_and_hub() {
        let mut root = fixture();
        let hub = root["hub_profiles"].as_array_mut().unwrap().iter_mut()
            .find(|h| h["hub_id"] == "amz_cold_depot").unwrap();
        hub["storage_temperature_zones"] = json!([{"min":10.0,"max":15.0}]);
        assert_eq!(disposition(&root, "amz-coldchain-good"), "blocked_chain_mismatch");
    }
}
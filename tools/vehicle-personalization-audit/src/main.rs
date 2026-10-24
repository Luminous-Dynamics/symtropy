use jsonschema::draft202012;
use jsonschema::Validator;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const PROFILE_SET_SCHEMA_ID: &str =
    "urn:luminous-dynamics:vehicle:personalization-profile-set:0.1";
const PROFILE_SET_VERSION: &str = "vehicle-personalization-profile-set/0.1";
const PROFILE_IDS: [&str; 4] = [
    "vpf_daily001",
    "vpf_access001",
    "vpf_shared001",
    "vpf_guest0001",
];
const CONTROL_IDS: [&str; 8] = [
    "NC-PERS-01",
    "NC-PERS-02",
    "NC-PERS-03",
    "NC-PERS-04",
    "NC-PERS-05",
    "NC-PERS-06",
    "NC-PERS-07",
    "NC-PERS-08",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    profile_count: usize,
    control_count: usize,
    rejected_control_count: usize,
    false_accept_count: usize,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn schema_validator(schema: &Value, expected_id: &str) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("profile schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(expected_id) {
        return Err(format!("profile schema $id must be {expected_id}"));
    }
    if !draft202012::meta::is_valid(schema) {
        return Err("profile schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile profile schema: {error}"))
}

fn instance_errors(validator: &Validator, value: &Value, label: &str) -> Errors {
    validator
        .iter_errors(value)
        .map(|error| format!("{label}: {error}"))
        .collect()
}

fn profile_semantic_errors(profile: &Value) -> Errors {
    let mut errors = Vec::new();
    let profile_id = text_at(profile, "profile_id").unwrap_or("<unknown-profile>");

    let interaction = &profile["interaction_policy"];
    for (key, expected) in [
        ("moving_ui_policy", "minimal_low_distraction"),
        ("profile_changes_while_moving", "blocked"),
        ("safety_critical_settings", "manufacturer_controlled_non_overridable"),
        ("manual_override", "always_available"),
        ("unsupported_preference_behavior", "preserve_but_do_not_apply_and_explain"),
    ] {
        if text_at(interaction, key) != Some(expected) {
            errors.push(format!("{profile_id}: interaction_policy.{key} must remain {expected}"));
        }
    }

    let privacy = &profile["privacy"];
    let cloud_mode = text_at(privacy, "cloud_sync_mode");
    let cloud_consent = bool_at(privacy, "cloud_sync_consent");
    if cloud_mode == Some("explicit_opt_in") && cloud_consent != Some(true) {
        errors.push(format!("{profile_id}: cloud sync requires explicit consent"));
    }
    if cloud_mode == Some("disabled") && cloud_consent != Some(false) {
        errors.push(format!("{profile_id}: disabled cloud sync must not imply consent"));
    }

    let telemetry_mode = text_at(privacy, "telemetry_mode");
    let telemetry_consent = bool_at(privacy, "telemetry_consent");
    if telemetry_mode == Some("explicit_opt_in") && telemetry_consent != Some(true) {
        errors.push(format!("{profile_id}: telemetry requires explicit consent"));
    }
    if telemetry_mode == Some("disabled") && telemetry_consent != Some(false) {
        errors.push(format!("{profile_id}: disabled telemetry must not imply consent"));
    }

    if text_at(privacy, "profile_scope") == Some("guest") {
        if bool_at(privacy, "clear_on_session_end") != Some(true) {
            errors.push(format!("{profile_id}: guest profile must clear at session end"));
        }
        if privacy["retention_days"].as_u64().unwrap_or(u64::MAX) > 1 {
            errors.push(format!("{profile_id}: guest retention must not exceed one day"));
        }
        if cloud_mode != Some("disabled") || telemetry_mode != Some("disabled") {
            errors.push(format!("{profile_id}: guest profile must disable cloud sync and telemetry"));
        }
    }

    if text_at(&profile["display_preferences"], "theme") == Some("high_contrast")
        && text_at(&profile["display_preferences"], "contrast") != Some("high")
    {
        errors.push(format!("{profile_id}: high_contrast theme requires high contrast mode"));
    }

    if bool_at(&profile["reversibility"], "neutral_reset_available") != Some(true)
        || bool_at(&profile["privacy"], "local_export_enabled") != Some(true)
        || bool_at(&profile["privacy"], "local_delete_enabled") != Some(true)
    {
        errors.push(format!("{profile_id}: owner reset, export and delete must remain available"));
    }

    errors
}

fn expected_violation_present(control: &Value, profile: &Value, profile_schema: &Value) -> bool {
    let expected = text_at(control, "expected_rejection");
    match expected {
        Some("unknown_property") => {
            let property = text_at(control, "property").unwrap_or("");
            !profile_schema["properties"]
                .as_object()
                .is_some_and(|properties| properties.contains_key(property))
                && profile.get(property).is_some()
        }
        Some("safety_policy_override") => {
            text_at(
                &profile["interaction_policy"],
                "safety_critical_settings",
            ) != Some("manufacturer_controlled_non_overridable")
        }
        Some("moving_ui_not_restricted") => {
            text_at(&profile["interaction_policy"], "moving_ui_policy")
                != Some("minimal_low_distraction")
                || text_at(
                    &profile["interaction_policy"],
                    "profile_changes_while_moving",
                ) != Some("blocked")
        }
        Some("cloud_sync_without_consent") => {
            text_at(&profile["privacy"], "cloud_sync_mode") == Some("explicit_opt_in")
                && bool_at(&profile["privacy"], "cloud_sync_consent") != Some(true)
        }
        Some("telemetry_without_consent") => {
            text_at(&profile["privacy"], "telemetry_mode") == Some("explicit_opt_in")
                && bool_at(&profile["privacy"], "telemetry_consent") != Some(true)
        }
        Some("guest_data_persistence") => {
            text_at(&profile["privacy"], "profile_scope") == Some("guest")
                && (bool_at(&profile["privacy"], "clear_on_session_end") != Some(true)
                    || profile["privacy"]["retention_days"]
                        .as_u64()
                        .unwrap_or(u64::MAX)
                        > 1
                    || text_at(&profile["privacy"], "cloud_sync_mode") != Some("disabled")
                    || text_at(&profile["privacy"], "telemetry_mode") != Some("disabled"))
        }
        Some("excessive_ambient_intensity") => {
            profile["cabin_preferences"]["ambient_intensity_fraction"]
                .as_f64()
                .is_some_and(|intensity| intensity > 0.25)
        }
        Some("irreversible_profile_customization") => {
            bool_at(&profile["reversibility"], "neutral_reset_available") != Some(true)
                || text_at(&profile["reversibility"], "settings_export_format")
                    != Some("versioned_open_json")
                || bool_at(
                    &profile["reversibility"],
                    "personalization_modules_replaceable",
                ) != Some(true)
        }
        _ => false,
    }
}

fn mutate_profile(base: &Value, control: &Value) -> Result<Value, String> {
    let operation = text_at(control, "operation").ok_or("negative control lacks operation")?;
    let property = text_at(control, "property").ok_or("negative control lacks property")?;
    let mutation = control
        .get("mutation_value")
        .ok_or("negative control lacks mutation_value")?;
    let mut profile = base.clone();

    match operation {
        "add_root_property" => {
            let object = profile
                .as_object_mut()
                .ok_or("profile must be an object")?;
            object.insert(property.to_owned(), mutation.clone());
        }
        "replace_interaction_policy" if property == "interaction_policy" => {
            profile["interaction_policy"] = mutation.clone();
        }
        "replace_privacy" if property == "privacy" => {
            profile["privacy"] = mutation.clone();
        }
        "replace_cabin_preferences" if property == "cabin_preferences" => {
            profile["cabin_preferences"] = mutation.clone();
        }
        "replace_reversibility" if property == "reversibility" => {
            profile["reversibility"] = mutation.clone();
        }
        other => return Err(format!("unsupported mutation operation/property: {other}/{property}")),
    }
    Ok(profile)
}

fn control_ids(value: &Value, errors: &mut Errors) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let Some(controls) = value.get("negative_controls").and_then(Value::as_array) else {
        errors.push("negative_controls must be an array".to_owned());
        return seen;
    };
    for (index, control) in controls.iter().enumerate() {
        match text_at(control, "id") {
            Some(id) if !seen.insert(id.to_owned()) => {
                errors.push(format!("negative_controls[{index}]: duplicate id {id}"));
            }
            Some(_) => {}
            None => errors.push(format!("negative_controls[{index}]: missing id")),
        }
    }
    seen
}

fn audit(profile_set_schema: &Value, fixture: &Value) -> Result<AuditSummary, Errors> {
    let set_validator = schema_validator(profile_set_schema, PROFILE_SET_SCHEMA_ID)
        .map_err(|error| vec![error])?;
    let profile_schema = profile_set_schema
        .get("$defs")
        .and_then(|defs| defs.get("profile"))
        .ok_or_else(|| vec!["profile schema missing $defs.profile".to_owned()])?;
    let profile_validator = draft202012::options()
        .should_validate_formats(true)
        .build(profile_schema)
        .map_err(|error| vec![format!("cannot compile nested profile schema: {error}")])?;

    let mut errors = instance_errors(&set_validator, fixture, "profile set");
    if text_at(fixture, "schema_version") != Some(PROFILE_SET_VERSION) {
        errors.push(format!("schema_version must be {PROFILE_SET_VERSION}"));
    }
    if text_at(fixture, "evidence_class") != Some("synthetic_profile_fixture") {
        errors.push("profile set must remain synthetic".to_owned());
    }
    if text_at(fixture, "claim_ceiling")
        != Some("synthetic profile-contract and policy-guard tests only; no production vehicle, human-factors, accessibility or safety claim")
    {
        errors.push("claim ceiling must remain explicit".to_owned());
    }

    let profiles = fixture.get("profiles").and_then(Value::as_array);
    let mut valid_profiles = BTreeSet::new();
    if let Some(profiles) = profiles {
        for (index, profile) in profiles.iter().enumerate() {
            let id = text_at(profile, "profile_id").unwrap_or("");
            if id.is_empty() {
                errors.push(format!("profiles[{index}]: profile_id is missing"));
            } else if !valid_profiles.insert(id.to_owned()) {
                errors.push(format!("profiles[{index}]: duplicate profile_id {id}"));
            }
            errors.extend(instance_errors(
                &profile_validator,
                profile,
                &format!("profiles[{index}]"),
            ));
            errors.extend(profile_semantic_errors(profile));
            if text_at(profile, "evidence_class") != Some("synthetic_profile_fixture") {
                errors.push(format!("profiles[{index}]: profile must be synthetic"));
            }
        }
    }

    let expected_profile_ids: BTreeSet<String> =
        PROFILE_IDS.iter().map(|id| (*id).to_owned()).collect();
    if valid_profiles != expected_profile_ids {
        errors.push("profile fixture must contain the four frozen synthetic user profiles".to_owned());
    }

    let controls = fixture
        .get("negative_controls")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["negative_controls must be an array".to_owned()])?;
    let seen_controls = control_ids(fixture, &mut errors);
    let expected_control_ids: BTreeSet<String> =
        CONTROL_IDS.iter().map(|id| (*id).to_owned()).collect();
    if seen_controls != expected_control_ids {
        errors.push("negative controls must match the seven frozen personalization anti-patterns".to_owned());
    }

    let mut rejected_control_count = 0;
    let mut false_accept_count = 0;
    for (index, control) in controls.iter().enumerate() {
        if bool_at(control, "must_reject") != Some(true) {
            errors.push(format!("negative_controls[{index}]: must_reject must be true"));
        }
        let control_id = text_at(control, "id").unwrap_or("<unknown-control>");
        let profile_id = text_at(control, "profile_id").unwrap_or("");
        let Some(base_profile) = profiles.and_then(|items| {
            items
                .iter()
                .find(|profile| text_at(profile, "profile_id") == Some(profile_id))
        }) else {
            errors.push(format!("{control_id}: target profile {profile_id} does not exist"));
            false_accept_count += 1;
            continue;
        };
        let mutated = match mutate_profile(base_profile, control) {
            Ok(profile) => profile,
            Err(error) => {
                errors.push(format!("{control_id}: {error}"));
                false_accept_count += 1;
                continue;
            }
        };

        let schema_errors = instance_errors(&profile_validator, &mutated, control_id);
        let semantic_errors = profile_semantic_errors(&mutated);
        let rejected = !schema_errors.is_empty() || !semantic_errors.is_empty();
        let correct_violation = expected_violation_present(control, &mutated, profile_schema);
        if rejected && correct_violation {
            rejected_control_count += 1;
        } else {
            false_accept_count += 1;
            errors.push(format!(
                "{control_id}: expected rejection {} was not demonstrated; schema_errors={}, semantic_errors={}, expected_violation={correct_violation}",
                text_at(control, "expected_rejection").unwrap_or("<missing>"),
                schema_errors.len(),
                semantic_errors.len()
            ));
        }
    }

    if errors.is_empty() {
        Ok(AuditSummary {
            profile_count: profiles.map_or(0, Vec::len),
            control_count: controls.len(),
            rejected_control_count,
            false_accept_count,
        })
    } else {
        Err(errors)
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_json(path: &PathBuf) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    Ok((bytes, value))
}

fn main() -> ExitCode {
    let mut schema_path = PathBuf::from(
        "schemas/transport/vehicle_personalization_profile_set_v0_1.schema.json",
    );
    let mut fixture_path =
        PathBuf::from("fixtures/personal_vehicle/vehicle_personalization_profiles_v0_1.json");
    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let target = match args[index].as_str() {
            "--schema" => Some(&mut schema_path),
            "--fixture" => Some(&mut fixture_path),
            _ => None,
        };
        if let Some(path) = target {
            if index + 1 >= args.len() {
                eprintln!("missing path after {}", args[index]);
                return ExitCode::from(2);
            }
            *path = PathBuf::from(&args[index + 1]);
            index += 2;
        } else {
            eprintln!("unknown argument: {}\nusage: vehicle-personalization-audit [--schema PATH] [--fixture PATH]", args[index]);
            return ExitCode::from(2);
        }
    }

    let (schema_bytes, schema) = match read_json(&schema_path) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let (fixture_bytes, fixture) = match read_json(&fixture_path) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };

    match audit(&schema, &fixture) {
        Ok(summary) => {
            let report = json!({
                "report_version": "vehicle-personalization-audit-report/0.1",
                "evidence_class": "synthetic_contract_audit",
                "claim_ceiling": "profile schema and seeded policy-guard tests only; no real user, production vehicle, safety or accessibility validation",
                "schema_sha256": sha256(&schema_bytes),
                "fixture_sha256": sha256(&fixture_bytes),
                "profile_count": summary.profile_count,
                "negative_control_count": summary.control_count,
                "correctly_rejected_control_count": summary.rejected_control_count,
                "false_accept_count": summary.false_accept_count,
                "digest_replay_requirement": "same exact bytes must produce the same report inputs and counts"
            });
            match serde_json::to_string_pretty(&report) {
                Ok(text) => { println!("{text}"); ExitCode::SUCCESS }
                Err(error) => { eprintln!("cannot serialize report: {error}"); ExitCode::FAILURE }
            }
        }
        Err(errors) => {
            eprintln!("vehicle personalization profile audit: FAIL");
            for error in errors { eprintln!("- {error}"); }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frozen_values() -> (Value, Value, Validator) {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/vehicle_personalization_profile_set_v0_1.schema.json"
        )).expect("profile schema parses");
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/vehicle_personalization_profiles_v0_1.json"
        )).expect("profile fixture parses");
        let validator = schema_validator(&schema, PROFILE_SET_SCHEMA_ID)
            .expect("profile set schema compiles");
        (schema, fixture, validator)
    }

    #[test]
    fn accepts_four_synthetic_profiles_and_eight_controls() {
        let (schema, fixture, _) = frozen_values();
        let result = audit(&schema, &fixture).expect("frozen personalization contract");
        assert_eq!(result.profile_count, 4);
        assert_eq!(result.control_count, 8);
        assert_eq!(result.rejected_control_count, 8);
        assert_eq!(result.false_accept_count, 0);
    }

    #[test]
    fn draft_2020_12_schema_validates_the_profile_set_fixture() {
        let (_, fixture, validator) = frozen_values();
        assert!(validator.is_valid(&fixture));
    }

    #[test]
    fn rejects_unlisted_root_safety_control() {
        let (schema, fixture, _) = frozen_values();
        let profiles = fixture["profiles"].as_array().expect("profiles");
        let base = profiles.iter()
            .find(|profile| text_at(profile, "profile_id") == Some("vpf_daily001"))
            .expect("daily profile");
        let mutated = json!({
            "schema_version": base["schema_version"],
            "profile_id": base["profile_id"],
            "label": base["label"],
            "evidence_class": base["evidence_class"],
            "display_preferences": base["display_preferences"],
            "cabin_preferences": base["cabin_preferences"],
            "audio_preferences": base["audio_preferences"],
            "accessibility_preferences": base["accessibility_preferences"],
            "interaction_policy": base["interaction_policy"],
            "privacy": base["privacy"],
            "reversibility": base["reversibility"],
            "steering_response_curve": "custom"
        });
        let profile_schema = &schema["$defs"]["profile"];
        let profile_validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!profile_validator.is_valid(&mutated));
    }

    #[test]
    fn rejects_cloud_sync_without_per_profile_consent() {
        let (schema, fixture, _) = frozen_values();
        let base = fixture["profiles"][0].clone();
        let mut mutated = base;
        mutated["privacy"]["cloud_sync_mode"] = json!("explicit_opt_in");
        mutated["privacy"]["cloud_sync_consent"] = json!(false);
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }

    #[test]
    fn rejects_guest_profile_persistence() {
        let (schema, fixture, _) = frozen_values();
        let mut mutated = fixture["profiles"][3].clone();
        mutated["privacy"]["clear_on_session_end"] = json!(false);
        mutated["privacy"]["retention_days"] = json!(90);
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }

    #[test]
    fn rejects_unrestricted_moving_ui() {
        let (schema, fixture, _) = frozen_values();
        let mut mutated = fixture["profiles"][0].clone();
        mutated["interaction_policy"]["moving_ui_policy"] = json!("full_custom_interactive");
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }

    #[test]
    fn rejects_user_override_of_safety_critical_settings() {
        let (schema, fixture, _) = frozen_values();
        let mut mutated = fixture["profiles"][0].clone();
        mutated["interaction_policy"]["safety_critical_settings"] = json!("user_customizable");
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }

    #[test]
    fn rejects_guest_cloud_sync_even_when_consent_is_present() {
        let (schema, fixture, _) = frozen_values();
        let mut mutated = fixture["profiles"][3].clone();
        mutated["privacy"]["cloud_sync_mode"] = json!("explicit_opt_in");
        mutated["privacy"]["cloud_sync_consent"] = json!(true);
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }

    #[test]
    fn high_contrast_theme_requires_high_contrast_controls() {
        let (_, fixture, _) = frozen_values();
        let mut mutated = fixture["profiles"][1].clone();
        mutated["display_preferences"]["contrast"] = json!("standard");
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }

    #[test]
    fn invalid_ambient_intensity_is_rejected() {
        let (schema, fixture, _) = frozen_values();
        let mut mutated = fixture["profiles"][0].clone();
        mutated["cabin_preferences"]["ambient_intensity_fraction"] = json!(1.0);
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
    }


    #[test]
    fn rejects_nonreversible_profile_changes() {
        let (schema, fixture, _) = frozen_values();
        let base = fixture["profiles"][0].clone();
        let control = fixture["negative_controls"].as_array().expect("controls")
            .iter()
            .find(|item| text_at(item, "id") == Some("NC-PERS-08"))
            .expect("reversibility control");
        let mutated = mutate_profile(&base, control).expect("mutation applies");
        let profile_schema = &schema["$defs"]["profile"];
        let validator = draft202012::options().build(profile_schema).expect("profile schema");
        assert!(!validator.is_valid(&mutated));
        assert!(!profile_semantic_errors(&mutated).is_empty());
    }
    #[test]
    fn exact_input_replay_keeps_fixture_digest_stable() {
        let bytes = include_bytes!(
            "../../../fixtures/personal_vehicle/vehicle_personalization_profiles_v0_1.json"
        );
        assert_eq!(sha256(bytes), sha256(bytes));
        let (schema, fixture, _) = frozen_values();
        let first = audit(&schema, &fixture).expect("first audit");
        let second = audit(&schema, &fixture).expect("second audit");
        assert_eq!(first, second);
    }
}

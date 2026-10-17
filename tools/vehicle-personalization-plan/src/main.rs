use jsonschema::Validator;
use jsonschema::draft202012;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const PROFILE_SCHEMA_ID: &str = "urn:luminous-dynamics:vehicle:personalization-profile-set:0.1";
const CAPABILITY_SCHEMA_ID: &str =
    "urn:luminous-dynamics:vehicle:personalization-capability-set:0.1";
const PROFILE_SET_VERSION: &str = "vehicle-personalization-profile-set/0.1";
const CAPABILITY_SET_VERSION: &str = "vehicle-personalization-capability-set/0.1";

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn schema_validator(schema: &Value, expected_id: &str) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(expected_id) {
        return Err(format!("schema $id must be {expected_id}"));
    }
    if !draft202012::meta::is_valid(schema) {
        return Err(format!(
            "schema {expected_id} fails Draft 2020-12 meta-schema validation"
        ));
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile schema {expected_id}: {error}"))
}

fn validate_instance(
    validator: &Validator,
    instance: &Value,
    label: &str,
) -> Result<(), Vec<String>> {
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| format!("{label}: {error}"))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn has_value(features: &Value, array_key: &str, requested: &Value) -> bool {
    features
        .get(array_key)
        .and_then(Value::as_array)
        .is_some_and(|values| values.contains(requested))
}

fn decide(
    profile_id: &str,
    feature: &str,
    requested_value: Value,
    context: &str,
    supported: bool,
    unsupported_reason: &str,
) -> Value {
    let (disposition, reason) = if requested_value.is_null() {
        (
            "not_requested",
            "no stored preference is present; no default or identity is inferred",
        )
    } else if context == "moving" {
        (
            "defer_until_parked",
            "profile changes are blocked while moving; no setting is applied by this planner",
        )
    } else if supported {
        (
            "preview_apply_if_owner_confirms",
            "requested preference is within the declared synthetic capability envelope",
        )
    } else {
        ("preserve_unapplied", unsupported_reason)
    };

    json!({
        "profile_id": profile_id,
        "feature": feature,
        "requested_value": requested_value,
        "disposition": disposition,
        "reason": reason
    })
}

fn compile_profile_plan(profile: &Value, capability_set: &Value) -> Result<Value, String> {
    let profile_id = text_at(profile, "profile_id").ok_or("profile_id missing")?;
    let capability_id = text_at(capability_set, "id").ok_or("capability-set id missing")?;
    let context = text_at(capability_set, "context").ok_or("capability context missing")?;
    if !["parked", "moving"].contains(&context) {
        return Err(format!("{capability_id}: unknown context {context}"));
    }
    if bool_at(capability_set, "profile_changes_blocked_while_moving") != Some(true) {
        return Err(format!(
            "{capability_id}: moving profile edits must remain blocked"
        ));
    }

    let features = capability_set
        .get("features")
        .ok_or("capability features missing")?;
    let display = profile
        .get("display_preferences")
        .ok_or("display_preferences missing")?;
    let cabin = profile
        .get("cabin_preferences")
        .ok_or("cabin_preferences missing")?;
    let audio = profile
        .get("audio_preferences")
        .ok_or("audio_preferences missing")?;
    let accessibility = profile
        .get("accessibility_preferences")
        .ok_or("accessibility_preferences missing")?;

    let requested_theme = display.get("theme").cloned().unwrap_or(Value::Null);
    let theme_supported = has_value(features, "supported_themes", &requested_theme);

    let requested_scale = number_at(display, "text_scale").ok_or("text_scale missing")?;
    let maximum_scale = number_at(features, "max_text_scale").ok_or("max_text_scale missing")?;
    let scale_supported = requested_scale <= maximum_scale;

    let requested_contrast = text_at(display, "contrast").ok_or("contrast missing")?;
    let contrast_supported = requested_contrast != "high"
        || has_value(features, "supported_themes", &json!("high_contrast"));

    let reduced_motion_requested =
        bool_at(display, "reduced_motion").ok_or("reduced_motion missing")?;
    let reduced_motion_supported =
        !reduced_motion_requested || bool_at(features, "supports_reduced_motion") == Some(true);

    let requested_temperature =
        number_at(cabin, "preferred_temperature_c").ok_or("preferred_temperature_c missing")?;
    let temperature_range = features
        .get("allowed_temperature_range_c")
        .and_then(Value::as_array)
        .ok_or("allowed_temperature_range_c missing")?;
    if temperature_range.len() != 2 {
        return Err(format!(
            "{capability_id}: temperature range must contain exactly two values"
        ));
    }
    let minimum_temperature = temperature_range[0]
        .as_f64()
        .ok_or("minimum temperature is not numeric")?;
    let maximum_temperature = temperature_range[1]
        .as_f64()
        .ok_or("maximum temperature is not numeric")?;
    if minimum_temperature > maximum_temperature {
        return Err(format!("{capability_id}: inverted temperature range"));
    }
    let temperature_supported = bool_at(features, "supports_climate_control") == Some(true)
        && (minimum_temperature..=maximum_temperature).contains(&requested_temperature);

    let requested_ambient_color = cabin.get("ambient_light").cloned().unwrap_or(Value::Null);
    let ambient_supported = bool_at(features, "supports_ambient_lighting") == Some(true)
        && has_value(
            features,
            "supported_ambient_colors",
            &requested_ambient_color,
        );
    let requested_ambient_intensity = number_at(cabin, "ambient_intensity_fraction")
        .ok_or("ambient_intensity_fraction missing")?;
    let maximum_ambient_intensity = number_at(features, "max_ambient_intensity_fraction")
        .ok_or("max_ambient_intensity_fraction missing")?;
    let ambient_intensity_supported = bool_at(features, "supports_ambient_lighting") == Some(true)
        && requested_ambient_intensity <= maximum_ambient_intensity;

    let requested_seat_ref = cabin.get("seat_preset_ref").cloned().unwrap_or(Value::Null);
    let seat_supported =
        requested_seat_ref.is_null() || bool_at(features, "supports_seat_memory") == Some(true);

    let requested_listening_style = audio.get("listening_style").cloned().unwrap_or(Value::Null);
    let listening_supported = has_value(
        features,
        "supported_listening_styles",
        &requested_listening_style,
    );
    let requested_cue_style = audio
        .get("preferred_cue_style")
        .cloned()
        .unwrap_or(Value::Null);
    let cue_supported = has_value(features, "supported_cue_styles", &requested_cue_style);
    let requested_volume =
        number_at(audio, "preferred_volume_fraction").ok_or("preferred_volume_fraction missing")?;
    let maximum_volume = number_at(features, "max_preferred_volume_fraction")
        .ok_or("max_preferred_volume_fraction missing")?;
    let volume_supported = requested_volume <= maximum_volume;

    let large_controls_requested =
        bool_at(accessibility, "large_controls").ok_or("large_controls missing")?;
    let large_controls_supported =
        !large_controls_requested || bool_at(features, "supports_large_controls") == Some(true);
    let captions =
        text_at(accessibility, "optional_captions").ok_or("optional_captions missing")?;
    let captions_supported =
        captions == "off" || bool_at(features, "supports_captions") == Some(true);
    let tactile_requested =
        bool_at(accessibility, "tactile_feedback").ok_or("tactile_feedback missing")?;
    let tactile_supported =
        !tactile_requested || bool_at(features, "supports_tactile_feedback") == Some(true);
    let requested_voice = accessibility
        .get("voice_assistance")
        .cloned()
        .unwrap_or(Value::Null);
    let voice_supported = has_value(features, "supported_voice_modes", &requested_voice);

    let mut decisions = vec![
        decide(
            profile_id,
            "display.theme",
            requested_theme,
            context,
            theme_supported,
            "vehicle configuration does not declare support for the requested theme",
        ),
        decide(
            profile_id,
            "display.text_scale",
            json!(requested_scale),
            context,
            scale_supported,
            "requested text scale exceeds the vehicle's declared supported maximum; preference is not clamped",
        ),
        decide(
            profile_id,
            "display.contrast",
            json!(requested_contrast),
            context,
            contrast_supported,
            "high-contrast presentation is unavailable in the declared vehicle capabilities",
        ),
        decide(
            profile_id,
            "display.reduced_motion",
            json!(reduced_motion_requested),
            context,
            reduced_motion_supported,
            "reduced-motion display behavior is not declared as supported",
        ),
        decide(
            profile_id,
            "cabin.preferred_temperature_c",
            json!(requested_temperature),
            context,
            temperature_supported,
            "climate support is absent or the requested temperature lies outside the declared range",
        ),
        decide(
            profile_id,
            "cabin.ambient_light",
            requested_ambient_color,
            context,
            ambient_supported,
            "ambient lighting or the requested color is not declared as supported",
        ),
        decide(
            profile_id,
            "cabin.ambient_intensity_fraction",
            json!(requested_ambient_intensity),
            context,
            ambient_intensity_supported,
            "requested ambient intensity exceeds the validated capability; preference is not clamped",
        ),
        decide(
            profile_id,
            "cabin.seat_preset_ref",
            requested_seat_ref,
            context,
            seat_supported,
            "seat-memory capability is not declared as supported; the reference is preserved, not applied",
        ),
        decide(
            profile_id,
            "audio.listening_style",
            requested_listening_style,
            context,
            listening_supported,
            "requested listening style is not declared as supported",
        ),
        decide(
            profile_id,
            "audio.preferred_cue_style",
            requested_cue_style,
            context,
            cue_supported,
            "requested cue style is not declared as supported",
        ),
        decide(
            profile_id,
            "audio.preferred_volume_fraction",
            json!(requested_volume),
            context,
            volume_supported,
            "requested preferred volume exceeds the vehicle's declared preference envelope; it is not clamped",
        ),
        decide(
            profile_id,
            "accessibility.large_controls",
            json!(large_controls_requested),
            context,
            large_controls_supported,
            "large-control presentation is not declared as supported",
        ),
        decide(
            profile_id,
            "accessibility.optional_captions",
            json!(captions),
            context,
            captions_supported,
            "captions are not declared as supported; preference is preserved",
        ),
        decide(
            profile_id,
            "accessibility.tactile_feedback",
            json!(tactile_requested),
            context,
            tactile_supported,
            "tactile feedback is not declared as supported",
        ),
        decide(
            profile_id,
            "accessibility.voice_assistance",
            requested_voice,
            context,
            voice_supported,
            "requested voice mode is not declared as supported",
        ),
    ];

    if context != "parked"
        && decisions.iter().any(|decision| {
            text_at(decision, "disposition") == Some("preview_apply_if_owner_confirms")
        })
    {
        return Err(format!(
            "{capability_id}: moving context produced an applicable preference"
        ));
    }

    let privacy = profile
        .get("privacy")
        .ok_or("privacy preferences missing")?;
    let safety = profile
        .get("interaction_policy")
        .ok_or("interaction policy missing")?;
    let report = json!({
        "plan_id": format!("{profile_id}::{capability_id}"),
        "profile_id": profile_id,
        "capability_set_id": capability_id,
        "vehicle_configuration_ref": text_at(capability_set, "vehicle_configuration_ref"),
        "context": context,
        "evidence_class": "synthetic_capability_match_output",
        "preview_only": true,
        "hardware_or_vehicle_control_calls": 0,
        "safety_boundary": {
            "safety_critical_settings": text_at(safety, "safety_critical_settings"),
            "profile_changes_while_moving": text_at(safety, "profile_changes_while_moving"),
            "moving_context_defers_changes": context == "moving",
            "vehicle_dynamics_or_controls_modified": false
        },
        "privacy_boundary": {
            "storage_mode": text_at(privacy, "storage_mode"),
            "cloud_sync_mode": text_at(privacy, "cloud_sync_mode"),
            "telemetry_mode": text_at(privacy, "telemetry_mode"),
            "local_export_enabled": bool_at(privacy, "local_export_enabled"),
            "local_delete_enabled": bool_at(privacy, "local_delete_enabled"),
            "profile_preferences_written_to_vehicle": false
        },
        "decisions": decisions,
        "summary": {
            "requested_preference_count": decisions.iter().filter(|item| text_at(item, "disposition") != Some("not_requested")).count(),
            "preview_apply_count": decisions.iter().filter(|item| text_at(item, "disposition") == Some("preview_apply_if_owner_confirms")).count(),
            "preserved_unapplied_count": decisions.iter().filter(|item| text_at(item, "disposition") == Some("preserve_unapplied")).count(),
            "deferred_until_parked_count": decisions.iter().filter(|item| text_at(item, "disposition") == Some("defer_until_parked")).count()
        },
        "claim_ceiling": "synthetic capability matching and preview planning only; no actuation, real-vehicle capability claim, user study or safety validation"
    });
    Ok(report)
}

fn build_report(
    profile_schema_bytes: &[u8],
    profile_set_bytes: &[u8],
    capability_schema_bytes: &[u8],
    capability_set_bytes: &[u8],
    profile_set: &Value,
    capability_set: &Value,
) -> Result<Value, String> {
    let profile_sets = profile_set
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or("profile set profiles must be an array")?;
    let capabilities = capability_set
        .get("capability_sets")
        .and_then(Value::as_array)
        .ok_or("capability_sets must be an array")?;

    let mut plans = Vec::new();
    for profile in profile_sets {
        for capability in capabilities {
            plans.push(compile_profile_plan(profile, capability)?);
        }
    }

    let core = json!({
        "report_version": "vehicle-personalization-plan-report/0.1",
        "evidence_class": "synthetic_capability_match_output",
        "preview_only": true,
        "hardware_or_vehicle_control_calls": 0,
        "input_digests": {
            "profile_schema_sha256": sha256(profile_schema_bytes),
            "profile_set_sha256": sha256(profile_set_bytes),
            "capability_schema_sha256": sha256(capability_schema_bytes),
            "capability_set_sha256": sha256(capability_set_bytes)
        },
        "profile_count": profile_sets.len(),
        "capability_set_count": capabilities.len(),
        "plan_count": plans.len(),
        "plans": plans,
        "claim_ceiling": "synthetic capability matching and preview planning only; no actuation, real-vehicle capability claim, user study or safety validation",
        "limitations": [
            "Capability manifests are synthetic and do not identify the equipment in any real vehicle.",
            "The planner emits recommendations only; it does not communicate with actuators, vehicle buses or a deployed UI.",
            "A supported value is only a candidate for owner confirmation in the parked context; a real implementation must independently enforce hard safety and vehicle-configuration constraints.",
            "The model does not measure user satisfaction, comfort, accessibility, workload or distraction."
        ]
    });
    let digest_input = serde_json::to_vec(&core)
        .map_err(|error| format!("cannot serialize plan report: {error}"))?;
    let mut report = core;
    report["report_digest_sha256"] = json!(sha256(&digest_input));
    Ok(report)
}

fn read_json(path: &PathBuf) -> Result<(Vec<u8>, Value), String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    Ok((bytes, value))
}

fn main() -> ExitCode {
    let mut profile_schema_path =
        PathBuf::from("schemas/transport/vehicle_personalization_profile_set_v0_1.schema.json");
    let mut profile_set_path =
        PathBuf::from("fixtures/personal_vehicle/vehicle_personalization_profiles_v0_1.json");
    let mut capability_schema_path =
        PathBuf::from("schemas/transport/vehicle_personalization_capability_set_v0_1.schema.json");
    let mut capability_set_path = PathBuf::from(
        "fixtures/personal_vehicle/vehicle_personalization_capability_sets_v0_1.json",
    );

    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let target = match args[index].as_str() {
            "--profile-schema" => Some(&mut profile_schema_path),
            "--profile-set" => Some(&mut profile_set_path),
            "--capability-schema" => Some(&mut capability_schema_path),
            "--capabilities" => Some(&mut capability_set_path),
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
            eprintln!(
                "unknown argument: {}\nusage: vehicle-personalization-plan [--profile-schema PATH] [--profile-set PATH] [--capability-schema PATH] [--capabilities PATH]",
                args[index]
            );
            return ExitCode::from(2);
        }
    }

    let (profile_schema_bytes, profile_schema) = match read_json(&profile_schema_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let (profile_set_bytes, profile_set) = match read_json(&profile_set_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let (capability_schema_bytes, capability_schema) = match read_json(&capability_schema_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let (capability_set_bytes, capability_set) = match read_json(&capability_set_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let profile_validator = match schema_validator(&profile_schema, PROFILE_SCHEMA_ID) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let capability_validator = match schema_validator(&capability_schema, CAPABILITY_SCHEMA_ID) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    if text_at(&profile_set, "schema_version") != Some(PROFILE_SET_VERSION)
        || text_at(&profile_set, "evidence_class") != Some("synthetic_profile_fixture")
    {
        eprintln!("profile set version/evidence class mismatch");
        return ExitCode::FAILURE;
    }
    if text_at(&capability_set, "schema_version") != Some(CAPABILITY_SET_VERSION)
        || text_at(&capability_set, "evidence_class") != Some("synthetic_capability_fixture")
    {
        eprintln!("capability set version/evidence class mismatch");
        return ExitCode::FAILURE;
    }

    for (validator, value, label) in [
        (&profile_validator, &profile_set, "profile set"),
        (&capability_validator, &capability_set, "capability set"),
    ] {
        if let Err(errors) = validate_instance(validator, value, label) {
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::FAILURE;
        }
    }

    match build_report(
        &profile_schema_bytes,
        &profile_set_bytes,
        &capability_schema_bytes,
        &capability_set_bytes,
        &profile_set,
        &capability_set,
    ) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize report: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("personalization plan failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen_values() -> (Value, Value, Value, Value) {
        let profile_schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/vehicle_personalization_profile_set_v0_1.schema.json"
        ))
        .expect("profile schema");
        let profile_set: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/vehicle_personalization_profiles_v0_1.json"
        ))
        .expect("profile set");
        let capability_schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/vehicle_personalization_capability_set_v0_1.schema.json"
        ))
        .expect("capability schema");
        let capability_set: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/vehicle_personalization_capability_sets_v0_1.json"
        ))
        .expect("capability set");
        (
            profile_schema,
            profile_set,
            capability_schema,
            capability_set,
        )
    }

    fn report_fixture() -> Value {
        let (profile_schema, profile_set, capability_schema, capability_set) = frozen_values();
        let profile_schema_bytes = serde_json::to_vec(&profile_schema).expect("schema bytes");
        let profile_set_bytes = serde_json::to_vec(&profile_set).expect("profile bytes");
        let capability_schema_bytes =
            serde_json::to_vec(&capability_schema).expect("capability schema bytes");
        let capability_set_bytes = serde_json::to_vec(&capability_set).expect("capability bytes");
        build_report(
            &profile_schema_bytes,
            &profile_set_bytes,
            &capability_schema_bytes,
            &capability_set_bytes,
            &profile_set,
            &capability_set,
        )
        .expect("report builds")
    }

    fn find_plan<'a>(report: &'a Value, profile_id: &str, capability_id: &str) -> &'a Value {
        report["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .find(|plan| {
                text_at(plan, "profile_id") == Some(profile_id)
                    && text_at(plan, "capability_set_id") == Some(capability_id)
            })
            .expect("plan exists")
    }

    fn find_decision<'a>(plan: &'a Value, feature: &str) -> &'a Value {
        plan["decisions"]
            .as_array()
            .expect("decisions")
            .iter()
            .find(|decision| text_at(decision, "feature") == Some(feature))
            .expect("decision exists")
    }

    #[test]
    fn schema_fixtures_validate_with_draft_2020_12() {
        let (profile_schema, profile_set, capability_schema, capability_set) = frozen_values();
        let profile_validator =
            schema_validator(&profile_schema, PROFILE_SCHEMA_ID).expect("profile schema");
        let capability_validator =
            schema_validator(&capability_schema, CAPABILITY_SCHEMA_ID).expect("capability schema");
        assert!(profile_validator.is_valid(&profile_set));
        assert!(capability_validator.is_valid(&capability_set));
    }

    #[test]
    fn compiles_all_four_profiles_for_both_vehicle_contexts() {
        let report = report_fixture();
        assert_eq!(report["profile_count"].as_u64(), Some(4));
        assert_eq!(report["capability_set_count"].as_u64(), Some(2));
        assert_eq!(report["plan_count"].as_u64(), Some(8));
        assert_eq!(report["plans"].as_array().expect("plans").len(), 8);
        assert_eq!(
            report["evidence_class"],
            "synthetic_capability_match_output"
        );
    }

    #[test]
    fn moving_context_defers_all_requested_profile_changes() {
        let report = report_fixture();
        let plan = find_plan(&report, "vpf_access001", "demo_moving_vehicle");
        assert_eq!(plan["preview_only"], true);
        assert_eq!(plan["hardware_or_vehicle_control_calls"].as_u64(), Some(0));
        assert_eq!(
            plan["safety_boundary"]["moving_context_defers_changes"],
            true
        );
        for decision in plan["decisions"].as_array().expect("decisions") {
            if text_at(decision, "disposition") != Some("not_requested") {
                assert_eq!(decision["disposition"], "defer_until_parked");
            }
        }
    }

    #[test]
    fn supported_high_contrast_is_previewed_for_owner_confirmation() {
        let report = report_fixture();
        let plan = find_plan(&report, "vpf_access001", "demo_parked_vehicle");
        let decision = find_decision(plan, "display.theme");
        assert_eq!(decision["disposition"], "preview_apply_if_owner_confirms");
        assert_eq!(decision["requested_value"], "high_contrast");
    }

    #[test]
    fn unsupported_text_scale_is_preserved_without_clamping() {
        let report = report_fixture();
        let plan = find_plan(&report, "vpf_access001", "demo_parked_vehicle");
        let decision = find_decision(plan, "display.text_scale");
        assert_eq!(decision["disposition"], "preserve_unapplied");
        assert_eq!(decision["requested_value"].as_f64(), Some(1.3));
    }

    #[test]
    fn excessive_ambient_intensity_is_preserved_without_clamping() {
        let report = report_fixture();
        let plan = find_plan(&report, "vpf_daily001", "demo_parked_vehicle");
        let decision = find_decision(plan, "cabin.ambient_intensity_fraction");
        assert_eq!(decision["disposition"], "preserve_unapplied");
        assert_eq!(decision["requested_value"].as_f64(), Some(0.16));
    }

    #[test]
    fn unsupported_seat_memory_is_explained_not_silently_dropped() {
        let report = report_fixture();
        let plan = find_plan(&report, "vpf_access001", "demo_parked_vehicle");
        let decision = find_decision(plan, "cabin.seat_preset_ref");
        assert_eq!(decision["disposition"], "preserve_unapplied");
        assert_eq!(decision["requested_value"], "seat_access01");
        assert!(
            text_at(decision, "reason")
                .unwrap()
                .contains("not declared")
        );
    }

    #[test]
    fn unsupported_voice_mode_is_preserved() {
        let report = report_fixture();
        let plan = find_plan(&report, "vpf_access001", "demo_parked_vehicle");
        let decision = find_decision(plan, "accessibility.voice_assistance");
        assert_eq!(decision["disposition"], "preserve_unapplied");
        assert_eq!(decision["requested_value"], "hands_free_when_supported");
    }

    #[test]
    fn owner_privacy_and_recovery_guards_are_carried_into_every_plan() {
        let report = report_fixture();
        for plan in report["plans"].as_array().expect("plans") {
            assert_eq!(
                plan["privacy_boundary"]["storage_mode"],
                "device_local_encrypted"
            );
            assert_eq!(plan["privacy_boundary"]["local_export_enabled"], true);
            assert_eq!(plan["privacy_boundary"]["local_delete_enabled"], true);
            assert_eq!(
                plan["privacy_boundary"]["profile_preferences_written_to_vehicle"],
                false
            );
            assert_eq!(
                plan["safety_boundary"]["vehicle_dynamics_or_controls_modified"],
                false
            );
        }
    }

    #[test]
    fn identical_inputs_produce_identical_report_digest() {
        let first = report_fixture();
        let second = report_fixture();
        assert_eq!(
            first["report_digest_sha256"],
            second["report_digest_sha256"]
        );
        assert_eq!(first, second);
    }

    #[test]
    fn profile_preferences_cannot_override_safety_settings() {
        let (profile_schema, mut profile_set, capability_schema, capability_set) = frozen_values();
        profile_set["profiles"][0]["interaction_policy"]["safety_critical_settings"] =
            json!("user_customizable");
        let profile_validator =
            schema_validator(&profile_schema, PROFILE_SCHEMA_ID).expect("profile schema");
        assert!(!profile_validator.is_valid(&profile_set));
        let capability_validator =
            schema_validator(&capability_schema, CAPABILITY_SCHEMA_ID).expect("capability schema");
        assert!(capability_validator.is_valid(&capability_set));
    }

    #[test]
    fn moving_manifest_cannot_allow_profile_changes() {
        let (profile_schema, profile_set, capability_schema, mut capability_set) = frozen_values();
        capability_set["capability_sets"][1]["profile_changes_blocked_while_moving"] = json!(false);
        let profile_validator =
            schema_validator(&profile_schema, PROFILE_SCHEMA_ID).expect("profile schema");
        let capability_validator =
            schema_validator(&capability_schema, CAPABILITY_SCHEMA_ID).expect("capability schema");
        assert!(profile_validator.is_valid(&profile_set));
        assert!(!capability_validator.is_valid(&capability_set));
    }
}

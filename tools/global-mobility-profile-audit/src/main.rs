use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../fixtures/global_mobility/global_mobility_profiles_v0_1.json");

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    profiles: usize,
    negative_controls: usize,
    archetypes: usize,
    market_specific_evidence_required: usize,
}

fn check_keys(value: &Value, path: &str, required: &[&str], allowed: &[&str], errors: &mut Vec<String>) {
    let Some(object) = value.as_object() else {
        errors.push(format!("{path}: expected object"));
        return;
    };
    for key in required {
        if !object.contains_key(*key) {
            errors.push(format!("{path}: missing required key {key}"));
        }
    }
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("{path}: unknown key {key}"));
        }
    }
}

fn string_at<'a>(value: &'a Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<&'a str> {
    match value.get(key).and_then(Value::as_str) {
        Some(s) => Some(s),
        None => {
            errors.push(format!("{path}.{key}: expected string"));
            None
        }
    }
}

fn bool_at(value: &Value, key: &str, expected: bool, path: &str, errors: &mut Vec<String>) {
    match value.get(key).and_then(Value::as_bool) {
        Some(actual) if actual == expected => {}
        Some(actual) => errors.push(format!("{path}.{key}: expected {expected}, found {actual}")),
        None => errors.push(format!("{path}.{key}: expected boolean")),
    }
}

fn null_at(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) {
    if !value.get(key).is_some_and(Value::is_null) {
        errors.push(format!("{path}.{key}: expected null while evidence is unassessed"));
    }
}

fn status_at(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) {
    match value.get(key).and_then(Value::as_str) {
        Some("not_assessed") => {}
        Some(other) => errors.push(format!("{path}.{key}: synthetic corpus must say not_assessed, found {other}")),
        None => errors.push(format!("{path}.{key}: expected status string")),
    }
}

fn string_array_at(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Vec<String> {
    let Some(array) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{path}.{key}: expected array"));
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for (index, item) in array.iter().enumerate() {
        match item.as_str() {
            Some(s) => {
                if !seen.insert(s.to_string()) {
                    errors.push(format!("{path}.{key}[{index}]: duplicate item {s}"));
                }
                result.push(s.to_string());
            }
            None => errors.push(format!("{path}.{key}[{index}]: expected string")),
        }
    }
    result
}

fn require_array_has_min(value: &Value, key: &str, minimum: usize, path: &str, errors: &mut Vec<String>) {
    match value.get(key).and_then(Value::as_array) {
        Some(array) if array.len() >= minimum => {}
        Some(array) => errors.push(format!("{path}.{key}: expected at least {minimum} items, found {}", array.len())),
        None => errors.push(format!("{path}.{key}: expected array")),
    }
}

fn check_enum(value: &str, allowed: &[&str], path: &str, errors: &mut Vec<String>) {
    if !allowed.contains(&value) {
        errors.push(format!("{path}: unsupported value {value}"));
    }
}

fn audit_profile(profile: &Value, index: usize, errors: &mut Vec<String>) -> (bool, Option<String>) {
    let path = format!("profiles[{index}]");
    let profile_keys = [
        "profile_id", "profile_kind", "archetype", "context_constraints", "intended_user_segments",
        "platform_families", "energy_options", "market_stage", "infrastructure", "affordability",
        "regulatory", "serviceability", "digital_access", "language_access", "lifecycle",
        "required_evidence", "metrics", "negative_control", "expected_disposition", "rationale"
    ];
    check_keys(profile, &path, &profile_keys, &profile_keys, errors);

    let id = string_at(profile, "profile_id", &path, errors).map(str::to_owned);
    if string_at(profile, "profile_kind", &path, errors) != Some("synthetic_archetype") {
        errors.push(format!("{path}.profile_kind: this auditor only accepts synthetic archetypes"));
    }
    if string_at(profile, "market_stage", &path, errors) != Some("not_assessed") {
        errors.push(format!("{path}.market_stage: synthetic archetypes cannot establish a real market stage"));
    }
    const ARCHETYPES: &[&str] = &[
        "dense_urban_no_private_charging", "hot_dusty_remote_service", "cold_high_altitude",
        "humid_flood_prone", "mountain_rough_roads", "long_distance_rural_sparse_service",
        "island_import_dependent", "weak_or_intermittent_grid", "low_bandwidth_multilingual",
        "disaster_infrastructure_disruption", "cash_flow_constrained_purchase",
        "mixed_traffic_vulnerable_road_users", "accessibility_oriented_passenger_service"
    ];
    if let Some(archetype) = string_at(profile, "archetype", &path, errors) {
        check_enum(archetype, ARCHETYPES, &format!("{path}.archetype"), errors);
    }
    for key in ["context_constraints", "intended_user_segments", "platform_families", "energy_options", "required_evidence", "metrics"] {
        let values = string_array_at(profile, key, &path, errors);
        if values.is_empty() {
            errors.push(format!("{path}.{key}: must not be empty"));
        }
    }
    string_at(profile, "rationale", &path, errors);
    let negative = match profile.get("negative_control").and_then(Value::as_bool) {
        Some(v) => v,
        None => {
            errors.push(format!("{path}.negative_control: expected boolean"));
            false
        }
    };
    let expected = if negative { "block_global_availability_claim" } else { "requires_market_specific_evidence" };
    if string_at(profile, "expected_disposition", &path, errors) != Some(expected) {
        errors.push(format!("{path}.expected_disposition: expected {expected} for negative_control={negative}"));
    }

    let infra = &profile["infrastructure"];
    const INFRA_KEYS: &[&str] = &["charging_access", "fuel_access", "grid_reliability", "service_network", "parts_logistics", "digital_connectivity", "observed_values_included"];
    check_keys(infra, &format!("{path}.infrastructure"), INFRA_KEYS, INFRA_KEYS, errors);
    for key in ["charging_access", "fuel_access", "grid_reliability", "service_network", "parts_logistics", "digital_connectivity"] {
        status_at(infra, key, &format!("{path}.infrastructure"), errors);
    }
    bool_at(infra, "observed_values_included", false, &format!("{path}.infrastructure"), errors);

    let affordability = &profile["affordability"];
    const AFF_KEYS: &[&str] = &["local_currency_code", "price_status", "income_distribution_status", "ownership_cost_status", "target_segment_cost_threshold_frozen", "unknowns_remain_unknown", "required_cost_components"];
    check_keys(affordability, &format!("{path}.affordability"), AFF_KEYS, AFF_KEYS, errors);
    null_at(affordability, "local_currency_code", &format!("{path}.affordability"), errors);
    for key in ["price_status", "income_distribution_status", "ownership_cost_status"] {
        status_at(affordability, key, &format!("{path}.affordability"), errors);
    }
    bool_at(affordability, "target_segment_cost_threshold_frozen", false, &format!("{path}.affordability"), errors);
    bool_at(affordability, "unknowns_remain_unknown", true, &format!("{path}.affordability"), errors);
    require_array_has_min(affordability, "required_cost_components", 5, &format!("{path}.affordability"), errors);

    let regulatory = &profile["regulatory"];
    const REG_KEYS: &[&str] = &["jurisdiction_id", "status", "requirements_to_check", "standards_are_not_approval"];
    check_keys(regulatory, &format!("{path}.regulatory"), REG_KEYS, REG_KEYS, errors);
    null_at(regulatory, "jurisdiction_id", &format!("{path}.regulatory"), errors);
    status_at(regulatory, "status", &format!("{path}.regulatory"), errors);
    require_array_has_min(regulatory, "requirements_to_check", 5, &format!("{path}.regulatory"), errors);
    bool_at(regulatory, "standards_are_not_approval", true, &format!("{path}.regulatory"), errors);

    let serviceability = &profile["serviceability"];
    const SERVICE_KEYS: &[&str] = &["critical_parts_basket_status", "qualified_repair_coverage_status", "repair_manual_status", "offline_repair_docs_required", "critical_procedure_coverage_percent", "observed_data_included"];
    check_keys(serviceability, &format!("{path}.serviceability"), SERVICE_KEYS, SERVICE_KEYS, errors);
    for key in ["critical_parts_basket_status", "qualified_repair_coverage_status", "repair_manual_status"] {
        status_at(serviceability, key, &format!("{path}.serviceability"), errors);
    }
    bool_at(serviceability, "offline_repair_docs_required", true, &format!("{path}.serviceability"), errors);
    null_at(serviceability, "critical_procedure_coverage_percent", &format!("{path}.serviceability"), errors);
    bool_at(serviceability, "observed_data_included", false, &format!("{path}.serviceability"), errors);

    let digital = &profile["digital_access"];
    const DIGITAL_KEYS: &[&str] = &["basic_operation_without_cloud_required", "offline_safe_degraded_behavior_required", "owner_data_control_required", "cloud_required_for_basic_function", "privacy_by_default_required"];
    check_keys(digital, &format!("{path}.digital_access"), DIGITAL_KEYS, DIGITAL_KEYS, errors);
    for key in ["basic_operation_without_cloud_required", "offline_safe_degraded_behavior_required", "owner_data_control_required", "privacy_by_default_required"] {
        bool_at(digital, key, true, &format!("{path}.digital_access"), errors);
    }
    bool_at(digital, "cloud_required_for_basic_function", false, &format!("{path}.digital_access"), errors);

    let language = &profile["language_access"];
    const LANG_KEYS: &[&str] = &["manual_language_status", "local_units_support_required", "offline_manuals_required", "language_list"];
    check_keys(language, &format!("{path}.language_access"), LANG_KEYS, LANG_KEYS, errors);
    status_at(language, "manual_language_status", &format!("{path}.language_access"), errors);
    bool_at(language, "local_units_support_required", true, &format!("{path}.language_access"), errors);
    bool_at(language, "offline_manuals_required", true, &format!("{path}.language_access"), errors);
    let languages = string_array_at(language, "language_list", &format!("{path}.language_access"), errors);
    if !languages.is_empty() {
        errors.push(format!("{path}.language_access.language_list: synthetic panel must not assert market language coverage"));
    }

    let lifecycle = &profile["lifecycle"];
    const LIFE_KEYS: &[&str] = &["end_of_life_status", "material_traceability_status", "repair_reuse_route_required", "observed_impact_data_included"];
    check_keys(lifecycle, &format!("{path}.lifecycle"), LIFE_KEYS, LIFE_KEYS, errors);
    status_at(lifecycle, "end_of_life_status", &format!("{path}.lifecycle"), errors);
    status_at(lifecycle, "material_traceability_status", &format!("{path}.lifecycle"), errors);
    bool_at(lifecycle, "repair_reuse_route_required", true, &format!("{path}.lifecycle"), errors);
    bool_at(lifecycle, "observed_impact_data_included", false, &format!("{path}.lifecycle"), errors);

    (negative, id)
}

fn audit_corpus(corpus: &Value) -> Result<AuditSummary, Vec<String>> {
    let mut errors = Vec::new();
    const ROOT_KEYS: &[&str] = &["schema_version", "corpus_id", "provenance", "claim_ceiling", "profiles"];
    check_keys(corpus, "corpus", ROOT_KEYS, ROOT_KEYS, &mut errors);
    if string_at(corpus, "schema_version", "corpus", &mut errors) != Some("0.1") {
        errors.push("corpus.schema_version: expected 0.1".to_string());
    }
    if string_at(corpus, "corpus_id", "corpus", &mut errors) != Some("global-mobility-access-v0.1") {
        errors.push("corpus.corpus_id: unexpected identity".to_string());
    }
    if string_at(corpus, "claim_ceiling", "corpus", &mut errors)
        != Some("synthetic_archetypes_are_not_country_market_approval_or_global_availability_evidence")
    {
        errors.push("corpus.claim_ceiling: invalid or missing claim ceiling".to_string());
    }
    let provenance = &corpus["provenance"];
    const PROV_KEYS: &[&str] = &["created_on", "kind", "external_market_observations_included", "notes"];
    check_keys(provenance, "corpus.provenance", PROV_KEYS, PROV_KEYS, &mut errors);
    if string_at(provenance, "kind", "corpus.provenance", &mut errors) != Some("synthetic_regional_archetype_panel") {
        errors.push("corpus.provenance.kind: expected synthetic_regional_archetype_panel".to_string());
    }
    bool_at(provenance, "external_market_observations_included", false, "corpus.provenance", &mut errors);
    if string_at(provenance, "notes", "corpus.provenance", &mut errors).is_some_and(|s| s.len() < 20) {
        errors.push("corpus.provenance.notes: description too short".to_string());
    }
    if let Some(date) = string_at(provenance, "created_on", "corpus.provenance", &mut errors) {
        if date.len() != 10 || date.as_bytes().get(4) != Some(&b'-') || date.as_bytes().get(7) != Some(&b'-') {
            errors.push("corpus.provenance.created_on: expected YYYY-MM-DD".to_string());
        }
    }

    let Some(profiles) = corpus.get("profiles").and_then(Value::as_array) else {
        errors.push("corpus.profiles: expected array".to_string());
        return Err(errors);
    };
    if profiles.len() < 10 {
        errors.push(format!("corpus.profiles: expected at least 10 profiles, found {}", profiles.len()));
    }

    let mut ids = BTreeSet::new();
    let mut archetypes = BTreeSet::new();
    let mut negative_controls = 0;
    let mut requires_evidence = 0;
    for (index, profile) in profiles.iter().enumerate() {
        let (negative, id) = audit_profile(profile, index, &mut errors);
        if negative { negative_controls += 1; }
        if let Some(id) = id {
            if !ids.insert(id.clone()) {
                errors.push(format!("profiles[{index}].profile_id: duplicate ID {id}"));
            }
        }
        if let Some(archetype) = profile.get("archetype").and_then(Value::as_str) {
            archetypes.insert(archetype.to_string());
        }
        if profile.get("expected_disposition").and_then(Value::as_str) == Some("requires_market_specific_evidence") {
            requires_evidence += 1;
        }
    }
    if negative_controls == 0 {
        errors.push("corpus: at least one negative control is required".to_string());
    }
    if ids.len() != profiles.len() {
        errors.push("corpus: profile IDs must all be unique".to_string());
    }

    if errors.is_empty() {
        Ok(AuditSummary {
            profiles: profiles.len(),
            negative_controls,
            archetypes: archetypes.len(),
            market_specific_evidence_required: requires_evidence,
        })
    } else {
        Err(errors)
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "fixtures/global_mobility/global_mobility_profiles_v0_1.json".to_string());
    if args.next().is_some() {
        eprintln!("usage: global-mobility-profile-audit [path/to/corpus.json]");
        std::process::exit(2);
    }

    let source = if Path::new(&path).exists() {
        match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("FAIL: could not read {path}: {error}");
                std::process::exit(2);
            }
        }
    } else if path == "-" {
        EMBEDDED_CORPUS.to_string()
    } else {
        eprintln!("FAIL: file not found: {path}");
        std::process::exit(2);
    };
    let corpus: Value = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("FAIL: invalid JSON: {error}");
            std::process::exit(2);
        }
    };
    match audit_corpus(&corpus) {
        Ok(summary) => {
            println!("PASS: synthetic global mobility corpus contract");
            println!("profiles={}", summary.profiles);
            println!("negative_controls={}", summary.negative_controls);
            println!("unique_archetypes={}", summary.archetypes);
            println!("profiles_requiring_market_specific_evidence={}", summary.market_specific_evidence_required);
            println!("claim_ceiling=synthetic archetypes do not establish worldwide availability");
        }
        Err(errors) => {
            eprintln!("FAIL: {} finding(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_str(EMBEDDED_CORPUS).expect("committed corpus parses as JSON")
    }

    #[test]
    fn committed_corpus_passes_semantic_audit() {
        let result = audit_corpus(&corpus()).expect("synthetic profile corpus should pass");
        assert_eq!(result.profiles, 13);
        assert_eq!(result.negative_controls, 3);
        assert_eq!(result.archetypes, 13);
        assert_eq!(result.market_specific_evidence_required, 10);
    }

    #[test]
    fn evidence_backed_price_cannot_be_claimed_without_currency_and_source_records() {
        let mut value = corpus();
        value["profiles"][0]["affordability"]["price_status"] = Value::String("evidence_backed".to_string());
        assert!(audit_corpus(&value).is_err());
    }

    #[test]
    fn synthetic_archetype_cannot_claim_a_market_release_stage() {
        let mut value = corpus();
        value["profiles"][0]["market_stage"] = Value::String("market_release_evidenced".to_string());
        assert!(audit_corpus(&value).is_err());
    }

    #[test]
    fn missing_energy_or_cost_evidence_does_not_become_zero_or_available() {
        let mut value = corpus();
        value["profiles"][0]["infrastructure"]["charging_access"] = Value::String("evidence_backed".to_string());
        value["profiles"][0]["affordability"]["local_currency_code"] = Value::String("USD".to_string());
        assert!(audit_corpus(&value).is_err());
    }

    #[test]
    fn negative_control_must_block_global_availability_claim() {
        let mut value = corpus();
        value["profiles"][6]["expected_disposition"] = Value::String("requires_market_specific_evidence".to_string());
        assert!(audit_corpus(&value).is_err());
    }

    #[test]
    fn unknown_profile_fields_fail_closed() {
        let mut value = corpus();
        value["profiles"][0]["invented_global_approval"] = Value::Bool(true);
        assert!(audit_corpus(&value).is_err());
    }
}

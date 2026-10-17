use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const RECEIPT_VERSION: &str = "wis-sim-fixture-verification/0.1";
const CLAIM_CEILING: &str = "synthetic input contract only; this report is not a simulation result or evidence about human behaviour, consciousness, wisdom, spiritual practice, or real-world outcomes";
const FIXTURE: &str = include_str!("../../../docs/research/fixtures/wis-sim-scenarios-v0.1.jsonl");
const MANIFEST: &str = include_str!("../../../docs/research/fixtures/wis-sim-scenarios-v0.1.manifest.json");
const SCHEMA: &str = include_str!("../../../docs/research/fixtures/wis-sim-scenarios-v0.1.schema.json");

const EXPECTED_VARIANTS: [(&str, &str); 4] = [
    ("base", "baseline"),
    ("adversarial-pressure", "adversarial"),
    ("adversarial-low-information", "adversarial"),
    ("adversarial-role-reversal", "adversarial"),
];

pub fn verify_files(
    fixture_path: &str,
    manifest_path: &str,
    schema_path: &str,
) -> Result<Value, String> {
    let fixture = std::fs::read(fixture_path)
        .map_err(|error| format!("cannot read fixture {}: {}", fixture_path, error))?;
    let manifest = std::fs::read_to_string(manifest_path)
        .map_err(|error| format!("cannot read manifest {}: {}", manifest_path, error))?;
    let schema = std::fs::read_to_string(schema_path)
        .map_err(|error| format!("cannot read schema {}: {}", schema_path, error))?;
    verify_bytes(&fixture, &manifest, &schema)
}

pub fn verify_bytes(
    fixture_bytes: &[u8],
    manifest_text: &str,
    schema_text: &str,
) -> Result<Value, String> {
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|error| format!("manifest is not valid JSON: {}", error))?;
    if required_str(&manifest, "hash_algorithm", "manifest")? != "SHA-256" {
        return Err("manifest.hash_algorithm must be SHA-256".to_owned());
    }

    let actual_fixture_hash = sha256_hex(fixture_bytes);
    let expected_fixture_hash = required_str(&manifest, "fixture_sha256", "manifest")?;
    if actual_fixture_hash != expected_fixture_hash {
        return Err(format!(
            "fixture SHA-256 mismatch: manifest={}, actual={}",
            expected_fixture_hash, actual_fixture_hash
        ));
    }
    if !fixture_bytes.ends_with(b"\n") {
        return Err("fixture must be UTF-8 JSONL with a final newline".to_owned());
    }

    let actual_schema_hash = sha256_hex(schema_text.as_bytes());
    let expected_schema_hash = required_str(&manifest, "schema_sha256", "manifest")?;
    if actual_schema_hash != expected_schema_hash {
        return Err(format!(
            "schema SHA-256 mismatch: manifest={}, actual={}",
            expected_schema_hash, actual_schema_hash
        ));
    }
    let schema: Value = serde_json::from_str(schema_text)
        .map_err(|error| format!("schema is not valid JSON: {}", error))?;
    validate_schema_contract(&schema)?;

    let expected_count = manifest.get("record_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "manifest.record_count must be an unsigned integer".to_owned())? as usize;
    let expected_scenarios = manifest.get("unique_scenario_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "manifest.unique_scenario_count must be an unsigned integer".to_owned())? as usize;
    let declared_variant_count = manifest.get("variant_count_per_scenario")
        .and_then(Value::as_u64)
        .ok_or_else(|| "manifest.variant_count_per_scenario must be an unsigned integer".to_owned())? as usize;
    if declared_variant_count != EXPECTED_VARIANTS.len() {
        return Err("manifest.variant_count_per_scenario differs from the pinned 4-variant contract".to_owned());
    }
    let declared_families_array = manifest.get("scenario_families").and_then(Value::as_array)
        .ok_or_else(|| "manifest.scenario_families must be an array".to_owned())?;
    let mut declared_families: BTreeMap<String, (String, String)> = BTreeMap::new();
    for item in declared_families_array {
        let id = required_str(item, "scenario_id", "manifest.scenario_families")?.to_owned();
        let family = required_str(item, "family", "manifest.scenario_families")?.to_owned();
        let title = required_str(item, "title", "manifest.scenario_families")?.to_owned();
        if declared_families.insert(id.clone(), (family, title)).is_some() {
            return Err(format!("manifest repeats scenario_families id {}", id));
        }
    }
    if declared_families.len() != expected_scenarios {
        return Err("manifest scenario_families count differs from unique_scenario_count".to_owned());
    }
    if expected_count != 28 || expected_scenarios != 7 {
        return Err("this verifier version requires the frozen 28-record / 7-scenario contract".to_owned());
    }
    if required_str(&manifest, "execution_profile", "manifest")? != "symtropy-discrete-synthetic-v1" {
        return Err("unexpected execution profile in manifest".to_owned());
    }
    if required_str(&manifest, "baseline_policy", "manifest")? != "no_intervention_v0" {
        return Err("unexpected baseline policy in manifest".to_owned());
    }

    let manifest_policies = string_set(
        manifest.get("policy_comparison_set"),
        "manifest.policy_comparison_set",
    )?;
    if manifest_policies.len() < 2 || !manifest_policies.contains("no_intervention_v0") {
        return Err("manifest policy comparison set must include the baseline and at least one alternative".to_owned());
    }

    let mut scenarios: BTreeMap<String, ScenarioVariants> = BTreeMap::new();
    let mut seen_seeds = BTreeSet::new();
    let mut seen_case_variants = BTreeSet::new();
    let mut records = Vec::new();

    let lines: Vec<&[u8]> = fixture_bytes.split(|byte| *byte == b'\n').collect();
    for (index, raw) in lines.iter().enumerate() {
        if raw.is_empty() {
            if index == lines.len() - 1 {
                continue;
            }
            return Err(format!("blank JSONL line {}", index + 1));
        }
        let record: Value = serde_json::from_slice(raw)
            .map_err(|error| format!("line {} is not valid JSON: {}", index + 1, error))?;
        let where_ = format!("line {}", index + 1);
        validate_record_shape(&record, &schema, &where_)?;

        let scenario_id = required_str(&record, "scenario_id", &where_)?;
        if !valid_scenario_id(scenario_id) {
            return Err(format!("{} has invalid scenario_id {}", where_, scenario_id));
        }
        if required_str(&record, "scenario_version", &where_)? != "0.1" {
            return Err(format!("{} has unsupported scenario_version", where_));
        }
        let scenario_family = required_str(&record, "scenario_family", &where_)?.to_owned();
        let variant_id = required_str(&record, "variant_id", &where_)?;
        let variant_kind = required_str(&record, "variant_kind", &where_)?;
        let declared_kind = EXPECTED_VARIANTS.iter()
            .find(|(expected_id, _)| *expected_id == variant_id)
            .map(|(_, expected_kind)| *expected_kind)
            .ok_or_else(|| format!("{} uses undeclared variant_id {}", where_, variant_id))?;
        if variant_kind != declared_kind {
            return Err(format!(
                "{} variant {} must have variant_kind {}",
                where_, variant_id, declared_kind
            ));
        }

        if required_str(&record, "execution_profile", &where_)? != "symtropy-discrete-synthetic-v1" {
            return Err(format!("{} has unsupported execution_profile", where_));
        }
        if required_str(&record, "policy_version", &where_)? != "wis-policy-contract-v0.1" {
            return Err(format!("{} has unsupported policy_version", where_));
        }
        let policies = string_set(
            record.get("policies_under_comparison"),
            &format!("{}.policies_under_comparison", where_),
        )?;
        if policies != manifest_policies {
            return Err(format!("{} policy comparison set differs from manifest", where_));
        }
        let baseline_policy = required_str(&record, "baseline_policy", &where_)?;
        if baseline_policy != required_str(&manifest, "baseline_policy", "manifest")?
            || !policies.contains(baseline_policy)
        {
            return Err(format!("{} baseline policy is undeclared", where_));
        }

        let seed = record.get("deterministic_seed")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{}.deterministic_seed must be a non-negative integer", where_))?;
        if !seen_seeds.insert(seed) {
            return Err(format!("duplicate deterministic_seed {}", seed));
        }

        let variant_key = format!("{}::{}", scenario_id, variant_id);
        if !seen_case_variants.insert(variant_key.clone()) {
            return Err(format!("duplicate scenario/variant pair {}", variant_key));
        }

        let invariants = record.get("expected_invariants").and_then(Value::as_array)
            .ok_or_else(|| format!("{}.expected_invariants must be an array", where_))?;
        if invariants.is_empty() {
            return Err(format!("{}.expected_invariants cannot be empty", where_));
        }
        let mut invariant_ids = BTreeSet::new();
        for (invariant_index, invariant) in invariants.iter().enumerate() {
            let invariant_where = format!("{}.expected_invariants[{}]", where_, invariant_index);
            require_exact_keys(invariant, &["id", "statement"], &invariant_where)?;
            let id = required_str(invariant, "id", &invariant_where)?;
            let _ = required_str(invariant, "statement", &invariant_where)?;
            if !invariant_ids.insert(id.to_owned()) {
                return Err(format!("{} duplicates invariant id {}", where_, id));
            }
        }

        let observables = string_set(record.get("observables"), &format!("{}.observables", where_))?;
        if observables.is_empty() {
            return Err(format!("{}.observables cannot be empty", where_));
        }
        let initial = record.get("initial_conditions").and_then(Value::as_object)
            .ok_or_else(|| format!("{}.initial_conditions must be an object", where_))?;
        if initial.is_empty() {
            return Err(format!("{}.initial_conditions cannot be empty", where_));
        }
        let events = record.get("exogenous_events").and_then(Value::as_array)
            .ok_or_else(|| format!("{}.exogenous_events must be an array", where_))?;
        if events.iter().any(|event| !event.is_object()) {
            return Err(format!("{}.exogenous_events entries must be objects", where_));
        }
        let _ = required_str(&record, "known_model_limits", &where_)?;
        let claim_ceiling = required_str(&record, "claim_ceiling", &where_)?;
        if !claim_ceiling.contains("synthetic") {
            return Err(format!("{} claim ceiling must explicitly limit itself to synthetic evidence", where_));
        }

        let variant_map = scenarios.entry(scenario_id.to_owned()).or_default();
        if let Some(previous_family) = &variant_map.family {
            if previous_family != &scenario_family {
                return Err(format!("{} changes scenario_family across variants", scenario_id));
            }
        } else {
            variant_map.family = Some(scenario_family.clone());
        }
        if let Some(previous_title) = &variant_map.title {
            if previous_title != required_str(&record, "title", &where_)? {
                return Err(format!("{} changes title across variants", scenario_id));
            }
        } else {
            variant_map.title = Some(required_str(&record, "title", &where_)?.to_owned());
        }
        if variant_map.variants.insert(variant_id.to_owned(), variant_kind.to_owned()).is_some() {
            return Err(format!("{} repeats variant {}", scenario_id, variant_id));
        }

        records.push(json!({
            "scenario_id": scenario_id,
            "scenario_family": scenario_family,
            "variant_id": variant_id,
            "variant_kind": variant_kind,
            "deterministic_seed": seed,
            "invariant_count": invariants.len(),
            "observable_count": observables.len()
        }));
    }

    if records.len() != expected_count {
        return Err(format!(
            "record_count mismatch: manifest={}, parsed={}",
            expected_count, records.len()
        ));
    }
    if scenarios.len() != expected_scenarios {
        return Err(format!(
            "unique_scenario_count mismatch: manifest={}, parsed={}",
            expected_scenarios, scenarios.len()
        ));
    }

    let mut variant_distribution: BTreeMap<String, usize> = BTreeMap::new();
    for (scenario_id, scenario) in &scenarios {
        let observed: BTreeMap<&str, &str> = scenario.variants.iter()
            .map(|(id, kind)| (id.as_str(), kind.as_str()))
            .collect();
        if observed.len() != EXPECTED_VARIANTS.len() {
            return Err(format!(
                "{} must have exactly {} variants; observed {}",
                scenario_id, EXPECTED_VARIANTS.len(), observed.len()
            ));
        }
        for (expected_id, expected_kind) in EXPECTED_VARIANTS {
            if observed.get(expected_id).copied() != Some(expected_kind) {
                return Err(format!(
                    "{} missing or has wrong kind for variant {}",
                    scenario_id, expected_id
                ));
            }
            *variant_distribution.entry(expected_kind.to_owned()).or_default() += 1;
        }
    }

    for (scenario_id, scenario) in &scenarios {
        let declared = declared_families.get(scenario_id)
            .ok_or_else(|| format!("scenario {} is absent from manifest.scenario_families", scenario_id))?;
        if scenario.family.as_deref() != Some(declared.0.as_str()) {
            return Err(format!("scenario {} family differs from manifest", scenario_id));
        }
        if scenario.title.as_deref() != Some(declared.1.as_str()) {
            return Err(format!("scenario {} title differs from manifest", scenario_id));
        }
    }
    for scenario_id in declared_families.keys() {
        if !scenarios.contains_key(scenario_id) {
            return Err(format!("manifest scenario {} has no fixture records", scenario_id));
        }
    }

    let declared_distribution = manifest.get("variant_distribution")
        .and_then(Value::as_object)
        .ok_or_else(|| "manifest.variant_distribution must be an object".to_owned())?;
    let actual_distribution_keys: BTreeSet<String> = variant_distribution.keys().cloned().collect();
    let declared_distribution_keys: BTreeSet<String> = declared_distribution.keys().cloned().collect();
    if actual_distribution_keys != declared_distribution_keys {
        return Err("manifest.variant_distribution keys differ from observed variants".to_owned());
    }
    for (kind, actual) in &variant_distribution {
        let declared = declared_distribution.get(kind).and_then(Value::as_u64)
            .ok_or_else(|| format!("manifest.variant_distribution missing {}", kind))? as usize;
        if declared != *actual {
            return Err(format!("variant distribution mismatch for {}", kind));
        }
    }

    records.sort_by(|left, right| {
        left["scenario_id"].as_str().unwrap_or("").cmp(right["scenario_id"].as_str().unwrap_or(""))
            .then_with(|| left["variant_id"].as_str().unwrap_or("").cmp(right["variant_id"].as_str().unwrap_or("")))
    });
    let family_counts = scenarios.values().fold(BTreeMap::<String, usize>::new(), |mut counts, item| {
        if let Some(family) = &item.family {
            *counts.entry(family.clone()).or_default() += 1;
        }
        counts
    });

    Ok(json!({
        "receipt_version": RECEIPT_VERSION,
        "status": "STRUCTURAL_FIXTURE_MATCH_NOT_SIMULATION",
        "claim_ceiling": CLAIM_CEILING,
        "corpus_id": required_str(&manifest, "corpus_id", "manifest")?,
        "corpus_version": required_str(&manifest, "corpus_version", "manifest")?,
        "fixture_sha256": actual_fixture_hash,
        "schema_sha256": actual_schema_hash,
        "record_count": records.len(),
        "unique_scenario_count": scenarios.len(),
        "variant_distribution": variant_distribution,
        "unique_deterministic_seed_count": seen_seeds.len(),
        "family_counts": family_counts,
        "selected_top_level_schema_contract": "MATCH",
        "full_json_schema_conformance": "NOT_ASSERTED",
        "simulation_executed": false,
        "policy_outcomes": "NOT_ASSESSED",
        "human_outcomes": "NOT_ASSESSED",
        "records": records
    }))
}

#[derive(Default)]
struct ScenarioVariants {
    family: Option<String>,
    title: Option<String>,
    variants: BTreeMap<String, String>,
}

fn validate_schema_contract(schema: &Value) -> Result<(), String> {
    if required_str(schema, "$id", "schema")? != "wis-sim-scenarios-v0.1.schema.json" {
        return Err("unexpected schema $id".to_owned());
    }
    if required_str(schema, "type", "schema")? != "object" {
        return Err("scenario schema top-level type must be object".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        return Err("scenario schema must forbid additional top-level properties".to_owned());
    }
    let properties = schema.get("properties").and_then(Value::as_object)
        .ok_or_else(|| "schema.properties must be an object".to_owned())?;
    let required = string_set(schema.get("required"), "schema.required")?;
    let property_names: BTreeSet<String> = properties.keys().cloned().collect();
    let pinned_fields: BTreeSet<String> = [
        "scenario_id", "scenario_version", "scenario_family", "title", "variant_id",
        "variant_kind", "variant_axis", "variant_description", "deterministic_seed",
        "execution_profile", "policy_version", "policies_under_comparison", "baseline_policy",
        "initial_conditions", "exogenous_events", "expected_invariants", "observables",
        "known_model_limits", "claim_ceiling"
    ].into_iter().map(str::to_owned).collect();
    if property_names != pinned_fields || required != pinned_fields {
        return Err("schema field set differs from this verifier version".to_owned());
    }
    if required != property_names {
        return Err("schema.required and schema.properties must describe the same selected top-level contract".to_owned());
    }
    for required_property in &required {
        if !properties.contains_key(required_property) {
            return Err(format!("schema.required property {} has no schema", required_property));
        }
    }
    let variant_enum = properties.get("variant_kind")
        .and_then(|property| property.get("enum"))
        .and_then(Value::as_array)
        .ok_or_else(|| "schema.variant_kind.enum is missing".to_owned())?;
    let variants: BTreeSet<String> = variant_enum.iter()
        .map(|value| value.as_str().map(str::to_owned).ok_or_else(|| "variant_kind enum values must be strings".to_owned()))
        .collect::<Result<_, _>>()?;
    if variants != BTreeSet::from(["baseline".to_owned(), "adversarial".to_owned()]) {
        return Err("schema.variant_kind enum differs from the expected contract".to_owned());
    }
    Ok(())
}

fn validate_record_shape(record: &Value, schema: &Value, where_: &str) -> Result<(), String> {
    let object = record.as_object().ok_or_else(|| format!("{} must be an object", where_))?;
    let properties = schema.get("properties").and_then(Value::as_object)
        .ok_or_else(|| "schema.properties must be an object".to_owned())?;
    let required = string_set(schema.get("required"), "schema.required")?;
    for key in object.keys() {
        if !properties.contains_key(key) {
            return Err(format!("{} has undeclared top-level field {}", where_, key));
        }
    }
    for key in &required {
        if !object.contains_key(key) {
            return Err(format!("{} missing required field {}", where_, key));
        }
    }
    for (key, property_schema) in properties {
        let Some(value) = object.get(key) else { continue };
        if let Some(enum_values) = property_schema.get("enum").and_then(Value::as_array) {
            if !enum_values.iter().any(|candidate| candidate == value) {
                return Err(format!("{}.{} violates the declared enum", where_, key));
            }
        }
        if let Some(constant) = property_schema.get("const") {
            if value != constant {
                return Err(format!("{}.{} differs from declared const", where_, key));
            }
        }
        let kind = property_schema.get("type").and_then(Value::as_str);
        let matches_type = match kind {
            Some("string") => value.is_string(),
            Some("integer") => value.as_i64().is_some() || value.as_u64().is_some(),
            Some("number") => value.is_number(),
            Some("object") => value.is_object(),
            Some("array") => value.is_array(),
            Some(other) => return Err(format!("schema has unsupported selected type {} for {}", other, key)),
            None => true,
        };
        if !matches_type {
            return Err(format!("{}.{} has the wrong JSON type", where_, key));
        }
        if matches!(kind, Some("string")) && value.as_str().map(str::trim).unwrap_or_default().is_empty() {
            return Err(format!("{}.{} cannot be blank", where_, key));
        }
    }
    let scenario_id = required_str(record, "scenario_id", where_)?;
    if !valid_scenario_id(scenario_id) {
        return Err(format!("{}.scenario_id has invalid format", where_));
    }
    Ok(())
}

fn require_exact_keys(value: &Value, expected: &[&str], where_: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{} must be an object", where_))?;
    let actual: BTreeSet<String> = object.keys().cloned().collect();
    let expected: BTreeSet<String> = expected.iter().map(|key| (*key).to_owned()).collect();
    if actual != expected {
        return Err(format!("{} keys differ from the selected strict contract", where_));
    }
    Ok(())
}

fn string_set(value: Option<&Value>, where_: &str) -> Result<BTreeSet<String>, String> {
    let array = value.and_then(Value::as_array)
        .ok_or_else(|| format!("{} must be an array of strings", where_))?;
    let mut values = BTreeSet::new();
    for item in array {
        let value = item.as_str().ok_or_else(|| format!("{} entries must be strings", where_))?;
        if value.trim().is_empty() {
            return Err(format!("{} entries cannot be blank", where_));
        }
        if !values.insert(value.to_owned()) {
            return Err(format!("{} contains duplicate entry {}", where_, value));
        }
    }
    Ok(values)
}

fn required_str<'a>(value: &'a Value, key: &str, where_: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{}.{} must be a non-empty string", where_, key))
}

fn valid_scenario_id(value: &str) -> bool {
    value.len() == 7 && value.starts_with("WSS-") && value[4..].bytes().all(|byte| byte.is_ascii_digit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<_>>().join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> Value {
        verify_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA).expect("frozen fixture is structurally valid")
    }

    fn rewritten_fixture<F>(mutate: F) -> (Vec<u8>, String)
    where
        F: FnOnce(&mut Vec<Value>),
    {
        let mut records: Vec<Value> = FIXTURE.lines()
            .map(|line| serde_json::from_str(line).expect("fixture row is JSON"))
            .collect();
        mutate(&mut records);
        let text = records.iter()
            .map(|record| serde_json::to_string(record).expect("record serializes"))
            .collect::<Vec<_>>()
            .join("\n") + "\n";
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["fixture_sha256"] = json!(sha256_hex(text.as_bytes()));
        manifest["record_count"] = json!(records.len());
        (text.into_bytes(), serde_json::to_string(&manifest).unwrap())
    }

    #[test]
    fn golden_corpus_matches_selected_contract() {
        let report = good();
        assert_eq!(report["status"], "STRUCTURAL_FIXTURE_MATCH_NOT_SIMULATION");
        assert_eq!(report["record_count"], 28);
        assert_eq!(report["unique_scenario_count"], 7);
        assert_eq!(report["unique_deterministic_seed_count"], 28);
        assert_eq!(report["variant_distribution"]["baseline"], 7);
        assert_eq!(report["variant_distribution"]["adversarial"], 21);
        assert_eq!(report["simulation_executed"], false);
        assert_eq!(report["full_json_schema_conformance"], "NOT_ASSERTED");
    }

    #[test]
    fn changed_fixture_bytes_fail_hash_first() {
        let mut bytes = FIXTURE.as_bytes().to_vec();
        bytes[0] ^= 1;
        assert!(verify_bytes(&bytes, MANIFEST, SCHEMA).unwrap_err().contains("fixture SHA-256 mismatch"));
    }

    #[test]
    fn changed_schema_bytes_fail_hash_first() {
        let changed = SCHEMA.replacen("deterministic synthetic scenario", "changed synthetic scenario", 1);
        assert!(verify_bytes(FIXTURE.as_bytes(), MANIFEST, &changed).unwrap_err().contains("schema SHA-256 mismatch"));
    }

    #[test]
    fn fixture_requires_final_newline() {
        assert!(verify_bytes(FIXTURE.trim_end().as_bytes(), MANIFEST, SCHEMA).unwrap_err().contains("final newline"));
    }

    #[test]
    fn blank_jsonl_line_fails_closed() {
        let bytes = FIXTURE.replace("\n", "\n\n").into_bytes();
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["fixture_sha256"] = json!(sha256_hex(&bytes));
        let text = serde_json::to_string(&manifest).unwrap();
        assert!(verify_bytes(&bytes, &text, SCHEMA).unwrap_err().contains("blank JSONL line"));
    }

    #[test]
    fn duplicate_scenario_variant_pair_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            rows[1]["variant_id"] = json!("base");
            rows[1]["variant_kind"] = json!("baseline");
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("duplicate scenario/variant pair"));
    }

    #[test]
    fn duplicate_seed_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            let first_seed = rows[0]["deterministic_seed"].clone();
            rows[1]["deterministic_seed"] = first_seed;
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("duplicate deterministic_seed"));
    }

    #[test]
    fn undeclared_variant_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| rows[1]["variant_id"] = json!("maybe"));
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("undeclared variant_id maybe"));
    }

    #[test]
    fn missing_required_top_level_field_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| { rows[0].as_object_mut().unwrap().remove("claim_ceiling"); });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("missing required field claim_ceiling"));
    }

    #[test]
    fn undeclared_top_level_field_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| rows[0]["opaque_wisdom_score"] = json!(0.99));
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("undeclared top-level field"));
    }

    #[test]
    fn empty_invariant_list_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| rows[0]["expected_invariants"] = json!([]));
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("expected_invariants cannot be empty"));
    }

    #[test]
    fn duplicate_invariant_id_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            let first_id = rows[0]["expected_invariants"][0]["id"].clone();
            rows[0]["expected_invariants"][1]["id"] = first_id;
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("duplicates invariant id"));
    }

    #[test]
    fn empty_observables_fail_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| rows[0]["observables"] = json!([]));
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("observables cannot be empty"));
    }

    #[test]
    fn missing_variant_fails_family_contract() {
        let (bytes, manifest) = rewritten_fixture(|rows| { rows.pop(); });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("requires the frozen 28-record"));
    }

    #[test]
    fn policy_set_must_match_the_pinned_comparison_contract() {
        let (bytes, manifest) = rewritten_fixture(|rows| rows[0]["policies_under_comparison"] = json!(["no_intervention_v0", "invented_policy"]));
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("policy comparison set differs from manifest"));
    }

    #[test]
    fn baseline_must_be_in_policy_set() {
        let (bytes, manifest) = rewritten_fixture(|rows| rows[0]["baseline_policy"] = json!("undeclared_baseline"));
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("baseline policy is undeclared"));
    }

    #[test]
    fn schema_must_forbid_additional_top_level_properties() {
        let changed_schema = SCHEMA.replace("\"additionalProperties\": false", "\"additionalProperties\": true");
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["schema_sha256"] = json!(sha256_hex(changed_schema.as_bytes()));
        let manifest_text = serde_json::to_string(&manifest).unwrap();
        assert!(verify_bytes(FIXTURE.as_bytes(), &manifest_text, &changed_schema).unwrap_err().contains("must forbid additional top-level properties"));
    }


    #[test]
    fn manifest_variant_count_must_match_pinned_contract() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["variant_count_per_scenario"] = json!(3);
        let text = serde_json::to_string(&manifest).unwrap();
        assert!(verify_bytes(FIXTURE.as_bytes(), &text, SCHEMA).unwrap_err().contains("variant_count_per_scenario differs"));
    }

    #[test]
    fn manifest_distribution_rejects_undeclared_variant_keys() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["variant_distribution"]["other"] = json!(1);
        let text = serde_json::to_string(&manifest).unwrap();
        assert!(verify_bytes(FIXTURE.as_bytes(), &text, SCHEMA).unwrap_err().contains("variant_distribution keys differ"));
    }

    #[test]
    fn scenario_family_and_title_are_bound_to_manifest() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            for row in rows.iter_mut().filter(|row| row["scenario_id"] == "WSS-001") {
                row["scenario_family"] = json!("invented_family");
            }
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("family differs from manifest"));
    }

    #[test]
    fn schema_drift_fails_even_if_its_manifest_digest_is_updated() {
        let mut schema: Value = serde_json::from_str(SCHEMA).unwrap();
        schema["properties"]["opaque_wisdom_score"] = json!({"type":"number"});
        schema["required"].as_array_mut().unwrap().push(json!("opaque_wisdom_score"));
        let text_schema = serde_json::to_string_pretty(&schema).unwrap() + "\n";
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["schema_sha256"] = json!(sha256_hex(text_schema.as_bytes()));
        let text_manifest = serde_json::to_string(&manifest).unwrap();
        assert!(verify_bytes(FIXTURE.as_bytes(), &text_manifest, &text_schema).unwrap_err().contains("schema field set differs"));
    }

    #[test]
    fn identical_inputs_produce_identical_receipt_bytes() {
        let first = serde_json::to_vec(&good()).unwrap();
        let second = serde_json::to_vec(&good()).unwrap();
        assert_eq!(first, second);
    }
}

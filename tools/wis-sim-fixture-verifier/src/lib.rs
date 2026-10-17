use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const RECEIPT_VERSION: &str = "wis-sim-fixture-verification/0.1";
const CLAIM_CEILING: &str = "synthetic input contract only; this report is not a simulation result or evidence about human behaviour, consciousness, wisdom, spiritual practice, or real-world outcomes";
const FIXTURE: &str = include_str!("../../../docs/research/fixtures/wis-sim-scenarios-v0.1.jsonl");
const MANIFEST: &str = include_str!("../../../docs/research/fixtures/wis-sim-scenarios-v0.1.manifest.json");
const SCHEMA: &str = include_str!("../../../docs/research/fixtures/wis-sim-scenarios-v0.1.schema.json");
const RUN_REPORT_SCHEMA: &str = include_str!("../../../docs/research/fixtures/wis-sim-run-report-v0.1.schema.json");

const EXPECTED_VARIANTS: [(&str, &str); 4] = [
    ("base", "baseline"),
    ("adversarial-pressure", "adversarial"),
    ("adversarial-low-information", "adversarial"),
    ("adversarial-role-or-value-reversal", "adversarial"),
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

    let actual_run_report_schema_hash = sha256_hex(RUN_REPORT_SCHEMA.as_bytes());
    let expected_run_report_schema_hash = required_str(&manifest, "run_report_schema_sha256", "manifest")?;
    if actual_run_report_schema_hash != expected_run_report_schema_hash {
        return Err(format!(
            "run-report schema SHA-256 mismatch: manifest={}, actual={}",
            expected_run_report_schema_hash, actual_run_report_schema_hash
        ));
    }
    let run_report_schema: Value = serde_json::from_str(RUN_REPORT_SCHEMA)
        .map_err(|error| format!("embedded run-report schema is not valid JSON: {}", error))?;
    validate_run_report_schema_contract(&run_report_schema)?;

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

        let mut assertion_ids: Vec<String> = record["assertion_specifications"].as_array().unwrap()
            .iter()
            .map(|assertion| assertion.get("assertion_id").and_then(Value::as_str).unwrap().to_owned())
            .collect();
        assertion_ids.sort();
        let variant_map = scenarios.entry(scenario_id.to_owned()).or_default();
        if let Some(previous_ids) = &variant_map.assertion_ids {
            if previous_ids != &assertion_ids {
                return Err(format!("{} changes assertion contract across variants", scenario_id));
            }
        } else {
            variant_map.assertion_ids = Some(assertion_ids);
        }
        if let Some(previous_family) = &variant_map.family {
            if previous_family != &scenario_family {
                return Err(format!("{} changes scenario_family across variants", scenario_id));
            }
        } else {
            variant_map.family = Some(scenario_family.clone());
        }
        if let Some(previous_title) = &variant_map.title {
            if previous_title.as_str() != required_str(&record, "title", &where_)? {
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


pub fn verify_run_report_files(
    fixture_path: &str,
    manifest_path: &str,
    scenario_schema_path: &str,
    report_path: &str,
) -> Result<Value, String> {
    let fixture = std::fs::read(fixture_path)
        .map_err(|error| format!("cannot read fixture {}: {}", fixture_path, error))?;
    let manifest = std::fs::read_to_string(manifest_path)
        .map_err(|error| format!("cannot read manifest {}: {}", manifest_path, error))?;
    let scenario_schema = std::fs::read_to_string(scenario_schema_path)
        .map_err(|error| format!("cannot read scenario schema {}: {}", scenario_schema_path, error))?;
    let report = std::fs::read_to_string(report_path)
        .map_err(|error| format!("cannot read run report {}: {}", report_path, error))?;
    verify_run_report_bytes(&fixture, &manifest, &scenario_schema, &report)
}

/// Independently checks one runner-produced report against the exact scenario's typed assertions.
/// A valid structural receipt does not make a failed invariant into a pass and does not validate
/// the simulator's model assumptions or any human outcome.
pub fn verify_run_report_bytes(
    fixture_bytes: &[u8],
    manifest_text: &str,
    scenario_schema_text: &str,
    report_text: &str,
) -> Result<Value, String> {
    let _corpus_receipt = verify_bytes(fixture_bytes, manifest_text, scenario_schema_text)?;
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|error| format!("manifest is not valid JSON: {}", error))?;
    let report: Value = serde_json::from_str(report_text)
        .map_err(|error| format!("run report is not valid JSON: {}", error))?;
    let report_schema: Value = serde_json::from_str(RUN_REPORT_SCHEMA)
        .map_err(|error| format!("embedded run-report schema is malformed: {}", error))?;
    validate_run_report_shape(&report, &report_schema)?;

    let identity = report.get("identity")
        .ok_or_else(|| "run report identity is missing".to_owned())?;
    let fixture_hash = required_str(&manifest, "fixture_sha256", "manifest")?;
    let scenario_schema_hash = required_str(&manifest, "schema_sha256", "manifest")?;
    for (field, expected) in [
        ("corpus_id", required_str(&manifest, "corpus_id", "manifest")?),
        ("corpus_version", required_str(&manifest, "corpus_version", "manifest")?),
        ("fixture_sha256", fixture_hash),
        ("schema_sha256", scenario_schema_hash),
    ] {
        if required_str(identity, field, "run report identity")? != expected {
            return Err(format!("run report identity.{} does not match the pinned manifest", field));
        }
    }

    let scenario_id = required_str(identity, "scenario_id", "run report identity")?;
    let variant_id = required_str(identity, "variant_id", "run report identity")?;
    let scenario_version = required_str(identity, "scenario_version", "run report identity")?;
    let mut matched_record: Option<Value> = None;
    let mut matched_line: Option<Vec<u8>> = None;
    for raw in fixture_bytes.split(|byte| *byte == b'\n') {
        if raw.is_empty() { continue; }
        let candidate: Value = serde_json::from_slice(raw)
            .map_err(|error| format!("fixture unexpectedly became invalid JSON: {}", error))?;
        if candidate.get("scenario_id").and_then(Value::as_str) == Some(scenario_id)
            && candidate.get("variant_id").and_then(Value::as_str) == Some(variant_id) {
            matched_record = Some(candidate);
            matched_line = Some(raw.to_vec());
            break;
        }
    }
    let scenario = matched_record.ok_or_else(|| {
        format!("run report scenario/variant {}/{} is not in the pinned corpus", scenario_id, variant_id)
    })?;
    let raw_line = matched_line.expect("scenario record and raw line are set together");
    if required_str(&scenario, "scenario_version", "scenario")? != scenario_version {
        return Err("run report scenario_version differs from pinned scenario".to_owned());
    }

    let policy_id = required_str(identity, "policy_id", "run report identity")?;
    let allowed_policies = string_set(
        scenario.get("policies_under_comparison"),
        "scenario.policies_under_comparison",
    )?;
    if !allowed_policies.contains(policy_id) {
        return Err(format!("run report policy_id {} is not declared for {}", policy_id, scenario_id));
    }
    for (field, scenario_field) in [
        ("policy_version", "policy_version"),
        ("execution_profile", "execution_profile"),
    ] {
        if required_str(identity, field, "run report identity")?
            != required_str(&scenario, scenario_field, "scenario")? {
            return Err(format!("run report identity.{} differs from the pinned scenario", field));
        }
    }
    if identity.get("deterministic_seed").and_then(Value::as_u64)
        != scenario.get("deterministic_seed").and_then(Value::as_u64) {
        return Err("run report deterministic_seed differs from pinned scenario".to_owned());
    }
    let input_hash = required_str(identity, "input_sha256", "run report identity")?;
    let actual_input_hash = sha256_hex(&raw_line);
    if input_hash != actual_input_hash {
        return Err(format!(
            "run report input_sha256 mismatch: declared={}, actual={}",
            input_hash, actual_input_hash
        ));
    }
    for field in ["engine_revision", "toolchain_profile", "target_triple"] {
        let _ = required_str(identity, field, "run report identity")?;
    }
    for field in ["environment_fingerprint", "input_sha256"] {
        let value = required_str(identity, field, "run report identity")?;
        if !valid_sha256(value) {
            return Err(format!("run report identity.{} must be a lowercase SHA-256 digest", field));
        }
    }
    let claim_ceiling = required_str(&report, "claim_ceiling", "run report")?;
    if !claim_ceiling.to_ascii_lowercase().contains("synthetic") {
        return Err("run report claim_ceiling must explicitly remain synthetic".to_owned());
    }

    let run_status = required_str(&report, "run_status", "run report")?;
    let simulation_executed = report.get("simulation_executed").and_then(Value::as_bool).unwrap_or(false);
    if run_status != "completed" || !simulation_executed {
        return Ok(json!({
            "receipt_version": "wis-sim-run-report-verification/0.1",
            "status": "RUN_NOT_QUALIFIED",
            "claim_ceiling": "synthetic run receipt only; the simulation did not complete qualification",
            "report_sha256": sha256_hex(report_text.as_bytes()),
            "fixture_sha256": fixture_hash,
            "scenario_schema_sha256": scenario_schema_hash,
            "scenario_id": scenario_id,
            "variant_id": variant_id,
            "policy_id": policy_id,
            "run_status": run_status,
            "simulation_executed": simulation_executed,
            "assertions_evaluated": false,
            "reason": "run_status must be completed and simulation_executed must be true before invariant qualification"
        }));
    }

    let specs = scenario.get("assertion_specifications").and_then(Value::as_array)
        .ok_or_else(|| format!("{}.assertion_specifications is missing", scenario_id))?;
    let declared_results = report.get("invariant_results").and_then(Value::as_array)
        .ok_or_else(|| "run report invariant_results must be an array".to_owned())?;
    let mut declared_by_id = BTreeMap::new();
    for item in declared_results {
        let id = required_str(item, "assertion_id", "run report invariant result")?;
        if declared_by_id.insert(id.to_owned(), item).is_some() {
            return Err(format!("run report duplicates invariant result {}", id));
        }
    }
    let expected_ids: BTreeSet<String> = specs.iter()
        .map(|item| required_str(item, "assertion_id", "assertion specification").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    let declared_ids: BTreeSet<String> = declared_by_id.keys().cloned().collect();
    if expected_ids != declared_ids {
        return Err(format!(
            "run report invariant-result IDs do not exactly match scenario assertions: expected={:?}, actual={:?}",
            expected_ids, declared_ids
        ));
    }

    let mut evaluated = Vec::new();
    let mut hard_failed = 0usize;
    let mut hard_unknown = 0usize;
    let mut diagnostic_failed = 0usize;
    for spec in specs {
        let result = evaluate_assertion(spec, &scenario, &report)?;
        let id = required_str(spec, "assertion_id", "assertion specification")?;
        let declared = declared_by_id.get(id)
            .ok_or_else(|| format!("run report omitted invariant result {}", id))?;
        let declared_status = required_str(declared, "status", "run report invariant result")?;
        let declared_pointer = required_str(declared, "evidence_pointer", "run report invariant result")?;
        let expected_pointer = required_str(spec, "report_pointer", "assertion specification")?;
        if declared_status != result["status"].as_str().unwrap_or("unknown") {
            return Err(format!(
                "runner invariant status for {} disagrees with independent evaluation: runner={}, evaluator={}",
                id, declared_status, result["status"].as_str().unwrap_or("unknown")
            ));
        }
        if declared_pointer != expected_pointer {
            return Err(format!("runner evidence pointer for {} differs from the assertion contract", id));
        }
        match (required_str(spec, "severity", "assertion specification")?, result["status"].as_str().unwrap_or("unknown")) {
            ("hard", "fail") => hard_failed += 1,
            ("hard", "unknown") => hard_unknown += 1,
            ("diagnostic", "fail") => diagnostic_failed += 1,
            _ => {}
        }
        evaluated.push(result);
    }

    let status = if hard_failed > 0 {
        "RUN_REPORT_ASSERTIONS_FAIL"
    } else if hard_unknown > 0 {
        "RUN_REPORT_ASSERTIONS_UNKNOWN"
    } else {
        "RUN_REPORT_ASSERTIONS_PASS"
    };
    Ok(json!({
        "receipt_version": "wis-sim-run-report-verification/0.1",
        "status": status,
        "claim_ceiling": "synthetic policy run only; does not validate model assumptions or human outcomes",
        "report_sha256": sha256_hex(report_text.as_bytes()),
        "fixture_sha256": fixture_hash,
        "scenario_schema_sha256": scenario_schema_hash,
        "run_report_schema_sha256": sha256_hex(RUN_REPORT_SCHEMA.as_bytes()),
        "scenario_id": scenario_id,
        "variant_id": variant_id,
        "policy_id": policy_id,
        "run_status": run_status,
        "simulation_executed": true,
        "assertion_count": evaluated.len(),
        "hard_assertions_failed": hard_failed,
        "hard_assertions_unknown": hard_unknown,
        "diagnostic_assertions_failed": diagnostic_failed,
        "assertion_results": evaluated
    }))
}

fn validate_run_report_schema_contract(schema: &Value) -> Result<(), String> {
    if required_str(schema, "$id", "run-report schema")? != "wis-sim-run-report-v0.1.schema.json" {
        return Err("unexpected run-report schema $id".to_owned());
    }
    if required_str(schema, "type", "run-report schema")? != "object"
        || schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        return Err("run-report schema must be a closed top-level object".to_owned());
    }
    let top_properties = schema.get("properties").and_then(Value::as_object)
        .ok_or_else(|| "run-report schema.properties must be an object".to_owned())?;
    let top_required = string_set(schema.get("required"), "run-report schema.required")?;
    let top_names: BTreeSet<String> = top_properties.keys().cloned().collect();
    if top_names != top_required {
        return Err("run-report schema must require every declared top-level field".to_owned());
    }
    let identity = top_properties.get("identity")
        .ok_or_else(|| "run-report schema identity property missing".to_owned())?;
    let identity_properties = identity.get("properties").and_then(Value::as_object)
        .ok_or_else(|| "run-report schema identity.properties missing".to_owned())?;
    let identity_required = string_set(identity.get("required"), "run-report schema identity.required")?;
    let identity_names: BTreeSet<String> = identity_properties.keys().cloned().collect();
    if identity_names != identity_required {
        return Err("run-report identity schema must require every declared identity field".to_owned());
    }
    if identity.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        return Err("run-report identity must be a closed object".to_owned());
    }
    Ok(())
}

fn validate_run_report_shape(report: &Value, schema: &Value) -> Result<(), String> {
    let object = report.as_object().ok_or_else(|| "run report must be an object".to_owned())?;
    let properties = schema.get("properties").and_then(Value::as_object)
        .ok_or_else(|| "run-report schema properties missing".to_owned())?;
    let required = string_set(schema.get("required"), "run-report schema.required")?;
    for key in object.keys() {
        if !properties.contains_key(key) {
            return Err(format!("run report has undeclared top-level field {}", key));
        }
    }
    for key in &required {
        if !object.contains_key(key) {
            return Err(format!("run report missing required top-level field {}", key));
        }
    }
    if report.get("report_version") != Some(&json!("wis-sim-run-report/0.1")) {
        return Err("run report has unsupported report_version".to_owned());
    }
    let status = required_str(report, "run_status", "run report")?;
    if !["completed", "failed", "unsupported", "not_run"].contains(&status) {
        return Err(format!("run report has unsupported run_status {}", status));
    }
    for (key, kind) in [
        ("simulation_executed", "boolean"),
        ("metrics", "object"),
        ("claims_retained", "array"),
        ("claims", "object"),
        ("decision", "object"),
        ("objections", "object"),
        ("events", "object"),
        ("invariant_results", "array"),
        ("warnings", "array"),
        ("identity", "object"),
    ] {
        let value = report.get(key).ok_or_else(|| format!("run report missing {}", key))?;
        let valid = match kind {
            "boolean" => value.is_boolean(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            _ => false,
        };
        if !valid {
            return Err(format!("run report.{} must be a {}", key, kind));
        }
    }
    for warning in report["warnings"].as_array().unwrap() {
        if !warning.is_string() {
            return Err("run report warnings entries must be strings".to_owned());
        }
    }
    for claim in report["claims_retained"].as_array().unwrap() {
        let _ = required_str(claim, "id", "run report claims_retained item")?;
    }

    let identity = report.get("identity").unwrap();
    let identity_schema = properties.get("identity").unwrap();
    let identity_properties = identity_schema.get("properties").and_then(Value::as_object).unwrap();
    let identity_required = string_set(identity_schema.get("required"), "run-report identity.required")?;
    let identity_object = identity.as_object().unwrap();
    for key in identity_object.keys() {
        if !identity_properties.contains_key(key) {
            return Err(format!("run report identity has undeclared field {}", key));
        }
    }
    for key in &identity_required {
        if !identity_object.contains_key(key) {
            return Err(format!("run report identity missing field {}", key));
        }
    }
    for key in [
        "corpus_id", "corpus_version", "fixture_sha256", "scenario_id", "scenario_version",
        "variant_id", "policy_id", "policy_version", "execution_profile",
        "engine_revision", "toolchain_profile", "target_triple", "environment_fingerprint", "input_sha256",
    ] {
        let _ = required_str(identity, key, "run report identity")?;
    }
    if identity.get("deterministic_seed").and_then(Value::as_u64).is_none() {
        return Err("run report identity.deterministic_seed must be non-negative integer".to_owned());
    }
    let mut invariant_ids = BTreeSet::new();
    for item in report["invariant_results"].as_array().unwrap() {
        let fields = item.as_object().ok_or_else(|| "run report invariant result must be an object".to_owned())?;
        for key in fields.keys() {
            if !["assertion_id", "status", "observed", "expected", "evidence_pointer", "explanation"].contains(&key.as_str()) {
                return Err(format!("run report invariant result has undeclared field {}", key));
            }
        }
        for key in ["assertion_id", "status", "evidence_pointer", "explanation"] {
            if !fields.contains_key(key) {
                return Err(format!("run report invariant result missing {}", key));
            }
        }
        let id = required_str(item, "assertion_id", "run report invariant result")?;
        if !invariant_ids.insert(id.to_owned()) {
            return Err(format!("run report duplicates invariant result {}", id));
        }
        let result_status = required_str(item, "status", "run report invariant result")?;
        if !["pass", "fail", "unknown", "not_applicable"].contains(&result_status) {
            return Err(format!("run report invariant result {} has invalid status", id));
        }
        let pointer = required_str(item, "evidence_pointer", "run report invariant result")?;
        if !valid_json_pointer(pointer) {
            return Err(format!("run report invariant result {} has invalid evidence_pointer", id));
        }
        let _ = required_str(item, "explanation", "run report invariant result")?;
    }
    Ok(())
}

fn evaluate_assertion(spec: &Value, scenario: &Value, report: &Value) -> Result<Value, String> {
    let id = required_str(spec, "assertion_id", "assertion specification")?;
    let kind = required_str(spec, "assertion_kind", "assertion specification")?;
    let report_pointer = required_str(spec, "report_pointer", "assertion specification")?;
    let actual = resolve_json_pointer(report, report_pointer);
    let mut expected_receipt = Value::Null;
    let mut observed_receipt = actual.cloned().unwrap_or(Value::Null);
    let status = match kind {
        "numeric_upper_bound" => {
            let scenario_pointer = required_str(spec, "scenario_pointer", "assertion specification")?;
            let bound = resolve_json_pointer(scenario, scenario_pointer)
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("{} scenario bound is missing or non-numeric", id))?;
            expected_receipt = json!({"less_than_or_equal": bound});
            match actual.and_then(Value::as_f64) {
                Some(value) if value <= bound => "pass",
                _ => "fail",
            }
        }
        "collection_contains_all_ids" => {
            let scenario_pointer = required_str(spec, "scenario_pointer", "assertion specification")?;
            let identity_field = required_str(spec, "identity_field", "assertion specification")?;
            let expected = resolve_json_pointer(scenario, scenario_pointer)
                .ok_or_else(|| format!("{} scenario collection is missing", id))?;
            let expected_items = expected.as_array().ok_or_else(|| format!("{} scenario collection must be an array", id))?;
            let observed_items = actual.and_then(Value::as_array);
            let mut expected_ids = BTreeSet::new();
            for item in expected_items {
                let item_id = required_str(item, identity_field, &format!("{}.scenario", id))?;
                if !expected_ids.insert(item_id.to_owned()) {
                    return Err(format!("{} scenario identity collection contains duplicate {}", id, item_id));
                }
            }
            let mut observed_ids = BTreeSet::new();
            let mut observed_valid = observed_items.is_some();
            if let Some(items) = observed_items {
                for item in items {
                    let item_id = item.get(identity_field).and_then(Value::as_str);
                    match item_id {
                        Some(value) => {
                            if !observed_ids.insert(value.to_owned()) {
                                observed_valid = false;
                            }
                        }
                        None => observed_valid = false,
                    }
                }
            }
            expected_receipt = json!({"required_ids": expected_ids});
            observed_receipt = json!({"observed_ids": observed_ids, "collection_valid": observed_valid});
            if observed_valid && expected_ids.is_subset(&observed_ids) { "pass" } else { "fail" }
        }
        "field_present" => {
            match actual {
                Some(Value::Null) | None => "fail",
                Some(Value::String(value)) if value.trim().is_empty() => "fail",
                Some(_) => "pass",
            }
        }
        "metric_zero" => {
            expected_receipt = json!(0);
            if actual.and_then(Value::as_f64) == Some(0.0) { "pass" } else { "fail" }
        }
        "value_equals" => {
            let pointer = required_str(spec, "scenario_pointer", "assertion specification")?;
            let expected = resolve_json_pointer(scenario, pointer)
                .ok_or_else(|| format!("{} scenario value is missing", id))?;
            expected_receipt = expected.clone();
            if actual == Some(expected) { "pass" } else { "fail" }
        }
        "boolean_true" => {
            expected_receipt = json!(true);
            if actual.and_then(Value::as_bool) == Some(true) { "pass" } else { "fail" }
        }
        "field_absent" => {
            expected_receipt = json!({"present": false});
            observed_receipt = json!({"present": actual.is_some()});
            if actual.is_none() { "pass" } else { "fail" }
        }
        _ => return Err(format!("{} has unsupported assertion kind {}", id, kind)),
    };
    Ok(json!({
        "assertion_id": id,
        "assertion_kind": kind,
        "severity": required_str(spec, "severity", "assertion specification")?,
        "status": status,
        "evidence_pointer": report_pointer,
        "expected": expected_receipt,
        "observed": observed_receipt,
        "explanation": required_str(spec, "rationale", "assertion specification")?
    }))
}

fn resolve_json_pointer<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(root);
    }
    if !pointer.starts_with('/') {
        return None;
    }
    let mut current = root;
    for segment in pointer[1..].split('/') {
        let mut decoded = String::with_capacity(segment.len());
        let mut chars = segment.chars();
        while let Some(ch) = chars.next() {
            if ch == '~' {
                match chars.next()? {
                    '0' => decoded.push('~'),
                    '1' => decoded.push('/'),
                    _ => return None,
                }
            } else {
                decoded.push(ch);
            }
        }
        current = match current {
            Value::Object(map) => map.get(&decoded)?,
            Value::Array(items) => {
                if decoded.is_empty() || (decoded.len() > 1 && decoded.starts_with('0')) {
                    return None;
                }
                let index = decoded.parse::<usize>().ok()?;
                items.get(index)?
            }
            _ => return None,
        };
    }
    Some(current)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[derive(Default)]
struct ScenarioVariants {
    family: Option<String>,
    title: Option<String>,
    assertion_ids: Option<Vec<String>>,
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
        "assertion_specifications", "known_model_limits", "claim_ceiling"
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
    validate_assertion_specifications(record, where_)?;
    let scenario_id = required_str(record, "scenario_id", where_)?;
    if !valid_scenario_id(scenario_id) {
        return Err(format!("{}.scenario_id has invalid format", where_));
    }
    Ok(())
}


fn validate_assertion_specifications(record: &Value, where_: &str) -> Result<(), String> {
    let items = record.get("assertion_specifications").and_then(Value::as_array)
        .ok_or_else(|| format!("{}.assertion_specifications must be an array", where_))?;
    if items.len() < 3 {
        return Err(format!("{}.assertion_specifications must declare at least three executable assertions", where_));
    }
    let allowed_kinds: BTreeSet<&str> = [
        "numeric_upper_bound",
        "collection_contains_all_ids",
        "field_present",
        "metric_zero",
        "value_equals",
        "boolean_true",
        "field_absent",
    ].into_iter().collect();
    let allowed_scopes: BTreeSet<&str> = [
        "each_policy", "policy_matrix", "per_scenario", "paired_replay"
    ].into_iter().collect();
    let allowed_severities: BTreeSet<&str> = ["hard", "diagnostic"].into_iter().collect();
    let mut seen_ids = BTreeSet::new();

    for (index, item) in items.iter().enumerate() {
        let context = format!("{}.assertion_specifications[{}]", where_, index);
        let object = item.as_object().ok_or_else(|| format!("{} must be an object", context))?;
        let allowed_fields: BTreeSet<&str> = [
            "assertion_id", "assertion_kind", "scope", "severity",
            "report_pointer", "scenario_pointer", "identity_field", "rationale"
        ].into_iter().collect();
        for key in object.keys() {
            if !allowed_fields.contains(key.as_str()) {
                return Err(format!("{} has undeclared field {}", context, key));
            }
        }
        for key in ["assertion_id", "assertion_kind", "scope", "severity", "report_pointer", "rationale"] {
            if !object.contains_key(key) {
                return Err(format!("{} missing required field {}", context, key));
            }
        }
        let id = required_str(item, "assertion_id", &context)?;
        if !seen_ids.insert(id.to_owned()) {
            return Err(format!("{} duplicates assertion_id {}", context, id));
        }
        let kind = required_str(item, "assertion_kind", &context)?;
        if !allowed_kinds.contains(kind) {
            return Err(format!("{} has undeclared assertion_kind {}", context, kind));
        }
        let scope = required_str(item, "scope", &context)?;
        if !allowed_scopes.contains(scope) {
            return Err(format!("{} has undeclared assertion scope {}", context, scope));
        }
        let severity = required_str(item, "severity", &context)?;
        if !allowed_severities.contains(severity) {
            return Err(format!("{} has undeclared assertion severity {}", context, severity));
        }
        let report_pointer = required_str(item, "report_pointer", &context)?;
        if !valid_json_pointer(report_pointer) {
            return Err(format!("{} report_pointer is not a valid absolute JSON Pointer", context));
        }
        let _rationale = required_str(item, "rationale", &context)?;
        let scenario_pointer = item.get("scenario_pointer").and_then(Value::as_str);
        let identity_field = item.get("identity_field").and_then(Value::as_str);
        match kind {
            "numeric_upper_bound" | "value_equals" => {
                let pointer = scenario_pointer.ok_or_else(|| format!("{} {} requires scenario_pointer", context, kind))?;
                if !valid_json_pointer(pointer) {
                    return Err(format!("{} scenario_pointer is not a valid absolute JSON Pointer", context));
                }
            }
            "collection_contains_all_ids" => {
                let pointer = scenario_pointer.ok_or_else(|| format!("{} requires scenario_pointer", context))?;
                if !valid_json_pointer(pointer) {
                    return Err(format!("{} scenario_pointer is not a valid absolute JSON Pointer", context));
                }
                if identity_field.map(str::trim).filter(|value| !value.is_empty()).is_none() {
                    return Err(format!("{} collection_contains_all_ids requires identity_field", context));
                }
            }
            "field_present" | "metric_zero" | "boolean_true" | "field_absent" => {}
            _ => return Err(format!("{} has unsupported assertion kind {}", context, kind)),
        }
    }
    Ok(())
}

fn valid_json_pointer(pointer: &str) -> bool {
    if pointer.is_empty() || !pointer.starts_with('/') {
        return false;
    }
    pointer.split('/').skip(1).all(|segment| {
        let bytes = segment.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'~' {
                if index + 1 >= bytes.len() || !matches!(bytes[index + 1], b'0' | b'1') {
                    return false;
                }
                index += 2;
            } else {
                index += 1;
            }
        }
        true
    })
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

    fn synthetic_run_report(scenario_id: &str, variant_id: &str) -> String {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let mut scenario: Option<Value> = None;
        let mut input_line: Option<String> = None;
        for line in FIXTURE.lines() {
            let value: Value = serde_json::from_str(line).unwrap();
            if value["scenario_id"] == scenario_id && value["variant_id"] == variant_id {
                scenario = Some(value);
                input_line = Some(line.to_owned());
                break;
            }
        }
        let scenario = scenario.expect("synthetic test scenario exists");
        let input_line = input_line.expect("synthetic input line exists");
        let input_sha = sha256_hex(input_line.as_bytes());
        let mut report = json!({
            "report_version": "wis-sim-run-report/0.1",
            "run_status": "completed",
            "simulation_executed": true,
            "claim_ceiling": "synthetic verifier test report only; not a simulation result",
            "identity": {
                "corpus_id": manifest["corpus_id"],
                "corpus_version": manifest["corpus_version"],
                "fixture_sha256": manifest["fixture_sha256"],
                "scenario_id": scenario["scenario_id"],
                "scenario_version": scenario["scenario_version"],
                "variant_id": scenario["variant_id"],
                "policy_id": "no_intervention_v0",
                "policy_version": scenario["policy_version"],
                "deterministic_seed": scenario["deterministic_seed"],
                "execution_profile": scenario["execution_profile"],
                "engine_revision": "synthetic-test-engine-revision",
                "toolchain_profile": "synthetic-unit-test-toolchain",
                "target_triple": "x86_64-unknown-linux-gnu",
                "environment_fingerprint": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "input_sha256": input_sha
            },
            "metrics": {},
            "claims_retained": [],
            "claims": {},
            "decision": {},
            "objections": {},
            "events": {},
            "invariant_results": [],
            "warnings": []
        });

        let assertions = scenario["assertion_specifications"].as_array().unwrap();
        for spec in assertions {
            let kind = spec["assertion_kind"].as_str().unwrap();
            let report_pointer = spec["report_pointer"].as_str().unwrap();
            if kind == "field_absent" {
                continue;
            }
            let value = match kind {
                "numeric_upper_bound" | "value_equals" => {
                    resolve_json_pointer(&scenario, spec["scenario_pointer"].as_str().unwrap())
                        .expect("source pointer exists in synthetic scenario").clone()
                }
                "collection_contains_all_ids" => {
                    resolve_json_pointer(&scenario, spec["scenario_pointer"].as_str().unwrap())
                        .expect("source collection exists in synthetic scenario").clone()
                }
                "field_present" => json!("synthetic-unit-test-observation"),
                "metric_zero" => json!(0),
                "boolean_true" => json!(true),
                _ => panic!("unsupported assertion type in test helper: {}", kind),
            };
            set_json_pointer(&mut report, report_pointer, value);
        }

        let results: Vec<Value> = assertions.iter().map(|spec| json!({
            "assertion_id": spec["assertion_id"],
            "status": "pass",
            "evidence_pointer": spec["report_pointer"],
            "explanation": "synthetic unit-test report; no simulation was executed"
        })).collect();
        report["invariant_results"] = json!(results);
        serde_json::to_string(&report).unwrap()
    }

    fn set_json_pointer(root: &mut Value, pointer: &str, value: Value) {
        assert!(pointer.starts_with('/'));
        let segments: Vec<String> = pointer[1..].split('/')
            .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
            .collect();
        assert!(!segments.is_empty());
        let mut current = root;
        for segment in &segments[..segments.len() - 1] {
            let child_is_object = current.get(segment).map(Value::is_object).unwrap_or(false);
            if !child_is_object {
                current.as_object_mut().unwrap().insert(segment.clone(), json!({}));
            }
            current = current.as_object_mut().unwrap().get_mut(segment).unwrap();
        }
        current.as_object_mut().unwrap().insert(
            segments[segments.len() - 1].clone(),
            value,
        );
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
        let bytes = FIXTURE.trim_end().as_bytes();
        let mut manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        manifest["fixture_sha256"] = json!(sha256_hex(bytes));
        let text = serde_json::to_string(&manifest).unwrap();
        assert!(verify_bytes(bytes, &text, SCHEMA).unwrap_err().contains("final newline"));
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
    fn every_scenario_declares_machine_checkable_assertions() {
        let report = good();
        assert_eq!(report["record_count"], 28);
        assert_eq!(report["selected_top_level_schema_contract"], "MATCH");
    }

    #[test]
    fn missing_assertions_fail_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            rows[0]["assertion_specifications"] = json!([]);
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("at least three executable assertions"));
    }

    #[test]
    fn unknown_assertion_kind_fails_closed() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            rows[0]["assertion_specifications"][0]["assertion_kind"] = json!("global_wisdom_score");
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("undeclared assertion_kind"));
    }

    #[test]
    fn assertion_json_pointers_must_be_well_formed() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            rows[0]["assertion_specifications"][0]["report_pointer"] = json!("metrics.allocation");
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("not a valid absolute JSON Pointer"));
    }

    #[test]
    fn collection_assertions_require_identity_field() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            rows[0]["assertion_specifications"][1].as_object_mut().unwrap().remove("identity_field");
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("requires identity_field"));
    }


    #[test]
    fn scenario_variants_must_share_assertion_contract() {
        let (bytes, manifest) = rewritten_fixture(|rows| {
            let row = rows.iter_mut().find(|row| row["scenario_id"] == "WSS-001" && row["variant_id"] == "adversarial-pressure").unwrap();
            row["assertion_specifications"].as_array_mut().unwrap().pop();
        });
        assert!(verify_bytes(&bytes, &manifest, SCHEMA).unwrap_err().contains("changes assertion contract across variants"));
    }

    #[test]
    fn global_wisdom_and_consciousness_fields_are_forbidden_assertions() {
        let report = good();
        assert_eq!(report["record_count"], 28);
        for row in FIXTURE.lines().map(|line| serde_json::from_str::<Value>(line).unwrap()) {
            assert!(row["assertion_specifications"].as_array().unwrap().iter().any(|a| {
                a["assertion_kind"] == "field_absent" && a["report_pointer"] == "/metrics/global_wisdom_score"
            }));
            assert!(row["assertion_specifications"].as_array().unwrap().iter().any(|a| {
                a["assertion_kind"] == "field_absent" && a["report_pointer"] == "/metrics/consciousness_score"
            }));
        }
    }


    #[test]
    fn synthetic_run_report_assertions_are_rechecked_independently_for_all_scenarios() {
        for line in FIXTURE.lines() {
            let scenario: Value = serde_json::from_str(line).unwrap();
            let scenario_id = scenario["scenario_id"].as_str().unwrap();
            let variant_id = scenario["variant_id"].as_str().unwrap();
            let report = synthetic_run_report(scenario_id, variant_id);
            let receipt = verify_run_report_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA, &report).unwrap();
            assert_eq!(receipt["status"], "RUN_REPORT_ASSERTIONS_PASS", "{} / {}", scenario_id, variant_id);
            assert_eq!(receipt["assertion_count"], scenario["assertion_specifications"].as_array().unwrap().len());
        }
    }

    #[test]
    fn run_report_identity_must_match_exact_scenario_seed() {
        let mut report: Value = serde_json::from_str(&synthetic_run_report("WSS-001", "base")).unwrap();
        report["identity"]["deterministic_seed"] = json!(42);
        let text = serde_json::to_string(&report).unwrap();
        assert!(verify_run_report_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA, &text)
            .unwrap_err().contains("deterministic_seed differs"));
    }

    #[test]
    fn report_that_violates_resource_bound_cannot_qualify() {
        let mut report: Value = serde_json::from_str(&synthetic_run_report("WSS-001", "base")).unwrap();
        report["metrics"]["allocation"]["total_units"] = json!(999);
        for item in report["invariant_results"].as_array_mut().unwrap() {
            if item["assertion_id"] == "WSS-001-A1" {
                item["status"] = json!("fail");
            }
        }
        let text = serde_json::to_string(&report).unwrap();
        let receipt = verify_run_report_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA, &text).unwrap();
        assert_eq!(receipt["status"], "RUN_REPORT_ASSERTIONS_FAIL");
        assert_eq!(receipt["hard_assertions_failed"], 1);
    }

    #[test]
    fn runner_cannot_claim_pass_when_resource_bound_is_violated() {
        let mut report: Value = serde_json::from_str(&synthetic_run_report("WSS-001", "base")).unwrap();
        report["metrics"]["allocation"]["total_units"] = json!(999);
        let text = serde_json::to_string(&report).unwrap();
        assert!(verify_run_report_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA, &text)
            .unwrap_err().contains("disagrees with independent evaluation"));
    }

    #[test]
    fn global_wisdom_score_in_report_fails_independent_contract() {
        let mut report: Value = serde_json::from_str(&synthetic_run_report("WSS-001", "base")).unwrap();
        report["metrics"]["global_wisdom_score"] = json!(0.91);
        for item in report["invariant_results"].as_array_mut().unwrap() {
            if item["assertion_id"] == "WSS-001-A90" {
                item["status"] = json!("fail");
            }
        }
        let text = serde_json::to_string(&report).unwrap();
        let receipt = verify_run_report_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA, &text).unwrap();
        assert_eq!(receipt["status"], "RUN_REPORT_ASSERTIONS_FAIL");
    }

    #[test]
    fn unexecuted_run_is_not_a_qualification_pass() {
        let mut report: Value = serde_json::from_str(&synthetic_run_report("WSS-001", "base")).unwrap();
        report["run_status"] = json!("not_run");
        report["simulation_executed"] = json!(false);
        let text = serde_json::to_string(&report).unwrap();
        let receipt = verify_run_report_bytes(FIXTURE.as_bytes(), MANIFEST, SCHEMA, &text).unwrap();
        assert_eq!(receipt["status"], "RUN_NOT_QUALIFIED");
        assert_eq!(receipt["assertions_evaluated"], false);
    }

    #[test]
    fn identical_inputs_produce_identical_receipt_bytes() {
        let first = serde_json::to_vec(&good()).unwrap();
        let second = serde_json::to_vec(&good()).unwrap();
        assert_eq!(first, second);
    }
}

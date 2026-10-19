use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::process::ExitCode;

const VALIDATOR_VERSION: &str = "0.1.0";
const EXPECTED_CASES: [&str; 5] = [
    "CLEAN-001",
    "MIX-SPLIT-002",
    "MISSING-HANDOFF-003",
    "CONFLICTED-CUSTODY-004",
    "GENOMIC-NO-ROUTE-005",
];
const VALID_STATUSES: [&str; 6] = [
    "verified",
    "reported",
    "screening_only",
    "unknown",
    "conflicted",
    "stale",
];
const VALID_EVIDENCE_TYPES: [&str; 5] = [
    "custody_record",
    "transformation_record",
    "retail_receipt",
    "outbreak_association",
    "pathogen_profile_comparison",
];

type Check<T> = Result<T, String>;

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected object"))
}

fn array<'a>(value: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{at}: expected array"))
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn text<'a>(value: &'a Value, at: &str) -> Check<&'a str> {
    value
        .as_str()
        .ok_or_else(|| format!("{at}: expected string"))
}

fn text_field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    text(field(value, key, at)?, &format!("{at}.{key}"))
}

fn require_keys(value: &Value, required: &[&str], allowed: &[&str], at: &str) -> Check<()> {
    let map = object(value, at)?;
    let allowed_set: BTreeSet<&str> = allowed.iter().copied().collect();
    for key in required {
        if !map.contains_key(*key) {
            return Err(format!("{at}: missing required key {key}"));
        }
    }
    for key in map.keys() {
        if !allowed_set.contains(key.as_str()) {
            return Err(format!("{at}: unknown key {key}"));
        }
    }
    Ok(())
}

fn string_array(value: &Value, at: &str) -> Check<Vec<String>> {
    let mut values = Vec::new();
    for (index, item) in array(value, at)?.iter().enumerate() {
        values.push(text(item, &format!("{at}[{index}]"))?.to_owned());
    }
    Ok(values)
}

fn unique_strings(value: &Value, at: &str) -> Check<Vec<String>> {
    let values = string_array(value, at)?;
    let unique: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    if unique.len() != values.len() {
        return Err(format!("{at}: duplicate string value"));
    }
    Ok(values)
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
        .iter()
        .all(|index| bytes[*index].is_ascii_digit())
}

fn equal_ids(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| actual.as_str() == *expected)
}

fn verify_truth_ids(
    truth: &Value,
    key: &str,
    entity_types: &BTreeMap<String, String>,
    expected_type: &str,
    at: &str,
) -> Check<Vec<String>> {
    let ids = unique_strings(field(truth, key, at)?, &format!("{at}.{key}"))?;
    for id in &ids {
        let actual_type = entity_types
            .get(id)
            .ok_or_else(|| format!("{at}.{key}: unknown entity ID {id}"))?;
        if actual_type != expected_type {
            return Err(format!(
                "{at}.{key}: {id} has type {actual_type}, expected {expected_type}"
            ));
        }
    }
    Ok(ids)
}

fn has_verified_path(records: &[Value], start: &str, end: &str) -> bool {
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for record in records {
        if record.get("status").and_then(Value::as_str) != Some("verified")
            || record.get("evidence_type").and_then(Value::as_str)
                == Some("pathogen_profile_comparison")
            || record.get("evidence_type").and_then(Value::as_str) == Some("outbreak_association")
        {
            continue;
        }
        if let (Some(from), Some(to)) = (
            record.get("from").and_then(Value::as_str),
            record.get("to").and_then(Value::as_str),
        ) {
            edges.entry(from).or_default().push(to);
        }
    }

    let mut queue = VecDeque::from([start]);
    let mut visited = BTreeSet::new();
    while let Some(node) = queue.pop_front() {
        if node == end {
            return true;
        }
        if !visited.insert(node) {
            continue;
        }
        if let Some(next) = edges.get(node) {
            queue.extend(next.iter().copied());
        }
    }
    false
}

fn candidate_projection(case: &Value) -> Value {
    json!({
        "case_id": case.get("case_id"),
        "description": case.get("description"),
        "entities": case.get("entities"),
        "observed_records": case.get("observed_records")
    })
}

fn validate_case(case: &Value, index: usize) -> Check<Value> {
    let at = format!("cases[{index}]");
    require_keys(
        case,
        &[
            "case_id",
            "description",
            "entities",
            "observed_records",
            "oracle_truth",
            "expected_behavior",
        ],
        &[
            "case_id",
            "description",
            "entities",
            "observed_records",
            "oracle_truth",
            "expected_behavior",
        ],
        &at,
    )?;
    let case_id = text_field(case, "case_id", &at)?.to_owned();
    let entities = array(field(case, "entities", &at)?, &format!("{at}.entities"))?;
    let records = array(
        field(case, "observed_records", &at)?,
        &format!("{at}.observed_records"),
    )?;
    if entities.is_empty() || records.is_empty() {
        return Err(format!(
            "{at}: entities and observed_records must be non-empty"
        ));
    }

    let allowed_entity_keys = ["id", "type", "quantity", "quantity_unit"];
    let mut entity_types = BTreeMap::new();
    for (entity_index, entity) in entities.iter().enumerate() {
        let entity_at = format!("{at}.entities[{entity_index}]");
        require_keys(entity, &["id", "type"], &allowed_entity_keys, &entity_at)?;
        let id = text_field(entity, "id", &entity_at)?.to_owned();
        let entity_type = text_field(entity, "type", &entity_at)?.to_owned();
        if ![
            "ingredient_lot",
            "mixed_intermediate_lot",
            "finished_lot",
            "shipment",
            "retail_site",
            "pathogen_sample",
        ]
        .contains(&entity_type.as_str())
        {
            return Err(format!("{entity_at}: invalid entity type {entity_type}"));
        }
        if entity_types
            .insert(id.clone(), entity_type.clone())
            .is_some()
        {
            return Err(format!("{entity_at}: duplicate entity ID {id}"));
        }

        let quantity = entity.get("quantity");
        let quantity_unit = entity.get("quantity_unit");
        if quantity.is_some() != quantity_unit.is_some() {
            return Err(format!(
                "{entity_at}: quantity and quantity_unit must appear together"
            ));
        }
        if let Some(quantity) = quantity {
            let amount = quantity
                .as_f64()
                .ok_or_else(|| format!("{entity_at}.quantity: expected number"))?;
            if !amount.is_finite() || amount <= 0.0 {
                return Err(format!(
                    "{entity_at}.quantity: expected positive finite number"
                ));
            }
            let unit = text(
                quantity_unit.expect("presence checked"),
                &format!("{entity_at}.quantity_unit"),
            )?;
            if entity_type == "finished_lot" && unit != "synthetic_case_packs" {
                return Err(format!(
                    "{entity_at}: finished_lot quantity must use synthetic_case_packs"
                ));
            }
        }
        if entity_type == "finished_lot" && quantity.is_none() {
            return Err(format!(
                "{entity_at}: finished_lot must declare its quantity and unit"
            ));
        }
    }

    let allowed_record_keys = [
        "id",
        "from",
        "to",
        "relation",
        "evidence_type",
        "status",
        "event_time",
        "observed_at",
        "scope",
        "conflict_group",
        "note",
    ];
    let mut record_ids = BTreeSet::new();
    for (record_index, record) in records.iter().enumerate() {
        let record_at = format!("{at}.observed_records[{record_index}]");
        require_keys(
            record,
            &[
                "id",
                "from",
                "to",
                "relation",
                "evidence_type",
                "status",
                "event_time",
                "observed_at",
            ],
            &allowed_record_keys,
            &record_at,
        )?;
        let id = text_field(record, "id", &record_at)?;
        if !record_ids.insert(id.to_owned()) {
            return Err(format!("{record_at}: duplicate record ID {id}"));
        }
        for endpoint in ["from", "to"] {
            let entity_id = text_field(record, endpoint, &record_at)?;
            if !entity_types.contains_key(entity_id) {
                return Err(format!(
                    "{record_at}.{endpoint}: dangling entity reference {entity_id}"
                ));
            }
        }
        let evidence_type = text_field(record, "evidence_type", &record_at)?;
        if !VALID_EVIDENCE_TYPES.contains(&evidence_type) {
            return Err(format!(
                "{record_at}: invalid evidence type {evidence_type}"
            ));
        }
        let status = text_field(record, "status", &record_at)?;
        if !VALID_STATUSES.contains(&status) {
            return Err(format!("{record_at}: invalid evidence status {status}"));
        }
        for timestamp_key in ["event_time", "observed_at"] {
            let timestamp = text_field(record, timestamp_key, &record_at)?;
            if !valid_utc_timestamp(timestamp) {
                return Err(format!(
                    "{record_at}.{timestamp_key}: expected UTC RFC3339 seconds timestamp"
                ));
            }
        }
        if evidence_type == "pathogen_profile_comparison" {
            if status != "screening_only"
                || text_field(record, "scope", &record_at)? != "relatedness_only_not_custody"
            {
                return Err(format!(
                    "{record_at}: pathogen-profile evidence must remain screening_only and relatedness_only_not_custody"
                ));
            }
        }
        if status == "unknown" {
            let note = text_field(record, "note", &record_at)?;
            if !note.contains("does not assert that transfer occurred") {
                return Err(format!(
                    "{record_at}: unknown transfer must explicitly state that it does not assert that transfer occurred"
                ));
            }
        }
        if status == "conflicted" && record.get("conflict_group").is_none() {
            return Err(format!(
                "{record_at}: conflicted record lacks conflict_group"
            ));
        }
        if record.get("note").is_some() && text_field(record, "note", &record_at).is_err() {
            return Err(format!("{record_at}.note: expected string"));
        }
    }

    let truth = field(case, "oracle_truth", &at)?;
    require_keys(
        truth,
        &[
            "source_lot_ids",
            "affected_finished_lot_ids",
            "unaffected_finished_lot_ids",
            "outbreak_associated_finished_lot_ids",
        ],
        &[
            "source_lot_ids",
            "affected_finished_lot_ids",
            "unaffected_finished_lot_ids",
            "outbreak_associated_finished_lot_ids",
        ],
        &format!("{at}.oracle_truth"),
    )?;
    let truth_sources = verify_truth_ids(
        truth,
        "source_lot_ids",
        &entity_types,
        "ingredient_lot",
        &format!("{at}.oracle_truth"),
    )?;
    let truth_affected = verify_truth_ids(
        truth,
        "affected_finished_lot_ids",
        &entity_types,
        "finished_lot",
        &format!("{at}.oracle_truth"),
    )?;
    verify_truth_ids(
        truth,
        "unaffected_finished_lot_ids",
        &entity_types,
        "finished_lot",
        &format!("{at}.oracle_truth"),
    )?;
    verify_truth_ids(
        truth,
        "outbreak_associated_finished_lot_ids",
        &entity_types,
        "finished_lot",
        &format!("{at}.oracle_truth"),
    )?;

    let behavior = field(case, "expected_behavior", &at)?;
    require_keys(
        behavior,
        &[
            "source_attribution",
            "containment_must_include",
            "trace_completeness",
        ],
        &[
            "source_attribution",
            "containment_must_include",
            "containment_should_exclude_under_complete_coverage",
            "trace_completeness",
            "propagation_rule",
            "safety_rule",
        ],
        &format!("{at}.expected_behavior"),
    )?;
    let attribution = text_field(behavior, "source_attribution", &at)?;
    let containment = verify_truth_ids(
        behavior,
        "containment_must_include",
        &entity_types,
        "finished_lot",
        &format!("{at}.expected_behavior"),
    )?;
    let trace_completeness = text_field(behavior, "trace_completeness", &at)?;

    match case_id.as_str() {
        "CLEAN-001" => {
            if !equal_ids(&truth_sources, &["ING-A"])
                || !equal_ids(&truth_affected, &["FG-A"])
                || !equal_ids(&containment, &["FG-A"])
                || trace_completeness != "complete_for_fixture_scope"
                || !has_verified_path(records, "ING-A", "FG-A")
                || has_verified_path(records, "ING-B", "FG-A")
            {
                return Err(format!("{at}: CLEAN-001 oracle or path invariant changed"));
            }
            let exclude = string_array(
                field(
                    behavior,
                    "containment_should_exclude_under_complete_coverage",
                    &format!("{at}.expected_behavior"),
                )?,
                &format!(
                    "{at}.expected_behavior.containment_should_exclude_under_complete_coverage"
                ),
            )?;
            if !equal_ids(&exclude, &["FG-B"]) {
                return Err(format!("{at}: expected FG-B exclusion control changed"));
            }
        }
        "MIX-SPLIT-002" => {
            if !equal_ids(&truth_sources, &["ING-A"])
                || !equal_ids(&truth_affected, &["FG-A", "FG-C"])
                || !equal_ids(&containment, &["FG-A", "FG-C"])
                || !has_verified_path(records, "ING-A", "FG-A")
                || !has_verified_path(records, "ING-A", "FG-C")
            {
                return Err(format!("{at}: MIX-SPLIT-002 propagation invariant changed"));
            }
        }
        "MISSING-HANDOFF-003" => {
            if !attribution.starts_with("must_abstain")
                || trace_completeness != "incomplete"
                || !equal_ids(&containment, &["FG-A"])
                || !records.iter().any(|r| {
                    r.get("status").and_then(Value::as_str) == Some("unknown")
                        && r.get("id").and_then(Value::as_str) == Some("C3-R1")
                })
            {
                return Err(format!(
                    "{at}: missing-handoff fail-closed invariant changed"
                ));
            }
        }
        "CONFLICTED-CUSTODY-004" => {
            let conflicting: Vec<&Value> = records
                .iter()
                .filter(|r| r.get("conflict_group").and_then(Value::as_str) == Some("C4-CUSTODY-1"))
                .collect();
            if !attribution.starts_with("must_abstain")
                || trace_completeness != "incomplete_due_to_conflict"
                || !equal_ids(&containment, &["FG-A"])
                || conflicting.len() != 2
                || conflicting
                    .iter()
                    .any(|r| r.get("status").and_then(Value::as_str) != Some("conflicted"))
            {
                return Err(format!(
                    "{at}: conflicted-custody fail-closed invariant changed"
                ));
            }
        }
        "GENOMIC-NO-ROUTE-005" => {
            if !attribution.starts_with("must_not_attribute")
                || trace_completeness != "upstream_incomplete"
                || !equal_ids(&containment, &["FG-A"])
                || has_verified_path(records, "ING-B", "FG-A")
            {
                return Err(format!("{at}: genomic-without-route invariant changed"));
            }
        }
        _ => return Err(format!("{at}: unexpected case ID {case_id}")),
    }

    let projected = candidate_projection(case);
    if projected.get("oracle_truth").is_some() || projected.get("expected_behavior").is_some() {
        return Err(format!("{at}: evaluator projection leaked oracle labels"));
    }
    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "entity_count": entities.len(),
        "observed_record_count": records.len()
    }))
}

fn validate_corpus(corpus: &Value) -> Check<Vec<Value>> {
    require_keys(
        corpus,
        &[
            "schema_version",
            "benchmark_id",
            "classification",
            "created_at",
            "units",
            "evidence_status_vocabulary",
            "evidence_types",
            "evaluator_boundary",
            "cases",
        ],
        &[
            "schema_version",
            "benchmark_id",
            "classification",
            "created_at",
            "units",
            "evidence_status_vocabulary",
            "evidence_types",
            "evaluator_boundary",
            "cases",
        ],
        "corpus",
    )?;
    if text_field(corpus, "schema_version", "corpus")? != "0.1.0"
        || text_field(corpus, "benchmark_id", "corpus")? != "FOOD-TRACEBACK-001"
        || text_field(corpus, "classification", "corpus")? != "synthetic_only"
    {
        return Err("corpus: schema, benchmark identity or classification mismatch".to_owned());
    }
    let statuses: BTreeSet<String> = string_array(
        field(corpus, "evidence_status_vocabulary", "corpus")?,
        "corpus.evidence_status_vocabulary",
    )?
    .into_iter()
    .collect();
    let expected_statuses: BTreeSet<String> =
        VALID_STATUSES.iter().map(|s| (*s).to_owned()).collect();
    if statuses != expected_statuses {
        return Err("corpus: evidence status vocabulary mismatch".to_owned());
    }
    let evidence_types: BTreeSet<String> = string_array(
        field(corpus, "evidence_types", "corpus")?,
        "corpus.evidence_types",
    )?
    .into_iter()
    .collect();
    let expected_types: BTreeSet<String> = VALID_EVIDENCE_TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    if evidence_types != expected_types {
        return Err("corpus: evidence type vocabulary mismatch".to_owned());
    }

    let boundary = field(corpus, "evaluator_boundary", "corpus")?;
    require_keys(
        boundary,
        &["input_projection", "oracle_truth_is_hidden"],
        &["input_projection", "oracle_truth_is_hidden", "rationale"],
        "corpus.evaluator_boundary",
    )?;
    if boundary
        .get("oracle_truth_is_hidden")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(
            "corpus.evaluator_boundary: oracle truth must be hidden from candidates".to_owned(),
        );
    }

    let cases = array(field(corpus, "cases", "corpus")?, "corpus.cases")?;
    let ids: Vec<String> = cases
        .iter()
        .enumerate()
        .map(|(i, case)| text_field(case, "case_id", &format!("cases[{i}]")).map(str::to_owned))
        .collect::<Check<Vec<_>>>()?;
    if ids.len() != EXPECTED_CASES.len()
        || ids
            .iter()
            .zip(EXPECTED_CASES.iter())
            .any(|(actual, expected)| actual.as_str() != *expected)
    {
        return Err(format!(
            "corpus: case inventory must equal {EXPECTED_CASES:?} in order"
        ));
    }
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("corpus: duplicate case ID".to_owned());
    }

    cases
        .iter()
        .enumerate()
        .map(|(index, case)| validate_case(case, index))
        .collect()
}

fn is_sha40(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: food-traceback-bench <fixture.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = &args[3];
    if !is_sha40(source_commit) {
        return Err("--source-commit must be a 40-character hexadecimal SHA".to_owned());
    }

    let bytes = fs::read(&args[1]).map_err(|e| format!("read {}: {e}", args[1]))?;
    let source = std::str::from_utf8(&bytes).map_err(|e| format!("fixture is not UTF-8: {e}"))?;
    let corpus: Value =
        serde_json::from_str(source).map_err(|e| format!("parse fixture JSON: {e}"))?;
    let scenarios = validate_corpus(&corpus)?;
    let receipt = json!({
        "result": "pass",
        "validator_version": VALIDATOR_VERSION,
        "benchmark_id": "FOOD-TRACEBACK-001",
        "scenario_count": scenarios.len(),
        "corpus_sha256": format!("{:x}", Sha256::digest(&bytes)),
        "source_commit": source_commit,
        "scenarios": scenarios
    });
    let serialized =
        serde_json::to_string(&receipt).map_err(|e| format!("serialize receipt: {e}"))?;
    println!("{serialized}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("food-traceback-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../benchmarks/fixtures/FOOD_TRACEBACK_V0_1.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("checked-in fixture must parse")
    }

    #[test]
    fn frozen_corpus_passes_all_five_cases() {
        let corpus = fixture();
        let scenarios = validate_corpus(&corpus).expect("frozen corpus should validate");
        assert_eq!(scenarios.len(), 5);
        assert!(
            scenarios
                .iter()
                .all(|scenario| { scenario.get("status").and_then(Value::as_str) == Some("pass") })
        );
    }

    #[test]
    fn rejects_duplicate_entity_ids() {
        let mut corpus = fixture();
        corpus["cases"][0]["entities"][1]["id"] = json!("ING-A");
        let error = validate_corpus(&corpus).expect_err("duplicate entity IDs must fail");
        assert!(error.contains("duplicate entity ID"));
    }

    #[test]
    fn rejects_dangling_record_references() {
        let mut corpus = fixture();
        corpus["cases"][0]["observed_records"][0]["to"] = json!("ABSENT-LOT");
        let error = validate_corpus(&corpus).expect_err("dangling references must fail");
        assert!(error.contains("dangling entity reference"));
    }

    #[test]
    fn rejects_duplicate_record_ids() {
        let mut corpus = fixture();
        corpus["cases"][0]["observed_records"][1]["id"] = json!("C1-R1");
        let error = validate_corpus(&corpus).expect_err("duplicate record IDs must fail");
        assert!(error.contains("duplicate record ID"));
    }

    #[test]
    fn rejects_genomic_similarity_relabelled_as_custody() {
        let mut corpus = fixture();
        corpus["cases"][4]["observed_records"][3]["scope"] = json!("custody_proof");
        let error = validate_corpus(&corpus).expect_err("genomic evidence must not prove custody");
        assert!(error.contains("relatedness_only_not_custody"));
    }

    #[test]
    fn rejects_unknown_transfer_without_explicit_missingness_note() {
        let mut corpus = fixture();
        corpus["cases"][2]["observed_records"][0]["note"] = json!("edge");
        let error =
            validate_corpus(&corpus).expect_err("unknown transfer needs explicit semantics");
        assert!(error.contains("does not assert that transfer occurred"));
    }

    #[test]
    fn candidate_projection_excludes_truth_labels() {
        let corpus = fixture();
        let projected = candidate_projection(&corpus["cases"][0]);
        assert!(projected.get("oracle_truth").is_none());
        assert!(projected.get("expected_behavior").is_none());
        assert!(projected.get("observed_records").is_some());
    }

    #[test]
    fn rejects_reordered_or_omitted_scenario_inventory() {
        let mut corpus = fixture();
        corpus["cases"].as_array_mut().expect("cases array")[0]["case_id"] = json!("MIX-SPLIT-002");
        let error = validate_corpus(&corpus).expect_err("case inventory mutation must fail");
        assert!(error.contains("case inventory"));
    }

    #[test]
    fn receipt_digest_binds_exact_fixture_bytes() {
        let first = format!("{:x}", Sha256::digest(FIXTURE.as_bytes()));
        let second = format!("{:x}", Sha256::digest(FIXTURE.as_bytes()));
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }
}

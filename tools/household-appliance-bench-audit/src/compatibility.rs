use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA_ID: &str = "urn:luminous-dynamics:parts-compatibility-evidence-graph:0.1.0";
const GRAPH_ID: &str = "PARTS-EVIDENCE-001";
const EXPECTED_MODELS: [&str; 3] = [
    "SYN-MODEL-WASHER-A",
    "SYN-MODEL-WASHER-B",
    "SYN-MODEL-HVAC-A",
];
const EXPECTED_PARTS: [&str; 5] = [
    "SYN-PART-PUMP-A",
    "SYN-PART-SEAL-A",
    "SYN-PART-VALVE-A",
    "SYN-PART-PUMP-OLD",
    "SYN-PART-FAN-A",
];
const EXPECTED_CLAIMS: [&str; 9] = [
    "SYN-COMP-001",
    "SYN-COMP-002",
    "SYN-COMP-003",
    "SYN-COMP-004",
    "SYN-COMP-005",
    "SYN-COMP-006",
    "SYN-COMP-007",
    "SYN-COMP-008",
    "SYN-COMP-009",
];
const EXPECTED_EVIDENCE: [&str; 14] = [
    "SYN-EV-001",
    "SYN-EV-002",
    "SYN-EV-003",
    "SYN-EV-004",
    "SYN-EV-005",
    "SYN-EV-006",
    "SYN-EV-007",
    "SYN-EV-008",
    "SYN-EV-009",
    "SYN-EV-010",
    "SYN-EV-011",
    "SYN-EV-012",
    "SYN-EV-013",
    "SYN-EV-014",
];
const EXPECTED_PATHS: [&str; 3] = ["SYN-PATH-001", "SYN-PATH-002", "SYN-PATH-003"];
const EXPECTED_BENCHMARKS: [&str; 6] = [
    "PC-B-001", "PC-B-002", "PC-B-003", "PC-B-004", "PC-B-005", "PC-B-006",
];
const VALID_FAMILIES: [&str; 9] = [
    "F1-WASHER",
    "F2-DISHWASHER",
    "F3-REFRIGERATOR-FREEZER",
    "F4-HEAT-PUMP-DRYER",
    "F5-WASHER-DRYER-COMBO",
    "F6-HEAT-PUMP-WATER-HEATER",
    "F7-RESIDENTIAL-HVAC",
    "F8-COOKING-APPLIANCES",
    "F9-VENTILATION-HRV",
];
type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CompatibilityAuditSummary {
    pub(crate) model_count: usize,
    pub(crate) part_count: usize,
    pub(crate) claim_count: usize,
    pub(crate) evidence_count: usize,
    pub(crate) supplier_path_count: usize,
    pub(crate) functional_claim_examples: usize,
    pub(crate) incompatible_claim_examples: usize,
    pub(crate) unknown_claim_examples: usize,
    pub(crate) independent_delivered_groups_in_fixture: usize,
    pub(crate) benchmark_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) graph_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("compatibility schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("compatibility schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "compatibility schema does not validate against the Draft 2020-12 meta-schema"
                .to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile compatibility schema: {error}"))
}

fn kinds_for_claim<'a>(
    claim: &Value,
    evidence: &'a BTreeMap<String, &'a Value>,
) -> BTreeSet<&'a str> {
    claim
        .get("evidence_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|id| evidence.get(id).copied())
        .filter_map(|item| text_at(item, "evidence_kind"))
        .collect()
}

fn audit_values(
    schema: &Value,
    graph: &Value,
    schema_raw: &str,
    graph_raw: &str,
) -> Result<CompatibilityAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(graph)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }
    let mut errors = Vec::new();
    if text_at(graph, "schema_version") != Some("0.1.0")
        || text_at(graph, "graph_id") != Some(GRAPH_ID)
        || text_at(graph, "status") != Some("synthetic_contract_examples_only")
        || text_at(graph, "claim_ceiling")
            != Some("synthetic_examples_are_not_real_fitment_supply_repair_or_safety_evidence")
        || text_at(graph, "target_market") != Some("ZA")
    {
        errors.push(
            "compatibility graph must remain ZA synthetic-only with its no-claim ceiling"
                .to_owned(),
        );
    }
    let Some(models) = graph.get("models").and_then(Value::as_array) else {
        return Err(vec!["models must be an array".to_owned()]);
    };
    let Some(parts) = graph.get("parts").and_then(Value::as_array) else {
        return Err(vec!["parts must be an array".to_owned()]);
    };
    let Some(ev_items) = graph.get("evidence").and_then(Value::as_array) else {
        return Err(vec!["evidence must be an array".to_owned()]);
    };
    let Some(claims) = graph.get("compatibility_claims").and_then(Value::as_array) else {
        return Err(vec!["compatibility_claims must be an array".to_owned()]);
    };
    let Some(paths) = graph.get("supplier_paths").and_then(Value::as_array) else {
        return Err(vec!["supplier_paths must be an array".to_owned()]);
    };
    let Some(benchmarks) = graph.get("benchmarks").and_then(Value::as_array) else {
        return Err(vec!["benchmarks must be an array".to_owned()]);
    };
    if models.len() != 3
        || parts.len() != 5
        || ev_items.len() != 14
        || claims.len() != 9
        || paths.len() != 3
        || benchmarks.len() != 6
    {
        errors.push("synthetic compatibility fixture census changed".to_owned());
    }
    let mut model_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    let mut model_ids = BTreeSet::new();
    for (i, model) in models.iter().enumerate() {
        let Some(id) = text_at(model, "model_id") else {
            errors.push(format!("models[{i}].model_id must be a string"));
            continue;
        };
        if !model_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate model ID {id}"));
        }
        if EXPECTED_MODELS.get(i).copied() != Some(id) {
            errors.push(format!("model identity/order changed at index {i}: {id}"));
        }
        if !VALID_FAMILIES.contains(&text_at(model, "family_id").unwrap_or("")) {
            errors.push(format!("{id}: unknown appliance family"));
        }
        if text_at(model, "evidence_class") != Some("synthetic_fixture") {
            errors.push(format!("{id}: model must remain synthetic"));
        }
        let has_from = model.get("serial_from_token").is_some_and(|v| !v.is_null());
        let has_to = model.get("serial_to_token").is_some_and(|v| !v.is_null());
        if (text_at(model, "serial_scope") == Some("known_window")) != (has_from && has_to)
            || (has_from != has_to)
        {
            errors.push(format!("{id}: serial scope and bounds conflict"));
        }
        model_by_id.insert(id.to_owned(), model);
    }
    let mut part_ids = BTreeSet::new();
    for (i, part) in parts.iter().enumerate() {
        let Some(id) = text_at(part, "part_id") else {
            errors.push(format!("parts[{i}].part_id must be a string"));
            continue;
        };
        if !part_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate part ID {id}"));
        }
        if EXPECTED_PARTS.get(i).copied() != Some(id) {
            errors.push(format!("part identity/order changed at index {i}: {id}"));
        }
        if text_at(part, "evidence_class") != Some("synthetic_fixture") {
            errors.push(format!("{id}: part must remain synthetic"));
        }
    }
    let mut evidence_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    let mut evidence_ids = BTreeSet::new();
    for (i, item) in ev_items.iter().enumerate() {
        let Some(id) = text_at(item, "evidence_id") else {
            errors.push(format!("evidence[{i}].evidence_id must be a string"));
            continue;
        };
        if !evidence_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate evidence ID {id}"));
        }
        if EXPECTED_EVIDENCE.get(i).copied() != Some(id) {
            errors.push(format!(
                "evidence identity/order changed at index {i}: {id}"
            ));
        }
        if text_at(item, "evidence_class") != Some("synthetic_fixture")
            || item.get("empirical_eligible").and_then(Value::as_bool) != Some(false)
            || item.get("source_url").is_some_and(|v| !v.is_null())
        {
            errors.push(format!(
                "{id}: synthetic evidence cannot be promoted to empirical/source-backed evidence"
            ));
        }
        evidence_by_id.insert(id.to_owned(), item);
    }
    let mut claim_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    let mut claim_ids = BTreeSet::new();
    for (i, claim) in claims.iter().enumerate() {
        let Some(id) = text_at(claim, "claim_id") else {
            errors.push(format!(
                "compatibility_claims[{i}].claim_id must be a string"
            ));
            continue;
        };
        if !claim_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate compatibility claim {id}"));
        }
        if EXPECTED_CLAIMS.get(i).copied() != Some(id) {
            errors.push(format!("claim identity/order changed at index {i}: {id}"));
        }
        if claim.get("empirical_eligible").and_then(Value::as_bool) != Some(false) {
            errors.push(format!("{id}: claim cannot be empirically eligible"));
        }
        let model_id = text_at(claim, "model_id").unwrap_or("");
        let part_id = text_at(claim, "part_id").unwrap_or("");
        let Some(model) = model_by_id.get(model_id) else {
            errors.push(format!("{id}: unknown model {model_id}"));
            continue;
        };
        if !part_ids.contains(part_id) {
            errors.push(format!("{id}: unknown part {part_id}"));
        }
        if text_at(claim, "market_code") != text_at(model, "market_code") {
            errors.push(format!("{id}: claim market differs from model market"));
        }
        if text_at(claim, "model_revision_scope") == Some("exact_revision")
            && text_at(claim, "model_revision_token") != text_at(model, "revision_token")
        {
            errors.push(format!(
                "{id}: exact-revision claim must match model revision"
            ));
        }
        if text_at(claim, "model_revision_scope") == Some("unknown_revision")
            && matches!(
                text_at(claim, "claim_status"),
                Some("physical_fit_verified" | "functional_compatibility_verified")
            )
        {
            errors.push(format!(
                "{id}: unknown revision cannot support positive fitment"
            ));
        }
        if text_at(claim, "serial_scope") == Some("known_window")
            && text_at(model, "serial_scope") != Some("known_window")
        {
            errors.push(format!(
                "{id}: claim serial window not supported by model identity"
            ));
        }
        if let Some(until) = text_at(claim, "effective_until") {
            if until < text_at(graph, "as_of_date").unwrap_or("")
                && text_at(claim, "claim_status") != Some("superseded")
            {
                errors.push(format!(
                    "{id}: expired claim cannot be presented as current"
                ));
            }
        }
        let mut refs = BTreeSet::new();
        for ev_id in claim
            .get("evidence_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !evidence_by_id.contains_key(ev_id) {
                errors.push(format!("{id}: unknown evidence reference {ev_id}"));
            }
            if !refs.insert(ev_id.to_owned()) {
                errors.push(format!("{id}: duplicate evidence reference {ev_id}"));
            }
        }
        let kinds = kinds_for_claim(claim, &evidence_by_id);
        let status = text_at(claim, "claim_status").unwrap_or("");
        let fit = text_at(claim, "physical_fit_check").unwrap_or("");
        let func = text_at(claim, "functional_check").unwrap_or("");
        let safety = text_at(claim, "safety_qualification").unwrap_or("");
        match status {
            "manufacturer_listed_only" if !kinds.contains("manufacturer_catalogue") => errors.push(
                format!("{id}: manufacturer-listed requires catalog evidence"),
            ),
            "seller_claim_only"
                if !kinds.contains("seller_claim")
                    || fit != "not_assessed"
                    || func != "not_assessed" =>
            {
                errors.push(format!("{id}: seller claim cannot imply fit/function"))
            }
            "cross_reference_inferred"
                if !kinds.contains("cross_reference")
                    || fit != "not_assessed"
                    || func != "not_assessed" =>
            {
                errors.push(format!("{id}: cross-reference cannot imply fit/function"))
            }
            "physical_fit_verified"
                if fit != "passed" || !kinds.contains("physical_fit_test") || func == "failed" =>
            {
                errors.push(format!(
                    "{id}: physical fit requires fit-test evidence and no hidden functional failure"
                ))
            }
            "functional_compatibility_verified"
                if fit != "passed"
                    || func != "passed"
                    || !kinds.contains("physical_fit_test")
                    || !kinds.contains("functional_test") =>
            {
                errors.push(format!(
                    "{id}: functional compatibility requires fit and functional evidence"
                ))
            }
            "verified_incompatible"
                if ((!matches!(fit, "failed") && !matches!(func, "failed"))
                    || !kinds.contains("negative_fit_test")) =>
            {
                errors.push(format!(
                    "{id}: incompatibility requires failed check and negative evidence"
                ))
            }
            "unknown"
                if fit != "not_assessed"
                    || func != "not_assessed"
                    || !refs.is_empty()
                    || safety != "not_assessed" =>
            {
                errors.push(format!(
                    "{id}: unknown cannot contain invented result/evidence"
                ))
            }
            "superseded"
                if text_at(claim, "superseded_by_claim_id").is_none()
                    || !kinds.contains("supersession_record")
                    || text_at(claim, "effective_until").is_none() =>
            {
                errors.push(format!(
                    "{id}: superseded claim requires dated successor and record"
                ))
            }
            _ => {}
        }
        if safety == "qualified_for_declared_scope"
            && (status != "functional_compatibility_verified"
                || !kinds.contains("safety_qualification"))
        {
            errors.push(format!("{id}: safety qualification requires a functional claim and distinct safety evidence"));
        }
        if status != "superseded" && text_at(claim, "superseded_by_claim_id").is_some() {
            errors.push(format!("{id}: non-superseded claim cannot name successor"));
        }
        claim_by_id.insert(id.to_owned(), claim);
    }
    for claim in claims {
        if text_at(claim, "claim_status") != Some("superseded") {
            continue;
        }
        let id = text_at(claim, "claim_id").unwrap_or("");
        let Some(successor_id) = text_at(claim, "superseded_by_claim_id") else {
            continue;
        };
        let Some(successor) = claim_by_id.get(successor_id) else {
            errors.push(format!("{id}: unknown successor claim {successor_id}"));
            continue;
        };
        if text_at(successor, "model_id") != text_at(claim, "model_id")
            || text_at(successor, "claim_status") == Some("superseded")
        {
            errors.push(format!(
                "{id}: successor must be current for the same model identity"
            ));
        }
    }
    let mut path_ids = BTreeSet::new();
    let mut delivered_groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (i, path) in paths.iter().enumerate() {
        let Some(id) = text_at(path, "path_id") else {
            errors.push(format!("supplier_paths[{i}].path_id must be a string"));
            continue;
        };
        if !path_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate supplier path {id}"));
        }
        if EXPECTED_PATHS.get(i).copied() != Some(id) {
            errors.push(format!(
                "supplier path identity/order changed at index {i}: {id}"
            ));
        }
        if path.get("empirical_eligible").and_then(Value::as_bool) != Some(false) {
            errors.push(format!(
                "{id}: supplier path cannot be empirically eligible"
            ));
        }
        let claim_id = text_at(path, "claim_id").unwrap_or("");
        let Some(claim) = claim_by_id.get(claim_id) else {
            errors.push(format!("{id}: unknown claim {claim_id}"));
            continue;
        };
        if text_at(claim, "claim_status") != Some("functional_compatibility_verified") {
            errors.push(format!(
                "{id}: supplier path must reference a function-verified claim"
            ));
        }
        let mut delivery_receipt = false;
        for ev_id in path
            .get("evidence_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let Some(item) = evidence_by_id.get(ev_id) else {
                errors.push(format!("{id}: unknown evidence {ev_id}"));
                continue;
            };
            if text_at(item, "evidence_kind") == Some("delivery_receipt") {
                delivery_receipt = true;
            }
        }
        if text_at(path, "commercial_state") == Some("delivered_inspected") && !delivery_receipt {
            errors.push(format!("{id}: delivered path requires delivery receipt"));
        }
        let path_kinds: BTreeSet<&str> = path
            .get("evidence_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|ev_id| evidence_by_id.get(ev_id).copied())
            .filter_map(|item| text_at(item, "evidence_kind"))
            .collect();
        if text_at(path, "commercial_state") == Some("written_quote")
            && !path_kinds.contains("supplier_quote")
        {
            errors.push(format!("{id}: written quote state requires quote evidence"));
        }
        if text_at(path, "commercial_state") == Some("accepted_order")
            && !path_kinds.contains("accepted_order")
        {
            errors.push(format!(
                "{id}: accepted order state requires order evidence"
            ));
        }
        if text_at(path, "commercial_state") == Some("delivered_inspected") {
            delivered_groups
                .entry(claim_id.to_owned())
                .or_default()
                .insert(
                    text_at(path, "upstream_dependency_group_token")
                        .unwrap_or("")
                        .to_owned(),
                );
        }
    }
    let mut benchmark_ids = BTreeSet::new();
    for (i, b) in benchmarks.iter().enumerate() {
        let Some(id) = text_at(b, "id") else {
            errors.push(format!("benchmarks[{i}].id must be a string"));
            continue;
        };
        if !benchmark_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate benchmark ID {id}"));
        }
        if EXPECTED_BENCHMARKS.get(i).copied() != Some(id) {
            errors.push(format!(
                "benchmark identity/order changed at index {i}: {id}"
            ));
        }
        if !matches!(
            text_at(b, "status"),
            Some("proposal_not_frozen" | "guardrail_contract_only")
        ) {
            errors.push(format!("{id}: benchmark cannot be promoted to result"));
        }
    }
    for expected in EXPECTED_BENCHMARKS {
        if !benchmark_ids.contains(expected) {
            errors.push(format!("missing benchmark {expected}"));
        }
    }
    let policy = graph.get("policy").unwrap_or(&Value::Null);
    if text_at(policy, "identity_rule")
        != Some("exact_model_revision_and_market_required_for_positive_fitment_claim")
        || text_at(policy, "evidence_hierarchy")
            != Some(
                "seller_claim_or_catalogue_listing_never_implies_physical_fit_function_or_safety_qualification",
            )
        || text_at(policy, "unknown_policy")
            != Some("absence_of_evidence_is_unknown_not_incompatible")
        || text_at(policy, "independence_rule")
            != Some("supplier_paths_sharing_upstream_dependency_group_count_as_one_source")
        || text_at(policy, "freshness_rule")
            != Some("expired_or_superseded_claim_cannot_be_current_fitment_evidence")
        || policy.get("winner_claim_allowed").and_then(Value::as_bool) != Some(false)
    {
        errors.push("compatibility policy changed its claim ceiling or evidence rules".to_owned());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(CompatibilityAuditSummary {
        model_count: models.len(),
        part_count: parts.len(),
        claim_count: claims.len(),
        evidence_count: ev_items.len(),
        supplier_path_count: paths.len(),
        functional_claim_examples: claims
            .iter()
            .filter(|c| text_at(c, "claim_status") == Some("functional_compatibility_verified"))
            .count(),
        incompatible_claim_examples: claims
            .iter()
            .filter(|c| text_at(c, "claim_status") == Some("verified_incompatible"))
            .count(),
        unknown_claim_examples: claims
            .iter()
            .filter(|c| text_at(c, "claim_status") == Some("unknown"))
            .count(),
        independent_delivered_groups_in_fixture: delivered_groups.values().map(BTreeSet::len).sum(),
        benchmark_count: benchmarks.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        graph_sha256: format!("{:x}", Sha256::digest(graph_raw.as_bytes())),
    })
}

pub(crate) fn audit_compatibility_texts(
    schema_raw: &str,
    graph_raw: &str,
) -> Result<CompatibilityAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|e| vec![format!("compatibility schema JSON parse failed: {e}")])?;
    let graph: Value = serde_json::from_str(graph_raw)
        .map_err(|e| vec![format!("compatibility graph JSON parse failed: {e}")])?;
    audit_values(&schema, &graph, schema_raw, graph_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/parts_compatibility_evidence_graph_v0_1.schema.json"
    );
    const GRAPH: &str =
        include_str!("../../../fixtures/appliances/parts_compatibility_evidence_graph_v0_1.json");
    fn canonical() -> Value {
        serde_json::from_str(GRAPH).expect("graph fixture parses")
    }
    fn audit(graph: &Value) -> Result<CompatibilityAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema parses");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let graph_raw = serde_json::to_string(graph).expect("graph serializes");
        audit_values(&schema, graph, &schema_raw, &graph_raw)
    }
    #[test]
    fn canonical_graph_qualifies_synthetic_contract() {
        let s = audit_compatibility_texts(SCHEMA, GRAPH).expect("canonical graph");
        assert_eq!(s.model_count, 3);
        assert_eq!(s.part_count, 5);
        assert_eq!(s.claim_count, 9);
        assert_eq!(s.evidence_count, 14);
        assert_eq!(s.supplier_path_count, 3);
        assert_eq!(s.functional_claim_examples, 2);
        assert_eq!(s.incompatible_claim_examples, 1);
        assert_eq!(s.unknown_claim_examples, 1);
        assert_eq!(s.independent_delivered_groups_in_fixture, 1);
        assert_eq!(s.benchmark_count, 6);
    }
    #[test]
    fn duplicate_model_identity_is_rejected() {
        let mut g = canonical();
        g["models"][1]["model_id"] = json!("SYN-MODEL-WASHER-A");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn seller_claim_cannot_be_promoted_to_functional() {
        let mut g = canonical();
        g["compatibility_claims"][1]["claim_status"] = json!("functional_compatibility_verified");
        g["compatibility_claims"][1]["physical_fit_check"] = json!("passed");
        g["compatibility_claims"][1]["functional_check"] = json!("passed");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn manufacturer_listing_is_not_a_fit_test() {
        let mut g = canonical();
        g["compatibility_claims"][0]["claim_status"] = json!("physical_fit_verified");
        g["compatibility_claims"][0]["physical_fit_check"] = json!("passed");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn absence_of_evidence_is_unknown_not_incompatible() {
        let mut g = canonical();
        g["compatibility_claims"][6]["claim_status"] = json!("verified_incompatible");
        g["compatibility_claims"][6]["physical_fit_check"] = json!("failed");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn verified_incompatibility_requires_negative_test() {
        let mut g = canonical();
        g["compatibility_claims"][5]["evidence_ids"] = json!(["SYN-EV-001"]);
        assert!(audit(&g).is_err());
    }
    #[test]
    fn physical_fit_does_not_imply_function() {
        let mut g = canonical();
        g["compatibility_claims"][3]["claim_status"] = json!("functional_compatibility_verified");
        g["compatibility_claims"][3]["functional_check"] = json!("passed");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn expired_claim_cannot_remain_current() {
        let mut g = canonical();
        g["compatibility_claims"][0]["effective_until"] = json!("2025-01-01");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn superseded_claim_requires_valid_successor() {
        let mut g = canonical();
        g["compatibility_claims"][7]["superseded_by_claim_id"] = json!("SYN-COMP-999");
        assert!(audit(&g).is_err());
    }
    #[test]
    fn upstream_group_collapses_apparent_duplicate_sources() {
        let mut g = canonical();
        g["supplier_paths"][1]["upstream_dependency_group_token"] = json!("SYN-UPSTREAM-2");
        let s = audit(&g).expect("well-formed synthetic regrouping");
        assert_eq!(s.independent_delivered_groups_in_fixture, 2);
    }
    #[test]
    fn listing_only_paths_do_not_count_as_delivered_supply() {
        let mut g = canonical();
        for path in g["supplier_paths"].as_array_mut().unwrap() {
            path["commercial_state"] = json!("listing_only");
        }
        let s = audit(&g).expect("listing state is valid but not delivered");
        assert_eq!(s.independent_delivered_groups_in_fixture, 0);
    }
    #[test]
    fn synthetic_evidence_cannot_be_promoted_to_empirical() {
        let mut g = canonical();
        g["evidence"][0]["empirical_eligible"] = json!(true);
        assert!(audit(&g).is_err());
    }
    #[test]
    fn safety_qualification_requires_distinct_evidence() {
        let mut g = canonical();
        g["compatibility_claims"][4]["safety_qualification"] =
            json!("qualified_for_declared_scope");
        assert!(audit(&g).is_err());
    }
}

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
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/decision.rs:

use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:repairability-architecture-decision-engine:0.1.0";
const DECISION_ID: &str = "REPAIRABILITY-DECISION-001";
const EXPECTED_GATES: [&str; 4] = [
    "product_safety",
    "core_wash_function",
    "durability_and_post_repair",
    "market_conformity_route",
];
const EXPECTED_CRITERIA: [(&str, &str); 8] = [
    ("lifecycle_cost_per_useful_service", "minimize"),
    ("restore_time_p90", "minimize"),
    ("first_visit_verified_repair_rate", "maximize"),
    ("energy_per_successful_load", "minimize"),
    ("water_per_successful_load", "minimize"),
    ("discarded_material_per_verified_repair", "minimize"),
    ("added_interface_failure_rate", "minimize"),
    ("target_window_part_fulfilment_rate", "maximize"),
];
const EXPECTED_CANDIDATES: [(&str, &str); 5] = [
    ("W0-RETAIL-CONTROL", "control"),
    ("W1-ACCESSIBLE-CONVENTIONAL", "candidate"),
    ("W2-SERVICE-SPINE", "candidate"),
    ("W3-BEARING-TUB-TRADE", "orthogonal_trade"),
    ("W4-DRIVE-TOPOLOGY", "orthogonal_trade"),
];
const ELIGIBLE_EVIDENCE: [&str; 2] = ["physical_measurement", "independent_replication"];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DecisionAuditSummary {
    pub(crate) candidate_count: usize,
    pub(crate) hard_gate_count: usize,
    pub(crate) criterion_count: usize,
    pub(crate) observation_count: usize,
    pub(crate) eligible_candidate_count: usize,
    pub(crate) pareto_frontier_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) decision_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("decision schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("decision schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "decision schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile decision schema: {error}"))
}

fn criterion_by_id<'a>(decision: &'a Value, id: &str) -> Option<&'a Value> {
    decision
        .get("criteria")?
        .as_array()?
        .iter()
        .find(|criterion| text_at(criterion, "id") == Some(id))
}

fn observation_by_id<'a>(candidate: &'a Value, id: &str) -> Option<&'a Value> {
    candidate
        .get("observations")?
        .as_array()?
        .iter()
        .find(|observation| text_at(observation, "criterion_id") == Some(id))
}

fn gate_status(candidate: &Value, expected_gate: &str) -> &'static str {
    let Some(gates) = candidate.get("hard_gates").and_then(Value::as_array) else {
        return "unknown";
    };
    let mut matches = gates
        .iter()
        .filter(|gate| text_at(gate, "gate_id") == Some(expected_gate));
    let Some(gate) = matches.next() else {
        return "unknown";
    };
    if matches.next().is_some() {
        return "unknown";
    }
    match text_at(gate, "status") {
        Some("pass") => "pass",
        Some("fail") => "fail",
        _ => "unknown",
    }
}

fn all_gates_pass(candidate: &Value) -> bool {
    EXPECTED_GATES
        .iter()
        .all(|gate| gate_status(candidate, gate) == "pass")
}

fn any_gate_fails(candidate: &Value) -> bool {
    EXPECTED_GATES
        .iter()
        .any(|gate| gate_status(candidate, gate) == "fail")
}

fn observation_is_rank_eligible(observation: Option<&Value>, criterion: &Value) -> bool {
    if text_at(criterion, "status") != Some("method_frozen") {
        return false;
    }
    let Some(observation) = observation else {
        return false;
    };
    let evidence_class = text_at(observation, "evidence_class").unwrap_or("");
    if !ELIGIBLE_EVIDENCE.contains(&evidence_class) {
        return false;
    }
    if text_at(observation, "unit") != text_at(criterion, "unit") {
        return false;
    }
    if observation.get("method_frozen").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    if text_at(observation, "evidence_id").is_none_or(str::is_empty)
        || text_at(observation, "method_id").is_none_or(str::is_empty)
    {
        return false;
    }
    let (Some(lower), Some(upper)) = (
        observation.get("lower").and_then(Value::as_f64),
        observation.get("upper").and_then(Value::as_f64),
    ) else {
        return false;
    };
    lower.is_finite() && upper.is_finite() && lower <= upper
}

fn all_criteria_measured(decision: &Value, candidate: &Value) -> bool {
    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return false;
    };
    let Some(observations) = candidate.get("observations").and_then(Value::as_array) else {
        return false;
    };
    let observed_ids: BTreeSet<&str> = observations
        .iter()
        .filter_map(|observation| text_at(observation, "criterion_id"))
        .collect();
    if observed_ids.len() != observations.len() || observed_ids.len() != criteria.len() {
        return false;
    }
    criteria.iter().all(|criterion| {
        let Some(id) = text_at(criterion, "id") else {
            return false;
        };
        observation_is_rank_eligible(observation_by_id(candidate, id), criterion)
    })
}

fn candidate_status(decision: &Value, candidate: &Value) -> &'static str {
    if any_gate_fails(candidate) {
        return "rejected_hard_gate";
    }
    if !all_gates_pass(candidate) {
        return "blocked_unknown_gate";
    }
    if !all_criteria_measured(decision, candidate) {
        return "insufficient_eligible_evidence";
    }
    "eligible_for_robust_pareto"
}

fn robustly_dominates(decision: &Value, a: &Value, b: &Value) -> bool {
    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return false;
    };
    let mut strict_improvement = false;
    for criterion in criteria {
        let Some(id) = text_at(criterion, "id") else {
            return false;
        };
        let (Some(a_obs), Some(b_obs)) = (observation_by_id(a, id), observation_by_id(b, id))
        else {
            return false;
        };
        let (Some(a_lower), Some(a_upper), Some(b_lower), Some(b_upper)) = (
            a_obs.get("lower").and_then(Value::as_f64),
            a_obs.get("upper").and_then(Value::as_f64),
            b_obs.get("lower").and_then(Value::as_f64),
            b_obs.get("upper").and_then(Value::as_f64),
        ) else {
            return false;
        };
        match text_at(criterion, "direction") {
            Some("minimize") => {
                if a_upper > b_lower {
                    return false;
                }
                strict_improvement |= a_upper < b_lower;
            }
            Some("maximize") => {
                if a_lower < b_upper {
                    return false;
                }
                strict_improvement |= a_lower > b_upper;
            }
            _ => return false,
        }
    }
    strict_improvement
}

fn evaluate_decision(decision: &Value) -> Value {
    let candidates = decision
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let preliminary: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate_status(decision, candidate))
        .collect();
    let mut outcomes = Vec::new();
    let mut eligible_count = 0usize;
    let mut frontier_count = 0usize;

    for (index, candidate) in candidates.iter().enumerate() {
        let status = preliminary[index];
        let mut dominated_by = Vec::<String>::new();
        if status == "eligible_for_robust_pareto" {
            eligible_count += 1;
            for (other_index, other) in candidates.iter().enumerate() {
                if index != other_index
                    && preliminary[other_index] == "eligible_for_robust_pareto"
                    && robustly_dominates(decision, other, candidate)
                {
                    if let Some(id) = text_at(other, "id") {
                        dominated_by.push(id.to_owned());
                    }
                }
            }
        }
        let is_frontier = status == "eligible_for_robust_pareto" && dominated_by.is_empty();
        if is_frontier {
            frontier_count += 1;
        }
        outcomes.push(json!({
            "candidate_id": text_at(candidate, "id").unwrap_or("unknown"),
            "status": status,
            "robustly_dominated_by": dominated_by,
            "robust_pareto_frontier": is_frontier
        }));
    }

    json!({
        "policy": "hard_gates_then_interval_robust_pareto",
        "candidate_assessments": outcomes,
        "eligible_candidate_count": eligible_count,
        "robust_pareto_frontier_count": frontier_count,
        "physical_ranking_claim_permitted": eligible_count >= 2,
        "product_winner_declared": false
    })
}

fn audit_values(
    schema: &Value,
    decision: &Value,
    schema_raw: &str,
    decision_raw: &str,
) -> Result<DecisionAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(decision)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(decision, "schema_version") != Some("0.1.0")
        || text_at(decision, "decision_id") != Some(DECISION_ID)
    {
        errors.push(
            "decision identity/version must remain REPAIRABILITY-DECISION-001 v0.1.0".to_owned(),
        );
    }
    if text_at(decision, "status") != Some("decision_profile_under_evaluation")
        || text_at(decision, "claim_ceiling")
            != Some("no_architecture_ranked_without_eligible_physical_evidence")
    {
        errors.push("decision profile cannot claim an architectural winner".to_owned());
    }
    let policy = decision.get("decision_policy").unwrap_or(&Value::Null);
    if text_at(policy, "unknown_policy") != Some("unknown_is_not_pass")
        || text_at(policy, "missing_numeric_policy") != Some("unknown_not_zero")
        || text_at(policy, "scoring_policy")
            != Some("non_compensatory_hard_gates_then_robust_pareto")
        || text_at(policy, "dominance_policy")
            != Some(
                "interval_bounds_must_separate_on_every_criterion_and_strictly_improve_at_least_one",
            )
    {
        errors.push(
            "decision policy must preserve non-compensatory gates, unknown-not-zero and interval-robust dominance"
                .to_owned(),
        );
    }

    let Some(gates) = policy.get("hard_gate_ids").and_then(Value::as_array) else {
        return Err(vec![
            "decision_policy.hard_gate_ids must be an array".to_owned(),
        ]);
    };
    let gate_ids: Vec<&str> = gates.iter().filter_map(Value::as_str).collect();
    if gate_ids.as_slice() != EXPECTED_GATES.as_slice() {
        errors.push(format!(
            "hard gate IDs/order must remain {:?}",
            EXPECTED_GATES
        ));
    }

    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return Err(vec!["criteria must be an array".to_owned()]);
    };
    if criteria.len() != EXPECTED_CRITERIA.len() {
        errors.push(format!(
            "decision must contain {} criteria",
            EXPECTED_CRITERIA.len()
        ));
    }
    let mut criterion_ids = BTreeSet::new();
    for (index, criterion) in criteria.iter().enumerate() {
        let Some(id) = text_at(criterion, "id") else {
            errors.push(format!("criteria[{index}].id must be a string"));
            continue;
        };
        if !criterion_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate criterion ID {id}"));
        }
        if let Some((expected_id, expected_direction)) = EXPECTED_CRITERIA.get(index) {
            if id != *expected_id || text_at(criterion, "direction") != Some(*expected_direction) {
                errors.push(format!(
                    "criteria[{index}] must remain {} with direction {}",
                    expected_id, expected_direction
                ));
            }
        }
        if !matches!(
            text_at(criterion, "status"),
            Some("method_definition_not_yet_frozen" | "method_frozen")
        ) {
            errors.push(format!("{id}: unsupported measurement-method state"));
        }
    }

    let Some(candidates) = decision.get("candidates").and_then(Value::as_array) else {
        return Err(vec!["candidates must be an array".to_owned()]);
    };
    if candidates.len() != EXPECTED_CANDIDATES.len() {
        errors.push(format!(
            "decision must contain {} reviewed washer alternatives",
            EXPECTED_CANDIDATES.len()
        ));
    }
    let mut candidate_ids = BTreeSet::new();
    let mut observation_count = 0usize;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(id) = text_at(candidate, "id") else {
            errors.push(format!("candidates[{index}].id must be a string"));
            continue;
        };
        if !candidate_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate candidate identity {id}"));
        }
        if let Some((expected_id, expected_role)) = EXPECTED_CANDIDATES.get(index) {
            if id != *expected_id || text_at(candidate, "role") != Some(*expected_role) {
                errors.push(format!(
                    "candidate[{index}] must retain reviewed identity/role {expected_id}/{expected_role}"
                ));
            }
        }
        let candidate_gates = candidate.get("hard_gates").and_then(Value::as_array);
        if candidate_gates.is_none_or(|items| items.len() != EXPECTED_GATES.len()) {
            errors.push(format!(
                "{id}: every hard gate must be represented exactly once"
            ));
        } else if let Some(items) = candidate_gates {
            let found: Vec<&str> = items
                .iter()
                .filter_map(|item| text_at(item, "gate_id"))
                .collect();
            if found.iter().copied().collect::<BTreeSet<_>>().len() != found.len()
                || EXPECTED_GATES
                    .iter()
                    .any(|expected| !found.contains(expected))
            {
                errors.push(format!("{id}: hard gate IDs must be unique and complete"));
            }
            for gate in items {
                if text_at(gate, "status") != Some("unknown")
                    && text_at(gate, "evidence_id").is_none_or(str::is_empty)
                {
                    errors.push(format!(
                        "{id}.{}: non-unknown gate state requires an evidence ID",
                        text_at(gate, "gate_id").unwrap_or("unknown_gate")
                    ));
                }
            }
        }

        let observations = candidate.get("observations").and_then(Value::as_array);
        if let Some(observations) = observations {
            observation_count += observations.len();
            let mut seen_observations = BTreeSet::new();
            for observation in observations {
                let Some(metric_id) = text_at(observation, "criterion_id") else {
                    errors.push(format!("{id}: observation criterion_id must be a string"));
                    continue;
                };
                if !seen_observations.insert(metric_id.to_owned()) {
                    errors.push(format!(
                        "{id}: duplicate observation for criterion {metric_id}"
                    ));
                }
                let Some(criterion) = criterion_by_id(decision, metric_id) else {
                    errors.push(format!("{id}: unknown observation criterion {metric_id}"));
                    continue;
                };
                let evidence_class = text_at(observation, "evidence_class").unwrap_or("");
                if ELIGIBLE_EVIDENCE.contains(&evidence_class) {
                    if observation.get("method_frozen").and_then(Value::as_bool) != Some(true) {
                        errors.push(format!(
                            "{id}.{metric_id}: scored evidence requires a frozen method"
                        ));
                    }
                    if text_at(observation, "evidence_id").is_none_or(str::is_empty)
                        || text_at(observation, "method_id").is_none_or(str::is_empty)
                        || text_at(observation, "evidence_id")
                            .unwrap_or("")
                            .starts_with("SYNTHETIC-")
                    {
                        errors.push(format!(
                            "{id}.{metric_id}: eligible evidence requires a real source receipt and method ID, not synthetic test evidence"
                        ));
                    }
                    if text_at(observation, "unit") != text_at(criterion, "unit") {
                        errors.push(format!(
                            "{id}.{metric_id}: unit does not match the criterion"
                        ));
                    }
                    if text_at(criterion, "status") != Some("method_frozen") {
                        errors.push(format!(
                            "{id}.{metric_id}: eligible observations require the criterion method to be frozen"
                        ));
                    }
                    let (Some(lower), Some(upper)) = (
                        observation.get("lower").and_then(Value::as_f64),
                        observation.get("upper").and_then(Value::as_f64),
                    ) else {
                        errors.push(format!(
                            "{id}.{metric_id}: eligible evidence requires finite numeric bounds"
                        ));
                        continue;
                    };
                    if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || lower > upper {
                        errors.push(format!(
                            "{id}.{metric_id}: bounds must be finite, non-negative and satisfy lower <= upper"
                        ));
                    }
                    if matches!(
                        metric_id,
                        "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
                    ) && upper > 1.0
                    {
                        errors.push(format!(
                            "{id}.{metric_id}: proportion bounds must not exceed 1.0"
                        ));
                    }
                }
            }
        }
    }
    for (expected, _) in EXPECTED_CANDIDATES {
        if !candidate_ids.contains(expected) {
            errors.push(format!("required candidate {expected} is missing"));
        }
    }

    let experiment = decision
        .get("next_experiment_policy")
        .unwrap_or(&Value::Null);
    if text_at(experiment, "objective")
        != Some("maximize_decision_relevant_information_per_unit_cost_and_risk")
        || text_at(experiment, "existing_information_value_owner")
            .is_none_or(|value| !value.contains("Symthaea") || !value.contains("EIG/EVSI"))
    {
        errors.push(
            "experiment policy must preserve value-of-information objective and existing Symthaea EIG/EVSI ownership"
                .to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let assessment = evaluate_decision(decision);
    let eligible_candidate_count =
        assessment["eligible_candidate_count"].as_u64().unwrap_or(0) as usize;
    let pareto_frontier_count = assessment["robust_pareto_frontier_count"]
        .as_u64()
        .unwrap_or(0) as usize;
    Ok(DecisionAuditSummary {
        candidate_count: candidates.len(),
        hard_gate_count: EXPECTED_GATES.len(),
        criterion_count: criteria.len(),
        observation_count,
        eligible_candidate_count,
        pareto_frontier_count,
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        decision_sha256: format!("{:x}", Sha256::digest(decision_raw.as_bytes())),
    })
}

pub(crate) fn audit_decision_texts(
    schema_raw: &str,
    decision_raw: &str,
) -> Result<(DecisionAuditSummary, Value), Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("decision schema JSON parse failed: {error}")])?;
    let decision: Value = serde_json::from_str(decision_raw)
        .map_err(|error| vec![format!("decision profile JSON parse failed: {error}")])?;
    let summary = audit_values(&schema, &decision, schema_raw, decision_raw)?;
    let assessment = evaluate_decision(&decision);
    Ok((summary, assessment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/repairability_architecture_decision_v0_1.schema.json"
    );
    const DECISION: &str =
        include_str!("../../../fixtures/appliances/repairability_architecture_decision_v0_1.json");

    fn canonical_decision() -> Value {
        serde_json::from_str(DECISION).expect("decision JSON must parse")
    }

    fn synthetic_scored_decision() -> Value {
        let mut decision = canonical_decision();
        for criterion in decision["criteria"].as_array_mut().expect("criteria") {
            criterion["status"] = json!("method_frozen");
        }
        let criteria = decision["criteria"].as_array().expect("criteria").clone();
        for (candidate_index, candidate) in decision["candidates"]
            .as_array_mut()
            .expect("candidates")
            .iter_mut()
            .enumerate()
        {
            for gate in candidate["hard_gates"].as_array_mut().expect("gates") {
                gate["status"] = json!("pass");
                gate["evidence_id"] = json!("SYNTHETIC-TEST-NOT-PHYSICAL");
            }
            let mut observations = Vec::new();
            for (criterion_index, criterion) in criteria.iter().enumerate() {
                let direction = text_at(criterion, "direction").unwrap_or("minimize");
                let criterion_id = text_at(criterion, "id").unwrap_or("");
                let lower_value = if direction == "minimize" {
                    (candidate_index * 10 + criterion_index) as f64
                } else if matches!(
                    criterion_id,
                    "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
                ) {
                    0.95 - (candidate_index as f64 * 0.1)
                } else {
                    100.0 - (candidate_index * 10 + criterion_index) as f64
                };
                observations.push(json!({
                    "criterion_id": text_at(criterion, "id").unwrap_or("unknown"),
                    "lower": lower_value,
                    "upper": lower_value,
                    "unit": text_at(criterion, "unit").unwrap_or("unknown"),
                    "evidence_class": "physical_measurement",
                    "evidence_id": "SYNTHETIC-TEST-NOT-PHYSICAL",
                    "method_id": "SYNTHETIC-TEST-METHOD",
                    "method_frozen": true,
                    "uncertainty_note": "Synthetic algorithm test only; no appliance data."
                }));
            }
            candidate["observations"] = json!(observations);
        }
        decision
    }

    #[test]
    fn canonical_profile_is_valid_but_ranks_no_architecture() {
        let (summary, assessment) = audit_decision_texts(SCHEMA, DECISION)
            .expect("canonical decision profile must pass contract audit");
        assert_eq!(summary.candidate_count, 5);
        assert_eq!(summary.hard_gate_count, 4);
        assert_eq!(summary.criterion_count, 8);
        assert_eq!(summary.observation_count, 0);
        assert_eq!(summary.eligible_candidate_count, 0);
        assert_eq!(summary.pareto_frontier_count, 0);
        assert!(
            !assessment["physical_ranking_claim_permitted"]
                .as_bool()
                .unwrap()
        );
        for candidate in assessment["candidate_assessments"].as_array().unwrap() {
            assert_eq!(candidate["status"], "blocked_unknown_gate");
        }
    }

    #[test]
    fn failed_safety_gate_rejects_candidate_regardless_of_cost() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["hard_gates"][0]["status"] = json!("fail");
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "rejected_hard_gate"
        );
    }

    #[test]
    fn unknown_hard_gate_blocks_candidate() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["hard_gates"][0]["status"] = json!("unknown");
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "blocked_unknown_gate"
        );
    }

    #[test]
    fn missing_criterion_prevents_comparison() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["observations"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "insufficient_eligible_evidence"
        );
    }

    #[test]
    fn desk_estimate_cannot_qualify_as_physical_measurement() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["observations"][0]["evidence_class"] = json!("desk_estimate");
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "insufficient_eligible_evidence"
        );
    }

    #[test]
    fn invalid_bounds_prevent_comparison() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["observations"][0]["lower"] = json!(2.0);
        decision["candidates"][0]["observations"][0]["upper"] = json!(1.0);
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "insufficient_eligible_evidence"
        );
    }

    #[test]
    fn interval_robust_dominance_requires_separation_on_every_criterion() {
        let decision = synthetic_scored_decision();
        assert!(robustly_dominates(
            &decision,
            &decision["candidates"][0],
            &decision["candidates"][1]
        ));
    }

    #[test]
    fn overlapping_bounds_do_not_create_a_false_winner() {
        let mut decision = synthetic_scored_decision();
        for observation in decision["candidates"][1]["observations"]
            .as_array_mut()
            .unwrap()
        {
            observation["lower"] = json!(0.0);
            observation["upper"] = json!(5.0);
            if matches!(
                text_at(observation, "criterion_id"),
                Some("first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate")
            ) {
                observation["upper"] = json!(1.0);
            }
        }
        assert!(!robustly_dominates(
            &decision,
            &decision["candidates"][0],
            &decision["candidates"][1]
        ));
    }

    #[test]
    fn audit_rejects_synthetic_ids_promoted_to_real_evidence() {
        let decision = synthetic_scored_decision();
        let raw = serde_json::to_string(&decision).unwrap();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let schema_raw = serde_json::to_string(&schema).unwrap();
        assert!(audit_values(&schema, &decision, &schema_raw, &raw).is_err());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/experiment.rs:

use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:appliance-repairability-factorial-plan:0.1.0";
const DESIGN_ID: &str = "APPLIANCE-DOE-001";
const EXPECTED_CELLS: [&str; 8] = [
    "DOE-000", "DOE-001", "DOE-010", "DOE-011", "DOE-100", "DOE-101", "DOE-110", "DOE-111",
];
const EXPECTED_BENCHMARKS: [&str; 6] = [
    "DOE-B-001",
    "DOE-B-002",
    "DOE-B-003",
    "DOE-B-004",
    "DOE-B-005",
    "DOE-B-006",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ExperimentAuditSummary {
    pub(crate) cell_count: usize,
    pub(crate) factor_count: usize,
    pub(crate) planned_trials_per_cell: usize,
    pub(crate) planned_trial_count: usize,
    pub(crate) observed_trial_count: usize,
    pub(crate) benchmark_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) plan_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("factorial plan schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("factorial plan schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "factorial plan schema does not validate against the Draft 2020-12 meta-schema"
                .to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile factorial plan schema: {error}"))
}

fn expected_level(levels: &Value, key: &str, bit: char, zero: &str, one: &str) -> bool {
    let expected = if bit == '0' { zero } else { one };
    text_at(levels, key) == Some(expected)
}

fn audit_values(
    schema: &Value,
    plan: &Value,
    schema_raw: &str,
    plan_raw: &str,
) -> Result<ExperimentAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(plan)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(plan, "schema_version") != Some("0.1.0")
        || text_at(plan, "design_id") != Some(DESIGN_ID)
        || text_at(plan, "status") != Some("plan_only_not_frozen")
        || text_at(plan, "claim_ceiling")
            != Some("synthetic_plan_only_no_physical_result_no_winner")
        || plan.get("physical_trials_started").and_then(Value::as_bool) != Some(false)
        || plan.get("observed_trial_count").and_then(Value::as_u64) != Some(0)
        || plan.get("winner_declared").and_then(Value::as_bool) != Some(false)
        || text_at(plan, "allowed_claim_scope") != Some("service_task_mockup_hypotheses_only")
    {
        errors.push(
            "factorial plan must remain not-run, not-frozen, synthetic-only and winner-free"
                .to_owned(),
        );
    }

    let Some(design) = plan.get("factorial_design") else {
        return Err(vec!["factorial_design object is required".to_owned()]);
    };
    if text_at(design, "design_type") != Some("full_2_level_factorial_2_pow_3")
        || design.get("factor_count").and_then(Value::as_u64) != Some(3)
        || text_at(design, "external_control_id") != Some("W0-RETAIL-CONTROL")
        || design.get("frozen_before_outcome").and_then(Value::as_bool) != Some(false)
        || design
            .get("planned_technician_count")
            .and_then(Value::as_u64)
            != Some(3)
        || design
            .get("planned_trials_per_cell")
            .and_then(Value::as_u64)
            != Some(12)
        || text_at(design, "denominator_policy")
            != Some(
                "every_started_task_attempt_including_failed_abandoned_and_safety_stop_stays_in_attempt_denominator",
            )
    {
        errors.push(
            "factorial design, pilot floor, external control or denominator policy changed"
                .to_owned(),
        );
    }
    let expected_interactions = ["A:B", "A:C", "B:C", "A:B:C"];
    let actual_interactions: Vec<&str> = design
        .get("preregistered_interactions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if actual_interactions != expected_interactions {
        errors.push(
            "all three two-way and the three-way interaction terms must remain preregistered"
                .to_owned(),
        );
    }
    let task_families: Vec<&str> = design
        .get("task_families")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if task_families.len() != 4 {
        errors.push("initial factorial pilot must retain four declared task families".to_owned());
    }

    let Some(cells) = plan.get("cells").and_then(Value::as_array) else {
        return Err(vec!["factorial cells must be an array".to_owned()]);
    };
    if cells.len() != EXPECTED_CELLS.len() {
        errors.push(format!(
            "full factorial must have exactly {} cells",
            EXPECTED_CELLS.len()
        ));
    }
    let mut cell_ids = BTreeSet::new();
    let mut combinations = BTreeSet::new();
    for (index, cell) in cells.iter().enumerate() {
        let Some(cell_id) = text_at(cell, "cell_id") else {
            errors.push(format!("cells[{index}].cell_id must be a string"));
            continue;
        };
        if !cell_ids.insert(cell_id.to_owned()) {
            errors.push(format!("duplicate factorial cell {cell_id}"));
        }
        if EXPECTED_CELLS.get(index).copied() != Some(cell_id) {
            errors.push(format!(
                "factorial cell order/identity changed at index {index}: {cell_id}"
            ));
        }
        if text_at(cell, "status") != Some("planned_not_run")
            || cell.get("observed_trial_count").and_then(Value::as_u64) != Some(0)
        {
            errors.push(format!(
                "{cell_id}: unrun design cell cannot contain observations"
            ));
        }
        let Some(bits) = cell_id.strip_prefix("DOE-") else {
            errors.push(format!(
                "{cell_id}: cell ID must encode three binary factor levels"
            ));
            continue;
        };
        if bits.len() != 3 || !bits.chars().all(|bit| bit == '0' || bit == '1') {
            errors.push(format!(
                "{cell_id}: cell ID must encode three binary factor levels"
            ));
            continue;
        }
        let Some(levels) = cell.get("factor_levels") else {
            errors.push(format!("{cell_id}: factor_levels are required"));
            continue;
        };
        let chars: Vec<char> = bits.chars().collect();
        let valid = expected_level(
            levels,
            "access_architecture",
            chars[0],
            "conventional_access",
            "service_spine",
        ) && expected_level(
            levels,
            "repair_unit_granularity",
            chars[1],
            "assembly_level",
            "smallest_justified_unit",
        ) && expected_level(
            levels,
            "diagnostic_support",
            chars[2],
            "baseline_documentation",
            "evidence_guided_test_before_replace",
        );
        if !valid {
            errors.push(format!(
                "{cell_id}: factor values do not match the binary cell identity"
            ));
        }
        let combination = format!(
            "{}|{}|{}",
            text_at(levels, "access_architecture").unwrap_or(""),
            text_at(levels, "repair_unit_granularity").unwrap_or(""),
            text_at(levels, "diagnostic_support").unwrap_or("")
        );
        if !combinations.insert(combination) {
            errors.push(format!("{cell_id}: duplicate factor combination"));
        }
        let hypothesis_count = cell
            .get("hypotheses")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let receipt_count = cell
            .get("required_receipt_types")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if hypothesis_count < 2 || receipt_count < 3 {
            errors.push(format!(
                "{cell_id}: hypotheses or evidence receipt requirements are incomplete"
            ));
        }
    }
    for expected in EXPECTED_CELLS {
        if !cell_ids.contains(expected) {
            errors.push(format!("required factorial cell {expected} is missing"));
        }
    }
    if combinations.len() != 8 {
        errors.push("all eight unique factor combinations must appear exactly once".to_owned());
    }

    let per_cell = design
        .get("planned_trials_per_cell")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let planned_total = design
        .get("planned_trial_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    if per_cell.saturating_mul(cells.len()) != planned_total {
        errors.push("planned trial total must equal trials per cell times cell count".to_owned());
    }
    if planned_total != 96 {
        errors.push(
            "v0.1 feasibility pilot plan must retain the declared 96-attempt floor".to_owned(),
        );
    }

    let Some(benchmarks) = plan.get("benchmarks").and_then(Value::as_array) else {
        return Err(vec!["benchmarks must be an array".to_owned()]);
    };
    if benchmarks.len() != EXPECTED_BENCHMARKS.len() {
        errors.push(format!(
            "factorial plan must retain {} benchmark guards",
            EXPECTED_BENCHMARKS.len()
        ));
    }
    let mut benchmark_ids = BTreeSet::new();
    for (index, benchmark) in benchmarks.iter().enumerate() {
        let Some(id) = text_at(benchmark, "id") else {
            errors.push(format!("benchmarks[{index}].id must be a string"));
            continue;
        };
        if !benchmark_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate benchmark ID {id}"));
        }
        if EXPECTED_BENCHMARKS.get(index).copied() != Some(id) {
            errors.push(format!("benchmark identity changed at index {index}: {id}"));
        }
        if text_at(benchmark, "status").is_none_or(|status| {
            !matches!(status, "proposal_not_frozen" | "guardrail_contract_only")
        }) {
            errors.push(format!(
                "{id}: benchmark cannot be promoted to an observed result"
            ));
        }
    }
    for expected in EXPECTED_BENCHMARKS {
        if !benchmark_ids.contains(expected) {
            errors.push(format!("required benchmark {expected} is missing"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(ExperimentAuditSummary {
        cell_count: cells.len(),
        factor_count: design
            .get("factor_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        planned_trials_per_cell: per_cell,
        planned_trial_count: planned_total,
        observed_trial_count: plan
            .get("observed_trial_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        benchmark_count: benchmarks.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        plan_sha256: format!("{:x}", Sha256::digest(plan_raw.as_bytes())),
    })
}

pub(crate) fn audit_experiment_texts(
    schema_raw: &str,
    plan_raw: &str,
) -> Result<ExperimentAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("factorial schema JSON parse failed: {error}")])?;
    let plan: Value = serde_json::from_str(plan_raw)
        .map_err(|error| vec![format!("factorial plan JSON parse failed: {error}")])?;
    audit_values(&schema, &plan, schema_raw, plan_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/washer_repairability_factorial_plan_v0_1.schema.json"
    );
    const PLAN: &str =
        include_str!("../../../fixtures/appliances/washer_repairability_factorial_plan_v0_1.json");

    fn canonical_plan() -> Value {
        serde_json::from_str(PLAN).expect("factorial plan fixture must parse")
    }

    fn audit_mutation(plan: &Value) -> Result<ExperimentAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("factorial schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let plan_raw = serde_json::to_string(plan).expect("plan serializes");
        audit_values(&schema, plan, &schema_raw, &plan_raw)
    }

    #[test]
    fn canonical_factorial_has_eight_cells_and_ninety_six_planned_trials() {
        let summary =
            audit_experiment_texts(SCHEMA, PLAN).expect("canonical factorial plan qualifies");
        assert_eq!(summary.cell_count, 8);
        assert_eq!(summary.factor_count, 3);
        assert_eq!(summary.planned_trials_per_cell, 12);
        assert_eq!(summary.planned_trial_count, 96);
        assert_eq!(summary.observed_trial_count, 0);
        assert_eq!(summary.benchmark_count, 6);
    }

    #[test]
    fn missing_factorial_cell_is_rejected() {
        let mut plan = canonical_plan();
        plan["cells"].as_array_mut().unwrap().remove(7);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn duplicate_cell_identity_is_rejected() {
        let mut plan = canonical_plan();
        plan["cells"][7]["cell_id"] = json!("DOE-110");
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn factor_values_must_match_binary_cell_identity() {
        let mut plan = canonical_plan();
        plan["cells"][7]["factor_levels"]["diagnostic_support"] = json!("baseline_documentation");
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn unrun_plan_cannot_report_observed_trials() {
        let mut plan = canonical_plan();
        plan["cells"][0]["observed_trial_count"] = json!(1);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn planned_trial_total_must_match_cell_repetitions() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["planned_trial_count"] = json!(88);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn winner_cannot_be_declared_from_a_plan() {
        let mut plan = canonical_plan();
        plan["winner_declared"] = json!(true);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn protocol_cannot_be_claimed_frozen_before_it_is_reviewed() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["frozen_before_outcome"] = json!(true);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn all_interaction_terms_are_preregistered_in_the_plan() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["preregistered_interactions"] = json!(["A:B", "A:C", "B:C"]);
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn external_retail_control_is_not_silently_made_a_factorial_cell() {
        let mut plan = canonical_plan();
        plan["factorial_design"]["external_control_id"] = json!("DOE-000");
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn duplicated_factor_combination_is_rejected_even_with_unique_ids() {
        let mut plan = canonical_plan();
        let levels = plan["cells"][6]["factor_levels"].clone();
        plan["cells"][7]["factor_levels"] = levels;
        assert!(audit_mutation(&plan).is_err());
    }

    #[test]
    fn proposed_benchmark_cannot_be_promoted_to_result() {
        let mut plan = canonical_plan();
        plan["benchmarks"][0]["status"] = json!("validated_winner");
        assert!(audit_mutation(&plan).is_err());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/faults.rs:

use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:washer-fault-to-repair-curriculum:0.1.0";
const CURRICULUM_ID: &str = "WASHER-FAULT-CURRICULUM-001";
const EXPECTED_TASKS: [(&str, &str); 8] = [
    ("WFT-01", "U0"),
    ("WFT-02", "P"),
    ("WFT-03", "P"),
    ("WFT-04", "P"),
    ("WFT-05", "P"),
    ("WFT-06", "P"),
    ("WFT-07", "P"),
    ("WFT-08", "P"),
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FaultCurriculumSummary {
    pub(crate) task_count: usize,
    pub(crate) user_tier_count: usize,
    pub(crate) qualified_tier_count: usize,
    pub(crate) reference_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) curriculum_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("fault curriculum schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("fault curriculum schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "fault curriculum schema does not validate against the Draft 2020-12 meta-schema"
                .to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile fault curriculum schema: {error}"))
}

fn audit_values(
    schema: &Value,
    curriculum: &Value,
    schema_raw: &str,
    curriculum_raw: &str,
) -> Result<FaultCurriculumSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(curriculum)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(curriculum, "schema_version") != Some("0.1.0")
        || text_at(curriculum, "curriculum_id") != Some(CURRICULUM_ID)
    {
        errors.push("fault curriculum identity/version changed".to_owned());
    }
    if text_at(curriculum, "status") != Some("benchmark_design_only")
        || text_at(curriculum, "claim_ceiling")
            != Some("no_local_prevalence_no_physical_repair_or_superiority_claim")
    {
        errors.push(
            "fault curriculum cannot claim prevalence, physical repair results or superiority"
                .to_owned(),
        );
    }
    if text_at(curriculum, "selection_basis") != Some("coverage_balanced_not_prevalence_weighted") {
        errors.push(
            "fault curriculum must remain coverage-balanced, not prevalence-weighted".to_owned(),
        );
    }

    let Some(references) = curriculum.get("references").and_then(Value::as_array) else {
        return Err(vec![
            "fault curriculum references must be an array".to_owned(),
        ]);
    };
    let mut reference_ids = BTreeSet::new();
    for reference in references {
        if let Some(id) = text_at(reference, "id") {
            if !reference_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate fault curriculum reference {id}"));
            }
        }
    }
    let study_scope = references
        .iter()
        .find(|reference| text_at(reference, "id") == Some("REPAIR-DIAG-11K"));
    let scope_note = study_scope
        .and_then(|reference| text_at(reference, "scope_note"))
        .unwrap_or("")
        .to_ascii_lowercase();
    if !scope_note.contains("not a target-market prevalence")
        || !scope_note.contains("calibrated washer prior")
    {
        errors.push(
            "11k diagnosis source must retain target-market prevalence/calibration caveat"
                .to_owned(),
        );
    }

    let Some(tasks) = curriculum.get("tasks").and_then(Value::as_array) else {
        return Err(vec!["fault curriculum tasks must be an array".to_owned()]);
    };
    if tasks.len() != EXPECTED_TASKS.len() {
        errors.push(format!(
            "fault curriculum must contain {} task families",
            EXPECTED_TASKS.len()
        ));
    }
    let mut task_ids = BTreeSet::new();
    let mut user_tier_count = 0usize;
    let mut qualified_tier_count = 0usize;
    for (index, task) in tasks.iter().enumerate() {
        let Some(id) = text_at(task, "id") else {
            errors.push(format!("tasks[{index}].id must be a string"));
            continue;
        };
        if !task_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate fault task ID {id}"));
        }
        if let Some((expected_id, expected_tier)) = EXPECTED_TASKS.get(index) {
            if id != *expected_id || text_at(task, "service_tier") != Some(*expected_tier) {
                errors.push(format!(
                    "task[{index}] must retain reviewed identity/tier {expected_id}/{expected_tier}"
                ));
            }
        }
        if text_at(task, "status") != Some("curriculum_design_only") {
            errors.push(format!("{id}: task cannot claim physical validation"));
        }
        match text_at(task, "service_tier") {
            Some("U0") => user_tier_count += 1,
            Some("P") => qualified_tier_count += 1,
            other => errors.push(format!("{id}: unsupported role boundary {other:?}")),
        }

        let task_refs: BTreeSet<&str> = task
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if task_refs.is_empty()
            || task_refs
                .iter()
                .any(|reference| !reference_ids.contains(*reference))
        {
            errors.push(format!(
                "{id}: source_ids must reference known curriculum sources"
            ));
        }
        for (field, minimum) in [
            ("competing_hypotheses", 2usize),
            ("safe_discriminating_checks", 2usize),
            ("repair_scope_options", 2usize),
            ("post_repair_verification", 2usize),
        ] {
            if task
                .get(field)
                .and_then(Value::as_array)
                .is_none_or(|items| items.len() < minimum)
            {
                errors.push(format!(
                    "{id}: {field} must retain at least {minimum} entries"
                ));
            }
        }
        let boundary = text_at(task, "hazard_boundary")
            .unwrap_or("")
            .to_ascii_lowercase();
        if !boundary.contains("qualified")
            && (boundary.contains("mains")
                || boundary.contains("heater")
                || boundary.contains("rotating"))
        {
            errors.push(format!(
                "{id}: hazardous task boundary must identify qualified-service controls"
            ));
        }
        if text_at(task, "service_tier") == Some("U0") {
            if !boundary.contains("user")
                || boundary.contains("may open internal mains")
                || boundary.contains("may perform refrigerant charging")
            {
                errors.push(
                    "U0 task boundary must remain limited to intended safe user maintenance"
                        .to_owned(),
                );
            }
        }
    }
    for (expected, _) in EXPECTED_TASKS {
        if !task_ids.contains(expected) {
            errors.push(format!("required fault task {expected} is missing"));
        }
    }
    if user_tier_count != 1 || qualified_tier_count != 7 {
        errors.push(format!(
            "reviewed task tier census must remain U0=1/P=7; got U0={user_tier_count}/P={qualified_tier_count}"
        ));
    }

    let policy = curriculum.get("measurement_policy").unwrap_or(&Value::Null);
    if text_at(policy, "aggregate_policy")
        != Some("per_class_plus_macro_average_no_easy_task_masking")
        || text_at(policy, "prevalence_policy")
            != Some("no_prevalence_weights_without_validated_target_market_denominator")
        || text_at(policy, "unknown_policy") != Some("unknown_not_zero")
        || text_at(policy, "safety_gate") != Some("non_compensatory")
        || policy
            .get("success_receipt_required")
            .and_then(Value::as_bool)
            != Some(true)
    {
        errors.push("fault measurement policy must preserve class coverage, unknown-not-zero, hard safety gates and repair receipts".to_owned());
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(FaultCurriculumSummary {
        task_count: tasks.len(),
        user_tier_count,
        qualified_tier_count,
        reference_count: references.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        curriculum_sha256: format!("{:x}", Sha256::digest(curriculum_raw.as_bytes())),
    })
}

pub(crate) fn audit_fault_curriculum_texts(
    schema_raw: &str,
    curriculum_raw: &str,
) -> Result<FaultCurriculumSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("fault curriculum schema parse failed: {error}")])?;
    let curriculum: Value = serde_json::from_str(curriculum_raw)
        .map_err(|error| vec![format!("fault curriculum parse failed: {error}")])?;
    audit_values(&schema, &curriculum, schema_raw, curriculum_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/washer_fault_to_repair_curriculum_v0_1.schema.json"
    );
    const CURRICULUM: &str =
        include_str!("../../../fixtures/appliances/washer_fault_to_repair_curriculum_v0_1.json");

    fn canonical_curriculum() -> Value {
        serde_json::from_str(CURRICULUM).expect("fault curriculum must parse")
    }

    fn audit_mutation(curriculum: &Value) -> Result<FaultCurriculumSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let curriculum_raw = serde_json::to_string(curriculum).expect("curriculum serializes");
        audit_values(&schema, curriculum, &schema_raw, &curriculum_raw)
    }

    #[test]
    fn canonical_fault_curriculum_has_full_coverage_and_no_prevalence_weighting() {
        let summary = audit_fault_curriculum_texts(SCHEMA, CURRICULUM)
            .expect("canonical fault curriculum must qualify as design-only");
        assert_eq!(summary.task_count, 8);
        assert_eq!(summary.user_tier_count, 1);
        assert_eq!(summary.qualified_tier_count, 7);
        assert_eq!(summary.reference_count, 3);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.curriculum_sha256.len(), 64);
    }

    #[test]
    fn duplicate_fault_family_id_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][1]["id"] = json!("WFT-01");
        let errors = audit_mutation(&curriculum).expect_err("duplicate task ID must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn removing_post_repair_verification_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][7]["post_repair_verification"] = json!([]);
        let errors = audit_mutation(&curriculum).expect_err("post-repair proof is required");
        assert!(!errors.is_empty());
    }

    #[test]
    fn literature_cannot_be_promoted_to_south_african_prevalence() {
        let mut curriculum = canonical_curriculum();
        curriculum["references"][0]["scope_note"] =
            json!("This source is a calibrated South African washer prevalence prior.");
        let errors = audit_mutation(&curriculum).expect_err("source caveat mutation must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn a_qualified_internal_task_cannot_be_downgraded_to_user_maintenance() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][6]["service_tier"] = json!("U0");
        let errors =
            audit_mutation(&curriculum).expect_err("drive internals must remain qualified");
        assert!(!errors.is_empty());
    }

    #[test]
    fn a_weighted_fault_prevalence_score_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["selection_basis"] = json!("frequency_weighted_by_repair_center_incidence");
        let errors = audit_mutation(&curriculum).expect_err("prevalence weighting must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn missing_competing_diagnostic_hypothesis_is_rejected() {
        let mut curriculum = canonical_curriculum();
        curriculum["tasks"][7]["competing_hypotheses"] = json!(["bearing wear"]);
        let errors =
            audit_mutation(&curriculum).expect_err("single hypothesis is not a diagnostic task");
        assert!(!errors.is_empty());
    }

    #[test]
    fn no_task_can_remove_the_non_compensatory_safety_gate() {
        let mut curriculum = canonical_curriculum();
        curriculum["measurement_policy"]["safety_gate"] = json!("weighted_cost_penalty");
        let errors = audit_mutation(&curriculum).expect_err("safety gate cannot be weighted away");
        assert!(!errors.is_empty());
    }

    #[test]
    fn success_receipt_cannot_be_optional() {
        let mut curriculum = canonical_curriculum();
        curriculum["measurement_policy"]["success_receipt_required"] = json!(false);
        let errors = audit_mutation(&curriculum).expect_err("success receipt is mandatory");
        assert!(!errors.is_empty());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/main.rs:

use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

mod compatibility;
mod decision;
mod experiment;
mod faults;
mod morphology;
mod portfolio;
mod restoration;
mod sourcing;
mod stakeholder;

const SCHEMA_ID: &str = "urn:luminous-dynamics:household-appliance-benchmark-corpus:0.1.0";
const EXPECTED_CASE_IDS: [&str; 30] = [
    "HAP-001", "HAP-002", "HAP-003", "HAP-004", "HAP-005", "HAP-006", "HAP-007", "HAP-008",
    "HAP-009", "HAP-010", "HAP-011", "HAP-012", "HAP-013", "HAP-014", "HAP-015", "HAP-016",
    "HAP-017", "HAP-018", "HAP-019", "HAP-020", "HAP-021", "HAP-022", "HAP-023", "HAP-024",
    "HAP-025", "HAP-026", "HAP-027", "HAP-028", "HAP-029", "HAP-030",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    fixture_count: usize,
    positive_controls: usize,
    negative_controls: usize,
    indeterminate_controls: usize,
    schema_sha256: String,
    corpus_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn default_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json")
}

fn default_corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/household_appliance/benchmark_scenarios_v0_1.json")
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err("schema does not validate against the Draft 2020-12 meta-schema".to_owned());
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile schema: {error}"))
}

fn case_by_id<'a>(cases: &'a [Value], id: &str) -> Option<&'a Value> {
    cases
        .iter()
        .find(|case| text_at(case, "case_id") == Some(id))
}

fn input_by_id<'a>(cases: &'a [Value], id: &str) -> Option<&'a Value> {
    case_by_id(cases, id).and_then(|case| case.get("input"))
}

fn require_disposition(cases: &[Value], id: &str, expected: &str, errors: &mut Errors) {
    let actual = case_by_id(cases, id).and_then(|case| text_at(case, "expected_disposition"));
    if actual != Some(expected) {
        errors.push(format!(
            "{id}: expected_disposition must be {expected}, got {actual:?}"
        ));
    }
}

fn require_case_text(cases: &[Value], id: &str, field: &str, expected: &str, errors: &mut Errors) {
    let actual = case_by_id(cases, id).and_then(|case| text_at(case, field));
    if actual != Some(expected) {
        errors.push(format!("{id}.{field}: expected {expected}, got {actual:?}"));
    }
}

fn require_input_text(cases: &[Value], id: &str, field: &str, expected: &str, errors: &mut Errors) {
    let actual = input_by_id(cases, id).and_then(|input| text_at(input, field));
    if actual != Some(expected) {
        errors.push(format!(
            "{id}.input.{field}: expected {expected}, got {actual:?}"
        ));
    }
}

fn require_input_bool(cases: &[Value], id: &str, field: &str, expected: bool, errors: &mut Errors) {
    let actual = input_by_id(cases, id)
        .and_then(|input| input.get(field))
        .and_then(Value::as_bool);
    if actual != Some(expected) {
        errors.push(format!(
            "{id}.input.{field}: expected {expected}, got {actual:?}"
        ));
    }
}

fn require_input_number(
    cases: &[Value],
    id: &str,
    field: &str,
    expected: f64,
    errors: &mut Errors,
) {
    let actual = input_by_id(cases, id)
        .and_then(|input| input.get(field))
        .and_then(Value::as_f64);
    if actual.is_none_or(|value| (value - expected).abs() > f64::EPSILON) {
        errors.push(format!(
            "{id}.input.{field}: expected {expected}, got {actual:?}"
        ));
    }
}

fn require_input_number_pair_different(
    cases: &[Value],
    id: &str,
    first: &str,
    second: &str,
    errors: &mut Errors,
) {
    let input = input_by_id(cases, id);
    let a = input
        .and_then(|value| value.get(first))
        .and_then(Value::as_f64);
    let b = input
        .and_then(|value| value.get(second))
        .and_then(Value::as_f64);
    if a.is_none() || b.is_none() || a == b {
        errors.push(format!(
            "{id}.input: {first} and {second} must both exist and differ for this negative control"
        ));
    }
}

fn require_input_number_is_zero(cases: &[Value], id: &str, field: &str, errors: &mut Errors) {
    require_input_number(cases, id, field, 0.0, errors);
}

fn require_case_has_assertion(cases: &[Value], id: &str, needle: &str, errors: &mut Errors) {
    let found = case_by_id(cases, id)
        .and_then(|case| case.get("assertions"))
        .and_then(Value::as_array)
        .is_some_and(|assertions| {
            assertions
                .iter()
                .filter_map(Value::as_str)
                .any(|assertion| assertion.contains(needle))
        });
    if !found {
        errors.push(format!(
            "{id}.assertions: missing required invariant text containing {needle:?}"
        ));
    }
}

fn check_critical_controls(cases: &[Value]) -> Errors {
    let mut errors = Vec::new();

    // Work-order status cannot replace evidence that the appliance's function was restored.
    require_disposition(cases, "HAP-005", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-005",
        "work_order_status",
        "completed",
        &mut errors,
    );
    require_input_text(cases, "HAP-005", "post_repair_test", "missing", &mut errors);
    require_case_has_assertion(
        cases,
        "HAP-005",
        "Work-order completion cannot establish restored capability",
        &mut errors,
    );

    // A single nominal sample cannot establish a recurrence-free outcome.
    require_disposition(cases, "HAP-007", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-007",
        "recurrence_window",
        "missing",
        &mut errors,
    );
    require_input_bool(cases, "HAP-007", "claim_recurrence_free", true, &mut errors);

    // Performance comparisons need matched delivered function.
    require_disposition(cases, "HAP-009", "reject_claim", &mut errors);
    require_input_number_pair_different(
        cases,
        "HAP-009",
        "candidate_airflow_m3_h",
        "baseline_airflow_m3_h",
        &mut errors,
    );
    require_input_bool(
        cases,
        "HAP-009",
        "efficiency_superiority_claim",
        true,
        &mut errors,
    );

    // Unknown waiting time cannot be interpreted as zero.
    require_disposition(cases, "HAP-011", "reject_claim", &mut errors);
    require_input_text(cases, "HAP-011", "parts_wait_days", "unknown", &mut errors);
    require_input_number_is_zero(cases, "HAP-011", "cost_model_substituted_days", &mut errors);

    // Hardware protection takes precedence over model confidence.
    require_disposition(cases, "HAP-014", "reject_claim", &mut errors);
    require_input_bool(
        cases,
        "HAP-014",
        "critical_interlock_triggered",
        true,
        &mut errors,
    );
    require_input_text(
        cases,
        "HAP-014",
        "model_output",
        "safe_to_operate",
        &mut errors,
    );

    // Qualification scope cannot broaden from one unit to a batch.
    require_disposition(cases, "HAP-016", "reject_claim", &mut errors);
    require_input_number(cases, "HAP-016", "qualified_units", 1.0, &mut errors);
    require_input_bool(
        cases,
        "HAP-016",
        "batch_qualification_claim",
        true,
        &mut errors,
    );

    // Stress screening does not automatically prove a decade of field life.
    require_disposition(cases, "HAP-018", "reject_claim", &mut errors);
    require_input_number(cases, "HAP-018", "stress_screen_hours", 1000.0, &mut errors);
    require_input_bool(
        cases,
        "HAP-018",
        "validated_acceleration_model",
        false,
        &mut errors,
    );

    // Unknown material burden cannot be coerced to zero.
    require_disposition(cases, "HAP-019", "reject_claim", &mut errors);
    require_input_text(cases, "HAP-019", "material_mass", "unknown", &mut errors);
    require_input_number_is_zero(cases, "HAP-019", "modelled_material_mass", &mut errors);

    // Essential operation must not require a cloud or network service.
    require_disposition(cases, "HAP-021", "reject_claim", &mut errors);
    require_input_bool(
        cases,
        "HAP-021",
        "essential_function_requires_network",
        true,
        &mut errors,
    );
    require_input_text(
        cases,
        "HAP-021",
        "offline_function_test",
        "fail",
        &mut errors,
    );

    // Event data cannot be represented as unbiased population probability.
    require_disposition(cases, "HAP-023", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-023",
        "claim_scope",
        "general_population_probability",
        &mut errors,
    );

    // Synthetic results cannot be promoted to physical field evidence.
    require_disposition(cases, "HAP-027", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-027",
        "evidence_class",
        "synthetic_simulation",
        &mut errors,
    );
    require_input_text(
        cases,
        "HAP-027",
        "claimed_evidence_class",
        "physical_field",
        &mut errors,
    );

    // Pilot-size repair trials do not justify an unplanned confirmatory quantile claim.
    require_disposition(cases, "HAP-028", "reject_claim", &mut errors);
    require_input_bool(
        cases,
        "HAP-028",
        "analysis_plan_preregistered",
        false,
        &mut errors,
    );

    // A diagnosis hypothesis is not an observed cause.
    require_disposition(cases, "HAP-013", "reject_claim", &mut errors);
    require_input_text(
        cases,
        "HAP-013",
        "source_class",
        "observed_cause",
        &mut errors,
    );
    require_input_bool(cases, "HAP-013", "inspection_evidence", false, &mut errors);

    // More module boundaries are not, by themselves, proof of better repairability.
    require_disposition(cases, "HAP-025", "reject_claim", &mut errors);
    require_input_bool(cases, "HAP-025", "repair_time_measured", false, &mut errors);
    require_input_bool(
        cases,
        "HAP-025",
        "repairability_superiority_claim",
        true,
        &mut errors,
    );

    // Positive controls check that evidence remains scoped and that unknowns can be preserved.
    require_disposition(cases, "HAP-001", "accept_synthetic_contract", &mut errors);
    require_case_text(cases, "HAP-001", "kind", "positive_control", &mut errors);
    require_disposition(cases, "HAP-015", "accept_synthetic_contract", &mut errors);
    require_input_text(cases, "HAP-015", "diagnosis", "unknown", &mut errors);
    require_input_bool(cases, "HAP-015", "hardware_actuation", false, &mut errors);
    require_disposition(cases, "HAP-022", "accept_synthetic_contract", &mut errors);
    require_input_text(
        cases,
        "HAP-022",
        "offline_function_test",
        "pass",
        &mut errors,
    );
    require_input_bool(cases, "HAP-022", "remote_actuation", false, &mut errors);
    require_disposition(cases, "HAP-024", "accept_synthetic_contract", &mut errors);
    require_input_bool(cases, "HAP-024", "attribution", true, &mut errors);
    require_input_bool(
        cases,
        "HAP-024",
        "full_dataset_embedded",
        false,
        &mut errors,
    );

    errors
}

fn audit_values(
    schema: &Value,
    corpus: &Value,
    schema_raw: &str,
    corpus_raw: &str,
) -> Result<AuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(corpus)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["corpus.cases must be an array".to_owned()]);
    };

    let mut errors = Vec::new();
    let mut seen = BTreeSet::<String>::new();
    let mut positive_controls = 0usize;
    let mut negative_controls = 0usize;
    let mut indeterminate_controls = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let Some(id) = text_at(case, "case_id") else {
            errors.push(format!("cases[{index}].case_id must be a string"));
            continue;
        };
        if !seen.insert(id.to_owned()) {
            errors.push(format!("duplicate case_id {id}"));
        }
        let kind = text_at(case, "kind").unwrap_or("");
        let disposition = text_at(case, "expected_disposition").unwrap_or("");
        let expected_disposition = match kind {
            "positive_control" => {
                positive_controls += 1;
                "accept_synthetic_contract"
            }
            "negative_control" => {
                negative_controls += 1;
                "reject_claim"
            }
            "indeterminate_control" => {
                indeterminate_controls += 1;
                "remain_indeterminate"
            }
            _ => {
                errors.push(format!("{id}: unsupported kind {kind:?}"));
                continue;
            }
        };
        if disposition != expected_disposition {
            errors.push(format!(
                "{id}: kind {kind} requires disposition {expected_disposition}, got {disposition}"
            ));
        }
    }

    let expected_ids: BTreeSet<String> = EXPECTED_CASE_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect();
    for missing in expected_ids.difference(&seen) {
        errors.push(format!("required case {missing} is missing"));
    }
    for extra in seen.difference(&expected_ids) {
        errors.push(format!("unexpected case_id {extra}"));
    }

    if positive_controls != 11 || negative_controls != 16 || indeterminate_controls != 3 {
        errors.push(format!(
            "control census changed: positives={positive_controls}, negatives={negative_controls}, indeterminate={indeterminate_controls}; expected 11/16/3"
        ));
    }

    errors.extend(check_critical_controls(cases));
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(AuditSummary {
        fixture_count: cases.len(),
        positive_controls,
        negative_controls,
        indeterminate_controls,
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        corpus_sha256: format!("{:x}", Sha256::digest(corpus_raw.as_bytes())),
    })
}

fn audit_texts(schema_raw: &str, corpus_raw: &str) -> Result<AuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("schema JSON parse failed: {error}")])?;
    let corpus: Value = serde_json::from_str(corpus_raw)
        .map_err(|error| vec![format!("corpus JSON parse failed: {error}")])?;
    audit_values(&schema, &corpus, schema_raw, corpus_raw)
}

fn read_arg_path(flag: &str) -> Option<PathBuf> {
    let mut args = env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == flag {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn run() -> Result<(), String> {
    let schema_path = read_arg_path("--schema").unwrap_or_else(default_schema_path);
    let corpus_path = read_arg_path("--corpus").unwrap_or_else(default_corpus_path);
    let portfolio_schema_path = read_arg_path("--portfolio-schema");
    let portfolio_path = read_arg_path("--portfolio");
    if portfolio_schema_path.is_some() != portfolio_path.is_some() {
        return Err("--portfolio-schema and --portfolio must be supplied together".to_owned());
    }
    let sourcing_schema_path = read_arg_path("--sourcing-schema");
    let sourcing_snapshot_path = read_arg_path("--sourcing-snapshot");
    if sourcing_schema_path.is_some() != sourcing_snapshot_path.is_some() {
        return Err(
            "--sourcing-schema and --sourcing-snapshot must be supplied together".to_owned(),
        );
    }
    let decision_schema_path = read_arg_path("--decision-schema");
    let decision_profile_path = read_arg_path("--decision-profile");
    if decision_schema_path.is_some() != decision_profile_path.is_some() {
        return Err(
            "--decision-schema and --decision-profile must be supplied together".to_owned(),
        );
    }
    let faults_schema_path = read_arg_path("--faults-schema");
    let faults_curriculum_path = read_arg_path("--faults-curriculum");
    if faults_schema_path.is_some() != faults_curriculum_path.is_some() {
        return Err("--faults-schema and --faults-curriculum must be supplied together".to_owned());
    }
    let morphology_schema_path = read_arg_path("--morphology-schema");
    let morphology_portfolio_path = read_arg_path("--morphology-portfolio");
    if morphology_schema_path.is_some() != morphology_portfolio_path.is_some() {
        return Err(
            "--morphology-schema and --morphology-portfolio must be supplied together".to_owned(),
        );
    }
    let restoration_schema_path = read_arg_path("--restoration-schema");
    let restoration_ledger_path = read_arg_path("--restoration-ledger");
    if restoration_schema_path.is_some() != restoration_ledger_path.is_some() {
        return Err(
            "--restoration-schema and --restoration-ledger must be supplied together".to_owned(),
        );
    }
    let experiment_schema_path = read_arg_path("--experiment-schema");
    let experiment_plan_path = read_arg_path("--experiment-plan");
    if experiment_schema_path.is_some() != experiment_plan_path.is_some() {
        return Err(
            "--experiment-schema and --experiment-plan must be supplied together".to_owned(),
        );
    }
    let compatibility_schema_path = read_arg_path("--compatibility-schema");
    let compatibility_graph_path = read_arg_path("--compatibility-graph");
    if compatibility_schema_path.is_some() != compatibility_graph_path.is_some() {
        return Err(
            "--compatibility-schema and --compatibility-graph must be supplied together".to_owned(),
        );
    }

    let schema_raw = fs::read_to_string(&schema_path)
        .map_err(|error| format!("cannot read {}: {error}", schema_path.display()))?;
    let corpus_raw = fs::read_to_string(&corpus_path)
        .map_err(|error| format!("cannot read {}: {error}", corpus_path.display()))?;

    let summary = audit_texts(&schema_raw, &corpus_raw).map_err(|errors| errors.join("\n"))?;
    let mut report = json!({
        "report_version": "household-appliance-bench-audit/0.2",
        "status": "PASS_SYNTHETIC_CONTRACT_ONLY",
        "physical_qualification": false,
        "product_safety_assessment": false,
        "fixture_count": summary.fixture_count,
        "positive_controls": summary.positive_controls,
        "negative_controls": summary.negative_controls,
        "indeterminate_controls": summary.indeterminate_controls,
        "schema_sha256": summary.schema_sha256,
        "corpus_sha256": summary.corpus_sha256,
        "schema_path": schema_path.display().to_string(),
        "corpus_path": corpus_path.display().to_string()
    });

    if let (Some(portfolio_schema_path), Some(portfolio_path)) =
        (portfolio_schema_path, portfolio_path)
    {
        let portfolio_schema_raw = fs::read_to_string(&portfolio_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", portfolio_schema_path.display()))?;
        let portfolio_raw = fs::read_to_string(&portfolio_path)
            .map_err(|error| format!("cannot read {}: {error}", portfolio_path.display()))?;
        let portfolio_summary =
            portfolio::audit_portfolio_texts(&portfolio_schema_raw, &portfolio_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["large_appliance_portfolio"] = json!({
            "status": "PASS_SYNTHETIC_PORTFOLIO_CONTRACT_ONLY",
            "family_count": portfolio_summary.family_count,
            "washer_variant_count": portfolio_summary.washer_variant_count,
            "washer_protocol_metric_count": portfolio_summary.washer_protocol_metric_count,
            "proposed_target_count": portfolio_summary.proposed_target_count,
            "schema_sha256": portfolio_summary.schema_sha256,
            "portfolio_sha256": portfolio_summary.portfolio_sha256,
            "schema_path": portfolio_schema_path.display().to_string(),
            "portfolio_path": portfolio_path.display().to_string(),
            "physical_performance_established": false,
            "product_safety_assessed": false
        });
    }

    if let (Some(sourcing_schema_path), Some(sourcing_snapshot_path)) =
        (sourcing_schema_path, sourcing_snapshot_path)
    {
        let sourcing_schema_raw = fs::read_to_string(&sourcing_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", sourcing_schema_path.display()))?;
        let sourcing_snapshot_raw =
            fs::read_to_string(&sourcing_snapshot_path).map_err(|error| {
                format!("cannot read {}: {error}", sourcing_snapshot_path.display())
            })?;
        let sourcing_summary =
            sourcing::audit_sourcing_texts(&sourcing_schema_raw, &sourcing_snapshot_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["washer_parts_resilience_sourcing"] = json!({
            "status": "PASS_DESK_SNAPSHOT_CONTRACT_ONLY",
            "candidate_lead_count": sourcing_summary.candidate_count,
            "price_observation_count": sourcing_summary.price_observation_count,
            "parts_catalogue_probe_count": sourcing_summary.part_probe_count,
            "model_fit_confirmed_probe_count": sourcing_summary.model_fit_confirmed_probe_count,
            "selected_control_count": sourcing_summary.selected_control_count,
            "purchased_candidate_count": sourcing_summary.purchased_candidate_count,
            "schema_sha256": sourcing_summary.schema_sha256,
            "snapshot_sha256": sourcing_summary.snapshot_sha256,
            "schema_path": sourcing_schema_path.display().to_string(),
            "snapshot_path": sourcing_snapshot_path.display().to_string(),
            "physical_performance_established": false,
            "product_safety_assessed": false,
            "purchase_authorized": false
        });
    }

    if let (Some(decision_schema_path), Some(decision_profile_path)) =
        (decision_schema_path, decision_profile_path)
    {
        let decision_schema_raw = fs::read_to_string(&decision_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", decision_schema_path.display()))?;
        let decision_profile_raw = fs::read_to_string(&decision_profile_path)
            .map_err(|error| format!("cannot read {}: {error}", decision_profile_path.display()))?;
        let (decision_summary, assessment) =
            decision::audit_decision_texts(&decision_schema_raw, &decision_profile_raw)
                .map_err(|errors| errors.join("\n"))?;

        let (stakeholder_summary, stakeholder_report) =
            stakeholder::audit_stakeholder_texts(&decision_schema_raw, &decision_profile_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["stakeholder_repairability_profiles"] = json!({
            "status": "PASS_STAKEHOLDER_PROFILE_CONTRACT_ONLY",
            "profile_count": stakeholder_summary.profile_count,
            "requirement_count": stakeholder_summary.requirement_count,
            "frozen_requirement_count": stakeholder_summary.frozen_requirement_count,
            "profile_ready_count": stakeholder_summary.profile_ready_count,
            "report": stakeholder_report,
            "profile_specific_selection_claim_permitted": false,
            "physical_user_research_completed": false
        });

        report["repairability_architecture_decision"] = json!({
            "status": "PASS_DECISION_PROFILE_CONTRACT_ONLY",
            "candidate_count": decision_summary.candidate_count,
            "hard_gate_count": decision_summary.hard_gate_count,
            "criterion_count": decision_summary.criterion_count,
            "measurement_observation_count": decision_summary.observation_count,
            "eligible_candidate_count": decision_summary.eligible_candidate_count,
            "robust_pareto_frontier_count": decision_summary.pareto_frontier_count,
            "schema_sha256": decision_summary.schema_sha256,
            "decision_sha256": decision_summary.decision_sha256,
            "schema_path": decision_schema_path.display().to_string(),
            "decision_profile_path": decision_profile_path.display().to_string(),
            "assessment": assessment,
            "eligible_comparative_evidence_available": decision_summary.eligible_candidate_count >= 2,
            "product_winner_declared": false,
            "product_safety_assessed": false
        });
    }

    if let (Some(faults_schema_path), Some(faults_curriculum_path)) =
        (faults_schema_path, faults_curriculum_path)
    {
        let faults_schema_raw = fs::read_to_string(&faults_schema_path)
            .map_err(|error| format!("cannot read {}: {error}", faults_schema_path.display()))?;
        let faults_curriculum_raw =
            fs::read_to_string(&faults_curriculum_path).map_err(|error| {
                format!("cannot read {}: {error}", faults_curriculum_path.display())
            })?;
        let faults_summary =
            faults::audit_fault_curriculum_texts(&faults_schema_raw, &faults_curriculum_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["washer_fault_to_repair_curriculum"] = json!({
            "status": "PASS_FAULT_CURRICULUM_CONTRACT_ONLY",
            "task_count": faults_summary.task_count,
            "user_tier_task_count": faults_summary.user_tier_count,
            "qualified_tier_task_count": faults_summary.qualified_tier_count,
            "reference_count": faults_summary.reference_count,
            "schema_sha256": faults_summary.schema_sha256,
            "curriculum_sha256": faults_summary.curriculum_sha256,
            "schema_path": faults_schema_path.display().to_string(),
            "curriculum_path": faults_curriculum_path.display().to_string(),
            "physical_fault_trials_completed": false,
            "local_prevalence_established": false,
            "product_safety_assessed": false
        });
    }

    if let (Some(morphology_schema_path), Some(morphology_portfolio_path)) =
        (morphology_schema_path, morphology_portfolio_path)
    {
        let morphology_schema_raw =
            fs::read_to_string(&morphology_schema_path).map_err(|error| {
                format!("cannot read {}: {error}", morphology_schema_path.display())
            })?;
        let morphology_portfolio_raw =
            fs::read_to_string(&morphology_portfolio_path).map_err(|error| {
                format!(
                    "cannot read {}: {error}",
                    morphology_portfolio_path.display()
                )
            })?;
        let morphology_summary =
            morphology::audit_morphology_texts(&morphology_schema_raw, &morphology_portfolio_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["cross_appliance_morphology"] = json!({
            "status": "PASS_MORPHOLOGY_CONTRACT_ONLY",
            "family_count": morphology_summary.family_count,
            "variant_count": morphology_summary.variant_count,
            "design_pattern_count": morphology_summary.pattern_count,
            "hybrid_candidate_count": morphology_summary.hybrid_count,
            "schema_sha256": morphology_summary.schema_sha256,
            "portfolio_sha256": morphology_summary.portfolio_sha256,
            "schema_path": morphology_schema_path.display().to_string(),
            "portfolio_path": morphology_portfolio_path.display().to_string(),
            "physical_interchangeability_established": false,
            "winner_declared": false
        });
    }

    if let (Some(restoration_schema_path), Some(restoration_ledger_path)) =
        (restoration_schema_path, restoration_ledger_path)
    {
        let restoration_schema_raw =
            fs::read_to_string(&restoration_schema_path).map_err(|error| {
                format!("cannot read {}: {error}", restoration_schema_path.display())
            })?;
        let restoration_ledger_raw =
            fs::read_to_string(&restoration_ledger_path).map_err(|error| {
                format!("cannot read {}: {error}", restoration_ledger_path.display())
            })?;
        let restoration_summary =
            restoration::audit_restoration_texts(&restoration_schema_raw, &restoration_ledger_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["repair_restoration_evidence"] = json!({
            "status": "PASS_SYNTHETIC_RESTORATION_CONTRACT_ONLY",
            "episode_count": restoration_summary.episode_count,
            "synthetic_episode_count": restoration_summary.synthetic_episode_count,
            "empirical_episode_count": restoration_summary.empirical_episode_count,
            "started_attempt_count_in_synthetic_fixture": restoration_summary.started_attempt_count,
            "verified_restoration_examples_in_synthetic_fixture": restoration_summary.verified_restoration_example_count,
            "completed_followup_examples_in_synthetic_fixture": restoration_summary.completed_followup_example_count,
            "recurrence_examples_in_synthetic_fixture": restoration_summary.recurrence_example_count,
            "durable_success_examples_in_synthetic_fixture": restoration_summary.durable_success_example_count,
            "benchmark_count": restoration_summary.benchmark_count,
            "schema_sha256": restoration_summary.schema_sha256,
            "ledger_sha256": restoration_summary.ledger_sha256,
            "schema_path": restoration_schema_path.display().to_string(),
            "ledger_path": restoration_ledger_path.display().to_string(),
            "physical_repair_outcomes_established": false,
            "product_safety_assessed": false,
            "architecture_winner_declared": false
        });
    }

    if let (Some(experiment_schema_path), Some(experiment_plan_path)) =
        (experiment_schema_path, experiment_plan_path)
    {
        let experiment_schema_raw =
            fs::read_to_string(&experiment_schema_path).map_err(|error| {
                format!("cannot read {}: {error}", experiment_schema_path.display())
            })?;
        let experiment_plan_raw = fs::read_to_string(&experiment_plan_path)
            .map_err(|error| format!("cannot read {}: {error}", experiment_plan_path.display()))?;
        let experiment_summary =
            experiment::audit_experiment_texts(&experiment_schema_raw, &experiment_plan_raw)
                .map_err(|errors| errors.join("\n"))?;

        report["repairability_factorial_experiment"] = json!({
            "status": "PASS_FACTORIAL_PLAN_CONTRACT_ONLY",
            "cell_count": experiment_summary.cell_count,
            "factor_count": experiment_summary.factor_count,
            "planned_trials_per_cell": experiment_summary.planned_trials_per_cell,
            "planned_trial_count": experiment_summary.planned_trial_count,
            "observed_trial_count": experiment_summary.observed_trial_count,
            "benchmark_count": experiment_summary.benchmark_count,
            "schema_sha256": experiment_summary.schema_sha256,
            "plan_sha256": experiment_summary.plan_sha256,
            "schema_path": experiment_schema_path.display().to_string(),
            "plan_path": experiment_plan_path.display().to_string(),
            "physical_trials_started": false,
            "observed_outcome_count": 0,
            "winner_declared": false
        });
    }

    if let (Some(compatibility_schema_path), Some(compatibility_graph_path)) =
        (compatibility_schema_path, compatibility_graph_path)
    {
        let compatibility_schema_raw =
            fs::read_to_string(&compatibility_schema_path).map_err(|error| {
                format!(
                    "cannot read {}: {error}",
                    compatibility_schema_path.display()
                )
            })?;
        let compatibility_graph_raw =
            fs::read_to_string(&compatibility_graph_path).map_err(|error| {
                format!(
                    "cannot read {}: {error}",
                    compatibility_graph_path.display()
                )
            })?;
        let compatibility_summary = compatibility::audit_compatibility_texts(
            &compatibility_schema_raw,
            &compatibility_graph_raw,
        )
        .map_err(|errors| errors.join("\n"))?;

        report["parts_compatibility_evidence_graph"] = json!({
            "status": "PASS_SYNTHETIC_PARTS_COMPATIBILITY_CONTRACT_ONLY",
            "model_count": compatibility_summary.model_count,
            "part_count": compatibility_summary.part_count,
            "claim_count": compatibility_summary.claim_count,
            "evidence_count": compatibility_summary.evidence_count,
            "supplier_path_count": compatibility_summary.supplier_path_count,
            "functional_claim_examples_in_synthetic_fixture": compatibility_summary.functional_claim_examples,
            "incompatibility_examples_in_synthetic_fixture": compatibility_summary.incompatible_claim_examples,
            "unknown_claim_examples_in_synthetic_fixture": compatibility_summary.unknown_claim_examples,
            "independent_delivered_groups_in_synthetic_fixture": compatibility_summary.independent_delivered_groups_in_fixture,
            "benchmark_count": compatibility_summary.benchmark_count,
            "schema_sha256": compatibility_summary.schema_sha256,
            "graph_sha256": compatibility_summary.graph_sha256,
            "schema_path": compatibility_schema_path.display().to_string(),
            "graph_path": compatibility_graph_path.display().to_string(),
            "real_compatibility_claims_established": false,
            "purchase_authorized": false,
            "product_safety_assessed": false
        });
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("household-appliance-bench-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_schema() -> Value {
        serde_json::from_str(include_str!(
            "../../../schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json"
        ))
        .expect("canonical schema JSON must parse")
    }

    fn canonical_corpus() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/household_appliance/benchmark_scenarios_v0_1.json"
        ))
        .expect("canonical corpus JSON must parse")
    }

    fn audit_mutation(corpus: &Value) -> Result<AuditSummary, Errors> {
        let schema = canonical_schema();
        let schema_raw = serde_json::to_string(&schema).expect("schema serialize");
        let corpus_raw = serde_json::to_string(corpus).expect("corpus serialize");
        audit_values(&schema, corpus, &schema_raw, &corpus_raw)
    }

    #[test]
    fn canonical_corpus_passes_synthetic_contract_audit() {
        let schema_raw =
            include_str!("../../../schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json");
        let corpus_raw =
            include_str!("../../../fixtures/household_appliance/benchmark_scenarios_v0_1.json");
        let summary = audit_texts(schema_raw, corpus_raw).expect("canonical corpus must qualify");
        assert_eq!(summary.fixture_count, 30);
        assert_eq!(summary.positive_controls, 11);
        assert_eq!(summary.negative_controls, 16);
        assert_eq!(summary.indeterminate_controls, 3);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.corpus_sha256.len(), 64);
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut corpus = canonical_corpus();
        corpus["cases"][1]["case_id"] = json!("HAP-001");
        let errors = audit_mutation(&corpus).expect_err("duplicate identity must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate case_id HAP-001"))
        );
    }

    #[test]
    fn negative_control_cannot_be_relabelled_as_accepted() {
        let mut corpus = canonical_corpus();
        corpus["cases"][4]["expected_disposition"] = json!("accept_synthetic_contract");
        let errors = audit_mutation(&corpus).expect_err("contradictory negative control must fail");
        assert!(errors.iter().any(|error| error.contains("HAP-005")));
    }

    #[test]
    fn critical_interlock_case_cannot_become_safe_to_operate() {
        let mut corpus = canonical_corpus();
        corpus["cases"][13]["input"]["model_output"] = json!("unknown");
        let errors = audit_mutation(&corpus).expect_err("critical fixture mutation must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("HAP-014.input.model_output"))
        );
    }

    #[test]
    fn missing_required_negative_control_is_rejected() {
        let mut corpus = canonical_corpus();
        corpus["cases"]
            .as_array_mut()
            .expect("cases array")
            .remove(26);
        let errors = audit_mutation(&corpus).expect_err("missing negative control must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("required case HAP-027 is missing"))
        );
    }

    #[test]
    fn a_shortened_source_registry_fails_the_schema() {
        let mut corpus = canonical_corpus();
        corpus["provenance"]["source_urls"]
            .as_array_mut()
            .expect("source URLs array")
            .truncate(2);
        let errors = audit_mutation(&corpus).expect_err("too few source URLs must fail");
        assert!(!errors.is_empty());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/morphology.rs:

use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:cross-appliance-architecture-morphology:0.1.0";
const PORTFOLIO_ID: &str = "APPLIANCE-MORPHOLOGY-001";
const EXPECTED_FAMILIES: [&str; 9] = [
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
const EXPECTED_PATTERNS: [&str; 10] = [
    "M01", "M02", "M03", "M04", "M05", "M06", "M07", "M08", "M09", "M10",
];
const EXPECTED_HYBRIDS: [&str; 10] = [
    "H01", "H02", "H03", "H04", "H05", "H06", "H07", "H08", "H09", "H10",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MorphologyAuditSummary {
    pub(crate) family_count: usize,
    pub(crate) variant_count: usize,
    pub(crate) pattern_count: usize,
    pub(crate) hybrid_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) portfolio_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("morphology schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("morphology schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "morphology schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile morphology schema: {error}"))
}

fn audit_values(
    schema: &Value,
    portfolio: &Value,
    schema_raw: &str,
    portfolio_raw: &str,
) -> Result<MorphologyAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(portfolio)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(portfolio, "schema_version") != Some("0.1.0")
        || text_at(portfolio, "portfolio_id") != Some(PORTFOLIO_ID)
    {
        errors.push("morphology portfolio identity/version changed".to_owned());
    }
    if text_at(portfolio, "status") != Some("design_space_hypotheses_only")
        || text_at(portfolio, "claim_ceiling")
            != Some("no_physical_compatibility_performance_savings_safety_or_superiority_claim")
    {
        errors.push("morphology portfolio cannot claim physical compatibility, performance, savings, safety or superiority".to_owned());
    }

    let Some(families) = portfolio.get("families").and_then(Value::as_array) else {
        return Err(vec!["families must be an array".to_owned()]);
    };
    if families.len() != EXPECTED_FAMILIES.len() {
        errors.push(format!(
            "morphology must retain {} product families",
            EXPECTED_FAMILIES.len()
        ));
    }
    let mut family_ids = BTreeSet::new();
    let mut variant_ids = BTreeSet::new();
    let mut variant_count = 0usize;

    for (index, family) in families.iter().enumerate() {
        let Some(family_id) = text_at(family, "id") else {
            errors.push(format!("families[{index}].id must be a string"));
            continue;
        };
        if !family_ids.insert(family_id.to_owned()) {
            errors.push(format!("duplicate family ID {family_id}"));
        }
        if EXPECTED_FAMILIES.get(index).copied() != Some(family_id) {
            errors.push(format!(
                "family order/identity changed at index {index}: {family_id}"
            ));
        }
        if text_at(family, "status") != Some("family_profile_not_physically_validated") {
            errors.push(format!("{family_id}: family must remain unvalidated"));
        }
        if family
            .get("critical_constraints")
            .and_then(Value::as_array)
            .is_none_or(|items| items.len() < 3)
            || family
                .get("useful_output_metrics")
                .and_then(Value::as_array)
                .is_none_or(|items| items.len() < 3)
        {
            errors.push(format!(
                "{family_id}: critical constraints and useful-output metrics are mandatory"
            ));
        }

        let Some(variants) = family.get("variants").and_then(Value::as_array) else {
            errors.push(format!("{family_id}: variants must be an array"));
            continue;
        };
        if variants.len() != 3 {
            errors.push(format!(
                "{family_id}: exactly three initial variants are required"
            ));
        }
        for variant in variants {
            variant_count += 1;
            let Some(variant_id) = text_at(variant, "id") else {
                errors.push(format!("{family_id}: variant ID must be a string"));
                continue;
            };
            if !variant_ids.insert(variant_id.to_owned()) {
                errors.push(format!("duplicate variant ID {variant_id}"));
            }
            if text_at(variant, "status") != Some("hypothesis_not_validated") {
                errors.push(format!("{variant_id}: variant cannot claim validation"));
            }
            if variant
                .get("tradeoffs")
                .and_then(Value::as_array)
                .is_none_or(|items| items.len() < 2)
                || variant
                    .get("required_tests")
                    .and_then(Value::as_array)
                    .is_none_or(|items| items.len() < 2)
            {
                errors.push(format!(
                    "{variant_id}: at least two trade-offs and validation tests are required"
                ));
            }
        }
    }
    for expected in EXPECTED_FAMILIES {
        if !family_ids.contains(expected) {
            errors.push(format!("required family {expected} is missing"));
        }
    }
    if variant_count != 27 {
        errors.push(format!(
            "initial morphology variant census must remain 27; got {variant_count}"
        ));
    }

    let Some(patterns) = portfolio.get("design_patterns").and_then(Value::as_array) else {
        return Err(vec!["design_patterns must be an array".to_owned()]);
    };
    if patterns.len() != EXPECTED_PATTERNS.len() {
        errors.push(format!(
            "morphology must retain {} design patterns",
            EXPECTED_PATTERNS.len()
        ));
    }
    let mut pattern_ids = BTreeSet::new();
    for (index, pattern) in patterns.iter().enumerate() {
        let Some(pattern_id) = text_at(pattern, "id") else {
            errors.push(format!("design_patterns[{index}].id must be a string"));
            continue;
        };
        if !pattern_ids.insert(pattern_id.to_owned()) {
            errors.push(format!("duplicate design pattern ID {pattern_id}"));
        }
        if EXPECTED_PATTERNS.get(index).copied() != Some(pattern_id) {
            errors.push(format!(
                "design pattern order/identity changed at index {index}"
            ));
        }
        if text_at(pattern, "status") != Some("hypothesis_not_validated") {
            errors.push(format!("{pattern_id}: pattern must remain a hypothesis"));
        }
        let Some(applicable) = pattern.get("applicable_families").and_then(Value::as_array) else {
            errors.push(format!(
                "{pattern_id}: applicable_families must be an array"
            ));
            continue;
        };
        for family_ref in applicable.iter().filter_map(Value::as_str) {
            if !family_ids.contains(family_ref) {
                errors.push(format!(
                    "{pattern_id}: unknown family reference {family_ref}"
                ));
            }
        }
    }
    for expected in EXPECTED_PATTERNS {
        if !pattern_ids.contains(expected) {
            errors.push(format!("required pattern {expected} is missing"));
        }
    }

    let Some(hybrids) = portfolio.get("hybrid_candidates").and_then(Value::as_array) else {
        return Err(vec!["hybrid_candidates must be an array".to_owned()]);
    };
    if hybrids.len() != EXPECTED_HYBRIDS.len() {
        errors.push(format!(
            "morphology must retain {} hybrid candidates",
            EXPECTED_HYBRIDS.len()
        ));
    }
    let mut hybrid_ids = BTreeSet::new();
    for (index, hybrid) in hybrids.iter().enumerate() {
        let Some(hybrid_id) = text_at(hybrid, "id") else {
            errors.push(format!("hybrid_candidates[{index}].id must be a string"));
            continue;
        };
        if !hybrid_ids.insert(hybrid_id.to_owned()) {
            errors.push(format!("duplicate hybrid ID {hybrid_id}"));
        }
        if EXPECTED_HYBRIDS.get(index).copied() != Some(hybrid_id) {
            errors.push(format!("hybrid order/identity changed at index {index}"));
        }
        if text_at(hybrid, "status") != Some("hypothesis_not_validated") {
            errors.push(format!(
                "{hybrid_id}: hybrid must remain an unvalidated hypothesis"
            ));
        }
        if text_at(hybrid, "physical_interchangeability") != Some("none_claimed") {
            errors.push(format!(
                "{hybrid_id}: shared service concepts cannot imply physical interchangeability"
            ));
        }
        let Some(family_refs) = hybrid.get("family_ids").and_then(Value::as_array) else {
            errors.push(format!("{hybrid_id}: family_ids must be an array"));
            continue;
        };
        if family_refs.is_empty() {
            errors.push(format!(
                "{hybrid_id}: at least one family reference is required"
            ));
        }
        for family_ref in family_refs.iter().filter_map(Value::as_str) {
            if !family_ids.contains(family_ref) {
                errors.push(format!(
                    "{hybrid_id}: unknown family reference {family_ref}"
                ));
            }
        }
        let Some(pattern_refs) = hybrid.get("pattern_ids").and_then(Value::as_array) else {
            errors.push(format!("{hybrid_id}: pattern_ids must be an array"));
            continue;
        };
        for pattern_ref in pattern_refs.iter().filter_map(Value::as_str) {
            if !pattern_ids.contains(pattern_ref) {
                errors.push(format!(
                    "{hybrid_id}: unknown pattern reference {pattern_ref}"
                ));
            }
        }
        for (field, minimum) in [
            ("non_claims", 1usize),
            ("key_trades", 2usize),
            ("discriminating_tests", 2usize),
        ] {
            if hybrid
                .get(field)
                .and_then(Value::as_array)
                .is_none_or(|items| items.len() < minimum)
            {
                errors.push(format!("{hybrid_id}: {field} is incomplete"));
            }
        }
    }
    for expected in EXPECTED_HYBRIDS {
        if !hybrid_ids.contains(expected) {
            errors.push(format!("required hybrid {expected} is missing"));
        }
    }

    let policy = portfolio.get("portfolio_policy").unwrap_or(&Value::Null);
    if text_at(policy, "unknown_policy") != Some("unknown_not_zero")
        || text_at(policy, "safety_policy") != Some("non_compensatory_product_specific_hard_gates")
        || text_at(policy, "component_reuse_policy")
            != Some("shared_service_contracts_do_not_imply_shared_physical_components")
        || text_at(policy, "prioritization_policy")
            != Some("value_of_information_cost_and_risk_before_breadth")
        || text_at(policy, "market_rule")
            != Some("verify_jurisdiction_specific_rules_before_design_freeze")
        || policy.get("no_winner_claim").and_then(Value::as_bool) != Some(true)
    {
        errors.push("portfolio policy must preserve unknown-not-zero, non-compensatory safety, no hardware-interchangeability inference and no winner claim".to_owned());
    }
    let pilots: BTreeSet<&str> = policy
        .get("initial_pilots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if pilots != BTreeSet::from(["washer_service_spine", "heat_pump_dryer_air_side_service"]) {
        errors.push(
            "initial pilot selection must retain the washer and heat-pump dryer learning pair"
                .to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(MorphologyAuditSummary {
        family_count: families.len(),
        variant_count,
        pattern_count: patterns.len(),
        hybrid_count: hybrids.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        portfolio_sha256: format!("{:x}", Sha256::digest(portfolio_raw.as_bytes())),
    })
}

pub(crate) fn audit_morphology_texts(
    schema_raw: &str,
    portfolio_raw: &str,
) -> Result<MorphologyAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("morphology schema JSON parse failed: {error}")])?;
    let portfolio: Value = serde_json::from_str(portfolio_raw)
        .map_err(|error| vec![format!("morphology portfolio JSON parse failed: {error}")])?;
    audit_values(&schema, &portfolio, schema_raw, portfolio_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/cross_appliance_architecture_morphology_v0_1.schema.json"
    );
    const PORTFOLIO: &str = include_str!(
        "../../../fixtures/appliances/cross_appliance_architecture_morphology_v0_1.json"
    );

    fn canonical_portfolio() -> Value {
        serde_json::from_str(PORTFOLIO).expect("morphology fixture must parse")
    }

    fn audit_mutation(portfolio: &Value) -> Result<MorphologyAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("morphology schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let portfolio_raw = serde_json::to_string(portfolio).expect("portfolio serializes");
        audit_values(&schema, portfolio, &schema_raw, &portfolio_raw)
    }

    #[test]
    fn canonical_morphology_covers_nine_families_and_ten_hybrids() {
        let summary = audit_morphology_texts(SCHEMA, PORTFOLIO)
            .expect("canonical design-space hypothesis portfolio should qualify");
        assert_eq!(summary.family_count, 9);
        assert_eq!(summary.variant_count, 27);
        assert_eq!(summary.pattern_count, 10);
        assert_eq!(summary.hybrid_count, 10);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.portfolio_sha256.len(), 64);
    }

    #[test]
    fn duplicate_family_identity_is_rejected() {
        let mut portfolio = canonical_portfolio();
        portfolio["families"][1]["id"] = json!("F1-WASHER");
        let errors = audit_mutation(&portfolio).expect_err("duplicate family must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn hybrid_cannot_claim_physical_interchangeability() {
        let mut portfolio = canonical_portfolio();
        portfolio["hybrid_candidates"][0]["physical_interchangeability"] =
            json!("common_pump_hardware_is_compatible");
        let errors = audit_mutation(&portfolio).expect_err("interchangeability claim must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn unvalidated_variant_cannot_be_promoted_to_result() {
        let mut portfolio = canonical_portfolio();
        portfolio["families"][0]["variants"][1]["status"] = json!("validated_winner");
        let errors = audit_mutation(&portfolio).expect_err("premature win claim must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn unknown_family_reference_is_rejected() {
        let mut portfolio = canonical_portfolio();
        portfolio["hybrid_candidates"][1]["family_ids"][0] = json!("UNREGISTERED-FAMILY");
        let errors = audit_mutation(&portfolio).expect_err("unknown family ref must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn safety_cannot_become_a_weighted_penalty() {
        let mut portfolio = canonical_portfolio();
        portfolio["portfolio_policy"]["safety_policy"] = json!("weighted_cost_penalty");
        let errors = audit_mutation(&portfolio).expect_err("safety gate mutation must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn removing_tradeoffs_or_tests_is_rejected() {
        let mut portfolio = canonical_portfolio();
        portfolio["hybrid_candidates"][2]["discriminating_tests"] = json!([]);
        let errors = audit_mutation(&portfolio).expect_err("hybrid tests must be mandatory");
        assert!(!errors.is_empty());
    }

    #[test]
    fn family_count_cannot_become_an_unvalidated_coverage_claim() {
        let mut portfolio = canonical_portfolio();
        portfolio["status"] = json!("validated_all_appliances");
        let errors = audit_mutation(&portfolio).expect_err("coverage claim must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn unverified_part_reuse_does_not_replace_explicit_non_claims() {
        let mut portfolio = canonical_portfolio();
        portfolio["hybrid_candidates"][3]["non_claims"] = json!([]);
        let errors = audit_mutation(&portfolio).expect_err("hybrid non-claims are mandatory");
        assert!(!errors.is_empty());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/portfolio.rs:

use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:large-appliance-repairable-portfolio:0.1.0";
const EXPECTED_FAMILIES: [&str; 6] = [
    "washing_machine",
    "dishwasher",
    "refrigerator_freezer",
    "heat_pump_dryer",
    "heat_pump_water_heater",
    "heat_pump_hvac",
];
const EXPECTED_WASHER_VARIANTS: [(&str, &str); 5] = [
    ("W0-RETAIL-CONTROL", "control"),
    ("W1-ACCESSIBLE-CONVENTIONAL", "candidate"),
    ("W2-SERVICE-SPINE", "candidate"),
    ("W3-BEARING-TUB-TRADE", "orthogonal_trade"),
    ("W4-DRIVE-TOPOLOGY", "orthogonal_trade"),
];
const EXPECTED_WASHER_METRICS: [&str; 11] = [
    "cleaning_quality",
    "rinsing_quality",
    "spin_drying_outcome",
    "water_per_successful_load",
    "energy_per_successful_load",
    "cycle_duration",
    "total_restore_time",
    "repair_cost",
    "recurrence",
    "critical_safety_findings",
    "discarded_material",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PortfolioAuditSummary {
    pub(crate) family_count: usize,
    pub(crate) washer_variant_count: usize,
    pub(crate) washer_protocol_metric_count: usize,
    pub(crate) proposed_target_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) portfolio_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("portfolio schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("portfolio schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "portfolio schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile portfolio schema: {error}"))
}

fn require_text_at(value: &Value, key: &str, expected: &str, errors: &mut Errors) {
    let actual = text_at(value, key);
    if actual != Some(expected) {
        errors.push(format!("{key}: expected {expected:?}, got {actual:?}"));
    }
}

fn audit_values(
    schema: &Value,
    portfolio: &Value,
    schema_raw: &str,
    portfolio_raw: &str,
) -> Result<PortfolioAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(portfolio)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    require_text_at(portfolio, "schema_version", "0.1.0", &mut errors);
    require_text_at(
        portfolio,
        "portfolio_id",
        "LARGE-APPLIANCE-REPAIRABLE-001",
        &mut errors,
    );
    require_text_at(
        portfolio,
        "status",
        "proposed_architecture_and_benchmark_profile",
        &mut errors,
    );
    require_text_at(
        portfolio,
        "claim_ceiling",
        "no_physical_performance_or_conformity_claim",
        &mut errors,
    );
    require_text_at(schema, "$id", SCHEMA_ID, &mut errors);

    let selection = portfolio.get("selection_policy");
    let first_family = selection.and_then(|value| text_at(value, "first_large_appliance"));
    if first_family != Some("washing_machine") {
        errors.push(format!(
            "selection_policy.first_large_appliance must remain washing_machine until the choice is explicitly reviewed; got {first_family:?}"
        ));
    }
    if selection
        .and_then(|value| value.get("not_a_market_ranking"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("selection_policy must state that priority is not a market ranking".to_owned());
    }

    let Some(families) = portfolio.get("families").and_then(Value::as_array) else {
        return Err(vec!["portfolio.families must be an array".to_owned()]);
    };
    let mut family_ids = BTreeSet::new();
    if families.len() != EXPECTED_FAMILIES.len() {
        errors.push(format!(
            "portfolio must contain {} appliance families; got {}",
            EXPECTED_FAMILIES.len(),
            families.len()
        ));
    }
    for (index, family) in families.iter().enumerate() {
        let Some(id) = text_at(family, "id") else {
            errors.push(format!("families[{index}].id must be a string"));
            continue;
        };
        if !family_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate appliance family id {id}"));
        }
        if EXPECTED_FAMILIES.get(index).copied() != Some(id) {
            errors.push(format!(
                "families[{index}] must preserve reviewed portfolio order; expected {:?}, got {id}",
                EXPECTED_FAMILIES.get(index)
            ));
        }
        if family.get("priority").and_then(Value::as_u64) != Some((index + 1) as u64) {
            errors.push(format!(
                "{id}.priority must equal {} and describe engineering sequence, not market size",
                index + 1
            ));
        }
        if text_at(family, "status") != Some("proposed_no_physical_validation") {
            errors.push(format!(
                "{id}: physical-validation status must remain explicitly proposed/unvalidated"
            ));
        }

        let architecture_hypotheses = family
            .get("architecture_hypotheses")
            .and_then(Value::as_array);
        if architecture_hypotheses.is_none_or(|items| items.len() < 2) {
            errors.push(format!(
                "{id}: at least two architecture hypotheses are required"
            ));
        }
        let metrics = family.get("function_metrics").and_then(Value::as_array);
        if metrics.is_none_or(|items| items.len() < 3) {
            errors.push(format!(
                "{id}: function output and resource/service metrics are incomplete"
            ));
        }
        let user_tasks = family
            .get("user_maintenance_tasks")
            .and_then(Value::as_array);
        if user_tasks.is_none_or(|tasks| tasks.is_empty()) {
            errors.push(format!(
                "{id}: permitted user maintenance boundary is missing"
            ));
        }
        let qualified_tasks = family
            .get("qualified_service_tasks")
            .and_then(Value::as_array);
        if qualified_tasks.is_none_or(|tasks| tasks.is_empty()) {
            errors.push(format!("{id}: qualified-service boundary is missing"));
        }

        let user_text = user_tasks
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        if user_text.contains("refrigerant charging")
            || user_text.contains("refrigerant circuit")
            || user_text.contains("open pressure")
            || user_text.contains("open mains")
        {
            errors.push(format!(
                "{id}: user-maintenance tasks cross a prohibited refrigerant/pressure/mains boundary"
            ));
        }

        let regulatory_references = family
            .get("regulatory_references")
            .and_then(Value::as_array);
        if regulatory_references.is_none_or(|refs| {
            refs.iter().any(|reference| {
                reference
                    .as_str()
                    .is_none_or(|url| !url.starts_with("https://"))
            })
        }) {
            errors.push(format!("{id}: regulation references must be HTTPS URLs"));
        }
    }

    for expected in EXPECTED_FAMILIES {
        if !family_ids.contains(expected) {
            errors.push(format!("required appliance family {expected} is missing"));
        }
    }

    let Some(targets) = portfolio
        .get("benchmark_policy")
        .and_then(|policy| policy.get("proposed_targets"))
        .and_then(Value::as_array)
    else {
        return Err(vec![
            "benchmark_policy.proposed_targets must be an array".to_owned(),
        ]);
    };
    if targets.len() < 8 {
        errors.push(format!(
            "at least eight proposed benchmark targets are required; got {}",
            targets.len()
        ));
    }
    let mut target_ids = BTreeSet::new();
    for target in targets {
        let Some(id) = text_at(target, "id") else {
            errors.push("benchmark target id must be a string".to_owned());
            continue;
        };
        if !target_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate benchmark target id {id}"));
        }
        if text_at(target, "status") != Some("proposal_not_result") {
            errors.push(format!(
                "{id}: benchmark target must remain proposal_not_result until measured"
            ));
        }
    }

    let Some(variants) = portfolio
        .get("washer_architecture_variants")
        .and_then(Value::as_array)
    else {
        return Err(vec![
            "portfolio.washer_architecture_variants must be an array".to_owned(),
        ]);
    };
    if variants.len() != EXPECTED_WASHER_VARIANTS.len() {
        errors.push(format!(
            "washer architecture matrix must contain {} variants; got {}",
            EXPECTED_WASHER_VARIANTS.len(),
            variants.len()
        ));
    }
    let mut variant_ids = BTreeSet::new();
    for (index, variant) in variants.iter().enumerate() {
        let Some(id) = text_at(variant, "id") else {
            errors.push(format!(
                "washer_architecture_variants[{index}].id must be a string"
            ));
            continue;
        };
        if !variant_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate washer architecture variant {id}"));
        }
        if let Some((expected_id, expected_role)) = EXPECTED_WASHER_VARIANTS.get(index) {
            if id != *expected_id {
                errors.push(format!(
                    "washer variant at index {index} must be {expected_id}; got {id}"
                ));
            }
            if text_at(variant, "role") != Some(*expected_role) {
                errors.push(format!(
                    "{id}.role must be {expected_role}, got {:?}",
                    text_at(variant, "role")
                ));
            }
        }
        if text_at(variant, "status") != Some("proposed_no_measurements") {
            errors.push(format!(
                "{id}: architecture status must remain proposed_no_measurements until tested"
            ));
        }
        let compare_against: BTreeSet<&str> = variant
            .get("compare_against")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if index > 0 && !compare_against.contains("W0-RETAIL-CONTROL") {
            errors.push(format!(
                "{id}: every candidate/trade must compare against W0 retail control"
            ));
        }
        if variant
            .get("required_measurements")
            .and_then(Value::as_array)
            .is_none_or(|measurements| measurements.len() < 4)
        {
            errors.push(format!(
                "{id}: at least four required measurements must be named"
            ));
        }
    }
    if variants.first().and_then(|variant| text_at(variant, "id")) != Some("W0-RETAIL-CONTROL") {
        errors.push("washer architecture matrix must begin with the retail control W0".to_owned());
    }

    let Some(protocol) = portfolio.get("washer_benchmark_protocol") else {
        return Err(vec![
            "portfolio.washer_benchmark_protocol is required".to_owned(),
        ]);
    };
    if text_at(protocol, "protocol_id") != Some("WASHER-BENCHMARK-PROTOCOL-001") {
        errors.push("washer benchmark protocol identity changed".to_owned());
    }
    if text_at(protocol, "status") != Some("draft_pending_control_selection")
        || text_at(protocol, "claim_ceiling") != Some("no_measurements_no_comparison_claim")
    {
        errors.push(
            "washer benchmark protocol must remain draft and make no measurement claim".to_owned(),
        );
    }
    let control_plan = protocol.get("control_plan");
    if control_plan
        .and_then(|plan| plan.get("minimum_units"))
        .and_then(Value::as_u64)
        .is_none_or(|count| count < 3)
        || text_at(control_plan.unwrap_or(&Value::Null), "selection_state") != Some("not_selected")
    {
        errors.push(
            "washer control plan must specify at least three comparable units and remain not selected"
                .to_owned(),
        );
    }
    if text_at(
        protocol.get("workload_plan").unwrap_or(&Value::Null),
        "status",
    ) != Some("not_frozen")
        || text_at(
            protocol.get("instrumentation_plan").unwrap_or(&Value::Null),
            "status",
        ) != Some("not_selected")
    {
        errors.push(
            "washer workload/instrumentation cannot be called frozen before control selection"
                .to_owned(),
        );
    }
    let freeze = protocol.get("analysis_freeze");
    for (key, expected) in [
        (
            "stage_a",
            "freeze_measurement_method_and_control_selection_before_any_measurements",
        ),
        (
            "stage_b",
            "control_only_pilot_establishes_repeatability_and_instrument_uncertainty",
        ),
        (
            "stage_c",
            "freeze_numeric_margins_exclusions_and_analysis_before_candidate_measurements",
        ),
    ] {
        if text_at(freeze.unwrap_or(&Value::Null), key) != Some(expected) {
            errors.push(format!(
                "washer analysis freeze {key} violates the control-first sequence"
            ));
        }
    }
    if text_at(freeze.unwrap_or(&Value::Null), "missing_data_policy") != Some("unknown_not_zero") {
        errors.push("washer benchmark missing data must remain unknown, not zero".to_owned());
    }
    let Some(protocol_metrics) = protocol.get("metrics").and_then(Value::as_array) else {
        return Err(vec![
            "washer_benchmark_protocol.metrics must be an array".to_owned(),
        ]);
    };
    let mut metric_ids = BTreeSet::new();
    for metric in protocol_metrics {
        let Some(id) = text_at(metric, "id") else {
            errors.push("washer protocol metric id must be a string".to_owned());
            continue;
        };
        if !metric_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate washer protocol metric {id}"));
        }
        if text_at(metric, "status") != Some("method_to_be_frozen_before_candidate_measurements") {
            errors.push(format!(
                "{id}: metric status cannot claim a frozen or measured method"
            ));
        }
        if text_at(metric, "method").is_none_or(str::is_empty) {
            errors.push(format!("{id}: metric needs a declared method"));
        }
    }
    if protocol_metrics.len() != EXPECTED_WASHER_METRICS.len()
        || EXPECTED_WASHER_METRICS
            .iter()
            .any(|id| !metric_ids.contains(*id))
    {
        errors
            .push("washer benchmark metric set must retain the reviewed 11 dimensions".to_owned());
    }
    if text_at(
        protocol.get("service_trials").unwrap_or(&Value::Null),
        "status",
    ) != Some("not_scheduled")
    {
        errors.push(
            "washer service trials must remain unscheduled until controls and role gates are reviewed"
                .to_owned(),
        );
    }
    let evidence_rules = protocol
        .get("evidence_rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    for required_rule in [
        "Unknown parts delay, labour, cost, calibration or mass stays unknown",
        "Safety and conformity remain separate hard gates",
    ] {
        if !evidence_rules
            .iter()
            .any(|rule| rule.contains(required_rule))
        {
            errors.push(format!(
                "washer benchmark evidence rule missing: {required_rule}"
            ));
        }
    }

    let reference_titles: BTreeSet<String> = portfolio
        .get("references")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|reference| text_at(reference, "title").map(str::to_owned))
        .collect();
    for required_reference in [
        "EU ecodesign requirements: household washing machines and washer-dryers",
        "EU ecodesign requirements: household dishwashers",
        "EU ecodesign requirements: refrigerators and freezers",
        "EU ecodesign requirements: tumble dryers",
        "Open Arsenal Specs Universal Appliance Standard concept",
        "Open Repair Alliance repair-attempt dataset (October 2026 release)",
    ] {
        if !reference_titles.contains(required_reference) {
            errors.push(format!(
                "required source reference missing: {required_reference}"
            ));
        }
    }

    let repair_dataset = portfolio
        .get("references")
        .and_then(Value::as_array)
        .and_then(|references| {
            references.iter().find(|reference| {
                text_at(reference, "title")
                    == Some("Open Repair Alliance repair-attempt dataset (October 2026 release)")
            })
        });
    let scope_note = repair_dataset.and_then(|reference| text_at(reference, "scope_note"));
    if scope_note.is_none_or(|note| {
        !note.contains("not a representative random sample")
            || !note.contains("selection")
            || !note.contains("washer fault prevalence")
    }) {
        errors.push(
            "Open Repair Alliance source must preserve the sampling-bias/no-washer-prevalence caveat"
                .to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(PortfolioAuditSummary {
        family_count: families.len(),
        washer_variant_count: variants.len(),
        washer_protocol_metric_count: protocol_metrics.len(),
        proposed_target_count: targets.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        portfolio_sha256: format!("{:x}", Sha256::digest(portfolio_raw.as_bytes())),
    })
}

pub(crate) fn audit_portfolio_texts(
    schema_raw: &str,
    portfolio_raw: &str,
) -> Result<PortfolioAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("portfolio schema JSON parse failed: {error}")])?;
    let portfolio: Value = serde_json::from_str(portfolio_raw)
        .map_err(|error| vec![format!("portfolio JSON parse failed: {error}")])?;
    audit_values(&schema, &portfolio, schema_raw, portfolio_raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/large_appliance_repairable_portfolio_v0_1.schema.json"
    );
    const PORTFOLIO: &str =
        include_str!("../../../fixtures/appliances/large_appliance_repairable_portfolio_v0_1.json");

    fn canonical_portfolio() -> Value {
        serde_json::from_str(PORTFOLIO).expect("canonical portfolio JSON must parse")
    }

    fn audit_mutation(portfolio: &Value) -> Result<PortfolioAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema JSON must parse");
        let schema_raw = serde_json::to_string(&schema).expect("serialize schema");
        let portfolio_raw = serde_json::to_string(portfolio).expect("serialize portfolio");
        audit_values(&schema, portfolio, &schema_raw, &portfolio_raw)
    }

    #[test]
    fn canonical_portfolio_schema_and_semantics_are_valid() {
        let summary =
            audit_portfolio_texts(SCHEMA, PORTFOLIO).expect("portfolio must qualify synthetically");
        assert_eq!(summary.family_count, 6);
        assert_eq!(summary.washer_variant_count, 5);
        assert_eq!(summary.washer_protocol_metric_count, 11);
        assert_eq!(summary.proposed_target_count, 8);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.portfolio_sha256.len(), 64);
    }

    #[test]
    fn duplicate_family_identity_is_rejected() {
        let mut portfolio = canonical_portfolio();
        portfolio["families"][1]["id"] = json!("washing_machine");
        let errors = audit_mutation(&portfolio).expect_err("duplicate family must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate appliance family id"))
        );
    }

    #[test]
    fn first_large_appliance_cannot_change_without_explicit_review() {
        let mut portfolio = canonical_portfolio();
        portfolio["selection_policy"]["first_large_appliance"] = json!("heat_pump_hvac");
        let errors = audit_mutation(&portfolio).expect_err("unreviewed target change must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("first_large_appliance"))
        );
    }

    #[test]
    fn unmeasured_target_cannot_be_relabelled_as_result() {
        let mut portfolio = canonical_portfolio();
        portfolio["benchmark_policy"]["proposed_targets"][0]["status"] = json!("achieved");
        let errors =
            audit_mutation(&portfolio).expect_err("proposed target must not become result");
        assert!(!errors.is_empty());
    }

    #[test]
    fn refrigerant_task_cannot_be_moved_into_user_maintenance_boundary() {
        let mut portfolio = canonical_portfolio();
        portfolio["families"][2]["user_maintenance_tasks"][0] =
            json!("perform refrigerant charging and recovery");
        let errors = audit_mutation(&portfolio).expect_err("DIY refrigerant should fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("prohibited refrigerant/pressure/mains boundary"))
        );
    }

    #[test]
    fn washer_protocol_cannot_promote_candidate_results_before_preregistration() {
        let mut portfolio = canonical_portfolio();
        portfolio["washer_benchmark_protocol"]["analysis_freeze"]["stage_c"] =
            json!("freeze_numeric_margins_after_candidate_measurements");
        let errors = audit_mutation(&portfolio)
            .expect_err("analysis must be frozen before candidate measurements");
        assert!(!errors.is_empty());
    }

    #[test]
    fn washer_protocol_cannot_turn_unknown_costs_into_zero() {
        let mut portfolio = canonical_portfolio();
        portfolio["washer_benchmark_protocol"]["analysis_freeze"]["missing_data_policy"] =
            json!("unknown_equals_zero");
        let errors = audit_mutation(&portfolio).expect_err("unknown evidence cannot become zero");
        assert!(!errors.is_empty());
    }

    #[test]
    fn washer_protocol_rejects_missing_function_metric() {
        let mut portfolio = canonical_portfolio();
        portfolio["washer_benchmark_protocol"]["metrics"]
            .as_array_mut()
            .expect("metrics")
            .remove(0);
        let errors = audit_mutation(&portfolio)
            .expect_err("required functional output metric must remain present");
        assert!(!errors.is_empty());
    }

    #[test]
    fn washer_protocol_requires_three_control_units() {
        let mut portfolio = canonical_portfolio();
        portfolio["washer_benchmark_protocol"]["control_plan"]["minimum_units"] = json!(1);
        let errors =
            audit_mutation(&portfolio).expect_err("single control does not meet pilot floor");
        assert!(!errors.is_empty());
    }

    #[test]
    fn open_repair_dataset_cannot_be_used_as_unqualified_washer_prevalence() {
        let mut portfolio = canonical_portfolio();
        let reference = portfolio["references"]
            .as_array_mut()
            .expect("references")
            .iter_mut()
            .find(|reference| {
                reference["title"]
                    == "Open Repair Alliance repair-attempt dataset (October 2026 release)"
            })
            .expect("Open Repair Alliance reference");
        reference["scope_note"] =
            json!("This dataset proves the most common washer field failure rates.");
        let errors = audit_mutation(&portfolio)
            .expect_err("repair event data must retain the sampling caveat");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("sampling-bias/no-washer-prevalence caveat"))
        );
    }

    #[test]
    fn washer_variant_cannot_be_marked_as_measured_before_physical_tests() {
        let mut portfolio = canonical_portfolio();
        portfolio["washer_architecture_variants"][2]["status"] = json!("measured_pass");
        let errors =
            audit_mutation(&portfolio).expect_err("an unmeasured variant must not be promoted");
        assert!(!errors.is_empty());
    }

    #[test]
    fn washer_variant_matrix_must_keep_the_retail_control() {
        let mut portfolio = canonical_portfolio();
        portfolio["washer_architecture_variants"][0]["id"] = json!("W0-NEW-DESIGN");
        let errors =
            audit_mutation(&portfolio).expect_err("retail control identity must be stable");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("W0-RETAIL-CONTROL"))
        );
    }

    #[test]
    fn missing_appliance_family_is_rejected() {
        let mut portfolio = canonical_portfolio();
        portfolio["families"][5]["id"] = json!("unexpected_hvac_family");
        let errors = audit_mutation(&portfolio).expect_err("missing family must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("required appliance family heat_pump_hvac is missing"))
        );
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/restoration.rs:

use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:repair-restoration-evidence:0.1.0";
const LEDGER_ID: &str = "REPAIR-RESTORE-001";
const EXPECTED_EPISODES: [&str; 5] = [
    "SYN-REST-001",
    "SYN-REST-002",
    "SYN-REST-003",
    "SYN-REST-004",
    "SYN-REST-005",
];
const EXPECTED_BENCHMARKS: [&str; 7] = [
    "RB-001", "RB-002", "RB-003", "RB-004", "RB-005", "RB-006", "RB-007",
];
const EXPECTED_FAMILIES: [&str; 9] = [
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
const COST_FIELDS: [&str; 9] = [
    "parts_minor",
    "labour_minor",
    "travel_minor",
    "freight_minor",
    "failed_attempts_minor",
    "downtime_minor",
    "rework_minor",
    "waste_handling_minor",
    "discarded_material_grams",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RestorationAuditSummary {
    pub(crate) episode_count: usize,
    pub(crate) synthetic_episode_count: usize,
    pub(crate) empirical_episode_count: usize,
    pub(crate) started_attempt_count: usize,
    pub(crate) verified_restoration_example_count: usize,
    pub(crate) completed_followup_example_count: usize,
    pub(crate) recurrence_example_count: usize,
    pub(crate) durable_success_example_count: usize,
    pub(crate) benchmark_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) ledger_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("restoration schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("restoration schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "restoration schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile restoration schema: {error}"))
}

fn audit_values(
    schema: &Value,
    ledger: &Value,
    schema_raw: &str,
    ledger_raw: &str,
) -> Result<RestorationAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(ledger)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(ledger, "schema_version") != Some("0.1.0")
        || text_at(ledger, "ledger_id") != Some(LEDGER_ID)
        || text_at(ledger, "status") != Some("synthetic_contract_examples_only")
        || text_at(ledger, "claim_ceiling")
            != Some("synthetic_examples_are_not_empirical_or_product_safety_evidence")
    {
        errors.push(
            "canonical restoration fixture must remain synthetic-only and keep its claim ceiling"
                .to_owned(),
        );
    }

    let Some(episodes) = ledger.get("episodes").and_then(Value::as_array) else {
        return Err(vec!["restoration episodes must be an array".to_owned()]);
    };
    if episodes.len() != EXPECTED_EPISODES.len() {
        errors.push(format!(
            "synthetic contract fixture must retain {} episodes",
            EXPECTED_EPISODES.len()
        ));
    }

    let mut episode_ids = BTreeSet::new();
    let mut unit_tokens = BTreeSet::new();
    let mut started_attempt_count = 0usize;
    let mut verified_restoration_count = 0usize;
    let mut completed_followup_count = 0usize;
    let mut recurrence_count = 0usize;
    let mut durable_success_count = 0usize;
    let mut empirical_count = 0usize;

    for (index, episode) in episodes.iter().enumerate() {
        let Some(episode_id) = text_at(episode, "episode_id") else {
            errors.push(format!("episodes[{index}].episode_id must be a string"));
            continue;
        };
        if !episode_ids.insert(episode_id.to_owned()) {
            errors.push(format!("duplicate repair episode ID {episode_id}"));
        }
        if EXPECTED_EPISODES.get(index).copied() != Some(episode_id) {
            errors.push(format!(
                "synthetic episode order/identity changed at index {index}: {episode_id}"
            ));
        }
        if text_at(episode, "evidence_class") != Some("synthetic_fixture") {
            errors.push(format!(
                "{episode_id}: canonical example must remain synthetic"
            ));
        }
        if bool_at(episode, "empirical_eligible") != Some(false) {
            empirical_count += 1;
            errors.push(format!(
                "{episode_id}: synthetic fixture cannot be promoted to empirical evidence"
            ));
        }

        let Some(appliance) = episode.get("appliance") else {
            errors.push(format!("{episode_id}: appliance identity is required"));
            continue;
        };
        let family_id = text_at(appliance, "family_id").unwrap_or("");
        if !EXPECTED_FAMILIES.contains(&family_id) {
            errors.push(format!(
                "{episode_id}: unknown appliance family {family_id}"
            ));
        }
        if let Some(unit_token) = text_at(appliance, "unit_token") {
            if !unit_tokens.insert(unit_token.to_owned()) {
                errors.push(format!(
                    "{episode_id}: duplicate synthetic unit token {unit_token}"
                ));
            }
        }

        let Some(intervention) = episode.get("intervention") else {
            errors.push(format!("{episode_id}: intervention object is required"));
            continue;
        };
        let started = bool_at(intervention, "repair_attempt_started").unwrap_or(false);
        let intervention_status = text_at(intervention, "status").unwrap_or("");
        let outcome = text_at(episode, "outcome").unwrap_or("");
        if started {
            started_attempt_count += 1;
            if intervention_status == "not_started" {
                errors.push(format!(
                    "{episode_id}: a counted repair attempt cannot have status not_started"
                ));
            }
        } else if intervention_status != "not_started" {
            errors.push(format!(
                "{episode_id}: intervention status {intervention_status} contradicts repair_attempt_started=false"
            ));
        }

        let Some(verification) = episode.get("verification") else {
            errors.push(format!("{episode_id}: verification object is required"));
            continue;
        };
        let function_gate = text_at(verification, "core_function_gate").unwrap_or("");
        let safety_gate = text_at(verification, "safety_gate").unwrap_or("");
        let required_tests = verification
            .get("required_test_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let passed_tests = verification
            .get("passed_test_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let measurement_receipts = verification
            .get("measurement_receipt_ids")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if passed_tests > required_tests {
            errors.push(format!(
                "{episode_id}: passed_test_count cannot exceed required_test_count"
            ));
        }
        if outcome == "verified_restoration" {
            verified_restoration_count += 1;
            if !started
                || intervention_status != "completed"
                || function_gate != "passed"
                || safety_gate != "passed"
                || required_tests == 0
                || passed_tests < required_tests
                || measurement_receipts == 0
                || text_at(verification, "useful_output_metric_id").is_none_or(str::is_empty)
            {
                errors.push(format!(
                    "{episode_id}: verified restoration requires a started/completed attempt, passed function and safety gates, all required tests, a useful-output metric, and measurement receipts"
                ));
            }
        }
        if outcome == "failed_repair" && (!started || function_gate == "passed") {
            errors.push(format!(
                "{episode_id}: failed_repair must be a started attempt that did not pass the core-function gate"
            ));
        }
        if outcome == "diagnosis_only" && started {
            errors.push(format!(
                "{episode_id}: diagnosis_only cannot be used to hide a started repair attempt"
            ));
        }
        let diagnosis = episode.get("diagnosis");
        let diagnosis_status = diagnosis.and_then(|value| text_at(value, "status"));
        let hazard_screen = diagnosis.and_then(|value| text_at(value, "hazard_screen"));
        if outcome == "safety_stop"
            && diagnosis_status != Some("safety_stop")
            && hazard_screen != Some("stop_now")
            && hazard_screen != Some("refer_to_qualified_role")
        {
            errors.push(format!(
                "{episode_id}: safety_stop outcome needs an explicit stop or referral record"
            ));
        }
        if bool_at(verification, "energy_water_improvement_claimed") == Some(true)
            && (function_gate != "passed"
                || safety_gate != "passed"
                || text_at(verification, "useful_output_metric_id").is_none_or(str::is_empty))
        {
            errors.push(format!(
                "{episode_id}: energy/water improvement cannot be claimed without verified useful output and a passed safety gate"
            ));
        }

        let Some(follow_up) = episode.get("follow_up") else {
            errors.push(format!("{episode_id}: follow_up object is required"));
            continue;
        };
        let follow_status = text_at(follow_up, "status").unwrap_or("");
        let recurred = follow_up
            .get("same_fault_recurred")
            .and_then(Value::as_bool);
        let recurrence_is_null = follow_up
            .get("same_fault_recurred")
            .is_none_or(Value::is_null);
        let follow_days = follow_up.get("follow_up_days").and_then(Value::as_f64);
        let exposure = follow_up.get("exposure_value").and_then(Value::as_f64);
        let exposure_unit = text_at(follow_up, "exposure_unit").unwrap_or("");
        if follow_status == "complete" {
            completed_followup_count += 1;
            if recurred.is_none()
                || follow_days.is_none_or(|days| days <= 0.0)
                || exposure.is_none_or(|value| value <= 0.0)
                || !matches!(
                    exposure_unit,
                    "operating_hours" | "cycles" | "heating_cycles" | "compressor_hours"
                )
            {
                errors.push(format!(
                    "{episode_id}: completed follow-up requires a known recurrence result, positive calendar duration and positive product-use exposure"
                ));
            }
            if recurred == Some(true) {
                recurrence_count += 1;
            }
            if outcome == "verified_restoration" && recurred == Some(false) {
                durable_success_count += 1;
            }
        } else if !recurrence_is_null {
            errors.push(format!(
                "{episode_id}: censored, pending, lost or not-applicable follow-up cannot be recorded as no recurrence or recurrence"
            ));
        }
        if follow_status == "not_applicable"
            && (follow_days.is_some() || exposure.is_some() || exposure_unit != "not_applicable")
        {
            errors.push(format!(
                "{episode_id}: not_applicable follow-up must not invent a duration or exposure"
            ));
        }

        let Some(costs) = episode.get("costs") else {
            errors.push(format!("{episode_id}: costs object is required"));
            continue;
        };
        let cost_status = text_at(costs, "observation_status").unwrap_or("");
        let currency = text_at(costs, "currency").unwrap_or("");
        let known_cost_count = COST_FIELDS
            .iter()
            .filter(|field| costs.get(**field).is_some_and(|value| !value.is_null()))
            .count();
        match cost_status {
            "complete" if known_cost_count != COST_FIELDS.len() => errors.push(format!(
                "{episode_id}: complete cost record contains unknown cost fields"
            )),
            "partial" if known_cost_count == 0 || known_cost_count == COST_FIELDS.len() => {
                errors.push(format!(
                    "{episode_id}: partial costs must contain both known and unknown values"
                ));
            }
            "unknown" if known_cost_count != 0 || currency != "unknown" => errors.push(format!(
                "{episode_id}: unknown costs must be null, never encoded as zero, with currency unknown"
            )),
            _ => {}
        }

        let preregistration = episode.get("preregistration").unwrap_or(&Value::Null);
        if bool_at(preregistration, "frozen_before_outcome") != Some(true) {
            errors.push(format!(
                "{episode_id}: protocol must be frozen before any eligible outcome"
            ));
        }
        let evidence_receipts = episode
            .get("evidence_receipt_ids")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if evidence_receipts == 0 {
            errors.push(format!(
                "{episode_id}: evidence receipt identity cannot be omitted"
            ));
        }
    }

    for expected in EXPECTED_EPISODES {
        if !episode_ids.contains(expected) {
            errors.push(format!("required synthetic episode {expected} is missing"));
        }
    }
    if empirical_count != 0 {
        errors.push("synthetic fixture must report zero empirical episodes".to_owned());
    }

    let Some(benchmarks) = ledger.get("proposed_benchmarks").and_then(Value::as_array) else {
        return Err(vec!["proposed_benchmarks must be an array".to_owned()]);
    };
    if benchmarks.len() != EXPECTED_BENCHMARKS.len() {
        errors.push(format!(
            "fixture must retain {} proposed benchmark definitions",
            EXPECTED_BENCHMARKS.len()
        ));
    }
    let mut benchmark_ids = BTreeSet::new();
    for (index, benchmark) in benchmarks.iter().enumerate() {
        let Some(id) = text_at(benchmark, "id") else {
            errors.push(format!("proposed_benchmarks[{index}].id must be a string"));
            continue;
        };
        if !benchmark_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate benchmark ID {id}"));
        }
        if EXPECTED_BENCHMARKS.get(index).copied() != Some(id) {
            errors.push(format!(
                "benchmark order/identity changed at index {index}: {id}"
            ));
        }
        if text_at(benchmark, "status").is_none_or(|status| {
            !matches!(status, "proposal_not_frozen" | "guardrail_contract_only")
        }) {
            errors.push(format!(
                "{id}: benchmark must remain a proposal or contract guardrail"
            ));
        }
    }
    for expected in EXPECTED_BENCHMARKS {
        if !benchmark_ids.contains(expected) {
            errors.push(format!("required benchmark {expected} is missing"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(RestorationAuditSummary {
        episode_count: episodes.len(),
        synthetic_episode_count: episodes
            .iter()
            .filter(|episode| text_at(episode, "evidence_class") == Some("synthetic_fixture"))
            .count(),
        empirical_episode_count: empirical_count,
        started_attempt_count,
        verified_restoration_example_count: verified_restoration_count,
        completed_followup_example_count: completed_followup_count,
        recurrence_example_count: recurrence_count,
        durable_success_example_count: durable_success_count,
        benchmark_count: benchmarks.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        ledger_sha256: format!("{:x}", Sha256::digest(ledger_raw.as_bytes())),
    })
}

pub(crate) fn audit_restoration_texts(
    schema_raw: &str,
    ledger_raw: &str,
) -> Result<RestorationAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("restoration schema JSON parse failed: {error}")])?;
    let ledger: Value = serde_json::from_str(ledger_raw)
        .map_err(|error| vec![format!("restoration ledger JSON parse failed: {error}")])?;
    audit_values(&schema, &ledger, schema_raw, ledger_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str =
        include_str!("../../../schemas/appliances/repair_restoration_evidence_v0_1.schema.json");
    const LEDGER: &str =
        include_str!("../../../fixtures/appliances/repair_restoration_evidence_v0_1.json");

    fn canonical_ledger() -> Value {
        serde_json::from_str(LEDGER).expect("restoration fixture must parse")
    }

    fn audit_mutation(ledger: &Value) -> Result<RestorationAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("restoration schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let ledger_raw = serde_json::to_string(ledger).expect("ledger serializes");
        audit_values(&schema, ledger, &schema_raw, &ledger_raw)
    }

    #[test]
    fn canonical_ledger_counts_attempts_separately_from_durable_success() {
        let summary = audit_restoration_texts(SCHEMA, LEDGER)
            .expect("canonical synthetic restoration ledger should qualify");
        assert_eq!(summary.episode_count, 5);
        assert_eq!(summary.synthetic_episode_count, 5);
        assert_eq!(summary.empirical_episode_count, 0);
        assert_eq!(summary.started_attempt_count, 4);
        assert_eq!(summary.verified_restoration_example_count, 3);
        assert_eq!(summary.completed_followup_example_count, 2);
        assert_eq!(summary.recurrence_example_count, 1);
        assert_eq!(summary.durable_success_example_count, 1);
        assert_eq!(summary.benchmark_count, 7);
    }

    #[test]
    fn synthetic_episode_cannot_be_promoted_to_empirical_evidence() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["empirical_eligible"] = json!(true);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn failed_started_attempt_cannot_disappear_from_denominator() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][1]["intervention"]["repair_attempt_started"] = json!(false);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn verified_restoration_requires_both_hard_gates_and_test_receipts() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["verification"]["safety_gate"] = json!("unknown");
        assert!(audit_mutation(&ledger).is_err());
        ledger = canonical_ledger();
        ledger["episodes"][0]["verification"]["measurement_receipt_ids"] = json!([]);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn energy_or_water_claim_cannot_compensate_for_failed_core_function() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][1]["verification"]["energy_water_improvement_claimed"] = json!(true);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn censored_or_lost_followup_cannot_be_encoded_as_no_recurrence() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][3]["follow_up"]["same_fault_recurred"] = json!(false);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn unknown_cost_is_null_not_zero() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][2]["costs"]["parts_minor"] = json!(0);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn failed_function_test_cannot_be_called_a_verified_restoration() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["verification"]["passed_test_count"] = json!(2);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn duplicate_episode_identity_is_rejected() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][1]["episode_id"] = json!("SYN-REST-001");
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn duplicate_or_promoted_benchmark_identity_is_rejected() {
        let mut ledger = canonical_ledger();
        ledger["proposed_benchmarks"][1]["id"] = json!("RB-001");
        assert!(audit_mutation(&ledger).is_err());
        ledger = canonical_ledger();
        ledger["proposed_benchmarks"][0]["status"] = json!("validated_winner");
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn complete_followup_requires_actual_exposure_not_just_calendar_time() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["follow_up"]["exposure_value"] = json!(0);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn unknown_costs_require_every_cost_to_remain_null() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][2]["costs"]["observation_status"] = json!("partial");
        assert!(audit_mutation(&ledger).is_err());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/sourcing.rs:

use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:washer-parts-resilience-snapshot:0.1.0";
const SNAPSHOT_ID: &str = "WASHER-SOURCING-SNAPSHOT-001";
const CANDIDATE_IDS: [&str; 4] = [
    "DEFY-DAW408",
    "HISENSE-WF1I8022BT",
    "SAMSUNG-WW80T3040BS-FA",
    "MIDEA-MF110W80B-T",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SourcingAuditSummary {
    pub(crate) candidate_count: usize,
    pub(crate) price_observation_count: usize,
    pub(crate) part_probe_count: usize,
    pub(crate) model_fit_confirmed_probe_count: usize,
    pub(crate) selected_control_count: usize,
    pub(crate) purchased_candidate_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) snapshot_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("sourcing schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("sourcing schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "sourcing schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile sourcing schema: {error}"))
}

fn audit_values(
    schema: &Value,
    snapshot: &Value,
    schema_raw: &str,
    snapshot_raw: &str,
) -> Result<SourcingAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(snapshot)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(snapshot, "schema_version") != Some("0.1.0") {
        errors.push("sourcing snapshot schema_version must remain 0.1.0".to_owned());
    }
    if text_at(snapshot, "snapshot_id") != Some(SNAPSHOT_ID) {
        errors.push(format!(
            "sourcing snapshot identity must remain {SNAPSHOT_ID}"
        ));
    }
    if text_at(snapshot, "target_market") != Some("ZA") {
        errors.push("sourcing snapshot market must remain ZA until explicitly reviewed".to_owned());
    }
    if text_at(snapshot, "status") != Some("desk_research_not_purchased")
        || text_at(snapshot, "claim_ceiling")
            != Some("no_control_selected_no_physical_test_no_parts_fit_claim")
    {
        errors.push(
            "sourcing snapshot cannot claim a selected control, purchase, physical test or verified part fit"
                .to_owned(),
        );
    }

    let selection = snapshot.get("selection_rule").unwrap_or(&Value::Null);
    if selection
        .get("minimum_physical_control_units")
        .and_then(Value::as_u64)
        .is_none_or(|n| n < 3)
        || selection
            .get("control_units_same_model_revision")
            .and_then(Value::as_bool)
            != Some(true)
        || selection
            .get("three_brands_may_be_pooled_as_n")
            .and_then(Value::as_bool)
            != Some(false)
        || selection
            .get("donor_is_valid_control")
            .and_then(Value::as_bool)
            != Some(false)
        || text_at(selection, "missing_data_policy") != Some("unknown_not_zero")
    {
        errors.push(
            "selection rule must preserve a >=3 same-model/revision control pilot, prohibit pooled brands/donor-as-control, and keep unknown data unknown"
                .to_owned(),
        );
    }

    let Some(candidates) = snapshot
        .get("candidate_shortlist")
        .and_then(Value::as_array)
    else {
        return Err(vec!["candidate_shortlist must be an array".to_owned()]);
    };
    if candidates.len() != CANDIDATE_IDS.len() {
        errors.push(format!(
            "reviewed desk snapshot must contain {} model leads; got {}",
            CANDIDATE_IDS.len(),
            candidates.len()
        ));
    }
    let mut seen_ids = BTreeSet::new();
    let mut price_observation_count = 0usize;
    let mut selected_control_count = 0usize;
    let mut purchased_candidate_count = 0usize;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(id) = text_at(candidate, "id") else {
            errors.push(format!("candidate_shortlist[{index}].id must be a string"));
            continue;
        };
        if !seen_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate candidate ID {id}"));
        }
        if CANDIDATE_IDS.get(index).copied() != Some(id) {
            errors.push(format!(
                "candidate_shortlist[{index}] must remain {:?}; got {id}",
                CANDIDATE_IDS.get(index)
            ));
        }
        if text_at(candidate, "purchase_state") != Some("not_purchased") {
            purchased_candidate_count += 1;
            errors.push(format!(
                "{id}: purchase state cannot change without reviewed procurement evidence"
            ));
        }
        if text_at(candidate, "control_state") != Some("not_selected") {
            selected_control_count += 1;
            errors.push(format!(
                "{id}: no control can be selected from a desk snapshot alone"
            ));
        }
        if candidate.get("capacity_kg").and_then(Value::as_f64) != Some(8.0) {
            errors.push(format!(
                "{id}: this reviewed shortlist is the 8 kg sourcing cohort; changing capacity requires review"
            ));
        }
        let Some(observations) = candidate
            .get("price_observations")
            .and_then(Value::as_array)
        else {
            errors.push(format!("{id}: price_observations must be an array"));
            continue;
        };
        if observations.is_empty() {
            errors.push(format!(
                "{id}: at least one dated price-discovery observation is required"
            ));
        }
        price_observation_count += observations.len();
        for (price_index, observation) in observations.iter().enumerate() {
            if text_at(observation, "evidence_state") != Some("public_snapshot_not_a_quote") {
                errors.push(format!(
                    "{id}.price_observations[{price_index}]: public listing cannot be relabelled a quote or purchase"
                ));
            }
            if text_at(observation, "currency") != Some("ZAR") {
                errors.push(format!(
                    "{id}.price_observations[{price_index}]: currency must be ZAR for this snapshot"
                ));
            }
            if observation
                .get("amount_minor")
                .and_then(Value::as_u64)
                .is_none_or(|amount| amount == 0)
            {
                errors.push(format!(
                    "{id}.price_observations[{price_index}]: price must be a positive integer in minor currency units"
                ));
            }
        }
        let docs = candidate.get("model_documentation").unwrap_or(&Value::Null);
        if text_at(docs, "service_manual_url").is_some() {
            // A service-manual URL may be recorded as evidence, but it still needs
            // a human check that it is applicable and lawfully accessible.
            let url = text_at(docs, "service_manual_url").unwrap_or_default();
            if !url.starts_with("https://") {
                errors.push(format!(
                    "{id}: service_manual_url must be HTTPS when present"
                ));
            }
        }
    }
    for expected in CANDIDATE_IDS {
        if !seen_ids.contains(expected) {
            errors.push(format!("required candidate lead {expected} is missing"));
        }
    }

    let Some(probes) = snapshot
        .get("parts_catalogue_probes")
        .and_then(Value::as_array)
    else {
        return Err(vec!["parts_catalogue_probes must be an array".to_owned()]);
    };
    let mut part_probe_ids = BTreeSet::new();
    let mut model_fit_confirmed_probe_count = 0usize;
    for (index, probe) in probes.iter().enumerate() {
        let Some(id) = text_at(probe, "id") else {
            errors.push(format!(
                "parts_catalogue_probes[{index}].id must be a string"
            ));
            continue;
        };
        if !part_probe_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate part probe ID {id}"));
        }
        let fit = text_at(probe, "exact_model_fit_state");
        let evidence = text_at(probe, "evidence_state");
        if fit == Some("confirmed_with_documented_fit") {
            model_fit_confirmed_probe_count += 1;
            if text_at(probe, "compatibility_evidence_url").is_none() {
                errors.push(format!(
                    "{id}: a confirmed fit requires a model/revision-specific compatibility evidence URL"
                ));
            }
        } else if evidence == Some("installed_and_verified") {
            errors.push(format!(
                "{id}: cannot claim installed_and_verified while exact-model fit is unconfirmed"
            ));
        }
        if evidence == Some("public_catalogue_listing_only")
            && fit == Some("confirmed_with_documented_fit")
        {
            errors.push(format!(
                "{id}: a generic public catalogue listing cannot by itself confirm exact-model fit"
            ));
        }
    }

    let Some(targets) = snapshot.get("proposed_targets").and_then(Value::as_array) else {
        return Err(vec!["proposed_targets must be an array".to_owned()]);
    };
    let mut target_ids = BTreeSet::new();
    for target in targets {
        let Some(id) = text_at(target, "id") else {
            errors.push("proposed target id must be a string".to_owned());
            continue;
        };
        if !target_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate proposed target {id}"));
        }
        if text_at(target, "status") != Some("proposal_not_result") {
            errors.push(format!(
                "{id}: sourcing benchmark target must remain proposal_not_result"
            ));
        }
    }
    let evidence_rules = snapshot.get("evidence_rules").and_then(Value::as_array);
    if evidence_rules.is_none_or(|rules| {
        rules
            .iter()
            .filter_map(Value::as_str)
            .any(|rule| rule.to_ascii_lowercase().contains("unknown equals zero"))
            || !rules.iter().any(|rule| {
                rule.as_str()
                    .is_some_and(|text| text.contains("Three different brands cannot be pooled"))
            })
    }) {
        errors.push(
            "evidence rules must preserve unknown-not-zero and the same-design control interpretation"
                .to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(SourcingAuditSummary {
        candidate_count: candidates.len(),
        price_observation_count,
        part_probe_count: probes.len(),
        model_fit_confirmed_probe_count,
        selected_control_count,
        purchased_candidate_count,
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        snapshot_sha256: format!("{:x}", Sha256::digest(snapshot_raw.as_bytes())),
    })
}

pub(crate) fn audit_sourcing_texts(
    schema_raw: &str,
    snapshot_raw: &str,
) -> Result<SourcingAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("sourcing schema JSON parse failed: {error}")])?;
    let snapshot: Value = serde_json::from_str(snapshot_raw)
        .map_err(|error| vec![format!("sourcing snapshot JSON parse failed: {error}")])?;
    audit_values(&schema, &snapshot, schema_raw, snapshot_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/washer_parts_resilience_snapshot_v0_1.schema.json"
    );
    const SNAPSHOT: &str =
        include_str!("../../../fixtures/appliances/washer_parts_resilience_snapshot_v0_1.json");

    fn canonical_snapshot() -> Value {
        serde_json::from_str(SNAPSHOT).expect("sourcing snapshot JSON must parse")
    }

    fn audit_mutation(snapshot: &Value) -> Result<SourcingAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("sourcing schema JSON must parse");
        let schema_raw = serde_json::to_string(&schema).expect("serialize schema");
        let snapshot_raw = serde_json::to_string(snapshot).expect("serialize snapshot");
        audit_values(&schema, snapshot, &schema_raw, &snapshot_raw)
    }

    #[test]
    fn canonical_snapshot_audits_as_desk_research_only() {
        let summary = audit_sourcing_texts(SCHEMA, SNAPSHOT)
            .expect("canonical sourcing snapshot must qualify synthetically");
        assert_eq!(summary.candidate_count, 4);
        assert_eq!(summary.price_observation_count, 7);
        assert_eq!(summary.part_probe_count, 1);
        assert_eq!(summary.model_fit_confirmed_probe_count, 0);
        assert_eq!(summary.selected_control_count, 0);
        assert_eq!(summary.purchased_candidate_count, 0);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.snapshot_sha256.len(), 64);
    }

    #[test]
    fn duplicate_candidate_identity_is_rejected() {
        let mut snapshot = canonical_snapshot();
        snapshot["candidate_shortlist"][1]["id"] = json!("DEFY-DAW408");
        let errors = audit_mutation(&snapshot).expect_err("duplicate candidate ID must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate candidate ID"))
        );
    }

    #[test]
    fn a_desk_listing_cannot_select_a_control() {
        let mut snapshot = canonical_snapshot();
        snapshot["candidate_shortlist"][0]["control_state"] = json!("selected_control");
        let errors = audit_mutation(&snapshot).expect_err("desk listing cannot be selected");
        assert!(!errors.is_empty());
    }

    #[test]
    fn price_discovery_cannot_be_promoted_to_a_quote_or_purchase() {
        let mut snapshot = canonical_snapshot();
        snapshot["candidate_shortlist"][0]["price_observations"][0]["evidence_state"] =
            json!("written_quote");
        let errors = audit_mutation(&snapshot).expect_err("price evidence promotion must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn generic_catalogue_entry_cannot_become_verified_installed_part_fit() {
        let mut snapshot = canonical_snapshot();
        snapshot["parts_catalogue_probes"][0]["evidence_state"] = json!("installed_and_verified");
        let errors = audit_mutation(&snapshot).expect_err("generic listing cannot confirm fit");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("cannot claim installed_and_verified"))
        );
    }

    #[test]
    fn sourcing_target_cannot_be_relabelled_as_achieved() {
        let mut snapshot = canonical_snapshot();
        snapshot["proposed_targets"][0]["status"] = json!("achieved");
        let errors = audit_mutation(&snapshot).expect_err("proposal cannot become result");
        assert!(!errors.is_empty());
    }

    #[test]
    fn three_models_cannot_replace_three_repeatability_controls() {
        let mut snapshot = canonical_snapshot();
        snapshot["selection_rule"]["three_brands_may_be_pooled_as_n"] = json!(true);
        let errors = audit_mutation(&snapshot).expect_err("pooled brands must fail");
        assert!(!errors.is_empty());
    }

    #[test]
    fn confirmed_part_fit_requires_compatibility_evidence() {
        let mut snapshot = canonical_snapshot();
        snapshot["parts_catalogue_probes"][0]["exact_model_fit_state"] =
            json!("confirmed_with_documented_fit");
        let errors = audit_mutation(&snapshot).expect_err("confirmed fit needs explicit evidence");
        assert!(!errors.is_empty());
    }
}
/home/runner/work/symtropy/symtropy/tools/household-appliance-bench-audit/src/stakeholder.rs:

use jsonschema::Validator;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:repairability-architecture-decision-engine:0.1.0";
const EXPECTED_PROFILES: [&str; 5] = [
    "P1-HOUSEHOLD-OWNER",
    "P2-INDEPENDENT-TECHNICIAN",
    "P3-PARTS-AND-SERVICE-NETWORK",
    "P4-LIFECYCLE-MATERIAL-STEWARD",
    "P5-REMOTE-INSTITUTIONAL-OPERATOR",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StakeholderAuditSummary {
    pub(crate) profile_count: usize,
    pub(crate) requirement_count: usize,
    pub(crate) frozen_requirement_count: usize,
    pub(crate) profile_ready_count: usize,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn criterion_by_id<'a>(criteria: &'a [Value], id: &str) -> Option<&'a Value> {
    criteria
        .iter()
        .find(|criterion| text_at(criterion, "id") == Some(id))
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("decision schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("decision schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "decision schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile decision schema: {error}"))
}

fn observation_by_id<'a>(candidate: &'a Value, criterion_id: &str) -> Option<&'a Value> {
    candidate
        .get("observations")?
        .as_array()?
        .iter()
        .find(|observation| text_at(observation, "criterion_id") == Some(criterion_id))
}

fn candidate_evidence_status(decision: &Value, candidate: &Value) -> &'static str {
    let Some(gates) = candidate.get("hard_gates").and_then(Value::as_array) else {
        return "blocked_unknown_gate";
    };
    let mut seen_pass = BTreeSet::new();
    for gate in gates {
        let Some(id) = text_at(gate, "gate_id") else {
            return "blocked_unknown_gate";
        };
        if text_at(gate, "status") == Some("fail") {
            return "rejected_hard_gate";
        }
        if text_at(gate, "status") != Some("pass")
            || text_at(gate, "evidence_id").is_none_or(str::is_empty)
            || text_at(gate, "evidence_id")
                .unwrap_or("")
                .starts_with("SYNTHETIC-")
        {
            return "blocked_unknown_gate";
        }
        if !seen_pass.insert(id.to_owned()) {
            return "blocked_unknown_gate";
        }
    }
    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return "insufficient_eligible_evidence";
    };
    if criteria.is_empty() {
        return "insufficient_eligible_evidence";
    }
    for criterion in criteria {
        let Some(id) = text_at(criterion, "id") else {
            return "insufficient_eligible_evidence";
        };
        if text_at(criterion, "status") != Some("method_frozen") {
            return "insufficient_eligible_evidence";
        }
        let Some(observation) = observation_by_id(candidate, id) else {
            return "insufficient_eligible_evidence";
        };
        if !matches!(
            text_at(observation, "evidence_class"),
            Some("physical_measurement" | "independent_replication")
        ) || observation.get("method_frozen").and_then(Value::as_bool) != Some(true)
            || text_at(observation, "unit") != text_at(criterion, "unit")
            || text_at(observation, "evidence_id").is_none_or(str::is_empty)
            || text_at(observation, "method_id").is_none_or(str::is_empty)
            || text_at(observation, "evidence_id")
                .unwrap_or("")
                .starts_with("SYNTHETIC-")
        {
            return "insufficient_eligible_evidence";
        }
        let (Some(lower), Some(upper)) = (
            observation.get("lower").and_then(Value::as_f64),
            observation.get("upper").and_then(Value::as_f64),
        ) else {
            return "insufficient_eligible_evidence";
        };
        if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || lower > upper {
            return "insufficient_eligible_evidence";
        }
        if matches!(
            id,
            "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
        ) && upper > 1.0
        {
            return "insufficient_eligible_evidence";
        }
    }
    "eligible_for_profile_comparison"
}

fn candidate_meets_profile_requirements(
    decision: &Value,
    profile: &Value,
    candidate: &Value,
) -> &'static str {
    if text_at(profile, "status") != Some("requirements_frozen") {
        return "requirements_not_frozen";
    }
    let Some(requirements) = profile.get("requirements").and_then(Value::as_array) else {
        return "requirements_not_frozen";
    };
    for requirement in requirements {
        if text_at(requirement, "status") != Some("frozen_before_candidate_measurements")
            || requirement
                .get("frozen_before_candidate_measurements")
                .and_then(Value::as_bool)
                != Some(true)
        {
            return "requirements_not_frozen";
        }
        let Some(threshold) = requirement.get("threshold").and_then(Value::as_f64) else {
            return "requirements_not_frozen";
        };
        let Some(criterion_id) = text_at(requirement, "criterion_id") else {
            return "requirements_not_frozen";
        };
        let Some(_criterion) =
            decision
                .get("criteria")
                .and_then(Value::as_array)
                .and_then(|criteria| {
                    criteria
                        .iter()
                        .find(|criterion| text_at(criterion, "id") == Some(criterion_id))
                })
        else {
            return "requirements_not_frozen";
        };
        let Some(observation) = observation_by_id(candidate, criterion_id) else {
            return "insufficient_eligible_evidence";
        };
        let (Some(lower), Some(upper)) = (
            observation.get("lower").and_then(Value::as_f64),
            observation.get("upper").and_then(Value::as_f64),
        ) else {
            return "insufficient_eligible_evidence";
        };
        match text_at(requirement, "direction") {
            Some("minimize") if upper <= threshold => {}
            Some("minimize") if lower > threshold => return "fails_profile_constraint",
            Some("maximize") if lower >= threshold => {}
            Some("maximize") if upper < threshold => return "fails_profile_constraint",
            Some("minimize" | "maximize") => return "profile_constraint_unresolved",
            _ => return "requirements_not_frozen",
        }
    }
    "meets_profile_constraints"
}

fn robustly_dominates_for_profile(decision: &Value, profile: &Value, a: &Value, b: &Value) -> bool {
    let Some(ids) = profile.get("criteria").and_then(Value::as_array) else {
        return false;
    };
    let mut strict_improvement = false;
    for id_value in ids {
        let Some(id) = id_value.as_str() else {
            return false;
        };
        let Some(criterion) =
            decision
                .get("criteria")
                .and_then(Value::as_array)
                .and_then(|criteria| {
                    criteria
                        .iter()
                        .find(|criterion| text_at(criterion, "id") == Some(id))
                })
        else {
            return false;
        };
        let (Some(a_obs), Some(b_obs)) = (observation_by_id(a, id), observation_by_id(b, id))
        else {
            return false;
        };
        let (Some(a_lower), Some(a_upper), Some(b_lower), Some(b_upper)) = (
            a_obs.get("lower").and_then(Value::as_f64),
            a_obs.get("upper").and_then(Value::as_f64),
            b_obs.get("lower").and_then(Value::as_f64),
            b_obs.get("upper").and_then(Value::as_f64),
        ) else {
            return false;
        };
        match text_at(criterion, "direction") {
            Some("minimize") => {
                if a_upper > b_lower {
                    return false;
                }
                strict_improvement |= a_upper < b_lower;
            }
            Some("maximize") => {
                if a_lower < b_upper {
                    return false;
                }
                strict_improvement |= a_lower > b_upper;
            }
            _ => return false,
        }
    }
    strict_improvement
}

fn audit_values(
    schema: &Value,
    decision: &Value,
) -> Result<(StakeholderAuditSummary, Value), Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(decision)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    let profile_policy = decision.get("profile_policy").unwrap_or(&Value::Null);
    if text_at(profile_policy, "profile_policy_id")
        != Some("STAKEHOLDER-REPAIRABILITY-PROFILES-001")
        || profile_policy
            .get("hard_gate_override")
            .and_then(Value::as_bool)
            != Some(false)
        || profile_policy
            .get("selection_before_evidence")
            .and_then(Value::as_bool)
            != Some(false)
        || text_at(profile_policy, "threshold_policy") != Some("unfrozen_threshold_remains_unknown")
        || text_at(profile_policy, "weighting_policy") != Some("no_hidden_global_weights")
    {
        errors.push(
            "stakeholder profiles cannot override hard gates, select before evidence, or invent hidden weights"
                .to_owned(),
        );
    }

    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return Err(vec!["decision criteria must be an array".to_owned()]);
    };
    let Some(profiles) = decision
        .get("stakeholder_profiles")
        .and_then(Value::as_array)
    else {
        return Err(vec!["stakeholder_profiles must be an array".to_owned()]);
    };
    if profiles.len() != EXPECTED_PROFILES.len() {
        errors.push(format!(
            "decision must contain {} reviewed stakeholder profiles",
            EXPECTED_PROFILES.len()
        ));
    }

    let mut profile_ids = BTreeSet::new();
    let mut total_requirements = 0usize;
    let mut frozen_requirements = 0usize;
    let mut profile_results = Vec::new();

    for (index, profile) in profiles.iter().enumerate() {
        let Some(profile_id) = text_at(profile, "id") else {
            errors.push(format!("stakeholder_profiles[{index}].id must be a string"));
            continue;
        };
        if !profile_ids.insert(profile_id.to_owned()) {
            errors.push(format!("duplicate stakeholder profile {profile_id}"));
        }
        if EXPECTED_PROFILES.get(index).copied() != Some(profile_id) {
            errors.push(format!(
                "stakeholder profile order must preserve {:?}; got {profile_id}",
                EXPECTED_PROFILES.get(index)
            ));
        }

        let profile_criteria: BTreeSet<&str> = profile
            .get("criteria")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if profile_criteria.is_empty() {
            errors.push(format!(
                "{profile_id}: at least one mapped criterion is required"
            ));
        }
        for criterion_id in &profile_criteria {
            if criterion_by_id(criteria, criterion_id).is_none() {
                errors.push(format!(
                    "{profile_id}: unknown criterion mapping {criterion_id}"
                ));
            }
        }

        let Some(requirements) = profile.get("requirements").and_then(Value::as_array) else {
            errors.push(format!("{profile_id}: requirements must be an array"));
            continue;
        };
        total_requirements += requirements.len();
        let mut req_ids = BTreeSet::new();
        let mut all_frozen = !requirements.is_empty();

        for requirement in requirements {
            let Some(req_id) = text_at(requirement, "id") else {
                errors.push(format!("{profile_id}: requirement ID must be a string"));
                all_frozen = false;
                continue;
            };
            if !req_ids.insert(req_id.to_owned()) {
                errors.push(format!("{profile_id}: duplicate requirement ID {req_id}"));
            }
            let Some(criterion_id) = text_at(requirement, "criterion_id") else {
                errors.push(format!("{profile_id}.{req_id}: criterion_id is required"));
                all_frozen = false;
                continue;
            };
            let Some(criterion) = criterion_by_id(criteria, criterion_id) else {
                errors.push(format!(
                    "{profile_id}.{req_id}: unknown criterion {criterion_id}"
                ));
                all_frozen = false;
                continue;
            };
            if !profile_criteria.contains(criterion_id) {
                errors.push(format!(
                    "{profile_id}.{req_id}: requirement must map to a declared profile criterion"
                ));
            }
            if text_at(requirement, "direction") != text_at(criterion, "direction")
                || text_at(requirement, "unit") != text_at(criterion, "unit")
            {
                errors.push(format!(
                    "{profile_id}.{req_id}: direction and unit must match the canonical criterion"
                ));
            }

            let threshold = requirement.get("threshold").unwrap_or(&Value::Null);
            let frozen = text_at(requirement, "status")
                == Some("frozen_before_candidate_measurements")
                && requirement
                    .get("frozen_before_candidate_measurements")
                    .and_then(Value::as_bool)
                    == Some(true)
                && threshold
                    .as_f64()
                    .is_some_and(|value| value.is_finite() && value >= 0.0);
            let unfrozen = text_at(requirement, "status") == Some("threshold_not_frozen")
                && requirement
                    .get("frozen_before_candidate_measurements")
                    .and_then(Value::as_bool)
                    == Some(false)
                && threshold.is_null();
            if frozen
                && matches!(
                    criterion_id,
                    "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
                )
                && threshold.as_f64().is_some_and(|value| value > 1.0)
            {
                errors.push(format!(
                    "{profile_id}.{req_id}: proportion thresholds must not exceed 1.0"
                ));
            }
            if frozen {
                frozen_requirements += 1;
            } else if !unfrozen {
                errors.push(format!(
                    "{profile_id}.{req_id}: threshold value, status and preregistration flag disagree"
                ));
            }
            all_frozen &= frozen;
        }

        let status = text_at(profile, "status");
        if status == Some("requirements_frozen") && !all_frozen {
            errors.push(format!(
                "{profile_id}: cannot freeze a profile while a requirement is unfrozen"
            ));
        }
        if status == Some("requirements_not_frozen") && all_frozen {
            errors.push(format!(
                "{profile_id}: profile status disagrees with frozen requirements"
            ));
        }

        if profile_id == "P1-HOUSEHOLD-OWNER" {
            let boundary = text_at(profile, "required_task_boundary")
                .unwrap_or("")
                .to_ascii_lowercase();
            if !boundary.contains("no opening internal mains")
                || !boundary.contains("refrigerant")
                || boundary.contains("may open internal mains")
                || boundary.contains("may perform refrigerant charging")
                || boundary.contains("refrigerant work is permitted")
            {
                errors.push(
                    "P1 household task boundary must explicitly prohibit hazardous internal mains/refrigerant work"
                        .to_owned(),
                );
            }
        }

        let candidates = decision
            .get("candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut candidate_assessments = Vec::new();
        let mut qualifying_candidates = Vec::new();
        for candidate in &candidates {
            let universal = candidate_evidence_status(decision, candidate);
            let candidate_status = if universal != "eligible_for_profile_comparison" {
                universal
            } else {
                candidate_meets_profile_requirements(decision, profile, candidate)
            };
            if candidate_status == "meets_profile_constraints" {
                qualifying_candidates.push(candidate.clone());
            }
            candidate_assessments.push(json!({
                "candidate_id": text_at(candidate, "id").unwrap_or("unknown"),
                "status": candidate_status,
                "robust_pareto_frontier": false,
                "robustly_dominated_by": []
            }));
        }
        let mut frontier_count = 0usize;
        for candidate in &qualifying_candidates {
            let dominators: Vec<String> = qualifying_candidates
                .iter()
                .filter(|other| {
                    text_at(other, "id") != text_at(candidate, "id")
                        && robustly_dominates_for_profile(decision, profile, other, candidate)
                })
                .filter_map(|other| text_at(other, "id").map(str::to_owned))
                .collect();
            let is_frontier = dominators.is_empty();
            if is_frontier {
                frontier_count += 1;
            }
            if let Some(outcome) = candidate_assessments
                .iter_mut()
                .find(|outcome| text_at(outcome, "candidate_id") == text_at(candidate, "id"))
            {
                outcome["robust_pareto_frontier"] = json!(is_frontier);
                outcome["robustly_dominated_by"] = json!(dominators);
            }
        }
        profile_results.push(json!({
            "profile_id": profile_id,
            "status": status.unwrap_or("unknown"),
            "criteria_count": profile_criteria.len(),
            "requirement_count": requirements.len(),
            "frozen_requirement_count": requirements.iter().filter(|requirement| {
                text_at(requirement, "status") == Some("frozen_before_candidate_measurements")
                    && requirement.get("threshold").and_then(Value::as_f64).is_some()
                    && requirement.get("frozen_before_candidate_measurements").and_then(Value::as_bool) == Some(true)
            }).count(),
            "candidate_assessments": candidate_assessments,
            "candidate_meeting_constraints_count": qualifying_candidates.len(),
            "profile_pareto_frontier_count": frontier_count,
            "profile_frontier_status": if status == Some("requirements_frozen") {
                "evaluated_only_when_universal_gates_and_candidate_evidence_are_eligible"
            } else {
                "requirements_not_frozen"
            },
            "profile_specific_selection_claim_permitted": false
        }));
    }
    for expected in EXPECTED_PROFILES {
        if !profile_ids.contains(expected) {
            errors.push(format!(
                "required stakeholder profile {expected} is missing"
            ));
        }
    }
    if total_requirements != 18 {
        errors.push(format!(
            "reviewed stakeholder requirement census must remain 18; got {total_requirements}"
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let summary = StakeholderAuditSummary {
        profile_count: profiles.len(),
        requirement_count: total_requirements,
        frozen_requirement_count: frozen_requirements,
        profile_ready_count: profiles
            .iter()
            .filter(|profile| text_at(profile, "status") == Some("requirements_frozen"))
            .count(),
    };
    let report = json!({
        "policy": "stakeholder_constraints_without_hidden_global_weights",
        "profile_count": summary.profile_count,
        "requirement_count": summary.requirement_count,
        "frozen_requirement_count": summary.frozen_requirement_count,
        "profile_ready_count": summary.profile_ready_count,
        "profile_specific_selection_claim_permitted": false,
        "profiles": profile_results
    });
    Ok((summary, report))
}

pub(crate) fn audit_stakeholder_texts(
    schema_raw: &str,
    decision_raw: &str,
) -> Result<(StakeholderAuditSummary, Value), Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("decision schema JSON parse failed: {error}")])?;
    let decision: Value = serde_json::from_str(decision_raw)
        .map_err(|error| vec![format!("decision profile JSON parse failed: {error}")])?;
    audit_values(&schema, &decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/repairability_architecture_decision_v0_1.schema.json"
    );
    const DECISION: &str =
        include_str!("../../../fixtures/appliances/repairability_architecture_decision_v0_1.json");

    fn canonical_decision() -> Value {
        serde_json::from_str(DECISION).expect("decision profile must parse")
    }

    fn synthetic_frozen_decision() -> Value {
        let mut decision = canonical_decision();
        for criterion in decision["criteria"].as_array_mut().expect("criteria") {
            criterion["status"] = json!("method_frozen");
        }
        for profile in decision["stakeholder_profiles"]
            .as_array_mut()
            .expect("profiles")
        {
            profile["status"] = json!("requirements_frozen");
            for requirement in profile["requirements"]
                .as_array_mut()
                .expect("requirements")
            {
                requirement["status"] = json!("frozen_before_candidate_measurements");
                requirement["frozen_before_candidate_measurements"] = json!(true);
                requirement["threshold"] = if text_at(requirement, "direction") == Some("minimize")
                {
                    json!(100000.0)
                } else {
                    json!(0.0)
                };
            }
        }
        let criteria = decision["criteria"].as_array().expect("criteria").clone();
        for (candidate_index, candidate) in decision["candidates"]
            .as_array_mut()
            .expect("candidates")
            .iter_mut()
            .enumerate()
        {
            for gate in candidate["hard_gates"].as_array_mut().expect("gates") {
                gate["status"] = json!("pass");
                gate["evidence_id"] = json!(format!("LAB-RECEIPT-W{candidate_index}"));
            }
            let mut observations = Vec::new();
            for (criterion_index, criterion) in criteria.iter().enumerate() {
                let id = text_at(criterion, "id").unwrap_or("unknown");
                let direction = text_at(criterion, "direction").unwrap_or("minimize");
                let value = if direction == "minimize" {
                    (candidate_index * 10 + criterion_index + 2) as f64
                } else {
                    0.95 - candidate_index as f64 * 0.1
                };
                observations.push(json!({
                    "criterion_id": id,
                    "lower": value,
                    "upper": value,
                    "unit": text_at(criterion, "unit").unwrap_or("unknown"),
                    "evidence_class": "physical_measurement",
                    "evidence_id": format!("LAB-RECEIPT-W{candidate_index}-{id}"),
                    "method_id": format!("FROZEN-METHOD-{id}"),
                    "method_frozen": true,
                    "uncertainty_note": "Synthetic logic test fixture; not physical appliance evidence."
                }));
            }
            candidate["observations"] = json!(observations);
        }
        decision
    }

    fn audit_mutation(decision: &Value) -> Result<(StakeholderAuditSummary, Value), Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema must parse");
        audit_values(&schema, decision)
    }

    #[test]
    fn canonical_profiles_keep_all_thresholds_unknown() {
        let (summary, report) = audit_stakeholder_texts(SCHEMA, DECISION)
            .expect("stakeholder profile fixture must qualify");
        assert_eq!(summary.profile_count, 5);
        assert_eq!(summary.requirement_count, 18);
        assert_eq!(summary.frozen_requirement_count, 0);
        assert_eq!(summary.profile_ready_count, 0);
        assert_eq!(
            report["profile_specific_selection_claim_permitted"],
            json!(false)
        );
        for profile in report["profiles"].as_array().expect("profiles") {
            assert_eq!(profile["status"], "requirements_not_frozen");
            assert_eq!(
                profile["profile_specific_selection_claim_permitted"],
                json!(false)
            );
        }
    }

    #[test]
    fn frozen_profiles_apply_constraints_then_compute_separate_frontiers() {
        let decision = synthetic_frozen_decision();
        let raw = serde_json::to_string(&decision).unwrap();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let (summary, report) = audit_values(&schema, &decision)
            .expect("synthetic stakeholder logic fixture must validate");
        assert_eq!(summary.profile_ready_count, 5);
        assert_eq!(summary.frozen_requirement_count, 18);
        assert_eq!(
            report["profile_specific_selection_claim_permitted"],
            json!(false)
        );
        for profile in report["profiles"].as_array().expect("profiles") {
            assert_eq!(profile["status"], "requirements_frozen");
            assert_eq!(profile["candidate_meeting_constraints_count"], json!(5));
            assert_eq!(profile["profile_pareto_frontier_count"], json!(1));
            assert_eq!(
                profile["profile_specific_selection_claim_permitted"],
                json!(false)
            );
        }
        assert!(!raw.is_empty());
    }

    #[test]
    fn overlapping_uncertainty_at_profile_threshold_is_unresolved() {
        let mut decision = synthetic_frozen_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["threshold"] = json!(3.0);
        decision["candidates"][0]["observations"][0]["lower"] = json!(2.0);
        decision["candidates"][0]["observations"][0]["upper"] = json!(4.0);
        let raw = serde_json::to_string(&decision).unwrap();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let (_, report) = audit_values(&schema, &decision)
            .expect("overlapping profile interval must remain valid evidence");
        assert_eq!(
            report["profiles"][0]["candidate_assessments"][0]["status"],
            json!("profile_constraint_unresolved")
        );
        assert!(!raw.is_empty());
    }

    #[test]
    fn unmet_profile_threshold_is_reported_without_global_score() {
        let mut decision = synthetic_frozen_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["threshold"] = json!(0.0);
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let (_, report) =
            audit_values(&schema, &decision).expect("profile threshold is a valid constraint");
        assert_eq!(
            report["profiles"][0]["candidate_assessments"][0]["status"],
            json!("fails_profile_constraint")
        );
        assert_eq!(
            report["policy"],
            json!("stakeholder_constraints_without_hidden_global_weights")
        );
    }

    #[test]
    fn duplicate_stakeholder_profile_is_rejected() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][1]["id"] = json!("P1-HOUSEHOLD-OWNER");
        let errors = audit_mutation(&decision).expect_err("duplicate profile must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate stakeholder profile"))
        );
    }

    #[test]
    fn unfrozen_threshold_cannot_be_filled_with_a_numeric_value() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["threshold"] = json!(0.0);
        let errors = audit_mutation(&decision).expect_err("unfrozen value must remain null");
        assert!(errors.iter().any(|error| {
            error.contains("threshold value, status and preregistration flag disagree")
        }));
    }

    #[test]
    fn household_profile_cannot_expand_into_hazardous_diy_work() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][0]["required_task_boundary"] =
            json!("DIY users may open internal mains and may perform refrigerant charging.");
        let errors = audit_mutation(&decision).expect_err("hazardous user scope must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("household task boundary"))
        );
    }

    #[test]
    fn profile_requirement_cannot_redefine_criterion_units() {
        let mut decision = canonical_decision();
        decision["stakeholder_profiles"][0]["requirements"][0]["unit"] = json!("invented_units");
        let errors = audit_mutation(&decision).expect_err("profile units must be canonical");
        assert!(
            errors
                .iter()
                .any(|error| { error.contains("direction and unit must match") })
        );
    }

    #[test]
    fn profiles_cannot_override_universal_hard_gates() {
        let mut decision = canonical_decision();
        decision["profile_policy"]["hard_gate_override"] = json!(true);
        let errors = audit_mutation(&decision).expect_err("profile hard-gate override must fail");
        assert!(!errors.is_empty());
    }
}

use jsonschema::Validator;
use serde_json::{Value, json};
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
        return Err("sourcing schema does not validate against the Draft 2020-12 meta-schema".to_owned());
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
        errors.push(format!("sourcing snapshot identity must remain {SNAPSHOT_ID}"));
    }
    if text_at(snapshot, "target_market") != Some("ZA") {
        errors.push("sourcing snapshot market must remain ZA until explicitly reviewed".to_owned());
    }
    if text_at(snapshot, "status") != Some("desk_research_not_purchased")
        || text_at(snapshot, "claim_ceiling")
            != Some("no_control_selected_no_physical_test_no_parts_fit_claim")
    {
        errors.push("sourcing snapshot cannot claim a selected control, purchase, physical test or verified part fit".to_owned());
    }

    let selection = snapshot.get("selection_rule").unwrap_or(&Value::Null);
    if selection.get("minimum_physical_control_units").and_then(Value::as_u64).is_none_or(|n| n < 3)
        || selection.get("control_units_same_model_revision").and_then(Value::as_bool) != Some(true)
        || selection.get("three_brands_may_be_pooled_as_n").and_then(Value::as_bool) != Some(false)
        || selection.get("donor_is_valid_control").and_then(Value::as_bool) != Some(false)
        || text_at(selection, "missing_data_policy") != Some("unknown_not_zero")
    {
        errors.push("selection rule must preserve a >=3 same-model/revision control pilot, prohibit pooled brands/donor-as-control, and keep unknown data unknown".to_owned());
    }

    let Some(candidates) = snapshot.get("candidate_shortlist").and_then(Value::as_array) else {
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
            errors.push(format!("{id}: purchase state cannot change without reviewed procurement evidence"));
        }
        if text_at(candidate, "control_state") != Some("not_selected") {
            selected_control_count += 1;
            errors.push(format!("{id}: no control can be selected from a desk snapshot alone"));
        }
        if candidate.get("capacity_kg").and_then(Value::as_f64) != Some(8.0) {
            errors.push(format!("{id}: this reviewed shortlist is the 8 kg sourcing cohort; changing capacity requires review"));
        }
        let Some(observations) = candidate.get("price_observations").and_then(Value::as_array) else {
            errors.push(format!("{id}: price_observations must be an array"));
            continue;
        };
        if observations.is_empty() {
            errors.push(format!("{id}: at least one dated price-discovery observation is required"));
        }
        price_observation_count += observations.len();
        for (price_index, observation) in observations.iter().enumerate() {
            if text_at(observation, "evidence_state") != Some("public_snapshot_not_a_quote") {
                errors.push(format!("{id}.price_observations[{price_index}]: public listing cannot be relabelled a quote or purchase"));
            }
            if text_at(observation, "currency") != Some("ZAR") {
                errors.push(format!("{id}.price_observations[{price_index}]: currency must be ZAR for this snapshot"));
            }
            if observation.get("amount_minor").and_then(Value::as_u64).is_none_or(|amount| amount == 0) {
                errors.push(format!("{id}.price_observations[{price_index}]: price must be a positive integer in minor currency units"));
            }
        }
        let docs = candidate.get("model_documentation").unwrap_or(&Value::Null);
        if text_at(docs, "service_manual_url").is_some() {
            // A service-manual URL may be recorded as evidence, but it still needs
            // a human check that it is applicable and lawfully accessible.
            let url = text_at(docs, "service_manual_url").unwrap_or_default();
            if !url.starts_with("https://") {
                errors.push(format!("{id}: service_manual_url must be HTTPS when present"));
            }
        }
    }
    for expected in CANDIDATE_IDS {
        if !seen_ids.contains(expected) {
            errors.push(format!("required candidate lead {expected} is missing"));
        }
    }

    let Some(probes) = snapshot.get("parts_catalogue_probes").and_then(Value::as_array) else {
        return Err(vec!["parts_catalogue_probes must be an array".to_owned()]);
    };
    let mut part_probe_ids = BTreeSet::new();
    let mut model_fit_confirmed_probe_count = 0usize;
    for (index, probe) in probes.iter().enumerate() {
        let Some(id) = text_at(probe, "id") else {
            errors.push(format!("parts_catalogue_probes[{index}].id must be a string"));
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
                errors.push(format!("{id}: a confirmed fit requires a model/revision-specific compatibility evidence URL"));
            }
        } else if evidence == Some("installed_and_verified") {
            errors.push(format!("{id}: cannot claim installed_and_verified while exact-model fit is unconfirmed"));
        }
        if evidence == Some("public_catalogue_listing_only") && fit == Some("confirmed_with_documented_fit") {
            errors.push(format!("{id}: a generic public catalogue listing cannot by itself confirm exact-model fit"));
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
            errors.push(format!("{id}: sourcing benchmark target must remain proposal_not_result"));
        }
    }
    let evidence_rules = snapshot.get("evidence_rules").and_then(Value::as_array);
    if evidence_rules.is_none_or(|rules| {
        rules.iter().filter_map(Value::as_str).any(|rule| rule.to_ascii_lowercase().contains("unknown equals zero"))
            || !rules.iter().any(|rule| rule.contains("Three different brands cannot be pooled"))
    }) {
        errors.push("evidence rules must preserve unknown-not-zero and the same-design control interpretation".to_owned());
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

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/washer_parts_resilience_snapshot_v0_1.schema.json"
    );
    const SNAPSHOT: &str = include_str!(
        "../../../fixtures/appliances/washer_parts_resilience_snapshot_v0_1.json"
    );

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
        assert!(errors.iter().any(|error| error.contains("duplicate candidate ID")));
    }

    #[test]
    fn a_desk_listing_cannot_select_a_control() {
        let mut snapshot = canonical_snapshot();
        snapshot["candidate_shortlist"][0]["control_state"] = json!("selected_control");
        let errors = audit_mutation(&snapshot).expect_err("desk listing cannot be selected");
        assert!(errors.iter().any(|error| error.contains("no control can be selected")));
    }

    #[test]
    fn price_discovery_cannot_be_promoted_to_a_quote_or_purchase() {
        let mut snapshot = canonical_snapshot();
        snapshot["candidate_shortlist"][0]["price_observations"][0]["evidence_state"] =
            json!("written_quote");
        let errors = audit_mutation(&snapshot).expect_err("price evidence promotion must fail");
        assert!(errors.iter().any(|error| error.contains("cannot be relabelled a quote")));
    }

    #[test]
    fn generic_catalogue_entry_cannot_become_verified_installed_part_fit() {
        let mut snapshot = canonical_snapshot();
        snapshot["parts_catalogue_probes"][0]["evidence_state"] = json!("installed_and_verified");
        let errors = audit_mutation(&snapshot).expect_err("generic listing cannot confirm fit");
        assert!(errors.iter().any(|error| error.contains("cannot claim installed_and_verified")));
    }

    #[test]
    fn sourcing_target_cannot_be_relabelled_as_achieved() {
        let mut snapshot = canonical_snapshot();
        snapshot["proposed_targets"][0]["status"] = json!("achieved");
        let errors = audit_mutation(&snapshot).expect_err("proposal cannot become result");
        assert!(errors.iter().any(|error| error.contains("must remain proposal_not_result")));
    }

    #[test]
    fn three_models_cannot_replace_three_repeatability_controls() {
        let mut snapshot = canonical_snapshot();
        snapshot["selection_rule"]["three_brands_may_be_pooled_as_n"] = json!(true);
        let errors = audit_mutation(&snapshot).expect_err("pooled brands must fail");
        assert!(errors.iter().any(|error| error.contains("same-model/revision control pilot")));
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

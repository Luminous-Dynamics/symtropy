//! Read-only structural audit for the Global Manufacturing Deployment Profile v0.1 corpus.
//!
//! This tool checks fixture identity, cross-profile core consistency, separation of
//! jurisdiction dimensions, synthetic evidence boundaries, and required adversarial
//! controls. It does not interpret law, calculate origin, certify a site, or authorize
//! production.
use serde_json::{Value, json};
use std::collections::BTreeSet;

const REQUIRED_DIMENSIONS: [&str; 7] = [
    "manufacturing_site_jurisdictions",
    "destination_market_jurisdictions",
    "supplier_site_jurisdictions",
    "component_origin_jurisdictions",
    "transformation_jurisdictions",
    "data_processing_jurisdictions",
    "service_jurisdictions",
];

const REQUIRED_CONTROLS: [&str; 20] = [
    "proposal_as_law",
    "wrong_destination_market",
    "wrong_product_category",
    "expired_source_as_current",
    "false_mutual_recognition",
    "supplier_location_as_origin",
    "unknown_origin_as_eligible",
    "unknown_as_satisfied",
    "data_processing_jurisdiction_omitted",
    "silent_global_core_change",
    "unknown_customs_classification_as_eligible",
    "origin_preference_without_evidence",
    "responsible_party_mismatch",
    "effective_window_mismatch",
    "local_content_claim_without_method",
    "obligation_dimension_mismatch",
    "supplier_site_jurisdiction_omitted",
    "transformation_jurisdiction_missing",
    "jurisdiction_level_mismatch",
    "translation_status_without_artifact",
];

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path}: expected object"))
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    object(value, path)?
        .get(key)
        .ok_or_else(|| format!("{path}.{key}: missing field"))
}

fn string<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{path}: expected non-empty string"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{path}: expected array"))
}

fn require_string(value: &Value, key: &str, path: &str, expected: &str) -> Result<(), String> {
    let actual = string(field(value, key, path)?, &format!("{path}.{key}"))?;
    if actual != expected {
        return Err(format!("{path}.{key}: expected {expected}, got {actual}"));
    }
    Ok(())
}

fn require_not(value: &Value, key: &str, path: &str, forbidden: &str) -> Result<(), String> {
    let actual = string(field(value, key, path)?, &format!("{path}.{key}"))?;
    if actual == forbidden {
        return Err(format!(
            "{path}.{key}: forbidden synthetic promotion {forbidden}"
        ));
    }
    Ok(())
}

fn require_dimension_member(
    context: &Value,
    dimension: &str,
    jurisdiction: &str,
    path: &str,
) -> Result<(), String> {
    let values = array(
        field(context, dimension, path)?,
        &format!("{path}.{dimension}"),
    )?;
    let found = values
        .iter()
        .any(|value| value.as_str() == Some(jurisdiction));
    if !found {
        return Err(format!(
            "{path}.{dimension}: jurisdiction {jurisdiction} is not declared in this dimension"
        ));
    }
    Ok(())
}

fn obligation_dimension_field(dimension: &str) -> Option<&'static str> {
    match dimension {
        "manufacturing_site" => Some("manufacturing_site_jurisdictions"),
        "destination_market" => Some("destination_market_jurisdictions"),
        "supplier_site" => Some("supplier_site_jurisdictions"),
        "component_origin" => Some("component_origin_jurisdictions"),
        "transformation" => Some("transformation_jurisdictions"),
        "data_processing" => Some("data_processing_jurisdictions"),
        "service_lifecycle" => Some("service_jurisdictions"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoreIdentity {
    product_family: String,
    revision: String,
    engineering_core_digest: String,
    process_definition_refs: Vec<String>,
    inspection_requirement_refs: Vec<String>,
}

fn sorted_unique_string_array(value: &Value, path: &str) -> Result<Vec<String>, String> {
    let values = array(value, path)?;
    if values.is_empty() {
        return Err(format!("{path}: empty reference list"));
    }
    let mut refs = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let reference = string(item, &format!("{path}[{index}]"))?;
        if !refs.insert(reference.to_owned()) {
            return Err(format!("{path}: duplicate reference {reference}"));
        }
    }
    Ok(refs.into_iter().collect())
}

fn validate_profile(profile: &Value, index: usize) -> Result<(String, CoreIdentity), String> {
    let path = format!("profiles[{index}]");
    let profile_id = string(
        field(profile, "profile_id", &path)?,
        &format!("{path}.profile_id"),
    )?
    .to_owned();
    require_string(profile, "profile_class", &path, "synthetic_archetype")?;

    let context = field(profile, "deployment_context", &path)?;
    for dimension in REQUIRED_DIMENSIONS {
        let values = array(
            field(context, dimension, &format!("{path}.deployment_context"))?,
            &format!("{path}.deployment_context.{dimension}"),
        )?;
        if values.is_empty() {
            return Err(format!(
                "{path}.deployment_context.{dimension}: empty jurisdiction dimension"
            ));
        }
        let mut dimension_ids = BTreeSet::new();
        for (j, value) in values.iter().enumerate() {
            let id = string(
                value,
                &format!("{path}.deployment_context.{dimension}[{j}]"),
            )?;
            if !dimension_ids.insert(id.to_owned()) {
                return Err(format!(
                    "{path}.deployment_context.{dimension}: duplicate jurisdiction {id}"
                ));
            }
            if !id.starts_with("urn:luminous:synthetic:jurisdiction:") {
                return Err(format!(
                    "{path}.deployment_context.{dimension}[{j}]: real jurisdiction not permitted in synthetic corpus"
                ));
            }
        }
    }

    let core = field(profile, "global_core", &path)?;
    let product_family = string(
        field(core, "product_family_id", &format!("{path}.global_core"))?,
        &format!("{path}.global_core.product_family_id"),
    )?
    .to_owned();
    let revision = string(
        field(core, "product_revision", &format!("{path}.global_core"))?,
        &format!("{path}.global_core.product_revision"),
    )?
    .to_owned();
    let digest = string(
        field(
            core,
            "engineering_core_digest",
            &format!("{path}.global_core"),
        )?,
        &format!("{path}.global_core.engineering_core_digest"),
    )?
    .to_owned();
    if digest != format!("sha256:{}", "0".repeat(64)) {
        return Err(format!(
            "{path}.global_core.engineering_core_digest: synthetic sentinel required"
        ));
    }
    let process_definition_refs = sorted_unique_string_array(
        field(
            core,
            "process_definition_refs",
            &format!("{path}.global_core"),
        )?,
        &format!("{path}.global_core.process_definition_refs"),
    )?;
    let inspection_requirement_refs = sorted_unique_string_array(
        field(
            core,
            "inspection_requirement_refs",
            &format!("{path}.global_core"),
        )?,
        &format!("{path}.global_core.inspection_requirement_refs"),
    )?;
    let core_identity = CoreIdentity {
        product_family,
        revision,
        engineering_core_digest: digest,
        process_definition_refs,
        inspection_requirement_refs,
    };

    let site = field(profile, "manufacturing_site", &path)?;
    let site_jurisdiction = string(
        field(
            site,
            "site_jurisdiction",
            &format!("{path}.manufacturing_site"),
        )?,
        &format!("{path}.manufacturing_site.site_jurisdiction"),
    )?;
    require_dimension_member(
        context,
        "manufacturing_site_jurisdictions",
        site_jurisdiction,
        &format!("{path}.deployment_context"),
    )?;
    require_string(
        site,
        "site_evidence_class",
        &format!("{path}.manufacturing_site"),
        "synthetic_fixture",
    )?;

    let market_access = array(
        field(profile, "market_access", &path)?,
        &format!("{path}.market_access"),
    )?;
    if market_access.is_empty() {
        return Err(format!(
            "{path}.market_access: at least one destination market required"
        ));
    }
    let mut market_ids = BTreeSet::new();
    for (j, market) in market_access.iter().enumerate() {
        let mpath = format!("{path}.market_access[{j}]");
        let destination = string(
            field(market, "destination_jurisdiction", &mpath)?,
            &format!("{mpath}.destination_jurisdiction"),
        )?;
        require_dimension_member(
            context,
            "destination_market_jurisdictions",
            destination,
            &format!("{path}.deployment_context"),
        )?;
        if !market_ids.insert(destination.to_owned()) {
            return Err(format!(
                "{path}.market_access: duplicate destination {destination}"
            ));
        }
        require_not(
            market,
            "approval_status",
            &mpath,
            "independently_established",
        )?;
        require_not(market, "responsible_operator_status", &mpath, "authorized")?;
        require_string(market, "approval_route_class", &mpath, "unknown")?;
    }

    let supply = field(profile, "supply_chain", &path)?;
    require_string(
        supply,
        "origin_claims",
        &format!("{path}.supply_chain"),
        "not_established",
    )?;
    let components = array(
        field(supply, "components", &format!("{path}.supply_chain"))?,
        &format!("{path}.supply_chain.components"),
    )?;
    if components.is_empty() {
        return Err(format!(
            "{path}.supply_chain.components: empty component set"
        ));
    }
    let mut component_ids = BTreeSet::new();
    for (j, component) in components.iter().enumerate() {
        let cpath = format!("{path}.supply_chain.components[{j}]");
        let component_id = string(
            field(component, "component_ref", &cpath)?,
            &format!("{cpath}.component_ref"),
        )?;
        if !component_ids.insert(component_id.to_owned()) {
            return Err(format!(
                "{path}.supply_chain.components: duplicate component {component_id}"
            ));
        }
        let supplier_site = string(
            field(component, "supplier_site_jurisdiction", &cpath)?,
            &format!("{cpath}.supplier_site_jurisdiction"),
        )?;
        require_dimension_member(
            context,
            "supplier_site_jurisdictions",
            supplier_site,
            &format!("{path}.deployment_context"),
        )?;
        let material_origin = string(
            field(component, "material_origin_jurisdiction", &cpath)?,
            &format!("{cpath}.material_origin_jurisdiction"),
        )?;
        require_dimension_member(
            context,
            "component_origin_jurisdictions",
            material_origin,
            &format!("{path}.deployment_context"),
        )?;
        let transformation_sites = array(
            field(component, "transformation_jurisdictions", &cpath)?,
            &format!("{cpath}.transformation_jurisdictions"),
        )?;
        if transformation_sites.is_empty() {
            return Err(format!("{cpath}.transformation_jurisdictions: empty list"));
        }
        let mut component_transformation_ids = BTreeSet::new();
        for (k, value) in transformation_sites.iter().enumerate() {
            let jurisdiction =
                string(value, &format!("{cpath}.transformation_jurisdictions[{k}]"))?;
            if !component_transformation_ids.insert(jurisdiction.to_owned()) {
                return Err(format!(
                    "{cpath}.transformation_jurisdictions: duplicate jurisdiction {jurisdiction}"
                ));
            }
            require_dimension_member(
                context,
                "transformation_jurisdictions",
                jurisdiction,
                &format!("{path}.deployment_context"),
            )?;
        }
        require_not(
            component,
            "origin_evidence_status",
            &cpath,
            "independently_verified",
        )?;
        require_string(
            component,
            "customs_classification_status",
            &cpath,
            "unknown",
        )?;
        require_string(component, "preference_claim_status", &cpath, "unknown")?;
        require_string(component, "local_content_status", &cpath, "unknown")?;
        for key in [
            "customs_classification_code",
            "origin_rule_ref",
            "local_content_method_ref",
        ] {
            if !field(component, key, &cpath)?.is_null() {
                return Err(format!(
                    "{cpath}.{key}: synthetic fixture cannot claim customs/origin/local-content evidence"
                ));
            }
        }
    }

    let data = field(profile, "data_processing", &path)?;
    let data_jurisdictions = array(
        field(
            data,
            "processing_jurisdictions",
            &format!("{path}.data_processing"),
        )?,
        &format!("{path}.data_processing.processing_jurisdictions"),
    )?;
    if data_jurisdictions.is_empty() {
        return Err(format!(
            "{path}.data_processing.processing_jurisdictions: empty list"
        ));
    }
    for (j, value) in data_jurisdictions.iter().enumerate() {
        let jurisdiction = string(
            value,
            &format!("{path}.data_processing.processing_jurisdictions[{j}]"),
        )?;
        require_dimension_member(
            context,
            "data_processing_jurisdictions",
            jurisdiction,
            &format!("{path}.deployment_context"),
        )?;
    }
    require_not(
        data,
        "data_classification_status",
        &format!("{path}.data_processing"),
        "reviewed",
    )?;
    require_not(
        data,
        "transfer_review_status",
        &format!("{path}.data_processing"),
        "reviewed",
    )?;

    let service = field(profile, "service_lifecycle", &path)?;
    let service_jurisdictions = array(
        field(
            service,
            "service_jurisdictions",
            &format!("{path}.service_lifecycle"),
        )?,
        &format!("{path}.service_lifecycle.service_jurisdictions"),
    )?;
    if service_jurisdictions.is_empty() {
        return Err(format!(
            "{path}.service_lifecycle.service_jurisdictions: empty list"
        ));
    }
    for (j, value) in service_jurisdictions.iter().enumerate() {
        let jurisdiction = string(
            value,
            &format!("{path}.service_lifecycle.service_jurisdictions[{j}]"),
        )?;
        require_dimension_member(
            context,
            "service_jurisdictions",
            jurisdiction,
            &format!("{path}.deployment_context"),
        )?;
    }
    for (key, forbidden) in [
        ("critical_parts_status", "available_and_verified"),
        ("repair_capability_status", "verified_for_scope"),
        ("end_of_life_status", "approved_route_verified"),
    ] {
        require_not(
            service,
            key,
            &format!("{path}.service_lifecycle"),
            forbidden,
        )?;
    }

    let obligations = array(
        field(profile, "obligations", &path)?,
        &format!("{path}.obligations"),
    )?;
    let mut obligation_ids = BTreeSet::new();
    for (j, obligation) in obligations.iter().enumerate() {
        let opath = format!("{path}.obligations[{j}]");
        let obligation_id = string(
            field(obligation, "obligation_id", &opath)?,
            &format!("{opath}.obligation_id"),
        )?;
        if !obligation_ids.insert(obligation_id.to_owned()) {
            return Err(format!(
                "{path}.obligations: duplicate obligation {obligation_id}"
            ));
        }
        let jurisdiction = string(
            field(obligation, "jurisdiction", &opath)?,
            &format!("{opath}.jurisdiction"),
        )?;
        let declared_dimension = string(
            field(obligation, "jurisdiction_dimension", &opath)?,
            &format!("{opath}.jurisdiction_dimension"),
        )?;
        let dimension_field = obligation_dimension_field(declared_dimension).ok_or_else(|| {
            format!("{opath}.jurisdiction_dimension: unsupported dimension {declared_dimension}")
        })?;
        require_dimension_member(
            context,
            dimension_field,
            jurisdiction,
            &format!("{path}.deployment_context"),
        )?;
        require_string(obligation, "jurisdiction_level", &opath, "synthetic")?;
        require_string(obligation, "source_status", &opath, "synthetic_fixture")?;
        require_string(obligation, "disposition", &opath, "unknown")?;
        require_string(obligation, "reviewer_status", &opath, "not_reviewed")?;
        require_string(obligation, "conformity_route", &opath, "unknown")?;
        let required_evidence = array(
            field(obligation, "required_evidence_refs", &opath)?,
            &format!("{opath}.required_evidence_refs"),
        )?;
        if !required_evidence.is_empty() {
            return Err(format!(
                "{opath}.required_evidence_refs: synthetic fixture cannot claim real evidence"
            ));
        }
        for key in ["responsible_party_ref", "reviewer_ref"] {
            if !field(obligation, key, &opath)?.is_null() {
                return Err(format!(
                    "{opath}.{key}: synthetic fixture cannot identify a real responsible party or reviewer"
                ));
            }
        }
        let source = field(obligation, "source", &opath)?;
        if string(
            field(source, "source_digest", &format!("{opath}.source"))?,
            &format!("{opath}.source.source_digest"),
        )? != "synthetic"
        {
            return Err(format!(
                "{opath}.source.source_digest: synthetic sentinel required"
            ));
        }
        require_string(
            source,
            "source_language",
            &format!("{opath}.source"),
            "undetermined",
        )?;
        require_string(
            source,
            "translation_status",
            &format!("{opath}.source"),
            "unknown",
        )?;
        require_string(
            source,
            "translation_language",
            &format!("{opath}.source"),
            "undetermined",
        )?;
        if !field(
            source,
            "translation_artifact_ref",
            &format!("{opath}.source"),
        )?
        .is_null()
        {
            return Err(format!(
                "{opath}.source.translation_artifact_ref: synthetic fixture cannot claim a real translation artifact"
            ));
        }
        for date_key in [
            "publication_date",
            "entry_into_force_date",
            "application_date",
            "source_checked_at",
            "transition_start_date",
            "transition_end_date",
            "expiry_date",
        ] {
            if !field(source, date_key, &format!("{opath}.source"))?.is_null() {
                return Err(format!(
                    "{opath}.source.{date_key}: dates must remain null in synthetic corpus"
                ));
            }
        }
        let applicability = field(obligation, "applicability", &opath)?;
        require_string(
            applicability,
            "applicability_status",
            &format!("{opath}.applicability"),
            "unknown",
        )?;
    }

    let claims = field(profile, "claims", &path)?;
    require_string(
        claims,
        "global_compliance_claim",
        &format!("{path}.claims"),
        "not_established",
    )?;
    require_string(
        claims,
        "site_readiness_claim",
        &format!("{path}.claims"),
        "not_established",
    )?;
    require_string(
        claims,
        "market_approval_claim",
        &format!("{path}.claims"),
        "not_established",
    )?;
    if field(
        claims,
        "production_authorization",
        &format!("{path}.claims"),
    )?
    .as_bool()
        != Some(false)
    {
        return Err(format!(
            "{path}.claims.production_authorization: must remain false"
        ));
    }

    Ok((profile_id, core_identity))
}

/// Validates the JSON Schema document and corpus against Draft 2020-12,
/// then applies the deployment-specific semantic audit.
pub fn audit_schema_and_corpus(schema_input: &str, corpus_input: &str) -> Result<Value, String> {
    let schema: Value =
        serde_json::from_str(schema_input).map_err(|e| format!("invalid schema JSON: {e}"))?;
    let corpus: Value =
        serde_json::from_str(corpus_input).map_err(|e| format!("invalid corpus JSON: {e}"))?;

    require_string(
        &schema,
        "$schema",
        "$",
        "https://json-schema.org/draft/2020-12/schema",
    )?;

    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|e| format!("Draft 2020-12 meta-schema validation failed: {e}"))?;

    let validator = jsonschema::draft202012::options()
        .offline()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|e| format!("could not compile Draft 2020-12 schema: {e}"))?;

    validator
        .validate(&corpus)
        .map_err(|e| format!("corpus violates Draft 2020-12 schema: {e}"))?;

    let mut report = audit_corpus(corpus_input)?;
    if let Some(map) = report.as_object_mut() {
        map.insert("json_schema_valid".to_owned(), Value::Bool(true));
        map.insert(
            "json_schema_draft".to_owned(),
            Value::String("2020-12".to_owned()),
        );
    }
    Ok(report)
}

/// Audits a corpus and returns a deterministic, synthetic-only structural report.
pub fn audit_corpus(input: &str) -> Result<Value, String> {
    let corpus: Value = serde_json::from_str(input).map_err(|e| format!("invalid JSON: {e}"))?;
    require_string(&corpus, "corpus_id", "$", "GLOBAL-MFG-DEPLOY-CORPUS-V0.1")?;
    require_string(&corpus, "schema_version", "$", "0.1.0")?;
    require_string(&corpus, "evidence_class", "$", "synthetic_fixture")?;

    let profiles = array(field(&corpus, "profiles", "$")?, "$.profiles")?;
    if profiles.len() < 4 {
        return Err("$.profiles: at least four synthetic archetypes required".to_owned());
    }

    let mut profile_ids = BTreeSet::new();
    let mut first_core: Option<CoreIdentity> = None;
    for (index, profile) in profiles.iter().enumerate() {
        let (id, core_identity) = validate_profile(profile, index)?;
        if !profile_ids.insert(id.clone()) {
            return Err(format!("duplicate profile_id: {id}"));
        }
        match &first_core {
            None => first_core = Some(core_identity),
            Some(expected_core) => {
                if &core_identity != expected_core {
                    return Err(format!(
                        "profiles[{index}]: global engineering core identity/digest/process/inspection references changed across regional realization"
                    ));
                }
            }
        }
    }

    let controls = array(
        field(&corpus, "negative_controls", "$")?,
        "$.negative_controls",
    )?;
    let mut mutations = BTreeSet::new();
    let mut case_ids = BTreeSet::new();
    for (index, control) in controls.iter().enumerate() {
        let path = format!("negative_controls[{index}]");
        let case_id = string(
            field(control, "case_id", &path)?,
            &format!("{path}.case_id"),
        )?;
        if !case_ids.insert(case_id.to_owned()) {
            return Err(format!("duplicate negative-control case_id: {case_id}"));
        }
        require_string(control, "expected_disposition", &path, "REJECT_OR_ABSTAIN")?;
        let mutation = string(
            field(control, "mutation", &path)?,
            &format!("{path}.mutation"),
        )?;
        mutations.insert(mutation.to_owned());
    }
    for required in REQUIRED_CONTROLS {
        if !mutations.contains(required) {
            return Err(format!("missing required negative control: {required}"));
        }
    }

    let core = first_core.ok_or_else(|| "no profiles".to_owned())?;
    Ok(json!({
        "corpus_id": "GLOBAL-MFG-DEPLOY-CORPUS-V0.1",
        "schema_version": "0.1.0",
        "profile_count": profiles.len(),
        "negative_control_count": controls.len(),
        "jurisdiction_dimension_count": REQUIRED_DIMENSIONS.len(),
        "shared_product_family_id": core.product_family,
        "shared_product_revision": core.revision,
        "shared_engineering_core_digest": core.engineering_core_digest,
        "shared_process_definition_ref_count": core.process_definition_refs.len(),
        "shared_inspection_requirement_ref_count": core.inspection_requirement_refs.len(),
        "jurisdiction_dimensions": REQUIRED_DIMENSIONS,
        "all_profile_claims_not_established": true,
        "production_authorization": false,
        "disposition": "STRUCTURALLY_VALID_SYNTHETIC_ONLY",
        "legal_or_market_approval": false,
        "physical_site_qualification": false
    }))
}

#[cfg(test)]
mod tests {
    use super::{REQUIRED_DIMENSIONS, audit_corpus, audit_schema_and_corpus};
    use serde_json::Value;

    const FIXTURE: &str =
        include_str!("../../../fixtures/global/global_mfg_deployment_profiles_v0_1.json");
    const SCHEMA: &str =
        include_str!("../../../schemas/global/global_mfg_deployment_profile_v0_1.schema.json");

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON parses")
    }

    #[test]
    fn validates_corpus_against_draft_2020_12_schema() {
        let report = audit_schema_and_corpus(SCHEMA, FIXTURE)
            .expect("schema and corpus pass Draft 2020-12 and semantic audit");
        assert_eq!(report["json_schema_valid"], true);
        assert_eq!(report["json_schema_draft"], "2020-12");
    }

    #[test]
    fn rejects_schema_invalid_missing_jurisdiction_dimension() {
        let mut fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        fixture["profiles"][0]["deployment_context"]
            .as_object_mut()
            .expect("deployment context")
            .remove("data_processing_jurisdictions");
        let encoded = serde_json::to_string(&fixture).expect("encode mutation");
        assert!(audit_schema_and_corpus(SCHEMA, &encoded).is_err());
    }

    #[test]
    fn rejects_external_schema_references_without_network_access() {
        let remote_schema = r#"{
            "$schema":"https://json-schema.org/draft/2020-12/schema",
            "$ref":"https://example.com/remote-schema.json"
        }"#;
        assert!(audit_schema_and_corpus(remote_schema, FIXTURE).is_err());
    }

    #[test]
    fn rejects_invalid_schema_document() {
        let invalid_schema = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"not-a-json-type"}"#;
        assert!(audit_schema_and_corpus(invalid_schema, FIXTURE).is_err());
    }

    #[test]
    fn rejects_customs_eligibility_from_unknown_classification() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["supply_chain"]["components"][0]["customs_classification_status"] =
            Value::String("verified".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_unsubstantiated_preference_claim() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["supply_chain"]["components"][0]["preference_claim_status"] =
            Value::String("verified_eligible".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_responsible_party_or_reviewer_promotion_in_synthetic_profile() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["obligations"][0]["responsible_party_ref"] =
            Value::String("urn:luminous:entity:unreviewed".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_out_of_window_source_evidence() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["obligations"][0]["source"]["expiry_date"] =
            Value::String("2026-01-01".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_local_content_claim_without_named_method() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["supply_chain"]["components"][0]["local_content_status"] =
            Value::String("verified".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_obligation_jurisdiction_in_the_wrong_dimension() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["obligations"][0]["jurisdiction_dimension"] =
            Value::String("destination_market".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_supplier_site_missing_from_supplier_dimension() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["supply_chain"]["components"][0]["supplier_site_jurisdiction"] =
            Value::String("urn:luminous:synthetic:jurisdiction:transform-1-1".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_transformation_site_missing_from_transformation_dimension() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["supply_chain"]["components"][0]["transformation_jurisdictions"]
            [0] = Value::String("urn:luminous:synthetic:jurisdiction:market-1".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_market_access_reference_not_in_destination_dimension() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["market_access"][0]["destination_jurisdiction"] =
            Value::String("urn:luminous:synthetic:jurisdiction:site-1".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_engineering_core_digest_drift_between_regions() {
        let mut fixture = fixture_value();
        fixture["profiles"][1]["global_core"]["engineering_core_digest"] =
            Value::String(format!("sha256:{}", "1".repeat(64)));
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_process_definition_reference_drift_between_regions() {
        let mut fixture = fixture_value();
        fixture["profiles"][1]["global_core"]["process_definition_refs"][0] =
            Value::String("urn:luminous:synthetic:process:different-process".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_inspection_requirement_reference_drift_between_regions() {
        let mut fixture = fixture_value();
        fixture["profiles"][1]["global_core"]["inspection_requirement_refs"][0] =
            Value::String("urn:luminous:synthetic:inspection:different-plan".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_data_processing_jurisdiction_not_in_data_axis() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["data_processing"]["processing_jurisdictions"][0] =
            Value::String("urn:luminous:synthetic:jurisdiction:site-1".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_service_jurisdiction_not_in_service_axis() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["service_lifecycle"]["service_jurisdictions"][0] =
            Value::String("urn:luminous:synthetic:jurisdiction:market-1".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_jurisdiction_level_promotion_in_synthetic_corpus() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["obligations"][0]["jurisdiction_level"] =
            Value::String("national".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_translation_claim_without_translation_artifact() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["obligations"][0]["source"]["translation_status"] =
            Value::String("official_translation".to_owned());
        fixture["profiles"][0]["obligations"][0]["source"]["translation_language"] =
            Value::String("en".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_duplicate_jurisdiction_in_one_dimension() {
        let mut fixture = fixture_value();
        let first =
            fixture["profiles"][0]["deployment_context"]["supplier_site_jurisdictions"][0].clone();
        fixture["profiles"][0]["deployment_context"]["supplier_site_jurisdictions"][1] = first;
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_duplicate_obligation_identity() {
        let mut fixture = fixture_value();
        let first = fixture["profiles"][0]["obligations"][0]["obligation_id"].clone();
        fixture["profiles"][0]["obligations"][1]["obligation_id"] = first;
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn report_is_invariant_to_input_array_order() {
        let expected = audit_corpus(FIXTURE).expect("canonical fixture audit");
        let mut fixture = fixture_value();
        fixture["profiles"]
            .as_array_mut()
            .expect("profiles")
            .reverse();
        fixture["negative_controls"]
            .as_array_mut()
            .expect("controls")
            .reverse();
        for profile in fixture["profiles"].as_array_mut().expect("profiles") {
            profile["obligations"]
                .as_array_mut()
                .expect("obligations")
                .reverse();
            profile["supply_chain"]["components"]
                .as_array_mut()
                .expect("components")
                .reverse();
            for dimension in REQUIRED_DIMENSIONS {
                profile["deployment_context"][dimension]
                    .as_array_mut()
                    .expect("dimension")
                    .reverse();
            }
        }
        let encoded = serde_json::to_string(&fixture).expect("encode permuted fixture");
        let actual = audit_corpus(&encoded).expect("permuted fixture audit");
        assert_eq!(expected, actual);
        assert_eq!(actual["jurisdiction_dimension_count"], 7);
    }

    #[test]
    fn accepts_frozen_synthetic_corpus() {
        let report = audit_corpus(FIXTURE).expect("synthetic corpus is structurally valid");
        assert_eq!(report["profile_count"], 4);
        assert_eq!(report["negative_control_count"], 20);
        assert_eq!(report["production_authorization"], false);
        assert_eq!(report["legal_or_market_approval"], false);
    }

    #[test]
    fn rejects_unknown_evidence_promoted_to_satisfied() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["obligations"][0]["disposition"] =
            Value::String("satisfied".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_synthetic_market_approval_claim() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["market_access"][0]["approval_status"] =
            Value::String("independently_established".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_global_core_drift_between_regions() {
        let mut fixture = fixture_value();
        fixture["profiles"][1]["global_core"]["product_revision"] =
            Value::String("0.2.0-synthetic".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_duplicate_profile_identity() {
        let mut fixture = fixture_value();
        let duplicate_id = fixture["profiles"][0]["profile_id"].clone();
        fixture["profiles"][1]["profile_id"] = duplicate_id;
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_a_missing_adversarial_control() {
        let mut fixture = fixture_value();
        fixture["negative_controls"]
            .as_array_mut()
            .expect("controls")
            .retain(|item| item["mutation"] != "false_mutual_recognition");
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }

    #[test]
    fn rejects_a_real_jurisdiction_inside_synthetic_corpus() {
        let mut fixture = fixture_value();
        fixture["profiles"][0]["deployment_context"]["destination_market_jurisdictions"][0] =
            Value::String("DE".to_owned());
        let encoded = serde_json::to_string(&fixture).expect("encode fixture");
        assert!(audit_corpus(&encoded).is_err());
    }
}

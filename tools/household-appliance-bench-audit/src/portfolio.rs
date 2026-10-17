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

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PortfolioAuditSummary {
    pub(crate) family_count: usize,
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
    require_text_at(
        schema,
        "$id",
        SCHEMA_ID,
        &mut errors,
    );

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
            errors.push(format!("{id}: at least two architecture hypotheses are required"));
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
            errors.push(format!("{id}: permitted user maintenance boundary is missing"));
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
                reference.as_str().is_none_or(|url| !url.starts_with("https://"))
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
        return Err(vec!["benchmark_policy.proposed_targets must be an array".to_owned()]);
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
    ] {
        if !reference_titles.contains(required_reference) {
            errors.push(format!("required source reference missing: {required_reference}"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(PortfolioAuditSummary {
        family_count: families.len(),
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
        assert_eq!(summary.proposed_target_count, 8);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.portfolio_sha256.len(), 64);
    }

    #[test]
    fn duplicate_family_identity_is_rejected() {
        let mut portfolio = canonical_portfolio();
        portfolio["families"][1]["id"] = json!("washing_machine");
        let errors = audit_mutation(&portfolio).expect_err("duplicate family must fail");
        assert!(errors.iter().any(|error| error.contains("duplicate appliance family id")));
    }

    #[test]
    fn first_large_appliance_cannot_change_without_explicit_review() {
        let mut portfolio = canonical_portfolio();
        portfolio["selection_policy"]["first_large_appliance"] = json!("heat_pump_hvac");
        let errors = audit_mutation(&portfolio).expect_err("unreviewed target change must fail");
        assert!(errors.iter().any(|error| error.contains("first_large_appliance")));
    }

    #[test]
    fn unmeasured_target_cannot_be_relabelled_as_result() {
        let mut portfolio = canonical_portfolio();
        portfolio["benchmark_policy"]["proposed_targets"][0]["status"] = json!("achieved");
        let errors = audit_mutation(&portfolio)
            .expect_err("proposed target must not become result");
        assert!(errors.iter().any(|error| error.contains("proposal_not_result")));
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

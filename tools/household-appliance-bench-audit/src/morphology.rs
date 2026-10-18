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

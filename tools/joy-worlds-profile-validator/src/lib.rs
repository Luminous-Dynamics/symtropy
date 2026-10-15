pub mod comparison;

use serde_json::Value;
use std::collections::HashSet;

fn nonempty(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn required_text(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> String {
    match nonempty(value.get(key)) {
        Some(text) => text.to_owned(),
        None => {
            errors.push(format!("{path}.{key}: required non-empty string"));
            String::new()
        }
    }
}

fn required_strings(
    value: &Value,
    key: &str,
    path: &str,
    minimum: usize,
    errors: &mut Vec<String>,
) -> Vec<String> {
    let field_path = format!("{path}.{key}");
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{field_path}: required array of non-empty strings"));
        return Vec::new();
    };

    if items.len() < minimum {
        errors.push(format!(
            "{field_path}: expected at least {minimum} item(s), found {}",
            items.len()
        ));
    }

    let mut output = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        match item.as_str().map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => output.push(text.to_owned()),
            None => errors.push(format!(
                "{field_path}[{index}]: expected non-empty string"
            )),
        }
    }
    output
}

fn required_array<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
    minimum: usize,
    errors: &mut Vec<String>,
) -> Option<&'a Vec<Value>> {
    let field_path = format!("{path}.{key}");
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{field_path}: required array"));
        return None;
    };
    if items.len() < minimum {
        errors.push(format!(
            "{field_path}: expected at least {minimum} item(s), found {}",
            items.len()
        ));
    }
    Some(items)
}

fn unique_id(
    value: &Value,
    path: &str,
    seen: &mut HashSet<String>,
    errors: &mut Vec<String>,
) -> String {
    let id = required_text(value, "id", path, errors);
    if !id.is_empty() && !seen.insert(id.clone()) {
        errors.push(format!("{path}.id: duplicate identifier '{id}'"));
    }
    id
}

fn validate_https_urls(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) {
    let urls = required_strings(value, key, path, 1, errors);
    for (index, url) in urls.iter().enumerate() {
        if !url.starts_with("https://") || url.chars().any(char::is_whitespace) {
            errors.push(format!(
                "{path}.{key}[{index}]: expected an https URL without whitespace"
            ));
        }
    }
}

fn contains_any(values: &[String], needles: &[&str]) -> bool {
    values.iter().any(|value| {
        let lower = value.to_lowercase();
        needles.iter().any(|needle| lower.contains(needle))
    })
}

fn require_tokens(values: &[String], required: &[&str], path: &str, errors: &mut Vec<String>) {
    for token in required {
        if !values.iter().any(|value| value == token) {
            errors.push(format!("{path}: required evidence/guard token '{token}' is missing"));
        }
    }
}

fn require_exact_sequence(values: &[String], expected: &[&str], path: &str, errors: &mut Vec<String>) {
    let matches = values.len() == expected.len()
        && values.iter().zip(expected.iter()).all(|(actual, expected)| actual.as_str() == *expected);
    if !matches {
        errors.push(format!("{path}: expected exact ordered sequence [{}]", expected.join(", ")));
    }
}

/// Validates the Joy Worlds experience-portfolio manifest structurally.
///
/// A successful result establishes manifest consistency only. It does not
/// establish feasibility, demand, safety, sustainability, or guest preference.
pub fn validate_portfolio_json(raw: &str) -> Result<(), Vec<String>> {
    let document: Value = serde_json::from_str(raw)
        .map_err(|error| vec![format!("JSON parse error: {error}")])?;
    let mut errors = Vec::new();

    for key in ["schema_version", "id", "status"] {
        required_text(&document, key, "$", &mut errors);
    }
    required_text(&document, "purpose", "$", &mut errors);
    required_strings(
        &document,
        "experience_intents",
        "$",
        1,
        &mut errors,
    );

    if let Some(contract) = document.get("constraint_policy") {
        required_strings(contract, "hard_constraints", "$.constraint_policy", 1, &mut errors);
        required_strings(contract, "soft_tradeoffs", "$.constraint_policy", 1, &mut errors);
        required_text(contract, "rule", "$.constraint_policy", &mut errors);
    } else {
        errors.push("$.constraint_policy: required object".to_owned());
    }
    let evidence_fields =
        required_strings(&document, "common_evidence_fields", "$", 1, &mut errors);
    require_tokens(
        &evidence_fields,
        &[
            "model_and_code_identity",
            "assumptions_and_units",
            "infeasibility_and_unknowns",
            "site_specific_provenance_refs",
            "capital_commitments_and_reversibility",
        ],
        "$.common_evidence_fields",
        &mut errors,
    );

    let mut candidate_ids = HashSet::new();
    if let Some(candidates) = required_array(&document, "candidates", "$", 1, &mut errors) {
        for (index, candidate) in candidates.iter().enumerate() {
            let path = format!("$.candidates[{index}]");
            let id = unique_id(candidate, &path, &mut candidate_ids, &mut errors);
            required_text(candidate, "name", &path, &mut errors);
            required_text(candidate, "design_hypothesis", &path, &mut errors);
            let modules = required_strings(candidate, "modules", &path, 1, &mut errors);
            required_strings(candidate, "intents", &path, 1, &mut errors);
            let dependencies = required_strings(candidate, "dependencies", &path, 1, &mut errors);
            required_strings(candidate, "key_unknowns", &path, 1, &mut errors);

            let water_related = contains_any(
                &modules,
                &["water", "aqua", "surf", "wave", "river", "coast", "bathing"],
            );
            if water_related
                && !contains_any(
                    &dependencies,
                    &["safety", "aquatic", "water_quality", "wellness_review"],
                )
            {
                errors.push(format!(
                    "{path}.dependencies: water-related candidate '{id}' must declare a water/activity safety or qualified review dependency"
                ));
            }
        }
    }

    if let Some(protocol) = document.get("evaluation_protocol") {
        let initial = required_strings(protocol, "initial_candidates", "$.evaluation_protocol", 1, &mut errors);
        for id in initial {
            if !candidate_ids.contains(&id) {
                errors.push(format!(
                    "$.evaluation_protocol.initial_candidates: unknown candidate '{id}'"
                ));
            }
        }
        required_strings(protocol, "scenario_factors", "$.evaluation_protocol", 1, &mut errors);
        required_strings(protocol, "required_comparisons", "$.evaluation_protocol", 1, &mut errors);
        required_strings(protocol, "required_controls", "$.evaluation_protocol", 1, &mut errors);
        required_strings(protocol, "outcomes_reported_separately", "$.evaluation_protocol", 1, &mut errors);
        required_text(protocol, "promotion_rule", "$.evaluation_protocol", &mut errors);
    } else {
        errors.push("$.evaluation_protocol: required object");
    }

    validate_https_urls(&document, "references", "$", &mut errors);
    finish(errors)
}

/// Validates the regional-adaptation profile set structurally and checks that
/// the comparison protocol references existing profiles.
pub fn validate_regional_profiles_json(raw: &str) -> Result<(), Vec<String>> {
    let document: Value = serde_json::from_str(raw)
        .map_err(|error| vec![format!("JSON parse error: {error}")])?;
    let mut errors = Vec::new();

    for key in ["schema_version", "artifact_id", "status", "governing_rule"] {
        required_text(&document, key, "$", &mut errors);
    }
    required_strings(&document, "non_claims", "$", 1, &mut errors);

    if let Some(shared) = document.get("shared_contract") {
        for key in [
            "site_identity",
            "climate_and_hazards",
            "resource_systems",
            "place_and_people",
            "operations",
        ] {
            required_strings(shared, key, "$.shared_contract", 1, &mut errors);
        }
        let assessment =
            required_strings(shared, "assessment", "$.shared_contract", 1, &mut errors);
        require_tokens(
            &assessment,
            &["model_identity", "baseline_and_counterfactual", "reversibility_and_stranded_asset_exposure"],
            "$.shared_contract.assessment",
            &mut errors,
        );
    } else {
        errors.push("$.shared_contract: required object".to_owned());
    }

    let mut profile_ids = HashSet::new();
    if let Some(profiles) = required_array(&document, "profiles", "$", 1, &mut errors) {
        for (index, profile) in profiles.iter().enumerate() {
            let path = format!("$.profiles[{index}]");
            let id = unique_id(profile, &path, &mut profile_ids, &mut errors);
            required_text(profile, "name", &path, &mut errors);
            required_strings(profile, "applies_to", &path, 1, &mut errors);
            required_strings(profile, "design_hypotheses", &path, 1, &mut errors);
            let drivers = required_strings(profile, "key_drivers_to_measure", &path, 1, &mut errors);
            let constraints = required_strings(profile, "hard_constraints", &path, 1, &mut errors);
            required_strings(profile, "closure_and_degraded_modes", &path, 1, &mut errors);
            required_strings(profile, "unknowns", &path, 1, &mut errors);
            let non_assumptions = required_strings(profile, "non_assumptions", &path, 1, &mut errors);

            // Targeted semantic guards for these explicitly named profile families.
            // These are completeness checks, not evidence that the listed risks
            // or controls are sufficient for any real site.
            let upper_id = id.to_uppercase();
            if upper_id.contains("COAST") {
                if !contains_any(&drivers, &["coast", "shore", "erosion", "sea-level", "storm surge"]) {
                    errors.push(format!(
                        "{path}.key_drivers_to_measure: coastal profile must name a coastal process/hazard"
                    ));
                }
                if !contains_any(&constraints, &["coastal", "shore", "beach", "erosion"]) {
                    errors.push(format!(
                        "{path}.hard_constraints: coastal profile must name a coastal/access constraint"
                    ));
                }
            }
            if upper_id.contains("SKI") {
                if !contains_any(&drivers, &["snow"]) {
                    errors.push(format!(
                        "{path}.key_drivers_to_measure: ski profile must explicitly model snow reliability"
                    ));
                }
                if !contains_any(&constraints, &["avalanche"]) {
                    errors.push(format!(
                        "{path}.hard_constraints: ski profile must name avalanche or mountain hazard authority"
                    ));
                }
                if !contains_any(&non_assumptions, &["snowmaking"]) {
                    errors.push(format!(
                        "{path}.non_assumptions: ski profile must not imply snowmaking guarantees viability"
                    ));
                }
            }
            if upper_id.contains("CRUISE") {
                let scope = required_strings(profile, "applies_to", &path, 1, &mut errors);
                if !contains_any(&scope, &["ship"]) || !contains_any(&scope, &["port"]) {
                    errors.push(format!(
                        "{path}.applies_to: cruise profile must include both ship and port scope"
                    ));
                }
                if !contains_any(&drivers, &["shore-power", "port", "berth"]) {
                    errors.push(format!(
                        "{path}.key_drivers_to_measure: cruise profile must include port/infrastructure constraints"
                    ));
                }
                if !contains_any(&non_assumptions, &["shore-power"]) {
                    errors.push(format!(
                        "{path}.non_assumptions: cruise profile must not equate ship readiness with port supply"
                    ));
                }
            }
        }
    }

    if let Some(rules) = required_array(&document, "reusable_rules", "$", 1, &mut errors) {
        let mut rule_ids = HashSet::new();
        for (index, rule) in rules.iter().enumerate() {
            let path = format!("$.reusable_rules[{index}]");
            unique_id(rule, &path, &mut rule_ids, &mut errors);
            required_text(rule, "name", &path, &mut errors);
            required_text(rule, "rule", &path, &mut errors);
        }
    }

    if let Some(protocol) = document.get("comparative_protocol") {
        let initial = required_strings(protocol, "initial_profiles", "$.comparative_protocol", 1, &mut errors);
        for id in initial {
            if !profile_ids.contains(&id) {
                errors.push(format!(
                    "$.comparative_protocol.initial_profiles: unknown profile '{id}'"
                ));
            }
        }
        required_strings(protocol, "shared_scenarios", "$.comparative_protocol", 1, &mut errors);
        required_text(protocol, "paired_comparison_rule", "$.comparative_protocol", &mut errors);
        required_strings(protocol, "required_negative_controls", "$.comparative_protocol", 1, &mut errors);
        let outcomes = required_strings(
            protocol,
            "outcome_dimensions",
            "$.comparative_protocol",
            1,
            &mut errors,
        );
        require_tokens(
            &outcomes,
            &["option_value_and_stranded_asset_exposure"],
            "$.comparative_protocol.outcome_dimensions",
            &mut errors,
        );
        required_text(protocol, "promotion_rule", "$.comparative_protocol", &mut errors);
    } else {
        errors.push("$.comparative_protocol: required object");
    }

    validate_sources(&document, &mut errors);
    finish(errors)
}


/// Validates the machine-readable destination decision contract structurally.
/// It checks the stated ordering and required concepts; it does not execute a
/// destination optimizer or certify any real-world decision.
pub fn validate_decision_contract_json(raw: &str) -> Result<(), Vec<String>> {
    let document: Value = serde_json::from_str(raw)
        .map_err(|error| vec![format!("JSON parse error: {error}")])?;
    let mut errors = Vec::new();

    for key in ["schema_version", "artifact_id", "status", "governing_rule"] {
        required_text(&document, key, "$", &mut errors);
    }
    required_strings(&document, "non_claims", "$", 1, &mut errors);

    let states = required_strings(&document, "decision_states", "$", 3, &mut errors);
    require_tokens(
        &states,
        &["infeasible", "not_evaluable", "feasible_under_declared_assumptions"],
        "$.decision_states",
        &mut errors,
    );

    if let Some(policy) = document.get("hard_constraint_policy") {
        let order = required_strings(policy, "evaluation_order", "$.hard_constraint_policy", 4, &mut errors);
        require_exact_sequence(
            &order,
            &["rights_and_feasibility_screen", "hard_constraint_screen", "pareto_analysis", "robustness_analysis"],
            "$.hard_constraint_policy.evaluation_order",
            &mut errors,
        );
        let missing_state = required_text(policy, "missing_critical_evidence_state", "$.hard_constraint_policy", &mut errors);
        if missing_state != "not_evaluable" {
            errors.push("$.hard_constraint_policy.missing_critical_evidence_state: must be not_evaluable".to_owned());
        }
        match policy.get("score_override_prohibited").and_then(Value::as_bool) {
            Some(true) => {}
            _ => errors.push("$.hard_constraint_policy.score_override_prohibited: must be true".to_owned()),
        }
        required_text(policy, "non_substitution_rule", "$.hard_constraint_policy", &mut errors);
        required_text(policy, "authority_rule", "$.hard_constraint_policy", &mut errors);
    } else {
        errors.push("$.hard_constraint_policy: required object".to_owned());
    }

    let alternatives = required_strings(&document, "required_alternatives", "$", 6, &mut errors);
    require_tokens(
        &alternatives,
        &[
            "current_or_no_build_baseline",
            "maintenance_or_adaptive_reuse",
            "reversible_pilot",
            "proposed_new_build",
            "distributed_local_operator_portfolio",
            "managed_transition_or_closure",
        ],
        "$.required_alternatives",
        &mut errors,
    );

    let gates = required_strings(&document, "gate_order", "$", 8, &mut errors);
    require_exact_sequence(
        &gates,
        &[
            "rights_and_feasibility_envelope",
            "meaningful_alternatives",
            "scenario_set",
            "hard_constraint_screen",
            "separate_outcome_vector",
            "pareto_and_robustness",
            "staged_investment_gate",
            "decision_receipt",
        ],
        "$.gate_order",
        &mut errors,
    );

    let outcomes = required_strings(&document, "required_outcome_dimensions", "$", 7, &mut errors);
    require_tokens(
        &outcomes,
        &[
            "visitor_experience_and_access",
            "workforce_quality_and_safety",
            "host_community_wellbeing",
            "local_economic_value_retention",
            "ecology_and_resource_flows",
            "climate_and_mobility",
            "resilience_and_option_value",
        ],
        "$.required_outcome_dimensions",
        &mut errors,
    );

    let controls = required_strings(&document, "required_negative_controls", "$", 6, &mut errors);
    require_tokens(
        &controls,
        &[
            "zero_demand",
            "unavailable_service",
            "closed_route",
            "missing_or_stale_observation",
            "resource_limit_breach",
            "model_version_mismatch",
        ],
        "$.required_negative_controls",
        &mut errors,
    );

    let receipt = required_strings(&document, "required_decision_receipt_fields", "$", 1, &mut errors);
    require_tokens(
        &receipt,
        &[
            "model_and_code_identity",
            "hard_constraints_and_authority_references",
            "independent_checks_and_negative_controls",
            "local_value_commitments_and_distribution",
            "community_decision_provenance_and_dissent",
            "accountable_review_sign_off",
            "investment_gate_and_exit_plan",
        ],
        "$.required_decision_receipt_fields",
        &mut errors,
    );

    let authorities = required_strings(&document, "required_authority_boundaries", "$", 5, &mut errors);
    require_tokens(
        &authorities,
        &[
            "community_rights_and_consent_remain_human_and_local",
            "qualified_safety_authority_remains_accountable",
            "no_autonomous_physical_actuation",
            "privacy_and_culturally_restricted_evidence_protection",
            "simulation_or_attestation_does_not_equal_real_world_truth",
        ],
        "$.required_authority_boundaries",
        &mut errors,
    );

    let rules = required_strings(&document, "required_analysis_rules", "$", 6, &mut errors);
    require_tokens(
        &rules,
        &[
            "hard_constraints_before_pareto",
            "report_outcomes_separately",
            "no_universal_weights",
            "unknown_is_not_zero",
            "preserve_no_build_and_reversible_options",
            "publish_adverse_and_null_results",
        ],
        "$.required_analysis_rules",
        &mut errors,
    );

    required_text(&document, "promotion_rule", "$", &mut errors);
    validate_sources(&document, &mut errors);
    finish(errors)
}

fn validate_sources(document: &Value, errors: &mut Vec<String>) {
    let Some(sources) = required_array(document, "sources", "$", 1, errors) else {
        return;
    };
    for (index, source) in sources.iter().enumerate() {
        let path = format!("$.sources[{index}]");
        required_text(source, "topic", &path, errors);
        let url = required_text(source, "url", &path, errors);
        if !url.is_empty() && (!url.starts_with("https://") || url.chars().any(char::is_whitespace)) {
            errors.push(format!("{path}.url: expected an https URL without whitespace"));
        }
    }
}

fn finish(errors: Vec<String>) -> Result<(), Vec<String>> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_portfolio() -> Value {
        json!({
            "schema_version": "0.1.0",
            "id": "test.portfolio",
            "status": "synthetic",
            "purpose": "Fixture only",
            "experience_intents": ["play"],
            "constraint_policy": {
                "hard_constraints": ["law", "safety"],
                "soft_tradeoffs": ["cost", "variety"],
                "rule": "Hard constraints cannot be traded away."
            },
            "common_evidence_fields": [
                "candidate_id",
                "model_and_code_identity",
                "assumptions_and_units",
                "infeasibility_and_unknowns",
                "site_specific_provenance_refs",
                "capital_commitments_and_reversibility"
            ],
            "candidates": [{
                "id": "H1",
                "name": "Aqua",
                "modules": ["water_world"],
                "intents": ["play"],
                "design_hypothesis": "Synthetic fixture",
                "dependencies": ["aquatic_safety_review"],
                "key_unknowns": ["site suitability"]
            }],
            "evaluation_protocol": {
                "initial_candidates": ["H1"],
                "scenario_factors": ["demand"],
                "required_comparisons": ["same-start"],
                "required_controls": ["unaffected control"],
                "outcomes_reported_separately": ["waiting time"],
                "promotion_rule": "Synthetic only."
            },
            "references": ["https://example.org/source"]
        })
    }

    fn valid_regions() -> Value {
        json!({
            "schema_version": "0.1.0",
            "artifact_id": "test.regions",
            "status": "synthetic",
            "governing_rule": "Localize each site.",
            "non_claims": ["No site feasibility claim."],
            "shared_contract": {
                "site_identity": ["site_id"],
                "climate_and_hazards": ["source", "uncertainty"],
                "resource_systems": ["water"],
                "place_and_people": ["community_review"],
                "operations": ["closure_modes"],
                "assessment": [
                    "model_identity",
                    "baseline_and_counterfactual",
                    "reversibility_and_stranded_asset_exposure"
                ]
            },
            "profiles": [{
                "id": "CUSTOM-REGION",
                "name": "Synthetic region",
                "applies_to": ["test site"],
                "design_hypotheses": ["Test one bounded choice."],
                "key_drivers_to_measure": ["local conditions"],
                "hard_constraints": ["local safety requirements"],
                "closure_and_degraded_modes": ["close safely"],
                "unknowns": ["real site unknown"],
                "non_assumptions": ["No universal coefficient."]
            }],
            "reusable_rules": [{"id": "R1", "name": "Rule", "rule": "Keep it local."}],
            "comparative_protocol": {
                "initial_profiles": ["CUSTOM-REGION"],
                "shared_scenarios": ["baseline"],
                "paired_comparison_rule": "Change one declared intervention.",
                "required_negative_controls": ["unchanged control"],
                "outcome_dimensions": ["resource use", "option_value_and_stranded_asset_exposure"],
                "promotion_rule": "Synthetic only."
            },
            "sources": [{"topic": "Fixture source", "url": "https://example.org/source"}]
        })
    }


    fn valid_decision_contract() -> Value {
        json!({
            "schema_version": "0.1.0",
            "artifact_id": "test.decision-contract",
            "status": "synthetic",
            "governing_rule": "Constraints first; preserve uncertainty.",
            "non_claims": ["Structural completeness is not site feasibility."],
            "decision_states": ["infeasible", "not_evaluable", "feasible_under_declared_assumptions"],
            "hard_constraint_policy": {
                "evaluation_order": ["rights_and_feasibility_screen", "hard_constraint_screen", "pareto_analysis", "robustness_analysis"],
                "missing_critical_evidence_state": "not_evaluable",
                "score_override_prohibited": true,
                "non_substitution_rule": "Scores cannot override a constraint.",
                "authority_rule": "Qualified authority remains accountable."
            },
            "required_alternatives": [
                "current_or_no_build_baseline", "maintenance_or_adaptive_reuse", "reversible_pilot",
                "proposed_new_build", "distributed_local_operator_portfolio", "managed_transition_or_closure"
            ],
            "gate_order": [
                "rights_and_feasibility_envelope", "meaningful_alternatives", "scenario_set",
                "hard_constraint_screen", "separate_outcome_vector", "pareto_and_robustness",
                "staged_investment_gate", "decision_receipt"
            ],
            "required_outcome_dimensions": [
                "visitor_experience_and_access", "workforce_quality_and_safety", "host_community_wellbeing",
                "local_economic_value_retention", "ecology_and_resource_flows", "climate_and_mobility",
                "resilience_and_option_value"
            ],
            "required_negative_controls": [
                "zero_demand", "unavailable_service", "closed_route", "missing_or_stale_observation",
                "resource_limit_breach", "model_version_mismatch"
            ],
            "required_decision_receipt_fields": [
                "candidate_and_scenario_identity", "data_and_source_versions", "model_and_code_identity",
                "assumptions_units_and_uncertainty", "hard_constraints_and_authority_references",
                "independent_checks_and_negative_controls", "separate_outcome_vector",
                "local_value_commitments_and_distribution", "community_decision_provenance_and_dissent",
                "unresolved_objections_and_appeal_path", "selected_and_rejected_alternatives",
                "accountable_review_sign_off", "investment_gate_and_exit_plan"
            ],
            "required_authority_boundaries": [
                "community_rights_and_consent_remain_human_and_local",
                "qualified_safety_authority_remains_accountable", "no_autonomous_physical_actuation",
                "privacy_and_culturally_restricted_evidence_protection",
                "simulation_or_attestation_does_not_equal_real_world_truth"
            ],
            "required_analysis_rules": [
                "hard_constraints_before_pareto", "report_outcomes_separately", "no_universal_weights",
                "unknown_is_not_zero", "preserve_no_build_and_reversible_options", "publish_adverse_and_null_results"
            ],
            "promotion_rule": "Structural completeness does not certify a site.",
            "sources": [{"topic": "Fixture", "url": "https://example.org/source"}]
        })
    }

    #[test]
    fn accepts_structurally_valid_decision_contract() {
        assert!(validate_decision_contract_json(&valid_decision_contract().to_string()).is_ok());
    }

    #[test]
    fn rejects_decision_contract_without_no_build_alternative() {
        let mut document = valid_decision_contract();
        document["required_alternatives"] = json!([
            "maintenance_or_adaptive_reuse", "reversible_pilot", "proposed_new_build",
            "distributed_local_operator_portfolio", "managed_transition_or_closure"
        ]);
        let errors = validate_decision_contract_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("current_or_no_build_baseline")));
    }

    #[test]
    fn rejects_decision_contract_missing_local_value_dimension() {
        let mut document = valid_decision_contract();
        document["required_outcome_dimensions"] = json!([
            "visitor_experience_and_access", "workforce_quality_and_safety", "host_community_wellbeing",
            "ecology_and_resource_flows", "climate_and_mobility", "resilience_and_option_value"
        ]);
        let errors = validate_decision_contract_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("local_economic_value_retention")));
    }

    #[test]
    fn rejects_decision_contract_without_authority_boundary() {
        let mut document = valid_decision_contract();
        document["required_authority_boundaries"] = json!([
            "community_rights_and_consent_remain_human_and_local",
            "qualified_safety_authority_remains_accountable",
            "privacy_and_culturally_restricted_evidence_protection",
            "simulation_or_attestation_does_not_equal_real_world_truth"
        ]);
        let errors = validate_decision_contract_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("no_autonomous_physical_actuation")));
    }

    #[test]
    fn rejects_decision_contract_when_gate_order_is_changed() {
        let mut document = valid_decision_contract();
        document["gate_order"] = json!([
            "rights_and_feasibility_envelope", "meaningful_alternatives", "hard_constraint_screen",
            "scenario_set", "separate_outcome_vector", "pareto_and_robustness",
            "staged_investment_gate", "decision_receipt"
        ]);
        let errors = validate_decision_contract_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("$.gate_order: expected exact ordered sequence")));
    }

    #[test]
    fn rejects_decision_contract_when_scores_can_override_constraints() {
        let mut document = valid_decision_contract();
        document["hard_constraint_policy"]["score_override_prohibited"] = json!(false);
        let errors = validate_decision_contract_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("score_override_prohibited")));
    }

    #[test]
    fn accepts_structurally_valid_synthetic_manifests() {
        assert!(validate_portfolio_json(&valid_portfolio().to_string()).is_ok());
        assert!(validate_regional_profiles_json(&valid_regions().to_string()).is_ok());
    }

    #[test]
    fn rejects_water_candidate_without_safety_dependency() {
        let mut document = valid_portfolio();
        document["candidates"][0]["dependencies"] = json!(["queue_model"]);
        let errors = validate_portfolio_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("safety or qualified review dependency")));
    }

    #[test]
    fn rejects_unresolved_initial_candidate() {
        let mut document = valid_portfolio();
        document["evaluation_protocol"]["initial_candidates"] = json!(["NOT-DEFINED"]);
        let errors = validate_portfolio_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("unknown candidate")));
    }

    #[test]
    fn rejects_duplicate_candidate_ids() {
        let mut document = valid_portfolio();
        let duplicate = document["candidates"][0].clone();
        document["candidates"].as_array_mut().unwrap().push(duplicate);
        let errors = validate_portfolio_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("duplicate identifier 'H1'")));
    }

    #[test]
    fn rejects_unknown_regional_profile_reference() {
        let mut document = valid_regions();
        document["comparative_protocol"]["initial_profiles"] = json!(["MISSING-PROFILE"]);
        let errors = validate_regional_profiles_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("unknown profile")));
    }

    #[test]
    fn rejects_portfolio_missing_reversibility_evidence_field() {
        let mut document = valid_portfolio();
        document["common_evidence_fields"] = json!(["candidate_id", "model_and_code_identity"]);
        let errors = validate_portfolio_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("capital_commitments_and_reversibility")));
    }

    #[test]
    fn rejects_regional_manifest_missing_option_value_dimension() {
        let mut document = valid_regions();
        document["comparative_protocol"]["outcome_dimensions"] = json!(["resource use"]);
        let errors = validate_regional_profiles_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("option_value_and_stranded_asset_exposure")));
    }

    #[test]
    fn rejects_ski_profile_without_snowmaking_uncertainty() {
        let mut document = valid_regions();
        document["profiles"][0]["id"] = json!("MOUNTAIN-ALPINE-SKI");
        document["profiles"][0]["key_drivers_to_measure"] = json!(["snow reliability"]);
        document["profiles"][0]["hard_constraints"] = json!(["avalanche authority"]);
        document["profiles"][0]["non_assumptions"] = json!(["Winter will be busy."]);
        let errors = validate_regional_profiles_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("must not imply snowmaking guarantees viability")));
    }

    #[test]
    fn rejects_non_https_source() {
        let mut document = valid_regions();
        document["sources"][0]["url"] = json!("http://example.org/source");
        let errors = validate_regional_profiles_json(&document.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("expected an https URL")));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(validate_portfolio_json("{").is_err());
        assert!(validate_regional_profiles_json("{").is_err());
    }
}

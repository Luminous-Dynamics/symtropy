// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic semantic-loss auditor for design-to-fabrication exchange.
//!
//! This is a requirement/observation comparator, not an IFC parser, CAD kernel,
//! structural checker, code-compliance engine, or manufacturing release gate.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    benchmark_id: String,
    evidence_class: EvidenceClass,
    exchange_profile: String,
    cases: Vec<Scenario>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum EvidenceClass {
    Synthetic,
    Modeled,
    Measured,
    IndependentReview,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Scenario {
    case_id: String,
    expected_decision: Decision,
    requirements: Vec<Requirement>,
    observations: Vec<Observation>,
    transforms: Vec<TransformRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Requirement {
    requirement_id: String,
    object_id: String,
    property_key: String,
    expected_value: String,
    unit: Option<String>,
    criticality: Criticality,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Observation {
    requirement_id: String,
    object_id: String,
    disposition: LossDisposition,
    observed_value: Option<String>,
    unit: Option<String>,
    transform_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TransformRule {
    transform_id: String,
    from_unit: String,
    to_unit: String,
    scale: f64,
    offset: f64,
    absolute_tolerance: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum Criticality {
    Hard,
    Advisory,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum LossDisposition {
    Preserved,
    TransformedWithDeclaredRule,
    Unsupported,
    Missing,
    Changed,
    NotEvaluated,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum Decision {
    Conformant,
    Blocked,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum RequirementOutcome {
    Preserved,
    Transformed,
    Blocked,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum DiagnosticCode {
    NoTargetObservation,
    DuplicateTargetObservation,
    ObjectIdentityMismatch,
    PreservedCarriesTransform,
    PreservedValueOrUnitMismatch,
    MissingOrDuplicateTransformRule,
    DeclaredTransformResultMismatch,
    DeclaredTransformScopeOrNumericValueInvalid,
    UnsupportedTargetProperty,
    MissingTargetProperty,
    ChangedTargetProperty,
    TargetPropertyNotEvaluated,
    DuplicateSourceRequirementId,
}

#[derive(Debug, Clone, Serialize)]
struct RequirementResult {
    requirement_id: String,
    source_object_id: String,
    property_key: String,
    criticality: Criticality,
    expected_value: String,
    expected_unit: Option<String>,
    observed_object_id: Option<String>,
    observed_value: Option<String>,
    observed_unit: Option<String>,
    disposition: Option<LossDisposition>,
    transform_id: Option<String>,
    outcome: RequirementOutcome,
    diagnostic_codes: Vec<DiagnosticCode>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    case_id: String,
    computed_decision: Decision,
    expected_decision: Decision,
    oracle_match: bool,
    requirement_results: Vec<RequirementResult>,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    case_count: usize,
    oracle_matches: usize,
    suite_passed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    exchange_profile: String,
    raw_input_sha256: String,
    canonical_input_sha256: String,
    result_sha256: String,
    scenario_results: Vec<ScenarioResult>,
    summary: Summary,
}

#[derive(Serialize)]
struct ResultDigestPayload<'a> {
    schema_version: &'a str,
    benchmark_id: &'a str,
    exchange_profile: &'a str,
    canonical_input_sha256: &'a str,
    scenario_results: &'a [ScenarioResult],
    summary: &'a Summary,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn normalize_manifest(manifest: &mut Manifest) {
    manifest.cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    for case in &mut manifest.cases {
        case.requirements.sort_by(|a, b| {
            a.requirement_id
                .cmp(&b.requirement_id)
                .then_with(|| a.object_id.cmp(&b.object_id))
                .then_with(|| a.property_key.cmp(&b.property_key))
        });
        case.observations.sort_by(|a, b| {
            a.requirement_id
                .cmp(&b.requirement_id)
                .then_with(|| a.object_id.cmp(&b.object_id))
                .then_with(|| a.disposition.cmp(&b.disposition))
                .then_with(|| a.observed_value.cmp(&b.observed_value))
                .then_with(|| a.unit.cmp(&b.unit))
                .then_with(|| a.transform_id.cmp(&b.transform_id))
        });
        case.transforms
            .sort_by(|a, b| a.transform_id.cmp(&b.transform_id));
    }
}

fn validate_manifest(manifest: &Manifest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if manifest.schema_version != "0.1.0" {
        errors.push(format!(
            "unsupported_schema_version:{}",
            manifest.schema_version
        ));
    }
    if manifest.benchmark_id.trim().is_empty() {
        errors.push("empty_benchmark_id".to_string());
    }
    if manifest.exchange_profile.trim().is_empty() {
        errors.push("empty_exchange_profile".to_string());
    }
    if manifest.evidence_class != EvidenceClass::Synthetic {
        errors.push(
            "v0.1_accepts_synthetic_scenarios_only_not_real_exchange_qualification".to_string(),
        );
    }
    if manifest.cases.is_empty() {
        errors.push("empty_case_set".to_string());
    }
    let mut case_ids = BTreeSet::new();
    for case in &manifest.cases {
        if case.case_id.trim().is_empty() {
            errors.push("empty_case_id".to_string());
        } else if !case_ids.insert(case.case_id.clone()) {
            errors.push(format!("duplicate_case_id:{}", case.case_id));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        Err(errors)
    }
}

fn nonconformity_outcome(criticality: Criticality) -> RequirementOutcome {
    match criticality {
        Criticality::Hard => RequirementOutcome::Blocked,
        Criticality::Advisory => RequirementOutcome::Unresolved,
    }
}

fn evaluate_requirement(
    requirement: &Requirement,
    observations: &[Observation],
    transforms: &[TransformRule],
) -> RequirementResult {
    let matches: Vec<&Observation> = observations
        .iter()
        .filter(|observation| observation.requirement_id == requirement.requirement_id)
        .collect();
    let observation = match matches.as_slice() {
        [single] => Some(*single),
        _ => None,
    };
    let mut result = RequirementResult {
        requirement_id: requirement.requirement_id.clone(),
        source_object_id: requirement.object_id.clone(),
        property_key: requirement.property_key.clone(),
        criticality: requirement.criticality,
        expected_value: requirement.expected_value.clone(),
        expected_unit: requirement.unit.clone(),
        observed_object_id: observation.map(|value| value.object_id.clone()),
        observed_value: observation.and_then(|value| value.observed_value.clone()),
        observed_unit: observation.and_then(|value| value.unit.clone()),
        disposition: observation.map(|value| value.disposition),
        transform_id: observation.and_then(|value| value.transform_id.clone()),
        outcome: RequirementOutcome::Unresolved,
        diagnostic_codes: Vec::new(),
    };

    if matches.is_empty() {
        result.outcome = nonconformity_outcome(requirement.criticality);
        result
            .diagnostic_codes
            .push(DiagnosticCode::NoTargetObservation);
        return result;
    }
    if matches.len() > 1 {
        result.outcome = RequirementOutcome::Blocked;
        result
            .diagnostic_codes
            .push(DiagnosticCode::DuplicateTargetObservation);
        return result;
    }
    let observation = matches[0];
    if observation.object_id != requirement.object_id {
        result.outcome = RequirementOutcome::Blocked;
        result
            .diagnostic_codes
            .push(DiagnosticCode::ObjectIdentityMismatch);
        return result;
    }

    match observation.disposition {
        LossDisposition::Preserved => {
            if observation.transform_id.is_some() {
                result.outcome = RequirementOutcome::Blocked;
                result
                    .diagnostic_codes
                    .push(DiagnosticCode::PreservedCarriesTransform);
                return result;
            }
            if observation.observed_value.as_deref() != Some(requirement.expected_value.as_str())
                || observation.unit != requirement.unit
            {
                result.outcome = nonconformity_outcome(requirement.criticality);
                result
                    .diagnostic_codes
                    .push(DiagnosticCode::PreservedValueOrUnitMismatch);
                return result;
            }
            result.outcome = RequirementOutcome::Preserved;
        }
        LossDisposition::TransformedWithDeclaredRule => {
            let matching_rules: Vec<&TransformRule> = match observation.transform_id.as_deref() {
                Some(id) => transforms
                    .iter()
                    .filter(|rule| rule.transform_id == id)
                    .collect(),
                None => Vec::new(),
            };
            let rule = match matching_rules.as_slice() {
                [rule] => *rule,
                _ => {
                    result.outcome = nonconformity_outcome(requirement.criticality);
                    result
                        .diagnostic_codes
                        .push(DiagnosticCode::MissingOrDuplicateTransformRule);
                    return result;
                }
            };
            let expected_number = requirement.expected_value.parse::<f64>();
            let observed_number = observation
                .observed_value
                .as_deref()
                .unwrap_or("")
                .parse::<f64>();
            let valid_units = requirement.unit.as_deref() == Some(rule.from_unit.as_str())
                && observation.unit.as_deref() == Some(rule.to_unit.as_str());
            match (expected_number, observed_number) {
                (Ok(expected), Ok(observed))
                    if valid_units
                        && expected.is_finite()
                        && observed.is_finite()
                        && rule.scale.is_finite()
                        && rule.scale > 0.0
                        && rule.offset.is_finite()
                        && rule.absolute_tolerance.is_finite()
                        && rule.absolute_tolerance >= 0.0 =>
                {
                    let transformed = expected * rule.scale + rule.offset;
                    if !transformed.is_finite()
                        || (observed - transformed).abs() > rule.absolute_tolerance
                    {
                        result.outcome = nonconformity_outcome(requirement.criticality);
                        result
                            .diagnostic_codes
                            .push(DiagnosticCode::DeclaredTransformResultMismatch);
                        return result;
                    }
                    result.outcome = RequirementOutcome::Transformed;
                }
                _ => {
                    result.outcome = nonconformity_outcome(requirement.criticality);
                    result
                        .diagnostic_codes
                        .push(DiagnosticCode::DeclaredTransformScopeOrNumericValueInvalid);
                }
            }
        }
        LossDisposition::Unsupported => {
            result.outcome = nonconformity_outcome(requirement.criticality);
            result
                .diagnostic_codes
                .push(DiagnosticCode::UnsupportedTargetProperty);
        }
        LossDisposition::Missing => {
            result.outcome = nonconformity_outcome(requirement.criticality);
            result
                .diagnostic_codes
                .push(DiagnosticCode::MissingTargetProperty);
        }
        LossDisposition::Changed => {
            result.outcome = nonconformity_outcome(requirement.criticality);
            result
                .diagnostic_codes
                .push(DiagnosticCode::ChangedTargetProperty);
        }
        LossDisposition::NotEvaluated => {
            result.outcome = nonconformity_outcome(requirement.criticality);
            result
                .diagnostic_codes
                .push(DiagnosticCode::TargetPropertyNotEvaluated);
        }
    }
    result
}

fn evaluate_scenario(case: &Scenario) -> ScenarioResult {
    let mut blocked = false;
    let mut unresolved = false;
    let mut global_issues = Vec::new();

    if case.requirements.is_empty() {
        unresolved = true;
        global_issues.push("NO_SOURCE_REQUIREMENTS_DECLARED".to_string());
    }

    let mut requirement_ids = BTreeSet::new();
    for requirement in &case.requirements {
        if requirement.requirement_id.trim().is_empty()
            || requirement.object_id.trim().is_empty()
            || requirement.property_key.trim().is_empty()
        {
            blocked = true;
            global_issues.push(format!(
                "EMPTY_SOURCE_IDENTITY_FIELD:{}",
                requirement.requirement_id
            ));
        }
        if !requirement_ids.insert(requirement.requirement_id.clone()) {
            blocked = true;
            global_issues.push(format!(
                "DUPLICATE_SOURCE_REQUIREMENT_ID:{}",
                requirement.requirement_id
            ));
        }
    }

    let mut transform_ids = BTreeSet::new();
    for rule in &case.transforms {
        if rule.transform_id.trim().is_empty() || !transform_ids.insert(rule.transform_id.clone()) {
            blocked = true;
            global_issues.push(format!(
                "EMPTY_OR_DUPLICATE_TRANSFORM_ID:{}",
                rule.transform_id
            ));
        }
        if !rule.scale.is_finite()
            || rule.scale <= 0.0
            || !rule.offset.is_finite()
            || !rule.absolute_tolerance.is_finite()
            || rule.absolute_tolerance < 0.0
            || rule.from_unit.trim().is_empty()
            || rule.to_unit.trim().is_empty()
        {
            blocked = true;
            global_issues.push(format!("INVALID_TRANSFORM_RULE:{}", rule.transform_id));
        }
    }

    for observation in &case.observations {
        if !requirement_ids.contains(&observation.requirement_id) {
            blocked = true;
            global_issues.push(format!(
                "OBSERVATION_WITHOUT_SOURCE_REQUIREMENT:{}",
                observation.requirement_id
            ));
        }
        if observation.requirement_id.trim().is_empty() || observation.object_id.trim().is_empty() {
            blocked = true;
            global_issues.push("EMPTY_TARGET_IDENTITY_FIELD".to_string());
        }
    }

    let mut requirement_results: Vec<RequirementResult> = case
        .requirements
        .iter()
        .map(|requirement| evaluate_requirement(requirement, &case.observations, &case.transforms))
        .collect();

    for requirement in &case.requirements {
        if case
            .requirements
            .iter()
            .filter(|candidate| candidate.requirement_id == requirement.requirement_id)
            .count()
            > 1
        {
            for result in requirement_results
                .iter_mut()
                .filter(|result| result.requirement_id == requirement.requirement_id)
            {
                result.outcome = RequirementOutcome::Blocked;
                result
                    .diagnostic_codes
                    .push(DiagnosticCode::DuplicateSourceRequirementId);
            }
        }
    }

    for result in &mut requirement_results {
        result.diagnostic_codes.sort();
        result.diagnostic_codes.dedup();
        match result.outcome {
            RequirementOutcome::Blocked => blocked = true,
            RequirementOutcome::Unresolved => unresolved = true,
            RequirementOutcome::Preserved | RequirementOutcome::Transformed => {}
        }
    }

    let mut issues = global_issues;
    for result in &requirement_results {
        for code in &result.diagnostic_codes {
            issues.push(format!("{:?}:{}", code, result.requirement_id));
        }
    }
    issues.sort();
    issues.dedup();

    let computed_decision = if blocked {
        Decision::Blocked
    } else if unresolved {
        Decision::Unresolved
    } else {
        Decision::Conformant
    };
    ScenarioResult {
        case_id: case.case_id.clone(),
        computed_decision,
        expected_decision: case.expected_decision,
        oracle_match: computed_decision == case.expected_decision,
        requirement_results,
        issues,
    }
}

fn build_report(manifest: &Manifest, raw_input: &[u8]) -> Report {
    let mut canonical_manifest = manifest.clone();
    normalize_manifest(&mut canonical_manifest);
    let canonical_bytes = serde_json::to_vec(&canonical_manifest)
        .expect("serializing a parsed manifest should not fail");
    let canonical_input_sha256 = sha256_hex(&canonical_bytes);
    let scenario_results: Vec<ScenarioResult> = canonical_manifest
        .cases
        .iter()
        .map(evaluate_scenario)
        .collect();
    let oracle_matches = scenario_results.iter().filter(|r| r.oracle_match).count();
    let summary = Summary {
        case_count: scenario_results.len(),
        oracle_matches,
        suite_passed: oracle_matches == scenario_results.len(),
    };
    let digest_payload = ResultDigestPayload {
        schema_version: &canonical_manifest.schema_version,
        benchmark_id: &canonical_manifest.benchmark_id,
        exchange_profile: &canonical_manifest.exchange_profile,
        canonical_input_sha256: &canonical_input_sha256,
        scenario_results: &scenario_results,
        summary: &summary,
    };
    let digest_bytes =
        serde_json::to_vec(&digest_payload).expect("serializing a report payload should not fail");
    Report {
        schema_version: canonical_manifest.schema_version,
        benchmark_id: canonical_manifest.benchmark_id,
        exchange_profile: canonical_manifest.exchange_profile,
        raw_input_sha256: sha256_hex(raw_input),
        canonical_input_sha256,
        result_sha256: sha256_hex(&digest_bytes),
        scenario_results,
        summary,
    }
}

fn usage() -> &'static str {
    "Usage: built-interop-audit [manifest.json] [--output report.json]\n\
     Evaluates synthetic semantic-loss scenarios only; Conformant is not construction approval."
}

fn main() {
    let mut args = env::args().skip(1);
    let mut input_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{}", usage());
                return;
            }
            "--output" | "-o" => {
                let Some(path) = args.next() else {
                    eprintln!("--output requires a path");
                    process::exit(2);
                };
                output_path = Some(PathBuf::from(path));
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {arg}\n{}", usage());
                process::exit(2);
            }
            _ => {
                if input_path.replace(PathBuf::from(arg)).is_some() {
                    eprintln!("Only one input manifest is supported.\n{}", usage());
                    process::exit(2);
                }
            }
        }
    }
    let input_path = input_path.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("panel-exchange-v0.1.json")
    });
    let raw_input = fs::read(&input_path).unwrap_or_else(|error| {
        eprintln!("Cannot read {}: {error}", input_path.display());
        process::exit(2);
    });
    let manifest: Manifest = serde_json::from_slice(&raw_input).unwrap_or_else(|error| {
        eprintln!("Invalid manifest {}: {error}", input_path.display());
        process::exit(2);
    });
    if let Err(errors) = validate_manifest(&manifest) {
        eprintln!("Manifest contract rejected:");
        for error in errors {
            eprintln!("  - {error}");
        }
        process::exit(2);
    }
    let report = build_report(&manifest, &raw_input);
    let json = serde_json::to_vec_pretty(&report).expect("serializing a report should not fail");
    if let Some(path) = output_path
        && let Err(error) = fs::write(&path, &json)
    {
        eprintln!("Cannot write {}: {error}", path.display());
        process::exit(2);
    }
    println!("{}", String::from_utf8_lossy(&json));
    if !report.summary.suite_passed {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Manifest, Vec<u8>) {
        let raw = include_bytes!("../fixtures/panel-exchange-v0.1.json").to_vec();
        let manifest = serde_json::from_slice(&raw).expect("fixture should parse");
        (manifest, raw)
    }

    #[test]
    fn synthetic_manifest_passes_its_pinned_json_schema() {
        let schema: serde_json::Value =
            serde_json::from_slice(include_bytes!("../semantic-loss-v0.1.schema.json"))
                .expect("input JSON Schema parses");
        let validator = jsonschema::validator_for(&schema).expect("input JSON Schema compiles");
        let instance: serde_json::Value =
            serde_json::from_slice(include_bytes!("../fixtures/panel-exchange-v0.1.json"))
                .expect("fixture manifest parses");
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|error| error.to_string())
            .collect();
        assert!(
            errors.is_empty(),
            "manifest schema validation failed: {}",
            errors.join("; ")
        );

        let mut invalid = instance;
        invalid["evidence_class"] = serde_json::json!("Measured");
        assert!(
            !validator.is_valid(&invalid),
            "the synthetic-only evidence-class guard must reject a measured manifest"
        );
    }

    #[test]
    fn serialized_report_passes_schema_and_rejects_unknown_diagnostic_code() {
        let (manifest, raw) = fixture();
        let report = build_report(&manifest, &raw);
        let instance = serde_json::to_value(&report).expect("report serializes");
        let schema: serde_json::Value =
            serde_json::from_slice(include_bytes!("../report-v0.1.schema.json"))
                .expect("report JSON Schema parses");
        let validator = jsonschema::validator_for(&schema).expect("report JSON Schema compiles");
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|error| error.to_string())
            .collect();
        assert!(
            errors.is_empty(),
            "report schema validation failed: {}",
            errors.join("; ")
        );

        let mut invalid = instance;
        invalid["scenario_results"][0]["requirement_results"][0]["diagnostic_codes"] =
            serde_json::json!(["UNLISTED_DIAGNOSTIC_CODE"]);
        assert!(
            !validator.is_valid(&invalid),
            "output schema must reject a diagnostic code outside the pinned vocabulary"
        );
    }

    #[test]
    fn independent_pinned_scenario_oracles_match() {
        const EXPECTED: &[(&str, Decision)] = &[
            ("01-exact-property-preservation", Decision::Conformant),
            ("02-hard-property-explicitly-missing", Decision::Blocked),
            ("03-unit-changed_without_transform", Decision::Blocked),
            ("04-source-object-identity-swapped", Decision::Blocked),
            ("05-declared-unit-transform-correct", Decision::Conformant),
            ("06-declared-transform-rule-missing", Decision::Blocked),
            ("07-unknown-promoted-to-numeric-default", Decision::Blocked),
            ("08-hard-property-unsupported", Decision::Blocked),
            ("09-advisory-property-not-evaluated", Decision::Unresolved),
            ("10-duplicate-imported-observation", Decision::Blocked),
            ("11-no-observation-record", Decision::Blocked),
            ("12-unreferenced-target-observation", Decision::Blocked),
        ];
        let (manifest, raw) = fixture();
        validate_manifest(&manifest).expect("top-level manifest should be valid");
        let report = build_report(&manifest, &raw);
        assert_eq!(report.summary.case_count, EXPECTED.len());
        assert_eq!(report.summary.oracle_matches, EXPECTED.len());
        assert!(report.summary.suite_passed);
        for result in &report.scenario_results {
            let expected = EXPECTED
                .iter()
                .find(|(id, _)| *id == result.case_id.as_str())
                .map(|(_, value)| *value)
                .expect("scenario ID must have a separately pinned oracle");
            assert_eq!(
                result.expected_decision, expected,
                "fixture oracle changed: {}",
                result.case_id
            );
            assert_eq!(
                result.computed_decision, expected,
                "comparison outcome changed: {}",
                result.case_id
            );
        }
    }

    #[test]
    fn approved_transform_rule_is_pinned_out_of_band() {
        const EXPECTED: &[(&str, &str, &str, f64, f64, f64)] =
            &[("mm-to-m-v1", "mm", "m", 0.001, 0.0, 0.000000001)];

        let (manifest, _) = fixture();
        let rules: Vec<&TransformRule> = manifest
            .cases
            .iter()
            .flat_map(|case| case.transforms.iter())
            .collect();
        assert_eq!(rules.len(), EXPECTED.len());

        for (id, from_unit, to_unit, scale, offset, tolerance) in EXPECTED {
            let rule = rules
                .iter()
                .find(|rule| rule.transform_id == *id)
                .expect("every transform must match an independently pinned rule");
            assert_eq!(rule.from_unit, *from_unit);
            assert_eq!(rule.to_unit, *to_unit);
            assert_eq!(rule.scale, *scale);
            assert_eq!(rule.offset, *offset);
            assert_eq!(rule.absolute_tolerance, *tolerance);
        }
    }

    #[test]
    fn zero_scale_cannot_erase_a_measurement() {
        let (manifest, _) = fixture();
        let mut case = manifest
            .cases
            .iter()
            .find(|case| case.case_id == "05-declared-unit-transform-correct")
            .expect("transform fixture exists")
            .clone();
        case.transforms[0].scale = 0.0;
        assert_eq!(
            evaluate_scenario(&case).computed_decision,
            Decision::Blocked
        );
    }

    #[test]
    fn structured_requirement_result_retains_source_and_target_values() {
        let (manifest, _) = fixture();
        let mut case = manifest
            .cases
            .iter()
            .find(|case| case.case_id == "01-exact-property-preservation")
            .expect("preservation fixture exists")
            .clone();
        case.requirements
            .retain(|requirement| requirement.requirement_id == "req-thickness");
        case.observations
            .retain(|observation| observation.requirement_id == "req-thickness");
        let result = evaluate_scenario(&case);
        let property = result
            .requirement_results
            .first()
            .expect("property result exists");
        assert_eq!(property.source_object_id, "panel-001");
        assert_eq!(property.property_key, "PanelThickness");
        assert_eq!(property.expected_value, "18.0");
        assert_eq!(property.expected_unit.as_deref(), Some("mm"));
        assert_eq!(property.observed_object_id.as_deref(), Some("panel-001"));
        assert_eq!(property.observed_value.as_deref(), Some("18.0"));
        assert_eq!(property.observed_unit.as_deref(), Some("mm"));
        assert_eq!(property.outcome, RequirementOutcome::Preserved);
        assert!(property.diagnostic_codes.is_empty());
    }

    #[test]
    fn structured_loss_result_has_machine_readable_diagnostic_code() {
        let (manifest, _) = fixture();
        let case = manifest
            .cases
            .iter()
            .find(|case| case.case_id == "02-hard-property-explicitly-missing")
            .expect("missing-property fixture exists");
        let result = evaluate_scenario(case);
        let property = result
            .requirement_results
            .first()
            .expect("property result exists");
        assert_eq!(property.outcome, RequirementOutcome::Blocked);
        assert_eq!(
            property.diagnostic_codes,
            vec![DiagnosticCode::MissingTargetProperty]
        );
        let value = serde_json::to_value(&result).expect("scenario result serializes");
        assert_eq!(
            value["requirement_results"][0]["diagnostic_codes"][0],
            "MISSING_TARGET_PROPERTY"
        );
        assert_eq!(property.source_object_id, "panel-001");
        assert_eq!(property.property_key, "RequiredFireClass");
        assert_eq!(property.disposition, Some(LossDisposition::Missing));
        assert!(property.observed_value.is_none());
    }

    #[test]
    fn absent_observation_has_distinct_machine_readable_code() {
        let (manifest, _) = fixture();
        let case = manifest
            .cases
            .iter()
            .find(|case| case.case_id == "11-no-observation-record")
            .expect("no-observation fixture exists");
        let result = evaluate_scenario(case);
        let property = result
            .requirement_results
            .first()
            .expect("property result exists");
        assert_eq!(property.outcome, RequirementOutcome::Blocked);
        assert_eq!(
            property.diagnostic_codes,
            vec![DiagnosticCode::NoTargetObservation]
        );
        assert!(property.disposition.is_none());
        assert!(property.observed_object_id.is_none());
        assert!(property.observed_value.is_none());
    }

    #[test]
    fn serialized_report_contains_one_result_per_source_requirement() {
        let (manifest, raw) = fixture();
        let report = build_report(&manifest, &raw);
        let value = serde_json::to_value(&report).expect("full report serializes");
        let rows = value["scenario_results"]
            .as_array()
            .expect("scenario_results is an array");
        assert_eq!(rows.len(), manifest.cases.len());

        for row in rows {
            let case_id = row["case_id"].as_str().expect("case ID is serialized");
            let source_case = manifest
                .cases
                .iter()
                .find(|case| case.case_id == case_id)
                .expect("report scenario maps to a source case");
            let properties = row["requirement_results"]
                .as_array()
                .expect("structured requirement results are serialized");
            assert_eq!(properties.len(), source_case.requirements.len());
            for requirement in &source_case.requirements {
                let property = properties
                    .iter()
                    .find(|property| property["requirement_id"] == requirement.requirement_id)
                    .expect("every source requirement has a structured result");
                assert_eq!(property["source_object_id"], requirement.object_id);
                assert_eq!(property["property_key"], requirement.property_key);
                assert!(property["diagnostic_codes"].is_array());
                assert!(property["outcome"].is_string());
            }
        }
    }

    #[test]
    fn structured_transform_result_retains_rule_and_source_target_units() {
        let (manifest, _) = fixture();
        let case = manifest
            .cases
            .iter()
            .find(|case| case.case_id == "05-declared-unit-transform-correct")
            .expect("transform fixture exists");
        let result = evaluate_scenario(case);
        let property = result
            .requirement_results
            .first()
            .expect("transformed property result exists");
        assert_eq!(property.outcome, RequirementOutcome::Transformed);
        assert_eq!(property.transform_id.as_deref(), Some("mm-to-m-v1"));
        assert_eq!(property.expected_value, "18.0");
        assert_eq!(property.expected_unit.as_deref(), Some("mm"));
        assert_eq!(property.observed_value.as_deref(), Some("0.018"));
        assert_eq!(property.observed_unit.as_deref(), Some("m"));
        assert!(property.diagnostic_codes.is_empty());

        let value = serde_json::to_value(&result).expect("scenario result serializes");
        assert_eq!(value["requirement_results"][0]["outcome"], "Transformed");
        assert_eq!(
            value["requirement_results"][0]["transform_id"],
            "mm-to-m-v1"
        );
        assert_eq!(value["requirement_results"][0]["expected_unit"], "mm");
        assert_eq!(value["requirement_results"][0]["observed_unit"], "m");
    }

    #[test]
    fn row_order_permutation_preserves_semantic_digest() {
        let (manifest, raw) = fixture();
        let first = build_report(&manifest, &raw);
        let mut permuted = manifest.clone();
        permuted.cases.reverse();
        for case in &mut permuted.cases {
            case.requirements.reverse();
            case.observations.reverse();
            case.transforms.reverse();
        }
        let permuted_raw = serde_json::to_vec(&permuted).expect("serialize permutation");
        let second = build_report(&permuted, &permuted_raw);
        assert_eq!(first.canonical_input_sha256, second.canonical_input_sha256);
        assert_eq!(first.result_sha256, second.result_sha256);
        assert_ne!(first.raw_input_sha256, second.raw_input_sha256);
    }

    #[test]
    fn numeric_transform_does_not_hide_a_wrong_unit_or_wrong_value() {
        let (manifest, _) = fixture();
        let mut case = manifest
            .cases
            .iter()
            .find(|c| c.case_id == "05-declared-unit-transform-correct")
            .expect("transform fixture exists")
            .clone();
        case.observations[0].observed_value = Some("0.02".to_string());
        assert_eq!(
            evaluate_scenario(&case).computed_decision,
            Decision::Blocked
        );
    }

    #[test]
    fn criticality_cannot_be_laundered_by_observation_label() {
        let (manifest, _) = fixture();
        let mut case = manifest
            .cases
            .iter()
            .find(|c| c.case_id == "02-hard-property-explicitly-missing")
            .expect("missing fixture exists")
            .clone();
        case.observations[0].disposition = LossDisposition::NotEvaluated;
        assert_eq!(
            evaluate_scenario(&case).computed_decision,
            Decision::Blocked
        );
    }
}

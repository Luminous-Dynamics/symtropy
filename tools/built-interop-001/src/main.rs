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

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    case_id: String,
    computed_decision: Decision,
    expected_decision: Decision,
    oracle_match: bool,
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

#[derive(Clone, Copy)]
enum Severity {
    Blocked,
    Unresolved,
}

fn add_issue(
    issues: &mut Vec<String>,
    blocked: &mut bool,
    unresolved: &mut bool,
    severity: Severity,
    message: String,
) {
    match severity {
        Severity::Blocked => *blocked = true,
        Severity::Unresolved => *unresolved = true,
    }
    issues.push(message);
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

fn evaluate_scenario(case: &Scenario) -> ScenarioResult {
    let mut issues = Vec::new();
    let mut blocked = false;
    let mut unresolved = false;

    if case.requirements.is_empty() {
        add_issue(
            &mut issues,
            &mut blocked,
            &mut unresolved,
            Severity::Unresolved,
            "no_source_requirements_declared".to_string(),
        );
    }

    let mut requirement_ids = BTreeSet::new();
    for requirement in &case.requirements {
        if requirement.requirement_id.trim().is_empty()
            || requirement.object_id.trim().is_empty()
            || requirement.property_key.trim().is_empty()
        {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("empty_source_identity_field:{}", requirement.requirement_id),
            );
        }
        if !requirement_ids.insert(requirement.requirement_id.clone()) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "duplicate_source_requirement_id:{}",
                    requirement.requirement_id
                ),
            );
        }
    }

    let mut transform_ids = BTreeSet::new();
    for rule in &case.transforms {
        if rule.transform_id.trim().is_empty() || !transform_ids.insert(rule.transform_id.clone()) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("empty_or_duplicate_transform_id:{}", rule.transform_id),
            );
        }
        if !rule.scale.is_finite()
            || rule.scale <= 0.0
            || !rule.offset.is_finite()
            || !rule.absolute_tolerance.is_finite()
            || rule.absolute_tolerance < 0.0
            || rule.from_unit.trim().is_empty()
            || rule.to_unit.trim().is_empty()
        {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("invalid_transform_rule:{}", rule.transform_id),
            );
        }
    }

    for observation in &case.observations {
        if !requirement_ids.contains(&observation.requirement_id) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "observation_without_source_requirement:{}",
                    observation.requirement_id
                ),
            );
        }
        if observation.requirement_id.trim().is_empty() || observation.object_id.trim().is_empty() {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                "empty_target_identity_field".to_string(),
            );
        }
    }

    for requirement in &case.requirements {
        let matches: Vec<&Observation> = case
            .observations
            .iter()
            .filter(|observation| observation.requirement_id == requirement.requirement_id)
            .collect();

        let observation = match matches.as_slice() {
            [] => {
                let severity = match requirement.criticality {
                    Criticality::Hard => Severity::Blocked,
                    Criticality::Advisory => Severity::Unresolved,
                };
                add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    severity,
                    format!("no_target_observation:{}", requirement.requirement_id),
                );
                continue;
            }
            [single] => *single,
            _ => {
                add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Blocked,
                    format!(
                        "duplicate_target_observation:{}",
                        requirement.requirement_id
                    ),
                );
                continue;
            }
        };

        if observation.object_id != requirement.object_id {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "object_identity_mismatch:{}:expected={}:actual={}",
                    requirement.requirement_id, requirement.object_id, observation.object_id
                ),
            );
            continue;
        }

        let nonconformity_severity = match requirement.criticality {
            Criticality::Hard => Severity::Blocked,
            Criticality::Advisory => Severity::Unresolved,
        };

        match observation.disposition {
            LossDisposition::Preserved => {
                if observation.transform_id.is_some() {
                    add_issue(
                        &mut issues,
                        &mut blocked,
                        &mut unresolved,
                        Severity::Blocked,
                        format!(
                            "preserved_property_carries_transform:{}",
                            requirement.requirement_id
                        ),
                    );
                }
                if observation.observed_value.as_deref()
                    != Some(requirement.expected_value.as_str())
                    || observation.unit != requirement.unit
                {
                    add_issue(
                        &mut issues,
                        &mut blocked,
                        &mut unresolved,
                        nonconformity_severity,
                        format!(
                            "preserved_value_or_unit_mismatch:{}",
                            requirement.requirement_id
                        ),
                    );
                }
            }
            LossDisposition::TransformedWithDeclaredRule => {
                let matching_rules: Vec<&TransformRule> = match observation.transform_id.as_deref()
                {
                    Some(id) => case
                        .transforms
                        .iter()
                        .filter(|rule| rule.transform_id == id)
                        .collect(),
                    None => Vec::new(),
                };
                match matching_rules.as_slice() {
                    [rule] => {
                        let expected_number = requirement.expected_value.parse::<f64>();
                        let observed_number = observation
                            .observed_value
                            .as_deref()
                            .unwrap_or("")
                            .parse::<f64>();
                        let valid_units = requirement.unit.as_deref()
                            == Some(rule.from_unit.as_str())
                            && observation.unit.as_deref() == Some(rule.to_unit.as_str());
                        match (expected_number, observed_number) {
                            (Ok(expected), Ok(observed))
                                if valid_units
                                    && expected.is_finite()
                                    && observed.is_finite()
                                    && rule.scale.is_finite()
                                    && rule.offset.is_finite()
                                    && rule.absolute_tolerance.is_finite()
                                    && rule.absolute_tolerance >= 0.0 =>
                            {
                                let transformed = expected * rule.scale + rule.offset;
                                if !transformed.is_finite()
                                    || (observed - transformed).abs() > rule.absolute_tolerance
                                {
                                    add_issue(
                                        &mut issues,
                                        &mut blocked,
                                        &mut unresolved,
                                        nonconformity_severity,
                                        format!(
                                            "declared_transform_result_mismatch:{}",
                                            requirement.requirement_id
                                        ),
                                    );
                                }
                            }
                            _ => add_issue(
                                &mut issues,
                                &mut blocked,
                                &mut unresolved,
                                nonconformity_severity,
                                format!(
                                    "declared_transform_scope_or_numeric_value_invalid:{}",
                                    requirement.requirement_id
                                ),
                            ),
                        }
                    }
                    _ => add_issue(
                        &mut issues,
                        &mut blocked,
                        &mut unresolved,
                        nonconformity_severity,
                        format!(
                            "missing_or_duplicate_transform_rule:{}",
                            requirement.requirement_id
                        ),
                    ),
                }
            }
            LossDisposition::Unsupported => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                nonconformity_severity,
                format!("unsupported_target_property:{}", requirement.requirement_id),
            ),
            LossDisposition::Missing => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                nonconformity_severity,
                format!("missing_target_property:{}", requirement.requirement_id),
            ),
            LossDisposition::Changed => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                nonconformity_severity,
                format!("changed_target_property:{}", requirement.requirement_id),
            ),
            LossDisposition::NotEvaluated => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                nonconformity_severity,
                format!(
                    "target_property_not_evaluated:{}",
                    requirement.requirement_id
                ),
            ),
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

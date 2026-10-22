use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

pub const PLAN_SCHEMA: &str = "pcb-route-rule-fidelity-plan-v0.1";
pub const RECEIPT_SCHEMA: &str = "pcb-route-rule-fidelity-receipt-v0.1";
pub const COMPARISON_SCHEMA: &str = "pcb-route-rule-fidelity-comparison-v0.1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleFidelityPlan {
    pub schema: String,
    pub benchmark_id: String,
    pub dataset_id: String,
    pub split_id: String,
    pub board_id: String,
    pub source_board_relative_path: String,
    pub source_board_sha256: String,
    pub output_board_relative_path: String,
    pub output_board_sha256: String,
    pub source_rule_profile_relative_path: Option<String>,
    pub source_rule_profile_sha256: Option<String>,
    pub source_kicad_version: Option<String>,
    pub compatibility_policy_relative_path: String,
    pub compatibility_policy_sha256: String,
    pub compatibility_kicad_version: String,
    pub compatibility_drc_report_relative_path: String,
    pub strict_source_policy_relative_path: Option<String>,
    pub strict_source_policy_sha256: Option<String>,
    pub strict_source_kicad_version: Option<String>,
    pub strict_source_drc_report_relative_path: Option<String>,
    pub strict_source_policy_derivation_evidence_relative_path: Option<String>,
    pub strict_source_policy_derivation_evidence_sha256: Option<String>,
    pub rule_assessments: Vec<RuleAssessment>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleAssessment {
    pub rule_id: String,
    pub classification: RuleClassification,
    pub source_value: Option<String>,
    pub evaluated_value: Option<String>,
    pub mapping_evidence_relative_path: Option<String>,
    pub mapping_evidence_sha256: Option<String>,
    pub waiver_evidence_relative_path: Option<String>,
    pub waiver_evidence_sha256: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleClassification {
    SourcePreserved,
    VersionMigratedEquivalent,
    ExplicitlyWaived,
    Inferred,
    Relaxed,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DrcReportSummary {
    pub report_sha256: String,
    pub violation_count: usize,
    pub violation_type_counts: BTreeMap<String, usize>,
    pub unconnected_item_count: usize,
    pub clean_pass: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifiedRuleEvidence {
    pub purpose: String,
    pub relative_path: String,
    pub byte_len: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuleFidelityReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub plan_sha256: String,
    pub constraint_fingerprint_sha256: String,
    pub benchmark_id: String,
    pub dataset_id: String,
    pub split_id: String,
    pub board_id: String,
    pub source_board_sha256: String,
    pub output_board_sha256: String,
    pub source_rule_profile_sha256: Option<String>,
    pub compatibility_policy_sha256: String,
    pub strict_source_policy_sha256: Option<String>,
    pub source_rule_status: String,
    pub source_authoritative: bool,
    pub rule_classification_counts: BTreeMap<String, usize>,
    pub verified_rule_evidence: Vec<VerifiedRuleEvidence>,
    pub compatibility_drc: DrcReportSummary,
    pub strict_source_drc: Option<DrcReportSummary>,
    pub compatibility_policy_clean_pass: bool,
    pub strict_source_policy_evaluated: bool,
    pub strict_source_clean_pass: bool,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuleFidelityComparison {
    pub schema: String,
    pub board_id: String,
    pub constraint_fingerprint_sha256: String,
    pub baseline_compatibility_clean_pass: bool,
    pub treatment_compatibility_clean_pass: bool,
    pub baseline_strict_source_clean_pass: bool,
    pub treatment_strict_source_clean_pass: bool,
    pub strict_source_comparison_available: bool,
    pub compatibility_outcome_changed: bool,
    pub strict_source_outcome_changed: bool,
    pub authority: String,
    pub claim_ceiling: String,
}

pub fn evaluate_rule_fidelity(
    plan_bytes: &[u8],
    source_board_bytes: &[u8],
    output_board_bytes: &[u8],
    source_rule_profile_bytes: Option<&[u8]>,
    compatibility_policy_bytes: &[u8],
    compatibility_drc_report_bytes: &[u8],
    strict_source_policy_bytes: Option<&[u8]>,
    strict_source_drc_report_bytes: Option<&[u8]>,
    rule_evidence_files: &BTreeMap<String, Vec<u8>>,
    evaluator_git_head: &str,
) -> Result<RuleFidelityReceipt, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err("evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into());
    }

    let plan: RuleFidelityPlan = serde_json::from_slice(plan_bytes)
        .map_err(|error| format!("rule-fidelity plan JSON/schema invalid: {error}"))?;
    validate_plan_shape(&plan)?;

    let source_board_sha = sha256_hex(source_board_bytes);
    if source_board_sha != plan.source_board_sha256 {
        return Err("source-board bytes SHA-256 differs from the rule-fidelity plan".into());
    }
    let output_board_sha = sha256_hex(output_board_bytes);
    if output_board_sha != plan.output_board_sha256 {
        return Err("output-board bytes SHA-256 differs from the rule-fidelity plan".into());
    }

    match (
        plan.source_rule_profile_relative_path.as_deref(),
        plan.source_rule_profile_sha256.as_deref(),
        source_rule_profile_bytes,
    ) {
        (Some(path), Some(expected), Some(bytes)) => {
            validate_relative_path(path, "source rule-profile path")?;
            require_sha256(expected, "source_rule_profile_sha256")?;
            if sha256_hex(bytes) != expected {
                return Err("source rule-profile bytes SHA-256 differs from the rule-fidelity plan".into());
            }
            if bytes.is_empty() {
                return Err("source rule-profile bytes must not be empty".into());
            }
        }
        (None, None, None) => {}
        _ => {
            return Err(
                "source rule-profile path, digest and supplied bytes must be present together or all absent"
                    .into(),
            )
        }
    }

    validate_policy_bytes(
        compatibility_policy_bytes,
        &plan.compatibility_policy_sha256,
        &plan.compatibility_kicad_version,
        "compatibility",
    )?;
    validate_report_path(&plan.compatibility_drc_report_relative_path, "compatibility DRC report path")?;
    let compatibility_drc = parse_drc_report(compatibility_drc_report_bytes)?;

    match (
        plan.strict_source_policy_relative_path.as_deref(),
        plan.strict_source_policy_sha256.as_deref(),
        plan.strict_source_kicad_version.as_deref(),
        plan.strict_source_drc_report_relative_path.as_deref(),
        plan.strict_source_policy_derivation_evidence_relative_path.as_deref(),
        plan.strict_source_policy_derivation_evidence_sha256.as_deref(),
        strict_source_policy_bytes,
        strict_source_drc_report_bytes,
    ) {
        (Some(policy_path), Some(policy_sha), Some(version), Some(report_path), Some(derivation_path), Some(derivation_sha), Some(policy_bytes), Some(report_bytes)) => {
            validate_report_path(policy_path, "strict source policy path")?;
            validate_report_path(report_path, "strict source DRC report path")?;
            validate_relative_path(derivation_path, "strict-source policy derivation evidence path")?;
            require_sha256(policy_sha, "strict_source_policy_sha256")?;
            require_sha256(derivation_sha, "strict_source_policy_derivation_evidence_sha256")?;
            validate_policy_bytes(policy_bytes, policy_sha, version, "strict source")?;
            let _ = parse_drc_report(report_bytes)?;
        }
        (None, None, None, None, None, None, None, None) => {}
        _ => {
            return Err(
                "strict-source policy, report, derivation evidence path/digest and supplied bytes must be present together or all absent"
                    .into(),
            )
        }
    }

    let verified_rule_evidence = verify_rule_evidence(&plan, rule_evidence_files)?;
    let strict_derivation_verified = plan
        .strict_source_policy_derivation_evidence_relative_path
        .as_deref()
        .is_some_and(|path| verified_rule_evidence.iter().any(|evidence| {
            evidence.purpose == "strict_source_policy_derivation" && evidence.relative_path == path
        }));
    let source_rule_profile_present = source_rule_profile_bytes.is_some();
    let source_authoritative = source_rule_profile_present
        && plan.rule_assessments.iter().all(|assessment| {
            matches!(
                assessment.classification,
                RuleClassification::SourcePreserved | RuleClassification::VersionMigratedEquivalent
            )
        });
    let mut class_counts = BTreeMap::new();
    for assessment in &plan.rule_assessments {
        *class_counts
            .entry(classification_name(assessment.classification).to_string())
            .or_insert(0usize) += 1;
    }

    let strict_source_drc = match strict_source_drc_report_bytes {
        Some(bytes) => Some(parse_drc_report(bytes)?),
        None => None,
    };
    let strict_source_policy_evaluated = strict_source_drc.is_some();
    let strict_source_clean_pass = source_authoritative
        && strict_derivation_verified
        && plan.strict_source_policy_sha256.is_some()
        && strict_source_policy_evaluated
        && strict_source_drc.as_ref().is_some_and(|report| report.clean_pass);

    let mut reason_codes = Vec::new();
    if !source_rule_profile_present {
        reason_codes.push("SOURCE_RULE_PROFILE_UNAVAILABLE".to_string());
    }
    if plan.strict_source_policy_sha256.is_some() && !strict_derivation_verified {
        reason_codes.push("STRICT_SOURCE_POLICY_DERIVATION_EVIDENCE_MISSING".to_string());
    }
    for assessment in &plan.rule_assessments {
        match assessment.classification {
            RuleClassification::SourcePreserved | RuleClassification::VersionMigratedEquivalent => {}
            RuleClassification::ExplicitlyWaived => reason_codes.push(format!("RULE_EXPLICITLY_WAIVED:{}", assessment.rule_id)),
            RuleClassification::Inferred => reason_codes.push(format!("RULE_INFERRED:{}", assessment.rule_id)),
            RuleClassification::Relaxed => reason_codes.push(format!("RULE_RELAXED:{}", assessment.rule_id)),
            RuleClassification::Unavailable => reason_codes.push(format!("RULE_UNAVAILABLE:{}", assessment.rule_id)),
        }
    }
    if strict_source_drc.is_none() {
        reason_codes.push("STRICT_SOURCE_DRC_NOT_EVALUATED".to_string());
    } else if !strict_source_clean_pass && source_authoritative {
        reason_codes.push("STRICT_SOURCE_DRC_FAILED".to_string());
    }
    if !compatibility_drc.clean_pass {
        reason_codes.push("COMPATIBILITY_DRC_FAILED".to_string());
    }

    let source_rule_status = if !source_rule_profile_present {
        "unavailable"
    } else if !source_authoritative {
        "partially_inferred_relaxed_waived_or_unavailable"
    } else if plan.rule_assessments.iter().any(|r| r.classification == RuleClassification::VersionMigratedEquivalent) {
        "version_migrated_equivalent"
    } else {
        "source_preserved"
    };
    let disposition = if !compatibility_drc.clean_pass {
        "compatibility_policy_failed"
    } else if !source_authoritative {
        "compatibility_only_source_constraint_fidelity_unresolved"
    } else if !strict_source_policy_evaluated {
        "source_rules_pinned_strict_policy_not_evaluated"
    } else if !strict_source_clean_pass {
        "compatibility_pass_strict_source_policy_failed"
    } else {
        "compatibility_and_strict_source_pass"
    };

    let plan_sha = sha256_hex(plan_bytes);
    let constraint_fingerprint_sha = constraint_fingerprint(&plan)?;
    Ok(RuleFidelityReceipt {
        schema: RECEIPT_SCHEMA.to_string(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        plan_sha256: plan_sha,
        constraint_fingerprint_sha256: constraint_fingerprint_sha,
        benchmark_id: plan.benchmark_id,
        dataset_id: plan.dataset_id,
        split_id: plan.split_id,
        board_id: plan.board_id,
        source_board_sha256: source_board_sha,
        output_board_sha256: output_board_sha,
        source_rule_profile_sha256: plan.source_rule_profile_sha256,
        compatibility_policy_sha256: plan.compatibility_policy_sha256,
        strict_source_policy_sha256: plan.strict_source_policy_sha256,
        source_rule_status: source_rule_status.to_string(),
        source_authoritative,
        rule_classification_counts: class_counts,
        verified_rule_evidence,
        compatibility_policy_clean_pass: compatibility_drc.clean_pass,
        compatibility_drc,
        strict_source_drc,
        strict_source_policy_evaluated,
        strict_source_clean_pass,
        disposition: disposition.to_string(),
        reason_codes,
        authority: "source_and_evaluation_rule_fidelity_evidence_only_no_si_pi_or_physical_release_authority".into(),
        claim_ceiling: "A clean compatibility-policy DRC does not imply source-rule fidelity. Strict-source clean pass is credited only when source rule bytes are pinned, every rule is source-preserved or explicitly version-migrated with evidence, and the strict-source DRC report is clean. This receipt does not establish SI/PI, manufacturability, physical quality, safety or release readiness.".into(),
    })
}

pub fn compare_rule_fidelity(
    baseline: &RuleFidelityReceipt,
    treatment: &RuleFidelityReceipt,
) -> Result<RuleFidelityComparison, String> {
    if baseline.schema != RECEIPT_SCHEMA || treatment.schema != RECEIPT_SCHEMA {
        return Err("cannot compare unsupported rule-fidelity receipt schema".into());
    }
    if baseline.board_id != treatment.board_id
        || baseline.benchmark_id != treatment.benchmark_id
        || baseline.dataset_id != treatment.dataset_id
        || baseline.split_id != treatment.split_id
        || baseline.source_board_sha256 != treatment.source_board_sha256
    {
        return Err("cannot compare rule-fidelity receipts for different board/input identities".into());
    }
    if baseline.constraint_fingerprint_sha256 != treatment.constraint_fingerprint_sha256
        || baseline.compatibility_policy_sha256 != treatment.compatibility_policy_sha256
        || baseline.source_rule_profile_sha256 != treatment.source_rule_profile_sha256
        || baseline.strict_source_policy_sha256 != treatment.strict_source_policy_sha256
    {
        return Err("cannot compare routes evaluated under different source-rule or compatibility constraints".into());
    }
    let strict_available = baseline.strict_source_policy_evaluated
        && treatment.strict_source_policy_evaluated
        && baseline.source_authoritative
        && treatment.source_authoritative;
    Ok(RuleFidelityComparison {
        schema: COMPARISON_SCHEMA.to_string(),
        board_id: baseline.board_id.clone(),
        constraint_fingerprint_sha256: baseline.constraint_fingerprint_sha256.clone(),
        baseline_compatibility_clean_pass: baseline.compatibility_policy_clean_pass,
        treatment_compatibility_clean_pass: treatment.compatibility_policy_clean_pass,
        baseline_strict_source_clean_pass: baseline.strict_source_clean_pass,
        treatment_strict_source_clean_pass: treatment.strict_source_clean_pass,
        strict_source_comparison_available: strict_available,
        compatibility_outcome_changed: baseline.compatibility_policy_clean_pass != treatment.compatibility_policy_clean_pass,
        strict_source_outcome_changed: strict_available
            && baseline.strict_source_clean_pass != treatment.strict_source_clean_pass,
        authority: "paired_constraint_fidelity_outcomes_only_no_physical_or_production_release_authority".into(),
        claim_ceiling: "Only identical source board/rule profile, compatibility and strict-source policy pins are compared. A clean DRC result is not electrical or physical qualification.".into(),
    })
}

fn validate_plan_shape(plan: &RuleFidelityPlan) -> Result<(), String> {
    if plan.schema != PLAN_SCHEMA {
        return Err(format!("unsupported rule-fidelity plan schema: {}", plan.schema));
    }
    for (field, value) in [
        ("benchmark_id", plan.benchmark_id.as_str()),
        ("dataset_id", plan.dataset_id.as_str()),
        ("split_id", plan.split_id.as_str()),
        ("board_id", plan.board_id.as_str()),
        ("compatibility_kicad_version", plan.compatibility_kicad_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{field} must not be empty"));
        }
    }
    require_sha256(&plan.source_board_sha256, "source_board_sha256")?;
    require_sha256(&plan.output_board_sha256, "output_board_sha256")?;
    require_sha256(&plan.compatibility_policy_sha256, "compatibility_policy_sha256")?;
    validate_relative_path(&plan.source_board_relative_path, "source board path")?;
    validate_relative_path(&plan.output_board_relative_path, "output board path")?;
    validate_relative_path(&plan.compatibility_policy_relative_path, "compatibility policy path")?;
    validate_report_path(&plan.compatibility_drc_report_relative_path, "compatibility DRC report path")?;

    match (
        plan.source_rule_profile_relative_path.as_deref(),
        plan.source_rule_profile_sha256.as_deref(),
        plan.source_kicad_version.as_deref(),
    ) {
        (Some(path), Some(digest), Some(version)) => {
            validate_relative_path(path, "source rule-profile path")?;
            require_sha256(digest, "source_rule_profile_sha256")?;
            if version.trim().is_empty() {
                return Err("source_kicad_version must not be empty when source rule bytes are pinned".into());
            }
        }
        (None, None, None) => {}
        _ => return Err("source rule-profile path/digest/version must be supplied together or all absent".into()),
    }
    match (
        plan.strict_source_policy_relative_path.as_deref(),
        plan.strict_source_policy_sha256.as_deref(),
        plan.strict_source_kicad_version.as_deref(),
        plan.strict_source_drc_report_relative_path.as_deref(),
        plan.strict_source_policy_derivation_evidence_relative_path.as_deref(),
        plan.strict_source_policy_derivation_evidence_sha256.as_deref(),
    ) {
        (Some(policy), Some(digest), Some(version), Some(report), Some(derivation_path), Some(derivation_sha)) => {
            validate_relative_path(policy, "strict source policy path")?;
            validate_report_path(report, "strict source DRC report path")?;
            validate_relative_path(derivation_path, "strict-source policy derivation evidence path")?;
            require_sha256(digest, "strict_source_policy_sha256")?;
            require_sha256(derivation_sha, "strict_source_policy_derivation_evidence_sha256")?;
            if version.trim().is_empty() {
                return Err("strict_source_kicad_version must not be empty".into());
            }
        }
        (None, None, None, None, None, None) => {}
        _ => return Err("strict-source policy, report, and derivation evidence pins must be supplied together or all absent".into()),
    }
    if plan.rule_assessments.is_empty() {
        return Err("rule_assessments must enumerate every required rule or explicitly mark unavailable".into());
    }
    let mut seen = BTreeSet::new();
    for assessment in &plan.rule_assessments {
        if assessment.rule_id.trim().is_empty() || !seen.insert(assessment.rule_id.as_str()) {
            return Err(format!("empty or duplicate rule ID: {}", assessment.rule_id));
        }
        validate_assessment(assessment, plan.source_rule_profile_sha256.is_some())?;
    }
    Ok(())
}

fn validate_assessment(assessment: &RuleAssessment, source_profile_present: bool) -> Result<(), String> {
    let rationale_required = |value: &str| !value.trim().is_empty();
    match assessment.classification {
        RuleClassification::SourcePreserved => {
            if !source_profile_present
                || assessment.source_value.as_deref().is_none_or(str::is_empty)
                || assessment.evaluated_value.as_deref().is_none_or(str::is_empty)
                || assessment.source_value != assessment.evaluated_value
            {
                return Err(format!("{}: source_preserved requires pinned source bytes and equal non-empty source/evaluated values", assessment.rule_id));
            }
        }
        RuleClassification::VersionMigratedEquivalent => {
            if !source_profile_present
                || assessment.source_value.as_deref().is_none_or(str::is_empty)
                || assessment.evaluated_value.as_deref().is_none_or(str::is_empty)
                || assessment.mapping_evidence_relative_path.as_deref().is_none_or(str::is_empty)
                || assessment.mapping_evidence_sha256.as_deref().is_none_or(|v| !is_sha256(v))
                || !rationale_required(&assessment.rationale)
            {
                return Err(format!("{}: version_migrated_equivalent requires source/evaluated values, mapping evidence path/digest and rationale", assessment.rule_id));
            }
        }
        RuleClassification::ExplicitlyWaived => {
            if !source_profile_present
                || assessment.waiver_evidence_relative_path.as_deref().is_none_or(str::is_empty)
                || assessment.waiver_evidence_sha256.as_deref().is_none_or(|v| !is_sha256(v))
                || !rationale_required(&assessment.rationale)
            {
                return Err(format!("{}: explicitly_waived requires pinned source rules, waiver evidence path/digest and rationale", assessment.rule_id));
            }
        }
        RuleClassification::Inferred => {
            if assessment.evaluated_value.as_deref().is_none_or(str::is_empty)
                || !rationale_required(&assessment.rationale)
            {
                return Err(format!("{}: inferred requires the inferred value and rationale", assessment.rule_id));
            }
        }
        RuleClassification::Relaxed => {
            if assessment.source_value.as_deref().is_none_or(str::is_empty)
                || assessment.evaluated_value.as_deref().is_none_or(str::is_empty)
                || !rationale_required(&assessment.rationale)
            {
                return Err(format!("{}: relaxed requires source/evaluated values and rationale", assessment.rule_id));
            }
        }
        RuleClassification::Unavailable => {
            if !rationale_required(&assessment.rationale) {
                return Err(format!("{}: unavailable requires a rationale", assessment.rule_id));
            }
        }
    }
    for (path, digest, label) in [
        (assessment.mapping_evidence_relative_path.as_deref(), assessment.mapping_evidence_sha256.as_deref(), "mapping_evidence_sha256"),
        (assessment.waiver_evidence_relative_path.as_deref(), assessment.waiver_evidence_sha256.as_deref(), "waiver_evidence_sha256"),
    ] {
        match (path, digest) {
            (Some(path), Some(digest)) => {
                validate_relative_path(path, label)?;
                require_sha256(digest, label)?;
            }
            (None, None) => {}
            _ => return Err(format!("{}: evidence path and SHA-256 must be present together", assessment.rule_id)),
        }
    }
    Ok(())
}

fn verify_rule_evidence(
    plan: &RuleFidelityPlan,
    evidence_files: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<VerifiedRuleEvidence>, String> {
    let mut expected = Vec::<(String, String, String)>::new();
    if let (Some(path), Some(digest)) = (
        plan.strict_source_policy_derivation_evidence_relative_path.as_deref(),
        plan.strict_source_policy_derivation_evidence_sha256.as_deref(),
    ) {
        expected.push((
            "strict_source_policy_derivation".to_string(),
            path.to_string(),
            digest.to_string(),
        ));
    }
    for assessment in &plan.rule_assessments {
        if let (Some(path), Some(digest)) = (
            assessment.mapping_evidence_relative_path.as_deref(),
            assessment.mapping_evidence_sha256.as_deref(),
        ) {
            expected.push((
                format!("rule_mapping:{}", assessment.rule_id),
                path.to_string(),
                digest.to_string(),
            ));
        }
        if let (Some(path), Some(digest)) = (
            assessment.waiver_evidence_relative_path.as_deref(),
            assessment.waiver_evidence_sha256.as_deref(),
        ) {
            expected.push((
                format!("rule_waiver:{}", assessment.rule_id),
                path.to_string(),
                digest.to_string(),
            ));
        }
    }

    let expected_paths: BTreeSet<&str> = expected.iter().map(|(_, path, _)| path.as_str()).collect();
    if evidence_files.keys().any(|path| !expected_paths.contains(path.as_str())) {
        return Err("rule-evidence input contains an unreferenced artifact".into());
    }
    let mut verified = Vec::with_capacity(expected.len());
    for (purpose, relative_path, expected_sha) in expected {
        validate_relative_path(&relative_path, "rule-evidence relative path")?;
        require_sha256(&expected_sha, "rule-evidence SHA-256")?;
        let bytes = evidence_files.get(&relative_path)
            .ok_or_else(|| format!("rule-evidence file is missing: {relative_path}"))?;
        if bytes.is_empty() {
            return Err(format!("rule-evidence file is empty: {relative_path}"));
        }
        let actual = sha256_hex(bytes);
        if actual != expected_sha {
            return Err(format!("rule-evidence bytes SHA-256 mismatch: {relative_path}"));
        }
        verified.push(VerifiedRuleEvidence {
            purpose,
            relative_path,
            byte_len: bytes.len() as u64,
            sha256: actual,
        });
    }
    verified.sort_by(|a, b| {
        a.relative_path
            .cmp(&b.relative_path)
            .then_with(|| a.purpose.cmp(&b.purpose))
    });
    Ok(verified)
}

fn validate_policy_bytes(
    bytes: &[u8],
    expected_sha256: &str,
    expected_version: &str,
    label: &str,
) -> Result<(), String> {
    if sha256_hex(bytes) != expected_sha256 {
        return Err(format!("{label} policy bytes SHA-256 does not match its pin"));
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("{label} policy JSON invalid: {error}"))?;
    if value.get("schema").and_then(Value::as_str) != Some("kicad-drc-routing-policy-v0.1") {
        return Err(format!("{label} policy has an unsupported schema"));
    }
    if value.get("kicad_version").and_then(Value::as_str) != Some(expected_version) {
        return Err(format!("{label} policy KiCad version differs from the declared version"));
    }
    let routing_types = value.get("routing_violation_types").and_then(Value::as_array)
        .ok_or_else(|| format!("{label} policy has no routing_violation_types array"))?;
    if routing_types.is_empty() || routing_types.iter().any(|v| v.as_str().is_none_or(str::is_empty)) {
        return Err(format!("{label} policy routing_violation_types must contain non-empty strings"));
    }
    let severities = value.get("blocking_severities").and_then(Value::as_array)
        .ok_or_else(|| format!("{label} policy has no blocking_severities array"))?;
    if severities.is_empty() || !severities.iter().any(|item| item.as_str() == Some("error")) {
        return Err(format!("{label} policy must block error-severity violations"));
    }
    if value.get("block_unconnected_items").and_then(Value::as_bool) != Some(true) {
        return Err(format!("{label} policy must block unconnected items"));
    }
    Ok(())
}

fn parse_drc_report(bytes: &[u8]) -> Result<DrcReportSummary, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("KiCad DRC report JSON invalid: {error}"))?;
    let violations = value.get("violations").and_then(Value::as_array)
        .ok_or_else(|| "KiCad DRC report must contain a violations array".to_string())?;
    let unconnected = value.get("unconnected_items").and_then(Value::as_array)
        .ok_or_else(|| "KiCad DRC report must contain an unconnected_items array".to_string())?;
    let mut counts = BTreeMap::new();
    for violation in violations {
        let kind = violation.get("type").and_then(Value::as_str)
            .or_else(|| violation.get("error_type").and_then(Value::as_str))
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| "DRC violation entry has no non-empty type/error_type".to_string())?;
        *counts.entry(kind.to_string()).or_insert(0usize) += 1;
    }
    Ok(DrcReportSummary {
        report_sha256: sha256_hex(bytes),
        violation_count: violations.len(),
        violation_type_counts: counts,
        unconnected_item_count: unconnected.len(),
        clean_pass: violations.is_empty() && unconnected.is_empty(),
    })
}

fn constraint_fingerprint(plan: &RuleFidelityPlan) -> Result<String, String> {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        benchmark_id: &'a str,
        dataset_id: &'a str,
        split_id: &'a str,
        board_id: &'a str,
        source_board_sha256: &'a str,
        source_rule_profile_sha256: &'a Option<String>,
        source_kicad_version: &'a Option<String>,
        compatibility_policy_sha256: &'a str,
        compatibility_kicad_version: &'a str,
        strict_source_policy_sha256: &'a Option<String>,
        strict_source_kicad_version: &'a Option<String>,
        rule_assessments: &'a [RuleAssessment],
    }
    let fingerprint = Fingerprint {
        benchmark_id: &plan.benchmark_id,
        dataset_id: &plan.dataset_id,
        split_id: &plan.split_id,
        board_id: &plan.board_id,
        source_board_sha256: &plan.source_board_sha256,
        source_rule_profile_sha256: &plan.source_rule_profile_sha256,
        source_kicad_version: &plan.source_kicad_version,
        compatibility_policy_sha256: &plan.compatibility_policy_sha256,
        compatibility_kicad_version: &plan.compatibility_kicad_version,
        strict_source_policy_sha256: &plan.strict_source_policy_sha256,
        strict_source_kicad_version: &plan.strict_source_kicad_version,
        rule_assessments: &plan.rule_assessments,
    };
    let bytes = serde_json::to_vec(&fingerprint)
        .map_err(|error| format!("cannot serialize canonical rule-fidelity fingerprint: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn classification_name(classification: RuleClassification) -> &'static str {
    match classification {
        RuleClassification::SourcePreserved => "source_preserved",
        RuleClassification::VersionMigratedEquivalent => "version_migrated_equivalent",
        RuleClassification::ExplicitlyWaived => "explicitly_waived",
        RuleClassification::Inferred => "inferred",
        RuleClassification::Relaxed => "relaxed",
        RuleClassification::Unavailable => "unavailable",
    }
}

fn validate_report_path(raw: &str, label: &str) -> Result<(), String> {
    validate_relative_path(raw, label)?;
    if !raw.to_ascii_lowercase().ends_with(".json") {
        return Err(format!("{label} must end with .json"));
    }
    Ok(())
}

fn validate_relative_path(raw: &str, label: &str) -> Result<(), String> {
    if raw.trim().is_empty() || raw.contains('\\') || raw.contains(':') {
        return Err(format!("{label} must be a safe relative path"));
    }
    let path = Path::new(raw);
    if path.is_absolute()
        || path.components().any(|component| !matches!(component, Component::Normal(_)))
        || raw.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("{label} must be a safe relative path"));
    }
    Ok(())
}

fn require_sha256(value: &str, label: &str) -> Result<(), String> {
    if !is_sha256(value) {
        return Err(format!("{label} must be a 64-character hexadecimal SHA-256"));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_commit_sha(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "abababababababababababababababababababab";
    const SOURCE_BOARD: &[u8] = b"synthetic source board fixture; not a KiCad board";
    const OUTPUT_A: &[u8] = b"synthetic output board A; not a KiCad board";
    const OUTPUT_B: &[u8] = b"synthetic output board B; not a KiCad board";
    const SOURCE_RULES: &[u8] = b"synthetic source rule profile fixture";
    const COMPAT_POLICY: &str = r#"{
      "schema":"kicad-drc-routing-policy-v0.1",
      "kicad_version":"9.0.8",
      "routing_violation_types":["clearance","tracks_crossing","unconnected_items"]
    }"#;
    const STRICT_POLICY: &str = r#"{
      "schema":"kicad-drc-routing-policy-v0.1",
      "kicad_version":"9.0.8",
      "routing_violation_types":["clearance","tracks_crossing","unconnected_items"]
    }"#;
    const COMPAT_CLEAN: &str = r#"{"violations":[],"unconnected_items":[]}"#;
    const STRICT_FAIL: &str = r#"{"violations":[{"type":"clearance","severity":"error"}],"unconnected_items":[]}"#;
    const STRICT_CLEAN: &str = r#"{"violations":[],"unconnected_items":[]}"#;

    fn policy_sha(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    fn plan(output: &[u8], assessments: Vec<RuleAssessment>, strict: bool) -> RuleFidelityPlan {
        let compat = COMPAT_POLICY.as_bytes();
        let strict_bytes = STRICT_POLICY.as_bytes();
        RuleFidelityPlan {
            schema: PLAN_SCHEMA.into(),
            benchmark_id: "synthetic-rule-fidelity".into(),
            dataset_id: "synthetic-route-fixtures".into(),
            split_id: "synthetic-v0.1".into(),
            board_id: "synthetic-board-001".into(),
            source_board_relative_path: "boards/synthetic-board-001.kicad_pcb".into(),
            source_board_sha256: sha256_hex(SOURCE_BOARD),
            output_board_relative_path: "out/board.kicad_pcb".into(),
            output_board_sha256: sha256_hex(output),
            source_rule_profile_relative_path: Some("boards/synthetic-board-001.kicad_pro".into()),
            source_rule_profile_sha256: Some(sha256_hex(SOURCE_RULES)),
            source_kicad_version: Some("9.0.8".into()),
            compatibility_policy_relative_path: "policies/compatibility.json".into(),
            compatibility_policy_sha256: policy_sha(compat),
            compatibility_kicad_version: "9.0.8".into(),
            compatibility_drc_report_relative_path: "drc/compatibility.json".into(),
            strict_source_policy_relative_path: strict.then(|| "policies/strict-source.json".into()),
            strict_source_policy_sha256: strict.then(|| policy_sha(strict_bytes)),
            strict_source_kicad_version: strict.then(|| "9.0.8".into()),
            strict_source_drc_report_relative_path: strict.then(|| "drc/strict-source.json".into()),
            rule_assessments: assessments,
        }
    }

    fn preserved_rule() -> RuleAssessment {
        RuleAssessment {
            rule_id: "clearance.default".into(),
            classification: RuleClassification::SourcePreserved,
            source_value: Some("0.20mm".into()),
            evaluated_value: Some("0.20mm".into()),
            mapping_evidence_sha256: None,
            waiver_evidence_sha256: None,
            rationale: "source and evaluation settings are byte-audited equal".into(),
        }
    }

    fn evaluate(
        plan: &RuleFidelityPlan,
        compat_drc: &[u8],
        strict_drc: Option<&[u8]>,
    ) -> RuleFidelityReceipt {
        evaluate_rule_fidelity(
            &serde_json::to_vec(plan).unwrap(),
            SOURCE_BOARD,
            if plan.output_board_sha256 == sha256_hex(OUTPUT_A) { OUTPUT_A } else { OUTPUT_B },
            Some(SOURCE_RULES),
            COMPAT_POLICY.as_bytes(),
            compat_drc,
            plan.strict_source_policy_sha256.as_ref().map(|_| STRICT_POLICY.as_bytes()),
            strict_drc,
            HEAD,
        ).unwrap()
    }

    #[test]
    fn compatibility_pass_does_not_hide_a_strict_source_rule_failure() {
        let plan = plan(OUTPUT_A, vec![preserved_rule()], true);
        let receipt = evaluate(&plan, COMPAT_CLEAN.as_bytes(), Some(STRICT_FAIL.as_bytes()));
        assert!(receipt.compatibility_policy_clean_pass);
        assert!(receipt.source_authoritative);
        assert!(receipt.strict_source_policy_evaluated);
        assert!(!receipt.strict_source_clean_pass);
        assert_eq!(receipt.disposition, "compatibility_pass_strict_source_policy_failed");
    }

    #[test]
    fn strict_source_pass_requires_every_rule_to_be_source_preserved_or_equivalent() {
        let inferred = RuleAssessment {
            rule_id: "clearance.missing_source_value".into(),
            classification: RuleClassification::Inferred,
            source_value: None,
            evaluated_value: Some("0.20mm".into()),
            mapping_evidence_sha256: None,
            waiver_evidence_sha256: None,
            rationale: "source setting was reconstructed from existing copper".into(),
        };
        let plan = plan(OUTPUT_A, vec![preserved_rule(), inferred], true);
        let receipt = evaluate(&plan, COMPAT_CLEAN.as_bytes(), Some(STRICT_CLEAN.as_bytes()));
        assert!(receipt.compatibility_policy_clean_pass);
        assert!(!receipt.source_authoritative);
        assert!(!receipt.strict_source_clean_pass);
        assert!(receipt.reason_codes.iter().any(|r| r == "RULE_INFERRED:clearance.missing_source_value"));
    }

    #[test]
    fn absent_strict_report_is_unresolved_not_a_pass() {
        let plan = plan(OUTPUT_A, vec![preserved_rule()], false);
        let receipt = evaluate(&plan, COMPAT_CLEAN.as_bytes(), None);
        assert!(receipt.compatibility_policy_clean_pass);
        assert!(!receipt.strict_source_policy_evaluated);
        assert!(!receipt.strict_source_clean_pass);
        assert_eq!(receipt.disposition, "source_rules_pinned_strict_policy_not_evaluated");
    }

    #[test]
    fn tampered_source_or_output_bytes_reject() {
        let plan = plan(OUTPUT_A, vec![preserved_rule()], true);
        let bytes = serde_json::to_vec(&plan).unwrap();
        assert!(evaluate_rule_fidelity(
            &bytes, b"tampered source", OUTPUT_A, Some(SOURCE_RULES),
            COMPAT_POLICY.as_bytes(), COMPAT_CLEAN.as_bytes(),
            Some(STRICT_POLICY.as_bytes()), Some(STRICT_CLEAN.as_bytes()), HEAD
        ).unwrap_err().contains("source-board bytes SHA-256"));
        assert!(evaluate_rule_fidelity(
            &bytes, SOURCE_BOARD, b"tampered output", Some(SOURCE_RULES),
            COMPAT_POLICY.as_bytes(), COMPAT_CLEAN.as_bytes(),
            Some(STRICT_POLICY.as_bytes()), Some(STRICT_CLEAN.as_bytes()), HEAD
        ).unwrap_err().contains("output-board bytes SHA-256"));
    }

    #[test]
    fn altered_policy_bytes_or_wrong_report_shape_reject() {
        let plan = plan(OUTPUT_A, vec![preserved_rule()], true);
        assert!(evaluate_rule_fidelity(
            &serde_json::to_vec(&plan).unwrap(), SOURCE_BOARD, OUTPUT_A, Some(SOURCE_RULES),
            br#"{"schema":"kicad-drc-routing-policy-v0.1","kicad_version":"9.0.8","routing_violation_types":["clearance"]}"#,
            COMPAT_CLEAN.as_bytes(), Some(STRICT_POLICY.as_bytes()), Some(STRICT_CLEAN.as_bytes()), HEAD
        ).unwrap_err().contains("policy bytes SHA-256"));
        assert!(evaluate_rule_fidelity(
            &serde_json::to_vec(&plan).unwrap(), SOURCE_BOARD, OUTPUT_A, Some(SOURCE_RULES),
            COMPAT_POLICY.as_bytes(), br#"{"violations":[]}"#,
            Some(STRICT_POLICY.as_bytes()), Some(STRICT_CLEAN.as_bytes()), HEAD
        ).unwrap_err().contains("unconnected_items array"));
    }

    #[test]
    fn source_profile_and_migration_classifications_fail_closed() {
        let migrated = RuleAssessment {
            rule_id: "clearance.edge".into(),
            classification: RuleClassification::VersionMigratedEquivalent,
            source_value: Some("0.25mm".into()),
            evaluated_value: Some("0.25mm effective".into()),
            mapping_evidence_sha256: Some("e".repeat(64)),
            waiver_evidence_sha256: None,
            rationale: "pinned migration mapping says equivalent".into(),
        };
        let good = plan(OUTPUT_A, vec![migrated], true);
        let receipt = evaluate(&good, COMPAT_CLEAN.as_bytes(), Some(STRICT_CLEAN.as_bytes()));
        assert!(receipt.source_authoritative);
        assert!(receipt.strict_source_clean_pass);
        assert_eq!(receipt.source_rule_status, "version_migrated_equivalent");

        let invalid = RuleAssessment {
            rule_id: "clearance.edge".into(),
            mapping_evidence_sha256: None,
            ..good.rule_assessments[0].clone()
        };
        let bad = plan(OUTPUT_A, vec![invalid], true);
        assert!(evaluate_rule_fidelity(
            &serde_json::to_vec(&bad).unwrap(), SOURCE_BOARD, OUTPUT_A, Some(SOURCE_RULES),
            COMPAT_POLICY.as_bytes(), COMPAT_CLEAN.as_bytes(), Some(STRICT_POLICY.as_bytes()),
            Some(STRICT_CLEAN.as_bytes()), HEAD
        ).unwrap_err().contains("mapping evidence digest"));
    }

    #[test]
    fn paired_fidelity_requires_identical_constraint_fingerprint() {
        let plan = plan(OUTPUT_A, vec![preserved_rule()], true);
        let baseline = evaluate(&plan, COMPAT_CLEAN.as_bytes(), Some(STRICT_CLEAN.as_bytes()));
        let mut altered_plan = plan.clone();
        altered_plan.rule_assessments[0].evaluated_value = Some("0.30mm".into());
        altered_plan.rule_assessments[0].classification = RuleClassification::Relaxed;
        altered_plan.rule_assessments[0].rationale = "deliberate mismatch negative control".into();
        let altered = evaluate(&plan_to_receipt_bytes_variant(&altered_plan, OUTPUT_B), COMPAT_CLEAN.as_bytes(), Some(STRICT_CLEAN.as_bytes()));
        assert!(compare_rule_fidelity(&baseline, &altered).unwrap_err().contains("different source-rule or compatibility constraints"));
    }

    fn plan_to_receipt_bytes_variant(plan: &RuleFidelityPlan, output: &[u8]) -> RuleFidelityPlan {
        let mut altered = plan.clone();
        altered.output_board_sha256 = sha256_hex(output);
        altered
    }
}

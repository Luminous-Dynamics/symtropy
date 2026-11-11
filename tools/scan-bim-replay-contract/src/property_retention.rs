// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Requirement-weighted property-retention audit for IFC representation changes.
//!
//! This module consumes declared comparison/correspondence receipts. It does
//! not parse IFC, compute semantic diffs, normalize units, or infer project
//! criticality. Those authorities remain with the existing interop owners.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_RECEIPT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RequirementCompleteness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PropertyImportance {
    Critical,
    Important,
    Advisory,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PropertyValueState {
    Present,
    Absent,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PropertyDisposition {
    PreservedExact,
    PreservedNormalized,
    Changed,
    Lost,
    Added,
    Unsupported,
    Unresolved,
    AbsentBoth,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyRetentionReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub scope: PropertyRetentionScope,
    pub requirements: Vec<PropertyRequirement>,
    pub comparisons: Vec<PropertyComparison>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyRetentionScope {
    pub replay_run_id: String,
    pub scene_id: String,
    pub source_model_revision: String,
    pub target_model_revision: String,
    pub exchange_profile_id: String,
    pub source_ifc_sha256: String,
    pub target_ifc_sha256: String,
    pub requirement_profile_sha256: String,
    pub requirement_completeness: RequirementCompleteness,
    pub semantic_diff_report_sha256: String,
    pub identity_correspondence_report_sha256: String,
    /// Required only when at least one row claims unit-normalized equivalence.
    pub unit_comparison_report_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyRequirement {
    pub requirement_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub ifc_entity_type: String,
    pub property_set: String,
    pub property_name: String,
    pub data_type: String,
    /// Project-declared criticality. This is never inferred from property name.
    pub importance: PropertyImportance,
    pub required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyComparison {
    pub comparison_record_id: String,
    pub requirement_id: String,
    pub correspondence_event_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub source_state: PropertyValueState,
    pub target_state: PropertyValueState,
    pub disposition: PropertyDisposition,
    pub source_value_sha256: Option<String>,
    pub target_value_sha256: Option<String>,
    /// Present only when an independent unit/semantic normalizer emits a
    /// canonical value identity. This module does not calculate it.
    pub canonical_value_sha256: Option<String>,
    pub semantic_finding_id: Option<String>,
    pub unit_comparison_finding_id: Option<String>,
    pub comparison_evidence_sha256: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PropertyRetentionFinding {
    pub requirement_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub property_set: String,
    pub property_name: String,
    pub data_type: String,
    pub importance: String,
    pub required: bool,
    pub source_state: String,
    pub target_state: String,
    pub disposition: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PropertyRetentionReport {
    pub schema_version: String,
    pub receipt_id: String,
    pub input_sha256: String,
    pub replay_run_id: String,
    pub scene_id: String,
    pub source_ifc_sha256: String,
    pub target_ifc_sha256: String,
    pub requirement_profile_sha256: String,
    pub requirement_completeness: String,
    pub requirement_count: usize,
    pub comparison_count: usize,
    pub required_property_count: usize,
    pub source_present_required_count: usize,
    pub preserved_required_count: usize,
    pub required_property_retention_ratio: Option<f64>,
    pub critical_required_count: usize,
    pub critical_loss_count: usize,
    pub critical_requirement_gap_count: usize,
    pub critical_unresolved_count: usize,
    pub important_degraded_count: usize,
    pub added_property_count: usize,
    pub changed_property_count: usize,
    pub lost_property_count: usize,
    pub unsupported_property_count: usize,
    pub unresolved_property_count: usize,
    pub absent_both_property_count: usize,
    pub preservation_state: String,
    pub findings: Vec<PropertyRetentionFinding>,
    pub claim_ceiling: &'static str,
}

/// Audits exact receipt bytes. Referenced producer artifacts are not opened or
/// authenticated here; digest binding is a structural receipt check only.
pub fn audit_property_retention_bytes(
    bytes: &[u8],
) -> Result<PropertyRetentionReport, String> {
    if bytes.is_empty() {
        return Err("property-retention receipt must not be empty".to_owned());
    }
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(format!(
            "property-retention receipt exceeds {MAX_RECEIPT_BYTES} bytes"
        ));
    }
    let receipt: PropertyRetentionReceipt = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid property-retention receipt: {error}"))?;
    let digest = sha256_hex(bytes);
    audit_property_retention(receipt, digest)
}

fn audit_property_retention(
    receipt: PropertyRetentionReceipt,
    input_sha256: String,
) -> Result<PropertyRetentionReport, String> {
    if receipt.schema_version != "0.1" {
        return Err("unsupported property-retention schema_version".to_owned());
    }
    for (field, value) in [
        ("receipt_id", receipt.receipt_id.as_str()),
        ("scope.replay_run_id", receipt.scope.replay_run_id.as_str()),
        ("scope.scene_id", receipt.scope.scene_id.as_str()),
        (
            "scope.source_model_revision",
            receipt.scope.source_model_revision.as_str(),
        ),
        (
            "scope.target_model_revision",
            receipt.scope.target_model_revision.as_str(),
        ),
        ("scope.exchange_profile_id", receipt.scope.exchange_profile_id.as_str()),
    ] {
        validate_identifier(field, value)?;
    }
    for (field, digest) in [
        ("scope.source_ifc_sha256", receipt.scope.source_ifc_sha256.as_str()),
        ("scope.target_ifc_sha256", receipt.scope.target_ifc_sha256.as_str()),
        (
            "scope.requirement_profile_sha256",
            receipt.scope.requirement_profile_sha256.as_str(),
        ),
        (
            "scope.semantic_diff_report_sha256",
            receipt.scope.semantic_diff_report_sha256.as_str(),
        ),
        (
            "scope.identity_correspondence_report_sha256",
            receipt.scope.identity_correspondence_report_sha256.as_str(),
        ),
    ] {
        validate_sha256(field, digest)?;
    }
    if let Some(digest) = receipt.scope.unit_comparison_report_sha256.as_deref() {
        validate_sha256("scope.unit_comparison_report_sha256", digest)?;
    }
    if receipt.requirements.is_empty() {
        return Err("property-retention receipt must declare at least one requirement".to_owned());
    }
    if receipt.requirements.len() != receipt.comparisons.len() {
        return Err(format!(
            "each property requirement needs exactly one comparison row: {} requirements, {} comparisons",
            receipt.requirements.len(),
            receipt.comparisons.len()
        ));
    }

    let mut requirement_ids = BTreeSet::new();
    let mut requirements = BTreeMap::<&str, &PropertyRequirement>::new();
    for requirement in &receipt.requirements {
        for (field, value) in [
            ("requirement_id", requirement.requirement_id.as_str()),
            ("source_entity_id", requirement.source_entity_id.as_str()),
            ("target_entity_id", requirement.target_entity_id.as_str()),
            ("ifc_entity_type", requirement.ifc_entity_type.as_str()),
            ("property_set", requirement.property_set.as_str()),
            ("property_name", requirement.property_name.as_str()),
            ("data_type", requirement.data_type.as_str()),
        ] {
            validate_identifier(field, value)?;
        }
        if requirement.importance == PropertyImportance::Critical && !requirement.required {
            return Err(format!(
                "critical property requirement {} must be required",
                requirement.requirement_id
            ));
        }
        if !requirement_ids.insert(requirement.requirement_id.as_str()) {
            return Err(format!(
                "duplicate property requirement ID: {}",
                requirement.requirement_id
            ));
        }
        requirements.insert(requirement.requirement_id.as_str(), requirement);
    }

    let mut comparison_ids = BTreeSet::new();
    let mut comparisons = BTreeMap::<&str, &PropertyComparison>::new();
    let mut needs_unit_report = false;
    for comparison in &receipt.comparisons {
        for (field, value) in [
            ("comparison_record_id", comparison.comparison_record_id.as_str()),
            ("requirement_id", comparison.requirement_id.as_str()),
            ("correspondence_event_id", comparison.correspondence_event_id.as_str()),
        ] {
            validate_identifier(field, value)?;
        }
        if !comparison_ids.insert(comparison.comparison_record_id.as_str()) {
            return Err(format!(
                "duplicate property comparison record ID: {}",
                comparison.comparison_record_id
            ));
        }
        let requirement = requirements
            .get(comparison.requirement_id.as_str())
            .ok_or_else(|| {
                format!(
                    "comparison {} references an undeclared requirement",
                    comparison.comparison_record_id
                )
            })?;
        if comparison.source_entity_id != requirement.source_entity_id
            || comparison.target_entity_id != requirement.target_entity_id
        {
            return Err(format!(
                "comparison {} entity binding does not match requirement {}",
                comparison.comparison_record_id, comparison.requirement_id
            ));
        }
        validate_sha256(
            "comparison_evidence_sha256",
            &comparison.comparison_evidence_sha256,
        )?;
        validate_value_state(
            "source",
            comparison.source_state,
            comparison.source_value_sha256.as_deref(),
        )?;
        validate_value_state(
            "target",
            comparison.target_state,
            comparison.target_value_sha256.as_deref(),
        )?;
        if let Some(digest) = comparison.canonical_value_sha256.as_deref() {
            validate_sha256("canonical_value_sha256", digest)?;
        }
        if let Some(id) = comparison.semantic_finding_id.as_deref() {
            validate_identifier("semantic_finding_id", id)?;
        }
        if let Some(id) = comparison.unit_comparison_finding_id.as_deref() {
            validate_identifier("unit_comparison_finding_id", id)?;
        }
        validate_comparison_disposition(comparison, &receipt.scope)?;
        needs_unit_report |= comparison.disposition == PropertyDisposition::PreservedNormalized;
        comparisons.insert(comparison.requirement_id.as_str(), comparison);
    }
    if needs_unit_report && receipt.scope.unit_comparison_report_sha256.is_none() {
        return Err(
            "preserved_normalized requires a scoped unit-comparison report digest".to_owned(),
        );
    }
    for requirement_id in requirements.keys() {
        if !comparisons.contains_key(requirement_id) {
            return Err(format!(
                "missing property comparison for requirement {requirement_id}"
            ));
        }
    }

    let mut required_property_count = 0usize;
    let mut source_present_required_count = 0usize;
    let mut preserved_required_count = 0usize;
    let mut critical_required_count = 0usize;
    let mut critical_loss_count = 0usize;
    let mut critical_requirement_gap_count = 0usize;
    let mut critical_unresolved_count = 0usize;
    let mut important_degraded_count = 0usize;
    let mut added_property_count = 0usize;
    let mut changed_property_count = 0usize;
    let mut lost_property_count = 0usize;
    let mut unsupported_property_count = 0usize;
    let mut unresolved_property_count = 0usize;
    let mut absent_both_property_count = 0usize;
    let mut findings = Vec::with_capacity(receipt.requirements.len());

    for requirement in &receipt.requirements {
        let comparison = comparisons
            .get(requirement.requirement_id.as_str())
            .expect("comparison cardinality was checked above");
        if requirement.required {
            required_property_count += 1;
            if comparison.source_state == PropertyValueState::Present {
                source_present_required_count += 1;
            }
            if matches!(
                comparison.disposition,
                PropertyDisposition::PreservedExact | PropertyDisposition::PreservedNormalized
            ) {
                preserved_required_count += 1;
            }
            if requirement.importance == PropertyImportance::Critical {
                critical_required_count += 1;
                if matches!(
                    comparison.disposition,
                    PropertyDisposition::Changed | PropertyDisposition::Lost
                ) {
                    critical_loss_count += 1;
                }
                if comparison.disposition == PropertyDisposition::AbsentBoth {
                    critical_requirement_gap_count += 1;
                }
                if matches!(
                    comparison.disposition,
                    PropertyDisposition::Unsupported | PropertyDisposition::Unresolved
                ) {
                    critical_unresolved_count += 1;
                }
            }
            if requirement.importance == PropertyImportance::Important
                && matches!(
                    comparison.disposition,
                    PropertyDisposition::Changed
                        | PropertyDisposition::Lost
                        | PropertyDisposition::AbsentBoth
                )
            {
                important_degraded_count += 1;
            }
        }

        match comparison.disposition {
            PropertyDisposition::Added => added_property_count += 1,
            PropertyDisposition::Changed => changed_property_count += 1,
            PropertyDisposition::Lost => lost_property_count += 1,
            PropertyDisposition::Unsupported => unsupported_property_count += 1,
            PropertyDisposition::Unresolved => unresolved_property_count += 1,
            PropertyDisposition::AbsentBoth => absent_both_property_count += 1,
            PropertyDisposition::PreservedExact | PropertyDisposition::PreservedNormalized => {}
        }
        findings.push(PropertyRetentionFinding {
            requirement_id: requirement.requirement_id.clone(),
            source_entity_id: requirement.source_entity_id.clone(),
            target_entity_id: requirement.target_entity_id.clone(),
            property_set: requirement.property_set.clone(),
            property_name: requirement.property_name.clone(),
            data_type: requirement.data_type.clone(),
            importance: label_importance(requirement.importance).to_owned(),
            required: requirement.required,
            source_state: label_value_state(comparison.source_state).to_owned(),
            target_state: label_value_state(comparison.target_state).to_owned(),
            disposition: label_disposition(comparison.disposition).to_owned(),
            reason: comparison.reason.clone(),
        });
    }
    findings.sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));

    let retention_ratio = if source_present_required_count > 0 {
        Some(preserved_required_count as f64 / source_present_required_count as f64)
    } else {
        None
    };
    let important_unresolved = receipt.requirements.iter().any(|requirement| {
        requirement.required
            && requirement.importance == PropertyImportance::Important
            && comparisons
                .get(requirement.requirement_id.as_str())
                .is_some_and(|comparison| {
                    matches!(
                        comparison.disposition,
                        PropertyDisposition::Unsupported | PropertyDisposition::Unresolved
                    )
                })
    });
    let advisory_degraded = receipt.requirements.iter().any(|requirement| {
        requirement.importance == PropertyImportance::Advisory
            && comparisons
                .get(requirement.requirement_id.as_str())
                .is_some_and(|comparison| {
                    matches!(
                        comparison.disposition,
                        PropertyDisposition::Changed
                            | PropertyDisposition::Lost
                            | PropertyDisposition::Unsupported
                            | PropertyDisposition::Unresolved
                            | PropertyDisposition::AbsentBoth
                    )
                })
    });
    let preservation_state = if critical_loss_count > 0 || critical_requirement_gap_count > 0 {
        "blocked"
    } else if receipt.scope.requirement_completeness != RequirementCompleteness::Complete
        || critical_unresolved_count > 0
        || important_unresolved
    {
        "unresolved"
    } else if important_degraded_count > 0 || advisory_degraded {
        "degraded"
    } else {
        "preserved"
    };

    Ok(PropertyRetentionReport {
        schema_version: "0.1".to_owned(),
        receipt_id: receipt.receipt_id,
        input_sha256,
        replay_run_id: receipt.scope.replay_run_id,
        scene_id: receipt.scope.scene_id,
        source_ifc_sha256: receipt.scope.source_ifc_sha256,
        target_ifc_sha256: receipt.scope.target_ifc_sha256,
        requirement_profile_sha256: receipt.scope.requirement_profile_sha256,
        requirement_completeness: label_completeness(receipt.scope.requirement_completeness).to_owned(),
        requirement_count: receipt.requirements.len(),
        comparison_count: receipt.comparisons.len(),
        required_property_count,
        source_present_required_count,
        preserved_required_count,
        required_property_retention_ratio: retention_ratio,
        critical_required_count,
        critical_loss_count,
        critical_requirement_gap_count,
        critical_unresolved_count,
        important_degraded_count,
        added_property_count,
        changed_property_count,
        lost_property_count,
        unsupported_property_count,
        unresolved_property_count,
        absent_both_property_count,
        preservation_state: preservation_state.to_owned(),
        findings,
        claim_ceiling: "SYNTHETIC_OR_RECEIPT-SEMANTIC_PROPERTY-COMPARISON_ONLY_NO_IFC-PARSING_UNIT-CONVERSION_OR_ENGINEERING-VALIDATION",
    })
}

fn validate_comparison_disposition(
    comparison: &PropertyComparison,
    scope: &PropertyRetentionScope,
) -> Result<(), String> {
    let source = comparison.source_state;
    let target = comparison.target_state;
    let source_hash = comparison.source_value_sha256.as_deref();
    let target_hash = comparison.target_value_sha256.as_deref();
    let reason = comparison.reason.as_deref().filter(|value| !value.trim().is_empty());
    let semantic_finding = comparison
        .semantic_finding_id
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    match comparison.disposition {
        PropertyDisposition::PreservedExact => {
            if source != PropertyValueState::Present
                || target != PropertyValueState::Present
                || source_hash != target_hash
                || comparison.canonical_value_sha256.is_some()
                || comparison.semantic_finding_id.is_some()
                || comparison.unit_comparison_finding_id.is_some()
                || reason.is_some()
            {
                return Err(format!(
                    "preserved_exact row {} requires identical present value digests and no normalization/reason",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::PreservedNormalized => {
            let canonical = comparison
                .canonical_value_sha256
                .as_deref()
                .ok_or_else(|| {
                    format!(
                        "preserved_normalized row {} requires canonical value digest",
                        comparison.comparison_record_id
                    )
                })?;
            validate_sha256("canonical_value_sha256", canonical)?;
            if source != PropertyValueState::Present
                || target != PropertyValueState::Present
                || source_hash == target_hash
                || scope.unit_comparison_report_sha256.is_none()
                || comparison.semantic_finding_id.is_some()
                || comparison
                    .unit_comparison_finding_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                || reason.is_some()
            {
                return Err(format!(
                    "preserved_normalized row {} requires unequal present raw values and a scoped unit-comparison finding",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::Changed => {
            if source != PropertyValueState::Present
                || target != PropertyValueState::Present
                || source_hash == target_hash
                || semantic_finding.is_none()
                || reason.is_none()
            {
                return Err(format!(
                    "changed row {} requires two different present values, a semantic finding and a reason",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::Lost => {
            if source != PropertyValueState::Present
                || target != PropertyValueState::Absent
                || semantic_finding.is_none()
                || reason.is_none()
            {
                return Err(format!(
                    "lost row {} requires a present source, absent target, semantic finding and reason",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::Added => {
            if source != PropertyValueState::Absent
                || target != PropertyValueState::Present
                || semantic_finding.is_none()
                || reason.is_none()
            {
                return Err(format!(
                    "added row {} requires an absent source, present target, semantic finding and reason",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::Unsupported => {
            if target != PropertyValueState::Unsupported || reason.is_none() {
                return Err(format!(
                    "unsupported row {} requires unsupported target state and reason",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::Unresolved => {
            if (source != PropertyValueState::Unknown
                && target != PropertyValueState::Unknown
                && !(source == PropertyValueState::Present && target == PropertyValueState::Present))
                || reason.is_none()
            {
                return Err(format!(
                    "unresolved row {} requires unknown/ambiguous values or an unverified present-value comparison and reason",
                    comparison.comparison_record_id
                ));
            }
        }
        PropertyDisposition::AbsentBoth => {
            if source != PropertyValueState::Absent
                || target != PropertyValueState::Absent
                || reason.is_some()
            {
                return Err(format!(
                    "absent_both row {} requires absent source/target and no loss reason",
                    comparison.comparison_record_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_value_state(
    side: &str,
    state: PropertyValueState,
    digest: Option<&str>,
) -> Result<(), String> {
    match (state, digest) {
        (PropertyValueState::Present, Some(value)) => validate_sha256(
            &format!("{side}_value_sha256"),
            value,
        ),
        (PropertyValueState::Present, None) => Err(format!("{side} present value requires SHA-256")),
        (PropertyValueState::Absent | PropertyValueState::Unknown | PropertyValueState::Unsupported, None) => Ok(()),
        (_, Some(_)) => Err(format!("{side} non-present value must not carry a value digest")),
    }
}

fn validate_identifier(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 256 {
        return Err(format!("{field} must be non-empty and at most 256 bytes"));
    }
    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{field} must be 64 lowercase hexadecimal characters"));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn label_completeness(value: RequirementCompleteness) -> &'static str {
    match value {
        RequirementCompleteness::Complete => "complete",
        RequirementCompleteness::Partial => "partial",
        RequirementCompleteness::Unknown => "unknown",
    }
}

fn label_importance(value: PropertyImportance) -> &'static str {
    match value {
        PropertyImportance::Critical => "critical",
        PropertyImportance::Important => "important",
        PropertyImportance::Advisory => "advisory",
    }
}

fn label_value_state(value: PropertyValueState) -> &'static str {
    match value {
        PropertyValueState::Present => "present",
        PropertyValueState::Absent => "absent",
        PropertyValueState::Unknown => "unknown",
        PropertyValueState::Unsupported => "unsupported",
    }
}

fn label_disposition(value: PropertyDisposition) -> &'static str {
    match value {
        PropertyDisposition::PreservedExact => "preserved_exact",
        PropertyDisposition::PreservedNormalized => "preserved_normalized",
        PropertyDisposition::Changed => "changed",
        PropertyDisposition::Lost => "lost",
        PropertyDisposition::Added => "added",
        PropertyDisposition::Unsupported => "unsupported",
        PropertyDisposition::Unresolved => "unresolved",
        PropertyDisposition::AbsentBoth => "absent_both",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!(
        "../../../research/scan-bim-critical-attribute-retention-001/fixtures/known_answers_v0_1.json"
    );

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("property retention fixture parses")
    }

    fn audit_value(value: &Value) -> Result<PropertyRetentionReport, String> {
        let bytes = serde_json::to_vec(value).expect("serialize fixture");
        audit_property_retention_bytes(&bytes)
    }

    #[test]
    fn accepts_exact_and_evidenced_normalized_property_preservation() {
        let report = audit_property_retention_bytes(FIXTURE.as_bytes())
            .expect("complete property inventory preserves required fields");
        assert_eq!(report.preservation_state, "preserved");
        assert_eq!(report.requirement_count, 3);
        assert_eq!(report.comparison_count, 3);
        assert_eq!(report.critical_required_count, 2);
        assert_eq!(report.source_present_required_count, 3);
        assert_eq!(report.preserved_required_count, 3);
        assert_eq!(report.critical_loss_count, 0);
        assert_eq!(report.required_property_retention_ratio, Some(1.0));
    }

    #[test]
    fn critical_property_loss_blocks_preservation() {
        let mut value = fixture_value();
        value["comparisons"][0]["source_state"] = serde_json::json!("present");
        value["comparisons"][0]["target_state"] = serde_json::json!("absent");
        value["comparisons"][0]["disposition"] = serde_json::json!("lost");
        value["comparisons"][0]["target_value_sha256"] = Value::Null;
        value["comparisons"][0]["semantic_finding_id"] = serde_json::json!("DIFF-LOSS-001");
        value["comparisons"][0]["reason"] = serde_json::json!("required FireRating was dropped");
        let report = audit_value(&value).expect("loss is a valid finding");
        assert_eq!(report.preservation_state, "blocked");
        assert_eq!(report.critical_loss_count, 1);
        assert_eq!(report.lost_property_count, 1);
    }

    #[test]
    fn critical_value_change_blocks_preservation() {
        let mut value = fixture_value();
        value["comparisons"][0]["target_value_sha256"] = serde_json::json!("f".repeat(64));
        value["comparisons"][0]["disposition"] = serde_json::json!("changed");
        value["comparisons"][0]["semantic_finding_id"] = serde_json::json!("DIFF-CHANGE-001");
        value["comparisons"][0]["reason"] = serde_json::json!("FireRating value changed");
        let report = audit_value(&value).expect("changed critical property is reported");
        assert_eq!(report.preservation_state, "blocked");
        assert_eq!(report.critical_loss_count, 1);
        assert_eq!(report.changed_property_count, 1);
    }

    #[test]
    fn critical_unsupported_property_is_unresolved_not_preserved() {
        let mut value = fixture_value();
        value["comparisons"][0]["target_state"] = serde_json::json!("unsupported");
        value["comparisons"][0]["target_value_sha256"] = Value::Null;
        value["comparisons"][0]["disposition"] = serde_json::json!("unsupported");
        value["comparisons"][0]["reason"] = serde_json::json!("target profile cannot encode FireRating");
        let report = audit_value(&value).expect("unsupported target is visible");
        assert_eq!(report.preservation_state, "unresolved");
        assert_eq!(report.critical_unresolved_count, 1);
        assert_eq!(report.unsupported_property_count, 1);
    }

    #[test]
    fn important_property_loss_degrades_without_overriding_critical_counts() {
        let mut value = fixture_value();
        value["comparisons"][2]["target_state"] = serde_json::json!("absent");
        value["comparisons"][2]["target_value_sha256"] = Value::Null;
        value["comparisons"][2]["disposition"] = serde_json::json!("lost");
        value["comparisons"][2]["semantic_finding_id"] = serde_json::json!("DIFF-LOSS-003");
        value["comparisons"][2]["reason"] = serde_json::json!("AcousticRating dropped");
        let report = audit_value(&value).expect("important property loss is reported");
        assert_eq!(report.preservation_state, "degraded");
        assert_eq!(report.important_degraded_count, 1);
        assert_eq!(report.critical_loss_count, 0);
    }

    #[test]
    fn critical_property_absent_from_both_sides_is_requirement_gap_not_retention() {
        let mut value = fixture_value();
        value["comparisons"][0]["source_state"] = serde_json::json!("absent");
        value["comparisons"][0]["target_state"] = serde_json::json!("absent");
        value["comparisons"][0]["source_value_sha256"] = Value::Null;
        value["comparisons"][0]["target_value_sha256"] = Value::Null;
        value["comparisons"][0]["disposition"] = serde_json::json!("absent_both");
        let report = audit_value(&value).expect("unmet requirement is distinct from transformation loss");
        assert_eq!(report.preservation_state, "blocked");
        assert_eq!(report.critical_requirement_gap_count, 1);
        assert_eq!(report.critical_loss_count, 0);
    }

    #[test]
    fn preserved_rows_reject_conflicting_semantic_findings() {
        let mut exact = fixture_value();
        exact["comparisons"][0]["semantic_finding_id"] =
            serde_json::json!("DIFF-CONTRADICTS-EXACT");
        let error = audit_value(&exact).expect_err("exact preservation cannot carry a semantic diff");
        assert!(error.contains("preserved_exact"));

        let mut normalized = fixture_value();
        normalized["comparisons"][1]["semantic_finding_id"] =
            serde_json::json!("DIFF-CONTRADICTS-NORMALIZED");
        let error = audit_value(&normalized)
            .expect_err("normalized equivalence cannot carry a semantic difference finding");
        assert!(error.contains("preserved_normalized"));
    }

    #[test]
    fn invalid_upstream_report_digest_is_rejected() {
        let mut value = fixture_value();
        value["scope"]["semantic_diff_report_sha256"] = serde_json::json!("not-a-digest");
        let error = audit_value(&value).expect_err("producer report identity must be digest-shaped");
        assert!(error.contains("semantic_diff_report_sha256"));
    }

    #[test]
    fn normalized_equivalence_requires_unit_comparison_receipt() {
        let mut value = fixture_value();
        value["scope"]["unit_comparison_report_sha256"] = Value::Null;
        let error = audit_value(&value).expect_err("normalized preservation needs unit evidence");
        assert!(error.contains("unit-comparison report digest"));
    }

    #[test]
    fn unit_normalized_comparison_requires_distinct_raw_values_and_finding() {
        let mut value = fixture_value();
        value["comparisons"][1]["unit_comparison_finding_id"] = Value::Null;
        let error = audit_value(&value).expect_err("normalized equivalence needs a finding");
        assert!(error.contains("unit-comparison finding"));
        let mut same_raw = fixture_value();
        same_raw["comparisons"][1]["target_value_sha256"] =
            same_raw["comparisons"][1]["source_value_sha256"].clone();
        let error = audit_value(&same_raw).expect_err("normalized disposition must not hide exact equality");
        assert!(error.contains("unequal present raw values"));
    }

    #[test]
    fn wrong_subject_identity_cannot_reuse_property_comparison() {
        let mut value = fixture_value();
        value["comparisons"][0]["target_entity_id"] = serde_json::json!("WALL-OTHER");
        let error = audit_value(&value).expect_err("comparison must target the declared entity");
        assert!(error.contains("entity binding"));
    }

    #[test]
    fn missing_and_duplicate_comparisons_are_rejected() {
        let mut missing = fixture_value();
        missing["comparisons"].as_array_mut().unwrap().pop();
        let error = audit_value(&missing).expect_err("comparison coverage must be complete");
        assert!(error.contains("exactly one comparison"));

        let mut duplicate = fixture_value();
        let row = duplicate["comparisons"][0].clone();
        duplicate["comparisons"].as_array_mut().unwrap().push(row);
        let error = audit_value(&duplicate).expect_err("duplicate comparison must fail");
        assert!(error.contains("exactly one comparison"));
    }

    #[test]
    fn duplicate_requirement_ids_are_rejected() {
        let mut value = fixture_value();
        value["requirements"][1]["requirement_id"] =
            value["requirements"][0]["requirement_id"].clone();
        let error = audit_value(&value).expect_err("requirement IDs must be unique");
        assert!(error.contains("duplicate property requirement ID"));
    }

    #[test]
    fn incomplete_requirement_profile_never_claims_complete_preservation() {
        let mut value = fixture_value();
        value["scope"]["requirement_completeness"] = serde_json::json!("partial");
        let report = audit_value(&value).expect("partial profile remains reportable");
        assert_eq!(report.preservation_state, "unresolved");
    }

    #[test]
    fn unknown_source_cannot_be_promoted_to_exact_preservation_by_default() {
        let mut value = fixture_value();
        value["comparisons"][0]["source_state"] = serde_json::json!("unknown");
        value["comparisons"][0]["disposition"] = serde_json::json!("preserved_exact");
        let error = audit_value(&value).expect_err("unknown is not zero or preserved");
        assert!(error.contains("present"));
    }

    #[test]
    fn advisory_loss_is_visible_and_degrades_report() {
        let mut value = fixture_value();
        value["comparisons"][2]["target_state"] = serde_json::json!("absent");
        value["comparisons"][2]["target_value_sha256"] = Value::Null;
        value["comparisons"][2]["disposition"] = serde_json::json!("lost");
        value["comparisons"][2]["semantic_finding_id"] = serde_json::json!("DIFF-ADVISORY-003");
        value["comparisons"][2]["reason"] = serde_json::json!("advisory field dropped");
        let report = audit_value(&value).expect("advisory loss remains visible");
        assert_eq!(report.preservation_state, "degraded");
    }

    #[test]
    fn project_criticality_is_not_inferred_from_property_name() {
        let mut value = fixture_value();
        value["requirements"][0]["property_name"] =
            serde_json::json!("ProjectSpecificAttributeA");
        value["comparisons"][0]["target_state"] = serde_json::json!("absent");
        value["comparisons"][0]["target_value_sha256"] = Value::Null;
        value["comparisons"][0]["disposition"] = serde_json::json!("lost");
        value["comparisons"][0]["semantic_finding_id"] = serde_json::json!("DIFF-LOSS-PROFILE-001");
        value["comparisons"][0]["reason"] = serde_json::json!("project-required attribute was dropped");
        let report = audit_value(&value).expect("project profile controls criticality");
        assert_eq!(report.preservation_state, "blocked");
        assert_eq!(report.critical_loss_count, 1);
        assert_eq!(report.findings[0].property_name, "ProjectSpecificAttributeA");
    }

    #[test]
    fn unknown_source_replaced_by_default_zero_remains_unresolved() {
        let mut value = fixture_value();
        value["comparisons"][0]["source_state"] = serde_json::json!("unknown");
        value["comparisons"][0]["source_value_sha256"] = Value::Null;
        value["comparisons"][0]["target_value_sha256"] = serde_json::json!("0".repeat(64));
        value["comparisons"][0]["disposition"] = serde_json::json!("unresolved");
        value["comparisons"][0]["reason"] =
            serde_json::json!("source value was unknown; target zero may be a default");
        let report = audit_value(&value).expect("unknown-to-default must remain unresolved");
        assert_eq!(report.preservation_state, "unresolved");
        assert_eq!(report.critical_unresolved_count, 1);
        assert_eq!(report.preserved_required_count, 2);
    }

    #[test]
    fn target_only_property_is_added_not_counted_as_source_retention() {
        let mut value = fixture_value();
        value["comparisons"][2]["source_state"] = serde_json::json!("absent");
        value["comparisons"][2]["source_value_sha256"] = Value::Null;
        value["comparisons"][2]["target_state"] = serde_json::json!("present");
        value["comparisons"][2]["target_value_sha256"] = serde_json::json!("7".repeat(64));
        value["comparisons"][2]["disposition"] = serde_json::json!("added");
        value["comparisons"][2]["semantic_finding_id"] = serde_json::json!("DIFF-ADDED-001");
        value["comparisons"][2]["reason"] = serde_json::json!("target adds an acoustic field");
        let report = audit_value(&value).expect("target-only attribute is reported as added");
        assert_eq!(report.added_property_count, 1);
        assert_eq!(report.source_present_required_count, 2);
        assert_eq!(report.preserved_required_count, 2);
        assert_eq!(report.required_property_retention_ratio, Some(1.0));
        assert_eq!(report.preservation_state, "preserved");
    }

    #[test]
    fn critical_importance_must_be_project_required() {
        let mut value = fixture_value();
        value["requirements"][0]["required"] = serde_json::json!(false);
        let error = audit_value(&value).expect_err("criticality and requiredness cannot conflict");
        assert!(error.contains("critical property requirement"));
    }

    #[test]
    fn comparison_order_does_not_change_semantic_report() {
        let original = audit_property_retention_bytes(FIXTURE.as_bytes())
            .expect("original receipt audits");
        let mut value = fixture_value();
        value["requirements"].as_array_mut().unwrap().reverse();
        value["comparisons"].as_array_mut().unwrap().reverse();
        let bytes = serde_json::to_vec(&value).expect("serialize permuted receipt");
        let permuted = audit_property_retention_bytes(&bytes).expect("permuted receipt audits");
        assert_eq!(original.findings.len(), permuted.findings.len());
        assert_eq!(original.findings[0].requirement_id, permuted.findings[0].requirement_id);
        assert_eq!(original.findings[1].requirement_id, permuted.findings[1].requirement_id);
        assert_eq!(original.findings[2].requirement_id, permuted.findings[2].requirement_id);
        assert_ne!(original.input_sha256, permuted.input_sha256);
        assert_eq!(original.preservation_state, permuted.preservation_state);
    }
}

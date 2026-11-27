// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed commitment to owner-native AeroCommons and BASALT evidence cuts.
//!
//! This module binds exact references/digests and owner-reported freshness
//! dispositions. It does not duplicate AeroCommons artifact storage, BASALT
//! invalidation, receiver receipt issuance, observer qualification, trust-root
//! management or cryptographic verification. Referenced owner bytes and
//! signatures must still be verified by their authoritative owners.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerEvidenceLifecycle {
    Active,
    Superseded,
    Disputed,
    Revoked,
    Withdrawn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerEvidenceFreshness {
    Fresh,
    StaleDirect,
    StaleTransitive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerDependencyCoverage {
    Complete,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerObserverCoverageStatus {
    QualifiedCompleteForScope,
    Incomplete,
    Unverified,
    OutOfScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceRecordV0_1 {
    /// Exact owner-native AeroEvidence ID.
    pub evidence_ref: String,
    /// Digest of the exact serialized owner evidence record.
    pub record_sha256: String,
    /// Physical unit or article identity from the owner record.
    pub subject_ref: String,
    /// Exact configuration identity from the owner record.
    pub configuration_ref: String,
    /// Current owner-declared lifecycle state.
    pub lifecycle: OwnerEvidenceLifecycle,
    /// Owner-declared predecessor for supersession.
    pub predecessor_ref: Option<String>,
    /// BASALT freshness assessment for this exact evidence record.
    pub freshness: OwnerEvidenceFreshness,
    /// BASALT's declared dependency coverage.
    pub dependency_coverage: OwnerDependencyCoverage,
    /// Exact unresolved inputs when dependency coverage is incomplete.
    pub unresolved_inputs: Vec<String>,
    /// Whether BASALT currently requires this record to be reverified.
    pub requires_reverification: bool,
    /// BASALT graph eligibility under the declared dependency coverage only.
    pub eligible_under_declared_coverage: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceCutV0_1 {
    pub schema: String,
    /// Mycelix/AeroCommons owner namespace.
    pub owner_namespace: String,
    /// Stable component build packet identity.
    pub component_packet_id: String,
    /// Physical unit identity bound by the component packet.
    pub physical_unit_ref: String,
    /// Exact configuration identity bound by the component packet.
    pub configuration_ref: String,
    /// Digest and byte length of the exact packet bytes.
    pub component_packet_sha256: String,
    pub component_packet_byte_length: u64,
    /// True only when the owner verifier checked every required artifact payload.
    /// This is consumed as an owner disposition; AERO does not redo the byte fetch.
    pub artifact_payloads_verified: bool,
    /// Exact owner AeroEvidence records relevant to the claim.
    pub evidence_records: Vec<OwnerEvidenceRecordV0_1>,
    /// BASALT audit report identity and exact inputs/outputs.
    pub basalt_run_id: String,
    pub basalt_report_version: u32,
    pub basalt_timeline_input_sha256: String,
    pub basalt_final_state_sha256: String,
    pub basalt_report_sha256: String,
    /// Preserved as an owner claim; false prevents confirmatory reviewability.
    pub basalt_confirmatory_eligible: bool,
    /// Owner evidence-receipt manifest and receiver inclusion cutoff.
    pub receiver_receipt_manifest_sha256: String,
    pub receiver_receipt_watermark_unix_ms: u64,
    /// Exact pinned owner trust-policy snapshot.
    pub trust_policy_sha256: String,
    /// Owner/BASALT observer coverage receipt and status.
    pub observer_coverage_profile_id: String,
    pub observer_coverage_receipt_sha256: String,
    pub observer_coverage_status: OwnerObserverCoverageStatus,
    pub observer_coverage_watermark_unix_ms: u64,
}

#[derive(Serialize)]
struct OwnerEvidenceCutEnvelope<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    owner_namespace: &'a str,
    component_packet_id: &'a str,
    physical_unit_ref: &'a str,
    configuration_ref: &'a str,
    component_packet_sha256: &'a str,
    component_packet_byte_length: u64,
    artifact_payloads_verified: bool,
    evidence_records: Vec<&'a OwnerEvidenceRecordV0_1>,
    basalt_run_id: &'a str,
    basalt_report_version: u32,
    basalt_timeline_input_sha256: &'a str,
    basalt_final_state_sha256: &'a str,
    basalt_report_sha256: &'a str,
    basalt_confirmatory_eligible: bool,
    receiver_receipt_manifest_sha256: &'a str,
    receiver_receipt_watermark_unix_ms: u64,
    trust_policy_sha256: &'a str,
    observer_coverage_profile_id: &'a str,
    observer_coverage_receipt_sha256: &'a str,
    observer_coverage_status: OwnerObserverCoverageStatus,
    observer_coverage_watermark_unix_ms: u64,
}

impl OwnerEvidenceCutV0_1 {
    /// Validate identifiers, digests and local owner-report consistency.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.schema != "AeroOwnerEvidenceCutV0_1" {
            errors.push("OWNER_CUT_SCHEMA_INVALID".to_owned());
        }
        for (field, value) in [
            ("OWNER_NAMESPACE", self.owner_namespace.as_str()),
            ("COMPONENT_PACKET_ID", self.component_packet_id.as_str()),
            ("PHYSICAL_UNIT", self.physical_unit_ref.as_str()),
            ("CONFIGURATION", self.configuration_ref.as_str()),
            ("BASALT_RUN_ID", self.basalt_run_id.as_str()),
            ("OBSERVER_COVERAGE_PROFILE", self.observer_coverage_profile_id.as_str()),
        ] {
            if !valid_identifier(value) {
                errors.push(format!("OWNER_CUT_{field}_INVALID"));
            }
        }
        if self.component_packet_byte_length == 0 {
            errors.push("OWNER_CUT_COMPONENT_PACKET_LENGTH_INVALID".to_owned());
        }
        for (field, value) in [
            ("COMPONENT_PACKET", self.component_packet_sha256.as_str()),
            ("BASALT_INPUT", self.basalt_timeline_input_sha256.as_str()),
            ("BASALT_FINAL_STATE", self.basalt_final_state_sha256.as_str()),
            ("BASALT_REPORT", self.basalt_report_sha256.as_str()),
            ("RECEIPT_MANIFEST", self.receiver_receipt_manifest_sha256.as_str()),
            ("TRUST_POLICY", self.trust_policy_sha256.as_str()),
            ("OBSERVER_RECEIPT", self.observer_coverage_receipt_sha256.as_str()),
        ] {
            if !valid_sha256(value) {
                errors.push(format!("OWNER_CUT_{field}_DIGEST_INVALID"));
            }
        }
        if self.basalt_report_version == 0 {
            errors.push("OWNER_CUT_BASALT_REPORT_VERSION_INVALID".to_owned());
        }
        if self.observer_coverage_watermark_unix_ms
            > self.receiver_receipt_watermark_unix_ms
        {
            errors.push("OWNER_CUT_OBSERVER_WATERMARK_AFTER_RECEIPT_CUT".to_owned());
        }
        let mut seen = BTreeSet::new();
        for record in &self.evidence_records {
            if !valid_identifier(&record.evidence_ref)
                || !valid_identifier(&record.subject_ref)
                || !valid_identifier(&record.configuration_ref)
            {
                errors.push("OWNER_CUT_EVIDENCE_IDENTITY_INVALID".to_owned());
            }
            if !valid_sha256(&record.record_sha256) {
                errors.push(format!(
                    "OWNER_CUT_EVIDENCE_RECORD_DIGEST_INVALID:{}",
                    record.evidence_ref
                ));
            }
            if !seen.insert(record.evidence_ref.as_str()) {
                errors.push(format!(
                    "OWNER_CUT_DUPLICATE_EVIDENCE_REF:{}",
                    record.evidence_ref
                ));
            }
            match (record.lifecycle, record.predecessor_ref.as_deref()) {
                (OwnerEvidenceLifecycle::Superseded, None) => errors.push(format!(
                    "OWNER_CUT_SUPERSESSION_PREDECESSOR_MISSING:{}",
                    record.evidence_ref
                )),
                (_, Some(predecessor)) if !valid_identifier(predecessor) => errors.push(format!(
                    "OWNER_CUT_PREDECESSOR_INVALID:{}",
                    record.evidence_ref
                )),
                _ => {}
            }
            if record.unresolved_inputs.iter().any(|value| !valid_identifier(value)) {
                errors.push(format!(
                    "OWNER_CUT_UNRESOLVED_INPUT_ID_INVALID:{}",
                    record.evidence_ref
                ));
            }
            match record.dependency_coverage {
                OwnerDependencyCoverage::Complete if !record.unresolved_inputs.is_empty() => {
                    errors.push(format!(
                        "OWNER_CUT_COMPLETE_COVERAGE_HAS_UNRESOLVED_INPUTS:{}",
                        record.evidence_ref
                    ));
                }
                OwnerDependencyCoverage::Incomplete if record.unresolved_inputs.is_empty() => {
                    errors.push(format!(
                        "OWNER_CUT_INCOMPLETE_COVERAGE_HAS_NO_UNRESOLVED_INPUTS:{}",
                        record.evidence_ref
                    ));
                }
                _ => {}
            }
        }
        errors.sort();
        errors
    }

    /// Hash the versioned owner evidence cut with evidence records sorted by exact owner ID.
    ///
    /// This commits to owner-declared references/dispositions. It does not retrieve
    /// or authenticate packet bytes, BASALT runner receipts, signatures or source clocks.
    pub fn compute_sha256(&self) -> Result<String, Vec<String>> {
        let errors = self.validation_errors();
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut records = self.evidence_records.iter().collect::<Vec<_>>();
        records.sort_by(|left, right| left.evidence_ref.cmp(&right.evidence_ref));
        let envelope = OwnerEvidenceCutEnvelope {
            schema: "AeroOwnerEvidenceCutV0_1",
            canonicalization_profile: "rust-serde-json-struct-order-v1",
            owner_namespace: &self.owner_namespace,
            component_packet_id: &self.component_packet_id,
            physical_unit_ref: &self.physical_unit_ref,
            configuration_ref: &self.configuration_ref,
            component_packet_sha256: &self.component_packet_sha256,
            component_packet_byte_length: self.component_packet_byte_length,
            artifact_payloads_verified: self.artifact_payloads_verified,
            evidence_records: records,
            basalt_run_id: &self.basalt_run_id,
            basalt_report_version: self.basalt_report_version,
            basalt_timeline_input_sha256: &self.basalt_timeline_input_sha256,
            basalt_final_state_sha256: &self.basalt_final_state_sha256,
            basalt_report_sha256: &self.basalt_report_sha256,
            basalt_confirmatory_eligible: self.basalt_confirmatory_eligible,
            receiver_receipt_manifest_sha256: &self.receiver_receipt_manifest_sha256,
            receiver_receipt_watermark_unix_ms: self.receiver_receipt_watermark_unix_ms,
            trust_policy_sha256: &self.trust_policy_sha256,
            observer_coverage_profile_id: &self.observer_coverage_profile_id,
            observer_coverage_receipt_sha256: &self.observer_coverage_receipt_sha256,
            observer_coverage_status: self.observer_coverage_status,
            observer_coverage_watermark_unix_ms: self.observer_coverage_watermark_unix_ms,
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| vec!["OWNER_CUT_SERIALIZATION_FAILED".to_owned()])?;
        Ok(sha256_hex(&bytes))
    }

    /// Owner-cut prerequisites that prevent a positive claim review, separate from malformed input.
    pub fn reviewability_blockers(&self) -> Vec<String> {
        let mut blockers = BTreeSet::new();
        if !self.artifact_payloads_verified {
            blockers.insert("MYCELIX_ARTIFACT_PAYLOADS_NOT_VERIFIED".to_owned());
        }
        if !self.basalt_confirmatory_eligible {
            blockers.insert("BASALT_CONFIRMATORY_ELIGIBILITY_NOT_ESTABLISHED".to_owned());
        }
        match self.observer_coverage_status {
            OwnerObserverCoverageStatus::QualifiedCompleteForScope => {}
            OwnerObserverCoverageStatus::Incomplete => {
                blockers.insert("BASALT_OBSERVER_COVERAGE_INCOMPLETE".to_owned());
            }
            OwnerObserverCoverageStatus::Unverified => {
                blockers.insert("BASALT_OBSERVER_COVERAGE_UNVERIFIED".to_owned());
            }
            OwnerObserverCoverageStatus::OutOfScope => {
                blockers.insert("BASALT_OBSERVER_COVERAGE_OUT_OF_SCOPE".to_owned());
            }
        }
        for record in &self.evidence_records {
            if record.lifecycle != OwnerEvidenceLifecycle::Active {
                blockers.insert(format!(
                    "AEROCOMMONS_EVIDENCE_LIFECYCLE_NOT_ACTIVE:{}",
                    record.evidence_ref
                ));
            }
            match record.freshness {
                OwnerEvidenceFreshness::Fresh => {}
                OwnerEvidenceFreshness::StaleDirect => {
                    blockers.insert(format!(
                        "BASALT_EVIDENCE_STALE_DIRECT:{}",
                        record.evidence_ref
                    ));
                }
                OwnerEvidenceFreshness::StaleTransitive => {
                    blockers.insert(format!(
                        "BASALT_EVIDENCE_STALE_TRANSITIVE:{}",
                        record.evidence_ref
                    ));
                }
            }
            if record.dependency_coverage == OwnerDependencyCoverage::Incomplete {
                blockers.insert(format!(
                    "BASALT_DEPENDENCY_COVERAGE_INCOMPLETE:{}",
                    record.evidence_ref
                ));
            }
            if record.requires_reverification {
                blockers.insert(format!(
                    "BASALT_EVIDENCE_REVERIFICATION_REQUIRED:{}",
                    record.evidence_ref
                ));
            }
            if !record.eligible_under_declared_coverage {
                blockers.insert(format!(
                    "BASALT_EVIDENCE_NOT_ELIGIBLE_UNDER_DECLARED_COVERAGE:{}",
                    record.evidence_ref
                ));
            }
        }
        blockers.into_iter().collect()
    }

    pub fn find_evidence(&self, evidence_ref: &str) -> Option<&OwnerEvidenceRecordV0_1> {
        self.evidence_records
            .iter()
            .find(|record| record.evidence_ref == evidence_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OwnerDependencyCoverage, OwnerEvidenceCutV0_1, OwnerEvidenceFreshness,
        OwnerEvidenceLifecycle, OwnerEvidenceRecordV0_1, OwnerObserverCoverageStatus,
    };

    fn record(evidence_ref: &str) -> OwnerEvidenceRecordV0_1 {
        OwnerEvidenceRecordV0_1 {
            evidence_ref: evidence_ref.to_owned(),
            record_sha256: "1".repeat(64),
            subject_ref: "part:wing-panel-001".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            lifecycle: OwnerEvidenceLifecycle::Active,
            predecessor_ref: None,
            freshness: OwnerEvidenceFreshness::Fresh,
            dependency_coverage: OwnerDependencyCoverage::Complete,
            unresolved_inputs: vec![],
            requires_reverification: false,
            eligible_under_declared_coverage: true,
        }
    }

    pub fn fixture() -> OwnerEvidenceCutV0_1 {
        OwnerEvidenceCutV0_1 {
            schema: "AeroOwnerEvidenceCutV0_1".to_owned(),
            owner_namespace: "mycelix:aerocommons".to_owned(),
            component_packet_id: "packet:coupon-001-r1".to_owned(),
            physical_unit_ref: "part:wing-panel-001".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            component_packet_sha256: "2".repeat(64),
            component_packet_byte_length: 4096,
            artifact_payloads_verified: true,
            evidence_records: vec![
                record("receipt:inspection-01"),
                record("receipt:calibration-02"),
            ],
            basalt_run_id: "basalt:run-001".to_owned(),
            basalt_report_version: 1,
            basalt_timeline_input_sha256: "3".repeat(64),
            basalt_final_state_sha256: "4".repeat(64),
            basalt_report_sha256: "5".repeat(64),
            basalt_confirmatory_eligible: true,
            receiver_receipt_manifest_sha256: "6".repeat(64),
            receiver_receipt_watermark_unix_ms: 1_795_516_700_000,
            trust_policy_sha256: "c".repeat(64),
            observer_coverage_profile_id: "basalt-coverage-v0.1".to_owned(),
            observer_coverage_receipt_sha256: "7".repeat(64),
            observer_coverage_status: OwnerObserverCoverageStatus::QualifiedCompleteForScope,
            observer_coverage_watermark_unix_ms: 1_795_516_600_000,
        }
    }

    #[test]
    fn deterministic_hash_is_order_independent() {
        let cut = fixture();
        let mut reversed = cut.clone();
        reversed.evidence_records.reverse();
        assert_eq!(cut.compute_sha256(), reversed.compute_sha256());
    }

    #[test]
    fn every_owner_digest_or_packet_byte_check_changes_the_commitment() {
        let baseline = fixture().compute_sha256().expect("valid owner cut");
        let mut cut = fixture();
        cut.basalt_final_state_sha256 = "8".repeat(64);
        assert_ne!(baseline, cut.compute_sha256().expect("valid changed cut"));
        let mut cut = fixture();
        cut.component_packet_byte_length += 1;
        assert_ne!(baseline, cut.compute_sha256().expect("valid changed cut"));
        let mut cut = fixture();
        cut.artifact_payloads_verified = false;
        assert_ne!(baseline, cut.compute_sha256().expect("valid changed cut"));
    }

    #[test]
    fn incomplete_or_stale_basaltevidence_is_a_reviewability_blocker() {
        let mut cut = fixture();
        cut.basalt_confirmatory_eligible = false;
        cut.evidence_records[0].freshness = OwnerEvidenceFreshness::StaleTransitive;
        cut.evidence_records[0].dependency_coverage = OwnerDependencyCoverage::Incomplete;
        cut.evidence_records[0].unresolved_inputs = vec!["toolchain:rust".to_owned()];
        let blockers = cut.reviewability_blockers();
        assert!(blockers.contains(&"BASALT_CONFIRMATORY_ELIGIBILITY_NOT_ESTABLISHED".to_owned()));
        assert!(blockers.contains(&"BASALT_EVIDENCE_STALE_TRANSITIVE:receipt:inspection-01".to_owned()));
        assert!(blockers.contains(&"BASALT_DEPENDENCY_COVERAGE_INCOMPLETE:receipt:inspection-01".to_owned()));
    }

    #[test]
    fn non_active_aerocommons_lifecycle_is_not_promoted_to_active() {
        let mut cut = fixture();
        cut.evidence_records[0].lifecycle = OwnerEvidenceLifecycle::Superseded;
        cut.evidence_records[0].predecessor_ref = Some("receipt:inspection-00".to_owned());
        assert!(cut.reviewability_blockers().contains(
            &"AEROCOMMONS_EVIDENCE_LIFECYCLE_NOT_ACTIVE:receipt:inspection-01".to_owned()
        ));
    }

    #[test]
    fn malformed_or_duplicate_owner_references_reject_cut() {
        let mut malformed = fixture();
        malformed.component_packet_sha256 = "NOT-A-SHA256".to_owned();
        assert!(malformed.compute_sha256().is_err());

        let mut duplicate = fixture();
        duplicate.evidence_records.push(record("receipt:inspection-01"));
        assert!(duplicate.compute_sha256().is_err());
    }
}

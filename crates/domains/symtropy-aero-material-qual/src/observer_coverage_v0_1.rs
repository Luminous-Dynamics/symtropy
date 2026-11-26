// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only observer-coverage contract evaluator for AERO v0.1.
//!
//! This module classifies synthetic coverage facts. It does not discover the
//! source universe, observe physical systems, verify signatures or receipts,
//! qualify an observer, or establish physical completeness. Real qualification
//! remains owned by BASALT and the relevant source owners.

use serde_json::Value;
use std::collections::BTreeSet;

const CORPUS: &str = include_str!(
    "../../../../docs/research/aviation/examples/aero_observer_coverage_corpus_v0_1.json"
);

const REQUIRED_BOOLEAN_FIELDS: &[&str] = &[
    "scope_closed",
    "requested_claim_in_scope",
    "scope_manifest_digest_valid",
    "checkpoint_digest_valid",
    "receipt_present",
    "receipt_digest_valid",
    "independently_qualified",
    "source_identity_binding_valid",
    "stream_epoch_binding_verified",
    "source_roster_reconciled",
    "required_obligations_complete",
    "critical_channels_complete",
    "time_interval_complete",
    "stream_continuity_valid",
    "retention_available",
    "checkpoint_binding_valid",
];

const INTEGRITY_FIELDS: &[&str] = &[
    "scope_manifest_digest_valid",
    "checkpoint_digest_valid",
    "receipt_digest_valid",
    "checkpoint_binding_valid",
];

const UNVERIFIED_FIELDS: &[&str] = &[
    "scope_closed",
    "receipt_present",
    "independently_qualified",
    "source_identity_binding_valid",
    "stream_epoch_binding_verified",
];

const INCOMPLETE_FIELDS: &[&str] = &[
    "source_roster_reconciled",
    "required_obligations_complete",
    "critical_channels_complete",
    "time_interval_complete",
    "stream_continuity_valid",
    "retention_available",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageStatus {
    QualifiedCompleteForScope,
    Incomplete,
    Unverified,
    Invalid,
    OutOfScope,
}

impl CoverageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QualifiedCompleteForScope => "QUALIFIED_COMPLETE_FOR_SCOPE",
            Self::Incomplete => "INCOMPLETE",
            Self::Unverified => "UNVERIFIED",
            Self::Invalid => "INVALID",
            Self::OutOfScope => "OUT_OF_SCOPE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayConsequence {
    AllowScopedAbsenceInference,
    Abstain,
    TrustUnresolved,
    Reject,
}

impl ReplayConsequence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllowScopedAbsenceInference => "ALLOW_SCOPED_ABSENCE_INFERENCE",
            Self::Abstain => "ABSTAIN",
            Self::TrustUnresolved => "TRUST_UNRESOLVED",
            Self::Reject => "REJECT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageAssessment {
    pub status: CoverageStatus,
    pub replay_consequence: ReplayConsequence,
    pub reason_codes: Vec<String>,
}

fn bool_field(fixture: &Value, key: &str) -> Option<bool> {
    fixture.get(key).and_then(Value::as_bool)
}

fn consequence(status: CoverageStatus) -> ReplayConsequence {
    match status {
        CoverageStatus::QualifiedCompleteForScope => ReplayConsequence::AllowScopedAbsenceInference,
        CoverageStatus::Incomplete | CoverageStatus::OutOfScope => ReplayConsequence::Abstain,
        CoverageStatus::Unverified => ReplayConsequence::TrustUnresolved,
        CoverageStatus::Invalid => ReplayConsequence::Reject,
    }
}

fn reason_for_field(key: &str) -> &'static str {
    match key {
        "scope_closed" => "OBSERVATION_BOUNDARY_NOT_CLOSED",
        "requested_claim_in_scope" => "CLAIM_OUTSIDE_COVERAGE_SCOPE",
        "scope_manifest_digest_valid" => "SCOPE_MANIFEST_INTEGRITY_INVALID",
        "checkpoint_digest_valid" => "CHECKPOINT_INTEGRITY_INVALID",
        "receipt_present" => "COVERAGE_RECEIPT_MISSING",
        "receipt_digest_valid" => "COVERAGE_RECEIPT_INTEGRITY_INVALID",
        "independently_qualified" => "OBSERVER_NOT_INDEPENDENTLY_QUALIFIED",
        "source_identity_binding_valid" => "SOURCE_IDENTITY_BINDING_UNVERIFIED",
        "stream_epoch_binding_verified" => "STREAM_EPOCH_TRANSITION_UNVERIFIED",
        "source_roster_reconciled" => "SOURCE_ROSTER_NOT_RECONCILED",
        "required_obligations_complete" => "REQUIRED_CAPTURE_OBLIGATION_MISSING",
        "critical_channels_complete" => "CRITICAL_CHANNEL_INCOMPLETE",
        "time_interval_complete" => "ACQUISITION_INTERVAL_INCOMPLETE",
        "stream_continuity_valid" => "SOURCE_STREAM_DISCONTINUITY",
        "retention_available" => "SOURCE_BYTES_NOT_RETAINED",
        "checkpoint_binding_valid" => "CHECKPOINT_SCOPE_RECEIPT_BINDING_MISMATCH",
        _ => "COVERAGE_FACT_UNRESOLVED",
    }
}

fn assessment(status: CoverageStatus, reasons: impl IntoIterator<Item = String>) -> CoverageAssessment {
    CoverageAssessment {
        status,
        replay_consequence: consequence(status),
        reason_codes: reasons
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}

/// Classify a coverage fixture using non-compensatory proof obligations.
///
/// Known integrity failures reject. Missing facts and unqualified identity or
/// observer assertions remain unresolved. A known capture omission abstains.
/// Only a closed, in-scope, independently qualified, fully reconciled fixture
/// can permit absence inference, and only within its declared scope.
pub fn evaluate_coverage_fixture(fixture: &Value) -> CoverageAssessment {
    let invalid = INTEGRITY_FIELDS
        .iter()
        .copied()
        .filter(|key| bool_field(fixture, key) == Some(false))
        .map(|key| reason_for_field(key).to_owned())
        .collect::<Vec<_>>();
    if !invalid.is_empty() {
        return assessment(CoverageStatus::Invalid, invalid);
    }

    let missing = REQUIRED_BOOLEAN_FIELDS
        .iter()
        .copied()
        .filter(|key| bool_field(fixture, key).is_none())
        .map(|key| format!("MISSING_{key}"))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return assessment(CoverageStatus::Unverified, missing);
    }

    if bool_field(fixture, "requested_claim_in_scope") == Some(false) {
        return assessment(
            CoverageStatus::OutOfScope,
            [reason_for_field("requested_claim_in_scope").to_owned()],
        );
    }

    let unresolved = UNVERIFIED_FIELDS
        .iter()
        .copied()
        .filter(|key| bool_field(fixture, key) == Some(false))
        .map(|key| reason_for_field(key).to_owned())
        .collect::<Vec<_>>();
    if !unresolved.is_empty() {
        return assessment(CoverageStatus::Unverified, unresolved);
    }

    let incomplete = INCOMPLETE_FIELDS
        .iter()
        .copied()
        .filter(|key| bool_field(fixture, key) == Some(false))
        .map(|key| reason_for_field(key).to_owned())
        .collect::<Vec<_>>();
    if !incomplete.is_empty() {
        return assessment(CoverageStatus::Incomplete, incomplete);
    }

    assessment(CoverageStatus::QualifiedCompleteForScope, [])
}

#[cfg(test)]
mod tests {
    use super::{CORPUS, CoverageStatus, ReplayConsequence, evaluate_coverage_fixture};
    use serde_json::Value;

    fn corpus() -> Value {
        serde_json::from_str(CORPUS).expect("observer coverage corpus parses")
    }

    fn case_fixture(case: &Value) -> &Value {
        case.get("fixture").expect("coverage fixture")
    }

    #[test]
    fn frozen_observer_coverage_corpus_matches_rust_reference_evaluator() {
        let corpus = corpus();
        assert_eq!(
            corpus.get("oracle_review_status").and_then(Value::as_str),
            Some("NOT_INDEPENDENTLY_REVIEWED")
        );
        let cases = corpus
            .get("cases")
            .and_then(Value::as_array)
            .expect("coverage challenge cases");
        assert_eq!(cases.len(), 24);

        for case in cases {
            let case_id = case
                .get("case_id")
                .and_then(Value::as_str)
                .expect("case ID");
            let actual = evaluate_coverage_fixture(case_fixture(case));
            assert_eq!(
                actual.status.as_str(),
                case.get("coverage_status")
                    .and_then(Value::as_str)
                    .expect("expected coverage status"),
                "{case_id}: coverage status"
            );
            assert_eq!(
                actual.replay_consequence.as_str(),
                case.get("replay_consequence")
                    .and_then(Value::as_str)
                    .expect("expected replay consequence"),
                "{case_id}: replay consequence"
            );
        }
    }

    #[test]
    fn missing_coverage_fact_never_promotes_to_complete() {
        let mut fixture = case_fixture(&corpus()["cases"][0]).to_owned();
        assert!(fixture
            .as_object_mut()
            .expect("fixture object")
            .remove("source_roster_reconciled")
            .is_some());
        let actual = evaluate_coverage_fixture(&fixture);
        assert_eq!(actual.status, CoverageStatus::Unverified);
        assert_eq!(actual.replay_consequence, ReplayConsequence::TrustUnresolved);
    }

    #[test]
    fn critical_channel_failure_is_non_compensatory() {
        let mut fixture = case_fixture(&corpus()["cases"][0]).to_owned();
        fixture["critical_channels_complete"] = Value::Bool(false);
        let actual = evaluate_coverage_fixture(&fixture);
        assert_eq!(actual.status, CoverageStatus::Incomplete);
        assert_eq!(actual.replay_consequence, ReplayConsequence::Abstain);
    }

    #[test]
    fn invalid_checkpoint_binding_rejects_even_when_other_facts_are_missing() {
        let mut fixture = case_fixture(&corpus()["cases"][0]).to_owned();
        fixture["checkpoint_binding_valid"] = Value::Bool(false);
        assert!(fixture
            .as_object_mut()
            .expect("fixture object")
            .remove("scope_closed")
            .is_some());
        let actual = evaluate_coverage_fixture(&fixture);
        assert_eq!(actual.status, CoverageStatus::Invalid);
        assert_eq!(actual.replay_consequence, ReplayConsequence::Reject);
    }

    #[test]
    fn open_world_scope_cannot_support_negative_inference() {
        let mut fixture = case_fixture(&corpus()["cases"][0]).to_owned();
        fixture["scope_closed"] = Value::Bool(false);
        let actual = evaluate_coverage_fixture(&fixture);
        assert_eq!(actual.status, CoverageStatus::Unverified);
    }

    #[test]
    fn out_of_scope_claim_abstains_without_marking_the_observer_invalid() {
        let mut fixture = case_fixture(&corpus()["cases"][0]).to_owned();
        fixture["requested_claim_in_scope"] = Value::Bool(false);
        let actual = evaluate_coverage_fixture(&fixture);
        assert_eq!(actual.status, CoverageStatus::OutOfScope);
        assert_eq!(actual.replay_consequence, ReplayConsequence::Abstain);
    }
}

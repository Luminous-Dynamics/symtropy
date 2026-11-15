// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence admission for measurements that are present but may be stale,
//! misconfigured, dependent, reconstructed or integrity-altered.
//! Latent causes are intentionally absent from candidate-facing types.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    PrimaryTelemetry,
    IndependentPhysicalMeasurement,
    InstrumentSelfTest,
    ForensicIntegrityReview,
    DerivedReconstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    Measured,
    Derived,
    Reconstructed,
    Attested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceResult {
    Numeric(i64),
    SelfTestPass,
    SelfTestFail,
    IntegrityIndicator,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyProfile {
    /// Identity of the underlying physical observation, before broker/format duplication.
    pub observation_root_id: Option<String>,
    /// Upstream dataset or alert record from which this evidence was acquired.
    pub upstream_dataset_id: Option<String>,
    /// Processing/model/mapping lineage that produced this evidence record.
    pub processing_lineage_id: Option<String>,
    /// Organization or administrative domain controlling the evidence source.
    pub operator_domain_id: Option<String>,
    /// Sensor/instrument lineage, distinct from access gateway identity.
    pub instrument_lineage_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyRelation {
    IndependentForThisProfile,
    Correlated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub class: EvidenceClass,
    pub result: EvidenceResult,
    pub unit: String,
    pub event_tick: u64,
    pub ingestion_tick: u64,
    pub source_id: String,
    pub gateway_id: String,
    pub trust_root_id: String,
    /// Explicit common-cause lineage; missing dimensions remain unknown.
    pub dependencies: DependencyProfile,
    pub calibration_revision: String,
    pub configuration_revision: String,
    pub authenticated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPolicy {
    pub scenario_id: String,
    pub source_digest: String,
    pub expected_unit: String,
    pub expected_calibration_revision: String,
    pub expected_configuration_revision: String,
    pub decision_cutoff_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub maximum_transport_lag_ticks: u64,
    pub numeric_tolerance: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDisposition {
    UseCorroboratedMeasurement,
    AbstainObservationalEquivalence,
    RequestIndependentPhysicalMeasurement,
    IntegrityInvestigation,
    EvidenceConflict,
    RejectEvidence,
}

impl CandidateDisposition {
    pub const fn id(self) -> &'static str {
        match self {
            Self::UseCorroboratedMeasurement => "use_corroborated_measurement",
            Self::AbstainObservationalEquivalence => "abstain_observational_equivalence",
            Self::RequestIndependentPhysicalMeasurement => {
                "request_independent_physical_measurement"
            }
            Self::IntegrityInvestigation => "integrity_investigation",
            Self::EvidenceConflict => "evidence_conflict",
            Self::RejectEvidence => "reject_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateDecision {
    pub scenario_id: String,
    pub disposition: CandidateDisposition,
    pub primary_evidence_id: String,
    pub primary_value: Option<i64>,
    pub admitted_evidence_ids: Vec<String>,
    pub rejected_evidence_ids: Vec<String>,
    pub unresolved_hypotheses: Vec<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityAuditError {
    InvalidPolicy,
    DuplicateEvidenceId,
    MissingPrimaryTelemetry,
    InvalidEvidenceIdentity,
    InvalidEvidenceShape,
    InvalidClockOrder,
}

fn valid_policy(p: &AdmissionPolicy) -> bool {
    !p.scenario_id.trim().is_empty()
        && !p.source_digest.trim().is_empty()
        && !p.expected_unit.trim().is_empty()
        && !p.expected_calibration_revision.trim().is_empty()
        && !p.expected_configuration_revision.trim().is_empty()
}

fn validate_record(record: &EvidenceRecord) -> Result<(), IntegrityAuditError> {
    if record.evidence_id.trim().is_empty()
        || record.unit.trim().is_empty()
        || record.source_id.trim().is_empty()
        || record.gateway_id.trim().is_empty()
        || record.trust_root_id.trim().is_empty()
        || record
            .dependencies
            .observation_root_id
            .as_ref()
            .is_some_and(|v| v.trim().is_empty())
        || record
            .dependencies
            .upstream_dataset_id
            .as_ref()
            .is_some_and(|v| v.trim().is_empty())
        || record
            .dependencies
            .processing_lineage_id
            .as_ref()
            .is_some_and(|v| v.trim().is_empty())
        || record
            .dependencies
            .operator_domain_id
            .as_ref()
            .is_some_and(|v| v.trim().is_empty())
        || record
            .dependencies
            .instrument_lineage_id
            .as_ref()
            .is_some_and(|v| v.trim().is_empty())
        || record.calibration_revision.trim().is_empty()
        || record.configuration_revision.trim().is_empty()
    {
        return Err(IntegrityAuditError::InvalidEvidenceIdentity);
    }
    if record.ingestion_tick < record.event_tick {
        return Err(IntegrityAuditError::InvalidClockOrder);
    }
    let shape_ok = matches!(
        (record.kind, record.class, record.result),
        (
            EvidenceKind::PrimaryTelemetry,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(_)
        ) | (
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(_)
        ) | (
            EvidenceKind::DerivedReconstruction,
            EvidenceClass::Reconstructed,
            EvidenceResult::Numeric(_)
        ) | (
            EvidenceKind::InstrumentSelfTest,
            EvidenceClass::Attested,
            EvidenceResult::SelfTestPass
        ) | (
            EvidenceKind::InstrumentSelfTest,
            EvidenceClass::Attested,
            EvidenceResult::SelfTestFail
        ) | (
            EvidenceKind::ForensicIntegrityReview,
            EvidenceClass::Attested,
            EvidenceResult::IntegrityIndicator
        )
    );
    if !shape_ok {
        return Err(IntegrityAuditError::InvalidEvidenceShape);
    }
    Ok(())
}

fn freshness_failure(record: &EvidenceRecord, policy: &AdmissionPolicy) -> Option<&'static str> {
    if record.event_tick > policy.decision_cutoff_tick {
        return Some("event_after_decision_cutoff");
    }
    if record.ingestion_tick > policy.decision_cutoff_tick {
        return Some("arrived_after_decision_cutoff");
    }
    if policy
        .decision_cutoff_tick
        .saturating_sub(record.event_tick)
        > policy.maximum_observation_age_ticks
    {
        return Some("observation_stale_at_decision_cutoff");
    }
    if record.ingestion_tick.saturating_sub(record.event_tick) > policy.maximum_transport_lag_ticks
    {
        return Some("event_to_ingestion_lag_exceeded");
    }
    None
}

fn contract_matches(record: &EvidenceRecord, policy: &AdmissionPolicy) -> bool {
    record.unit == policy.expected_unit
        && record.calibration_revision == policy.expected_calibration_revision
        && record.configuration_revision == policy.expected_configuration_revision
}

fn dependency_relation(primary: &EvidenceRecord, side: &EvidenceRecord) -> DependencyRelation {
    // A known shared dependency is stronger evidence than an unknown dimension.
    if primary.source_id == side.source_id
        || primary.gateway_id == side.gateway_id
        || primary.trust_root_id == side.trust_root_id
    {
        return DependencyRelation::Correlated;
    }

    let mut unknown = false;
    let dimensions = [
        (
            primary.dependencies.observation_root_id.as_deref(),
            side.dependencies.observation_root_id.as_deref(),
        ),
        (
            primary.dependencies.upstream_dataset_id.as_deref(),
            side.dependencies.upstream_dataset_id.as_deref(),
        ),
        (
            primary.dependencies.processing_lineage_id.as_deref(),
            side.dependencies.processing_lineage_id.as_deref(),
        ),
        (
            primary.dependencies.operator_domain_id.as_deref(),
            side.dependencies.operator_domain_id.as_deref(),
        ),
        (
            primary.dependencies.instrument_lineage_id.as_deref(),
            side.dependencies.instrument_lineage_id.as_deref(),
        ),
    ];

    for (left, right) in dimensions {
        match (left, right) {
            (Some(a), Some(b)) if a == b => return DependencyRelation::Correlated,
            (Some(_), Some(_)) => {}
            _ => unknown = true,
        }
    }

    if unknown {
        DependencyRelation::Unknown
    } else {
        DependencyRelation::IndependentForThisProfile
    }
}

fn finish(mut d: CandidateDecision) -> CandidateDecision {
    d.admitted_evidence_ids.sort();
    d.rejected_evidence_ids.sort();
    d.unresolved_hypotheses.sort();
    d.reason_codes.sort();
    d.reason_codes.dedup();
    d
}

fn unresolved() -> Vec<String> {
    vec![
        "physical_state_change".to_owned(),
        "instrument_measurement_fault".to_owned(),
        "in_path_integrity_anomaly".to_owned(),
    ]
}

/// Candidate-facing triage. It receives provenance-bearing evidence only; it
/// has no parameter for latent world identity or evaluator cause labels.
pub fn triage_observation_integrity(
    policy: &AdmissionPolicy,
    evidence: &[EvidenceRecord],
) -> Result<CandidateDecision, IntegrityAuditError> {
    if !valid_policy(policy) {
        return Err(IntegrityAuditError::InvalidPolicy);
    }

    let mut ids = BTreeSet::new();
    for r in evidence {
        validate_record(r)?;
        if !ids.insert(r.evidence_id.as_str()) {
            return Err(IntegrityAuditError::DuplicateEvidenceId);
        }
    }
    let primary = evidence
        .iter()
        .find(|r| r.kind == EvidenceKind::PrimaryTelemetry)
        .ok_or(IntegrityAuditError::MissingPrimaryTelemetry)?;

    let reject_primary = |reason: &str| {
        finish(CandidateDecision {
            scenario_id: policy.scenario_id.clone(),
            disposition: CandidateDisposition::RejectEvidence,
            primary_evidence_id: primary.evidence_id.clone(),
            primary_value: match primary.result {
                EvidenceResult::Numeric(v) => Some(v),
                _ => None,
            },
            admitted_evidence_ids: Vec::new(),
            rejected_evidence_ids: vec![primary.evidence_id.clone()],
            unresolved_hypotheses: Vec::new(),
            reason_codes: vec![reason.to_owned()],
        })
    };

    if !primary.authenticated {
        return Ok(reject_primary("primary_not_authenticated"));
    }
    if primary.class != EvidenceClass::Measured {
        return Ok(reject_primary("primary_not_direct_measurement"));
    }
    if let Some(reason) = freshness_failure(primary, policy) {
        return Ok(reject_primary(reason));
    }
    if !contract_matches(primary, policy) {
        return Ok(reject_primary(
            "primary_unit_calibration_or_configuration_mismatch",
        ));
    }
    let primary_value = match primary.result {
        EvidenceResult::Numeric(value) => value,
        _ => return Err(IntegrityAuditError::InvalidEvidenceShape),
    };

    let mut admitted = vec![primary.evidence_id.clone()];
    let mut rejected = Vec::new();
    let mut reasons = Vec::new();
    let mut physical = Vec::new();
    let mut forensic = false;
    let mut self_test_failed = false;

    for side in evidence
        .iter()
        .filter(|r| r.evidence_id != primary.evidence_id)
    {
        if !side.authenticated {
            rejected.push(side.evidence_id.clone());
            reasons.push(format!("{}:not_authenticated", side.evidence_id));
            continue;
        }
        if let Some(reason) = freshness_failure(side, policy) {
            rejected.push(side.evidence_id.clone());
            reasons.push(format!("{}:{reason}", side.evidence_id));
            continue;
        }
        match side.kind {
            EvidenceKind::IndependentPhysicalMeasurement => {
                if !contract_matches(side, policy) || side.class != EvidenceClass::Measured {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!(
                        "{}:measurement_contract_mismatch",
                        side.evidence_id
                    ));
                } else if dependency_relation(primary, side) == DependencyRelation::Correlated {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:shared_dependency", side.evidence_id));
                } else if dependency_relation(primary, side) == DependencyRelation::Unknown {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:dependency_unknown", side.evidence_id));
                } else {
                    admitted.push(side.evidence_id.clone());
                    if let EvidenceResult::Numeric(value) = side.result {
                        physical.push((side.evidence_id.clone(), value));
                    }
                }
            }
            EvidenceKind::InstrumentSelfTest => {
                if side.class != EvidenceClass::Attested {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:self_test_not_attested", side.evidence_id));
                } else {
                    match dependency_relation(primary, side) {
                        DependencyRelation::Correlated => {
                            rejected.push(side.evidence_id.clone());
                            reasons.push(format!("{}:shared_dependency", side.evidence_id));
                        }
                        DependencyRelation::Unknown => {
                            rejected.push(side.evidence_id.clone());
                            reasons.push(format!("{}:dependency_unknown", side.evidence_id));
                        }
                        DependencyRelation::IndependentForThisProfile => {
                            admitted.push(side.evidence_id.clone());
                            self_test_failed |= side.result == EvidenceResult::SelfTestFail;
                        }
                    }
                }
            }
            EvidenceKind::ForensicIntegrityReview => {
                if side.class != EvidenceClass::Attested {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!(
                        "{}:forensic_receipt_not_attested",
                        side.evidence_id
                    ));
                } else {
                    match dependency_relation(primary, side) {
                        DependencyRelation::Correlated => {
                            rejected.push(side.evidence_id.clone());
                            reasons.push(format!("{}:shared_dependency", side.evidence_id));
                        }
                        DependencyRelation::Unknown => {
                            rejected.push(side.evidence_id.clone());
                            reasons.push(format!("{}:dependency_unknown", side.evidence_id));
                        }
                        DependencyRelation::IndependentForThisProfile => {
                            admitted.push(side.evidence_id.clone());
                            forensic |= side.result == EvidenceResult::IntegrityIndicator;
                        }
                    }
                }
            }
            EvidenceKind::DerivedReconstruction => {
                rejected.push(side.evidence_id.clone());
                reasons.push(format!(
                    "{}:derived_evidence_not_independent",
                    side.evidence_id
                ));
            }
            EvidenceKind::PrimaryTelemetry => {
                rejected.push(side.evidence_id.clone());
                reasons.push(format!("{}:duplicate_primary_kind", side.evidence_id));
            }
        }
    }

    let primary_conflict = physical
        .iter()
        .any(|(_, value)| value.abs_diff(primary_value) > policy.numeric_tolerance);
    let cross_measurement_conflict = physical.iter().enumerate().any(|(index, (_, value))| {
        physical
            .iter()
            .skip(index + 1)
            .any(|(_, other)| value.abs_diff(*other) > policy.numeric_tolerance)
    });
    if primary_conflict || cross_measurement_conflict {
        reasons.push("independent_physical_measurement_conflict".to_owned());
        if cross_measurement_conflict {
            reasons.push("independent_physical_measurements_disagree_pairwise".to_owned());
        }
        return Ok(finish(CandidateDecision {
            scenario_id: policy.scenario_id.clone(),
            disposition: CandidateDisposition::EvidenceConflict,
            primary_evidence_id: primary.evidence_id.clone(),
            primary_value: Some(primary_value),
            admitted_evidence_ids: admitted,
            rejected_evidence_ids: rejected,
            unresolved_hypotheses: unresolved(),
            reason_codes: reasons,
        }));
    }
    if !physical.is_empty() {
        reasons.push("independent_physical_cross_check_agrees".to_owned());
        return Ok(finish(CandidateDecision {
            scenario_id: policy.scenario_id.clone(),
            disposition: CandidateDisposition::UseCorroboratedMeasurement,
            primary_evidence_id: primary.evidence_id.clone(),
            // Keep the primary measurement as the reported value; a supporting
            // record must not win merely because it appeared first in the slice.
            primary_value: Some(primary_value),
            admitted_evidence_ids: admitted,
            rejected_evidence_ids: rejected,
            unresolved_hypotheses: vec![
                "physical_state_supported_but_fault_cause_unattributed".to_owned(),
            ],
            reason_codes: reasons,
        }));
    }
    if forensic {
        reasons.push("independent_forensic_integrity_indicator".to_owned());
        return Ok(finish(CandidateDecision {
            scenario_id: policy.scenario_id.clone(),
            disposition: CandidateDisposition::IntegrityInvestigation,
            primary_evidence_id: primary.evidence_id.clone(),
            primary_value: None,
            admitted_evidence_ids: admitted,
            rejected_evidence_ids: rejected,
            unresolved_hypotheses: unresolved(),
            reason_codes: reasons,
        }));
    }
    if self_test_failed {
        reasons.push("self_test_failure_does_not_exclude_in_path_integrity".to_owned());
        return Ok(finish(CandidateDecision {
            scenario_id: policy.scenario_id.clone(),
            disposition: CandidateDisposition::RequestIndependentPhysicalMeasurement,
            primary_evidence_id: primary.evidence_id.clone(),
            primary_value: None,
            admitted_evidence_ids: admitted,
            rejected_evidence_ids: rejected,
            unresolved_hypotheses: unresolved(),
            reason_codes: reasons,
        }));
    }

    reasons.push("telemetry_signature_is_not_unique_cause_evidence".to_owned());
    Ok(finish(CandidateDecision {
        scenario_id: policy.scenario_id.clone(),
        disposition: CandidateDisposition::AbstainObservationalEquivalence,
        primary_evidence_id: primary.evidence_id.clone(),
        primary_value: None,
        admitted_evidence_ids: admitted,
        rejected_evidence_ids: rejected,
        unresolved_hypotheses: unresolved(),
        reason_codes: reasons,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> AdmissionPolicy {
        AdmissionPolicy {
            scenario_id: "equivalent-value-triage-v1".to_owned(),
            source_digest: "sha256-observation-integrity-source-v1".to_owned(),
            expected_unit: "service_index".to_owned(),
            expected_calibration_revision: "cal-7".to_owned(),
            expected_configuration_revision: "cfg-3".to_owned(),
            decision_cutoff_tick: 20,
            maximum_observation_age_ticks: 10,
            maximum_transport_lag_ticks: 2,
            numeric_tolerance: 2,
        }
    }

    fn record(
        id: &str,
        kind: EvidenceKind,
        class: EvidenceClass,
        result: EvidenceResult,
        source: &str,
        gateway: &str,
        trust: &str,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: id.to_owned(),
            kind,
            class,
            result,
            unit: "service_index".to_owned(),
            event_tick: 10,
            ingestion_tick: 10,
            source_id: source.to_owned(),
            gateway_id: gateway.to_owned(),
            trust_root_id: trust.to_owned(),
            dependencies: DependencyProfile {
                observation_root_id: Some(format!("observation-{id}")),
                upstream_dataset_id: Some(format!("dataset-{id}")),
                processing_lineage_id: Some(format!("pipeline-{id}")),
                operator_domain_id: Some(format!("operator-{source}")),
                instrument_lineage_id: Some(format!("instrument-{source}")),
            },
            calibration_revision: "cal-7".to_owned(),
            configuration_revision: "cfg-3".to_owned(),
            authenticated: true,
        }
    }

    fn primary() -> EvidenceRecord {
        record(
            "primary",
            EvidenceKind::PrimaryTelemetry,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "sensor-a",
            "gw-a",
            "root-a",
        )
    }

    #[test]
    fn telemetry_only_equivalence_abstains_without_cause_attribution() {
        let d = triage_observation_integrity(&policy(), &[primary()]).unwrap();
        assert_eq!(
            d.disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );
        assert_eq!(d.unresolved_hypotheses.len(), 3);
        assert!(d.primary_value.is_none());
    }

    #[test]
    fn shared_gateway_or_trust_root_does_not_break_equivalence() {
        let mut e = vec![primary()];
        e.push(record(
            "side",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "sensor-b",
            "gw-a",
            "root-a",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(
            d.disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );
        assert!(d.rejected_evidence_ids.contains(&"side".to_owned()));
    }

    #[test]
    fn different_brokers_republishing_one_observation_are_not_independent() {
        let primary = primary();
        let mut mirrored = record(
            "broker-b-copy",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "broker-b",
            "gateway-b",
            "trust-root-b",
        );
        mirrored.dependencies.observation_root_id =
            primary.dependencies.observation_root_id.clone();

        let decision = triage_observation_integrity(&policy(), &[primary, mirrored]).unwrap();
        assert_eq!(
            decision.disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );
        assert!(
            decision
                .reason_codes
                .iter()
                .any(|code| code == "broker-b-copy:shared_dependency")
        );
    }

    #[test]
    fn distinct_observations_from_one_upstream_dataset_remain_correlated() {
        let primary = primary();
        let mut republished = record(
            "national-gateway-copy",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "gateway-source-b",
            "gateway-b",
            "trust-root-b",
        );
        republished.dependencies.upstream_dataset_id =
            primary.dependencies.upstream_dataset_id.clone();

        let decision = triage_observation_integrity(&policy(), &[primary, republished]).unwrap();
        assert_eq!(
            decision.disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );
        assert!(
            decision
                .reason_codes
                .iter()
                .any(|code| code == "national-gateway-copy:shared_dependency")
        );
    }

    #[test]
    fn unknown_dependency_lineage_never_upgrades_to_independence() {
        let primary = primary();
        let mut unknown = record(
            "unknown-upstream",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "sensor-b",
            "gateway-b",
            "trust-root-b",
        );
        unknown.dependencies.upstream_dataset_id = None;

        let decision = triage_observation_integrity(&policy(), &[primary, unknown]).unwrap();
        assert_eq!(
            decision.disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );
        assert!(
            decision
                .reason_codes
                .iter()
                .any(|code| code == "unknown-upstream:dependency_unknown")
        );
    }

    #[test]
    fn shared_processing_lineage_is_a_common_cause_even_with_different_sources() {
        let primary = primary();
        let mut model_b = record(
            "model-b",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "sensor-b",
            "gateway-b",
            "trust-root-b",
        );
        model_b.dependencies.processing_lineage_id =
            primary.dependencies.processing_lineage_id.clone();

        let decision = triage_observation_integrity(&policy(), &[primary, model_b]).unwrap();
        assert_eq!(
            decision.disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );
        assert!(
            decision
                .reason_codes
                .iter()
                .any(|code| code == "model-b:shared_dependency")
        );
    }

    #[test]
    fn independent_physical_measurement_can_corroborate_but_not_attribute_cause() {
        let mut e = vec![primary()];
        e.push(record(
            "side",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(50),
            "sensor-b",
            "gw-b",
            "root-b",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(
            d.disposition,
            CandidateDisposition::UseCorroboratedMeasurement
        );
        assert_eq!(d.primary_value, Some(50));
        assert!(d.unresolved_hypotheses[0].contains("unattributed"));
    }

    #[test]
    fn independent_physical_conflict_is_not_resolved_by_source_count() {
        let mut e = vec![primary()];
        e.push(record(
            "side",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(90),
            "sensor-b",
            "gw-b",
            "root-b",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::EvidenceConflict);
    }

    #[test]
    fn stale_or_mismatched_primary_is_rejected() {
        let mut stale = primary();
        stale.event_tick = 1;
        assert_eq!(
            triage_observation_integrity(&policy(), &[stale])
                .unwrap()
                .disposition,
            CandidateDisposition::RejectEvidence
        );

        let mut bad_cal = primary();
        bad_cal.calibration_revision = "cal-old".to_owned();
        assert_eq!(
            triage_observation_integrity(&policy(), &[bad_cal])
                .unwrap()
                .disposition,
            CandidateDisposition::RejectEvidence
        );
    }

    #[test]
    fn derived_and_forensic_evidence_do_not_become_physical_or_cyber_truth() {
        let mut derived = vec![primary()];
        derived.push(record(
            "model",
            EvidenceKind::DerivedReconstruction,
            EvidenceClass::Reconstructed,
            EvidenceResult::Numeric(50),
            "model",
            "gw-model",
            "root-model",
        ));
        assert_eq!(
            triage_observation_integrity(&policy(), &derived)
                .unwrap()
                .disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );

        let mut forensic = vec![primary()];
        forensic.push(record(
            "forensic",
            EvidenceKind::ForensicIntegrityReview,
            EvidenceClass::Attested,
            EvidenceResult::IntegrityIndicator,
            "security",
            "gw-sec",
            "root-sec",
        ));
        let d = triage_observation_integrity(&policy(), &forensic).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::IntegrityInvestigation);
        assert_eq!(d.unresolved_hypotheses.len(), 3);
    }

    #[test]
    fn self_test_failure_requests_independent_measurement_not_cause_attribution() {
        let mut e = vec![primary()];
        e.push(record(
            "self-test",
            EvidenceKind::InstrumentSelfTest,
            EvidenceClass::Attested,
            EvidenceResult::SelfTestFail,
            "self-test",
            "gw-test",
            "root-test",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(
            d.disposition,
            CandidateDisposition::RequestIndependentPhysicalMeasurement
        );
        assert_eq!(d.unresolved_hypotheses.len(), 3);
    }

    #[test]
    fn multiple_cross_checks_are_pairwise_consistent_not_first_record_wins() {
        let primary = primary();
        let side_a = record(
            "side-a",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(48),
            "sensor-b",
            "gateway-b",
            "root-b",
        );
        let side_b = record(
            "side-b",
            EvidenceKind::IndependentPhysicalMeasurement,
            EvidenceClass::Measured,
            EvidenceResult::Numeric(52),
            "sensor-c",
            "gateway-c",
            "root-c",
        );
        let forward = vec![primary.clone(), side_a.clone(), side_b.clone()];
        let reverse = vec![primary, side_b, side_a];

        let forward_decision = triage_observation_integrity(&policy(), &forward).unwrap();
        let reverse_decision = triage_observation_integrity(&policy(), &reverse).unwrap();
        assert_eq!(
            forward_decision.disposition,
            CandidateDisposition::EvidenceConflict
        );
        assert_eq!(forward_decision.reason_codes, reverse_decision.reason_codes);
        assert_eq!(
            forward_decision.admitted_evidence_ids,
            reverse_decision.admitted_evidence_ids
        );
        assert_eq!(forward_decision, reverse_decision);
        assert!(
            forward_decision
                .reason_codes
                .contains(&"independent_physical_measurements_disagree_pairwise".to_owned())
        );
    }

    #[test]
    fn decision_cutoff_freshness_separates_age_lag_and_late_arrival() {
        let mut stale_at_cutoff = primary();
        stale_at_cutoff.event_tick = 1;
        stale_at_cutoff.ingestion_tick = 10;
        let stale = triage_observation_integrity(&policy(), &[stale_at_cutoff]).unwrap();
        assert_eq!(stale.disposition, CandidateDisposition::RejectEvidence);
        assert!(
            stale
                .reason_codes
                .contains(&"observation_stale_at_decision_cutoff".to_owned())
        );

        let mut high_transport_lag = primary();
        high_transport_lag.ingestion_tick = 13;
        let delayed = triage_observation_integrity(&policy(), &[high_transport_lag]).unwrap();
        assert_eq!(delayed.disposition, CandidateDisposition::RejectEvidence);
        assert!(
            delayed
                .reason_codes
                .contains(&"event_to_ingestion_lag_exceeded".to_owned())
        );

        let mut late_arrival = primary();
        late_arrival.ingestion_tick = 21;
        let late = triage_observation_integrity(&policy(), &[late_arrival]).unwrap();
        assert_eq!(late.disposition, CandidateDisposition::RejectEvidence);
        assert!(
            late.reason_codes
                .contains(&"arrived_after_decision_cutoff".to_owned())
        );
    }

    #[test]
    fn malformed_or_duplicate_evidence_fails_closed() {
        let mut e = vec![primary(), primary()];
        assert_eq!(
            triage_observation_integrity(&policy(), &e).unwrap_err(),
            IntegrityAuditError::DuplicateEvidenceId
        );
        e = vec![primary()];
        e[0].ingestion_tick = 1;
        assert_eq!(
            triage_observation_integrity(&policy(), &e).unwrap_err(),
            IntegrityAuditError::InvalidClockOrder
        );
    }
}

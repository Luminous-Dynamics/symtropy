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
pub enum ReceiptStatus {
    Verified,
    Unverified,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceResult {
    Numeric(i64),
    SelfTestPass,
    SelfTestFail,
    IntegrityIndicator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub class: EvidenceClass,
    pub result: EvidenceResult,
    pub unit: String,
    pub event_tick: u64,
    pub measurement_tick: u64,
    pub ingestion_tick: u64,
    pub maximum_age_ticks: u64,
    pub source_id: String,
    pub source_digest: String,
    pub acquisition_path_id: String,
    pub gateway_id: String,
    pub trust_root_id: String,
    pub time_root_id: String,
    pub calibration_authority_id: String,
    pub calibration_revision: String,
    pub configuration_revision: String,
    pub receipt_status: ReceiptStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPolicy {
    pub scenario_id: String,
    pub source_digest: String,
    pub assessment_tick: u64,
    pub expected_unit: String,
    pub expected_calibration_revision: String,
    pub expected_configuration_revision: String,
    pub maximum_age_ticks: u64,
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
            Self::RequestIndependentPhysicalMeasurement => "request_independent_physical_measurement",
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
        || record.source_digest.trim().is_empty()
        || record.acquisition_path_id.trim().is_empty()
        || record.gateway_id.trim().is_empty()
        || record.trust_root_id.trim().is_empty()
        || record.time_root_id.trim().is_empty()
        || record.calibration_authority_id.trim().is_empty()
        || record.calibration_revision.trim().is_empty()
        || record.configuration_revision.trim().is_empty()
    {
        return Err(IntegrityAuditError::InvalidEvidenceIdentity);
    }
    if record.event_tick > record.measurement_tick
        || record.measurement_tick > record.ingestion_tick
    {
        return Err(IntegrityAuditError::InvalidClockOrder);
    }
    let shape_ok = matches!(
        (record.kind, record.class, record.result),
        (EvidenceKind::PrimaryTelemetry, EvidenceClass::Measured, EvidenceResult::Numeric(_))
            | (EvidenceKind::IndependentPhysicalMeasurement, EvidenceClass::Measured, EvidenceResult::Numeric(_))
            | (EvidenceKind::DerivedReconstruction, EvidenceClass::Reconstructed, EvidenceResult::Numeric(_))
            | (EvidenceKind::InstrumentSelfTest, EvidenceClass::Attested, EvidenceResult::SelfTestPass)
            | (EvidenceKind::InstrumentSelfTest, EvidenceClass::Attested, EvidenceResult::SelfTestFail)
            | (EvidenceKind::ForensicIntegrityReview, EvidenceClass::Attested, EvidenceResult::IntegrityIndicator)
    );
    if !shape_ok {
        return Err(IntegrityAuditError::InvalidEvidenceShape);
    }
    Ok(())
}

fn fresh(record: &EvidenceRecord, policy: &AdmissionPolicy) -> bool {
    let allowed_age = policy.maximum_age_ticks.min(record.maximum_age_ticks);
    record.measurement_tick <= policy.assessment_tick
        && record.ingestion_tick <= policy.assessment_tick
        && policy.assessment_tick.saturating_sub(record.measurement_tick) <= allowed_age
}

fn contract_matches(record: &EvidenceRecord, policy: &AdmissionPolicy) -> bool {
    record.unit == policy.expected_unit
        && record.calibration_revision == policy.expected_calibration_revision
        && record.configuration_revision == policy.expected_configuration_revision
}

fn independent(primary: &EvidenceRecord, side: &EvidenceRecord) -> bool {
    primary.source_id != side.source_id
        && primary.acquisition_path_id != side.acquisition_path_id
        && primary.gateway_id != side.gateway_id
        && primary.trust_root_id != side.trust_root_id
        && primary.time_root_id != side.time_root_id
        && primary.calibration_authority_id != side.calibration_authority_id
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
            primary_value: match primary.result { EvidenceResult::Numeric(v) => Some(v), _ => None },
            admitted_evidence_ids: Vec::new(),
            rejected_evidence_ids: vec![primary.evidence_id.clone()],
            unresolved_hypotheses: Vec::new(),
            reason_codes: vec![reason.to_owned()],
        })
    };

    if primary.receipt_status != ReceiptStatus::Verified {
        return Ok(reject_primary("primary_receipt_not_verified"));
    }
    if primary.class != EvidenceClass::Measured {
        return Ok(reject_primary("primary_not_direct_measurement"));
    }
    if !fresh(primary, policy) {
        return Ok(reject_primary("primary_stale"));
    }
    if !contract_matches(primary, policy) {
        return Ok(reject_primary("primary_unit_calibration_or_configuration_mismatch"));
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

    for side in evidence.iter().filter(|r| r.evidence_id != primary.evidence_id) {
        if side.receipt_status != ReceiptStatus::Verified {
            rejected.push(side.evidence_id.clone());
            reasons.push(format!("{}:receipt_not_verified", side.evidence_id));
            continue;
        }
        if !fresh(side, policy) {
            rejected.push(side.evidence_id.clone());
            reasons.push(format!("{}:stale", side.evidence_id));
            continue;
        }
        match side.kind {
            EvidenceKind::IndependentPhysicalMeasurement => {
                if !contract_matches(side, policy) || side.class != EvidenceClass::Measured {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:measurement_contract_mismatch", side.evidence_id));
                } else if !independent(primary, side) {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:shared_dependency", side.evidence_id));
                } else {
                    admitted.push(side.evidence_id.clone());
                    if let EvidenceResult::Numeric(value) = side.result {
                        physical.push((side.evidence_id.clone(), value));
                    }
                }
            }
            EvidenceKind::InstrumentSelfTest => {
                if side.class != EvidenceClass::Attested || !independent(primary, side) {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:self_test_not_independent_or_attested", side.evidence_id));
                } else {
                    admitted.push(side.evidence_id.clone());
                    self_test_failed |= side.result == EvidenceResult::SelfTestFail;
                }
            }
            EvidenceKind::ForensicIntegrityReview => {
                if side.class != EvidenceClass::Attested || !independent(primary, side) {
                    rejected.push(side.evidence_id.clone());
                    reasons.push(format!("{}:forensic_receipt_not_independent_or_attested", side.evidence_id));
                } else {
                    admitted.push(side.evidence_id.clone());
                    forensic |= side.result == EvidenceResult::IntegrityIndicator;
                }
            }
            EvidenceKind::DerivedReconstruction => {
                rejected.push(side.evidence_id.clone());
                reasons.push(format!("{}:derived_evidence_not_independent", side.evidence_id));
            }
            EvidenceKind::PrimaryTelemetry => {
                rejected.push(side.evidence_id.clone());
                reasons.push(format!("{}:duplicate_primary_kind", side.evidence_id));
            }
        }
    }

    if physical.iter().any(|(_, value)| value.abs_diff(primary_value) > policy.numeric_tolerance) {
        reasons.push("independent_physical_measurement_conflict".to_owned());
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
    if let Some((_, value)) = physical.first() {
        reasons.push("independent_physical_cross_check_agrees".to_owned());
        return Ok(finish(CandidateDecision {
            scenario_id: policy.scenario_id.clone(),
            disposition: CandidateDisposition::UseCorroboratedMeasurement,
            primary_evidence_id: primary.evidence_id.clone(),
            primary_value: Some(*value),
            admitted_evidence_ids: admitted,
            rejected_evidence_ids: rejected,
            unresolved_hypotheses: vec!["physical_state_supported_but_fault_cause_unattributed".to_owned()],
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
            assessment_tick: 12,
            expected_unit: "service_index".to_owned(),
            expected_calibration_revision: "cal-7".to_owned(),
            expected_configuration_revision: "cfg-3".to_owned(),
            maximum_age_ticks: 2,
            numeric_tolerance: 2,
        }
    }

    fn record(id: &str, kind: EvidenceKind, class: EvidenceClass, result: EvidenceResult,
        source: &str, gateway: &str, trust: &str) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: id.to_owned(),
            kind,
            class,
            result,
            unit: "service_index".to_owned(),
            event_tick: 10,
            ingestion_tick: 10,
            measurement_tick: 10,
            maximum_age_ticks: 2,
            source_id: source.to_owned(),
            source_digest: format!("sha256-{source}"),
            acquisition_path_id: format!("path-{source}"),
            gateway_id: gateway.to_owned(),
            trust_root_id: trust.to_owned(),
            time_root_id: format!("time-{source}"),
            calibration_authority_id: format!("cal-authority-{source}"),
            calibration_revision: "cal-7".to_owned(),
            configuration_revision: "cfg-3".to_owned(),
            receipt_status: ReceiptStatus::Verified,
        }
    }

    fn primary() -> EvidenceRecord {
        record(
            "primary", EvidenceKind::PrimaryTelemetry, EvidenceClass::Measured,
            EvidenceResult::Numeric(50), "sensor-a", "gw-a", "root-a",
        )
    }

    #[test]
    fn telemetry_only_equivalence_abstains_without_cause_attribution() {
        let d = triage_observation_integrity(&policy(), &[primary()]).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::AbstainObservationalEquivalence);
        assert_eq!(d.unresolved_hypotheses.len(), 3);
        assert!(d.primary_value.is_none());
    }

    #[test]
    fn shared_gateway_or_trust_root_does_not_break_equivalence() {
        let mut e = vec![primary()];
        e.push(record(
            "side", EvidenceKind::IndependentPhysicalMeasurement, EvidenceClass::Measured,
            EvidenceResult::Numeric(50), "sensor-b", "gw-a", "root-a",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::AbstainObservationalEquivalence);
        assert!(d.rejected_evidence_ids.contains(&"side".to_owned()));
    }

    #[test]
    fn independent_physical_measurement_can_corroborate_but_not_attribute_cause() {
        let mut e = vec![primary()];
        e.push(record(
            "side", EvidenceKind::IndependentPhysicalMeasurement, EvidenceClass::Measured,
            EvidenceResult::Numeric(50), "sensor-b", "gw-b", "root-b",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::UseCorroboratedMeasurement);
        assert_eq!(d.primary_value, Some(50));
        assert!(d.unresolved_hypotheses[0].contains("unattributed"));
    }

    #[test]
    fn independent_physical_conflict_is_not_resolved_by_source_count() {
        let mut e = vec![primary()];
        e.push(record(
            "side", EvidenceKind::IndependentPhysicalMeasurement, EvidenceClass::Measured,
            EvidenceResult::Numeric(90), "sensor-b", "gw-b", "root-b",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::EvidenceConflict);
    }

    #[test]
    fn stale_or_mismatched_primary_is_rejected() {
        let mut stale = primary();
        stale.event_tick = 1;
        stale.measurement_tick = 1;
        stale.ingestion_tick = 1;
        assert_eq!(
            triage_observation_integrity(&policy(), &[stale]).unwrap().disposition,
            CandidateDisposition::RejectEvidence
        );

        let mut bad_cal = primary();
        bad_cal.calibration_revision = "cal-old".to_owned();
        assert_eq!(
            triage_observation_integrity(&policy(), &[bad_cal]).unwrap().disposition,
            CandidateDisposition::RejectEvidence
        );
    }

    #[test]
    fn derived_and_forensic_evidence_do_not_become_physical_or_cyber_truth() {
        let mut derived = vec![primary()];
        derived.push(record(
            "model", EvidenceKind::DerivedReconstruction, EvidenceClass::Reconstructed,
            EvidenceResult::Numeric(50), "model", "gw-model", "root-model",
        ));
        assert_eq!(
            triage_observation_integrity(&policy(), &derived).unwrap().disposition,
            CandidateDisposition::AbstainObservationalEquivalence
        );

        let mut forensic = vec![primary()];
        forensic.push(record(
            "forensic", EvidenceKind::ForensicIntegrityReview, EvidenceClass::Attested,
            EvidenceResult::IntegrityIndicator, "security", "gw-sec", "root-sec",
        ));
        let d = triage_observation_integrity(&policy(), &forensic).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::IntegrityInvestigation);
        assert_eq!(d.unresolved_hypotheses.len(), 3);
    }

    #[test]
    fn self_test_failure_requests_independent_measurement_not_cause_attribution() {
        let mut e = vec![primary()];
        e.push(record(
            "self-test", EvidenceKind::InstrumentSelfTest, EvidenceClass::Attested,
            EvidenceResult::SelfTestFail, "self-test", "gw-test", "root-test",
        ));
        let d = triage_observation_integrity(&policy(), &e).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::RequestIndependentPhysicalMeasurement);
        assert_eq!(d.unresolved_hypotheses.len(), 3);
    }

    #[test]
    fn independence_rejects_shared_path_time_root_or_calibration_authority() {
        let p = primary();
        for field in ["path", "time", "calibration_authority"] {
            let mut side = record(
                "side", EvidenceKind::IndependentPhysicalMeasurement, EvidenceClass::Measured,
                EvidenceResult::Numeric(50), "sensor-b", "gw-b", "root-b",
            );
            match field {
                "path" => side.acquisition_path_id = p.acquisition_path_id.clone(),
                "time" => side.time_root_id = p.time_root_id.clone(),
                "calibration_authority" => side.calibration_authority_id = p.calibration_authority_id.clone(),
                _ => unreachable!(),
            }
            assert!(!independent(&p, &side), "shared {field} must preserve dependence");
            let d = triage_observation_integrity(&policy(), &[p.clone(), side]).unwrap();
            assert_eq!(d.disposition, CandidateDisposition::AbstainObservationalEquivalence);
        }
    }

    #[test]
    fn unverified_and_invalid_side_receipts_do_not_count_as_corroboration() {
        for status in [ReceiptStatus::Unverified, ReceiptStatus::Invalid] {
            let mut side = record(
                "side", EvidenceKind::IndependentPhysicalMeasurement, EvidenceClass::Measured,
                EvidenceResult::Numeric(50), "sensor-b", "gw-b", "root-b",
            );
            side.receipt_status = status;
            let d = triage_observation_integrity(&policy(), &[primary(), side]).unwrap();
            assert_eq!(d.disposition, CandidateDisposition::AbstainObservationalEquivalence);
        }
    }

    #[test]
    fn source_record_ttl_and_assessment_time_both_constrain_freshness() {
        let mut record = primary();
        record.maximum_age_ticks = 1;
        let d = triage_observation_integrity(&policy(), &[record]).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::RejectEvidence);

        let mut future = primary();
        future.measurement_tick = 13;
        future.ingestion_tick = 13;
        future.event_tick = 12;
        let d = triage_observation_integrity(&policy(), &[future]).unwrap();
        assert_eq!(d.disposition, CandidateDisposition::RejectEvidence);
    }

    #[test]
    fn malformed_or_duplicate_evidence_fails_closed() {
        let mut e = vec![primary(), primary()];
        assert_eq!(
            triage_observation_integrity(&policy(), &e).unwrap_err(),
            IntegrityAuditError::DuplicateEvidenceId
        );
        e = vec![primary()];
        e[0].measurement_tick = 1;
        e[0].ingestion_tick = 1;
        assert_eq!(
            triage_observation_integrity(&policy(), &e).unwrap_err(),
            IntegrityAuditError::InvalidClockOrder
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed supervision of scope-bound PIV calibration validity.
//!
//! This crate checks already-qualified identities, feature-envelope evidence
//! and externally computed shift signals. It does not estimate density ratios,
//! detect drift, construct conformal intervals, authenticate receipts, operate
//! sensors, or prove physical passivity.
//!
//! A calibration certificate is a revocable capability scoped to a frozen
//! predictor, preprocessing path, acquisition configuration and physical
//! domain. A hard invalidator suspends that capability. A clean later sample
//! cannot silently restore it; explicit independent requalification is needed.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprints {
    pub predictor: String,
    pub preprocessing: String,
    pub acquisition_protocol: String,
    pub instrument: String,
    pub calibration: String,
    pub clock: String,
    pub reference_graph: String,
    pub observer_state: String,
}

impl Fingerprints {
    fn validate(&self) -> Result<(), CertificateError> {
        for (field, value) in [
            ("predictor", self.predictor.as_str()),
            ("preprocessing", self.preprocessing.as_str()),
            ("acquisition_protocol", self.acquisition_protocol.as_str()),
            ("instrument", self.instrument.as_str()),
            ("calibration", self.calibration.as_str()),
            ("clock", self.clock.as_str()),
            ("reference_graph", self.reference_graph.as_str()),
            ("observer_state", self.observer_state.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(CertificateError::EmptyIdentity { field });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeatureBound {
    pub name: String,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationMethod {
    /// Valid only while the declared exchangeability scope remains supported.
    StaticExchangeable,
    /// A covariate-shift method whose density-ratio support is checked below.
    /// The estimator and the underlying covariate-shift assumption are external.
    WeightedCovariateShift,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedSupportPolicy {
    pub minimum_effective_sample_size: f64,
    pub maximum_normalized_weight: f64,
    pub minimum_overlap_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationCertificate {
    pub certificate_id: String,
    pub fingerprints: Fingerprints,
    pub method: CalibrationMethod,
    pub issued_at_tick: u64,
    pub maximum_age_ticks: u64,
    pub scope: Vec<FeatureBound>,
    pub calibration_cluster_ids: Vec<String>,
    /// Sealed evaluation cohort fixed by this certificate, not supplied ad hoc
    /// by the caller when a failed certificate is requalified.
    pub sealed_test_cluster_ids: Vec<String>,
    pub weighted_support_policy: Option<WeightedSupportPolicy>,
}

impl CalibrationCertificate {
    pub fn validate(&self) -> Result<(), CertificateError> {
        if self.certificate_id.trim().is_empty() {
            return Err(CertificateError::EmptyIdentity {
                field: "certificate_id",
            });
        }
        self.fingerprints.validate()?;
        if self.maximum_age_ticks == 0 {
            return Err(CertificateError::InvalidAge);
        }
        if self.scope.is_empty() {
            return Err(CertificateError::EmptyScope);
        }

        let mut feature_names = BTreeSet::new();
        for bound in &self.scope {
            if bound.name.trim().is_empty() {
                return Err(CertificateError::EmptyIdentity {
                    field: "feature_name",
                });
            }
            if !bound.minimum.is_finite()
                || !bound.maximum.is_finite()
                || bound.minimum >= bound.maximum
            {
                return Err(CertificateError::InvalidFeatureBound {
                    feature: bound.name.clone(),
                });
            }
            if !feature_names.insert(bound.name.clone()) {
                return Err(CertificateError::DuplicateFeature {
                    feature: bound.name.clone(),
                });
            }
        }

        if self.calibration_cluster_ids.is_empty() {
            return Err(CertificateError::EmptyCalibrationCohort);
        }
        let mut cluster_ids = BTreeSet::new();
        for id in &self.calibration_cluster_ids {
            if id.trim().is_empty() {
                return Err(CertificateError::EmptyIdentity {
                    field: "calibration_cluster_id",
                });
            }
            if !cluster_ids.insert(id) {
                return Err(CertificateError::DuplicateCalibrationCluster {
                    cluster_id: id.clone(),
                });
            }
        }

        if self.sealed_test_cluster_ids.is_empty() {
            return Err(CertificateError::EmptySealedTestCohort);
        }
        let mut sealed_test_ids = BTreeSet::new();
        for id in &self.sealed_test_cluster_ids {
            if id.trim().is_empty() {
                return Err(CertificateError::EmptyIdentity {
                    field: "sealed_test_cluster_id",
                });
            }
            if !sealed_test_ids.insert(id) {
                return Err(CertificateError::DuplicateSealedTestCluster {
                    cluster_id: id.clone(),
                });
            }
        }
        if !cluster_ids.is_disjoint(&sealed_test_ids) {
            return Err(CertificateError::CalibrationAndTestOverlap);
        }

        match (self.method, self.weighted_support_policy.as_ref()) {
            (CalibrationMethod::StaticExchangeable, None) => {}
            (CalibrationMethod::StaticExchangeable, Some(_)) => {
                return Err(CertificateError::UnexpectedWeightedPolicy);
            }
            (CalibrationMethod::WeightedCovariateShift, Some(policy)) => {
                if !policy.minimum_effective_sample_size.is_finite()
                    || policy.minimum_effective_sample_size <= 0.0
                    || !policy.maximum_normalized_weight.is_finite()
                    || policy.maximum_normalized_weight < 1.0
                    || !policy.minimum_overlap_fraction.is_finite()
                    || !(0.0..=1.0).contains(&policy.minimum_overlap_fraction)
                {
                    return Err(CertificateError::InvalidWeightedSupportPolicy);
                }
            }
            (CalibrationMethod::WeightedCovariateShift, None) => {
                return Err(CertificateError::MissingWeightedPolicy);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedSupportEvidence {
    pub estimator_id: String,
    /// Independently qualified evidence that the conditional residual law is
    /// stable for the declared covariate-shift operation. A density-ratio
    /// estimate alone cannot establish this assumption.
    pub conditional_law_stability_evidence_id: String,
    pub effective_sample_size: f64,
    pub maximum_normalized_weight: f64,
    pub overlap_fraction: f64,
    /// Set only by the external density-ratio validator. This crate does not
    /// inspect or authenticate the estimator's source data.
    pub finite_ratio_evidence: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShiftSignal {
    CovariateShiftDetected,
    ConditionalResidualDriftDetected,
    ReferenceUnqualified,
    SharedCauseUnresolved,
    ObserverIntervention,
    ClockDiscontinuity,
    AcquisitionProtocolChanged,
    FeatureSupportUnknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveEvidence {
    pub evidence_id: String,
    pub dependence_cluster_id: String,
    pub tick: u64,
    pub fingerprints: Fingerprints,
    pub features: BTreeMap<String, f64>,
    pub signals: BTreeSet<ShiftSignal>,
    pub weighted_support: Option<WeightedSupportEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailureAction {
    RecalibrationRequired,
    IndependentDiagnosticRequired,
    AbstainOutOfEnvelope { feature: String },
    AbstainInsufficientEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    EvidenceIdentityMissing,
    EvidenceReplayOrOutOfOrder,
    CertificateStale,
    CalibrationClusterReused,
    PredictorChanged,
    PreprocessingChanged,
    AcquisitionChanged,
    InstrumentChanged,
    CalibrationChanged,
    ClockChanged,
    ReferenceChanged,
    ObserverChanged,
    ConditionalResidualDrift,
    CovariateShiftUnsupported,
    CovariateShiftWeightedSupportAdmitted,
    WeightedSupportInsufficient,
    ReferenceUnqualified,
    SharedCauseUnresolved,
    ObserverIntervention,
    ClockDiscontinuity,
    AcquisitionProtocolChanged,
    FeatureSupportUnknown,
    FeatureMissing,
    FeatureNonFinite,
    FeatureOutsideScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disposition {
    QualifiedInAdmittedScope,
    QualifiedWithWarning {
        reason: Reason,
    },
    RecalibrationRequired {
        reason: Reason,
    },
    IndependentDiagnosticRequired {
        reason: Reason,
    },
    AbstainOutOfEnvelope {
        feature: String,
    },
    AbstainInsufficientEvidence {
        reason: Reason,
    },
    AlreadySuspended {
        action: FailureAction,
        reason: Reason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateError {
    EmptyIdentity { field: &'static str },
    InvalidAge,
    EmptyScope,
    InvalidFeatureBound { feature: String },
    DuplicateFeature { feature: String },
    EmptyCalibrationCohort,
    DuplicateCalibrationCluster { cluster_id: String },
    EmptySealedTestCohort,
    DuplicateSealedTestCluster { cluster_id: String },
    CalibrationAndTestOverlap,
    UnexpectedWeightedPolicy,
    MissingWeightedPolicy,
    InvalidWeightedSupportPolicy,
}

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid calibration certificate: {self:?}")
    }
}

impl Error for CertificateError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequalificationError {
    NotSuspended,
    InvalidCertificate(CertificateError),
    CertificateIdReused,
    CertificateNotNewer,
    ReceiptIncomplete,
    CalibrationCohortMismatch,
    SealedTestCohortMismatch,
    CalibrationClustersOverlap,
    SealedTestClustersOverlap,
    CalibrationAndTestOverlap,
}

impl fmt::Display for RequalificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "calibration requalification rejected: {self:?}")
    }
}

impl Error for RequalificationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequalificationReceipt {
    pub calibration_cluster_ids: BTreeSet<String>,
    pub sealed_test_cluster_ids: BTreeSet<String>,
    pub predictor_frozen: bool,
    pub independent_reference_qualified: bool,
    /// This is an assertion from the caller; the crate does not authenticate it.
    pub external_evidence_verified: bool,
}

#[derive(Clone, Debug)]
struct Suspension {
    action: FailureAction,
    reason: Reason,
}

#[derive(Clone, Debug)]
pub struct CalibrationSupervisor {
    certificate: CalibrationCertificate,
    last_tick: Option<u64>,
    seen_evidence_ids: BTreeSet<String>,
    seen_dependence_clusters: BTreeSet<String>,
    suspension: Option<Suspension>,
}

impl CalibrationSupervisor {
    pub fn new(certificate: CalibrationCertificate) -> Result<Self, CertificateError> {
        certificate.validate()?;
        Ok(Self {
            certificate,
            last_tick: None,
            seen_evidence_ids: BTreeSet::new(),
            seen_dependence_clusters: BTreeSet::new(),
            suspension: None,
        })
    }

    pub fn certificate(&self) -> &CalibrationCertificate {
        &self.certificate
    }

    pub fn is_suspended(&self) -> bool {
        self.suspension.is_some()
    }

    pub fn observe(&mut self, evidence: &LiveEvidence) -> Disposition {
        if let Some(suspension) = &self.suspension {
            return Disposition::AlreadySuspended {
                action: suspension.action.clone(),
                reason: suspension.reason.clone(),
            };
        }

        if evidence.evidence_id.trim().is_empty()
            || evidence.dependence_cluster_id.trim().is_empty()
        {
            return self.suspend(
                FailureAction::AbstainInsufficientEvidence,
                Reason::EvidenceIdentityMissing,
            );
        }
        if self.seen_evidence_ids.contains(&evidence.evidence_id)
            || self
                .last_tick
                .is_some_and(|last_tick| evidence.tick <= last_tick)
        {
            return self.suspend(
                FailureAction::AbstainInsufficientEvidence,
                Reason::EvidenceReplayOrOutOfOrder,
            );
        }

        self.seen_evidence_ids.insert(evidence.evidence_id.clone());
        self.last_tick = Some(evidence.tick);
        self.seen_dependence_clusters
            .insert(evidence.dependence_cluster_id.clone());

        if evidence.tick < self.certificate.issued_at_tick
            || evidence.tick - self.certificate.issued_at_tick > self.certificate.maximum_age_ticks
        {
            return self.suspend(
                FailureAction::RecalibrationRequired,
                Reason::CertificateStale,
            );
        }

        if self
            .certificate
            .calibration_cluster_ids
            .iter()
            .any(|id| id == &evidence.dependence_cluster_id)
        {
            return self.suspend(
                FailureAction::AbstainInsufficientEvidence,
                Reason::CalibrationClusterReused,
            );
        }

        let expected = &self.certificate.fingerprints;
        let actual = &evidence.fingerprints;
        if expected.predictor != actual.predictor {
            return self.suspend(
                FailureAction::RecalibrationRequired,
                Reason::PredictorChanged,
            );
        }
        if expected.preprocessing != actual.preprocessing {
            return self.suspend(
                FailureAction::RecalibrationRequired,
                Reason::PreprocessingChanged,
            );
        }
        if expected.acquisition_protocol != actual.acquisition_protocol {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::AcquisitionChanged,
            );
        }
        if expected.instrument != actual.instrument {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::InstrumentChanged,
            );
        }
        if expected.calibration != actual.calibration {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::CalibrationChanged,
            );
        }
        if expected.clock != actual.clock {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::ClockChanged,
            );
        }
        if expected.reference_graph != actual.reference_graph {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::ReferenceChanged,
            );
        }
        if expected.observer_state != actual.observer_state {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::ObserverChanged,
            );
        }

        for bound in &self.certificate.scope {
            let Some(value) = evidence.features.get(&bound.name) else {
                return self.suspend(
                    FailureAction::AbstainInsufficientEvidence,
                    Reason::FeatureMissing,
                );
            };
            if !value.is_finite() {
                return self.suspend(
                    FailureAction::AbstainInsufficientEvidence,
                    Reason::FeatureNonFinite,
                );
            }
            if *value < bound.minimum || *value > bound.maximum {
                return self.suspend(
                    FailureAction::AbstainOutOfEnvelope {
                        feature: bound.name.clone(),
                    },
                    Reason::FeatureOutsideScope,
                );
            }
        }

        if evidence
            .signals
            .contains(&ShiftSignal::FeatureSupportUnknown)
        {
            return self.suspend(
                FailureAction::AbstainInsufficientEvidence,
                Reason::FeatureSupportUnknown,
            );
        }
        if evidence
            .signals
            .contains(&ShiftSignal::ReferenceUnqualified)
        {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::ReferenceUnqualified,
            );
        }
        if evidence
            .signals
            .contains(&ShiftSignal::SharedCauseUnresolved)
        {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::SharedCauseUnresolved,
            );
        }
        if evidence
            .signals
            .contains(&ShiftSignal::ObserverIntervention)
        {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::ObserverIntervention,
            );
        }
        if evidence.signals.contains(&ShiftSignal::ClockDiscontinuity) {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::ClockDiscontinuity,
            );
        }
        if evidence
            .signals
            .contains(&ShiftSignal::AcquisitionProtocolChanged)
        {
            return self.suspend(
                FailureAction::IndependentDiagnosticRequired,
                Reason::AcquisitionProtocolChanged,
            );
        }
        if evidence
            .signals
            .contains(&ShiftSignal::ConditionalResidualDriftDetected)
        {
            return self.suspend(
                FailureAction::RecalibrationRequired,
                Reason::ConditionalResidualDrift,
            );
        }

        if evidence
            .signals
            .contains(&ShiftSignal::CovariateShiftDetected)
        {
            if self.certificate.method != CalibrationMethod::WeightedCovariateShift {
                return self.suspend(
                    FailureAction::RecalibrationRequired,
                    Reason::CovariateShiftUnsupported,
                );
            }
            let Some(policy) = self.certificate.weighted_support_policy.as_ref() else {
                return self.suspend(
                    FailureAction::AbstainInsufficientEvidence,
                    Reason::WeightedSupportInsufficient,
                );
            };
            let Some(support) = evidence.weighted_support.as_ref() else {
                return self.suspend(
                    FailureAction::AbstainInsufficientEvidence,
                    Reason::WeightedSupportInsufficient,
                );
            };
            let adequate = !support.estimator_id.trim().is_empty()
                && !support
                    .conditional_law_stability_evidence_id
                    .trim()
                    .is_empty()
                && support.finite_ratio_evidence
                && support.effective_sample_size.is_finite()
                && support.effective_sample_size >= policy.minimum_effective_sample_size
                && support.maximum_normalized_weight.is_finite()
                && support.maximum_normalized_weight >= 1.0
                && support.maximum_normalized_weight <= policy.maximum_normalized_weight
                && support.overlap_fraction.is_finite()
                && (0.0..=1.0).contains(&support.overlap_fraction)
                && support.overlap_fraction >= policy.minimum_overlap_fraction;
            if !adequate {
                return self.suspend(
                    FailureAction::AbstainInsufficientEvidence,
                    Reason::WeightedSupportInsufficient,
                );
            }
            return Disposition::QualifiedWithWarning {
                reason: Reason::CovariateShiftWeightedSupportAdmitted,
            };
        }

        Disposition::QualifiedInAdmittedScope
    }

    pub fn requalify(
        &mut self,
        next: CalibrationCertificate,
        receipt: &RequalificationReceipt,
    ) -> Result<(), RequalificationError> {
        if self.suspension.is_none() {
            return Err(RequalificationError::NotSuspended);
        }
        next.validate()
            .map_err(RequalificationError::InvalidCertificate)?;
        if next.certificate_id == self.certificate.certificate_id {
            return Err(RequalificationError::CertificateIdReused);
        }
        if next.issued_at_tick <= self.certificate.issued_at_tick
            || self
                .last_tick
                .is_some_and(|last_tick| next.issued_at_tick < last_tick)
        {
            return Err(RequalificationError::CertificateNotNewer);
        }
        if !receipt.predictor_frozen
            || !receipt.independent_reference_qualified
            || !receipt.external_evidence_verified
            || receipt.calibration_cluster_ids.is_empty()
            || receipt.sealed_test_cluster_ids.is_empty()
        {
            return Err(RequalificationError::ReceiptIncomplete);
        }

        let declared_calibration: BTreeSet<String> =
            next.calibration_cluster_ids.iter().cloned().collect();
        if declared_calibration != receipt.calibration_cluster_ids {
            return Err(RequalificationError::CalibrationCohortMismatch);
        }
        let declared_sealed_test: BTreeSet<String> =
            next.sealed_test_cluster_ids.iter().cloned().collect();
        if declared_sealed_test != receipt.sealed_test_cluster_ids {
            return Err(RequalificationError::SealedTestCohortMismatch);
        }
        if !declared_calibration.is_disjoint(&receipt.sealed_test_cluster_ids) {
            return Err(RequalificationError::CalibrationAndTestOverlap);
        }

        let mut prior_clusters: BTreeSet<String> = self
            .certificate
            .calibration_cluster_ids
            .iter()
            .cloned()
            .collect();
        prior_clusters.extend(self.certificate.sealed_test_cluster_ids.iter().cloned());
        prior_clusters.extend(self.seen_dependence_clusters.iter().cloned());

        if !declared_calibration.is_disjoint(&prior_clusters) {
            return Err(RequalificationError::CalibrationClustersOverlap);
        }
        if !receipt.sealed_test_cluster_ids.is_disjoint(&prior_clusters) {
            return Err(RequalificationError::SealedTestClustersOverlap);
        }

        self.certificate = next;
        self.suspension = None;
        Ok(())
    }

    fn suspend(&mut self, action: FailureAction, reason: Reason) -> Disposition {
        self.suspension = Some(Suspension {
            action: action.clone(),
            reason: reason.clone(),
        });
        match action {
            FailureAction::RecalibrationRequired => Disposition::RecalibrationRequired { reason },
            FailureAction::IndependentDiagnosticRequired => {
                Disposition::IndependentDiagnosticRequired { reason }
            }
            FailureAction::AbstainOutOfEnvelope { feature } => {
                Disposition::AbstainOutOfEnvelope { feature }
            }
            FailureAction::AbstainInsufficientEvidence => {
                Disposition::AbstainInsufficientEvidence { reason }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprints() -> Fingerprints {
        Fingerprints {
            predictor: "predictor-v1".into(),
            preprocessing: "preprocess-v1".into(),
            acquisition_protocol: "protocol-v1".into(),
            instrument: "instrument-v1".into(),
            calibration: "calibration-v1".into(),
            clock: "clock-v1".into(),
            reference_graph: "reference-v1".into(),
            observer_state: "observer-v1".into(),
        }
    }

    fn certificate() -> CalibrationCertificate {
        CalibrationCertificate {
            certificate_id: "certificate-v1".into(),
            fingerprints: fingerprints(),
            method: CalibrationMethod::StaticExchangeable,
            issued_at_tick: 10,
            maximum_age_ticks: 100,
            scope: vec![
                FeatureBound {
                    name: "velocity".into(),
                    minimum: 0.0,
                    maximum: 10.0,
                },
                FeatureBound {
                    name: "reynolds".into(),
                    minimum: 100.0,
                    maximum: 10_000.0,
                },
            ],
            calibration_cluster_ids: vec!["cal-a".into(), "cal-b".into()],
            sealed_test_cluster_ids: vec!["test-old-a".into(), "test-old-b".into()],
            weighted_support_policy: None,
        }
    }

    fn evidence(id: &str, tick: u64) -> LiveEvidence {
        LiveEvidence {
            evidence_id: id.into(),
            dependence_cluster_id: format!("session-{id}"),
            tick,
            fingerprints: fingerprints(),
            features: BTreeMap::from([("velocity".into(), 3.0), ("reynolds".into(), 1_000.0)]),
            signals: BTreeSet::new(),
            weighted_support: None,
        }
    }

    #[test]
    fn unchanged_evidence_remains_in_admitted_scope() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        assert_eq!(
            supervisor.observe(&evidence("e-1", 11)),
            Disposition::QualifiedInAdmittedScope
        );
    }

    #[test]
    fn out_of_scope_feature_suspends_the_certificate() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.features.insert("velocity".into(), 10.1);
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::AbstainOutOfEnvelope {
                feature: "velocity".into()
            }
        );
        assert!(supervisor.is_suspended());
        assert_eq!(
            supervisor.observe(&evidence("e-2", 12)),
            Disposition::AlreadySuspended {
                action: FailureAction::AbstainOutOfEnvelope {
                    feature: "velocity".into()
                },
                reason: Reason::FeatureOutsideScope
            }
        );
    }

    #[test]
    fn predictor_change_requires_recalibration() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.fingerprints.predictor = "predictor-v2".into();
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::RecalibrationRequired {
                reason: Reason::PredictorChanged
            }
        );
    }

    #[test]
    fn observer_change_requires_an_independent_diagnostic() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.signals.insert(ShiftSignal::ObserverIntervention);
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::IndependentDiagnosticRequired {
                reason: Reason::ObserverIntervention
            }
        );
    }

    type FingerprintMutationCase = (&'static str, fn(&mut Fingerprints), Reason);

    #[test]
    fn measurement_identity_changes_require_independent_diagnostics() {
        let cases: [FingerprintMutationCase; 6] = [
            (
                "acquisition",
                |value| value.acquisition_protocol.push_str("-changed"),
                Reason::AcquisitionChanged,
            ),
            (
                "instrument",
                |value| value.instrument.push_str("-changed"),
                Reason::InstrumentChanged,
            ),
            (
                "calibration",
                |value| value.calibration.push_str("-changed"),
                Reason::CalibrationChanged,
            ),
            (
                "clock",
                |value| value.clock.push_str("-changed"),
                Reason::ClockChanged,
            ),
            (
                "reference",
                |value| value.reference_graph.push_str("-changed"),
                Reason::ReferenceChanged,
            ),
            (
                "observer",
                |value| value.observer_state.push_str("-changed"),
                Reason::ObserverChanged,
            ),
        ];

        for (index, (_name, mutate, reason)) in cases.into_iter().enumerate() {
            let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
            let mut sample = evidence(&format!("identity-{index}"), 11);
            mutate(&mut sample.fingerprints);
            assert_eq!(
                supervisor.observe(&sample),
                Disposition::IndependentDiagnosticRequired { reason }
            );
        }
    }

    #[test]
    fn expired_certificate_requires_recalibration() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        assert_eq!(
            supervisor.observe(&evidence("expired", 111)),
            Disposition::RecalibrationRequired {
                reason: Reason::CertificateStale
            }
        );
    }

    #[test]
    fn non_finite_scope_feature_abstains() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut sample = evidence("non-finite", 11);
        sample.features.insert("velocity".into(), f64::NAN);
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::AbstainInsufficientEvidence {
                reason: Reason::FeatureNonFinite
            }
        );
    }

    #[test]
    fn static_calibration_does_not_claim_validity_after_covariate_shift() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.signals.insert(ShiftSignal::CovariateShiftDetected);
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::RecalibrationRequired {
                reason: Reason::CovariateShiftUnsupported
            }
        );
    }

    #[test]
    fn weighted_shift_requires_finite_supported_weights() {
        let mut cert = certificate();
        cert.method = CalibrationMethod::WeightedCovariateShift;
        cert.weighted_support_policy = Some(WeightedSupportPolicy {
            minimum_effective_sample_size: 50.0,
            maximum_normalized_weight: 20.0,
            minimum_overlap_fraction: 0.8,
        });
        let mut supervisor = CalibrationSupervisor::new(cert).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.signals.insert(ShiftSignal::CovariateShiftDetected);
        sample.weighted_support = Some(WeightedSupportEvidence {
            estimator_id: "ratio-estimator-v1".into(),
            conditional_law_stability_evidence_id: "conditional-law-audit-v1".into(),
            effective_sample_size: 75.0,
            maximum_normalized_weight: 4.0,
            overlap_fraction: 0.95,
            finite_ratio_evidence: true,
        });
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::QualifiedWithWarning {
                reason: Reason::CovariateShiftWeightedSupportAdmitted
            }
        );
    }

    #[test]
    fn weighted_shift_abstains_when_effective_sample_size_is_too_small() {
        let mut cert = certificate();
        cert.method = CalibrationMethod::WeightedCovariateShift;
        cert.weighted_support_policy = Some(WeightedSupportPolicy {
            minimum_effective_sample_size: 50.0,
            maximum_normalized_weight: 20.0,
            minimum_overlap_fraction: 0.8,
        });
        let mut supervisor = CalibrationSupervisor::new(cert).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.signals.insert(ShiftSignal::CovariateShiftDetected);
        sample.weighted_support = Some(WeightedSupportEvidence {
            estimator_id: "ratio-estimator-v1".into(),
            conditional_law_stability_evidence_id: "conditional-law-audit-v1".into(),
            effective_sample_size: 12.0,
            maximum_normalized_weight: 4.0,
            overlap_fraction: 0.95,
            finite_ratio_evidence: true,
        });
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::AbstainInsufficientEvidence {
                reason: Reason::WeightedSupportInsufficient
            }
        );
    }

    #[test]
    fn weighted_shift_abstains_without_conditional_law_support() {
        let mut cert = certificate();
        cert.method = CalibrationMethod::WeightedCovariateShift;
        cert.weighted_support_policy = Some(WeightedSupportPolicy {
            minimum_effective_sample_size: 50.0,
            maximum_normalized_weight: 20.0,
            minimum_overlap_fraction: 0.8,
        });
        let mut supervisor = CalibrationSupervisor::new(cert).expect("valid fixture");
        let mut sample = evidence("e-1", 11);
        sample.signals.insert(ShiftSignal::CovariateShiftDetected);
        sample.weighted_support = Some(WeightedSupportEvidence {
            estimator_id: "ratio-estimator-v1".into(),
            conditional_law_stability_evidence_id: String::new(),
            effective_sample_size: 75.0,
            maximum_normalized_weight: 4.0,
            overlap_fraction: 0.95,
            finite_ratio_evidence: true,
        });
        assert_eq!(
            supervisor.observe(&sample),
            Disposition::AbstainInsufficientEvidence {
                reason: Reason::WeightedSupportInsufficient
            }
        );
    }

    #[test]
    fn replayed_or_out_of_order_evidence_suspends_the_supervisor() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        assert_eq!(
            supervisor.observe(&evidence("e-1", 11)),
            Disposition::QualifiedInAdmittedScope
        );
        assert_eq!(
            supervisor.observe(&evidence("e-1", 12)),
            Disposition::AbstainInsufficientEvidence {
                reason: Reason::EvidenceReplayOrOutOfOrder
            }
        );
        assert!(supervisor.is_suspended());
        assert_eq!(
            supervisor.observe(&evidence("e-2", 13)),
            Disposition::AlreadySuspended {
                action: FailureAction::AbstainInsufficientEvidence,
                reason: Reason::EvidenceReplayOrOutOfOrder
            }
        );
    }

    #[test]
    fn fresh_samples_cannot_self_restore_a_suspended_certificate() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut changed = evidence("e-1", 11);
        changed
            .signals
            .insert(ShiftSignal::ConditionalResidualDriftDetected);
        assert!(matches!(
            supervisor.observe(&changed),
            Disposition::RecalibrationRequired { .. }
        ));
        assert!(matches!(
            supervisor.observe(&evidence("e-2", 12)),
            Disposition::AlreadySuspended { .. }
        ));
    }

    #[test]
    fn requalification_requires_fresh_disjoint_calibration_and_sealed_test_clusters() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut changed = evidence("e-1", 11);
        changed
            .signals
            .insert(ShiftSignal::ConditionalResidualDriftDetected);
        let _ = supervisor.observe(&changed);

        let mut next = certificate();
        next.certificate_id = "certificate-v2".into();
        next.issued_at_tick = 12;
        next.calibration_cluster_ids = vec!["cal-c".into(), "cal-d".into()];
        next.sealed_test_cluster_ids = vec!["test-a".into(), "test-b".into()];
        let receipt = RequalificationReceipt {
            calibration_cluster_ids: BTreeSet::from(["cal-c".into(), "cal-d".into()]),
            sealed_test_cluster_ids: BTreeSet::from(["test-a".into(), "test-b".into()]),
            predictor_frozen: true,
            independent_reference_qualified: true,
            external_evidence_verified: true,
        };
        assert_eq!(supervisor.requalify(next, &receipt), Ok(()));
        assert!(!supervisor.is_suspended());
        assert_eq!(
            supervisor.observe(&evidence("e-2", 13)),
            Disposition::QualifiedInAdmittedScope
        );
    }

    #[test]
    fn requalification_rejects_reuse_of_observed_dependence_clusters() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut changed = evidence("e-1", 11);
        changed
            .signals
            .insert(ShiftSignal::ConditionalResidualDriftDetected);
        let _ = supervisor.observe(&changed);

        let mut next = certificate();
        next.certificate_id = "certificate-v2".into();
        next.issued_at_tick = 12;
        next.calibration_cluster_ids = vec!["session-e-1".into()];
        next.sealed_test_cluster_ids = vec!["test-a".into(), "test-b".into()];
        let receipt = RequalificationReceipt {
            calibration_cluster_ids: BTreeSet::from(["session-e-1".into()]),
            sealed_test_cluster_ids: BTreeSet::from(["test-a".into(), "test-b".into()]),
            predictor_frozen: true,
            independent_reference_qualified: true,
            external_evidence_verified: true,
        };
        assert_eq!(
            supervisor.requalify(next, &receipt),
            Err(RequalificationError::CalibrationClustersOverlap)
        );
    }
    #[test]
    fn requalification_rejects_receipt_test_cohort_not_bound_by_certificate() {
        let mut supervisor = CalibrationSupervisor::new(certificate()).expect("valid fixture");
        let mut changed = evidence("e-1", 11);
        changed
            .signals
            .insert(ShiftSignal::ConditionalResidualDriftDetected);
        let _ = supervisor.observe(&changed);

        let mut next = certificate();
        next.certificate_id = "certificate-v2".into();
        next.issued_at_tick = 12;
        next.calibration_cluster_ids = vec!["cal-c".into(), "cal-d".into()];
        next.sealed_test_cluster_ids = vec!["sealed-a".into(), "sealed-b".into()];
        let receipt = RequalificationReceipt {
            calibration_cluster_ids: BTreeSet::from(["cal-c".into(), "cal-d".into()]),
            sealed_test_cluster_ids: BTreeSet::from(["unbound-a".into(), "unbound-b".into()]),
            predictor_frozen: true,
            independent_reference_qualified: true,
            external_evidence_verified: true,
        };
        assert_eq!(
            supervisor.requalify(next, &receipt),
            Err(RequalificationError::SealedTestCohortMismatch)
        );
    }
}

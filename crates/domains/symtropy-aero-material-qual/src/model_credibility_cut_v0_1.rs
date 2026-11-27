// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Claim-scoped model credibility/applicability evidence cut.
//!
//! This is a typed adapter to AeroWeft's owner-produced VVUQ and applicability
//! package. It preserves evidence identities, model/configuration/domain,
//! independent data splits, modality-specific evidence, uncertainty components
//! and per-stratum metric reports. It does not calculate VVUQ metrics, validate
//! physics, rank validation modalities, or replace the model-credibility owner.

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationModality {
    SyntheticKnownTruth,
    AnalyticReference,
    SoftwareInLoop,
    HardwareInLoop,
    CalibratedComponentBench,
    ControlledFlightTest,
    ShadowModeFieldReplay,
}

impl ValidationModality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyntheticKnownTruth => "synthetic_known_truth",
            Self::AnalyticReference => "analytic_reference",
            Self::SoftwareInLoop => "software_in_loop",
            Self::HardwareInLoop => "hardware_in_loop",
            Self::CalibratedComponentBench => "calibrated_component_bench",
            Self::ControlledFlightTest => "controlled_flight_test",
            Self::ShadowModeFieldReplay => "shadow_mode_field_replay",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModalityEvidenceStatus {
    QualifiedForDeclaredPurpose,
    Unqualified,
    NotAvailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPackageStatus {
    StructurallyComplete,
    Incomplete,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitIntegrityStatus {
    DisjointByIndependentUnit,
    LeakageDetected,
    Unverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDomainCoverageStatus {
    Covered,
    NotCovered,
    Indeterminate,
    Unverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependentReferenceStatus {
    Qualified,
    Unverified,
    Absent,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyComponentKind {
    ObservationNoise,
    MeasurementUncertainty,
    ParameterUncertainty,
    StructuralModelDiscrepancy,
    EnvironmentalUncertainty,
}

impl UncertaintyComponentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ObservationNoise => "observation_noise",
            Self::MeasurementUncertainty => "measurement_uncertainty",
            Self::ParameterUncertainty => "parameter_uncertainty",
            Self::StructuralModelDiscrepancy => "structural_model_discrepancy",
            Self::EnvironmentalUncertainty => "environmental_uncertainty",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyEvidenceStatus {
    Characterized,
    Uncharacterized,
    Unverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricStratumStatus {
    WithinDeclaredProfile,
    OutsideDeclaredProfile,
    Indeterminate,
    Unverified,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModalityEvidenceV0_1 {
    pub modality: ValidationModality,
    pub evidence_ref: String,
    pub report_sha256: String,
    pub scope_manifest_sha256: String,
    pub status: ModalityEvidenceStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyComponentEvidenceV0_1 {
    pub component: UncertaintyComponentKind,
    pub method_profile_sha256: String,
    pub report_sha256: String,
    pub status: UncertaintyEvidenceStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredMetricStratumV0_1 {
    pub metric_id: String,
    pub stratum_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricStratumReportV0_1 {
    pub metric_id: String,
    pub stratum_ref: String,
    /// Exact population/grouping manifest used for this metric/stratum.
    pub population_manifest_sha256: String,
    /// Exact metric result report; AERO does not reinterpret its unit/threshold.
    pub report_sha256: String,
    /// Exact uncertainty/interval report for this metric/stratum.
    pub uncertainty_report_sha256: String,
    pub sample_count: u64,
    pub adverse_count: u64,
    pub status: MetricStratumStatus,
}

/// Typed owner-produced model credibility cut for one exact claim/configuration/domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCredibilityCutV0_1 {
    pub schema: String,
    /// Owner-native evidence reference included in the broader owner evidence cut.
    pub evidence_ref: String,
    pub claim_id: String,
    pub model_ref: String,
    pub model_artifact_sha256: String,
    pub solver_configuration_sha256: String,
    pub configuration_ref: String,
    pub operating_envelope_ref: String,
    /// Must equal DecisionContextV0_1.model_applicability_profile_sha256.
    pub applicability_profile_sha256: String,
    /// Must equal DecisionContextV0_1.risk_classification_sha256.
    pub risk_classification_sha256: String,
    pub validation_package_sha256: String,
    pub validation_protocol_sha256: String,
    pub calibration_manifest_sha256: String,
    pub development_manifest_sha256: String,
    pub confirmatory_manifest_sha256: String,
    pub independent_reference_manifest_sha256: String,
    pub package_status: ModelPackageStatus,
    pub split_integrity_status: SplitIntegrityStatus,
    pub independent_reference_status: IndependentReferenceStatus,
    pub domain_coverage_status: ModelDomainCoverageStatus,
    /// Declared by the model/profile owner; no universal modality ladder is inferred.
    pub required_modalities: Vec<ValidationModality>,
    pub modality_evidence: Vec<ModalityEvidenceV0_1>,
    /// Keep observation, measurement, parameter, model-form and environmental terms separate.
    pub required_uncertainty_components: Vec<UncertaintyComponentKind>,
    pub uncertainty_components: Vec<UncertaintyComponentEvidenceV0_1>,
    /// Required metric/stratum pairs, with no aggregate score or cross-stratum averaging.
    pub required_metric_strata: Vec<RequiredMetricStratumV0_1>,
    pub metric_stratum_reports: Vec<MetricStratumReportV0_1>,
}

#[derive(Serialize)]
struct ModelCredibilityEnvelope<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    evidence_ref: &'a str,
    claim_id: &'a str,
    model_ref: &'a str,
    model_artifact_sha256: &'a str,
    solver_configuration_sha256: &'a str,
    configuration_ref: &'a str,
    operating_envelope_ref: &'a str,
    applicability_profile_sha256: &'a str,
    risk_classification_sha256: &'a str,
    validation_package_sha256: &'a str,
    validation_protocol_sha256: &'a str,
    calibration_manifest_sha256: &'a str,
    development_manifest_sha256: &'a str,
    confirmatory_manifest_sha256: &'a str,
    independent_reference_manifest_sha256: &'a str,
    package_status: ModelPackageStatus,
    split_integrity_status: SplitIntegrityStatus,
    independent_reference_status: IndependentReferenceStatus,
    domain_coverage_status: ModelDomainCoverageStatus,
    required_modalities: Vec<ValidationModality>,
    modality_evidence: Vec<&'a ModalityEvidenceV0_1>,
    required_uncertainty_components: Vec<UncertaintyComponentKind>,
    uncertainty_components: Vec<&'a UncertaintyComponentEvidenceV0_1>,
    required_metric_strata: Vec<&'a RequiredMetricStratumV0_1>,
    metric_stratum_reports: Vec<&'a MetricStratumReportV0_1>,
}

impl ModelCredibilityCutV0_1 {
    /// Validate the cut's identity, digest, uniqueness and metric arithmetic.
    /// Missing required evidence is retained as a reviewability blocker, not repaired.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.schema != "AeroModelCredibilityCutV0_1" {
            errors.push("MODEL_CREDIBILITY_SCHEMA_INVALID".to_owned());
        }
        for (field, value) in [
            ("EVIDENCE_REF", self.evidence_ref.as_str()),
            ("CLAIM_ID", self.claim_id.as_str()),
            ("MODEL_REF", self.model_ref.as_str()),
            ("CONFIGURATION_REF", self.configuration_ref.as_str()),
            ("OPERATING_ENVELOPE_REF", self.operating_envelope_ref.as_str()),
        ] {
            if !valid_identifier(value) {
                errors.push(format!("MODEL_CREDIBILITY_{field}_INVALID"));
            }
        }
        for (field, value) in [
            ("MODEL_ARTIFACT", self.model_artifact_sha256.as_str()),
            ("SOLVER_CONFIGURATION", self.solver_configuration_sha256.as_str()),
            ("APPLICABILITY_PROFILE", self.applicability_profile_sha256.as_str()),
            ("RISK_CLASSIFICATION", self.risk_classification_sha256.as_str()),
            ("VALIDATION_PACKAGE", self.validation_package_sha256.as_str()),
            ("VALIDATION_PROTOCOL", self.validation_protocol_sha256.as_str()),
            ("CALIBRATION_MANIFEST", self.calibration_manifest_sha256.as_str()),
            ("DEVELOPMENT_MANIFEST", self.development_manifest_sha256.as_str()),
            ("CONFIRMATORY_MANIFEST", self.confirmatory_manifest_sha256.as_str()),
            ("INDEPENDENT_REFERENCE_MANIFEST", self.independent_reference_manifest_sha256.as_str()),
        ] {
            if !valid_sha256(value) {
                errors.push(format!("MODEL_CREDIBILITY_{field}_DIGEST_INVALID"));
            }
        }

        if self.required_modalities.is_empty() {
            errors.push("MODEL_CREDIBILITY_REQUIRED_MODALITIES_EMPTY".to_owned());
        }
        if self.required_uncertainty_components.is_empty() {
            errors.push("MODEL_CREDIBILITY_REQUIRED_UNCERTAINTY_EMPTY".to_owned());
        }
        if self.required_metric_strata.is_empty() {
            errors.push("MODEL_CREDIBILITY_REQUIRED_METRIC_STRATA_EMPTY".to_owned());
        }

        let mut required_modalities = BTreeSet::new();
        for modality in &self.required_modalities {
            if !required_modalities.insert(*modality) {
                errors.push(format!("MODEL_CREDIBILITY_DUPLICATE_REQUIRED_MODALITY:{}", modality.as_str()));
            }
        }
        let mut modality_records = BTreeSet::new();
        for record in &self.modality_evidence {
            if !valid_identifier(&record.evidence_ref)
                || !valid_sha256(&record.report_sha256)
                || !valid_sha256(&record.scope_manifest_sha256)
            {
                errors.push("MODEL_CREDIBILITY_MODALITY_EVIDENCE_INVALID".to_owned());
            }
            if !modality_records.insert((record.modality, record.evidence_ref.as_str())) {
                errors.push(format!("MODEL_CREDIBILITY_DUPLICATE_MODALITY_EVIDENCE:{}", record.evidence_ref));
            }
        }

        let mut required_uncertainty = BTreeSet::new();
        for component in &self.required_uncertainty_components {
            if !required_uncertainty.insert(*component) {
                errors.push(format!("MODEL_CREDIBILITY_DUPLICATE_REQUIRED_UNCERTAINTY:{}", component.as_str()));
            }
        }
        let mut uncertainty_records = BTreeSet::new();
        for record in &self.uncertainty_components {
            if !valid_sha256(&record.method_profile_sha256)
                || !valid_sha256(&record.report_sha256)
            {
                errors.push("MODEL_CREDIBILITY_UNCERTAINTY_EVIDENCE_INVALID".to_owned());
            }
            if !uncertainty_records.insert(record.component) {
                errors.push(format!(
                    "MODEL_CREDIBILITY_DUPLICATE_UNCERTAINTY_COMPONENT:{}",
                    record.component.as_str()
                ));
            }
        }

        let mut required_pairs = BTreeSet::new();
        for pair in &self.required_metric_strata {
            if !valid_identifier(&pair.metric_id) || !valid_identifier(&pair.stratum_ref) {
                errors.push("MODEL_CREDIBILITY_REQUIRED_METRIC_STRATUM_INVALID".to_owned());
            }
            if !required_pairs.insert((pair.metric_id.as_str(), pair.stratum_ref.as_str())) {
                errors.push("MODEL_CREDIBILITY_DUPLICATE_REQUIRED_METRIC_STRATUM".to_owned());
            }
        }
        let mut reported_pairs = BTreeSet::new();
        for report in &self.metric_stratum_reports {
            if !valid_identifier(&report.metric_id)
                || !valid_identifier(&report.stratum_ref)
                || !valid_sha256(&report.population_manifest_sha256)
                || !valid_sha256(&report.report_sha256)
                || !valid_sha256(&report.uncertainty_report_sha256)
                || report.sample_count == 0
                || report.adverse_count > report.sample_count
            {
                errors.push(format!(
                    "MODEL_CREDIBILITY_METRIC_REPORT_INVALID:{}:{}",
                    report.metric_id, report.stratum_ref
                ));
            }
            if !reported_pairs.insert((report.metric_id.as_str(), report.stratum_ref.as_str())) {
                errors.push(format!(
                    "MODEL_CREDIBILITY_DUPLICATE_METRIC_REPORT:{}:{}",
                    report.metric_id, report.stratum_ref
                ));
            }
        }

        errors.sort();
        errors
    }

    /// Canonical, order-invariant digest of the exact model/profile evidence cut.
    pub fn compute_sha256(&self) -> Result<String, Vec<String>> {
        let errors = self.validation_errors();
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut required_modalities = self.required_modalities.clone();
        required_modalities.sort();
        let mut modality_evidence = self.modality_evidence.iter().collect::<Vec<_>>();
        modality_evidence.sort_by(|left, right| {
            (left.modality, left.evidence_ref.as_str())
                .cmp(&(right.modality, right.evidence_ref.as_str()))
        });
        let mut required_uncertainty_components = self.required_uncertainty_components.clone();
        required_uncertainty_components.sort();
        let mut uncertainty_components = self.uncertainty_components.iter().collect::<Vec<_>>();
        uncertainty_components.sort_by_key(|item| item.component);
        let mut required_metric_strata = self.required_metric_strata.iter().collect::<Vec<_>>();
        required_metric_strata.sort();
        let mut metric_stratum_reports = self.metric_stratum_reports.iter().collect::<Vec<_>>();
        metric_stratum_reports.sort_by(|left, right| {
            (left.metric_id.as_str(), left.stratum_ref.as_str())
                .cmp(&(right.metric_id.as_str(), right.stratum_ref.as_str()))
        });

        let envelope = ModelCredibilityEnvelope {
            schema: "AeroModelCredibilityCutV0_1",
            canonicalization_profile: "rust-serde-json-struct-order-v1",
            evidence_ref: &self.evidence_ref,
            claim_id: &self.claim_id,
            model_ref: &self.model_ref,
            model_artifact_sha256: &self.model_artifact_sha256,
            solver_configuration_sha256: &self.solver_configuration_sha256,
            configuration_ref: &self.configuration_ref,
            operating_envelope_ref: &self.operating_envelope_ref,
            applicability_profile_sha256: &self.applicability_profile_sha256,
            risk_classification_sha256: &self.risk_classification_sha256,
            validation_package_sha256: &self.validation_package_sha256,
            validation_protocol_sha256: &self.validation_protocol_sha256,
            calibration_manifest_sha256: &self.calibration_manifest_sha256,
            development_manifest_sha256: &self.development_manifest_sha256,
            confirmatory_manifest_sha256: &self.confirmatory_manifest_sha256,
            independent_reference_manifest_sha256: &self.independent_reference_manifest_sha256,
            package_status: self.package_status,
            split_integrity_status: self.split_integrity_status,
            independent_reference_status: self.independent_reference_status,
            domain_coverage_status: self.domain_coverage_status,
            required_modalities,
            modality_evidence,
            required_uncertainty_components,
            uncertainty_components,
            required_metric_strata,
            metric_stratum_reports,
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| vec!["MODEL_CREDIBILITY_SERIALIZATION_FAILED".to_owned()])?;
        Ok(sha256_hex(&bytes))
    }

    /// Claim-specific blockers; no scalar score or modality ladder is computed.
    pub fn reviewability_blockers(&self) -> Vec<String> {
        let mut blockers = BTreeSet::new();
        match self.package_status {
            ModelPackageStatus::StructurallyComplete => {}
            ModelPackageStatus::Incomplete => {
                blockers.insert("MODEL_VALIDATION_PACKAGE_INCOMPLETE".to_owned());
            }
            ModelPackageStatus::Rejected => {
                blockers.insert("MODEL_VALIDATION_PACKAGE_REJECTED".to_owned());
            }
        }
        match self.split_integrity_status {
            SplitIntegrityStatus::DisjointByIndependentUnit => {}
            SplitIntegrityStatus::LeakageDetected => {
                blockers.insert("MODEL_CONFIRMATORY_SPLIT_LEAKAGE".to_owned());
            }
            SplitIntegrityStatus::Unverified => {
                blockers.insert("MODEL_CONFIRMATORY_SPLIT_UNVERIFIED".to_owned());
            }
        }
        match self.independent_reference_status {
            IndependentReferenceStatus::Qualified => {}
            IndependentReferenceStatus::Unverified => {
                blockers.insert("MODEL_INDEPENDENT_REFERENCE_UNVERIFIED".to_owned());
            }
            IndependentReferenceStatus::Absent => {
                blockers.insert("MODEL_INDEPENDENT_REFERENCE_ABSENT".to_owned());
            }
        }
        match self.domain_coverage_status {
            ModelDomainCoverageStatus::Covered => {}
            ModelDomainCoverageStatus::NotCovered => {
                blockers.insert("MODEL_CLAIM_OUTSIDE_VALIDATED_DOMAIN".to_owned());
            }
            ModelDomainCoverageStatus::Indeterminate => {
                blockers.insert("MODEL_DOMAIN_COVERAGE_INDETERMINATE".to_owned());
            }
            ModelDomainCoverageStatus::Unverified => {
                blockers.insert("MODEL_DOMAIN_COVERAGE_UNVERIFIED".to_owned());
            }
        }

        for required in &self.required_modalities {
            let qualified = self.modality_evidence.iter().any(|evidence| {
                evidence.modality == *required
                    && evidence.status == ModalityEvidenceStatus::QualifiedForDeclaredPurpose
            });
            if !qualified {
                blockers.insert(format!(
                    "MODEL_REQUIRED_MODALITY_NOT_QUALIFIED:{}",
                    required.as_str()
                ));
            }
        }
        for required in &self.required_uncertainty_components {
            match self
                .uncertainty_components
                .iter()
                .find(|evidence| evidence.component == *required)
                .map(|evidence| evidence.status)
            {
                Some(UncertaintyEvidenceStatus::Characterized) => {}
                Some(UncertaintyEvidenceStatus::Uncharacterized) => {
                    blockers.insert(format!(
                        "MODEL_UNCERTAINTY_COMPONENT_UNCHARACTERIZED:{}",
                        required.as_str()
                    ));
                }
                Some(UncertaintyEvidenceStatus::Unverified) | None => {
                    blockers.insert(format!(
                        "MODEL_UNCERTAINTY_COMPONENT_UNVERIFIED:{}",
                        required.as_str()
                    ));
                }
            }
        }
        for required in &self.required_metric_strata {
            match self.metric_stratum_reports.iter().find(|report| {
                report.metric_id == required.metric_id && report.stratum_ref == required.stratum_ref
            }) {
                None => {
                    blockers.insert(format!(
                        "MODEL_METRIC_STRATUM_REPORT_MISSING:{}:{}",
                        required.metric_id, required.stratum_ref
                    ));
                }
                Some(report) => match report.status {
                    MetricStratumStatus::WithinDeclaredProfile => {}
                    MetricStratumStatus::OutsideDeclaredProfile => {
                        blockers.insert(format!(
                            "MODEL_METRIC_OUTSIDE_DECLARED_PROFILE:{}:{}",
                            required.metric_id, required.stratum_ref
                        ));
                    }
                    MetricStratumStatus::Indeterminate => {
                        blockers.insert(format!(
                            "MODEL_METRIC_STRATUM_INDETERMINATE:{}:{}",
                            required.metric_id, required.stratum_ref
                        ));
                    }
                    MetricStratumStatus::Unverified => {
                        blockers.insert(format!(
                            "MODEL_METRIC_STRATUM_UNVERIFIED:{}:{}",
                            required.metric_id, required.stratum_ref
                        ));
                    }
                },
            }
        }
        blockers.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IndependentReferenceStatus, MetricStratumReportV0_1, MetricStratumStatus,
        ModelCredibilityCutV0_1, ModelDomainCoverageStatus, ModelPackageStatus,
        ModalityEvidenceStatus, ModalityEvidenceV0_1, RequiredMetricStratumV0_1,
        SplitIntegrityStatus, UncertaintyComponentEvidenceV0_1, UncertaintyComponentKind,
        UncertaintyEvidenceStatus, ValidationModality,
    };

    pub fn fixture() -> ModelCredibilityCutV0_1 {
        let required_modalities = vec![
            ValidationModality::AnalyticReference,
            ValidationModality::CalibratedComponentBench,
            ValidationModality::SoftwareInLoop,
        ];
        let modality_evidence = required_modalities
            .iter()
            .map(|modality| ModalityEvidenceV0_1 {
                modality: *modality,
                evidence_ref: format!("aeroweift:modality:{}", modality.as_str()),
                report_sha256: "1".repeat(64),
                scope_manifest_sha256: "2".repeat(64),
                status: ModalityEvidenceStatus::QualifiedForDeclaredPurpose,
            })
            .collect();
        let required_uncertainty_components = vec![
            UncertaintyComponentKind::ObservationNoise,
            UncertaintyComponentKind::MeasurementUncertainty,
            UncertaintyComponentKind::ParameterUncertainty,
            UncertaintyComponentKind::StructuralModelDiscrepancy,
            UncertaintyComponentKind::EnvironmentalUncertainty,
        ];
        let uncertainty_components = required_uncertainty_components
            .iter()
            .map(|component| UncertaintyComponentEvidenceV0_1 {
                component: *component,
                method_profile_sha256: "3".repeat(64),
                report_sha256: "4".repeat(64),
                status: UncertaintyEvidenceStatus::Characterized,
            })
            .collect();
        let required_metric_strata = vec![
            RequiredMetricStratumV0_1 {
                metric_id: "prediction_interval_coverage".to_owned(),
                stratum_ref: "material:aluminium".to_owned(),
            },
            RequiredMetricStratumV0_1 {
                metric_id: "out_of_domain_recall".to_owned(),
                stratum_ref: "environment:temperature-extremes".to_owned(),
            },
        ];
        let metric_stratum_reports = required_metric_strata
            .iter()
            .map(|required| MetricStratumReportV0_1 {
                metric_id: required.metric_id.clone(),
                stratum_ref: required.stratum_ref.clone(),
                population_manifest_sha256: "5".repeat(64),
                report_sha256: "6".repeat(64),
                uncertainty_report_sha256: "7".repeat(64),
                sample_count: 100,
                adverse_count: 2,
                status: MetricStratumStatus::WithinDeclaredProfile,
            })
            .collect();
        ModelCredibilityCutV0_1 {
            schema: "AeroModelCredibilityCutV0_1".to_owned(),
            evidence_ref: "aeroweift:validation:wing-panel-001".to_owned(),
            claim_id: "claim-wing-panel-001".to_owned(),
            model_ref: "model:aeroweift-wing-structural-v4".to_owned(),
            model_artifact_sha256: "8".repeat(64),
            solver_configuration_sha256: "9".repeat(64),
            configuration_ref: "configuration:wing-r17".to_owned(),
            operating_envelope_ref: "envelope:transport-normal".to_owned(),
            applicability_profile_sha256: "a".repeat(64),
            risk_classification_sha256: "b".repeat(64),
            validation_package_sha256: "c".repeat(64),
            validation_protocol_sha256: "d".repeat(64),
            calibration_manifest_sha256: "e".repeat(64),
            development_manifest_sha256: "f".repeat(64),
            confirmatory_manifest_sha256: "0".repeat(64),
            independent_reference_manifest_sha256: "1".repeat(64),
            package_status: ModelPackageStatus::StructurallyComplete,
            split_integrity_status: SplitIntegrityStatus::DisjointByIndependentUnit,
            independent_reference_status: IndependentReferenceStatus::Qualified,
            domain_coverage_status: ModelDomainCoverageStatus::Covered,
            required_modalities,
            modality_evidence,
            required_uncertainty_components,
            uncertainty_components,
            required_metric_strata,
            metric_stratum_reports,
        }
    }

    #[test]
    fn model_credibility_cut_hash_is_order_invariant() {
        let cut = fixture();
        let mut reversed = cut.clone();
        reversed.required_modalities.reverse();
        reversed.modality_evidence.reverse();
        reversed.required_uncertainty_components.reverse();
        reversed.uncertainty_components.reverse();
        reversed.required_metric_strata.reverse();
        reversed.metric_stratum_reports.reverse();
        assert_eq!(cut.compute_sha256(), reversed.compute_sha256());
    }

    #[test]
    fn every_model_or_validation_reference_changes_the_commitment() {
        let baseline = fixture().compute_sha256().expect("valid model cut");
        let mut cut = fixture();
        cut.model_artifact_sha256 = "9".repeat(64);
        assert_ne!(baseline, cut.compute_sha256().expect("valid changed cut"));
        let mut cut = fixture();
        cut.confirmatory_manifest_sha256 = "9".repeat(64);
        assert_ne!(baseline, cut.compute_sha256().expect("valid changed cut"));
        let mut cut = fixture();
        cut.applicability_profile_sha256 = "9".repeat(64);
        assert_ne!(baseline, cut.compute_sha256().expect("valid changed cut"));
    }

    #[test]
    fn model_credibility_blockers_preserve_domain_and_no_leakage() {
        let mut cut = fixture();
        cut.split_integrity_status = SplitIntegrityStatus::LeakageDetected;
        cut.domain_coverage_status = ModelDomainCoverageStatus::Indeterminate;
        cut.required_modalities.clear();
        cut.modality_evidence.clear();
        let blockers = cut.reviewability_blockers();
        assert!(blockers.contains(&"MODEL_CONFIRMATORY_SPLIT_LEAKAGE".to_owned()));
        assert!(blockers.contains(&"MODEL_DOMAIN_COVERAGE_INDETERMINATE".to_owned()));
        assert!(blockers.contains(
            &"MODEL_REQUIRED_MODALITY_NOT_QUALIFIED:analytic_reference".to_owned()
        ));
    }

    #[test]
    fn missing_uncertainty_or_stratum_report_is_not_silently_averaged() {
        let mut cut = fixture();
        cut.uncertainty_components.clear();
        cut.metric_stratum_reports.remove(0);
        let blockers = cut.reviewability_blockers();
        assert!(blockers.contains(
            &"MODEL_UNCERTAINTY_COMPONENT_UNVERIFIED:observation_noise".to_owned()
        ));
        assert!(blockers.contains(
            &"MODEL_METRIC_STRATUM_REPORT_MISSING:prediction_interval_coverage:material:aluminium".to_owned()
        ));
    }

    #[test]
    fn invalid_metric_denominator_and_duplicate_pairs_reject_cut() {
        let mut zero = fixture();
        zero.metric_stratum_reports[0].sample_count = 0;
        assert!(zero.compute_sha256().is_err());

        let mut duplicate = fixture();
        duplicate.metric_stratum_reports.push(duplicate.metric_stratum_reports[0].clone());
        assert!(duplicate.compute_sha256().is_err());
    }
}

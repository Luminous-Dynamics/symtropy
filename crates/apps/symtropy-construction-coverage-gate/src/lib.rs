// SPDX-License-Identifier: AGPL-3.0-or-later
//! Claim/task-specific admission of a survey owner's control-domain coverage
//! assessment. This is not a geometry engine, survey solver or print controller.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use symtropy_robotics_bridge_core::{
    SurveyCoverageCellDispositionV1, SurveyCoverageCellV1, SurveyControlCoverageMapV1,
    SurveyProcessingDispositionV1, SurveyProcessingStageV1, VerifiedSurveyControlCoverageMapV1,
    VerifiedSurveyCoverageAssessmentV1, VerifiedSurveyEvidenceEnvelopeV1,
    verify_coverage_map_summary, world_contract::ArtifactRef,
};

pub const COVERAGE_GATE_POLICY_SCHEMA_VERSION: u16 = 1;
pub const COVERAGE_GATE_RESULT_SCHEMA_VERSION: u16 = 1;

fn valid_identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_json<T: Serialize>(value: &T) -> Result<String, CoverageGateError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| CoverageGateError::InvalidPolicy("policy_serialization"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn verify_artifact_bytes<T: Serialize>(
    value: &T,
    artifact: &ArtifactRef,
) -> Result<(), CoverageGateError> {
    if !valid_identity(&artifact.artifact_id)
        || !valid_identity(&artifact.uri)
        || !valid_sha256(&artifact.sha256)
    {
        return Err(CoverageGateError::InvalidPolicy("policy_artifact_reference"));
    }
    let actual = digest_json(value)?;
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(CoverageGateError::PolicyDigestMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageTaskBindingV1 {
    pub site_id: String,
    pub design_revision: String,
    pub work_package_revision: String,
    pub task_scope: String,
    pub work_zone_id: String,
    pub worksite_epoch_sha256: String,
    pub target_frame_id: String,
    pub task_footprint_artifact: ArtifactRef,
}

impl CoverageTaskBindingV1 {
    fn validate(&self) -> Result<(), CoverageGateError> {
        for value in [
            self.site_id.as_str(),
            self.design_revision.as_str(),
            self.work_package_revision.as_str(),
            self.task_scope.as_str(),
            self.work_zone_id.as_str(),
            self.target_frame_id.as_str(),
        ] {
            if !valid_identity(value) {
                return Err(CoverageGateError::InvalidPolicy("task_binding_identity"));
            }
        }
        if !valid_sha256(&self.worksite_epoch_sha256)
            || !valid_sha256(&self.task_footprint_artifact.sha256)
            || !valid_identity(&self.task_footprint_artifact.artifact_id)
            || !valid_identity(&self.task_footprint_artifact.uri)
        {
            return Err(CoverageGateError::InvalidPolicy("task_binding_digest"));
        }
        Ok(())
    }
}

/// Trusted project/site policy. The caller must obtain the policy and its
/// ArtifactRef from an independently authenticated authority channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageAdmissionPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub task: CoverageTaskBindingV1,
    pub survey_envelope_artifact: ArtifactRef,
    pub control_domain_artifact: ArtifactRef,
    pub required_algorithm_id: String,
    pub required_algorithm_build_sha256: String,
    pub required_support_domain_method_id: String,
    pub required_uncertainty_method_id: String,
    pub minimum_independent_checkpoints: usize,
    pub maximum_unsupported_area_fraction: f64,
    pub maximum_unmeasured_area_fraction: f64,
    pub maximum_critical_unsupported_area_m2: f64,
    pub maximum_critical_unmeasured_area_m2: f64,
    pub maximum_support_gap_m: f64,
    pub maximum_support_distance_m: f64,
    pub minimum_local_control_density_per_m2: f64,
    pub maximum_local_u95_m: f64,
}

impl CoverageAdmissionPolicyV1 {
    pub fn validate(&self) -> Result<(), CoverageGateError> {
        if self.schema_version != COVERAGE_GATE_POLICY_SCHEMA_VERSION
            || !valid_identity(&self.policy_id)
            || !valid_identity(&self.required_algorithm_id)
            || !valid_identity(&self.required_support_domain_method_id)
            || !valid_identity(&self.required_uncertainty_method_id)
        {
            return Err(CoverageGateError::InvalidPolicy("policy_identity"));
        }
        self.task.validate()?;
        for artifact in [&self.survey_envelope_artifact, &self.control_domain_artifact] {
            if !valid_identity(&artifact.artifact_id)
                || !valid_identity(&artifact.uri)
                || !valid_sha256(&artifact.sha256)
            {
                return Err(CoverageGateError::InvalidPolicy("policy_artifact_binding"));
            }
        }
        if !valid_sha256(&self.required_algorithm_build_sha256) {
            return Err(CoverageGateError::InvalidPolicy("required_algorithm_build_digest"));
        }
        if self.minimum_independent_checkpoints == 0 {
            return Err(CoverageGateError::InvalidPolicy("minimum_checkpoints"));
        }
        let fractions = [
            self.maximum_unsupported_area_fraction,
            self.maximum_unmeasured_area_fraction,
        ];
        if fractions
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(CoverageGateError::InvalidPolicy("area_fraction_threshold"));
        }
        let nonnegative = [
            self.maximum_critical_unsupported_area_m2,
            self.maximum_critical_unmeasured_area_m2,
            self.maximum_support_gap_m,
            self.maximum_support_distance_m,
            self.minimum_local_control_density_per_m2,
            self.maximum_local_u95_m,
        ];
        if nonnegative
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(CoverageGateError::InvalidPolicy("metric_threshold"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageGateError {
    InvalidPolicy(&'static str),
    PolicyDigestMismatch,
    SurveyEnvelopeBindingMismatch,
    MissingControlAdjustmentReceipt,
    InvalidControlAdjustmentReceipt,
    MissingTargetFrame,
    InvalidCoverageAssessment,
    CoverageMapArtifactMismatch,
    CoverageMapInconsistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageDecisionV1 {
    AdmissibleForNamedTask,
    InadmissibleForNamedTask,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageEvaluationV1 {
    pub schema_version: u16,
    pub assessment_id: String,
    pub assessment_sha256: String,
    pub survey_envelope_sha256: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub task_scope: String,
    pub decision: CoverageDecisionV1,
    pub reason_codes: Vec<String>,
    pub footprint_area_m2: f64,
    pub supported_area_m2: f64,
    pub unsupported_area_m2: f64,
    pub unmeasured_area_m2: f64,
    pub unsupported_area_fraction: f64,
    pub unmeasured_area_fraction: f64,
    pub critical_unsupported_area_m2: f64,
    pub critical_unmeasured_area_m2: f64,
    pub maximum_support_gap_m: f64,
    pub maximum_support_distance_m: f64,
    pub minimum_local_control_density_per_m2: f64,
    pub maximum_local_u95_m: f64,
    pub independent_checkpoint_count: usize,
}

#[derive(Default)]
struct Findings {
    inadmissible: BTreeSet<String>,
    indeterminate: BTreeSet<String>,
}

impl Findings {
    fn hard(&mut self, code: &str) {
        self.inadmissible.insert(code.to_owned());
    }

    fn unknown(&mut self, code: &str) {
        self.indeterminate.insert(code.to_owned());
    }

    fn decision(&self) -> CoverageDecisionV1 {
        if !self.inadmissible.is_empty() {
            CoverageDecisionV1::InadmissibleForNamedTask
        } else if !self.indeterminate.is_empty() {
            CoverageDecisionV1::Indeterminate
        } else {
            CoverageDecisionV1::AdmissibleForNamedTask
        }
    }

    fn reasons(&self) -> Vec<String> {
        self.inadmissible
            .iter()
            .chain(self.indeterminate.iter())
            .cloned()
            .collect()
    }
}

/// Evaluate one exact task footprint against a verified survey envelope,
/// a verified survey coverage assessment, and a separately supplied policy.
/// The policy artifact digest is checked; publisher authentication remains the
/// responsibility of the caller's trusted project/site authority channel.
pub fn evaluate_coverage(
    assessment: &VerifiedSurveyCoverageAssessmentV1,
    support_map: &VerifiedSurveyControlCoverageMapV1,
    survey: &VerifiedSurveyEvidenceEnvelopeV1,
    policy: &CoverageAdmissionPolicyV1,
    policy_artifact: &ArtifactRef,
) -> Result<CoverageEvaluationV1, CoverageGateError> {
    policy.validate()?;
    verify_artifact_bytes(policy, policy_artifact)?;

    let assessment_data = &assessment.assessment;
    assessment_data
        .validate()
        .map_err(|_| CoverageGateError::InvalidCoverageAssessment)?;
    if support_map.source_artifact() != &assessment_data.support_map_artifact {
        return Err(CoverageGateError::CoverageMapArtifactMismatch);
    }
    verify_coverage_map_summary(assessment_data, &support_map.coverage_map)
        .map_err(|_| CoverageGateError::CoverageMapInconsistent)?;
    survey
        .envelope
        .validate()
        .map_err(|_| CoverageGateError::InvalidCoverageAssessment)?;

    if survey.source_artifact() != &assessment_data.survey_envelope_artifact
        || survey.source_artifact() != &policy.survey_envelope_artifact
    {
        return Err(CoverageGateError::SurveyEnvelopeBindingMismatch);
    }

    let receipt = survey
        .envelope
        .processing_receipts
        .iter()
        .find(|receipt| {
            receipt
                .verify_artifact(&assessment_data.control_adjustment_receipt_artifact)
                .is_ok()
        })
        .ok_or(CoverageGateError::MissingControlAdjustmentReceipt)?;

    if receipt.stage != SurveyProcessingStageV1::AdjustedControlSolution
        || receipt.disposition != SurveyProcessingDispositionV1::Qualified
        || receipt.output_artifact.as_ref() != Some(&assessment_data.control_domain_artifact)
        || assessment_data.control_domain_artifact != policy.control_domain_artifact
    {
        return Err(CoverageGateError::InvalidControlAdjustmentReceipt);
    }
    receipt
        .validate_against(&survey.envelope.observations, &survey.envelope.frames)
        .map_err(|_| CoverageGateError::InvalidControlAdjustmentReceipt)?;

    if !survey
        .envelope
        .frames
        .iter()
        .any(|frame| frame.frame_id == assessment_data.target_frame_id)
    {
        return Err(CoverageGateError::MissingTargetFrame);
    }

    let binding = &policy.task;
    let mut findings = Findings::default();
    for (actual, expected, reason) in [
        (assessment_data.site_id.as_str(), binding.site_id.as_str(), "site_binding_mismatch"),
        (
            assessment_data.design_revision.as_str(),
            binding.design_revision.as_str(),
            "design_revision_mismatch",
        ),
        (
            assessment_data.work_package_revision.as_str(),
            binding.work_package_revision.as_str(),
            "work_package_revision_mismatch",
        ),
        (assessment_data.task_scope.as_str(), binding.task_scope.as_str(), "task_scope_mismatch"),
        (
            assessment_data.work_zone_id.as_str(),
            binding.work_zone_id.as_str(),
            "work_zone_mismatch",
        ),
        (
            assessment_data.worksite_epoch_sha256.as_str(),
            binding.worksite_epoch_sha256.as_str(),
            "worksite_epoch_mismatch",
        ),
        (
            assessment_data.target_frame_id.as_str(),
            binding.target_frame_id.as_str(),
            "target_frame_mismatch",
        ),
    ] {
        if actual != expected {
            findings.hard(reason);
        }
    }
    if assessment_data.task_footprint_artifact != binding.task_footprint_artifact {
        findings.hard("task_footprint_artifact_mismatch");
    }
    if assessment_data.algorithm_id != policy.required_algorithm_id
        || assessment_data.algorithm_build_sha256 != policy.required_algorithm_build_sha256
    {
        findings.hard("coverage_algorithm_not_approved_by_policy");
    }
    if assessment_data.support_domain_method_id != policy.required_support_domain_method_id {
        findings.hard("support_domain_method_mismatch");
    }
    if assessment_data.uncertainty_method_id != policy.required_uncertainty_method_id {
        findings.hard("uncertainty_method_mismatch");
    }

    let checkpoint_count = receipt.independent_checkpoint_ids.len();
    if checkpoint_count < policy.minimum_independent_checkpoints {
        findings.unknown("independent_checkpoint_count_below_policy");
    }

    let metrics = assessment_data;
    let unsupported_fraction = metrics.unsupported_area_m2 / metrics.footprint_area_m2;
    let unmeasured_fraction = metrics.unmeasured_area_m2 / metrics.footprint_area_m2;

    if unsupported_fraction > policy.maximum_unsupported_area_fraction {
        findings.hard("unsupported_area_fraction_exceeds_policy");
    }
    if unmeasured_fraction > policy.maximum_unmeasured_area_fraction {
        findings.unknown("unmeasured_area_fraction_exceeds_policy");
    }
    if metrics.critical_unsupported_area_m2 > policy.maximum_critical_unsupported_area_m2 {
        findings.hard("critical_unsupported_area_exceeds_policy");
    }
    if metrics.critical_unmeasured_area_m2 > policy.maximum_critical_unmeasured_area_m2 {
        findings.unknown("critical_unmeasured_area_exceeds_policy");
    }
    if metrics.maximum_support_gap_m > policy.maximum_support_gap_m {
        findings.hard("maximum_support_gap_exceeds_policy");
    }
    if metrics.maximum_support_distance_m > policy.maximum_support_distance_m {
        findings.hard("maximum_support_distance_exceeds_policy");
    }
    if metrics.minimum_local_control_density_per_m2 < policy.minimum_local_control_density_per_m2 {
        findings.hard("minimum_local_control_density_below_policy");
    }
    if metrics.maximum_local_u95_m > policy.maximum_local_u95_m {
        findings.hard("maximum_local_u95_exceeds_policy");
    }

    Ok(CoverageEvaluationV1 {
        schema_version: COVERAGE_GATE_RESULT_SCHEMA_VERSION,
        assessment_id: metrics.assessment_id.clone(),
        assessment_sha256: assessment.source_artifact().sha256.clone(),
        survey_envelope_sha256: survey.source_artifact().sha256.clone(),
        policy_id: policy.policy_id.clone(),
        policy_sha256: policy_artifact.sha256.clone(),
        task_scope: binding.task_scope.clone(),
        decision: findings.decision(),
        reason_codes: findings.reasons(),
        footprint_area_m2: metrics.footprint_area_m2,
        supported_area_m2: metrics.supported_area_m2,
        unsupported_area_m2: metrics.unsupported_area_m2,
        unmeasured_area_m2: metrics.unmeasured_area_m2,
        unsupported_area_fraction: unsupported_fraction,
        unmeasured_area_fraction: unmeasured_fraction,
        critical_unsupported_area_m2: metrics.critical_unsupported_area_m2,
        critical_unmeasured_area_m2: metrics.critical_unmeasured_area_m2,
        maximum_support_gap_m: metrics.maximum_support_gap_m,
        maximum_support_distance_m: metrics.maximum_support_distance_m,
        minimum_local_control_density_per_m2: metrics.minimum_local_control_density_per_m2,
        maximum_local_u95_m: metrics.maximum_local_u95_m,
        independent_checkpoint_count: checkpoint_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_robotics_bridge_core::{
        SurveyProcessingReceiptV1, SurveyProcessingStageV1,
        SurveyProcessingDispositionV1, SurveyEvidenceEnvelopeV1, SurveyCoordinateFrameV1,
        SurveyCoordinateUnitV1, SurveyAxisDirectionV1, SurveyHandednessV1, SurveyHeightTypeV1,
        SurveySessionV1, SurveyObservationV1, SurveyModalityV1, SurveyObservationRoleV1,
        SurveyUncertaintyV1, SurveyCoverageAssessmentV1, SurveyCoverageCellDispositionV1,
        SurveyCoverageCellV1, SurveyControlCoverageMapV1, SurveyControlCoverageMapV1,
        SURVEY_COVERAGE_ASSESSMENT_SCHEMA_VERSION, VerifiedSurveyCoverageAssessmentV1,
        VerifiedSurveyControlCoverageMapV1, VerifiedSurveyEvidenceEnvelopeV1,
        SURVEY_EVIDENCE_SCHEMA_VERSION, SURVEY_CONTROL_COVERAGE_MAP_SCHEMA_VERSION,
    };

    fn artifact(id: &str, bytes: &[u8]) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("coverage-test://{id}"),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    fn adjustment_receipt(
        footprint: &ArtifactRef,
        domain: &ArtifactRef,
    ) -> SurveyProcessingReceiptV1 {
        SurveyProcessingReceiptV1 {
            schema_version: SURVEY_EVIDENCE_SCHEMA_VERSION,
            receipt_id: "adjustment-1".to_owned(),
            stage: SurveyProcessingStageV1::AdjustedControlSolution,
            disposition: SurveyProcessingDispositionV1::Qualified,
            input_artifacts: vec![footprint.clone()],
            output_artifact: Some(domain.clone()),
            algorithm_id: "control-adjustment".to_owned(),
            algorithm_version: "1.0".to_owned(),
            algorithm_build_sha256: "d".repeat(64),
            configuration_sha256: "e".repeat(64),
            source_frame_ids: vec!["site-grid-2026".to_owned()],
            output_frame_id: Some("site-grid-2026".to_owned()),
            fitting_observation_ids: vec!["fit-1".to_owned()],
            independent_checkpoint_ids: vec!["checkpoint-1".to_owned()],
            excluded_observation_ids: vec![],
            accuracy_metrics: Some(symtropy_robotics_bridge_core::SurveyAccuracyMetricsV1 {
                horizontal_rmse_m: 0.001,
                vertical_rmse_m: 0.002,
                detection_limit_m: None,
                estimated_changed_volume_m3: None,
                changed_volume_standard_uncertainty_m3: None,
                uncertainty_method_id: "held-out-checkpoint-v1".to_owned(),
            }),
        }
    }

    fn serialized_artifact<T: Serialize>(id: &str, value: &T) -> ArtifactRef {
        let bytes = serde_json::to_vec(value).unwrap();
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("coverage-test://{id}"),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        }
    }

    fn survey_envelope_value(
        footprint: &ArtifactRef,
        domain: &ArtifactRef,
    ) -> SurveyEvidenceEnvelopeV1 {
        let fit = SurveyObservationV1 {
            schema_version: SURVEY_EVIDENCE_SCHEMA_VERSION,
            observation_id: "fit-1".to_owned(),
            session_id: "session-1".to_owned(),
            source_id: "total-station-1".to_owned(),
            sequence: 1,
            captured_at_unix_ns: 150,
            modality: SurveyModalityV1::TotalStation,
            role: SurveyObservationRoleV1::FitOrCalibration,
            frame_id: "site-grid-2026".to_owned(),
            payload_artifact: artifact("fit-observation", b"fit bytes"),
            calibration_artifact: artifact("calibration", b"calibration bytes"),
            coordinate: Some([0.0, 0.0, 0.0]),
            uncertainty: Some(SurveyUncertaintyV1::StandardDeviation {
                standard_deviation: [0.001, 0.001, 0.002],
            }),
            uncertainty_artifact: None,
            quality_flags: vec![],
        };
        let checkpoint = SurveyObservationV1 {
            observation_id: "checkpoint-1".to_owned(),
            role: SurveyObservationRoleV1::IndependentCheckpoint,
            sequence: 2,
            captured_at_unix_ns: 160,
            payload_artifact: artifact("checkpoint", b"checkpoint bytes"),
            ..fit.clone()
        };
        let source_receipt = adjustment_receipt(footprint, domain);
        SurveyEvidenceEnvelopeV1 {
            schema_version: SURVEY_EVIDENCE_SCHEMA_VERSION,
            frames: vec![frame()],
            sessions: vec![SurveySessionV1 {
                schema_version: SURVEY_EVIDENCE_SCHEMA_VERSION,
                session_id: "session-1".to_owned(),
                instrument_id: "total-station-1".to_owned(),
                instrument_profile_artifact: artifact("instrument-profile", b"profile"),
                calibration_artifact: artifact("calibration", b"calibration bytes"),
                instrument_frame_id: "site-grid-2026".to_owned(),
                started_at_unix_ns: 100,
                ended_at_unix_ns: Some(200),
                clock_uncertainty_ns: 1,
                raw_observation_artifacts: vec![artifact("raw", b"raw")],
            }],
            observations: vec![fit, checkpoint],
            transforms: vec![],
            processing_receipts: vec![source_receipt],
            change_candidates: vec![],
        }
    }

    fn assessment() -> SurveyCoverageAssessmentV1 {
        let footprint = artifact("task-footprint", b"task polygon bytes");
        let domain = artifact("control-domain", b"qualified support domain bytes");
        let support = artifact("support-map", b"placeholder support-map bytes");
        let receipt = adjustment_receipt(&footprint, &domain);
        let receipt_artifact = serialized_artifact("control-adjustment", &receipt);
        let envelope_value = survey_envelope_value(&footprint, &domain);
        let (_, envelope_artifact) = envelope_value
            .encode_with_digest("survey-envelope", "coverage-test://survey-envelope")
            .unwrap();

        let mut assessment = SurveyCoverageAssessmentV1 {
            schema_version: 1,
            assessment_id: "coverage-assessment-001".to_owned(),
            site_id: "site-001".to_owned(),
            design_revision: "design-r7".to_owned(),
            work_package_revision: "wp-r3".to_owned(),
            task_scope: "level-2-layout".to_owned(),
            work_zone_id: "zone-c".to_owned(),
            worksite_epoch_sha256: "a".repeat(64),
            target_frame_id: "site-grid-2026".to_owned(),
            survey_envelope_artifact: envelope_artifact,
            control_adjustment_receipt_artifact: receipt_artifact,
            task_footprint_artifact: footprint,
            control_domain_artifact: domain,
            support_map_artifact: support,
            algorithm_id: "support-analyzer".to_owned(),
            algorithm_version: "1.2.0".to_owned(),
            algorithm_build_sha256: "b".repeat(64),
            configuration_sha256: "c".repeat(64),
            support_domain_method_id: "support-tiles-v1".to_owned(),
            uncertainty_method_id: "local-u95-v1".to_owned(),
            footprint_area_m2: 100.0,
            supported_area_m2: 100.0,
            unsupported_area_m2: 0.0,
            unmeasured_area_m2: 0.0,
            critical_unsupported_area_m2: 0.0,
            critical_unmeasured_area_m2: 0.0,
            maximum_support_gap_m: 2.0,
            maximum_support_distance_m: 1.0,
            minimum_local_control_density_per_m2: 0.08,
            maximum_local_u95_m: 0.004,
            total_cell_count: 100,
            supported_cell_count: 100,
            unsupported_cell_count: 0,
            unmeasured_cell_count: 0,
            critical_cell_count: 20,
            critical_supported_cell_count: 20,
            critical_unsupported_cell_count: 0,
            critical_unmeasured_cell_count: 0,
        };
        let map = support_map_value(&assessment);
        assessment.support_map_artifact = serialized_artifact("support-map", &map);
        assessment
    }

    fn frame() -> SurveyCoordinateFrameV1 {
        SurveyCoordinateFrameV1 {
            frame_id: "site-grid-2026".to_owned(),
            crs_id: "LOCAL:site-grid-2026".to_owned(),
            horizontal_datum_id: "LOCAL:survey-control".to_owned(),
            vertical_reference_id: "LOCAL:building-level".to_owned(),
            height_type: SurveyHeightTypeV1::LocalVertical,
            axis_directions: [
                SurveyAxisDirectionV1::LocalX,
                SurveyAxisDirectionV1::LocalY,
                SurveyAxisDirectionV1::LocalZ,
            ],
            axis_units: [
                SurveyCoordinateUnitV1::Metre,
                SurveyCoordinateUnitV1::Metre,
                SurveyCoordinateUnitV1::Metre,
            ],
            handedness: SurveyHandednessV1::RightHanded,
            coordinate_epoch_year: None,
        }
    }

    fn survey_envelope(assess: &SurveyCoverageAssessmentV1) -> VerifiedSurveyEvidenceEnvelopeV1 {
        let envelope_value = survey_envelope_value(
            &assess.task_footprint_artifact,
            &assess.control_domain_artifact,
        );
        let (bytes, reference) = envelope_value
            .encode_with_digest("survey-envelope", "coverage-test://survey-envelope")
            .unwrap();
        assert_eq!(reference, assess.survey_envelope_artifact);
        VerifiedSurveyEvidenceEnvelopeV1::decode_verified_json(&bytes, reference).unwrap()
    }

    fn policy(assess: &SurveyCoverageAssessmentV1) -> CoverageAdmissionPolicyV1 {
        CoverageAdmissionPolicyV1 {
            schema_version: 1,
            policy_id: "site-coverage-policy-v1".to_owned(),
            task: CoverageTaskBindingV1 {
                site_id: assess.site_id.clone(),
                design_revision: assess.design_revision.clone(),
                work_package_revision: assess.work_package_revision.clone(),
                task_scope: assess.task_scope.clone(),
                work_zone_id: assess.work_zone_id.clone(),
                worksite_epoch_sha256: assess.worksite_epoch_sha256.clone(),
                target_frame_id: assess.target_frame_id.clone(),
                task_footprint_artifact: assess.task_footprint_artifact.clone(),
            },
            survey_envelope_artifact: assess.survey_envelope_artifact.clone(),
            control_domain_artifact: assess.control_domain_artifact.clone(),
            required_algorithm_id: assess.algorithm_id.clone(),
            required_algorithm_build_sha256: assess.algorithm_build_sha256.clone(),
            required_support_domain_method_id: assess.support_domain_method_id.clone(),
            required_uncertainty_method_id: assess.uncertainty_method_id.clone(),
            minimum_independent_checkpoints: 1,
            maximum_unsupported_area_fraction: 0.01,
            maximum_unmeasured_area_fraction: 0.005,
            maximum_critical_unsupported_area_m2: 0.0,
            maximum_critical_unmeasured_area_m2: 0.0,
            maximum_support_gap_m: 3.0,
            maximum_support_distance_m: 2.0,
            minimum_local_control_density_per_m2: 0.05,
            maximum_local_u95_m: 0.01,
        }
    }

    fn policy_artifact(policy: &CoverageAdmissionPolicyV1) -> ArtifactRef {
        let bytes = serde_json::to_vec(policy).unwrap();
        ArtifactRef {
            artifact_id: "site-coverage-policy-v1".to_owned(),
            uri: "coverage-test://policy".to_owned(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    fn run(
        assessment: &SurveyCoverageAssessmentV1,
        policy: &CoverageAdmissionPolicyV1,
    ) -> CoverageEvaluationV1 {
        let mut assessment = assessment.clone();
        let support_map = support_map_value(&assessment);
        assessment.support_map_artifact = serialized_artifact("support-map", &support_map);
        let support_map_bytes = serde_json::to_vec(&support_map).unwrap();
        let support_map_artifact = ArtifactRef {
            artifact_id: "support-map".to_owned(),
            uri: "coverage-test://support-map".to_owned(),
            sha256: format!("{:x}", Sha256::digest(&support_map_bytes)),
        };
        let verified_support_map =
            VerifiedSurveyControlCoverageMapV1::decode_verified_json(
                &support_map_bytes,
                support_map_artifact,
            )
            .unwrap();
        let bytes = serde_json::to_vec(&assessment).unwrap();
        let assessment_artifact = ArtifactRef {
            artifact_id: "coverage-assessment".to_owned(),
            uri: "coverage-test://assessment".to_owned(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        };
        let verified_assessment =
            VerifiedSurveyCoverageAssessmentV1::decode_verified_json(&bytes, assessment_artifact)
                .unwrap();
        evaluate_coverage(
            &verified_assessment,
            &verified_support_map,
            &survey_envelope(&assessment),
            policy,
            &policy_artifact(policy),
        ).unwrap()
    }

    #[test]
    fn complete_coverage_for_exact_task_is_admissible() {
        let assessment = assessment();
        let policy = policy(&assessment);
        let result = run(&assessment, &policy);
        assert_eq!(result.decision, CoverageDecisionV1::AdmissibleForNamedTask);
        assert!(result.reason_codes.is_empty());
    }

    #[test]
    fn large_interior_support_gap_fails_even_when_footprint_area_is_supported() {
        let mut assessment = assessment();
        assessment.maximum_support_gap_m = 6.0;
        let result = run(&assessment, &policy(&assessment));
        assert_eq!(result.decision, CoverageDecisionV1::InadmissibleForNamedTask);
        assert!(result.reason_codes.contains(&"maximum_support_gap_exceeds_policy".to_owned()));
    }

    #[test]
    fn critical_region_outside_support_domain_is_inadmissible() {
        let mut assessment = assessment();
        assessment.critical_unsupported_area_m2 = 0.25;
        assessment.unsupported_area_m2 = 0.25;
        assessment.supported_area_m2 = 99.75;
        assessment.total_cell_count = 100;
        assessment.supported_cell_count = 99;
        assessment.unsupported_cell_count = 1;
        assessment.critical_cell_count = 20;
        assessment.critical_supported_cell_count = 19;
        assessment.critical_unsupported_cell_count = 1;
        let result = run(&assessment, &policy(&assessment));
        assert_eq!(result.decision, CoverageDecisionV1::InadmissibleForNamedTask);
        assert!(
            result
                .reason_codes
                .contains(&"critical_unsupported_area_exceeds_policy".to_owned())
        );
    }

    #[test]
    fn unmeasured_critical_region_is_indeterminate_not_clear() {
        let mut assessment = assessment();
        assessment.critical_unmeasured_area_m2 = 0.25;
        assessment.unmeasured_area_m2 = 0.25;
        assessment.supported_area_m2 = 99.75;
        assessment.total_cell_count = 100;
        assessment.supported_cell_count = 99;
        assessment.unmeasured_cell_count = 1;
        assessment.critical_cell_count = 20;
        assessment.critical_supported_cell_count = 19;
        assessment.critical_unmeasured_cell_count = 1;
        let result = run(&assessment, &policy(&assessment));
        assert_eq!(result.decision, CoverageDecisionV1::Indeterminate);
        assert!(
            result
                .reason_codes
                .contains(&"critical_unmeasured_area_exceeds_policy".to_owned())
        );
    }

    #[test]
    fn frame_epoch_and_footprint_mismatch_cannot_be_promoted() {
        let assessment = assessment();
        let mut policy = policy(&assessment);
        policy.task.target_frame_id = "wrong-frame".to_owned();
        policy.task.worksite_epoch_sha256 = "f".repeat(64);
        policy.task.task_footprint_artifact = artifact("other-footprint", b"different footprint");
        let result = run(&assessment, &policy);
        assert_eq!(result.decision, CoverageDecisionV1::InadmissibleForNamedTask);
        assert!(result.reason_codes.contains(&"target_frame_mismatch".to_owned()));
        assert!(result.reason_codes.contains(&"worksite_epoch_mismatch".to_owned()));
        assert!(result.reason_codes.contains(&"task_footprint_artifact_mismatch".to_owned()));
    }

    #[test]
    fn insufficient_local_density_is_not_hidden_by_global_area() {
        let mut assessment = assessment();
        assessment.minimum_local_control_density_per_m2 = 0.001;
        let result = run(&assessment, &policy(&assessment));
        assert_eq!(result.decision, CoverageDecisionV1::InadmissibleForNamedTask);
        assert!(
            result
                .reason_codes
                .contains(&"minimum_local_control_density_below_policy".to_owned())
        );
    }

    #[test]
    fn local_u95_is_checked_separately_from_gap_and_density() {
        let mut assessment = assessment();
        assessment.maximum_local_u95_m = 0.04;
        let result = run(&assessment, &policy(&assessment));
        assert_eq!(result.decision, CoverageDecisionV1::InadmissibleForNamedTask);
        assert!(result.reason_codes.contains(&"maximum_local_u95_exceeds_policy".to_owned()));
    }

    #[test]
    fn packet_cannot_self_award_admission_decision() {
        let assessment = assessment();
        let mut value = serde_json::to_value(&assessment).unwrap();
        value["decision"] = serde_json::json!("admissible_for_named_task");
        let bytes = serde_json::to_vec(&value).unwrap();
        let artifact = ArtifactRef {
            artifact_id: "coverage-assessment".to_owned(),
            uri: "coverage-test://assessment".to_owned(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        };
        assert!(
            VerifiedSurveyCoverageAssessmentV1::decode_verified_json(&bytes, artifact).is_err()
        );
    }

    #[test]
    fn caller_cannot_change_thresholds_without_matching_policy_artifact() {
        let assessment = assessment();
        let mut policy = policy(&assessment);
        policy.maximum_support_gap_m = 1000.0;
        let policy_artifact = ArtifactRef {
            artifact_id: "stale-policy".to_owned(),
            uri: "coverage-test://stale-policy".to_owned(),
            sha256: "0".repeat(64),
        };
        let bytes = serde_json::to_vec(&assessment).unwrap();
        let assessment_ref = ArtifactRef {
            artifact_id: "coverage-assessment".to_owned(),
            uri: "coverage-test://assessment".to_owned(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        };
        let verified_assessment =
            VerifiedSurveyCoverageAssessmentV1::decode_verified_json(&bytes, assessment_ref)
                .unwrap();
        let verified_support_map = verified_support_map(&assessment);
        assert_eq!(
            evaluate_coverage(
                &verified_assessment,
                &verified_support_map,
                &survey_envelope(&assessment),
                &policy,
                &policy_artifact,
            ),
            Err(CoverageGateError::PolicyDigestMismatch)
        );
    }
}

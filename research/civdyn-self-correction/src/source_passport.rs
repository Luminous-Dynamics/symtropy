// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source passports and cross-domain compatibility gates for CIVDYN empirical
//! intake. This module validates declared rights, acquisition, lineage, time,
//! geography and observation semantics. It does not fetch or authenticate data.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use crate::model_adequacy::IndependentUnitKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionStatus {
    CandidateAvailable,
    AcquisitionPending,
    Acquired,
}

impl AcquisitionStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CandidateAvailable => "candidate_available",
            Self::AcquisitionPending => "acquisition_pending",
            Self::Acquired => "acquired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightsStatus {
    Permitted,
    Restricted,
    Prohibited,
    Unknown,
}

impl RightsStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Permitted => "permitted",
            Self::Restricted => "restricted",
            Self::Prohibited => "prohibited",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationOrigin {
    DirectMeasurement,
    AdministrativeSubmission,
    SurveyEstimate,
    DerivedFromRegisteredInputs,
    ExternalStudyEstimate,
    CategoryDefault,
    Imputed,
    Unknown,
}

impl ObservationOrigin {
    pub const fn name(self) -> &'static str {
        match self {
            Self::DirectMeasurement => "direct_measurement",
            Self::AdministrativeSubmission => "administrative_submission",
            Self::SurveyEstimate => "survey_estimate",
            Self::DerivedFromRegisteredInputs => "derived_from_registered_inputs",
            Self::ExternalStudyEstimate => "external_study_estimate",
            Self::CategoryDefault => "category_default",
            Self::Imputed => "imputed",
            Self::Unknown => "unknown",
        }
    }

    const fn requires_parent_lineage(self) -> bool {
        matches!(self, Self::DerivedFromRegisteredInputs | Self::Imputed)
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalResolution {
    Hour,
    Day,
    Month,
    Quarter,
    FinancialYear,
    CalendarYear,
    SurveyWave,
    IrregularAssessment,
}

impl TemporalResolution {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::FinancialYear => "financial_year",
            Self::CalendarYear => "calendar_year",
            Self::SurveyWave => "survey_wave",
            Self::IrregularAssessment => "irregular_assessment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalSupport {
    /// Integer tick on the explicitly declared time basis.
    pub start_inclusive: i64,
    /// Exclusive end tick; intervals are [start, end).
    pub end_exclusive: i64,
    pub time_basis_id: String,
    pub resolution: TemporalResolution,
}

impl TemporalSupport {
    pub fn validate(&self) -> Result<(), SourcePassportError> {
        if self.start_inclusive >= self.end_exclusive || self.time_basis_id.trim().is_empty() {
            return Err(SourcePassportError::InvalidTemporalSupport);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeographySupport {
    pub geography_id: String,
    pub geography_kind: String,
    pub boundary_version: String,
    pub boundary_digest: String,
}

impl GeographySupport {
    pub fn validate(&self) -> Result<(), SourcePassportError> {
        if self.geography_id.trim().is_empty()
            || self.geography_kind.trim().is_empty()
            || self.boundary_version.trim().is_empty()
            || self.boundary_digest.trim().is_empty()
        {
            return Err(SourcePassportError::InvalidGeographySupport);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifactPassport {
    pub artifact_id: String,
    pub publisher: String,
    pub dataset_title: String,
    pub dataset_version: String,
    pub source_family: String,
    pub landing_page: String,
    pub acquisition_locator: String,
    pub acquisition_status: AcquisitionStatus,
    pub retrieved_at_epoch: Option<i64>,
    /// Digest of exact raw artifact bytes; must be computed by intake.
    pub artifact_digest: Option<String>,
    /// Digest of the deterministic transform/normalization path.
    pub transformation_digest: Option<String>,
    pub terms_digest: Option<String>,
    pub rights_status: RightsStatus,
    pub permitted_purposes: Vec<String>,
    pub restriction_receipt_digest: Option<String>,
    pub source_unit: String,
    pub target_population: String,
    pub population_exclusions: String,
    pub geographic_coverage: String,
    pub native_measurement_unit: String,
    pub temporal_support: TemporalSupport,
    pub geography: GeographySupport,
    pub sampling_unit: IndependentUnitKind,
    pub sample_design_digest: Option<String>,
    pub weights_digest: Option<String>,
    /// Release/verification state of the publisher's snapshot, if supplied.
    pub release_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationPassport {
    pub observation_id: String,
    pub artifact_id: String,
    pub artifact_digest: String,
    pub variable_id: String,
    pub construct: String,
    pub measurement_unit: String,
    pub origin: ObservationOrigin,
    pub source_method_id: String,
    pub temporal_support: TemporalSupport,
    pub geography: GeographySupport,
    /// Analytical reporting unit (e.g. province, municipality or region).
    pub analytical_unit: IndependentUnitKind,
    /// Actual sampling/reporting unit (e.g. household or municipal submission).
    pub sampling_unit: IndependentUnitKind,
    pub sample_size: Option<u64>,
    pub sample_design_digest: Option<String>,
    pub weights_digest: Option<String>,
    pub parent_observation_digests: Vec<String>,
    pub transformation_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisteredPurpose {
    ScenarioAssumption,
    EmpiricalValidation,
}

impl RegisteredPurpose {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ScenarioAssumption => "scenario_assumption",
            Self::EmpiricalValidation => "empirical_validation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceQualificationStatus {
    CandidateAvailable,
    AcquisitionPending,
    QualifiedForRegisteredUse,
    RightsBlocked,
    NotAssessed,
}

impl SourceQualificationStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CandidateAvailable => "candidate_available",
            Self::AcquisitionPending => "acquisition_pending",
            Self::QualifiedForRegisteredUse => "qualified_for_registered_use",
            Self::RightsBlocked => "rights_blocked",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    Compatible,
    CompatibleWithExplicitTransform,
    NotComparable,
    RightsBlocked,
    NotAssessed,
}

impl CompatibilityStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::CompatibleWithExplicitTransform => "compatible_with_explicit_transform",
            Self::NotComparable => "not_comparable",
            Self::RightsBlocked => "rights_blocked",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityReason {
    MissingSourceArtifact,
    AcquisitionNotComplete,
    SourceMetadataIncomplete,
    RightsUnknown,
    RightsProhibited,
    RestrictedPurposeNotAuthorized,
    ArtifactOrTransformDigestMissing,
    ObservationOriginUnknown,
    ObservationSourceMismatch,
    ObservationMetadataIncomplete,
    SurveyDesignOrWeightsMissing,
    DerivedLineageMissing,
    DifferentVariableOrConstruct,
    DifferentMeasurementUnit,
    TemporalSupportMismatch,
    GeographyMismatch,
    AnalyticalUnitMismatch,
    CategoryDefaultNotEligibleForEmpiricalValidation,
    ImputedObservationNotEligibleForEmpiricalValidation,
    ExplicitTransformUsed,
    TransformReceiptMissingOrInvalid,
    TransformCoverageBelowThreshold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceQualification {
    pub artifact_id: String,
    pub status: SourceQualificationStatus,
    pub reason: Option<CompatibilityReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformReceipt {
    pub transform_id: String,
    pub transform_kind: TransformKind,
    pub input_artifact_digest: String,
    pub method_id: String,
    pub receipt_digest: String,
    pub target_period: Option<TemporalSupport>,
    pub target_geography: Option<GeographySupport>,
    pub target_measurement_unit: Option<String>,
    pub target_analytical_unit: Option<IndependentUnitKind>,
    pub coverage_fraction: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    TemporalAggregation,
    SpatialCrosswalk,
    UnitConversion,
    AnalyticalUnitAggregation,
}

impl TransformKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::TemporalAggregation => "temporal_aggregation",
            Self::SpatialCrosswalk => "spatial_crosswalk",
            Self::UnitConversion => "unit_conversion",
            Self::AnalyticalUnitAggregation => "analytical_unit_aggregation",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinAlignmentPlan {
    pub target_period: TemporalSupport,
    pub target_geography: GeographySupport,
    pub target_measurement_unit: String,
    pub target_analytical_unit: IndependentUnitKind,
    pub minimum_transform_coverage: f64,
    pub transforms: Vec<TransformReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibilityReport {
    pub left_observation_id: String,
    pub right_observation_id: String,
    pub purpose: RegisteredPurpose,
    pub status: CompatibilityStatus,
    pub reasons: Vec<CompatibilityReason>,
    pub left_artifact_digest: String,
    pub right_artifact_digest: String,
    pub explicit_transform_digests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcePassportError {
    InvalidTemporalSupport,
    InvalidGeographySupport,
    MissingArtifactIdentity,
    MissingObservationIdentity,
    InvalidAcquisitionMetadata,
    DuplicateParentDigest,
    InvalidTransformReceipt,
}

impl fmt::Display for SourcePassportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SourcePassportError {}

pub fn qualify_source(
    passport: &SourceArtifactPassport,
    purpose: RegisteredPurpose,
) -> SourceQualification {
    let result = |status, reason| SourceQualification {
        artifact_id: passport.artifact_id.clone(),
        status,
        reason,
    };

    if passport.artifact_id.trim().is_empty()
        || passport.publisher.trim().is_empty()
        || passport.dataset_title.trim().is_empty()
        || passport.dataset_version.trim().is_empty()
        || passport.source_family.trim().is_empty()
        || passport.landing_page.trim().is_empty()
        || passport.acquisition_locator.trim().is_empty()
        || passport.source_unit.trim().is_empty()
        || passport.native_measurement_unit.trim().is_empty()
        || passport.release_status.trim().is_empty()
        || passport.temporal_support.validate().is_err()
        || passport.geography.validate().is_err()
    {
        return result(
            SourceQualificationStatus::NotAssessed,
            Some(CompatibilityReason::SourceMetadataIncomplete),
        );
    }

    match passport.rights_status {
        RightsStatus::Permitted => {}
        RightsStatus::Prohibited => {
            return result(
                SourceQualificationStatus::RightsBlocked,
                Some(CompatibilityReason::RightsProhibited),
            );
        }
        RightsStatus::Unknown => {
            return result(
                SourceQualificationStatus::RightsBlocked,
                Some(CompatibilityReason::RightsUnknown),
            );
        }
        RightsStatus::Restricted => {
            if !passport
                .permitted_purposes
                .iter()
                .any(|value| value == purpose.name())
                || passport
                    .restriction_receipt_digest
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                return result(
                    SourceQualificationStatus::RightsBlocked,
                    Some(CompatibilityReason::RestrictedPurposeNotAuthorized),
                );
            }
        }
    }

    match passport.acquisition_status {
        AcquisitionStatus::CandidateAvailable => {
            return result(SourceQualificationStatus::CandidateAvailable, None);
        }
        AcquisitionStatus::AcquisitionPending => {
            return result(SourceQualificationStatus::AcquisitionPending, None);
        }
        AcquisitionStatus::Acquired => {}
    }

    if passport.retrieved_at_epoch.is_none_or(|value| value < 0)
        || passport.artifact_digest.as_deref().is_none_or(str::is_empty)
        || passport.transformation_digest.as_deref().is_none_or(str::is_empty)
        || passport.terms_digest.as_deref().is_none_or(str::is_empty)
    {
        return result(
            SourceQualificationStatus::NotAssessed,
            Some(CompatibilityReason::ArtifactOrTransformDigestMissing),
        );
    }
    result(SourceQualificationStatus::QualifiedForRegisteredUse, None)
}

fn validate_observation(
    observation: &ObservationPassport,
    passport: &SourceArtifactPassport,
) -> Vec<CompatibilityReason> {
    let mut reasons = Vec::new();

    if observation.observation_id.trim().is_empty()
        || observation.artifact_id != passport.artifact_id
        || observation.artifact_digest.trim().is_empty()
        || passport.artifact_digest.as_deref() != Some(observation.artifact_digest.as_str())
        || observation.variable_id.trim().is_empty()
        || observation.construct.trim().is_empty()
        || observation.measurement_unit.trim().is_empty()
        || observation.source_method_id.trim().is_empty()
        || observation.temporal_support.validate().is_err()
        || observation.geography.validate().is_err()
        || observation.sample_size == Some(0)
    {
        reasons.push(CompatibilityReason::ObservationMetadataIncomplete);
    }

    if observation.origin == ObservationOrigin::Unknown {
        reasons.push(CompatibilityReason::ObservationOriginUnknown);
    }

    if observation.origin == ObservationOrigin::SurveyEstimate {
        let sample_design_matches = observation.sample_design_digest.as_deref()
            == passport.sample_design_digest.as_deref();
        let weights_match =
            observation.weights_digest.as_deref() == passport.weights_digest.as_deref();
        if !sample_design_matches
            || !weights_match
            || observation.sample_design_digest.as_deref().is_none_or(str::is_empty)
            || observation.weights_digest.as_deref().is_none_or(str::is_empty)
        {
            reasons.push(CompatibilityReason::SurveyDesignOrWeightsMissing);
        }
    }

    if observation.sampling_unit != passport.sampling_unit {
        reasons.push(CompatibilityReason::ObservationSourceMismatch);
    }
    if observation.measurement_unit != passport.native_measurement_unit
        && observation.origin != ObservationOrigin::DerivedFromRegisteredInputs
    {
        reasons.push(CompatibilityReason::ObservationSourceMismatch);
    }

    if observation.temporal_support.time_basis_id != passport.temporal_support.time_basis_id
        || observation.temporal_support.start_inclusive < passport.temporal_support.start_inclusive
        || observation.temporal_support.end_exclusive > passport.temporal_support.end_exclusive
        || (observation.temporal_support.resolution != passport.temporal_support.resolution
            && (observation.origin != ObservationOrigin::DerivedFromRegisteredInputs
                || observation.transformation_digest.as_deref()
                    != passport.transformation_digest.as_deref()))
    {
        reasons.push(CompatibilityReason::ObservationSourceMismatch);
    }

    if observation.geography.boundary_digest != passport.geography.boundary_digest
        || observation.geography.geography_kind != passport.geography.geography_kind
    {
        reasons.push(CompatibilityReason::ObservationSourceMismatch);
    }

    if observation.origin.requires_parent_lineage() {
        let mut parents = BTreeSet::new();
        if observation.parent_observation_digests.is_empty()
            || observation.transformation_digest.as_deref().is_none_or(str::is_empty)
            || observation.transformation_digest != passport.transformation_digest
            || observation
                .parent_observation_digests
                .iter()
                .any(|digest| digest.trim().is_empty() || !parents.insert(digest.as_str()))
        {
            reasons.push(CompatibilityReason::DerivedLineageMissing);
        }
    }

    reasons
}

fn transform_for<'a>(
    plan: &'a JoinAlignmentPlan,
    artifact_digest: &str,
    kind: TransformKind,
) -> Option<&'a TransformReceipt> {
    plan.transforms.iter().find(|receipt| {
        receipt.transform_kind == kind
            && receipt.input_artifact_digest == artifact_digest
            && !receipt.transform_id.trim().is_empty()
            && !receipt.method_id.trim().is_empty()
            && !receipt.receipt_digest.trim().is_empty()
    })
}

fn validate_transform_coverage(
    receipt: &TransformReceipt,
    minimum_coverage: f64,
) -> bool {
    receipt.coverage_fraction.is_some_and(|value| {
        value.is_finite() && (minimum_coverage..=1.0).contains(&value)
    })
}

/// Check whether two observation passports can be aligned on a declared panel
/// grain. Any required time/geography/unit conversion must have an explicit
/// receipt tied to the exact source artifact digest.
pub fn assess_observation_compatibility(
    left: &ObservationPassport,
    left_source: &SourceArtifactPassport,
    right: &ObservationPassport,
    right_source: &SourceArtifactPassport,
    purpose: RegisteredPurpose,
    plan: &JoinAlignmentPlan,
) -> CompatibilityReport {
    let mut reasons = Vec::new();
    let left_qualification = qualify_source(left_source, purpose);
    let right_qualification = qualify_source(right_source, purpose);

    for qualification in [&left_qualification, &right_qualification] {
        match qualification.status {
            SourceQualificationStatus::RightsBlocked => {
                reasons.push(qualification.reason.clone().unwrap_or(
                    CompatibilityReason::RightsUnknown,
                ));
            }
            SourceQualificationStatus::CandidateAvailable
            | SourceQualificationStatus::AcquisitionPending
            | SourceQualificationStatus::NotAssessed => {
                reasons.push(CompatibilityReason::MissingSourceArtifact);
            }
            SourceQualificationStatus::QualifiedForRegisteredUse => {}
        }
    }

    reasons.extend(validate_observation(left, left_source));
    reasons.extend(validate_observation(right, right_source));

    if left.variable_id != right.variable_id || left.construct != right.construct {
        reasons.push(CompatibilityReason::DifferentVariableOrConstruct);
    }
    for (observation, source) in [(left, left_source), (right, right_source)] {
        if observation.measurement_unit != plan.target_measurement_unit {
            let receipt = transform_for(
                plan,
                source.artifact_digest.as_deref().unwrap_or(""),
                TransformKind::UnitConversion,
            );
            if let Some(receipt) = receipt.filter(|receipt| {
                receipt.target_measurement_unit.as_deref()
                    == Some(plan.target_measurement_unit.as_str())
            }) {
                used_transform_digests.push(receipt.receipt_digest.clone());
                reasons.push(CompatibilityReason::ExplicitTransformUsed);
            } else {
                reasons.push(CompatibilityReason::DifferentMeasurementUnit);
            }
        }
    }

    let mut used_transform_digests = Vec::new();
    for (observation, source) in [(left, left_source), (right, right_source)] {
        if observation.temporal_support != plan.target_period {
            if let Some(receipt) = transform_for(
                plan,
                source.artifact_digest.as_deref().unwrap_or(""),
                TransformKind::TemporalAggregation,
            ) {
                if receipt.target_period.as_ref() == Some(&plan.target_period)
                    && validate_transform_coverage(receipt, plan.minimum_transform_coverage)
                {
                    used_transform_digests.push(receipt.receipt_digest.clone());
                    reasons.push(CompatibilityReason::ExplicitTransformUsed);
                } else {
                    reasons.push(CompatibilityReason::TemporalSupportMismatch);
                    if !validate_transform_coverage(receipt, plan.minimum_transform_coverage) {
                        reasons.push(CompatibilityReason::TransformCoverageBelowThreshold);
                    }
                }
            } else {
                reasons.push(CompatibilityReason::TemporalSupportMismatch);
            }
        }
    }

    for (observation, source) in [(left, left_source), (right, right_source)] {
        if observation.geography != plan.target_geography {
            if let Some(receipt) = transform_for(
                plan,
                source.artifact_digest.as_deref().unwrap_or(""),
                TransformKind::SpatialCrosswalk,
            ) {
                if receipt.target_geography.as_ref() == Some(&plan.target_geography)
                    && validate_transform_coverage(receipt, plan.minimum_transform_coverage)
                {
                    used_transform_digests.push(receipt.receipt_digest.clone());
                    reasons.push(CompatibilityReason::ExplicitTransformUsed);
                } else {
                    reasons.push(CompatibilityReason::GeographyMismatch);
                    if receipt.coverage_fraction.is_some_and(|value| {
                        !value.is_finite() || value < plan.minimum_transform_coverage
                    }) {
                        reasons.push(CompatibilityReason::TransformCoverageBelowThreshold);
                    }
                }
            } else {
                reasons.push(CompatibilityReason::GeographyMismatch);
            }
        }
    }

    for (observation, source) in [(left, left_source), (right, right_source)] {
        if observation.analytical_unit != plan.target_analytical_unit {
            if let Some(receipt) = transform_for(
                plan,
                source.artifact_digest.as_deref().unwrap_or(""),
                TransformKind::AnalyticalUnitAggregation,
            ) {
                if receipt.target_analytical_unit == Some(plan.target_analytical_unit)
                    && validate_transform_coverage(receipt, plan.minimum_transform_coverage)
                {
                    used_transform_digests.push(receipt.receipt_digest.clone());
                    reasons.push(CompatibilityReason::ExplicitTransformUsed);
                } else {
                    reasons.push(CompatibilityReason::AnalyticalUnitMismatch);
                    if !validate_transform_coverage(receipt, plan.minimum_transform_coverage) {
                        reasons.push(CompatibilityReason::TransformCoverageBelowThreshold);
                    }
                }
            } else {
                reasons.push(CompatibilityReason::AnalyticalUnitMismatch);
            }
        }
    }

    for observation in [left, right] {
        if purpose == RegisteredPurpose::EmpiricalValidation {
            match observation.origin {
                ObservationOrigin::CategoryDefault => {
                    reasons.push(CompatibilityReason::CategoryDefaultNotEligibleForEmpiricalValidation);
                }
                ObservationOrigin::Imputed => {
                    reasons.push(CompatibilityReason::ImputedObservationNotEligibleForEmpiricalValidation);
                }
                ObservationOrigin::Unknown => {
                    reasons.push(CompatibilityReason::ObservationOriginUnknown);
                }
                _ => {}
            }
        }
    }

    if !plan.minimum_transform_coverage.is_finite()
        || !(0.0..=1.0).contains(&plan.minimum_transform_coverage)
        || plan.target_measurement_unit.trim().is_empty()
        || plan.target_period.validate().is_err()
        || plan.target_geography.validate().is_err()
    {
        reasons.push(CompatibilityReason::TransformReceiptMissingOrInvalid);
    }

    reasons.sort_by_key(|reason| format!("{reason:?}"));
    reasons.dedup();

    let rights_blocked = left_qualification.status == SourceQualificationStatus::RightsBlocked
        || right_qualification.status == SourceQualificationStatus::RightsBlocked;
    let not_assessed = left_qualification.status != SourceQualificationStatus::QualifiedForRegisteredUse
        && left_qualification.status != SourceQualificationStatus::RightsBlocked
        || right_qualification.status != SourceQualificationStatus::QualifiedForRegisteredUse
            && right_qualification.status != SourceQualificationStatus::RightsBlocked;

    let has_hard_comparability_failure = reasons.iter().any(|reason| {
        matches!(
            reason,
            CompatibilityReason::DifferentVariableOrConstruct
                | CompatibilityReason::DifferentMeasurementUnit
                | CompatibilityReason::TemporalSupportMismatch
                | CompatibilityReason::GeographyMismatch
                | CompatibilityReason::AnalyticalUnitMismatch
                | CompatibilityReason::CategoryDefaultNotEligibleForEmpiricalValidation
                | CompatibilityReason::ImputedObservationNotEligibleForEmpiricalValidation
        )
    });
    let has_explicit_transform = reasons.contains(&CompatibilityReason::ExplicitTransformUsed);
    let status = if rights_blocked {
        CompatibilityStatus::RightsBlocked
    } else if not_assessed
        || reasons.iter().any(|reason| {
            matches!(
                reason,
                CompatibilityReason::MissingSourceArtifact
                    | CompatibilityReason::SourceMetadataIncomplete
                    | CompatibilityReason::ArtifactOrTransformDigestMissing
                    | CompatibilityReason::ObservationMetadataIncomplete
                    | CompatibilityReason::ObservationOriginUnknown
                    | CompatibilityReason::SurveyDesignOrWeightsMissing
                    | CompatibilityReason::DerivedLineageMissing
                    | CompatibilityReason::ObservationSourceMismatch
                    | CompatibilityReason::TransformReceiptMissingOrInvalid
            )
        })
    {
        CompatibilityStatus::NotAssessed
    } else if has_hard_comparability_failure {
        CompatibilityStatus::NotComparable
    } else if has_explicit_transform {
        CompatibilityStatus::CompatibleWithExplicitTransform
    } else {
        CompatibilityStatus::Compatible
    };

    CompatibilityReport {
        left_observation_id: left.observation_id.clone(),
        right_observation_id: right.observation_id.clone(),
        purpose,
        status,
        reasons,
        left_artifact_digest: left_source.artifact_digest.clone().unwrap_or_default(),
        right_artifact_digest: right_source.artifact_digest.clone().unwrap_or_default(),
        explicit_transform_digests: used_transform_digests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(start: i64, end: i64, resolution: TemporalResolution) -> TemporalSupport {
        TemporalSupport {
            start_inclusive: start,
            end_exclusive: end,
            time_basis_id: "calendar-year".to_string(),
            resolution,
        }
    }

    fn geography(id: &str, boundary: &str) -> GeographySupport {
        GeographySupport {
            geography_id: id.to_string(),
            geography_kind: "province".to_string(),
            boundary_version: "MDB-2026".to_string(),
            boundary_digest: boundary.to_string(),
        }
    }

    fn passport(
        id: &str,
        digest: &str,
        rights: RightsStatus,
        acquisition: AcquisitionStatus,
    ) -> SourceArtifactPassport {
        SourceArtifactPassport {
            artifact_id: id.to_string(),
            publisher: "synthetic-publisher".to_string(),
            dataset_title: "synthetic household services".to_string(),
            dataset_version: "2026-v1".to_string(),
            source_family: "synthetic-survey".to_string(),
            landing_page: "https://example.invalid/dataset".to_string(),
            acquisition_locator: "fixture://synthetic-data".to_string(),
            acquisition_status: acquisition,
            retrieved_at_epoch: (acquisition == AcquisitionStatus::Acquired).then_some(1_800_000_000),
            artifact_digest: (acquisition == AcquisitionStatus::Acquired).then(|| digest.to_string()),
            transformation_digest: (acquisition == AcquisitionStatus::Acquired)
                .then(|| format!("transform-{digest}")),
            terms_digest: (acquisition == AcquisitionStatus::Acquired)
                .then(|| format!("terms-{digest}")),
            rights_status: rights,
            permitted_purposes: vec![
                RegisteredPurpose::ScenarioAssumption.name().to_string(),
                RegisteredPurpose::EmpiricalValidation.name().to_string(),
            ],
            restriction_receipt_digest: Some("restriction-receipt-fixture".to_string()),
            source_unit: "households".to_string(),
            target_population: "synthetic private households".to_string(),
            population_exclusions: "none declared in synthetic fixture".to_string(),
            geographic_coverage: "synthetic province-a".to_string(),
            native_measurement_unit: "percent".to_string(),
            temporal_support: period(2025, 2026, TemporalResolution::CalendarYear),
            geography: geography("province-a", "boundary-v1"),
            sampling_unit: IndependentUnitKind::Household,
            sample_design_digest: Some(format!("design-{digest}")),
            weights_digest: Some(format!("weights-{digest}")),
            release_status: "synthetic-verified-release".to_string(),
        }
    }

    fn observation(
        id: &str,
        artifact_id: &str,
        artifact_digest: &str,
        origin: ObservationOrigin,
    ) -> ObservationPassport {
        ObservationPassport {
            observation_id: id.to_string(),
            artifact_id: artifact_id.to_string(),
            artifact_digest: artifact_digest.to_string(),
            variable_id: "electricity-access".to_string(),
            construct: "household electricity access".to_string(),
            measurement_unit: "percent".to_string(),
            origin,
            source_method_id: "survey-weighted-estimate-v1".to_string(),
            temporal_support: period(2025, 2026, TemporalResolution::CalendarYear),
            geography: geography("province-a", "boundary-v1"),
            analytical_unit: IndependentUnitKind::Region,
            sampling_unit: IndependentUnitKind::Household,
            sample_size: Some(500),
            sample_design_digest: Some(format!("design-{artifact_digest}")),
            weights_digest: Some(format!("weights-{artifact_digest}")),
            parent_observation_digests: Vec::new(),
            transformation_digest: None,
        }
    }

    fn plan() -> JoinAlignmentPlan {
        JoinAlignmentPlan {
            target_period: period(2025, 2026, TemporalResolution::CalendarYear),
            target_geography: geography("province-a", "boundary-v1"),
            target_measurement_unit: "percent".to_string(),
            target_analytical_unit: IndependentUnitKind::Region,
            minimum_transform_coverage: 0.95,
            transforms: Vec::new(),
        }
    }

    #[test]
    fn same_semantics_period_and_boundary_are_compatible() {
        let left_source = passport("ghs-2025", "sha256-ghs", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("survey-2025", "sha256-other-survey", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let left = observation("ghs-prov-a", "ghs-2025", "sha256-ghs", ObservationOrigin::SurveyEstimate);
        let right = observation("other-prov-a", "survey-2025", "sha256-other-survey", ObservationOrigin::SurveyEstimate);
        let report = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(report.status, CompatibilityStatus::Compatible);
    }

    #[test]
    fn unaligned_financial_quarter_and_daily_water_are_not_comparable_without_transform() {
        let left_source = passport("treasury", "sha256-treasury", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("dws", "sha256-dws", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let mut left = observation("treasury-quarter", "treasury", "sha256-treasury", ObservationOrigin::AdministrativeSubmission);
        let mut right = observation("dws-daily", "dws", "sha256-dws", ObservationOrigin::AdministrativeSubmission);
        left.temporal_support = period(2025, 2025 + 1, TemporalResolution::Quarter);
        right.temporal_support = period(2025, 2025 + 1, TemporalResolution::Day);
        let report = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(report.status, CompatibilityStatus::NotComparable);
        assert!(report.reasons.contains(&CompatibilityReason::TemporalSupportMismatch));
    }

    #[test]
    fn category_defaults_are_allowed_as_scenario_assumptions_but_not_measured_validation() {
        let left_source = passport("dws-default", "sha256-dws-default", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("other", "sha256-other", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let left = observation("dws-default-value", "dws-default", "sha256-dws-default", ObservationOrigin::CategoryDefault);
        let right = observation("other-value", "other", "sha256-other", ObservationOrigin::AdministrativeSubmission);
        let scenario = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::ScenarioAssumption, &plan()
        );
        assert_eq!(scenario.status, CompatibilityStatus::Compatible);
        let validation = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(validation.status, CompatibilityStatus::NotComparable);
        assert!(validation.reasons.contains(&CompatibilityReason::CategoryDefaultNotEligibleForEmpiricalValidation));
    }

    #[test]
    fn acquisition_pending_is_not_assessed_not_a_pass() {
        let pending = passport("eskom-hourly", "sha256-pending", RightsStatus::Permitted, AcquisitionStatus::AcquisitionPending);
        let other = passport("other", "sha256-other", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let left = observation("eskom", "eskom-hourly", "sha256-pending", ObservationOrigin::DirectMeasurement);
        let right = observation("other", "other", "sha256-other", ObservationOrigin::DirectMeasurement);
        let report = assess_observation_compatibility(
            &left, &pending, &right, &other,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(report.status, CompatibilityStatus::NotAssessed);
    }

    #[test]
    fn known_rights_block_is_not_masked_by_pending_acquisition() {
        let prohibited_candidate = passport(
            "prohibited-candidate",
            "not-acquired",
            RightsStatus::Prohibited,
            AcquisitionStatus::CandidateAvailable,
        );
        let report = qualify_source(
            &prohibited_candidate,
            RegisteredPurpose::EmpiricalValidation,
        );
        assert_eq!(report.status, SourceQualificationStatus::RightsBlocked);
        assert_eq!(
            report.reason,
            Some(CompatibilityReason::RightsProhibited)
        );
    }

    #[test]
    fn survey_observation_must_match_the_exact_passport_weight_and_design_digests() {
        let left_source = passport(
            "survey-a",
            "sha256-survey-a",
            RightsStatus::Permitted,
            AcquisitionStatus::Acquired,
        );
        let right_source = passport(
            "survey-b",
            "sha256-survey-b",
            RightsStatus::Permitted,
            AcquisitionStatus::Acquired,
        );
        let mut left = observation(
            "left",
            "survey-a",
            "sha256-survey-a",
            ObservationOrigin::SurveyEstimate,
        );
        left.weights_digest = Some("sha256-other-weighting".to_string());
        let right = observation(
            "right",
            "survey-b",
            "sha256-survey-b",
            ObservationOrigin::SurveyEstimate,
        );
        let report = assess_observation_compatibility(
            &left,
            &left_source,
            &right,
            &right_source,
            RegisteredPurpose::EmpiricalValidation,
            &plan(),
        );
        assert_eq!(report.status, CompatibilityStatus::NotAssessed);
        assert!(report
            .reasons
            .contains(&CompatibilityReason::SurveyDesignOrWeightsMissing));
    }

    #[test]
    fn unknown_and_prohibited_rights_block_empirical_use() {
        for rights in [RightsStatus::Unknown, RightsStatus::Prohibited] {
            let left_source = passport("restricted", "sha256-restricted", rights, AcquisitionStatus::Acquired);
            let right_source = passport("other", "sha256-other", RightsStatus::Permitted, AcquisitionStatus::Acquired);
            let left = observation("left", "restricted", "sha256-restricted", ObservationOrigin::SurveyEstimate);
            let right = observation("right", "other", "sha256-other", ObservationOrigin::SurveyEstimate);
            let report = assess_observation_compatibility(
                &left, &left_source, &right, &right_source,
                RegisteredPurpose::EmpiricalValidation, &plan()
            );
            assert_eq!(report.status, CompatibilityStatus::RightsBlocked);
        }
    }

    #[test]
    fn boundary_mismatch_requires_crosswalk_bound_to_source_digest() {
        let left_source = passport("old-boundary", "sha256-old", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("new-boundary", "sha256-new", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let left = observation("old", "old-boundary", "sha256-old", ObservationOrigin::AdministrativeSubmission);
        let mut right = observation("new", "new-boundary", "sha256-new", ObservationOrigin::AdministrativeSubmission);
        right.geography = geography("province-a", "boundary-v2");
        let mut alignment = plan();
        alignment.target_geography = geography("province-a", "boundary-v2");
        let blocked = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &alignment
        );
        assert_eq!(blocked.status, CompatibilityStatus::NotComparable);
        alignment.transforms.push(TransformReceipt {
            transform_id: "mdb-crosswalk-v1".to_string(),
            transform_kind: TransformKind::SpatialCrosswalk,
            input_artifact_digest: "sha256-old".to_string(),
            method_id: "areal-weighted-mdb-crosswalk".to_string(),
            receipt_digest: "sha256-crosswalk-receipt".to_string(),
            target_period: None,
            target_geography: Some(geography("province-a", "boundary-v2")),
            target_measurement_unit: None,
            target_analytical_unit: None,
            coverage_fraction: Some(0.98),
        });
        let allowed = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &alignment
        );
        assert_eq!(allowed.status, CompatibilityStatus::CompatibleWithExplicitTransform);
        assert!(allowed.explicit_transform_digests.contains(&"sha256-crosswalk-receipt".to_string()));
    }

    #[test]
    fn low_crosswalk_coverage_does_not_pass() {
        let left_source = passport("old", "sha256-old", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("new", "sha256-new", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let mut left = observation("left", "old", "sha256-old", ObservationOrigin::AdministrativeSubmission);
        left.geography = geography("muni-old", "boundary-old");
        let right = observation("right", "new", "sha256-new", ObservationOrigin::AdministrativeSubmission);
        let mut alignment = plan();
        alignment.target_geography = geography("province-a", "boundary-v1");
        alignment.transforms.push(TransformReceipt {
            transform_id: "weak-crosswalk".to_string(),
            transform_kind: TransformKind::SpatialCrosswalk,
            input_artifact_digest: "sha256-old".to_string(),
            method_id: "unvalidated-area-allocation".to_string(),
            receipt_digest: "sha256-weak-crosswalk".to_string(),
            target_period: None,
            target_geography: Some(geography("province-a", "boundary-v1")),
            target_measurement_unit: None,
            target_analytical_unit: None,
            coverage_fraction: Some(0.5),
        });
        let report = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &alignment
        );
        assert_eq!(report.status, CompatibilityStatus::NotComparable);
        assert!(report.reasons.contains(&CompatibilityReason::TransformCoverageBelowThreshold));
    }

    #[test]
    fn missing_survey_design_and_weights_remain_not_assessed() {
        let left_source = passport("survey-a", "sha256-survey-a", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("survey-b", "sha256-survey-b", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let mut left = observation("left", "survey-a", "sha256-survey-a", ObservationOrigin::SurveyEstimate);
        left.weights_digest = None;
        let right = observation("right", "survey-b", "sha256-survey-b", ObservationOrigin::SurveyEstimate);
        let report = assess_observation_compatibility(
            &left, &left_source, &right, &right_source,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(report.status, CompatibilityStatus::NotAssessed);
        assert!(report.reasons.contains(&CompatibilityReason::SurveyDesignOrWeightsMissing));
    }

    #[test]
    fn unit_conversion_requires_source_bound_receipt_and_is_reported() {
        let left_source = passport("left", "sha256-left", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("right", "sha256-right", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let left = observation("left", "left", "sha256-left", ObservationOrigin::AdministrativeSubmission);
        let right = observation("right", "right", "sha256-right", ObservationOrigin::AdministrativeSubmission);
        let mut alignment = plan();
        alignment.target_measurement_unit = "fraction".to_string();
        alignment.transforms.push(TransformReceipt {
            transform_id: "percent-to-fraction".to_string(),
            transform_kind: TransformKind::UnitConversion,
            input_artifact_digest: "sha256-left".to_string(),
            method_id: "registered-scale-and-offset".to_string(),
            receipt_digest: "sha256-unit-conversion".to_string(),
            target_period: None,
            target_geography: None,
            target_measurement_unit: Some("fraction".to_string()),
            target_analytical_unit: None,
            coverage_fraction: None,
        });
        let report = assess_observation_compatibility(
            &left,
            &left_source,
            &right,
            &right_source,
            RegisteredPurpose::EmpiricalValidation,
            &alignment,
        );
        assert_eq!(report.status, CompatibilityStatus::NotComparable);
        assert!(report.reasons.contains(&CompatibilityReason::DifferentMeasurementUnit));
        alignment.transforms.push(TransformReceipt {
            transform_id: "percent-to-fraction-right".to_string(),
            transform_kind: TransformKind::UnitConversion,
            input_artifact_digest: "sha256-right".to_string(),
            method_id: "registered-scale-and-offset".to_string(),
            receipt_digest: "sha256-unit-conversion-right".to_string(),
            target_period: None,
            target_geography: None,
            target_measurement_unit: Some("fraction".to_string()),
            target_analytical_unit: None,
            coverage_fraction: None,
        });
        let report = assess_observation_compatibility(
            &left,
            &left_source,
            &right,
            &right_source,
            RegisteredPurpose::EmpiricalValidation,
            &alignment,
        );
        assert_eq!(report.status, CompatibilityStatus::CompatibleWithExplicitTransform);
        assert_eq!(report.explicit_transform_digests.len(), 2);
    }

    #[test]
    fn source_sample_unit_mismatch_is_not_reported_as_compatible() {
        let left_source = passport("left", "sha256-left", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let right_source = passport("right", "sha256-right", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let mut left = observation("left", "left", "sha256-left", ObservationOrigin::AdministrativeSubmission);
        left.sampling_unit = IndependentUnitKind::Region;
        let right = observation("right", "right", "sha256-right", ObservationOrigin::AdministrativeSubmission);
        let report = assess_observation_compatibility(
            &left,
            &left_source,
            &right,
            &right_source,
            RegisteredPurpose::EmpiricalValidation,
            &plan(),
        );
        assert_eq!(report.status, CompatibilityStatus::NotAssessed);
        assert!(report.reasons.contains(&CompatibilityReason::ObservationSourceMismatch));
    }

    #[test]
    fn derived_observations_require_unique_parent_and_transform_lineage() {
        let source = passport("derived", "sha256-derived", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let mut obs = observation("derived-observation", "derived", "sha256-derived", ObservationOrigin::DerivedFromRegisteredInputs);
        obs.parent_observation_digests = vec!["sha256-parent".to_string(), "sha256-parent".to_string()];
        obs.transformation_digest = source.transformation_digest.clone();
        let other_source = passport("other", "sha256-other", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let other = observation("other", "other", "sha256-other", ObservationOrigin::DirectMeasurement);
        let report = assess_observation_compatibility(
            &obs, &source, &other, &other_source,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(report.status, CompatibilityStatus::NotAssessed);
        assert!(report.reasons.contains(&CompatibilityReason::DerivedLineageMissing));
    }

    #[test]
    fn restricted_rights_require_purpose_specific_receipt() {
        let mut source = passport("restricted", "sha256-restricted", RightsStatus::Restricted, AcquisitionStatus::Acquired);
        source.permitted_purposes = vec![RegisteredPurpose::ScenarioAssumption.name().to_string()];
        let other_source = passport("other", "sha256-other", RightsStatus::Permitted, AcquisitionStatus::Acquired);
        let left = observation("left", "restricted", "sha256-restricted", ObservationOrigin::SurveyEstimate);
        let right = observation("right", "other", "sha256-other", ObservationOrigin::SurveyEstimate);
        let report = assess_observation_compatibility(
            &left, &source, &right, &other_source,
            RegisteredPurpose::EmpiricalValidation, &plan()
        );
        assert_eq!(report.status, CompatibilityStatus::RightsBlocked);
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rights-aware empirical source passports for CIVDYN.
//!
//! This module makes acquisition state, rights, revision state, units,
//! geography, time resolution and observation provenance explicit. It does not
//! download, authenticate or independently certify source bytes. A caller
//! supplies digests and acquisition attestations; upstream ingestion must
//! hash and independently verify the bytes before constructing a passport.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsePurpose {
    Exploration,
    Calibration,
    Validation,
    Challenge,
    Publication,
}

impl UsePurpose {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Exploration => "exploration",
            Self::Calibration => "calibration",
            Self::Validation => "validation",
            Self::Challenge => "challenge",
            Self::Publication => "publication",
        }
    }

    const fn requires_semantic_verification(self) -> bool {
        !matches!(self, Self::Exploration)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightsStatus {
    OpenPublic,
    ConditionalPublic,
    DataUseAgreement,
    ConsentRequired,
    Prohibited,
    Unknown,
}

impl RightsStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::OpenPublic => "open_public",
            Self::ConditionalPublic => "conditional_public",
            Self::DataUseAgreement => "data_use_agreement",
            Self::ConsentRequired => "consent_required",
            Self::Prohibited => "prohibited",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RightsAuthorization {
    pub status: RightsStatus,
    pub policy_id: String,
    pub permitted_purposes: Vec<UsePurpose>,
    /// Digest of the license, terms, DUA, consent or rights-review record.
    /// This is provenance, not proof that the underlying authorization is valid.
    pub authorization_digest: Option<String>,
}

impl RightsAuthorization {
    fn permits(&self, purpose: UsePurpose) -> bool {
        if self.policy_id.trim().is_empty()
            || !self.permitted_purposes.contains(&purpose)
        {
            return false;
        }
        match self.status {
            RightsStatus::OpenPublic => self
                .authorization_digest
                .as_deref()
                .is_some_and(|digest| !digest.trim().is_empty()),
            RightsStatus::ConditionalPublic
            | RightsStatus::DataUseAgreement
            | RightsStatus::ConsentRequired => self
                .authorization_digest
                .as_deref()
                .is_some_and(|digest| !digest.trim().is_empty()),
            RightsStatus::Prohibited | RightsStatus::Unknown => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionState {
    NotAcquired,
    Requested,
    Received,
    IntegrityVerified,
    SemanticallyVerified,
}

impl AcquisitionState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotAcquired => "not_acquired",
            Self::Requested => "requested",
            Self::Received => "received",
            Self::IntegrityVerified => "integrity_verified",
            Self::SemanticallyVerified => "semantically_verified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionStatus {
    Final,
    Provisional,
    UnderReview,
    Revised,
    Withdrawn,
    Unknown,
}

impl RevisionStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Final => "final",
            Self::Provisional => "provisional",
            Self::UnderReview => "under_review",
            Self::Revised => "revised",
            Self::Withdrawn => "withdrawn",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeographicResolution {
    National,
    Province,
    District,
    Municipality,
    SubMunicipalArea,
    Household,
    GridZone,
    Unknown,
}

impl GeographicResolution {
    pub const fn name(self) -> &'static str {
        match self {
            Self::National => "national",
            Self::Province => "province",
            Self::District => "district",
            Self::Municipality => "municipality",
            Self::SubMunicipalArea => "sub_municipal_area",
            Self::Household => "household",
            Self::GridZone => "grid_zone",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalResolution {
    Hourly,
    Daily,
    Monthly,
    Quarterly,
    FinancialYear,
    Annual,
    Irregular,
    Unknown,
}

impl TemporalResolution {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
            Self::FinancialYear => "financial_year",
            Self::Annual => "annual",
            Self::Irregular => "irregular",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpecification {
    pub source_id: String,
    pub specification_version: String,
    pub construct_id: String,
    pub construct_description: String,
    pub native_measurement_unit: String,
    pub population: String,
    pub source_family: String,
    pub geography: GeographicResolution,
    pub temporal_resolution: TemporalResolution,
    pub method_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifact {
    pub artifact_id: String,
    pub dataset_version: String,
    pub publisher: String,
    pub landing_page: String,
    pub retrieval_timestamp: String,
    pub request_or_query_digest: Option<String>,
    pub artifact_digest: String,
    pub content_digest: String,
    /// Required when revision_status is Revised; binds the superseded artifact.
    pub supersedes_artifact_digest: Option<String>,
    pub acquisition_state: AcquisitionState,
    pub revision_status: RevisionStatus,
    pub rights: RightsAuthorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationClass {
    Observed,
    Estimated,
    Defaulted,
    Imputed,
    Missing,
    Suppressed,
}

impl ObservationClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Estimated => "estimated",
            Self::Defaulted => "defaulted",
            Self::Imputed => "imputed",
            Self::Missing => "missing",
            Self::Suppressed => "suppressed",
        }
    }

    const fn requires_value(self) -> bool {
        matches!(
            self,
            Self::Observed | Self::Estimated | Self::Defaulted | Self::Imputed
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceObservation {
    pub record_id: String,
    pub source_id: String,
    pub artifact_digest: String,
    pub construct_id: String,
    pub geography_code: String,
    pub boundary_version: String,
    /// Exact source-period key, e.g. 2025, 2025-Q2, 2026-11-10T14.
    /// No implicit resampling or period interpolation is performed.
    pub period_key: String,
    pub measurement_unit: String,
    pub class: ObservationClass,
    pub value: Option<f64>,
    pub independent_unit_count: Option<u64>,
    pub measurement_standard_error: Option<f64>,
    pub method_or_default_rule_digest: Option<String>,
    pub missing_or_suppression_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmpiricalSourceBundle {
    pub specification: SourceSpecification,
    pub artifact: SourceArtifact,
    pub observations: Vec<SourceObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcePassportError {
    MissingIdentity,
    ArtifactNotAcquired { state: AcquisitionState },
    ArtifactIntegrityNotVerified,
    ArtifactSemanticVerificationRequired,
    ArtifactRevisionNotUsable { status: RevisionStatus, purpose: UsePurpose },
    ArtifactRevisionLineageMissing,
    RightsNotAuthorized { status: RightsStatus, purpose: UsePurpose },
    EmptyObservationSet,
    DuplicateRecordId { record_id: String },
    ObservationIdentityMismatch { record_id: String },
    ObservationUnitMismatch { record_id: String },
    ObservationValueClassMismatch { record_id: String },
    MissingMethodProvenance { record_id: String },
    MissingAbsenceReason { record_id: String },
    InvalidMeasurementUncertainty { record_id: String },
    InvalidIndependentUnitCount { record_id: String },
}

impl fmt::Display for SourcePassportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SourcePassportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationClassCounts {
    pub observed: usize,
    pub estimated: usize,
    pub defaulted: usize,
    pub imputed: usize,
    pub missing: usize,
    pub suppressed: usize,
}

impl ObservationClassCounts {
    fn from_observations(observations: &[SourceObservation]) -> Self {
        let mut counts = Self {
            observed: 0,
            estimated: 0,
            defaulted: 0,
            imputed: 0,
            missing: 0,
            suppressed: 0,
        };
        for observation in observations {
            match observation.class {
                ObservationClass::Observed => counts.observed += 1,
                ObservationClass::Estimated => counts.estimated += 1,
                ObservationClass::Defaulted => counts.defaulted += 1,
                ObservationClass::Imputed => counts.imputed += 1,
                ObservationClass::Missing => counts.missing += 1,
                ObservationClass::Suppressed => counts.suppressed += 1,
            }
        }
        counts
    }

    pub fn total(&self) -> usize {
        self.observed
            + self.estimated
            + self.defaulted
            + self.imputed
            + self.missing
            + self.suppressed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBundleReceipt {
    pub source_id: String,
    pub specification_version: String,
    pub dataset_version: String,
    pub artifact_digest: String,
    pub content_digest: String,
    pub rights_policy_id: String,
    pub rights_status: RightsStatus,
    pub purpose: UsePurpose,
    pub acquisition_state: AcquisitionState,
    pub revision_status: RevisionStatus,
    pub geography: GeographicResolution,
    pub temporal_resolution: TemporalResolution,
    pub observation_counts: ObservationClassCounts,
}

fn validate_nonempty(values: &[&str]) -> bool {
    values.iter().all(|value| !value.trim().is_empty())
}

/// Validate an acquired source bundle for one explicit use purpose.
///
/// A provisional or under-review source may be used for exploration only.
/// Calibration, validation, challenge and publication require semantic
/// verification and a final source revision. Estimated/defaulted/imputed and
/// absent observations remain separate counts; they are never recast as
/// observed values or silently dropped.
pub fn qualify_source_bundle(
    bundle: &EmpiricalSourceBundle,
    purpose: UsePurpose,
) -> Result<SourceBundleReceipt, SourcePassportError> {
    let spec = &bundle.specification;
    let artifact = &bundle.artifact;
    if !validate_nonempty(&[
        &spec.source_id,
        &spec.specification_version,
        &spec.construct_id,
        &spec.construct_description,
        &spec.native_measurement_unit,
        &spec.population,
        &spec.source_family,
        &spec.method_id,
        &artifact.artifact_id,
        &artifact.dataset_version,
        &artifact.publisher,
        &artifact.landing_page,
        &artifact.retrieval_timestamp,
        &artifact.artifact_digest,
        &artifact.content_digest,
        &artifact.rights.policy_id,
    ]) {
        return Err(SourcePassportError::MissingIdentity);
    }

    match artifact.acquisition_state {
        AcquisitionState::NotAcquired | AcquisitionState::Requested | AcquisitionState::Received => {
            return Err(SourcePassportError::ArtifactNotAcquired {
                state: artifact.acquisition_state,
            });
        }
        AcquisitionState::IntegrityVerified => {
            if purpose.requires_semantic_verification() {
                return Err(SourcePassportError::ArtifactSemanticVerificationRequired);
            }
        }
        AcquisitionState::SemanticallyVerified => {}
    }

    match artifact.revision_status {
        RevisionStatus::Final => {}
        RevisionStatus::Revised
            if artifact
                .supersedes_artifact_digest
                .as_deref()
                .is_some_and(|digest| !digest.trim().is_empty()) => {}
        RevisionStatus::Revised => {
            return Err(SourcePassportError::ArtifactRevisionLineageMissing);
        }
        RevisionStatus::Provisional if purpose == UsePurpose::Exploration => {}
        RevisionStatus::UnderReview if purpose == UsePurpose::Exploration => {}
        other => {
            return Err(SourcePassportError::ArtifactRevisionNotUsable {
                status: other,
                purpose,
            });
        }
    }

    if !artifact.rights.permits(purpose) {
        return Err(SourcePassportError::RightsNotAuthorized {
            status: artifact.rights.status,
            purpose,
        });
    }

    if bundle.observations.is_empty() {
        return Err(SourcePassportError::EmptyObservationSet);
    }

    let mut record_ids = BTreeSet::new();
    for observation in &bundle.observations {
        if !record_ids.insert(observation.record_id.as_str()) {
            return Err(SourcePassportError::DuplicateRecordId {
                record_id: observation.record_id.clone(),
            });
        }

        if !validate_nonempty(&[
            &observation.record_id,
            &observation.source_id,
            &observation.artifact_digest,
            &observation.construct_id,
            &observation.geography_code,
            &observation.boundary_version,
            &observation.period_key,
        ]) {
            return Err(SourcePassportError::MissingIdentity);
        }
        if observation.source_id != spec.source_id
            || observation.artifact_digest != artifact.artifact_digest
            || observation.construct_id != spec.construct_id
        {
            return Err(SourcePassportError::ObservationIdentityMismatch {
                record_id: observation.record_id.clone(),
            });
        }
        if observation.measurement_unit != spec.native_measurement_unit
            || spec.geography == GeographicResolution::Unknown
            || spec.temporal_resolution == TemporalResolution::Unknown
        {
            return Err(SourcePassportError::ObservationUnitMismatch {
                record_id: observation.record_id.clone(),
            });
        }

        let value_matches_class = match observation.class {
            ObservationClass::Observed
            | ObservationClass::Estimated
            | ObservationClass::Defaulted
            | ObservationClass::Imputed => observation.value.is_some_and(f64::is_finite),
            ObservationClass::Missing | ObservationClass::Suppressed => observation.value.is_none(),
        };
        if !value_matches_class {
            return Err(SourcePassportError::ObservationValueClassMismatch {
                record_id: observation.record_id.clone(),
            });
        }
        if matches!(
            observation.class,
            ObservationClass::Observed | ObservationClass::Estimated | ObservationClass::Imputed
        ) && observation
            .method_or_default_rule_digest
            .as_deref()
            .is_none_or(|digest| digest.trim().is_empty())
        {
            return Err(SourcePassportError::MissingMethodProvenance {
                record_id: observation.record_id.clone(),
            });
        }
        if observation.class == ObservationClass::Defaulted
            && observation
                .method_or_default_rule_digest
                .as_deref()
                .is_none_or(|digest| digest.trim().is_empty())
        {
            return Err(SourcePassportError::MissingMethodProvenance {
                record_id: observation.record_id.clone(),
            });
        }
        if matches!(
            observation.class,
            ObservationClass::Missing | ObservationClass::Suppressed
        ) && observation
            .missing_or_suppression_reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            return Err(SourcePassportError::MissingAbsenceReason {
                record_id: observation.record_id.clone(),
            });
        }
        if observation
            .measurement_standard_error
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(SourcePassportError::InvalidMeasurementUncertainty {
                record_id: observation.record_id.clone(),
            });
        }
        if observation.independent_unit_count == Some(0) {
            return Err(SourcePassportError::InvalidIndependentUnitCount {
                record_id: observation.record_id.clone(),
            });
        }
    }

    Ok(SourceBundleReceipt {
        source_id: spec.source_id.clone(),
        specification_version: spec.specification_version.clone(),
        dataset_version: artifact.dataset_version.clone(),
        artifact_digest: artifact.artifact_digest.clone(),
        content_digest: artifact.content_digest.clone(),
        rights_policy_id: artifact.rights.policy_id.clone(),
        rights_status: artifact.rights.status,
        purpose,
        acquisition_state: artifact.acquisition_state,
        revision_status: artifact.revision_status,
        geography: spec.geography,
        temporal_resolution: spec.temporal_resolution,
        observation_counts: ObservationClassCounts::from_observations(&bundle.observations),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelJoinFailure {
    ConstructMismatch,
    MeasurementUnitMismatch,
    GeographicResolutionMismatch,
    GeographyCodeMismatch,
    BoundaryVersionMismatch,
    TemporalResolutionMismatch,
    PeriodMismatch,
}

impl PanelJoinFailure {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ConstructMismatch => "construct_mismatch",
            Self::MeasurementUnitMismatch => "measurement_unit_mismatch",
            Self::GeographicResolutionMismatch => "geographic_resolution_mismatch",
            Self::GeographyCodeMismatch => "geography_code_mismatch",
            Self::BoundaryVersionMismatch => "boundary_version_mismatch",
            Self::TemporalResolutionMismatch => "temporal_resolution_mismatch",
            Self::PeriodMismatch => "period_mismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelJoinDecision {
    pub eligible: bool,
    pub failures: Vec<PanelJoinFailure>,
    /// True when the observations have different measurement provenance
    /// classes (observed versus estimated/defaulted/etc.). This does not
    /// invalidate the key join, but requires explicit treatment in analysis.
    pub observation_class_mismatch: bool,
}

/// Decide whether two aggregate observations can occupy the same empirical
/// panel cell. No temporal resampling, geographic crosswalk or unit conversion
/// is performed implicitly.
pub fn compare_panel_cells(
    left_spec: &SourceSpecification,
    left: &SourceObservation,
    right_spec: &SourceSpecification,
    right: &SourceObservation,
) -> PanelJoinDecision {
    let mut failures = Vec::new();
    if left_spec.construct_id != right_spec.construct_id
        || left.construct_id != left_spec.construct_id
        || right.construct_id != right_spec.construct_id
    {
        failures.push(PanelJoinFailure::ConstructMismatch);
    }
    if left_spec.native_measurement_unit != right_spec.native_measurement_unit
        || left.measurement_unit != left_spec.native_measurement_unit
        || right.measurement_unit != right_spec.native_measurement_unit
    {
        failures.push(PanelJoinFailure::MeasurementUnitMismatch);
    }
    if left_spec.geography != right_spec.geography
        || left_spec.geography == GeographicResolution::Unknown
    {
        failures.push(PanelJoinFailure::GeographicResolutionMismatch);
    }
    if left.geography_code != right.geography_code {
        failures.push(PanelJoinFailure::GeographyCodeMismatch);
    }
    if left.boundary_version != right.boundary_version {
        failures.push(PanelJoinFailure::BoundaryVersionMismatch);
    }
    if left_spec.temporal_resolution != right_spec.temporal_resolution
        || left_spec.temporal_resolution == TemporalResolution::Unknown
    {
        failures.push(PanelJoinFailure::TemporalResolutionMismatch);
    }
    if left.period_key != right.period_key {
        failures.push(PanelJoinFailure::PeriodMismatch);
    }

    PanelJoinDecision {
        eligible: failures.is_empty(),
        failures,
        observation_class_mismatch: left.class != right.class,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rights() -> RightsAuthorization {
        RightsAuthorization {
            status: RightsStatus::OpenPublic,
            policy_id: "license-open-v1".to_string(),
            permitted_purposes: vec![
                UsePurpose::Exploration,
                UsePurpose::Calibration,
                UsePurpose::Validation,
                UsePurpose::Challenge,
                UsePurpose::Publication,
            ],
            authorization_digest: Some("sha256-license-record".to_string()),
        }
    }

    fn spec() -> SourceSpecification {
        SourceSpecification {
            source_id: "dws-water-balance".to_string(),
            specification_version: "v1".to_string(),
            construct_id: "municipal-non-revenue-water".to_string(),
            construct_description: "non-revenue water fraction".to_string(),
            native_measurement_unit: "fraction".to_string(),
            population: "municipal-water-systems".to_string(),
            source_family: "department-water-sanitation".to_string(),
            geography: GeographicResolution::Municipality,
            temporal_resolution: TemporalResolution::Annual,
            method_id: "water-balance-v1".to_string(),
        }
    }

    fn artifact() -> SourceArtifact {
        SourceArtifact {
            artifact_id: "dws-nrw-2024".to_string(),
            dataset_version: "release-2024-v1".to_string(),
            publisher: "Department of Water and Sanitation".to_string(),
            landing_page: "https://www.dws.gov.za/niwis2/".to_string(),
            retrieval_timestamp: "2026-11-10T12:00:00+02:00".to_string(),
            request_or_query_digest: Some("sha256-query".to_string()),
            artifact_digest: "sha256-artifact".to_string(),
            content_digest: "sha256-content".to_string(),
            supersedes_artifact_digest: None,
            acquisition_state: AcquisitionState::SemanticallyVerified,
            revision_status: RevisionStatus::Final,
            rights: rights(),
        }
    }

    fn observation(record_id: &str, class: ObservationClass, value: Option<f64>) -> SourceObservation {
        SourceObservation {
            record_id: record_id.to_string(),
            source_id: "dws-water-balance".to_string(),
            artifact_digest: "sha256-artifact".to_string(),
            construct_id: "municipal-non-revenue-water".to_string(),
            geography_code: "ZA-TSH".to_string(),
            boundary_version: "municipal-demarcation-2021".to_string(),
            period_key: "2024".to_string(),
            measurement_unit: "fraction".to_string(),
            class,
            value,
            independent_unit_count: Some(1),
            measurement_standard_error: None,
            method_or_default_rule_digest: if matches!(
                class,
                ObservationClass::Observed | ObservationClass::Estimated | ObservationClass::Imputed
            ) {
                Some("sha256-method".to_string())
            } else if class == ObservationClass::Defaulted {
                Some("sha256-category-default-rule".to_string())
            } else {
                None
            },
            missing_or_suppression_reason: if matches!(
                class,
                ObservationClass::Missing | ObservationClass::Suppressed
            ) {
                Some("not-reported-by-source".to_string())
            } else {
                None
            },
        }
    }

    fn bundle(observations: Vec<SourceObservation>) -> EmpiricalSourceBundle {
        EmpiricalSourceBundle {
            specification: spec(),
            artifact: artifact(),
            observations,
        }
    }

    #[test]
    fn qualifies_final_open_source_and_preserves_each_observation_class() {
        let candidate = bundle(vec![
            observation("a", ObservationClass::Observed, Some(0.32)),
            observation("b", ObservationClass::Estimated, Some(0.28)),
            observation("c", ObservationClass::Defaulted, Some(0.25)),
            observation("d", ObservationClass::Imputed, Some(0.30)),
            observation("e", ObservationClass::Missing, None),
            observation("f", ObservationClass::Suppressed, None),
        ]);
        let receipt = qualify_source_bundle(&candidate, UsePurpose::Validation)
            .expect("source passport");
        assert_eq!(receipt.observation_counts.observed, 1);
        assert_eq!(receipt.observation_counts.estimated, 1);
        assert_eq!(receipt.observation_counts.defaulted, 1);
        assert_eq!(receipt.observation_counts.imputed, 1);
        assert_eq!(receipt.observation_counts.missing, 1);
        assert_eq!(receipt.observation_counts.suppressed, 1);
        assert_eq!(receipt.observation_counts.total(), 6);
    }

    #[test]
    fn unknown_or_prohibited_rights_never_qualify() {
        let mut unknown = bundle(vec![observation(
            "a",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        unknown.artifact.rights.status = RightsStatus::Unknown;
        assert!(matches!(
            qualify_source_bundle(&unknown, UsePurpose::Validation),
            Err(SourcePassportError::RightsNotAuthorized {
                status: RightsStatus::Unknown,
                ..
            })
        ));

        let mut prohibited = bundle(vec![observation(
            "a",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        prohibited.artifact.rights.status = RightsStatus::Prohibited;
        assert!(matches!(
            qualify_source_bundle(&prohibited, UsePurpose::Exploration),
            Err(SourcePassportError::RightsNotAuthorized {
                status: RightsStatus::Prohibited,
                ..
            })
        ));
    }

    #[test]
    fn missing_license_authorization_digest_fails_closed() {
        let mut candidate = bundle(vec![observation(
            "a",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        candidate.artifact.rights.authorization_digest = None;
        assert!(matches!(
            qualify_source_bundle(&candidate, UsePurpose::Exploration),
            Err(SourcePassportError::RightsNotAuthorized { .. })
        ));
    }

    #[test]
    fn provisional_or_under_review_data_are_exploration_only() {
        let mut provisional = bundle(vec![observation(
            "a",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        provisional.artifact.revision_status = RevisionStatus::Provisional;
        assert!(qualify_source_bundle(&provisional, UsePurpose::Exploration).is_ok());
        assert!(matches!(
            qualify_source_bundle(&provisional, UsePurpose::Validation),
            Err(SourcePassportError::ArtifactRevisionNotUsable { .. })
        ));

        provisional.artifact.revision_status = RevisionStatus::UnderReview;
        assert!(qualify_source_bundle(&provisional, UsePurpose::Exploration).is_ok());
        assert!(matches!(
            qualify_source_bundle(&provisional, UsePurpose::Challenge),
            Err(SourcePassportError::ArtifactRevisionNotUsable { .. })
        ));
    }

    #[test]
    fn integrity_only_artifact_can_support_exploration_but_not_validation() {
        let mut candidate = bundle(vec![observation(
            "a",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        candidate.artifact.acquisition_state = AcquisitionState::IntegrityVerified;
        assert!(qualify_source_bundle(&candidate, UsePurpose::Exploration).is_ok());
        assert_eq!(
            qualify_source_bundle(&candidate, UsePurpose::Validation).unwrap_err(),
            SourcePassportError::ArtifactSemanticVerificationRequired
        );
    }

    #[test]
    fn nan_cannot_be_attached_to_missing_or_suppressed_record() {
        let mut missing = bundle(vec![observation("missing", ObservationClass::Missing, Some(f64::NAN))]);
        missing.observations[0].missing_or_suppression_reason =
            Some("not reported".to_string());
        assert!(matches!(
            qualify_source_bundle(&missing, UsePurpose::Validation),
            Err(SourcePassportError::ObservationValueClassMismatch { .. })
        ));

        let mut suppressed = bundle(vec![observation("suppressed", ObservationClass::Suppressed, Some(f64::NAN))]);
        suppressed.observations[0].missing_or_suppression_reason =
            Some("privacy threshold".to_string());
        assert!(matches!(
            qualify_source_bundle(&suppressed, UsePurpose::Validation),
            Err(SourcePassportError::ObservationValueClassMismatch { .. })
        ));
    }

    #[test]
    fn revised_artifact_requires_supersession_lineage() {
        let mut revised = bundle(vec![observation("a", ObservationClass::Observed, Some(0.32))]);
        revised.artifact.revision_status = RevisionStatus::Revised;
        assert_eq!(
            qualify_source_bundle(&revised, UsePurpose::Validation).unwrap_err(),
            SourcePassportError::ArtifactRevisionLineageMissing
        );
        revised.artifact.supersedes_artifact_digest = Some("sha256-prior-version".to_string());
        assert!(qualify_source_bundle(&revised, UsePurpose::Validation).is_ok());
    }

    #[test]
    fn data_not_acquired_or_merely_received_cannot_become_evidence() {
        let mut candidate = bundle(vec![observation(
            "a",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        candidate.artifact.acquisition_state = AcquisitionState::Requested;
        assert!(matches!(
            qualify_source_bundle(&candidate, UsePurpose::Exploration),
            Err(SourcePassportError::ArtifactNotAcquired {
                state: AcquisitionState::Requested
            })
        ));
    }

    #[test]
    fn defaulted_missing_and_suppressed_values_keep_distinct_semantics() {
        let candidate = bundle(vec![
            observation("default", ObservationClass::Defaulted, Some(0.25)),
            observation("missing", ObservationClass::Missing, None),
            observation("suppressed", ObservationClass::Suppressed, None),
        ]);
        let receipt =
            qualify_source_bundle(&candidate, UsePurpose::Validation).expect("explicit statuses");
        assert_eq!(receipt.observation_counts.defaulted, 1);
        assert_eq!(receipt.observation_counts.missing, 1);
        assert_eq!(receipt.observation_counts.suppressed, 1);
        assert_eq!(receipt.observation_counts.observed, 0);
    }

    #[test]
    fn default_without_rule_and_missing_without_reason_are_rejected() {
        let mut default = bundle(vec![observation(
            "default",
            ObservationClass::Defaulted,
            Some(0.25),
        )]);
        default.observations[0].method_or_default_rule_digest = None;
        assert!(matches!(
            qualify_source_bundle(&default, UsePurpose::Validation),
            Err(SourcePassportError::MissingMethodProvenance { .. })
        ));

        let mut missing = bundle(vec![observation("missing", ObservationClass::Missing, None)]);
        missing.observations[0].missing_or_suppression_reason = None;
        assert!(matches!(
            qualify_source_bundle(&missing, UsePurpose::Validation),
            Err(SourcePassportError::MissingAbsenceReason { .. })
        ));
    }

    #[test]
    fn observation_class_value_consistency_and_digest_lineage_are_enforced() {
        let mut class_mismatch = bundle(vec![observation(
            "a",
            ObservationClass::Missing,
            Some(0.0),
        )]);
        class_mismatch.observations[0].missing_or_suppression_reason =
            Some("not reported".to_string());
        assert!(matches!(
            qualify_source_bundle(&class_mismatch, UsePurpose::Validation),
            Err(SourcePassportError::ObservationValueClassMismatch { .. })
        ));

        let mut wrong_artifact = bundle(vec![observation(
            "b",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        wrong_artifact.observations[0].artifact_digest = "sha256-another-artifact".to_string();
        assert!(matches!(
            qualify_source_bundle(&wrong_artifact, UsePurpose::Validation),
            Err(SourcePassportError::ObservationIdentityMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_records_and_invalid_uncertainty_are_rejected() {
        let duplicate = bundle(vec![
            observation("a", ObservationClass::Observed, Some(0.32)),
            observation("a", ObservationClass::Observed, Some(0.33)),
        ]);
        assert!(matches!(
            qualify_source_bundle(&duplicate, UsePurpose::Validation),
            Err(SourcePassportError::DuplicateRecordId { .. })
        ));

        let mut uncertainty = bundle(vec![observation(
            "b",
            ObservationClass::Observed,
            Some(0.32),
        )]);
        uncertainty.observations[0].measurement_standard_error = Some(-0.1);
        assert!(matches!(
            qualify_source_bundle(&uncertainty, UsePurpose::Validation),
            Err(SourcePassportError::InvalidMeasurementUncertainty { .. })
        ));
    }

    #[test]
    fn panel_join_requires_same_construct_unit_geography_boundary_and_period() {
        let left_spec = spec();
        let mut right_spec = spec();
        let left = observation("a", ObservationClass::Observed, Some(0.32));
        let mut right = observation("b", ObservationClass::Observed, Some(0.34));
        let exact = compare_panel_cells(&left_spec, &left, &right_spec, &right);
        assert!(exact.eligible);
        assert!(!exact.observation_class_mismatch);

        right.boundary_version = "municipal-demarcation-2016".to_string();
        right.period_key = "2025".to_string();
        right_spec.temporal_resolution = TemporalResolution::Quarterly;
        right_spec.geography = GeographicResolution::District;
        let shifted = compare_panel_cells(&left_spec, &left, &right_spec, &right);
        assert!(!shifted.eligible);
        assert!(shifted.failures.contains(&PanelJoinFailure::GeographicResolutionMismatch));
        assert!(shifted.failures.contains(&PanelJoinFailure::BoundaryVersionMismatch));
        assert!(shifted.failures.contains(&PanelJoinFailure::TemporalResolutionMismatch));
        assert!(shifted.failures.contains(&PanelJoinFailure::PeriodMismatch));
    }

    #[test]
    fn observed_and_defaulted_join_is_flagged_not_silently_homogenized() {
        let left_spec = spec();
        let right_spec = spec();
        let left = observation("observed", ObservationClass::Observed, Some(0.32));
        let right = observation("default", ObservationClass::Defaulted, Some(0.25));
        let decision = compare_panel_cells(&left_spec, &left, &right_spec, &right);
        assert!(decision.eligible);
        assert!(decision.observation_class_mismatch);
    }
}

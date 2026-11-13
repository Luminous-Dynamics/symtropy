// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rights-bound animal observation manifests and leakage-resistant replay splits.
//!
//! This crate validates metadata and evaluation boundaries. It does not fetch
//! data, authorize use independently of its permission receipts, or establish
//! scientific accuracy.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_fauna_core::EvidenceClass;

pub const MANIFEST_SCHEMA_VERSION: &str = "ANIMAL-OBSERVATION-REFERENCE-PACK-V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationModality {
    ImageKeypoints2d,
    VideoKeypoints2d,
    MotionCapture3d,
    ForcePlate3d,
    PhysiologySensor,
    SensorimotorExperiment,
    AnimalTracking,
    PopulationSurvey,
    AcousticRecording,
    GenericMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    VisiblePose2d,
    Kinematics2d,
    Kinematics3d,
    ContactForce,
    Physiology,
    SensorimotorResponse,
    IndividualMovementEcology,
    PopulationDynamics,
    AcousticSignal,
    VisualAnimalTracking,
    TaxonIdentity,
}

fn modality_can_support(modality: ObservationModality, claim: ClaimKind) -> bool {
    match claim {
        ClaimKind::VisualAnimalTracking => modality == ObservationModality::VideoKeypoints2d,
        ClaimKind::TaxonIdentity => true,
        ClaimKind::VisiblePose2d => matches!(
            modality,
            ObservationModality::ImageKeypoints2d
                | ObservationModality::VideoKeypoints2d
                | ObservationModality::MotionCapture3d
                | ObservationModality::ForcePlate3d
        ),
        ClaimKind::Kinematics2d => matches!(
            modality,
            ObservationModality::VideoKeypoints2d
                | ObservationModality::MotionCapture3d
                | ObservationModality::ForcePlate3d
        ),
        ClaimKind::Kinematics3d => matches!(
            modality,
            ObservationModality::MotionCapture3d | ObservationModality::ForcePlate3d
        ),
        ClaimKind::ContactForce => modality == ObservationModality::ForcePlate3d,
        ClaimKind::Physiology => modality == ObservationModality::PhysiologySensor,
        ClaimKind::SensorimotorResponse => modality == ObservationModality::SensorimotorExperiment,
        ClaimKind::IndividualMovementEcology => modality == ObservationModality::AnimalTracking,
        ClaimKind::PopulationDynamics => modality == ObservationModality::PopulationSurvey,
        ClaimKind::AcousticSignal => modality == ObservationModality::AcousticRecording,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataOperation {
    ReadMetadata,
    AccessPayload,
    LocalAnalysis,
    Transform,
    RedistributeOriginal,
    RedistributeDerivative,
    TrainModel,
    EvaluateModel,
    PublishMetrics,
    PublishGranularLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    Granted,
    AttributionRequired,
    Restricted,
    Unknown,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OperationPermission {
    pub operation: DataOperation,
    pub decision: PermissionDecision,
    pub authority_ref: Option<String>,
    pub scope_ref: Option<String>,
    pub attribution: Option<String>,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocationDisclosurePolicy {
    pub sensitive_species_location: bool,
    pub minimum_export_grid_m: Option<u32>,
    pub actual_export_grid_m: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub source_id: String,
    pub dataset_id: String,
    pub version: String,
    pub immutable_ref: String,
    pub byte_sha256: Option<String>,
    pub digest_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaxonomyBinding {
    pub source: String,
    pub release: String,
    pub taxon_ids: Vec<String>,
    pub mapping_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MeasurementField {
    pub id: String,
    pub quantity: bool,
    pub unit: Option<String>,
    pub coordinate: bool,
    pub coordinate_frame: Option<String>,
    pub temporal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeBasis {
    UnixMilliseconds,
    UnixSeconds,
    StudyRelativeMilliseconds,
    StudyRelativeSeconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitRole {
    Training,
    Validation,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneralizationTarget {
    UnseenSubject,
    UnseenStudy,
    UnseenSite,
    FutureTime,
    UnseenTaxon,
    WithinSubjectInterpolation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SplitRecord {
    pub record_id: String,
    pub study_id: String,
    pub subject_id: String,
    pub site_id: Option<String>,
    pub session_id: Option<String>,
    pub track_id: String,
    pub duplicate_group: Option<String>,
    pub taxon_id: String,
    pub role: SplitRole,
    pub start_time: i64,
    pub end_time: i64,
    pub provenance: EvidenceClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferencePackManifest {
    pub schema_version: String,
    pub pack_id: String,
    pub source: SourceSnapshot,
    pub taxonomy: TaxonomyBinding,
    pub provenance: EvidenceClass,
    pub modality: ObservationModality,
    pub fields: Vec<MeasurementField>,
    pub time_basis: Option<TimeBasis>,
    pub sample_interval_ms: Option<u64>,
    pub requested_operations: Vec<DataOperation>,
    pub permissions: Vec<OperationPermission>,
    pub location_disclosure: Option<LocationDisclosurePolicy>,
    pub declared_supported_claims: Vec<ClaimKind>,
    pub requested_claims: Vec<ClaimKind>,
    pub generalization: GeneralizationTarget,
    pub minimum_temporal_gap: i64,
    pub records: Vec<SplitRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestDisposition {
    Eligible,
    Indeterminate,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestReason {
    UnsupportedSchemaVersion,
    EmptyPackId,
    EmptySourceId,
    EmptyDatasetId,
    EmptySourceVersion,
    EmptyImmutableReference,
    MissingRequiredDigest,
    MalformedDigest,
    EmptyTaxonomySource,
    EmptyTaxonomyRelease,
    EmptyTaxonId,
    DuplicateTaxonBinding,
    MissingTaxonomyMappingReference,
    NoMeasurementFields,
    EmptyFieldId,
    DuplicateFieldId,
    MissingQuantityUnit,
    MissingCoordinateFrame,
    MissingTemporalSemantics,
    MissingRequestedOperations,
    DuplicateRequestedOperation,
    MissingPermissionReceipt,
    DuplicatePermissionReceipt,
    PermissionUnknown,
    PermissionRestricted,
    PermissionRevoked,
    MissingPermissionAuthority,
    MissingPermissionScope,
    MissingPermissionReviewDate,
    MissingRequiredAttribution,
    EmptyClaimList,
    DuplicateClaim,
    ClaimExceedsMeasurementEnvelope,
    NoSplitRecords,
    EmptyRecordId,
    DuplicateRecordId,
    EmptyStudyId,
    EmptySubjectId,
    EmptyTrackId,
    EmptyRecordTaxon,
    RecordTaxonMismatch,
    InvalidSamplingInterval,
    MissingRequiredSplitRole,
    InvalidTimeRange,
    ProvenancePromotion,
    SplitIdentityLeakage,
    DuplicateObservationLeakage,
    TemporalLeakage,
    MissingRequiredSite,
    MissingLocationPrivacyPolicy,
    MissingLocationResolution,
    GranularLocationPrivacyViolation,
    MissingWithinSubjectContinuity,
    InvalidTemporalGap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestAssessment {
    pub pack_id: String,
    pub disposition: ManifestDisposition,
    pub reason_codes: Vec<ManifestReason>,
    pub split_report: SplitReport,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LeakageViolation {
    pub reason: ManifestReason,
    pub group_key: String,
    pub roles: Vec<SplitRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitReport {
    pub record_count: usize,
    pub training_count: usize,
    pub validation_count: usize,
    pub test_count: usize,
    pub leakage_violations: Vec<LeakageViolation>,
    pub valid: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn is_lowercase_sha256(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn add_cross_split(
    group_key: String,
    role: SplitRole,
    groups: &mut BTreeMap<String, BTreeSet<SplitRole>>,
) {
    groups.entry(group_key).or_default().insert(role);
}

/// Check split leakage using the declared generalization target.
/// Subject, site, duplicate-group, and taxon IDs are canonical within the pack;
/// track IDs are study-scoped. Raw study-local subject IDs must be namespaced
/// unless evidence-backed cross-study identity resolution links the same animal.
pub fn validate_split(
    records: &[SplitRecord],
    target: GeneralizationTarget,
    minimum_temporal_gap: i64,
) -> SplitReport {
    let mut counts = BTreeMap::<SplitRole, usize>::new();
    for record in records {
        *counts.entry(record.role).or_default() += 1;
    }

    let mut violations = BTreeSet::new();
    let mut duplicate_groups = BTreeMap::new();
    let mut track_groups = BTreeMap::new();
    let mut subject_groups = BTreeMap::new();
    let mut study_groups = BTreeMap::new();
    let mut site_groups = BTreeMap::new();
    let mut taxon_groups = BTreeMap::new();

    let mut seen_record_ids = BTreeSet::new();
    for record in records {
        if record.start_time > record.end_time {
            violations.insert(LeakageViolation {
                reason: ManifestReason::InvalidTimeRange,
                group_key: record.record_id.clone(),
                roles: vec![record.role],
            });
        }
        if !seen_record_ids.insert(record.record_id.clone()) {
            violations.insert(LeakageViolation {
                reason: ManifestReason::DuplicateRecordId,
                group_key: record.record_id.clone(),
                roles: vec![record.role],
            });
        }
        if let Some(duplicate_group) = record
            .duplicate_group
            .as_deref()
            .filter(|group| !group.trim().is_empty())
        {
            add_cross_split(
                format!("duplicate:{duplicate_group}"),
                record.role,
                &mut duplicate_groups,
            );
        }
        if !matches!(
            target,
            GeneralizationTarget::FutureTime | GeneralizationTarget::WithinSubjectInterpolation
        ) {
            add_cross_split(
                format!("track:{}:{}", record.study_id, record.track_id),
                record.role,
                &mut track_groups,
            );
        }

        match target {
            GeneralizationTarget::UnseenSubject => add_cross_split(
                format!("subject:{}", record.subject_id),
                record.role,
                &mut subject_groups,
            ),
            GeneralizationTarget::UnseenStudy => add_cross_split(
                format!("study:{}", record.study_id),
                record.role,
                &mut study_groups,
            ),
            GeneralizationTarget::UnseenSite => {
                if let Some(site_id) = record.site_id.as_deref() {
                    add_cross_split(format!("site:{site_id}"), record.role, &mut site_groups);
                }
            }
            GeneralizationTarget::UnseenTaxon => add_cross_split(
                format!("taxon:{}", record.taxon_id),
                record.role,
                &mut taxon_groups,
            ),
            GeneralizationTarget::FutureTime | GeneralizationTarget::WithinSubjectInterpolation => {
            }
        }
    }

    for (groups, reason) in [
        (
            &duplicate_groups,
            ManifestReason::DuplicateObservationLeakage,
        ),
        (&track_groups, ManifestReason::DuplicateObservationLeakage),
        (&subject_groups, ManifestReason::SplitIdentityLeakage),
        (&study_groups, ManifestReason::SplitIdentityLeakage),
        (&site_groups, ManifestReason::SplitIdentityLeakage),
        (&taxon_groups, ManifestReason::SplitIdentityLeakage),
    ] {
        for (group_key, roles) in groups {
            if roles.len() > 1 {
                violations.insert(LeakageViolation {
                    reason,
                    group_key: group_key.clone(),
                    roles: roles.iter().copied().collect(),
                });
            }
        }
    }

    if target == GeneralizationTarget::WithinSubjectInterpolation {
        let mut windows = BTreeMap::<(String, String, String), Vec<&SplitRecord>>::new();
        for record in records {
            windows
                .entry((
                    record.study_id.clone(),
                    record.subject_id.clone(),
                    record.track_id.clone(),
                ))
                .or_default()
                .push(record);
        }
        for ((study_id, subject_id, track_id), group) in &mut windows {
            group.sort_by(|left, right| {
                left.start_time
                    .cmp(&right.start_time)
                    .then_with(|| left.end_time.cmp(&right.end_time))
                    .then_with(|| left.record_id.cmp(&right.record_id))
            });
            let mut maximum_end_by_role = BTreeMap::<SplitRole, i64>::new();
            for record in group.iter() {
                for other_role in [SplitRole::Training, SplitRole::Validation, SplitRole::Test] {
                    if other_role != record.role
                        && maximum_end_by_role
                            .get(&other_role)
                            .is_some_and(|end| *end >= record.start_time)
                    {
                        let roles = [other_role, record.role]
                            .into_iter()
                            .collect::<BTreeSet<_>>();
                        violations.insert(LeakageViolation {
                            reason: ManifestReason::TemporalLeakage,
                            group_key: format!(
                                "window:{study_id}:{subject_id}:{track_id}:{}",
                                record.record_id
                            ),
                            roles: roles.into_iter().collect(),
                        });
                    }
                }
                maximum_end_by_role
                    .entry(record.role)
                    .and_modify(|maximum| *maximum = (*maximum).max(record.end_time))
                    .or_insert(record.end_time);
            }
        }
    }

    if target == GeneralizationTarget::FutureTime {
        let max_training_end = records
            .iter()
            .filter(|record| {
                record.role == SplitRole::Training && record.start_time <= record.end_time
            })
            .map(|record| record.end_time)
            .max();
        if let Some(max_end) = max_training_end {
            let earliest_allowed = max_end.saturating_add(minimum_temporal_gap.max(0));
            for record in records.iter().filter(|record| {
                record.role != SplitRole::Training
                    && record.start_time <= record.end_time
                    && record.start_time <= earliest_allowed
            }) {
                violations.insert(LeakageViolation {
                    reason: ManifestReason::TemporalLeakage,
                    group_key: record.record_id.clone(),
                    roles: vec![record.role],
                });
            }
        }
    }

    let leakage_violations = violations.into_iter().collect::<Vec<_>>();
    SplitReport {
        record_count: records.len(),
        training_count: counts
            .get(&SplitRole::Training)
            .copied()
            .unwrap_or_default(),
        validation_count: counts
            .get(&SplitRole::Validation)
            .copied()
            .unwrap_or_default(),
        test_count: counts.get(&SplitRole::Test).copied().unwrap_or_default(),
        valid: !records.is_empty()
            && leakage_violations.is_empty()
            && counts
                .get(&SplitRole::Training)
                .copied()
                .unwrap_or_default()
                > 0
            && counts.get(&SplitRole::Test).copied().unwrap_or_default() > 0,
        leakage_violations,
    }
}

/// Validate rights, measurement semantics, claim ceilings, and split integrity.
/// This function does not ingest payload bytes or override the source owner's
/// permission decision; it checks that required permission receipts are present.
pub fn validate_manifest(manifest: &ReferencePackManifest) -> ManifestAssessment {
    let mut rejected = BTreeSet::new();
    let mut indeterminate = BTreeSet::new();

    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        rejected.insert(ManifestReason::UnsupportedSchemaVersion);
    }
    if manifest.pack_id.trim().is_empty() {
        rejected.insert(ManifestReason::EmptyPackId);
    }

    let source = &manifest.source;
    if source.source_id.trim().is_empty() {
        rejected.insert(ManifestReason::EmptySourceId);
    }
    if source.dataset_id.trim().is_empty() {
        rejected.insert(ManifestReason::EmptyDatasetId);
    }
    if source.version.trim().is_empty() {
        indeterminate.insert(ManifestReason::EmptySourceVersion);
    }
    if source.immutable_ref.trim().is_empty() {
        indeterminate.insert(ManifestReason::EmptyImmutableReference);
    }
    match source.byte_sha256.as_deref() {
        Some(digest) if !is_lowercase_sha256(digest) => {
            rejected.insert(ManifestReason::MalformedDigest);
        }
        None if source.digest_required => {
            rejected.insert(ManifestReason::MissingRequiredDigest);
        }
        _ => {}
    }

    if manifest.taxonomy.source.trim().is_empty() {
        indeterminate.insert(ManifestReason::EmptyTaxonomySource);
    }
    if manifest.taxonomy.release.trim().is_empty() {
        indeterminate.insert(ManifestReason::EmptyTaxonomyRelease);
    }
    if manifest.taxonomy.taxon_ids.is_empty()
        || manifest
            .taxonomy
            .taxon_ids
            .iter()
            .any(|taxon_id| taxon_id.trim().is_empty())
    {
        indeterminate.insert(ManifestReason::EmptyTaxonId);
    }
    let mut declared_taxa = BTreeSet::new();
    for taxon_id in &manifest.taxonomy.taxon_ids {
        if !declared_taxa.insert(taxon_id.clone()) {
            rejected.insert(ManifestReason::DuplicateTaxonBinding);
        }
    }
    if manifest
        .taxonomy
        .mapping_ref
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        indeterminate.insert(ManifestReason::MissingTaxonomyMappingReference);
    }

    if manifest.fields.is_empty() {
        indeterminate.insert(ManifestReason::NoMeasurementFields);
    }
    if manifest.sample_interval_ms == Some(0) {
        rejected.insert(ManifestReason::InvalidSamplingInterval);
    }
    let mut field_ids = BTreeSet::new();
    for field in &manifest.fields {
        if field.id.trim().is_empty() {
            rejected.insert(ManifestReason::EmptyFieldId);
        }
        if !field_ids.insert(field.id.clone()) {
            rejected.insert(ManifestReason::DuplicateFieldId);
        }
        if field.quantity
            && field
                .unit
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            indeterminate.insert(ManifestReason::MissingQuantityUnit);
        }
        if field.coordinate
            && field
                .coordinate_frame
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            indeterminate.insert(ManifestReason::MissingCoordinateFrame);
        }
        if field.temporal
            && (manifest.time_basis.is_none() || manifest.sample_interval_ms.is_none())
        {
            indeterminate.insert(ManifestReason::MissingTemporalSemantics);
        }
    }

    if manifest.requested_operations.is_empty() {
        indeterminate.insert(ManifestReason::MissingRequestedOperations);
    }
    let mut requested = BTreeSet::new();
    for operation in &manifest.requested_operations {
        if !requested.insert(*operation) {
            rejected.insert(ManifestReason::DuplicateRequestedOperation);
        }
    }

    let mut permissions = BTreeMap::<DataOperation, Vec<&OperationPermission>>::new();
    for permission in &manifest.permissions {
        permissions
            .entry(permission.operation)
            .or_default()
            .push(permission);
    }
    for receipts in permissions.values() {
        if receipts.len() > 1 {
            rejected.insert(ManifestReason::DuplicatePermissionReceipt);
        }
    }
    for operation in &manifest.requested_operations {
        let Some(receipts) = permissions.get(operation) else {
            indeterminate.insert(ManifestReason::MissingPermissionReceipt);
            continue;
        };
        if receipts.is_empty() {
            indeterminate.insert(ManifestReason::MissingPermissionReceipt);
            continue;
        }
        // Inspect every duplicate receipt: never make the outcome depend on
        // whichever conflicting permission happened to appear first.
        for permission in receipts {
            match permission.decision {
                PermissionDecision::Restricted => {
                    rejected.insert(ManifestReason::PermissionRestricted);
                }
                PermissionDecision::Revoked => {
                    rejected.insert(ManifestReason::PermissionRevoked);
                }
                PermissionDecision::Unknown => {
                    indeterminate.insert(ManifestReason::PermissionUnknown);
                }
                PermissionDecision::Granted | PermissionDecision::AttributionRequired => {
                    if permission
                        .authority_ref
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    {
                        indeterminate.insert(ManifestReason::MissingPermissionAuthority);
                    }
                    if permission
                        .scope_ref
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    {
                        indeterminate.insert(ManifestReason::MissingPermissionScope);
                    }
                    if permission
                        .reviewed_at
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    {
                        indeterminate.insert(ManifestReason::MissingPermissionReviewDate);
                    }
                    if permission.decision == PermissionDecision::AttributionRequired
                        && permission
                            .attribution
                            .as_deref()
                            .is_none_or(|value| value.trim().is_empty())
                    {
                        rejected.insert(ManifestReason::MissingRequiredAttribution);
                    }
                }
            }
        }
    }

    if requested.contains(&DataOperation::PublishGranularLocation) {
        match manifest.location_disclosure.as_ref() {
            None => {
                indeterminate.insert(ManifestReason::MissingLocationPrivacyPolicy);
            }
            Some(policy) => {
                if policy.actual_export_grid_m.is_none() {
                    indeterminate.insert(ManifestReason::MissingLocationResolution);
                }
                if policy.sensitive_species_location {
                    match (policy.minimum_export_grid_m, policy.actual_export_grid_m) {
                        (None, _) => {
                            indeterminate.insert(ManifestReason::MissingLocationPrivacyPolicy);
                        }
                        (Some(minimum), Some(actual)) if actual < minimum => {
                            rejected.insert(ManifestReason::GranularLocationPrivacyViolation);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if manifest.declared_supported_claims.is_empty() || manifest.requested_claims.is_empty() {
        indeterminate.insert(ManifestReason::EmptyClaimList);
    }
    for claims in [
        &manifest.declared_supported_claims,
        &manifest.requested_claims,
    ] {
        let mut seen = BTreeSet::new();
        for claim in claims {
            if !seen.insert(*claim) {
                rejected.insert(ManifestReason::DuplicateClaim);
            }
            if !modality_can_support(manifest.modality, *claim) {
                rejected.insert(ManifestReason::ClaimExceedsMeasurementEnvelope);
            }
        }
    }
    let supported = manifest
        .declared_supported_claims
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if manifest
        .requested_claims
        .iter()
        .any(|claim| !supported.contains(claim))
    {
        rejected.insert(ManifestReason::ClaimExceedsMeasurementEnvelope);
    }

    if manifest.records.is_empty() {
        indeterminate.insert(ManifestReason::NoSplitRecords);
    }
    if !manifest
        .records
        .iter()
        .any(|record| record.role == SplitRole::Training)
        || !manifest
            .records
            .iter()
            .any(|record| record.role == SplitRole::Test)
    {
        indeterminate.insert(ManifestReason::MissingRequiredSplitRole);
    }
    if manifest.minimum_temporal_gap < 0 {
        rejected.insert(ManifestReason::InvalidTemporalGap);
    }
    let mut record_ids = BTreeSet::new();
    for record in &manifest.records {
        if record.record_id.trim().is_empty() {
            rejected.insert(ManifestReason::EmptyRecordId);
        }
        if !record_ids.insert(record.record_id.clone()) {
            rejected.insert(ManifestReason::DuplicateRecordId);
        }
        if record.study_id.trim().is_empty() {
            rejected.insert(ManifestReason::EmptyStudyId);
        }
        if record.subject_id.trim().is_empty() {
            rejected.insert(ManifestReason::EmptySubjectId);
        }
        if record.track_id.trim().is_empty() {
            rejected.insert(ManifestReason::EmptyTrackId);
        }
        if record.taxon_id.trim().is_empty() {
            rejected.insert(ManifestReason::EmptyRecordTaxon);
        } else if !declared_taxa.is_empty() && !declared_taxa.contains(&record.taxon_id) {
            indeterminate.insert(ManifestReason::RecordTaxonMismatch);
        }
        if record.start_time > record.end_time {
            rejected.insert(ManifestReason::InvalidTimeRange);
        }
        if manifest.provenance != EvidenceClass::Synthetic
            && record.provenance == EvidenceClass::Synthetic
        {
            rejected.insert(ManifestReason::ProvenancePromotion);
        }
        if manifest.generalization == GeneralizationTarget::UnseenSite
            && record.site_id.as_deref().is_none_or(str::is_empty)
        {
            indeterminate.insert(ManifestReason::MissingRequiredSite);
        }
    }

    if manifest.generalization == GeneralizationTarget::WithinSubjectInterpolation {
        let mut identity_roles = BTreeMap::<(String, String, String), BTreeSet<SplitRole>>::new();
        for record in &manifest.records {
            identity_roles
                .entry((
                    record.study_id.clone(),
                    record.subject_id.clone(),
                    record.track_id.clone(),
                ))
                .or_default()
                .insert(record.role);
        }
        if !identity_roles.values().any(|roles| roles.len() > 1) {
            indeterminate.insert(ManifestReason::MissingWithinSubjectContinuity);
        }
    }

    let split_report = validate_split(
        &manifest.records,
        manifest.generalization,
        manifest.minimum_temporal_gap,
    );
    for violation in &split_report.leakage_violations {
        match violation.reason {
            ManifestReason::DuplicateRecordId => {
                rejected.insert(ManifestReason::DuplicateRecordId);
            }
            ManifestReason::InvalidTimeRange => {
                rejected.insert(ManifestReason::InvalidTimeRange);
            }
            ManifestReason::SplitIdentityLeakage => {
                rejected.insert(ManifestReason::SplitIdentityLeakage);
            }
            ManifestReason::DuplicateObservationLeakage => {
                rejected.insert(ManifestReason::DuplicateObservationLeakage);
            }
            ManifestReason::TemporalLeakage => {
                rejected.insert(ManifestReason::TemporalLeakage);
            }
            _ => {}
        }
    }

    let disposition = if !rejected.is_empty() {
        ManifestDisposition::Reject
    } else if !indeterminate.is_empty() {
        ManifestDisposition::Indeterminate
    } else {
        ManifestDisposition::Eligible
    };
    let reason_codes = rejected.union(&indeterminate).copied().collect();

    ManifestAssessment {
        pack_id: manifest.pack_id.clone(),
        disposition,
        reason_codes,
        split_report,
    }
}

/// Canonical SHA-256 digest insensitive to input ordering for manifest lists.
/// The digest binds all declared fields and policy receipts; it does not assert
/// that source payload bytes were legally available or scientifically valid.
pub fn manifest_sha256(manifest: &ReferencePackManifest) -> Result<String, serde_json::Error> {
    let mut canonical = manifest.clone();
    canonical.taxonomy.taxon_ids.sort();
    canonical.fields.sort();
    canonical.requested_operations.sort();
    canonical.permissions.sort();
    canonical.declared_supported_claims.sort();
    canonical.requested_claims.sort();
    canonical.records.sort();
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(sha256_hex(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> ReferencePackManifest {
        let permission = |operation| OperationPermission {
            operation,
            decision: PermissionDecision::Granted,
            authority_ref: Some("test://authority/receipt".to_owned()),
            scope_ref: Some("test://scope/local-evaluation".to_owned()),
            attribution: None,
            reviewed_at: Some("2026-01-01".to_owned()),
        };
        ReferencePackManifest {
            schema_version: MANIFEST_SCHEMA_VERSION.to_owned(),
            pack_id: "synthetic/valid-pack".to_owned(),
            source: SourceSnapshot {
                source_id: "synthetic-source".to_owned(),
                dataset_id: "synthetic-dataset".to_owned(),
                version: "v1".to_owned(),
                immutable_ref: "test://snapshot/v1".to_owned(),
                byte_sha256: Some("a".repeat(64)),
                digest_required: true,
            },
            taxonomy: TaxonomyBinding {
                source: "test-taxonomy".to_owned(),
                release: "test-release-1".to_owned(),
                taxon_ids: vec!["TEST:1".to_owned()],
                mapping_ref: Some("test://mapping/1".to_owned()),
            },
            provenance: EvidenceClass::Synthetic,
            modality: ObservationModality::VideoKeypoints2d,
            fields: vec![
                MeasurementField {
                    id: "keypoints".to_owned(),
                    quantity: true,
                    unit: Some("pixel".to_owned()),
                    coordinate: true,
                    coordinate_frame: Some("image-pixel-v1".to_owned()),
                    temporal: true,
                },
                MeasurementField {
                    id: "frame-index".to_owned(),
                    quantity: true,
                    unit: Some("frame".to_owned()),
                    coordinate: false,
                    coordinate_frame: None,
                    temporal: false,
                },
            ],
            time_basis: Some(TimeBasis::StudyRelativeMilliseconds),
            sample_interval_ms: Some(40),
            requested_operations: vec![DataOperation::LocalAnalysis, DataOperation::EvaluateModel],
            permissions: vec![
                permission(DataOperation::LocalAnalysis),
                permission(DataOperation::EvaluateModel),
            ],
            location_disclosure: None,
            declared_supported_claims: vec![
                ClaimKind::VisiblePose2d,
                ClaimKind::Kinematics2d,
                ClaimKind::VisualAnimalTracking,
            ],
            requested_claims: vec![ClaimKind::VisiblePose2d],
            generalization: GeneralizationTarget::UnseenSubject,
            minimum_temporal_gap: 0,
            records: vec![
                SplitRecord {
                    record_id: "r-train".to_owned(),
                    study_id: "study-train".to_owned(),
                    subject_id: "animal-1".to_owned(),
                    site_id: Some("site-a".to_owned()),
                    session_id: Some("session-a".to_owned()),
                    track_id: "track-1".to_owned(),
                    duplicate_group: Some("dup-train".to_owned()),
                    taxon_id: "TEST:1".to_owned(),
                    role: SplitRole::Training,
                    start_time: 0,
                    end_time: 100,
                    provenance: EvidenceClass::Synthetic,
                },
                SplitRecord {
                    record_id: "r-test".to_owned(),
                    study_id: "study-test".to_owned(),
                    subject_id: "animal-2".to_owned(),
                    site_id: Some("site-b".to_owned()),
                    session_id: Some("session-b".to_owned()),
                    track_id: "track-2".to_owned(),
                    duplicate_group: Some("dup-test".to_owned()),
                    taxon_id: "TEST:1".to_owned(),
                    role: SplitRole::Test,
                    start_time: 200,
                    end_time: 300,
                    provenance: EvidenceClass::Synthetic,
                },
            ],
        }
    }

    #[test]
    fn valid_synthetic_manifest_is_eligible() {
        assert_eq!(
            validate_manifest(&valid_manifest()).disposition,
            ManifestDisposition::Eligible
        );
    }

    #[test]
    fn requested_use_without_permission_receipt_is_indeterminate() {
        let mut manifest = valid_manifest();
        manifest.permissions.pop();
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Indeterminate);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::MissingPermissionReceipt)
        );
    }

    #[test]
    fn revoked_or_restricted_permission_is_rejected() {
        for decision in [PermissionDecision::Revoked, PermissionDecision::Restricted] {
            let mut manifest = valid_manifest();
            manifest.permissions[0].decision = decision;
            assert_eq!(
                validate_manifest(&manifest).disposition,
                ManifestDisposition::Reject
            );
        }
    }

    #[test]
    fn unknown_permission_never_passes() {
        let mut manifest = valid_manifest();
        manifest.permissions[0].decision = PermissionDecision::Unknown;
        assert_ne!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Eligible
        );
    }

    #[test]
    fn missing_scope_for_granted_permission_is_indeterminate() {
        let mut manifest = valid_manifest();
        manifest.permissions[0].scope_ref = None;
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Indeterminate);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::MissingPermissionScope)
        );
    }

    #[test]
    fn missing_attribution_is_rejected() {
        let mut manifest = valid_manifest();
        manifest.permissions[0].decision = PermissionDecision::AttributionRequired;
        manifest.permissions[0].attribution = Some("  ".to_owned());
        assert_eq!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Reject
        );
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::MissingRequiredAttribution)
        );
    }

    #[test]
    fn still_image_cannot_claim_three_dimensional_kinematics_or_forces() {
        let mut manifest = valid_manifest();
        manifest.modality = ObservationModality::ImageKeypoints2d;
        manifest.declared_supported_claims = vec![ClaimKind::Kinematics3d, ClaimKind::ContactForce];
        manifest.requested_claims = vec![ClaimKind::Kinematics3d];
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Reject);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::ClaimExceedsMeasurementEnvelope)
        );
    }

    #[test]
    fn digest_must_be_well_formed_when_supplied() {
        let mut manifest = valid_manifest();
        manifest.source.byte_sha256 = Some("not-a-digest".to_owned());
        assert_eq!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Reject
        );
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::MalformedDigest)
        );
    }

    #[test]
    fn required_missing_digest_is_rejected() {
        let mut manifest = valid_manifest();
        manifest.source.byte_sha256 = None;
        assert_eq!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Reject
        );
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::MissingRequiredDigest)
        );
    }

    #[test]
    fn quantity_without_units_is_indeterminate() {
        let mut manifest = valid_manifest();
        manifest.fields[0].unit = None;
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Indeterminate);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::MissingQuantityUnit)
        );
    }

    #[test]
    fn coordinate_without_frame_is_indeterminate() {
        let mut manifest = valid_manifest();
        manifest.fields[0].coordinate_frame = None;
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Indeterminate);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::MissingCoordinateFrame)
        );
    }

    #[test]
    fn temporal_data_without_time_semantics_is_indeterminate() {
        let mut manifest = valid_manifest();
        manifest.time_basis = None;
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Indeterminate);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::MissingTemporalSemantics)
        );
    }

    #[test]
    fn unknown_taxon_mapping_never_becomes_validated() {
        let mut manifest = valid_manifest();
        manifest.taxonomy.mapping_ref = None;
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Indeterminate);
    }

    #[test]
    fn duplicate_record_ids_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.records[1].record_id = manifest.records[0].record_id.clone();
        assert_eq!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Reject
        );
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::DuplicateRecordId)
        );
    }

    #[test]
    fn subject_leakage_is_detected_across_study_scoped_identity() {
        let mut manifest = valid_manifest();
        manifest.records[1].study_id = manifest.records[0].study_id.clone();
        manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Reject);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::SplitIdentityLeakage)
        );
    }

    #[test]
    fn same_numeric_subject_id_in_different_studies_is_not_assumed_to_be_same_animal() {
        let mut manifest = valid_manifest();
        manifest.records[0].subject_id = "animal-1".to_owned();
        manifest.records[1].subject_id = "animal-1".to_owned();
        assert_eq!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Eligible
        );
    }

    #[test]
    fn unseen_study_split_rejects_same_study_across_roles() {
        let mut manifest = valid_manifest();
        manifest.generalization = GeneralizationTarget::UnseenStudy;
        manifest.records[1].study_id = manifest.records[0].study_id.clone();
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::SplitIdentityLeakage)
        );
    }

    #[test]
    fn unseen_site_split_rejects_same_site_across_roles() {
        let mut manifest = valid_manifest();
        manifest.generalization = GeneralizationTarget::UnseenSite;
        manifest.records[1].site_id = manifest.records[0].site_id.clone();
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::SplitIdentityLeakage)
        );
    }

    #[test]
    fn future_time_split_enforces_declared_gap() {
        let mut manifest = valid_manifest();
        manifest.generalization = GeneralizationTarget::FutureTime;
        manifest.minimum_temporal_gap = 50;
        manifest.records[1].start_time = 120;
        manifest.records[1].end_time = 300;
        let assessment = validate_manifest(&manifest);
        assert_eq!(assessment.disposition, ManifestDisposition::Reject);
        assert!(
            assessment
                .reason_codes
                .contains(&ManifestReason::TemporalLeakage)
        );
    }

    #[test]
    fn provenance_promotion_is_rejected() {
        let mut manifest = valid_manifest();
        manifest.provenance = EvidenceClass::Measured;
        assert_eq!(
            validate_manifest(&manifest).disposition,
            ManifestDisposition::Reject
        );
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::ProvenancePromotion)
        );
    }

    #[test]
    fn duplicate_observation_group_may_not_cross_splits() {
        let mut manifest = valid_manifest();
        manifest.records[1].duplicate_group = manifest.records[0].duplicate_group.clone();
        assert!(
            validate_manifest(&manifest)
                .reason_codes
                .contains(&ManifestReason::DuplicateObservationLeakage)
        );
    }

    #[test]
    fn manifest_digest_is_order_independent() {
        let a = valid_manifest();
        let mut b = a.clone();
        b.fields.reverse();
        b.permissions.reverse();
        b.records.reverse();
        assert_eq!(manifest_sha256(&a).unwrap(), manifest_sha256(&b).unwrap());
    }

    #[test]
    fn manifest_digest_changes_when_scope_changes() {
        let a = valid_manifest();
        let mut b = a.clone();
        b.permissions[0].scope_ref = Some("test://scope/another-use".to_owned());
        assert_ne!(manifest_sha256(&a).unwrap(), manifest_sha256(&b).unwrap());
    }

    #[test]
    fn absent_records_do_not_produce_a_valid_split() {
        assert!(!validate_split(&[], GeneralizationTarget::UnseenSubject, 0).valid);
    }
}

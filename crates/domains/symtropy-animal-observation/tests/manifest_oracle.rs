// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::Deserialize;
use symtropy_animal_observation::{
    ClaimKind, DataOperation, GeneralizationTarget, ManifestDisposition, ManifestReason,
    ObservationModality, OperationPermission, PermissionDecision, ReferencePackManifest, SplitRole,
    validate_manifest,
};

const FIXTURE: &str = include_str!("../fixtures/observation-manifest-oracle-v1.json");

#[derive(Debug, Deserialize)]
struct OracleCorpus {
    schema_version: String,
    cases: Vec<OracleCase>,
}

#[derive(Debug, Deserialize)]
struct OracleCase {
    id: String,
    mutation: Mutation,
    expected_disposition: ManifestDisposition,
    expected_reason_codes: Vec<ManifestReason>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum Mutation {
    #[serde(rename = "none")]
    Unchanged,
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
    MissingTaxonIds,
    DuplicateTaxonBinding,
    MissingTaxonomyMapping,
    NoMeasurementFields,
    EmptyFieldId,
    DuplicateFieldId,
    MissingQuantityUnit,
    MissingCoordinateFrame,
    MissingTemporalSemantics,
    NoRequestedOperations,
    DuplicateRequestedOperation,
    MissingPermissionReceipt,
    DuplicatePermissionReceipt,
    UnknownPermission,
    RestrictedPermission,
    RevokedPermission,
    MissingPermissionAuthority,
    MissingPermissionScope,
    MissingPermissionReviewDate,
    MissingAttribution,
    EmptyClaimLists,
    DuplicateClaim,
    ClaimExceedsModalityEnvelope,
    NoSplitRecords,
    EmptyRecordId,
    DuplicateRecordId,
    EmptyStudyId,
    EmptySubjectId,
    EmptyTrackId,
    EmptyRecordTaxon,
    RecordTaxonMismatch,
    InvalidTimeRange,
    ProvenancePromotion,
    UnseenSubjectLeak,
    CrossStudyLinkedSubjectLeak,
    DuplicateObservationLeak,
    FutureTimeLeak,
    MissingRequiredSite,
    InvalidTemporalGap,
    InvalidSamplingInterval,
    MissingTrainingRole,
    ConflictingPermissionReceipts,
    MultiTaxonPack,
    UnseenTaxonLeak,
    FutureTimeSameTrackDisjoint,
    FutureTimeBoundaryTouch,
    WithinSubjectInterpolationOverlap,
    WithinSubjectInterpolationDisjoint,
    WithinSubjectInterpolationNoContinuity,
    GranularLocationMissingPolicy,
    GranularLocationTooFine,
    GranularLocationSafe,
    GranularLocationMissingResolution,
    WhitespacePermissionScope,
    WhitespaceAttribution,
}

fn apply_mutation(
    mut manifest: ReferencePackManifest,
    mutation: Mutation,
) -> ReferencePackManifest {
    match mutation {
        Mutation::Unchanged => {}
        Mutation::UnsupportedSchemaVersion => manifest.schema_version = "UNKNOWN-V9".to_owned(),
        Mutation::EmptyPackId => manifest.pack_id.clear(),
        Mutation::EmptySourceId => manifest.source.source_id.clear(),
        Mutation::EmptyDatasetId => manifest.source.dataset_id.clear(),
        Mutation::EmptySourceVersion => manifest.source.version.clear(),
        Mutation::EmptyImmutableReference => manifest.source.immutable_ref.clear(),
        Mutation::MissingRequiredDigest => manifest.source.byte_sha256 = None,
        Mutation::MalformedDigest => manifest.source.byte_sha256 = Some("not-a-digest".to_owned()),
        Mutation::EmptyTaxonomySource => manifest.taxonomy.source.clear(),
        Mutation::EmptyTaxonomyRelease => manifest.taxonomy.release.clear(),
        Mutation::MissingTaxonIds => manifest.taxonomy.taxon_ids.clear(),
        Mutation::DuplicateTaxonBinding => {
            manifest
                .taxonomy
                .taxon_ids
                .push(manifest.taxonomy.taxon_ids[0].clone());
        }
        Mutation::MissingTaxonomyMapping => manifest.taxonomy.mapping_ref = None,
        Mutation::NoMeasurementFields => manifest.fields.clear(),
        Mutation::EmptyFieldId => manifest.fields[0].id.clear(),
        Mutation::DuplicateFieldId => manifest.fields[1].id = manifest.fields[0].id.clone(),
        Mutation::MissingQuantityUnit => manifest.fields[0].unit = None,
        Mutation::MissingCoordinateFrame => manifest.fields[0].coordinate_frame = None,
        Mutation::MissingTemporalSemantics => manifest.time_basis = None,
        Mutation::NoRequestedOperations => manifest.requested_operations.clear(),
        Mutation::DuplicateRequestedOperation => {
            manifest
                .requested_operations
                .push(manifest.requested_operations[0]);
        }
        Mutation::MissingPermissionReceipt => {
            manifest.permissions.pop();
        }
        Mutation::DuplicatePermissionReceipt => {
            manifest.permissions.push(manifest.permissions[0].clone());
        }
        Mutation::UnknownPermission => {
            manifest.permissions[0].decision = PermissionDecision::Unknown;
        }
        Mutation::RestrictedPermission => {
            manifest.permissions[0].decision = PermissionDecision::Restricted;
        }
        Mutation::RevokedPermission => {
            manifest.permissions[0].decision = PermissionDecision::Revoked;
        }
        Mutation::MissingPermissionAuthority => manifest.permissions[0].authority_ref = None,
        Mutation::MissingPermissionScope => manifest.permissions[0].scope_ref = None,
        Mutation::MissingPermissionReviewDate => manifest.permissions[0].reviewed_at = None,
        Mutation::MissingAttribution => {
            manifest.permissions[0].decision = PermissionDecision::AttributionRequired;
            manifest.permissions[0].attribution = None;
        }
        Mutation::EmptyClaimLists => {
            manifest.declared_supported_claims.clear();
            manifest.requested_claims.clear();
        }
        Mutation::DuplicateClaim => {
            manifest
                .declared_supported_claims
                .push(manifest.declared_supported_claims[0]);
        }
        Mutation::ClaimExceedsModalityEnvelope => {
            manifest
                .declared_supported_claims
                .push(ClaimKind::Physiology);
            manifest.requested_claims = vec![ClaimKind::Physiology];
        }
        Mutation::NoSplitRecords => manifest.records.clear(),
        Mutation::EmptyRecordId => manifest.records[1].record_id.clear(),
        Mutation::DuplicateRecordId => {
            manifest.records[1].record_id = manifest.records[0].record_id.clone();
        }
        Mutation::EmptyStudyId => manifest.records[1].study_id.clear(),
        Mutation::EmptySubjectId => manifest.records[1].subject_id.clear(),
        Mutation::EmptyTrackId => manifest.records[1].track_id.clear(),
        Mutation::EmptyRecordTaxon => manifest.records[1].taxon_id.clear(),
        Mutation::RecordTaxonMismatch => manifest.records[1].taxon_id = "TEST:2".to_owned(),
        Mutation::InvalidTimeRange => {
            manifest.records[1].start_time = 301;
            manifest.records[1].end_time = 300;
        }
        Mutation::ProvenancePromotion => {
            manifest.provenance = symtropy_fauna_core::EvidenceClass::Measured;
        }
        Mutation::UnseenSubjectLeak => {
            manifest.records[1].study_id = manifest.records[0].study_id.clone();
            manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
        }
        Mutation::CrossStudyLinkedSubjectLeak => {
            manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
        }
        Mutation::DuplicateObservationLeak => {
            manifest.records[1].duplicate_group = manifest.records[0].duplicate_group.clone();
        }
        Mutation::FutureTimeLeak => {
            manifest.generalization = GeneralizationTarget::FutureTime;
            manifest.minimum_temporal_gap = 50;
            manifest.records[1].start_time = 120;
            manifest.records[1].end_time = 300;
        }
        Mutation::MissingRequiredSite => {
            manifest.generalization = GeneralizationTarget::UnseenSite;
            manifest.records[1].site_id = None;
        }
        Mutation::InvalidTemporalGap => manifest.minimum_temporal_gap = -1,
        Mutation::InvalidSamplingInterval => manifest.sample_interval_ms = Some(0),
        Mutation::MissingTrainingRole => manifest.records[0].role = SplitRole::Test,
        Mutation::ConflictingPermissionReceipts => {
            let mut receipt = manifest.permissions[0].clone();
            receipt.decision = PermissionDecision::Restricted;
            manifest.permissions.push(receipt);
        }
        Mutation::MultiTaxonPack => {
            manifest.taxonomy.taxon_ids.push("TEST:2".to_owned());
            manifest.records[1].taxon_id = "TEST:2".to_owned();
        }
        Mutation::UnseenTaxonLeak => {
            manifest.generalization = GeneralizationTarget::UnseenTaxon;
        }
        Mutation::FutureTimeSameTrackDisjoint => {
            manifest.generalization = GeneralizationTarget::FutureTime;
            manifest.records[1].study_id = manifest.records[0].study_id.clone();
            manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
            manifest.records[1].track_id = manifest.records[0].track_id.clone();
            manifest.records[1].start_time = 200;
            manifest.records[1].end_time = 300;
        }
        Mutation::FutureTimeBoundaryTouch => {
            manifest.generalization = GeneralizationTarget::FutureTime;
            manifest.records[1].start_time = 100;
            manifest.records[1].end_time = 300;
        }
        Mutation::WithinSubjectInterpolationOverlap => {
            manifest.generalization = GeneralizationTarget::WithinSubjectInterpolation;
            manifest.records[1].study_id = manifest.records[0].study_id.clone();
            manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
            manifest.records[1].track_id = manifest.records[0].track_id.clone();
            manifest.records[1].start_time = 50;
            manifest.records[1].end_time = 150;
        }
        Mutation::WithinSubjectInterpolationDisjoint => {
            manifest.generalization = GeneralizationTarget::WithinSubjectInterpolation;
            manifest.records[1].study_id = manifest.records[0].study_id.clone();
            manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
            manifest.records[1].track_id = manifest.records[0].track_id.clone();
            manifest.records[1].start_time = 200;
            manifest.records[1].end_time = 300;
        }
        Mutation::WithinSubjectInterpolationNoContinuity => {
            manifest.generalization = GeneralizationTarget::WithinSubjectInterpolation;
        }
        Mutation::GranularLocationMissingPolicy => {
            grant_location_operation(&mut manifest);
            manifest.location_disclosure = None;
        }
        Mutation::GranularLocationTooFine => {
            grant_location_operation(&mut manifest);
            manifest.location_disclosure =
                Some(symtropy_animal_observation::LocationDisclosurePolicy {
                    sensitive_species_location: true,
                    minimum_export_grid_m: Some(10_000),
                    actual_export_grid_m: Some(100),
                });
        }
        Mutation::GranularLocationSafe => {
            grant_location_operation(&mut manifest);
            manifest.location_disclosure =
                Some(symtropy_animal_observation::LocationDisclosurePolicy {
                    sensitive_species_location: true,
                    minimum_export_grid_m: Some(10_000),
                    actual_export_grid_m: Some(20_000),
                });
        }
        Mutation::GranularLocationMissingResolution => {
            grant_location_operation(&mut manifest);
            manifest.location_disclosure =
                Some(symtropy_animal_observation::LocationDisclosurePolicy {
                    sensitive_species_location: false,
                    minimum_export_grid_m: None,
                    actual_export_grid_m: None,
                });
        }
        Mutation::WhitespacePermissionScope => {
            manifest.permissions[0].scope_ref = Some("   ".to_owned());
        }
        Mutation::WhitespaceAttribution => {
            manifest.permissions[0].decision = PermissionDecision::AttributionRequired;
            manifest.permissions[0].attribution = Some("  ".to_owned());
        }
    }
    manifest
}

#[test]
fn frozen_manifest_oracle_matches_every_expected_disposition_and_reason() {
    let fixture: OracleCorpus = serde_json::from_str(FIXTURE).expect("valid synthetic oracle JSON");
    assert_eq!(fixture.schema_version, "ANIMAL-OBSERVATION-ORACLE-V1");
    assert!(fixture.cases.len() >= 40);
    let base = manifest_fixture();

    for case in &fixture.cases {
        let manifest = apply_mutation(base.clone(), case.mutation.clone());
        let result = validate_manifest(&manifest);
        assert_eq!(
            result.disposition, case.expected_disposition,
            "wrong disposition for {}: {:?}",
            case.id, result.reason_codes
        );
        assert_eq!(
            result.reason_codes, case.expected_reason_codes,
            "wrong reason codes for {}",
            case.id
        );
    }
}

fn grant_location_operation(manifest: &mut ReferencePackManifest) {
    manifest
        .requested_operations
        .push(DataOperation::PublishGranularLocation);
    manifest.permissions.push(OperationPermission {
        operation: DataOperation::PublishGranularLocation,
        decision: PermissionDecision::Granted,
        authority_ref: Some("test://authority/location-publication".to_owned()),
        scope_ref: Some("test://scope/aggregated-location-only".to_owned()),
        attribution: None,
        reviewed_at: Some("2026-01-01".to_owned()),
    });
}

fn manifest_fixture() -> ReferencePackManifest {
    let permission = |operation| OperationPermission {
        operation,
        decision: PermissionDecision::Granted,
        authority_ref: Some("test://authority/receipt".to_owned()),
        scope_ref: Some("test://scope/local-evaluation".to_owned()),
        attribution: None,
        reviewed_at: Some("2026-01-01".to_owned()),
    };
    ReferencePackManifest {
        schema_version: symtropy_animal_observation::MANIFEST_SCHEMA_VERSION.to_owned(),
        pack_id: "synthetic/valid-pack".to_owned(),
        source: symtropy_animal_observation::SourceSnapshot {
            source_id: "synthetic-source".to_owned(),
            dataset_id: "synthetic-dataset".to_owned(),
            version: "v1".to_owned(),
            immutable_ref: "test://snapshot/v1".to_owned(),
            byte_sha256: Some("a".repeat(64)),
            digest_required: true,
        },
        taxonomy: symtropy_animal_observation::TaxonomyBinding {
            source: "test-taxonomy".to_owned(),
            release: "test-release-1".to_owned(),
            taxon_ids: vec!["TEST:1".to_owned()],
            mapping_ref: Some("test://mapping/1".to_owned()),
        },
        provenance: symtropy_fauna_core::EvidenceClass::Synthetic,
        modality: ObservationModality::VideoKeypoints2d,
        fields: vec![
            symtropy_animal_observation::MeasurementField {
                id: "keypoints".to_owned(),
                quantity: true,
                unit: Some("pixel".to_owned()),
                coordinate: true,
                coordinate_frame: Some("image-pixel-v1".to_owned()),
                temporal: true,
            },
            symtropy_animal_observation::MeasurementField {
                id: "frame-index".to_owned(),
                quantity: true,
                unit: Some("frame".to_owned()),
                coordinate: false,
                coordinate_frame: None,
                temporal: false,
            },
        ],
        time_basis: Some(symtropy_animal_observation::TimeBasis::StudyRelativeMilliseconds),
        sample_interval_ms: Some(40),
        requested_operations: vec![DataOperation::LocalAnalysis, DataOperation::EvaluateModel],
        permissions: vec![
            permission(DataOperation::LocalAnalysis),
            permission(DataOperation::EvaluateModel),
        ],
        declared_supported_claims: vec![
            ClaimKind::VisiblePose2d,
            ClaimKind::Kinematics2d,
            ClaimKind::VisualAnimalTracking,
        ],
        requested_claims: vec![ClaimKind::VisiblePose2d],
        generalization: GeneralizationTarget::UnseenSubject,
        minimum_temporal_gap: 0,
        location_disclosure: None,
        records: vec![
            symtropy_animal_observation::SplitRecord {
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
                provenance: symtropy_fauna_core::EvidenceClass::Synthetic,
            },
            symtropy_animal_observation::SplitRecord {
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
                provenance: symtropy_fauna_core::EvidenceClass::Synthetic,
            },
        ],
    }
}

#[test]
fn multi_taxon_manifests_support_category_disjoint_validation() {
    let mut manifest = manifest_fixture();
    manifest.taxonomy.taxon_ids = vec!["TEST:1".to_owned(), "TEST:2".to_owned()];
    manifest.generalization = GeneralizationTarget::UnseenTaxon;
    manifest.records[1].taxon_id = "TEST:2".to_owned();
    assert_eq!(
        validate_manifest(&manifest).disposition,
        ManifestDisposition::Eligible
    );
    manifest.records[1].taxon_id = "TEST:1".to_owned();
    assert!(
        validate_manifest(&manifest)
            .reason_codes
            .contains(&ManifestReason::SplitIdentityLeakage)
    );
}

#[test]
fn canonical_manifest_digest_binds_policy_and_ignores_list_order() {
    use symtropy_animal_observation::manifest_sha256;

    let mut forward = manifest_fixture();
    let mut reversed = forward.clone();
    reversed.fields.reverse();
    reversed.permissions.reverse();
    reversed.requested_operations.reverse();
    reversed.requested_claims.reverse();
    reversed.declared_supported_claims.reverse();
    reversed.taxonomy.taxon_ids.reverse();
    reversed.records.reverse();
    assert_eq!(
        manifest_sha256(&forward).unwrap(),
        manifest_sha256(&reversed).unwrap()
    );

    reversed.permissions[0].scope_ref = Some("test://scope/changed".to_owned());
    assert_ne!(
        manifest_sha256(&forward).unwrap(),
        manifest_sha256(&reversed).unwrap()
    );
    forward.requested_operations.push(DataOperation::TrainModel);
    assert_ne!(
        manifest_sha256(&forward).unwrap(),
        manifest_sha256(&reversed).unwrap()
    );
}

#[test]
fn split_report_is_independent_of_record_order() {
    use symtropy_animal_observation::validate_split;

    let mut manifest = manifest_fixture();
    let mut validation = manifest.records[1].clone();
    validation.record_id = "r-validation".to_owned();
    validation.study_id = manifest.records[0].study_id.clone();
    validation.subject_id = manifest.records[0].subject_id.clone();
    validation.track_id = "track-validation".to_owned();
    validation.duplicate_group = Some("dup-validation".to_owned());
    validation.role = SplitRole::Validation;
    manifest.records[1].study_id = manifest.records[0].study_id.clone();
    manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
    manifest.records.push(validation);

    let first = validate_split(
        &manifest.records,
        manifest.generalization,
        manifest.minimum_temporal_gap,
    );
    manifest.records.reverse();
    let reversed = validate_split(
        &manifest.records,
        manifest.generalization,
        manifest.minimum_temporal_gap,
    );
    assert_eq!(first, reversed);
}

#[test]
fn within_subject_interpolation_requires_same_subject_windows_and_detects_overlap() {
    let mut disjoint = manifest_fixture();
    disjoint.generalization = GeneralizationTarget::WithinSubjectInterpolation;
    disjoint.records[1].study_id = disjoint.records[0].study_id.clone();
    disjoint.records[1].subject_id = disjoint.records[0].subject_id.clone();
    disjoint.records[1].track_id = disjoint.records[0].track_id.clone();
    disjoint.records[1].start_time = 200;
    disjoint.records[1].end_time = 300;
    assert_eq!(
        validate_manifest(&disjoint).disposition,
        ManifestDisposition::Eligible
    );

    disjoint.records[1].start_time = 100;
    disjoint.records[1].end_time = 200;
    assert!(
        validate_manifest(&disjoint)
            .reason_codes
            .contains(&ManifestReason::TemporalLeakage)
    );
}

#[test]
fn within_subject_interpolation_without_shared_subject_windows_is_indeterminate() {
    let mut manifest = manifest_fixture();
    manifest.generalization = GeneralizationTarget::WithinSubjectInterpolation;
    let result = validate_manifest(&manifest);
    assert_eq!(result.disposition, ManifestDisposition::Indeterminate);
    assert!(
        result
            .reason_codes
            .contains(&ManifestReason::MissingWithinSubjectContinuity)
    );
}

#[test]
fn unseen_subject_split_rejects_linked_individual_across_different_studies() {
    let mut manifest = manifest_fixture();
    manifest.generalization = GeneralizationTarget::UnseenSubject;
    manifest.records[1].subject_id = manifest.records[0].subject_id.clone();
    assert_ne!(manifest.records[0].study_id, manifest.records[1].study_id);
    let result = validate_manifest(&manifest);
    assert_eq!(result.disposition, ManifestDisposition::Reject);
    assert!(
        result
            .reason_codes
            .contains(&ManifestReason::SplitIdentityLeakage)
    );
}

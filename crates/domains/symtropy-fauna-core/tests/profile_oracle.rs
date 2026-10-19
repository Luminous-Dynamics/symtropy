// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::Deserialize;
use symtropy_fauna_core::{
    AnimalProfile, CapabilityClaim, CoverageDeclaration, CoverageError, CoverageState,
    EvidenceClass, EvidenceState, ReasonCode, TaxonBinding, TaxonomyScope, UsePermission,
    ValidationDisposition, corpus_sha256, coverage_report, validate_profile, validate_profiles,
};

const FIXTURE: &str = include_str!("../fixtures/animal-capability-profiles-v1.json");

#[derive(Debug, Deserialize)]
struct CorpusFixture {
    schema_version: String,
    synthetic_template: AnimalProfile,
    real_template: AnimalProfile,
    archetypes: Vec<AnimalProfile>,
    cases: Vec<OracleCase>,
}

#[derive(Debug, Deserialize)]
struct OracleCase {
    id: String,
    base_profile: String,
    mutation: Mutation,
    expected_disposition: ValidationDisposition,
    expected_reason_codes: Vec<ReasonCode>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum Mutation {
    #[serde(rename = "none")]
    NoMutation,
    EmptyProfileId,
    WhitespaceProfileId,
    SyntheticTaxonBinding,
    SyntheticMeasuredProvenance,
    SyntheticModeledProvenance,
    RealMissingTaxon,
    RealEmptyTaxonSource,
    RealEmptyTaxonRelease,
    RealEmptyTaxonId,
    RealSyntheticProvenance,
    RealSyntheticClaimEvidence,
    EmptyCapabilityAxis,
    WhitespaceCapabilityAxis,
    EmptyCapabilityKey,
    DuplicateCapabilityKey,
    RealMissingEvidenceSource,
    RealUnknownRights,
    RealRestrictedRights,
    RealSyntheticOnlyRights,
    RealAttributionMissing,
    RealEvidenceTaxonMismatch,
    RealEvidenceTaxonMissing,
    RealStageRequirementMismatch,
    RealEvidenceStageMismatch,
    RealUnknownCapability,
    RealNotMeasuredCapability,
    RealConflictedCapability,
    RealNoCapabilityClaims,
    RealWhitespaceSourceRef,
    RealUnknownRightsForAbsence,
    RealRestrictedRightsUnasserted,
    RealUnknownRightsUnasserted,
    RealMissingPermissionScope,
    RealWhitespacePermissionScope,
    RealAttributionWhitespace,
    RealAbsentClaimValid,
    RealNotApplicableClaimValid,
    RealModeledEvidenceValid,
    RealReconstructedEvidenceValid,
    SyntheticAbsentClaimValid,
    SyntheticUnknownClaimValid,
    RealAttributionPresent,
    RealStageRequirementMatched,
    RealTaxonSourceWhitespace,
    RealTaxonIdWhitespace,
}

fn apply_mutation(mut profile: AnimalProfile, mutation: Mutation) -> AnimalProfile {
    match mutation {
        Mutation::NoMutation => {}
        Mutation::EmptyProfileId => profile.id.clear(),
        Mutation::WhitespaceProfileId => profile.id = "   ".to_owned(),
        Mutation::SyntheticTaxonBinding => {
            profile.taxon = Some(TaxonBinding {
                source: "test-taxonomy".to_owned(),
                release: "test-release-1".to_owned(),
                taxon_id: "TEST:synthetic".to_owned(),
            });
        }
        Mutation::SyntheticMeasuredProvenance => profile.provenance = EvidenceClass::Measured,
        Mutation::SyntheticModeledProvenance => profile.provenance = EvidenceClass::Modeled,
        Mutation::RealMissingTaxon => profile.taxon = None,
        Mutation::RealEmptyTaxonSource => profile.taxon.as_mut().unwrap().source.clear(),
        Mutation::RealEmptyTaxonRelease => profile.taxon.as_mut().unwrap().release.clear(),
        Mutation::RealEmptyTaxonId => profile.taxon.as_mut().unwrap().taxon_id.clear(),
        Mutation::RealSyntheticProvenance => profile.provenance = EvidenceClass::Synthetic,
        Mutation::RealSyntheticClaimEvidence => {
            profile.capabilities[0].evidence.class = EvidenceClass::Synthetic;
        }
        Mutation::EmptyCapabilityAxis => profile.capabilities[0].axis.clear(),
        Mutation::WhitespaceCapabilityAxis => profile.capabilities[0].axis = "  ".to_owned(),
        Mutation::EmptyCapabilityKey => profile.capabilities[0].key.clear(),
        Mutation::DuplicateCapabilityKey => {
            profile.capabilities.push(profile.capabilities[0].clone());
        }
        Mutation::RealMissingEvidenceSource => {
            profile.capabilities[0].evidence.source_ref = None;
        }
        Mutation::RealUnknownRights => {
            profile.capabilities[0].evidence.permission = UsePermission::Unknown;
        }
        Mutation::RealRestrictedRights => {
            profile.capabilities[0].evidence.permission = UsePermission::Restricted;
        }
        Mutation::RealSyntheticOnlyRights => {
            profile.capabilities[0].evidence.permission = UsePermission::SyntheticOnly;
        }
        Mutation::RealAttributionMissing => {
            profile.capabilities[0].evidence.permission = UsePermission::AttributionRequired;
            profile.capabilities[0].evidence.attribution = None;
        }
        Mutation::RealEvidenceTaxonMismatch => {
            profile.capabilities[0].evidence.taxon_id = Some("TEST:other".to_owned());
        }
        Mutation::RealEvidenceTaxonMissing => {
            profile.capabilities[0].evidence.taxon_id = None;
        }
        Mutation::RealStageRequirementMismatch => {
            profile.capabilities[0].stage_requirement = Some("larva".to_owned());
        }
        Mutation::RealEvidenceStageMismatch => {
            profile.capabilities[0].evidence.stage = Some("larva".to_owned());
        }
        Mutation::RealUnknownCapability => {
            profile.capabilities[0].state = EvidenceState::Unknown;
        }
        Mutation::RealNotMeasuredCapability => {
            profile.capabilities[0].state = EvidenceState::NotMeasured;
        }
        Mutation::RealConflictedCapability => {
            profile.capabilities[0].state = EvidenceState::Conflicted;
        }
        Mutation::RealNoCapabilityClaims => profile.capabilities.clear(),
        Mutation::RealWhitespaceSourceRef => {
            profile.capabilities[0].evidence.source_ref = Some("  ".to_owned());
        }
        Mutation::RealUnknownRightsForAbsence => {
            profile.capabilities[0].state = EvidenceState::Absent;
            profile.capabilities[0].evidence.permission = UsePermission::Unknown;
        }
        Mutation::RealRestrictedRightsUnasserted => {
            profile.capabilities[0].state = EvidenceState::Unknown;
            profile.capabilities[0].evidence.permission = UsePermission::Restricted;
        }
        Mutation::RealUnknownRightsUnasserted => {
            profile.capabilities[0].state = EvidenceState::Unknown;
            profile.capabilities[0].evidence.permission = UsePermission::Unknown;
        }
        Mutation::RealMissingPermissionScope => {
            profile.capabilities[0].evidence.permission_scope = None;
        }
        Mutation::RealWhitespacePermissionScope => {
            profile.capabilities[0].evidence.permission_scope = Some("  ".to_owned());
        }
        Mutation::RealAttributionWhitespace => {
            profile.capabilities[0].evidence.permission = UsePermission::AttributionRequired;
            profile.capabilities[0].evidence.attribution = Some("  ".to_owned());
        }
        Mutation::RealAbsentClaimValid => {
            profile.capabilities[0].state = EvidenceState::Absent;
        }
        Mutation::RealNotApplicableClaimValid => {
            profile.capabilities[0].state = EvidenceState::NotApplicable;
        }
        Mutation::RealModeledEvidenceValid => {
            profile.capabilities[0].evidence.class = EvidenceClass::Modeled;
        }
        Mutation::RealReconstructedEvidenceValid => {
            profile.capabilities[0].evidence.class = EvidenceClass::Reconstructed;
        }
        Mutation::SyntheticAbsentClaimValid => {
            profile.capabilities[0].state = EvidenceState::Absent;
        }
        Mutation::SyntheticUnknownClaimValid => {
            profile.capabilities[0].state = EvidenceState::Unknown;
        }
        Mutation::RealAttributionPresent => {
            profile.capabilities[0].evidence.permission = UsePermission::AttributionRequired;
            profile.capabilities[0].evidence.attribution =
                Some("citation:synthetic-test".to_owned());
        }
        Mutation::RealStageRequirementMatched => {
            profile.capabilities[0].stage_requirement = Some("adult".to_owned());
        }
        Mutation::RealTaxonSourceWhitespace => {
            profile.taxon.as_mut().unwrap().source = "  ".to_owned();
        }
        Mutation::RealTaxonIdWhitespace => {
            profile.taxon.as_mut().unwrap().taxon_id = "  ".to_owned();
        }
    }
    profile
}

fn real_profile(id: &str, taxon_id: &str) -> AnimalProfile {
    let mut profile = AnimalProfile {
        id: id.to_owned(),
        kind: symtropy_fauna_core::ProfileKind::RealTaxonBound,
        taxon: Some(TaxonBinding {
            source: "test-taxonomy".to_owned(),
            release: "test-release-1".to_owned(),
            taxon_id: taxon_id.to_owned(),
        }),
        stage: Some("adult".to_owned()),
        provenance: EvidenceClass::Modeled,
        capabilities: Vec::new(),
    };
    profile.capabilities.push(CapabilityClaim {
        axis: "locomotion".to_owned(),
        key: "walking".to_owned(),
        state: EvidenceState::Present,
        evidence: symtropy_fauna_core::EvidenceRecord {
            class: EvidenceClass::Modeled,
            source_ref: Some("test://synthetic-validation-fixture/reference".to_owned()),
            permission: UsePermission::Granted,
            permission_scope: Some("test-fixture-only".to_owned()),
            attribution: None,
            taxon_id: Some(taxon_id.to_owned()),
            stage: Some("adult".to_owned()),
        },
        stage_requirement: None,
    });
    profile
}

#[test]
fn frozen_profile_oracle_matches_all_declared_case_results() {
    let fixture: CorpusFixture = serde_json::from_str(FIXTURE).expect("valid frozen JSON corpus");
    assert_eq!(fixture.schema_version, "ANIMAL-CAPABILITY-PROFILES-V1");
    assert_eq!(fixture.archetypes.len(), 26);
    assert!(fixture.cases.len() >= 40);

    for archetype in &fixture.archetypes {
        assert_eq!(
            validate_profile(archetype).disposition,
            ValidationDisposition::Eligible,
            "synthetic archetype {} must be structurally admissible",
            archetype.id
        );
    }

    for case in &fixture.cases {
        let base = match case.base_profile.as_str() {
            "synthetic" => fixture.synthetic_template.clone(),
            "real" => fixture.real_template.clone(),
            other => panic!("unknown base profile {other} in case {}", case.id),
        };
        let profile = apply_mutation(base, case.mutation.clone());
        let result = validate_profile(&profile);
        assert_eq!(
            result.disposition, case.expected_disposition,
            "wrong disposition for fixture case {}: {:?}",
            case.id, result.reason_codes
        );
        assert_eq!(
            result.reason_codes, case.expected_reason_codes,
            "wrong reason codes for fixture case {}",
            case.id
        );
    }
}

#[test]
fn validation_order_is_deterministic_and_duplicate_ids_fail_closed() {
    let a = fixture_profile("synthetic/a");
    let b = fixture_profile("synthetic/b");
    let forward = validate_profiles(&[a.clone(), b.clone()]);
    let backward = validate_profiles(&[b, a]);
    assert_eq!(forward, backward);

    let repeated = fixture_profile("synthetic/duplicate");
    let results = validate_profiles(&[repeated.clone(), repeated]);
    assert!(results.iter().all(|result| {
        result.disposition == ValidationDisposition::Reject
            && result
                .reason_codes
                .contains(&ReasonCode::DuplicateProfileId)
    }));
}

fn fixture_profile(id: &str) -> AnimalProfile {
    let fixture: CorpusFixture = serde_json::from_str(FIXTURE).expect("valid frozen JSON corpus");
    let mut profile = fixture.synthetic_template;
    profile.id = id.to_owned();
    profile
}

#[test]
fn corpus_digest_is_order_independent_but_evidence_sensitive() {
    let fixture: CorpusFixture = serde_json::from_str(FIXTURE).expect("valid frozen JSON corpus");
    let profiles = fixture.archetypes;
    let mut reversed = profiles.clone();
    reversed.reverse();
    assert_eq!(
        corpus_sha256(&profiles).unwrap(),
        corpus_sha256(&reversed).unwrap()
    );

    let mut altered = profiles.clone();
    altered[0].capabilities[0].key.push_str("_changed");
    assert_ne!(
        corpus_sha256(&profiles).unwrap(),
        corpus_sha256(&altered).unwrap()
    );
}

#[test]
fn coverage_scope_cannot_hide_unreviewed_taxa_or_promote_invalid_profiles() {
    let scope = TaxonomyScope {
        source: "test-taxonomy".to_owned(),
        release: "test-release-1".to_owned(),
        taxon_ids: vec!["TEST:1".to_owned(), "TEST:2".to_owned()],
    };
    let declarations = vec![
        CoverageDeclaration {
            taxon_id: "TEST:1".to_owned(),
            state: CoverageState::Covered,
        },
        CoverageDeclaration {
            taxon_id: "TEST:2".to_owned(),
            state: CoverageState::Covered,
        },
    ];
    let valid = vec![
        real_profile("real/1", "TEST:1"),
        real_profile("real/2", "TEST:2"),
    ];
    assert!(
        coverage_report(&scope, &declarations, &valid)
            .unwrap()
            .profile_registry_complete
    );
    assert_eq!(
        coverage_report(&scope, &declarations, &[real_profile("real/1", "TEST:1")]),
        Err(CoverageError::CoveredWithoutEligibleProfile)
    );
}

#[test]
fn coverage_rejects_duplicate_and_out_of_scope_declarations() {
    let scope = TaxonomyScope {
        source: "test-taxonomy".to_owned(),
        release: "test-release-1".to_owned(),
        taxon_ids: vec!["TEST:1".to_owned()],
    };
    let duplicate = vec![
        CoverageDeclaration {
            taxon_id: "TEST:1".to_owned(),
            state: CoverageState::Covered,
        },
        CoverageDeclaration {
            taxon_id: "TEST:1".to_owned(),
            state: CoverageState::ProfileInProgress,
        },
    ];
    assert_eq!(
        coverage_report(&scope, &duplicate, &[real_profile("real/1", "TEST:1")]),
        Err(CoverageError::DuplicateDeclaration)
    );
    let outside = vec![CoverageDeclaration {
        taxon_id: "TEST:outside".to_owned(),
        state: CoverageState::Covered,
    }];
    assert_eq!(
        coverage_report(&scope, &outside, &[]),
        Err(CoverageError::DeclarationOutsideScope)
    );
}

#[test]
fn coverage_rejects_profiles_from_a_different_taxonomy_source_or_release() {
    let declarations = vec![CoverageDeclaration {
        taxon_id: "TEST:1".to_owned(),
        state: CoverageState::Covered,
    }];
    let profiles = vec![real_profile("real/1", "TEST:1")];

    let wrong_source = TaxonomyScope {
        source: "another-taxonomy".to_owned(),
        release: "test-release-1".to_owned(),
        taxon_ids: vec!["TEST:1".to_owned()],
    };
    assert_eq!(
        coverage_report(&wrong_source, &declarations, &profiles),
        Err(CoverageError::CoveredWithoutEligibleProfile)
    );

    let wrong_release = TaxonomyScope {
        source: "test-taxonomy".to_owned(),
        release: "another-release".to_owned(),
        taxon_ids: vec!["TEST:1".to_owned()],
    };
    assert_eq!(
        coverage_report(&wrong_release, &declarations, &profiles),
        Err(CoverageError::CoveredWithoutEligibleProfile)
    );
}

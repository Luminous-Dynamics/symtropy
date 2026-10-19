// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-aware, dependency-light animal capability profiles.
//!
//! This crate validates claims about simulated capabilities. It does not infer
//! biology from taxonomy, implement animal behavior, or establish empirical
//! realism. Renderers and simulation backends should consume validated profiles
//! without becoming authorities for taxonomy or evidence.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    SyntheticArchetype,
    RealTaxonBound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    Reconstructed,
    Modeled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Present,
    Absent,
    Unknown,
    NotMeasured,
    NotApplicable,
    Conflicted,
}

impl EvidenceState {
    fn is_assertion(self) -> bool {
        matches!(self, Self::Present | Self::Absent)
    }

    fn is_unresolved(self) -> bool {
        matches!(self, Self::Unknown | Self::NotMeasured | Self::Conflicted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsePermission {
    SyntheticOnly,
    Granted,
    AttributionRequired,
    Unknown,
    Restricted,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaxonBinding {
    pub source: String,
    pub release: String,
    pub taxon_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub class: EvidenceClass,
    pub source_ref: Option<String>,
    pub permission: UsePermission,
    pub permission_scope: Option<String>,
    pub attribution: Option<String>,
    pub taxon_id: Option<String>,
    pub stage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CapabilityClaim {
    pub axis: String,
    pub key: String,
    pub state: EvidenceState,
    pub evidence: EvidenceRecord,
    pub stage_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimalProfile {
    pub id: String,
    pub kind: ProfileKind,
    pub taxon: Option<TaxonBinding>,
    pub stage: Option<String>,
    pub provenance: EvidenceClass,
    pub capabilities: Vec<CapabilityClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationDisposition {
    Eligible,
    Indeterminate,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    EmptyProfileId,
    DuplicateProfileId,
    SyntheticProfileHasTaxon,
    SyntheticProvenanceMismatch,
    RealProfileUsesSyntheticProvenance,
    MissingTaxonBinding,
    MissingTaxonSource,
    MissingTaxonRelease,
    MissingTaxonId,
    NoCapabilityClaims,
    EmptyCapabilityAxis,
    EmptyCapabilityKey,
    DuplicateCapabilityKey,
    SyntheticEvidenceForRealTaxon,
    SyntheticOnlyEvidenceForRealTaxon,
    EvidenceRightsUnknown,
    EvidenceRightsRestricted,
    MissingRequiredAttribution,
    MissingPermissionScope,
    MissingEvidenceSource,
    MissingEvidenceTaxonId,
    EvidenceTaxonMismatch,
    StageRequirementMismatch,
    EvidenceStageMismatch,
    UnresolvedCapabilityEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub profile_id: String,
    pub disposition: ValidationDisposition,
    pub reason_codes: Vec<ReasonCode>,
}

impl ValidationResult {
    fn new(
        profile_id: &str,
        rejected: &BTreeSet<ReasonCode>,
        indeterminate: &BTreeSet<ReasonCode>,
    ) -> Self {
        let disposition = if !rejected.is_empty() {
            ValidationDisposition::Reject
        } else if !indeterminate.is_empty() {
            ValidationDisposition::Indeterminate
        } else {
            ValidationDisposition::Eligible
        };
        let reasons = rejected.union(indeterminate).copied().collect();
        Self {
            profile_id: profile_id.to_owned(),
            disposition,
            reason_codes: reasons,
        }
    }

    fn add_reason(&mut self, reason: ReasonCode) {
        if !self.reason_codes.contains(&reason) {
            self.reason_codes.push(reason);
            self.reason_codes.sort();
        }
        self.disposition = ValidationDisposition::Reject;
    }
}

/// Validate structural and evidence boundaries without inferring unsupported biology.
///
/// A real-taxon profile may be structurally incomplete (indeterminate) or
/// ineligible for use (reject). Neither state is silently promoted to eligible.
pub fn validate_profile(profile: &AnimalProfile) -> ValidationResult {
    let mut rejected = BTreeSet::new();
    let mut indeterminate = BTreeSet::new();

    if profile.id.trim().is_empty() {
        rejected.insert(ReasonCode::EmptyProfileId);
    }

    match profile.kind {
        ProfileKind::SyntheticArchetype => {
            if profile.taxon.is_some() {
                rejected.insert(ReasonCode::SyntheticProfileHasTaxon);
            }
            if profile.provenance != EvidenceClass::Synthetic {
                rejected.insert(ReasonCode::SyntheticProvenanceMismatch);
            }
        }
        ProfileKind::RealTaxonBound => {
            if profile.provenance == EvidenceClass::Synthetic {
                rejected.insert(ReasonCode::RealProfileUsesSyntheticProvenance);
            }
            match profile.taxon.as_ref() {
                None => {
                    indeterminate.insert(ReasonCode::MissingTaxonBinding);
                }
                Some(taxon) => {
                    if taxon.source.trim().is_empty() {
                        indeterminate.insert(ReasonCode::MissingTaxonSource);
                    }
                    if taxon.release.trim().is_empty() {
                        indeterminate.insert(ReasonCode::MissingTaxonRelease);
                    }
                    if taxon.taxon_id.trim().is_empty() {
                        indeterminate.insert(ReasonCode::MissingTaxonId);
                    }
                }
            }
        }
    }

    if profile.capabilities.is_empty() {
        indeterminate.insert(ReasonCode::NoCapabilityClaims);
    }

    let mut capability_keys = BTreeSet::new();
    for claim in &profile.capabilities {
        if claim.axis.trim().is_empty() {
            rejected.insert(ReasonCode::EmptyCapabilityAxis);
        }
        if claim.key.trim().is_empty() {
            rejected.insert(ReasonCode::EmptyCapabilityKey);
        }
        let capability_identity = (
            claim.axis.trim().to_lowercase(),
            claim.key.trim().to_lowercase(),
            claim
                .stage_requirement
                .as_deref()
                .map(str::trim)
                .map(str::to_lowercase),
        );
        if !capability_keys.insert(capability_identity) {
            rejected.insert(ReasonCode::DuplicateCapabilityKey);
        }

        if let Some(required_stage) = claim.stage_requirement.as_deref()
            && profile.stage.as_deref() != Some(required_stage)
        {
            indeterminate.insert(ReasonCode::StageRequirementMismatch);
        }
        if let Some(evidence_stage) = claim.evidence.stage.as_deref()
            && profile.stage.as_deref() != Some(evidence_stage)
        {
            indeterminate.insert(ReasonCode::EvidenceStageMismatch);
        }

        // Unknown/non-applicable records describe limits of knowledge, not
        // assertions that the capability exists or is absent.
        if claim.state.is_unresolved() && profile.kind == ProfileKind::RealTaxonBound {
            indeterminate.insert(ReasonCode::UnresolvedCapabilityEvidence);
        }

        if profile.kind == ProfileKind::RealTaxonBound {
            if claim.evidence.class == EvidenceClass::Synthetic {
                rejected.insert(ReasonCode::SyntheticEvidenceForRealTaxon);
            }
            match claim.evidence.permission {
                UsePermission::SyntheticOnly => {
                    rejected.insert(ReasonCode::SyntheticOnlyEvidenceForRealTaxon);
                }
                UsePermission::Unknown => {
                    rejected.insert(ReasonCode::EvidenceRightsUnknown);
                }
                UsePermission::Restricted => {
                    rejected.insert(ReasonCode::EvidenceRightsRestricted);
                }
                UsePermission::Granted | UsePermission::AttributionRequired => {
                    if claim
                        .evidence
                        .permission_scope
                        .as_deref()
                        .is_none_or(|scope| scope.trim().is_empty())
                    {
                        indeterminate.insert(ReasonCode::MissingPermissionScope);
                    }
                    if claim.evidence.permission == UsePermission::AttributionRequired
                        && claim
                            .evidence
                            .attribution
                            .as_deref()
                            .is_none_or(|attribution| attribution.trim().is_empty())
                    {
                        rejected.insert(ReasonCode::MissingRequiredAttribution);
                    }
                }
            }

            if claim.state.is_assertion() {
                if claim
                    .evidence
                    .source_ref
                    .as_deref()
                    .is_none_or(|source| source.trim().is_empty())
                {
                    indeterminate.insert(ReasonCode::MissingEvidenceSource);
                }

                match (
                    profile.taxon.as_ref().map(|taxon| taxon.taxon_id.trim()),
                    claim.evidence.taxon_id.as_deref().map(str::trim),
                ) {
                    (_, None) | (_, Some("")) => {
                        indeterminate.insert(ReasonCode::MissingEvidenceTaxonId);
                    }
                    (Some(profile_taxon), Some(evidence_taxon))
                        if !profile_taxon.is_empty() && profile_taxon != evidence_taxon =>
                    {
                        rejected.insert(ReasonCode::EvidenceTaxonMismatch);
                    }
                    _ => {}
                }
            }
        }
    }

    ValidationResult::new(&profile.id, &rejected, &indeterminate)
}

/// Validate a profile set and reject every member of a duplicate-ID group.
/// Results are sorted canonically so caller input order is not semantically relevant.
pub fn validate_profiles(profiles: &[AnimalProfile]) -> Vec<ValidationResult> {
    let mut counts = BTreeMap::<String, usize>::new();
    for profile in profiles {
        *counts.entry(profile.id.clone()).or_default() += 1;
    }

    let mut results = profiles
        .iter()
        .map(|profile| {
            let mut result = validate_profile(profile);
            if counts.get(&profile.id).copied().unwrap_or_default() > 1 {
                result.add_reason(ReasonCode::DuplicateProfileId);
            }
            result
        })
        .collect::<Vec<_>>();
    results.sort_by(|a, b| {
        a.profile_id.cmp(&b.profile_id).then_with(|| {
            serde_json::to_string(a)
                .unwrap_or_default()
                .cmp(&serde_json::to_string(b).unwrap_or_default())
        })
    });
    results
}

/// Return the lowercase SHA-256 digest of arbitrary bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

/// Hash a profile corpus after sorting profiles and each profile's claims.
/// This makes the digest insensitive to caller ordering while still binding all
/// serialized profile fields and evidence metadata.
pub fn corpus_sha256(profiles: &[AnimalProfile]) -> Result<String, serde_json::Error> {
    let mut canonical = profiles.to_vec();
    for profile in &mut canonical {
        profile.capabilities.sort();
    }
    canonical.sort_by(|a, b| {
        a.id.cmp(&b.id).then_with(|| {
            serde_json::to_string(a)
                .unwrap_or_default()
                .cmp(&serde_json::to_string(b).unwrap_or_default())
        })
    });
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(sha256_hex(&bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageState {
    Covered,
    ProfileInProgress,
    Unsupported,
    NotReviewed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyScope {
    pub source: String,
    pub release: String,
    pub taxon_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageDeclaration {
    pub taxon_id: String,
    pub state: CoverageState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonCoverage {
    pub taxon_id: String,
    pub state: CoverageState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageReport {
    pub taxonomy_source: String,
    pub taxonomy_release: String,
    pub taxon_count: usize,
    pub covered: usize,
    pub in_progress: usize,
    pub unsupported: usize,
    pub not_reviewed: usize,
    pub profile_registry_complete: bool,
    pub taxa: Vec<TaxonCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageError {
    MissingTaxonomySource,
    MissingTaxonomyRelease,
    EmptyTaxonId,
    DuplicateTaxonId,
    DuplicateDeclaration,
    DeclarationOutsideScope,
    CoveredWithoutEligibleProfile,
}

/// Build a report for a pinned, explicitly selected taxonomy scope.
///
/// Covered means profile-registry coverage: a real-taxon profile for that taxon
/// must pass this crate's structural/evidence gates. It is not a claim that the
/// species' behavior or biology is comprehensively simulated. The completeness
/// flag applies only to this explicit, pinned registry scope.
pub fn coverage_report(
    scope: &TaxonomyScope,
    declarations: &[CoverageDeclaration],
    profiles: &[AnimalProfile],
) -> Result<CoverageReport, CoverageError> {
    if scope.source.trim().is_empty() {
        return Err(CoverageError::MissingTaxonomySource);
    }
    if scope.release.trim().is_empty() {
        return Err(CoverageError::MissingTaxonomyRelease);
    }

    let mut scoped_ids = BTreeSet::new();
    for taxon_id in &scope.taxon_ids {
        if taxon_id.trim().is_empty() {
            return Err(CoverageError::EmptyTaxonId);
        }
        if !scoped_ids.insert(taxon_id.clone()) {
            return Err(CoverageError::DuplicateTaxonId);
        }
    }

    let mut declared = BTreeMap::new();
    for declaration in declarations {
        if !scoped_ids.contains(&declaration.taxon_id) {
            return Err(CoverageError::DeclarationOutsideScope);
        }
        if declared
            .insert(declaration.taxon_id.clone(), declaration.state)
            .is_some()
        {
            return Err(CoverageError::DuplicateDeclaration);
        }
    }

    let eligible_taxa = profiles
        .iter()
        .filter(|profile| profile.kind == ProfileKind::RealTaxonBound)
        .filter(|profile| validate_profile(profile).disposition == ValidationDisposition::Eligible)
        .filter(|profile| {
            profile
                .taxon
                .as_ref()
                .is_some_and(|taxon| taxon.source == scope.source && taxon.release == scope.release)
        })
        .filter_map(|profile| profile.taxon.as_ref().map(|taxon| taxon.taxon_id.clone()))
        .collect::<BTreeSet<_>>();

    let mut taxa = scope
        .taxon_ids
        .iter()
        .map(|taxon_id| TaxonCoverage {
            taxon_id: taxon_id.clone(),
            state: declared
                .get(taxon_id)
                .copied()
                .unwrap_or(CoverageState::NotReviewed),
        })
        .collect::<Vec<_>>();
    taxa.sort_by(|a, b| a.taxon_id.cmp(&b.taxon_id));

    for taxon in &taxa {
        if taxon.state == CoverageState::Covered && !eligible_taxa.contains(&taxon.taxon_id) {
            return Err(CoverageError::CoveredWithoutEligibleProfile);
        }
    }

    let covered = taxa
        .iter()
        .filter(|taxon| taxon.state == CoverageState::Covered)
        .count();
    let in_progress = taxa
        .iter()
        .filter(|taxon| taxon.state == CoverageState::ProfileInProgress)
        .count();
    let unsupported = taxa
        .iter()
        .filter(|taxon| taxon.state == CoverageState::Unsupported)
        .count();
    let not_reviewed = taxa
        .iter()
        .filter(|taxon| taxon.state == CoverageState::NotReviewed)
        .count();

    Ok(CoverageReport {
        taxonomy_source: scope.source.clone(),
        taxonomy_release: scope.release.clone(),
        taxon_count: taxa.len(),
        covered,
        in_progress,
        unsupported,
        not_reviewed,
        profile_registry_complete: !taxa.is_empty() && covered == taxa.len(),
        taxa,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(class: EvidenceClass, taxon_id: Option<&str>) -> EvidenceRecord {
        EvidenceRecord {
            class,
            source_ref: Some("test://evidence/1".to_owned()),
            permission: if class == EvidenceClass::Synthetic {
                UsePermission::SyntheticOnly
            } else {
                UsePermission::Granted
            },
            permission_scope: if class == EvidenceClass::Synthetic {
                None
            } else {
                Some("test-fixture-only".to_owned())
            },
            attribution: None,
            taxon_id: taxon_id.map(str::to_owned),
            stage: Some("adult".to_owned()),
        }
    }

    fn real_profile(id: &str, taxon_id: &str) -> AnimalProfile {
        AnimalProfile {
            id: id.to_owned(),
            kind: ProfileKind::RealTaxonBound,
            taxon: Some(TaxonBinding {
                source: "test-taxonomy".to_owned(),
                release: "test-release-1".to_owned(),
                taxon_id: taxon_id.to_owned(),
            }),
            stage: Some("adult".to_owned()),
            provenance: EvidenceClass::Measured,
            capabilities: vec![CapabilityClaim {
                axis: "locomotion".to_owned(),
                key: "walking".to_owned(),
                state: EvidenceState::Present,
                evidence: evidence(EvidenceClass::Measured, Some(taxon_id)),
                stage_requirement: None,
            }],
        }
    }

    fn synthetic_profile(id: &str) -> AnimalProfile {
        AnimalProfile {
            id: id.to_owned(),
            kind: ProfileKind::SyntheticArchetype,
            taxon: None,
            stage: Some("adult".to_owned()),
            provenance: EvidenceClass::Synthetic,
            capabilities: vec![CapabilityClaim {
                axis: "body_plan".to_owned(),
                key: "archetype_shape".to_owned(),
                state: EvidenceState::Present,
                evidence: evidence(EvidenceClass::Synthetic, None),
                stage_requirement: None,
            }],
        }
    }

    #[test]
    fn known_sha256_vector_is_correct() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn valid_synthetic_and_real_profiles_are_eligible() {
        assert_eq!(
            validate_profile(&synthetic_profile("synthetic/a")).disposition,
            ValidationDisposition::Eligible
        );
        assert_eq!(
            validate_profile(&real_profile("real/a", "TEST:1")).disposition,
            ValidationDisposition::Eligible
        );
    }

    #[test]
    fn profile_corpus_digest_ignores_input_order() {
        let a = synthetic_profile("synthetic/a");
        let b = synthetic_profile("synthetic/b");
        assert_eq!(
            corpus_sha256(&[a.clone(), b.clone()]).unwrap(),
            corpus_sha256(&[b, a]).unwrap()
        );
    }

    #[test]
    fn profile_corpus_digest_binds_evidence_changes() {
        let a = real_profile("real/a", "TEST:1");
        let mut b = a.clone();
        b.capabilities[0].evidence.source_ref = Some("test://evidence/2".to_owned());
        assert_ne!(corpus_sha256(&[a]).unwrap(), corpus_sha256(&[b]).unwrap());
    }

    #[test]
    fn duplicate_profile_ids_reject_all_members_of_group() {
        let profiles = vec![
            synthetic_profile("duplicate"),
            synthetic_profile("duplicate"),
        ];
        let results = validate_profiles(&profiles);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|result| {
            result.disposition == ValidationDisposition::Reject
                && result
                    .reason_codes
                    .contains(&ReasonCode::DuplicateProfileId)
        }));
    }

    #[test]
    fn coverage_defaults_omitted_taxa_to_not_reviewed() {
        let scope = TaxonomyScope {
            source: "test-taxonomy".to_owned(),
            release: "test-release-1".to_owned(),
            taxon_ids: vec!["TEST:1".to_owned(), "TEST:2".to_owned()],
        };
        let declarations = vec![CoverageDeclaration {
            taxon_id: "TEST:1".to_owned(),
            state: CoverageState::Covered,
        }];
        let report =
            coverage_report(&scope, &declarations, &[real_profile("real/1", "TEST:1")]).unwrap();
        assert_eq!(report.covered, 1);
        assert_eq!(report.not_reviewed, 1);
        assert!(!report.profile_registry_complete);
    }

    #[test]
    fn coverage_cannot_be_claimed_without_an_eligible_profile() {
        let scope = TaxonomyScope {
            source: "test-taxonomy".to_owned(),
            release: "test-release-1".to_owned(),
            taxon_ids: vec!["TEST:1".to_owned()],
        };
        let declarations = vec![CoverageDeclaration {
            taxon_id: "TEST:1".to_owned(),
            state: CoverageState::Covered,
        }];
        assert_eq!(
            coverage_report(&scope, &declarations, &[]),
            Err(CoverageError::CoveredWithoutEligibleProfile)
        );
    }

    #[test]
    fn coverage_is_complete_only_when_every_scoped_taxon_is_covered() {
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
        let profiles = vec![
            real_profile("real/1", "TEST:1"),
            real_profile("real/2", "TEST:2"),
        ];
        let report = coverage_report(&scope, &declarations, &profiles).unwrap();
        assert_eq!(report.covered, 2);
        assert!(report.profile_registry_complete);
    }
}

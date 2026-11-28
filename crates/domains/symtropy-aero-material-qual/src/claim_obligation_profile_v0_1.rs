// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Derive an immutable obligation profile from a formally assessed claim spec.
//!
//! This module derives *requirements*, not evidence states. It does not mark an
//! obligation satisfied, authenticate owner receipts, determine producer
//! authority, evaluate VVUQ, assess physical truth, or grant engineering or
//! physical-work authority. The existing claim gate and source-owner evaluators
//! remain responsible for those assessments.

use super::assurance_disposition_v0_1::ObligationKindV0_1;
use super::claim_specification_v0_1::{
    ClaimSpecificationDispositionV0_1, ClaimSpecificationV0_1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CLAIM_OBLIGATION_PROFILE_SCHEMA_V0_1: &str = "AeroClaimObligationProfileV0_1";

/// Closed registry of requirement profiles supported by this module version.
///
/// Adding a new profile requires a deliberate code/schema/version change and
/// independent review; callers cannot submit a policy blob that changes the
/// mandatory or hard-gate status of individual requirements.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RegisteredClaimRequirementProfileV0_1 {
    #[serde(rename = "AERO_SCALAR_ENGINEERING_CLAIM_V0_1")]
    AeroScalarEngineeringClaimV0_1,
}

impl RegisteredClaimRequirementProfileV0_1 {
    pub const fn profile_id(self) -> &'static str {
        match self {
            Self::AeroScalarEngineeringClaimV0_1 => "aero-scalar-engineering-claim-v0.1",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimObligationProfileDispositionV0_1 {
    #[serde(rename = "DERIVED")]
    Derived,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "UNSUPPORTED_PROFILE")]
    UnsupportedProfile,
}

impl ClaimObligationProfileDispositionV0_1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Derived => "DERIVED",
            Self::Incomplete => "INCOMPLETE",
            Self::Rejected => "REJECTED",
            Self::UnsupportedProfile => "UNSUPPORTED_PROFILE",
        }
    }
}

/// One requirement derived from a source field or registered policy rule.
/// The profile never contains a proposition-evidence or verification state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedClaimObligationV0_1 {
    pub obligation_id: String,
    pub kind: ObligationKindV0_1,
    pub mandatory: bool,
    pub hard_gate: bool,
    pub requirement_source_ref: String,
    pub scoped_subject_ref: String,
}

/// Complete derived profile for one exact semantic claim commitment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedClaimObligationProfileV0_1 {
    pub schema: String,
    pub profile_id: String,
    pub claim_id: String,
    pub claim_specification_semantic_sha256: String,
    pub profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub obligations: Vec<DerivedClaimObligationV0_1>,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub execution_authority_granted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimObligationProfileAssessmentV0_1 {
    pub disposition: ClaimObligationProfileDispositionV0_1,
    pub profile: Option<DerivedClaimObligationProfileV0_1>,
    pub reason_codes: Vec<String>,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub execution_authority_granted: bool,
}

#[derive(Serialize)]
struct ProfileCommitmentEnvelopeV0_1 {
    schema: &'static str,
    canonicalization_profile: &'static str,
    profile_id: &'static str,
    required_rules: Vec<(&'static str, &'static str, bool, bool)>,
    strata_rule: (&'static str, bool, bool),
    evidence_class_rule: (&'static str, bool, bool),
    uncertainty_rule: (&'static str, bool, bool),
    defeater_rule: (&'static str, bool, bool),
}

#[derive(Serialize)]
struct ObligationManifestEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    claim_specification_semantic_sha256: &'a str,
    profile_sha256: &'a str,
    obligations: &'a [DerivedClaimObligationV0_1],
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn profile_sha256(profile: RegisteredClaimRequirementProfileV0_1) -> String {
    let envelope = ProfileCommitmentEnvelopeV0_1 {
        schema: "AeroClaimRequirementProfileCommitmentV1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        profile_id: profile.profile_id(),
        required_rules: vec![
            ("claim_identity", "CLAIM_IDENTITY", true, true),
            ("semantic_validity", "SEMANTIC_VALIDITY", true, true),
            ("producer_authority", "PRODUCER_AUTHORITY", true, true),
            ("model_applicability", "MODEL_APPLICABILITY", true, true),
            ("temporal_knowledge", "TEMPORAL_KNOWLEDGE", true, true),
            ("dependency_observation", "DEPENDENCY_OBSERVATION", true, true),
        ],
        strata_rule: ("MODEL_CREDIBILITY_PER_REQUIRED_STRATUM", true, true),
        evidence_class_rule: ("EVIDENCE_INTEGRITY_PER_REQUIRED_CLASS", true, true),
        uncertainty_rule: ("MODEL_CREDIBILITY_FOR_DECLARED_UNCERTAINTY", true, true),
        defeater_rule: ("DEFEATER_RESOLUTION_FOR_ASSUMPTIONS_AND_EXCLUSIONS", false, false),
    };
    sha256_hex(
        &serde_json::to_vec(&envelope)
            .expect("closed obligation profile is serializable"),
    )
}

fn add_obligation(
    output: &mut Vec<DerivedClaimObligationV0_1>,
    id: String,
    kind: ObligationKindV0_1,
    mandatory: bool,
    hard_gate: bool,
    source_ref: String,
    scoped_subject_ref: String,
) {
    output.push(DerivedClaimObligationV0_1 {
        obligation_id: id,
        kind,
        mandatory,
        hard_gate,
        requirement_source_ref: source_ref,
        scoped_subject_ref,
    });
}

fn derive_obligations(
    specification: &ClaimSpecificationV0_1,
) -> Vec<DerivedClaimObligationV0_1> {
    let mut obligations = Vec::new();

    for (id, kind, source_ref) in [
        (
            "claim-identity",
            ObligationKindV0_1::ClaimIdentity,
            "claim-spec:/claim_id+subject_ref+configuration_ref",
        ),
        (
            "semantic-validity",
            ObligationKindV0_1::SemanticValidity,
            "claim-spec:/measurand+predicate+population+intended_use_ref",
        ),
        (
            "producer-authority",
            ObligationKindV0_1::ProducerAuthority,
            "claim-spec:/acceptance_authority_role_ref",
        ),
        (
            "model-applicability",
            ObligationKindV0_1::ModelApplicability,
            "claim-spec:/operating_envelope_ref+intended_use_ref",
        ),
        (
            "temporal-knowledge",
            ObligationKindV0_1::TemporalKnowledge,
            "claim-spec:/validity",
        ),
        (
            "dependency-observation",
            ObligationKindV0_1::DependencyObservation,
            "claim-spec:/subject_ref+configuration_ref+validity",
        ),
    ] {
        add_obligation(
            &mut obligations,
            id.to_owned(),
            kind,
            true,
            true,
            source_ref.to_owned(),
            specification.subject_ref.clone(),
        );
    }

    for evidence_class in &specification.required_evidence_classes {
        add_obligation(
            &mut obligations,
            format!("evidence-integrity:{evidence_class}"),
            ObligationKindV0_1::EvidenceIntegrity,
            true,
            true,
            format!("claim-spec:/required_evidence_classes/{evidence_class}"),
            specification.subject_ref.clone(),
        );
    }

    for stratum in &specification.population.required_strata {
        add_obligation(
            &mut obligations,
            format!("model-credibility:{}", stratum.stratum_id),
            ObligationKindV0_1::ModelCredibility,
            true,
            true,
            format!("claim-spec:/population/required_strata/{}", stratum.stratum_id),
            format!(
                "{}|{}|{}",
                specification.population.population_ref,
                stratum.dimension_ref,
                stratum.level_ref
            ),
        );
    }

    add_obligation(
        &mut obligations,
        "model-uncertainty".to_owned(),
        ObligationKindV0_1::ModelCredibility,
        true,
        true,
        format!(
            "claim-spec:/uncertainty/treatment+method_ref:{}",
            specification.uncertainty.method_ref
        ),
        specification.subject_ref.clone(),
    );

    for assumption in &specification.assumptions {
        add_obligation(
            &mut obligations,
            format!("assumption-defeater:{assumption}"),
            ObligationKindV0_1::DefeaterResolution,
            false,
            false,
            format!("claim-spec:/assumptions/{assumption}"),
            specification.subject_ref.clone(),
        );
    }
    for exclusion in &specification.exclusions {
        add_obligation(
            &mut obligations,
            format!("exclusion-defeater:{exclusion}"),
            ObligationKindV0_1::DefeaterResolution,
            false,
            false,
            format!("claim-spec:/exclusions/{exclusion}"),
            specification.subject_ref.clone(),
        );
    }

    obligations.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    obligations
}

fn obligation_manifest_sha256(
    claim_id: &str,
    semantic_sha256: &str,
    profile_digest: &str,
    obligations: &[DerivedClaimObligationV0_1],
) -> String {
    let envelope = ObligationManifestEnvelopeV0_1 {
        schema: "AeroDerivedClaimObligationManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        claim_id,
        claim_specification_semantic_sha256: semantic_sha256,
        profile_sha256: profile_digest,
        obligations,
    };
    sha256_hex(
        &serde_json::to_vec(&envelope)
            .expect("derived obligation manifest is serializable"),
    )
}

/// Derive mandatory obligations from one formally assessed, typed scalar claim.
///
/// The closed profile registry, not the caller, supplies mandatory/hard-gate
/// policy. The returned obligations intentionally contain no evidence status;
/// they cannot be used as evidence of satisfaction or acceptance.
pub fn derive_claim_obligation_profile_v0_1(
    specification: &ClaimSpecificationV0_1,
    profile: RegisteredClaimRequirementProfileV0_1,
) -> ClaimObligationProfileAssessmentV0_1 {
    let assessment = specification.assess();
    let mut reasons = BTreeSet::new();
    reasons.extend(assessment.reason_codes);

    match assessment.disposition {
        ClaimSpecificationDispositionV0_1::Incomplete => {
            return ClaimObligationProfileAssessmentV0_1 {
                disposition: ClaimObligationProfileDispositionV0_1::Incomplete,
                profile: None,
                reason_codes: reasons.into_iter().collect(),
                physical_truth_established: false,
                engineering_acceptance_granted: false,
                execution_authority_granted: false,
            };
        }
        ClaimSpecificationDispositionV0_1::Rejected => {
            return ClaimObligationProfileAssessmentV0_1 {
                disposition: ClaimObligationProfileDispositionV0_1::Rejected,
                profile: None,
                reason_codes: reasons.into_iter().collect(),
                physical_truth_established: false,
                engineering_acceptance_granted: false,
                execution_authority_granted: false,
            };
        }
        ClaimSpecificationDispositionV0_1::UnsupportedProfile => {
            return ClaimObligationProfileAssessmentV0_1 {
                disposition: ClaimObligationProfileDispositionV0_1::UnsupportedProfile,
                profile: None,
                reason_codes: reasons.into_iter().collect(),
                physical_truth_established: false,
                engineering_acceptance_granted: false,
                execution_authority_granted: false,
            };
        }
        ClaimSpecificationDispositionV0_1::FormallyWellSpecified => {}
    }

    if !valid_identifier(&specification.claim_id) || !valid_identifier(&specification.subject_ref) {
        reasons.insert("CLAIM_OBLIGATION_PROFILE_CLAIM_BINDING_INVALID".to_owned());
    }
    let semantic_sha256 = match specification.compute_semantic_sha256() {
        Ok(value) => value,
        Err(errors) => {
            reasons.extend(errors);
            String::new()
        }
    };
    if semantic_sha256.is_empty() {
        return ClaimObligationProfileAssessmentV0_1 {
            disposition: ClaimObligationProfileDispositionV0_1::Rejected,
            profile: None,
            reason_codes: reasons.into_iter().collect(),
            physical_truth_established: false,
            engineering_acceptance_granted: false,
            execution_authority_granted: false,
        };
    }

    let obligations = derive_obligations(specification);
    let mut seen = BTreeSet::new();
    if obligations.iter().any(|item| {
        !valid_identifier(&item.obligation_id)
            || !valid_identifier(&item.requirement_source_ref)
            || !valid_identifier(&item.scoped_subject_ref)
            || !seen.insert(item.obligation_id.as_str())
    }) {
        reasons.insert("CLAIM_OBLIGATION_PROFILE_DERIVATION_INVALID_OR_DUPLICATE".to_owned());
    }
    if !reasons.is_empty() {
        return ClaimObligationProfileAssessmentV0_1 {
            disposition: ClaimObligationProfileDispositionV0_1::Rejected,
            profile: None,
            reason_codes: reasons.into_iter().collect(),
            physical_truth_established: false,
            engineering_acceptance_granted: false,
            execution_authority_granted: false,
        };
    }

    let profile_digest = profile_sha256(profile);
    let manifest_digest = obligation_manifest_sha256(
        &specification.claim_id,
        &semantic_sha256,
        &profile_digest,
        &obligations,
    );
    let derived = DerivedClaimObligationProfileV0_1 {
        schema: CLAIM_OBLIGATION_PROFILE_SCHEMA_V0_1.to_owned(),
        profile_id: profile.profile_id().to_owned(),
        claim_id: specification.claim_id.clone(),
        claim_specification_semantic_sha256: semantic_sha256,
        profile_sha256: profile_digest,
        obligation_manifest_sha256: manifest_digest,
        obligations,
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        execution_authority_granted: false,
    };

    ClaimObligationProfileAssessmentV0_1 {
        disposition: ClaimObligationProfileDispositionV0_1::Derived,
        profile: Some(derived),
        reason_codes: Vec::new(),
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        execution_authority_granted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPECIFICATION_FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/aero_claim_specification_v0_1.json"
    );

    fn specification() -> ClaimSpecificationV0_1 {
        serde_json::from_str(SPECIFICATION_FIXTURE)
            .expect("synthetic claim specification parses")
    }

    #[test]
    fn derives_one_hard_gate_per_required_evidence_class_and_stratum() {
        let specification = specification();
        let result = derive_claim_obligation_profile_v0_1(
            &specification,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        );
        assert_eq!(result.disposition, ClaimObligationProfileDispositionV0_1::Derived);
        let profile = result.profile.expect("derived obligation profile");

        let evidence_class_count = specification.required_evidence_classes.len();
        let stratum_count = specification.population.required_strata.len();
        let evidence_obligations = profile
            .obligations
            .iter()
            .filter(|item| item.kind == ObligationKindV0_1::EvidenceIntegrity)
            .collect::<Vec<_>>();
        let stratum_obligations = profile
            .obligations
            .iter()
            .filter(|item| {
                item.kind == ObligationKindV0_1::ModelCredibility
                    && item.obligation_id.starts_with("model-credibility:")
            })
            .collect::<Vec<_>>();

        assert_eq!(evidence_obligations.len(), evidence_class_count);
        assert_eq!(stratum_obligations.len(), stratum_count);
        assert!(profile.obligations.iter().all(|item| {
            if item.mandatory {
                item.hard_gate
            } else {
                !item.hard_gate
            }
        }));
        assert!(profile.obligations.iter().any(|item| {
            item.obligation_id == "producer-authority"
                && item.mandatory
                && item.hard_gate
                && item.kind == ObligationKindV0_1::ProducerAuthority
        }));
        assert!(!profile.physical_truth_established);
        assert!(!profile.engineering_acceptance_granted);
        assert!(!profile.execution_authority_granted);
    }

    #[test]
    fn caller_cannot_downgrade_required_class_or_stratum_obligations() {
        let specification = specification();
        let first = derive_claim_obligation_profile_v0_1(
            &specification,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        )
        .profile
        .expect("profile");
        assert!(first.obligations.iter().any(|item| {
            item.obligation_id == "evidence-integrity:evidence-class:structural-test"
                && item.mandatory
                && item.hard_gate
        }));
        assert!(first.obligations.iter().any(|item| {
            item.obligation_id == "model-credibility:stratum:temperature-ambient"
                && item.mandatory
                && item.hard_gate
        }));
    }

    #[test]
    fn assumption_and_exclusion_defeaters_are_advisory_not_silent_gates() {
        let specification = specification();
        let profile = derive_claim_obligation_profile_v0_1(
            &specification,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        )
        .profile
        .expect("profile");
        let advisory = profile
            .obligations
            .iter()
            .filter(|item| item.kind == ObligationKindV0_1::DefeaterResolution)
            .collect::<Vec<_>>();
        assert_eq!(
            advisory.len(),
            specification.assumptions.len() + specification.exclusions.len()
        );
        assert!(advisory.iter().all(|item| !item.mandatory && !item.hard_gate));
    }

    #[test]
    fn incomplete_specification_does_not_produce_obligations() {
        let mut specification = specification();
        specification.subject_ref.clear();
        let result = derive_claim_obligation_profile_v0_1(
            &specification,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        );
        assert_eq!(result.disposition, ClaimObligationProfileDispositionV0_1::Incomplete);
        assert!(result.profile.is_none());
        assert!(!result.physical_truth_established);
    }

    #[test]
    fn semantic_mutation_changes_the_derived_manifest() {
        let mut specification = specification();
        let before = derive_claim_obligation_profile_v0_1(
            &specification,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        )
        .profile
        .expect("baseline profile")
        .obligation_manifest_sha256;
        specification.population.required_strata[0].level_ref =
            "level:changed-load-case".to_owned();
        let after = derive_claim_obligation_profile_v0_1(
            &specification,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        )
        .profile
        .expect("mutated profile")
        .obligation_manifest_sha256;
        assert_ne!(before, after);
    }

    #[test]
    fn repeated_derivation_is_deterministic() {
        let specification = specification();
        let profile = RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1;
        let left = derive_claim_obligation_profile_v0_1(&specification, profile);
        let right = derive_claim_obligation_profile_v0_1(&specification, profile);
        assert_eq!(left, right);
    }
}

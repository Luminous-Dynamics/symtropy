// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Requirement traceability and verification-obligation contracts for Symtropy designs.
//!
//! This crate records what a design requires and how each requirement is intended
//! to be verified. It deliberately does not store pass/fail/verified state and
//! does not evaluate `symtropy-fabrication` functional constraints.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};
use symtropy_design::{ContentDigest, DesignArtifactRef, DesignSemanticRef};
use symtropy_game_state::StableId;

pub const REQUIREMENT_SET_SCHEMA_VERSION: u32 = 1;
const REQUIREMENT_SET_DIGEST_DOMAIN: &[u8] = b"symtropy.design.requirement-set.v1\0";
const REQUIREMENT_SET_KIND_ID: &str = "design-semantic:requirement-set";
const SHA256_ALGORITHM_ID: &str = "sha256";

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(StableId);

        impl $name {
            pub const fn new(id: StableId) -> Self {
                Self(id)
            }

            pub const fn stable_id(&self) -> &StableId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id_type!(RequirementSetId);
stable_id_type!(DesignRequirementId);
stable_id_type!(VerificationObligationId);

/// One verification target named by the authority that actually owns its
/// semantics. None of these targets implies that verification has succeeded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "target_kind", rename_all = "snake_case")]
pub enum VerificationTarget {
    /// A checkable engineering constraint owned by an external functional
    /// engineering evaluator such as Symtropy Fabrication F7.
    EngineeringConstraint {
        authority_id: StableId,
        constraint_id: StableId,
    },
    /// A required kind of evidence. The evidence authority decides what counts
    /// as a valid instance of that kind.
    EvidenceKind {
        authority_id: StableId,
        evidence_kind: StableId,
    },
    /// An observable requested from one exact analysis profile. This is an
    /// analysis obligation, not permission to promote a solver output to truth.
    AnalysisObservable {
        analysis_profile_id: StableId,
        observable_id: StableId,
    },
    /// A verification case defined by an external standard, regulator, test
    /// laboratory, certification scheme, or other named authority.
    ExternalVerificationCase {
        scheme_id: StableId,
        case_id: StableId,
    },
}

impl VerificationTarget {
    fn validate(&self) -> Result<(), RequirementError> {
        match self {
            Self::EngineeringConstraint {
                authority_id,
                constraint_id,
            } => {
                validate_stable_id(authority_id)?;
                validate_stable_id(constraint_id)
            }
            Self::EvidenceKind {
                authority_id,
                evidence_kind,
            } => {
                validate_stable_id(authority_id)?;
                validate_stable_id(evidence_kind)
            }
            Self::AnalysisObservable {
                analysis_profile_id,
                observable_id,
            } => {
                validate_stable_id(analysis_profile_id)?;
                validate_stable_id(observable_id)
            }
            Self::ExternalVerificationCase { scheme_id, case_id } => {
                validate_stable_id(scheme_id)?;
                validate_stable_id(case_id)
            }
        }
    }
}

/// One stable obligation attached to a design requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationObligation {
    pub id: VerificationObligationId,
    pub target: VerificationTarget,
}

impl VerificationObligation {
    pub fn new(
        id: VerificationObligationId,
        target: VerificationTarget,
    ) -> Result<Self, RequirementError> {
        validate_stable_id(id.stable_id())?;
        target.validate()?;
        Ok(Self { id, target })
    }
}

/// One requirement in a canonical requirement set.
///
/// `derives_from` expresses requirement lineage within this exact set. A
/// requirement may temporarily have no verification obligations; that is an
/// explicit traceability gap, not an implicit pass or structural error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignRequirement {
    pub id: DesignRequirementId,
    derives_from: Vec<DesignRequirementId>,
    pub statement_artifact: Option<DesignArtifactRef>,
    obligations: Vec<VerificationObligation>,
}

impl DesignRequirement {
    pub fn new(
        id: DesignRequirementId,
        mut derives_from: Vec<DesignRequirementId>,
        statement_artifact: Option<DesignArtifactRef>,
        mut obligations: Vec<VerificationObligation>,
    ) -> Result<Self, RequirementError> {
        validate_stable_id(id.stable_id())?;

        derives_from.sort();
        if derives_from.iter().any(|parent| parent == &id) {
            return Err(RequirementError::SelfDerivation(id));
        }
        for pair in derives_from.windows(2) {
            if pair[0] == pair[1] {
                return Err(RequirementError::DuplicateDerivationParent(pair[0].clone()));
            }
        }

        if let Some(artifact) = &statement_artifact {
            artifact.validate().map_err(RequirementError::Design)?;
        }

        obligations.sort_by(|left, right| left.id.cmp(&right.id));
        let mut targets = BTreeSet::new();
        for (index, obligation) in obligations.iter().enumerate() {
            validate_stable_id(obligation.id.stable_id())?;
            obligation.target.validate()?;
            if index > 0 && obligations[index - 1].id == obligation.id {
                return Err(RequirementError::DuplicateObligationId(
                    obligation.id.clone(),
                ));
            }
            if !targets.insert(obligation.target.clone()) {
                return Err(RequirementError::DuplicateVerificationTarget {
                    requirement_id: id.clone(),
                    target: obligation.target.clone(),
                });
            }
        }

        Ok(Self {
            id,
            derives_from,
            statement_artifact,
            obligations,
        })
    }

    pub fn derives_from(&self) -> &[DesignRequirementId] {
        &self.derives_from
    }

    pub fn obligations(&self) -> &[VerificationObligation] {
        &self.obligations
    }

    pub fn has_verification_path(&self) -> bool {
        !self.obligations.is_empty()
    }
}

/// Canonical, immutable requirement-set snapshot.
///
/// The set contains no verification result. Consumers resolve the obligations
/// against their owning authorities and exact evidence cuts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementSet {
    pub schema_version: u32,
    pub id: RequirementSetId,
    pub revision: u64,
    requirements: Vec<DesignRequirement>,
}

impl RequirementSet {
    pub fn new(
        id: RequirementSetId,
        revision: u64,
        mut requirements: Vec<DesignRequirement>,
    ) -> Result<Self, RequirementError> {
        if requirements.is_empty() {
            return Err(RequirementError::RequirementRequired);
        }
        validate_stable_id(id.stable_id())?;

        requirements.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in requirements.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(RequirementError::DuplicateRequirement(pair[0].id.clone()));
            }
        }

        let set = Self {
            schema_version: REQUIREMENT_SET_SCHEMA_VERSION,
            id,
            revision,
            requirements,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn requirements(&self) -> &[DesignRequirement] {
        &self.requirements
    }

    pub fn requirement(&self, id: &DesignRequirementId) -> Option<&DesignRequirement> {
        self.requirements
            .binary_search_by(|candidate| candidate.id.cmp(id))
            .ok()
            .map(|index| &self.requirements[index])
    }

    /// Requirements without any declared verification path. This is a
    /// deterministic traceability diagnostic, not a pass/fail evaluation.
    pub fn traceability_gaps(&self) -> Vec<DesignRequirementId> {
        self.requirements
            .iter()
            .filter(|requirement| !requirement.has_verification_path())
            .map(|requirement| requirement.id.clone())
            .collect()
    }

    /// Deterministic parent-before-child order for requirement derivation.
    pub fn topological_order(&self) -> Vec<DesignRequirementId> {
        topological_order(&self.requirements)
    }

    pub fn content_digest(&self) -> Result<ContentDigest, RequirementError> {
        self.validate()?;
        let bytes = self.canonical_preimage()?;
        let digest = Sha256::digest(bytes);
        ContentDigest::new(
            StableId::parse(SHA256_ALGORITHM_ID)
                .expect("sha256 is a valid stable identifier literal"),
            hex(&digest),
        )
        .map_err(RequirementError::Design)
    }

    /// Exact D1 semantic reference used by a `DesignRevisionManifest`.
    pub fn semantic_ref(&self) -> Result<DesignSemanticRef, RequirementError> {
        DesignSemanticRef::new(
            StableId::parse(REQUIREMENT_SET_KIND_ID)
                .expect("requirement-set kind literal is a valid stable identifier"),
            self.id.stable_id().clone(),
            self.revision,
            self.content_digest()?,
        )
        .map_err(RequirementError::Design)
    }

    pub fn validate(&self) -> Result<(), RequirementError> {
        if self.schema_version != REQUIREMENT_SET_SCHEMA_VERSION {
            return Err(RequirementError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        validate_stable_id(self.id.stable_id())?;
        if self.requirements.is_empty() {
            return Err(RequirementError::RequirementRequired);
        }

        let ids = self
            .requirements
            .iter()
            .map(|requirement| requirement.id.clone())
            .collect::<BTreeSet<_>>();
        if ids.len() != self.requirements.len() {
            return Err(RequirementError::NonCanonicalRequirementOrder);
        }

        let mut obligation_ids = BTreeSet::new();
        for (index, requirement) in self.requirements.iter().enumerate() {
            if index > 0 && self.requirements[index - 1].id >= requirement.id {
                return Err(RequirementError::NonCanonicalRequirementOrder);
            }
            validate_stable_id(requirement.id.stable_id())?;

            for parent in &requirement.derives_from {
                if parent == &requirement.id {
                    return Err(RequirementError::SelfDerivation(requirement.id.clone()));
                }
                if !ids.contains(parent) {
                    return Err(RequirementError::UnknownDerivationParent {
                        requirement_id: requirement.id.clone(),
                        parent_id: parent.clone(),
                    });
                }
            }
            for pair in requirement.derives_from.windows(2) {
                if pair[0] >= pair[1] {
                    return Err(RequirementError::NonCanonicalDerivationOrder(
                        requirement.id.clone(),
                    ));
                }
            }

            if let Some(artifact) = &requirement.statement_artifact {
                artifact.validate().map_err(RequirementError::Design)?;
            }

            let mut targets = BTreeSet::new();
            for (obligation_index, obligation) in requirement.obligations.iter().enumerate() {
                validate_stable_id(obligation.id.stable_id())?;
                obligation.target.validate()?;
                if obligation_index > 0
                    && requirement.obligations[obligation_index - 1].id >= obligation.id
                {
                    return Err(RequirementError::NonCanonicalObligationOrder(
                        requirement.id.clone(),
                    ));
                }
                if !obligation_ids.insert(obligation.id.clone()) {
                    return Err(RequirementError::DuplicateObligationId(
                        obligation.id.clone(),
                    ));
                }
                if !targets.insert(obligation.target.clone()) {
                    return Err(RequirementError::DuplicateVerificationTarget {
                        requirement_id: requirement.id.clone(),
                        target: obligation.target.clone(),
                    });
                }
            }
        }

        if topological_order(&self.requirements).len() != self.requirements.len() {
            return Err(RequirementError::DerivationCycle);
        }
        Ok(())
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, RequirementError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(REQUIREMENT_SET_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        bytes.extend_from_slice(&self.revision.to_le_bytes());
        encode_len(&mut bytes, self.requirements.len())?;

        for requirement in &self.requirements {
            encode_stable_id(&mut bytes, requirement.id.stable_id());
            encode_len(&mut bytes, requirement.derives_from.len())?;
            for parent in &requirement.derives_from {
                encode_stable_id(&mut bytes, parent.stable_id());
            }

            match &requirement.statement_artifact {
                Some(artifact) => {
                    bytes.push(1);
                    encode_stable_id(&mut bytes, artifact.id.stable_id());
                    encode_stable_id(&mut bytes, &artifact.role_id);
                    encode_digest(&mut bytes, &artifact.content_digest)?;
                }
                None => bytes.push(0),
            }

            encode_len(&mut bytes, requirement.obligations.len())?;
            for obligation in &requirement.obligations {
                encode_stable_id(&mut bytes, obligation.id.stable_id());
                encode_target(&mut bytes, &obligation.target);
            }
        }
        Ok(bytes)
    }
}

fn topological_order(requirements: &[DesignRequirement]) -> Vec<DesignRequirementId> {
    let mut indegree = requirements
        .iter()
        .map(|requirement| (requirement.id.clone(), requirement.derives_from.len()))
        .collect::<BTreeMap<_, _>>();
    let mut children = BTreeMap::<DesignRequirementId, Vec<DesignRequirementId>>::new();

    for requirement in requirements {
        for parent in &requirement.derives_from {
            children
                .entry(parent.clone())
                .or_default()
                .push(requirement.id.clone());
        }
    }
    for values in children.values_mut() {
        values.sort();
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(requirements.len());

    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        if let Some(dependents) = children.get(&id) {
            for dependent in dependents {
                let degree = indegree
                    .get_mut(dependent)
                    .expect("all derivation children originate from set requirements");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
    }
    order
}

fn encode_target(bytes: &mut Vec<u8>, target: &VerificationTarget) {
    match target {
        VerificationTarget::EngineeringConstraint {
            authority_id,
            constraint_id,
        } => {
            bytes.push(0);
            encode_stable_id(bytes, authority_id);
            encode_stable_id(bytes, constraint_id);
        }
        VerificationTarget::EvidenceKind {
            authority_id,
            evidence_kind,
        } => {
            bytes.push(1);
            encode_stable_id(bytes, authority_id);
            encode_stable_id(bytes, evidence_kind);
        }
        VerificationTarget::AnalysisObservable {
            analysis_profile_id,
            observable_id,
        } => {
            bytes.push(2);
            encode_stable_id(bytes, analysis_profile_id);
            encode_stable_id(bytes, observable_id);
        }
        VerificationTarget::ExternalVerificationCase { scheme_id, case_id } => {
            bytes.push(3);
            encode_stable_id(bytes, scheme_id);
            encode_stable_id(bytes, case_id);
        }
    }
}

fn encode_digest(bytes: &mut Vec<u8>, digest: &ContentDigest) -> Result<(), RequirementError> {
    digest.validate().map_err(RequirementError::Design)?;
    encode_stable_id(bytes, &digest.algorithm);
    encode_string(bytes, &digest.value)
}

fn encode_stable_id(bytes: &mut Vec<u8>, id: &StableId) {
    encode_string(bytes, id.as_str()).expect("StableId length is bounded well below usize::MAX");
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), RequirementError> {
    encode_len(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), RequirementError> {
    let len = u64::try_from(len).map_err(|_| RequirementError::LengthOverflow)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn validate_stable_id(id: &StableId) -> Result<(), RequirementError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| RequirementError::InvalidStableId(id.as_str().to_string()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug)]
pub enum RequirementError {
    InvalidStableId(String),
    UnsupportedSchemaVersion(u32),
    RequirementRequired,
    DuplicateRequirement(DesignRequirementId),
    SelfDerivation(DesignRequirementId),
    DuplicateDerivationParent(DesignRequirementId),
    UnknownDerivationParent {
        requirement_id: DesignRequirementId,
        parent_id: DesignRequirementId,
    },
    DerivationCycle,
    DuplicateObligationId(VerificationObligationId),
    DuplicateVerificationTarget {
        requirement_id: DesignRequirementId,
        target: VerificationTarget,
    },
    NonCanonicalRequirementOrder,
    NonCanonicalDerivationOrder(DesignRequirementId),
    NonCanonicalObligationOrder(DesignRequirementId),
    LengthOverflow,
    Design(symtropy_design::DesignError),
}

impl fmt::Display for RequirementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId(id) => write!(formatter, "invalid stable identifier: {id}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported requirement-set schema version {version}")
            }
            Self::RequirementRequired => write!(formatter, "requirement set must not be empty"),
            Self::DuplicateRequirement(id) => write!(formatter, "duplicate requirement {id}"),
            Self::SelfDerivation(id) => {
                write!(formatter, "requirement {id} cannot derive from itself")
            }
            Self::DuplicateDerivationParent(id) => {
                write!(formatter, "duplicate derivation parent {id}")
            }
            Self::UnknownDerivationParent {
                requirement_id,
                parent_id,
            } => write!(
                formatter,
                "requirement {requirement_id} derives from unknown requirement {parent_id}"
            ),
            Self::DerivationCycle => write!(formatter, "requirement derivation graph contains a cycle"),
            Self::DuplicateObligationId(id) => {
                write!(formatter, "duplicate verification obligation identity {id}")
            }
            Self::DuplicateVerificationTarget {
                requirement_id,
                target,
            } => write!(
                formatter,
                "requirement {requirement_id} repeats verification target {target:?}"
            ),
            Self::NonCanonicalRequirementOrder => {
                write!(formatter, "requirements are not in canonical identity order")
            }
            Self::NonCanonicalDerivationOrder(id) => write!(
                formatter,
                "derivation parents for requirement {id} are not in canonical order"
            ),
            Self::NonCanonicalObligationOrder(id) => write!(
                formatter,
                "verification obligations for requirement {id} are not in canonical order"
            ),
            Self::LengthOverflow => write!(formatter, "canonical design length exceeds u64"),
            Self::Design(error) => error.fmt(formatter),
        }
    }
}

impl Error for RequirementError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Design(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_design::{DesignArtifactId, ContentDigest};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn req_id(value: &str) -> DesignRequirementId {
        DesignRequirementId::new(id(value))
    }

    fn obligation(id_value: &str, observable: &str) -> VerificationObligation {
        VerificationObligation::new(
            VerificationObligationId::new(id(id_value)),
            VerificationTarget::AnalysisObservable {
                analysis_profile_id: id("analysis-profile:structural-static-v1"),
                observable_id: id(observable),
            },
        )
        .unwrap()
    }

    fn requirement(
        name: &str,
        derives_from: Vec<DesignRequirementId>,
        obligations: Vec<VerificationObligation>,
    ) -> DesignRequirement {
        DesignRequirement::new(req_id(name), derives_from, None, obligations).unwrap()
    }

    fn set(requirements: Vec<DesignRequirement>) -> RequirementSet {
        RequirementSet::new(
            RequirementSetId::new(id("requirement-set:bracket")),
            1,
            requirements,
        )
        .unwrap()
    }

    #[test]
    fn insertion_order_does_not_change_requirement_set_digest() {
        let root = requirement(
            "requirement:service-load",
            Vec::new(),
            vec![obligation("obligation:deflection", "observable:deflection")],
        );
        let child = requirement(
            "requirement:mass",
            vec![root.id.clone()],
            vec![obligation("obligation:mass", "observable:mass")],
        );

        let left = set(vec![root.clone(), child.clone()]);
        let right = set(vec![child, root]);
        assert_eq!(left, right);
        assert_eq!(left.content_digest().unwrap(), right.content_digest().unwrap());
    }

    #[test]
    fn changing_verification_target_changes_semantic_identity() {
        let left = set(vec![requirement(
            "requirement:service-load",
            Vec::new(),
            vec![obligation("obligation:deflection", "observable:deflection")],
        )]);
        let right = set(vec![requirement(
            "requirement:service-load",
            Vec::new(),
            vec![obligation("obligation:deflection", "observable:strain")],
        )]);

        assert_ne!(left.content_digest().unwrap(), right.content_digest().unwrap());
        assert_ne!(left.semantic_ref().unwrap(), right.semantic_ref().unwrap());
    }

    #[test]
    fn unknown_derivation_parent_is_rejected() {
        let child = requirement(
            "requirement:child",
            vec![req_id("requirement:missing")],
            Vec::new(),
        );
        let result = RequirementSet::new(
            RequirementSetId::new(id("requirement-set:test")),
            1,
            vec![child],
        );
        assert!(matches!(
            result,
            Err(RequirementError::UnknownDerivationParent { .. })
        ));
    }

    #[test]
    fn derivation_cycle_is_rejected() {
        let a = requirement(
            "requirement:a",
            vec![req_id("requirement:b")],
            Vec::new(),
        );
        let b = requirement(
            "requirement:b",
            vec![req_id("requirement:a")],
            Vec::new(),
        );
        let result = RequirementSet::new(
            RequirementSetId::new(id("requirement-set:test")),
            1,
            vec![a, b],
        );
        assert!(matches!(result, Err(RequirementError::DerivationCycle)));
    }

    #[test]
    fn obligation_ids_are_unique_across_the_set() {
        let shared_id = "obligation:shared";
        let first = requirement(
            "requirement:a",
            Vec::new(),
            vec![obligation(shared_id, "observable:a")],
        );
        let second = requirement(
            "requirement:b",
            Vec::new(),
            vec![obligation(shared_id, "observable:b")],
        );
        let result = RequirementSet::new(
            RequirementSetId::new(id("requirement-set:test")),
            1,
            vec![first, second],
        );
        assert!(matches!(
            result,
            Err(RequirementError::DuplicateObligationId(_))
        ));
    }

    #[test]
    fn duplicate_target_within_one_requirement_is_rejected() {
        let target = VerificationTarget::EngineeringConstraint {
            authority_id: id("authority:fabrication-f7"),
            constraint_id: id("constraint:deflection"),
        };
        let result = DesignRequirement::new(
            req_id("requirement:deflection"),
            Vec::new(),
            None,
            vec![
                VerificationObligation::new(
                    VerificationObligationId::new(id("obligation:a")),
                    target.clone(),
                )
                .unwrap(),
                VerificationObligation::new(
                    VerificationObligationId::new(id("obligation:b")),
                    target,
                )
                .unwrap(),
            ],
        );
        assert!(matches!(
            result,
            Err(RequirementError::DuplicateVerificationTarget { .. })
        ));
    }

    #[test]
    fn missing_verification_path_is_an_explicit_gap_not_a_structural_failure() {
        let requirement = requirement("requirement:future", Vec::new(), Vec::new());
        let set = set(vec![requirement]);
        assert_eq!(set.traceability_gaps(), vec![req_id("requirement:future")]);
    }

    #[test]
    fn semantic_ref_is_exactly_bound_to_requirement_set_identity() {
        let set = set(vec![requirement(
            "requirement:mass",
            Vec::new(),
            vec![obligation("obligation:mass", "observable:mass")],
        )]);
        let reference = set.semantic_ref().unwrap();
        assert_eq!(reference.kind_id, id(REQUIREMENT_SET_KIND_ID));
        assert_eq!(reference.subject_id, id("requirement-set:bracket"));
        assert_eq!(reference.revision, 1);
        assert_eq!(reference.content_digest, set.content_digest().unwrap());
    }

    #[test]
    fn statement_artifact_content_is_bound_into_requirement_set_digest() {
        let artifact = |value: &str| {
            DesignArtifactRef::new(
                DesignArtifactId::new(id("artifact:requirement-statement")),
                id("design-artifact:requirement-statement"),
                ContentDigest::new(id("blake3"), value).unwrap(),
            )
            .unwrap()
        };

        let make = |value: &str| {
            set(vec![DesignRequirement::new(
                req_id("requirement:statement"),
                Vec::new(),
                Some(artifact(value)),
                Vec::new(),
            )
            .unwrap()])
        };

        assert_ne!(
            make("aaaaaaaa").content_digest().unwrap(),
            make("bbbbbbbb").content_digest().unwrap()
        );
    }

    #[test]
    fn topological_order_is_deterministic_parent_before_child() {
        let root = requirement("requirement:root", Vec::new(), Vec::new());
        let b = requirement(
            "requirement:b",
            vec![root.id.clone()],
            Vec::new(),
        );
        let a = requirement(
            "requirement:a",
            vec![root.id.clone()],
            Vec::new(),
        );
        let set = set(vec![b, root.clone(), a]);
        assert_eq!(
            set.topological_order(),
            vec![root.id, req_id("requirement:a"), req_id("requirement:b")]
        );
    }
}

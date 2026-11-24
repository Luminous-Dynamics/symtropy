//! Owner-side validation for evidence-bound ErgoStream transfer bundles.
//!
//! This module joins the sidecar contract to the existing blind-process
//! evidence receipts without changing AgentEpisode/AgentAction protocol v0.
//! It validates exact frame/episode bindings and checks that every structural
//! node/relation citation is present in its corresponding public evidence
//! prefix before delegating to the scoped structural witness.
//!
//! It does not parse JSON, hash arbitrary payloads, authenticate a process,
//! establish the truth of an observation, or reveal evaluator-owned outcomes.
//! The caller must resolve these records from the trusted owner-side receipt
//! store and pass the exact frame hashes emitted by ProcessActionExchange.

use std::collections::BTreeSet;
use std::fmt;

use super::scoped::{
    ScopedStructuralCandidateV0, ScopedTransferErrorV0, ScopedStructuralWitnessV0,
    validate_scoped_structural_correspondence,
};
use super::transfer::StructureV0;

pub const AGENT_PROTOCOL_VERSION_V0: &str = "symthaea-open-world-research-episode-v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessActionKindV0 {
    Probe,
    Submit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSubmitFrameV0 {
    pub episode_id: String,
    pub evidence_prefix_sha256: String,
    /// Exact AgentView bytes as sent by the parent, including the JSONL newline.
    pub agent_view_frame_sha256: String,
    /// Exact AgentAction bytes as returned by the child, including the JSONL newline.
    pub agent_action_frame_sha256: String,
    pub action_kind: ProcessActionKindV0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerEpisodeReceiptIdentityV0 {
    pub episode_id: String,
    /// Digest of the canonical owner-issued ProcessEpisodeReceipt artifact.
    pub process_episode_receipt_sha256: String,
    pub final_evidence_prefix_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSubmissionBindingV0 {
    pub agent_protocol_version: String,
    pub submission_episode_id: String,
    pub submission_evidence_prefix_sha256: String,
    pub agent_view_frame_sha256: String,
    pub agent_action_frame_sha256: String,
    pub source_episode_id: String,
    pub target_episode_id: String,
    pub source_evidence_prefix_sha256: String,
    pub target_evidence_prefix_sha256: String,
    pub source_episode_receipt_sha256: String,
    pub target_episode_receipt_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessBindingWitnessV0 {
    pub source_episode_bound: bool,
    pub target_episode_bound: bool,
    pub submit_frame_bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicEvidencePrefixV0 {
    pub episode_id: String,
    pub evidence_prefix_sha256: String,
    /// Evidence IDs that were actually returned in this exact public prefix.
    pub available_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEvidenceBindingV0 {
    pub entity_id: u32,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBoundStructureV0 {
    pub episode_id: String,
    pub evidence_prefix_sha256: String,
    pub structure: StructureV0,
    pub node_evidence: Vec<EntityEvidenceBindingV0>,
    pub relation_evidence: Vec<EntityEvidenceBindingV0>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBoundTransferWitnessV0 {
    pub structural: ScopedStructuralWitnessV0,
    pub source_node_evidence_references: usize,
    pub source_relation_evidence_references: usize,
    pub target_node_evidence_references: usize,
    pub target_relation_evidence_references: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleValidationErrorV0 {
    InvalidProtocolVersion,
    EmptyEpisodeId,
    SameSourceAndTargetEpisode,
    InvalidDigest,
    SourceReceiptBindingMismatch,
    TargetReceiptBindingMismatch,
    SubmissionEpisodeMismatch,
    SubmissionEvidencePrefixMismatch,
    SubmitActionRequired,
    AgentViewFrameMismatch,
    AgentActionFrameMismatch,
    PrefixEpisodeMismatch,
    PrefixDigestMismatch,
    DuplicateAvailableEvidenceId,
    InvalidEvidenceId,
    DuplicateEntityEvidenceBinding,
    UnknownEntityEvidenceBinding,
    MissingEntityEvidenceBinding,
    EmptyEntityEvidence,
    DuplicateEntityEvidenceId,
    UnknownEvidenceReference,
    ScopedStructuralRejected(ScopedTransferErrorV0),
}

impl fmt::Display for BundleValidationErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidProtocolVersion => "unsupported blind-agent protocol version",
            Self::EmptyEpisodeId => "episode IDs must be non-empty",
            Self::SameSourceAndTargetEpisode => "source and target must be distinct episodes",
            Self::InvalidDigest => "binding contains a malformed SHA-256 digest",
            Self::SourceReceiptBindingMismatch => {
                "source episode receipt does not match the transfer bundle binding"
            }
            Self::TargetReceiptBindingMismatch => {
                "target episode receipt does not match the transfer bundle binding"
            }
            Self::SubmissionEpisodeMismatch => {
                "submit-frame episode must be the declared target episode"
            }
            Self::SubmissionEvidencePrefixMismatch => {
                "submit frame evidence prefix does not match the declared target prefix"
            }
            Self::SubmitActionRequired => {
                "transfer bundle must bind to a terminal AgentAction::Submit frame"
            }
            Self::AgentViewFrameMismatch => {
                "AgentView frame digest does not match the owner process receipt"
            }
            Self::AgentActionFrameMismatch => {
                "AgentAction frame digest does not match the owner process receipt"
            }
            Self::PrefixEpisodeMismatch => "public evidence prefix episode ID does not match",
            Self::PrefixDigestMismatch => "public evidence prefix digest does not match",
            Self::DuplicateAvailableEvidenceId => {
                "public evidence prefix contains duplicate evidence IDs"
            }
            Self::InvalidEvidenceId => "evidence ID does not match the public evidence identifier format",
            Self::DuplicateEntityEvidenceBinding => {
                "entity evidence bindings contain a duplicate entity ID"
            }
            Self::UnknownEntityEvidenceBinding => {
                "entity evidence binding references an unknown node or relation"
            }
            Self::MissingEntityEvidenceBinding => {
                "a declared node or relation has no evidence binding"
            }
            Self::EmptyEntityEvidence => "each declared node or relation requires at least one evidence ID",
            Self::DuplicateEntityEvidenceId => {
                "one node or relation cites the same evidence ID more than once"
            }
            Self::UnknownEvidenceReference => {
                "a node or relation cites evidence outside its public episode prefix"
            }
            Self::ScopedStructuralRejected(error) => {
                return write!(formatter, "scoped structural candidate rejected: {error}");
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for BundleValidationErrorV0 {}

/// Match the sidecar binding against trusted source/target process receipts
/// and the exact terminal request/response frame pair.
///
/// The existing v0 runner hashes complete JSONL frames, including the newline.
/// The target Submit frame is the submission event; hashing only the nested
/// AgentSubmission payload is not equivalent and is deliberately not accepted.
pub fn validate_transfer_submission_binding(
    binding: &TransferSubmissionBindingV0,
    source_receipt: &OwnerEpisodeReceiptIdentityV0,
    target_receipt: &OwnerEpisodeReceiptIdentityV0,
    submit_frame: &ProcessSubmitFrameV0,
) -> Result<ProcessBindingWitnessV0, BundleValidationErrorV0> {
    if binding.agent_protocol_version != AGENT_PROTOCOL_VERSION_V0 {
        return Err(BundleValidationErrorV0::InvalidProtocolVersion);
    }

    if binding.source_episode_id.trim().is_empty()
        || binding.target_episode_id.trim().is_empty()
        || binding.submission_episode_id.trim().is_empty()
        || source_receipt.episode_id.trim().is_empty()
        || target_receipt.episode_id.trim().is_empty()
        || submit_frame.episode_id.trim().is_empty()
    {
        return Err(BundleValidationErrorV0::EmptyEpisodeId);
    }

    if binding.source_episode_id == binding.target_episode_id {
        return Err(BundleValidationErrorV0::SameSourceAndTargetEpisode);
    }

    for digest in [
        binding.submission_evidence_prefix_sha256.as_str(),
        binding.agent_view_frame_sha256.as_str(),
        binding.agent_action_frame_sha256.as_str(),
        binding.source_evidence_prefix_sha256.as_str(),
        binding.target_evidence_prefix_sha256.as_str(),
        binding.source_episode_receipt_sha256.as_str(),
        binding.target_episode_receipt_sha256.as_str(),
        source_receipt.process_episode_receipt_sha256.as_str(),
        source_receipt.final_evidence_prefix_sha256.as_str(),
        target_receipt.process_episode_receipt_sha256.as_str(),
        target_receipt.final_evidence_prefix_sha256.as_str(),
        submit_frame.evidence_prefix_sha256.as_str(),
        submit_frame.agent_view_frame_sha256.as_str(),
        submit_frame.agent_action_frame_sha256.as_str(),
    ] {
        if !is_sha256_digest(digest) {
            return Err(BundleValidationErrorV0::InvalidDigest);
        }
    }

    if source_receipt.episode_id != binding.source_episode_id
        || source_receipt.process_episode_receipt_sha256
            != binding.source_episode_receipt_sha256
        || source_receipt.final_evidence_prefix_sha256
            != binding.source_evidence_prefix_sha256
    {
        return Err(BundleValidationErrorV0::SourceReceiptBindingMismatch);
    }

    if target_receipt.episode_id != binding.target_episode_id
        || target_receipt.process_episode_receipt_sha256
            != binding.target_episode_receipt_sha256
        || target_receipt.final_evidence_prefix_sha256
            != binding.target_evidence_prefix_sha256
    {
        return Err(BundleValidationErrorV0::TargetReceiptBindingMismatch);
    }

    if binding.submission_episode_id != binding.target_episode_id
        || submit_frame.episode_id != binding.target_episode_id
    {
        return Err(BundleValidationErrorV0::SubmissionEpisodeMismatch);
    }

    if binding.submission_evidence_prefix_sha256 != binding.target_evidence_prefix_sha256
        || submit_frame.evidence_prefix_sha256 != binding.submission_evidence_prefix_sha256
    {
        return Err(BundleValidationErrorV0::SubmissionEvidencePrefixMismatch);
    }

    if submit_frame.action_kind != ProcessActionKindV0::Submit {
        return Err(BundleValidationErrorV0::SubmitActionRequired);
    }

    if submit_frame.agent_view_frame_sha256 != binding.agent_view_frame_sha256 {
        return Err(BundleValidationErrorV0::AgentViewFrameMismatch);
    }
    if submit_frame.agent_action_frame_sha256 != binding.agent_action_frame_sha256 {
        return Err(BundleValidationErrorV0::AgentActionFrameMismatch);
    }

    Ok(ProcessBindingWitnessV0 {
        source_episode_bound: true,
        target_episode_bound: true,
        submit_frame_bound: true,
    })
}

/// Check that a learner-authored structure is supported only by evidence IDs
/// present in the exact source or target public prefix. The owner still needs
/// to reconstruct/qualify the claimed structure independently; this function
/// establishes citation membership, not observational truth.
pub fn validate_evidence_bound_scoped_transfer(
    source_prefix: &PublicEvidencePrefixV0,
    target_prefix: &PublicEvidencePrefixV0,
    source: &EvidenceBoundStructureV0,
    target: &EvidenceBoundStructureV0,
    candidate: &ScopedStructuralCandidateV0,
) -> Result<EvidenceBoundTransferWitnessV0, BundleValidationErrorV0> {
    validate_prefix_binding(source_prefix, source)?;
    validate_prefix_binding(target_prefix, target)?;

    let source_counts = validate_structure_evidence(source, source_prefix)?;
    let target_counts = validate_structure_evidence(target, target_prefix)?;

    let structural = validate_scoped_structural_correspondence(
        &source.structure,
        &target.structure,
        candidate,
    )
    .map_err(BundleValidationErrorV0::ScopedStructuralRejected)?;

    Ok(EvidenceBoundTransferWitnessV0 {
        structural,
        source_node_evidence_references: source_counts.0,
        source_relation_evidence_references: source_counts.1,
        target_node_evidence_references: target_counts.0,
        target_relation_evidence_references: target_counts.1,
    })
}

fn validate_prefix_binding(
    prefix: &PublicEvidencePrefixV0,
    structure: &EvidenceBoundStructureV0,
) -> Result<(), BundleValidationErrorV0> {
    if prefix.episode_id != structure.episode_id {
        return Err(BundleValidationErrorV0::PrefixEpisodeMismatch);
    }
    if !is_sha256_digest(&prefix.evidence_prefix_sha256)
        || !is_sha256_digest(&structure.evidence_prefix_sha256)
    {
        return Err(BundleValidationErrorV0::InvalidDigest);
    }
    if prefix.evidence_prefix_sha256 != structure.evidence_prefix_sha256 {
        return Err(BundleValidationErrorV0::PrefixDigestMismatch);
    }

    let mut available = BTreeSet::new();
    for evidence_id in &prefix.available_evidence_ids {
        if !is_evidence_id(evidence_id) {
            return Err(BundleValidationErrorV0::InvalidEvidenceId);
        }
        if !available.insert(evidence_id.as_str()) {
            return Err(BundleValidationErrorV0::DuplicateAvailableEvidenceId);
        }
    }
    Ok(())
}

fn validate_structure_evidence(
    structure: &EvidenceBoundStructureV0,
    prefix: &PublicEvidencePrefixV0,
) -> Result<(usize, usize), BundleValidationErrorV0> {
    let available = prefix
        .available_evidence_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    let node_references = validate_entity_evidence(
        structure
            .structure
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>(),
        &structure.node_evidence,
        &available,
    )?;
    let relation_references = validate_entity_evidence(
        structure
            .structure
            .relations
            .iter()
            .map(|relation| relation.id)
            .collect::<BTreeSet<_>>(),
        &structure.relation_evidence,
        &available,
    )?;

    Ok((node_references, relation_references))
}

fn validate_entity_evidence(
    known_entity_ids: BTreeSet<u32>,
    bindings: &[EntityEvidenceBindingV0],
    available_evidence_ids: &BTreeSet<&str>,
) -> Result<usize, BundleValidationErrorV0> {
    let mut bound_entities = BTreeSet::new();
    let mut reference_count = 0_usize;

    for binding in bindings {
        if !bound_entities.insert(binding.entity_id) {
            return Err(BundleValidationErrorV0::DuplicateEntityEvidenceBinding);
        }
        if !known_entity_ids.contains(&binding.entity_id) {
            return Err(BundleValidationErrorV0::UnknownEntityEvidenceBinding);
        }
        if binding.evidence_ids.is_empty() {
            return Err(BundleValidationErrorV0::EmptyEntityEvidence);
        }

        let mut entity_evidence_ids = BTreeSet::new();
        for evidence_id in &binding.evidence_ids {
            if !is_evidence_id(evidence_id) {
                return Err(BundleValidationErrorV0::InvalidEvidenceId);
            }
            if !entity_evidence_ids.insert(evidence_id.as_str()) {
                return Err(BundleValidationErrorV0::DuplicateEntityEvidenceId);
            }
            if !available_evidence_ids.contains(evidence_id.as_str()) {
                return Err(BundleValidationErrorV0::UnknownEvidenceReference);
            }
            reference_count = reference_count
                .checked_add(1)
                .ok_or(BundleValidationErrorV0::InvalidEvidenceId)?;
        }
    }

    if known_entity_ids
        .iter()
        .any(|entity_id| !bound_entities.contains(entity_id))
    {
        return Err(BundleValidationErrorV0::MissingEntityEvidenceBinding);
    }

    Ok(reference_count)
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_evidence_id(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('E') else {
        return false;
    };
    digits.len() >= 4 && digits.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::super::scoped::{
        StructuralMapKindV0, StructureScopeV0,
    };
    use super::super::transfer::{
        NodeCorrespondenceV0, NodeV0, ParticipantTransformV0, RelationCorrespondenceV0,
        RelationKindCorrespondenceV0, RelationOrientationV0, RelationV0, StructuralCorrespondenceV0,
    };
    use super::*;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn source_receipt() -> OwnerEpisodeReceiptIdentityV0 {
        OwnerEpisodeReceiptIdentityV0 {
            episode_id: "source-episode-001".to_owned(),
            process_episode_receipt_sha256: digest('5'),
            final_evidence_prefix_sha256: digest('1'),
        }
    }

    fn target_receipt() -> OwnerEpisodeReceiptIdentityV0 {
        OwnerEpisodeReceiptIdentityV0 {
            episode_id: "target-episode-014".to_owned(),
            process_episode_receipt_sha256: digest('6'),
            final_evidence_prefix_sha256: digest('2'),
        }
    }

    fn submit_frame() -> ProcessSubmitFrameV0 {
        ProcessSubmitFrameV0 {
            episode_id: "target-episode-014".to_owned(),
            evidence_prefix_sha256: digest('2'),
            agent_view_frame_sha256: digest('4'),
            agent_action_frame_sha256: digest('3'),
            action_kind: ProcessActionKindV0::Submit,
        }
    }

    fn valid_binding() -> TransferSubmissionBindingV0 {
        TransferSubmissionBindingV0 {
            agent_protocol_version: AGENT_PROTOCOL_VERSION_V0.to_owned(),
            submission_episode_id: "target-episode-014".to_owned(),
            submission_evidence_prefix_sha256: digest('2'),
            agent_view_frame_sha256: digest('4'),
            agent_action_frame_sha256: digest('3'),
            source_episode_id: "source-episode-001".to_owned(),
            target_episode_id: "target-episode-014".to_owned(),
            source_evidence_prefix_sha256: digest('1'),
            target_evidence_prefix_sha256: digest('2'),
            source_episode_receipt_sha256: digest('5'),
            target_episode_receipt_sha256: digest('6'),
        }
    }

    fn source_structure() -> EvidenceBoundStructureV0 {
        EvidenceBoundStructureV0 {
            episode_id: "source-episode-001".to_owned(),
            evidence_prefix_sha256: digest('1'),
            structure: StructureV0 {
                nodes: vec![NodeV0 { id: 1 }, NodeV0 { id: 2 }, NodeV0 { id: 3 }],
                relations: vec![
                    RelationV0 {
                        id: 10,
                        kind: 1,
                        orientation: RelationOrientationV0::Undirected,
                        participants: vec![1, 2],
                    },
                    RelationV0 {
                        id: 11,
                        kind: 1,
                        orientation: RelationOrientationV0::Undirected,
                        participants: vec![2, 3],
                    },
                ],
            },
            node_evidence: vec![
                EntityEvidenceBindingV0 { entity_id: 1, evidence_ids: vec!["E0001".to_owned()] },
                EntityEvidenceBindingV0 { entity_id: 2, evidence_ids: vec!["E0002".to_owned()] },
                EntityEvidenceBindingV0 { entity_id: 3, evidence_ids: vec!["E0003".to_owned()] },
            ],
            relation_evidence: vec![
                EntityEvidenceBindingV0 { entity_id: 10, evidence_ids: vec!["E0004".to_owned()] },
                EntityEvidenceBindingV0 { entity_id: 11, evidence_ids: vec!["E0005".to_owned()] },
            ],
        }
    }

    fn target_structure() -> EvidenceBoundStructureV0 {
        EvidenceBoundStructureV0 {
            episode_id: "target-episode-014".to_owned(),
            evidence_prefix_sha256: digest('2'),
            structure: StructureV0 {
                nodes: vec![NodeV0 { id: 11 }, NodeV0 { id: 12 }, NodeV0 { id: 13 }],
                relations: vec![
                    RelationV0 {
                        id: 110,
                        kind: 100,
                        orientation: RelationOrientationV0::Undirected,
                        participants: vec![11, 12],
                    },
                    RelationV0 {
                        id: 111,
                        kind: 100,
                        orientation: RelationOrientationV0::Undirected,
                        participants: vec![12, 13],
                    },
                ],
            },
            node_evidence: vec![
                EntityEvidenceBindingV0 { entity_id: 11, evidence_ids: vec!["E0011".to_owned()] },
                EntityEvidenceBindingV0 { entity_id: 12, evidence_ids: vec!["E0012".to_owned()] },
                EntityEvidenceBindingV0 { entity_id: 13, evidence_ids: vec!["E0013".to_owned()] },
            ],
            relation_evidence: vec![
                EntityEvidenceBindingV0 { entity_id: 110, evidence_ids: vec!["E0014".to_owned()] },
                EntityEvidenceBindingV0 { entity_id: 111, evidence_ids: vec!["E0015".to_owned()] },
            ],
        }
    }

    fn source_prefix() -> PublicEvidencePrefixV0 {
        PublicEvidencePrefixV0 {
            episode_id: "source-episode-001".to_owned(),
            evidence_prefix_sha256: digest('1'),
            available_evidence_ids: (1..=5).map(|index| format!("E{index:04}")).collect(),
        }
    }

    fn target_prefix() -> PublicEvidencePrefixV0 {
        PublicEvidencePrefixV0 {
            episode_id: "target-episode-014".to_owned(),
            evidence_prefix_sha256: digest('2'),
            available_evidence_ids: (11..=15).map(|index| format!("E{index:04}")).collect(),
        }
    }

    fn candidate() -> ScopedStructuralCandidateV0 {
        ScopedStructuralCandidateV0 {
            map_kind: StructuralMapKindV0::InducedEmbedding,
            source_scope: StructureScopeV0 {
                node_ids: vec![1, 2],
                relation_ids: vec![10],
                excluded_node_ids: vec![3],
                excluded_relation_ids: vec![11],
            },
            target_scope: StructureScopeV0 {
                node_ids: vec![11, 12],
                relation_ids: vec![110],
                excluded_node_ids: vec![13],
                excluded_relation_ids: vec![111],
            },
            correspondence: StructuralCorrespondenceV0 {
                nodes: vec![
                    NodeCorrespondenceV0 { source_node_id: 1, target_node_id: 11 },
                    NodeCorrespondenceV0 { source_node_id: 2, target_node_id: 12 },
                ],
                relation_kinds: vec![RelationKindCorrespondenceV0 {
                    source_kind: 1,
                    target_kind: 100,
                    participant_transform: ParticipantTransformV0::Unordered,
                }],
                relations: vec![RelationCorrespondenceV0 {
                    source_relation_id: 10,
                    target_relation_id: 110,
                }],
            },
        }
    }

    #[test]
    fn accepts_exact_process_frame_and_episode_receipt_bindings() {
        assert_eq!(
            validate_transfer_submission_binding(
                &valid_binding(),
                &source_receipt(),
                &target_receipt(),
                &submit_frame(),
            ),
            Ok(ProcessBindingWitnessV0 {
                source_episode_bound: true,
                target_episode_bound: true,
                submit_frame_bound: true,
            })
        );
    }

    #[test]
    fn rejects_probe_frame_as_transfer_submission() {
        let mut frame = submit_frame();
        frame.action_kind = ProcessActionKindV0::Probe;
        assert_eq!(
            validate_transfer_submission_binding(
                &valid_binding(),
                &source_receipt(),
                &target_receipt(),
                &frame,
            ),
            Err(BundleValidationErrorV0::SubmitActionRequired)
        );
    }

    #[test]
    fn rejects_mismatched_frame_hashes() {
        let mut binding = valid_binding();
        binding.agent_action_frame_sha256 = digest('a');
        assert_eq!(
            validate_transfer_submission_binding(
                &binding,
                &source_receipt(),
                &target_receipt(),
                &submit_frame(),
            ),
            Err(BundleValidationErrorV0::AgentActionFrameMismatch)
        );
    }

    #[test]
    fn rejects_source_or_target_episode_receipt_mismatch() {
        let mut source = source_receipt();
        source.process_episode_receipt_sha256 = digest('a');
        assert_eq!(
            validate_transfer_submission_binding(
                &valid_binding(),
                &source,
                &target_receipt(),
                &submit_frame(),
            ),
            Err(BundleValidationErrorV0::SourceReceiptBindingMismatch)
        );

        let mut target = target_receipt();
        target.final_evidence_prefix_sha256 = digest('a');
        assert_eq!(
            validate_transfer_submission_binding(
                &valid_binding(),
                &source_receipt(),
                &target,
                &submit_frame(),
            ),
            Err(BundleValidationErrorV0::TargetReceiptBindingMismatch)
        );
    }

    #[test]
    fn rejects_a_submit_frame_for_the_wrong_episode() {
        let mut frame = submit_frame();
        frame.episode_id = "source-episode-001".to_owned();
        assert_eq!(
            validate_transfer_submission_binding(
                &valid_binding(),
                &source_receipt(),
                &target_receipt(),
                &frame,
            ),
            Err(BundleValidationErrorV0::SubmissionEpisodeMismatch)
        );
    }

    #[test]
    fn accepts_evidence_bound_scoped_transfer_candidate() {
        let source = source_structure();
        let target = target_structure();
        let witness = validate_evidence_bound_scoped_transfer(
            &source_prefix(),
            &target_prefix(),
            &source,
            &target,
            &candidate(),
        )
        .unwrap();
        assert_eq!(witness.structural.structural.mapped_nodes, 2);
        assert_eq!(witness.source_node_evidence_references, 3);
        assert_eq!(witness.source_relation_evidence_references, 2);
        assert_eq!(witness.target_node_evidence_references, 3);
        assert_eq!(witness.target_relation_evidence_references, 2);
    }

    #[test]
    fn rejects_evidence_id_outside_public_prefix() {
        let mut source = source_structure();
        source.node_evidence[0].evidence_ids[0] = "E9999".to_owned();
        assert_eq!(
            validate_evidence_bound_scoped_transfer(
                &source_prefix(),
                &target_prefix(),
                &source,
                &target_structure(),
                &candidate(),
            ),
            Err(BundleValidationErrorV0::UnknownEvidenceReference)
        );
    }

    #[test]
    fn rejects_missing_entity_evidence_binding() {
        let mut target = target_structure();
        target.relation_evidence.pop();
        assert_eq!(
            validate_evidence_bound_scoped_transfer(
                &source_prefix(),
                &target_prefix(),
                &source_structure(),
                &target,
                &candidate(),
            ),
            Err(BundleValidationErrorV0::MissingEntityEvidenceBinding)
        );
    }

    #[test]
    fn rejects_snapshot_episode_and_prefix_mismatch() {
        let mut target = target_structure();
        target.evidence_prefix_sha256 = digest('3');
        assert_eq!(
            validate_evidence_bound_scoped_transfer(
                &source_prefix(),
                &target_prefix(),
                &source_structure(),
                &target,
                &candidate(),
            ),
            Err(BundleValidationErrorV0::PrefixDigestMismatch)
        );
    }

    #[test]
    fn rejects_duplicate_entity_evidence_references() {
        let mut source = source_structure();
        source.node_evidence[0].evidence_ids.push("E0001".to_owned());
        assert_eq!(
            validate_evidence_bound_scoped_transfer(
                &source_prefix(),
                &target_prefix(),
                &source,
                &target_structure(),
                &candidate(),
            ),
            Err(BundleValidationErrorV0::DuplicateEntityEvidenceId)
        );
    }

    #[test]
    fn rejects_incomplete_scoped_structural_map_after_evidence_validation() {
        let mut candidate = candidate();
        candidate.correspondence.nodes.pop();
        assert!(matches!(
            validate_evidence_bound_scoped_transfer(
                &source_prefix(),
                &target_prefix(),
                &source_structure(),
                &target_structure(),
                &candidate,
            ),
            Err(BundleValidationErrorV0::ScopedStructuralRejected(_))
        ));
    }
}

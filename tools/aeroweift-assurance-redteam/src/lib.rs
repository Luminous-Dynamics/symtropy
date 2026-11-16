//! Seeded adversarial challenge harness for AeroWeft operational assurance cases.
//!
//! This crate mutates the actual Track W assurance-case types, evaluates them with
//! Track W, and compares the outcome with a separately implemented finite invariant
//! oracle. It reports counterexamples and minimizes witnesses, but never proves the
//! hazard universe complete or declares an operation safe.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use aeroweift_assurance_case::{
    AssuranceCasePolicy, AssuranceEdge, AssuranceNode, AssuranceStatus, AuthorityDecisionStatus,
    ClaimState, EdgeKind, EvaluationError, EvidenceDisposition, EvidencePurpose,
    EvidenceSourceKind, NodeKind, evaluate_assurance_case,
};

/// Generator version pinned into every replay record.
pub const GENERATOR_VERSION: &str = "aeroweift-assurance-redteam-v0.1";

/// One frozen Track W assurance-case input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseSnapshot {
    /// Exact policy and operation scope.
    pub policy: AssuranceCasePolicy,
    /// Declared top-level claim identifier.
    pub root_claim_id: String,
    /// Assurance argument nodes.
    pub nodes: Vec<AssuranceNode>,
    /// Typed assurance argument relations.
    pub edges: Vec<AssuranceEdge>,
}

/// A domain-aware mutation attempted against one frozen case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttackMutation {
    /// Delete one typed relation.
    RemoveEdge {
        /// Source node identifier.
        from_node_id: String,
        /// Target node identifier.
        to_node_id: String,
        /// Exact relation kind.
        kind: EdgeKind,
    },
    /// Replace the purpose and owner of one evidence receipt.
    SubstituteEvidencePurpose {
        /// Evidence node identifier.
        node_id: String,
        /// New owner/source kind.
        source_kind: EvidenceSourceKind,
        /// New declared proposition.
        purpose: EvidencePurpose,
    },
    /// Change an evidence owner's disposition.
    ChangeEvidenceDisposition {
        /// Evidence node identifier.
        node_id: String,
        /// Adversarial disposition.
        disposition: EvidenceDisposition,
    },
    /// Splice evidence or an argument node into another aircraft scope.
    SpliceAircraftScope {
        /// Node identifier.
        node_id: String,
        /// Foreign pseudonymous aircraft identifier.
        aircraft_id: String,
    },
    /// Revoke/reject an external authority decision.
    ChangeAuthorityStatus {
        /// Authority decision node identifier.
        node_id: String,
        /// Adversarial decision state.
        status: AuthorityDecisionStatus,
    },
    /// Make a node expire before the operation interval ends.
    ShortenNodeValidity {
        /// Node identifier.
        node_id: String,
        /// New exclusive validity end.
        valid_until: i64,
    },
    /// Remove the declared correlation group from one evidence reference.
    RemoveCorrelationGroup {
        /// Evidence node identifier.
        node_id: String,
    },
}

/// One generated attack with stable identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackSpec {
    /// Stable attack identifier.
    pub attack_id: String,
    /// Mutation to apply.
    pub mutation: AttackMutation,
}

/// Independent invariant violation found by the red-team oracle.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OracleViolation {
    /// A node's case/aircraft/operation scope differs from the frozen case.
    ScopeSplice,
    /// A relied-on node expires before the full operation window ends.
    NodeValidityShort,
    /// A required hazard is missing.
    RequiredHazardMissing,
    /// A hazard is present but is not linked from a claim or strategy.
    HazardNotAddressedByArgument,
    /// A required hazard has no mitigation.
    HazardWithoutMitigation,
    /// A mitigation does not have a supported owner state.
    MitigationNotSupported,
    /// A claim/mitigation lacks its required typed evidence proposition.
    RequiredEvidencePurposeMissing,
    /// Evidence purpose is incompatible with its declared source owner.
    EvidenceOwnerPurposeMismatch,
    /// Independent evidence groups do not meet the policy floor.
    InsufficientIndependentEvidenceGroups,
    /// An authority decision is missing, duplicated, rejected or outside its time scope.
    AuthorityDecisionInvalid,
    /// Planning-only or generic CI evidence is represented as operational evidence.
    NonOperationalEvidencePromoted,
}

/// Classification of the attack result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttackDisposition {
    /// An independent invariant oracle found a violation and Track W blocked the case.
    ConfirmedCounterexample,
    /// An independent invariant oracle found a violation but Track W returned review-ready.
    MissedCounterexample,
    /// Track W blocked/rejected the case but the finite independent oracle found no violation.
    VerifierOracleDisagreement,
    /// The mutation was applied and no counterexample was found within this declared profile.
    NoCounterexampleFoundWithinBudget,
}

/// Track W's observed outcome on one mutated case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifierObservation {
    /// Track W status when graph evaluation returned a result.
    pub status: Option<AssuranceStatus>,
    /// Structural input error, if Track W rejected before producing a status.
    pub error: Option<EvaluationError>,
}

impl VerifierObservation {
    fn from_case(case: &CaseSnapshot) -> Self {
        match evaluate_assurance_case(
            &case.nodes,
            &case.edges,
            &case.root_claim_id,
            &case.policy,
        ) {
            Ok(result) => Self {
                status: Some(result.status),
                error: None,
            },
            Err(error) => Self {
                status: None,
                error: Some(error),
            },
        }
    }

    fn is_blocked(self) -> bool {
        self.error.is_some()
            || self
                .status
                .is_some_and(|status| status != AssuranceStatus::ReviewReadyOnly)
    }

    fn is_review_ready(self) -> bool {
        self.status == Some(AssuranceStatus::ReviewReadyOnly) && self.error.is_none()
    }
}

/// Reproducibility metadata for an attack result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayMetadata {
    /// Generator version.
    pub generator_version: String,
    /// Deterministic campaign seed.
    pub seed: u64,
    /// Exact frozen case identifier.
    pub case_id: String,
    /// Exact frozen policy digest string.
    pub policy_sha256: String,
    /// Attack identifier.
    pub attack_id: String,
}

/// Minimized witness preserving the same oracle violations and verifier observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimizedWitness {
    /// Reduced, replayable case.
    pub case: CaseSnapshot,
    /// Node IDs removed during minimization.
    pub removed_node_ids: Vec<String>,
    /// Edges removed during minimization.
    pub removed_edges: Vec<AssuranceEdge>,
    /// Number of nodes in the original attacked case.
    pub original_node_count: usize,
    /// Number of edges in the original attacked case.
    pub original_edge_count: usize,
}

/// Complete result for one mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackReport {
    /// Replay metadata.
    pub replay: ReplayMetadata,
    /// Classification.
    pub disposition: AttackDisposition,
    /// Independently found violations, in deterministic order.
    pub oracle_violations: Vec<OracleViolation>,
    /// Track W outcome.
    pub verifier: VerifierObservation,
    /// Nodes directly changed by the attack.
    pub mutated_node_ids: Vec<String>,
    /// Minimized witness for reproducibility.
    pub minimized_witness: MinimizedWitness,
}

/// An attack cannot be executed when the case or mutation preconditions are invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttackError {
    /// The supplied case is not a positive control under the frozen profile.
    BaselineNotReviewReady,
    /// Mutation target is absent, has the wrong node kind, or is already in the requested state.
    InvalidAttack,
}

/// Configuration for deterministic attack generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CampaignConfig {
    /// Seed used to shuffle the deterministic candidate catalogue.
    pub seed: u64,
    /// Maximum number of attacks to execute.
    pub attack_budget: usize,
}

/// Summary of one seeded campaign.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignReport {
    /// Generator and case replay metadata.
    pub replay: ReplayMetadata,
    /// Number of candidates considered before budget truncation.
    pub candidate_count: usize,
    /// Executed reports, in deterministic seeded order.
    pub attacks: Vec<AttackReport>,
}

/// Check the allowed owner-to-purpose pairs independently of Track W.
fn owner_matches_purpose(source: EvidenceSourceKind, purpose: EvidencePurpose) -> bool {
    matches!(
        (source, purpose),
        (EvidenceSourceKind::TrackL, EvidencePurpose::PntReachability)
            | (EvidenceSourceKind::TrackM, EvidencePurpose::SurveillanceObservability)
            | (EvidenceSourceKind::TrackN, EvidencePurpose::IntentConformance)
            | (EvidenceSourceKind::TrackP, EvidencePurpose::CausalReceiptBinding)
            | (EvidenceSourceKind::TrackQ, EvidencePurpose::AuthenticatedCustody)
            | (EvidenceSourceKind::TrackR, EvidencePurpose::ConflictPerformance)
            | (EvidenceSourceKind::TrackS, EvidencePurpose::MultiSubjectScope)
            | (EvidenceSourceKind::TrackT, EvidencePurpose::ModelCredibility)
            | (EvidenceSourceKind::TrackU, EvidencePurpose::ModelRequalification)
            | (EvidenceSourceKind::TrackVPlanning, EvidencePurpose::ExperimentPlan)
            | (
                EvidenceSourceKind::ExternalFlightTest,
                EvidencePurpose::PhysicalMitigationEffectiveness
            )
            | (
                EvidenceSourceKind::ExternalMeasurement,
                EvidencePurpose::PhysicalMitigationEffectiveness
                    | EvidencePurpose::ReferenceMeasurement
            )
            | (
                EvidenceSourceKind::ExternalAuthority,
                EvidencePurpose::ExternalAuthorityRecord
            )
            | (
                EvidenceSourceKind::GenericSoftwareCi,
                EvidencePurpose::GenericSoftwareVerification
            )
            | (EvidenceSourceKind::Other, EvidencePurpose::Unclassified)
    )
}

/// Independently evaluate a finite set of policy-declared assurance invariants.
///
/// This deliberately does not call Track W. It is not a complete hazard oracle,
/// a physical truth oracle, or a replacement for domain-owner evaluation.
pub fn independent_oracle(case: &CaseSnapshot) -> BTreeSet<OracleViolation> {
    let mut violations = BTreeSet::new();
    let policy = &case.policy;
    let by_id: BTreeMap<&str, &AssuranceNode> = case
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect();

    // Keep malformed/ disconnected mutations distinct from semantic counterexamples.
    let mut edge_keys = BTreeSet::new();
    if !by_id.contains_key(case.root_claim_id.as_str())
        || case.nodes.iter().any(|node| node.node_id.trim().is_empty())
        || by_id.len() != case.nodes.len()
        || case.edges.iter().any(|edge| {
            !by_id.contains_key(edge.from_node_id.as_str())
                || !by_id.contains_key(edge.to_node_id.as_str())
                || !edge_keys.insert((
                    edge.from_node_id.as_str(),
                    edge.to_node_id.as_str(),
                    edge.kind,
                ))
        })
    {
        violations.insert(OracleViolation::GraphStructureInvalid);
    } else {
        let mut reached = BTreeSet::from([case.root_claim_id.as_str()]);
        loop {
            let before = reached.len();
            for edge in &case.edges {
                if reached.contains(edge.from_node_id.as_str()) {
                    reached.insert(edge.to_node_id.as_str());
                }
                if reached.contains(edge.to_node_id.as_str()) {
                    reached.insert(edge.from_node_id.as_str());
                }
            }
            if reached.len() == before {
                break;
            }
        }
        if reached.len() != case.nodes.len() {
            violations.insert(OracleViolation::GraphStructureInvalid);
        }

        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &case.edges {
            adjacency
                .entry(edge.from_node_id.as_str())
                .or_default()
                .push(edge.to_node_id.as_str());
        }
        let mut colors: BTreeMap<&str, u8> = BTreeMap::new();
        let mut stack: Vec<(&str, bool)> = vec![(case.root_claim_id.as_str(), false)];
        while let Some((id, exiting)) = stack.pop() {
            if exiting {
                colors.insert(id, 2);
                continue;
            }
            match colors.get(id).copied().unwrap_or(0) {
                1 => {
                    violations.insert(OracleViolation::GraphStructureInvalid);
                    break;
                }
                2 => continue,
                _ => {}
            }
            colors.insert(id, 1);
            stack.push((id, true));
            if let Some(children) = adjacency.get(id) {
                for child in children.iter().rev() {
                    if colors.get(child).copied() == Some(1) {
                        violations.insert(OracleViolation::GraphStructureInvalid);
                        break;
                    }
                    if colors.get(child).copied() != Some(2) {
                        stack.push((child, false));
                    }
                }
            }
            if violations.contains(&OracleViolation::GraphStructureInvalid) {
                break;
            }
        }
    }

    for node in &case.nodes {
        if node.scope != policy.expected_scope {
            violations.insert(OracleViolation::ScopeSplice);
        }
        if node.valid_from > policy.expected_scope.operation_start
            || node.valid_until < policy.expected_scope.operation_end
        {
            violations.insert(OracleViolation::NodeValidityShort);
        }
        if let Some(evidence) = node.evidence.as_ref() {
            if !owner_matches_purpose(evidence.source_kind, evidence.purpose) {
                violations.insert(OracleViolation::EvidenceOwnerPurposeMismatch);
            }
            if matches!(
                evidence.source_kind,
                EvidenceSourceKind::GenericSoftwareCi | EvidenceSourceKind::TrackVPlanning
            ) && case.edges.iter().any(|edge| {
                edge.from_node_id == node.node_id
                    && edge.kind == EdgeKind::Supports
                    && by_id
                        .get(edge.to_node_id.as_str())
                        .is_some_and(|target| {
                            matches!(target.kind, NodeKind::TopClaim | NodeKind::Claim | NodeKind::Mitigation)
                        })
            }) {
                violations.insert(OracleViolation::NonOperationalEvidencePromoted);
            }
        }
    }

    for requirement in &policy.required_claim_evidence {
        let Some(claim_node) = by_id.get(requirement.claim_id.as_str()).copied() else {
            violations.insert(OracleViolation::RequiredEvidencePurposeMissing);
            continue;
        };
        let Some(claim) = claim_node.claim.as_ref() else {
            violations.insert(OracleViolation::RequiredEvidencePurposeMissing);
            continue;
        };
        if claim.state != ClaimState::Supported {
            violations.insert(OracleViolation::ClaimNotSupported);
        }
        let supporting: Vec<&AssuranceNode> = case
            .edges
            .iter()
            .filter(|edge| edge.to_node_id == requirement.claim_id && edge.kind == EdgeKind::Supports)
            .filter_map(|edge| by_id.get(edge.from_node_id.as_str()).copied())
            .filter(|node| node.kind == NodeKind::Evidence)
            .collect();

        for purpose in &requirement.required_purposes {
            if !supporting.iter().any(|evidence_node| {
                evidence_node.evidence.as_ref().is_some_and(|evidence| {
                    evidence.purpose == *purpose
                        && evidence.disposition == EvidenceDisposition::Qualified
                        && owner_matches_purpose(evidence.source_kind, evidence.purpose)
                })
            }) {
                violations.insert(OracleViolation::RequiredEvidencePurposeMissing);
            }
        }

        let eligible_groups: BTreeSet<&str> = supporting
            .iter()
            .filter_map(|node| {
                let evidence = node.evidence.as_ref()?;
                (requirement.required_purposes.contains(&evidence.purpose)
                    && evidence.disposition == EvidenceDisposition::Qualified
                    && owner_matches_purpose(evidence.source_kind, evidence.purpose))
                    .then_some(evidence.correlation_group_id.as_deref())
                    .flatten()
            })
            .collect();
        let required = claim
            .minimum_independent_evidence_groups
            .max(requirement.minimum_independent_evidence_groups);
        if eligible_groups.len() < required {
            violations.insert(OracleViolation::InsufficientIndependentEvidenceGroups);
        }
    }

    for requirement in &policy.required_mitigation_evidence {
        let Some(mitigation_node) = by_id.get(requirement.mitigation_id.as_str()).copied() else {
            violations.insert(OracleViolation::RequiredEvidencePurposeMissing);
            continue;
        };
        if mitigation_node.kind != NodeKind::Mitigation
            || mitigation_node.mitigation_state != Some(ClaimState::Supported)
        {
            violations.insert(OracleViolation::MitigationNotSupported);
        }
        let supporting: Vec<&AssuranceNode> = case
            .edges
            .iter()
            .filter(|edge| edge.to_node_id == requirement.mitigation_id && edge.kind == EdgeKind::Supports)
            .filter_map(|edge| by_id.get(edge.from_node_id.as_str()).copied())
            .filter(|node| node.kind == NodeKind::Evidence)
            .collect();

        for purpose in &requirement.required_purposes {
            if !supporting.iter().any(|node| {
                node.evidence.as_ref().is_some_and(|evidence| {
                    evidence.purpose == *purpose
                        && evidence.disposition == EvidenceDisposition::Qualified
                        && owner_matches_purpose(evidence.source_kind, evidence.purpose)
                })
            }) {
                violations.insert(OracleViolation::RequiredEvidencePurposeMissing);
            }
        }

        let groups: BTreeSet<&str> = supporting
            .iter()
            .filter_map(|node| {
                let evidence = node.evidence.as_ref()?;
                (requirement.required_purposes.contains(&evidence.purpose)
                    && evidence.disposition == EvidenceDisposition::Qualified
                    && owner_matches_purpose(evidence.source_kind, evidence.purpose))
                    .then_some(evidence.correlation_group_id.as_deref())
                    .flatten()
            })
            .collect();
        if groups.len() < requirement.minimum_independent_evidence_groups {
            violations.insert(OracleViolation::InsufficientIndependentEvidenceGroups);
        }
    }

    for hazard_id in &policy.required_hazards {
        let hazards: Vec<&AssuranceNode> = case
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Hazard && node.hazard_id.as_deref() == Some(hazard_id))
            .collect();
        if hazards.is_empty() {
            violations.insert(OracleViolation::RequiredHazardMissing);
            continue;
        }
        for hazard in hazards {
            let addressed = case.edges.iter().any(|edge| {
                edge.to_node_id == hazard.node_id
                    && edge.kind == EdgeKind::AddressesHazard
                    && by_id.get(edge.from_node_id.as_str()).is_some_and(|node| {
                        matches!(node.kind, NodeKind::TopClaim | NodeKind::Claim | NodeKind::Strategy)
                    })
            });
            if !addressed {
                violations.insert(OracleViolation::HazardNotAddressedByArgument);
            }
            let mitigation_ids: Vec<&str> = case
                .edges
                .iter()
                .filter(|edge| edge.to_node_id == hazard.node_id && edge.kind == EdgeKind::Mitigates)
                .filter_map(|edge| by_id.get(edge.from_node_id.as_str()))
                .filter(|node| node.kind == NodeKind::Mitigation)
                .map(|node| node.node_id.as_str())
                .collect();
            if mitigation_ids.is_empty() {
                violations.insert(OracleViolation::HazardWithoutMitigation);
            }
            for mitigation_id in mitigation_ids {
                if !policy
                    .required_mitigation_evidence
                    .iter()
                    .any(|required| required.mitigation_id == mitigation_id)
                {
                    violations.insert(OracleViolation::RequiredEvidencePurposeMissing);
                }
            }
        }
    }

    for required in &policy.required_authorities {
        let decisions: Vec<&AssuranceNode> = case
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::AuthorityDecision)
            .filter(|node| {
                node.authority_decision.as_ref().is_some_and(|decision| {
                    decision.role == required.role && decision.authority_id == required.authority_id
                })
            })
            .collect();
        if decisions.len() != 1 {
            violations.insert(OracleViolation::AuthorityDecisionInvalid);
            continue;
        }
        let decision = decisions[0].authority_decision.as_ref().expect("validated kind");
        if decision.status != AuthorityDecisionStatus::Accepted
            || decision.effective_at > policy.expected_scope.operation_start
            || decision.recorded_at > policy.expected_scope.knowledge_cutoff
            || decision
                .valid_until
                .is_some_and(|end| end < policy.expected_scope.operation_end)
        {
            violations.insert(OracleViolation::AuthorityDecisionInvalid);
        }
    }

    violations
}

fn node_mut<'a>(case: &'a mut CaseSnapshot, id: &str) -> Option<&'a mut AssuranceNode> {
    case.nodes.iter_mut().find(|node| node.node_id == id)
}

/// Apply one mutation; returns directly mutated node identifiers.
fn apply_mutation(
    case: &mut CaseSnapshot,
    mutation: &AttackMutation,
) -> Result<Vec<String>, AttackError> {
    match mutation {
        AttackMutation::RemoveEdge {
            from_node_id,
            to_node_id,
            kind,
        } => {
            let before = case.edges.len();
            case.edges.retain(|edge| {
                !(edge.from_node_id == *from_node_id
                    && edge.to_node_id == *to_node_id
                    && edge.kind == *kind)
            });
            if case.edges.len() == before {
                return Err(AttackError::InvalidAttack);
            }
            Ok(vec![from_node_id.clone(), to_node_id.clone()])
        }
        AttackMutation::SubstituteEvidencePurpose {
            node_id,
            source_kind,
            purpose,
        } => {
            let node = node_mut(case, node_id).ok_or(AttackError::InvalidAttack)?;
            let evidence = node.evidence.as_mut().ok_or(AttackError::InvalidAttack)?;
            if evidence.source_kind == *source_kind && evidence.purpose == *purpose {
                return Err(AttackError::InvalidAttack);
            }
            evidence.source_kind = *source_kind;
            evidence.purpose = *purpose;
            Ok(vec![node_id.clone()])
        }
        AttackMutation::ChangeEvidenceDisposition { node_id, disposition } => {
            let node = node_mut(case, node_id).ok_or(AttackError::InvalidAttack)?;
            let evidence = node.evidence.as_mut().ok_or(AttackError::InvalidAttack)?;
            if evidence.disposition == *disposition {
                return Err(AttackError::InvalidAttack);
            }
            evidence.disposition = *disposition;
            Ok(vec![node_id.clone()])
        }
        AttackMutation::SpliceAircraftScope { node_id, aircraft_id } => {
            let node = node_mut(case, node_id).ok_or(AttackError::InvalidAttack)?;
            if node.scope.aircraft_id == *aircraft_id {
                return Err(AttackError::InvalidAttack);
            }
            node.scope.aircraft_id = aircraft_id.clone();
            Ok(vec![node_id.clone()])
        }
        AttackMutation::ChangeAuthorityStatus { node_id, status } => {
            let node = node_mut(case, node_id).ok_or(AttackError::InvalidAttack)?;
            let decision = node
                .authority_decision
                .as_mut()
                .ok_or(AttackError::InvalidAttack)?;
            if decision.status == *status {
                return Err(AttackError::InvalidAttack);
            }
            decision.status = *status;
            Ok(vec![node_id.clone()])
        }
        AttackMutation::ShortenNodeValidity { node_id, valid_until } => {
            let node = node_mut(case, node_id).ok_or(AttackError::InvalidAttack)?;
            if *valid_until >= node.valid_until {
                return Err(AttackError::InvalidAttack);
            }
            node.valid_until = *valid_until;
            Ok(vec![node_id.clone()])
        }
        AttackMutation::RemoveCorrelationGroup { node_id } => {
            let node = node_mut(case, node_id).ok_or(AttackError::InvalidAttack)?;
            let evidence = node.evidence.as_mut().ok_or(AttackError::InvalidAttack)?;
            if evidence.correlation_group_id.is_none() {
                return Err(AttackError::InvalidAttack);
            }
            evidence.correlation_group_id = None;
            Ok(vec![node_id.clone()])
        }
    }
}

fn verifier_observation(case: &CaseSnapshot) -> VerifierObservation {
    match evaluate_assurance_case(
        &case.nodes,
        &case.edges,
        &case.root_claim_id,
        &case.policy,
    ) {
        Ok(result) => VerifierObservation {
            status: Some(result.status),
            error: None,
        },
        Err(error) => VerifierObservation {
            status: None,
            error: Some(error),
        },
    }
}

/// Check whether a candidate witness preserves the exact independent violations and
/// the same high-level Track W outcome.
fn preserves_witness(
    candidate: &CaseSnapshot,
    expected_violations: &BTreeSet<OracleViolation>,
    expected_verifier: VerifierObservation,
) -> bool {
    independent_oracle(candidate) == *expected_violations
        && verifier_observation(candidate) == expected_verifier
}

/// Greedily remove irrelevant nodes and edges while preserving the exact witness.
fn minimize_witness(
    attacked: &CaseSnapshot,
    expected_violations: &BTreeSet<OracleViolation>,
    expected_verifier: VerifierObservation,
) -> MinimizedWitness {
    let original_node_count = attacked.nodes.len();
    let original_edge_count = attacked.edges.len();
    let mut current = attacked.clone();
    let mut removed_node_ids = Vec::new();
    let mut removed_edges = Vec::new();

    let node_ids: Vec<String> = current.nodes.iter().map(|node| node.node_id.clone()).collect();
    for node_id in node_ids {
        if current.nodes.len() <= 1 {
            break;
        }
        let mut candidate = current.clone();
        candidate.nodes.retain(|node| node.node_id != node_id);
        candidate.edges.retain(|edge| {
            edge.from_node_id != node_id && edge.to_node_id != node_id
        });
        if preserves_witness(&candidate, expected_violations, expected_verifier) {
            current = candidate;
            removed_node_ids.push(node_id);
        }
    }

    let edge_candidates = current.edges.clone();
    for edge in edge_candidates {
        let mut candidate = current.clone();
        if let Some(index) = candidate.edges.iter().position(|item| item == &edge) {
            candidate.edges.remove(index);
            if preserves_witness(&candidate, expected_violations, expected_verifier) {
                current = candidate;
                removed_edges.push(edge);
            }
        }
    }

    MinimizedWitness {
        case: current,
        removed_node_ids,
        removed_edges,
        original_node_count,
        original_edge_count,
    }
}

/// Execute one adversarial mutation and compare independent oracle and Track W.
pub fn execute_attack(
    case: &CaseSnapshot,
    attack: &AttackSpec,
    seed: u64,
) -> Result<AttackReport, AttackError> {
    let baseline_violations = independent_oracle(case);
    let baseline_verifier = verifier_observation(case);
    if !baseline_violations.is_empty() || !baseline_verifier.is_review_ready() {
        return Err(AttackError::BaselineNotReviewReady);
    }

    let mut mutated = case.clone();
    let mutated_node_ids = apply_mutation(&mut mutated, &attack.mutation)?;
    let oracle_violations = independent_oracle(&mutated);
    let verifier = verifier_observation(&mutated);

    let disposition = if !oracle_violations.is_empty() && verifier.is_blocked() {
        AttackDisposition::ConfirmedCounterexample
    } else if !oracle_violations.is_empty() && verifier.is_review_ready() {
        AttackDisposition::MissedCounterexample
    } else if oracle_violations.is_empty() && verifier.is_blocked() {
        AttackDisposition::VerifierOracleDisagreement
    } else {
        AttackDisposition::NoCounterexampleFoundWithinBudget
    };

    let minimized_witness = minimize_witness(
        &mutated,
        &oracle_violations,
        verifier,
    );

    Ok(AttackReport {
        replay: ReplayMetadata {
            generator_version: GENERATOR_VERSION.to_owned(),
            seed,
            case_id: case.policy.expected_scope.case_id.clone(),
            policy_sha256: case.policy.policy_sha256.clone(),
            attack_id: attack.attack_id.clone(),
        },
        disposition,
        oracle_violations: oracle_violations.into_iter().collect(),
        verifier,
        mutated_node_ids,
        minimized_witness,
    })
}

/// Generate a deterministic candidate catalogue from the case dependency frontier.
pub fn generate_attacks(case: &CaseSnapshot, config: CampaignConfig) -> Vec<AttackSpec> {
    let mut candidates = Vec::new();

    for edge in &case.edges {
        if edge.kind == EdgeKind::AddressesHazard {
            candidates.push(AttackSpec {
                attack_id: format!("drop-hazard-link:{}->{}", edge.from_node_id, edge.to_node_id),
                mutation: AttackMutation::RemoveEdge {
                    from_node_id: edge.from_node_id.clone(),
                    to_node_id: edge.to_node_id.clone(),
                    kind: edge.kind,
                },
            });
        }
    }

    for node in &case.nodes {
        if let Some(evidence) = node.evidence.as_ref() {
            if evidence.source_kind == EvidenceSourceKind::TrackT
                && evidence.purpose == EvidencePurpose::ModelCredibility
            {
                candidates.push(AttackSpec {
                    attack_id: format!("swap-model-purpose:{}", node.node_id),
                    mutation: AttackMutation::SubstituteEvidencePurpose {
                        node_id: node.node_id.clone(),
                        source_kind: EvidenceSourceKind::TrackR,
                        purpose: EvidencePurpose::ConflictPerformance,
                    },
                });
            }
            if evidence.disposition == EvidenceDisposition::Qualified {
                candidates.push(AttackSpec {
                    attack_id: format!("historicalize-evidence:{}", node.node_id),
                    mutation: AttackMutation::ChangeEvidenceDisposition {
                        node_id: node.node_id.clone(),
                        disposition: EvidenceDisposition::HistoricalOnly,
                    },
                });
            }
            if evidence.correlation_group_id.is_some() {
                candidates.push(AttackSpec {
                    attack_id: format!("remove-correlation:{}", node.node_id),
                    mutation: AttackMutation::RemoveCorrelationGroup {
                        node_id: node.node_id.clone(),
                    },
                });
            }
        }
        if node.kind == NodeKind::AuthorityDecision {
            candidates.push(AttackSpec {
                attack_id: format!("reject-authority:{}", node.node_id),
                mutation: AttackMutation::ChangeAuthorityStatus {
                    node_id: node.node_id.clone(),
                    status: AuthorityDecisionStatus::Rejected,
                },
            });
        }
        if node.valid_until >= case.policy.expected_scope.operation_end {
            candidates.push(AttackSpec {
                attack_id: format!("expire-before-operation-end:{}", node.node_id),
                mutation: AttackMutation::ShortenNodeValidity {
                    node_id: node.node_id.clone(),
                    valid_until: case.policy.expected_scope.operation_end - 1,
                },
            });
        }
    }

    deterministic_shuffle(&mut candidates, config.seed);
    candidates.truncate(config.attack_budget);
    candidates
}

fn deterministic_shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed };
    for index in (1..items.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let other = (state as usize) % (index + 1);
        items.swap(index, other);
    }
}

/// Execute a deterministic, budgeted campaign against one frozen case.
pub fn run_campaign(
    case: &CaseSnapshot,
    config: CampaignConfig,
) -> Result<CampaignReport, AttackError> {
    if !independent_oracle(case).is_empty() || !verifier_observation(case).is_review_ready() {
        return Err(AttackError::BaselineNotReviewReady);
    }
    let candidates = generate_attacks(case, config);
    let candidate_count = candidates.len();
    let mut attacks = Vec::with_capacity(candidate_count);
    for attack in candidates {
        attacks.push(execute_attack(case, &attack, config.seed)?);
    }
    Ok(CampaignReport {
        replay: ReplayMetadata {
            generator_version: GENERATOR_VERSION.to_owned(),
            seed: config.seed,
            case_id: case.policy.expected_scope.case_id.clone(),
            policy_sha256: case.policy.policy_sha256.clone(),
            attack_id: "campaign".to_owned(),
        },
        candidate_count,
        attacks,
    })
}

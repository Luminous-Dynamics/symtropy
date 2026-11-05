//! Set-valued event correspondence -> typed stochastic address projection.
//!
//! Correspondence is not causal invariance. This module keeps the two decisions
//! separate and emits one address plan per admissible correspondence hypothesis.
//! It serializes semantic addresses only; the pinned Philox adapter remains the
//! downstream raw-block resolver.

use std::collections::{BTreeMap, BTreeSet};

use super::StableId;
use super::correspondence::{
    CorrespondenceDisposition, CorrespondenceError, CorrespondenceSet,
    EventOccurrence, EventPair, EventPresence, IdentityBasis, UnmatchedReason,
};

const DOMAIN: &[u8] = b"symtropy.road-correspondence-coupling.address.v0.2\0";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceProfile {
    pub source_id: StableId,
    pub mechanism_id: StableId,
    pub mechanism_version: StableId,
    pub draw_component: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDrawScope {
    pub branch_id: StableId,
    pub event_id: StableId,
    pub tick: u64,
    pub event_kind: StableId,
    pub source: SourceProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CausalDisposition {
    /// A causal model separately establishes that this source is invariant
    /// across this exact intervention. Event matching alone is insufficient.
    SharedInvariant {
        equivalence_group: StableId,
        justification: StableId,
    },
    /// The events correspond, but their stochastic inputs must remain local.
    BranchSpecific,
    /// A declared sensitivity coupling is evaluated, but not identified.
    SensitivityOnly {
        sensitivity_id: StableId,
        justification: StableId,
    },
    /// Evidence does not justify a cross-branch draw pairing.
    NotIdentified,
    /// Required causal evidence or manifest is incomplete.
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairCausalRule {
    pub left_event: StableId,
    pub right_event: StableId,
    pub basis: IdentityBasis,
    pub disposition: CausalDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharingStatus {
    SharedInvariant,
    SensitivityOnly,
    BranchSpecific,
    NotIdentified,
    Incomplete,
    NoCounterpartUnderHypothesis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedPresence {
    Observed { evidence_ref: StableId },
    NotRealized { condition_witness: StableId },
    Unknown { evidence_ref: StableId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedAddress {
    pub branch_id: StableId,
    pub event_id: StableId,
    /// Canonical v0.2 bytes. These are not a hash, commitment or random output.
    pub canonical_bytes: Vec<u8>,
    /// Presence and sharing status are receipt metadata, not stochastic key input.
    pub presence: PlannedPresence,
    pub sharing: SharingStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanCompleteness {
    CompleteWithinDeclaredSets,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisAddressPlan {
    pub hypothesis_id: StableId,
    pub completeness: PlanCompleteness,
    pub addresses: Vec<PlannedAddress>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceAddressPlan {
    pub left_branch: StableId,
    pub right_branch: StableId,
    pub completeness: PlanCompleteness,
    pub hypotheses: Vec<HypothesisAddressPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    Correspondence(CorrespondenceError),
    MissingEventScope,
    DuplicateEventScope,
    WrongBranchScope,
    MissingPairCausalRule,
    DuplicatePairCausalRule,
    OrphanPairCausalRule,
    SharedSourceMismatch,
    SharedTickMismatch,
    SharedEventKindMismatch,
}

impl From<CorrespondenceError> for ProjectionError {
    fn from(value: CorrespondenceError) -> Self {
        Self::Correspondence(value)
    }
}

/// Project a validated finite correspondence set into one address plan per
/// declared hypothesis. Unknown or incomplete causal matches are kept local.
pub fn project_correspondence_addresses(
    correspondence: &CorrespondenceSet,
    event_scopes: &[EventDrawScope],
    pair_rules: &[PairCausalRule],
) -> Result<CorrespondenceAddressPlan, ProjectionError> {
    let assessment = correspondence.assess()?;
    let global_complete =
        assessment.disposition != CorrespondenceDisposition::Incomplete;

    let scopes = index_scopes(correspondence, event_scopes)?;
    let pair_rule_index = index_pair_rules(correspondence, pair_rules)?;

    let mut hypotheses = Vec::with_capacity(correspondence.hypotheses.len());
    for hypothesis in &correspondence.hypotheses {
        let mut addresses = Vec::new();

        for pair in &hypothesis.pairs {
            let left_event = find_event(&correspondence.left_events, &pair.left_event)
                .ok_or(ProjectionError::MissingEventScope)?;
            let right_event = find_event(&correspondence.right_events, &pair.right_event)
                .ok_or(ProjectionError::MissingEventScope)?;
            let left_scope = scopes
                .get(&(correspondence.left_branch.clone(), pair.left_event.clone()))
                .ok_or(ProjectionError::MissingEventScope)?;
            let right_scope = scopes
                .get(&(correspondence.right_branch.clone(), pair.right_event.clone()))
                .ok_or(ProjectionError::MissingEventScope)?;
            let rule = pair_rule_index
                .get(&PairKey::from_pair(pair))
                .ok_or(ProjectionError::MissingPairCausalRule)?;

            let presence_unknown = matches!(&left_event.presence, EventPresence::Unknown { .. })
                || matches!(&right_event.presence, EventPresence::Unknown { .. });

            if presence_unknown {
                addresses.push(local_address(
                    left_event,
                    left_scope,
                    SharingStatus::NotIdentified,
                ));
                addresses.push(local_address(
                    right_event,
                    right_scope,
                    SharingStatus::NotIdentified,
                ));
                continue;
            }

            match &rule.disposition {
                CausalDisposition::SharedInvariant {
                    equivalence_group,
                    justification,
                } => {
                    validate_shared_profiles(left_scope, right_scope)?;
                    let bytes = shared_pair_bytes(
                        &correspondence.left_branch,
                        &correspondence.right_branch,
                        left_event,
                        right_event,
                        pair,
                        left_scope,
                        equivalence_group,
                        justification,
                    );
                    addresses.push(PlannedAddress {
                        branch_id: correspondence.left_branch.clone(),
                        event_id: pair.left_event.clone(),
                        canonical_bytes: bytes.clone(),
                        presence: presence(&left_event.presence),
                        sharing: SharingStatus::SharedInvariant,
                    });
                    addresses.push(PlannedAddress {
                        branch_id: correspondence.right_branch.clone(),
                        event_id: pair.right_event.clone(),
                        canonical_bytes: bytes,
                        presence: presence(&right_event.presence),
                        sharing: SharingStatus::SharedInvariant,
                    });
                }
                CausalDisposition::SensitivityOnly {
                    sensitivity_id,
                    justification,
                } => {
                    validate_shared_profiles(left_scope, right_scope)?;
                    let bytes = sensitivity_pair_bytes(
                        &correspondence.left_branch,
                        &correspondence.right_branch,
                        left_event,
                        right_event,
                        pair,
                        left_scope,
                        sensitivity_id,
                        justification,
                    );
                    addresses.push(PlannedAddress {
                        branch_id: correspondence.left_branch.clone(),
                        event_id: pair.left_event.clone(),
                        canonical_bytes: bytes.clone(),
                        presence: presence(&left_event.presence),
                        sharing: SharingStatus::SensitivityOnly,
                    });
                    addresses.push(PlannedAddress {
                        branch_id: correspondence.right_branch.clone(),
                        event_id: pair.right_event.clone(),
                        canonical_bytes: bytes,
                        presence: presence(&right_event.presence),
                        sharing: SharingStatus::SensitivityOnly,
                    });
                }
                CausalDisposition::BranchSpecific => {
                    addresses.push(local_address(
                        left_event,
                        left_scope,
                        SharingStatus::BranchSpecific,
                    ));
                    addresses.push(local_address(
                        right_event,
                        right_scope,
                        SharingStatus::BranchSpecific,
                    ));
                }
                CausalDisposition::NotIdentified => {
                    addresses.push(local_address(
                        left_event,
                        left_scope,
                        SharingStatus::NotIdentified,
                    ));
                    addresses.push(local_address(
                        right_event,
                        right_scope,
                        SharingStatus::NotIdentified,
                    ));
                }
                CausalDisposition::Incomplete => {
                    addresses.push(local_address(
                        left_event,
                        left_scope,
                        SharingStatus::Incomplete,
                    ));
                    addresses.push(local_address(
                        right_event,
                        right_scope,
                        SharingStatus::Incomplete,
                    ));
                }
            }
        }

        for unmatched in &hypothesis.unmatched_left {
            let event = find_event(&correspondence.left_events, &unmatched.event_id)
                .ok_or(ProjectionError::MissingEventScope)?;
            let scope = scopes
                .get(&(correspondence.left_branch.clone(), unmatched.event_id.clone()))
                .ok_or(ProjectionError::MissingEventScope)?;
            let sharing = unmatched_status(&unmatched.reason, &event.presence);
            addresses.push(local_address(event, scope, sharing));
        }

        for unmatched in &hypothesis.unmatched_right {
            let event = find_event(&correspondence.right_events, &unmatched.event_id)
                .ok_or(ProjectionError::MissingEventScope)?;
            let scope = scopes
                .get(&(correspondence.right_branch.clone(), unmatched.event_id.clone()))
                .ok_or(ProjectionError::MissingEventScope)?;
            let sharing = unmatched_status(&unmatched.reason, &event.presence);
            addresses.push(local_address(event, scope, sharing));
        }

        addresses.sort_by(|a, b| {
            (&a.branch_id, &a.event_id, &a.canonical_bytes).cmp(&(
                &b.branch_id,
                &b.event_id,
                &b.canonical_bytes,
            ))
        });
        hypotheses.push(HypothesisAddressPlan {
            hypothesis_id: hypothesis.hypothesis_id.clone(),
            completeness: if global_complete {
                PlanCompleteness::CompleteWithinDeclaredSets
            } else {
                PlanCompleteness::Incomplete
            },
            addresses,
        });
    }

    hypotheses.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
    Ok(CorrespondenceAddressPlan {
        left_branch: correspondence.left_branch.clone(),
        right_branch: correspondence.right_branch.clone(),
        completeness: if global_complete {
            PlanCompleteness::CompleteWithinDeclaredSets
        } else {
            PlanCompleteness::Incomplete
        },
        hypotheses,
    })
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PairKey {
    left: StableId,
    right: StableId,
    basis: IdentityBasis,
}

impl PairKey {
    fn from_pair(pair: &EventPair) -> Self {
        Self {
            left: pair.left_event.clone(),
            right: pair.right_event.clone(),
            basis: pair.basis,
        }
    }
}

fn index_scopes(
    correspondence: &CorrespondenceSet,
    scopes: &[EventDrawScope],
) -> Result<BTreeMap<(StableId, StableId), EventDrawScope>, ProjectionError> {
    let mut expected = BTreeSet::new();
    for event in &correspondence.left_events {
        expected.insert((correspondence.left_branch.clone(), event.event_id.clone()));
    }
    for event in &correspondence.right_events {
        expected.insert((correspondence.right_branch.clone(), event.event_id.clone()));
    }

    let mut indexed = BTreeMap::new();
    for scope in scopes {
        let key = (scope.branch_id.clone(), scope.event_id.clone());
        if !expected.contains(&key) {
            return Err(ProjectionError::WrongBranchScope);
        }
        if indexed.insert(key, scope.clone()).is_some() {
            return Err(ProjectionError::DuplicateEventScope);
        }
    }
    if indexed.len() != expected.len() {
        return Err(ProjectionError::MissingEventScope);
    }
    Ok(indexed)
}

fn index_pair_rules(
    correspondence: &CorrespondenceSet,
    rules: &[PairCausalRule],
) -> Result<BTreeMap<PairKey, PairCausalRule>, ProjectionError> {
    let mut expected = BTreeSet::new();
    for hypothesis in &correspondence.hypotheses {
        for pair in &hypothesis.pairs {
            expected.insert(PairKey::from_pair(pair));
        }
    }

    let mut indexed = BTreeMap::new();
    for rule in rules {
        let key = PairKey {
            left: rule.left_event.clone(),
            right: rule.right_event.clone(),
            basis: rule.basis,
        };
        if !expected.contains(&key) {
            return Err(ProjectionError::OrphanPairCausalRule);
        }
        if indexed.insert(key, rule.clone()).is_some() {
            return Err(ProjectionError::DuplicatePairCausalRule);
        }
    }
    Ok(indexed)
}

fn find_event<'a>(
    events: &'a [EventOccurrence],
    event_id: &StableId,
) -> Option<&'a EventOccurrence> {
    events.iter().find(|event| &event.event_id == event_id)
}

fn validate_shared_profiles(
    left: &EventDrawScope,
    right: &EventDrawScope,
) -> Result<(), ProjectionError> {
    if left.source != right.source {
        return Err(ProjectionError::SharedSourceMismatch);
    }
    if left.tick != right.tick {
        return Err(ProjectionError::SharedTickMismatch);
    }
    if left.event_kind != right.event_kind {
        return Err(ProjectionError::SharedEventKindMismatch);
    }
    Ok(())
}

fn unmatched_status(reason: &UnmatchedReason, presence: &EventPresence) -> SharingStatus {
    if matches!(presence, EventPresence::Unknown { .. }) {
        return SharingStatus::NotIdentified;
    }
    match reason {
        UnmatchedReason::NoCounterpartUnderHypothesis { .. } => {
            SharingStatus::NoCounterpartUnderHypothesis
        }
        UnmatchedReason::CorrespondenceUnknown { .. } => SharingStatus::NotIdentified,
    }
}

fn presence(value: &EventPresence) -> PlannedPresence {
    match value {
        EventPresence::Observed { evidence_ref } => PlannedPresence::Observed {
            evidence_ref: evidence_ref.clone(),
        },
        EventPresence::NotRealized { condition_witness } => PlannedPresence::NotRealized {
            condition_witness: condition_witness.clone(),
        },
        EventPresence::Unknown { evidence_ref } => PlannedPresence::Unknown {
            evidence_ref: evidence_ref.clone(),
        },
    }
}

fn local_address(
    event: &EventOccurrence,
    scope: &EventDrawScope,
    sharing: SharingStatus,
) -> PlannedAddress {
    PlannedAddress {
        branch_id: scope.branch_id.clone(),
        event_id: scope.event_id.clone(),
        canonical_bytes: local_bytes(event, scope),
        presence: presence(&event.presence),
        sharing,
    }
}

fn local_bytes(event: &EventOccurrence, scope: &EventDrawScope) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(DOMAIN);
    out.push(1); // branch-local event address
    push_id(&mut out, &scope.branch_id);
    push_id(&mut out, &scope.event_id);
    push_id(&mut out, &event.event_family);
    out.extend_from_slice(&scope.tick.to_le_bytes());
    push_id(&mut out, &scope.event_kind);
    push_profile(&mut out, &scope.source);
    out
}

fn shared_pair_bytes(
    left_branch: &StableId,
    right_branch: &StableId,
    left_event: &EventOccurrence,
    right_event: &EventOccurrence,
    pair: &EventPair,
    scope: &EventDrawScope,
    equivalence_group: &StableId,
    justification: &StableId,
) -> Vec<u8> {
    pair_bytes(
        left_branch,
        right_branch,
        left_event,
        right_event,
        pair,
        scope,
        2,
        equivalence_group,
        justification,
    )
}

fn sensitivity_pair_bytes(
    left_branch: &StableId,
    right_branch: &StableId,
    left_event: &EventOccurrence,
    right_event: &EventOccurrence,
    pair: &EventPair,
    scope: &EventDrawScope,
    sensitivity_id: &StableId,
    justification: &StableId,
) -> Vec<u8> {
    pair_bytes(
        left_branch,
        right_branch,
        left_event,
        right_event,
        pair,
        scope,
        3,
        sensitivity_id,
        justification,
    )
}

fn pair_bytes(
    left_branch: &StableId,
    right_branch: &StableId,
    left_event: &EventOccurrence,
    right_event: &EventOccurrence,
    pair: &EventPair,
    scope: &EventDrawScope,
    coupling_tag: u8,
    group_or_sensitivity: &StableId,
    justification: &StableId,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(DOMAIN);
    out.push(2); // hypothesis-conditioned cross-world event pair
    // Canonicalize endpoint order so reversing branch presentation does not
    // change the pair identity or the resolved raw draw.
    let left_endpoint = (left_branch, &left_event.event_id);
    let right_endpoint = (right_branch, &right_event.event_id);
    let (first, second) = if left_endpoint <= right_endpoint {
        (left_endpoint, right_endpoint)
    } else {
        (right_endpoint, left_endpoint)
    };
    push_id(&mut out, first.0);
    push_id(&mut out, first.1);
    push_id(&mut out, second.0);
    push_id(&mut out, second.1);
    out.push(match pair.basis {
        IdentityBasis::Actor => 1,
        IdentityBasis::DirectedDyad => 2,
        IdentityBasis::EncounterSlot => 3,
    });
    push_id(&mut out, &left_event.event_family);
    out.extend_from_slice(&scope.tick.to_le_bytes());
    push_id(&mut out, &scope.event_kind);
    push_profile(&mut out, &scope.source);
    out.push(coupling_tag);
    push_id(&mut out, group_or_sensitivity);
    push_id(&mut out, justification);
    out
}

fn push_profile(out: &mut Vec<u8>, profile: &SourceProfile) {
    push_id(out, &profile.source_id);
    push_id(out, &profile.mechanism_id);
    push_id(out, &profile.mechanism_version);
    push_id(out, &profile.draw_component);
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let len = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondence::{
        CandidateUniverseStatus, CorrespondenceHypothesis, CorrespondenceSet, EventOccurrence,
        EventPair, EventPresence, HypothesisSetStatus, UnmatchedEvent, UnmatchedReason,
    };

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("valid fixture id")
    }

    fn event(
        event_id: &str,
        actor: &str,
        slot: &str,
        presence: EventPresence,
    ) -> EventOccurrence {
        EventOccurrence {
            event_id: id(event_id),
            event_family: id("crossing-gap-choice"),
            actor: Some(id(actor)),
            initiator: Some(id("ego-1")),
            recipient: Some(id(actor)),
            encounter_slot: Some(id(slot)),
            presence,
        }
    }

    fn scope(branch: &str, event: &str) -> EventDrawScope {
        EventDrawScope {
            branch_id: id(branch),
            event_id: id(event),
            tick: 40,
            event_kind: id("pedestrian-response-noise"),
            source: SourceProfile {
                source_id: id("policy-noise-v1"),
                mechanism_id: id("accept-gap-v1"),
                mechanism_version: id("mechanism-sha"),
                draw_component: id("normal-0"),
            },
        }
    }

    fn pair(left: &str, right: &str, basis: IdentityBasis) -> EventPair {
        EventPair {
            left_event: id(left),
            right_event: id(right),
            basis,
            justification_ref: id("pairing-justification"),
        }
    }

    fn hypothesis(
        name: &str,
        pairs: Vec<EventPair>,
        unmatched_right: Vec<UnmatchedEvent>,
    ) -> CorrespondenceHypothesis {
        CorrespondenceHypothesis {
            hypothesis_id: id(name),
            justification_ref: id("hypothesis-justification"),
            pairs,
            unmatched_left: Vec::new(),
            unmatched_right,
        }
    }

    fn base_correspondence() -> CorrespondenceSet {
        let left = event(
            "L1",
            "ped-A",
            "slot-1",
            EventPresence::Observed {
                evidence_ref: id("left-observation"),
            },
        );
        let right_actor = event(
            "R1",
            "ped-A",
            "slot-2",
            EventPresence::Observed {
                evidence_ref: id("right-actor-observation"),
            },
        );
        let right_slot = event(
            "R2",
            "ped-B",
            "slot-1",
            EventPresence::Observed {
                evidence_ref: id("right-slot-observation"),
            },
        );
        CorrespondenceSet {
            left_branch: id("continue"),
            right_branch: id("yield"),
            left_events: vec![left],
            right_events: vec![right_actor, right_slot],
            hypotheses: vec![
                hypothesis(
                    "H-actor",
                    vec![pair("L1", "R1", IdentityBasis::Actor)],
                    vec![UnmatchedEvent {
                        event_id: id("R2"),
                        reason: UnmatchedReason::NoCounterpartUnderHypothesis {
                            rationale_ref: id("no-actor-counterpart"),
                        },
                    }],
                ),
                hypothesis(
                    "H-slot",
                    vec![pair("L1", "R2", IdentityBasis::EncounterSlot)],
                    vec![UnmatchedEvent {
                        event_id: id("R1"),
                        reason: UnmatchedReason::NoCounterpartUnderHypothesis {
                            rationale_ref: id("no-slot-counterpart"),
                        },
                    }],
                ),
            ],
            universe_status: CandidateUniverseStatus::ExhaustiveWithinDeclaredScope {
                evidence_ref: id("event-universe"),
            },
            hypothesis_set_status:
                HypothesisSetStatus::ExhaustiveWithinDeclaredModelFamily {
                    evidence_ref: id("hypothesis-search"),
                },
        }
    }

    fn all_scopes() -> Vec<EventDrawScope> {
        vec![scope("continue", "L1"), scope("yield", "R1"), scope("yield", "R2")]
    }

    fn shared_rule(left: &str, right: &str, basis: IdentityBasis) -> PairCausalRule {
        PairCausalRule {
            left_event: id(left),
            right_event: id(right),
            basis,
            disposition: CausalDisposition::SharedInvariant {
                equivalence_group: id("weather-gust-group"),
                justification: id("scm-weather-invariant"),
            },
        }
    }

    #[test]
    fn actor_and_slot_hypotheses_produce_distinct_retained_address_plans() {
        let correspondence = base_correspondence();
        let rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        let plan =
            project_correspondence_addresses(&correspondence, &all_scopes(), &rules).unwrap();

        assert_eq!(plan.completeness, PlanCompleteness::CompleteWithinDeclaredSets);
        assert_eq!(plan.hypotheses.len(), 2);
        let actor = &plan.hypotheses[0];
        let slot = &plan.hypotheses[1];
        let actor_left = actor
            .addresses
            .iter()
            .find(|address| address.branch_id == id("continue"))
            .unwrap();
        let slot_left = slot
            .addresses
            .iter()
            .find(|address| address.branch_id == id("continue"))
            .unwrap();
        assert_ne!(actor_left.canonical_bytes, slot_left.canonical_bytes);
        assert_eq!(actor_left.sharing, SharingStatus::SharedInvariant);
        assert_eq!(slot_left.sharing, SharingStatus::SharedInvariant);
    }

    #[test]
    fn shared_invariant_pair_has_identical_bytes_but_branch_specific_pair_does_not() {
        let correspondence = base_correspondence();
        let shared = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        let plan = project_correspondence_addresses(&correspondence, &all_scopes(), &shared).unwrap();
        for hypothesis in &plan.hypotheses {
            let left = hypothesis.addresses.iter().find(|a| a.branch_id == id("continue")).unwrap();
            let right = hypothesis.addresses.iter().find(|a| a.branch_id == id("yield") && a.sharing == SharingStatus::SharedInvariant).unwrap();
            assert_eq!(left.canonical_bytes, right.canonical_bytes);
        }

        let mut branch_specific = shared;
        for rule in &mut branch_specific {
            rule.disposition = CausalDisposition::BranchSpecific;
        }
        let local_plan = project_correspondence_addresses(&correspondence, &all_scopes(), &branch_specific).unwrap();
        let hypothesis = &local_plan.hypotheses[0];
        let left = hypothesis.addresses.iter().find(|a| a.branch_id == id("continue")).unwrap();
        let right = hypothesis.addresses.iter().find(|a| a.branch_id == id("yield") && a.event_id == id("R1")).unwrap();
        assert_ne!(left.canonical_bytes, right.canonical_bytes);
    }

    #[test]
    fn branch_local_address_does_not_depend_on_which_counterpart_hypothesis_was_selected() {
        let correspondence = base_correspondence();
        let mut rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        for rule in &mut rules {
            rule.disposition = CausalDisposition::BranchSpecific;
        }
        let plan = project_correspondence_addresses(&correspondence, &all_scopes(), &rules).unwrap();
        let left_actor = plan.hypotheses[0].addresses.iter().find(|a| a.branch_id == id("continue")).unwrap();
        let left_slot = plan.hypotheses[1].addresses.iter().find(|a| a.branch_id == id("continue")).unwrap();
        assert_eq!(left_actor.canonical_bytes, left_slot.canonical_bytes);
    }

    #[test]
    fn unknown_presence_never_receives_a_shared_address() {
        let mut correspondence = base_correspondence();
        correspondence.right_events[0].presence = EventPresence::Unknown {
            evidence_ref: id("tracker-gap"),
        };
        let rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        let plan = project_correspondence_addresses(&correspondence, &all_scopes(), &rules).unwrap();
        assert_eq!(plan.completeness, PlanCompleteness::Incomplete);
        let actor = &plan.hypotheses[0];
        assert!(actor.addresses.iter().all(|a| a.sharing != SharingStatus::SharedInvariant));
        assert!(actor.addresses.iter().any(|a| a.sharing == SharingStatus::NotIdentified));
    }

    #[test]
    fn conditional_nonrealization_keeps_the_shared_potential_event_address() {
        let mut correspondence = base_correspondence();
        correspondence.right_events[0].presence = EventPresence::NotRealized {
            condition_witness: id("not-visible-in-yield-branch"),
        };
        let rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        let plan = project_correspondence_addresses(&correspondence, &all_scopes(), &rules).unwrap();
        let actor = &plan.hypotheses[0];
        let left = actor.addresses.iter().find(|a| a.branch_id == id("continue")).unwrap();
        let right = actor.addresses.iter().find(|a| a.branch_id == id("yield") && a.event_id == id("R1")).unwrap();
        assert_eq!(left.canonical_bytes, right.canonical_bytes);
        assert!(matches!(right.presence, PlannedPresence::NotRealized { .. }));
    }

    #[test]
    fn incomplete_hypothesis_search_retains_known_plans_but_marks_them_incomplete() {
        let mut correspondence = base_correspondence();
        correspondence.hypothesis_set_status =
            HypothesisSetStatus::IncompleteOrBudgetLimited {
                evidence_ref: id("budget-limited-search"),
            };
        let rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        let plan = project_correspondence_addresses(&correspondence, &all_scopes(), &rules).unwrap();
        assert_eq!(plan.completeness, PlanCompleteness::Incomplete);
        assert_eq!(plan.hypotheses.len(), 2);
        assert!(plan.hypotheses.iter().all(|h| h.completeness == PlanCompleteness::Incomplete));
    }

    #[test]
    fn shared_pair_rejects_source_or_tick_mismatch() {
        let correspondence = base_correspondence();
        let mut scopes = all_scopes();
        scopes[1].source.mechanism_version = id("different-version");
        let rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        assert_eq!(
            project_correspondence_addresses(&correspondence, &scopes, &rules),
            Err(ProjectionError::SharedSourceMismatch)
        );

        let mut scopes = all_scopes();
        scopes[1].tick = 41;
        assert_eq!(
            project_correspondence_addresses(&correspondence, &scopes, &rules),
            Err(ProjectionError::SharedTickMismatch)
        );
    }

    #[test]
    fn sensitivity_pair_is_shared_only_with_explicit_sensitivity_receipt() {
        let correspondence = base_correspondence();
        let mut rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        for rule in &mut rules {
            rule.disposition = CausalDisposition::SensitivityOnly {
                sensitivity_id: id("sensitivity-map"),
                justification: id("declared-alternative-coupling"),
            };
        }
        let plan = project_correspondence_addresses(&correspondence, &all_scopes(), &rules).unwrap();
        for hypothesis in &plan.hypotheses {
            let left = hypothesis.addresses.iter().find(|a| a.branch_id == id("continue")).unwrap();
            let right = hypothesis.addresses.iter().find(|a| a.branch_id == id("yield") && a.sharing == SharingStatus::SensitivityOnly).unwrap();
            assert_eq!(left.canonical_bytes, right.canonical_bytes);
            assert_eq!(left.sharing, SharingStatus::SensitivityOnly);
        }
    }

    #[test]
    fn duplicate_pair_rules_reject() {
        let correspondence = base_correspondence();
        let rules = vec![
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R1", IdentityBasis::Actor),
            shared_rule("L1", "R2", IdentityBasis::EncounterSlot),
        ];
        assert_eq!(
            project_correspondence_addresses(&correspondence, &all_scopes(), &rules),
            Err(ProjectionError::DuplicatePairCausalRule)
        );
    }
}

//! Explicit finite sets of event correspondences between counterfactual branches.
//!
//! This module does not infer event identity. It validates caller-supplied
//! hypotheses, preserves ambiguity, and separates missing evidence from
//! conditionally unrealized events.

use std::collections::{BTreeMap, BTreeSet};

use super::StableId;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EventPresence {
    Observed { evidence_ref: StableId },
    NotRealized { condition_witness: StableId },
    Unknown { evidence_ref: StableId },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EventOccurrence {
    pub event_id: StableId,
    pub event_family: StableId,
    pub actor: Option<StableId>,
    pub initiator: Option<StableId>,
    pub recipient: Option<StableId>,
    pub encounter_slot: Option<StableId>,
    pub presence: EventPresence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum IdentityBasis {
    Actor,
    DirectedDyad,
    EncounterSlot,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EventPair {
    pub left_event: StableId,
    pub right_event: StableId,
    pub basis: IdentityBasis,
    pub justification_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum UnmatchedReason {
    /// This map does not assert a counterpart for this event.
    NoCounterpartUnderHypothesis { rationale_ref: StableId },
    /// The available evidence cannot establish whether a counterpart exists.
    CorrespondenceUnknown { evidence_ref: StableId },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct UnmatchedEvent {
    pub event_id: StableId,
    pub reason: UnmatchedReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceHypothesis {
    pub hypothesis_id: StableId,
    pub justification_ref: StableId,
    pub pairs: Vec<EventPair>,
    pub unmatched_left: Vec<UnmatchedEvent>,
    pub unmatched_right: Vec<UnmatchedEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateUniverseStatus {
    ExhaustiveWithinDeclaredScope { evidence_ref: StableId },
    Incomplete { evidence_ref: StableId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HypothesisSetStatus {
    /// The declared matching family is asserted exhaustive within a named model family.
    ExhaustiveWithinDeclaredModelFamily { evidence_ref: StableId },
    /// Matching search was bounded, interrupted, or otherwise not exhaustive.
    IncompleteOrBudgetLimited { evidence_ref: StableId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceSet {
    pub left_branch: StableId,
    pub right_branch: StableId,
    pub left_events: Vec<EventOccurrence>,
    pub right_events: Vec<EventOccurrence>,
    pub hypotheses: Vec<CorrespondenceHypothesis>,
    pub universe_status: CandidateUniverseStatus,
    pub hypothesis_set_status: HypothesisSetStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorrespondenceError {
    SameBranchOnBothSides,
    EmptyHypothesisSet,
    DuplicateEventId,
    DuplicateHypothesisId,
    UnknownLeftEndpoint,
    UnknownRightEndpoint,
    DuplicateLeftEndpoint,
    DuplicateRightEndpoint,
    UnclassifiedLeftEvent,
    UnclassifiedRightEvent,
    EventFamilyMismatch,
    MissingActorIdentity,
    ActorIdentityMismatch,
    MissingDirectedDyadIdentity,
    DirectedDyadMismatch,
    MissingEncounterSlot,
    EncounterSlotMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrespondenceDisposition {
    UniqueWithinDeclaredSet,
    MultipleAdmissibleMaps,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisSummary {
    pub hypothesis_id: StableId,
    pub matched_pairs: usize,
    pub unmatched_left: usize,
    pub unmatched_right: usize,
    pub conditional_presence_differences: usize,
    pub unknown_presence_or_correspondence: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceAssessment {
    pub disposition: CorrespondenceDisposition,
    pub hypothesis_count: usize,
    pub summaries: Vec<HypothesisSummary>,
}

impl CorrespondenceSet {
    /// Validate all declared maps and classify what the declared evidence permits.
    ///
    /// ExhaustiveWithinDeclaredScope is an evidence-backed caller assertion; this
    /// function checks internal coverage of that declared universe, not whether
    /// the caller has discovered every scientifically possible event.
    pub fn assess(&self) -> Result<CorrespondenceAssessment, CorrespondenceError> {
        if self.left_branch == self.right_branch {
            return Err(CorrespondenceError::SameBranchOnBothSides);
        }
        if self.hypotheses.is_empty() {
            return Err(CorrespondenceError::EmptyHypothesisSet);
        }

        let left = event_index(&self.left_events)?;
        let right = event_index(&self.right_events)?;
        let mut hypothesis_ids = BTreeSet::new();
        let mut summaries = Vec::with_capacity(self.hypotheses.len());
        let mut any_unknown = matches!(
            &self.universe_status,
            CandidateUniverseStatus::Incomplete { .. }
        ) || matches!(
            &self.hypothesis_set_status,
            HypothesisSetStatus::IncompleteOrBudgetLimited { .. }
        );

        for hypothesis in &self.hypotheses {
            if !hypothesis_ids.insert(hypothesis.hypothesis_id.clone()) {
                return Err(CorrespondenceError::DuplicateHypothesisId);
            }

            let mut covered_left = BTreeSet::new();
            let mut covered_right = BTreeSet::new();
            let mut conditional_presence_differences = 0;
            let mut unknown_presence_or_correspondence = false;

            for pair in &hypothesis.pairs {
                let left_event = left
                    .get(&pair.left_event)
                    .ok_or(CorrespondenceError::UnknownLeftEndpoint)?;
                let right_event = right
                    .get(&pair.right_event)
                    .ok_or(CorrespondenceError::UnknownRightEndpoint)?;

                if !covered_left.insert(pair.left_event.clone()) {
                    return Err(CorrespondenceError::DuplicateLeftEndpoint);
                }
                if !covered_right.insert(pair.right_event.clone()) {
                    return Err(CorrespondenceError::DuplicateRightEndpoint);
                }

                validate_pair(left_event, right_event, pair.basis)?;

                match (&left_event.presence, &right_event.presence) {
                    (EventPresence::Unknown { .. }, _) | (_, EventPresence::Unknown { .. }) => {
                        unknown_presence_or_correspondence = true;
                    }
                    (EventPresence::Observed { .. }, EventPresence::NotRealized { .. })
                    | (EventPresence::NotRealized { .. }, EventPresence::Observed { .. }) => {
                        conditional_presence_differences += 1;
                    }
                    _ => {}
                }
            }

            for unmatched in &hypothesis.unmatched_left {
                if !left.contains_key(&unmatched.event_id) {
                    return Err(CorrespondenceError::UnknownLeftEndpoint);
                }
                if !covered_left.insert(unmatched.event_id.clone()) {
                    return Err(CorrespondenceError::DuplicateLeftEndpoint);
                }
                if matches!(
                    &unmatched.reason,
                    UnmatchedReason::CorrespondenceUnknown { .. }
                ) {
                    unknown_presence_or_correspondence = true;
                }
                if matches!(
                    &left[&unmatched.event_id].presence,
                    EventPresence::Unknown { .. }
                ) {
                    unknown_presence_or_correspondence = true;
                }
            }

            for unmatched in &hypothesis.unmatched_right {
                if !right.contains_key(&unmatched.event_id) {
                    return Err(CorrespondenceError::UnknownRightEndpoint);
                }
                if !covered_right.insert(unmatched.event_id.clone()) {
                    return Err(CorrespondenceError::DuplicateRightEndpoint);
                }
                if matches!(
                    unmatched.reason,
                    UnmatchedReason::CorrespondenceUnknown { .. }
                ) {
                    unknown_presence_or_correspondence = true;
                }
                if matches!(
                    &right[&unmatched.event_id].presence,
                    EventPresence::Unknown { .. }
                ) {
                    unknown_presence_or_correspondence = true;
                }
            }

            if covered_left.len() != left.len() {
                return Err(CorrespondenceError::UnclassifiedLeftEvent);
            }
            if covered_right.len() != right.len() {
                return Err(CorrespondenceError::UnclassifiedRightEvent);
            }

            any_unknown |= unknown_presence_or_correspondence;
            summaries.push(HypothesisSummary {
                hypothesis_id: hypothesis.hypothesis_id.clone(),
                matched_pairs: hypothesis.pairs.len(),
                unmatched_left: hypothesis.unmatched_left.len(),
                unmatched_right: hypothesis.unmatched_right.len(),
                conditional_presence_differences,
                unknown_presence_or_correspondence,
            });
        }

        summaries.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        let disposition = if any_unknown {
            CorrespondenceDisposition::Incomplete
        } else if self.hypotheses.len() == 1 {
            CorrespondenceDisposition::UniqueWithinDeclaredSet
        } else {
            CorrespondenceDisposition::MultipleAdmissibleMaps
        };

        Ok(CorrespondenceAssessment {
            disposition,
            hypothesis_count: self.hypotheses.len(),
            summaries,
        })
    }

    /// Canonical, order-independent serialization after structural validation.
    ///
    /// These bytes are not hashed or authenticated here. A downstream receipt
    /// may digest them using its separately pinned commitment implementation.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CorrespondenceError> {
        self.assess()?;

        let mut out = Vec::new();
        out.extend_from_slice(b"symtropy.road-event-correspondence.v0.1\0");
        push_id(&mut out, &self.left_branch);
        push_id(&mut out, &self.right_branch);

        match &self.universe_status {
            CandidateUniverseStatus::ExhaustiveWithinDeclaredScope { evidence_ref } => {
                out.push(1);
                push_id(&mut out, evidence_ref);
            }
            CandidateUniverseStatus::Incomplete { evidence_ref } => {
                out.push(2);
                push_id(&mut out, evidence_ref);
            }
        }
        match &self.hypothesis_set_status {
            HypothesisSetStatus::ExhaustiveWithinDeclaredModelFamily { evidence_ref } => {
                out.push(1);
                push_id(&mut out, evidence_ref);
            }
            HypothesisSetStatus::IncompleteOrBudgetLimited { evidence_ref } => {
                out.push(2);
                push_id(&mut out, evidence_ref);
            }
        }

        let mut left_events = self.left_events.iter().collect::<Vec<_>>();
        left_events.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        push_len(&mut out, left_events.len());
        for event in left_events {
            push_event(&mut out, event);
        }

        let mut right_events = self.right_events.iter().collect::<Vec<_>>();
        right_events.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        push_len(&mut out, right_events.len());
        for event in right_events {
            push_event(&mut out, event);
        }

        let mut hypotheses = self.hypotheses.iter().collect::<Vec<_>>();
        hypotheses.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        push_len(&mut out, hypotheses.len());
        for hypothesis in hypotheses {
            push_id(&mut out, &hypothesis.hypothesis_id);
            push_id(&mut out, &hypothesis.justification_ref);

            let mut pairs = hypothesis.pairs.iter().collect::<Vec<_>>();
            pairs.sort_by(|a, b| {
                (&a.left_event, &a.right_event, a.basis).cmp(&(
                    &b.left_event,
                    &b.right_event,
                    b.basis,
                ))
            });
            push_len(&mut out, pairs.len());
            for pair in pairs {
                push_id(&mut out, &pair.left_event);
                push_id(&mut out, &pair.right_event);
                out.push(match pair.basis {
                    IdentityBasis::Actor => 1,
                    IdentityBasis::DirectedDyad => 2,
                    IdentityBasis::EncounterSlot => 3,
                });
                push_id(&mut out, &pair.justification_ref);
            }

            push_unmatched(&mut out, &hypothesis.unmatched_left);
            push_unmatched(&mut out, &hypothesis.unmatched_right);
        }
        Ok(out)
    }
}

fn event_index(
    events: &[EventOccurrence],
) -> Result<BTreeMap<StableId, &EventOccurrence>, CorrespondenceError> {
    let mut index = BTreeMap::new();
    for event in events {
        if index.insert(event.event_id.clone(), event).is_some() {
            return Err(CorrespondenceError::DuplicateEventId);
        }
    }
    Ok(index)
}

fn validate_pair(
    left: &EventOccurrence,
    right: &EventOccurrence,
    basis: IdentityBasis,
) -> Result<(), CorrespondenceError> {
    if left.event_family != right.event_family {
        return Err(CorrespondenceError::EventFamilyMismatch);
    }
    match basis {
        IdentityBasis::Actor => match (&left.actor, &right.actor) {
            (Some(a), Some(b)) if a == b => Ok(()),
            (Some(_), Some(_)) => Err(CorrespondenceError::ActorIdentityMismatch),
            _ => Err(CorrespondenceError::MissingActorIdentity),
        },
        IdentityBasis::DirectedDyad => {
            match (
                &left.initiator,
                &left.recipient,
                &right.initiator,
                &right.recipient,
            ) {
                (Some(li), Some(lr), Some(ri), Some(rr)) if li == ri && lr == rr => Ok(()),
                (Some(_), Some(_), Some(_), Some(_)) => {
                    Err(CorrespondenceError::DirectedDyadMismatch)
                }
                _ => Err(CorrespondenceError::MissingDirectedDyadIdentity),
            }
        }
        IdentityBasis::EncounterSlot => match (&left.encounter_slot, &right.encounter_slot) {
            (Some(a), Some(b)) if a == b => Ok(()),
            (Some(_), Some(_)) => Err(CorrespondenceError::EncounterSlotMismatch),
            _ => Err(CorrespondenceError::MissingEncounterSlot),
        },
    }
}

fn push_len(out: &mut Vec<u8>, value: usize) {
    let value = u32::try_from(value).expect("fixture collection length must fit u32");
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_option_id(out: &mut Vec<u8>, id: &Option<StableId>) {
    match id {
        Some(value) => {
            out.push(1);
            push_id(out, value);
        }
        None => out.push(0),
    }
}

fn push_event(out: &mut Vec<u8>, event: &EventOccurrence) {
    push_id(out, &event.event_id);
    push_id(out, &event.event_family);
    push_option_id(out, &event.actor);
    push_option_id(out, &event.initiator);
    push_option_id(out, &event.recipient);
    push_option_id(out, &event.encounter_slot);
    match &event.presence {
        EventPresence::Observed { evidence_ref } => {
            out.push(1);
            push_id(out, evidence_ref);
        }
        EventPresence::NotRealized { condition_witness } => {
            out.push(2);
            push_id(out, condition_witness);
        }
        EventPresence::Unknown { evidence_ref } => {
            out.push(3);
            push_id(out, evidence_ref);
        }
    }
}

fn push_unmatched(out: &mut Vec<u8>, unmatched: &[UnmatchedEvent]) {
    let mut unmatched = unmatched.iter().collect::<Vec<_>>();
    unmatched.sort_by(|a, b| a.event_id.cmp(&b.event_id));
    push_len(out, unmatched.len());
    for event in unmatched {
        push_id(out, &event.event_id);
        match &event.reason {
            UnmatchedReason::NoCounterpartUnderHypothesis { rationale_ref } => {
                out.push(1);
                push_id(out, rationale_ref);
            }
            UnmatchedReason::CorrespondenceUnknown { evidence_ref } => {
                out.push(2);
                push_id(out, evidence_ref);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID must be canonical")
    }

    fn observed(id_value: &str, actor: &str, slot: &str) -> EventOccurrence {
        EventOccurrence {
            event_id: id(id_value),
            event_family: id("pedestrian-gap-choice"),
            actor: Some(id(actor)),
            initiator: Some(id("ego-1")),
            recipient: Some(id(actor)),
            encounter_slot: Some(id(slot)),
            presence: EventPresence::Observed {
                evidence_ref: id("receipt:observed"),
            },
        }
    }

    fn pair(left: &str, right: &str, basis: IdentityBasis) -> EventPair {
        EventPair {
            left_event: id(left),
            right_event: id(right),
            basis,
            justification_ref: id("assumption:pairing-v1"),
        }
    }

    fn one_map(
        left_events: Vec<EventOccurrence>,
        right_events: Vec<EventOccurrence>,
        pairs: Vec<EventPair>,
    ) -> CorrespondenceSet {
        CorrespondenceSet {
            left_branch: id("continue"),
            right_branch: id("yield"),
            left_events,
            right_events,
            hypotheses: vec![CorrespondenceHypothesis {
                hypothesis_id: id("map-0"),
                justification_ref: id("assumption:map-0"),
                pairs,
                unmatched_left: vec![],
                unmatched_right: vec![],
            }],
            universe_status: CandidateUniverseStatus::ExhaustiveWithinDeclaredScope {
                evidence_ref: id("universe:frozen"),
            },
            hypothesis_set_status: HypothesisSetStatus::ExhaustiveWithinDeclaredModelFamily {
                evidence_ref: id("hypotheses:declared-family"),
            },
        }
    }

    #[test]
    fn unique_actor_correspondence_is_qualified_within_declared_set() {
        let set = one_map(
            vec![observed("l-a", "ped-a", "slot-1")],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        let assessment = set.assess().unwrap();
        assert_eq!(
            assessment.disposition,
            CorrespondenceDisposition::UniqueWithinDeclaredSet
        );
        assert_eq!(assessment.summaries[0].matched_pairs, 1);
    }

    #[test]
    fn actor_and_slot_maps_are_preserved_as_distinct_hypotheses() {
        let left = vec![
            observed("l-a", "ped-a", "slot-1"),
            observed("l-b", "ped-b", "slot-1"),
        ];
        let right = vec![
            observed("r-a", "ped-a", "slot-1"),
            observed("r-b", "ped-b", "slot-1"),
        ];
        let actor_map = CorrespondenceHypothesis {
            hypothesis_id: id("actor-map"),
            justification_ref: id("assumption:actor"),
            pairs: vec![
                pair("l-a", "r-a", IdentityBasis::Actor),
                pair("l-b", "r-b", IdentityBasis::Actor),
            ],
            unmatched_left: vec![],
            unmatched_right: vec![],
        };
        let slot_map = CorrespondenceHypothesis {
            hypothesis_id: id("slot-map"),
            justification_ref: id("assumption:slot-exchangeability"),
            pairs: vec![
                pair("l-a", "r-b", IdentityBasis::EncounterSlot),
                pair("l-b", "r-a", IdentityBasis::EncounterSlot),
            ],
            unmatched_left: vec![],
            unmatched_right: vec![],
        };
        let set = CorrespondenceSet {
            left_branch: id("continue"),
            right_branch: id("yield"),
            left_events: left,
            right_events: right,
            hypotheses: vec![actor_map, slot_map],
            universe_status: CandidateUniverseStatus::ExhaustiveWithinDeclaredScope {
                evidence_ref: id("universe:symmetric-crossing"),
            },
            hypothesis_set_status: HypothesisSetStatus::ExhaustiveWithinDeclaredModelFamily {
                evidence_ref: id("hypotheses:actor-vs-slot"),
            },
        };
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::MultipleAdmissibleMaps
        );
    }

    #[test]
    fn encounter_slot_can_map_different_actors_without_claiming_actor_identity() {
        let set = one_map(
            vec![observed("l-a", "ped-a", "crosswalk-slot-7")],
            vec![observed("r-b", "ped-b", "crosswalk-slot-7")],
            vec![pair("l-a", "r-b", IdentityBasis::EncounterSlot)],
        );
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::UniqueWithinDeclaredSet
        );
    }

    #[test]
    fn actor_basis_rejects_different_actor_identity() {
        let set = one_map(
            vec![observed("l-a", "ped-a", "slot-1")],
            vec![observed("r-b", "ped-b", "slot-1")],
            vec![pair("l-a", "r-b", IdentityBasis::Actor)],
        );
        assert_eq!(
            set.assess(),
            Err(CorrespondenceError::ActorIdentityMismatch)
        );
    }

    #[test]
    fn directed_dyad_basis_preserves_initiator_and_recipient_roles() {
        let mut left = observed("l-a", "ped-a", "slot-1");
        let mut right = observed("r-a", "ped-a", "slot-1");
        left.initiator = Some(id("ego-1"));
        left.recipient = Some(id("ped-a"));
        right.initiator = Some(id("ped-a"));
        right.recipient = Some(id("ego-1"));
        let set = one_map(
            vec![left],
            vec![right],
            vec![pair("l-a", "r-a", IdentityBasis::DirectedDyad)],
        );
        assert_eq!(set.assess(), Err(CorrespondenceError::DirectedDyadMismatch));
    }

    #[test]
    fn paired_observed_and_not_realized_is_conditional_presence_not_missing_data() {
        let left = observed("l-a", "ped-a", "slot-1");
        let mut right = observed("r-a", "ped-a", "slot-1");
        right.presence = EventPresence::NotRealized {
            condition_witness: id("condition:ego-never-entered-view"),
        };
        let set = one_map(
            vec![left],
            vec![right],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        let assessment = set.assess().unwrap();
        assert_eq!(
            assessment.disposition,
            CorrespondenceDisposition::UniqueWithinDeclaredSet
        );
        assert_eq!(assessment.summaries[0].conditional_presence_differences, 1);
    }

    #[test]
    fn unknown_presence_makes_the_correspondence_incomplete() {
        let left = observed("l-a", "ped-a", "slot-1");
        let mut right = observed("r-a", "ped-a", "slot-1");
        right.presence = EventPresence::Unknown {
            evidence_ref: id("evidence:tracker-dropout"),
        };
        let set = one_map(
            vec![left],
            vec![right],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::Incomplete
        );
    }

    #[test]
    fn correspondence_unknown_is_not_conditionally_absent() {
        let mut set = one_map(vec![observed("l-a", "ped-a", "slot-1")], vec![], vec![]);
        set.hypotheses[0].unmatched_left.push(UnmatchedEvent {
            event_id: id("l-a"),
            reason: UnmatchedReason::CorrespondenceUnknown {
                evidence_ref: id("evidence:occluded"),
            },
        });
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::Incomplete
        );
    }

    #[test]
    fn explicitly_unmatched_event_can_be_complete_under_one_hypothesis() {
        let mut set = one_map(vec![observed("l-a", "ped-a", "slot-1")], vec![], vec![]);
        set.hypotheses[0].unmatched_left.push(UnmatchedEvent {
            event_id: id("l-a"),
            reason: UnmatchedReason::NoCounterpartUnderHypothesis {
                rationale_ref: id("assumption:no-shared-event"),
            },
        });
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::UniqueWithinDeclaredSet
        );
    }

    #[test]
    fn omitted_event_is_rejected_instead_of_silently_ignored() {
        let set = one_map(
            vec![
                observed("l-a", "ped-a", "slot-1"),
                observed("l-b", "ped-b", "slot-2"),
            ],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        assert_eq!(
            set.assess(),
            Err(CorrespondenceError::UnclassifiedLeftEvent)
        );
    }

    #[test]
    fn duplicate_endpoint_is_rejected() {
        let set = one_map(
            vec![
                observed("l-a", "ped-a", "slot-1"),
                observed("l-b", "ped-b", "slot-2"),
            ],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![
                pair("l-a", "r-a", IdentityBasis::Actor),
                pair("l-b", "r-a", IdentityBasis::EncounterSlot),
            ],
        );
        assert_eq!(
            set.assess(),
            Err(CorrespondenceError::DuplicateRightEndpoint)
        );
    }

    #[test]
    fn incomplete_candidate_universe_never_claims_unique_complete_mapping() {
        let mut set = one_map(
            vec![observed("l-a", "ped-a", "slot-1")],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        set.universe_status = CandidateUniverseStatus::Incomplete {
            evidence_ref: id("universe:partial"),
        };
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::Incomplete
        );
    }

    #[test]
    fn canonical_bytes_ignore_input_order() {
        let left_a = observed("l-a", "ped-a", "slot-1");
        let left_b = observed("l-b", "ped-b", "slot-2");
        let right_a = observed("r-a", "ped-a", "slot-1");
        let right_b = observed("r-b", "ped-b", "slot-2");
        let mut first = one_map(
            vec![left_a.clone(), left_b.clone()],
            vec![right_a.clone(), right_b.clone()],
            vec![
                pair("l-a", "r-a", IdentityBasis::Actor),
                pair("l-b", "r-b", IdentityBasis::Actor),
            ],
        );
        let mut second = first.clone();
        second.left_events.reverse();
        second.right_events.reverse();
        second.hypotheses[0].pairs.reverse();
        assert_eq!(
            first.canonical_bytes().unwrap(),
            second.canonical_bytes().unwrap()
        );
        // Keep this mutable to ensure the test checks serialization, not a shared borrow.
        first.hypotheses[0].justification_ref = id("assumption:changed");
        assert_ne!(
            first.canonical_bytes().unwrap(),
            second.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn incomplete_hypothesis_search_does_not_claim_an_exhaustive_envelope() {
        let mut set = one_map(
            vec![observed("l-a", "ped-a", "slot-1")],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        set.hypothesis_set_status = HypothesisSetStatus::IncompleteOrBudgetLimited {
            evidence_ref: id("search:budget-exhausted"),
        };
        assert_eq!(
            set.assess().unwrap().disposition,
            CorrespondenceDisposition::Incomplete
        );
    }

    #[test]
    fn duplicate_event_ids_and_hypothesis_ids_are_rejected() {
        let mut set = one_map(
            vec![observed("l-a", "ped-a", "slot-1")],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        set.left_events.push(observed("l-a", "ped-a", "slot-1"));
        assert_eq!(set.assess(), Err(CorrespondenceError::DuplicateEventId));

        let mut set = one_map(
            vec![observed("l-a", "ped-a", "slot-1")],
            vec![observed("r-a", "ped-a", "slot-1")],
            vec![pair("l-a", "r-a", IdentityBasis::Actor)],
        );
        set.hypotheses.push(set.hypotheses[0].clone());
        assert_eq!(
            set.assess(),
            Err(CorrespondenceError::DuplicateHypothesisId)
        );
    }
}

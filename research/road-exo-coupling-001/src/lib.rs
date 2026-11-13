#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const DOMAIN: &[u8] = b"symtropy.road-exo-coupling.draw-address.v0.1\0";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StableId(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityError {
    EmptyId,
    IdTooLong,
    NonCanonicalId,
    SelfDirectedPair,
    WrongBranchScope,
    DuplicateAddress,
    SharedValueMismatch,
}

impl StableId {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentityError> {
        let value = value.into();
        if value.is_empty() {
            return Err(IdentityError::EmptyId);
        }
        if value.len() > 192 {
            return Err(IdentityError::IdTooLong);
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._:/@".contains(&b))
        {
            return Err(IdentityError::NonCanonicalId);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// This choice defines what counts as the same stochastic event across worlds.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EventIdentity {
    ActorEvent {
        actor: StableId,
        tick: u64,
        event_kind: StableId,
    },
    /// Initiator and recipient roles are significant.
    DirectedPairEvent {
        initiator: StableId,
        recipient: StableId,
        tick: u64,
        event_kind: StableId,
    },
    /// Reuse across different occupants asserts slot-level exchangeability.
    SceneSlotEvent {
        slot: StableId,
        tick: u64,
        event_kind: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Coupling {
    SharedInvariant {
        equivalence_group: StableId,
        justification: StableId,
    },
    BranchSpecific {
        branch: StableId,
    },
    IndependentReplicate {
        branch: StableId,
        replicate: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DrawAddress {
    pub world: StableId,
    pub source: StableId,
    pub mechanism: StableId,
    pub event: EventIdentity,
    pub component: StableId,
    pub coupling: Coupling,
}

impl DrawAddress {
    pub fn new(
        world: StableId,
        source: StableId,
        mechanism: StableId,
        event: EventIdentity,
        component: StableId,
        coupling: Coupling,
    ) -> Result<Self, IdentityError> {
        if let EventIdentity::DirectedPairEvent {
            initiator,
            recipient,
            ..
        } = &event
            && initiator == recipient
        {
            return Err(IdentityError::SelfDirectedPair);
        }
        Ok(Self {
            world,
            source,
            mechanism,
            event,
            component,
            coupling,
        })
    }

    /// Canonical length-prefixed address bytes, not a hash or random output.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(192);
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.world);
        push_id(&mut out, &self.source);
        push_id(&mut out, &self.mechanism);
        match &self.event {
            EventIdentity::ActorEvent {
                actor,
                tick,
                event_kind,
            } => {
                out.push(1);
                push_id(&mut out, actor);
                out.extend_from_slice(&tick.to_le_bytes());
                push_id(&mut out, event_kind);
            }
            EventIdentity::DirectedPairEvent {
                initiator,
                recipient,
                tick,
                event_kind,
            } => {
                out.push(2);
                push_id(&mut out, initiator);
                push_id(&mut out, recipient);
                out.extend_from_slice(&tick.to_le_bytes());
                push_id(&mut out, event_kind);
            }
            EventIdentity::SceneSlotEvent {
                slot,
                tick,
                event_kind,
            } => {
                out.push(3);
                push_id(&mut out, slot);
                out.extend_from_slice(&tick.to_le_bytes());
                push_id(&mut out, event_kind);
            }
        }
        push_id(&mut out, &self.component);
        match &self.coupling {
            Coupling::SharedInvariant {
                equivalence_group,
                justification,
            } => {
                out.push(1);
                push_id(&mut out, equivalence_group);
                push_id(&mut out, justification);
                // No branch ID is included for an explicitly shared exogenous cause.
            }
            Coupling::BranchSpecific { branch } => {
                out.push(2);
                push_id(&mut out, branch);
            }
            Coupling::IndependentReplicate { branch, replicate } => {
                out.push(3);
                push_id(&mut out, branch);
                push_id(&mut out, replicate);
            }
        }
        out
    }

    fn validate_branch(&self, branch: &StableId) -> Result<(), IdentityError> {
        match &self.coupling {
            Coupling::SharedInvariant { .. } => Ok(()),
            Coupling::BranchSpecific { branch: b }
            | Coupling::IndependentReplicate { branch: b, .. }
                if b == branch =>
            {
                Ok(())
            }
            _ => Err(IdentityError::WrongBranchScope),
        }
    }
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let len = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrawPresence {
    Realized {
        value_commitment: [u8; 32],
    },
    /// The potential event existed but its causal conditions were not met.
    NotRealized {
        condition_witness: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrawRecord {
    pub branch: StableId,
    pub address: DrawAddress,
    pub presence: DrawPresence,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PairAudit {
    pub shared_value_matches: usize,
    pub absent_in_both: usize,
    pub conditionally_realized: usize,
    pub unpaired_left: usize,
    pub unpaired_right: usize,
}
impl PairAudit {
    /// Missing records are incomplete; they are not inferred to be absent events.
    pub fn is_complete(&self) -> bool {
        self.unpaired_left == 0 && self.unpaired_right == 0
    }
}

pub fn audit_shared_pair(
    left_branch: &StableId,
    left: &[DrawRecord],
    right_branch: &StableId,
    right: &[DrawRecord],
) -> Result<PairAudit, IdentityError> {
    let left_map = shared_records(left_branch, left)?;
    let right_map = shared_records(right_branch, right)?;
    let keys: BTreeSet<Vec<u8>> = left_map.keys().chain(right_map.keys()).cloned().collect();
    let mut report = PairAudit::default();
    for key in keys {
        match (left_map.get(&key), right_map.get(&key)) {
            (
                Some(DrawPresence::Realized {
                    value_commitment: a,
                }),
                Some(DrawPresence::Realized {
                    value_commitment: b,
                }),
            ) => {
                if a != b {
                    return Err(IdentityError::SharedValueMismatch);
                }
                report.shared_value_matches += 1;
            }
            (Some(DrawPresence::NotRealized { .. }), Some(DrawPresence::NotRealized { .. })) => {
                report.absent_in_both += 1
            }
            (Some(DrawPresence::Realized { .. }), Some(DrawPresence::NotRealized { .. }))
            | (Some(DrawPresence::NotRealized { .. }), Some(DrawPresence::Realized { .. })) => {
                report.conditionally_realized += 1
            }
            (Some(_), None) => report.unpaired_left += 1,
            (None, Some(_)) => report.unpaired_right += 1,
            (None, None) => unreachable!("key came from one map"),
        }
    }
    Ok(report)
}

fn shared_records(
    branch: &StableId,
    records: &[DrawRecord],
) -> Result<BTreeMap<Vec<u8>, DrawPresence>, IdentityError> {
    let mut seen = BTreeSet::new();
    let mut shared = BTreeMap::new();
    for record in records {
        if &record.branch != branch {
            return Err(IdentityError::WrongBranchScope);
        }
        record.address.validate_branch(branch)?;
        let key = record.address.canonical_bytes();
        if !seen.insert(key.clone()) {
            return Err(IdentityError::DuplicateAddress);
        }
        if matches!(&record.address.coupling, Coupling::SharedInvariant { .. }) {
            shared.insert(key, record.presence.clone());
        }
    }
    Ok(shared)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(s: &str) -> StableId {
        StableId::new(s).expect("fixture ID")
    }
    fn actor_address(branch: &str, shared: bool) -> DrawAddress {
        let coupling = if shared {
            Coupling::SharedInvariant {
                equivalence_group: id("crossing-weather"),
                justification: id("scm:weather-v1"),
            }
        } else {
            Coupling::BranchSpecific { branch: id(branch) }
        };
        DrawAddress::new(
            id("world-17"),
            id("weather-source"),
            id("gust-mechanism-v1"),
            EventIdentity::ActorEvent {
                actor: id("ped-004"),
                tick: 120,
                event_kind: id("policy-reaction-noise"),
            },
            id("uniform-0"),
            coupling,
        )
        .expect("valid address")
    }

    #[test]
    fn shared_actor_event_is_branch_invariant() {
        assert_eq!(
            actor_address("continue", true).canonical_bytes(),
            actor_address("yield", true).canonical_bytes()
        );
    }
    #[test]
    fn branch_local_address_contains_branch_identity() {
        assert_ne!(
            actor_address("continue", false).canonical_bytes(),
            actor_address("yield", false).canonical_bytes()
        );
    }
    #[test]
    fn address_is_independent_of_call_order() {
        let first = actor_address("continue", true).canonical_bytes();
        let _unrelated = actor_address("yield", false).canonical_bytes();
        let later = actor_address("continue", true).canonical_bytes();
        assert_eq!(first, later);
    }
    #[test]
    fn actor_event_and_scene_slot_are_distinct_hypotheses() {
        let actor = actor_address("continue", true);
        let slot = DrawAddress::new(
            id("world-17"),
            id("weather-source"),
            id("gust-mechanism-v1"),
            EventIdentity::SceneSlotEvent {
                slot: id("crosswalk-2-slot-7"),
                tick: 120,
                event_kind: id("policy-reaction-noise"),
            },
            id("uniform-0"),
            Coupling::SharedInvariant {
                equivalence_group: id("crossing-weather"),
                justification: id("scm:weather-v1"),
            },
        )
        .unwrap();
        assert_ne!(actor.canonical_bytes(), slot.canonical_bytes());
    }
    #[test]
    fn directed_pair_preserves_roles() {
        let make = |a: &str, b: &str| {
            DrawAddress::new(
                id("world-17"),
                id("interaction-source"),
                id("yield-response-v1"),
                EventIdentity::DirectedPairEvent {
                    initiator: id(a),
                    recipient: id(b),
                    tick: 121,
                    event_kind: id("gap-acceptance"),
                },
                id("uniform-0"),
                Coupling::SharedInvariant {
                    equivalence_group: id("pair-event"),
                    justification: id("same-directed-pair"),
                },
            )
            .unwrap()
        };
        assert_ne!(
            make("ego-1", "ped-004").canonical_bytes(),
            make("ped-004", "ego-1").canonical_bytes()
        );
    }
    #[test]
    fn duplicate_consumption_is_rejected() {
        let branch = id("continue");
        let record = DrawRecord {
            branch: branch.clone(),
            address: actor_address("continue", true),
            presence: DrawPresence::Realized {
                value_commitment: [7; 32],
            },
        };
        assert_eq!(
            shared_records(&branch, &[record.clone(), record]),
            Err(IdentityError::DuplicateAddress)
        );
    }
    #[test]
    fn mismatched_value_for_same_shared_event_is_rejected() {
        let l = id("continue");
        let r = id("yield");
        let left = [DrawRecord {
            branch: l.clone(),
            address: actor_address("continue", true),
            presence: DrawPresence::Realized {
                value_commitment: [1; 32],
            },
        }];
        let right = [DrawRecord {
            branch: r.clone(),
            address: actor_address("yield", true),
            presence: DrawPresence::Realized {
                value_commitment: [2; 32],
            },
        }];
        assert_eq!(
            audit_shared_pair(&l, &left, &r, &right),
            Err(IdentityError::SharedValueMismatch)
        );
    }
    #[test]
    fn conditional_presence_is_not_a_value_mismatch() {
        let l = id("continue");
        let r = id("yield");
        let address = actor_address("continue", true);
        let left = [DrawRecord {
            branch: l.clone(),
            address: address.clone(),
            presence: DrawPresence::Realized {
                value_commitment: [9; 32],
            },
        }];
        let right = [DrawRecord {
            branch: r.clone(),
            address,
            presence: DrawPresence::NotRealized {
                condition_witness: id("ego-outside-visibility-zone"),
            },
        }];
        let audit = audit_shared_pair(&l, &left, &r, &right).unwrap();
        assert_eq!(audit.conditionally_realized, 1);
        assert!(audit.is_complete());
    }
    #[test]
    fn missing_counterpart_is_incomplete_not_assumed_conditional() {
        let l = id("continue");
        let r = id("yield");
        let left = [DrawRecord {
            branch: l.clone(),
            address: actor_address("continue", true),
            presence: DrawPresence::Realized {
                value_commitment: [9; 32],
            },
        }];
        let audit = audit_shared_pair(&l, &left, &r, &[]).unwrap();
        assert_eq!(audit.unpaired_left, 1);
        assert!(!audit.is_complete());
    }
    #[test]
    fn branch_scope_cannot_be_replayed_in_sibling_branch() {
        let branch = id("continue");
        let record = DrawRecord {
            branch: id("yield"),
            address: actor_address("yield", false),
            presence: DrawPresence::Realized {
                value_commitment: [1; 32],
            },
        };
        assert_eq!(
            shared_records(&branch, &[record]),
            Err(IdentityError::WrongBranchScope)
        );
    }
    #[test]
    fn malformed_ids_are_rejected() {
        assert_eq!(StableId::new(""), Err(IdentityError::EmptyId));
        assert_eq!(
            StableId::new("actor with spaces"),
            Err(IdentityError::NonCanonicalId)
        );
    }
}

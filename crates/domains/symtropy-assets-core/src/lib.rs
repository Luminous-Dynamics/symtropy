// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Separated asset relations for civilization-scale simulation.
//!
//! Ownership, operation, custody, effective control, beneficial interest, and
//! creditor interest are represented independently. No relation implies any
//! other relation, and this crate does not create physical existence authority
//! for the referenced asset.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_epistemics_core::EpistemicRef;
use symtropy_game_state::StableId;
use symtropy_residents::CustodyItem;

/// One semantically distinct relationship between a party and an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AssetRelationKind {
    /// Legal/economic ownership record or claim.
    Owner,
    /// Party authorized/assigned to operate the asset.
    Operator,
    /// Party responsible for custody/continuity of the asset.
    Custodian,
    /// Party exercising effective control over the asset.
    Controller,
    /// Party entitled to beneficial/economic proceeds.
    Beneficiary,
    /// Party holding a creditor/security interest.
    Creditor,
}

/// Optional exact fractional interest expressed in parts-per-million.
///
/// The ledger does not require interests across parties to sum to one million:
/// contested/overlapping records are representable rather than silently resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InterestPpm(u32);

impl InterestPpm {
    pub const MAX: u32 = 1_000_000;

    /// Creates an exact interest fraction from 0..=1,000,000 ppm.
    pub fn new(value: u32) -> Result<Self, AssetError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(AssetError::InvalidInterestPpm(value))
        }
    }

    /// Returns the stored parts-per-million value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Immutable relation between one party and one externally identified asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRelation {
    /// Stable relation identity.
    pub id: StableId,
    /// Asset identity owned by the relevant physical/inventory domain.
    pub asset_id: StableId,
    /// Actor/institution/household/organization holding this relation.
    pub party_id: StableId,
    /// Exact semantic relationship.
    pub kind: AssetRelationKind,
    /// Optional location relevant to this relation, especially custody/control.
    pub location_id: Option<StableId>,
    /// Optional exact beneficial/economic fraction.
    pub interest_ppm: Option<InterestPpm>,
    /// First canonical tick at which the relation applies.
    pub valid_from_tick: u64,
    /// Optional final canonical tick, inclusive.
    pub valid_until_tick: Option<u64>,
    /// Optional epistemic basis for the recorded relation.
    pub epistemic_basis: BTreeSet<EpistemicRef>,
    /// Causal-history event that introduced this relation.
    pub source_event_id: StableId,
}

impl AssetRelation {
    /// Returns whether this relation's own temporal window includes `tick`.
    pub fn temporally_active(&self, tick: u64) -> bool {
        tick >= self.valid_from_tick
            && self
                .valid_until_tick
                .is_none_or(|until| tick <= until)
    }
}

/// Immutable early termination of one relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationTermination {
    /// Stable termination identity.
    pub id: StableId,
    /// Relation being terminated.
    pub relation_id: StableId,
    /// Canonical tick at which the relation ceases to be active.
    pub terminated_tick: u64,
    /// Causal-history event recording termination.
    pub source_event_id: StableId,
}

/// Append-only asset-relation ledger.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetLedger {
    relations: BTreeMap<StableId, AssetRelation>,
    terminations: BTreeMap<StableId, RelationTermination>,
    termination_by_relation: BTreeMap<StableId, StableId>,
}

impl AssetLedger {
    /// Creates an empty relation ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one immutable asset relation.
    pub fn record_relation(&mut self, relation: AssetRelation) -> Result<(), AssetError> {
        validate_window(
            relation.valid_from_tick,
            relation.valid_until_tick,
            &relation.id,
        )?;
        if self.relations.contains_key(&relation.id) {
            return Err(AssetError::DuplicateRelation(relation.id));
        }
        self.relations.insert(relation.id.clone(), relation);
        Ok(())
    }

    /// Returns one relation regardless of current activity.
    pub fn relation(&self, relation_id: &StableId) -> Option<&AssetRelation> {
        self.relations.get(relation_id)
    }

    /// Records early termination without rewriting the relation record.
    pub fn terminate_relation(
        &mut self,
        termination: RelationTermination,
    ) -> Result<(), AssetError> {
        if self.terminations.contains_key(&termination.id) {
            return Err(AssetError::DuplicateTermination(termination.id));
        }
        let relation = self
            .relations
            .get(&termination.relation_id)
            .ok_or_else(|| AssetError::UnknownRelation(termination.relation_id.clone()))?;
        if self
            .termination_by_relation
            .contains_key(&termination.relation_id)
        {
            return Err(AssetError::RelationAlreadyTerminated(
                termination.relation_id,
            ));
        }
        if termination.terminated_tick < relation.valid_from_tick {
            return Err(AssetError::TerminationBeforeRelation {
                relation_id: termination.relation_id,
                valid_from_tick: relation.valid_from_tick,
                terminated_tick: termination.terminated_tick,
            });
        }
        self.termination_by_relation.insert(
            termination.relation_id.clone(),
            termination.id.clone(),
        );
        self.terminations
            .insert(termination.id.clone(), termination);
        Ok(())
    }

    /// Returns the termination for one relation, if any.
    pub fn termination_for_relation(
        &self,
        relation_id: &StableId,
    ) -> Option<&RelationTermination> {
        self.termination_by_relation
            .get(relation_id)
            .and_then(|id| self.terminations.get(id))
    }

    /// Returns whether one relation is active at `tick`.
    pub fn relation_is_active(&self, relation_id: &StableId, tick: u64) -> bool {
        self.relations.get(relation_id).is_some_and(|relation| {
            relation.temporally_active(tick)
                && self
                    .termination_for_relation(relation_id)
                    .is_none_or(|termination| tick < termination.terminated_tick)
        })
    }

    /// Returns all active relations for one asset at `tick`.
    pub fn active_relations_for_asset<'a>(
        &'a self,
        asset_id: &'a StableId,
        tick: u64,
    ) -> impl Iterator<Item = &'a AssetRelation> {
        self.relations.values().filter(move |relation| {
            relation.asset_id == *asset_id && self.relation_is_active(&relation.id, tick)
        })
    }

    /// Returns active relations of one exact kind for an asset.
    pub fn active_relations_by_kind<'a>(
        &'a self,
        asset_id: &'a StableId,
        kind: AssetRelationKind,
        tick: u64,
    ) -> impl Iterator<Item = &'a AssetRelation> {
        self.active_relations_for_asset(asset_id, tick)
            .filter(move |relation| relation.kind == kind)
    }
}

/// Adapts the existing resident continuity-custody record into the generic
/// asset relation vocabulary without mutating or superseding resident custody.
///
/// `CustodyItem` remains the source record for household/resident continuity;
/// this helper merely allows asset-aware systems to view the same state through
/// the `Custodian` relation class.
pub fn relation_from_custody_item(
    item: &CustodyItem,
    relation_id: StableId,
    valid_from_tick: u64,
    source_event_id: StableId,
) -> AssetRelation {
    AssetRelation {
        id: relation_id,
        asset_id: item.id.clone(),
        party_id: item.custodian_id.clone(),
        kind: AssetRelationKind::Custodian,
        location_id: Some(item.location_id.clone()),
        interest_ppm: None,
        valid_from_tick,
        valid_until_tick: None,
        epistemic_basis: BTreeSet::new(),
        source_event_id,
    }
}

fn validate_window(
    valid_from_tick: u64,
    valid_until_tick: Option<u64>,
    relation_id: &StableId,
) -> Result<(), AssetError> {
    if valid_until_tick.is_some_and(|until| until < valid_from_tick) {
        Err(AssetError::InvalidTemporalWindow {
            relation_id: relation_id.clone(),
            valid_from_tick,
            valid_until_tick,
        })
    } else {
        Ok(())
    }
}

/// Structural failures in asset-relation history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    DuplicateRelation(StableId),
    UnknownRelation(StableId),
    DuplicateTermination(StableId),
    RelationAlreadyTerminated(StableId),
    InvalidInterestPpm(u32),
    InvalidTemporalWindow {
        relation_id: StableId,
        valid_from_tick: u64,
        valid_until_tick: Option<u64>,
    },
    TerminationBeforeRelation {
        relation_id: StableId,
        valid_from_tick: u64,
        terminated_tick: u64,
    },
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRelation(id) => write!(formatter, "asset relation {id} already exists"),
            Self::UnknownRelation(id) => write!(formatter, "unknown asset relation {id}"),
            Self::DuplicateTermination(id) => {
                write!(formatter, "asset relation termination {id} already exists")
            }
            Self::RelationAlreadyTerminated(id) => {
                write!(formatter, "asset relation {id} is already terminated")
            }
            Self::InvalidInterestPpm(value) => write!(
                formatter,
                "asset interest {value} ppm exceeds {}",
                InterestPpm::MAX
            ),
            Self::InvalidTemporalWindow {
                relation_id,
                valid_from_tick,
                valid_until_tick,
            } => write!(
                formatter,
                "asset relation {relation_id} has invalid window {valid_from_tick}..={valid_until_tick:?}"
            ),
            Self::TerminationBeforeRelation {
                relation_id,
                valid_from_tick,
                terminated_tick,
            } => write!(
                formatter,
                "asset relation {relation_id} begins at {valid_from_tick} and cannot terminate earlier at {terminated_tick}"
            ),
        }
    }
}

impl Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn relation(id_value: &str, party: &str, kind: AssetRelationKind) -> AssetRelation {
        AssetRelation {
            id: id(id_value),
            asset_id: id("asset:ship-7"),
            party_id: id(party),
            kind,
            location_id: Some(id("place:orbital-yard")),
            interest_ppm: None,
            valid_from_tick: 10,
            valid_until_tick: None,
            epistemic_basis: BTreeSet::new(),
            source_event_id: id(&format!("event:{id_value}")),
        }
    }

    #[test]
    fn ownership_does_not_imply_operation_control_or_custody() {
        let mut ledger = AssetLedger::new();
        ledger
            .record_relation(relation(
                "relation:owner",
                "institution:merchant-house",
                AssetRelationKind::Owner,
            ))
            .expect("owner");
        ledger
            .record_relation(relation(
                "relation:operator",
                "institution:shipping-coop",
                AssetRelationKind::Operator,
            ))
            .expect("operator");
        ledger
            .record_relation(relation(
                "relation:controller",
                "institution:fleet-command",
                AssetRelationKind::Controller,
            ))
            .expect("controller");

        let owners = ledger
            .active_relations_by_kind(&id("asset:ship-7"), AssetRelationKind::Owner, 20)
            .map(|relation| relation.party_id.clone())
            .collect::<BTreeSet<_>>();
        let operators = ledger
            .active_relations_by_kind(&id("asset:ship-7"), AssetRelationKind::Operator, 20)
            .map(|relation| relation.party_id.clone())
            .collect::<BTreeSet<_>>();
        let controllers = ledger
            .active_relations_by_kind(&id("asset:ship-7"), AssetRelationKind::Controller, 20)
            .map(|relation| relation.party_id.clone())
            .collect::<BTreeSet<_>>();
        let custodians = ledger
            .active_relations_by_kind(&id("asset:ship-7"), AssetRelationKind::Custodian, 20)
            .count();

        assert_eq!(owners, BTreeSet::from([id("institution:merchant-house")]));
        assert_eq!(operators, BTreeSet::from([id("institution:shipping-coop")]));
        assert_eq!(controllers, BTreeSet::from([id("institution:fleet-command")]));
        assert_eq!(custodians, 0);
    }

    #[test]
    fn overlapping_owner_records_are_representable() {
        let mut ledger = AssetLedger::new();
        for (relation_id, party) in [
            ("relation:owner-a", "institution:claimant-a"),
            ("relation:owner-b", "institution:claimant-b"),
        ] {
            let mut ownership = relation(relation_id, party, AssetRelationKind::Owner);
            ownership.interest_ppm = Some(InterestPpm::new(1_000_000).expect("full interest"));
            ledger.record_relation(ownership).expect("ownership record");
        }
        assert_eq!(
            ledger
                .active_relations_by_kind(&id("asset:ship-7"), AssetRelationKind::Owner, 20)
                .count(),
            2
        );
    }

    #[test]
    fn resident_custody_projects_without_becoming_ownership() {
        let item = CustodyItem {
            id: id("asset:medicine-case"),
            custodian_id: id("resident:mina"),
            location_id: id("place:crawler"),
            consent_required: true,
        };
        let projected = relation_from_custody_item(
            &item,
            id("relation:medicine-custody"),
            40,
            id("event:custody-projection"),
        );
        assert_eq!(projected.asset_id, item.id);
        assert_eq!(projected.party_id, item.custodian_id);
        assert_eq!(projected.kind, AssetRelationKind::Custodian);
        assert_ne!(projected.kind, AssetRelationKind::Owner);
        assert!(item.consent_required);
    }

    #[test]
    fn termination_ends_future_activity_but_retains_relation() {
        let mut ledger = AssetLedger::new();
        ledger
            .record_relation(relation(
                "relation:operator",
                "institution:shipping-coop",
                AssetRelationKind::Operator,
            ))
            .expect("operator");
        ledger
            .terminate_relation(RelationTermination {
                id: id("termination:operator"),
                relation_id: id("relation:operator"),
                terminated_tick: 30,
                source_event_id: id("event:operator-termination"),
            })
            .expect("terminate");

        assert!(ledger.relation_is_active(&id("relation:operator"), 29));
        assert!(!ledger.relation_is_active(&id("relation:operator"), 30));
        assert!(ledger.relation(&id("relation:operator")).is_some());
        assert!(
            ledger
                .termination_for_relation(&id("relation:operator"))
                .is_some()
        );
    }

    #[test]
    fn invalid_interest_fails_closed() {
        assert_eq!(
            InterestPpm::new(1_000_001),
            Err(AssetError::InvalidInterestPpm(1_000_001))
        );
    }
}

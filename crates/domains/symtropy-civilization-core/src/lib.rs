// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dependency-light institutional state for civilization-scale simulation.
//!
//! This crate owns only institutional records and scoped normative authority.
//! It deliberately does **not** model effective control, ownership, legitimacy,
//! belief, succession truth, physical possession, or military capability. Those
//! concepts may disagree with institutional records and belong to separate
//! authorities under `docs/CIVILIZATIONAL_AUTHORITY_CONTRACT.md`.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Generic institution with no privileged political/economic form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Institution {
    /// Stable institution identity.
    pub id: StableId,
    /// Human-readable name; never used as identity.
    pub name: String,
    /// Extensible descriptive tags such as `hereditary`, `cooperative`,
    /// `religious-order`, `fleet-command`, or scenario-specific values.
    pub tags: BTreeSet<String>,
    /// Current institution-recorded membership state by actor.
    memberships: BTreeMap<StableId, MembershipRecord>,
    /// Office definitions owned by this institution.
    offices: BTreeMap<StableId, OfficeDefinition>,
    /// Current institution-recorded office holders by office.
    office_holders: BTreeMap<StableId, OfficeHolderRecord>,
    /// Scoped normative authority grants issued by this institution.
    authority_grants: BTreeMap<StableId, AuthorityGrant>,
}

impl Institution {
    /// Creates an empty generic institution.
    pub fn new(id: StableId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            tags: BTreeSet::new(),
            memberships: BTreeMap::new(),
            offices: BTreeMap::new(),
            office_holders: BTreeMap::new(),
            authority_grants: BTreeMap::new(),
        }
    }

    /// Returns current membership records in deterministic actor order.
    pub fn memberships(&self) -> impl Iterator<Item = &MembershipRecord> {
        self.memberships.values()
    }

    /// Returns office definitions in deterministic identity order.
    pub fn offices(&self) -> impl Iterator<Item = &OfficeDefinition> {
        self.offices.values()
    }

    /// Returns current office-holder records in deterministic office order.
    pub fn office_holders(&self) -> impl Iterator<Item = &OfficeHolderRecord> {
        self.office_holders.values()
    }

    /// Returns authority grants in deterministic grant-identity order.
    pub fn authority_grants(&self) -> impl Iterator<Item = &AuthorityGrant> {
        self.authority_grants.values()
    }

    /// Records current membership state.
    ///
    /// Replacing a record is permitted only at the same or a later canonical
    /// tick. Historical membership changes remain events in the owning history
    /// layer; this map is only the current institutional projection.
    pub fn record_membership(&mut self, record: MembershipRecord) -> Result<(), InstitutionError> {
        if let Some(existing) = self.memberships.get(&record.actor_id)
            && record.recorded_tick < existing.recorded_tick
        {
            return Err(InstitutionError::StaleMembershipRecord {
                actor_id: record.actor_id,
                existing_tick: existing.recorded_tick,
                attempted_tick: record.recorded_tick,
            });
        }
        self.memberships.insert(record.actor_id.clone(), record);
        Ok(())
    }

    /// Adds an office definition. Office identity is immutable in V0.
    pub fn define_office(&mut self, office: OfficeDefinition) -> Result<(), InstitutionError> {
        if self.offices.contains_key(&office.id) {
            return Err(InstitutionError::DuplicateOffice(office.id));
        }
        self.offices.insert(office.id.clone(), office);
        Ok(())
    }

    /// Records the institution's current holder of one known office.
    ///
    /// This is explicitly an **institutional record**, not proof that the actor
    /// has effective control, external recognition, ownership, or legitimacy.
    pub fn record_office_holder(
        &mut self,
        record: OfficeHolderRecord,
    ) -> Result<(), InstitutionError> {
        if !self.offices.contains_key(&record.office_id) {
            return Err(InstitutionError::UnknownOffice(record.office_id));
        }
        if let Some(existing) = self.office_holders.get(&record.office_id)
            && record.recorded_tick < existing.recorded_tick
        {
            return Err(InstitutionError::StaleOfficeHolderRecord {
                office_id: record.office_id,
                existing_tick: existing.recorded_tick,
                attempted_tick: record.recorded_tick,
            });
        }
        self.office_holders.insert(record.office_id.clone(), record);
        Ok(())
    }

    /// Adds one immutable scoped authority grant.
    pub fn grant_authority(&mut self, grant: AuthorityGrant) -> Result<(), InstitutionError> {
        if self.authority_grants.contains_key(&grant.id) {
            return Err(InstitutionError::DuplicateAuthorityGrant(grant.id));
        }
        if let AuthorityPrincipal::Office(office_id) = &grant.principal
            && !self.offices.contains_key(office_id)
        {
            return Err(InstitutionError::UnknownOffice(office_id.clone()));
        }
        if grant
            .valid_until_tick
            .is_some_and(|until| until < grant.valid_from_tick)
        {
            return Err(InstitutionError::InvalidAuthorityWindow {
                grant_id: grant.id,
                valid_from_tick: grant.valid_from_tick,
                valid_until_tick: grant.valid_until_tick,
            });
        }
        self.authority_grants.insert(grant.id.clone(), grant);
        Ok(())
    }

    /// Returns active grants issued directly to one exact principal.
    pub fn active_grants_for_principal<'a>(
        &'a self,
        principal: &'a AuthorityPrincipal,
        tick: u64,
    ) -> impl Iterator<Item = &'a AuthorityGrant> {
        self.authority_grants
            .values()
            .filter(move |grant| &grant.principal == principal && grant.is_active(tick))
    }

    /// Returns currently active office-derived grants for an actor according to
    /// this institution's own current office records.
    ///
    /// This does not assert that the actor can physically exercise the power.
    pub fn active_office_grants_for_actor<'a>(
        &'a self,
        actor_id: &'a StableId,
        tick: u64,
    ) -> impl Iterator<Item = &'a AuthorityGrant> {
        self.authority_grants.values().filter(move |grant| {
            let AuthorityPrincipal::Office(office_id) = &grant.principal else {
                return false;
            };
            let Some(holder) = self.office_holders.get(office_id) else {
                return false;
            };
            holder.holder_id == *actor_id
                && holder.recorded_tick <= tick
                && grant.is_active(tick)
        })
    }
}

/// Current membership state as recorded by one institution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRecord {
    /// Actor whose membership is being recorded.
    pub actor_id: StableId,
    /// Current institutional membership state.
    pub status: MembershipStatus,
    /// Canonical tick at which the institution recorded this state.
    pub recorded_tick: u64,
    /// Causal-history event supporting this current projection.
    pub source_event_id: StableId,
}

/// Institution-local membership state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MembershipStatus {
    /// Actor is currently a member.
    Active,
    /// Membership exists but current participation/rights are suspended.
    Suspended,
    /// Actor was a member but is no longer recorded as current.
    Former,
}

/// Generic office definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeDefinition {
    /// Stable office identity.
    pub id: StableId,
    /// Human-readable title.
    pub name: String,
    /// Extensible office descriptors; these carry no engine-privileged meaning.
    pub tags: BTreeSet<String>,
}

/// Current office holder according to this institution's own records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeHolderRecord {
    /// Office whose holder is recorded.
    pub office_id: StableId,
    /// Actor recorded as holder.
    pub holder_id: StableId,
    /// Canonical tick at which this record became the institution's current view.
    pub recorded_tick: u64,
    /// Causal-history event supporting this current projection.
    pub source_event_id: StableId,
}

/// Principal to whom an institution grants normative authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuthorityPrincipal {
    /// Exact actor receives authority directly.
    Actor(StableId),
    /// Whoever this institution currently records as holding the office receives
    /// the office-scoped authority.
    Office(StableId),
    /// Another institution receives authority as an institution. This does not
    /// implicitly delegate the authority to every member of that institution.
    Institution(StableId),
}

/// Machine-readable action namespace for one authority grant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthorityCapability {
    /// Domain such as `treasury`, `fleet-command`, `treaty`, or scenario-specific vocabulary.
    pub namespace: String,
    /// Operation such as `spend`, `order`, `sign`, or scenario-specific vocabulary.
    pub operation: String,
}

/// Exact scope of one authority grant.
///
/// `None` means the dimension is not part of the grant's scope, not a magical
/// wildcard query. Hierarchical/wildcard scope matching is deliberately deferred.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthorityScope {
    /// Optional jurisdiction/region/institutional domain.
    pub jurisdiction_id: Option<StableId>,
    /// Optional exact target asset, institution, force, account, or other subject.
    pub target_id: Option<StableId>,
}

/// Immutable scoped normative authority issued by an institution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityGrant {
    /// Stable grant identity.
    pub id: StableId,
    /// Principal receiving the normative authority.
    pub principal: AuthorityPrincipal,
    /// Action this grant authorizes under the issuing institution's rules.
    pub capability: AuthorityCapability,
    /// Exact authority scope.
    pub scope: AuthorityScope,
    /// First canonical tick at which the grant is active.
    pub valid_from_tick: u64,
    /// Optional final canonical tick, inclusive.
    pub valid_until_tick: Option<u64>,
    /// Causal-history event that issued/recorded this grant.
    pub source_event_id: StableId,
}

impl AuthorityGrant {
    /// Returns whether the grant is temporally active at `tick`.
    pub fn is_active(&self, tick: u64) -> bool {
        tick >= self.valid_from_tick
            && self
                .valid_until_tick
                .is_none_or(|valid_until| tick <= valid_until)
    }
}

/// Structural failures in one institution's current recorded state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionError {
    /// Office identity was already defined.
    DuplicateOffice(StableId),
    /// A record or office-scoped grant referenced an office this institution does not define.
    UnknownOffice(StableId),
    /// Authority-grant identity was already used.
    DuplicateAuthorityGrant(StableId),
    /// Current membership projection attempted to move backward in canonical time.
    StaleMembershipRecord {
        actor_id: StableId,
        existing_tick: u64,
        attempted_tick: u64,
    },
    /// Current office-holder projection attempted to move backward in canonical time.
    StaleOfficeHolderRecord {
        office_id: StableId,
        existing_tick: u64,
        attempted_tick: u64,
    },
    /// Authority validity ended before it began.
    InvalidAuthorityWindow {
        grant_id: StableId,
        valid_from_tick: u64,
        valid_until_tick: Option<u64>,
    },
}

impl fmt::Display for InstitutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateOffice(office_id) => write!(formatter, "office {office_id} already exists"),
            Self::UnknownOffice(office_id) => write!(formatter, "unknown office {office_id}"),
            Self::DuplicateAuthorityGrant(grant_id) => {
                write!(formatter, "authority grant {grant_id} already exists")
            }
            Self::StaleMembershipRecord {
                actor_id,
                existing_tick,
                attempted_tick,
            } => write!(
                formatter,
                "membership record for {actor_id} moved backward from tick {existing_tick} to {attempted_tick}"
            ),
            Self::StaleOfficeHolderRecord {
                office_id,
                existing_tick,
                attempted_tick,
            } => write!(
                formatter,
                "office-holder record for {office_id} moved backward from tick {existing_tick} to {attempted_tick}"
            ),
            Self::InvalidAuthorityWindow {
                grant_id,
                valid_from_tick,
                valid_until_tick,
            } => write!(
                formatter,
                "authority grant {grant_id} has invalid window {valid_from_tick}..={valid_until_tick:?}"
            ),
        }
    }
}

impl Error for InstitutionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn office(id_value: &str, name: &str) -> OfficeDefinition {
        OfficeDefinition {
            id: id(id_value),
            name: name.into(),
            tags: BTreeSet::new(),
        }
    }

    #[test]
    fn arbitrary_institution_forms_share_one_primitive() {
        let mut hereditary = Institution::new(id("institution:ardent"), "Ardent Line");
        hereditary.tags.insert("hereditary".into());
        let mut cooperative = Institution::new(id("institution:helix"), "Helix Cooperative");
        cooperative.tags.insert("worker-cooperative".into());
        assert!(hereditary.tags.contains("hereditary"));
        assert!(cooperative.tags.contains("worker-cooperative"));
    }

    #[test]
    fn office_holder_receives_only_institution_recorded_office_authority() {
        let mut institution = Institution::new(id("institution:navy"), "Orbital Navy");
        institution
            .define_office(office("office:admiral", "Admiral"))
            .expect("define admiral office");
        institution
            .record_office_holder(OfficeHolderRecord {
                office_id: id("office:admiral"),
                holder_id: id("actor:kael"),
                recorded_tick: 10,
                source_event_id: id("event:appointment"),
            })
            .expect("record office holder");
        institution
            .grant_authority(AuthorityGrant {
                id: id("grant:fleet-orders"),
                principal: AuthorityPrincipal::Office(id("office:admiral")),
                capability: AuthorityCapability {
                    namespace: "fleet-command".into(),
                    operation: "issue-order".into(),
                },
                scope: AuthorityScope {
                    jurisdiction_id: Some(id("region:inner-system")),
                    target_id: None,
                },
                valid_from_tick: 10,
                valid_until_tick: None,
                source_event_id: id("event:authority"),
            })
            .expect("grant office authority");

        assert_eq!(
            institution
                .active_office_grants_for_actor(&id("actor:kael"), 20)
                .count(),
            1
        );
        assert_eq!(
            institution
                .active_office_grants_for_actor(&id("actor:mara"), 20)
                .count(),
            0
        );
    }

    #[test]
    fn unknown_office_cannot_receive_local_office_grant() {
        let mut institution = Institution::new(id("institution:navy"), "Orbital Navy");
        let result = institution.grant_authority(AuthorityGrant {
            id: id("grant:unknown-office"),
            principal: AuthorityPrincipal::Office(id("office:missing")),
            capability: AuthorityCapability {
                namespace: "fleet-command".into(),
                operation: "issue-order".into(),
            },
            scope: AuthorityScope {
                jurisdiction_id: None,
                target_id: None,
            },
            valid_from_tick: 1,
            valid_until_tick: None,
            source_event_id: id("event:authority"),
        });
        assert!(matches!(result, Err(InstitutionError::UnknownOffice(_))));
    }

    #[test]
    fn expired_authority_is_not_active() {
        let mut institution = Institution::new(id("institution:treasury"), "Treasury");
        let principal = AuthorityPrincipal::Actor(id("actor:mina"));
        institution
            .grant_authority(AuthorityGrant {
                id: id("grant:temporary"),
                principal: principal.clone(),
                capability: AuthorityCapability {
                    namespace: "treasury".into(),
                    operation: "spend".into(),
                },
                scope: AuthorityScope {
                    jurisdiction_id: None,
                    target_id: Some(id("account:relief")),
                },
                valid_from_tick: 5,
                valid_until_tick: Some(10),
                source_event_id: id("event:temporary-grant"),
            })
            .expect("grant temporary authority");
        assert_eq!(institution.active_grants_for_principal(&principal, 10).count(), 1);
        assert_eq!(institution.active_grants_for_principal(&principal, 11).count(), 0);
    }

    #[test]
    fn current_projection_rejects_backward_office_record() {
        let mut institution = Institution::new(id("institution:council"), "Council");
        institution
            .define_office(office("office:speaker", "Speaker"))
            .expect("define speaker");
        institution
            .record_office_holder(OfficeHolderRecord {
                office_id: id("office:speaker"),
                holder_id: id("actor:first"),
                recorded_tick: 20,
                source_event_id: id("event:first"),
            })
            .expect("record current holder");
        let result = institution.record_office_holder(OfficeHolderRecord {
            office_id: id("office:speaker"),
            holder_id: id("actor:older"),
            recorded_tick: 19,
            source_event_id: id("event:older"),
        });
        assert!(matches!(
            result,
            Err(InstitutionError::StaleOfficeHolderRecord { .. })
        ));
    }

    #[test]
    fn invalid_authority_window_fails_closed() {
        let mut institution = Institution::new(id("institution:test"), "Test");
        let result = institution.grant_authority(AuthorityGrant {
            id: id("grant:bad-window"),
            principal: AuthorityPrincipal::Actor(id("actor:test")),
            capability: AuthorityCapability {
                namespace: "test".into(),
                operation: "act".into(),
            },
            scope: AuthorityScope {
                jurisdiction_id: None,
                target_id: None,
            },
            valid_from_tick: 20,
            valid_until_tick: Some(19),
            source_event_id: id("event:bad-window"),
        });
        assert!(matches!(
            result,
            Err(InstitutionError::InvalidAuthorityWindow { .. })
        ));
    }
}

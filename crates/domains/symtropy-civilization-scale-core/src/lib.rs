// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Identity-safe civilization projections over an external fidelity authority.
//!
//! This crate does not choose simulation fidelity, prove transforms lossless,
//! reconcile physical/ecological fluxes, or authenticate external snapshots.
//! Those remain owned by the Living World / domain-specific authority stack.
//!
//! Civilization adds one narrower invariant: coarse population summaries may not
//! fabricate exact historical identities. A background cohort can be materialized
//! into a new persistent actor only from the materialization tick forward; an
//! already-persistent actor may be summarized for cheaper simulation but retains
//! the same stable identity and may later re-expand without being recreated.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Opaque reference to one exact transition/certificate owned by an external
/// fidelity/representation authority (for example Living World).
///
/// The reference is evidence identity only. Constructing it here does not prove
/// the external record valid; a product adapter must resolve it against the
/// current external authority before consequential use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaleAuthorityRef {
    /// Producing authority family, e.g. `living-world-transition`.
    pub authority_namespace: String,
    /// Exact external authority/certificate/receipt identity.
    pub authority_record_id: StableId,
    /// Source logical snapshot identity.
    pub source_snapshot_id: StableId,
    /// Destination logical snapshot identity.
    pub destination_snapshot_id: StableId,
}

impl ScaleAuthorityRef {
    pub fn validate(&self) -> Result<(), ScaleError> {
        if self.authority_namespace.trim().is_empty() {
            return Err(ScaleError::EmptyAuthorityNamespace);
        }
        if self.source_snapshot_id == self.destination_snapshot_id {
            return Err(ScaleError::IdenticalSnapshotBoundary(
                self.source_snapshot_id.clone(),
            ));
        }
        Ok(())
    }
}

/// Extensible descriptor for anonymous population grouping.
///
/// Descriptor values aid aggregation; they do not create named individuals or
/// establish demographic truth outside the authority that produced the cohort.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CohortDescriptor {
    pub namespace: String,
    pub dimensions: BTreeMap<String, String>,
}

impl CohortDescriptor {
    pub fn validate(&self) -> Result<(), ScaleError> {
        if self.namespace.trim().is_empty() {
            return Err(ScaleError::EmptyCohortNamespace);
        }
        Ok(())
    }
}

/// Anonymous aggregate population. No member has a persistent `StableId` merely
/// because the cohort exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationCohort {
    pub id: StableId,
    pub descriptor: CohortDescriptor,
    pub anonymous_count: u64,
    /// Exact external evidence supporting this aggregate projection.
    pub authority_ref: ScaleAuthorityRef,
}

impl PopulationCohort {
    pub fn validate(&self) -> Result<(), ScaleError> {
        self.descriptor.validate()?;
        self.authority_ref.validate()?;
        Ok(())
    }
}

/// Whether a persistent actor is currently represented in detailed or summarized
/// form. Both forms retain the same stable identity and historical continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistentRepresentation {
    Exact,
    Summarized,
}

/// Opaque reference to state retained when a persistent actor is summarized.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PersistentStateRef {
    pub namespace: String,
    pub record_id: StableId,
}

/// Persistent actor continuity record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentActorProjection {
    pub actor_id: StableId,
    /// First tick at which this exact identity became canonical.
    /// No API in this crate can move this boundary backward.
    pub canonical_since_tick: u64,
    pub representation: PersistentRepresentation,
    /// Most recent tick at which an exact representation was canonical.
    pub last_exact_tick: u64,
    /// Optional compact state references retained while summarized.
    pub summary_state_refs: BTreeSet<PersistentStateRef>,
    /// Authority for the most recent representation transition.
    pub authority_ref: ScaleAuthorityRef,
    pub source_event_id: StableId,
}

impl PersistentActorProjection {
    fn validate(&self) -> Result<(), ScaleError> {
        self.authority_ref.validate()?;
        if self.last_exact_tick < self.canonical_since_tick {
            return Err(ScaleError::ExactTickBeforeCanonicalIdentity {
                actor_id: self.actor_id.clone(),
                canonical_since_tick: self.canonical_since_tick,
                last_exact_tick: self.last_exact_tick,
            });
        }
        Ok(())
    }
}

/// Monotonic owner-local revision of one civilization projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectionRevision(pub u64);

/// One scope's current identity-safe population projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CivilizationProjection {
    pub id: StableId,
    pub scope_id: StableId,
    pub revision: ProjectionRevision,
    pub canonical_tick: u64,
    cohorts: BTreeMap<StableId, PopulationCohort>,
    persistent_actors: BTreeMap<StableId, PersistentActorProjection>,
    committed_requests: BTreeMap<StableId, CommittedProjectionRequest>,
}

impl CivilizationProjection {
    pub fn new(
        id: StableId,
        scope_id: StableId,
        canonical_tick: u64,
        cohorts: impl IntoIterator<Item = PopulationCohort>,
        persistent_actors: impl IntoIterator<Item = PersistentActorProjection>,
    ) -> Result<Self, ScaleError> {
        let mut cohort_map = BTreeMap::new();
        for cohort in cohorts {
            cohort.validate()?;
            if cohort_map.insert(cohort.id.clone(), cohort).is_some() {
                return Err(ScaleError::DuplicateCohort);
            }
        }
        let mut actor_map = BTreeMap::new();
        for actor in persistent_actors {
            actor.validate()?;
            if actor.canonical_since_tick > canonical_tick {
                return Err(ScaleError::IdentityFromFuture(actor.actor_id));
            }
            if actor_map.insert(actor.actor_id.clone(), actor).is_some() {
                return Err(ScaleError::DuplicatePersistentActor);
            }
        }
        Ok(Self {
            id,
            scope_id,
            revision: ProjectionRevision(0),
            canonical_tick,
            cohorts: cohort_map,
            persistent_actors: actor_map,
            committed_requests: BTreeMap::new(),
        })
    }

    pub fn cohorts(&self) -> impl Iterator<Item = &PopulationCohort> {
        self.cohorts.values()
    }

    pub fn persistent_actors(&self) -> impl Iterator<Item = &PersistentActorProjection> {
        self.persistent_actors.values()
    }

    pub fn persistent_actor(&self, actor_id: &StableId) -> Option<&PersistentActorProjection> {
        self.persistent_actors.get(actor_id)
    }

    pub fn cohort(&self, cohort_id: &StableId) -> Option<&PopulationCohort> {
        self.cohorts.get(cohort_id)
    }

    /// Total represented people = anonymous population + retained persistent
    /// identities. Summarizing an existing persistent actor does not alter this.
    pub fn represented_population(&self) -> Result<u64, ScaleError> {
        let anonymous = self.cohorts.values().try_fold(0u64, |total, cohort| {
            total
                .checked_add(cohort.anonymous_count)
                .ok_or(ScaleError::PopulationOverflow)
        })?;
        anonymous
            .checked_add(
                u64::try_from(self.persistent_actors.len())
                    .map_err(|_| ScaleError::PopulationOverflow)?,
            )
            .ok_or(ScaleError::PopulationOverflow)
    }

    /// Applies one exact request against the expected revision. A retry with the
    /// same request id/content returns the prior result; conflicting id reuse fails.
    pub fn apply(
        &mut self,
        expected_revision: ProjectionRevision,
        request: ProjectionRequest,
    ) -> Result<ProjectionCommit, ScaleError> {
        if let Some(committed) = self.committed_requests.get(&request.id) {
            if committed.request == request {
                return Ok(committed.result.clone());
            }
            return Err(ScaleError::ConflictingRequestIdReuse(request.id));
        }
        if expected_revision != self.revision {
            return Err(ScaleError::StaleProjectionRevision {
                expected: self.revision,
                supplied: expected_revision,
            });
        }
        request.authority_ref.validate()?;
        if request.canonical_tick < self.canonical_tick {
            return Err(ScaleError::NonMonotonicProjectionTick {
                current_tick: self.canonical_tick,
                attempted_tick: request.canonical_tick,
            });
        }

        let population_before = self.represented_population()?;
        match &request.operation {
            ProjectionOperation::MaterializePersistentIdentity {
                cohort_id,
                actor_id,
                source_event_id,
            } => {
                if self.persistent_actors.contains_key(actor_id) {
                    return Err(ScaleError::PersistentActorAlreadyExists(actor_id.clone()));
                }
                let cohort = self
                    .cohorts
                    .get_mut(cohort_id)
                    .ok_or_else(|| ScaleError::UnknownCohort(cohort_id.clone()))?;
                if cohort.anonymous_count == 0 {
                    return Err(ScaleError::EmptyCohort(cohort_id.clone()));
                }
                cohort.anonymous_count -= 1;
                self.persistent_actors.insert(
                    actor_id.clone(),
                    PersistentActorProjection {
                        actor_id: actor_id.clone(),
                        // Critical theorem: identity begins now, not at cohort creation.
                        canonical_since_tick: request.canonical_tick,
                        representation: PersistentRepresentation::Exact,
                        last_exact_tick: request.canonical_tick,
                        summary_state_refs: BTreeSet::new(),
                        authority_ref: request.authority_ref.clone(),
                        source_event_id: source_event_id.clone(),
                    },
                );
            }
            ProjectionOperation::SummarizePersistentActor {
                actor_id,
                summary_state_refs,
                source_event_id,
            } => {
                let actor = self
                    .persistent_actors
                    .get_mut(actor_id)
                    .ok_or_else(|| ScaleError::UnknownPersistentActor(actor_id.clone()))?;
                if actor.representation == PersistentRepresentation::Summarized {
                    return Err(ScaleError::ActorAlreadySummarized(actor_id.clone()));
                }
                actor.representation = PersistentRepresentation::Summarized;
                actor.last_exact_tick = request.canonical_tick;
                actor.summary_state_refs = summary_state_refs.clone();
                actor.authority_ref = request.authority_ref.clone();
                actor.source_event_id = source_event_id.clone();
            }
            ProjectionOperation::ReexpandPersistentActor {
                actor_id,
                source_event_id,
            } => {
                let actor = self
                    .persistent_actors
                    .get_mut(actor_id)
                    .ok_or_else(|| ScaleError::UnknownPersistentActor(actor_id.clone()))?;
                if actor.representation == PersistentRepresentation::Exact {
                    return Err(ScaleError::ActorAlreadyExact(actor_id.clone()));
                }
                actor.representation = PersistentRepresentation::Exact;
                actor.last_exact_tick = request.canonical_tick;
                actor.authority_ref = request.authority_ref.clone();
                actor.source_event_id = source_event_id.clone();
                // Summary refs remain inspectable; re-expansion does not erase the
                // evidence used while the actor was coarse.
            }
        }

        let population_after = self.represented_population()?;
        if population_after != population_before {
            return Err(ScaleError::PopulationConservationViolation {
                before: population_before,
                after: population_after,
            });
        }

        self.revision = ProjectionRevision(
            self.revision
                .0
                .checked_add(1)
                .ok_or(ScaleError::RevisionOverflow)?,
        );
        self.canonical_tick = request.canonical_tick;
        let result = ProjectionCommit {
            request_id: request.id.clone(),
            resulting_revision: self.revision,
            canonical_tick: self.canonical_tick,
            represented_population: population_after,
            authority_ref: request.authority_ref.clone(),
        };
        self.committed_requests.insert(
            request.id.clone(),
            CommittedProjectionRequest {
                request,
                result: result.clone(),
            },
        );
        Ok(result)
    }
}

/// One revision-stamped projection request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRequest {
    pub id: StableId,
    pub canonical_tick: u64,
    pub authority_ref: ScaleAuthorityRef,
    pub operation: ProjectionOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionOperation {
    /// Materialize exactly one anonymous cohort member into a newly canonical
    /// persistent identity. No pre-materialization exact history is created.
    MaterializePersistentIdentity {
        cohort_id: StableId,
        actor_id: StableId,
        source_event_id: StableId,
    },
    /// Reduce simulation detail for an already-persistent identity without
    /// merging it back into anonymous population.
    SummarizePersistentActor {
        actor_id: StableId,
        summary_state_refs: BTreeSet<PersistentStateRef>,
        source_event_id: StableId,
    },
    /// Restore detailed simulation for the same persistent identity.
    ReexpandPersistentActor {
        actor_id: StableId,
        source_event_id: StableId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionCommit {
    pub request_id: StableId,
    pub resulting_revision: ProjectionRevision,
    pub canonical_tick: u64,
    pub represented_population: u64,
    pub authority_ref: ScaleAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CommittedProjectionRequest {
    request: ProjectionRequest,
    result: ProjectionCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScaleError {
    EmptyAuthorityNamespace,
    IdenticalSnapshotBoundary(StableId),
    EmptyCohortNamespace,
    ExactTickBeforeCanonicalIdentity {
        actor_id: StableId,
        canonical_since_tick: u64,
        last_exact_tick: u64,
    },
    DuplicateCohort,
    DuplicatePersistentActor,
    IdentityFromFuture(StableId),
    PopulationOverflow,
    ConflictingRequestIdReuse(StableId),
    StaleProjectionRevision {
        expected: ProjectionRevision,
        supplied: ProjectionRevision,
    },
    NonMonotonicProjectionTick {
        current_tick: u64,
        attempted_tick: u64,
    },
    PersistentActorAlreadyExists(StableId),
    UnknownCohort(StableId),
    EmptyCohort(StableId),
    UnknownPersistentActor(StableId),
    ActorAlreadySummarized(StableId),
    ActorAlreadyExact(StableId),
    PopulationConservationViolation { before: u64, after: u64 },
    RevisionOverflow,
}

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAuthorityNamespace => f.write_str("scale authority namespace is empty"),
            Self::IdenticalSnapshotBoundary(id) => write!(f, "scale authority uses identical source/destination snapshot {id}"),
            Self::EmptyCohortNamespace => f.write_str("cohort namespace is empty"),
            Self::ExactTickBeforeCanonicalIdentity { actor_id, .. } => write!(f, "actor {actor_id} has exact state before its canonical identity begins"),
            Self::DuplicateCohort => f.write_str("duplicate cohort identity"),
            Self::DuplicatePersistentActor => f.write_str("duplicate persistent actor identity"),
            Self::IdentityFromFuture(id) => write!(f, "persistent actor {id} begins after projection time"),
            Self::PopulationOverflow => f.write_str("represented population overflow"),
            Self::ConflictingRequestIdReuse(id) => write!(f, "projection request id {id} was reused with different content"),
            Self::StaleProjectionRevision { expected, supplied } => write!(f, "stale projection revision: expected {}, supplied {}", expected.0, supplied.0),
            Self::NonMonotonicProjectionTick { current_tick, attempted_tick } => write!(f, "projection tick moved backward from {current_tick} to {attempted_tick}"),
            Self::PersistentActorAlreadyExists(id) => write!(f, "persistent actor {id} already exists"),
            Self::UnknownCohort(id) => write!(f, "unknown population cohort {id}"),
            Self::EmptyCohort(id) => write!(f, "population cohort {id} has no anonymous members to materialize"),
            Self::UnknownPersistentActor(id) => write!(f, "unknown persistent actor {id}"),
            Self::ActorAlreadySummarized(id) => write!(f, "persistent actor {id} is already summarized"),
            Self::ActorAlreadyExact(id) => write!(f, "persistent actor {id} is already exact"),
            Self::PopulationConservationViolation { before, after } => write!(f, "projection population changed from {before} to {after}"),
            Self::RevisionOverflow => f.write_str("projection revision overflow"),
        }
    }
}

impl Error for ScaleError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn authority(label: &str) -> ScaleAuthorityRef {
        ScaleAuthorityRef {
            authority_namespace: "living-world-transition".into(),
            authority_record_id: id(&format!("authority:{label}")),
            source_snapshot_id: id(&format!("snapshot:{label}:source")),
            destination_snapshot_id: id(&format!("snapshot:{label}:destination")),
        }
    }

    fn cohort(count: u64) -> PopulationCohort {
        PopulationCohort {
            id: id("cohort:background"),
            descriptor: CohortDescriptor {
                namespace: "settlement-background".into(),
                dimensions: BTreeMap::from([
                    ("occupation".into(), "mixed".into()),
                    ("age-band".into(), "adult".into()),
                ]),
            },
            anonymous_count: count,
            authority_ref: authority("cohort"),
        }
    }

    #[test]
    fn materializing_identity_preserves_population_and_starts_history_now() {
        let mut projection = CivilizationProjection::new(
            id("projection:hub"),
            id("settlement:hub"),
            100,
            [cohort(10_000)],
            [],
        )
        .expect("projection");
        let before = projection.represented_population().expect("population");
        projection
            .apply(
                ProjectionRevision(0),
                ProjectionRequest {
                    id: id("request:materialize-mina"),
                    canonical_tick: 250,
                    authority_ref: authority("materialize-mina"),
                    operation: ProjectionOperation::MaterializePersistentIdentity {
                        cohort_id: id("cohort:background"),
                        actor_id: id("resident:mina"),
                        source_event_id: id("event:materialize-mina"),
                    },
                },
            )
            .expect("materialize");
        let actor = projection
            .persistent_actor(&id("resident:mina"))
            .expect("persistent actor");
        assert_eq!(actor.canonical_since_tick, 250);
        assert_eq!(actor.last_exact_tick, 250);
        assert_eq!(projection.cohort(&id("cohort:background")).unwrap().anonymous_count, 9_999);
        assert_eq!(projection.represented_population().unwrap(), before);
    }

    #[test]
    fn summarization_retains_identity_and_reexpansion_reuses_it() {
        let actor = PersistentActorProjection {
            actor_id: id("resident:mara"),
            canonical_since_tick: 10,
            representation: PersistentRepresentation::Exact,
            last_exact_tick: 100,
            summary_state_refs: BTreeSet::new(),
            authority_ref: authority("bootstrap-mara"),
            source_event_id: id("event:bootstrap-mara"),
        };
        let mut projection = CivilizationProjection::new(
            id("projection:capital"),
            id("settlement:capital"),
            100,
            [cohort(500)],
            [actor],
        )
        .expect("projection");
        let population = projection.represented_population().unwrap();
        projection
            .apply(
                ProjectionRevision(0),
                ProjectionRequest {
                    id: id("request:summarize-mara"),
                    canonical_tick: 200,
                    authority_ref: authority("summarize-mara"),
                    operation: ProjectionOperation::SummarizePersistentActor {
                        actor_id: id("resident:mara"),
                        summary_state_refs: BTreeSet::from([PersistentStateRef {
                            namespace: "resident-summary".into(),
                            record_id: id("summary:mara:200"),
                        }]),
                        source_event_id: id("event:summarize-mara"),
                    },
                },
            )
            .expect("summarize");
        assert_eq!(
            projection.persistent_actor(&id("resident:mara")).unwrap().representation,
            PersistentRepresentation::Summarized
        );
        projection
            .apply(
                ProjectionRevision(1),
                ProjectionRequest {
                    id: id("request:reexpand-mara"),
                    canonical_tick: 300,
                    authority_ref: authority("reexpand-mara"),
                    operation: ProjectionOperation::ReexpandPersistentActor {
                        actor_id: id("resident:mara"),
                        source_event_id: id("event:reexpand-mara"),
                    },
                },
            )
            .expect("reexpand");
        let mara = projection.persistent_actor(&id("resident:mara")).unwrap();
        assert_eq!(mara.actor_id, id("resident:mara"));
        assert_eq!(mara.canonical_since_tick, 10);
        assert_eq!(mara.representation, PersistentRepresentation::Exact);
        assert_eq!(projection.represented_population().unwrap(), population);
    }

    #[test]
    fn same_materialization_request_is_idempotent_but_conflicting_reuse_fails() {
        let mut projection = CivilizationProjection::new(
            id("projection:hub"),
            id("settlement:hub"),
            10,
            [cohort(2)],
            [],
        )
        .expect("projection");
        let request = ProjectionRequest {
            id: id("request:one"),
            canonical_tick: 20,
            authority_ref: authority("one"),
            operation: ProjectionOperation::MaterializePersistentIdentity {
                cohort_id: id("cohort:background"),
                actor_id: id("resident:one"),
                source_event_id: id("event:one"),
            },
        };
        let first = projection
            .apply(ProjectionRevision(0), request.clone())
            .expect("first");
        let retry = projection
            .apply(ProjectionRevision(0), request)
            .expect("retry returns committed result");
        assert_eq!(first, retry);
        let conflicting = projection.apply(
            ProjectionRevision(1),
            ProjectionRequest {
                id: id("request:one"),
                canonical_tick: 30,
                authority_ref: authority("two"),
                operation: ProjectionOperation::MaterializePersistentIdentity {
                    cohort_id: id("cohort:background"),
                    actor_id: id("resident:two"),
                    source_event_id: id("event:two"),
                },
            },
        );
        assert!(matches!(conflicting, Err(ScaleError::ConflictingRequestIdReuse(_))));
    }

    #[test]
    fn exhausted_cohort_cannot_fabricate_another_identity() {
        let mut projection = CivilizationProjection::new(
            id("projection:small"),
            id("settlement:small"),
            0,
            [cohort(1)],
            [],
        )
        .expect("projection");
        for (revision, actor, tick) in [
            (ProjectionRevision(0), "resident:first", 1),
            (ProjectionRevision(1), "resident:impossible", 2),
        ] {
            let result = projection.apply(
                revision,
                ProjectionRequest {
                    id: id(&format!("request:{actor}")),
                    canonical_tick: tick,
                    authority_ref: authority(actor),
                    operation: ProjectionOperation::MaterializePersistentIdentity {
                        cohort_id: id("cohort:background"),
                        actor_id: id(actor),
                        source_event_id: id(&format!("event:{actor}")),
                    },
                },
            );
            if actor == "resident:first" {
                result.expect("first materialization");
            } else {
                assert!(matches!(result, Err(ScaleError::EmptyCohort(_))));
            }
        }
    }
}

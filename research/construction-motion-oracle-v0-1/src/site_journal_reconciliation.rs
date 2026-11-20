//! Deterministic reconciliation of offline construction-site event journals.
//!
//! A field site can lose network connectivity while robots, surveyors and
//! referees continue recording evidence. This module validates actor-owned
//! local hash chains, combines their explicit causal dependencies, detects
//! concurrent writes to the same construction resource, and emits a stable
//! topological import order.
//!
//! It deliberately does not choose a winner for concurrent mutations and does
//! not assign authoritative site-ledger sequence numbers. If a conflict exists,
//! the plan is not ingestable until an authorized human/system disposition is
//! recorded. The caller must verify canonical bytes, hashes and signatures
//! before passing journals to this planner.

use std::collections::{BTreeMap, BTreeSet};

use crate::site_state_ledger::{SiteActorRegistry, SiteActorRole, SiteEventPayload};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteJournalCheckpoint {
    /// Authoritative site-ledger sequence at the moment this offline journal
    /// was forked. All journals in one reconciliation bundle must share it.
    pub sequence: u64,
    pub event_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OfflineSiteJournal {
    pub journal_id: String,
    pub site_id: String,
    /// One authenticated actor owns a local journal. Cross-actor events belong
    /// in another journal and are linked through explicit causal parents.
    pub owner_actor_id: String,
    pub base_checkpoint: SiteJournalCheckpoint,
    pub entries: Vec<OfflineSiteJournalEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OfflineSiteJournalEntry {
    /// Local sequence, starting at 1 and increasing without gaps.
    pub local_sequence: u64,
    /// Hash-chain link over this actor's canonical local journal records.
    pub previous_record_sha256: Option<String>,
    /// Digest of the canonical local journal record; verified by the caller.
    pub record_sha256: String,
    /// Stable logical event identity shared by replicated copies.
    pub event_id: String,
    /// Digest of immutable event content (actor, role and payload), excluding
    /// the local journal position. Replicated copies must match this digest.
    pub event_content_sha256: String,
    pub actor_id: String,
    pub actor_role: SiteActorRole,
    /// Explicit cross-journal dependencies. Same-journal predecessor ordering
    /// is added automatically from local_sequence / previous_record_sha256.
    pub causal_parent_event_ids: Vec<String>,
    pub payload: SiteEventPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SiteMutationKey {
    /// Two offline work-package publications compete to become current.
    ActiveWorkPackage,
    /// Station calibration/qualification writes to the same named station.
    Station {
        package_sha256: String,
        station_id: String,
    },
    /// Two attempts or a waiver compete for one design-intent entity.
    Entity {
        package_sha256: String,
        entity_id: String,
    },
    /// Two observations/interruption/referee dispositions compete for one attempt.
    Attempt {
        package_sha256: String,
        attempt_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcurrentSiteMutation {
    pub key: SiteMutationKey,
    /// Deterministically sorted logical event IDs; neither event is selected.
    pub event_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReconciledSiteJournalEvent {
    pub event_id: String,
    pub event_content_sha256: String,
    pub actor_id: String,
    pub actor_role: SiteActorRole,
    pub causal_parent_event_ids: Vec<String>,
    pub origin_journal_ids: Vec<String>,
    pub payload: SiteEventPayload,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteJournalCommittedContext {
    /// Logical event IDs at or before the common checkpoint.
    pub event_ids: BTreeSet<String>,
    /// Work-package digests already published at the checkpoint.
    pub package_sha256s: BTreeSet<String>,
    /// Station-qualification event IDs already committed at the checkpoint.
    pub station_qualification_event_ids: BTreeSet<String>,
    /// Execution attempt IDs already started at the checkpoint.
    pub attempt_ids: BTreeSet<String>,
    /// Attempt IDs with a robot observation/interruption already committed.
    pub observed_attempt_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SiteJournalReconciliationPlan {
    pub site_id: String,
    pub base_checkpoint: SiteJournalCheckpoint,
    /// Stable topological order. This is a candidate import order only; it is
    /// not a site-ledger sequence and does not by itself authorize ingestion.
    pub ordered_events: Vec<ReconciledSiteJournalEvent>,
    /// Concurrent mutations remain unresolved. Any non-empty list blocks the
    /// complete bundle from being committed as an authoritative site history.
    pub concurrent_mutations: Vec<ConcurrentSiteMutation>,
    pub duplicate_replicas_collapsed: u64,
}

impl SiteJournalReconciliationPlan {
    pub fn can_ingest(&self) -> bool {
        self.concurrent_mutations.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteJournalReconciliationError {
    EmptyBundle,
    EmptyExpectedSiteId,
    EmptyJournalId,
    EmptyJournalOwner,
    DuplicateJournalId(String),
    SiteIdMismatch {
        journal_id: String,
        expected: String,
        actual: String,
    },
    InvalidCheckpoint,
    StaleJournalCheckpoint(String),
    EmptyJournal,
    InvalidLocalSequence {
        journal_id: String,
        expected: u64,
        actual: u64,
    },
    EmptyEventIdentity(String),
    InvalidRecordDigest(String),
    InvalidEventContentDigest(String),
    InvalidPreviousRecordDigest(String),
    LocalRecordChainMismatch {
        journal_id: String,
        local_sequence: u64,
    },
    DuplicateLocalRecordDigest(String),
    JournalActorMismatch {
        journal_id: String,
        event_id: String,
    },
    ActorRoleDenied {
        actor_id: String,
        role: SiteActorRole,
    },
    DuplicateCausalParent {
        event_id: String,
        parent_event_id: String,
    },
    SelfCausalParent(String),
    MissingCausalParent {
        event_id: String,
        parent_event_id: String,
    },
    MissingWorkPackage {
        event_id: String,
        package_sha256: String,
    },
    MissingStationQualification {
        event_id: String,
        station_qualification_event_id: String,
    },
    MissingAttemptStart {
        event_id: String,
        attempt_id: String,
    },
    MissingPriorRobotObservation {
        event_id: String,
        attempt_id: String,
    },
    AttemptAlreadyCommitted(String),
    AttemptAlreadyObserved(String),
    DuplicateAttemptStart(String),
    DuplicateEventWithinJournal(String),
    EventAlreadyCommitted(String),
    ConflictingEventId(String),
    ConflictingEventContentDigest(String),
    DuplicatePublishedPackageDigest(String),
    InvalidCommittedContext,
    CausalCycle,
}

#[derive(Clone, Debug)]
struct EventAccumulator {
    event_id: String,
    event_content_sha256: String,
    actor_id: String,
    actor_role: SiteActorRole,
    causal_parents: BTreeSet<String>,
    origin_journal_ids: BTreeSet<String>,
    payload: SiteEventPayload,
}

/// Reconcile actor-owned journals forked from the same authoritative checkpoint.
///
/// The committed-event set contains logical IDs already present at or before
/// the common checkpoint. Parents in this set terminate at the checkpoint and
/// need not appear in the import bundle. Parents created after the checkpoint
/// must be present in the bundle.
pub fn reconcile_site_journal_bundle(
    journals: &[OfflineSiteJournal],
    expected_site_id: &str,
    expected_checkpoint: &SiteJournalCheckpoint,
    committed_context: &SiteJournalCommittedContext,
    actor_registry: &SiteActorRegistry,
) -> Result<SiteJournalReconciliationPlan, SiteJournalReconciliationError> {
    if journals.is_empty() {
        return Err(SiteJournalReconciliationError::EmptyBundle);
    }
    if expected_site_id.trim().is_empty() {
        return Err(SiteJournalReconciliationError::EmptyExpectedSiteId);
    }
    validate_checkpoint(expected_checkpoint)?;
    if !committed_context
        .station_qualification_event_ids
        .is_subset(&committed_context.event_ids)
        || !committed_context
            .observed_attempt_ids
            .is_subset(&committed_context.attempt_ids)
    {
        return Err(SiteJournalReconciliationError::InvalidCommittedContext);
    }

    let mut journal_ids = BTreeSet::new();
    let mut events = BTreeMap::<String, EventAccumulator>::new();
    let mut duplicate_replicas_collapsed = 0_u64;

    for journal in journals {
        if journal.journal_id.trim().is_empty() {
            return Err(SiteJournalReconciliationError::EmptyJournalId);
        }
        if !journal_ids.insert(journal.journal_id.as_str()) {
            return Err(SiteJournalReconciliationError::DuplicateJournalId(
                journal.journal_id.clone(),
            ));
        }
        if journal.owner_actor_id.trim().is_empty() {
            return Err(SiteJournalReconciliationError::EmptyJournalOwner);
        }
        if journal.site_id != expected_site_id {
            return Err(SiteJournalReconciliationError::SiteIdMismatch {
                journal_id: journal.journal_id.clone(),
                expected: expected_site_id.to_owned(),
                actual: journal.site_id.clone(),
            });
        }
        if &journal.base_checkpoint != expected_checkpoint {
            return Err(SiteJournalReconciliationError::StaleJournalCheckpoint(
                journal.journal_id.clone(),
            ));
        }
        if journal.entries.is_empty() {
            return Err(SiteJournalReconciliationError::EmptyJournal);
        }

        let mut previous_record_sha256: Option<&str> = None;
        let mut previous_event_id: Option<String> = None;
        let mut local_ids = BTreeSet::new();
        let mut local_record_digests = BTreeSet::new();

        for (index, entry) in journal.entries.iter().enumerate() {
            let expected_sequence = (index as u64)
                .checked_add(1)
                .ok_or_else(|| SiteJournalReconciliationError::InvalidLocalSequence {
                    journal_id: journal.journal_id.clone(),
                    expected: u64::MAX,
                    actual: entry.local_sequence,
                })?;
            if entry.local_sequence != expected_sequence {
                return Err(SiteJournalReconciliationError::InvalidLocalSequence {
                    journal_id: journal.journal_id.clone(),
                    expected: expected_sequence,
                    actual: entry.local_sequence,
                });
            }
            if entry.event_id.trim().is_empty() || entry.actor_id.trim().is_empty() {
                return Err(SiteJournalReconciliationError::EmptyEventIdentity(
                    journal.journal_id.clone(),
                ));
            }
            if committed_event_ids.contains(&entry.event_id) {
                return Err(SiteJournalReconciliationError::EventAlreadyCommitted(
                    entry.event_id.clone(),
                ));
            }
            if !is_sha256(&entry.record_sha256) {
                return Err(SiteJournalReconciliationError::InvalidRecordDigest(
                    entry.event_id.clone(),
                ));
            }
            if !is_sha256(&entry.event_content_sha256) {
                return Err(SiteJournalReconciliationError::InvalidEventContentDigest(
                    entry.event_id.clone(),
                ));
            }
            if entry
                .previous_record_sha256
                .as_ref()
                .is_some_and(|digest| !is_sha256(digest))
            {
                return Err(
                    SiteJournalReconciliationError::InvalidPreviousRecordDigest(
                        entry.event_id.clone(),
                    ),
                );
            }
            if entry.previous_record_sha256.as_deref() != previous_record_sha256 {
                return Err(
                    SiteJournalReconciliationError::LocalRecordChainMismatch {
                        journal_id: journal.journal_id.clone(),
                        local_sequence: entry.local_sequence,
                    },
                );
            }
            if !local_record_digests.insert(entry.record_sha256.as_str()) {
                return Err(SiteJournalReconciliationError::DuplicateLocalRecordDigest(
                    entry.record_sha256.clone(),
                ));
            }
            if entry.actor_id != journal.owner_actor_id {
                return Err(SiteJournalReconciliationError::JournalActorMismatch {
                    journal_id: journal.journal_id.clone(),
                    event_id: entry.event_id.clone(),
                });
            }
            if !actor_registry.allows(&entry.actor_id, entry.actor_role) {
                return Err(SiteJournalReconciliationError::ActorRoleDenied {
                    actor_id: entry.actor_id.clone(),
                    role: entry.actor_role,
                });
            }
            if !local_ids.insert(entry.event_id.as_str()) {
                return Err(
                    SiteJournalReconciliationError::DuplicateEventWithinJournal(
                        entry.event_id.clone(),
                    ),
                );
            }

            let mut parents = BTreeSet::new();
            if let Some(previous_id) = previous_event_id.as_ref() {
                parents.insert(previous_id.clone());
            }
            for parent_id in &entry.causal_parent_event_ids {
                if parent_id == &entry.event_id {
                    return Err(SiteJournalReconciliationError::SelfCausalParent(
                        entry.event_id.clone(),
                    ));
                }
                if !parents.insert(parent_id.clone()) {
                    return Err(
                        SiteJournalReconciliationError::DuplicateCausalParent {
                            event_id: entry.event_id.clone(),
                            parent_event_id: parent_id.clone(),
                        },
                    );
                }
            }

            match events.get_mut(&entry.event_id) {
                Some(existing) => {
                    if existing.event_content_sha256 != entry.event_content_sha256
                        || existing.actor_id != entry.actor_id
                        || existing.actor_role != entry.actor_role
                        || existing.payload != entry.payload
                    {
                        return Err(SiteJournalReconciliationError::ConflictingEventId(
                            entry.event_id.clone(),
                        ));
                    }
                    existing.causal_parents.extend(parents);
                    existing
                        .origin_journal_ids
                        .insert(journal.journal_id.clone());
                    duplicate_replicas_collapsed = duplicate_replicas_collapsed
                        .checked_add(1)
                        .ok_or_else(|| {
                            SiteJournalReconciliationError::ConflictingEventContentDigest(
                                entry.event_content_sha256.clone(),
                            )
                        })?;
                }
                None => {
                    events.insert(
                        entry.event_id.clone(),
                        EventAccumulator {
                            event_id: entry.event_id.clone(),
                            event_content_sha256: entry.event_content_sha256.clone(),
                            actor_id: entry.actor_id.clone(),
                            actor_role: entry.actor_role,
                            causal_parents: parents,
                            origin_journal_ids: BTreeSet::from([journal.journal_id.clone()]),
                            payload: entry.payload.clone(),
                        },
                    );
                }
            }

            previous_record_sha256 = Some(entry.record_sha256.as_str());
            previous_event_id = Some(entry.event_id.clone());
        }
    }

    // Add domain-derived causal edges for references already explicit in the
    // payload. Adapters still supply cross-event causal parents that have no
    // typed reference in SiteEventPayload.
    add_payload_dependencies(&mut events, committed_context)?;

    // A logical event ID and its immutable content digest are one-to-one.
    let mut event_ids_by_digest = BTreeMap::<&str, &str>::new();
    for event in events.values() {
        if let Some(existing_event_id) =
            event_ids_by_digest.insert(&event.event_content_sha256, &event.event_id)
        {
            if existing_event_id != event.event_id {
                return Err(
                    SiteJournalReconciliationError::ConflictingEventContentDigest(
                        event.event_content_sha256.clone(),
                    ),
                );
            }
        }
    }

    for event in events.values() {
        for parent_id in &event.causal_parents {
            if !events.contains_key(parent_id) && !committed_context.event_ids.contains(parent_id) {
                return Err(SiteJournalReconciliationError::MissingCausalParent {
                    event_id: event.event_id.clone(),
                    parent_event_id: parent_id.clone(),
                });
            }
        }
    }

    let (topological_event_ids, ancestors) = topological_order_and_ancestors(&events)?;
    let conflicts = find_concurrent_mutations(&events, &topological_event_ids, &ancestors);

    let ordered_events = topological_event_ids
        .iter()
        .map(|event_id| {
            let event = events
                .get(event_id)
                .expect("topological order contains only known event IDs");
            ReconciledSiteJournalEvent {
                event_id: event.event_id.clone(),
                event_content_sha256: event.event_content_sha256.clone(),
                actor_id: event.actor_id.clone(),
                actor_role: event.actor_role,
                causal_parent_event_ids: event.causal_parents.iter().cloned().collect(),
                origin_journal_ids: event.origin_journal_ids.iter().cloned().collect(),
                payload: event.payload.clone(),
            }
        })
        .collect();

    Ok(SiteJournalReconciliationPlan {
        site_id: expected_site_id.to_owned(),
        base_checkpoint: expected_checkpoint.clone(),
        ordered_events,
        concurrent_mutations: conflicts,
        duplicate_replicas_collapsed,
    })
}


fn add_payload_dependencies(
    events: &mut BTreeMap<String, EventAccumulator>,
    committed_context: &SiteJournalCommittedContext,
) -> Result<(), SiteJournalReconciliationError> {
    let mut station_events = BTreeSet::new();
    let mut attempt_starts = BTreeMap::<String, String>::new();
    let mut attempt_observations = BTreeMap::<String, BTreeSet<String>>::new();
    let mut package_publications = BTreeMap::<String, String>::new();

    for event in events.values() {
        match &event.payload {
            SiteEventPayload::WorkPackagePublished { package } => {
                if package_publications
                    .insert(package.package_sha256.clone(), event.event_id.clone())
                    .is_some()
                {
                    return Err(
                        SiteJournalReconciliationError::DuplicatePublishedPackageDigest(
                            package.package_sha256.clone(),
                        ),
                    );
                }
            }
            SiteEventPayload::StationQualified { .. } => {
                station_events.insert(event.event_id.clone());
            }
            SiteEventPayload::ExecutionStarted {
                package_sha256,
                attempt_id,
                ..
            } => {
                if committed_context.attempt_ids.contains(attempt_id) {
                    return Err(SiteJournalReconciliationError::AttemptAlreadyCommitted(
                        attempt_id.clone(),
                    ));
                }
                if attempt_starts
                    .insert(attempt_id.clone(), event.event_id.clone())
                    .is_some()
                {
                    return Err(SiteJournalReconciliationError::DuplicateAttemptStart(
                        attempt_id.clone(),
                    ));
                }
                if committed_context.package_sha256s.contains(package_sha256)
                    || package_publications.contains_key(package_sha256)
                {
                    // Package references are checked again below after all
                    // package publication events have been collected.
                }
            }
            SiteEventPayload::RobotObservationRecorded { attempt_id, .. }
            | SiteEventPayload::ExecutionInterrupted { attempt_id, .. } => {
                attempt_observations
                    .entry(attempt_id.clone())
                    .or_default()
                    .insert(event.event_id.clone());
            }
            SiteEventPayload::RefereeAssessmentRecorded { .. }
            | SiteEventPayload::HumanWaiverIssued { .. } => {}
        }
    }

    let event_ids: Vec<String> = events.keys().cloned().collect();
    for event_id in event_ids {
        let payload = events
            .get(&event_id)
            .expect("event ID is taken from the event map")
            .payload
            .clone();

        let mut required_parents = BTreeSet::new();
        match &payload {
            SiteEventPayload::WorkPackagePublished { .. } => {}
            SiteEventPayload::StationQualified { package_sha256, .. }
            | SiteEventPayload::ExecutionStarted { package_sha256, .. }
            | SiteEventPayload::RobotObservationRecorded { package_sha256, .. }
            | SiteEventPayload::ExecutionInterrupted { package_sha256, .. }
            | SiteEventPayload::RefereeAssessmentRecorded { package_sha256, .. }
            | SiteEventPayload::HumanWaiverIssued { package_sha256, .. } => {
                if let Some(package_event_id) = package_publications.get(package_sha256) {
                    if package_event_id != &event_id {
                        required_parents.insert(package_event_id.clone());
                    }
                } else if !committed_context.package_sha256s.contains(package_sha256) {
                    return Err(SiteJournalReconciliationError::MissingWorkPackage {
                        event_id: event_id.clone(),
                        package_sha256: package_sha256.clone(),
                    });
                }
            }
        }

        match &payload {
            SiteEventPayload::ExecutionStarted {
                station_qualification_event_id,
                ..
            } => {
                if station_events.contains(station_qualification_event_id) {
                    required_parents.insert(station_qualification_event_id.clone());
                } else if !committed_context
                    .station_qualification_event_ids
                    .contains(station_qualification_event_id)
                {
                    return Err(
                        SiteJournalReconciliationError::MissingStationQualification {
                            event_id: event_id.clone(),
                            station_qualification_event_id: station_qualification_event_id.clone(),
                        },
                    );
                }
            }
            SiteEventPayload::RobotObservationRecorded { attempt_id, .. }
            | SiteEventPayload::ExecutionInterrupted { attempt_id, .. } => {
                if let Some(start_event_id) = attempt_starts.get(attempt_id) {
                    if start_event_id != &event_id {
                        required_parents.insert(start_event_id.clone());
                    }
                } else if !committed_context.attempt_ids.contains(attempt_id) {
                    return Err(SiteJournalReconciliationError::MissingAttemptStart {
                        event_id: event_id.clone(),
                        attempt_id: attempt_id.clone(),
                    });
                }
                if matches!(&payload, SiteEventPayload::RobotObservationRecorded { .. })
                    && committed_context.observed_attempt_ids.contains(attempt_id)
                {
                    return Err(SiteJournalReconciliationError::AttemptAlreadyObserved(
                        attempt_id.clone(),
                    ));
                }
            }
            SiteEventPayload::RefereeAssessmentRecorded { attempt_id, .. } => {
                if let Some(start_event_id) = attempt_starts.get(attempt_id) {
                    if start_event_id != &event_id {
                        required_parents.insert(start_event_id.clone());
                    }
                } else if !committed_context.attempt_ids.contains(attempt_id) {
                    return Err(SiteJournalReconciliationError::MissingAttemptStart {
                        event_id: event_id.clone(),
                        attempt_id: attempt_id.clone(),
                    });
                }
                if let Some(observation_event_ids) = attempt_observations.get(attempt_id) {
                    required_parents.extend(
                        observation_event_ids
                            .iter()
                            .filter(|observation_event_id| *observation_event_id != &event_id)
                            .cloned(),
                    );
                } else if !committed_context
                    .observed_attempt_ids
                    .contains(attempt_id)
                {
                    return Err(
                        SiteJournalReconciliationError::MissingPriorRobotObservation {
                            event_id: event_id.clone(),
                            attempt_id: attempt_id.clone(),
                        },
                    );
                }
            }
            SiteEventPayload::WorkPackagePublished { .. }
            | SiteEventPayload::StationQualified { .. }
            | SiteEventPayload::HumanWaiverIssued { .. } => {}
        }

        if let Some(event) = events.get_mut(&event_id) {
            event.causal_parents.extend(required_parents);
        }
    }
    Ok(())
}

fn validate_checkpoint(
    checkpoint: &SiteJournalCheckpoint,
) -> Result<(), SiteJournalReconciliationError> {
    match (checkpoint.sequence, checkpoint.event_sha256.as_deref()) {
        (0, None) => Ok(()),
        (0, Some(_)) | (_, None) => Err(SiteJournalReconciliationError::InvalidCheckpoint),
        (_, Some(digest)) if !is_sha256(digest) => {
            Err(SiteJournalReconciliationError::InvalidCheckpoint)
        }
        _ => Ok(()),
    }
}

fn topological_order_and_ancestors(
    events: &BTreeMap<String, EventAccumulator>,
) -> Result<(Vec<String>, BTreeMap<String, BTreeSet<String>>), SiteJournalReconciliationError> {
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut children = BTreeMap::<String, BTreeSet<String>>::new();

    for event in events.values() {
        let in_bundle_parents = event
            .causal_parents
            .iter()
            .filter(|parent| events.contains_key(parent.as_str()))
            .count();
        indegree.insert(event.event_id.clone(), in_bundle_parents);
        for parent_id in &event.causal_parents {
            if events.contains_key(parent_id) {
                children
                    .entry(parent_id.clone())
                    .or_default()
                    .insert(event.event_id.clone());
            }
        }
    }

    // BTreeSet makes the order deterministic when multiple events are causally
    // independent. This tie-break does not resolve semantic write conflicts.
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(event_id, degree)| (*degree == 0).then_some(event_id.clone()))
        .collect();
    let mut ordered = Vec::with_capacity(events.len());
    let mut ancestors = BTreeMap::<String, BTreeSet<String>>::new();

    while let Some(event_id) = ready.pop_first() {
        let event = events
            .get(&event_id)
            .expect("ready event IDs are drawn from the event map");
        let mut event_ancestors = BTreeSet::new();
        for parent_id in &event.causal_parents {
            if events.contains_key(parent_id) {
                event_ancestors.insert(parent_id.clone());
                if let Some(parent_ancestors) = ancestors.get(parent_id) {
                    event_ancestors.extend(parent_ancestors.iter().cloned());
                }
            }
        }
        ancestors.insert(event_id.clone(), event_ancestors);
        ordered.push(event_id.clone());

        if let Some(event_children) = children.get(&event_id) {
            for child_id in event_children {
                let degree = indegree
                    .get_mut(child_id)
                    .expect("every child is present in indegree map");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child_id.clone());
                }
            }
        }
    }

    if ordered.len() != events.len() {
        return Err(SiteJournalReconciliationError::CausalCycle);
    }
    Ok((ordered, ancestors))
}

fn find_concurrent_mutations(
    events: &BTreeMap<String, EventAccumulator>,
    topological_order: &[String],
    ancestors: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<ConcurrentSiteMutation> {
    let mut occurrences = Vec::<(String, SiteMutationKey)>::new();
    for event_id in topological_order {
        if let Some(event) = events.get(event_id) {
            for key in mutation_keys(&event.payload) {
                occurrences.push((event.event_id.clone(), key));
            }
        }
    }

    let mut conflicts = Vec::new();
    for first_index in 0..occurrences.len() {
        for second_index in (first_index + 1)..occurrences.len() {
            let (first_id, first_key) = &occurrences[first_index];
            let (second_id, second_key) = &occurrences[second_index];
            if first_key != second_key {
                continue;
            }
            let first_before_second = ancestors
                .get(second_id)
                .is_some_and(|items| items.contains(first_id));
            let second_before_first = ancestors
                .get(first_id)
                .is_some_and(|items| items.contains(second_id));
            if !first_before_second && !second_before_first {
                let mut event_ids = vec![first_id.clone(), second_id.clone()];
                event_ids.sort();
                conflicts.push(ConcurrentSiteMutation {
                    key: first_key.clone(),
                    event_ids,
                });
            }
        }
    }
    conflicts.sort_by(|left, right| {
        left.key
            .cmp(&right.key)
            .then_with(|| left.event_ids.cmp(&right.event_ids))
    });
    conflicts
}

fn mutation_keys(payload: &SiteEventPayload) -> Vec<SiteMutationKey> {
    match payload {
        SiteEventPayload::WorkPackagePublished { .. } => {
            vec![SiteMutationKey::ActiveWorkPackage]
        }
        SiteEventPayload::StationQualified {
            package_sha256,
            station_id,
            ..
        } => vec![SiteMutationKey::Station {
            package_sha256: package_sha256.clone(),
            station_id: station_id.clone(),
        }],
        SiteEventPayload::ExecutionStarted {
            package_sha256,
            entity_id,
            attempt_id,
            ..
        }
        | SiteEventPayload::RobotObservationRecorded {
            package_sha256,
            entity_id,
            attempt_id,
            ..
        }
        | SiteEventPayload::ExecutionInterrupted {
            package_sha256,
            entity_id,
            attempt_id,
            ..
        }
        | SiteEventPayload::RefereeAssessmentRecorded {
            package_sha256,
            entity_id,
            attempt_id,
            ..
        } => vec![
            SiteMutationKey::Entity {
                package_sha256: package_sha256.clone(),
                entity_id: entity_id.clone(),
            },
            SiteMutationKey::Attempt {
                package_sha256: package_sha256.clone(),
                attempt_id: attempt_id.clone(),
            },
        ],
        SiteEventPayload::HumanWaiverIssued {
            package_sha256,
            waiver,
        } => vec![SiteMutationKey::Entity {
            package_sha256: package_sha256.clone(),
            entity_id: waiver.design_entity_id.clone(),
        }],
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn checkpoint() -> SiteJournalCheckpoint {
        SiteJournalCheckpoint {
            sequence: 0,
            event_sha256: None,
        }
    }

    fn registry() -> SiteActorRegistry {
        let mut registry = SiteActorRegistry::new();
        registry.grant("robot-a", SiteActorRole::Robot).unwrap();
        registry.grant("robot-b", SiteActorRole::Robot).unwrap();
        registry.grant("referee-a", SiteActorRole::Referee).unwrap();
        registry.grant("surveyor-a", SiteActorRole::Surveyor).unwrap();
        registry
    }

    fn observation_payload(attempt_id: &str) -> SiteEventPayload {
        SiteEventPayload::RobotObservationRecorded {
            package_sha256: digest('a'),
            entity_id: "entity-1".into(),
            attempt_id: attempt_id.into(),
            disposition: crate::site_state_ledger::RobotExecutionDisposition::Printed,
            robot_receipt_sha256: digest('b'),
        }
    }

    fn interruption_payload(attempt_id: &str) -> SiteEventPayload {
        SiteEventPayload::ExecutionInterrupted {
            package_sha256: digest('a'),
            entity_id: "entity-1".into(),
            attempt_id: attempt_id.into(),
            reason: "connectivity lost".into(),
            interruption_receipt_sha256: digest('c'),
        }
    }

    fn entry(
        local_sequence: u64,
        previous: Option<char>,
        event_id: &str,
        content_digest: char,
        actor_id: &str,
        role: SiteActorRole,
        parents: &[&str],
        payload: SiteEventPayload,
    ) -> OfflineSiteJournalEntry {
        OfflineSiteJournalEntry {
            local_sequence,
            previous_record_sha256: previous.map(digest),
            record_sha256: digest(char::from_digit(local_sequence as u32 + 1, 16).unwrap()),
            event_id: event_id.into(),
            event_content_sha256: digest(content_digest),
            actor_id: actor_id.into(),
            actor_role: role,
            causal_parent_event_ids: parents.iter().map(|value| (*value).into()).collect(),
            payload,
        }
    }

    fn journal(
        journal_id: &str,
        owner: &str,
        entries: Vec<OfflineSiteJournalEntry>,
    ) -> OfflineSiteJournal {
        OfflineSiteJournal {
            journal_id: journal_id.into(),
            site_id: "site-001".into(),
            owner_actor_id: owner.into(),
            base_checkpoint: checkpoint(),
            entries,
        }
    }

    fn reconcile(
        journals: &[OfflineSiteJournal],
    ) -> Result<SiteJournalReconciliationPlan, SiteJournalReconciliationError> {
        reconcile_site_journal_bundle(
            journals,
            "site-001",
            &checkpoint(),
            &SiteJournalCommittedContext {
                package_sha256s: BTreeSet::from([digest('a')]),
                attempt_ids: BTreeSet::from([
                    "attempt-a".to_owned(),
                    "attempt-b".to_owned(),
                ]),
                station_qualification_event_ids: BTreeSet::from(["station-1".to_owned()]),
                ..SiteJournalCommittedContext::default()
            },
            &registry(),
        )
    }

    #[test]
    fn independent_writes_get_a_stable_order_without_conflict() {
        let first = journal(
            "journal-robot-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "event-a",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        let second = journal(
            "journal-robot-b",
            "robot-b",
            vec![entry(
                1,
                None,
                "event-b",
                '2',
                "robot-b",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-b"),
            )],
        );
        let forward = reconcile(&[first.clone(), second.clone()]).unwrap();
        let reverse = reconcile(&[second, first]).unwrap();
        assert!(forward.can_ingest());
        assert!(reverse.can_ingest());
        assert_eq!(
            forward
                .ordered_events
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>(),
            reverse
                .ordered_events
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn uncaused_same_entity_attempts_are_reported_not_arbitrarily_ordered() {
        let first = journal(
            "journal-robot-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "start-a",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                SiteEventPayload::ExecutionStarted {
                    package_sha256: digest('a'),
                    entity_id: "entity-1".into(),
                    attempt_id: "attempt-a".into(),
                    station_qualification_event_id: "station-1".into(),
                },
            )],
        );
        let second = journal(
            "journal-robot-b",
            "robot-b",
            vec![entry(
                1,
                None,
                "start-b",
                '2',
                "robot-b",
                SiteActorRole::Robot,
                &[],
                SiteEventPayload::ExecutionStarted {
                    package_sha256: digest('a'),
                    entity_id: "entity-1".into(),
                    attempt_id: "attempt-b".into(),
                    station_qualification_event_id: "station-1".into(),
                },
            )],
        );
        let plan = reconcile(&[first, second]).unwrap();
        assert!(!plan.can_ingest());
        assert_eq!(plan.concurrent_mutations.len(), 1);
        assert_eq!(
            plan.concurrent_mutations[0].key,
            SiteMutationKey::Entity {
                package_sha256: digest('a'),
                entity_id: "entity-1".into()
            }
        );
    }

    #[test]
    fn causal_cross_journal_referee_result_follows_robot_observation() {
        let robot = journal(
            "journal-robot-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "robot-observation",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        let referee = journal(
            "journal-referee-a",
            "referee-a",
            vec![entry(
                1,
                None,
                "referee-result",
                '2',
                "referee-a",
                SiteActorRole::Referee,
                &[],
                SiteEventPayload::RefereeAssessmentRecorded {
                    package_sha256: digest('a'),
                    entity_id: "entity-1".into(),
                    attempt_id: "attempt-a".into(),
                    assessment: crate::site_state_ledger::SiteRefereeAssessment::Unresolved {
                        receipt_sha256: digest('b'),
                        reason: "metrology target occluded".into(),
                    },
                },
            )],
        );
        let plan = reconcile(&[referee, robot]).unwrap();
        assert!(plan.can_ingest());
        let ids: Vec<_> = plan
            .ordered_events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect();
        assert_eq!(ids, vec!["robot-observation", "referee-result"]);
    }

    #[test]
    fn same_attempt_events_without_causality_are_a_conflict() {
        let robot = journal(
            "journal-robot-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "robot-observation",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        let interruption = journal(
            "journal-referee-a",
            "referee-a",
            vec![entry(
                1,
                None,
                "interruption",
                '2',
                "referee-a",
                SiteActorRole::Referee,
                &[],
                interruption_payload("attempt-a"),
            )],
        );
        let plan = reconcile(&[robot, interruption]).unwrap();
        assert!(!plan.can_ingest());
        assert_eq!(
            plan.concurrent_mutations[0].key,
            SiteMutationKey::Attempt {
                package_sha256: digest('a'),
                attempt_id: "attempt-a".into()
            }
        );
    }

    #[test]
    fn exact_replicas_collapse_to_one_logical_event_and_preserve_origins() {
        let original = entry(
            1,
            None,
            "event-replicated",
            '1',
            "robot-a",
            SiteActorRole::Robot,
            &[],
            observation_payload("attempt-a"),
        );
        let replica = entry(
            1,
            None,
            "event-replicated",
            '1',
            "robot-a",
            SiteActorRole::Robot,
            &[],
            observation_payload("attempt-a"),
        );
        let plan = reconcile(&[
            journal("journal-a", "robot-a", vec![original]),
            journal("journal-b", "robot-a", vec![replica]),
        ])
        .unwrap();
        assert_eq!(plan.ordered_events.len(), 1);
        assert_eq!(plan.duplicate_replicas_collapsed, 1);
        assert_eq!(
            plan.ordered_events[0].origin_journal_ids,
            vec!["journal-a".to_owned(), "journal-b".to_owned()]
        );
    }

    #[test]
    fn a_changed_payload_reusing_an_event_id_is_rejected() {
        let first = journal(
            "journal-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "same-id",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        let second = journal(
            "journal-b",
            "robot-a",
            vec![entry(
                1,
                None,
                "same-id",
                '2',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-b"),
            )],
        );
        assert_eq!(
            reconcile(&[first, second]),
            Err(SiteJournalReconciliationError::ConflictingEventId(
                "same-id".into()
            ))
        );
    }

    #[test]
    fn missing_parent_and_causal_cycle_fail_closed() {
        let missing = journal(
            "journal-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "child",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &["missing-parent"],
                observation_payload("attempt-a"),
            )],
        );
        assert!(matches!(
            reconcile(&[missing]),
            Err(SiteJournalReconciliationError::MissingCausalParent { .. })
        ));

        let first = journal(
            "journal-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "event-a",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &["event-b"],
                observation_payload("attempt-a"),
            )],
        );
        let second = journal(
            "journal-b",
            "robot-b",
            vec![entry(
                1,
                None,
                "event-b",
                '2',
                "robot-b",
                SiteActorRole::Robot,
                &["event-a"],
                observation_payload("attempt-b"),
            )],
        );
        assert_eq!(
            reconcile(&[first, second]),
            Err(SiteJournalReconciliationError::CausalCycle)
        );
    }

    #[test]
    fn local_chain_gaps_stale_checkpoints_and_actor_spoofing_are_rejected() {
        let mut gap = journal(
            "journal-gap",
            "robot-a",
            vec![entry(
                2,
                None,
                "event-gap",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        assert!(matches!(
            reconcile(&[gap.clone()]),
            Err(SiteJournalReconciliationError::InvalidLocalSequence { .. })
        ));

        gap.entries[0].local_sequence = 1;
        gap.base_checkpoint.sequence = 4;
        gap.base_checkpoint.event_sha256 = Some(digest('9'));
        assert!(matches!(
            reconcile(&[gap]),
            Err(SiteJournalReconciliationError::StaleJournalCheckpoint(_))
        ));

        let spoofed = journal(
            "journal-spoof",
            "robot-a",
            vec![entry(
                1,
                None,
                "event-spoof",
                '1',
                "robot-b",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        assert!(matches!(
            reconcile(&[spoofed]),
            Err(SiteJournalReconciliationError::JournalActorMismatch { .. })
        ));
    }

    #[test]
    fn committed_checkpoint_parents_are_allowed_without_reimport() {
        let first = journal(
            "journal-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "event-a",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &["committed-event"],
                observation_payload("attempt-a"),
            )],
        );
        let committed = BTreeSet::from(["committed-event".to_owned()]);
        let plan = reconcile_site_journal_bundle(
            &[first],
            "site-001",
            &checkpoint(),
            &SiteJournalCommittedContext {
                event_ids: committed,
                package_sha256s: BTreeSet::from([digest('a')]),
                attempt_ids: BTreeSet::from(["attempt-a".to_owned()]),
                ..SiteJournalCommittedContext::default()
            },
            &registry(),
        )
        .unwrap();
        assert!(plan.can_ingest());
        assert_eq!(plan.ordered_events.len(), 1);
    }

    #[test]
    fn start_of_a_second_attempt_conflicts_with_an_existing_entity_attempt() {
        let robot_a = journal(
            "journal-robot-a",
            "robot-a",
            vec![entry(
                1,
                None,
                "robot-observation-a",
                '1',
                "robot-a",
                SiteActorRole::Robot,
                &[],
                observation_payload("attempt-a"),
            )],
        );
        let robot_b = journal(
            "journal-robot-b",
            "robot-b",
            vec![entry(
                1,
                None,
                "start-attempt-b",
                '2',
                "robot-b",
                SiteActorRole::Robot,
                &[],
                SiteEventPayload::ExecutionStarted {
                    package_sha256: digest('a'),
                    entity_id: "entity-1".into(),
                    attempt_id: "attempt-b".into(),
                    station_qualification_event_id: "station-1".into(),
                },
            )],
        );
        let plan = reconcile(&[robot_a, robot_b]).unwrap();
        assert!(!plan.can_ingest());
        assert!(plan.concurrent_mutations.iter().any(|conflict| {
            conflict.key
                == SiteMutationKey::Entity {
                    package_sha256: digest('a'),
                    entity_id: "entity-1".into(),
                }
        }));
    }

}

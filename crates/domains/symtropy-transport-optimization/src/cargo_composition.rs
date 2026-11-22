// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Identity-preserving aggregation/disaggregation for physical cargo modules.
//!
//! This is a composition relation over module IDs already validated by the
//! parent cargo-module contract. It does not invent cargo identities, validate
//! fit/stacking/load-securing, change custody, or count a parent and its
//! children as additive cargo stock.

use std::collections::{BTreeMap, BTreeSet};

use crate::CargoModuleFamily;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleIdentity {
    pub module_id: String,
    pub family: CargoModuleFamily,
}

impl ModuleIdentity {
    pub fn new(module_id: impl Into<String>, family: CargoModuleFamily) -> Self {
        Self {
            module_id: module_id.into(),
            family,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceEventKey {
    pub source_id: String,
    pub source_event_id: String,
}

impl SourceEventKey {
    pub fn new(source_id: impl Into<String>, source_event_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            source_event_id: source_event_id.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipAction {
    Add,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventClassification {
    Planned,
    Actual,
}

/// An identity-preserving physical containment assertion.
///
/// A child keeps its original module ID while associated with a parent. The
/// parent does not replace or consume the child. Source semantics and
/// timestamp normalization are owned by the relevant EPCIS/DCSA adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionEvent {
    pub source_key: SourceEventKey,
    pub revision: u32,
    pub published_at: i64,
    pub event_time: i64,
    pub parent_module_id: String,
    pub child_module_id: String,
    pub action: MembershipAction,
    pub classification: EventClassification,
    pub source_profile_id: String,
    pub source_profile_version: String,
    pub location_id: String,
    pub evidence_id: String,
    pub voided: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Membership {
    pub parent_module_id: String,
    pub child_module_id: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CompositionFailure {
    EmptySourceIdentity,
    InvalidRevision,
    EmptyModuleIdentity,
    EmptySourceProfile,
    EmptyLocation,
    MissingEvidenceReference,
    UnknownParent(String),
    UnknownChild(String),
    SelfContainment(String),
    DuplicateMembership(Membership),
    MultipleParents {
        child_module_id: String,
        existing_parent_id: String,
        proposed_parent_id: String,
    },
    UnmatchedDelete(Membership),
    Cycle(Membership),
    AmbiguousEventTime {
        event_time: i64,
        module_id: String,
    },
    EmptyMutationBatch,
    DuplicateMutation,
    ConflictingMutationsWithinBatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionError {
    DuplicateModuleId(String),
    InvalidModuleId(String),
    ConflictingEventRevision {
        source_key: SourceEventKey,
        revision: u32,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompositionReport {
    pub module_ids: BTreeSet<String>,
    pub input_event_rows: usize,
    pub visible_event_rows: usize,
    pub future_event_rows: usize,
    pub superseded_event_rows: usize,
    pub replayed_event_rows: usize,
    pub voided_event_rows: usize,
    pub planned_event_rows: usize,
    pub actual_event_rows: usize,
    pub active_memberships: BTreeSet<Membership>,
    /// Number of effective parent-scoped disaggregation-all transactions.
    pub disaggregation_all_events: usize,
    /// Number of direct parent/child membership edges removed by those events.
    pub disaggregated_membership_edges: usize,
    /// Number of superseded source-revision edges retracted when present.
    pub revision_retracted_membership_edges: usize,
    pub failures: BTreeMap<CompositionFailure, usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleCountBasis {
    /// Count every distinct physical module identity.
    AllModuleIdentities,
    /// Count modules with no active parent (top-level handling units).
    RootHandlingUnits,
    /// Count modules that contain no active children (leaf units).
    LeafHandlingUnits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionProjectionError {
    UnqualifiedComposition,
}

impl CompositionReport {
    pub fn is_qualified(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn active_edges(&self) -> usize {
        self.active_memberships.len()
    }

    /// A count projection is available only for a fully qualified graph.
    /// Module identities come from the exact catalogue captured at evaluation.
    pub fn count_modules(
        &self,
        basis: ModuleCountBasis,
    ) -> Result<usize, CompositionProjectionError> {
        if !self.is_qualified() {
            return Err(CompositionProjectionError::UnqualifiedComposition);
        }

        let parent_ids: BTreeSet<&str> = self
            .active_memberships
            .iter()
            .map(|membership| membership.parent_module_id.as_str())
            .collect();
        let child_ids: BTreeSet<&str> = self
            .active_memberships
            .iter()
            .map(|membership| membership.child_module_id.as_str())
            .collect();
        let all_ids: BTreeSet<&str> = self.module_ids.iter().map(String::as_str).collect();

        Ok(match basis {
            ModuleCountBasis::AllModuleIdentities => all_ids.len(),
            ModuleCountBasis::RootHandlingUnits => all_ids.difference(&child_ids).count(),
            ModuleCountBasis::LeafHandlingUnits => all_ids.difference(&parent_ids).count(),
        })
    }
}
fn record_failure(report: &mut CompositionReport, failure: CompositionFailure) {
    *report.failures.entry(failure).or_default() += 1;
}

fn validate_event_shape(event: &CompositionEvent) -> Option<CompositionFailure> {
    if event.source_key.source_id.trim().is_empty()
        || event.source_key.source_event_id.trim().is_empty()
        || event.source_key.source_id.trim() != event.source_key.source_id
        || event.source_key.source_event_id.trim() != event.source_key.source_event_id
    {
        return Some(CompositionFailure::EmptySourceIdentity);
    }
    if event.revision == 0 {
        return Some(CompositionFailure::InvalidRevision);
    }
    if event.parent_module_id.trim().is_empty()
        || event.child_module_id.trim().is_empty()
        || event.parent_module_id.trim() != event.parent_module_id
        || event.child_module_id.trim() != event.child_module_id
    {
        return Some(CompositionFailure::EmptyModuleIdentity);
    }
    if event.source_profile_id.trim().is_empty()
        || event.source_profile_version.trim().is_empty()
        || event.source_profile_id.trim() != event.source_profile_id
        || event.source_profile_version.trim() != event.source_profile_version
    {
        return Some(CompositionFailure::EmptySourceProfile);
    }
    if event.location_id.trim().is_empty() || event.location_id.trim() != event.location_id {
        return Some(CompositionFailure::EmptyLocation);
    }
    if event.evidence_id.trim().is_empty() || event.evidence_id.trim() != event.evidence_id {
        return Some(CompositionFailure::MissingEvidenceReference);
    }
    None
}

fn active_parent_of<'a>(
    active: &'a BTreeSet<Membership>,
    child_module_id: &str,
) -> Option<&'a str> {
    active
        .iter()
        .find(|membership| membership.child_module_id == child_module_id)
        .map(|membership| membership.parent_module_id.as_str())
}

fn creates_cycle(active: &BTreeSet<Membership>, proposed: &Membership) -> bool {
    let mut cursor = proposed.parent_module_id.as_str();
    let mut visited = BTreeSet::new();

    loop {
        if cursor == proposed.child_module_id {
            return true;
        }
        if !visited.insert(cursor) {
            // Existing state should already be acyclic. Treat any cycle
            // encountered during traversal as unsafe.
            return true;
        }
        match active_parent_of(active, cursor) {
            Some(parent) => cursor = parent,
            None => return false,
        }
    }
}

fn apply_event(
    report: &mut CompositionReport,
    known_modules: &BTreeSet<&str>,
    event: &CompositionEvent,
) {
    if let Some(failure) = validate_event_shape(event) {
        record_failure(report, failure);
        return;
    }
    if !known_modules.contains(event.parent_module_id.as_str()) {
        record_failure(
            report,
            CompositionFailure::UnknownParent(event.parent_module_id.clone()),
        );
        return;
    }
    if !known_modules.contains(event.child_module_id.as_str()) {
        record_failure(
            report,
            CompositionFailure::UnknownChild(event.child_module_id.clone()),
        );
        return;
    }
    if event.parent_module_id == event.child_module_id {
        record_failure(
            report,
            CompositionFailure::SelfContainment(event.parent_module_id.clone()),
        );
        return;
    }

    let membership = Membership {
        parent_module_id: event.parent_module_id.clone(),
        child_module_id: event.child_module_id.clone(),
    };

    match event.action {
        MembershipAction::Add => {
            if report.active_memberships.contains(&membership) {
                record_failure(report, CompositionFailure::DuplicateMembership(membership));
                return;
            }
            if let Some(existing_parent) =
                active_parent_of(&report.active_memberships, &membership.child_module_id)
            {
                record_failure(
                    report,
                    CompositionFailure::MultipleParents {
                        child_module_id: membership.child_module_id,
                        existing_parent_id: existing_parent.to_string(),
                        proposed_parent_id: membership.parent_module_id,
                    },
                );
                return;
            }
            if creates_cycle(&report.active_memberships, &membership) {
                record_failure(report, CompositionFailure::Cycle(membership));
                return;
            }
            report.active_memberships.insert(membership);
        }
        MembershipAction::Delete => {
            if !report.active_memberships.remove(&membership) {
                record_failure(report, CompositionFailure::UnmatchedDelete(membership));
            }
        }
    }
}

/// Reconstruct an identity-preserving composition graph at a publication-time
/// cutoff. Same-time events that touch the same module are rejected rather
/// than ordered by event ID or ingestion order.
///
/// This does not alter custody, location, contents digests, seal status,
/// physical inventory stock, or any cargo-module profile/instance.
pub fn evaluate_composition(
    modules: &[ModuleIdentity],
    events: &[CompositionEvent],
    as_of: i64,
) -> Result<CompositionReport, CompositionError> {
    let mut known_modules = BTreeSet::new();
    for module in modules {
        if module.module_id.trim().is_empty() || module.module_id.trim() != module.module_id {
            return Err(CompositionError::InvalidModuleId(module.module_id.clone()));
        }
        if !known_modules.insert(module.module_id.as_str()) {
            return Err(CompositionError::DuplicateModuleId(
                module.module_id.clone(),
            ));
        }
    }

    let mut report = CompositionReport {
        module_ids: known_modules
            .iter()
            .map(|module_id| (*module_id).to_string())
            .collect(),
        input_event_rows: events.len(),
        ..CompositionReport::default()
    };
    let mut by_source_key: BTreeMap<&SourceEventKey, Vec<&CompositionEvent>> = BTreeMap::new();

    for event in events {
        if event.published_at <= as_of {
            report.visible_event_rows += 1;
            by_source_key
                .entry(&event.source_key)
                .or_default()
                .push(event);
        } else {
            report.future_event_rows += 1;
        }
    }

    let mut effective_events = Vec::new();
    for (source_key, revisions) in by_source_key {
        let latest_revision = revisions
            .iter()
            .map(|event| event.revision)
            .max()
            .unwrap_or_default();
        let latest: Vec<&CompositionEvent> = revisions
            .iter()
            .copied()
            .filter(|event| event.revision == latest_revision)
            .collect();
        let first = latest[0];

        if latest.iter().any(|candidate| *candidate != first) {
            return Err(CompositionError::ConflictingEventRevision {
                source_key: source_key.clone(),
                revision: latest_revision,
            });
        }

        report.replayed_event_rows += latest.len().saturating_sub(1);
        report.superseded_event_rows += revisions.len().saturating_sub(latest.len());

        if first.voided {
            report.voided_event_rows += 1;
        } else {
            effective_events.push(first);
        }
    }

    let mut actual_by_time: BTreeMap<i64, Vec<&CompositionEvent>> = BTreeMap::new();
    for event in effective_events {
        match event.classification {
            EventClassification::Planned => {
                report.planned_event_rows += 1;
            }
            EventClassification::Actual => {
                report.actual_event_rows += 1;
                actual_by_time
                    .entry(event.event_time)
                    .or_default()
                    .push(event);
            }
        }
    }

    for (event_time, batch) in actual_by_time {
        let mut touched: BTreeMap<&str, usize> = BTreeMap::new();
        for event in &batch {
            let event_modules = [
                event.parent_module_id.as_str(),
                event.child_module_id.as_str(),
            ];
            let unique_modules: BTreeSet<&str> = event_modules.into_iter().collect();
            for module_id in unique_modules {
                *touched.entry(module_id).or_default() += 1;
            }
        }

        let ambiguous_modules: BTreeSet<&str> = touched
            .into_iter()
            .filter_map(|(module_id, count)| (count > 1).then_some(module_id))
            .collect();

        let mut ordered_batch = batch;
        ordered_batch.sort_by(|left, right| left.source_key.cmp(&right.source_key));

        for event in ordered_batch {
            let ambiguous = ambiguous_modules.contains(event.parent_module_id.as_str())
                || ambiguous_modules.contains(event.child_module_id.as_str());
            if ambiguous {
                let module_id = if ambiguous_modules.contains(event.child_module_id.as_str()) {
                    event.child_module_id.clone()
                } else {
                    event.parent_module_id.clone()
                };
                record_failure(
                    &mut report,
                    CompositionFailure::AmbiguousEventTime {
                        event_time,
                        module_id,
                    },
                );
                continue;
            }

            apply_event(&mut report, &known_modules, event);
        }
    }

    Ok(report)
}

/// One atomic mutation in a source-event-level composition transaction.
///
/// A transaction may contain multiple child-specific ADDs or DELETEs from one
/// source event. DELETE_ALL is parent-scoped and deliberately has no fabricated
/// child identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CompositionMutation {
    Add {
        parent_module_id: String,
        child_module_id: String,
    },
    Delete {
        parent_module_id: String,
        child_module_id: String,
    },
    /// Source-revision compensation: retract an earlier assertion if present.
    /// Unlike physical DELETE, absence is not a failure during from-scratch replay.
    RetractIfPresent {
        parent_module_id: String,
        child_module_id: String,
    },
    DisaggregateAll {
        parent_module_id: String,
    },
}

/// A source-event-level, atomic composition transaction.
///
/// Unlike CompositionEvent, this shape retains the source event identity once
/// and can carry the complete unordered child list. This avoids treating
/// several children from one EPCIS event as independent simultaneous events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionEventBatch {
    pub source_key: SourceEventKey,
    pub revision: u32,
    pub published_at: i64,
    pub event_time: i64,
    pub mutations: Vec<CompositionMutation>,
    pub classification: EventClassification,
    pub source_profile_id: String,
    pub source_profile_version: String,
    pub location_id: String,
    pub evidence_id: String,
    pub voided: bool,
}

fn mutation_touched_modules(mutation: &CompositionMutation) -> BTreeSet<&str> {
    match mutation {
        CompositionMutation::Add {
            parent_module_id,
            child_module_id,
        }
        | CompositionMutation::Delete {
            parent_module_id,
            child_module_id,
        }
        | CompositionMutation::RetractIfPresent {
            parent_module_id,
            child_module_id,
        } => BTreeSet::from([parent_module_id.as_str(), child_module_id.as_str()]),
        CompositionMutation::DisaggregateAll { parent_module_id } => {
            BTreeSet::from([parent_module_id.as_str()])
        }
    }
}

fn batch_touched_modules(batch: &CompositionEventBatch) -> BTreeSet<&str> {
    let mut touched = BTreeSet::new();
    for mutation in &batch.mutations {
        touched.extend(mutation_touched_modules(mutation));
    }
    touched
}

fn validate_batch_shape(batch: &CompositionEventBatch) -> Option<CompositionFailure> {
    if batch.source_key.source_id.trim().is_empty()
        || batch.source_key.source_event_id.trim().is_empty()
        || batch.source_key.source_id.trim() != batch.source_key.source_id
        || batch.source_key.source_event_id.trim() != batch.source_key.source_event_id
    {
        return Some(CompositionFailure::EmptySourceIdentity);
    }
    if batch.revision == 0 {
        return Some(CompositionFailure::InvalidRevision);
    }
    if batch.source_profile_id.trim().is_empty()
        || batch.source_profile_version.trim().is_empty()
        || batch.source_profile_id.trim() != batch.source_profile_id
        || batch.source_profile_version.trim() != batch.source_profile_version
    {
        return Some(CompositionFailure::EmptySourceProfile);
    }
    if batch.location_id.trim().is_empty() || batch.location_id.trim() != batch.location_id {
        return Some(CompositionFailure::EmptyLocation);
    }
    if batch.evidence_id.trim().is_empty() || batch.evidence_id.trim() != batch.evidence_id {
        return Some(CompositionFailure::MissingEvidenceReference);
    }
    if batch.mutations.is_empty() {
        return Some(CompositionFailure::EmptyMutationBatch);
    }
    let has_disaggregate_all = batch
        .mutations
        .iter()
        .any(|mutation| matches!(mutation, CompositionMutation::DisaggregateAll { .. }));
    if has_disaggregate_all && batch.mutations.len() != 1 {
        return Some(CompositionFailure::ConflictingMutationsWithinBatch);
    }
    let mut unique_mutations = BTreeSet::new();
    if batch
        .mutations
        .iter()
        .any(|mutation| !unique_mutations.insert(mutation))
    {
        return Some(CompositionFailure::DuplicateMutation);
    }
    for mutation in &batch.mutations {
        let identities = mutation_touched_modules(mutation);
        if identities
            .iter()
            .any(|identity| identity.trim().is_empty() || identity.trim() != *identity)
        {
            return Some(CompositionFailure::EmptyModuleIdentity);
        }
    }
    None
}

/// Apply one transaction against a candidate graph. A failed child assertion
/// rejects the whole transaction; no partial child list is committed.
fn apply_composition_batch(
    report: &mut CompositionReport,
    known_modules: &BTreeSet<&str>,
    batch: &CompositionEventBatch,
) {
    if let Some(failure) = validate_batch_shape(batch) {
        record_failure(report, failure);
        return;
    }

    let mut candidate = report.active_memberships.clone();
    let mut ordered = batch.mutations.clone();
    // Revision replacement must remove superseded edges before adding the
    // latest source revision's child list. All changes still commit atomically.
    ordered.sort_by(|left, right| {
        let rank = |mutation: &CompositionMutation| match mutation {
            CompositionMutation::RetractIfPresent { .. } => 0_u8,
            CompositionMutation::Delete { .. } => 1,
            CompositionMutation::DisaggregateAll { .. } => 2,
            CompositionMutation::Add { .. } => 3,
        };
        rank(left).cmp(&rank(right)).then_with(|| left.cmp(right))
    });

    let mut disaggregation_count = 0usize;
    let mut retracted_count = 0usize;
    for mutation in &ordered {
        let failure = match mutation {
            CompositionMutation::Add {
                parent_module_id,
                child_module_id,
            } => {
                if !known_modules.contains(parent_module_id.as_str()) {
                    Some(CompositionFailure::UnknownParent(parent_module_id.clone()))
                } else if !known_modules.contains(child_module_id.as_str()) {
                    Some(CompositionFailure::UnknownChild(child_module_id.clone()))
                } else if parent_module_id == child_module_id {
                    Some(CompositionFailure::SelfContainment(parent_module_id.clone()))
                } else {
                    let membership = Membership {
                        parent_module_id: parent_module_id.clone(),
                        child_module_id: child_module_id.clone(),
                    };
                    if candidate.contains(&membership) {
                        Some(CompositionFailure::DuplicateMembership(membership))
                    } else if let Some(existing_parent) =
                        active_parent_of(&candidate, child_module_id)
                    {
                        Some(CompositionFailure::MultipleParents {
                            child_module_id: child_module_id.clone(),
                            existing_parent_id: existing_parent.to_string(),
                            proposed_parent_id: parent_module_id.clone(),
                        })
                    } else if creates_cycle(&candidate, &membership) {
                        Some(CompositionFailure::Cycle(membership))
                    } else {
                        candidate.insert(membership);
                        None
                    }
                }
            }
            CompositionMutation::Delete {
                parent_module_id,
                child_module_id,
            } => {
                if !known_modules.contains(parent_module_id.as_str()) {
                    Some(CompositionFailure::UnknownParent(parent_module_id.clone()))
                } else if !known_modules.contains(child_module_id.as_str()) {
                    Some(CompositionFailure::UnknownChild(child_module_id.clone()))
                } else if parent_module_id == child_module_id {
                    Some(CompositionFailure::SelfContainment(parent_module_id.clone()))
                } else {
                    let membership = Membership {
                        parent_module_id: parent_module_id.clone(),
                        child_module_id: child_module_id.clone(),
                    };
                    if !candidate.remove(&membership) {
                        Some(CompositionFailure::UnmatchedDelete(membership))
                    } else {
                        None
                    }
                }
            }
            CompositionMutation::RetractIfPresent {
                parent_module_id,
                child_module_id,
            } => {
                if !known_modules.contains(parent_module_id.as_str()) {
                    Some(CompositionFailure::UnknownParent(parent_module_id.clone()))
                } else if !known_modules.contains(child_module_id.as_str()) {
                    Some(CompositionFailure::UnknownChild(child_module_id.clone()))
                } else if parent_module_id == child_module_id {
                    Some(CompositionFailure::SelfContainment(parent_module_id.clone()))
                } else {
                    let membership = Membership {
                        parent_module_id: parent_module_id.clone(),
                        child_module_id: child_module_id.clone(),
                    };
                    if candidate.remove(&membership) {
                        disaggregation_count = disaggregation_count.saturating_add(0);
                        retracted_count += 1;
                    }
                    None
                }
            }
            CompositionMutation::DisaggregateAll { parent_module_id } => {
                if !known_modules.contains(parent_module_id.as_str()) {
                    Some(CompositionFailure::UnknownParent(parent_module_id.clone()))
                } else {
                    let before = candidate.len();
                    candidate.retain(|membership| {
                        membership.parent_module_id != *parent_module_id
                    });
                    disaggregation_count += before - candidate.len();
                    None
                }
            }
        };

        if let Some(failure) = failure {
            record_failure(report, failure);
            return;
        }
    }

    if ordered
        .iter()
        .any(|mutation| matches!(mutation, CompositionMutation::DisaggregateAll { .. }))
    {
        report.disaggregation_all_events += 1;
        report.disaggregated_membership_edges += disaggregation_count;
    }
    report.revision_retracted_membership_edges += retracted_count;
    report.active_memberships = candidate;
}

/// Replay source-event-level transactions at a publication-time cutoff.
///
/// A batch is atomic: several child additions/deletions from one source event
/// are one event-time actor, while independent source events touching a common
/// module at the same tick remain ambiguous. DisaggregateAll removes only the
/// selected parent's direct membership edges; it never cascades into child
/// contents or other relation families.
///
/// The legacy evaluate_composition API remains available for single-edge
/// callers. EPCIS adapters that carry unordered multi-child lists should use
/// this grouped transaction API.
pub fn evaluate_composition_batches(
    modules: &[ModuleIdentity],
    batches: &[CompositionEventBatch],
    as_of: i64,
) -> Result<CompositionReport, CompositionError> {
    let mut known_modules = BTreeSet::new();
    for module in modules {
        if module.module_id.trim().is_empty() || module.module_id.trim() != module.module_id {
            return Err(CompositionError::InvalidModuleId(module.module_id.clone()));
        }
        if !known_modules.insert(module.module_id.as_str()) {
            return Err(CompositionError::DuplicateModuleId(
                module.module_id.clone(),
            ));
        }
    }

    let mut report = CompositionReport {
        module_ids: known_modules
            .iter()
            .map(|module_id| (*module_id).to_string())
            .collect(),
        input_event_rows: batches.len(),
        ..CompositionReport::default()
    };
    let mut by_source_key: BTreeMap<&SourceEventKey, Vec<&CompositionEventBatch>> =
        BTreeMap::new();

    for batch in batches {
        if batch.published_at <= as_of {
            report.visible_event_rows += 1;
            by_source_key
                .entry(&batch.source_key)
                .or_default()
                .push(batch);
        } else {
            report.future_event_rows += 1;
        }
    }

    let mut effective = Vec::new();
    for (source_key, revisions) in by_source_key {
        let latest_revision = revisions
            .iter()
            .map(|batch| batch.revision)
            .max()
            .unwrap_or_default();
        let latest: Vec<&CompositionEventBatch> = revisions
            .iter()
            .copied()
            .filter(|batch| batch.revision == latest_revision)
            .collect();
        let first = latest[0];
        if latest.iter().any(|candidate| *candidate != first) {
            return Err(CompositionError::ConflictingEventRevision {
                source_key: source_key.clone(),
                revision: latest_revision,
            });
        }
        report.replayed_event_rows += latest.len().saturating_sub(1);
        report.superseded_event_rows += revisions.len().saturating_sub(latest.len());
        if first.voided {
            report.voided_event_rows += 1;
        } else {
            effective.push(first);
        }
    }

    let mut actual_by_time: BTreeMap<i64, Vec<&CompositionEventBatch>> = BTreeMap::new();
    for batch in effective {
        match batch.classification {
            EventClassification::Planned => report.planned_event_rows += 1,
            EventClassification::Actual => {
                report.actual_event_rows += 1;
                actual_by_time.entry(batch.event_time).or_default().push(batch);
            }
        }
    }

    for (event_time, time_batch) in actual_by_time {
        let mut touched: BTreeMap<&str, usize> = BTreeMap::new();
        for batch in &time_batch {
            for module_id in batch_touched_modules(batch) {
                *touched.entry(module_id).or_default() += 1;
            }
        }
        let ambiguous_modules: BTreeSet<&str> = touched
            .into_iter()
            .filter_map(|(module_id, count)| (count > 1).then_some(module_id))
            .collect();

        let mut ordered_batches = time_batch;
        ordered_batches.sort_by(|left, right| left.source_key.cmp(&right.source_key));
        for batch in ordered_batches {
            let batch_modules = batch_touched_modules(batch);
            let conflicts: BTreeSet<&str> = batch_modules
                .intersection(&ambiguous_modules)
                .copied()
                .collect();
            if !conflicts.is_empty() {
                for module_id in conflicts {
                    record_failure(
                        &mut report,
                        CompositionFailure::AmbiguousEventTime {
                            event_time,
                            module_id: module_id.to_string(),
                        },
                    );
                }
                continue;
            }
            apply_composition_batch(&mut report, &known_modules, batch);
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules(ids: &[&str]) -> Vec<ModuleIdentity> {
        ids.iter()
            .map(|id| ModuleIdentity::new(*id, CargoModuleFamily::Custom))
            .collect()
    }

    fn event(
        event_id: &str,
        time: i64,
        parent: &str,
        child: &str,
        action: MembershipAction,
    ) -> CompositionEvent {
        CompositionEvent {
            source_key: SourceEventKey::new("WMS_A", event_id),
            revision: 1,
            published_at: time,
            event_time: time,
            parent_module_id: parent.to_string(),
            child_module_id: child.to_string(),
            action,
            classification: EventClassification::Actual,
            source_profile_id: "EPCIS_AGGREGATION".to_string(),
            source_profile_version: "2.0.1".to_string(),
            location_id: "WAREHOUSE_01".to_string(),
            evidence_id: format!("EV-{event_id}"),
            voided: false,
        }
    }

    fn batch(
        event_id: &str,
        time: i64,
        mutations: Vec<CompositionMutation>,
    ) -> CompositionEventBatch {
        CompositionEventBatch {
            source_key: SourceEventKey::new("WMS_A", event_id),
            revision: 1,
            published_at: time,
            event_time: time,
            mutations,
            classification: EventClassification::Actual,
            source_profile_id: "EPCIS_AGGREGATION".to_string(),
            source_profile_version: "2.0.1".to_string(),
            location_id: "WAREHOUSE_01".to_string(),
            evidence_id: format!("EV-{event_id}"),
            voided: false,
        }
    }

    #[test]
    fn one_source_event_atomically_adds_multiple_unordered_children() {
        let modules = modules(&["CONTAINER", "PALLET_A", "PALLET_B", "CARTON"]);
        let event = batch(
            "ADD-MANY",
            10,
            vec![
                CompositionMutation::Add {
                    parent_module_id: "CONTAINER".to_string(),
                    child_module_id: "PALLET_B".to_string(),
                },
                CompositionMutation::Add {
                    parent_module_id: "CONTAINER".to_string(),
                    child_module_id: "PALLET_A".to_string(),
                },
            ],
        );
        let report = evaluate_composition_batches(&modules, &[event], 100).unwrap();
        assert!(report.is_qualified());
        assert_eq!(report.active_edges(), 2);
        assert_eq!(
            report
                .active_memberships
                .iter()
                .map(|edge| edge.child_module_id.as_str())
                .collect::<Vec<_>>(),
            vec!["PALLET_A", "PALLET_B"]
        );
    }

    #[test]
    fn delete_all_removes_only_direct_children_and_preserves_nested_contents() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON", "OTHER"]);
        let events = vec![
            batch(
                "ADD-1",
                10,
                vec![CompositionMutation::Add {
                    parent_module_id: "CONTAINER".to_string(),
                    child_module_id: "PALLET".to_string(),
                }],
            ),
            batch(
                "ADD-2",
                20,
                vec![CompositionMutation::Add {
                    parent_module_id: "PALLET".to_string(),
                    child_module_id: "CARTON".to_string(),
                }],
            ),
            batch(
                "ADD-3",
                25,
                vec![CompositionMutation::Add {
                    parent_module_id: "OTHER".to_string(),
                    child_module_id: "CONTAINER".to_string(),
                }],
            ),
            batch(
                "DELETE-ALL",
                30,
                vec![CompositionMutation::DisaggregateAll {
                    parent_module_id: "CONTAINER".to_string(),
                }],
            ),
        ];
        let report = evaluate_composition_batches(&modules, &events, 100).unwrap();
        assert!(report.is_qualified());
        assert_eq!(report.active_edges(), 2);
        assert!(report.active_memberships.contains(&Membership {
            parent_module_id: "PALLET".to_string(),
            child_module_id: "CARTON".to_string(),
        }));
        assert!(report.active_memberships.contains(&Membership {
            parent_module_id: "OTHER".to_string(),
            child_module_id: "CONTAINER".to_string(),
        }));
        assert!(!report.active_memberships.iter().any(|edge| {
            edge.parent_module_id == "CONTAINER"
        }));
        assert_eq!(report.disaggregation_all_events, 1);
        assert_eq!(report.disaggregated_membership_edges, 1);
    }

    #[test]
    fn delete_all_clears_every_direct_child_from_the_selected_parent() {
        let modules = modules(&["CONTAINER", "PALLET_A", "PALLET_B", "CARTON"]);
        let events = vec![
            batch(
                "ADD-MANY",
                10,
                vec![
                    CompositionMutation::Add {
                        parent_module_id: "CONTAINER".to_string(),
                        child_module_id: "PALLET_A".to_string(),
                    },
                    CompositionMutation::Add {
                        parent_module_id: "CONTAINER".to_string(),
                        child_module_id: "PALLET_B".to_string(),
                    },
                ],
            ),
            batch(
                "ADD-NESTED",
                20,
                vec![CompositionMutation::Add {
                    parent_module_id: "PALLET_A".to_string(),
                    child_module_id: "CARTON".to_string(),
                }],
            ),
            batch(
                "DELETE-ALL",
                30,
                vec![CompositionMutation::DisaggregateAll {
                    parent_module_id: "CONTAINER".to_string(),
                }],
            ),
        ];
        let report = evaluate_composition_batches(&modules, &events, 100).unwrap();
        assert!(report.is_qualified());
        assert_eq!(report.active_edges(), 1);
        assert_eq!(report.disaggregated_membership_edges, 2);
        assert_eq!(
            report.active_memberships.iter().next(),
            Some(&Membership {
                parent_module_id: "PALLET_A".to_string(),
                child_module_id: "CARTON".to_string(),
            })
        );
    }

    #[test]
    fn delete_all_obeys_publication_cutoff_and_future_visibility() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let add = batch(
            "ADD",
            10,
            vec![CompositionMutation::Add {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET".to_string(),
            }],
        );
        let mut delete_all = batch(
            "DELETE-ALL",
            30,
            vec![CompositionMutation::DisaggregateAll {
                parent_module_id: "CONTAINER".to_string(),
            }],
        );
        delete_all.published_at = 50;
        let before_publication =
            evaluate_composition_batches(&modules, &[add.clone(), delete_all.clone()], 40).unwrap();
        assert_eq!(before_publication.active_edges(), 1);
        assert_eq!(before_publication.future_event_rows, 1);

        let after_publication =
            evaluate_composition_batches(&modules, &[add, delete_all], 60).unwrap();
        assert_eq!(after_publication.active_edges(), 0);
        assert_eq!(after_publication.disaggregation_all_events, 1);
    }

    #[test]
    fn exact_duplicate_delete_all_replay_is_idempotent() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let add = batch(
            "ADD",
            10,
            vec![CompositionMutation::Add {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET".to_string(),
            }],
        );
        let delete_all = batch(
            "DELETE-ALL",
            20,
            vec![CompositionMutation::DisaggregateAll {
                parent_module_id: "CONTAINER".to_string(),
            }],
        );
        let report = evaluate_composition_batches(
            &modules,
            &[add, delete_all.clone(), delete_all],
            100,
        )
        .unwrap();
        assert_eq!(report.replayed_event_rows, 1);
        assert_eq!(report.disaggregation_all_events, 1);
        assert_eq!(report.disaggregated_membership_edges, 1);
        assert_eq!(report.active_edges(), 0);
    }

    #[test]
    fn conflicting_same_revision_delete_all_is_rejected() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let mut first = batch(
            "DELETE-ALL",
            20,
            vec![CompositionMutation::DisaggregateAll {
                parent_module_id: "CONTAINER".to_string(),
            }],
        );
        let mut second = first.clone();
        second.mutations = vec![CompositionMutation::DisaggregateAll {
            parent_module_id: "PALLET".to_string(),
        }];
        assert!(matches!(
            evaluate_composition_batches(&modules, &[first, second], 100),
            Err(CompositionError::ConflictingEventRevision { .. })
        ));
    }

    #[test]
    fn later_void_revision_suppresses_prior_delete_all() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let add = batch(
            "ADD",
            10,
            vec![CompositionMutation::Add {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET".to_string(),
            }],
        );
        let mut delete_all_v1 = batch(
            "DELETE-ALL",
            20,
            vec![CompositionMutation::DisaggregateAll {
                parent_module_id: "CONTAINER".to_string(),
            }],
        );
        delete_all_v1.revision = 1;
        let mut delete_all_v2 = delete_all_v1.clone();
        delete_all_v2.revision = 2;
        delete_all_v2.published_at = 30;
        delete_all_v2.voided = true;
        let report =
            evaluate_composition_batches(&modules, &[add, delete_all_v1, delete_all_v2], 100)
                .unwrap();
        assert_eq!(report.active_edges(), 1);
        assert_eq!(report.voided_event_rows, 1);
        assert_eq!(report.disaggregation_all_events, 0);
    }

    #[test]
    fn failed_multi_child_batch_rolls_back_all_changes() {
        let modules = modules(&["CONTAINER", "PALLET_A", "PALLET_B"]);
        let existing = batch(
            "ADD-OLD",
            10,
            vec![CompositionMutation::Add {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET_A".to_string(),
            }],
        );
        let invalid_batch = batch(
            "ADD-ATOMIC",
            20,
            vec![
                CompositionMutation::Add {
                    parent_module_id: "CONTAINER".to_string(),
                    child_module_id: "PALLET_A".to_string(),
                },
                CompositionMutation::Add {
                    parent_module_id: "CONTAINER".to_string(),
                    child_module_id: "PALLET_B".to_string(),
                },
            ],
        );
        let report =
            evaluate_composition_batches(&modules, &[existing, invalid_batch], 100).unwrap();
        assert_eq!(report.active_edges(), 1);
        assert!(!report.active_memberships.iter().any(|edge| {
            edge.child_module_id == "PALLET_B"
        }));
        assert!(report.failures.contains_key(
            &CompositionFailure::DuplicateMembership(Membership {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET_A".to_string(),
            })
        ));
    }

    #[test]
    fn independent_source_transactions_touching_same_parent_at_same_time_are_ambiguous() {
        let modules = modules(&["CONTAINER", "PALLET_A", "PALLET_B"]);
        let first = batch(
            "ADD-A",
            10,
            vec![CompositionMutation::Add {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET_A".to_string(),
            }],
        );
        let second = batch(
            "ADD-B",
            10,
            vec![CompositionMutation::Add {
                parent_module_id: "CONTAINER".to_string(),
                child_module_id: "PALLET_B".to_string(),
            }],
        );
        let report = evaluate_composition_batches(&modules, &[first, second], 100).unwrap();
        assert_eq!(report.active_edges(), 0);
        assert!(report.failures.contains_key(
            &CompositionFailure::AmbiguousEventTime {
                event_time: 10,
                module_id: "CONTAINER".to_string(),
            }
        ));
    }

    #[test]
    fn delete_all_from_unknown_parent_fails_closed() {
        let modules = modules(&["CONTAINER"]);
        let event = batch(
            "DELETE-ALL",
            10,
            vec![CompositionMutation::DisaggregateAll {
                parent_module_id: "UNKNOWN".to_string(),
            }],
        );
        let report = evaluate_composition_batches(&modules, &[event], 100).unwrap();
        assert!(!report.is_qualified());
        assert_eq!(
            report.failures.get(&CompositionFailure::UnknownParent("UNKNOWN".to_string())),
            Some(&1)
        );
        assert_eq!(report.active_edges(), 0);
    }

    #[test]
    fn nested_aggregation_preserves_all_module_identities() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON_A", "CARTON_B"]);
        let events = vec![
            event("ADD-1", 10, "CONTAINER", "PALLET", MembershipAction::Add),
            event("ADD-2", 20, "PALLET", "CARTON_A", MembershipAction::Add),
            event("ADD-3", 30, "PALLET", "CARTON_B", MembershipAction::Add),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert!(report.is_qualified());
        assert_eq!(report.active_edges(), 3);
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::AllModuleIdentities)
                .unwrap(),
            4
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::RootHandlingUnits)
                .unwrap(),
            1
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::LeafHandlingUnits)
                .unwrap(),
            2
        );
    }

    #[test]
    fn disaggregation_removes_only_the_declared_parent_child_edge() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let events = vec![
            event("ADD-1", 10, "CONTAINER", "PALLET", MembershipAction::Add),
            event("ADD-2", 20, "PALLET", "CARTON", MembershipAction::Add),
            event("DEL-1", 30, "PALLET", "CARTON", MembershipAction::Delete),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert!(report.is_qualified());
        assert_eq!(report.active_edges(), 1);
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::RootHandlingUnits)
                .unwrap(),
            2
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::LeafHandlingUnits)
                .unwrap(),
            2
        );
    }

    #[test]
    fn removing_a_parent_edge_does_not_cascade_to_nested_children() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let events = vec![
            event("ADD-1", 10, "CONTAINER", "PALLET", MembershipAction::Add),
            event("ADD-2", 20, "PALLET", "CARTON", MembershipAction::Add),
            event(
                "DELETE-1",
                30,
                "CONTAINER",
                "PALLET",
                MembershipAction::Delete,
            ),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert!(report.is_qualified());
        assert_eq!(
            report.active_memberships,
            BTreeSet::from([Membership {
                parent_module_id: "PALLET".to_string(),
                child_module_id: "CARTON".to_string(),
            }])
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::RootHandlingUnits)
                .unwrap(),
            2
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::LeafHandlingUnits)
                .unwrap(),
            2
        );
    }

    #[test]
    fn whitespace_identifiers_are_rejected_not_normalized() {
        let invalid_modules = modules(&["CONTAINER", " PALLET "]);
        assert_eq!(
            evaluate_composition(&invalid_modules, &[], 100),
            Err(CompositionError::InvalidModuleId(" PALLET ".to_string()))
        );

        let known_modules = modules(&["CONTAINER", "PALLET"]);
        let mut invalid = event("ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        invalid.child_module_id = " PALLET ".to_string();
        let report = evaluate_composition(&known_modules, &[invalid], 100).unwrap();
        assert_eq!(
            report
                .failures
                .get(&CompositionFailure::EmptyModuleIdentity),
            Some(&1)
        );
        assert!(!report.is_qualified());
    }

    #[test]
    fn the_same_child_cannot_have_two_active_parents() {
        let modules = modules(&["CONTAINER_A", "CONTAINER_B", "PALLET"]);
        let events = vec![
            event("ADD-1", 10, "CONTAINER_A", "PALLET", MembershipAction::Add),
            event("ADD-2", 20, "CONTAINER_B", "PALLET", MembershipAction::Add),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert!(!report.is_qualified());
        assert_eq!(
            report.failures.get(&CompositionFailure::MultipleParents {
                child_module_id: "PALLET".to_string(),
                existing_parent_id: "CONTAINER_A".to_string(),
                proposed_parent_id: "CONTAINER_B".to_string(),
            }),
            Some(&1)
        );
        assert_eq!(report.active_edges(), 1);
    }

    #[test]
    fn child_can_be_disaggregated_then_reparented_without_identity_change() {
        let modules = modules(&["CONTAINER_A", "CONTAINER_B", "PALLET"]);
        let events = vec![
            event("ADD-A", 10, "CONTAINER_A", "PALLET", MembershipAction::Add),
            event(
                "DELETE-A",
                20,
                "CONTAINER_A",
                "PALLET",
                MembershipAction::Delete,
            ),
            event("ADD-B", 30, "CONTAINER_B", "PALLET", MembershipAction::Add),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert!(report.is_qualified());
        assert_eq!(
            report.active_memberships,
            BTreeSet::from([Membership {
                parent_module_id: "CONTAINER_B".to_string(),
                child_module_id: "PALLET".to_string(),
            }])
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::AllModuleIdentities)
                .unwrap(),
            3
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::RootHandlingUnits)
                .unwrap(),
            2
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::LeafHandlingUnits)
                .unwrap(),
            2
        );
    }

    #[test]
    fn missing_evidence_reference_blocks_composition_and_count_projection() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let mut missing = event("ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        missing.evidence_id.clear();
        let report = evaluate_composition(&modules, &[missing], 100).unwrap();

        assert!(!report.is_qualified());
        assert_eq!(
            report
                .failures
                .get(&CompositionFailure::MissingEvidenceReference),
            Some(&1)
        );
        assert_eq!(
            report.count_modules(ModuleCountBasis::AllModuleIdentities),
            Err(CompositionProjectionError::UnqualifiedComposition)
        );
    }

    #[test]
    fn historical_cutoff_replays_only_membership_events_visible_at_that_time() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let add = event("ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        let mut delete = event(
            "DELETE",
            20,
            "CONTAINER",
            "PALLET",
            MembershipAction::Delete,
        );
        delete.published_at = 30;

        let before_publication =
            evaluate_composition(&modules, &[add.clone(), delete.clone()], 20).unwrap();
        let after_publication = evaluate_composition(&modules, &[add, delete], 30).unwrap();

        assert_eq!(before_publication.active_edges(), 1);
        assert_eq!(after_publication.active_edges(), 0);
        assert!(before_publication.is_qualified());
        assert!(after_publication.is_qualified());
    }

    #[test]
    fn unmatched_delete_is_a_visible_failure() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let events = vec![event(
            "DEL-1",
            10,
            "CONTAINER",
            "PALLET",
            MembershipAction::Delete,
        )];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert!(!report.is_qualified());
        assert_eq!(
            report
                .failures
                .get(&CompositionFailure::UnmatchedDelete(Membership {
                    parent_module_id: "CONTAINER".to_string(),
                    child_module_id: "PALLET".to_string(),
                })),
            Some(&1)
        );
    }

    #[test]
    fn self_containment_is_rejected() {
        let modules = modules(&["CONTAINER"]);
        let events = vec![event(
            "ADD-1",
            10,
            "CONTAINER",
            "CONTAINER",
            MembershipAction::Add,
        )];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert_eq!(
            report.failures.get(&CompositionFailure::SelfContainment(
                "CONTAINER".to_string()
            )),
            Some(&1)
        );
    }

    #[test]
    fn cycles_are_rejected() {
        let modules = modules(&["A", "B", "C"]);
        let events = vec![
            event("ADD-1", 10, "A", "B", MembershipAction::Add),
            event("ADD-2", 20, "B", "C", MembershipAction::Add),
            event("ADD-3", 30, "C", "A", MembershipAction::Add),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert_eq!(
            report.failures.get(&CompositionFailure::Cycle(Membership {
                parent_module_id: "C".to_string(),
                child_module_id: "A".to_string(),
            })),
            Some(&1)
        );
        assert_eq!(report.active_edges(), 2);
    }

    #[test]
    fn same_time_events_sharing_a_module_are_not_lexically_ordered() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let events = vec![
            event("A-ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add),
            event("Z-ADD", 10, "PALLET", "CARTON", MembershipAction::Add),
        ];
        let report = evaluate_composition(&modules, &events, 100).unwrap();

        assert_eq!(
            report
                .failures
                .get(&CompositionFailure::AmbiguousEventTime {
                    event_time: 10,
                    module_id: "PALLET".to_string(),
                }),
            Some(&2)
        );
        assert_eq!(report.active_edges(), 0);
    }

    #[test]
    fn same_time_independent_aggregations_are_permutation_invariant() {
        let modules = modules(&["CONTAINER_A", "PALLET_A", "CONTAINER_B", "PALLET_B"]);
        let events = vec![
            event(
                "ADD-A",
                10,
                "CONTAINER_A",
                "PALLET_A",
                MembershipAction::Add,
            ),
            event(
                "ADD-B",
                10,
                "CONTAINER_B",
                "PALLET_B",
                MembershipAction::Add,
            ),
        ];
        let forward = evaluate_composition(&modules, &events, 100).unwrap();
        let mut reversed_events = events;
        reversed_events.reverse();
        let reversed = evaluate_composition(&modules, &reversed_events, 100).unwrap();

        assert_eq!(forward, reversed);
        assert!(forward.is_qualified());
        assert_eq!(forward.active_edges(), 2);
    }

    #[test]
    fn planned_and_future_events_do_not_change_composition() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let mut planned = event("PLAN", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        planned.classification = EventClassification::Planned;
        let mut future = event("FUTURE", 20, "PALLET", "CARTON", MembershipAction::Add);
        future.published_at = 101;
        let report = evaluate_composition(&modules, &[planned, future], 100).unwrap();

        assert_eq!(report.planned_event_rows, 1);
        assert_eq!(report.future_event_rows, 1);
        assert_eq!(report.active_edges(), 0);
    }

    #[test]
    fn latest_void_revision_suppresses_prior_addition() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let original = event("ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        let mut voided = original.clone();
        voided.revision = 2;
        voided.published_at = 30;
        voided.voided = true;
        let report = evaluate_composition(&modules, &[original, voided], 100).unwrap();

        assert_eq!(report.voided_event_rows, 1);
        assert_eq!(report.active_edges(), 0);
    }

    #[test]
    fn exact_duplicate_source_event_is_idempotent() {
        let modules = modules(&["CONTAINER", "PALLET"]);
        let original = event("ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        let report = evaluate_composition(&modules, &[original.clone(), original], 100).unwrap();

        assert_eq!(report.replayed_event_rows, 1);
        assert_eq!(report.active_edges(), 1);
        assert!(report.is_qualified());
    }

    #[test]
    fn conflicting_same_revision_rejects_the_snapshot() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let first = event("ADD", 10, "CONTAINER", "PALLET", MembershipAction::Add);
        let mut conflict = first.clone();
        conflict.child_module_id = "CARTON".to_string();

        assert_eq!(
            evaluate_composition(&modules, &[first, conflict], 100),
            Err(CompositionError::ConflictingEventRevision {
                source_key: SourceEventKey::new("WMS_A", "ADD"),
                revision: 1,
            })
        );
    }

    #[test]
    fn module_id_is_unique_and_unknown_nodes_fail_closed() {
        let duplicated = modules(&["A", "A"]);
        assert_eq!(
            evaluate_composition(&duplicated, &[], 100),
            Err(CompositionError::DuplicateModuleId("A".to_string()))
        );

        let known = modules(&["A"]);
        let report = evaluate_composition(
            &known,
            &[event("ADD", 10, "A", "UNKNOWN", MembershipAction::Add)],
            100,
        )
        .unwrap();
        assert_eq!(
            report
                .failures
                .get(&CompositionFailure::UnknownChild("UNKNOWN".to_string())),
            Some(&1)
        );
    }

    #[test]
    fn invalid_composition_does_not_emit_a_module_count() {
        let modules = modules(&["A", "B", "C"]);
        let report = evaluate_composition(
            &modules,
            &[
                event("ADD-1", 10, "A", "B", MembershipAction::Add),
                event("ADD-2", 20, "C", "B", MembershipAction::Add),
            ],
            100,
        )
        .unwrap();

        assert!(!report.is_qualified());
        assert_eq!(
            report.count_modules(ModuleCountBasis::AllModuleIdentities),
            Err(CompositionProjectionError::UnqualifiedComposition)
        );
    }

    #[test]
    fn composition_is_not_a_custody_or_inventory_delta() {
        let modules = modules(&["CONTAINER", "PALLET", "CARTON"]);
        let report = evaluate_composition(
            &modules,
            &[
                event("ADD-1", 10, "CONTAINER", "PALLET", MembershipAction::Add),
                event("ADD-2", 20, "PALLET", "CARTON", MembershipAction::Add),
            ],
            100,
        )
        .unwrap();

        assert_eq!(
            report
                .count_modules(ModuleCountBasis::AllModuleIdentities)
                .unwrap(),
            3
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::RootHandlingUnits)
                .unwrap(),
            1
        );
        assert_eq!(
            report
                .count_modules(ModuleCountBasis::LeafHandlingUnits)
                .unwrap(),
            1
        );
        // There is no inventory or custody mutation API in this module.
        assert!(report.is_qualified());
    }
}

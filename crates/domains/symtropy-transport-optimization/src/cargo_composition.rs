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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionError {
    DuplicateModuleId(String),
    ConflictingEventRevision {
        source_key: SourceEventKey,
        revision: u32,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompositionReport {
    pub input_event_rows: usize,
    pub visible_event_rows: usize,
    pub future_event_rows: usize,
    pub superseded_event_rows: usize,
    pub replayed_event_rows: usize,
    pub voided_event_rows: usize,
    pub planned_event_rows: usize,
    pub actual_event_rows: usize,
    pub active_memberships: BTreeSet<Membership>,
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

impl CompositionReport {
    pub fn is_qualified(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn active_edges(&self) -> usize {
        self.active_memberships.len()
    }

    pub fn count_modules(
        &self,
        modules: &[ModuleIdentity],
        basis: ModuleCountBasis,
    ) -> usize {
        let all_ids: BTreeSet<&str> = modules
            .iter()
            .map(|module| module.module_id.as_str())
            .collect();
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

        match basis {
            ModuleCountBasis::AllModuleIdentities => all_ids.len(),
            ModuleCountBasis::RootHandlingUnits => {
                all_ids.difference(&child_ids).count()
            }
            ModuleCountBasis::LeafHandlingUnits => {
                all_ids.difference(&parent_ids).count()
            }
        }
    }
}

fn record_failure(
    report: &mut CompositionReport,
    failure: CompositionFailure,
) {
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
    if event.parent_module_id.trim().is_empty() || event.child_module_id.trim().is_empty() {
        return Some(CompositionFailure::EmptyModuleIdentity);
    }
    if event.source_profile_id.trim().is_empty() || event.source_profile_version.trim().is_empty()
    {
        return Some(CompositionFailure::EmptySourceProfile);
    }
    if event.location_id.trim().is_empty() {
        return Some(CompositionFailure::EmptyLocation);
    }
    if event.evidence_id.trim().is_empty() {
        return Some(CompositionFailure::MissingEvidenceReference);
    }
    None
}

fn active_parent_of(
    active: &BTreeSet<Membership>,
    child_module_id: &str,
) -> Option<&str> {
    active
        .iter()
        .find(|membership| membership.child_module_id == child_module_id)
        .map(|membership| membership.parent_module_id.as_str())
}

fn creates_cycle(
    active: &BTreeSet<Membership>,
    proposed: &Membership,
) -> bool {
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
                record_failure(
                    report,
                    CompositionFailure::DuplicateMembership(membership),
                );
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
        if module.module_id.trim().is_empty() {
            return Err(CompositionError::DuplicateModuleId(
                module.module_id.clone(),
            ));
        }
        if !known_modules.insert(module.module_id.as_str()) {
            return Err(CompositionError::DuplicateModuleId(
                module.module_id.clone(),
            ));
        }
    }

    let mut report = CompositionReport {
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
                actual_by_time.entry(event.event_time).or_default().push(event);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn modules(ids: &[&str]) -> Vec<ModuleIdentity> {
        ids.iter()
            .map(|id| {
                ModuleIdentity::new(*id, CargoModuleFamily::Custom)
            })
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
            report.count_modules(&modules, ModuleCountBasis::AllModuleIdentities),
            4
        );
        assert_eq!(
            report.count_modules(&modules, ModuleCountBasis::RootHandlingUnits),
            1
        );
        assert_eq!(
            report.count_modules(&modules, ModuleCountBasis::LeafHandlingUnits),
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
            report.count_modules(&modules, ModuleCountBasis::RootHandlingUnits),
            2
        );
        assert_eq!(
            report.count_modules(&modules, ModuleCountBasis::LeafHandlingUnits),
            2
        );
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
            report
                .failures
                .get(&CompositionFailure::MultipleParents {
                    child_module_id: "PALLET".to_string(),
                    existing_parent_id: "CONTAINER_A".to_string(),
                    proposed_parent_id: "CONTAINER_B".to_string(),
                }),
            Some(&1)
        );
        assert_eq!(report.active_edges(), 1);
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
            report.failures.get(&CompositionFailure::UnmatchedDelete(Membership {
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
            report
                .failures
                .get(&CompositionFailure::SelfContainment("CONTAINER".to_string())),
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
            event("ADD-A", 10, "CONTAINER_A", "PALLET_A", MembershipAction::Add),
            event("ADD-B", 10, "CONTAINER_B", "PALLET_B", MembershipAction::Add),
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
        let report =
            evaluate_composition(&modules, &[original.clone(), original], 100).unwrap();

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
            report.count_modules(&modules, ModuleCountBasis::AllModuleIdentities),
            3
        );
        assert_eq!(
            report.count_modules(&modules, ModuleCountBasis::RootHandlingUnits),
            1
        );
        assert_eq!(
            report.count_modules(&modules, ModuleCountBasis::LeafHandlingUnits),
            1
        );
        // There is no inventory or custody mutation API in this module.
        assert!(report.is_qualified());
    }
}

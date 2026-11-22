// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Loss-aware projection from normalized EPCIS AggregationEvent meanings into
//! the existing instance-level cargo-composition reducer.
//!
//! This module is intentionally read-only. It emits CompositionEvent rows
//! only for instance-level ADD and child-specific DELETE. OBSERVE remains an
//! observation ledger, class QuantityElements remain a separate quantity
//! ledger, and DELETE_ALL remains an explicit parent-scoped disaggregation
//! command that the current v0.1 graph reducer cannot yet consume. A report
//! containing quarantined events or DELETE_ALL cannot emit composition counts.

use std::collections::{BTreeMap, BTreeSet};

use crate::cargo_composition::{
    CompositionError, CompositionEvent, CompositionProjectionError, CompositionReport,
    EventClassification, MembershipAction, ModuleCountBasis, ModuleIdentity, SourceEventKey,
    evaluate_composition,
};
use crate::epcis_aggregation::{
    AggregationEffect, NormalizedAggregationEvent, QuantityElement,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionTimeQualification {
    /// P1 evidence: the ID must be bound to the exact source-event revision.
    Qualified {
        time_basis_id: String,
        tick_quantum_id: String,
        time_evidence_id: String,
        earliest_event_time_ticks: i64,
        latest_event_time_ticks: i64,
    },
    /// P0 only: dimensionless fixture ticks, never physical clock chronology.
    SyntheticP0 {
        fixture_profile_id: String,
    },
    /// Missing / rejected / incomparable chronology; event is quarantined.
    Unqualified {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregationProjectionEnvelope {
    pub event: NormalizedAggregationEvent,
    pub revision: u32,
    pub published_at: i64,
    /// Already-normalized event-time ticks in the declared time domain.
    pub event_time_ticks: i64,
    pub classification: EventClassification,
    pub location_id: String,
    /// SHA-256 of the immutable original source payload bytes.
    pub source_payload_sha256: String,
    /// SHA-256 of the adapter/profile build used to normalize the event.
    pub adapter_build_sha256: String,
    pub time_qualification: ProjectionTimeQualification,
    /// Latest visible source revision can void a prior assertion.
    pub voided: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ProjectionSourceRef {
    pub source_id: String,
    pub source_event_id: String,
    pub revision: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantityAssertionAction {
    Add,
    Delete,
    Observe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedQuantityAssertion {
    pub source: ProjectionSourceRef,
    pub parent_id: Option<String>,
    pub action: QuantityAssertionAction,
    /// The exact source QuantityElement; never rounded, summed or expanded.
    pub quantity: QuantityElement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedObservation {
    pub source: ProjectionSourceRef,
    pub parent_id: Option<String>,
    pub observed_child_epcs: Vec<String>,
    pub observed_child_quantities: Vec<QuantityElement>,
    pub may_be_incomplete: bool,
    pub event_time_ticks: i64,
    pub time_qualification: ProjectionTimeQualification,
    pub source_payload_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParentScopedDisaggregation {
    pub source: ProjectionSourceRef,
    pub parent_id: String,
    pub event_time_ticks: i64,
    pub published_at: i64,
    pub classification: EventClassification,
    pub source_payload_sha256: String,
    pub time_qualification: ProjectionTimeQualification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipProjectionProvenance {
    pub source: ProjectionSourceRef,
    pub child_epc: String,
    /// Stable derived event key used by the existing one-child-per-event API.
    pub derived_source_event_id: String,
    pub action: MembershipAction,
    pub is_revision_tombstone: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedMembershipEvent {
    pub event: CompositionEvent,
    pub provenance: MembershipProjectionProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionQuarantineReason {
    UnqualifiedChronology(String),
    EventTimeEvidenceIdMismatch,
    InvalidTimeEnclosure,
    InvalidRevision,
    InvalidLocation,
    InvalidPayloadDigest,
    InvalidAdapterDigest,
    MissingParentForMembership,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuarantinedAggregationEvent {
    pub source: ProjectionSourceRef,
    pub reason: ProjectionQuarantineReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregationProjectionError {
    ConflictingSourceEventRevision {
        source: ProjectionSourceRef,
    },
    MixedChronologyDomains {
        first_domain: String,
        second_domain: String,
    },
    TimelineOrderNotProven {
        module_id: String,
        previous_event: ProjectionSourceRef,
        current_event: ProjectionSourceRef,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregationCompositionProjectionReport {
    pub as_of: i64,
    pub input_event_rows: usize,
    pub visible_event_rows: usize,
    pub future_event_rows: usize,
    pub superseded_event_rows: usize,
    pub exact_duplicate_replays: usize,
    pub voided_source_events: usize,
    /// Latest visible source envelopes, retaining raw time lexemes, recordTime,
    /// source profile and source/adapter evidence digests for independent replay.
    pub source_envelopes: Vec<AggregationProjectionEnvelope>,
    pub membership_events: Vec<ProjectedMembershipEvent>,
    pub observations: Vec<ProjectedObservation>,
    pub class_quantity_assertions: Vec<ProjectedQuantityAssertion>,
    pub parent_scoped_disaggregations: Vec<ParentScopedDisaggregation>,
    pub quarantined: Vec<QuarantinedAggregationEvent>,
    pub chronology_domain: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionEvaluationError {
    ProjectionNotComplete,
    Composition(CompositionError),
    CountsUnavailable(CompositionProjectionError),
}

impl AggregationCompositionProjectionReport {
    /// Counts are withheld while any selected source event is quarantined or
    /// any DELETE_ALL remains unconsumed by the current composition reducer.
    pub fn is_complete_for_instance_projection(&self) -> bool {
        self.quarantined.is_empty() && self.parent_scoped_disaggregations.is_empty()
    }

    /// Replay only the instance-level events supported by cargo composition.
    /// This does not consume class quantities, OBSERVE or DELETE_ALL.
    pub fn evaluate_instance_composition(
        &self,
        modules: &[ModuleIdentity],
    ) -> Result<CompositionReport, ProjectionEvaluationError> {
        if !self.is_complete_for_instance_projection() {
            return Err(ProjectionEvaluationError::ProjectionNotComplete);
        }
        let events: Vec<CompositionEvent> = self
            .membership_events
            .iter()
            .map(|projected| projected.event.clone())
            .collect();
        evaluate_composition(modules, &events, self.as_of)
            .map_err(ProjectionEvaluationError::Composition)
    }

    pub fn count_modules(
        &self,
        modules: &[ModuleIdentity],
        basis: ModuleCountBasis,
    ) -> Result<usize, ProjectionEvaluationError> {
        let report = self.evaluate_instance_composition(modules)?;
        report
            .count_modules(basis)
            .map_err(ProjectionEvaluationError::CountsUnavailable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum ChronologyDomain {
    Qualified {
        time_basis_id: String,
        tick_quantum_id: String,
    },
    SyntheticP0 {
        fixture_profile_id: String,
    },
}

fn canonical_event_time_evidence_id(source: &ProjectionSourceRef) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        source.source_id.len(),
        source.source_id,
        source.source_event_id.len(),
        source.source_event_id,
        source.revision
    )
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn chronology_domain(
    envelope: &AggregationProjectionEnvelope,
) -> Result<ChronologyDomain, ProjectionQuarantineReason> {
    match &envelope.time_qualification {
        ProjectionTimeQualification::Qualified {
            time_basis_id,
            tick_quantum_id,
            time_evidence_id,
        } => {
            let source = ProjectionSourceRef {
                source_id: envelope.event.source_id.clone(),
                source_event_id: envelope.event.source_event_id.clone(),
                revision: envelope.revision,
            };
            if time_basis_id.trim().is_empty() || tick_quantum_id.trim().is_empty() {
                return Err(ProjectionQuarantineReason::UnqualifiedChronology(
                    "time basis or tick quantum is empty".to_string(),
                ));
            }
            if time_evidence_id != &canonical_event_time_evidence_id(&source) {
                return Err(ProjectionQuarantineReason::EventTimeEvidenceIdMismatch);
            }
            if earliest_event_time_ticks > latest_event_time_ticks
                || envelope.event_time_ticks < earliest_event_time_ticks
                || envelope.event_time_ticks > latest_event_time_ticks
            {
                return Err(ProjectionQuarantineReason::InvalidTimeEnclosure);
            }
            Ok(ChronologyDomain::Qualified {
                time_basis_id: time_basis_id.clone(),
                tick_quantum_id: tick_quantum_id.clone(),
            })
        }
        ProjectionTimeQualification::SyntheticP0 { fixture_profile_id } => {
            if fixture_profile_id.trim().is_empty()
                || fixture_profile_id.trim() != fixture_profile_id
            {
                return Err(ProjectionQuarantineReason::UnqualifiedChronology(
                    "synthetic fixture profile is empty or padded".to_string(),
                ));
            }
            Ok(ChronologyDomain::SyntheticP0 {
                fixture_profile_id: fixture_profile_id.clone(),
            })
        }
        ProjectionTimeQualification::Unqualified { reason } => {
            Err(ProjectionQuarantineReason::UnqualifiedChronology(
                reason.clone(),
            ))
        }
    }
}

fn event_time_interval(
    envelope: &AggregationProjectionEnvelope,
) -> Option<(i64, i64)> {
    match &envelope.time_qualification {
        ProjectionTimeQualification::Qualified {
            earliest_event_time_ticks,
            latest_event_time_ticks,
            ..
        } => Some((*earliest_event_time_ticks, *latest_event_time_ticks)),
        ProjectionTimeQualification::SyntheticP0 { .. } => {
            Some((envelope.event_time_ticks, envelope.event_time_ticks))
        }
        ProjectionTimeQualification::Unqualified { .. } => None,
    }
}

fn modules_touched_by_event(event: &NormalizedAggregationEvent) -> BTreeSet<String> {
    let mut modules = BTreeSet::new();
    match &event.effect {
        AggregationEffect::Add {
            parent_id,
            child_epcs,
            ..
        }
        | AggregationEffect::Delete {
            parent_id,
            child_epcs,
            ..
        } => {
            modules.insert(parent_id.clone());
            modules.extend(child_epcs.iter().cloned());
        }
        AggregationEffect::Observe {
            parent_id,
            observed_child_epcs,
            ..
        } => {
            if let Some(parent_id) = parent_id {
                modules.insert(parent_id.clone());
            }
            modules.extend(observed_child_epcs.iter().cloned());
        }
        AggregationEffect::DeleteAll { parent_id } => {
            modules.insert(parent_id.clone());
        }
    }
    modules
}

fn verify_strict_module_timeline(
    envelopes: &[&AggregationProjectionEnvelope],
) -> Result<(), AggregationProjectionError> {
    let mut by_module: BTreeMap<String, Vec<(i64, i64, ProjectionSourceRef)>> = BTreeMap::new();
    for envelope in envelopes {
        if envelope.voided || envelope.classification != EventClassification::Actual {
            continue;
        }
        let Some((earliest, latest)) = event_time_interval(envelope) else {
            continue;
        };
        for module_id in modules_touched_by_event(&envelope.event) {
            by_module.entry(module_id).or_default().push((
                earliest,
                latest,
                source_ref(envelope),
            ));
        }
    }

    for (module_id, mut intervals) in by_module {
        intervals.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        let mut prefix_max: Option<(i64, ProjectionSourceRef)> = None;
        for (earliest, latest, source) in intervals {
            if let Some((previous_latest, previous_source)) = &prefix_max
                && *previous_latest >= earliest
            {
                return Err(AggregationProjectionError::TimelineOrderNotProven {
                    module_id,
                    previous_event: previous_source.clone(),
                    current_event: source,
                });
            }
            let replace = prefix_max
                .as_ref()
                .is_none_or(|(previous_latest, _)| latest > *previous_latest);
            if replace {
                prefix_max = Some((latest, source));
            }
        }
    }
    Ok(())
}

fn domain_label(domain: &ChronologyDomain) -> String {
    match domain {
        ChronologyDomain::Qualified {
            time_basis_id,
            tick_quantum_id,
        } => format!("qualified:{time_basis_id}:{tick_quantum_id}"),
        ChronologyDomain::SyntheticP0 { fixture_profile_id } => {
            format!("synthetic-p0:{fixture_profile_id}")
        }
    }
}

/// One stable graph-reducer event key per (originating source event, instance
/// child). Length-prefixing avoids delimiter ambiguity. The provenance sidecar
/// always carries the exact original EPCIS source key and revision.
fn derived_membership_event_id(source_event_id: &str, child_epc: &str) -> String {
    format!(
        "epcis-aggregation-child:{}:{}:{}:{}",
        source_event_id.len(),
        source_event_id,
        child_epc.len(),
        child_epc
    )
}

fn source_ref(envelope: &AggregationProjectionEnvelope) -> ProjectionSourceRef {
    ProjectionSourceRef {
        source_id: envelope.event.source_id.clone(),
        source_event_id: envelope.event.source_event_id.clone(),
        revision: envelope.revision,
    }
}

fn composition_event(
    envelope: &AggregationProjectionEnvelope,
    child_epc: &str,
    parent_id: &str,
    action: MembershipAction,
    voided: bool,
) -> ProjectedMembershipEvent {
    let source = source_ref(envelope);
    let derived = derived_membership_event_id(&source.source_event_id, child_epc);
    ProjectedMembershipEvent {
        event: CompositionEvent {
            source_key: SourceEventKey::new(&source.source_id, &derived),
            revision: source.revision,
            published_at: envelope.published_at,
            event_time: envelope.event_time_ticks,
            parent_module_id: parent_id.to_string(),
            child_module_id: child_epc.to_string(),
            action,
            classification: envelope.classification,
            source_profile_id: envelope.event.source_profile_id.clone(),
            source_profile_version: envelope.event.source_profile_version.clone(),
            location_id: envelope.location_id.clone(),
            evidence_id: envelope.source_payload_sha256.clone(),
            voided,
        },
        provenance: MembershipProjectionProvenance {
            source,
            child_epc: child_epc.to_string(),
            derived_source_event_id: derived,
            action,
            is_revision_tombstone: voided,
        },
    }
}

fn membership_children(event: &NormalizedAggregationEvent) -> BTreeMap<String, String> {
    match &event.effect {
        AggregationEffect::Add {
            parent_id,
            child_epcs,
            ..
        }
        | AggregationEffect::Delete {
            parent_id,
            child_epcs,
            ..
        } => child_epcs
            .iter()
            .map(|child| (child.clone(), parent_id.clone()))
            .collect(),
        AggregationEffect::Observe { .. } | AggregationEffect::DeleteAll { .. } => BTreeMap::new(),
    }
}

/// Convert a frozen set of normalized AggregationEvent revisions into a
/// replayable, loss-aware projection report.
///
/// Source revision selection is performed before projection. Superseded child
/// assertions are tombstoned under their stable derived event keys so a newer
/// source revision that removes a child cannot leave an older graph edge alive.
/// Actual events in one report must share one qualified time basis and tick
/// quantum, or one explicitly synthetic P0 fixture profile. No raw timestamp
/// or repository recordTime is used as a chronology fallback.
pub fn project_aggregation_events(
    envelopes: &[AggregationProjectionEnvelope],
    as_of: i64,
) -> Result<AggregationCompositionProjectionReport, AggregationProjectionError> {
    let mut report = AggregationCompositionProjectionReport {
        as_of,
        input_event_rows: envelopes.len(),
        visible_event_rows: 0,
        future_event_rows: 0,
        superseded_event_rows: 0,
        exact_duplicate_replays: 0,
        voided_source_events: 0,
        source_envelopes: Vec::new(),
        membership_events: Vec::new(),
        observations: Vec::new(),
        class_quantity_assertions: Vec::new(),
        parent_scoped_disaggregations: Vec::new(),
        quarantined: Vec::new(),
        chronology_domain: None,
    };

    let mut by_source: BTreeMap<SourceEventKey, Vec<&AggregationProjectionEnvelope>> =
        BTreeMap::new();
    for envelope in envelopes {
        if envelope.published_at <= as_of {
            report.visible_event_rows += 1;
            by_source
                .entry(SourceEventKey::new(
                    &envelope.event.source_id,
                    &envelope.event.source_event_id,
                ))
                .or_default()
                .push(envelope);
        } else {
            report.future_event_rows += 1;
        }
    }

    let mut effective: Vec<(&AggregationProjectionEnvelope, Vec<&AggregationProjectionEnvelope>)> =
        Vec::new();
    for (source_key, revisions) in by_source {
        let latest_revision = revisions
            .iter()
            .map(|envelope| envelope.revision)
            .max()
            .unwrap_or_default();
        let latest: Vec<&AggregationProjectionEnvelope> = revisions
            .iter()
            .copied()
            .filter(|envelope| envelope.revision == latest_revision)
            .collect();
        let first = latest[0];
        if latest.iter().any(|candidate| *candidate != first) {
            return Err(AggregationProjectionError::ConflictingSourceEventRevision {
                source: ProjectionSourceRef {
                    source_id: source_key.source_id,
                    source_event_id: source_key.source_event_id,
                    revision: latest_revision,
                },
            });
        }
        report.exact_duplicate_replays += latest.len().saturating_sub(1);
        report.superseded_event_rows += revisions.len().saturating_sub(latest.len());
        effective.push((first, revisions));
    }

    let mut common_domain: Option<ChronologyDomain> = None;
    for (envelope, _) in &effective {
        report.source_envelopes.push((*envelope).clone());
        if envelope.revision == 0 {
            report.quarantined.push(QuarantinedAggregationEvent {
                source: source_ref(envelope),
                reason: ProjectionQuarantineReason::InvalidRevision,
            });
            continue;
        }
        if envelope.voided || envelope.classification == EventClassification::Planned {
            continue;
        }
        match chronology_domain(envelope) {
            Ok(domain) => {
                if let Some(existing) = &common_domain {
                    if existing != &domain {
                        return Err(AggregationProjectionError::MixedChronologyDomains {
                            first_domain: domain_label(existing),
                            second_domain: domain_label(&domain),
                        });
                    }
                } else {
                    common_domain = Some(domain);
                }
            }
            Err(reason) => report.quarantined.push(QuarantinedAggregationEvent {
                source: source_ref(envelope),
                reason,
            }),
        }
    }
    report.chronology_domain = common_domain.as_ref().map(domain_label);

    let timeline_events: Vec<&AggregationProjectionEnvelope> = effective
        .iter()
        .map(|(envelope, _)| *envelope)
        .collect();
    verify_strict_module_timeline(&timeline_events)?;

    for (envelope, revisions) in effective {
        let source = source_ref(envelope);
        if envelope.voided {
            report.voided_source_events += 1;
        }

        let source_context_valid = envelope.revision != 0
            && envelope.location_id.trim() == envelope.location_id
            && !envelope.location_id.trim().is_empty()
            && valid_sha256(&envelope.source_payload_sha256)
            && valid_sha256(&envelope.adapter_build_sha256)
            && (envelope.voided
                || !matches!(envelope.classification, EventClassification::Actual)
                || chronology_domain(envelope).is_ok());

        if !source_context_valid {
            if !report.quarantined.iter().any(|item| item.source == source) {
                let reason = if envelope.revision == 0 {
                    ProjectionQuarantineReason::InvalidRevision
                } else if envelope.location_id.trim().is_empty()
                    || envelope.location_id.trim() != envelope.location_id
                {
                    ProjectionQuarantineReason::InvalidLocation
                } else if !valid_sha256(&envelope.source_payload_sha256) {
                    ProjectionQuarantineReason::InvalidPayloadDigest
                } else if !valid_sha256(&envelope.adapter_build_sha256) {
                    ProjectionQuarantineReason::InvalidAdapterDigest
                } else {
                    chronology_domain(envelope).err().unwrap_or(
                        ProjectionQuarantineReason::UnqualifiedChronology(
                            "event context is not admissible".to_string(),
                        ),
                    )
                };
                report.quarantined.push(QuarantinedAggregationEvent { source, reason });
            }
            continue;
        }

        let mut latest_children = membership_children(&envelope.event);
        if envelope.voided {
            latest_children.clear();
        }

        // A newer source revision replaces the earlier event in full. Create
        // explicit void tombstones for old per-child projection keys that are
        // no longer asserted as an instance membership by the latest revision.
        let mut previous_children: BTreeMap<String, (u32, String)> = BTreeMap::new();
        for previous in revisions
            .iter()
            .filter(|previous| previous.revision < envelope.revision && !previous.voided)
        {
            for (child, parent) in membership_children(&previous.event) {
                let replace = previous_children
                    .get(&child)
                    .is_none_or(|(revision, _)| previous.revision > *revision);
                if replace {
                    previous_children.insert(child, (previous.revision, parent));
                }
            }
        }
        for (child, (_, previous_parent)) in previous_children {
            if !latest_children.contains_key(&child) {
                let tombstone = composition_event(
                    envelope,
                    &child,
                    &previous_parent,
                    MembershipAction::Delete,
                    true,
                );
                report.membership_events.push(tombstone);
            }
        }

        if envelope.voided {
            continue;
        }

        match &envelope.event.effect {
            AggregationEffect::Add {
                parent_id,
                child_epcs,
                child_quantity_list,
            } => {
                for child in child_epcs {
                    report.membership_events.push(composition_event(
                        envelope,
                        child,
                        parent_id,
                        MembershipAction::Add,
                        false,
                    ));
                }
                for quantity in child_quantity_list {
                    report.class_quantity_assertions.push(ProjectedQuantityAssertion {
                        source: source.clone(),
                        parent_id: Some(parent_id.clone()),
                        action: QuantityAssertionAction::Add,
                        quantity: quantity.clone(),
                    });
                }
            }
            AggregationEffect::Observe {
                parent_id,
                observed_child_epcs,
                observed_child_quantities,
                may_be_incomplete,
            } => {
                report.observations.push(ProjectedObservation {
                    source: source.clone(),
                    parent_id: parent_id.clone(),
                    observed_child_epcs: observed_child_epcs.clone(),
                    observed_child_quantities: observed_child_quantities.clone(),
                    may_be_incomplete: *may_be_incomplete,
                    event_time_ticks: envelope.event_time_ticks,
                    time_qualification: envelope.time_qualification.clone(),
                    source_payload_sha256: envelope.source_payload_sha256.clone(),
                });
                for quantity in observed_child_quantities {
                    report.class_quantity_assertions.push(ProjectedQuantityAssertion {
                        source: source.clone(),
                        parent_id: parent_id.clone(),
                        action: QuantityAssertionAction::Observe,
                        quantity: quantity.clone(),
                    });
                }
            }
            AggregationEffect::Delete {
                parent_id,
                child_epcs,
                child_quantity_list,
            } => {
                for child in child_epcs {
                    report.membership_events.push(composition_event(
                        envelope,
                        child,
                        parent_id,
                        MembershipAction::Delete,
                        false,
                    ));
                }
                for quantity in child_quantity_list {
                    report.class_quantity_assertions.push(ProjectedQuantityAssertion {
                        source: source.clone(),
                        parent_id: Some(parent_id.clone()),
                        action: QuantityAssertionAction::Delete,
                        quantity: quantity.clone(),
                    });
                }
            }
            AggregationEffect::DeleteAll { parent_id } => {
                report.parent_scoped_disaggregations.push(ParentScopedDisaggregation {
                    source: source.clone(),
                    parent_id: parent_id.clone(),
                    event_time_ticks: envelope.event_time_ticks,
                    published_at: envelope.published_at,
                    classification: envelope.classification,
                    source_payload_sha256: envelope.source_payload_sha256.clone(),
                    time_qualification: envelope.time_qualification.clone(),
                });
            }
        }
    }

    report.source_envelopes.sort_by(|left, right| {
        left.event
            .source_id
            .cmp(&right.event.source_id)
            .then_with(|| left.event.source_event_id.cmp(&right.event.source_event_id))
            .then_with(|| left.revision.cmp(&right.revision))
    });
    report.membership_events.sort_by(|left, right| {
        left.event
            .source_key
            .cmp(&right.event.source_key)
            .then_with(|| left.event.revision.cmp(&right.event.revision))
    });
    report.observations.sort_by(|left, right| left.source.cmp(&right.source));
    report
        .class_quantity_assertions
        .sort_by(|left, right| left.source.cmp(&right.source));
    report
        .parent_scoped_disaggregations
        .sort_by(|left, right| left.source.cmp(&right.source));
    report.quarantined.sort_by(|left, right| left.source.cmp(&right.source));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epcis_aggregation::{
        AggregationAction, EpcisAggregationEvent, normalize_aggregation_event,
    };
    use crate::CargoModuleFamily;

    fn quantity(class_id: &str, value: Option<&str>, uom: Option<&str>) -> QuantityElement {
        QuantityElement {
            object_class_id: class_id.to_string(),
            quantity_lexeme: value.map(str::to_string),
            unit_of_measure: uom.map(str::to_string),
        }
    }

    fn envelope(
        id: &str,
        revision: u32,
        action: AggregationAction,
        parent: Option<&str>,
        children: &[&str],
        quantities: Vec<QuantityElement>,
    ) -> AggregationProjectionEnvelope {
        let event = EpcisAggregationEvent {
            source_id: "WMS_A".to_string(),
            source_event_id: id.to_string(),
            source_profile_id: "GS1_EPCIS".to_string(),
            source_profile_version: "2.0.1".to_string(),
            action,
            parent_id: parent.map(str::to_string),
            child_epcs: children.iter().map(|child| (*child).to_string()).collect(),
            child_quantity_list: quantities,
            event_time_lexeme: "2026-11-22T10:00:00+02:00".to_string(),
            event_time_zone_offset: "+02:00".to_string(),
            record_time_lexeme: Some("2026-11-22T08:00:04Z".to_string()),
        };
        let normalized = normalize_aggregation_event(&event).unwrap();
        let source_event = ProjectionSourceRef {
            source_id: event.source_id.clone(),
            source_event_id: event.source_event_id.clone(),
            revision,
        };
        AggregationProjectionEnvelope {
            event: normalized,
            revision,
            published_at: 100,
            event_time_ticks: 1_000,
            classification: EventClassification::Actual,
            location_id: "terminal-a".to_string(),
            source_payload_sha256: "a".repeat(64),
            adapter_build_sha256: "b".repeat(64),
            time_qualification: ProjectionTimeQualification::Qualified {
                time_basis_id: "UTC(ZA)".to_string(),
                tick_quantum_id: "SI_NANOSECOND".to_string(),
                time_evidence_id: canonical_event_time_evidence_id(&source_event),
                earliest_event_time_ticks: 990,
                latest_event_time_ticks: 1_010,
            },
            voided: false,
        }
    }

    fn modules() -> Vec<ModuleIdentity> {
        vec![
            ModuleIdentity::new("container-1", CargoModuleFamily::IsoSeries1Container),
            ModuleIdentity::new("pallet-1", CargoModuleFamily::Pallet),
            ModuleIdentity::new("carton-1", CargoModuleFamily::ReturnableCrate),
        ]
    }

    #[test]
    fn instance_add_projects_to_composition_and_class_quantity_stays_separate() {
        let input = envelope(
            "add-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            vec![quantity("class-carton", Some("12"), None)],
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(report.membership_events.len(), 1);
        assert_eq!(
            report.membership_events[0].event.action,
            MembershipAction::Add
        );
        assert_eq!(report.class_quantity_assertions.len(), 1);
        assert_eq!(
            report.class_quantity_assertions[0].quantity,
            quantity("class-carton", Some("12"), None)
        );
        let composition = report.evaluate_instance_composition(&modules()).unwrap();
        assert_eq!(composition.active_memberships.len(), 1);
    }

    #[test]
    fn observe_is_only_an_observation_and_does_not_create_membership() {
        let input = envelope(
            "observe-1",
            1,
            AggregationAction::Observe,
            None,
            &["pallet-1"],
            vec![quantity("class-carton", None, None)],
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert!(report.membership_events.is_empty());
        assert_eq!(report.observations.len(), 1);
        assert_eq!(report.observations[0].parent_id, None);
        assert_eq!(report.class_quantity_assertions[0].action, QuantityAssertionAction::Observe);
        assert!(report.is_complete_for_instance_projection());
    }

    #[test]
    fn child_specific_delete_projects_only_the_named_instance() {
        let input = envelope(
            "delete-1",
            1,
            AggregationAction::Delete,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(report.membership_events.len(), 1);
        assert_eq!(
            report.membership_events[0].event.action,
            MembershipAction::Delete
        );
        assert_eq!(report.membership_events[0].event.child_module_id, "pallet-1");
    }

    #[test]
    fn delete_all_remains_parent_scoped_and_blocks_counts_until_reducer_supports_it() {
        let input = envelope(
            "delete-all-1",
            1,
            AggregationAction::Delete,
            Some("container-1"),
            &[],
            Vec::new(),
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(report.membership_events.len(), 0);
        assert_eq!(report.parent_scoped_disaggregations.len(), 1);
        assert_eq!(
            report.parent_scoped_disaggregations[0].parent_id,
            "container-1"
        );
        assert_eq!(
            report.evaluate_instance_composition(&modules()),
            Err(ProjectionEvaluationError::ProjectionNotComplete)
        );
    }

    #[test]
    fn class_only_add_does_not_create_fake_module_identities() {
        let input = envelope(
            "class-only",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &[],
            vec![quantity("class-carton", Some("100"), None)],
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert!(report.membership_events.is_empty());
        assert_eq!(report.class_quantity_assertions.len(), 1);
        let composition = report.evaluate_instance_composition(&modules()).unwrap();
        assert_eq!(
            composition.count_modules(ModuleCountBasis::AllModuleIdentities),
            Ok(3)
        );
    }

    #[test]
    fn unknown_class_quantity_remains_unknown_and_never_zero() {
        let input = envelope(
            "unknown-q",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &[],
            vec![quantity("class-carton", None, None)],
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(report.class_quantity_assertions[0].quantity.quantity_lexeme, None);
    }

    #[test]
    fn repeated_quantity_entries_are_preserved_not_aggregated() {
        let input = envelope(
            "repeated-q",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &[],
            vec![
                quantity("class-carton", Some("2"), None),
                quantity("class-carton", Some("3"), None),
            ],
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(report.class_quantity_assertions.len(), 2);
        assert_eq!(
            report.class_quantity_assertions[0].quantity.quantity_lexeme.as_deref(),
            Some("2")
        );
        assert_eq!(
            report.class_quantity_assertions[1].quantity.quantity_lexeme.as_deref(),
            Some("3")
        );
    }

    #[test]
    fn exact_duplicate_source_revision_is_idempotent() {
        let input = envelope(
            "duplicate-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let report = project_aggregation_events(&[input.clone(), input], 200).unwrap();
        assert_eq!(report.exact_duplicate_replays, 1);
        assert_eq!(report.membership_events.len(), 1);
    }

    #[test]
    fn conflicting_same_revision_source_event_is_rejected() {
        let first = envelope(
            "conflict-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let second = envelope(
            "conflict-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["carton-1"],
            Vec::new(),
        );
        assert!(matches!(
            project_aggregation_events(&[first, second], 200),
            Err(AggregationProjectionError::ConflictingSourceEventRevision { .. })
        ));
    }

    #[test]
    fn source_revision_that_removes_a_child_emits_stable_void_tombstone() {
        let first = envelope(
            "revisioned-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1", "carton-1"],
            Vec::new(),
        );
        let second = envelope(
            "revisioned-1",
            2,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let report = project_aggregation_events(&[first, second], 200).unwrap();
        assert_eq!(report.membership_events.len(), 2);
        let tombstone = report
            .membership_events
            .iter()
            .find(|row| row.provenance.child_epc == "carton-1")
            .unwrap();
        assert!(tombstone.event.voided);
        assert!(tombstone.provenance.is_revision_tombstone);
        assert_eq!(tombstone.event.revision, 2);
    }

    #[test]
    fn source_revision_void_suppresses_prior_membership_projection() {
        let mut first = envelope(
            "revision-void-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let mut second = envelope(
            "revision-void-1",
            2,
            AggregationAction::Add,
            Some("container-1"),
            &[],
            Vec::new(),
        );
        second.voided = true;
        first.published_at = 10;
        second.published_at = 20;
        let report = project_aggregation_events(&[first, second], 200).unwrap();
        assert_eq!(report.membership_events.len(), 1);
        assert!(report.membership_events[0].event.voided);
    }

    #[test]
    fn future_revisions_do_not_supersede_visible_source_events() {
        let first = envelope(
            "future-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let mut future = envelope(
            "future-1",
            2,
            AggregationAction::Add,
            Some("container-1"),
            &["carton-1"],
            Vec::new(),
        );
        future.published_at = 300;
        let report = project_aggregation_events(&[first, future], 200).unwrap();
        assert_eq!(report.future_event_rows, 1);
        assert_eq!(report.superseded_event_rows, 0);
        assert_eq!(
            report.membership_events[0].provenance.child_epc,
            "pallet-1"
        );
    }

    #[test]
    fn unqualified_chronology_is_quarantined_and_blocks_count_projection() {
        let mut input = envelope(
            "unqualified-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        input.time_qualification = ProjectionTimeQualification::Unqualified {
            reason: "missing calibration evidence".to_string(),
        };
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(report.quarantined.len(), 1);
        assert!(report.membership_events.is_empty());
        assert_eq!(
            report.evaluate_instance_composition(&modules()),
            Err(ProjectionEvaluationError::ProjectionNotComplete)
        );
    }

    #[test]
    fn qualified_time_evidence_must_bind_exact_source_revision() {
        let mut input = envelope(
            "wrong-time-id",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        input.time_qualification = ProjectionTimeQualification::Qualified {
            time_basis_id: "UTC(ZA)".to_string(),
            tick_quantum_id: "SI_NANOSECOND".to_string(),
            time_evidence_id: "another-event".to_string(),
            earliest_event_time_ticks: 990,
            latest_event_time_ticks: 1_010,
        };
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(
            report.quarantined[0].reason,
            ProjectionQuarantineReason::EventTimeEvidenceIdMismatch
        );
    }

    #[test]
    fn mixed_actual_time_bases_or_tick_quanta_reject_the_projection_batch() {
        let first = envelope(
            "time-a",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let mut second = envelope(
            "time-b",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["carton-1"],
            Vec::new(),
        );
        second.time_qualification = ProjectionTimeQualification::Qualified {
            time_basis_id: "UTC(ZA)".to_string(),
            tick_quantum_id: "TEN_NANOSECONDS".to_string(),
            time_evidence_id: canonical_event_time_evidence_id(&source_ref(&second)),
            earliest_event_time_ticks: 990,
            latest_event_time_ticks: 1_010,
        };
        assert!(matches!(
            project_aggregation_events(&[first, second], 200),
            Err(AggregationProjectionError::MixedChronologyDomains { .. })
        ));
    }

    #[test]
    fn synthetic_p0_and_qualified_p1_actual_events_cannot_be_mixed() {
        let first = envelope(
            "synthetic",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let mut second = envelope(
            "qualified",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["carton-1"],
            Vec::new(),
        );
        second.time_qualification = ProjectionTimeQualification::SyntheticP0 {
            fixture_profile_id: "fixture-v1".to_string(),
        };
        assert!(matches!(
            project_aggregation_events(&[first, second], 200),
            Err(AggregationProjectionError::MixedChronologyDomains { .. })
        ));
    }

    #[test]
    fn projection_is_deterministic_under_source_arrival_permutation() {
        let first = envelope(
            "permutation-a",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let second = envelope(
            "permutation-b",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["carton-1"],
            Vec::new(),
        );
        let forward = project_aggregation_events(&[first.clone(), second.clone()], 200).unwrap();
        let reverse = project_aggregation_events(&[second, first], 200).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn instance_composition_count_projections_remain_distinct() {
        let input = envelope(
            "nested-1",
            1,
            AggregationAction::Add,
            Some("container-1"),
            &["pallet-1"],
            Vec::new(),
        );
        let report = project_aggregation_events(&[input], 200).unwrap();
        assert_eq!(
            report.count_modules(&modules(), ModuleCountBasis::AllModuleIdentities),
            Ok(3)
        );
        assert_eq!(
            report.count_modules(&modules(), ModuleCountBasis::RootHandlingUnits),
            Ok(2)
        );
        assert_eq!(
            report.count_modules(&modules(), ModuleCountBasis::LeafHandlingUnits),
            Ok(2)
        );
    }
}

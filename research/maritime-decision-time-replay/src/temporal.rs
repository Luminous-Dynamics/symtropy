//! Tri-temporal projections for maritime evidence.
//!
//! Three timelines are deliberately kept separate:
//! - valid time: when a claim applies in the physical domain;
//! - receiver knowledge time: publication AND receipt by a decision cutoff;
//! - adjudication time: when an independent assessment was recorded.
//!
//! A later correction or adjudication can change a later projection, but does
//! not mutate the result of an earlier decision-time projection.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalCutoffs {
    pub physical_at: i64,
    pub decision_at: i64,
    pub audit_at: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdjudicationState {
    Accepted,
    Rejected,
    Disputed,
    NotEvaluable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdjudicationRevision {
    pub revision: u32,
    pub recorded_at: i64,
    pub state: AdjudicationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalEvidenceRevision {
    pub source_id: String,
    pub event_id: String,
    pub subject_id: String,
    pub source_revision: u32,
    pub payload_digest: String,
    /// Provenance label only. Distinct labels do not establish independence.
    pub dependency_cluster: String,
    pub valid_from: i64,
    pub valid_until: i64,
    pub published_at: i64,
    pub retrieved_at: i64,
    pub rights_clear: bool,
    pub rights_valid_from: i64,
    pub rights_valid_until: i64,
    pub adjudications: Vec<AdjudicationRevision>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdjudicationAsOf {
    NotYetAdjudicated,
    Recorded(AdjudicationState),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalProjection {
    pub source_id: String,
    pub event_id: String,
    pub subject_id: String,
    pub source_revision: u32,
    pub payload_digest: String,
    pub dependency_cluster: String,
    pub valid_at_physical_time: bool,
    pub published_by_decision: bool,
    pub retrieved_by_decision: bool,
    pub rights_valid_at_decision: bool,
    pub decision_admissible: bool,
    pub adjudication_as_of: AdjudicationAsOf,
    pub adjudication_revision_as_of: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalError {
    ConflictingSourceRevision {
        source_id: String,
        event_id: String,
        revision: u32,
    },
    ConflictingAdjudicationRevision {
        source_id: String,
        event_id: String,
        revision: u32,
    },
    SubjectBindingMismatch {
        source_id: String,
        event_id: String,
        expected_subject: String,
        observed_subject: String,
    },
}

fn latest_adjudication_as_of(
    source_id: &str,
    event_id: &str,
    adjudications: &[AdjudicationRevision],
    audit_at: i64,
) -> Result<(AdjudicationAsOf, Option<u32>), TemporalError> {
    let mut latest: Option<AdjudicationRevision> = None;

    for candidate in adjudications
        .iter()
        .filter(|candidate| candidate.recorded_at <= audit_at)
    {
        match latest {
            None => latest = Some(*candidate),
            Some(current) if candidate.revision > current.revision => {
                latest = Some(*candidate);
            }
            Some(current) if candidate.revision == current.revision => {
                if candidate != &current {
                    return Err(TemporalError::ConflictingAdjudicationRevision {
                        source_id: source_id.to_owned(),
                        event_id: event_id.to_owned(),
                        revision: candidate.revision,
                    });
                }
            }
            Some(_) => {}
        }
    }

    Ok(match latest {
        Some(value) => (
            AdjudicationAsOf::Recorded(value.state),
            Some(value.revision),
        ),
        None => (AdjudicationAsOf::NotYetAdjudicated, None),
    })
}

fn decision_admissible(record: &TemporalEvidenceRevision, cutoffs: TemporalCutoffs) -> bool {
    record.valid_from <= cutoffs.physical_at
        && cutoffs.physical_at <= record.valid_until
        && record.published_at <= cutoffs.decision_at
        && record.retrieved_at <= cutoffs.decision_at
        && record.rights_clear
        && record.rights_valid_from <= cutoffs.decision_at
        && cutoffs.decision_at <= record.rights_valid_until
}

/// Computes independent valid-time, receiver-knowledge and audit projections
/// for one source revision. The physical target and the two knowledge cutoffs
/// are separate inputs and must never be substituted for each other.
pub fn project_temporal_revision(
    record: &TemporalEvidenceRevision,
    cutoffs: TemporalCutoffs,
) -> Result<TemporalProjection, TemporalError> {
    let valid_at_physical_time =
        record.valid_from <= cutoffs.physical_at && cutoffs.physical_at <= record.valid_until;
    let published_by_decision = record.published_at <= cutoffs.decision_at;
    let retrieved_by_decision = record.retrieved_at <= cutoffs.decision_at;
    let rights_valid_at_decision = record.rights_clear
        && record.rights_valid_from <= cutoffs.decision_at
        && cutoffs.decision_at <= record.rights_valid_until;
    let decision_admissible = decision_admissible(record, cutoffs);

    let (adjudication_as_of, adjudication_revision_as_of) =
        latest_adjudication_as_of(
            &record.source_id,
            &record.event_id,
            &record.adjudications,
            cutoffs.audit_at,
        )?;

    Ok(TemporalProjection {
        source_id: record.source_id.clone(),
        event_id: record.event_id.clone(),
        subject_id: record.subject_id.clone(),
        source_revision: record.source_revision,
        payload_digest: record.payload_digest.clone(),
        dependency_cluster: record.dependency_cluster.clone(),
        valid_at_physical_time,
        published_by_decision,
        retrieved_by_decision,
        rights_valid_at_decision,
        decision_admissible,
        adjudication_as_of,
        adjudication_revision_as_of,
    })
}

/// Selects the latest source revision per event that was valid and actually
/// available to the receiver at the historical decision cutoff. Revisions
/// published or retrieved later are excluded, even if they are known to the
/// auditor today. Conflicting payloads under one visible (event, revision)
/// identity fail closed. This policy projection never reads adjudication state.
pub fn select_decision_revisions(
    records: &[TemporalEvidenceRevision],
    expected_subject_id: &str,
    cutoffs: TemporalCutoffs,
) -> Result<Vec<&TemporalEvidenceRevision>, TemporalError> {
    let mut selected: BTreeMap<(&str, &str), &TemporalEvidenceRevision> = BTreeMap::new();
    let mut visible_identities: BTreeMap<((&str, &str), u32), &str> = BTreeMap::new();

    for record in records {
        if !decision_admissible(record, cutoffs) {
            continue;
        }
        if record.subject_id != expected_subject_id {
            return Err(TemporalError::SubjectBindingMismatch {
                source_id: record.source_id.clone(),
                event_id: record.event_id.clone(),
                expected_subject: expected_subject_id.to_owned(),
                observed_subject: record.subject_id.clone(),
            });
        }

        let source_event = (record.source_id.as_str(), record.event_id.as_str());
        let identity = (source_event, record.source_revision);
        match visible_identities.get(&identity) {
            Some(existing_digest) if *existing_digest != record.payload_digest => {
                return Err(TemporalError::ConflictingSourceRevision {
                    source_id: record.source_id.clone(),
                    event_id: record.event_id.clone(),
                    revision: record.source_revision,
                });
            }
            Some(_) => continue,
            None => {
                visible_identities.insert(identity, record.payload_digest.as_str());
            }
        }

        match selected.get(&source_event) {
            Some(current) if current.source_revision >= record.source_revision => {}
            _ => {
                selected.insert(source_event, record);
            }
        }
    }

    Ok(selected.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        revision: u32,
        digest: &str,
        published: i64,
        retrieved: i64,
    ) -> TemporalEvidenceRevision {
        TemporalEvidenceRevision {
            source_id: "SOURCE-A".to_owned(),
            event_id: "EV-1".to_owned(),
            subject_id: "BOX-1".to_owned(),
            source_revision: revision,
            payload_digest: digest.to_owned(),
            dependency_cluster: "carrier-portal-cluster".to_owned(),
            valid_from: 80,
            valid_until: 200,
            published_at: published,
            retrieved_at: retrieved,
            rights_clear: true,
            rights_valid_from: 0,
            rights_valid_until: 500,
            adjudications: Vec::new(),
        }
    }

    fn cutoffs(physical_at: i64, decision_at: i64, audit_at: i64) -> TemporalCutoffs {
        TemporalCutoffs {
            physical_at,
            decision_at,
            audit_at,
        }
    }

    #[test]
    fn late_correction_does_not_rewrite_historical_decision_revision() {
        let first = record(1, "digest-v1", 90, 95);
        let correction = record(2, "digest-v2", 120, 130);
        let history = vec![first, correction];

        let then = select_decision_revisions(&history, "BOX-1", cutoffs(100, 100, 200)).unwrap();
        assert_eq!(then.len(), 1);
        assert_eq!(then[0].source_revision, 1);
        assert_eq!(then[0].payload_digest, "digest-v1");

        let later = select_decision_revisions(&history, "BOX-1", cutoffs(100, 150, 200)).unwrap();
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].source_revision, 2);
        assert_eq!(later[0].payload_digest, "digest-v2");
    }

    #[test]
    fn event_validity_publication_and_retrieval_are_independent() {
        let mut value = record(1, "digest-v1", 90, 120);
        let projection = project_temporal_revision(&value, cutoffs(100, 100, 200)).unwrap();

        assert!(projection.valid_at_physical_time);
        assert!(projection.published_by_decision);
        assert!(!projection.retrieved_by_decision);
        assert!(!projection.decision_admissible);

        value.retrieved_at = 99;
        let retrieved = project_temporal_revision(&value, cutoffs(100, 100, 200)).unwrap();
        assert!(retrieved.decision_admissible);
    }

    #[test]
    fn future_plan_can_be_known_before_its_valid_time() {
        let mut future = record(1, "planned", 90, 95);
        future.valid_from = 120;
        future.valid_until = 180;

        let projection = project_temporal_revision(&future, cutoffs(120, 100, 200)).unwrap();
        assert!(projection.valid_at_physical_time);
        assert!(projection.decision_admissible);
    }

    #[test]
    fn later_adjudication_changes_audit_view_not_decision_admissibility() {
        let mut value = record(1, "digest-v1", 90, 95);
        value.adjudications.push(AdjudicationRevision {
            revision: 1,
            recorded_at: 150,
            state: AdjudicationState::Accepted,
        });

        let before_audit = project_temporal_revision(&value, cutoffs(100, 100, 140)).unwrap();
        let after_audit = project_temporal_revision(&value, cutoffs(100, 100, 160)).unwrap();

        assert!(before_audit.decision_admissible);
        assert_eq!(
            before_audit.adjudication_as_of,
            AdjudicationAsOf::NotYetAdjudicated
        );
        assert!(after_audit.decision_admissible);
        assert_eq!(
            after_audit.adjudication_as_of,
            AdjudicationAsOf::Recorded(AdjudicationState::Accepted)
        );
        assert_eq!(before_audit.source_revision, after_audit.source_revision);
    }

    #[test]
    fn physical_validity_and_rights_each_gate_decision_admissibility() {
        let mut value = record(1, "digest-v1", 90, 95);
        value.valid_until = 99;
        let not_valid = project_temporal_revision(&value, cutoffs(100, 100, 200)).unwrap();
        assert!(!not_valid.valid_at_physical_time);
        assert!(!not_valid.decision_admissible);

        value.valid_until = 200;
        value.rights_valid_until = 99;
        let no_rights = project_temporal_revision(&value, cutoffs(100, 100, 200)).unwrap();
        assert!(no_rights.valid_at_physical_time);
        assert!(!no_rights.rights_valid_at_decision);
        assert!(!no_rights.decision_admissible);
    }

    #[test]
    fn conflicting_visible_payloads_for_same_source_revision_fail_closed() {
        let first = record(1, "digest-a", 90, 95);
        let conflicting = record(1, "digest-b", 91, 96);
        let error = select_decision_revisions(&[first, conflicting], "BOX-1", cutoffs(100, 100, 200))
            .unwrap_err();

        assert_eq!(
            error,
            TemporalError::ConflictingSourceRevision {
                source_id: "SOURCE-A".to_owned(),
                event_id: "EV-1".to_owned(),
                revision: 1,
            }
        );
    }

    #[test]
    fn same_native_event_id_from_distinct_sources_remains_distinct() {
        let first = record(1, "digest-a", 90, 95);
        let mut second = record(1, "digest-b", 91, 96);
        second.source_id = "SOURCE-B".to_owned();

        let selected =
            select_decision_revisions(&[first, second], "BOX-1", cutoffs(100, 100, 200))
                .unwrap();
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().any(|item| item.source_id == "SOURCE-A"));
        assert!(selected.iter().any(|item| item.source_id == "SOURCE-B"));
        assert_eq!(selected[0].dependency_cluster, selected[1].dependency_cluster);
    }

    #[test]
    fn visible_source_record_with_wrong_subject_is_rejected() {
        let mut value = record(1, "digest-v1", 90, 95);
        value.subject_id = "BOX-OTHER".to_owned();

        let error = select_decision_revisions(&[value], "BOX-1", cutoffs(100, 100, 200))
            .unwrap_err();
        assert_eq!(
            error,
            TemporalError::SubjectBindingMismatch {
                source_id: "SOURCE-A".to_owned(),
                event_id: "EV-1".to_owned(),
                expected_subject: "BOX-1".to_owned(),
                observed_subject: "BOX-OTHER".to_owned(),
            }
        );
    }

    #[test]
    fn decision_projection_does_not_depend_on_later_audit_conflicts() {
        let mut value = record(1, "digest-v1", 90, 95);
        value.adjudications = vec![
            AdjudicationRevision {
                revision: 1,
                recorded_at: 150,
                state: AdjudicationState::Accepted,
            },
            AdjudicationRevision {
                revision: 1,
                recorded_at: 151,
                state: AdjudicationState::Rejected,
            },
        ];

        let selected = select_decision_revisions(&[value], "BOX-1", cutoffs(100, 100, 200)).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].source_revision, 1);
    }

    #[test]
    fn adjudication_revisions_are_also_immutable() {
        let mut value = record(1, "digest-v1", 90, 95);
        value.adjudications = vec![
            AdjudicationRevision {
                revision: 1,
                recorded_at: 150,
                state: AdjudicationState::Accepted,
            },
            AdjudicationRevision {
                revision: 1,
                recorded_at: 151,
                state: AdjudicationState::Rejected,
            },
        ];

        let error = project_temporal_revision(&value, cutoffs(100, 100, 200)).unwrap_err();
        assert_eq!(
            error,
            TemporalError::ConflictingAdjudicationRevision {
                source_id: "SOURCE-A".to_owned(),
                event_id: "EV-1".to_owned(),
                revision: 1,
            }
        );
    }
}

//! Conservative partial ordering for source-qualified event-time enclosures.
//!
//! This module does not parse provider timestamps or establish that a clock
//! bound is sound. Callers must normalize event times into one explicitly
//! identified time basis and qualify the enclosure before marking it
//! QualifiedEnclosure. Incomparable bases and overlapping/touching intervals
//! remain unresolved rather than being arbitrarily ordered.

use std::collections::BTreeMap;

/// A closed event-time interval expressed in caller-normalized integer ticks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeInterval {
    earliest: i64,
    latest: i64,
}

impl TimeInterval {
    pub fn new(earliest: i64, latest: i64) -> Result<Self, TemporalOrderError> {
        if earliest > latest {
            return Err(TemporalOrderError::ReversedInterval);
        }
        Ok(Self { earliest, latest })
    }

    pub fn earliest(&self) -> i64 {
        self.earliest
    }

    pub fn latest(&self) -> i64 {
        self.latest
    }
}

/// Whether the caller supplies a qualified enclosure or an estimate only.
///
/// QualifiedEnclosure is an upstream claim. This module checks order
/// conditional on that claim; it does not verify the clock calibration,
/// traceability or physical event-time measurement that justifies it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeBoundKind {
    QualifiedEnclosure,
    EstimateOnly,
}

/// Event-time evidence normalized into a declared, exact time-basis identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeEvidence {
    event_id: String,
    time_basis_id: String,
    interval: TimeInterval,
    bound_kind: TimeBoundKind,
}

impl TimeEvidence {
    pub fn new(
        event_id: impl Into<String>,
        time_basis_id: impl Into<String>,
        interval: TimeInterval,
        bound_kind: TimeBoundKind,
    ) -> Result<Self, TemporalOrderError> {
        let event_id = event_id.into();
        let time_basis_id = time_basis_id.into();

        if event_id.trim().is_empty() {
            return Err(TemporalOrderError::EmptyEventId);
        }
        if event_id.trim() != event_id {
            return Err(TemporalOrderError::UntrimmedEventId);
        }
        if time_basis_id.trim().is_empty() {
            return Err(TemporalOrderError::EmptyTimeBasisId);
        }
        if time_basis_id.trim() != time_basis_id {
            return Err(TemporalOrderError::UntrimmedTimeBasisId);
        }

        Ok(Self {
            event_id,
            time_basis_id,
            interval,
            bound_kind,
        })
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn time_basis_id(&self) -> &str {
        &self.time_basis_id
    }

    pub fn interval(&self) -> TimeInterval {
        self.interval
    }

    pub fn bound_kind(&self) -> TimeBoundKind {
        self.bound_kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalOrderError {
    ReversedInterval,
    EmptyEventId,
    UntrimmedEventId,
    EmptyTimeBasisId,
    UntrimmedTimeBasisId,
    DuplicateEventId(String),
}

/// Relation of the first event to the second event.
///
/// An overlap, including a shared endpoint or equal singleton intervals, is
/// not a simultaneity certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalRelation {
    ProvenBefore,
    ProvenAfter,
    EstimatedBefore,
    EstimatedAfter,
    UnresolvedOverlap,
    IncomparableTimeBasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventPairOrder {
    /// Lexicographically first event ID; this is only a stable report key.
    pub first_event_id: String,
    pub second_event_id: String,
    /// Relation of first_event_id to second_event_id.
    pub relation: TemporalRelation,
}

pub fn compare_time_evidence(first: &TimeEvidence, second: &TimeEvidence) -> TemporalRelation {
    if first.time_basis_id != second.time_basis_id {
        return TemporalRelation::IncomparableTimeBasis;
    }

    let both_qualified = first.bound_kind == TimeBoundKind::QualifiedEnclosure
        && second.bound_kind == TimeBoundKind::QualifiedEnclosure;

    if first.interval.latest < second.interval.earliest {
        if both_qualified {
            TemporalRelation::ProvenBefore
        } else {
            TemporalRelation::EstimatedBefore
        }
    } else if second.interval.latest < first.interval.earliest {
        if both_qualified {
            TemporalRelation::ProvenAfter
        } else {
            TemporalRelation::EstimatedAfter
        }
    } else {
        TemporalRelation::UnresolvedOverlap
    }
}

/// Build a deterministic pairwise partial-order report.
///
/// This intentionally returns pairwise relations, not a fabricated total
/// ordering. Input permutation does not change the report because event IDs
/// are validated and sorted before comparison.
pub fn build_partial_order(
    events: &[TimeEvidence],
) -> Result<Vec<EventPairOrder>, TemporalOrderError> {
    let mut by_id: BTreeMap<&str, &TimeEvidence> = BTreeMap::new();

    for event in events {
        if event.event_id.trim().is_empty() {
            return Err(TemporalOrderError::EmptyEventId);
        }
        if event.event_id.trim() != event.event_id {
            return Err(TemporalOrderError::UntrimmedEventId);
        }
        if event.time_basis_id.trim().is_empty() {
            return Err(TemporalOrderError::EmptyTimeBasisId);
        }
        if event.time_basis_id.trim() != event.time_basis_id {
            return Err(TemporalOrderError::UntrimmedTimeBasisId);
        }
        if by_id.insert(&event.event_id, event).is_some() {
            return Err(TemporalOrderError::DuplicateEventId(event.event_id.clone()));
        }
    }

    let ordered: Vec<&TimeEvidence> = by_id.into_values().collect();
    let mut pairs = Vec::new();

    for first_index in 0..ordered.len() {
        for second_index in (first_index + 1)..ordered.len() {
            let first = ordered[first_index];
            let second = ordered[second_index];
            pairs.push(EventPairOrder {
                first_event_id: first.event_id.clone(),
                second_event_id: second.event_id.clone(),
                relation: compare_time_evidence(first, second),
            });
        }
    }

    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualified(id: &str, earliest: i64, latest: i64) -> TimeEvidence {
        TimeEvidence::new(
            id,
            "UTC_NS_PROFILE_V1",
            TimeInterval::new(earliest, latest).unwrap(),
            TimeBoundKind::QualifiedEnclosure,
        )
        .unwrap()
    }

    fn estimated(id: &str, earliest: i64, latest: i64) -> TimeEvidence {
        TimeEvidence::new(
            id,
            "UTC_NS_PROFILE_V1",
            TimeInterval::new(earliest, latest).unwrap(),
            TimeBoundKind::EstimateOnly,
        )
        .unwrap()
    }

    #[test]
    fn disjoint_qualified_enclosures_prove_strict_order() {
        assert_eq!(
            compare_time_evidence(&qualified("release", 10, 19), &qualified("accept", 20, 30)),
            TemporalRelation::ProvenBefore
        );
    }

    #[test]
    fn reverse_disjoint_qualified_enclosures_prove_reverse_order() {
        assert_eq!(
            compare_time_evidence(&qualified("accept", 20, 30), &qualified("release", 10, 19)),
            TemporalRelation::ProvenAfter
        );
    }

    #[test]
    fn overlapping_enclosures_remain_unresolved() {
        assert_eq!(
            compare_time_evidence(&qualified("a", 10, 20), &qualified("b", 15, 25)),
            TemporalRelation::UnresolvedOverlap
        );
    }

    #[test]
    fn touching_endpoints_remain_unresolved() {
        assert_eq!(
            compare_time_evidence(&qualified("a", 10, 20), &qualified("b", 20, 30)),
            TemporalRelation::UnresolvedOverlap
        );
    }

    #[test]
    fn identical_singleton_times_are_not_a_simultaneity_certificate() {
        assert_eq!(
            compare_time_evidence(&qualified("a", 20, 20), &qualified("b", 20, 20)),
            TemporalRelation::UnresolvedOverlap
        );
    }

    #[test]
    fn disjoint_estimates_produce_estimated_not_proven_order() {
        assert_eq!(
            compare_time_evidence(&estimated("a", 10, 19), &estimated("b", 20, 30)),
            TemporalRelation::EstimatedBefore
        );
    }

    #[test]
    fn mixed_qualified_and_estimated_bounds_are_not_proof_grade() {
        assert_eq!(
            compare_time_evidence(&qualified("a", 10, 19), &estimated("b", 20, 30)),
            TemporalRelation::EstimatedBefore
        );
    }

    #[test]
    fn different_time_bases_are_incomparable() {
        let first = qualified("a", 10, 19);
        let second = TimeEvidence::new(
            "b",
            "DEVICE_MONOTONIC_CLOCK_7",
            TimeInterval::new(20, 30).unwrap(),
            TimeBoundKind::QualifiedEnclosure,
        )
        .unwrap();

        assert_eq!(
            compare_time_evidence(&first, &second),
            TemporalRelation::IncomparableTimeBasis
        );
    }

    #[test]
    fn reversed_interval_is_rejected() {
        assert_eq!(
            TimeInterval::new(20, 10),
            Err(TemporalOrderError::ReversedInterval)
        );
    }

    #[test]
    fn event_evidence_rejects_empty_or_untrimmed_ids() {
        let interval = TimeInterval::new(1, 2).unwrap();
        assert_eq!(
            TimeEvidence::new("", "UTC", interval, TimeBoundKind::QualifiedEnclosure),
            Err(TemporalOrderError::EmptyEventId)
        );
        assert_eq!(
            TimeEvidence::new(" event", "UTC", interval, TimeBoundKind::QualifiedEnclosure),
            Err(TemporalOrderError::UntrimmedEventId)
        );
        assert_eq!(
            TimeEvidence::new("event", "", interval, TimeBoundKind::QualifiedEnclosure),
            Err(TemporalOrderError::EmptyTimeBasisId)
        );
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let events = vec![qualified("same", 1, 2), qualified("same", 3, 4)];
        assert_eq!(
            build_partial_order(&events),
            Err(TemporalOrderError::DuplicateEventId("same".to_string()))
        );
    }

    #[test]
    fn report_is_invariant_to_input_permutation() {
        let events = vec![
            qualified("c", 30, 40),
            qualified("a", 0, 10),
            qualified("b", 15, 25),
        ];
        let forward = build_partial_order(&events).unwrap();
        let mut reversed_events = events;
        reversed_events.reverse();
        let reversed = build_partial_order(&reversed_events).unwrap();

        assert_eq!(forward, reversed);
        assert_eq!(forward.len(), 3);
    }

    #[test]
    fn positive_common_time_scale_preserves_relation() {
        let original = compare_time_evidence(&qualified("a", 10, 19), &qualified("b", 20, 30));
        let scaled =
            compare_time_evidence(&qualified("a", 1_000, 1_900), &qualified("b", 2_000, 3_000));

        assert_eq!(original, TemporalRelation::ProvenBefore);
        assert_eq!(scaled, original);
    }

    #[test]
    fn partial_order_preserves_only_proven_pairs() {
        let events = vec![
            qualified("a", 0, 10),
            qualified("b", 5, 15),
            qualified("c", 20, 25),
        ];
        let report = build_partial_order(&events).unwrap();
        assert_eq!(report.len(), 3);
        assert_eq!(report[0].relation, TemporalRelation::UnresolvedOverlap);
        assert_eq!(report[1].relation, TemporalRelation::ProvenBefore);
        assert_eq!(report[2].relation, TemporalRelation::ProvenBefore);
    }

    #[test]
    fn widening_an_enclosure_cannot_upgrade_an_unresolved_pair() {
        let narrow = compare_time_evidence(&qualified("a", 10, 14), &qualified("b", 16, 20));
        let widened = compare_time_evidence(&qualified("a", 10, 16), &qualified("b", 16, 20));
        assert_eq!(narrow, TemporalRelation::ProvenBefore);
        assert_eq!(widened, TemporalRelation::UnresolvedOverlap);
    }

    #[test]
    fn empty_event_set_has_an_empty_partial_order() {
        assert_eq!(build_partial_order(&[]), Ok(Vec::new()));
    }
}

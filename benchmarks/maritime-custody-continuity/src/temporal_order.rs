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
/// QualifiedEnclosure can only be produced through a ClockBoundProfile in
/// this crate. The profile carries declared evidence provenance, but this
/// module does not independently authenticate or audit that evidence.
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
    clock_qualification: Option<ClockQualificationRef>,
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
        if bound_kind == TimeBoundKind::QualifiedEnclosure {
            return Err(TemporalOrderError::QualifiedBoundsRequireClockProfile);
        }

        Ok(Self {
            event_id,
            time_basis_id,
            interval,
            bound_kind,
            clock_qualification: None,
        })
    }

    pub(crate) fn from_clock_qualification(
        event_id: impl Into<String>,
        time_basis_id: impl Into<String>,
        interval: TimeInterval,
        qualification: ClockQualificationRef,
    ) -> Result<Self, TemporalOrderError> {
        let mut evidence = Self::new(
            event_id,
            time_basis_id,
            interval,
            TimeBoundKind::EstimateOnly,
        )?;
        evidence.bound_kind = TimeBoundKind::QualifiedEnclosure;
        evidence.clock_qualification = Some(qualification);
        Ok(evidence)
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

    pub fn clock_qualification(&self) -> Option<&ClockQualificationRef> {
        self.clock_qualification.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockSyncState {
    Locked,
    Holdover,
    Alarmed,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockBoundProfileSpec {
    pub clock_id: String,
    pub profile_id: String,
    pub time_basis_id: String,
    pub calibration_evidence_id: String,
    pub synchronization_method: String,
    pub sync_state: ClockSyncState,
    pub calibrated_at: i64,
    pub valid_until: i64,
    pub max_offset_error_ticks: Option<u64>,
    pub max_holdover_drift_ticks: Option<u64>,
    pub max_capture_path_ticks: Option<u64>,
    pub max_rounding_error_ticks: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockQualificationRef {
    pub clock_id: String,
    pub profile_id: String,
    pub calibration_evidence_id: String,
    pub synchronization_method: String,
    pub sync_state: ClockSyncState,
    pub calibrated_at: i64,
    pub valid_until: i64,
    pub maximum_total_error_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockBoundError {
    EmptyClockId,
    EmptyProfileId,
    EmptyTimeBasisId,
    EmptyCalibrationEvidenceId,
    EmptySynchronizationMethod,
    InvalidCalibrationWindow,
    ClockNotQualified,
    MissingErrorComponent(&'static str),
    ErrorBudgetOverflow,
    CalibrationOutsideValidityWindow,
    IntervalOverflow,
    InvalidTimeEvidence(TemporalOrderError),
}

/// A clock profile with a mandatory, conservative error budget. The
/// calibration evidence reference is opaque; this type does not authenticate
/// the issuer or validate the calibration artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockBoundProfile {
    spec: ClockBoundProfileSpec,
    maximum_total_error_ticks: u64,
}

impl ClockBoundProfile {
    pub fn new(spec: ClockBoundProfileSpec) -> Result<Self, ClockBoundError> {
        for (value, error) in [
            (&spec.clock_id, ClockBoundError::EmptyClockId),
            (&spec.profile_id, ClockBoundError::EmptyProfileId),
            (&spec.time_basis_id, ClockBoundError::EmptyTimeBasisId),
            (
                &spec.calibration_evidence_id,
                ClockBoundError::EmptyCalibrationEvidenceId,
            ),
            (
                &spec.synchronization_method,
                ClockBoundError::EmptySynchronizationMethod,
            ),
        ] {
            if value.trim().is_empty() || value.trim() != value {
                return Err(error);
            }
        }
        if spec.calibrated_at > spec.valid_until {
            return Err(ClockBoundError::InvalidCalibrationWindow);
        }
        if !matches!(
            spec.sync_state,
            ClockSyncState::Locked | ClockSyncState::Holdover
        ) {
            return Err(ClockBoundError::ClockNotQualified);
        }

        let offset = spec
            .max_offset_error_ticks
            .ok_or(ClockBoundError::MissingErrorComponent("offset"))?;
        let holdover = spec
            .max_holdover_drift_ticks
            .ok_or(ClockBoundError::MissingErrorComponent("holdover drift"))?;
        let capture = spec
            .max_capture_path_ticks
            .ok_or(ClockBoundError::MissingErrorComponent("capture path"))?;
        let rounding = spec
            .max_rounding_error_ticks
            .ok_or(ClockBoundError::MissingErrorComponent("rounding"))?;

        let maximum_total_error_ticks = offset
            .checked_add(holdover)
            .and_then(|value| value.checked_add(capture))
            .and_then(|value| value.checked_add(rounding))
            .ok_or(ClockBoundError::ErrorBudgetOverflow)?;

        Ok(Self {
            spec,
            maximum_total_error_ticks,
        })
    }

    /// Derive a closed event-time interval from a source timestamp and the
    /// declared worst-case error budget. This remains conditional on the
    /// caller's calibration claims and is not an independent clock audit.
    pub fn qualify_event(
        &self,
        event_id: impl Into<String>,
        source_timestamp_ticks: i64,
    ) -> Result<TimeEvidence, ClockBoundError> {
        if source_timestamp_ticks < self.spec.calibrated_at
            || source_timestamp_ticks > self.spec.valid_until
        {
            return Err(ClockBoundError::CalibrationOutsideValidityWindow);
        }

        let radius = i64::try_from(self.maximum_total_error_ticks)
            .map_err(|_| ClockBoundError::IntervalOverflow)?;
        let earliest = source_timestamp_ticks
            .checked_sub(radius)
            .ok_or(ClockBoundError::IntervalOverflow)?;
        let latest = source_timestamp_ticks
            .checked_add(radius)
            .ok_or(ClockBoundError::IntervalOverflow)?;
        let interval =
            TimeInterval::new(earliest, latest).map_err(|_| ClockBoundError::IntervalOverflow)?;
        if interval.earliest() < self.spec.calibrated_at
            || interval.latest() > self.spec.valid_until
        {
            return Err(ClockBoundError::CalibrationOutsideValidityWindow);
        }

        let qualification = ClockQualificationRef {
            clock_id: self.spec.clock_id.clone(),
            profile_id: self.spec.profile_id.clone(),
            calibration_evidence_id: self.spec.calibration_evidence_id.clone(),
            synchronization_method: self.spec.synchronization_method.clone(),
            sync_state: self.spec.sync_state,
            calibrated_at: self.spec.calibrated_at,
            valid_until: self.spec.valid_until,
            maximum_total_error_ticks: self.maximum_total_error_ticks,
        };

        TimeEvidence::from_clock_qualification(
            event_id,
            self.spec.time_basis_id.clone(),
            interval,
            qualification,
        )
        .map_err(ClockBoundError::InvalidTimeEvidence)
    }

    pub fn maximum_total_error_ticks(&self) -> u64 {
        self.maximum_total_error_ticks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalOrderError {
    ReversedInterval,
    EmptyEventId,
    UntrimmedEventId,
    EmptyTimeBasisId,
    UntrimmedTimeBasisId,
    QualifiedBoundsRequireClockProfile,
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
        let center = earliest + (latest - earliest) / 2;
        let radius = (center - earliest).max(latest - center) as u64;
        let profile = ClockBoundProfile::new(ClockBoundProfileSpec {
            clock_id: "SYNTHETIC_CLOCK".to_string(),
            profile_id: "SYNTHETIC_PROFILE_V1".to_string(),
            time_basis_id: "UTC_NS_PROFILE_V1".to_string(),
            calibration_evidence_id: "SYNTHETIC_CALIBRATION".to_string(),
            synchronization_method: "SYNTHETIC_REFERENCE".to_string(),
            sync_state: ClockSyncState::Locked,
            calibrated_at: i64::MIN,
            valid_until: i64::MAX,
            max_offset_error_ticks: Some(0),
            max_holdover_drift_ticks: Some(0),
            max_capture_path_ticks: Some(0),
            max_rounding_error_ticks: Some(radius),
        })
        .unwrap();
        profile.qualify_event(id, center).unwrap()
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

    fn clock_profile(
        sync_state: ClockSyncState,
        offset: Option<u64>,
        holdover: Option<u64>,
        capture: Option<u64>,
        rounding: Option<u64>,
    ) -> Result<ClockBoundProfile, ClockBoundError> {
        ClockBoundProfile::new(ClockBoundProfileSpec {
            clock_id: "PTP_CLOCK_01".to_string(),
            profile_id: "PTP_PROFILE_V1".to_string(),
            time_basis_id: "UTC_NS_PROFILE_V1".to_string(),
            calibration_evidence_id: "CALIBRATION_RECEIPT_01".to_string(),
            synchronization_method: "IEEE_1588_PTP".to_string(),
            sync_state,
            calibrated_at: 0,
            valid_until: 1_000,
            max_offset_error_ticks: offset,
            max_holdover_drift_ticks: holdover,
            max_capture_path_ticks: capture,
            max_rounding_error_ticks: rounding,
        })
    }

    #[test]
    fn proof_grade_time_evidence_retains_clock_provenance() {
        let profile =
            clock_profile(ClockSyncState::Locked, Some(2), Some(1), Some(2), Some(1)).unwrap();
        let evidence = profile.qualify_event("handoff", 100).unwrap();

        assert_eq!(profile.maximum_total_error_ticks(), 6);
        assert_eq!(evidence.interval(), TimeInterval::new(94, 106).unwrap());
        assert_eq!(evidence.bound_kind(), TimeBoundKind::QualifiedEnclosure);
        let provenance = evidence.clock_qualification().unwrap();
        assert_eq!(provenance.clock_id, "PTP_CLOCK_01");
        assert_eq!(provenance.profile_id, "PTP_PROFILE_V1");
        assert_eq!(provenance.calibration_evidence_id, "CALIBRATION_RECEIPT_01");
        assert_eq!(provenance.maximum_total_error_ticks, 6);
    }

    #[test]
    fn unknown_error_component_blocks_clock_qualification() {
        assert_eq!(
            clock_profile(ClockSyncState::Locked, Some(1), None, Some(0), Some(0)),
            Err(ClockBoundError::MissingErrorComponent("holdover drift"))
        );
    }

    #[test]
    fn alarmed_or_unknown_clock_state_cannot_produce_proof_grade_bounds() {
        assert_eq!(
            clock_profile(ClockSyncState::Alarmed, Some(1), Some(0), Some(0), Some(0)),
            Err(ClockBoundError::ClockNotQualified)
        );
        assert_eq!(
            clock_profile(ClockSyncState::Unknown, Some(1), Some(0), Some(0), Some(0)),
            Err(ClockBoundError::ClockNotQualified)
        );
    }

    #[test]
    fn event_outside_clock_profile_validity_window_is_rejected() {
        let profile =
            clock_profile(ClockSyncState::Holdover, Some(1), Some(1), Some(0), Some(0)).unwrap();
        assert_eq!(
            profile.qualify_event("late_event", 1_001),
            Err(ClockBoundError::CalibrationOutsideValidityWindow)
        );
    }

    #[test]
    fn derived_interval_must_fit_inside_clock_calibration_window() {
        let profile =
            clock_profile(ClockSyncState::Locked, Some(5), Some(0), Some(0), Some(0)).unwrap();
        assert_eq!(
            profile.qualify_event("near_window_start", 2),
            Err(ClockBoundError::CalibrationOutsideValidityWindow)
        );
    }

    #[test]
    fn clock_error_budget_overflow_is_rejected() {
        assert_eq!(
            clock_profile(
                ClockSyncState::Locked,
                Some(u64::MAX),
                Some(1),
                Some(0),
                Some(0),
            ),
            Err(ClockBoundError::ErrorBudgetOverflow)
        );
    }

    #[test]
    fn derived_interval_overflow_is_rejected() {
        let profile = ClockBoundProfile::new(ClockBoundProfileSpec {
            clock_id: "CLOCK".to_string(),
            profile_id: "PROFILE".to_string(),
            time_basis_id: "UTC_NS".to_string(),
            calibration_evidence_id: "CAL".to_string(),
            synchronization_method: "TEST".to_string(),
            sync_state: ClockSyncState::Locked,
            calibrated_at: i64::MIN,
            valid_until: i64::MAX,
            max_offset_error_ticks: Some(10),
            max_holdover_drift_ticks: Some(0),
            max_capture_path_ticks: Some(0),
            max_rounding_error_ticks: Some(0),
        })
        .unwrap();
        assert_eq!(
            profile.qualify_event("near_min", i64::MIN),
            Err(ClockBoundError::IntervalOverflow)
        );
    }

    #[test]
    fn qualified_enclosures_cannot_be_constructed_by_setting_enum_alone() {
        assert_eq!(
            TimeEvidence::new(
                "event",
                "UTC_NS_PROFILE_V1",
                TimeInterval::new(1, 2).unwrap(),
                TimeBoundKind::QualifiedEnclosure,
            ),
            Err(TemporalOrderError::QualifiedBoundsRequireClockProfile)
        );
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
            TimeBoundKind::EstimateOnly,
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
            TimeEvidence::new("", "UTC", interval, TimeBoundKind::EstimateOnly),
            Err(TemporalOrderError::EmptyEventId)
        );
        assert_eq!(
            TimeEvidence::new(" event", "UTC", interval, TimeBoundKind::EstimateOnly),
            Err(TemporalOrderError::UntrimmedEventId)
        );
        assert_eq!(
            TimeEvidence::new("event", "", interval, TimeBoundKind::EstimateOnly),
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

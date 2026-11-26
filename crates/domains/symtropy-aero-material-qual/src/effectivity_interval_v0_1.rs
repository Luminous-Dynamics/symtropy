// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only interval-to-interval temporal effectivity comparison.
//!
//! A source event's effective time may itself be uncertain, just as physical
//! acquisition time may be uncertain. This module compares declared bounds
//! without assigning probabilities or selecting favourable endpoints.
//!
//! The intervals are hard bounds for this contract. Statistical confidence
//! intervals must carry their coverage method/level separately and must not be
//! silently treated as deterministic bounds.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeInterval {
    pub earliest_unix_ms: u64,
    pub latest_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum EffectivityRelation {
    DefinitelyEffective,
    DefinitelyNotYetEffective,
    BoundaryAmbiguous,
    InvalidInterval,
}

impl EffectivityRelation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefinitelyEffective => "DEFINITELY_EFFECTIVE",
            Self::DefinitelyNotYetEffective => "DEFINITELY_NOT_YET_EFFECTIVE",
            Self::BoundaryAmbiguous => "TIME_BOUNDARY_AMBIGUOUS",
            Self::InvalidInterval => "INVALID_TIME_INTERVAL",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectivityComparison {
    pub relation: EffectivityRelation,
    pub reason_code: &'static str,
}

/// Compare the uncertain effective-time interval of an event with the
/// uncertain physical acquisition-time interval of a measurement.
///
/// For inclusive event effectivity:
/// - event.latest <= acquisition.earliest: event is effective for every
///   acquisition instant admitted by the bounds.
/// - event.earliest > acquisition.latest: event is not yet effective for any
///   acquisition instant admitted by the bounds.
/// - otherwise: the intervals overlap or touch without a universal ordering;
///   chronology is ambiguous and the engineering evaluator must not guess.
///
/// The function does not decide whether the event was received by the evidence
/// watermark or trusted under the policy snapshot. Those are separate axes.
pub fn compare_effectivity_intervals(
    event_effective: TimeInterval,
    acquisition: TimeInterval,
) -> EffectivityComparison {
    if event_effective.earliest_unix_ms > event_effective.latest_unix_ms
        || acquisition.earliest_unix_ms > acquisition.latest_unix_ms
    {
        return EffectivityComparison {
            relation: EffectivityRelation::InvalidInterval,
            reason_code: "TEMPORAL_INTERVAL_START_AFTER_END",
        };
    }

    if event_effective.latest_unix_ms <= acquisition.earliest_unix_ms {
        return EffectivityComparison {
            relation: EffectivityRelation::DefinitelyEffective,
            reason_code: "EVENT_EFFECTIVE_BEFORE_EVERY_POSSIBLE_ACQUISITION",
        };
    }

    if event_effective.earliest_unix_ms > acquisition.latest_unix_ms {
        return EffectivityComparison {
            relation: EffectivityRelation::DefinitelyNotYetEffective,
            reason_code: "EVENT_EFFECTIVE_AFTER_EVERY_POSSIBLE_ACQUISITION",
        };
    }

    EffectivityComparison {
        relation: EffectivityRelation::BoundaryAmbiguous,
        reason_code: "EVENT_AND_ACQUISITION_TIME_BOUNDS_OVERLAP",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EffectivityRelation, TimeInterval, compare_effectivity_intervals,
    };

    fn interval(earliest: u64, latest: u64) -> TimeInterval {
        TimeInterval {
            earliest_unix_ms: earliest,
            latest_unix_ms: latest,
        }
    }

    #[test]
    fn event_definitely_effective_before_every_acquisition_time() {
        let result =
            compare_effectivity_intervals(interval(80, 90), interval(100, 110));
        assert_eq!(result.relation, EffectivityRelation::DefinitelyEffective);
    }

    #[test]
    fn event_definitely_not_yet_effective_after_every_acquisition_time() {
        let result =
            compare_effectivity_intervals(interval(111, 120), interval(100, 110));
        assert_eq!(
            result.relation,
            EffectivityRelation::DefinitelyNotYetEffective
        );
    }

    #[test]
    fn overlapping_intervals_abstain_instead_of_choosing_an_endpoint() {
        let result =
            compare_effectivity_intervals(interval(105, 115), interval(100, 110));
        assert_eq!(result.relation, EffectivityRelation::BoundaryAmbiguous);
    }

    #[test]
    fn exact_point_effectivity_at_exact_acquisition_is_inclusive() {
        let result =
            compare_effectivity_intervals(interval(100, 100), interval(100, 100));
        assert_eq!(result.relation, EffectivityRelation::DefinitelyEffective);
    }

    #[test]
    fn touching_uncertainty_bounds_without_universal_order_are_ambiguous() {
        let result =
            compare_effectivity_intervals(interval(110, 120), interval(100, 110));
        assert_eq!(result.relation, EffectivityRelation::BoundaryAmbiguous);
    }

    #[test]
    fn invalid_event_or_acquisition_interval_is_rejected() {
        let event_invalid =
            compare_effectivity_intervals(interval(120, 110), interval(100, 110));
        assert_eq!(event_invalid.relation, EffectivityRelation::InvalidInterval);

        let acquisition_invalid =
            compare_effectivity_intervals(interval(80, 90), interval(120, 110));
        assert_eq!(
            acquisition_invalid.relation,
            EffectivityRelation::InvalidInterval
        );
    }

    #[test]
    fn permuting_interval_arguments_changes_only_the_semantic_roles() {
        let event = interval(80, 90);
        let acquisition = interval(100, 110);
        let forward = compare_effectivity_intervals(event, acquisition);
        let reverse = compare_effectivity_intervals(acquisition, event);
        assert_eq!(forward.relation, EffectivityRelation::DefinitelyEffective);
        assert_eq!(
            reverse.relation,
            EffectivityRelation::DefinitelyNotYetEffective
        );
    }
}

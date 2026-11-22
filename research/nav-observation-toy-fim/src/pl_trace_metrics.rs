//! Pointwise protection-level exceedance and time-exposure trace metrics.
//!
//! Input records carry an explicit support interval. No time exposure is
//! inferred from row rate or assumed sample-and-hold. Invalid reference-truth
//! intervals are excluded from denominators and split breach episodes.

use std::collections::BTreeSet;
use std::fmt;

const PPM_SCALE: u128 = 1_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct ProtectionLevelTraceRecord {
    pub record_id: String,
    pub cluster_id: String,
    /// Start of this record's declared support interval, Unix/route monotonic ns.
    pub timestamp_ns: u64,
    /// Explicit duration represented by this sample; must be positive.
    pub support_duration_ns: u64,
    /// If false, both error values must be None; the record is counted as
    /// reference-invalid and is not silently treated as a safe sample.
    pub truth_valid: bool,
    pub error_longitudinal_m: Option<f64>,
    pub error_cross_track_m: Option<f64>,
    pub protection_level_longitudinal_m: f64,
    pub protection_level_cross_track_m: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BreachEpisodeMetrics {
    pub event_count: usize,
    pub total_exposure_ns: u128,
    pub maximum_contiguous_duration_ns: u64,
    pub mean_event_duration_ns: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectionLevelTraceSummary {
    pub record_count: usize,
    pub valid_truth_samples: usize,
    pub invalid_truth_samples: usize,
    pub distinct_cluster_count: usize,

    pub longitudinal_exceedance_samples: usize,
    pub cross_track_exceedance_samples: usize,
    pub either_axis_exceedance_samples: usize,

    pub longitudinal_exceedance_ppm: Option<u32>,
    pub cross_track_exceedance_ppm: Option<u32>,
    pub either_axis_exceedance_ppm: Option<u32>,

    pub total_valid_exposure_ns: u128,
    pub longitudinal_exposure_above_pl_ns: u128,
    pub cross_track_exposure_above_pl_ns: u128,
    pub either_axis_exposure_above_pl_ns: u128,

    pub longitudinal_breaches: BreachEpisodeMetrics,
    pub cross_track_breaches: BreachEpisodeMetrics,
    pub either_axis_breaches: BreachEpisodeMetrics,

    pub maximum_longitudinal_exceedance_micrometers: u64,
    pub maximum_cross_track_exceedance_micrometers: u64,
    pub maximum_either_axis_exceedance_micrometers: u64,

    /// Descriptive breach-event counts divided by valid exposure hours. This
    /// is not a statistically calibrated integrity probability.
    /// Breach events per valid exposure hour, multiplied by 1,000,000.
    /// This descriptive rate is not a probability or integrity guarantee.
    pub either_axis_breach_events_per_valid_hour_micros: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProtectionLevelTraceError {
    EmptyTrace,
    EmptyRecordId,
    EmptyClusterId,
    DuplicateRecordId(String),
    ZeroSupportDuration(String),
    SupportIntervalOverflow(String),
    InvalidProtectionLevel(String),
    InvalidError(String),
    TruthValidityMismatch(String),
    OverlappingSupportIntervals {
        previous_record_id: String,
        current_record_id: String,
    },
}

impl fmt::Display for ProtectionLevelTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "trace must contain at least one record"),
            Self::EmptyRecordId => write!(f, "record ID must not be empty"),
            Self::EmptyClusterId => write!(f, "cluster ID must not be empty"),
            Self::DuplicateRecordId(id) => write!(f, "duplicate record ID: {id}"),
            Self::ZeroSupportDuration(id) => write!(f, "support duration must be positive: {id}"),
            Self::SupportIntervalOverflow(id) => write!(f, "support interval overflows u64 timestamp: {id}"),
            Self::InvalidProtectionLevel(id) => write!(f, "protection levels must be finite and nonnegative: {id}"),
            Self::InvalidError(id) => write!(f, "valid truth errors must be finite and present: {id}"),
            Self::TruthValidityMismatch(id) => write!(f, "truth_valid/error option mismatch: {id}"),
            Self::OverlappingSupportIntervals {
                previous_record_id,
                current_record_id,
            } => write!(
                f,
                "support intervals overlap: {previous_record_id} and {current_record_id}"
            ),
        }
    }
}
impl std::error::Error for ProtectionLevelTraceError {}

#[derive(Clone, Copy, Debug, Default)]
struct EpisodeAccumulator {
    event_count: usize,
    total_exposure_ns: u128,
    maximum_duration_ns: u64,
    active_duration_ns: u64,
    previous_end_ns: Option<u64>,
}

impl EpisodeAccumulator {
    fn add(&mut self, timestamp_ns: u64, duration_ns: u64, violation: bool) {
        if !violation {
            self.active_duration_ns = 0;
            self.previous_end_ns = None;
            return;
        }

        let contiguous = self.previous_end_ns == Some(timestamp_ns);
        if !contiguous {
            if self.active_duration_ns > 0 {
                self.maximum_duration_ns =
                    self.maximum_duration_ns.max(self.active_duration_ns);
            }
            self.event_count += 1;
            self.active_duration_ns = 0;
        }

        self.active_duration_ns = self.active_duration_ns.saturating_add(duration_ns);
        self.total_exposure_ns = self.total_exposure_ns.saturating_add(duration_ns as u128);
        self.previous_end_ns = timestamp_ns.checked_add(duration_ns);
    }

    fn finish(mut self) -> BreachEpisodeMetrics {
        self.maximum_duration_ns = self.maximum_duration_ns.max(self.active_duration_ns);
        BreachEpisodeMetrics {
            event_count: self.event_count,
            total_exposure_ns: self.total_exposure_ns,
            maximum_contiguous_duration_ns: self.maximum_duration_ns,
            mean_event_duration_ns: (self.event_count > 0).then(|| {
                (self.total_exposure_ns / self.event_count as u128)
                    .min(u64::MAX as u128) as u64
            }),
        }
    }
}

fn pointwise_ppm(exceedances: usize, valid: usize) -> Option<u32> {
    (valid > 0).then(|| {
        ((exceedances as u128 * PPM_SCALE / valid as u128).min(PPM_SCALE)) as u32
    })
}

fn meters_to_micrometers(exceedance_m: f64) -> u64 {
    if exceedance_m <= 0.0 {
        return 0;
    }
    (exceedance_m * 1_000_000.0)
        .ceil()
        .min(u64::MAX as f64) as u64
}

/// Evaluate an ordered trace while canonically sorting it by timestamp and ID.
///
/// Each sample's exceedance state is treated as constant over its explicit
/// support interval. Adjacent violating support intervals form one breach
/// event; any valid non-violating sample, invalid-truth interval or time gap
/// terminates the current event. This is a descriptive sampled-trace metric,
/// not interpolation of continuous physical error.
pub fn evaluate_protection_level_trace(
    records: &[ProtectionLevelTraceRecord],
) -> Result<ProtectionLevelTraceSummary, ProtectionLevelTraceError> {
    if records.is_empty() {
        return Err(ProtectionLevelTraceError::EmptyTrace);
    }

    let mut ids = BTreeSet::new();
    let mut clusters = BTreeSet::new();
    let mut ordered = records.to_vec();

    for record in &ordered {
        if record.record_id.trim().is_empty() {
            return Err(ProtectionLevelTraceError::EmptyRecordId);
        }
        if !ids.insert(record.record_id.clone()) {
            return Err(ProtectionLevelTraceError::DuplicateRecordId(
                record.record_id.clone(),
            ));
        }
        if record.cluster_id.trim().is_empty() {
            return Err(ProtectionLevelTraceError::EmptyClusterId);
        }
        clusters.insert(record.cluster_id.clone());

        if record.support_duration_ns == 0 {
            return Err(ProtectionLevelTraceError::ZeroSupportDuration(
                record.record_id.clone(),
            ));
        }
        if record.timestamp_ns.checked_add(record.support_duration_ns).is_none() {
            return Err(ProtectionLevelTraceError::SupportIntervalOverflow(
                record.record_id.clone(),
            ));
        }
        if !record.protection_level_longitudinal_m.is_finite()
            || record.protection_level_longitudinal_m < 0.0
            || !record.protection_level_cross_track_m.is_finite()
            || record.protection_level_cross_track_m < 0.0
        {
            return Err(ProtectionLevelTraceError::InvalidProtectionLevel(
                record.record_id.clone(),
            ));
        }

        match (
            record.truth_valid,
            record.error_longitudinal_m,
            record.error_cross_track_m,
        ) {
            (true, Some(long), Some(cross)) if long.is_finite() && cross.is_finite() => {}
            (false, None, None) => {}
            (true, _, _) => {
                return Err(ProtectionLevelTraceError::InvalidError(
                    record.record_id.clone(),
                ));
            }
            (false, _, _) => {
                return Err(ProtectionLevelTraceError::TruthValidityMismatch(
                    record.record_id.clone(),
                ));
            }
        }
    }

    ordered.sort_by(|a, b| {
        a.timestamp_ns
            .cmp(&b.timestamp_ns)
            .then_with(|| a.record_id.cmp(&b.record_id))
    });

    let mut previous: Option<&ProtectionLevelTraceRecord> = None;
    for current in &ordered {
        if let Some(prior) = previous {
            let prior_end = prior.timestamp_ns + prior.support_duration_ns;
            if current.timestamp_ns < prior_end {
                return Err(ProtectionLevelTraceError::OverlappingSupportIntervals {
                    previous_record_id: prior.record_id.clone(),
                    current_record_id: current.record_id.clone(),
                });
            }
        }
        previous = Some(current);
    }

    let mut valid_truth_samples = 0_usize;
    let mut invalid_truth_samples = 0_usize;

    let mut longitudinal_exceedance_samples = 0_usize;
    let mut cross_track_exceedance_samples = 0_usize;
    let mut either_axis_exceedance_samples = 0_usize;

    let mut total_valid_exposure_ns = 0_u128;
    let mut longitudinal_exposure_above_pl_ns = 0_u128;
    let mut cross_track_exposure_above_pl_ns = 0_u128;
    let mut either_axis_exposure_above_pl_ns = 0_u128;

    let mut longitudinal_max_micrometers = 0_u64;
    let mut cross_max_micrometers = 0_u64;
    let mut either_max_micrometers = 0_u64;

    let mut longitudinal_episodes = EpisodeAccumulator::default();
    let mut cross_episodes = EpisodeAccumulator::default();
    let mut either_episodes = EpisodeAccumulator::default();

    for record in &ordered {
        if !record.truth_valid {
            invalid_truth_samples += 1;
            longitudinal_episodes.add(record.timestamp_ns, record.support_duration_ns, false);
            cross_episodes.add(record.timestamp_ns, record.support_duration_ns, false);
            either_episodes.add(record.timestamp_ns, record.support_duration_ns, false);
            continue;
        }

        valid_truth_samples += 1;
        total_valid_exposure_ns =
            total_valid_exposure_ns.saturating_add(record.support_duration_ns as u128);

        let longitudinal_error = record.error_longitudinal_m.unwrap().abs();
        let cross_error = record.error_cross_track_m.unwrap().abs();
        let longitudinal_exceedance =
            (longitudinal_error - record.protection_level_longitudinal_m).max(0.0);
        let cross_exceedance =
            (cross_error - record.protection_level_cross_track_m).max(0.0);

        let long_violation = longitudinal_exceedance > 0.0;
        let cross_violation = cross_exceedance > 0.0;
        let either_violation = long_violation || cross_violation;

        if long_violation {
            longitudinal_exceedance_samples += 1;
            longitudinal_exposure_above_pl_ns =
                longitudinal_exposure_above_pl_ns.saturating_add(record.support_duration_ns as u128);
        }
        if cross_violation {
            cross_track_exceedance_samples += 1;
            cross_track_exposure_above_pl_ns =
                cross_track_exposure_above_pl_ns.saturating_add(record.support_duration_ns as u128);
        }
        if either_violation {
            either_axis_exceedance_samples += 1;
            either_axis_exposure_above_pl_ns =
                either_axis_exposure_above_pl_ns.saturating_add(record.support_duration_ns as u128);
        }

        longitudinal_max_micrometers = longitudinal_max_micrometers
            .max(meters_to_micrometers(longitudinal_exceedance));
        cross_max_micrometers = cross_max_micrometers
            .max(meters_to_micrometers(cross_exceedance));
        either_max_micrometers = either_max_micrometers.max(
            meters_to_micrometers(longitudinal_exceedance.max(cross_exceedance)),
        );

        longitudinal_episodes.add(
            record.timestamp_ns,
            record.support_duration_ns,
            long_violation,
        );
        cross_episodes.add(
            record.timestamp_ns,
            record.support_duration_ns,
            cross_violation,
        );
        either_episodes.add(
            record.timestamp_ns,
            record.support_duration_ns,
            either_violation,
        );
    }

    let longitudinal_breaches = longitudinal_episodes.finish();
    let cross_track_breaches = cross_episodes.finish();
    let either_axis_breaches = either_episodes.finish();

    let valid_hours_ns = total_valid_exposure_ns as f64 / 3_600_000_000_000.0;
    let event_rate_micros = (valid_hours_ns > 0.0).then(|| {
        ((either_axis_breaches.event_count as f64 / valid_hours_ns) * 1_000_000.0)
            .ceil()
            .min(u64::MAX as f64) as u64
    });

    Ok(ProtectionLevelTraceSummary {
        record_count: ordered.len(),
        valid_truth_samples,
        invalid_truth_samples,
        distinct_cluster_count: clusters.len(),
        longitudinal_exceedance_samples,
        cross_track_exceedance_samples,
        either_axis_exceedance_samples,
        longitudinal_exceedance_ppm: pointwise_ppm(
            longitudinal_exceedance_samples,
            valid_truth_samples,
        ),
        cross_track_exceedance_ppm: pointwise_ppm(
            cross_track_exceedance_samples,
            valid_truth_samples,
        ),
        either_axis_exceedance_ppm: pointwise_ppm(
            either_axis_exceedance_samples,
            valid_truth_samples,
        ),
        total_valid_exposure_ns,
        longitudinal_exposure_above_pl_ns,
        cross_track_exposure_above_pl_ns,
        either_axis_exposure_above_pl_ns,
        longitudinal_breaches,
        cross_track_breaches,
        either_axis_breaches,
        maximum_longitudinal_exceedance_m: longitudinal_max_micrometers,
        maximum_cross_track_exceedance_m: cross_max_micrometers,
        maximum_either_axis_exceedance_m: either_max_micrometers,
        either_axis_breach_events_per_valid_hour_micros: event_rate_micros,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        id: &str,
        cluster: &str,
        timestamp_ms: u64,
        duration_ms: u64,
        truth_valid: bool,
        longitudinal_error: Option<f64>,
        cross_error: Option<f64>,
    ) -> ProtectionLevelTraceRecord {
        ProtectionLevelTraceRecord {
            record_id: id.to_owned(),
            cluster_id: cluster.to_owned(),
            timestamp_ns: timestamp_ms * 1_000_000,
            support_duration_ns: duration_ms * 1_000_000,
            truth_valid,
            error_longitudinal_m: longitudinal_error,
            error_cross_track_m: cross_error,
            protection_level_longitudinal_m: 1.0,
            protection_level_cross_track_m: 2.0,
        }
    }

    #[test]
    fn pointwise_stanford_numerator_and_denominator_are_exact() {
        let records = vec![
            record("a", "route-a", 0, 100, true, Some(1.2), Some(0.0)),
            record("b", "route-a", 100, 100, true, Some(0.5), Some(2.5)),
            record("c", "route-b", 200, 100, true, Some(0.0), Some(0.0)),
            record("d", "route-b", 300, 100, false, None, None),
        ];
        let metrics = evaluate_protection_level_trace(&records).unwrap();
        assert_eq!(metrics.valid_truth_samples, 3);
        assert_eq!(metrics.invalid_truth_samples, 1);
        assert_eq!(metrics.longitudinal_exceedance_samples, 1);
        assert_eq!(metrics.cross_track_exceedance_samples, 1);
        assert_eq!(metrics.either_axis_exceedance_samples, 2);
        assert_eq!(metrics.longitudinal_exceedance_ppm, Some(333_333));
        assert_eq!(metrics.cross_track_exceedance_ppm, Some(333_333));
        assert_eq!(metrics.either_axis_exceedance_ppm, Some(666_666));
    }

    #[test]
    fn prolonged_high_rate_breach_is_one_event_not_many_events() {
        let records = vec![
            record("a", "event-1", 0, 10, true, Some(2.0), Some(0.0)),
            record("b", "event-1", 10, 10, true, Some(2.1), Some(0.0)),
            record("c", "event-1", 20, 10, true, Some(2.2), Some(0.0)),
            record("d", "event-1", 30, 10, true, Some(0.0), Some(0.0)),
        ];
        let metrics = evaluate_protection_level_trace(&records).unwrap();
        assert_eq!(metrics.longitudinal_exceedance_samples, 3);
        assert_eq!(metrics.longitudinal_breaches.event_count, 1);
        assert_eq!(metrics.longitudinal_breaches.total_exposure_ns, 30_000_000);
        assert_eq!(metrics.longitudinal_breaches.maximum_contiguous_duration_ns, 30_000_000);
        assert_eq!(metrics.either_axis_breaches.event_count, 1);
    }

    #[test]
    fn explicit_time_gap_splits_breach_episodes() {
        let records = vec![
            record("a", "event-1", 0, 10, true, Some(2.0), Some(0.0)),
            record("b", "event-1", 20, 10, true, Some(2.0), Some(0.0)),
        ];
        let metrics = evaluate_protection_level_trace(&records).unwrap();
        assert_eq!(metrics.longitudinal_breaches.event_count, 2);
        assert_eq!(metrics.longitudinal_breaches.maximum_contiguous_duration_ns, 10_000_000);
    }

    #[test]
    fn invalid_truth_interval_splits_breach_and_is_not_counted_safe() {
        let records = vec![
            record("a", "event-1", 0, 10, true, Some(2.0), Some(0.0)),
            record("b", "event-1", 10, 10, false, None, None),
            record("c", "event-1", 20, 10, true, Some(2.0), Some(0.0)),
        ];
        let metrics = evaluate_protection_level_trace(&records).unwrap();
        assert_eq!(metrics.valid_truth_samples, 2);
        assert_eq!(metrics.invalid_truth_samples, 1);
        assert_eq!(metrics.longitudinal_exceedance_ppm, Some(1_000_000));
        assert_eq!(metrics.longitudinal_breaches.event_count, 2);
        assert_eq!(metrics.total_valid_exposure_ns, 20_000_000);
    }

    #[test]
    fn sample_order_is_canonical_and_permutation_invariant() {
        let mut forward = vec![
            record("a", "route-a", 0, 10, true, Some(2.0), Some(0.0)),
            record("b", "route-a", 10, 10, true, Some(2.0), Some(0.0)),
            record("c", "route-b", 20, 10, true, Some(0.0), Some(0.0)),
        ];
        let mut reverse = forward.clone();
        reverse.reverse();
        assert_eq!(
            evaluate_protection_level_trace(&forward).unwrap(),
            evaluate_protection_level_trace(&reverse).unwrap()
        );
        forward.reverse();
    }

    #[test]
    fn overlapping_support_intervals_fail_closed() {
        let records = vec![
            record("a", "event-a", 0, 20, true, Some(0.0), Some(0.0)),
            record("b", "event-a", 10, 20, true, Some(0.0), Some(0.0)),
        ];
        assert!(matches!(
            evaluate_protection_level_trace(&records),
            Err(ProtectionLevelTraceError::OverlappingSupportIntervals { .. })
        ));
    }

    #[test]
    fn missing_error_on_valid_truth_and_error_on_invalid_truth_fail_closed() {
        let valid_missing = vec![record("a", "event-a", 0, 10, true, None, Some(0.0))];
        assert!(matches!(
            evaluate_protection_level_trace(&valid_missing),
            Err(ProtectionLevelTraceError::InvalidError(_))
        ));
        let invalid_present = vec![record("a", "event-a", 0, 10, false, Some(0.0), None)];
        assert!(matches!(
            evaluate_protection_level_trace(&invalid_present),
            Err(ProtectionLevelTraceError::TruthValidityMismatch(_))
        ));
    }

    #[test]
    fn duplicate_record_ids_zero_duration_and_overflow_fail_closed() {
        let duplicate = vec![
            record("a", "event-a", 0, 10, true, Some(0.0), Some(0.0)),
            record("a", "event-b", 10, 10, true, Some(0.0), Some(0.0)),
        ];
        assert!(matches!(
            evaluate_protection_level_trace(&duplicate),
            Err(ProtectionLevelTraceError::DuplicateRecordId(_))
        ));
        let mut zero = record("z", "event", 0, 10, true, Some(0.0), Some(0.0));
        zero.support_duration_ns = 0;
        assert!(matches!(
            evaluate_protection_level_trace(&[zero]),
            Err(ProtectionLevelTraceError::ZeroSupportDuration(_))
        ));
        let mut overflow = record("x", "event", 0, 10, true, Some(0.0), Some(0.0));
        overflow.timestamp_ns = u64::MAX - 1;
        assert!(matches!(
            evaluate_protection_level_trace(&[overflow]),
            Err(ProtectionLevelTraceError::SupportIntervalOverflow(_))
        ));
    }

    #[test]
    fn cluster_count_and_exposure_weighted_metrics_are_separate() {
        let records = vec![
            record("a", "event-a", 0, 100, true, Some(2.0), Some(0.0)),
            record("b", "event-a", 100, 100, true, Some(0.0), Some(0.0)),
            record("c", "event-b", 200, 300, true, Some(2.0), Some(0.0)),
        ];
        let metrics = evaluate_protection_level_trace(&records).unwrap();
        assert_eq!(metrics.distinct_cluster_count, 2);
        assert_eq!(metrics.longitudinal_exceedance_samples, 2);
        assert_eq!(metrics.longitudinal_exposure_above_pl_ns, 400_000_000);
        assert_eq!(metrics.total_valid_exposure_ns, 500_000_000);
        assert_eq!(metrics.longitudinal_breaches.event_count, 2);
    }

    #[test]
    fn invalid_pl_and_empty_trace_fail_closed() {
        assert_eq!(
            evaluate_protection_level_trace(&[]),
            Err(ProtectionLevelTraceError::EmptyTrace)
        );
        let mut bad = record("a", "event-a", 0, 10, true, Some(0.0), Some(0.0));
        bad.protection_level_longitudinal_m = -1.0;
        assert!(matches!(
            evaluate_protection_level_trace(&[bad]),
            Err(ProtectionLevelTraceError::InvalidProtectionLevel(_))
        ));
    }
}

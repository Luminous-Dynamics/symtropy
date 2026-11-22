//! Trajectory availability, evidence support and service-disposition metrics.
//!
//! This evaluator separates challenge-compatible pose availability from poses
//! supported by a qualified measurement or bounded propagation. Interpolated
//! or extrapolated poses may improve raw trajectory coverage, but they are
//! reported separately and must not be treated as equivalent evidence.
//!
//! Input records are already timestamp/frame matched to an independently
//! supplied reference. This module does not perform ROS bag decoding, camera/IMU
//! synchronization, coordinate transforms, trajectory association or oracle
//! validation.

use std::collections::BTreeSet;
use std::fmt;

const PPM_SCALE: u128 = 1_000_000;
const HILTI_COVERAGE_GATE_PPM: u32 = 990_000;
const HILTI_EXPONENTIAL_SCORE_C: f64 = 0.460_517_018_598_809_17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrajectorySupport {
    MeasuredQualified,
    DeadReckonedBounded,
    MeasuredUnqualified,
    DeadReckonedUnbounded,
    Interpolated,
    Extrapolated,
    Missing,
}

impl TrajectorySupport {
    fn has_estimate(self) -> bool {
        self != Self::Missing
    }

    fn evidence_backed(self) -> bool {
        matches!(self, Self::MeasuredQualified | Self::DeadReckonedBounded)
    }

    fn unqualified_output(self) -> bool {
        self.has_estimate() && !self.evidence_backed()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrajectoryServiceState {
    Normal,
    Degraded,
    IntegrityUncertain,
    Unavailable,
    Recovering,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrajectoryServiceRecord {
    pub record_id: String,
    pub cluster_id: String,
    /// Start of this record's declared support interval, in a per-cluster monotonic clock.
    pub timestamp_ns: u64,
    /// Explicit interval represented by the record; never inferred from row frequency.
    pub support_duration_ns: u64,
    /// False means reference truth is unavailable for this interval, not that the output is safe.
    pub reference_valid: bool,
    pub support: TrajectorySupport,
    pub service_state: TrajectoryServiceState,
    /// Euclidean position error in metres, only when reference_valid and support != Missing.
    pub position_error_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorQuantiles {
    pub sample_count: usize,
    pub p50_micrometers: Option<u64>,
    pub p95_micrometers: Option<u64>,
    pub p99_micrometers: Option<u64>,
    pub maximum_micrometers: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrajectoryServiceSummary {
    pub record_count: usize,
    pub distinct_cluster_count: usize,
    pub valid_reference_samples: usize,
    pub invalid_reference_samples: usize,
    pub valid_reference_exposure_ns: u128,
    pub invalid_reference_exposure_ns: u128,
    pub estimate_available_samples: usize,
    pub missing_estimate_samples: usize,
    pub evidence_backed_samples: usize,
    pub unqualified_output_samples: usize,
    pub measured_qualified_samples: usize,
    pub dead_reckoned_bounded_samples: usize,
    pub measured_unqualified_samples: usize,
    pub dead_reckoned_unbounded_samples: usize,
    pub interpolated_samples: usize,
    pub extrapolated_samples: usize,
    pub official_pose_coverage_ppm: Option<u32>,
    pub evidence_backed_pose_coverage_ppm: Option<u32>,
    pub unqualified_output_coverage_ppm: Option<u32>,
    pub official_coverage_gate_met: Option<bool>,
    /// Exponential position score over matched samples, gated to zero below 99% coverage.
    pub challenge_compatible_exponential_score: Option<f64>,
    pub measured_qualified_exposure_ns: u128,
    pub dead_reckoned_bounded_exposure_ns: u128,
    pub measured_unqualified_exposure_ns: u128,
    pub dead_reckoned_unbounded_exposure_ns: u128,
    pub interpolated_exposure_ns: u128,
    pub extrapolated_exposure_ns: u128,
    pub missing_estimate_exposure_ns: u128,
    pub total_estimated_exposure_ns: u128,
    pub evidence_backed_exposure_ns: u128,
    pub unqualified_output_exposure_ns: u128,
    pub normal_without_qualified_support_samples: usize,
    pub normal_without_qualified_support_exposure_ns: u128,
    pub maximum_contiguous_missing_estimate_ns: u128,
    pub maximum_contiguous_unqualified_output_ns: u128,
    pub normal_exposure_ns: u128,
    pub degraded_exposure_ns: u128,
    pub integrity_uncertain_exposure_ns: u128,
    pub unavailable_exposure_ns: u128,
    pub recovering_exposure_ns: u128,
    pub indeterminate_exposure_ns: u128,
    pub all_estimate_errors: ErrorQuantiles,
    pub evidence_backed_errors: ErrorQuantiles,
    pub unqualified_output_errors: ErrorQuantiles,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrajectoryServiceError {
    EmptyTrace,
    EmptyRecordId,
    EmptyClusterId,
    DuplicateRecordId(String),
    ZeroSupportDuration(String),
    SupportIntervalOverflow(String),
    InvalidPositionError(String),
    ReferenceValidityMismatch(String),
    OverlappingSupportIntervals { previous_record_id: String, current_record_id: String },
}

impl fmt::Display for TrajectoryServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "trajectory trace must contain at least one record"),
            Self::EmptyRecordId => write!(f, "record ID must not be empty"),
            Self::EmptyClusterId => write!(f, "cluster ID must not be empty"),
            Self::DuplicateRecordId(id) => write!(f, "duplicate trajectory record ID: {id}"),
            Self::ZeroSupportDuration(id) => write!(f, "support duration must be positive: {id}"),
            Self::SupportIntervalOverflow(id) => write!(f, "support interval overflows timestamp: {id}"),
            Self::InvalidPositionError(id) => write!(f, "position error must be finite and nonnegative: {id}"),
            Self::ReferenceValidityMismatch(id) => write!(f, "position-error/reference-validity/support mismatch: {id}"),
            Self::OverlappingSupportIntervals { previous_record_id, current_record_id } => write!(f, "support intervals overlap within cluster: {previous_record_id} and {current_record_id}"),
        }
    }
}
impl std::error::Error for TrajectoryServiceError {}

#[derive(Clone, Copy, Debug, Default)]
struct DurationRun {
    active_ns: u128,
    maximum_ns: u128,
    previous_end_ns: Option<u64>,
}

impl DurationRun {
    fn add(&mut self, timestamp_ns: u64, duration_ns: u64, active: bool) {
        if !active {
            self.maximum_ns = self.maximum_ns.max(self.active_ns);
            self.active_ns = 0;
            self.previous_end_ns = None;
            return;
        }
        if self.previous_end_ns != Some(timestamp_ns) {
            self.maximum_ns = self.maximum_ns.max(self.active_ns);
            self.active_ns = 0;
        }
        self.active_ns = self.active_ns.saturating_add(u128::from(duration_ns));
        self.previous_end_ns = timestamp_ns.checked_add(duration_ns);
    }

    fn finish(mut self) -> u128 {
        self.maximum_ns = self.maximum_ns.max(self.active_ns);
        self.maximum_ns
    }
}

fn ppm(numerator: usize, denominator: usize) -> Option<u32> {
    (denominator > 0).then(|| {
        ((numerator as u128 * PPM_SCALE / denominator as u128).min(PPM_SCALE)) as u32
    })
}

fn meters_to_micrometers(meters: f64) -> u64 {
    if meters <= 0.0 {
        return 0;
    }
    (meters * 1_000_000.0).ceil().min(u64::MAX as f64) as u64
}

fn percentile(sorted: &[u64], numerator: usize) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (sorted.len() * numerator).div_ceil(100).max(1);
    sorted.get(rank.saturating_sub(1)).copied()
}

fn error_quantiles(mut values: Vec<u64>) -> ErrorQuantiles {
    values.sort_unstable();
    ErrorQuantiles {
        sample_count: values.len(),
        p50_micrometers: percentile(&values, 50),
        p95_micrometers: percentile(&values, 95),
        p99_micrometers: percentile(&values, 99),
        maximum_micrometers: values.last().copied(),
    }
}

/// Evaluate a timestamp/frame-matched trajectory against a separate reference.
///
/// Records are sorted by `(cluster_id, timestamp, record_id)` and support intervals
/// may not overlap within a cluster. Different sessions may use independent clock
/// origins, so overlap is not compared across clusters.
///
/// `official_pose_coverage_ppm` counts any supplied pose, including interpolation
/// and extrapolation. `evidence_backed_pose_coverage_ppm` counts only explicitly
/// qualified measurement support or bounded dead reckoning. A NORMAL disposition
/// with any other support type is measured as a policy violation, not hidden.
///
/// The challenge-compatible score is calculated only after the caller has performed
/// timestamp/frame matching and applied the benchmark's reference-validity window.
/// It applies the official exponential per-pose curve and the 99% coverage gate,
/// but does not implement ROS parsing, pose association, coordinate transforms or
/// the official evaluation server.
pub fn evaluate_trajectory_service_trace(
    records: &[TrajectoryServiceRecord],
) -> Result<TrajectoryServiceSummary, TrajectoryServiceError> {
    if records.is_empty() {
        return Err(TrajectoryServiceError::EmptyTrace);
    }
    let mut ids = BTreeSet::new();
    let mut clusters = BTreeSet::new();
    let mut ordered = records.to_vec();
    for record in &ordered {
        if record.record_id.trim().is_empty() {
            return Err(TrajectoryServiceError::EmptyRecordId);
        }
        if !ids.insert(record.record_id.clone()) {
            return Err(TrajectoryServiceError::DuplicateRecordId(record.record_id.clone()));
        }
        if record.cluster_id.trim().is_empty() {
            return Err(TrajectoryServiceError::EmptyClusterId);
        }
        clusters.insert(record.cluster_id.clone());
        if record.support_duration_ns == 0 {
            return Err(TrajectoryServiceError::ZeroSupportDuration(record.record_id.clone()));
        }
        if record.timestamp_ns.checked_add(record.support_duration_ns).is_none() {
            return Err(TrajectoryServiceError::SupportIntervalOverflow(record.record_id.clone()));
        }
        match (record.reference_valid, record.support.has_estimate(), record.position_error_m) {
            (true, true, Some(error)) if error.is_finite() && error >= 0.0 => {},
            (true, false, None) => {},
            (false, _, None) => {},
            (true, _, Some(error)) if !error.is_finite() || error < 0.0 => {
                return Err(TrajectoryServiceError::InvalidPositionError(record.record_id.clone()));
            }
            _ => return Err(TrajectoryServiceError::ReferenceValidityMismatch(record.record_id.clone())),
        }
    }

    ordered.sort_by(|a, b| {
        a.cluster_id.cmp(&b.cluster_id)
            .then_with(|| a.timestamp_ns.cmp(&b.timestamp_ns))
            .then_with(|| a.record_id.cmp(&b.record_id))
    });
    let mut previous: Option<&TrajectoryServiceRecord> = None;
    for current in &ordered {
        if let Some(prior) = previous {
            if prior.cluster_id == current.cluster_id {
                let prior_end = prior.timestamp_ns + prior.support_duration_ns;
                if current.timestamp_ns < prior_end {
                    return Err(TrajectoryServiceError::OverlappingSupportIntervals {
                        previous_record_id: prior.record_id.clone(),
                        current_record_id: current.record_id.clone(),
                    });
                }
            }
        }
        previous = Some(current);
    }

    let mut valid_reference_samples = 0_usize;
    let mut invalid_reference_samples = 0_usize;
    let mut valid_reference_exposure_ns = 0_u128;
    let mut invalid_reference_exposure_ns = 0_u128;
    let mut estimate_available_samples = 0_usize;
    let mut missing_estimate_samples = 0_usize;
    let mut evidence_backed_samples = 0_usize;
    let mut unqualified_output_samples = 0_usize;
    let mut measured_qualified_samples = 0_usize;
    let mut dead_reckoned_bounded_samples = 0_usize;
    let mut measured_unqualified_samples = 0_usize;
    let mut dead_reckoned_unbounded_samples = 0_usize;
    let mut interpolated_samples = 0_usize;
    let mut extrapolated_samples = 0_usize;

    let mut measured_qualified_exposure_ns = 0_u128;
    let mut dead_reckoned_bounded_exposure_ns = 0_u128;
    let mut measured_unqualified_exposure_ns = 0_u128;
    let mut dead_reckoned_unbounded_exposure_ns = 0_u128;
    let mut interpolated_exposure_ns = 0_u128;
    let mut extrapolated_exposure_ns = 0_u128;
    let mut missing_estimate_exposure_ns = 0_u128;
    let mut total_estimated_exposure_ns = 0_u128;
    let mut evidence_backed_exposure_ns = 0_u128;
    let mut unqualified_output_exposure_ns = 0_u128;

    let mut normal_without_qualified_support_samples = 0_usize;
    let mut normal_without_qualified_support_exposure_ns = 0_u128;
    let mut normal_exposure_ns = 0_u128;
    let mut degraded_exposure_ns = 0_u128;
    let mut integrity_uncertain_exposure_ns = 0_u128;
    let mut unavailable_exposure_ns = 0_u128;
    let mut recovering_exposure_ns = 0_u128;
    let mut indeterminate_exposure_ns = 0_u128;

    let mut missing_run = DurationRun::default();
    let mut unqualified_run = DurationRun::default();
    let mut previous_cluster: Option<&str> = None;
    let mut all_errors = Vec::new();
    let mut evidence_backed_errors = Vec::new();
    let mut unqualified_errors = Vec::new();

    for record in &ordered {
        let cluster_changed = previous_cluster.is_some_and(|cluster| cluster != record.cluster_id);
        if cluster_changed {
            missing_run.add(record.timestamp_ns, record.support_duration_ns, false);
            unqualified_run.add(record.timestamp_ns, record.support_duration_ns, false);
        }
        previous_cluster = Some(&record.cluster_id);

        let duration = u128::from(record.support_duration_ns);
        match record.service_state {
            TrajectoryServiceState::Normal => normal_exposure_ns = normal_exposure_ns.saturating_add(duration),
            TrajectoryServiceState::Degraded => degraded_exposure_ns = degraded_exposure_ns.saturating_add(duration),
            TrajectoryServiceState::IntegrityUncertain => integrity_uncertain_exposure_ns = integrity_uncertain_exposure_ns.saturating_add(duration),
            TrajectoryServiceState::Unavailable => unavailable_exposure_ns = unavailable_exposure_ns.saturating_add(duration),
            TrajectoryServiceState::Recovering => recovering_exposure_ns = recovering_exposure_ns.saturating_add(duration),
            TrajectoryServiceState::Indeterminate => indeterminate_exposure_ns = indeterminate_exposure_ns.saturating_add(duration),
        }

        match record.support {
            TrajectorySupport::MeasuredQualified => {
                measured_qualified_samples += usize::from(record.reference_valid);
                measured_qualified_exposure_ns = measured_qualified_exposure_ns.saturating_add(duration);
            }
            TrajectorySupport::DeadReckonedBounded => {
                dead_reckoned_bounded_samples += usize::from(record.reference_valid);
                dead_reckoned_bounded_exposure_ns = dead_reckoned_bounded_exposure_ns.saturating_add(duration);
            }
            TrajectorySupport::MeasuredUnqualified => {
                measured_unqualified_samples += usize::from(record.reference_valid);
                measured_unqualified_exposure_ns = measured_unqualified_exposure_ns.saturating_add(duration);
            }
            TrajectorySupport::DeadReckonedUnbounded => {
                dead_reckoned_unbounded_samples += usize::from(record.reference_valid);
                dead_reckoned_unbounded_exposure_ns = dead_reckoned_unbounded_exposure_ns.saturating_add(duration);
            }
            TrajectorySupport::Interpolated => {
                interpolated_samples += usize::from(record.reference_valid);
                interpolated_exposure_ns = interpolated_exposure_ns.saturating_add(duration);
            }
            TrajectorySupport::Extrapolated => {
                extrapolated_samples += usize::from(record.reference_valid);
                extrapolated_exposure_ns = extrapolated_exposure_ns.saturating_add(duration);
            }
            TrajectorySupport::Missing => {
                missing_estimate_samples += usize::from(record.reference_valid);
                missing_estimate_exposure_ns = missing_estimate_exposure_ns.saturating_add(duration);
            }
        }

        if record.support.has_estimate() {
            total_estimated_exposure_ns = total_estimated_exposure_ns.saturating_add(duration);
        }
        if record.support.evidence_backed() {
            evidence_backed_exposure_ns = evidence_backed_exposure_ns.saturating_add(duration);
        }
        if record.support.unqualified_output() {
            unqualified_output_exposure_ns = unqualified_output_exposure_ns.saturating_add(duration);
        }
        if record.service_state == TrajectoryServiceState::Normal && !record.support.evidence_backed() {
            normal_without_qualified_support_samples += usize::from(record.reference_valid);
            normal_without_qualified_support_exposure_ns = normal_without_qualified_support_exposure_ns.saturating_add(duration);
        }

        missing_run.add(record.timestamp_ns, record.support_duration_ns, record.support == TrajectorySupport::Missing);
        unqualified_run.add(record.timestamp_ns, record.support_duration_ns, record.support.unqualified_output());

        if !record.reference_valid {
            invalid_reference_samples += 1;
            invalid_reference_exposure_ns = invalid_reference_exposure_ns.saturating_add(duration);
            continue;
        }
        valid_reference_samples += 1;
        valid_reference_exposure_ns = valid_reference_exposure_ns.saturating_add(duration);
        if record.support.has_estimate() {
            estimate_available_samples += 1;
            let error = record.position_error_m.expect("validated matched position error");
            let quantized = meters_to_micrometers(error);
            all_errors.push(quantized);
            if record.support.evidence_backed() {
                evidence_backed_samples += 1;
                evidence_backed_errors.push(quantized);
            } else {
                unqualified_output_samples += 1;
                unqualified_errors.push(quantized);
            }
        }
    }

    let official_pose_coverage_ppm = ppm(estimate_available_samples, valid_reference_samples);
    let evidence_backed_pose_coverage_ppm = ppm(evidence_backed_samples, valid_reference_samples);
    let unqualified_output_coverage_ppm = ppm(unqualified_output_samples, valid_reference_samples);
    let official_coverage_gate_met = official_pose_coverage_ppm.map(|value| value >= HILTI_COVERAGE_GATE_PPM);
    let challenge_compatible_exponential_score = official_coverage_gate_met.map(|gate_met| {
        if !gate_met || all_errors.is_empty() {
            0.0
        } else {
            all_errors.iter().map(|error_um| {
                let error_m = *error_um as f64 / 1_000_000.0;
                100.0 * (-HILTI_EXPONENTIAL_SCORE_C * error_m).exp()
            }).sum::<f64>() / all_errors.len() as f64
        }
    });

    Ok(TrajectoryServiceSummary {
        record_count: ordered.len(),
        distinct_cluster_count: clusters.len(),
        valid_reference_samples,
        invalid_reference_samples,
        valid_reference_exposure_ns,
        invalid_reference_exposure_ns,
        estimate_available_samples,
        missing_estimate_samples,
        evidence_backed_samples,
        unqualified_output_samples,
        measured_qualified_samples,
        dead_reckoned_bounded_samples,
        measured_unqualified_samples,
        dead_reckoned_unbounded_samples,
        interpolated_samples,
        extrapolated_samples,
        official_pose_coverage_ppm,
        evidence_backed_pose_coverage_ppm,
        unqualified_output_coverage_ppm,
        official_coverage_gate_met,
        challenge_compatible_exponential_score,
        measured_qualified_exposure_ns,
        dead_reckoned_bounded_exposure_ns,
        measured_unqualified_exposure_ns,
        dead_reckoned_unbounded_exposure_ns,
        interpolated_exposure_ns,
        extrapolated_exposure_ns,
        missing_estimate_exposure_ns,
        total_estimated_exposure_ns,
        evidence_backed_exposure_ns,
        unqualified_output_exposure_ns,
        normal_without_qualified_support_samples,
        normal_without_qualified_support_exposure_ns,
        maximum_contiguous_missing_estimate_ns: missing_run.finish(),
        maximum_contiguous_unqualified_output_ns: unqualified_run.finish(),
        normal_exposure_ns,
        degraded_exposure_ns,
        integrity_uncertain_exposure_ns,
        unavailable_exposure_ns,
        recovering_exposure_ns,
        indeterminate_exposure_ns,
        all_estimate_errors: error_quantiles(all_errors),
        evidence_backed_errors: error_quantiles(evidence_backed_errors),
        unqualified_output_errors: error_quantiles(unqualified_errors),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(
        id: &str,
        cluster: &str,
        timestamp_ms: u64,
        duration_ms: u64,
        reference_valid: bool,
        support: TrajectorySupport,
        state: TrajectoryServiceState,
        error_m: Option<f64>,
    ) -> TrajectoryServiceRecord {
        TrajectoryServiceRecord {
            record_id: id.to_owned(),
            cluster_id: cluster.to_owned(),
            timestamp_ns: timestamp_ms * 1_000_000,
            support_duration_ns: duration_ms * 1_000_000,
            reference_valid,
            support,
            service_state: state,
            position_error_m: error_m,
        }
    }

    #[test]
    fn official_coverage_and_evidence_backed_coverage_are_reported_separately() {
        let mut records = Vec::new();
        for index in 0..80 {
            records.push(rec(&format!("m-{index}"), "session-a", index, 1, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)));
        }
        for index in 80..90 {
            records.push(rec(&format!("d-{index}"), "session-a", index, 1, true, TrajectorySupport::DeadReckonedBounded, TrajectoryServiceState::Normal, Some(0.2)));
        }
        for index in 90..95 {
            records.push(rec(&format!("i-{index}"), "session-a", index, 1, true, TrajectorySupport::Interpolated, TrajectoryServiceState::Degraded, Some(0.3)));
        }
        for index in 95..99 {
            records.push(rec(&format!("x-{index}"), "session-a", index, 1, true, TrajectorySupport::Extrapolated, TrajectoryServiceState::IntegrityUncertain, Some(0.4)));
        }
        records.push(rec("missing", "session-a", 99, 1, true, TrajectorySupport::Missing, TrajectoryServiceState::Unavailable, None));
        let summary = evaluate_trajectory_service_trace(&records).unwrap();
        assert_eq!(summary.official_pose_coverage_ppm, Some(990_000));
        assert_eq!(summary.evidence_backed_pose_coverage_ppm, Some(900_000));
        assert_eq!(summary.unqualified_output_coverage_ppm, Some(90_000));
        assert_eq!(summary.official_coverage_gate_met, Some(true));
        assert!(summary.challenge_compatible_exponential_score.unwrap() > 90.0);
        assert_eq!(summary.normal_without_qualified_support_samples, 0);
        assert_eq!(summary.measured_qualified_samples, 80);
        assert_eq!(summary.dead_reckoned_bounded_samples, 10);
        assert_eq!(summary.interpolated_samples, 5);
        assert_eq!(summary.extrapolated_samples, 4);
        assert_eq!(summary.missing_estimate_samples, 1);
    }

    #[test]
    fn full_official_coverage_can_coexist_with_unsupported_normal_output() {
        let records = vec![
            rec("measured", "session-a", 0, 10, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.05)),
            rec("filled", "session-a", 10, 10, true, TrajectorySupport::Interpolated, TrajectoryServiceState::Normal, Some(0.01)),
        ];
        let summary = evaluate_trajectory_service_trace(&records).unwrap();
        assert_eq!(summary.official_pose_coverage_ppm, Some(1_000_000));
        assert_eq!(summary.evidence_backed_pose_coverage_ppm, Some(500_000));
        assert_eq!(summary.normal_without_qualified_support_samples, 1);
        assert_eq!(summary.normal_without_qualified_support_exposure_ns, 10_000_000);
    }

    #[test]
    fn below_99_percent_coverage_zeroes_challenge_compatible_score() {
        let mut records = Vec::new();
        for index in 0..98 {
            records.push(rec(&format!("p-{index}"), "session-a", index, 1, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)));
        }
        for index in 98..100 {
            records.push(rec(&format!("m-{index}"), "session-a", index, 1, true, TrajectorySupport::Missing, TrajectoryServiceState::Unavailable, None));
        }
        let summary = evaluate_trajectory_service_trace(&records).unwrap();
        assert_eq!(summary.official_pose_coverage_ppm, Some(980_000));
        assert_eq!(summary.official_coverage_gate_met, Some(false));
        assert_eq!(summary.challenge_compatible_exponential_score, Some(0.0));
    }

    #[test]
    fn invalid_reference_samples_are_reported_not_counted_as_safe_or_covered() {
        let records = vec![
            rec("valid", "session-a", 0, 10, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)),
            rec("invalid", "session-a", 10, 10, false, TrajectorySupport::Missing, TrajectoryServiceState::Indeterminate, None),
        ];
        let summary = evaluate_trajectory_service_trace(&records).unwrap();
        assert_eq!(summary.valid_reference_samples, 1);
        assert_eq!(summary.invalid_reference_samples, 1);
        assert_eq!(summary.official_pose_coverage_ppm, Some(1_000_000));
        assert_eq!(summary.invalid_reference_exposure_ns, 10_000_000);
        assert_eq!(summary.indeterminate_exposure_ns, 10_000_000);
    }

    #[test]
    fn explicit_gaps_and_cluster_boundaries_split_unqualified_runs() {
        let records = vec![
            rec("a", "session-a", 0, 10, true, TrajectorySupport::Interpolated, TrajectoryServiceState::Degraded, Some(0.2)),
            rec("b", "session-a", 20, 10, true, TrajectorySupport::Extrapolated, TrajectoryServiceState::IntegrityUncertain, Some(0.3)),
            rec("c", "session-b", 30, 10, true, TrajectorySupport::Extrapolated, TrajectoryServiceState::IntegrityUncertain, Some(0.3)),
        ];
        let summary = evaluate_trajectory_service_trace(&records).unwrap();
        assert_eq!(summary.maximum_contiguous_unqualified_output_ns, 10_000_000);
    }

    #[test]
    fn duplicate_ids_overlaps_and_invalid_error_shapes_fail_closed() {
        let duplicate = vec![
            rec("same", "session-a", 0, 10, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)),
            rec("same", "session-a", 10, 10, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)),
        ];
        assert!(matches!(evaluate_trajectory_service_trace(&duplicate), Err(TrajectoryServiceError::DuplicateRecordId(_))));

        let overlap = vec![
            rec("a", "session-a", 0, 20, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)),
            rec("b", "session-a", 10, 20, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1)),
        ];
        assert!(matches!(evaluate_trajectory_service_trace(&overlap), Err(TrajectoryServiceError::OverlappingSupportIntervals { .. })));

        let invalid = vec![rec("bad", "session-a", 0, 10, true, TrajectorySupport::Missing, TrajectoryServiceState::Unavailable, Some(0.1))];
        assert!(matches!(evaluate_trajectory_service_trace(&invalid), Err(TrajectoryServiceError::ReferenceValidityMismatch(_))));
    }

    #[test]
    fn zero_support_and_timestamp_overflow_fail_closed() {
        let mut zero = rec("zero", "session-a", 0, 10, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1));
        zero.support_duration_ns = 0;
        assert!(matches!(evaluate_trajectory_service_trace(&[zero]), Err(TrajectoryServiceError::ZeroSupportDuration(_))));

        let mut overflow = rec("overflow", "session-a", 0, 10, true, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Normal, Some(0.1));
        overflow.timestamp_ns = u64::MAX - 2;
        assert!(matches!(evaluate_trajectory_service_trace(&[overflow]), Err(TrajectoryServiceError::SupportIntervalOverflow(_))));
    }

    #[test]
    fn empty_trace_and_unreferenced_error_fail_closed() {
        assert_eq!(evaluate_trajectory_service_trace(&[]), Err(TrajectoryServiceError::EmptyTrace));
        let invalid = vec![rec("invalid", "session-a", 0, 10, false, TrajectorySupport::MeasuredQualified, TrajectoryServiceState::Degraded, Some(0.1))];
        assert!(matches!(evaluate_trajectory_service_trace(&invalid), Err(TrajectoryServiceError::ReferenceValidityMismatch(_))));
    }
}

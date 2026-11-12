//! Uncertainty-bounded event-time alignment for multimodal shipyard telemetry.
//!
//! V0 is an offline synthetic contract. It preserves each stream's original
//! cadence, maps source timestamps with checked integer arithmetic, and emits
//! an event-window receipt. It does not resample, interpolate, or authorize
//! physical decisions.

const PPB_SCALE: i128 = 1_000_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventWindow {
    pub event_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub pass_id: String,
    pub master_clock_domain: String,
    pub start_master_ns: i64,
    pub end_master_ns: i64,
    pub decision_cutoff_master_ns: i64,
    pub boundary_uncertainty_ns: u64,
    pub maximum_alignment_uncertainty_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamIdentity {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub pass_id: String,
    pub source_clock_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelContract {
    pub channel_id: String,
    pub source_clock_domain: String,
    pub nominal_interval_ns: u64,
    pub maximum_jitter_ns: u64,
    pub maximum_gap_ns: u64,
    pub source_timestamp_uncertainty_ns: u64,
    pub minimum_coverage_permille: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockTransform {
    pub source_clock_domain: String,
    pub master_clock_domain: String,
    pub reference_source_ns: i64,
    pub reference_master_ns: i64,
    pub drift_ppb: i64,
    pub drift_uncertainty_ppb: u64,
    pub transform_uncertainty_ns: u64,
    pub valid_from_source_ns: i64,
    pub valid_until_source_ns: i64,
    pub transform_sha256: [u8; 32],
    pub calibration_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleQuality {
    pub valid: bool,
    pub clipped: bool,
    pub dropped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetrySample {
    pub sequence: u64,
    pub source_time_ns: i64,
    pub arrival_master_ns: Option<i64>,
    pub sample_sha256: [u8; 32],
    pub quality: SampleQuality,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamInput {
    pub identity: StreamIdentity,
    pub source_stream_sha256: [u8; 32],
    pub samples: Vec<TelemetrySample>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExclusionCounts {
    pub outside_event: usize,
    pub boundary_ambiguous: usize,
    pub invalid: usize,
    pub clipped: usize,
    pub dropped: usize,
    pub arrived_after_cutoff: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignmentReceipt {
    pub event_id: String,
    pub channel_id: String,
    pub source_stream_sha256: [u8; 32],
    pub transform_sha256: [u8; 32],
    pub calibration_sha256: [u8; 32],
    pub first_source_time_ns: i64,
    pub last_source_time_ns: i64,
    pub first_master_time_ns: i64,
    pub last_master_time_ns: i64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub selected_sequences: Vec<u64>,
    pub expected_minimum_sample_count: u64,
    pub selected_sample_count: u64,
    pub coverage_permille: u16,
    pub maximum_observed_gap_ns: u64,
    pub maximum_alignment_uncertainty_ns: u64,
    pub exclusions: ExclusionCounts,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignmentFailure {
    EmptyIdentity,
    InvalidEventWindow,
    DecisionCutoffBeforeEventEnd,
    SourceStreamDigestMissing,
    TransformDigestMissing,
    CalibrationDigestMissing,
    UnknownClockDomain,
    InvalidChannelCadence,
    InvalidCoverageThreshold,
    InvalidTransformInterval,
    TransformOutsideCalibration,
    InvalidClockRate,
    ArithmeticOverflow,
    SampleDigestMissing,
    NonIncreasingSequence,
    NonIncreasingSourceTime,
    UncertaintyBudgetExceeded,
    NoEligibleSamples,
    InsufficientTemporalResolution,
    MaximumGapExceeded,
    CoverageBelowMinimum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappedTime {
    pub master_time_ns: i64,
    pub uncertainty_ns: u64,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn digest_present(digest: &[u8; 32]) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn checked_u64(value: u128) -> Result<u64, AlignmentFailure> {
    u64::try_from(value).map_err(|_| AlignmentFailure::ArithmeticOverflow)
}

impl ClockTransform {
    /// Map one source timestamp into the master clock without floating-point
    /// arithmetic. The uncertainty grows with distance from the calibration
    /// reference according to the declared drift uncertainty.
    pub fn map_source_time(
        &self,
        source_time_ns: i64,
        source_timestamp_uncertainty_ns: u64,
    ) -> Result<MappedTime, AlignmentFailure> {
        if self.valid_from_source_ns > self.valid_until_source_ns {
            return Err(AlignmentFailure::InvalidTransformInterval);
        }
        if source_time_ns < self.valid_from_source_ns
            || source_time_ns > self.valid_until_source_ns
        {
            return Err(AlignmentFailure::TransformOutsideCalibration);
        }
        if !digest_present(&self.transform_sha256) {
            return Err(AlignmentFailure::TransformDigestMissing);
        }
        if !digest_present(&self.calibration_sha256) {
            return Err(AlignmentFailure::CalibrationDigestMissing);
        }

        let rate = PPB_SCALE
            .checked_add(i128::from(self.drift_ppb))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        if rate <= 0 {
            return Err(AlignmentFailure::InvalidClockRate);
        }

        let delta = i128::from(source_time_ns)
            .checked_sub(i128::from(self.reference_source_ns))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        let scaled_delta = delta
            .checked_mul(rate)
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        let corrected_delta = scaled_delta / PPB_SCALE;
        let master_time = i128::from(self.reference_master_ns)
            .checked_add(corrected_delta)
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        let master_time_ns =
            i64::try_from(master_time).map_err(|_| AlignmentFailure::ArithmeticOverflow)?;

        let distance = delta.unsigned_abs();
        let drift_growth = distance
            .checked_mul(u128::from(self.drift_uncertainty_ppb))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?
            / u128::try_from(PPB_SCALE).expect("positive scale");
        let total_uncertainty = u128::from(self.transform_uncertainty_ns)
            .checked_add(u128::from(source_timestamp_uncertainty_ns))
            .and_then(|value| value.checked_add(drift_growth))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;

        Ok(MappedTime {
            master_time_ns,
            uncertainty_ns: checked_u64(total_uncertainty)?,
        })
    }
}

/// Conservative lower bound on how many samples should exist in an event
/// window, assuming no interval exceeds nominal cadence plus allowed jitter.
pub fn minimum_expected_sample_count(
    duration_ns: u64,
    nominal_interval_ns: u64,
    maximum_jitter_ns: u64,
) -> Result<u64, AlignmentFailure> {
    if nominal_interval_ns == 0 {
        return Err(AlignmentFailure::InvalidChannelCadence);
    }
    let maximum_interval = nominal_interval_ns
        .checked_add(maximum_jitter_ns)
        .ok_or(AlignmentFailure::ArithmeticOverflow)?;
    if maximum_interval == 0 {
        return Err(AlignmentFailure::InvalidChannelCadence);
    }
    let count = duration_ns / maximum_interval;
    if count == 0 {
        return Err(AlignmentFailure::InsufficientTemporalResolution);
    }
    Ok(count)
}

/// Align a source stream to one physical event without resampling or
/// interpolating. Only samples whose entire time-uncertainty envelope lies
/// within the half-open event window [start, end) are selected.
pub fn align_stream(
    event: &EventWindow,
    channel: &ChannelContract,
    transform: &ClockTransform,
    stream: &StreamInput,
) -> Result<AlignmentReceipt, AlignmentFailure> {
    let identity_values = [
        event.event_id.as_str(),
        event.workpiece_id.as_str(),
        event.joint_revision.as_str(),
        event.pass_id.as_str(),
        event.master_clock_domain.as_str(),
        channel.channel_id.as_str(),
        channel.source_clock_domain.as_str(),
        transform.source_clock_domain.as_str(),
        transform.master_clock_domain.as_str(),
        stream.identity.workpiece_id.as_str(),
        stream.identity.joint_revision.as_str(),
        stream.identity.pass_id.as_str(),
        stream.identity.source_clock_domain.as_str(),
    ];
    if identity_values.iter().any(|value| !nonempty(value)) {
        return Err(AlignmentFailure::EmptyIdentity);
    }
    if event.start_master_ns >= event.end_master_ns {
        return Err(AlignmentFailure::InvalidEventWindow);
    }
    if event.decision_cutoff_master_ns < event.end_master_ns {
        return Err(AlignmentFailure::DecisionCutoffBeforeEventEnd);
    }
    if !digest_present(&stream.source_stream_sha256) {
        return Err(AlignmentFailure::SourceStreamDigestMissing);
    }
    if !digest_present(&transform.transform_sha256) {
        return Err(AlignmentFailure::TransformDigestMissing);
    }
    if !digest_present(&transform.calibration_sha256) {
        return Err(AlignmentFailure::CalibrationDigestMissing);
    }
    if stream.identity.workpiece_id != event.workpiece_id
        || stream.identity.joint_revision != event.joint_revision
        || stream.identity.pass_id != event.pass_id
    {
        return Err(AlignmentFailure::EmptyIdentity);
    }
    if stream.identity.source_clock_domain != channel.source_clock_domain
        || transform.source_clock_domain != channel.source_clock_domain
        || transform.master_clock_domain != event.master_clock_domain
    {
        return Err(AlignmentFailure::UnknownClockDomain);
    }
    if channel.nominal_interval_ns == 0 || channel.maximum_gap_ns == 0 {
        return Err(AlignmentFailure::InvalidChannelCadence);
    }
    if u64::from(channel.minimum_coverage_permille) > 1_000 {
        return Err(AlignmentFailure::InvalidCoverageThreshold);
    }

    let duration_i128 = i128::from(event.end_master_ns)
        .checked_sub(i128::from(event.start_master_ns))
        .ok_or(AlignmentFailure::ArithmeticOverflow)?;
    let duration_ns =
        u64::try_from(duration_i128).map_err(|_| AlignmentFailure::ArithmeticOverflow)?;
    let expected_minimum_sample_count = minimum_expected_sample_count(
        duration_ns,
        channel.nominal_interval_ns,
        channel.maximum_jitter_ns,
    )?;

    let mut exclusions = ExclusionCounts {
        outside_event: 0,
        boundary_ambiguous: 0,
        invalid: 0,
        clipped: 0,
        dropped: 0,
        arrived_after_cutoff: 0,
    };
    let mut selected = Vec::new();
    let mut last_sequence = None;
    let mut last_source_time = None;

    for sample in &stream.samples {
        if let Some(previous) = last_sequence {
            if sample.sequence <= previous {
                return Err(AlignmentFailure::NonIncreasingSequence);
            }
        }
        if let Some(previous) = last_source_time {
            if sample.source_time_ns <= previous {
                return Err(AlignmentFailure::NonIncreasingSourceTime);
            }
        }
        last_sequence = Some(sample.sequence);
        last_source_time = Some(sample.source_time_ns);

        if !digest_present(&sample.sample_sha256) {
            return Err(AlignmentFailure::SampleDigestMissing);
        }
        if let Some(arrival_time) = sample.arrival_master_ns {
            if arrival_time > event.decision_cutoff_master_ns {
                exclusions.arrived_after_cutoff += 1;
                continue;
            }
        }
        if sample.quality.clipped {
            exclusions.clipped += 1;
            continue;
        }
        if sample.quality.dropped {
            exclusions.dropped += 1;
            continue;
        }
        if !sample.quality.valid {
            exclusions.invalid += 1;
            continue;
        }

        let mapped = transform.map_source_time(
            sample.source_time_ns,
            channel.source_timestamp_uncertainty_ns,
        )?;
        let total_uncertainty = mapped
            .uncertainty_ns
            .checked_add(event.boundary_uncertainty_ns)
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        if total_uncertainty > event.maximum_alignment_uncertainty_ns {
            return Err(AlignmentFailure::UncertaintyBudgetExceeded);
        }

        let lower_bound = i128::from(mapped.master_time_ns)
            .checked_sub(i128::from(total_uncertainty))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        let upper_bound = i128::from(mapped.master_time_ns)
            .checked_add(i128::from(total_uncertainty))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        let event_start = i128::from(event.start_master_ns);
        let event_end = i128::from(event.end_master_ns);

        if upper_bound < event_start || lower_bound >= event_end {
            exclusions.outside_event += 1;
            continue;
        }
        if lower_bound < event_start || upper_bound >= event_end {
            exclusions.boundary_ambiguous += 1;
            continue;
        }

        selected.push((
            sample.sequence,
            sample.source_time_ns,
            mapped.master_time_ns,
            total_uncertainty,
        ));
    }

    if selected.is_empty() {
        return Err(AlignmentFailure::NoEligibleSamples);
    }

    let mut maximum_observed_gap_ns = 0u64;
    let mut previous_master = event.start_master_ns;
    for item in &selected {
        let gap = i128::from(item.2)
            .checked_sub(i128::from(previous_master))
            .ok_or(AlignmentFailure::ArithmeticOverflow)?;
        let gap_ns = u64::try_from(gap).map_err(|_| AlignmentFailure::ArithmeticOverflow)?;
        maximum_observed_gap_ns = maximum_observed_gap_ns.max(gap_ns);
        previous_master = item.2;
    }
    let trailing_gap = i128::from(event.end_master_ns)
        .checked_sub(i128::from(previous_master))
        .ok_or(AlignmentFailure::ArithmeticOverflow)?;
    let trailing_gap_ns =
        u64::try_from(trailing_gap).map_err(|_| AlignmentFailure::ArithmeticOverflow)?;
    maximum_observed_gap_ns = maximum_observed_gap_ns.max(trailing_gap_ns);

    if maximum_observed_gap_ns > channel.maximum_gap_ns {
        return Err(AlignmentFailure::MaximumGapExceeded);
    }

    let selected_count =
        u64::try_from(selected.len()).map_err(|_| AlignmentFailure::ArithmeticOverflow)?;
    let coverage_numerator = selected_count
        .checked_mul(1_000)
        .ok_or(AlignmentFailure::ArithmeticOverflow)?;
    let coverage_permille = ((coverage_numerator / expected_minimum_sample_count).min(1_000))
        as u16;
    if coverage_permille < channel.minimum_coverage_permille {
        return Err(AlignmentFailure::CoverageBelowMinimum);
    }

    let first = selected.first().expect("nonempty selected samples");
    let last = selected.last().expect("nonempty selected samples");
    let maximum_alignment_uncertainty_ns =
        selected.iter().map(|sample| sample.3).max().unwrap_or(0);

    Ok(AlignmentReceipt {
        event_id: event.event_id.clone(),
        channel_id: channel.channel_id.clone(),
        source_stream_sha256: stream.source_stream_sha256,
        transform_sha256: transform.transform_sha256,
        calibration_sha256: transform.calibration_sha256,
        first_source_time_ns: first.1,
        last_source_time_ns: last.1,
        first_master_time_ns: first.2,
        last_master_time_ns: last.2,
        first_sequence: first.0,
        last_sequence: last.0,
        selected_sequences: selected.iter().map(|sample| sample.0).collect(),
        expected_minimum_sample_count,
        selected_sample_count: selected_count,
        coverage_permille,
        maximum_observed_gap_ns,
        maximum_alignment_uncertainty_ns,
        exclusions,
        claim_ceiling: "EVENT_TIME_ALIGNMENT_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn base_input() -> (EventWindow, ChannelContract, ClockTransform, StreamInput) {
        let event = EventWindow {
            event_id: "EVENT-PASS-001".to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            pass_id: "PASS-01".to_owned(),
            master_clock_domain: "MASTER-UTC".to_owned(),
            start_master_ns: 0,
            end_master_ns: 100_000_000,
            decision_cutoff_master_ns: 200_000_000,
            boundary_uncertainty_ns: 0,
            maximum_alignment_uncertainty_ns: 1_000,
        };
        let channel = ChannelContract {
            channel_id: "process.current".to_owned(),
            source_clock_domain: "DEVICE-A".to_owned(),
            nominal_interval_ns: 10_000_000,
            maximum_jitter_ns: 0,
            maximum_gap_ns: 10_000_000,
            source_timestamp_uncertainty_ns: 50,
            minimum_coverage_permille: 1_000,
        };
        let transform = ClockTransform {
            source_clock_domain: "DEVICE-A".to_owned(),
            master_clock_domain: "MASTER-UTC".to_owned(),
            reference_source_ns: 0,
            reference_master_ns: 0,
            drift_ppb: 0,
            drift_uncertainty_ppb: 0,
            transform_uncertainty_ns: 100,
            valid_from_source_ns: -1_000_000,
            valid_until_source_ns: 200_000_000,
            transform_sha256: digest(3),
            calibration_sha256: digest(4),
        };
        let samples = (0..10)
            .map(|index| {
                let source_time_ns = i64::from(index) * 10_000_000;
                TelemetrySample {
                    sequence: index,
                    source_time_ns,
                    arrival_master_ns: Some(source_time_ns),
                    sample_sha256: digest(index as u8 + 10),
                    quality: SampleQuality {
                        valid: true,
                        clipped: false,
                        dropped: false,
                    },
                }
            })
            .collect();
        let stream = StreamInput {
            identity: StreamIdentity {
                workpiece_id: "WID-SYNTH-01".to_owned(),
                joint_revision: "JOINT-R1".to_owned(),
                pass_id: "PASS-01".to_owned(),
                source_clock_domain: "DEVICE-A".to_owned(),
            },
            source_stream_sha256: digest(5),
            samples,
        };
        (event, channel, transform, stream)
    }

    #[test]
    fn exact_identity_alignment_preserves_original_samples() {
        let (event, channel, transform, stream) = base_input();
        let receipt = align_stream(&event, &channel, &transform, &stream).unwrap();
        assert_eq!(receipt.selected_sample_count, 10);
        assert_eq!(receipt.selected_sequences, (0..10).collect::<Vec<_>>());
        assert_eq!(receipt.coverage_permille, 1_000);
        assert_eq!(receipt.maximum_observed_gap_ns, 10_000_000);
        assert_eq!(receipt.claim_ceiling, "EVENT_TIME_ALIGNMENT_ONLY");
    }

    #[test]
    fn positive_clock_offset_is_applied() {
        let (mut event, _, mut transform, stream) = base_input();
        transform.reference_master_ns = 1_000;
        event.start_master_ns = 1_000;
        event.end_master_ns = 100_001_000;
        event.decision_cutoff_master_ns = 200_001_000;
        let mapped = transform.map_source_time(10_000_000, 50).unwrap();
        assert_eq!(mapped.master_time_ns, 10_001_000);
    }

    #[test]
    fn negative_clock_offset_is_applied() {
        let (_, _, mut transform, _) = base_input();
        transform.reference_master_ns = -1_000;
        let mapped = transform.map_source_time(10_000_000, 50).unwrap();
        assert_eq!(mapped.master_time_ns, 9_999_000);
    }

    #[test]
    fn positive_drift_is_applied_with_integer_arithmetic() {
        let (_, _, mut transform, _) = base_input();
        transform.drift_ppb = 1_000;
        let mapped = transform.map_source_time(100_000_000, 0).unwrap();
        assert_eq!(mapped.master_time_ns, 100_000_100);
    }

    #[test]
    fn negative_drift_is_applied_with_integer_arithmetic() {
        let (_, _, mut transform, _) = base_input();
        transform.drift_ppb = -1_000;
        let mapped = transform.map_source_time(100_000_000, 0).unwrap();
        assert_eq!(mapped.master_time_ns, 99_999_900);
    }

    #[test]
    fn drift_uncertainty_grows_with_distance_from_reference() {
        let (_, _, mut transform, _) = base_input();
        transform.drift_uncertainty_ppb = 1_000;
        let mapped = transform.map_source_time(100_000_000, 0).unwrap();
        assert_eq!(mapped.uncertainty_ns, 200);
    }

    #[test]
    fn transform_outside_calibration_interval_is_rejected() {
        let (_, _, mut transform, _) = base_input();
        transform.valid_until_source_ns = 50_000_000;
        assert_eq!(
            transform.map_source_time(100_000_000, 0),
            Err(AlignmentFailure::TransformOutsideCalibration)
        );
    }

    #[test]
    fn nonpositive_clock_rate_is_rejected() {
        let (_, _, mut transform, _) = base_input();
        transform.drift_ppb = -1_000_000_001;
        assert_eq!(
            transform.map_source_time(10_000_000, 0),
            Err(AlignmentFailure::InvalidClockRate)
        );
    }

    #[test]
    fn wrong_source_clock_domain_is_rejected() {
        let (event, channel, mut transform, stream) = base_input();
        transform.source_clock_domain = "DEVICE-B".to_owned();
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::UnknownClockDomain)
        );
    }

    #[test]
    fn wrong_workpiece_or_pass_identity_is_rejected() {
        let (event, channel, transform, mut stream) = base_input();
        stream.identity.workpiece_id = "WID-OTHER".to_owned();
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::EmptyIdentity)
        );
    }

    #[test]
    fn duplicate_or_decreasing_sequence_is_rejected() {
        let (event, channel, transform, mut stream) = base_input();
        stream.samples[4].sequence = 2;
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::NonIncreasingSequence)
        );
    }

    #[test]
    fn nonincreasing_source_timestamp_is_rejected() {
        let (event, channel, transform, mut stream) = base_input();
        stream.samples[4].source_time_ns = stream.samples[3].source_time_ns;
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::NonIncreasingSourceTime)
        );
    }

    #[test]
    fn missing_source_stream_digest_is_rejected() {
        let (event, channel, transform, mut stream) = base_input();
        stream.source_stream_sha256 = [0; 32];
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::SourceStreamDigestMissing)
        );
    }

    #[test]
    fn missing_sample_digest_is_rejected() {
        let (event, channel, transform, mut stream) = base_input();
        stream.samples[3].sample_sha256 = [0; 32];
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::SampleDigestMissing)
        );
    }

    #[test]
    fn end_boundary_is_exclusive() {
        let (event, channel, transform, mut stream) = base_input();
        let mut end_sample = stream.samples[9].clone();
        end_sample.sequence = 10;
        end_sample.source_time_ns = event.end_master_ns;
        end_sample.arrival_master_ns = Some(event.end_master_ns);
        stream.samples.push(end_sample);
        let receipt = align_stream(&event, &channel, &transform, &stream).unwrap();
        assert_eq!(receipt.selected_sample_count, 10);
        assert_eq!(receipt.exclusions.outside_event, 1);
    }

    #[test]
    fn boundary_uncertain_sample_is_not_silently_assigned() {
        let (mut event, mut channel, transform, mut stream) = base_input();
        event.maximum_alignment_uncertainty_ns = 1_000;
        channel.minimum_coverage_permille = 900;
        stream.samples[0].source_time_ns = -50;
        stream.samples[0].arrival_master_ns = Some(0);
        let receipt = align_stream(&event, &channel, &transform, &stream).unwrap();
        assert_eq!(receipt.exclusions.boundary_ambiguous, 1);
        assert_eq!(receipt.selected_sample_count, 9);
    }

    #[test]
    fn clipped_and_dropped_samples_do_not_count_toward_coverage() {
        let (event, mut channel, transform, mut stream) = base_input();
        channel.minimum_coverage_permille = 800;
        channel.maximum_gap_ns = 20_000_000;
        stream.samples[2].quality.clipped = true;
        stream.samples[7].quality.dropped = true;
        let receipt = align_stream(&event, &channel, &transform, &stream).unwrap();
        assert_eq!(receipt.exclusions.clipped, 1);
        assert_eq!(receipt.exclusions.dropped, 1);
        assert_eq!(receipt.selected_sample_count, 8);
        assert_eq!(receipt.coverage_permille, 800);
    }

    #[test]
    fn late_arrivals_after_decision_cutoff_are_excluded() {
        let (event, mut channel, transform, mut stream) = base_input();
        channel.minimum_coverage_permille = 900;
        stream.samples[0].arrival_master_ns = Some(event.decision_cutoff_master_ns + 1);
        let receipt = align_stream(&event, &channel, &transform, &stream).unwrap();
        assert_eq!(receipt.exclusions.arrived_after_cutoff, 1);
        assert_eq!(receipt.selected_sample_count, 9);
    }

    #[test]
    fn maximum_gap_violation_is_rejected() {
        let (event, channel, transform, mut stream) = base_input();
        stream.samples.remove(5);
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::MaximumGapExceeded)
        );
    }

    #[test]
    fn insufficient_channel_cadence_for_event_duration_is_rejected() {
        assert_eq!(
            minimum_expected_sample_count(5_000_000, 10_000_000, 0),
            Err(AlignmentFailure::InsufficientTemporalResolution)
        );
    }

    #[test]
    fn uncertainty_above_event_budget_is_rejected() {
        let (mut event, _, mut transform, stream) = base_input();
        event.maximum_alignment_uncertainty_ns = 100;
        transform.transform_uncertainty_ns = 200;
        let (event, channel, transform, stream) = (event, base_input().1, transform, stream);
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::UncertaintyBudgetExceeded)
        );
    }

    #[test]
    fn different_channel_cadences_produce_different_expected_counts() {
        let duration = 1_000_000_000;
        let process = minimum_expected_sample_count(duration, 100_000, 0).unwrap();
        let robot = minimum_expected_sample_count(duration, 8_000_000, 0).unwrap();
        let power_source = minimum_expected_sample_count(duration, 100_000_000, 0).unwrap();
        assert_eq!(process, 10_000);
        assert_eq!(robot, 125);
        assert_eq!(power_source, 10);
    }

    #[test]
    fn missing_transform_digest_is_rejected() {
        let (_, _, mut transform, _) = base_input();
        transform.transform_sha256 = [0; 32];
        assert_eq!(
            transform.map_source_time(10_000_000, 0),
            Err(AlignmentFailure::TransformDigestMissing)
        );
    }

    #[test]
    fn missing_calibration_digest_is_rejected() {
        let (_, _, mut transform, _) = base_input();
        transform.calibration_sha256 = [0; 32];
        assert_eq!(
            transform.map_source_time(10_000_000, 0),
            Err(AlignmentFailure::CalibrationDigestMissing)
        );
    }

    #[test]
    fn zero_or_reversed_event_window_is_rejected() {
        let (mut event, channel, transform, stream) = base_input();
        event.end_master_ns = event.start_master_ns;
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::InvalidEventWindow)
        );
    }

    #[test]
    fn future_event_window_cannot_be_used_before_its_end() {
        let (mut event, channel, transform, stream) = base_input();
        event.decision_cutoff_master_ns = event.end_master_ns - 1;
        assert_eq!(
            align_stream(&event, &channel, &transform, &stream),
            Err(AlignmentFailure::DecisionCutoffBeforeEventEnd)
        );
    }

    #[test]
    fn cadence_jitter_is_included_in_conservative_expected_count() {
        assert_eq!(
            minimum_expected_sample_count(100_000_000, 10_000_000, 1_000_000).unwrap(),
            9
        );
    }
}

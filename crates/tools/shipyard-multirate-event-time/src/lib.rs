//! Deterministic, uncertainty-bounded event-time alignment.
//!
//! This crate aligns synthetic multimodal samples to physical event windows.
//! It preserves source cadence and sample identity; it does not resample,
//! interpolate, infer weld quality, or authorize equipment activity.

use std::collections::HashSet;

const NS_PER_SECOND: i128 = 1_000_000_000;
const COVERAGE_SCALE: u128 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleQuality {
    Valid,
    Clipped,
    Dropped,
    Invalid,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockTransform {
    pub source_clock_id: String,
    pub master_clock_id: String,
    pub reference_source_ns: i64,
    pub reference_master_ns: i64,
    pub drift_ppb: i64,
    pub drift_uncertainty_ppb: u64,
    pub calibration_uncertainty_ns: u64,
    pub valid_from_source_ns: i64,
    pub valid_until_source_ns: i64,
    pub source_sha256: String,
    pub calibration_receipt_sha256: String,
}

impl ClockTransform {
    /// Map one source timestamp using a checked affine transform.
    /// Rounding is nearest integer nanosecond, with half ties away from zero.
    pub fn map_timestamp(
        &self,
        source_ns: i64,
        source_uncertainty_ns: u64,
    ) -> Result<(i64, u64), AbstainReason> {
        if self.source_clock_id.trim().is_empty() || self.master_clock_id.trim().is_empty() {
            return Err(AbstainReason::UnknownClockDomain);
        }
        if source_ns < self.valid_from_source_ns || source_ns >= self.valid_until_source_ns {
            return Err(AbstainReason::TransformExpired);
        }
        if !valid_digest(&self.source_sha256)
            || !valid_digest(&self.calibration_receipt_sha256)
        {
            return Err(AbstainReason::InvalidEvidenceDigest);
        }
        let slope = NS_PER_SECOND
            .checked_add(self.drift_ppb as i128)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        if slope <= 0 {
            return Err(AbstainReason::InvalidClockTransform);
        }

        let delta = (source_ns as i128)
            .checked_sub(self.reference_source_ns as i128)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        let scaled = delta
            .checked_mul(slope)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        let mapped_delta = div_round_nearest_away(scaled, NS_PER_SECOND)?;
        let mapped = (self.reference_master_ns as i128)
            .checked_add(mapped_delta)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        let mapped_ns = i64::try_from(mapped).map_err(|_| AbstainReason::ArithmeticOverflow)?;

        let drift_product = delta
            .unsigned_abs()
            .checked_mul(self.drift_uncertainty_ppb as u128)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        let drift_uncertainty = div_ceil_u128(drift_product, NS_PER_SECOND as u128)?;
        let total_uncertainty = (self.calibration_uncertainty_ns as u128)
            .checked_add(source_uncertainty_ns as u128)
            .and_then(|value| value.checked_add(drift_uncertainty))
            .and_then(|value| value.checked_add(1))
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        let uncertainty_ns =
            u64::try_from(total_uncertainty).map_err(|_| AbstainReason::ArithmeticOverflow)?;
        Ok((mapped_ns, uncertainty_ns))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sample {
    pub sequence: u64,
    pub source_timestamp_ns: Option<i64>,
    pub timestamp_uncertainty_ns: u64,
    pub received_master_ns: Option<i64>,
    pub quality: SampleQuality,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamContract {
    pub stream_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub pass_id: String,
    pub source_clock_id: String,
    pub source_sha256: String,
    pub transform: ClockTransform,
    pub nominal_period_ns: u64,
    pub allowed_jitter_ns: u64,
    pub maximum_gap_ns: u64,
    pub maximum_sample_age_ns: u64,
    pub maximum_lateness_ns: u64,
    pub maximum_mapping_uncertainty_ns: u64,
    pub minimum_coverage_permille: u16,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventWindow {
    pub event_id: String,
    pub event_type: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub pass_id: String,
    pub master_clock_id: String,
    pub start_ns: i64,
    pub end_ns: i64,
    pub start_uncertainty_ns: u64,
    pub end_uncertainty_ns: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    DuplicateStreamIdentity,
    EventIdentityMismatch,
    UnknownClockDomain,
    InvalidClockTransform,
    TransformExpired,
    InvalidEvidenceDigest,
    ArithmeticOverflow,
    InvalidCadence,
    InvalidCoveragePolicy,
    MissingUncertaintyDeclaration,
    InvalidEventWindow,
    DuplicateSequence,
    OutOfOrderSequence,
    DuplicateTimestamp,
    OutOfOrderTimestamp,
    MissingReceiveTime,
    FutureInformationLeakage,
    ReceiveBeforeEventTime,
    FutureSampleTimestamp,
    BoundaryAmbiguity,
    InsufficientTemporalResolution,
    InsufficientSamples,
    InsufficientCoverage,
    MaximumGapExceeded,
    ExcessiveMappingUncertainty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamReceipt {
    pub stream_id: String,
    pub source_clock_id: String,
    pub master_clock_id: String,
    pub source_sha256: String,
    pub calibration_receipt_sha256: String,
    pub raw_timestamp_min_ns: i64,
    pub raw_timestamp_max_ns: i64,
    pub mapped_timestamp_min_ns: i64,
    pub mapped_timestamp_max_ns: i64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub source_sample_count: usize,
    pub included_sample_count: usize,
    pub expected_minimum_sample_count: u64,
    pub coverage_permille: u16,
    pub maximum_gap_ns: u64,
    pub maximum_mapping_uncertainty_ns: u64,
    pub duplicate_sequence_count: usize,
    pub out_of_order_sequence_count: usize,
    pub missing_timestamp_count: usize,
    pub clipped_sample_count: usize,
    pub dropped_sample_count: usize,
    pub invalid_sample_count: usize,
    pub late_sample_count: usize,
    pub stale_sample_count: usize,
    pub out_of_window_count: usize,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventAlignmentReceipt {
    pub event_id: String,
    pub event_type: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub pass_id: String,
    pub master_clock_id: String,
    pub decision_cutoff_ns: i64,
    pub stream_receipts: Vec<StreamReceipt>,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Membership {
    Inside,
    Outside,
    Ambiguous,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn div_round_nearest_away(numerator: i128, denominator: i128) -> Result<i128, AbstainReason> {
    if denominator <= 0 {
        return Err(AbstainReason::InvalidClockTransform);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }
    let twice_remainder = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    if twice_remainder < denominator as u128 {
        return Ok(quotient);
    }
    if numerator < 0 {
        quotient.checked_sub(1).ok_or(AbstainReason::ArithmeticOverflow)
    } else {
        quotient.checked_add(1).ok_or(AbstainReason::ArithmeticOverflow)
    }
}

fn div_ceil_u128(numerator: u128, denominator: u128) -> Result<u128, AbstainReason> {
    if denominator == 0 {
        return Err(AbstainReason::InvalidClockTransform);
    }
    let quotient = numerator / denominator;
    if numerator % denominator == 0 {
        Ok(quotient)
    } else {
        quotient.checked_add(1).ok_or(AbstainReason::ArithmeticOverflow)
    }
}

fn checked_add_signed(left: i64, right: u64) -> Result<i64, AbstainReason> {
    let value = (left as i128)
        .checked_add(right as i128)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    i64::try_from(value).map_err(|_| AbstainReason::ArithmeticOverflow)
}

fn checked_sub_signed(left: i64, right: u64) -> Result<i64, AbstainReason> {
    let value = (left as i128)
        .checked_sub(right as i128)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    i64::try_from(value).map_err(|_| AbstainReason::ArithmeticOverflow)
}

fn classify_membership(
    mapped_ns: i64,
    uncertainty_ns: u64,
    event: &EventWindow,
) -> Result<Membership, AbstainReason> {
    let sample_low = checked_sub_signed(mapped_ns, uncertainty_ns)?;
    let sample_high = checked_add_signed(mapped_ns, uncertainty_ns)?;
    let certain_start = checked_add_signed(event.start_ns, event.start_uncertainty_ns)?;
    let certain_end = checked_sub_signed(event.end_ns, event.end_uncertainty_ns)?;
    let possible_start = checked_sub_signed(event.start_ns, event.start_uncertainty_ns)?;
    let possible_end = checked_add_signed(event.end_ns, event.end_uncertainty_ns)?;
    if sample_low >= certain_start && sample_high < certain_end {
        return Ok(Membership::Inside);
    }
    if sample_high < possible_start || sample_low >= possible_end {
        return Ok(Membership::Outside);
    }
    Ok(Membership::Ambiguous)
}

fn validate_stream_identity(
    event: &EventWindow,
    stream: &StreamContract,
) -> Result<(), AbstainReason> {
    if stream.stream_id.trim().is_empty()
        || stream.workpiece_id.trim().is_empty()
        || stream.joint_revision.trim().is_empty()
        || stream.pass_id.trim().is_empty()
        || event.event_id.trim().is_empty()
        || event.event_type.trim().is_empty()
    {
        return Err(AbstainReason::EmptyIdentity);
    }
    if stream.workpiece_id != event.workpiece_id
        || stream.joint_revision != event.joint_revision
        || stream.pass_id != event.pass_id
    {
        return Err(AbstainReason::EventIdentityMismatch);
    }
    if stream.source_clock_id != stream.transform.source_clock_id
        || stream.transform.master_clock_id != event.master_clock_id
        || event.master_clock_id.trim().is_empty()
    {
        return Err(AbstainReason::UnknownClockDomain);
    }
    if stream.source_sha256 != stream.transform.source_sha256
        || !valid_digest(&stream.source_sha256)
        || !valid_digest(&stream.transform.calibration_receipt_sha256)
    {
        return Err(AbstainReason::InvalidEvidenceDigest);
    }
    if stream.transform.calibration_uncertainty_ns == 0
        || stream.transform.drift_uncertainty_ppb == 0
        || stream.samples.iter().any(|sample| {
            sample.source_timestamp_ns.is_some() && sample.timestamp_uncertainty_ns == 0
        })
    {
        return Err(AbstainReason::MissingUncertaintyDeclaration);
    }
    if stream.nominal_period_ns == 0
        || stream.maximum_gap_ns == 0
        || stream.maximum_sample_age_ns == 0
        || stream.maximum_mapping_uncertainty_ns == 0
    {
        return Err(AbstainReason::InvalidCadence);
    }
    if stream.minimum_coverage_permille == 0 || stream.minimum_coverage_permille > 1_000 {
        return Err(AbstainReason::InvalidCoveragePolicy);
    }
    if event.start_ns >= event.end_ns {
        return Err(AbstainReason::InvalidEventWindow);
    }
    let certain_start = checked_add_signed(event.start_ns, event.start_uncertainty_ns)?;
    let certain_end = checked_sub_signed(event.end_ns, event.end_uncertainty_ns)?;
    if certain_start >= certain_end {
        return Err(AbstainReason::InvalidEventWindow);
    }
    Ok(())
}

pub fn align_stream(
    event: &EventWindow,
    stream: &StreamContract,
    decision_cutoff_ns: i64,
) -> Result<StreamReceipt, AbstainReason> {
    validate_stream_identity(event, stream)?;

    let maximum_spacing = stream
        .nominal_period_ns
        .checked_add(
            stream
                .allowed_jitter_ns
                .checked_mul(2)
                .ok_or(AbstainReason::ArithmeticOverflow)?,
        )
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    if maximum_spacing == 0 {
        return Err(AbstainReason::InvalidCadence);
    }

    let certain_start = checked_add_signed(event.start_ns, event.start_uncertainty_ns)?;
    let certain_end = checked_sub_signed(event.end_ns, event.end_uncertainty_ns)?;
    let effective_duration = (certain_end as i128)
        .checked_sub(certain_start as i128)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    let expected_minimum = effective_duration / (maximum_spacing as i128);
    let expected_minimum =
        u64::try_from(expected_minimum).map_err(|_| AbstainReason::ArithmeticOverflow)?;
    if expected_minimum == 0 {
        return Err(AbstainReason::InsufficientTemporalResolution);
    }

    let mut last_sequence: Option<u64> = None;
    let mut last_source_timestamp: Option<i64> = None;
    let mut raw_min = i64::MAX;
    let mut raw_max = i64::MIN;
    let mut mapped_min = i64::MAX;
    let mut mapped_max = i64::MIN;
    let mut first_sequence = None;
    let mut last_included_sequence = None;
    let mut included_times = Vec::new();
    let mut max_uncertainty = 0u64;
    let mut maximum_gap = 0u64;
    let mut duplicate_sequence_count = 0usize;
    let mut out_of_order_sequence_count = 0usize;
    let mut missing_timestamp_count = 0usize;
    let mut clipped_sample_count = 0usize;
    let mut dropped_sample_count = 0usize;
    let mut invalid_sample_count = 0usize;
    let mut late_sample_count = 0usize;
    let mut stale_sample_count = 0usize;
    let mut out_of_window_count = 0usize;

    for sample in &stream.samples {
        if last_sequence.is_some_and(|last| sample.sequence == last) {
            duplicate_sequence_count += 1;
            return Err(AbstainReason::DuplicateSequence);
        }
        if last_sequence.is_some_and(|last| sample.sequence < last) {
            out_of_order_sequence_count += 1;
            return Err(AbstainReason::OutOfOrderSequence);
        }
        last_sequence = Some(sample.sequence);
        if first_sequence.is_none() {
            first_sequence = Some(sample.sequence);
        }

        let Some(source_timestamp_ns) = sample.source_timestamp_ns else {
            missing_timestamp_count += 1;
            continue;
        };
        if let Some(previous) = last_source_timestamp {
            if source_timestamp_ns == previous {
                return Err(AbstainReason::DuplicateTimestamp);
            }
            if source_timestamp_ns < previous {
                return Err(AbstainReason::OutOfOrderTimestamp);
            }
        }
        last_source_timestamp = Some(source_timestamp_ns);
        raw_min = raw_min.min(source_timestamp_ns);
        raw_max = raw_max.max(source_timestamp_ns);

        let (mapped_ns, uncertainty_ns) = stream
            .transform
            .map_timestamp(source_timestamp_ns, sample.timestamp_uncertainty_ns)?;
        if uncertainty_ns > stream.maximum_mapping_uncertainty_ns {
            return Err(AbstainReason::ExcessiveMappingUncertainty);
        }
        max_uncertainty = max_uncertainty.max(uncertainty_ns);

        let received_ns = sample
            .received_master_ns
            .ok_or(AbstainReason::MissingReceiveTime)?;
        if received_ns > decision_cutoff_ns {
            return Err(AbstainReason::FutureInformationLeakage);
        }
        let sample_age = (decision_cutoff_ns as i128)
            .checked_sub(mapped_ns as i128)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        if sample_age < 0 {
            return Err(AbstainReason::FutureSampleTimestamp);
        }
        let arrival_delay = (received_ns as i128)
            .checked_sub(mapped_ns as i128)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        if arrival_delay < 0 {
            return Err(AbstainReason::ReceiveBeforeEventTime);
        }
        if arrival_delay > stream.maximum_lateness_ns as i128 {
            late_sample_count += 1;
            continue;
        }
        if sample_age > stream.maximum_sample_age_ns as i128 {
            stale_sample_count += 1;
            continue;
        }

        match sample.quality {
            SampleQuality::Valid => {}
            SampleQuality::Clipped => {
                clipped_sample_count += 1;
                continue;
            }
            SampleQuality::Dropped => {
                dropped_sample_count += 1;
                continue;
            }
            SampleQuality::Invalid => {
                invalid_sample_count += 1;
                continue;
            }
            SampleQuality::Missing => {
                missing_timestamp_count += 1;
                continue;
            }
        }

        match classify_membership(mapped_ns, uncertainty_ns, event)? {
            Membership::Inside => {}
            Membership::Outside => {
                out_of_window_count += 1;
                continue;
            }
            Membership::Ambiguous => return Err(AbstainReason::BoundaryAmbiguity),
        }

        mapped_min = mapped_min.min(mapped_ns);
        mapped_max = mapped_max.max(mapped_ns);
        last_included_sequence = Some(sample.sequence);
        included_times.push(mapped_ns);
    }

    if included_times.is_empty() {
        return Err(AbstainReason::InsufficientSamples);
    }
    let included_count = included_times.len();
    let coverage = (included_count as u128)
        .checked_mul(COVERAGE_SCALE)
        .ok_or(AbstainReason::ArithmeticOverflow)?
        / (expected_minimum as u128);
    let coverage_permille = u16::try_from(coverage.min(COVERAGE_SCALE))
        .map_err(|_| AbstainReason::ArithmeticOverflow)?;
    if coverage_permille < stream.minimum_coverage_permille {
        return Err(AbstainReason::InsufficientCoverage);
    }

    let first_time = *included_times.first().ok_or(AbstainReason::InsufficientSamples)?;
    let last_time = *included_times.last().ok_or(AbstainReason::InsufficientSamples)?;
    let leading_gap = (first_time as i128)
        .checked_sub(certain_start as i128)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    let trailing_gap = (certain_end as i128)
        .checked_sub(last_time as i128)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    if leading_gap < 0 || trailing_gap < 0 {
        return Err(AbstainReason::BoundaryAmbiguity);
    }
    maximum_gap = u64::try_from(leading_gap.max(trailing_gap))
        .map_err(|_| AbstainReason::ArithmeticOverflow)?;
    for pair in included_times.windows(2) {
        let gap = (pair[1] as i128)
            .checked_sub(pair[0] as i128)
            .ok_or(AbstainReason::ArithmeticOverflow)?;
        let gap = u64::try_from(gap).map_err(|_| AbstainReason::ArithmeticOverflow)?;
        maximum_gap = maximum_gap.max(gap);
    }
    if maximum_gap > stream.maximum_gap_ns {
        return Err(AbstainReason::MaximumGapExceeded);
    }

    Ok(StreamReceipt {
        stream_id: stream.stream_id.clone(),
        source_clock_id: stream.source_clock_id.clone(),
        master_clock_id: event.master_clock_id.clone(),
        source_sha256: stream.source_sha256.clone(),
        calibration_receipt_sha256: stream.transform.calibration_receipt_sha256.clone(),
        raw_timestamp_min_ns: if raw_min == i64::MAX { 0 } else { raw_min },
        raw_timestamp_max_ns: if raw_max == i64::MIN { 0 } else { raw_max },
        mapped_timestamp_min_ns: mapped_min,
        mapped_timestamp_max_ns: mapped_max,
        first_sequence: first_sequence.unwrap_or(0),
        last_sequence: last_included_sequence.unwrap_or(0),
        source_sample_count: stream.samples.len(),
        included_sample_count: included_count,
        expected_minimum_sample_count: expected_minimum,
        coverage_permille,
        maximum_gap_ns: maximum_gap,
        maximum_mapping_uncertainty_ns: max_uncertainty,
        duplicate_sequence_count,
        out_of_order_sequence_count,
        missing_timestamp_count,
        clipped_sample_count,
        dropped_sample_count,
        invalid_sample_count,
        late_sample_count,
        stale_sample_count,
        out_of_window_count,
        claim_ceiling: "TEMPORAL_ALIGNMENT_ONLY",
    })
}

pub fn align_event(
    event: &EventWindow,
    streams: &[StreamContract],
    decision_cutoff_ns: i64,
) -> Result<EventAlignmentReceipt, AbstainReason> {
    if streams.is_empty() {
        return Err(AbstainReason::InsufficientSamples);
    }
    let mut identities = HashSet::new();
    let mut receipts = Vec::with_capacity(streams.len());
    for stream in streams {
        if !identities.insert(stream.stream_id.as_str()) {
            return Err(AbstainReason::DuplicateStreamIdentity);
        }
        receipts.push(align_stream(event, stream, decision_cutoff_ns)?);
    }
    receipts.sort_by(|left, right| left.stream_id.cmp(&right.stream_id));
    Ok(EventAlignmentReceipt {
        event_id: event.event_id.clone(),
        event_type: event.event_type.clone(),
        workpiece_id: event.workpiece_id.clone(),
        joint_revision: event.joint_revision.clone(),
        pass_id: event.pass_id.clone(),
        master_clock_id: event.master_clock_id.clone(),
        decision_cutoff_ns,
        stream_receipts: receipts,
        claim_ceiling: "MULTIMODAL_EVENT_TIME_ALIGNMENT_ONLY",
    })
}

/// A lead/lag claim is resolvable only when it exceeds combined uncertainty.
pub fn causal_lag_resolvable(
    absolute_lag_ns: u64,
    first_uncertainty_ns: u64,
    second_uncertainty_ns: u64,
) -> Result<bool, AbstainReason> {
    let combined = first_uncertainty_ns
        .checked_add(second_uncertainty_ns)
        .ok_or(AbstainReason::ArithmeticOverflow)?;
    Ok(absolute_lag_ns > combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT_START: i64 = 0;
    const EVENT_END: i64 = 100_000_000;
    const DECISION: i64 = 105_000_000;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn event() -> EventWindow {
        EventWindow {
            event_id: "EV-PASS-02".to_owned(),
            event_type: "WELD_PASS".to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            pass_id: "PASS-02".to_owned(),
            master_clock_id: "UTC-MASTER".to_owned(),
            start_ns: EVENT_START,
            end_ns: EVENT_END,
            start_uncertainty_ns: 1_000,
            end_uncertainty_ns: 1_000,
        }
    }

    fn stream(id: &str, period_ns: u64, samples: usize) -> StreamContract {
        let source_hash = digest('a');
        let transform = ClockTransform {
            source_clock_id: format!("CLK-{id}"),
            master_clock_id: "UTC-MASTER".to_owned(),
            reference_source_ns: 0,
            reference_master_ns: 0,
            drift_ppb: 0,
            drift_uncertainty_ppb: 1,
            calibration_uncertainty_ns: 50,
            valid_from_source_ns: -1_000_000_000,
            valid_until_source_ns: 1_000_000_000,
            source_sha256: source_hash.clone(),
            calibration_receipt_sha256: digest('b'),
        };
        let samples = (0..samples)
            .map(|index| {
                let time = 2_000_000 + (index as i64) * (period_ns as i64);
                Sample {
                    sequence: index as u64 + 1,
                    source_timestamp_ns: Some(time),
                    timestamp_uncertainty_ns: 50,
                    received_master_ns: Some(time + 1_000_000),
                    quality: SampleQuality::Valid,
                }
            })
            .collect();
        StreamContract {
            stream_id: id.to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            pass_id: "PASS-02".to_owned(),
            source_clock_id: format!("CLK-{id}"),
            source_sha256: source_hash,
            transform,
            nominal_period_ns: period_ns,
            allowed_jitter_ns: 100_000,
            maximum_gap_ns: 12_000_000,
            maximum_sample_age_ns: 200_000_000,
            maximum_lateness_ns: 2_000_000,
            maximum_mapping_uncertainty_ns: 10_000,
            minimum_coverage_permille: 900,
            samples,
        }
    }

    #[test]
    fn identity_clock_aligns_without_resampling() {
        let input = stream("PROCESS", 10_000_000, 10);
        let receipt = align_stream(&event(), &input, DECISION).expect("alignment");
        assert_eq!(receipt.included_sample_count, 10);
        assert_eq!(receipt.coverage_permille, 1_000);
        assert_eq!(receipt.maximum_mapping_uncertainty_ns, 102);
        assert_eq!(receipt.claim_ceiling, "TEMPORAL_ALIGNMENT_ONLY");
    }

    #[test]
    fn multimodal_streams_keep_their_own_sample_cadence() {
        let mut window = event();
        window.end_ns = 500_000_000;
        let decision = 1_000_000_000;
        let make_rate = |id: &str, period: u64, maximum_gap: u64| {
            let sample_count = ((500_000_000u64 - 2_000_000 - 1) / period + 1) as usize;
            let mut value = stream(id, period, sample_count);
            value.maximum_gap_ns = maximum_gap;
            value.maximum_sample_age_ns = 1_000_000_000;
            value
        };
        let process = make_rate("PROCESS-01", 100_000, 3_000_000);
        let robot = make_rate("ROBOT-01", 8_000_000, 9_000_000);
        let opcua = make_rate("OPCUA-01", 100_000_000, 110_000_000);
        let receipt = align_event(&window, &[opcua, robot, process], decision)
            .expect("multirate event alignment");
        assert_eq!(receipt.stream_receipts.len(), 3);
        assert_eq!(receipt.stream_receipts[0].stream_id, "OPCUA-01");
        assert_eq!(receipt.stream_receipts[0].included_sample_count, 5);
        assert_eq!(receipt.stream_receipts[1].included_sample_count, 13);
        assert_eq!(receipt.stream_receipts[2].included_sample_count, 1_000);
    }

    #[test]
    fn positive_clock_offset_is_applied_exactly() {
        let mut t = stream("OFFSET", 10_000_000, 10).transform;
        t.reference_master_ns = 5_000_000;
        let (mapped, _) = t.map_timestamp(10_000_000, 50).expect("map");
        assert_eq!(mapped, 15_000_000);
    }

    #[test]
    fn negative_clock_offset_is_applied_exactly() {
        let mut t = stream("OFFSET", 10_000_000, 10).transform;
        t.reference_master_ns = -3_000_000;
        let (mapped, _) = t.map_timestamp(10_000_000, 50).expect("map");
        assert_eq!(mapped, 7_000_000);
    }

    #[test]
    fn positive_drift_is_applied_in_parts_per_billion() {
        let mut t = stream("DRIFT", 10_000_000, 10).transform;
        t.drift_ppb = 1_000_000;
        t.valid_until_source_ns = 20_000_000_000;
        let (mapped, _) = t.map_timestamp(1_000_000_000, 50).expect("map");
        assert_eq!(mapped, 1_001_000_000);
    }

    #[test]
    fn negative_drift_is_applied_in_parts_per_billion() {
        let mut t = stream("DRIFT", 10_000_000, 10).transform;
        t.drift_ppb = -1_000_000;
        t.valid_until_source_ns = 20_000_000_000;
        let (mapped, _) = t.map_timestamp(1_000_000_000, 50).expect("map");
        assert_eq!(mapped, 999_000_000);
    }

    #[test]
    fn drift_uncertainty_grows_with_distance_from_reference_epoch() {
        let mut t = stream("DRIFT", 10_000_000, 10).transform;
        t.drift_uncertainty_ppb = 10;
        t.valid_until_source_ns = 20_000_000_000;
        let (_, near) = t.map_timestamp(1_000_000_000, 50).expect("near");
        let (_, far) = t.map_timestamp(10_000_000_000, 50).expect("far");
        assert!(far > near);
    }

    #[test]
    fn transform_expiry_is_fail_closed() {
        let mut t = stream("EXPIRY", 10_000_000, 10).transform;
        t.valid_until_source_ns = 5_000_000;
        assert_eq!(
            t.map_timestamp(5_000_000, 50),
            Err(AbstainReason::TransformExpired)
        );
    }

    #[test]
    fn unknown_clock_domain_is_rejected() {
        let mut input = stream("UNKNOWN", 10_000_000, 10);
        input.transform.source_clock_id = "OTHER-CLOCK".to_owned();
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::UnknownClockDomain)
        );
    }

    #[test]
    fn invalid_nonpositive_clock_slope_is_rejected() {
        let mut t = stream("SLOPE", 10_000_000, 10).transform;
        t.drift_ppb = -1_000_000_000;
        assert_eq!(
            t.map_timestamp(10_000_000, 50),
            Err(AbstainReason::InvalidClockTransform)
        );
    }

    #[test]
    fn clock_mapping_overflow_is_rejected() {
        let mut t = stream("OVERFLOW", 10_000_000, 10).transform;
        t.reference_master_ns = i64::MAX;
        assert_eq!(
            t.map_timestamp(10, 50),
            Err(AbstainReason::ArithmeticOverflow)
        );
    }

    #[test]
    fn source_and_transform_digest_must_match() {
        let mut input = stream("DIGEST", 10_000_000, 10);
        input.transform.source_sha256 = digest('c');
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::InvalidEvidenceDigest)
        );
    }

    #[test]
    fn malformed_transform_digest_is_rejected() {
        let mut input = stream("DIGEST", 10_000_000, 10);
        input.transform.calibration_receipt_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::InvalidEvidenceDigest)
        );
    }

    #[test]
    fn wrong_workpiece_cannot_join_event() {
        let mut input = stream("WID", 10_000_000, 10);
        input.workpiece_id = "WID-OTHER".to_owned();
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::EventIdentityMismatch)
        );
    }

    #[test]
    fn future_received_sample_is_not_available_to_decision() {
        let mut input = stream("FUTURE", 10_000_000, 10);
        input.samples[0].received_master_ns = Some(DECISION + 1);
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::FutureInformationLeakage)
        );
    }

    #[test]
    fn missing_receive_time_is_not_assumed_available() {
        let mut input = stream("ARRIVAL", 10_000_000, 10);
        input.samples[0].received_master_ns = None;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::MissingReceiveTime)
        );
    }

    #[test]
    fn receive_before_event_timestamp_is_rejected() {
        let mut input = stream("ARRIVAL", 10_000_000, 10);
        input.samples[0].received_master_ns = Some(0);
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::ReceiveBeforeEventTime)
        );
    }

    #[test]
    fn late_samples_are_counted_not_silently_retimed() {
        let mut input = stream("LATE", 10_000_000, 10);
        input.samples[0].received_master_ns = Some(10_000_000);
        let receipt = align_stream(&event(), &input, DECISION).expect("remaining coverage");
        assert_eq!(receipt.late_sample_count, 1);
        assert_eq!(receipt.included_sample_count, 9);
    }

    #[test]
    fn stale_slow_state_is_excluded() {
        let mut input = stream("STALE", 10_000_000, 10);
        input.maximum_sample_age_ns = 100_000_000;
        let receipt = align_stream(&event(), &input, DECISION).expect("remaining coverage");
        assert!(receipt.stale_sample_count > 0);
        assert!(receipt.included_sample_count < 10);
    }

    #[test]
    fn missing_timestamp_is_counted_not_replaced_with_epoch_zero() {
        let mut input = stream("MISSING", 10_000_000, 10);
        input.samples[0].source_timestamp_ns = None;
        let receipt = align_stream(&event(), &input, DECISION).expect("remaining coverage");
        assert_eq!(receipt.missing_timestamp_count, 1);
        assert_eq!(receipt.raw_timestamp_min_ns, 12_000_000);
    }

    #[test]
    fn clipped_dropped_and_invalid_samples_do_not_count_as_valid() {
        let mut input = stream("QUALITY", 10_000_000, 10);
        input.samples[0].quality = SampleQuality::Clipped;
        input.samples[1].quality = SampleQuality::Dropped;
        input.samples[2].quality = SampleQuality::Invalid;
        input.minimum_coverage_permille = 700;
        input.maximum_gap_ns = 50_000_000;
        let receipt = align_stream(&event(), &input, DECISION).expect("remaining coverage");
        assert_eq!(receipt.clipped_sample_count, 1);
        assert_eq!(receipt.dropped_sample_count, 1);
        assert_eq!(receipt.invalid_sample_count, 1);
        assert_eq!(receipt.included_sample_count, 7);
    }

    #[test]
    fn duplicate_sequence_number_abstains() {
        let mut input = stream("DUP-SEQ", 10_000_000, 10);
        input.samples[1].sequence = input.samples[0].sequence;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::DuplicateSequence)
        );
    }

    #[test]
    fn out_of_order_sequence_abstains() {
        let mut input = stream("ORDER-SEQ", 10_000_000, 10);
        input.samples[1].sequence = 0;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::OutOfOrderSequence)
        );
    }

    #[test]
    fn duplicate_timestamp_abstains() {
        let mut input = stream("DUP-TIME", 10_000_000, 10);
        input.samples[1].source_timestamp_ns = input.samples[0].source_timestamp_ns;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::DuplicateTimestamp)
        );
    }

    #[test]
    fn out_of_order_timestamp_abstains() {
        let mut input = stream("ORDER-TIME", 10_000_000, 10);
        input.samples[1].source_timestamp_ns = Some(1_000_000);
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::OutOfOrderTimestamp)
        );
    }

    #[test]
    fn event_end_is_half_open_and_not_double_counted() {
        let mut input = stream("END", 10_000_000, 10);
        input.samples[9].source_timestamp_ns = Some(EVENT_END + 3_000);
        input.samples[9].received_master_ns = Some(EVENT_END + 1_003_000);
        input.maximum_gap_ns = 20_000_000;
        let receipt = align_stream(&event(), &input, DECISION).expect("window still covered");
        assert_eq!(receipt.out_of_window_count, 1);
        assert_eq!(receipt.included_sample_count, 9);
    }

    #[test]
    fn boundary_uncertainty_causes_abstention() {
        let mut input = stream("AMBIGUOUS", 10_000_000, 10);
        input.samples[0].source_timestamp_ns = Some(500);
        input.samples[0].received_master_ns = Some(1_000_500);
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::BoundaryAmbiguity)
        );
    }

    #[test]
    fn large_internal_gap_abstains() {
        let mut input = stream("GAP", 10_000_000, 10);
        input.samples.remove(4);
        input.maximum_gap_ns = 10_500_000;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::MaximumGapExceeded)
        );
    }

    #[test]
    fn inadequate_coverage_abstains() {
        let input = stream("COVERAGE", 10_000_000, 5);
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::InsufficientCoverage)
        );
    }

    #[test]
    fn maximum_mapping_uncertainty_is_enforced() {
        let mut input = stream("UNCERTAINTY", 10_000_000, 10);
        input.maximum_mapping_uncertainty_ns = 50;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::ExcessiveMappingUncertainty)
        );
    }

    #[test]
    fn different_event_master_clock_is_rejected() {
        let mut window = event();
        window.master_clock_id = "OTHER-MASTER".to_owned();
        assert_eq!(
            align_stream(&window, &stream("MASTER", 10_000_000, 10), DECISION),
            Err(AbstainReason::UnknownClockDomain)
        );
    }

    #[test]
    fn duplicate_stream_ids_cannot_be_composed() {
        let one = stream("SAME", 10_000_000, 10);
        let two = stream("SAME", 10_000_000, 10);
        assert_eq!(
            align_event(&event(), &[one, two], DECISION),
            Err(AbstainReason::DuplicateStreamIdentity)
        );
    }

    #[test]
    fn causal_lag_below_combined_uncertainty_is_not_resolvable() {
        assert!(!causal_lag_resolvable(100, 60, 50).expect("comparison"));
        assert!(!causal_lag_resolvable(110, 60, 50).expect("comparison"));
    }

    #[test]
    fn causal_lag_above_combined_uncertainty_is_resolvable() {
        assert!(causal_lag_resolvable(111, 60, 50).expect("comparison"));
    }

    #[test]
    fn zero_or_undeclared_clock_uncertainty_is_rejected() {
        let mut input = stream("ZERO-UNCERTAINTY", 10_000_000, 10);
        input.transform.calibration_uncertainty_ns = 0;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::MissingUncertaintyDeclaration)
        );
    }

    #[test]
    fn zero_sample_timestamp_uncertainty_is_rejected() {
        let mut input = stream("ZERO-SAMPLE-UNCERTAINTY", 10_000_000, 10);
        input.samples[0].timestamp_uncertainty_ns = 0;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::MissingUncertaintyDeclaration)
        );
    }

    #[test]
    fn common_clock_does_not_imply_zero_uncertainty() {
        let mut input = stream("COMMON-CLOCK", 10_000_000, 10);
        input.transform.calibration_uncertainty_ns = 5_000;
        input.maximum_mapping_uncertainty_ns = 4_000;
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::ExcessiveMappingUncertainty)
        );
    }

    #[test]
    fn future_mapped_sample_abstains_even_if_received_before_cutoff() {
        let mut input = stream("FUTURE-MAP", 10_000_000, 10);
        input.samples[0].source_timestamp_ns = Some(110_000_000);
        input.samples[0].received_master_ns = Some(DECISION);
        assert_eq!(
            align_stream(&event(), &input, DECISION),
            Err(AbstainReason::FutureSampleTimestamp)
        );
    }

    #[test]
    fn causal_uncertainty_sum_overflow_abstains() {
        assert_eq!(
            causal_lag_resolvable(10, u64::MAX, 1),
            Err(AbstainReason::ArithmeticOverflow)
        );
    }
}

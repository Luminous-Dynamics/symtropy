//! Multi-body reference CCD across independently qualified robot clock domains.
//!
//! Each source span is mapped to one site-monotonic clock by a hard offset
//! interval and a hard relative-drift bound. The worst offset and drift error
//! over the complete span becomes a symmetric timing bound passed to the
//! existing conservative two-body clock-uncertainty oracle.
//!
//! This is an exhaustive O(n^2) reference evaluator, not a scalable broad phase.
//! It does not estimate or synchronize clocks. Missing, stale, or mismatched
//! mappings create explicit UNRESOLVED pair receipts rather than guessed times.

use std::collections::BTreeSet;

use crate::{
    CapsuleMotionSpan, ClockUncertaintyCcdResult, ClockUncertaintyDisposition,
    ClockUncertaintyError, clock_uncertainty_aware_capsule_query,
};

const DRIFT_PPM_DENOMINATOR: u128 = 1_000_000;
pub const MAX_RELATIVE_TIME_BODIES: usize = 1_024;
pub const MAX_RELATIVE_TIME_PAIR_CHECKS: u64 = 524_288;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedClockMapping {
    pub source_clock_domain_id: String,
    pub site_clock_domain_id: String,
    pub site_epoch: String,
    pub mapping_id: String,
    pub evidence_ref: String,
    pub reference_source_time_ms: u64,
    /// Inclusive bounds for (site time - source time) at the reference instant.
    pub offset_min_ms: i64,
    pub offset_max_ms: i64,
    /// Maximum absolute relative-rate error in parts per million.
    /// Missing drift qualification must abstain.
    pub drift_bound_ppm: Option<u64>,
    /// Half-open mapping validity interval in the source clock domain.
    pub valid_from_source_ms: u64,
    pub valid_until_source_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockDomainCapsuleBody {
    pub body_id: String,
    /// Motion timestamps are expressed in motion.clock_domain_id.
    pub motion: CapsuleMotionSpan,
    pub clock_mapping: Option<QualifiedClockMapping>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClockMappingFailure {
    MissingMapping,
    EmptySourceClockDomain,
    EmptySiteClockDomain,
    EmptySiteEpoch,
    EmptyMappingId,
    EmptyEvidenceReference,
    SourceClockDomainMismatch,
    SiteEpochMismatch,
    SiteClockDomainMismatch,
    FrameMismatch,
    InvalidValidityInterval,
    ReferenceOutsideValidity,
    SpanOutsideValidity,
    InvalidOffsetInterval,
    MissingDriftBound,
    DriftBoundOverflow,
    ClockUncertaintyOverflow,
    TimestampMappingOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClockMappingDisposition {
    Mapped,
    Unresolved(ClockMappingFailure),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyClockMappingReceipt {
    pub body_id: String,
    pub source_clock_domain_id: String,
    pub mapping_id: Option<String>,
    pub nominal_site_start_ms: Option<u64>,
    pub nominal_site_end_ms: Option<u64>,
    pub nominal_offset_ms: Option<i64>,
    /// Hard site-clock uncertainty after offset, drift and source-span bounds.
    pub total_uncertainty_ms: Option<u64>,
    pub drift_margin_ms: Option<u64>,
    pub disposition: ClockMappingDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeTimePairDisposition {
    ExactContact,
    ExactNoContact,
    BoundaryCandidateUnresolved,
    NoPossibleTemporalOverlap,
    NoPossibleContact,
    PotentialContactUnresolved,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelativeTimePairUnresolvedReason {
    FirstBodyMapping(ClockMappingFailure),
    SecondBodyMapping(ClockMappingFailure),
    ClockQueryRejected(ClockUncertaintyError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelativeTimePairReceipt {
    pub first_body_id: String,
    pub second_body_id: String,
    pub first_mapping_id: Option<String>,
    pub second_mapping_id: Option<String>,
    pub first_total_clock_uncertainty_ms: Option<u64>,
    pub second_total_clock_uncertainty_ms: Option<u64>,
    pub disposition: RelativeTimePairDisposition,
    pub unresolved_reason: Option<RelativeTimePairUnresolvedReason>,
    pub query_result: Option<ClockUncertaintyCcdResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativeTimeMultiBodyConfig {
    pub site_epoch: String,
    pub frame_id: String,
    pub site_clock_domain_id: String,
    pub max_pair_checks: u64,
}

impl Default for RelativeTimeMultiBodyConfig {
    fn default() -> Self {
        Self {
            site_epoch: String::new(),
            frame_id: String::new(),
            site_clock_domain_id: String::new(),
            max_pair_checks: MAX_RELATIVE_TIME_PAIR_CHECKS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeTimeMultiBodyError {
    NoBodies,
    TooManyBodies,
    EmptyBodyId,
    DuplicateBodyId,
    EmptyTargetIdentity,
    PairCountOverflow,
    PairBudgetExceeded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelativeTimeMultiBodyReport {
    pub site_epoch: String,
    pub frame_id: String,
    pub site_clock_domain_id: String,
    pub body_count: usize,
    pub pair_checks: u64,
    pub exact_contact_pairs: u64,
    pub exact_no_contact_pairs: u64,
    pub boundary_candidate_pairs: u64,
    pub no_possible_temporal_overlap_pairs: u64,
    pub no_possible_contact_pairs: u64,
    pub potential_contact_unresolved_pairs: u64,
    pub unresolved_pairs: u64,
    pub body_mappings: Vec<BodyClockMappingReceipt>,
    pub pairs: Vec<RelativeTimePairReceipt>,
}

/// Exhaustively evaluate every unordered body pair after independently mapping
/// source-clock spans into one requested site clock.
///
/// Mapping equation at the reference instant is site_time = source_time + offset.
/// The actual offset may vary within the interval by at most the declared drift
/// bound. The maximum endpoint drift, half the offset interval width, and the
/// source-span uncertainty are combined into a conservative symmetric timing
/// error bound. The pairwise owner then returns exact, proven-clear, possible,
/// or unresolved outcomes without asserting a physically realized collision.
pub fn evaluate_relative_time_multibody_ccd(
    bodies: &[ClockDomainCapsuleBody],
    config: &RelativeTimeMultiBodyConfig,
) -> Result<RelativeTimeMultiBodyReport, RelativeTimeMultiBodyError> {
    if bodies.is_empty() {
        return Err(RelativeTimeMultiBodyError::NoBodies);
    }
    if bodies.len() > MAX_RELATIVE_TIME_BODIES {
        return Err(RelativeTimeMultiBodyError::TooManyBodies);
    }
    for value in [
        config.site_epoch.as_str(),
        config.frame_id.as_str(),
        config.site_clock_domain_id.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RelativeTimeMultiBodyError::EmptyTargetIdentity);
        }
    }

    let count = bodies.len() as u64;
    let pair_checks = count
        .checked_mul(count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(RelativeTimeMultiBodyError::PairCountOverflow)?;
    if config.max_pair_checks > MAX_RELATIVE_TIME_PAIR_CHECKS
        || pair_checks > config.max_pair_checks
    {
        return Err(RelativeTimeMultiBodyError::PairBudgetExceeded);
    }

    let mut ids = BTreeSet::new();
    for body in bodies {
        if body.body_id.trim().is_empty() {
            return Err(RelativeTimeMultiBodyError::EmptyBodyId);
        }
        if !ids.insert(body.body_id.as_str()) {
            return Err(RelativeTimeMultiBodyError::DuplicateBodyId);
        }
    }

    let mut ordered: Vec<&ClockDomainCapsuleBody> = bodies.iter().collect();
    ordered.sort_by(|left, right| left.body_id.cmp(&right.body_id));
    let mapped: Vec<MappedBody> = ordered
        .iter()
        .map(|body| map_body_clock(body, config))
        .collect();

    let mut report = RelativeTimeMultiBodyReport {
        site_epoch: config.site_epoch.clone(),
        frame_id: config.frame_id.clone(),
        site_clock_domain_id: config.site_clock_domain_id.clone(),
        body_count: bodies.len(),
        pair_checks,
        exact_contact_pairs: 0,
        exact_no_contact_pairs: 0,
        boundary_candidate_pairs: 0,
        no_possible_temporal_overlap_pairs: 0,
        no_possible_contact_pairs: 0,
        potential_contact_unresolved_pairs: 0,
        unresolved_pairs: 0,
        body_mappings: mapped.iter().map(|body| body.receipt.clone()).collect(),
        pairs: Vec::with_capacity(pair_checks as usize),
    };

    for first_index in 0..mapped.len() {
        for second_index in (first_index + 1)..mapped.len() {
            let receipt = evaluate_pair(&mapped[first_index], &mapped[second_index]);
            match receipt.disposition {
                RelativeTimePairDisposition::ExactContact => report.exact_contact_pairs += 1,
                RelativeTimePairDisposition::ExactNoContact => report.exact_no_contact_pairs += 1,
                RelativeTimePairDisposition::BoundaryCandidateUnresolved => {
                    report.boundary_candidate_pairs += 1;
                }
                RelativeTimePairDisposition::NoPossibleTemporalOverlap => {
                    report.no_possible_temporal_overlap_pairs += 1;
                }
                RelativeTimePairDisposition::NoPossibleContact => {
                    report.no_possible_contact_pairs += 1;
                }
                RelativeTimePairDisposition::PotentialContactUnresolved => {
                    report.potential_contact_unresolved_pairs += 1;
                }
                RelativeTimePairDisposition::Unresolved => report.unresolved_pairs += 1,
            }
            report.pairs.push(receipt);
        }
    }
    Ok(report)
}

struct MappedBody {
    body_id: String,
    receipt: BodyClockMappingReceipt,
    span: Option<CapsuleMotionSpan>,
    failure: Option<ClockMappingFailure>,
}

fn map_body_clock(
    body: &ClockDomainCapsuleBody,
    config: &RelativeTimeMultiBodyConfig,
) -> MappedBody {
    let source_clock_domain_id = body.motion.clock_domain_id.clone();
    let Some(mapping) = body.clock_mapping.as_ref() else {
        return unresolved_body(body, None, ClockMappingFailure::MissingMapping);
    };
    let fail = |reason| unresolved_body(body, Some(mapping), reason);

    if source_clock_domain_id.trim().is_empty() {
        return fail(ClockMappingFailure::EmptySourceClockDomain);
    }
    if mapping.site_clock_domain_id.trim().is_empty() {
        return fail(ClockMappingFailure::EmptySiteClockDomain);
    }
    if mapping.site_epoch.trim().is_empty() {
        return fail(ClockMappingFailure::EmptySiteEpoch);
    }
    if mapping.mapping_id.trim().is_empty() {
        return fail(ClockMappingFailure::EmptyMappingId);
    }
    if mapping.evidence_ref.trim().is_empty() {
        return fail(ClockMappingFailure::EmptyEvidenceReference);
    }
    if mapping.source_clock_domain_id != source_clock_domain_id {
        return fail(ClockMappingFailure::SourceClockDomainMismatch);
    }
    if mapping.site_epoch != body.motion.site_epoch || mapping.site_epoch != config.site_epoch {
        return fail(ClockMappingFailure::SiteEpochMismatch);
    }
    if mapping.site_clock_domain_id != config.site_clock_domain_id {
        return fail(ClockMappingFailure::SiteClockDomainMismatch);
    }
    if body.motion.frame_id != config.frame_id {
        return fail(ClockMappingFailure::FrameMismatch);
    }
    if mapping.valid_from_source_ms >= mapping.valid_until_source_ms {
        return fail(ClockMappingFailure::InvalidValidityInterval);
    }
    if mapping.reference_source_time_ms < mapping.valid_from_source_ms
        || mapping.reference_source_time_ms >= mapping.valid_until_source_ms
    {
        return fail(ClockMappingFailure::ReferenceOutsideValidity);
    }
    if body.motion.start_ms < mapping.valid_from_source_ms
        || body.motion.end_ms > mapping.valid_until_source_ms
    {
        return fail(ClockMappingFailure::SpanOutsideValidity);
    }
    if mapping.offset_min_ms > mapping.offset_max_ms {
        return fail(ClockMappingFailure::InvalidOffsetInterval);
    }
    let Some(drift_bound_ppm) = mapping.drift_bound_ppm else {
        return fail(ClockMappingFailure::MissingDriftBound);
    };

    let offset_min = i128::from(mapping.offset_min_ms);
    let offset_max = i128::from(mapping.offset_max_ms);
    let midpoint = (offset_min + offset_max) / 2;
    let width = (offset_max - offset_min) as u128;
    let offset_half_width = width / 2 + width % 2;

    let start_delta = u128::from(
        body.motion
            .start_ms
            .abs_diff(mapping.reference_source_time_ms),
    );
    let end_delta = u128::from(
        body.motion
            .end_ms
            .abs_diff(mapping.reference_source_time_ms),
    );
    let drift_delta = start_delta.max(end_delta);
    let Some(drift_product) = drift_delta.checked_mul(u128::from(drift_bound_ppm)) else {
        return fail(ClockMappingFailure::DriftBoundOverflow);
    };
    let drift_quotient = drift_product / DRIFT_PPM_DENOMINATOR;
    let drift_remainder = drift_product % DRIFT_PPM_DENOMINATOR;
    let drift_margin = drift_quotient + u128::from(drift_remainder != 0);

    let Some(total_uncertainty) = offset_half_width
        .checked_add(drift_margin)
        .and_then(|value| value.checked_add(u128::from(body.motion.clock_uncertainty_ms)))
    else {
        return fail(ClockMappingFailure::ClockUncertaintyOverflow);
    };
    let Ok(total_uncertainty_ms) = u64::try_from(total_uncertainty) else {
        return fail(ClockMappingFailure::ClockUncertaintyOverflow);
    };
    let Ok(drift_margin_ms) = u64::try_from(drift_margin) else {
        return fail(ClockMappingFailure::DriftBoundOverflow);
    };
    let Ok(nominal_offset_ms) = i64::try_from(midpoint) else {
        return fail(ClockMappingFailure::TimestampMappingOverflow);
    };
    let Some(start) = shift_timestamp(body.motion.start_ms, midpoint) else {
        return fail(ClockMappingFailure::TimestampMappingOverflow);
    };
    let Some(end) = shift_timestamp(body.motion.end_ms, midpoint) else {
        return fail(ClockMappingFailure::TimestampMappingOverflow);
    };
    if start >= end {
        return fail(ClockMappingFailure::TimestampMappingOverflow);
    }

    let mut span = body.motion.clone();
    span.start_ms = start;
    span.end_ms = end;
    span.clock_domain_id = config.site_clock_domain_id.clone();
    span.clock_uncertainty_ms = total_uncertainty_ms;

    MappedBody {
        body_id: body.body_id.clone(),
        receipt: BodyClockMappingReceipt {
            body_id: body.body_id.clone(),
            source_clock_domain_id,
            mapping_id: Some(mapping.mapping_id.clone()),
            nominal_site_start_ms: Some(start),
            nominal_site_end_ms: Some(end),
            nominal_offset_ms: Some(nominal_offset_ms),
            total_uncertainty_ms: Some(total_uncertainty_ms),
            drift_margin_ms: Some(drift_margin_ms),
            disposition: ClockMappingDisposition::Mapped,
        },
        span: Some(span),
        failure: None,
    }
}

fn shift_timestamp(timestamp_ms: u64, offset_ms: i128) -> Option<u64> {
    let shifted = i128::from(timestamp_ms).checked_add(offset_ms)?;
    u64::try_from(shifted).ok()
}

fn unresolved_body(
    body: &ClockDomainCapsuleBody,
    mapping: Option<&QualifiedClockMapping>,
    failure: ClockMappingFailure,
) -> MappedBody {
    MappedBody {
        body_id: body.body_id.clone(),
        receipt: BodyClockMappingReceipt {
            body_id: body.body_id.clone(),
            source_clock_domain_id: body.motion.clock_domain_id.clone(),
            mapping_id: mapping.map(|value| value.mapping_id.clone()),
            nominal_site_start_ms: None,
            nominal_site_end_ms: None,
            nominal_offset_ms: None,
            total_uncertainty_ms: None,
            drift_margin_ms: None,
            disposition: ClockMappingDisposition::Unresolved(failure.clone()),
        },
        span: None,
        failure: Some(failure),
    }
}

fn evaluate_pair(first: &MappedBody, second: &MappedBody) -> RelativeTimePairReceipt {
    let mapping_ids = (
        first.receipt.mapping_id.clone(),
        second.receipt.mapping_id.clone(),
    );
    let uncertainty = (
        first.receipt.total_uncertainty_ms,
        second.receipt.total_uncertainty_ms,
    );

    if let Some(reason) = first.failure.as_ref() {
        return unresolved_pair(
            first,
            second,
            mapping_ids,
            uncertainty,
            RelativeTimePairUnresolvedReason::FirstBodyMapping(reason.clone()),
        );
    }
    if let Some(reason) = second.failure.as_ref() {
        return unresolved_pair(
            first,
            second,
            mapping_ids,
            uncertainty,
            RelativeTimePairUnresolvedReason::SecondBodyMapping(reason.clone()),
        );
    }
    let (Some(first_span), Some(second_span)) = (first.span.as_ref(), second.span.as_ref()) else {
        return unresolved_pair(
            first,
            second,
            mapping_ids,
            uncertainty,
            RelativeTimePairUnresolvedReason::ClockQueryRejected(
                ClockUncertaintyError::InvalidTimeInterval,
            ),
        );
    };

    match clock_uncertainty_aware_capsule_query(first_span, second_span) {
        Ok(query_result) => {
            let disposition = match query_result.disposition {
                ClockUncertaintyDisposition::ExactContact => {
                    RelativeTimePairDisposition::ExactContact
                }
                ClockUncertaintyDisposition::ExactNoContact => {
                    RelativeTimePairDisposition::ExactNoContact
                }
                ClockUncertaintyDisposition::BoundaryCandidateUnresolved => {
                    RelativeTimePairDisposition::BoundaryCandidateUnresolved
                }
                ClockUncertaintyDisposition::NoPossibleTemporalOverlap => {
                    RelativeTimePairDisposition::NoPossibleTemporalOverlap
                }
                ClockUncertaintyDisposition::NoPossibleContact => {
                    RelativeTimePairDisposition::NoPossibleContact
                }
                ClockUncertaintyDisposition::PotentialContactUnresolved => {
                    RelativeTimePairDisposition::PotentialContactUnresolved
                }
            };
            RelativeTimePairReceipt {
                first_body_id: first.body_id.clone(),
                second_body_id: second.body_id.clone(),
                first_mapping_id: mapping_ids.0,
                second_mapping_id: mapping_ids.1,
                first_total_clock_uncertainty_ms: uncertainty.0,
                second_total_clock_uncertainty_ms: uncertainty.1,
                disposition,
                unresolved_reason: None,
                query_result: Some(query_result),
            }
        }
        Err(error) => unresolved_pair(
            first,
            second,
            mapping_ids,
            uncertainty,
            RelativeTimePairUnresolvedReason::ClockQueryRejected(error),
        ),
    }
}

fn unresolved_pair(
    first: &MappedBody,
    second: &MappedBody,
    mapping_ids: (Option<String>, Option<String>),
    uncertainty: (Option<u64>, Option<u64>),
    reason: RelativeTimePairUnresolvedReason,
) -> RelativeTimePairReceipt {
    RelativeTimePairReceipt {
        first_body_id: first.body_id.clone(),
        second_body_id: second.body_id.clone(),
        first_mapping_id: mapping_ids.0,
        second_mapping_id: mapping_ids.1,
        first_total_clock_uncertainty_ms: uncertainty.0,
        second_total_clock_uncertainty_ms: uncertainty.1,
        disposition: RelativeTimePairDisposition::Unresolved,
        unresolved_reason: Some(reason),
        query_result: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Capsule, Vec3, timed_co_translating_capsule_vs_capsule};

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn span(x: f64, dx: f64, start_ms: u64, end_ms: u64, source_clock: &str) -> CapsuleMotionSpan {
        CapsuleMotionSpan {
            capsule_at_start: Capsule::new(p(x, -1.0, 0.0), p(x, 1.0, 0.0), 0.5).unwrap(),
            translation: p(dx, 0.0, 0.0),
            start_ms,
            end_ms,
            site_epoch: "site-epoch-7".into(),
            frame_id: "site-world".into(),
            clock_domain_id: source_clock.into(),
            clock_uncertainty_ms: 0,
        }
    }

    fn mapping(
        source: &str,
        offset_min: i64,
        offset_max: i64,
        drift_ppm: u64,
    ) -> QualifiedClockMapping {
        QualifiedClockMapping {
            source_clock_domain_id: source.into(),
            site_clock_domain_id: "site-clock".into(),
            site_epoch: "site-epoch-7".into(),
            mapping_id: format!("map-{source}"),
            evidence_ref: format!("evidence://clock/{source}"),
            reference_source_time_ms: 1_000,
            offset_min_ms: offset_min,
            offset_max_ms: offset_max,
            drift_bound_ppm: Some(drift_ppm),
            valid_from_source_ms: 0,
            valid_until_source_ms: u64::MAX,
        }
    }

    fn body(
        id: &str,
        motion: CapsuleMotionSpan,
        clock_mapping: Option<QualifiedClockMapping>,
    ) -> ClockDomainCapsuleBody {
        ClockDomainCapsuleBody {
            body_id: id.into(),
            motion,
            clock_mapping,
        }
    }

    fn config() -> RelativeTimeMultiBodyConfig {
        RelativeTimeMultiBodyConfig {
            site_epoch: "site-epoch-7".into(),
            frame_id: "site-world".into(),
            site_clock_domain_id: "site-clock".into(),
            max_pair_checks: MAX_RELATIVE_TIME_PAIR_CHECKS,
        }
    }

    #[test]
    fn different_source_clocks_map_to_one_site_clock_and_detect_contact() {
        let bodies = [
            body(
                "robot-a",
                span(-10.0, 20.0, 0, 1_000, "robot-a-clock"),
                Some(mapping("robot-a-clock", 100, 100, 0)),
            ),
            body(
                "robot-b",
                span(10.0, -20.0, 100, 1_100, "robot-b-clock"),
                Some(mapping("robot-b-clock", 0, 0, 0)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        assert_eq!(report.pair_checks, 1);
        assert_eq!(report.exact_contact_pairs, 1);
        assert_eq!(
            report.pairs[0].disposition,
            RelativeTimePairDisposition::ExactContact
        );
        assert_eq!(report.pairs[0].first_body_id, "robot-a");
        assert_eq!(report.pairs[0].second_body_id, "robot-b");
    }

    #[test]
    fn offset_interval_and_drift_never_claim_exact_contact() {
        let bodies = [
            body(
                "robot-a",
                span(-10.0, 20.0, 0, 1_000, "robot-a-clock"),
                Some(mapping("robot-a-clock", 90, 110, 100)),
            ),
            body(
                "robot-b",
                span(10.0, -20.0, 100, 1_100, "robot-b-clock"),
                Some(mapping("robot-b-clock", -10, 10, 100)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        assert_eq!(report.potential_contact_unresolved_pairs, 1);
        assert_eq!(
            report.pairs[0].disposition,
            RelativeTimePairDisposition::PotentialContactUnresolved
        );
        assert!(report.pairs[0].first_total_clock_uncertainty_ms.unwrap() > 0);
        assert!(report.pairs[0].second_total_clock_uncertainty_ms.unwrap() > 0);
    }

    #[test]
    fn missing_or_expired_mapping_produces_pairwise_unresolved_receipt() {
        let bodies = [
            body("robot-a", span(0.0, 0.0, 0, 1_000, "robot-a-clock"), None),
            body(
                "robot-b",
                span(20.0, 0.0, 0, 1_000, "robot-b-clock"),
                Some(mapping("robot-b-clock", 0, 0, 0)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        assert_eq!(report.unresolved_pairs, 1);
        assert_eq!(
            report.pairs[0].unresolved_reason,
            Some(RelativeTimePairUnresolvedReason::FirstBodyMapping(
                ClockMappingFailure::MissingMapping
            ))
        );

        let mut expired = mapping("robot-a-clock", 0, 0, 0);
        expired.valid_until_source_ms = 900;
        let bodies = [
            body(
                "robot-a",
                span(0.0, 0.0, 0, 1_000, "robot-a-clock"),
                Some(expired),
            ),
            body(
                "robot-b",
                span(20.0, 0.0, 0, 1_000, "robot-b-clock"),
                Some(mapping("robot-b-clock", 0, 0, 0)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        assert_eq!(
            report.pairs[0].unresolved_reason,
            Some(RelativeTimePairUnresolvedReason::FirstBodyMapping(
                ClockMappingFailure::SpanOutsideValidity
            ))
        );
    }

    #[test]
    fn missing_drift_bound_and_wrong_source_clock_abstain() {
        let mut missing_drift = mapping("robot-a-clock", 0, 0, 0);
        missing_drift.drift_bound_ppm = None;
        let bodies = [
            body(
                "robot-a",
                span(0.0, 0.0, 0, 1_000, "robot-a-clock"),
                Some(missing_drift),
            ),
            body(
                "robot-b",
                span(20.0, 0.0, 0, 1_000, "robot-b-clock"),
                Some(mapping("robot-b-clock", 0, 0, 0)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        assert_eq!(
            report.pairs[0].unresolved_reason,
            Some(RelativeTimePairUnresolvedReason::FirstBodyMapping(
                ClockMappingFailure::MissingDriftBound
            ))
        );

        let bodies = [
            body(
                "robot-a",
                span(0.0, 0.0, 0, 1_000, "wrong-clock"),
                Some(mapping("robot-a-clock", 0, 0, 0)),
            ),
            body(
                "robot-b",
                span(20.0, 0.0, 0, 1_000, "robot-b-clock"),
                Some(mapping("robot-b-clock", 0, 0, 0)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        assert_eq!(
            report.pairs[0].unresolved_reason,
            Some(RelativeTimePairUnresolvedReason::FirstBodyMapping(
                ClockMappingFailure::SourceClockDomainMismatch
            ))
        );
    }

    #[test]
    fn drift_margin_uses_worst_endpoint_and_rounds_outward() {
        let a = body(
            "robot-a",
            span(0.0, 0.0, 0, 2_000_000, "robot-a-clock"),
            Some(QualifiedClockMapping {
                reference_source_time_ms: 1_000,
                drift_bound_ppm: Some(1_001),
                ..mapping("robot-a-clock", -2, 3, 1_001)
            }),
        );
        let b = body(
            "robot-b",
            span(50.0, 0.0, 0, 2_000_000, "robot-b-clock"),
            Some(mapping("robot-b-clock", 0, 0, 0)),
        );
        let report = evaluate_relative_time_multibody_ccd(&[a, b], &config()).unwrap();
        assert_eq!(report.body_mappings[0].drift_margin_ms, Some(2_001));
        assert_eq!(report.body_mappings[0].total_uncertainty_ms, Some(2_004));
    }

    #[test]
    fn input_order_does_not_change_pair_order_or_dispositions() {
        let a = body(
            "a",
            span(-10.0, 20.0, 0, 1_000, "clock-a"),
            Some(mapping("clock-a", 0, 0, 0)),
        );
        let b = body(
            "b",
            span(10.0, -20.0, 0, 1_000, "clock-b"),
            Some(mapping("clock-b", 0, 0, 0)),
        );
        let c = body(
            "c",
            span(500.0, 0.0, 0, 1_000, "clock-c"),
            Some(mapping("clock-c", 0, 0, 0)),
        );
        let forward =
            evaluate_relative_time_multibody_ccd(&[a.clone(), b.clone(), c.clone()], &config())
                .unwrap();
        let reversed = evaluate_relative_time_multibody_ccd(&[c, b, a], &config()).unwrap();
        assert_eq!(forward, reversed);
    }

    #[test]
    fn pair_budget_is_checked_before_quadratic_work() {
        let bodies = [
            body(
                "a",
                span(0.0, 0.0, 0, 1_000, "clock-a"),
                Some(mapping("clock-a", 0, 0, 0)),
            ),
            body(
                "b",
                span(10.0, 0.0, 0, 1_000, "clock-b"),
                Some(mapping("clock-b", 0, 0, 0)),
            ),
            body(
                "c",
                span(20.0, 0.0, 0, 1_000, "clock-c"),
                Some(mapping("clock-c", 0, 0, 0)),
            ),
        ];
        let mut constrained = config();
        constrained.max_pair_checks = 2;
        assert_eq!(
            evaluate_relative_time_multibody_ccd(&bodies, &constrained),
            Err(RelativeTimeMultiBodyError::PairBudgetExceeded)
        );
    }

    #[test]
    fn generated_physical_contacts_are_not_pruned_by_bounded_clock_mapping() {
        let mut seed = 0x5ea1_2026_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 2_001) as i64 - 1_000
        };
        let mut physical_contacts = 0_u32;
        for case in 0..64_u64 {
            let offset_a = (case as i64 % 21) - 10;
            let offset_b = ((case * 7) as i64 % 21) - 10;
            let start = 100_000_u64;
            let ax = if case < 16 { -10.0 } else { next() as f64 };
            let bx = if case < 16 { 10.0 } else { next() as f64 };
            let adx = if case < 16 { 20.0 } else { next() as f64 };
            let bdx = if case < 16 { -20.0 } else { next() as f64 };
            let source_a = span(ax, adx, start, start + 1_000, "clock-a");
            let source_b = span(bx, bdx, start, start + 1_000, "clock-b");
            let actual_a = CapsuleMotionSpan {
                start_ms: (start as i64 + offset_a) as u64,
                end_ms: (start as i64 + 1_000 + offset_a) as u64,
                ..source_a.clone()
            };
            let actual_b = CapsuleMotionSpan {
                start_ms: (start as i64 + offset_b) as u64,
                end_ms: (start as i64 + 1_000 + offset_b) as u64,
                ..source_b.clone()
            };
            let exact = timed_co_translating_capsule_vs_capsule(&actual_a, &actual_b);
            if exact.is_ok_and(|value| value.is_contact(0.5, 0.5)) {
                physical_contacts += 1;
                let bodies = [
                    body("a", source_a, Some(mapping("clock-a", -10, 10, 0))),
                    body("b", source_b, Some(mapping("clock-b", -10, 10, 0))),
                ];
                let report = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
                assert!(
                    matches!(
                        report.pairs[0].disposition,
                        RelativeTimePairDisposition::PotentialContactUnresolved
                            | RelativeTimePairDisposition::ExactContact
                    ),
                    "physical contact was pruned in generated case {case}"
                );
            }
        }
        assert!(physical_contacts > 0);
    }
}

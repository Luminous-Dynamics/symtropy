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

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Capsule, CapsuleMotionSpan, ClockUncertaintyCcdResult, ClockUncertaintyDisposition,
    ClockUncertaintyError, HierarchicalMultiBodyBroadphaseReport, MultiBodyBroadphaseError,
    MultiBodyBroadphaseReport, TimedCapsuleBody, clock_uncertainty_aware_capsule_query,
    time_swept_capsule_broadphase_hierarchical,
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
    CandidateIdentityMismatch,
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

    let source_uncertainty = u128::from(body.motion.clock_uncertainty_ms);
    let Some(source_uncertainty_product) =
        source_uncertainty.checked_mul(DRIFT_PPM_DENOMINATOR + u128::from(drift_bound_ppm))
    else {
        return fail(ClockMappingFailure::ClockUncertaintyOverflow);
    };
    let source_uncertainty_quotient = source_uncertainty_product / DRIFT_PPM_DENOMINATOR;
    let source_uncertainty_remainder = source_uncertainty_product % DRIFT_PPM_DENOMINATOR;
    let source_uncertainty_site_ms =
        source_uncertainty_quotient + u128::from(source_uncertainty_remainder != 0);
    let Some(total_uncertainty) = offset_half_width
        .checked_add(drift_margin)
        .and_then(|value| value.checked_add(source_uncertainty_site_ms))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OuterEnvelopeFailure {
    InvalidTimeInterval,
    TimestampUnderflow,
    TimestampOverflow,
    NonFiniteExpansion,
    InvalidExpandedCapsule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelativeTimeBroadphaseUnresolvedReason {
    FirstBodyMapping(ClockMappingFailure),
    SecondBodyMapping(ClockMappingFailure),
    FirstOuterEnvelope(OuterEnvelopeFailure),
    SecondOuterEnvelope(OuterEnvelopeFailure),
    PairwiseClockQuery(RelativeTimePairUnresolvedReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeTimeBroadphasePairDisposition {
    /// The conservative SAP broad phase emitted this pair and the independent
    /// pairwise clock-uncertainty oracle was run.
    PairwiseChecked,
    /// The emitted candidate could not be classified by the pairwise oracle.
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelativeTimeBroadphasePairReceipt {
    pub first_body_id: String,
    pub second_body_id: String,
    pub disposition: RelativeTimeBroadphasePairDisposition,
    pub candidate_slab_indices: Vec<u32>,
    pub pairwise_result: Option<RelativeTimePairReceipt>,
    pub unresolved_reason: Option<RelativeTimeBroadphaseUnresolvedReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativeTimeBroadphaseConfig {
    pub clock_mapping: RelativeTimeMultiBodyConfig,
    pub subdivision_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeTimeBroadphaseError {
    InvalidSubdivisionCount,
    NoBodies,
    TooManyBodies,
    EmptyBodyId,
    DuplicateBodyId,
    EmptyTargetIdentity,
    PairCountOverflow,
    PairBudgetExceeded,
    CandidateIdentityMismatch,
    Broadphase(MultiBodyBroadphaseError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelativeTimeBroadphaseReport {
    pub site_epoch: String,
    pub frame_id: String,
    pub site_clock_domain_id: String,
    pub body_count: usize,
    pub reference_pair_count: u64,
    pub valid_outer_envelope_bodies: u64,
    pub coarse_broadphase_candidate_pairs: u64,
    pub broadphase_candidate_pairs: u64,
    pub refinement_subdivision_count: u32,
    pub refinement_tested_pair_slabs: u64,
    pub pairwise_checked_pairs: u64,
    /// Aggregate count for valid-body pairs omitted by the conservative
    /// space-time SAP broad phase (both temporal and spatial pruning).
    pub outer_space_time_pruned_pairs: u64,
    /// Pairs touching at least one mapping/envelope-unresolved body, plus
    /// candidates for which pairwise checking abstained.
    pub unresolved_pairs: u64,
    pub pairwise_noncontact_candidates: u64,
    pub body_mappings: Vec<BodyClockMappingReceipt>,
    pub body_envelope_failures: Vec<(String, OuterEnvelopeFailure)>,
    pub broadphase: Option<MultiBodyBroadphaseReport>,
    pub pairs: Vec<RelativeTimeBroadphasePairReceipt>,
}

struct OuterEnvelopeBody {
    body_id: String,
    mapped: MappedBody,
    envelope_span: Option<CapsuleMotionSpan>,
    envelope_failure: Option<OuterEnvelopeFailure>,
}

/// Generate candidate pairs after mapping independent robot clocks into a
/// common site clock. Per-pair receipts are retained for candidates; pruned and
/// unresolved pair totals are aggregated so output size follows candidate count,
/// not the quadratic total possible-pair count.
///
/// For each body, the pairwise clock-uncertainty outer model is converted into
/// a zero-clock-uncertainty capsule span: its half-open interval is widened,
/// its starting capsule is rebased to the widened start, and its radius is
/// expanded by the declared maximum affine position error. The existing exact-
/// clock multi-body sweep-and-prune then operates on those outer spans.
///
/// Emitted pairs are checked with the independent pairwise clock-uncertainty
/// oracle. Pairs omitted by SAP are represented by an aggregate count, not
/// individual CLEAR receipts. The existing eight-ULP bounds expansion is a
/// numerical guard, not a formally directed-rounded proof. Invalid clock
/// mappings or unrepresentable outer envelopes never participate in pruning;
/// every pair touching such a body is counted as Unresolved.
pub fn evaluate_relative_time_multibody_ccd_with_broadphase(
    bodies: &[ClockDomainCapsuleBody],
    config: &RelativeTimeBroadphaseConfig,
) -> Result<RelativeTimeBroadphaseReport, RelativeTimeBroadphaseError> {
    if bodies.is_empty() {
        return Err(RelativeTimeBroadphaseError::NoBodies);
    }
    if bodies.len() > MAX_RELATIVE_TIME_BODIES {
        return Err(RelativeTimeBroadphaseError::TooManyBodies);
    }
    if config.subdivision_count == 0
        || !config.subdivision_count.is_power_of_two()
        || config.subdivision_count > crate::MAX_TIME_SLABS
    {
        return Err(RelativeTimeBroadphaseError::InvalidSubdivisionCount);
    }
    for value in [
        config.clock_mapping.site_epoch.as_str(),
        config.clock_mapping.frame_id.as_str(),
        config.clock_mapping.site_clock_domain_id.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RelativeTimeBroadphaseError::EmptyTargetIdentity);
        }
    }

    let body_count = bodies.len() as u64;
    let reference_pair_count = body_count
        .checked_mul(body_count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(RelativeTimeBroadphaseError::PairCountOverflow)?;
    if config.clock_mapping.max_pair_checks == 0
        || config.clock_mapping.max_pair_checks > MAX_RELATIVE_TIME_PAIR_CHECKS
        || reference_pair_count > config.clock_mapping.max_pair_checks
    {
        return Err(RelativeTimeBroadphaseError::PairBudgetExceeded);
    }

    let mut ids = BTreeSet::new();
    for body in bodies {
        if body.body_id.trim().is_empty() {
            return Err(RelativeTimeBroadphaseError::EmptyBodyId);
        }
        if !ids.insert(body.body_id.as_str()) {
            return Err(RelativeTimeBroadphaseError::DuplicateBodyId);
        }
    }

    let mut ordered: Vec<&ClockDomainCapsuleBody> = bodies.iter().collect();
    ordered.sort_by(|left, right| left.body_id.cmp(&right.body_id));
    let mut mapped_bodies = Vec::with_capacity(ordered.len());
    for body in ordered {
        let mapped = map_body_clock(body, &config.clock_mapping);
        let (envelope_span, envelope_failure) = match mapped.span.as_ref() {
            Some(span) => match build_clock_uncertainty_outer_span(span) {
                Ok(envelope) => (Some(envelope), None),
                Err(error) => (None, Some(error)),
            },
            None => (None, None),
        };
        mapped_bodies.push(OuterEnvelopeBody {
            body_id: body.body_id.clone(),
            mapped,
            envelope_span,
            envelope_failure,
        });
    }

    let outer_bodies: Vec<TimedCapsuleBody> = mapped_bodies
        .iter()
        .filter_map(|body| {
            body.envelope_span.as_ref().map(|motion| TimedCapsuleBody {
                body_id: body.body_id.clone(),
                motion: motion.clone(),
            })
        })
        .collect();

    // Coarse SAP is run once across the full horizon. Temporal refinement
    // then visits only this conservative candidate set, not every body in
    // every slab.
    let hierarchical: Option<HierarchicalMultiBodyBroadphaseReport> = if outer_bodies.len() >= 2 {
        Some(
            time_swept_capsule_broadphase_hierarchical(&outer_bodies, config.subdivision_count)
                .map_err(RelativeTimeBroadphaseError::Broadphase)?,
        )
    } else {
        None
    };
    let broadphase = hierarchical.as_ref().map(|report| report.coarse.clone());
    let refined_pairs = hierarchical
        .as_ref()
        .map_or(&[][..], |report| report.refinement.pairs.as_slice());
    let coarse_broadphase_candidate_pairs = hierarchical
        .as_ref()
        .map_or(0, |report| report.refinement.coarse_candidate_pairs);
    let broadphase_candidate_pairs = refined_pairs.len() as u64;
    let refinement_tested_pair_slabs = hierarchical
        .as_ref()
        .map_or(0, |report| report.refinement.tested_pair_slabs);
    let valid_outer_envelope_bodies = outer_bodies.len() as u64;
    let valid_outer_pair_count = valid_outer_envelope_bodies
        .checked_mul(valid_outer_envelope_bodies.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(RelativeTimeBroadphaseError::PairCountOverflow)?;
    if broadphase_candidate_pairs > valid_outer_pair_count {
        return Err(RelativeTimeBroadphaseError::PairBudgetExceeded);
    }
    let outer_space_time_pruned_pairs = valid_outer_pair_count - broadphase_candidate_pairs;
    let unresolved_body_pairs = reference_pair_count - valid_outer_pair_count;

    let mapped_by_id: BTreeMap<&str, &OuterEnvelopeBody> = mapped_bodies
        .iter()
        .map(|body| (body.body_id.as_str(), body))
        .collect();
    let mut output = RelativeTimeBroadphaseReport {
        site_epoch: config.clock_mapping.site_epoch.clone(),
        frame_id: config.clock_mapping.frame_id.clone(),
        site_clock_domain_id: config.clock_mapping.site_clock_domain_id.clone(),
        body_count: bodies.len(),
        reference_pair_count,
        valid_outer_envelope_bodies,
        coarse_broadphase_candidate_pairs,
        broadphase_candidate_pairs,
        refinement_subdivision_count: config.subdivision_count,
        refinement_tested_pair_slabs,
        pairwise_checked_pairs: 0,
        outer_space_time_pruned_pairs,
        unresolved_pairs: unresolved_body_pairs,
        pairwise_noncontact_candidates: 0,
        body_mappings: mapped_bodies
            .iter()
            .map(|body| body.mapped.receipt.clone())
            .collect(),
        body_envelope_failures: mapped_bodies
            .iter()
            .filter_map(|body| {
                body.envelope_failure
                    .map(|failure| (body.body_id.clone(), failure))
            })
            .collect(),
        broadphase,
        pairs: Vec::with_capacity(broadphase_candidate_pairs as usize),
    };

    for candidate in refined_pairs {
        let Some(first) = mapped_by_id.get(candidate.first_body_id.as_str()).copied() else {
            return Err(RelativeTimeBroadphaseError::CandidateIdentityMismatch);
        };
        let Some(second) = mapped_by_id.get(candidate.second_body_id.as_str()).copied() else {
            return Err(RelativeTimeBroadphaseError::CandidateIdentityMismatch);
        };
        let pairwise = evaluate_pair(&first.mapped, &second.mapped);
        let unresolved_reason = if pairwise.disposition == RelativeTimePairDisposition::Unresolved {
            pairwise
                .unresolved_reason
                .clone()
                .map(RelativeTimeBroadphaseUnresolvedReason::PairwiseClockQuery)
        } else {
            None
        };
        if unresolved_reason.is_some() {
            output.unresolved_pairs += 1;
        }
        if matches!(
            pairwise.disposition,
            RelativeTimePairDisposition::ExactNoContact
                | RelativeTimePairDisposition::NoPossibleContact
                | RelativeTimePairDisposition::NoPossibleTemporalOverlap
        ) {
            output.pairwise_noncontact_candidates += 1;
        }
        output.pairwise_checked_pairs += 1;
        output.pairs.push(RelativeTimeBroadphasePairReceipt {
            first_body_id: candidate.first_body_id.clone(),
            second_body_id: candidate.second_body_id.clone(),
            disposition: if unresolved_reason.is_some() {
                RelativeTimeBroadphasePairDisposition::Unresolved
            } else {
                RelativeTimeBroadphasePairDisposition::PairwiseChecked
            },
            candidate_slab_indices: candidate
                .candidate_slabs
                .iter()
                .map(|slab| slab.slab_index)
                .collect(),
            pairwise_result: Some(pairwise),
            unresolved_reason,
        });
    }
    Ok(output)
}

/// Build a zero-uncertainty span that encloses the complete clock-offset
/// uncertainty tube represented by the pairwise oracle:
/// widened interval [start-epsilon, end+epsilon), linear extrapolation at both
/// ends, and capsule radius expanded by |translation|*epsilon/duration.
fn build_clock_uncertainty_outer_span(
    span: &CapsuleMotionSpan,
) -> Result<CapsuleMotionSpan, OuterEnvelopeFailure> {
    if span.start_ms >= span.end_ms {
        return Err(OuterEnvelopeFailure::InvalidTimeInterval);
    }
    if span.clock_uncertainty_ms == 0 {
        if !finite_vec3(span.translation) {
            return Err(OuterEnvelopeFailure::NonFiniteExpansion);
        }
        Capsule::new(
            span.capsule_at_start.start,
            span.capsule_at_start.end,
            span.capsule_at_start.radius_mm,
        )
        .map_err(|_| OuterEnvelopeFailure::InvalidExpandedCapsule)?;
        return Ok(span.clone());
    }

    let widened_start = span
        .start_ms
        .checked_sub(span.clock_uncertainty_ms)
        .ok_or(OuterEnvelopeFailure::TimestampUnderflow)?;
    let widened_end = span
        .end_ms
        .checked_add(span.clock_uncertainty_ms)
        .ok_or(OuterEnvelopeFailure::TimestampOverflow)?;
    if widened_start >= widened_end {
        return Err(OuterEnvelopeFailure::InvalidTimeInterval);
    }

    let duration = (span.end_ms - span.start_ms) as f64;
    let uncertainty_fraction = span.clock_uncertainty_ms as f64 / duration;
    let translation_norm = span
        .translation
        .x
        .hypot(span.translation.y)
        .hypot(span.translation.z);
    let margin = translation_norm * uncertainty_fraction;
    let expanded_radius = span.capsule_at_start.radius_mm + margin;
    let widened_duration_fraction = 1.0 + 2.0 * uncertainty_fraction;
    let start_shift = span.translation.scale(-uncertainty_fraction);
    let widened_translation = span.translation.scale(widened_duration_fraction);
    let start = span.capsule_at_start.start.add(start_shift);
    let end = span.capsule_at_start.end.add(start_shift);
    if !margin.is_finite()
        || !expanded_radius.is_finite()
        || !widened_duration_fraction.is_finite()
        || !finite_vec3(start)
        || !finite_vec3(end)
        || !finite_vec3(widened_translation)
    {
        return Err(OuterEnvelopeFailure::NonFiniteExpansion);
    }
    let capsule = Capsule::new(start, end, expanded_radius)
        .map_err(|_| OuterEnvelopeFailure::InvalidExpandedCapsule)?;

    let mut outer = span.clone();
    outer.capsule_at_start = capsule;
    outer.translation = widened_translation;
    outer.start_ms = widened_start;
    outer.end_ms = widened_end;
    outer.clock_uncertainty_ms = 0;
    Ok(outer)
}

fn finite_vec3(value: crate::Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
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
        expired.reference_source_time_ms = 500;
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
            let mut actual_a = CapsuleMotionSpan {
                start_ms: (start as i64 + offset_a) as u64,
                end_ms: (start as i64 + 1_000 + offset_a) as u64,
                ..source_a.clone()
            };
            let mut actual_b = CapsuleMotionSpan {
                start_ms: (start as i64 + offset_b) as u64,
                end_ms: (start as i64 + 1_000 + offset_b) as u64,
                ..source_b.clone()
            };
            actual_a.clock_domain_id = "site-clock".into();
            actual_b.clock_domain_id = "site-clock".into();
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

    #[test]
    fn source_timestamp_uncertainty_is_scaled_by_the_clock_rate_bound() {
        let mut motion = span(0.0, 0.0, 0, 1_000, "clock-a");
        motion.clock_uncertainty_ms = 3;
        let a = body("a", motion, Some(mapping("clock-a", 0, 0, 500)));
        let b = body(
            "b",
            span(50.0, 0.0, 0, 1_000, "clock-b"),
            Some(mapping("clock-b", 0, 0, 0)),
        );
        let report = evaluate_relative_time_multibody_ccd(&[a, b], &config()).unwrap();
        // ceil(3 ms * 1.0005 rate) + ceil(1,000 ms * 500 ppm) = 4 + 1.
        assert_eq!(report.body_mappings[0].total_uncertainty_ms, Some(5));
    }

    fn broadphase_config() -> RelativeTimeBroadphaseConfig {
        RelativeTimeBroadphaseConfig {
            clock_mapping: config(),
            subdivision_count: 8,
        }
    }

    #[test]
    fn broadphase_keeps_all_exhaustive_potential_pairs() {
        let bodies = [
            body(
                "robot-a",
                span(-10.0, 20.0, 1_000, 2_000, "clock-a"),
                Some(mapping("clock-a", -10, 10, 100)),
            ),
            body(
                "robot-b",
                span(10.0, -20.0, 1_000, 2_000, "clock-b"),
                Some(mapping("clock-b", -10, 10, 100)),
            ),
            body(
                "robot-c",
                span(10_000.0, 0.0, 1_000, 2_000, "clock-c"),
                Some(mapping("clock-c", -10, 10, 100)),
            ),
            body(
                "robot-d",
                span(20_000.0, 0.0, 1_000, 2_000, "clock-d"),
                Some(mapping("clock-d", -10, 10, 100)),
            ),
        ];

        let reference = evaluate_relative_time_multibody_ccd(&bodies, &config()).unwrap();
        let accelerated =
            evaluate_relative_time_multibody_ccd_with_broadphase(&bodies, &broadphase_config())
                .unwrap();
        let candidates: BTreeSet<_> = accelerated
            .pairs
            .iter()
            .filter(|pair| {
                pair.disposition == RelativeTimeBroadphasePairDisposition::PairwiseChecked
            })
            .map(|pair| (pair.first_body_id.clone(), pair.second_body_id.clone()))
            .collect();

        for pair in reference.pairs.iter().filter(|pair| {
            matches!(
                pair.disposition,
                RelativeTimePairDisposition::ExactContact
                    | RelativeTimePairDisposition::BoundaryCandidateUnresolved
                    | RelativeTimePairDisposition::PotentialContactUnresolved
            )
        }) {
            assert!(
                candidates.contains(&(pair.first_body_id.clone(), pair.second_body_id.clone())),
                "broad phase pruned exhaustive potential pair {} / {}",
                pair.first_body_id,
                pair.second_body_id
            );
        }
        assert!(candidates.contains(&("robot-a".into(), "robot-b".into())));
        assert!(accelerated.outer_space_time_pruned_pairs > 0);
        assert_eq!(
            accelerated.broadphase_candidate_pairs,
            accelerated.pairs.len() as u64
        );
    }

    #[test]
    fn sparse_scene_is_pruned_without_materializing_quadratic_pruned_receipts() {
        let bodies: Vec<_> = (0..32)
            .map(|index| {
                let id = format!("robot-{index:03}");
                let x = index as f64 * 1_000.0;
                body(
                    &id,
                    span(x, 0.0, 1_000, 2_000, &format!("clock-{index:03}")),
                    Some(mapping(&format!("clock-{index:03}"), -5, 5, 20)),
                )
            })
            .collect();
        let report =
            evaluate_relative_time_multibody_ccd_with_broadphase(&bodies, &broadphase_config())
                .unwrap();
        assert_eq!(report.reference_pair_count, 496);
        assert_eq!(report.pairs.len(), 0);
        assert_eq!(report.broadphase_candidate_pairs, 0);
        assert_eq!(report.outer_space_time_pruned_pairs, 496);
        assert_eq!(report.unresolved_pairs, 0);
    }

    #[test]
    fn missing_mapping_and_unrepresentable_outer_envelope_are_never_pruned() {
        let missing = [
            body("a", span(0.0, 0.0, 1_000, 2_000, "clock-a"), None),
            body(
                "b",
                span(10.0, 0.0, 1_000, 2_000, "clock-b"),
                Some(mapping("clock-b", 0, 0, 0)),
            ),
        ];
        let missing_report =
            evaluate_relative_time_multibody_ccd_with_broadphase(&missing, &broadphase_config())
                .unwrap();
        assert_eq!(missing_report.unresolved_pairs, 1);
        assert_eq!(missing_report.pairs.len(), 0);
        assert!(matches!(
            missing_report.body_mappings[0].disposition,
            ClockMappingDisposition::Unresolved(ClockMappingFailure::MissingMapping)
        ));

        let mut at_zero = span(0.0, 0.0, 0, 1_000, "clock-a");
        at_zero.clock_uncertainty_ms = 10;
        let unrepresentable = [
            body(
                "a",
                at_zero,
                Some(QualifiedClockMapping {
                    reference_source_time_ms: 500,
                    valid_from_source_ms: 0,
                    valid_until_source_ms: 2_000,
                    ..mapping("clock-a", 0, 0, 0)
                }),
            ),
            body(
                "b",
                span(10.0, 0.0, 1_000, 2_000, "clock-b"),
                Some(mapping("clock-b", 0, 0, 0)),
            ),
        ];
        let report = evaluate_relative_time_multibody_ccd_with_broadphase(
            &unrepresentable,
            &broadphase_config(),
        )
        .unwrap();
        assert_eq!(report.unresolved_pairs, 1);
        assert_eq!(report.pairs.len(), 0);
        assert_eq!(
            report.body_envelope_failures,
            vec![("a".into(), OuterEnvelopeFailure::TimestampUnderflow)]
        );
    }

    #[test]
    fn clock_outer_span_contains_endpoint_offset_trajectories() {
        let source = span(0.0, 100.0, 1_000, 2_000, "clock-a");
        let mut uncertain = source.clone();
        uncertain.clock_uncertainty_ms = 50;
        let outer = build_clock_uncertainty_outer_span(&uncertain).unwrap();
        assert_eq!(outer.start_ms, 950);
        assert_eq!(outer.end_ms, 2_050);
        assert_eq!(outer.clock_uncertainty_ms, 0);
        assert!((outer.capsule_at_start.start.x + 5.0).abs() < 1.0e-9);
        assert!((outer.capsule_at_start.radius_mm - 5.5).abs() < 1.0e-9);

        for site_time in [950_u64, 1_000, 1_250, 1_500, 1_999, 2_049] {
            let fraction =
                (site_time - outer.start_ms) as f64 / (outer.end_ms - outer.start_ms) as f64;
            let outer_center = outer.capsule_at_start.start.x + outer.translation.x * fraction;
            for offset in [-50_i64, 0, 50] {
                let source_time = site_time as i64 - offset;
                let source_fraction = (source_time - 1_000) as f64 / 1_000.0;
                let actual_center = source.translation.x * source_fraction;
                assert!(
                    (actual_center - outer_center).abs()
                        <= outer.capsule_at_start.radius_mm - source.capsule_at_start.radius_mm
                            + 1.0e-9,
                    "clock-offset pose escaped outer capsule at t={site_time}, offset={offset}"
                );
            }
        }
    }

    #[test]
    fn generated_multibody_candidate_set_is_a_superset_of_exhaustive_potential_pairs() {
        let base_ms = 2_000_000_u64;
        let mut bodies = Vec::new();
        for index in 0..64_u64 {
            let pair = index / 2;
            let (x, dx) = if index < 16 {
                if index % 2 == 0 {
                    (-10.0 + pair as f64 * 1_000.0, 20.0)
                } else {
                    (10.0 + pair as f64 * 1_000.0, -20.0)
                }
            } else {
                (
                    ((index * 137) % 4_000) as f64,
                    ((index * 53) % 41) as f64 - 20.0,
                )
            };
            let clock = format!("generated-clock-{index:03}");
            let start = base_ms + (pair % 4) * 50;
            let motion = span(x, dx, start, start + 1_000, &clock);
            let mapping = QualifiedClockMapping {
                source_clock_domain_id: clock.clone(),
                site_clock_domain_id: "site-clock".into(),
                site_epoch: "site-epoch-7".into(),
                mapping_id: format!("generated-map-{index:03}"),
                evidence_ref: format!("synthetic://clock/{index:03}"),
                reference_source_time_ms: start + 500,
                offset_min_ms: -3,
                offset_max_ms: 3,
                drift_bound_ppm: Some(100),
                valid_from_source_ms: base_ms,
                valid_until_source_ms: base_ms + 10_000,
            };
            bodies.push(body(&format!("robot-{index:03}"), motion, Some(mapping)));
        }

        let clock_config = config();
        let broadphase_config = broadphase_config();
        let reference = evaluate_relative_time_multibody_ccd(&bodies, &clock_config).unwrap();
        let accelerated =
            evaluate_relative_time_multibody_ccd_with_broadphase(&bodies, &broadphase_config)
                .unwrap();
        let candidates: BTreeSet<_> = accelerated
            .pairs
            .iter()
            .map(|pair| (pair.first_body_id.clone(), pair.second_body_id.clone()))
            .collect();

        for pair in reference.pairs.iter().filter(|pair| {
            matches!(
                pair.disposition,
                RelativeTimePairDisposition::ExactContact
                    | RelativeTimePairDisposition::BoundaryCandidateUnresolved
                    | RelativeTimePairDisposition::PotentialContactUnresolved
            )
        }) {
            assert!(
                candidates.contains(&(pair.first_body_id.clone(), pair.second_body_id.clone())),
                "generated possible pair was pruned: {} / {}",
                pair.first_body_id,
                pair.second_body_id
            );
        }
        assert_eq!(
            accelerated.reference_pair_count,
            accelerated.broadphase_candidate_pairs
                + accelerated.outer_space_time_pruned_pairs
                + accelerated.unresolved_pairs
        );
        assert_eq!(
            accelerated.pairwise_checked_pairs,
            accelerated.broadphase_candidate_pairs
        );
        assert_eq!(accelerated.unresolved_pairs, 0);
    }

    #[test]
    fn invalid_zero_uncertainty_geometry_is_unresolved_not_a_global_broadphase_abort() {
        let mut invalid_motion = span(0.0, 0.0, 1_000, 2_000, "clock-a");
        invalid_motion.translation.x = f64::NAN;
        let bodies = [
            body("a", invalid_motion, Some(mapping("clock-a", 0, 0, 0))),
            body(
                "b",
                span(10.0, 0.0, 1_000, 2_000, "clock-b"),
                Some(mapping("clock-b", 0, 0, 0)),
            ),
        ];
        let report =
            evaluate_relative_time_multibody_ccd_with_broadphase(&bodies, &broadphase_config())
                .unwrap();
        assert_eq!(report.unresolved_pairs, 1);
        assert_eq!(
            report.body_envelope_failures,
            vec![("a".into(), OuterEnvelopeFailure::NonFiniteExpansion)]
        );
    }
}

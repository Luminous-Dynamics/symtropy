//! Aviation-specific adapter from upstream PNT/time evidence to a bounded
//! initial position set for the existing AeroWeft reachability primitive.
//!
//! This crate does not perform sensor fusion, clock conversion, PNT detection,
//! dependency-graph analysis, aircraft dynamics, trajectory scheduling or
//! flight authorization. It consumes outputs from those existing owners and
//! fails closed when the claim is not admissible for a deterministic hard
//! occupancy bound.
//!
//! All test profiles and position/velocity values are synthetic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use aeroweft_reachability_core::Interval;

/// Upstream PNT service state supplied by Symtropy #3909 / #4079.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PntServiceState {
    /// Primary service is qualified under its declared profile.
    Normal,
    /// Primary service is degraded; only a separately qualified fallback is considered.
    Degraded,
    /// Primary navigation integrity is explicitly uncertain.
    IntegrityUncertain,
    /// Primary service is unavailable.
    Unavailable,
    /// Upstream service is still performing its requalification procedure.
    Recovering,
    /// Upstream service or evidence state is indeterminate.
    Indeterminate,
}

/// Claim class. Statistical predictions are deliberately not hard bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundKind {
    /// Deterministic closed enclosure under the declared model/profile.
    DeterministicHardBound,
    /// Statistical region, confidence ellipse, prediction or learned estimate.
    StatisticalPrediction,
}

/// Qualification state of the already-normalized clock correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockQualification {
    /// Mycelix #435 supplies a valid, current correlation profile.
    Qualified,
    /// Clock correlation is known to be invalid or outside its validity interval.
    Unqualified,
    /// Time-scale/clock identity or correlation status is incomplete.
    Unknown,
}

/// Independence result from the exact Mycelix #5904 observation snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndependenceAssessment {
    /// Complete dependency evidence meets the profile's distinct-family requirement.
    QualifiedWithinProfile,
    /// Channels share one or more declared relevant failure domains.
    SharedOrCorrelated,
    /// The dependency graph is incomplete or unknown.
    Unknown,
    /// Independently declared evidence contradicts on the current observation set.
    Contradictory,
}

/// Three-dimensional position enclosure in the declared local frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionBounds {
    /// East position interval, metres.
    pub east_m: Interval,
    /// North position interval, metres.
    pub north_m: Interval,
    /// Up position interval, metres from the declared vertical datum.
    pub up_m: Interval,
}

/// Three-dimensional instantaneous velocity enclosure supplied by the aircraft-model owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityBounds {
    /// East velocity interval, metres/second.
    pub east_mps: Interval,
    /// North velocity interval, metres/second.
    pub north_mps: Interval,
    /// Up velocity interval, metres/second.
    pub up_mps: Interval,
}

/// One source claim already assessed by the upstream evidence owners.
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationClaim {
    /// Stable source-claim identifier.
    pub source_id: String,
    /// Source claim digest syntax (the adapter does not authenticate or fetch bytes).
    pub source_claim_sha256: String,
    /// Navigation/model profile expected for this source.
    pub profile_id: String,
    /// Horizontal coordinate frame identifier.
    pub horizontal_frame_id: String,
    /// Vertical datum identifier.
    pub vertical_datum_id: String,
    /// The type of bound carried by the claim.
    pub bound_kind: BoundKind,
    /// Lower sample observation time, normalized by Mycelix #435.
    pub sample_time_lower_utc_ms: u64,
    /// Upper sample observation time, normalized by Mycelix #435.
    pub sample_time_upper_utc_ms: u64,
    /// Time-scale identity for the normalized sample interval.
    pub sample_time_scale_id: String,
    /// Time-scale identity for the assessment epoch.
    pub assessment_time_scale_id: String,
    /// Clock-correlation qualification from Mycelix #435.
    pub clock_qualification: ClockQualification,
    /// Exact upstream clock-correlation reference.
    pub clock_correlation_id: Option<String>,
    /// Clock-correlation artifact digest syntax.
    pub clock_correlation_sha256: Option<String>,
    /// Common-mode assessment from Mycelix #5904.
    pub independence: IndependenceAssessment,
    /// Number of independent evidence families in the declared snapshot.
    pub independent_family_count: Option<u16>,
    /// Whether the dependency snapshot is complete for this claim.
    pub dependency_snapshot_complete: bool,
    /// Exact dependency snapshot identifier.
    pub dependency_snapshot_id: Option<String>,
    /// Position enclosure at the source observation time.
    pub position: PositionBounds,
    /// Valid instantaneous velocity enclosure over the observation-age interval.
    pub velocity: VelocityBounds,
}

/// Operation/profile-specific requirements for this adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterPolicy {
    /// Required navigation/reachability profile.
    pub profile_id: String,
    /// Required horizontal frame.
    pub horizontal_frame_id: String,
    /// Required vertical datum.
    pub vertical_datum_id: String,
    /// Maximum permitted age of the oldest possible observation time.
    pub maximum_sample_age_ms: u64,
    /// Minimum independent evidence-family count required by the profile.
    pub minimum_independent_families: u16,
}

/// Input to the aviation-specific PNT/reachability gate.
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterInput {
    /// Upstream navigation/PNT service state.
    pub service_state: PntServiceState,
    /// Primary source claim, if supplied.
    pub primary: Option<NavigationClaim>,
    /// Independent fallback claim, if supplied.
    pub fallback: Option<NavigationClaim>,
    /// Lower bound of the assessment-time interval, normalized by Mycelix #435.
    pub assessment_time_lower_utc_ms: u64,
    /// Upper bound of the assessment-time interval, normalized by Mycelix #435.
    pub assessment_time_upper_utc_ms: u64,
    /// Assessment time-scale identity.
    pub assessment_time_scale_id: String,
    /// Qualification of the evaluation clock correlation.
    pub assessment_clock_qualification: ClockQualification,
    /// Identity of the evaluation clock-correlation artifact.
    pub assessment_clock_correlation_id: Option<String>,
    /// Digest syntax for the evaluation clock-correlation artifact.
    pub assessment_clock_correlation_sha256: Option<String>,
    /// Exact upstream PNT service-state receipt digest syntax.
    pub service_state_receipt_sha256: String,
    /// Active operation/reachability policy.
    pub policy: AdapterPolicy,
}

/// Exact reason that a claim cannot enter the deterministic occupancy path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAssessableReason {
    /// No primary source claim was supplied for a NORMAL state.
    MissingClaim,
    /// A degraded/unavailable/uncertain primary has no separately qualified fallback.
    NoQualifiedIndependentFallback,
    /// A statistical prediction was supplied instead of a deterministic hard bound.
    StatisticalEnvelopeNotHardBound,
    /// The source claim or upstream state receipt has invalid digest syntax.
    InvalidDigest,
    /// Profile identity does not match the active policy.
    ProfileMismatch,
    /// Horizontal frame or vertical datum does not match the active policy.
    FrameOrDatumMismatch,
    /// Sample-time interval is reversed.
    InvalidSampleTimeInterval,
    /// The sample-time interval overlaps or extends beyond the assessment epoch.
    SampleTimeOverlapsAssessmentEpoch,
    /// The oldest possible sample time exceeds the profile's maximum age.
    SampleAgeExceedsProfileLimit,
    /// The sample and assessment time-scale identities do not match.
    TimeScaleMismatch,
    /// The evaluation clock correlation is missing or unqualified.
    AssessmentClockUnqualified,
    /// The assessment-time interval is reversed.
    InvalidAssessmentTimeInterval,
    /// Clock correlation is not qualified.
    ClockCorrelationUnqualified,
    /// Clock correlation state is unknown.
    ClockCorrelationUnknown,
    /// Clock correlation identity or digest is absent.
    MissingClockCorrelation,
    /// Independence evidence is shared, unknown, or contradictory.
    IndependenceNotQualified,
    /// The dependency inventory is incomplete or its snapshot identity is absent.
    DependencySnapshotIncomplete,
    /// Independent family count is absent or below the profile requirement.
    InsufficientIndependentFamilies,
    /// Source identifier is empty.
    EmptySourceId,
    /// Position/velocity arithmetic overflowed or was not representable.
    ArithmeticFailure,
    /// The upstream service remains in its recovery state.
    UpstreamStillRecovering,
    /// The upstream PNT/service state is indeterminate.
    UpstreamIndeterminate,
    /// A required policy identifier or independent-family count is invalid.
    InvalidPolicy,
    /// The declared time-scale identity is absent.
    InvalidTimeScaleIdentity,
}


/// Evidence lineage retained by every accepted initial-position receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedClaimLineage {
    /// Stable source-claim identifier.
    pub source_id: String,
    /// Exact source-claim digest syntax.
    pub source_claim_sha256: String,
    /// Navigation/reachability profile identity.
    pub profile_id: String,
    /// Horizontal coordinate frame identity.
    pub horizontal_frame_id: String,
    /// Vertical datum identity.
    pub vertical_datum_id: String,
    /// Source observation clock-correlation identity.
    pub clock_correlation_id: String,
    /// Source observation clock-correlation digest syntax.
    pub clock_correlation_sha256: String,
    /// Evaluation clock-correlation identity.
    pub assessment_clock_correlation_id: String,
    /// Evaluation clock-correlation digest syntax.
    pub assessment_clock_correlation_sha256: String,
    /// Upstream PNT service-state receipt digest syntax.
    pub service_state_receipt_sha256: String,
    /// Exact source observation-time interval.
    pub sample_time_interval_utc_ms: [u64; 2],
    /// Exact evaluation-time interval.
    pub assessment_time_interval_utc_ms: [u64; 2],
    /// Exact Mycelix common-mode/dependency snapshot identity.
    pub dependency_snapshot_id: String,
    /// Independence assessment from the exact observation snapshot.
    pub independence_assessment: IndependenceAssessment,
    /// Independent evidence-family count admitted under the profile.
    pub independent_family_count: u16,
    /// Minimum family count required by the operation profile.
    pub minimum_independent_families: u16,
    /// Position enclosure at the source observation epoch.
    pub source_position_bounds: PositionBounds,
    /// Velocity enclosure used to propagate the source observation.
    pub velocity_bounds: VelocityBounds,
}

/// Result of the PNT/time admission and position-set expansion.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterOutcome {
    /// Primary deterministic hard bound was admitted and expanded.
    AcceptedPrimary {
        /// Complete source/time/profile/dependency lineage.
        lineage: AcceptedClaimLineage,
        /// Derived elapsed-time interval in seconds.
        elapsed_seconds: Interval,
        /// Conservative position enclosure at the evaluation epoch.
        position: PositionBounds,
    },
    /// Independent fallback deterministic hard bound was admitted and expanded.
    AcceptedFallback {
        /// Complete fallback source/time/profile/dependency lineage.
        lineage: AcceptedClaimLineage,
        /// Derived elapsed-time interval in seconds.
        elapsed_seconds: Interval,
        /// Conservative position enclosure at the evaluation epoch.
        position: PositionBounds,
    },
    /// No deterministic hard-bound initial position is admissible.
    NotAssessable {
        /// Typed reason; no empty or clear occupancy is implied.
        reason: NotAssessableReason,
    },
}

/// Evaluate one upstream state and transform the allowed claim into a bounded initial position.
///
/// NORMAL admits the primary claim only. DEGRADED, INTEGRITY_UNCERTAIN and
/// UNAVAILABLE admit only a separately qualified fallback. RECOVERING and
/// INDETERMINATE are not admitted; upstream owners must complete requalification
/// and provide a new admissible service state.
///
/// The returned position can be supplied to the existing reachability primitive.
/// It is not a route, clearance, safety decision or physical-truth determination.
pub fn evaluate(input: &AdapterInput) -> AdapterOutcome {
    if !is_sha256(&input.service_state_receipt_sha256) {
        return reject(NotAssessableReason::InvalidDigest);
    }

    let (claim, accepted_as_fallback) = match input.service_state {
        PntServiceState::Normal => {
            let Some(claim) = input.primary.as_ref() else {
                return reject(NotAssessableReason::MissingClaim);
            };
            (claim, false)
        }
        PntServiceState::Degraded
        | PntServiceState::IntegrityUncertain
        | PntServiceState::Unavailable => {
            let Some(claim) = input.fallback.as_ref() else {
                return reject(NotAssessableReason::NoQualifiedIndependentFallback);
            };
            (claim, true)
        }
        PntServiceState::Recovering => {
            return reject(NotAssessableReason::UpstreamStillRecovering);
        }
        PntServiceState::Indeterminate => {
            return reject(NotAssessableReason::UpstreamIndeterminate);
        }
    };

    if input.policy.profile_id.trim().is_empty()
        || input.policy.horizontal_frame_id.trim().is_empty()
        || input.policy.vertical_datum_id.trim().is_empty()
        || input.policy.minimum_independent_families == 0
    {
        return reject(NotAssessableReason::InvalidPolicy);
    }
    if input.assessment_time_scale_id.trim().is_empty() {
        return reject(NotAssessableReason::InvalidTimeScaleIdentity);
    }

    match evaluate_claim(claim, input) {
        Ok((elapsed_seconds, position)) => {
            let Some(dependency_snapshot_id) = claim.dependency_snapshot_id.clone() else {
                return reject(NotAssessableReason::DependencySnapshotIncomplete);
            };
            let Some(clock_correlation_id) = claim.clock_correlation_id.clone() else {
                return reject(NotAssessableReason::MissingClockCorrelation);
            };
            let Some(clock_correlation_sha256) = claim.clock_correlation_sha256.clone() else {
                return reject(NotAssessableReason::MissingClockCorrelation);
            };
            let Some(assessment_clock_correlation_id) =
                input.assessment_clock_correlation_id.as_deref()
            else {
                return reject(NotAssessableReason::AssessmentClockUnqualified);
            };
            if assessment_clock_correlation_id.trim().is_empty() {
                return reject(NotAssessableReason::AssessmentClockUnqualified);
            }
            let Some(assessment_clock_correlation_sha256) =
                input.assessment_clock_correlation_sha256.as_deref()
            else {
                return reject(NotAssessableReason::AssessmentClockUnqualified);
            };

            let Some(independent_family_count) = claim.independent_family_count else {
                return reject(NotAssessableReason::InsufficientIndependentFamilies);
            };
            let lineage = AcceptedClaimLineage {
                source_id: claim.source_id.clone(),
                source_claim_sha256: claim.source_claim_sha256.clone(),
                profile_id: claim.profile_id.clone(),
                horizontal_frame_id: claim.horizontal_frame_id.clone(),
                vertical_datum_id: claim.vertical_datum_id.clone(),
                clock_correlation_id,
                clock_correlation_sha256,
                assessment_clock_correlation_id: assessment_clock_correlation_id.to_owned(),
                assessment_clock_correlation_sha256: assessment_clock_correlation_sha256.to_owned(),
                service_state_receipt_sha256: input.service_state_receipt_sha256.clone(),
                sample_time_interval_utc_ms: [
                    claim.sample_time_lower_utc_ms,
                    claim.sample_time_upper_utc_ms,
                ],
                assessment_time_interval_utc_ms: [
                    input.assessment_time_lower_utc_ms,
                    input.assessment_time_upper_utc_ms,
                ],
                dependency_snapshot_id,
                independence_assessment: claim.independence,
                independent_family_count,
                minimum_independent_families: input.policy.minimum_independent_families,
                source_position_bounds: claim.position,
                velocity_bounds: claim.velocity,
            };

            if accepted_as_fallback {
                AdapterOutcome::AcceptedFallback {
                    lineage,
                    elapsed_seconds,
                    position,
                }
            } else {
                AdapterOutcome::AcceptedPrimary {
                    lineage,
                    elapsed_seconds,
                    position,
                }
            }
        }
        Err(reason) => reject(reason),
    }
}

fn evaluate_claim(
    claim: &NavigationClaim,
    input: &AdapterInput,
) -> Result<(Interval, PositionBounds), NotAssessableReason> {
    if claim.source_id.trim().is_empty() {
        return Err(NotAssessableReason::EmptySourceId);
    }
    if !is_sha256(&claim.source_claim_sha256) {
        return Err(NotAssessableReason::InvalidDigest);
    }
    if claim.bound_kind != BoundKind::DeterministicHardBound {
        return Err(NotAssessableReason::StatisticalEnvelopeNotHardBound);
    }
    if claim.profile_id != input.policy.profile_id {
        return Err(NotAssessableReason::ProfileMismatch);
    }
    if claim.horizontal_frame_id != input.policy.horizontal_frame_id
        || claim.vertical_datum_id != input.policy.vertical_datum_id
    {
        return Err(NotAssessableReason::FrameOrDatumMismatch);
    }
    if claim.sample_time_scale_id.trim().is_empty()
        || claim.assessment_time_scale_id.trim().is_empty()
    {
        return Err(NotAssessableReason::InvalidTimeScaleIdentity);
    }
    if claim.sample_time_scale_id != input.assessment_time_scale_id
        || claim.assessment_time_scale_id != input.assessment_time_scale_id
    {
        return Err(NotAssessableReason::TimeScaleMismatch);
    }
    if input.assessment_time_lower_utc_ms > input.assessment_time_upper_utc_ms {
        return Err(NotAssessableReason::InvalidAssessmentTimeInterval);
    }
    if input.assessment_clock_qualification != ClockQualification::Qualified
        || !input
            .assessment_clock_correlation_sha256
            .as_deref()
            .is_some_and(is_sha256)
    {
        return Err(NotAssessableReason::AssessmentClockUnqualified);
    }
    match claim.clock_qualification {
        ClockQualification::Qualified => {}
        ClockQualification::Unqualified => {
            return Err(NotAssessableReason::ClockCorrelationUnqualified);
        }
        ClockQualification::Unknown => {
            return Err(NotAssessableReason::ClockCorrelationUnknown);
        }
    }
    let Some(correlation_id) = claim.clock_correlation_id.as_deref() else {
        return Err(NotAssessableReason::MissingClockCorrelation);
    };
    if correlation_id.trim().is_empty()
        || !claim
            .clock_correlation_sha256
            .as_deref()
            .is_some_and(is_sha256)
    {
        return Err(NotAssessableReason::MissingClockCorrelation);
    }
    if claim.independence != IndependenceAssessment::QualifiedWithinProfile {
        return Err(NotAssessableReason::IndependenceNotQualified);
    }
    if !claim.dependency_snapshot_complete
        || !claim
            .dependency_snapshot_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(NotAssessableReason::DependencySnapshotIncomplete);
    }
    let Some(independent_count) = claim.independent_family_count else {
        return Err(NotAssessableReason::InsufficientIndependentFamilies);
    };
    if independent_count < input.policy.minimum_independent_families {
        return Err(NotAssessableReason::InsufficientIndependentFamilies);
    }
    if claim.sample_time_lower_utc_ms > claim.sample_time_upper_utc_ms {
        return Err(NotAssessableReason::InvalidSampleTimeInterval);
    }
    if claim.sample_time_upper_utc_ms > input.assessment_time_lower_utc_ms {
        return Err(NotAssessableReason::SampleTimeOverlapsAssessmentEpoch);
    }

    let oldest_possible_age_ms =
        input.assessment_time_upper_utc_ms - claim.sample_time_lower_utc_ms;
    if oldest_possible_age_ms > input.policy.maximum_sample_age_ms {
        return Err(NotAssessableReason::SampleAgeExceedsProfileLimit);
    }

    let elapsed_min_ms =
        input.assessment_time_lower_utc_ms - claim.sample_time_upper_utc_ms;
    let elapsed_max_ms = oldest_possible_age_ms;
    let Some(elapsed_seconds) = elapsed_interval_seconds(elapsed_min_ms, elapsed_max_ms)
    else {
        return Err(NotAssessableReason::ArithmeticFailure);
    };

    let position = PositionBounds {
        east_m: claim
            .position
            .east_m
            .checked_add(
                claim
                    .velocity
                    .east_mps
                    .checked_mul(elapsed_seconds)
                    .map_err(|_| NotAssessableReason::ArithmeticFailure)?,
            )
            .map_err(|_| NotAssessableReason::ArithmeticFailure)?,
        north_m: claim
            .position
            .north_m
            .checked_add(
                claim
                    .velocity
                    .north_mps
                    .checked_mul(elapsed_seconds)
                    .map_err(|_| NotAssessableReason::ArithmeticFailure)?,
            )
            .map_err(|_| NotAssessableReason::ArithmeticFailure)?,
        up_m: claim
            .position
            .up_m
            .checked_add(
                claim
                    .velocity
                    .up_mps
                    .checked_mul(elapsed_seconds)
                    .map_err(|_| NotAssessableReason::ArithmeticFailure)?,
            )
            .map_err(|_| NotAssessableReason::ArithmeticFailure)?,
    };

    Ok((elapsed_seconds, position))
}

const MAX_EXACT_INTEGER_MS: u64 = 9_007_199_254_740_991;

fn elapsed_interval_seconds(
    lower_ms: u64,
    upper_ms: u64,
) -> Option<Interval> {
    if lower_ms > upper_ms || upper_ms > MAX_EXACT_INTEGER_MS {
        return None;
    }
    // Keep elapsed integer milliseconds within the core's documented exact
    // binary64 integer domain before converting to fractional seconds.
    let lower_nearest = lower_ms as f64 / 1_000.0;
    let upper_nearest = upper_ms as f64 / 1_000.0;
    if !lower_nearest.is_finite() || !upper_nearest.is_finite() {
        return None;
    }

    // Preserve an exact zero lower bound. Otherwise widen the rounded
    // millisecond-to-second conversion by one representable value in each
    // direction before it is multiplied by the supplied velocity interval.
    let lower = if lower_ms == 0 {
        0.0
    } else {
        next_down(lower_nearest)
    };
    let upper = next_up(upper_nearest);
    Interval::new(lower, upper).ok()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

fn reject(reason: NotAssessableReason) -> AdapterOutcome {
    AdapterOutcome::NotAssessable { reason }
}

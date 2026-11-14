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

use aeroweift_reachability_core::Interval;

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
    /// Assessment time, normalized to the declared assessment scale.
    pub assessment_time_utc_ms: u64,
    /// Assessment time-scale identity.
    pub assessment_time_scale_id: String,
    /// Exact upstream PNT service-state receipt digest syntax.
    pub service_state_receipt_sha256: String,
    /// Active operation/reachability policy.
    pub policy: AdapterPolicy,
}

/// Exact reason that a claim cannot enter the deterministic occupancy path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAssessableReason {
    /// No source claim was supplied for the state-allowed role.
    MissingClaim,
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
    /// Clock correlation is not qualified.
    ClockCorrelationUnqualified,
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
    /// The service state provides no admissible source role in this adapter version.
    StateNotAdmissible,
}

/// Result of the PNT/time admission and position-set expansion.
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterOutcome {
    /// Primary deterministic hard bound was admitted and expanded.
    AcceptedPrimary {
        /// Source identifier preserved from the claim.
        source_id: String,
        /// Exact source claim digest syntax preserved from the claim.
        source_claim_sha256: String,
        /// Dependency snapshot identifier preserved from the claim.
        dependency_snapshot_id: String,
        /// Derived elapsed-time interval in seconds.
        elapsed_seconds: Interval,
        /// Conservative position enclosure at the assessment epoch.
        position: PositionBounds,
    },
    /// Independent fallback deterministic hard bound was admitted and expanded.
    AcceptedFallback {
        /// Source identifier preserved from the fallback claim.
        source_id: String,
        /// Exact source claim digest syntax preserved from the fallback.
        source_claim_sha256: String,
        /// Dependency snapshot identifier preserved from the fallback.
        dependency_snapshot_id: String,
        /// Derived elapsed-time interval in seconds.
        elapsed_seconds: Interval,
        /// Conservative position enclosure at the assessment epoch.
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
                return reject(NotAssessableReason::MissingClaim);
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

    match evaluate_claim(claim, input) {
        Ok((elapsed_seconds, position)) => {
            let Some(dependency_snapshot_id) = claim.dependency_snapshot_id.clone() else {
                return reject(NotAssessableReason::DependencySnapshotIncomplete);
            };
            if accepted_as_fallback {
                AdapterOutcome::AcceptedFallback {
                    source_id: claim.source_id.clone(),
                    source_claim_sha256: claim.source_claim_sha256.clone(),
                    dependency_snapshot_id,
                    elapsed_seconds,
                    position,
                }
            } else {
                AdapterOutcome::AcceptedPrimary {
                    source_id: claim.source_id.clone(),
                    source_claim_sha256: claim.source_claim_sha256.clone(),
                    dependency_snapshot_id,
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
    if claim.sample_time_scale_id != input.assessment_time_scale_id
        || claim.assessment_time_scale_id != input.assessment_time_scale_id
    {
        return Err(NotAssessableReason::TimeScaleMismatch);
    }
    if claim.clock_qualification != ClockQualification::Qualified {
        return Err(NotAssessableReason::ClockCorrelationUnqualified);
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
    if claim.sample_time_upper_utc_ms > input.assessment_time_utc_ms {
        return Err(NotAssessableReason::SampleTimeOverlapsAssessmentEpoch);
    }

    let oldest_possible_age_ms =
        input.assessment_time_utc_ms - claim.sample_time_lower_utc_ms;
    if oldest_possible_age_ms > input.policy.maximum_sample_age_ms {
        return Err(NotAssessableReason::SampleAgeExceedsProfileLimit);
    }

    let elapsed_min_ms =
        input.assessment_time_utc_ms - claim.sample_time_upper_utc_ms;
    let elapsed_max_ms = oldest_possible_age_ms;
    let elapsed_seconds = elapsed_interval_seconds(elapsed_min_ms, elapsed_max_ms)
        .map_err(|_| NotAssessableReason::ArithmeticFailure)?;

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

fn elapsed_interval_seconds(
    lower_ms: u64,
    upper_ms: u64,
) -> Result<Interval, ()> {
    if lower_ms > upper_ms {
        return Err(());
    }
    // Accepted profile ages are deliberately bounded by the operation policy,
    // and therefore lie inside the exact integer range of binary64.
    let lower_nearest = lower_ms as f64 / 1_000.0;
    let upper_nearest = upper_ms as f64 / 1_000.0;
    if !lower_nearest.is_finite() || !upper_nearest.is_finite() {
        return Err(());
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
    Interval::new(lower, upper).map_err(|_| ())
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

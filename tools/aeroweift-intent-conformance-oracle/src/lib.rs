//! Finite reference oracle for operational-intent conformance.
 //!
 //! It compares integer-bounded observed 4D occupancy windows with a versioned
 //! authorization and profile-defined deviation expansion. It separately
 //! reports definite, possible, unassessable and unobserved duration.
 //!
 //! This crate is not a UTM/U-space service, a flight-authority mechanism,
 //! conflict-resolution engine, aircraft model, avoidance policy or controller.
 //! All test values are fictional and synthetic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Closed integer interval for a spatial coordinate or observation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosedInterval {
    /// Inclusive lower endpoint.
    pub lower: i64,
    /// Inclusive upper endpoint.
    pub upper: i64,
}

impl ClosedInterval {
    fn is_valid(self) -> bool {
        self.lower <= self.upper
    }

    fn contains(self, other: Self) -> bool {
        self.lower <= other.lower && other.upper <= self.upper
    }

    fn is_disjoint(self, other: Self) -> bool {
        self.upper < other.lower || other.upper < self.lower
    }

    fn expand(self, amount: i64) -> Option<Self> {
        Some(Self {
            lower: self.lower.checked_sub(amount)?,
            upper: self.upper.checked_add(amount)?,
        })
    }
}

/// Half-open integer time interval [start, end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeInterval {
    /// Inclusive start tick.
    pub start: i64,
    /// Exclusive end tick.
    pub end: i64,
}

impl TimeInterval {
    /// Duration in integer ticks, if the interval is valid.
    pub fn duration(self) -> Option<u64> {
        if self.start >= self.end {
            return None;
        }
        u64::try_from(self.end.checked_sub(self.start)?).ok()
    }

    fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    fn is_disjoint(self, other: Self) -> bool {
        self.end <= other.start || other.end <= self.start
    }

    fn intersects(self, other: Self) -> bool {
        !self.is_disjoint(other)
    }

    fn expand(self, amount: i64) -> Option<Self> {
        Some(Self {
            start: self.start.checked_sub(amount)?,
            end: self.end.checked_add(amount)?,
        })
    }
}

/// Spatial 3D axis-aligned box with integer coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionBox {
    /// East-axis interval.
    pub east: ClosedInterval,
    /// North-axis interval.
    pub north: ClosedInterval,
    /// Up-axis interval.
    pub up: ClosedInterval,
}

impl PositionBox {
    fn is_valid(self) -> bool {
        self.east.is_valid() && self.north.is_valid() && self.up.is_valid()
    }
}

/// Space-time 4D box. Space is closed; time is half-open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume4D {
    /// East-axis interval.
    pub east: ClosedInterval,
    /// North-axis interval.
    pub north: ClosedInterval,
    /// Up-axis interval.
    pub up: ClosedInterval,
    /// Half-open time interval.
    pub time: TimeInterval,
}

impl Volume4D {
    fn is_valid(self) -> bool {
        self.east.is_valid()
            && self.north.is_valid()
            && self.up.is_valid()
            && self.time.duration().is_some()
    }

    fn expand(self, expansion: DeviationExpansion) -> Option<Self> {
        Some(Self {
            east: self.east.expand(expansion.east)?,
            north: self.north.expand(expansion.north)?,
            up: self.up.expand(expansion.up)?,
            time: self.time.expand(expansion.time)?,
        })
    }

    fn contains(self, other: Self) -> bool {
        self.east.contains(other.east)
            && self.north.contains(other.north)
            && self.up.contains(other.up)
            && self.time.contains(other.time)
    }

    fn is_disjoint(self, other: Self) -> bool {
        self.east.is_disjoint(other.east)
            || self.north.is_disjoint(other.north)
            || self.up.is_disjoint(other.up)
            || self.time.is_disjoint(other.time)
    }

    fn intersects(self, other: Self) -> bool {
        !self.is_disjoint(other)
    }
}

/// Explicit, profile-owned expansion around the authorized 4D volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviationExpansion {
    /// Additional east/west bound in synthetic coordinate units.
    pub east: i64,
    /// Additional north/south bound in synthetic coordinate units.
    pub north: i64,
    /// Additional vertical bound in synthetic coordinate units.
    pub up: i64,
    /// Additional time allowance in ticks on each end.
    pub time: i64,
}

impl DeviationExpansion {
    fn is_valid(self) -> bool {
        self.east >= 0 && self.north >= 0 && self.up >= 0 && self.time >= 0
    }
}

/// Versioned authorization used for the conformance comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    /// Current authorization identity.
    pub authorization_id: String,
    /// Authorization identity currently recognized by the caller.
    pub current_authorization_id: String,
    /// Operational-intent identity.
    pub intent_id: String,
    /// Revision bound to this authorization.
    pub intent_revision: u64,
    /// Current operational-intent revision.
    pub current_intent_revision: u64,
    /// Issuing authority reference (opaque; no authority is inferred).
    pub issuer_reference: String,
    /// Artifact digest syntax for the authorization.
    pub authorization_sha256: String,
    /// Declared time-scale identity; no conversion is performed here.
    pub time_scale_id: String,
    /// Horizontal coordinate frame identity.
    pub horizontal_frame_id: String,
    /// Vertical datum identity.
    pub vertical_datum_id: String,
    /// Activation tick.
    pub activated_at_tick: i64,
    /// Exclusive authorization expiry tick.
    pub expires_at_tick: i64,
    /// Whether the authorization has been withdrawn.
    pub withdrawn: bool,
    /// Authorized 4D volume.
    pub volume: Volume4D,
}

/// A profile-specific conformance and observation policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformancePolicy {
    /// Stable policy identity.
    pub policy_id: String,
    /// Aircraft/operation applicability profile identity.
    pub profile_id: String,
    /// Declared comparison time scale.
    pub time_scale_id: String,
    /// Evaluation window over which duration is accumulated.
    pub evaluation_window: TimeInterval,
    /// Current evaluation tick for contact-loss determination.
    pub evaluation_tick: i64,
    /// Maximum age of an observation, measured from observed-at time to receipt.
    pub maximum_observation_age_ticks: u64,
    /// Time after the last qualified support window before the state is contingent.
    pub contact_loss_timeout_ticks: u64,
    /// Maximum definite-outside share in parts per million.
    pub maximum_definite_outside_ppm: u32,
    /// Explicit deviation expansion; no universal epsilon is inferred.
    pub deviation_expansion: DeviationExpansion,
}

/// One time-bounded, qualified observation enclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationWindow {
    /// Stable observation/track identity.
    pub observation_id: String,
    /// Observation source identity.
    pub source_id: String,
    /// Time interval over which this bounded observation claim applies.
    pub support_window: TimeInterval,
    /// Interval-valued measurement timestamp, already normalized to the policy scale.
    pub observed_at: ClosedInterval,
    /// Receipt tick. It does not replace observed-at time for freshness.
    pub receipt_tick: i64,
    /// Time-scale identity for the observed-at interval.
    pub time_scale_id: String,
    /// Position enclosure over the complete support window.
    pub position: PositionBox,
    /// Whether upstream PNT/time qualification succeeded.
    pub pnt_time_qualified: bool,
    /// Exact upstream PNT/time qualification receipt identity.
    pub pnt_qualification_reference: String,
    /// Whether upstream surveillance/track qualification succeeded.
    pub surveillance_qualified: bool,
    /// Exact upstream surveillance/track qualification receipt identity.
    pub surveillance_qualification_reference: String,
    /// Horizontal coordinate frame identity.
    pub horizontal_frame_id: String,
    /// Vertical datum identity.
    pub vertical_datum_id: String,
}

/// New dynamic airspace restriction with an explicit effective interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRestriction {
    /// Restriction snapshot identity.
    pub restriction_id: String,
    /// Issuing authority reference for the restriction snapshot.
    pub issuer_reference: String,
    /// Monotonic restriction snapshot revision.
    pub snapshot_revision: u64,
    /// Restriction artifact digest syntax.
    pub restriction_sha256: String,
    /// Declared time-scale identity; no conversion is performed here.
    pub time_scale_id: String,
    /// Effective interval of the restriction.
    pub effective_window: TimeInterval,
    /// Restricted 4D volume.
    pub volume: Volume4D,
}

/// Complete finite input to the conformance evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceInput {
    /// Current authorization claim.
    pub authorization: Authorization,
    /// Profile-specific conformance policy.
    pub policy: ConformancePolicy,
    /// Observation windows; order is canonicalized by support-window start.
    pub observations: Vec<ObservationWindow>,
    /// Optional newly effective dynamic restriction.
    pub dynamic_restriction: Option<DynamicRestriction>,
}

/// Conformance state, deliberately distinct from a flight authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceStatus {
    /// Every evaluated interval is qualified and within the permitted 4D volume.
    Conforming,
    /// Some outside occupancy is possible or definite, but confirmation is not met.
    PotentiallyNonconforming,
    /// Definite outside exposure exceeds the configured profile threshold.
    Nonconforming,
    /// Qualified contact is lost beyond the explicit profile timeout.
    Contingent,
    /// Authorization is expired, withdrawn, superseded or invalidated by a restriction.
    ReauthorizationRequired,
    /// Required evidence or input semantics are absent, stale or unsupported.
    NotAssessable,
}

/// Exact reason accompanying a non-green/authority-dependent status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceReason {
    /// Current authorization ID differs from the supplied authorization.
    AuthorizationIdentityChanged,
    /// Operational-intent revision has been superseded.
    IntentRevisionSuperseded,
    /// Authorization has been withdrawn.
    AuthorizationWithdrawn,
    /// Authorization has expired at the evaluation tick.
    AuthorizationExpired,
    /// A currently effective dynamic restriction intersects the authorization.
    DynamicRestrictionIntersection,
    /// Policy or authorization fields are invalid.
    InvalidPolicyOrAuthorization,
    /// Time-scale identity is missing or mismatched.
    TimeScaleMismatch,
    /// Dynamic-restriction evidence is invalid.
    InvalidDynamicRestriction,
    /// Dynamic restriction uses a different time scale from the policy.
    DynamicRestrictionTimeScaleMismatch,
    /// PNT qualification reference is missing.
    MissingPntQualificationReference,
    /// Surveillance qualification reference is missing.
    MissingSurveillanceQualificationReference,
    /// Observation support window is invalid or outside the evaluation window.
    InvalidObservationWindow,
    /// Observation windows overlap and would double-count duration.
    OverlappingObservationWindows,
    /// Observation timestamp is reversed or outside its support window.
    InvalidObservedAtInterval,
    /// Receipt precedes the latest possible observation time.
    ReceiptBeforeObservation,
    /// Receipt tick is later than the evaluation tick.
    ReceiptAfterEvaluation,
    /// Observation is stale based on observed-at, not receipt time.
    StaleObservation,
    /// Upstream PNT/time evidence is unqualified.
    PntTimeUnqualified,
    /// Upstream surveillance/track evidence is unqualified.
    SurveillanceUnqualified,
    /// Horizontal frame or vertical datum does not match the authorization.
    FrameOrDatumMismatch,
    /// Observation cannot be proven to belong to an admitted time scale.
    ObservationTimeScaleUnknown,
    /// No qualified observation is present.
    NoQualifiedObservation,
    /// Last qualified observation exceeds the explicit contact-loss timeout.
    ContactTimeout,
    /// One or more observation sets overlap both permitted and outside space-time.
    ObservedSetStraddlesDeviationBoundary,
    /// Definite deviation exists but does not yet exceed the policy threshold.
    DefiniteDeviationBelowConfirmationThreshold,
    /// Definite deviation exceeds the policy threshold.
    DefiniteDeviationThresholdExceeded,
    /// Arithmetic or duration accounting overflowed.
    ArithmeticOverflow,
}

/// Evidence and authority lineage preserved with each assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceLineage {
    /// Complete authorization input, including identity, revision and 4D volume.
    pub authorization: Authorization,
    /// Complete conformance policy, including evaluation window and thresholds.
    pub policy: ConformancePolicy,
    /// Exact dynamic restriction snapshot considered, if supplied.
    pub dynamic_restriction: Option<DynamicRestriction>,
    /// Complete observation inputs in canonical support-window order.
    pub canonicalized_observations: Vec<ObservationWindow>,
    /// IDs of observations admitted into duration accounting.
    pub accepted_observation_ids: Vec<String>,
    /// IDs of observations rejected as unassessable.
    pub rejected_observation_ids: Vec<String>,
}

/// Conformance result and separately accounted evidence durations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceAssessment {
    /// Final conformance state.
    pub status: ConformanceStatus,
    /// Reason for a non-conforming/contingent/unknown state, if present.
    pub reason: Option<ConformanceReason>,
    /// Definitely-inside qualified duration.
    pub definite_inside_ticks: u64,
    /// Definitely-outside qualified duration.
    pub definite_outside_ticks: u64,
    /// Possible-outside duration, including definite-outside duration.
    pub possible_outside_ticks: u64,
    /// Duration covered by unassessable observations.
    pub unassessable_ticks: u64,
    /// Duration without an observation support claim.
    pub unobserved_ticks: u64,
    /// Total duration of qualified observation support.
    pub qualified_observed_ticks: u64,
    /// Definite outside lower-bound share in parts per million of the full evaluation window.
    pub definite_outside_ppm: u32,
    /// Possible outside upper-bound share in parts per million of the full evaluation window.
    /// Unobserved and unassessable time contributes to this upper bound.
    pub possible_outside_ppm: u32,
    /// Exact authority/observation lineage.
    pub lineage: ConformanceLineage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoxRelation {
    Inside,
    Outside,
    Ambiguous,
}

/// Evaluate observed 4D support windows against a versioned authorization.
///
/// Time intervals are half-open. Spatial intervals are closed. All clocks must
/// already be normalized into the same declared scale by the upstream clock
/// owner. The evaluator never generates aircraft motion or an authorization.
pub fn evaluate(input: &ConformanceInput) -> ConformanceAssessment {
    let mut lineage = ConformanceLineage {
        authorization: input.authorization.clone(),
        policy: input.policy.clone(),
        dynamic_restriction: input.dynamic_restriction.clone(),
        canonicalized_observations: Vec::new(),
        accepted_observation_ids: Vec::new(),
        rejected_observation_ids: Vec::new(),
    };

    let empty = |status, reason, lineage| ConformanceAssessment {
        status,
        reason,
        definite_inside_ticks: 0,
        definite_outside_ticks: 0,
        possible_outside_ticks: 0,
        unassessable_ticks: 0,
        unobserved_ticks: 0,
        qualified_observed_ticks: 0,
        definite_outside_ppm: 0,
        possible_outside_ppm: 0,
        lineage,
    };

    let auth = &input.authorization;
    let policy = &input.policy;

    if auth.authorization_id.trim().is_empty()
        || auth.current_authorization_id.trim().is_empty()
        || auth.intent_id.trim().is_empty()
        || auth.issuer_reference.trim().is_empty()
        || auth.horizontal_frame_id.trim().is_empty()
        || auth.vertical_datum_id.trim().is_empty()
        || auth.time_scale_id.trim().is_empty()
        || !is_sha256(&auth.authorization_sha256)
        || !auth.volume.is_valid()
        || auth.activated_at_tick >= auth.expires_at_tick
        || policy.policy_id.trim().is_empty()
        || policy.profile_id.trim().is_empty()
        || policy.time_scale_id.trim().is_empty()
        || policy.evaluation_window.duration().is_none()
        || policy.evaluation_tick < policy.evaluation_window.end
        || policy.maximum_definite_outside_ppm > 1_000_000
        || policy.evaluation_tick < auth.activated_at_tick
        || !policy.deviation_expansion.is_valid()
    {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::InvalidPolicyOrAuthorization),
            lineage,
        );
    }

    if auth.authorization_id != auth.current_authorization_id {
        return empty(
            ConformanceStatus::ReauthorizationRequired,
            Some(ConformanceReason::AuthorizationIdentityChanged),
            lineage,
        );
    }
    if auth.intent_revision != auth.current_intent_revision {
        return empty(
            ConformanceStatus::ReauthorizationRequired,
            Some(ConformanceReason::IntentRevisionSuperseded),
            lineage,
        );
    }
    if auth.withdrawn {
        return empty(
            ConformanceStatus::ReauthorizationRequired,
            Some(ConformanceReason::AuthorizationWithdrawn),
            lineage,
        );
    }
    if policy.evaluation_tick >= auth.expires_at_tick {
        return empty(
            ConformanceStatus::ReauthorizationRequired,
            Some(ConformanceReason::AuthorizationExpired),
            lineage,
        );
    }
    if policy.time_scale_id != auth.time_scale_id {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::TimeScaleMismatch),
            lineage,
        );
    }

    if let Some(restriction) = input.dynamic_restriction.as_ref() {
        if restriction.restriction_id.trim().is_empty()
            || restriction.issuer_reference.trim().is_empty()
            || restriction.snapshot_revision == 0
            || !is_sha256(&restriction.restriction_sha256)
            || restriction.effective_window.duration().is_none()
            || !restriction.volume.is_valid()
        {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::InvalidDynamicRestriction),
                lineage,
            );
        }
        if restriction.time_scale_id != policy.time_scale_id {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::DynamicRestrictionTimeScaleMismatch),
                lineage,
            );
        }
        let effective_start = restriction
            .volume
            .time
            .start
            .max(restriction.effective_window.start)
            .max(policy.evaluation_window.start);
        let effective_end = restriction
            .volume
            .time
            .end
            .min(restriction.effective_window.end)
            .min(policy.evaluation_window.end);
        let effective = effective_start < effective_end;
        let effective_volume = Volume4D {
            time: TimeInterval {
                start: effective_start,
                end: effective_end,
            },
            ..restriction.volume
        };
        if effective && effective_volume.intersects(auth.volume) {
            return empty(
                ConformanceStatus::ReauthorizationRequired,
                Some(ConformanceReason::DynamicRestrictionIntersection),
                lineage,
            );
        }
    }

    let Some(allowed_volume) = auth.volume.expand(policy.deviation_expansion) else {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::ArithmeticOverflow),
            lineage,
        );
    };

    let mut observations: Vec<_> = input.observations.iter().collect();
    observations.sort_by_key(|observation| {
        (
            observation.support_window.start,
            observation.support_window.end,
            observation.observation_id.as_str(),
        )
    });
    lineage.canonicalized_observations =
        observations.iter().map(|observation| (**observation).clone()).collect();

    let mut definite_inside = 0_u64;
    let mut definite_outside = 0_u64;
    let mut possible_outside = 0_u64;
    let mut unassessable = 0_u64;
    let mut covered_window_ticks = 0_u64;
    let mut last_qualified_end: Option<i64> = None;
    let mut previous_end: Option<i64> = None;
    let mut first_unassessable_reason = None;

    for observation in observations {
        let support = observation.support_window;
        let Some(duration) = support.duration() else {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::InvalidObservationWindow),
                lineage,
            );
        };
        if !policy.evaluation_window.contains(support) {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::InvalidObservationWindow),
                lineage,
            );
        }
        if previous_end.is_some_and(|end| support.start < end) {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::OverlappingObservationWindows),
                lineage,
            );
        }
        previous_end = Some(support.end);
        let Some(next_covered) = covered_window_ticks.checked_add(duration) else {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::ArithmeticOverflow),
                lineage,
            );
        };
        covered_window_ticks = next_covered;

        let reason = if observation.observation_id.trim().is_empty()
            || observation.source_id.trim().is_empty()
            || !observation.position.is_valid()
        {
            Some(ConformanceReason::InvalidObservationWindow)
        } else if !observation.observed_at.is_valid()
            || observation.observed_at.lower < support.start
            || observation.observed_at.upper >= support.end
        {
            Some(ConformanceReason::InvalidObservedAtInterval)
        } else if observation.receipt_tick < observation.observed_at.upper {
            Some(ConformanceReason::ReceiptBeforeObservation)
        } else if observation.receipt_tick > policy.evaluation_tick {
            Some(ConformanceReason::ReceiptAfterEvaluation)
        } else if observation
            .receipt_tick
            .checked_sub(observation.observed_at.lower)
            .and_then(|age| u64::try_from(age).ok())
            .is_none_or(|age| age > policy.maximum_observation_age_ticks)
        {
            Some(ConformanceReason::StaleObservation)
        } else if observation.time_scale_id != policy.time_scale_id {
            Some(ConformanceReason::TimeScaleMismatch)
        } else if !observation.pnt_time_qualified {
            Some(ConformanceReason::PntTimeUnqualified)
        } else if observation.pnt_qualification_reference.trim().is_empty() {
            Some(ConformanceReason::MissingPntQualificationReference)
        } else if !observation.surveillance_qualified {
            Some(ConformanceReason::SurveillanceUnqualified)
        } else if observation.surveillance_qualification_reference.trim().is_empty() {
            Some(ConformanceReason::MissingSurveillanceQualificationReference)
        } else if observation.horizontal_frame_id != auth.horizontal_frame_id
            || observation.vertical_datum_id != auth.vertical_datum_id
        {
            Some(ConformanceReason::FrameOrDatumMismatch)
        } else {
            None
        };

        if let Some(reason) = reason {
            unassessable = match unassessable.checked_add(duration) {
                Some(value) => value,
                None => {
                    return empty(
                        ConformanceStatus::NotAssessable,
                        Some(ConformanceReason::ArithmeticOverflow),
                        lineage,
                    );
                }
            };
            lineage
                .rejected_observation_ids
                .push(observation.observation_id.clone());
            first_unassessable_reason.get_or_insert(reason);
            continue;
        }

        let observed_volume = Volume4D {
            east: observation.position.east,
            north: observation.position.north,
            up: observation.position.up,
            time: support,
        };
        let relation = if allowed_volume.contains(observed_volume) {
            BoxRelation::Inside
        } else if allowed_volume.is_disjoint(observed_volume) {
            BoxRelation::Outside
        } else {
            BoxRelation::Ambiguous
        };

        match relation {
            BoxRelation::Inside => {
                definite_inside = match definite_inside.checked_add(duration) {
                    Some(value) => value,
                    None => {
                        return empty(
                            ConformanceStatus::NotAssessable,
                            Some(ConformanceReason::ArithmeticOverflow),
                            lineage,
                        );
                    }
                };
            }
            BoxRelation::Outside => {
                definite_outside = match definite_outside.checked_add(duration) {
                    Some(value) => value,
                    None => {
                        return empty(
                            ConformanceStatus::NotAssessable,
                            Some(ConformanceReason::ArithmeticOverflow),
                            lineage,
                        );
                    }
                };
                possible_outside = match possible_outside.checked_add(duration) {
                    Some(value) => value,
                    None => {
                        return empty(
                            ConformanceStatus::NotAssessable,
                            Some(ConformanceReason::ArithmeticOverflow),
                            lineage,
                        );
                    }
                };
            }
            BoxRelation::Ambiguous => {
                possible_outside = match possible_outside.checked_add(duration) {
                    Some(value) => value,
                    None => {
                        return empty(
                            ConformanceStatus::NotAssessable,
                            Some(ConformanceReason::ArithmeticOverflow),
                            lineage,
                        );
                    }
                };
            }
        }

        lineage
            .accepted_observation_ids
            .push(observation.observation_id.clone());
        last_qualified_end = Some(
            last_qualified_end.map_or(support.end, |previous| previous.max(support.end)),
        );
    }

    let Some(evaluation_duration) = policy.evaluation_window.duration() else {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::InvalidPolicyOrAuthorization),
            lineage,
        );
    };
    let Some(unobserved) = evaluation_duration.checked_sub(covered_window_ticks) else {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::ArithmeticOverflow),
            lineage,
        );
    };

    let qualified_observed = match definite_inside.checked_add(possible_outside) {
        Some(value) => value,
        None => {
            return empty(
                ConformanceStatus::NotAssessable,
                Some(ConformanceReason::ArithmeticOverflow),
                lineage,
            );
        }
    };
    let Some(possible_or_unknown_ticks) = possible_outside
        .checked_add(unassessable)
        .and_then(|value| value.checked_add(unobserved))
    else {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::ArithmeticOverflow),
            lineage,
        );
    };
    // The definite fraction is a lower bound over the full evaluation window.
    // The possible fraction is an upper bound: all unobserved/unassessable time
    // remains unresolved rather than being counted as observed-inside time.
    let definite_outside_ppm = ratio_ppm(definite_outside, evaluation_duration);
    let possible_outside_ppm = ratio_ppm(possible_or_unknown_ticks, evaluation_duration);

    if definite_outside_ppm > policy.maximum_definite_outside_ppm {
        return ConformanceAssessment {
            status: ConformanceStatus::Nonconforming,
            reason: Some(ConformanceReason::DefiniteDeviationThresholdExceeded),
            definite_inside_ticks: definite_inside,
            definite_outside_ticks: definite_outside,
            possible_outside_ticks: possible_outside,
            unassessable_ticks: unassessable,
            unobserved_ticks: unobserved,
            qualified_observed_ticks: qualified_observed,
            definite_outside_ppm,
            possible_outside_ppm,
            lineage,
        };
    }

    if unassessable > 0 {
        return ConformanceAssessment {
            status: ConformanceStatus::NotAssessable,
            reason: first_unassessable_reason,
            definite_inside_ticks: definite_inside,
            definite_outside_ticks: definite_outside,
            possible_outside_ticks: possible_outside,
            unassessable_ticks: unassessable,
            unobserved_ticks: unobserved,
            qualified_observed_ticks: qualified_observed,
            definite_outside_ppm,
            possible_outside_ppm,
            lineage,
        };
    }

    let last_end = last_qualified_end.unwrap_or(auth.activated_at_tick);
    let Some(time_since_last) = policy.evaluation_tick.checked_sub(last_end) else {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::ArithmeticOverflow),
            lineage,
        );
    };
    let Ok(time_since_last) = u64::try_from(time_since_last) else {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::InvalidObservationWindow),
            lineage,
        );
    };
    if time_since_last > policy.contact_loss_timeout_ticks {
        return ConformanceAssessment {
            status: ConformanceStatus::Contingent,
            reason: Some(ConformanceReason::ContactTimeout),
            definite_inside_ticks: definite_inside,
            definite_outside_ticks: definite_outside,
            possible_outside_ticks: possible_outside,
            unassessable_ticks: unassessable,
            unobserved_ticks: unobserved,
            qualified_observed_ticks: qualified_observed,
            definite_outside_ppm,
            possible_outside_ppm,
            lineage,
        };
    }

    if qualified_observed == 0 {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::NoQualifiedObservation),
            lineage,
        );
    }

    if possible_outside > 0 {
        return ConformanceAssessment {
            status: ConformanceStatus::PotentiallyNonconforming,
            reason: Some(if definite_outside > 0 {
                ConformanceReason::DefiniteDeviationBelowConfirmationThreshold
            } else {
                ConformanceReason::ObservedSetStraddlesDeviationBoundary
            }),
            definite_inside_ticks: definite_inside,
            definite_outside_ticks: definite_outside,
            possible_outside_ticks: possible_outside,
            unassessable_ticks: unassessable,
            unobserved_ticks: unobserved,
            qualified_observed_ticks: qualified_observed,
            definite_outside_ppm,
            possible_outside_ppm,
            lineage,
        };
    }

    if unobserved > 0 {
        return empty(
            ConformanceStatus::NotAssessable,
            Some(ConformanceReason::NoQualifiedObservation),
            lineage,
        );
    }

    ConformanceAssessment {
        status: ConformanceStatus::Conforming,
        reason: None,
        definite_inside_ticks: definite_inside,
        definite_outside_ticks: definite_outside,
        possible_outside_ticks: possible_outside,
        unassessable_ticks: unassessable,
        unobserved_ticks: unobserved,
        qualified_observed_ticks: qualified_observed,
        definite_outside_ppm,
        possible_outside_ppm,
        lineage,
    }
}

fn ratio_ppm(numerator: u64, denominator: u64) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let scaled = u128::from(numerator) * 1_000_000_u128 / u128::from(denominator);
    u32::try_from(scaled).unwrap_or(1_000_000)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

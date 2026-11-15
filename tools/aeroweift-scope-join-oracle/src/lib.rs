//! Aviation-specific evidence scope and point-in-time join contract.
//!
//! This crate checks only that already-qualified owner claims refer to the
//! requested operation, authorization revision, evaluation cutoff and
//! surveillance query. It does not authenticate receipts or re-qualify any
//! upstream physical, navigation, surveillance or authorization claim.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceOwner {
    Pnt,
    Reachability,
    Intent,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TrafficClass {
    CooperativeCrewed,
    NonCooperativeCrewed,
}

/// Exact operation and authority identity for one evaluation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OperationScope {
    pub evaluation_id: &'static str,
    pub operation_id: &'static str,
    pub flight_instance_id: &'static str,
    pub aircraft_instance_id: &'static str,
    pub intent_id: &'static str,
    pub authorization_id: &'static str,
    pub authorization_revision: u64,
}

/// Coordinate, time and profile identity shared by qualified evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EvidenceIdentity {
    pub profile_revision: &'static str,
    pub time_scale_id: &'static str,
    pub horizontal_frame_id: &'static str,
    pub vertical_datum_id: &'static str,
}

/// Half-open interval [start, end) in an already-normalized time scale.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TimeWindow {
    pub start: i64,
    pub end: i64,
}

impl TimeWindow {
    pub fn is_valid(self) -> bool {
        self.start < self.end
    }

    pub fn covers(self, required: Self) -> bool {
        self.is_valid()
            && required.is_valid()
            && self.start <= required.start
            && required.end <= self.end
    }

    pub fn contains_tick(self, tick: i64) -> bool {
        self.is_valid() && self.start <= tick && tick < self.end
    }
}

/// A time and identity envelope attached to one owner-produced operation claim.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OperationClaim {
    pub owner: EvidenceOwner,
    pub receipt_ref: Option<&'static str>,
    pub scope: OperationScope,
    pub identity: EvidenceIdentity,
    /// Interval over which this owner claim is applicable.
    pub validity_window: TimeWindow,
    /// Time the owner produced/evaluated this claim.
    pub evaluated_at: i64,
    /// Physical event/sample time, when this claim represents an observation.
    pub observed_at: Option<i64>,
    /// First time the composition service could know this claim.
    pub received_at: i64,
}

/// One required surveillance query for a cell, traffic class and time window.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CoverageRequirement {
    pub query_id: &'static str,
    pub airspace_id: &'static str,
    pub cell_id: &'static str,
    pub traffic_class: TrafficClass,
    pub window: TimeWindow,
}

/// A surveillance result bound to one exact query, not to one particular aircraft.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SurveillanceClaim {
    pub evaluation_id: &'static str,
    pub query_id: &'static str,
    pub airspace_id: &'static str,
    pub cell_id: &'static str,
    pub traffic_class: TrafficClass,
    /// Exact requested interval answered by the coverage assessment.
    pub query_window: TimeWindow,
    /// Interval for which the coverage claim is declared applicable.
    pub validity_window: TimeWindow,
    pub coverage_receipt_ref: Option<&'static str>,
    pub dependency_snapshot_ref: Option<&'static str>,
    /// None represents a qualified no-report result; Track M owns its semantics.
    pub observation_ref: Option<&'static str>,
    pub observed_at: Option<i64>,
    pub received_at: i64,
    pub identity: EvidenceIdentity,
}

/// Scope request for one cross-layer evidence composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeRequest {
    pub operation: OperationScope,
    pub identity: EvidenceIdentity,
    pub as_of_tick: i64,
    /// PNT support is scoped to the evaluation epoch, not the full flight horizon.
    pub pnt_support_window: TimeWindow,
    /// Reachability and intent claims must cover this requested operation interval.
    pub operation_window: TimeWindow,
    pub required_coverage: Vec<CoverageRequirement>,
}

/// Complete candidate input. The source snapshot is retained even on rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JoinInput {
    pub request: ScopeRequest,
    pub operation_claims: Vec<OperationClaim>,
    pub surveillance_claims: Vec<SurveillanceClaim>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeJoinDisposition {
    ScopeMatchedForComposition,
    NotAssessable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reason {
    IncompleteOperationScope,
    IncompleteEvidenceIdentity,
    InvalidPntSupportWindow,
    PntSupportWindowMissesEvaluation,
    InvalidOperationWindow,
    OperationWindowMissesEvaluation,
    MissingRequiredCoverage,
    InvalidCoverageRequirement,
    DuplicateCoverageRequirement,
    MissingOperationClaim(EvidenceOwner),
    DuplicateOperationClaim(EvidenceOwner),
    MissingOperationReceipt(EvidenceOwner),
    OperationScopeMismatch(EvidenceOwner),
    OperationIdentityMismatch(EvidenceOwner),
    InvalidOwnerValidityWindow(EvidenceOwner),
    OwnerValidityDoesNotCoverRequest(EvidenceOwner),
    OwnerEvaluatedAfterCutoff(EvidenceOwner),
    OwnerReceiptAfterCutoff(EvidenceOwner),
    OwnerEvaluationAfterReceipt(EvidenceOwner),
    OwnerEventAfterCutoff(EvidenceOwner),
    OwnerEventAfterReceipt(EvidenceOwner),
    MissingSurveillanceClaim(&'static str),
    DuplicateSurveillanceClaim(&'static str),
    UnexpectedSurveillanceClaim(&'static str),
    SurveillanceQueryMismatch(&'static str),
    InvalidSurveillanceValidity(&'static str),
    SurveillanceValidityDoesNotCoverQuery(&'static str),
    MissingCoverageReceipt(&'static str),
    MissingDependencySnapshot(&'static str),
    SurveillanceIdentityMismatch(&'static str),
    SurveillanceReceiptAfterCutoff(&'static str),
    SurveillanceObservationAfterCutoff(&'static str),
    SurveillanceObservationAfterReceipt(&'static str),
    ObservationTimeReferenceMismatch(&'static str),
}

/// Scope-match output. It is not a flight-clearance or safety result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeJoinResult {
    pub disposition: ScopeJoinDisposition,
    pub reason: Option<Reason>,
    /// Query IDs are emitted only on total success; failures expose no partial set.
    pub matched_query_ids: Vec<&'static str>,
    pub source_snapshot: JoinInput,
}

fn has_reference(value: Option<&str>) -> bool {
    value.is_some_and(|item| !item.trim().is_empty())
}

fn operation_scope_is_complete(scope: OperationScope) -> bool {
    !scope.evaluation_id.trim().is_empty()
        && !scope.operation_id.trim().is_empty()
        && !scope.flight_instance_id.trim().is_empty()
        && !scope.aircraft_instance_id.trim().is_empty()
        && !scope.intent_id.trim().is_empty()
        && !scope.authorization_id.trim().is_empty()
        && scope.authorization_revision > 0
}

fn identity_is_complete(identity: EvidenceIdentity) -> bool {
    !identity.profile_revision.trim().is_empty()
        && !identity.time_scale_id.trim().is_empty()
        && !identity.horizontal_frame_id.trim().is_empty()
        && !identity.vertical_datum_id.trim().is_empty()
}

fn canonical_snapshot(input: &JoinInput) -> JoinInput {
    let mut snapshot = input.clone();
    snapshot.request.required_coverage.sort_unstable();
    snapshot.operation_claims.sort_unstable();
    snapshot.surveillance_claims.sort_unstable();
    snapshot
}

fn blocked(input: &JoinInput, reason: Reason) -> ScopeJoinResult {
    ScopeJoinResult {
        disposition: ScopeJoinDisposition::NotAssessable,
        reason: Some(reason),
        matched_query_ids: Vec::new(),
        source_snapshot: canonical_snapshot(input),
    }
}

/// Check subject, authority, identity, validity and as-known-at scope only.
///
/// Freshness limits, clock conversion, receipt authentication, aircraft physics,
/// surveillance coverage truth and intent policy remain with their owner layers.
pub fn join_scope(input: &JoinInput) -> ScopeJoinResult {
    let request = &input.request;

    if !operation_scope_is_complete(request.operation) {
        return blocked(input, Reason::IncompleteOperationScope);
    }
    if !identity_is_complete(request.identity) {
        return blocked(input, Reason::IncompleteEvidenceIdentity);
    }
    if !request.pnt_support_window.is_valid() {
        return blocked(input, Reason::InvalidPntSupportWindow);
    }
    if !request.pnt_support_window.contains_tick(request.as_of_tick) {
        return blocked(input, Reason::PntSupportWindowMissesEvaluation);
    }
    if !request.operation_window.is_valid() {
        return blocked(input, Reason::InvalidOperationWindow);
    }
    if !request.operation_window.contains_tick(request.as_of_tick) {
        return blocked(input, Reason::OperationWindowMissesEvaluation);
    }
    if request.required_coverage.is_empty() {
        return blocked(input, Reason::MissingRequiredCoverage);
    }

    let mut requirements = request.required_coverage.clone();
    requirements.sort_unstable();
    for requirement in &requirements {
        if requirement.query_id.trim().is_empty()
            || requirement.airspace_id.trim().is_empty()
            || requirement.cell_id.trim().is_empty()
            || !requirement.window.is_valid()
            || !request.operation_window.covers(requirement.window)
        {
            return blocked(input, Reason::InvalidCoverageRequirement);
        }
    }
    for pair in requirements.windows(2) {
        if pair[0].query_id == pair[1].query_id {
            return blocked(input, Reason::DuplicateCoverageRequirement);
        }
    }

    let required_owners = [
        EvidenceOwner::Pnt,
        EvidenceOwner::Reachability,
        EvidenceOwner::Intent,
    ];
    for owner in required_owners {
        let mut matches = input
            .operation_claims
            .iter()
            .filter(|claim| claim.owner == owner);
        let Some(claim) = matches.next() else {
            return blocked(input, Reason::MissingOperationClaim(owner));
        };
        if matches.next().is_some() {
            return blocked(input, Reason::DuplicateOperationClaim(owner));
        }
        if !has_reference(claim.receipt_ref) {
            return blocked(input, Reason::MissingOperationReceipt(owner));
        }
        if claim.scope != request.operation {
            return blocked(input, Reason::OperationScopeMismatch(owner));
        }
        if !identity_is_complete(claim.identity) || claim.identity != request.identity {
            return blocked(input, Reason::OperationIdentityMismatch(owner));
        }
        if !claim.validity_window.is_valid() {
            return blocked(input, Reason::InvalidOwnerValidityWindow(owner));
        }
        let required_window = match owner {
            EvidenceOwner::Pnt => request.pnt_support_window,
            EvidenceOwner::Reachability | EvidenceOwner::Intent => request.operation_window,
        };
        if !claim.validity_window.covers(required_window) {
            return blocked(input, Reason::OwnerValidityDoesNotCoverRequest(owner));
        }
        if claim.evaluated_at > request.as_of_tick {
            return blocked(input, Reason::OwnerEvaluatedAfterCutoff(owner));
        }
        if claim.received_at > request.as_of_tick {
            return blocked(input, Reason::OwnerReceiptAfterCutoff(owner));
        }
        if claim.evaluated_at > claim.received_at {
            return blocked(input, Reason::OwnerEvaluationAfterReceipt(owner));
        }
        if let Some(event_tick) = claim.observed_at {
            if event_tick > request.as_of_tick {
                return blocked(input, Reason::OwnerEventAfterCutoff(owner));
            }
            if event_tick > claim.received_at {
                return blocked(input, Reason::OwnerEventAfterReceipt(owner));
            }
        }
    }

    if input.operation_claims.len() != required_owners.len() {
        return blocked(
            input,
            Reason::DuplicateOperationClaim(EvidenceOwner::Intent),
        );
    }

    if input.surveillance_claims.len() != requirements.len() {
        return blocked(input, Reason::MissingRequiredCoverage);
    }

    let mut matched_query_ids = Vec::with_capacity(requirements.len());
    for requirement in &requirements {
        let mut matches = input
            .surveillance_claims
            .iter()
            .filter(|claim| claim.query_id == requirement.query_id);
        let Some(claim) = matches.next() else {
            return blocked(input, Reason::MissingSurveillanceClaim(requirement.query_id));
        };
        if matches.next().is_some() {
            return blocked(input, Reason::DuplicateSurveillanceClaim(requirement.query_id));
        }
        if claim.evaluation_id != request.operation.evaluation_id
            || claim.airspace_id != requirement.airspace_id
            || claim.cell_id != requirement.cell_id
            || claim.traffic_class != requirement.traffic_class
            || claim.query_window != requirement.window
        {
            return blocked(input, Reason::SurveillanceQueryMismatch(requirement.query_id));
        }
        if !claim.validity_window.is_valid() {
            return blocked(input, Reason::InvalidSurveillanceValidity(requirement.query_id));
        }
        if !claim.validity_window.covers(requirement.window) {
            return blocked(
                input,
                Reason::SurveillanceValidityDoesNotCoverQuery(requirement.query_id),
            );
        }
        if !has_reference(claim.coverage_receipt_ref) {
            return blocked(input, Reason::MissingCoverageReceipt(requirement.query_id));
        }
        if !has_reference(claim.dependency_snapshot_ref) {
            return blocked(input, Reason::MissingDependencySnapshot(requirement.query_id));
        }
        if !identity_is_complete(claim.identity) || claim.identity != request.identity {
            return blocked(input, Reason::SurveillanceIdentityMismatch(requirement.query_id));
        }
        if claim.received_at > request.as_of_tick {
            return blocked(
                input,
                Reason::SurveillanceReceiptAfterCutoff(requirement.query_id),
            );
        }
        match (claim.observation_ref, claim.observed_at) {
            (Some(reference), Some(event_tick)) if !reference.trim().is_empty() => {
                if event_tick > request.as_of_tick {
                    return blocked(
                        input,
                        Reason::SurveillanceObservationAfterCutoff(requirement.query_id),
                    );
                }
                if event_tick > claim.received_at {
                    return blocked(
                        input,
                        Reason::SurveillanceObservationAfterReceipt(requirement.query_id),
                    );
                }
            }
            (None, None) => {}
            _ => {
                return blocked(
                    input,
                    Reason::ObservationTimeReferenceMismatch(requirement.query_id),
                );
            }
        }
        matched_query_ids.push(requirement.query_id);
    }

    let mut snapshot = canonical_snapshot(input);
    matched_query_ids.sort_unstable();
    ScopeJoinResult {
        disposition: ScopeJoinDisposition::ScopeMatchedForComposition,
        reason: None,
        matched_query_ids,
        source_snapshot: std::mem::replace(&mut snapshot, input.clone()),
    }
}

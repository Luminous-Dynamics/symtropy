//! Finite composition contract for already-qualified AeroWeft owner claims.
//!
//! This crate does not qualify upstream evidence. It preserves fail-closed,
//! non-compensatory composition semantics and never emits an operational clearance.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TrafficClass {
    CooperativeCrewed,
    NonCooperativeCrewed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvidenceIdentity {
    pub time_scale_id: &'static str,
    pub profile_id: &'static str,
    pub frame_id: &'static str,
    pub vertical_datum_id: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PntDisposition {
    AcceptedPrimaryHardBound,
    AcceptedQualifiedFallback,
    StatisticalOnly,
    Stale,
    Recovering,
    Indeterminate,
    ClockUnqualified,
    Contradictory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReachabilityDisposition {
    QualifiedBounded,
    NotAssessable,
    ArithmeticFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentDisposition {
    Conforming,
    PotentiallyNonconforming,
    Nonconforming,
    Contingent,
    ReauthorizationRequired,
    NotAssessable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurveillanceDisposition {
    QualifiedTrack,
    NoReportQualifiedCoverage,
    NoQualifiedCoverage,
    UnqualifiedTrack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurveillanceEvidence {
    pub traffic_class: TrafficClass,
    pub disposition: SurveillanceDisposition,
    pub coverage_qualification_ref: Option<&'static str>,
    pub observation_ref: Option<&'static str>,
    pub dependency_snapshot_ref: Option<&'static str>,
    pub independence_qualified: bool,
    pub identity: EvidenceIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionInput {
    pub expected_identity: EvidenceIdentity,
    pub pnt_disposition: PntDisposition,
    pub pnt_receipt_ref: Option<&'static str>,
    pub pnt_identity: EvidenceIdentity,
    pub reachability_disposition: ReachabilityDisposition,
    pub reachability_receipt_ref: Option<&'static str>,
    pub reachability_identity: EvidenceIdentity,
    pub intent_disposition: IntentDisposition,
    pub intent_receipt_ref: Option<&'static str>,
    pub intent_identity: EvidenceIdentity,
    pub required_traffic_classes: Vec<TrafficClass>,
    pub surveillance: Vec<SurveillanceEvidence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    ReadyForConflictScreen,
    NotAssessable,
    ReauthorizationRequired,
    Nonconforming,
    ContingencyHandoff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityOwner {
    Pnt,
    Reachability,
    Intent,
    Surveillance,
    Expected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reason {
    MissingPntReceipt,
    MissingReachabilityReceipt,
    MissingIntentReceipt,
    PntNotHardBound,
    ReachabilityNotQualified,
    IntentNotQualified,
    NoRequiredTrafficClasses,
    DuplicateRequiredTrafficClass(TrafficClass),
    IncompleteIdentity(IdentityOwner),
    IdentityMismatch(IdentityOwner),
    MissingTrafficClass(TrafficClass),
    DuplicateTrafficClass(TrafficClass),
    MissingSurveillanceReceipt(TrafficClass),
    IndependenceNotQualified(TrafficClass),
    SurveillanceNotQualified(TrafficClass),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OccupancyInterpretation {
    ObservedTrack,
    UnresolvedPossibleOccupancy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClassOccupancy {
    pub traffic_class: TrafficClass,
    pub interpretation: OccupancyInterpretation,
    pub coverage_qualification_ref: &'static str,
    pub observation_ref: Option<&'static str>,
    pub dependency_snapshot_ref: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionResult {
    pub decision: Decision,
    pub reason: Option<Reason>,
    pub class_occupancy: Vec<ClassOccupancy>,
    /// Exact owner-produced inputs retained even when composition fails.
    pub source_snapshot: CompositionInput,
}

fn has_reference(reference: Option<&str>) -> bool {
    reference.is_some_and(|value| !value.trim().is_empty())
}

fn blocked(input: &CompositionInput, decision: Decision, reason: Reason) -> CompositionResult {
    CompositionResult {
        decision,
        reason: Some(reason),
        class_occupancy: Vec::new(),
        source_snapshot: canonical_snapshot(input),
    }
}

fn not_assessable(input: &CompositionInput, reason: Reason) -> CompositionResult {
    blocked(input, Decision::NotAssessable, reason)
}

/// Retain all submitted evidence while making non-semantic list order deterministic.
fn canonical_snapshot(input: &CompositionInput) -> CompositionInput {
    let mut snapshot = input.clone();
    snapshot.required_traffic_classes.sort_unstable();
    snapshot.surveillance.sort_by_key(|evidence| evidence.traffic_class);
    snapshot
}

fn identity_is_complete(identity: EvidenceIdentity) -> bool {
    !identity.time_scale_id.trim().is_empty()
        && !identity.profile_id.trim().is_empty()
        && !identity.frame_id.trim().is_empty()
        && !identity.vertical_datum_id.trim().is_empty()
}

/// Compose qualified owner outputs without recalculating their domain logic.
///
/// A successful result means only that the supplied claims are eligible for the
/// existing conflict-screening owner. It is not a clearance or conflict-free result.
pub fn compose(input: &CompositionInput) -> CompositionResult {
    if !identity_is_complete(input.expected_identity) {
        return not_assessable(input, Reason::IncompleteIdentity(IdentityOwner::Expected));
    }

    if !has_reference(input.intent_receipt_ref) {
        return not_assessable(input, Reason::MissingIntentReceipt);
    }

    if !identity_is_complete(input.intent_identity) {
        return not_assessable(input, Reason::IncompleteIdentity(IdentityOwner::Intent));
    }
    if input.intent_identity != input.expected_identity {
        return not_assessable(input, Reason::IdentityMismatch(IdentityOwner::Intent));
    }

    match input.intent_disposition {
        IntentDisposition::ReauthorizationRequired => {
            return blocked(input, Decision::ReauthorizationRequired, Reason::IntentNotQualified);
        }
        IntentDisposition::Nonconforming => {
            return blocked(input, Decision::Nonconforming, Reason::IntentNotQualified);
        }
        IntentDisposition::Contingent => {
            return blocked(input, Decision::ContingencyHandoff, Reason::IntentNotQualified);
        }
        IntentDisposition::PotentiallyNonconforming | IntentDisposition::NotAssessable => {
            return not_assessable(input, Reason::IntentNotQualified);
        }
        IntentDisposition::Conforming => {}
    }

    if !has_reference(input.pnt_receipt_ref) {
        return not_assessable(input, Reason::MissingPntReceipt);
    }

    if !has_reference(input.reachability_receipt_ref) {
        return not_assessable(input, Reason::MissingReachabilityReceipt);
    }

    if !identity_is_complete(input.pnt_identity) {
        return not_assessable(input, Reason::IncompleteIdentity(IdentityOwner::Pnt));
    }
    if input.pnt_identity != input.expected_identity {
        return not_assessable(input, Reason::IdentityMismatch(IdentityOwner::Pnt));
    }

    if !identity_is_complete(input.reachability_identity) {
        return not_assessable(input, Reason::IncompleteIdentity(IdentityOwner::Reachability));
    }
    if input.reachability_identity != input.expected_identity {
        return not_assessable(input, Reason::IdentityMismatch(IdentityOwner::Reachability));
    }

    match input.pnt_disposition {
        PntDisposition::AcceptedPrimaryHardBound
        | PntDisposition::AcceptedQualifiedFallback => {}
        PntDisposition::StatisticalOnly
        | PntDisposition::Stale
        | PntDisposition::Recovering
        | PntDisposition::Indeterminate
        | PntDisposition::ClockUnqualified
        | PntDisposition::Contradictory => {
            return not_assessable(input, Reason::PntNotHardBound);
        }
    }

    match input.reachability_disposition {
        ReachabilityDisposition::QualifiedBounded => {}
        ReachabilityDisposition::NotAssessable | ReachabilityDisposition::ArithmeticFailure => {
            return not_assessable(input, Reason::ReachabilityNotQualified);
        }
    }

    if input.required_traffic_classes.is_empty() {
        return not_assessable(input, Reason::NoRequiredTrafficClasses);
    }

    let mut required_classes = input.required_traffic_classes.clone();
    required_classes.sort_unstable();
    for pair in required_classes.windows(2) {
        if pair[0] == pair[1] {
            return not_assessable(input, Reason::DuplicateRequiredTrafficClass(pair[0]));
        }
    }

    let mut class_occupancy = Vec::with_capacity(required_classes.len());

    for traffic_class in required_classes {
        let mut matches = input
            .surveillance
            .iter()
            .filter(|evidence| evidence.traffic_class == traffic_class);

        let Some(evidence) = matches.next() else {
            return not_assessable(input, Reason::MissingTrafficClass(traffic_class));
        };

        if matches.next().is_some() {
            return not_assessable(input, Reason::DuplicateTrafficClass(traffic_class));
        }

        if !identity_is_complete(evidence.identity) {
            return not_assessable(input, Reason::IncompleteIdentity(IdentityOwner::Surveillance));
        }
        if evidence.identity != input.expected_identity {
            return not_assessable(input, Reason::IdentityMismatch(IdentityOwner::Surveillance));
        }

        if !has_reference(evidence.coverage_qualification_ref)
            || !has_reference(evidence.dependency_snapshot_ref)
        {
            return not_assessable(input, Reason::MissingSurveillanceReceipt(traffic_class));
        }

        if !evidence.independence_qualified {
            return not_assessable(input, Reason::IndependenceNotQualified(traffic_class));
        }

        let interpretation = match evidence.disposition {
            SurveillanceDisposition::QualifiedTrack => {
                if !has_reference(evidence.observation_ref) {
                    return not_assessable(input, Reason::MissingSurveillanceReceipt(traffic_class));
                }
                OccupancyInterpretation::ObservedTrack
            }
            SurveillanceDisposition::NoReportQualifiedCoverage => {
                if evidence.observation_ref.is_some() {
                    return not_assessable(input, Reason::SurveillanceNotQualified(traffic_class));
                }
                OccupancyInterpretation::UnresolvedPossibleOccupancy
            }
            SurveillanceDisposition::NoQualifiedCoverage
            | SurveillanceDisposition::UnqualifiedTrack => {
                return not_assessable(input, Reason::SurveillanceNotQualified(traffic_class));
            }
        };

        class_occupancy.push(ClassOccupancy {
            traffic_class,
            interpretation,
            coverage_qualification_ref: evidence
                .coverage_qualification_ref
                .expect("reference presence was checked"),
            observation_ref: evidence.observation_ref,
            dependency_snapshot_ref: evidence
                .dependency_snapshot_ref
                .expect("reference presence was checked"),
        });
    }

    CompositionResult {
        decision: Decision::ReadyForConflictScreen,
        reason: None,
        class_occupancy,
        source_snapshot: canonical_snapshot(input),
    }
}

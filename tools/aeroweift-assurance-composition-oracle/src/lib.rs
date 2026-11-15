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
}

fn has_reference(reference: Option<&str>) -> bool {
    reference.is_some_and(|value| !value.trim().is_empty())
}

fn blocked(decision: Decision, reason: Reason) -> CompositionResult {
    CompositionResult {
        decision,
        reason: Some(reason),
        class_occupancy: Vec::new(),
    }
}

fn not_assessable(reason: Reason) -> CompositionResult {
    blocked(Decision::NotAssessable, reason)
}

/// Compose qualified owner outputs without recalculating their domain logic.
///
/// A successful result means only that the supplied claims are eligible for the
/// existing conflict-screening owner. It is not a clearance or conflict-free result.
pub fn compose(input: &CompositionInput) -> CompositionResult {
    if !has_reference(input.intent_receipt_ref) {
        return not_assessable(Reason::MissingIntentReceipt);
    }

    if input.intent_identity != input.expected_identity {
        return not_assessable(Reason::IdentityMismatch(IdentityOwner::Intent));
    }

    match input.intent_disposition {
        IntentDisposition::ReauthorizationRequired => {
            return blocked(Decision::ReauthorizationRequired, Reason::IntentNotQualified);
        }
        IntentDisposition::Nonconforming => {
            return blocked(Decision::Nonconforming, Reason::IntentNotQualified);
        }
        IntentDisposition::Contingent => {
            return blocked(Decision::ContingencyHandoff, Reason::IntentNotQualified);
        }
        IntentDisposition::PotentiallyNonconforming | IntentDisposition::NotAssessable => {
            return not_assessable(Reason::IntentNotQualified);
        }
        IntentDisposition::Conforming => {}
    }

    if !has_reference(input.pnt_receipt_ref) {
        return not_assessable(Reason::MissingPntReceipt);
    }

    if !has_reference(input.reachability_receipt_ref) {
        return not_assessable(Reason::MissingReachabilityReceipt);
    }

    if input.pnt_identity != input.expected_identity {
        return not_assessable(Reason::IdentityMismatch(IdentityOwner::Pnt));
    }

    if input.reachability_identity != input.expected_identity {
        return not_assessable(Reason::IdentityMismatch(IdentityOwner::Reachability));
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
            return not_assessable(Reason::PntNotHardBound);
        }
    }

    match input.reachability_disposition {
        ReachabilityDisposition::QualifiedBounded => {}
        ReachabilityDisposition::NotAssessable | ReachabilityDisposition::ArithmeticFailure => {
            return not_assessable(Reason::ReachabilityNotQualified);
        }
    }

    if input.required_traffic_classes.is_empty() {
        return not_assessable(Reason::NoRequiredTrafficClasses);
    }

    let mut required_classes = input.required_traffic_classes.clone();
    required_classes.sort_unstable();

    let mut class_occupancy = Vec::with_capacity(required_classes.len());

    for traffic_class in required_classes {
        let mut matches = input
            .surveillance
            .iter()
            .filter(|evidence| evidence.traffic_class == traffic_class);

        let Some(evidence) = matches.next() else {
            return not_assessable(Reason::MissingTrafficClass(traffic_class));
        };

        if matches.next().is_some() {
            return not_assessable(Reason::DuplicateTrafficClass(traffic_class));
        }

        if evidence.identity != input.expected_identity {
            return not_assessable(Reason::IdentityMismatch(IdentityOwner::Surveillance));
        }

        if !has_reference(evidence.coverage_qualification_ref)
            || !has_reference(evidence.dependency_snapshot_ref)
        {
            return not_assessable(Reason::MissingSurveillanceReceipt(traffic_class));
        }

        if !evidence.independence_qualified {
            return not_assessable(Reason::IndependenceNotQualified(traffic_class));
        }

        let interpretation = match evidence.disposition {
            SurveillanceDisposition::QualifiedTrack => {
                if !has_reference(evidence.observation_ref) {
                    return not_assessable(Reason::MissingSurveillanceReceipt(traffic_class));
                }
                OccupancyInterpretation::ObservedTrack
            }
            SurveillanceDisposition::NoReportQualifiedCoverage => {
                if evidence.observation_ref.is_some() {
                    return not_assessable(Reason::SurveillanceNotQualified(traffic_class));
                }
                OccupancyInterpretation::UnresolvedPossibleOccupancy
            }
            SurveillanceDisposition::NoQualifiedCoverage
            | SurveillanceDisposition::UnqualifiedTrack => {
                return not_assessable(Reason::SurveillanceNotQualified(traffic_class));
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
    }
}

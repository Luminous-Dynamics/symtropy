//! Partial event-order relation for hybrid mechanics.
//!
//! This module does not schedule or merge events. It compares two event-time
//! enclosures in a common time basis and preserves overlap as unresolved.
//! Numerical brackets are never promoted to proof-grade order.

/// Evidence grade carried by an event-time interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTimeEvidence {
    /// The interval is an independently justified enclosure of the event time.
    CertifiedEnclosure,
    /// The interval is a numerical estimate (for example, a floating-point
    /// bisection bracket) and is not a formal enclosure.
    NumericalEstimate,
}

/// One event's time enclosure and its identity.
#[derive(Debug, Clone, PartialEq)]
pub struct EventTimeRecord {
    pub event_id: String,
    /// Identifier for the common physical time coordinate/basis.
    pub time_basis_id: String,
    /// Stable reference to the external interval-evidence receipt. This module
    /// requires traceability but does not authenticate or verify the receipt.
    pub evidence_reference: String,
    pub lower_time: f64,
    pub upper_time: f64,
    pub evidence: EventTimeEvidence,
}

/// Pairwise relation between two event times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOrderRelation {
    /// Certified disjoint enclosures prove event A occurs before event B.
    ProvenBefore,
    /// Certified disjoint enclosures prove event A occurs after event B.
    ProvenAfter,
    /// Certified enclosures overlap or touch; no order is established.
    UnresolvedOverlap,
    /// Numerical-only intervals are disjoint and suggest A before B.
    EstimatedBefore,
    /// Numerical-only intervals are disjoint and suggest A after B.
    EstimatedAfter,
    /// Numerical-only intervals overlap or touch; no order is established.
    EstimatedOverlap,
}

/// Invalid event identity, time basis or interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOrderError {
    EmptyEventIdentity,
    EmptyTimeBasisIdentity,
    EmptyEvidenceReference,
    SameEventComparedWithItself,
    IncompatibleTimeBases,
    NonFiniteTimeBound,
    NegativeTimeBound,
    ReversedTimeInterval,
}

/// Validate one event-time record before comparing it with another.
fn validate_record(record: &EventTimeRecord) -> Result<(), EventOrderError> {
    if record.event_id.trim().is_empty() {
        return Err(EventOrderError::EmptyEventIdentity);
    }
    if record.time_basis_id.trim().is_empty() {
        return Err(EventOrderError::EmptyTimeBasisIdentity);
    }
    if record.evidence_reference.trim().is_empty() {
        return Err(EventOrderError::EmptyEvidenceReference);
    }
    if !record.lower_time.is_finite() || !record.upper_time.is_finite() {
        return Err(EventOrderError::NonFiniteTimeBound);
    }
    if record.lower_time < 0.0 || record.upper_time < 0.0 {
        return Err(EventOrderError::NegativeTimeBound);
    }
    if record.lower_time > record.upper_time {
        return Err(EventOrderError::ReversedTimeInterval);
    }
    Ok(())
}

/// Compare event A with event B without inventing a total order.
///
/// The caller must authenticate/verify any receipt referenced by a
/// CertifiedEnclosure record before calling this function. This module checks
/// receipt-reference presence for traceability but does not authenticate it.
///
/// Strict disjointness is required for any order relation:
///
/// - A before B only when A.upper_time < B.lower_time.
/// - A after B only when B.upper_time < A.lower_time.
///
/// Touching endpoints and all overlapping enclosures remain unresolved.
/// Even identical certified singleton intervals produce
/// UnresolvedOverlap; proof of simultaneity requires an independent joint
/// event/contact certificate and is intentionally outside this pairwise API.
///
/// If either input is only a numerical estimate, disjointness yields an
/// estimated relation, never a proof-grade relation.
pub fn compare_event_order(
    event_a: &EventTimeRecord,
    event_b: &EventTimeRecord,
) -> Result<EventOrderRelation, EventOrderError> {
    validate_record(event_a)?;
    validate_record(event_b)?;

    if event_a.event_id == event_b.event_id {
        return Err(EventOrderError::SameEventComparedWithItself);
    }
    if event_a.time_basis_id != event_b.time_basis_id {
        return Err(EventOrderError::IncompatibleTimeBases);
    }

    let proof_grade = event_a.evidence == EventTimeEvidence::CertifiedEnclosure
        && event_b.evidence == EventTimeEvidence::CertifiedEnclosure;

    if event_a.upper_time < event_b.lower_time {
        return Ok(if proof_grade {
            EventOrderRelation::ProvenBefore
        } else {
            EventOrderRelation::EstimatedBefore
        });
    }
    if event_b.upper_time < event_a.lower_time {
        return Ok(if proof_grade {
            EventOrderRelation::ProvenAfter
        } else {
            EventOrderRelation::EstimatedAfter
        });
    }

    Ok(if proof_grade {
        EventOrderRelation::UnresolvedOverlap
    } else {
        EventOrderRelation::EstimatedOverlap
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        id: &str,
        lower_time: f64,
        upper_time: f64,
        evidence: EventTimeEvidence,
    ) -> EventTimeRecord {
        EventTimeRecord {
            event_id: id.into(),
            time_basis_id: "simulation-seconds-v1".into(),
            evidence_reference: format!("fixture-receipt-{id}"),
            lower_time,
            upper_time,
            evidence,
        }
    }

    #[test]
    fn disjoint_certified_intervals_prove_order_in_both_directions() {
        let a = event("impact-a", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        let b = event("impact-b", 1.2, 1.3, EventTimeEvidence::CertifiedEnclosure);

        assert_eq!(
            compare_event_order(&a, &b),
            Ok(EventOrderRelation::ProvenBefore)
        );
        assert_eq!(
            compare_event_order(&b, &a),
            Ok(EventOrderRelation::ProvenAfter)
        );
    }

    #[test]
    fn overlapping_certified_intervals_with_equal_midpoints_remain_unresolved() {
        let a = event("impact-a", 1.0, 1.4, EventTimeEvidence::CertifiedEnclosure);
        let b = event("impact-b", 1.1, 1.3, EventTimeEvidence::CertifiedEnclosure);

        assert_eq!(
            compare_event_order(&a, &b),
            Ok(EventOrderRelation::UnresolvedOverlap)
        );
    }

    #[test]
    fn touching_certified_endpoints_are_not_a_proven_order() {
        let a = event("impact-a", 1.0, 1.2, EventTimeEvidence::CertifiedEnclosure);
        let b = event("impact-b", 1.2, 1.4, EventTimeEvidence::CertifiedEnclosure);

        assert_eq!(
            compare_event_order(&a, &b),
            Ok(EventOrderRelation::UnresolvedOverlap)
        );
    }

    #[test]
    fn identical_certified_singletons_do_not_prove_simultaneity() {
        let a = event("impact-a", 1.25, 1.25, EventTimeEvidence::CertifiedEnclosure);
        let b = event("impact-b", 1.25, 1.25, EventTimeEvidence::CertifiedEnclosure);

        assert_eq!(
            compare_event_order(&a, &b),
            Ok(EventOrderRelation::UnresolvedOverlap)
        );
    }

    #[test]
    fn numerical_brackets_never_produce_proof_grade_order() {
        let a = event("impact-a", 1.0, 1.1, EventTimeEvidence::NumericalEstimate);
        let b = event("impact-b", 1.2, 1.3, EventTimeEvidence::CertifiedEnclosure);
        let overlap = event("impact-c", 1.05, 1.25, EventTimeEvidence::NumericalEstimate);

        assert_eq!(
            compare_event_order(&a, &b),
            Ok(EventOrderRelation::EstimatedBefore)
        );
        assert_eq!(
            compare_event_order(&a, &overlap),
            Ok(EventOrderRelation::EstimatedOverlap)
        );
    }

    #[test]
    fn positive_time_unit_scaling_preserves_event_order() {
        let a = event("impact-a", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        let b = event("impact-b", 1.2, 1.3, EventTimeEvidence::CertifiedEnclosure);
        let scale = 1000.0;
        let mut a_milliseconds = a.clone();
        let mut b_milliseconds = b.clone();
        a_milliseconds.time_basis_id = "simulation-milliseconds-v1".into();
        b_milliseconds.time_basis_id = "simulation-milliseconds-v1".into();
        a_milliseconds.lower_time *= scale;
        a_milliseconds.upper_time *= scale;
        b_milliseconds.lower_time *= scale;
        b_milliseconds.upper_time *= scale;

        assert_eq!(
            compare_event_order(&a, &b),
            compare_event_order(&a_milliseconds, &b_milliseconds)
        );
    }

    #[test]
    fn rejects_different_time_bases() {
        let a = event("impact-a", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        let mut b = event("impact-b", 1.2, 1.3, EventTimeEvidence::CertifiedEnclosure);
        b.time_basis_id = "wall-clock-seconds-v1".into();

        assert_eq!(
            compare_event_order(&a, &b),
            Err(EventOrderError::IncompatibleTimeBases)
        );
    }

    #[test]
    fn rejects_invalid_event_intervals_and_identities() {
        let valid = event("impact-a", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        let empty_id = event("", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        let negative = event("negative", -0.1, 0.1, EventTimeEvidence::CertifiedEnclosure);
        let reversed = event("reversed", 2.0, 1.0, EventTimeEvidence::CertifiedEnclosure);
        let non_finite = event(
            "non-finite",
            1.0,
            f64::INFINITY,
            EventTimeEvidence::CertifiedEnclosure,
        );

        assert_eq!(
            compare_event_order(&empty_id, &valid),
            Err(EventOrderError::EmptyEventIdentity)
        );
        assert_eq!(
            compare_event_order(&negative, &valid),
            Err(EventOrderError::NegativeTimeBound)
        );
        assert_eq!(
            compare_event_order(&reversed, &valid),
            Err(EventOrderError::ReversedTimeInterval)
        );
        assert_eq!(
            compare_event_order(&non_finite, &valid),
            Err(EventOrderError::NonFiniteTimeBound)
        );
    }

    #[test]
    fn rejects_missing_interval_evidence_reference() {
        let mut a = event("impact-a", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        let b = event("impact-b", 1.2, 1.3, EventTimeEvidence::CertifiedEnclosure);
        a.evidence_reference.clear();

        assert_eq!(
            compare_event_order(&a, &b),
            Err(EventOrderError::EmptyEvidenceReference)
        );
    }

    #[test]
    fn rejects_comparing_an_event_with_itself() {
        let a = event("impact-a", 1.0, 1.1, EventTimeEvidence::CertifiedEnclosure);
        assert_eq!(
            compare_event_order(&a, &a),
            Err(EventOrderError::SameEventComparedWithItself)
        );
    }
}

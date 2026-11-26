//! Conservative historical replay when source clocks are uncertain.
//!
//! All intervals must already be expressed on the same declared time scale
//! (for example UTC epoch milliseconds) by a rights-aware upstream adapter.
//! This module does not infer timezone, clock source, synchronization quality
//! or the honesty of the supplied bounds.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeInterval {
    earliest: i64,
    latest: i64,
}

impl TimeInterval {
    pub fn new(earliest: i64, latest: i64) -> Result<Self, ClockError> {
        if earliest > latest {
            return Err(ClockError::InvertedInterval);
        }
        Ok(Self { earliest, latest })
    }

    pub fn point(timestamp: i64) -> Self {
        Self {
            earliest: timestamp,
            latest: timestamp,
        }
    }

    pub fn earliest(self) -> i64 {
        self.earliest
    }

    pub fn latest(self) -> i64 {
        self.latest
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderingResult {
    DefinitelyAtOrBefore,
    DefinitelyAfter,
    OverlapUncertain,
}

/// Conservative closed-interval comparison for the proposition left <= right.
///
/// Equality at a shared boundary is definite: left.latest == right.earliest
/// satisfies the <= relation. If intervals overlap without that proof, the
/// ordering is uncertain rather than guessed.
pub fn compare_at_or_before(left: TimeInterval, right: TimeInterval) -> OrderingResult {
    if left.latest <= right.earliest {
        OrderingResult::DefinitelyAtOrBefore
    } else if left.earliest > right.latest {
        OrderingResult::DefinitelyAfter
    } else {
        OrderingResult::OverlapUncertain
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockAdmission {
    DefinitelyAdmissible,
    DefinitelyInadmissible,
    TemporallyUncertain,
}

impl ClockAdmission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefinitelyAdmissible => "DEFINITELY_ADMISSIBLE",
            Self::DefinitelyInadmissible => "DEFINITELY_INADMISSIBLE",
            Self::TemporallyUncertain => "TEMPORALLY_UNCERTAIN",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockEvidence {
    pub valid_from: TimeInterval,
    pub valid_until: TimeInterval,
    pub published_at: TimeInterval,
    pub retrieved_at: TimeInterval,
    pub rights_clear: bool,
    pub rights_valid_from: TimeInterval,
    pub rights_valid_until: TimeInterval,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockCutoffs {
    pub physical_at: TimeInterval,
    pub decision_at: TimeInterval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockQualification {
    pub admission: ClockAdmission,
    pub reasons: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockError {
    InvertedInterval,
}

impl std::fmt::Display for ClockError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ClockError {}

fn record_lte(
    left: TimeInterval,
    right: TimeInterval,
    inadmissible_reason: &'static str,
    uncertain_reason: &'static str,
    reasons: &mut Vec<&'static str>,
    uncertain: &mut bool,
    inadmissible: &mut bool,
) {
    match compare_at_or_before(left, right) {
        OrderingResult::DefinitelyAtOrBefore => {}
        OrderingResult::DefinitelyAfter => {
            reasons.push(inadmissible_reason);
            *inadmissible = true;
        }
        OrderingResult::OverlapUncertain => {
            reasons.push(uncertain_reason);
            *uncertain = true;
        }
    }
}

/// Qualifies physical validity, receiver publication/retrieval, and rights
/// effectiveness without inventing a total order for overlapping clock bounds.
///
/// A policy-facing selector must admit only DefinitelyAdmissible. Uncertainty
/// is an abstention/query/reconciliation condition; definite violations reject.
pub fn qualify_clock_bounds(
    evidence: &ClockEvidence,
    cutoffs: ClockCutoffs,
) -> ClockQualification {
    let mut reasons = Vec::new();
    let mut uncertain = false;
    let mut inadmissible = false;

    // Physical valid-time interval contains the physical target.
    record_lte(
        evidence.valid_from,
        cutoffs.physical_at,
        "PHYSICAL_BEFORE_VALID_FROM",
        "PHYSICAL_VALID_FROM_ORDER_UNCERTAIN",
        &mut reasons,
        &mut uncertain,
        &mut inadmissible,
    );
    record_lte(
        cutoffs.physical_at,
        evidence.valid_until,
        "PHYSICAL_AFTER_VALID_UNTIL",
        "PHYSICAL_VALID_UNTIL_ORDER_UNCERTAIN",
        &mut reasons,
        &mut uncertain,
        &mut inadmissible,
    );

    // Receiver knowledge: publication and actual retrieval must both precede
    // the decision cutoff, with definite ordering from the supplied bounds.
    record_lte(
        evidence.published_at,
        cutoffs.decision_at,
        "PUBLISHED_AFTER_DECISION",
        "PUBLICATION_ORDER_UNCERTAIN",
        &mut reasons,
        &mut uncertain,
        &mut inadmissible,
    );
    record_lte(
        evidence.retrieved_at,
        cutoffs.decision_at,
        "RETRIEVED_AFTER_DECISION",
        "RETRIEVAL_ORDER_UNCERTAIN",
        &mut reasons,
        &mut uncertain,
        &mut inadmissible,
    );

    if !evidence.rights_clear {
        reasons.push("SOURCE_RIGHTS_DENIED");
        inadmissible = true;
    }

    record_lte(
        evidence.rights_valid_from,
        cutoffs.decision_at,
        "RIGHTS_NOT_YET_EFFECTIVE",
        "RIGHTS_START_ORDER_UNCERTAIN",
        &mut reasons,
        &mut uncertain,
        &mut inadmissible,
    );
    record_lte(
        cutoffs.decision_at,
        evidence.rights_valid_until,
        "RIGHTS_EXPIRED",
        "RIGHTS_END_ORDER_UNCERTAIN",
        &mut reasons,
        &mut uncertain,
        &mut inadmissible,
    );

    let admission = if inadmissible {
        ClockAdmission::DefinitelyInadmissible
    } else if uncertain {
        ClockAdmission::TemporallyUncertain
    } else {
        ClockAdmission::DefinitelyAdmissible
    };

    if reasons.is_empty() {
        reasons.push("ALL_CLOCK_BOUNDS_ESTABLISHED");
    }

    ClockQualification { admission, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(lower: i64, upper: i64) -> TimeInterval {
        TimeInterval::new(lower, upper).unwrap()
    }

    fn base_evidence() -> ClockEvidence {
        ClockEvidence {
            valid_from: interval(80, 80),
            valid_until: interval(200, 200),
            published_at: interval(90, 90),
            retrieved_at: interval(95, 95),
            rights_clear: true,
            rights_valid_from: interval(0, 0),
            rights_valid_until: interval(500, 500),
        }
    }

    fn cutoffs(physical: i64, decision: i64) -> ClockCutoffs {
        ClockCutoffs {
            physical_at: TimeInterval::point(physical),
            decision_at: TimeInterval::point(decision),
        }
    }

    #[test]
    fn exact_boundary_equality_is_inclusive_and_admissible() {
        let mut evidence = base_evidence();
        evidence.valid_from = TimeInterval::point(100);
        evidence.published_at = TimeInterval::point(100);
        evidence.retrieved_at = TimeInterval::point(100);
        evidence.rights_valid_from = TimeInterval::point(100);

        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::DefinitelyAdmissible);
        assert_eq!(result.reasons, vec!["ALL_CLOCK_BOUNDS_ESTABLISHED"]);
    }

    #[test]
    fn overlapping_publication_and_decision_bounds_are_uncertain() {
        let mut evidence = base_evidence();
        evidence.published_at = interval(98, 102);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::TemporallyUncertain);
        assert_eq!(result.reasons, vec!["PUBLICATION_ORDER_UNCERTAIN"]);
    }

    #[test]
    fn publication_definitely_after_cutoff_is_inadmissible() {
        let mut evidence = base_evidence();
        evidence.published_at = interval(101, 103);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::DefinitelyInadmissible);
        assert_eq!(result.reasons, vec!["PUBLISHED_AFTER_DECISION"]);
    }

    #[test]
    fn receiver_retrieval_uncertainty_is_not_hidden_by_early_publication() {
        let mut evidence = base_evidence();
        evidence.retrieved_at = interval(98, 102);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::TemporallyUncertain);
        assert_eq!(result.reasons, vec!["RETRIEVAL_ORDER_UNCERTAIN"]);
    }

    #[test]
    fn rights_not_yet_effective_is_rejected() {
        let mut evidence = base_evidence();
        evidence.rights_valid_from = interval(101, 103);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::DefinitelyInadmissible);
        assert_eq!(result.reasons, vec!["RIGHTS_NOT_YET_EFFECTIVE"]);
    }

    #[test]
    fn rights_expiry_overlap_is_uncertain() {
        let mut evidence = base_evidence();
        evidence.rights_valid_until = interval(98, 102);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::TemporallyUncertain);
        assert_eq!(result.reasons, vec!["RIGHTS_END_ORDER_UNCERTAIN"]);
    }

    #[test]
    fn physical_validity_overlap_is_uncertain() {
        let mut evidence = base_evidence();
        evidence.valid_from = interval(98, 102);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::TemporallyUncertain);
        assert_eq!(
            result.reasons,
            vec!["PHYSICAL_VALID_FROM_ORDER_UNCERTAIN"]
        );
    }

    #[test]
    fn physical_target_definitely_after_valid_until_is_rejected() {
        let mut evidence = base_evidence();
        evidence.valid_until = interval(80, 90);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::DefinitelyInadmissible);
        assert_eq!(result.reasons, vec!["PHYSICAL_AFTER_VALID_UNTIL"]);
    }

    #[test]
    fn a_definite_violation_dominates_uncertainty_and_preserves_both_reasons() {
        let mut evidence = base_evidence();
        evidence.published_at = interval(98, 102);
        evidence.retrieved_at = interval(105, 110);
        let result = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(result.admission, ClockAdmission::DefinitelyInadmissible);
        assert_eq!(
            result.reasons,
            vec![
                "PUBLICATION_ORDER_UNCERTAIN",
                "RETRIEVED_AFTER_DECISION"
            ]
        );
    }

    #[test]
    fn widening_publication_uncertainty_cannot_upgrade_eligibility() {
        let mut evidence = base_evidence();
        let exact = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(exact.admission, ClockAdmission::DefinitelyAdmissible);

        evidence.published_at = interval(90, 101);
        let widened = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(widened.admission, ClockAdmission::TemporallyUncertain);

        evidence.published_at = interval(95, 105);
        let wider = qualify_clock_bounds(&evidence, cutoffs(100, 100));
        assert_eq!(wider.admission, ClockAdmission::TemporallyUncertain);
    }

    #[test]
    fn inverted_interval_is_rejected_at_construction() {
        assert_eq!(
            TimeInterval::new(101, 100),
            Err(ClockError::InvertedInterval)
        );
    }

    #[test]
    fn distinct_overlapping_source_times_are_not_totally_ordered() {
        assert_eq!(
            compare_at_or_before(interval(90, 110), interval(100, 120)),
            OrderingResult::OverlapUncertain
        );
        assert_eq!(
            compare_at_or_before(interval(90, 100), interval(100, 120)),
            OrderingResult::DefinitelyAtOrBefore
        );
    }
}

//! Conservative Pareto ranking for interval-valued furnishing objectives.
//!
//! This API deliberately remains separate from the point-valued frontier. It
//! reports only dominance guaranteed by the declared intervals. Overlapping
//! intervals are incomparable under this relation; that does not imply equal
//! preference, physical safety, or equivalence in the real world.

use crate::alternative_frontier::{ObjectiveDirection, ObjectiveEvidence, VerificationStatus};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntervalSemanticsKind {
    /// The interval is declared as an enclosure for all admissible values.
    DeterministicEnvelope,
    /// A vendor or source states this range; this does not authenticate it.
    DeclaredRange,
    /// Statistical interval; not a deterministic guarantee outside its model.
    StatisticalInterval,
    /// Interval from a named empirical quantile procedure.
    EmpiricalQuantile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntervalSemantics {
    /// How the interval should be interpreted, not who published it.
    pub kind: IntervalSemanticsKind,
    /// Stable ID for the method / level / convention applied to all compared values.
    /// Examples: "exact-envelope-v1", "bootstrap-95pct-v2", "p05-p95-v1".
    pub basis_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalObjectiveObservation {
    /// Stable objective name.
    pub dimension: String,
    /// Explicit unit or normalized scale; no implicit conversion is performed.
    pub unit: String,
    /// Minimize or maximize this objective.
    pub direction: ObjectiveDirection,
    /// Lower endpoint in the declared unit.
    pub lower: f64,
    /// Upper endpoint in the declared unit.
    pub upper: f64,
    /// Explicit uncertainty interpretation and basis.
    pub semantics: IntervalSemantics,
    /// Caller policy's admission state for this observation.
    pub evidence: ObjectiveEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalAlternativeCandidate {
    pub candidate_id: String,
    pub revision_sha256: Option<String>,
    pub verifier_receipt_sha256: Option<String>,
    /// Exact revision digest claimed by the caller-supplied verifier receipt.
    pub verified_revision_sha256: Option<String>,
    /// Status from the independent hard-constraint verifier.
    pub verification: VerificationStatus,
    pub objectives: Vec<IntervalObjectiveObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntervalCandidateDisposition {
    /// No eligible alternative is guaranteed to dominate this candidate.
    OnFrontier,
    /// Stable witness ID whose intervals guaranteed-dominate this candidate.
    GuaranteedDominatedBy(String),
    ExcludedBlocked,
    ExcludedIndeterminate,
    ExcludedUnrankable(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalCandidateResult {
    pub candidate_id: String,
    pub revision_sha256: Option<String>,
    pub verifier_receipt_sha256: Option<String>,
    pub verified_revision_sha256: Option<String>,
    pub disposition: IntervalCandidateDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalFrontierReport {
    /// Guaranteed non-dominated IDs, sorted in stable order.
    pub frontier_ids: Vec<String>,
    /// Every input candidate, sorted by ID with a disposition.
    pub candidates: Vec<IntervalCandidateResult>,
    /// False when no candidate can be ranked or a verified candidate was unrankable.
    pub ranking_complete: bool,
    pub relation_name: &'static str,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntervalFrontierError {
    EmptyRequest,
    InvalidCandidateId { index: usize },
    DuplicateCandidateId(String),
    IncompatibleObjectiveSchemas { candidate_id: String },
}

impl fmt::Display for IntervalFrontierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRequest => write!(f, "interval frontier request contains no candidates"),
            Self::InvalidCandidateId { index } => {
                write!(f, "candidate at index {index} has an invalid identifier")
            }
            Self::DuplicateCandidateId(id) => {
                write!(f, "candidate identifier {id:?} occurs more than once")
            }
            Self::IncompatibleObjectiveSchemas { candidate_id } => {
                write!(
                    f,
                    "candidate {candidate_id:?} has an interval objective schema incompatible with the ranking set"
                )
            }
        }
    }
}

impl std::error::Error for IntervalFrontierError {}

#[derive(Clone, Debug)]
struct RankableIntervalCandidate<'a> {
    candidate: &'a IntervalAlternativeCandidate,
    objectives: Vec<&'a IntervalObjectiveObservation>,
}

impl RankableIntervalCandidate<'_> {
    fn schema(&self) -> Vec<(&str, &str, ObjectiveDirection, &IntervalSemantics)> {
        self.objectives
            .iter()
            .map(|objective| {
                (
                    objective.dimension.as_str(),
                    objective.unit.as_str(),
                    objective.direction,
                    &objective.semantics,
                )
            })
            .collect()
    }
}

fn valid_sha256(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn unrankable_reasons(candidate: &IntervalAlternativeCandidate) -> Vec<String> {
    let mut reasons = Vec::new();
    if !valid_sha256(candidate.revision_sha256.as_deref()) {
        reasons.push("missing or malformed candidate revision SHA-256".to_owned());
    }
    if !valid_sha256(candidate.verifier_receipt_sha256.as_deref()) {
        reasons.push("missing or malformed verifier receipt SHA-256".to_owned());
    }
    if !valid_sha256(candidate.verified_revision_sha256.as_deref()) {
        reasons.push("missing or malformed receipt-bound revision SHA-256".to_owned());
    } else if candidate.verified_revision_sha256 != candidate.revision_sha256 {
        reasons.push("verifier receipt is bound to a different candidate revision".to_owned());
    }
    if candidate.objectives.is_empty() {
        reasons.push("no objective observations supplied".to_owned());
        return reasons;
    }

    let mut seen = BTreeSet::new();
    for objective in &candidate.objectives {
        if objective.dimension.trim().is_empty()
            || objective.dimension != objective.dimension.trim()
        {
            reasons.push("objective dimension is blank or has surrounding whitespace".to_owned());
        } else if !seen.insert(objective.dimension.as_str()) {
            reasons.push(format!(
                "duplicate objective dimension {:?}",
                objective.dimension
            ));
        }
        if objective.unit.trim().is_empty() || objective.unit != objective.unit.trim() {
            reasons.push(format!(
                "objective {:?} has a blank unit or surrounding whitespace",
                objective.dimension
            ));
        }
        if objective.semantics.basis_id.trim().is_empty()
            || objective.semantics.basis_id != objective.semantics.basis_id.trim()
        {
            reasons.push(format!(
                "objective {:?} has a blank or malformed uncertainty basis ID",
                objective.dimension
            ));
        }
        if objective.semantics.kind != IntervalSemanticsKind::DeterministicEnvelope {
            reasons.push(format!(
                "objective {:?} is not a deterministic envelope; this API does not claim guaranteed dominance for statistical or declared ranges",
                objective.dimension
            ));
        }
        if !objective.lower.is_finite() || !objective.upper.is_finite() {
            reasons.push(format!(
                "objective {:?} interval endpoints must be finite",
                objective.dimension
            ));
        } else if objective.lower > objective.upper {
            reasons.push(format!(
                "objective {:?} interval lower bound exceeds its upper bound",
                objective.dimension
            ));
        }
        if objective.evidence != ObjectiveEvidence::AcceptedByPolicy {
            reasons.push(format!(
                "objective {:?} has non-admitted evidence state {:?}",
                objective.dimension, objective.evidence
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

/// True only when the left interval is strictly separated from the right
/// interval in the preferred direction for at least one objective, while no
/// objective has an overlapping or worse interval.
///
/// This relation is intentionally conservative:
/// - minimizing: left.upper <= right.lower;
/// - maximizing: left.lower >= right.upper;
/// - at least one of those inequalities must be strict.
///
/// Equal/touching endpoints can satisfy non-strict dimensions, but a candidate
/// must be strictly better on at least one dimension. This only establishes a
/// relation over the caller-supplied intervals; interval semantics determine
/// what those intervals mean in the real world.
fn guaranteed_interval_dominates(
    left: &RankableIntervalCandidate<'_>,
    right: &RankableIntervalCandidate<'_>,
) -> bool {
    let mut strictly_better = false;
    for (left_value, right_value) in left.objectives.iter().zip(&right.objectives) {
        match left_value.direction {
            ObjectiveDirection::Minimize => {
                if left_value.upper > right_value.lower {
                    return false;
                }
                strictly_better |= left_value.upper < right_value.lower;
            }
            ObjectiveDirection::Maximize => {
                if left_value.lower < right_value.upper {
                    return false;
                }
                strictly_better |= left_value.lower > right_value.upper;
            }
        }
    }
    strictly_better
}

/// Compute a conservative guaranteed-interval Pareto frontier.
///
/// Only candidates labelled Verified by the caller's independent verifier can
/// enter the comparison. Blocked and Indeterminate candidates never enter it.
/// A verified candidate with non-admitted objective evidence or malformed
/// intervals is excluded and makes the report incomplete.
///
/// Rankable candidates must share objective dimensions, units/scales, directions,
/// and interval-semantic keys exactly. No unit conversion, midpoint imputation,
/// interval widening, or hidden utility weight is performed.
///
/// This function validates SHA-256 syntax and equality of the receipt-bound
/// revision field only. It does not authenticate the receipt or recompute any
/// digest. Similarly, the relation is guaranteed only relative to the supplied
/// interval values and declared semantics—not a guarantee of real-world truth.
pub fn compute_interval_frontier(
    candidates: &[IntervalAlternativeCandidate],
) -> Result<IntervalFrontierReport, IntervalFrontierError> {
    if candidates.is_empty() {
        return Err(IntervalFrontierError::EmptyRequest);
    }

    let mut seen = BTreeSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.candidate_id.trim().is_empty()
            || candidate.candidate_id != candidate.candidate_id.trim()
        {
            return Err(IntervalFrontierError::InvalidCandidateId { index });
        }
        if !seen.insert(candidate.candidate_id.as_str()) {
            return Err(IntervalFrontierError::DuplicateCandidateId(
                candidate.candidate_id.clone(),
            ));
        }
    }

    let mut results = Vec::with_capacity(candidates.len());
    let mut rankable = Vec::new();
    let mut ranking_complete = true;

    for candidate in candidates {
        match candidate.verification {
            VerificationStatus::Blocked => results.push(IntervalCandidateResult {
                candidate_id: candidate.candidate_id.clone(),
                revision_sha256: candidate.revision_sha256.clone(),
                verifier_receipt_sha256: candidate.verifier_receipt_sha256.clone(),
                verified_revision_sha256: candidate.verified_revision_sha256.clone(),
                disposition: IntervalCandidateDisposition::ExcludedBlocked,
            }),
            VerificationStatus::Indeterminate => results.push(IntervalCandidateResult {
                candidate_id: candidate.candidate_id.clone(),
                revision_sha256: candidate.revision_sha256.clone(),
                verifier_receipt_sha256: candidate.verifier_receipt_sha256.clone(),
                verified_revision_sha256: candidate.verified_revision_sha256.clone(),
                disposition: IntervalCandidateDisposition::ExcludedIndeterminate,
            }),
            VerificationStatus::Verified => {
                let reasons = unrankable_reasons(candidate);
                if !reasons.is_empty() {
                    ranking_complete = false;
                    results.push(IntervalCandidateResult {
                        candidate_id: candidate.candidate_id.clone(),
                        revision_sha256: candidate.revision_sha256.clone(),
                        verifier_receipt_sha256: candidate.verifier_receipt_sha256.clone(),
                        verified_revision_sha256: candidate.verified_revision_sha256.clone(),
                        disposition: IntervalCandidateDisposition::ExcludedUnrankable(reasons),
                    });
                    continue;
                }
                let mut objectives = candidate.objectives.iter().collect::<Vec<_>>();
                objectives.sort_by_key(|objective| objective.dimension.clone());
                rankable.push(RankableIntervalCandidate {
                    candidate,
                    objectives,
                });
            }
        }
    }

    rankable.sort_by_key(|candidate| candidate.candidate.candidate_id.clone());
    if let Some(first) = rankable.first() {
        let schema = first.schema();
        for candidate in rankable.iter().skip(1) {
            if candidate.schema() != schema {
                return Err(IntervalFrontierError::IncompatibleObjectiveSchemas {
                    candidate_id: candidate.candidate.candidate_id.clone(),
                });
            }
        }
    }

    let mut frontier_ids = Vec::new();
    for candidate in &rankable {
        let witness = rankable
            .iter()
            .find(|other| {
                other.candidate.candidate_id != candidate.candidate.candidate_id
                    && guaranteed_interval_dominates(other, candidate)
            })
            .map(|other| other.candidate.candidate_id.clone());

        let disposition = match witness {
            Some(witness_id) => IntervalCandidateDisposition::GuaranteedDominatedBy(witness_id),
            None => {
                frontier_ids.push(candidate.candidate.candidate_id.clone());
                IntervalCandidateDisposition::OnFrontier
            }
        };
        results.push(IntervalCandidateResult {
            candidate_id: candidate.candidate.candidate_id.clone(),
            revision_sha256: candidate.candidate.revision_sha256.clone(),
            verifier_receipt_sha256: candidate.candidate.verifier_receipt_sha256.clone(),
            verified_revision_sha256: candidate.candidate.verified_revision_sha256.clone(),
            disposition,
        });
    }

    results.sort_by_key(|result| result.candidate_id.clone());
    frontier_ids.sort();

    Ok(IntervalFrontierReport {
        frontier_ids,
        candidates: results,
        ranking_complete: ranking_complete && !rankable.is_empty(),
        relation_name: "guaranteed_interval_dominance_v0_1",
        claim_ceiling: "A conservative Pareto comparison under the caller-supplied objective intervals and matched interval semantics among candidates labelled Verified by an independent caller. This does not authenticate interval sources or receipts, establish real-world truth, prove global optimality, certify safety/accessibility/code compliance, or measure resident benefit.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semantics() -> IntervalSemantics {
        IntervalSemantics {
            kind: IntervalSemanticsKind::DeterministicEnvelope,
            basis_id: "synthetic-deterministic-bounds-v1".to_owned(),
        }
    }

    fn objective(
        dimension: &str,
        unit: &str,
        direction: ObjectiveDirection,
        lower: f64,
        upper: f64,
    ) -> IntervalObjectiveObservation {
        IntervalObjectiveObservation {
            dimension: dimension.to_owned(),
            unit: unit.to_owned(),
            direction,
            lower,
            upper,
            semantics: semantics(),
            evidence: ObjectiveEvidence::AcceptedByPolicy,
        }
    }

    fn candidate(
        id: &str,
        verification: VerificationStatus,
        cost: (f64, f64),
        coverage: (f64, f64),
    ) -> IntervalAlternativeCandidate {
        IntervalAlternativeCandidate {
            candidate_id: id.to_owned(),
            revision_sha256: Some("a".repeat(64)),
            verifier_receipt_sha256: Some("b".repeat(64)),
            verified_revision_sha256: Some("a".repeat(64)),
            verification,
            objectives: vec![
                objective(
                    "install_cost",
                    "USD",
                    ObjectiveDirection::Minimize,
                    cost.0,
                    cost.1,
                ),
                objective(
                    "task_coverage",
                    "ratio",
                    ObjectiveDirection::Maximize,
                    coverage.0,
                    coverage.1,
                ),
            ],
        }
    }

    fn result<'a>(
        report: &'a IntervalFrontierReport,
        id: &str,
    ) -> &'a IntervalCandidateDisposition {
        &report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == id)
            .expect("candidate result must exist")
            .disposition
    }

    #[test]
    fn intervals_that_are_separated_guarantee_dominance() {
        let input = vec![
            candidate(
                "better",
                VerificationStatus::Verified,
                (10.0, 12.0),
                (80.0, 85.0),
            ),
            candidate(
                "worse",
                VerificationStatus::Verified,
                (14.0, 16.0),
                (70.0, 75.0),
            ),
        ];
        let report = compute_interval_frontier(&input).expect("valid interval schema");
        assert_eq!(report.frontier_ids, vec!["better"]);
        assert_eq!(
            result(&report, "worse"),
            &IntervalCandidateDisposition::GuaranteedDominatedBy("better".to_owned())
        );
        assert!(report.ranking_complete);
        assert_eq!(report.relation_name, "guaranteed_interval_dominance_v0_1");
    }

    #[test]
    fn overlapping_intervals_are_not_collapsed_to_midpoints() {
        // A midpoint-only ranking would claim a dominates b in both objectives,
        // but neither interval is separated enough to guarantee that relation.
        let input = vec![
            candidate(
                "a",
                VerificationStatus::Verified,
                (10.0, 20.0),
                (80.0, 100.0),
            ),
            candidate(
                "b",
                VerificationStatus::Verified,
                (15.0, 25.0),
                (75.0, 85.0),
            ),
        ];
        let report = compute_interval_frontier(&input).expect("valid interval schema");
        assert_eq!(report.frontier_ids, vec!["a", "b"]);
        assert_eq!(
            result(&report, "a"),
            &IntervalCandidateDisposition::OnFrontier
        );
        assert_eq!(
            result(&report, "b"),
            &IntervalCandidateDisposition::OnFrontier
        );
    }

    #[test]
    fn every_guaranteed_dominance_holds_for_all_interval_endpoints() {
        let bounds = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (-1.0, 2.0), (2.0, 3.0)];
        for cost_left in bounds {
            for cost_right in bounds {
                for coverage_left in bounds {
                    for coverage_right in bounds {
                        let left = candidate(
                            "left",
                            VerificationStatus::Verified,
                            cost_left,
                            coverage_left,
                        );
                        let right = candidate(
                            "right",
                            VerificationStatus::Verified,
                            cost_right,
                            coverage_right,
                        );
                        let mut left_objectives = left.objectives.iter().collect::<Vec<_>>();
                        left_objectives.sort_by_key(|objective| objective.dimension.clone());
                        let mut right_objectives = right.objectives.iter().collect::<Vec<_>>();
                        right_objectives.sort_by_key(|objective| objective.dimension.clone());
                        let left_rankable = RankableIntervalCandidate {
                            candidate: &left,
                            objectives: left_objectives,
                        };
                        let right_rankable = RankableIntervalCandidate {
                            candidate: &right,
                            objectives: right_objectives,
                        };
                        if !guaranteed_interval_dominates(&left_rankable, &right_rankable) {
                            continue;
                        }

                        for a_cost in [cost_left.0, cost_left.1] {
                            for b_cost in [cost_right.0, cost_right.1] {
                                for a_coverage in [coverage_left.0, coverage_left.1] {
                                    for b_coverage in [coverage_right.0, coverage_right.1] {
                                        assert!(
                                            a_cost <= b_cost,
                                            "guaranteed minimize relation violated: {cost_left:?} vs {cost_right:?}"
                                        );
                                        assert!(
                                            a_coverage >= b_coverage,
                                            "guaranteed maximize relation violated: {coverage_left:?} vs {coverage_right:?}"
                                        );
                                        assert!(
                                            a_cost < b_cost || a_coverage > b_coverage,
                                            "strict separation must hold for every endpoint realization"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn point_intervals_agree_with_point_dominance() {
        let point = |id: &str, cost: f64, coverage: f64| {
            candidate(
                id,
                VerificationStatus::Verified,
                (cost, cost),
                (coverage, coverage),
            )
        };
        let input = vec![
            point("a", 10.0, 80.0),
            point("b", 12.0, 70.0),
            point("c", 15.0, 60.0),
        ];
        let report = compute_interval_frontier(&input).expect("valid point intervals");
        assert_eq!(report.frontier_ids, vec!["a"]);
        assert_eq!(
            result(&report, "b"),
            &IntervalCandidateDisposition::GuaranteedDominatedBy("a".to_owned())
        );
        assert_eq!(
            result(&report, "c"),
            &IntervalCandidateDisposition::GuaranteedDominatedBy("a".to_owned())
        );
    }

    #[test]
    fn equal_or_touching_intervals_without_strict_separation_remain_tied() {
        let input = vec![
            candidate(
                "a",
                VerificationStatus::Verified,
                (10.0, 12.0),
                (80.0, 90.0),
            ),
            candidate(
                "b",
                VerificationStatus::Verified,
                (12.0, 15.0),
                (75.0, 80.0),
            ),
        ];
        let report = compute_interval_frontier(&input).expect("valid touching intervals");
        assert_eq!(report.frontier_ids, vec!["a", "b"]);
    }

    #[test]
    fn blocked_and_indeterminate_candidates_never_enter_frontier() {
        let input = vec![
            candidate(
                "verified",
                VerificationStatus::Verified,
                (10.0, 12.0),
                (80.0, 85.0),
            ),
            candidate(
                "blocked",
                VerificationStatus::Blocked,
                (0.0, 1.0),
                (99.0, 100.0),
            ),
            candidate(
                "unknown-gate",
                VerificationStatus::Indeterminate,
                (0.0, 1.0),
                (99.0, 100.0),
            ),
        ];
        let report = compute_interval_frontier(&input).expect("valid interval schema");
        assert_eq!(report.frontier_ids, vec!["verified"]);
        assert_eq!(
            result(&report, "blocked"),
            &IntervalCandidateDisposition::ExcludedBlocked
        );
        assert_eq!(
            result(&report, "unknown-gate"),
            &IntervalCandidateDisposition::ExcludedIndeterminate
        );
    }

    #[test]
    fn non_admitted_objective_makes_ranking_incomplete() {
        let mut unknown = candidate(
            "unknown",
            VerificationStatus::Verified,
            (0.0, 1.0),
            (99.0, 100.0),
        );
        unknown.objectives[0].evidence = ObjectiveEvidence::Unknown;
        let known = candidate(
            "known",
            VerificationStatus::Verified,
            (10.0, 12.0),
            (80.0, 85.0),
        );
        let report =
            compute_interval_frontier(&[unknown, known]).expect("excluded evidence is explicit");
        assert_eq!(report.frontier_ids, vec!["known"]);
        assert!(!report.ranking_complete);
        assert!(matches!(
            result(&report, "unknown"),
            IntervalCandidateDisposition::ExcludedUnrankable(reasons)
                if reasons.iter().any(|reason| reason.contains("Unknown"))
        ));
    }

    #[test]
    fn reversed_and_non_finite_intervals_are_excluded() {
        let mut reversed = candidate(
            "reversed",
            VerificationStatus::Verified,
            (14.0, 10.0),
            (70.0, 80.0),
        );
        reversed.objectives[0].lower = 14.0;
        reversed.objectives[0].upper = 10.0;
        let mut non_finite = candidate(
            "non-finite",
            VerificationStatus::Verified,
            (10.0, 12.0),
            (80.0, 85.0),
        );
        non_finite.objectives[0].lower = f64::NAN;
        let report = compute_interval_frontier(&[reversed, non_finite])
            .expect("malformed intervals remain visible as exclusions");
        assert!(report.frontier_ids.is_empty());
        assert!(!report.ranking_complete);
        for id in ["reversed", "non-finite"] {
            assert!(matches!(
                result(&report, id),
                IntervalCandidateDisposition::ExcludedUnrankable(_)
            ));
        }
    }

    #[test]
    fn uncertainty_semantics_and_units_must_match() {
        let mut different_semantics = candidate(
            "b",
            VerificationStatus::Verified,
            (10.0, 11.0),
            (80.0, 81.0),
        );
        different_semantics.objectives[0].semantics = IntervalSemantics {
            kind: IntervalSemanticsKind::DeterministicEnvelope,
            basis_id: "alternate-envelope-v2".to_owned(),
        };
        let a = candidate(
            "a",
            VerificationStatus::Verified,
            (12.0, 13.0),
            (70.0, 71.0),
        );
        assert!(matches!(
            compute_interval_frontier(&[a.clone(), different_semantics]),
            Err(IntervalFrontierError::IncompatibleObjectiveSchemas { .. })
        ));

        let mut different_unit = candidate(
            "zar",
            VerificationStatus::Verified,
            (10.0, 11.0),
            (80.0, 81.0),
        );
        different_unit.objectives[0].unit = "ZAR".to_owned();
        assert!(matches!(
            compute_interval_frontier(&[a, different_unit]),
            Err(IntervalFrontierError::IncompatibleObjectiveSchemas { .. })
        ));
    }

    #[test]
    fn statistical_intervals_do_not_claim_deterministic_guarantees() {
        let mut statistical = candidate(
            "statistical",
            VerificationStatus::Verified,
            (10.0, 12.0),
            (80.0, 85.0),
        );
        statistical.objectives[0].semantics = IntervalSemantics {
            kind: IntervalSemanticsKind::ConfidenceInterval,
            basis_id: "bootstrap-95pct-v1".to_owned(),
        };
        let report = compute_interval_frontier(&[statistical])
            .expect("unsupported semantics must be explicitly excluded");
        assert!(report.frontier_ids.is_empty());
        assert!(!report.ranking_complete);
        assert!(matches!(
            result(&report, "statistical"),
            IntervalCandidateDisposition::ExcludedUnrankable(reasons)
                if reasons.iter().any(|reason| reason.contains("not a deterministic envelope"))
        ));
    }

    #[test]
    fn stale_receipt_cannot_rank_a_new_candidate_revision() {
        let mut stale = candidate(
            "edited",
            VerificationStatus::Verified,
            (1.0, 2.0),
            (99.0, 100.0),
        );
        stale.revision_sha256 = Some("c".repeat(64));
        stale.verified_revision_sha256 = Some("a".repeat(64));
        let current = candidate(
            "current",
            VerificationStatus::Verified,
            (10.0, 12.0),
            (80.0, 85.0),
        );
        let report =
            compute_interval_frontier(&[stale, current]).expect("stale binding is reported");
        assert_eq!(report.frontier_ids, vec!["current"]);
        assert!(!report.ranking_complete);
        assert!(matches!(
            result(&report, "edited"),
            IntervalCandidateDisposition::ExcludedUnrankable(reasons)
                if reasons.iter().any(|reason| reason.contains("different candidate revision"))
        ));
    }

    #[test]
    fn frontier_is_deterministic_under_input_order() {
        let candidates = vec![
            candidate(
                "a",
                VerificationStatus::Verified,
                (10.0, 20.0),
                (70.0, 90.0),
            ),
            candidate(
                "b",
                VerificationStatus::Verified,
                (15.0, 25.0),
                (75.0, 95.0),
            ),
            candidate(
                "c",
                VerificationStatus::Verified,
                (30.0, 35.0),
                (40.0, 45.0),
            ),
        ];
        let first = compute_interval_frontier(&candidates).expect("valid schema");
        let mut reverse = candidates.clone();
        reverse.reverse();
        let second = compute_interval_frontier(&reverse).expect("valid schema");
        assert_eq!(first, second);
    }

    #[test]
    fn empty_request_and_duplicate_ids_fail_closed() {
        assert_eq!(
            compute_interval_frontier(&[]),
            Err(IntervalFrontierError::EmptyRequest)
        );
        let a = candidate(
            "duplicate",
            VerificationStatus::Verified,
            (1.0, 2.0),
            (80.0, 90.0),
        );
        let b = candidate(
            "duplicate",
            VerificationStatus::Verified,
            (4.0, 5.0),
            (60.0, 70.0),
        );
        assert_eq!(
            compute_interval_frontier(&[a, b]),
            Err(IntervalFrontierError::DuplicateCandidateId(
                "duplicate".to_owned()
            ))
        );
    }
}

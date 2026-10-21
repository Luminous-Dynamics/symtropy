//! Evidence-qualified Pareto ranking for furnishing alternatives.
//!
//! This module ranks only candidates reported as verified by an independent
//! layout oracle. It never lets preference, price, aesthetics, or another
//! objective compensate for a blocked or indeterminate hard constraint.
//!
//! Digest validation here checks syntax and preserves identifiers; it does not
//! authenticate a source, recompute a digest, or validate a verifier receipt.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    /// The independent verifier accepted this exact candidate revision.
    Verified,
    /// At least one mandatory condition failed.
    Blocked,
    /// A mandatory condition could not be resolved.
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectiveDirection {
    /// Smaller values are preferred.
    Minimize,
    /// Larger values are preferred.
    Maximize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectiveEvidence {
    /// A caller's explicit policy allows this observation for comparison.
    AcceptedByPolicy,
    /// The observation is not yet eligible for ranking.
    Provisional,
    /// The observation is unavailable or outside its scope.
    Unknown,
    /// Multiple incompatible claims remain unresolved.
    Conflicting,
    /// Policy explicitly disallows this observation.
    Blocked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveObservation {
    /// Stable objective key, such as install_cost or task_coverage.
    pub dimension: String,
    /// Explicit unit or normalized scale; comparisons require an exact match.
    pub unit: String,
    /// Whether the objective is minimized or maximized.
    pub direction: ObjectiveDirection,
    /// Finite numeric observation in the dimension's declared units.
    pub value: f64,
    /// Evidence admission state supplied by the caller's policy layer.
    pub evidence: ObjectiveEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlternativeCandidate {
    /// Stable, unique identifier for this alternative in the request.
    pub candidate_id: String,
    /// SHA-256 digest of the exact candidate revision, if available.
    pub revision_sha256: Option<String>,
    /// SHA-256 digest identifying the independent verifier receipt, if available.
    pub verifier_receipt_sha256: Option<String>,
    /// Exact candidate revision SHA-256 that the receipt claims to have verified.
    /// This is a caller-provided receipt field, not independently authenticated here.
    pub verified_revision_sha256: Option<String>,
    /// Result from the independent hard-constraint verifier.
    pub verification: VerificationStatus,
    /// Objective observations; all rankable candidates must share the same schema.
    pub objectives: Vec<ObjectiveObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateDisposition {
    /// Eligible and not dominated by another eligible candidate.
    OnFrontier,
    /// A deterministic witness eligible candidate dominates this alternative.
    DominatedBy(String),
    /// The independent verifier rejected one or more mandatory conditions.
    ExcludedBlocked,
    /// A mandatory condition remained unresolved in the independent verifier.
    ExcludedIndeterminate,
    /// The alternative cannot safely join the ranking (with reasons).
    ExcludedUnrankable(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateResult {
    /// Stable candidate identifier from the request.
    pub candidate_id: String,
    /// Candidate revision digest passed through for traceability.
    pub revision_sha256: Option<String>,
    /// Independent verifier receipt digest passed through for traceability.
    pub verifier_receipt_sha256: Option<String>,
    /// Candidate revision digest claimed by the verifier receipt.
    pub verified_revision_sha256: Option<String>,
    /// Ranking or exclusion result; never a substitute for the independent verifier.
    pub disposition: CandidateDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrontierReport {
    /// Non-dominated, independently verified candidate IDs in stable order.
    pub frontier_ids: Vec<String>,
    /// One result for every input candidate, sorted by candidate ID.
    pub candidates: Vec<CandidateResult>,
    /// False when a verified candidate was unrankable or no complete frontier exists.
    pub ranking_complete: bool,
    /// Objective dimensions used for this comparison, in stable order.
    pub objective_dimensions: Vec<String>,
    /// Explicit limit on what this report establishes.
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontierError {
    /// A ranking request must contain at least one candidate.
    EmptyRequest,
    /// Candidate IDs must be nonblank and have no surrounding whitespace.
    InvalidCandidateId { index: usize },
    /// Candidate IDs must be unique within a request.
    DuplicateCandidateId(String),
    /// Rankable candidates must declare identical objective keys and directions.
    IncompatibleObjectiveSchemas { candidate_id: String },
}

impl fmt::Display for FrontierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRequest => write!(f, "frontier request contains no candidates"),
            Self::InvalidCandidateId { index } => {
                write!(f, "candidate at index {index} has an invalid identifier")
            }
            Self::DuplicateCandidateId(id) => {
                write!(f, "candidate identifier {id:?} occurs more than once")
            }
            Self::IncompatibleObjectiveSchemas { candidate_id } => {
                write!(
                    f,
                    "candidate {candidate_id:?} has an objective schema incompatible with the ranking set"
                )
            }
        }
    }
}

impl std::error::Error for FrontierError {}

#[derive(Clone, Debug)]
struct RankableCandidate<'a> {
    candidate: &'a AlternativeCandidate,
    /// Sorted by objective dimension for order-independent comparison.
    objectives: Vec<&'a ObjectiveObservation>,
}

impl RankableCandidate<'_> {
    fn schema(&self) -> Vec<(&str, ObjectiveDirection, &str)> {
        self.objectives
            .iter()
            .map(|objective| {
                (
                    objective.dimension.as_str(),
                    objective.direction,
                    objective.unit.as_str(),
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

fn objective_exclusion_reasons(candidate: &AlternativeCandidate) -> Vec<String> {
    let mut reasons = Vec::new();
    if !valid_sha256(candidate.revision_sha256.as_deref()) {
        reasons.push("missing or malformed exact revision SHA-256 digest".to_owned());
    }
    if !valid_sha256(candidate.verifier_receipt_sha256.as_deref()) {
        reasons.push("missing or malformed independent verifier receipt SHA-256 digest".to_owned());
    }
    if !valid_sha256(candidate.verified_revision_sha256.as_deref()) {
        reasons.push(
            "missing or malformed verifier-receipt candidate revision SHA-256 digest".to_owned(),
        );
    } else if candidate.verified_revision_sha256 != candidate.revision_sha256 {
        reasons.push(
            "independent verifier receipt is bound to a different candidate revision".to_owned(),
        );
    }
    if candidate.objectives.is_empty() {
        reasons.push("no objective observations supplied".to_owned());
        return reasons;
    }

    let mut dimensions = BTreeSet::new();
    for objective in &candidate.objectives {
        if objective.dimension.trim().is_empty()
            || objective.dimension != objective.dimension.trim()
        {
            reasons.push("objective dimension is blank or has surrounding whitespace".to_owned());
        } else if !dimensions.insert(objective.dimension.as_str()) {
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
        if !objective.value.is_finite() {
            reasons.push(format!("objective {:?} is not finite", objective.dimension));
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

fn dominates(left: &RankableCandidate<'_>, right: &RankableCandidate<'_>) -> bool {
    let mut strictly_better = false;
    for (left_value, right_value) in left.objectives.iter().zip(&right.objectives) {
        match left_value.direction {
            ObjectiveDirection::Minimize => {
                if left_value.value > right_value.value {
                    return false;
                }
                strictly_better |= left_value.value < right_value.value;
            }
            ObjectiveDirection::Maximize => {
                if left_value.value < right_value.value {
                    return false;
                }
                strictly_better |= left_value.value > right_value.value;
            }
        }
    }
    strictly_better
}

/// Convert an objective into a minimization coordinate without retaining signed zero.
fn minimization_coordinate(objective: &ObjectiveObservation) -> f64 {
    let value = if objective.value == 0.0 {
        0.0
    } else {
        objective.value
    };
    match objective.direction {
        ObjectiveDirection::Minimize => value,
        ObjectiveDirection::Maximize => -value,
    }
}

/// Find one deterministic dominator per dominated candidate in two dimensions.
///
/// This sweep is O(n log n): sort by the first minimization coordinate, then
/// maintain the smallest second coordinate seen in earlier coordinate groups.
/// Equal first-coordinate points are processed as a group so identical points
/// do not incorrectly dominate one another.
fn two_dimensional_dominators(rankable: &[RankableCandidate<'_>]) -> BTreeMap<String, String> {
    let mut points = rankable
        .iter()
        .map(|candidate| {
            (
                candidate.candidate.candidate_id.as_str(),
                minimization_coordinate(candidate.objectives[0]),
                minimization_coordinate(candidate.objectives[1]),
            )
        })
        .collect::<Vec<_>>();
    points.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.2.total_cmp(&right.2))
            .then_with(|| left.0.cmp(right.0))
    });

    let mut dominators = BTreeMap::new();
    let mut prefix_min_y: Option<(f64, String)> = None;
    let mut group_start = 0;

    while group_start < points.len() {
        let group_x = points[group_start].1;
        let mut group_end = group_start + 1;
        while group_end < points.len() && points[group_end].1 == group_x {
            group_end += 1;
        }

        // Within an equal-x group, points are sorted by y then stable ID.
        let group_min_y = points[group_start].2;
        let group_min_id = points[group_start].0.to_owned();
        if let Some((best_y, witness_id)) = &prefix_min_y {
            if *best_y <= group_min_y {
                // A strictly smaller x coordinate makes this a real
                // dominator even when the y coordinates are exactly equal.
                for point in &points[group_start..group_end] {
                    dominators.insert(point.0.to_owned(), witness_id.clone());
                }
            } else {
                // No prior x group dominates the minimum-y points. Those
                // points form this group's frontier; a group minimum
                // dominates each point with a strictly larger y coordinate.
                for point in &points[group_start..group_end] {
                    if point.2 > group_min_y {
                        dominators.insert(point.0.to_owned(), group_min_id.clone());
                    }
                }
            }
        } else {
            for point in &points[group_start..group_end] {
                if point.2 > group_min_y {
                    dominators.insert(point.0.to_owned(), group_min_id.clone());
                }
            }
        }

        match &prefix_min_y {
            None => prefix_min_y = Some((group_min_y, group_min_id)),
            Some((best_y, _)) if group_min_y < *best_y => {
                prefix_min_y = Some((group_min_y, group_min_id));
            }
            Some(_) => {}
        }
        group_start = group_end;
    }

    dominators
}

/// Compute a Pareto frontier without scalarizing objectives.
///
/// Only alternatives marked Verified by the caller's independent verifier can
/// enter the ranking. Blocked and indeterminate alternatives are always
/// excluded, regardless of how attractive their objective values look.
/// Verified alternatives with unknown or disallowed objective evidence are
/// excluded and make the report incomplete, rather than being silently treated
/// as worse or better.
///
/// All rankable alternatives must use the same objective dimensions and
/// directions. Numeric comparisons use the supplied finite f64 values directly;
/// this function invents neither weights nor tolerances. Callers must define
/// units, evidence-admission rules, and any justified normalization upstream.
///
/// Two-objective inputs use an O(n log n) sweep-line frontier. Higher-dimensional
/// inputs use worst-case quadratic comparison time. Only one dominance witness
/// is stored per excluded candidate, keeping output memory linear in candidate
/// count.
///
/// This function checks digest shape but does not authenticate digests or
/// receipts. The claim of independent verification originates outside this
/// module and must be bound by the caller to the exact candidate revision.
pub fn compute_pareto_frontier(
    candidates: &[AlternativeCandidate],
) -> Result<FrontierReport, FrontierError> {
    if candidates.is_empty() {
        return Err(FrontierError::EmptyRequest);
    }

    let mut seen = BTreeSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.candidate_id.trim().is_empty()
            || candidate.candidate_id != candidate.candidate_id.trim()
        {
            return Err(FrontierError::InvalidCandidateId { index });
        }
        if !seen.insert(candidate.candidate_id.as_str()) {
            return Err(FrontierError::DuplicateCandidateId(
                candidate.candidate_id.clone(),
            ));
        }
    }

    let mut results = Vec::with_capacity(candidates.len());
    let mut rankable = Vec::new();
    let mut ranking_complete = true;

    for candidate in candidates {
        match candidate.verification {
            VerificationStatus::Blocked => results.push(CandidateResult {
                candidate_id: candidate.candidate_id.clone(),
                revision_sha256: candidate.revision_sha256.clone(),
                verifier_receipt_sha256: candidate.verifier_receipt_sha256.clone(),
                verified_revision_sha256: candidate.verified_revision_sha256.clone(),
                disposition: CandidateDisposition::ExcludedBlocked,
            }),
            VerificationStatus::Indeterminate => results.push(CandidateResult {
                candidate_id: candidate.candidate_id.clone(),
                revision_sha256: candidate.revision_sha256.clone(),
                verifier_receipt_sha256: candidate.verifier_receipt_sha256.clone(),
                verified_revision_sha256: candidate.verified_revision_sha256.clone(),
                disposition: CandidateDisposition::ExcludedIndeterminate,
            }),
            VerificationStatus::Verified => {
                let reasons = objective_exclusion_reasons(candidate);
                if !reasons.is_empty() {
                    ranking_complete = false;
                    results.push(CandidateResult {
                        candidate_id: candidate.candidate_id.clone(),
                        revision_sha256: candidate.revision_sha256.clone(),
                        verifier_receipt_sha256: candidate.verifier_receipt_sha256.clone(),
                        verified_revision_sha256: candidate.verified_revision_sha256.clone(),
                        disposition: CandidateDisposition::ExcludedUnrankable(reasons),
                    });
                    continue;
                }

                let mut objectives = candidate.objectives.iter().collect::<Vec<_>>();
                objectives.sort_by_key(|objective| objective.dimension.clone());
                rankable.push(RankableCandidate {
                    candidate,
                    objectives,
                });
            }
        }
    }

    rankable.sort_by(|left, right| {
        left.candidate
            .candidate_id
            .cmp(&right.candidate.candidate_id)
    });
    if let Some(first) = rankable.first() {
        let expected_schema = first.schema();
        for candidate in rankable.iter().skip(1) {
            if candidate.schema() != expected_schema {
                return Err(FrontierError::IncompatibleObjectiveSchemas {
                    candidate_id: candidate.candidate.candidate_id.clone(),
                });
            }
        }
    }

    let two_dimensional = rankable
        .first()
        .is_some_and(|candidate| candidate.objectives.len() == 2);
    let sweep_dominators = two_dimensional.then(|| two_dimensional_dominators(&rankable));

    let mut frontier_ids = Vec::new();
    for candidate in &rankable {
        // Two-objective inputs use the sweep result; higher-dimensional inputs
        // retain the general pairwise verifier. Neither path invents weights.
        let first_dominator = if let Some(dominators) = &sweep_dominators {
            dominators.get(&candidate.candidate.candidate_id).cloned()
        } else {
            rankable
                .iter()
                .find(|other| {
                    other.candidate.candidate_id != candidate.candidate.candidate_id
                        && dominates(other, candidate)
                })
                .map(|other| other.candidate.candidate_id.clone())
        };

        let disposition = if let Some(witness) = first_dominator {
            CandidateDisposition::DominatedBy(witness)
        } else {
            frontier_ids.push(candidate.candidate.candidate_id.clone());
            CandidateDisposition::OnFrontier
        };
        results.push(CandidateResult {
            candidate_id: candidate.candidate.candidate_id.clone(),
            revision_sha256: candidate.candidate.revision_sha256.clone(),
            verifier_receipt_sha256: candidate.candidate.verifier_receipt_sha256.clone(),
            verified_revision_sha256: candidate.candidate.verified_revision_sha256.clone(),
            disposition,
        });
    }

    results.sort_by_key(|result| result.candidate_id.clone());
    frontier_ids.sort();
    let objective_dimensions = rankable
        .first()
        .map(|candidate| {
            candidate
                .objectives
                .iter()
                .map(|objective| objective.dimension.clone())
                .collect()
        })
        .unwrap_or_default();

    Ok(FrontierReport {
        frontier_ids,
        candidates: results,
        ranking_complete: ranking_complete && !rankable.is_empty(),
        objective_dimensions,
        claim_ceiling: "A deterministic Pareto comparison of caller-supplied finite objective observations among candidates labelled verified by an independent caller. This report does not authenticate evidence, establish source accuracy, prove global optimality, certify safety/accessibility/code compliance, or measure resident benefit.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective_in_unit(
        dimension: &str,
        unit: &str,
        direction: ObjectiveDirection,
        value: f64,
    ) -> ObjectiveObservation {
        ObjectiveObservation {
            dimension: dimension.to_owned(),
            unit: unit.to_owned(),
            direction,
            value,
            evidence: ObjectiveEvidence::AcceptedByPolicy,
        }
    }

    fn objective(
        dimension: &str,
        direction: ObjectiveDirection,
        value: f64,
    ) -> ObjectiveObservation {
        let unit = match dimension {
            "install_cost" | "cost" => "USD",
            "task_coverage" => "ratio",
            "aesthetics" => "score_0_to_100",
            _ => "test_unit",
        };
        objective_in_unit(dimension, unit, direction, value)
    }

    fn candidate(
        id: &str,
        verification: VerificationStatus,
        objectives: Vec<ObjectiveObservation>,
    ) -> AlternativeCandidate {
        AlternativeCandidate {
            candidate_id: id.to_owned(),
            revision_sha256: Some("a".repeat(64)),
            verifier_receipt_sha256: Some("b".repeat(64)),
            verified_revision_sha256: Some("a".repeat(64)),
            verification,
            objectives,
        }
    }

    fn two_objectives(cost: f64, coverage: f64) -> Vec<ObjectiveObservation> {
        vec![
            objective("install_cost", ObjectiveDirection::Minimize, cost),
            objective("task_coverage", ObjectiveDirection::Maximize, coverage),
        ]
    }

    fn disposition<'a>(report: &'a FrontierReport, id: &str) -> &'a CandidateDisposition {
        &report
            .candidates
            .iter()
            .find(|result| result.candidate_id == id)
            .expect("candidate result must be present")
            .disposition
    }

    fn pairwise_frontier_ids(candidates: &[AlternativeCandidate]) -> Vec<String> {
        let mut frontier = Vec::new();
        for (index, subject) in candidates.iter().enumerate() {
            let mut subject_objectives = subject.objectives.iter().collect::<Vec<_>>();
            subject_objectives.sort_by_key(|objective| objective.dimension.clone());
            let subject_rankable = RankableCandidate {
                candidate: subject,
                objectives: subject_objectives,
            };
            let is_dominated = candidates.iter().enumerate().any(|(other_index, other)| {
                if index == other_index {
                    return false;
                }
                let mut other_objectives = other.objectives.iter().collect::<Vec<_>>();
                other_objectives.sort_by_key(|objective| objective.dimension.clone());
                dominates(
                    &RankableCandidate {
                        candidate: other,
                        objectives: other_objectives,
                    },
                    &subject_rankable,
                )
            });
            if !is_dominated {
                frontier.push(subject.candidate_id.clone());
            }
        }
        frontier.sort();
        frontier
    }

    #[test]
    fn returns_non_dominated_tradeoffs_without_scalar_weights() {
        let input = vec![
            candidate(
                "low-cost",
                VerificationStatus::Verified,
                two_objectives(10.0, 70.0),
            ),
            candidate(
                "high-coverage",
                VerificationStatus::Verified,
                two_objectives(12.0, 90.0),
            ),
            candidate(
                "dominated",
                VerificationStatus::Verified,
                two_objectives(13.0, 60.0),
            ),
        ];
        let report = compute_pareto_frontier(&input).expect("valid frontier");
        assert_eq!(report.frontier_ids, vec!["high-coverage", "low-cost"]);
        assert_eq!(
            report.objective_dimensions,
            vec!["install_cost", "task_coverage"]
        );
        assert_eq!(
            disposition(&report, "dominated"),
            &CandidateDisposition::DominatedBy("high-coverage".to_owned())
        );
        assert!(report.ranking_complete);
    }

    #[test]
    fn blocked_and_indeterminate_never_enter_frontier() {
        let input = vec![
            candidate(
                "verified",
                VerificationStatus::Verified,
                two_objectives(10.0, 70.0),
            ),
            candidate(
                "blocked-best-score",
                VerificationStatus::Blocked,
                two_objectives(0.0, 100.0),
            ),
            candidate(
                "unknown-hard-gate",
                VerificationStatus::Indeterminate,
                two_objectives(0.0, 100.0),
            ),
        ];
        let report = compute_pareto_frontier(&input).expect("valid frontier");
        assert_eq!(report.frontier_ids, vec!["verified"]);
        assert_eq!(
            disposition(&report, "blocked-best-score"),
            &CandidateDisposition::ExcludedBlocked
        );
        assert_eq!(
            disposition(&report, "unknown-hard-gate"),
            &CandidateDisposition::ExcludedIndeterminate
        );
        assert!(report.ranking_complete);
    }

    #[test]
    fn non_admitted_objective_evidence_excludes_candidate_and_marks_incomplete() {
        let mut unknown = candidate(
            "unknown-cost",
            VerificationStatus::Verified,
            two_objectives(1.0, 100.0),
        );
        unknown.objectives[0].evidence = ObjectiveEvidence::Unknown;
        let input = vec![
            unknown,
            candidate(
                "known",
                VerificationStatus::Verified,
                two_objectives(10.0, 70.0),
            ),
        ];
        let report = compute_pareto_frontier(&input).expect("partial frontier is explicit");
        assert_eq!(report.frontier_ids, vec!["known"]);
        assert!(!report.ranking_complete);
        assert!(matches!(
            disposition(&report, "unknown-cost"),
            CandidateDisposition::ExcludedUnrankable(reasons)
                if reasons.iter().any(|reason| reason.contains("Unknown"))
        ));
    }

    #[test]
    fn every_non_admitted_objective_state_is_excluded() {
        for evidence in [
            ObjectiveEvidence::Provisional,
            ObjectiveEvidence::Unknown,
            ObjectiveEvidence::Conflicting,
            ObjectiveEvidence::Blocked,
        ] {
            let mut uncertain = candidate(
                "uncertain",
                VerificationStatus::Verified,
                two_objectives(1.0, 100.0),
            );
            uncertain.objectives[0].evidence = evidence;
            let admitted = candidate(
                "admitted",
                VerificationStatus::Verified,
                two_objectives(10.0, 70.0),
            );
            let report = compute_pareto_frontier(&[uncertain, admitted])
                .expect("non-admitted values must be reported as exclusions");
            assert_eq!(report.frontier_ids, vec!["admitted"], "{evidence:?}");
            assert!(!report.ranking_complete, "{evidence:?}");
            assert!(
                matches!(
                    disposition(&report, "uncertain"),
                    CandidateDisposition::ExcludedUnrankable(_)
                ),
                "{evidence:?}"
            );
        }
    }

    #[test]
    fn result_is_invariant_to_candidate_and_objective_input_order() {
        let a = candidate(
            "a",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let mut b = candidate(
            "b",
            VerificationStatus::Verified,
            two_objectives(12.0, 90.0),
        );
        b.objectives.reverse();
        let left = compute_pareto_frontier(&[a.clone(), b.clone()]).expect("valid frontier");
        let right = compute_pareto_frontier(&[b, a]).expect("valid frontier");
        assert_eq!(left, right);
    }

    #[test]
    fn incompatible_objective_schemas_fail_closed() {
        let a = candidate(
            "a",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let b = candidate(
            "b",
            VerificationStatus::Verified,
            vec![
                objective("install_cost", ObjectiveDirection::Minimize, 9.0),
                objective("aesthetics", ObjectiveDirection::Maximize, 99.0),
            ],
        );
        assert!(matches!(
            compute_pareto_frontier(&[a, b]),
            Err(FrontierError::IncompatibleObjectiveSchemas { .. })
        ));
    }

    #[test]
    fn two_dimensional_sweep_matches_pairwise_dominance_on_adversarial_points() {
        let specs = [
            ("a", 1.0, 8.0),
            ("b", 2.0, 7.0),
            ("c", 2.0, 9.0),
            ("d", 3.0, 6.0),
            ("e", 3.0, 6.0),
            ("f", 4.0, 8.0),
            ("g", 5.0, 4.0),
            ("h", 6.0, 3.0),
            ("i", 6.0, 3.0),
            ("j", 6.0, 7.0),
            ("k", 7.0, 2.0),
            ("l", 8.0, 1.0),
        ];
        let candidates = specs
            .iter()
            .map(|(id, cost, coverage)| {
                candidate(
                    id,
                    VerificationStatus::Verified,
                    two_objectives(*cost, *coverage),
                )
            })
            .collect::<Vec<_>>();
        let report = compute_pareto_frontier(&candidates).expect("valid adversarial fixture");

        assert_eq!(report.frontier_ids, pairwise_frontier_ids(&candidates));

        for result in &report.candidates {
            if let CandidateDisposition::DominatedBy(witness_id) = &result.disposition {
                let witness = candidates
                    .iter()
                    .find(|candidate| &candidate.candidate_id == witness_id)
                    .expect("reported witness must be a candidate");
                let subject = candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == result.candidate_id)
                    .expect("reported subject must be a candidate");
                let mut witness_objectives = witness.objectives.iter().collect::<Vec<_>>();
                witness_objectives.sort_by_key(|objective| objective.dimension.clone());
                let mut subject_objectives = subject.objectives.iter().collect::<Vec<_>>();
                subject_objectives.sort_by_key(|objective| objective.dimension.clone());
                assert!(dominates(
                    &RankableCandidate {
                        candidate: witness,
                        objectives: witness_objectives,
                    },
                    &RankableCandidate {
                        candidate: subject,
                        objectives: subject_objectives,
                    }
                ));
            }
        }
    }

    #[test]
    fn two_dimensional_sweep_matches_pairwise_on_generated_grid() {
        let candidates = (0..257)
            .map(|index| {
                let cost = ((index * 37) % 101) as f64;
                let coverage = ((index * 53) % 97) as f64;
                candidate(
                    &format!("generated-{index:03}"),
                    VerificationStatus::Verified,
                    two_objectives(cost, coverage),
                )
            })
            .collect::<Vec<_>>();
        let report = compute_pareto_frontier(&candidates).expect("generated schema is valid");
        assert_eq!(report.frontier_ids, pairwise_frontier_ids(&candidates));
    }

    #[test]
    fn higher_dimensional_inputs_use_pairwise_dominance() {
        let triple = |id: &str, cost: f64, area: f64, coverage: f64| {
            candidate(
                id,
                VerificationStatus::Verified,
                vec![
                    objective_in_unit("install_cost", "USD", ObjectiveDirection::Minimize, cost),
                    objective_in_unit("occupied_area", "m2", ObjectiveDirection::Minimize, area),
                    objective_in_unit(
                        "task_coverage",
                        "ratio",
                        ObjectiveDirection::Maximize,
                        coverage,
                    ),
                ],
            )
        };
        let candidates = vec![
            triple("a", 10.0, 20.0, 80.0),
            triple("b", 12.0, 15.0, 70.0),
            triple("c", 11.0, 22.0, 75.0),
            triple("d", 11.0, 25.0, 85.0),
            triple("e", 13.0, 30.0, 60.0),
        ];
        let report =
            compute_pareto_frontier(&candidates).expect("three-dimensional schema is valid");
        assert_eq!(report.frontier_ids, vec!["a", "b", "d"]);
        assert_eq!(report.frontier_ids, pairwise_frontier_ids(&candidates));
    }

    #[test]
    fn stale_verifier_receipt_cannot_rank_an_edited_revision() {
        let mut stale = candidate(
            "edited",
            VerificationStatus::Verified,
            two_objectives(1.0, 100.0),
        );
        let edited_revision = "c".repeat(64);
        let receipt_revision = "a".repeat(64);
        stale.revision_sha256 = Some(edited_revision.clone());
        // The receipt still refers to the prior candidate revision.
        stale.verified_revision_sha256 = Some(receipt_revision.clone());
        let current = candidate(
            "current",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let report = compute_pareto_frontier(&[stale, current])
            .expect("stale binding is an explicit exclusion, not a panic");
        assert_eq!(report.frontier_ids, vec!["current"]);
        assert!(!report.ranking_complete);
        assert!(matches!(
            disposition(&report, "edited"),
            CandidateDisposition::ExcludedUnrankable(reasons)
                if reasons.iter().any(|reason| reason.contains("different candidate revision"))
        ));
        let edited = report
            .candidates
            .iter()
            .find(|result| result.candidate_id == "edited")
            .expect("excluded candidate remains in the report");
        assert_eq!(
            edited.revision_sha256.as_deref(),
            Some(edited_revision.as_str())
        );
        assert_eq!(
            edited.verified_revision_sha256.as_deref(),
            Some(receipt_revision.as_str())
        );
    }

    #[test]
    fn incompatible_units_fail_closed() {
        let usd = candidate(
            "usd",
            VerificationStatus::Verified,
            vec![objective_in_unit(
                "cost",
                "USD",
                ObjectiveDirection::Minimize,
                10.0,
            )],
        );
        let zar = candidate(
            "zar",
            VerificationStatus::Verified,
            vec![objective_in_unit(
                "cost",
                "ZAR",
                ObjectiveDirection::Minimize,
                190.0,
            )],
        );
        assert!(matches!(
            compute_pareto_frontier(&[usd, zar]),
            Err(FrontierError::IncompatibleObjectiveSchemas { .. })
        ));
    }

    #[test]
    fn tied_alternatives_are_both_preserved_on_frontier() {
        let a = candidate(
            "a",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let b = candidate(
            "b",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let report = compute_pareto_frontier(&[b, a]).expect("valid frontier");
        assert_eq!(report.frontier_ids, vec!["a", "b"]);
    }

    #[test]
    fn malformed_digest_is_not_eligible_for_ranking() {
        let mut malformed = candidate(
            "bad-digest",
            VerificationStatus::Verified,
            two_objectives(1.0, 100.0),
        );
        malformed.revision_sha256 = Some("not-a-digest".to_owned());
        let good = candidate(
            "good",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let report =
            compute_pareto_frontier(&[malformed, good]).expect("unrankable candidate is reported");
        assert_eq!(report.frontier_ids, vec!["good"]);
        assert!(!report.ranking_complete);
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let a = candidate(
            "duplicate",
            VerificationStatus::Verified,
            two_objectives(10.0, 70.0),
        );
        let b = candidate("duplicate", VerificationStatus::Blocked, Vec::new());
        assert_eq!(
            compute_pareto_frontier(&[a, b]),
            Err(FrontierError::DuplicateCandidateId("duplicate".to_owned()))
        );
    }

    #[test]
    fn non_finite_objective_is_never_ranked() {
        let bad = candidate(
            "non-finite",
            VerificationStatus::Verified,
            vec![objective("cost", ObjectiveDirection::Minimize, f64::NAN)],
        );
        let report =
            compute_pareto_frontier(&[bad]).expect("bad objective remains an explicit exclusion");
        assert!(report.frontier_ids.is_empty());
        assert!(!report.ranking_complete);
        assert!(matches!(
            disposition(&report, "non-finite"),
            CandidateDisposition::ExcludedUnrankable(_)
        ));
    }

    #[test]
    fn empty_request_and_invalid_candidate_ids_are_rejected() {
        assert_eq!(
            compute_pareto_frontier(&[]),
            Err(FrontierError::EmptyRequest)
        );
        let bad = candidate(" padded ", VerificationStatus::Blocked, Vec::new());
        assert!(matches!(
            compute_pareto_frontier(&[bad]),
            Err(FrontierError::InvalidCandidateId { index: 0 })
        ));
    }

    #[test]
    fn no_verified_candidates_produce_no_frontier() {
        let input = vec![candidate(
            "blocked",
            VerificationStatus::Blocked,
            Vec::new(),
        )];
        let report = compute_pareto_frontier(&input).expect("exclusion report");
        assert!(report.frontier_ids.is_empty());
        assert!(!report.ranking_complete);
    }
}

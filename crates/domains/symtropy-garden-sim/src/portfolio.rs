// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact, deterministic Pareto comparison for already-assessed garden/food
//! portfolio candidates.
//!
//! This module compares declared objective values; it does not calculate crop
//! yields, establish source truth, or replace resource, nutrition, safety, land,
//! rights, or community-consent gates. A candidate can enter the frontier only
//! when the caller declares it feasible and supplies evidence references. Such
//! references are identifiers, not authentication.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;

pub const GARDEN_PORTFOLIO_SCHEMA_VERSION: u16 = 1;

fn serialize_i128_as_decimal_string<S>(
    value: &i128,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_i128_from_decimal_string<'de, D>(
    deserializer: D,
) -> Result<i128, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    raw.parse::<i128>().map_err(serde::de::Error::custom)
}

fn serialize_u64_as_decimal_string<S>(
    value: &u64,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64_from_decimal_string<'de, D>(
    deserializer: D,
) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    raw.parse::<u64>().map_err(serde::de::Error::custom)
}

/// Direction is explicit for each objective; there is no hidden composite score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioObjectiveDirectionV1 {
    Minimize,
    Maximize,
}

/// A dimension whose unit and integer scale must match across every candidate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioObjectiveDefinitionV1 {
    pub objective_id: String,
    pub direction: PortfolioObjectiveDirectionV1,
    pub unit: String,
    #[serde(
        serialize_with = "serialize_u64_as_decimal_string",
        deserialize_with = "deserialize_u64_from_decimal_string"
    )]
    pub value_scale: u64,
}

/// Exact scaled integer for one objective. Values are decimal strings on the JSON
/// wire so Rust integer precision survives clients based on IEEE-754 numbers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioObjectiveValueV1 {
    pub objective_id: String,
    pub unit: String,
    #[serde(
        serialize_with = "serialize_u64_as_decimal_string",
        deserialize_with = "deserialize_u64_from_decimal_string"
    )]
    pub value_scale: u64,
    #[serde(
        serialize_with = "serialize_i128_as_decimal_string",
        deserialize_with = "deserialize_i128_from_decimal_string"
    )]
    pub value_scaled: i128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioCandidateFeasibilityV1 {
    Feasible,
    Infeasible,
    Unassessed,
}

/// A candidate passed to this comparer. Feasibility must come from upstream checks;
/// this comparer deliberately does not infer it from objective values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioCandidateV1 {
    pub candidate_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub feasibility: PortfolioCandidateFeasibilityV1,
    /// References to the hard constraint/evidence findings for infeasible or
    /// unassessed candidates. Feasible candidates must have no blocker references.
    pub blocker_refs: Vec<String>,
    /// Traceability references for the assessed candidate. These do not
    /// authenticate the referenced records.
    pub evidence_refs: Vec<String>,
    /// Feasible candidates require exactly one value per declared objective.
    /// Non-feasible candidates must not include scoreable objective values.
    pub objective_values: Vec<PortfolioObjectiveValueV1>,
}

/// A complete comparison request binds scope and time so incomparable scenarios
/// cannot be silently mixed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioComparisonRequestV1 {
    pub schema_version: u16,
    pub comparison_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub objectives: Vec<PortfolioObjectiveDefinitionV1>,
    pub candidates: Vec<PortfolioCandidateV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioComparisonStatusV1 {
    Compared,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioComparisonBlockerCodeV1 {
    UnsupportedSchema,
    EmptyComparisonId,
    EmptyScope,
    EmptyPeriod,
    NoObjectives,
    NoCandidates,
    EmptyObjectiveId,
    DuplicateObjectiveId,
    EmptyObjectiveUnit,
    ZeroObjectiveScale,
    EmptyCandidateId,
    DuplicateCandidateId,
    CandidateScopeMismatch,
    CandidatePeriodMismatch,
    FeasibleCandidateHasBlockers,
    CandidateMissingBlocker,
    CandidateMissingEvidence,
    BlankEvidenceReference,
    NonFeasibleCandidateHasObjectiveValues,
    MissingObjectiveValue,
    DuplicateCandidateObjective,
    UnexpectedCandidateObjective,
    EmptyCandidateObjectiveUnit,
    CandidateObjectiveUnitMismatch,
    CandidateObjectiveScaleMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioComparisonBlockerV1 {
    pub code: PortfolioComparisonBlockerCodeV1,
    pub candidate_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioCandidateExclusionV1 {
    pub candidate_id: String,
    pub feasibility: PortfolioCandidateFeasibilityV1,
    pub blocker_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioDominanceWitnessV1 {
    /// Candidate that is dominated.
    pub candidate_id: String,
    /// Lexicographically first candidate that strictly dominates it.
    pub dominated_by_candidate_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioComparisonReportV1 {
    pub schema_version: u16,
    pub comparison_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub status: PortfolioComparisonStatusV1,
    /// Sorted by candidate ID. Equal objective vectors remain separate alternatives.
    pub frontier_candidate_ids: Vec<String>,
    pub dominated_candidate_ids: Vec<String>,
    pub dominance_witnesses: Vec<PortfolioDominanceWitnessV1>,
    /// Infeasible and unassessed candidates are never scored or put on the frontier.
    pub excluded_candidates: Vec<PortfolioCandidateExclusionV1>,
    pub blockers: Vec<PortfolioComparisonBlockerV1>,
}

struct PreparedCandidate<'a> {
    candidate: &'a PortfolioCandidateV1,
    values: Vec<i128>,
}

fn add_blocker(
    blockers: &mut Vec<PortfolioComparisonBlockerV1>,
    code: PortfolioComparisonBlockerCodeV1,
    candidate_id: Option<&str>,
    detail: &str,
) {
    blockers.push(PortfolioComparisonBlockerV1 {
        code,
        candidate_id: candidate_id.map(str::to_owned),
        detail: detail.to_owned(),
    });
}

fn sort_blockers(blockers: &mut [PortfolioComparisonBlockerV1]) {
    blockers.sort_by(|left, right| {
        left.candidate_id
            .cmp(&right.candidate_id)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.detail.cmp(&right.detail))
    });
}

fn non_blank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn dominates(
    left: &[i128],
    right: &[i128],
    objectives: &[&PortfolioObjectiveDefinitionV1],
) -> bool {
    if left.len() != right.len() || left.len() != objectives.len() {
        return false;
    }

    let mut strictly_better = false;
    for ((left_value, right_value), objective) in
        left.iter().zip(right.iter()).zip(objectives.iter())
    {
        let ordering = left_value.cmp(right_value);
        let no_worse = match objective.direction {
            PortfolioObjectiveDirectionV1::Minimize => ordering != Ordering::Greater,
            PortfolioObjectiveDirectionV1::Maximize => ordering != Ordering::Less,
        };
        if !no_worse {
            return false;
        }
        if ordering != Ordering::Equal {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Compare pre-assessed candidates with exact integer dominance and deterministic
/// output ordering. Any malformed request blocks the entire comparison rather than
/// silently dropping invalid objective dimensions or candidate rows.
///
/// Scope/period, units, scales and objective IDs must match exactly. Hard constraint
/// results are supplied as feasibility + blocker references; this function is not
/// a substitute for validating those results at their canonical owners.
pub fn compare_portfolios_v1(
    request: &PortfolioComparisonRequestV1,
) -> PortfolioComparisonReportV1 {
    let mut blockers = Vec::new();
    let mut objectives: Vec<&PortfolioObjectiveDefinitionV1> =
        request.objectives.iter().collect();
    objectives.sort_by(|left, right| left.objective_id.cmp(&right.objective_id));

    if request.schema_version != GARDEN_PORTFOLIO_SCHEMA_VERSION {
        add_blocker(
            &mut blockers,
            PortfolioComparisonBlockerCodeV1::UnsupportedSchema,
            None,
            "unsupported portfolio-comparison schema version",
        );
    }
    if !non_blank(&request.comparison_id) {
        add_blocker(
            &mut blockers,
            PortfolioComparisonBlockerCodeV1::EmptyComparisonId,
            None,
            "comparison identity is empty",
        );
    }
    if !non_blank(&request.scope_ref) {
        add_blocker(
            &mut blockers,
            PortfolioComparisonBlockerCodeV1::EmptyScope,
            None,
            "comparison scope reference is empty",
        );
    }
    if !non_blank(&request.period_ref) {
        add_blocker(
            &mut blockers,
            PortfolioComparisonBlockerCodeV1::EmptyPeriod,
            None,
            "comparison period reference is empty",
        );
    }
    if objectives.is_empty() {
        add_blocker(
            &mut blockers,
            PortfolioComparisonBlockerCodeV1::NoObjectives,
            None,
            "at least one objective definition is required",
        );
    }
    if request.candidates.is_empty() {
        add_blocker(
            &mut blockers,
            PortfolioComparisonBlockerCodeV1::NoCandidates,
            None,
            "at least one candidate is required",
        );
    }

    for objective in &objectives {
        if !non_blank(&objective.objective_id) {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::EmptyObjectiveId,
                None,
                "objective identity is empty",
            );
        }
        if !non_blank(&objective.unit) {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::EmptyObjectiveUnit,
                None,
                "objective unit is empty",
            );
        }
        if objective.value_scale == 0 {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::ZeroObjectiveScale,
                None,
                "objective value scale must be greater than zero",
            );
        }
    }
    for pair in objectives.windows(2) {
        if pair[0].objective_id == pair[1].objective_id {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::DuplicateObjectiveId,
                None,
                "objective IDs must be unique",
            );
        }
    }

    let mut candidates: Vec<&PortfolioCandidateV1> = request.candidates.iter().collect();
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    for candidate in &candidates {
        if !non_blank(&candidate.candidate_id) {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::EmptyCandidateId,
                Some(candidate.candidate_id.as_str()),
                "candidate identity is empty",
            );
        }
        if candidate.scope_ref != request.scope_ref {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::CandidateScopeMismatch,
                Some(candidate.candidate_id.as_str()),
                "candidate scope does not match comparison scope",
            );
        }
        if candidate.period_ref != request.period_ref {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::CandidatePeriodMismatch,
                Some(candidate.candidate_id.as_str()),
                "candidate period does not match comparison period",
            );
        }

        match candidate.feasibility {
            PortfolioCandidateFeasibilityV1::Feasible => {
                if !candidate.blocker_refs.is_empty() {
                    add_blocker(
                        &mut blockers,
                        PortfolioComparisonBlockerCodeV1::FeasibleCandidateHasBlockers,
                        Some(candidate.candidate_id.as_str()),
                        "a feasible candidate cannot carry unresolved hard-blocker references",
                    );
                }
                if candidate.evidence_refs.is_empty() {
                    add_blocker(
                        &mut blockers,
                        PortfolioComparisonBlockerCodeV1::CandidateMissingEvidence,
                        Some(candidate.candidate_id.as_str()),
                        "a feasible candidate requires evidence references",
                    );
                }
                if candidate.evidence_refs.iter().any(|item| !non_blank(item)) {
                    add_blocker(
                        &mut blockers,
                        PortfolioComparisonBlockerCodeV1::BlankEvidenceReference,
                        Some(candidate.candidate_id.as_str()),
                        "evidence references must not be blank",
                    );
                }

                let mut values: Vec<&PortfolioObjectiveValueV1> =
                    candidate.objective_values.iter().collect();
                values.sort_by(|left, right| left.objective_id.cmp(&right.objective_id));
                for pair in values.windows(2) {
                    if pair[0].objective_id == pair[1].objective_id {
                        add_blocker(
                            &mut blockers,
                            PortfolioComparisonBlockerCodeV1::DuplicateCandidateObjective,
                            Some(candidate.candidate_id.as_str()),
                            "candidate objective IDs must be unique",
                        );
                    }
                }

                for objective in &objectives {
                    let matching = values
                        .iter()
                        .filter(|value| value.objective_id == objective.objective_id)
                        .count();
                    if matching == 0 {
                        add_blocker(
                            &mut blockers,
                            PortfolioComparisonBlockerCodeV1::MissingObjectiveValue,
                            Some(candidate.candidate_id.as_str()),
                            &format!("candidate is missing objective {}", objective.objective_id),
                        );
                    }
                }

                for value in &values {
                    let Some(objective) = objectives
                        .iter()
                        .find(|objective| objective.objective_id == value.objective_id)
                    else {
                        add_blocker(
                            &mut blockers,
                            PortfolioComparisonBlockerCodeV1::UnexpectedCandidateObjective,
                            Some(candidate.candidate_id.as_str()),
                            &format!(
                                "candidate supplied undeclared objective {}",
                                value.objective_id
                            ),
                        );
                        continue;
                    };
                    if !non_blank(&value.unit) {
                        add_blocker(
                            &mut blockers,
                            PortfolioComparisonBlockerCodeV1::EmptyCandidateObjectiveUnit,
                            Some(candidate.candidate_id.as_str()),
                            &format!("objective {} has an empty unit", value.objective_id),
                        );
                    }
                    if value.unit != objective.unit {
                        add_blocker(
                            &mut blockers,
                            PortfolioComparisonBlockerCodeV1::CandidateObjectiveUnitMismatch,
                            Some(candidate.candidate_id.as_str()),
                            &format!(
                                "objective {} unit does not match its definition",
                                value.objective_id
                            ),
                        );
                    }
                    if value.value_scale != objective.value_scale {
                        add_blocker(
                            &mut blockers,
                            PortfolioComparisonBlockerCodeV1::CandidateObjectiveScaleMismatch,
                            Some(candidate.candidate_id.as_str()),
                            &format!(
                                "objective {} scale does not match its definition",
                                value.objective_id
                            ),
                        );
                    }
                }
            }
            PortfolioCandidateFeasibilityV1::Infeasible
            | PortfolioCandidateFeasibilityV1::Unassessed => {
                if candidate.blocker_refs.is_empty()
                    || candidate.blocker_refs.iter().any(|item| !non_blank(item))
                {
                    add_blocker(
                        &mut blockers,
                        PortfolioComparisonBlockerCodeV1::CandidateMissingBlocker,
                        Some(candidate.candidate_id.as_str()),
                        "infeasible and unassessed candidates require non-empty blocker references",
                    );
                }
                if !candidate.objective_values.is_empty() {
                    add_blocker(
                        &mut blockers,
                        PortfolioComparisonBlockerCodeV1::NonFeasibleCandidateHasObjectiveValues,
                        Some(candidate.candidate_id.as_str()),
                        "a non-feasible candidate must not provide scoreable objective values",
                    );
                }
            }
        }
    }

    for pair in candidates.windows(2) {
        if pair[0].candidate_id == pair[1].candidate_id {
            add_blocker(
                &mut blockers,
                PortfolioComparisonBlockerCodeV1::DuplicateCandidateId,
                Some(&pair[0].candidate_id),
                "candidate IDs must be unique",
            );
        }
    }

    sort_blockers(&mut blockers);
    if !blockers.is_empty() {
        return PortfolioComparisonReportV1 {
            schema_version: GARDEN_PORTFOLIO_SCHEMA_VERSION,
            comparison_id: request.comparison_id.clone(),
            scope_ref: request.scope_ref.clone(),
            period_ref: request.period_ref.clone(),
            status: PortfolioComparisonStatusV1::Blocked,
            frontier_candidate_ids: Vec::new(),
            dominated_candidate_ids: Vec::new(),
            dominance_witnesses: Vec::new(),
            excluded_candidates: Vec::new(),
            blockers,
        };
    }

    let mut prepared = Vec::new();
    let mut excluded_candidates = Vec::new();
    for candidate in candidates {
        match candidate.feasibility {
            PortfolioCandidateFeasibilityV1::Feasible => {
                let mut values: Vec<_> = candidate.objective_values.iter().collect();
                values.sort_by(|left, right| left.objective_id.cmp(&right.objective_id));
                prepared.push(PreparedCandidate {
                    candidate,
                    values: values.iter().map(|value| value.value_scaled).collect(),
                });
            }
            feasibility => {
                let mut blocker_refs = candidate.blocker_refs.clone();
                blocker_refs.sort();
                excluded_candidates.push(PortfolioCandidateExclusionV1 {
                    candidate_id: candidate.candidate_id.clone(),
                    feasibility,
                    blocker_refs,
                });
            }
        }
    }
    excluded_candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let mut frontier_candidate_ids = Vec::new();
    let mut dominated_candidate_ids = Vec::new();
    let mut dominance_witnesses = Vec::new();

    for candidate in &prepared {
        let witness = prepared
            .iter()
            .filter(|other| other.candidate.candidate_id != candidate.candidate.candidate_id)
            .filter(|other| dominates(&other.values, &candidate.values, &objectives))
            .map(|other| other.candidate.candidate_id.as_str())
            .min();

        if let Some(dominated_by_candidate_id) = witness {
            dominated_candidate_ids.push(candidate.candidate.candidate_id.clone());
            dominance_witnesses.push(PortfolioDominanceWitnessV1 {
                candidate_id: candidate.candidate.candidate_id.clone(),
                dominated_by_candidate_id: dominated_by_candidate_id.to_owned(),
            });
        } else {
            frontier_candidate_ids.push(candidate.candidate.candidate_id.clone());
        }
    }

    PortfolioComparisonReportV1 {
        schema_version: GARDEN_PORTFOLIO_SCHEMA_VERSION,
        comparison_id: request.comparison_id.clone(),
        scope_ref: request.scope_ref.clone(),
        period_ref: request.period_ref.clone(),
        status: PortfolioComparisonStatusV1::Compared,
        frontier_candidate_ids,
        dominated_candidate_ids,
        dominance_witnesses,
        excluded_candidates,
        blockers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective(
        id: &str,
        direction: PortfolioObjectiveDirectionV1,
        unit: &str,
    ) -> PortfolioObjectiveDefinitionV1 {
        PortfolioObjectiveDefinitionV1 {
            objective_id: id.to_owned(),
            direction,
            unit: unit.to_owned(),
            value_scale: 1,
        }
    }

    fn value(id: &str, unit: &str, value_scaled: i128) -> PortfolioObjectiveValueV1 {
        PortfolioObjectiveValueV1 {
            objective_id: id.to_owned(),
            unit: unit.to_owned(),
            value_scale: 1,
            value_scaled,
        }
    }

    fn candidate(
        id: &str,
        feasibility: PortfolioCandidateFeasibilityV1,
        blocker_refs: Vec<&str>,
        objectives: Vec<PortfolioObjectiveValueV1>,
    ) -> PortfolioCandidateV1 {
        PortfolioCandidateV1 {
            candidate_id: id.to_owned(),
            scope_ref: "scope:community-1".to_owned(),
            period_ref: "period:2026-11".to_owned(),
            feasibility,
            blocker_refs: blocker_refs.into_iter().map(str::to_owned).collect(),
            evidence_refs: if feasibility == PortfolioCandidateFeasibilityV1::Feasible {
                vec![format!("fixture:evidence:{id}")]
            } else {
                Vec::new()
            },
            objective_values: objectives,
        }
    }

    fn valid_request() -> PortfolioComparisonRequestV1 {
        PortfolioComparisonRequestV1 {
            schema_version: GARDEN_PORTFOLIO_SCHEMA_VERSION,
            comparison_id: "comparison:synthetic-001".to_owned(),
            scope_ref: "scope:community-1".to_owned(),
            period_ref: "period:2026-11".to_owned(),
            objectives: vec![
                objective("water_ml", PortfolioObjectiveDirectionV1::Minimize, "ml"),
                objective("edible_grams", PortfolioObjectiveDirectionV1::Maximize, "g"),
            ],
            candidates: vec![
                candidate(
                    "A",
                    PortfolioCandidateFeasibilityV1::Feasible,
                    Vec::new(),
                    vec![value("water_ml", "ml", 4), value("edible_grams", "g", 10)],
                ),
                candidate(
                    "B",
                    PortfolioCandidateFeasibilityV1::Feasible,
                    Vec::new(),
                    vec![value("water_ml", "ml", 6), value("edible_grams", "g", 12)],
                ),
                candidate(
                    "C",
                    PortfolioCandidateFeasibilityV1::Feasible,
                    Vec::new(),
                    vec![value("water_ml", "ml", 5), value("edible_grams", "g", 9)],
                ),
                candidate(
                    "D",
                    PortfolioCandidateFeasibilityV1::Feasible,
                    Vec::new(),
                    vec![value("water_ml", "ml", 3), value("edible_grams", "g", 8)],
                ),
                candidate(
                    "Z-UNSAFE",
                    PortfolioCandidateFeasibilityV1::Infeasible,
                    vec!["constraint:water-limit"],
                    Vec::new(),
                ),
                candidate(
                    "Z-UNKNOWN",
                    PortfolioCandidateFeasibilityV1::Unassessed,
                    vec!["evidence:missing-site-water"],
                    Vec::new(),
                ),
            ],
        }
    }

    #[test]
    fn frontier_matches_hand_calculated_exact_oracle() {
        let report = compare_portfolios_v1(&valid_request());
        assert_eq!(report.status, PortfolioComparisonStatusV1::Compared);
        assert_eq!(report.frontier_candidate_ids, vec!["A", "B", "D"]);
        assert_eq!(report.dominated_candidate_ids, vec!["C"]);
        assert_eq!(
            report.dominance_witnesses,
            vec![PortfolioDominanceWitnessV1 {
                candidate_id: "C".to_owned(),
                dominated_by_candidate_id: "A".to_owned(),
            }]
        );
        assert_eq!(report.excluded_candidates.len(), 2);
        assert_eq!(report.excluded_candidates[0].candidate_id, "Z-UNKNOWN");
        assert_eq!(report.excluded_candidates[1].candidate_id, "Z-UNSAFE");
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn spectacular_objectives_cannot_rescue_an_infeasible_candidate() {
        let mut request = valid_request();
        let mut unsafe_candidate = request.candidates[4].clone();
        unsafe_candidate.objective_values = vec![
            value("water_ml", "ml", i128::MIN),
            value("edible_grams", "g", i128::MAX),
        ];
        request.candidates[4] = unsafe_candidate;
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.status, PortfolioComparisonStatusV1::Blocked);
        assert!(report.frontier_candidate_ids.is_empty());
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::NonFeasibleCandidateHasObjectiveValues
        }));
    }

    #[test]
    fn infeasible_and_unassessed_candidates_never_enter_frontier() {
        let report = compare_portfolios_v1(&valid_request());
        assert!(!report.frontier_candidate_ids.contains(&"Z-UNSAFE".to_owned()));
        assert!(!report.frontier_candidate_ids.contains(&"Z-UNKNOWN".to_owned()));
        assert!(report.excluded_candidates.iter().all(|item| {
            item.feasibility != PortfolioCandidateFeasibilityV1::Feasible
        }));
    }

    #[test]
    fn equal_objective_vectors_remain_distinct_nondominated_alternatives() {
        let mut request = valid_request();
        request.candidates = vec![
            candidate(
                "SAME-A",
                PortfolioCandidateFeasibilityV1::Feasible,
                Vec::new(),
                vec![value("water_ml", "ml", 5), value("edible_grams", "g", 7)],
            ),
            candidate(
                "SAME-B",
                PortfolioCandidateFeasibilityV1::Feasible,
                Vec::new(),
                vec![value("water_ml", "ml", 5), value("edible_grams", "g", 7)],
            ),
        ];
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.frontier_candidate_ids, vec!["SAME-A", "SAME-B"]);
        assert!(report.dominated_candidate_ids.is_empty());
    }

    #[test]
    fn scope_and_period_mismatches_fail_closed() {
        let mut request = valid_request();
        request.candidates[0].scope_ref = "scope:other".to_owned();
        request.candidates[1].period_ref = "period:2026-12".to_owned();
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.status, PortfolioComparisonStatusV1::Blocked);
        assert!(report.frontier_candidate_ids.is_empty());
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::CandidateScopeMismatch
        }));
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::CandidatePeriodMismatch
        }));
    }

    #[test]
    fn unit_scale_and_missing_dimensions_fail_closed() {
        let mut request = valid_request();
        request.candidates[0].objective_values[0].unit = "litres".to_owned();
        request.candidates[1].objective_values.pop();
        request.candidates[2].objective_values[1].value_scale = 100;
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.status, PortfolioComparisonStatusV1::Blocked);
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::CandidateObjectiveUnitMismatch
        }));
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::MissingObjectiveValue
        }));
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::CandidateObjectiveScaleMismatch
        }));
    }

    #[test]
    fn duplicate_candidate_or_objective_ids_fail_closed() {
        let mut request = valid_request();
        request.candidates.push(request.candidates[0].clone());
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.status, PortfolioComparisonStatusV1::Blocked);
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::DuplicateCandidateId
        }));

        let mut request = valid_request();
        request.objectives.push(request.objectives[0].clone());
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.status, PortfolioComparisonStatusV1::Blocked);
        assert!(report.blockers.iter().any(|item| {
            item.code == PortfolioComparisonBlockerCodeV1::DuplicateObjectiveId
        }));
    }

    fn generated_request(seed: u64, case_index: usize) -> PortfolioComparisonRequestV1 {
        let mut state = seed;
        let objectives = vec![
            objective("cost_minor_units", PortfolioObjectiveDirectionV1::Minimize, "minor_unit"),
            objective("water_ml", PortfolioObjectiveDirectionV1::Minimize, "ml"),
            objective("edible_grams", PortfolioObjectiveDirectionV1::Maximize, "g"),
            objective("diversity_groups", PortfolioObjectiveDirectionV1::Maximize, "group"),
        ];
        let mut candidates = Vec::new();
        for index in 0..12 {
            let id = format!("CASE-{case_index:03}-CAND-{index:02}");
            let feasibility = if index == 0 {
                PortfolioCandidateFeasibilityV1::Unassessed
            } else if index % 7 == 0 {
                PortfolioCandidateFeasibilityV1::Infeasible
            } else {
                PortfolioCandidateFeasibilityV1::Feasible
            };
            let blocker_refs = match feasibility {
                PortfolioCandidateFeasibilityV1::Feasible => Vec::new(),
                PortfolioCandidateFeasibilityV1::Infeasible => {
                    vec![format!("constraint:{id}")]
                }
                PortfolioCandidateFeasibilityV1::Unassessed => {
                    vec![format!("missing-evidence:{id}")]
                }
            };
            let values = if feasibility == PortfolioCandidateFeasibilityV1::Feasible {
                vec![
                    value(
                        "cost_minor_units",
                        "minor_unit",
                        i128::from(next_random(&mut state) % 10_001),
                    ),
                    value("water_ml", "ml", i128::from(next_random(&mut state) % 10_001)),
                    value("edible_grams", "g", i128::from(next_random(&mut state) % 10_001)),
                    value("diversity_groups", "group", i128::from(next_random(&mut state) % 101)),
                ]
            } else {
                Vec::new()
            };
            candidates.push(candidate(
                &id,
                feasibility,
                blocker_refs.iter().map(String::as_str).collect(),
                values,
            ));
        }
        PortfolioComparisonRequestV1 {
            schema_version: GARDEN_PORTFOLIO_SCHEMA_VERSION,
            comparison_id: format!("generated-comparison-{case_index:03}"),
            scope_ref: "scope:community-1".to_owned(),
            period_ref: "period:2026-11".to_owned(),
            objectives,
            candidates,
        }
    }

    fn next_random(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    /// Deliberately separate oracle: enumerates every feasible pair and reads axes
    /// by ID instead of using the production prepared-vector/dominance helper.
    fn reference_frontier(
        request: &PortfolioComparisonRequestV1,
    ) -> (Vec<String>, Vec<String>, Vec<PortfolioDominanceWitnessV1>) {
        let mut feasible: Vec<_> = request
            .candidates
            .iter()
            .filter(|candidate| candidate.feasibility == PortfolioCandidateFeasibilityV1::Feasible)
            .collect();
        feasible.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

        let mut frontier = Vec::new();
        let mut dominated = Vec::new();
        let mut witnesses = Vec::new();
        for target in &feasible {
            let mut dominators = Vec::new();
            for challenger in &feasible {
                if challenger.candidate_id == target.candidate_id {
                    continue;
                }
                let mut no_worse_on_every_axis = true;
                let mut strictly_better_on_an_axis = false;
                for axis in &request.objectives {
                    let challenger_value = challenger
                        .objective_values
                        .iter()
                        .find(|value| value.objective_id == axis.objective_id)
                        .expect("generated feasible candidate supplies every axis")
                        .value_scaled;
                    let target_value = target
                        .objective_values
                        .iter()
                        .find(|value| value.objective_id == axis.objective_id)
                        .expect("generated feasible candidate supplies every axis")
                        .value_scaled;
                    let wins_axis = match axis.direction {
                        PortfolioObjectiveDirectionV1::Minimize => challenger_value <= target_value,
                        PortfolioObjectiveDirectionV1::Maximize => challenger_value >= target_value,
                    };
                    if !wins_axis {
                        no_worse_on_every_axis = false;
                        break;
                    }
                    if challenger_value != target_value {
                        strictly_better_on_an_axis = true;
                    }
                }
                if no_worse_on_every_axis && strictly_better_on_an_axis {
                    dominators.push(challenger.candidate_id.clone());
                }
            }
            if dominators.is_empty() {
                frontier.push(target.candidate_id.clone());
            } else {
                dominators.sort();
                dominated.push(target.candidate_id.clone());
                witnesses.push(PortfolioDominanceWitnessV1 {
                    candidate_id: target.candidate_id.clone(),
                    dominated_by_candidate_id: dominators[0].clone(),
                });
            }
        }
        (frontier, dominated, witnesses)
    }

    #[test]
    fn one_hundred_seeded_scenarios_match_independent_exhaustive_oracle() {
        for case_index in 0..100 {
            let request = generated_request(
                0x5eed_u64.wrapping_add(case_index as u64 * 7919),
                case_index,
            );
            let expected = reference_frontier(&request);
            let actual = compare_portfolios_v1(&request);
            assert_eq!(actual.status, PortfolioComparisonStatusV1::Compared, "case {case_index}");
            assert_eq!(actual.frontier_candidate_ids, expected.0, "frontier case {case_index}");
            assert_eq!(actual.dominated_candidate_ids, expected.1, "dominated case {case_index}");
            assert_eq!(actual.dominance_witnesses, expected.2, "witness case {case_index}");

            let mut expected_exclusions: Vec<_> = request
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate.feasibility != PortfolioCandidateFeasibilityV1::Feasible
                })
                .map(|candidate| candidate.candidate_id.clone())
                .collect();
            expected_exclusions.sort();
            let actual_exclusions: Vec<_> = actual
                .excluded_candidates
                .iter()
                .map(|candidate| candidate.candidate_id.clone())
                .collect();
            assert_eq!(actual_exclusions, expected_exclusions, "exclusions case {case_index}");
        }
    }

    #[test]
    fn strict_dominance_is_irreflexive_and_asymmetric() {
        let definitions = vec![
            objective("cost", PortfolioObjectiveDirectionV1::Minimize, "minor_unit"),
            objective("delivered", PortfolioObjectiveDirectionV1::Maximize, "gram"),
        ];
        let axes: Vec<_> = definitions.iter().collect();
        let better = [1_i128, 9_i128];
        let worse = [2_i128, 8_i128];

        assert!(!dominates(&better, &better, &axes));
        assert!(dominates(&better, &worse, &axes));
        assert!(!dominates(&worse, &better, &axes));
    }

    #[test]
    fn strict_dominance_is_transitive_for_mixed_objective_directions() {
        let definitions = vec![
            objective("cost", PortfolioObjectiveDirectionV1::Minimize, "minor_unit"),
            objective("delivered", PortfolioObjectiveDirectionV1::Maximize, "gram"),
        ];
        let axes: Vec<_> = definitions.iter().collect();
        let a = [1_i128, 9_i128];
        let b = [2_i128, 8_i128];
        let c = [3_i128, 7_i128];

        assert!(dominates(&a, &b, &axes));
        assert!(dominates(&b, &c, &axes));
        assert!(dominates(&a, &c, &axes));
    }

    #[test]
    fn input_order_is_irrelevant_for_semantically_unordered_collections() {
        let expected = compare_portfolios_v1(&valid_request());
        let mut state = 0x5eed_u64;
        for _ in 0..1_000 {
            let mut request = valid_request();
            deterministic_shuffle(&mut request.objectives, &mut state);
            deterministic_shuffle(&mut request.candidates, &mut state);
            for candidate in &mut request.candidates {
                deterministic_shuffle(&mut candidate.objective_values, &mut state);
            }
            assert_eq!(compare_portfolios_v1(&request), expected);
        }
    }

    fn deterministic_shuffle<T>(items: &mut [T], state: &mut u64) {
        for index in (1..items.len()).rev() {
            *state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let other = (*state as usize) % (index + 1);
            items.swap(index, other);
        }
    }

    #[test]
    fn wide_objective_integers_serialize_as_decimal_strings() {
        let mut request = valid_request();
        request.candidates.truncate(1);
        request.candidates[0].objective_values[0].value_scaled = i128::MIN;
        request.objectives[0].value_scale = u64::MAX;
        request.candidates[0].objective_values[0].value_scale = u64::MAX;
        let encoded = serde_json::to_string(&request).expect("request serializes");
        assert!(encoded.contains("\"value_scaled\":\"-170141183460469231731687303715884105728\""));
        assert!(encoded.contains("\"value_scale\":\"18446744073709551615\""));
        let decoded: PortfolioComparisonRequestV1 =
            serde_json::from_str(&encoded).expect("request deserializes");
        assert_eq!(decoded, request);
    }

    #[test]
    fn malformed_request_does_not_emit_partial_frontier() {
        let mut request = valid_request();
        request.comparison_id.clear();
        request.objectives[0].value_scale = 0;
        let report = compare_portfolios_v1(&request);
        assert_eq!(report.status, PortfolioComparisonStatusV1::Blocked);
        assert!(report.frontier_candidate_ids.is_empty());
        assert!(report.dominated_candidate_ids.is_empty());
        assert!(!report.blockers.is_empty());
    }

    #[test]
    fn output_is_deterministic_and_round_trips_through_json() {
        let request = valid_request();
        let first = compare_portfolios_v1(&request);
        let second = compare_portfolios_v1(&request);
        assert_eq!(first, second);
        let encoded = serde_json::to_string(&first).expect("report serializes");
        let decoded: PortfolioComparisonReportV1 =
            serde_json::from_str(&encoded).expect("report deserializes");
        assert_eq!(decoded, first);
    }
}

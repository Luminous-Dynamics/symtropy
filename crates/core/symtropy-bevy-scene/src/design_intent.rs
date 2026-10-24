// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Owner-led design intent and explainable alternative comparison.
//!
//! This model explicitly separates binding requirements, owner-declared hard
//! constraints, preferences, and exploratory goals. It intentionally has no
//! single aggregate "best home" score: incomparable values such as privacy,
//! cultural expression, cost, daylight and spatial character remain visible.
//! It does not solve architecture, validate code, or certify buildability.

use std::collections::{BTreeMap, BTreeSet, HashSet};

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesignIntentClass {
    /// A rule already identified as binding by a reviewed jurisdiction profile.
    /// This module validates provenance presence; it does not establish legal truth.
    BindingRequirement,
    /// An owner-declared non-negotiable project constraint, distinct from law.
    OwnerHardConstraint,
    /// A personal or household preference that should guide exploration.
    OwnerPreference,
    /// A goal to explore and compare, not a pass/fail gate.
    ExplorationGoal,
    /// A declared modelling assumption requiring verification.
    EngineeringAssumption,
    /// A site/context fact not yet verified.
    UnknownSiteConstraint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentPriority {
    Essential,
    Important,
    Flexible,
    Explore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentOrigin {
    OwnerBrief,
    JurisdictionRulePack,
    ProfessionalAssessment,
    VerifiedSiteEvidence,
    UnverifiedSiteObservation,
    DesignTeamAssumption,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignIntent {
    pub intent_id: String,
    pub title: String,
    pub description: String,
    pub class: DesignIntentClass,
    pub priority: IntentPriority,
    pub origin: IntentOrigin,
    /// Required for a binding requirement; must resolve to the exact source
    /// clause in the selected jurisdiction/module edition.
    pub source_ref: Option<String>,
    pub source_module_id: Option<String>,
    /// Explicit owner/decision-maker for owner-declared hard constraints.
    pub decision_owner_id: Option<String>,
    /// Required for assumptions and unknown site constraints.
    pub uncertainty_note: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentEvaluationOutcome {
    Satisfied,
    NotSatisfied,
    /// Preferences are allowed to be unmet without being called a violation.
    PreferenceUnmet,
    Tradeoff,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentEvaluation {
    pub intent_id: String,
    pub outcome: IntentEvaluationOutcome,
    pub explanation: String,
    /// Link to geometry checks, calculation, survey, or other evidence when
    /// the conclusion depends on a measured/verified fact.
    pub evidence_ref: Option<String>,
    pub not_applicable_rationale: Option<String>,
    pub uncertainty_note: Option<String>,
    /// Optional owner-provided 0–100 preference rating. It is subjective input,
    /// valid only for OwnerPreference intent and never for binding requirements.
    pub preference_rating: Option<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlternativeDisposition {
    /// Keep it available for comparison; unresolved issues remain visible.
    OpenForExploration,
    /// No declared hard constraints remain unsatisfied/unknown, so it can move
    /// to independent technical assessment. This is not legal/build approval.
    ReadyForTechnicalAssessment,
    /// At least one hard constraint is known not to be met, with evidence.
    BlockedByHardConstraint,
    /// The owner explicitly declined this option; that is not a code finding.
    OwnerDeclined,
}

/// An immutable comparison snapshot can be pinned to an owner-brief revision.
/// It deliberately has no aggregate ranking; alternatives preserve one explicit
/// evaluation per intent so the owner decides their trade-offs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignComparison {
    pub comparison_id: String,
    pub owner_brief_revision_id: String,
    pub intents: Vec<DesignIntent>,
    pub alternatives: Vec<DesignAlternative>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignAlternative {
    pub candidate_id: String,
    pub design_revision_id: String,
    pub title: String,
    pub disposition: AlternativeDisposition,
    pub decision_owner_id: Option<String>,
    pub disposition_rationale: String,
    /// Exactly one evaluation per declared intent. There is deliberately no
    /// scalar overall score and no automatic ranking of unlike preferences.
    pub evaluations: Vec<IntentEvaluation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesignIntentIssue {
    EmptyIntentId,
    EmptyComparisonId,
    EmptyOwnerBriefRevisionId,
    DuplicateIntentId(String),
    EmptyIntentTitle(String),
    EmptyIntentDescription(String),
    BindingIntentMissingProvenance(String),
    OwnerHardConstraintMissingDecisionOwner(String),
    UncertaintyNotDeclared(String),
    EmptyCandidateId,
    DuplicateCandidateId(String),
    EmptyCandidateRevision(String),
    EmptyCandidateTitle(String),
    EmptyCandidateRationale(String),
    DuplicateEvaluation {
        candidate_id: String,
        intent_id: String,
    },
    UnknownIntent {
        candidate_id: String,
        intent_id: String,
    },
    MissingEvaluation {
        candidate_id: String,
        intent_id: String,
    },
    EmptyEvaluationExplanation {
        candidate_id: String,
        intent_id: String,
    },
    InvalidOutcomeForIntentClass {
        candidate_id: String,
        intent_id: String,
    },
    NotApplicableWithoutRationale {
        candidate_id: String,
        intent_id: String,
    },
    HardConstraintOutcomeLacksEvidence {
        candidate_id: String,
        intent_id: String,
    },
    PreferenceMisclassifiedAsHardViolation {
        candidate_id: String,
        intent_id: String,
    },
    TechnicalAssessmentHasUnresolvedHardConstraint {
        candidate_id: String,
        intent_id: String,
    },
    BlockedWithoutFailedHardConstraint(String),
    BlockedConstraintMissingEvidence(String),
    OwnerDeclinedWithoutOwner(String),
    OwnerDeclinedWithoutRationale(String),
    UnverifiedConclusionClaimedSatisfied {
        candidate_id: String,
        intent_id: String,
    },
    PreferenceRatingOnNonPreference {
        candidate_id: String,
        intent_id: String,
    },
    PreferenceRatingOutOfRange {
        candidate_id: String,
        intent_id: String,
        rating: u8,
    },
}

/// Validate a comparison matrix before showing it to an owner or using it to
/// prune alternatives. This validates completeness and semantics, not the
/// truth of each engineering/legal evaluation or the quality of a design.
pub fn validate_design_comparison(
    intents: &[DesignIntent],
    alternatives: &[DesignAlternative],
) -> Result<(), Vec<DesignIntentIssue>> {
    let mut issues = Vec::new();
    let mut intent_ids = HashSet::new();

    for intent in intents {
        if !nonblank(&intent.intent_id) {
            issues.push(DesignIntentIssue::EmptyIntentId);
        } else if !intent_ids.insert(intent.intent_id.as_str()) {
            issues.push(DesignIntentIssue::DuplicateIntentId(
                intent.intent_id.clone(),
            ));
        }
        if !nonblank(&intent.title) {
            issues.push(DesignIntentIssue::EmptyIntentTitle(
                intent.intent_id.clone(),
            ));
        }
        if !nonblank(&intent.description) {
            issues.push(DesignIntentIssue::EmptyIntentDescription(
                intent.intent_id.clone(),
            ));
        }
        match intent.class {
            DesignIntentClass::BindingRequirement => {
                if !intent.source_ref.as_deref().is_some_and(nonblank)
                    || !intent.source_module_id.as_deref().is_some_and(nonblank)
                {
                    issues.push(DesignIntentIssue::BindingIntentMissingProvenance(
                        intent.intent_id.clone(),
                    ));
                }
            }
            DesignIntentClass::OwnerHardConstraint => {
                if !intent.decision_owner_id.as_deref().is_some_and(nonblank) {
                    issues.push(DesignIntentIssue::OwnerHardConstraintMissingDecisionOwner(
                        intent.intent_id.clone(),
                    ));
                }
            }
            DesignIntentClass::EngineeringAssumption | DesignIntentClass::UnknownSiteConstraint => {
                if !intent.uncertainty_note.as_deref().is_some_and(nonblank) {
                    issues.push(DesignIntentIssue::UncertaintyNotDeclared(
                        intent.intent_id.clone(),
                    ));
                }
            }
            DesignIntentClass::OwnerPreference | DesignIntentClass::ExplorationGoal => {}
        }
    }

    let mut candidate_ids = HashSet::new();
    for candidate in alternatives {
        if !nonblank(&candidate.candidate_id) {
            issues.push(DesignIntentIssue::EmptyCandidateId);
        } else if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            issues.push(DesignIntentIssue::DuplicateCandidateId(
                candidate.candidate_id.clone(),
            ));
        }
        if !nonblank(&candidate.design_revision_id) {
            issues.push(DesignIntentIssue::EmptyCandidateRevision(
                candidate.candidate_id.clone(),
            ));
        }
        if !nonblank(&candidate.title) {
            issues.push(DesignIntentIssue::EmptyCandidateTitle(
                candidate.candidate_id.clone(),
            ));
        }
        if !nonblank(&candidate.disposition_rationale) {
            issues.push(DesignIntentIssue::EmptyCandidateRationale(
                candidate.candidate_id.clone(),
            ));
        }

        let mut seen_evaluations = HashSet::new();
        let mut failed_hard = Vec::new();

        for evaluation in &candidate.evaluations {
            if !intent_ids.contains(evaluation.intent_id.as_str()) {
                issues.push(DesignIntentIssue::UnknownIntent {
                    candidate_id: candidate.candidate_id.clone(),
                    intent_id: evaluation.intent_id.clone(),
                });
                continue;
            }
            if !seen_evaluations.insert(evaluation.intent_id.as_str()) {
                issues.push(DesignIntentIssue::DuplicateEvaluation {
                    candidate_id: candidate.candidate_id.clone(),
                    intent_id: evaluation.intent_id.clone(),
                });
                continue;
            }
            if !nonblank(&evaluation.explanation) {
                issues.push(DesignIntentIssue::EmptyEvaluationExplanation {
                    candidate_id: candidate.candidate_id.clone(),
                    intent_id: evaluation.intent_id.clone(),
                });
            }
            if let Some(rating) = evaluation.preference_rating {
                if rating > 100 {
                    issues.push(DesignIntentIssue::PreferenceRatingOutOfRange {
                        candidate_id: candidate.candidate_id.clone(),
                        intent_id: evaluation.intent_id.clone(),
                        rating,
                    });
                }
                if intents
                    .iter()
                    .find(|intent| intent.intent_id == evaluation.intent_id)
                    .is_some_and(|intent| intent.class != DesignIntentClass::OwnerPreference)
                {
                    issues.push(DesignIntentIssue::PreferenceRatingOnNonPreference {
                        candidate_id: candidate.candidate_id.clone(),
                        intent_id: evaluation.intent_id.clone(),
                    });
                }
            }
            let Some(intent) = intents
                .iter()
                .find(|item| item.intent_id == evaluation.intent_id)
            else {
                continue;
            };

            let hard_class = matches!(
                intent.class,
                DesignIntentClass::BindingRequirement | DesignIntentClass::OwnerHardConstraint
            );
            match intent.class {
                DesignIntentClass::OwnerPreference => {
                    if evaluation.outcome == IntentEvaluationOutcome::NotSatisfied {
                        issues.push(DesignIntentIssue::PreferenceMisclassifiedAsHardViolation {
                            candidate_id: candidate.candidate_id.clone(),
                            intent_id: evaluation.intent_id.clone(),
                        });
                    }
                }
                DesignIntentClass::BindingRequirement | DesignIntentClass::OwnerHardConstraint => {
                    if matches!(
                        evaluation.outcome,
                        IntentEvaluationOutcome::PreferenceUnmet
                            | IntentEvaluationOutcome::Tradeoff
                    ) {
                        issues.push(DesignIntentIssue::InvalidOutcomeForIntentClass {
                            candidate_id: candidate.candidate_id.clone(),
                            intent_id: evaluation.intent_id.clone(),
                        });
                    }
                }
                DesignIntentClass::ExplorationGoal => {
                    if evaluation.outcome == IntentEvaluationOutcome::NotSatisfied {
                        issues.push(DesignIntentIssue::InvalidOutcomeForIntentClass {
                            candidate_id: candidate.candidate_id.clone(),
                            intent_id: evaluation.intent_id.clone(),
                        });
                    }
                }
                DesignIntentClass::EngineeringAssumption
                | DesignIntentClass::UnknownSiteConstraint => {
                    if evaluation.outcome == IntentEvaluationOutcome::Satisfied
                        && !evaluation.evidence_ref.as_deref().is_some_and(nonblank)
                    {
                        issues.push(DesignIntentIssue::UnverifiedConclusionClaimedSatisfied {
                            candidate_id: candidate.candidate_id.clone(),
                            intent_id: evaluation.intent_id.clone(),
                        });
                    }
                }
            }

            if evaluation.outcome == IntentEvaluationOutcome::NotApplicable
                && !evaluation
                    .not_applicable_rationale
                    .as_deref()
                    .is_some_and(nonblank)
            {
                issues.push(DesignIntentIssue::NotApplicableWithoutRationale {
                    candidate_id: candidate.candidate_id.clone(),
                    intent_id: evaluation.intent_id.clone(),
                });
            }
            if hard_class && evaluation.outcome == IntentEvaluationOutcome::NotSatisfied {
                failed_hard.push(evaluation);
                if !evaluation.evidence_ref.as_deref().is_some_and(nonblank) {
                    issues.push(DesignIntentIssue::HardConstraintOutcomeLacksEvidence {
                        candidate_id: candidate.candidate_id.clone(),
                        intent_id: evaluation.intent_id.clone(),
                    });
                }
            }
            if candidate.disposition == AlternativeDisposition::ReadyForTechnicalAssessment
                && hard_class
                && !matches!(
                    evaluation.outcome,
                    IntentEvaluationOutcome::Satisfied | IntentEvaluationOutcome::NotApplicable
                )
            {
                issues.push(
                    DesignIntentIssue::TechnicalAssessmentHasUnresolvedHardConstraint {
                        candidate_id: candidate.candidate_id.clone(),
                        intent_id: evaluation.intent_id.clone(),
                    },
                );
            }
        }

        for intent in intents {
            if !seen_evaluations.contains(intent.intent_id.as_str()) {
                issues.push(DesignIntentIssue::MissingEvaluation {
                    candidate_id: candidate.candidate_id.clone(),
                    intent_id: intent.intent_id.clone(),
                });
            }
        }

        match candidate.disposition {
            AlternativeDisposition::BlockedByHardConstraint if failed_hard.is_empty() => {
                issues.push(DesignIntentIssue::BlockedWithoutFailedHardConstraint(
                    candidate.candidate_id.clone(),
                ));
            }
            AlternativeDisposition::BlockedByHardConstraint => {
                if failed_hard
                    .iter()
                    .any(|evaluation| !evaluation.evidence_ref.as_deref().is_some_and(nonblank))
                {
                    issues.push(DesignIntentIssue::BlockedConstraintMissingEvidence(
                        candidate.candidate_id.clone(),
                    ));
                }
            }
            AlternativeDisposition::OwnerDeclined => {
                if !candidate.decision_owner_id.as_deref().is_some_and(nonblank) {
                    issues.push(DesignIntentIssue::OwnerDeclinedWithoutOwner(
                        candidate.candidate_id.clone(),
                    ));
                }
                if !nonblank(&candidate.disposition_rationale) {
                    issues.push(DesignIntentIssue::OwnerDeclinedWithoutRationale(
                        candidate.candidate_id.clone(),
                    ));
                }
            }
            AlternativeDisposition::OpenForExploration
            | AlternativeDisposition::ReadyForTechnicalAssessment => {}
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Validate an identified comparison snapshot, including the owner-brief
/// revision it was derived from, before validating its comparison matrix.
pub fn validate_design_comparison_snapshot(
    comparison: &DesignComparison,
) -> Result<(), Vec<DesignIntentIssue>> {
    let mut issues = Vec::new();
    if !nonblank(&comparison.comparison_id) {
        issues.push(DesignIntentIssue::EmptyComparisonId);
    }
    if !nonblank(&comparison.owner_brief_revision_id) {
        issues.push(DesignIntentIssue::EmptyOwnerBriefRevisionId);
    }
    if let Err(mut matrix_issues) =
        validate_design_comparison(&comparison.intents, &comparison.alternatives)
    {
        issues.append(&mut matrix_issues);
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Pairwise result. Preferences are independent axes; ratings are compared
/// by Pareto dominance only, never added or weighted into one total.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreferencePairRelation {
    LeftDominates,
    RightDominates,
    Equivalent,
    Tradeoff,
    IncomparableMissingRatings,
    NotComparable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferencePairComparison {
    pub left_candidate_id: String,
    pub right_candidate_id: String,
    pub relation: PreferencePairRelation,
    pub left_better_on: Vec<String>,
    pub right_better_on: Vec<String>,
    pub missing_ratings_on: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontierExclusionReason {
    OwnerDeclined,
    ExplicitlyBlocked,
    HardConstraintFailed { intent_id: String },
    HardConstraintUnresolved { intent_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierExclusion {
    pub candidate_id: String,
    pub reason: FrontierExclusionReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceFrontierReport {
    /// Candidates with no known failed/unresolved hard constraint.
    /// This is comparison eligibility, not legal or professional approval.
    pub eligible_candidate_ids: Vec<String>,
    /// Eligible alternatives not Pareto-dominated on owner-entered ratings.
    pub frontier_candidate_ids: Vec<String>,
    pub dominated_candidate_ids: Vec<String>,
    pub excluded_candidates: Vec<FrontierExclusion>,
    pub pairwise_comparisons: Vec<PreferencePairComparison>,
}

/// Compute a transparent Pareto frontier from explicit owner preference ratings.
/// Missing ratings are never imputed; a pair with asymmetric missing ratings is
/// incomparable. Failed/unresolved binding or owner-hard constraints stay visible
/// as exclusions rather than being traded against aesthetic preferences.
pub fn compute_owner_preference_frontier(
    comparison: &DesignComparison,
) -> Result<PreferenceFrontierReport, Vec<DesignIntentIssue>> {
    validate_design_comparison_snapshot(comparison)?;

    let preference_ids: Vec<&str> = comparison
        .intents
        .iter()
        .filter(|intent| intent.class == DesignIntentClass::OwnerPreference)
        .map(|intent| intent.intent_id.as_str())
        .collect();

    let mut eligible = Vec::new();
    let mut excluded = Vec::new();
    for candidate in &comparison.alternatives {
        if candidate.disposition == AlternativeDisposition::OwnerDeclined {
            excluded.push(FrontierExclusion {
                candidate_id: candidate.candidate_id.clone(),
                reason: FrontierExclusionReason::OwnerDeclined,
            });
            continue;
        }
        if candidate.disposition == AlternativeDisposition::BlockedByHardConstraint {
            excluded.push(FrontierExclusion {
                candidate_id: candidate.candidate_id.clone(),
                reason: FrontierExclusionReason::ExplicitlyBlocked,
            });
            continue;
        }

        let mut hard_issue = None;
        for intent in &comparison.intents {
            if !matches!(
                intent.class,
                DesignIntentClass::BindingRequirement | DesignIntentClass::OwnerHardConstraint
            ) {
                continue;
            }
            let evaluation = candidate
                .evaluations
                .iter()
                .find(|evaluation| evaluation.intent_id == intent.intent_id)
                .expect("validated comparison has one evaluation per intent");
            match evaluation.outcome {
                IntentEvaluationOutcome::Satisfied | IntentEvaluationOutcome::NotApplicable => {}
                IntentEvaluationOutcome::NotSatisfied => {
                    hard_issue = Some(FrontierExclusionReason::HardConstraintFailed {
                        intent_id: intent.intent_id.clone(),
                    });
                    break;
                }
                IntentEvaluationOutcome::Unknown
                | IntentEvaluationOutcome::PreferenceUnmet
                | IntentEvaluationOutcome::Tradeoff => {
                    hard_issue = Some(FrontierExclusionReason::HardConstraintUnresolved {
                        intent_id: intent.intent_id.clone(),
                    });
                    break;
                }
            }
        }
        if let Some(reason) = hard_issue {
            excluded.push(FrontierExclusion {
                candidate_id: candidate.candidate_id.clone(),
                reason,
            });
        } else {
            eligible.push(candidate);
        }
    }

    eligible.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    excluded.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let ratings_for = |candidate: &DesignAlternative| -> BTreeMap<&str, Option<u8>> {
        preference_ids
            .iter()
            .map(|intent_id| {
                let rating = candidate
                    .evaluations
                    .iter()
                    .find(|evaluation| evaluation.intent_id == *intent_id)
                    .and_then(|evaluation| evaluation.preference_rating);
                (*intent_id, rating)
            })
            .collect()
    };
    let ratings: BTreeMap<&str, BTreeMap<&str, Option<u8>>> = eligible
        .iter()
        .map(|candidate| (candidate.candidate_id.as_str(), ratings_for(candidate)))
        .collect();

    let mut pairwise_comparisons = Vec::new();
    let mut dominated = BTreeSet::new();
    for left_index in 0..eligible.len() {
        for right_index in (left_index + 1)..eligible.len() {
            let left = eligible[left_index];
            let right = eligible[right_index];
            let left_ratings = &ratings[left.candidate_id.as_str()];
            let right_ratings = &ratings[right.candidate_id.as_str()];
            let mut left_better_on = Vec::new();
            let mut right_better_on = Vec::new();
            let mut missing_ratings_on = Vec::new();
            let mut comparable_dimensions = 0;

            for intent_id in &preference_ids {
                match (left_ratings[*intent_id], right_ratings[*intent_id]) {
                    (Some(left_rating), Some(right_rating)) => {
                        comparable_dimensions += 1;
                        if left_rating > right_rating {
                            left_better_on.push((*intent_id).to_owned());
                        } else if right_rating > left_rating {
                            right_better_on.push((*intent_id).to_owned());
                        }
                    }
                    (Some(_), None) | (None, Some(_)) => {
                        missing_ratings_on.push((*intent_id).to_owned());
                    }
                    (None, None) => {}
                }
            }

            let relation = if !missing_ratings_on.is_empty() {
                PreferencePairRelation::IncomparableMissingRatings
            } else if comparable_dimensions == 0 {
                PreferencePairRelation::NotComparable
            } else if !left_better_on.is_empty() && right_better_on.is_empty() {
                dominated.insert(right.candidate_id.clone());
                PreferencePairRelation::LeftDominates
            } else if !right_better_on.is_empty() && left_better_on.is_empty() {
                dominated.insert(left.candidate_id.clone());
                PreferencePairRelation::RightDominates
            } else if left_better_on.is_empty() && right_better_on.is_empty() {
                PreferencePairRelation::Equivalent
            } else {
                PreferencePairRelation::Tradeoff
            };

            pairwise_comparisons.push(PreferencePairComparison {
                left_candidate_id: left.candidate_id.clone(),
                right_candidate_id: right.candidate_id.clone(),
                relation,
                left_better_on,
                right_better_on,
                missing_ratings_on,
            });
        }
    }

    let eligible_candidate_ids: Vec<String> = eligible
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect();
    let dominated_candidate_ids: Vec<String> = dominated.into_iter().collect();
    let frontier_candidate_ids = eligible_candidate_ids
        .iter()
        .filter(|candidate_id| !dominated_candidate_ids.contains(candidate_id))
        .cloned()
        .collect();

    Ok(PreferenceFrontierReport {
        eligible_candidate_ids,
        frontier_candidate_ids,
        dominated_candidate_ids,
        excluded_candidates: excluded,
        pairwise_comparisons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(id: &str, class: DesignIntentClass) -> DesignIntent {
        DesignIntent {
            intent_id: id.into(),
            title: id.into(),
            description: "Test design intent".into(),
            class,
            priority: IntentPriority::Important,
            origin: match class {
                DesignIntentClass::BindingRequirement => IntentOrigin::JurisdictionRulePack,
                DesignIntentClass::OwnerHardConstraint | DesignIntentClass::OwnerPreference => {
                    IntentOrigin::OwnerBrief
                }
                DesignIntentClass::ExplorationGoal => IntentOrigin::OwnerBrief,
                DesignIntentClass::EngineeringAssumption => IntentOrigin::DesignTeamAssumption,
                DesignIntentClass::UnknownSiteConstraint => IntentOrigin::UnverifiedSiteObservation,
            },
            source_ref: (class == DesignIntentClass::BindingRequirement)
                .then(|| "source:clause-1".into()),
            source_module_id: (class == DesignIntentClass::BindingRequirement)
                .then(|| "module:za:r1".into()),
            decision_owner_id: (class == DesignIntentClass::OwnerHardConstraint)
                .then(|| "owner:household".into()),
            uncertainty_note: matches!(
                class,
                DesignIntentClass::EngineeringAssumption | DesignIntentClass::UnknownSiteConstraint
            )
            .then(|| "Assumption/unknown must be validated".into()),
        }
    }

    fn evaluation(id: &str, outcome: IntentEvaluationOutcome) -> IntentEvaluation {
        IntentEvaluation {
            intent_id: id.into(),
            outcome,
            explanation: "Synthetic test rationale".into(),
            evidence_ref: matches!(
                outcome,
                IntentEvaluationOutcome::NotSatisfied | IntentEvaluationOutcome::Satisfied
            )
            .then(|| format!("evidence:{id}")),
            not_applicable_rationale: (outcome == IntentEvaluationOutcome::NotApplicable)
                .then(|| "The stated scope excludes this item for this reviewed option".into()),
            uncertainty_note: None,
            preference_rating: None,
        }
    }

    fn candidate(
        id: &str,
        disposition: AlternativeDisposition,
        evaluations: Vec<IntentEvaluation>,
    ) -> DesignAlternative {
        DesignAlternative {
            candidate_id: id.into(),
            design_revision_id: format!("{id}:revision:1"),
            title: format!("Design alternative {id}"),
            disposition,
            decision_owner_id: (disposition == AlternativeDisposition::OwnerDeclined)
                .then(|| "owner:household".into()),
            disposition_rationale: "Synthetic comparison disposition".into(),
            evaluations,
        }
    }

    #[test]
    fn comparison_snapshot_requires_stable_identity_and_owner_brief_revision() {
        let comparison = DesignComparison {
            comparison_id: String::new(),
            owner_brief_revision_id: String::new(),
            intents: Vec::new(),
            alternatives: Vec::new(),
        };
        let issues = validate_design_comparison_snapshot(&comparison).unwrap_err();
        assert!(issues.contains(&DesignIntentIssue::EmptyComparisonId));
        assert!(issues.contains(&DesignIntentIssue::EmptyOwnerBriefRevisionId));
    }

    fn preference_rating(id: &str, rating: u8) -> IntentEvaluation {
        let mut item = evaluation(id, IntentEvaluationOutcome::Satisfied);
        item.preference_rating = Some(rating);
        item
    }

    fn preference_comparison() -> DesignComparison {
        let intents = vec![
            intent("rule:egress", DesignIntentClass::BindingRequirement),
            intent("owner:daylight", DesignIntentClass::OwnerPreference),
            intent("owner:privacy", DesignIntentClass::OwnerPreference),
        ];
        let hard_pass = evaluation("rule:egress", IntentEvaluationOutcome::Satisfied);
        DesignComparison {
            comparison_id: "comparison:brief-r1".into(),
            owner_brief_revision_id: "owner-brief:r1".into(),
            intents,
            alternatives: vec![
                candidate(
                    "candidate:a",
                    AlternativeDisposition::ReadyForTechnicalAssessment,
                    vec![
                        hard_pass.clone(),
                        preference_rating("owner:daylight", 90),
                        preference_rating("owner:privacy", 35),
                    ],
                ),
                candidate(
                    "candidate:b",
                    AlternativeDisposition::ReadyForTechnicalAssessment,
                    vec![
                        hard_pass.clone(),
                        preference_rating("owner:daylight", 45),
                        preference_rating("owner:privacy", 90),
                    ],
                ),
                candidate(
                    "candidate:c",
                    AlternativeDisposition::OpenForExploration,
                    vec![
                        hard_pass.clone(),
                        preference_rating("owner:daylight", 20),
                        preference_rating("owner:privacy", 20),
                    ],
                ),
                candidate(
                    "candidate:d",
                    AlternativeDisposition::OpenForExploration,
                    vec![
                        hard_pass,
                        preference_rating("owner:daylight", 70),
                        evaluation("owner:privacy", IntentEvaluationOutcome::Unknown),
                    ],
                ),
            ],
        }
    }

    #[test]
    fn pareto_frontier_preserves_tradeoffs_and_only_removes_dominated_options() {
        let report = compute_owner_preference_frontier(&preference_comparison()).unwrap();
        assert_eq!(
            report.frontier_candidate_ids,
            vec![
                "candidate:a".to_owned(),
                "candidate:b".to_owned(),
                "candidate:d".to_owned()
            ]
        );
        assert_eq!(
            report.dominated_candidate_ids,
            vec!["candidate:c".to_owned()]
        );
        assert!(report.pairwise_comparisons.iter().any(|pair| {
            pair.left_candidate_id == "candidate:a"
                && pair.right_candidate_id == "candidate:b"
                && pair.relation == PreferencePairRelation::Tradeoff
        }));
    }

    #[test]
    fn preference_rating_above_one_hundred_is_rejected() {
        let mut comparison = preference_comparison();
        comparison.alternatives[0].evaluations[2].preference_rating = Some(101);
        assert!(
            validate_design_comparison_snapshot(&comparison)
                .unwrap_err()
                .contains(&DesignIntentIssue::PreferenceRatingOutOfRange {
                    candidate_id: "candidate:a".into(),
                    intent_id: "owner:privacy".into(),
                    rating: 101,
                })
        );
    }

    #[test]
    fn missing_preference_ratings_make_alternatives_incomparable_not_imputed() {
        let report = compute_owner_preference_frontier(&preference_comparison()).unwrap();
        assert!(report.pairwise_comparisons.iter().any(|pair| {
            pair.left_candidate_id == "candidate:a"
                && pair.right_candidate_id == "candidate:d"
                && pair.relation == PreferencePairRelation::IncomparableMissingRatings
                && pair.missing_ratings_on == vec!["owner:privacy".to_owned()]
        }));
    }

    #[test]
    fn unresolved_hard_constraint_excludes_candidate_from_preference_frontier() {
        let mut comparison = preference_comparison();
        comparison.alternatives[0].evaluations[0].outcome = IntentEvaluationOutcome::Unknown;
        comparison.alternatives[0].evaluations[0].evidence_ref = None;
        comparison.alternatives[0].disposition = AlternativeDisposition::OpenForExploration;
        let report = compute_owner_preference_frontier(&comparison).unwrap();
        assert!(
            !report
                .frontier_candidate_ids
                .contains(&"candidate:a".to_owned())
        );
        assert!(report.excluded_candidates.iter().any(|item| {
            item.candidate_id == "candidate:a"
                && matches!(
                    &item.reason,
                    FrontierExclusionReason::HardConstraintUnresolved { intent_id }
                        if intent_id == "rule:egress"
                )
        }));
    }

    #[test]
    fn preference_ratings_cannot_be_attached_to_binding_requirements() {
        let mut comparison = preference_comparison();
        comparison.alternatives[0].evaluations[0].preference_rating = Some(90);
        assert!(
            validate_design_comparison_snapshot(&comparison)
                .unwrap_err()
                .contains(&DesignIntentIssue::PreferenceRatingOnNonPreference {
                    candidate_id: "candidate:a".into(),
                    intent_id: "rule:egress".into(),
                })
        );
    }

    #[test]
    fn an_unmet_owner_preference_does_not_block_a_concept() {
        let intents = vec![
            intent("rule:egress", DesignIntentClass::BindingRequirement),
            intent("owner:arched-windows", DesignIntentClass::OwnerPreference),
        ];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::ReadyForTechnicalAssessment,
            vec![
                evaluation("rule:egress", IntentEvaluationOutcome::Satisfied),
                evaluation(
                    "owner:arched-windows",
                    IntentEvaluationOutcome::PreferenceUnmet,
                ),
            ],
        );
        assert_eq!(validate_design_comparison(&intents, &[candidate]), Ok(()));
    }

    #[test]
    fn preference_must_not_be_mislabeled_as_a_hard_failure() {
        let intents = vec![intent(
            "owner:arched-windows",
            DesignIntentClass::OwnerPreference,
        )];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::OpenForExploration,
            vec![evaluation(
                "owner:arched-windows",
                IntentEvaluationOutcome::NotSatisfied,
            )],
        );
        assert!(
            validate_design_comparison(&intents, &[candidate])
                .unwrap_err()
                .contains(&DesignIntentIssue::PreferenceMisclassifiedAsHardViolation {
                    candidate_id: "candidate:a".into(),
                    intent_id: "owner:arched-windows".into(),
                })
        );
    }

    #[test]
    fn rejected_alternative_requires_evidence_of_a_failed_hard_constraint() {
        let intents = vec![intent("rule:egress", DesignIntentClass::BindingRequirement)];
        let mut failed = evaluation("rule:egress", IntentEvaluationOutcome::NotSatisfied);
        failed.evidence_ref = Some("measurement:door-clearance".into());
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::BlockedByHardConstraint,
            vec![failed],
        );
        assert_eq!(validate_design_comparison(&intents, &[candidate]), Ok(()));
    }

    #[test]
    fn hard_failure_without_evidence_cannot_block_a_candidate() {
        let intents = vec![intent("rule:egress", DesignIntentClass::BindingRequirement)];
        let mut failed = evaluation("rule:egress", IntentEvaluationOutcome::NotSatisfied);
        failed.evidence_ref = None;
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::BlockedByHardConstraint,
            vec![failed],
        );
        assert!(
            validate_design_comparison(&intents, &[candidate])
                .unwrap_err()
                .contains(&DesignIntentIssue::HardConstraintOutcomeLacksEvidence {
                    candidate_id: "candidate:a".into(),
                    intent_id: "rule:egress".into(),
                })
        );
    }

    #[test]
    fn every_candidate_must_address_every_declared_intent() {
        let intents = vec![
            intent("owner:privacy", DesignIntentClass::OwnerPreference),
            intent("goal:daylight", DesignIntentClass::ExplorationGoal),
        ];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::OpenForExploration,
            vec![evaluation(
                "owner:privacy",
                IntentEvaluationOutcome::Satisfied,
            )],
        );
        assert!(
            validate_design_comparison(&intents, &[candidate])
                .unwrap_err()
                .contains(&DesignIntentIssue::MissingEvaluation {
                    candidate_id: "candidate:a".into(),
                    intent_id: "goal:daylight".into(),
                })
        );
    }

    #[test]
    fn binding_intent_requires_source_and_module_provenance() {
        let mut binding = intent("rule:egress", DesignIntentClass::BindingRequirement);
        binding.source_ref = None;
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::OpenForExploration,
            vec![evaluation("rule:egress", IntentEvaluationOutcome::Unknown)],
        );
        assert!(
            validate_design_comparison(&[binding], &[candidate])
                .unwrap_err()
                .contains(&DesignIntentIssue::BindingIntentMissingProvenance(
                    "rule:egress".into()
                ))
        );
    }

    #[test]
    fn owner_decline_is_explicit_and_does_not_claim_rule_violation() {
        let intents = vec![intent("owner:material", DesignIntentClass::OwnerPreference)];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::OwnerDeclined,
            vec![evaluation(
                "owner:material",
                IntentEvaluationOutcome::PreferenceUnmet,
            )],
        );
        assert_eq!(validate_design_comparison(&intents, &[candidate]), Ok(()));
    }

    #[test]
    fn ready_for_technical_assessment_cannot_contain_unknown_hard_rules() {
        let intents = vec![intent("rule:fire", DesignIntentClass::BindingRequirement)];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::ReadyForTechnicalAssessment,
            vec![evaluation("rule:fire", IntentEvaluationOutcome::Unknown)],
        );
        assert!(
            validate_design_comparison(&intents, &[candidate])
                .unwrap_err()
                .contains(
                    &DesignIntentIssue::TechnicalAssessmentHasUnresolvedHardConstraint {
                        candidate_id: "candidate:a".into(),
                        intent_id: "rule:fire".into(),
                    }
                )
        );
    }
}

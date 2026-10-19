// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Owner-led design intent and explainable alternative comparison.
//!
//! This model explicitly separates binding requirements, owner-declared hard
//! constraints, preferences, and exploratory goals. It intentionally has no
//! single aggregate "best home" score: incomparable values such as privacy,
//! cultural expression, cost, daylight and spatial character remain visible.
//! It does not solve architecture, validate code, or certify buildability.

use std::collections::HashSet;

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
    DuplicateEvaluation { candidate_id: String, intent_id: String },
    UnknownIntent { candidate_id: String, intent_id: String },
    MissingEvaluation { candidate_id: String, intent_id: String },
    EmptyEvaluationExplanation { candidate_id: String, intent_id: String },
    InvalidOutcomeForIntentClass { candidate_id: String, intent_id: String },
    NotApplicableWithoutRationale { candidate_id: String, intent_id: String },
    HardConstraintOutcomeLacksEvidence { candidate_id: String, intent_id: String },
    PreferenceMisclassifiedAsHardViolation { candidate_id: String, intent_id: String },
    TechnicalAssessmentHasUnresolvedHardConstraint { candidate_id: String, intent_id: String },
    BlockedWithoutFailedHardConstraint(String),
    BlockedConstraintMissingEvidence(String),
    OwnerDeclinedWithoutOwner(String),
    OwnerDeclinedWithoutRationale(String),
    UnverifiedConclusionClaimedSatisfied { candidate_id: String, intent_id: String },
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
            issues.push(DesignIntentIssue::DuplicateIntentId(intent.intent_id.clone()));
        }
        if !nonblank(&intent.title) {
            issues.push(DesignIntentIssue::EmptyIntentTitle(intent.intent_id.clone()));
        }
        if !nonblank(&intent.description) {
            issues.push(DesignIntentIssue::EmptyIntentDescription(intent.intent_id.clone()));
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
                if !intent
                    .decision_owner_id
                    .as_deref()
                    .is_some_and(nonblank)
                {
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
            let Some(intent) = intents.iter().find(|item| item.intent_id == evaluation.intent_id)
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
                    if matches!(
                        evaluation.outcome,
                        IntentEvaluationOutcome::Satisfied | IntentEvaluationOutcome::Tradeoff
                    ) && intent.class == DesignIntentClass::OwnerPreference
                        && evaluation.outcome == IntentEvaluationOutcome::Tradeoff
                    {
                        // A tradeoff is a legitimate description, never a failure verdict.
                    }
                }
                DesignIntentClass::BindingRequirement | DesignIntentClass::OwnerHardConstraint => {
                    if matches!(
                        evaluation.outcome,
                        IntentEvaluationOutcome::PreferenceUnmet | IntentEvaluationOutcome::Tradeoff
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
                DesignIntentClass::EngineeringAssumption | DesignIntentClass::UnknownSiteConstraint => {
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
                issues.push(DesignIntentIssue::TechnicalAssessmentHasUnresolvedHardConstraint {
                    candidate_id: candidate.candidate_id.clone(),
                    intent_id: evaluation.intent_id.clone(),
                });
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
                if failed_hard.iter().any(|evaluation| {
                    !evaluation.evidence_ref.as_deref().is_some_and(nonblank)
                }) {
                    issues.push(DesignIntentIssue::BlockedConstraintMissingEvidence(
                        candidate.candidate_id.clone(),
                    ));
                }
            }
            AlternativeDisposition::OwnerDeclined => {
                if !candidate
                    .decision_owner_id
                    .as_deref()
                    .is_some_and(nonblank)
                {
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
            evidence_ref: if outcome == IntentEvaluationOutcome::NotSatisfied {
                Some(format!("evidence:{id}"))
            } else if outcome == IntentEvaluationOutcome::Satisfied {
                Some(format!("evidence:{id}"))
            } else {
                None
            },
            not_applicable_rationale: (outcome == IntentEvaluationOutcome::NotApplicable)
                .then(|| "The stated scope excludes this item for this reviewed option".into()),
            uncertainty_note: None,
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
                evaluation("owner:arched-windows", IntentEvaluationOutcome::PreferenceUnmet),
            ],
        );
        assert_eq!(validate_design_comparison(&intents, &[candidate]), Ok(()));
    }

    #[test]
    fn preference_must_not_be_mislabeled_as_a_hard_failure() {
        let intents = vec![intent("owner:arched-windows", DesignIntentClass::OwnerPreference)];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::OpenForExploration,
            vec![evaluation("owner:arched-windows", IntentEvaluationOutcome::NotSatisfied)],
        );
        assert!(validate_design_comparison(&intents, &[candidate]).unwrap_err().contains(
            &DesignIntentIssue::PreferenceMisclassifiedAsHardViolation {
                candidate_id: "candidate:a".into(),
                intent_id: "owner:arched-windows".into(),
            }
        ));
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
        assert!(validate_design_comparison(&intents, &[candidate]).unwrap_err().contains(
            &DesignIntentIssue::HardConstraintOutcomeLacksEvidence {
                candidate_id: "candidate:a".into(),
                intent_id: "rule:egress".into(),
            }
        ));
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
            vec![evaluation("owner:privacy", IntentEvaluationOutcome::Satisfied)],
        );
        assert!(validate_design_comparison(&intents, &[candidate]).unwrap_err().contains(
            &DesignIntentIssue::MissingEvaluation {
                candidate_id: "candidate:a".into(),
                intent_id: "goal:daylight".into(),
            }
        ));
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
        assert!(validate_design_comparison(&[binding], &[candidate]).unwrap_err().contains(
            &DesignIntentIssue::BindingIntentMissingProvenance("rule:egress".into())
        ));
    }

    #[test]
    fn owner_decline_is_explicit_and_does_not_claim_rule_violation() {
        let intents = vec![intent("owner:material", DesignIntentClass::OwnerPreference)];
        let candidate = candidate(
            "candidate:a",
            AlternativeDisposition::OwnerDeclined,
            vec![evaluation("owner:material", IntentEvaluationOutcome::PreferenceUnmet)],
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
        assert!(validate_design_comparison(&intents, &[candidate]).unwrap_err().contains(
            &DesignIntentIssue::TechnicalAssessmentHasUnresolvedHardConstraint {
                candidate_id: "candidate:a".into(),
                intent_id: "rule:fire".into(),
            }
        ));
    }
}

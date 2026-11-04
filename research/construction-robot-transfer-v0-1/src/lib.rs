//! Deterministic, evidence-denominator-aware evaluator for construction robot site transfer.
//!
//! This crate validates declared split guards, transfer regimes and accepted-output
//! denominators. It does not authenticate evidence, determine physical truth, estimate
//! causal effects, verify an ODD, authorize robot motion, or infer safety from productivity.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferRegime {
    ZeroShot,
    CalibrationOnly,
    BoundedOfflineAdaptation,
    HumanSupervisedMethodAdaptation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationRole {
    SourceReference,
    TargetEvaluation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estimability {
    Estimable,
    NotEstimable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitGuard {
    pub dimension: String,
    pub training_ids: Vec<String>,
    pub adaptation_ids: Vec<String>,
    pub evaluation_ids: Vec<String>,
    /// Only use when adaptation and evaluation legitimately share a target domain
    /// (for example, same target site but disjoint work packages).
    pub allow_adaptation_evaluation_domain_overlap: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationRecord {
    pub evaluation_id: String,
    pub role: EvaluationRole,
    pub work_item_ids: Vec<String>,
    pub match_stratum_id: String,
    pub task_family: String,
    pub functional_unit_class: String,
    pub acceptance_profile_digest: String,
    pub planned_units: u64,
    pub attempted_units: u64,
    pub first_pass_inspected_units: u64,
    pub first_pass_accepted_units: u64,
    pub finally_accepted_units: u64,
    pub finally_rejected_units: u64,
    pub uninspected_units: u64,
    pub unresolved_units: u64,
    pub total_workflow_crew_minutes: f64,
    pub robot_operating_minutes: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptationBudget {
    pub max_target_label_count: u64,
    pub max_calibration_person_hours: f64,
    pub max_engineering_person_hours: f64,
    pub max_operator_person_hours: f64,
    pub max_inspection_person_hours: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptationEffort {
    pub target_label_count: u64,
    pub calibration_person_hours: f64,
    pub engineering_person_hours: f64,
    pub operator_person_hours: f64,
    pub inspection_person_hours: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferStudy {
    pub study_id: String,
    pub regime: TransferRegime,
    pub source_model_digest: String,
    pub target_model_digest: String,
    pub source_robot_config_digest: String,
    pub target_robot_config_digest: String,
    pub calibration_change_receipt_ref: Option<String>,
    pub adaptation_budget: AdaptationBudget,
    pub adaptation_effort: AdaptationEffort,
    pub split_guards: Vec<SplitGuard>,
    pub target_adaptation_work_item_ids: Vec<String>,
    pub target_evaluation_work_item_ids: Vec<String>,
    pub source: EvaluationRecord,
    pub target: EvaluationRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
    EmptyField(&'static str),
    EmptyDimension,
    DuplicateSplitId {
        dimension: String,
        id: String,
    },
    TrainingAdaptationLeakage {
        dimension: String,
        id: String,
    },
    TrainingEvaluationLeakage {
        dimension: String,
        id: String,
    },
    AdaptationEvaluationLeakage {
        dimension: String,
        id: String,
    },
    DuplicateWorkItem {
        id: String,
    },
    AdaptationEvaluationWorkItemLeakage {
        id: String,
    },
    TrainingAdaptationWorkItemLeakage {
        id: String,
    },
    TrainingEvaluationWorkItemLeakage {
        id: String,
    },
    MissingSplitPartition(&'static str),
    InvalidRegime(&'static str),
    LabelBudgetExceeded { budget: u64, actual: u64 },
    AdaptationBudgetExceeded {
        field: &'static str,
        budget_person_hours: f64,
        actual_person_hours: f64,
    },
    InvalidCount(&'static str),
    NonFiniteOrNegativeMetric(&'static str),
    InvalidRole(&'static str),
    InvalidEvaluation(&'static str),
    MismatchedComparison(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationMetrics {
    pub first_pass_yield: Option<f64>,
    pub final_acceptance_fraction_of_attempted: Option<f64>,
    pub accepted_units_per_total_workflow_crew_hour: Option<f64>,
    pub accepted_units_per_robot_hour: Option<f64>,
    pub first_pass_inspection_denominator: u64,
    pub attempted_denominator: u64,
    pub accepted_numerator: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferRetention {
    pub estimability: Estimability,
    pub source_accepted_units_per_crew_hour: Option<f64>,
    pub target_accepted_units_per_crew_hour: Option<f64>,
    pub target_to_source_ratio: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptationBurden {
    pub target_labels: u64,
    pub declared_label_budget: u64,
    pub calibration_person_hours: f64,
    pub engineering_person_hours: f64,
    pub operator_person_hours: f64,
    pub inspection_person_hours: f64,
    pub total_recorded_person_hours: f64,
}

impl TransferStudy {
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        for (field, value) in [
            ("study_id", self.study_id.as_str()),
            ("source_model_digest", self.source_model_digest.as_str()),
            ("target_model_digest", self.target_model_digest.as_str()),
            (
                "source_robot_config_digest",
                self.source_robot_config_digest.as_str(),
            ),
            (
                "target_robot_config_digest",
                self.target_robot_config_digest.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                errors.push(ValidationError::EmptyField(field));
            }
        }

        if self.source.role != EvaluationRole::SourceReference {
            errors.push(ValidationError::InvalidRole(
                "source_must_be_source_reference",
            ));
        }
        if self.target.role != EvaluationRole::TargetEvaluation {
            errors.push(ValidationError::InvalidRole(
                "target_must_be_target_evaluation",
            ));
        }

        validate_record(&self.source, &mut errors);
        validate_record(&self.target, &mut errors);

        for guard in &self.split_guards {
            validate_split_guard(guard, &mut errors);
        }

        validate_unique_ids(
            &self.target_adaptation_work_item_ids,
            "target_adaptation_work_item_ids",
            &mut errors,
        );
        validate_unique_ids(
            &self.target_evaluation_work_item_ids,
            "target_evaluation_work_item_ids",
            &mut errors,
        );

        let adaptation_items: BTreeSet<&str> = self
            .target_adaptation_work_item_ids
            .iter()
            .map(String::as_str)
            .collect();
        for item in &self.target_evaluation_work_item_ids {
            if adaptation_items.contains(item.as_str()) {
                errors.push(ValidationError::AdaptationEvaluationWorkItemLeakage {
                    id: item.clone(),
                });
            }
        }
        let source_items: BTreeSet<&str> = self
            .source
            .work_item_ids
            .iter()
            .map(String::as_str)
            .collect();
        for item in &self.target_adaptation_work_item_ids {
            if source_items.contains(item.as_str()) {
                errors
                    .push(ValidationError::TrainingAdaptationWorkItemLeakage { id: item.clone() });
            }
        }
        for item in &self.target.work_item_ids {
            if source_items.contains(item.as_str()) {
                errors
                    .push(ValidationError::TrainingEvaluationWorkItemLeakage { id: item.clone() });
            }
        }

        for (field, budget, actual) in [
            (
                "calibration_person_hours",
                self.adaptation_budget.max_calibration_person_hours,
                self.adaptation_effort.calibration_person_hours,
            ),
            (
                "engineering_person_hours",
                self.adaptation_budget.max_engineering_person_hours,
                self.adaptation_effort.engineering_person_hours,
            ),
            (
                "operator_person_hours",
                self.adaptation_budget.max_operator_person_hours,
                self.adaptation_effort.operator_person_hours,
            ),
            (
                "inspection_person_hours",
                self.adaptation_budget.max_inspection_person_hours,
                self.adaptation_effort.inspection_person_hours,
            ),
        ] {
            if !budget.is_finite() || budget < 0.0 {
                errors.push(ValidationError::NonFiniteOrNegativeMetric(
                    "adaptation_budget_person_hours",
                ));
            }
            if !actual.is_finite() || actual < 0.0 {
                errors.push(ValidationError::NonFiniteOrNegativeMetric(
                    "adaptation_effort_person_hours",
                ));
            }
            if actual > budget {
                errors.push(ValidationError::AdaptationBudgetExceeded {
                    field,
                    budget_person_hours: budget,
                    actual_person_hours: actual,
                });
            }
        }

        if self.adaptation_effort.target_label_count > self.adaptation_budget.max_target_label_count
        {
            errors.push(ValidationError::LabelBudgetExceeded {
                budget: self.adaptation_budget.max_target_label_count,
                actual: self.adaptation_effort.target_label_count,
            });
        }

        match self.regime {
            TransferRegime::ZeroShot => {
                if self.adaptation_effort.target_label_count != 0 {
                    errors.push(ValidationError::InvalidRegime(
                        "zero_shot_must_not_use_target_task_labels",
                    ));
                }
                if self.source_model_digest != self.target_model_digest {
                    errors.push(ValidationError::InvalidRegime(
                        "zero_shot_model_digest_must_remain_frozen",
                    ));
                }
                if self.source_robot_config_digest != self.target_robot_config_digest {
                    errors.push(ValidationError::InvalidRegime(
                        "zero_shot_robot_config_digest_must_remain_frozen",
                    ));
                }
                if self.calibration_change_receipt_ref.is_some() {
                    errors.push(ValidationError::InvalidRegime(
                        "zero_shot_must_not_claim_calibration_change_receipt",
                    ));
                }
            }
            TransferRegime::CalibrationOnly => {
                if self.adaptation_effort.target_label_count != 0 {
                    errors.push(ValidationError::InvalidRegime(
                        "calibration_only_must_not_use_target_task_labels",
                    ));
                }
                if self.source_model_digest != self.target_model_digest {
                    errors.push(ValidationError::InvalidRegime(
                        "calibration_only_policy_digest_must_remain_frozen",
                    ));
                }
                if self.source_robot_config_digest != self.target_robot_config_digest
                    && self
                        .calibration_change_receipt_ref
                        .as_deref()
                        .is_none_or(str::is_empty)
                {
                    errors.push(ValidationError::InvalidRegime(
                        "calibration_only_config_change_requires_receipt",
                    ));
                }
            }
            TransferRegime::BoundedOfflineAdaptation => {}
            TransferRegime::HumanSupervisedMethodAdaptation => {
                if self.adaptation_effort.engineering_person_hours <= 0.0 {
                    errors.push(ValidationError::InvalidRegime(
                        "human_supervised_adaptation_requires_recorded_engineering_effort",
                    ));
                }
            }
        }

        if self.source.match_stratum_id != self.target.match_stratum_id {
            errors.push(ValidationError::MismatchedComparison("match_stratum_id"));
        }
        if self.source.task_family != self.target.task_family {
            errors.push(ValidationError::MismatchedComparison("task_family"));
        }
        if self.source.functional_unit_class != self.target.functional_unit_class {
            errors.push(ValidationError::MismatchedComparison(
                "functional_unit_class",
            ));
        }
        if self.source.acceptance_profile_digest != self.target.acceptance_profile_digest {
            errors.push(ValidationError::MismatchedComparison(
                "acceptance_profile_digest",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort_by_key(|error| format!("{error:?}"));
            errors.dedup();
            Err(errors)
        }
    }

    pub fn adaptation_burden(&self) -> Result<AdaptationBurden, Vec<ValidationError>> {
        self.validate()?;

        let total_recorded_person_hours = self.adaptation_effort.calibration_person_hours
            + self.adaptation_effort.engineering_person_hours
            + self.adaptation_effort.operator_person_hours
            + self.adaptation_effort.inspection_person_hours;

        Ok(AdaptationBurden {
            target_labels: self.adaptation_effort.target_label_count,
            declared_label_budget: self.adaptation_budget.max_target_label_count,
            calibration_person_hours: self.adaptation_effort.calibration_person_hours,
            engineering_person_hours: self.adaptation_effort.engineering_person_hours,
            operator_person_hours: self.adaptation_effort.operator_person_hours,
            inspection_person_hours: self.adaptation_effort.inspection_person_hours,
            total_recorded_person_hours,
        })
    }

    pub fn transfer_retention(&self) -> Result<TransferRetention, Vec<ValidationError>> {
        self.validate()?;

        let source_metrics = calculate_metrics(&self.source).map_err(|error| vec![error])?;
        let target_metrics = calculate_metrics(&self.target).map_err(|error| vec![error])?;

        let source_rate = source_metrics.accepted_units_per_total_workflow_crew_hour;
        let target_rate = target_metrics.accepted_units_per_total_workflow_crew_hour;

        let ratio = match (source_rate, target_rate) {
            (Some(source), Some(target)) if source > 0.0 => Some(target / source),
            _ => None,
        };

        Ok(TransferRetention {
            estimability: if ratio.is_some() {
                Estimability::Estimable
            } else {
                Estimability::NotEstimable
            },
            source_accepted_units_per_crew_hour: source_rate,
            target_accepted_units_per_crew_hour: target_rate,
            target_to_source_ratio: ratio,
        })
    }
}

pub fn calculate_metrics(record: &EvaluationRecord) -> Result<EvaluationMetrics, ValidationError> {
    let mut errors = Vec::new();
    validate_record(record, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }

    let first_pass_yield = ratio(
        record.first_pass_accepted_units,
        record.first_pass_inspected_units,
    );
    let final_acceptance_fraction_of_attempted =
        ratio(record.finally_accepted_units, record.attempted_units);
    let accepted_units_per_total_workflow_crew_hour = rate_per_hour(
        record.finally_accepted_units,
        record.total_workflow_crew_minutes,
    )?;
    let accepted_units_per_robot_hour = rate_per_hour(
        record.finally_accepted_units,
        record.robot_operating_minutes,
    )?;

    Ok(EvaluationMetrics {
        first_pass_yield,
        final_acceptance_fraction_of_attempted,
        accepted_units_per_total_workflow_crew_hour,
        accepted_units_per_robot_hour,
        first_pass_inspection_denominator: record.first_pass_inspected_units,
        attempted_denominator: record.attempted_units,
        accepted_numerator: record.finally_accepted_units,
    })
}

fn rate_per_hour(units: u64, minutes: f64) -> Result<Option<f64>, ValidationError> {
    if !minutes.is_finite() || minutes < 0.0 {
        return Err(ValidationError::NonFiniteOrNegativeMetric("minutes"));
    }
    if minutes == 0.0 {
        return Ok(None);
    }
    Ok(Some(units as f64 / (minutes / 60.0)))
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

fn validate_record(record: &EvaluationRecord, errors: &mut Vec<ValidationError>) {
    for (field, value) in [
        ("evaluation_id", record.evaluation_id.as_str()),
        ("match_stratum_id", record.match_stratum_id.as_str()),
        ("task_family", record.task_family.as_str()),
        (
            "functional_unit_class",
            record.functional_unit_class.as_str(),
        ),
        (
            "acceptance_profile_digest",
            record.acceptance_profile_digest.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            errors.push(ValidationError::EmptyField(field));
        }
    }

    let disposition_count = record
        .finally_accepted_units
        .checked_add(record.finally_rejected_units)
        .and_then(|count| count.checked_add(record.uninspected_units))
        .and_then(|count| count.checked_add(record.unresolved_units));

    if record.planned_units == 0
        || record.attempted_units > record.planned_units
        || record.first_pass_inspected_units > record.attempted_units
        || record.first_pass_accepted_units > record.first_pass_inspected_units
        || record.finally_accepted_units > record.attempted_units
        || record.finally_rejected_units > record.attempted_units
        || disposition_count.is_none_or(|count| count > record.attempted_units)
    {
        errors.push(ValidationError::InvalidCount("unit_denominator_invariant"));
    }

    for (name, value) in [
        (
            "total_workflow_crew_minutes",
            record.total_workflow_crew_minutes,
        ),
        ("robot_operating_minutes", record.robot_operating_minutes),
    ] {
        if !value.is_finite() || value < 0.0 {
            errors.push(ValidationError::NonFiniteOrNegativeMetric(name));
        }
    }

    if record.work_item_ids.is_empty() {
        errors.push(ValidationError::InvalidEvaluation("work_item_ids_required"));
    }
    validate_unique_ids(&record.work_item_ids, "work_item_ids", errors);
}

fn validate_split_guard(guard: &SplitGuard, errors: &mut Vec<ValidationError>) {
    if guard.dimension.trim().is_empty() {
        errors.push(ValidationError::EmptyDimension);
        return;
    }

    validate_unique_ids(&guard.training_ids, "training_ids", errors);
    validate_unique_ids(&guard.adaptation_ids, "adaptation_ids", errors);
    validate_unique_ids(&guard.evaluation_ids, "evaluation_ids", errors);
    if guard.training_ids.is_empty() {
        errors.push(ValidationError::MissingSplitPartition("training_ids"));
    }
    if guard.evaluation_ids.is_empty() {
        errors.push(ValidationError::MissingSplitPartition("evaluation_ids"));
    }

    let training: BTreeSet<&str> = guard.training_ids.iter().map(String::as_str).collect();
    let adaptation: BTreeSet<&str> = guard.adaptation_ids.iter().map(String::as_str).collect();
    let evaluation: BTreeSet<&str> = guard.evaluation_ids.iter().map(String::as_str).collect();

    for id in training.intersection(&adaptation) {
        errors.push(ValidationError::TrainingAdaptationLeakage {
            dimension: guard.dimension.clone(),
            id: (*id).to_owned(),
        });
    }
    for id in training.intersection(&evaluation) {
        errors.push(ValidationError::TrainingEvaluationLeakage {
            dimension: guard.dimension.clone(),
            id: (*id).to_owned(),
        });
    }
    if !guard.allow_adaptation_evaluation_domain_overlap {
        for id in adaptation.intersection(&evaluation) {
            errors.push(ValidationError::AdaptationEvaluationLeakage {
                dimension: guard.dimension.clone(),
                id: (*id).to_owned(),
            });
        }
    }
}

fn validate_unique_ids(values: &[String], _field: &'static str, errors: &mut Vec<ValidationError>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            errors.push(ValidationError::EmptyField("split_or_work_item_id"));
        } else if !seen.insert(value.as_str()) {
            errors.push(ValidationError::DuplicateWorkItem { id: value.clone() });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, role: EvaluationRole) -> EvaluationRecord {
        EvaluationRecord {
            evaluation_id: id.to_owned(),
            role,
            work_item_ids: vec![format!("work-{id}")],
            match_stratum_id: "layout-level-2".to_owned(),
            task_family: "layout".to_owned(),
            functional_unit_class: "accepted-point".to_owned(),
            acceptance_profile_digest: "sha256:acceptance-v1".to_owned(),
            planned_units: 100,
            attempted_units: 90,
            first_pass_inspected_units: 80,
            first_pass_accepted_units: 72,
            finally_accepted_units: 85,
            finally_rejected_units: 5,
            uninspected_units: 0,
            unresolved_units: 0,
            total_workflow_crew_minutes: 600.0,
            robot_operating_minutes: 300.0,
        }
    }

    fn study() -> TransferStudy {
        let mut source = record("source-1", EvaluationRole::SourceReference);
        source.finally_accepted_units = 80;
        source.total_workflow_crew_minutes = 600.0;

        let mut target = record("target-1", EvaluationRole::TargetEvaluation);
        target.finally_accepted_units = 60;
        target.total_workflow_crew_minutes = 600.0;

        TransferStudy {
            study_id: "transfer-study-1".to_owned(),
            regime: TransferRegime::BoundedOfflineAdaptation,
            source_model_digest: "sha256:model-source".to_owned(),
            target_model_digest: "sha256:model-target".to_owned(),
            source_robot_config_digest: "sha256:robot-config-source".to_owned(),
            target_robot_config_digest: "sha256:robot-config-target".to_owned(),
            calibration_change_receipt_ref: None,
            adaptation_budget: AdaptationBudget {
                max_target_label_count: 10,
                max_calibration_person_hours: 1.0,
                max_engineering_person_hours: 1.5,
                max_operator_person_hours: 1.0,
                max_inspection_person_hours: 0.5,
            },
            adaptation_effort: AdaptationEffort {
                target_label_count: 8,
                calibration_person_hours: 0.75,
                engineering_person_hours: 1.0,
                operator_person_hours: 0.75,
                inspection_person_hours: 0.4,
            },
            split_guards: vec![
                SplitGuard {
                    dimension: "site_id".to_owned(),
                    training_ids: vec!["site-source".to_owned()],
                    adaptation_ids: vec!["site-target".to_owned()],
                    evaluation_ids: vec!["site-target".to_owned()],
                    allow_adaptation_evaluation_domain_overlap: true,
                },
                SplitGuard {
                    dimension: "work_package_id".to_owned(),
                    training_ids: vec!["wp-source".to_owned()],
                    adaptation_ids: vec!["wp-target-adapt".to_owned()],
                    evaluation_ids: vec!["wp-target-eval".to_owned()],
                    allow_adaptation_evaluation_domain_overlap: false,
                },
            ],
            target_adaptation_work_item_ids: vec!["calibration-task-1".to_owned()],
            target_evaluation_work_item_ids: vec!["evaluation-task-1".to_owned()],
            source,
            target,
        }
    }

    #[test]
    fn valid_bounded_adaptation_study_passes_integrity_checks() {
        assert_eq!(study().validate(), Ok(()));
    }

    #[test]
    fn source_training_and_target_evaluation_site_leakage_is_rejected() {
        let mut input = study();
        input.split_guards[0].training_ids = vec!["site-target".to_owned()];
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::TrainingEvaluationLeakage {
                dimension: "site_id".to_owned(),
                id: "site-target".to_owned()
            })
        ));
    }

    #[test]
    fn target_adaptation_and_evaluation_must_separate_work_items() {
        let mut input = study();
        input.target_adaptation_work_item_ids = vec!["same-task".to_owned()];
        input.target_evaluation_work_item_ids = vec!["same-task".to_owned()];
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::AdaptationEvaluationWorkItemLeakage {
                id: "same-task".to_owned()
            })
        ));
    }

    #[test]
    fn domain_overlap_can_be_allowed_only_for_declared_adaptation_dimension() {
        let mut input = study();
        input.split_guards[0].allow_adaptation_evaluation_domain_overlap = false;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::AdaptationEvaluationLeakage {
                dimension: "site_id".to_owned(),
                id: "site-target".to_owned()
            })
        ));
    }

    #[test]
    fn duplicate_split_ids_are_rejected() {
        let mut input = study();
        input.split_guards[1].evaluation_ids = vec!["wp-eval".to_owned(), "wp-eval".to_owned()];
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.iter().any(|error| matches!(
                error,
                ValidationError::DuplicateWorkItem { id } if id == "wp-eval"
            ))
        ));
    }

    #[test]
    fn zero_shot_must_keep_model_digest_and_use_no_target_labels() {
        let mut input = study();
        input.regime = TransferRegime::ZeroShot;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidRegime(
                "zero_shot_must_not_use_target_task_labels"
            )) && errors.contains(&ValidationError::InvalidRegime(
                "zero_shot_model_digest_must_remain_frozen"
            ))
        ));
    }

    #[test]
    fn calibration_only_cannot_update_policy_or_use_target_task_labels() {
        let mut input = study();
        input.regime = TransferRegime::CalibrationOnly;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidRegime(
                "calibration_only_must_not_use_target_task_labels"
            )) && errors.contains(&ValidationError::InvalidRegime(
                "calibration_only_policy_digest_must_remain_frozen"
            ))
        ));
    }


    #[test]
    fn zero_shot_requires_frozen_robot_configuration() {
        let mut input = study();
        input.regime = TransferRegime::ZeroShot;
        input.adaptation_effort.target_label_count = 0;
        input.target_model_digest = input.source_model_digest.clone();
        input.target_robot_config_digest = "sha256:different-config".to_owned();
        input.target_adaptation_work_item_ids.clear();
        input.adaptation_effort.target_label_count = 0;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidRegime(
                "zero_shot_robot_config_digest_must_remain_frozen"
            ))
        ));
    }

    #[test]
    fn calibration_only_config_change_requires_explicit_receipt() {
        let mut input = study();
        input.regime = TransferRegime::CalibrationOnly;
        input.adaptation_effort.target_label_count = 0;
        input.target_model_digest = input.source_model_digest.clone();
        input.adaptation_effort.target_label_count = 0;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidRegime(
                "calibration_only_config_change_requires_receipt"
            ))
        ));

        input.calibration_change_receipt_ref = Some("calibration-receipt-1".to_owned());
        assert_eq!(input.validate(), Ok(()));
    }

    #[test]
    fn bounded_offline_adaptation_enforces_frozen_label_budget() {
        let mut input = study();
        input.adaptation_effort.target_label_count = 11;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::LabelBudgetExceeded {
                budget: 10,
                actual: 11
            })
        ));
    }

    #[test]
    fn human_supervised_adaptation_requires_engineering_effort_record() {
        let mut input = study();
        input.regime = TransferRegime::HumanSupervisedMethodAdaptation;
        input.adaptation_effort.engineering_person_hours = 0.0;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidRegime(
                "human_supervised_adaptation_requires_recorded_engineering_effort"
            ))
        ));
    }

    #[test]
    fn mismatched_task_family_blocks_transfer_retention() {
        let mut input = study();
        input.target.task_family = "piling".to_owned();
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::MismatchedComparison("task_family"))
        ));
    }

    #[test]
    fn mismatched_functional_unit_blocks_transfer_retention() {
        let mut input = study();
        input.target.functional_unit_class = "accepted-wall-area".to_owned();
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::MismatchedComparison(
                "functional_unit_class"
            ))
        ));
    }

    #[test]
    fn mismatched_acceptance_profile_blocks_transfer_retention() {
        let mut input = study();
        input.target.acceptance_profile_digest = "sha256:other".to_owned();
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::MismatchedComparison(
                "acceptance_profile_digest"
            ))
        ));
    }

    #[test]
    fn first_pass_yield_uses_inspected_not_attempted_denominator() {
        let input = study();
        let metrics = calculate_metrics(&input.target).expect("valid metrics");
        assert_eq!(metrics.first_pass_yield, Some(0.9));
        assert_eq!(metrics.first_pass_inspection_denominator, 80);
    }

    #[test]
    fn zero_first_pass_denominator_is_not_estimable() {
        let mut input = study();
        input.source.first_pass_inspected_units = 0;
        input.source.first_pass_accepted_units = 0;
        let metrics = calculate_metrics(&input.source).expect("valid metrics");
        assert_eq!(metrics.first_pass_yield, None);
    }

    #[test]
    fn zero_crew_time_makes_accepted_output_rate_not_estimable() {
        let mut input = study();
        input.target.total_workflow_crew_minutes = 0.0;
        let metrics = calculate_metrics(&input.target).expect("valid metrics");
        assert_eq!(metrics.accepted_units_per_total_workflow_crew_hour, None);
    }

    #[test]
    fn accepted_output_per_crew_hour_uses_total_workflow_time() {
        let input = study();
        let metrics = calculate_metrics(&input.target).expect("valid metrics");
        assert_eq!(
            metrics.accepted_units_per_total_workflow_crew_hour,
            Some(6.0)
        );
        assert_eq!(metrics.accepted_units_per_robot_hour, Some(12.0));
    }

    #[test]
    fn adaptation_budget_hours_are_not_double_counted_as_effort() {
        let input = study();
        let burden = input.adaptation_burden().expect("valid adaptation burden");
        assert_eq!(burden.total_recorded_person_hours, 2.9);
        assert_eq!(burden.calibration_person_hours, 0.75);
        assert_eq!(burden.engineering_person_hours, 1.0);
    }

    #[test]
    fn transfer_retention_uses_matched_accepted_output_rates() {
        let input = study();
        let retention = input.transfer_retention().expect("matched study");
        assert_eq!(retention.estimability, Estimability::Estimable);
        assert_eq!(retention.source_accepted_units_per_crew_hour, Some(8.0));
        assert_eq!(retention.target_accepted_units_per_crew_hour, Some(6.0));
        assert_eq!(retention.target_to_source_ratio, Some(0.75));
    }

    #[test]
    fn zero_source_rate_makes_retention_not_estimable_not_infinite() {
        let mut input = study();
        input.source.finally_accepted_units = 0;
        let retention = input.transfer_retention().expect("valid but zero source");
        assert_eq!(retention.estimability, Estimability::NotEstimable);
        assert_eq!(retention.target_to_source_ratio, None);
    }

    #[test]
    fn incomplete_first_pass_and_final_disposition_counts_are_rejected() {
        let mut input = study();
        input.target.first_pass_accepted_units = 81;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidCount(
                "unit_denominator_invariant"
            ))
        ));
    }

    #[test]
    fn non_finite_and_negative_adaptation_metrics_are_rejected() {
        let mut input = study();
        input.adaptation_effort.engineering_person_hours = f64::NAN;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::NonFiniteOrNegativeMetric(
                "engineering_person_hours"
            ))
        ));
    }


    #[test]
    fn actual_adaptation_hours_cannot_exceed_declared_effort_budget() {
        let mut input = study();
        input.adaptation_effort.engineering_person_hours = 2.0;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::AdaptationBudgetExceeded {
                field: "engineering_person_hours",
                budget_person_hours: 1.5,
                actual_person_hours: 2.0
            })
        ));
    }

    #[test]
    fn source_and_target_adaptation_work_items_must_be_disjoint() {
        let mut input = study();
        input.target_adaptation_work_item_ids = input.source.work_item_ids.clone();
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::TrainingAdaptationWorkItemLeakage {
                id: "work-source-1".to_owned()
            })
        ));
    }

    #[test]
    fn source_and_target_evaluation_work_items_must_be_disjoint() {
        let mut input = study();
        input.target.work_item_ids = input.source.work_item_ids.clone();
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::TrainingEvaluationWorkItemLeakage {
                id: "work-source-1".to_owned()
            })
        ));
    }

    #[test]
    fn final_disposition_subcounts_cannot_exceed_attempted_units() {
        let mut input = study();
        input.target.uninspected_units = 85;
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::InvalidCount(
                "unit_denominator_invariant"
            ))
        ));
    }

    #[test]
    fn duplicate_work_items_inside_evaluation_are_rejected() {
        let mut input = study();
        input.target.work_item_ids = vec!["repeat".to_owned(), "repeat".to_owned()];
        assert!(matches!(
            input.validate(),
            Err(errors) if errors.contains(&ValidationError::DuplicateWorkItem {
                id: "repeat".to_owned()
            })
        ));
    }

    #[test]
    fn output_evaluations_must_be_matched_on_declared_stratum() {
        let mut input = study();
        input.target.match_stratum_id = "different-level".to_owned();
        assert!(matches!(
            input.transfer_retention(),
            Err(errors) if errors.contains(&ValidationError::MismatchedComparison(
                "match_stratum_id"
            ))
        ));
    }
}

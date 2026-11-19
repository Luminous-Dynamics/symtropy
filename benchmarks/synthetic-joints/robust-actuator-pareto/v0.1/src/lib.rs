#![forbid(unsafe_code)]

//! Evidence-lane-separated robust feasibility and Pareto filtering.
//!
//! This M1 reference is deliberately not an optimizer. It classifies candidate
//! design records against declared worst-case hard-constraint margins, then
//! computes non-dominated sets only within comparable evidence lanes.
//! Unknown/infeasible candidates are retained as dispositions but excluded
//! from Pareto ranking.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectiveDirection {
    Minimize,
    Maximize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    ModelDerived,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceLane {
    Independent,
    SourceReported,
    Conditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDisposition {
    Infeasible,
    Unknown,
    Feasible(EvidenceLane),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveDefinition {
    pub name: String,
    pub unit: String,
    pub direction: ObjectiveDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardConstraintDefinition {
    pub name: String,
    pub unit: String,
    /// A nonnegative minimum robust margin is required.
    pub interpretation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricEvidence {
    pub name: String,
    pub value: Option<f64>,
    pub unit: String,
    pub evidence_class: EvidenceClass,
    pub source_reference: Option<String>,
    pub method: Option<String>,
    pub configuration_digest: String,
    pub operating_condition_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintEvidence {
    pub name: String,
    /// Worst-case margin over the declared bounded stress set. Positive passes.
    pub minimum_robust_margin: Option<f64>,
    pub unit: String,
    pub evidence_class: EvidenceClass,
    pub source_reference: Option<String>,
    pub method: Option<String>,
    pub configuration_digest: String,
    pub operating_condition_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesignCandidate {
    pub candidate_id: String,
    pub configuration_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    pub objectives: Vec<MetricEvidence>,
    pub hard_constraints: Vec<ConstraintEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateAssessment {
    pub candidate_id: String,
    pub disposition: CandidateDisposition,
    pub violated_constraints: Vec<String>,
    pub unknown_fields: Vec<String>,
    pub dominated_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceLaneFront {
    pub lane: EvidenceLane,
    pub candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParetoReport {
    pub assessments: Vec<CandidateAssessment>,
    pub fronts: Vec<EvidenceLaneFront>,
    pub excluded_infeasible_ids: Vec<String>,
    pub excluded_unknown_ids: Vec<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_evidence(
    evidence_class: EvidenceClass,
    source_reference: &Option<String>,
    method: &Option<String>,
    configuration_digest: &str,
    expected_configuration_digest: &str,
    operating_condition_digest: &str,
    expected_operating_condition_digest: &str,
    label: &str,
) -> Result<(), String> {
    if configuration_digest != expected_configuration_digest {
        return Err(format!("{label} belongs to a different candidate configuration"));
    }
    if operating_condition_digest != expected_operating_condition_digest {
        return Err(format!("{label} belongs to different operating conditions"));
    }
    if evidence_class != EvidenceClass::Unknown {
        if source_reference.as_deref().is_none_or(|value| !nonempty(value)) {
            return Err(format!("{label} has a non-unknown evidence class but no source reference"));
        }
        if method.as_deref().is_none_or(|value| !nonempty(value)) {
            return Err(format!("{label} has a non-unknown evidence class but no method"));
        }
    }
    Ok(())
}

fn lane_for_evidence(classes: &[EvidenceClass]) -> Option<EvidenceLane> {
    if classes.is_empty() || classes.contains(&EvidenceClass::Unknown) {
        return None;
    }
    if classes.iter().all(|class| *class == EvidenceClass::IndependentPhysical) {
        Some(EvidenceLane::Independent)
    } else if classes.iter().all(|class| {
        matches!(
            class,
            EvidenceClass::IndependentPhysical | EvidenceClass::SourceAuthorPhysical
        )
    }) {
        Some(EvidenceLane::SourceReported)
    } else {
        Some(EvidenceLane::Conditional)
    }
}

fn validate_definitions(
    objectives: &[ObjectiveDefinition],
    constraints: &[HardConstraintDefinition],
) -> Result<(), String> {
    if objectives.is_empty() || constraints.is_empty() {
        return Err("at least one objective and one hard constraint are required".to_owned());
    }

    let mut objective_names = BTreeSet::new();
    for objective in objectives {
        if !nonempty(&objective.name) || !nonempty(&objective.unit) {
            return Err("objective names and units must be nonempty".to_owned());
        }
        if !objective_names.insert(objective.name.as_str()) {
            return Err(format!("duplicate objective definition {:?}", objective.name));
        }
    }

    let mut constraint_names = BTreeSet::new();
    for constraint in constraints {
        if !nonempty(&constraint.name)
            || !nonempty(&constraint.unit)
            || !nonempty(&constraint.interpretation)
        {
            return Err("hard-constraint names, units and interpretation must be nonempty".to_owned());
        }
        if !constraint_names.insert(constraint.name.as_str()) {
            return Err(format!("duplicate hard-constraint definition {:?}", constraint.name));
        }
    }
    Ok(())
}

fn assess_candidate(
    candidate: &DesignCandidate,
    objectives: &[ObjectiveDefinition],
    constraints: &[HardConstraintDefinition],
) -> Result<CandidateAssessment, String> {
    if !nonempty(&candidate.candidate_id)
        || !nonempty(&candidate.configuration_digest)
        || !nonempty(&candidate.operating_condition_digest)
        || !nonempty(&candidate.mission_set_digest)
    {
        return Err("candidate, configuration, condition and mission-set identities are required".to_owned());
    }
    if candidate.objectives.len() != objectives.len() {
        return Err(format!(
            "candidate {:?} has {} objective records; expected {}",
            candidate.candidate_id,
            candidate.objectives.len(),
            objectives.len()
        ));
    }
    if candidate.hard_constraints.len() != constraints.len() {
        return Err(format!(
            "candidate {:?} has {} hard-constraint records; expected {}",
            candidate.candidate_id,
            candidate.hard_constraints.len(),
            constraints.len()
        ));
    }

    let mut classes = Vec::with_capacity(objectives.len() + constraints.len());
    let mut unknown_fields = Vec::new();
    let mut violated_constraints = Vec::new();

    for (index, (definition, observation)) in objectives.iter().zip(&candidate.objectives).enumerate() {
        if observation.name != definition.name {
            return Err(format!(
                "candidate {:?} objective name {:?} does not match declared name {:?}",
                candidate.candidate_id, observation.name, definition.name
            ));
        }
        if observation.unit != definition.unit {
            return Err(format!(
                "candidate {:?} objective {} unit {:?} does not match declared unit {:?}",
                candidate.candidate_id, definition.name, observation.unit, definition.unit
            ));
        }
        validate_evidence(
            observation.evidence_class,
            &observation.source_reference,
            &observation.method,
            &observation.configuration_digest,
            &candidate.configuration_digest,
            &observation.operating_condition_digest,
            &candidate.operating_condition_digest,
            &format!("objective {:?}", definition.name),
        )?;
        if observation.value.is_some_and(|value| !value.is_finite()) {
            return Err(format!("candidate {:?} objective {} is non-finite", candidate.candidate_id, definition.name));
        }
        if observation.value.is_none() || observation.evidence_class == EvidenceClass::Unknown {
            unknown_fields.push(format!("objective[{index}]={}", definition.name));
        }
        classes.push(observation.evidence_class);
    }

    for (index, (definition, observation)) in constraints.iter().zip(&candidate.hard_constraints).enumerate() {
        if observation.name != definition.name {
            return Err(format!(
                "candidate {:?} constraint name {:?} does not match declared name {:?}",
                candidate.candidate_id, observation.name, definition.name
            ));
        }
        if observation.unit != definition.unit {
            return Err(format!(
                "candidate {:?} constraint {} unit {:?} does not match declared unit {:?}",
                candidate.candidate_id, definition.name, observation.unit, definition.unit
            ));
        }
        validate_evidence(
            observation.evidence_class,
            &observation.source_reference,
            &observation.method,
            &observation.configuration_digest,
            &candidate.configuration_digest,
            &observation.operating_condition_digest,
            &candidate.operating_condition_digest,
            &format!("constraint {:?}", definition.name),
        )?;
        if observation.minimum_robust_margin.is_some_and(|margin| !margin.is_finite()) {
            return Err(format!("candidate {:?} constraint {} has non-finite margin", candidate.candidate_id, definition.name));
        }
        match observation.minimum_robust_margin {
            Some(margin) if margin < 0.0 && observation.evidence_class != EvidenceClass::Unknown => {
                violated_constraints.push(definition.name.clone())
            }
            None => unknown_fields.push(format!("constraint[{index}]={}", definition.name)),
            Some(_) => {}
        }
        if observation.evidence_class == EvidenceClass::Unknown {
            unknown_fields.push(format!("constraint_evidence[{index}]={}", definition.name));
        }
        classes.push(observation.evidence_class);
    }

    let disposition = if !violated_constraints.is_empty() {
        CandidateDisposition::Infeasible
    } else if !unknown_fields.is_empty() {
        CandidateDisposition::Unknown
    } else if let Some(lane) = lane_for_evidence(&classes) {
        CandidateDisposition::Feasible(lane)
    } else {
        CandidateDisposition::Unknown
    };

    Ok(CandidateAssessment {
        candidate_id: candidate.candidate_id.clone(),
        disposition,
        violated_constraints,
        unknown_fields,
        dominated_by: Vec::new(),
    })
}

fn dominates(
    left: &DesignCandidate,
    right: &DesignCandidate,
    definitions: &[ObjectiveDefinition],
) -> bool {
    let mut strictly_better = false;

    for ((left_metric, right_metric), definition) in left
        .objectives
        .iter()
        .zip(&right.objectives)
        .zip(definitions)
    {
        let (Some(left_value), Some(right_value)) = (left_metric.value, right_metric.value) else {
            return false;
        };

        match definition.direction {
            ObjectiveDirection::Minimize => {
                if left_value > right_value {
                    return false;
                }
                if left_value < right_value {
                    strictly_better = true;
                }
            }
            ObjectiveDirection::Maximize => {
                if left_value < right_value {
                    return false;
                }
                if left_value > right_value {
                    strictly_better = true;
                }
            }
        }
    }
    strictly_better
}

/// Classifies candidate robust feasibility and computes separate Pareto fronts
/// for independent, source-reported and conditional evidence lanes.
///
/// An infeasible candidate is excluded when any known worst-case hard margin
/// is negative. An unknown candidate is excluded when any required metric,
/// margin or evidence source is missing. Pareto dominance is calculated only
/// among candidates in the same evidence lane.
pub fn robust_pareto_report(
    candidates: &[DesignCandidate],
    objectives: &[ObjectiveDefinition],
    constraints: &[HardConstraintDefinition],
) -> Result<ParetoReport, String> {
    validate_definitions(objectives, constraints)?;

    let mut ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !ids.insert(candidate.candidate_id.as_str()) {
            return Err(format!("duplicate candidate_id {:?}", candidate.candidate_id));
        }
        assessments.push(assess_candidate(candidate, objectives, constraints)?);
    }

    let candidate_index: BTreeMap<&str, usize> = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| (candidate.candidate_id.as_str(), index))
        .collect();

    for index in 0..assessments.len() {
        let CandidateDisposition::Feasible(lane) = assessments[index].disposition else {
            continue;
        };
        let candidate_id = assessments[index].candidate_id.clone();
        let right_index = *candidate_index.get(candidate_id.as_str()).unwrap();
        let right = &candidates[right_index];
        let mut dominated_by = Vec::new();

        for other in candidates {
            if other.candidate_id == candidate_id {
                continue;
            }
            let other_index = *candidate_index.get(other.candidate_id.as_str()).unwrap();
            if assessments[other_index].disposition == CandidateDisposition::Feasible(lane)
                && dominates(other, right, objectives)
            {
                dominated_by.push(other.candidate_id.clone());
            }
        }
        dominated_by.sort();
        assessments[index].dominated_by = dominated_by;
    }

    let mut fronts = Vec::new();
    for lane in [
        EvidenceLane::Independent,
        EvidenceLane::SourceReported,
        EvidenceLane::Conditional,
    ] {
        let mut candidate_ids: Vec<String> = assessments
            .iter()
            .filter(|assessment| {
                assessment.disposition == CandidateDisposition::Feasible(lane)
                    && assessment.dominated_by.is_empty()
            })
            .map(|assessment| assessment.candidate_id.clone())
            .collect();
        candidate_ids.sort();
        if !candidate_ids.is_empty() {
            fronts.push(EvidenceLaneFront { lane, candidate_ids });
        }
    }

    let excluded_infeasible_ids = assessments
        .iter()
        .filter(|assessment| assessment.disposition == CandidateDisposition::Infeasible)
        .map(|assessment| assessment.candidate_id.clone())
        .collect();
    let excluded_unknown_ids = assessments
        .iter()
        .filter(|assessment| assessment.disposition == CandidateDisposition::Unknown)
        .map(|assessment| assessment.candidate_id.clone())
        .collect();

    Ok(ParetoReport {
        assessments,
        fronts,
        excluded_infeasible_ids,
        excluded_unknown_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "configuration-a";
    const CONDITION: &str = "48V-25C-cooling-a";
    const MISSIONS: &str = "mission-set-v1";

    fn objective_defs() -> Vec<ObjectiveDefinition> {
        vec![
            ObjectiveDefinition { name: "installed_mass".to_owned(), unit: "kg".to_owned(), direction: ObjectiveDirection::Minimize },
            ObjectiveDefinition { name: "task_energy".to_owned(), unit: "J".to_owned(), direction: ObjectiveDirection::Minimize },
            ObjectiveDefinition { name: "task_coverage".to_owned(), unit: "fraction".to_owned(), direction: ObjectiveDirection::Maximize },
        ]
    }

    fn constraint_defs() -> Vec<HardConstraintDefinition> {
        vec![
            HardConstraintDefinition { name: "thermal_margin".to_owned(), unit: "degC".to_owned(), interpretation: "Minimum robust margin to declared thermal limit".to_owned() },
            HardConstraintDefinition { name: "torque_margin".to_owned(), unit: "Nm".to_owned(), interpretation: "Minimum robust margin over every required trajectory sample".to_owned() },
        ]
    }

    fn metric(value: f64, unit: &str, class: EvidenceClass) -> MetricEvidence {
        MetricEvidence {
            name: String::new(),
            value: Some(value),
            unit: unit.to_owned(),
            evidence_class: class,
            source_reference: Some("https://example.invalid/fixture".to_owned()),
            method: Some("synthetic known-answer fixture".to_owned()),
            configuration_digest: CONFIG.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
        }
    }

    fn constraint(margin: f64, unit: &str, class: EvidenceClass) -> ConstraintEvidence {
        ConstraintEvidence {
            name: String::new(),
            minimum_robust_margin: Some(margin),
            unit: unit.to_owned(),
            evidence_class: class,
            source_reference: Some("https://example.invalid/fixture".to_owned()),
            method: Some("synthetic bounded stress-set oracle".to_owned()),
            configuration_digest: CONFIG.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
        }
    }

    fn candidate(
        id: &str,
        mass: f64,
        energy: f64,
        coverage: f64,
        class: EvidenceClass,
    ) -> DesignCandidate {
        DesignCandidate {
            candidate_id: id.to_owned(),
            configuration_digest: CONFIG.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSIONS.to_owned(),
            objectives: vec![
                MetricEvidence { name: "installed_mass".to_owned(), ..metric(mass, "kg", class) },
                MetricEvidence { name: "task_energy".to_owned(), ..metric(energy, "J", class) },
                MetricEvidence { name: "task_coverage".to_owned(), ..metric(coverage, "fraction", class) },
            ],
            hard_constraints: vec![
                ConstraintEvidence { name: "thermal_margin".to_owned(), ..constraint(5.0, "degC", class) },
                ConstraintEvidence { name: "torque_margin".to_owned(), ..constraint(2.0, "Nm", class) },
            ],
        }
    }

    fn assessment<'a>(report: &'a ParetoReport, id: &str) -> &'a CandidateAssessment {
        report.assessments.iter().find(|candidate| candidate.candidate_id == id).unwrap()
    }

    #[test]
    fn pareto_001_dominance_respects_mixed_objective_directions() {
        let a = candidate("A", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        let b = candidate("B", 2.0, 110.0, 0.8, EvidenceClass::IndependentPhysical);
        let report = robust_pareto_report(&[a, b], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(report.fronts.len(), 1);
        assert_eq!(report.fronts[0].candidate_ids, vec!["A"]);
        assert_eq!(assessment(&report, "B").dominated_by, vec!["A"]);
    }

    #[test]
    fn pareto_002_tradeoffs_remain_non_dominated() {
        let a = candidate("A", 1.0, 100.0, 0.8, EvidenceClass::IndependentPhysical);
        let b = candidate("B", 1.2, 90.0, 0.9, EvidenceClass::IndependentPhysical);
        let report = robust_pareto_report(&[a, b], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(report.fronts[0].candidate_ids, vec!["A", "B"]);
    }

    #[test]
    fn pareto_003_evidence_lanes_are_never_cross_dominated() {
        let independent = candidate("independent", 1.0, 100.0, 0.8, EvidenceClass::IndependentPhysical);
        let vendor = candidate("vendor", 0.5, 50.0, 1.0, EvidenceClass::VendorPhysical);
        let source = candidate("source", 1.5, 120.0, 0.75, EvidenceClass::SourceAuthorPhysical);
        let report = robust_pareto_report(&[independent, vendor, source], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(report.fronts.len(), 3);
        assert!(report.fronts.iter().any(|front| front.lane == EvidenceLane::Independent && front.candidate_ids == vec!["independent"]));
        assert!(report.fronts.iter().any(|front| front.lane == EvidenceLane::SourceReported && front.candidate_ids == vec!["source"]));
        assert!(report.fronts.iter().any(|front| front.lane == EvidenceLane::Conditional && front.candidate_ids == vec!["vendor"]));
    }

    #[test]
    fn pareto_004_known_hard_constraint_violation_excludes_candidate() {
        let mut bad = candidate("bad", 0.1, 1.0, 1.0, EvidenceClass::IndependentPhysical);
        bad.hard_constraints[0].minimum_robust_margin = Some(-0.1);
        let report = robust_pareto_report(&[bad], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(assessment(&report, "bad").disposition, CandidateDisposition::Infeasible);
        assert_eq!(report.excluded_infeasible_ids, vec!["bad"]);
        assert!(report.fronts.is_empty());
    }

    #[test]
    fn pareto_005_unknown_margin_or_evidence_is_not_promoted() {
        let mut unknown = candidate("unknown", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        unknown.hard_constraints[1].minimum_robust_margin = None;
        let report = robust_pareto_report(&[unknown], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(assessment(&report, "unknown").disposition, CandidateDisposition::Unknown);
        assert_eq!(report.excluded_unknown_ids, vec!["unknown"]);
    }

    #[test]
    fn pareto_006_known_violation_precedes_other_unknowns() {
        let mut candidate = candidate("mixed", 1.0, 100.0, 0.9, EvidenceClass::Unknown);
        candidate.hard_constraints[0].minimum_robust_margin = Some(-1.0);
        let report = robust_pareto_report(&[candidate], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(assessment(&report, "mixed").disposition, CandidateDisposition::Infeasible);
    }

    #[test]
    fn pareto_007_duplicate_candidate_ids_reject() {
        let a = candidate("same", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        let b = candidate("same", 2.0, 110.0, 0.8, EvidenceClass::IndependentPhysical);
        assert!(robust_pareto_report(&[a, b], &objective_defs(), &constraint_defs()).is_err());
    }

    #[test]
    fn pareto_008_metric_dimension_and_unit_mismatch_reject() {
        let mut short = candidate("short", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        short.objectives.pop();
        assert!(robust_pareto_report(&[short], &objective_defs(), &constraint_defs()).is_err());

        let mut wrong_unit = candidate("wrong-unit", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        wrong_unit.objectives[0].unit = "g".to_owned();
        assert!(robust_pareto_report(&[wrong_unit], &objective_defs(), &constraint_defs()).is_err());
    }

    #[test]
    fn pareto_009_nonfinite_metrics_and_margins_reject() {
        let mut bad_metric = candidate("bad-metric", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        bad_metric.objectives[0].value = Some(f64::NAN);
        assert!(robust_pareto_report(&[bad_metric], &objective_defs(), &constraint_defs()).is_err());

        let mut bad_margin = candidate("bad-margin", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        bad_margin.hard_constraints[0].minimum_robust_margin = Some(f64::INFINITY);
        assert!(robust_pareto_report(&[bad_margin], &objective_defs(), &constraint_defs()).is_err());
    }

    #[test]
    fn pareto_010_equal_objective_vectors_do_not_dominate_each_other() {
        let a = candidate("A", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        let b = candidate("B", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        let report = robust_pareto_report(&[a, b], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(report.fronts[0].candidate_ids, vec!["A", "B"]);
    }

    #[test]
    fn pareto_011_missing_objective_value_is_unknown() {
        let mut candidate = candidate("unknown-objective", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        candidate.objectives[2].value = None;
        let report = robust_pareto_report(&[candidate], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(assessment(&report, "unknown-objective").disposition, CandidateDisposition::Unknown);
    }

    #[test]
    fn pareto_012_unknown_source_must_not_claim_a_measured_lane() {
        let mut candidate = candidate("no-source", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        candidate.objectives[0].source_reference = None;
        assert!(robust_pareto_report(&[candidate], &objective_defs(), &constraint_defs()).is_err());
    }

    #[test]
    fn pareto_013_evidence_from_another_configuration_rejects() {
        let mut candidate = candidate("stale", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        candidate.objectives[0].configuration_digest = "old-config".to_owned();
        assert!(robust_pareto_report(&[candidate], &objective_defs(), &constraint_defs()).is_err());
    }

    #[test]
    fn pareto_014_all_candidates_are_retained_as_assessments() {
        let good = candidate("good", 1.0, 100.0, 0.9, EvidenceClass::IndependentPhysical);
        let mut bad = candidate("bad", 0.5, 50.0, 1.0, EvidenceClass::IndependentPhysical);
        bad.hard_constraints[0].minimum_robust_margin = Some(-0.5);
        let report = robust_pareto_report(&[good, bad], &objective_defs(), &constraint_defs()).unwrap();
        assert_eq!(report.assessments.len(), 2);
        assert_eq!(report.excluded_infeasible_ids, vec!["bad"]);
    }
}

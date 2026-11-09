//! Finite fault-tolerant morphology design matrix validator and Pareto auditor.
//!
//! Compares candidates only inside one frozen campaign/mission/evidence context.
//! Hard physical and assurance gates are non-tradable. No weighted design score,
//! topology interpolation, or physical-safety computation is performed here.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Valid,
    Rejected,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkContext {
    pub campaign_digest: String,
    pub mission_digest: String,
    pub fault_manifest_digest: String,
    pub terrain_manifest_digest: String,
    pub metric_schema_digest: String,
    pub stratum_id: String,
    pub evidence_tier: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignIdentity {
    pub candidate_id: String,
    pub design_family: String,
    pub morphology_digest: String,
    pub topology_id: String,
    pub actuator_architecture_id: String,
    pub transmission_architecture_id: String,
    pub power_architecture_id: String,
    pub sensing_architecture_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Maximize,
    Minimize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricDefinition {
    pub metric_id: String,
    pub unit_id: String,
    pub direction: ObjectiveDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricValue {
    pub metric_id: String,
    pub value: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintRelation {
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardConstraint {
    pub metric_id: String,
    pub relation: ConstraintRelation,
    pub threshold: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvidence {
    pub robust_viability: EvidenceStatus,
    pub recovery: EvidenceStatus,
    pub adverse_outcome_safety: EvidenceStatus,
    pub energy_and_thermal_reserve: EvidenceStatus,
    pub runtime_assurance: EvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignCandidate {
    pub identity: DesignIdentity,
    pub context: BenchmarkContext,
    /// Categorical architecture variables are preserved as metadata. They are
    /// never interpolated as continuous axes by this auditor.
    pub categorical_design_axes: BTreeMap<String, String>,
    pub metrics: Vec<MetricValue>,
    pub gates: GateEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorphologyDesignProblem {
    pub context: BenchmarkContext,
    pub metric_definitions: Vec<MetricDefinition>,
    pub hard_constraints: Vec<HardConstraint>,
    pub candidates: Vec<DesignCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateFailure {
    RobustViability(EvidenceStatus),
    Recovery(EvidenceStatus),
    AdverseOutcomeSafety(EvidenceStatus),
    EnergyAndThermalReserve(EvidenceStatus),
    RuntimeAssurance(EvidenceStatus),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintFailure {
    pub metric_id: String,
    pub observed_value: i64,
    pub relation: ConstraintRelation,
    pub threshold: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateDisposition {
    pub candidate_id: String,
    pub eligible_for_pareto: bool,
    pub gate_failures: Vec<GateFailure>,
    pub constraint_failures: Vec<ConstraintFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParetoRelation {
    pub dominating_candidate_id: String,
    pub dominated_candidate_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorphologyDesignReport {
    pub context: BenchmarkContext,
    pub eligible_candidate_ids: Vec<String>,
    pub pareto_candidate_ids: Vec<String>,
    pub dispositions: Vec<CandidateDisposition>,
    pub dominance_relations: Vec<ParetoRelation>,
    /// Architecture identities remain visible so a reviewer can see that
    /// categorical topologies were compared as discrete designs.
    pub categorical_design_axes: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphologyDesignError {
    EmptyIdentifier,
    NoMetricDefinitions,
    NoCandidates,
    DuplicateMetricDefinition,
    DuplicateHardConstraint,
    ConstraintReferencesUnknownMetric,
    DuplicateCandidateId,
    CandidateContextMismatch,
    DuplicateCategoricalAxis,
    DuplicateMetricValue,
    MissingMetricValue,
    UnknownMetricValue,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn context_is_valid(context: &BenchmarkContext) -> bool {
    [
        context.campaign_digest.as_str(),
        context.mission_digest.as_str(),
        context.fault_manifest_digest.as_str(),
        context.terrain_manifest_digest.as_str(),
        context.metric_schema_digest.as_str(),
        context.stratum_id.as_str(),
        context.evidence_tier.as_str(),
    ]
    .iter()
    .all(|value| nonempty(value))
}

fn candidate_identity_is_valid(identity: &DesignIdentity) -> bool {
    [
        identity.candidate_id.as_str(),
        identity.design_family.as_str(),
        identity.morphology_digest.as_str(),
        identity.topology_id.as_str(),
        identity.actuator_architecture_id.as_str(),
        identity.transmission_architecture_id.as_str(),
        identity.power_architecture_id.as_str(),
        identity.sensing_architecture_id.as_str(),
    ]
    .iter()
    .all(|value| nonempty(value))
}

fn gate_failures(gates: &GateEvidence) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    if gates.robust_viability != EvidenceStatus::Valid {
        failures.push(GateFailure::RobustViability(gates.robust_viability));
    }
    if gates.recovery != EvidenceStatus::Valid {
        failures.push(GateFailure::Recovery(gates.recovery));
    }
    if gates.adverse_outcome_safety != EvidenceStatus::Valid {
        failures.push(GateFailure::AdverseOutcomeSafety(
            gates.adverse_outcome_safety,
        ));
    }
    if gates.energy_and_thermal_reserve != EvidenceStatus::Valid {
        failures.push(GateFailure::EnergyAndThermalReserve(
            gates.energy_and_thermal_reserve,
        ));
    }
    if gates.runtime_assurance != EvidenceStatus::Valid {
        failures.push(GateFailure::RuntimeAssurance(
            gates.runtime_assurance,
        ));
    }
    failures
}

fn validate_metric_values(
    candidate: &DesignCandidate,
    definitions: &[MetricDefinition],
) -> Result<BTreeMap<&str, i64>, MorphologyDesignError> {
    let definition_ids: BTreeSet<&str> =
        definitions.iter().map(|item| item.metric_id.as_str()).collect();
    let mut values = BTreeMap::new();
    for metric in &candidate.metrics {
        if !nonempty(&metric.metric_id) {
            return Err(MorphologyDesignError::EmptyIdentifier);
        }
        if !definition_ids.contains(metric.metric_id.as_str()) {
            return Err(MorphologyDesignError::UnknownMetricValue);
        }
        if values
            .insert(metric.metric_id.as_str(), metric.value)
            .is_some()
        {
            return Err(MorphologyDesignError::DuplicateMetricValue);
        }
    }
    if values.len() != definitions.len() {
        return Err(MorphologyDesignError::MissingMetricValue);
    }
    for definition in definitions {
        if !values.contains_key(definition.metric_id.as_str()) {
            return Err(MorphologyDesignError::MissingMetricValue);
        }
    }
    Ok(values)
}

fn constraint_failures(
    values: &BTreeMap<&str, i64>,
    constraints: &[HardConstraint],
) -> Result<Vec<ConstraintFailure>, MorphologyDesignError> {
    let mut failures = Vec::new();
    for constraint in constraints {
        let value = values
            .get(constraint.metric_id.as_str())
            .copied()
            .ok_or(MorphologyDesignError::ConstraintReferencesUnknownMetric)?;
        let passes = match constraint.relation {
            ConstraintRelation::LessThanOrEqual => value <= constraint.threshold,
            ConstraintRelation::GreaterThanOrEqual => value >= constraint.threshold,
        };
        if !passes {
            failures.push(ConstraintFailure {
                metric_id: constraint.metric_id.clone(),
                observed_value: value,
                relation: constraint.relation,
                threshold: constraint.threshold,
            });
        }
    }
    failures.sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
    Ok(failures)
}

fn dominates(
    left: &BTreeMap<&str, i64>,
    right: &BTreeMap<&str, i64>,
    definitions: &[MetricDefinition],
) -> bool {
    let mut strictly_better = false;
    for definition in definitions {
        let Some(left_value) = left.get(definition.metric_id.as_str()) else {
            return false;
        };
        let Some(right_value) = right.get(definition.metric_id.as_str()) else {
            return false;
        };
        match definition.direction {
            ObjectiveDirection::Maximize => {
                if left_value < right_value {
                    return false;
                }
                strictly_better |= left_value > right_value;
            }
            ObjectiveDirection::Minimize => {
                if left_value > right_value {
                    return false;
                }
                strictly_better |= left_value < right_value;
            }
        }
    }
    strictly_better
}

pub fn audit_morphology_designs(
    problem: &MorphologyDesignProblem,
) -> Result<MorphologyDesignReport, MorphologyDesignError> {
    if !context_is_valid(&problem.context) {
        return Err(MorphologyDesignError::EmptyIdentifier);
    }
    if problem.metric_definitions.is_empty() {
        return Err(MorphologyDesignError::NoMetricDefinitions);
    }
    if problem.candidates.is_empty() {
        return Err(MorphologyDesignError::NoCandidates);
    }

    let mut definitions = BTreeMap::new();
    for definition in &problem.metric_definitions {
        if !nonempty(&definition.metric_id) || !nonempty(&definition.unit_id) {
            return Err(MorphologyDesignError::EmptyIdentifier);
        }
        if definitions
            .insert(definition.metric_id.as_str(), definition)
            .is_some()
        {
            return Err(MorphologyDesignError::DuplicateMetricDefinition);
        }
    }
    for constraint in &problem.hard_constraints {
        if !definitions.contains_key(constraint.metric_id.as_str()) {
            return Err(MorphologyDesignError::ConstraintReferencesUnknownMetric);
        }
    }
    let mut unique_constraints = BTreeSet::new();
    for constraint in &problem.hard_constraints {
        if !unique_constraints.insert((
            constraint.metric_id.as_str(),
            constraint.relation,
            constraint.threshold,
        )) {
            return Err(MorphologyDesignError::DuplicateHardConstraint);
        }
    }

    let mut candidate_ids = BTreeSet::new();
    let mut candidate_values = BTreeMap::<String, BTreeMap<&str, i64>>::new();
    let mut dispositions = Vec::with_capacity(problem.candidates.len());
    let mut categorical_design_axes = BTreeMap::new();

    for candidate in &problem.candidates {
        if !candidate_identity_is_valid(&candidate.identity) {
            return Err(MorphologyDesignError::EmptyIdentifier);
        }
        if !context_is_valid(&candidate.context) {
            return Err(MorphologyDesignError::EmptyIdentifier);
        }
        if candidate.context != problem.context {
            return Err(MorphologyDesignError::CandidateContextMismatch);
        }
        if !candidate_ids.insert(candidate.identity.candidate_id.as_str()) {
            return Err(MorphologyDesignError::DuplicateCandidateId);
        }

        let mut axes = BTreeMap::new();
        for (axis_id, value) in &candidate.categorical_design_axes {
            if !nonempty(axis_id) || !nonempty(value) {
                return Err(MorphologyDesignError::EmptyIdentifier);
            }
            if axes.insert(axis_id.clone(), value.clone()).is_some() {
                return Err(MorphologyDesignError::DuplicateCategoricalAxis);
            }
        }
        categorical_design_axes.insert(candidate.identity.candidate_id.clone(), axes);

        let values = validate_metric_values(candidate, &problem.metric_definitions)?;
        let constraint_failures = constraint_failures(&values, &problem.hard_constraints)?;
        let gate_failures = gate_failures(&candidate.gates);
        let eligible = gate_failures.is_empty() && constraint_failures.is_empty();
        if eligible {
            candidate_values.insert(candidate.identity.candidate_id.clone(), values);
        }
        dispositions.push(CandidateDisposition {
            candidate_id: candidate.identity.candidate_id.clone(),
            eligible_for_pareto: eligible,
            gate_failures,
            constraint_failures,
        });
    }

    let mut eligible_candidate_ids: Vec<String> = candidate_values.keys().cloned().collect();
    eligible_candidate_ids.sort();
    let mut pareto_candidate_ids = Vec::new();
    let mut dominance_relations = Vec::new();

    for candidate_id in &eligible_candidate_ids {
        let candidate_metrics = candidate_values
            .get(candidate_id)
            .ok_or(MorphologyDesignError::MissingMetricValue)?;
        let mut dominated = false;
        for other_id in &eligible_candidate_ids {
            if other_id == candidate_id {
                continue;
            }
            let other_metrics = candidate_values
                .get(other_id)
                .ok_or(MorphologyDesignError::MissingMetricValue)?;
            if dominates(other_metrics, candidate_metrics, &problem.metric_definitions) {
                dominated = true;
                dominance_relations.push(ParetoRelation {
                    dominating_candidate_id: other_id.clone(),
                    dominated_candidate_id: candidate_id.clone(),
                });
            }
        }
        if !dominated {
            pareto_candidate_ids.push(candidate_id.clone());
        }
    }

    dispositions.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    dominance_relations.sort_by(|left, right| {
        left.dominating_candidate_id
            .cmp(&right.dominating_candidate_id)
            .then_with(|| left.dominated_candidate_id.cmp(&right.dominated_candidate_id))
    });

    Ok(MorphologyDesignReport {
        context: problem.context.clone(),
        eligible_candidate_ids,
        pareto_candidate_ids,
        dispositions,
        dominance_relations,
        categorical_design_axes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> BenchmarkContext {
        BenchmarkContext {
            campaign_digest: "morph-campaign-v1".to_owned(),
            mission_digest: "mission-walk-payload-v1".to_owned(),
            fault_manifest_digest: "fault-manifest-v1".to_owned(),
            terrain_manifest_digest: "terrain-manifest-v1".to_owned(),
            metric_schema_digest: "metrics-v1".to_owned(),
            stratum_id: "fullsize-quadruped".to_owned(),
            evidence_tier: "hardware".to_owned(),
        }
    }

    fn definitions() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                metric_id: "nominal_success_ppm".to_owned(),
                unit_id: "ppm".to_owned(),
                direction: ObjectiveDirection::Maximize,
            },
            MetricDefinition {
                metric_id: "faulted_success_ppm".to_owned(),
                unit_id: "ppm".to_owned(),
                direction: ObjectiveDirection::Maximize,
            },
            MetricDefinition {
                metric_id: "nominal_energy_mj".to_owned(),
                unit_id: "mJ_per_task".to_owned(),
                direction: ObjectiveDirection::Minimize,
            },
            MetricDefinition {
                metric_id: "recovery_time_ms".to_owned(),
                unit_id: "ms".to_owned(),
                direction: ObjectiveDirection::Minimize,
            },
            MetricDefinition {
                metric_id: "mass_g".to_owned(),
                unit_id: "g".to_owned(),
                direction: ObjectiveDirection::Minimize,
            },
        ]
    }

    fn candidate(
        id: &str,
        values: [i64; 5],
        topology: &str,
        viability: EvidenceStatus,
    ) -> DesignCandidate {
        DesignCandidate {
            identity: DesignIdentity {
                candidate_id: id.to_owned(),
                design_family: "quadruped".to_owned(),
                morphology_digest: format!("morphology-{id}"),
                topology_id: topology.to_owned(),
                actuator_architecture_id: format!("actuator-{id}"),
                transmission_architecture_id: format!("transmission-{id}"),
                power_architecture_id: "isolated-drive-v1".to_owned(),
                sensing_architecture_id: "dual-encoder-current-v1".to_owned(),
            },
            context: context(),
            categorical_design_axes: BTreeMap::from([
                ("limb_topology".to_owned(), topology.to_owned()),
                ("transmission".to_owned(), format!("transmission-{id}")),
            ]),
            metrics: vec![
                MetricValue {
                    metric_id: "nominal_success_ppm".to_owned(),
                    value: values[0],
                },
                MetricValue {
                    metric_id: "faulted_success_ppm".to_owned(),
                    value: values[1],
                },
                MetricValue {
                    metric_id: "nominal_energy_mj".to_owned(),
                    value: values[2],
                },
                MetricValue {
                    metric_id: "recovery_time_ms".to_owned(),
                    value: values[3],
                },
                MetricValue {
                    metric_id: "mass_g".to_owned(),
                    value: values[4],
                },
            ],
            gates: GateEvidence {
                robust_viability: viability,
                recovery: EvidenceStatus::Valid,
                adverse_outcome_safety: EvidenceStatus::Valid,
                energy_and_thermal_reserve: EvidenceStatus::Valid,
                runtime_assurance: EvidenceStatus::Valid,
            },
        }
    }

    fn problem(candidates: Vec<DesignCandidate>) -> MorphologyDesignProblem {
        MorphologyDesignProblem {
            context: context(),
            metric_definitions: definitions(),
            hard_constraints: vec![
                HardConstraint {
                    metric_id: "mass_g".to_owned(),
                    relation: ConstraintRelation::LessThanOrEqual,
                    threshold: 20_000,
                },
                HardConstraint {
                    metric_id: "faulted_success_ppm".to_owned(),
                    relation: ConstraintRelation::GreaterThanOrEqual,
                    threshold: 500_000,
                },
            ],
            candidates,
        }
    }

    #[test]
    fn nominal_degraded_tradeoffs_remain_on_pareto_frontier() {
        let report = audit_morphology_designs(&problem(vec![
            candidate(
                "nominal-specialist",
                [990_000, 600_000, 4_000, 900, 12_000],
                "four-leg",
                EvidenceStatus::Valid,
            ),
            candidate(
                "fault-tolerant",
                [930_000, 900_000, 5_500, 500, 15_000],
                "four-leg-plus-support-arm",
                EvidenceStatus::Valid,
            ),
        ]))
        .expect("valid candidate matrix");
        assert_eq!(
            report.pareto_candidate_ids,
            vec!["fault-tolerant".to_owned(), "nominal-specialist".to_owned()]
        );
        assert!(report.dominance_relations.is_empty());
    }

    #[test]
    fn candidate_dominated_on_every_declared_metric_is_removed() {
        let report = audit_morphology_designs(&problem(vec![
            candidate(
                "best",
                [990_000, 900_000, 4_000, 400, 12_000],
                "four-leg",
                EvidenceStatus::Valid,
            ),
            candidate(
                "dominated",
                [900_000, 700_000, 5_000, 700, 15_000],
                "four-leg-with-passive-clutch",
                EvidenceStatus::Valid,
            ),
        ]))
        .expect("valid candidate matrix");
        assert_eq!(report.pareto_candidate_ids, vec!["best".to_owned()]);
        assert_eq!(
            report.dominance_relations,
            vec![ParetoRelation {
                dominating_candidate_id: "best".to_owned(),
                dominated_candidate_id: "dominated".to_owned(),
            }]
        );
    }

    #[test]
    fn superior_metrics_do_not_override_unknown_viability_gate() {
        let report = audit_morphology_designs(&problem(vec![
            candidate(
                "fast-but-unqualified",
                [1_000_000, 1_000_000, 1_000, 10, 8_000],
                "four-leg",
                EvidenceStatus::Unknown,
            ),
            candidate(
                "qualified",
                [900_000, 600_000, 4_000, 700, 12_000],
                "four-leg",
                EvidenceStatus::Valid,
            ),
        ]))
        .expect("candidate disposition report");
        assert_eq!(report.pareto_candidate_ids, vec!["qualified".to_owned()]);
        assert!(!report.dispositions[0].eligible_for_pareto);
        assert_eq!(
            report.dispositions[0].gate_failures,
            vec![GateFailure::RobustViability(EvidenceStatus::Unknown)]
        );
    }

    #[test]
    fn topology_is_preserved_as_categorical_metadata_not_interpolated() {
        let report = audit_morphology_designs(&problem(vec![
            candidate(
                "four-leg",
                [900_000, 700_000, 4_000, 600, 12_000],
                "four-leg",
                EvidenceStatus::Valid,
            ),
            candidate(
                "four-leg-arms",
                [900_000, 800_000, 5_000, 500, 15_000],
                "four-leg-plus-two-arms",
                EvidenceStatus::Valid,
            ),
        ]))
        .expect("categorical topology comparison");
        assert_eq!(
            report.categorical_design_axes["four-leg-arms"]["limb_topology"],
            "four-leg-plus-two-arms"
        );
        assert_eq!(report.pareto_candidate_ids.len(), 2);
    }

    #[test]
    fn out_of_context_candidate_is_rejected_not_pooled() {
        let mut candidate = candidate(
            "simulator-candidate",
            [900_000, 800_000, 4_000, 600, 12_000],
            "four-leg",
            EvidenceStatus::Valid,
        );
        candidate.context.evidence_tier = "simulation".to_owned();
        assert_eq!(
            audit_morphology_designs(&problem(vec![candidate])),
            Err(MorphologyDesignError::CandidateContextMismatch)
        );
    }

    #[test]
    fn missing_or_duplicate_metric_rows_are_rejected() {
        let mut missing = candidate(
            "missing",
            [900_000, 800_000, 4_000, 600, 12_000],
            "four-leg",
            EvidenceStatus::Valid,
        );
        missing.metrics.pop();
        assert_eq!(
            audit_morphology_designs(&problem(vec![missing])),
            Err(MorphologyDesignError::MissingMetricValue)
        );

        let mut duplicate = candidate(
            "duplicate",
            [900_000, 800_000, 4_000, 600, 12_000],
            "four-leg",
            EvidenceStatus::Valid,
        );
        duplicate.metrics.push(duplicate.metrics[0].clone());
        assert_eq!(
            audit_morphology_designs(&problem(vec![duplicate])),
            Err(MorphologyDesignError::DuplicateMetricValue)
        );
    }

    #[test]
    fn hard_resource_constraints_remove_candidate_before_pareto_ranking() {
        let report = audit_morphology_designs(&problem(vec![
            candidate(
                "overweight",
                [1_000_000, 950_000, 3_000, 300, 22_000],
                "four-leg-plus-arms",
                EvidenceStatus::Valid,
            ),
            candidate(
                "within-limit",
                [900_000, 800_000, 4_000, 500, 15_000],
                "four-leg",
                EvidenceStatus::Valid,
            ),
        ]))
        .expect("candidate disposition report");
        assert_eq!(report.pareto_candidate_ids, vec!["within-limit".to_owned()]);
        assert_eq!(report.dispositions[1].candidate_id, "overweight");
        assert_eq!(
            report.dispositions[1].constraint_failures[0].metric_id,
            "mass_g"
        );
    }
}

//! Deterministic counterexample search over a declared family of kernel mutations.
//!
//! This is a bounded model-falsification primitive, not an open-world proof.
//! Mutations preserve the finite kernel contract and are replayed against one
//! fixed public policy. A new observation branch that the policy cannot handle
//! is itself a concrete counterexample, rather than a branch to be silently
//! marginalized or discarded.

use super::robust_finite_horizon::{
    replay_shared_policy_across_worlds, DecisionModelWorld, WorldDecisionObjectives,
};
use super::finite_horizon::{DecisionBudgets, DecisionPlan};
use super::{DecisionLossTable, ObservationProtocol};
use crate::{ActionRealizationKernel, LatentState, Rational};
use std::collections::BTreeSet;
use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ModelWorldPerturbation {
    /// Swap two measurement symbols for one command and current sensor state.
    SwapMeasurementLabels {
        command: usize,
        sensor_state: usize,
        left_measurement: usize,
        right_measurement: usize,
    },
    /// Split readback-bearing outcomes into delivered and missing-readback paths.
    DropReadback {
        command: usize,
        sensor_state: Option<usize>,
        dropout_probability: Rational,
    },
    /// Split safe outcomes into safe and terminal-hazard paths.
    InjectHazard {
        command: usize,
        sensor_state: Option<usize>,
        hazard_probability: Rational,
    },
    /// Move exact probability mass between two outcomes in one complete row.
    ShiftOutcomeMass {
        state: LatentState,
        command: usize,
        source_outcome: usize,
        target_outcome: usize,
        amount: Rational,
    },
    /// Route subsequent outcomes from a selected apparatus state into a
    /// declared degraded apparatus state.
    DegradeApparatus {
        command: usize,
        current_apparatus_state_at_least: usize,
        degraded_next_state: usize,
    },
    /// Change the hidden realized-action class while preserving the vocabulary.
    RemapRealizedAction {
        command: usize,
        sensor_state: Option<usize>,
        from_action: usize,
        to_action: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PerturbationSpec {
    pub source_world_id: String,
    pub candidate_world_id: String,
    pub perturbation: ModelWorldPerturbation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CounterexampleClass {
    NoViolation,
    DecisionLossAmplified,
    AbstentionBudgetViolated,
    HazardBudgetViolated,
    PolicyInvalidated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterexampleOutcome {
    PolicyInvalidated {
        reason: String,
    },
    HazardBudgetViolated {
        metrics: WorldDecisionObjectives,
        allowed_hazard_probability: Rational,
    },
    AbstentionBudgetViolated {
        metrics: WorldDecisionObjectives,
        allowed_abstention_probability: Rational,
    },
    DecisionLossAmplified {
        metrics: WorldDecisionObjectives,
        baseline_worst_case_terminal_loss: Rational,
    },
    NoViolation {
        metrics: WorldDecisionObjectives,
    },
}

impl CounterexampleOutcome {
    pub fn class(&self) -> CounterexampleClass {
        match self {
            Self::PolicyInvalidated { .. } => CounterexampleClass::PolicyInvalidated,
            Self::HazardBudgetViolated { .. } => CounterexampleClass::HazardBudgetViolated,
            Self::AbstentionBudgetViolated { .. } => {
                CounterexampleClass::AbstentionBudgetViolated
            }
            Self::DecisionLossAmplified { .. } => CounterexampleClass::DecisionLossAmplified,
            Self::NoViolation { .. } => CounterexampleClass::NoViolation,
        }
    }

    fn metrics(&self) -> Option<&WorldDecisionObjectives> {
        match self {
            Self::PolicyInvalidated { .. } => None,
            Self::HazardBudgetViolated { metrics, .. }
            | Self::AbstentionBudgetViolated { metrics, .. }
            | Self::DecisionLossAmplified { metrics, .. }
            | Self::NoViolation { metrics } => Some(metrics),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleWitness {
    pub spec: PerturbationSpec,
    /// The validated, complete model world that falsifies or challenges the policy.
    pub candidate_world: DecisionModelWorld,
    pub outcome: CounterexampleOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectedPerturbation {
    pub spec: PerturbationSpec,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleSearchReport {
    /// Exact input snapshots needed to reproduce the falsification.
    pub baseline_worlds: Vec<DecisionModelWorld>,
    pub baseline_world_ids: Vec<String>,
    pub baseline_worst_case_total_loss: Rational,
    pub baseline_worst_case_terminal_loss: Rational,
    pub nominal_world_id: String,
    pub loss_table: DecisionLossTable,
    pub protocol: ObservationProtocol,
    pub command_costs: Vec<Rational>,
    pub hazard_loss: Rational,
    pub policy: DecisionPlan,
    pub horizon: usize,
    pub budgets: DecisionBudgets,
    pub attempted: usize,
    pub witnesses: Vec<CounterexampleWitness>,
    pub non_violating_candidates: Vec<CounterexampleWitness>,
    pub rejected: Vec<RejectedPerturbation>,
}

impl CounterexampleSearchReport {
    pub fn strongest_counterexample(&self) -> Option<&CounterexampleWitness> {
        self.witnesses.first()
    }
}

/// Apply one deterministic, exact perturbation and validate the resulting kernel.
///
/// Every mutation preserves the base world's state/action/observation
/// vocabularies so it can be admitted into the same robust family. The caller
/// supplies a unique candidate ID; it becomes immutable provenance for this
/// generated world.
pub fn apply_model_world_perturbation(
    source: &DecisionModelWorld,
    candidate_world_id: &str,
    perturbation: &ModelWorldPerturbation,
) -> Result<DecisionModelWorld, String> {
    if candidate_world_id.trim().is_empty() || candidate_world_id == source.id {
        return Err("candidate world ID must be non-empty and distinct from its source".into());
    }
    source.kernel.validate()?;
    let mut candidate = source.clone();
    candidate.id = candidate_world_id.to_string();

    match perturbation {
        ModelWorldPerturbation::SwapMeasurementLabels {
            command,
            sensor_state,
            left_measurement,
            right_measurement,
        } => {
            if *command >= candidate.kernel.command_ids.len()
                || *sensor_state >= candidate.kernel.sensor_state_count
                || *left_measurement >= candidate.kernel.measurement_ids.len()
                || *right_measurement >= candidate.kernel.measurement_ids.len()
                || left_measurement == right_measurement
            {
                return Err("measurement-swap parameters are outside the source vocabulary".into());
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if row.command != *command || row.state.sensor_state != *sensor_state {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    match outcome.measurement {
                        Some(value) if value == *left_measurement => {
                            outcome.measurement = Some(*right_measurement);
                            changed = true;
                        }
                        Some(value) if value == *right_measurement => {
                            outcome.measurement = Some(*left_measurement);
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            require_changed(changed, "measurement swap matched no outcomes")?;
        }
        ModelWorldPerturbation::DropReadback {
            command,
            sensor_state,
            dropout_probability,
        } => {
            validate_split_probability(*dropout_probability, "readback dropout")?;
            validate_scope(&candidate.kernel, *command, *sensor_state)?;
            let keep_probability = complement(*dropout_probability)?;
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if !row_matches(row.state, row.command, *command, *sensor_state) {
                    continue;
                }
                let mut expanded = Vec::with_capacity(row.outcomes.len().saturating_mul(2));
                for outcome in &row.outcomes {
                    if outcome.readback.is_some() {
                        let mut delivered = outcome.clone();
                        delivered.probability =
                            delivered.probability.checked_mul(keep_probability)?;
                        let mut missing = outcome.clone();
                        missing.probability =
                            missing.probability.checked_mul(*dropout_probability)?;
                        missing.readback = None;
                        expanded.push(delivered);
                        expanded.push(missing);
                        changed = true;
                    } else {
                        expanded.push(outcome.clone());
                    }
                }
                row.outcomes = expanded;
            }
            require_changed(changed, "readback dropout matched no readback-bearing outcomes")?;
        }
        ModelWorldPerturbation::InjectHazard {
            command,
            sensor_state,
            hazard_probability,
        } => {
            validate_split_probability(*hazard_probability, "hazard injection")?;
            validate_scope(&candidate.kernel, *command, *sensor_state)?;
            let safe_probability = complement(*hazard_probability)?;
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if !row_matches(row.state, row.command, *command, *sensor_state) {
                    continue;
                }
                let mut expanded = Vec::with_capacity(row.outcomes.len().saturating_mul(2));
                for outcome in &row.outcomes {
                    if !outcome.hazard {
                        let mut safe = outcome.clone();
                        safe.probability = safe.probability.checked_mul(safe_probability)?;
                        let mut hazard = outcome.clone();
                        hazard.probability =
                            hazard.probability.checked_mul(*hazard_probability)?;
                        hazard.hazard = true;
                        hazard.measurement = None;
                        expanded.push(safe);
                        expanded.push(hazard);
                        changed = true;
                    } else {
                        expanded.push(outcome.clone());
                    }
                }
                row.outcomes = expanded;
            }
            require_changed(changed, "hazard injection matched no safe outcomes")?;
        }
        ModelWorldPerturbation::ShiftOutcomeMass {
            state,
            command,
            source_outcome,
            target_outcome,
            amount,
        } => {
            if *command >= candidate.kernel.command_ids.len()
                || state.hypothesis >= candidate.kernel.hypothesis_ids.len()
                || state.physical_state >= candidate.kernel.physical_state_count
                || state.sensor_state >= candidate.kernel.sensor_state_count
                || state.apparatus_state >= candidate.kernel.apparatus_state_count
                || source_outcome == target_outcome
                || amount.is_zero()
            {
                return Err("outcome-mass shift parameters are invalid".into());
            }
            let row_index = candidate.kernel.row_index(*state, *command);
            let row = &mut candidate.kernel.rows[row_index];
            if *source_outcome >= row.outcomes.len() || *target_outcome >= row.outcomes.len() {
                return Err("outcome-mass shift index is outside the selected row".into());
            }
            let source_probability = row.outcomes[*source_outcome].probability;
            if *amount >= source_probability {
                return Err("outcome-mass shift must leave positive source probability".into());
            }
            row.outcomes[*source_outcome].probability =
                subtract_nonnegative(source_probability, *amount)?;
            row.outcomes[*target_outcome].probability =
                row.outcomes[*target_outcome].probability.checked_add(*amount)?;
        }
        ModelWorldPerturbation::DegradeApparatus {
            command,
            current_apparatus_state_at_least,
            degraded_next_state,
        } => {
            if *command >= candidate.kernel.command_ids.len()
                || *current_apparatus_state_at_least >= candidate.kernel.apparatus_state_count
                || *degraded_next_state >= candidate.kernel.apparatus_state_count
            {
                return Err("apparatus-degradation parameters are outside the state space".into());
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if row.command != *command
                    || row.state.apparatus_state < *current_apparatus_state_at_least
                {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    if outcome.next_state.apparatus_state != *degraded_next_state {
                        outcome.next_state.apparatus_state = *degraded_next_state;
                        changed = true;
                    }
                }
            }
            require_changed(changed, "apparatus degradation matched no next states")?;
        }
        ModelWorldPerturbation::RemapRealizedAction {
            command,
            sensor_state,
            from_action,
            to_action,
        } => {
            validate_scope(&candidate.kernel, *command, *sensor_state)?;
            if *from_action >= candidate.kernel.realized_action_ids.len()
                || *to_action >= candidate.kernel.realized_action_ids.len()
                || from_action == to_action
            {
                return Err("realized-action remap parameters are invalid".into());
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if !row_matches(row.state, row.command, *command, *sensor_state) {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    if outcome.realized_action == *from_action {
                        outcome.realized_action = *to_action;
                        changed = true;
                    }
                }
            }
            require_changed(changed, "realized-action remap matched no outcomes")?;
        }
    }

    candidate.kernel.validate()?;
    Ok(candidate)
}

fn require_changed(changed: bool, reason: &str) -> Result<(), String> {
    if changed {
        Ok(())
    } else {
        Err(reason.to_string())
    }
}

fn validate_scope(
    kernel: &ActionRealizationKernel,
    command: usize,
    sensor_state: Option<usize>,
) -> Result<(), String> {
    if command >= kernel.command_ids.len()
        || sensor_state.is_some_and(|state| state >= kernel.sensor_state_count)
    {
        return Err("perturbation command or sensor scope is outside the source model".into());
    }
    Ok(())
}

fn row_matches(
    state: LatentState,
    row_command: usize,
    command: usize,
    sensor_state: Option<usize>,
) -> bool {
    row_command == command && sensor_state.is_none_or(|sensor| state.sensor_state == sensor)
}

fn validate_split_probability(probability: Rational, label: &str) -> Result<(), String> {
    if probability.is_zero() || probability >= Rational::one() {
        return Err(format!("{label} probability must be strictly between zero and one"));
    }
    Ok(())
}

fn complement(probability: Rational) -> Result<Rational, String> {
    if !probability.is_probability() {
        return Err("probability complement requires a value in [0, 1]".into());
    }
    Rational::new(
        probability.denominator() - probability.numerator(),
        probability.denominator(),
    )
}

fn subtract_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("rational subtraction would be negative".into());
    }
    let common = gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_numerator = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let right_numerator = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
    Rational::new(left_numerator - right_numerator, denominator)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Search a fixed policy against a finite, deterministic perturbation family.
///
/// A policy that has no branch for a newly reachable public observation is
/// reported as invalidated. Valid counterexamples are kept with their exact
/// perturbation parameters and candidate kernel so a caller can explicitly
/// admit that immutable world into a subsequent planning run. The function
/// never silently modifies the input family or policy.
#[allow(clippy::too_many_arguments)]
pub fn search_policy_counterexamples(
    worlds: &[DecisionModelWorld],
    nominal_world_id: &str,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    policy: &DecisionPlan,
    horizon: usize,
    budgets: DecisionBudgets,
    specs: &[PerturbationSpec],
) -> Result<CounterexampleSearchReport, String> {
    budgets.validate()?;
    let baseline = replay_shared_policy_across_worlds(
        worlds,
        nominal_world_id,
        table,
        protocol,
        command_costs,
        hazard_loss,
        policy,
        horizon,
    )?;
    let baseline_worst_case_total_loss = baseline.worst_case_total_loss()?;
    let mut report = CounterexampleSearchReport {
        baseline_worlds: worlds.to_vec(),
        baseline_world_ids: baseline
            .per_world
            .iter()
            .map(|world| world.world_id.clone())
            .collect(),
        baseline_worst_case_total_loss,
        baseline_worst_case_terminal_loss: baseline
            .per_world
            .iter()
            .map(|world| world.expected_terminal_loss)
            .max()
            .unwrap_or_else(Rational::zero),
        nominal_world_id: nominal_world_id.to_string(),
        loss_table: table.clone(),
        protocol,
        command_costs: command_costs.to_vec(),
        hazard_loss,
        policy: policy.clone(),
        horizon,
        budgets,
        attempted: 0,
        witnesses: Vec::new(),
        non_violating_candidates: Vec::new(),
        rejected: Vec::new(),
    };

    let mut used_candidate_ids: BTreeSet<String> =
        worlds.iter().map(|world| world.id.clone()).collect();
    for spec in specs {
        report.attempted = report
            .attempted
            .checked_add(1)
            .ok_or_else(|| "counterexample attempt counter overflow".to_string())?;
        if spec.candidate_world_id.trim().is_empty()
            || !used_candidate_ids.insert(spec.candidate_world_id.clone())
        {
            report.rejected.push(RejectedPerturbation {
                spec: spec.clone(),
                reason: "candidate world ID is empty or already present in the family".into(),
            });
            continue;
        }
        let Some(source) = worlds
            .iter()
            .find(|world| world.id == spec.source_world_id)
        else {
            report.rejected.push(RejectedPerturbation {
                spec: spec.clone(),
                reason: "source world is absent from the supplied model family".into(),
            });
            continue;
        };
        let candidate_world = match apply_model_world_perturbation(
            source,
            &spec.candidate_world_id,
            &spec.perturbation,
        ) {
            Ok(candidate) => candidate,
            Err(reason) => {
                report.rejected.push(RejectedPerturbation {
                    spec: spec.clone(),
                    reason,
                });
                continue;
            }
        };

        let mut expanded_worlds = worlds.to_vec();
        expanded_worlds.push(candidate_world.clone());
        let outcome = match replay_shared_policy_across_worlds(
            &expanded_worlds,
            nominal_world_id,
            table,
            protocol,
            command_costs,
            hazard_loss,
            policy,
            horizon,
        ) {
            Err(reason) => CounterexampleOutcome::PolicyInvalidated { reason },
            Ok(replay) => {
                let Some(metrics) = replay
                    .per_world
                    .iter()
                    .find(|metrics| metrics.world_id == candidate_world.id)
                    .cloned()
                else {
                    report.rejected.push(RejectedPerturbation {
                        spec: spec.clone(),
                        reason: "replay receipt omitted the admitted candidate world".into(),
                    });
                    continue;
                };

                if metrics.hazard_probability > budgets.max_hazard_probability {
                    CounterexampleOutcome::HazardBudgetViolated {
                        metrics,
                        allowed_hazard_probability: budgets.max_hazard_probability,
                    }
                } else if metrics.abstention_probability
                    > budgets.max_abstention_probability
                {
                    CounterexampleOutcome::AbstentionBudgetViolated {
                        metrics,
                        allowed_abstention_probability: budgets.max_abstention_probability,
                    }
                } else if metrics.expected_terminal_loss
                    > report.baseline_worst_case_terminal_loss
                {
                    CounterexampleOutcome::DecisionLossAmplified {
                        metrics,
                        baseline_worst_case_terminal_loss: report
                            .baseline_worst_case_terminal_loss,
                    }
                } else {
                    CounterexampleOutcome::NoViolation { metrics }
                }
            }
        };

        let witness = CounterexampleWitness {
            spec: spec.clone(),
            candidate_world,
            outcome,
        };
        if witness.outcome.class() == CounterexampleClass::NoViolation {
            report.non_violating_candidates.push(witness);
        } else {
            report.witnesses.push(witness);
        }
    }

    report.witnesses.sort_by(compare_witness_severity);
    report.non_violating_candidates.sort_by(|left, right| {
        left.spec.candidate_world_id.cmp(&right.spec.candidate_world_id)
    });
    report.rejected.sort_by(|left, right| {
        left.spec.candidate_world_id.cmp(&right.spec.candidate_world_id)
    });
    Ok(report)
}

fn compare_witness_severity(
    left: &CounterexampleWitness,
    right: &CounterexampleWitness,
) -> Ordering {
    right
        .outcome
        .class()
        .cmp(&left.outcome.class())
        .then_with(|| {
            let left_metrics = left.outcome.metrics();
            let right_metrics = right.outcome.metrics();
            match (left_metrics, right_metrics) {
                (Some(a), Some(b)) => {
                    let loss_order = match (
                        a.expected_total_loss(),
                        b.expected_total_loss(),
                    ) {
                        (Ok(left_loss), Ok(right_loss)) => right_loss.cmp(&left_loss),
                        (Err(_), Ok(_)) => Ordering::Less,
                        (Ok(_), Err(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => Ordering::Equal,
                    };
                    loss_order.then_with(|| b.hazard_probability.cmp(&a.hazard_probability))
                }
                _ => Ordering::Equal,
            }
        })
        .then_with(|| {
            left.spec
                .candidate_world_id
                .cmp(&right.spec.candidate_world_id)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BeliefMass, KernelRow, LatentState, Rational, TransitionOutcome};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn world() -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(hypothesis),
                    hazard: false,
                }],
            });
            rows.push(KernelRow {
                state,
                command: 1,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(hypothesis),
                    hazard: false,
                }],
            });
        }
        DecisionModelWorld {
            id: "nominal".into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["read-0".into(), "read-1".into()],
                realized_action_ids: vec!["executed".into(), "dropped".into()],
                readback_ids: vec!["ok".into()],
                measurement_ids: vec!["zero".into(), "one".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 0,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 1,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn world_with_two_apparatus_states() -> DecisionModelWorld {
        let mut source = world();
        source.kernel.apparatus_state_count = 2;
        source.kernel.rows.clear();
        for hypothesis in 0..2 {
            for apparatus_state in 0..2 {
                let state = LatentState {
                    hypothesis,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state,
                };
                for command in 0..2 {
                    source.kernel.rows.push(KernelRow {
                        state,
                        command,
                        outcomes: vec![TransitionOutcome {
                            probability: Rational::one(),
                            next_state: state,
                            realized_action: 0,
                            readback: Some(0),
                            measurement: Some(hypothesis),
                            hazard: false,
                        }],
                    });
                }
            }
        }
        source.kernel.validate().unwrap();
        source
    }

    fn table() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["claim-h0".into(), "claim-h1".into()],
            losses: vec![
                vec![
                    vec![vec![vec![Rational::zero()]]],
                    vec![vec![vec![Rational::one()]]],
                ],
                vec![
                    vec![vec![vec![Rational::one()]]],
                    vec![vec![vec![Rational::zero()]]],
                ],
            ],
            abstention_loss: Rational::one(),
        }
    }

    fn policy() -> DecisionPlan {
        let first = crate::PublicObservation {
            hazard: false,
            readback: crate::ObservationField::Value(0),
            measurement: crate::ObservationField::Value(0),
        };
        let second = crate::PublicObservation {
            measurement: crate::ObservationField::Value(1),
            ..first
        };
        DecisionPlan::Experiment {
            command: 0,
            branches: vec![
                super::super::finite_horizon::DecisionBranch {
                    observation: first,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                super::super::finite_horizon::DecisionBranch {
                    observation: second,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
            ],
        }
    }

    fn spec(id: &str, perturbation: ModelWorldPerturbation) -> PerturbationSpec {
        PerturbationSpec {
            source_world_id: "nominal".into(),
            candidate_world_id: id.into(),
            perturbation,
        }
    }

    fn base_inputs() -> (Vec<DecisionModelWorld>, DecisionLossTable, Vec<Rational>) {
        (
            vec![world()],
            table(),
            vec![Rational::new(1, 10).unwrap(), Rational::new(1, 10).unwrap()],
        )
    }

    #[test]
    fn measurement_inversion_is_a_decision_loss_counterexample() {
        let (worlds, table, costs) = base_inputs();
        let perturbation = spec(
            "sensor-inverted",
            ModelWorldPerturbation::SwapMeasurementLabels {
                command: 0,
                sensor_state: 0,
                left_measurement: 0,
                right_measurement: 1,
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[perturbation],
        )
        .unwrap();
        let counterexample = report.strongest_counterexample().unwrap();
        assert_eq!(
            counterexample.outcome.class(),
            CounterexampleClass::DecisionLossAmplified
        );
        match &counterexample.outcome {
            CounterexampleOutcome::DecisionLossAmplified { metrics, .. } => {
                assert_eq!(metrics.expected_terminal_loss, Rational::one());
            }
            _ => panic!("measurement inversion should amplify terminal decision loss"),
        }
        assert_eq!(report.baseline_worst_case_total_loss, Rational::new(1, 10).unwrap());
        assert_eq!(report.attempted, 1);
        assert_eq!(report.witnesses.len(), 1);
    }

    #[test]
    fn newly_reachable_missing_readback_invalidates_the_fixed_policy() {
        let (worlds, table, costs) = base_inputs();
        let perturbation = spec(
            "readback-dropped",
            ModelWorldPerturbation::DropReadback {
                command: 0,
                sensor_state: None,
                dropout_probability: Rational::new(1, 10).unwrap(),
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[perturbation],
        )
        .unwrap();
        assert_eq!(
            report.strongest_counterexample().unwrap().outcome.class(),
            CounterexampleClass::PolicyInvalidated
        );
    }

    #[test]
    fn hazard_injection_invalidates_policy_when_it_lacks_hazard_fallback() {
        let (worlds, table, costs) = base_inputs();
        let perturbation = spec(
            "hazard-shifted",
            ModelWorldPerturbation::InjectHazard {
                command: 0,
                sensor_state: None,
                hazard_probability: Rational::new(1, 20).unwrap(),
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::one(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[perturbation],
        )
        .unwrap();
        assert_eq!(
            report.strongest_counterexample().unwrap().outcome.class(),
            CounterexampleClass::PolicyInvalidated
        );
    }

    #[test]
    fn outcome_mass_shift_preserves_exact_row_normalization() {
        let source = world();
        let candidate = apply_model_world_perturbation(
            &source,
            "shifted",
            &ModelWorldPerturbation::ShiftOutcomeMass {
                state: source.kernel.initial_belief[0].state,
                command: 0,
                source_outcome: 0,
                target_outcome: 0,
                amount: Rational::new(1, 10).unwrap(),
            },
        );
        assert!(candidate.is_err());

        let mut split = source.clone();
        split.kernel.rows[0].outcomes.push(TransitionOutcome {
            probability: Rational::new(1, 5).unwrap(),
            next_state: split.kernel.rows[0].outcomes[0].next_state,
            realized_action: 0,
            readback: Some(0),
            measurement: Some(1),
            hazard: false,
        });
        split.kernel.rows[0].outcomes[0].probability = Rational::new(4, 5).unwrap();
        let candidate = apply_model_world_perturbation(
            &split,
            "shifted-two-outcome",
            &ModelWorldPerturbation::ShiftOutcomeMass {
                state: split.kernel.initial_belief[0].state,
                command: 0,
                source_outcome: 0,
                target_outcome: 1,
                amount: Rational::new(1, 10).unwrap(),
            },
        )
        .unwrap();
        candidate.kernel.validate().unwrap();
        assert_eq!(candidate.kernel.rows[0].outcomes[0].probability, Rational::new(7, 10).unwrap());
        assert_eq!(candidate.kernel.rows[0].outcomes[1].probability, Rational::new(3, 10).unwrap());
    }

    #[test]
    fn readback_dropout_and_hazard_mutations_preserve_kernel_mass() {
        let source = world();
        let dropout = apply_model_world_perturbation(
            &source,
            "dropout",
            &ModelWorldPerturbation::DropReadback {
                command: 0,
                sensor_state: None,
                dropout_probability: Rational::new(1, 10).unwrap(),
            },
        )
        .unwrap();
        dropout.kernel.validate().unwrap();
        assert_eq!(dropout.kernel.rows[0].outcomes.len(), 2);
        assert_eq!(
            dropout.kernel.rows[0].outcomes[0].probability
                .checked_add(dropout.kernel.rows[0].outcomes[1].probability)
                .unwrap(),
            Rational::one()
        );

        let hazard = apply_model_world_perturbation(
            &source,
            "hazard",
            &ModelWorldPerturbation::InjectHazard {
                command: 0,
                sensor_state: None,
                hazard_probability: Rational::new(1, 20).unwrap(),
            },
        )
        .unwrap();
        hazard.kernel.validate().unwrap();
        assert_eq!(hazard.kernel.rows[0].outcomes.len(), 2);
        assert_eq!(
            hazard.kernel.rows[0].outcomes.iter().filter(|outcome| outcome.hazard).count(),
            1
        );
    }

    #[test]
    fn apparatus_degradation_and_hidden_action_remap_are_exact() {
        let source = world_with_two_apparatus_states();
        let degraded = apply_model_world_perturbation(
            &source,
            "degraded",
            &ModelWorldPerturbation::DegradeApparatus {
                command: 0,
                current_apparatus_state_at_least: 0,
                degraded_next_state: 1,
            },
        )
        .unwrap();
        degraded.kernel.validate().unwrap();
        assert!(degraded
            .kernel
            .rows
            .iter()
            .filter(|row| row.command == 0 && row.state.apparatus_state == 0)
            .all(|row| row.outcomes.iter().all(|outcome| outcome.next_state.apparatus_state == 1)));

        let remapped = apply_model_world_perturbation(
            &source,
            "action-remapped",
            &ModelWorldPerturbation::RemapRealizedAction {
                command: 0,
                sensor_state: None,
                from_action: 0,
                to_action: 1,
            },
        )
        .unwrap();
        remapped.kernel.validate().unwrap();
        assert!(remapped
            .kernel
            .rows
            .iter()
            .filter(|row| row.command == 0)
            .all(|row| row.outcomes.iter().all(|outcome| outcome.realized_action == 1)));
    }

    #[test]
    fn non_violating_hidden_action_change_is_retained_as_a_control() {
        let (worlds, table, costs) = base_inputs();
        let action_remap = spec(
            "hidden-action-remap",
            ModelWorldPerturbation::RemapRealizedAction {
                command: 0,
                sensor_state: None,
                from_action: 0,
                to_action: 1,
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[action_remap],
        )
        .unwrap();
        assert_eq!(report.witnesses.len(), 0);
        assert_eq!(report.non_violating_candidates.len(), 1);
        assert_eq!(
            report.non_violating_candidates[0].outcome.class(),
            CounterexampleClass::NoViolation
        );
        assert_eq!(report.baseline_worlds, worlds);
        assert_eq!(report.loss_table, table);
        assert_eq!(report.policy, policy());
        assert_eq!(report.command_costs, costs);
    }

    #[test]
    fn repeated_candidate_id_is_rejected_within_one_search_batch() {
        let (worlds, table, costs) = base_inputs();
        let specs = [
            spec(
                "same-candidate",
                ModelWorldPerturbation::SwapMeasurementLabels {
                    command: 0,
                    sensor_state: 0,
                    left_measurement: 0,
                    right_measurement: 1,
                },
            ),
            spec(
                "same-candidate",
                ModelWorldPerturbation::InjectHazard {
                    command: 0,
                    sensor_state: None,
                    hazard_probability: Rational::new(1, 10).unwrap(),
                },
            ),
        ];
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::one(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &specs,
        )
        .unwrap();
        assert_eq!(report.attempted, 2);
        assert_eq!(report.witnesses.len(), 1);
        assert_eq!(report.rejected.len(), 1);
    }

    #[test]
    fn generator_rejects_invalid_probability_and_missing_source() {
        let source = world();
        assert!(apply_model_world_perturbation(
            &source,
            "bad-probability",
            &ModelWorldPerturbation::InjectHazard {
                command: 0,
                sensor_state: None,
                hazard_probability: Rational::one(),
            },
        )
        .is_err());
        assert!(apply_model_world_perturbation(
            &source,
            "bad-measurement",
            &ModelWorldPerturbation::SwapMeasurementLabels {
                command: 0,
                sensor_state: 0,
                left_measurement: 0,
                right_measurement: 9,
            },
        )
        .is_err());
    }

    #[test]
    fn search_rejects_duplicate_candidate_ids_and_records_rejections() {
        let (worlds, table, costs) = base_inputs();
        let duplicate = spec(
            "nominal",
            ModelWorldPerturbation::SwapMeasurementLabels {
                command: 0,
                sensor_state: 0,
                left_measurement: 0,
                right_measurement: 1,
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[duplicate],
        )
        .unwrap();
        assert_eq!(report.attempted, 1);
        assert_eq!(report.rejected.len(), 1);
        assert!(report.witnesses.is_empty());
    }
}

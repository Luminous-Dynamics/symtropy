// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Causal-path conformance for matched socio-ecological interventions.
//!
//! This crate does not replace the resource ledger or estimate real-world
//! treatment effects. It checks whether a predeclared model mechanism path is
//! present in paired runs, whether exogenous inputs are held fixed, and whether
//! the claimed outcome is independently measured by the shared resource ledger.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub mod paired_effects;
pub mod temporal_graph;

const VALUE_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CausalStage {
    Intervention,
    Mechanism,
    Intermediate,
    Outcome,
    NegativeControl,
}

impl CausalStage {
    fn rank(self) -> Option<u8> {
        match self {
            Self::Intervention => Some(0),
            Self::Mechanism => Some(1),
            Self::Intermediate => Some(2),
            Self::Outcome => Some(3),
            Self::NegativeControl => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalVariable {
    pub id: String,
    pub stage: CausalStage,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalEdge {
    pub id: String,
    pub from_variable: String,
    pub to_variable: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionReceipt {
    pub edge_id: String,
    pub source_value: f64,
    pub target_before: f64,
    pub target_after: f64,
    /// True only when the model actually applied the declared transition.
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndependentOutcomeReceipt {
    pub variable_id: String,
    pub value: f64,
    pub unit: String,
    pub resource: String,
    pub region_id: String,
    pub tick: u64,
    /// Deterministic row key for the ledger outcome in this run artifact; not a ledger-issued signature.
    pub source_record_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunArm {
    pub arm_id: String,
    /// Opaque identifier for the full exogenous state/configuration snapshot.
    /// Equality is a protocol assertion; this crate does not hash or verify it.
    pub exogenous_snapshot_id: String,
    pub initial_values: BTreeMap<String, f64>,
    pub final_values: BTreeMap<String, f64>,
    pub transitions: Vec<TransitionReceipt>,
    pub independent_outcome: IndependentOutcomeReceipt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MechanismAblation {
    /// Exogenous switch used only in the ablation arm.
    pub block_variable_id: String,
    /// Mediator/intermediate expected to return to its control value when blocked.
    pub blocked_mechanism_variable_id: String,
    pub arm: RunArm,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CausalExperiment {
    pub experiment_id: String,
    pub intervention_variable_id: String,
    pub negative_control_variable_id: String,
    pub outcome_variable_id: String,
    /// Ordered contiguous edge IDs from intervention through measured outcome.
    pub required_path_edge_ids: Vec<String>,
    pub variables: Vec<CausalVariable>,
    pub edges: Vec<CausalEdge>,
    pub control: RunArm,
    pub treatment: RunArm,
    /// A sham arm changes only an internal negative-control variable.
    pub negative_control: RunArm,
    /// A fourth arm keeps the primary intervention enabled but blocks a declared mediator.
    pub mechanism_ablation: MechanismAblation,
}

impl CausalExperiment {
    pub fn validate(&self) -> Result<(), CausalPathError> {
        if self.experiment_id.trim().is_empty() {
            return Err(CausalPathError::EmptyExperimentId);
        }
        if self.required_path_edge_ids.is_empty() {
            return Err(CausalPathError::EmptyRequiredPath);
        }
        let variable_map = self.variable_map()?;
        let edge_map = self.edge_map(&variable_map)?;
        self.validate_declared_path(&variable_map, &edge_map)?;
        self.validate_acyclic(&variable_map)?;

        if self.intervention_variable_id == self.negative_control_variable_id
            || self.intervention_variable_id == self.outcome_variable_id
            || self.negative_control_variable_id == self.outcome_variable_id
        {
            return Err(CausalPathError::VariableRolesOverlap);
        }

        let intervention = variable_map
            .get(self.intervention_variable_id.as_str())
            .ok_or_else(|| {
                CausalPathError::UnknownVariable(self.intervention_variable_id.clone())
            })?;
        let outcome = variable_map
            .get(self.outcome_variable_id.as_str())
            .ok_or_else(|| CausalPathError::UnknownVariable(self.outcome_variable_id.clone()))?;
        let negative = variable_map
            .get(self.negative_control_variable_id.as_str())
            .ok_or_else(|| {
                CausalPathError::UnknownVariable(self.negative_control_variable_id.clone())
            })?;

        if intervention.stage != CausalStage::Intervention {
            return Err(CausalPathError::WrongVariableStage {
                variable_id: intervention.id.clone(),
                expected: CausalStage::Intervention,
                actual: intervention.stage,
            });
        }
        if outcome.stage != CausalStage::Outcome {
            return Err(CausalPathError::WrongVariableStage {
                variable_id: outcome.id.clone(),
                expected: CausalStage::Outcome,
                actual: outcome.stage,
            });
        }
        if negative.stage != CausalStage::NegativeControl {
            return Err(CausalPathError::WrongVariableStage {
                variable_id: negative.id.clone(),
                expected: CausalStage::NegativeControl,
                actual: negative.stage,
            });
        }
        if self.required_path_edge_ids.iter().any(|edge_id| {
            edge_map[edge_id.as_str()].to_variable == self.negative_control_variable_id
                || edge_map[edge_id.as_str()].from_variable == self.negative_control_variable_id
        }) {
            return Err(CausalPathError::NegativeControlOnRequiredPath);
        }

        self.validate_run(&self.control, &variable_map, &edge_map)?;
        self.validate_run(&self.treatment, &variable_map, &edge_map)?;
        self.validate_run(&self.negative_control, &variable_map, &edge_map)?;
        self.validate_run(&self.mechanism_ablation.arm, &variable_map, &edge_map)?;
        self.validate_paired_inputs(&variable_map)?;
        self.validate_negative_control(&variable_map)?;
        self.validate_mechanism_ablation(&variable_map)?;
        Ok(())
    }

    pub fn evaluate(&self) -> Result<CausalPathResult, CausalPathError> {
        self.validate()?;
        let control_value = self.control.independent_outcome.value;
        let treatment_value = self.treatment.independent_outcome.value;
        let negative_value = self.negative_control.independent_outcome.value;
        let ablation_value = self.mechanism_ablation.arm.independent_outcome.value;
        let observed_effect = treatment_value - control_value;
        let negative_control_effect = negative_value - control_value;
        let mechanism_ablation_effect = ablation_value - control_value;

        let variable_map = self.variable_map()?;
        let edge_map = self.edge_map(&variable_map)?;
        let mut path_edges = Vec::with_capacity(self.required_path_edge_ids.len());
        let mut first_broken_edge = None;

        for edge_id in &self.required_path_edge_ids {
            let edge = edge_map[edge_id.as_str()];
            let treatment_receipt = self
                .treatment
                .transitions
                .iter()
                .find(|receipt| receipt.edge_id == *edge_id);
            let control_receipt = self
                .control
                .transitions
                .iter()
                .find(|receipt| receipt.edge_id == *edge_id);
            let receipt = treatment_receipt.expect("validate_run guarantees every edge receipt");
            let control_source_value = control_receipt
                .expect("validate_run guarantees every control edge receipt")
                .source_value;
            let source_changed = !close(receipt.source_value, control_source_value);
            let complete = source_changed
                && receipt.applied
                && !close(receipt.target_before, receipt.target_after)
                && control_receipt.is_some_and(|control| {
                    !control.applied && close(control.target_before, control.target_after)
                });

            if !complete && first_broken_edge.is_none() {
                first_broken_edge = Some(edge_id.clone());
            }
            path_edges.push(PathEdgeResult {
                edge_id: edge_id.clone(),
                from_variable: edge.from_variable.clone(),
                to_variable: edge.to_variable.clone(),
                treatment_source_value: receipt.source_value,
                control_source_value,
                source_changed,
                treatment_target_delta: receipt.target_after - receipt.target_before,
                treatment_transition_applied: receipt.applied,
                control_transition_applied: control_receipt.is_some_and(|item| item.applied),
                complete,
            });
        }

        let mechanism_path_complete = first_broken_edge.is_none();
        let negative_control_isolated = close(negative_control_effect, 0.0)
            && self.negative_control.final_values[&self.negative_control_variable_id]
                != self.control.final_values[&self.negative_control_variable_id];
        let exogenous_inputs_matched = self.control.exogenous_snapshot_id
            == self.treatment.exogenous_snapshot_id
            && self.control.exogenous_snapshot_id == self.negative_control.exogenous_snapshot_id;
        let single_intervention_difference = only_one_initial_value_differs(
            &self.control,
            &self.treatment,
            &self.intervention_variable_id,
        );
        let independent_outcome_agrees = self.treatment.independent_outcome.variable_id
            == self.outcome_variable_id
            && self.control.independent_outcome.variable_id == self.outcome_variable_id
            && self.negative_control.independent_outcome.variable_id == self.outcome_variable_id
            && close(
                self.treatment.final_values[&self.outcome_variable_id],
                treatment_value,
            )
            && close(
                self.control.final_values[&self.outcome_variable_id],
                control_value,
            )
            && close(
                self.negative_control.final_values[&self.outcome_variable_id],
                negative_value,
            )
            && self.treatment.independent_outcome.unit
                == variable_map[self.outcome_variable_id.as_str()].unit
            && self.control.independent_outcome.unit
                == variable_map[self.outcome_variable_id.as_str()].unit
            && self.negative_control.independent_outcome.unit
                == variable_map[self.outcome_variable_id.as_str()].unit;

        let ablation_exogenous_matched =
            self.mechanism_ablation.arm.exogenous_snapshot_id == self.control.exogenous_snapshot_id;
        let ablation_inputs_matched = only_one_initial_value_differs(
            &self.treatment,
            &self.mechanism_ablation.arm,
            &self.mechanism_ablation.block_variable_id,
        );
        let primary_intervention_preserved = close(
            self.treatment.initial_values[&self.intervention_variable_id],
            self.mechanism_ablation.arm.initial_values[&self.intervention_variable_id],
        );
        let blocked_mechanism_suppressed = close(
            self.mechanism_ablation.arm.final_values
                [&self.mechanism_ablation.blocked_mechanism_variable_id],
            self.control.final_values[&self.mechanism_ablation.blocked_mechanism_variable_id],
        ) && !close(
            self.mechanism_ablation.arm.final_values
                [&self.mechanism_ablation.blocked_mechanism_variable_id],
            self.treatment.final_values[&self.mechanism_ablation.blocked_mechanism_variable_id],
        );
        let ablation_outcome_agrees =
            close(
                self.mechanism_ablation.arm.final_values[&self.outcome_variable_id],
                ablation_value,
            ) && self.mechanism_ablation.arm.independent_outcome.variable_id
                == self.outcome_variable_id
                && self.mechanism_ablation.arm.independent_outcome.unit
                    == variable_map[self.outcome_variable_id.as_str()].unit;
        let ablation_protocol_valid = ablation_exogenous_matched
            && ablation_inputs_matched
            && primary_intervention_preserved
            && blocked_mechanism_suppressed
            && ablation_outcome_agrees;
        let qualified = mechanism_path_complete
            && negative_control_isolated
            && exogenous_inputs_matched
            && single_intervention_difference
            && independent_outcome_agrees;
        let mechanism_necessity_supported = qualified
            && ablation_protocol_valid
            && !close(observed_effect, 0.0)
            && close(ablation_value, control_value);

        Ok(CausalPathResult {
            experiment_id: self.experiment_id.clone(),
            control_outcome: control_value,
            treatment_outcome: treatment_value,
            observed_treatment_effect: observed_effect,
            negative_control_outcome: negative_value,
            negative_control_effect,
            mechanism_ablation_outcome: ablation_value,
            mechanism_ablation_effect,
            ablation_exogenous_matched,
            ablation_inputs_matched,
            primary_intervention_preserved,
            blocked_mechanism_suppressed,
            ablation_outcome_agrees,
            ablation_protocol_valid,
            mechanism_necessity_supported,
            mechanism_path_complete,
            first_broken_edge,
            negative_control_isolated,
            exogenous_inputs_matched,
            single_intervention_difference,
            independent_outcome_agrees,
            qualified,
            qualified_treatment_effect: qualified.then_some(observed_effect),
            path_edges,
            claim_ceiling: "paired deterministic model contrast with path conformance; not empirical causal identification".into(),
        })
    }

    fn variable_map(&self) -> Result<BTreeMap<&str, &CausalVariable>, CausalPathError> {
        let mut map = BTreeMap::new();
        for variable in &self.variables {
            if variable.id.trim().is_empty() || variable.unit.trim().is_empty() {
                return Err(CausalPathError::InvalidVariable);
            }
            if map.insert(variable.id.as_str(), variable).is_some() {
                return Err(CausalPathError::DuplicateVariable(variable.id.clone()));
            }
        }
        Ok(map)
    }

    fn edge_map<'a>(
        &'a self,
        variables: &BTreeMap<&str, &CausalVariable>,
    ) -> Result<BTreeMap<&'a str, &'a CausalEdge>, CausalPathError> {
        let mut map = BTreeMap::new();
        for edge in &self.edges {
            if edge.id.trim().is_empty() {
                return Err(CausalPathError::InvalidEdge);
            }
            if !variables.contains_key(edge.from_variable.as_str()) {
                return Err(CausalPathError::UnknownVariable(edge.from_variable.clone()));
            }
            if !variables.contains_key(edge.to_variable.as_str()) {
                return Err(CausalPathError::UnknownVariable(edge.to_variable.clone()));
            }
            if edge.from_variable == edge.to_variable {
                return Err(CausalPathError::SelfEdge(edge.id.clone()));
            }
            if map.insert(edge.id.as_str(), edge).is_some() {
                return Err(CausalPathError::DuplicateEdge(edge.id.clone()));
            }
        }
        Ok(map)
    }

    fn validate_declared_path(
        &self,
        variables: &BTreeMap<&str, &CausalVariable>,
        edges: &BTreeMap<&str, &CausalEdge>,
    ) -> Result<(), CausalPathError> {
        let mut previous_target: Option<&str> = None;
        let mut previous_stage = None;
        for edge_id in &self.required_path_edge_ids {
            let edge = edges
                .get(edge_id.as_str())
                .ok_or_else(|| CausalPathError::UnknownEdge(edge_id.clone()))?;
            if let Some(previous) = previous_target
                && edge.from_variable != previous
            {
                return Err(CausalPathError::DiscontinuousPath(edge_id.clone()));
            }
            let from = variables[edge.from_variable.as_str()];
            let to = variables[edge.to_variable.as_str()];
            let from_rank =
                from.stage
                    .rank()
                    .ok_or_else(|| CausalPathError::WrongVariableStage {
                        variable_id: from.id.clone(),
                        expected: CausalStage::Intervention,
                        actual: from.stage,
                    })?;
            let to_rank = to
                .stage
                .rank()
                .ok_or_else(|| CausalPathError::WrongVariableStage {
                    variable_id: to.id.clone(),
                    expected: CausalStage::Outcome,
                    actual: to.stage,
                })?;
            if to_rank < from_rank || previous_stage.is_some_and(|rank| from_rank < rank) {
                return Err(CausalPathError::InvalidStageOrder(edge_id.clone()));
            }
            previous_stage = Some(to_rank);
            previous_target = Some(edge.to_variable.as_str());
        }
        let first = edges[self.required_path_edge_ids[0].as_str()];
        let last =
            edges[self.required_path_edge_ids[self.required_path_edge_ids.len() - 1].as_str()];
        if first.from_variable != self.intervention_variable_id
            || last.to_variable != self.outcome_variable_id
        {
            return Err(CausalPathError::PathEndpointsMismatch);
        }
        Ok(())
    }

    fn validate_acyclic(
        &self,
        variables: &BTreeMap<&str, &CausalVariable>,
    ) -> Result<(), CausalPathError> {
        let mut indegree = variables
            .keys()
            .map(|id| (*id, 0usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &self.edges {
            *indegree
                .get_mut(edge.to_variable.as_str())
                .expect("edge endpoints validated") += 1;
            outgoing
                .entry(edge.from_variable.as_str())
                .or_default()
                .push(edge.to_variable.as_str());
        }
        let mut queue = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| *id)
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            for next in outgoing.get(node).into_iter().flatten() {
                let degree = indegree.get_mut(next).expect("edge endpoint exists");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(next);
                }
            }
        }
        if visited != variables.len() {
            return Err(CausalPathError::CyclicGraph);
        }
        Ok(())
    }

    fn validate_run(
        &self,
        run: &RunArm,
        variables: &BTreeMap<&str, &CausalVariable>,
        edges: &BTreeMap<&str, &CausalEdge>,
    ) -> Result<(), CausalPathError> {
        if run.arm_id.trim().is_empty() || run.exogenous_snapshot_id.trim().is_empty() {
            return Err(CausalPathError::InvalidRunIdentity);
        }
        if run.initial_values.len() != variables.len()
            || run.final_values.len() != variables.len()
            || variables.keys().any(|id| {
                !run.initial_values.contains_key(*id) || !run.final_values.contains_key(*id)
            })
        {
            return Err(CausalPathError::RunStateSchemaMismatch(run.arm_id.clone()));
        }
        for (variable_id, value) in run.initial_values.iter().chain(run.final_values.iter()) {
            if !value.is_finite() {
                return Err(CausalPathError::NonFiniteState {
                    run_id: run.arm_id.clone(),
                    variable_id: variable_id.clone(),
                    value: *value,
                });
            }
        }
        if run.transitions.len() != edges.len() {
            return Err(CausalPathError::TransitionReceiptCountMismatch {
                run_id: run.arm_id.clone(),
                expected: edges.len(),
                actual: run.transitions.len(),
            });
        }
        let mut seen = BTreeSet::new();
        for receipt in &run.transitions {
            let edge = edges
                .get(receipt.edge_id.as_str())
                .ok_or_else(|| CausalPathError::UnknownEdge(receipt.edge_id.clone()))?;
            if !seen.insert(receipt.edge_id.as_str()) {
                return Err(CausalPathError::DuplicateTransitionReceipt {
                    run_id: run.arm_id.clone(),
                    edge_id: receipt.edge_id.clone(),
                });
            }
            for value in [
                receipt.source_value,
                receipt.target_before,
                receipt.target_after,
            ] {
                if !value.is_finite() {
                    return Err(CausalPathError::NonFiniteTransition {
                        run_id: run.arm_id.clone(),
                        edge_id: receipt.edge_id.clone(),
                    });
                }
            }
            let expected_source = run.final_values[&edge.from_variable];
            let expected_before = run.initial_values[&edge.to_variable];
            let expected_after = run.final_values[&edge.to_variable];
            if !close(receipt.source_value, expected_source)
                || !close(receipt.target_before, expected_before)
                || !close(receipt.target_after, expected_after)
            {
                return Err(CausalPathError::TransitionStateMismatch {
                    run_id: run.arm_id.clone(),
                    edge_id: receipt.edge_id.clone(),
                });
            }
            let changed = !close(receipt.target_before, receipt.target_after);
            if receipt.applied != changed {
                return Err(CausalPathError::AppliedFlagMismatch {
                    run_id: run.arm_id.clone(),
                    edge_id: receipt.edge_id.clone(),
                });
            }
        }
        let outcome = &run.independent_outcome;
        if outcome.variable_id != self.outcome_variable_id
            || outcome.unit != variables[&self.outcome_variable_id.as_str()].unit
            || outcome.resource.trim().is_empty()
            || outcome.region_id.trim().is_empty()
            || outcome.tick == 0
            || outcome.source_record_id.trim().is_empty()
            || !outcome.value.is_finite()
        {
            return Err(CausalPathError::InvalidIndependentOutcome(
                run.arm_id.clone(),
            ));
        }
        if !close(run.final_values[&self.outcome_variable_id], outcome.value) {
            return Err(CausalPathError::IndependentOutcomeMismatch(
                run.arm_id.clone(),
            ));
        }
        Ok(())
    }

    fn validate_paired_inputs(
        &self,
        variables: &BTreeMap<&str, &CausalVariable>,
    ) -> Result<(), CausalPathError> {
        for run in [&self.control, &self.treatment, &self.negative_control] {
            if run.exogenous_snapshot_id != self.control.exogenous_snapshot_id {
                return Err(CausalPathError::ExogenousSnapshotMismatch(
                    run.arm_id.clone(),
                ));
            }
        }
        let reference_outcome = &self.control.independent_outcome;
        let mut source_record_ids = BTreeSet::new();
        for run in [
            &self.control,
            &self.treatment,
            &self.negative_control,
            &self.mechanism_ablation.arm,
        ] {
            if !source_record_ids.insert(run.independent_outcome.source_record_id.as_str()) {
                return Err(CausalPathError::DuplicateIndependentOutcomeRecord(
                    run.independent_outcome.source_record_id.clone(),
                ));
            }
        }
        for run in [
            &self.treatment,
            &self.negative_control,
            &self.mechanism_ablation.arm,
        ] {
            let outcome = &run.independent_outcome;
            if outcome.resource != reference_outcome.resource
                || outcome.region_id != reference_outcome.region_id
                || outcome.tick != reference_outcome.tick
            {
                return Err(CausalPathError::OutcomeIdentityMismatch(run.arm_id.clone()));
            }
        }
        for id in variables.keys() {
            let control = self.control.initial_values[*id];
            let treatment = self.treatment.initial_values[*id];
            if *id == self.intervention_variable_id {
                if close(control, treatment) {
                    return Err(CausalPathError::InterventionNotChanged);
                }
            } else if !close(control, treatment) {
                return Err(CausalPathError::UncontrolledInitialState((*id).to_owned()));
            }
            let negative = self.negative_control.initial_values[*id];
            if !close(control, negative) {
                return Err(CausalPathError::NegativeControlInitialStateChanged(
                    (*id).to_owned(),
                ));
            }
        }
        if close(
            self.control.initial_values[&self.intervention_variable_id],
            self.treatment.initial_values[&self.intervention_variable_id],
        ) {
            return Err(CausalPathError::InterventionNotChanged);
        }
        Ok(())
    }

    fn validate_mechanism_ablation(
        &self,
        variables: &BTreeMap<&str, &CausalVariable>,
    ) -> Result<(), CausalPathError> {
        let block_id = self.mechanism_ablation.block_variable_id.as_str();
        let mechanism_id = self
            .mechanism_ablation
            .blocked_mechanism_variable_id
            .as_str();
        if block_id == self.intervention_variable_id
            || block_id == self.negative_control_variable_id
            || block_id == self.outcome_variable_id
            || mechanism_id == self.intervention_variable_id
            || mechanism_id == self.negative_control_variable_id
            || mechanism_id == self.outcome_variable_id
            || block_id == mechanism_id
        {
            return Err(CausalPathError::AblationVariableRolesOverlap);
        }
        let block = variables
            .get(block_id)
            .ok_or_else(|| CausalPathError::UnknownVariable(block_id.to_owned()))?;
        let mechanism = variables
            .get(mechanism_id)
            .ok_or_else(|| CausalPathError::UnknownVariable(mechanism_id.to_owned()))?;
        if block.stage != CausalStage::Intervention {
            return Err(CausalPathError::AblationSwitchWrongStage(
                block_id.to_owned(),
            ));
        }
        if self.required_path_edge_ids.iter().any(|edge_id| {
            let edge = self.edges.iter().find(|candidate| candidate.id == *edge_id);
            edge.is_some_and(|item| item.from_variable == block_id || item.to_variable == block_id)
        }) {
            return Err(CausalPathError::AblationSwitchOnRequiredPath(
                block_id.to_owned(),
            ));
        }
        if !matches!(
            mechanism.stage,
            CausalStage::Mechanism | CausalStage::Intermediate
        ) {
            return Err(CausalPathError::AblationTargetWrongStage(
                mechanism_id.to_owned(),
            ));
        }
        let mechanism_is_on_required_path = self.required_path_edge_ids.iter().any(|edge_id| {
            self.edges
                .iter()
                .find(|edge| edge.id == *edge_id)
                .is_some_and(|edge| {
                    edge.from_variable == mechanism_id || edge.to_variable == mechanism_id
                })
        });
        if !mechanism_is_on_required_path {
            return Err(CausalPathError::AblationTargetNotOnRequiredPath(
                mechanism_id.to_owned(),
            ));
        }
        if self.mechanism_ablation.arm.exogenous_snapshot_id != self.control.exogenous_snapshot_id {
            return Err(CausalPathError::AblationExogenousSnapshotMismatch);
        }
        if !only_one_initial_value_differs(&self.treatment, &self.mechanism_ablation.arm, block_id)
        {
            return Err(CausalPathError::AblationInitialStateMismatch);
        }
        if !close(
            self.treatment.initial_values[&self.intervention_variable_id],
            self.mechanism_ablation.arm.initial_values[&self.intervention_variable_id],
        ) || !close(
            self.treatment.final_values[&self.intervention_variable_id],
            self.mechanism_ablation.arm.final_values[&self.intervention_variable_id],
        ) {
            return Err(CausalPathError::AblationChangedPrimaryIntervention);
        }
        if !close(
            self.treatment.final_values[block_id],
            self.treatment.initial_values[block_id],
        ) || !close(
            self.mechanism_ablation.arm.final_values[block_id],
            self.mechanism_ablation.arm.initial_values[block_id],
        ) {
            return Err(CausalPathError::AblationSwitchStateChanged(
                block_id.to_owned(),
            ));
        }
        if close(
            self.mechanism_ablation.arm.final_values[mechanism_id],
            self.treatment.final_values[mechanism_id],
        ) || !close(
            self.mechanism_ablation.arm.final_values[mechanism_id],
            self.control.final_values[mechanism_id],
        ) {
            return Err(CausalPathError::MechanismAblationDidNotBlock(
                mechanism_id.to_owned(),
            ));
        }
        Ok(())
    }

    fn validate_negative_control(
        &self,
        _variables: &BTreeMap<&str, &CausalVariable>,
    ) -> Result<(), CausalPathError> {
        for id in self.control.final_values.keys() {
            if id == &self.negative_control_variable_id {
                if close(
                    self.control.final_values[id],
                    self.negative_control.final_values[id],
                ) {
                    return Err(CausalPathError::NegativeControlDidNotActivate);
                }
            } else if !close(
                self.control.final_values[id],
                self.negative_control.final_values[id],
            ) {
                return Err(CausalPathError::NegativeControlChangedOutcomeState(
                    id.clone(),
                ));
            }
        }
        if self.control.independent_outcome.value != self.negative_control.independent_outcome.value
        {
            return Err(CausalPathError::NegativeControlChangedIndependentOutcome);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathEdgeResult {
    pub edge_id: String,
    pub from_variable: String,
    pub to_variable: String,
    pub treatment_source_value: f64,
    pub control_source_value: f64,
    pub source_changed: bool,
    pub treatment_target_delta: f64,
    pub treatment_transition_applied: bool,
    pub control_transition_applied: bool,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CausalPathResult {
    pub experiment_id: String,
    pub control_outcome: f64,
    pub treatment_outcome: f64,
    pub observed_treatment_effect: f64,
    pub negative_control_outcome: f64,
    pub negative_control_effect: f64,
    pub mechanism_ablation_outcome: f64,
    pub mechanism_ablation_effect: f64,
    pub ablation_exogenous_matched: bool,
    pub ablation_inputs_matched: bool,
    pub primary_intervention_preserved: bool,
    pub blocked_mechanism_suppressed: bool,
    pub ablation_outcome_agrees: bool,
    pub ablation_protocol_valid: bool,
    /// True only when blocking the named mediator removes the treatment effect.
    pub mechanism_necessity_supported: bool,
    pub mechanism_path_complete: bool,
    pub first_broken_edge: Option<String>,
    pub negative_control_isolated: bool,
    pub exogenous_inputs_matched: bool,
    pub single_intervention_difference: bool,
    pub independent_outcome_agrees: bool,
    pub qualified: bool,
    /// None when any predeclared conformance gate fails.
    pub qualified_treatment_effect: Option<f64>,
    pub path_edges: Vec<PathEdgeResult>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CausalPathError {
    EmptyExperimentId,
    EmptyRequiredPath,
    InvalidVariable,
    DuplicateVariable(String),
    InvalidEdge,
    DuplicateEdge(String),
    UnknownVariable(String),
    UnknownEdge(String),
    SelfEdge(String),
    VariableRolesOverlap,
    WrongVariableStage {
        variable_id: String,
        expected: CausalStage,
        actual: CausalStage,
    },
    NegativeControlOnRequiredPath,
    DiscontinuousPath(String),
    InvalidStageOrder(String),
    PathEndpointsMismatch,
    CyclicGraph,
    InvalidRunIdentity,
    RunStateSchemaMismatch(String),
    NonFiniteState {
        run_id: String,
        variable_id: String,
        value: f64,
    },
    TransitionReceiptCountMismatch {
        run_id: String,
        expected: usize,
        actual: usize,
    },
    DuplicateTransitionReceipt {
        run_id: String,
        edge_id: String,
    },
    NonFiniteTransition {
        run_id: String,
        edge_id: String,
    },
    TransitionStateMismatch {
        run_id: String,
        edge_id: String,
    },
    AppliedFlagMismatch {
        run_id: String,
        edge_id: String,
    },
    InvalidIndependentOutcome(String),
    IndependentOutcomeMismatch(String),
    OutcomeIdentityMismatch(String),
    DuplicateIndependentOutcomeRecord(String),
    ExogenousSnapshotMismatch(String),
    InterventionNotChanged,
    UncontrolledInitialState(String),
    NegativeControlInitialStateChanged(String),
    NegativeControlDidNotActivate,
    NegativeControlChangedOutcomeState(String),
    NegativeControlChangedIndependentOutcome,
    AblationVariableRolesOverlap,
    AblationSwitchWrongStage(String),
    AblationSwitchOnRequiredPath(String),
    AblationTargetWrongStage(String),
    AblationTargetNotOnRequiredPath(String),
    AblationExogenousSnapshotMismatch,
    AblationInitialStateMismatch,
    AblationChangedPrimaryIntervention,
    AblationSwitchStateChanged(String),
    MechanismAblationDidNotBlock(String),
}

impl fmt::Display for CausalPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExperimentId => write!(f, "experiment ID must not be empty"),
            Self::EmptyRequiredPath => write!(f, "required causal path must not be empty"),
            Self::InvalidVariable => write!(f, "variable ID and unit must not be empty"),
            Self::DuplicateVariable(id) => write!(f, "duplicate causal variable: {id}"),
            Self::InvalidEdge => write!(f, "causal edge ID must not be empty"),
            Self::DuplicateEdge(id) => write!(f, "duplicate causal edge: {id}"),
            Self::UnknownVariable(id) => write!(f, "unknown causal variable: {id}"),
            Self::UnknownEdge(id) => write!(f, "unknown causal edge: {id}"),
            Self::SelfEdge(id) => write!(f, "causal edge {id} is a self-edge"),
            Self::VariableRolesOverlap => write!(
                f,
                "intervention, outcome and negative-control variables must be distinct"
            ),
            Self::WrongVariableStage {
                variable_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "variable {variable_id} expected stage {expected:?}, got {actual:?}"
                )
            }
            Self::NegativeControlOnRequiredPath => write!(
                f,
                "negative-control variable cannot be on the required causal path"
            ),
            Self::DiscontinuousPath(id) => write!(f, "required path is discontinuous at edge {id}"),
            Self::InvalidStageOrder(id) => write!(f, "causal stage order is invalid at edge {id}"),
            Self::PathEndpointsMismatch => write!(
                f,
                "required path must start at the intervention and end at the measured outcome"
            ),
            Self::CyclicGraph => write!(f, "causal graph must be acyclic"),
            Self::InvalidRunIdentity => {
                write!(f, "run ID and exogenous snapshot ID must not be empty")
            }
            Self::RunStateSchemaMismatch(run) => write!(
                f,
                "run {run} does not contain exactly the declared state variables"
            ),
            Self::NonFiniteState {
                run_id,
                variable_id,
                value,
            } => write!(
                f,
                "run {run_id} variable {variable_id} has non-finite value {value}"
            ),
            Self::TransitionReceiptCountMismatch {
                run_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "run {run_id} has {actual} transition receipts; expected {expected}"
                )
            }
            Self::DuplicateTransitionReceipt { run_id, edge_id } => {
                write!(f, "run {run_id} has duplicate receipt for edge {edge_id}")
            }
            Self::NonFiniteTransition { run_id, edge_id } => write!(
                f,
                "run {run_id} edge {edge_id} has non-finite transition values"
            ),
            Self::TransitionStateMismatch { run_id, edge_id } => write!(
                f,
                "run {run_id} receipt {edge_id} does not match its state snapshot"
            ),
            Self::AppliedFlagMismatch { run_id, edge_id } => write!(
                f,
                "run {run_id} receipt {edge_id} applied flag disagrees with target state change"
            ),
            Self::InvalidIndependentOutcome(run) => write!(
                f,
                "run {run} has an incomplete or invalid independent outcome receipt"
            ),
            Self::IndependentOutcomeMismatch(run) => write!(
                f,
                "run {run} ledger outcome differs from declared final state"
            ),
            Self::OutcomeIdentityMismatch(run) => write!(
                f,
                "run {run} outcome receipt does not match the control resource, region and tick"
            ),
            Self::DuplicateIndependentOutcomeRecord(record_id) => {
                write!(
                    f,
                    "independent outcome record key is reused across arms: {record_id}"
                )
            }
            Self::ExogenousSnapshotMismatch(run) => {
                write!(f, "run {run} does not use the matched exogenous snapshot")
            }
            Self::InterventionNotChanged => write!(
                f,
                "treatment and control intervention assignments are identical"
            ),
            Self::UncontrolledInitialState(id) => write!(
                f,
                "initial state variable {id} changed outside the declared intervention"
            ),
            Self::NegativeControlInitialStateChanged(id) => {
                write!(f, "negative control initial state changed at {id}")
            }
            Self::NegativeControlDidNotActivate => {
                write!(f, "negative-control marker did not change")
            }
            Self::NegativeControlChangedOutcomeState(id) => {
                write!(f, "negative control unexpectedly changed {id}")
            }
            Self::NegativeControlChangedIndependentOutcome => write!(
                f,
                "negative control changed the independent measured outcome"
            ),
            Self::AblationVariableRolesOverlap => write!(
                f,
                "mechanism ablation switch, blocked mediator, primary intervention, outcome and negative-control roles must be distinct"
            ),
            Self::AblationSwitchWrongStage(id) => {
                write!(
                    f,
                    "mechanism ablation switch {id} must be an intervention-stage variable"
                )
            }
            Self::AblationSwitchOnRequiredPath(id) => {
                write!(
                    f,
                    "mechanism ablation switch {id} must not be a node on the required path"
                )
            }
            Self::AblationTargetWrongStage(id) => {
                write!(
                    f,
                    "mechanism ablation target {id} must be a mechanism or intermediate variable"
                )
            }
            Self::AblationTargetNotOnRequiredPath(id) => {
                write!(
                    f,
                    "mechanism ablation target {id} must lie on the required intervention-to-outcome path"
                )
            }
            Self::AblationExogenousSnapshotMismatch => {
                write!(
                    f,
                    "mechanism ablation arm does not use the matched exogenous snapshot"
                )
            }
            Self::AblationInitialStateMismatch => {
                write!(
                    f,
                    "mechanism ablation arm must differ from treatment only in the declared ablation switch"
                )
            }
            Self::AblationChangedPrimaryIntervention => {
                write!(
                    f,
                    "mechanism ablation arm must preserve the primary intervention assignment"
                )
            }
            Self::AblationSwitchStateChanged(id) => {
                write!(f, "mechanism ablation switch {id} changed during the run")
            }
            Self::MechanismAblationDidNotBlock(id) => {
                write!(
                    f,
                    "mechanism ablation did not suppress mediator {id} to its control value"
                )
            }
        }
    }
}
impl std::error::Error for CausalPathError {}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= VALUE_TOLERANCE
}

fn only_one_initial_value_differs(
    control: &RunArm,
    treatment: &RunArm,
    intervention_variable_id: &str,
) -> bool {
    control.initial_values.iter().all(|(id, control_value)| {
        if id == intervention_variable_id {
            !close(*control_value, treatment.initial_values[id])
        } else {
            close(*control_value, treatment.initial_values[id])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn experiment() -> CausalExperiment {
        let variables = vec![
            CausalVariable {
                id: "safeguard_enabled".into(),
                stage: CausalStage::Intervention,
                unit: "boolean".into(),
            },
            CausalVariable {
                id: "mechanism_blocked".into(),
                stage: CausalStage::Intervention,
                unit: "boolean".into(),
            },
            CausalVariable {
                id: "transfer_eligible".into(),
                stage: CausalStage::Mechanism,
                unit: "boolean".into(),
            },
            CausalVariable {
                id: "authorized_transfer_m3".into(),
                stage: CausalStage::Intermediate,
                unit: "m3".into(),
            },
            CausalVariable {
                id: "delivered_water_m3".into(),
                stage: CausalStage::Intermediate,
                unit: "m3".into(),
            },
            CausalVariable {
                id: "household_water_coverage".into(),
                stage: CausalStage::Outcome,
                unit: "fraction".into(),
            },
            CausalVariable {
                id: "unrelated_intermediate".into(),
                stage: CausalStage::Intermediate,
                unit: "m3".into(),
            },
            CausalVariable {
                id: "reputation_counter".into(),
                stage: CausalStage::NegativeControl,
                unit: "count".into(),
            },
        ];
        let edges = vec![
            CausalEdge {
                id: "safeguard_to_eligibility".into(),
                from_variable: "safeguard_enabled".into(),
                to_variable: "transfer_eligible".into(),
            },
            CausalEdge {
                id: "eligibility_to_authorization".into(),
                from_variable: "transfer_eligible".into(),
                to_variable: "authorized_transfer_m3".into(),
            },
            CausalEdge {
                id: "authorization_to_delivery".into(),
                from_variable: "authorized_transfer_m3".into(),
                to_variable: "delivered_water_m3".into(),
            },
            CausalEdge {
                id: "delivery_to_service".into(),
                from_variable: "delivered_water_m3".into(),
                to_variable: "household_water_coverage".into(),
            },
        ];
        let control_values = BTreeMap::from([
            ("safeguard_enabled".into(), 0.0),
            ("mechanism_blocked".into(), 0.0),
            ("transfer_eligible".into(), 0.0),
            ("authorized_transfer_m3".into(), 0.0),
            ("delivered_water_m3".into(), 0.0),
            ("household_water_coverage".into(), 0.0),
            ("unrelated_intermediate".into(), 0.0),
            ("reputation_counter".into(), 0.0),
        ]);
        let treatment_initial = BTreeMap::from([
            ("safeguard_enabled".into(), 1.0),
            ("mechanism_blocked".into(), 0.0),
            ("transfer_eligible".into(), 0.0),
            ("authorized_transfer_m3".into(), 0.0),
            ("delivered_water_m3".into(), 0.0),
            ("household_water_coverage".into(), 0.0),
            ("unrelated_intermediate".into(), 0.0),
            ("reputation_counter".into(), 0.0),
        ]);
        let treatment_final = BTreeMap::from([
            ("safeguard_enabled".into(), 1.0),
            ("mechanism_blocked".into(), 0.0),
            ("transfer_eligible".into(), 1.0),
            ("authorized_transfer_m3".into(), 10.0),
            ("delivered_water_m3".into(), 10.0),
            ("household_water_coverage".into(), 1.0),
            ("unrelated_intermediate".into(), 0.0),
            ("reputation_counter".into(), 0.0),
        ]);
        let negative_final = BTreeMap::from([
            ("safeguard_enabled".into(), 0.0),
            ("mechanism_blocked".into(), 0.0),
            ("transfer_eligible".into(), 0.0),
            ("authorized_transfer_m3".into(), 0.0),
            ("delivered_water_m3".into(), 0.0),
            ("household_water_coverage".into(), 0.0),
            ("unrelated_intermediate".into(), 0.0),
            ("reputation_counter".into(), 1.0),
        ]);
        let build_receipts = |initial: &BTreeMap<String, f64>,
                              final_values: &BTreeMap<String, f64>| {
            edges
                .iter()
                .map(|edge| {
                    let before = initial[&edge.to_variable];
                    let after = final_values[&edge.to_variable];
                    TransitionReceipt {
                        edge_id: edge.id.clone(),
                        source_value: final_values[&edge.from_variable],
                        target_before: before,
                        target_after: after,
                        applied: !close(before, after),
                    }
                })
                .collect::<Vec<_>>()
        };
        let ledger_outcome = |value: f64, record_id: &str| IndependentOutcomeReceipt {
            variable_id: "household_water_coverage".into(),
            value,
            unit: "fraction".into(),
            resource: "water".into(),
            region_id: "household_1".into(),
            tick: 1,
            source_record_id: record_id.into(),
        };

        CausalExperiment {
            experiment_id: "synthetic_safeguard_water_delivery".into(),
            intervention_variable_id: "safeguard_enabled".into(),
            negative_control_variable_id: "reputation_counter".into(),
            outcome_variable_id: "household_water_coverage".into(),
            required_path_edge_ids: edges.iter().map(|edge| edge.id.clone()).collect(),
            variables,
            edges: edges.clone(),
            control: RunArm {
                arm_id: "control".into(),
                exogenous_snapshot_id: "fixture:opening20_demand10_production0".into(),
                initial_values: control_values.clone(),
                final_values: control_values.clone(),
                transitions: build_receipts(&control_values, &control_values),
                independent_outcome: ledger_outcome(0.0, "ledger:control:tick1:household_1"),
            },
            treatment: RunArm {
                arm_id: "treatment".into(),
                exogenous_snapshot_id: "fixture:opening20_demand10_production0".into(),
                initial_values: treatment_initial.clone(),
                final_values: treatment_final.clone(),
                transitions: build_receipts(&treatment_initial, &treatment_final),
                independent_outcome: ledger_outcome(1.0, "ledger:treatment:tick1:household_1"),
            },
            negative_control: RunArm {
                arm_id: "negative_control".into(),
                exogenous_snapshot_id: "fixture:opening20_demand10_production0".into(),
                initial_values: control_values.clone(),
                final_values: negative_final.clone(),
                transitions: build_receipts(&control_values, &negative_final),
                independent_outcome: ledger_outcome(0.0, "ledger:negative:tick1:household_1"),
            },
            mechanism_ablation: MechanismAblation {
                block_variable_id: "mechanism_blocked".into(),
                blocked_mechanism_variable_id: "transfer_eligible".into(),
                arm: {
                    let ablation_initial = BTreeMap::from([
                        ("safeguard_enabled".into(), 1.0),
                        ("mechanism_blocked".into(), 1.0),
                        ("transfer_eligible".into(), 0.0),
                        ("authorized_transfer_m3".into(), 0.0),
                        ("delivered_water_m3".into(), 0.0),
                        ("household_water_coverage".into(), 0.0),
                        ("unrelated_intermediate".into(), 0.0),
                        ("reputation_counter".into(), 0.0),
                    ]);
                    RunArm {
                        arm_id: "mechanism_ablation".into(),
                        exogenous_snapshot_id: "fixture:opening20_demand10_production0".into(),
                        initial_values: ablation_initial.clone(),
                        final_values: ablation_initial.clone(),
                        transitions: build_receipts(&ablation_initial, &ablation_initial),
                        independent_outcome: ledger_outcome(
                            0.0,
                            "ledger:ablation:tick1:household_1",
                        ),
                    }
                },
            },
        }
    }

    #[test]
    fn paired_treatment_qualifies_only_when_the_whole_mechanism_path_changes() {
        let result = experiment().evaluate().unwrap();
        assert!(result.qualified);
        assert!(result.mechanism_path_complete);
        assert!(result.negative_control_isolated);
        assert!(result.exogenous_inputs_matched);
        assert!(result.single_intervention_difference);
        assert!(result.independent_outcome_agrees);
        assert_eq!(result.observed_treatment_effect, 1.0);
        assert_eq!(result.qualified_treatment_effect, Some(1.0));
        assert_eq!(result.path_edges.len(), 4);
    }

    #[test]
    fn mediator_ablation_removes_the_treatment_effect() {
        let result = experiment().evaluate().unwrap();
        assert!(result.qualified);
        assert!(result.ablation_protocol_valid);
        assert!(result.ablation_exogenous_matched);
        assert!(result.ablation_inputs_matched);
        assert!(result.primary_intervention_preserved);
        assert!(result.blocked_mechanism_suppressed);
        assert!(result.ablation_outcome_agrees);
        assert_eq!(result.mechanism_ablation_outcome, result.control_outcome);
        assert_eq!(result.mechanism_ablation_effect, 0.0);
        assert!(result.mechanism_necessity_supported);
    }

    #[test]
    fn uncontrolled_ablation_inputs_fail_closed() {
        let mut invalid = experiment();
        invalid
            .mechanism_ablation
            .arm
            .initial_values
            .insert("authorized_transfer_m3".into(), 2.0);
        let edge = invalid.required_path_edge_ids[1].clone();
        let receipt = invalid
            .mechanism_ablation
            .arm
            .transitions
            .iter_mut()
            .find(|receipt| receipt.edge_id == edge)
            .unwrap();
        receipt.target_before = 2.0;
        receipt.applied = true;
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::AblationInitialStateMismatch)
        ));
    }

    #[test]
    fn ablation_switch_assignment_must_remain_fixed_during_the_run() {
        let mut invalid = experiment();
        invalid
            .mechanism_ablation
            .arm
            .final_values
            .insert("mechanism_blocked".into(), 0.0);
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::AblationSwitchStateChanged(_))
        ));
    }

    #[test]
    fn mediator_must_be_blocked_to_its_control_value() {
        let mut invalid = experiment();
        invalid
            .mechanism_ablation
            .arm
            .final_values
            .insert("transfer_eligible".into(), 1.0);
        let edge = invalid.required_path_edge_ids[0].clone();
        let receipt = invalid
            .mechanism_ablation
            .arm
            .transitions
            .iter_mut()
            .find(|receipt| receipt.edge_id == edge)
            .unwrap();
        receipt.target_after = 1.0;
        receipt.applied = true;
        let next_edge = invalid.required_path_edge_ids[1].clone();
        invalid
            .mechanism_ablation
            .arm
            .transitions
            .iter_mut()
            .find(|receipt| receipt.edge_id == next_edge)
            .unwrap()
            .source_value = 1.0;
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::MechanismAblationDidNotBlock(_))
        ));
    }

    #[test]
    fn internal_counter_only_change_is_a_null_outcome_control() {
        let result = experiment().evaluate().unwrap();
        assert_eq!(result.negative_control_effect, 0.0);
        assert!(result.negative_control_isolated);
        assert_eq!(result.negative_control_outcome, result.control_outcome);
    }

    #[test]
    fn disconnected_downstream_path_withholds_effect_even_when_ledger_outcome_changes() {
        let mut broken = experiment();
        broken
            .treatment
            .final_values
            .insert("delivered_water_m3".into(), 0.0);

        let authorization_to_delivery = broken.required_path_edge_ids[2].clone();
        let receipt = broken
            .treatment
            .transitions
            .iter_mut()
            .find(|receipt| receipt.edge_id == authorization_to_delivery)
            .unwrap();
        receipt.target_after = 0.0;
        receipt.applied = false;

        let delivery_to_service = broken.required_path_edge_ids[3].clone();
        let receipt = broken
            .treatment
            .transitions
            .iter_mut()
            .find(|receipt| receipt.edge_id == delivery_to_service)
            .unwrap();
        receipt.source_value = 0.0;
        receipt.target_after = 1.0;
        receipt.applied = true;

        let result = broken.evaluate().unwrap();
        assert_eq!(result.observed_treatment_effect, 1.0);
        assert!(!result.mechanism_path_complete);
        assert_eq!(
            result.first_broken_edge.as_deref(),
            Some("authorization_to_delivery")
        );
        assert!(!result.qualified);
        assert_eq!(result.qualified_treatment_effect, None);
    }

    #[test]
    fn unrelated_initial_state_change_fails_the_matched_pair_design() {
        let mut invalid = experiment();
        invalid
            .treatment
            .initial_values
            .insert("authorized_transfer_m3".into(), 2.0);
        let authorization_edge = invalid.required_path_edge_ids[1].clone();
        invalid
            .treatment
            .transitions
            .iter_mut()
            .find(|receipt| receipt.edge_id == authorization_edge)
            .unwrap()
            .target_before = 2.0;
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::UncontrolledInitialState(_))
        ));
    }

    #[test]
    fn independent_outcome_record_keys_must_be_unique_across_arms() {
        let mut invalid = experiment();
        invalid
            .negative_control
            .independent_outcome
            .source_record_id = invalid.control.independent_outcome.source_record_id.clone();
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::DuplicateIndependentOutcomeRecord(_))
        ));
    }

    #[test]
    fn outcome_receipts_must_refer_to_the_same_resource_region_and_tick() {
        let mut invalid = experiment();
        invalid.negative_control.independent_outcome.region_id = "different_household".into();
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::OutcomeIdentityMismatch(_))
        ));
    }

    #[test]
    fn ablation_target_must_be_on_the_required_path() {
        let mut unrelated = experiment();
        unrelated.mechanism_ablation.blocked_mechanism_variable_id =
            "unrelated_intermediate".into();
        assert!(matches!(
            unrelated.validate(),
            Err(CausalPathError::AblationTargetNotOnRequiredPath(_))
        ));

        let mut invalid = experiment();
        invalid.mechanism_ablation.blocked_mechanism_variable_id = "authorized_transfer_m3".into();
        invalid
            .mechanism_ablation
            .arm
            .final_values
            .insert("authorized_transfer_m3".into(), 0.0);
        assert!(invalid.validate().is_ok());

        invalid.mechanism_ablation.blocked_mechanism_variable_id = "reputation_counter".into();
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::AblationVariableRolesOverlap)
        ));
    }

    #[test]
    fn changed_exogenous_snapshot_fails_closed() {
        let mut invalid = experiment();
        invalid.treatment.exogenous_snapshot_id = "different-weather".into();
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::ExogenousSnapshotMismatch(_))
        ));
    }

    #[test]
    fn independent_ledger_outcome_cannot_be_replaced_by_an_internal_proxy() {
        let mut invalid = experiment();
        invalid.treatment.independent_outcome.value = 0.5;
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::IndependentOutcomeMismatch(_))
        ));
    }

    #[test]
    fn causal_graph_cycles_are_rejected() {
        let mut invalid = experiment();
        invalid.edges.push(CausalEdge {
            id: "outcome_back_to_intervention".into(),
            from_variable: "household_water_coverage".into(),
            to_variable: "safeguard_enabled".into(),
        });
        invalid.control.transitions.push(TransitionReceipt {
            edge_id: "outcome_back_to_intervention".into(),
            source_value: 0.0,
            target_before: 0.0,
            target_after: 0.0,
            applied: false,
        });
        invalid.treatment.transitions.push(TransitionReceipt {
            edge_id: "outcome_back_to_intervention".into(),
            source_value: 1.0,
            target_before: 1.0,
            target_after: 1.0,
            applied: false,
        });
        invalid
            .negative_control
            .transitions
            .push(TransitionReceipt {
                edge_id: "outcome_back_to_intervention".into(),
                source_value: 0.0,
                target_before: 0.0,
                target_after: 0.0,
                applied: false,
            });
        assert!(matches!(
            invalid.validate(),
            Err(CausalPathError::CyclicGraph)
        ));
    }
}

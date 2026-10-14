// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic fabrication planning as a dependency DAG.
//!
//! Plans describe intended work against exact process specifications and
//! workpiece identities. They do not instantiate outputs, store execution
//! progress, predict success, or confer engineering/civil authority.

use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

use crate::{CapabilityNeedId, ProcessSpecId, WorkpieceId};

/// Stable identity of one reusable fabrication plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FabricationPlanId(StableId);

impl FabricationPlanId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for FabricationPlanId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Stable identity of one step inside a plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlanStepId(StableId);

impl PlanStepId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for PlanStepId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One intended process application. Capability needs point to F5 rich
/// envelopes; evidence kinds state what should be captured, not whether the
/// resulting work will pass an engineering constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanStep {
    pub id: PlanStepId,
    pub process_spec_id: ProcessSpecId,
    pub process_spec_revision: u64,
    workpieces: Vec<WorkpieceId>,
    capability_needs: Vec<CapabilityNeedId>,
    expected_evidence_kinds: Vec<StableId>,
}

#[derive(Deserialize)]
struct PlanStepWire {
    id: PlanStepId,
    process_spec_id: ProcessSpecId,
    process_spec_revision: u64,
    workpieces: Vec<WorkpieceId>,
    capability_needs: Vec<CapabilityNeedId>,
    expected_evidence_kinds: Vec<StableId>,
}

impl<'de> Deserialize<'de> for PlanStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanStepWire::deserialize(deserializer)?;
        Self::new(
            wire.id,
            wire.process_spec_id,
            wire.process_spec_revision,
            wire.workpieces,
            wire.capability_needs,
            wire.expected_evidence_kinds,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl PlanStep {
    pub fn new(
        id: PlanStepId,
        process_spec_id: ProcessSpecId,
        process_spec_revision: u64,
        mut workpieces: Vec<WorkpieceId>,
        mut capability_needs: Vec<CapabilityNeedId>,
        mut expected_evidence_kinds: Vec<StableId>,
    ) -> Result<Self, PlanError> {
        if workpieces.is_empty() {
            return Err(PlanError::StepWorkpieceRequired(id));
        }
        sort_unique_workpieces(&id, &mut workpieces)?;
        sort_unique_capability_needs(&id, &mut capability_needs)?;
        sort_unique_evidence_kinds(&id, &mut expected_evidence_kinds)?;
        Ok(Self {
            id,
            process_spec_id,
            process_spec_revision,
            workpieces,
            capability_needs,
            expected_evidence_kinds,
        })
    }

    pub fn workpieces(&self) -> &[WorkpieceId] {
        &self.workpieces
    }

    pub fn capability_needs(&self) -> &[CapabilityNeedId] {
        &self.capability_needs
    }

    pub fn expected_evidence_kinds(&self) -> &[StableId] {
        &self.expected_evidence_kinds
    }
}

/// Directed prerequisite edge. `prerequisite` must finish before `dependent`
/// may begin in a concrete execution state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PlanDependency {
    pub prerequisite: PlanStepId,
    pub dependent: PlanStepId,
}

#[derive(Deserialize)]
struct PlanDependencyWire {
    prerequisite: PlanStepId,
    dependent: PlanStepId,
}

impl<'de> Deserialize<'de> for PlanDependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanDependencyWire::deserialize(deserializer)?;
        Self::new(wire.prerequisite, wire.dependent).map_err(serde::de::Error::custom)
    }
}

impl PlanDependency {
    pub fn new(prerequisite: PlanStepId, dependent: PlanStepId) -> Result<Self, PlanError> {
        if prerequisite == dependent {
            return Err(PlanError::SelfDependency(prerequisite));
        }
        Ok(Self {
            prerequisite,
            dependent,
        })
    }

    /// Replays the primitive dependency theorem for a publicly mutable value
    /// before it is admitted into a plan graph.
    pub fn validate_current(&self) -> Result<(), PlanError> {
        if self.prerequisite == self.dependent {
            return Err(PlanError::SelfDependency(self.prerequisite.clone()));
        }
        Ok(())
    }
}

/// Immutable reusable plan definition. Runtime progress belongs to a later
/// execution layer and must not mutate this design knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FabricationPlan {
    pub id: FabricationPlanId,
    pub revision: u64,
    steps: Vec<PlanStep>,
    dependencies: Vec<PlanDependency>,
}

#[derive(Deserialize)]
struct FabricationPlanWire {
    id: FabricationPlanId,
    revision: u64,
    steps: Vec<PlanStep>,
    dependencies: Vec<PlanDependency>,
}

impl<'de> Deserialize<'de> for FabricationPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FabricationPlanWire::deserialize(deserializer)?;
        Self::new(wire.id, wire.revision, wire.steps, wire.dependencies)
            .map_err(serde::de::Error::custom)
    }
}

impl FabricationPlan {
    pub fn new(
        id: FabricationPlanId,
        revision: u64,
        mut steps: Vec<PlanStep>,
        mut dependencies: Vec<PlanDependency>,
    ) -> Result<Self, PlanError> {
        if steps.is_empty() {
            return Err(PlanError::StepRequired);
        }

        steps.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in steps.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(PlanError::DuplicateStep(pair[0].id.clone()));
            }
        }

        for dependency in &dependencies {
            dependency.validate_current()?;
        }
        dependencies.sort();
        for pair in dependencies.windows(2) {
            if pair[0] == pair[1] {
                return Err(PlanError::DuplicateDependency(pair[0].clone()));
            }
        }

        let step_ids = steps
            .iter()
            .map(|step| step.id.clone())
            .collect::<BTreeSet<_>>();
        for dependency in &dependencies {
            if !step_ids.contains(&dependency.prerequisite) {
                return Err(PlanError::UnknownDependencyStep(
                    dependency.prerequisite.clone(),
                ));
            }
            if !step_ids.contains(&dependency.dependent) {
                return Err(PlanError::UnknownDependencyStep(
                    dependency.dependent.clone(),
                ));
            }
        }

        let plan = Self {
            id,
            revision,
            steps,
            dependencies,
        };
        if plan.topological_order_internal().len() != plan.steps.len() {
            return Err(PlanError::CycleDetected);
        }
        Ok(plan)
    }

    pub fn steps(&self) -> &[PlanStep] {
        &self.steps
    }

    pub fn dependencies(&self) -> &[PlanDependency] {
        &self.dependencies
    }

    pub fn step(&self, id: &PlanStepId) -> Option<&PlanStep> {
        self.steps
            .binary_search_by(|candidate| candidate.id.cmp(id))
            .ok()
            .map(|index| &self.steps[index])
    }

    /// Stable topological order. When several nodes are simultaneously ready,
    /// their stable step IDs determine order; no hidden cost/quality heuristic
    /// is used.
    pub fn topological_order(&self) -> Vec<PlanStepId> {
        self.topological_order_internal()
    }

    /// Returns currently executable plan steps for an externally supplied set
    /// of completed steps. The plan itself stores no mutable progress.
    pub fn ready_steps(&self, completed: &[PlanStepId]) -> Result<Vec<PlanStepId>, PlanError> {
        let mut completed_set = BTreeSet::new();
        for step_id in completed {
            if self.step(step_id).is_none() {
                return Err(PlanError::UnknownCompletedStep(step_id.clone()));
            }
            if !completed_set.insert(step_id.clone()) {
                return Err(PlanError::DuplicateCompletedStep(step_id.clone()));
            }
        }

        for step_id in &completed_set {
            for dependency in self
                .dependencies
                .iter()
                .filter(|dependency| &dependency.dependent == step_id)
            {
                if !completed_set.contains(&dependency.prerequisite) {
                    return Err(PlanError::InvalidCompletionOrder {
                        step_id: step_id.clone(),
                        missing_prerequisite: dependency.prerequisite.clone(),
                    });
                }
            }
        }

        let mut ready = Vec::new();
        for step in &self.steps {
            if completed_set.contains(&step.id) {
                continue;
            }
            let prerequisites_satisfied = self
                .dependencies
                .iter()
                .filter(|dependency| dependency.dependent == step.id)
                .all(|dependency| completed_set.contains(&dependency.prerequisite));
            if prerequisites_satisfied {
                ready.push(step.id.clone());
            }
        }
        Ok(ready)
    }

    fn topological_order_internal(&self) -> Vec<PlanStepId> {
        let mut indegree = self
            .steps
            .iter()
            .map(|step| (step.id.clone(), 0usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing = BTreeMap::<PlanStepId, Vec<PlanStepId>>::new();

        for dependency in &self.dependencies {
            *indegree
                .get_mut(&dependency.dependent)
                .expect("dependencies are validated against plan steps") += 1;
            outgoing
                .entry(dependency.prerequisite.clone())
                .or_default()
                .push(dependency.dependent.clone());
        }
        for dependents in outgoing.values_mut() {
            dependents.sort();
        }

        let mut ready = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.steps.len());

        while let Some(step_id) = ready.pop_first() {
            order.push(step_id.clone());
            if let Some(dependents) = outgoing.get(&step_id) {
                for dependent in dependents {
                    let degree = indegree
                        .get_mut(dependent)
                        .expect("dependency target is a validated plan step");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(dependent.clone());
                    }
                }
            }
        }
        order
    }
}

#[derive(Debug)]
pub enum PlanError {
    StepRequired,
    StepWorkpieceRequired(PlanStepId),
    DuplicateStep(PlanStepId),
    DuplicateStepWorkpiece {
        step_id: PlanStepId,
        workpiece_id: WorkpieceId,
    },
    DuplicateCapabilityNeed {
        step_id: PlanStepId,
        capability_need_id: CapabilityNeedId,
    },
    DuplicateEvidenceKind {
        step_id: PlanStepId,
        evidence_kind: StableId,
    },
    SelfDependency(PlanStepId),
    DuplicateDependency(PlanDependency),
    UnknownDependencyStep(PlanStepId),
    CycleDetected,
    UnknownCompletedStep(PlanStepId),
    DuplicateCompletedStep(PlanStepId),
    InvalidCompletionOrder {
        step_id: PlanStepId,
        missing_prerequisite: PlanStepId,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StepRequired => write!(formatter, "fabrication plan requires at least one step"),
            Self::StepWorkpieceRequired(id) => {
                write!(formatter, "plan step {id} requires at least one workpiece")
            }
            Self::DuplicateStep(id) => write!(formatter, "fabrication plan repeats step {id}"),
            Self::DuplicateStepWorkpiece {
                step_id,
                workpiece_id,
            } => write!(
                formatter,
                "plan step {step_id} repeats workpiece {workpiece_id}"
            ),
            Self::DuplicateCapabilityNeed {
                step_id,
                capability_need_id,
            } => write!(
                formatter,
                "plan step {step_id} repeats capability need {capability_need_id}"
            ),
            Self::DuplicateEvidenceKind {
                step_id,
                evidence_kind,
            } => write!(
                formatter,
                "plan step {step_id} repeats expected evidence kind {evidence_kind}"
            ),
            Self::SelfDependency(id) => write!(formatter, "plan step {id} cannot depend on itself"),
            Self::DuplicateDependency(dependency) => write!(
                formatter,
                "fabrication plan repeats dependency {} -> {}",
                dependency.prerequisite, dependency.dependent
            ),
            Self::UnknownDependencyStep(id) => {
                write!(
                    formatter,
                    "fabrication dependency references unknown step {id}"
                )
            }
            Self::CycleDetected => write!(
                formatter,
                "fabrication plan dependency graph contains a cycle"
            ),
            Self::UnknownCompletedStep(id) => {
                write!(formatter, "completed-set references unknown plan step {id}")
            }
            Self::DuplicateCompletedStep(id) => {
                write!(formatter, "completed-set repeats plan step {id}")
            }
            Self::InvalidCompletionOrder {
                step_id,
                missing_prerequisite,
            } => write!(
                formatter,
                "plan step {step_id} cannot be completed before prerequisite {missing_prerequisite}"
            ),
        }
    }
}

impl Error for PlanError {}

fn sort_unique_workpieces(
    step_id: &PlanStepId,
    values: &mut [WorkpieceId],
) -> Result<(), PlanError> {
    values.sort();
    for pair in values.windows(2) {
        if pair[0] == pair[1] {
            return Err(PlanError::DuplicateStepWorkpiece {
                step_id: step_id.clone(),
                workpiece_id: pair[0].clone(),
            });
        }
    }
    Ok(())
}

fn sort_unique_capability_needs(
    step_id: &PlanStepId,
    values: &mut [CapabilityNeedId],
) -> Result<(), PlanError> {
    values.sort();
    for pair in values.windows(2) {
        if pair[0] == pair[1] {
            return Err(PlanError::DuplicateCapabilityNeed {
                step_id: step_id.clone(),
                capability_need_id: pair[0].clone(),
            });
        }
    }
    Ok(())
}

fn sort_unique_evidence_kinds(
    step_id: &PlanStepId,
    values: &mut [StableId],
) -> Result<(), PlanError> {
    values.sort();
    for pair in values.windows(2) {
        if pair[0] == pair[1] {
            return Err(PlanError::DuplicateEvidenceKind {
                step_id: step_id.clone(),
                evidence_kind: pair[0].clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn step(name: &str, workpiece: &str) -> PlanStep {
        PlanStep::new(
            PlanStepId::new(id(&format!("step:{name}"))),
            ProcessSpecId::new(id(&format!("process-spec:{name}"))),
            1,
            vec![WorkpieceId::new(id(workpiece))],
            Vec::new(),
            vec![id(&format!("evidence-kind:{name}"))],
        )
        .unwrap()
    }

    fn dependency(left: &str, right: &str) -> PlanDependency {
        PlanDependency::new(
            PlanStepId::new(id(&format!("step:{left}"))),
            PlanStepId::new(id(&format!("step:{right}"))),
        )
        .unwrap()
    }

    fn patch_plan(steps: Vec<PlanStep>, dependencies: Vec<PlanDependency>) -> FabricationPlan {
        FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:patch-conduit")),
            3,
            steps,
            dependencies,
        )
        .unwrap()
    }

    #[test]
    fn patch_conduit_plan_is_shared_process_dag_not_item_constructor() {
        let plan = patch_plan(
            vec![
                step("pressure-test", "workpiece:conduit"),
                step("seal", "workpiece:conduit"),
                step("align", "workpiece:patch"),
                step("clean", "workpiece:patch"),
                step("clamp", "workpiece:patch"),
            ],
            vec![
                dependency("clean", "align"),
                dependency("align", "clamp"),
                dependency("clamp", "seal"),
                dependency("seal", "pressure-test"),
            ],
        );

        let names = plan
            .topological_order()
            .into_iter()
            .map(|id| id.stable_id().as_str().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "step:clean",
                "step:align",
                "step:clamp",
                "step:seal",
                "step:pressure-test",
            ]
        );
    }

    #[test]
    fn cycle_is_rejected_at_plan_construction() {
        let result = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:cycle")),
            1,
            vec![step("a", "workpiece:a"), step("b", "workpiece:b")],
            vec![dependency("a", "b"), dependency("b", "a")],
        );
        assert!(matches!(result, Err(PlanError::CycleDetected)));
    }

    #[test]
    fn topological_order_is_independent_of_input_order() {
        let forward = patch_plan(
            vec![step("a", "workpiece:a"), step("b", "workpiece:b")],
            Vec::new(),
        );
        let reverse = patch_plan(
            vec![step("b", "workpiece:b"), step("a", "workpiece:a")],
            Vec::new(),
        );
        assert_eq!(forward.topological_order(), reverse.topological_order());
    }

    #[test]
    fn ready_steps_validate_external_progress_order() {
        let plan = patch_plan(
            vec![step("clean", "workpiece:a"), step("seal", "workpiece:a")],
            vec![dependency("clean", "seal")],
        );
        assert_eq!(
            plan.ready_steps(&[]).unwrap(),
            vec![PlanStepId::new(id("step:clean"))]
        );
        assert_eq!(
            plan.ready_steps(&[PlanStepId::new(id("step:clean"))])
                .unwrap(),
            vec![PlanStepId::new(id("step:seal"))]
        );
        let invalid = plan.ready_steps(&[PlanStepId::new(id("step:seal"))]);
        assert!(matches!(
            invalid,
            Err(PlanError::InvalidCompletionOrder { .. })
        ));
    }

    #[test]
    fn plan_step_wire_normalizes_set_like_order() {
        let step = PlanStep::new(
            PlanStepId::new(id("step:wire-order")),
            ProcessSpecId::new(id("process-spec:wire-order")),
            4,
            vec![
                WorkpieceId::new(id("workpiece:a")),
                WorkpieceId::new(id("workpiece:b")),
            ],
            vec![
                CapabilityNeedId::new(id("capability-need:a")),
                CapabilityNeedId::new(id("capability-need:b")),
            ],
            vec![id("evidence-kind:a"), id("evidence-kind:b")],
        )
        .unwrap();
        let mut value = serde_json::to_value(&step).unwrap();
        value["workpieces"].as_array_mut().unwrap().reverse();
        value["capability_needs"].as_array_mut().unwrap().reverse();
        value["expected_evidence_kinds"]
            .as_array_mut()
            .unwrap()
            .reverse();

        let restored: PlanStep = serde_json::from_value(value).unwrap();
        assert_eq!(restored, step);
    }

    #[test]
    fn plan_step_wire_rejects_duplicate_or_missing_set_members() {
        let step = PlanStep::new(
            PlanStepId::new(id("step:wire-invalid")),
            ProcessSpecId::new(id("process-spec:wire-invalid")),
            1,
            vec![WorkpieceId::new(id("workpiece:a"))],
            vec![CapabilityNeedId::new(id("capability-need:a"))],
            vec![id("evidence-kind:a")],
        )
        .unwrap();

        let mut duplicate_workpiece = serde_json::to_value(&step).unwrap();
        let duplicate = duplicate_workpiece["workpieces"][0].clone();
        duplicate_workpiece["workpieces"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<PlanStep>(duplicate_workpiece).is_err());

        let mut duplicate_capability = serde_json::to_value(&step).unwrap();
        let duplicate = duplicate_capability["capability_needs"][0].clone();
        duplicate_capability["capability_needs"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<PlanStep>(duplicate_capability).is_err());

        let mut duplicate_evidence = serde_json::to_value(&step).unwrap();
        let duplicate = duplicate_evidence["expected_evidence_kinds"][0].clone();
        duplicate_evidence["expected_evidence_kinds"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<PlanStep>(duplicate_evidence).is_err());

        let mut empty_workpieces = serde_json::to_value(&step).unwrap();
        empty_workpieces["workpieces"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<PlanStep>(empty_workpieces).is_err());
    }

    #[test]
    fn plan_step_canonical_round_trip_succeeds() {
        let original = step("round-trip", "workpiece:round-trip");
        let encoded = serde_json::to_vec(&original).unwrap();
        let restored: PlanStep = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn dependency_wire_and_public_mutation_reject_self_edges() {
        let original = dependency("clean", "inspect");
        let mut value = serde_json::to_value(&original).unwrap();
        value["dependent"] = value["prerequisite"].clone();
        assert!(serde_json::from_value::<PlanDependency>(value).is_err());

        let mut mutated = original;
        mutated.dependent = mutated.prerequisite.clone();
        let result = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:mutated-dependency")),
            1,
            vec![step("clean", "workpiece:clean")],
            vec![mutated],
        );
        assert!(matches!(result, Err(PlanError::SelfDependency(_))));
    }

    #[test]
    fn plan_wire_normalizes_valid_step_and_dependency_order() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("seal", "workpiece:seal"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "seal"), dependency("seal", "inspect")],
        );
        let expected_order = original.topological_order();
        let mut value = serde_json::to_value(&original).unwrap();
        value["steps"].as_array_mut().unwrap().reverse();
        value["dependencies"].as_array_mut().unwrap().reverse();

        let restored: FabricationPlan = serde_json::from_value(value).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.topological_order(), expected_order);
    }

    #[test]
    fn plan_wire_rejects_duplicate_step_and_dependency() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "inspect")],
        );

        let mut duplicate_step = serde_json::to_value(&original).unwrap();
        let duplicate = duplicate_step["steps"][0].clone();
        duplicate_step["steps"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<FabricationPlan>(duplicate_step).is_err());

        let mut duplicate_dependency = serde_json::to_value(&original).unwrap();
        let duplicate = duplicate_dependency["dependencies"][0].clone();
        duplicate_dependency["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<FabricationPlan>(duplicate_dependency).is_err());
    }

    #[test]
    fn plan_wire_rejects_unknown_dependency_endpoints() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "inspect")],
        );

        let mut unknown_prerequisite = serde_json::to_value(&original).unwrap();
        unknown_prerequisite["dependencies"][0]["prerequisite"] =
            serde_json::Value::String("step:missing-prerequisite".into());
        assert!(serde_json::from_value::<FabricationPlan>(unknown_prerequisite).is_err());

        let mut unknown_dependent = serde_json::to_value(&original).unwrap();
        unknown_dependent["dependencies"][0]["dependent"] =
            serde_json::Value::String("step:missing-dependent".into());
        assert!(serde_json::from_value::<FabricationPlan>(unknown_dependent).is_err());
    }

    #[test]
    fn plan_wire_rejects_simple_and_multi_node_cycles() {
        let simple = patch_plan(
            vec![step("a", "workpiece:a"), step("b", "workpiece:b")],
            vec![dependency("a", "b")],
        );
        let mut simple_value = serde_json::to_value(&simple).unwrap();
        simple_value["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "prerequisite": "step:b",
                "dependent": "step:a"
            }));
        assert!(serde_json::from_value::<FabricationPlan>(simple_value).is_err());

        let multi = patch_plan(
            vec![
                step("a", "workpiece:a"),
                step("b", "workpiece:b"),
                step("c", "workpiece:c"),
            ],
            vec![dependency("a", "b"), dependency("b", "c")],
        );
        let mut multi_value = serde_json::to_value(&multi).unwrap();
        multi_value["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "prerequisite": "step:c",
                "dependent": "step:a"
            }));
        assert!(serde_json::from_value::<FabricationPlan>(multi_value).is_err());
    }

    #[test]
    fn fabrication_plan_canonical_round_trip_succeeds() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "inspect")],
        );
        let expected_order = original.topological_order();
        let encoded = serde_json::to_vec(&original).unwrap();
        let restored: FabricationPlan = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.topological_order(), expected_order);
    }

    #[test]
    fn plan_serialization_contains_no_runtime_or_authority_claims() {
        let plan = patch_plan(vec![step("clean", "workpiece:a")], Vec::new());
        let value = serde_json::to_value(plan).unwrap();
        for forbidden in [
            "output",
            "item",
            "progress",
            "success",
            "quality",
            "score",
            "authorized",
            "commissioned",
        ] {
            assert!(
                value.get(forbidden).is_none(),
                "unexpected field {forbidden}"
            );
        }
    }
}

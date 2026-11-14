// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact executable-plan semantics over authored fabrication plans.
//!
//! `FabricationPlan` remains planning/design structure. This module adds the
//! execution boundary: every plan step must be bound to one exact canonical F4
//! `ProcessSpecSnapshot` before the plan can be treated as executable. The
//! resulting value is structural authority; construction can later bind this
//! whole contract rather than trusting plan ID/revision alone.

use serde::{Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{FabricationPlan, PlanStepId, ProcessSpecId, ProcessSpecSnapshot};

/// One exact F10 -> F4 semantic binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactPlanProcessBinding {
    pub step_id: PlanStepId,
    pub process_spec: ProcessSpecSnapshot,
}

impl ExactPlanProcessBinding {
    pub fn new(step_id: PlanStepId, process_spec: ProcessSpecSnapshot) -> Self {
        Self {
            step_id,
            process_spec,
        }
    }
}

/// A fabrication plan whose every step has exactly one canonical F4 process
/// contract and whose F5 capability-need identities agree with that contract's
/// bootstrap requirements.
///
/// This is deliberately a separate type from `FabricationPlan`: authored plan
/// shape is useful before process resolution, but only this exact-bound value is
/// suitable for a construction execution authority boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutableFabricationPlan {
    plan: FabricationPlan,
    process_bindings: Vec<ExactPlanProcessBinding>,
}

impl ExecutableFabricationPlan {
    pub fn new(
        plan: FabricationPlan,
        mut process_bindings: Vec<ExactPlanProcessBinding>,
    ) -> Result<Self, ExecutablePlanError> {
        process_bindings.sort_by(|left, right| left.step_id.cmp(&right.step_id));
        for pair in process_bindings.windows(2) {
            if pair[0].step_id == pair[1].step_id {
                return Err(ExecutablePlanError::DuplicateStepBinding(
                    pair[0].step_id.clone(),
                ));
            }
        }

        for binding in &process_bindings {
            let step = plan
                .step(&binding.step_id)
                .ok_or_else(|| ExecutablePlanError::UnknownStepBinding(binding.step_id.clone()))?;

            if binding.process_spec.id != step.process_spec_id
                || binding.process_spec.revision != step.process_spec_revision
            {
                return Err(ExecutablePlanError::ProcessIdentityMismatch {
                    step_id: binding.step_id.clone(),
                    expected_id: step.process_spec_id.clone(),
                    expected_revision: step.process_spec_revision,
                    actual_id: binding.process_spec.id.clone(),
                    actual_revision: binding.process_spec.revision,
                });
            }

            let mut expected_capabilities = step
                .capability_needs()
                .iter()
                .map(|need| need.stable_id().clone())
                .collect::<Vec<_>>();
            expected_capabilities.sort();

            let mut actual_capabilities = binding
                .process_spec
                .required_capabilities()
                .iter()
                .map(|requirement| requirement.capability_id.clone())
                .collect::<Vec<_>>();
            actual_capabilities.sort();

            if expected_capabilities != actual_capabilities {
                return Err(ExecutablePlanError::CapabilityBindingMismatch {
                    step_id: binding.step_id.clone(),
                    expected: expected_capabilities,
                    actual: actual_capabilities,
                });
            }
        }

        for step in plan.steps() {
            if process_bindings
                .binary_search_by(|binding| binding.step_id.cmp(&step.id))
                .is_err()
            {
                return Err(ExecutablePlanError::MissingStepBinding(step.id.clone()));
            }
        }

        Ok(Self {
            plan,
            process_bindings,
        })
    }

    pub fn plan(&self) -> &FabricationPlan {
        &self.plan
    }

    pub fn process_bindings(&self) -> &[ExactPlanProcessBinding] {
        &self.process_bindings
    }

    pub fn binding(&self, step_id: &PlanStepId) -> Option<&ExactPlanProcessBinding> {
        self.process_bindings
            .binary_search_by(|binding| binding.step_id.cmp(step_id))
            .ok()
            .map(|index| &self.process_bindings[index])
    }

    pub fn process_spec(&self, step_id: &PlanStepId) -> Option<&ProcessSpecSnapshot> {
        self.binding(step_id).map(|binding| &binding.process_spec)
    }
}

#[derive(Deserialize)]
struct ExecutableFabricationPlanWire {
    plan: FabricationPlan,
    process_bindings: Vec<ExactPlanProcessBinding>,
}

impl<'de> Deserialize<'de> for ExecutableFabricationPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExecutableFabricationPlanWire::deserialize(deserializer)?;
        Self::new(wire.plan, wire.process_bindings).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum ExecutablePlanError {
    MissingStepBinding(PlanStepId),
    DuplicateStepBinding(PlanStepId),
    UnknownStepBinding(PlanStepId),
    ProcessIdentityMismatch {
        step_id: PlanStepId,
        expected_id: ProcessSpecId,
        expected_revision: u64,
        actual_id: ProcessSpecId,
        actual_revision: u64,
    },
    CapabilityBindingMismatch {
        step_id: PlanStepId,
        expected: Vec<StableId>,
        actual: Vec<StableId>,
    },
}

impl fmt::Display for ExecutablePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStepBinding(step_id) => {
                write!(
                    formatter,
                    "executable plan lacks an exact process binding for step {step_id}"
                )
            }
            Self::DuplicateStepBinding(step_id) => {
                write!(
                    formatter,
                    "executable plan repeats process binding for step {step_id}"
                )
            }
            Self::UnknownStepBinding(step_id) => {
                write!(
                    formatter,
                    "executable process binding references unknown step {step_id}"
                )
            }
            Self::ProcessIdentityMismatch {
                step_id,
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "plan step {step_id} references process {expected_id}@{expected_revision}, exact binding supplied {actual_id}@{actual_revision}"
            ),
            Self::CapabilityBindingMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "plan step {step_id} capability bindings differ from exact F4 bootstrap requirements: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl Error for ExecutablePlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CapabilityNeedId, CapabilityRequirement, FabricationPlanId, PlanDependency, PlanStep,
        ProcessKind, ProcessSpec, WorkpieceId, WorkpieceLifecycle,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn process_id(name: &str) -> ProcessSpecId {
        ProcessSpecId::new(id(&format!("process-spec:{name}")))
    }

    fn step_id(name: &str) -> PlanStepId {
        PlanStepId::new(id(&format!("step:{name}")))
    }

    fn capability_id(name: &str) -> CapabilityNeedId {
        CapabilityNeedId::new(id(&format!("capability-need:{name}")))
    }

    fn step(name: &str, capability: Option<&str>) -> PlanStep {
        PlanStep::new(
            step_id(name),
            process_id(name),
            1,
            vec![WorkpieceId::new(id(&format!("workpiece:{name}")))],
            capability.into_iter().map(capability_id).collect(),
            vec![id(&format!("evidence-kind:{name}"))],
        )
        .unwrap()
    }

    fn plan() -> FabricationPlan {
        FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:test")),
            3,
            vec![step("clean", Some("clean")), step("inspect", None)],
            vec![PlanDependency::new(step_id("clean"), step_id("inspect")).unwrap()],
        )
        .unwrap()
    }

    fn spec(
        name: &str,
        kind: ProcessKind,
        capability: Option<&str>,
        states: Vec<WorkpieceLifecycle>,
    ) -> ProcessSpecSnapshot {
        ProcessSpec::new(
            process_id(name),
            1,
            kind,
            capability
                .into_iter()
                .map(|name| CapabilityRequirement {
                    capability_id: capability_id(name).stable_id().clone(),
                    minimum_value: 1,
                })
                .collect(),
            states,
        )
        .unwrap()
        .snapshot()
    }

    fn bindings() -> Vec<ExactPlanProcessBinding> {
        vec![
            ExactPlanProcessBinding::new(
                step_id("clean"),
                spec(
                    "clean",
                    ProcessKind::Clean,
                    Some("clean"),
                    vec![WorkpieceLifecycle::Available],
                ),
            ),
            ExactPlanProcessBinding::new(
                step_id("inspect"),
                spec(
                    "inspect",
                    ProcessKind::Inspect,
                    None,
                    vec![WorkpieceLifecycle::Available],
                ),
            ),
        ]
    }

    #[test]
    fn every_plan_step_requires_one_exact_process_binding() {
        let mut incomplete = bindings();
        incomplete.pop();
        let result = ExecutableFabricationPlan::new(plan(), incomplete);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::MissingStepBinding(step))
                if step == step_id("inspect")
        ));
    }

    #[test]
    fn duplicate_exact_binding_is_rejected() {
        let mut duplicated = bindings();
        duplicated.push(duplicated[0].clone());
        let result = ExecutableFabricationPlan::new(plan(), duplicated);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::DuplicateStepBinding(step))
                if step == step_id("clean")
        ));
    }

    #[test]
    fn unknown_step_binding_is_rejected() {
        let mut values = bindings();
        values.push(ExactPlanProcessBinding::new(
            step_id("unknown"),
            spec(
                "unknown",
                ProcessKind::Inspect,
                None,
                vec![WorkpieceLifecycle::Available],
            ),
        ));
        let result = ExecutableFabricationPlan::new(plan(), values);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::UnknownStepBinding(step))
                if step == step_id("unknown")
        ));
    }

    #[test]
    fn process_identity_and_revision_must_match_authored_step_reference() {
        let mut values = bindings();
        values[0].process_spec = ProcessSpec::new(
            process_id("clean"),
            2,
            ProcessKind::Clean,
            vec![CapabilityRequirement {
                capability_id: capability_id("clean").stable_id().clone(),
                minimum_value: 1,
            }],
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
        .snapshot();

        let result = ExecutableFabricationPlan::new(plan(), values);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::ProcessIdentityMismatch {
                expected_revision: 1,
                actual_revision: 2,
                ..
            })
        ));
    }

    #[test]
    fn exact_f4_bootstrap_requirements_must_match_f10_capability_needs() {
        let mut values = bindings();
        values[0].process_spec = spec(
            "clean",
            ProcessKind::Clean,
            None,
            vec![WorkpieceLifecycle::Available],
        );

        let result = ExecutableFabricationPlan::new(plan(), values);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::CapabilityBindingMismatch {
                step_id: actual_step_id,
                ..
            }) if actual_step_id == step_id("clean")
        ));
    }

    #[test]
    fn same_plan_id_revision_with_changed_process_semantics_is_structurally_distinct() {
        let left = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let mut altered = bindings();
        altered[1].process_spec = spec(
            "inspect",
            ProcessKind::Inspect,
            None,
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        );
        let right = ExecutableFabricationPlan::new(plan(), altered).unwrap();

        assert_eq!(left.plan().id, right.plan().id);
        assert_eq!(left.plan().revision, right.plan().revision);
        assert_ne!(left, right);
    }

    #[test]
    fn deserialization_revalidates_executable_plan_contract() {
        let executable = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["process_bindings"][0]["process_spec"]["revision"] = 99.into();

        let restored = serde_json::from_value::<ExecutableFabricationPlan>(value);
        assert!(restored.is_err());
    }

    #[test]
    fn valid_contract_round_trips_and_resolves_exact_process_by_step() {
        let executable = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let encoded = serde_json::to_vec(&executable).unwrap();
        let restored: ExecutableFabricationPlan = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(restored, executable);
        assert_eq!(
            restored.process_spec(&step_id("inspect")).unwrap().kind,
            ProcessKind::Inspect
        );
    }

    #[test]
    fn executable_contract_contains_no_runtime_or_authority_claims() {
        let executable = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let json = serde_json::to_string(&executable).unwrap();
        for forbidden in [
            "\"progress\"",
            "\"success\"",
            "\"quality\"",
            "\"score\"",
            "\"authorized\"",
            "\"commissioned\"",
            "\"safe\"",
        ] {
            assert!(!json.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}

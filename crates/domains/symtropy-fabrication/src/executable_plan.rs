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
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{
    CapabilityError, CapabilityNeedId, CapabilityNeedSnapshot, FabricationPlan, PlanStepId,
    ProcessSpecId, ProcessSpecSnapshot, ProcessSpecSnapshotError,
};

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
    capability_needs: Vec<CapabilityNeedSnapshot>,
}

impl ExecutableFabricationPlan {
    /// Convenience constructor for executable plans that require no F5
    /// capability needs. Capability-bearing plans fail closed here and must use
    /// [`Self::new_with_capability_needs`].
    pub fn new(
        plan: FabricationPlan,
        process_bindings: Vec<ExactPlanProcessBinding>,
    ) -> Result<Self, ExecutablePlanError> {
        Self::new_with_capability_needs(plan, process_bindings, Vec::new())
    }

    pub fn new_with_capability_needs(
        plan: FabricationPlan,
        mut process_bindings: Vec<ExactPlanProcessBinding>,
        mut capability_needs: Vec<CapabilityNeedSnapshot>,
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
            binding.process_spec.validate_canonical().map_err(|error| {
                ExecutablePlanError::NonCanonicalProcessSpec {
                    step_id: binding.step_id.clone(),
                    error,
                }
            })?;
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

            // F5 deliberately projects a rich satisfied admission into the F4
            // compatibility token `available_value = 1`. Once F10 and F4 agree
            // that these capability IDs are F5 need identities, any other F4
            // threshold would create a second scalar capability semantics that
            // the rich F5 theorem can never prove.
            for requirement in binding.process_spec.required_capabilities() {
                if requirement.minimum_value != 1 {
                    return Err(ExecutablePlanError::NonBinaryF5BootstrapRequirement {
                        step_id: binding.step_id.clone(),
                        need_id: CapabilityNeedId::new(requirement.capability_id.clone()),
                        minimum_value: requirement.minimum_value,
                    });
                }
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

        capability_needs.sort_by(|left, right| left.id().cmp(right.id()));
        for need in &capability_needs {
            need.validate_canonical().map_err(|error| {
                ExecutablePlanError::NonCanonicalCapabilityNeed {
                    need_id: need.id().clone(),
                    error,
                }
            })?;
        }
        for pair in capability_needs.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(ExecutablePlanError::DuplicateCapabilityNeedBinding(
                    pair[0].id().clone(),
                ));
            }
        }

        let expected_capability_needs = plan
            .steps()
            .iter()
            .flat_map(|step| step.capability_needs().iter().cloned())
            .collect::<BTreeSet<_>>();
        for need_id in &expected_capability_needs {
            if capability_needs
                .binary_search_by(|need| need.id().cmp(need_id))
                .is_err()
            {
                return Err(ExecutablePlanError::MissingCapabilityNeedBinding(
                    need_id.clone(),
                ));
            }
        }
        for need in &capability_needs {
            if !expected_capability_needs.contains(need.id()) {
                return Err(ExecutablePlanError::UnexpectedCapabilityNeedBinding(
                    need.id().clone(),
                ));
            }
        }

        Ok(Self {
            plan,
            process_bindings,
            capability_needs,
        })
    }

    pub fn plan(&self) -> &FabricationPlan {
        &self.plan
    }

    pub fn process_bindings(&self) -> &[ExactPlanProcessBinding] {
        &self.process_bindings
    }

    pub fn capability_needs(&self) -> &[CapabilityNeedSnapshot] {
        &self.capability_needs
    }

    pub fn capability_need(&self, id: &CapabilityNeedId) -> Option<&CapabilityNeedSnapshot> {
        self.capability_needs
            .binary_search_by(|need| need.id().cmp(id))
            .ok()
            .map(|index| &self.capability_needs[index])
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
    capability_needs: Vec<CapabilityNeedSnapshot>,
}

impl<'de> Deserialize<'de> for ExecutableFabricationPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExecutableFabricationPlanWire::deserialize(deserializer)?;
        Self::new_with_capability_needs(wire.plan, wire.process_bindings, wire.capability_needs)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum ExecutablePlanError {
    MissingStepBinding(PlanStepId),
    DuplicateStepBinding(PlanStepId),
    UnknownStepBinding(PlanStepId),
    NonCanonicalProcessSpec {
        step_id: PlanStepId,
        error: ProcessSpecSnapshotError,
    },
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
    NonBinaryF5BootstrapRequirement {
        step_id: PlanStepId,
        need_id: CapabilityNeedId,
        minimum_value: u64,
    },
    MissingCapabilityNeedBinding(CapabilityNeedId),
    DuplicateCapabilityNeedBinding(CapabilityNeedId),
    UnexpectedCapabilityNeedBinding(CapabilityNeedId),
    NonCanonicalCapabilityNeed {
        need_id: CapabilityNeedId,
        error: CapabilityError,
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
            Self::NonCanonicalProcessSpec { step_id, error } => write!(
                formatter,
                "executable plan step {step_id} carries a noncanonical F4 process snapshot: {error}"
            ),
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
            Self::NonBinaryF5BootstrapRequirement {
                step_id,
                need_id,
                minimum_value,
            } => write!(
                formatter,
                "plan step {step_id} maps F5 need {need_id} to non-binary F4 threshold {minimum_value}; exact F5 bootstrap requires 1"
            ),
            Self::MissingCapabilityNeedBinding(need_id) => write!(
                formatter,
                "executable plan lacks exact F5 semantics for required capability need {need_id}"
            ),
            Self::DuplicateCapabilityNeedBinding(need_id) => write!(
                formatter,
                "executable plan repeats exact F5 semantics for capability need {need_id}"
            ),
            Self::UnexpectedCapabilityNeedBinding(need_id) => write!(
                formatter,
                "executable plan carries unreferenced exact F5 capability need {need_id}"
            ),
            Self::NonCanonicalCapabilityNeed { need_id, error } => write!(
                formatter,
                "executable plan capability need {need_id} is not canonical: {error}"
            ),
        }
    }
}

impl Error for ExecutablePlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NonCanonicalProcessSpec { error, .. } => Some(error),
            Self::NonCanonicalCapabilityNeed { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CapabilityNeed, CapabilityNeedId, CapabilityNeedSnapshot, CapabilityRequirement,
        FabricationPlanId, PlanDependency, PlanStep, ProcessKind, ProcessSpec, WorkpieceId,
        WorkpieceLifecycle,
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

    fn capability_needs() -> Vec<CapabilityNeedSnapshot> {
        vec![
            CapabilityNeed::new(
                capability_id("clean"),
                id("capability:clean"),
                None,
                Vec::new(),
                vec![id("condition:a"), id("condition:b")],
            )
            .unwrap()
            .snapshot()
            .unwrap(),
        ]
    }

    fn executable_with(
        process_bindings: Vec<ExactPlanProcessBinding>,
    ) -> Result<ExecutableFabricationPlan, ExecutablePlanError> {
        ExecutableFabricationPlan::new_with_capability_needs(
            plan(),
            process_bindings,
            capability_needs(),
        )
    }

    #[test]
    fn every_plan_step_requires_one_exact_process_binding() {
        let mut incomplete = bindings();
        incomplete.pop();
        let result = executable_with(incomplete);
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
        let result = executable_with(duplicated);
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
        let result = executable_with(values);
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

        let result = executable_with(values);
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

        let result = executable_with(values);
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
        let left = executable_with(bindings()).unwrap();
        let mut altered = bindings();
        altered[1].process_spec = spec(
            "inspect",
            ProcessKind::Inspect,
            None,
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        );
        let right = executable_with(altered).unwrap();

        assert_eq!(left.plan().id, right.plan().id);
        assert_eq!(left.plan().revision, right.plan().revision);
        assert_ne!(left, right);
    }

    #[test]
    fn f5_backed_process_requirement_must_remain_binary_at_f4_bridge() {
        let mut process_bindings = bindings();
        process_bindings[0].process_spec = ProcessSpec::new(
            process_id("clean"),
            1,
            ProcessKind::Clean,
            vec![CapabilityRequirement {
                capability_id: capability_id("clean").stable_id().clone(),
                minimum_value: 2,
            }],
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
        .snapshot();

        let result = ExecutableFabricationPlan::new_with_capability_needs(
            plan(),
            process_bindings,
            capability_needs(),
        );
        assert!(matches!(
            result,
            Err(ExecutablePlanError::NonBinaryF5BootstrapRequirement {
                need_id,
                minimum_value: 2,
                ..
            }) if need_id == capability_id("clean")
        ));
    }

    #[test]
    fn capability_bearing_plan_requires_exact_f5_need_binding() {
        let result = ExecutableFabricationPlan::new(plan(), bindings());
        assert!(matches!(
            result,
            Err(ExecutablePlanError::MissingCapabilityNeedBinding(need_id))
                if need_id == capability_id("clean")
        ));
    }

    #[test]
    fn exact_f5_need_bindings_reject_duplicates_and_extras() {
        let mut duplicated = capability_needs();
        duplicated.push(duplicated[0].clone());
        assert!(matches!(
            ExecutableFabricationPlan::new_with_capability_needs(
                plan(),
                bindings(),
                duplicated,
            ),
            Err(ExecutablePlanError::DuplicateCapabilityNeedBinding(need_id))
                if need_id == capability_id("clean")
        ));

        let mut extra = capability_needs();
        extra.push(
            CapabilityNeed::new(
                capability_id("extra"),
                id("capability:extra"),
                None,
                Vec::new(),
                Vec::new(),
            )
            .unwrap()
            .snapshot()
            .unwrap(),
        );
        assert!(matches!(
            ExecutableFabricationPlan::new_with_capability_needs(plan(), bindings(), extra),
            Err(ExecutablePlanError::UnexpectedCapabilityNeedBinding(need_id))
                if need_id == capability_id("extra")
        ));
    }

    #[test]
    fn same_plan_id_revision_with_changed_f5_semantics_is_structurally_distinct() {
        let left = executable_with(bindings()).unwrap();
        let altered_need = CapabilityNeed::new(
            capability_id("clean"),
            id("capability:clean"),
            Some(id("mode:changed")),
            Vec::new(),
            vec![id("condition:a"), id("condition:b")],
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let right = ExecutableFabricationPlan::new_with_capability_needs(
            plan(),
            bindings(),
            vec![altered_need],
        )
        .unwrap();

        assert_eq!(left.plan().id, right.plan().id);
        assert_eq!(left.plan().revision, right.plan().revision);
        assert_ne!(left, right);
    }

    #[test]
    fn exact_f5_need_wire_rejects_noncanonical_historical_order() {
        let executable = executable_with(bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["capability_needs"][0]["required_conditions"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

    #[test]
    fn exact_f5_need_binding_round_trip_is_queryable_by_need_id() {
        let executable = executable_with(bindings()).unwrap();
        let encoded = serde_json::to_vec(&executable).unwrap();
        let restored: ExecutableFabricationPlan = serde_json::from_slice(&encoded).unwrap();
        let need = restored.capability_need(&capability_id("clean")).unwrap();

        assert_eq!(restored, executable);
        assert_eq!(need.capability_id(), &id("capability:clean"));
        assert_eq!(
            need.required_conditions(),
            &[id("condition:a"), id("condition:b")]
        );
    }

    #[test]
    fn constructor_rejects_snapshot_from_publicly_mutated_process_spec() {
        let mut process = ProcessSpec::new(
            process_id("clean"),
            1,
            ProcessKind::Clean,
            vec![CapabilityRequirement {
                capability_id: capability_id("clean").stable_id().clone(),
                minimum_value: 1,
            }],
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap();
        process.allowed_workpiece_states.reverse();

        let mut values = bindings();
        values[0].process_spec = process.snapshot();
        let result = executable_with(values);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::NonCanonicalProcessSpec {
                error: ProcessSpecSnapshotError::NonCanonicalLifecycleOrder,
                ..
            })
        ));
    }

    #[test]
    fn deserialization_rejects_nested_noncanonical_process_snapshot() {
        let executable = executable_with(bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["process_bindings"][0]["process_spec"]["allowed_workpiece_states"] =
            serde_json::json!(["installed", "available"]);
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

    #[test]
    fn deserialization_rejects_nested_cyclic_f10_plan() {
        let executable = executable_with(bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["plan"]["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "prerequisite": "step:inspect",
                "dependent": "step:clean"
            }));
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

    #[test]
    fn deserialization_rejects_nested_f10_unknown_dependency_step() {
        let executable = executable_with(bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["plan"]["dependencies"][0]["prerequisite"] =
            serde_json::Value::String("step:unknown".into());
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

    #[test]
    fn deserialization_revalidates_executable_plan_contract() {
        let executable = executable_with(bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["process_bindings"][0]["process_spec"]["revision"] = 99.into();

        let restored = serde_json::from_value::<ExecutableFabricationPlan>(value);
        assert!(restored.is_err());
    }

    #[test]
    fn valid_contract_round_trips_and_resolves_exact_process_by_step() {
        let executable = executable_with(bindings()).unwrap();
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
        let executable = executable_with(bindings()).unwrap();
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

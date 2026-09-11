// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Projects Patch Conduit's already-compiled F10/F4/F5 profile into the exact
//! `ExecutableFabricationPlan` contract required by the construction authority.
//!
//! This layer deliberately does not execute construction. It proves that the
//! real Firstlight process catalog—not synthetic unit-test identifiers—can bind
//! every compiled plan step to one exact canonical `ProcessSpecSnapshot`.

use crate::{
    patch_conduit::PatchConduitScenario, patch_conduit_execution::PatchConduitExecutionProfile,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_construction::TemporaryWorkContract;
use symtropy_fabrication::{
    CapabilityError, CapabilityNeedId, ExactPlanProcessBinding, ExecutableFabricationPlan,
    ExecutablePlanError, FunctionalSubject,
};

/// One real Patch Conduit approach with an exact immutable executable-plan
/// contract. Temporary works remain construction orchestration contracts and are
/// checked against the exact plan's authored structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactPatchConduitApproach {
    pub subject: FunctionalSubject,
    pub plan: ExecutableFabricationPlan,
    pub temporary_works: Vec<TemporaryWorkContract>,
}

/// Exact execution-contract projection for every authored Patch Conduit
/// approach. This is the value shape #237 can later hand to `ConstructionSite`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactPatchConduitProfile {
    approaches: Vec<ExactPatchConduitApproach>,
}

impl ExactPatchConduitProfile {
    pub fn compile(scenario: &PatchConduitScenario) -> Result<Self, ExactPatchConduitError> {
        let compiled = PatchConduitExecutionProfile::compile(scenario)
            .map_err(|error| ExactPatchConduitError::Profile(error.to_string()))?;
        Self::from_compiled(&compiled)
    }

    pub fn from_compiled(
        compiled: &PatchConduitExecutionProfile,
    ) -> Result<Self, ExactPatchConduitError> {
        let mut approaches = Vec::with_capacity(compiled.approaches().len());

        for source in compiled.approaches() {
            let mut bindings = Vec::with_capacity(source.plan.steps().len());
            for step in source.plan.steps() {
                let contract = compiled
                    .catalog
                    .process_for_step(&step.id)
                    .ok_or_else(|| ExactPatchConduitError::MissingProcess(step.id.clone()))?;
                bindings.push(ExactPlanProcessBinding::new(
                    step.id.clone(),
                    contract.spec.snapshot(),
                ));
            }

            let capability_need_ids = source
                .plan
                .steps()
                .iter()
                .flat_map(|step| step.capability_needs().iter().cloned())
                .collect::<BTreeSet<_>>();
            let mut capability_needs = Vec::with_capacity(capability_need_ids.len());
            for need_id in capability_need_ids {
                let need = compiled.catalog.capability_need(&need_id).ok_or_else(|| {
                    ExactPatchConduitError::MissingCapabilityNeed(need_id.clone())
                })?;
                capability_needs.push(need.snapshot().map_err(|error| {
                    ExactPatchConduitError::CapabilityNeedSnapshot {
                        need_id: need_id.clone(),
                        error,
                    }
                })?);
            }

            let exact_plan = ExecutableFabricationPlan::new_with_capability_needs(
                source.plan.clone(),
                bindings,
                capability_needs,
            )?;
            for temporary_work in &source.temporary_works {
                temporary_work
                    .validate_plan(exact_plan.plan())
                    .map_err(|error| ExactPatchConduitError::TemporaryWork(error.to_string()))?;
            }

            approaches.push(ExactPatchConduitApproach {
                subject: source.subject.clone(),
                plan: exact_plan,
                temporary_works: source.temporary_works.clone(),
            });
        }

        approaches.sort_by(|left, right| left.subject.id.cmp(&right.subject.id));
        Ok(Self { approaches })
    }

    pub fn approaches(&self) -> &[ExactPatchConduitApproach] {
        &self.approaches
    }

    pub fn approach_for_subject(
        &self,
        subject: &FunctionalSubject,
    ) -> Option<&ExactPatchConduitApproach> {
        self.approaches
            .iter()
            .find(|candidate| &candidate.subject == subject)
    }

    pub fn total_exact_steps(&self) -> usize {
        self.approaches
            .iter()
            .map(|approach| approach.plan.plan().steps().len())
            .sum()
    }
}

#[derive(Debug)]
pub enum ExactPatchConduitError {
    Profile(String),
    MissingProcess(symtropy_fabrication::PlanStepId),
    MissingCapabilityNeed(CapabilityNeedId),
    CapabilityNeedSnapshot {
        need_id: CapabilityNeedId,
        error: CapabilityError,
    },
    ExecutablePlan(ExecutablePlanError),
    TemporaryWork(String),
}

impl From<ExecutablePlanError> for ExactPatchConduitError {
    fn from(error: ExecutablePlanError) -> Self {
        Self::ExecutablePlan(error)
    }
}

impl fmt::Display for ExactPatchConduitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Profile(error) => write!(
                formatter,
                "Patch Conduit profile compilation failed: {error}"
            ),
            Self::MissingProcess(step) => {
                write!(
                    formatter,
                    "Patch Conduit exact plan lacks a process for step {step}"
                )
            }
            Self::MissingCapabilityNeed(need_id) => write!(
                formatter,
                "Patch Conduit exact plan lacks catalog semantics for capability need {need_id}"
            ),
            Self::CapabilityNeedSnapshot { need_id, error } => write!(
                formatter,
                "Patch Conduit capability need {need_id} could not become exact executable semantics: {error}"
            ),
            Self::ExecutablePlan(error) => fmt::Display::fmt(error, formatter),
            Self::TemporaryWork(error) => {
                write!(
                    formatter,
                    "Patch Conduit temporary-work validation failed: {error}"
                )
            }
        }
    }
}

impl Error for ExactPatchConduitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CapabilityNeedSnapshot { error, .. } => Some(error),
            Self::ExecutablePlan(error) => Some(error),
            Self::Profile(_)
            | Self::MissingProcess(_)
            | Self::MissingCapabilityNeed(_)
            | Self::TemporaryWork(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{ProcessKind, WorkpieceLifecycle};
    use symtropy_game_state::StableId;

    fn sid(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn subject(value: &str) -> FunctionalSubject {
        FunctionalSubject {
            kind: symtropy_fabrication::FunctionalSubjectKind::Assembly,
            id: sid(value),
        }
    }

    #[test]
    fn real_patch_conduit_catalog_builds_exact_executable_plans() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let compiled = PatchConduitExecutionProfile::compile(&scenario).unwrap();
        let exact = ExactPatchConduitProfile::from_compiled(&compiled).unwrap();

        assert_eq!(exact.approaches().len(), compiled.approaches().len());
        assert_eq!(
            exact.total_exact_steps(),
            compiled
                .approaches()
                .iter()
                .map(|approach| approach.plan.steps().len())
                .sum::<usize>()
        );

        for approach in exact.approaches() {
            for step in approach.plan.plan().steps() {
                let expected = compiled.catalog.process_for_step(&step.id).unwrap();
                assert_eq!(
                    approach.plan.process_spec(&step.id),
                    Some(&expected.spec.snapshot())
                );
            }
        }
    }

    #[test]
    fn real_patch_conduit_exact_plan_binds_every_f5_need_semantically() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let compiled = PatchConduitExecutionProfile::compile(&scenario).unwrap();
        let exact = ExactPatchConduitProfile::from_compiled(&compiled).unwrap();

        for approach in exact.approaches() {
            for step in approach.plan.plan().steps() {
                for need_id in step.capability_needs() {
                    let expected = compiled
                        .catalog
                        .capability_need(need_id)
                        .unwrap()
                        .snapshot()
                        .unwrap();
                    assert_eq!(approach.plan.capability_need(need_id), Some(&expected));
                }
            }
        }
    }

    #[test]
    fn pressure_test_exact_binding_retains_in_situ_installed_admission() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let exact = ExactPatchConduitProfile::compile(&scenario).unwrap();
        let bypass = exact
            .approach_for_subject(&subject("assembly:patch-conduit:emergency-bypass"))
            .unwrap();
        let step = symtropy_fabrication::PlanStepId::new(sid(
            "step:firstlight:patch-conduit:serviceable-bypass-pressure-test",
        ));
        let spec = bypass.plan.process_spec(&step).unwrap();

        assert_eq!(spec.kind, ProcessKind::PressureTest);
        assert!(spec.admits_workpiece_state(WorkpieceLifecycle::Installed));
        assert_eq!(spec.required_capabilities().len(), 2);
    }

    #[test]
    fn exact_projection_preserves_temporary_work_plan_binding() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let exact = ExactPatchConduitProfile::compile(&scenario).unwrap();

        for approach in exact.approaches() {
            for temporary_work in &approach.temporary_works {
                temporary_work.validate_plan(approach.plan.plan()).unwrap();
            }
        }
    }

    #[test]
    fn exact_profile_contains_no_runtime_success_or_authorization_oracle() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let exact = ExactPatchConduitProfile::compile(&scenario).unwrap();
        let json = serde_json::to_string(&exact).unwrap();

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

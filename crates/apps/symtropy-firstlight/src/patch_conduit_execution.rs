// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable process/capability compilation for the Patch Conduit vertical slice.
//!
//! The discovery scenario owns functional candidates and authored F10 plan shape.
//! This module compiles that shape into the next exact plan revision with:
//! - shared semantic F4 process specifications;
//! - explicit F5 capability-need identities;
//! - unchanged workpiece/evidence/dependency identity;
//! - C5 temporary-work contracts rebound to that exact compiled plan revision.
//!
//! The compiler never claims that a provider capability is currently available,
//! that physical work succeeded, or that the resulting repair is safe/authorized.

use crate::patch_conduit::{PatchConduitApproach, PatchConduitScenario};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_construction::{TemporaryWorkContract, TemporaryWorkError};
use symtropy_fabrication::{
    CapabilityAxisNeed, CapabilityError, CapabilityNeed, CapabilityNeedId, FabricationPlan,
    FunctionalSubject, PlanError, PlanStep, PlanStepId, ProcessError, ProcessKind, ProcessSpec,
    ProcessSpecId, WorkpieceLifecycle,
};
use symtropy_game_state::StableId;

/// Shared executable contract for one physical operation.
///
/// The specification is reusable across multiple plan steps. Capability needs
/// are stable F5 identities rather than skill levels or scalar difficulty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchConduitProcessContract {
    pub semantic_id: StableId,
    pub spec: ProcessSpec,
    capability_need_ids: Vec<CapabilityNeedId>,
}

impl PatchConduitProcessContract {
    pub fn capability_need_ids(&self) -> &[CapabilityNeedId] {
        &self.capability_need_ids
    }
}

/// Canonical F4/F5 knowledge required by the vertical slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchConduitProcessCatalog {
    capability_needs: Vec<CapabilityNeed>,
    processes: Vec<PatchConduitProcessContract>,
}

impl PatchConduitProcessCatalog {
    pub fn canonical() -> Result<Self, PatchConduitExecutionError> {
        // These are deterministic simulation envelopes, not real-world safety
        // limits. Their purpose is to prove multidimensional capability
        // reasoning and preserve the units/axes needed for later tuning.
        let capability_needs = vec![
            need(
                "surface-preparation",
                "capability:surface-preparation",
                vec![axis("axis:surface-area-mm2", 0, 250_000, Some(1_000))?],
                &["condition:surface-accessible"],
            )?,
            need(
                "dimensional-inspection",
                "capability:dimensional-inspection",
                vec![
                    axis("axis:diameter-mm", 20, 400, Some(1))?,
                    axis("axis:ovality-um", 0, 5_000, Some(50))?,
                ],
                &[],
            )?,
            need(
                "brace-clamp-control",
                "capability:clamp-preload-control",
                vec![axis("axis:preload-n", 500, 8_000, Some(50))?],
                &["condition:clamp-accessible"],
            )?,
            need(
                "alignment-control",
                "capability:alignment-control",
                vec![
                    axis("axis:radial-offset-um", -5_000, 5_000, Some(100))?,
                    axis("axis:angular-offset-millirad", -50, 50, Some(1))?,
                ],
                &[],
            )?,
            need(
                "repair-clamp-control",
                "capability:clamp-preload-control",
                vec![axis("axis:preload-n", 1_000, 12_000, Some(25))?],
                &["condition:clamp-accessible"],
            )?,
            need(
                "seal-installation",
                "capability:seal-installation",
                vec![axis("axis:contact-pressure-kpa", 100, 3_000, Some(25))?],
                &["condition:surface-clean"],
            )?,
            need(
                "hydrostatic-pressure-source",
                "capability:hydrostatic-pressure-source",
                vec![axis("axis:pressure-kpa", 0, 1_000, Some(10))?],
                &["condition:service-line-isolated"],
            )?,
            need(
                "pressure-measurement",
                "capability:pressure-measurement",
                vec![
                    axis("axis:pressure-kpa", 0, 1_000, Some(5))?,
                    axis("axis:hold-duration-s", 0, 600, Some(1))?,
                ],
                &[],
            )?,
            need(
                "controlled-clamp-release",
                "capability:controlled-clamp-release",
                vec![axis("axis:preload-n", 0, 8_000, Some(50))?],
                &["condition:clamp-accessible"],
            )?,
            need(
                "fluid-coupling-control",
                "capability:fluid-coupling-control",
                vec![axis("axis:coupling-torque-cnm", 200, 6_000, Some(25))?],
                &["condition:coupling-accessible"],
            )?,
        ];
        reject_duplicate_needs(&capability_needs)?;

        let processes = vec![
            process(
                "surface-clean",
                ProcessKind::Clean,
                &["surface-preparation"],
                vec![
                    WorkpieceLifecycle::Staged,
                    WorkpieceLifecycle::Available,
                    WorkpieceLifecycle::Installed,
                    WorkpieceLifecycle::Removed,
                ],
                &capability_needs,
            )?,
            process(
                "component-inspect",
                ProcessKind::Inspect,
                &["dimensional-inspection"],
                vec![
                    WorkpieceLifecycle::Staged,
                    WorkpieceLifecycle::Available,
                    WorkpieceLifecycle::Removed,
                ],
                &capability_needs,
            )?,
            process(
                "brace-install",
                ProcessKind::Clamp,
                &["brace-clamp-control"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
            process(
                "alignment",
                ProcessKind::Align,
                &["alignment-control"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
            process(
                "repair-clamp",
                ProcessKind::Clamp,
                &["repair-clamp-control"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
            process(
                "water-seal",
                ProcessKind::Seal,
                &["seal-installation"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
            process(
                "water-pressure-test",
                ProcessKind::PressureTest,
                &["hydrostatic-pressure-source", "pressure-measurement"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
            process(
                "brace-release",
                ProcessKind::ReleaseClamp,
                &["controlled-clamp-release"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
            process(
                "temporary-fluid-couple",
                ProcessKind::Couple,
                &["fluid-coupling-control"],
                vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
                &capability_needs,
            )?,
        ];
        reject_duplicate_processes(&processes)?;

        Ok(Self {
            capability_needs,
            processes,
        })
    }

    pub fn capability_needs(&self) -> &[CapabilityNeed] {
        &self.capability_needs
    }

    pub fn processes(&self) -> &[PatchConduitProcessContract] {
        &self.processes
    }

    pub fn capability_need(&self, id: &CapabilityNeedId) -> Option<&CapabilityNeed> {
        self.capability_needs
            .iter()
            .find(|candidate| &candidate.id == id)
    }

    pub fn process_for_step(&self, step_id: &PlanStepId) -> Option<&PatchConduitProcessContract> {
        process_key_for_step(step_id).and_then(|key| self.process_by_key(key))
    }

    fn process_by_key(&self, key: &str) -> Option<&PatchConduitProcessContract> {
        let id = process_spec_id(key);
        self.processes
            .iter()
            .find(|candidate| candidate.spec.id == id)
    }

    fn validate_executable_plan(
        &self,
        plan: &FabricationPlan,
    ) -> Result<(), PatchConduitExecutionError> {
        for step in plan.steps() {
            let contract = self
                .process_for_step(&step.id)
                .ok_or_else(|| PatchConduitExecutionError::UnmappedStep(step.id.clone()))?;

            if step.process_spec_id != contract.spec.id
                || step.process_spec_revision != contract.spec.revision
            {
                return Err(PatchConduitExecutionError::ProcessBindingMismatch {
                    step_id: step.id.clone(),
                    expected_id: contract.spec.id.clone(),
                    expected_revision: contract.spec.revision,
                    actual_id: step.process_spec_id.clone(),
                    actual_revision: step.process_spec_revision,
                });
            }

            let expected = contract
                .capability_need_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            let actual = step
                .capability_needs()
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            if expected != actual {
                return Err(PatchConduitExecutionError::CapabilityBindingMismatch {
                    step_id: step.id.clone(),
                    expected: expected.into_iter().collect(),
                    actual: actual.into_iter().collect(),
                });
            }

            for need_id in &contract.capability_need_ids {
                if self.capability_need(need_id).is_none() {
                    return Err(PatchConduitExecutionError::UnknownCapabilityNeed(
                        need_id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// One compiled approach ready to be consumed by C3/C2/C1/F4 later.
///
/// "Executable" here means every planned step has an exact process/capability
/// contract. It does not mean providers, material placement, or authority have
/// been admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutablePatchConduitApproach {
    pub subject: FunctionalSubject,
    pub plan: FabricationPlan,
    pub temporary_works: Vec<TemporaryWorkContract>,
}

/// Deterministic compilation of Patch Conduit authored plan shape into exact
/// F4/F5/C5 contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchConduitExecutionProfile {
    pub catalog: PatchConduitProcessCatalog,
    approaches: Vec<ExecutablePatchConduitApproach>,
}

impl PatchConduitExecutionProfile {
    pub fn compile(scenario: &PatchConduitScenario) -> Result<Self, PatchConduitExecutionError> {
        let catalog = PatchConduitProcessCatalog::canonical()?;
        let mut approaches = Vec::with_capacity(scenario.approaches().len());
        for source in scenario.approaches() {
            approaches.push(compile_approach(&catalog, source)?);
        }

        Ok(Self {
            catalog,
            approaches,
        })
    }

    pub fn approaches(&self) -> &[ExecutablePatchConduitApproach] {
        &self.approaches
    }

    pub fn approach_for_subject(
        &self,
        subject: &FunctionalSubject,
    ) -> Option<&ExecutablePatchConduitApproach> {
        self.approaches
            .iter()
            .find(|candidate| &candidate.subject == subject)
    }
}

fn compile_approach(
    catalog: &PatchConduitProcessCatalog,
    source: &PatchConduitApproach,
) -> Result<ExecutablePatchConduitApproach, PatchConduitExecutionError> {
    let revision =
        source.plan.revision.checked_add(1).ok_or_else(|| {
            PatchConduitExecutionError::PlanRevisionOverflow(source.plan.id.clone())
        })?;

    let mut compiled_steps = Vec::with_capacity(source.plan.steps().len());
    for source_step in source.plan.steps() {
        let contract = catalog
            .process_for_step(&source_step.id)
            .ok_or_else(|| PatchConduitExecutionError::UnmappedStep(source_step.id.clone()))?;

        compiled_steps.push(PlanStep::new(
            source_step.id.clone(),
            contract.spec.id.clone(),
            contract.spec.revision,
            source_step.workpieces().to_vec(),
            contract.capability_need_ids.clone(),
            source_step.expected_evidence_kinds().to_vec(),
        )?);
    }

    let plan = FabricationPlan::new(
        source.plan.id.clone(),
        revision,
        compiled_steps,
        source.plan.dependencies().to_vec(),
    )?;
    catalog.validate_executable_plan(&plan)?;

    let mut temporary_works = Vec::with_capacity(source.temporary_works.len());
    for source_work in &source.temporary_works {
        temporary_works.push(TemporaryWorkContract::new(
            source_work.id.clone(),
            &plan,
            source_work.kind,
            source_work.purpose_id.clone(),
            source_work.temporary_workpieces().to_vec(),
            source_work.installation_step_id.clone(),
            source_work.protected_step_ids().to_vec(),
            source_work.removal_step_id.clone(),
        )?);
    }

    Ok(ExecutablePatchConduitApproach {
        subject: source.subject.clone(),
        plan,
        temporary_works,
    })
}

fn process(
    key: &str,
    kind: ProcessKind,
    need_keys: &[&str],
    allowed_workpiece_states: Vec<WorkpieceLifecycle>,
    needs: &[CapabilityNeed],
) -> Result<PatchConduitProcessContract, PatchConduitExecutionError> {
    let mut capability_need_ids = Vec::with_capacity(need_keys.len());
    let mut bootstrap_requirements = Vec::with_capacity(need_keys.len());
    for key in need_keys {
        let id = capability_need_id(key);
        let need = needs
            .iter()
            .find(|candidate| candidate.id == id)
            .ok_or_else(|| PatchConduitExecutionError::UnknownCapabilityNeed(id.clone()))?;
        capability_need_ids.push(id);
        bootstrap_requirements.push(need.bootstrap_requirement());
    }

    Ok(PatchConduitProcessContract {
        semantic_id: sid(format!("process-semantic:firstlight:{key}")),
        spec: ProcessSpec::new(
            process_spec_id(key),
            1,
            kind,
            bootstrap_requirements,
            allowed_workpiece_states,
        )?,
        capability_need_ids,
    })
}

fn need(
    key: &str,
    capability_id: &str,
    axes: Vec<CapabilityAxisNeed>,
    conditions: &[&str],
) -> Result<CapabilityNeed, CapabilityError> {
    CapabilityNeed::new(
        capability_need_id(key),
        sid(capability_id),
        None,
        axes,
        conditions.iter().map(|value| sid(*value)).collect(),
    )
}

fn axis(
    id: &str,
    lower: i64,
    upper: i64,
    max_resolution: Option<u64>,
) -> Result<CapabilityAxisNeed, CapabilityError> {
    CapabilityAxisNeed::new(sid(id), lower, upper, max_resolution)
}

fn process_spec_id(key: &str) -> ProcessSpecId {
    ProcessSpecId::new(sid(format!("process-spec:firstlight:field:{key}")))
}

fn capability_need_id(key: &str) -> CapabilityNeedId {
    CapabilityNeedId::new(sid(format!("capability-need:firstlight:field:{key}")))
}

fn process_key_for_step(step_id: &PlanStepId) -> Option<&'static str> {
    match step_id.stable_id().as_str() {
        "step:firstlight:patch-conduit:standard-clean"
        | "step:firstlight:patch-conduit:salvage-clean" => Some("surface-clean"),

        "step:firstlight:patch-conduit:salvage-inspect" => Some("component-inspect"),

        "step:firstlight:patch-conduit:standard-install-brace"
        | "step:firstlight:patch-conduit:salvage-install-brace" => Some("brace-install"),

        "step:firstlight:patch-conduit:standard-align"
        | "step:firstlight:patch-conduit:salvage-align" => Some("alignment"),

        "step:firstlight:patch-conduit:standard-clamp"
        | "step:firstlight:patch-conduit:salvage-clamp" => Some("repair-clamp"),

        "step:firstlight:patch-conduit:standard-seal"
        | "step:firstlight:patch-conduit:salvage-seal" => Some("water-seal"),

        "step:firstlight:patch-conduit:standard-pressure-test"
        | "step:firstlight:patch-conduit:salvage-pressure-test"
        | "step:firstlight:patch-conduit:serviceable-bypass-pressure-test"
        | "step:firstlight:patch-conduit:degraded-bypass-pressure-test" => {
            Some("water-pressure-test")
        }

        "step:firstlight:patch-conduit:standard-remove-brace"
        | "step:firstlight:patch-conduit:salvage-remove-brace" => Some("brace-release"),

        "step:firstlight:patch-conduit:serviceable-bypass-install"
        | "step:firstlight:patch-conduit:degraded-bypass-install" => Some("temporary-fluid-couple"),

        _ => None,
    }
}

fn reject_duplicate_needs(needs: &[CapabilityNeed]) -> Result<(), PatchConduitExecutionError> {
    let mut seen = BTreeSet::new();
    for need in needs {
        if !seen.insert(need.id.clone()) {
            return Err(PatchConduitExecutionError::DuplicateCapabilityNeed(
                need.id.clone(),
            ));
        }
    }
    Ok(())
}

fn reject_duplicate_processes(
    processes: &[PatchConduitProcessContract],
) -> Result<(), PatchConduitExecutionError> {
    let mut seen = BTreeSet::new();
    for process in processes {
        if !seen.insert(process.spec.id.clone()) {
            return Err(PatchConduitExecutionError::DuplicateProcessSpec(
                process.spec.id.clone(),
            ));
        }
    }
    Ok(())
}

fn sid(value: impl Into<String>) -> StableId {
    StableId::parse(value).expect("Patch Conduit execution identifiers must be valid")
}

#[derive(Debug)]
pub enum PatchConduitExecutionError {
    Capability(CapabilityError),
    Process(ProcessError),
    Plan(PlanError),
    TemporaryWork(TemporaryWorkError),
    DuplicateCapabilityNeed(CapabilityNeedId),
    DuplicateProcessSpec(ProcessSpecId),
    UnknownCapabilityNeed(CapabilityNeedId),
    UnmappedStep(PlanStepId),
    PlanRevisionOverflow(symtropy_fabrication::FabricationPlanId),
    ProcessBindingMismatch {
        step_id: PlanStepId,
        expected_id: ProcessSpecId,
        expected_revision: u64,
        actual_id: ProcessSpecId,
        actual_revision: u64,
    },
    CapabilityBindingMismatch {
        step_id: PlanStepId,
        expected: Vec<CapabilityNeedId>,
        actual: Vec<CapabilityNeedId>,
    },
}

impl From<CapabilityError> for PatchConduitExecutionError {
    fn from(error: CapabilityError) -> Self {
        Self::Capability(error)
    }
}

impl From<ProcessError> for PatchConduitExecutionError {
    fn from(error: ProcessError) -> Self {
        Self::Process(error)
    }
}

impl From<PlanError> for PatchConduitExecutionError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<TemporaryWorkError> for PatchConduitExecutionError {
    fn from(error: TemporaryWorkError) -> Self {
        Self::TemporaryWork(error)
    }
}

impl fmt::Display for PatchConduitExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capability(error) => {
                write!(
                    formatter,
                    "Patch Conduit capability contract failed: {error}"
                )
            }
            Self::Process(error) => {
                write!(formatter, "Patch Conduit process contract failed: {error}")
            }
            Self::Plan(error) => write!(formatter, "Patch Conduit executable plan failed: {error}"),
            Self::TemporaryWork(error) => write!(
                formatter,
                "Patch Conduit temporary-work rebinding failed: {error}"
            ),
            Self::DuplicateCapabilityNeed(id) => {
                write!(
                    formatter,
                    "Patch Conduit capability need {id} is duplicated"
                )
            }
            Self::DuplicateProcessSpec(id) => {
                write!(formatter, "Patch Conduit process spec {id} is duplicated")
            }
            Self::UnknownCapabilityNeed(id) => write!(
                formatter,
                "Patch Conduit process references unknown capability need {id}"
            ),
            Self::UnmappedStep(id) => write!(
                formatter,
                "Patch Conduit plan step {id} has no executable process contract"
            ),
            Self::PlanRevisionOverflow(id) => {
                write!(
                    formatter,
                    "Patch Conduit plan {id} cannot advance its revision"
                )
            }
            Self::ProcessBindingMismatch {
                step_id,
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "Patch Conduit step {step_id} expected process {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::CapabilityBindingMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "Patch Conduit step {step_id} capability bindings differ: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl Error for PatchConduitExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Capability(error) => Some(error),
            Self::Process(error) => Some(error),
            Self::Plan(error) => Some(error),
            Self::TemporaryWork(error) => Some(error),
            Self::DuplicateCapabilityNeed(_)
            | Self::DuplicateProcessSpec(_)
            | Self::UnknownCapabilityNeed(_)
            | Self::UnmappedStep(_)
            | Self::PlanRevisionOverflow(_)
            | Self::ProcessBindingMismatch { .. }
            | Self::CapabilityBindingMismatch { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{
        CapabilityAdmissionId, CapabilityAxisRange, CapabilityEnvelope, CapabilityEvidenceRef,
        CapabilityOutcome,
    };

    fn plan_step(name: &str) -> PlanStepId {
        PlanStepId::new(sid(format!("step:firstlight:patch-conduit:{name}")))
    }

    fn approach<'a>(
        profile: &'a PatchConduitExecutionProfile,
        subject: &str,
    ) -> &'a ExecutablePatchConduitApproach {
        profile
            .approaches()
            .iter()
            .find(|candidate| candidate.subject.id == sid(subject))
            .unwrap()
    }

    #[test]
    fn compilation_advances_plan_revision_and_rebinds_temporary_works() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let profile = PatchConduitExecutionProfile::compile(&scenario).unwrap();

        assert_eq!(profile.approaches().len(), scenario.approaches().len());
        for executable in profile.approaches() {
            let source = scenario
                .approaches()
                .iter()
                .find(|candidate| candidate.subject == executable.subject)
                .unwrap();
            assert_eq!(executable.plan.id, source.plan.id);
            assert_eq!(executable.plan.revision, source.plan.revision + 1);
            assert_eq!(
                executable.temporary_works.len(),
                source.temporary_works.len()
            );
            for contract in &executable.temporary_works {
                assert_eq!(contract.plan_id, executable.plan.id);
                assert_eq!(contract.plan_revision, executable.plan.revision);
                contract.validate_plan(&executable.plan).unwrap();
            }
        }
    }

    #[test]
    fn shared_physical_work_uses_shared_process_specs() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let profile = PatchConduitExecutionProfile::compile(&scenario).unwrap();
        let standard = approach(&profile, "assembly:patch-conduit:standard-banded");
        let salvaged = approach(&profile, "assembly:patch-conduit:salvaged-sleeve");

        assert_eq!(
            standard
                .plan
                .step(&plan_step("standard-clean"))
                .unwrap()
                .process_spec_id,
            salvaged
                .plan
                .step(&plan_step("salvage-clean"))
                .unwrap()
                .process_spec_id
        );
        assert_eq!(
            standard
                .plan
                .step(&plan_step("standard-pressure-test"))
                .unwrap()
                .process_spec_id,
            salvaged
                .plan
                .step(&plan_step("salvage-pressure-test"))
                .unwrap()
                .process_spec_id
        );
    }

    #[test]
    fn brace_removal_is_real_non_destructive_release_work() {
        let catalog = PatchConduitProcessCatalog::canonical().unwrap();
        let contract = catalog
            .process_for_step(&plan_step("standard-remove-brace"))
            .unwrap();
        assert_eq!(contract.spec.kind, ProcessKind::ReleaseClamp);
        assert_eq!(
            contract.spec.kind.separation_semantics(),
            Some(symtropy_fabrication::SeparationSemantics::NonDestructive)
        );
    }

    #[test]
    fn bypass_install_uses_explicit_coupling_not_splice_or_terminate() {
        let catalog = PatchConduitProcessCatalog::canonical().unwrap();
        for step in [
            plan_step("serviceable-bypass-install"),
            plan_step("degraded-bypass-install"),
        ] {
            let contract = catalog.process_for_step(&step).unwrap();
            assert_eq!(contract.spec.kind, ProcessKind::Couple);
            assert_eq!(contract.spec.kind.separation_semantics(), None);
        }
    }

    #[test]
    fn pressure_test_requires_independent_source_and_measurement_needs() {
        let catalog = PatchConduitProcessCatalog::canonical().unwrap();
        let contract = catalog
            .process_for_step(&plan_step("standard-pressure-test"))
            .unwrap();

        assert_eq!(contract.spec.kind, ProcessKind::PressureTest);
        assert_eq!(contract.capability_need_ids().len(), 2);
        assert!(
            contract
                .capability_need_ids()
                .contains(&capability_need_id("hydrostatic-pressure-source"))
        );
        assert!(
            contract
                .capability_need_ids()
                .contains(&capability_need_id("pressure-measurement"))
        );
        assert_eq!(contract.spec.required_capabilities.len(), 2);
    }

    #[test]
    fn pressure_measurement_is_range_and_resolution_not_a_tool_level() {
        let catalog = PatchConduitProcessCatalog::canonical().unwrap();
        let need = catalog
            .capability_need(&capability_need_id("pressure-measurement"))
            .unwrap();

        let evidence = CapabilityEvidenceRef::new(
            sid("authority:firstlight:tool-calibration"),
            sid("evidence:firstlight:gauge:good"),
            3,
            "digest:firstlight:gauge:good:3",
        )
        .unwrap();
        let adequate = CapabilityEnvelope::new(
            sid("provider:firstlight:field-gauge"),
            7,
            sid("capability:pressure-measurement"),
            sid("mode:gauge:hydrostatic"),
            vec![
                CapabilityAxisRange::new(sid("axis:pressure-kpa"), 0, 1_200, 5).unwrap(),
                CapabilityAxisRange::new(sid("axis:hold-duration-s"), 0, 900, 1).unwrap(),
            ],
            vec![],
            evidence,
        )
        .unwrap();
        let admission = need.evaluate(
            CapabilityAdmissionId::new(sid("admission:gauge:good")),
            &adequate,
        );
        assert_eq!(admission.outcome, CapabilityOutcome::Satisfied);

        let coarse = CapabilityEnvelope::new(
            sid("provider:firstlight:coarse-gauge"),
            1,
            sid("capability:pressure-measurement"),
            sid("mode:gauge:hydrostatic"),
            vec![
                CapabilityAxisRange::new(sid("axis:pressure-kpa"), 0, 1_200, 25).unwrap(),
                CapabilityAxisRange::new(sid("axis:hold-duration-s"), 0, 900, 1).unwrap(),
            ],
            vec![],
            CapabilityEvidenceRef::new(
                sid("authority:firstlight:tool-calibration"),
                sid("evidence:firstlight:gauge:coarse"),
                1,
                "digest:firstlight:gauge:coarse:1",
            )
            .unwrap(),
        )
        .unwrap();
        let admission = need.evaluate(
            CapabilityAdmissionId::new(sid("admission:gauge:coarse")),
            &coarse,
        );
        assert_eq!(admission.outcome, CapabilityOutcome::Unsatisfied);
        assert!(admission.failures.iter().any(|failure| matches!(
            failure,
            symtropy_fabrication::CapabilityMismatch::Resolution { axis_id, .. }
                if axis_id == &sid("axis:pressure-kpa")
        )));
    }

    #[test]
    fn compiled_profile_contains_no_scalar_quality_or_authority_oracle() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let profile = PatchConduitExecutionProfile::compile(&scenario).unwrap();
        let json = serde_json::to_string(&profile).unwrap();
        for forbidden in [
            "\"quality\"",
            "\"score\"",
            "\"level\"",
            "\"difficulty\"",
            "\"authorized\"",
            "\"commissioned\"",
            "\"safe\"",
            "\"success\"",
        ] {
            assert!(!json.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}

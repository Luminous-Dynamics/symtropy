// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Construction-site execution ledger over fabrication plans and process evidence.
//!
//! A site binds orchestration to one exact fabrication-plan revision. Planned
//! work becomes completed site work only after a pre-admitted process execution
//! returns matching `ProcessEvidence`. The site never claims that planned work
//! is technically commissioned, structurally safe, civically authorized, or
//! equivalent to a generic progress percentage.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_fabrication::{
    FabricationPlan, FabricationPlanId, PlanError, PlanStepId, ProcessEvidence,
    ProcessExecutionId, ProcessExecutionState, ProcessSpecId, WorkpieceId,
};
use symtropy_game_state::StableId;

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(StableId);

        impl $name {
            pub const fn new(id: StableId) -> Self {
                Self(id)
            }

            pub const fn stable_id(&self) -> &StableId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id_type!(ConstructionSiteId);
stable_id_type!(SiteStepAdmissionId);

/// Site lifecycle is orchestration state only. `PlanClosed` means every planned
/// fabrication step has evidence-backed completion; it is not technical
/// commissioning, structural fitness, Device Bus registration, or permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionSiteLifecycle {
    Open,
    Suspended,
    PlanClosed,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteStepAdmissionState {
    Open,
    Cancelled,
    Completed,
}

/// Minimal durable reference to the process evidence that closed one admitted
/// site step. The fabrication domain remains authoritative for the full process
/// record and conserved-matter transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteProcessEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
    pub execution_id: ProcessExecutionId,
    pub process_spec_id: ProcessSpecId,
    pub process_spec_revision: u64,
}

impl SiteProcessEvidenceRef {
    fn from_process(evidence: &ProcessEvidence) -> Self {
        Self {
            authority_id: evidence.authority_id.clone(),
            evidence_id: evidence.evidence_id.clone(),
            revision: evidence.revision,
            digest: evidence.digest.clone(),
            execution_id: evidence.execution_id.clone(),
            process_spec_id: evidence.spec_id.clone(),
            process_spec_revision: evidence.spec_revision,
        }
    }
}

/// Pre-execution site admission binds one exact process execution identity to a
/// plan step before physical work is accepted for that step. This prevents
/// arbitrary old process evidence from another context from silently closing a
/// construction step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteStepAdmission {
    pub id: SiteStepAdmissionId,
    pub step_id: PlanStepId,
    pub execution_id: ProcessExecutionId,
    pub state: SiteStepAdmissionState,
    pub cancellation_reason_id: Option<StableId>,
    pub completion: Option<SiteProcessEvidenceRef>,
}

/// Persistent site orchestration state. Physical and engineering state remain
/// external; this ledger records which exact planned processes were admitted
/// and which returned matching evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionSite {
    pub id: ConstructionSiteId,
    pub revision: u64,
    pub plan_id: FabricationPlanId,
    pub plan_revision: u64,
    pub lifecycle: ConstructionSiteLifecycle,
    admissions: Vec<SiteStepAdmission>,
}

impl ConstructionSite {
    pub fn new(id: ConstructionSiteId, plan: &FabricationPlan) -> Self {
        Self {
            id,
            revision: 1,
            plan_id: plan.id.clone(),
            plan_revision: plan.revision,
            lifecycle: ConstructionSiteLifecycle::Open,
            admissions: Vec::new(),
        }
    }

    pub fn admissions(&self) -> &[SiteStepAdmission] {
        &self.admissions
    }

    pub fn completed_step_ids(&self) -> Vec<PlanStepId> {
        self.admissions
            .iter()
            .filter(|admission| admission.state == SiteStepAdmissionState::Completed)
            .map(|admission| admission.step_id.clone())
            .collect()
    }

    /// Returns steps whose plan prerequisites are completed and which do not
    /// already have an open execution admission.
    pub fn ready_steps(&self, plan: &FabricationPlan) -> Result<Vec<PlanStepId>, SiteError> {
        self.validate_plan(plan)?;
        if matches!(
            self.lifecycle,
            ConstructionSiteLifecycle::PlanClosed | ConstructionSiteLifecycle::Abandoned
        ) {
            return Ok(Vec::new());
        }

        let completed = self.completed_step_ids();
        let mut ready = plan.ready_steps(&completed)?;
        ready.retain(|step_id| {
            !self.admissions.iter().any(|admission| {
                admission.step_id == *step_id && admission.state == SiteStepAdmissionState::Open
            })
        });
        Ok(ready)
    }

    /// Pre-admits one exact process execution for a currently ready plan step.
    /// The execution identity becomes the context binding used at completion.
    pub fn admit_step(
        &mut self,
        plan: &FabricationPlan,
        admission_id: SiteStepAdmissionId,
        step_id: PlanStepId,
        execution_id: ProcessExecutionId,
    ) -> Result<(), SiteError> {
        self.validate_plan(plan)?;
        if self.lifecycle != ConstructionSiteLifecycle::Open {
            return Err(SiteError::SiteNotOpen(self.lifecycle));
        }
        if self
            .admissions
            .iter()
            .any(|admission| admission.id == admission_id)
        {
            return Err(SiteError::DuplicateAdmissionId(admission_id));
        }
        if self
            .admissions
            .iter()
            .any(|admission| admission.execution_id == execution_id)
        {
            return Err(SiteError::DuplicateExecutionId(execution_id));
        }
        if self.admissions.iter().any(|admission| {
            admission.step_id == step_id && admission.state == SiteStepAdmissionState::Completed
        }) {
            return Err(SiteError::StepAlreadyCompleted(step_id));
        }
        if self.admissions.iter().any(|admission| {
            admission.step_id == step_id && admission.state == SiteStepAdmissionState::Open
        }) {
            return Err(SiteError::StepAlreadyAdmitted(step_id));
        }

        if plan.step(&step_id).is_none() {
            return Err(SiteError::UnknownPlanStep(step_id));
        }
        if !self.ready_steps(plan)?.contains(&step_id) {
            return Err(SiteError::StepNotReady(step_id));
        }

        self.admissions.push(SiteStepAdmission {
            id: admission_id,
            step_id,
            execution_id,
            state: SiteStepAdmissionState::Open,
            cancellation_reason_id: None,
            completion: None,
        });
        self.admissions.sort_by(|left, right| left.id.cmp(&right.id));
        self.bump_revision()?;
        Ok(())
    }

    /// Cancels an orchestration admission without claiming physical rollback.
    /// A later retry must use a fresh admission and process-execution identity.
    pub fn cancel_admission(
        &mut self,
        admission_id: &SiteStepAdmissionId,
        reason_id: StableId,
    ) -> Result<(), SiteError> {
        self.ensure_mutable()?;
        let admission = self
            .admissions
            .iter_mut()
            .find(|admission| &admission.id == admission_id)
            .ok_or_else(|| SiteError::UnknownAdmission(admission_id.clone()))?;
        if admission.state != SiteStepAdmissionState::Open {
            return Err(SiteError::AdmissionNotOpen(admission_id.clone()));
        }
        admission.state = SiteStepAdmissionState::Cancelled;
        admission.cancellation_reason_id = Some(reason_id);
        self.bump_revision()?;
        Ok(())
    }

    /// Accepts completion only for a process execution that was admitted for
    /// this site/step beforehand and whose exact spec/workpiece contract matches
    /// the immutable F10 plan step.
    pub fn record_completion(
        &mut self,
        plan: &FabricationPlan,
        evidence: &ProcessEvidence,
    ) -> Result<PlanStepId, SiteError> {
        self.validate_plan(plan)?;
        self.ensure_mutable()?;

        let admission_index = self
            .admissions
            .iter()
            .position(|admission| admission.execution_id == evidence.execution_id)
            .ok_or_else(|| SiteError::UnknownExecutionAdmission(evidence.execution_id.clone()))?;

        if self.admissions[admission_index].state != SiteStepAdmissionState::Open {
            return Err(SiteError::AdmissionNotOpen(
                self.admissions[admission_index].id.clone(),
            ));
        }

        if evidence.outcome != ProcessExecutionState::Completed {
            return Err(SiteError::ProcessNotCompleted(evidence.execution_id.clone()));
        }
        if evidence.digest.is_empty() || evidence.digest.len() > 256 {
            return Err(SiteError::InvalidProcessEvidenceDigest(evidence.digest.clone()));
        }
        if evidence.resulting_matter.is_empty() {
            return Err(SiteError::MissingResultingMatterEvidence(
                evidence.execution_id.clone(),
            ));
        }
        if self.admissions.iter().any(|admission| {
            admission.completion.as_ref().is_some_and(|existing| {
                existing.authority_id == evidence.authority_id
                    && existing.evidence_id == evidence.evidence_id
            })
        }) {
            return Err(SiteError::DuplicateProcessEvidence {
                authority_id: evidence.authority_id.clone(),
                evidence_id: evidence.evidence_id.clone(),
            });
        }

        let step_id = self.admissions[admission_index].step_id.clone();
        let step = plan
            .step(&step_id)
            .ok_or_else(|| SiteError::UnknownPlanStep(step_id.clone()))?;

        if evidence.spec_id != step.process_spec_id
            || evidence.spec_revision != step.process_spec_revision
        {
            return Err(SiteError::ProcessSpecMismatch {
                step_id,
                expected_id: step.process_spec_id.clone(),
                expected_revision: step.process_spec_revision,
                actual_id: evidence.spec_id.clone(),
                actual_revision: evidence.spec_revision,
            });
        }

        let expected_workpieces = step.workpieces().to_vec();
        let mut actual_workpieces = evidence
            .inputs
            .iter()
            .map(|input| input.workpiece_id.clone())
            .collect::<Vec<_>>();
        actual_workpieces.sort();
        if actual_workpieces != expected_workpieces {
            return Err(SiteError::WorkpieceSetMismatch {
                step_id: self.admissions[admission_index].step_id.clone(),
                expected: expected_workpieces,
                actual: actual_workpieces,
            });
        }

        // Re-evaluate readiness at acceptance time. A corrupted/restored site
        // snapshot cannot bypass dependency ordering merely because an admission
        // object exists.
        let completed = self.completed_step_ids();
        if !plan.ready_steps(&completed)?.contains(&self.admissions[admission_index].step_id) {
            return Err(SiteError::StepNotReady(
                self.admissions[admission_index].step_id.clone(),
            ));
        }

        let completion = SiteProcessEvidenceRef::from_process(evidence);
        self.admissions[admission_index].state = SiteStepAdmissionState::Completed;
        self.admissions[admission_index].completion = Some(completion);
        self.bump_revision()?;
        Ok(self.admissions[admission_index].step_id.clone())
    }

    pub fn plan_complete(&self, plan: &FabricationPlan) -> Result<bool, SiteError> {
        self.validate_plan(plan)?;
        Ok(self.completed_step_ids().len() == plan.steps().len())
    }

    pub fn suspend(&mut self) -> Result<(), SiteError> {
        if self.lifecycle != ConstructionSiteLifecycle::Open {
            return Err(SiteError::InvalidLifecycleTransition {
                from: self.lifecycle,
                to: ConstructionSiteLifecycle::Suspended,
            });
        }
        self.lifecycle = ConstructionSiteLifecycle::Suspended;
        self.bump_revision()?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), SiteError> {
        if self.lifecycle != ConstructionSiteLifecycle::Suspended {
            return Err(SiteError::InvalidLifecycleTransition {
                from: self.lifecycle,
                to: ConstructionSiteLifecycle::Open,
            });
        }
        self.lifecycle = ConstructionSiteLifecycle::Open;
        self.bump_revision()?;
        Ok(())
    }

    /// Closes the *planned work ledger* only after every plan step has matching
    /// completion evidence. This still does not imply technical commissioning.
    pub fn close_plan(&mut self, plan: &FabricationPlan) -> Result<(), SiteError> {
        self.validate_plan(plan)?;
        if self.lifecycle != ConstructionSiteLifecycle::Open
            && self.lifecycle != ConstructionSiteLifecycle::Suspended
        {
            return Err(SiteError::InvalidLifecycleTransition {
                from: self.lifecycle,
                to: ConstructionSiteLifecycle::PlanClosed,
            });
        }
        if !self.plan_complete(plan)? {
            return Err(SiteError::PlanIncomplete);
        }
        self.lifecycle = ConstructionSiteLifecycle::PlanClosed;
        self.bump_revision()?;
        Ok(())
    }

    pub fn abandon(&mut self) -> Result<(), SiteError> {
        if matches!(
            self.lifecycle,
            ConstructionSiteLifecycle::PlanClosed | ConstructionSiteLifecycle::Abandoned
        ) {
            return Err(SiteError::InvalidLifecycleTransition {
                from: self.lifecycle,
                to: ConstructionSiteLifecycle::Abandoned,
            });
        }
        self.lifecycle = ConstructionSiteLifecycle::Abandoned;
        self.bump_revision()?;
        Ok(())
    }

    fn validate_plan(&self, plan: &FabricationPlan) -> Result<(), SiteError> {
        if plan.id != self.plan_id || plan.revision != self.plan_revision {
            return Err(SiteError::PlanMismatch {
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: plan.id.clone(),
                actual_revision: plan.revision,
            });
        }
        Ok(())
    }

    fn ensure_mutable(&self) -> Result<(), SiteError> {
        if matches!(
            self.lifecycle,
            ConstructionSiteLifecycle::PlanClosed | ConstructionSiteLifecycle::Abandoned
        ) {
            Err(SiteError::SiteImmutable(self.lifecycle))
        } else {
            Ok(())
        }
    }

    fn bump_revision(&mut self) -> Result<(), SiteError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SiteError::RevisionOverflow)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum SiteError {
    PlanMismatch {
        expected_id: FabricationPlanId,
        expected_revision: u64,
        actual_id: FabricationPlanId,
        actual_revision: u64,
    },
    SiteNotOpen(ConstructionSiteLifecycle),
    SiteImmutable(ConstructionSiteLifecycle),
    DuplicateAdmissionId(SiteStepAdmissionId),
    DuplicateExecutionId(ProcessExecutionId),
    StepAlreadyCompleted(PlanStepId),
    StepAlreadyAdmitted(PlanStepId),
    UnknownPlanStep(PlanStepId),
    StepNotReady(PlanStepId),
    UnknownAdmission(SiteStepAdmissionId),
    UnknownExecutionAdmission(ProcessExecutionId),
    AdmissionNotOpen(SiteStepAdmissionId),
    ProcessNotCompleted(ProcessExecutionId),
    InvalidProcessEvidenceDigest(String),
    MissingResultingMatterEvidence(ProcessExecutionId),
    DuplicateProcessEvidence {
        authority_id: StableId,
        evidence_id: StableId,
    },
    ProcessSpecMismatch {
        step_id: PlanStepId,
        expected_id: ProcessSpecId,
        expected_revision: u64,
        actual_id: ProcessSpecId,
        actual_revision: u64,
    },
    WorkpieceSetMismatch {
        step_id: PlanStepId,
        expected: Vec<WorkpieceId>,
        actual: Vec<WorkpieceId>,
    },
    PlanIncomplete,
    InvalidLifecycleTransition {
        from: ConstructionSiteLifecycle,
        to: ConstructionSiteLifecycle,
    },
    RevisionOverflow,
    Plan(PlanError),
}

impl From<PlanError> for SiteError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error)
    }
}

impl fmt::Display for SiteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "construction site binds plan {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::SiteNotOpen(state) => write!(formatter, "construction site is not open: {state:?}"),
            Self::SiteImmutable(state) => write!(formatter, "construction site is immutable: {state:?}"),
            Self::DuplicateAdmissionId(id) => write!(formatter, "site admission {id} already exists"),
            Self::DuplicateExecutionId(id) => write!(formatter, "process execution {id} is already site-bound"),
            Self::StepAlreadyCompleted(id) => write!(formatter, "plan step {id} is already completed"),
            Self::StepAlreadyAdmitted(id) => write!(formatter, "plan step {id} already has an open admission"),
            Self::UnknownPlanStep(id) => write!(formatter, "plan step {id} is not part of the site plan"),
            Self::StepNotReady(id) => write!(formatter, "plan step {id} is not currently ready"),
            Self::UnknownAdmission(id) => write!(formatter, "site admission {id} does not exist"),
            Self::UnknownExecutionAdmission(id) => write!(formatter, "process execution {id} was never admitted for this site"),
            Self::AdmissionNotOpen(id) => write!(formatter, "site admission {id} is not open"),
            Self::ProcessNotCompleted(id) => write!(formatter, "process execution {id} is not completed"),
            Self::InvalidProcessEvidenceDigest(digest) => write!(
                formatter,
                "process evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::MissingResultingMatterEvidence(id) => write!(
                formatter,
                "completed process execution {id} has no resulting matter evidence"
            ),
            Self::DuplicateProcessEvidence {
                authority_id,
                evidence_id,
            } => write!(
                formatter,
                "process evidence {authority_id}/{evidence_id} is already consumed by this site"
            ),
            Self::ProcessSpecMismatch {
                step_id,
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "plan step {step_id} requires process {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::WorkpieceSetMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "plan step {step_id} workpieces mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::PlanIncomplete => write!(formatter, "construction plan still has incomplete steps"),
            Self::InvalidLifecycleTransition { from, to } => {
                write!(formatter, "invalid construction-site transition {from:?} -> {to:?}")
            }
            Self::RevisionOverflow => write!(formatter, "construction-site revision overflow"),
            Self::Plan(error) => error.fmt(formatter),
        }
    }
}

impl Error for SiteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{
        FabricationPlan, MatterBinding, PlanDependency, PlanStep, ProcessExecution, ProcessKind,
        ProcessSpec, Workpiece, WorkpieceLifecycle,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn binding(name: &str, revision: u64) -> MatterBinding {
        MatterBinding::new(
            id("matter:test"),
            id(&format!("allocation:{name}")),
            revision,
            format!("digest:{name}:{revision}"),
        )
        .unwrap()
    }

    fn workpiece(name: &str) -> Workpiece {
        let mut value = Workpiece::new(
            symtropy_fabrication::WorkpieceId::new(id(&format!("workpiece:{name}"))),
            vec![binding(name, 1)],
        )
        .unwrap();
        value.transition(WorkpieceLifecycle::Available).unwrap();
        value
    }

    fn spec(name: &str, kind: ProcessKind) -> ProcessSpec {
        ProcessSpec::new(
            ProcessSpecId::new(id(&format!("process-spec:{name}"))),
            1,
            kind,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
    }

    fn step(name: &str, workpiece: &Workpiece) -> PlanStep {
        PlanStep::new(
            PlanStepId::new(id(&format!("step:{name}"))),
            ProcessSpecId::new(id(&format!("process-spec:{name}"))),
            1,
            vec![workpiece.id.clone()],
            Vec::new(),
            vec![id(&format!("evidence-kind:{name}"))],
        )
        .unwrap()
    }

    fn plan(workpiece: &Workpiece) -> FabricationPlan {
        FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:patch-conduit")),
            1,
            vec![step("clean", workpiece), step("seal", workpiece)],
            vec![
                PlanDependency::new(
                    PlanStepId::new(id("step:clean")),
                    PlanStepId::new(id("step:seal")),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn run_process(
        execution_name: &str,
        evidence_name: &str,
        spec: &ProcessSpec,
        workpiece: &Workpiece,
        result_revision: u64,
    ) -> ProcessEvidence {
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id(&format!("process-execution:{execution_name}"))),
            spec,
            &[workpiece],
            &[],
        )
        .unwrap();
        execution
            .complete(
                id("matter:test"),
                id(&format!("process-evidence:{evidence_name}")),
                result_revision,
                format!("digest:process:{evidence_name}:{result_revision}"),
                vec![binding(&format!("{evidence_name}:result"), result_revision)],
            )
            .unwrap()
    }

    fn site(plan: &FabricationPlan) -> ConstructionSite {
        ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch-conduit")),
            plan,
        )
    }

    #[test]
    fn process_evidence_cannot_close_a_step_without_prior_site_admission() {
        let workpiece = workpiece("conduit");
        let plan = plan(&workpiece);
        let mut site = site(&plan);
        let evidence = run_process("clean", "clean", &spec("clean", ProcessKind::Clean), &workpiece, 2);

        let result = site.record_completion(&plan, &evidence);
        assert!(matches!(result, Err(SiteError::UnknownExecutionAdmission(_))));
    }

    #[test]
    fn dependency_order_is_enforced_at_admission_and_completion() {
        let workpiece = workpiece("conduit");
        let plan = plan(&workpiece);
        let mut site = site(&plan);

        let early_seal = site.admit_step(
            &plan,
            SiteStepAdmissionId::new(id("site-admission:seal:early")),
            PlanStepId::new(id("step:seal")),
            ProcessExecutionId::new(id("process-execution:seal:early")),
        );
        assert!(matches!(early_seal, Err(SiteError::StepNotReady(_))));

        site.admit_step(
            &plan,
            SiteStepAdmissionId::new(id("site-admission:clean")),
            PlanStepId::new(id("step:clean")),
            ProcessExecutionId::new(id("process-execution:clean")),
        )
        .unwrap();
        let clean = run_process("clean", "clean", &spec("clean", ProcessKind::Clean), &workpiece, 2);
        site.record_completion(&plan, &clean).unwrap();

        assert_eq!(
            site.ready_steps(&plan).unwrap(),
            vec![PlanStepId::new(id("step:seal"))]
        );
    }

    #[test]
    fn admitted_execution_must_match_exact_process_spec_and_workpiece_set() {
        let conduit = workpiece("conduit");
        let other = workpiece("other");
        let plan = plan(&conduit);
        let mut site = site(&plan);

        site.admit_step(
            &plan,
            SiteStepAdmissionId::new(id("site-admission:clean")),
            PlanStepId::new(id("step:clean")),
            ProcessExecutionId::new(id("process-execution:clean")),
        )
        .unwrap();

        let wrong_workpiece = run_process(
            "clean",
            "clean-wrong-workpiece",
            &spec("clean", ProcessKind::Clean),
            &other,
            2,
        );
        assert!(matches!(
            site.record_completion(&plan, &wrong_workpiece),
            Err(SiteError::WorkpieceSetMismatch { .. })
        ));

        let wrong_spec = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:clean")),
            2,
            ProcessKind::Clean,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap();
        let wrong_revision = run_process("clean", "clean-wrong-revision", &wrong_spec, &conduit, 2);
        assert!(matches!(
            site.record_completion(&plan, &wrong_revision),
            Err(SiteError::ProcessSpecMismatch { .. })
        ));
    }

    #[test]
    fn cancelled_admission_allows_retry_with_fresh_execution_identity() {
        let workpiece = workpiece("conduit");
        let plan = plan(&workpiece);
        let mut site = site(&plan);
        let first = SiteStepAdmissionId::new(id("site-admission:clean:first"));

        site.admit_step(
            &plan,
            first.clone(),
            PlanStepId::new(id("step:clean")),
            ProcessExecutionId::new(id("process-execution:clean:first")),
        )
        .unwrap();
        site.cancel_admission(&first, id("reason:process-aborted"))
            .unwrap();

        site.admit_step(
            &plan,
            SiteStepAdmissionId::new(id("site-admission:clean:retry")),
            PlanStepId::new(id("step:clean")),
            ProcessExecutionId::new(id("process-execution:clean:retry")),
        )
        .unwrap();
    }

    #[test]
    fn plan_close_is_evidence_backed_but_not_technical_commissioning() {
        let workpiece = workpiece("conduit");
        let plan = plan(&workpiece);
        let mut site = site(&plan);

        for (name, kind, revision) in [
            ("clean", ProcessKind::Clean, 2_u64),
            ("seal", ProcessKind::Seal, 3_u64),
        ] {
            site.admit_step(
                &plan,
                SiteStepAdmissionId::new(id(&format!("site-admission:{name}"))),
                PlanStepId::new(id(&format!("step:{name}"))),
                ProcessExecutionId::new(id(&format!("process-execution:{name}"))),
            )
            .unwrap();
            let evidence = run_process(
                name,
                name,
                &spec(name, kind),
                &workpiece,
                revision,
            );
            site.record_completion(&plan, &evidence).unwrap();
        }

        assert!(site.plan_complete(&plan).unwrap());
        site.close_plan(&plan).unwrap();
        assert_eq!(site.lifecycle, ConstructionSiteLifecycle::PlanClosed);

        let value = serde_json::to_value(site).unwrap();
        for forbidden in [
            "built",
            "progress",
            "quality",
            "score",
            "technical_ready",
            "authorized",
            "commissioned",
        ] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }

    #[test]
    fn suspended_site_blocks_new_admissions_but_can_accept_in_flight_completion() {
        let workpiece = workpiece("conduit");
        let plan = plan(&workpiece);
        let mut site = site(&plan);

        site.admit_step(
            &plan,
            SiteStepAdmissionId::new(id("site-admission:clean")),
            PlanStepId::new(id("step:clean")),
            ProcessExecutionId::new(id("process-execution:clean")),
        )
        .unwrap();
        site.suspend().unwrap();

        let clean = run_process("clean", "clean", &spec("clean", ProcessKind::Clean), &workpiece, 2);
        site.record_completion(&plan, &clean).unwrap();
        let next = site.admit_step(
            &plan,
            SiteStepAdmissionId::new(id("site-admission:seal")),
            PlanStepId::new(id("step:seal")),
            ProcessExecutionId::new(id("process-execution:seal")),
        );
        assert!(matches!(next, Err(SiteError::SiteNotOpen(_))));
    }
}

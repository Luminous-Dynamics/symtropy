// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact executable-plan authority for construction-site execution.
//!
//! `ConstructionSite` remains the existing C1 ledger during migration. This
//! layer owns one immutable `ExecutableFabricationPlan`, removes caller-supplied
//! plan values from the execution API, accepts only validated F4 process
//! evidence, and persists the exact process semantics that closed each step.

use serde::{Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt};
use symtropy_fabrication::{
    CapabilityAdmissionId, ExecutableFabricationPlan, FabricationPlanId, PlanStepId,
    ProcessExecutionId, ProcessSpecSnapshot, ValidatedProcessEvidence, WorkpieceId,
};
use symtropy_game_state::StableId;

use crate::{
    ConstructionSite, ConstructionSiteId, ConstructionSiteLifecycle, SiteError, SiteStepAdmission,
    SiteStepAdmissionId, SiteStepAdmissionState,
};

/// Exact evidence projection retained by the construction authority after C1
/// accepts one completed F4 execution.
///
/// This is not a second fabrication record. The authority/evidence identity and
/// digest point back to Fabrication; Construction persists only the exact
/// semantic/work-context facts required to prove that the accepted evidence
/// closed the site-owned plan step it claims to close.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactSiteProcessEvidenceRef {
    pub step_id: PlanStepId,
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
    pub execution_id: ProcessExecutionId,
    pub process_spec: ProcessSpecSnapshot,
    input_workpiece_ids: Vec<WorkpieceId>,
    capability_evidence_ids: Vec<StableId>,
}

impl ExactSiteProcessEvidenceRef {
    fn from_validated(step_id: PlanStepId, evidence: &ValidatedProcessEvidence) -> Self {
        let mut input_workpiece_ids = evidence
            .inputs()
            .iter()
            .map(|input| input.workpiece_id.clone())
            .collect::<Vec<_>>();
        input_workpiece_ids.sort();

        let mut capability_evidence_ids = evidence
            .admitted_capabilities()
            .iter()
            .map(|capability| capability.evidence_id.clone())
            .collect::<Vec<_>>();
        capability_evidence_ids.sort();

        Self {
            step_id,
            authority_id: evidence.authority_id().clone(),
            evidence_id: evidence.evidence_id().clone(),
            revision: evidence.evidence_revision(),
            digest: evidence.evidence_digest().to_owned(),
            execution_id: evidence.execution_id().clone(),
            process_spec: evidence.spec_snapshot().clone(),
            input_workpiece_ids,
            capability_evidence_ids,
        }
    }

    pub fn input_workpiece_ids(&self) -> &[WorkpieceId] {
        &self.input_workpiece_ids
    }

    pub fn capability_evidence_ids(&self) -> &[StableId] {
        &self.capability_evidence_ids
    }
}

/// Public exact construction execution boundary.
///
/// The authored `FabricationPlan` cannot be supplied to runtime methods. The
/// exact executable contract is captured once at construction and is the only
/// plan authority used for readiness, admission, completion, and closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExactConstructionSite {
    site: ConstructionSite,
    executable_plan: ExecutableFabricationPlan,
    exact_completions: Vec<ExactSiteProcessEvidenceRef>,
}

impl ExactConstructionSite {
    pub fn new(id: ConstructionSiteId, executable_plan: ExecutableFabricationPlan) -> Self {
        let site = ConstructionSite::new(id, executable_plan.plan());
        Self {
            site,
            executable_plan,
            exact_completions: Vec::new(),
        }
    }

    pub fn id(&self) -> &ConstructionSiteId {
        &self.site.id
    }

    pub fn revision(&self) -> u64 {
        self.site.revision
    }

    pub fn lifecycle(&self) -> ConstructionSiteLifecycle {
        self.site.lifecycle
    }

    pub fn executable_plan(&self) -> &ExecutableFabricationPlan {
        &self.executable_plan
    }

    pub fn plan_id(&self) -> &FabricationPlanId {
        &self.executable_plan.plan().id
    }

    pub fn plan_revision(&self) -> u64 {
        self.executable_plan.plan().revision
    }

    pub fn admissions(&self) -> &[SiteStepAdmission] {
        self.site.admissions()
    }

    pub fn exact_completions(&self) -> &[ExactSiteProcessEvidenceRef] {
        &self.exact_completions
    }

    pub fn completed_step_ids(&self) -> Vec<PlanStepId> {
        self.site.completed_step_ids()
    }

    pub fn ready_steps(&self) -> Result<Vec<PlanStepId>, ExactSiteError> {
        Ok(self.site.ready_steps(self.executable_plan.plan())?)
    }

    pub fn admit_step(
        &mut self,
        admission_id: SiteStepAdmissionId,
        step_id: PlanStepId,
        execution_id: ProcessExecutionId,
    ) -> Result<(), ExactSiteError> {
        self.site.admit_step(
            self.executable_plan.plan(),
            admission_id,
            step_id,
            execution_id,
        )?;
        Ok(())
    }

    pub fn admit_step_with_capabilities(
        &mut self,
        admission_id: SiteStepAdmissionId,
        step_id: PlanStepId,
        execution_id: ProcessExecutionId,
        capability_admission_ids: Vec<CapabilityAdmissionId>,
    ) -> Result<(), ExactSiteError> {
        self.site.admit_step_with_capabilities(
            self.executable_plan.plan(),
            admission_id,
            step_id,
            execution_id,
            capability_admission_ids,
        )?;
        Ok(())
    }

    pub fn cancel_admission(
        &mut self,
        admission_id: &SiteStepAdmissionId,
        reason_id: StableId,
    ) -> Result<(), ExactSiteError> {
        self.site.cancel_admission(admission_id, reason_id)?;
        Ok(())
    }

    /// Accepts only already-validated F4 evidence and requires the full
    /// canonical process snapshot to equal the exact binding owned by this site.
    /// ID/revision equality alone is intentionally insufficient.
    pub fn record_completion(
        &mut self,
        evidence: &ValidatedProcessEvidence,
    ) -> Result<PlanStepId, ExactSiteError> {
        let admission = self
            .site
            .admissions()
            .iter()
            .find(|admission| &admission.execution_id == evidence.execution_id())
            .ok_or_else(|| {
                ExactSiteError::UnknownExecutionAdmission(evidence.execution_id().clone())
            })?;
        let step_id = admission.step_id.clone();
        let expected = self
            .executable_plan
            .process_spec(&step_id)
            .ok_or_else(|| ExactSiteError::MissingExactProcessBinding(step_id.clone()))?;

        if expected != evidence.spec_snapshot() {
            return Err(ExactSiteError::ExactProcessSemanticsMismatch {
                step_id,
                expected: expected.clone(),
                actual: evidence.spec_snapshot().clone(),
            });
        }

        let completed_step = self
            .site
            .record_completion(self.executable_plan.plan(), evidence.as_process_evidence())?;
        let exact_ref =
            ExactSiteProcessEvidenceRef::from_validated(completed_step.clone(), evidence);
        self.exact_completions.push(exact_ref);
        self.exact_completions
            .sort_by(|left, right| left.step_id.cmp(&right.step_id));
        Ok(completed_step)
    }

    pub fn plan_complete(&self) -> Result<bool, ExactSiteError> {
        Ok(self.site.plan_complete(self.executable_plan.plan())?)
    }

    pub fn close_plan(&mut self) -> Result<(), ExactSiteError> {
        self.site.close_plan(self.executable_plan.plan())?;
        Ok(())
    }

    pub fn suspend(&mut self) -> Result<(), ExactSiteError> {
        self.site.suspend()?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), ExactSiteError> {
        self.site.resume()?;
        Ok(())
    }

    pub fn abandon(&mut self) -> Result<(), ExactSiteError> {
        self.site.abandon()?;
        Ok(())
    }

    /// Internal migration bridge for C2/C3. External callers should not regain
    /// a caller-supplied plan boundary through the legacy ledger.
    pub(crate) fn legacy_site(&self) -> &ConstructionSite {
        &self.site
    }

    pub(crate) fn legacy_site_mut(&mut self) -> &mut ConstructionSite {
        &mut self.site
    }

    fn from_wire(
        site: ConstructionSite,
        executable_plan: ExecutableFabricationPlan,
        exact_completions: Vec<ExactSiteProcessEvidenceRef>,
    ) -> Result<Self, ExactSiteError> {
        let value = Self {
            site,
            executable_plan,
            exact_completions,
        };
        value.validate_restored_state()?;
        Ok(value)
    }

    fn validate_restored_state(&self) -> Result<(), ExactSiteError> {
        if self.site.revision == 0 {
            return Err(ExactSiteError::InvalidSiteRevision);
        }
        if self.site.plan_id != self.executable_plan.plan().id
            || self.site.plan_revision != self.executable_plan.plan().revision
        {
            return Err(ExactSiteError::LegacyPlanIdentityMismatch {
                site_id: self.site.plan_id.clone(),
                site_revision: self.site.plan_revision,
                exact_id: self.executable_plan.plan().id.clone(),
                exact_revision: self.executable_plan.plan().revision,
            });
        }

        self.validate_admissions()?;
        self.validate_exact_completions()?;

        if self.site.lifecycle == ConstructionSiteLifecycle::PlanClosed
            && !self.site.plan_complete(self.executable_plan.plan())?
        {
            return Err(ExactSiteError::ClosedSiteHasIncompletePlan);
        }

        Ok(())
    }

    fn validate_admissions(&self) -> Result<(), ExactSiteError> {
        let admissions = self.site.admissions();
        for (index, admission) in admissions.iter().enumerate() {
            if admissions[..index]
                .iter()
                .any(|existing| existing.id == admission.id)
            {
                return Err(ExactSiteError::DuplicateAdmissionId(admission.id.clone()));
            }
            if admissions[..index]
                .iter()
                .any(|existing| existing.execution_id == admission.execution_id)
            {
                return Err(ExactSiteError::DuplicateExecutionId(
                    admission.execution_id.clone(),
                ));
            }
            if self
                .executable_plan
                .plan()
                .step(&admission.step_id)
                .is_none()
            {
                return Err(ExactSiteError::UnknownAdmissionStep(
                    admission.step_id.clone(),
                ));
            }

            let capability_ids = admission.capability_admission_ids();
            for pair in capability_ids.windows(2) {
                if pair[0] >= pair[1] {
                    return Err(ExactSiteError::NonCanonicalCapabilityAdmissions(
                        admission.id.clone(),
                    ));
                }
            }

            match admission.state {
                SiteStepAdmissionState::Open => {
                    if admission.completion.is_some() || admission.cancellation_reason_id.is_some()
                    {
                        return Err(ExactSiteError::InconsistentAdmissionState(
                            admission.id.clone(),
                        ));
                    }
                }
                SiteStepAdmissionState::Cancelled => {
                    if admission.completion.is_some() || admission.cancellation_reason_id.is_none()
                    {
                        return Err(ExactSiteError::InconsistentAdmissionState(
                            admission.id.clone(),
                        ));
                    }
                }
                SiteStepAdmissionState::Completed => {
                    if admission.completion.is_none() || admission.cancellation_reason_id.is_some()
                    {
                        return Err(ExactSiteError::InconsistentAdmissionState(
                            admission.id.clone(),
                        ));
                    }
                }
            }
        }

        for (index, admission) in admissions.iter().enumerate() {
            if admission.state == SiteStepAdmissionState::Completed
                && admissions[..index].iter().any(|existing| {
                    existing.step_id == admission.step_id
                        && existing.state == SiteStepAdmissionState::Completed
                })
            {
                return Err(ExactSiteError::DuplicateCompletedStep(
                    admission.step_id.clone(),
                ));
            }
            if admission.state == SiteStepAdmissionState::Open
                && admissions[..index].iter().any(|existing| {
                    existing.step_id == admission.step_id
                        && existing.state == SiteStepAdmissionState::Open
                })
            {
                return Err(ExactSiteError::DuplicateOpenStep(admission.step_id.clone()));
            }
        }

        Ok(())
    }

    fn validate_exact_completions(&self) -> Result<(), ExactSiteError> {
        for pair in self.exact_completions.windows(2) {
            if pair[0].step_id >= pair[1].step_id {
                return Err(ExactSiteError::NonCanonicalExactCompletions);
            }
        }

        for (index, completion) in self.exact_completions.iter().enumerate() {
            if completion.digest.is_empty() || completion.digest.len() > 256 {
                return Err(ExactSiteError::InvalidEvidenceDigest(
                    completion.digest.clone(),
                ));
            }
            if self.exact_completions[..index].iter().any(|existing| {
                existing.authority_id == completion.authority_id
                    && existing.evidence_id == completion.evidence_id
            }) {
                return Err(ExactSiteError::DuplicateExactEvidence {
                    authority_id: completion.authority_id.clone(),
                    evidence_id: completion.evidence_id.clone(),
                });
            }

            let expected_spec = self
                .executable_plan
                .process_spec(&completion.step_id)
                .ok_or_else(|| {
                    ExactSiteError::MissingExactProcessBinding(completion.step_id.clone())
                })?;
            if expected_spec != &completion.process_spec {
                return Err(ExactSiteError::ExactProcessSemanticsMismatch {
                    step_id: completion.step_id.clone(),
                    expected: expected_spec.clone(),
                    actual: completion.process_spec.clone(),
                });
            }

            let step = self
                .executable_plan
                .plan()
                .step(&completion.step_id)
                .ok_or_else(|| ExactSiteError::UnknownAdmissionStep(completion.step_id.clone()))?;
            if completion.input_workpiece_ids != step.workpieces() {
                return Err(ExactSiteError::CompletionWorkpieceMismatch(
                    completion.step_id.clone(),
                ));
            }

            let admission = self
                .site
                .admissions()
                .iter()
                .find(|admission| {
                    admission.step_id == completion.step_id
                        && admission.execution_id == completion.execution_id
                        && admission.state == SiteStepAdmissionState::Completed
                })
                .ok_or_else(|| {
                    ExactSiteError::MissingCompletedAdmission(completion.step_id.clone())
                })?;

            let mut expected_capability_evidence = admission
                .capability_admission_ids()
                .iter()
                .map(|id| id.stable_id().clone())
                .collect::<Vec<_>>();
            expected_capability_evidence.sort();
            if completion.capability_evidence_ids != expected_capability_evidence {
                return Err(ExactSiteError::CompletionCapabilityMismatch(
                    completion.step_id.clone(),
                ));
            }

            let legacy = admission.completion.as_ref().ok_or_else(|| {
                ExactSiteError::MissingLegacyCompletion(completion.step_id.clone())
            })?;
            if legacy.authority_id != completion.authority_id
                || legacy.evidence_id != completion.evidence_id
                || legacy.revision != completion.revision
                || legacy.digest != completion.digest
                || legacy.execution_id != completion.execution_id
                || legacy.process_spec_id != completion.process_spec.id
                || legacy.process_spec_revision != completion.process_spec.revision
            {
                return Err(ExactSiteError::LegacyCompletionMismatch(
                    completion.step_id.clone(),
                ));
            }
        }

        for admission in self.site.admissions() {
            if admission.state == SiteStepAdmissionState::Completed
                && !self.exact_completions.iter().any(|completion| {
                    completion.step_id == admission.step_id
                        && completion.execution_id == admission.execution_id
                })
            {
                return Err(ExactSiteError::MissingExactCompletion(
                    admission.step_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct ExactConstructionSiteWire {
    site: ConstructionSite,
    executable_plan: ExecutableFabricationPlan,
    exact_completions: Vec<ExactSiteProcessEvidenceRef>,
}

impl<'de> Deserialize<'de> for ExactConstructionSite {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExactConstructionSiteWire::deserialize(deserializer)?;
        Self::from_wire(wire.site, wire.executable_plan, wire.exact_completions)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum ExactSiteError {
    UnknownExecutionAdmission(ProcessExecutionId),
    MissingExactProcessBinding(PlanStepId),
    ExactProcessSemanticsMismatch {
        step_id: PlanStepId,
        expected: ProcessSpecSnapshot,
        actual: ProcessSpecSnapshot,
    },
    InvalidSiteRevision,
    LegacyPlanIdentityMismatch {
        site_id: FabricationPlanId,
        site_revision: u64,
        exact_id: FabricationPlanId,
        exact_revision: u64,
    },
    DuplicateAdmissionId(SiteStepAdmissionId),
    DuplicateExecutionId(ProcessExecutionId),
    UnknownAdmissionStep(PlanStepId),
    NonCanonicalCapabilityAdmissions(SiteStepAdmissionId),
    InconsistentAdmissionState(SiteStepAdmissionId),
    DuplicateCompletedStep(PlanStepId),
    DuplicateOpenStep(PlanStepId),
    NonCanonicalExactCompletions,
    InvalidEvidenceDigest(String),
    DuplicateExactEvidence {
        authority_id: StableId,
        evidence_id: StableId,
    },
    CompletionWorkpieceMismatch(PlanStepId),
    CompletionCapabilityMismatch(PlanStepId),
    MissingCompletedAdmission(PlanStepId),
    MissingLegacyCompletion(PlanStepId),
    LegacyCompletionMismatch(PlanStepId),
    MissingExactCompletion(PlanStepId),
    ClosedSiteHasIncompletePlan,
    Site(SiteError),
}

impl From<SiteError> for ExactSiteError {
    fn from(error: SiteError) -> Self {
        Self::Site(error)
    }
}

impl fmt::Display for ExactSiteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownExecutionAdmission(id) => write!(
                formatter,
                "process execution {id} was never admitted by the exact construction site"
            ),
            Self::MissingExactProcessBinding(step) => write!(
                formatter,
                "exact construction site lacks a process binding for step {step}"
            ),
            Self::ExactProcessSemanticsMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "step {step_id} exact process semantics mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::InvalidSiteRevision => {
                write!(
                    formatter,
                    "exact construction site revision must be non-zero"
                )
            }
            Self::LegacyPlanIdentityMismatch {
                site_id,
                site_revision,
                exact_id,
                exact_revision,
            } => write!(
                formatter,
                "legacy site binds {site_id}@{site_revision}, exact site owns {exact_id}@{exact_revision}"
            ),
            Self::DuplicateAdmissionId(id) => write!(formatter, "site repeats admission {id}"),
            Self::DuplicateExecutionId(id) => write!(formatter, "site repeats execution {id}"),
            Self::UnknownAdmissionStep(step) => {
                write!(
                    formatter,
                    "site admission references unknown exact-plan step {step}"
                )
            }
            Self::NonCanonicalCapabilityAdmissions(id) => write!(
                formatter,
                "site admission {id} capability admissions are not canonical and unique"
            ),
            Self::InconsistentAdmissionState(id) => {
                write!(
                    formatter,
                    "site admission {id} has inconsistent terminal fields"
                )
            }
            Self::DuplicateCompletedStep(step) => {
                write!(formatter, "site repeats completed step {step}")
            }
            Self::DuplicateOpenStep(step) => write!(formatter, "site repeats open step {step}"),
            Self::NonCanonicalExactCompletions => {
                write!(
                    formatter,
                    "exact site completions are not canonical and unique"
                )
            }
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "exact site evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::DuplicateExactEvidence {
                authority_id,
                evidence_id,
            } => write!(
                formatter,
                "exact site repeats process evidence {authority_id}/{evidence_id}"
            ),
            Self::CompletionWorkpieceMismatch(step) => write!(
                formatter,
                "exact completion workpieces disagree with site-owned step {step}"
            ),
            Self::CompletionCapabilityMismatch(step) => write!(
                formatter,
                "exact completion capability evidence disagrees with admission for step {step}"
            ),
            Self::MissingCompletedAdmission(step) => write!(
                formatter,
                "exact completion for step {step} lacks a matching completed site admission"
            ),
            Self::MissingLegacyCompletion(step) => write!(
                formatter,
                "completed site admission for step {step} lacks its C1 evidence reference"
            ),
            Self::LegacyCompletionMismatch(step) => write!(
                formatter,
                "exact completion for step {step} disagrees with the C1 evidence reference"
            ),
            Self::MissingExactCompletion(step) => write!(
                formatter,
                "completed site admission for step {step} lacks exact semantic completion evidence"
            ),
            Self::ClosedSiteHasIncompletePlan => write!(
                formatter,
                "plan-closed exact site does not have evidence-backed completion for every step"
            ),
            Self::Site(error) => error.fmt(formatter),
        }
    }
}

impl Error for ExactSiteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Site(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{
        ExactPlanProcessBinding, FabricationPlan, MatterBinding, PlanStep, ProcessExecution,
        ProcessExecutionId, ProcessKind, ProcessSpec, ProcessSpecId, Workpiece, WorkpieceLifecycle,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn workpiece() -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:exact-site")),
            vec![
                MatterBinding::new(
                    id("matter:test"),
                    id("allocation:exact-site"),
                    1,
                    "digest:exact-site:1",
                )
                .unwrap(),
            ],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn process(states: Vec<WorkpieceLifecycle>) -> ProcessSpec {
        ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:exact-site")),
            1,
            ProcessKind::Clean,
            Vec::new(),
            states,
        )
        .unwrap()
    }

    fn executable(workpiece: &Workpiece, process: &ProcessSpec) -> ExecutableFabricationPlan {
        let step_id = PlanStepId::new(id("step:exact-site"));
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:exact-site")),
            1,
            vec![
                PlanStep::new(
                    step_id.clone(),
                    process.id.clone(),
                    process.revision,
                    vec![workpiece.id.clone()],
                    Vec::new(),
                    vec![id("evidence-kind:exact-site")],
                )
                .unwrap(),
            ],
            Vec::new(),
        )
        .unwrap();
        ExecutableFabricationPlan::new(
            plan,
            vec![ExactPlanProcessBinding::new(step_id, process.snapshot())],
        )
        .unwrap()
    }

    fn evidence(workpiece: &Workpiece, process: &ProcessSpec) -> ValidatedProcessEvidence {
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:exact-site")),
            process,
            &[workpiece],
            &[],
        )
        .unwrap();
        ValidatedProcessEvidence::new(
            execution
                .complete(
                    id("process-authority:test"),
                    id("process-evidence:exact-site"),
                    1,
                    "digest:process:exact-site",
                    vec![
                        MatterBinding::new(
                            id("matter:test"),
                            id("allocation:exact-site-result"),
                            2,
                            "digest:exact-site-result:2",
                        )
                        .unwrap(),
                    ],
                )
                .unwrap(),
        )
        .unwrap()
    }

    fn admitted_site(workpiece: &Workpiece, process: &ProcessSpec) -> ExactConstructionSite {
        let mut site = ExactConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:exact")),
            executable(workpiece, process),
        );
        site.admit_step(
            SiteStepAdmissionId::new(id("site-admission:exact")),
            PlanStepId::new(id("step:exact-site")),
            ProcessExecutionId::new(id("process-execution:exact-site")),
        )
        .unwrap();
        site
    }

    #[test]
    fn exact_site_executes_without_caller_supplied_plan() {
        let workpiece = workpiece();
        let process = process(vec![WorkpieceLifecycle::Available]);
        let mut site = admitted_site(&workpiece, &process);
        let evidence = evidence(&workpiece, &process);

        assert_eq!(
            site.record_completion(&evidence).unwrap(),
            PlanStepId::new(id("step:exact-site"))
        );
        assert!(site.plan_complete().unwrap());
        assert_eq!(site.exact_completions().len(), 1);
    }

    #[test]
    fn same_process_id_revision_with_changed_semantics_is_rejected() {
        let workpiece = workpiece();
        let exact_process = process(vec![WorkpieceLifecycle::Available]);
        let mut site = admitted_site(&workpiece, &exact_process);
        let altered = process(vec![
            WorkpieceLifecycle::Available,
            WorkpieceLifecycle::Installed,
        ]);
        let evidence = evidence(&workpiece, &altered);

        let result = site.record_completion(&evidence);
        assert!(matches!(
            result,
            Err(ExactSiteError::ExactProcessSemanticsMismatch { .. })
        ));
    }

    #[test]
    fn exact_site_round_trip_preserves_plan_and_completion_contract() {
        let workpiece = workpiece();
        let process = process(vec![WorkpieceLifecycle::Available]);
        let mut site = admitted_site(&workpiece, &process);
        site.record_completion(&evidence(&workpiece, &process))
            .unwrap();

        let encoded = serde_json::to_vec(&site).unwrap();
        let restored: ExactConstructionSite = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, site);
        assert_eq!(restored.exact_completions().len(), 1);
    }

    #[test]
    fn restored_exact_completion_cannot_change_process_semantics() {
        let workpiece = workpiece();
        let process = process(vec![WorkpieceLifecycle::Available]);
        let mut site = admitted_site(&workpiece, &process);
        site.record_completion(&evidence(&workpiece, &process))
            .unwrap();
        let mut value = serde_json::to_value(&site).unwrap();
        value["exact_completions"][0]["process_spec"]["allowed_workpiece_states"] =
            serde_json::json!(["available", "installed"]);

        assert!(serde_json::from_value::<ExactConstructionSite>(value).is_err());
    }

    #[test]
    fn restored_site_requires_exact_completion_for_every_completed_admission() {
        let workpiece = workpiece();
        let process = process(vec![WorkpieceLifecycle::Available]);
        let mut site = admitted_site(&workpiece, &process);
        site.record_completion(&evidence(&workpiece, &process))
            .unwrap();
        let mut value = serde_json::to_value(&site).unwrap();
        value["exact_completions"] = serde_json::Value::Array(Vec::new());

        assert!(serde_json::from_value::<ExactConstructionSite>(value).is_err());
    }

    #[test]
    fn exact_site_serialization_has_no_quality_or_authorization_oracle() {
        let workpiece = workpiece();
        let process = process(vec![WorkpieceLifecycle::Available]);
        let site = ExactConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:exact")),
            executable(&workpiece, &process),
        );
        let json = serde_json::to_string(&site).unwrap();
        for forbidden in [
            "\"quality\"",
            "\"score\"",
            "\"safe\"",
            "\"authorized\"",
            "\"commissioned\"",
            "\"restored_service\"",
        ] {
            assert!(!json.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}

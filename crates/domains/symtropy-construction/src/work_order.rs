// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-bound construction work orders.
//!
//! A work order is scheduling and responsibility intent. Releasing an order
//! presents exact F5 capability admissions and creates a C1 site-step admission,
//! but the order never claims that physical work completed, that engineering
//! constraints pass, or that the result is technically/civically commissioned.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_fabrication::{
    CapabilityAdmission, CapabilityAdmissionId, CapabilityEvidence, CapabilityEvidenceRef,
    CapabilityNeedId, FabricationPlan, FabricationPlanId, PlanStepId, ProcessExecutionId,
};
use symtropy_game_state::StableId;

use crate::{
    ConstructionSite, ConstructionSiteId, ConstructionSiteLifecycle, SiteError,
    SiteStepAdmissionId, SiteStepAdmissionState,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkOrderId(StableId);

impl WorkOrderId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for WorkOrderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Responsible actor identity is intentionally generic: player, resident,
/// crew, robot, or automated controller can all occupy this role. Tool/machine
/// capabilities remain independently evidenced through F5 admissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkActorRef {
    pub authority_id: StableId,
    pub actor_id: StableId,
    pub actor_revision: u64,
}

/// Minimal exact F5 admission retained at work release. This captures *why* a
/// provider was considered capable without turning capability into a level or
/// copying the whole mutable tool state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCapabilityAdmissionRef {
    pub admission_id: CapabilityAdmissionId,
    pub need_id: CapabilityNeedId,
    pub provider_id: StableId,
    pub provider_revision: u64,
    pub envelope_evidence: CapabilityEvidenceRef,
}

impl WorkCapabilityAdmissionRef {
    fn from_admission(admission: &CapabilityAdmission) -> Self {
        Self {
            admission_id: admission.id.clone(),
            need_id: admission.need_id.clone(),
            provider_id: admission.provider_id.clone(),
            provider_revision: admission.provider_revision,
            envelope_evidence: admission.envelope_evidence.clone(),
        }
    }

    /// Exact F4 bootstrap token corresponding to this already-satisfied F5
    /// admission. The scalar remains deliberately binary; rich reasoning stays
    /// in F5 while the stable admission identity is preserved end-to-end.
    pub fn bootstrap_evidence(&self) -> CapabilityEvidence {
        CapabilityEvidence {
            capability_id: self.need_id.stable_id().clone(),
            available_value: 1,
            evidence_id: self.admission_id.stable_id().clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderLifecycle {
    Active,
    Cancelled,
}

/// Release binds scheduled work to the exact capability evidence, site
/// admission, and process-execution identity used when execution begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkOrderRelease {
    pub site_admission_id: SiteStepAdmissionId,
    pub execution_id: ProcessExecutionId,
    capability_admissions: Vec<WorkCapabilityAdmissionRef>,
}

impl WorkOrderRelease {
    pub fn capability_admissions(&self) -> &[WorkCapabilityAdmissionRef] {
        &self.capability_admissions
    }

    pub fn bootstrap_evidence(&self) -> Vec<CapabilityEvidence> {
        self.capability_admissions
            .iter()
            .map(WorkCapabilityAdmissionRef::bootstrap_evidence)
            .collect()
    }
}

/// Derived view; never persisted as a competing completion flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkOrderStatus {
    Issued,
    Released,
    SiteCancelled,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkOrder {
    pub id: WorkOrderId,
    pub revision: u64,
    pub site_id: ConstructionSiteId,
    pub plan_id: FabricationPlanId,
    pub plan_revision: u64,
    pub step_id: PlanStepId,
    pub responsible_actor: WorkActorRef,
    pub lifecycle: WorkOrderLifecycle,
    pub cancellation_reason_id: Option<StableId>,
    pub release: Option<WorkOrderRelease>,
}

impl WorkOrder {
    /// Issues scheduling/responsibility intent against one exact site/plan step.
    /// Capability is deliberately *not* frozen here; an order may sit in a queue
    /// while tools, machines, operators, or conditions change.
    pub fn issue(
        id: WorkOrderId,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        step_id: PlanStepId,
        responsible_actor: WorkActorRef,
    ) -> Result<Self, WorkOrderError> {
        validate_site_plan(site, plan)?;
        if matches!(
            site.lifecycle,
            ConstructionSiteLifecycle::PlanClosed | ConstructionSiteLifecycle::Abandoned
        ) {
            return Err(WorkOrderError::SiteUnavailable(site.lifecycle));
        }
        if plan.step(&step_id).is_none() {
            return Err(WorkOrderError::UnknownPlanStep(step_id));
        }

        Ok(Self {
            id,
            revision: 1,
            site_id: site.id.clone(),
            plan_id: plan.id.clone(),
            plan_revision: plan.revision,
            step_id,
            responsible_actor,
            lifecycle: WorkOrderLifecycle::Active,
            cancellation_reason_id: None,
            release: None,
        })
    }

    /// Releases scheduled intent into a concrete C1 execution admission.
    /// Exact F5 capability coverage is checked here, at the execution boundary,
    /// rather than when the order was scheduled.
    pub fn release(
        &mut self,
        site: &mut ConstructionSite,
        plan: &FabricationPlan,
        site_admission_id: SiteStepAdmissionId,
        execution_id: ProcessExecutionId,
        capability_admissions: &[CapabilityAdmission],
    ) -> Result<(), WorkOrderError> {
        self.validate_context(site, plan)?;
        if self.lifecycle == WorkOrderLifecycle::Cancelled {
            return Err(WorkOrderError::OrderCancelled(self.id.clone()));
        }
        if self.release.is_some() {
            return Err(WorkOrderError::AlreadyReleased(self.id.clone()));
        }

        let step = plan
            .step(&self.step_id)
            .ok_or_else(|| WorkOrderError::UnknownPlanStep(self.step_id.clone()))?;
        let capability_refs =
            validate_capability_coverage(step.capability_needs(), capability_admissions)?;
        let capability_ids = capability_refs
            .iter()
            .map(|admission| admission.admission_id.clone())
            .collect::<Vec<_>>();
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(WorkOrderError::RevisionOverflow)?;

        site.admit_step_with_capabilities(
            plan,
            site_admission_id.clone(),
            self.step_id.clone(),
            execution_id.clone(),
            capability_ids,
        )?;

        self.release = Some(WorkOrderRelease {
            site_admission_id,
            execution_id,
            capability_admissions: capability_refs,
        });
        self.revision = next_revision;
        Ok(())
    }

    /// Cancels only an unreleased order. Once work is released, cancellation of
    /// the in-flight execution belongs to the C1 site admission so the physical
    /// attempt and retry history remain explicit.
    pub fn cancel_before_release(&mut self, reason_id: StableId) -> Result<(), WorkOrderError> {
        if self.lifecycle == WorkOrderLifecycle::Cancelled {
            return Err(WorkOrderError::OrderCancelled(self.id.clone()));
        }
        if self.release.is_some() {
            return Err(WorkOrderError::AlreadyReleased(self.id.clone()));
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(WorkOrderError::RevisionOverflow)?;
        self.lifecycle = WorkOrderLifecycle::Cancelled;
        self.cancellation_reason_id = Some(reason_id);
        self.revision = next_revision;
        Ok(())
    }

    /// Derives runtime status from the authoritative site admission rather than
    /// persisting a second `completed` flag on the order.
    pub fn status(&self, site: &ConstructionSite) -> Result<WorkOrderStatus, WorkOrderError> {
        if site.id != self.site_id {
            return Err(WorkOrderError::SiteMismatch {
                expected: self.site_id.clone(),
                actual: site.id.clone(),
            });
        }
        if self.lifecycle == WorkOrderLifecycle::Cancelled {
            if self.release.is_some() {
                return Err(WorkOrderError::InconsistentCancelledRelease(self.id.clone()));
            }
            return Ok(WorkOrderStatus::Cancelled);
        }
        let Some(release) = &self.release else {
            return Ok(WorkOrderStatus::Issued);
        };
        let admission = site
            .admissions()
            .iter()
            .find(|admission| admission.id == release.site_admission_id)
            .ok_or_else(|| WorkOrderError::MissingSiteAdmission(release.site_admission_id.clone()))?;
        if admission.execution_id != release.execution_id || admission.step_id != self.step_id {
            return Err(WorkOrderError::SiteAdmissionMismatch(
                release.site_admission_id.clone(),
            ));
        }

        Ok(match admission.state {
            SiteStepAdmissionState::Open => WorkOrderStatus::Released,
            SiteStepAdmissionState::Cancelled => WorkOrderStatus::SiteCancelled,
            SiteStepAdmissionState::Completed => WorkOrderStatus::Completed,
        })
    }

    fn validate_context(
        &self,
        site: &ConstructionSite,
        plan: &FabricationPlan,
    ) -> Result<(), WorkOrderError> {
        if site.id != self.site_id {
            return Err(WorkOrderError::SiteMismatch {
                expected: self.site_id.clone(),
                actual: site.id.clone(),
            });
        }
        if plan.id != self.plan_id
            || plan.revision != self.plan_revision
            || site.plan_id != self.plan_id
            || site.plan_revision != self.plan_revision
        {
            return Err(WorkOrderError::PlanMismatch {
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: plan.id.clone(),
                actual_revision: plan.revision,
            });
        }
        Ok(())
    }
}

fn validate_site_plan(
    site: &ConstructionSite,
    plan: &FabricationPlan,
) -> Result<(), WorkOrderError> {
    if site.plan_id != plan.id || site.plan_revision != plan.revision {
        return Err(WorkOrderError::PlanMismatch {
            expected_id: site.plan_id.clone(),
            expected_revision: site.plan_revision,
            actual_id: plan.id.clone(),
            actual_revision: plan.revision,
        });
    }
    Ok(())
}

fn validate_capability_coverage(
    expected: &[CapabilityNeedId],
    admissions: &[CapabilityAdmission],
) -> Result<Vec<WorkCapabilityAdmissionRef>, WorkOrderError> {
    let mut coverage = BTreeMap::<CapabilityNeedId, WorkCapabilityAdmissionRef>::new();
    let mut seen_admissions = Vec::<CapabilityAdmissionId>::new();

    for admission in admissions {
        if seen_admissions.contains(&admission.id) {
            return Err(WorkOrderError::DuplicateCapabilityAdmissionId(
                admission.id.clone(),
            ));
        }
        seen_admissions.push(admission.id.clone());

        if !admission.is_satisfied() {
            return Err(WorkOrderError::UnsatisfiedCapabilityAdmission(
                admission.id.clone(),
            ));
        }
        if !expected.contains(&admission.need_id) {
            return Err(WorkOrderError::UnexpectedCapabilityNeed(
                admission.need_id.clone(),
            ));
        }
        if admission.envelope_evidence.digest.is_empty()
            || admission.envelope_evidence.digest.len() > 256
        {
            return Err(WorkOrderError::InvalidCapabilityEvidenceDigest(
                admission.envelope_evidence.digest.clone(),
            ));
        }
        if coverage
            .insert(
                admission.need_id.clone(),
                WorkCapabilityAdmissionRef::from_admission(admission),
            )
            .is_some()
        {
            return Err(WorkOrderError::DuplicateCapabilityCoverage(
                admission.need_id.clone(),
            ));
        }
    }

    for need_id in expected {
        if !coverage.contains_key(need_id) {
            return Err(WorkOrderError::MissingCapabilityCoverage(need_id.clone()));
        }
    }

    Ok(coverage.into_values().collect())
}

#[derive(Debug)]
pub enum WorkOrderError {
    PlanMismatch {
        expected_id: FabricationPlanId,
        expected_revision: u64,
        actual_id: FabricationPlanId,
        actual_revision: u64,
    },
    SiteMismatch {
        expected: ConstructionSiteId,
        actual: ConstructionSiteId,
    },
    SiteUnavailable(ConstructionSiteLifecycle),
    UnknownPlanStep(PlanStepId),
    DuplicateCapabilityAdmissionId(CapabilityAdmissionId),
    UnsatisfiedCapabilityAdmission(CapabilityAdmissionId),
    UnexpectedCapabilityNeed(CapabilityNeedId),
    DuplicateCapabilityCoverage(CapabilityNeedId),
    MissingCapabilityCoverage(CapabilityNeedId),
    InvalidCapabilityEvidenceDigest(String),
    OrderCancelled(WorkOrderId),
    AlreadyReleased(WorkOrderId),
    InconsistentCancelledRelease(WorkOrderId),
    MissingSiteAdmission(SiteStepAdmissionId),
    SiteAdmissionMismatch(SiteStepAdmissionId),
    RevisionOverflow,
    Site(SiteError),
}

impl From<SiteError> for WorkOrderError {
    fn from(error: SiteError) -> Self {
        Self::Site(error)
    }
}

impl fmt::Display for WorkOrderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "work order binds plan {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::SiteMismatch { expected, actual } => {
                write!(formatter, "work order binds site {expected}, got {actual}")
            }
            Self::SiteUnavailable(state) => write!(
                formatter,
                "construction site cannot accept new work orders: {state:?}"
            ),
            Self::UnknownPlanStep(id) => {
                write!(formatter, "work order references unknown plan step {id}")
            }
            Self::DuplicateCapabilityAdmissionId(id) => {
                write!(formatter, "capability admission {id} is repeated")
            }
            Self::UnsatisfiedCapabilityAdmission(id) => {
                write!(formatter, "capability admission {id} is not satisfied")
            }
            Self::UnexpectedCapabilityNeed(id) => {
                write!(formatter, "capability admission covers unexpected need {id}")
            }
            Self::DuplicateCapabilityCoverage(id) => {
                write!(formatter, "work order has duplicate coverage for capability need {id}")
            }
            Self::MissingCapabilityCoverage(id) => {
                write!(formatter, "work order lacks coverage for capability need {id}")
            }
            Self::InvalidCapabilityEvidenceDigest(digest) => write!(
                formatter,
                "capability evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::OrderCancelled(id) => write!(formatter, "work order {id} is cancelled"),
            Self::AlreadyReleased(id) => write!(formatter, "work order {id} is already released"),
            Self::InconsistentCancelledRelease(id) => write!(
                formatter,
                "cancelled work order {id} unexpectedly retains a release binding"
            ),
            Self::MissingSiteAdmission(id) => {
                write!(formatter, "released site admission {id} is missing")
            }
            Self::SiteAdmissionMismatch(id) => write!(
                formatter,
                "site admission {id} no longer matches the released work order"
            ),
            Self::RevisionOverflow => write!(formatter, "work-order revision overflow"),
            Self::Site(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for WorkOrderError {
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
        CapabilityAxisNeed, CapabilityAxisRange, CapabilityEnvelope, CapabilityNeed,
        MatterBinding, PlanDependency, PlanStep, ProcessExecution, ProcessKind, ProcessSpec,
        ProcessSpecId, Workpiece, WorkpieceLifecycle,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn actor() -> WorkActorRef {
        WorkActorRef {
            authority_id: id("authority:residents"),
            actor_id: id("resident:maintenance-tech"),
            actor_revision: 7,
        }
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

    fn workpiece() -> Workpiece {
        let mut value = Workpiece::new(
            symtropy_fabrication::WorkpieceId::new(id("workpiece:conduit")),
            vec![binding("conduit", 1)],
        )
        .unwrap();
        value.transition(WorkpieceLifecycle::Available).unwrap();
        value
    }

    fn plan(workpiece: &Workpiece, seal_need: Option<CapabilityNeedId>) -> FabricationPlan {
        let clean = PlanStep::new(
            PlanStepId::new(id("step:clean")),
            ProcessSpecId::new(id("process-spec:clean")),
            1,
            vec![workpiece.id.clone()],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let seal = PlanStep::new(
            PlanStepId::new(id("step:seal")),
            ProcessSpecId::new(id("process-spec:seal")),
            1,
            vec![workpiece.id.clone()],
            seal_need.into_iter().collect(),
            Vec::new(),
        )
        .unwrap();
        FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:patch-conduit")),
            1,
            vec![clean, seal],
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

    fn seal_need() -> CapabilityNeed {
        CapabilityNeed::new(
            CapabilityNeedId::new(id("capability-need:seal")),
            id("capability:controlled-sealing"),
            Some(id("mode:seal:field")),
            vec![
                CapabilityAxisNeed::new(id("axis:clamp-force-n"), 200, 400, Some(10)).unwrap(),
            ],
            vec![id("condition:surface-clean")],
        )
        .unwrap()
    }

    fn seal_admission(satisfied: bool) -> CapabilityAdmission {
        let need = seal_need();
        let envelope = CapabilityEnvelope::new(
            id("tool:field-clamp"),
            4,
            if satisfied {
                id("capability:controlled-sealing")
            } else {
                id("capability:wrong")
            },
            id("mode:seal:field"),
            vec![
                CapabilityAxisRange::new(id("axis:clamp-force-n"), 100, 500, 5).unwrap(),
            ],
            vec![id("condition:surface-clean")],
            CapabilityEvidenceRef::new(
                id("authority:tool-diagnostics"),
                id("evidence:field-clamp:4"),
                4,
                "digest:field-clamp:4",
            )
            .unwrap(),
        )
        .unwrap();
        need.evaluate(
            CapabilityAdmissionId::new(id(if satisfied {
                "capability-admission:seal:good"
            } else {
                "capability-admission:seal:bad"
            })),
            &envelope,
        )
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

    fn complete_clean(site: &mut ConstructionSite, plan: &FabricationPlan, workpiece: &Workpiece) {
        let execution_id = ProcessExecutionId::new(id("process-execution:clean"));
        site.admit_step(
            plan,
            SiteStepAdmissionId::new(id("site-admission:clean")),
            PlanStepId::new(id("step:clean")),
            execution_id.clone(),
        )
        .unwrap();
        let mut execution = ProcessExecution::begin(
            execution_id,
            &spec("clean", ProcessKind::Clean),
            &[workpiece],
            &[],
        )
        .unwrap();
        let evidence = execution
            .complete(
                id("matter:test"),
                id("process-evidence:clean"),
                2,
                "digest:process:clean",
                vec![binding("conduit-clean", 2)],
            )
            .unwrap();
        site.record_completion(plan, &evidence).unwrap();
    }

    #[test]
    fn capability_is_gated_at_release_and_traced_into_process_evidence() {
        let workpiece = workpiece();
        let need = seal_need();
        let plan = plan(&workpiece, Some(need.id.clone()));
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch-conduit")),
            &plan,
        );
        let mut order = WorkOrder::issue(
            WorkOrderId::new(id("work-order:seal")),
            &site,
            &plan,
            PlanStepId::new(id("step:seal")),
            actor(),
        )
        .unwrap();

        complete_clean(&mut site, &plan, &workpiece);

        let missing = order.release(
            &mut site,
            &plan,
            SiteStepAdmissionId::new(id("site-admission:seal:missing")),
            ProcessExecutionId::new(id("process-execution:seal:missing")),
            &[],
        );
        assert!(matches!(
            missing,
            Err(WorkOrderError::MissingCapabilityCoverage(_))
        ));

        let bad = seal_admission(false);
        let unsatisfied = order.release(
            &mut site,
            &plan,
            SiteStepAdmissionId::new(id("site-admission:seal:bad")),
            ProcessExecutionId::new(id("process-execution:seal:bad")),
            &[bad],
        );
        assert!(matches!(
            unsatisfied,
            Err(WorkOrderError::UnsatisfiedCapabilityAdmission(_))
        ));

        let good = seal_admission(true);
        let execution_id = ProcessExecutionId::new(id("process-execution:seal:good"));
        order
            .release(
                &mut site,
                &plan,
                SiteStepAdmissionId::new(id("site-admission:seal:good")),
                execution_id.clone(),
                &[good],
            )
            .unwrap();

        let release = order.release.as_ref().unwrap();
        assert_eq!(release.capability_admissions().len(), 1);
        let bootstrap = release.bootstrap_evidence();

        let mut execution = ProcessExecution::begin(
            execution_id,
            &spec("seal", ProcessKind::Seal),
            &[&workpiece],
            &bootstrap,
        )
        .unwrap();
        let evidence = execution
            .complete(
                id("matter:test"),
                id("process-evidence:seal"),
                3,
                "digest:process:seal",
                vec![binding("conduit-sealed", 3)],
            )
            .unwrap();
        site.record_completion(&plan, &evidence).unwrap();
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Completed);
    }

    #[test]
    fn work_can_be_scheduled_early_but_not_released_before_site_readiness() {
        let workpiece = workpiece();
        let plan = plan(&workpiece, None);
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch-conduit")),
            &plan,
        );
        let mut order = WorkOrder::issue(
            WorkOrderId::new(id("work-order:seal")),
            &site,
            &plan,
            PlanStepId::new(id("step:seal")),
            actor(),
        )
        .unwrap();

        let early = order.release(
            &mut site,
            &plan,
            SiteStepAdmissionId::new(id("site-admission:seal")),
            ProcessExecutionId::new(id("process-execution:seal")),
            &[],
        );
        assert!(matches!(
            early,
            Err(WorkOrderError::Site(SiteError::StepNotReady(_)))
        ));

        complete_clean(&mut site, &plan, &workpiece);
        order
            .release(
                &mut site,
                &plan,
                SiteStepAdmissionId::new(id("site-admission:seal")),
                ProcessExecutionId::new(id("process-execution:seal")),
                &[],
            )
            .unwrap();
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Released);
    }

    #[test]
    fn completion_is_derived_from_site_evidence_without_mutating_order() {
        let workpiece = workpiece();
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:single")),
            1,
            vec![
                PlanStep::new(
                    PlanStepId::new(id("step:clean")),
                    ProcessSpecId::new(id("process-spec:clean")),
                    1,
                    vec![workpiece.id.clone()],
                    Vec::new(),
                    Vec::new(),
                )
                .unwrap(),
            ],
            Vec::new(),
        )
        .unwrap();
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:single")),
            &plan,
        );
        let mut order = WorkOrder::issue(
            WorkOrderId::new(id("work-order:clean")),
            &site,
            &plan,
            PlanStepId::new(id("step:clean")),
            actor(),
        )
        .unwrap();
        let execution_id = ProcessExecutionId::new(id("process-execution:clean"));
        order
            .release(
                &mut site,
                &plan,
                SiteStepAdmissionId::new(id("site-admission:clean")),
                execution_id.clone(),
                &[],
            )
            .unwrap();
        let order_revision_after_release = order.revision;

        let mut execution = ProcessExecution::begin(
            execution_id,
            &spec("clean", ProcessKind::Clean),
            &[&workpiece],
            &[],
        )
        .unwrap();
        let evidence = execution
            .complete(
                id("matter:test"),
                id("process-evidence:clean"),
                2,
                "digest:process:clean",
                vec![binding("clean-result", 2)],
            )
            .unwrap();
        site.record_completion(&plan, &evidence).unwrap();

        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Completed);
        assert_eq!(order.revision, order_revision_after_release);
    }

    #[test]
    fn cancelling_unreleased_order_does_not_mutate_site() {
        let workpiece = workpiece();
        let plan = plan(&workpiece, None);
        let site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch-conduit")),
            &plan,
        );
        let site_revision = site.revision;
        let mut order = WorkOrder::issue(
            WorkOrderId::new(id("work-order:clean")),
            &site,
            &plan,
            PlanStepId::new(id("step:clean")),
            actor(),
        )
        .unwrap();

        order
            .cancel_before_release(id("reason:schedule-changed"))
            .unwrap();
        assert_eq!(site.revision, site_revision);
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Cancelled);
    }

    #[test]
    fn serialization_contains_no_competing_completion_or_authority_flags() {
        let workpiece = workpiece();
        let plan = plan(&workpiece, None);
        let site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch-conduit")),
            &plan,
        );
        let order = WorkOrder::issue(
            WorkOrderId::new(id("work-order:clean")),
            &site,
            &plan,
            PlanStepId::new(id("step:clean")),
            actor(),
        )
        .unwrap();
        let value = serde_json::to_value(order).unwrap();
        for forbidden in [
            "progress",
            "physical_complete",
            "technical_ready",
            "authorized",
            "commissioned",
            "quality",
            "score",
        ] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }
}

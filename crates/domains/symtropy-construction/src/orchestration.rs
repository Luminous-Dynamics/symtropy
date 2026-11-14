// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public work-order facade for staged construction execution.
//!
//! The raw C2 `WorkOrder` remains an internal scheduling/capability primitive.
//! External consumers can issue, inspect, cancel, and obtain admitted F4
//! bootstrap evidence through this wrapper, but only C3 staging may release it.

use serde::{Deserialize, Serialize};
use symtropy_fabrication::{
    CapabilityAdmission, CapabilityEvidence, FabricationPlan, PlanStepId, ProcessExecutionId,
};
use symtropy_game_state::StableId;

use crate::{
    ConstructionSite, SiteStepAdmissionId, WorkActorRef, WorkOrder, WorkOrderError, WorkOrderId,
    WorkOrderStatus,
};

/// Public scheduled-work handle. Its internal C2 order is intentionally opaque
/// so callers cannot bypass the C3 staged-release boundary. Serialization keeps
/// the same durable C2 scheduling/release record across save/load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledWorkOrder {
    inner: WorkOrder,
}

impl ScheduledWorkOrder {
    pub fn issue(
        id: WorkOrderId,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        step_id: PlanStepId,
        responsible_actor: WorkActorRef,
    ) -> Result<Self, WorkOrderError> {
        Ok(Self {
            inner: WorkOrder::issue(id, site, plan, step_id, responsible_actor)?,
        })
    }

    pub fn id(&self) -> &WorkOrderId {
        &self.inner.id
    }

    pub fn revision(&self) -> u64 {
        self.inner.revision
    }

    pub fn step_id(&self) -> &PlanStepId {
        &self.inner.step_id
    }

    pub fn responsible_actor(&self) -> &WorkActorRef {
        &self.inner.responsible_actor
    }

    pub fn cancel_before_release(&mut self, reason_id: StableId) -> Result<(), WorkOrderError> {
        self.inner.cancel_before_release(reason_id)
    }

    pub fn status(&self, site: &ConstructionSite) -> Result<WorkOrderStatus, WorkOrderError> {
        self.inner.status(site)
    }

    /// Returns the exact F4 bootstrap tokens retained by the successful C2
    /// release. `None` means the staged work order has not been released.
    pub fn bootstrap_evidence(&self) -> Option<Vec<CapabilityEvidence>> {
        self.inner
            .release
            .as_ref()
            .map(|release| release.bootstrap_evidence())
    }

    pub fn released_execution_id(&self) -> Option<&ProcessExecutionId> {
        self.inner
            .release
            .as_ref()
            .map(|release| &release.execution_id)
    }

    pub fn released_site_admission_id(&self) -> Option<&SiteStepAdmissionId> {
        self.inner
            .release
            .as_ref()
            .map(|release| &release.site_admission_id)
    }

    /// Crate-internal bridge used only by C3 after staging has been validated.
    pub(crate) fn release_after_staging(
        &mut self,
        site: &mut ConstructionSite,
        plan: &FabricationPlan,
        site_admission_id: SiteStepAdmissionId,
        execution_id: ProcessExecutionId,
        capability_admissions: &[CapabilityAdmission],
    ) -> Result<(), WorkOrderError> {
        self.inner.release(
            site,
            plan,
            site_admission_id,
            execution_id,
            capability_admissions,
        )
    }

    #[cfg(test)]
    pub(crate) fn is_released(&self) -> bool {
        self.inner.release.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{
        FabricationPlanId, MatterBinding, PlanStep, ProcessSpecId, Workpiece, WorkpieceId,
        WorkpieceLifecycle,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn fixture() -> (Workpiece, FabricationPlan, ConstructionSite) {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:patch")),
            vec![
                MatterBinding::new(
                    id("matter:test"),
                    id("allocation:patch"),
                    1,
                    "digest:patch:1",
                )
                .unwrap(),
            ],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:patch")),
            1,
            vec![
                PlanStep::new(
                    PlanStepId::new(id("step:align")),
                    ProcessSpecId::new(id("process-spec:align")),
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
        let site = ConstructionSite::new(
            crate::ConstructionSiteId::new(id("construction-site:patch")),
            &plan,
        );
        (workpiece, plan, site)
    }

    #[test]
    fn facade_exposes_scheduling_but_starts_unreleased_and_round_trips() {
        let (_workpiece, plan, site) = fixture();
        let order = ScheduledWorkOrder::issue(
            WorkOrderId::new(id("work-order:align")),
            &site,
            &plan,
            PlanStepId::new(id("step:align")),
            WorkActorRef {
                authority_id: id("authority:residents"),
                actor_id: id("resident:tech"),
                actor_revision: 1,
            },
        )
        .unwrap();

        assert!(!order.is_released());
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Issued);
        assert!(order.bootstrap_evidence().is_none());

        let json = serde_json::to_string(&order).unwrap();
        let restored: ScheduledWorkOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, order);
    }
}

// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Release-to-process input continuity for construction execution.
//!
//! C3 proves that exact workpieces were staged when work was released. F4 later
//! records the actual before-image used when `ProcessExecution` began. This
//! layer requires the independently observed identity, lifecycle, and conserved-
//! matter bindings to be identical before C1 may accept completion.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_fabrication::{
    FabricationPlan, MatterBinding, PlanStepId, ProcessEvidence, ProcessExecutionId,
    ProcessInputSnapshot, WorkpieceId, WorkpieceLifecycle,
};

use crate::{ConstructionSite, ConstructionStagingLedger, SiteError, StagingReleaseRecord};

/// Canonical comparison view of one released/started process input before-image.
///
/// Construction neither owns nor mutates these facts. `schema_version` is
/// deliberately absent because F4 does not independently record it; C4 proves
/// only facts present on both sides of the release/process boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuityInputSnapshot {
    pub workpiece_id: WorkpieceId,
    pub lifecycle: WorkpieceLifecycle,
    pub matter_bindings: Vec<MatterBinding>,
}

impl ContinuityInputSnapshot {
    fn from_release(
        workpiece_id: WorkpieceId,
        lifecycle: WorkpieceLifecycle,
        matter_bindings: &[MatterBinding],
    ) -> Self {
        let mut matter_bindings = matter_bindings.to_vec();
        canonicalize_bindings(&mut matter_bindings);
        Self {
            workpiece_id,
            lifecycle,
            matter_bindings,
        }
    }

    fn from_process(input: &ProcessInputSnapshot) -> Self {
        let mut matter_bindings = input.matter_bindings.clone();
        canonicalize_bindings(&mut matter_bindings);
        Self {
            workpiece_id: input.workpiece_id.clone(),
            lifecycle: input.lifecycle,
            matter_bindings,
        }
    }
}

/// Validates the exact independently observable before-image admitted by C3
/// against the exact before-image captured by F4, then delegates completion to
/// C1. A mismatch leaves the C1 site admission open and cannot advance the plan.
pub fn record_released_process_completion(
    staging: &ConstructionStagingLedger,
    site: &mut ConstructionSite,
    plan: &FabricationPlan,
    evidence: &ProcessEvidence,
) -> Result<PlanStepId, ContinuityError> {
    let release = unique_release_for_execution(staging, site, plan, &evidence.execution_id)?;
    let expected = expected_release_inputs(staging, release)?;
    let actual = canonical_process_inputs(&evidence.inputs)?;

    if expected != actual {
        return Err(ContinuityError::InputSnapshotMismatch {
            execution_id: evidence.execution_id.clone(),
            expected,
            actual,
        });
    }

    site.record_completion(plan, evidence)
        .map_err(ContinuityError::Site)
}

fn unique_release_for_execution<'a>(
    staging: &'a ConstructionStagingLedger,
    site: &ConstructionSite,
    plan: &FabricationPlan,
    execution_id: &ProcessExecutionId,
) -> Result<&'a StagingReleaseRecord, ContinuityError> {
    let matching = staging
        .releases()
        .iter()
        .filter(|release| {
            release.execution_id == *execution_id
                && release.site_id == site.id
                && release.plan_id == plan.id
                && release.plan_revision == plan.revision
        })
        .collect::<Vec<_>>();

    match matching.as_slice() {
        [] => Err(ContinuityError::MissingRelease(execution_id.clone())),
        [release] => Ok(*release),
        _ => Err(ContinuityError::AmbiguousRelease(execution_id.clone())),
    }
}

fn expected_release_inputs(
    staging: &ConstructionStagingLedger,
    release: &StagingReleaseRecord,
) -> Result<Vec<ContinuityInputSnapshot>, ContinuityError> {
    let mut expected = Vec::with_capacity(release.staged_workpieces().len());

    for staged in release.staged_workpieces() {
        let reservation = staging
            .reservation(&staged.reservation_id)
            .ok_or_else(|| ContinuityError::MissingReservation(staged.reservation_id.clone()))?;
        if reservation.snapshot.workpiece_id != staged.workpiece_id {
            return Err(ContinuityError::ReleaseReservationMismatch {
                execution_id: release.execution_id.clone(),
                expected: reservation.snapshot.workpiece_id.clone(),
                actual: staged.workpiece_id.clone(),
            });
        }
        expected.push(ContinuityInputSnapshot::from_release(
            reservation.snapshot.workpiece_id.clone(),
            reservation.snapshot.lifecycle,
            reservation.snapshot.matter_bindings(),
        ));
    }

    canonicalize_inputs(&mut expected, ContinuityError::DuplicateReleaseWorkpiece)?;
    Ok(expected)
}

fn canonical_process_inputs(
    actual: &[ProcessInputSnapshot],
) -> Result<Vec<ContinuityInputSnapshot>, ContinuityError> {
    let mut canonical = actual
        .iter()
        .map(ContinuityInputSnapshot::from_process)
        .collect::<Vec<_>>();
    canonicalize_inputs(&mut canonical, ContinuityError::DuplicateProcessWorkpiece)?;
    Ok(canonical)
}

fn canonicalize_inputs(
    inputs: &mut [ContinuityInputSnapshot],
    duplicate: fn(WorkpieceId) -> ContinuityError,
) -> Result<(), ContinuityError> {
    inputs.sort_by(|left, right| left.workpiece_id.cmp(&right.workpiece_id));
    for pair in inputs.windows(2) {
        if pair[0].workpiece_id == pair[1].workpiece_id {
            return Err(duplicate(pair[0].workpiece_id.clone()));
        }
    }
    Ok(())
}

fn canonicalize_bindings(bindings: &mut [MatterBinding]) {
    bindings.sort_by(|left, right| {
        (
            &left.authority_id,
            &left.allocation_id,
            left.revision,
            &left.binding_digest,
        )
            .cmp(&(
                &right.authority_id,
                &right.allocation_id,
                right.revision,
                &right.binding_digest,
            ))
    });
}

#[derive(Debug)]
pub enum ContinuityError {
    MissingRelease(ProcessExecutionId),
    AmbiguousRelease(ProcessExecutionId),
    MissingReservation(crate::StagingReservationId),
    ReleaseReservationMismatch {
        execution_id: ProcessExecutionId,
        expected: WorkpieceId,
        actual: WorkpieceId,
    },
    DuplicateReleaseWorkpiece(WorkpieceId),
    DuplicateProcessWorkpiece(WorkpieceId),
    InputSnapshotMismatch {
        execution_id: ProcessExecutionId,
        expected: Vec<ContinuityInputSnapshot>,
        actual: Vec<ContinuityInputSnapshot>,
    },
    Site(SiteError),
}

impl fmt::Display for ContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRelease(id) => {
                write!(formatter, "process execution {id} has no staged construction release")
            }
            Self::AmbiguousRelease(id) => write!(
                formatter,
                "process execution {id} matches more than one staged construction release"
            ),
            Self::MissingReservation(id) => {
                write!(formatter, "staged release references missing reservation {id}")
            }
            Self::ReleaseReservationMismatch {
                execution_id,
                expected,
                actual,
            } => write!(
                formatter,
                "process execution {execution_id} release reservation binds workpiece {expected}, release record names {actual}"
            ),
            Self::DuplicateReleaseWorkpiece(id) => write!(
                formatter,
                "staged release repeats workpiece {id} while reconstructing input continuity"
            ),
            Self::DuplicateProcessWorkpiece(id) => write!(
                formatter,
                "process evidence repeats input workpiece {id}"
            ),
            Self::InputSnapshotMismatch {
                execution_id,
                expected,
                actual,
            } => write!(
                formatter,
                "process execution {execution_id} began from a different before-image than staged release: expected {expected:?}, got {actual:?}"
            ),
            Self::Site(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for ContinuityError {
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
    use crate::{
        ConstructionSiteId, PlacementEvidenceRef, ScheduledWorkOrder, SiteStepAdmissionId,
        StagingReleaseInput, StagingReleaseRecordId, StagingReservationId, WorkActorRef,
        WorkOrderId, WorkOrderStatus,
    };
    use symtropy_fabrication::{
        FabricationPlanId, PlanStep, ProcessExecution, ProcessKind, ProcessSpec, ProcessSpecId,
        Workpiece,
    };
    use symtropy_game_state::StableId;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn binding(authority: &str, allocation: &str, revision: u64) -> MatterBinding {
        MatterBinding::new(
            id(authority),
            id(allocation),
            revision,
            format!("digest:{authority}:{allocation}:{revision}"),
        )
        .unwrap()
    }

    fn workpiece(revision: u64) -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:patch")),
            vec![binding("matter:test", "allocation:patch", revision)],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn multi_binding_workpiece() -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:patch")),
            vec![
                binding("matter:z", "allocation:z", 4),
                binding("matter:a", "allocation:a", 2),
            ],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn plan(workpiece: &Workpiece) -> FabricationPlan {
        FabricationPlan::new(
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
        .unwrap()
    }

    fn placement(workpiece: &Workpiece, revision: u64) -> PlacementEvidenceRef {
        PlacementEvidenceRef::new(
            id("authority:logistics"),
            id(&format!("placement:{revision}")),
            workpiece.id.stable_id().clone(),
            revision,
            id("location:site-bench"),
            format!("digest:placement:{revision}"),
        )
        .unwrap()
    }

    fn release(
        staging: &mut ConstructionStagingLedger,
        site: &mut ConstructionSite,
        plan: &FabricationPlan,
        workpiece: &Workpiece,
        execution_id: ProcessExecutionId,
    ) -> ScheduledWorkOrder {
        let reservation_id = StagingReservationId::new(id("staging:patch"));
        staging
            .reserve(
                reservation_id.clone(),
                site,
                plan,
                workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(workpiece, 7),
            )
            .unwrap();

        let mut order = ScheduledWorkOrder::issue(
            WorkOrderId::new(id("work-order:align")),
            site,
            plan,
            PlanStepId::new(id("step:align")),
            WorkActorRef {
                authority_id: id("authority:residents"),
                actor_id: id("resident:tech"),
                actor_revision: 1,
            },
        )
        .unwrap();

        staging
            .release_work_order(
                StagingReleaseRecordId::new(id("staging-release:align")),
                &mut order,
                site,
                plan,
                SiteStepAdmissionId::new(id("site-admission:align")),
                execution_id,
                &[],
                vec![StagingReleaseInput {
                    reservation_id: &reservation_id,
                    workpiece,
                    placement: placement(workpiece, 8),
                }],
            )
            .unwrap();
        order
    }

    fn align_spec(states: Vec<WorkpieceLifecycle>) -> ProcessSpec {
        ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:align")),
            1,
            ProcessKind::Align,
            Vec::new(),
            states,
        )
        .unwrap()
    }

    fn complete(
        execution_id: ProcessExecutionId,
        workpiece: &Workpiece,
        states: Vec<WorkpieceLifecycle>,
        evidence_name: &str,
    ) -> ProcessEvidence {
        let mut execution = ProcessExecution::begin(
            execution_id,
            &align_spec(states),
            &[workpiece],
            &[],
        )
        .unwrap();
        execution
            .complete(
                id("matter:test"),
                id(&format!("process-evidence:{evidence_name}")),
                2,
                format!("digest:process:{evidence_name}"),
                vec![binding("matter:test", "allocation:result", 2)],
            )
            .unwrap()
    }

    #[test]
    fn matter_revision_drift_between_release_and_process_begin_is_rejected() {
        let released = workpiece(1);
        let plan = plan(&released);
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch")),
            &plan,
        );
        let mut staging = ConstructionStagingLedger::default();
        let execution_id = ProcessExecutionId::new(id("process-execution:align"));
        let order = release(
            &mut staging,
            &mut site,
            &plan,
            &released,
            execution_id.clone(),
        );

        let changed = workpiece(2);
        let evidence = complete(
            execution_id,
            &changed,
            vec![WorkpieceLifecycle::Available],
            "changed-matter",
        );
        let result = record_released_process_completion(&staging, &mut site, &plan, &evidence);

        assert!(matches!(
            result,
            Err(ContinuityError::InputSnapshotMismatch { .. })
        ));
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Released);
        assert!(site.completed_step_ids().is_empty());
    }

    #[test]
    fn lifecycle_drift_with_same_matter_and_identity_is_rejected() {
        let released = workpiece(1);
        let plan = plan(&released);
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch")),
            &plan,
        );
        let mut staging = ConstructionStagingLedger::default();
        let execution_id = ProcessExecutionId::new(id("process-execution:align"));
        release(
            &mut staging,
            &mut site,
            &plan,
            &released,
            execution_id.clone(),
        );

        let mut changed = released.clone();
        changed.transition(WorkpieceLifecycle::Installed).unwrap();
        let evidence = complete(
            execution_id,
            &changed,
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
            "changed-lifecycle",
        );
        let result = record_released_process_completion(&staging, &mut site, &plan, &evidence);

        assert!(matches!(
            result,
            Err(ContinuityError::InputSnapshotMismatch { .. })
        ));
        assert!(site.completed_step_ids().is_empty());
    }

    #[test]
    fn semantically_equal_binding_sets_are_order_independent() {
        let released = multi_binding_workpiece();
        let plan = plan(&released);
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch")),
            &plan,
        );
        let mut staging = ConstructionStagingLedger::default();
        let execution_id = ProcessExecutionId::new(id("process-execution:align"));
        let order = release(
            &mut staging,
            &mut site,
            &plan,
            &released,
            execution_id.clone(),
        );

        // F4 preserves caller order while C3 canonicalizes its reservation
        // snapshot. Continuity compares the semantic binding set, not vector order.
        let evidence = complete(
            execution_id,
            &released,
            vec![WorkpieceLifecycle::Available],
            "same-inputs",
        );
        record_released_process_completion(&staging, &mut site, &plan, &evidence).unwrap();

        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Completed);
        assert_eq!(site.completed_step_ids(), vec![PlanStepId::new(id("step:align"))]);
    }

    #[test]
    fn completion_without_staged_release_is_rejected_before_c1() {
        let workpiece = workpiece(1);
        let plan = plan(&workpiece);
        let mut site = ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch")),
            &plan,
        );
        let staging = ConstructionStagingLedger::default();
        let execution_id = ProcessExecutionId::new(id("process-execution:align"));
        let evidence = complete(
            execution_id.clone(),
            &workpiece,
            vec![WorkpieceLifecycle::Available],
            "orphan",
        );

        let result = record_released_process_completion(&staging, &mut site, &plan, &evidence);
        assert!(matches!(
            result,
            Err(ContinuityError::MissingRelease(id)) if id == execution_id
        ));
        assert!(site.admissions().is_empty());
    }
}

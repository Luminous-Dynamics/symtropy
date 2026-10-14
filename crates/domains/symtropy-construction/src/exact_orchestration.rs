// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Plan-free C2/C3/C4 orchestration over [`ExactConstructionSite`].
//!
//! The legacy construction primitives still accept an authored `FabricationPlan`
//! during migration. This facade is the stronger public execution path: it
//! derives that plan only from the site-owned `ExecutableFabricationPlan`, so a
//! caller cannot substitute another plan body with the same identity/revision.
//! Completion accepts only [`ValidatedProcessEvidence`] and proves C3 -> F4
//! before-image continuity before delegating to exact C1 completion.

use std::{error::Error, fmt};
use symtropy_fabrication::{
    CapabilityAdmission, MatterBinding, PlanStepId, ProcessExecutionId, ValidatedProcessEvidence,
    Workpiece, WorkpieceId,
};
use symtropy_game_state::StableId;

use crate::{
    ConstructionStagingLedger, ContinuityInputSnapshot, ExactConstructionSite, ExactSiteError,
    PlacementEvidenceRef, ScheduledWorkOrder, SiteStepAdmissionId, StagingError,
    StagingReleaseInput, StagingReleaseRecord, StagingReleaseRecordId, StagingReservation,
    StagingReservationId, WorkActorRef, WorkOrderError, WorkOrderId, WorkOrderStatus,
};

/// Issues one C2 work order against the exact plan already owned by `site`.
/// No caller-supplied plan value crosses this boundary.
pub fn issue_exact_work_order(
    id: WorkOrderId,
    site: &ExactConstructionSite,
    step_id: PlanStepId,
    responsible_actor: WorkActorRef,
) -> Result<ScheduledWorkOrder, WorkOrderError> {
    ScheduledWorkOrder::issue(
        id,
        site.legacy_site(),
        site.executable_plan().plan(),
        step_id,
        responsible_actor,
    )
}

/// Derives C2 status from the exact site's underlying C1 admission ledger.
pub fn exact_work_order_status(
    order: &ScheduledWorkOrder,
    site: &ExactConstructionSite,
) -> Result<WorkOrderStatus, WorkOrderError> {
    order.status(site.legacy_site())
}

/// Reserves one exact workpiece for exact site-owned plan steps without
/// accepting a replacement plan object from the caller.
pub fn reserve_exact_workpiece(
    staging: &mut ConstructionStagingLedger,
    id: StagingReservationId,
    site: &ExactConstructionSite,
    workpiece: &Workpiece,
    step_ids: Vec<PlanStepId>,
    placement: PlacementEvidenceRef,
) -> Result<(), StagingError> {
    staging.reserve(
        id,
        site.legacy_site(),
        site.executable_plan().plan(),
        workpiece,
        step_ids,
        placement,
    )
}

/// Performs C3 staged release and C2 capability release against only the exact
/// plan owned by `site`.
///
/// The internal plan clone exists solely to satisfy the legacy mutable-borrow
/// API while it is being retired. It is derived from the site-owned exact
/// contract and is never supplied by the caller.
#[allow(clippy::too_many_arguments)]
pub fn release_exact_work_order(
    staging: &mut ConstructionStagingLedger,
    release_id: StagingReleaseRecordId,
    order: &mut ScheduledWorkOrder,
    site: &mut ExactConstructionSite,
    site_admission_id: SiteStepAdmissionId,
    execution_id: ProcessExecutionId,
    capability_admissions: &[CapabilityAdmission],
    inputs: Vec<StagingReleaseInput<'_>>,
) -> Result<(), StagingError> {
    let plan = site.executable_plan().plan().clone();
    staging.release_work_order(
        release_id,
        order,
        site.legacy_site_mut(),
        &plan,
        site_admission_id,
        execution_id,
        capability_admissions,
        inputs,
    )
}

/// Proves the exact physical before-image released by C3 is the exact
/// before-image recorded when F4 execution began, then delegates completion to
/// `ExactConstructionSite`, which independently requires full process-snapshot
/// equality with the site-owned executable plan.
pub fn record_exact_released_process_completion(
    staging: &ConstructionStagingLedger,
    site: &mut ExactConstructionSite,
    evidence: &ValidatedProcessEvidence,
) -> Result<PlanStepId, ExactContinuityError> {
    let release = unique_exact_release_for_execution(staging, site, evidence.execution_id())?;
    validate_release_admission(site, release)?;
    let expected = exact_release_inputs(staging, site, release)?;
    let actual = exact_process_inputs(evidence)?;

    if expected != actual {
        return Err(ExactContinuityError::InputSnapshotMismatch {
            execution_id: evidence.execution_id().clone(),
            expected,
            actual,
        });
    }

    site.record_completion(evidence)
        .map_err(ExactContinuityError::ExactSite)
}

fn unique_exact_release_for_execution<'a>(
    staging: &'a ConstructionStagingLedger,
    site: &ExactConstructionSite,
    execution_id: &ProcessExecutionId,
) -> Result<&'a StagingReleaseRecord, ExactContinuityError> {
    let matching = staging
        .releases()
        .iter()
        .filter(|release| {
            release.execution_id == *execution_id
                && &release.site_id == site.id()
                && &release.plan_id == site.plan_id()
                && release.plan_revision == site.plan_revision()
        })
        .collect::<Vec<_>>();

    match matching.as_slice() {
        [] => Err(ExactContinuityError::MissingRelease(execution_id.clone())),
        [release] => Ok(*release),
        _ => Err(ExactContinuityError::AmbiguousRelease(
            execution_id.clone(),
        )),
    }
}

fn validate_release_admission(
    site: &ExactConstructionSite,
    release: &StagingReleaseRecord,
) -> Result<(), ExactContinuityError> {
    let admission = site
        .admissions()
        .iter()
        .find(|admission| admission.id == release.site_admission_id)
        .ok_or_else(|| {
            ExactContinuityError::MissingSiteAdmission(release.site_admission_id.clone())
        })?;

    if admission.execution_id != release.execution_id || admission.step_id != release.step_id {
        return Err(ExactContinuityError::ReleaseAdmissionMismatch {
            site_admission_id: release.site_admission_id.clone(),
            release_step_id: release.step_id.clone(),
            admission_step_id: admission.step_id.clone(),
            release_execution_id: release.execution_id.clone(),
            admission_execution_id: admission.execution_id.clone(),
        });
    }
    Ok(())
}

fn exact_release_inputs(
    staging: &ConstructionStagingLedger,
    site: &ExactConstructionSite,
    release: &StagingReleaseRecord,
) -> Result<Vec<ContinuityInputSnapshot>, ExactContinuityError> {
    let mut expected = Vec::with_capacity(release.staged_workpieces().len());

    for staged in release.staged_workpieces() {
        let reservation = staging
            .reservation(&staged.reservation_id)
            .ok_or_else(|| {
                ExactContinuityError::MissingReservation(staged.reservation_id.clone())
            })?;
        validate_release_reservation(site, release, reservation, staged.workpiece_id.clone())?;
        validate_release_placement(reservation, staged)?;

        let mut matter_bindings = reservation.snapshot.matter_bindings().to_vec();
        canonicalize_validated_bindings(&mut matter_bindings)?;
        expected.push(ContinuityInputSnapshot {
            workpiece_id: reservation.snapshot.workpiece_id.clone(),
            lifecycle: reservation.snapshot.lifecycle,
            matter_bindings,
        });
    }

    canonicalize_inputs(
        &mut expected,
        ExactContinuityError::DuplicateReleaseWorkpiece,
    )?;
    Ok(expected)
}

fn validate_release_reservation(
    site: &ExactConstructionSite,
    release: &StagingReleaseRecord,
    reservation: &StagingReservation,
    released_workpiece_id: WorkpieceId,
) -> Result<(), ExactContinuityError> {
    if reservation.site_id != release.site_id
        || &reservation.site_id != site.id()
        || reservation.plan_id != release.plan_id
        || &reservation.plan_id != site.plan_id()
        || reservation.plan_revision != release.plan_revision
        || reservation.plan_revision != site.plan_revision()
    {
        return Err(ExactContinuityError::ReservationContextMismatch(
            reservation.id.clone(),
        ));
    }
    if !reservation.step_ids().contains(&release.step_id) {
        return Err(ExactContinuityError::ReservationDoesNotCoverStep {
            reservation_id: reservation.id.clone(),
            step_id: release.step_id.clone(),
        });
    }
    if reservation.snapshot.workpiece_id != released_workpiece_id {
        return Err(ExactContinuityError::ReleaseReservationMismatch {
            execution_id: release.execution_id.clone(),
            expected: reservation.snapshot.workpiece_id.clone(),
            actual: released_workpiece_id,
        });
    }
    Ok(())
}

fn validate_release_placement(
    reservation: &StagingReservation,
    staged: &crate::ConfirmedStagedWorkpiece,
) -> Result<(), ExactContinuityError> {
    let placement = &staged.placement_evidence;
    validate_digest(&reservation.placement_at_reservation.digest)?;
    validate_digest(&placement.digest)?;

    if &reservation.placement_at_reservation.subject_id != staged.workpiece_id.stable_id()
        || &placement.subject_id != staged.workpiece_id.stable_id()
    {
        return Err(ExactContinuityError::PlacementSubjectMismatch(
            staged.reservation_id.clone(),
        ));
    }
    if placement.authority_id != reservation.placement_at_reservation.authority_id
        || placement.location_id != reservation.placement_at_reservation.location_id
    {
        return Err(ExactContinuityError::PlacementContextMismatch(
            staged.reservation_id.clone(),
        ));
    }
    if placement.subject_revision < reservation.placement_at_reservation.subject_revision {
        return Err(ExactContinuityError::StalePlacementEvidence(
            staged.reservation_id.clone(),
        ));
    }
    if placement.subject_revision == reservation.placement_at_reservation.subject_revision
        && placement.digest != reservation.placement_at_reservation.digest
    {
        return Err(ExactContinuityError::ConflictingPlacementEvidence(
            staged.reservation_id.clone(),
        ));
    }
    Ok(())
}

fn exact_process_inputs(
    evidence: &ValidatedProcessEvidence,
) -> Result<Vec<ContinuityInputSnapshot>, ExactContinuityError> {
    let mut actual = Vec::with_capacity(evidence.inputs().len());
    for input in evidence.inputs() {
        let mut matter_bindings = input.matter_bindings.clone();
        canonicalize_validated_bindings(&mut matter_bindings)?;
        actual.push(ContinuityInputSnapshot {
            workpiece_id: input.workpiece_id.clone(),
            lifecycle: input.lifecycle,
            matter_bindings,
        });
    }
    canonicalize_inputs(
        &mut actual,
        ExactContinuityError::DuplicateProcessWorkpiece,
    )?;
    Ok(actual)
}

fn canonicalize_inputs(
    inputs: &mut [ContinuityInputSnapshot],
    duplicate: fn(WorkpieceId) -> ExactContinuityError,
) -> Result<(), ExactContinuityError> {
    inputs.sort_by(|left, right| left.workpiece_id.cmp(&right.workpiece_id));
    for pair in inputs.windows(2) {
        if pair[0].workpiece_id == pair[1].workpiece_id {
            return Err(duplicate(pair[0].workpiece_id.clone()));
        }
    }
    Ok(())
}

fn canonicalize_validated_bindings(
    bindings: &mut [MatterBinding],
) -> Result<(), ExactContinuityError> {
    for binding in bindings.iter() {
        validate_digest(&binding.binding_digest)?;
    }
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
    for pair in bindings.windows(2) {
        if pair[0].authority_id == pair[1].authority_id
            && pair[0].allocation_id == pair[1].allocation_id
        {
            return Err(ExactContinuityError::DuplicateMatterAllocation {
                authority_id: pair[0].authority_id.clone(),
                allocation_id: pair[0].allocation_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), ExactContinuityError> {
    if digest.is_empty() || digest.len() > 256 {
        Err(ExactContinuityError::InvalidDigest(digest.to_owned()))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub enum ExactContinuityError {
    MissingRelease(ProcessExecutionId),
    AmbiguousRelease(ProcessExecutionId),
    MissingSiteAdmission(SiteStepAdmissionId),
    ReleaseAdmissionMismatch {
        site_admission_id: SiteStepAdmissionId,
        release_step_id: PlanStepId,
        admission_step_id: PlanStepId,
        release_execution_id: ProcessExecutionId,
        admission_execution_id: ProcessExecutionId,
    },
    MissingReservation(StagingReservationId),
    ReservationContextMismatch(StagingReservationId),
    ReservationDoesNotCoverStep {
        reservation_id: StagingReservationId,
        step_id: PlanStepId,
    },
    ReleaseReservationMismatch {
        execution_id: ProcessExecutionId,
        expected: WorkpieceId,
        actual: WorkpieceId,
    },
    PlacementSubjectMismatch(StagingReservationId),
    PlacementContextMismatch(StagingReservationId),
    StalePlacementEvidence(StagingReservationId),
    ConflictingPlacementEvidence(StagingReservationId),
    DuplicateReleaseWorkpiece(WorkpieceId),
    DuplicateProcessWorkpiece(WorkpieceId),
    DuplicateMatterAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
    InvalidDigest(String),
    InputSnapshotMismatch {
        execution_id: ProcessExecutionId,
        expected: Vec<ContinuityInputSnapshot>,
        actual: Vec<ContinuityInputSnapshot>,
    },
    ExactSite(ExactSiteError),
}

impl fmt::Display for ExactContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRelease(id) => {
                write!(formatter, "process execution {id} has no exact staged release")
            }
            Self::AmbiguousRelease(id) => write!(
                formatter,
                "process execution {id} matches more than one exact staged release"
            ),
            Self::MissingSiteAdmission(id) => {
                write!(formatter, "staged release references missing site admission {id}")
            }
            Self::ReleaseAdmissionMismatch {
                site_admission_id,
                release_step_id,
                admission_step_id,
                release_execution_id,
                admission_execution_id,
            } => write!(
                formatter,
                "staged release admission {site_admission_id} disagrees with exact C1 context: step {release_step_id}/{admission_step_id}, execution {release_execution_id}/{admission_execution_id}"
            ),
            Self::MissingReservation(id) => {
                write!(formatter, "staged release references missing reservation {id}")
            }
            Self::ReservationContextMismatch(id) => write!(
                formatter,
                "staging reservation {id} disagrees with exact site/plan context"
            ),
            Self::ReservationDoesNotCoverStep {
                reservation_id,
                step_id,
            } => write!(
                formatter,
                "staging reservation {reservation_id} does not cover exact step {step_id}"
            ),
            Self::ReleaseReservationMismatch {
                execution_id,
                expected,
                actual,
            } => write!(
                formatter,
                "process execution {execution_id} release reservation binds workpiece {expected}, release record names {actual}"
            ),
            Self::PlacementSubjectMismatch(id) => write!(
                formatter,
                "staging reservation {id} placement subject disagrees with released workpiece"
            ),
            Self::PlacementContextMismatch(id) => write!(
                formatter,
                "staging reservation {id} placement authority/location changed"
            ),
            Self::StalePlacementEvidence(id) => {
                write!(formatter, "staging reservation {id} uses stale placement evidence")
            }
            Self::ConflictingPlacementEvidence(id) => write!(
                formatter,
                "staging reservation {id} has conflicting same-revision placement evidence"
            ),
            Self::DuplicateReleaseWorkpiece(id) => {
                write!(formatter, "exact staged release repeats workpiece {id}")
            }
            Self::DuplicateProcessWorkpiece(id) => {
                write!(formatter, "validated process evidence repeats workpiece {id}")
            }
            Self::DuplicateMatterAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "exact continuity input repeats matter allocation {authority_id}/{allocation_id}"
            ),
            Self::InvalidDigest(digest) => write!(
                formatter,
                "exact continuity digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::InputSnapshotMismatch {
                execution_id,
                expected,
                actual,
            } => write!(
                formatter,
                "process execution {execution_id} began from a different before-image than exact staged release: expected {expected:?}, got {actual:?}"
            ),
            Self::ExactSite(error) => error.fmt(formatter),
        }
    }
}

impl Error for ExactContinuityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExactSite(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{
        ExactPlanProcessBinding, ExecutableFabricationPlan, FabricationPlan, FabricationPlanId,
        MatterBinding, PlanStep, ProcessExecution, ProcessKind, ProcessSpec, ProcessSpecId,
        WorkpieceLifecycle,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn workpiece(revision: u64) -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:exact-orchestration")),
            vec![
                MatterBinding::new(
                    id("matter:test"),
                    id("allocation:exact-orchestration"),
                    revision,
                    format!("digest:matter:{revision}"),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn process() -> ProcessSpec {
        ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:align")),
            1,
            ProcessKind::Align,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
    }

    fn exact_plan(workpiece: &Workpiece, process_spec: &ProcessSpec) -> ExecutableFabricationPlan {
        let step_id = PlanStepId::new(id("step:align"));
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:exact-orchestration")),
            1,
            vec![
                PlanStep::new(
                    step_id.clone(),
                    process_spec.id.clone(),
                    process_spec.revision,
                    vec![workpiece.id.clone()],
                    Vec::new(),
                    vec![id("evidence-kind:align")],
                )
                .unwrap(),
            ],
            Vec::new(),
        )
        .unwrap();
        ExecutableFabricationPlan::new(
            plan,
            vec![ExactPlanProcessBinding::new(
                step_id,
                process_spec.snapshot(),
            )],
        )
        .unwrap()
    }

    fn placement(workpiece: &Workpiece, revision: u64) -> PlacementEvidenceRef {
        PlacementEvidenceRef::new(
            id("authority:logistics"),
            id(&format!("placement:{revision}")),
            workpiece.id.stable_id().clone(),
            revision,
            id("location:repair-bench"),
            format!("digest:placement:{revision}"),
        )
        .unwrap()
    }

    fn actor() -> WorkActorRef {
        WorkActorRef {
            authority_id: id("authority:residents"),
            actor_id: id("resident:technician"),
            actor_revision: 1,
        }
    }

    fn released_fixture(
        released_workpiece: &Workpiece,
    ) -> (
        ExactConstructionSite,
        ConstructionStagingLedger,
        ScheduledWorkOrder,
        StagingReservationId,
        ProcessExecutionId,
        ProcessSpec,
    ) {
        let process_spec = process();
        let mut site = ExactConstructionSite::new(
            crate::ConstructionSiteId::new(id("construction-site:exact-orchestration")),
            exact_plan(released_workpiece, &process_spec),
        );
        let mut staging = ConstructionStagingLedger::default();
        let reservation_id = StagingReservationId::new(id("staging-reservation:align"));
        reserve_exact_workpiece(
            &mut staging,
            reservation_id.clone(),
            &site,
            released_workpiece,
            vec![PlanStepId::new(id("step:align"))],
            placement(released_workpiece, 1),
        )
        .unwrap();
        let mut order = issue_exact_work_order(
            WorkOrderId::new(id("work-order:align")),
            &site,
            PlanStepId::new(id("step:align")),
            actor(),
        )
        .unwrap();
        let execution_id = ProcessExecutionId::new(id("process-execution:align"));
        release_exact_work_order(
            &mut staging,
            StagingReleaseRecordId::new(id("staging-release:align")),
            &mut order,
            &mut site,
            SiteStepAdmissionId::new(id("site-admission:align")),
            execution_id.clone(),
            &[],
            vec![StagingReleaseInput {
                reservation_id: &reservation_id,
                workpiece: released_workpiece,
                placement: placement(released_workpiece, 2),
            }],
        )
        .unwrap();
        (
            site,
            staging,
            order,
            reservation_id,
            execution_id,
            process_spec,
        )
    }

    fn evidence(
        execution_id: ProcessExecutionId,
        workpiece: &Workpiece,
        process_spec: &ProcessSpec,
    ) -> ValidatedProcessEvidence {
        let mut execution =
            ProcessExecution::begin(execution_id, process_spec, &[workpiece], &[]).unwrap();
        ValidatedProcessEvidence::new(
            execution
                .complete(
                    id("process-authority:test"),
                    id("process-evidence:align"),
                    2,
                    "digest:process:align:2",
                    vec![
                        MatterBinding::new(
                            id("matter:test"),
                            id("allocation:align-result"),
                            2,
                            "digest:align-result:2",
                        )
                        .unwrap(),
                    ],
                )
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn exact_c2_c3_c4_path_uses_no_caller_plan() {
        let released = workpiece(1);
        let (mut site, staging, order, _reservation, execution_id, process_spec) =
            released_fixture(&released);
        let evidence = evidence(execution_id, &released, &process_spec);

        assert_eq!(
            record_exact_released_process_completion(&staging, &mut site, &evidence).unwrap(),
            PlanStepId::new(id("step:align"))
        );
        assert_eq!(
            exact_work_order_status(&order, &site).unwrap(),
            WorkOrderStatus::Completed
        );
        assert!(site.plan_complete().unwrap());
        assert_eq!(site.exact_completions().len(), 1);
    }

    #[test]
    fn c4_rejects_matter_drift_before_exact_site_mutation() {
        let released = workpiece(1);
        let (mut site, staging, order, _reservation, execution_id, process_spec) =
            released_fixture(&released);
        let drifted = workpiece(2);
        let evidence = evidence(execution_id, &drifted, &process_spec);
        let revision_before = site.revision();

        let result = record_exact_released_process_completion(&staging, &mut site, &evidence);
        assert!(matches!(
            result,
            Err(ExactContinuityError::InputSnapshotMismatch { .. })
        ));
        assert_eq!(site.revision(), revision_before);
        assert_eq!(
            exact_work_order_status(&order, &site).unwrap(),
            WorkOrderStatus::Released
        );
        assert!(site.exact_completions().is_empty());
    }

    #[test]
    fn exact_release_context_rejects_different_site_owned_plan() {
        let released = workpiece(1);
        let (mut site, staging, _order, _reservation, execution_id, process_spec) =
            released_fixture(&released);
        let evidence = evidence(execution_id, &released, &process_spec);

        let other_process = process();
        let mut other_site = ExactConstructionSite::new(
            crate::ConstructionSiteId::new(id("construction-site:other")),
            exact_plan(&released, &other_process),
        );
        let result =
            record_exact_released_process_completion(&staging, &mut other_site, &evidence);
        assert!(matches!(result, Err(ExactContinuityError::MissingRelease(_))));

        // The original exact site remains the only authority matching the release.
        assert!(record_exact_released_process_completion(&staging, &mut site, &evidence).is_ok());
    }
}

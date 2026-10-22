// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-bound construction staging and staged work release.
//!
//! Construction reserves exact fabrication workpieces for exact site plan
//! steps. It does not own inventory, mass, transport, consumption, or physical
//! location. Matter identity remains fabrication/matter authority; placement is
//! supplied by an external revisioned authority. The public release boundary
//! requires exact staging coverage before delegating to the C2 capability gate.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_fabrication::{
    CapabilityAdmission, FabricationPlan, FabricationPlanId, MatterBinding, PlanStepId,
    ProcessExecutionId, Workpiece, WorkpieceId, WorkpieceLifecycle,
};
use symtropy_game_state::StableId;

use crate::{
    ConstructionSite, ConstructionSiteId, ConstructionSiteLifecycle, ScheduledWorkOrder,
    SiteStepAdmissionId, WorkOrderError, WorkOrderId,
};

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

stable_id_type!(StagingReservationId);
stable_id_type!(StagingReleaseRecordId);

/// External attestation that one subject is at one semantic placement/location.
///
/// `subject_revision` is owned by the supplying authority. Construction uses it
/// only to reject stale/conflicting evidence and never increments it itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub subject_id: StableId,
    pub subject_revision: u64,
    pub location_id: StableId,
    pub digest: String,
}

impl PlacementEvidenceRef {
    pub fn new(
        authority_id: StableId,
        evidence_id: StableId,
        subject_id: StableId,
        subject_revision: u64,
        location_id: StableId,
        digest: impl Into<String>,
    ) -> Result<Self, StagingError> {
        let digest = digest.into();
        validate_digest(&digest)?;
        Ok(Self {
            authority_id,
            evidence_id,
            subject_id,
            subject_revision,
            location_id,
            digest,
        })
    }
}

/// Immutable fabrication/matter before-image retained by a construction
/// reservation. This is only a drift detector; construction never mutates the
/// referenced matter bindings or fabrication lifecycle through this snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedWorkpieceSnapshot {
    pub schema_version: u32,
    pub workpiece_id: WorkpieceId,
    pub lifecycle: WorkpieceLifecycle,
    matter_bindings: Vec<MatterBinding>,
}

impl StagedWorkpieceSnapshot {
    pub fn capture(workpiece: &Workpiece) -> Self {
        let mut matter_bindings = workpiece.matter_bindings.clone();
        matter_bindings.sort_by(|left, right| {
            (&left.authority_id, &left.allocation_id)
                .cmp(&(&right.authority_id, &right.allocation_id))
        });
        Self {
            schema_version: workpiece.schema_version,
            workpiece_id: workpiece.id.clone(),
            lifecycle: workpiece.lifecycle,
            matter_bindings,
        }
    }

    pub fn matter_bindings(&self) -> &[MatterBinding] {
        &self.matter_bindings
    }

    pub fn matches(&self, workpiece: &Workpiece) -> bool {
        Self::capture(workpiece) == *self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingReservationLifecycle {
    Held,
    Released,
}

/// Durable reservation of one exact workpiece for one or more exact plan steps.
/// A held reservation is a construction scheduling claim, not an inventory or
/// ownership claim and never means that material was consumed or moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagingReservation {
    pub id: StagingReservationId,
    pub site_id: ConstructionSiteId,
    pub plan_id: FabricationPlanId,
    pub plan_revision: u64,
    pub snapshot: StagedWorkpieceSnapshot,
    step_ids: Vec<PlanStepId>,
    pub placement_at_reservation: PlacementEvidenceRef,
    pub lifecycle: StagingReservationLifecycle,
    pub release_reason_id: Option<StableId>,
}

impl StagingReservation {
    pub fn step_ids(&self) -> &[PlanStepId] {
        &self.step_ids
    }
}

/// One caller-supplied current staging observation used while releasing work.
/// The ledger validates every field against its durable reservation; callers
/// cannot turn this request object into evidence by construction alone.
pub struct StagingReleaseInput<'a> {
    pub reservation_id: &'a StagingReservationId,
    pub workpiece: &'a Workpiece,
    pub placement: PlacementEvidenceRef,
}

/// Durable record of the exact staged workpiece and external placement evidence
/// accepted at a successful work-release boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedStagedWorkpiece {
    pub reservation_id: StagingReservationId,
    pub workpiece_id: WorkpieceId,
    pub placement_evidence: PlacementEvidenceRef,
}

/// Audit record joining C3 staging evidence to the exact C2/C1 release context.
/// It does not claim the physical process subsequently completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagingReleaseRecord {
    pub id: StagingReleaseRecordId,
    pub work_order_id: WorkOrderId,
    pub site_id: ConstructionSiteId,
    pub plan_id: FabricationPlanId,
    pub plan_revision: u64,
    pub step_id: PlanStepId,
    pub site_admission_id: SiteStepAdmissionId,
    pub execution_id: ProcessExecutionId,
    staged_workpieces: Vec<ConfirmedStagedWorkpiece>,
}

impl StagingReleaseRecord {
    pub fn staged_workpieces(&self) -> &[ConfirmedStagedWorkpiece] {
        &self.staged_workpieces
    }
}

/// Construction reservation and release-evidence registry. This prevents
/// construction-level double booking and records staged-release provenance while
/// leaving inventory, matter, transport, and location truth external.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConstructionStagingLedger {
    reservations: Vec<StagingReservation>,
    releases: Vec<StagingReleaseRecord>,
}

impl ConstructionStagingLedger {
    pub fn reservations(&self) -> &[StagingReservation] {
        &self.reservations
    }

    pub fn releases(&self) -> &[StagingReleaseRecord] {
        &self.releases
    }

    pub fn reservation(&self, id: &StagingReservationId) -> Option<&StagingReservation> {
        self.reservations.iter().find(|reservation| &reservation.id == id)
    }

    pub fn release_record(&self, id: &StagingReleaseRecordId) -> Option<&StagingReleaseRecord> {
        self.releases.iter().find(|record| &record.id == id)
    }

    /// Reserves one exact workpiece against the plan steps that actually use it.
    /// Closed/abandoned sites cannot acquire new construction reservations.
    pub fn reserve(
        &mut self,
        id: StagingReservationId,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        workpiece: &Workpiece,
        mut step_ids: Vec<PlanStepId>,
        placement: PlacementEvidenceRef,
    ) -> Result<(), StagingError> {
        validate_site_plan(site, plan)?;
        if matches!(
            site.lifecycle,
            ConstructionSiteLifecycle::PlanClosed | ConstructionSiteLifecycle::Abandoned
        ) {
            return Err(StagingError::SiteUnavailable(site.lifecycle));
        }
        if self.reservation(&id).is_some() {
            return Err(StagingError::DuplicateReservationId(id));
        }
        if self.reservations.iter().any(|reservation| {
            reservation.lifecycle == StagingReservationLifecycle::Held
                && reservation.snapshot.workpiece_id == workpiece.id
        }) {
            return Err(StagingError::WorkpieceAlreadyReserved(workpiece.id.clone()));
        }
        if step_ids.is_empty() {
            return Err(StagingError::PlanStepRequired);
        }
        step_ids.sort();
        for pair in step_ids.windows(2) {
            if pair[0] == pair[1] {
                return Err(StagingError::DuplicatePlanStep(pair[0].clone()));
            }
        }
        for step_id in &step_ids {
            let step = plan
                .step(step_id)
                .ok_or_else(|| StagingError::UnknownPlanStep(step_id.clone()))?;
            if !step.workpieces().contains(&workpiece.id) {
                return Err(StagingError::WorkpieceNotUsedByStep {
                    workpiece_id: workpiece.id.clone(),
                    step_id: step_id.clone(),
                });
            }
        }
        if !matches!(
            workpiece.lifecycle,
            WorkpieceLifecycle::Staged | WorkpieceLifecycle::Available | WorkpieceLifecycle::Removed
        ) {
            return Err(StagingError::WorkpieceNotReservable {
                workpiece_id: workpiece.id.clone(),
                lifecycle: workpiece.lifecycle,
            });
        }
        validate_placement_subject(workpiece, &placement)?;

        self.reservations.push(StagingReservation {
            id,
            site_id: site.id.clone(),
            plan_id: plan.id.clone(),
            plan_revision: plan.revision,
            snapshot: StagedWorkpieceSnapshot::capture(workpiece),
            step_ids,
            placement_at_reservation: placement,
            lifecycle: StagingReservationLifecycle::Held,
            release_reason_id: None,
        });
        self.reservations.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(())
    }

    /// Public construction release transaction.
    ///
    /// Every exact F10 workpiece must have one held reservation that still
    /// matches its fabrication/matter before-image and current external placement
    /// evidence. Only after that exact set is established does C2 perform its
    /// release-time F5 capability check and create the C1 execution admission.
    pub fn release_work_order(
        &mut self,
        release_id: StagingReleaseRecordId,
        order: &mut ScheduledWorkOrder,
        site: &mut ConstructionSite,
        plan: &FabricationPlan,
        site_admission_id: SiteStepAdmissionId,
        execution_id: ProcessExecutionId,
        capability_admissions: &[CapabilityAdmission],
        inputs: Vec<StagingReleaseInput<'_>>,
    ) -> Result<(), StagingError> {
        validate_site_plan(site, plan)?;
        if self.release_record(&release_id).is_some() {
            return Err(StagingError::DuplicateReleaseRecordId(release_id));
        }

        let step = plan
            .step(order.step_id())
            .ok_or_else(|| StagingError::UnknownPlanStep(order.step_id().clone()))?;
        let expected = step.workpieces().to_vec();
        let mut actual = Vec::with_capacity(inputs.len());
        let mut confirmed = Vec::with_capacity(inputs.len());

        for input in inputs {
            if actual.contains(&input.workpiece.id) {
                return Err(StagingError::DuplicateReleaseWorkpiece(
                    input.workpiece.id.clone(),
                ));
            }
            actual.push(input.workpiece.id.clone());
            confirmed.push(self.confirm_for_step(
                input.reservation_id,
                site,
                plan,
                order.step_id(),
                input.workpiece,
                input.placement,
            )?);
        }
        actual.sort();
        if actual != expected {
            return Err(StagingError::WorkpieceCoverageMismatch {
                step_id: order.step_id().clone(),
                expected,
                actual,
            });
        }
        confirmed.sort_by(|left, right| left.workpiece_id.cmp(&right.workpiece_id));

        order.release_after_staging(
            site,
            plan,
            site_admission_id.clone(),
            execution_id.clone(),
            capability_admissions,
        )?;

        self.releases.push(StagingReleaseRecord {
            id: release_id,
            work_order_id: order.id().clone(),
            site_id: site.id.clone(),
            plan_id: plan.id.clone(),
            plan_revision: plan.revision,
            step_id: order.step_id().clone(),
            site_admission_id,
            execution_id,
            staged_workpieces: confirmed,
        });
        self.releases.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(())
    }

    /// Validates a reservation against the exact current fabrication snapshot
    /// and a newly supplied placement attestation for the same external
    /// authority/location. This method does not persist a second location truth.
    fn confirm_for_step(
        &self,
        reservation_id: &StagingReservationId,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        step_id: &PlanStepId,
        workpiece: &Workpiece,
        placement: PlacementEvidenceRef,
    ) -> Result<ConfirmedStagedWorkpiece, StagingError> {
        let reservation = self
            .reservation(reservation_id)
            .ok_or_else(|| StagingError::UnknownReservation(reservation_id.clone()))?;
        if reservation.lifecycle != StagingReservationLifecycle::Held {
            return Err(StagingError::ReservationNotHeld(reservation_id.clone()));
        }
        if reservation.site_id != site.id
            || reservation.plan_id != plan.id
            || reservation.plan_revision != plan.revision
        {
            return Err(StagingError::ReservationContextMismatch(
                reservation_id.clone(),
            ));
        }
        if !reservation.step_ids.contains(step_id) {
            return Err(StagingError::ReservationDoesNotCoverStep {
                reservation_id: reservation_id.clone(),
                step_id: step_id.clone(),
            });
        }
        if reservation.snapshot.workpiece_id != workpiece.id
            || !reservation.snapshot.matches(workpiece)
        {
            return Err(StagingError::WorkpieceSnapshotChanged(
                workpiece.id.clone(),
            ));
        }
        validate_placement_subject(workpiece, &placement)?;
        if placement.authority_id != reservation.placement_at_reservation.authority_id
            || placement.location_id != reservation.placement_at_reservation.location_id
        {
            return Err(StagingError::PlacementContextChanged {
                reservation_id: reservation_id.clone(),
                expected_authority: reservation.placement_at_reservation.authority_id.clone(),
                expected_location: reservation.placement_at_reservation.location_id.clone(),
                actual_authority: placement.authority_id.clone(),
                actual_location: placement.location_id.clone(),
            });
        }
        if placement.subject_revision < reservation.placement_at_reservation.subject_revision {
            return Err(StagingError::StalePlacementEvidence {
                reservation_id: reservation_id.clone(),
                reserved_revision: reservation.placement_at_reservation.subject_revision,
                supplied_revision: placement.subject_revision,
            });
        }
        if placement.subject_revision == reservation.placement_at_reservation.subject_revision
            && placement.digest != reservation.placement_at_reservation.digest
        {
            return Err(StagingError::ConflictingPlacementEvidence(
                reservation_id.clone(),
            ));
        }

        Ok(ConfirmedStagedWorkpiece {
            reservation_id: reservation_id.clone(),
            workpiece_id: workpiece.id.clone(),
            placement_evidence: placement,
        })
    }

    /// Releases only the construction reservation. It makes no claim that the
    /// workpiece moved, was consumed, transformed, or ceased to exist.
    pub fn release_reservation(
        &mut self,
        reservation_id: &StagingReservationId,
        reason_id: StableId,
    ) -> Result<(), StagingError> {
        let reservation = self
            .reservations
            .iter_mut()
            .find(|reservation| &reservation.id == reservation_id)
            .ok_or_else(|| StagingError::UnknownReservation(reservation_id.clone()))?;
        if reservation.lifecycle != StagingReservationLifecycle::Held {
            return Err(StagingError::ReservationNotHeld(reservation_id.clone()));
        }
        reservation.lifecycle = StagingReservationLifecycle::Released;
        reservation.release_reason_id = Some(reason_id);
        Ok(())
    }
}

fn validate_site_plan(
    site: &ConstructionSite,
    plan: &FabricationPlan,
) -> Result<(), StagingError> {
    if site.plan_id != plan.id || site.plan_revision != plan.revision {
        return Err(StagingError::PlanMismatch {
            expected_id: site.plan_id.clone(),
            expected_revision: site.plan_revision,
            actual_id: plan.id.clone(),
            actual_revision: plan.revision,
        });
    }
    Ok(())
}

fn validate_placement_subject(
    workpiece: &Workpiece,
    placement: &PlacementEvidenceRef,
) -> Result<(), StagingError> {
    if placement.subject_id != *workpiece.id.stable_id() {
        return Err(StagingError::PlacementSubjectMismatch {
            expected: workpiece.id.clone(),
            actual: placement.subject_id.clone(),
        });
    }
    validate_digest(&placement.digest)?;
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), StagingError> {
    if digest.is_empty() || digest.len() > 256 {
        return Err(StagingError::InvalidPlacementDigest(digest.to_owned()));
    }
    Ok(())
}

#[derive(Debug)]
pub enum StagingError {
    PlanMismatch {
        expected_id: FabricationPlanId,
        expected_revision: u64,
        actual_id: FabricationPlanId,
        actual_revision: u64,
    },
    SiteUnavailable(ConstructionSiteLifecycle),
    DuplicateReservationId(StagingReservationId),
    DuplicateReleaseRecordId(StagingReleaseRecordId),
    WorkpieceAlreadyReserved(WorkpieceId),
    PlanStepRequired,
    DuplicatePlanStep(PlanStepId),
    UnknownPlanStep(PlanStepId),
    WorkpieceNotUsedByStep {
        workpiece_id: WorkpieceId,
        step_id: PlanStepId,
    },
    WorkpieceNotReservable {
        workpiece_id: WorkpieceId,
        lifecycle: WorkpieceLifecycle,
    },
    InvalidPlacementDigest(String),
    PlacementSubjectMismatch {
        expected: WorkpieceId,
        actual: StableId,
    },
    UnknownReservation(StagingReservationId),
    ReservationNotHeld(StagingReservationId),
    ReservationContextMismatch(StagingReservationId),
    ReservationDoesNotCoverStep {
        reservation_id: StagingReservationId,
        step_id: PlanStepId,
    },
    WorkpieceSnapshotChanged(WorkpieceId),
    PlacementContextChanged {
        reservation_id: StagingReservationId,
        expected_authority: StableId,
        expected_location: StableId,
        actual_authority: StableId,
        actual_location: StableId,
    },
    StalePlacementEvidence {
        reservation_id: StagingReservationId,
        reserved_revision: u64,
        supplied_revision: u64,
    },
    ConflictingPlacementEvidence(StagingReservationId),
    DuplicateReleaseWorkpiece(WorkpieceId),
    WorkpieceCoverageMismatch {
        step_id: PlanStepId,
        expected: Vec<WorkpieceId>,
        actual: Vec<WorkpieceId>,
    },
    WorkOrder(WorkOrderError),
}

impl From<WorkOrderError> for StagingError {
    fn from(error: WorkOrderError) -> Self {
        Self::WorkOrder(error)
    }
}

impl fmt::Display for StagingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "staging binds plan {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::SiteUnavailable(state) => {
                write!(formatter, "construction site cannot accept staging: {state:?}")
            }
            Self::DuplicateReservationId(id) => {
                write!(formatter, "staging reservation {id} already exists")
            }
            Self::DuplicateReleaseRecordId(id) => {
                write!(formatter, "staging release record {id} already exists")
            }
            Self::WorkpieceAlreadyReserved(id) => {
                write!(formatter, "workpiece {id} already has a held construction reservation")
            }
            Self::PlanStepRequired => write!(formatter, "staging reservation requires at least one plan step"),
            Self::DuplicatePlanStep(id) => write!(formatter, "staging reservation repeats plan step {id}"),
            Self::UnknownPlanStep(id) => write!(formatter, "staging references unknown plan step {id}"),
            Self::WorkpieceNotUsedByStep {
                workpiece_id,
                step_id,
            } => write!(
                formatter,
                "workpiece {workpiece_id} is not an input of plan step {step_id}"
            ),
            Self::WorkpieceNotReservable {
                workpiece_id,
                lifecycle,
            } => write!(
                formatter,
                "workpiece {workpiece_id} cannot be construction-staged in {lifecycle:?} lifecycle"
            ),
            Self::InvalidPlacementDigest(digest) => write!(
                formatter,
                "placement evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::PlacementSubjectMismatch { expected, actual } => write!(
                formatter,
                "placement evidence subject {actual} does not match workpiece {expected}"
            ),
            Self::UnknownReservation(id) => write!(formatter, "staging reservation {id} does not exist"),
            Self::ReservationNotHeld(id) => write!(formatter, "staging reservation {id} is not held"),
            Self::ReservationContextMismatch(id) => write!(formatter, "staging reservation {id} does not match site/plan context"),
            Self::ReservationDoesNotCoverStep {
                reservation_id,
                step_id,
            } => write!(
                formatter,
                "staging reservation {reservation_id} does not cover plan step {step_id}"
            ),
            Self::WorkpieceSnapshotChanged(id) => write!(
                formatter,
                "workpiece {id} changed after construction staging reservation"
            ),
            Self::PlacementContextChanged {
                reservation_id,
                expected_authority,
                expected_location,
                actual_authority,
                actual_location,
            } => write!(
                formatter,
                "staging reservation {reservation_id} expected placement {expected_authority}/{expected_location}, got {actual_authority}/{actual_location}"
            ),
            Self::StalePlacementEvidence {
                reservation_id,
                reserved_revision,
                supplied_revision,
            } => write!(
                formatter,
                "staging reservation {reservation_id} placement evidence regressed from revision {reserved_revision} to {supplied_revision}"
            ),
            Self::ConflictingPlacementEvidence(id) => write!(
                formatter,
                "staging reservation {id} received conflicting evidence at the same placement revision"
            ),
            Self::DuplicateReleaseWorkpiece(id) => {
                write!(formatter, "workpiece {id} is repeated in staged work release")
            }
            Self::WorkpieceCoverageMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "staged release for step {step_id} requires workpieces {expected:?}, got {actual:?}"
            ),
            Self::WorkOrder(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for StagingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::WorkOrder(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WorkActorRef, WorkOrderStatus};
    use symtropy_fabrication::{
        FabricationPlan, PlanStep, ProcessExecution, ProcessKind, ProcessSpec, ProcessSpecId,
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

    fn available_workpiece(name: &str, revision: u64) -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id(&format!("workpiece:{name}"))),
            vec![binding(name, revision)],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn single_step_plan(workpieces: &[&Workpiece]) -> FabricationPlan {
        FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:patch-conduit")),
            4,
            vec![
                PlanStep::new(
                    PlanStepId::new(id("step:align")),
                    ProcessSpecId::new(id("process-spec:align")),
                    1,
                    workpieces.iter().map(|workpiece| workpiece.id.clone()).collect(),
                    Vec::new(),
                    Vec::new(),
                )
                .unwrap(),
            ],
            Vec::new(),
        )
        .unwrap()
    }

    fn site(plan: &FabricationPlan) -> ConstructionSite {
        ConstructionSite::new(
            ConstructionSiteId::new(id("construction-site:patch-conduit")),
            plan,
        )
    }

    fn placement(workpiece: &Workpiece, revision: u64, location: &str) -> PlacementEvidenceRef {
        PlacementEvidenceRef::new(
            id("authority:site-logistics"),
            id(&format!("placement:{}:{revision}", workpiece.id.stable_id().as_str())),
            workpiece.id.stable_id().clone(),
            revision,
            id(location),
            format!("digest:placement:{}:{revision}:{location}", workpiece.id.stable_id().as_str()),
        )
        .unwrap()
    }

    fn order(site: &ConstructionSite, plan: &FabricationPlan) -> ScheduledWorkOrder {
        ScheduledWorkOrder::issue(
            WorkOrderId::new(id("work-order:align")),
            site,
            plan,
            PlanStepId::new(id("step:align")),
            actor(),
        )
        .unwrap()
    }

    fn reserve(
        ledger: &mut ConstructionStagingLedger,
        name: &str,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        workpiece: &Workpiece,
    ) -> StagingReservationId {
        let reservation_id = StagingReservationId::new(id(&format!("staging:{name}")));
        ledger
            .reserve(
                reservation_id.clone(),
                site,
                plan,
                workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(workpiece, 7, "location:site-bench"),
            )
            .unwrap();
        reservation_id
    }

    #[test]
    fn reservation_binds_exact_site_plan_workpiece_and_external_placement() {
        let patch = available_workpiece("patch", 1);
        let plan = single_step_plan(&[&patch]);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation_id = reserve(&mut ledger, "patch", &site, &plan, &patch);

        let reservation = ledger.reservation(&reservation_id).unwrap();
        assert_eq!(reservation.snapshot.workpiece_id, patch.id);
        assert_eq!(reservation.step_ids(), &[PlanStepId::new(id("step:align"))]);
        assert_eq!(reservation.lifecycle, StagingReservationLifecycle::Held);
        assert_eq!(
            reservation.placement_at_reservation.location_id,
            id("location:site-bench")
        );
    }

    #[test]
    fn abandoned_site_cannot_acquire_new_staging_reservations() {
        let patch = available_workpiece("patch", 1);
        let plan = single_step_plan(&[&patch]);
        let mut site = site(&plan);
        site.abandon().unwrap();
        let mut ledger = ConstructionStagingLedger::default();
        let result = ledger.reserve(
            StagingReservationId::new(id("staging:patch")),
            &site,
            &plan,
            &patch,
            vec![PlanStepId::new(id("step:align"))],
            placement(&patch, 7, "location:site-bench"),
        );
        assert!(matches!(
            result,
            Err(StagingError::SiteUnavailable(ConstructionSiteLifecycle::Abandoned))
        ));
    }

    #[test]
    fn held_workpiece_cannot_be_double_booked() {
        let patch = available_workpiece("patch", 1);
        let plan = single_step_plan(&[&patch]);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        reserve(&mut ledger, "first", &site, &plan, &patch);

        let result = ledger.reserve(
            StagingReservationId::new(id("staging:second")),
            &site,
            &plan,
            &patch,
            vec![PlanStepId::new(id("step:align"))],
            placement(&patch, 8, "location:site-bench"),
        );
        assert!(matches!(result, Err(StagingError::WorkpieceAlreadyReserved(_))));
    }

    #[test]
    fn changed_matter_snapshot_invalidates_release() {
        let patch = available_workpiece("patch", 1);
        let plan = single_step_plan(&[&patch]);
        let mut site = site(&plan);
        let mut order = order(&site, &plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation = reserve(&mut ledger, "patch", &site, &plan, &patch);
        let changed = available_workpiece("patch", 2);

        let result = ledger.release_work_order(
            StagingReleaseRecordId::new(id("staging-release:align")),
            &mut order,
            &mut site,
            &plan,
            SiteStepAdmissionId::new(id("site-admission:align")),
            ProcessExecutionId::new(id("process-execution:align")),
            &[],
            vec![StagingReleaseInput {
                reservation_id: &reservation,
                workpiece: &changed,
                placement: placement(&changed, 8, "location:site-bench"),
            }],
        );
        assert!(matches!(result, Err(StagingError::WorkpieceSnapshotChanged(_))));
        assert!(!order.is_released());
        assert!(ledger.releases().is_empty());
    }

    #[test]
    fn moved_stale_or_conflicting_placement_evidence_is_rejected() {
        let patch = available_workpiece("patch", 1);
        let plan = single_step_plan(&[&patch]);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation = reserve(&mut ledger, "patch", &site, &plan, &patch);

        let moved = ledger.confirm_for_step(
            &reservation,
            &site,
            &plan,
            &PlanStepId::new(id("step:align")),
            &patch,
            placement(&patch, 8, "location:warehouse"),
        );
        assert!(matches!(moved, Err(StagingError::PlacementContextChanged { .. })));

        let stale = ledger.confirm_for_step(
            &reservation,
            &site,
            &plan,
            &PlanStepId::new(id("step:align")),
            &patch,
            placement(&patch, 6, "location:site-bench"),
        );
        assert!(matches!(stale, Err(StagingError::StalePlacementEvidence { .. })));

        let mut conflict = placement(&patch, 7, "location:site-bench");
        conflict.digest = "digest:conflicting-same-revision".to_owned();
        let conflicting = ledger.confirm_for_step(
            &reservation,
            &site,
            &plan,
            &PlanStepId::new(id("step:align")),
            &patch,
            conflict,
        );
        assert!(matches!(
            conflicting,
            Err(StagingError::ConflictingPlacementEvidence(_))
        ));
    }

    #[test]
    fn staged_release_requires_exact_f10_workpiece_set() {
        let patch = available_workpiece("patch", 1);
        let clamp = available_workpiece("clamp", 1);
        let plan = single_step_plan(&[&patch, &clamp]);
        let mut site = site(&plan);
        let mut order = order(&site, &plan);
        let mut ledger = ConstructionStagingLedger::default();
        let patch_reservation = reserve(&mut ledger, "patch", &site, &plan, &patch);
        reserve(&mut ledger, "clamp", &site, &plan, &clamp);

        let result = ledger.release_work_order(
            StagingReleaseRecordId::new(id("staging-release:align")),
            &mut order,
            &mut site,
            &plan,
            SiteStepAdmissionId::new(id("site-admission:align")),
            ProcessExecutionId::new(id("process-execution:align")),
            &[],
            vec![StagingReleaseInput {
                reservation_id: &patch_reservation,
                workpiece: &patch,
                placement: placement(&patch, 8, "location:site-bench"),
            }],
        );

        assert!(matches!(
            result,
            Err(StagingError::WorkpieceCoverageMismatch { .. })
        ));
        assert!(!order.is_released());
        assert!(site.admissions().is_empty());
    }

    #[test]
    fn successful_staged_release_records_provenance_then_c2_c1_authority() {
        let patch = available_workpiece("patch", 1);
        let clamp = available_workpiece("clamp", 1);
        let plan = single_step_plan(&[&patch, &clamp]);
        let mut site = site(&plan);
        let mut order = order(&site, &plan);
        let mut ledger = ConstructionStagingLedger::default();
        let patch_reservation = reserve(&mut ledger, "patch", &site, &plan, &patch);
        let clamp_reservation = reserve(&mut ledger, "clamp", &site, &plan, &clamp);
        let release_id = StagingReleaseRecordId::new(id("staging-release:align"));
        let execution_id = ProcessExecutionId::new(id("process-execution:align"));

        ledger
            .release_work_order(
                release_id.clone(),
                &mut order,
                &mut site,
                &plan,
                SiteStepAdmissionId::new(id("site-admission:align")),
                execution_id.clone(),
                &[],
                vec![
                    StagingReleaseInput {
                        reservation_id: &clamp_reservation,
                        workpiece: &clamp,
                        placement: placement(&clamp, 9, "location:site-bench"),
                    },
                    StagingReleaseInput {
                        reservation_id: &patch_reservation,
                        workpiece: &patch,
                        placement: placement(&patch, 8, "location:site-bench"),
                    },
                ],
            )
            .unwrap();

        let record = ledger.release_record(&release_id).unwrap();
        assert_eq!(record.work_order_id, *order.id());
        assert_eq!(record.execution_id, execution_id);
        assert_eq!(record.staged_workpieces().len(), 2);
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Released);

        let spec = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:align")),
            1,
            ProcessKind::Align,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap();
        let mut execution = ProcessExecution::begin(
            execution_id,
            &spec,
            &[&patch, &clamp],
            &[],
        )
        .unwrap();
        let evidence = execution
            .complete(
                id("matter:test"),
                id("process-evidence:align"),
                2,
                "digest:process:align",
                vec![binding("aligned-result", 2)],
            )
            .unwrap();
        site.record_completion(&plan, &evidence).unwrap();
        assert_eq!(order.status(&site).unwrap(), WorkOrderStatus::Completed);
    }

    #[test]
    fn release_reservation_frees_only_construction_claim() {
        let patch = available_workpiece("patch", 1);
        let plan = single_step_plan(&[&patch]);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let first = reserve(&mut ledger, "first", &site, &plan, &patch);
        ledger
            .release_reservation(&first, id("reason:restage"))
            .unwrap();

        reserve(&mut ledger, "second", &site, &plan, &patch);

        let value = serde_json::to_value(ledger).unwrap();
        let serialized = value.to_string();
        for forbidden in [
            "inventory",
            "quantity",
            "mass",
            "consumed",
            "transported",
            "progress",
            "quality",
            "score",
        ] {
            assert!(!serialized.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}
